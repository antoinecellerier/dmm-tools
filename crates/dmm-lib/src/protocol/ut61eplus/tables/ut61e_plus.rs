use super::{AC_PEAK_MODES, DeviceTable, RangeInfo, m, r};
use crate::protocol::cycle::{CycleButton, DialPosition, Ring};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61E+.
pub(crate) struct Ut61ePlusTable {
    // Tables indexed by range byte (0x00..0x07 typically)
    dc_v: [RangeInfo; 4],
    ac_v: [RangeInfo; 4],
    dc_mv: [RangeInfo; 1],
    ac_mv: [RangeInfo; 1],
    ohm: [RangeInfo; 7],
    capacitance: [RangeInfo; 8],
    hz: [RangeInfo; 8],
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
    pub(crate) fn new() -> Self {
        Self {
            // Four ranges, verified on a real UT61E+ (2026-03-21): the
            // meter never sends a fifth. The vendor software listed a
            // 220mV entry here, but 220 mV is the separate DC mV mode
            // (0x03) on its own dial position, not a range of mode 0x02.
            dc_v: [
                r("2.2V", "V"),
                r("22V", "V"),
                r("220V", "V"),
                r("1000V", "V"),
            ],
            // Top rung is 1000.0V, not the 750V this table carried from the
            // bootstrap commit: the UT61+ Series User Manual's AC table
            // (printed page 27, rendered) lists 1000.0V at 0.1V resolution
            // with no 750V row, and gives max input and overload protection as
            // 1000V. The 2026-09-07 RANGE walk read one decimal there, which
            // is that row's resolution. The walk cannot tell rung 3 from a
            // 750.0 V one, which would carry the same 0.1 V; its label is the
            // manual's.
            ac_v: [
                r("2.2V", "V"),
                r("22V", "V"),
                r("220V", "V"),
                r("1000V", "V"),
            ],
            // One range on this model, and RANGE does nothing in either mV
            // mode — DC mV verified 2026-03-21, AC mV 2026-09-07 (three
            // presses, range byte and AUTO annunciator unmoved); see
            // ut61-family spec §9. The protocol deck's UT61E+ table has this
            // one rung too.
            dc_mv: [r("220mV", "mV")],
            ac_mv: [r("220mV", "mV")],
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
                // Range bytes 0x35-0x37, from the protocol deck's UT61E+
                // table; the manual's span runs to 220MHz.
                r("2.2MHz", "MHz"),
                r("22MHz", "MHz"),
                r("220MHz", "MHz"),
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

/// Dial positions of the UT61E+/UT161E — `[MANUAL]`, UT61+ Series User Manual
/// §VII "Function Dial" (printed page 9).
///
/// Membership only: the driver presses until the target shows, so no ring
/// here claims a press order. What SELECT and Hz/% were observed to do on
/// this model is recorded in
/// `docs/research/ut61-family/reverse-engineered-protocol.md` §3.1.
pub(super) const DIAL: &[DialPosition] = &[
    // Hz/% — the dedicated frequency position, listed first because it is the
    // smallest one reaching Hz and Duty %, which every ring below also reaches.
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Hz,
            modes: &[m(Mode::Hz), m(Mode::DutyCycle)],
        }],
    },
    // V~
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::AcV), m(Mode::LpfV)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcV), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
    },
    // V⎓
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::DcV), m(Mode::AcDcV)],
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
    // Ω / continuity / diode / capacitance
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[
                m(Mode::Ohm),
                m(Mode::Continuity),
                m(Mode::Diode),
                m(Mode::Capacitance),
            ],
        }],
    },
    // hFE
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::Hfe)],
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

impl DeviceTable for Ut61ePlusTable {
    fn model_name(&self) -> &'static str {
        "UNI-T UT61E+"
    }

    fn dial_positions(&self) -> &'static [DialPosition] {
        DIAL
    }

    fn peak_modes(&self) -> &'static [Mode] {
        AC_PEAK_MODES
    }

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
            Mode::DcA => &self.dc_a,
            Mode::AcA => &self.ac_a,
            Mode::Hfe => &self.hfe,
            // Derived modes share their base mode's range table.
            Mode::AcDcV | Mode::LpfV | Mode::LozV => &self.dc_v,
            // Modes without range tables: NCV, and the protocol deck's
            // current and clamp variants, which no dial position reaches
            // and whose ranges it does not give.
            Mode::Ncv
            | Mode::Live
            | Mode::ClampAcA
            | Mode::ClampDcA
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

    #[test]
    fn model_name() {
        assert_eq!(Ut61ePlusTable::new().model_name(), "UNI-T UT61E+");
    }
}
