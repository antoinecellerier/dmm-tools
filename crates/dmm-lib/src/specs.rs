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
    /// Display resolution (e.g. "0.01mV", "1Ω").
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
    /// The table's name: the manual's section title, or the mode's name for
    /// a family whose tables follow its mode bytes.
    pub name: &'static str,
    /// The mode byte the table belongs to, for a family whose tables follow
    /// its mode bytes.
    pub mode_raw: Option<u16>,
    /// The manual's PDF page (not the printed page number), when recorded.
    pub page: Option<u16>,
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
    /// The range byte the row is for, `None` where any range byte is.
    pub range_raw: Option<u8>,
    pub spec: &'static SpecInfo,
}

/// One row of a manual spec table, keyed by the range byte it answers.
#[derive(Debug)]
pub(crate) struct RangeSpec {
    /// The range byte, `None` for a mode with one range whose byte varies
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
            mode_raw: None,
            page: Some(self.page),
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
