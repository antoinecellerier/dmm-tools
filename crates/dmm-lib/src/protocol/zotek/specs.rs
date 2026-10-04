//! Specification data for the ZOYI ZT-5B (6000 counts), which the ANENG
//! V05B shares its entry with.
//!
//! Transcribed from the ZT-5B manual (`ZT-5B.pdf`, no revision printed),
//! "Electrical Specifications" on PDF page 2 (printed pp. 7-8), with values
//! as printed but for the slips noted where they are. Cross-checked against
//! the ZT-5B product page on zotektools.com, which gives ranges only. The
//! notes are short app notes condensed from the tables' Max column and
//! remarks, and from the operating instructions (PDF pp. 1-2): the current
//! time limit, and two tables of notes only, what makes AUTO pick a
//! function and the continuity beeper.
//!
//! The packets carry no range byte: they are an image of the LCD (spec §6,
//! §7.3), where the range shows only in the unit's prefix and where the
//! point sits. So a manual table is split into parts of the same name, one
//! per unit, and a row's `range` stands for the digits the display shows
//! after the point on that range: 3 on 4.000V, 0 on 600V.

use super::layout::{Coupling, Function};
use crate::measurement::{MeasuredValue, Measurement};
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// A table's parts, each with the unit it answers.
type Parts = &'static [(&'static str, &'static ModeSpecs)];

/// Each function's table, and the coupling that picks it where AC/DC does.
/// The first part's mode data goes to a unit no part answers, such as mV,
/// µA or a bare F.
///
/// The manual gives no table for the diode and NCV, which it ticks without
/// a figure, nor for the duty cycle; and V or A with neither AC nor DC lit
/// (or both, which the decoder reports) has no table to pick.
static TABLES: &[(Function, Option<Coupling>, Parts)] = &[
    (Function::None, None, &[("", &AUTO)]),
    (Function::Volts, Some(Coupling::Dc), &[("V", &DC_V)]),
    (Function::Volts, Some(Coupling::Ac), &[("V", &AC_V)]),
    (
        Function::Amps,
        Some(Coupling::Dc),
        &[("mA", &DC_MA), ("A", &DC_A)],
    ),
    (
        Function::Amps,
        Some(Coupling::Ac),
        &[("mA", &AC_MA), ("A", &AC_A)],
    ),
    (
        Function::Ohms,
        None,
        &[("kΩ", &RESISTANCE_KOHM), ("MΩ", &RESISTANCE_MOHM)],
    ),
    (Function::Continuity, None, &[("Ω", &CONTINUITY)]),
    (
        Function::Capacitance,
        None,
        &[
            ("nF", &CAPACITANCE_NF),
            ("µF", &CAPACITANCE_UF),
            ("mF", &CAPACITANCE_MF),
        ],
    ),
    (
        Function::Frequency,
        None,
        &[
            ("Hz", &FREQUENCY_HZ),
            ("kHz", &FREQUENCY_KHZ),
            ("MHz", &FREQUENCY_MHZ),
        ],
    ),
    (Function::Celsius, None, &[("°C", &TEMPERATURE)]),
    (Function::Fahrenheit, None, &[("°F", &TEMPERATURE)]),
];

/// The table part for ZT-5B reading `m`, and whether its rows answer the
/// reading's unit. Auto takes its notes only while the meter shows its
/// word: with digits and no function lit, the mode is unknown.
pub(super) fn table(m: &Measurement) -> Option<(&'static ModeSpecs, bool)> {
    let coupling = Coupling::shown_in(m);
    let &(function, _, parts) = TABLES
        .iter()
        .find(|(f, c, _)| f.shown_in(m) && c.is_none_or(|c| c == coupling))?;
    if function == Function::None && !matches!(m.value, MeasuredValue::NoReading(_)) {
        return None;
    }
    Some(match parts.iter().find(|(unit, _)| *unit == m.unit) {
        Some((_, part)) => (part, true),
        None => (parts[0].1, false),
    })
}

/// The row for ZT-5B reading `m`: its part's row for the digits shown after
/// the point. OL and the Auto word have no digits to count, and take their
/// table's mode data only.
pub(super) fn row(m: &Measurement) -> Option<&'static RangeSpec> {
    let (part, true) = table(m)? else {
        return None;
    };
    let MeasuredValue::Normal(_) = m.value else {
        return None;
    };
    let shown = m.display_raw.as_deref()?;
    let decimals = shown.split_once('.').map_or(0, |(_, after)| after.len());
    part.row(u8::try_from(decimals).ok()?)
}

/// Every table, in manual order, the parts of one table together.
pub(super) static ALL: &[&ModeSpecs] = &[
    &AUTO,
    &DC_V,
    &AC_V,
    &AC_MA,
    &AC_A,
    &DC_MA,
    &DC_A,
    &RESISTANCE_KOHM,
    &RESISTANCE_MOHM,
    &CAPACITANCE_NF,
    &CAPACITANCE_UF,
    &CAPACITANCE_MF,
    &FREQUENCY_HZ,
    &FREQUENCY_KHZ,
    &FREQUENCY_MHZ,
    &CONTINUITY,
    &TEMPERATURE,
];

// What makes AUTO pick a function, from the operating instructions (PDF pages
// 1-2, printed pp. 3-6): notes only, while the meter shows "Auto".
static AUTO: ModeSpecs = ModeSpecs {
    name: "Auto",
    page: 1,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Red lead in VΩ: picks V above 0.8V or Ω",
            "Red lead in A mA: picks current",
            "SEL/NCV: continuity/diode, cap, Hz, °C",
            "Hold SEL/NCV for NCV",
        ],
    },
};

static DC_V: ModeSpecs = ModeSpecs {
    name: "DC VOLTAGE (V)",
    page: 2,
    ranges: &[
        // Printed so, yet a ZT-5B showed 4.036 V at 0.001V (issue #31).
        RangeSpec {
            range: Some(3),
            label: "4.000V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "40.00V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "400.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(0),
            label: "600V",
            spec: SpecInfo {
                // Not printed.
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 600V"],
    },
};

static AC_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage (V)",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 600V", "Frequency response 40Hz–1kHz"],
    },
};

static AC_MA: ModeSpecs = ModeSpecs {
    name: "AC current (mA)",
    page: 2,
    ranges: &[
        // 9999 counts, as printed, on a 6000-count meter.
        RangeSpec {
            range: Some(1),
            label: "999.9mA",
            spec: SpecInfo {
                resolution: "0.1mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 9.999A", "Frequency response 40Hz–1kHz"],
    },
};

static AC_A: ModeSpecs = ModeSpecs {
    name: "AC current (A)",
    page: 2,
    ranges: &[
        // 9999 counts, as printed, on a 6000-count meter.
        RangeSpec {
            range: Some(3),
            label: "9.999A",
            spec: SpecInfo {
                resolution: "0.001A",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Max 9.999A",
            "Frequency response 40Hz–1kHz",
            "Over 2A: measure for less than 3s",
        ],
    },
};

static DC_MA: ModeSpecs = ModeSpecs {
    name: "DC current (mA)",
    page: 2,
    ranges: &[
        // 9999 counts, as printed, on a 6000-count meter.
        RangeSpec {
            range: Some(1),
            label: "999.9mA",
            spec: SpecInfo {
                resolution: "0.1mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 9.999A"],
    },
};

static DC_A: ModeSpecs = ModeSpecs {
    name: "DC current (A)",
    page: 2,
    ranges: &[
        // 9999 counts, as printed, on a 6000-count meter.
        RangeSpec {
            range: Some(3),
            label: "9.999A",
            spec: SpecInfo {
                resolution: "0.001A",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 9.999A", "Over 2A: measure for less than 3s"],
    },
};

static RESISTANCE_KOHM: ModeSpecs = ModeSpecs {
    name: "Resistance",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0kΩ",
            spec: SpecInfo {
                resolution: "0.1kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 40MΩ"],
    },
};

static RESISTANCE_MOHM: ModeSpecs = ModeSpecs {
    name: "Resistance",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00MΩ",
            spec: SpecInfo {
                resolution: "0.01MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            // Printed so, below the top row, 60.00MΩ, which the product page gives.
            "Max 40MΩ",
        ],
    },
};

static CAPACITANCE_NF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5.0%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0nF",
            spec: SpecInfo {
                resolution: "0.1nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 4mF"],
    },
};

static CAPACITANCE_UF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0µF",
            spec: SpecInfo {
                resolution: "0.1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 4mF"],
    },
};

static CAPACITANCE_MF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 2,
    ranges: &[RangeSpec {
        range: Some(3),
        label: "6.000mF",
        spec: SpecInfo {
            resolution: "0.001mF",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "5.0%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            // Printed so, below the top row, 6.000mF, which the product page gives.
            "Max 4mF",
        ],
    },
};

static FREQUENCY_HZ: ModeSpecs = ModeSpecs {
    name: "Frequency",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000Hz",
            spec: SpecInfo {
                resolution: "0.001Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00Hz",
            spec: SpecInfo {
                resolution: "0.01Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0Hz",
            spec: SpecInfo {
                resolution: "0.1Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 10MHz"],
    },
};

// Labels and resolutions printed "KHz".
static FREQUENCY_KHZ: ModeSpecs = ModeSpecs {
    name: "Frequency",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0kHz",
            spec: SpecInfo {
                resolution: "0.1kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 10MHz"],
    },
};

static FREQUENCY_MHZ: ModeSpecs = ModeSpecs {
    name: "Frequency",
    page: 2,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "10.00MHz",
            spec: SpecInfo {
                resolution: "0.01MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+2",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Max 10MHz"],
    },
};

// From the operating instructions (PDF page 1, printed p. 4): notes only.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "Continuity",
    page: 1,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Beeps <50Ω"],
    },
};

static TEMPERATURE: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 2,
    ranges: &[RangeSpec {
        range: None,
        label: "-20℃-1000℃/-4℉-1832℉",
        spec: SpecInfo {
            // Not printed.
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "3%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[],
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::zotek::ZotekProtocol;
    use crate::protocol::zotek::frame::tests::EXAMPLES;
    use crate::protocol::zotek::glyph::{Cell, Glyph};
    use crate::protocol::zotek::layout::{self, Meaning, Prefix, Unit, ZT5B};
    use crate::protocol::{Protocol, test_support::unit_family};
    use std::collections::HashSet;

    /// Why a reading has no row or no spec, and which readings that is.
    type Listed = (&'static str, fn(&Measurement) -> bool);

    /// The modes with a table of ranges.
    fn ranged(m: &Measurement) -> bool {
        matches!(
            m.mode.as_ref(),
            "DC V" | "AC V" | "DC A" | "AC A" | "Ω" | "Capacitance" | "Hz" | "°C" | "°F"
        )
    }

    /// The digits after the point a part's rows answer, by mode and unit,
    /// as the manual's range labels give them; `None` for a unit with no
    /// part.
    fn decimals_printed(m: &Measurement) -> Option<&'static [usize]> {
        Some(match (m.mode.as_ref(), m.unit.as_ref()) {
            ("DC V", "V") => &[0, 1, 2, 3],
            ("AC V", "V") => &[1, 2, 3],
            ("DC A" | "AC A", "mA") => &[1],
            ("DC A" | "AC A", "A") => &[3],
            ("Ω", "kΩ") => &[1, 2, 3],
            ("Ω", "MΩ") => &[2, 3],
            ("Capacitance", "nF" | "µF") => &[1, 2, 3],
            ("Capacitance", "mF") => &[3],
            ("Hz", "Hz" | "kHz") => &[1, 2, 3],
            ("Hz", "MHz") => &[2, 3],
            ("°C", "°C") | ("°F", "°F") => &[0, 1, 2, 3],
            _ => return None,
        })
    }

    fn decimals(m: &Measurement) -> usize {
        let shown = m.display_raw.as_deref().unwrap();
        shown.split_once('.').map_or(0, |(_, after)| after.len())
    }

    fn number(m: &Measurement) -> bool {
        matches!(m.value, MeasuredValue::Normal(_))
    }

    /// Readings that take their table's mode data but no row.
    const MODE_SPEC_ONLY: &[Listed] = &[
        (
            "Auto and continuity: notes from the operating instructions, no figures",
            |m| matches!(m.mode.as_ref(), "Auto" | "Continuity"),
        ),
        (
            "OL, or the Auto word in a function: no digits to tell the range by",
            |m| ranged(m) && !number(m),
        ),
        (
            "a unit no table prints, such as mV, µA, a bare F or Ω",
            |m| ranged(m) && number(m) && decimals_printed(m).is_none(),
        ),
        (
            "a point no range of the unit puts there, such as Hz with none",
            |m| number(m) && decimals_printed(m).is_some_and(|d| !d.contains(&decimals(m))),
        ),
    ];

    /// Readings the decoder accepts that have no spec in the manual.
    const NO_SPEC: &[Listed] = &[
        ("diode: the manual ticks it, with no figure", |m| {
            m.mode == "Diode"
        }),
        ("NCV: the manual ticks it, with no figure", |m| {
            m.mode == "NCV"
        }),
        ("duty cycle: no table", |m| m.mode == "Duty %"),
        ("V or A with neither AC nor DC lit: no table to pick", |m| {
            matches!(m.mode.as_ref(), "V" | "A")
        }),
    ];

    /// `glyphs` as a row of cells, the point before cell `dp_at`.
    fn cells(glyphs: [Glyph; 4], dp_at: Option<usize>) -> [Cell; 4] {
        let mut i = 0;
        glyphs.map(|glyph| {
            i += 1;
            Cell {
                glyph,
                dp: dp_at == Some(i - 1),
            }
        })
    }

    /// What the display can show: a number with 0-3 digits after the
    /// point, OL, the Auto word, EF and NCV's dashes.
    fn displays() -> Vec<[Cell; 4]> {
        let digits = [1, 2, 3, 4].map(Glyph::Digit);
        let mut out: Vec<_> = [None, Some(1), Some(2), Some(3)]
            .into_iter()
            .map(|dp| cells(digits, dp))
            .collect();
        let ol = [Glyph::Blank, Glyph::Digit(0), Glyph::L, Glyph::Blank];
        out.push(cells(ol, Some(2)));
        out.push(cells([Glyph::A, Glyph::U, Glyph::T, Glyph::O], None));
        out.push(cells(
            [Glyph::Blank, Glyph::E, Glyph::F, Glyph::Blank],
            None,
        ));
        out.push(cells(
            [Glyph::Dash, Glyph::Dash, Glyph::Blank, Glyph::Blank],
            None,
        ));
        out
    }

    /// Every unit, prefix, coupling and symbol over every display, as the
    /// ZT-5B layout draws them, kept where the decoder takes the packet
    /// without a report.
    fn accepted_readings() -> Vec<Measurement> {
        let units = [
            None,
            Some(Unit::Volt),
            Some(Unit::Amp),
            Some(Unit::Ohm),
            Some(Unit::Farad),
            Some(Unit::Hertz),
            Some(Unit::Percent),
            Some(Unit::Celsius),
            Some(Unit::Fahrenheit),
        ];
        let prefixes = [
            None,
            Some(Prefix::Nano),
            Some(Prefix::Micro),
            Some(Prefix::Milli),
            Some(Prefix::Kilo),
            Some(Prefix::Mega),
        ];
        let couplings: [&[Meaning]; 4] = [
            &[],
            &[Meaning::Dc],
            &[Meaning::Ac],
            &[Meaning::Dc, Meaning::Ac],
        ];
        let symbols = [None, Some(Meaning::Diode), Some(Meaning::Continuity)];
        let mut readings = Vec::new();
        for display in displays() {
            for unit in units {
                for prefix in prefixes {
                    for coupling in couplings {
                        for symbol in symbols {
                            let mut lit: Vec<Meaning> = coupling.to_vec();
                            lit.extend(unit.map(Meaning::Unit));
                            lit.extend(prefix.map(|p| Meaning::Prefix(p, &[])));
                            lit.extend(symbol);
                            let packet = ZT5B.draw(&display, false, &lit).unwrap();
                            if let (Ok(m), reports) = capture_reports(|| layout::decode(&packet))
                                && reports.is_empty()
                            {
                                readings.push(m);
                            }
                        }
                    }
                }
            }
        }
        readings
    }

    fn what(m: &Measurement) -> String {
        format!("{} {:?} in {}", m.mode, m.display_raw, m.unit)
    }

    /// Each reading resolves a row, is listed as taking its table's mode
    /// data only, or is listed as having none; every row of every table is
    /// some reading's.
    #[test]
    fn every_reading_has_a_spec_or_is_listed() {
        let proto = ZotekProtocol::new_zt5b();
        let lists = [MODE_SPEC_ONLY, NO_SPEC];
        let mut rows_reached = HashSet::new();
        let mut listed_reached = HashSet::new();
        for m in accepted_readings() {
            let listed: Vec<(usize, &str)> = lists
                .iter()
                .enumerate()
                .flat_map(|(l, list)| list.iter().map(move |entry| (l, entry)))
                .filter(|(_, (_, hit))| hit(&m))
                .map(|(l, (why, _))| (l, *why))
                .collect();
            let has_mode_spec = proto.mode_spec_info(&m).is_some();
            match (row(&m), listed.as_slice()) {
                (Some(row), []) => {
                    assert!(std::ptr::eq(proto.spec_info(&m).unwrap(), &row.spec));
                    assert!(has_mode_spec, "{}", what(&m));
                    rows_reached.insert(std::ptr::from_ref(row));
                }
                (None, [(l, why)]) => {
                    assert!(proto.spec_info(&m).is_none(), "{}", what(&m));
                    // The first list keeps the mode data, the second has none.
                    assert_eq!(has_mode_spec, *l == 0, "{}: {why}", what(&m));
                    listed_reached.insert(*why);
                }
                (Some(_), _) => panic!("{}: has a row, yet is listed", what(&m)),
                (None, _) => panic!("{}: no row, and not listed once", what(&m)),
            }
        }
        for (why, _) in lists.iter().flat_map(|list| list.iter()) {
            assert!(listed_reached.contains(why), "no reading is {why}");
        }
        for table in ALL {
            for row in table.ranges {
                assert!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    "{} / {} is no reading's",
                    table.name,
                    row.label
                );
            }
        }
    }

    /// A row's label is in the reading's unit, with as many digits after
    /// the point as the display shows. The temperature row spans its
    /// function.
    #[test]
    fn rows_carry_the_readings_unit_and_point() {
        for m in accepted_readings() {
            let Some(row) = row(&m) else {
                continue;
            };
            if row.range.is_none() {
                continue;
            }
            let unit = row
                .label
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            let shown = row.label[..row.label.len() - unit.len()]
                .split_once('.')
                .map_or(0, |(_, after)| after.len());
            assert_eq!(unit, m.unit, "{}: row {}", what(&m), row.label);
            assert_eq!(shown, decimals(&m), "{}: row {}", what(&m), row.label);
        }
    }

    /// A DC table answers only DC readings, an AC table only AC ones; the
    /// other tables answer readings whose mode names no coupling.
    #[test]
    fn tables_match_the_readings_coupling() {
        for m in accepted_readings() {
            let Some((table, _)) = table(&m) else {
                continue;
            };
            let named = |coupling: &str| m.mode.starts_with(coupling);
            let fits = match &table.name[..2] {
                "DC" => named("DC ") && m.flags.dc,
                "AC" => named("AC "),
                _ => !named("DC ") && !named("AC "),
            };
            assert!(fits, "{}: {}", what(&m), table.name);
        }
        // AC and DC lit together, which the decoder reports, has no table.
        let digits = cells([1, 2, 3, 4].map(Glyph::Digit), Some(1));
        let lit = [Meaning::Unit(Unit::Volt), Meaning::Dc, Meaning::Ac];
        let packet = ZT5B.draw(&digits, false, &lit).unwrap();
        let (m, reports) = capture_reports(|| layout::decode(&packet));
        let m = m.unwrap();
        assert_eq!((m.mode.as_ref(), reports.len()), ("AC+DC V", 1));
        assert!(table(&m).is_none());
    }

    /// A row's resolution is in the reading's unit, prefix aside.
    #[test]
    fn rows_resolve_in_the_readings_unit() {
        for m in accepted_readings() {
            let Some(row) = row(&m) else {
                continue;
            };
            // Not printed for the row.
            if row.spec.resolution.is_empty() {
                continue;
            }
            let unit = row
                .spec
                .resolution
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            assert_eq!(
                unit_family(unit),
                unit_family(&m.unit),
                "{}: row {} resolves in {}",
                what(&m),
                row.label,
                row.spec.resolution
            );
        }
    }

    /// Only the ZT-5B entry has the tables, and only for the ZT-5B's own
    /// packets: an entry decodes another layout's packets too.
    #[test]
    fn only_a_zt5b_packet_has_specs() {
        let zt5b = ZotekProtocol::new_zt5b();
        let (_, plain) = EXAMPLES[2];
        let ohms = layout::decode(plain).unwrap();
        assert_eq!(zt5b.spec_info(&ohms).unwrap().resolution, "0.001kΩ");
        assert!(zt5b.mode_spec_info(&ohms).is_some());
        assert_eq!(zt5b.spec_sheet().len(), ALL.len());

        // The ZT-300AB's -12.34 V DC: a row on a ZT-5B, by unit and point.
        let (_, plain) = EXAMPLES[0];
        let other = layout::decode(plain).unwrap();
        assert_eq!(other.mode, "DC V");
        assert!(zt5b.spec_info(&other).is_none());
        assert!(zt5b.mode_spec_info(&other).is_none());

        let zt300ab = ZotekProtocol::new_zt300ab();
        assert!(zt300ab.spec_info(&ohms).is_none());
        assert!(zt300ab.mode_spec_info(&ohms).is_none());
        assert!(zt300ab.spec_sheet().is_empty());
    }
}
