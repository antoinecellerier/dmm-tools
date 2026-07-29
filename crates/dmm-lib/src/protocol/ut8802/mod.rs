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
///
/// `unit` carries the SI prefix for the position, because the meter's five
/// display digits are **range-relative**: with a decimal point at 0-4 the
/// smallest magnitude it can express is 0.0001, so 10 nF or 1.5 MHz simply
/// cannot be sent in farads or hertz. The prefix per position comes from
/// `FUN_1001cd30` in uci.dll (line 23603), which returns the UnitMag index
/// (0=n, 1=µ, 2=m, 3=none, 4=k, 5=M) that the vendor renders via
/// `FUN_1001cec0`; the base units it combines with are `FUN_1001cf30`
/// (line 23729) and match the `unit` column here. The sibling UT8803 parser
/// resolves its units the same way (`display_unit`, FUN_1001cdc0/FUN_1001cff0).
/// See docs/research/uci-bench-family/reverse-engineered-protocol.md §3.3.
///
/// `range_label` is left empty where the manual publishes no numeric span —
/// the capacitance and frequency positions are named only by their decade,
/// which is now in `unit`.
const POSITION_TABLE: &[(u8, &str, &str, &str)] = &[
    (0x01, "DC V", "mV", "200mV"),
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
    (0x1A, "Ω", "kΩ", "2kΩ"),
    (0x1B, "Ω", "kΩ", "20kΩ"),
    (0x1C, "Ω", "kΩ", "200kΩ"),
    (0x1D, "Ω", "MΩ", "2MΩ"),
    (0x1F, "Ω", "MΩ", "200MΩ"),
    (0x22, "Duty %", "%", ""),
    (0x23, "Diode", "V", ""),
    (0x24, "Continuity", "Ω", ""),
    (0x25, "hFE", "", ""),
    (0x27, "Capacitance", "nF", ""),
    (0x28, "Capacitance", "µF", ""),
    (0x29, "Capacitance", "mF", ""),
    (0x2A, "SCR", "V", ""),
    (0x2B, "Hz", "Hz", ""),
    (0x2C, "Hz", "kHz", ""),
    (0x2D, "Hz", "MHz", ""),
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
                verification_issue: Some(12),
            },
        }
    }
}

impl Protocol for Ut8802Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // No trigger byte: the vendor's CP2110 init path (uci.dll
        // FUN_1001d460) never writes to the UART; the 0x5A trigger we
        // previously sent belongs to the QinHeng/CH9325 init path
        // (FUN_1001d360) for other meters. See the UT8803 init for the
        // same correction.
        debug!("ut8802: init (no trigger byte; meter streams unprompted)");
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

/// Whether a position code measures a DC quantity.
///
/// The vendor derives the status-word AC/DC field from the position code
/// alone via a lookup (`FUN_1001ca30`, uci_dll_decompiled.txt:23411-23445:
/// returns 2=DC for the codes below, 1=AC for 0x09-0x0C/0x10/0x13/0x14/0x18,
/// 0 otherwise). Frame byte 5 bits 4-5 — previously misread as AC/DC
/// coupling — are diode/SCR probe-direction indicators.
fn position_is_dc(position: u8) -> bool {
    matches!(
        position,
        0x01 | 0x03..=0x06 | 0x0D | 0x0E | 0x11 | 0x12 | 0x16
    )
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
/// Layout (from Ghidra FUN_1001e0a0 + programming manual; payload index =
/// frame byte − 1):
/// - byte 0: position code (0x01-0x2D, combined function + range)
/// - bytes 1-3: 5 display nibbles, most significant digit in byte 3's low
///   nibble: d1=b3 lo, d2=b2 hi, d3=b2 lo, d4=b1 hi, d5(LSD)=b1 lo
///   (vendor stack slots at uci_dll_decompiled.txt:24714-24719; byte 3's
///   high nibble is never read)
/// - byte 4: decimal point position (low nibble, 0-4) + diode/SCR probe
///   direction (bits 4-5, consumed only for positions 0x23/0x2A,
///   uci_dll_decompiled.txt:24727, 24777-24800 — NOT AC/DC coupling)
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
    // The framing extractor already rejects dp_pos > 4, but the index math
    // below (`chars.len() - dp_pos - 1`) would panic on larger values, so
    // guard here too in case this is ever called on unvalidated payloads.
    if dp_pos > 4 {
        return Err(Error::invalid_response(
            format!("ut8802 invalid decimal point position {dp_pos}"),
            payload,
        ));
    }
    let sign_byte = payload[6];

    // Look up position code
    let (mode, unit, range_label): (Cow<'static, str>, &'static str, &'static str) =
        if let Some((m, u, r)) = lookup_position(position) {
            (Cow::Borrowed(m), u, r)
        } else {
            debug!("ut8802: unknown position code {position:#04x}");
            (Cow::Owned(format!("Unknown({position:#04x})")), "", "")
        };

    // Decode the 5 display nibbles, MSD first. The vendor parser fills its
    // display buffer as [b3 lo, b2 hi, b2 lo, b1 hi, b1 lo] (payload
    // indexing; uci_dll_decompiled.txt:24714-24719) and runs atof directly
    // on the resulting string, so byte 3's low nibble is the most
    // significant digit.
    let nibbles = [
        payload[3] & 0x0F,
        payload[2] >> 4,
        payload[2] & 0x0F,
        payload[1] >> 4,
        payload[1] & 0x0F,
    ];

    // The vendor's only overload mechanism is byte 6 bit 6: it skips the
    // atof and substitutes a sentinel plus the literal display "  0L "
    // (uci_dll_decompiled.txt:24806-24821). The digit-nibble 0x0C ('L')
    // check below is kept as a defensive secondary — the vendor never
    // validates digit nibbles.
    //
    // Both sources must be resolved before the display string is built: the
    // decimal-point insertion below is deliberately skipped on overload, so
    // folding the vendor bit in afterwards left an over-range frame carrying
    // a formatted numeric display ("1234.5") next to MeasuredValue::Overload.
    let mut overload = sign_byte & 0x40 != 0;
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

    // Flag bit positions from the UT8802 parser in uci.dll (`FUN_1001e0a0`,
    // status-word construction at line 24768-24773). Tracing each source
    // bit through the shift chain back to `param_2[7]` and matching the
    // status-word bits to the debug format string at line 24865 gives:
    //
    //   bit 0: MIN  (→ status-word D29)
    //   bit 1: MAX  (→ D28)
    //   bit 2: AUTO **inverted** (→ D6 via `~bit2`) [VENDOR]
    //   bit 3: REL  (→ D30)
    //   bit 4: HOLD (→ D31)
    //   bit 5: Over (→ D18; not surfaced by StatusFlags)
    //   bit 6: OL   (→ D7)
    //   bit 7: sign (→ D19, handled above)
    //
    // See docs/research/uci-bench-family/reverse-engineered-protocol.md §3.5.
    let auto_range = sign_byte & 0x04 == 0;
    let hold = sign_byte & 0x10 != 0;
    let rel = sign_byte & 0x08 != 0;
    let max_flag = sign_byte & 0x02 != 0;
    let min_flag = sign_byte & 0x01 != 0;

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery: false,
        hv_warning: false,
        dc: position_is_dc(position),
        peak_max: false,
        peak_min: false,
        ..Default::default()
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
        spec: None,
        mode_spec: None,
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
        // digits[] is MSD-first; wire order is d1=b3 lo, d2=b2 hi,
        // d3=b2 lo, d4=b1 hi, d5=b1 lo (payload bytes 1-3).
        vec![
            position,
            (digits[3] << 4) | digits[4],
            (digits[1] << 4) | digits[2],
            digits[0],
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
        // Resistance 2kΩ, display "1.234" (dp_pos=3). The digits are
        // range-relative, so this is 1.234 kΩ — the unit carries the prefix.
        let payload = make_payload(0x1A, [0, 1, 2, 3, 4], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "2kΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.234).abs() < 1e-6));
    }

    /// The five display digits are range-relative: 10 nF is sent as "10.00"
    /// on the nF position, and reporting that in farads would be off by 10^9.
    /// Prefixes come from `FUN_1001cd30` (uci.dll).
    #[test]
    fn capacitance_units_carry_the_range_decade() {
        for (position, expected) in [(0x27, "nF"), (0x28, "µF"), (0x29, "mF")] {
            let payload = make_payload(position, [0, 1, 0, 0, 0], 2, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.mode, "Capacitance");
            assert_eq!(m.unit, expected, "position {position:#04x}");
            assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 10.0).abs() < 1e-6));
        }
    }

    #[test]
    fn frequency_units_carry_the_range_decade() {
        for (position, expected) in [(0x2B, "Hz"), (0x2C, "kHz"), (0x2D, "MHz")] {
            let payload = make_payload(position, [0, 1, 5, 0, 0], 3, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.mode, "Hz");
            assert_eq!(m.unit, expected, "position {position:#04x}");
            assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.5).abs() < 1e-6));
        }
    }

    /// The 200 mV position reports millivolts, not volts — `FUN_1001cd30`
    /// returns the `m` prefix for position 0x01.
    #[test]
    fn millivolt_range_reports_millivolts() {
        let payload = make_payload(0x01, [0, 1, 2, 3, 4], 2, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "mV");
        assert_eq!(m.range_label, "200mV");
    }

    /// Positions whose base unit needs no prefix must stay unprefixed.
    #[test]
    fn unprefixed_positions_keep_the_base_unit() {
        for (position, expected) in [
            (0x04, "V"),  // DC V 20V
            (0x16, "A"),  // DC A 2A
            (0x19, "Ω"),  // 200Ω
            (0x22, "%"),  // duty
            (0x23, "V"),  // diode
            (0x24, "Ω"),  // continuity
            (0x25, ""),   // hFE
            (0x2B, "Hz"), // Hz
        ] {
            let payload = make_payload(position, [0, 0, 1, 0, 0], 1, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.unit, expected, "position {position:#04x}");
        }
    }

    #[test]
    fn parse_overload() {
        // Digit with 0x0C nibble → overload
        let payload = make_payload(0x01, [0, 0, 0x0C, 0, 0], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn out_of_range_dp_pos_errors_instead_of_panicking() {
        // dp_pos 5-15 would underflow the decimal-insertion index math;
        // normally rejected by the framing extractor, but parse_measurement
        // must also refuse it rather than panic.
        for dp_pos in 5..=15u8 {
            let payload = make_payload(0x05, [1, 2, 3, 4, 5], dp_pos, 0x02, 0x00, 0x00);
            assert!(parse_measurement(&payload).is_err(), "dp_pos={dp_pos}");
        }
    }

    #[test]
    fn parse_negative() {
        // Bit 7 of sign byte = negative
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x80);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-1234.5)).abs() < 1e-6));
    }

    #[test]
    fn digit_order_matches_vendor_wire_layout() {
        // Raw bytes, not via make_payload: digits 1,2,3,4,5 (MSD→LSD) are
        // d1=byte3 lo, d2=byte2 hi, d3=byte2 lo, d4=byte1 hi, d5=byte1 lo
        // (uci_dll_decompiled.txt:24714-24719). dp_pos=1 → "1234.5".
        let payload = [0x05, 0x45, 0x23, 0x01, 0x01, 0x00, 0x00];
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1234.5).abs() < 1e-6));
    }

    #[test]
    fn overload_from_status_bit() {
        // Vendor overload = byte 6 bit 6, with ordinary digits on the wire
        // (uci_dll_decompiled.txt:24806-24821).
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x40);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    /// The vendor bit has to be folded in before the display string is
    /// formatted. Inserting the decimal point first left the measurement
    /// carrying `display_raw = "1234.5"` next to `MeasuredValue::Overload`,
    /// so anything rendering the raw digits showed an over-range input as a
    /// plausible reading.
    #[test]
    fn status_bit_overload_does_not_produce_a_formatted_number() {
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x40);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        let raw = m.display_raw.as_deref().unwrap_or_default();
        assert!(
            !raw.contains('.'),
            "overload display must not be decimal-formatted, got {raw:?}"
        );
        // And the export/display path must call it what it is.
        assert_eq!(m.value_export_str(), "OL");
    }

    #[test]
    fn dc_flag_from_position_code() {
        // AC position (0x09 = AC V per FUN_1001ca30) must not set dc even
        // though byte 4 bits 4-5 (diode/SCR direction) read as "2".
        let payload = make_payload(0x09, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.dc);

        // DC position (0x05 = DC V) sets dc regardless of byte 4 bits.
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x01, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.dc);
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
        // HOLD = byte 7 bit 4 (status-word D31)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x10);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        // REL = byte 7 bit 3 (D30)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x08);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        // MAX = byte 7 bit 1 (D28), MIN = byte 7 bit 0 (D29)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);

        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.max);
        assert!(m.flags.min);
    }

    /// Byte 4 bits 4-5 do **not** encode AC/DC coupling.
    ///
    /// They were read that way once ("0=OFF, 1=AC, 2=DC, 3=AC+DC"), and this
    /// test was written to that reading — but it only ever passed because
    /// each case's position code happened to agree with the coupling bits it
    /// asserted on. `position_is_dc` documents the corrected reading: the
    /// vendor derives AC/DC from the position code alone (`FUN_1001ca30`),
    /// and these bits are diode/SCR probe-direction indicators.
    ///
    /// So sweep all four bit values against a fixed position and assert the
    /// flag doesn't move. That fails if anyone reintroduces the old reading.
    #[test]
    fn acdc_bits_do_not_affect_the_dc_flag() {
        for acdc_bits in 0..=3u8 {
            // 0x05 = DC V: dc must stay set whatever the bits say.
            let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, acdc_bits, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(m.flags.dc, "DC position with acdc_bits={acdc_bits}");

            // 0x0A = AC V: dc must stay clear whatever the bits say.
            let payload = make_payload(0x0A, [0, 1, 2, 3, 4], 2, acdc_bits, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(!m.flags.dc, "AC position with acdc_bits={acdc_bits}");
        }
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
