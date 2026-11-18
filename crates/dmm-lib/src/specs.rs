//! Shared specification metadata types.
//!
//! These describe resolution, accuracy, and per-mode notes for a measurement.
//! Any protocol family can provide specs by implementing the optional
//! `Protocol::spec_info` / `Protocol::mode_spec_info` methods.

/// Accuracy for a specific frequency band (or DC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccuracyBand {
    /// Frequency range label, or `None` for DC / single-band modes.
    pub freq_range: Option<&'static str>,
    /// Accuracy string without leading `±` (e.g. "0.1%+5").
    pub accuracy: &'static str,
}

/// Per-range specification data (resolution and accuracy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecInfo {
    /// Display resolution (e.g. "0.01mV", "1Ω"); empty where the manual
    /// prints none for the range.
    pub resolution: &'static str,
    /// Accuracy bands: 1 for DC, 2-3 for AC with multiple frequency ranges.
    pub accuracy: &'static [AccuracyBand],
}

/// Per-mode specification data shared across all ranges.
#[derive(Debug, Clone)]
pub struct ModeSpecInfo {
    /// Input impedance (e.g. "~10 MΩ"), if applicable.
    pub input_impedance: Option<&'static str>,
    /// Overload protection description.
    pub overload_protection: Option<&'static str>,
    /// Additional notes (e.g. "True RMS", "K-type thermocouple").
    pub notes: &'static [&'static str],
}

/// One table of a model's spec sheet, for reviewing the spec data against
/// the manual (`Protocol::spec_sheet`).
#[derive(Debug, Clone)]
pub struct SpecSheetTable {
    /// The manual's table title.
    pub name: &'static str,
    /// The manual's PDF page (not the printed page number).
    pub page: u16,
    /// Input impedance, overload protection and notes.
    pub mode: &'static ModeSpecInfo,
    /// The ranges, in the order the sheet lists them.
    pub rows: Vec<SpecSheetRow>,
}

/// One range of a [`SpecSheetTable`].
#[derive(Debug, Clone)]
pub struct SpecSheetRow {
    /// The range as the manual labels it.
    pub label: &'static str,
    /// The range byte the row is for (or, where the frames carry no range
    /// byte, the family's stand-in), `None` where any range byte is.
    pub range_raw: Option<u8>,
    pub spec: &'static SpecInfo,
}

/// One row of a manual spec table, keyed by the range byte it answers.
#[derive(Debug)]
pub(crate) struct RangeSpec {
    /// The range byte (or, where the frames carry no range byte, the
    /// family's stand-in), `None` for a mode with one range whose byte varies
    /// on the wire (or that has no reading at all).
    pub(crate) range: Option<u8>,
    /// The range as the manual labels it.
    pub(crate) label: &'static str,
    pub(crate) spec: SpecInfo,
}

/// A manual spec table, or the part of one that a set of readings shares:
/// a table whose rows differ in input impedance or overload protection, or
/// that spans several modes, is split into parts of the same name.
#[derive(Debug)]
pub(crate) struct ModeSpecs {
    /// The manual's table title.
    pub(crate) name: &'static str,
    /// The manual's PDF page (not the printed page number).
    pub(crate) page: u16,
    pub(crate) ranges: &'static [RangeSpec],
    pub(crate) mode: ModeSpecInfo,
}

impl ModeSpecs {
    /// The row for range byte `range`.
    pub(crate) fn row(&self, range: u8) -> Option<&RangeSpec> {
        self.ranges
            .iter()
            .find(|r| r.range.is_none_or(|b| b == range))
    }

    /// This table as `Protocol::spec_sheet` lists it.
    pub(crate) fn sheet_table(&'static self) -> SpecSheetTable {
        SpecSheetTable {
            name: self.name,
            page: self.page,
            mode: &self.mode,
            rows: self
                .ranges
                .iter()
                .map(|r| SpecSheetRow {
                    label: r.label,
                    range_raw: r.range,
                    spec: &r.spec,
                })
                .collect(),
        }
    }
}

/// Panic unless `tables`, in order, are the tables `pins` names, each by
/// its name, its first row's label and that row's first accuracy figure
/// (`None` where the row prints none). The figures are read from the
/// verified transcription, so a table swapped for another's, or for another
/// model's column, fails.
#[cfg(test)]
pub(crate) fn assert_first_figures(tables: &[&ModeSpecs], pins: &[(&str, &str, Option<&str>)]) {
    let got: Vec<_> = tables
        .iter()
        .map(|t| {
            let row = &t.ranges[0];
            let figure = row.spec.accuracy.first().map(|b| b.accuracy);
            (t.name, row.label, figure)
        })
        .collect();
    assert_eq!(got, pins);
}
