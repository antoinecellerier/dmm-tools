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
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::Instant;

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
            warn!("ut171: unknown mode byte {:#04x}", byte);
            (Cow::Owned(format!("Unknown({:#04x})", byte)), "")
        }
    }
}

const UT171_COMMANDS: &[&str] = &["connect", "pause"];

/// Known UT171 command frames (complete wire bytes from RE docs).
/// Frame format: AB CD len payload chk_lo chk_hi
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
            framing::extract_frame_abcd_1byte_le16,
            // Only accept measurement frames (type byte = 0x02 at payload[1])
            |p| p.len() >= 2 && p[1] == 0x02,
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
            CaptureStep {
                id: "vdc",
                instruction: "Set meter to V DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "vac",
                instruction: "Set meter to V AC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "vacdc",
                instruction: "Set meter to V AC+DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "mvdc",
                instruction: "Set meter to mV DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "mvac",
                instruction: "Set meter to mV AC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "cont",
                instruction: "Set meter to Continuity",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "cap",
                instruction: "Set meter to Capacitance",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "ohm",
                instruction: "Set meter to Resistance",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "diode",
                instruction: "Set meter to Diode",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "tempc",
                instruction: "Set meter to Temperature C (if available)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "ns",
                instruction: "Set meter to Conductance nS (if available)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "hz",
                instruction: "Set meter to Frequency (Hz)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "duty",
                instruction: "Set meter to Duty Cycle (%)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "uadc",
                instruction: "Set meter to uA DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "uaac",
                instruction: "Set meter to uA AC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "madc",
                instruction: "Set meter to mA DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "maac",
                instruction: "Set meter to mA AC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "adc",
                instruction: "Set meter to A DC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "aac",
                instruction: "Set meter to A AC",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "ncv",
                instruction: "Set meter to NCV",
                command: None,
                samples: 5,
            },
        ]
    }
}

/// Parse a UT171 measurement payload (pure function).
///
/// Standard frame payload (length=0x11, 17 bytes):
/// - byte 0:   reserved (0x00)
/// - byte 1:   type (0x02 = measurement)
/// - byte 2:   flags byte
/// - byte 3:   frame type (0x01=standard, 0x03=extended)
/// - byte 4:   mode byte
/// - byte 5:   range byte (raw, 1-based)
/// - bytes 6-9: main value (float32 LE)
/// - byte 10:  status2 (0x40=DC, 0x20=AC)
/// - byte 11:  unknown
/// - bytes 12-15: aux value (float32 LE)
/// - byte 16:  padding
pub fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    if payload.len() < 17 {
        return Err(Error::invalid_response(
            format!(
                "ut171 payload too short: {} bytes, expected >= 17",
                payload.len()
            ),
            payload,
        ));
    }

    let flags_byte = payload[2];
    let mode_byte = payload[4];
    let range_byte = payload[5];

    let (mode, unit) = lookup_mode(mode_byte);
    let range_label = lookup_range(mode_byte, range_byte).unwrap_or("");

    // Parse IEEE 754 float32 LE main value
    let main_bytes: [u8; 4] = [payload[6], payload[7], payload[8], payload[9]];
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

    Ok(Measurement {
        timestamp: Instant::now(),
        mode,
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: None,
        display_raw: None,
        flags,
        aux_values: vec![],
        raw_payload: payload.to_vec(),
        spec: None,
        mode_spec: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(mode: u8, range: u8, value: f32, flags: u8) -> Vec<u8> {
        let vbytes = value.to_le_bytes();
        vec![
            0x00,  // reserved
            0x02,  // type = measurement
            flags, // flags byte
            0x01,  // frame type = standard
            mode,  // mode
            range, // range
            vbytes[0], vbytes[1], vbytes[2], vbytes[3], // main value
            0x00,      // status2
            0x00,      // unknown
            0x00, 0x00, 0x00, 0x00, // aux value
            0x00, // padding
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

    #[test]
    fn parse_ohm() {
        let payload = make_payload(0x0A, 0x02, 470.5, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "Ω");
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
