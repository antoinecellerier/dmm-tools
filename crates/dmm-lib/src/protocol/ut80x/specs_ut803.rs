//! Specification data for the UNI-T UT803 (6000 counts).
//!
//! Transcribed from the UT803 operating manual (REV.3), "Accuracy
//! Specifications" (PDF pages 39-48, printed 38-47), with values as printed
//! but for the typesetting slips noted where they are. Cross-checked against
//! the UT803 datasheet and the UT800 series page. The notes are short app
//! notes condensed from the manual's remarks.
//!
//! A manual table is split into parts of the same name where its rows need
//! different mode-level data or belong to different modes. Range bytes
//! follow the decimal points in `ut803_mode_info`.

use super::Coupling;
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// Every table, in manual order.
pub(super) static ALL: &[&ModeSpecs] = &[
    &DC_MV,
    &DC_V,
    &AC_MV,
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
    &TEMP_C,
    &TEMP_F,
    &TRANSISTOR,
];

/// The table for a reading with mode code `mode`, range byte `range`, the
/// alt bit and the coupling bits (`Ut803Fields`).
///
/// `None` where the manual has no table: the tachometer (frequency with the
/// alt bit), ADP (mode 0xE), and volts or current without exactly one
/// coupling bit. The manual documents AC+DC, but it never reaches the
/// computer: "+DC, hFE and β cannot output to the computer" ("RS232
/// Button", PDF p. 37).
pub(super) fn table(
    mode: u8,
    range: u8,
    alt: bool,
    coupling: Option<Coupling>,
) -> Option<&'static ModeSpecs> {
    use Coupling::{Ac, Dc};
    Some(match (mode, coupling) {
        (0xB, Some(Dc)) if range == 4 => &DC_MV,
        (0xB, Some(Dc)) => &DC_V,
        (0xB, Some(Ac)) if range == 4 => &AC_MV,
        (0xB, Some(Ac)) => &AC_V,
        (0xD, Some(Dc)) => &DC_UA,
        (0xF, Some(Dc)) => &DC_MA,
        (0x9, Some(Dc)) => &DC_A,
        (0xD, Some(Ac)) => &AC_UA,
        (0xF, Some(Ac)) => &AC_MA,
        (0x9, Some(Ac)) => &AC_A,
        (0x3, _) => &RESISTANCE,
        (0x5, _) => &CONTINUITY,
        (0x1, _) => &DIODE,
        (0x6, _) => &CAPACITANCE,
        (0x2, _) if !alt => &FREQUENCY,
        (0x4, _) if alt => &TEMP_C,
        (0x4, _) => &TEMP_F,
        _ => return None,
    })
}

// ── A. DC Voltage, 600mV (manual PDF page 39) ────────────────────────────

// The 600mV row is range 4 of the volts mode, which reads as DC mV, and
// has an input impedance of its own.
// The datasheet gives it as "Around 3GΩ".
static DC_MV: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 39,
    ranges: &[RangeSpec {
        range: Some(4),
        label: "600mV",
        spec: SpecInfo {
            resolution: "0.1mV",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.6%+2",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("Around > 3000MΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── A. DC Voltage (manual PDF page 39) ───────────────────────────────────

// Range bytes 0-3.
static DC_V: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 39,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "6V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "60V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+2",
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
                    accuracy: "0.5%+3",
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

// ── B. AC Voltage, 600mV (manual PDF page 40) ────────────────────────────

// Range 4 of the volts mode, as for DC.
static AC_MV: ModeSpecs = ModeSpecs {
    name: "B. AC Voltage",
    page: 40,
    ranges: &[RangeSpec {
        range: Some(4),
        label: "600mV",
        spec: SpecInfo {
            resolution: "0.1mV",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40Hz–50kHz"),
                    accuracy: "0.6%+5",
                },
                AccuracyBand {
                    freq_range: Some(">50kHZ–100kHz"),
                    accuracy: "1%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("Around > 3000MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS valid 10%–95% of range",
            "Crest factor 3.0 (1000V: 1.5)",
            "Shorted input: residual under ~30 digits, accuracy unaffected",
            "AC+DC: range accuracy + 1%",
        ],
    },
};

// ── B. AC Voltage (manual PDF page 40) ───────────────────────────────────

// Range bytes 0-3; the remarks are printed on the next page (PDF p. 41).
static AC_V: ModeSpecs = ModeSpecs {
    name: "B. AC Voltage",
    page: 40,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "6V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz–1kHz"),
                        accuracy: "0.6%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz–10kHz"),
                        accuracy: "1.0%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz–100kHz"),
                        accuracy: "3%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "60V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz–1kHz"),
                        accuracy: "0.6%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz–10kHz"),
                        accuracy: "1.5%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz–20kHz"),
                        accuracy: "3%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">20kHz–100kHz"),
                        accuracy: "8%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz–1kHz"),
                        accuracy: "0.6%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz–10kHz"),
                        accuracy: "3.5%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "1000V",
            spec: SpecInfo {
                resolution: "1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz–1kHz"),
                        accuracy: "1.2%+3",
                    },
                    AccuracyBand {
                        freq_range: Some(">1kHz–3kHz"),
                        accuracy: "3%+3",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("Around 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS valid 10%–95% of range",
            "Crest factor 3.0 (1000V: 1.5)",
            "Shorted input: residual under ~30 digits, accuracy unaffected",
            "AC+DC: range accuracy + 1%",
        ],
    },
};

// ── C. DC Current, µA (manual PDF page 42) ───────────────────────────────

// One table in the manual. µA, mA and A are modes of their own, each
// numbering its ranges from 0, and the 10A range has a fuse of its own.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 42,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "600µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
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
                    accuracy: "0.5%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 500mA, 125V, fast type, ø5x20mm.".
        overload_protection: Some("Fuse 500mA 125V"),
        notes: &["≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart"],
    },
};

// ── C. DC Current, mA (manual PDF page 42) ───────────────────────────────

static DC_MA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 42,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "60mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600mA",
            spec: SpecInfo {
                resolution: "0.1mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 500mA, 125V, fast type, ø5x20mm.".
        overload_protection: Some("Fuse 500mA 125V"),
        notes: &["≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart"],
    },
};

// ── C. DC Current, A (manual PDF page 42) ────────────────────────────────

static DC_A: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 42,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "10mA",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "1.2%+3",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 10A, 250V, fast type, ø5x20mm.".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &["≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart"],
    },
};

// ── D. AC Current, µA (manual PDF page 43) ───────────────────────────────

// Split as DC current is.
static AC_UA: ModeSpecs = ModeSpecs {
    name: "D. AC Current",
    page: 43,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "600µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~10kHz"),
                        accuracy: "1.0%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~15kHz"),
                        accuracy: "2%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "6000µA",
            spec: SpecInfo {
                resolution: "1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~10kHz"),
                        accuracy: "1.0%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~15kHz"),
                        accuracy: "2%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 500mA, 125V, fast type,ø5x20mm.".
        overload_protection: Some("Fuse 500mA 125V"),
        notes: &[
            "True RMS valid 10%–95% of range; crest factor 3.0",
            "Shorted input: residual under ~30 digits, accuracy unaffected",
            "AC+DC: range accuracy + 1%",
            "≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart",
        ],
    },
};

// ── D. AC Current, mA (manual PDF page 43) ───────────────────────────────

static AC_MA: ModeSpecs = ModeSpecs {
    name: "D. AC Current",
    page: 43,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "60mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~10kHz"),
                        accuracy: "1.0%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~15kHz"),
                        accuracy: "2%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "600mA",
            spec: SpecInfo {
                resolution: "0.1mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40Hz~10kHz"),
                        accuracy: "1%+5",
                    },
                    AccuracyBand {
                        freq_range: Some(">10kHz~15kHz"),
                        accuracy: "3%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 500mA, 125V, fast type,ø5x20mm.".
        overload_protection: Some("Fuse 500mA 125V"),
        notes: &[
            "True RMS valid 10%–95% of range; crest factor 3.0",
            "Shorted input: residual under ~30 digits, accuracy unaffected",
            "AC+DC: range accuracy + 1%",
            "≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart",
        ],
    },
};

// ── D. AC Current, A (manual PDF page 43) ────────────────────────────────

static AC_A: ModeSpecs = ModeSpecs {
    name: "D. AC Current",
    page: 43,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "10mA",
            accuracy: &[AccuracyBand {
                freq_range: Some("40Hz~5kHz"),
                accuracy: "2.0%+6",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 10A,250V, fast type, ø5x20mm.".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &[
            "True RMS valid 10%–95% of range; crest factor 3.0",
            "Shorted input: residual under ~30 digits, accuracy unaffected",
            "AC+DC: range accuracy + 1%",
            "≤5A range: continuous; >5A range: ≤10s at a time, ≥15min apart",
        ],
    },
};

// ── E. Resistance (manual PDF page 44) ───────────────────────────────────

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "E. Resistance",
    page: 44,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "600Ω",
            spec: SpecInfo {
                resolution: "0.1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "6kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "60kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "600kΩ",
            spec: SpecInfo {
                resolution: "0.1kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "6MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+2",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "60MΩ",
            spec: SpecInfo {
                resolution: "0.01MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.2%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &["600Ω: accuracy + test-lead short-circuit resistance"],
    },
};

// ── F. Continuity Test (manual PDF page 45) ──────────────────────────────

// The table has no accuracy column; the range cell is the continuity symbol.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "F. Continuity Test",
    page: 45,
    ranges: &[RangeSpec {
        range: None,
        label: "(continuity symbol)",
        spec: SpecInfo {
            resolution: "1Ω",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &[
            "Open circuit ≈-1.2V",
            "Beeps continuously ≤10Ω; silent >30Ω",
        ],
    },
};

// ── G. Diode Testst (manual PDF page 45) ─────────────────────────────────

// Title as printed. No accuracy column; the range cell is the diode symbol.
static DIODE: ModeSpecs = ModeSpecs {
    name: "G. Diode Testst",
    page: 45,
    ranges: &[RangeSpec {
        range: None,
        label: "(diode symbol)",
        spec: SpecInfo {
            resolution: "10mV",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &["Open circuit ≈2.7V", "Working current ≈1mA"],
    },
};

// ── H. Capacitance (manual PDF page 46) ──────────────────────────────────

// Range bytes 0-6. The parser also knows range 7 (60mF), which the manual
// does not list.
static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "H. Capacitance",
    page: 46,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "6nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.5%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
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
            range: Some(2),
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
            range: Some(3),
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
            range: Some(4),
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
            range: Some(5),
            label: "600µF",
            spec: SpecInfo {
                resolution: "0.1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3%+4",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "6mF",
            spec: SpecInfo {
                resolution: "0.001mF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5%+4",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &["6nF, 60nF, 600nF: subtract open-circuit test-lead capacitance"],
    },
};

// ── I. Frequency (manual PDF page 47) ────────────────────────────────────

// Range bytes 0-4, frequency only (alt bit clear): the manual gives the
// tachometer no spec. The parser also knows range 5 (600MHz), which the
// manual does not list.
static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "I. Frequency",
    page: 47,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "6kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "60kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "600kHz",
            spec: SpecInfo {
                resolution: "0.1kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "6MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "60MHz",
            spec: SpecInfo {
                resolution: "0.01MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+3",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &[
            "Input 10Hz–1MHz: 150mV–30V rms",
            "Input >1MHz–10MHz: 300mV–30V rms",
            "Input >10MHz–50MHz: 600mV–30V rms",
            "Input at zero DC level; >50MHz unspecified",
        ],
    },
};

// ── J. Temperature, °C (manual PDF page 48) ──────────────────────────────

// One table in the manual, one row per unit; the alt bit picks °C.
static TEMP_C: ModeSpecs = ModeSpecs {
    name: "J. Temperature",
    page: 48,
    ranges: &[RangeSpec {
        range: None,
        label: "°C",
        spec: SpecInfo {
            resolution: "1°C",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°C~0°C"),
                    accuracy: "8%+5",
                },
                AccuracyBand {
                    freq_range: Some(">0°C~400°C"),
                    accuracy: "1%+3",
                },
                AccuracyBand {
                    freq_range: Some(">400°C~1000°C"),
                    accuracy: "1.5%+3",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &[
            "Included point-contact probe: below 230°C only",
            "Above 230°C: use the rod-type probe",
        ],
    },
};

// ── J. Temperature, °F (manual PDF page 48) ──────────────────────────────

// The first band is printed in °C.
static TEMP_F: ModeSpecs = ModeSpecs {
    name: "J. Temperature",
    page: 48,
    ranges: &[RangeSpec {
        range: None,
        label: "°F",
        spec: SpecInfo {
            resolution: "1°F",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("-40°C~32°C"),
                    accuracy: "8%+5",
                },
                AccuracyBand {
                    freq_range: Some(">32°F~752°F"),
                    accuracy: "1.5%+5",
                },
                AccuracyBand {
                    freq_range: Some(">752°F~1832°F"),
                    accuracy: "2.5%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("250V rms"),
        notes: &[
            "Included point-contact probe: below 230°C only",
            "Above 230°C: use the rod-type probe",
        ],
    },
};

// ── K. Transistor (manual PDF page 48) ───────────────────────────────────

// No reading reaches this table: the manual says hFE is not sent to the
// computer ("RS232 Button", PDF p. 37), and mode 0xE (ADP) being hFE is
// only a guess. No accuracy column.
static TRANSISTOR: ModeSpecs = ModeSpecs {
    name: "K. Transistor",
    page: 48,
    ranges: &[RangeSpec {
        range: None,
        label: "hFE",
        spec: SpecInfo {
            resolution: "1β",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "Fuse 200mA, 250V, fast type, ø5x20mm.; Fuse 500mA, 125V, fast type, ø5x20mm".
        overload_protection: Some("Fuse 200mA 250V; Fuse 500mA 125V"),
        notes: &["Vce ≈2.2V, bo ≈10µA", "Max 1000β"],
    },
};
