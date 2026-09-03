//! Shared helpers for the Voltcraft VC-880 and VC-890 protocol families.
//!
//! Both use AB CD framing with BE16 checksums and share identical command
//! byte assignments and DeviceID retrieval logic.
//!
//! The live-data payloads differ in length, field layout and function-code
//! assignment, but the decoding *steps* are the same, so the skeleton lives
//! here too: [`RangeEntry`]/[`re`] and [`resolve_range`] for the range
//! tables, and [`resolve_function`], [`main_display`], [`common_flags`] and
//! [`parse_value`] for the parts of `parse_measurement` that only differ by
//! family label and byte offset. Each family keeps its own frame layout
//! constants, function and range tables, extra status flags, and the
//! `Measurement` it builds.

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::MeasuredValue;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{CaptureStep, unknown_mode};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;

/// Build a command frame: `[0xAB, 0xCD, 0x03, cmd, chk_hi, chk_lo]`.
pub(crate) fn build_command(cmd: u8) -> Vec<u8> {
    let mut frame = vec![0xAB, 0xCD, 0x03, cmd];
    let sum: u16 = frame.iter().map(|&b| b as u16).sum();
    frame.push((sum >> 8) as u8);
    frame.push((sum & 0xFF) as u8);
    frame
}

/// Map a command name to its byte value.
///
/// Command bytes are identical for VC-880 and VC-890.
pub(crate) fn command_byte(command: &str) -> Result<u8> {
    match command {
        "hold" => Ok(0x4A),
        "rel" => Ok(0x48),
        "max_min_avg" => Ok(0x49),
        "exit_max_min_avg" => Ok(0x43),
        "range_auto" => Ok(0x47),
        "range_manual" => Ok(0x46),
        "light" => Ok(0x4B),
        "select" => Ok(0x4C),
        _ => Err(Error::UnsupportedCommand(command.to_string())),
    }
}

/// Send the GetDeviceID command (0x00) and read the 20-byte ASCII name.
pub(crate) fn read_device_name(
    rx_buf: &mut Vec<u8>,
    transport: &dyn Transport,
    label: &str,
) -> Result<Option<String>> {
    let frame = build_command(0x00);
    debug!("{label}: sending GetDeviceID command");
    transport.write(&frame)?;

    match framing::read_frame(
        rx_buf,
        transport,
        framing::extract_frame_abcd_be16,
        |p| !p.is_empty() && p[0] == 0x00, // DeviceID type
        FrameErrorRecovery::SkipAndRetry,
        &format!("{label}-id"),
        &framing::HEADER,
    ) {
        Ok(payload) if payload.len() >= 21 => {
            let name = String::from_utf8_lossy(&payload[1..21]).trim().to_string();
            debug!("{label}: device name: {name}");
            if name.is_empty() {
                Ok(None)
            } else {
                Ok(Some(name))
            }
        }
        Ok(_) => {
            debug!("{label}: DeviceID response too short");
            Ok(None)
        }
        Err(e) => {
            debug!("{label}: failed to read DeviceID: {e}");
            Ok(None)
        }
    }
}

/// Button commands, identical on both meters.
pub(crate) const COMMANDS: &[&str] = &[
    "hold",
    "rel",
    "max_min_avg",
    "exit_max_min_avg",
    "range_auto",
    "range_manual",
    "light",
    "select",
];

/// Capture steps shared by the VC-880 and VC-890.
///
/// Both meters expose the same dial positions, so the list lived as ~120
/// duplicated lines in each protocol. Device-specific steps are appended by
/// the caller.
pub(crate) fn capture_steps() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("dcv", "Set meter to DC V"),
        CaptureStep::basic("acv", "Set meter to AC V"),
        CaptureStep::basic("acdcv", "Set meter to AC+DC V"),
        CaptureStep::basic("dcmv", "Set meter to DC mV"),
        CaptureStep::basic("dcua", "Set meter to DC µA"),
        CaptureStep::basic("acua", "Set meter to AC µA"),
        CaptureStep::basic("dcma", "Set meter to DC mA"),
        CaptureStep::basic("acma", "Set meter to AC mA"),
        CaptureStep::basic("dca", "Set meter to DC A"),
        CaptureStep::basic("aca", "Set meter to AC A"),
        CaptureStep::basic("ohm", "Set meter to Resistance (Ω)"),
        CaptureStep::basic("cont", "Set meter to Continuity"),
        CaptureStep::basic("diode", "Set meter to Diode"),
        CaptureStep::basic("cap", "Set meter to Capacitance"),
        CaptureStep::basic("hz", "Set meter to Frequency (Hz)"),
        CaptureStep::basic("duty", "Set meter to Duty Cycle (%)"),
        CaptureStep::basic("tempc", "Set meter to Temperature °C"),
        CaptureStep::basic("tempf", "Set meter to Temperature °F"),
        CaptureStep::basic("lpf", "Set meter to ACV Low-Pass Filter"),
    ]
}

/// Range table entry: `unit_override` replaces the function's base unit
/// when non-empty. `range_label` is a human-readable string like "40kΩ".
pub(crate) struct RangeEntry {
    unit_override: &'static str,
    range_label: &'static str,
}

/// Shorthand for a [`RangeEntry`] literal, so a range table reads as one
/// line per range.
pub(crate) const fn re(unit_override: &'static str, range_label: &'static str) -> RangeEntry {
    RangeEntry {
        unit_override,
        range_label,
    }
}

/// Index a range table and resolve the entry's unit override against the
/// function's base unit from `function_table`.
///
/// Returns (unit, range_label), or None if the range index is out of bounds.
pub(crate) fn resolve_range(
    table: &[RangeEntry],
    range_idx: u8,
    function_table: &[(u8, &'static str, &'static str)],
    function: u8,
) -> Option<(&'static str, &'static str)> {
    table.get(range_idx as usize).map(|e| {
        let unit = if e.unit_override.is_empty() {
            // Use the function's base unit
            function_table
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

/// Look a function code up in a family's function table.
///
/// Returns (mode name, base unit); an unrecognised code becomes a generic
/// `unknown_mode` label with an empty base unit.
pub(crate) fn resolve_function(
    table: &[(u8, &'static str, &'static str)],
    code: u8,
    family: &str,
) -> (Cow<'static, str>, &'static str) {
    if let Some((_, name, unit)) = table.iter().find(|(c, _, _)| *c == code) {
        (Cow::Borrowed(*name), *unit)
    } else {
        debug!("{family}: unknown function code {code:#04x}");
        (unknown_mode(code), "")
    }
}

/// Decode the ASCII main display field.
///
/// Returns (raw string, same string with all whitespace removed).
pub(crate) fn main_display(bytes: &[u8]) -> (String, String) {
    let raw = String::from_utf8_lossy(bytes).to_string();
    let trimmed: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    (raw, trimmed)
}

/// Extract the status flags both families encode in the same bits of
/// status bytes 1-3, plus the OL1 (primary overload) bit.
///
/// Callers add their family-specific flags to the returned `StatusFlags`.
pub(crate) fn common_flags(status: &[u8]) -> (StatusFlags, bool) {
    // Status byte 2: bit2 = OL1 (primary overload)
    let ol1 = status[2] & 0x04 != 0;

    let flags = StatusFlags {
        // Status byte 2: bit0=Hold, bit1=Manual
        hold: status[2] & 0x01 != 0,
        // Status byte 1: bit0=Rel, bit1=Avg, bit2=Min, bit3=Max
        rel: status[1] & 0x01 != 0,
        min: status[1] & 0x04 != 0,
        max: status[1] & 0x08 != 0,
        auto_range: status[2] & 0x02 == 0, // Manual bit: 0=auto, 1=manual
        // Status byte 3: bit1=Warning
        hv_warning: status[3] & 0x02 != 0,
        dc: false,
        peak_max: false,
        peak_min: false,
        ..Default::default()
    };

    (flags, ol1)
}

/// Turn the decoded main display into a `MeasuredValue`.
///
/// `trimmed` is the whitespace-stripped display, `raw` the original (only
/// used for the diagnostic).
pub(crate) fn parse_value(family: &str, ol1: bool, trimmed: &str, raw: &str) -> MeasuredValue {
    if ol1 || trimmed.contains("OL") || trimmed.contains("---") {
        return MeasuredValue::Overload;
    }
    match trimmed.parse::<f64>() {
        // Sign is in the ASCII string itself (leading '-')
        Ok(v) => MeasuredValue::Normal(v),
        Err(_) => {
            if trimmed.is_empty() {
                warn!("{family}: empty display value");
            } else {
                warn!("{family}: could not parse display value: {raw:?}");
            }
            MeasuredValue::Overload
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_checksum() {
        let frame = build_command(0x4A);
        assert_eq!(frame.len(), 6);
        assert_eq!(&frame[..4], &[0xAB, 0xCD, 0x03, 0x4A]);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    #[test]
    fn command_byte_known() {
        assert_eq!(command_byte("hold").unwrap(), 0x4A);
        assert_eq!(command_byte("rel").unwrap(), 0x48);
        assert_eq!(command_byte("light").unwrap(), 0x4B);
    }

    #[test]
    fn command_byte_unknown() {
        assert!(command_byte("nonexistent").is_err());
    }
}
