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
