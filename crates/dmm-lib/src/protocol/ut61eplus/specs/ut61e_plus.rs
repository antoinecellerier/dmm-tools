//! Specification data for the UNI-T UT61E+ (22,000 counts).
//!
//! Transcribed from references/ut61eplus/ut61e_manual.pdf, the UT61+ Series
//! User Manual (P/N:110401109614X), "IX. Specifications", 2. Electrical
//! Specifications (PDF pages 14-18, printed 25-34): the UT61E+ tables and
//! rows, with values as printed but for the typesetting slips noted where
//! they are. Cross-checked against the UT61+ series product page and the
//! UT61+/UT161 series datasheet. The notes are short app notes condensed from
//! the manual's remarks (the verbatim text is in the local verified
//! transcription).
//!
//! A manual table is split into parts of the same name where its rows need
//! different mode-level data or belong to different modes. Range bytes index
//! the UT61E+ range table (`tables/ut61e_plus.rs`).

use super::{CONTINUITY, DIODE};
use crate::protocol::ut61eplus::mode::Mode;
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// Every table, in manual order.
pub(super) static ALL: &[&ModeSpecs] = &[
    &DC_MV,
    &DC_V,
    &AC_MV,
    &AC_V,
    &LPF_V,
    &ACDC_V,
    &RESISTANCE,
    &CONTINUITY,
    &DIODE,
    &TRANSISTOR,
    &CAPACITANCE,
    &DC_UA,
    &DC_MA,
    &DC_A,
    &AC_UA,
    &AC_MA,
    &AC_A,
    &FREQUENCY,
    &DUTY,
];

/// The table for a reading in `mode`.
///
/// `None` where the manual has no table: NCV, and temperature, which it
/// gives the UT61D+ only ("8) Temperature", PDF p. 17). The modes off the
/// UT61E+ dial (LoZ, and the LPF and AC+DC variants of mV and A) have none
/// either.
pub(super) fn table(mode: Mode) -> Option<&'static ModeSpecs> {
    Some(match mode {
        Mode::DcMv => &DC_MV,
        Mode::DcV => &DC_V,
        Mode::AcMv => &AC_MV,
        Mode::AcV => &AC_V,
        Mode::LpfV => &LPF_V,
        Mode::AcDcV => &ACDC_V,
        Mode::Ohm => &RESISTANCE,
        Mode::Continuity => &CONTINUITY,
        Mode::Diode => &DIODE,
        Mode::Hfe => &TRANSISTOR,
        Mode::Capacitance => &CAPACITANCE,
        Mode::DcUa => &DC_UA,
        Mode::DcMa => &DC_MA,
        Mode::DcA => &DC_A,
        Mode::AcUa => &AC_UA,
        Mode::AcMa => &AC_MA,
        Mode::AcA => &AC_A,
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

// The 220.00mV row is range 0 of DC mV, on the mV dial position, with an
// input impedance of its own. The UT61E+ mV position is fixed at 220mV
// in DC and AC: RANGE does nothing there and only range byte 0 has been
// seen, so the range table's second entry (2.2V) has no row.
static DC_MV: ModeSpecs = ModeSpecs {
    name: "DC Voltage",
    page: 14,
    ranges: &[RangeSpec {
        range: Some(0),
        label: "220.00mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.1%+5",
            }],
        },
    }],
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
            label: "2.2000V",
            spec: SpecInfo {
                resolution: "0.1mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "22.000V",
            spec: SpecInfo {
                resolution: "1mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "220.00V",
            spec: SpecInfo {
                resolution: "10mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
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

// ── 2) AC Voltage, mV (manual PDF page 15) ───────────────────────────────

// Range 0 of AC mV, as for DC. The row has no LPF band: LPF is on the V~
// position only.
static AC_MV: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &[RangeSpec {
        range: Some(0),
        label: "220.00mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40Hz~1kHz"),
                    accuracy: "1.0%+10",
                },
                AccuracyBand {
                    freq_range: Some("1kHz~10kHz"),
                    accuracy: "1.5%+30",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Shorted leads: residual ≤10 digits",
        ],
    },
};

// ── 2) AC Voltage (manual PDF page 15) ───────────────────────────────────

// Range bytes 0-3 of AC V, with each row's two frequency bands. The
// rows' third band applies with LPF on, and is the next part's.
static AC_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "2.2000V",
            spec: SpecInfo {
                resolution: "0.1mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "0.8%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "1.2%+50",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "22.000V",
            spec: SpecInfo {
                resolution: "1mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "0.8%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "1.2%+50",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "220.00V",
            spec: SpecInfo {
                resolution: "10mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "0.8%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "2.0%+50",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "1.2%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3.0%+50",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Shorted leads: residual ≤10 digits",
        ],
    },
};

// ── 2) AC Voltage, LPF (manual PDF page 15) ──────────────────────────────

// The same rows' "40Hz~100Hz (LPF)" band, for LPF V on range bytes 0-3:
// with LPF on it is the only band that applies. The 1000.0V row's merged
// 3.0%+50 cell spans its 1kHz~10kHz and LPF bands, so it is a band in both
// parts.
static LPF_V: ModeSpecs = ModeSpecs {
    name: "AC Voltage",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "2.2000V",
            spec: SpecInfo {
                resolution: "0.1mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~100Hz (LPF)"),
                    accuracy: "1.2%+50",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "22.000V",
            spec: SpecInfo {
                resolution: "1mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~100Hz (LPF)"),
                    accuracy: "1.8%+50",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "220.00V",
            spec: SpecInfo {
                resolution: "10mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~100Hz (LPF)"),
                    accuracy: "2.0%+50",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~100Hz (LPF)"),
                    accuracy: "3.0%+50",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Shorted leads: residual ≤10 digits",
        ],
    },
};

// ── 3) AC+DC Voltage (manual PDF page 15) ────────────────────────────────

// Range bytes 0-3 of AC+DC V, which takes the DC V range table.
static ACDC_V: ModeSpecs = ModeSpecs {
    name: "AC+DC Voltage",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "2.2000V",
            spec: SpecInfo {
                resolution: "0.1mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~500Hz"),
                    accuracy: "1.8%+70",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "22.000V",
            spec: SpecInfo {
                resolution: "1mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~500Hz"),
                    accuracy: "1.8%+70",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "220.00V",
            spec: SpecInfo {
                resolution: "10mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~500Hz"),
                    accuracy: "1.8%+70",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz~500Hz"),
                    accuracy: "4.0%+70",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS; accuracy valid 10%–100% of range",
            "For AC voltage, shorted leads: residual ≤200 digits",
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
            label: "220.00Ω",
            spec: SpecInfo {
                resolution: "0.01Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    // Printed "± (0.5+10)", without the %.
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "2.2000kΩ",
            spec: SpecInfo {
                resolution: "0.1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    // Printed "± (0.5+10)", without the %.
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "22.000kΩ",
            spec: SpecInfo {
                resolution: "1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    // Printed "± (0.5+10)", without the %.
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "220.00kΩ",
            spec: SpecInfo {
                resolution: "10Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    // Printed "± (0.5+10)", without the %.
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "2.2000MΩ",
            spec: SpecInfo {
                resolution: "100Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    // Printed "± (0.8+10)", without the %.
                    accuracy: "0.8%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "22.000MΩ",
            spec: SpecInfo {
                resolution: "1kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "220.00MΩ",
            spec: SpecInfo {
                resolution: "10kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+50",
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

// ── 6) Transistor Magnification (manual PDF page 16) ─────────────────────

// No accuracy column, and no overload protection printed.
static TRANSISTOR: ModeSpecs = ModeSpecs {
    name: "Transistor Magnification",
    page: 16,
    ranges: &[RangeSpec {
        range: None,
        label: "1000β",
        spec: SpecInfo {
            resolution: "1β",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Ib0 ≈1.8µA, Vce ≈2.5V", "Reading for reference only"],
    },
};

// ── 7) Capacitance (manual PDF page 16) ──────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "Capacitance",
    page: 16,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "22.000nF",
            spec: SpecInfo {
                resolution: "1pF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "220.00nF",
            spec: SpecInfo {
                resolution: "10pF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "2.2000µF",
            spec: SpecInfo {
                resolution: "100pF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "22.000µF",
            spec: SpecInfo {
                resolution: "1nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "220.00µF",
            spec: SpecInfo {
                resolution: "10nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "4.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "2.2000mF",
            spec: SpecInfo {
                resolution: "100nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "4.0%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "22.000mF",
            spec: SpecInfo {
                resolution: "1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "10%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "220.00mF",
            spec: SpecInfo {
                resolution: "10µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "20%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Result = reading − open-lead capacitance; ≤22nF: use REL",
            "Accuracy valid 1%–100% of range",
            "Ranges ≤2.2µF: add 10 digits when accuracy is ≤3%",
            "220mF range: measurement takes ≈20s",
        ],
    },
};

// ── 9) DC Current, µA (manual PDF page 17) ───────────────────────────────

// One table in the manual. µA, mA and A are modes of their own, each
// numbering its ranges from 0, and the A range has a fuse of its own.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "220.00µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "2200.0µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤10 digits",
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
            label: "22.000mA",
            spec: SpecInfo {
                resolution: "1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "220.00mA",
            spec: SpecInfo {
                resolution: "10µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤10 digits",
            "Accuracy valid 1%–100% of range",
        ],
    },
};

// ── 9) DC Current, A (manual PDF page 17) ────────────────────────────────

// The manual prints one A range; the range table labels both A range
// bytes 20A, so the row answers either. The >5A note is the A range's.
static DC_A: ModeSpecs = ModeSpecs {
    name: "DC Current",
    page: 17,
    ranges: &[RangeSpec {
        range: None,
        label: "20.000A",
        spec: SpecInfo {
            resolution: "1mA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+50",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "Open circuit: residual ≤10 digits",
            "Accuracy valid 1%–100% of range",
            ">5A: max 10s, rest ≥15 min",
        ],
    },
};

// ── 10) AC Current, µA (manual PDF page 18) ──────────────────────────────

// Split as DC current is. The AC table prints its range labels without the
// DC table's decimals (220µA, not 220.00µA).
static AC_UA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "220µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "0.8%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+50",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "2200µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "0.8%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+50",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Open circuit: residual ≤10 digits",
            "µA ranges: min 30µA",
        ],
    },
};

// ── 10) AC Current, mA (manual PDF page 18) ──────────────────────────────

static AC_MA: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "22mA",
            spec: SpecInfo {
                resolution: "1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "1.2%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+50",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "220mA",
            spec: SpecInfo {
                resolution: "10µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~1kHz"),
                        accuracy: "1.2%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+50",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F1 Fuse 1A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Open circuit: residual ≤10 digits",
        ],
    },
};

// ── 10) AC Current, A (manual PDF page 18) ───────────────────────────────

// One A range, as for DC.
static AC_A: ModeSpecs = ModeSpecs {
    name: "AC Current",
    page: 18,
    ranges: &[RangeSpec {
        range: None,
        label: "20A",
        spec: SpecInfo {
            resolution: "1mA",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40Hz~1kHz"),
                    accuracy: "1.2%+10",
                },
                AccuracyBand {
                    freq_range: Some("1kHz~10kHz"),
                    accuracy: "3%+50",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("F2 Fuse 10A 240V Φ6x25mm"),
        notes: &[
            "True RMS; crest factor ≤2.0 at 10000, ≤1 at 22000 counts",
            "Non-sine: add 4% (crest factor 1–2), 5% (2–2.5), 7% (2.5–3)",
            "Accuracy valid 1%–100% of range (1kHz–10kHz: 10%–100%)",
            "Open circuit: residual ≤10 digits",
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
        // Printed "10Hz~220MHZ".
        label: "10Hz~220MHz",
        spec: SpecInfo {
            resolution: "0.01Hz~0.01MHz",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.01%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Hz input ≤100kHz: 200mV–20V rms",
            "Hz input >100kHz–1MHz: 600mV–20V rms",
            "Hz input >1MHz–40MHz: 1V–20V rms; >40MHz not specified",
        ],
    },
};

// ── 11) Frequency/Duty Ratio, duty (manual PDF page 18) ──────────────────

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
                accuracy: "2%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Duty: square waves only, 1Vpp–20Vpp, ≤10kHz, 10.0%–90.0%"],
    },
};
