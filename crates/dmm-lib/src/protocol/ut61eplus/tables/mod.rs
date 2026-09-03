pub mod specs_ut61b_plus;
pub mod specs_ut61d_plus;
pub mod specs_ut61e_plus;
pub mod ut61b_plus;
pub mod ut61d_plus;
pub mod ut61e_plus;

use super::mode::Mode;

pub use crate::specs::{AccuracyBand, ModeSpecInfo, SpecInfo};

/// Information about a specific measurement range.
#[derive(Debug, Clone)]
pub struct RangeInfo {
    pub label: &'static str,
    pub unit: &'static str,
}

/// One range table entry. The manuals' full-scale limits are recorded in
/// `docs/research/ut61-family/reverse-engineered-protocol.md`, section 9.
pub(crate) const fn r(label: &'static str, unit: &'static str) -> RangeInfo {
    RangeInfo { label, unit }
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

/// Everything a device table knows about one mode.
pub(crate) struct ModeEntry<'a> {
    pub(crate) ranges: Option<&'a [RangeInfo]>,
    pub(crate) specs: Option<&'static [SpecInfo]>,
    pub(crate) mode_spec: Option<&'static ModeSpecInfo>,
}

impl<'a> ModeEntry<'a> {
    /// A mode with range labels, per-range specs and mode-level specs.
    pub(crate) fn full(
        ranges: &'a [RangeInfo],
        specs: &'static [SpecInfo],
        mode_spec: &'static ModeSpecInfo,
    ) -> Self {
        Self {
            ranges: Some(ranges),
            specs: Some(specs),
            mode_spec: Some(mode_spec),
        }
    }

    /// A mode with range labels but no published specification data.
    pub(crate) fn ranges_only(ranges: &'a [RangeInfo]) -> Self {
        Self {
            ranges: Some(ranges),
            specs: None,
            mode_spec: None,
        }
    }

    /// A mode this model does not have.
    pub(crate) fn none() -> Self {
        Self {
            ranges: None,
            specs: None,
            mode_spec: None,
        }
    }
}

/// Per-model data behind `DeviceTable`: one match per mode instead of three.
///
/// Keeping ranges, per-range specs and the mode-level spec in a single match
/// arm is what stops the three from drifting apart when a mode is added.
pub(crate) trait ModeTables: Send {
    /// Model name reported by `DeviceTable::model_name`. An associated const
    /// rather than a method so it cannot collide with the trait method the
    /// blanket impl below derives from it.
    const MODEL_NAME: &'static str;

    fn entry(&self, mode: Mode) -> ModeEntry<'_>;
}

impl<T: ModeTables> DeviceTable for T {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo> {
        self.entry(mode)
            .ranges
            .and_then(|table| lookup_range(table, range))
    }

    fn model_name(&self) -> &'static str {
        T::MODEL_NAME
    }

    fn spec_info(&self, mode: Mode, range: u8) -> Option<&'static SpecInfo> {
        self.entry(mode)
            .specs
            .and_then(|table| table.get(range as usize))
    }

    fn mode_spec_info(&self, mode: Mode) -> Option<&'static ModeSpecInfo> {
        self.entry(mode).mode_spec
    }
}

#[cfg(test)]
mod tests {
    use super::ut61b_plus::Ut61bPlusTable;
    use super::ut61d_plus::Ut61dPlusTable;
    use super::ut61e_plus::Ut61ePlusTable;
    use super::*;

    #[test]
    fn spec_lookup_rejects_an_out_of_bounds_range() {
        // DC V has 5 ranges on the UT61E+; range 99 is not one of them.
        assert!(Ut61ePlusTable::new().spec_info(Mode::DcV, 99).is_none());
    }

    #[test]
    fn ut61b_plus_dcv_specs() {
        // Range 0 = 60mV on the 6,000-count UT61B+.
        let spec = Ut61bPlusTable::new().spec_info(Mode::DcV, 0).unwrap();
        assert_eq!(spec.resolution, "0.01mV");
    }

    /// UT161B has no table of its own — `Ut61PlusProtocol::for_model` hands it
    /// the UT61B+'s, so these are the specs a UT161B reports.
    #[test]
    fn ut161b_uses_the_ut61b_plus_table() {
        let t = Ut61bPlusTable::new();
        assert_eq!(t.model_name(), "UNI-T UT61B+");
        assert_eq!(
            t.spec_info(Mode::DcV, 0).map(|s| s.resolution),
            Some("0.01mV")
        );
    }

    #[test]
    fn ut61d_plus_temperature_specs() {
        let spec = Ut61dPlusTable::new().spec_info(Mode::TempC, 0).unwrap();
        assert!(spec.resolution.contains('°'));
    }

    #[test]
    fn acv_has_multiple_accuracy_bands() {
        let spec = Ut61ePlusTable::new().spec_info(Mode::AcV, 0).unwrap();
        assert!(
            spec.accuracy.len() >= 2,
            "AC V should have multiple frequency bands"
        );
    }
}
