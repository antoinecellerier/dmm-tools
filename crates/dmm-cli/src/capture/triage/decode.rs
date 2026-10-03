//! A step's recorded frames decoded by the device's own protocol, as a
//! sequence of readings, errors and what the library logged on the way.

use super::log;
use super::replay::{Record, Replay, records};
use crate::capture::report::{FrameDir, FrameRecord};
use dmm_lib::error::Error;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{Delivery, capture_reports};
use dmm_lib::transport::Link;

/// Readings in a row that served nothing new before the decode gives up on
/// a protocol that never runs dry; real ones hold a few frames at most.
const MAX_IDLE_READINGS: usize = 64;

/// A step's frames as its protocol read them.
pub(super) struct Wire {
    pub records: Vec<Record>,
    pub events: Vec<Event>,
    /// Whether the meter answered requests rather than streaming.
    pub polled: bool,
    /// Recorded transmissions the protocol never writes itself: the keys
    /// the capture pressed.
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

pub(super) enum EventKind {
    Reading(Box<Measurement>),
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

/// `frames` decoded by a fresh `device` protocol over `link`.
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
) -> Wire {
    let records = records(frames);
    let replay = Replay::new(&records, link);
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

    let mut idle = 0;
    loop {
        let (result, reports) = capture_reports(|| protocol.request_measurement(&replay));
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
    let unread = replay.unread();
    drop(replay);
    Wire {
        records,
        events,
        polled,
        commands,
        polls,
        unread,
    }
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
            let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable));
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
        let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable));
        assert_eq!(shown(&wire), ["DC V -0.0065", "DC V -0.0031"]);
        assert!(wire.commands.is_empty(), "{:?}", wire.commands);
    }

    /// A request nothing answered is a timeout in the middle of the step,
    /// not its end.
    #[test]
    fn an_unanswered_request_is_reported() {
        let ut61eplus = find_device("ut61eplus").unwrap();
        let recorded = frames(&[(0, "tx", POLL), (200, "tx", POLL), (280, "rx", HELD)]);
        let wire = decode(&recorded, ut61eplus, Some(Link::UsbCable));
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
        let wire = decode(&recorded, bm86x, Some(Link::UsbCable));
        assert_eq!(shown(&wire), ["AC V 312.71", "AC V 312.71"]);
        assert!(wire.commands.is_empty());
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
        let wire = decode(&recorded, zt5b, Some(Link::Bluetooth));
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
