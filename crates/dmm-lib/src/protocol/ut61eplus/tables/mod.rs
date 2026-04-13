pub mod specs_ut61b_plus;
pub mod specs_ut61d_plus;
pub mod specs_ut61e_plus;
pub mod ut61b_plus;
pub mod ut61d_plus;
pub mod ut61e_plus;

use super::mode::Mode;

/// Information about a specific measurement range.
#[derive(Debug, Clone)]
pub struct RangeInfo {
    pub label: &'static str,
    pub unit: &'static str,
    pub overload_pos: f64,
    pub overload_neg: f64,
}

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

/// Look up a range entry by index. Shared by all device table implementations.
fn lookup_range(table: &[RangeInfo], range: u8) -> Option<&RangeInfo> {
    table.get(range as usize)
}

/// Trait for device-specific range/unit lookup tables.
pub trait DeviceTable: Send {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo>;
    fn model_name(&self) -> &'static str;

    /// Per-range specification data (resolution, accuracy).
    fn spec_info(&self, _mode: Mode, _range: u8) -> Option<&'static SpecInfo> {
        None
    }

    /// Per-mode specification data (input impedance, notes).
    fn mode_spec_info(&self, _mode: Mode) -> Option<&'static ModeSpecInfo> {
        None
    }
}

/// Look up per-range specs for a device. Takes `device_id` (registry ID),
/// `mode_raw` (protocol mode byte), and `range_raw` (protocol range byte).
/// Returns `None` for unsupported devices or modes.
pub fn lookup_spec(device_id: &str, mode_raw: u16, range_raw: u8) -> Option<&'static SpecInfo> {
    let mode = Mode::from_byte(mode_raw as u8).ok()?;
    let table = table_for_device(device_id)?;
    table.spec_info(mode, range_raw)
}

/// Look up per-mode specs (input impedance, notes) for a device.
pub fn lookup_mode_spec(device_id: &str, mode_raw: u16) -> Option<&'static ModeSpecInfo> {
    let mode = Mode::from_byte(mode_raw as u8).ok()?;
    let table = table_for_device(device_id)?;
    table.mode_spec_info(mode)
}

/// Get a static DeviceTable reference for spec lookups.
fn table_for_device(device_id: &str) -> Option<&'static dyn DeviceTable> {
    // Use thread-local statics to avoid repeated allocations.
    // These tables are tiny and immortal.
    use std::sync::LazyLock;
    static UT61E: LazyLock<ut61e_plus::Ut61ePlusTable> =
        LazyLock::new(ut61e_plus::Ut61ePlusTable::new);
    static UT61B: LazyLock<ut61b_plus::Ut61bPlusTable> =
        LazyLock::new(ut61b_plus::Ut61bPlusTable::new);
    static UT61D: LazyLock<ut61d_plus::Ut61dPlusTable> =
        LazyLock::new(ut61d_plus::Ut61dPlusTable::new);

    match device_id {
        "ut61eplus" | "ut161e" | "mock" => Some(&*UT61E),
        "ut61b+" | "ut161b" => Some(&*UT61B),
        "ut61d+" | "ut161d" => Some(&*UT61D),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_spec_ut61eplus_dcv() {
        // DcV = 0x02, range 0 = 2.2V → resolution "0.1mV"
        let spec = lookup_spec("ut61eplus", 0x02, 0).unwrap();
        assert_eq!(spec.resolution, "0.1mV");
        assert!(!spec.accuracy.is_empty());
    }

    #[test]
    fn lookup_mode_spec_ut61eplus_dcv() {
        let ms = lookup_mode_spec("ut61eplus", 0x02).unwrap();
        assert!(ms.input_impedance.is_some());
    }

    #[test]
    fn lookup_spec_unknown_device() {
        assert!(lookup_spec("ut8803", 0x02, 0).is_none());
    }

    #[test]
    fn lookup_spec_invalid_mode() {
        assert!(lookup_spec("ut61eplus", 0xFF, 0).is_none());
    }

    #[test]
    fn lookup_spec_invalid_range() {
        // DcV with range 99 → None
        assert!(lookup_spec("ut61eplus", 0x02, 99).is_none());
    }

    #[test]
    fn mock_delegates_to_ut61eplus() {
        let mock_spec = lookup_spec("mock", 0x02, 0);
        let eplus_spec = lookup_spec("ut61eplus", 0x02, 0);
        assert_eq!(
            mock_spec.map(|s| s.resolution),
            eplus_spec.map(|s| s.resolution)
        );
    }

    #[test]
    fn lookup_ut61b_plus() {
        // DcV = 0x02, range 0 = 60mV on UT61B+
        let spec = lookup_spec("ut61b+", 0x02, 0).unwrap();
        assert_eq!(spec.resolution, "0.01mV");
    }

    #[test]
    fn lookup_ut61d_plus_temp() {
        // TempC = 0x0A
        let spec = lookup_spec("ut61d+", 0x0A, 0).unwrap();
        assert!(spec.resolution.contains('°'));
    }

    #[test]
    fn ut161_delegates() {
        let ut161b = lookup_spec("ut161b", 0x02, 0);
        let ut61b = lookup_spec("ut61b+", 0x02, 0);
        assert_eq!(ut161b.map(|s| s.resolution), ut61b.map(|s| s.resolution));
    }

    #[test]
    fn acv_has_multiple_accuracy_bands() {
        // AcV = 0x00, range 0 on UT61E+ should have 2+ frequency bands
        let spec = lookup_spec("ut61eplus", 0x00, 0).unwrap();
        assert!(
            spec.accuracy.len() >= 2,
            "AC V should have multiple frequency bands"
        );
    }
}
