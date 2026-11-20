//! Specification data for the UNI-T UT71A (20000 counts).
//!
//! The UT71A column of the UT71A/B/C/D/E operating manual's tables
//! (P/N:110401111273X, 2022.8.18 REV.3), "Accuracy Specifications" A-I (PDF
//! pages 63-70, printed 60-67), with values as printed but for the
//! typesetting slips noted where they are. The notes are short app notes
//! condensed from the manual's remarks. The UT71A has no temperature, 4-20 mA
//! or power function, so tables J-L are not its; how a reading picks a part
//! is in `specs_ut71`.

use super::specs_ut71::Ut71Tables;
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
];

/// The tables `specs_ut71::table` picks from.
pub(super) static TABLES: Ut71Tables = Ut71Tables {
    all: ALL,
    dc_mv: &DC_MV,
    dc_v: &DC_V,
    ac_v: &AC_V,
    dc_ua: &DC_UA,
    dc_ma: &DC_MA,
    dc_a: &DC_A,
    ac_ua: &AC_UA,
    ac_ma: &AC_MA,
    ac_a: &AC_A,
    resistance: &RESISTANCE,
    continuity: &CONTINUITY,
    diode: &DIODE,
    capacitance: &CAPACITANCE,
    frequency: &FREQUENCY,
    temp_c: None,
    temp_f: None,
    loop_current: None,
    power: None,
};

// ── A. DC Voltage, mV (manual PDF page 63) ─────────────────────────────────

// The DC mV mode (0x3), whatever its range byte, with an input
// impedance of its own.
static DC_MV: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 63,
    ranges: &[RangeSpec {
        range: None,
        label: "200mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.1%+8",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("About 2.5GΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── A. DC Voltage, V (manual PDF page 63) ──────────────────────────────────

// Modes 0x1 and 0x2 with DC coupling.
static DC_V: ModeSpecs = ModeSpecs {
    name: "A. DC Voltage",
    page: 63,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "2V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "20V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+8",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "200V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+8",
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
                    accuracy: "0.15%+8",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── B. AC Voltage (manual PDF page 64) ─────────────────────────────────────

// Modes 0x1 and 0x2 with AC or AC+DC coupling: the title makes AC+DC
// available, and a remark gives its adder.
static AC_V: ModeSpecs = ModeSpecs {
    name: "B. AC Voltage (AC+DC measurement is available)",
    page: 64,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "2V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.8%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10kHz~100kHz"),
                        accuracy: "7%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "20V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.8%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "3%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10kHz~100kHz"),
                        accuracy: "7%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "200V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "0.8%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "5%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("10kHz~100kHz"),
                        accuracy: "Not specified",
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
                        accuracy: "1.5%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~5kHz"),
                        accuracy: "6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("5kHz~10kHz"),
                        accuracy: "10%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("About 10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS valid 10%–100% of range; crest factor ≤3.0 (1000V: 1.5)",
            "Shorted leads: residual 80 digits, accuracy unaffected",
            "Below 100kHz: accuracy guaranteed 10%–100%",
            "AC+DC: add 1%+35 digits to accuracy",
        ],
    },
};

// ── C. DC Current, µA (manual PDF page 66) ─────────────────────────────────

// µA, mA and A are modes of their own, and the 10A range has a fuse of
// its own. The Chinese manual gives ø5×20mm 250V fuses; the English
// REV.3 table's are kept.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 66,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "200µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "2000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+20",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 1000V, fast type fuse, ø6.3x32mm".
        overload_protection: Some("Fuse 0.5A 1000V"),
        notes: &["10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min"],
    },
};

// ── C. DC Current, mA (manual PDF page 66) ─────────────────────────────────

static DC_MA: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 66,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "20mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "200mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.2%+20",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 1000V, fast type fuse, ø6.3x32mm".
        overload_protection: Some("Fuse 0.5A 1000V"),
        notes: &["10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min"],
    },
};

// ── C. DC Current, A (manual PDF page 66) ──────────────────────────────────

static DC_A: ModeSpecs = ModeSpecs {
    name: "C. DC Current",
    page: 66,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[AccuracyBand {
                freq_range: None,
                accuracy: "0.8%+30",
            }],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A, 1000V, fast type fuse, ø10.3x38mm".
        overload_protection: Some("Fuse 10A 1000V"),
        notes: &["10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min"],
    },
};

// ── D. AC Current, µA (manual PDF page 67) ─────────────────────────────────

// AC or AC+DC coupling, split as DC current is.
static AC_UA: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 67,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "200µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "1%+15",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "2000µA",
            spec: SpecInfo {
                resolution: "0.1µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "1%+15",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 1000V, fast type fuse, ø6.3x32mm".
        overload_protection: Some("Fuse 0.5A 1000V"),
        notes: &[
            "True RMS valid 10%–100% of range; crest factor ≤3.0",
            "Shorted leads: residual 80 digits, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min",
        ],
    },
};

// ── D. AC Current, mA (manual PDF page 67) ─────────────────────────────────

static AC_MA: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 67,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "20mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "1%+15",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "200mA",
            spec: SpecInfo {
                resolution: "0.01mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("45Hz~1kHz"),
                        accuracy: "1%+15",
                    },
                    AccuracyBand {
                        freq_range: Some("1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A, 1000V, fast type fuse, ø6.3x32mm".
        overload_protection: Some("Fuse 0.5A 1000V"),
        notes: &[
            "True RMS valid 10%–100% of range; crest factor ≤3.0",
            "Shorted leads: residual 80 digits, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min",
        ],
    },
};

// ── D. AC Current, A (manual PDF page 67) ──────────────────────────────────

static AC_A: ModeSpecs = ModeSpecs {
    name: "D. AC Current (AC+DC measurement is available)",
    page: 67,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("45Hz~1kHz"),
                    accuracy: "2%+20",
                },
                AccuracyBand {
                    freq_range: Some("1kHz~10kHz"),
                    accuracy: "6%+40",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A,1000V, fast type fuse,ø10.3x38mm".
        overload_protection: Some("Fuse 10A 1000V"),
        notes: &[
            "True RMS valid 10%–100% of range; crest factor ≤3.0",
            "Shorted leads: residual 80 digits, accuracy unaffected",
            "AC+DC: add 1%+35 digits to accuracy",
            "10A: ≤5A continuous; 5A–10A max. 10s, then rest >15min",
        ],
    },
};

// ── E. Resistance (manual PDF page 68) ─────────────────────────────────────

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "E. Resistance",
    page: 68,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "200Ω",
            spec: SpecInfo {
                resolution: "0.01Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "2kΩ",
            spec: SpecInfo {
                resolution: "0.0001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "20kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "200kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "2MΩ",
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
            label: "20MΩ",
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
        notes: &["200Ω: accuracy + test-lead short-circuit resistance"],
    },
};

// ── F. Continuity Test (manual PDF page 68) ────────────────────────────────

// No accuracy column; the range cell is the continuity symbol.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "F. Continuity Test",
    page: 68,
    ranges: &[RangeSpec {
        range: None,
        label: "•)))",
        spec: SpecInfo {
            resolution: "0.01Ω",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Open circuit ≈-1.2V",
            "Silent >60Ω",
            "Beeps continuously ≤40Ω",
        ],
    },
};

// ── G. Diode Test (manual PDF page 69) ─────────────────────────────────────

// No accuracy column; the range cell is the diode symbol.
static DIODE: ModeSpecs = ModeSpecs {
    name: "G. Diode Test",
    page: 69,
    ranges: &[RangeSpec {
        range: None,
        label: "▶|",
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

// ── H. Capacitance (manual PDF page 69) ────────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "H. Capacitance",
    page: 69,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "20nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "200nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "2µF",
            spec: SpecInfo {
                resolution: "0.0001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "20µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "200µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "1.5%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "2mF",
            spec: SpecInfo {
                resolution: "0.0001mF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5%+40",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "20mF",
            spec: SpecInfo {
                // Printed "0.001 mF".
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
        notes: &["20nF: accuracy + open-circuit test-lead capacitance"],
    },
};

// ── I. Frequency (manual PDF page 70) ──────────────────────────────────────

static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "I. Frequency",
    page: 70,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "20Hz",
            spec: SpecInfo {
                resolution: "0.001Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(1),
            label: "200Hz",
            spec: SpecInfo {
                resolution: "0.01Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "2kHz",
            spec: SpecInfo {
                resolution: "0.0001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "20kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "200kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "2MHz",
            spec: SpecInfo {
                resolution: "0.0001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "20MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "200MHz",
            spec: SpecInfo {
                resolution: "0.01MHz",
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
        notes: &[
            "Input 10Hz–40MHz: 200mV–30V rms",
            "Input at zero DC level; >40MHz not specified",
        ],
    },
};

#[cfg(test)]
mod tests {
    /// One figure per table, from the verified transcription.
    #[test]
    fn each_table_keeps_its_first_figure() {
        crate::specs::assert_first_figures(
            super::TABLES.all,
            &[
                ("A. DC Voltage", "200mV", Some("0.1%+8")),
                ("A. DC Voltage", "2V", Some("0.1%+8")),
                (
                    "B. AC Voltage (AC+DC measurement is available)",
                    "2V",
                    Some("0.8%+40"),
                ),
                ("C. DC Current", "200µA", Some("0.2%+20")),
                ("C. DC Current", "20mA", Some("0.2%+20")),
                ("C. DC Current", "10A", Some("0.8%+30")),
                (
                    "D. AC Current (AC+DC measurement is available)",
                    "200µA",
                    Some("1%+15"),
                ),
                (
                    "D. AC Current (AC+DC measurement is available)",
                    "20mA",
                    Some("1%+15"),
                ),
                (
                    "D. AC Current (AC+DC measurement is available)",
                    "10A",
                    Some("2%+20"),
                ),
                ("E. Resistance", "200Ω", Some("0.5%+20")),
                ("F. Continuity Test", "•)))", None),
                ("G. Diode Test", "▶|", None),
                ("H. Capacitance", "20nF", Some("1.5%+20")),
                ("I. Frequency", "20Hz", Some("0.1%+15")),
            ],
        );
    }
}
