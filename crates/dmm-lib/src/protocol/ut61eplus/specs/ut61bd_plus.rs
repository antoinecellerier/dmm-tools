//! Specification data for the UNI-T UT61B+ and UT61D+ (6,000 counts).
//!
//! Transcribed from references/ut61eplus/ut61e_manual.pdf, the UT61+ Series
//! User Manual (P/N:110401109614X), "IX. Specifications", 2. Electrical
//! Specifications (PDF pages 14-18, printed 25-34): the UT61B+/UT61D+ tables,
//! with values as printed but for the typesetting slips noted where they are.
//! Cross-checked against the UT61+ series product page and the UT61+/UT161
//! series datasheet. The notes are short app notes condensed from the
//! manual's remarks (the verbatim text is in the local verified
//! transcription).
//!
//! The two models share one column. Parts they share are single statics;
//! where only the notes differ (the AC frequency response), each model has
//! its own part over one shared row slice. Rows the manual tags for one
//! model ("10.00A (UT61B+)", "20.00A (UT61D+)", the LoZ rows) are in that
//! model's parts only, and so is temperature, which is the UT61D+'s. A
//! manual table is split into parts of the same name where its rows need
//! different mode-level data or belong to different modes. Range bytes index
//! the range tables in `tables/ut61b_plus.rs` and `tables/ut61d_plus.rs`.

use super::{CONTINUITY, DIODE, UT161_F1, UT161_F2, ut161_fuse};
use crate::protocol::ut61eplus::mode::Mode;
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// Every UT61B+ table, in manual order.
pub(super) static UT61B_PLUS: &[&ModeSpecs] = &[
    &DC_MV,
    &DC_V,
    &B_AC_MV,
    &B_AC_V,
    &RESISTANCE,
    &CONTINUITY,
    &DIODE,
    &CAPACITANCE,
    &DC_UA,
    &DC_MA,
    &B_DC_A,
    &B_AC_UA,
    &B_AC_MA,
    &B_AC_A,
    &FREQUENCY,
    &DUTY,
];

/// Every UT61D+ table, in manual order.
pub(super) static UT61D_PLUS: &[&ModeSpecs] = &[
    &DC_MV,
    &DC_V,
    &D_AC_MV,
    &D_AC_V,
    &LOZ_ACV,
    &RESISTANCE,
    &CONTINUITY,
    &DIODE,
    &CAPACITANCE,
    &TEMP_C,
    &TEMP_F,
    &DC_UA,
    &DC_MA,
    &D_DC_A,
    &D_AC_UA,
    &D_AC_MA,
    &D_AC_A,
    &FREQUENCY,
    &DUTY,
];

/// The UT61B+ table for a reading in `mode`.
///
/// `None` where the manual has no table: NCV. The UT61B+ has no
/// temperature, hFE, LoZ, LPF or AC+DC.
pub(super) fn ut61b_plus(mode: Mode) -> Option<&'static ModeSpecs> {
    Some(match mode {
        Mode::DcMv => &DC_MV,
        Mode::DcV => &DC_V,
        Mode::AcMv => &B_AC_MV,
        Mode::AcV => &B_AC_V,
        Mode::Ohm => &RESISTANCE,
        Mode::Continuity => &CONTINUITY,
        Mode::Diode => &DIODE,
        Mode::Capacitance => &CAPACITANCE,
        Mode::DcUa => &DC_UA,
        Mode::DcMa => &DC_MA,
        Mode::DcA => &B_DC_A,
        Mode::AcUa => &B_AC_UA,
        Mode::AcMa => &B_AC_MA,
        Mode::AcA => &B_AC_A,
        // Known limitation: Hz and Duty % send the same mode byte from every
        // dial position, so a reading from V~, mV, µA, mA or A takes the Hz/%
        // position's table, though the manual's AC V and AC current remarks
        // give those readings their own terms.
        Mode::Hz => &FREQUENCY,
        Mode::DutyCycle => &DUTY,
        _ => return None,
    })
}

/// The UT61D+ table for a reading in `mode`.
///
/// `None` where the manual has no table: NCV. The UT61D+ has no hFE, LPF or
/// AC+DC.
pub(super) fn ut61d_plus(mode: Mode) -> Option<&'static ModeSpecs> {
    Some(match mode {
        Mode::DcMv => &DC_MV,
        Mode::DcV => &DC_V,
        Mode::AcMv => &D_AC_MV,
        Mode::AcV => &D_AC_V,
        // Which of the two LoZ bytes the UT61D+ sends is unresolved (family
        // spec §3); both read the LoZ rows.
        Mode::LozV | Mode::LozV2 => &LOZ_ACV,
        Mode::Ohm => &RESISTANCE,
        Mode::Continuity => &CONTINUITY,
        Mode::Diode => &DIODE,
        Mode::Capacitance => &CAPACITANCE,
        Mode::TempC => &TEMP_C,
        Mode::TempF => &TEMP_F,
        Mode::DcUa => &DC_UA,
        Mode::DcMa => &DC_MA,
        Mode::DcA => &D_DC_A,
        Mode::AcUa => &D_AC_UA,
        Mode::AcMa => &D_AC_MA,
        Mode::AcA => &D_AC_A,
        // Known limitation: Hz and Duty % send the same mode byte from every
        // dial position, so a reading from V~, mV, µA, mA or A takes the Hz/%
        // position's table, though the manual's AC V and AC current remarks
        // give those readings their own terms.
        Mode::Hz => &FREQUENCY,
        Mode::DutyCycle => &DUTY,
        _ => return None,
    })
}

// ── 1) DC Voltage, mV (manual PDF page 14) ───────────────────────────────

// The mV rows are ranges 0-1 of DC mV, on the mV dial position, with an
// input impedance of their own.
static DC_MV: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 14,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "60.00mV",
            spec: SpecInfo {
                resolution: "0.01mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mV",
            spec: SpecInfo {
                resolution: "0.1mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 1GΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "Accuracy valid 1%–100% of range",
            "Shorted leads: residual ≤5 digits",
        ],
    },
};

// ── 1) DC Voltage (manual PDF page 14) ───────────────────────────────────

// Range bytes 0-3 of DC V.
static DC_V: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 14,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "6.000V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "60.00V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000V",
            spec: SpecInfo {
                resolution: "1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "Accuracy valid 1%–100% of range",
            "Shorted leads: residual ≤5 digits",
        ],
    },
};

// ── 2) AC Voltage (manual PDF page 15) ───────────────────────────────────

// The two models' rows are the same; their notes give each its own
// frequency response, so each has its own parts over these rows. The
// column prints no frequency band per row.
static AC_MV_ROWS: [RangeSpec; 2] = [
    RangeSpec {
        range: Some(0),
        label: "60.00mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+5",
            }],
        },
    },
    RangeSpec {
        range: Some(1),
        label: "600.0mV",
        spec: SpecInfo {
            resolution: "0.1mV",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+5",
            }],
        },
    },
];

static AC_V_ROWS: [RangeSpec; 4] = [
    RangeSpec {
        range: Some(0),
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
        range: Some(1),
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
        range: Some(2),
        label: "600.0V",
        spec: SpecInfo {
            resolution: "0.1V",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.0%+3",
            }],
        },
    },
    RangeSpec {
        range: Some(3),
        label: "1000V",
        spec: SpecInfo {
            resolution: "1V",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+5",
            }],
        },
    },
];

// Ranges 0-1 of AC mV, and 0-3 of AC V.
static B_AC_MV: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &AC_MV_ROWS,
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–500Hz, 1%–100% of range (60mV: 2%–100%)",
            "Shorted leads: residual ≤3 digits",
        ],
    },
};

static B_AC_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &AC_V_ROWS,
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–500Hz, 1%–100% of range (60mV: 2%–100%)",
            "Shorted leads: residual ≤3 digits",
        ],
    },
};

static D_AC_MV: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &AC_MV_ROWS,
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)",
            "Shorted leads: residual ≤3 digits",
        ],
    },
};

static D_AC_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &AC_V_ROWS,
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)",
            "Shorted leads: residual ≤3 digits",
        ],
    },
};

// The "(UT61D+)" LoZ rows, ranges 0-1 of both LoZ mode bytes.
static LOZ_ACV: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "LoZ ACV 600.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            // Printed "LoZ ACV  1000V", with two spaces.
            label: "LoZ ACV 1000V",
            spec: SpecInfo {
                resolution: "1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        // The section's unqualified "About 10MΩ" cannot apply to a
        // low-impedance input, and the manual prints none for LoZ.
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (60mV: 2%–100%)",
            "Shorted leads: residual ≤3 digits",
        ],
    },
};

// ── 4) Resistance (manual PDF page 16) ───────────────────────────────────

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "Resistance",
    page: 16,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "600.0Ω",
            spec: SpecInfo {
                resolution: "0.1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "6.000kΩ",
            spec: SpecInfo {
                resolution: "1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60.00kΩ",
            spec: SpecInfo {
                resolution: "10Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600.0kΩ",
            spec: SpecInfo {
                resolution: "100Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "6.000MΩ",
            spec: SpecInfo {
                resolution: "1kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "60.00MΩ",
            spec: SpecInfo {
                resolution: "10kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Result = reading − shorted-lead resistance",
            "Open circuit ≈1V",
            "Accuracy valid 1%–100% of range",
        ],
    },
};

// ── 7) Capacitance (manual PDF page 17) ──────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 17,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "60.00nF",
            spec: SpecInfo {
                resolution: "10pF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0nF",
            spec: SpecInfo {
                resolution: "100pF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "6.000µF",
            spec: SpecInfo {
                resolution: "1nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "60.00µF",
            spec: SpecInfo {
                resolution: "10nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "600.0µF",
            spec: SpecInfo {
                resolution: "100nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "6.000mF",
            spec: SpecInfo {
                resolution: "1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "10%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "60.00mF",
            spec: SpecInfo {
                resolution: "10µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "10%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Result = reading − open-lead capacitance; ≤1µF: use REL",
            "Accuracy valid 1%–100% of range",
            "60mF range: measurement takes ≈20s",
        ],
    },
};

// ── 8) Temperature, °C (manual PDF page 17) ──────────────────────────────

// UT61D+ only. One table, one row per unit, whose resolution is a span
// over its accuracy bands; SELECT picks the unit. No overload protection
// printed.
static TEMP_C: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 17,
    ranges: &[RangeSpec {
        range: None,
        label: "-40~1000°C",
        spec: SpecInfo {
            resolution: "0.1°C~1°C",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40~0°C"),
                    accuracy: "1.0%+3°C",
                },
                AccuracyBand {
                    freq_range: Some("0~300°C"),
                    accuracy: "1.0%+2°C",
                },
                AccuracyBand {
                    freq_range: Some("300~1000°C"),
                    accuracy: "1.0%+3°C",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "K-type thermocouple only",
            "Measured temperature should stay below 230°C/446°F",
        ],
    },
};

// ── 8) Temperature, °F (manual PDF page 17) ──────────────────────────────

static TEMP_F: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 17,
    ranges: &[RangeSpec {
        range: None,
        label: "-40~1832°F",
        spec: SpecInfo {
            resolution: "0.2°F~2°F",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40~32°F"),
                    accuracy: "1.0%+6°F",
                },
                AccuracyBand {
                    freq_range: Some("32~572°F"),
                    accuracy: "1.0%+4°F",
                },
                AccuracyBand {
                    freq_range: Some("572~1832°F"),
                    accuracy: "1.0%+6°F",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "K-type thermocouple only",
            "Measured temperature should stay below 230°C/446°F",
        ],
    },
};

// ── 9) DC Current, µA (manual PDF page 17) ───────────────────────────────

// One table in the manual. µA, mA and A are modes of their own, each
// numbering its ranges from 0, and the A ranges have a fuse and the >5A
// note of their own.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "600.0µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+2",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤5 digits",
            "Accuracy valid 1%–100% of range",
        ],
    },
};

// ── 9) DC Current, mA (manual PDF page 17) ───────────────────────────────

static DC_MA: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "60.00mA",
            spec: SpecInfo {
                resolution: "10µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600.0mA",
            spec: SpecInfo {
                resolution: "0.1mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.0%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤5 digits",
            "Accuracy valid 1%–100% of range",
        ],
    },
};

// ── 9) DC Current, A (manual PDF page 17) ────────────────────────────────

// The 6.000A row is both models'; the second A row is the UT61B+'s
// 10.00A or the UT61D+'s 20.00A.
const DC_6A: RangeSpec = RangeSpec {
    range: Some(0),
    label: "6.000A",
    spec: SpecInfo {
        resolution: "1mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+5",
        }],
    },
};

static B_DC_A: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[
        DC_6A,
        RangeSpec {
            range: Some(1),
            label: "10.00A",
            spec: SpecInfo {
                resolution: "10mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤5 digits",
            "Accuracy valid 1%–100% of range",
            ">5A: max 10s, rest ≥15 min",
        ],
    },
};

static D_DC_A: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[
        DC_6A,
        RangeSpec {
            range: Some(1),
            label: "20.00A",
            spec: SpecInfo {
                resolution: "10mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤5 digits",
            "Accuracy valid 1%–100% of range",
            ">5A: max 10s, rest ≥15 min",
        ],
    },
};

// ── 10) AC Current (manual PDF page 18) ──────────────────────────────────

// Split as DC current is, and per model as AC voltage is.
static AC_UA_ROWS: [RangeSpec; 2] = [
    RangeSpec {
        range: Some(0),
        label: "600.0µA",
        spec: SpecInfo {
            resolution: "0.1µA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+5",
            }],
        },
    },
    RangeSpec {
        range: Some(1),
        label: "6000µA",
        spec: SpecInfo {
            resolution: "1µA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+5",
            }],
        },
    },
];

static AC_MA_ROWS: [RangeSpec; 2] = [
    RangeSpec {
        range: Some(0),
        label: "60.00mA",
        spec: SpecInfo {
            resolution: "10µA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.5%+5",
            }],
        },
    },
    RangeSpec {
        range: Some(1),
        label: "600.0mA",
        spec: SpecInfo {
            resolution: "0.1mA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.5%+5",
            }],
        },
    },
];

const AC_6A: RangeSpec = RangeSpec {
    range: Some(0),
    label: "6.000A",
    spec: SpecInfo {
        resolution: "1mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "2.0%+5",
        }],
    },
};

static B_AC_UA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &AC_UA_ROWS,
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–500Hz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
        ],
    },
};

static B_AC_MA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &AC_MA_ROWS,
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–500Hz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
        ],
    },
};

static B_AC_A: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &[
        AC_6A,
        RangeSpec {
            range: Some(1),
            label: "10.00A",
            spec: SpecInfo {
                resolution: "10mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–500Hz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
            ">5A: max 10s, rest ≥15 min",
        ],
    },
};

static D_AC_UA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &AC_UA_ROWS,
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
        ],
    },
};

static D_AC_MA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &AC_MA_ROWS,
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
        ],
    },
};

static D_AC_A: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &[
        AC_6A,
        RangeSpec {
            range: Some(1),
            label: "20.00A",
            spec: SpecInfo {
                resolution: "10mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤3.0 at 3000, ≤1.5 at 6000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 40Hz–1kHz, 1%–100% of range (600.0µA: 5%–100%)",
            "Open circuit: residual ≤5 digits",
            ">5A: max 10s, rest ≥15 min",
        ],
    },
};

// ── 11) Frequency/Duty Ratio, frequency (manual PDF page 18) ─────────────

// The manual prints one frequency row, a span, where the range table has
// five range bytes: all of them take it. Each row's remarks are its own,
// so each part carries its row's notes.
static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "Frequency/Duty Ratio",
    page: 18,
    ranges: &[RangeSpec {
        range: None,
        // Printed "10.00Hz~10.00MHZ".
        label: "10.00Hz~10.00MHz",
        spec: SpecInfo {
            resolution: "0.01Hz~0.01MHz",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.1%+4",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Hz input ≤100kHz: 200mV–20V rms",
            "Hz input >100kHz–1MHz: 600mV–20V rms",
            "Hz input >1MHz: 1V–20V rms",
        ],
    },
};

// ── 11) Frequency/Duty Ratio, duty (manual PDF page 18) ──────────────────

// The UT161 manual prints this accuracy "± (2%+5)", the same value;
// the UT161B and UT161D share the row.
static DUTY: ModeSpecs = ModeSpecs {
    name: "Frequency/Duty Ratio",
    page: 18,
    ranges: &[RangeSpec {
        range: None,
        label: "0.1%~99.9%",
        spec: SpecInfo {
            resolution: "0.1%",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "2.0%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Duty: square waves only, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%"],
    },
};

// ── UT161B and UT161D current ranges (UT161 manual PDF page 17) ──────────

// Their current parts: the UT61B+'s and UT61D+'s with the UT161 manual's
// fuses.
static UT161_DC_UA: ModeSpecs = ut161_fuse(&DC_UA, UT161_F1);
static UT161_DC_MA: ModeSpecs = ut161_fuse(&DC_MA, UT161_F1);
static UT161B_DC_A: ModeSpecs = ut161_fuse(&B_DC_A, UT161_F2);
static UT161D_DC_A: ModeSpecs = ut161_fuse(&D_DC_A, UT161_F2);
static UT161B_AC_UA: ModeSpecs = ut161_fuse(&B_AC_UA, UT161_F1);
static UT161B_AC_MA: ModeSpecs = ut161_fuse(&B_AC_MA, UT161_F1);
static UT161B_AC_A: ModeSpecs = ut161_fuse(&B_AC_A, UT161_F2);
static UT161D_AC_UA: ModeSpecs = ut161_fuse(&D_AC_UA, UT161_F1);
static UT161D_AC_MA: ModeSpecs = ut161_fuse(&D_AC_MA, UT161_F1);
static UT161D_AC_A: ModeSpecs = ut161_fuse(&D_AC_A, UT161_F2);

/// Each UT61B+ or UT61D+ current part and its UT161 twin.
pub(super) static UT161_FUSES: &[(&ModeSpecs, &ModeSpecs)] = &[
    (&DC_UA, &UT161_DC_UA),
    (&DC_MA, &UT161_DC_MA),
    (&B_DC_A, &UT161B_DC_A),
    (&D_DC_A, &UT161D_DC_A),
    (&B_AC_UA, &UT161B_AC_UA),
    (&B_AC_MA, &UT161B_AC_MA),
    (&B_AC_A, &UT161B_AC_A),
    (&D_AC_UA, &UT161D_AC_UA),
    (&D_AC_MA, &UT161D_AC_MA),
    (&D_AC_A, &UT161D_AC_A),
];
