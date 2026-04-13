//! Specification data for the UNI-T UT61E+ (22,000 counts).
//! Transcribed from references/ut61eplus/ut61e_manual.pdf, section IX.2.

use super::{AccuracyBand, ModeSpecInfo, SpecInfo};

// ── DC Voltage (manual page 26) ─────────────────────────────────────────

// Range order matches ut61e_plus.rs: 2.2V, 22V, 220V, 1000V, 220mV
// ±(0.1%+5) for 220mV; ±(0.05%+5) merged for 2.2V/22V/220V; ±(0.1%+5) for 1000V
pub static DC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.05%+5",
        }],
    },
    SpecInfo {
        resolution: "1mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.05%+5",
        }],
    },
    SpecInfo {
        resolution: "10mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.05%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+5",
        }],
    },
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+5",
        }],
    },
];

pub static DC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &[],
};

// ── AC Voltage (manual page 27) ─────────────────────────────────────────

// Range order: 2.2V, 22V, 220V, 750V, 220mV
pub static AC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "1.2%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "1mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "1.2%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "10mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "2.0%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.2%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3.0%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.0%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "1.5%+30",
            },
        ],
    },
];

pub static AC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}10kHz"],
};

// ── DC mV (manual page 26) ──────────────────────────────────────────────

// Range order: 220mV, 2.2V (in mV mode)
pub static DC_MV_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.05%+5",
        }],
    },
];

pub static DC_MV_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~1 G\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &[],
};

// ── AC mV (manual page 27) ──────────────────────────────────────────────

pub static AC_MV_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.0%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "1.5%+30",
            },
        ],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "1.2%+50",
            },
        ],
    },
];

pub static AC_MV_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}10kHz"],
};

// ── Resistance (manual page 29) ─────────────────────────────────────────

// Range order: 220Ω, 2.2kΩ, 22kΩ, 220kΩ, 2.2MΩ, 22MΩ, 220MΩ
// ±(0.5+10) merged for 220Ω–220kΩ, then individual values for MΩ ranges
pub static OHM_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "0.1\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "1\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "10\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "100\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.8%+10",
        }],
    },
    SpecInfo {
        resolution: "1k\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.5%+10",
        }],
    },
    SpecInfo {
        resolution: "10k\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3.0%+50",
        }],
    },
];

pub static OHM_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Open circuit ~3V"],
};

// ── Continuity (manual page 30) ─────────────────────────────────────────

// Manual lists resolution only, no accuracy value.
pub static CONTINUITY_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.1\u{2126}",
    accuracy: &[],
}];

pub static CONTINUITY_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Beep <50\u{2126}"],
};

// ── Diode (manual page 30) ──────────────────────────────────────────────

// Manual lists resolution only, no accuracy value.
pub static DIODE_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.001V",
    accuracy: &[],
}];

pub static DIODE_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Forward 0.12V\u{2013}2V"],
};

// ── Capacitance (manual page 30) ────────────────────────────────────────

// Range order: 22nF, 220nF, 2.2µF, 22µF, 220µF, 2.2mF, 22mF, 220mF
// ±(3.0%+5) merged for 22nF/220nF/2.2µF/22µF; ±(4.0%+5) merged for 220µF/2.2mF
pub static CAP_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1pF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3.0%+5",
        }],
    },
    SpecInfo {
        resolution: "10pF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3.0%+5",
        }],
    },
    SpecInfo {
        resolution: "100pF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3.0%+5",
        }],
    },
    SpecInfo {
        resolution: "1nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3.0%+5",
        }],
    },
    SpecInfo {
        resolution: "10nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "4.0%+5",
        }],
    },
    SpecInfo {
        resolution: "100nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "4.0%+5",
        }],
    },
    SpecInfo {
        resolution: "1\u{00B5}F",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "10%+5",
        }],
    },
    SpecInfo {
        resolution: "10\u{00B5}F",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "20%+5",
        }],
    },
];

pub static CAP_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Use REL for small values"],
};

// ── Temperature (manual page 31) ────────────────────────────────────────

// Single protocol range, but accuracy varies by measured temperature.
pub static TEMP_C_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.1\u{00B0}C",
    accuracy: &[
        AccuracyBand {
            freq_range: Some("-40\u{00B0}C\u{2013}0\u{00B0}C"),
            accuracy: "1.0%+3\u{00B0}C",
        },
        AccuracyBand {
            freq_range: Some("0\u{00B0}C\u{2013}300\u{00B0}C"),
            accuracy: "1.0%+2\u{00B0}C",
        },
        AccuracyBand {
            freq_range: Some("300\u{00B0}C\u{2013}1000\u{00B0}C"),
            accuracy: "1.0%+3\u{00B0}C",
        },
    ],
}];

pub static TEMP_F_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.2\u{00B0}F",
    accuracy: &[
        AccuracyBand {
            freq_range: Some("-40\u{00B0}F\u{2013}32\u{00B0}F"),
            accuracy: "1.0%+6\u{00B0}F",
        },
        AccuracyBand {
            freq_range: Some("32\u{00B0}F\u{2013}572\u{00B0}F"),
            accuracy: "1.0%+4\u{00B0}F",
        },
        AccuracyBand {
            freq_range: Some("572\u{00B0}F\u{2013}1832\u{00B0}F"),
            accuracy: "1.0%+6\u{00B0}F",
        },
    ],
}];

pub static TEMP_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: None,
    notes: &["K-type thermocouple", "Max 1000\u{00B0}C / 1832\u{00B0}F"],
};

// ── DC µA (manual page 32) ──────────────────────────────────────────────

// Range order: 220µA, 2200µA
pub static DC_UA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "0.1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
];

pub static DC_UA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &[],
};

// ── AC µA (manual page 33) ──────────────────────────────────────────────

pub static AC_UA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01\u{00B5}A",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "0.1\u{00B5}A",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "0.8%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
];

pub static AC_UA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}10kHz"],
};

// ── DC mA (manual page 32) ──────────────────────────────────────────────

// Range order: 22mA, 220mA
// ±(0.5%+10) merged for all µA+mA ranges (220µA/2200µA/22mA/220mA)
pub static DC_MA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
    SpecInfo {
        resolution: "10\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+10",
        }],
    },
];

pub static DC_MA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &[],
};

// ── AC mA (manual page 33) ──────────────────────────────────────────────

pub static AC_MA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1\u{00B5}A",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.2%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "10\u{00B5}A",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.2%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
];

pub static AC_MA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}10kHz"],
};

// ── DC A (manual page 32) ───────────────────────────────────────────────

// Range order: 20A, 20A (two identical range bytes)
pub static DC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+50",
        }],
    },
    SpecInfo {
        resolution: "1mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+50",
        }],
    },
];

pub static DC_A_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F2 10A 240V"),
    notes: &[">5A: max 10s, rest 15min"],
};

// ── AC A (manual page 33) ───────────────────────────────────────────────

pub static AC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1mA",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.2%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
    SpecInfo {
        resolution: "1mA",
        accuracy: &[
            AccuracyBand {
                freq_range: Some("40Hz\u{2013}1kHz"),
                accuracy: "1.2%+10",
            },
            AccuracyBand {
                freq_range: Some("1kHz\u{2013}10kHz"),
                accuracy: "3%+50",
            },
        ],
    },
];

pub static AC_A_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F2 10A 240V"),
    notes: &["True RMS", "40Hz\u{2013}10kHz", ">5A: max 10s, rest 15min"],
};

// ── Hz (manual page 34) ─────────────────────────────────────────────────

// Range order: 22Hz, 220Hz, 2.2kHz, 22kHz, 220kHz
pub static HZ_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.001Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.01%+5",
        }],
    },
    SpecInfo {
        resolution: "0.01Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.01%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.01%+5",
        }],
    },
    SpecInfo {
        resolution: "1Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.01%+5",
        }],
    },
    SpecInfo {
        resolution: "10Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.01%+5",
        }],
    },
];

pub static HZ_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["10Hz\u{2013}220MHz"],
};

// ── Duty % (manual page 34) ─────────────────────────────────────────────

pub static DUTY_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.1%",
    accuracy: &[AccuracyBand {
        freq_range: None,
        accuracy: "2%+5",
    }],
}];

pub static DUTY_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Square waves only"],
};

// ── LPF Voltage (UT61E+ only, manual page 27) ──────────────────────────

// LPF mode accuracy from the AC Voltage table.
// When LPF is enabled (SELECT in AC V mode), bandwidth is 40Hz–100Hz.
// The meter reports Mode::LpfV (0x18) — a distinct mode byte from AcV.
// Range order matches dc_v: 2.2V, 22V, 220V, 1000V, 220mV
// 220mV has no LPF data in the manual.
pub static LPF_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}100Hz"),
            accuracy: "1.2%+50",
        }],
    },
    SpecInfo {
        resolution: "1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}100Hz"),
            accuracy: "1.8%+50",
        }],
    },
    SpecInfo {
        resolution: "10mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}100Hz"),
            accuracy: "2.0%+50",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}100Hz"),
            accuracy: "3.0%+50",
        }],
    },
];

pub static LPF_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "LPF 40Hz\u{2013}100Hz"],
};

// ── LPF mV (UT61E+ only, manual page 27) ────────────────────────────────

// Range order matches dc_mv: 220mV (index 0), 2.2V (index 1).
// 220mV has no LPF row in the manual — empty accuracy signals "no data".
pub static LPF_MV_SPECS: &[SpecInfo] = &[
    // Index 0: 220mV — not in manual's LPF column.
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[],
    },
    // Index 1: 2.2V
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}100Hz"),
            accuracy: "1.2%+50",
        }],
    },
];

pub static LPF_MV_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "LPF 40Hz\u{2013}100Hz"],
};

// ── hFE / Transistor Magnification (UT61E+ only, manual page 30) ───────
pub static HFE_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "1\u{03B2}",
    accuracy: &[AccuracyBand {
        freq_range: None,
        accuracy: "reference only",
    }],
}];

pub static HFE_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: None,
    notes: &["Ib0 \u{2248}1.8\u{00B5}A", "Vce \u{2248}2.5V"],
};

// ── AC+DC Voltage (UT61E+ only, manual page 28) ────────────────────────

// Range order matches dc_v: 2.2V, 22V, 220V, 1000V, 220mV
// AC+DC starts at 2.2V (no 220mV range in manual), but we match the
// range byte index from the DcV table. Ranges 0-3 have data, range 4 (220mV) = None.
// ±(1.8%+70) merged for 2.2V/22V/220V; ±(4.0%+70) for 1000V
pub static ACDC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.8%+70",
        }],
    },
    SpecInfo {
        resolution: "1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.8%+70",
        }],
    },
    SpecInfo {
        resolution: "10mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.8%+70",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "4.0%+70",
        }],
    },
];

pub static ACDC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_v_has_5_ranges() {
        assert_eq!(DC_V_SPECS.len(), 5);
    }

    #[test]
    fn ac_v_has_5_ranges_with_2_bands_each() {
        assert_eq!(AC_V_SPECS.len(), 5);
        for spec in AC_V_SPECS {
            assert_eq!(spec.accuracy.len(), 2);
        }
    }

    #[test]
    fn lpf_v_has_4_ranges() {
        // LPF V covers 2.2V–1000V (no 220mV), matching dc_v indices 0-3
        assert_eq!(LPF_V_SPECS.len(), 4);
    }

    #[test]
    fn ohm_has_7_ranges() {
        assert_eq!(OHM_SPECS.len(), 7);
    }

    #[test]
    fn cap_has_8_ranges() {
        assert_eq!(CAP_SPECS.len(), 8);
    }

    #[test]
    fn hz_has_5_ranges() {
        assert_eq!(HZ_SPECS.len(), 5);
    }

    #[test]
    fn dc_v_resolution_matches_manual() {
        // 2.2V range (index 0) → 0.1mV resolution
        assert_eq!(DC_V_SPECS[0].resolution, "0.1mV");
        // 220mV range (index 4) → 0.01mV resolution
        assert_eq!(DC_V_SPECS[4].resolution, "0.01mV");
    }

    #[test]
    fn acdc_v_has_4_ranges() {
        assert_eq!(ACDC_V_SPECS.len(), 4);
    }
}
