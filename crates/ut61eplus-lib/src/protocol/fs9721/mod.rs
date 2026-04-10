//! UT803/UT804 bench multimeter protocol.
//!
//! These meters use FS9721-style 14-byte framing (index nibble in high 4 bits)
//! but with a **proprietary data encoding** — the data nibbles carry structured
//! measurement data (mode codes, range codes, digit values, status flags),
//! NOT raw LCD segment data.
//!
//! Confirmed by Ghidra decompilation of UT803.exe V1.01 and UT804.exe V2.00
//! (standalone PC applications) plus binary constant extraction.
//!
//! See docs/research/ut803/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{CaptureStep, DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::Instant;

/// Mode table entry: (mode_code, mode_name, base_unit).
///
/// The base_unit may be modified by the range code (e.g., "Ω" → "kΩ").
/// Modes common to both UT803 and UT804 are listed first.
const MODE_TABLE: &[(u8, &str, &str)] = &[
    (0x01, "DC V", "V"),
    (0x02, "AC V", "V"),
    (0x03, "DC mV", "mV"),
    (0x04, "Ω", "Ω"),
    (0x05, "Capacitance", "F"),
    (0x06, "Diode", "V"),
    (0x07, "Hz", "Hz"),
    (0x08, "Duty %", "%"),
    (0x09, "hFE", ""),
    (0x0A, "Temperature", "°C"),
    (0x0B, "DC µA", "µA"),
    (0x0C, "A", "A"),
    (0x0D, "Continuity", "Ω"),
    (0x0E, "ADP", ""), // UT804 only, [UNVERIFIED] purpose
    (0x0F, "AC mA", "mA"),
];

/// Look up a mode code. Returns (mode_name, base_unit).
fn lookup_mode(code: u8) -> Option<(&'static str, &'static str)> {
    MODE_TABLE
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, name, unit)| (*name, *unit))
}

/// Derive the unit string from mode, range code, and base unit.
///
/// The range code selects prefix multipliers within certain modes.
/// Exact range-to-unit mappings are [UNVERIFIED] — these are best guesses
/// from the decompiled switch statements.
fn derive_unit(mode: u8, range: u8, base_unit: &'static str) -> Cow<'static, str> {
    match mode {
        0x04 => {
            // Resistance: range selects Ω/kΩ/MΩ
            match range {
                0..=2 => Cow::Borrowed("Ω"),
                3..=4 => Cow::Borrowed("kΩ"),
                5..=6 => Cow::Borrowed("MΩ"),
                _ => Cow::Borrowed("Ω"),
            }
        }
        0x05 => {
            // Capacitance: range selects nF/µF/mF
            match range {
                0..=2 => Cow::Borrowed("nF"),
                3..=5 => Cow::Borrowed("µF"),
                6..=7 => Cow::Borrowed("mF"),
                _ => Cow::Borrowed("F"),
            }
        }
        0x07 => {
            // Frequency: range selects Hz/kHz/MHz
            match range {
                0..=2 => Cow::Borrowed("Hz"),
                3..=4 => Cow::Borrowed("kHz"),
                5..=7 => Cow::Borrowed("MHz"),
                _ => Cow::Borrowed("Hz"),
            }
        }
        0x0C => {
            // Current (A): range selects mA/A
            match range {
                0..=2 => Cow::Borrowed("mA"),
                3..=7 => Cow::Borrowed("A"),
                _ => Cow::Borrowed("A"),
            }
        }
        _ => Cow::Borrowed(base_unit),
    }
}

/// Derive the decimal point position from mode and range code.
///
/// Returns the number of decimal places (0-4). These are [UNVERIFIED]
/// best guesses from the decompiled range switch statements.
fn decimal_places(mode: u8, range: u8) -> u8 {
    match mode {
        // DC V / AC V: 4 ranges with decreasing decimal places
        0x01 | 0x02 => match range {
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 3,
            _ => 1,
        },
        // DC mV: fixed 2 decimal places
        0x03 => 2,
        // Resistance: varies by sub-range
        0x04 => match range {
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 2,
            5 => 3,
            6 => 2,
            _ => 1,
        },
        // Default: 1 decimal place
        _ => 1,
    }
}

/// Parse a UT803/UT804 measurement from 14 data nibbles.
///
/// The nibbles are the low 4 bits of each FS9721 byte, in order (nibble 1-14).
pub(crate) fn parse_measurement(nibbles: &[u8]) -> Result<Measurement> {
    if nibbles.len() < 14 {
        return Err(Error::invalid_response(
            format!(
                "fs9721 payload too short: {} nibbles, expected 14",
                nibbles.len()
            ),
            nibbles,
        ));
    }

    // Validate format markers (nibbles 10-11).
    // The manufacturer app (UT804.exe) only enters the main parse path when
    // nibble 10 = 0x0D and nibble 11 = 0x0A. Non-matching frames are skipped.
    if nibbles[9] != 0x0D || nibbles[10] != 0x0A {
        debug!(
            "fs9721: skipping frame with non-standard markers: nib10={:#x} nib11={:#x}",
            nibbles[9], nibbles[10]
        );
        return Err(Error::invalid_response(
            format!(
                "fs9721 format markers invalid: expected 0D 0A, got {:02X} {:02X}",
                nibbles[9], nibbles[10]
            ),
            nibbles,
        ));
    }

    // Mode code from nibble 7 (0-indexed: nibbles[6])
    let mode_code = nibbles[6];

    // Range code from nibble 6 (0-indexed: nibbles[5])
    let range_code = nibbles[5];

    // Look up mode
    let (mode_name, base_unit) = if let Some((m, u)) = lookup_mode(mode_code) {
        (Cow::Borrowed(m), u)
    } else {
        warn!("fs9721: unknown mode code {mode_code:#04x}");
        (Cow::Owned(format!("Unknown({mode_code:#04x})")), "")
    };

    let unit = derive_unit(mode_code, range_code, base_unit);

    // AC/DC from nibble 8 (0-indexed: nibbles[7])
    // Also check nibble 1-2 flag mode: when nibble 1 = 0x0A, nibble 2 = 0x0C means AC.
    let acdc_nibble = nibbles[7];
    let nibble1_ac = nibbles[0] == 0x0A && nibbles[1] == 0x0C;
    let mode_with_acdc = match acdc_nibble {
        1 => mode_name.clone(), // explicit AC
        3 => {
            // AC+DC
            Cow::Owned(format!(
                "AC+DC {}",
                mode_name
                    .trim_start_matches("AC ")
                    .trim_start_matches("DC ")
            ))
        }
        _ => mode_name.clone(), // 0 = default, 2 = DC
    };
    // Determine DC flag: only set for modes that are inherently DC.
    // Modes like Hz, Capacitance, hFE, Diode, Continuity are neither AC nor DC.
    let is_dc_mode = mode_name.starts_with("DC");
    let dc = match acdc_nibble {
        1 => false,                     // explicit AC
        2 => true,                      // explicit DC
        3 => false,                     // AC+DC
        _ => is_dc_mode && !nibble1_ac, // default: DC only if mode is inherently DC
    };

    // Status flags from nibble 9 (0-indexed: nibbles[8])
    // Bit decomposition from UT804 decompilation (lines 224244-224283):
    //   if value >= 8: value -= 8  (strip bit 3, purpose unknown)
    //   if value >= 4: value -= 4  → HOLD active
    //   if value == 1:             → AUTO active (exactly 1, not just bit 0)
    let status = nibbles[8];
    let hold = status & 0x04 != 0; // bit 2 [VENDOR]
    let auto_range = (status & 0x03) == 0x01; // bit 0 set AND bit 1 clear [VENDOR]

    // Extract digits from nibbles 1-5 (0-indexed: nibbles[0..5])
    //
    // When nibble 1 = 0x0A: flag mode (nibble 2 carries AC/DC subtype, not a digit)
    //   → digits from nibbles 3-5 only (3 digits)
    // When nibble 1 ≠ 0x0A: digit mode
    //   → digits from nibbles 1-5 (up to 5 digits, with 0x0A = blank)
    let (digits, negative) = extract_digits(nibbles);

    // Build display string with decimal point
    let dp = decimal_places(mode_code, range_code) as usize;
    let display_str = format_display(&digits, dp);

    // Parse numeric value
    let value = if digits.is_empty() {
        // Idle/clear frame (nibble 4 = 0x0B) — no digit data available.
        MeasuredValue::Overload
    } else if digits.contains(&b'L') {
        MeasuredValue::Overload
    } else {
        let trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();
        match trimmed.parse::<f64>() {
            Ok(mut v) => {
                if negative {
                    v = -v;
                }
                MeasuredValue::Normal(v)
            }
            Err(_) => {
                warn!("fs9721: could not parse display value: {display_str:?}");
                MeasuredValue::Overload
            }
        }
    };

    let flags = StatusFlags {
        hold,
        auto_range,
        dc,
        ..Default::default()
    };

    Ok(Measurement {
        timestamp: Instant::now(),
        mode: mode_with_acdc,
        mode_raw: mode_code as u16,
        range_raw: range_code,
        value,
        unit,
        range_label: Cow::Borrowed(""), // [UNVERIFIED] — need range tables
        progress: None,
        display_raw: Some(display_str),
        flags,
        aux_values: vec![],
        raw_payload: nibbles.to_vec(),
    })
}

/// Extract digit characters from the nibble data.
///
/// Returns (digit_chars, is_negative). Digit chars are b'0'-b'9' or b'L' for
/// overload. The sign encoding is [UNVERIFIED] — currently we don't detect
/// negative values from the nibble data.
fn extract_digits(nibbles: &[u8]) -> (Vec<u8>, bool) {
    let digit_nibbles = if nibbles[0] == 0x0A {
        // Flag mode: nibble 1 = 0x0A (flag indicator), nibble 2 = AC/DC subtype.
        // Neither is a digit. Digits come from nibbles 3-5 only.
        &nibbles[2..5]
    } else if nibbles[3] == 0x0B {
        // Idle/clear frame: nibble 4 = 0x0B means no data. The manufacturer
        // app (UT804.exe, line 223828) clears all digit displays in this case.
        // Return empty digits — the caller will treat this as an unparseable value.
        return (vec![], false);
    } else {
        // Digit mode: nibbles 1-5 are all digits
        &nibbles[0..5]
    };

    let mut digits = Vec::with_capacity(digit_nibbles.len());
    for &n in digit_nibbles {
        match n {
            0x00..=0x09 => digits.push(b'0' + n),
            0x0A => {}                 // blank — skip
            0x0C => digits.push(b'L'), // overload [DEDUCED from FS9721 convention]
            _ => digits.push(b'?'),    // unknown encoding
        }
    }

    // Sign: [UNVERIFIED] — we don't know how negative values are encoded.
    // Possible locations: a specific nibble value, nibble 12-14, or nibble 9 bit.
    let negative = false;

    (digits, negative)
}

/// Format a display string from digit characters with decimal point insertion.
///
/// When `dp >= digit_str.len()`, leading zeros and "0." are prepended
/// (e.g., digits "234" with dp=3 → "0.234", digits "5" with dp=3 → "0.005").
fn format_display(digits: &[u8], dp: usize) -> String {
    let digit_str: String = digits.iter().map(|&b| b as char).collect();

    if dp == 0 {
        return digit_str;
    }

    if dp >= digit_str.len() {
        // Need leading zeros: e.g., "234" with dp=3 → "0.234"
        let leading_zeros = dp - digit_str.len();
        let mut result = String::with_capacity(dp + 2);
        result.push_str("0.");
        for _ in 0..leading_zeros {
            result.push('0');
        }
        result.push_str(&digit_str);
        return result;
    }

    let insert_pos = digit_str.len() - dp;
    let mut result = String::with_capacity(digit_str.len() + 1);
    result.push_str(&digit_str[..insert_pos]);
    result.push('.');
    result.push_str(&digit_str[insert_pos..]);
    result
}

// --- Protocol trait implementation ---

const FS9721_COMMANDS: &[&str] = &[];

/// Protocol implementation for UT803/UT804 bench multimeters.
pub struct Fs9721Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Fs9721Protocol {
    pub(crate) fn new_ut803() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "FS9721",
                model_name: "UNI-T UT803",
                stability: Stability::Experimental,
                supported_commands: FS9721_COMMANDS,
            },
        }
    }

    pub(crate) fn new_ut804() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "FS9721",
                model_name: "UNI-T UT804",
                stability: Stability::Experimental,
                supported_commands: FS9721_COMMANDS,
            },
        }
    }
}

impl Protocol for Fs9721Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // The CH9325 transport handles baud rate configuration (2400 baud).
        // The meter streams continuously once the CH9325 is configured —
        // no trigger byte needed. [UNVERIFIED] whether 0x5A helps.
        debug!("fs9721: init (no trigger needed, meter streams on CH9325 connect)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // The FS9721 extractor handles false starts internally (no Err from framing).
        // Use accept_fn to skip non-data frames (wrong format markers) so that
        // read_frame keeps trying until it gets a valid data frame.
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_fs9721,
            |nibbles| nibbles.len() >= 12 && nibbles[9] == 0x0D && nibbles[10] == 0x0A,
            FrameErrorRecovery::Propagate,
            "fs9721",
            &framing::FS9721_HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        // UT803/UT804 don't support remote commands over USB
        Err(Error::UnsupportedCommand(command.to_string()))
    }

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
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
                id: "ohm",
                instruction: "Set meter to Resistance (Ω)",
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
                id: "diode",
                instruction: "Set meter to Diode",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "cont",
                instruction: "Set meter to Continuity",
                command: None,
                samples: 5,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 14-nibble payload with the proprietary UT803/UT804 format.
    ///
    /// Nibble layout: [nib1, nib2, nib3, nib4, nib5, range, mode, acdc, flags, 0xD, 0xA, 0, 0, 0]
    fn make_payload(digit_nibbles: &[u8], range: u8, mode: u8, acdc: u8, flags: u8) -> Vec<u8> {
        let mut p = vec![0u8; 14];
        for (i, &d) in digit_nibbles.iter().enumerate().take(5) {
            p[i] = d;
        }
        p[5] = range;
        p[6] = mode;
        p[7] = acdc;
        p[8] = flags;
        p[9] = 0x0D; // format marker
        p[10] = 0x0A; // format marker
        p
    }

    #[test]
    fn parse_dcv_flag_mode() {
        // Flag mode: nibble 1 = 0x0A, nibble 2 = DC flag (not 0x0C).
        // Digits from nibbles 3-5 only: [2, 3, 4] → "23.4" with range 2 (1 dp)
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.mode_raw, 1);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 23.4).abs() < 0.01));
    }

    #[test]
    fn parse_dcv_digit_mode() {
        // Digit mode: nibble 1 ≠ 0x0A → all 5 nibbles are digits.
        // Digits [0, 1, 2, 3, 4] → "0123.4" with range 2 (1 dp)
        let payload = make_payload(&[0, 1, 2, 3, 4], 2, 0x01, 2, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 123.4).abs() < 0.01));
    }

    #[test]
    fn parse_acv() {
        // Mode 2 (AC V), acdc=1 (AC), range 3 (2 dp)
        // Flag mode: nibble 2 = 0x0C (AC). Digits from nibbles 3-5: [5, 6, 7]
        let payload = make_payload(&[0x0A, 0x0C, 5, 6, 7], 3, 0x02, 1, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert!(!m.flags.dc);
    }

    #[test]
    fn parse_resistance() {
        // Mode 4 (Ω), range 3 (kΩ, 3 dp). Digits: [2, 3, 4]
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 3, 0x04, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
    }

    #[test]
    fn parse_capacitance() {
        // Mode 5 (Cap), range 4 (µF). Digits: [4, 7, 0]
        let payload = make_payload(&[0x0A, 0x00, 4, 7, 0], 4, 0x05, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Capacitance");
        assert_eq!(m.unit, "µF");
    }

    #[test]
    fn parse_frequency() {
        // Mode 7 (Hz), range 0 (Hz). Digits: [0, 0, 0]
        let payload = make_payload(&[0x0A, 0x00, 0, 0, 0], 0, 0x07, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Hz");
        assert_eq!(m.unit, "Hz");
    }

    #[test]
    fn parse_continuity() {
        let payload = make_payload(&[0x0A, 0x00, 0, 0, 0], 1, 0x0D, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Continuity");
        assert_eq!(m.unit, "Ω");
    }

    #[test]
    fn parse_hold_flag() {
        // Nibble 9 bit 2 = HOLD
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 0, 0x04);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_auto_flag() {
        // Nibble 9: exactly 1 = AUTO (bit 0 set, bit 1 clear)
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 0, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);
    }

    #[test]
    fn parse_auto_flag_not_set_when_bit1_also_set() {
        // Nibble 9 = 3 (bits 0+1): AUTO should NOT be set per decompilation
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 0, 0x03);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_acdc_mode() {
        // AC+DC (acdc = 3)
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 3, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.mode.contains("AC+DC"));
        assert!(!m.flags.dc);
    }

    #[test]
    fn parse_five_digit_mode() {
        // nibble 1 ≠ 0x0A → 5 digit values
        let payload = make_payload(&[3, 9, 9, 9, 0x0A], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        // nibble 5 = 0x0A → blank, so only 4 digits: "3999"
        // with 1 dp from range 2 → "399.9"
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 399.9).abs() < 0.1));
    }

    #[test]
    fn parse_blank_digit_skipped() {
        // Flag mode: nibbles 3-5. nibble 5 = 0x0A → blank.
        // Digits: [2, 3] (blank skipped) → "2.3" with 1 dp
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 0x0A], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("2.3"));
    }

    #[test]
    fn parse_unknown_mode() {
        let payload = make_payload(&[0x0A, 0x00, 0, 0, 0], 0, 0x00, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.mode.contains("Unknown"));
    }

    #[test]
    fn parse_all_valid_modes() {
        for &(code, _, _) in MODE_TABLE {
            let payload = make_payload(&[0x0A, 0x00, 1, 2, 3], 1, code, 0, 0);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.contains("Unknown"),
                "mode {code:#04x} should be known"
            );
        }
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x0A, 0x01, 0x02];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn dc_flag_set_for_dc_mode() {
        // Mode 1 (DC V) with acdc = 0 → DC because mode name starts with "DC"
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.dc);
    }

    #[test]
    fn dc_flag_clear_for_ac_mode_name() {
        // Mode 2 (AC V) with acdc = 0 → NOT DC because mode name starts with "AC"
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x02, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.dc);
    }

    #[test]
    fn dc_flag_clear_for_explicit_ac() {
        // acdc = 1 → AC
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 2, 0x01, 1, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.dc);
    }

    #[test]
    fn dc_flag_clear_for_nibble2_ac() {
        // nibble 1 = 0x0A, nibble 2 = 0x0C → AC via flag mode
        let payload = make_payload(&[0x0A, 0x0C, 2, 3, 4], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.dc);
    }

    #[test]
    fn nibble2_0c_not_treated_as_digit() {
        // Flag mode: nibble 2 = 0x0C (AC flag, not a digit).
        // Only nibbles 3-5 are digits: [1, 2, 3] → "12.3" with 1 dp
        let payload = make_payload(&[0x0A, 0x0C, 1, 2, 3], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.3"));
    }

    #[test]
    fn parse_overload() {
        // Nibble value 0x0C → 'L' (overload, from FS9721 convention)
        let payload = make_payload(&[0x0A, 0x00, 0x0C, 0, 0], 1, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_zero_reading() {
        // All-zero digits: [0, 0, 0] → "00.0" with 1 dp → value 0.0
        let payload = make_payload(&[0x0A, 0x00, 0, 0, 0], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn dc_flag_false_for_non_acdc_modes() {
        // Hz, Capacitance, hFE, Diode, Continuity are neither AC nor DC.
        // dc flag should be false for these even with acdc nibble = 0.
        for &mode in &[0x05, 0x06, 0x07, 0x08, 0x09, 0x0D, 0x0E] {
            let payload = make_payload(&[0x0A, 0x00, 1, 2, 3], 1, mode, 0, 0);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.flags.dc,
                "mode {mode:#04x} ({}) should not have dc=true",
                m.mode
            );
        }
    }

    #[test]
    fn dc_flag_true_for_explicit_dc_nibble() {
        // acdc = 2 (explicit DC) should set dc=true even for non-DC modes
        let payload = make_payload(&[0x0A, 0x00, 1, 2, 3], 1, 0x07, 2, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.dc);
    }

    #[test]
    fn format_display_dp_equals_digit_count() {
        // 3 digits with dp=3: "234" → "0.234" (not "234")
        let digits = vec![b'2', b'3', b'4'];
        assert_eq!(format_display(&digits, 3), "0.234");
    }

    #[test]
    fn format_display_dp_exceeds_digit_count() {
        // 1 digit with dp=3: "5" → "0.005"
        let digits = vec![b'5'];
        assert_eq!(format_display(&digits, 3), "0.005");
    }

    #[test]
    fn format_display_dp_zero() {
        let digits = vec![b'1', b'2', b'3'];
        assert_eq!(format_display(&digits, 0), "123");
    }

    #[test]
    fn format_display_empty_digits() {
        // Empty digits with dp=1: dp >= len (1 >= 0), so "0." + 1 leading zero + "" = "0.0"
        let digits: Vec<u8> = vec![];
        assert_eq!(format_display(&digits, 1), "0.0");
    }

    #[test]
    fn parse_invalid_markers_rejected() {
        // nibble 10 != 0x0D → rejected
        let mut payload = make_payload(&[0x0A, 0x00, 1, 2, 3], 1, 0x01, 0, 0);
        payload[9] = 0x00; // wrong marker
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn parse_idle_frame_nibble4_0b() {
        // Digit mode with nibble 4 = 0x0B → idle/clear frame.
        // Empty digits → parse failure → Overload (no crash).
        let payload = make_payload(&[0, 1, 2, 0x0B, 4], 2, 0x01, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn resistance_flag_mode_dp3() {
        // Resistance mode, range 3 (kΩ, dp=3), flag mode (3 digits: [2, 3, 4])
        // Should produce "0.234" → value 0.234
        let payload = make_payload(&[0x0A, 0x00, 2, 3, 4], 3, 0x04, 0, 0);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("0.234"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 0.234).abs() < 0.001));
    }
}
