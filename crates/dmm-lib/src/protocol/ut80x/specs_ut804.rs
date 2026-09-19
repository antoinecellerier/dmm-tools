//! Specification data for the UNI-T UT804 (40000 counts).
//!
//! Transcribed from references/ut800/ut804/ut804-manual.pdf, P/N:110401108661X
//! Jul.2019 REV.1, "Detailed Accuracy Specifications" (PDF pages 59-67,
//! printed 58-66), with values as printed but for the typesetting slips noted
//! where they are. Cross-checked against the basic specifications (PDF p. 58),
//! the UT804 datasheet and the UT800 series page. The notes are short app
//! notes condensed from the manual's remarks (the verbatim text is in the
//! local verified transcription).
//!
//! A manual table is split into parts of the same name where its rows belong
//! to different modes or need a different overload protection. Range bytes
//! follow `ut804_mode_info`, whose labels the rows carry.

use super::Coupling;
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// Every table, in manual order.
pub(super) static ALL: &[&ModeSpecs] = &[
    &DC_MV,
    &DC_V,
    &AC_V,
    &DC_UA,
    &DC_MA,
    &DC_A,
    &AC_UA,
    &AC_MA,
    &AC_A,
    &RESISTANCE,
    &CONTINUITY,
    &DIODE,
    &CAPACITANCE,
    &FREQUENCY,
    &DUTY,
    &TEMP_C,
    &TEMP_F,
    &LOOP_CURRENT,
];

/// Whether a reading with this coupling takes its row's accuracy. An AC+DC
/// reading takes B's or D's mode data only: their remarks add (1%+35
/// digits) to the table's accuracy, a sum the manual does not print.
pub(super) fn has_rows(coupling: Option<Coupling>) -> bool {
    coupling != Some(Coupling::AcDc)
}

/// The table for a reading with mode code `mode`, its coupling and whether
/// it is a duty cycle (`Ut804Fields`).
///
/// `None` where the manual has no table: ADP (mode 0xE), and AC or AC+DC
/// millivolts. The manual gives the UT804 no AC mV function: Table 2-1
/// (printed p. 14) has the mV position measure DC millivoltage, Table 2-3
/// lists only DCmV, section C is "Measuring DC Millivoltage", and AC+DC acts
/// only "at AC measurement mode". Table B has no 400mV row either.
pub(super) fn table(
    mode: u8,
    coupling: Option<Coupling>,
    duty: bool,
) -> Option<&'static ModeSpecs> {
    use Coupling::{Ac, AcDc, Dc};
    Some(match (mode, coupling) {
        (0xC, _) if duty => &DUTY,
        (0x1 | 0x2, Some(Dc)) => &DC_V,
        (0x1 | 0x2, Some(Ac | AcDc)) => &AC_V,
        (0x3, Some(Dc)) => &DC_MV,
        (0x7, Some(Dc)) => &DC_UA,
        (0x8, Some(Dc)) => &DC_MA,
        (0x9, Some(Dc)) => &DC_A,
        (0x7, Some(Ac | AcDc)) => &AC_UA,
        (0x8, Some(Ac | AcDc)) => &AC_MA,
        (0x9, Some(Ac | AcDc)) => &AC_A,
        (0x4, _) => &RESISTANCE,
        (0xA, _) => &CONTINUITY,
        (0xB, _) => &DIODE,
        (0x5, _) => &CAPACITANCE,
        (0xC, _) => &FREQUENCY,
        (0x6, _) => &TEMP_C,
        (0xD, _) => &TEMP_F,
        (0xF, _) => &LOOP_CURRENT,
        _ => return None,
    })
}

// ── A. DC Voltage, mV (manual PDF page 59) ───────────────────────────────

// The DC mV mode (0x3), whatever its range byte. The datasheet gives the
// input impedance as ~4GΩ. ±(0.025%+5) holds only "under REL mode",
// though the basic specifications (PDF p. 58), the datasheet and the web
// page quote it unconditionally.
static DC_MV: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 59,
    ranges: &[RangeSpec {
        range: None,
        label: "400mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[AccuracyBand {
                freq_range: Some("under REL mode"),
                accuracy: "0.025%+5",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("Around 2.5GΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── A. DC Voltage, V (manual PDF page 59) ────────────────────────────────

// Modes 0x1 and 0x2 with DC coupling.
static DC_V: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 59,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "4V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "40V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "400V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.05%+5",
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
                    accuracy: "0.1%+8",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("Around 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── B. AC Voltage (manual PDF page 60) ───────────────────────────────────

// Modes 0x1 and 0x2 with AC or AC+DC coupling: the title makes AC+DC
// available, and a remark gives its adder. Only the 4V and 40V rows
// specify the 100kHz bandwidth the basic specifications quote.
static AC_V: ModeSpecs = ModeSpecs {
    name: "B. AC Voltage (AC+DC measurement is available)",
    page: 60,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "4V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.4%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~10kHz"),
                        accuracy: "3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~100kHz"),
                        accuracy: "6%+30",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "40V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.4%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~10kHz"),
                        accuracy: "3%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~100kHz"),
                        accuracy: "6%+30",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "400V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.4%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~10kHz"),
                        accuracy: "5%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~100kHz"),
                        accuracy: "Not Specified",
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
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "1%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~5kHz"),
                        accuracy: "5%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">5kHz~10kHz"),
                        accuracy: "10%+30",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("Approx 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS and accuracy valid 10%–100% of range",
            "Crest factor up to 3.0 (1000V: 1.5)",
            "Shorted leads: 80-digit residual, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
        ],
    },
};

// ── C. DC Current, µA (manual PDF page 61) ───────────────────────────────

// µA, mA and A are modes of their own, and the 10A range has a fuse of its
// own. The table's header row prints Bandwidth over the accuracy and
// Accuracy over the fuse; the values are recorded by what they are.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "400µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "4000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &["10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart"],
    },
};

// ── C. DC Current, mA (manual PDF page 61) ───────────────────────────────

static DC_MA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "40mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.15%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "400mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.15%+15",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &["10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart"],
    },
};

// ── C. DC Current, A (manual PDF page 61) ────────────────────────────────

static DC_A: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 61,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.5%+30",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &["10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart"],
    },
};

// ── D. AC Current, µA (manual PDF page 62) ───────────────────────────────

// AC or AC+DC coupling, split as DC current is.
static AC_UA: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 62,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "400µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.7%+15",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~5kHz"),
                        accuracy: "1%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">5kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "4000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.7%+15",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~5kHz"),
                        accuracy: "1%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">5kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &[
            "True RMS and accuracy valid 10%–100% of range; crest factor up to 3.0",
            "Shorted leads: 80-digit residual, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart",
        ],
    },
};

// ── D. AC Current, mA (manual PDF page 62) ───────────────────────────────

static AC_MA: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 62,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "40mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.7%+15",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~5kHz"),
                        accuracy: "1%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">5kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "400mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.7%+15",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz~5kHz"),
                        accuracy: "1%+30",
                    },
                    AccuracyBand {
                        freq_range: Some(">5kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &[
            "True RMS and accuracy valid 10%–100% of range; crest factor up to 3.0",
            "Shorted leads: 80-digit residual, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart",
        ],
    },
};

// ── D. AC Current, A (manual PDF page 62) ────────────────────────────────

static AC_A: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 62,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("45Hz~1kHz"),
                    accuracy: "1.5%+40",
                },
                AccuracyBand {
                    // Printed ">1kHz~ 5kHz".
                    freq_range: Some(">1kHz~5kHz"),
                    accuracy: "2.5%+40",
                },
                AccuracyBand {
                    freq_range: Some(">5kHz~10kHz"),
                    accuracy: "5%+40",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &[
            "True RMS and accuracy valid 10%–100% of range; crest factor up to 3.0",
            "Shorted leads: 80-digit residual, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; >5A–10A ≤10s at a time, >15min apart",
        ],
    },
};

// ── E. Resistance (manual PDF page 63) ───────────────────────────────────

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "E. Resistance",
    page: 63,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "400Ω",
            spec: SpecInfo {
                resolution: "0.01Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "4kΩ",
            spec: SpecInfo {
                resolution: "0.0001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "40kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "400kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "4MΩ",
            spec: SpecInfo {
                resolution: "0.0001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "40MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+40",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["400Ω: accuracy + test-lead open-circuit value"],
    },
};

// ── F. Continuity Test (manual PDF page 63) ──────────────────────────────

// No accuracy column; the range cell is the continuity symbol.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "F. Continuity Test",
    page: 63,
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
        notes: &["Open circuit ≈1.2V", "Beeps continuously ≤10Ω; silent >50Ω"],
    },
};

// ── G. Diode Test (manual PDF page 64) ───────────────────────────────────

// No accuracy column; the range cell is the diode symbol.
static DIODE: ModeSpecs = ModeSpecs {
    name: "G. Diode Test",
    page: 64,
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
        notes: &["Open circuit ≈2.8V", "Good silicon junction: 0.5V–0.8V"],
    },
};

// ── H. Capacitance (manual PDF page 64) ──────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "H. Capacitance",
    page: 64,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "40nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "400nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "4µF",
            spec: SpecInfo {
                resolution: "0.0001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "40µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "400µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "4mF",
            spec: SpecInfo {
                resolution: "0.0001mF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "40mF",
            spec: SpecInfo {
                resolution: "0.001mF",
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
        notes: &["40nF: accuracy + open-circuit test-lead capacitance"],
    },
};

// ── I. Frequency (manual PDF page 65) ────────────────────────────────────

// Mode 0xC without the sign bit.
static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "I. Frequency",
    page: 65,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "40Hz",
            spec: SpecInfo {
                resolution: "0.001Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "400Hz",
            spec: SpecInfo {
                resolution: "0.01Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "4kHz",
            spec: SpecInfo {
                resolution: "0.0001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "40kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "400kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "4MHz",
            spec: SpecInfo {
                resolution: "0.0001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "40MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.01%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "400MHz",
            spec: SpecInfo {
                resolution: "0.01MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "Not Specified",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Input 10Hz–40MHz: 200mV–30V rms",
            "Input at zero DC level; >40MHz not specified",
        ],
    },
};

// ── J. Duty Cycle (manual PDF page 66) ───────────────────────────────────

// Mode 0xC with the sign bit, over any of the frequency range bytes.
static DUTY: ModeSpecs = ModeSpecs {
    name: "J. Duty Cycle",
    page: 66,
    ranges: &[RangeSpec {
        range: None,
        label: "100%",
        spec: SpecInfo {
            resolution: "0.01%",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.0%+40",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Valid 10%–90% of range, 5Hz–2kHz only",
            "Input 10Hz–40MHz: 200mV–30V rms",
            "Input at zero DC level; >40MHz not specified",
        ],
    },
};

// ── K. Temperature 1-1 (manual PDF page 66) ──────────────────────────────

// Mode 0x6. The spans are the bands of one range.
static TEMP_C: ModeSpecs = ModeSpecs {
    name: "K. Temperature 1-1. Degrees Celsius",
    page: 66,
    ranges: &[RangeSpec {
        range: None,
        label: "-40°C~1000°C",
        spec: SpecInfo {
            resolution: "0.1°C",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°C~40°C"),
                    accuracy: "3%+30",
                },
                AccuracyBand {
                    freq_range: Some("40°C~400°C"),
                    accuracy: "1%+30",
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
            "Included K-type point-contact probe: below 230°C only",
            "Above 230°C: use the rod contact probe",
        ],
    },
};

// ── K. Temperature 1-2 (manual PDF page 67) ──────────────────────────────

// Mode 0xD.
static TEMP_F: ModeSpecs = ModeSpecs {
    name: "K. Temperature 1-2. Fahrenheit",
    page: 67,
    ranges: &[RangeSpec {
        range: None,
        label: "-40°F~1832°F",
        spec: SpecInfo {
            resolution: "0.1°F",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°F~32°F"),
                    accuracy: "4%+50",
                },
                AccuracyBand {
                    freq_range: Some("32°F~752°F"),
                    accuracy: "1.5%+50",
                },
                AccuracyBand {
                    freq_range: Some("752°F~1832°F"),
                    accuracy: "3%",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Included K-type point-contact probe: below 230°C only",
            "Above 230°C: use the rod contact probe",
        ],
    },
};

// ── L. 4~20 mA loop current (manual PDF page 67) ─────────────────────────

// The mA% mode (0xF).
static LOOP_CURRENT: ModeSpecs = ModeSpecs {
    name: "L. 4~20 mA loop current",
    page: 67,
    ranges: &[RangeSpec {
        range: None,
        label: "(4~20mA)%",
        spec: SpecInfo {
            resolution: "0.01%",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1%+50",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 250V, fast type fuse, ø5×20mm".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &[
            "4mA shows 0%, 20mA shows 100%",
            "<4mA shows LO, >20mA shows HI",
        ],
    },
};
