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
use crate::protocol::{DeviceProfile, Protocol, Stability, check_len, unknown_mode};
use crate::transport::Transport;
use log::{debug, warn};
use std::borrow::Cow;

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

/// Display unit (magnitude prefix + base unit) for a (mode, range) pair.
///
/// The display value is range-relative — e.g. 5.999 on Ω range 1 means
/// 5.999 kΩ — so the unit must carry the magnitude prefix. Base units per
/// mode from `FUN_1001cff0` (uci_dll_decompiled.txt:23789-23829: 2=Ω also
/// for IndR/CapR ESR sub-modes, default 0xF=no unit for IndQ/CapD);
/// prefixes per (mode, range) from `FUN_1001cdc0` (lines 23644-23700:
/// 0=n, 1=µ, 2=m, 3=none, 4=k, 5=M).
fn display_unit(mode_byte: u8, range_idx: u8) -> &'static str {
    match mode_byte {
        // V: range 0 = mV, others V
        0x00 | 0x01 => {
            if range_idx == 0 {
                "mV"
            } else {
                "V"
            }
        }
        0x02 | 0x05 => "µA",
        0x03 | 0x06 => "mA",
        0x04 | 0x07 => "A",
        // Ω, Continuity, Inductance R (ESR), Capacitance R (ESR)
        0x08 | 0x09 | 0x0D | 0x10 => match range_idx {
            1..=3 => "kΩ",
            4..=5 => "MΩ",
            _ => "Ω",
        },
        0x0A | 0x12 => "V",
        // Inductance: ranges 0-3 = mH, above = H
        0x0B => {
            if range_idx < 4 {
                "mH"
            } else {
                "H"
            }
        }
        // Inductance Q / Capacitance D: dimensionless
        0x0C | 0x0F => "",
        0x0E => match range_idx {
            0..=2 => "nF",
            3..=5 => "µF",
            6 => "mF",
            _ => "F",
        },
        0x11 => "",
        0x13 => "°C",
        0x14 => "°F",
        0x15 => match range_idx {
            1..=3 => "kHz",
            4..=5 => "MHz",
            _ => "Hz",
        },
        0x16 => "%",
        _ => "",
    }
}

/// Whether a mode measures a DC quantity, per `FUN_1001ca90`
/// (uci_dll_decompiled.txt:23449-23466: AC = modes 0x00/0x02-0x04,
/// DC = 0x01/0x05-0x07).
fn mode_is_dc(mode_byte: u8) -> bool {
    matches!(mode_byte, 0x01 | 0x05..=0x07)
}

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
                max_aux_values: 0,
                verification_issue: Some(3),
            },
        }
    }
}

impl Protocol for Ut8803Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // No trigger byte: the vendor's CP2110 init path (uci.dll
        // FUN_1001d460, uci_dll_decompiled.txt:24031-24085) configures the
        // UART and reads — it never writes. The 0x5A trigger we previously
        // sent here belongs to the QinHeng/CH9325 init path for other
        // meters (FUN_1001d360, lines 24003-24005). Whether the UT8803
        // tolerates a stray 0x5A is untested on hardware.
        debug!("ut8803: init (no trigger byte; meter streams unprompted)");
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
            CaptureStep::basic("dcv", "Set meter to DC V (DCV)"),
            CaptureStep::basic("acv", "Set meter to AC V (ACV)"),
            CaptureStep::basic("dcua", "Set meter to DC uA"),
            CaptureStep::basic("dcma", "Set meter to DC mA"),
            CaptureStep::basic("dca", "Set meter to DC A"),
            CaptureStep::basic("acua", "Set meter to AC uA"),
            CaptureStep::basic("acma", "Set meter to AC mA"),
            CaptureStep::basic("aca", "Set meter to AC A"),
            CaptureStep::basic("ohm", "Set meter to Resistance (OHM)"),
            CaptureStep::basic("cont", "Set meter to Continuity"),
            CaptureStep::basic("diode", "Set meter to Diode"),
            CaptureStep::basic("ind", "Set meter to Inductance (L)"),
            CaptureStep::basic("cap", "Set meter to Capacitance (C)"),
            CaptureStep::basic("hfe", "Set meter to hFE (transistor test)"),
            CaptureStep::basic("scr", "Set meter to SCR (thyristor test)"),
            CaptureStep::basic("tempc", "Set meter to Temperature C"),
            CaptureStep::basic("tempf", "Set meter to Temperature F"),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)"),
            CaptureStep::basic("duty", "Set meter to Duty Cycle (%)"),
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
    check_len("ut8803", payload, 17)?;

    let mode_byte = payload[2];
    let range_raw = payload[3];
    let range_idx = if range_raw >= 0x30 {
        range_raw - 0x30
    } else {
        range_raw
    };
    let display_bytes = &payload[5..10];

    // Parse mode
    let mode: Cow<'static, str> = if (mode_byte as usize) < POSITION_TABLE.len() {
        Cow::Borrowed(POSITION_TABLE[mode_byte as usize].1)
    } else {
        debug!("ut8803: unknown mode byte {:#04x}", mode_byte);
        unknown_mode(mode_byte)
    };

    // Parse display value: 5 raw ASCII bytes appended by the vendor parser
    // (`FUN_1001fce0` per spec §2.3) and then converted to a float via
    // `FUN_1017f410`. Validate that every byte is in the character set a
    // numeric-or-"OL" display can produce; a byte outside that set means
    // the frame is malformed and should be surfaced as a parse error rather
    // than silently smuggled past `String::from_utf8_lossy` as U+FFFD,
    // which would mask real corruption during HW verification.
    for &b in display_bytes {
        let allowed = b == 0x00
            || b == b' '
            || b == b'+'
            || b == b'-'
            || b == b'.'
            || b.is_ascii_digit()
            || b == b'O'
            || b == b'L';
        if !allowed {
            return Err(Error::invalid_response(
                format!("ut8803 invalid display byte {b:#04x} in {display_bytes:02X?}"),
                payload,
            ));
        }
    }
    // All bytes validated as ASCII; construct string without the lossy path.
    // Null padding is dropped so the numeric parse below sees a clean string.
    let display_str: String = display_bytes
        .iter()
        .filter(|&&b| b != 0x00)
        .map(|&b| b as char)
        .collect();
    let display_trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Flag bit positions traced from the UT8803 parser in uci.dll
    // (`FUN_1001e5f0`). The parser builds a 32-bit status word by OR-ing
    // shifted extracts of a handful of intermediate locals, and the final
    // debug format string at line 25091 pins each status-word bit to its
    // name (`isauto = uVar9 >> 6 & 1`, `ismax = ... >> 0x1c & 1`, ...).
    // Tracing each intermediate local back to its source byte yields:
    //
    //   HOLD (D31) ← frame byte 14 bit 0 = payload[12] & 0x01
    //   OL   (D7)  ← frame byte 14 bit 2 = payload[12] & 0x04
    //   Sign (D19) ← frame byte 14 bit 3 = payload[12] & 0x08
    //   REL  (D30) ← frame byte 15 bit 0 = payload[13] & 0x01
    //   AUTO (D6)  ← frame byte 15 bit 1 = payload[13] & 0x02, **inverted**
    //                (vendor `bVar17 = local_16 == '\0';`)
    //   MIN  (D29) ← frame byte 16 bit 0 = payload[14] & 0x01
    //   MAX  (D28) ← frame byte 16 bit 1 = payload[14] & 0x02
    //
    // See docs/research/ut8803/reverse-engineered-protocol.md §2.3 for the
    // full derivation. `flags3 = payload[16]` is no longer used for any
    // documented status bit.
    let flags1_lo = payload[12];
    let flags1_hi = payload[13];
    let flags2_lo = payload[14];

    let hold = flags1_lo & 0x01 != 0;
    let overload = flags1_lo & 0x04 != 0;
    let negative = flags1_lo & 0x08 != 0;
    let rel = flags1_hi & 0x01 != 0;
    let auto_range = flags1_hi & 0x02 == 0; // inverted: clear = AUTO active
    let min_flag = flags2_lo & 0x01 != 0;
    let max_flag = flags2_lo & 0x02 != 0;

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery: false, // D15, would need to extract from status word
        hv_warning: false,
        dc: mode_is_dc(mode_byte),
        peak_max: false,
        peak_min: false,
        ..Default::default()
    };

    let unit = display_unit(mode_byte, range_idx);

    let value = if overload {
        MeasuredValue::Overload
    } else if let Ok(v) = display_trimmed.parse::<f64>() {
        // Vendor (`FUN_1001e5f0` after line 25040) checks status-word bit
        // 19 and negates the parsed float when set.
        MeasuredValue::Normal(if negative { -v } else { v })
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
        mode,
        mode_raw: mode_byte as u16,
        range_raw,
        value,
        unit: Cow::Borrowed(unit),
        display_raw: Some(display_str),
        flags,
        // `range_label` keeps the default empty string: labelling a UT8803
        // range would need the full range table, which we don't have.
        ..Measurement::from_payload(payload)
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
        _flags3: u8,
    ) -> Vec<u8> {
        make_payload_full(mode, range, display, flags1_lo, flags1_hi, 0x00)
    }

    fn make_payload_full(
        mode: u8,
        range: u8,
        display: &[u8; 5],
        flags1_lo: u8,
        flags1_hi: u8,
        flags2_lo: u8,
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
            flags2_lo,
            0x00, // bytes 14-15 (flags2)
            0x00, // byte 16 (flags3)
        ]
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x01, 0x01, b"12.34", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.34).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        let payload = make_payload(0x00, 0x01, b" 5.67", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert!(!m.flags.dc);
    }

    #[test]
    fn parse_ohm() {
        // Range 1 is the kΩ range: the display value is range-relative,
        // so the unit must carry the vendor's magnitude prefix
        // (FUN_1001cdc0, uci_dll_decompiled.txt:23644-23700).
        let payload = make_payload(0x08, 0x01, b"123.4", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
    }

    #[test]
    fn display_unit_magnitude_prefixes() {
        // Spot-check the (mode, range) → prefixed-unit table against
        // FUN_1001cdc0 / FUN_1001cff0.
        assert_eq!(display_unit(0x01, 0), "mV"); // V range 0 = mV
        assert_eq!(display_unit(0x08, 0), "Ω");
        assert_eq!(display_unit(0x08, 4), "MΩ");
        assert_eq!(display_unit(0x0D, 2), "kΩ"); // Inductance R = ESR in Ω
        assert_eq!(display_unit(0x10, 0), "Ω"); // Capacitance R = ESR in Ω
        assert_eq!(display_unit(0x0C, 0), ""); // Inductance Q: unitless
        assert_eq!(display_unit(0x0F, 0), ""); // Capacitance D: unitless
        assert_eq!(display_unit(0x0B, 0), "mH");
        assert_eq!(display_unit(0x0B, 4), "H");
        assert_eq!(display_unit(0x0E, 0), "nF");
        assert_eq!(display_unit(0x0E, 3), "µF");
        assert_eq!(display_unit(0x0E, 6), "mF");
        assert_eq!(display_unit(0x15, 0), "Hz");
        assert_eq!(display_unit(0x15, 2), "kHz");
        assert_eq!(display_unit(0x15, 5), "MHz");
        assert_eq!(display_unit(0x02, 0), "µA");
        assert_eq!(display_unit(0x07, 0), "A");
    }

    #[test]
    fn parse_unknown_mode_permissive() {
        let payload = make_payload(0x20, 0x00, b" 1.23", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Unknown(0x20)");
    }

    #[test]
    fn parse_overload_flag() {
        // OL = payload[12] bit 2 (D7 of the status word)
        let payload = make_payload(0x08, 0x00, b"    0", 0x04, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        // HOLD = payload[12] bit 0 (D31 of the status word)
        let payload = make_payload(0x01, 0x00, b" 1.23", 0x01, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_auto_range() {
        // AUTO is inverted: payload[13] bit 1 clear means AUTO active.
        let payload = make_payload(0x01, 0x00, b" 1.23", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        let payload = make_payload(0x01, 0x00, b" 1.23", 0x00, 0x02, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_rel_flag() {
        // REL = payload[13] bit 0 (D30 of the status word)
        let payload = make_payload(0x01, 0x00, b" 0.00", 0x00, 0x01, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_min_max_flags() {
        // MIN = payload[14] bit 0, MAX = payload[14] bit 1
        let payload = make_payload_full(0x01, 0x00, b" 1.23", 0x00, 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.min);
        assert!(!m.flags.max);

        let payload = make_payload_full(0x01, 0x00, b" 1.23", 0x00, 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);
    }

    #[test]
    fn parse_negative_value() {
        // Sign = payload[12] bit 3 (D19 of the status word); vendor negates
        // the parsed float when set.
        let payload = make_payload(0x01, 0x00, b"12.34", 0x08, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v + 12.34).abs() < 1e-6));
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

    #[test]
    fn display_overload_string() {
        // "   OL" is explicitly allowed by the validator
        let payload = make_payload(0x09, 0x00, b"   OL", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn display_null_padding_dropped() {
        // Trailing nulls are treated as padding, not data.
        let payload = make_payload(0x01, 0x00, b"1.2\0\0", 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("1.2"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.2).abs() < 1e-6));
    }

    #[test]
    fn invalid_display_byte_rejects_frame() {
        // A byte with the high bit set can't come from a numeric display;
        // surface it as a parse error rather than silently turning it into
        // U+FFFD via `from_utf8_lossy`.
        let payload = make_payload(0x01, 0x00, b"1\xff234", 0x00, 0x00, 0x00);
        let err = parse_measurement(&payload).unwrap_err();
        assert!(
            format!("{err}").contains("invalid display byte"),
            "expected display-byte error, got: {err}"
        );
    }

    #[test]
    fn unexpected_letter_rejects_frame() {
        // A stray ASCII letter that isn't O/L is also rejected.
        let payload = make_payload(0x01, 0x00, b"1A.34", 0x00, 0x00, 0x00);
        assert!(parse_measurement(&payload).is_err());
    }
}
