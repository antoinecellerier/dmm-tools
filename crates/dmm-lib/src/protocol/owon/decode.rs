//! 6-byte frames → [`Measurement`]
//! (`docs/research/owon/reverse-engineered-protocol.md` §6).
//!
//! Three little-endian words: function/range, status, reading. Every field
//! lands in the reading, or, where no spec section covers its value, in a
//! report as well: status bits 6-15 (but RMR on the B41T+), decimal-point
//! codes 5 and 6, the 0x6FFF magnitude, function 13 on the B series and
//! 14-15, a prefix on the functions that take none, and a frame without the
//! marker. Values the spec documents stay silent.

use super::frame::{FRAME_LEN, has_marker};
use super::model::{Function13, Model};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::unknown_mode;
use crate::protocol::unrecognised::report_unknown;
use std::borrow::Cow;

/// The function codes (spec §6.2).
pub(super) mod function {
    pub(crate) const DC_V: u8 = 0;
    pub(crate) const AC_V: u8 = 1;
    pub(crate) const DC_A: u8 = 2;
    pub(crate) const AC_A: u8 = 3;
    pub(crate) const OHMS: u8 = 4;
    pub(crate) const CAPACITANCE: u8 = 5;
    pub(crate) const HZ: u8 = 6;
    pub(crate) const DUTY: u8 = 7;
    pub(crate) const CELSIUS: u8 = 8;
    pub(crate) const FAHRENHEIT: u8 = 9;
    pub(crate) const DIODE: u8 = 10;
    pub(crate) const CONTINUITY: u8 = 11;
    pub(crate) const HFE: u8 = 12;
    pub(crate) const NCV: u8 = 13;
}

/// Prefix code 4: no prefix (spec §6.3).
const NO_PREFIX: u8 = 4;

/// Each prefixed base unit with prefixes p, n, µ, m, none, k, M, G, by
/// prefix code (spec §6.3).
pub(super) const VOLT_UNITS: [&str; 8] = ["pV", "nV", "µV", "mV", "V", "kV", "MV", "GV"];
pub(super) const AMP_UNITS: [&str; 8] = ["pA", "nA", "µA", "mA", "A", "kA", "MA", "GA"];
const OHM_UNITS: [&str; 8] = ["pΩ", "nΩ", "µΩ", "mΩ", "Ω", "kΩ", "MΩ", "GΩ"];
const FARAD_UNITS: [&str; 8] = ["pF", "nF", "µF", "mF", "F", "kF", "MF", "GF"];
const HERTZ_UNITS: [&str; 8] = ["pHz", "nHz", "µHz", "mHz", "Hz", "kHz", "MHz", "GHz"];

/// How a function's unit is written.
pub(super) enum Unit {
    /// With the frame's prefix.
    Prefixed(&'static [&'static str; 8]),
    /// With none: OWON's app forces prefix code 4 and leaves it out
    /// (spec §6.3), so any other code is reported.
    Bare(&'static str),
    /// A function no spec section covers, already reported.
    Unknown,
}

/// The mode name and unit of `function` on `model` (spec §6.2), or `None`
/// for one no spec section covers on it.
pub(super) fn function_of(function: u8, model: &Model) -> Option<(&'static str, Unit)> {
    use Unit::{Bare, Prefixed};
    Some(match function {
        function::DC_V => ("DC V", Prefixed(&VOLT_UNITS)),
        function::AC_V => ("AC V", Prefixed(&VOLT_UNITS)),
        function::DC_A => ("DC A", Prefixed(&AMP_UNITS)),
        function::AC_A => ("AC A", Prefixed(&AMP_UNITS)),
        function::OHMS => ("Ω", Prefixed(&OHM_UNITS)),
        function::CAPACITANCE => ("Capacitance", Prefixed(&FARAD_UNITS)),
        function::HZ => ("Hz", Prefixed(&HERTZ_UNITS)),
        function::DUTY => ("Duty %", Bare("%")),
        function::CELSIUS => ("°C", Bare("°C")),
        // Both programs label 9 ℉ and show the count as sent; a note on
        // one OW18E says it carries ℃ there (spec §14.3 D10).
        function::FAHRENHEIT => ("°F", Bare("°F")),
        function::DIODE => ("Diode", Prefixed(&VOLT_UNITS)),
        function::CONTINUITY => ("Continuity", Prefixed(&OHM_UNITS)),
        function::HFE => ("hFE", Bare("")),
        function::NCV if model.function13 == Function13::Ncv => ("NCV", Bare("")),
        // 13 on the B series, 14-15 (spec §6.2).
        _ => return None,
    })
}

/// Status bits 0-5 (spec §6.6), the same in the 15-byte frame (spec §10.2).
pub(super) const HOLD: u16 = 0x0001;
pub(super) const REL: u16 = 0x0002;
pub(super) const AUTO: u16 = 0x0004;
pub(super) const LOW_BATTERY: u16 = 0x0008;
pub(super) const MIN: u16 = 0x0010;
pub(super) const MAX: u16 = 0x0020;
/// Bits 6-15, whose meanings in a 6-byte meter's frames are open (spec
/// §6.6, §14.3 D2).
const UNDOCUMENTED: u16 = 0xFFC0;
/// Bit 7, RMR, "only B41 model" (spec §6.6).
const RMR: u16 = 0x0080;

/// Decimal-point codes 6 and 7: UL and OL (spec §6.4).
pub(super) const DP_UL: u8 = 6;
pub(super) const DP_OL: u8 = 7;
/// Decimal-point code 5: ÷10⁵ in OWON's app, unscaled in its PC software
/// (spec §6.4).
pub(super) const DP_5: u8 = 5;
/// The magnitude OWON's app treats as no reading (spec §6.5).
const NO_READING: u16 = 0x6FFF;

/// Decode one frame as `model` reads it.
pub(super) fn decode(p: &[u8], model: &Model) -> Result<Measurement> {
    if p.len() != FRAME_LEN {
        return Err(Error::invalid_response(
            format!(
                "{} payload is {} bytes, expected {FRAME_LEN}",
                model.id,
                p.len()
            ),
            p,
        ));
    }
    let report =
        |what: &'static str| report_unknown(model.id, what, format_args!("frame {p:02X?}"));

    // Function/range word (spec §6.1).
    let gear = u16::from_le_bytes([p[0], p[1]]);
    if !has_marker(p[1]) {
        report("function-word bits 10-15");
    }
    let dp = (gear & 0x07) as u8;
    let prefix = ((gear >> 3) & 0x07) as u8;
    let code = ((gear >> 6) & 0x0F) as u8;

    let (mode, unit) = match function_of(code, model) {
        Some((mode, unit)) => (Cow::Borrowed(mode), unit),
        None => {
            report("function code");
            (unknown_mode(code), Unit::Unknown)
        }
    };
    let unit = match unit {
        Unit::Prefixed(units) => units[usize::from(prefix)],
        Unit::Bare(unit) => {
            if prefix != NO_PREFIX {
                report("prefix");
            }
            unit
        }
        Unit::Unknown => "",
    };

    // Status word (spec §6.6).
    let status = u16::from_le_bytes([p[2], p[3]]);
    let silent = if model.rmr { RMR } else { 0 };
    if status & UNDOCUMENTED & !silent != 0 {
        report("status bits");
    }

    // Reading word: sign and magnitude (spec §6.5).
    let reading = u16::from_le_bytes([p[4], p[5]]);
    let negative = reading & 0x8000 != 0;
    let magnitude = reading & 0x7FFF;

    let ncv = code == function::NCV && model.function13 == Function13::Ncv;
    let (value, display_raw) = if ncv && dp == 0 && !negative && magnitude <= 4 {
        // Level 0 is the CM2100's "EF", 1-4 its dashes (spec §6.7).
        (MeasuredValue::NcvLevel(magnitude as u8), None)
    } else {
        if ncv {
            report("NCV level");
        }
        match dp {
            // Both programs ignore the magnitude under OL (spec §6.4).
            DP_OL => {
                if negative {
                    report("negative OL");
                }
                (MeasuredValue::Overload, None)
            }
            // No manual names UL, and no capture has it (spec §6.4).
            DP_UL => {
                report("decimal-point code 6 (UL)");
                (MeasuredValue::NoReading("UL"), None)
            }
            _ if magnitude == NO_READING => {
                report("magnitude 0x6FFF");
                (MeasuredValue::NoReading("----"), None)
            }
            _ => {
                // Code 5 follows OWON's app (spec §6.4).
                if dp == DP_5 {
                    report("decimal-point code 5");
                }
                let (value, digits) = number(negative, u32::from(magnitude), dp);
                (MeasuredValue::Normal(value), Some(digits))
            }
        }
    };

    // The frame names no range, only the prefix and point (spec §6.1).
    Ok(Measurement {
        mode,
        mode_raw: u16::from(code),
        range_raw: (gear & 0x3F) as u8,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags: StatusFlags {
            hold: status & HOLD != 0,
            rel: status & REL != 0,
            auto_range: status & AUTO != 0,
            low_battery: status & LOW_BATTERY != 0,
            min: status & MIN != 0,
            max: status & MAX != 0,
            dc: matches!(code, function::DC_V | function::DC_A),
            ..StatusFlags::default()
        },
        ..Measurement::from_payload(p)
    })
}

/// The value and the digits of a count with `decimals` digits after the
/// point, built from integers as the LCD draws them: no leading zeros
/// beyond the one before the point.
pub(super) fn number(negative: bool, magnitude: u32, decimals: u8) -> (f64, String) {
    let sign = if negative { "-" } else { "" };
    let divisor = 10u32.pow(u32::from(decimals));
    let digits = if decimals == 0 {
        format!("{sign}{magnitude}")
    } else {
        let width = usize::from(decimals);
        format!(
            "{sign}{}.{:0width$}",
            magnitude / divisor,
            magnitude % divisor
        )
    };
    let value = f64::from(magnitude) / f64::from(divisor);
    (if negative { -value } else { value }, digits)
}

#[cfg(test)]
mod tests {
    use super::super::model::{B35, B41, CM2100B, MODELS, OW18B, OW18E};
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::test_support::snapshot;

    /// Decode `p` as `model`, with no report allowed.
    fn quiet(p: &[u8], model: &Model) -> Measurement {
        let (m, reports) = capture_reports(|| decode(p, model));
        assert!(reports.is_empty(), "{p:02X?}: {reports:?}");
        m.unwrap()
    }

    /// Decode `p` as `model`, and what it reported.
    fn reported(p: &[u8], model: &Model) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(p, model));
        (m.unwrap(), reports)
    }

    /// Spec §12.1, OWON's own example, every field.
    #[test]
    fn the_worked_example_decodes_every_field() {
        let m = quiet(&[0x19, 0xF0, 0x04, 0x00, 0xBD, 0x09], &B35);
        assert_eq!(
            snapshot(&m),
            "mode=DC V\n\
             mode_raw=0x00\n\
             range_raw=0x19\n\
             value=Normal(249.3)\n\
             unit=mV\n\
             range_label=\n\
             display_raw=Some(\"249.3\")\n\
             flags=auto_range,dc\n\
             aux=0\n\
             raw_payload=6"
        );
    }

    /// Spec §12.4's constructed negative reading.
    #[test]
    fn a_negative_reading_is_sign_and_magnitude() {
        let m = quiet(&[0x22, 0xF0, 0x04, 0x00, 0xD2, 0x84], &B35);
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::Normal(-12.34))
        );
        assert_eq!(m.display_raw.as_deref(), Some("-12.34"));
    }

    /// Spec §14.5's captured frames, each as its model, with no report.
    #[test]
    fn the_captured_vectors_decode_silently() {
        use MeasuredValue::{Normal, Overload};
        type Case = (
            [u8; 6],
            &'static Model,
            &'static str,
            &'static str,
            MeasuredValue,
            Option<&'static str>,
        );
        let cases: [Case; 17] = [
            (
                [0x19, 0xF0, 0x04, 0x00, 0xE9, 0x0D],
                &B35,
                "DC V",
                "mV",
                Normal(356.1),
                Some("356.1"),
            ),
            (
                [0x20, 0xF2, 0x00, 0x00, 0x1D, 0x00],
                &B35,
                "°C",
                "°C",
                Normal(29.0),
                Some("29"),
            ),
            (
                [0x63, 0xF0, 0x04, 0x00, 0x10, 0x00],
                &B35,
                "AC V",
                "V",
                Normal(0.016),
                Some("0.016"),
            ),
            (
                [0x21, 0xF1, 0x04, 0x00, 0x07, 0x00],
                &B35,
                "Ω",
                "Ω",
                Normal(0.7),
                Some("0.7"),
            ),
            (
                [0xE7, 0xF2, 0x00, 0x00, 0x00, 0x00],
                &B35,
                "Continuity",
                "Ω",
                Overload,
                None,
            ),
            (
                [0x37, 0xF1, 0x04, 0x00, 0x00, 0x00],
                &B35,
                "Ω",
                "MΩ",
                Overload,
                None,
            ),
            (
                [0xA7, 0xF2, 0x00, 0x00, 0x00, 0x00],
                &B35,
                "Diode",
                "V",
                Overload,
                None,
            ),
            (
                [0x19, 0xF0, 0x04, 0x00, 0x2B, 0x89],
                &B35,
                "DC V",
                "mV",
                Normal(-234.7),
                Some("-234.7"),
            ),
            (
                [0x19, 0xF0, 0x04, 0x00, 0x00, 0x80],
                &B35,
                "DC V",
                "mV",
                Normal(-0.0),
                Some("-0.0"),
            ),
            (
                [0x19, 0xF0, 0x04, 0x00, 0x49, 0x04],
                &B35,
                "DC V",
                "mV",
                Normal(109.7),
                Some("109.7"),
            ),
            (
                [0x34, 0xF1, 0x04, 0x00, 0x81, 0x2B],
                &B41,
                "Ω",
                "MΩ",
                Normal(1.1137),
                Some("1.1137"),
            ),
            (
                [0x24, 0xF0, 0x04, 0x00, 0xC6, 0x3A],
                &CM2100B,
                "DC V",
                "V",
                Normal(1.5046),
                Some("1.5046"),
            ),
            (
                [0x22, 0xF1, 0x04, 0x00, 0x9F, 0x11],
                &CM2100B,
                "Ω",
                "Ω",
                Normal(45.11),
                Some("45.11"),
            ),
            (
                [0x2B, 0xF1, 0x04, 0x00, 0x56, 0x09],
                &OW18E,
                "Ω",
                "kΩ",
                Normal(2.39),
                Some("2.390"),
            ),
            (
                [0x22, 0xF0, 0x04, 0x00, 0x00, 0x00],
                &OW18B,
                "DC V",
                "V",
                Normal(0.0),
                Some("0.00"),
            ),
            (
                [0x24, 0xF0, 0x05, 0x00, 0x1F, 0x00],
                &OW18E,
                "DC V",
                "V",
                Normal(0.0031),
                Some("0.0031"),
            ),
            (
                [0x24, 0xF0, 0x04, 0x00, 0x03, 0x00],
                &B41,
                "DC V",
                "V",
                Normal(0.0003),
                Some("0.0003"),
            ),
        ];
        for (p, model, mode, unit, value, digits) in cases {
            let m = quiet(&p, model);
            assert_eq!(m.mode, mode, "{p:02X?}");
            assert_eq!(m.unit, unit, "{p:02X?}");
            assert_eq!(format!("{:?}", m.value), format!("{:?}", value), "{p:02X?}");
            assert_eq!(m.display_raw.as_deref(), digits, "{p:02X?}");
            assert_eq!(m.raw_payload, p);
        }
    }

    /// The flags of the captured frames: AUTO alone, none, HOLD with AUTO.
    #[test]
    fn the_status_word_sets_the_flags() {
        let auto = quiet(&[0x19, 0xF0, 0x04, 0x00, 0xE9, 0x0D], &B35);
        assert!(auto.flags.auto_range && auto.flags.dc && !auto.flags.hold);
        let none = quiet(&[0x20, 0xF2, 0x00, 0x00, 0x1D, 0x00], &B35);
        assert_eq!(none.flags, StatusFlags::default());
        let held = quiet(&[0x24, 0xF0, 0x05, 0x00, 0x1F, 0x00], &OW18E);
        assert!(held.flags.hold && held.flags.auto_range);
        // Bits 1, 3, 4, 5: REL, low battery, MIN, MAX; AC V is not DC.
        let m = quiet(&[0x63, 0xF0, 0x3A, 0x00, 0x10, 0x00], &B35);
        let f = m.flags;
        assert!(f.rel && f.low_battery && f.min && f.max);
        assert!(!f.hold && !f.auto_range && !f.dc);
        let amps = quiet(&[0xA2, 0xF0, 0x00, 0x00, 0x10, 0x00], &B35);
        assert_eq!(amps.mode, "DC A");
        assert!(amps.flags.dc);
    }

    /// A count past 14 bits, as on a 19999-count OW18E (spec §14.3 D4).
    #[test]
    fn a_count_above_16383_keeps_bit_14() {
        let m = quiet(&[0x2C, 0xF1, 0x04, 0x00, 0x0D, 0x7F], &OW18E);
        assert_eq!(m.unit, "kΩ");
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::Normal(3.2525))
        );
        assert_eq!(m.display_raw.as_deref(), Some("3.2525"));
    }

    /// Every function code a 6-byte meter documents, each prefix on the
    /// prefixed ones, silently.
    #[test]
    fn every_documented_function_is_silent() {
        let names = [
            ("DC V", "V"),
            ("AC V", "V"),
            ("DC A", "A"),
            ("AC A", "A"),
            ("Ω", "Ω"),
            ("Capacitance", "F"),
            ("Hz", "Hz"),
            ("Duty %", "%"),
            ("°C", "°C"),
            ("°F", "°F"),
            ("Diode", "V"),
            ("Continuity", "Ω"),
            ("hFE", ""),
        ];
        for (code, (mode, unit)) in names.iter().enumerate() {
            // Prefix 4, dp 1, 123 → 12.3.
            let gear = 0xF000 | (code as u16) << 6 | 4 << 3 | 1;
            let [lo, hi] = gear.to_le_bytes();
            let m = quiet(&[lo, hi, 0x00, 0x00, 123, 0x00], &B35);
            assert_eq!((m.mode.as_ref(), m.unit.as_ref()), (*mode, *unit));
            assert_eq!(m.mode_raw, code as u16);
            assert_eq!(m.display_raw.as_deref(), Some("12.3"));
        }
        let prefixes = ["pF", "nF", "µF", "mF", "F", "kF", "MF", "GF"];
        for (prefix, unit) in prefixes.iter().enumerate() {
            let gear = 0xF000 | 5 << 6 | (prefix as u16) << 3 | 3;
            let [lo, hi] = gear.to_le_bytes();
            assert_eq!(quiet(&[lo, hi, 0x00, 0x00, 1, 0x00], &B35).unit, *unit);
        }
    }

    /// NCV word `0xF360`, levels 0-4: a level on the meters with an NCV
    /// position, reported on the B series (spec §6.7, §14.2).
    #[test]
    fn ncv_levels_are_read_on_the_ncv_models_only() {
        for level in 0..=4u8 {
            let p = [0x60, 0xF3, 0x00, 0x00, level, 0x00];
            for model in [&OW18B, &OW18E, &CM2100B] {
                let m = quiet(&p, model);
                assert_eq!(m.mode, "NCV");
                assert_eq!(m.unit, "");
                assert_eq!(
                    format!("{:?}", m.value),
                    format!("{:?}", MeasuredValue::NcvLevel(level))
                );
                assert_eq!(m.display_raw, None);
            }
            let (m, reports) = reported(&p, &B35);
            assert_eq!(reports.len(), 1, "{reports:?}");
            assert!(reports[0].contains("function code"), "{reports:?}");
            assert_eq!(m.mode, "Unknown(0x0d)");
        }
        let (m, reports) = reported(&[0x60, 0xF3, 0x00, 0x00, 0x05, 0x00], &OW18B);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(reports[0].contains("NCV level"));
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::Normal(5.0))
        );
    }

    /// What no spec section covers is decoded where it can be and
    /// reported, one report per frame.
    #[test]
    fn undocumented_values_are_reported() {
        let cases: [(&str, [u8; 6], MeasuredValue); 8] = [
            (
                "status bits",
                [0x19, 0xF0, 0x44, 0x00, 0xBD, 0x09],
                MeasuredValue::Normal(249.3),
            ),
            (
                "decimal-point code 5",
                [0x25, 0xF0, 0x04, 0x00, 0x39, 0x30],
                MeasuredValue::Normal(0.12345),
            ),
            (
                "decimal-point code 6 (UL)",
                [0x26, 0xF0, 0x04, 0x00, 0x00, 0x00],
                MeasuredValue::NoReading("UL"),
            ),
            (
                "magnitude 0x6FFF",
                [0x22, 0xF0, 0x04, 0x00, 0xFF, 0x6F],
                MeasuredValue::NoReading("----"),
            ),
            (
                "function code",
                [0xA2, 0xF3, 0x04, 0x00, 0x10, 0x00],
                MeasuredValue::Normal(0.16),
            ),
            (
                "prefix",
                [0x18, 0xF2, 0x00, 0x00, 0x1D, 0x00],
                MeasuredValue::Normal(29.0),
            ),
            (
                "negative OL",
                [0x37, 0xF1, 0x04, 0x00, 0x00, 0x80],
                MeasuredValue::Overload,
            ),
            (
                "function-word bits 10-15",
                [0x19, 0xE0, 0x04, 0x00, 0xBD, 0x09],
                MeasuredValue::Normal(249.3),
            ),
        ];
        for (what, p, value) in cases {
            let (m, reports) = reported(&p, &B35);
            assert_eq!(reports.len(), 1, "{what}: {reports:?}");
            assert!(reports[0].contains(what), "{what}: {reports:?}");
            assert!(reports[0].starts_with("b35t+:"), "{reports:?}");
            assert_eq!(format!("{:?}", m.value), format!("{:?}", value), "{what}");
        }
        let (m, _) = reported(&[0x25, 0xF0, 0x04, 0x00, 0x39, 0x30], &B35);
        assert_eq!(m.display_raw.as_deref(), Some("0.12345"));
        let (m, _) = reported(&[0xA2, 0xF3, 0x04, 0x00, 0x10, 0x00], &B35);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("Unknown(0x0e)", ""));
    }

    /// RMR, status bit 7, is the B41T+'s own (spec §6.6): silent there,
    /// reported on every other model.
    #[test]
    fn rmr_is_silent_on_the_b41t_only() {
        let p = [0x24, 0xF0, 0x84, 0x00, 0x03, 0x00];
        for model in MODELS {
            let (m, reports) = reported(&p, model);
            assert!(m.flags.auto_range);
            assert_eq!(reports.is_empty(), model.id == "b41t+", "{}", model.id);
        }
        // Another high bit still reports on the B41T+.
        let (_, reports) = reported(&[0x24, 0xF0, 0x84, 0x01, 0x03, 0x00], &B41);
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    #[test]
    fn a_frame_of_another_length_is_an_error() {
        for len in [0, 5, 7, 12] {
            let p = vec![0xF0; len];
            assert!(
                matches!(decode(&p, &B35), Err(Error::InvalidResponse { .. })),
                "{len}"
            );
        }
    }
}
