use super::{AC_PEAK_MODES, BASE, ModelTable, RangeInfo, m, r};
use crate::protocol::cycle::{CycleButton, DialPosition, Ring};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61E+.
pub(crate) static UT61E_PLUS: ModelTable = ModelTable {
    name: "UNI-T UT61E+",
    entry,
    dial: DIAL,
    peak_modes: AC_PEAK_MODES,
    ..BASE
};

// Four ranges, verified on a real UT61E+ (2026-03-21): the
// meter never sends a fifth. The vendor software's 22,000-count
// list has these four too (2, 22, 220, 1000 V); 220 mV is the
// separate DC mV mode (0x03) on its own dial position, not a
// range of mode 0x02 (ut61-family spec §5.1).
static DC_V: [RangeInfo; 4] = [
    r("2.2V", "V"),
    r("22V", "V"),
    r("220V", "V"),
    r("1000V", "V"),
];

// Top rung is 1000.0V, not the 750V this table carried from the
// bootstrap commit: the UT61+ Series User Manual's AC table
// (printed page 27, rendered) lists 1000.0V at 0.1V resolution
// with no 750V row, and gives max input and overload protection as
// 1000V. The 2026-09-07 RANGE walk read one decimal there, which
// is that row's resolution. The walk cannot tell rung 3 from a
// 750.0 V one, which would carry the same 0.1 V; its label is the
// manual's.
static AC_V: [RangeInfo; 4] = [
    r("2.2V", "V"),
    r("22V", "V"),
    r("220V", "V"),
    r("1000V", "V"),
];

// One range on this model, and RANGE does nothing in either mV
// mode — DC mV verified 2026-03-21, AC mV 2026-09-07 (three
// presses, range byte and AUTO annunciator unmoved); see
// ut61-family spec §9. The protocol deck's UT61E+ table has this
// one rung too.
static DC_MV: [RangeInfo; 1] = [r("220mV", "mV")];
static AC_MV: [RangeInfo; 1] = [r("220mV", "mV")];

static OHM: [RangeInfo; 7] = [
    r("220Ω", "Ω"),
    r("2.2kΩ", "kΩ"),
    r("22kΩ", "kΩ"),
    r("220kΩ", "kΩ"),
    r("2.2MΩ", "MΩ"),
    r("22MΩ", "MΩ"),
    r("220MΩ", "MΩ"),
];

static CAPACITANCE: [RangeInfo; 8] = [
    r("22nF", "nF"),
    r("220nF", "nF"),
    r("2.2µF", "µF"),
    r("22µF", "µF"),
    r("220µF", "µF"),
    r("2.2mF", "mF"),
    r("22mF", "mF"),
    r("220mF", "mF"),
];

static HZ: [RangeInfo; 8] = [
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
];

static DUTY_CYCLE: [RangeInfo; 1] = [r("Duty", "%")];
// No temperature on this model; no vendor source has these one-rung
// rows, theirs being the UT61D+'s two (ut61-family spec §9).
static TEMP_C: [RangeInfo; 1] = [r("Temp", "°C")];
static TEMP_F: [RangeInfo; 1] = [r("Temp", "°F")];
static DIODE: [RangeInfo; 1] = [r("Diode", "V")];
static CONTINUITY: [RangeInfo; 1] = [r("Cont", "Ω")];
static DC_UA: [RangeInfo; 2] = [r("220µA", "µA"), r("2200µA", "µA")];
static AC_UA: [RangeInfo; 2] = [r("220µA", "µA"), r("2200µA", "µA")];
static DC_MA: [RangeInfo; 2] = [r("22mA", "mA"), r("220mA", "mA")];
static AC_MA: [RangeInfo; 2] = [r("22mA", "mA"), r("220mA", "mA")];

static DC_A: [RangeInfo; 2] = [
    // Range 0x00 unknown — may not be used. Placeholder no source
    // backs: the vendor software gives 2 A there, the deck and the
    // app nothing (ut61-family spec §5.5).
    r("20A", "A"),
    // Range 0x01 verified: 20A range (confirmed with bench PSU at 100mA)
    r("20A", "A"),
];

static AC_A: [RangeInfo; 2] = [r("20A", "A"), r("20A", "A")];
// The manual's 1000β; no vendor table gives this rung (ut61-family
// spec §9).
static HFE: [RangeInfo; 1] = [r("1000\u{03B2}", "\u{03B2}")];

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

fn entry(mode: Mode) -> Option<&'static [RangeInfo]> {
    Some(match mode {
        Mode::DcV => &DC_V,
        Mode::AcV => &AC_V,
        Mode::DcMv => &DC_MV,
        Mode::AcMv => &AC_MV,
        Mode::Ohm => &OHM,
        Mode::Capacitance => &CAPACITANCE,
        Mode::Hz => &HZ,
        Mode::DutyCycle => &DUTY_CYCLE,
        Mode::TempC => &TEMP_C,
        Mode::TempF => &TEMP_F,
        Mode::Diode => &DIODE,
        Mode::Continuity => &CONTINUITY,
        Mode::DcUa => &DC_UA,
        Mode::AcUa => &AC_UA,
        Mode::DcMa => &DC_MA,
        Mode::AcMa => &AC_MA,
        Mode::DcA => &DC_A,
        Mode::AcA => &AC_A,
        Mode::Hfe => &HFE,
        // Derived modes share their base mode's range table.
        Mode::AcDcV | Mode::LpfV | Mode::LozV => &DC_V,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name() {
        assert_eq!(UT61E_PLUS.name, "UNI-T UT61E+");
    }
}
