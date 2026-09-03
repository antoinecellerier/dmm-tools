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

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability, check_len, unknown_mode};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;

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
            debug!("ut171: unknown mode byte {:#04x}", byte);
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

/// Known UT171 command frames (complete wire bytes from RE docs).
/// Frame format: AB CD len_lo len_hi payload chk_lo chk_hi, where the
/// LE16 length counts payload + checksum (same framing as UT181A).
const UT171_CMD_CONNECT: &[u8] = &[0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x01, 0x0F, 0x00];
const UT171_CMD_PAUSE: &[u8] = &[0xAB, 0xCD, 0x04, 0x00, 0x0A, 0x00, 0x0E, 0x00];

/// Protocol implementation for the UT171A/B/C.
pub struct Ut171Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Default for Ut171Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut171Protocol {
    pub fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "UT171",
                model_name: "UNI-T UT171",
                stability: Stability::Experimental,
                supported_commands: UT171_COMMANDS,
                max_aux_values: 1,
                verification_issue: Some(4),
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
            // Accept only full measurement frames: type 0x02 at payload[0]
            // AND a measurement-sized payload — short ack/response frames
            // share the 0x02 type byte (gulux observes lengths 4-8).
            |p| p.len() >= 15 && p[0] == 0x02,
            FrameErrorRecovery::SkipAndRetry,
            "ut171",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
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

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::CaptureStep;
        // All UT171 modes (0x01-0x24)
        vec![
            CaptureStep::basic("vdc", "Set meter to V DC"),
            CaptureStep::basic("vac", "Set meter to V AC"),
            CaptureStep::basic("vacdc", "Set meter to V AC+DC"),
            CaptureStep::basic("mvdc", "Set meter to mV DC"),
            CaptureStep::basic("mvac", "Set meter to mV AC"),
            CaptureStep::basic("cont", "Set meter to Continuity"),
            CaptureStep::basic("cap", "Set meter to Capacitance"),
            CaptureStep::basic("ohm", "Set meter to Resistance"),
            CaptureStep::basic("diode", "Set meter to Diode"),
            CaptureStep::basic("tempc", "Set meter to Temperature C (if available)"),
            CaptureStep::basic("ns", "Set meter to Conductance nS (if available)"),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)"),
            CaptureStep::basic("duty", "Set meter to Duty Cycle (%)"),
            CaptureStep::basic("uadc", "Set meter to uA DC"),
            CaptureStep::basic("uaac", "Set meter to uA AC"),
            CaptureStep::basic("madc", "Set meter to mA DC"),
            CaptureStep::basic("maac", "Set meter to mA AC"),
            CaptureStep::basic("adc", "Set meter to A DC"),
            CaptureStep::basic("aac", "Set meter to A AC"),
            CaptureStep::basic("ncv", "Set meter to NCV"),
        ]
    }
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
pub fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len("ut171", payload, 15)?;

    let flags_byte = payload[1];
    let mode_byte = payload[3];
    let range_byte = payload[4];

    let (mode, base_unit) = lookup_mode(mode_byte);
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
        MeasuredValue::Overload
    } else if mode == "NCV" {
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
        MeasuredValue::Overload | MeasuredValue::NcvLevel(_) => None,
    };

    // Aux float32 at payload[11..15]. gulux/Uni-T-CP2110 (capture-driven)
    // labels the aux value "kHz" for the AC voltage modes; other modes'
    // aux semantics are unknown, so they get a neutral label and no unit.
    // Gate on finite + non-zero so static-layout modes that never use the
    // aux slot don't emit a spurious "Aux: 0" entry.
    let aux_bytes: [u8; 4] = [payload[11], payload[12], payload[13], payload[14]];
    let aux_float = f32::from_le_bytes(aux_bytes);
    let mut aux_values = Vec::new();
    if aux_float.is_finite() && aux_float != 0.0 {
        // 0x03 = V AC, 0x06 = mV AC
        let (aux_label, aux_unit) = if matches!(mode_byte, 0x03 | 0x06) {
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_vdc() {
        let payload = make_payload(0x02, 0x01, 12.345, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V DC");
        assert_eq!(m.unit, "V");
        assert!(m.flags.auto_range); // bit 6 clear = AUTO
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 12.345).abs() < 0.01);
        } else {
            panic!("expected Normal value");
        }
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
        let payload = make_payload(0x02, 0x01, 1.0, 0x80);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range); // bit 6 still clear
    }

    #[test]
    fn parse_manual_range() {
        let payload = make_payload(0x02, 0x01, 1.0, 0x40);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range); // bit 6 set = manual
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
        assert_eq!(m.mode, "Unknown(0x30)");
    }

    #[test]
    fn parse_nan_overload() {
        let payload = make_payload(0x0A, 0x01, f32::NAN, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_inf_overload() {
        let payload = make_payload(0x0A, 0x01, f32::INFINITY, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
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
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "Frequency");
        assert_eq!(m.aux_values[0].unit, "kHz");
        if let MeasuredValue::Normal(v) = m.aux_values[0].value {
            assert!((v - 50.0).abs() < 0.01);
        } else {
            panic!("expected Normal aux");
        }
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
}
