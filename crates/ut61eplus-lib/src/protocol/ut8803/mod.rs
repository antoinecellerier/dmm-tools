//! UT8803/UT8803E bench multimeter protocol.
//!
//! Streaming protocol: host sends 0x5A trigger byte after CP2110 init,
//! meter streams 21-byte measurement frames continuously at ~2-3 Hz.
//!
//! Frame format: AB CD [byte2] 02 [mode] [range] [byte6] [display x5]
//!   [flags0 x2] [flags1 x2] [flags2 x2] [flags3] [chk_hi] [chk_lo]
//!
//! Based on reverse engineering of uci.dll (Ghidra decompilation).
//! See docs/research/ut8803/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;
use std::time::Instant;

/// UT8803 position coding → (mode_name, acdc, unit_type, unit_mag).
/// From the programming manual section 4.2.
const POSITION_TABLE: &[(u8, &str)] = &[
    (0x00, "AC V"),
    (0x01, "DC V"),
    (0x02, "AC µA"),
    (0x03, "AC mA"),
    (0x04, "AC A"),
    (0x05, "DC µA"),
    (0x06, "DC mA"),
    (0x07, "DC A"),
    (0x08, "Ω"),
    (0x09, "Continuity"),
    (0x0A, "Diode"),
    (0x0B, "Inductance"),
    (0x0C, "Inductance Q"),
    (0x0D, "Inductance R"),
    (0x0E, "Capacitance"),
    (0x0F, "Capacitance D"),
    (0x10, "Capacitance R"),
    (0x11, "hFE"),
    (0x12, "SCR"),
    (0x13, "°C"),
    (0x14, "°F"),
    (0x15, "Hz"),
    (0x16, "Duty %"),
];

const UT8803_COMMANDS: &[&str] = &[];

/// Protocol implementation for the UT8803/UT8803E bench multimeter.
pub struct Ut8803Protocol {
    rx_buf: Vec<u8>,
    triggered: bool,
    profile: DeviceProfile,
}

impl Default for Ut8803Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut8803Protocol {
    pub fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            triggered: false,
            profile: DeviceProfile {
                family_name: "UT8803",
                model_name: "UNI-T UT8803",
                stability: Stability::Experimental,
                supported_commands: UT8803_COMMANDS,
            },
        }
    }
}

impl Protocol for Ut8803Protocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // Send 0x5A trigger byte to start streaming
        debug!("ut8803: sending 0x5A trigger byte");
        transport.write(&[0x5A])?;
        self.triggered = true;
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_ut8803,
            |_| true,
            FrameErrorRecovery::SkipAndRetry,
            "ut8803",
            &framing::HEADER,
        )?;
        parse_measurement(&payload)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        // UT8803 doesn't support remote commands beyond the initial trigger
        Err(Error::UnsupportedCommand(command.to_string()))
    }

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        // UT8803 doesn't support name query
        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::CaptureStep;
        // All 23 UT8803 position codes (modes 0x00-0x16)
        vec![
            CaptureStep {
                id: "dcv",
                instruction: "Set meter to DC V (DCV)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "acv",
                instruction: "Set meter to AC V (ACV)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "dcua",
                instruction: "Set meter to DC uA",
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
                id: "acua",
                instruction: "Set meter to AC uA",
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
                instruction: "Set meter to Resistance (OHM)",
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
                id: "ind",
                instruction: "Set meter to Inductance (L)",
                command: None,
                samples: 5,
            },
            CaptureStep {
                id: "cap",
                instruction: "Set meter to Capacitance (C)",
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
        ]
    }
}

/// Parse a UT8803 measurement payload (17 bytes = frame bytes 2..19).
///
/// Layout (from Ghidra decompilation of uci.dll):
/// - byte 0:    (frame byte 2, part of mode word, not directly consumed)
/// - byte 1:    type byte (0x02 = measurement, already verified by framing)
/// - byte 2:    mode byte (raw, 0x00-0x16)
/// - byte 3:    range byte (has 0x30 prefix, mask with -0x30, max 6)
/// - byte 4:    (included in checksum, not parsed — reserved/padding)
/// - bytes 5-9: display (5 raw bytes)
/// - bytes 10-11: flags0 (2 bytes)
/// - bytes 12-13: flags1 (2 bytes)
/// - bytes 14-15: flags2 (2 bytes)
/// - byte 16:   flags3 (1 byte)
pub fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    if payload.len() < 17 {
        return Err(Error::invalid_response(
            format!(
                "ut8803 payload too short: {} bytes, expected 17",
                payload.len()
            ),
            payload,
        ));
    }

    let mode_byte = payload[2];
    let range_raw = payload[3];
    let _range_byte = if range_raw >= 0x30 {
        range_raw - 0x30
    } else {
        range_raw
    };
    let display_bytes = &payload[5..10];

    // Parse mode
    let mode: Cow<'static, str> = if (mode_byte as usize) < POSITION_TABLE.len() {
        Cow::Borrowed(POSITION_TABLE[mode_byte as usize].1)
    } else {
        warn!("ut8803: unknown mode byte {:#04x}", mode_byte);
        Cow::Owned(format!("Unknown({:#04x})", mode_byte))
    };

    // Parse display value: 5 raw bytes → string → f64
    let display_str = String::from_utf8_lossy(display_bytes).to_string();
    let display_trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Flag extraction from raw bytes to semantic flags.
    //
    // IMPORTANT: These bit assignments are UNVERIFIED against real hardware.
    // The RE spec documents the *constructed* 32-bit status word layout
    // (D0-D31), but the raw-byte-to-status-word construction in uci.dll
    // involves complex bit-shifting (see RE spec section 2.3). The mapping
    // below is our best guess based on the decompilation, but the actual
    // bit positions in the raw flag bytes may differ from the status word.
    // These need real device verification.
    let flags1_lo = payload[12];
    let flags1_hi = payload[13];
    let flags3 = payload[16];

    let hold = flags3 & 0x02 != 0; // [UNVERIFIED] flag_15 → bit1 of flags3
    let rel = flags1_hi & 0x40 != 0; // [UNVERIFIED] bit 6 of flags1 high byte
    let min_flag = flags1_hi & 0x20 != 0; // [UNVERIFIED] bit 5
    let max_flag = flags1_hi & 0x10 != 0; // [UNVERIFIED] bit 4

    let auto_range = flags1_lo & 0x40 != 0; // [UNVERIFIED] D6 of status word

    // Overload
    let overload = flags1_lo & 0x80 != 0; // D7 of status word

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery: false, // D15, would need to extract from status word
        hv_warning: false,
        dc: false,
        peak_max: false,
        peak_min: false,
    };

    // Determine unit from functional coding (bits D8-D14 of status word)
    let _func_code = flags1_lo & 0x0F; // D0-D3
    let unit_type = (flags1_lo >> 4) & 0x03; // simplified — actual is D8-D11
    let _ = unit_type; // unit determination from mode is more reliable

    // Build unit string from mode name heuristics
    let unit: &'static str = match &*mode {
        "DC V" | "AC V" => "V",
        "DC µA" | "AC µA" => "µA",
        "DC mA" | "AC mA" => "mA",
        "DC A" | "AC A" => "A",
        "Ω" => "Ω",
        "Hz" => "Hz",
        "Duty %" => "%",
        "°C" => "°C",
        "°F" => "°F",
        "Capacitance" | "Capacitance D" | "Capacitance R" => "F",
        "Inductance" | "Inductance Q" | "Inductance R" => "H",
        "hFE" => "",
        "Continuity" => "Ω",
        "Diode" => "V",
        "SCR" => "V",
        _ => "",
    };

    let value = if overload {
        MeasuredValue::Overload
    } else if let Ok(v) = display_trimmed.parse::<f64>() {
        MeasuredValue::Normal(v)
    } else if display_trimmed.contains("OL") {
        MeasuredValue::Overload
    } else {
        warn!(
            "ut8803: could not parse display value: {:?} (raw: {:02X?})",
            display_trimmed, display_bytes
        );
        MeasuredValue::Overload
    };

    Ok(Measurement {
        timestamp: Instant::now(),
        mode,
        mode_raw: mode_byte as u16,
        range_raw,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(""), // UT8803 range label would need the full range table
        progress: None,
        display_raw: Some(display_str),
        flags,
        raw_payload: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(
        mode: u8,
        range: u8,
        display: &[u8; 5],
        flags1_lo: u8,
        flags1_hi: u8,
        flags3: u8,
    ) -> Vec<u8> {
        vec![
            0x00,         // byte 0 (frame byte 2)
            0x02,         // byte 1 (type = measurement)
            mode,         // byte 2 (mode)
            range | 0x30, // byte 3 (range with 0x30 prefix)
            0x00,         // byte 4 (reserved)
            display[0],   // bytes 5-9 (display)
            display[1],
            display[2],
            display[3],
            display[4],
            0x00,
            0x00, // bytes 10-11 (flags0)
            flags1_lo,
            flags1_hi, // bytes 12-13 (flags1)
            0x00,
            0x00,   // bytes 14-15 (flags2)
            flags3, // byte 16 (flags3)
        ]
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x01, 0x00, b"12.34", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.34).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        let payload = make_payload(0x00, 0x00, b" 5.67", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
    }

    #[test]
    fn parse_ohm() {
        let payload = make_payload(0x08, 0x01, b"123.4", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "Ω");
    }

    #[test]
    fn parse_unknown_mode_permissive() {
        let payload = make_payload(0x20, 0x00, b" 1.23", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Unknown(0x20)");
    }

    #[test]
    fn parse_overload_flag() {
        // D7 = overload bit in flags1_lo
        let payload = make_payload(0x08, 0x00, b"    0", 0x80, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        // flags3 bit1 = HOLD
        let payload = make_payload(0x01, 0x00, b" 1.23", 0x00, 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_auto_range() {
        // D6 of flags1_lo = auto range
        let payload = make_payload(0x01, 0x00, b" 1.23", 0x40, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x00; 10];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn parse_all_valid_modes() {
        for (code, _name) in POSITION_TABLE {
            let payload = make_payload(*code, 0x00, b" 1.00", 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "mode {:#04x} should be known",
                code
            );
        }
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x15, 0x00, b" 50.0", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x15);
        assert_eq!(m.mode, "Hz");
    }

    #[test]
    fn display_raw_preserved() {
        let payload = make_payload(0x01, 0x00, b"12.34", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.34"));
    }
}
