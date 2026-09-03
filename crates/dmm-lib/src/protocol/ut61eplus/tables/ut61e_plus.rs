use super::specs_ut61e_plus as specs;
use super::{ModeEntry, ModeTables, RangeInfo, r};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61E+.
pub struct Ut61ePlusTable {
    // Tables indexed by range byte (0x00..0x07 typically)
    dc_v: [RangeInfo; 5],
    ac_v: [RangeInfo; 5],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 7],
    capacitance: [RangeInfo; 8],
    hz: [RangeInfo; 5],
    duty_cycle: [RangeInfo; 1],
    temp_c: [RangeInfo; 1],
    temp_f: [RangeInfo; 1],
    diode: [RangeInfo; 1],
    continuity: [RangeInfo; 1],
    dc_ua: [RangeInfo; 2],
    ac_ua: [RangeInfo; 2],
    dc_ma: [RangeInfo; 2],
    ac_ma: [RangeInfo; 2],
    dc_a: [RangeInfo; 2],
    ac_a: [RangeInfo; 2],
    hfe: [RangeInfo; 1],
}

impl Ut61ePlusTable {
    pub fn new() -> Self {
        Self {
            dc_v: [
                r("2.2V", "V"),
                r("22V", "V"),
                r("220V", "V"),
                r("1000V", "V"),
                r("220mV", "mV"),
            ],
            ac_v: [
                r("2.2V", "V"),
                r("22V", "V"),
                r("220V", "V"),
                r("750V", "V"),
                r("220mV", "mV"),
            ],
            dc_mv: [r("220mV", "mV"), r("2.2V", "mV")],
            ac_mv: [r("220mV", "mV"), r("2.2V", "mV")],
            ohm: [
                r("220Ω", "Ω"),
                r("2.2kΩ", "kΩ"),
                r("22kΩ", "kΩ"),
                r("220kΩ", "kΩ"),
                r("2.2MΩ", "MΩ"),
                r("22MΩ", "MΩ"),
                r("220MΩ", "MΩ"),
            ],
            capacitance: [
                r("22nF", "nF"),
                r("220nF", "nF"),
                r("2.2µF", "µF"),
                r("22µF", "µF"),
                r("220µF", "µF"),
                r("2.2mF", "mF"),
                r("22mF", "mF"),
                r("220mF", "mF"),
            ],
            hz: [
                r("22Hz", "Hz"),
                r("220Hz", "Hz"),
                r("2.2kHz", "kHz"),
                r("22kHz", "kHz"),
                r("220kHz", "kHz"),
            ],
            duty_cycle: [r("Duty", "%")],
            temp_c: [r("Temp", "°C")],
            temp_f: [r("Temp", "°F")],
            diode: [r("Diode", "V")],
            continuity: [r("Cont", "Ω")],
            dc_ua: [r("220µA", "µA"), r("2200µA", "µA")],
            ac_ua: [r("220µA", "µA"), r("2200µA", "µA")],
            dc_ma: [r("22mA", "mA"), r("220mA", "mA")],
            ac_ma: [r("22mA", "mA"), r("220mA", "mA")],
            dc_a: [
                // Range 0x00 unknown — may not be used. Placeholder.
                r("20A", "A"),
                // Range 0x01 verified: 20A range (confirmed with bench PSU at 100mA)
                r("20A", "A"),
            ],
            ac_a: [r("20A", "A"), r("20A", "A")],
            hfe: [r("1000\u{03B2}", "\u{03B2}")],
        }
    }
}

impl Default for Ut61ePlusTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ModeTables for Ut61ePlusTable {
    const MODEL_NAME: &'static str = "UNI-T UT61E+";

    fn entry(&self, mode: Mode) -> ModeEntry<'_> {
        match mode {
            Mode::DcV => ModeEntry::full(&self.dc_v, specs::DC_V_SPECS, &specs::DC_V_MODE),
            Mode::AcV => ModeEntry::full(&self.ac_v, specs::AC_V_SPECS, &specs::AC_V_MODE),
            Mode::DcMv => ModeEntry::full(&self.dc_mv, specs::DC_MV_SPECS, &specs::DC_MV_MODE),
            Mode::AcMv => ModeEntry::full(&self.ac_mv, specs::AC_MV_SPECS, &specs::AC_MV_MODE),
            Mode::Ohm => ModeEntry::full(&self.ohm, specs::OHM_SPECS, &specs::OHM_MODE),
            Mode::Capacitance => {
                ModeEntry::full(&self.capacitance, specs::CAP_SPECS, &specs::CAP_MODE)
            }
            Mode::Hz => ModeEntry::full(&self.hz, specs::HZ_SPECS, &specs::HZ_MODE),
            Mode::DutyCycle => {
                ModeEntry::full(&self.duty_cycle, specs::DUTY_SPECS, &specs::DUTY_MODE)
            }
            Mode::TempC => ModeEntry::full(&self.temp_c, specs::TEMP_C_SPECS, &specs::TEMP_MODE),
            Mode::TempF => ModeEntry::full(&self.temp_f, specs::TEMP_F_SPECS, &specs::TEMP_MODE),
            Mode::Diode => ModeEntry::full(&self.diode, specs::DIODE_SPECS, &specs::DIODE_MODE),
            Mode::Continuity => ModeEntry::full(
                &self.continuity,
                specs::CONTINUITY_SPECS,
                &specs::CONTINUITY_MODE,
            ),
            Mode::DcUa => ModeEntry::full(&self.dc_ua, specs::DC_UA_SPECS, &specs::DC_UA_MODE),
            Mode::AcUa => ModeEntry::full(&self.ac_ua, specs::AC_UA_SPECS, &specs::AC_UA_MODE),
            Mode::DcMa => ModeEntry::full(&self.dc_ma, specs::DC_MA_SPECS, &specs::DC_MA_MODE),
            Mode::AcMa => ModeEntry::full(&self.ac_ma, specs::AC_MA_SPECS, &specs::AC_MA_MODE),
            Mode::DcA => ModeEntry::full(&self.dc_a, specs::DC_A_SPECS, &specs::DC_A_MODE),
            Mode::AcA => ModeEntry::full(&self.ac_a, specs::AC_A_SPECS, &specs::AC_A_MODE),
            Mode::Hfe => ModeEntry::full(&self.hfe, specs::HFE_SPECS, &specs::HFE_MODE),
            // Derived modes share their base mode's range table, and have specs
            // of their own wherever the manual publishes them.
            Mode::AcDcV => ModeEntry::full(&self.dc_v, specs::ACDC_V_SPECS, &specs::ACDC_V_MODE),
            Mode::LpfV => ModeEntry::full(&self.dc_v, specs::LPF_V_SPECS, &specs::LPF_V_MODE),
            Mode::LpfMv => ModeEntry::full(&self.dc_mv, specs::LPF_MV_SPECS, &specs::LPF_MV_MODE),
            // Derived modes the manual gives no specs for.
            Mode::LozV => ModeEntry::ranges_only(&self.dc_v),
            Mode::AcDcMv => ModeEntry::ranges_only(&self.dc_mv),
            Mode::LozV2 | Mode::Lpf | Mode::AcDcA2 | Mode::LpfA => {
                ModeEntry::ranges_only(&self.dc_a)
            }
            // Modes without range tables or specs.
            Mode::Ncv | Mode::Live | Mode::Inrush => ModeEntry::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::tables::DeviceTable;

    fn table() -> Ut61ePlusTable {
        Ut61ePlusTable::new()
    }

    // --- DC Voltage ---
    #[test]
    fn dcv_ranges() {
        let t = table();
        let r0 = t.range_info(Mode::DcV, 0).unwrap();
        assert_eq!(r0.label, "2.2V");
        assert_eq!(r0.unit, "V");

        let r1 = t.range_info(Mode::DcV, 1).unwrap();
        assert_eq!(r1.label, "22V");

        let r2 = t.range_info(Mode::DcV, 2).unwrap();
        assert_eq!(r2.label, "220V");

        let r3 = t.range_info(Mode::DcV, 3).unwrap();
        assert_eq!(r3.label, "1000V");

        let r4 = t.range_info(Mode::DcV, 4).unwrap();
        assert_eq!(r4.label, "220mV");
        assert_eq!(r4.unit, "mV");

        assert!(t.range_info(Mode::DcV, 5).is_none());
    }

    // --- AC Voltage ---
    #[test]
    fn acv_ranges() {
        let t = table();
        assert_eq!(t.range_info(Mode::AcV, 0).unwrap().label, "2.2V");
        assert_eq!(t.range_info(Mode::AcV, 3).unwrap().label, "750V");
        assert_eq!(t.range_info(Mode::AcV, 4).unwrap().label, "220mV");
        assert!(t.range_info(Mode::AcV, 5).is_none());
    }

    // --- DC/AC millivolts ---
    #[test]
    fn millivolt_ranges() {
        let t = table();
        for mode in [Mode::DcMv, Mode::AcMv] {
            let r0 = t.range_info(mode, 0).unwrap();
            assert_eq!(r0.label, "220mV");
            assert_eq!(r0.unit, "mV");

            let r1 = t.range_info(mode, 1).unwrap();
            assert_eq!(r1.label, "2.2V");

            assert!(t.range_info(mode, 2).is_none());
        }
    }

    // --- Resistance ---
    #[test]
    fn ohm_ranges() {
        let t = table();
        let cases = [
            (0, "220Ω", "Ω"),
            (1, "2.2kΩ", "kΩ"),
            (2, "22kΩ", "kΩ"),
            (3, "220kΩ", "kΩ"),
            (4, "2.2MΩ", "MΩ"),
            (5, "22MΩ", "MΩ"),
            (6, "220MΩ", "MΩ"),
        ];
        for (range, label, unit) in cases {
            let r = t.range_info(Mode::Ohm, range).unwrap();
            assert_eq!(r.label, label, "Ohm range {range}");
            assert_eq!(r.unit, unit, "Ohm range {range}");
        }
        assert!(t.range_info(Mode::Ohm, 7).is_none());
    }

    // --- Capacitance ---
    #[test]
    fn capacitance_ranges() {
        let t = table();
        let cases = [
            (0, "22nF", "nF"),
            (1, "220nF", "nF"),
            (2, "2.2µF", "µF"),
            (3, "22µF", "µF"),
            (4, "220µF", "µF"),
            (5, "2.2mF", "mF"),
            (6, "22mF", "mF"),
            (7, "220mF", "mF"),
        ];
        for (range, label, unit) in cases {
            let r = t.range_info(Mode::Capacitance, range).unwrap();
            assert_eq!(r.label, label, "Capacitance range {range}");
            assert_eq!(r.unit, unit, "Capacitance range {range}");
        }
        assert!(t.range_info(Mode::Capacitance, 8).is_none());
    }

    // --- Hz ---
    #[test]
    fn hz_ranges() {
        let t = table();
        assert_eq!(t.range_info(Mode::Hz, 0).unwrap().label, "22Hz");
        assert_eq!(t.range_info(Mode::Hz, 0).unwrap().unit, "Hz");
        assert_eq!(t.range_info(Mode::Hz, 2).unwrap().label, "2.2kHz");
        assert_eq!(t.range_info(Mode::Hz, 2).unwrap().unit, "kHz");
        assert_eq!(t.range_info(Mode::Hz, 4).unwrap().label, "220kHz");
        assert!(t.range_info(Mode::Hz, 5).is_none());
    }

    // --- Single-range modes ---
    #[test]
    fn duty_cycle_range() {
        let t = table();
        let r = t.range_info(Mode::DutyCycle, 0).unwrap();
        assert_eq!(r.unit, "%");
        assert!(t.range_info(Mode::DutyCycle, 1).is_none());
    }

    #[test]
    fn temp_ranges() {
        let t = table();
        let tc = t.range_info(Mode::TempC, 0).unwrap();
        assert_eq!(tc.unit, "°C");

        let tf = t.range_info(Mode::TempF, 0).unwrap();
        assert_eq!(tf.unit, "°F");
    }

    #[test]
    fn diode_range() {
        let t = table();
        let r = t.range_info(Mode::Diode, 0).unwrap();
        assert_eq!(r.unit, "V");
    }

    #[test]
    fn continuity_range() {
        let t = table();
        let r = t.range_info(Mode::Continuity, 0).unwrap();
        assert_eq!(r.unit, "Ω");
    }

    // --- Current ranges ---
    #[test]
    fn microamp_ranges() {
        let t = table();
        for mode in [Mode::DcUa, Mode::AcUa] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "220µA");
            assert_eq!(t.range_info(mode, 0).unwrap().unit, "µA");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "2200µA");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    #[test]
    fn milliamp_ranges() {
        let t = table();
        for mode in [Mode::DcMa, Mode::AcMa] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "22mA");
            assert_eq!(t.range_info(mode, 0).unwrap().unit, "mA");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "220mA");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    #[test]
    fn amp_ranges() {
        let t = table();
        for mode in [Mode::DcA, Mode::AcA] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "20A");
            assert_eq!(t.range_info(mode, 0).unwrap().unit, "A");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "20A");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    // --- Derived modes delegate to base tables ---
    #[test]
    fn derived_voltage_modes_use_dcv_table() {
        let t = table();
        for mode in [Mode::AcDcV, Mode::LpfV, Mode::LozV] {
            let r = t.range_info(mode, 0).unwrap();
            assert_eq!(r.label, "2.2V", "{mode:?} should use DCV table");
            assert_eq!(r.unit, "V");
            let r3 = t.range_info(mode, 3).unwrap();
            assert_eq!(r3.label, "1000V", "{mode:?} range 3 should be 1000V");
        }
    }

    #[test]
    fn derived_millivolt_modes_use_dcmv_table() {
        let t = table();
        for mode in [Mode::AcDcMv, Mode::LpfMv] {
            let r = t.range_info(mode, 0).unwrap();
            assert_eq!(r.label, "220mV", "{mode:?} should use DCmV table");
        }
    }

    #[test]
    fn derived_amp_modes_use_dca_table() {
        let t = table();
        for mode in [Mode::LozV2, Mode::Lpf, Mode::AcDcA2, Mode::LpfA] {
            let r = t.range_info(mode, 0).unwrap();
            assert_eq!(r.label, "20A", "{mode:?} should use DCA table");
            assert_eq!(r.unit, "A");
        }
    }

    // --- Modes without range tables ---
    #[test]
    fn no_range_table_modes() {
        let t = table();
        for mode in [Mode::Ncv, Mode::Live, Mode::Inrush] {
            assert!(
                t.range_info(mode, 0).is_none(),
                "{mode:?} should have no range table"
            );
        }
    }

    #[test]
    fn out_of_range_bytes_return_none() {
        let t = table();
        // Every mode should return None for a sufficiently large range byte
        assert!(t.range_info(Mode::DcV, 0xFF).is_none());
        assert!(t.range_info(Mode::Ohm, 0x10).is_none());
        assert!(t.range_info(Mode::Capacitance, 0x20).is_none());
    }

    #[test]
    fn model_name() {
        let t = table();
        assert_eq!(t.model_name(), "UNI-T UT61E+");
    }

    #[test]
    fn default_matches_new() {
        let t1 = Ut61ePlusTable::new();
        let t2 = Ut61ePlusTable::default();
        // Both should return the same range info
        assert_eq!(
            t1.range_info(Mode::DcV, 0).unwrap().label,
            t2.range_info(Mode::DcV, 0).unwrap().label,
        );
    }
}
