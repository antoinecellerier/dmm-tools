//! Work out which meter is on the far end of the USB cable from the bytes it
//! sends.
//!
//! Every CP2110 cable enumerates as the same `10C4:EA80`, so the bridge chip
//! says nothing about the meter behind it. This module probes: it sends the
//! commands the families answer, accumulates whatever comes back, and
//! classifies the buffer after every read. Streaming meters (UT8803, UT8802,
//! VC-880) never need a probe — they are identified in whichever window they
//! first speak.
//!
//! What each family sends and what it recognises is the family's own
//! [`Fingerprint`], next to the constants it already puts on the wire; this
//! module is the engine that runs them, and the two tables below are the
//! order it runs them in. The reasoning behind both lives in
//! `docs/detection-design.md`.

use crate::error::{Error, Result};
use crate::protocol::registry::{self, SelectableDevice};
use crate::protocol::{
    DeviceFamily, Evidence, Fingerprint, Probing, fs9721, ut61eplus, ut171, ut181a, ut8802, ut8803,
    vc8x0,
};
use crate::transport::Transport;
use log::{debug, info, warn};
use std::time::{Duration, Instant};

/// What the probe cascade concluded.
///
/// [`SelectableDevice`] is a table entry with function pointers in it and
/// derives no `Debug`, so this one prints the registry id instead.
pub struct Detected {
    /// The registry entry to open the meter with.
    pub device: &'static SelectableDevice,
    /// The model name the meter reported, when it sent one. Kept even when it
    /// resolves to no registry entry, so the user can be told what answered.
    pub reported_name: Option<String>,
}

impl std::fmt::Debug for Detected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Detected")
            .field("device", &self.device.id)
            .field("reported_name", &self.reported_name)
            .finish()
    }
}

/// How long each probe step listens.
///
/// Real time, not the session clock: this paces physical USB. The UT61+ name
/// frame has been seen 144–191 ms after the request on both bridges and the
/// UT8803 streams at 2–3 Hz, so 600 ms clears the slowest frame we know of.
const WINDOW: Duration = Duration::from_millis(600);

/// Per-read timeout inside a window. Short, because a window ends on its
/// deadline rather than on a single read, and a frame split across reads has
/// to be re-classified promptly.
const READ_TIMEOUT_MS: i32 = 50;

/// Guard against a transport that returns empty reports without blocking —
/// `MockTransport` does exactly that once drained, and the deadline alone
/// would leave us spinning on it. Same value and reason as
/// `framing::read_uart_bytes`.
const MAX_EMPTY_READS: usize = 256;

/// Buffer cap. A wrong baud rate or a noisy line can deliver garbage forever;
/// the oldest bytes are dropped so growth stays bounded.
const MAX_RX_BUF: usize = 4096;

/// Every family's fingerprint, in the order they are consulted, strictest
/// format first.
///
/// The order is about which extractor is willing to accept another family's
/// bytes. The checksummed UT8803 leads: a UT61+ DC V frame also carries
/// `0x02` at byte 3, but it is 19 bytes long, so its checksum cannot pass
/// there. The 2-byte-LE pair comes next, UT181A before UT171 because the
/// UT181A claims only what its own trigger elicited or what only it can send,
/// leaving the rest to the UT171. The 1-byte BE16 families follow, because a
/// UT8803 frame's byte 2 is a mode byte that their extractor would read as a
/// length; the Voltcraft pair before the UT61+, whose bare reading settles
/// only the family and ends the walk — nothing may sit below it that a
/// stray frame could still name. The checksum-less `0xAC` UT8802 rule is
/// last: its validation accepts roughly 1% of random input, and a UT181A
/// frame is full of arbitrary float32 bytes.
static FINGERPRINTS: [&Fingerprint; 8] = [
    &ut8803::FINGERPRINT,
    &ut181a::FINGERPRINT,
    &ut171::FINGERPRINT,
    &vc8x0::VC880_FINGERPRINT,
    &vc8x0::VC890_FINGERPRINT,
    &ut61eplus::FINGERPRINT,
    &ut8802::FINGERPRINT,
    &fs9721::FINGERPRINT,
];

/// The fingerprints that have something to send, in the order they send it.
///
/// Get Name first because it is the most verified probe and the fastest to
/// answer; SET_MONITOR before the UT171 connect so a UT181A is identified
/// before it can be sent `0x0A`, which is its own *start recording* opcode.
static CASCADE: [&Fingerprint; 4] = [
    &ut61eplus::FINGERPRINT,
    &ut181a::FINGERPRINT,
    &ut171::FINGERPRINT,
    &vc8x0::VC890_FINGERPRINT,
];

/// A family settled by a frame that named no model.
#[derive(Clone, Copy)]
struct FamilyEvidence {
    /// Which family the frame came from, for the warning the user reads.
    family: DeviceFamily,
    /// Registry id to open when nothing better arrives.
    fallback: &'static str,
}

/// The fingerprints worth running on `bridge`, in [`FINGERPRINTS`] order:
/// those of the families the registry places on that cable.
///
/// That is what keeps the AB CD probes off the CH9325, whose FS9721 meters
/// they mean nothing to, and the FS9721 rule off every other bridge, where
/// its frames cannot arrive — without this module knowing either bridge by
/// name.
fn fingerprints_on(bridge: &str) -> Vec<&'static Fingerprint> {
    let families: Vec<DeviceFamily> = crate::devices_on_bridge(bridge)
        .iter()
        .map(|d| d.family)
        .collect();
    FINGERPRINTS
        .iter()
        .copied()
        .filter(|fp| families.contains(&fp.family))
        .collect()
}

/// Identify the meter answering on `transport`, `bridge` being the USB bridge
/// it is reached through, spelled as `KNOWN_TRANSPORTS` spells it.
///
/// Sends each family's trigger in turn and classifies everything that comes
/// back; the receive buffer persists across steps, so a meter that speaks
/// slowly still gets the whole cascade's worth of time.
pub fn detect_device(transport: &dyn Transport, bridge: &'static str) -> Result<Detected> {
    let carried = fingerprints_on(bridge);
    let probes: Vec<&'static Fingerprint> = CASCADE
        .iter()
        .copied()
        .filter(|fp| carried.iter().any(|c| c.family == fp.family))
        .collect();
    // A bridge no probe belongs to gets one listen window per family it
    // carries instead: those meters stream on their own, and the window
    // sends nothing — which is what makes probing such a bridge blind safe.
    let probes = if probes.is_empty() {
        carried.clone()
    } else {
        probes
    };

    let mut buf: Vec<u8> = Vec::with_capacity(MAX_RX_BUF);
    let mut probing = Probing::default();
    // A frame that settles the family but not the model — a bare UT61+
    // reading, stale from an earlier session or caught mid-poll. Remember it
    // and keep probing: a name frame later in the cascade outranks it.
    let mut family_only: Option<FamilyEvidence> = None;

    for fp in probes {
        match fp.trigger {
            Some(send) => {
                debug!("detect: probing {bridge} with {}", fp.label);
                send(transport)?;
                probing.sent.push(fp.family);
            }
            None => debug!("detect: listening on {bridge} for {}", fp.label),
        }
        if let Some(detected) = listen(
            transport,
            &mut buf,
            fp.label,
            &carried,
            &probing,
            &mut family_only,
        )? {
            return Ok(announce(detected, bridge));
        }
        // The family is settled, only the model is not — the remaining
        // probes belong to other families and would go out to a meter we
        // already know the family of.
        if let Some(found) = family_only {
            let device = pin(found.fallback);
            warn!(
                "detect: a {} frame arrived on {bridge} but the meter never named its model; \
                 falling back to the {} tables",
                found.family, device.display_name
            );
            return Ok(announce(
                Detected {
                    device,
                    reported_name: None,
                },
                bridge,
            ));
        }
    }

    debug!("detect: nothing recognisable on {bridge}");
    Err(Error::DeviceNotIdentified { bridge })
}

/// Log the one INFO line the whole detection produces.
fn announce(detected: Detected, bridge: &'static str) -> Detected {
    match &detected.reported_name {
        Some(name) if name != detected.device.display_name => info!(
            "detected {} over {bridge} (the meter reports {name:?})",
            detected.device.display_name
        ),
        _ => info!("detected {} over {bridge}", detected.device.display_name),
    }
    detected
}

/// Read until the window's deadline (or the empty-read cap), classifying the
/// whole buffer after every non-empty read.
///
/// The buffer is the caller's, and never cleared: CP2110 can deliver a single
/// UART byte per HID report, and a frame that straddles a step boundary has to
/// survive it. `label` names the probe this window follows, for the log.
fn listen(
    transport: &dyn Transport,
    buf: &mut Vec<u8>,
    label: &str,
    recognisers: &[&'static Fingerprint],
    probing: &Probing,
    family_only: &mut Option<FamilyEvidence>,
) -> Result<Option<Detected>> {
    let deadline = Instant::now() + WINDOW;
    let mut chunk = [0u8; 64];
    // Consecutive, as in `framing::read_uart_bytes`: the CH9325 answers every
    // idle poll with an empty report, so a meter that is merely between frames
    // would otherwise spend the whole cap long before the deadline.
    let mut empty_reads = 0usize;

    while empty_reads < MAX_EMPTY_READS && Instant::now() < deadline {
        let n = transport.read_timeout(&mut chunk, READ_TIMEOUT_MS)?;
        if n == 0 {
            empty_reads += 1;
            continue;
        }
        empty_reads = 0;
        push(buf, &chunk[..n]);
        match classify(buf, recognisers, probing) {
            Some((
                family,
                Evidence::Model {
                    id,
                    reported_name: name,
                },
            )) => match registry::find_device(id) {
                Some(device) => {
                    debug!(
                        "detect: {id} identified from {} received bytes during {label}",
                        buf.len()
                    );
                    return Ok(Some(Detected {
                        device,
                        reported_name: name,
                    }));
                }
                // Unreachable while the tables below hold: every id a
                // fingerprint returns is one `DEVICES` carries.
                None => warn!("detect: the {family} rule claims {id:?}, which is no registry id"),
            },
            Some((family, Evidence::FamilyOnly { fallback })) => {
                *family_only = Some(FamilyEvidence { family, fallback })
            }
            None => {}
        }
    }
    Ok(None)
}

/// Append `bytes`, dropping the oldest data past [`MAX_RX_BUF`].
fn push(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(bytes);
    if buf.len() > MAX_RX_BUF {
        let excess = buf.len() - MAX_RX_BUF;
        buf.drain(..excess);
    }
}

/// Classify everything received so far: the first fingerprint that
/// recognises anything wins.
///
/// That is what arbitrates between families — [`FINGERPRINTS`] is ordered
/// strictest first — and it is also why [`Evidence::FamilyOnly`] ends the
/// walk: a UT61+ reading must not be second-guessed by the checksum-less
/// UT8802 rule sitting below it.
///
/// Extractor errors are ignored inside every recogniser: to a classifier
/// "this is not that format here" is the answer, not a failure, and a
/// checksum mismatch at one offset says nothing about the next.
fn classify(
    buf: &[u8],
    recognisers: &[&'static Fingerprint],
    probing: &Probing,
) -> Option<(DeviceFamily, Evidence)> {
    recognisers
        .iter()
        .find_map(|fp| (fp.recognise)(buf, probing).map(|evidence| (fp.family, evidence)))
}

/// The registry entry a fingerprint's fallback id names.
///
/// Every id a fingerprint returns is a literal `DEVICES` carries; falling back
/// to the default keeps a registry rename degrading to the UT61E+ tables
/// rather than aborting a user's session.
fn pin(id: &'static str) -> &'static SelectableDevice {
    registry::find_device(id).unwrap_or_else(registry::default_device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::framing::test_ut8803_body;
    use crate::protocol::framing::{test_frame_be16, test_frame_le16, test_frame_ut8803};
    use crate::protocol::ut61eplus::command::Command;
    use crate::protocol::ut171::UT171_CMD_CONNECT;
    use crate::protocol::ut181a::parse::tests::{real_frame_temp_dual_probe, real_frame_vac_hz};
    use crate::protocol::ut181a::set_monitor_frame;
    use crate::protocol::vc8x0::vc890::{ACK_FRAME, POLL_FRAME};
    use crate::protocol::vc8x0::{MSG_TYPE_LIVE_DATA, Vc8x0Model, vc890::Vc890Model};
    use crate::transport::mock::MockTransport;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// The two frames a UT61+ answers Get Name with: the ack, then the ASCII
    /// name. The bytes our UT61E+ sends over CP2110, and what
    /// `protocol::ut61eplus`'s own tests pin.
    fn ack_frame() -> Vec<u8> {
        test_frame_be16(&[0xFF, 0x00])
    }
    fn name_frame() -> Vec<u8> {
        test_frame_be16(b"UT61E+")
    }

    /// A meter that only speaks when it is asked the right question.
    ///
    /// `MockTransport` replays its queue regardless of what was written, so it
    /// cannot express "silent until the trigger arrives" — which is exactly
    /// what separates a UT181A from a UT171, and a VC-890 from a VC-880. It
    /// covers the meters that stream unprompted; this one covers the rest.
    /// Modelled on `VoltsDial` in `protocol::ut61eplus`.
    struct ScriptedMeter {
        /// `(trigger, reply)`: the reply is queued when exactly those bytes
        /// are written.
        script: Vec<(Vec<u8>, Vec<u8>)>,
        queued: RefCell<VecDeque<Vec<u8>>>,
    }

    impl ScriptedMeter {
        /// A meter that answers `trigger` with `reply` and is otherwise silent.
        fn answering(trigger: &[u8], reply: Vec<u8>) -> Self {
            Self {
                script: vec![(trigger.to_vec(), reply)],
                queued: RefCell::new(VecDeque::new()),
            }
        }
    }

    impl Transport for ScriptedMeter {
        fn write(&self, data: &[u8]) -> Result<()> {
            for (trigger, reply) in &self.script {
                if data == trigger.as_slice() {
                    self.queued.borrow_mut().push_back(reply.clone());
                }
            }
            Ok(())
        }

        /// A frame longer than the caller's buffer is split across reads, as
        /// a bridge splits one over several HID reports — the VC-890's live
        /// frame is 66 bytes and never arrives in one.
        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let mut queued = self.queued.borrow_mut();
            let Some(frame) = queued.pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            if len < frame.len() {
                queued.push_front(frame[len..].to_vec());
            }
            Ok(len)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    /// A LE16 measurement payload of `len` bytes, shaped like the UT171's
    /// standard response: type 0x02, flags, frame type, mode, range, then
    /// little-endian float fields.
    fn le16_measurement_payload(len: usize) -> Vec<u8> {
        let mut payload = vec![0u8; len];
        payload[0] = 0x02; // response type = measurement
        payload[2] = 0x01; // frame type = standard
        payload
    }

    /// A UT61+ measurement frame, as one arrives from a meter mid-poll.
    fn ut61plus_reading() -> Vec<u8> {
        test_frame_be16(&[
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, 0x00, 0x02, 0x30, 0x30, 0x30,
        ])
    }

    fn detect(transport: &dyn Transport) -> Result<Detected> {
        detect_device(transport, "CP2110")
    }

    #[test]
    fn ut61eplus_answers_get_name_with_whole_frames() {
        let mock = MockTransport::new(vec![ack_frame(), name_frame()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
        // The name arrives in the first window, so nothing after Get Name is
        // ever sent to a meter that answers it.
        let written = mock.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], Command::GetName.encode());
    }

    /// CP2110 can deliver one UART byte per HID report, with idle polls in
    /// between: the classifier has to accumulate and re-run after every read.
    #[test]
    fn ut61eplus_name_arrives_one_byte_at_a_time() {
        let mut reports: Vec<Vec<u8>> = Vec::new();
        for byte in ack_frame().iter().chain(name_frame().iter()) {
            reports.push(Vec::new()); // an HID report with no UART payload
            reports.push(vec![*byte]);
        }
        let mock = MockTransport::new(reports);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
    }

    /// The cap on empty reads guards against a transport that never blocks,
    /// not against a meter pacing its frames: the CH9325 answers every idle
    /// poll with an empty report, so one window can hold many times the cap
    /// in total and still be a meter answering normally.
    #[test]
    fn idle_reports_between_frames_do_not_end_a_window_early() {
        const IDLE_PER_BYTE: usize = 40;
        let mut reports: Vec<Vec<u8>> = Vec::new();
        for byte in ack_frame().iter().chain(name_frame().iter()) {
            reports.extend(vec![Vec::new(); IDLE_PER_BYTE]);
            reports.push(vec![*byte]);
        }
        let empty = reports.iter().filter(|r| r.is_empty()).count();
        assert!(
            empty > MAX_EMPTY_READS,
            "{empty} idle reports is under the cap"
        );
        let mock = MockTransport::new(reports);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
        // Still inside the first window, so no other family's trigger went out.
        assert_eq!(
            mock.written.borrow().as_slice(),
            &[Command::GetName.encode().to_vec()]
        );
    }

    /// The CH9329 does not purge its RX buffer on open, so a reading from an
    /// earlier session can arrive before the name. The name outranks it.
    #[test]
    fn a_stale_measurement_frame_does_not_outrank_the_name() {
        let mock = MockTransport::new(vec![ut61plus_reading(), ack_frame(), name_frame()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
    }

    /// A meter that only ever sends a measurement frame is still a UT61+, and
    /// the UT61E+ tables are the family fallback — but no model is claimed.
    #[test]
    fn a_lone_measurement_frame_falls_back_to_the_family() {
        let mock = MockTransport::new(vec![ut61plus_reading()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name, None);
        // The family is known after the name window, so no other family's
        // trigger goes out to it.
        assert_eq!(
            mock.written.borrow().as_slice(),
            &[Command::GetName.encode().to_vec()]
        );
    }

    /// Real UT181A frames, the ones `ut181a::parse` pins: 32 and 57 payload
    /// bytes, both well past the length only a UT181A reaches. They arrive
    /// only after SET_MONITOR, as the meter does on the bench.
    #[test]
    fn ut181a_real_frames_after_set_monitor() {
        for payload in [real_frame_temp_dual_probe(), real_frame_vac_hz()] {
            let meter = ScriptedMeter::answering(&set_monitor_frame(), test_frame_le16(&payload));
            let detected = detect(&meter).unwrap();
            assert_eq!(detected.device.id, "ut181a", "payload {}", payload.len());
            assert_eq!(detected.reported_name, None);
        }
    }

    /// A UT181A in its shortest normal format is 19 payload bytes — inside
    /// the UT171's range. The probe that elicited it is what decides.
    #[test]
    fn a_short_le16_frame_after_set_monitor_is_a_ut181a() {
        let meter = ScriptedMeter::answering(
            &set_monitor_frame(),
            test_frame_le16(&le16_measurement_payload(19)),
        );
        let detected = detect(&meter).unwrap();
        assert_eq!(detected.device.id, "ut181a");
    }

    /// The same frame after the UT171 connect instead: a UT181A would have
    /// answered the probe before.
    #[test]
    fn a_short_le16_frame_after_the_connect_is_a_ut171() {
        let meter = ScriptedMeter::answering(
            UT171_CMD_CONNECT,
            test_frame_le16(&le16_measurement_payload(15)),
        );
        let detected = detect(&meter).unwrap();
        assert_eq!(detected.device.id, "ut171");
    }

    /// The UT8803 streams without being asked, so it is identified in the
    /// first window — before any probe can reach it.
    #[test]
    fn ut8803_stream_is_identified_unprompted() {
        let mock = MockTransport::new(vec![test_frame_ut8803(&test_ut8803_body())]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut8803");
    }

    /// The VC-890 is polled: it answers only the `0x5E` that follows the
    /// three acks, in the last step of the cascade.
    #[test]
    fn vc890_answers_only_the_poll() {
        let mut payload = vec![b'0'; Vc890Model::PAYLOAD_LEN];
        payload[0] = MSG_TYPE_LIVE_DATA;
        let meter = ScriptedMeter::answering(&POLL_FRAME, test_frame_be16(&payload));
        let detected = detect(&meter).unwrap();
        assert_eq!(detected.device.id, "vc890");
    }

    /// The CH9325 window sends nothing at all, and the FS9721 family is the
    /// only one it can identify.
    #[test]
    fn the_ch9325_window_writes_nothing() {
        // 14 bytes, high nibble = index 1..14. Nibbles 9 and 10 carry the
        // UT804's 0xD 0xA marker pair.
        let mut ut804: Vec<u8> = (1..=14u8).map(|i| i << 4).collect();
        ut804[9] |= 0x0D;
        ut804[10] |= 0x0A;
        let mock = MockTransport::new(vec![ut804]);
        assert_eq!(detect_device(&mock, "CH9325").unwrap().device.id, "ut804");
        assert!(mock.written.borrow().is_empty(), "nothing is sent");
    }

    #[test]
    fn silence_is_not_identified() {
        let mock = MockTransport::new(vec![]);
        let err = detect(&mock).unwrap_err();
        assert!(matches!(
            err,
            Error::DeviceNotIdentified { bridge: "CP2110" }
        ));
    }

    /// The full cascade, in the order the design fixes: Get Name first
    /// (most verified, fastest), SET_MONITOR before the UT171 connect so a
    /// UT181A is never sent `0x0A`, then the VC-890 ack burst and poll.
    #[test]
    fn the_probes_go_out_in_the_designed_order() {
        let mock = MockTransport::new(vec![]);
        assert!(detect(&mock).is_err());
        let written = mock.written.borrow();
        assert_eq!(
            written[0],
            Command::GetName.encode(),
            "Get Name is always first"
        );
        assert_eq!(
            *written,
            vec![
                Command::GetName.encode().to_vec(),
                set_monitor_frame(),
                UT171_CMD_CONNECT.to_vec(),
                ACK_FRAME.to_vec(),
                ACK_FRAME.to_vec(),
                ACK_FRAME.to_vec(),
                POLL_FRAME.to_vec(),
            ]
        );
    }

    /// A garbage flood (wrong baud, noisy line) must neither grow the buffer
    /// without bound nor stop the cascade from finishing.
    #[test]
    fn a_garbage_flood_stays_bounded_and_finishes() {
        // `AB CD 00` repeated: a header at every third offset, none of which
        // can ever complete a frame, so the classifier walks them all.
        let junk: Vec<u8> = [0xABu8, 0xCD, 0x00]
            .iter()
            .cycle()
            .take(64)
            .copied()
            .collect();
        let mock = MockTransport::new(vec![junk; 200]);
        assert!(matches!(
            detect(&mock),
            Err(Error::DeviceNotIdentified { .. })
        ));
    }

    #[test]
    fn push_drops_the_oldest_bytes_at_the_cap() {
        let mut buf = Vec::new();
        for _ in 0..(MAX_RX_BUF / 64 + 10) {
            push(&mut buf, &[0xAA; 64]);
            assert!(buf.len() <= MAX_RX_BUF, "buffer grew to {}", buf.len());
        }
        assert_eq!(buf.len(), MAX_RX_BUF);
    }

    /// A meter the registry can open but no fingerprint recognises is one
    /// `--device auto` silently never finds.
    #[test]
    fn every_hardware_family_has_one_fingerprint() {
        for device in registry::DEVICES.iter().filter(|d| d.requires_hardware) {
            let found = FINGERPRINTS
                .iter()
                .filter(|fp| fp.family == device.family)
                .count();
            assert_eq!(found, 1, "{} has {found} fingerprints", device.id);
        }
    }

    /// Two rules for one family would make the weaker one unreachable, and
    /// which of them ran would depend on the table order alone.
    #[test]
    fn no_family_is_fingerprinted_twice() {
        for (i, fp) in FINGERPRINTS.iter().enumerate() {
            for other in &FINGERPRINTS[i + 1..] {
                assert_ne!(
                    fp.family, other.family,
                    "{} and {} share a family",
                    fp.label, other.label
                );
            }
        }
    }

    /// The registry, not this module, says which rules a bridge gets: the
    /// CH9325 carries the FS9721 family alone, and no AB CD bridge carries it.
    #[test]
    fn the_registry_decides_which_fingerprints_a_bridge_gets() {
        let families = |bridge: &str| -> Vec<DeviceFamily> {
            fingerprints_on(bridge).iter().map(|fp| fp.family).collect()
        };
        assert_eq!(families("CH9325"), vec![DeviceFamily::Fs9721]);
        for bridge in ["CP2110", "CH9329"] {
            let on_bridge = families(bridge);
            assert!(!on_bridge.contains(&DeviceFamily::Fs9721), "{bridge}");
            // Get Name goes out on both: a UT61B+ is verified over CH9329.
            assert!(on_bridge.contains(&DeviceFamily::Ut61EPlus), "{bridge}");
            // And SET_MONITOR before the UT171 connect on both — the order
            // that keeps `0x0A` away from a UT181A, on whichever cable it has.
            assert!(on_bridge.contains(&DeviceFamily::Ut181a), "{bridge}");
            assert!(on_bridge.contains(&DeviceFamily::Ut171), "{bridge}");
        }
        assert!(fingerprints_on("no such bridge").is_empty());
    }

    /// A cascade entry is a fingerprint with something to send, and the
    /// window it opens only recognises what its own table row does.
    #[test]
    fn every_cascade_entry_sends_something_and_is_recognised() {
        for fp in CASCADE {
            assert!(
                fp.trigger.is_some(),
                "{} is in the cascade but sends nothing",
                fp.label
            );
            let listed = FINGERPRINTS
                .iter()
                .filter(|f| f.family == fp.family)
                .count();
            assert_eq!(listed, 1, "{} is listed {listed} times", fp.label);
        }
    }
}
