//! A step's recorded frames decoded by the device's own protocol, as a
//! sequence of readings, errors and what the library logged on the way.

use super::log;
use super::replay::{Record, Replay, records};
use crate::capture::report::{FrameDir, FrameRecord};
use dmm_lib::error::Error;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{Delivery, Protocol, capture_reports};
use dmm_lib::transport::Link;

/// Readings in a row that served nothing new before the decode gives up on
/// a protocol that never runs dry; real ones hold a few frames at most.
const MAX_IDLE_READINGS: usize = 64;

/// A step's frames, or a run's opening ones, as its protocol read them.
pub(super) struct Wire {
    /// When the frames start: offsets are measured from it.
    pub t0: u64,
    pub records: Vec<Record>,
    pub events: Vec<Event>,
    /// What the frames were: a step's, or the run's connect.
    pub part: Part,
    /// Whether the meter answered requests rather than streaming.
    pub polled: bool,
    /// Recorded transmissions the protocol never writes itself: the keys
    /// the capture pressed, or in the connect a detection probe's.
    pub commands: Vec<usize>,
    /// Recorded requests the protocol's own requests matched.
    pub polls: Vec<usize>,
    /// Received records a request skipped unread: replies to the keys.
    pub unread: Vec<usize>,
}

pub(super) struct Event {
    /// The record the event's last byte came from, which dates it.
    pub record: Option<usize>,
    pub kind: EventKind,
    /// What the parser called unrecognised on the way.
    pub reports: Vec<String>,
    /// What the library logged on the way.
    pub logs: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Part {
    Step,
    /// The frames before the first step. `by_detection`: the run opened
    /// through detection, whose probe is among them.
    Connect {
        by_detection: bool,
    },
}

pub(super) enum EventKind {
    Reading(Box<Measurement>),
    /// The name the meter gave. `by_detection`: to the detection probe the
    /// run opened through, so the open itself asked nothing.
    Name {
        name: String,
        by_detection: bool,
    },
    Error(String),
    /// No reading came before the frames paused: on a polled meter, a
    /// request nothing answered.
    Timeout,
    /// The frames ran out; any logs say what was left over.
    End,
}

impl Wire {
    pub(super) fn at_ms(&self, record: usize) -> u64 {
        self.records[record].at_ms
    }

    pub(super) fn readings(&self) -> impl Iterator<Item = (Option<usize>, &Measurement)> {
        self.events.iter().filter_map(|e| match &e.kind {
            EventKind::Reading(m) => Some((e.record, m.as_ref())),
            _ => None,
        })
    }
}

/// `frames` decoded by a fresh `device` protocol over `link`, which read
/// `info` as the meter's device information, as the capture's link did.
///
/// Fast whatever the step's length: every read path gives up on an empty
/// transport after a bounded number of reads, not at a deadline. The waits
/// a protocol takes itself still run, though no report in hand has them:
/// the UT803's `init` settles its baud rate for 100 ms per step, and the
/// VC890 sleeps between the acknowledgements around each reading.
pub(super) fn decode(
    frames: &[FrameRecord],
    device: &SelectableDevice,
    link: Option<Link>,
    info: Option<&[u8]>,
) -> Wire {
    let records = records(frames);
    let replay = Replay::new(&records, link, info);
    let mut protocol = (device.new_protocol)();
    let mut events = Vec::new();

    // Its writes are kept, to tell its requests from the keys; its log
    // lines are the same for every step.
    let (init, reports) = capture_reports(|| protocol.init(&replay));
    let logs = log::take();
    if let Err(e) = init {
        events.push(Event {
            record: None,
            kind: EventKind::Error(format!("init: {e}")),
            reports,
            logs,
        });
    }
    let polled = protocol.delivery() == Delivery::Polled;
    replay.open(polled);
    replay.take_progress();
    read_until_dry(protocol.as_mut(), &replay, &mut events);

    let (commands, polls, unread) = sorted_transfers(&records, &replay);
    drop(replay);
    Wire {
        t0: records.first().map_or(0, |r| r.at_ms),
        records,
        events,
        part: Part::Step,
        polled,
        commands,
        polls,
        unread,
    }
}

/// Readings until the frames run dry, each an event.
fn read_until_dry(protocol: &mut dyn Protocol, replay: &Replay, events: &mut Vec<Event>) {
    let mut idle = 0;
    loop {
        let (result, reports) = capture_reports(|| protocol.request_measurement(replay));
        let logs = log::take();
        let progressed = replay.take_progress();
        let record = replay.last_served();
        match result {
            Ok(m) => {
                idle = if progressed { 0 } else { idle + 1 };
                events.push(Event {
                    record,
                    kind: EventKind::Reading(Box::new(m)),
                    reports,
                    logs,
                });
                if idle > MAX_IDLE_READINGS {
                    break;
                }
            }
            // Out of frames: the read found nothing new, or a timeout once
            // nothing is left to serve.
            Err(e) if !progressed || (replay.exhausted() && matches!(e, Error::Timeout)) => {
                if !logs.is_empty() || !reports.is_empty() {
                    events.push(Event {
                        record,
                        kind: EventKind::End,
                        reports,
                        logs,
                    });
                }
                break;
            }
            Err(e) => events.push(Event {
                record,
                kind: match e {
                    Error::Timeout => EventKind::Timeout,
                    e => EventKind::Error(e.to_string()),
                },
                reports,
                logs,
            }),
        }
    }
}

/// `frames` from before a run's first step decoded as the run made them:
/// detection's probe when the run opened through it, [`dmm_lib::Dmm`]'s open
/// (the name first, for a protocol that wants it before `init`, then
/// `init`), then capture's check that the meter answers (its name, else a
/// reading), then whatever else the cable delivered, as a no-response run
/// files it.
///
/// `by_detection`: the run opened through detection. `named_by_detection`:
/// its probe got the name, so neither the open nor the check asked again.
pub(super) fn decode_connect(
    frames: &[FrameRecord],
    device: &SelectableDevice,
    link: Option<Link>,
    info: Option<&[u8]>,
    by_detection: bool,
    named_by_detection: bool,
) -> Wire {
    let records = records(frames);
    let replay = Replay::new(&records, link, info);
    let mut protocol = (device.new_protocol)();
    let mut events = Vec::new();

    // A name asked before `init` is a request and its reply, and so is a
    // probe of a meter that answers requests; a streaming meter's frames
    // after a probe are served past the probes' writes. Otherwise `init`
    // reads nothing, as for a step, so a purge there cannot eat the check's
    // reply.
    let asks_first = protocol.name_before_init(&replay);
    let opened = by_detection || asks_first;
    if opened {
        replay.open(asks_first || protocol.delivery() == Delivery::Polled);
    }
    let mut named = named_by_detection;
    if by_detection {
        if named_by_detection {
            ask_name(protocol.as_mut(), &replay, true, &mut events);
        } else {
            read_once(protocol.as_mut(), &replay, &mut events);
        }
    }
    if !named && asks_first {
        named = ask_name(protocol.as_mut(), &replay, false, &mut events);
    }
    let (init, reports) = capture_reports(|| protocol.init(&replay));
    let logs = log::take();
    replay.take_progress();
    if let Err(e) = init {
        events.push(Event {
            record: at(&replay),
            kind: EventKind::Error(format!("init: {e}")),
            reports,
            logs,
        });
    }
    let polled = protocol.delivery() == Delivery::Polled;
    if !opened {
        replay.open(polled);
    }
    if !named && !ask_name(protocol.as_mut(), &replay, false, &mut events) {
        read_once(protocol.as_mut(), &replay, &mut events);
    }
    if !replay.exhausted() {
        read_until_dry(protocol.as_mut(), &replay, &mut events);
    }

    let (commands, _, unread) = sorted_transfers(&records, &replay);
    // Every write is shown in the connect: the protocol's own, a stream
    // start included, whether or not it released a reply.
    let polls = (0..records.len())
        .filter(|i| records[*i].dir == FrameDir::Tx && !commands.contains(i))
        .collect();
    drop(replay);
    Wire {
        t0: frames.first().map_or(0, |f| f.at_ms),
        records,
        events,
        part: Part::Connect { by_detection },
        polled,
        commands,
        polls,
        unread,
    }
}

/// Where an event happened: the last record served, or the last request
/// written when that came later, so an unanswered one follows its request.
fn at(replay: &Replay) -> Option<usize> {
    replay.last_served().max(replay.polls().last().copied())
}

/// One reading asked for, as an event.
fn read_once(protocol: &mut dyn Protocol, replay: &Replay, events: &mut Vec<Event>) {
    let (result, reports) = capture_reports(|| protocol.request_measurement(replay));
    let logs = log::take();
    replay.take_progress();
    events.push(Event {
        record: at(replay),
        kind: match result {
            Ok(m) => EventKind::Reading(Box::new(m)),
            Err(Error::Timeout) => EventKind::Timeout,
            Err(e) => EventKind::Error(e.to_string()),
        },
        reports,
        logs,
    });
}

/// The protocol's name query over `replay`, as an event; whether it gave a
/// name. A family with no query gives none and leaves no event.
fn ask_name(
    protocol: &mut dyn Protocol,
    replay: &Replay,
    by_detection: bool,
    events: &mut Vec<Event>,
) -> bool {
    let (result, reports) = capture_reports(|| protocol.get_name(replay));
    let logs = log::take();
    replay.take_progress();
    let record = at(replay);
    let (kind, named) = match result {
        Ok(Some(name)) => (EventKind::Name { name, by_detection }, true),
        Ok(None) => return false,
        Err(e) => (EventKind::Error(format!("name: {e}")), false),
    };
    events.push(Event {
        record,
        kind,
        reports,
        logs,
    });
    named
}

/// The recorded transmissions this protocol never wrote, the requests its
/// writes matched, and the replies a request skipped unread.
fn sorted_transfers(records: &[Record], replay: &Replay) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // A request a build that asked differently wrote is matched by
    // position, so it is in `polls` with bytes this protocol never wrote.
    let polls = replay.polls();
    let commands = records
        .iter()
        .enumerate()
        .filter(|(i, r)| {
            r.dir == FrameDir::Tx
                && !replay.written_by_protocol(&r.bytes)
                && polls.binary_search(i).is_err()
        })
        .map(|(i, _)| i)
        .collect();
    (commands, polls, replay.unread())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use dmm_lib::protocol::registry::find_device;

    /// Frames as a report holds them: time, direction, hex.
    pub(in crate::capture::triage) fn frames(list: &[(u64, &str, &str)]) -> Vec<FrameRecord> {
        list.iter()
            .map(|&(at_ms, dir, hex)| FrameRecord {
                at_ms,
                dir: if dir == "tx" {
                    FrameDir::Tx
                } else {
                    FrameDir::Rx
                },
                hex: hex.to_string(),
                feature: false,
                baud: None,
            })
            .collect()
    }

    fn shown(wire: &Wire) -> Vec<String> {
        wire.readings()
            .map(|(_, m)| format!("{} {}", m.mode, m.display_raw.as_deref().unwrap_or("")))
            .collect()
    }

    const POLL: &str = "AB CD 03 5E 01 D9";
    const HOLD_KEY: &str = "AB CD 03 4A 01 C5";
    const ACK: &str = "AB CD 04 FF 00 02 7B";
    const BEFORE: &str = "AB CD 10 02 30 2D 30 2E 30 30 36 35 00 00 30 30 30 03 A0";
    const HELD: &str = "AB CD 10 02 30 2D 30 2E 30 30 33 31 00 01 32 30 30 03 9C";

    /// A UT61E+ on its cable answers each request; a key pressed between
    /// two of them is acknowledged in either order, before or after the
    /// next request goes out, and the ack is no reading.
    #[test]
    fn a_polled_step_decodes_around_a_key_in_either_order() {
        let ut61eplus = find_device("ut61eplus").unwrap();
        let new_order = frames(&[
            (0, "tx", POLL),
            (80, "rx", BEFORE),
            (100, "tx", HOLD_KEY),
            (150, "rx", ACK),
            (200, "tx", POLL),
            (280, "rx", HELD),
        ]);
        let old_order = frames(&[
            (0, "tx", POLL),
            (80, "rx", BEFORE),
            (100, "tx", HOLD_KEY),
            (150, "tx", POLL),
            (175, "rx", ACK),
            (280, "rx", HELD),
        ]);
        for recorded in [new_order, old_order] {
            let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable), None);
            assert!(wire.polled);
            assert_eq!(shown(&wire), ["DC V -0.0065", "DC V -0.0031"]);
            assert_eq!(wire.commands, [2]);
            assert_eq!(wire.polls.len(), 2);
            assert!(
                wire.events
                    .iter()
                    .all(|e| !matches!(e.kind, EventKind::Error(_) | EventKind::Timeout))
            );
        }
    }

    /// A build that asked with other bytes still has its requests matched
    /// by position, and they are requests, not keys.
    #[test]
    fn an_older_request_is_not_a_key() {
        let ut61eplus = find_device("ut61eplus").unwrap();
        let other = "AB CD 03 5E 02 DA";
        let recorded = frames(&[
            (0, "tx", other),
            (80, "rx", BEFORE),
            (100, "tx", other),
            (180, "rx", HELD),
        ]);
        let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable), None);
        assert_eq!(shown(&wire), ["DC V -0.0065", "DC V -0.0031"]);
        assert!(wire.commands.is_empty(), "{:?}", wire.commands);
    }

    /// A request nothing answered is a timeout in the middle of the step,
    /// not its end.
    #[test]
    fn an_unanswered_request_is_reported() {
        let ut61eplus = find_device("ut61eplus").unwrap();
        let recorded = frames(&[(0, "tx", POLL), (200, "tx", POLL), (280, "rx", HELD)]);
        let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable), None);
        assert!(matches!(wire.events[0].kind, EventKind::Timeout));
        assert_eq!(shown(&wire), ["DC V -0.0031"]);
    }

    /// A BM86x drops whatever is queued before each request: served all
    /// at once, the first request's purge would take the second reply too.
    #[test]
    fn a_reply_waits_for_its_request() {
        let bm86x = find_device("bm86x").unwrap();
        let request = "00 86 66";
        let reply = [
            "00 01 11 F8 A0 DA A9 A0",
            "01 00 7E BF A0 A0 04 00",
            "86 86 86 86 00 00 00 00",
        ];
        let recorded = frames(&[
            (0, "tx", request),
            (10, "rx", reply[0]),
            (20, "rx", reply[1]),
            (30, "rx", reply[2]),
            (500, "tx", request),
            (510, "rx", reply[0]),
            (520, "rx", reply[1]),
            (530, "rx", reply[2]),
        ]);
        let wire = decode(&recorded, bm86x, Some(Link::UsbCable), None);
        assert_eq!(shown(&wire), ["AC V 312.71", "AC V 312.71"]);
        assert!(wire.commands.is_empty());
    }

    /// A UT217A's step opened under the CMS101's entry (#45): with the
    /// FFF2 value its link read, the backlog of whole frames each step
    /// starts on decodes as the live run read it, every frame a reading.
    #[test]
    fn a_15_byte_step_decodes_with_the_device_information_read() {
        super::log::install();
        let cms101 = find_device("cms101").unwrap();
        const OHMS: &str = "22 01 F0 07 00 00 20 00 F0 00 00 00 04 00 00";
        let backlog = [OHMS; 74].join(" ");
        let recorded = frames(&[(0, "rx", &backlog), (195, "rx", OHMS), (390, "rx", OHMS)]);
        let info = [0x3D, 0x00, 0x01, 0x04, 0x00, 0x00];
        let wire = decode(&recorded, cms101, Some(Link::Bluetooth), Some(&info));
        let errors: Vec<&str> = wire
            .events
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::Error(e) => Some(e.as_str()),
                _ => None,
            })
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(wire.readings().count(), 76);
        assert!(wire.readings().all(|(_, m)| m.mode == "Ω"));
    }

    const GET_NAME: &str = "AB CD 03 5F 01 DA";
    const NAMED_UT61EPLUS: &str = "AB CD 08 55 54 36 31 45 2B 03 00";

    fn names(wire: &Wire) -> Vec<(String, bool)> {
        wire.events
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::Name { name, by_detection } => Some((name.clone(), *by_detection)),
                _ => None,
            })
            .collect()
    }

    /// A UT61E+ on its cable is asked its name by capture's check, after
    /// an `init` that sends nothing; the ack before the name is no reading.
    #[test]
    fn a_cable_connect_decodes_the_name_the_check_asked() {
        let recorded = frames(&[
            (0, "tx", GET_NAME),
            (80, "rx", ACK),
            (190, "rx", NAMED_UT61EPLUS),
        ]);
        let ut61eplus = find_device("ut61eplus").unwrap();
        let wire = decode_connect(
            &recorded,
            ut61eplus,
            Some(Link::UsbCable),
            None,
            false,
            false,
        );
        assert_eq!(names(&wire), [("UT61E+".to_string(), false)]);
        assert_eq!(wire.events.len(), 1, "no reading is asked once named");
        assert_eq!(wire.polls, [0]);
        assert!(wire.commands.is_empty());
    }

    /// A run opened through detection had the name from the probe: the
    /// frames decode as the name query, said to be the probe's.
    #[test]
    fn a_detected_connect_names_the_probe() {
        let recorded = frames(&[
            (0, "tx", GET_NAME),
            (80, "rx", ACK),
            (190, "rx", NAMED_UT61EPLUS),
        ]);
        let ut61eplus = find_device("ut61eplus").unwrap();
        let wire = decode_connect(&recorded, ut61eplus, Some(Link::UsbCable), None, true, true);
        assert_eq!(names(&wire), [("UT61E+".to_string(), true)]);
        assert_eq!(wire.part, Part::Connect { by_detection: true });
    }

    /// A UT60BT over Bluetooth is asked its name before `init` starts its
    /// stream (family spec §6.4), and the stream start is the open's own
    /// write, not a key.
    #[test]
    fn a_name_before_init_decodes_ahead_of_the_stream_start() {
        // "UT60BT", its sum 0x0325.
        let named = "AB CD 08 55 54 36 30 42 54 03 25";
        let start_stream = "AB CD 03 5D 01 D8";
        let recorded = frames(&[
            (0, "tx", GET_NAME),
            (120, "rx", named),
            (300, "tx", start_stream),
            (400, "rx", ACK),
        ]);
        let ut60bt = find_device("ut60bt").unwrap();
        let wire = decode_connect(&recorded, ut60bt, Some(Link::Bluetooth), None, false, false);
        assert_eq!(names(&wire), [("UT60BT".to_string(), false)]);
        assert_eq!(wire.events[0].record, Some(1));
        assert_eq!(wire.polls, [0, 2]);
        assert!(wire.commands.is_empty(), "{:?}", wire.commands);
        assert!(!wire.polled, "streaming once started");
    }

    /// A run opened through detection whose meter gave no name: the probe
    /// was a reading, and the check asked for another.
    #[test]
    fn an_unnamed_detection_decodes_the_probe_then_the_check() {
        let reply = [
            "00 01 11 F8 A0 DA A9 A0",
            "01 00 7E BF A0 A0 04 00",
            "86 86 86 86 00 00 00 00",
        ];
        let mut list = Vec::new();
        for start in [0, 500] {
            list.push((start, "tx", "00 86 66"));
            for (k, part) in reply.iter().enumerate() {
                list.push((start + 10 * (k as u64 + 1), "rx", *part));
            }
        }
        let bm86x = find_device("bm86x").unwrap();
        let wire = decode_connect(
            &frames(&list),
            bm86x,
            Some(Link::UsbCable),
            None,
            true,
            false,
        );
        assert_eq!(shown(&wire), ["AC V 312.71", "AC V 312.71"]);
        assert_eq!(wire.polls, [0, 4]);
        assert!(wire.commands.is_empty(), "{:?}", wire.commands);
    }

    /// A meter that never answered: each unanswered request is placed
    /// after it, not before the first frame.
    #[test]
    fn an_unanswered_connect_places_each_silence_after_its_request() {
        let recorded = frames(&[(0, "tx", GET_NAME), (2001, "tx", POLL)]);
        let ut61eplus = find_device("ut61eplus").unwrap();
        let wire = decode_connect(
            &recorded,
            ut61eplus,
            Some(Link::UsbCable),
            None,
            false,
            false,
        );
        let last = wire.events.last().unwrap();
        assert!(matches!(last.kind, EventKind::Timeout));
        assert_eq!(last.record, Some(1));
    }

    /// What a no-response run filed is everything the cable delivered: it is
    /// all decoded, not only the check's reading.
    #[test]
    fn a_connect_decodes_every_frame_the_cable_delivered() {
        let packet = "B0 B0 B0 B0 B0 31 B9 B0 34 0D 8A";
        let recorded = frames(&[(0, "rx", packet), (500, "rx", packet), (1000, "rx", packet)]);
        let ut804 = find_device("ut804").unwrap();
        let wire = decode_connect(&recorded, ut804, Some(Link::UsbCable), None, false, false);
        assert_eq!(wire.readings().count(), 3);
    }

    /// A streaming meter opened through detection: the probes written for
    /// other families stand between its frames, which are all read.
    #[test]
    fn a_detected_streaming_meter_reads_past_the_probes() {
        let packet = "B0 B0 B0 B0 B0 31 B9 B0 34 0D 8A";
        let recorded = frames(&[
            (0, "tx", GET_NAME),
            (100, "rx", packet),
            (600, "rx", packet),
            (700, "tx", "00 86 66"),
            (1100, "rx", packet),
        ]);
        let ut804 = find_device("ut804").unwrap();
        let wire = decode_connect(&recorded, ut804, Some(Link::UsbCable), None, true, false);
        assert_eq!(wire.readings().count(), 3);
        assert_eq!(wire.commands, [0, 3]);
    }

    /// A family with no name query is checked with a reading.
    #[test]
    fn a_connect_without_a_name_query_decodes_the_checks_reading() {
        let reply = [
            "00 01 11 F8 A0 DA A9 A0",
            "01 00 7E BF A0 A0 04 00",
            "86 86 86 86 00 00 00 00",
        ];
        let recorded = frames(&[
            (0, "tx", "00 86 66"),
            (10, "rx", reply[0]),
            (20, "rx", reply[1]),
            (30, "rx", reply[2]),
        ]);
        let bm86x = find_device("bm86x").unwrap();
        let wire = decode_connect(&recorded, bm86x, Some(Link::UsbCable), None, false, false);
        assert!(names(&wire).is_empty());
        assert_eq!(shown(&wire), ["AC V 312.71"]);
    }

    /// The ZT-5B streams, scrambled: the key's frame and its acknowledgement
    /// are passed over, and a blank-display packet is skipped with the line
    /// the library logs for it.
    #[test]
    fn a_streamed_step_decodes_past_a_key() {
        super::log::install();
        let zt5b = find_device("zt5b").unwrap();
        // °F 83, then HOLD on; the blank packet is 5A A5 02 00 00 00 10 80
        // 0A 00, a decimal point and nothing else.
        let recorded = frames(&[
            (0, "rx", "1B 84 71 55 A2 21 BD FE 66 EA"),
            (340, "rx", "1B 84 71 55 A2 21 BD FE 66 EA"),
            (340, "tx", "EA EC 70 E1 A2 C1 32 71 64 85"),
            (560, "rx", "EA EC 8E E1 A2 C1 32 71 65 83"),
            (900, "rx", "1B 84 71 55 A2 C1 22 F1 6C AA"),
            (1240, "rx", "1B 84 71 57 A2 21 BD FE 66 EA"),
        ]);
        let wire = decode(&recorded, zt5b, Some(Link::Bluetooth), None);
        assert!(!wire.polled);
        assert_eq!(wire.commands, [2]);
        let readings: Vec<(String, bool)> = wire
            .readings()
            .map(|(_, m)| (m.mode.to_string(), m.flags.hold))
            .collect();
        assert_eq!(
            readings,
            [
                ("°F".to_string(), false),
                ("°F".to_string(), false),
                ("°F".to_string(), true)
            ]
        );
        let last = wire
            .events
            .iter()
            .rfind(|e| matches!(e.kind, EventKind::Reading(_)))
            .unwrap();
        assert_eq!(last.record, Some(5));
        assert!(
            last.logs.iter().any(|l| l.contains("blank main display")),
            "{:?}",
            last.logs
        );
    }
}
