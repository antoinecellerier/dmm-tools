use super::specs_ut61b_plus as specs;
use super::{DeviceTable, ModeSpecInfo, RangeInfo, SpecInfo, lookup_range};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61B+ (and UT161B).
///
/// 6,000-count (3¾ digit) model. Range values from the UT61+ Series
/// User Manual, range index ordering is [DEDUCED] (ascending assumed).
///
/// Key differences from UT61E+ (22,000-count):
/// - DC/AC V: 6 ranges (60mV..1000V) vs 5 (220mV..1000V)
/// - Resistance: 6 ranges (600Ω..60MΩ) vs 7 (220Ω..220MΩ)
/// - Capacitance: 7 ranges (60nF..60mF) vs 8 (22nF..220mF)
/// - µA: 600µA/6000µA vs 220µA/2200µA
/// - mA: 60mA/600mA vs 22mA/220mA
/// - A: 6A + 10A vs 20A + 20A
/// - No temperature, no hFE, no LoZ, no LPF, no AC+DC, no Peak
pub struct Ut61bPlusTable {
    dc_v: [RangeInfo; 6],
    ac_v: [RangeInfo; 6],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 6],
    capacitance: [RangeInfo; 7],
    hz: [RangeInfo; 5],
    duty_cycle: [RangeInfo; 1],
    diode: [RangeInfo; 1],
    continuity: [RangeInfo; 1],
    dc_ua: [RangeInfo; 2],
    ac_ua: [RangeInfo; 2],
    dc_ma: [RangeInfo; 2],
    ac_ma: [RangeInfo; 2],
    dc_a: [RangeInfo; 2],
    ac_a: [RangeInfo; 2],
}

impl Ut61bPlusTable {
    pub fn new() -> Self {
        Self {
            // 6 ranges: 60mV, 600mV, 6V, 60V, 600V, 1000V
            dc_v: [
                RangeInfo {
                    label: "60mV",
                    unit: "mV",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mV",
                    unit: "mV",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "6V",
                    unit: "V",
                    overload_pos: 6.0,
                    overload_neg: -6.0,
                },
                RangeInfo {
                    label: "60V",
                    unit: "V",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600V",
                    unit: "V",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "1000V",
                    unit: "V",
                    overload_pos: 1000.0,
                    overload_neg: -1000.0,
                },
            ],
            // Same structure as DC voltage for AC
            ac_v: [
                RangeInfo {
                    label: "60mV",
                    unit: "mV",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mV",
                    unit: "mV",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "6V",
                    unit: "V",
                    overload_pos: 6.0,
                    overload_neg: -6.0,
                },
                RangeInfo {
                    label: "60V",
                    unit: "V",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600V",
                    unit: "V",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "750V",
                    unit: "V",
                    overload_pos: 750.0,
                    overload_neg: -750.0,
                },
            ],
            // mV modes: same as range 0-1 of the V tables
            dc_mv: [
                RangeInfo {
                    label: "60mV",
                    unit: "mV",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mV",
                    unit: "mV",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
            ],
            ac_mv: [
                RangeInfo {
                    label: "60mV",
                    unit: "mV",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mV",
                    unit: "mV",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
            ],
            // 6 ranges: 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ
            ohm: [
                RangeInfo {
                    label: "600Ω",
                    unit: "Ω",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "6kΩ",
                    unit: "kΩ",
                    overload_pos: 6.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "60kΩ",
                    unit: "kΩ",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "600kΩ",
                    unit: "kΩ",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "6MΩ",
                    unit: "MΩ",
                    overload_pos: 6.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "60MΩ",
                    unit: "MΩ",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            // 7 ranges: 60nF, 600nF, 6µF, 60µF, 600µF, 6mF, 60mF
            capacitance: [
                RangeInfo {
                    label: "60nF",
                    unit: "nF",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "600nF",
                    unit: "nF",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "6µF",
                    unit: "µF",
                    overload_pos: 6.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "60µF",
                    unit: "µF",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "600µF",
                    unit: "µF",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "6mF",
                    unit: "mF",
                    overload_pos: 6.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "60mF",
                    unit: "mF",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            // Hz: 6,000-count models max out at 10 MHz (manual)
            // Using same 5-range structure, scaled to 6000-count values
            hz: [
                RangeInfo {
                    label: "60Hz",
                    unit: "Hz",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "600Hz",
                    unit: "Hz",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "6kHz",
                    unit: "kHz",
                    overload_pos: 6.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "60kHz",
                    unit: "kHz",
                    overload_pos: 60.0,
                    overload_neg: f64::NEG_INFINITY,
                },
                RangeInfo {
                    label: "600kHz",
                    unit: "kHz",
                    overload_pos: 600.0,
                    overload_neg: f64::NEG_INFINITY,
                },
            ],
            duty_cycle: [RangeInfo {
                label: "Duty",
                unit: "%",
                overload_pos: 100.0,
                overload_neg: 0.0,
            }],
            diode: [RangeInfo {
                label: "Diode",
                unit: "V",
                overload_pos: 3.0,
                overload_neg: 0.0,
            }],
            // Continuity: 600Ω range for 6,000-count models
            continuity: [RangeInfo {
                label: "Cont",
                unit: "Ω",
                overload_pos: 600.0,
                overload_neg: f64::NEG_INFINITY,
            }],
            // µA: 600µA, 6000µA
            dc_ua: [
                RangeInfo {
                    label: "600µA",
                    unit: "µA",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "6000µA",
                    unit: "µA",
                    overload_pos: 6000.0,
                    overload_neg: -6000.0,
                },
            ],
            ac_ua: [
                RangeInfo {
                    label: "600µA",
                    unit: "µA",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
                RangeInfo {
                    label: "6000µA",
                    unit: "µA",
                    overload_pos: 6000.0,
                    overload_neg: -6000.0,
                },
            ],
            // mA: 60mA, 600mA
            dc_ma: [
                RangeInfo {
                    label: "60mA",
                    unit: "mA",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mA",
                    unit: "mA",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
            ],
            ac_ma: [
                RangeInfo {
                    label: "60mA",
                    unit: "mA",
                    overload_pos: 60.0,
                    overload_neg: -60.0,
                },
                RangeInfo {
                    label: "600mA",
                    unit: "mA",
                    overload_pos: 600.0,
                    overload_neg: -600.0,
                },
            ],
            // A: 6A and 10A (UT61B+ has lower max than D+/E+)
            dc_a: [
                RangeInfo {
                    label: "6A",
                    unit: "A",
                    overload_pos: 6.0,
                    overload_neg: -6.0,
                },
                RangeInfo {
                    label: "10A",
                    unit: "A",
                    overload_pos: 10.0,
                    overload_neg: -10.0,
                },
            ],
            ac_a: [
                RangeInfo {
                    label: "6A",
                    unit: "A",
                    overload_pos: 6.0,
                    overload_neg: -6.0,
                },
                RangeInfo {
                    label: "10A",
                    unit: "A",
                    overload_pos: 10.0,
                    overload_neg: -10.0,
                },
            ],
        }
    }
}

impl Default for Ut61bPlusTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceTable for Ut61bPlusTable {
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
            Mode::Diode => lookup_range(&self.diode, range),
            Mode::Continuity => lookup_range(&self.continuity, range),
            Mode::DcUa => lookup_range(&self.dc_ua, range),
            Mode::AcUa => lookup_range(&self.ac_ua, range),
            Mode::DcMa => lookup_range(&self.dc_ma, range),
            Mode::AcMa => lookup_range(&self.ac_ma, range),
            Mode::DcA => lookup_range(&self.dc_a, range),
            Mode::AcA => lookup_range(&self.ac_a, range),
            // UT61B+ has no temperature, hFE, LoZ, LPF, AC+DC, Peak, Inrush
            Mode::TempC
            | Mode::TempF
            | Mode::Hfe
            | Mode::Live
            | Mode::Ncv
            | Mode::LozV
            | Mode::LozV2
            | Mode::Lpf
            | Mode::LpfV
            | Mode::AcDcV
            | Mode::LpfMv
            | Mode::AcDcMv
            | Mode::LpfA
            | Mode::AcDcA2
            | Mode::Inrush => None,
        }
    }

    fn model_name(&self) -> &'static str {
        "UNI-T UT61B+"
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
            Mode::DcUa => specs::DC_UA_SPECS,
            Mode::AcUa => specs::AC_UA_SPECS,
            Mode::DcMa => specs::DC_MA_SPECS,
            Mode::AcMa => specs::AC_MA_SPECS,
            Mode::DcA => specs::DC_A_SPECS,
            Mode::AcA => specs::AC_A_SPECS,
            Mode::Hz => specs::HZ_SPECS,
            Mode::DutyCycle => specs::DUTY_SPECS,
            _ => return None,
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
            Mode::DcUa => &specs::DC_UA_MODE,
            Mode::AcUa => &specs::AC_UA_MODE,
            Mode::DcMa => &specs::DC_MA_MODE,
            Mode::AcMa => &specs::AC_MA_MODE,
            Mode::DcA => &specs::DC_A_MODE,
            Mode::AcA => &specs::AC_A_MODE,
            Mode::Hz => &specs::HZ_MODE,
            Mode::DutyCycle => &specs::DUTY_MODE,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Ut61bPlusTable {
        Ut61bPlusTable::new()
    }

    #[test]
    fn model_name() {
        assert_eq!(table().model_name(), "UNI-T UT61B+");
    }

    // --- DC Voltage ---
    #[test]
    fn dcv_ranges() {
        let t = table();
        // 6 ranges: 60mV, 600mV, 6V, 60V, 600V, 1000V
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().unit, "mV");
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().overload_pos, 60.0);

        assert_eq!(t.range_info(Mode::DcV, 1).unwrap().label, "600mV");
        assert_eq!(t.range_info(Mode::DcV, 2).unwrap().label, "6V");
        assert_eq!(t.range_info(Mode::DcV, 3).unwrap().label, "60V");
        assert_eq!(t.range_info(Mode::DcV, 4).unwrap().label, "600V");

        let last = t.range_info(Mode::DcV, 5).unwrap();
        assert_eq!(last.label, "1000V");
        assert_eq!(last.unit, "V");
        assert_eq!(last.overload_pos, 1000.0);

        // Out of range
        assert!(t.range_info(Mode::DcV, 6).is_none());
    }

    // --- AC Voltage ---
    #[test]
    fn acv_ranges() {
        let t = table();
        assert_eq!(t.range_info(Mode::AcV, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::AcV, 5).unwrap().label, "750V");
        assert_eq!(t.range_info(Mode::AcV, 5).unwrap().overload_pos, 750.0);
        assert!(t.range_info(Mode::AcV, 6).is_none());
    }

    // --- Resistance ---
    #[test]
    fn ohm_ranges() {
        let t = table();
        let cases = [
            (0, "600Ω", "Ω", 600.0),
            (1, "6kΩ", "kΩ", 6.0),
            (2, "60kΩ", "kΩ", 60.0),
            (3, "600kΩ", "kΩ", 600.0),
            (4, "6MΩ", "MΩ", 6.0),
            (5, "60MΩ", "MΩ", 60.0),
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
        assert!(t.range_info(Mode::Ohm, 6).is_none());
    }

    // --- Capacitance ---
    #[test]
    fn capacitance_ranges() {
        let t = table();
        let cases = [
            (0, "60nF", "nF"),
            (1, "600nF", "nF"),
            (2, "6µF", "µF"),
            (3, "60µF", "µF"),
            (4, "600µF", "µF"),
            (5, "6mF", "mF"),
            (6, "60mF", "mF"),
        ];
        for (range, label, unit) in cases {
            let r = t.range_info(Mode::Capacitance, range).unwrap();
            assert_eq!(r.label, label, "Capacitance range {range}");
            assert_eq!(r.unit, unit, "Capacitance range {range}");
        }
        assert!(t.range_info(Mode::Capacitance, 7).is_none());
    }

    // --- Current ---
    #[test]
    fn microamp_ranges() {
        let t = table();
        for mode in [Mode::DcUa, Mode::AcUa] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "600µA");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "6000µA");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    #[test]
    fn milliamp_ranges() {
        let t = table();
        for mode in [Mode::DcMa, Mode::AcMa] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "60mA");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "600mA");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    #[test]
    fn amp_ranges() {
        let t = table();
        for mode in [Mode::DcA, Mode::AcA] {
            assert_eq!(t.range_info(mode, 0).unwrap().label, "6A");
            assert_eq!(t.range_info(mode, 0).unwrap().overload_pos, 6.0);
            assert_eq!(t.range_info(mode, 1).unwrap().label, "10A");
            assert_eq!(t.range_info(mode, 1).unwrap().overload_pos, 10.0);
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    // --- Modes without range tables ---
    #[test]
    fn no_range_table_modes() {
        let t = table();
        // UT61B+ lacks temperature, hFE, LoZ, LPF, AC+DC, Inrush
        for mode in [
            Mode::TempC,
            Mode::TempF,
            Mode::Hfe,
            Mode::Live,
            Mode::Ncv,
            Mode::LozV,
            Mode::LozV2,
            Mode::Lpf,
            Mode::LpfV,
            Mode::AcDcV,
            Mode::LpfMv,
            Mode::AcDcMv,
            Mode::LpfA,
            Mode::AcDcA2,
            Mode::Inrush,
        ] {
            assert!(
                t.range_info(mode, 0).is_none(),
                "{mode:?} should have no range table on UT61B+"
            );
        }
    }

    #[test]
    fn out_of_range_bytes_return_none() {
        let t = table();
        assert!(t.range_info(Mode::DcV, 0xFF).is_none());
        assert!(t.range_info(Mode::Ohm, 0x10).is_none());
        assert!(t.range_info(Mode::Capacitance, 0x20).is_none());
    }

    #[test]
    fn default_matches_new() {
        let t1 = Ut61bPlusTable::new();
        let t2 = Ut61bPlusTable::default();
        assert_eq!(
            t1.range_info(Mode::DcV, 0).unwrap().label,
            t2.range_info(Mode::DcV, 0).unwrap().label,
        );
    }
}
