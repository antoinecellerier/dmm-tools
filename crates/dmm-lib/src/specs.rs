//! Shared specification metadata types.
//!
//! These describe resolution, accuracy, and per-mode notes for a measurement.
//! Any protocol family can provide specs by implementing the optional
//! `Protocol::spec_info` / `Protocol::mode_spec_info` methods.

/// Accuracy for a specific frequency band (or DC).
#[derive(Debug, Clone)]
pub struct AccuracyBand {
    /// Frequency range label, or `None` for DC / single-band modes.
    pub freq_range: Option<&'static str>,
    /// Accuracy string without leading `±` (e.g. "0.1%+5").
    pub accuracy: &'static str,
}

/// Per-range specification data (resolution and accuracy).
#[derive(Debug, Clone)]
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
