use super::specs_ut61b_plus as specs;
use super::{ModeEntry, ModeTables, RangeInfo, m, r};
use crate::protocol::cycle::{CycleButton, DialPosition, Ring};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61B+ (and UT161B).
///
/// 6,000-count (3¾ digit) model. Range values from the UT61+ Series User
/// Manual. Ascending index order is [VERIFIED] at the rungs three UT61B+
/// captures reached (2026-09-09 to 2026-09-11, issue #19): every Ω rung,
/// every DC V rung, AC V 0 and 2, capacitance 0 and 5, and µA, mA and A 0-1.
/// Still [DEDUCED]: AC V 1 and 3, capacitance 1-4 and 6, and both mV tables.
///
/// Key differences from UT61E+ (22,000-count):
/// - DC/AC V: 4 ranges (6V..1000V) vs 4 (2.2V..1000V)
/// - Resistance: 6 ranges (600Ω..60MΩ) vs 7 (220Ω..220MΩ)
/// - Capacitance: 7 ranges (60nF..60mF) vs 8 (22nF..220mF)
/// - µA: 600µA/6000µA vs 220µA/2200µA
/// - mA: 60mA/600mA vs 22mA/220mA
/// - A: 6A + 10A vs 20A + 20A
/// - No temperature, no hFE, no LoZ, no LPF, no AC+DC, no Peak
pub struct Ut61bPlusTable {
    dc_v: [RangeInfo; 4],
    ac_v: [RangeInfo; 4],
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
            // Four ranges: the V⎓ position starts at 6V. Range byte 0 came
            // back as 6.000 V on a UT61B+ (capture 2026-09-09, step `dcv`:
            // "  0.000", three decimals on a 6,000-count meter). 60mV and
            // 600mV are the separate DC mV mode (0x03) on its own dial
            // position, the same correction the UT61E+ needed.
            dc_v: [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("1000V", "V")],
            // Same structure as DC voltage for AC; the same capture read
            // "  0.037" at range 0 in V~, which the meter showed as volts.
            ac_v: [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("750V", "V")],
            // mV modes: their own dial position, not a rung of the V tables
            dc_mv: [r("60mV", "mV"), r("600mV", "mV")],
            ac_mv: [r("60mV", "mV"), r("600mV", "mV")],
            // 6 ranges: 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ
            ohm: [
                r("600Ω", "Ω"),
                r("6kΩ", "kΩ"),
                r("60kΩ", "kΩ"),
                r("600kΩ", "kΩ"),
                r("6MΩ", "MΩ"),
                r("60MΩ", "MΩ"),
            ],
            // 7 ranges: 60nF, 600nF, 6µF, 60µF, 600µF, 6mF, 60mF
            capacitance: [
                r("60nF", "nF"),
                r("600nF", "nF"),
                r("6µF", "µF"),
                r("60µF", "µF"),
                r("600µF", "µF"),
                r("6mF", "mF"),
                r("60mF", "mF"),
            ],
            // Hz: 6,000-count models max out at 10 MHz (manual)
            // Using same 5-range structure, scaled to 6000-count values
            hz: [
                r("60Hz", "Hz"),
                r("600Hz", "Hz"),
                r("6kHz", "kHz"),
                r("60kHz", "kHz"),
                r("600kHz", "kHz"),
            ],
            duty_cycle: [r("Duty", "%")],
            diode: [r("Diode", "V")],
            // Continuity: 600Ω range for 6,000-count models
            continuity: [r("Cont", "Ω")],
            // µA: 600µA, 6000µA
            dc_ua: [r("600µA", "µA"), r("6000µA", "µA")],
            ac_ua: [r("600µA", "µA"), r("6000µA", "µA")],
            // mA: 60mA, 600mA
            dc_ma: [r("60mA", "mA"), r("600mA", "mA")],
            ac_ma: [r("60mA", "mA"), r("600mA", "mA")],
            // A: 6A and 10A (UT61B+ has lower max than D+/E+)
            dc_a: [r("6A", "A"), r("10A", "A")],
            ac_a: [r("6A", "A"), r("10A", "A")],
        }
    }
}

impl Default for Ut61bPlusTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Dial positions of the UT61B+/UT161B — [MANUAL], UT61+ Series User Manual
/// §VII "Function Dial" (printed page 9).
///
/// Membership only: the driver presses until the target shows, so no ring
/// here claims a press order. No press has been observed on this model; see
/// `docs/research/ut61-family/reverse-engineered-protocol.md` §3.1.
const DIAL: &[DialPosition] = &[
    // Hz/% — the dedicated frequency position, listed first because it is the
    // smallest one reaching Hz and Duty %, which every ring below also reaches.
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Hz,
            modes: &[m(Mode::Hz), m(Mode::DutyCycle)],
        }],
    },
    // V~ — no AC+DC and no LPF on this model, so SELECT offers nothing here.
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Hz,
            modes: &[m(Mode::AcV), m(Mode::Hz), m(Mode::DutyCycle)],
        }],
    },
    // V⎓
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::DcV)],
        }],
    },
    // mV
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::DcMv), m(Mode::AcMv)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcMv), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
    },
    // Ω / continuity
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::Ohm), m(Mode::Continuity)],
        }],
    },
    // Diode / capacitance — a position of its own on this model.
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::Diode), m(Mode::Capacitance)],
        }],
    },
    // µA
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::DcUa), m(Mode::AcUa)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcUa), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
    },
    // mA
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::DcMa), m(Mode::AcMa)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcMa), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
    },
    // A
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::DcA), m(Mode::AcA)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcA), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
    },
    // NCV
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::Ncv)],
        }],
    },
];

impl ModeTables for Ut61bPlusTable {
    const DIAL_POSITIONS: &'static [DialPosition] = DIAL;
    const MODEL_NAME: &'static str = "UNI-T UT61B+";

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
            | Mode::Inrush => ModeEntry::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::tables::DeviceTable;

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
        // 4 ranges: 6V, 60V, 600V, 1000V
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().label, "6V");
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().unit, "V");

        assert_eq!(t.range_info(Mode::DcV, 1).unwrap().label, "60V");
        assert_eq!(t.range_info(Mode::DcV, 2).unwrap().label, "600V");

        let last = t.range_info(Mode::DcV, 3).unwrap();
        assert_eq!(last.label, "1000V");
        assert_eq!(last.unit, "V");

        // Out of range
        assert!(t.range_info(Mode::DcV, 4).is_none());
    }

    // --- AC Voltage ---
    #[test]
    fn acv_ranges() {
        let t = table();
        assert_eq!(t.range_info(Mode::AcV, 0).unwrap().label, "6V");
        assert_eq!(t.range_info(Mode::AcV, 3).unwrap().label, "750V");
        assert!(t.range_info(Mode::AcV, 4).is_none());
    }

    /// Range byte 0 in V⎓/V~ is 6V, not 60mV: a UT61B+ sent `  0.000` and
    /// `  0.037` there while its screen showed volts (capture 2026-09-09).
    /// Reading those rungs as mV labelled every voltage a thousandth low.
    #[test]
    fn voltage_range_0_is_volts_not_millivolts() {
        let t = table();
        for mode in [Mode::DcV, Mode::AcV] {
            let r0 = t.range_info(mode, 0).unwrap();
            assert_eq!(r0.label, "6V", "{mode:?} range 0");
            assert_eq!(r0.unit, "V", "{mode:?} range 0");
        }
        // The mV ranges are reached through the mV modes instead.
        assert_eq!(t.range_info(Mode::DcMv, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::AcMv, 0).unwrap().label, "60mV");
    }

    // --- Resistance ---
    #[test]
    fn ohm_ranges() {
        let t = table();
        let cases = [
            (0, "600Ω", "Ω"),
            (1, "6kΩ", "kΩ"),
            (2, "60kΩ", "kΩ"),
            (3, "600kΩ", "kΩ"),
            (4, "6MΩ", "MΩ"),
            (5, "60MΩ", "MΩ"),
        ];
        for (range, label, unit) in cases {
            let r = t.range_info(Mode::Ohm, range).unwrap();
            assert_eq!(r.label, label, "Ohm range {range}");
            assert_eq!(r.unit, unit, "Ohm range {range}");
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
            assert_eq!(t.range_info(mode, 1).unwrap().label, "10A");
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
