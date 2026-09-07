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
use crate::protocol::cycle::RANGE_BUTTON_NAME;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{CaptureStep, cycle, unknown_mode};
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

/// The button the vendor DLL calls `Select` (spec §5, "SELECT button (cycle
/// sub-function)"). Both meters' front panel labels it SHIFT/SETUP, and it is
/// what steps a dial position through its sub-functions.
pub(crate) const CMD_SELECT: u8 = 0x4C;

/// What the front panel calls [`CMD_SELECT`], for messages the user reads.
pub(crate) const SELECT_BUTTON_NAME: &str = "SHIFT/SETUP";

/// The RANGE button: one press steps the manual range ladder (spec §5,
/// "RANGE button"). Whether repeated presses really step it one rung at a
/// time is unverified on both meters — see the VC-880 and VC-890 sections of
/// docs/verification-backlog.md — which is why the range driver reads the
/// range back after every press rather than counting them.
pub(crate) const CMD_RANGE_MANUAL: u8 = 0x46;

/// Return to auto-ranging: its own command, as it is its own button.
pub(crate) const CMD_RANGE_AUTO: u8 = 0x47;

/// HOLD, REL and the MAX/MIN/AVG pair (spec §5). The MAX/MIN/AVG button
/// steps the states and only 0x43 leaves them, exactly as the UT61+ family's
/// 0x41/0x42 pair does.
pub(crate) const CMD_HOLD: u8 = 0x4A;
pub(crate) const CMD_REL: u8 = 0x48;
pub(crate) const CMD_MAX_MIN_AVG: u8 = 0x49;
pub(crate) const CMD_EXIT_MAX_MIN_AVG: u8 = 0x43;

/// What the front panels call the buttons the flag-backed settings press.
pub(crate) const HOLD_BUTTON_NAME: &str = "HOLD";
pub(crate) const REL_BUTTON_NAME: &str = "REL";
pub(crate) const MAX_MIN_AVG_BUTTON_NAME: &str = "MAX/MIN/AVG";

/// The command byte one press of `button` sends, or `None` for a button
/// neither Voltcraft meter has.
pub(crate) fn press_command(button: cycle::CycleButton) -> Option<u8> {
    Some(match button {
        cycle::CycleButton::Select => CMD_SELECT,
        cycle::CycleButton::Range => CMD_RANGE_MANUAL,
        cycle::CycleButton::Hold => CMD_HOLD,
        cycle::CycleButton::Rel => CMD_REL,
        cycle::CycleButton::MinMax => CMD_MAX_MIN_AVG,
        // No Hz/% button (the SHIFT/SETUP ring reaches Hz), and no Peak
        // function at all: the vendor command table lists neither.
        cycle::CycleButton::Hz | cycle::CycleButton::Peak => return None,
    })
}

/// What the front panel calls `button`, for the messages the user reads.
pub(crate) fn button_name(button: cycle::CycleButton) -> &'static str {
    match button {
        cycle::CycleButton::Select => SELECT_BUTTON_NAME,
        cycle::CycleButton::Range => RANGE_BUTTON_NAME,
        cycle::CycleButton::Hold => HOLD_BUTTON_NAME,
        cycle::CycleButton::Rel => REL_BUTTON_NAME,
        cycle::CycleButton::MinMax => MAX_MIN_AVG_BUTTON_NAME,
        cycle::CycleButton::Hz => "Hz/%",
        cycle::CycleButton::Peak => "PEAK",
    }
}

/// The states each flag-backed setting offers; identical on both meters.
///
/// HOLD and REL toggle. MAX/MIN/AVG is a ring of three plus off, its press
/// order unverified — which is why the driver reads the flags back after
/// every press instead of counting them. Neither meter has Peak.
pub(crate) fn flag_states(setting: cycle::FlagSetting) -> &'static [u16] {
    match setting {
        cycle::FlagSetting::Hold | cycle::FlagSetting::Rel => &[0, 1],
        cycle::FlagSetting::MinMax => &[0, 1, 2, 3],
        cycle::FlagSetting::Peak => &[],
    }
}

/// Map a command name to its byte value.
///
/// Command bytes are identical for VC-880 and VC-890.
pub(crate) fn command_byte(command: &str) -> Result<u8> {
    match command {
        "hold" => Ok(CMD_HOLD),
        "rel" => Ok(CMD_REL),
        "max_min_avg" => Ok(CMD_MAX_MIN_AVG),
        "exit_max_min_avg" => Ok(CMD_EXIT_MAX_MIN_AVG),
        "range_auto" => Ok(CMD_RANGE_AUTO),
        "range_manual" => Ok(CMD_RANGE_MANUAL),
        "light" => Ok(0x4B),
        "select" => Ok(CMD_SELECT),
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
    use crate::protocol::{Expect, Need, ValueExpect};

    vec![
        CaptureStep::basic("dcv", "Set meter to DC V")
            .gate()
            .expect(Expect::mode("DC V").value(ValueExpect::Finite)),
        CaptureStep::basic("dcv_short", "DC V mode: touch the two probe tips together.")
            .gate()
            .needs(&[Need::ShortedLeads])
            .expect(Expect::mode("DC V").value(ValueExpect::Finite)),
        CaptureStep::basic(
            "dcv_negative",
            "DC V mode: leads reversed on a battery or any DC source (skip if none).",
        )
        .gate()
        .needs(&[Need::DcSource])
        .expect(Expect::mode("DC V").value(ValueExpect::Negative)),
        CaptureStep::basic("acv", "Set meter to AC V").expect(Expect::mode("AC V")),
        CaptureStep::basic("acdcv", "Set meter to AC+DC V").expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set meter to DC mV").expect(Expect::mode("DC mV")),
        CaptureStep::basic("dcua", "Set meter to DC µA").expect(Expect::mode("DC µA")),
        CaptureStep::basic("acua", "Set meter to AC µA").expect(Expect::mode("AC µA")),
        CaptureStep::basic("dcma", "Set meter to DC mA").expect(Expect::mode("DC mA")),
        CaptureStep::basic("acma", "Set meter to AC mA").expect(Expect::mode("AC mA")),
        CaptureStep::basic("dca", "Set meter to DC A").expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "Set meter to AC A").expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "ohm",
            "Set meter to Resistance (Ω). Leave leads open (should show OL).",
        )
        .gate()
        .expect(Expect::mode("Ω").value(ValueExpect::Overload)),
        // Open and shorted leads repeat one digit value, so a digit-order
        // or digit-value bug hides; a body reading spreads the digits out.
        CaptureStep::basic(
            "ohm_body",
            "Resistance mode: hold one probe tip between the fingers of each \
             hand (body resistance, hundreds of kΩ).",
        )
        .gate()
        .expect(Expect::mode("Ω").value(ValueExpect::Finite)),
        CaptureStep::basic(
            "ohm_short",
            "Resistance mode: touch the two probe tips together.",
        )
        .gate()
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Ω").value(ValueExpect::Finite)),
        CaptureStep::basic("cont", "Set meter to Continuity").expect(Expect::mode("Continuity")),
        CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
        CaptureStep::basic("cap", "Set meter to Capacitance").expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Frequency")),
        CaptureStep::basic("duty", "Set meter to Duty Cycle (%)").expect(Expect::mode("Duty %")),
        CaptureStep::basic("tempc", "Set meter to Temperature °C")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C")),
        CaptureStep::basic("tempf", "Set meter to Temperature °F")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
        CaptureStep::basic("lpf", "Set meter to ACV Low-Pass Filter")
            .expect(Expect::mode("ACV LPF")),
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

/// The manual range ladder of a function's range table, for
/// `Setting::Range`. Rung `n` of the choice list is entry `n - 1`, which is
/// what the meter reports as `range_raw - 0x30`.
pub(crate) fn range_ladder(table: &'static [RangeEntry]) -> Vec<Cow<'static, str>> {
    cycle::usable_ladder(table.iter().map(|e| Cow::Borrowed(e.range_label)).collect())
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
        // (vc880 spec byte 31, tables at lines 117 and 272; vc890 spec byte 57,
        // line 95 — both families share this byte layout.)
        rel: status[1] & 0x01 != 0,
        avg: status[1] & 0x02 != 0,
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
        // The range driver presses the same byte "range_manual" sends.
        assert_eq!(command_byte("range_manual").unwrap(), CMD_RANGE_MANUAL);
        assert_eq!(command_byte("range_auto").unwrap(), CMD_RANGE_AUTO);
        assert_eq!(command_byte("hold").unwrap(), 0x4A);
        assert_eq!(command_byte("rel").unwrap(), 0x48);
        // The flag-setting driver presses the same bytes.
        assert_eq!(press_command(cycle::CycleButton::Hold), Some(0x4A));
        assert_eq!(press_command(cycle::CycleButton::Rel), Some(0x48));
        assert_eq!(press_command(cycle::CycleButton::MinMax), Some(0x49));
        assert_eq!(command_byte("max_min_avg").unwrap(), CMD_MAX_MIN_AVG);
        assert_eq!(
            command_byte("exit_max_min_avg").unwrap(),
            CMD_EXIT_MAX_MIN_AVG
        );
        // Neither meter has these two.
        assert_eq!(press_command(cycle::CycleButton::Hz), None);
        assert_eq!(press_command(cycle::CycleButton::Peak), None);
        assert_eq!(command_byte("light").unwrap(), 0x4B);
        // The mode driver presses the same byte the "select" command sends.
        assert_eq!(command_byte("select").unwrap(), CMD_SELECT);
    }

    #[test]
    fn command_byte_unknown() {
        assert!(command_byte("nonexistent").is_err());
    }
}
