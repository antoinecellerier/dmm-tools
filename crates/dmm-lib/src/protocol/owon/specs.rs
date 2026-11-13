//! Specification data for the CMS061 clamp meter (UNI-T UT217A).
//!
//! Transcribed from the CMS061 user manual (V1.0.4, Sep. 2025), the
//! multimeter specification table on PDF pages 35-36 (printed 30-31), with
//! values as printed but for the typesetting slip noted where it is. The
//! table gives no input impedance or overload protection. Cross-checked
//! against UNI-T's UT217 manual: where it prints another range or
//! resolution, a comment says so and the row keeps OWON's; one accuracy cell
//! the OWON manual leaves blank takes the UT217 manual's figure. The notes
//! are short app notes condensed from the table's remarks. The Diode and
//! On-Off rows give no accuracy: the manual's cell is a remark, carried as
//! a note.
//!
//! A row's `range` is the reading's range byte, `range_raw`: the function
//! word's prefix code shifted left three and its decimal-point code (spec
//! §10.3, §6.3, §6.4). A printed range gives both as its unit's prefix and
//! the digits after its point: 200.00mV is prefix m, code 3, and 2 digits,
//! 0x1A. So OL keeps its range's row where the reading word says OL (spec
//! §10.4), and has none where the decimal-point code does.

use super::decode::function;
use super::model::{self, Model};
use crate::measurement::Measurement;
use crate::specs::{AccuracyBand, ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo};

/// The manual's table titles, which a model's tables are found by.
mod name {
    pub(super) const DC_VOLTAGE: &str = "DC Voltage (V)";
    pub(super) const AC_VOLTAGE: &str = "AC Voltage (V)";
    pub(super) const DC_CURRENT: &str = "DC Current (A)";
    pub(super) const AC_CURRENT: &str = "AC Current (A)";
    pub(super) const INRUSH_CURRENT: &str = "Inrush Current(A)";
    pub(super) const RESISTANCE: &str = "Resistance(Ω)";
    pub(super) const CAPACITANCE: &str = "Capacitance(F)";
    pub(super) const FREQUENCY: &str = "Frequency (Hz)";
    pub(super) const DUTY_CYCLE: &str = "Duty Cycle (%)";
    pub(super) const DIODE: &str = "Diode";
    pub(super) const ON_OFF: &str = "On-Off";
}

/// The CMS061's tables, in manual order.
static CMS061_TABLES: &[&ModeSpecs] = &[
    &DC_VOLTAGE,
    &AC_VOLTAGE,
    &DC_CURRENT_600A,
    &AC_CURRENT_600A,
    &INRUSH_CURRENT_600A,
    &RESISTANCE,
    &CAPACITANCE,
    &FREQUENCY,
    &DUTY_CYCLE,
    &DIODE,
    &ON_OFF,
];

/// `model`'s tables, in manual order; none for a model whose manual's
/// tables are not here.
pub(crate) fn for_model(model: &Model) -> &'static [&'static ModeSpecs] {
    if model.code == model::CMS061.code {
        CMS061_TABLES
    } else {
        &[]
    }
}

/// The title of the table reading `m` takes, by its function code and the
/// mode the status word makes of it (spec §6.2, §6.6, §10.6). `None` for the
/// readings the manual gives no table: the scope, NCV, the functions the
/// meter has no position for, LPF and Peak, and inrush on another function
/// than AC A. REL, MAX, MIN and HOLD keep the function's table.
fn table_name(m: &Measurement) -> Option<&'static str> {
    let code = u8::try_from(m.mode_raw).ok()?;
    Some(match (code, m.mode.as_ref()) {
        (function::DC_V, "DC V") => name::DC_VOLTAGE,
        (function::AC_V, "AC V") => name::AC_VOLTAGE,
        (function::DC_A, "DC A") => name::DC_CURRENT,
        (function::AC_A, "AC A") => name::AC_CURRENT,
        // The inrush key on the A position: AC A with the status word's
        // inrush bit.
        (function::AC_A, "Inrush") => name::INRUSH_CURRENT,
        (function::OHMS, "Ω") => name::RESISTANCE,
        (function::CAPACITANCE, "Capacitance") => name::CAPACITANCE,
        // Hz at the Hz position, and in the large digits beside the AC
        // reading at the V and A positions: one table for both.
        (function::HZ, "Hz") => name::FREQUENCY,
        (function::DUTY, "Duty %") => name::DUTY_CYCLE,
        (function::DIODE, "Diode") => name::DIODE,
        (function::CONTINUITY, "Continuity") => name::ON_OFF,
        _ => return None,
    })
}

/// The table in `tables`, a model's list, that reading `m` takes.
pub(super) fn table(
    tables: &'static [&'static ModeSpecs],
    m: &Measurement,
) -> Option<&'static ModeSpecs> {
    let name = table_name(m)?;
    tables.iter().copied().find(|t| t.name == name)
}

/// The row of reading `m` in its table: the one for its range byte. `None`
/// for a prefix and point no printed range has, such as 0.000 Hz.
pub(super) fn row(
    tables: &'static [&'static ModeSpecs],
    m: &Measurement,
) -> Option<&'static RangeSpec> {
    table(tables, m)?.row(m.range_raw)
}

static DC_VOLTAGE: ModeSpecs = ModeSpecs {
    name: name::DC_VOLTAGE,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x1B),
            label: "20.000mV",
            spec: SpecInfo {
                resolution: "0.001mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+20",
                }],
            },
        },
        RangeSpec {
            range: Some(0x1A),
            label: "200.00mV",
            spec: SpecInfo {
                resolution: "0.01mV",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+6",
                }],
            },
        },
        RangeSpec {
            range: Some(0x24),
            label: "2.0000V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x23),
            label: "20.000V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x22),
            label: "200.00V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x21),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.15%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Accuracy valid 5%–100% of range"],
    },
};

static AC_VOLTAGE: ModeSpecs = ModeSpecs {
    name: name::AC_VOLTAGE,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x1A),
            label: "200.00mV",
            spec: SpecInfo {
                // Printed "0.01 mV".
                resolution: "0.01mV",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "0.6%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x24),
            label: "2.0000V",
            spec: SpecInfo {
                resolution: "0.0001V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "0.6%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x23),
            label: "20.000V",
            spec: SpecInfo {
                resolution: "0.001V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "0.6%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x22),
            label: "200.00V",
            spec: SpecInfo {
                resolution: "0.01V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "0.6%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x21),
            label: "1000.0V",
            spec: SpecInfo {
                resolution: "0.1V",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "0.8%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Accuracy valid 10%–100% of range"],
    },
};

static DC_CURRENT_600A: ModeSpecs = ModeSpecs {
    name: name::DC_CURRENT,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x22),
            label: "20.00A",
            spec: SpecInfo {
                resolution: "0.01A",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x21),
            label: "200.0A",
            spec: SpecInfo {
                resolution: "0.1A",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
        // The UT217 manual prints 600.0A at 0.1A.
        RangeSpec {
            range: Some(0x20),
            label: "600A",
            spec: SpecInfo {
                resolution: "1A",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "2.0%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Accuracy valid 5%–100% of range"],
    },
};

static AC_CURRENT_600A: ModeSpecs = ModeSpecs {
    name: name::AC_CURRENT,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x22),
            label: "20.00A",
            spec: SpecInfo {
                resolution: "0.01A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x21),
            label: "200.0A",
            spec: SpecInfo {
                resolution: "0.1A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "2.5%+5",
                }],
            },
        },
        // The UT217 manual prints 600.0A at 0.1A.
        RangeSpec {
            range: Some(0x20),
            label: "600A",
            spec: SpecInfo {
                resolution: "1A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "2.5%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Accuracy valid 10%–100% of range"],
    },
};

static INRUSH_CURRENT_600A: ModeSpecs = ModeSpecs {
    name: name::INRUSH_CURRENT,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x22),
            label: "20.00A",
            spec: SpecInfo {
                resolution: "0.01A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "10.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x21),
            label: "200.0A",
            spec: SpecInfo {
                resolution: "0.1A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "10.0%+10",
                }],
            },
        },
        // The UT217 manual prints 600.0A at 0.1A.
        RangeSpec {
            range: Some(0x20),
            label: "600A",
            spec: SpecInfo {
                resolution: "1A",
                accuracy: &[AccuracyBand {
                    freq_range: Some("40Hz–1000Hz"),
                    accuracy: "10.0%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[],
    },
};

static RESISTANCE: ModeSpecs = ModeSpecs {
    name: name::RESISTANCE,
    page: 35,
    ranges: &[
        RangeSpec {
            range: Some(0x22),
            label: "200.00Ω",
            spec: SpecInfo {
                resolution: "0.01Ω",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.8%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2C),
            label: "2.0000kΩ",
            spec: SpecInfo {
                resolution: "0.0001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2B),
            label: "20.000kΩ",
            spec: SpecInfo {
                resolution: "0.001kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2A),
            label: "200.00kΩ",
            spec: SpecInfo {
                resolution: "0.01kΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+10",
                }],
            },
        },
        // The OWON manual prints this accuracy cell blank; the figure is the
        // UT217 manual's, which merges it across 2.0000kΩ–2.0000MΩ.
        RangeSpec {
            range: Some(0x34),
            label: "2.0000MΩ",
            spec: SpecInfo {
                resolution: "0.0001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.3%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x33),
            label: "20.000MΩ",
            spec: SpecInfo {
                resolution: "0.001MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.5%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x32),
            label: "100.00MΩ",
            spec: SpecInfo {
                resolution: "0.01MΩ",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5.0%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Allow for the test leads' own resistance and reactance"],
    },
};

static CAPACITANCE: ModeSpecs = ModeSpecs {
    name: name::CAPACITANCE,
    page: 36,
    ranges: &[
        RangeSpec {
            range: Some(0x0B),
            label: "2.000nF",
            spec: SpecInfo {
                resolution: "0.001nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "5.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x0A),
            label: "20.00nF",
            spec: SpecInfo {
                resolution: "0.01nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x09),
            label: "200.0nF",
            spec: SpecInfo {
                resolution: "0.1nF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x13),
            label: "2.000µF",
            spec: SpecInfo {
                resolution: "0.001µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x12),
            label: "20.00µF",
            spec: SpecInfo {
                resolution: "0.01µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x11),
            label: "200.0µF",
            spec: SpecInfo {
                resolution: "0.1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        // The UT217 manual prints 2.000mF at 0.001mF.
        RangeSpec {
            range: Some(0x10),
            label: "2000µF",
            spec: SpecInfo {
                resolution: "1µF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
        RangeSpec {
            range: Some(0x1A),
            label: "20.00mF",
            spec: SpecInfo {
                resolution: "0.01mF",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "3.0%+10",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Accuracy valid 5%–100% of range",
            "Allow for the test leads' own resistance and reactance",
            "20.00mF: measuring duration should be over 30 seconds",
        ],
    },
};

static FREQUENCY: ModeSpecs = ModeSpecs {
    name: name::FREQUENCY,
    page: 36,
    ranges: &[
        RangeSpec {
            range: Some(0x22),
            label: "200.00Hz",
            spec: SpecInfo {
                resolution: "0.01Hz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2C),
            label: "2.0000kHz",
            spec: SpecInfo {
                resolution: "0.0001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2B),
            label: "20.000kHz",
            spec: SpecInfo {
                resolution: "0.001kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x2A),
            label: "200.00kHz",
            spec: SpecInfo {
                resolution: "0.01kHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x34),
            label: "2.0000MHz",
            spec: SpecInfo {
                resolution: "0.0001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
        RangeSpec {
            range: Some(0x33),
            label: "20.000MHz",
            spec: SpecInfo {
                resolution: "0.001MHz",
                accuracy: &[AccuracyBand {
                    freq_range: None,
                    accuracy: "0.1%+5",
                }],
            },
        },
    ],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &[
            "Typical waveform: square or sine",
            "Signal: 1Hz–20MHz, ≥ 1V rms",
        ],
    },
};

static DUTY_CYCLE: ModeSpecs = ModeSpecs {
    name: name::DUTY_CYCLE,
    page: 36,
    ranges: &[RangeSpec {
        range: Some(0x21),
        label: "0.1%-99.9%",
        spec: SpecInfo {
            resolution: "0.10%",
            accuracy: &[
                AccuracyBand {
                    freq_range: Some("Typical: Vrms=1 V, f=100Hz"),
                    accuracy: "1.2%+3",
                },
                AccuracyBand {
                    freq_range: Some("≥1 kHz"),
                    accuracy: "2.5%+10",
                },
            ],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Typical waveform: square"],
    },
};

static DIODE: ModeSpecs = ModeSpecs {
    name: name::DIODE,
    page: 36,
    ranges: &[RangeSpec {
        range: Some(0x24),
        label: "3.0000V",
        spec: SpecInfo {
            resolution: "0.0001V",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Open circuit 3.2V"],
    },
};

static ON_OFF: ModeSpecs = ModeSpecs {
    name: name::ON_OFF,
    page: 36,
    ranges: &[RangeSpec {
        range: Some(0x21),
        label: "0~200.0Ω",
        spec: SpecInfo {
            resolution: "0.1Ω",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: None,
        notes: &["Beeps <50Ω", "Shows OL above 200.0Ω"],
    },
};

#[cfg(test)]
mod tests {
    use super::super::decode::{AUTO, HOLD, MAX, MIN, REL};
    use super::super::decode15::tests::{frame, g24};
    use super::super::decode15::{self, AC, DC, INRUSH, LPF, OSC, PEAK, USB, code};
    use super::super::model::{CMS061, MODELS, OW69B};
    use super::*;
    use crate::protocol::test_support::hex;
    use crate::protocol::{capture_reports, golden_readings, registry};
    use crate::transform::si_prefix;
    use crate::transport::mock::MockTransport;
    use std::collections::HashSet;

    /// The models with tables. A 15-byte frame names no model (spec §10.2):
    /// each is decoded as its own.
    const SHEETED: [&Model; 1] = [&CMS061];

    /// Why a reading has no row or no spec, and which readings that is.
    type Listed = (&'static str, fn(&Measurement) -> bool);

    /// The function codes the manual has a table for.
    const TABLED: [u8; 10] = [
        function::DC_V,
        function::AC_V,
        function::DC_A,
        function::AC_A,
        function::OHMS,
        function::CAPACITANCE,
        function::HZ,
        function::DUTY,
        function::DIODE,
        function::CONTINUITY,
    ];

    /// The prefixes by prefix code (spec §6.3).
    const PREFIXES: [&str; 8] = ["p", "n", "µ", "m", "", "k", "M", "G"];

    fn code_of(m: &Measurement) -> u8 {
        u8::try_from(m.mode_raw).unwrap()
    }

    /// The decimal-point code (spec §6.4).
    fn point(m: &Measurement) -> u8 {
        m.range_raw & 0x07
    }

    /// The scope: its status bit, or its function codes (spec §9.4).
    fn scope(m: &Measurement) -> bool {
        m.screen_only || matches!(code_of(m), code::POWER_W | code::POWER_VA)
    }

    fn lpf_or_peak(m: &Measurement) -> bool {
        m.mode.starts_with("LPF ") || m.mode.starts_with("Peak ")
    }

    /// A reading of a function with a table, plain or inrush on AC A.
    fn tabled(m: &Measurement) -> bool {
        !scope(m)
            && TABLED.contains(&code_of(m))
            && !lpf_or_peak(m)
            && (m.mode != "Inrush" || code_of(m) == function::AC_A)
    }

    /// A printed range's unit and digits after the point; a span's upper
    /// end, as 99.9% of 0.1%-99.9%.
    fn printed(label: &str) -> (&str, u8) {
        let shown = label.rsplit(['-', '~']).next().unwrap();
        let unit = shown.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
        let digits = &shown[..shown.len() - unit.len()];
        let point = digits.split_once('.').map_or(0, |(_, d)| d.len());
        (unit, u8::try_from(point).unwrap())
    }

    /// A printed range: its label, and the prefix and digits after the point
    /// it shows.
    type Printed = (&'static str, &'static str, u8);

    /// The CMS061 manual's tables (PDF pp.35-36): each printed range, in
    /// manual order, with the prefix and point it shows; a span by its upper
    /// end.
    const CMS061_MANUAL: &[(&str, &[Printed])] = &[
        (
            "DC Voltage (V)",
            &[
                ("20.000mV", "m", 3),
                ("200.00mV", "m", 2),
                ("2.0000V", "", 4),
                ("20.000V", "", 3),
                ("200.00V", "", 2),
                ("1000.0V", "", 1),
            ],
        ),
        (
            "AC Voltage (V)",
            &[
                ("200.00mV", "m", 2),
                ("2.0000V", "", 4),
                ("20.000V", "", 3),
                ("200.00V", "", 2),
                ("1000.0V", "", 1),
            ],
        ),
        (
            "DC Current (A)",
            &[("20.00A", "", 2), ("200.0A", "", 1), ("600A", "", 0)],
        ),
        (
            "AC Current (A)",
            &[("20.00A", "", 2), ("200.0A", "", 1), ("600A", "", 0)],
        ),
        (
            "Inrush Current(A)",
            &[("20.00A", "", 2), ("200.0A", "", 1), ("600A", "", 0)],
        ),
        (
            "Resistance(Ω)",
            &[
                ("200.00Ω", "", 2),
                ("2.0000kΩ", "k", 4),
                ("20.000kΩ", "k", 3),
                ("200.00kΩ", "k", 2),
                ("2.0000MΩ", "M", 4),
                ("20.000MΩ", "M", 3),
                ("100.00MΩ", "M", 2),
            ],
        ),
        (
            "Capacitance(F)",
            &[
                ("2.000nF", "n", 3),
                ("20.00nF", "n", 2),
                ("200.0nF", "n", 1),
                ("2.000µF", "µ", 3),
                ("20.00µF", "µ", 2),
                ("200.0µF", "µ", 1),
                ("2000µF", "µ", 0),
                ("20.00mF", "m", 2),
            ],
        ),
        (
            "Frequency (Hz)",
            &[
                ("200.00Hz", "", 2),
                ("2.0000kHz", "k", 4),
                ("20.000kHz", "k", 3),
                ("200.00kHz", "k", 2),
                ("2.0000MHz", "M", 4),
                ("20.000MHz", "M", 3),
            ],
        ),
        ("Duty Cycle (%)", &[("0.1%-99.9%", "", 1)]),
        ("Diode", &[("3.0000V", "", 4)]),
        ("On-Off", &[("0~200.0Ω", "", 1)]),
    ];

    /// `model`'s manual tables, as [`CMS061_MANUAL`] writes them.
    fn manual(model: &Model) -> Vec<(&'static str, Vec<Printed>)> {
        if model.code == CMS061.code {
            CMS061_MANUAL
                .iter()
                .map(|(name, rows)| (*name, rows.to_vec()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// The range byte a prefix and point make (spec §10.3, §6.3, §6.4).
    fn key(prefix: &str, point: u8) -> u8 {
        let code = PREFIXES.iter().position(|p| *p == prefix).unwrap();
        u8::try_from(code).unwrap() << 3 | point
    }

    /// Whether the manual prints a range of the reading's table with the
    /// prefix and point its range byte gives. The CMS101's manual prints the
    /// same prefixes and points.
    fn printed_range(m: &Measurement) -> bool {
        table_name(m).is_some_and(|name| {
            CMS061_MANUAL.iter().any(|(table, rows)| {
                *table == name
                    && rows
                        .iter()
                        .any(|(_, prefix, point)| key(prefix, *point) == m.range_raw)
            })
        })
    }

    /// Readings that take their table's mode data but no row.
    const MODE_SPEC_ONLY: &[Listed] = &[
        (
            "OL or UL by the decimal-point code, which leaves the range untold",
            |m| tabled(m) && point(m) >= 6,
        ),
        (
            "a prefix and point no printed range has, such as 0.000 Hz",
            |m| tabled(m) && point(m) < 6 && !printed_range(m),
        ),
    ];

    /// Readings the decoder accepts that the manual gives no spec.
    const NO_SPEC: &[Listed] = &[
        (
            "the scope: the manual gives its readings no accuracy",
            scope,
        ),
        ("NCV: a level, which the manual gives no accuracy", |m| {
            !scope(m) && code_of(m) == function::NCV
        }),
        (
            "a function the meter has no position for: °C, °F, hFE and the other 15-byte meters' functions",
            |m| !scope(m) && code_of(m) != function::NCV && !TABLED.contains(&code_of(m)),
        ),
        ("LPF or Peak: the manual has no table for them", |m| {
            !scope(m) && TABLED.contains(&code_of(m)) && lpf_or_peak(m)
        }),
        (
            "inrush on another function than AC A: no state the manual shows",
            |m| {
                !scope(m)
                    && TABLED.contains(&code_of(m))
                    && m.mode == "Inrush"
                    && code_of(m) != function::AC_A
            },
        ),
    ];

    /// Every reading `model` decodes without a report, over every function
    /// code, prefix and decimal-point code: with no status bit, AUTO and
    /// HOLD, REL, MIN, MAX, inrush, the scope, LPF, Peak and the power
    /// contexts, under a count, an NCV level, OL by the reading word and
    /// dashes; and with a Hz, AC V, AC A or duty sub-display.
    fn each_reading(model: &Model, mut f: impl FnMut(Measurement)) {
        let flags = |bits: u16| u32::from(bits);
        let statuses = [
            0,
            flags(AUTO | HOLD),
            flags(REL),
            flags(MIN),
            flags(MAX),
            INRUSH,
            OSC,
            LPF,
            PEAK,
            LPF | PEAK,
            INRUSH | LPF,
            AC | DC | USB,
        ];
        let values = [1234, 3, 1 << 20, 6 << 20];
        let subs = [
            g24(function::HZ, 4, 2, false),
            g24(function::AC_V, 4, 4, false),
            g24(function::AC_A, 4, 2, false),
            g24(function::DUTY, 4, 1, false),
        ];
        let mut keep = |p: [u8; 15]| {
            if let (Ok(m), reports) = capture_reports(|| decode15::decode(&p, model))
                && reports.is_empty()
            {
                f(m);
            }
        };
        for function in 0..32 {
            for prefix in 0..8 {
                for dp in 0..8 {
                    for status in statuses {
                        for v in values {
                            keep(frame(g24(function, prefix, dp, false), v, 0, 0, status));
                        }
                    }
                    for sub in subs {
                        keep(frame(g24(function, prefix, dp, true), 1234, sub, 567, 0));
                    }
                }
            }
        }
    }

    fn what(m: &Measurement) -> String {
        format!(
            "{} {:?} {:?} in {} (range byte {:#04X})",
            m.mode, m.value, m.display_raw, m.unit, m.range_raw
        )
    }

    /// Each reading resolves a row, is listed as taking its table's mode
    /// data only, or is listed as having none; every table, every row and
    /// every list entry is some reading's.
    #[test]
    fn every_reading_has_a_spec_or_is_listed() {
        let lists = [MODE_SPEC_ONLY, NO_SPEC];
        let mut listed_reached = HashSet::new();
        for model in SHEETED {
            let tables = for_model(model);
            let proto = (model.entry.new_protocol)();
            let mut rows_reached = HashSet::new();
            let mut tables_reached = HashSet::new();
            each_reading(model, |m| {
                let listed: Vec<(usize, &str)> = lists
                    .iter()
                    .enumerate()
                    .flat_map(|(l, list)| list.iter().map(move |entry| (l, entry)))
                    .filter(|(_, (_, hit))| hit(&m))
                    .map(|(l, (why, _))| (l, *why))
                    .collect();
                let mode_spec = proto.mode_spec_info(&m);
                if let Some(t) = table(tables, &m) {
                    assert!(std::ptr::eq(mode_spec.unwrap(), &t.mode), "{}", what(&m));
                    tables_reached.insert(std::ptr::from_ref(t));
                }
                match (row(tables, &m), listed.as_slice()) {
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
            });
            for t in tables {
                assert!(
                    tables_reached.contains(&std::ptr::from_ref(*t)),
                    "{}: {} is no reading's",
                    model.entry.id,
                    t.name
                );
                for r in t.ranges {
                    assert!(
                        rows_reached.contains(&std::ptr::from_ref(r)),
                        "{}: {} / {} is no reading's",
                        model.entry.id,
                        t.name,
                        r.label
                    );
                }
            }
        }
        for (why, _) in lists.iter().flat_map(|list| list.iter()) {
            assert!(listed_reached.contains(why), "no reading is {why}");
        }
    }

    /// A row's printed range is in the reading's unit with the reading's
    /// point, and its resolution in the same unit.
    #[test]
    fn rows_carry_the_readings_unit_and_point() {
        for model in SHEETED {
            each_reading(model, |m| {
                let Some(row) = row(for_model(model), &m) else {
                    return;
                };
                assert_eq!(
                    printed(row.label),
                    (m.unit.as_ref(), point(&m)),
                    "{}: row {}",
                    what(&m),
                    row.label
                );
                let resolution = row
                    .spec
                    .resolution
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
                assert_eq!(
                    si_prefix(resolution),
                    si_prefix(&m.unit),
                    "{}: row {}",
                    what(&m),
                    row.label
                );
            });
        }
    }

    /// Each model's sheet holds the manual's tables and printed ranges, in
    /// manual order, each keyed on the range byte its prefix and point make.
    #[test]
    fn sheets_hold_the_manuals_ranges() {
        // Each table's title, and its rows' range bytes and labels.
        type Tables<'a> = Vec<(&'a str, Vec<(Option<u8>, &'a str)>)>;
        for model in SHEETED {
            let sheet = (model.entry.new_protocol)().spec_sheet();
            let got: Tables = sheet
                .iter()
                .map(|t| {
                    (
                        t.name,
                        t.rows.iter().map(|r| (r.range_raw, r.label)).collect(),
                    )
                })
                .collect();
            let want: Tables = manual(model)
                .into_iter()
                .map(|(name, rows)| {
                    let rows = rows
                        .iter()
                        .map(|(label, prefix, point)| (Some(key(prefix, *point)), *label))
                        .collect();
                    (name, rows)
                })
                .collect();
            assert_eq!(got, want, "{}", model.entry.id);
        }
    }

    /// The table and row label a CMS061 frame takes.
    fn lookup(p: [u8; 15]) -> (Option<&'static str>, Option<&'static str>) {
        let (m, reports) = capture_reports(|| decode15::decode(&p, &CMS061));
        assert!(reports.is_empty(), "{p:02X?}: {reports:?}");
        let m = m.unwrap();
        (
            table(CMS061_TABLES, &m).map(|t| t.name),
            row(CMS061_TABLES, &m).map(|r| r.label),
        )
    }

    /// Each function takes its own table: AC against DC, inrush its own,
    /// continuity against diode, Hz the frequency table at every position,
    /// and REL and MAX the function's.
    #[test]
    fn each_function_takes_its_table() {
        use function::*;
        let auto = u32::from(AUTO);
        let alone = |f, prefix, dp, status| frame(g24(f, prefix, dp, false), 1234, 0, 0, status);
        for (p, table_name, label) in [
            (alone(DC_V, 4, 3, auto), "DC Voltage (V)", "20.000V"),
            (alone(AC_V, 4, 3, auto), "AC Voltage (V)", "20.000V"),
            (alone(DC_A, 4, 2, auto), "DC Current (A)", "20.00A"),
            (alone(AC_A, 4, 2, auto), "AC Current (A)", "20.00A"),
            (alone(AC_A, 4, 2, INRUSH), "Inrush Current(A)", "20.00A"),
            (alone(OHMS, 5, 3, auto), "Resistance(Ω)", "20.000kΩ"),
            (alone(CAPACITANCE, 2, 2, auto), "Capacitance(F)", "20.00µF"),
            (alone(HZ, 6, 3, auto), "Frequency (Hz)", "20.000MHz"),
            // Hz in the large digits at the V and A positions, beside the
            // AC reading.
            (
                frame(
                    g24(HZ, 4, 2, true),
                    5000,
                    g24(AC_V, 4, 1, false),
                    2301,
                    auto,
                ),
                "Frequency (Hz)",
                "200.00Hz",
            ),
            (
                frame(
                    g24(HZ, 5, 4, true),
                    12345,
                    g24(AC_A, 4, 1, false),
                    123,
                    auto,
                ),
                "Frequency (Hz)",
                "2.0000kHz",
            ),
            (alone(DUTY, 4, 1, auto), "Duty Cycle (%)", "0.1%-99.9%"),
            (alone(DIODE, 4, 4, 0), "Diode", "3.0000V"),
            (alone(CONTINUITY, 4, 1, 0), "On-Off", "0~200.0Ω"),
            (
                alone(DC_V, 4, 1, u32::from(REL)),
                "DC Voltage (V)",
                "1000.0V",
            ),
            (
                frame(
                    g24(DC_V, 4, 1, true),
                    285,
                    g24(DC_V, 4, 1, false),
                    285,
                    u32::from(MAX),
                ),
                "DC Voltage (V)",
                "1000.0V",
            ),
        ] {
            assert_eq!(lookup(p), (Some(table_name), Some(label)), "{p:02X?}");
        }
        for p in [
            alone(DC_V, 4, 3, INRUSH),
            alone(AC_V, 4, 3, LPF),
            alone(AC_A, 4, 2, PEAK),
            frame(g24(NCV, 4, 1, false), 3, 0, 0, 0),
            alone(CELSIUS, 4, 1, 0),
        ] {
            assert_eq!(lookup(p), (None, None), "{p:02X?}");
        }
    }

    /// The table and printed range each CMS061 golden reading takes.
    #[test]
    fn real_frames_take_their_rows() {
        const DCV: &str = "DC Voltage (V)";
        const HZ: &str = "Frequency (Hz)";
        // By file: the table's title and the printed range, if any.
        type Want = Option<(&'static str, Option<&'static str>)>;
        let want: [(&str, Want); 33] = [
            ("aca_hz.yaml", Some((HZ, Some("200.00Hz")))),
            ("aca.yaml", Some(("AC Current (A)", Some("20.00A")))),
            ("acmv.yaml", Some(("AC Voltage (V)", Some("200.00mV")))),
            ("acv_hz.yaml", Some((HZ, Some("200.00Hz")))),
            ("cap.yaml", Some(("Capacitance(F)", Some("2.000nF")))),
            ("cont.yaml", Some(("On-Off", Some("0~200.0Ω")))),
            ("dca.yaml", Some(("DC Current (A)", Some("20.00A")))),
            ("dca_zero.yaml", Some(("DC Current (A)", Some("20.00A")))),
            ("dcmv.yaml", Some((DCV, Some("20.000mV")))),
            ("dcv_negative.yaml", Some((DCV, Some("200.00V")))),
            ("dcv_source.yaml", Some((DCV, Some("200.00V")))),
            ("dcv.yaml", Some((DCV, Some("2.0000V")))),
            // OL by the reading word, on its range.
            ("diode.yaml", Some(("Diode", Some("3.0000V")))),
            ("duty.yaml", Some(("Duty Cycle (%)", Some("0.1%-99.9%")))),
            ("hold.yaml", Some((DCV, Some("2.0000V")))),
            ("hz_duty_signal.yaml", Some((HZ, Some("200.00kHz")))),
            // 0.000 Hz: no printed range has three digits after the point.
            ("hz.yaml", Some((HZ, None))),
            ("inrush.yaml", Some(("Inrush Current(A)", Some("20.00A")))),
            ("key_hold.yaml", Some((DCV, Some("2.0000V")))),
            (
                "key_hz_duty.yaml",
                Some(("Duty Cycle (%)", Some("0.1%-99.9%"))),
            ),
            (
                "key_inrush.yaml",
                Some(("Inrush Current(A)", Some("20.00A"))),
            ),
            ("key_minmax.yaml", Some((DCV, Some("1000.0V")))),
            ("key_range.yaml", Some((DCV, Some("1000.0V")))),
            ("key_rel.yaml", Some((DCV, Some("200.00V")))),
            ("key_select.yaml", Some(("AC Voltage (V)", Some("200.00V")))),
            ("manual_range.yaml", Some((DCV, Some("20.000V")))),
            ("ncv_ef.yaml", None),
            ("ncv.yaml", None),
            ("ohm_body.yaml", Some(("Resistance(Ω)", Some("2.0000MΩ")))),
            ("ohm_short.yaml", Some(("Resistance(Ω)", Some("200.00Ω")))),
            ("rel.yaml", Some((DCV, Some("200.00V")))),
            // The scope, entered from Capacitance: its stale gear is no
            // reading.
            ("scope_after_cap.yaml", None),
            ("scope.yaml", None),
        ];
        let (proto, readings) = golden_readings("cms061");
        assert_eq!(readings.len(), want.len());
        for (file, m) in &readings {
            let (_, want) = want
                .iter()
                .find(|(name, _)| name == file)
                .unwrap_or_else(|| panic!("{file} has no expected row"));
            let got =
                table(CMS061_TABLES, m).map(|t| (t.name, row(CMS061_TABLES, m).map(|r| r.label)));
            assert_eq!(got, *want, "{file}: {}", what(m));
            assert_eq!(
                proto.spec_info(m).map(|s| s.resolution),
                row(CMS061_TABLES, m).map(|r| r.spec.resolution)
            );
        }
    }

    /// A reading through `Dmm` carries its row and its table's mode data:
    /// cms061/acmv.yaml, 26.28 mV AC, three frames in one notification.
    #[test]
    fn a_reading_through_dmm_carries_its_spec() {
        let acmv = hex("5A 00 F0 44 0A 00 A4 01 F0 23 04 00 00 00 00");
        let cable = MockTransport::new(vec![acmv.repeat(3)]);
        let entry = registry::find_device("cms061").unwrap();
        let mut dmm = crate::Dmm::new(cable, (entry.new_protocol)()).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("AC V", "mV"));
        let spec = m.spec.expect("a row");
        assert!(std::ptr::eq(spec, &AC_VOLTAGE.ranges[0].spec));
        assert_eq!(spec.accuracy[0].accuracy, "0.6%+10");
        let mode = m.mode_spec.expect("mode data");
        assert!(std::ptr::eq(mode, &AC_VOLTAGE.mode));
    }

    /// The models with tables list them in manual order; every other OWON
    /// model has no sheet and no spec, the CMS061's frames included.
    #[test]
    fn other_owon_models_have_no_sheet() {
        for model in MODELS {
            let sheet = (model.entry.new_protocol)().spec_sheet();
            if SHEETED.iter().any(|s| std::ptr::eq(*s, model)) {
                let tables = for_model(model);
                assert_eq!(sheet.len(), tables.len(), "{}", model.entry.id);
                for (t, want) in sheet.iter().zip(tables) {
                    assert!(std::ptr::eq(t.mode, &want.mode));
                }
                assert_eq!(sheet[0].name, "DC Voltage (V)");
            } else {
                assert!(sheet.is_empty(), "{}", model.entry.id);
                assert!(for_model(model).is_empty(), "{}", model.entry.id);
            }
        }
        let dcv = frame(g24(function::DC_V, 4, 2, false), 2861, 0, 0, 0);
        let m = decode15::decode(&dcv, &OW69B).unwrap();
        let ow69b = (OW69B.entry.new_protocol)();
        assert!(ow69b.spec_info(&m).is_none() && ow69b.mode_spec_info(&m).is_none());
        let cms061 = (CMS061.entry.new_protocol)();
        assert_eq!(cms061.spec_info(&m).unwrap().resolution, "0.01V");
    }
}
