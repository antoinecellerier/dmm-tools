//! Specification data for the UNI-T UT181A (60000 counts).
//!
//! Transcribed from references/ut181/ut181-user-manual.pdf (PDF title
//! "UTI181A English manual", no revision printed, file modified 2024-06-22),
//! "2. Electrical Specifications", tables (1)-(15) on PDF pages 12-19
//! (printed 36-43), with values as printed but for the typesetting slips noted
//! where they are. Cross-checked against the UT181 series flyer
//! (ut181a-data-sheet.pdf p. 4) and the product pages on meters.uni-trend.com
//! and uni-trendus.com. The notes are short app notes condensed from the
//! manual's remarks (the verbatim text is in the local verified
//! transcription).
//!
//! A manual table is split into parts of the same name where its rows belong
//! to different dial positions or need a different overload protection.
//! Range bytes are 1-based indexes into the position's range ladder
//! (research spec §7).

use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// Every table, in manual order.
pub(super) static ALL: &[&ModeSpecs] = &[
    &AC_MV,
    &AC_V,
    &DC_MV,
    &DC_V,
    &ACDC_MV,
    &ACDC_V,
    &AC_UA,
    &AC_MA,
    &AC_A,
    &DC_UA,
    &DC_MA,
    &DC_A,
    &ACDC_UA,
    &ACDC_MA,
    &ACDC_A,
    &RESISTANCE,
    &CONDUCTANCE,
    &CAPACITANCE,
    &TEMP_C,
    &TEMP_F,
    &FREQUENCY,
    &DUTY,
    &PULSE_WIDTH,
    &CONTINUITY,
    &DIODE,
];

/// The table for a reading in mode word `word`, with REL already taken off
/// (`mode::plain_word`).
///
/// The Hz variants take their AC table: the main reading stays the AC
/// quantity, with the frequency as a sub-value (golden `vac_hz_mains`).
/// `None` for the readings the manual gives no spec: the Peak, LPF, dBV and
/// dBm variants, and the T1-T2 and T2-T1 temperature differences.
pub(super) fn table(word: u16) -> Option<&'static ModeSpecs> {
    Some(match word {
        0x1111 | 0x1121 => &AC_V,
        0x2111 | 0x2121 => &AC_MV,
        0x2141 => &ACDC_MV,
        0x3111 => &DC_V,
        0x3121 => &ACDC_V,
        0x4111 => &DC_MV,
        // A REL reading reaches here as its plain word: REL is a display mode
        // of the same reading, while T1-T2 and T2-T1 combine two probes.
        0x4211 | 0x4221 => &TEMP_C,
        0x4311 | 0x4321 => &TEMP_F,
        0x5111 => &RESISTANCE,
        0x5211 | 0x5212 => &CONTINUITY,
        0x5311 => &CONDUCTANCE,
        0x6111 | 0x6112 => &DIODE,
        0x6211 => &CAPACITANCE,
        0x7111 => &FREQUENCY,
        0x7211 => &DUTY,
        0x7311 => &PULSE_WIDTH,
        0x8111 => &DC_UA,
        0x8121 => &ACDC_UA,
        0x8211 | 0x8221 => &AC_UA,
        0x9111 => &DC_MA,
        0x9121 => &ACDC_MA,
        0x9211 | 0x9221 => &AC_MA,
        0xA111 => &DC_A,
        0xA121 => &ACDC_A,
        0xA211 | 0xA221 => &AC_A,
        _ => return None,
    })
}

// ── (1) AC Voltage, mV (manual PDF page 12) ──────────────────────────────

// mV AC (0x21xx).
static AC_MV: ModeSpecs = ModeSpecs {
    name: "(1) AC Voltage",
    page: 12,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mV",
            spec: SpecInfo {
                resolution: "0.001mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+60",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+60",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~20kHz"),
                        accuracy: "3%+60",
                    },
                    AccuracyBand {
                        freq_range: Some("20k~100kHz"),
                        accuracy: "4%+60",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mV",
            spec: SpecInfo {
                resolution: "0.01mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~20kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("20k~100kHz"),
                        accuracy: "4%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &["True RMS valid 10%–100% of range"],
    },
};

// ── (1) AC Voltage, V (manual PDF page 12) ───────────────────────────────

// V AC (0x11xx).
static AC_V: ModeSpecs = ModeSpecs {
    name: "(1) AC Voltage",
    page: 12,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "6V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~20kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("20k~100kHz"),
                        accuracy: "4%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~20kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("20k~100kHz"),
                        accuracy: "4%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~20kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("20k~100kHz"),
                        accuracy: "Only for reference",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "1000V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+30",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~5kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("5k~10kHz"),
                        accuracy: "6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~100kHz"),
                        accuracy: "Only for reference",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &["True RMS valid 10%–100% of range"],
    },
};

// ── (2) DC Voltage, mV (manual PDF page 13) ──────────────────────────────

// mV DC (0x41xx).
static DC_MV: ModeSpecs = ModeSpecs {
    name: "(2) DC Voltage",
    page: 13,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mV",
            spec: SpecInfo {
                resolution: "0.001mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.025%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mV",
            spec: SpecInfo {
                resolution: "0.01mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.025%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &["60mV: use REL to compensate bias voltage"],
    },
};

// ── (2) DC Voltage, V (manual PDF page 13) ───────────────────────────────

// V DC (0x31xx). The product page gives the input impedance as "≥10MΩ".
static DC_V: ModeSpecs = ModeSpecs {
    name: "(2) DC Voltage",
    page: 13,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "6V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.025%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.025%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.03%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "1000V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.03%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &["60mV: use REL to compensate bias voltage"],
    },
};

// ── (3) AC Voltage + DC Voltage, mV (manual PDF page 13) ─────────────────

// mV AC's AC+DC variant (0x2141).
static ACDC_MV: ModeSpecs = ModeSpecs {
    name: "(3) AC Voltage + DC Voltage",
    page: 13,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mV",
            spec: SpecInfo {
                resolution: "0.001mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "6%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mV",
            spec: SpecInfo {
                resolution: "0.01mV",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "6%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        // Printed "1,000V".
        overload_protection: Some("1000V"),
        notes: &["True RMS valid 10%–100% of range"],
    },
};

// ── (3) AC Voltage + DC Voltage, V (manual PDF page 13) ──────────────────

// V DC's AC+DC variant (0x3121).
static ACDC_V: ModeSpecs = ModeSpecs {
    name: "(3) AC Voltage + DC Voltage",
    page: 13,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "6V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "6%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "6%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "Only for reference",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "Only for reference",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "1000V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "1.2%+80",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "Only for reference",
                    },
                    AccuracyBand {
                        freq_range: Some("10k~35kHz"),
                        accuracy: "Only for reference",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        // Printed "1,000V".
        overload_protection: Some("1000V"),
        notes: &["True RMS valid 10%–100% of range"],
    },
};

// ── (4) AC Current, µA (manual PDF page 14) ──────────────────────────────

// µA, mA and A are dial positions of their own, and the 10A range has a
// fuse of its own. µA AC (0x82xx).
static AC_UA: ModeSpecs = ModeSpecs {
    name: "(4) AC Current",
    page: 14,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (4) AC Current, mA (manual PDF page 14) ──────────────────────────────

// mA AC (0x92xx).
static AC_MA: ModeSpecs = ModeSpecs {
    name: "(4) AC Current",
    page: 14,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45~1kHz"),
                        accuracy: "0.6%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "1.2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (4) AC Current, A (manual PDF page 14) ───────────────────────────────

// A AC (0xA2xx), one fixed range.
static AC_A: ModeSpecs = ModeSpecs {
    name: "(4) AC Current",
    page: 14,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("45~1kHz"),
                    accuracy: "1%+20",
                },
                AccuracyBand {
                    freq_range: Some("1k~10kHz"),
                    accuracy: "3%+40",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Φ10x38mm".
        overload_protection: Some("10A H 1000V Fuse Type ø10x38mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (5) DC Current, µA (manual PDF page 15) ──────────────────────────────

// µA DC (0x81xx).
static DC_UA: ModeSpecs = ModeSpecs {
    name: "(5) DC Current",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &["20A: 30s on, then 10min off; not specified above 10A"],
    },
};

// ── (5) DC Current, mA (manual PDF page 15) ──────────────────────────────

// mA DC (0x91xx).
static DC_MA: ModeSpecs = ModeSpecs {
    name: "(5) DC Current",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.08%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.15%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &["20A: 30s on, then 10min off; not specified above 10A"],
    },
};

// ── (5) DC Current, A (manual PDF page 15) ───────────────────────────────

// A DC (0xA1xx), one fixed range.
static DC_A: ModeSpecs = ModeSpecs {
    name: "(5) DC Current",
    page: 15,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.5%+10",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Φ10x38mm".
        overload_protection: Some("10A H 1000V Fuse Type ø10x38mm"),
        notes: &["20A: 30s on, then 10min off; not specified above 10A"],
    },
};

// ── (6) AC Current + DC Current, µA (manual PDF page 15) ─────────────────

// The DC current positions' AC+DC variants: µA (0x8121).
static ACDC_UA: ModeSpecs = ModeSpecs {
    name: "(6) AC Current + DC Current",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "0.8%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "2.0%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "0.8%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "2.0%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (6) AC Current + DC Current, mA (manual PDF page 15) ─────────────────

// mA (0x9121).
static ACDC_MA: ModeSpecs = ModeSpecs {
    name: "(6) AC Current + DC Current",
    page: 15,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "0.8%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "2.0%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("50~1kHz"),
                        accuracy: "0.8%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("1k~10kHz"),
                        accuracy: "2.0%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0. 8A H 1000V Fuse Type Φ 6x32 mm".
        overload_protection: Some("0.8A H 1000V Fuse Type ø6x32mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (6) AC Current + DC Current, A (manual PDF page 15) ──────────────────

// A (0xA121).
static ACDC_A: ModeSpecs = ModeSpecs {
    name: "(6) AC Current + DC Current",
    page: 15,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("50~1kHz"),
                    accuracy: "1.2%+20",
                },
                AccuracyBand {
                    freq_range: Some("1k~10kHz"),
                    accuracy: "3%+40",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Φ10x38mm".
        overload_protection: Some("10A H 1000V Fuse Type ø10x38mm"),
        notes: &[
            "True RMS valid 10%–100% of range",
            "20A: 30s on, then 10min off; not specified above 10A",
        ],
    },
};

// ── (7) Resistance (manual PDF page 16) ──────────────────────────────────

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "(7) Resistance",
    page: 16,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "600Ω",
            spec: SpecInfo {
                resolution: "0.01Ω",
                accuracy: &[AccuracyBand {
                    freq_range: Some("In REL state"),
                    accuracy: "0.05%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "6kΩ",
            spec: SpecInfo {
                resolution: "0.0001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "60kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "600kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "6MΩ",
            spec: SpecInfo {
                resolution: "0.0001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "60MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "1,000V".
        overload_protection: Some("1000V"),
        notes: &["60MΩ: humidity <50%"],
    },
};

// ── (8) Conductance (manual PDF page 16) ─────────────────────────────────

// The flyer and the product page call it "Admittance".
static CONDUCTANCE: ModeSpecs = ModeSpecs {
    name: "(8) Conductance",
    page: 16,
    ranges: &[RangeSpec {
        range: None,
        label: "60nS",
        spec: SpecInfo {
            resolution: "0.01nS",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "2%+10",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Humidity <50%"],
    },
};

// ── (9) Capacitance (manual PDF page 17) ─────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "(9) Capacitance",
    page: 17,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "6nF",
            spec: SpecInfo {
                // Printed "0.001 nF".
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.5%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600nF",
            spec: SpecInfo {
                resolution: "0.1nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "6µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "60µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "600µF",
            spec: SpecInfo {
                resolution: "0.1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "6mF",
            spec: SpecInfo {
                resolution: "1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(8),
            label: "60mF",
            spec: SpecInfo {
                resolution: "10µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "Not specified",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Display digits: 6000"],
    },
};

// ── (10) Temperature, °C (manual PDF page 17) ────────────────────────────

// One table in the manual, one row per unit, for either channel. The °C
// position (0x42xx); its T1-T2 and T2-T1 differences have no spec.
static TEMP_C: ModeSpecs = ModeSpecs {
    name: "(10) Temperature",
    page: 17,
    ranges: &[RangeSpec {
        range: None,
        label: "°C",
        spec: SpecInfo {
            resolution: "0.1°C",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°C~40°C"),
                    accuracy: "2.0%+30",
                },
                AccuracyBand {
                    freq_range: Some("40°C~400°C"),
                    accuracy: "1.0%+30",
                },
                AccuracyBand {
                    freq_range: Some("400°C~1000°C"),
                    accuracy: "2.5%",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "K-type thermocouple; two channels via temperature connectors",
            "Accessory point-contact probe: below 230°C only",
        ],
    },
};

// ── (10) Temperature, °F (manual PDF page 17) ────────────────────────────

// The °F position (0x43xx).
static TEMP_F: ModeSpecs = ModeSpecs {
    name: "(10) Temperature",
    page: 17,
    ranges: &[RangeSpec {
        range: None,
        label: "°F",
        spec: SpecInfo {
            resolution: "0.2°F",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°F~104°F"),
                    accuracy: "2.5%+50",
                },
                AccuracyBand {
                    freq_range: Some("104°F~752°F"),
                    accuracy: "1.5%+50",
                },
                AccuracyBand {
                    freq_range: Some("752°F~1832°F"),
                    accuracy: "2.5%",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "K-type thermocouple; two channels via temperature connectors",
            "Accessory point-contact probe: below 230°C only",
        ],
    },
};

// ── (11) Frequency (manual PDF page 18) ──────────────────────────────────

// The Hz position (0x71xx).
static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "(11) Frequency",
    page: 18,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "60Hz",
            spec: SpecInfo {
                // Printed "0.001 Hz".
                resolution: "0.001Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.02%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600Hz",
            spec: SpecInfo {
                // Printed "0.01 Hz".
                resolution: "0.01Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "6kHz",
            spec: SpecInfo {
                resolution: "0.0001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "60kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "600kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "6MHz",
            spec: SpecInfo {
                resolution: "0.0001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "60MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Input 10Hz–30MHz: 600mV–30V rms",
            "Input >30MHz: not specified",
        ],
    },
};

// ── (12) Duty Cycle (manual PDF page 18) ─────────────────────────────────

// One range in the manual, for any range byte: the vendor ladder's four
// rungs are unnamed (research spec §7.1).
static DUTY: ModeSpecs = ModeSpecs {
    name: "(12) Duty Cycle",
    page: 18,
    ranges: &[RangeSpec {
        range: None,
        label: "10%~90%(10Hz~2kHz)",
        spec: SpecInfo {
            resolution: "0.01%",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+30",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["When rise time <1µs, signals center on trigger level"],
    },
};

// ── (13) Pulse Width (manual PDF page 18) ────────────────────────────────

// One range in the manual, for any range byte: the vendor ladder's four
// rungs are unnamed (research spec §7.1).
static PULSE_WIDTH: ModeSpecs = ModeSpecs {
    name: "(13) Pulse Width",
    page: 18,
    ranges: &[RangeSpec {
        range: None,
        label: "250mS",
        spec: SpecInfo {
            resolution: "0.001mS~0.01mS",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+30",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "When rise time <1µs, signals center on trigger level",
            "10Hz–200kHz: pulse width >2µs, depends on signal frequency",
        ],
    },
};

// ── (14) Continuity Test (manual PDF page 19) ────────────────────────────

// No accuracy column. The remarks cover the open-circuit beeper (0x5212)
// as well as the short-circuit one.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "(14) Continuity Test",
    page: 19,
    ranges: &[RangeSpec {
        range: None,
        label: "(continuity symbol)",
        spec: SpecInfo {
            resolution: "0.01Ω",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Open circuit ≈3V",
            "Short alarm: beeps <10Ω, silent >50Ω",
            "Open alarm: beeps >50Ω, silent <10Ω",
        ],
    },
};

// ── (15) Diode Test (manual PDF page 19) ─────────────────────────────────

// No accuracy column. The remarks cover the alarm function (0x6112).
static DIODE: ModeSpecs = ModeSpecs {
    name: "(15) Diode Test",
    page: 19,
    ranges: &[RangeSpec {
        range: None,
        label: "(diode symbol)",
        spec: SpecInfo {
            resolution: "0.0001V",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Open circuit ≈3V; forward drop up to ≈3V",
            "Brief beep on a normal junction, continuous if shorted",
            "Normal silicon junction: 0.5V–0.8V",
        ],
    },
};

#[cfg(test)]
mod tests {
    use super::super::Ut181aProtocol;
    use super::super::mode::{self, is_known_range, known_words, plain_word};
    use super::super::parse::lookup_range_label;
    use super::*;
    use crate::measurement::Measurement;
    use crate::protocol::Protocol;
    use std::collections::HashSet;

    fn reading(word: u16, range: u8) -> Measurement {
        Measurement {
            mode_raw: word,
            range_raw: range,
            ..Measurement::from_payload(&[])
        }
    }

    /// Why a reading has no spec, and which (word, range) pairs that is.
    type NoSpec = (&'static str, fn(u16, u8) -> bool);

    /// Readings the parser knows that have no spec in the manual.
    const NO_SPEC: &[NoSpec] = &[
        (
            "range 0 on a position with named ranges: auto, with no range reported",
            |word, range| range == 0 && !lookup_range_label(word, 1).is_empty(),
        ),
        (
            "Peak: the manual gives peak detection no spec",
            |word, _| {
                matches!(
                    word,
                    0x1131
                        | 0x2131
                        | 0x3131
                        | 0x4121
                        | 0x8131
                        | 0x8231
                        | 0x9131
                        | 0x9231
                        | 0xA131
                        | 0xA231
                )
            },
        ),
        (
            "LPF: the manual gives the low-pass filter no spec",
            |word, _| plain_word(word) == 0x1141,
        ),
        ("dBV and dBm: the manual gives them no spec", |word, _| {
            matches!(plain_word(word), 0x1151 | 0x1161)
        }),
        (
            "T1-T2 and T2-T1: the manual gives a difference of the two channels no spec",
            |word, _| matches!(word, 0x4231 | 0x4241 | 0x4331 | 0x4341),
        ),
    ];

    /// Every (word, range) pair the parser knows.
    fn known_readings() -> Vec<(u16, u8)> {
        known_words()
            .into_iter()
            .flat_map(|word| (0..=u8::MAX).map(move |range| (word, range)))
            .filter(|&(word, range)| is_known_range(word, range))
            .collect()
    }

    /// Each reading resolves a spec or is listed as having none, and every
    /// row of every table is some reading's.
    #[test]
    fn every_reading_has_a_spec_or_is_listed() {
        let proto = Ut181aProtocol::new();
        let mut rows_reached = HashSet::new();
        let mut listed_reached = [false; NO_SPEC.len()];
        for (word, range) in known_readings() {
            let m = reading(word, range);
            let listed: Vec<usize> = (0..NO_SPEC.len())
                .filter(|&i| (NO_SPEC[i].1)(word, range))
                .collect();
            match (proto.spec_info(&m), listed.is_empty()) {
                (Some(spec), true) => {
                    let row = table(plain_word(word)).and_then(|t| t.row(range)).unwrap();
                    assert!(std::ptr::eq(spec, &row.spec));
                    rows_reached.insert(std::ptr::from_ref(row));
                }
                (None, false) => listed.iter().for_each(|&i| listed_reached[i] = true),
                (Some(_), false) => panic!("{word:#06x} range {range}: has a spec, yet is listed"),
                (None, true) => panic!("{word:#06x} range {range}: no spec, and not listed"),
            }
        }
        for (i, (why, _)) in NO_SPEC.iter().enumerate() {
            assert!(listed_reached[i], "no reading is {why}");
        }
        for t in ALL {
            for row in t.ranges {
                assert!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    "{} / {} is no reading's",
                    t.name,
                    row.label
                );
            }
        }
    }

    /// A row answers the range byte the parser labels the same way. The
    /// parser writes Ω as the ohm sign, the manual as omega.
    #[test]
    fn rows_carry_the_parsers_range_labels() {
        for (word, range) in known_readings() {
            let Some(row) = table(plain_word(word)).and_then(|t| t.row(range)) else {
                continue;
            };
            if row.range.is_none() {
                continue;
            }
            assert_eq!(
                row.label,
                lookup_range_label(word, range).replace('\u{2126}', "\u{3a9}"),
                "{word:#06x} range {range}"
            );
        }
    }

    /// Range 0 has no row in a table keyed by range byte, yet the mode's
    /// spec shows; a table of one range answers it.
    #[test]
    fn range_0_keeps_the_mode_spec() {
        let proto = Ut181aProtocol::new();
        let m = reading(0x3111, 0);
        assert!(proto.spec_info(&m).is_none());
        assert_eq!(
            proto.mode_spec_info(&m).unwrap().input_impedance,
            Some("About 10MΩ")
        );
        let resolution = |word| proto.spec_info(&reading(word, 0)).map(|s| s.resolution);
        assert_eq!(resolution(0x4211), Some("0.1°C"));
        assert_eq!(resolution(0xA111), Some("0.001A"));
    }

    /// What a volts or current reading measures.
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Coupling {
        Ac,
        Dc,
        AcDc,
    }

    /// The coupling a reading in `word` measures, from the word alone: its
    /// dial family's, or AC+DC for the family's AC+DC variant (research spec
    /// §6.1). `None` for the positions that measure neither.
    fn word_coupling(word: u16) -> Option<Coupling> {
        let word = plain_word(word);
        let variant = (word >> 4) & 0xF;
        match (mode::family(word), variant) {
            (0x2100, 4) | (0x3100 | 0x8100 | 0x9100 | 0xA100, 2) => Some(Coupling::AcDc),
            (0x1100 | 0x2100 | 0x8200 | 0x9200 | 0xA200, _) => Some(Coupling::Ac),
            (0x3100 | 0x4100 | 0x8100 | 0x9100 | 0xA100, _) => Some(Coupling::Dc),
            _ => None,
        }
    }

    /// The coupling a table's title names, `None` for the other tables.
    fn table_coupling(table: &ModeSpecs) -> Option<Coupling> {
        match table.name {
            "(1) AC Voltage" | "(4) AC Current" => Some(Coupling::Ac),
            "(2) DC Voltage" | "(5) DC Current" => Some(Coupling::Dc),
            "(3) AC Voltage + DC Voltage" | "(6) AC Current + DC Current" => Some(Coupling::AcDc),
            _ => None,
        }
    }

    /// A reading's table measures what the reading does: AC, DC, AC+DC, or
    /// none of them.
    #[test]
    fn tables_match_the_readings_coupling() {
        for (word, range) in known_readings() {
            let Some(t) = table(plain_word(word)) else {
                continue;
            };
            assert_eq!(
                table_coupling(t),
                word_coupling(word),
                "{word:#06x} range {range}: {}",
                t.name
            );
        }
    }

    /// The unit a reading in `word` is in, prefix aside, from its dial
    /// position alone.
    fn word_unit(word: u16) -> &'static str {
        match mode::family(plain_word(word)) {
            0x1100 | 0x2100 | 0x3100 | 0x4100 | 0x6100 => "V",
            0x4200 => "°C",
            0x4300 => "°F",
            0x5100 | 0x5200 => "Ω",
            0x5300 => "S",
            0x6200 => "F",
            0x7100 => "Hz",
            0x7200 => "%",
            // The manual prints the pulse width's milliseconds as "mS".
            0x7300 => "S",
            0x8100 | 0x8200 | 0x9100 | 0x9200 | 0xA100 | 0xA200 => "A",
            other => panic!("{other:#06x} is no dial position"),
        }
    }

    /// A row's resolution is in the reading's unit, prefix aside: a row of
    /// another quantity (continuity for diode, °F for °C) resolves in
    /// another unit. Pulse width's resolution is a span, read up to its `~`.
    #[test]
    fn rows_resolve_in_the_readings_unit() {
        use crate::protocol::test_support::unit_family;
        let proto = Ut181aProtocol::new();
        for (word, range) in known_readings() {
            let Some(spec) = proto.spec_info(&reading(word, range)) else {
                continue;
            };
            let unit = spec
                .resolution
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
            let unit = unit.split('~').next().unwrap_or(unit);
            assert_eq!(
                unit_family(unit),
                word_unit(word),
                "{word:#06x} range {range}: resolution {}",
                spec.resolution
            );
        }
    }

    #[test]
    fn a_rel_reading_takes_its_functions_table() {
        assert!(std::ptr::eq(table(plain_word(0x1112)).unwrap(), &AC_V));
        let spec = Ut181aProtocol::new()
            .spec_info(&reading(0x1112, 1))
            .unwrap();
        assert_eq!(spec.resolution, "0.0001V");
    }

    /// Continuity's open-circuit beeper and the diode alarm are functions of
    /// their own, not REL words, so `plain_word` leaves them; the manual's
    /// continuity and diode tables cover them in their remarks.
    #[test]
    fn continuity_open_and_diode_alarm_keep_their_words() {
        assert_eq!(plain_word(0x5212), 0x5212);
        assert_eq!(plain_word(0x6112), 0x6112);
        assert!(!mode::rel_supported(0x5212));
        assert!(std::ptr::eq(table(0x5212).unwrap(), &CONTINUITY));
        assert!(std::ptr::eq(table(0x6112).unwrap(), &DIODE));
    }

    #[test]
    fn a_hz_variant_takes_its_ac_table() {
        for (hz, ac) in [
            (0x1121, 0x1111),
            (0x2121, 0x2111),
            (0x8221, 0x8211),
            (0x9221, 0x9211),
            (0xA221, 0xA211),
        ] {
            assert!(std::ptr::eq(table(hz).unwrap(), table(ac).unwrap()));
        }
    }

    /// A fixed-range temperature reading carries range 1.
    #[test]
    fn temperature_reads_at_range_1() {
        let proto = Ut181aProtocol::new();
        let spec = |word| proto.spec_info(&reading(word, 1)).map(|s| s.resolution);
        assert_eq!(spec(0x4211), Some("0.1°C"));
        assert_eq!(spec(0x4221), Some("0.1°C"));
        assert_eq!(spec(0x4311), Some("0.2°F"));
        assert_eq!(spec(0x4231), None);
    }
}
