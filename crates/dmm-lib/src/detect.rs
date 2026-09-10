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
//! The cascade, the check order and the reasoning behind both live in
//! `docs/detection-design.md`; the byte layouts come from the per-family
//! specs under `docs/research/<family>/reverse-engineered-protocol.md`, cited
//! at each rule below.

use crate::error::{Error, Result};
use crate::protocol::framing;
use crate::protocol::fs9721::{Fs9721Model, is_measurement_frame};
use crate::protocol::registry::{self, SelectableDevice};
use crate::transport::Transport;
use log::{debug, info, warn};
use std::thread;
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

/// Bridge whose meters use FS9721 framing, spelled as `KNOWN_TRANSPORTS`
/// spells it.
const CH9325_BRIDGE: &str = "CH9325";

/// UT61+/UT161 Get Name. Answered with an `FF 00` ack and then the ASCII
/// model name — the only reply that pins the exact model
/// (`docs/research/ut61eplus/reverse-engineered-protocol.md` §6, command
/// `0x5F`).
const CMD_GET_NAME: [u8; 6] = framing::build_abcd_be16(0x5F, &[]);

/// UT181A SET_MONITOR (`0x05`, enable = 1): the meter is silent until it
/// arrives, then streams measurement frames
/// (`docs/research/ut181/reverse-engineered-protocol.md` §7). Byte-identical
/// to what `Ut181aProtocol::init` sends, so a UT181A that answers this probe
/// is left in the state opening it would have produced anyway.
const CMD_SET_MONITOR: [u8; 8] = [0xAB, 0xCD, 0x04, 0x00, 0x05, 0x01, 0x0A, 0x00];

/// UT171 connect / start streaming
/// (`docs/research/ut171/reverse-engineered-protocol.md` §4.3).
///
/// The same bytes are UT181A opcode `0x0A`, *start recording*. Sending them
/// is only safe because the SET_MONITOR step runs first: a UT181A with
/// Communication ON has already identified itself by now, and one with it OFF
/// ignores everything on the wire.
const CMD_UT171_CONNECT: [u8; 8] = [0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x01, 0x0F, 0x00];

/// The ack the Voltcraft vendor software sends three times before every
/// command (`docs/research/vc890/reverse-engineered-protocol.md`; the same
/// frame `vc890::ACK_FRAME` builds).
const CMD_ACK: [u8; 7] = framing::build_abcd_be16(0xFF, &[0x00]);

/// VC-890 measurement poll (`0x5E`). The VC-890 answers nothing else; the
/// VC-880 streams without being asked.
const CMD_VC890_POLL: [u8; 6] = framing::build_abcd_be16(0x5E, &[]);

/// Gap between the three acks, matching the vendor's `Thread.Sleep(100)`.
const ACK_GAP: Duration = Duration::from_millis(100);

/// A LE16 measurement payload this long can only be a UT181A: its normal
/// format passes 31 bytes as soon as it carries an aux value or a bargraph
/// (`docs/research/ut181/reverse-engineered-protocol.md` §5.3), while the
/// UT171's longest measurement response is 21 payload bytes
/// (`docs/research/ut171/reverse-engineered-protocol.md` §3.4).
const LE16_UT181A_ONLY_PAYLOAD: usize = 31;

/// The payload length of a Voltcraft VC-880 live frame (`vc880.rs`).
const VC880_PAYLOAD_LEN: usize = 34;

/// The payload length of a Voltcraft VC-890 live frame (`vc890.rs`).
const VC890_PAYLOAD_LEN: usize = 61;

/// One step of the cascade: what to send, and what a frame arriving
/// afterwards most likely came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    /// UT61+ Get Name.
    Name,
    /// UT181A SET_MONITOR.
    Monitor,
    /// UT171 connect.
    Connect,
    /// The VC-890 ack burst and poll.
    Vc890,
    /// The CH9325 bridge: nothing to send, the meter streams on its own.
    Fs9721Stream,
}

impl Step {
    /// The cascade in order. Get Name first because it is the most verified
    /// probe and the fastest to answer; SET_MONITOR before the UT171 connect
    /// so a UT181A is identified before it can be sent `0x0A`.
    const CASCADE: [Step; 4] = [Step::Name, Step::Monitor, Step::Connect, Step::Vc890];

    fn label(self) -> &'static str {
        match self {
            Step::Name => "ut61+ get name",
            Step::Monitor => "ut181a set monitor",
            Step::Connect => "ut171 connect",
            Step::Vc890 => "vc-890 poll",
            Step::Fs9721Stream => "fs9721 stream",
        }
    }

    fn send(self, transport: &dyn Transport) -> Result<()> {
        match self {
            Step::Name => transport.write(&CMD_GET_NAME),
            Step::Monitor => transport.write(&CMD_SET_MONITOR),
            Step::Connect => transport.write(&CMD_UT171_CONNECT),
            Step::Vc890 => {
                // Three acks 100 ms apart, then the poll — the sequence the
                // vendor software sends before every command.
                for i in 0..3 {
                    transport.write(&CMD_ACK)?;
                    if i < 2 {
                        thread::sleep(ACK_GAP);
                    }
                }
                transport.write(&CMD_VC890_POLL)
            }
            // The CH9325 transport's own init already set the baud rate; the
            // UT803/UT804 stream from there on.
            Step::Fs9721Stream => Ok(()),
        }
    }
}

/// What a buffer of received bytes says about the meter.
enum Signature {
    /// A frame that pins a registry entry.
    Device(Detected),
    /// A UT61+ measurement frame: the family is right but the model is not.
    /// Evidence only — keep listening for the name frame.
    Ut61PlusReading,
}

/// Identify the meter answering on `transport`, `bridge` being the USB bridge
/// it is reached through (`"CP2110"`, `"CH9329"`, `"CH9325"`).
///
/// Sends each family's trigger in turn and classifies everything that comes
/// back; the receive buffer persists across steps, so a meter that speaks
/// slowly still gets the whole cascade's worth of time.
pub fn detect_device(transport: &dyn Transport, bridge: &'static str) -> Result<Detected> {
    // The CH9325 carries one family and needs no trigger, so it gets a single
    // window rather than the cascade — and none of the AB CD probes, which
    // mean nothing to an FS9721 meter.
    if bridge == CH9325_BRIDGE {
        return detect_fs9721(transport, bridge);
    }

    let mut buf: Vec<u8> = Vec::with_capacity(MAX_RX_BUF);
    // A bare UT61+ measurement frame (a stale one from an earlier session, or
    // a meter mid-poll) tells us the family but not the model. Remember it and
    // keep probing: a name frame later in the cascade outranks it.
    let mut saw_ut61plus_reading = false;

    for step in Step::CASCADE {
        debug!("detect: probing {bridge} with {}", step.label());
        step.send(transport)?;
        if let Some(detected) = listen(transport, &mut buf, step, &mut saw_ut61plus_reading)? {
            return Ok(announce(detected, bridge));
        }
        // The family is settled, only the model is not — the remaining
        // probes belong to other families and would go out to a meter we
        // already know is a UT61+.
        if saw_ut61plus_reading {
            warn!(
                "detect: a UT61+ measurement frame arrived on {bridge} but the meter never \
                 answered Get Name; falling back to the UT61E+ tables"
            );
            return Ok(announce(
                Detected {
                    device: registry::default_device(),
                    reported_name: None,
                },
                bridge,
            ));
        }
    }

    debug!("detect: nothing recognisable on {bridge} after the full cascade");
    Err(Error::DeviceNotIdentified { bridge })
}

/// One listen window on the CH9325, whose meters need no trigger at all.
fn detect_fs9721(transport: &dyn Transport, bridge: &'static str) -> Result<Detected> {
    let step = Step::Fs9721Stream;
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_RX_BUF);
    // No UT61+ can be on this bridge, so the name-frame evidence flag has
    // nothing to record here.
    let mut ignored = false;
    debug!("detect: listening on {bridge} for {}", step.label());
    // A no-op, and the reason this bridge is safe to probe blind: an FS9721
    // meter is never sent a byte it might act on.
    step.send(transport)?;
    match listen(transport, &mut buf, step, &mut ignored)? {
        Some(detected) => Ok(announce(detected, bridge)),
        None => {
            debug!("detect: nothing recognisable on {bridge}");
            Err(Error::DeviceNotIdentified { bridge })
        }
    }
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
/// survive it.
fn listen(
    transport: &dyn Transport,
    buf: &mut Vec<u8>,
    step: Step,
    saw_ut61plus_reading: &mut bool,
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
        match classify(buf, step) {
            Some(Signature::Device(detected)) => {
                debug!(
                    "detect: {} identified from {} received bytes during {}",
                    detected.device.id,
                    buf.len(),
                    step.label()
                );
                return Ok(Some(detected));
            }
            Some(Signature::Ut61PlusReading) => *saw_ut61plus_reading = true,
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

/// Classify everything received so far.
///
/// Extractor errors are ignored throughout: to a classifier "this is not that
/// format here" is the answer, not a failure, and a checksum mismatch at one
/// offset says nothing about the next.
fn classify(buf: &[u8], step: Step) -> Option<Signature> {
    if step == Step::Fs9721Stream {
        return classify_fs9721(buf);
    }

    let mut evidence = None;
    for start in header_offsets(buf) {
        match classify_abcd(&buf[start..], step) {
            Some(Signature::Device(detected)) => return Some(Signature::Device(detected)),
            // Keep scanning: a name frame further along the buffer outranks a
            // measurement frame.
            Some(Signature::Ut61PlusReading) => evidence = Some(Signature::Ut61PlusReading),
            None => {}
        }
    }

    // The UT8802's 8-byte format carries no checksum, so its validation
    // accepts roughly 1% of random input — and a UT181A frame is full of
    // arbitrary float32 bytes. Only look at 0xAC when no AB CD frame
    // classified at all.
    if evidence.is_none()
        && let Some(sig) = classify_ut8802(buf)
    {
        return Some(sig);
    }
    evidence
}

/// Offsets of every `AB CD` header in `buf`.
fn header_offsets(buf: &[u8]) -> impl Iterator<Item = usize> + '_ {
    buf.windows(framing::HEADER.len())
        .enumerate()
        .filter(|(_, w)| *w == framing::HEADER)
        .map(|(i, _)| i)
}

/// Classify the frame starting at one `AB CD` header, strictest format first.
fn classify_abcd(tail: &[u8], step: Step) -> Option<Signature> {
    // 1. UT8803: a fixed 21-byte frame with byte 3 == 0x02 and its own
    //    checksum (`docs/research/ut8803/reverse-engineered-protocol.md` §3).
    //    A UT61+ DC V frame also carries 0x02 at byte 3, but it is 19 bytes
    //    long, so its checksum cannot pass here.
    if matches!(framing::extract_frame_ut8803(tail), Ok(Some(_))) {
        return pin("ut8803");
    }

    // 2. The 2-byte LE length families, UT181A and UT171. Their framing and
    //    their measurement type byte are identical and their payload lengths
    //    overlap, so length alone cannot split them.
    if let Ok(Some((payload, _))) = framing::extract_frame_abcd_2byte_le16(tail) {
        // Payload byte 0 is the response type: 0x02 is a live measurement,
        // 0x01 the OK/ER reply to a command, which names no model.
        if payload.first() == Some(&0x02) {
            return match payload.len() {
                len if len >= LE16_UT181A_ONLY_PAYLOAD => pin("ut181a"),
                _ => match step {
                    Step::Monitor => pin("ut181a"),
                    Step::Name => {
                        // Streaming before any LE16 trigger went out: a UT171
                        // left connected, or a UT181A left in monitor mode by
                        // an earlier session. The UT171 is the likelier one,
                        // but say so out loud.
                        warn!(
                            "detect: a short LE16 measurement frame arrived before any trigger; \
                             assuming a UT171 (a UT181A left streaming looks the same)"
                        );
                        pin("ut171")
                    }
                    _ => pin("ut171"),
                },
            };
        }
        debug!(
            "detect: ignoring LE16 frame of type {:#04x}",
            payload.first().copied().unwrap_or(0)
        );
    }

    // 3. The 1-byte BE16 length families, last: a UT8803 frame's byte 2 is a
    //    mode byte, which this extractor would read as a length.
    if let Ok(Some((payload, _))) = framing::extract_frame_abcd_be16(tail) {
        // The ack every UT61+/Voltcraft command gets. Something is listening,
        // but the ack says nothing about what.
        if payload == [0xFF, 0x00] {
            return None;
        }

        // The UT61+ name frame: printable ASCII, e.g. "UT61E+"
        // (`docs/research/ut61eplus/reverse-engineered-protocol.md` §6).
        if (3..=20).contains(&payload.len()) && payload.iter().all(u8::is_ascii_graphic) {
            let name = String::from_utf8_lossy(&payload).into_owned();
            return Some(match registry::device_for_reported_name(&name) {
                Some(device) => {
                    debug!("detect: name frame {name:?} resolves to {}", device.id);
                    Signature::Device(Detected {
                        device,
                        reported_name: Some(name),
                    })
                }
                None => {
                    warn!(
                        "detect: the meter reports an unknown model {name:?}; using the UT61E+ \
                         tables. Please report the name so the registry can carry it."
                    );
                    Signature::Device(Detected {
                        device: registry::default_device(),
                        reported_name: Some(name),
                    })
                }
            });
        }

        // The Voltcraft live frames: type byte 0x01 and a fixed payload
        // length per model (`vc880.rs`, `vc890.rs`).
        if payload.first() == Some(&0x01) {
            match payload.len() {
                VC880_PAYLOAD_LEN => return pin("vc880"),
                VC890_PAYLOAD_LEN => return pin("vc890"),
                _ => {}
            }
        }

        // A UT61+ measurement frame. Only evidence: it does not say which
        // model of the family sent it, and the CH9329 does not purge its RX
        // buffer on open, so it may be left over from an earlier session.
        if payload.len() == framing::UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
            return Some(Signature::Ut61PlusReading);
        }
    }

    None
}

/// Classify a UT8802 stream: two consecutive 8-byte frames, the second
/// exactly 8 bytes after the first
/// (`docs/research/uci-bench-family/reverse-engineered-protocol.md` §3).
///
/// One frame alone is not enough — the format has no checksum, so a single
/// validation pass is weak evidence.
fn classify_ut8802(buf: &[u8]) -> Option<Signature> {
    const FRAME_LEN: usize = 8;
    for (start, _) in buf
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == framing::UT8802_HEADER[0])
    {
        let next = start + FRAME_LEN;
        if next + FRAME_LEN > buf.len() {
            break;
        }
        if matches!(framing::extract_frame_ut8802(&buf[start..]), Ok(Some(_)))
            && matches!(framing::extract_frame_ut8802(&buf[next..]), Ok(Some(_)))
        {
            return pin("ut8802");
        }
    }
    None
}

/// Split a UT803 from a UT804 on the CH9325 bridge.
///
/// Both send the same 14-byte FS9721 frames, so only the payload separates
/// them — and the same two checks the stream filter uses do it
/// (`docs/research/ut803/reverse-engineered-protocol.md`).
fn classify_fs9721(buf: &[u8]) -> Option<Signature> {
    let mut offset = 0;
    while let Ok(Some((nibbles, consumed))) = framing::extract_frame_fs9721(&buf[offset..]) {
        // UT804 first: its `0xD 0xA` marker pair at nibbles 9-10 is positive
        // evidence, where the UT803 check only asks whether the mode nibble is
        // one of the codes we know — which a UT804 frame can satisfy by
        // accident.
        if is_measurement_frame(Fs9721Model::Ut804, &nibbles) {
            return pin("ut804");
        }
        if is_measurement_frame(Fs9721Model::Ut803, &nibbles) {
            return pin("ut803");
        }
        offset += consumed;
    }
    None
}

/// Pin a registry entry by id.
///
/// Every id passed here is a literal `DEVICES` carries (the tests below walk
/// all of them); `?` rather than a panic so a registry rename degrades to "not
/// identified" instead of aborting a user's session.
fn pin(id: &'static str) -> Option<Signature> {
    Some(Signature::Device(Detected {
        device: registry::find_device(id)?,
        reported_name: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::framing::{
        test_frame_be16, test_frame_le16, test_frame_ut8802, test_frame_ut8803, test_ut8803_body,
    };
    use crate::protocol::ut181a::parse::tests::{real_frame_temp_dual_probe, real_frame_vac_hz};
    use crate::transport::mock::MockTransport;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// The UT61E+ name reply, as two whole frames: the ack, then the ASCII
    /// name. Both captured from our UT61E+ over CP2110.
    const ACK: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];
    const NAME_UT61EPLUS: [u8; 11] = [
        0xAB, 0xCD, 0x08, 0x55, 0x54, 0x36, 0x31, 0x45, 0x2B, 0x03, 0x00,
    ];
    /// The same reply from a UT61B+ over CH9329 (issue #19).
    const NAME_UT61BPLUS: [u8; 11] = [
        0xAB, 0xCD, 0x08, 0x55, 0x54, 0x36, 0x31, 0x42, 0x2B, 0x02, 0xFD,
    ];

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

    /// A Voltcraft live frame payload: type byte 0x01, then display fields
    /// the classifier never looks at.
    fn vc_live_payload(len: usize) -> Vec<u8> {
        let mut payload = vec![b'0'; len];
        payload[0] = 0x01;
        payload
    }

    fn detect(transport: &dyn Transport) -> Result<Detected> {
        detect_device(transport, "CP2110")
    }

    #[test]
    fn ut61eplus_answers_get_name_with_whole_frames() {
        let mock = MockTransport::new(vec![ACK.to_vec(), NAME_UT61EPLUS.to_vec()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
        // The name arrives in the first window, so nothing after Get Name is
        // ever sent to a meter that answers it.
        let written = mock.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], CMD_GET_NAME);
    }

    /// CP2110 can deliver one UART byte per HID report, with idle polls in
    /// between: the classifier has to accumulate and re-run after every read.
    #[test]
    fn ut61eplus_name_arrives_one_byte_at_a_time() {
        let mut reports: Vec<Vec<u8>> = Vec::new();
        for byte in ACK.iter().chain(NAME_UT61EPLUS.iter()) {
            reports.push(Vec::new()); // an HID report with no UART payload
            reports.push(vec![*byte]);
        }
        let mock = MockTransport::new(reports);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
    }

    #[test]
    fn ut61bplus_name_picks_its_own_entry() {
        let mock = MockTransport::new(vec![ACK.to_vec(), NAME_UT61BPLUS.to_vec()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61b+");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61B+"));
    }

    /// A name no registry entry carries still identifies the family: the
    /// UT61E+ tables are the fallback and the name is kept for the user.
    #[test]
    fn unknown_name_falls_back_to_the_ut61eplus_tables() {
        let name = test_frame_be16(b"UT60BT");
        let mock = MockTransport::new(vec![ACK.to_vec(), name]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT60BT"));
    }

    /// The cap on empty reads guards against a transport that never blocks,
    /// not against a meter pacing its frames: the CH9325 answers every idle
    /// poll with an empty report, so one window can hold many times the cap
    /// in total and still be a meter answering normally.
    #[test]
    fn idle_reports_between_frames_do_not_end_a_window_early() {
        const IDLE_PER_BYTE: usize = 40;
        let mut reports: Vec<Vec<u8>> = Vec::new();
        for byte in ACK.iter().chain(NAME_UT61EPLUS.iter()) {
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
        assert_eq!(mock.written.borrow().as_slice(), &[CMD_GET_NAME.to_vec()]);
    }

    /// The CH9329 does not purge its RX buffer on open, so a reading from an
    /// earlier session can arrive before the name. The name outranks it.
    #[test]
    fn a_stale_measurement_frame_does_not_outrank_the_name() {
        let stale = test_frame_be16(&[
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, 0x00, 0x02, 0x30, 0x30, 0x30,
        ]);
        let mock = MockTransport::new(vec![stale, ACK.to_vec(), NAME_UT61BPLUS.to_vec()]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61b+");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61B+"));
    }

    /// A meter that only ever sends a measurement frame is still a UT61+, and
    /// the UT61E+ tables are the family fallback — but no model is claimed.
    #[test]
    fn a_lone_measurement_frame_falls_back_to_the_family() {
        let stale = test_frame_be16(&[
            0x02, 0x30, 0x20, 0x30, 0x2E, 0x30, 0x30, 0x30, 0x34, 0x00, 0x02, 0x30, 0x30, 0x30,
        ]);
        let mock = MockTransport::new(vec![stale]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name, None);
        // The family is known after the name window, so no other family's
        // trigger goes out to it.
        assert_eq!(mock.written.borrow().as_slice(), &[CMD_GET_NAME.to_vec()]);
    }

    /// Real UT181A frames, the ones `ut181a::parse` pins: 32 and 57 payload
    /// bytes, both well past the length only a UT181A reaches. They arrive
    /// only after SET_MONITOR, as the meter does on the bench.
    #[test]
    fn ut181a_real_frames_after_set_monitor() {
        for payload in [real_frame_temp_dual_probe(), real_frame_vac_hz()] {
            let meter = ScriptedMeter::answering(&CMD_SET_MONITOR, test_frame_le16(&payload));
            let detected = detect(&meter).unwrap();
            assert_eq!(detected.device.id, "ut181a", "payload {}", payload.len());
            assert_eq!(detected.reported_name, None);
        }
    }

    /// A UT181A in its shortest normal format is 19 payload bytes — inside
    /// the UT171's range. The step that elicited it is what decides.
    #[test]
    fn a_short_le16_frame_after_set_monitor_is_a_ut181a() {
        let meter = ScriptedMeter::answering(
            &CMD_SET_MONITOR,
            test_frame_le16(&le16_measurement_payload(19)),
        );
        let detected = detect(&meter).unwrap();
        assert_eq!(detected.device.id, "ut181a");
    }

    /// The same frame after the UT171 connect instead: a UT181A would have
    /// answered the step before.
    #[test]
    fn a_short_le16_frame_after_the_connect_is_a_ut171() {
        let meter = ScriptedMeter::answering(
            &CMD_UT171_CONNECT,
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

    /// The UT8802's 8-byte format has no checksum, so one frame is not
    /// evidence: two consecutive ones are required.
    #[test]
    fn two_ut8802_frames_identify_it_and_one_does_not() {
        let frame = test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let mut pair = frame.clone();
        pair.extend_from_slice(&frame);
        let mock = MockTransport::new(vec![pair]);
        assert_eq!(detect(&mock).unwrap().device.id, "ut8802");

        let mock = MockTransport::new(vec![frame]);
        assert!(matches!(
            detect(&mock),
            Err(Error::DeviceNotIdentified { .. })
        ));
    }

    #[test]
    fn vc880_stream_is_identified_unprompted() {
        let mock = MockTransport::new(vec![test_frame_be16(&vc_live_payload(VC880_PAYLOAD_LEN))]);
        let detected = detect(&mock).unwrap();
        assert_eq!(detected.device.id, "vc880");
    }

    /// The VC-890 is polled: it answers only the `0x5E` that follows the
    /// three acks, in the last step of the cascade.
    #[test]
    fn vc890_answers_only_the_poll() {
        let meter = ScriptedMeter::answering(
            &CMD_VC890_POLL,
            test_frame_be16(&vc_live_payload(VC890_PAYLOAD_LEN)),
        );
        let detected = detect(&meter).unwrap();
        assert_eq!(detected.device.id, "vc890");
    }

    /// The UT804 marker check runs before the UT803 mode-code check, and
    /// nothing is written to a CH9325 meter.
    #[test]
    fn ch9325_splits_ut804_from_ut803() {
        // 14 bytes, high nibble = index 1..14. Nibbles 9 and 10 carry the
        // UT804's 0xD 0xA marker pair; the UT803 frame has a known mode code
        // (0x2) at nibble 6 instead.
        let mut ut804: Vec<u8> = (1..=14u8).map(|i| i << 4).collect();
        ut804[9] |= 0x0D;
        ut804[10] |= 0x0A;
        let mock = MockTransport::new(vec![ut804]);
        assert_eq!(
            detect_device(&mock, "CH9325").unwrap().device.id,
            "ut804",
            "0xD/0xA markers identify a UT804"
        );
        assert!(mock.written.borrow().is_empty(), "nothing is sent");

        let mut ut803: Vec<u8> = (1..=14u8).map(|i| i << 4).collect();
        ut803[6] |= 0x02;
        let mock = MockTransport::new(vec![ut803]);
        assert_eq!(detect_device(&mock, "CH9325").unwrap().device.id, "ut803");
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
        assert_eq!(written[0], CMD_GET_NAME, "Get Name is always first");
        assert_eq!(
            *written,
            vec![
                CMD_GET_NAME.to_vec(),
                CMD_SET_MONITOR.to_vec(),
                CMD_UT171_CONNECT.to_vec(),
                CMD_ACK.to_vec(),
                CMD_ACK.to_vec(),
                CMD_ACK.to_vec(),
                CMD_VC890_POLL.to_vec(),
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

    /// Every id `pin` can return has to be a real registry entry, or a rename
    /// would silently turn a detected meter into "not identified".
    #[test]
    fn every_pinned_id_is_in_the_registry() {
        for id in [
            "ut8803", "ut8802", "ut181a", "ut171", "vc880", "vc890", "ut803", "ut804",
        ] {
            assert!(pin(id).is_some(), "{id} is not a registry id");
        }
    }
}
