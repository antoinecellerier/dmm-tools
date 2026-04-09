//! UT8802/UT8802N bench multimeter protocol.
//!
//! Streaming protocol: host sends 0x5A trigger byte after CP2110 init,
//! meter streams 8-byte measurement frames continuously.
//!
//! Frame format: AC [position] [d1d2] [d3d4] [d5xx] [dp+flags] [status] [sign]
//!
//! Key difference from UT8803: single-byte 0xAC header, BCD-encoded display,
//! combined position codes (function + range), no checksum.
//!
//! Based on reverse engineering of uci.dll (Ghidra decompilation) and
//! UT8803E Programming Manual V1.1.
//! See docs/research/uci-bench-family/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::Instant;

/// UT8802 position code table: (code, mode_name, unit, range_label).
///
/// Combined function + range position codes from the programming manual
/// (page 10) and Ghidra decompilation (FUN_1001c7b0, line 23234).
/// Both sources agree on all 35 entries.
const POSITION_TABLE: &[(u8, &str, &str, &str)] = &[
    (0x01, "DC V", "V", "200mV"),
    (0x03, "DC V", "V", "2V"),
    (0x04, "DC V", "V", "20V"),
    (0x05, "DC V", "V", "200V"),
    (0x06, "DC V", "V", "1000V"),
    (0x09, "AC V", "V", "2V"),
    (0x0A, "AC V", "V", "20V"),
    (0x0B, "AC V", "V", "200V"),
    (0x0C, "AC V", "V", "750V"),
    (0x0D, "DC µA", "µA", "200µA"),
    (0x0E, "DC mA", "mA", "2mA"),
    (0x10, "AC mA", "mA", "2mA"),
    (0x11, "DC mA", "mA", "20mA"),
    (0x12, "DC mA", "mA", "200mA"),
    (0x13, "AC mA", "mA", "20mA"),
    (0x14, "AC mA", "mA", "200mA"),
    (0x16, "DC A", "A", "2A"),
    (0x18, "AC A", "A", "20A"),
    (0x19, "Ω", "Ω", "200Ω"),
    (0x1A, "Ω", "Ω", "2kΩ"),
    (0x1B, "Ω", "Ω", "20kΩ"),
    (0x1C, "Ω", "Ω", "200kΩ"),
    (0x1D, "Ω", "Ω", "2MΩ"),
    (0x1F, "Ω", "Ω", "200MΩ"),
    (0x22, "Duty %", "%", ""),
    (0x23, "Diode", "V", ""),
    (0x24, "Continuity", "Ω", ""),
    (0x25, "hFE", "", ""),
    (0x27, "Capacitance", "F", "nF"),
    (0x28, "Capacitance", "F", "µF"),
    (0x29, "Capacitance", "F", "mF"),
    (0x2A, "SCR", "V", ""),
    (0x2B, "Hz", "Hz", "Hz"),
    (0x2C, "Hz", "Hz", "kHz"),
    (0x2D, "Hz", "Hz", "MHz"),
];

/// Look up a position code in the table. Returns (mode, unit, range_label).
fn lookup_position(code: u8) -> Option<(&'static str, &'static str, &'static str)> {
    POSITION_TABLE
        .iter()
        .find(|(c, _, _, _)| *c == code)
        .map(|(_, mode, unit, range)| (*mode, *unit, *range))
}

const UT8802_COMMANDS: &[&str] = &[];

/// Protocol implementation for the UT8802/UT8802N bench multimeter.
pub struct Ut8802Protocol {
    rx_buf: Vec<u8>,
    triggered: bool,
    profile: DeviceProfile,
}

impl Default for Ut8802Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut8802Protocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            triggered: false,
            profile: DeviceProfile {
                family_name: "UT8802",
                model_name: "UNI-T UT8802",
                stability: Stability::Experimental,
                supported_commands: UT8802_COMMANDS,
            },
        }
    }
}

impl Protocol for Ut8802Protocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // Send 0x5A trigger byte to start streaming (same as UT8803)
        debug!("ut8802: sending 0x5A trigger byte");
        transport.write(&[0x5A])?;
        self.triggered = true;
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_ut8802,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "ut8802",
            &framing::UT8802_HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        Err(Error::UnsupportedCommand(command.to_string()))
    }

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::CaptureStep;
        vec![
            CaptureStep {
                id: "dcv",
                instruction: "Set meter to DC V",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "acv",
                instruction: "Set meter to AC V",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "dcua",
                instruction: "Set meter to DC µA",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "dcma",
                instruction: "Set meter to DC mA",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "dca",
                instruction: "Set meter to DC A",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "acma",
                instruction: "Set meter to AC mA",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "aca",
                instruction: "Set meter to AC A",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "ohm",
                instruction: "Set meter to Resistance (Ω)",
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
                id: "hfe",
                instruction: "Set meter to hFE (transistor test)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "scr",
                instruction: "Set meter to SCR (thyristor test)",
                command: None,
                samples: 5,
            },
        ]
    }
}

/// Convert a BCD nibble to its display character.
///
/// - 0x0-0x9 → '0'-'9'
/// - 0x0A → '0' (treated as zero per vendor code)
/// - 0x0C → 'L' (overload indicator)
///
/// Other values should be rejected by the frame extractor's validation,
/// but we handle them defensively with '?'.
fn bcd_to_char(nibble: u8) -> char {
    match nibble {
        0x00..=0x09 => (b'0' + nibble) as char,
        0x0A => '0',
        0x0C => 'L',
        _ => '?',
    }
}

/// Parse a UT8802 measurement payload (7 bytes = frame bytes 1..8).
///
/// Layout (from Ghidra FUN_1001e0a0 + programming manual):
/// - byte 0: position code (0x01-0x2D, combined function + range)
/// - byte 1: digits 1-2 (high nibble = d1, low nibble = d2)
/// - byte 2: digits 3-4 (high nibble = d3, low nibble = d4)
/// - byte 3: digit 5 (low nibble = d5, high nibble unused)
/// - byte 4: decimal point position (low nibble, 0-4) + AC/DC flags (bits 4-5)
/// - byte 5: status/bargraph byte [UNVERIFIED purpose]
/// - byte 6: sign (bit 7) + status flags (bits 0-6)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    if payload.len() < 7 {
        return Err(Error::invalid_response(
            format!(
                "ut8802 payload too short: {} bytes, expected 7",
                payload.len()
            ),
            payload,
        ));
    }

    let position = payload[0];
    let dp_pos = payload[4] & 0x0F;
    let sign_byte = payload[6];

    // Look up position code
    let (mode, unit, range_label): (Cow<'static, str>, &'static str, &'static str) =
        if let Some((m, u, r)) = lookup_position(position) {
            (Cow::Borrowed(m), u, r)
        } else {
            warn!("ut8802: unknown position code {position:#04x}");
            (Cow::Owned(format!("Unknown({position:#04x})")), "", "")
        };

    // Decode BCD digits
    let nibbles = [
        payload[1] >> 4,
        payload[1] & 0x0F,
        payload[2] >> 4,
        payload[2] & 0x0F,
        payload[3] & 0x0F,
    ];

    let mut overload = false;
    let mut chars: Vec<char> = Vec::with_capacity(6); // 5 digits + possible decimal point
    for &n in &nibbles {
        let ch = bcd_to_char(n);
        if ch == 'L' {
            overload = true;
        }
        chars.push(ch);
    }

    // Replace leading zeros with spaces (vendor behavior), but preserve
    // the digit just left of the decimal point (or the last digit when
    // dp_pos=0). Without this, an all-zero reading would become "     "
    // and fail to parse as 0.0.
    let keep_pos = if dp_pos > 0 {
        chars.len() - dp_pos as usize - 1
    } else {
        chars.len() - 1
    };
    for (i, ch) in chars.iter_mut().enumerate() {
        if i >= keep_pos {
            break;
        }
        if *ch == '0' {
            *ch = ' ';
        } else {
            break;
        }
    }

    // Insert decimal point
    if dp_pos > 0 && !overload {
        let insert_pos = chars.len() - dp_pos as usize;
        chars.insert(insert_pos, '.');
    }

    let display_str: String = chars.iter().collect();

    // Parse numeric value
    let value = if overload {
        MeasuredValue::Overload
    } else {
        let trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();
        match trimmed.parse::<f64>() {
            Ok(mut v) => {
                // Bit 7 of sign byte = polarity (1 = negative)
                if sign_byte & 0x80 != 0 {
                    v = -v;
                }
                MeasuredValue::Normal(v)
            }
            Err(_) => {
                warn!("ut8802: could not parse display value: {display_str:?}");
                MeasuredValue::Overload
            }
        }
    };

    // Flag extraction from byte 6 (sign_byte).
    //
    // Known:
    //   bit 7: sign/polarity (handled above)
    //   bit 2: AUTO flag, **inverted logic** (clear = auto ON) [VENDOR]
    //
    // Best-guess from Ghidra debug format string and D28-D31 status word layout:
    //   bit 6: HOLD  [UNVERIFIED]
    //   bit 5: REL   [UNVERIFIED]
    //   bit 4: MAX   [UNVERIFIED]
    //   bit 3: MIN   [UNVERIFIED]
    let auto_range = sign_byte & 0x04 == 0; // bit 2 inverted
    let hold = sign_byte & 0x40 != 0; // [UNVERIFIED] bit 6
    let rel = sign_byte & 0x20 != 0; // [UNVERIFIED] bit 5
    let max_flag = sign_byte & 0x10 != 0; // [UNVERIFIED] bit 4
    let min_flag = sign_byte & 0x08 != 0; // [UNVERIFIED] bit 3

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery: false,
        hv_warning: false,
        dc: false,
        peak_max: false,
        peak_min: false,
    };

    Ok(Measurement {
        timestamp: Instant::now(),
        mode,
        mode_raw: position as u16,
        range_raw: position, // UT8802 combines mode+range in position code
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: None,
        display_raw: Some(display_str),
        flags,
        aux_values: vec![],
        raw_payload: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 7-byte UT8802 payload from components.
    fn make_payload(
        position: u8,
        digits: [u8; 5],
        dp_pos: u8,
        acdc_bits: u8,
        status: u8,
        sign_flags: u8,
    ) -> Vec<u8> {
        vec![
            position,
            (digits[0] << 4) | digits[1],
            (digits[2] << 4) | digits[3],
            digits[4],
            (acdc_bits << 4) | dp_pos,
            status,
            sign_flags,
        ]
    }

    #[test]
    fn parse_dcv() {
        // DC V 200V, display "1234.5" (dp_pos=1)
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "200V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1234.5).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        // AC V 20V, display "12.34" (dp_pos=2)
        let payload = make_payload(0x0A, [0, 1, 2, 3, 4], 2, 0x01, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "20V");
    }

    #[test]
    fn parse_resistance() {
        // Resistance 2kΩ, display "1.234" (dp_pos=3)
        let payload = make_payload(0x1A, [0, 1, 2, 3, 4], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "Ω");
        assert_eq!(m.range_label, "2kΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.234).abs() < 1e-6));
    }

    #[test]
    fn parse_overload() {
        // Digit with 0x0C nibble → overload
        let payload = make_payload(0x01, [0, 0, 0x0C, 0, 0], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_negative() {
        // Bit 7 of sign byte = negative
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x80);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-1234.5)).abs() < 1e-6));
    }

    #[test]
    fn parse_auto_range() {
        // Bit 2 clear = auto ON (inverted logic)
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        // Bit 2 set = auto OFF
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x04);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_hold_flag() {
        // [UNVERIFIED] bit 6 = HOLD
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x40);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        // [UNVERIFIED] bit 5 = REL
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x20);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        // [UNVERIFIED] bit 4 = MAX, bit 3 = MIN
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x10);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);

        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x08);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.max);
        assert!(m.flags.min);
    }

    #[test]
    fn parse_unknown_position() {
        // Position 0x02 is a gap — frame extractor would reject this,
        // but the parser should handle it gracefully
        let payload = make_payload(0x02, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.mode.starts_with("Unknown"));
    }

    #[test]
    fn parse_all_valid_positions() {
        for &(code, _, _, _) in POSITION_TABLE {
            let payload = make_payload(code, [0, 1, 0, 0, 0], 0, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "position {code:#04x} should be known"
            );
        }
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x01, 0x12, 0x34];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn display_raw_preserved() {
        // Display "12345" with dp_pos=0 (no decimal)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12345"));
    }

    #[test]
    fn decimal_point_positions() {
        // dp_pos=0 → "12345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12345"));

        // dp_pos=1 → "1234.5"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("1234.5"));

        // dp_pos=2 → "123.45"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 2, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("123.45"));

        // dp_pos=3 → "12.345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.345"));

        // dp_pos=4 → "1.2345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 4, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("1.2345"));
    }

    #[test]
    fn leading_zeros_replaced_with_spaces() {
        // digits [0, 0, 1, 2, 3] → "  123"
        let payload = make_payload(0x01, [0, 0, 1, 2, 3], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("  123"));
    }

    #[test]
    fn zero_reading_integer() {
        // All-zero digits, dp_pos=0 → "    0" → value 0.0
        let payload = make_payload(0x01, [0, 0, 0, 0, 0], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("    0"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn zero_reading_with_decimal() {
        // All-zero digits, dp_pos=4 (200mV range) → "0.0000" → value 0.0
        let payload = make_payload(0x01, [0, 0, 0, 0, 0], 4, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("0.0000"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn zero_reading_dp3() {
        // All-zero digits, dp_pos=3 → " 0.000" → value 0.0
        let payload = make_payload(0x1A, [0, 0, 0, 0, 0], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some(" 0.000"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn nibble_0a_treated_as_zero() {
        // 0x0A → '0', and it's a leading zero so replaced with space
        let payload = make_payload(0x01, [0x0A, 0, 1, 2, 3], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("  123"));
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x2B, [0, 5, 0, 0, 0], 1, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x2B);
        assert_eq!(m.mode, "Hz");
    }
}
