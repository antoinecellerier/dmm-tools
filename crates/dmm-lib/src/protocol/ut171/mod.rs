//! UT171A/B/C protocol.
//!
//! Streaming protocol: user must manually enable communication ("Usb" in
//! the meter's setup) on the meter. Once the connect frame goes out, the
//! meter streams measurement frames.
//!
//! Frame format: AB CD len_lo len_hi payload chk_lo chk_hi. The LE16 length
//! counts payload + checksum, as on the UT181A.
//! Checksum = 16-bit LE sum of the length bytes + payload bytes.
//!
//! What each byte means is how UNI-T's UT171C software reads it; no UT171
//! has been read yet (`docs/research/ut171/verification.md`). The parser is
//! in [`parse`].
//!
//! See docs/research/ut171/reverse-engineered-protocol.md

mod command;
pub(crate) mod devices;
mod parse;

use crate::error::Result;
use crate::measurement::Measurement;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    Choice, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol, Setting,
    Stability,
};
use crate::transport::Transport;
use log::{debug, warn};
use parse::{MAX_MEASUREMENT_PAYLOAD, RESPONSE_MEASUREMENT, parse_measurement};
use std::ops::RangeInclusive;

#[cfg(any(test, feature = "test-support"))]
pub(crate) use parse::all_labels;
#[cfg(test)]
pub(crate) use parse::all_units;

/// Family label for [`report_unknown`].
const FAMILY: &str = "ut171";

/// Payload lengths of the short replies of the measurement type that spec
/// §10's cross-reference records (`[UNVERIFIED]`).
/// "Lengths 4-8" may count the payload or the length field (payload +
/// checksum), so both readings are covered.
const SHORT_REPLY_PAYLOAD: RangeInclusive<usize> = 2..=8;

/// The other frame types the meter sends (spec §5.10–5.11): `0x01` OK, ER or NO in
/// answer to a command, `0x72` the count, memory and device-information
/// replies, `0x03` a stored record. The stream skips them quietly.
const OTHER_REPLIES: [u8; 3] = [0x01, 0x72, 0x03];

/// Connect (command 0x0A, 1 = start streaming), in the UT181A's framing
/// (`docs/research/ut171/reverse-engineered-protocol.md` §4.3).
///
/// `pub(crate)` because detection sends it too.
pub(crate) fn connect_frame() -> Vec<u8> {
    framing::build_abcd_le16(&[0x0A, 0x01])
}

/// Pause (command 0x0A, 0 = stop streaming), spec §4.3.
fn pause_frame() -> Vec<u8> {
    framing::build_abcd_le16(&[0x0A, 0x00])
}

/// Protocol implementation for the UT171A/B/C.
pub(crate) struct Ut171Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// What the last reading showed, for the commands that depend on it.
    last: Option<command::Shown>,
}

impl Default for Ut171Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut171Protocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            last: None,
            profile: DeviceProfile {
                family_name: "UT171",
                id: "ut171",
                stability: Stability::Experimental,
                supported_commands: command::UT171_COMMANDS,
                // The square wave's duty and width (spec §5.9).
                max_aux_values: 2,
                verification_issue: Some(4),
                meter_keys: command::METER_KEYS,
            },
        }
    }
}

impl Protocol for Ut171Protocol {
    fn delivery(&self) -> crate::protocol::Delivery {
        crate::protocol::Delivery::Streamed
    }

    fn discard_input(&mut self, transport: &dyn Transport) -> Result<()> {
        self.rx_buf.clear();
        // What the meter shows now is the next reading's to say.
        self.last = None;
        crate::protocol::framing::discard_queued(transport)
    }

    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // Send connect command to start streaming.
        // User must also enable communication on the meter.
        debug!("ut171: sending connect command");
        transport.write(&connect_frame())?;
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            // UT171 framing is identical to UT181A: 2-byte LE length =
            // payload + checksum, LE16 sum. Verified against the connect
            // command (AB CD 04 00 0A 01 0F 00: len 4, sum 04+00+0A+01 =
            // 0x000F LE), spec §3.4.
            framing::extract_frame_abcd_2byte_le16,
            is_measurement_frame,
            FrameErrorRecovery::SkipAndRetry {
                header: &framing::HEADER,
            },
            "ut171",
        )?;
        let measurement = parse_measurement(&payload)?;
        self.last = Some(command::Shown::of(&measurement));
        Ok(measurement)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        self.send_key(transport, command)
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        self.setting_choices(setting, current)
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        self.select_setting(transport, setting, id)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::flags::Flag;
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{CaptureStep, Expect, Need, RangeExpect, command};

        let [vdc, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
            Volts::VDc,
            CaptureStep::basic("vdc", "Set meter to V DC"),
            Ohms::Word,
            CaptureStep::basic(
                "ohm",
                "Set meter to Resistance. Leave leads open (should show OL).",
            ),
        );

        vec![
            vdc,
            dcv_short,
            dcv_negative,
            ohm,
            ohm_body,
            ohm_short,
            // Capacitance and diode join the gate: 0x0B and 0x0D rest on
            // UNI-T's software alone, and spec §10 records a reading of 0x0B
            // as diode (spec §6). The operator confirms them at the step,
            // before the tool trusts the mode table enough to switch modes
            // by it.
            CaptureStep::basic("cap", "Set meter to Capacitance")
                .gate()
                .expect(Expect::mode("Capacitance")),
            CaptureStep::basic("diode", "Set meter to Diode")
                .gate()
                .expect(Expect::mode("Diode")),
            CaptureStep::basic("vac", "Set meter to V AC").expect(Expect::mode("V AC")),
            // Spec §6: the manual reaches the V-to-frequency converter by
            // long-pressing SELECT in AC V.
            CaptureStep::basic(
                "vfc",
                "V AC mode: long-press SELECT for VFC (V\u{2192}Hz converter)",
            )
            .expect(Expect::mode("VFC")),
            CaptureStep::basic("vacdc", "Set meter to V AC+DC").expect(Expect::mode("V AC+DC")),
            CaptureStep::basic("lozv", "Set meter to LoZ V~ (low-impedance AC V)")
                .expect(Expect::mode("LoZ V~")),
            CaptureStep::basic("mvdc", "Set meter to mV DC").expect(Expect::mode("mV DC")),
            CaptureStep::basic("mvac", "Set meter to mV AC").expect(Expect::mode("mV AC")),
            CaptureStep::basic("mvacdc", "Set meter to mV AC+DC").expect(Expect::mode("mV AC+DC")),
            CaptureStep::basic("cont", "Set meter to Continuity")
                .expect(Expect::mode("Continuity")),
            CaptureStep::basic("tempc", "Set meter to Temperature C (if available)")
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°C")),
            CaptureStep::basic("tempf", "Set meter to Temperature F (if available)")
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°F")),
            CaptureStep::basic("ns", "Set meter to Conductance nS (if available)")
                .expect(Expect::mode("nS")),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Hz")),
            // Duty has no mode byte of its own: the Hz% key steps the unit.
            CaptureStep::basic("duty", "Frequency (Hz): press Hz% for duty cycle (%)")
                .expect(Expect::mode("Hz")),
            CaptureStep::basic("uadc", "Set meter to µA DC").expect(Expect::mode("µA DC")),
            CaptureStep::basic("uaac", "Set meter to µA AC").expect(Expect::mode("µA AC")),
            CaptureStep::basic("uaacdc", "Set meter to µA AC+DC").expect(Expect::mode("µA AC+DC")),
            CaptureStep::basic("madc", "Set meter to mA DC").expect(Expect::mode("mA DC")),
            CaptureStep::basic("maac", "Set meter to mA AC").expect(Expect::mode("mA AC")),
            CaptureStep::basic("maacdc", "Set meter to mA AC+DC").expect(Expect::mode("mA AC+DC")),
            CaptureStep::basic("ma420", "mA: long-press SELECT for % (4-20 mA loop)")
                .expect(Expect::mode("% 4-20mA")),
            CaptureStep::basic("adc", "Set meter to A DC").expect(Expect::mode("A DC")),
            CaptureStep::basic("aac", "Set meter to A AC").expect(Expect::mode("A AC")),
            CaptureStep::basic("aacdc", "Set meter to A AC+DC").expect(Expect::mode("A AC+DC")),
            // The 600 A clamp and the square wave are UT171C-only (spec §1.1).
            CaptureStep::basic("a600dc", "Set meter to 600A DC (clamp, UT171C only)")
                .expect(Expect::mode("600A DC")),
            CaptureStep::basic("a600ac", "Set meter to 600A AC (clamp, UT171C only)")
                .expect(Expect::mode("600A AC")),
            CaptureStep::basic(
                "square_wave",
                "µA: long-press SELECT for the square-wave output (UT171C only)",
            )
            .expect(Expect::mode("Square wave")),
            // What an NCV frame carries past the mode is unknown (spec §5.8).
            CaptureStep::basic("ncv", "Set meter to NCV. Hold near a live wire.")
                .needs(&[Need::LiveWire])
                .expect(Expect::mode("NCV")),
            // The keys, sent by the tool once the meter shows the state they
            // start from, and the flag each sets by UNI-T's software (spec
            // §4.7, §5.3). RANGE comes before REL, which forces manual range
            // (manual p.10).
            CaptureStep::with_command("hold", "V DC: we will send HOLD.", command::HOLD, 3)
                .starts_from(Expect::mode("V DC"))
                .expect(Expect::new().flags(&[(Flag::Hold, true)])),
            CaptureStep::with_command(
                "hold_off",
                "We will send HOLD again to turn it off.",
                command::HOLD,
                3,
            )
            .expect(Expect::new().flags(&[(Flag::Hold, false)])),
            CaptureStep::with_command(
                "manual_range",
                "V DC: we will switch to a manual range.",
                command::RANGE,
                3,
            )
            .starts_from(Expect::mode("V DC").range(RangeExpect::Auto))
            .expect(Expect::new().range(RangeExpect::Manual)),
            CaptureStep::with_command("auto", "We will set auto-range.", command::AUTO, 3)
                .expect(Expect::new().range(RangeExpect::Auto)),
            CaptureStep::with_command("rel", "V DC: we will send REL.", command::REL, 3)
                .starts_from(Expect::mode("V DC"))
                .expect(Expect::new().flags(&[(Flag::Rel, true)])),
            CaptureStep::with_command("rel_off", "We will leave REL.", command::EXIT_REL, 3)
                .expect(Expect::new().flags(&[(Flag::Rel, false)])),
            CaptureStep::with_command("minmax", "We will enter MAX/MIN.", command::MINMAX, 3)
                .expect(Expect::new().flags(&[(Flag::Max, true)])),
            CaptureStep::with_command(
                "minmax_off",
                "We will leave MAX/MIN.",
                command::EXIT_MINMAX,
                3,
            )
            .expect(Expect::new().flags(&[
                (Flag::Max, false),
                (Flag::Avg, false),
                (Flag::Min, false),
            ])),
            CaptureStep::with_command("peak", "V AC: we will enter Peak hold.", command::PEAK, 3)
                .starts_from(Expect::mode("V AC"))
                .expect(Expect::mode("Peak V AC")),
            CaptureStep::with_command("peak_off", "We will leave Peak hold.", command::PEAK, 3)
                .expect(Expect::mode("V AC")),
            CaptureStep::with_command(
                "hz_key",
                "V AC: we will press Hz% to step the second display.",
                "hz",
                3,
            )
            .starts_from(Expect::mode("V AC")),
        ]
    }
}

/// The stream filter: whether `payload` is a measurement frame, the one
/// shape [`parse_measurement`] reads.
///
/// The meter's other frames (spec §5.10–5.11) and the short replies of the
/// measurement type are skipped quietly. Anything else — its checksum
/// already held, so it is the meter's — is reported.
fn is_measurement_frame(payload: &[u8]) -> bool {
    if parse::is_measurement(payload) {
        return true;
    }
    let len = payload.len();
    match payload.first() {
        Some(&RESPONSE_MEASUREMENT) if SHORT_REPLY_PAYLOAD.contains(&len) => {}
        Some(kind) if OTHER_REPLIES.contains(kind) => {}
        Some(kind) => report_unknown(
            FAMILY,
            "frame",
            format_args!("type {kind:#04x}, {len} bytes"),
        ),
        None => report_unknown(FAMILY, "frame", format_args!("{len} bytes")),
    }
    false
}

/// Detection for the UT171.
///
/// The UT181A speaks the same framing with the same measurement type byte, so
/// this rule claims only what its twin cannot have sent — see
/// [`recognise`] and `docs/detection-design.md`.
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Ut171,
    label: "ut171 connect",
    trigger: Some(send_connect),
    // The connect frame is UT181A opcode `0x0A` (start recording), so a
    // UT181A sharing this cable gets its own window — and its own chance to
    // be identified — before this one goes out.
    send_after: &[DeviceFamily::Ut181a],
    checksummed: true,
    claims_info_links: false,
    recognise,
};

/// Start the stream — the same frame [`Protocol::init`] writes.
fn send_connect(transport: &dyn Transport) -> Result<()> {
    transport.write(&connect_frame())
}

/// A measurement frame this meter could have sent, and that its twin the
/// UT181A could not.
///
/// Two things rule the UT181A out, both local to this family: a payload
/// longer than [`MAX_MEASUREMENT_PAYLOAD`] is a shape no UT171 frame takes,
/// and a frame arriving right after SET_MONITOR is the one that command just
/// asked for.
fn recognise(buf: &[u8], probing: &Probing) -> Option<Evidence> {
    for start in framing::abcd_header_offsets(buf) {
        let Ok(Some((payload, _))) = framing::extract_frame_abcd_2byte_le16(&buf[start..]) else {
            continue;
        };
        if !parse::is_measurement(&payload) {
            continue;
        }
        if payload.len() > MAX_MEASUREMENT_PAYLOAD {
            debug!(
                "detect: an LE16 measurement payload of {} bytes is longer than a UT171 frame",
                payload.len()
            );
            continue;
        }
        if probing.last() == Some(DeviceFamily::Ut181a) {
            // SET_MONITOR has just gone out and this is what came back: the
            // UT181A streams the same shape, and the frame it was asked for
            // is its own.
            continue;
        }
        if !probing.has_sent(DeviceFamily::Ut171) {
            // Streaming before the connect frame went out: a UT171 left
            // connected, or a UT181A left in monitor mode by an earlier
            // session. The UT171 is the likelier one, but say so out loud.
            warn!(
                "detect: a short LE16 measurement frame arrived before any trigger; \
                 assuming a UT171 (a UT181A left streaming looks the same)"
            );
        }
        return Some(Evidence::Model {
            id: devices::UT171.id,
            reported_name: None,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse::tests::{payload, with_aux};
    use super::*;
    use crate::protocol::capture_reports;
    use crate::transport::mock::MockTransport;

    /// An ordinary V DC reading's payload.
    fn vdc() -> Vec<u8> {
        payload(0x02, 1, 1.2345, 0x40, 0x00)
    }

    // --- Detection (crate::detect) ---------------------------------------

    fn recognised(buf: &[u8], probing: &Probing) -> Option<Evidence> {
        (FINGERPRINT.recognise)(buf, probing)
    }

    const UT171: Option<Evidence> = Some(Evidence::Model {
        id: "ut171",
        reported_name: None,
    });

    /// A measurement frame short enough for either family is a UT171 once the
    /// connect frame has gone out: SET_MONITOR went out before it and a
    /// UT181A would have answered that one. The longest UT171 frame and a
    /// four-byte NCV frame are as well.
    #[test]
    fn a_measurement_frame_after_the_connect_is_a_ut171() {
        let probing = Probing {
            sent: vec![DeviceFamily::Ut171],
            ..Probing::default()
        };
        let mut longest = with_aux(vdc(), 1.0, 0x10, 0x12);
        longest[1] |= 0x0A;
        longest.extend_from_slice(&[0; 6]);
        assert_eq!(longest.len(), MAX_MEASUREMENT_PAYLOAD);
        for payload in [vdc(), longest, vec![RESPONSE_MEASUREMENT, 0, 0, 0x1A]] {
            let frame = framing::build_abcd_le16(&payload);
            assert_eq!(recognised(&frame, &probing), UT171, "{payload:02X?}");
        }
    }

    /// The same frame before any trigger: a UT171 left connected is the
    /// likelier source than a UT181A left streaming, and the engine warns.
    #[test]
    fn a_measurement_frame_before_any_trigger_is_still_a_ut171() {
        let frame = framing::build_abcd_le16(&vdc());
        assert_eq!(recognised(&frame, &Probing::default()), UT171);
    }

    /// Past the longest UT171 payload (§3.4) no UT171 frame exists — that
    /// length belongs to the UT181A, whose framing is the same.
    #[test]
    fn a_payload_longer_than_the_longest_frame_is_not_a_ut171() {
        let mut payload = vdc();
        payload.resize(MAX_MEASUREMENT_PAYLOAD + 1, 0);
        let frame = framing::build_abcd_le16(&payload);
        for probing in [
            Probing::default(),
            Probing {
                sent: vec![DeviceFamily::Ut171],
                ..Probing::default()
            },
        ] {
            assert_eq!(recognised(&frame, &probing), None);
        }
    }

    /// The UT181A streams this very shape, and SET_MONITOR was the last thing
    /// sent: the frame it asked for is its own.
    #[test]
    fn a_measurement_frame_right_after_set_monitor_is_not_a_ut171() {
        let frame = framing::build_abcd_le16(&vdc());
        let probing = Probing {
            sent: vec![DeviceFamily::Ut181a],
            ..Probing::default()
        };
        assert_eq!(recognised(&frame, &probing), None);
    }

    /// Replies and short frames name no model: detection takes the shapes
    /// the stream filter takes.
    #[test]
    fn a_frame_that_is_no_measurement_identifies_nothing() {
        for payload in [
            vec![0x01, b'O', b'K'],
            vec![RESPONSE_MEASUREMENT, 0x00, 0x00, 0x00],
            vdc()[..10].to_vec(),
        ] {
            let frame = framing::build_abcd_le16(&payload);
            assert_eq!(recognised(&frame, &Probing::default()), None);
        }
    }

    /// The command frames are the captured bytes (spec §4.3).
    #[test]
    fn command_frames_match_the_captures() {
        assert_eq!(
            connect_frame(),
            [0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x01, 0x0F, 0x00]
        );
        assert_eq!(
            pause_frame(),
            [0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x00, 0x0E, 0x00]
        );
    }

    // --- The stream filter -------------------------------------------------

    /// Short replies of the measurement type and the meter's documented
    /// replies are skipped on the way to a reading without a word.
    #[test]
    fn replies_on_the_stream_report_nothing() {
        let mut proto = Ut171Protocol::new();
        let mut frames: Vec<Vec<u8>> = SHORT_REPLY_PAYLOAD
            .map(|len| {
                let mut reply = vec![0x00; len];
                reply[0] = RESPONSE_MEASUREMENT;
                // Not the NCV mode, which makes a four-byte measurement.
                if len > 3 {
                    reply[3] = 0x02;
                }
                framing::build_abcd_le16(&reply)
            })
            .collect();
        for reply in [
            vec![0x01, b'O', b'K'],
            vec![0x01, b'E', b'R'],
            vec![0x01, b'N', b'O'],
            vec![0x72, 0x11, 0x00, 0x05],
            vec![0x03; 25],
        ] {
            frames.push(framing::build_abcd_le16(&reply));
        }
        frames.push(framing::build_abcd_le16(&vdc()));
        let mock = MockTransport::new(frames);
        let (first, reports) = capture_reports(|| {
            proto.init(&mock).unwrap();
            proto.request_measurement(&mock)
        });
        assert_eq!(first.unwrap().mode, "V DC");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A frame of another type, or of the measurement type but neither a
    /// short reply nor a reading, is skipped and reported.
    #[test]
    fn unknown_frames_on_the_stream_are_reported() {
        let mut proto = Ut171Protocol::new();
        let mut short = vec![0x00; 10];
        short[0] = RESPONSE_MEASUREMENT;
        let mock = MockTransport::new(vec![
            framing::build_abcd_le16(&[0x04, 0x00]),
            framing::build_abcd_le16(&short),
            framing::build_abcd_le16(&[]),
            framing::build_abcd_le16(&vdc()),
        ]);
        let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().display_raw.as_deref(), Some("1.2345"));
        assert_eq!(
            reports,
            [
                "ut171: unrecognised frame: type 0x04, 2 bytes",
                "ut171: unrecognised frame: type 0x02, 10 bytes",
                "ut171: unrecognised frame: 0 bytes",
            ]
        );
    }

    /// Every mode the table lists, at each of its rows, with an aux value:
    /// none of it is reported, and every label is in the catalog.
    fn documented_payloads() -> Vec<Vec<u8>> {
        let mut payloads = Vec::new();
        for mode in (0x01..=0x0F)
            .chain(0x11..=0x19)
            .chain([0x1B, 0x1C, 0x1E, 0x1F])
        {
            let rows = parse::range_rows(mode).len().max(1) as u8;
            for range in 1..=rows {
                payloads.push(payload(mode, range, 1.5, 0x10, 0x00));
                payloads.push(with_aux(
                    payload(mode, range, 1.5, 0x10, 0x01),
                    0.05,
                    0x30,
                    0x13,
                ));
            }
        }
        payloads
    }

    #[test]
    fn documented_payloads_report_nothing() {
        for payload in documented_payloads() {
            let (m, reports) = capture_reports(|| parse_measurement(&payload));
            assert!(m.is_ok(), "{payload:02X?}: {m:?}");
            assert!(reports.is_empty(), "{payload:02X?}: {reports:?}");
        }
    }

    #[test]
    fn the_documented_payloads_labels_are_listed() {
        let all = crate::text::listed();
        let protocol = Ut171Protocol::new();
        for payload in documented_payloads() {
            let m = parse_measurement(&payload).unwrap();
            crate::text::assert_listed(all, &protocol, &m, devices::UT171.id);
        }
    }
}
