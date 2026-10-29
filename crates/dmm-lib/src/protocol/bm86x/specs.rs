//! Specification data for the BM820s (BM829s, BM827s, BM822s, BM821s) and
//! the BM520s (BM525s, BM521s): one manual and one spec section cover all
//! six models, so the `bm82x` and `bm52x` entries share these tables.
//!
//! Transcribed from the BM820s/520s manual (no revision printed; back cover
//! "COPYRIGHT © MMXXIII BTC, P/N: 7M1C-1301-B000"), "Specifications" on PDF
//! pages 19-23 (printed 18-22), with values as printed; the overload
//! protections are those PDF p.19 gives each input. Cross-checked against
//! Minipa's ET-2517A/ET-2615A manual (Portuguese) and Brymen's six product
//! pages: where Minipa prints another figure, a comment says so and the row
//! keeps Brymen's. The manual prints no resolution column. The notes are
//! short app notes condensed from the tables' remarks and the general
//! specifications (PDF pp.19-20). Crest and Record mode are tables of notes
//! only, which no reading takes.
//!
//! The replies carry no range or function code: they are an image of the
//! LCD (spec §5-§8), where the range shows only in the unit's prefix and
//! where the point sits. So a manual table is split into parts of the same
//! name, one per unit, and a row's `range` stands for the digits the display
//! shows after the point on that range: 3 on 9.999V, 0 on 6000µA. Line-level
//! frequency takes its row from the AC reading on the secondary display.

use super::decode::LABEL_AC;
use super::map::{self, Ann, Lit};
use super::reply::REPLY_LEN;
use crate::measurement::{MeasuredValue, Measurement};
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// A table's parts, each with the unit it answers. Where two parts share a
/// unit, the one with a row for the digits shown answers; the first part's
/// mode data goes to a unit no part answers.
type Parts = &'static [(&'static str, &'static ModeSpecs)];

/// DC mV is split by range, whose input capacitances differ. A point
/// neither range puts there, or no number, takes the 60.00mV part's mode
/// data, listed first.
static DC_VOLTAGE: Parts = &[("mV", &DC_V_60MV), ("mV", &DC_V_600MV), ("V", &DC_V_V)];
static AC_VOLTAGE: Parts = &[("mV", &AC_V_MV), ("V", &AC_V_V)];
static DC_AC_VOLTAGE: Parts = &[("mV", &DC_AC_V_MV), ("V", &DC_AC_V_V)];
static AUTOCHECK_DC_VOLTAGE: Parts = &[("V", &AUTOCHECK_DCV)];
static AUTOCHECK_AC_VOLTAGE: Parts = &[("V", &AUTOCHECK_ACV)];
static OHMS: Parts = &[
    ("Ω", &OHMS_OHM),
    ("kΩ", &OHMS_KOHM),
    ("MΩ", &OHMS_MOHM),
    ("nS", &OHMS_NS),
];
static AUTOCHECK_OHMS: Parts = &[
    ("Ω", &AUTOCHECK_OHMS_OHM),
    ("kΩ", &AUTOCHECK_OHMS_KOHM),
    ("MΩ", &AUTOCHECK_OHMS_MOHM),
];
static CAPACITANCE: Parts = &[
    ("nF", &CAPACITANCE_NF),
    ("µF", &CAPACITANCE_UF),
    ("mF", &CAPACITANCE_MF),
];
static AC_CURRENT: Parts = &[
    ("µA", &AC_CURRENT_UA),
    ("mA", &AC_CURRENT_MA),
    ("A", &AC_CURRENT_A),
];
static DC_CURRENT: Parts = &[
    ("µA", &DC_CURRENT_UA),
    ("mA", &DC_CURRENT_MA),
    ("A", &DC_CURRENT_A),
];
/// By the unit of the AC reading the frequency is taken on.
static LINE_LEVEL: Parts = &[
    ("mV", &LINE_HZ_MV),
    ("V", &LINE_HZ_V),
    ("µA", &LINE_HZ_UA),
    ("mA", &LINE_HZ_MA),
    ("A", &LINE_HZ_A),
];
static LOGIC_LEVEL: Parts = &[("Hz", &LOGIC_HZ), ("kHz", &LOGIC_HZ), ("MHz", &LOGIC_HZ)];
static DUTY: Parts = &[("%", &LOGIC_DUTY)];
static DIODE_TEST: Parts = &[("V", &DIODE)];
static TEMPERATURE: Parts = &[("°C", &TEMPERATURE_C), ("°F", &TEMPERATURE_F)];
static AUDIBLE_CONTINUITY: Parts = &[("Ω", &CONTINUITY)];
static DECIBELS: Parts = &[("dBm", &DBM)];
static FIELD_DETECTION: Parts = &[("", &EF)];

/// The table part for reading `m`, and the number whose digits after the
/// point pick its row: `None` where no row answers, as for OL or a unit no
/// part has. `None` for no table: T1-T2, AutoCheck's word, Recall, a word
/// with no function lit, and Hz beside anything but its AC reading.
pub(super) fn table(m: &Measurement) -> Option<(&'static ModeSpecs, Option<&str>)> {
    let ac = beside_ac(m);
    let main = (m.unit.as_ref(), number(&m.value, m.display_raw.as_deref()));
    let (parts, (unit, digits)): (Parts, (&str, Option<&str>)) = match m.mode.as_ref() {
        "DC V" | "DC mV" if ac => (DC_AC_VOLTAGE, main),
        "DC V" | "DC mV" => (DC_VOLTAGE, main),
        "AC+DC V" | "AC+DC mV" => (DC_AC_VOLTAGE, main),
        "AC V" | "AC mV" => (AC_VOLTAGE, main),
        "LoZ DC V" => (AUTOCHECK_DC_VOLTAGE, main),
        "LoZ AC V" => (AUTOCHECK_AC_VOLTAGE, main),
        "LoZ Ω" => (AUTOCHECK_OHMS, main),
        "Ω" | "nS" => (OHMS, main),
        "Capacitance" => (CAPACITANCE, main),
        "Continuity" => (AUDIBLE_CONTINUITY, main),
        // The manual has no table for DC current beside its AC: the DC
        // table's mode data only.
        "DC µA" | "DC mA" | "DC A" if ac => (DC_CURRENT, (main.0, None)),
        "DC µA" | "DC mA" | "DC A" => (DC_CURRENT, main),
        "AC µA" | "AC mA" | "AC A" | "AC+DC µA" | "AC+DC mA" | "AC+DC A" => (AC_CURRENT, main),
        // Line level: the AC reading the frequency is taken on picks the
        // row by its unit and range. The current rows answer a reply no
        // dial position is drawn showing: at A and µA, Hz goes on the
        // secondary display (BM820s manual p.12).
        "Hz" if ac => match m.aux_values.first() {
            Some(aux) => {
                let shown = number(&aux.value, aux.display_raw.as_deref());
                (LINE_LEVEL, (aux.unit.as_ref(), main.1.and(shown)))
            }
            // Dashes: the unit lit beside them picks the part.
            None => (LINE_LEVEL, (secondary_unit(m), None)),
        },
        // Logic level: Hz alone, at the mV position (BM820s manual p.8).
        "Hz" if m.aux_values.is_empty() => (LOGIC_LEVEL, main),
        "Duty %" => (DUTY, main),
        "Diode" => (DIODE_TEST, main),
        "T1" | "T2" => (TEMPERATURE, main),
        "dBm" | "dBm reference" => (DECIBELS, main),
        "EF" => (FIELD_DETECTION, main),
        _ => return None,
    };
    let mut same_unit = parts.iter().filter(|(u, _)| *u == unit);
    let with_row = same_unit
        .clone()
        .find(|(_, part)| digits.is_some_and(|shown| part.row(decimals(shown)).is_some()));
    Some(match with_row.or_else(|| same_unit.next()) {
        Some((_, part)) => (part, digits),
        None => (parts[0].1, None),
    })
}

/// The row for reading `m`: its part's row for the digits shown after the
/// point.
pub(super) fn row(m: &Measurement) -> Option<&'static RangeSpec> {
    let (part, Some(shown)) = table(m)? else {
        return None;
    };
    part.row(decimals(shown))
}

/// The digits after the point, as a row's `range` counts them.
fn decimals(shown: &str) -> u8 {
    let after = shown.split_once('.').map_or(0, |(_, after)| after.len());
    u8::try_from(after).unwrap_or(u8::MAX)
}

/// A reading's digits, where it shows a number.
fn number<'a>(value: &MeasuredValue, shown: Option<&'a str>) -> Option<&'a str> {
    matches!(value, MeasuredValue::Normal(_))
        .then_some(shown)
        .flatten()
}

/// Whether the secondary display shows the AC component: a reading labelled
/// AC, or its ∿ lit beside dashes, as for a moment after a range change
/// (spec §7.3), which leave no reading.
fn beside_ac(m: &Measurement) -> bool {
    match m.aux_values.first() {
        Some(aux) => aux.label == LABEL_AC,
        None => secondary_lit(m).is_some_and(|lit| lit.has(Ann::Ac2)),
    }
}

/// What the reply behind `m` lights, re-read from its payload.
fn secondary_lit(m: &Measurement) -> Option<Lit> {
    (m.raw_payload.len() == REPLY_LEN).then(|| Lit::read(&map::BM820, &m.raw_payload))
}

/// The voltage or current unit the secondary display lights; empty for
/// none, or for one no range has.
fn secondary_unit(m: &Measurement) -> &'static str {
    let Some(lit) = secondary_lit(m) else {
        return "";
    };
    let prefix = (lit.has(Ann::Milli2), lit.has(Ann::Micro2));
    match (lit.has(Ann::V2), lit.has(Ann::A2), prefix) {
        (true, false, (false, false)) => "V",
        (true, false, (true, false)) => "mV",
        (false, true, (false, true)) => "µA",
        (false, true, (true, false)) => "mA",
        (false, true, (false, false)) => "A",
        _ => "",
    }
}

/// Every table, in manual order, the parts of one table together.
pub(super) static ALL: &[&ModeSpecs] = &[
    &DC_V_60MV,
    &DC_V_600MV,
    &DC_V_V,
    &DBM,
    &AC_V_MV,
    &AC_V_V,
    &AUTOCHECK_DCV,
    &DC_AC_V_MV,
    &DC_AC_V_V,
    &AUTOCHECK_ACV,
    &OHMS_OHM,
    &OHMS_KOHM,
    &OHMS_MOHM,
    &OHMS_NS,
    &AUTOCHECK_OHMS_OHM,
    &AUTOCHECK_OHMS_KOHM,
    &AUTOCHECK_OHMS_MOHM,
    &CAPACITANCE_NF,
    &CAPACITANCE_UF,
    &CAPACITANCE_MF,
    &CONTINUITY,
    &AC_CURRENT_UA,
    &AC_CURRENT_MA,
    &AC_CURRENT_A,
    &DC_CURRENT_UA,
    &DC_CURRENT_MA,
    &DC_CURRENT_A,
    &LINE_HZ_MV,
    &LINE_HZ_V,
    &LINE_HZ_UA,
    &LINE_HZ_MA,
    &LINE_HZ_A,
    &LOGIC_HZ,
    &LOGIC_DUTY,
    &EF,
    &CREST,
    &RECORD,
    &DIODE,
    &TEMPERATURE_C,
    &TEMPERATURE_F,
];

static DC_V_60MV: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 20,
    ranges: &[RangeSpec {
        range: Some(2),
        label: "60.00mV",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.12%+2",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 50pF nominal"),
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[],
    },
};

static DC_V_600MV: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 20,
    ranges: &[RangeSpec {
        range: Some(1),
        label: "600.0mV",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.06%+2",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 80pF nominal"),
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[],
    },
};

static DC_V_V: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 20,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+2",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 50pF nominal"),
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[],
    },
};

static DBM: ModeSpecs = ModeSpecs {
    name: "dBm",
    page: 20,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 50pF nominal"),
        overload_protection: None,
        notes: &[
            "Range & accuracy depend on ACmV/ACV and reference impedance",
            "600Ω ref, typical: ACmV -42.22 to -02.22dBm, ACV -17.78 to 62.22dBm",
        ],
    },
};

// Minipa's ET-2517A/ET-2615A manual prints "80pF nominal para a faixa de
// 600mV" (600mV only) where Brymen's prints "mV ranges"; the rows keep
// Brymen's.
static AC_V_MV: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "60.00mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.5%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 500Hz"),
                        accuracy: "0.8%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("500Hz ~ 1kHz"),
                        accuracy: "2.0%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 3kHz"),
                        accuracy: "2.0%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("3kHz ~ 20kHz"),
                        accuracy: "2%+3",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.5%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 500Hz"),
                        accuracy: "0.8%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("500Hz ~ 1kHz"),
                        accuracy: "2.0%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 3kHz"),
                        accuracy: "2.0%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("3kHz ~ 20kHz"),
                        accuracy: "2%+3",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 80pF nominal"),
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "60.00mV, 600.0mV, 3kHz ~ 20kHz: specified 30% to 100% of range",
            "True RMS from 10% to 100% of range; BM821s: average sensing",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static AC_V_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.5%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 500Hz"),
                        accuracy: "1.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("500Hz ~ 1kHz"),
                        accuracy: "1.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 3kHz"),
                        accuracy: "3.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("3kHz ~ 20kHz"),
                        accuracy: "3dB",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.5%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 500Hz"),
                        accuracy: "1.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("500Hz ~ 1kHz"),
                        accuracy: "1.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 3kHz"),
                        accuracy: "3.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("3kHz ~ 20kHz"),
                        accuracy: "3dB",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.5%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 500Hz"),
                        accuracy: "2.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("500Hz ~ 1kHz"),
                        accuracy: "2.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 3kHz"),
                        accuracy: "3.0%+4",
                    },
                    AccuracyBand {
                        freq_range: Some("3kHz ~ 20kHz"),
                        accuracy: "Unspec’d",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 50pF nominal"),
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[
            "9.999V, 3kHz ~ 20kHz: specified for 3kHz ~ 15kHz only",
            "True RMS from 10% to 100% of range; BM821s: average sensing",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static AUTOCHECK_DCV: ModeSpecs = ModeSpecs {
    name: "AutoCheck™ (DCV)",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("Initially approx. 3.0kΩ, 165pF nominal"),
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[
            "Lo-Z threshold: > +1.5VDC or < -1.0VDC nominal",
            "Impedance rises within a fraction of a second above 50V (typ.)",
            "Then typically 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 500kΩ@1000V",
        ],
    },
};

static DC_AC_V_MV: ModeSpecs = ModeSpecs {
    name: "DC AC & AC+DC AC Voltage",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "60.00mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("DC, 50Hz ~ 60Hz"),
                        accuracy: "0.7%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "1.0%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 20kHz"),
                        accuracy: "2.2%+6",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("DC, 50Hz ~ 60Hz"),
                        accuracy: "0.7%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "1.0%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 20kHz"),
                        accuracy: "2.2%+6",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 80pF nominal"),
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "BM821s: specified from 10% to 100% of range",
            "60.00mV, 600.0mV, 1kHz ~ 20kHz: specified 30% to 100% of range",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static DC_AC_V_V: ModeSpecs = ModeSpecs {
    name: "DC AC & AC+DC AC Voltage",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("DC, 50Hz ~ 60Hz"),
                        accuracy: "0.7%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "2.2%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 20kHz"),
                        accuracy: "3dB",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("DC, 50Hz ~ 60Hz"),
                        accuracy: "0.7%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "2.2%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 20kHz"),
                        accuracy: "3dB",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("DC, 50Hz ~ 60Hz"),
                        accuracy: "0.7%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "2.2%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz ~ 20kHz"),
                        accuracy: "Unspec’d",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ, 50pF nominal"),
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[
            "BM821s: specified from 10% to 100% of range",
            "9.999V, 1kHz ~ 20kHz: specified for 1kHz ~ 15kHz only",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static AUTOCHECK_ACV: ModeSpecs = ModeSpecs {
    name: "AutoCheck™ (ACV)",
    page: 21,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: Some("50Hz ~ 60Hz"),
                    accuracy: "1.0%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: Some("50Hz ~ 60Hz"),
                    accuracy: "1.0%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: Some("50Hz ~ 60Hz"),
                    accuracy: "1.0%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("Initially approx. 3.0kΩ, 150pF nominal"),
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[
            "Lo-Z threshold: > 3VAC (50/60Hz) nominal",
            "Impedance rises within a fraction of a second above 50V (typ.)",
            "Then typically 18kΩ@100V, 125kΩ@300V, 320kΩ@600V, 460kΩ@1000V",
        ],
    },
};

static OHMS_OHM: ModeSpecs = ModeSpecs {
    name: "Ohms",
    page: 22,
    ranges: &[RangeSpec {
        range: Some(1),
        label: "600.0Ω",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.1%+3",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static OHMS_KOHM: ModeSpecs = ModeSpecs {
    name: "Ohms",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static OHMS_MOHM: ModeSpecs = ModeSpecs {
    name: "Ohms",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000MΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.4%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00MΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static OHMS_NS: ModeSpecs = ModeSpecs {
    name: "Ohms",
    page: 22,
    ranges: &[RangeSpec {
        range: Some(2),
        label: "99.99nS",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.8%+10",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static AUTOCHECK_OHMS_OHM: ModeSpecs = ModeSpecs {
    name: "AutoCheck™ (Ohms)",
    page: 22,
    ranges: &[RangeSpec {
        range: Some(1),
        label: "600.0Ω",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.5%+4",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static AUTOCHECK_OHMS_KOHM: ModeSpecs = ModeSpecs {
    name: "AutoCheck™ (Ohms)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0kΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)"],
    },
};

static AUTOCHECK_OHMS_MOHM: ModeSpecs = ModeSpecs {
    name: "AutoCheck™ (Ohms)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000MΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00MΩ",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)",
            "Open circuit < 1.2VDC (< 1.0VDC on 60.00MΩ)",
        ],
    },
};

static CAPACITANCE_NF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "60.00nF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0nF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Accuracy with film capacitor or better"],
    },
};

static CAPACITANCE_UF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000µF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00µF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0µF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.5%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "Accuracy with film capacitor or better",
            "600.0µF, manual range: not specified below 50.0µF",
        ],
    },
};

static CAPACITANCE_MF: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000mF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "25.00mF",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "6.5%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "Accuracy with film capacitor or better",
            "6.000mF, manual range: not specified below 0.54mF",
            "25.00mF, manual range: not specified below 5.4mF",
        ],
    },
};

static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "Audible Continuity Tester",
    page: 22,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "Audible threshold: between 20Ω and 300Ω",
            "Response time < 100µs",
        ],
    },
};

// Minipa's ET-2517A (BM827s) manual prints burden 1.5mV/mA and 0.04V/A and
// fuses 1A/600V, 10A/600V; Brymen's prints one column for all six models.
static AC_CURRENT_UA: ModeSpecs = ModeSpecs {
    name: "AC & AC+DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600.0µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.6%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "0.8%+4",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(0),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.6%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "0.8%+4",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &[
            "Burden voltage 0.08mV/µA",
            "True RMS from 10% to 100% of range; BM821s: average sensing",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static AC_CURRENT_MA: ModeSpecs = ModeSpecs {
    name: "AC & AC+DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "60.00mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.6%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "0.8%+4",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "1.0%+3",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "1.0%+4",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &[
            "Burden voltage 2.1mV/mA",
            "True RMS from 10% to 100% of range; BM821s: average sensing",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

static AC_CURRENT_A: ModeSpecs = ModeSpecs {
    name: "AC & AC+DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.8%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "0.8%+6",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "10.00A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50Hz ~ 60Hz"),
                        accuracy: "0.8%+6",
                    },
                    AccuracyBand {
                        freq_range: Some("40Hz ~ 1kHz"),
                        accuracy: "0.8%+6",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "11A/1000V DC/AC rms, IR 20kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 11A 1000V"),
        notes: &[
            "Burden voltage 0.02V/A",
            "10A continuous; >10A to 20A: 30s max, 5 min cool-down",
            "True RMS from 10% to 100% of range; BM821s: average sensing",
            "Crest factor < 2:1 at full scale, < 4:1 at half scale",
        ],
    },
};

// Minipa's ET-2517A (BM827s) manual prints burden 1.5mV/mA and 0.04V/A and
// fuses 1A/600V, 10A/600V; Brymen's prints one column for all six models.
static DC_CURRENT_UA: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600.0µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(0),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &["Burden voltage 0.08mV/µA"],
    },
};

static DC_CURRENT_MA: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "60.00mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &["Burden voltage 2.1mV/mA"],
    },
};

static DC_CURRENT_A: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "6.000A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "10.00A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "11A/1000V DC/AC rms, IR 20kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 11A 1000V"),
        notes: &[
            "Burden voltage 0.02V/A",
            "10A continuous; >10A to 20A: 30s max, 5 min cool-down",
        ],
    },
};

static LINE_HZ_MV: ModeSpecs = ModeSpecs {
    name: "Line Level Frequency (~Hz)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "AC 60.00mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "AC 600.0mV",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "AC 60.00mV: 15.00 ~ 50.00kHz, sensitivity 40mV (sine RMS)",
            "AC 600.0mV: 15.00 ~ 50.00kHz, sensitivity 60mV (sine RMS)",
        ],
    },
};

static LINE_HZ_V: ModeSpecs = ModeSpecs {
    name: "Line Level Frequency (~Hz)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "AC 9.999V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "AC 99.99V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "AC 999.9V",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1100V DC/AC rms"),
        notes: &[
            "AC 9.999V: 15.00 ~ 10.00kHz, sensitivity 2.5V (sine RMS)",
            "AC 99.99V: 15.00 ~ 10.00kHz, sensitivity 25V (sine RMS)",
            "AC 999.9V: 15.00 ~ 10.00kHz, sensitivity 100V (sine RMS)",
        ],
    },
};

static LINE_HZ_UA: ModeSpecs = ModeSpecs {
    name: "Line Level Frequency (~Hz)",
    page: 22,
    ranges: &[
        // Minipa's ET-2517A/ET-2615A manual prints sensitivity 45µA.
        RangeSpec {
            range: Some(1),
            label: "AC 600.0µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(0),
            label: "AC 6000µA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &[
            "AC 600.0µA: 15.00 ~ 3.000kHz, sensitivity 200µA (sine RMS)",
            "AC 6000µA: 15.00 ~ 3.000kHz, sensitivity 600µA (sine RMS)",
        ],
    },
};

static LINE_HZ_MA: ModeSpecs = ModeSpecs {
    name: "Line Level Frequency (~Hz)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "AC 60.00mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "AC 600.0mA",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.44A/1000V DC/AC rms, IR 10kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 0.44A 1000V"),
        notes: &[
            "AC 60.00mA: 15.00 ~ 3.000kHz, sensitivity 40mA (sine RMS)",
            "AC 600.0mA: 15.00 ~ 3.000kHz, sensitivity 60mA (sine RMS)",
        ],
    },
};

static LINE_HZ_A: ModeSpecs = ModeSpecs {
    name: "Line Level Frequency (~Hz)",
    page: 22,
    ranges: &[
        RangeSpec {
            range: Some(3),
            label: "AC 6.000A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "AC 10.00A",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "11A/1000V DC/AC rms, IR 20kA, F fuse" (PDF p.19).
        overload_protection: Some("Fuse 11A 1000V"),
        notes: &[
            "AC 6.000A: 15.00 ~ 3.000kHz, sensitivity 4A (sine RMS)",
            "AC 10.00A: 15.00 ~ 3.000kHz, sensitivity 6A (sine RMS)",
        ],
    },
};

static LOGIC_HZ: ModeSpecs = ModeSpecs {
    name: "Logic Level Frequency (⎍Hz) & Duty Cycle (D%)",
    page: 23,
    ranges: &[
        // Minipa's ET-2517A/ET-2615A manual prints 0.004%+4d.
        RangeSpec {
            range: None,
            label: "5.00Hz ~ 1.000MHz",
            spec: SpecInfo {
                resolution: "",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.04%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Sensitivity: 2.5Vp square wave, for 3V & 5V logic family"],
    },
};

static LOGIC_DUTY: ModeSpecs = ModeSpecs {
    name: "Logic Level Frequency (⎍Hz) & Duty Cycle (D%)",
    page: 23,
    ranges: &[RangeSpec {
        range: None,
        label: "0.00% ~ 100.0%",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "3d/kHz+2d",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &[
            "Sensitivity: 2.5Vp square wave, for 3V & 5V logic family",
            "Duty cycle specified for 5Hz ~ 10kHz",
        ],
    },
};

static EF: ModeSpecs = ModeSpecs {
    name: "Non-Contact EF-Detection",
    page: 23,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Typical: - 20V, -- 55V, --- 110V, ---- 220V, ----- 440V",
            "Tolerance: 10V~36V, 23V~83V, 59V~165V, 124V~330V, > 250V",
            "Bar segments & beep tones proportional to field strength",
            "Detects 50/60Hz; antenna at the top end of the meter",
        ],
    },
};

static CREST: ModeSpecs = ModeSpecs {
    name: "Crest mode (Instantaneous Peak Hold)",
    page: 23,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Crest: specified accuracy adds 250 digits for changes > 1.0 ms"],
    },
};

static RECORD: ModeSpecs = ModeSpecs {
    name: "Record mode",
    page: 23,
    ranges: &[],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Record: specified accuracy adds 10 digits for changes > 100 ms"],
    },
};

static DIODE: ModeSpecs = ModeSpecs {
    name: "Diode Tester",
    page: 23,
    ranges: &[RangeSpec {
        range: Some(3),
        label: "2.000V",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.0%+1",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Test current 0.4mA typical", "Open circuit < 3.5VDC"],
    },
};

static TEMPERATURE_C: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 23,
    ranges: &[RangeSpec {
        range: None,
        label: "-50°C to 1000°C",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.3%+2°C",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Type-K thermocouple range & accuracy not included"],
    },
};

static TEMPERATURE_F: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 23,
    ranges: &[RangeSpec {
        range: None,
        label: "-58°F to 1832°F",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.3%+5°F",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V DC/AC rms"),
        notes: &["Type-K thermocouple range & accuracy not included"],
    },
};

#[cfg(test)]
mod tests {
    use super::super::decode::tests::{BLANK, lcd8};
    use super::super::decode::{self, MODES};
    use super::super::reply::tests::ScriptedCable;
    use super::super::{Bm86xProtocol, Series, reply};
    use super::*;
    use crate::protocol::{Protocol, capture_reports, is_unknown_mode, registry};
    use Ann::*;
    use std::collections::HashSet;

    /// Why a reading has no row or no spec, and which readings that is.
    type Listed = (&'static str, fn(&Measurement) -> bool);

    const BM820_SERIES: [Series; 2] = [Series::Bm82x, Series::Bm52x];

    fn number_shown(m: &Measurement) -> bool {
        matches!(m.value, MeasuredValue::Normal(_))
    }

    fn dc_current(m: &Measurement) -> bool {
        matches!(m.mode.as_ref(), "DC µA" | "DC mA" | "DC A")
    }

    /// Hz on the main display beside its AC reading: line level.
    fn line_level(m: &Measurement) -> bool {
        m.mode == "Hz" && beside_ac(m)
    }

    /// The readings whose table has rows to pick from: not DC current
    /// beside its AC, nor Hz beside another reading than its AC.
    fn ranged(m: &Measurement) -> bool {
        let modes = [
            "DC V",
            "DC mV",
            "AC V",
            "AC mV",
            "AC+DC V",
            "AC+DC mV",
            "LoZ DC V",
            "LoZ AC V",
            "LoZ Ω",
            "Ω",
            "nS",
            "Capacitance",
            "DC µA",
            "DC mA",
            "DC A",
            "AC µA",
            "AC mA",
            "AC A",
            "AC+DC µA",
            "AC+DC mA",
            "AC+DC A",
            "Hz",
            "Duty %",
            "Diode",
            "T1",
            "T2",
        ];
        modes.contains(&m.mode.as_ref())
            && !(dc_current(m) && beside_ac(m))
            && !(m.mode == "Hz" && !m.aux_values.is_empty() && !beside_ac(m))
    }

    /// The text whose digits pick the row: line level's AC reading, else
    /// the main one.
    fn keyed(m: &Measurement) -> Option<(&str, &str)> {
        if line_level(m) {
            let aux = m.aux_values.first()?;
            matches!(aux.value, MeasuredValue::Normal(_))
                .then(|| Some((aux.unit.as_ref(), aux.display_raw.as_deref()?)))
                .flatten()
        } else {
            Some((m.unit.as_ref(), m.display_raw.as_deref()?))
        }
    }

    fn decimals(shown: &str) -> usize {
        shown.split_once('.').map_or(0, |(_, after)| after.len())
    }

    /// The digits after the point the manual's range labels give, by mode
    /// and the unit the row is keyed on; every count for a row that spans
    /// its function; `None` for a unit with no part.
    fn decimals_printed(m: &Measurement, unit: &str) -> Option<&'static [usize]> {
        const ANY: &[usize] = &[0, 1, 2, 3];
        let volts = |unit| match unit {
            "mV" => Some(&[1, 2][..]),
            "V" => Some(&[1, 2, 3][..]),
            _ => None,
        };
        let amps = |unit| match unit {
            "µA" => Some(&[0, 1][..]),
            "mA" => Some(&[1, 2][..]),
            "A" => Some(&[2, 3][..]),
            _ => None,
        };
        match m.mode.as_ref() {
            "DC V" | "DC mV" | "AC V" | "AC mV" | "AC+DC V" | "AC+DC mV" | "LoZ DC V"
            | "LoZ AC V" => volts(unit),
            "Ω" | "LoZ Ω" | "nS" => match unit {
                "Ω" => Some(&[1]),
                "kΩ" => Some(&[1, 2, 3]),
                "MΩ" => Some(&[2, 3]),
                "nS" => Some(&[2]),
                _ => None,
            },
            "Capacitance" => match unit {
                "nF" => Some(&[1, 2]),
                "µF" => Some(&[1, 2, 3]),
                "mF" => Some(&[2, 3]),
                _ => None,
            },
            "Hz" if line_level(m) => volts(unit).or_else(|| amps(unit)),
            "Hz" | "Duty %" | "T1" | "T2" => Some(ANY),
            "Diode" => (unit == "V").then_some(&[3]),
            _ => amps(unit),
        }
    }

    /// Readings that take their table's mode data but no row.
    const MODE_SPEC_ONLY: &[Listed] = &[
        (
            "continuity, dBm and EF detection: notes, no accuracy",
            |m| {
                matches!(
                    m.mode.as_ref(),
                    "Continuity" | "dBm" | "dBm reference" | "EF"
                )
            },
        ),
        (
            "DC current beside its AC: the manual has no table for it",
            |m| dc_current(m) && beside_ac(m),
        ),
        (
            "OL, dashes or a word: no digits to tell the range by",
            |m| ranged(m) && !number_shown(m),
        ),
        (
            "line level with its AC reading OL or dashed: no range to tell",
            |m| ranged(m) && number_shown(m) && line_level(m) && keyed(m).is_none(),
        ),
        ("a unit no part has, such as a diode test in mV", |m| {
            ranged(m)
                && number_shown(m)
                && keyed(m).is_some_and(|(unit, _)| decimals_printed(m, unit).is_none())
        }),
        (
            "a point no range of the unit puts there, such as 1234 V",
            |m| {
                ranged(m)
                    && number_shown(m)
                    && keyed(m).is_some_and(|(unit, shown)| {
                        decimals_printed(m, unit).is_some_and(|d| !d.contains(&decimals(shown)))
                    })
            },
        ),
    ];

    /// Readings the decoder accepts that have no spec in the manual.
    const NO_SPEC: &[Listed] = &[
        (
            "T1-T2: the manual gives no accuracy for the difference",
            |m| m.mode == "T1-T2",
        ),
        (
            "AutoCheck's word: waiting for an input, no function yet",
            |m| m.mode == "Auto V",
        ),
        ("Recall: logged data, not a reading", |m| m.mode == "Recall"),
        ("a word with no function lit, such as InEr or rE-O", |m| {
            is_unknown_mode(&m.mode)
        }),
        (
            "Hz beside a frequency, a DC reading, a temperature or %4-20mA: no state the manual shows",
            |m| m.mode == "Hz" && !m.aux_values.is_empty() && !beside_ac(m),
        ),
    ];

    /// Numbers with 0-3 digits after the point, OL and dashes.
    const NUMBERS: &[&str] = &["1234", "123.4", "12.34", "1.234", " 0L ", "----"];

    /// What the main display shows: a number, OL and dashes, a lone point,
    /// and the words of the manuals.
    const OTHER_MAINS: &[&str] = &[" .   ", "Auto", "E.F.  ", "1nEr", "rE-0", "LEFt", "P.028"];

    /// A temperature, its letter in the last digit, and its dashes.
    const TEMPERATURES: &[&str] = &["205C", "205F", "---C"];

    /// Every function the BM820 map lights, over what the main display
    /// shows, with every secondary reading, kept where the decoder takes
    /// the reply without a report. Continuity and LoZ go only beside V and
    /// Ω, the functions they change, and T1 and T2 only with a letter, which
    /// they need; prefixes are those of the manual's ranges, any other being
    /// reported. The secondary readings go beside numbers, OL and dashes.
    fn accepted_readings(series: Series) -> Vec<Measurement> {
        let couplings: [&[Ann]; 4] = [&[], &[Dc1], &[Ac1], &[Dc1, Ac1]];
        let units: [&[Ann]; 20] = [
            &[],
            &[V1],
            &[V1, Milli1],
            &[A1],
            &[A1, Micro1],
            &[A1, Milli1],
            &[Ohm1],
            &[Ohm1, Kilo1],
            &[Ohm1, Mega1],
            &[Farad1, Nano1],
            &[Farad1, Micro1],
            &[Farad1, Milli1],
            &[Hz1],
            &[Hz1, Kilo1],
            &[Hz1, Mega1],
            &[Siemens1, Nano1],
            &[Duty1],
            &[Db],
            &[Db, Milli1],
            &[Db, Milli1, Ohm1],
        ];
        let subs: [(&[Ann], &[&str]); 13] = [
            (&[], &[BLANK]),
            (&[Hz2], NUMBERS),
            (&[Hz2, Kilo2], NUMBERS),
            (&[Ac2, V2], NUMBERS),
            (&[Ac2, V2, Milli2], NUMBERS),
            (&[Ac2, A2, Micro2], NUMBERS),
            (&[Ac2, A2, Milli2], NUMBERS),
            (&[Ac2, A2], NUMBERS),
            (&[Dc2, V2], NUMBERS),
            (&[Dc2, A2, Milli2], NUMBERS),
            (&[T1Sub], &["325C", "325F"]),
            (&[T2Sub], &["325C", "325F"]),
            (&[Loop], NUMBERS),
        ];
        let mut mains: Vec<(Vec<Ann>, &[&str])> = Vec::new();
        for coupling in couplings {
            for unit in units {
                let symbols: &[&[Ann]] = match unit.first() {
                    Some(V1 | Ohm1) => &[&[], &[Continuity], &[LoZ], &[LoZ, Continuity]],
                    _ => &[&[]],
                };
                for symbol in symbols {
                    mains.push(([coupling, unit, symbol].concat(), NUMBERS));
                    mains.push(([coupling, unit, symbol].concat(), OTHER_MAINS));
                }
            }
            for probe in [&[T1][..], &[T2], &[T1, T2]] {
                mains.push(([coupling, probe].concat(), TEMPERATURES));
            }
        }
        let mut readings = Vec::new();
        let mut keep = |data: Vec<u8>| {
            if let (Ok(m), reports) = capture_reports(|| decode::decode(&data, series))
                && reports.is_empty()
            {
                readings.push(m);
            }
        };
        for (anns, shows) in mains {
            for main in shows {
                for (sub_anns, sub_shows) in subs {
                    // A word takes no secondary reading here.
                    if !sub_anns.is_empty() && !NUMBERS.contains(main) && shows != TEMPERATURES {
                        continue;
                    }
                    for sub in sub_shows {
                        keep(lcd8(series, main, sub, &[&anns[..], sub_anns].concat()));
                    }
                }
            }
        }
        // Recall on a logging model: R and C lit (spec §6.5).
        keep(lcd8(series, "12.34", BLANK, &[Record, Crest, Dc1, V1]));
        readings
    }

    fn what(m: &Measurement) -> String {
        let aux = m
            .aux_values
            .first()
            .map(|a| format!(" beside {} {:?} {}", a.label, a.display_raw, a.unit))
            .unwrap_or_default();
        format!("{} {:?} in {}{aux}", m.mode, m.display_raw, m.unit)
    }

    /// Each reading resolves a row, is listed as taking its table's mode
    /// data only, or is listed as having none; every part of every table
    /// and every row is some reading's, but for Crest and Record mode,
    /// whose adders the manual ties to no function.
    #[test]
    fn every_reading_has_a_spec_or_is_listed() {
        let lists = [MODE_SPEC_ONLY, NO_SPEC];
        let mut rows_reached = HashSet::new();
        let mut parts_reached = HashSet::new();
        let mut listed_reached = HashSet::new();
        let mut modes = HashSet::new();
        for series in BM820_SERIES {
            let proto = Bm86xProtocol::new(series);
            for m in accepted_readings(series) {
                modes.insert(m.mode.to_string());
                let listed: Vec<(usize, &str)> = lists
                    .iter()
                    .enumerate()
                    .flat_map(|(l, list)| list.iter().map(move |entry| (l, entry)))
                    .filter(|(_, (_, hit))| hit(&m))
                    .map(|(l, (why, _))| (l, *why))
                    .collect();
                let mode_spec = proto.mode_spec_info(&m);
                if let Some((part, _)) = table(&m) {
                    assert!(std::ptr::eq(mode_spec.unwrap(), &part.mode));
                    parts_reached.insert(std::ptr::from_ref(part));
                }
                match (row(&m), listed.as_slice()) {
                    (Some(row), []) => {
                        assert!(std::ptr::eq(proto.spec_info(&m).unwrap(), &row.spec));
                        assert!(mode_spec.is_some(), "{}", what(&m));
                        rows_reached.insert(std::ptr::from_ref(row));
                    }
                    (None, [(l, why)]) => {
                        assert!(proto.spec_info(&m).is_none(), "{}", what(&m));
                        // The first list keeps the mode data, the second has none.
                        assert_eq!(mode_spec.is_some(), *l == 0, "{}: {why}", what(&m));
                        listed_reached.insert(*why);
                    }
                    (Some(_), _) => panic!("{}: has a row, yet is listed", what(&m)),
                    (None, _) => panic!("{}: no row, and not listed once", what(&m)),
                }
            }
        }
        for (why, _) in lists.iter().flat_map(|list| list.iter()) {
            assert!(listed_reached.contains(why), "no reading is {why}");
        }
        // Crest and Record mode add to whatever accuracy the reading has:
        // the review sheet shows them, no reading takes them.
        let notes_only = [&CREST, &RECORD];
        for part in ALL {
            if notes_only.iter().any(|t| std::ptr::eq(*t, *part)) {
                assert!(!parts_reached.contains(&std::ptr::from_ref(*part)));
                continue;
            }
            assert!(
                parts_reached.contains(&std::ptr::from_ref(*part)),
                "{} {:?} is no reading's",
                part.name,
                part.ranges.first().map(|r| r.label)
            );
            for row in part.ranges {
                assert!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    "{} / {} is no reading's",
                    part.name,
                    row.label
                );
            }
        }
        // Every mode the decoder names occurs, but the BM860s' VFD
        // functions: the BM820 map has no VFD annunciator (spec §6.1).
        for mode in MODES {
            assert_eq!(modes.contains(mode), !mode.starts_with("VFD"), "{mode}");
        }
    }

    /// A row's label is in the unit the row is keyed on, with as many
    /// digits after the point as that reading shows. Rows that span their
    /// function, temperature and logic level, have no range to match.
    #[test]
    fn rows_carry_the_readings_unit_and_point() {
        for series in BM820_SERIES {
            for m in accepted_readings(series) {
                let Some(row) = row(&m) else {
                    continue;
                };
                if row.range.is_none() {
                    continue;
                }
                let (unit, shown) = keyed(&m).unwrap();
                let label = row.label.strip_prefix("AC ").unwrap_or(row.label);
                let label_unit = label.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
                let digits = &label[..label.len() - label_unit.len()];
                assert_eq!(label_unit, unit, "{}: row {}", what(&m), row.label);
                assert_eq!(
                    decimals(digits),
                    decimals(shown),
                    "{}: row {}",
                    what(&m),
                    row.label
                );
            }
        }
    }

    /// The table and row label for a BM82x reply showing `main` and `sub`
    /// with `anns` lit.
    fn lookup(main: &str, sub: &str, anns: &[Ann]) -> (Option<&'static str>, Option<&'static str>) {
        let s = Series::Bm82x;
        let (m, reports) = capture_reports(|| decode::decode(&lcd8(s, main, sub, anns), s));
        assert!(reports.is_empty(), "{main} {sub} {anns:?}: {reports:?}");
        let m = m.unwrap();
        (table(&m).map(|(t, _)| t.name), row(&m).map(|r| r.label))
    }

    /// One reading per range, by the unit and point the display shows.
    #[test]
    fn each_range_by_its_unit_and_point() {
        for (main, sub, anns, label) in [
            ("12.34", BLANK, &[Dc1, V1, Milli1][..], "60.00mV"),
            ("123.4", BLANK, &[Dc1, V1, Milli1], "600.0mV"),
            ("1.234", BLANK, &[Dc1, V1], "9.999V"),
            ("12.34", BLANK, &[Dc1, V1], "99.99V"),
            ("123.4", BLANK, &[Dc1, V1], "999.9V"),
            ("123.4", BLANK, &[Ohm1], "600.0Ω"),
            ("1.234", BLANK, &[Ohm1, Kilo1], "6.000kΩ"),
            ("12.34", BLANK, &[Ohm1, Kilo1], "60.00kΩ"),
            ("123.4", BLANK, &[Ohm1, Kilo1], "600.0kΩ"),
            ("1.234", BLANK, &[Ohm1, Mega1], "6.000MΩ"),
            ("12.34", BLANK, &[Ohm1, Mega1], "60.00MΩ"),
            ("12.34", BLANK, &[Siemens1, Nano1], "99.99nS"),
            ("12.34", BLANK, &[Farad1, Nano1], "60.00nF"),
            ("123.4", BLANK, &[Farad1, Nano1], "600.0nF"),
            ("1.234", BLANK, &[Farad1, Micro1], "6.000µF"),
            ("12.34", BLANK, &[Farad1, Micro1], "60.00µF"),
            ("123.4", BLANK, &[Farad1, Micro1], "600.0µF"),
            ("1.234", BLANK, &[Farad1, Milli1], "6.000mF"),
            ("12.34", BLANK, &[Farad1, Milli1], "25.00mF"),
            ("123.4", BLANK, &[Dc1, A1, Micro1], "600.0µA"),
            ("1234", BLANK, &[Dc1, A1, Micro1], "6000µA"),
            ("12.34", BLANK, &[Ac1, A1, Milli1], "60.00mA"),
            ("123.4", BLANK, &[Ac1, A1, Milli1], "600.0mA"),
            ("1.234", BLANK, &[Dc1, Ac1, A1], "6.000A"),
            ("12.34", BLANK, &[Dc1, Ac1, A1], "10.00A"),
            ("60.01", "12.34", &[Hz1, Ac2, V2, Milli2], "AC 60.00mV"),
            ("60.01", "123.4", &[Hz1, Ac2, V2, Milli2], "AC 600.0mV"),
            ("60.01", "1.234", &[Hz1, Ac2, V2], "AC 9.999V"),
            ("60.01", "12.34", &[Hz1, Ac2, V2], "AC 99.99V"),
            ("60.01", "123.4", &[Hz1, Ac2, V2], "AC 999.9V"),
            ("1.234", BLANK, &[Hz1, Kilo1], "5.00Hz ~ 1.000MHz"),
            ("12.34", BLANK, &[Duty1], "0.00% ~ 100.0%"),
            ("0.600", "diod", &[V1], "2.000V"),
            ("205C", BLANK, &[T1], "-50°C to 1000°C"),
            ("325F", BLANK, &[T2], "-58°F to 1832°F"),
        ] {
            assert_eq!(lookup(main, sub, anns).1, Some(label), "{main} {anns:?}");
        }
    }

    /// The secondary display picks the voltage table: DC alone, DC beside
    /// its AC (DC^AC), AC+DC and AC; DC current beside its AC takes the DC
    /// table's mode data only; Hz beside its AC is line level, alone logic
    /// level.
    #[test]
    fn the_secondary_display_picks_the_table() {
        const DC: &str = "DC Voltage";
        const DC_AC: &str = "DC AC & AC+DC AC Voltage";
        for (main, sub, anns, table, label) in [
            ("1.234", BLANK, &[Dc1, V1][..], DC, Some("9.999V")),
            ("1.234", "0.123", &[Dc1, V1, Ac2, V2], DC_AC, Some("9.999V")),
            (
                "12.34",
                "12.34",
                &[Dc1, V1, Milli1, Ac2, V2, Milli2],
                DC_AC,
                Some("60.00mV"),
            ),
            // Dashes beside the ∿: DC^AC, for a moment after a range change.
            ("1.234", "----", &[Dc1, V1, Ac2, V2], DC_AC, Some("9.999V")),
            // DC beside its frequency or DC: no AC component.
            ("1.234", "60.00", &[Dc1, V1, Hz2], DC, Some("9.999V")),
            (
                "1.234",
                "0.123",
                &[Dc1, Ac1, V1, Ac2, V2],
                DC_AC,
                Some("9.999V"),
            ),
            ("1.234", BLANK, &[Dc1, Ac1, V1], DC_AC, Some("9.999V")),
            (
                "1.234",
                "60.00",
                &[Ac1, V1, Hz2],
                "AC Voltage",
                Some("9.999V"),
            ),
            (
                "1.234",
                BLANK,
                &[Ac1, V1, LoZ],
                "AutoCheck™ (ACV)",
                Some("9.999V"),
            ),
            (
                "1.234",
                BLANK,
                &[Dc1, V1, LoZ],
                "AutoCheck™ (DCV)",
                Some("9.999V"),
            ),
            (
                "1.234",
                BLANK,
                &[Ohm1, Kilo1, LoZ],
                "AutoCheck™ (Ohms)",
                Some("6.000kΩ"),
            ),
            (
                "12.34",
                BLANK,
                &[Dc1, A1, Milli1],
                "DC Current",
                Some("60.00mA"),
            ),
            (
                "12.34",
                "12.34",
                &[Dc1, A1, Milli1, Ac2, A2, Milli2],
                "DC Current",
                None,
            ),
            (
                "12.34",
                "12.34",
                &[Dc1, Ac1, A1, Milli1, Ac2, A2, Milli2],
                "AC & AC+DC Current",
                Some("60.00mA"),
            ),
            (
                "12.34",
                "60.00",
                &[Ac1, A1, Milli1, Hz2],
                "AC & AC+DC Current",
                Some("60.00mA"),
            ),
            (
                "60.01",
                "1.234",
                &[Hz1, Ac2, V2],
                "Line Level Frequency (~Hz)",
                Some("AC 9.999V"),
            ),
            (
                "60.01",
                "----",
                &[Hz1, Ac2, V2],
                "Line Level Frequency (~Hz)",
                None,
            ),
            (
                "60.01",
                BLANK,
                &[Hz1],
                "Logic Level Frequency (⎍Hz) & Duty Cycle (D%)",
                Some("5.00Hz ~ 1.000MHz"),
            ),
        ] {
            assert_eq!(
                lookup(main, sub, anns),
                (Some(table), label),
                "{main} {sub} {anns:?}"
            );
        }
        // Hz beside a DC reading is no state the manual shows.
        assert_eq!(lookup("60.01", "1.234", &[Hz1, Dc2, V2]), (None, None));
        // Continuity: notes only.
        assert_eq!(
            lookup("000.7", BLANK, &[Ohm1, Continuity]),
            (Some("Audible Continuity Tester"), None)
        );
    }

    /// Every coupling of every voltage and current unit, beside its AC
    /// reading and alone, takes its coupling's table.
    #[test]
    fn each_coupled_mode_takes_its_table() {
        const DC_V: &str = "DC Voltage";
        const DC_AC_V: &str = "DC AC & AC+DC AC Voltage";
        const AC_V: &str = "AC Voltage";
        const DC_I: &str = "DC Current";
        const AC_I: &str = "AC & AC+DC Current";
        for (unit, main, base, base2, prefixes, label) in [
            ("µA", "123.4", A1, A2, &[Micro1, Micro2][..], "600.0µA"),
            ("mA", "12.34", A1, A2, &[Milli1, Milli2], "60.00mA"),
            ("A", "1.234", A1, A2, &[], "6.000A"),
            ("mV", "12.34", V1, V2, &[Milli1, Milli2], "60.00mV"),
            ("V", "1.234", V1, V2, &[], "9.999V"),
        ] {
            let volts = base == V1;
            for (coupling, alone, beside_its_ac) in [
                ("DC", &[Dc1][..], if volts { DC_V } else { DC_I }),
                ("AC", &[Ac1], if volts { AC_V } else { AC_I }),
                ("AC+DC", &[Dc1, Ac1], if volts { DC_AC_V } else { AC_I }),
            ] {
                let mode = format!("{coupling} {unit}");
                let anns = [alone, &[base], prefixes].concat();
                let want_alone = if coupling == "DC" && volts {
                    DC_V
                } else {
                    beside_its_ac
                };
                let want_ac = if coupling == "DC" && volts {
                    DC_AC_V
                } else {
                    beside_its_ac
                };
                assert_eq!(
                    lookup(main, BLANK, &anns),
                    (Some(want_alone), Some(label)),
                    "{mode}"
                );
                let with_ac = [&anns[..], &[Ac2, base2]].concat();
                // DC current beside its AC has no table of its own.
                let row = (coupling != "DC" || volts).then_some(label);
                assert_eq!(
                    lookup(main, main, &with_ac),
                    (Some(want_ac), row),
                    "{mode} beside AC"
                );
            }
        }
    }

    /// Logic-level Hz takes the frequency row in every prefix.
    #[test]
    fn logic_level_hz_takes_the_frequency_row() {
        for anns in [&[Hz1][..], &[Hz1, Kilo1], &[Hz1, Mega1]] {
            let s = Series::Bm82x;
            let m = decode::decode(&lcd8(s, "1.234", BLANK, anns), s).unwrap();
            assert!(
                std::ptr::eq(row(&m).unwrap(), &LOGIC_HZ.ranges[0]),
                "{anns:?}"
            );
            assert!(std::ptr::eq(table(&m).unwrap().0, &LOGIC_HZ), "{anns:?}");
        }
    }

    /// Line level with dashes beside the secondary unit takes that unit's
    /// part, or the first part where none is lit.
    #[test]
    fn dashed_line_level_takes_the_lit_units_part() {
        let s = Series::Bm82x;
        for (anns, part) in [
            (&[Ac2, V2][..], &LINE_HZ_V),
            (&[Ac2, V2, Milli2], &LINE_HZ_MV),
            (&[Ac2, A2, Micro2], &LINE_HZ_UA),
            (&[Ac2, A2, Milli2], &LINE_HZ_MA),
            (&[Ac2, A2], &LINE_HZ_A),
            (&[Ac2], &LINE_HZ_MV),
        ] {
            let data = lcd8(s, "60.01", "----", &[&[Hz1][..], anns].concat());
            let m = decode::decode(&data, s).unwrap();
            assert!(m.aux_values.is_empty());
            let (found, digits) = table(&m).unwrap();
            assert!(
                std::ptr::eq(found, part),
                "{anns:?}: {}",
                found.mode.notes[0]
            );
            assert_eq!(digits, None);
        }
    }

    /// DC mV takes each range's own input capacitance, and a point neither
    /// range has the 60.00mV part's.
    #[test]
    fn dc_millivolts_take_their_ranges_input_impedance() {
        let s = Series::Bm82x;
        for (main, part) in [
            ("12.34", &DC_V_60MV),
            ("123.4", &DC_V_600MV),
            ("1.234", &DC_V_60MV),
        ] {
            let m = decode::decode(&lcd8(s, main, BLANK, &[Dc1, V1, Milli1]), s).unwrap();
            assert!(std::ptr::eq(table(&m).unwrap().0, part), "{main}");
        }
        assert_eq!(DC_V_600MV.mode.input_impedance, Some("10MΩ, 80pF nominal"));
        assert_eq!(DC_V_60MV.mode.input_impedance, Some("10MΩ, 50pF nominal"));
        // Notes-only tables carry what PDF p.19 and p.20 give them.
        assert_eq!(CONTINUITY.mode.overload_protection, Some("1000V DC/AC rms"));
        assert_eq!(DBM.mode.input_impedance, Some("10MΩ, 50pF nominal"));
    }

    fn reply(hex: &str) -> Vec<u8> {
        hex.split(' ')
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// Replies a BM827s (ET-2517A, #35) and a BM525s (ET-2615A, #36) sent,
    /// as the golden fixtures and the captures hold them, with the table
    /// and row each reads against.
    #[test]
    fn real_replies_take_their_rows() {
        let line = "Line Level Frequency (~Hz)";
        let logic = "Logic Level Frequency (⎍Hz) & Duty Cycle (D%)";
        for (series, hex, table_name, label) in [
            // bm82x/acmv.yaml: 11.72 mV AC.
            (
                Series::Bm82x,
                "00 10 00 A0 B0 A8 CB 00 6D FD ED AF 00 01 60 0C 82 82 82 82 1F 00 00 00",
                Some("AC Voltage"),
                Some("60.00mV"),
            ),
            // bm82x/dcv_negative.yaml: -21.99 V DC beside 00.07 V AC.
            (
                Series::Bm82x,
                "00 20 80 CB B0 ED ED 40 AF BF AF A8 10 00 20 00 82 82 82 82 10 00 00 00",
                Some("DC AC & AC+DC AC Voltage"),
                Some("99.99V"),
            ),
            // bm82x/crest_min.yaml: -0.819 V DC alone.
            (
                Series::Bm82x,
                "00 28 80 BF EF A0 ED 00 00 00 00 00 00 00 20 00 82 82 82 82 50 00 00 00",
                Some("DC Voltage"),
                Some("9.999V"),
            ),
            // bm82x/hz_acmv.yaml: 60.01 Hz on 12.61 mV AC.
            (
                Series::Bm82x,
                "00 00 00 6F BF AF A0 40 A0 DB 6F A0 90 10 00 00 82 82 82 82 10 00 00 00",
                Some(line),
                Some("AC 60.00mV"),
            ),
            // bm82x/hz_acv.yaml: 0.000 Hz on 0.364 V AC.
            (
                Series::Bm82x,
                "00 00 00 BF AF AF AF 40 BF E9 6F E4 10 10 00 00 82 82 82 82 10 00 00 00",
                Some(line),
                Some("AC 9.999V"),
            ),
            // bm82x/ns.yaml, ohm_body.yaml, ohm_open_leads.yaml.
            (
                Series::Bm82x,
                "00 00 00 AF BF AF 6D 00 00 00 00 00 00 00 0A 00 82 82 82 82 00 00 00 00",
                Some("Ohms"),
                Some("99.99nS"),
            ),
            (
                Series::Bm82x,
                "00 00 00 B0 6D 6D EF 00 00 00 00 00 00 60 00 0C 82 82 82 82 1F 00 00 00",
                Some("Ohms"),
                Some("6.000MΩ"),
            ),
            (
                Series::Bm82x,
                "00 00 00 00 BF 07 00 00 00 00 00 00 00 60 00 FC 82 82 82 82 1F 00 00 00",
                Some("Ohms"),
                None,
            ),
            // bm82x/cap.yaml, dcma.yaml.
            (
                Series::Bm82x,
                "00 00 00 AF BF AF 6D 00 00 00 00 00 00 00 09 00 82 82 82 82 10 00 00 00",
                Some("Capacitance"),
                Some("60.00nF"),
            ),
            (
                Series::Bm82x,
                "00 20 00 AF BF AF AF 00 00 00 00 00 00 00 50 0C 82 82 82 82 10 00 00 00",
                Some("DC Current"),
                Some("60.00mA"),
            ),
            // The BM827s's acdcma step: DC mA beside its AC, mode data only.
            (
                Series::Bm82x,
                "00 20 00 AF BF AF AF 40 AF BF AF AF A0 00 50 00 82 82 82 82 10 00 00 00",
                Some("DC Current"),
                None,
            ),
            // Its logic_hz and duty steps.
            (
                Series::Bm82x,
                "00 00 00 BF AF AF AF 00 00 00 00 00 00 10 00 00 82 82 82 82 10 00 00 00",
                Some(logic),
                Some("5.00Hz ~ 1.000MHz"),
            ),
            (
                Series::Bm82x,
                "00 00 00 AF BF AF AF 00 00 00 00 00 00 00 04 00 82 82 82 82 10 00 00 00",
                Some(logic),
                Some("0.00% ~ 100.0%"),
            ),
            // bm82x/t1_fahrenheit.yaml, t1_open.yaml, diode_open.yaml.
            (
                Series::Bm82x,
                "00 00 01 AF A8 AF 4E 00 00 00 00 00 00 00 00 00 82 82 82 82 00 00 00 00",
                Some("Temperature"),
                Some("-58°F to 1832°F"),
            ),
            (
                Series::Bm82x,
                "00 00 01 40 40 40 0F 00 00 00 00 00 00 00 00 00 82 82 82 82 00 00 00 00",
                Some("Temperature"),
                None,
            ),
            (
                Series::Bm82x,
                "00 00 00 10 AF 07 00 00 E3 20 63 E3 00 00 20 00 82 82 82 82 00 00 00 00",
                Some("Diode Tester"),
                None,
            ),
            // bm52x/acdcmv.yaml: 111.9 mV AC+DC.
            (
                Series::Bm52x,
                "00 30 00 A0 A0 B0 ED 40 A0 A0 B0 ED 90 00 60 00 52 52 52 52 10 00 00 00",
                Some("DC AC & AC+DC AC Voltage"),
                Some("600.0mV"),
            ),
            // bm52x/autocheck_dc.yaml, autocheck_ohm_dashes.yaml, autocheck.yaml.
            (
                Series::Bm52x,
                "00 20 10 FF ED ED E4 00 EE 23 47 63 00 00 20 06 52 52 52 52 1F 00 00 00",
                Some("AutoCheck™ (DCV)"),
                Some("9.999V"),
            ),
            (
                Series::Bm52x,
                "00 00 10 50 40 40 40 00 EE 23 47 63 00 A0 00 FC 52 52 52 52 1F 00 00 00",
                Some("AutoCheck™ (Ohms)"),
                None,
            ),
            (
                Series::Bm52x,
                "00 00 10 EE 23 47 63 00 00 00 00 00 00 00 00 00 52 52 52 52 10 00 00 00",
                None,
                None,
            ),
            // The BM525s's autocheck_ac step: 121.8 V.
            (
                Series::Bm52x,
                "00 10 10 A0 CB B0 EF 00 EE 23 47 63 00 00 20 06 52 52 52 52 1F 00 00 00",
                Some("AutoCheck™ (ACV)"),
                Some("999.9V"),
            ),
            // Its iner step: 4.879 A AC+DC beside its AC.
            (
                Series::Bm52x,
                "00 30 00 F4 EF A8 ED 40 F4 EF A8 ED 20 00 10 00 52 52 52 52 10 00 00 00",
                Some("AC & AC+DC Current"),
                Some("6.000A"),
            ),
            // bm52x/hz_acv.yaml: 999.7 Hz on 1.234 V AC.
            (
                Series::Bm52x,
                "00 00 00 ED ED FD A8 40 B0 CB E9 E4 10 10 00 00 52 52 52 52 10 00 00 00",
                Some(line),
                Some("AC 9.999V"),
            ),
            // Its t1_t2 step: T1-T2 has no table.
            (
                Series::Bm52x,
                "00 00 0B 40 40 40 0F 02 40 40 40 0F 00 00 00 00 52 52 52 52 00 00 00 00",
                None,
                None,
            ),
        ] {
            let data = reply(hex);
            let (m, reports) = capture_reports(|| decode::decode(&data, series));
            assert!(reports.is_empty(), "{hex}: {reports:?}");
            let m = m.unwrap();
            assert_eq!(table(&m).map(|(t, _)| t.name), table_name, "{}", what(&m));
            assert_eq!(row(&m).map(|r| r.label), label, "{}", what(&m));
        }
    }

    /// A reading through `Dmm` carries its row and its table's mode data:
    /// bm82x/hz_acmv.yaml, on the cable.
    #[test]
    fn a_reading_through_dmm_carries_its_spec() {
        let data = reply("00 00 00 6F BF AF A0 40 A0 DB 6F A0 90 10 00 00 82 82 82 82 10 00 00 00");
        let reports = data.chunks(8).map(<[u8]>::to_vec).collect();
        let cable = ScriptedCable::new(reply::request(0x82), reports, Vec::new());
        let entry = registry::find_device("bm82x").unwrap();
        let mut dmm = crate::Dmm::new(cable, (entry.new_protocol)()).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Hz");
        let spec = m.spec.expect("a row");
        assert!(std::ptr::eq(spec, &LINE_HZ_MV.ranges[0].spec));
        assert_eq!(spec.accuracy[0].accuracy, "0.04%+4");
        let mode = m.mode_spec.expect("mode data");
        assert_eq!(mode.overload_protection, Some("1000V DC/AC rms"));
        assert!(std::ptr::eq(mode, &LINE_HZ_MV.mode));
    }

    /// The bm82x and bm52x entries share one sheet, in manual order; the
    /// bm86x entry has none.
    #[test]
    fn both_bm820_entries_share_the_sheet() {
        let sheet = |id| (registry::find_device(id).unwrap().new_protocol)().spec_sheet();
        let (bm82x, bm52x) = (sheet("bm82x"), sheet("bm52x"));
        assert_eq!(bm82x.len(), ALL.len());
        assert_eq!(bm52x.len(), ALL.len());
        for ((a, b), part) in bm82x.iter().zip(&bm52x).zip(ALL) {
            assert!(std::ptr::eq(a.mode, &part.mode) && std::ptr::eq(b.mode, &part.mode));
        }
        assert_eq!(bm82x[0].name, "DC Voltage");
        assert!(sheet("bm86x").is_empty());
        let m = decode::decode(&reply::tests::bm820_example(0x82), Series::Bm82x).unwrap();
        let bm86x = Bm86xProtocol::new(Series::Bm86x);
        assert!(bm86x.spec_info(&m).is_none() && bm86x.mode_spec_info(&m).is_none());
        let bm82x = Bm86xProtocol::new(Series::Bm82x);
        assert_eq!(bm82x.spec_info(&m).unwrap().accuracy.len(), 5);
    }
}
