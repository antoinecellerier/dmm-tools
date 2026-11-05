use super::{ModeTables, RangeInfo, m, r};
use crate::protocol::cycle::{CycleButton, DialPosition, Ring};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61B+ (and UT161B).
///
/// 6,000-count (3¾ digit) model. Range values from the UT61+ Series User
/// Manual. Ascending index order is `[VERIFIED]` at the rungs four UT61B+
/// captures reached (2026-09-09 to 2026-09-11, issue #19): every Ω rung,
/// every DC V rung, AC V 0 and 2, capacitance 0 and 5, both mV rung 0s, and
/// µA, mA and A 0-1. The rungs left over sit *between* measured ends of an
/// ascending ladder, so they have nowhere else to be: AC V 1 and 3,
/// capacitance 1-4 and 6, mV 1. `hz` is the protocol deck's ladder, seen
/// on a meter at rung 0 only — see
/// `docs/research/ut61-family/verification.md`.
///
/// Key differences from UT61E+ (22,000-count):
/// - DC/AC V: 4 ranges (6V..1000V) vs 4 (2.2V..1000V)
/// - Resistance: 6 ranges (600Ω..60MΩ) vs 7 (220Ω..220MΩ)
/// - Capacitance: 7 ranges (60nF..60mF) vs 8 (22nF..220mF)
/// - µA: 600µA/6000µA vs 220µA/2200µA
/// - mA: 60mA/600mA vs 22mA/220mA
/// - A: 6A + 10A vs 20A + 20A
/// - No temperature, no hFE, no LoZ, no LPF, no AC+DC, no Peak
pub(crate) struct Ut61bPlusTable {
    dc_v: [RangeInfo; 4],
    ac_v: [RangeInfo; 4],
    dc_mv: [RangeInfo; 2],
    ac_mv: [RangeInfo; 2],
    ohm: [RangeInfo; 6],
    capacitance: [RangeInfo; 7],
    hz: [RangeInfo; 6],
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
    pub(crate) fn new() -> Self {
        Self {
            // Four ranges: the V⎓ position starts at 6V. Range byte 0 came
            // back as 6.000 V on a UT61B+ (capture 2026-09-09, step `dcv`:
            // "  0.000", three decimals on a 6,000-count meter). 60mV and
            // 600mV are the separate DC mV mode (0x03) on its own dial
            // position, the same correction the UT61E+ needed.
            dc_v: [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("1000V", "V")],
            // Same structure as DC voltage for AC; the same capture read
            // "  0.037" at range 0 in V~, which the meter showed as volts.
            // Top rung is 1000V, not 750V: the manual's AC table (printed
            // page 27, rendered) reads 6.000V/60.00V/600.0V/1000V for this
            // model, and gives overload protection as 1000V.
            ac_v: [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("1000V", "V")],
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
            // Hz: the protocol deck's UT61B+/UT61D+ ladder, labels as it
            // prints them. The manual gives only a span (10.00Hz~10.00MHz).
            // Rung 0 is [VERIFIED] by 49.98 Hz of mains on the Hz/% position
            // (issue #19); the rest are the deck's, unconfirmed on a meter.
            hz: [
                r("99.99Hz", "Hz"),
                r("999.9Hz", "Hz"),
                r("9.999kHz", "kHz"),
                r("99.99kHz", "kHz"),
                r("999.9kHz", "kHz"),
                r("9.999MHz", "MHz"),
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

/// Dial positions of the UT61B+/UT161B — `[MANUAL]`, UT61+ Series User Manual
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
            Mode::Diode => &self.diode,
            Mode::Continuity => &self.continuity,
            Mode::DcUa => &self.dc_ua,
            Mode::AcUa => &self.ac_ua,
            Mode::DcMa => &self.dc_ma,
            Mode::AcMa => &self.ac_ma,
            Mode::DcA => &self.dc_a,
            Mode::AcA => &self.ac_a,
            // UT61B+ has no temperature, hFE, LoZ, LPF, AC+DC, Peak, Inrush
            Mode::TempC
            | Mode::TempF
            | Mode::Hfe
            | Mode::Live
            | Mode::Ncv
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

    fn table() -> Ut61bPlusTable {
        Ut61bPlusTable::new()
    }

    #[test]
    fn model_name() {
        assert_eq!(table().model_name(), "UNI-T UT61B+");
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
}
