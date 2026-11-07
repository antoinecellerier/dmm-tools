use super::{AC_PEAK_MODES, BASE, ModelTable, RangeInfo, m, r};
use crate::protocol::cycle::{CycleButton, DialPosition, Ring};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT61D+ (and UT161D).
///
/// 6,000-count (3¾ digit) model. Same ranges as UT61B+, but 20A for its
/// 10A, and WITH:
/// - Temperature (TempC/TempF, K-type thermocouple)
/// - LoZ V mode
/// - Peak (P-MAX/P-MIN)
///
/// Does NOT have: hFE, LPF, AC+DC, Inrush.
///
/// Range values from the UT61+ Series User Manual, range index
/// ordering is `[DEDUCED]` (ascending assumed). No UT61D+ has been connected;
/// the V tables follow the UT61B+, whose bottom rung was verified by a
/// capture on 2026-09-09 — see issue #7.
pub(crate) static UT61D_PLUS: ModelTable = ModelTable {
    name: "UNI-T UT61D+",
    entry,
    dial: DIAL,
    peak_modes: AC_PEAK_MODES,
    ..BASE
};

// Four ranges, [DEDUCED] from the UT61B+ capture (2026-09-09):
// same 6,000-count table shape, so range byte 0 is 6V here too and
// 60mV/600mV belong to the mV modes (0x03/0x01) on their own dial
// position. Unverified on a UT61D+ — issue #7.
static DC_V: [RangeInfo; 4] = [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("1000V", "V")];

// Top rung is 1000V, not 750V — the manual's AC table shares its
// UT61B+/UT61D+ column (printed page 27, rendered).
static AC_V: [RangeInfo; 4] = [r("6V", "V"), r("60V", "V"), r("600V", "V"), r("1000V", "V")];

static DC_MV: [RangeInfo; 2] = [r("60mV", "mV"), r("600mV", "mV")];
static AC_MV: [RangeInfo; 2] = [r("60mV", "mV"), r("600mV", "mV")];

// 6 ranges: 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ
static OHM: [RangeInfo; 6] = [
    r("600Ω", "Ω"),
    r("6kΩ", "kΩ"),
    r("60kΩ", "kΩ"),
    r("600kΩ", "kΩ"),
    r("6MΩ", "MΩ"),
    r("60MΩ", "MΩ"),
];

// 7 ranges: 60nF, 600nF, 6µF, 60µF, 600µF, 6mF, 60mF
static CAPACITANCE: [RangeInfo; 7] = [
    r("60nF", "nF"),
    r("600nF", "nF"),
    r("6µF", "µF"),
    r("60µF", "µF"),
    r("600µF", "µF"),
    r("6mF", "mF"),
    r("60mF", "mF"),
];

// Hz: the protocol deck's UT61B+/UT61D+ ladder, labels as it
// prints them. The manual gives only a span (10.00Hz~10.00MHz).
// Same ladder as the UT61B+, whose rung 0 is verified; no
// UT61D+ has confirmed it (issue #7).
static HZ: [RangeInfo; 6] = [
    r("99.99Hz", "Hz"),
    r("999.9Hz", "Hz"),
    r("9.999kHz", "kHz"),
    r("99.99kHz", "kHz"),
    r("999.9kHz", "kHz"),
    r("9.999MHz", "MHz"),
];

static DUTY_CYCLE: [RangeInfo; 1] = [r("Duty", "%")];

// UT61D+ has temperature (K-type thermocouple). Two range bytes,
// split where the manual's resolution changes (0.1° to 1° C,
// 0.2° to 2° F): the protocol deck's UT61B+/UT61D+ table.
static TEMP_C: [RangeInfo; 2] = [r("-40~300°C", "°C"), r("300~1000°C", "°C")];
static TEMP_F: [RangeInfo; 2] = [r("-40~572°F", "°F"), r("572~1832°F", "°F")];

static DIODE: [RangeInfo; 1] = [r("Diode", "V")];

// Continuity: 600Ω range for 6,000-count models
static CONTINUITY: [RangeInfo; 1] = [r("Cont", "Ω")];

// µA: 600µA, 6000µA
static DC_UA: [RangeInfo; 2] = [r("600µA", "µA"), r("6000µA", "µA")];
static AC_UA: [RangeInfo; 2] = [r("600µA", "µA"), r("6000µA", "µA")];

// mA: 60mA, 600mA
static DC_MA: [RangeInfo; 2] = [r("60mA", "mA"), r("600mA", "mA")];
static AC_MA: [RangeInfo; 2] = [r("60mA", "mA"), r("600mA", "mA")];

// A: 6A and 20A, the manual's 6.000A and 20.00A rows and the app's
// UT61D+ asset, kept over the 10A at byte 1 of the protocol deck
// and the vendor PC software (family spec §5.5). The order
// is [DEDUCED] from the UT61B+, whose 6A and 10A sit at 0 and 1
// (issue #19); unconfirmed on a UT61D+ — issue #7.
static DC_A: [RangeInfo; 2] = [r("6A", "A"), r("20A", "A")];
static AC_A: [RangeInfo; 2] = [r("6A", "A"), r("20A", "A")];

// LoZ ACV (0x15): 600V and 1000V ranges (UT61D+ only). The 0x16
// beside it is a clamp's AC A in the protocol deck and the app,
// Hz in the vendor PC software (family spec §3), so ClampAcA stays
// without ranges.
static LOZ_V: [RangeInfo; 2] = [r("600V", "V"), r("1000V", "V")];

/// Dial positions of the UT61D+/UT161D — `[MANUAL]`, UT61+ Series User Manual
/// §VII "Function Dial" (printed page 9), plus §11 for the temperature pair
/// ("Short press the SELECT button to switch between °C and °F").
///
/// Membership only: the driver presses until the target shows, so no ring
/// here claims a press order. No press has been observed on this model; see
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
    // V≂ — this model puts AC and DC volts on one position.
    DialPosition {
        rings: &[
            Ring {
                button: CycleButton::Select,
                modes: &[m(Mode::AcV), m(Mode::DcV)],
            },
            Ring {
                button: CycleButton::Hz,
                modes: &[m(Mode::AcV), m(Mode::Hz), m(Mode::DutyCycle)],
            },
        ],
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
    // °C / °F
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::TempC), m(Mode::TempF)],
        }],
    },
    // LoZ — 0x15, the protocol deck's low-impedance AC voltage (spec §3).
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[m(Mode::LozV)],
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
        // UT61D+ has LoZ V mode
        Mode::LozV => &LOZ_V,
        // UT61D+ has no hFE, no LPF, no AC+DC, and none of the protocol
        // deck's clamp functions
        Mode::Hfe
        | Mode::Live
        | Mode::Ncv
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name() {
        assert_eq!(UT61D_PLUS.name, "UNI-T UT61D+");
    }

    /// The D+ shares the B+'s V ladder: range 0 is 6V, and the mV ranges are
    /// their own modes. [DEDUCED] from the UT61B+ capture — issue #7.
    #[test]
    fn voltage_range_0_is_volts_not_millivolts() {
        let t = &UT61D_PLUS;
        for mode in [Mode::DcV, Mode::AcV] {
            let r0 = t.range_info(mode, 0).unwrap();
            assert_eq!(r0.label, "6V", "{mode:?} range 0");
            assert_eq!(r0.unit, "V", "{mode:?} range 0");
        }
        assert_eq!(t.range_info(Mode::DcMv, 0).unwrap().label, "60mV");
        assert_eq!(t.range_info(Mode::AcMv, 0).unwrap().label, "60mV");
    }
}
