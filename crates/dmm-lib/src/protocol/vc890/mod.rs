//! Voltcraft VC-890 multimeter protocol.
//!
//! Polled protocol: host sends 0x5E measurement request, meter responds
//! with a 66-byte live data frame. Same AB CD + BE16 framing as UT61E+.
//!
//! Key differences from VC-880:
//! - Polled (request/response) instead of streaming
//! - 60,000 counts (vs 40,000) — range values 6/60/600 instead of 4/40/400
//! - 66-byte frames (vs 39) — more display fields
//! - Different function code assignments (remapped)
//! - OLED display, ES51997P + EFM32 MCU chipset
//!
//! Based on ILSpy decompilation of Voltsoft DMSShare.dll (VC890Obj,
//! VC890Reading classes).
//! See docs/research/vc880/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::Instant;

/// Live data message type byte.
const MSG_TYPE_LIVE_DATA: u8 = 0x01;

/// Measurement request command (polled model).
const CMD_GET_MEASUREMENT: u8 = 0x5E;

/// Minimum payload length for a VC890 live data frame.
/// Payload = type(1) + function(1) + range(1) + value1(7) + value2(8) +
///   value3(10) + value4(8) + value5(8) + freq_unit(3) + value6(4) +
///   bar(2) + status(8) = 61 bytes.
const LIVE_DATA_PAYLOAD_LEN: usize = 61;

/// VC-890 function code table: (code, mode_name, base_unit).
///
/// Note: function codes are DIFFERENT from VC-880 — remapped!
/// From VC890Reading.SetDeviceMode_And_Unit_And_Range() in DMSShare.dll.
const FUNCTION_TABLE: &[(u8, &str, &str)] = &[
    (0x00, "AC V", "V"),
    (0x01, "ACV LPF", "V"),
    (0x02, "DC V", "V"),
    (0x03, "AC+DC V", "V"),
    (0x04, "DC mV", "mV"),
    (0x05, "Frequency", "Hz"),
    (0x06, "Duty %", "%"),
    (0x07, "Ω", "Ω"),
    (0x08, "Continuity", "Ω"),
    (0x09, "Diode", "V"),
    (0x0A, "Capacitance", "F"),
    (0x0B, "°C", "°C"),
    (0x0C, "°F", "°F"),
    (0x0D, "DC µA", "µA"),
    (0x0E, "AC µA", "µA"),
    (0x0F, "DC mA", "mA"),
    (0x10, "AC mA", "mA"),
    (0x11, "DC A", "A"),
    (0x12, "AC A", "A"),
];

/// Range table entry.
struct RangeEntry {
    unit_override: &'static str,
    range_label: &'static str,
}

/// Look up range info for a function code and range index.
/// VC-890 has 60,000 counts — range values are 6/60/600 (not 4/40/400).
fn lookup_range(function: u8, range_idx: u8) -> Option<(&'static str, &'static str)> {
    let table: &[RangeEntry] = match function {
        // ACV, ACV LPF, DCV, AC+DC V — voltage ranges
        0x00..=0x03 => &[
            RangeEntry {
                unit_override: "",
                range_label: "6V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "60V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "600V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "1000V",
            },
        ],
        // DC mV
        0x04 => &[RangeEntry {
            unit_override: "",
            range_label: "600mV",
        }],
        // Frequency
        0x05 => &[
            RangeEntry {
                unit_override: "Hz",
                range_label: "60Hz",
            },
            RangeEntry {
                unit_override: "Hz",
                range_label: "600Hz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "6kHz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "60kHz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "600kHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "6MHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "60MHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "600MHz",
            },
        ],
        // Impedance (Resistance)
        0x07 => &[
            RangeEntry {
                unit_override: "Ω",
                range_label: "600Ω",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "6kΩ",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "60kΩ",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "600kΩ",
            },
            RangeEntry {
                unit_override: "MΩ",
                range_label: "6MΩ",
            },
            RangeEntry {
                unit_override: "MΩ",
                range_label: "60MΩ",
            },
        ],
        // Capacitance
        0x0A => &[
            RangeEntry {
                unit_override: "nF",
                range_label: "60nF",
            },
            RangeEntry {
                unit_override: "nF",
                range_label: "600nF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "6µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "60µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "600µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "6000µF",
            },
            RangeEntry {
                unit_override: "mF",
                range_label: "60mF",
            },
        ],
        // DC/AC µA
        0x0D | 0x0E => &[
            RangeEntry {
                unit_override: "",
                range_label: "600µA",
            },
            RangeEntry {
                unit_override: "",
                range_label: "6000µA",
            },
        ],
        // DC/AC mA
        0x0F | 0x10 => &[
            RangeEntry {
                unit_override: "",
                range_label: "60mA",
            },
            RangeEntry {
                unit_override: "",
                range_label: "600mA",
            },
        ],
        // DC/AC A
        0x11 | 0x12 => &[RangeEntry {
            unit_override: "",
            range_label: "10A",
        }],
        // Single-range functions
        _ => return None,
    };

    let idx = range_idx as usize;
    table.get(idx).map(|e| {
        let unit = if e.unit_override.is_empty() {
            FUNCTION_TABLE
                .iter()
                .find(|(c, _, _)| *c == function)
                .map(|(_, _, u)| *u)
                .unwrap_or("")
        } else {
            e.unit_override
        };
        (unit, e.range_label)
    })
}

const VC890_COMMANDS: &[&str] = &[
    "hold",
    "rel",
    "max_min_avg",
    "exit_max_min_avg",
    "range_auto",
    "range_manual",
    "light",
    "select",
];

/// Protocol implementation for the Voltcraft VC-890.
pub struct Vc890Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Default for Vc890Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Vc890Protocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "VC890",
                model_name: "Voltcraft VC-890",
                stability: Stability::Experimental,
                supported_commands: VC890_COMMANDS,
                verification_issue: Some(14),
            },
        }
    }
}

impl Protocol for Vc890Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // VC-890 is polled — no init needed. The meter responds to
        // individual measurement requests (0x5E).
        debug!("vc890: init (polled model, no trigger needed)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // Send measurement request (0x5E) — same command as UT61E+
        let request = super::vc8x0_common::build_command(CMD_GET_MEASUREMENT);
        transport.write(&request)?;

        // Read response
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_be16,
            |p| !p.is_empty() && p[0] == MSG_TYPE_LIVE_DATA,
            FrameErrorRecovery::SkipAndRetry,
            "vc890",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        use super::vc8x0_common;
        let cmd_byte = vc8x0_common::command_byte(command)?;
        let frame = vc8x0_common::build_command(cmd_byte);
        debug!("vc890: sending command {command} ({cmd_byte:#04x})");
        transport.write(&frame)?;
        Ok(())
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        super::vc8x0_common::read_device_name(&mut self.rx_buf, transport, "vc890")
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
                id: "acdcv",
                instruction: "Set meter to AC+DC V",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "dcmv",
                instruction: "Set meter to DC mV",
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
                id: "acua",
                instruction: "Set meter to AC µA",
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
                id: "acma",
                instruction: "Set meter to AC mA",
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
                id: "tempc",
                instruction: "Set meter to Temperature °C",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "tempf",
                instruction: "Set meter to Temperature °F",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "lpf",
                instruction: "Set meter to ACV Low-Pass Filter",
                command: None,
                samples: 5,
            },
        ]
    }
}

/// Parse a VC890 live data payload (61+ bytes).
///
/// Frame layout (from VC890Reading.SetReadingValue + SetStatus):
///   payload[0]     = type byte (0x01)
///   payload[1]     = function code (0x00-0x12)
///   payload[2]     = range byte (0x30-based)
///   payload[3..10] = value 1: main display (7 ASCII bytes)
///   payload[10..18]= value 2: sub display (8 ASCII bytes)
///   payload[18..28]= value 3: (10 bytes)
///   payload[28..36]= value 4: (8 bytes)
///   payload[36..44]= value 5: (8 bytes)
///   payload[44..47]= second freq unit (3 bytes)
///   payload[47..51]= value 6: (4 bytes)
///   payload[51..53]= bar graph (2 bytes)
///   payload[53]    = status 0: COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
///   payload[54]    = status 1: Rel(0), Avg(1), Min(2), Max(3)
///   payload[55]    = status 2: Hold(0), Manual(1), OL1(2), OL2(3)
///   payload[56]    = status 3: AutoPower(0), Warning(1), Loz(2), Void(3)
///   payload[57]    = status 4: OuterSel(0), Pass(1), Comp(2), Log_h(3)
///   payload[58]    = status 5: Mem(0), BarPol(1), Clr(2), Shift(3)
///   payload[59]    = battery level (low nibble)
///   payload[60]    = misplug warning (low nibble: 0=none, 1=mA err, 2=A err)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    if payload.len() < LIVE_DATA_PAYLOAD_LEN {
        return Err(Error::invalid_response(
            format!(
                "vc890 payload too short: {} bytes, expected {}",
                payload.len(),
                LIVE_DATA_PAYLOAD_LEN
            ),
            payload,
        ));
    }

    let function_code = payload[1];
    let range_raw = payload[2];
    let main_display = &payload[3..10];

    // Status bytes start at payload[53] (msg[56] in the raw frame)
    let status_bytes = &payload[53..61];

    // Look up function code
    let (mode, base_unit): (Cow<'static, str>, &'static str) = if let Some((_, name, unit)) =
        FUNCTION_TABLE.iter().find(|(c, _, _)| *c == function_code)
    {
        (Cow::Borrowed(name), unit)
    } else {
        warn!("vc890: unknown function code {function_code:#04x}");
        (Cow::Owned(format!("Unknown({function_code:#04x})")), "")
    };

    // Decode range
    let range_idx = range_raw.wrapping_sub(0x30);
    let (unit, range_label) = if let Some((u, r)) = lookup_range(function_code, range_idx) {
        (u, r)
    } else {
        (base_unit, "")
    };

    // Parse main display
    let display_str = String::from_utf8_lossy(main_display).to_string();
    let display_trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Extract status flags
    let ol1 = status_bytes[2] & 0x04 != 0; // OL1 bit
    let rel = status_bytes[1] & 0x01 != 0;
    let min_flag = status_bytes[1] & 0x04 != 0;
    let max_flag = status_bytes[1] & 0x08 != 0;
    let hold = status_bytes[2] & 0x01 != 0;
    let auto_range = status_bytes[2] & 0x02 == 0; // Manual bit inverted
    let hv_warning = status_bytes[3] & 0x02 != 0; // Warning bit
    // Battery is a nibble value at status_bytes[6], not a flag bit
    let low_battery = status_bytes[6] & 0x0F >= 3; // heuristic: ≥3 = low [UNVERIFIED]

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery,
        hv_warning,
        dc: false,
        peak_max: false,
        peak_min: false,
        ..Default::default()
    };

    // Parse numeric value
    let value = if ol1 || display_trimmed.contains("OL") || display_trimmed.contains("---") {
        MeasuredValue::Overload
    } else {
        match display_trimmed.parse::<f64>() {
            Ok(v) => MeasuredValue::Normal(v),
            Err(_) => {
                if !display_trimmed.is_empty() {
                    warn!("vc890: could not parse display value: {display_str:?}");
                }
                MeasuredValue::Overload
            }
        }
    };

    Ok(Measurement {
        timestamp: Instant::now(),
        mode,
        mode_raw: function_code as u16,
        range_raw,
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

    /// Build a minimal VC890 live data payload for testing.
    fn make_payload(function: u8, range: u8, main_display: &[u8; 7], status: [u8; 8]) -> Vec<u8> {
        let mut p = vec![MSG_TYPE_LIVE_DATA, function, range];
        p.extend_from_slice(main_display); // value1 (7)
        p.extend_from_slice(b"        "); // value2 (8)
        p.extend_from_slice(b"          "); // value3 (10)
        p.extend_from_slice(b"        "); // value4 (8)
        p.extend_from_slice(b"        "); // value5 (8)
        p.extend_from_slice(b"   "); // freq_unit (3)
        p.extend_from_slice(b"    "); // value6 (4)
        p.extend_from_slice(b"  "); // bar (2)
        p.extend_from_slice(&status); // status (8 bytes: 6 flag bytes + battery + misplug)
        assert_eq!(p.len(), LIVE_DATA_PAYLOAD_LEN);
        p
    }

    fn zero_status() -> [u8; 8] {
        [0u8; 8]
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x02, 0x31, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "60V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        // Note: VC-890 function 0x00 = ACV (different from VC-880!)
        let payload = make_payload(0x00, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "6V");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x07, 0x32, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "60kΩ");
    }

    #[test]
    fn parse_overload_flag() {
        let mut status = zero_status();
        status[2] = 0x04; // OL1
        let payload = make_payload(0x07, 0x30, b"     OL", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        let mut status = zero_status();
        status[2] = 0x01;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        let mut status = zero_status();
        status[1] = 0x01;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        let mut status = zero_status();
        status[1] = 0x08; // Max
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);

        let mut status = zero_status();
        status[1] = 0x04; // Min
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.min);
    }

    #[test]
    fn parse_auto_range() {
        let payload = make_payload(0x02, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        let mut status = zero_status();
        status[2] = 0x02; // Manual bit
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_all_valid_functions() {
        for &(code, _, _) in FUNCTION_TABLE {
            let payload = make_payload(code, 0x30, b"  1.234", zero_status());
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "function {code:#04x} should be known"
            );
        }
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x01, 0x00, 0x30];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn range_60k_counts() {
        // Verify 60K count range values (6/60/600 instead of 4/40/400)
        let payload = make_payload(0x02, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_label, "6V"); // not 4V like VC-880

        let payload = make_payload(0x07, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_label, "600Ω"); // not 400Ω like VC-880
    }

    #[test]
    fn function_codes_differ_from_vc880() {
        // VC-890: 0x00 = ACV, 0x02 = DCV
        // VC-880: 0x00 = DCV, 0x05 = ACV
        let payload = make_payload(0x00, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V"); // NOT "DC V"
    }

    #[test]
    fn send_command_builds_correct_frame() {
        let frame = super::super::vc8x0_common::build_command(CMD_GET_MEASUREMENT);
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xCD);
        assert_eq!(frame[2], 0x03);
        assert_eq!(frame[3], CMD_GET_MEASUREMENT);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }
}
