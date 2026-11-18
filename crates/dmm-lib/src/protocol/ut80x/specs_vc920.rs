//! Specification data for the Voltcraft VC920, VC940 and VC960 (40000
//! counts).
//!
//! Transcribed from the English part of the VC920/VC940/VC960 operating
//! instructions (Version 11/10, with the important note of 01/13; Conrad
//! 123296), "Technical data" (PDF pages 60-62), with values as printed but
//! for the typesetting slips noted where they are. The manual sets a space
//! between number and unit ("4 V", "0.01 mV", "10 MΩ"); the tables close it
//! up, as the range labels the parser gives are. Where the 2006 English
//! VC940 manual prints other figures, this manual's are kept. The power
//! table is the VC940's W position. The notes are short app notes condensed
//! from the manual's remarks.
//!
//! Each row prints two accuracies: the "40.000" column, for the normal
//! display, and the "4.000" column, for the 4000-count display the blue key
//! held at power-on gives (ut71 spec §3.1). The rows keep both, each band
//! labelled by its column as the transcription is, so the review sheet and
//! the transcription compare see the table as printed; a reading takes only
//! its own display's bands, relabelled, through [`count_mode_spec`].
//! Resistance prints one column, 4000 counts, at which it is fixed.
//!
//! A table is split into parts of the same name where its rows belong to
//! different modes or need a different overload protection. Range bytes
//! follow `ut804_mode_info`.

use super::{CountMode, Coupling};
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};
use std::sync::LazyLock;

/// The table for a reading with mode code `mode`, its coupling and whether
/// it is a duty cycle (`Ut804Fields`). °F takes the °C table, for its mode
/// data ([`has_rows`]).
///
/// `None` where the manual gives the reading nothing: duty cycle, whose row
/// prints only the signal's frequency range, and AC or AC+DC millivolts,
/// which no Voltcraft manual has (ut71 spec §3.2).
pub(super) fn table(
    mode: u8,
    coupling: Option<Coupling>,
    duty: bool,
) -> Option<&'static ModeSpecs> {
    use Coupling::{Ac, AcDc, Dc};
    Some(match (mode, coupling) {
        (0xC, _) if duty => return None,
        (0x1 | 0x2, Some(Dc)) => &DC_V,
        (0x1 | 0x2, Some(Ac | AcDc)) => &AC_V,
        (0x3, Some(Dc)) => &DC_MV,
        (0x7, Some(Dc)) => &DC_UA,
        (0x8, Some(Dc)) => &DC_MA,
        (0x9, Some(Dc)) => &DC_A,
        (0x7, Some(Ac | AcDc)) => &AC_UA,
        (0x8, Some(Ac | AcDc)) => &AC_MA,
        (0x9, Some(Ac | AcDc)) => &AC_A,
        (0xE, _) => &POWER,
        (0x4 | 0xA, _) => &RESISTANCE,
        (0xB, _) => &DIODE,
        (0x5, _) => &CAPACITANCE,
        (0xC, _) => &FREQUENCY,
        (0x6 | 0xD, _) => &TEMP_C,
        (0xF, _) => &LOOP_CURRENT,
        _ => return None,
    })
}

/// Whether a reading takes a row of its table. Continuity takes the Ω
/// table's mode data, whose note gives its threshold, but no row: the manual
/// has none for it. °F takes the temperature table's mode data only: its
/// row is °C. An AC+DC reading takes the AC table's mode data only: its
/// remarks add (1%+30 digits) to the table's accuracy, a sum the manual does
/// not print.
pub(super) fn has_rows(mode: u8, coupling: Option<Coupling>) -> bool {
    !matches!(mode, 0xA | 0xD) && coupling != Some(Coupling::AcDc)
}

/// What the manual prints for a band it gives no figure.
const NOT_SPECIFIED: &str = "Not specified";

/// What a 4000-count band's label ends with.
const COUNT4000: &str = "(4000 counts)";

/// The display a band is printed for, by the column heading its label
/// starts with, and the condition after the heading ("" for none).
fn band_column(band: &AccuracyBand) -> Option<(CountMode, &'static str)> {
    let label = band.freq_range?;
    let (counts, rest) = if let Some(rest) = label.strip_prefix("40.000") {
        (CountMode::Full, rest)
    } else {
        (CountMode::Count4000, label.strip_prefix("4.000")?)
    };
    Some((counts, rest.strip_prefix(": ").unwrap_or(rest)))
}

/// Whether `row` prints a figure in display `counts`' column.
fn prints(row: &RangeSpec, counts: CountMode) -> bool {
    (row.spec.accuracy.iter()).any(|band| band_column(band).is_some_and(|(c, _)| c == counts))
}

/// Every row of every table, in [`ALL`] order.
fn rows() -> impl Iterator<Item = &'static RangeSpec> {
    ALL.iter().flat_map(|table| table.ranges)
}

/// A row's bands as a reading on one display lists them, each a label and
/// a figure.
type View = Vec<(Option<String>, &'static str)>;

/// `row`'s bands in display `counts`' column, as a reading on that display
/// lists them: the column heading dropped from the label, and a 4000-count
/// one marked as such. Empty where the column prints nothing, or nothing but
/// "Not specified".
fn view(row: &RangeSpec, counts: CountMode) -> View {
    // Each band's condition, its frequency part and its figure.
    let mut bands: Vec<(&str, Option<&str>, &'static str)> = Vec::new();
    for band in row.spec.accuracy {
        let Some((_, condition)) = band_column(band).filter(|&(c, _)| c == counts) else {
            continue;
        };
        let frequency = condition.split_once(", ").map(|(frequency, _)| frequency);
        // The 4000-count column prints one cell over sub-rows the full
        // display splits by share of the range ("10 - 15% of MR"), and the
        // transcription repeats it on each: one band of the frequency alone.
        match bands.last_mut() {
            Some(last)
                if counts == CountMode::Count4000
                    && frequency.is_some()
                    && (last.1, last.2) == (frequency, band.accuracy) =>
            {
                last.0 = frequency.unwrap_or_default();
            }
            _ => bands.push((condition, frequency, band.accuracy)),
        }
    }
    if bands.iter().all(|&(.., figure)| figure == NOT_SPECIFIED) {
        return Vec::new();
    }
    // The compact summary shows the first band, which has to be a figure.
    bands.sort_by_key(|&(.., figure)| figure == NOT_SPECIFIED);
    let label = |condition: &str| match (counts, condition) {
        (CountMode::Full, "") => None,
        (CountMode::Full, _) => Some(condition.to_string()),
        (CountMode::Count4000, "") => Some(COUNT4000.to_string()),
        (CountMode::Count4000, _) => Some(format!("{condition} {COUNT4000}")),
    };
    (bands.into_iter())
        .map(|(condition, _, figure)| (label(condition), figure))
        .collect()
}

/// Where a display's view sits in each row's pair.
fn index(counts: CountMode) -> usize {
    match counts {
        CountMode::Full => 0,
        CountMode::Count4000 => 1,
    }
}

/// Each row's views, full display then 4000-count, in [`rows`] order: the
/// labels the bands below borrow.
static VIEWS: LazyLock<Vec<[View; 2]>> = LazyLock::new(|| {
    rows()
        .map(|row| [CountMode::Full, CountMode::Count4000].map(|counts| view(row, counts)))
        .collect()
});

/// [`VIEWS`] as bands.
static VIEW_BANDS: LazyLock<Vec<[Vec<AccuracyBand>; 2]>> = LazyLock::new(|| {
    (VIEWS.iter())
        .map(|views| {
            views.each_ref().map(|bands| {
                (bands.iter())
                    .map(|(label, figure)| AccuracyBand {
                        freq_range: label.as_deref(),
                        accuracy: figure,
                    })
                    .collect()
            })
        })
        .collect()
});

/// Each row as each display shows it, `None` for an empty view. The manual
/// prints the resolution at 40000 counts, and a tenth of it is not printed,
/// so a 4000-count view has none but on a row printed for that display
/// alone (Ω).
static VIEW_SPECS: LazyLock<Vec<[Option<SpecInfo>; 2]>> = LazyLock::new(|| {
    rows()
        .zip(VIEW_BANDS.iter())
        .map(|(row, [full, count4000])| {
            let spec = |bands: &'static Vec<AccuracyBand>, resolution| {
                (!bands.is_empty()).then_some(SpecInfo {
                    resolution,
                    accuracy: bands,
                })
            };
            let count4000_resolution = if prints(row, CountMode::Full) {
                ""
            } else {
                row.spec.resolution
            };
            [
                spec(full, row.spec.resolution),
                spec(count4000, count4000_resolution),
            ]
        })
        .collect()
});

/// The spec a reading on `row` takes on display `counts`: its own display's
/// view of the row, the 4000-count one for a row printed for that display
/// alone (Ω, fixed at 4000 counts), and the row as printed on the full
/// display for a row with no figure at all (diode). `None` where the view
/// is empty: the reading takes its table's mode data only.
pub(super) fn count_mode_spec(
    row: &'static RangeSpec,
    counts: CountMode,
) -> Option<&'static SpecInfo> {
    let i = rows().position(|r| std::ptr::eq(r, row))?;
    if row.spec.accuracy.is_empty() {
        return (counts == CountMode::Full).then_some(&row.spec);
    }
    let counts = if prints(row, CountMode::Full) {
        counts
    } else {
        CountMode::Count4000
    };
    VIEW_SPECS[i][index(counts)].as_ref()
}

/// Every label a reading's view gives a band.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn view_labels() -> impl Iterator<Item = &'static str> {
    (VIEWS.iter().flatten().flatten()).filter_map(|(label, _)| label.as_deref())
}

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
    &POWER,
    &RESISTANCE,
    &DIODE,
    &CAPACITANCE,
    &FREQUENCY,
    &TEMP_C,
    &LOOP_CURRENT,
    &DUTY,
];

// ── DC / V, mV (manual PDF page 60) ────────────────────────────────────────

// The DC mV mode (0x3), whatever its range byte, with an input
// impedance of its own.
static DC_MV: ModeSpecs = ModeSpecs {
    name: "DC / V",
    page: 60,
    ranges: &[RangeSpec {
        range: None,
        label: "400mV",
        spec: SpecInfo {
            resolution: "0.01mV",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000"),
                    accuracy: "0.025%+10",
                },
                AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "0.1%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: Some("2.5GΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── DC / V, V (manual PDF page 60) ─────────────────────────────────────────

// Modes 0x1 and 0x2 with DC coupling.
static DC_V: ModeSpecs = ModeSpecs {
    name: "DC / V",
    page: 60,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "4V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.08%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.2%+5",
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
                        freq_range: Some("40.000"),
                        accuracy: "0.08%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.2%+5",
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
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.3%+5",
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
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.3%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── AC / V (manual PDF page 60) ────────────────────────────────────────────

// Modes 0x1 and 0x2 with AC or AC+DC coupling; a remark gives the AC+DC
// adder.
static AC_V: ModeSpecs = ModeSpecs {
    name: "AC / V",
    page: 60,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "4V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "0.5%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 45Hz~1kHz"),
                        accuracy: "0.5%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 1kHz~10kHz"),
                        accuracy: "1.5%+10",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 10kHz~120kHz, 10 - 15% of MR"),
                        accuracy: "6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 10kHz~120kHz, 10 - 15% of MR"),
                        accuracy: "4%+8",
                    },
                    // Printed "15 - 100% v. MR".
                    AccuracyBand {
                        freq_range: Some("40.000: 10kHz~120kHz, 15 - 100% of MR"),
                        accuracy: "3%+40",
                    },
                    // Printed "15 - 100% v. MR".
                    AccuracyBand {
                        freq_range: Some("4.000: 10kHz~120kHz, 15 - 100% of MR"),
                        accuracy: "4%+8",
                    },
                    // Printed "10 - 30% v. MR".
                    AccuracyBand {
                        freq_range: Some("40.000: 120kHz~250kHz, 10 - 30% of MR"),
                        accuracy: "10%+40",
                    },
                    // Printed "10 - 30% v. MR".
                    AccuracyBand {
                        freq_range: Some("4.000: 120kHz~250kHz, 10 - 30% of MR"),
                        accuracy: "5%+10",
                    },
                    // Printed "30 - 100% v. MR".
                    AccuracyBand {
                        freq_range: Some("40.000: 120kHz~250kHz, 30 - 100% of MR"),
                        accuracy: "7%+50",
                    },
                    // Printed "30 - 100% v. MR".
                    AccuracyBand {
                        freq_range: Some("4.000: 120kHz~250kHz, 30 - 100% of MR"),
                        accuracy: "5%+10",
                    },
                    // Printed "10 - 30% v. MR".
                    AccuracyBand {
                        freq_range: Some("40.000: 250kHz~400kHz, 10 - 30% of MR"),
                        accuracy: "15%+40",
                    },
                    // Printed "10 - 30% v. MR".
                    AccuracyBand {
                        freq_range: Some("4.000: 250kHz~400kHz, 10 - 30% of MR"),
                        accuracy: "12%+20",
                    },
                    // Printed "±(12+50dgt) (30 - 100% v. MR)".
                    AccuracyBand {
                        freq_range: Some("40.000: 30 - 100% of MR"),
                        accuracy: "12%+50",
                    },
                    // Printed "30 - 100% v. MR".
                    AccuracyBand {
                        freq_range: Some("4.000: 30 - 100% of MR"),
                        accuracy: "12%+20",
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
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "0.5%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 10kHz~20kHz"),
                        accuracy: "7%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 20kHz~100kHz"),
                        accuracy: "8%+40",
                    },
                    // Printed "Nicht spezifiziert".
                    AccuracyBand {
                        freq_range: Some("40.000: 100kHz~400kHz"),
                        accuracy: "Not specified",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 100kHz~400kHz"),
                        accuracy: "Not specified",
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
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "0.5%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "6%+40",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "750V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000: 10kHz~250kHz"),
                        accuracy: "Not specified",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 10kHz~250kHz"),
                        accuracy: "Not specified",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~5kHz"),
                        accuracy: "6%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 5kHz~10kHz"),
                        accuracy: "12%+40",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: Some("10MΩ"),
        overload_protection: Some("1000V"),
        notes: &[
            "True RMS accuracy valid 10%–100% of range",
            "AC+DC: add 1%+30 digits to accuracy",
            "Crest factor ≤3",
            "Shorted leads: display ≤70 digits",
        ],
    },
};

// ── DC / A, µA (manual PDF page 61) ────────────────────────────────────────

// µA, mA and A are modes of their own, and the 10A range has a fuse of
// its own.
static DC_UA: ModeSpecs = ModeSpecs {
    name: "DC / A",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "400µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.25%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.25%+10",
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
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A 250V fast-acting fuse".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &["10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause"],
    },
};

// ── DC / A, mA (manual PDF page 61) ────────────────────────────────────────

static DC_MA: ModeSpecs = ModeSpecs {
    name: "DC / A",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "40mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
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
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A 250V fast-acting fuse".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &["10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause"],
    },
};

// ── DC / A, A (manual PDF page 61) ─────────────────────────────────────────

static DC_A: ModeSpecs = ModeSpecs {
    name: "DC / A",
    page: 61,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000"),
                    accuracy: "2%+30",
                },
                AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "2%+20",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A 250V fast-acting fuse".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &["10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause"],
    },
};

// ── AC / A, µA (manual PDF page 61) ────────────────────────────────────────

// AC or AC+DC coupling, split as DC current is.
static AC_UA: ModeSpecs = ModeSpecs {
    name: "AC / A",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "400µA",
            spec: SpecInfo {
                resolution: "0.01µA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 45Hz~1kHz"),
                        accuracy: "1%+5",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 1kHz~10kHz"),
                        accuracy: "2%+5",
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
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 45Hz~1kHz"),
                        accuracy: "1%+5",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 1kHz~10kHz"),
                        accuracy: "2%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A 250V fast-acting fuse".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &[
            "10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause",
            "True RMS accuracy valid 10%–100% of range",
            "AC+DC: add 1%+30 digits to accuracy",
            "Crest factor ≤3",
        ],
    },
};

// ── AC / A, mA (manual PDF page 61) ────────────────────────────────────────

static AC_MA: ModeSpecs = ModeSpecs {
    name: "AC / A",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(0),
            label: "40mA",
            spec: SpecInfo {
                resolution: "0.001mA",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 45Hz~1kHz"),
                        accuracy: "1%+5",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 1kHz~10kHz"),
                        accuracy: "2%+5",
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
                        freq_range: Some("40.000: 45Hz~1kHz"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 45Hz~1kHz"),
                        accuracy: "1%+5",
                    },
                    AccuracyBand {
                        freq_range: Some("40.000: 1kHz~10kHz"),
                        accuracy: "2%+40",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000: 1kHz~10kHz"),
                        accuracy: "2%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "0.5A 250V fast-acting fuse".
        overload_protection: Some("Fuse 0.5A 250V"),
        notes: &[
            "10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause",
            "True RMS accuracy valid 10%–100% of range",
            "AC+DC: add 1%+30 digits to accuracy",
            "Crest factor ≤3",
        ],
    },
};

// ── AC / A, A (manual PDF page 61) ─────────────────────────────────────────

static AC_A: ModeSpecs = ModeSpecs {
    name: "AC / A",
    page: 61,
    ranges: &[RangeSpec {
        range: None,
        label: "10A",
        spec: SpecInfo {
            resolution: "0.001A",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000: 45Hz~1kHz"),
                    accuracy: "2%+20",
                },
                AccuracyBand {
                    freq_range: Some("4.000: 45Hz~1kHz"),
                    accuracy: "1%+5",
                },
                AccuracyBand {
                    freq_range: Some("40.000: 1kHz~10kHz"),
                    accuracy: "5%+40",
                },
                AccuracyBand {
                    freq_range: Some("4.000: 1kHz~10kHz"),
                    accuracy: "2%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        // Printed "10A 250 V fast-acting fuse".
        overload_protection: Some("Fuse 10A 250V"),
        notes: &[
            "10A: 0–5A continuous; >5A–10A max. 10s, then 15min pause",
            "True RMS accuracy valid 10%–100% of range",
            "AC+DC: add 1%+30 digits to accuracy",
            "Crest factor ≤3",
        ],
    },
};

// ── Power (manual PDF page 61) ─────────────────────────────────────────────

// The VC940's W position (mode 0xE). The table prints no overload
// protection.
static POWER: ModeSpecs = ModeSpecs {
    // Printed "Output", a mistranslation of the German "Leistung".
    name: "Power",
    page: 61,
    ranges: &[RangeSpec {
        range: None,
        label: "2500W",
        spec: SpecInfo {
            resolution: "0.1W",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000"),
                    accuracy: "2%+20",
                },
                AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "2%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Voltage 190V–250V AC",
            "0–1150W continuous; >1150W–2500W max. 10s, then 15min pause",
        ],
    },
};

// ── Ω (manual PDF page 61) ─────────────────────────────────────────────────

// One count column: Ω is fixed at 4000 counts, so the resolution is a
// 4000-count one. Continuity (mode 0xA) takes this table's mode data,
// whose note gives its threshold, but no row.
static RESISTANCE: ModeSpecs = ModeSpecs {
    name: "Ω",
    page: 61,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "400Ω",
            spec: SpecInfo {
                resolution: "0.1Ω",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "0.8%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "4kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "40kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "400kΩ",
            spec: SpecInfo {
                resolution: "0.1kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "0.5%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "4MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "1%+15",
                }],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "40MΩ",
            spec: SpecInfo {
                resolution: "0.01MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "2%+20",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Fixed at 4000 counts", "Continuity: beeps <50Ω"],
    },
};

// ── Diode test (manual PDF page 62) ────────────────────────────────────────

// One full-width cell: no range, no accuracy and no overload protection
// printed.
static DIODE: ModeSpecs = ModeSpecs {
    name: "Diode test",
    page: 62,
    ranges: &[RangeSpec {
        range: None,
        label: "Diode test",
        spec: SpecInfo {
            resolution: "0.0001V",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Test voltage max. 2.8V; test current ≈1mA"],
    },
};

// ── Capacity (manual PDF page 62) ──────────────────────────────────────────

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: "Capacity",
    page: 62,
    ranges: &[
        RangeSpec {
            range: Some(1),
            label: "40nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(2),
            label: "400nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "4µF",
            spec: SpecInfo {
                resolution: "0.0001µF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "40µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "400µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "1%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "1%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "4mF",
            spec: SpecInfo {
                resolution: "0.0001mF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "5%+20",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "5%+10",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "40mF",
            spec: SpecInfo {
                resolution: "0.001mF",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "Not specified",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "Not specified",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[],
    },
};

// ── Frequency (manual PDF page 62) ─────────────────────────────────────────

static FREQUENCY: ModeSpecs = ModeSpecs {
    name: "Frequency",
    page: 62,
    ranges: &[
        RangeSpec {
            range: Some(2),
            label: "4kHz",
            spec: SpecInfo {
                resolution: "0.0001kHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(3),
            label: "40kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(4),
            label: "400kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(5),
            label: "4MHz",
            spec: SpecInfo {
                resolution: "0.0001MHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(6),
            label: "40MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
        RangeSpec {
            range: Some(7),
            label: "400MHz",
            spec: SpecInfo {
                resolution: "0.01MHz",
                accuracy: &[
                    AccuracyBand {
                        freq_range: Some("40.000"),
                        accuracy: "0.1%+8",
                    },
                    AccuracyBand {
                        freq_range: Some("4.000"),
                        accuracy: "0.1%+5",
                    },
                ],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Sensitivity 10Hz–40MHz: 200mV; >40MHz not specified"],
    },
};

// ── Temperature (manual PDF page 62) ───────────────────────────────────────

// °C only: the manual prints no °F table.
static TEMP_C: ModeSpecs = ModeSpecs {
    name: "Temperature",
    page: 62,
    ranges: &[RangeSpec {
        range: None,
        label: "Temperature",
        spec: SpecInfo {
            resolution: "0.1°C",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000: -40 to +40 °C"),
                    accuracy: "3%+30",
                },
                AccuracyBand {
                    freq_range: Some("4.000: -40 to +40 °C"),
                    accuracy: "3%+10",
                },
                AccuracyBand {
                    freq_range: Some("40.000: +40 to +400 °C"),
                    accuracy: "1%+30",
                },
                AccuracyBand {
                    freq_range: Some("4.000: +40 to +400 °C"),
                    accuracy: "1%+1",
                },
                AccuracyBand {
                    freq_range: Some("40.000: +400 to +1000 °C"),
                    accuracy: "2.5%",
                },
                AccuracyBand {
                    freq_range: Some("4.000: +400 to +1000 °C"),
                    accuracy: "2.5%",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Supplied wire thermocouple: max. +230°C"],
    },
};

// ── 4-20 mA (manual PDF page 62) ───────────────────────────────────────────

static LOOP_CURRENT: ModeSpecs = ModeSpecs {
    name: "4-20 mA",
    page: 62,
    ranges: &[RangeSpec {
        range: None,
        label: "4-20mA",
        spec: SpecInfo {
            resolution: "0.01%",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("40.000"),
                    accuracy: "1%+50",
                },
                AccuracyBand {
                    freq_range: Some("4.000"),
                    accuracy: "1%+5",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[],
    },
};

// ── Duty Cycle (manual PDF page 62) ────────────────────────────────────────

// The row prints only the signal's frequency range: no resolution and no
// accuracy. No reading takes it (duty readings have no spec).
static DUTY: ModeSpecs = ModeSpecs {
    name: "Duty Cycle",
    page: 62,
    ranges: &[RangeSpec {
        range: None,
        // Printed "10Hz - 2 KHz".
        label: "10Hz - 2kHz",
        spec: SpecInfo {
            resolution: "",
            accuracy: &[],
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
    /// One figure per table, from the verified transcription: the first
    /// row's first band as printed.
    #[test]
    fn each_table_keeps_its_first_figure() {
        crate::specs::assert_first_figures(
            super::ALL,
            &[
                ("DC / V", "400mV", Some("0.025%+10")),
                ("DC / V", "4V", Some("0.08%+10")),
                ("AC / V", "4V", Some("0.5%+40")),
                ("DC / A", "400µA", Some("0.25%+20")),
                ("DC / A", "40mA", Some("1%+20")),
                ("DC / A", "10A", Some("2%+30")),
                ("AC / A", "400µA", Some("1%+20")),
                ("AC / A", "40mA", Some("1%+20")),
                ("AC / A", "10A", Some("2%+20")),
                ("Power", "2500W", Some("2%+20")),
                ("Ω", "400Ω", Some("0.8%+20")),
                ("Diode test", "Diode test", None),
                ("Capacity", "40nF", Some("1%+20")),
                ("Frequency", "4kHz", Some("0.1%+8")),
                ("Temperature", "Temperature", Some("3%+30")),
                ("4-20 mA", "4-20mA", Some("1%+50")),
                ("Duty Cycle", "10Hz - 2kHz", None),
            ],
        );
    }
}
