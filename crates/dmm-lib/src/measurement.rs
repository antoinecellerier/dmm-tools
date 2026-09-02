use crate::flags::StatusFlags;
use crate::specs::{ModeSpecInfo, SpecInfo};
use std::borrow::Cow;
use std::time::Instant;

/// Represents a parsed measurement value.
#[derive(Debug, Clone)]
pub enum MeasuredValue {
    /// A normal numeric reading.
    Normal(f64),
    /// The meter is showing OL (overload).
    Overload,
    /// NCV (non-contact voltage) detection level (0-4 typically).
    NcvLevel(u8),
}

/// An auxiliary value associated with a measurement.
///
/// Used by protocols that report multiple related values per reading:
/// UT181A secondary displays (a second thermocouple, a frequency and its
/// period), relative mode (delta/reference/absolute), min/max mode
/// (current/max/avg/min with timestamps), peak mode (max/min).
#[derive(Debug, Clone)]
pub struct AuxValue {
    /// Human-readable label (e.g. "Reference", "Max", "Peak Min").
    pub label: Cow<'static, str>,
    /// The numeric value (or overload).
    pub value: MeasuredValue,
    /// Unit string. Empty if same as main measurement unit.
    pub unit: Cow<'static, str>,
    /// Formatted display string (like `Measurement::display_raw`).
    pub display_raw: Option<String>,
    /// Elapsed seconds from mode start (min/max timestamps).
    pub elapsed_secs: Option<u32>,
}

impl AuxValue {
    /// The sub-value formatted for display and export.
    ///
    /// Mirrors [`Measurement::value_export_str`]: the parsed value decides
    /// first, so an overloaded sub-value reads "OL" rather than whatever
    /// digits happened to be in `display_raw`.
    pub fn value_str(&self) -> Cow<'_, str> {
        match &self.value {
            MeasuredValue::Normal(v) => match self.display_raw.as_deref() {
                Some(raw) => Cow::Borrowed(raw.trim()),
                None => Cow::Owned(v.to_string()),
            },
            MeasuredValue::Overload => Cow::Borrowed("OL"),
            MeasuredValue::NcvLevel(level) => Cow::Owned(format!("NCV:{level}")),
        }
    }
}

/// A fully parsed measurement from the meter.
///
/// This is the unified measurement type used by all protocol implementations.
/// String-based mode/unit fields allow each protocol to produce human-readable
/// values without sharing a common mode enum.
///
/// The `mode`, `unit`, and `range_label` fields use `Cow<'static, str>` to
/// avoid per-measurement heap allocations. Most values come from static lookup
/// tables (`Cow::Borrowed`); only fallback paths like `format!("Unknown(0x{:02x})")`
/// produce owned strings (`Cow::Owned`).
#[derive(Debug, Clone)]
pub struct Measurement {
    pub timestamp: Instant,
    /// Human-readable mode string (e.g. "DC V", "AC mV", "Unknown(0x05)").
    pub mode: Cow<'static, str>,
    /// Raw protocol-level mode value (for debugging and spec lookup).
    pub mode_raw: u16,
    /// Raw protocol-level range byte (for spec lookup).
    pub range_raw: u8,
    pub value: MeasuredValue,
    /// Unit string (e.g. "V", "mV", "kΩ", "nS").
    pub unit: Cow<'static, str>,
    /// Range label (e.g. "22V", "220mV", "" if not applicable).
    pub range_label: Cow<'static, str>,
    /// Bar graph progress value, None if the protocol doesn't provide it.
    pub progress: Option<u16>,
    /// The digits to show for this reading, in the meter's own precision.
    ///
    /// Character-display meters (UT61E+, UT8802, …) put the ASCII field
    /// straight from the wire here. Float-based meters (UT171, UT181A) format
    /// their f32 into it, because the widened f64 in `value` prints its
    /// binary-to-decimal artefact ("12.345000267028809") when formatted
    /// directly. `None` means no display string is available — formatters then
    /// fall back to `value`.
    ///
    /// Only meaningful for `MeasuredValue::Normal`: overload and NCV have no
    /// digits, and consumers must decide from `value`, not from this field.
    pub display_raw: Option<String>,
    pub flags: StatusFlags,
    /// Auxiliary values (e.g. relative reference/absolute, min/max/avg sub-values).
    /// Empty for normal single-value measurements.
    pub aux_values: Vec<AuxValue>,
    /// Raw payload bytes as received (for protocol debugging).
    pub raw_payload: Vec<u8>,
    /// Per-range resolution/accuracy for this mode+range, if the protocol provides specs.
    /// Populated by `Dmm::request_measurement` — protocols that don't override
    /// `Protocol::spec_info` leave this as `None`.
    pub spec: Option<&'static SpecInfo>,
    /// Per-mode input impedance and notes, if the protocol provides specs.
    pub mode_spec: Option<&'static ModeSpecInfo>,
}

impl Measurement {
    /// The measured value formatted for machine-readable export (CSV).
    ///
    /// Prefers the meter's own display digits so the exported precision
    /// matches what the meter showed ("5.0000", not "5"), but strips the
    /// spaces some protocols place between the sign and the digits — the
    /// UT61E+ sends `"- 55.79"` on some ranges, which would make the whole
    /// column parse as text. The result is the same string the protocol
    /// parsed the value from, so it always reads back as a number.
    ///
    /// For display use `to_string()` / `display_raw` instead: those keep the
    /// meter's padding, which holds the on-screen width steady.
    pub fn value_export_str(&self) -> Cow<'_, str> {
        match &self.value {
            MeasuredValue::Normal(v) => match self.display_raw.as_deref() {
                Some(raw) => {
                    let trimmed = raw.trim();
                    if trimmed.contains(' ') {
                        Cow::Owned(trimmed.chars().filter(|c| *c != ' ').collect())
                    } else {
                        Cow::Borrowed(trimmed)
                    }
                }
                None => Cow::Owned(v.to_string()),
            },
            MeasuredValue::Overload => Cow::Borrowed("OL"),
            MeasuredValue::NcvLevel(level) => Cow::Owned(format!("NCV:{level}")),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Measurement {
    /// Create a `Measurement` with sensible defaults for testing.
    ///
    /// Only `value`, `unit`, and `flags` are caller-specified; all other fields
    /// get safe dummy values (mode="DC V", range_label="22V", etc.).
    pub fn test_fixture(
        value: MeasuredValue,
        unit: &'static str,
        flags: StatusFlags,
    ) -> Measurement {
        Measurement {
            timestamp: Instant::now(),
            mode: "DC V".into(),
            mode_raw: 0x02,
            range_raw: 1,
            value,
            unit: unit.into(),
            range_label: "22V".into(),
            progress: Some(0),
            display_raw: Some("  5.678".to_string()),
            flags,
            aux_values: vec![],
            raw_payload: vec![],
            spec: None,
            mode_spec: None,
        }
    }
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value_str = match &self.value {
            MeasuredValue::Normal(_) => self
                .display_raw
                .as_deref()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| match &self.value {
                    MeasuredValue::Normal(v) => format!("{v}"),
                    _ => unreachable!(),
                }),
            MeasuredValue::Overload => "OL".to_string(),
            MeasuredValue::NcvLevel(level) => format!("NCV:{level}"),
        };
        write!(f, "{value_str} {}", self.unit)?;
        let flags_str = self.flags.to_string();
        if !flags_str.is_empty() {
            write!(f, " [{flags_str}]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_normal() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
    }

    #[test]
    fn display_overload() {
        let m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        assert!(m.to_string().contains("OL"));
    }

    #[test]
    fn export_str_keeps_the_meter_digits() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        // test_fixture's display_raw is "  5.678" — padding only.
        assert_eq!(m.value_export_str(), "5.678");
    }

    /// The UT61E+ sends the sign and digits separated by a space on some
    /// ranges; leaving it in makes the CSV `value` column non-numeric.
    #[test]
    fn export_str_strips_the_sign_space() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(-55.79), "V", StatusFlags::default());
        m.display_raw = Some("- 55.79".to_string());
        assert_eq!(m.value_export_str(), "-55.79");
        assert_eq!(m.value_export_str().parse::<f64>().unwrap(), -55.79);
    }

    #[test]
    fn export_str_falls_back_to_the_parsed_value() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.25), "V", StatusFlags::default());
        m.display_raw = None;
        assert_eq!(m.value_export_str(), "1.25");
    }

    #[test]
    fn export_str_labels_overload_and_ncv() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        assert_eq!(m.value_export_str(), "OL");
        m.value = MeasuredValue::NcvLevel(3);
        assert_eq!(m.value_export_str(), "NCV:3");
    }

    #[test]
    fn display_ncv() {
        let m = Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        assert!(m.to_string().contains("NCV:3"));
    }

    #[test]
    fn display_with_flags() {
        let flags = StatusFlags {
            hold: true,
            auto_range: true,
            ..Default::default()
        };
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", flags);
        let s = m.to_string();
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }
}
