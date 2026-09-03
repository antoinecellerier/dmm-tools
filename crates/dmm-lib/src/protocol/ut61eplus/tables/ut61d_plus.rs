use super::specs_ut61b_plus as specs_b;
use super::specs_ut61d_plus as specs;
use super::{ModeEntry, ModeTables, RangeInfo, r};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61D+ (and UT161D).
///
/// 6,000-count (3¾ digit) model. Same ranges as UT61B+ but WITH:
/// - Temperature (TempC/TempF, K-type thermocouple)
/// - LoZ V mode
/// - Peak (P-MAX/P-MIN)
///
/// Does NOT have: hFE, LPF, AC+DC, Inrush.
///
/// Range values from the UT61+ Series User Manual, range index
/// ordering is [DEDUCED] (ascending assumed).
pub struct Ut61dPlusTable {
    dc_v: [RangeInfo; 6],
    ac_v: [RangeInfo; 6],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 6],
    capacitance: [RangeInfo; 7],
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
    loz_v: [RangeInfo; 2],
}

impl Ut61dPlusTable {
    pub fn new() -> Self {
        Self {
            // 6 ranges: 60mV, 600mV, 6V, 60V, 600V, 1000V
            dc_v: [
                r("60mV", "mV"),
                r("600mV", "mV"),
                r("6V", "V"),
                r("60V", "V"),
                r("600V", "V"),
                r("1000V", "V"),
            ],
            ac_v: [
                r("60mV", "mV"),
                r("600mV", "mV"),
                r("6V", "V"),
                r("60V", "V"),
                r("600V", "V"),
                r("750V", "V"),
            ],
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
            // Hz: 6,000-count models max out at 10 MHz
            hz: [
                r("60Hz", "Hz"),
                r("600Hz", "Hz"),
                r("6kHz", "kHz"),
                r("60kHz", "kHz"),
                r("600kHz", "kHz"),
            ],
            duty_cycle: [r("Duty", "%")],
            // UT61D+ has temperature (K-type thermocouple)
            temp_c: [r("Temp", "°C")],
            temp_f: [r("Temp", "°F")],
            diode: [r("Diode", "V")],
            // Continuity: 600Ω range for 6,000-count models
            continuity: [r("Cont", "Ω")],
            // µA: 600µA, 6000µA
            dc_ua: [r("600µA", "µA"), r("6000µA", "µA")],
            ac_ua: [r("600µA", "µA"), r("6000µA", "µA")],
            // mA: 60mA, 600mA
            dc_ma: [r("60mA", "mA"), r("600mA", "mA")],
            ac_ma: [r("60mA", "mA"), r("600mA", "mA")],
            // A: UT61D+ has 20A max (same as E+)
            dc_a: [r("20A", "A"), r("20A", "A")],
            ac_a: [r("20A", "A"), r("20A", "A")],
            // LoZ ACV: 600V and 1000V ranges (UT61D+ only)
            loz_v: [r("600V", "V"), r("1000V", "V")],
        }
    }
}

impl Default for Ut61dPlusTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ModeTables for Ut61dPlusTable {
    const MODEL_NAME: &'static str = "UNI-T UT61D+";

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
            // DC µA/mA specs are shared with the UT61B+; the AC tables are not.
            Mode::DcUa => ModeEntry::full(&self.dc_ua, specs_b::DC_UA_SPECS, &specs_b::DC_UA_MODE),
            Mode::AcUa => ModeEntry::full(&self.ac_ua, specs::AC_UA_SPECS, &specs::AC_UA_MODE),
            Mode::DcMa => ModeEntry::full(&self.dc_ma, specs_b::DC_MA_SPECS, &specs_b::DC_MA_MODE),
            Mode::AcMa => ModeEntry::full(&self.ac_ma, specs::AC_MA_SPECS, &specs::AC_MA_MODE),
            Mode::DcA => ModeEntry::full(&self.dc_a, specs::DC_A_SPECS, &specs::DC_A_MODE),
            Mode::AcA => ModeEntry::full(&self.ac_a, specs::AC_A_SPECS, &specs::AC_A_MODE),
            // UT61D+ has LoZ V mode
            Mode::LozV | Mode::LozV2 => {
                ModeEntry::full(&self.loz_v, specs::LOZ_V_SPECS, &specs::LOZ_V_MODE)
            }
            // UT61D+ has no hFE, no LPF, no AC+DC, no Inrush
            Mode::Hfe
            | Mode::Live
            | Mode::Ncv
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

    fn table() -> Ut61dPlusTable {
        Ut61dPlusTable::new()
    }

    #[test]
    fn model_name() {
        assert_eq!(table().model_name(), "UNI-T UT61D+");
    }

    // --- DC Voltage ---
    #[test]
    fn dcv_ranges() {
        let t = table();
        // 6 ranges: 60mV, 600mV, 6V, 60V, 600V, 1000V
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::DcV, 0).unwrap().unit, "mV");

        assert_eq!(t.range_info(Mode::DcV, 1).unwrap().label, "600mV");
        assert_eq!(t.range_info(Mode::DcV, 2).unwrap().label, "6V");
        assert_eq!(t.range_info(Mode::DcV, 3).unwrap().label, "60V");
        assert_eq!(t.range_info(Mode::DcV, 4).unwrap().label, "600V");

        let last = t.range_info(Mode::DcV, 5).unwrap();
        assert_eq!(last.label, "1000V");
        assert_eq!(last.unit, "V");

        assert!(t.range_info(Mode::DcV, 6).is_none());
    }

    // --- AC Voltage ---
    #[test]
    fn acv_ranges() {
        let t = table();
        assert_eq!(t.range_info(Mode::AcV, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::AcV, 5).unwrap().label, "750V");
        assert!(t.range_info(Mode::AcV, 6).is_none());
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

    // --- Temperature (UT61D+ has it!) ---
    #[test]
    fn temp_ranges() {
        let t = table();
        let tc = t.range_info(Mode::TempC, 0).unwrap();
        assert_eq!(tc.unit, "°C");

        let tf = t.range_info(Mode::TempF, 0).unwrap();
        assert_eq!(tf.unit, "°F");

        assert!(t.range_info(Mode::TempC, 1).is_none());
        assert!(t.range_info(Mode::TempF, 1).is_none());
    }

    // --- LoZ V (UT61D+ has it!) ---
    #[test]
    fn loz_v_ranges() {
        let t = table();
        let r0 = t.range_info(Mode::LozV, 0).unwrap();
        assert_eq!(r0.label, "600V");
        assert_eq!(r0.unit, "V");

        let r1 = t.range_info(Mode::LozV, 1).unwrap();
        assert_eq!(r1.label, "1000V");

        // LozV2 also maps to loz_v table
        assert_eq!(t.range_info(Mode::LozV2, 0).unwrap().label, "600V");

        assert!(t.range_info(Mode::LozV, 2).is_none());
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
            assert_eq!(t.range_info(mode, 0).unwrap().label, "20A");
            assert_eq!(t.range_info(mode, 1).unwrap().label, "20A");
            assert!(t.range_info(mode, 2).is_none());
        }
    }

    // --- Modes without range tables ---
    #[test]
    fn no_range_table_modes() {
        let t = table();
        // UT61D+ lacks hFE, LPF, AC+DC, Inrush
        for mode in [
            Mode::Hfe,
            Mode::Live,
            Mode::Ncv,
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
                "{mode:?} should have no range table on UT61D+"
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
        let t1 = Ut61dPlusTable::new();
        let t2 = Ut61dPlusTable::default();
        assert_eq!(
            t1.range_info(Mode::DcV, 0).unwrap().label,
            t2.range_info(Mode::DcV, 0).unwrap().label,
        );
    }
}
