//! UT181A protocol.
//!
//! Streaming protocol: user must manually enable "Communication ON" on the meter.
//! Device streams measurement packets (type 0x02) continuously.
//!
//! Frame format: AB CD len_lo len_hi payload chk_lo chk_hi
//! Length = payload_size + 2 (includes checksum bytes).
//! Checksum = 16-bit LE sum of length + payload bytes.
//!
//! Values are IEEE 754 float32 (LE) with device-sent unit strings.
//! 97 mode words (uint16 LE) with structured nibble encoding.
//!
//! Based on 3 independent community implementations:
//! antage/ut181a (Rust), loblab/ut181a (C++), sigrok uni-t-ut181a (C).
//! See docs/research/ut181/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;
use std::time::Instant;

/// Decode a UT181A mode word (uint16 LE) into a human-readable string.
///
/// Nibble encoding: N3 N2 N1 N0
/// N3 = measurement family, N2 = sub-function, N1 = variant, N0 = 1=std/2=REL
fn decode_mode_word(mode: u16) -> Cow<'static, str> {
    let n3 = (mode >> 12) & 0xF;
    let n2 = (mode >> 8) & 0xF;
    let n1 = (mode >> 4) & 0xF;
    let n0 = mode & 0xF;

    let family = match n3 {
        0x1 => "V AC",
        0x2 => "mV AC",
        0x3 => "V DC",
        0x4 => match n2 {
            0x1 => "mV DC",
            0x2 => "°C",
            0x3 => "°F",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0x5 => match n2 {
            0x1 => "Ω",
            0x2 => "Continuity",
            0x3 => "nS",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0x6 => match n2 {
            0x1 => "Diode",
            0x2 => "Capacitance",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0x7 => match n2 {
            0x1 => "Hz",
            0x2 => "Duty %",
            0x3 => "Pulse Width",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0x8 => match n2 {
            0x1 => "µA DC",
            0x2 => "µA AC",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0x9 => match n2 {
            0x1 => "mA DC",
            0x2 => "mA AC",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        0xA => match n2 {
            0x1 => "A DC",
            0x2 => "A AC",
            _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
        },
        _ => return Cow::Owned(format!("Unknown({:#06x})", mode)),
    };

    let variant = match n1 {
        0x1 => "",
        0x2 => match n3 {
            0x1 | 0x2 => " Hz",
            0x3 => " AC+DC",
            0x8..=0xA => " Hz",
            _ => "",
        },
        0x3 => " Peak",
        0x4 => match n3 {
            0x1 => " LPF",
            0x2 => " AC+DC",
            _ => "",
        },
        0x5 => " dBV",
        0x6 => " dBm",
        _ => "",
    };

    let rel = if n0 == 0x2 { " REL" } else { "" };

    // When no variant or rel suffix, return the static family string directly
    if variant.is_empty() && rel.is_empty() {
        Cow::Borrowed(family)
    } else {
        Cow::Owned(format!("{family}{variant}{rel}"))
    }
}

/// Parse a UT181A unit string from 8 bytes (null-terminated).
fn parse_unit_string(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let trimmed = s.trim_end_matches('\0');
    trimmed.to_string()
}

const UT181A_COMMANDS: &[&str] = &[
    "hold",
    "range",
    "auto",
    "minmax",
    "exit_minmax",
    "monitor",
    "save",
];

/// Build a UT181A command frame: AB CD len_lo len_hi payload chk_lo chk_hi.
/// Length = payload.len() + 2 (includes checksum).
/// Checksum = LE sum of length field + payload bytes.
fn build_command(payload: &[u8]) -> Vec<u8> {
    let len_val = (payload.len() + 2) as u16;
    let mut frame = vec![0xAB, 0xCD];
    frame.push((len_val & 0xFF) as u8);
    frame.push((len_val >> 8) as u8);
    frame.extend_from_slice(payload);
    let checksum: u16 = frame[2..].iter().map(|&b| b as u16).sum();
    frame.push((checksum & 0xFF) as u8);
    frame.push((checksum >> 8) as u8);
    frame
}

/// Protocol implementation for the UT181A.
pub struct Ut181aProtocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Default for Ut181aProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut181aProtocol {
    pub fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(256),
            profile: DeviceProfile {
                family_name: "UT181A",
                model_name: "UNI-T UT181A",
                stability: Stability::Experimental,
                supported_commands: UT181A_COMMANDS,
            },
        }
    }
}

impl Protocol for Ut181aProtocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // User must enable "Communication ON" on the meter
        // Send CMD_CONT_DATA (0x05, enable=1) to start the measurement stream.
        // Verified against real UT181A hardware: bytes AB CD 04 00 05 01 0A 00.
        debug!("ut181a: sending start-stream command (CMD_CONT_DATA)");
        let frame = build_command(&[0x05, 0x01]);
        transport.write(&frame)?;
        debug!("ut181a: init (streaming, manual enable required)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_2byte_le16,
            // Only accept measurement frames (type 0x02)
            |p| !p.is_empty() && p[0] == 0x02,
            FrameErrorRecovery::SkipAndRetry,
            "ut181a",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let frame = match command {
            "hold" => build_command(&[0x12]),
            "range" => {
                // Cycle to next manual range (range + 1, wrapping)
                // Without state tracking, just toggle to range 1
                build_command(&[0x02, 0x01])
            }
            "auto" => build_command(&[0x02, 0x00]),
            "minmax" => build_command(&[0x04, 0x01, 0x00, 0x00, 0x00]),
            "exit_minmax" => build_command(&[0x04, 0x00, 0x00, 0x00, 0x00]),
            "monitor" => build_command(&[0x05, 0x01]),
            "save" => build_command(&[0x06]),
            _ => return Err(Error::UnsupportedCommand(command.to_string())),
        };
        debug!("ut181a: sending command {command}: {:02X?}", frame);
        transport.write(&frame)?;

        // Drain any response
        self.rx_buf.clear();
        let mut tmp = [0u8; 64];
        for _ in 0..3 {
            let n = transport.read_timeout(&mut tmp, 100)?;
            if n == 0 {
                break;
            }
        }
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
        // Core UT181A modes
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
                id: "ohm",
                instruction: "Set meter to Resistance",
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
                id: "ns",
                instruction: "Set meter to Conductance (nS)",
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
                id: "cap",
                instruction: "Set meter to Capacitance",
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
                id: "tempc",
                instruction: "Set meter to Temperature C",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "tempf",
                instruction: "Set meter to Temperature F",
                command: None,
                samples: 5,
            },
            // Remote command steps
            CaptureStep {
                id: "hold",
                instruction: "V DC mode: we will send HOLD.",
                command: Some("hold"),
                samples: 3,
            },
            CaptureStep {
                id: "hold_off",
                instruction: "We will send HOLD again to turn it off.",
                command: Some("hold"),
                samples: 3,
            },
            CaptureStep {
                id: "minmax",
                instruction: "We will enable MIN/MAX.",
                command: Some("minmax"),
                samples: 3,
            },
            CaptureStep {
                id: "minmax_off",
                instruction: "We will disable MIN/MAX.",
                command: Some("exit_minmax"),
                samples: 3,
            },
            CaptureStep {
                id: "auto",
                instruction: "We will set auto-range.",
                command: Some("auto"),
                samples: 3,
            },
        ]
    }
}

/// Look up range label from mode word and range byte.
///
/// Uses the table from protocol spec Section 7. The family nibble (N3) and
/// sub-function nibble (N2) together determine which range table applies.
/// Temperature and A current have fixed ranges (no label).
fn lookup_range_label(mode_word: u16, range: u8) -> &'static str {
    if range == 0 {
        return "Auto";
    }
    let family = (mode_word >> 12) & 0xF;
    let sub = (mode_word >> 8) & 0xF;

    match (family, sub, range) {
        // mV DC (0x4, sub 0x1) and mV AC (0x2, sub 0x1)
        (0x2 | 0x4, 0x1, 1) => "60mV",
        (0x2 | 0x4, 0x1, 2) => "600mV",

        // V AC (0x1) and V DC (0x3)
        (0x1 | 0x3, _, 1) => "6V",
        (0x1 | 0x3, _, 2) => "60V",
        (0x1 | 0x3, _, 3) => "600V",
        (0x1 | 0x3, _, 4) => "1000V",

        // µA DC (0x8, sub 0x1) and µA AC (0x8, sub 0x2)
        (0x8, _, 1) => "600\u{00B5}A",
        (0x8, _, 2) => "6000\u{00B5}A",

        // mA DC (0x9, sub 0x1) and mA AC (0x9, sub 0x2)
        (0x9, _, 1) => "60mA",
        (0x9, _, 2) => "600mA",

        // A DC/AC (0xA): fixed 10A range, no label needed
        (0xA, _, _) => "",

        // Resistance (0x5, sub 0x1)
        (0x5, 0x1, 1) => "600\u{2126}",
        (0x5, 0x1, 2) => "6k\u{2126}",
        (0x5, 0x1, 3) => "60k\u{2126}",
        (0x5, 0x1, 4) => "600k\u{2126}",
        (0x5, 0x1, 5) => "6M\u{2126}",
        (0x5, 0x1, 6) => "60M\u{2126}",

        // Continuity (0x5, sub 0x2), Conductance (0x5, sub 0x3): fixed range
        (0x5, _, _) => "",

        // Diode (0x6, sub 0x1): fixed range
        (0x6, 0x1, _) => "",

        // Capacitance (0x6, sub 0x2)
        (0x6, 0x2, 1) => "6nF",
        (0x6, 0x2, 2) => "60nF",
        (0x6, 0x2, 3) => "600nF",
        (0x6, 0x2, 4) => "6\u{00B5}F",
        (0x6, 0x2, 5) => "60\u{00B5}F",
        (0x6, 0x2, 6) => "600\u{00B5}F",
        (0x6, 0x2, 7) => "6mF",
        (0x6, 0x2, 8) => "60mF",

        // Frequency (0x7, sub 0x1)
        (0x7, 0x1, 1) => "60Hz",
        (0x7, 0x1, 2) => "600Hz",
        (0x7, 0x1, 3) => "6kHz",
        (0x7, 0x1, 4) => "60kHz",
        (0x7, 0x1, 5) => "600kHz",
        (0x7, 0x1, 6) => "6MHz",
        (0x7, 0x1, 7) => "60MHz",

        // Duty cycle (0x7, sub 0x2), Pulse width (0x7, sub 0x3): fixed range
        (0x7, _, _) => "",

        // Temperature (0x4, sub 0x2/0x3): fixed range
        (0x4, _, _) => "",

        _ => "",
    }
}

/// Parse a UT181A measurement payload (type 0x02 packet).
///
/// Common header (after type byte):
/// - byte 0:   type (0x02, already verified)
/// - byte 1:   misc (flags: bit7=HOLD, bits4-6=format, bit3=bargraph, etc.)
/// - byte 2:   misc2 (bit0=auto, bit1=HV, bit3=lead_error, bit4=COMP, bit5=record)
/// - bytes 3-4: mode word (uint16 LE)
/// - byte 5:   range (0x00=auto, 0x01-0x08=manual)
///
/// After header, the format-dependent value section starts at byte 6.
///
/// Full value = 13 bytes: float32(4) + precision(1) + unit_string(8)
/// Short value = 5 bytes: float32(4) + precision(1)
pub fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    if payload.len() < 19 {
        // Minimum: type(1) + misc(1) + misc2(1) + mode(2) + range(1) + value(13) = 19
        return Err(Error::invalid_response(
            format!(
                "ut181a payload too short: {} bytes, expected >= 19",
                payload.len()
            ),
            payload,
        ));
    }

    let misc = payload[1];
    let misc2 = payload[2];
    let mode_word = u16::from_le_bytes([payload[3], payload[4]]);
    let range = payload[5];

    let format_type = (misc >> 4) & 0x07;
    let hold = misc & 0x80 != 0;
    let auto_range = misc2 & 0x01 != 0;
    let hv_warning = misc2 & 0x02 != 0;

    let mode = decode_mode_word(mode_word);

    // Parse main value (starts at byte 6)
    if payload.len() < 6 + 13 {
        return Err(Error::invalid_response(
            format!(
                "ut181a payload too short for value: {} bytes",
                payload.len()
            ),
            payload,
        ));
    }

    let val_bytes: [u8; 4] = [payload[6], payload[7], payload[8], payload[9]];
    let main_float = f32::from_le_bytes(val_bytes);
    let precision = payload[10];
    let unit_bytes = &payload[11..19];
    let unit = parse_unit_string(unit_bytes);

    // Precision byte: bits 0-1 = overload flags, bits 4-7 = decimal places
    let is_overload = precision & 0x01 != 0 || precision & 0x02 != 0;
    let decimal_places = (precision >> 4) & 0x0F;

    let (value, display_raw) = if is_overload || main_float.is_nan() || main_float.is_infinite() {
        (MeasuredValue::Overload, None)
    } else {
        let v = main_float as f64;
        let dp = decimal_places as usize;
        let display = format!("{v:.dp$}");
        (MeasuredValue::Normal(v), Some(display))
    };

    let flags = StatusFlags {
        hold,
        auto_range,
        hv_warning,
        min: format_type == 0x02, // min/max format
        max: format_type == 0x02,
        rel: format_type == 0x01, // relative format
        ..Default::default()
    };

    Ok(Measurement {
        timestamp: Instant::now(),
        mode,
        mode_raw: mode_word,
        range_raw: range,
        value,
        unit: Cow::Owned(unit),
        range_label: Cow::Borrowed(lookup_range_label(mode_word, range)),
        progress: None,
        display_raw,
        flags,
        aux_values: vec![],
        raw_payload: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(
        mode: u16,
        value: f32,
        precision: u8,
        unit: &[u8; 8],
        misc: u8,
        misc2: u8,
    ) -> Vec<u8> {
        let vbytes = value.to_le_bytes();
        let mbytes = mode.to_le_bytes();
        let mut p = vec![
            0x02,  // type
            misc,  // misc
            misc2, // misc2
            mbytes[0], mbytes[1], // mode word LE
            0x00,      // range
            vbytes[0], vbytes[1], vbytes[2], vbytes[3], // value
            precision, // precision
        ];
        p.extend_from_slice(unit); // 8 bytes
        p
    }

    #[test]
    fn parse_vdc() {
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V DC");
        assert_eq!(m.unit, "VDC");
        assert!(m.flags.auto_range);
        if let MeasuredValue::Normal(v) = m.value {
            assert!((v - 12.345).abs() < 0.01);
        } else {
            panic!("expected Normal value");
        }
    }

    #[test]
    fn parse_vac() {
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V AC");
        assert_eq!(m.unit, "VAC");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x5111, 470.0, 0x20, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "~");
    }

    #[test]
    fn parse_overload_precision() {
        // Precision bit 0 = +OL
        let payload = make_payload(0x5111, 0.0, 0x01, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x80, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_hv_warning() {
        let payload = make_payload(0x3111, 500.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hv_warning);
    }

    #[test]
    fn decode_mode_word_known() {
        assert_eq!(decode_mode_word(0x1111), "V AC");
        assert_eq!(decode_mode_word(0x3111), "V DC");
        assert_eq!(decode_mode_word(0x5111), "Ω");
        assert_eq!(decode_mode_word(0x6211), "Capacitance");
        assert_eq!(decode_mode_word(0x7111), "Hz");
        assert_eq!(decode_mode_word(0x8111), "µA DC");
        assert_eq!(decode_mode_word(0xA111), "A DC");
    }

    #[test]
    fn decode_mode_word_variants() {
        assert_eq!(decode_mode_word(0x1121), "V AC Hz");
        assert_eq!(decode_mode_word(0x1131), "V AC Peak");
        assert_eq!(decode_mode_word(0x1141), "V AC LPF");
        assert_eq!(decode_mode_word(0x3121), "V DC AC+DC");
        assert_eq!(decode_mode_word(0x1112), "V AC REL");
    }

    #[test]
    fn decode_mode_word_unknown() {
        let s = decode_mode_word(0xFFFF);
        assert!(s.starts_with("Unknown"));
    }

    #[test]
    fn parse_nan_overload() {
        let payload = make_payload(0x5111, f32::NAN, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x02, 0x00, 0x00, 0x11, 0x31]; // 5 bytes, need >= 19
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x7211, 50.0, 0x00, b"%\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x7211);
        assert_eq!(m.mode, "Duty %");
    }

    #[test]
    fn display_raw_uses_precision_decimal_places() {
        // precision 0x40 => bits 4-7 = 4 decimal places
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.3450"));

        // precision 0x20 => bits 4-7 = 2 decimal places
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("230.50"));

        // precision 0x00 => 0 decimal places
        let payload = make_payload(0x5111, 470.0, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("470"));
    }

    #[test]
    fn display_raw_none_on_overload() {
        let payload = make_payload(0x5111, 0.0, 0x01, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.display_raw.is_none());
    }

    #[test]
    fn build_command_hold() {
        let frame = build_command(&[0x12]);
        // AB CD 03 00 12 15 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x03, 0x00, 0x12, 0x15, 0x00]);
    }

    #[test]
    fn build_command_set_range_auto() {
        let frame = build_command(&[0x02, 0x00]);
        // AB CD 04 00 02 00 06 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x02, 0x00, 0x06, 0x00]);
    }

    #[test]
    fn build_command_set_minmax_on() {
        let frame = build_command(&[0x04, 0x01, 0x00, 0x00, 0x00]);
        // AB CD 07 00 04 01 00 00 00 0C 00
        assert_eq!(
            frame,
            vec![
                0xAB, 0xCD, 0x07, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00
            ]
        );
    }

    #[test]
    fn build_command_monitor_on() {
        let frame = build_command(&[0x05, 0x01]);
        // AB CD 04 00 05 01 0A 00
        assert_eq!(frame, vec![0xAB, 0xCD, 0x04, 0x00, 0x05, 0x01, 0x0A, 0x00]);
    }

    #[test]
    fn init_sends_start_stream_command() {
        use crate::transport::mock::MockTransport;
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut181aProtocol::new();
        proto.init(&mock).unwrap();
        let written = mock.written.borrow();
        // CMD_CONT_DATA: AB CD 04 00 05 01 0A 00
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0],
            vec![0xAB, 0xCD, 0x04, 0x00, 0x05, 0x01, 0x0A, 0x00]
        );
    }

    #[test]
    fn range_label_auto() {
        assert_eq!(lookup_range_label(0x3111, 0x00), "Auto");
        assert_eq!(lookup_range_label(0x5111, 0x00), "Auto");
    }

    #[test]
    fn range_label_voltage() {
        assert_eq!(lookup_range_label(0x3111, 1), "6V");
        assert_eq!(lookup_range_label(0x3111, 2), "60V");
        assert_eq!(lookup_range_label(0x3111, 3), "600V");
        assert_eq!(lookup_range_label(0x3111, 4), "1000V");
        // V AC uses same ranges
        assert_eq!(lookup_range_label(0x1111, 2), "60V");
    }

    #[test]
    fn range_label_millivolt() {
        assert_eq!(lookup_range_label(0x4111, 1), "60mV");
        assert_eq!(lookup_range_label(0x4111, 2), "600mV");
        assert_eq!(lookup_range_label(0x2111, 1), "60mV");
    }

    #[test]
    fn range_label_resistance() {
        assert_eq!(lookup_range_label(0x5111, 1), "600\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 3), "60k\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 6), "60M\u{2126}");
    }

    #[test]
    fn range_label_capacitance() {
        assert_eq!(lookup_range_label(0x6211, 1), "6nF");
        assert_eq!(lookup_range_label(0x6211, 4), "6\u{00B5}F");
        assert_eq!(lookup_range_label(0x6211, 8), "60mF");
    }

    #[test]
    fn range_label_frequency() {
        assert_eq!(lookup_range_label(0x7111, 1), "60Hz");
        assert_eq!(lookup_range_label(0x7111, 5), "600kHz");
        assert_eq!(lookup_range_label(0x7111, 7), "60MHz");
    }

    #[test]
    fn range_label_current() {
        assert_eq!(lookup_range_label(0x8111, 1), "600\u{00B5}A");
        assert_eq!(lookup_range_label(0x9111, 2), "600mA");
        // A current: fixed range
        assert_eq!(lookup_range_label(0xA111, 1), "");
    }

    #[test]
    fn range_label_fixed_range_modes() {
        // Temperature, continuity, conductance, diode: no range label
        assert_eq!(lookup_range_label(0x4211, 1), ""); // Temp C
        assert_eq!(lookup_range_label(0x5211, 1), ""); // Continuity
        assert_eq!(lookup_range_label(0x5311, 1), ""); // Conductance
        assert_eq!(lookup_range_label(0x6111, 1), ""); // Diode
        assert_eq!(lookup_range_label(0x7211, 1), ""); // Duty cycle
    }

    #[test]
    fn range_raw_populated() {
        let payload = make_payload(0x3111, 12.0, 0x20, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        // range byte is at payload[5] which make_payload sets to 0x00
        assert_eq!(m.range_raw, 0x00);
        assert_eq!(m.range_label, "Auto");
    }
}
