use super::specs_ut61e_plus as specs;
use super::{DeviceTable, ModeSpecInfo, RangeInfo, SpecInfo, lookup_range};
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
                RangeInfo {
                    label: "2.2V",
                    unit: "V",
                    overload_pos: 2.2,
                    overload_neg: -2.2,
                },
                RangeInfo {
                    label: "22V",
                    unit: "V",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220V",
                    unit: "V",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "1000V",
                    unit: "V",
                    overload_pos: 1000.0,
                    overload_neg: -1000.0,
                },
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            ac_v: [
                RangeInfo {
                    label: "2.2V",
                    unit: "V",
                    overload_pos: 2.2,
                    overload_neg: -2.2,
                },
                RangeInfo {
                    label: "22V",
                    unit: "V",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220V",
                    unit: "V",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "750V",
                    unit: "V",
                    overload_pos: 750.0,
                    overload_neg: -750.0,
                },
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            dc_mv: [
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2.2V",
                    unit: "mV",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ac_mv: [
                RangeInfo {
                    label: "220mV",
                    unit: "mV",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2.2V",
                    unit: "mV",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ohm: [
                RangeInfo {
                    label: "220Ω",
                    unit: "Ω",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2kΩ",
                    unit: "kΩ",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22kΩ",
                    unit: "kΩ",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220kΩ",
                    unit: "kΩ",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2MΩ",
                    unit: "MΩ",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22MΩ",
                    unit: "MΩ",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220MΩ",
                    unit: "MΩ",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            capacitance: [
                RangeInfo {
                    label: "22nF",
                    unit: "nF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220nF",
                    unit: "nF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2µF",
                    unit: "µF",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22µF",
                    unit: "µF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220µF",
                    unit: "µF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2mF",
                    unit: "mF",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22mF",
                    unit: "mF",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220mF",
                    unit: "mF",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            hz: [
                RangeInfo {
                    label: "22Hz",
                    unit: "Hz",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220Hz",
                    unit: "Hz",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "2.2kHz",
                    unit: "kHz",
                    overload_pos: 2.2,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "22kHz",
                    unit: "kHz",
                    overload_pos: 22.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "220kHz",
                    unit: "kHz",
                    overload_pos: 220.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            duty_cycle: [RangeInfo {
                label: "Duty",
                unit: "%",
                overload_pos: 100.0,
                overload_neg: 0.0,
            }],
            temp_c: [RangeInfo {
                label: "Temp",
                unit: "°C",
                overload_pos: 1200.0,
                overload_neg: -40.0,
            }],
            temp_f: [RangeInfo {
                label: "Temp",
                unit: "°F",
                overload_pos: 2192.0,
                overload_neg: -40.0,
            }],
            diode: [RangeInfo {
                label: "Diode",
                unit: "V",
                overload_pos: 2.2,
                overload_neg: 0.0,
            }],
            continuity: [RangeInfo {
                label: "Cont",
                unit: "Ω",
                overload_pos: 220.0,
                overload_neg: f64::NEG_INFINITY,
            }],
            dc_ua: [
                RangeInfo {
                    label: "220µA",
                    unit: "µA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2200µA",
                    unit: "µA",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            ac_ua: [
                RangeInfo {
                    label: "220µA",
                    unit: "µA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
                RangeInfo {
                    label: "2200µA",
                    unit: "µA",
                    overload_pos: 2200.0,
                    overload_neg: -2200.0,
                },
            ],
            dc_ma: [
                RangeInfo {
                    label: "22mA",
                    unit: "mA",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220mA",
                    unit: "mA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            ac_ma: [
                RangeInfo {
                    label: "22mA",
                    unit: "mA",
                    overload_pos: 22.0,
                    overload_neg: -22.0,
                },
                RangeInfo {
                    label: "220mA",
                    unit: "mA",
                    overload_pos: 220.0,
                    overload_neg: -220.0,
                },
            ],
            dc_a: [
                // Range 0x00 unknown — may not be used. Placeholder.
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
                // Range 0x01 verified: 20A range (confirmed with bench PSU at 100mA)
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
            ],
            ac_a: [
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
                RangeInfo {
                    label: "20A",
                    unit: "A",
                    overload_pos: 20.0,
                    overload_neg: -20.0,
                },
            ],
            hfe: [RangeInfo {
                label: "1000\u{03B2}",
                unit: "\u{03B2}",
                overload_pos: 1000.0,
                overload_neg: 0.0,
            }],
        }
    }
}

impl Default for Ut61ePlusTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceTable for Ut61ePlusTable {
    fn range_info(&self, mode: Mode, range: u8) -> Option<&RangeInfo> {
        match mode {
            Mode::DcV => lookup_range(&self.dc_v, range),
            Mode::AcV => lookup_range(&self.ac_v, range),
            Mode::DcMv => lookup_range(&self.dc_mv, range),
            Mode::AcMv => lookup_range(&self.ac_mv, range),
            Mode::Ohm => lookup_range(&self.ohm, range),
            Mode::Capacitance => lookup_range(&self.capacitance, range),
            Mode::Hz => lookup_range(&self.hz, range),
            Mode::DutyCycle => lookup_range(&self.duty_cycle, range),
            Mode::TempC => lookup_range(&self.temp_c, range),
            Mode::TempF => lookup_range(&self.temp_f, range),
            Mode::Diode => lookup_range(&self.diode, range),
            Mode::Continuity => lookup_range(&self.continuity, range),
            Mode::DcUa => lookup_range(&self.dc_ua, range),
            Mode::AcUa => lookup_range(&self.ac_ua, range),
            Mode::DcMa => lookup_range(&self.dc_ma, range),
            Mode::AcMa => lookup_range(&self.ac_ma, range),
            Mode::DcA => lookup_range(&self.dc_a, range),
            Mode::AcA => lookup_range(&self.ac_a, range),
            // Derived modes share tables with their base mode
            Mode::AcDcV | Mode::LpfV | Mode::LozV => lookup_range(&self.dc_v, range),
            Mode::AcDcMv | Mode::LpfMv => lookup_range(&self.dc_mv, range),
            Mode::LozV2 | Mode::Lpf | Mode::AcDcA2 | Mode::LpfA => lookup_range(&self.dc_a, range),
            Mode::Hfe => lookup_range(&self.hfe, range),
            // Modes without range tables
            Mode::Ncv | Mode::Live | Mode::Inrush => None,
        }
    }

    fn model_name(&self) -> &'static str {
        "UNI-T UT61E+"
    }

    fn spec_info(&self, mode: Mode, range: u8) -> Option<&'static SpecInfo> {
        let table: &[SpecInfo] = match mode {
            Mode::DcV => specs::DC_V_SPECS,
            Mode::AcV => specs::AC_V_SPECS,
            Mode::DcMv => specs::DC_MV_SPECS,
            Mode::AcMv => specs::AC_MV_SPECS,
            Mode::Ohm => specs::OHM_SPECS,
            Mode::Continuity => specs::CONTINUITY_SPECS,
            Mode::Diode => specs::DIODE_SPECS,
            Mode::Capacitance => specs::CAP_SPECS,
            Mode::TempC => specs::TEMP_C_SPECS,
            Mode::TempF => specs::TEMP_F_SPECS,
            Mode::DcUa => specs::DC_UA_SPECS,
            Mode::AcUa => specs::AC_UA_SPECS,
            Mode::DcMa => specs::DC_MA_SPECS,
            Mode::AcMa => specs::AC_MA_SPECS,
            Mode::DcA => specs::DC_A_SPECS,
            Mode::AcA => specs::AC_A_SPECS,
            Mode::Hz => specs::HZ_SPECS,
            Mode::DutyCycle => specs::DUTY_SPECS,
            Mode::AcDcV => specs::ACDC_V_SPECS,
            Mode::LpfV => specs::LPF_V_SPECS,
            Mode::LpfMv => specs::LPF_MV_SPECS,
            Mode::Hfe => specs::HFE_SPECS,
            // No published specs for these modes on UT61E+
            Mode::LozV
            | Mode::LozV2
            | Mode::Lpf
            | Mode::AcDcMv
            | Mode::LpfA
            | Mode::AcDcA2
            | Mode::Ncv
            | Mode::Live
            | Mode::Inrush => return None,
        };
        table.get(range as usize)
    }

    fn mode_spec_info(&self, mode: Mode) -> Option<&'static ModeSpecInfo> {
        Some(match mode {
            Mode::DcV => &specs::DC_V_MODE,
            Mode::AcV => &specs::AC_V_MODE,
            Mode::DcMv => &specs::DC_MV_MODE,
            Mode::AcMv => &specs::AC_MV_MODE,
            Mode::Ohm => &specs::OHM_MODE,
            Mode::Continuity => &specs::CONTINUITY_MODE,
            Mode::Diode => &specs::DIODE_MODE,
            Mode::Capacitance => &specs::CAP_MODE,
            Mode::TempC | Mode::TempF => &specs::TEMP_MODE,
            Mode::DcUa => &specs::DC_UA_MODE,
            Mode::AcUa => &specs::AC_UA_MODE,
            Mode::DcMa => &specs::DC_MA_MODE,
            Mode::AcMa => &specs::AC_MA_MODE,
            Mode::DcA => &specs::DC_A_MODE,
            Mode::AcA => &specs::AC_A_MODE,
            Mode::Hz => &specs::HZ_MODE,
            Mode::DutyCycle => &specs::DUTY_MODE,
            Mode::AcDcV => &specs::ACDC_V_MODE,
            Mode::LpfV => &specs::LPF_V_MODE,
            Mode::LpfMv => &specs::LPF_MV_MODE,
            Mode::Hfe => &specs::HFE_MODE,
            Mode::LozV
            | Mode::LozV2
            | Mode::Lpf
            | Mode::AcDcMv
            | Mode::LpfA
            | Mode::AcDcA2
            | Mode::Ncv
            | Mode::Live
            | Mode::Inrush => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(r0.overload_pos, 2.2);
        assert_eq!(r0.overload_neg, -2.2);

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
        assert_eq!(t.range_info(Mode::AcV, 3).unwrap().overload_pos, 750.0);
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
            assert_eq!(r1.overload_pos, 2200.0);

            assert!(t.range_info(mode, 2).is_none());
        }
    }

    // --- Resistance ---
    #[test]
    fn ohm_ranges() {
        let t = table();
        let cases = [
            (0, "220Ω", "Ω", 220.0),
            (1, "2.2kΩ", "kΩ", 2.2),
            (2, "22kΩ", "kΩ", 22.0),
            (3, "220kΩ", "kΩ", 220.0),
            (4, "2.2MΩ", "MΩ", 2.2),
            (5, "22MΩ", "MΩ", 22.0),
            (6, "220MΩ", "MΩ", 220.0),
        ];
        for (range, label, unit, overload) in cases {
            let r = t.range_info(Mode::Ohm, range).unwrap();
            assert_eq!(r.label, label, "Ohm range {range}");
            assert_eq!(r.unit, unit, "Ohm range {range}");
            assert_eq!(r.overload_pos, overload, "Ohm range {range}");
            assert!(
                r.overload_neg.is_infinite(),
                "Ohm overload_neg should be -inf"
            );
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
        assert_eq!(r.overload_pos, 100.0);
        assert!(t.range_info(Mode::DutyCycle, 1).is_none());
    }

    #[test]
    fn temp_ranges() {
        let t = table();
        let tc = t.range_info(Mode::TempC, 0).unwrap();
        assert_eq!(tc.unit, "°C");
        assert_eq!(tc.overload_pos, 1200.0);
        assert_eq!(tc.overload_neg, -40.0);

        let tf = t.range_info(Mode::TempF, 0).unwrap();
        assert_eq!(tf.unit, "°F");
        assert_eq!(tf.overload_pos, 2192.0);
    }

    #[test]
    fn diode_range() {
        let t = table();
        let r = t.range_info(Mode::Diode, 0).unwrap();
        assert_eq!(r.unit, "V");
        assert_eq!(r.overload_pos, 2.2);
    }

    #[test]
    fn continuity_range() {
        let t = table();
        let r = t.range_info(Mode::Continuity, 0).unwrap();
        assert_eq!(r.unit, "Ω");
        assert_eq!(r.overload_pos, 220.0);
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
