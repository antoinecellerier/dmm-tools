use super::{ModeTables, RangeInfo, r};
use crate::protocol::cycle::DialPosition;
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT60BT, a handheld with Bluetooth built in.
///
/// 9,999-count model. Which modes it has, how many rungs each and their
/// range-byte order and units come from UNI-T's iDMM2.0 app, which ships the
/// table as the asset `funOl1_UT60BT.json`. The labels are the UT60BT
/// manual's printed ranges (§X, P2) where it prints the rung, the asset's
/// otherwise. No UT60BT has been connected; see
/// `docs/research/ut61-family/reverse-engineered-protocol.md` §9.
///
/// Key differences from the UT61E+ (22,000-count):
/// - V: 999.9mV..999.9V vs 2.2V..1000V; mV: 9.999mV/99.99mV vs 220mV
/// - Resistance: 6 ranges (999.9Ω..99.99MΩ); capacitance 8 (9.999nF..99.99mF)
/// - µA: one rung; mA and A share one dial position, the A rung sitting on
///   the mA modes, so there are no separate A modes
/// - Temperature; no hFE, LoZ, LPF, AC+DC, Peak
///
/// The dial is not described, so the meter offers no remote mode selection
/// until one confirms its button codes (docs/verification-backlog.md).
pub struct Ut60btTable {
    dc_v: [RangeInfo; 4],
    ac_v: [RangeInfo; 4],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 6],
    capacitance: [RangeInfo; 8],
    hz: [RangeInfo; 8],
    duty_cycle: [RangeInfo; 1],
    temp_c: [RangeInfo; 1],
    temp_f: [RangeInfo; 1],
    diode: [RangeInfo; 1],
    continuity: [RangeInfo; 1],
    dc_ua: [RangeInfo; 1],
    ac_ua: [RangeInfo; 1],
    dc_ma: [RangeInfo; 2],
    ac_ma: [RangeInfo; 2],
}

impl Ut60btTable {
    pub fn new() -> Self {
        Self {
            // The V position starts at 999.9mV, not at a volts rung: the mV
            // position's 9.999mV and 99.99mV are the separate mV modes.
            dc_v: [
                r("999.9mV", "mV"),
                r("9.999V", "V"),
                r("99.99V", "V"),
                r("999.9V", "V"),
            ],
            ac_v: [
                r("999.9mV", "mV"),
                r("9.999V", "V"),
                r("99.99V", "V"),
                r("999.9V", "V"),
            ],
            dc_mv: [r("9.999mV", "mV"), r("99.99mV", "mV")],
            ac_mv: [r("9.999mV", "mV"), r("99.99mV", "mV")],
            // Rung 4 is 9.999MΩ in the manual; the asset's label says 9.99MΩ
            // over a 9.999 bound.
            ohm: [
                r("999.9Ω", "Ω"),
                r("9.999kΩ", "kΩ"),
                r("99.99kΩ", "kΩ"),
                r("999.9kΩ", "kΩ"),
                r("9.999MΩ", "MΩ"),
                r("99.99MΩ", "MΩ"),
            ],
            // The manual stops at 9.999mF (seven rungs); the eighth is the
            // asset's alone. [UNVERIFIED]
            capacitance: [
                r("9.999nF", "nF"),
                r("99.99nF", "nF"),
                r("999.9nF", "nF"),
                r("9.999µF", "µF"),
                r("99.99µF", "µF"),
                r("999.9µF", "µF"),
                r("9.999mF", "mF"),
                r("99.99mF", "mF"),
            ],
            // Hz: the asset's eight rungs. The manual prints a single
            // "99.99Hz~9.999MHz" row, which rungs 1 and 6 bound; rungs 0 and
            // 7 are the asset's alone [UNVERIFIED]. The asset's labels carry
            // a fifth digit ("9.9990Hz") that its own bounds (9.999) and the
            // manual's span do not, and spell MHz "mHz"; the labels here
            // follow the bounds.
            hz: [
                r("9.999Hz", "Hz"),
                r("99.99Hz", "Hz"),
                r("999.9Hz", "Hz"),
                r("9.999kHz", "kHz"),
                r("99.99kHz", "kHz"),
                r("999.9kHz", "kHz"),
                r("9.999MHz", "MHz"),
                r("99.99MHz", "MHz"),
            ],
            duty_cycle: [r("Duty", "%")],
            // The manual's spans; the asset's single rungs say the same.
            temp_c: [r("-40~1000°C", "°C")],
            temp_f: [r("-40~1832°F", "°F")],
            diode: [r("Diode", "V")],
            continuity: [r("Cont", "Ω")],
            dc_ua: [r("999.9µA", "µA")],
            ac_ua: [r("999.9µA", "µA")],
            // The A mA position: past 999.9mA the meter reads on its 9.999A
            // rung, same position and terminal (manual §X). The manual's
            // further 10.00A row has no rung in the asset.
            dc_ma: [r("999.9mA", "mA"), r("9.999A", "A")],
            ac_ma: [r("999.9mA", "mA"), r("9.999A", "A")],
        }
    }
}

impl Default for Ut60btTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ModeTables for Ut60btTable {
    const DIAL_POSITIONS: &'static [DialPosition] = &[];
    const MODEL_NAME: &'static str = "UNI-T UT60BT";
    /// The bytes UNI-T's app sends this meter (family spec §6.5): HOLD,
    /// RANGE and its long press, REL and SELECT. It disables MAX/MIN, and
    /// sends no Peak, SELECT2 or backlight.
    const COMMANDS: &'static [&'static str] = &["hold", "range", "auto", "rel", "select"];
    /// Its Hz/% position takes SELECT (manual §VIII).
    const DUTY_INSTRUCTION: &'static str = "Hz/% position: press SELECT for Duty %.";
    /// UNI-T's app asks the name first, and a UT60BT is reported to need it
    /// (family spec §6.4).
    const NAME_BEFORE_STREAM: bool = true;
    const MODES: &'static [Mode] = &[
        Mode::AcV,
        Mode::AcMv,
        Mode::DcV,
        Mode::DcMv,
        Mode::Hz,
        Mode::DutyCycle,
        Mode::Ohm,
        Mode::Continuity,
        Mode::Diode,
        Mode::Capacitance,
        Mode::TempC,
        Mode::TempF,
        Mode::DcUa,
        Mode::AcUa,
        Mode::DcMa,
        Mode::AcMa,
        Mode::Ncv,
    ];

    fn entry(&self, mode: Mode) -> Option<&[RangeInfo]> {
        Some(match mode {
            Mode::DcV => &self.dc_v,
            Mode::AcV => &self.ac_v,
            Mode::DcMv => &self.dc_mv,
            Mode::AcMv => &self.ac_mv,
            Mode::Ohm => &self.ohm,
            Mode::Capacitance => &self.capacitance,
            Mode::Hz => &self.hz,
            Mode::DutyCycle => &self.duty_cycle,
            Mode::TempC => &self.temp_c,
            Mode::TempF => &self.temp_f,
            Mode::Diode => &self.diode,
            Mode::Continuity => &self.continuity,
            Mode::DcUa => &self.dc_ua,
            Mode::AcUa => &self.ac_ua,
            Mode::DcMa => &self.dc_ma,
            Mode::AcMa => &self.ac_ma,
            // NCV has no ranges; the asset lists DC A and AC A without any
            // and the manual has no A position of their own.
            Mode::Ncv
            | Mode::DcA
            | Mode::AcA
            | Mode::Hfe
            | Mode::Live
            | Mode::LozV
            | Mode::ClampAcA
            | Mode::ClampDcA
            | Mode::LpfV
            | Mode::AcDcV
            | Mode::LpfA
            | Mode::AcDcA
            | Mode::ClampLpfA
            | Mode::ClampAcDcA
            | Mode::Inrush => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::tables::DeviceTable;

    fn table() -> Ut60btTable {
        Ut60btTable::new()
    }

    fn labels(mode: Mode) -> Vec<&'static str> {
        table().ranges(mode).iter().map(|r| r.label).collect()
    }

    #[test]
    fn model_name() {
        assert_eq!(table().model_name(), "UNI-T UT60BT");
    }

    /// Range byte 0 on the V position is millivolts, as the asset says.
    #[test]
    fn volts_start_at_999_9_millivolts() {
        let t = table();
        for mode in [Mode::DcV, Mode::AcV] {
            let r0 = t.range_info(mode, 0).unwrap();
            assert_eq!((r0.label, r0.unit), ("999.9mV", "mV"), "{mode:?}");
            assert_eq!(t.range_info(mode, 3).unwrap().label, "999.9V");
            assert!(t.range_info(mode, 4).is_none());
        }
        assert_eq!(labels(Mode::DcMv), ["9.999mV", "99.99mV"]);
    }

    #[test]
    fn ohm_and_capacitance_ladders() {
        assert_eq!(
            labels(Mode::Ohm),
            [
                "999.9Ω", "9.999kΩ", "99.99kΩ", "999.9kΩ", "9.999MΩ", "99.99MΩ"
            ]
        );
        assert_eq!(labels(Mode::Capacitance).len(), 8);
        assert_eq!(labels(Mode::Capacitance)[7], "99.99mF");
    }

    #[test]
    fn hz_labels_spell_megahertz() {
        let t = table();
        let r6 = t.range_info(Mode::Hz, 6).unwrap();
        assert_eq!((r6.label, r6.unit), ("9.999MHz", "MHz"));
        assert!(t.range_info(Mode::Hz, 8).is_none());
    }

    /// The A rung is range byte 1 of the mA modes.
    #[test]
    fn milliamp_modes_reach_the_amp_rung() {
        let t = table();
        for mode in [Mode::DcMa, Mode::AcMa] {
            assert_eq!(t.range_info(mode, 0).unwrap().unit, "mA");
            let r1 = t.range_info(mode, 1).unwrap();
            assert_eq!((r1.label, r1.unit), ("9.999A", "A"), "{mode:?}");
        }
        assert_eq!(labels(Mode::DcUa), ["999.9µA"]);
    }

    #[test]
    fn no_range_table_modes() {
        let t = table();
        for mode in [
            Mode::Ncv,
            Mode::DcA,
            Mode::AcA,
            Mode::Hfe,
            Mode::LozV,
            Mode::LpfV,
            Mode::AcDcV,
            Mode::Inrush,
        ] {
            assert!(t.range_info(mode, 0).is_none(), "{mode:?}");
        }
    }
}
