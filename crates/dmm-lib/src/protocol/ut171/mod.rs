//! UT171A/B/C protocol.
//!
//! Streaming protocol: user must manually enable "Communication ON" on the meter.
//! No trigger byte needed — device streams 22-byte or 28-byte measurement frames.
//!
//! Frame format: AB CD len payload chk_lo chk_hi
//! Length is a 1-byte uint8 = payload size (does NOT include checksum).
//! Checksum = 16-bit LE sum of length byte + payload bytes.
//!
//! Values are IEEE 754 float32 (LE). 26 measurement modes.
//!
//! Based on Ghidra decompilation of UT171C.exe and USB captures.
//! See docs/research/ut171/reverse-engineered-protocol.md

pub(crate) mod devices;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Probing, Protocol, Stability,
    check_len, unknown_mode,
};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::ops::RangeInclusive;

/// Family label for [`report_unknown`].
const FAMILY: &str = "ut171";

/// Look up the human-readable range label for a (mode, range) pair.
///
/// Spec §5.4 lists vendor-extracted range tables from UT171C.exe strings.
/// Range byte is raw and 1-based (`0` = auto-range, no specific label).
/// Only the modes whose range tables are explicitly documented in the spec
/// are returned here; others get `None` and fall through to an empty label,
/// so we never fabricate range bounds that the vendor string dump did not
/// prove. AC variants of mADC/uADC/ADC share the same magnitudes as the DC
/// tables per §5.4; AC+DC variants are not documented and intentionally
/// return `None`.
fn lookup_range(mode_byte: u8, range_byte: u8) -> Option<&'static str> {
    if range_byte == 0 {
        return None;
    }
    match (mode_byte, range_byte) {
        // 0x08 Continuity (BEEP)
        (0x08, 1) => Some("600Ω"),
        // 0x09 Capacitance — continuous indexing nF/µF/mF
        (0x09, 1) => Some("6nF"),
        (0x09, 2) => Some("60nF"),
        (0x09, 3) => Some("600nF"),
        (0x09, 4) => Some("6µF"),
        (0x09, 5) => Some("60µF"),
        (0x09, 6) => Some("600µF"),
        (0x09, 7) => Some("6mF"),
        (0x09, 8) => Some("60mF"),
        // 0x0E Conductance (nS)
        (0x0E, 1) => Some("60nS"),
        // 0x0F Frequency — continuous indexing Hz/kHz/MHz
        (0x0F, 1) => Some("60Hz"),
        (0x0F, 2) => Some("600Hz"),
        (0x0F, 3) => Some("6kHz"),
        (0x0F, 4) => Some("60kHz"),
        (0x0F, 5) => Some("600kHz"),
        (0x0F, 6) => Some("6MHz"),
        (0x0F, 7) => Some("60MHz"),
        // 0x11 µA DC, 0x12 µA AC (same magnitudes per §5.4)
        (0x11 | 0x12, 1) => Some("600µA"),
        (0x11 | 0x12, 2) => Some("6000µA"),
        // 0x14 mA DC, 0x15 mA AC
        (0x14 | 0x15, 1) => Some("60mA"),
        (0x14 | 0x15, 2) => Some("600mA"),
        // 0x17 A DC
        (0x17, 1) => Some("6A"),
        (0x17, 2) => Some("20A"),
        _ => None,
    }
}

/// Look up mode name and unit from mode byte.
/// Returns `(Cow::Borrowed(name), unit)` for known modes,
/// or `(Cow::Owned("Unknown(0xNN)"), "")` for unknown bytes.
///
/// Mode byte values from Ghidra analysis of UT171C.exe.
fn lookup_mode(byte: u8) -> (Cow<'static, str>, &'static str) {
    match byte {
        0x01 => (Cow::Borrowed("LoZ V~"), "V"),
        0x02 => (Cow::Borrowed("V DC"), "V"),
        0x03 => (Cow::Borrowed("V AC"), "V"),
        0x04 => (Cow::Borrowed("V AC+DC"), "V"),
        0x05 => (Cow::Borrowed("mV DC"), "mV"),
        0x06 => (Cow::Borrowed("mV AC"), "mV"),
        0x07 => (Cow::Borrowed("mV AC+DC"), "mV"),
        0x08 => (Cow::Borrowed("Continuity"), "Ω"),
        0x09 => (Cow::Borrowed("Capacitance"), "F"),
        0x0A => (Cow::Borrowed("Ω"), "Ω"),
        0x0B => (Cow::Borrowed("Diode"), "V"),
        0x0C => (Cow::Borrowed("°C"), "°C"),
        0x0D => (Cow::Borrowed("°F"), "°F"),
        0x0E => (Cow::Borrowed("nS"), "nS"),
        0x0F => (Cow::Borrowed("Hz"), "Hz"),
        0x10 => (Cow::Borrowed("Duty %"), "%"),
        0x11 => (Cow::Borrowed("µA DC"), "µA"),
        0x12 => (Cow::Borrowed("µA AC"), "µA"),
        0x13 => (Cow::Borrowed("µA AC+DC"), "µA"),
        0x14 => (Cow::Borrowed("mA DC"), "mA"),
        0x15 => (Cow::Borrowed("mA AC"), "mA"),
        0x16 => (Cow::Borrowed("mA AC+DC"), "mA"),
        0x17 => (Cow::Borrowed("A DC"), "A"),
        0x18 => (Cow::Borrowed("A AC"), "A"),
        0x19 => (Cow::Borrowed("A AC+DC"), "A"),
        0x1A => (Cow::Borrowed("VFC"), "V"),
        0x1B => (Cow::Borrowed("% 4-20mA"), "%"),
        0x1C => (Cow::Borrowed("600A DC"), "A"),
        0x1D => (Cow::Borrowed("600A AC"), "A"),
        0x24 => (Cow::Borrowed("NCV"), ""),
        _ => {
            // §6 lists no other byte. Only the parser calls this.
            report_unknown(FAMILY, "mode byte", format_args!("{byte:#04x}"));
            (unknown_mode(byte), "")
        }
    }
}

/// Display unit for a (mode, range) pair.
///
/// The resistance float is range-relative: gulux/Uni-T-CP2110
/// (capture-driven against real hardware) multiplies by 1000 when the
/// range byte is >= 2 and again when >= 5 — i.e. ranges 2-4 read in kΩ
/// and 5-6 in MΩ. We keep the wire value and put the magnitude in the
/// unit instead. Scaling for other modes (capacitance, conductance) is
/// [UNVERIFIED]; they keep the base unit.
fn display_unit(mode_byte: u8, range_byte: u8, base_unit: &'static str) -> &'static str {
    match (mode_byte, range_byte) {
        (0x0A, 2..=4) => "kΩ",
        (0x0A, r) if r >= 5 => "MΩ",
        _ => base_unit,
    }
}

const UT171_COMMANDS: &[&str] = &["connect", "pause"];

/// Response type of a measurement frame; the meter's short replies share it
/// (gulux observes lengths 4-8), which is why the stream filter asks for a
/// measurement-sized payload as well
/// (`docs/research/ut171/reverse-engineered-protocol.md` §3.4).
const RESPONSE_MEASUREMENT: u8 = 0x02;

/// Payload of a standard measurement frame: length field `0x11` = 17 = 15
/// payload bytes + the 2-byte checksum
/// (`docs/research/ut171/reverse-engineered-protocol.md` §3.4, §5.1).
const STANDARD_PAYLOAD: usize = 15;

/// Payload lengths of the short replies [`RESPONSE_MEASUREMENT`] mentions.
/// "Lengths 4-8" may count the payload or the length field (payload +
/// checksum), so both readings are covered.
const SHORT_REPLY_PAYLOAD: RangeInclusive<usize> = 2..=8;

/// Longest measurement payload this meter can send: the extended frame, whose
/// length field is `0x17` = 23 = 21 payload bytes + the 2-byte checksum
/// (`docs/research/ut171/reverse-engineered-protocol.md` §3.4, §5.2). Anything
/// longer is some other family's frame — the UT181A's, on this framing.
const MAX_MEASUREMENT_PAYLOAD: usize = 21;

/// Known UT171 command frames (complete wire bytes from RE docs).
/// Frame format: AB CD len_lo len_hi payload chk_lo chk_hi, where the
/// LE16 length counts payload + checksum (same framing as UT181A).
///
/// The connect frame is `pub(crate)` because detection sends it too
/// (`docs/research/ut171/reverse-engineered-protocol.md` §4.3).
pub(crate) const UT171_CMD_CONNECT: &[u8] = &[0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x01, 0x0F, 0x00];
const UT171_CMD_PAUSE: &[u8] = &[0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x00, 0x0E, 0x00];

/// Protocol implementation for the UT171A/B/C.
pub(crate) struct Ut171Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
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
            profile: DeviceProfile {
                family_name: "UT171",
                model_name: "UNI-T UT171",
                stability: Stability::Experimental,
                supported_commands: UT171_COMMANDS,
                max_aux_values: 1,
                verification_issue: Some(4),
                meter_keys: MeterKeys::NONE,
            },
        }
    }
}

impl Protocol for Ut171Protocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // Send connect command to start streaming.
        // User must also enable "Communication ON" on the meter.
        debug!("ut171: sending connect command");
        transport.write(UT171_CMD_CONNECT)?;
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            // UT171 framing is identical to UT181A: 2-byte LE length =
            // payload + checksum, LE16 sum. Verified against the connect
            // command (AB CD 04 00 0A 01 0F 00: len 4, sum 04+00+0A+01 =
            // 0x000F LE) and gulux/Uni-T-CP2110's capture-driven parser.
            framing::extract_frame_abcd_2byte_le16,
            is_measurement_frame,
            FrameErrorRecovery::SkipAndRetry,
            "ut171",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let frame = match command {
            "connect" => UT171_CMD_CONNECT,
            "pause" => UT171_CMD_PAUSE,
            _ => return Err(Error::UnsupportedCommand(command.to_string())),
        };
        debug!("ut171: sending command {command}: {:02X?}", frame);
        transport.write(frame)?;
        Ok(())
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

        let [vdc, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
            Volts::VDc,
            CaptureStep::basic("vdc", "Set meter to V DC"),
            Ohms::Word,
            CaptureStep::basic(
                "ohm",
                "Set meter to Resistance. Leave leads open (should show OL).",
            ),
        );

        // All UT171 modes (0x01-0x24)
        vec![
            vdc,
            dcv_short,
            dcv_negative,
            CaptureStep::basic("vac", "Set meter to V AC").expect(Expect::mode("V AC")),
            // Spec §6: the manual reaches the V-to-frequency converter by
            // long-pressing in AC V mode.
            CaptureStep::basic(
                "vfc",
                "V AC mode: long-press for VFC (V\u{2192}Hz converter)",
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
            CaptureStep::basic("cap", "Set meter to Capacitance")
                .expect(Expect::mode("Capacitance")),
            ohm,
            ohm_body,
            ohm_short,
            CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
            CaptureStep::basic("tempc", "Set meter to Temperature C (if available)")
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°C")),
            CaptureStep::basic("tempf", "Set meter to Temperature F (if available)")
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("°F")),
            CaptureStep::basic("ns", "Set meter to Conductance nS (if available)")
                .expect(Expect::mode("nS")),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Hz")),
            CaptureStep::basic("duty", "Set meter to Duty Cycle (%)")
                .expect(Expect::mode("Duty %")),
            CaptureStep::basic("uadc", "Set meter to µA DC").expect(Expect::mode("µA DC")),
            CaptureStep::basic("uaac", "Set meter to µA AC").expect(Expect::mode("µA AC")),
            CaptureStep::basic("uaacdc", "Set meter to µA AC+DC").expect(Expect::mode("µA AC+DC")),
            CaptureStep::basic("madc", "Set meter to mA DC").expect(Expect::mode("mA DC")),
            CaptureStep::basic("maac", "Set meter to mA AC").expect(Expect::mode("mA AC")),
            CaptureStep::basic("maacdc", "Set meter to mA AC+DC").expect(Expect::mode("mA AC+DC")),
            CaptureStep::basic("ma420", "Set meter to % (4-20 mA loop)")
                .expect(Expect::mode("% 4-20mA")),
            CaptureStep::basic("adc", "Set meter to A DC").expect(Expect::mode("A DC")),
            CaptureStep::basic("aac", "Set meter to A AC").expect(Expect::mode("A AC")),
            CaptureStep::basic("aacdc", "Set meter to A AC+DC").expect(Expect::mode("A AC+DC")),
            // The 600 A clamp positions are UT171C-only (spec §1.1).
            CaptureStep::basic("a600dc", "Set meter to 600A DC (clamp, UT171C only)")
                .expect(Expect::mode("600A DC")),
            CaptureStep::basic("a600ac", "Set meter to 600A AC (clamp, UT171C only)")
                .expect(Expect::mode("600A AC")),
            CaptureStep::basic("ncv", "Set meter to NCV. Hold near a live wire.")
                .needs(&[Need::LiveWire])
                .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        ]
    }
}

/// The stream filter: whether `payload` is a measurement frame, the one
/// shape [`parse_measurement`] reads.
///
/// A measurement frame has type [`RESPONSE_MEASUREMENT`] and at least a
/// standard frame's payload; the parser judges a longer one. The short
/// replies that share the type are skipped quietly. Anything else — its
/// checksum already held, so it is the meter's — is reported: §5 documents
/// no other frame from the meter, and the driver sends nothing but connect
/// and pause (`docs/research/ut171/reverse-engineered-protocol.md`).
fn is_measurement_frame(payload: &[u8]) -> bool {
    let len = payload.len();
    match payload.first() {
        Some(&RESPONSE_MEASUREMENT) if len >= STANDARD_PAYLOAD => return true,
        Some(&RESPONSE_MEASUREMENT) if SHORT_REPLY_PAYLOAD.contains(&len) => {}
        Some(kind) => report_unknown(
            FAMILY,
            "frame",
            format_args!("type {kind:#04x}, {len} bytes"),
        ),
        None => report_unknown(FAMILY, "frame", format_args!("{len} bytes")),
    }
    false
}

/// Report what a measurement payload carries outside the spec. No UT171
/// capture is in the repo, so every check rests on the spec alone; the
/// reading is parsed the same either way.
///
/// Sections are `docs/research/ut171/reverse-engineered-protocol.md`.
fn report_unrecognised_fields(payload: &[u8]) {
    let len = payload.len();
    let (flags, frame_type, mode_byte, range_byte) =
        (payload[1], payload[2], payload[3], payload[4]);
    // §3.4, §5.1-5.2: a standard frame (type 0x01) carries 15 payload bytes,
    // an extended one (type 0x03) 21.
    if !matches!(
        (frame_type, len),
        (0x01, STANDARD_PAYLOAD) | (0x03, MAX_MEASUREMENT_PAYLOAD)
    ) {
        report_unknown(
            FAMILY,
            "frame",
            format_args!("frame type {frame_type:#04x}, {len} bytes"),
        );
    }
    // §5.4: a mode with a range table sends its indices or 0 (auto). Every
    // table starts at 1, so a label for 1 is what says the mode has one.
    if range_byte != 0
        && lookup_range(mode_byte, range_byte).is_none()
        && lookup_range(mode_byte, 1).is_some()
    {
        report_unknown(
            FAMILY,
            "range byte",
            format_args!("mode {mode_byte:#04x} range {range_byte}"),
        );
    }
    // §5.3: the vendor app reads no bit 4 or 5, and the reading of bits 0
    // and 3 lost its evidence (`docs/verification-backlog.md`, UT171).
    if flags & (0x30 | 0x08 | 0x01) != 0 {
        report_unknown(FAMILY, "flag bits", format_args!("{flags:#04x}"));
    }
    // §5.3: bit 1 marks an extended frame.
    if (flags & 0x02 != 0) != (len == MAX_MEASUREMENT_PAYLOAD) {
        report_unknown(
            FAMILY,
            "flag bits",
            format_args!("{flags:#04x} on a {len}-byte payload"),
        );
    }
    // §5.1: status2 (wire offset 13) holds 0x40 (DC) and 0x20 (AC); the
    // byte after it has been seen as 0x00 and 0x01.
    if payload[9] & !0x60 != 0 {
        report_unknown(
            FAMILY,
            "status byte",
            format_args!("payload[9] = {:#04x}", payload[9]),
        );
    }
    if payload[10] > 1 {
        report_unknown(
            FAMILY,
            "status byte",
            format_args!("payload[10] = {:#04x}", payload[10]),
        );
    }
}

/// Report a float slot holding what the spec does not describe.
fn report_float(what: &'static str, bytes: [u8; 4], mode_byte: u8) {
    report_unknown(
        FAMILY,
        what,
        format_args!(
            "{bytes:02X?} = {} in mode {mode_byte:#04x}",
            f32::from_le_bytes(bytes)
        ),
    );
}

/// Parse a UT171 measurement payload (pure function).
///
/// Payload from the 2-byte-LE-length extractor (standard frame: 21 bytes
/// on the wire → 15-byte payload; extended: 27 → 21). Offsets relative to
/// the payload (= wire frame offset − 4):
/// - byte 0:   type (0x02 = measurement)
/// - byte 1:   flags byte
/// - byte 2:   frame type (0x01=standard, 0x03=extended)
/// - byte 3:   mode byte
/// - byte 4:   range byte (raw, 1-based)
/// - bytes 5-8: main value (float32 LE)
/// - byte 9:   status2 (0x40=DC, 0x20=AC — capture-deduced, [UNVERIFIED])
/// - byte 10:  unknown
/// - bytes 11-14: aux value (float32 LE)
/// - extended frames continue with a third float at bytes 17-20 (unparsed)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len("ut171", payload, STANDARD_PAYLOAD)?;

    let flags_byte = payload[1];
    let mode_byte = payload[3];
    let range_byte = payload[4];

    let (mode, base_unit) = lookup_mode(mode_byte);
    report_unrecognised_fields(payload);
    let range_label = lookup_range(mode_byte, range_byte).unwrap_or("");
    let unit = display_unit(mode_byte, range_byte, base_unit);

    // Parse IEEE 754 float32 LE main value
    let main_bytes: [u8; 4] = [payload[5], payload[6], payload[7], payload[8]];
    let main_float = f32::from_le_bytes(main_bytes);

    // Parse flags
    let hold = flags_byte & 0x80 != 0;
    let auto_range = flags_byte & 0x40 == 0; // inverted: clear = AUTO active
    let low_battery = flags_byte & 0x04 != 0;

    let flags = StatusFlags {
        hold,
        auto_range,
        low_battery,
        ..Default::default()
    };

    let value = if main_float.is_nan() || main_float.is_infinite() {
        // §5.1 gives the main value as a float and says nothing of how an
        // overload is sent: a non-finite one is our guess.
        report_float("float", main_bytes, mode_byte);
        MeasuredValue::Overload
    } else if mode == "NCV" {
        // §6 names the NCV mode but not its value; a level is a whole number
        // that fits the u8 it becomes.
        if !(0.0..=255.0).contains(&main_float) || main_float.fract() != 0.0 {
            report_float("float", main_bytes, mode_byte);
        }
        MeasuredValue::NcvLevel(main_float as u8)
    } else {
        MeasuredValue::Normal(main_float as f64)
    };

    // The wire value is an f32. `Normal` keeps it widened to f64 so arithmetic
    // (stats, integration) works on the exact value the meter sent, but every
    // formatter falls back to that f64 when `display_raw` is None — and
    // `12.345f32 as f64` is 12.345000267028809, so a 60000-count meter printed
    // 17 digits of binary-to-decimal artefact in the CLI, the CSV and the GUI.
    //
    // f32's Display prints the shortest decimal that round-trips to the same
    // f32, so this is the wire value exactly, with no invented precision: the
    // frame carries no decimal-places field (spec §5.1 offset 14 is [UNVERIFIED]),
    // so we must not pad to a resolution the protocol never told us.
    let display_raw = match value {
        MeasuredValue::Normal(_) => Some(format!("{main_float}")),
        MeasuredValue::Overload
        | MeasuredValue::NcvLevel(_)
        | MeasuredValue::NoReading(_)
        | MeasuredValue::Absent => None,
    };

    // Aux float32 at payload[11..15]. gulux/Uni-T-CP2110 (capture-driven)
    // labels the aux value "kHz" for the AC voltage modes; other modes'
    // aux semantics are unknown, so they get a neutral label and no unit.
    // Gate on finite + non-zero so static-layout modes that never use the
    // aux slot don't emit a spurious "Aux: 0" entry.
    let aux_bytes: [u8; 4] = [payload[11], payload[12], payload[13], payload[14]];
    let aux_float = f32::from_le_bytes(aux_bytes);
    // 0x03 = V AC, 0x06 = mV AC
    let aux_is_frequency = matches!(mode_byte, 0x03 | 0x06);
    // §5.1 describes the aux float only as the frequency on those two modes:
    // a non-zero one elsewhere, or a non-finite one anywhere, is new.
    if !aux_float.is_finite() || (aux_float != 0.0 && !aux_is_frequency) {
        report_float("aux value", aux_bytes, mode_byte);
    }
    let mut aux_values = Vec::new();
    if aux_float.is_finite() && aux_float != 0.0 {
        let (aux_label, aux_unit) = if aux_is_frequency {
            ("Frequency", "kHz")
        } else {
            ("Aux", "")
        };
        aux_values.push(AuxValue {
            label: Cow::Borrowed(aux_label),
            value: MeasuredValue::Normal(aux_float as f64),
            unit: Cow::Borrowed(aux_unit),
            // Same f32-widening artefact as the main value above.
            display_raw: Some(format!("{aux_float}")),
            elapsed_secs: None,
        });
    }

    Ok(Measurement {
        mode,
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        display_raw,
        flags,
        aux_values,
        ..Measurement::from_payload(payload)
    })
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
    recognise,
};

/// Start the stream — the same frame [`Protocol::init`] writes.
fn send_connect(transport: &dyn Transport) -> Result<()> {
    transport.write(UT171_CMD_CONNECT)
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
        if payload.first() != Some(&RESPONSE_MEASUREMENT) {
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
    use super::*;
    use crate::protocol::test_support::snapshot;

    fn make_payload(mode: u8, range: u8, value: f32, flags: u8) -> Vec<u8> {
        make_payload_with_aux(mode, range, value, flags, 0.0)
    }

    fn make_payload_with_aux(mode: u8, range: u8, value: f32, flags: u8, aux: f32) -> Vec<u8> {
        let vbytes = value.to_le_bytes();
        let abytes = aux.to_le_bytes();
        // Payload as produced by the 2-byte-LE-length extractor: starts at
        // the type byte (wire frame offset 4).
        vec![
            0x02,  // type = measurement
            flags, // flags byte
            0x01,  // frame type = standard
            mode,  // mode
            range, // range
            vbytes[0], vbytes[1], vbytes[2], vbytes[3], // main value
            0x00,      // status2
            0x00,      // unknown
            abytes[0], abytes[1], abytes[2], abytes[3], // aux value
        ]
    }

    /// The one payload whose every parsed field is pinned: an ordinary
    /// auto-ranging V DC reading.
    #[test]
    fn parse_vdc() {
        // Flags bit 6 clear = AUTO.
        let payload = make_payload(0x02, 0x01, 12.345, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC
mode_raw=0x02
range_raw=0x01
value=Normal(12.345000267028809)
unit=V
range_label=
display_raw=Some("12.345")
flags=auto_range
aux=0
raw_payload=15"#
        );
    }

    /// The wire float is an f32; widening it to f64 and formatting that
    /// printed 12.345000267028809 for a meter showing 12.345. Every consumer
    /// (CLI stdout, CSV `value`, the GUI reading) went through that fallback.
    #[test]
    fn value_prints_the_wire_float_not_its_widening_artefact() {
        for v in [12.345f32, 5.999, -55.79, 0.1] {
            let payload = make_payload(0x02, 0x01, v, 0x00);
            let m = parse_measurement(&payload).unwrap();
            let expected = v.to_string();
            assert_eq!(m.display_raw.as_deref(), Some(expected.as_str()));
            assert_eq!(m.value_export_str(), expected);
            assert!(
                m.to_string().starts_with(&expected),
                "Display should show {expected}, got {m}"
            );
        }
    }

    /// The exported string must still parse back to the same f32 the meter
    /// sent — shortest-round-trip formatting, not truncation.
    #[test]
    fn exported_value_round_trips_to_the_wire_float() {
        for v in [12.345f32, 5.999, -55.79, 1234.5, 0.001] {
            let payload = make_payload(0x02, 0x01, v, 0x00);
            let m = parse_measurement(&payload).unwrap();
            let parsed: f32 = m.value_export_str().parse().unwrap();
            assert_eq!(parsed, v, "round trip failed for {v}");
        }
    }

    /// Overload and NCV carry no digits of their own — leaving a stale
    /// display string there is what made overloads render as numbers.
    #[test]
    fn overload_and_ncv_carry_no_display_string() {
        let payload = make_payload(0x02, 0x01, f32::NAN, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert!(m.display_raw.is_none());
    }

    #[test]
    fn parse_ohm() {
        // Range 1 reads in Ω; ranges 2-4 are range-relative kΩ and 5-6 MΩ
        // (gulux scales ×1000 at range >= 2 and again at >= 5).
        let payload = make_payload(0x0A, 0x01, 470.5, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "Ω");

        let payload = make_payload(0x0A, 0x02, 4.705, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "kΩ");

        let payload = make_payload(0x0A, 0x05, 5.99, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "MΩ");
    }

    #[test]
    fn parse_hold_flag() {
        // Flags bit 7 = HOLD; bit 6 still clear, so AUTO stays on.
        let payload = make_payload(0x02, 0x01, 1.0, 0x80);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range);
    }

    #[test]
    fn parse_manual_range() {
        // Flags bit 6 set = manual range.
        let payload = make_payload(0x02, 0x01, 1.0, 0x40);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_low_battery() {
        let payload = make_payload(0x02, 0x01, 1.0, 0x04);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.low_battery);
    }

    #[test]
    fn parse_unknown_mode_permissive() {
        let payload = make_payload(0x30, 0x01, 1.0, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x30)
mode_raw=0x30
range_raw=0x01
value=Normal(1.0)
unit=
range_label=
display_raw=Some("1")
flags=auto_range
aux=0
raw_payload=15"#
        );
    }

    #[test]
    fn parse_nan_overload() {
        let payload = make_payload(0x0A, 0x01, f32::NAN, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x0a
range_raw=0x01
value=Overload
unit=Ω
range_label=
display_raw=None
flags=auto_range
aux=0
raw_payload=15"#
        );
    }

    #[test]
    fn parse_inf_overload() {
        let payload = make_payload(0x0A, 0x01, f32::INFINITY, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x0a
range_raw=0x01
value=Overload
unit=Ω
range_label=
display_raw=None
flags=auto_range
aux=0
raw_payload=15"#
        );
    }

    #[test]
    fn parse_ncv() {
        let payload = make_payload(0x24, 0x00, 3.0, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "NCV");
        assert!(matches!(m.value, MeasuredValue::NcvLevel(3)));
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x00; 10];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x0F, 0x01, 50.0, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x0F);
        assert_eq!(m.mode, "Hz");
    }

    #[test]
    fn range_label_capacitance() {
        // 0x09 capacitance spans 8 continuous indices nF→µF→mF
        for (r, expected) in [
            (1, "6nF"),
            (2, "60nF"),
            (3, "600nF"),
            (4, "6µF"),
            (5, "60µF"),
            (6, "600µF"),
            (7, "6mF"),
            (8, "60mF"),
        ] {
            let payload = make_payload(0x09, r, 0.0, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.range_label, expected, "cap range {r}");
            assert_eq!(m.range_raw, r);
        }
    }

    #[test]
    fn range_label_frequency() {
        // 0x0F Hz continuous 1-7 across Hz/kHz/MHz
        for (r, expected) in [
            (1, "60Hz"),
            (2, "600Hz"),
            (3, "6kHz"),
            (4, "60kHz"),
            (5, "600kHz"),
            (6, "6MHz"),
            (7, "60MHz"),
        ] {
            let payload = make_payload(0x0F, r, 0.0, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.range_label, expected, "Hz range {r}");
        }
    }

    #[test]
    fn range_label_current() {
        // µA DC (0x11) and µA AC (0x12) share the same magnitudes per §5.4.
        let payload = make_payload(0x11, 1, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "600µA");
        let payload = make_payload(0x12, 2, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "6000µA");
        // mA DC (0x14) / mA AC (0x15)
        let payload = make_payload(0x14, 1, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "60mA");
        let payload = make_payload(0x15, 2, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "600mA");
        // A DC (0x17)
        let payload = make_payload(0x17, 1, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "6A");
        let payload = make_payload(0x17, 2, 0.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "20A");
    }

    #[test]
    fn range_label_auto_is_empty() {
        // Range byte 0 = auto-range, no specific label.
        let payload = make_payload(0x0F, 0, 50.0, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_label, "");
        assert_eq!(m.range_raw, 0);
    }

    #[test]
    fn aux_value_populated_when_nonzero() {
        // V AC (0x03): aux is the frequency readout in kHz per gulux.
        let payload = make_payload_with_aux(0x03, 0x00, 230.0, 0x00, 50.0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V AC
mode_raw=0x03
range_raw=0x00
value=Normal(230.0)
unit=V
range_label=
display_raw=Some("230")
flags=auto_range
aux=1
aux1=Frequency value=Normal(50.0) unit=kHz display_raw=Some("50") elapsed_secs=None
raw_payload=15"#
        );
    }

    #[test]
    fn aux_value_empty_when_zero() {
        // Modes that don't use the aux slot send 0.0; don't surface a spurious entry.
        let payload = make_payload_with_aux(0x02, 0x00, 12.345, 0x00, 0.0);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.aux_values.is_empty());
    }

    #[test]
    fn aux_value_skipped_when_nan() {
        let payload = make_payload_with_aux(0x03, 0x00, 230.0, 0x00, f32::NAN);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.aux_values.is_empty());
    }

    #[test]
    fn range_label_undocumented_mode_is_empty() {
        // Voltage modes have no range table in spec §5.4 — label stays empty
        // rather than fabricated.
        let payload = make_payload(0x02, 1, 12.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "");
        let payload = make_payload(0x0A, 2, 470.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "");
        // AC+DC variants (0x13, 0x16, 0x19) also absent from §5.4 — empty.
        let payload = make_payload(0x13, 1, 100.0, 0x00);
        assert_eq!(parse_measurement(&payload).unwrap().range_label, "");
    }

    #[test]
    fn all_known_modes_parse() {
        // All known mode bytes from Ghidra analysis of UT171C.exe
        let known_modes: &[u8] = &[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
            0x1D, 0x24,
        ];
        for &code in known_modes {
            let payload = make_payload(code, 0x01, 1.0, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "mode {:#04x} should be known",
                code
            );
        }
    }

    // --- Detection (crate::detect) ---------------------------------------

    fn recognised(buf: &[u8], probing: &Probing) -> Option<Evidence> {
        (FINGERPRINT.recognise)(buf, probing)
    }

    /// A measurement frame short enough for either family is a UT171 once the
    /// connect frame has gone out: SET_MONITOR went out before it and a
    /// UT181A would have answered that one.
    #[test]
    fn a_measurement_frame_after_the_connect_is_a_ut171() {
        let frame = framing::test_frame_le16(&make_payload(0x01, 0x01, 1.0, 0x00));
        let probing = Probing {
            sent: vec![DeviceFamily::Ut171],
            ..Probing::default()
        };
        assert_eq!(
            recognised(&frame, &probing),
            Some(Evidence::Model {
                id: "ut171",
                reported_name: None,
            })
        );
    }

    /// The same frame before any trigger: a UT171 left connected is the
    /// likelier source than a UT181A left streaming, and the engine warns.
    #[test]
    fn a_measurement_frame_before_any_trigger_is_still_a_ut171() {
        let frame = framing::test_frame_le16(&make_payload(0x01, 0x01, 1.0, 0x00));
        assert_eq!(
            recognised(&frame, &Probing::default()),
            Some(Evidence::Model {
                id: "ut171",
                reported_name: None,
            })
        );
    }

    /// Past the extended frame's 21 payload bytes (§3.4/§5.2) no UT171 frame
    /// exists — that length belongs to the UT181A, whose framing is the same.
    #[test]
    fn a_payload_longer_than_the_extended_frame_is_not_a_ut171() {
        let mut payload = make_payload(0x01, 0x01, 1.0, 0x00);
        payload.resize(MAX_MEASUREMENT_PAYLOAD + 4, 0);
        let frame = framing::test_frame_le16(&payload);
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
        let frame = framing::test_frame_le16(&make_payload(0x01, 0x01, 1.0, 0x00));
        let probing = Probing {
            sent: vec![DeviceFamily::Ut181a],
            ..Probing::default()
        };
        assert_eq!(recognised(&frame, &probing), None);
    }

    /// The meter's short replies share the measurement type byte, so they
    /// would be claimed here too — they name no model either way, and the
    /// stream filter is what keeps them out of the parser.
    #[test]
    fn a_frame_of_another_type_identifies_nothing() {
        let frame = framing::test_frame_le16(&[0x01, 0x00, 0x00, 0x00]);
        assert_eq!(recognised(&frame, &Probing::default()), None);
    }

    // --- Unrecognised data (protocol::unrecognised) -----------------------

    use crate::protocol::capture_reports;
    use crate::transport::mock::MockTransport;

    /// Parse `payload`, keeping what it reported.
    fn parse_reporting(payload: &[u8]) -> (Result<Measurement>, Vec<String>) {
        capture_reports(|| parse_measurement(payload))
    }

    /// An extended frame (§5.2): flags bit 1, frame type 0x03, then the two
    /// extra-flag bytes and the third float.
    fn make_extended_payload(mode: u8, value: f32, flags: u8, third: f32) -> Vec<u8> {
        let mut payload = make_payload(mode, 0x00, value, flags | 0x02);
        payload[2] = 0x03;
        payload.extend_from_slice(&[0x00, 0x00]);
        payload.extend_from_slice(&third.to_le_bytes());
        payload
    }

    /// The payloads the tests above parse cleanly, and the forms §5 documents:
    /// every flag bit it reads, both status2 values, byte 10 at 1, the aux
    /// frequency, an extended frame.
    fn documented_payloads() -> Vec<Vec<u8>> {
        let mut payloads = vec![
            make_payload(0x02, 0x01, 12.345, 0x00),
            make_payload(0x0A, 0x01, 470.5, 0x00),
            make_payload(0x0A, 0x02, 4.705, 0x00),
            make_payload(0x0A, 0x05, 5.99, 0x00),
            make_payload(0x02, 0x01, 1.0, 0x80),
            make_payload(0x02, 0x01, 1.0, 0x40),
            make_payload(0x02, 0x01, 1.0, 0x04),
            make_payload(0x02, 0x01, 1.0, 0xC4),
            make_payload(0x24, 0x00, 3.0, 0x00),
            make_payload(0x24, 0x00, 0.0, 0x00),
            make_payload(0x24, 0x00, 255.0, 0x00),
            make_payload(0x0F, 0x01, 50.0, 0x00),
            make_payload(0x0F, 0x00, 50.0, 0x00),
            make_payload(0x13, 0x01, 100.0, 0x00),
            make_payload_with_aux(0x03, 0x00, 230.0, 0x00, 50.0),
            make_payload_with_aux(0x06, 0x00, 12.5, 0x00, 1.25),
            make_payload_with_aux(0x02, 0x00, 12.345, 0x00, 0.0),
            make_extended_payload(0x03, 230.0, 0x00, 230.1),
            make_extended_payload(0x04, 230.0, 0x80, 230.1),
        ];
        for (status2, byte10) in [(0x40, 0x00), (0x20, 0x01), (0x60, 0x00)] {
            let mut payload = make_payload(0x02, 0x01, 1.5, 0x00);
            payload[9] = status2;
            payload[10] = byte10;
            payloads.push(payload);
        }
        for (mode, ranges) in [
            (0x08, 1),
            (0x09, 8),
            (0x0E, 1),
            (0x0F, 7),
            (0x11, 2),
            (0x12, 2),
            (0x14, 2),
            (0x15, 2),
            (0x17, 2),
        ] {
            for range in 1..=ranges {
                payloads.push(make_payload(mode, range, 0.0, 0x00));
            }
        }
        for mode in (0x01..=0x1D).chain([0x24]) {
            payloads.push(make_payload(mode, 0x01, 1.0, 0x00));
        }
        payloads
    }

    #[test]
    fn documented_payloads_report_nothing() {
        for payload in documented_payloads() {
            let (m, reports) = parse_reporting(&payload);
            assert!(m.is_ok(), "{payload:02X?}: {m:?}");
            assert!(reports.is_empty(), "{payload:02X?}: {reports:?}");
        }
    }

    /// Short replies of the measurement type (the note on
    /// [`RESPONSE_MEASUREMENT`]) are skipped on the way to a reading without
    /// a word, whichever way their length is counted.
    #[test]
    fn short_replies_on_the_stream_report_nothing() {
        let mut proto = Ut171Protocol::new();
        let mut frames: Vec<Vec<u8>> = SHORT_REPLY_PAYLOAD
            .map(|len| {
                let mut reply = vec![0x00; len];
                reply[0] = RESPONSE_MEASUREMENT;
                framing::test_frame_le16(&reply)
            })
            .collect();
        frames.push(framing::test_frame_le16(&make_payload(
            0x02, 0x01, 12.345, 0x00,
        )));
        frames.push(framing::test_frame_le16(&make_extended_payload(
            0x03, 230.0, 0x00, 230.1,
        )));
        let mock = MockTransport::new(frames);
        let (first, reports) = capture_reports(|| {
            proto.init(&mock).unwrap();
            proto.request_measurement(&mock)
        });
        assert_eq!(first.unwrap().mode, "V DC");
        assert!(reports.is_empty(), "{reports:?}");
        let (second, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(second.unwrap().mode, "V AC");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A frame of another type, or of the measurement type but neither a
    /// short reply nor a reading, is skipped and reported.
    #[test]
    fn unknown_frames_on_the_stream_are_reported() {
        let mut proto = Ut171Protocol::new();
        let mut short = vec![0x00; 12];
        short[0] = RESPONSE_MEASUREMENT;
        let mock = MockTransport::new(vec![
            framing::test_frame_le16(&[0x01, 0x4F, 0x4B, 0x00]),
            framing::test_frame_le16(&short),
            framing::test_frame_le16(&[RESPONSE_MEASUREMENT]),
            framing::test_frame_le16(&[]),
            framing::test_frame_le16(&make_payload(0x02, 0x01, 12.345, 0x00)),
        ]);
        let (m, reports) = capture_reports(|| proto.request_measurement(&mock));
        assert_eq!(m.unwrap().display_raw.as_deref(), Some("12.345"));
        assert_eq!(
            reports,
            [
                "ut171: unrecognised frame: type 0x01, 4 bytes",
                "ut171: unrecognised frame: type 0x02, 12 bytes",
                "ut171: unrecognised frame: type 0x02, 1 bytes",
                "ut171: unrecognised frame: 0 bytes",
            ]
        );
    }

    /// §3.4 and §5.1-5.2 pair frame type 0x01 with 15 payload bytes and 0x03
    /// with 21.
    #[test]
    fn a_payload_of_neither_frame_shape_is_reported() {
        let mut long_standard = make_extended_payload(0x02, 1.5, 0x00, 0.0);
        long_standard[2] = 0x01;
        let mut short_extended = make_payload(0x02, 0x01, 1.5, 0x00);
        short_extended[2] = 0x03;
        let mut sixteen = make_payload(0x02, 0x01, 1.5, 0x00);
        sixteen.push(0x00);
        let mut type_two = make_payload(0x02, 0x01, 1.5, 0x00);
        type_two[2] = 0x02;
        for (payload, report) in [
            (long_standard, "frame type 0x01, 21 bytes"),
            (short_extended, "frame type 0x03, 15 bytes"),
            (sixteen, "frame type 0x01, 16 bytes"),
            (type_two, "frame type 0x02, 15 bytes"),
        ] {
            let (m, reports) = parse_reporting(&payload);
            assert_eq!(m.unwrap().display_raw.as_deref(), Some("1.5"));
            assert_eq!(reports, [format!("ut171: unrecognised frame: {report}")]);
        }
    }

    #[test]
    fn an_unknown_mode_byte_is_reported() {
        for mode in [0x00u8, 0x1E, 0x23, 0x25, 0x30] {
            let payload = make_payload(mode, 0x01, 1.0, 0x00);
            let (m, reports) = parse_reporting(&payload);
            assert_eq!(m.unwrap().mode, format!("Unknown({mode:#04x})"));
            assert_eq!(
                reports,
                [format!("ut171: unrecognised mode byte: {mode:#04x}")]
            );
        }
    }

    /// The first index past each §5.4 table is reported; auto (0) and modes
    /// without a table are not.
    #[test]
    fn a_range_past_the_mode_s_table_is_reported() {
        for (mode, last) in [
            (0x08u8, 1u8),
            (0x09, 8),
            (0x0E, 1),
            (0x0F, 7),
            (0x11, 2),
            (0x12, 2),
            (0x14, 2),
            (0x15, 2),
            (0x17, 2),
        ] {
            let (m, reports) = parse_reporting(&make_payload(mode, last + 1, 1.0, 0x00));
            let m = m.unwrap();
            assert_eq!(m.range_label, "", "mode {mode:#04x}");
            assert_eq!(m.range_raw, last + 1);
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised range byte: mode {mode:#04x} range {}",
                    last + 1
                )]
            );
            let (_, reports) = parse_reporting(&make_payload(mode, 0x00, 1.0, 0x00));
            assert!(reports.is_empty(), "mode {mode:#04x}: {reports:?}");
        }
        for mode in [0x02, 0x0A, 0x13, 0x18] {
            let (_, reports) = parse_reporting(&make_payload(mode, 9, 1.0, 0x00));
            assert!(reports.is_empty(), "mode {mode:#04x}: {reports:?}");
        }
    }

    /// §5.3 leaves bits 4-5 unread, and bits 0 and 3 have no evidence left.
    #[test]
    fn undefined_flag_bits_are_reported() {
        for (flags, report) in [
            (0x01, "0x01"),
            (0x08, "0x08"),
            (0x10, "0x10"),
            (0x20, "0x20"),
            (0x89, "0x89"),
        ] {
            let (m, reports) = parse_reporting(&make_payload(0x02, 0x01, 1.0, flags));
            let m = m.unwrap();
            assert_eq!(m.flags.hold, flags & 0x80 != 0);
            assert!(m.flags.auto_range);
            assert_eq!(
                reports,
                [format!("ut171: unrecognised flag bits: {report}")]
            );
        }
    }

    /// §5.3: bit 1 marks an extended frame.
    #[test]
    fn flag_bit_1_disagreeing_with_the_frame_size_is_reported() {
        let (m, reports) = parse_reporting(&make_payload(0x02, 0x01, 1.0, 0x02));
        assert!(m.unwrap().flags.auto_range);
        assert_eq!(
            reports,
            ["ut171: unrecognised flag bits: 0x02 on a 15-byte payload"]
        );

        let mut payload = make_extended_payload(0x03, 230.0, 0x00, 230.1);
        payload[1] = 0x00;
        let (m, reports) = parse_reporting(&payload);
        assert_eq!(m.unwrap().mode, "V AC");
        assert_eq!(
            reports,
            ["ut171: unrecognised flag bits: 0x00 on a 21-byte payload"]
        );
    }

    /// §5.1: status2 holds 0x40 and 0x20; the byte after it 0x00 or 0x01.
    #[test]
    fn undocumented_status_bytes_are_reported() {
        for (offset, byte, report) in [
            (9, 0x80, "payload[9] = 0x80"),
            (9, 0x41, "payload[9] = 0x41"),
            (10, 0x02, "payload[10] = 0x02"),
            (10, 0xFF, "payload[10] = 0xff"),
        ] {
            let mut payload = make_payload(0x02, 0x01, 1.5, 0x00);
            payload[offset] = byte;
            let (m, reports) = parse_reporting(&payload);
            assert_eq!(m.unwrap().display_raw.as_deref(), Some("1.5"));
            assert_eq!(
                reports,
                [format!("ut171: unrecognised status byte: {report}")]
            );
        }
    }

    /// §5.1 says nothing of how an overload is sent: it still reads as OL.
    #[test]
    fn a_non_finite_main_value_is_reported_and_still_overload() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let (m, reports) = parse_reporting(&make_payload(0x0A, 0x01, value, 0x00));
            assert!(matches!(m.unwrap().value, MeasuredValue::Overload));
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised float: {:02X?} = {value} in mode 0x0a",
                    value.to_le_bytes()
                )]
            );
        }
    }

    #[test]
    fn an_ncv_level_that_is_not_a_whole_byte_is_reported() {
        for (value, level) in [(3.5f32, 3u8), (256.0, 255), (-1.0, 0)] {
            let (m, reports) = parse_reporting(&make_payload(0x24, 0x00, value, 0x00));
            assert!(
                matches!(m.as_ref().unwrap().value, MeasuredValue::NcvLevel(l) if l == level),
                "{m:?}"
            );
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised float: {:02X?} = {value} in mode 0x24",
                    value.to_le_bytes()
                )]
            );
        }
    }

    /// §5.1 has the aux float only as V AC / mV AC's frequency.
    #[test]
    fn an_aux_value_outside_the_ac_voltage_modes_is_reported() {
        let (m, reports) = parse_reporting(&make_payload_with_aux(0x02, 0x01, 1.5, 0x00, 2.5));
        let m = m.unwrap();
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "Aux");
        assert_eq!(
            reports,
            [format!(
                "ut171: unrecognised aux value: {:02X?} = 2.5 in mode 0x02",
                2.5f32.to_le_bytes()
            )]
        );

        for (mode, aux) in [(0x03, f32::NAN), (0x02, f32::INFINITY)] {
            let (m, reports) = parse_reporting(&make_payload_with_aux(mode, 0x00, 1.5, 0x00, aux));
            assert!(m.unwrap().aux_values.is_empty());
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised aux value: {:02X?} = {aux} in mode {mode:#04x}",
                    aux.to_le_bytes()
                )]
            );
        }
    }
}
