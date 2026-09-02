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

/// Format a measured value as a string that reads back as a number.
///
/// Shared by [`Measurement::value_export_str`] and [`AuxValue::value_str`] so
/// the main reading and its sub-values cannot drift apart: the parsed value
/// decides OL/NCV first, the meter's own digits are preferred over the
/// re-formatted float, and any space the meter puts between the sign and the
/// digits is removed.
fn export_value_str<'a>(value: &'a MeasuredValue, display_raw: Option<&'a str>) -> Cow<'a, str> {
    match value {
        MeasuredValue::Normal(v) => match display_raw {
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

/// Render sub-values as one `", "`-joined line, each entry reading
/// `"{label} {value}"` plus `" {unit}"` when the sub-value has a unit
/// (unitless modes like NCV would otherwise trail a space) and `" @{n}s"`
/// when it carries a mode-elapsed timestamp.
///
/// Takes already-resolved strings rather than [`AuxValue`]s so the same line
/// can be built from a capture report loaded off disk, where the sub-values
/// are plain strings and the originating [`Measurement`] is long gone. See
/// [`Measurement::aux_summary`] for the common case.
pub fn aux_summary_line<'a, V: AsRef<str>>(
    entries: impl IntoIterator<Item = (&'a str, V, &'a str, Option<u32>)>,
) -> String {
    let mut out = String::new();
    for (label, value, unit, elapsed_secs) in entries {
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(label);
        out.push(' ');
        out.push_str(value.as_ref());
        if !unit.is_empty() {
            out.push(' ');
            out.push_str(unit);
        }
        if let Some(secs) = elapsed_secs {
            out.push_str(&format!(" @{secs}s"));
        }
    }
    out
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

/// The columns one sub-value slot contributes to a tabular (CSV) export, as
/// header suffixes: `aux1_label,aux1_value,aux1_unit`.
///
/// The CLI and the GUI write the same file format from two different
/// functions. Both name their columns from this list and fill their rows from
/// [`AuxValue::export_cells`], whose array is exactly this long, so a column
/// cannot be added to one side's header without the other side's rows failing
/// to compile.
pub const AUX_EXPORT_COLUMNS: [&str; 3] = ["label", "value", "unit"];

impl AuxValue {
    /// The sub-value formatted for display and export.
    ///
    /// Applies the same rules as [`Measurement::value_export_str`], through
    /// the same helper: the parsed value decides first, so an overloaded
    /// sub-value reads "OL" rather than whatever digits happened to be in
    /// `display_raw`; the meter's own digits are preferred over the parsed
    /// float; and spaces are stripped — including one between the sign and
    /// the digits — so the string reads back as a number in the CSV column
    /// the sub-value is exported to.
    pub fn value_str(&self) -> Cow<'_, str> {
        export_value_str(&self.value, self.display_raw.as_deref())
    }

    /// The unit to show for this sub-value, given the main reading's unit.
    ///
    /// Protocols leave `unit` empty when the sub-value shares the main
    /// reading's unit — a relative-mode reference or a min/max sample always
    /// measures the same quantity as the value it tracks, so the wire never
    /// repeats the unit. Only genuinely different units (the UT181A's
    /// "Hz"/"ms" frequency pair, a second thermocouple) are spelled out.
    pub fn unit_or<'a>(&'a self, main_unit: &'a str) -> &'a str {
        if self.unit.is_empty() {
            main_unit
        } else {
            &self.unit
        }
    }

    /// The cells one exported slot writes, in [`AUX_EXPORT_COLUMNS`] order.
    ///
    /// `main_unit` is the parent reading's unit, used when the sub-value
    /// leaves its own empty (see [`AuxValue::unit_or`]).
    pub fn export_cells<'a>(
        &'a self,
        main_unit: &'a str,
    ) -> [Cow<'a, str>; AUX_EXPORT_COLUMNS.len()] {
        [
            Cow::Borrowed(self.label.as_ref()),
            self.value_str(),
            Cow::Borrowed(self.unit_or(main_unit)),
        ]
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
        export_value_str(&self.value, self.display_raw.as_deref())
    }

    /// The sub-values as one line, for status bars and log messages.
    ///
    /// Each entry reads `"{label} {value} {unit}"`, with the main reading's
    /// unit substituted where the sub-value doesn't carry its own (see
    /// [`AuxValue::unit_or`]), and `" @{n}s"` appended when the sub-value
    /// carries a timestamp. Entries are joined by `", "`:
    ///
    /// - `"Frequency 50.01 Hz, Period 20.00 ms"`
    /// - `"Max 5.0123 V @12s, Average 4.9902 V @12s, Min 4.9654 V @3s"`
    ///
    /// Empty when the measurement has no sub-values, so callers can print it
    /// unconditionally and get nothing for single-display meters.
    pub fn aux_summary(&self) -> String {
        aux_summary_line(self.aux_values.iter().map(|aux| {
            (
                aux.label.as_ref(),
                aux.value_str(),
                aux.unit_or(&self.unit),
                aux.elapsed_secs,
            )
        }))
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

    /// Sub-values land in numeric CSV columns of their own, so `value_str`
    /// has to strip the sign space exactly like the main value does
    /// (see `export_str_strips_the_sign_space`).
    #[test]
    fn aux_value_str_strips_the_sign_space() {
        let mut a = aux("Reference", "- 5.000", "");
        // The helper's parse can't read the spaced form; the wire value is
        // parsed by the protocol, not from `display_raw`.
        a.value = MeasuredValue::Normal(-5.0);
        assert_eq!(a.value_str(), "-5.000");
        assert_eq!(a.value_str().parse::<f64>().unwrap(), -5.0);
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

    fn aux(label: &'static str, display: &str, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(display.trim().parse().unwrap_or(0.0)),
            unit: unit.into(),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
    }

    #[test]
    fn aux_unit_falls_back_to_the_main_unit() {
        // Empty on the wire means "same unit as the main reading".
        assert_eq!(aux("Reference", "5.0000", "").unit_or("V"), "V");
    }

    #[test]
    fn aux_unit_keeps_its_own_when_set() {
        assert_eq!(aux("Period", "20.00", "ms").unit_or("V"), "ms");
    }

    #[test]
    fn aux_summary_is_empty_without_sub_values() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        assert_eq!(m.aux_summary(), "");
    }

    #[test]
    fn aux_summary_joins_labelled_sub_values() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(230.0), "V", StatusFlags::default());
        m.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];
        assert_eq!(m.aux_summary(), "Frequency 50.01 Hz, Period 20.00 ms");
    }

    #[test]
    fn aux_summary_uses_the_main_unit_and_timestamps() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.0), "V", StatusFlags::default());
        let mut max = aux("Max", "5.0123", "");
        max.elapsed_secs = Some(12);
        let mut avg = aux("Average", "4.9902", "");
        avg.elapsed_secs = Some(12);
        let mut min = aux("Min", "4.9654", "");
        min.elapsed_secs = Some(3);
        m.aux_values = vec![max, avg, min];
        assert_eq!(
            m.aux_summary(),
            "Max 5.0123 V @12s, Average 4.9902 V @12s, Min 4.9654 V @3s"
        );
    }

    #[test]
    fn aux_summary_reads_overload_from_the_value() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let mut over = aux("Max", "9.9999", "");
        over.value = MeasuredValue::Overload;
        m.aux_values = vec![over];
        assert_eq!(m.aux_summary(), "Max OL V");
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
