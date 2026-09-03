//! UT803/UT804 bench multimeter protocol.
//!
//! These meters use FS9721-style 14-byte framing (index nibble in high 4
//! bits) but with a **proprietary data encoding** — the data nibbles carry
//! structured measurement data (mode codes, range codes, digit values,
//! status flags), NOT raw LCD segment data.
//!
//! **UT803 and UT804 use different payload layouts** (2026-06 review,
//! re-derived from UT803.exe V1.01 / UT804.exe V2.00 with string constants
//! resolved from the recovered binaries and the vendor LCD fonts rendered):
//!
//! UT804 (nibble index = vendor 1-based char − 1):
//! - nibbles 0-4: digits 1-5 (MSD first; 0xA = blank)
//! - nibble 5: range (decimal point position via per-mode table)
//! - nibble 6: mode code (1-15)
//! - nibble 7: AC/DC (0=default, 1=AC, 2=DC, 3=AC+DC)
//! - nibble 8: status — bit 3 unknown, bit 2 = **negative sign** (duty-%
//!   selector in frequency mode), bits 1-0: AUTO when == 1
//! - nibbles 9-10: format markers 0xD 0xA (low nibbles of CR/LF)
//! - nibbles 11-13: never read by the vendor app; purpose unknown
//!
//! UT803:
//! - nibble 1: range
//! - nibbles 2-5: digits 1-4 (MSD first)
//! - nibble 6: mode code (different meanings from UT804!)
//! - nibble 7: bit 3 = alt-mode (RPM / °C-vs-°F), bit 2 = **negative
//!   sign**, bit 1 = unknown, bit 0 = overload
//! - nibble 8: bit 3 = HOLD, bits 2-1 = unknown indicators
//! - nibble 9: bit 3 = DC, bit 2 = AC, bit 1 = AUTO (else MANU)
//! - nibbles 0, 10-13: never read by the vendor app
//!
//! The decimal point value in the per-mode range tables is the position
//! FROM THE LEFT (point placed after digit position+1), matching the
//! vendor's display assembly.
//!
//! See docs/research/ut803/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{CaptureStep, DeviceProfile, Protocol, Stability, unknown_mode};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;

/// Which meter model the frames come from — the two share framing but
/// not payload layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Fs9721Model {
    Ut803,
    Ut804,
}

/// UT804 per-(mode, range) display info: mode name, unit, and decimal
/// point position from the left (point after digit `pos+1`; a position
/// past the last digit means an integer display).
///
/// From the UT804.exe parse function `FUN_00558a7c` unit-string appends
/// (ut804-decompiled.txt:224075-224184) and range switches
/// (223961-224033, 224129-224170), with unit glyphs resolved from the
/// vendor LCD fonts (`#`=°C, `?`=°F, `)`=diode, `&`=beeper, `*`=Ω).
fn ut804_mode_info(mode: u8, range: u8) -> Option<(&'static str, &'static str, u8)> {
    Some(match mode {
        // Modes 1 and 2 have byte-identical handlers; the AC/DC label
        // comes solely from nibble 7. Which dial sends 1 vs 2 is unknown.
        0x1 | 0x2 => (
            "V",
            "V",
            match range {
                1 => 0, // 3.999
                2 => 1, // 39.99
                3 => 2, // 399.9
                4 => 3, // 1000
                _ => return None,
            },
        ),
        0x3 => ("mV", "mV", 2), // 399.9 fixed
        0x4 => match range {
            1 => ("Ω", "Ω", 2), // 399.9 Ω
            2 => ("Ω", "kΩ", 0),
            3 => ("Ω", "kΩ", 1),
            4 => ("Ω", "kΩ", 2),
            5 => ("Ω", "MΩ", 0),
            6 => ("Ω", "MΩ", 1),
            _ => return None,
        },
        0x5 => match range {
            1 => ("Capacitance", "nF", 1),
            2 => ("Capacitance", "nF", 2),
            3 => ("Capacitance", "µF", 0),
            4 => ("Capacitance", "µF", 1),
            5 => ("Capacitance", "µF", 2),
            6 => ("Capacitance", "mF", 0),
            7 => ("Capacitance", "mF", 1),
            _ => return None,
        },
        0x6 => ("Temperature", "°C", 3),
        0x7 => match range {
            0 => ("µA", "µA", 2), // 399.9
            1 => ("µA", "µA", 3), // 3999
            _ => return None,
        },
        0x8 => match range {
            0 => ("mA", "mA", 1), // 39.99
            1 => ("mA", "mA", 2), // 399.9
            _ => return None,
        },
        0x9 => ("A", "A", 1), // 10.00
        0xA => ("Continuity", "Ω", 2),
        0xB => ("Diode", "V", 0),
        0xC => match range {
            0 => ("Frequency", "Hz", 1),
            1 => ("Frequency", "Hz", 2),
            2 => ("Frequency", "kHz", 0),
            3 => ("Frequency", "kHz", 1),
            4 => ("Frequency", "kHz", 2),
            5 => ("Frequency", "MHz", 0),
            6 => ("Frequency", "MHz", 1),
            7 => ("Frequency", "MHz", 2),
            _ => return None,
        },
        0xD => ("Temperature", "°F", 3),
        // Unknown glyph in the vendor font ("W"); possibly hFE/ADP.
        0xE => ("ADP", "", 3),
        // Unit string "mA%" — most plausibly 4-20 mA loop percentage.
        0xF => ("mA%", "mA%", 2),
        _ => return None,
    })
}

/// UT804 modes whose AC/DC nibble value 0 defaults to a DC label
/// (vendor applies "DC" to V/mV/µA/mA/A, ut804-decompiled.txt:224195-224215).
fn ut804_default_dc(mode: u8) -> bool {
    matches!(mode, 0x1 | 0x2 | 0x3 | 0x7 | 0x8 | 0x9)
}

/// UT803 per-(mode, range) display info, same convention as
/// [`ut804_mode_info`] but with 4 digits and the UT803's own mode codes
/// (ut803-decompiled.txt:224441-225068). `alt` is nibble 7 bit 3 (selects
/// RPM for frequency, °C vs °F for temperature).
fn ut803_mode_info(mode: u8, range: u8, alt: bool) -> Option<(&'static str, &'static str, u8)> {
    Some(match mode {
        0xB => match range {
            0 => ("V", "V", 0),   // 5.999
            1 => ("V", "V", 1),   // 59.99
            2 => ("V", "V", 2),   // 599.9
            3 => ("V", "V", 3),   // 1000
            4 => ("mV", "mV", 2), // 599.9 mV
            _ => return None,
        },
        0xD => match range {
            0 => ("µA", "µA", 2), // 599.9
            1 => ("µA", "µA", 3), // 5999
            _ => return None,
        },
        0xF => match range {
            0 => ("mA", "mA", 1), // 59.99
            1 => ("mA", "mA", 2), // 599.9
            _ => return None,
        },
        0x9 => ("A", "A", 1), // 10.00
        0x3 => match range {
            0 => ("Ω", "Ω", 2), // 599.9
            1 => ("Ω", "kΩ", 0),
            2 => ("Ω", "kΩ", 1),
            3 => ("Ω", "kΩ", 2),
            4 => ("Ω", "MΩ", 0),
            5 => ("Ω", "MΩ", 1),
            _ => return None,
        },
        0x2 => {
            // Frequency, or tachometer RPM when the alt bit is set.
            // Range 0's decimal position is [UNVERIFIED] (the vendor
            // handler for it is ambiguous); treat as integer Hz.
            let (name, unit) = if alt {
                ("Tachometer", "RPM")
            } else {
                ("Frequency", "Hz")
            };
            match (range, alt) {
                (0, _) => (name, unit, 3),
                (1, false) => (name, "kHz", 1),
                (2, false) => (name, "kHz", 2),
                (3, false) => (name, "MHz", 0),
                (4, false) => (name, "MHz", 1),
                (5, false) => (name, "MHz", 2),
                (1, true) => (name, "kRPM", 1),
                (2, true) => (name, "kRPM", 2),
                (3, true) => (name, "MRPM", 0),
                (4, true) => (name, "MRPM", 1),
                (5, true) => (name, "MRPM", 2),
                _ => return None,
            }
        }
        0x4 => {
            if alt {
                ("Temperature", "°C", 3)
            } else {
                ("Temperature", "°F", 3)
            }
        }
        0x5 => ("Continuity", "Ω", 3),
        0x1 => ("Diode", "V", 0), // 5.999
        0x6 => match range {
            0 => ("Capacitance", "nF", 0),
            1 => ("Capacitance", "nF", 1),
            2 => ("Capacitance", "nF", 2),
            3 => ("Capacitance", "µF", 0),
            4 => ("Capacitance", "µF", 1),
            5 => ("Capacitance", "µF", 2),
            6 => ("Capacitance", "mF", 0),
            7 => ("Capacitance", "mF", 1),
            _ => return None,
        },
        // Two indicators, no unit, possibly hFE.
        0xE => ("ADP", "", 3),
        _ => return None,
    })
}

/// Build the display string and numeric value from MSD-first digit
/// nibbles and a decimal position from the left (point after digit
/// `dp_pos+1`). Digit nibble 0xA renders as a blank (trailing blank =
/// 4-digit reading on the UT804's 5-digit field).
fn assemble_value(digits: &[u8], dp_pos: u8, negative: bool) -> Result<(String, f64)> {
    let mut s = String::with_capacity(digits.len() + 2);
    if negative {
        s.push('-');
    }
    let mut digit_count = 0usize;
    for &d in digits {
        match d {
            0x0..=0x9 => {
                s.push((b'0' + d) as char);
                digit_count += 1;
            }
            0xA => {} // blank digit
            _ => {
                return Err(Error::invalid_response(
                    format!("fs9721 invalid digit nibble {d:#04x}"),
                    digits,
                ));
            }
        }
        // Insert the decimal point after digit position dp_pos+1
        // (skipped when dp_pos points past the last digit = integer).
        if digit_count == dp_pos as usize + 1 && digit_count < digits.len() {
            s.push('.');
        }
    }
    let trimmed = s.trim_end_matches('.').to_string();
    let value: f64 = trimmed.parse().map_err(|_| {
        Error::invalid_response(format!("fs9721 unparseable value {trimmed:?}"), digits)
    })?;
    Ok((trimmed, value))
}

/// Parse a UT804 measurement payload (14 data nibbles).
pub(crate) fn parse_measurement_ut804(nibbles: &[u8]) -> Result<Measurement> {
    if nibbles.len() < 11 {
        return Err(Error::invalid_response(
            format!("fs9721 payload too short: {} nibbles", nibbles.len()),
            nibbles,
        ));
    }

    // Format markers (vendor chars 'D'/'A' = low nibbles of CR/LF).
    if nibbles[9] != 0x0D || nibbles[10] != 0x0A {
        return Err(Error::invalid_response(
            format!(
                "ut804 format markers {:#04x} {:#04x}, expected 0xD 0xA",
                nibbles[9], nibbles[10]
            ),
            nibbles,
        ));
    }

    let range = nibbles[5];
    let mode_code = nibbles[6];
    let acdc = nibbles[7];
    let status = nibbles[8];

    // Status nibble (vendor char 9, ut804-decompiled.txt:224244-224266):
    // bit 3 stripped (unknown), bit 2 = sign, remaining value == 1 → AUTO.
    let sign_bit = status & 0x4 != 0;
    let auto_range = status & 0x3 == 0x1;

    let (mode_name, unit, dp_pos) = match ut804_mode_info(mode_code, range) {
        Some(info) => info,
        None => {
            debug!("ut804: unknown mode/range {mode_code:#04x}/{range}");
            ("?", "", 3)
        }
    };

    // In frequency mode the sign bit selects the duty-cycle display
    // (ut804-decompiled.txt:224271-224283) — a negative frequency is
    // impossible, so the bit is reused.
    let (mode, negative, dp_pos, unit): (Cow<'static, str>, bool, u8, &'static str) =
        if mode_code == 0xC && sign_bit {
            (Cow::Borrowed("Duty %"), false, 2, "%")
        } else if mode_name == "?" {
            (unknown_mode(mode_code), sign_bit, dp_pos, unit)
        } else {
            // AC/DC labeling comes from nibble 7 for the V/mV/current
            // modes (0 = default DC); other modes keep their plain name.
            let label = match (acdc, ut804_default_dc(mode_code)) {
                (1, _) => match mode_name {
                    "V" => Some("AC V"),
                    "mV" => Some("AC mV"),
                    "µA" => Some("AC µA"),
                    "mA" => Some("AC mA"),
                    "A" => Some("AC A"),
                    _ => None,
                },
                (2, _) | (0, true) => match mode_name {
                    "V" => Some("DC V"),
                    "mV" => Some("DC mV"),
                    "µA" => Some("DC µA"),
                    "mA" => Some("DC mA"),
                    "A" => Some("DC A"),
                    _ => None,
                },
                (3, _) => match mode_name {
                    "V" => Some("AC+DC V"),
                    "mV" => Some("AC+DC mV"),
                    "µA" => Some("AC+DC µA"),
                    "mA" => Some("AC+DC mA"),
                    "A" => Some("AC+DC A"),
                    _ => None,
                },
                _ => None,
            };
            (
                Cow::Borrowed(label.unwrap_or(mode_name)),
                sign_bit,
                dp_pos,
                unit,
            )
        };

    let dc = matches!(acdc, 2 | 3) || (acdc == 0 && ut804_default_dc(mode_code));

    // Overload frames: digit 1 = 0xA. Vendor forces the displays to
    // "0L" (overload, possibly negative) when digit 2 == 0xC, or "L0"
    // → value 0.0 otherwise (ut804-decompiled.txt:223810-223823,
    // 224361-224391). An idle frame (digit 4 == 0xB) shows all zeros.
    let (value, display_raw) = if nibbles[0] == 0xA {
        if nibbles[1] == 0xC {
            (
                MeasuredValue::Overload,
                Some(if negative {
                    "-0L".to_string()
                } else {
                    "0L".to_string()
                }),
            )
        } else {
            (MeasuredValue::Normal(0.0), Some("L0".to_string()))
        }
    } else if nibbles[3] == 0xB {
        (MeasuredValue::Normal(0.0), Some("0".to_string()))
    } else {
        let (display, v) = assemble_value(&nibbles[0..5], dp_pos, negative)?;
        (MeasuredValue::Normal(v), Some(display))
    };

    let flags = StatusFlags {
        auto_range,
        dc,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: mode_code as u16,
        range_raw: range,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags,
        ..Measurement::from_payload(nibbles)
    })
}

/// Parse a UT803 measurement payload (14 data nibbles).
pub(crate) fn parse_measurement_ut803(nibbles: &[u8]) -> Result<Measurement> {
    if nibbles.len() < 10 {
        return Err(Error::invalid_response(
            format!("fs9721 payload too short: {} nibbles", nibbles.len()),
            nibbles,
        ));
    }

    let range = nibbles[1];
    let mode_code = nibbles[6];
    let nib8 = nibbles[7]; // vendor char 8
    let nib9 = nibbles[8]; // vendor char 9
    let nib10 = nibbles[9]; // vendor char 10

    let alt = nib8 & 0x8 != 0;
    let negative = nib8 & 0x4 != 0;
    let overload = nib8 & 0x1 != 0;
    // HOLD lights the LCDHold widget from char 9 bit 3
    // (ut803-decompiled.txt:225086); bits 2-1 drive unlabeled indicators.
    let hold = nib9 & 0x8 != 0;
    let dc = nib10 & 0x8 != 0;
    let auto_range = nib10 & 0x2 != 0;

    let (mode_name, unit, dp_pos) = match ut803_mode_info(mode_code, range, alt) {
        Some(info) => info,
        None => {
            debug!("ut803: unknown mode/range {mode_code:#04x}/{range}");
            ("?", "", 3)
        }
    };

    let mode: Cow<'static, str> = if mode_name == "?" {
        unknown_mode(mode_code)
    } else if mode_name == "V" || mode_name == "mV" {
        Cow::Borrowed(match (mode_name, dc) {
            ("V", true) => "DC V",
            ("V", false) => "AC V",
            ("mV", true) => "DC mV",
            _ => "AC mV",
        })
    } else {
        Cow::Borrowed(mode_name)
    };

    let (value, display_raw) = if overload {
        (
            MeasuredValue::Overload,
            Some(if negative {
                "-0L".to_string()
            } else {
                "0L".to_string()
            }),
        )
    } else {
        let (display, v) = assemble_value(&nibbles[2..6], dp_pos, negative)?;
        (MeasuredValue::Normal(v), Some(display))
    };

    let flags = StatusFlags {
        hold,
        auto_range,
        dc,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: mode_code as u16,
        range_raw: range,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags,
        ..Measurement::from_payload(nibbles)
    })
}

// --- Protocol trait implementation ---

const FS9721_COMMANDS: &[&str] = &[];

/// Known UT803 mode codes, used to filter garbage frames (the UT803
/// layout has no format markers to key on).
const UT803_MODES: &[u8] = &[0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x9, 0xB, 0xD, 0xE, 0xF];

/// Protocol implementation for UT803/UT804 bench multimeters.
pub struct Fs9721Protocol {
    rx_buf: Vec<u8>,
    model: Fs9721Model,
    profile: DeviceProfile,
}

impl Fs9721Protocol {
    pub(crate) fn new_ut803() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            model: Fs9721Model::Ut803,
            profile: DeviceProfile {
                family_name: "FS9721",
                model_name: "UNI-T UT803",
                stability: Stability::Experimental,
                supported_commands: FS9721_COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(15),
            },
        }
    }

    pub(crate) fn new_ut804() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            model: Fs9721Model::Ut804,
            profile: DeviceProfile {
                family_name: "FS9721",
                model_name: "UNI-T UT804",
                stability: Stability::Experimental,
                supported_commands: FS9721_COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(16),
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
        // The FS9721 extractor handles false starts internally (no Err
        // from framing). The accept_fn filters frames that can't be a
        // measurement for the model: UT804 frames carry 0xD 0xA markers
        // at nibbles 9-10; UT803 has no markers, so gate on a known mode
        // code instead.
        let model = self.model;
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_fs9721,
            |nibbles| match model {
                Fs9721Model::Ut804 => {
                    nibbles.len() >= 12 && nibbles[9] == 0x0D && nibbles[10] == 0x0A
                }
                Fs9721Model::Ut803 => nibbles.len() >= 12 && UT803_MODES.contains(&nibbles[6]),
            },
            FrameErrorRecovery::Propagate,
            "fs9721",
            &framing::FS9721_HEADER,
        )?;
        match self.model {
            Fs9721Model::Ut803 => parse_measurement_ut803(&payload),
            Fs9721Model::Ut804 => parse_measurement_ut804(&payload),
        }
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
                id: "dcv_negative",
                instruction: "Set meter to DC V with leads reversed (negative reading)",
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
                id: "ohm_ol",
                instruction: "Set meter to Resistance (Ω) with open leads (overload)",
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
            CaptureStep {
                id: "hold",
                instruction: "Press HOLD (wire encoding unknown — capture needed)",
                command: None,
                samples: 5,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 14-nibble UT804 payload.
    /// digits = MSD-first nibbles 0-4; then range, mode, acdc, status.
    fn ut804_payload(digits: &[u8; 5], range: u8, mode: u8, acdc: u8, status: u8) -> Vec<u8> {
        vec![
            digits[0], digits[1], digits[2], digits[3], digits[4], range, mode, acdc, status, 0x0D,
            0x0A, 0x0, 0x0, 0x0,
        ]
    }

    /// Build a 14-nibble UT803 payload.
    /// digits = MSD-first nibbles 2-5; range at nibble 1; mode at 6;
    /// nib8/nib9/nib10 at 7/8/9.
    fn ut803_payload(
        digits: &[u8; 4],
        range: u8,
        mode: u8,
        nib8: u8,
        nib9: u8,
        nib10: u8,
    ) -> Vec<u8> {
        vec![
            0x0, range, digits[0], digits[1], digits[2], digits[3], mode, nib8, nib9, nib10, 0x0,
            0x0, 0x0, 0x0,
        ]
    }

    // --- UT804 ---

    #[test]
    fn ut804_dcv_range1() {
        // DC V range 1: 3.999 full scale → decimal after digit 1.
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 1, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 3.999).abs() < 1e-9));
        assert_eq!(m.display_raw.as_deref(), Some("3.999"));
    }

    #[test]
    fn ut804_negative_sign_nibble9_bit2() {
        // Status nibble bit 2 = negative sign (NOT hold).
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(!m.flags.hold);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-9));
        assert_eq!(m.display_raw.as_deref(), Some("-12.34"));
    }

    #[test]
    fn ut804_auto_flag() {
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x1);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(m.flags.auto_range);
        // Sign bit set alongside: AUTO still derived after stripping.
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x5);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(m.flags.auto_range);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v < 0.0));
    }

    #[test]
    fn ut804_overload_nibble1() {
        // Digit1 = 0xA + digit2 = 0xC → overload.
        let p = ut804_payload(&[0xA, 0xC, 0, 0, 0], 1, 0x4, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));

        // Negative overload via status bit 2.
        let p = ut804_payload(&[0xA, 0xC, 0, 0, 0], 1, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw.as_deref(), Some("-0L"));
    }

    #[test]
    fn ut804_resistance_kilo_range() {
        // Ω range 2 = 39.99 kΩ? No: range 2 → kΩ with point after digit 1
        // (3.999 kΩ style).
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 2, 0x4, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 3.999).abs() < 1e-9));
    }

    #[test]
    fn ut804_frequency_and_duty() {
        // Mode 0xC range 2 = kHz, point after digit 1.
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0xC, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Frequency");
        assert_eq!(m.unit, "kHz");
        // Sign bit in frequency mode = duty-cycle display, not negative.
        let p = ut804_payload(&[5, 0, 0, 0, 0xA], 2, 0xC, 0, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Duty %");
        assert_eq!(m.unit, "%");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v > 0.0));
    }

    #[test]
    fn ut804_temperature_modes() {
        let p = ut804_payload(&[0, 0, 2, 5, 0xA], 0, 0x6, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Temperature");
        assert_eq!(m.unit, "°C");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 25.0).abs() < 1e-9));
        let p = ut804_payload(&[0, 0, 7, 7, 0xA], 0, 0xD, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.unit, "°F");
    }

    #[test]
    fn ut804_acv_label_from_acdc_nibble() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 1, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "AC V");
        assert!(!m.flags.dc);
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 3, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "AC+DC V");
    }

    #[test]
    fn ut804_current_modes() {
        // Mode 7 = µA (not Hz as the old table claimed).
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 0, 0x7, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC µA");
        assert_eq!(m.unit, "µA");
        // Mode 9 = A.
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 0, 0x9, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC A");
        assert_eq!(m.unit, "A");
    }

    #[test]
    fn ut804_idle_frame() {
        // Digit 4 == 0xB → idle, all displays zero.
        let p = ut804_payload(&[0, 0, 0, 0xB, 0], 1, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn ut804_bad_markers_rejected() {
        let mut p = ut804_payload(&[1, 2, 3, 4, 0xA], 1, 0x1, 2, 0x0);
        p[9] = 0x0;
        assert!(parse_measurement_ut804(&p).is_err());
    }

    #[test]
    fn ut804_five_digit_reading() {
        // All five digits present (no blank): 4-digit count plus extra digit.
        let p = ut804_payload(&[1, 2, 3, 4, 5], 2, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-9));
    }

    // --- UT803 ---

    #[test]
    fn ut803_dcv() {
        let p = ut803_payload(&[5, 9, 9, 9], 0, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 5.999).abs() < 1e-9));
    }

    #[test]
    fn ut803_negative_sign_nib8_bit2() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x4, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-9));
    }

    #[test]
    fn ut803_overload_nib8_bit0() {
        let p = ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x1, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn ut803_hold_nib9_bit3() {
        let p = ut803_payload(&[1, 0, 0, 0], 0, 0xB, 0x0, 0x8, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn ut803_auto_and_ac() {
        // nib10: bit 2 = AC, bit 1 = AUTO.
        let p = ut803_payload(&[2, 3, 0, 0], 1, 0xB, 0x0, 0x0, 0x6);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "AC V");
        assert!(m.flags.auto_range);
        assert!(!m.flags.dc);
    }

    #[test]
    fn ut803_mv_range4() {
        let p = ut803_payload(&[5, 9, 9, 9], 4, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "DC mV");
        assert_eq!(m.unit, "mV");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 599.9).abs() < 1e-9));
    }

    #[test]
    fn ut803_resistance_mega() {
        let p = ut803_payload(&[5, 9, 9, 9], 4, 0x3, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "MΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 5.999).abs() < 1e-9));
    }

    #[test]
    fn ut803_temperature_alt_bit() {
        // nib8 bit 3 set → °C; clear → °F.
        let p = ut803_payload(&[0, 0, 2, 5], 0, 0x4, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.unit, "°C");
        let p = ut803_payload(&[0, 0, 7, 7], 0, 0x4, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.unit, "°F");
    }

    #[test]
    fn ut803_tachometer_alt_bit() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "Tachometer");
        assert_eq!(m.unit, "kRPM");
    }

    #[test]
    fn ut803_unknown_mode_permissive() {
        let p = ut803_payload(&[1, 0, 0, 0], 0, 0x7, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(m.mode.starts_with("Unknown"));
    }

    #[test]
    fn payload_too_short() {
        assert!(parse_measurement_ut804(&[0x1, 0x2]).is_err());
        assert!(parse_measurement_ut803(&[0x1, 0x2]).is_err());
    }

    #[test]
    fn invalid_digit_nibble_errors() {
        let p = ut804_payload(&[1, 0xF, 0, 0, 0xA], 1, 0x1, 2, 0x0);
        assert!(parse_measurement_ut804(&p).is_err());
    }
}
