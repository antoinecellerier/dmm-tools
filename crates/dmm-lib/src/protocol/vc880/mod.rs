//! Voltcraft VC-880 / VC650BT multimeter protocol.
//!
//! Streaming protocol: meter streams live data continuously after the user
//! presses the PC button — no trigger command needed from the host.
//!
//! Frame format: identical to UT61E+ — AB CD header, BE16 checksum.
//! Reuses `extract_frame_abcd_be16()`.
//!
//! Live data frame (39 bytes): header(2) + length(1) + type(1) +
//!   function(1) + range(1) + main_value(7) + sub1(7) + sub2(7) +
//!   bar(3) + status(7) + checksum(2).
//!
//! Based on ILSpy decompilation of Voltsoft DMSShare.dll.
//! See docs/research/vc880/reverse-engineered-protocol.md

use crate::error::Result;
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability, check_len, unknown_mode};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;

/// Live data message type byte.
const MSG_TYPE_LIVE_DATA: u8 = 0x01;

/// Minimum payload length for a live data frame.
/// Payload from `extract_frame_abcd_be16` = everything between length byte
/// and checksum. For a 39-byte frame: type(1) + function(1) + range(1) +
/// main(7) + sub1(7) + sub2(7) + bar(3) + status(7) = 34 bytes.
const LIVE_DATA_PAYLOAD_LEN: usize = 34;

/// Function code table: (code, mode_name, unit).
///
/// From `SetDeviceMode_And_Unit_And_Range()` switch statement in
/// DMSShare.dll (line 16335). The unit here is the base unit; the
/// actual unit depends on the range byte (e.g., function 0x06 can be
/// Ω, kΩ, or MΩ depending on range).
const FUNCTION_TABLE: &[(u8, &str, &str)] = &[
    (0x00, "DC V", "V"),
    (0x01, "AC+DC V", "V"),
    (0x02, "DC mV", "mV"),
    (0x03, "Frequency", "Hz"),
    (0x04, "Duty %", "%"),
    (0x05, "AC V", "V"),
    (0x06, "Ω", "Ω"),
    (0x07, "Diode", "V"),
    (0x08, "Continuity", "Ω"),
    (0x09, "Capacitance", "F"),
    (0x0A, "°C", "°C"),
    (0x0B, "°F", "°F"),
    (0x0C, "DC µA", "µA"),
    (0x0D, "AC µA", "µA"),
    (0x0E, "DC mA", "mA"),
    (0x0F, "AC mA", "mA"),
    (0x10, "DC A", "A"),
    (0x11, "AC A", "A"),
    (0x12, "ACV LPF", "V"),
];

/// Range table entry: unit_override replaces the function's base unit
/// when non-empty. range_label is a human-readable string like "40kΩ".
struct RangeEntry {
    unit_override: &'static str,
    range_label: &'static str,
}

/// Look up range info for a given function code and range index.
/// Returns (unit, range_label) or None if the range index is out of bounds.
///
/// Range tables from DMSShare.dll `SetDeviceMode_And_Unit_And_Range()`
/// and cross-referenced against VC880 user manual pages 62-65.
fn lookup_range(function: u8, range_idx: u8) -> Option<(&'static str, &'static str)> {
    let table: &[RangeEntry] = match function {
        // DCV, ACV, AC+DC V, ACV LPF — all share voltage ranges
        0x00 | 0x01 | 0x05 | 0x12 => &[
            RangeEntry {
                unit_override: "",
                range_label: "4V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "40V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "400V",
            },
            RangeEntry {
                unit_override: "",
                range_label: "1000V",
            },
        ],
        // DC mV
        0x02 => &[RangeEntry {
            unit_override: "",
            range_label: "400mV",
        }],
        // Frequency
        0x03 => &[
            RangeEntry {
                unit_override: "Hz",
                range_label: "40Hz",
            },
            RangeEntry {
                unit_override: "Hz",
                range_label: "400Hz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "4kHz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "40kHz",
            },
            RangeEntry {
                unit_override: "kHz",
                range_label: "400kHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "4MHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "40MHz",
            },
            RangeEntry {
                unit_override: "MHz",
                range_label: "400MHz",
            },
        ],
        // Impedance (Resistance)
        0x06 => &[
            RangeEntry {
                unit_override: "Ω",
                range_label: "400Ω",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "4kΩ",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "40kΩ",
            },
            RangeEntry {
                unit_override: "kΩ",
                range_label: "400kΩ",
            },
            RangeEntry {
                unit_override: "MΩ",
                range_label: "4MΩ",
            },
            RangeEntry {
                unit_override: "MΩ",
                range_label: "40MΩ",
            },
        ],
        // Capacitance
        0x09 => &[
            RangeEntry {
                unit_override: "nF",
                range_label: "40nF",
            },
            RangeEntry {
                unit_override: "nF",
                range_label: "400nF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "4µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "40µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "400µF",
            },
            RangeEntry {
                unit_override: "µF",
                range_label: "4000µF",
            },
            RangeEntry {
                unit_override: "mF",
                range_label: "40mF",
            },
        ],
        // DC/AC µA
        0x0C | 0x0D => &[
            RangeEntry {
                unit_override: "",
                range_label: "400µA",
            },
            RangeEntry {
                unit_override: "",
                range_label: "4000µA",
            },
        ],
        // DC/AC mA
        0x0E | 0x0F => &[
            RangeEntry {
                unit_override: "",
                range_label: "40mA",
            },
            RangeEntry {
                unit_override: "",
                range_label: "400mA",
            },
        ],
        // DC/AC A
        0x10 | 0x11 => &[RangeEntry {
            unit_override: "",
            range_label: "10A",
        }],
        // Single-range functions (duty, diode, continuity, temp, LPF)
        _ => return None,
    };

    let idx = range_idx as usize;
    table.get(idx).map(|e| {
        let unit = if e.unit_override.is_empty() {
            // Use the function's base unit
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

use super::vc8x0_common::COMMANDS as VC880_COMMANDS;

/// Protocol implementation for the Voltcraft VC-880 and VC650BT.
pub struct Vc880Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
}

impl Default for Vc880Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Vc880Protocol {
    pub(crate) fn new() -> Self {
        Self::for_model("Voltcraft VC-880")
    }

    /// Build the protocol for one specific model of the family.
    ///
    /// The VC650BT speaks the same protocol, but it has to report its own name
    /// — otherwise a user who selected the VC650BT sees "Voltcraft VC-880" in
    /// the header and can't tell whether the right device was picked.
    pub(crate) fn for_model(model_name: &'static str) -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "VC880",
                model_name,
                stability: Stability::Experimental,
                supported_commands: VC880_COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(13),
            },
        }
    }
}

impl Protocol for Vc880Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // VC-880 streams automatically after the user presses the PC button.
        // No trigger command needed — just start reading.
        debug!("vc880: init (no trigger needed, meter streams after PC button press)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_be16,
            // Accept only live data frames (type byte = 0x01 at payload[0])
            |p| !p.is_empty() && p[0] == MSG_TYPE_LIVE_DATA,
            FrameErrorRecovery::SkipAndRetry,
            "vc880",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        use super::vc8x0_common;
        let cmd_byte = vc8x0_common::command_byte(command)?;
        let frame = vc8x0_common::build_command(cmd_byte);
        debug!("vc880: sending command {command} ({cmd_byte:#04x})");
        transport.write(&frame)?;
        Ok(())
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        super::vc8x0_common::read_device_name(&mut self.rx_buf, transport, "vc880")
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        super::vc8x0_common::capture_steps()
    }
}

/// Parse a VC880 live data payload.
///
/// The payload from `extract_frame_abcd_be16` is everything between
/// the length byte and the checksum:
///   payload[0]  = type byte (0x01, already filtered by accept_fn)
///   payload[1]  = function code (0x00-0x12)
///   payload[2]  = range byte (0x30-based ASCII)
///   payload[3..10]  = main display value (7 ASCII bytes)
///   payload[10..17] = sub display 1 (7 ASCII bytes)
///   payload[17..24] = sub display 2 (7 ASCII bytes)
///   payload[24..27] = bar graph / sub display 3 (3 bytes)
///   payload[27..34] = status flag bytes (7 bytes)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len("vc880", payload, LIVE_DATA_PAYLOAD_LEN)?;

    let function_code = payload[1];
    let range_raw = payload[2];
    let main_display = &payload[3..10];
    let status_bytes = &payload[27..34];

    // Look up function code
    let (mode, base_unit): (Cow<'static, str>, &'static str) = if let Some((_, name, unit)) =
        FUNCTION_TABLE.iter().find(|(c, _, _)| *c == function_code)
    {
        (Cow::Borrowed(name), unit)
    } else {
        debug!("vc880: unknown function code {function_code:#04x}");
        (unknown_mode(function_code), "")
    };

    // Decode range byte (0x30-based ASCII)
    let range_idx = range_raw.wrapping_sub(0x30);
    let (unit, range_label) = if let Some((u, r)) = lookup_range(function_code, range_idx) {
        (u, r)
    } else {
        // Single-range function or unknown range — use base unit
        (base_unit, "")
    };

    // Parse main display value (7 ASCII bytes)
    let display_str = String::from_utf8_lossy(main_display).to_string();
    let display_trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Extract status flags
    // Status byte 2 (payload[29]): bit2 = OL1 (primary overload)
    let ol1 = status_bytes[2] & 0x04 != 0;

    // Status byte 1 (payload[28]): bit0=Rel, bit1=Avg, bit2=Min, bit3=Max
    let rel = status_bytes[1] & 0x01 != 0;
    let min_flag = status_bytes[1] & 0x04 != 0;
    let max_flag = status_bytes[1] & 0x08 != 0;

    // Status byte 2 (payload[29]): bit0=Hold, bit1=Manual
    let hold = status_bytes[2] & 0x01 != 0;
    let auto_range = status_bytes[2] & 0x02 == 0; // Manual bit: 0=auto, 1=manual

    // Status byte 3 (payload[30]): bit1=Warning, bit3=LowBatt
    let hv_warning = status_bytes[3] & 0x02 != 0;
    let low_battery = status_bytes[3] & 0x08 != 0;

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
            Ok(v) => {
                // Sign is in the ASCII string itself (leading '-')
                MeasuredValue::Normal(v)
            }
            Err(_) => {
                if display_trimmed.is_empty() {
                    warn!("vc880: empty display value");
                } else {
                    warn!("vc880: could not parse display value: {display_str:?}");
                }
                MeasuredValue::Overload
            }
        }
    };

    Ok(Measurement {
        mode,
        mode_raw: function_code as u16,
        range_raw,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        display_raw: Some(display_str),
        flags,
        ..Measurement::from_payload(payload)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_support::snapshot;

    /// Build a minimal 34-byte VC880 live data payload for testing.
    fn make_payload(function: u8, range: u8, main_display: &[u8; 7], status: [u8; 7]) -> Vec<u8> {
        let mut p = vec![MSG_TYPE_LIVE_DATA, function, range];
        p.extend_from_slice(main_display); // main value (7 bytes)
        p.extend_from_slice(b"       "); // sub1 (7 bytes)
        p.extend_from_slice(b"       "); // sub2 (7 bytes)
        p.extend_from_slice(b"   "); // bar (3 bytes)
        p.extend_from_slice(&status); // status (7 bytes)
        assert_eq!(p.len(), LIVE_DATA_PAYLOAD_LEN);
        p
    }

    fn zero_status() -> [u8; 7] {
        [0u8; 7]
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x00, 0x31, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "40V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        let payload = make_payload(0x05, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "4V");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x06, 0x32, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "40kΩ");
    }

    #[test]
    fn parse_capacitance_nf() {
        let payload = make_payload(0x09, 0x30, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Capacitance");
        assert_eq!(m.unit, "nF");
        assert_eq!(m.range_label, "40nF");
    }

    #[test]
    fn parse_frequency_khz() {
        let payload = make_payload(0x03, 0x32, b" 1.2345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Frequency");
        assert_eq!(m.unit, "kHz");
        assert_eq!(m.range_label, "4kHz");
    }

    #[test]
    fn parse_temperature() {
        let payload = make_payload(0x0A, 0x30, b"  23.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "°C");
        assert_eq!(m.unit, "°C");
    }

    #[test]
    fn parse_overload_flag() {
        // OL1 = status byte 2, bit 2
        let mut status = zero_status();
        status[2] = 0x04; // OL1
        let payload = make_payload(0x06, 0x30, b"     OL", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_overload_display_string() {
        let payload = make_payload(0x06, 0x30, b"     OL", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_negative_value() {
        let payload = make_payload(0x00, 0x31, b" -12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-6));
    }

    #[test]
    fn parse_hold_flag() {
        let mut status = zero_status();
        status[2] = 0x01; // Hold bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        let mut status = zero_status();
        status[1] = 0x01; // Rel bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        let mut status = zero_status();
        status[1] = 0x08; // Max bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);

        let mut status = zero_status();
        status[1] = 0x04; // Min bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.max);
        assert!(m.flags.min);
    }

    #[test]
    fn parse_auto_range() {
        // Manual bit clear = auto range
        let payload = make_payload(0x00, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        // Manual bit set = manual range
        let mut status = zero_status();
        status[2] = 0x02; // Manual bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_low_battery() {
        let mut status = zero_status();
        status[3] = 0x08; // LowBatt bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.low_battery);
    }

    #[test]
    fn parse_hv_warning() {
        let mut status = zero_status();
        status[3] = 0x02; // Warning bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hv_warning);
    }

    #[test]
    fn parse_unknown_function() {
        let payload = make_payload(0x20, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(m.mode.starts_with("Unknown"));
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
        let payload = vec![0x01, 0x00, 0x30]; // way too short
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn display_raw_preserved() {
        let payload = make_payload(0x00, 0x30, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some(" 12.345"));
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x03, 0x32, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x03);
    }

    #[test]
    fn range_unit_override_works() {
        // Capacitance with nF range
        let payload = make_payload(0x09, 0x30, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "nF"); // overridden from base "F"

        // Capacitance with µF range
        let payload = make_payload(0x09, 0x32, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "µF");

        // Capacitance with mF range
        let payload = make_payload(0x09, 0x36, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "mF");
    }

    #[test]
    fn all_voltage_functions_share_ranges() {
        for func in [0x00, 0x01, 0x05, 0x12] {
            let payload = make_payload(func, 0x32, b"  123.4", zero_status());
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.range_label, "400V", "function {func:#04x}");
        }
    }

    #[test]
    fn send_command_builds_correct_frame() {
        let frame = super::super::vc8x0_common::build_command(0x47); // autorange
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xCD);
        assert_eq!(frame[2], 0x03);
        assert_eq!(frame[3], 0x47);
        // Checksum: 0xAB + 0xCD + 0x03 + 0x47 = 0x01C2
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    /// Every status byte 0xFF: the OL1 bit forces Overload, the manual-range
    /// bit clears AUTO, and hold/rel/min/max/HV/low-battery all light.
    #[test]
    fn snapshot_every_status_bit_set() {
        let payload = make_payload(0x00, b'1', b"-1.2345", [0xFF; 7]);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("-1.2345")
flags=hold,rel,min,max,low_battery,hv_warning
aux=0
raw_payload=34"#
        );
    }

    /// The same frame with every status byte clear: AUTO on (the manual bit
    /// is inverted), nothing else set, and the digits parsed as a value.
    #[test]
    fn snapshot_zero_status() {
        let payload = make_payload(0x00, b'1', b"-1.2345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Normal(-1.2345)
unit=V
range_label=40V
display_raw=Some("-1.2345")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Overload spelled out in the digits rather than flagged by OL1.
    #[test]
    fn snapshot_overload_display_string() {
        let payload = make_payload(0x06, b'0', b"     OL", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x06
range_raw=0x30
value=Overload
unit=Ω
range_label=400Ω
display_raw=Some("     OL")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// The meter's "---" blank-reading form also reads as overload.
    #[test]
    fn snapshot_dashes_display() {
        let payload = make_payload(0x06, b'0', b"    ---", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x06
range_raw=0x30
value=Overload
unit=Ω
range_label=400Ω
display_raw=Some("    ---")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// An all-spaces display: nothing to parse, so the reading falls back to
    /// Overload.
    #[test]
    fn snapshot_blank_display() {
        let payload = make_payload(0x00, b'1', b"       ", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("       ")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Digits that are not a number: same Overload fallback as a blank one.
    #[test]
    fn snapshot_unparsable_display() {
        let payload = make_payload(0x00, b'1', b"1.2.3.4", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("1.2.3.4")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// A function code outside FUNCTION_TABLE: mode falls back to
    /// `Unknown(..)` and the unit is empty.
    #[test]
    fn snapshot_unknown_function_code() {
        let payload = make_payload(0x7F, b'0', b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x7f)
mode_raw=0x7f
range_raw=0x30
value=Normal(1.234)
unit=
range_label=
display_raw=Some("  1.234")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Range byte b'9' indexes past the 4-entry voltage table: the range
    /// label goes empty and the unit falls back to the function's base unit.
    #[test]
    fn snapshot_range_index_past_the_table() {
        let payload = make_payload(0x00, b'9', b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x39
value=Normal(1.234)
unit=V
range_label=
display_raw=Some("  1.234")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// DC mV has a one-entry range table.
    #[test]
    fn snapshot_single_range_function() {
        let payload = make_payload(0x02, b'0', b" 123.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC mV
mode_raw=0x02
range_raw=0x30
value=Normal(123.45)
unit=mV
range_label=400mV
display_raw=Some(" 123.45")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// The whole range table as one string: every function code crossed with
    /// every range index, rendered `unit|range_label` (`-` where the lookup
    /// returns None). Pins both the labels and the unit-override fallback.
    #[test]
    fn range_table_snapshot() {
        let table: Vec<String> = (0x00u8..=0x12)
            .map(|function| {
                let cells: Vec<String> = (0u8..=8)
                    .map(|range_idx| match lookup_range(function, range_idx) {
                        Some((unit, label)) => format!("{unit}|{label}"),
                        None => "-".to_string(),
                    })
                    .collect();
                format!("{function:#04x}: {}", cells.join(" "))
            })
            .collect();
        assert_eq!(
            table.join("\n"),
            r#"0x00: V|4V V|40V V|400V V|1000V - - - - -
0x01: V|4V V|40V V|400V V|1000V - - - - -
0x02: mV|400mV - - - - - - - -
0x03: Hz|40Hz Hz|400Hz kHz|4kHz kHz|40kHz kHz|400kHz MHz|4MHz MHz|40MHz MHz|400MHz -
0x04: - - - - - - - - -
0x05: V|4V V|40V V|400V V|1000V - - - - -
0x06: Ω|400Ω kΩ|4kΩ kΩ|40kΩ kΩ|400kΩ MΩ|4MΩ MΩ|40MΩ - - -
0x07: - - - - - - - - -
0x08: - - - - - - - - -
0x09: nF|40nF nF|400nF µF|4µF µF|40µF µF|400µF µF|4000µF mF|40mF - -
0x0a: - - - - - - - - -
0x0b: - - - - - - - - -
0x0c: µA|400µA µA|4000µA - - - - - - -
0x0d: µA|400µA µA|4000µA - - - - - - -
0x0e: mA|40mA mA|400mA - - - - - - -
0x0f: mA|40mA mA|400mA - - - - - - -
0x10: A|10A - - - - - - - -
0x11: A|10A - - - - - - - -
0x12: V|4V V|40V V|400V V|1000V - - - - -"#
        );
    }
}
