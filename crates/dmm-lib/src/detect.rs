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
//! module is the engine that runs them. Which fingerprints run, and the order
//! their triggers go out in, is derived: membership and preference from
//! [`registry::DEVICES`], the one hard ordering constraint from the
//! fingerprint that needs it. Nothing here names a family. The reasoning
//! lives in `docs/detection-design.md`.

use crate::error::{Error, Result};
use crate::protocol::registry::{self, SelectableDevice};
use crate::protocol::{DeviceFamily, Evidence, Fingerprint, Probing};
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
/// UT8803 streams at 2–3 Hz, so 600 ms clears both. A step that sends nothing
/// listens for [`LISTEN_ONLY_WINDOW`] instead.
const WINDOW: Duration = Duration::from_millis(600);

/// How long a step that sends nothing listens, on a bridge no probe belongs to.
///
/// There the meter sets the pace. The slowest stream measured, issue #16's
/// UT804, sends a packet every 656 ms, and a [`WINDOW`] holds a whole one only
/// when it starts in the first ~554 ms — about five runs in six. 1.5 s gives
/// two chances.
const LISTEN_ONLY_WINDOW: Duration = Duration::from_millis(1500);

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

/// A family settled by a frame that named no model.
#[derive(Clone, Copy)]
struct FamilyEvidence {
    /// Which family the frame came from, for the warning the user reads.
    family: DeviceFamily,
    /// Registry id to open when nothing better arrives.
    fallback: &'static str,
}

/// The fingerprints worth running on `bridge`, in registry order and each one
/// once: the ones the entries the registry places on that cable point at.
///
/// That is what keeps the AB CD probes off the CH9325, whose UT80x-family
/// meters they mean nothing to, and the UT80x rule off every other bridge,
/// where its frames cannot arrive — without this module knowing either bridge
/// by name. A family has one fingerprint and several entries point at it, so
/// the list is deduplicated by identity.
///
/// `named` narrows it further: the meters the peer's advertised name belongs
/// to ([`crate::built_in_meters`]). Such a meter's name already says which
/// families it can be, so the other families' probes stay off it. Empty —
/// an adapter, a cable, a peer no name was heard from — leaves the bridge's
/// whole set.
fn fingerprints_on(bridge: &str, named: &[&'static SelectableDevice]) -> Vec<&'static Fingerprint> {
    let entries = if named.is_empty() {
        crate::devices_on_bridge(bridge)
    } else {
        named
            .iter()
            .copied()
            .filter(|d| crate::is_on_bridge(d, bridge))
            .collect()
    };
    let mut carried: Vec<&'static Fingerprint> = Vec::new();
    for fp in entries.iter().filter_map(|d| d.fingerprint) {
        if !carried.iter().any(|seen| std::ptr::eq(*seen, fp)) {
            carried.push(fp);
        }
    }
    carried
}

/// The triggers `carried` has to send, in the order they go out.
///
/// Registry order is the preference, because [`registry::DEVICES`] lists the
/// most common meters first: that is what puts the UT61+ Get Name — the
/// best-verified probe and the fastest to answer — at the head of the
/// cascade, without this module saying so.
///
/// The one hard constraint is a fingerprint's own [`Fingerprint::send_after`]:
/// it waits until every family it names has had its trigger taken. The UT171
/// connect frame is UT181A opcode `0x0A` (start recording), so SET_MONITOR
/// goes first and a UT181A is identified before the opcode reaches it. A
/// named family whose trigger does not go out on this bridge is ignored —
/// there is nothing there to wait for.
fn probe_order(carried: &[&'static Fingerprint]) -> Vec<&'static Fingerprint> {
    let mut pending: Vec<&'static Fingerprint> = carried
        .iter()
        .copied()
        .filter(|fp| fp.trigger.is_some())
        .collect();
    let sending: Vec<DeviceFamily> = pending.iter().map(|fp| fp.family).collect();
    let mut ordered: Vec<&'static Fingerprint> = Vec::with_capacity(pending.len());

    while !pending.is_empty() {
        // The earliest entry still waiting for nothing. One at a time,
        // rescanning from the top, so a probe that was deferred goes out as
        // soon as its constraint is met rather than at the back of the queue.
        let ready = pending.iter().position(|fp| {
            fp.send_after.iter().all(|needed| {
                !sending.contains(needed) || ordered.iter().any(|o| o.family == *needed)
            })
        });
        match ready {
            Some(i) => ordered.push(pending.remove(i)),
            None => {
                // The declarations contradict each other, so no order
                // satisfies them. Registry order at least still sends every
                // probe, which beats sending none — and the contradiction is
                // a code bug the log has to name.
                let labels: Vec<&str> = pending.iter().map(|fp| fp.label).collect();
                warn!(
                    "detect: the send-after rules of {} cannot all be met; falling back to \
                     registry order",
                    labels.join(", ")
                );
                ordered.append(&mut pending);
            }
        }
    }
    ordered
}

/// Identify the meter answering on `transport`, `bridge` being the USB bridge
/// it is reached through, spelled as `KNOWN_TRANSPORTS` spells it.
///
/// Sends each family's trigger in turn and classifies everything that comes
/// back; the receive buffer persists across steps, so a meter that speaks
/// slowly still gets the whole cascade's worth of time.
pub fn detect_device(transport: &dyn Transport, bridge: &'static str) -> Result<Detected> {
    detect_among(transport, bridge, &crate::built_in_meters(transport))
}

/// [`detect_device`], `named` being the meters the peer's advertised name
/// belongs to: their fingerprints alone run, and when the name is one
/// meter's alone, that meter is what a frame naming no model falls back to.
/// Several meters sharing a name leave the fallback to the frames.
fn detect_among(
    transport: &dyn Transport,
    bridge: &'static str,
    named: &[&'static SelectableDevice],
) -> Result<Detected> {
    let carried = fingerprints_on(bridge, named);
    let probes = probe_order(&carried);
    // A bridge no probe belongs to gets one listen window per family it
    // carries instead: those meters stream on their own, and the window
    // sends nothing — which is what makes probing such a bridge blind safe.
    let probes = if probes.is_empty() {
        carried.clone()
    } else {
        probes
    };

    let mut buf: Vec<u8> = Vec::with_capacity(MAX_RX_BUF);
    let mut probing = Probing {
        advertised: match named {
            [one] => Some(*one),
            _ => None,
        },
        ..Probing::default()
    };
    // A frame that settles the family but not the model — a bare UT61+
    // reading, stale from an earlier session or caught mid-poll. Remember it
    // and keep probing: a name frame later in the cascade outranks it.
    let mut family_only: Option<FamilyEvidence> = None;

    for fp in probes {
        let window = match fp.trigger {
            Some(send) => {
                debug!("detect: probing {bridge} with {}", fp.label);
                send(transport)?;
                probing.sent.push(fp.family);
                WINDOW
            }
            None => {
                debug!("detect: listening on {bridge} for {}", fp.label);
                LISTEN_ONLY_WINDOW
            }
        };
        if let Some(detected) = listen(
            transport,
            &mut buf,
            window,
            fp.label,
            &carried,
            &probing,
            &mut family_only,
        )? {
            // The meter's own answer wins over the name its radio advertises;
            // a mismatch is worth a report, not a second guess.
            if let Some(advertised) = probing.advertised.filter(|a| a.id != detected.device.id) {
                warn!(
                    "detect: the meter on {bridge} advertises itself as a {}, but its own reply \
                     identifies the {model}; using the {model} tables",
                    advertised.display_name,
                    model = detected.device.display_name
                );
            }
            return Ok(announce(detected, bridge));
        }
        // The family is settled, only the model is not — the remaining
        // probes belong to other families and would go out to a meter we
        // already know the family of.
        if let Some(found) = family_only {
            let (device, why) = family_fallback(found, probing.advertised, bridge);
            warn!("{why}");
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
    Err(Error::DeviceNotIdentified {
        bridge,
        built_in_radio: !named.is_empty(),
    })
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

/// Read for `window` (or until the empty-read cap), classifying the whole
/// buffer after every non-empty read.
///
/// The buffer is the caller's, and never cleared: CP2110 can deliver a single
/// UART byte per HID report, and a frame that straddles a step boundary has to
/// survive it. `label` names the probe this window follows, for the log.
fn listen(
    transport: &dyn Transport,
    buf: &mut Vec<u8>,
    window: Duration,
    label: &str,
    recognisers: &[&'static Fingerprint],
    probing: &Probing,
    family_only: &mut Option<FamilyEvidence>,
) -> Result<Option<Detected>> {
    let deadline = Instant::now() + window;
    let mut chunk = [0u8; 64];
    // Consecutive, as in `framing::read_uart_bytes`: the CH9325 answers every
    // idle poll with an empty report, so a meter that is merely between frames
    // would otherwise spend the whole cap long before the deadline. At one
    // such report every ~12 ms the cap is ~3 s, past the longest window.
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
                // Unreachable while the registry holds: every id a
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

/// How much a rule's answer is worth, weakest first.
///
/// This is what arbitrates when two rules claim the same bytes, in place of a
/// hand-ordered list of families: the ranking is a property of the evidence,
/// which each rule already knows about itself.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Strength {
    /// A model claimed by a rule whose extractor validates no checksum
    /// (UT8802, UT80x). The `0xAC` format passes roughly 1% of random
    /// bytes, and a UT181A frame is full of arbitrary float32 bytes.
    Unchecksummed,
    /// A checksummed frame that settles the family but names no model — a
    /// bare UT61+ reading. Above an unchecksummed claim, below anything that
    /// picks a model with a checksum behind it.
    FamilyOnly,
    /// A model claimed from a frame whose checksum held.
    Checksummed,
    /// A model the meter named itself. Only the UT61+ name frame reaches
    /// here, and it is what tells the siblings of that family apart.
    Named,
}

/// What one rule's answer is worth: a checksum behind it, and whether the
/// meter named itself, are all it takes to rank it.
fn strength(fp: &Fingerprint, evidence: &Evidence) -> Strength {
    match evidence {
        Evidence::Model {
            reported_name: Some(_),
            ..
        } => Strength::Named,
        Evidence::Model { .. } if fp.checksummed => Strength::Checksummed,
        Evidence::Model { .. } => Strength::Unchecksummed,
        Evidence::FamilyOnly { .. } => Strength::FamilyOnly,
    }
}

/// Classify everything received so far: every rule runs, and the strongest
/// evidence wins.
///
/// Two rules claiming the same bytes at the same [`Strength`] for *different*
/// families is a coincidence — a 2^-16 checksum collision, or a rule that
/// claims too much — and nothing is identified from it: the warning names
/// both and the caller keeps listening.
///
/// Extractor errors are ignored inside every recogniser: to a classifier
/// "this is not that format here" is the answer, not a failure, and a
/// checksum mismatch at one offset says nothing about the next.
fn classify(
    buf: &[u8],
    recognisers: &[&'static Fingerprint],
    probing: &Probing,
) -> Option<(DeviceFamily, Evidence)> {
    let mut best: Option<(Strength, DeviceFamily, Evidence)> = None;
    let mut tied_with: Option<DeviceFamily> = None;

    for fp in recognisers {
        let Some(evidence) = (fp.recognise)(buf, probing) else {
            continue;
        };
        let rank = strength(fp, &evidence);
        if let Some((top, family, _)) = &best {
            if rank < *top {
                continue;
            }
            if rank == *top {
                if *family != fp.family {
                    tied_with = Some(fp.family);
                }
                continue;
            }
        }
        // Strictly stronger than anything seen, which also settles any tie
        // between the weaker claims below it.
        tied_with = None;
        best = Some((rank, fp.family, evidence));
    }

    let (_, family, evidence) = best?;
    if let Some(other) = tied_with {
        warn!(
            "detect: the {family} and {other} rules claim the same bytes with equal \
             confidence; identifying neither and listening on"
        );
        return None;
    }
    Some((family, evidence))
}

/// The entry a detection that settled only the family opens, and the warning
/// that says which and why.
///
/// A meter with the radio built in named its model in the name it advertises
/// (`advertised`, when that name is its alone), and its ranges are not the
/// family fallback's: a UT60BT's V position starts at 999.9mV, the UT61E+'s
/// at 2.2V (ut61-family spec §9). Every other link — an adapter, a cable —
/// gets the fingerprint's fallback.
fn family_fallback(
    found: FamilyEvidence,
    advertised: Option<&'static SelectableDevice>,
    bridge: &str,
) -> (&'static SelectableDevice, String) {
    let unnamed = format!(
        "detect: a {} frame arrived on {bridge} but the meter never named its model",
        found.family
    );
    match advertised.filter(|d| d.family == found.family) {
        Some(device) => (
            device,
            format!(
                "{unnamed}; falling back to the {} tables, the model its Bluetooth name \
                 advertises",
                device.display_name
            ),
        ),
        None => {
            let device = pin(found.fallback);
            (
                device,
                format!(
                    "{unnamed}; falling back to the {} tables",
                    device.display_name
                ),
            )
        }
    }
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
    use std::cell::{OnceCell, RefCell};
    use std::collections::VecDeque;

    /// A packet from issue #16's UT804: DC V, digits 00000, open leads.
    const ISSUE16_DC_V_ZERO: [u8; 11] = [
        0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0x31, 0x31, 0xB0, 0x31, 0x0D, 0x8A,
    ];

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

    /// A CH9325 whose meter is between packets when the listen starts: each
    /// read takes ~12 ms, the bridge's idle report interval, and carries
    /// nothing until `silent_for` has passed since the first read. Then
    /// `packet` arrives one byte per read.
    struct PacedStream {
        silent_for: Duration,
        packet: RefCell<VecDeque<u8>>,
        first_read: OnceCell<Instant>,
    }

    impl Transport for PacedStream {
        fn write(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let first = *self.first_read.get_or_init(Instant::now);
            std::thread::sleep(Duration::from_millis(12));
            if first.elapsed() < self.silent_for {
                return Ok(0);
            }
            match self.packet.borrow_mut().pop_front() {
                Some(byte) => {
                    buf[0] = byte;
                    Ok(1)
                }
                None => Ok(0),
            }
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

    /// A Bluetooth peer that advertises `name`, as `Ble` reports one.
    struct Advertising {
        inner: MockTransport,
        name: &'static str,
    }

    impl Advertising {
        fn new(name: &'static str, responses: Vec<Vec<u8>>) -> Self {
            Self {
                inner: MockTransport::new(responses),
                name,
            }
        }

        /// The name one UT60BT advertises.
        fn ut60bt(responses: Vec<Vec<u8>>) -> Self {
            Self::new("UT60BTk", responses)
        }
    }

    impl Transport for Advertising {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.inner.write(data)
        }

        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
            self.inner.read_timeout(buf, timeout_ms)
        }

        fn send_feature_report(&self, data: &[u8]) -> Result<()> {
            self.inner.send_feature_report(data)
        }

        fn advertised_name(&self) -> Option<&str> {
            Some(self.name)
        }
    }

    /// What a silent peer on the radio, whose writes land in `mock`, was sent
    /// before detection gave up on it, and whether the error called the link
    /// a meter's own radio.
    fn probes_to_silent(transport: &dyn Transport, mock: &MockTransport) -> (Vec<Vec<u8>>, bool) {
        match detect_device(transport, crate::BLUETOOTH) {
            Err(Error::DeviceNotIdentified { built_in_radio, .. }) => {
                (mock.written.take(), built_in_radio)
            }
            other => panic!("silence identified as {other:?}"),
        }
    }

    /// Every probe the Bluetooth link carries, in the order they go out.
    fn every_bluetooth_probe() -> Vec<Vec<u8>> {
        vec![
            Command::GetName.encode().to_vec(),
            set_monitor_frame(),
            UT171_CMD_CONNECT.to_vec(),
        ]
    }

    /// A meter's own name says which families it can be: a UT60BT gets the
    /// UT61+ Get Name alone, never the UT181A's or the UT171's probes, and
    /// the error calls the link its own radio.
    #[test]
    fn a_meters_own_name_keeps_the_other_families_probes_off_it() {
        let meter = Advertising::ut60bt(vec![]);
        let (sent, built_in_radio) = probes_to_silent(&meter, &meter.inner);
        assert_eq!(sent, [Command::GetName.encode().to_vec()]);
        assert!(built_in_radio);
    }

    /// An adapter's name belongs to no meter, and neither does a peer opened
    /// by address with no name heard: both get every probe the link carries.
    #[test]
    fn a_peer_no_meter_advertises_gets_every_probe() {
        let adapter = Advertising::new("UT-D07B", vec![]);
        let (sent, built_in_radio) = probes_to_silent(&adapter, &adapter.inner);
        assert_eq!(sent, every_bluetooth_probe());
        assert!(!built_in_radio);

        let unnamed = MockTransport::new(vec![]);
        let (sent, built_in_radio) = probes_to_silent(&unnamed, &unnamed);
        assert_eq!(sent, every_bluetooth_probe());
        assert!(!built_in_radio);
    }

    /// Two made-up meters with the radio built in that advertise one name,
    /// on the fingerprints of `families`' first registry entries.
    fn sharing_a_name(families: [DeviceFamily; 2]) -> Vec<&'static SelectableDevice> {
        families
            .into_iter()
            .zip(["first", "second"])
            .map(|(family, id)| {
                let base = registry::DEVICES
                    .iter()
                    .copied()
                    .find(|d| d.family == family)
                    .unwrap();
                &*Box::leak(Box::new(SelectableDevice {
                    id,
                    bluetooth_only: true,
                    bluetooth_names: &["Shared DMM"],
                    ..*base
                }))
            })
            .collect()
    }

    /// A name several meters share runs the fingerprints of those meters
    /// alone, each family's once, and none of the others'.
    #[test]
    fn a_shared_name_runs_the_fingerprints_of_the_meters_sharing_it() {
        let meter = Advertising::new("Shared DMM", vec![]);
        let named = sharing_a_name([DeviceFamily::Ut171, DeviceFamily::Ut181a]);
        let err = detect_among(&meter, crate::BLUETOOTH, &named).unwrap_err();
        assert!(matches!(
            err,
            Error::DeviceNotIdentified {
                built_in_radio: true,
                ..
            }
        ));
        // The UT181A goes first, as its constraint with the UT171 says.
        assert_eq!(
            meter.inner.written.take(),
            [set_monitor_frame(), UT171_CMD_CONNECT.to_vec()]
        );

        let meter = Advertising::new("Shared DMM", vec![]);
        let named = sharing_a_name([DeviceFamily::Ut61EPlus, DeviceFamily::Ut61EPlus]);
        assert!(detect_among(&meter, crate::BLUETOOTH, &named).is_err());
        assert_eq!(
            meter.inner.written.take(),
            [Command::GetName.encode().to_vec()]
        );
    }

    /// A name several meters share names none of them: a frame that names no
    /// model gets the family's fallback, not one of the sharers.
    #[test]
    fn a_shared_name_picks_no_model() {
        let meter = Advertising::new("Shared DMM", vec![ut61plus_reading()]);
        let named = sharing_a_name([DeviceFamily::Ut61EPlus, DeviceFamily::Ut61EPlus]);
        let detected = detect_among(&meter, crate::BLUETOOTH, &named).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name, None);
    }

    /// A UT60BT whose name reply went missing still reads with its own
    /// tables: the name it advertises already picked the model.
    #[test]
    fn a_built_in_meter_that_never_names_itself_falls_back_to_its_own_entry() {
        let meter = Advertising::ut60bt(vec![ut61plus_reading()]);
        let detected = detect_device(&meter, crate::BLUETOOTH).unwrap();
        assert_eq!(detected.device.id, "ut60bt");
        assert_eq!(detected.reported_name, None);
    }

    /// The meter's own name reply outranks the name its radio advertises.
    #[test]
    fn a_name_reply_outranks_the_advertised_name() {
        let meter = Advertising::ut60bt(vec![ack_frame(), name_frame()]);
        let detected = detect_device(&meter, crate::BLUETOOTH).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT61E+"));
    }

    /// A name reply no registry entry carries opens the advertised model on a
    /// meter with Bluetooth built in, and the UT61E+ everywhere else.
    #[test]
    fn an_unknown_name_falls_back_to_the_advertised_model() {
        let unknown = || vec![ack_frame(), test_frame_be16(b"UT216XD")];
        let meter = Advertising::ut60bt(unknown());
        let detected = detect_device(&meter, crate::BLUETOOTH).unwrap();
        assert_eq!(detected.device.id, "ut60bt");
        assert_eq!(detected.reported_name.as_deref(), Some("UT216XD"));

        let detected = detect(&MockTransport::new(unknown())).unwrap();
        assert_eq!(detected.device.id, "ut61eplus");
        assert_eq!(detected.reported_name.as_deref(), Some("UT216XD"));
    }

    /// The warning names the entry fallen back to and why; an adapter or a
    /// cable keeps the family fallback and its wording.
    #[test]
    fn the_family_fallback_says_which_tables_and_why() {
        let found = FamilyEvidence {
            family: DeviceFamily::Ut61EPlus,
            fallback: "ut61eplus",
        };
        let (device, why) = family_fallback(found, None, "CP2110");
        assert_eq!(device.id, "ut61eplus");
        assert_eq!(
            why,
            "detect: a ut61eplus frame arrived on CP2110 but the meter never named its model; \
             falling back to the UT61E+ tables"
        );
        let ut60bt = registry::find_device("ut60bt");
        let (device, why) = family_fallback(found, ut60bt, crate::BLUETOOTH);
        assert_eq!(device.id, "ut60bt");
        assert_eq!(
            why,
            "detect: a ut61eplus frame arrived on Bluetooth but the meter never named its model; \
             falling back to the UT60BT tables, the model its Bluetooth name advertises"
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

    /// The CH9325 window sends nothing at all, and the UT80x family is the
    /// only one it can identify.
    #[test]
    fn the_ch9325_window_writes_nothing() {
        // Two packets from issue #16's UT804, one byte per report with empty
        // reports between them, joined mid-packet as a real listen would be.
        const DC_V_MINUS_0_0013: [u8; 11] = [
            0xB0, 0xB0, 0xB0, 0x31, 0xB3, 0x31, 0x31, 0xB0, 0xB5, 0x0D, 0x8A,
        ];
        let reports: Vec<Vec<u8>> = ISSUE16_DC_V_ZERO[5..]
            .iter()
            .map(|&b| vec![b])
            .chain(std::iter::repeat_n(Vec::new(), 50))
            .chain(DC_V_MINUS_0_0013.iter().map(|&b| vec![b]))
            .collect();
        let mock = MockTransport::new(reports);
        assert_eq!(detect_device(&mock, "CH9325").unwrap().device.id, "ut804");
        assert!(mock.written.borrow().is_empty(), "nothing is sent");
    }

    /// The UT804 sends a packet every 656 ms, so a listen can start just
    /// after one and hear nothing for longer than a probe window. The window
    /// on a bridge that sends nothing still catches the next packet.
    #[test]
    fn a_listen_only_window_waits_out_a_slow_meters_gap() {
        let meter = PacedStream {
            silent_for: Duration::from_millis(700),
            packet: RefCell::new(ISSUE16_DC_V_ZERO.into()),
            first_read: OnceCell::new(),
        };
        assert!(meter.silent_for > WINDOW, "the gap outlasts a probe window");
        assert_eq!(detect_device(&meter, "CH9325").unwrap().device.id, "ut804");
    }

    #[test]
    fn silence_is_not_identified() {
        let mock = MockTransport::new(vec![]);
        let err = detect(&mock).unwrap_err();
        assert!(matches!(
            err,
            Error::DeviceNotIdentified {
                bridge: "CP2110",
                built_in_radio: false
            }
        ));
    }

    /// The full cascade as it comes out of the registry and the rules: Get
    /// Name first (the registry's first entry, and the most verified probe),
    /// SET_MONITOR before the UT171 connect because the UT171 asks to be sent
    /// after it, then the VC-890 ack burst and poll. Pinned byte for byte, so
    /// a registry reshuffle that moves a probe fails here.
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

    /// The registry, not this module, says which rules a bridge gets, and in
    /// which order: exactly the families `devices_on_bridge` yields there,
    /// each one once though six registry entries share the UT61+'s.
    #[test]
    fn the_registry_decides_which_fingerprints_a_bridge_gets() {
        let families = |bridge: &str| -> Vec<DeviceFamily> {
            fingerprints_on(bridge, &[])
                .iter()
                .map(|fp| fp.family)
                .collect()
        };
        // The CH9325 carries the UT80x family (UT803/UT804, UT71, VC9x0)
        // and nothing else.
        assert_eq!(families("CH9325"), vec![DeviceFamily::Ut80x]);
        // The UT-D07B gets the probes of the families UNI-T lists for it, and
        // not the UT80x's — the UT71 is on the UT-D07A, a different adapter.
        // The ZOTEK meters have the radio built in.
        assert_eq!(
            families(crate::BLUETOOTH),
            vec![
                DeviceFamily::Ut61EPlus,
                DeviceFamily::Ut171,
                DeviceFamily::Ut181a,
                DeviceFamily::Zotek,
            ],
            "the registry places these families on the UT-D07B"
        );
        // Every AB CD family: the UT61+, the two bench meters, the LE16 twins
        // and the Voltcraft pair.
        assert_eq!(
            families("CP2110"),
            vec![
                DeviceFamily::Ut61EPlus,
                DeviceFamily::Ut8802,
                DeviceFamily::Ut8803,
                DeviceFamily::Ut171,
                DeviceFamily::Ut181a,
                DeviceFamily::Vc880,
                DeviceFamily::Vc890,
            ]
        );
        // The CH9329 carries the three handhelds seen on it — a UT61B+ is
        // verified there (issue #19) — and not the CP2110-only bench meters.
        assert_eq!(
            families("CH9329"),
            vec![
                DeviceFamily::Ut61EPlus,
                DeviceFamily::Ut171,
                DeviceFamily::Ut181a,
            ]
        );
        for bridge in ["CP2110", "CH9329"] {
            assert!(!families(bridge).contains(&DeviceFamily::Ut80x), "{bridge}");
        }
        assert!(fingerprints_on("no such bridge", &[]).is_empty());
    }

    /// The `0x0A` constraint, derived: whatever the registry order, the
    /// UT181A's SET_MONITOR goes out before the UT171 connect — on whichever
    /// cable the two share.
    #[test]
    fn the_ut181a_is_probed_before_the_ut171() {
        for bridge in ["CP2110", "CH9329"] {
            let order: Vec<DeviceFamily> = probe_order(&fingerprints_on(bridge, &[]))
                .iter()
                .map(|fp| fp.family)
                .collect();
            let at = |family| {
                order
                    .iter()
                    .position(|f| *f == family)
                    .unwrap_or_else(|| panic!("{family} sends nothing on {bridge}"))
            };
            assert!(
                at(DeviceFamily::Ut181a) < at(DeviceFamily::Ut171),
                "{bridge}"
            );
        }
    }

    /// Recognises nothing: a synthetic fingerprint is only ever asked what it
    /// sends and what it waits for.
    fn recognise_nothing(_buf: &[u8], _probing: &Probing) -> Option<Evidence> {
        None
    }

    fn send_nothing(_transport: &dyn Transport) -> Result<()> {
        Ok(())
    }

    /// A trigger that waits for a family no bridge here carries.
    static WAITS_FOR_AN_ABSENT_FAMILY: Fingerprint = Fingerprint {
        family: DeviceFamily::Vc890,
        label: "waits for the mock",
        trigger: Some(send_nothing),
        send_after: &[DeviceFamily::Mock],
        checksummed: true,
        recognise: recognise_nothing,
    };

    static FIRST_IN_A_CYCLE: Fingerprint = Fingerprint {
        family: DeviceFamily::Ut171,
        label: "first in a cycle",
        trigger: Some(send_nothing),
        send_after: &[DeviceFamily::Ut181a],
        checksummed: true,
        recognise: recognise_nothing,
    };

    static SECOND_IN_A_CYCLE: Fingerprint = Fingerprint {
        family: DeviceFamily::Ut181a,
        label: "second in a cycle",
        trigger: Some(send_nothing),
        send_after: &[DeviceFamily::Ut171],
        checksummed: true,
        recognise: recognise_nothing,
    };

    /// A family that is not on this bridge cannot be waited for: there is no
    /// trigger of its to go out first, so the constraint is moot.
    #[test]
    fn a_send_after_naming_an_absent_family_is_ignored() {
        let order = probe_order(&[&WAITS_FOR_AN_ABSENT_FAMILY]);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].label, "waits for the mock");
    }

    /// Two triggers each waiting for the other satisfy no order. Registry
    /// order still sends both, which beats sending neither.
    #[test]
    fn a_cycle_falls_back_to_registry_order() {
        let order: Vec<&str> = probe_order(&[&FIRST_IN_A_CYCLE, &SECOND_IN_A_CYCLE])
            .iter()
            .map(|fp| fp.label)
            .collect();
        assert_eq!(order, vec!["first in a cycle", "second in a cycle"]);
    }

    /// A rule that always claims `evidence`, for the ranking tests.
    macro_rules! claiming {
        ($name:ident, $family:expr, $checksummed:expr, $evidence:expr) => {
            static $name: Fingerprint = Fingerprint {
                family: $family,
                label: stringify!($name),
                trigger: None,
                send_after: &[],
                checksummed: $checksummed,
                recognise: |_buf: &[u8], _probing: &Probing| Some($evidence),
            };
        };
    }

    claiming!(
        NAMED,
        DeviceFamily::Ut61EPlus,
        true,
        Evidence::Model {
            id: "ut61b+",
            reported_name: Some("UT61B+".to_string()),
        }
    );
    claiming!(
        CHECKSUMMED,
        DeviceFamily::Ut8803,
        true,
        Evidence::Model {
            id: "ut8803",
            reported_name: None,
        }
    );
    claiming!(
        FAMILY_ONLY,
        DeviceFamily::Vc880,
        true,
        Evidence::FamilyOnly { fallback: "vc880" }
    );
    claiming!(
        UNCHECKSUMMED,
        DeviceFamily::Ut8802,
        false,
        Evidence::Model {
            id: "ut8802",
            reported_name: None,
        }
    );
    claiming!(
        RIVAL_CHECKSUMMED,
        DeviceFamily::Ut171,
        true,
        Evidence::Model {
            id: "ut171",
            reported_name: None,
        }
    );

    /// The ranking, in place of the old hand-ordered table: a named model
    /// beats a checksummed one, which beats a settled family, which beats a
    /// model claimed with no checksum behind it. Order in the slice is
    /// deliberately the reverse of the ranking.
    #[test]
    fn the_strongest_evidence_wins_whatever_the_order() {
        let rules: [&'static Fingerprint; 4] = [&UNCHECKSUMMED, &FAMILY_ONLY, &CHECKSUMMED, &NAMED];
        let probing = Probing::default();
        let family =
            |rules: &[&'static Fingerprint]| classify(b"", rules, &probing).map(|(f, _)| f);

        assert_eq!(family(&rules), Some(DeviceFamily::Ut61EPlus));
        assert_eq!(family(&rules[..3]), Some(DeviceFamily::Ut8803));
        assert_eq!(family(&rules[..2]), Some(DeviceFamily::Vc880));
        assert_eq!(family(&rules[..1]), Some(DeviceFamily::Ut8802));
    }

    /// Two families claiming the same bytes just as strongly is a
    /// coincidence, not an identification — the window keeps listening.
    #[test]
    fn two_families_tied_at_the_top_identify_nothing() {
        let probing = Probing::default();
        assert!(classify(b"", &[&CHECKSUMMED, &RIVAL_CHECKSUMMED], &probing).is_none());
        // The tie is only between the leaders: something stronger settles it.
        assert_eq!(
            classify(b"", &[&CHECKSUMMED, &RIVAL_CHECKSUMMED, &NAMED], &probing).map(|(f, _)| f),
            Some(DeviceFamily::Ut61EPlus)
        );
    }
}
