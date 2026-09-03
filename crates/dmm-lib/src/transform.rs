//! Software transforms applied to a reading after acquisition.
//!
//! A transform re-expresses the main reading — a current clamp's 100 mV/A, a
//! pressure transducer's V/PSI, °C to °F — without the meter knowing anything
//! about it. The arithmetic runs on the reading converted to its **base SI
//! unit** (`si_prefix`), because meters auto-range mid-run: a factor typed
//! against a reading in mV would be wrong by 1000× the moment the meter
//! switched to V.
//!
//! The transformed value becomes the main reading and the meter's own reading
//! is kept as a [`RAW_LABEL`] sub-value. That mirrors how meters present
//! derived readings themselves — Fluke's REL and the UT181A's relative and dBm
//! formats put the derived quantity on the main display and the measured one
//! beneath it — so the graph selector, the CSV aux columns and the status line
//! show both without any of them learning about transforms.

use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use std::borrow::Cow;

/// Label of the sub-value carrying the meter's untransformed reading.
pub const RAW_LABEL: &str = "Raw";

/// SI prefixes a meter unit may carry, with the multiplier that converts a
/// reading in the prefixed unit to the base unit.
///
/// Case-sensitive, so "mF" stays millifarads and "MΩ" stays megohms. Both the
/// micro sign "µ" the parsers emit and the ASCII "u" a user can type map to
/// 1e-6.
const SI_PREFIXES: &[(char, f64)] = &[
    ('p', 1e-12),
    ('n', 1e-9),
    ('µ', 1e-6),
    ('u', 1e-6),
    ('m', 1e-3),
    ('k', 1e3),
    ('M', 1e6),
    ('G', 1e9),
];

/// Units a prefix may be split off.
///
/// Deliberately not exhaustive over SI — it lists only what the parsers emit,
/// so a unit that merely starts with a prefix letter is left alone: "ms" is
/// milliseconds, not milli-seconds ("s" is not a base here), and "°C", "°F",
/// "%", "dBm" and "dBV" carry no prefix at all.
const BASE_UNITS: &[&str] = &[
    "V", "A", "Ω", "F", "Hz", "S", "W", "VAC", "VDC", "AAC", "ADC",
];

/// Split a meter unit into its base unit and the multiplier to that base:
/// "mV" → ("V", 1e-3), "kΩ" → ("Ω", 1e3), "µA" → ("A", 1e-6).
///
/// Unknown strings, and units that carry no prefix, pass through with a
/// multiplier of 1.0 — "°C", "ms" and "" all return themselves.
pub(crate) fn si_prefix(unit: &str) -> (&str, f64) {
    let mut chars = unit.chars();
    let Some(first) = chars.next() else {
        return (unit, 1.0);
    };
    let base = chars.as_str();
    let Some(&(_, mult)) = SI_PREFIXES.iter().find(|(prefix, _)| *prefix == first) else {
        return (unit, 1.0);
    };
    if BASE_UNITS.contains(&base) {
        (base, mult)
    } else {
        (unit, 1.0)
    }
}

/// Which arithmetic a transform applies.
///
/// Single variant today. `Binary` (a second series: V×I, A−B) and `Formula`
/// are the planned extensions; unlike `Linear` they produce a new quantity
/// rather than re-expressing the main reading, so they will append a
/// sub-value instead of replacing the main one. See `docs/architecture.md`,
/// "Derived series model".
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// `y = scale * x_base + offset`, with `x_base` in the reading's base unit.
    Linear { scale: f64, offset: f64 },
}

/// Why a scale or offset value is unusable.
///
/// The rules are the same wherever a factor is typed — the `--scale` and
/// `--offset` flags, the GUI's Scale row — but the wording of the complaint
/// is not, so this names the reason and leaves the message to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorError {
    /// NaN or infinity. Nothing downstream catches either: they propagate
    /// through every reading into the stats and the integral, whose summary
    /// then reads `NaN` with nothing to say where it came from.
    NotFinite,
    /// A zero scale, which collapses every reading onto the offset — it
    /// destroys the reading rather than re-expressing it.
    ZeroScale,
}

/// A user-supplied transform over the main reading.
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub op: Op,
    /// Unit to show instead of the reading's base unit. `None` keeps the base.
    pub unit: Option<String>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            op: Op::Linear {
                scale: 1.0,
                offset: 0.0,
            },
            unit: None,
        }
    }
}

impl Transform {
    /// A linear transform. An empty (or whitespace-only) `unit` is normalised
    /// to `None`, so a blank field on the command line or in the GUI means
    /// "no relabel" rather than "relabel to nothing".
    pub fn linear(scale: f64, offset: f64, unit: Option<String>) -> Self {
        Self {
            op: Op::Linear { scale, offset },
            unit: unit.map(|u| u.trim().to_string()).filter(|u| !u.is_empty()),
        }
    }

    /// Rules for `--scale` and the GUI's Scale field: finite and non-zero.
    ///
    /// Returns the value so a caller can chain it; the rejection carries only
    /// the reason, because each binary words it in its own voice.
    pub fn check_scale(scale: f64) -> Result<f64, FactorError> {
        Self::check_offset(scale)?;
        // Catches `-0` too: it compares equal to zero and flattens the
        // reading just as thoroughly.
        if scale == 0.0 {
            return Err(FactorError::ZeroScale);
        }
        Ok(scale)
    }

    /// Rules for `--offset` and the GUI's Offset field: finite.
    ///
    /// Any finite shift is meaningful — zero and negatives included — so only
    /// NaN and infinity are rejected.
    pub fn check_offset(offset: f64) -> Result<f64, FactorError> {
        if !offset.is_finite() {
            return Err(FactorError::NotFinite);
        }
        Ok(offset)
    }

    /// Whether this transform would leave a reading exactly as it is.
    pub fn is_identity(&self) -> bool {
        match self.op {
            Op::Linear { scale, offset } => scale == 1.0 && offset == 0.0 && self.unit.is_none(),
        }
    }

    /// How many sub-values [`apply`](Self::apply) appends to a reading.
    ///
    /// Consumers size their series bookkeeping from this before the first
    /// reading arrives, so it must agree with what `apply` actually pushes.
    pub fn extra_aux_count(&self) -> usize {
        usize::from(!self.is_identity())
    }

    /// Apply the transform in place.
    ///
    /// Leaves the reading byte-for-byte untouched (and appends nothing) when
    /// the transform is the identity, so a consumer can call this
    /// unconditionally on every reading.
    ///
    /// Mode, range, flags, progress, specs, raw payload and timestamp all
    /// describe what the meter did and are left alone.
    pub fn apply(&self, m: &mut Measurement) {
        if self.is_identity() {
            return;
        }
        let (scale, offset) = match self.op {
            Op::Linear { scale, offset } => (scale, offset),
        };

        let raw_value = m.value.clone();
        let raw_unit = m.unit.clone();
        let raw_display = m.display_raw.take();
        let (base, mult) = si_prefix(&raw_unit);

        match &raw_value {
            MeasuredValue::Normal(x) => {
                let (value, display) = scaled(*x, mult, scale, offset, raw_display.as_deref());
                m.value = MeasuredValue::Normal(value);
                m.display_raw = Some(display);
            }
            // Overload and NCV have no digits to scale (`display_raw` is only
            // meaningful for `Normal`), so the reading passes through with its
            // own display string restored. The Raw sub-value is still appended
            // below: a sub-value count that stays constant across an overload
            // keeps the GUI's series bookkeeping and the CSV's aux columns
            // stable.
            MeasuredValue::Overload | MeasuredValue::NcvLevel(_) => {
                m.display_raw = raw_display.clone();
            }
        }

        // One small allocation per transformed reading. `unit` is a
        // `Cow<'static, str>` and neither candidate is `'static`: the relabel
        // is a `String` owned by this `Transform`, and `base` borrows the
        // reading's own unit, which may itself be a `Cow::Owned`. Transforms
        // are opt-in, so the untransformed hot path still allocates nothing.
        let out_unit = self.unit.clone().unwrap_or_else(|| base.to_string());

        // A sub-value measuring the same quantity as the main reading is part
        // of the same physical series — a second thermocouple, a REL
        // reference, a MIN/MAX extreme — so the sensor the transform describes
        // sits between the meter and all of them. Scaling only the main
        // reading would leave them in the meter's units, silently mixing volts
        // into an amps trace. Sub-values of another quantity (a frequency
        // beside a voltage) are not the transform's business and pass through.
        //
        // Runs before the Raw sub-value is appended, so Raw always carries the
        // meter's own reading untouched.
        for aux in &mut m.aux_values {
            // Empty means "same unit as the main reading" (`AuxValue::unit_or`),
            // and otherwise the base units have to match: "mV" beside a "V"
            // reading is the same quantity, "Hz" or "ms" is not.
            let (aux_base, aux_mult) = if aux.unit.is_empty() {
                (base, mult)
            } else {
                si_prefix(&aux.unit)
            };
            if aux_base != base {
                continue;
            }
            // Overload and NCV have no digits to scale, exactly as on the main
            // path; they still take the output unit so the pair stays readable.
            if let MeasuredValue::Normal(x) = aux.value {
                let (value, display) =
                    scaled(x, aux_mult, scale, offset, aux.display_raw.as_deref());
                aux.value = MeasuredValue::Normal(value);
                aux.display_raw = Some(display);
            }
            // An empty unit is left empty so the sub-value keeps inheriting
            // the main reading's — now transformed — unit.
            if !aux.unit.is_empty() {
                aux.unit = Cow::Owned(out_unit.clone());
            }
        }

        m.unit = Cow::Owned(out_unit);

        // After any sub-values the meter sent, so the meter's own secondary
        // displays keep the positions (and CSV columns) they always had.
        m.aux_values.push(AuxValue {
            label: Cow::Borrowed(RAW_LABEL),
            value: raw_value,
            unit: raw_unit,
            display_raw: raw_display,
            elapsed_secs: None,
        });
    }

    /// One-line description for stderr banners and toasts: `"×100 → A"`,
    /// `"×1.8 + 32 → °F"`, `"×2"`.
    pub fn describe(&self) -> String {
        let mut out = match self.op {
            Op::Linear { scale, offset } => {
                let mut s = format!("×{scale}");
                if offset > 0.0 {
                    s.push_str(&format!(" + {offset}"));
                } else if offset < 0.0 {
                    s.push_str(&format!(" - {}", -offset));
                }
                s
            }
        };
        if let Some(unit) = &self.unit {
            out.push_str(&format!(" → {unit}"));
        }
        out
    }
}

/// The linear arithmetic for one value, and the digits it is shown with.
///
/// `x` is a reading in a unit whose multiplier to its base is `mult`;
/// `raw_display` is the meter's own digit string for it. Returns
/// `scale * x_base + offset` and its display string. Shared by the main
/// reading and by the sub-values transformed alongside it, so neither the
/// decimals rule nor the rounding can drift between them.
fn scaled(x: f64, mult: f64, scale: f64, offset: f64, raw_display: Option<&str>) -> (f64, String) {
    let y = scale * (x * mult) + offset;
    let decimals = scaled_decimals(raw_display, x, scale * mult);
    // Re-format rather than reusing the meter's digits: `{y}` on the bare f64
    // would print binary artefacts like "0.5678000000000001" everywhere
    // `display_raw` is None.
    let display = format!("{y:.decimals$}");
    // A transformed reading is its digits, as an untransformed one is: keeping
    // the unrounded `y` would let an offset below the displayed resolution
    // move the statistics and the `--integrate` total while never appearing in
    // the exported column. The parse cannot fail — `{:.N}` always prints a
    // number.
    (display.parse::<f64>().unwrap_or(y), display)
}

/// Decimal places for the transformed value.
///
/// The meter's own digit count sets the input precision; multiplying by `gain`
/// moves the decimal point, so the output needs `gain`'s decade subtracted.
/// That holds the physical resolution steady across an auto-range switch —
/// 123.4 mV and 0.1234 V both print "12.34" through a ×100 clamp factor.
fn scaled_decimals(raw_display: Option<&str>, value: f64, gain: f64) -> usize {
    let raw_decimals = match raw_display {
        Some(raw) => decimals_of(raw),
        // No meter digits: fall back to the float's own rendering, capped so a
        // binary artefact ("0.5678000000000001") can't ask for 16 places.
        None => decimals_of(&format!("{value}")).min(6),
    };
    // A zero or non-finite gain has no decade. Leaving the shift at 0 keeps
    // `log10(0.0)` — negative infinity — out of the subtraction below.
    let shift = if gain.is_finite() && gain != 0.0 {
        gain.abs().log10().floor()
    } else {
        0.0
    };
    (raw_decimals as f64 - shift).clamp(0.0, 9.0) as usize
}

/// Digits after the decimal point in a meter display string.
///
/// Counts digits rather than characters, so the UT61E+'s sign-space form
/// ("- 55.79") and any padding are measured correctly.
fn decimals_of(display: &str) -> usize {
    match display.split_once('.') {
        Some((_, frac)) => frac.chars().filter(char::is_ascii_digit).count(),
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::StatusFlags;

    /// A reading with the meter's digit string spelled out (`test_fixture`
    /// always uses "  5.678").
    fn reading(value: f64, unit: &'static str, display: &str) -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(value), unit, StatusFlags::default());
        m.display_raw = Some(display.to_string());
        m
    }

    fn normal(value: &MeasuredValue) -> f64 {
        match value {
            MeasuredValue::Normal(v) => *v,
            other => panic!("expected a normal reading, got {other:?}"),
        }
    }

    #[test]
    fn identity_leaves_the_reading_untouched() {
        let t = Transform::default();
        assert!(t.is_identity());
        let mut m = reading(123.4, "mV", "123.4");
        t.apply(&mut m);
        assert_eq!(normal(&m.value), 123.4);
        assert_eq!(m.unit, "mV");
        assert_eq!(m.display_raw.as_deref(), Some("123.4"));
        assert!(m.aux_values.is_empty());
    }

    /// A 100 mV/A current clamp: the meter reads millivolts, the user reads
    /// amps, and the millivolts stay visible as the Raw sub-value.
    #[test]
    fn linear_scales_from_the_base_unit() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut m = reading(123.4, "mV", "123.4");
        t.apply(&mut m);

        assert_eq!(normal(&m.value), 12.34);
        assert_eq!(m.display_raw.as_deref(), Some("12.34"));
        assert_eq!(m.unit, "A");

        assert_eq!(m.aux_values.len(), 1);
        let raw = &m.aux_values[0];
        assert_eq!(raw.label, RAW_LABEL);
        assert_eq!(normal(&raw.value), 123.4);
        assert_eq!(raw.unit, "mV");
        assert_eq!(raw.display_raw.as_deref(), Some("123.4"));
    }

    /// The reason the scale is applied in base units: the meter switching
    /// range mid-run must not change the transformed reading.
    #[test]
    fn the_same_transform_survives_an_auto_range_switch() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut millivolts = reading(123.4, "mV", "123.4");
        let mut volts = reading(0.1234, "V", "0.1234");
        t.apply(&mut millivolts);
        t.apply(&mut volts);

        assert_eq!(volts.display_raw.as_deref(), Some("12.34"));
        assert_eq!(millivolts.display_raw, volts.display_raw);
        assert_eq!(millivolts.unit, volts.unit);
    }

    #[test]
    fn a_divider_keeps_the_meter_resolution() {
        let t = Transform::linear(0.1, 0.0, None);
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        t.apply(&mut m);
        assert_eq!(m.display_raw.as_deref(), Some("0.5678"));
        assert_eq!(m.unit, "V");
    }

    /// Without a relabel the unit becomes the *base* unit, because that is
    /// what the arithmetic was done in.
    #[test]
    fn no_relabel_means_the_base_unit() {
        let t = Transform::linear(10.0, 0.0, None);
        let mut m = reading(123.4, "mV", "123.4");
        t.apply(&mut m);
        assert_eq!(m.unit, "V");
        assert_eq!(normal(&m.value), 1.234);
        assert_eq!(m.display_raw.as_deref(), Some("1.234"));
    }

    #[test]
    fn an_offset_converts_celsius_to_fahrenheit() {
        let t = Transform::linear(1.8, 32.0, Some("°F".to_string()));
        let mut m = reading(25.4, "°C", "25.4");
        t.apply(&mut m);
        assert_eq!(m.display_raw.as_deref(), Some("77.7"));
        assert_eq!(m.unit, "°F");
        assert_eq!(m.aux_values[0].unit, "°C");
        assert_eq!(m.aux_values[0].display_raw.as_deref(), Some("25.4"));
    }

    #[test]
    fn a_negative_scale_flips_the_sign() {
        let t = Transform::linear(-1.0, 0.0, None);
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        t.apply(&mut m);
        assert_eq!(normal(&m.value), -5.678);
        assert_eq!(m.display_raw.as_deref(), Some("-5.678"));
    }

    /// A relabel with no arithmetic is still not the identity: the prefix
    /// conversion and the Raw sub-value both happen.
    #[test]
    fn a_relabel_alone_still_converts_and_records_the_raw_reading() {
        let t = Transform::linear(1.0, 0.0, Some("PSI".to_string()));
        assert!(!t.is_identity());
        let mut m = reading(123.4, "mV", "123.4");
        t.apply(&mut m);
        assert_eq!(m.unit, "PSI");
        assert_eq!(normal(&m.value), 0.1234);
        assert_eq!(m.display_raw.as_deref(), Some("0.1234"));
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].unit, "mV");
    }

    /// A blank relabel field means "no relabel", not "relabel to nothing".
    #[test]
    fn an_empty_relabel_is_no_relabel() {
        assert_eq!(Transform::linear(2.0, 0.0, Some(String::new())).unit, None);
        assert_eq!(
            Transform::linear(2.0, 0.0, Some("  ".to_string())).unit,
            None
        );
        assert!(Transform::linear(1.0, 0.0, Some("".to_string())).is_identity());
    }

    #[test]
    fn a_relabel_is_trimmed() {
        assert_eq!(
            Transform::linear(1.8, 32.0, Some(" °F ".to_string())).unit,
            Some("°F".to_string())
        );
    }

    #[test]
    fn overload_passes_through_with_its_raw_sub_value() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut m =
            Measurement::test_fixture(MeasuredValue::Overload, "kΩ", StatusFlags::default());
        let before = m.display_raw.clone();
        t.apply(&mut m);

        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw, before);
        assert_eq!(m.unit, "A");
        assert_eq!(m.value_export_str(), "OL");

        assert_eq!(m.aux_values.len(), 1);
        assert!(matches!(m.aux_values[0].value, MeasuredValue::Overload));
        assert_eq!(m.aux_values[0].unit, "kΩ");
    }

    #[test]
    fn ncv_passes_through_with_its_raw_sub_value() {
        let t = Transform::linear(2.0, 0.0, None);
        let mut m =
            Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        let before = m.display_raw.clone();
        t.apply(&mut m);

        assert!(matches!(m.value, MeasuredValue::NcvLevel(3)));
        assert_eq!(m.display_raw, before);
        assert_eq!(m.unit, "");

        assert_eq!(m.aux_values.len(), 1);
        assert!(matches!(m.aux_values[0].value, MeasuredValue::NcvLevel(3)));
        assert_eq!(m.aux_values[0].unit, "");
    }

    /// The count consumers size their bookkeeping from must match what
    /// `apply` pushes, overload included.
    #[test]
    fn extra_aux_count_matches_what_apply_appends() {
        for t in [
            Transform::default(),
            Transform::linear(100.0, 0.0, Some("A".to_string())),
            Transform::linear(1.0, 0.0, Some("PSI".to_string())),
        ] {
            let mut m = reading(123.4, "mV", "123.4");
            let mut overloaded =
                Measurement::test_fixture(MeasuredValue::Overload, "mV", StatusFlags::default());
            t.apply(&mut m);
            t.apply(&mut overloaded);
            assert_eq!(m.aux_values.len(), t.extra_aux_count(), "{}", t.describe());
            assert_eq!(overloaded.aux_values.len(), t.extra_aux_count());
        }
        assert_eq!(Transform::default().extra_aux_count(), 0);
        assert_eq!(Transform::linear(2.0, 0.0, None).extra_aux_count(), 1);
    }

    /// The meter's own sub-values keep their positions — and therefore their
    /// CSV columns — when a transform is switched on.
    #[test]
    fn the_raw_sub_value_lands_after_the_meter_sub_values() {
        let t = Transform::linear(2.0, 0.0, None);
        let mut m = reading(230.0, "V", "230.0");
        m.aux_values.push(AuxValue {
            label: "Frequency".into(),
            value: MeasuredValue::Normal(50.01),
            unit: "Hz".into(),
            display_raw: Some("50.01".to_string()),
            elapsed_secs: None,
        });
        t.apply(&mut m);

        let labels: Vec<&str> = m.aux_values.iter().map(|a| a.label.as_ref()).collect();
        assert_eq!(labels, ["Frequency", RAW_LABEL]);
    }

    /// A sub-value, as a meter sends one: `unit` empty means "same unit as the
    /// main reading".
    fn aux(label: &'static str, value: f64, unit: &'static str, display: &str) -> AuxValue {
        AuxValue {
            label: Cow::Borrowed(label),
            value: MeasuredValue::Normal(value),
            unit: Cow::Borrowed(unit),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
    }

    /// The UT181A's dual-thermocouple frame: both probes measure the same
    /// quantity, so °C → °F has to move T2 with the main reading.
    #[test]
    fn a_same_unit_sub_value_is_transformed_with_the_reading() {
        let t = Transform::linear(1.8, 32.0, Some("°F".to_string()));
        let mut m = reading(25.4, "°C", "25.4");
        m.aux_values.push(aux("T2", 24.6, "°C", "24.6"));
        t.apply(&mut m);

        assert_eq!(m.display_raw.as_deref(), Some("77.7"));
        assert_eq!(m.unit, "°F");

        assert_eq!(m.aux_values.len(), 2);
        let t2 = &m.aux_values[0];
        assert_eq!(t2.label, "T2");
        assert_eq!(normal(&t2.value), 76.3);
        assert_eq!(t2.display_raw.as_deref(), Some("76.3"));
        assert_eq!(t2.unit, "°F");

        // The meter's own reading, untransformed, still last.
        let raw = &m.aux_values[1];
        assert_eq!(raw.label, RAW_LABEL);
        assert_eq!(normal(&raw.value), 25.4);
        assert_eq!(raw.unit, "°C");
    }

    /// MIN/MAX extremes come with an empty unit — "same as the main reading" —
    /// so they scale with it and keep inheriting its (now transformed) unit.
    #[test]
    fn empty_unit_sub_values_scale_and_stay_unitless() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut m = reading(123.4, "mV", "123.4");
        let mut max = aux("Max", 130.0, "", "130.0");
        max.elapsed_secs = Some(12);
        m.aux_values.push(max);
        m.aux_values.push(aux("Average", 120.0, "", "120.0"));
        m.aux_values.push(aux("Min", 110.0, "", "110.0"));
        t.apply(&mut m);

        assert_eq!(m.display_raw.as_deref(), Some("12.34"));
        let labels: Vec<&str> = m.aux_values.iter().map(|a| a.label.as_ref()).collect();
        assert_eq!(labels, ["Max", "Average", "Min", RAW_LABEL]);

        let max = &m.aux_values[0];
        assert_eq!(normal(&max.value), 13.0);
        assert_eq!(max.display_raw.as_deref(), Some("13.00"));
        assert_eq!(max.unit, "");
        assert_eq!(max.unit_or(&m.unit), "A");
        assert_eq!(max.elapsed_secs, Some(12));

        assert_eq!(m.aux_values[1].display_raw.as_deref(), Some("12.00"));
        assert_eq!(m.aux_values[2].display_raw.as_deref(), Some("11.00"));

        let raw = &m.aux_values[3];
        assert_eq!(raw.label, RAW_LABEL);
        assert_eq!(raw.unit, "mV");
        assert_eq!(raw.display_raw.as_deref(), Some("123.4"));
    }

    /// A frequency beside a voltage is another quantity: the transform
    /// describes the sensor in front of the volts, not the hertz.
    #[test]
    fn sub_values_of_another_quantity_are_left_alone() {
        let t = Transform::linear(2.0, 0.0, None);
        let mut m = reading(230.0, "V", "230.0");
        m.aux_values.push(aux("Frequency", 50.01, "Hz", "50.01"));
        m.aux_values.push(aux("Period", 20.0, "ms", "20.00"));
        let before = m.aux_values.clone();
        t.apply(&mut m);

        assert_eq!(m.aux_values.len(), 3);
        for (after, before) in m.aux_values.iter().zip(&before) {
            assert_eq!(after.label, before.label);
            assert_eq!(normal(&after.value), normal(&before.value));
            assert_eq!(after.unit, before.unit);
            assert_eq!(after.display_raw, before.display_raw);
        }
        assert_eq!(m.aux_values[2].label, RAW_LABEL);
    }

    /// Same base unit, different prefix: the sub-value converts through its
    /// own prefix, not the main reading's.
    #[test]
    fn a_prefixed_sub_value_converts_through_its_own_prefix() {
        let t = Transform::linear(10.0, 0.0, None);
        let mut m = reading(1.234, "V", "1.234");
        m.aux_values.push(aux("Reference", 500.0, "mV", "500.0"));
        t.apply(&mut m);

        assert_eq!(m.unit, "V");
        assert_eq!(m.display_raw.as_deref(), Some("12.34"));

        let reference = &m.aux_values[0];
        assert_eq!(normal(&reference.value), 5.0);
        // 0.5 V × 10, at the sub-value's own resolution: one decimal on a
        // millivolt display is 0.1 mV, and ×10 in volts leaves three places.
        assert_eq!(reference.display_raw.as_deref(), Some("5.000"));
        assert_eq!(reference.unit, "V");
        assert_eq!(m.aux_values[1].label, RAW_LABEL);
    }

    /// An overloaded sub-value has no digits to scale, exactly as an
    /// overloaded main reading has none.
    #[test]
    fn an_overloaded_sub_value_passes_through() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut m = reading(123.4, "mV", "123.4");
        m.aux_values.push(AuxValue {
            label: Cow::Borrowed("Max"),
            value: MeasuredValue::Overload,
            unit: Cow::Borrowed(""),
            display_raw: Some("OL".to_string()),
            elapsed_secs: None,
        });
        t.apply(&mut m);

        let max = &m.aux_values[0];
        assert!(matches!(max.value, MeasuredValue::Overload));
        assert_eq!(max.display_raw.as_deref(), Some("OL"));
        assert_eq!(max.unit, "");
        assert_eq!(max.unit_or(&m.unit), "A");
    }

    #[test]
    fn identity_leaves_sub_values_untouched() {
        let t = Transform::default();
        let mut m = reading(25.4, "°C", "25.4");
        m.aux_values.push(aux("T2", 24.6, "°C", "24.6"));
        m.aux_values.push(aux("Max", 26.0, "", "26.0"));
        let before = m.aux_values.clone();
        t.apply(&mut m);

        assert_eq!(m.aux_values.len(), before.len());
        for (after, before) in m.aux_values.iter().zip(&before) {
            assert_eq!(normal(&after.value), normal(&before.value));
            assert_eq!(after.unit, before.unit);
            assert_eq!(after.display_raw, before.display_raw);
        }
    }

    /// Transforming the meter's sub-values must not append any: the count
    /// consumers size their series bookkeeping from is still one.
    #[test]
    fn extra_aux_count_is_one_with_meter_sub_values_present() {
        let t = Transform::linear(1.8, 32.0, Some("°F".to_string()));
        let mut m = reading(25.4, "°C", "25.4");
        m.aux_values.push(aux("T2", 24.6, "°C", "24.6"));
        t.apply(&mut m);
        assert_eq!(t.extra_aux_count(), 1);
        assert_eq!(m.aux_values.len(), 1 + t.extra_aux_count());
    }

    #[test]
    fn the_transformed_value_exports_as_a_number() {
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        let mut m = reading(123.4, "mV", "123.4");
        t.apply(&mut m);
        assert_eq!(m.value_export_str().parse::<f64>().unwrap(), 12.34);
    }

    /// The stats, the graph and `--integrate` read the f64 while the CSV
    /// column shows the digits: they have to be the same number.
    #[test]
    fn the_stored_value_is_exactly_the_exported_digits() {
        for (t, mut m) in [
            (
                Transform::linear(100.0, 0.0, Some("A".to_string())),
                reading(123.4, "mV", "123.4"),
            ),
            (
                Transform::linear(0.3333, 0.0, None),
                reading(5.678, "V", "5.678"),
            ),
            (
                Transform::linear(1.8, 32.0, Some("°F".to_string())),
                reading(25.4, "°C", "25.4"),
            ),
            (
                Transform::linear(1.0, 0.007, None),
                reading(-55.79, "V", "- 55.79"),
            ),
        ] {
            t.apply(&mut m);
            let exported: f64 = m.value_export_str().parse().unwrap();
            assert_eq!(exported, normal(&m.value), "{}", t.describe());
        }
    }

    /// An offset below the displayed resolution changes neither the digits nor
    /// the stored value — otherwise it would shift a summary that no exported
    /// sample accounts for.
    #[test]
    fn an_offset_under_the_resolution_moves_nothing() {
        let t = Transform::linear(1.0, 0.00001, None);
        let mut m = reading(1.2345, "V", "1.2345");
        t.apply(&mut m);
        assert_eq!(m.display_raw.as_deref(), Some("1.2345"));
        assert_eq!(normal(&m.value), 1.2345);
    }

    /// The UT61E+ separates the sign from the digits on some ranges; the
    /// decimal count has to survive that.
    #[test]
    fn decimals_come_from_a_sign_spaced_display_string() {
        let t = Transform::linear(0.1, 0.0, None);
        let mut m = reading(-55.79, "V", "- 55.79");
        t.apply(&mut m);
        // Two decimals on the wire, ×0.1 shifts the point one place right.
        assert_eq!(m.display_raw.as_deref(), Some("-5.579"));
        assert_eq!(m.value_export_str().parse::<f64>().unwrap(), -5.579);
    }

    /// Float-based families leave `display_raw` unset on some frames; the
    /// fallback must not print the f64's binary artefact.
    #[test]
    fn a_missing_display_string_falls_back_to_the_float() {
        let t = Transform::linear(0.1, 0.0, None);
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        m.display_raw = None;
        t.apply(&mut m);
        assert_eq!(m.display_raw.as_deref(), Some("0.5678"));
    }

    #[test]
    fn describe_reads_as_the_formula() {
        assert_eq!(
            Transform::linear(100.0, 0.0, Some("A".to_string())).describe(),
            "×100 → A"
        );
        assert_eq!(
            Transform::linear(1.8, 32.0, Some("°F".to_string())).describe(),
            "×1.8 + 32 → °F"
        );
        assert_eq!(
            Transform::linear(10.0, 0.0, Some("V".to_string())).describe(),
            "×10 → V"
        );
        assert_eq!(Transform::linear(2.0, 0.0, None).describe(), "×2");
        assert_eq!(Transform::linear(1.0, -0.5, None).describe(), "×1 - 0.5");
    }

    #[test]
    fn si_prefix_splits_the_units_the_parsers_emit() {
        for (unit, base, mult) in [
            ("V", "V", 1.0),
            ("mV", "V", 1e-3),
            ("A", "A", 1.0),
            ("mA", "A", 1e-3),
            ("µA", "A", 1e-6),
            ("uA", "A", 1e-6),
            ("Ω", "Ω", 1.0),
            ("kΩ", "Ω", 1e3),
            ("MΩ", "Ω", 1e6),
            ("F", "F", 1.0),
            ("nF", "F", 1e-9),
            ("µF", "F", 1e-6),
            ("mF", "F", 1e-3),
            ("Hz", "Hz", 1.0),
            ("kHz", "Hz", 1e3),
            ("MHz", "Hz", 1e6),
            ("nS", "S", 1e-9),
            ("VAC", "VAC", 1.0),
            ("VDC", "VDC", 1.0),
        ] {
            assert_eq!(si_prefix(unit), (base, mult), "unit {unit}");
        }
    }

    /// Units that merely start with a prefix letter, or carry none at all,
    /// must pass through: "ms" is milliseconds, not milli-seconds.
    #[test]
    fn si_prefix_leaves_non_prefixed_units_alone() {
        for unit in ["ms", "°C", "°F", "%", "dBm", "dBV", "", "m", "M", "µ"] {
            assert_eq!(si_prefix(unit), (unit, 1.0), "unit {unit}");
        }
    }

    /// A zero scale flattens every reading onto the offset, and NaN or
    /// infinity poisons the stats and the integral. Both binaries reject the
    /// same values, so the rule lives here rather than in either of them.
    #[test]
    fn check_scale_rejects_zero_and_non_finite_factors() {
        assert_eq!(Transform::check_scale(0.0), Err(FactorError::ZeroScale));
        assert_eq!(Transform::check_scale(-0.0), Err(FactorError::ZeroScale));
        assert_eq!(
            Transform::check_scale(f64::NAN),
            Err(FactorError::NotFinite)
        );
        assert_eq!(
            Transform::check_scale(f64::INFINITY),
            Err(FactorError::NotFinite)
        );
        assert_eq!(
            Transform::check_scale(f64::NEG_INFINITY),
            Err(FactorError::NotFinite)
        );
        // A negative scale is a legitimate probe-polarity flip.
        assert_eq!(Transform::check_scale(-2.5), Ok(-2.5));
        assert_eq!(Transform::check_scale(1e-9), Ok(1e-9));
    }

    /// Zero is a meaningful offset — it is what "no offset" means — so only
    /// NaN and infinity are rejected.
    #[test]
    fn check_offset_rejects_only_non_finite_values() {
        assert_eq!(Transform::check_offset(0.0), Ok(0.0));
        assert_eq!(Transform::check_offset(-3.0), Ok(-3.0));
        assert_eq!(
            Transform::check_offset(f64::NAN),
            Err(FactorError::NotFinite)
        );
        assert_eq!(
            Transform::check_offset(f64::INFINITY),
            Err(FactorError::NotFinite)
        );
    }
}
