//! Specification data for the UNI-T UT61D+ (6,000 counts).
//! Same as UT61B+ for most modes, but adds Temperature and LoZ V.
//! Frequency response for AC modes is 40Hz–1kHz (B+ is 40Hz–500Hz).
//! Transcribed from references/ut61eplus/ut61e_manual.pdf.

use super::{AccuracyBand, ModeSpecInfo, SpecInfo};

// DC specs are identical to UT61B+ — re-export.
pub use super::specs_ut61b_plus::{
    CAP_MODE, CAP_SPECS, CONTINUITY_MODE, CONTINUITY_SPECS, DC_MV_MODE, DC_MV_SPECS, DC_V_MODE,
    DC_V_SPECS, DIODE_MODE, DIODE_SPECS, DUTY_MODE, DUTY_SPECS, HZ_MODE, HZ_SPECS, OHM_MODE,
    OHM_SPECS,
};

// DC current: same accuracy as B+, re-export.
pub use super::specs_ut61b_plus::{DC_MA_MODE, DC_MA_SPECS, DC_UA_MODE, DC_UA_SPECS};

// AC voltage/current: same accuracy values as B+, but D+ frequency
// response is 40Hz–1kHz (manual page 27/33). Define own specs with
// correct frequency label in AccuracyBand.

// ── AC Voltage (manual page 27, D+ freq response 40Hz–1kHz) ────────

// Same accuracy values as B+, different frequency label.
// ±(1.2%+5) merged for 60mV/600mV; ±(1.0%+3) merged for 6V/60V/600V
pub static AC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.001V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "0.01V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
];

// ── AC mV (manual page 27, D+ freq response 40Hz–1kHz) ─────────────

pub static AC_MV_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
];

// ── UT61D+ AC mode info overrides (manual page 27, 40Hz–1kHz) ───────

pub static AC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz"],
};

pub static AC_MV_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz"],
};

pub static AC_UA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz"],
};

pub static AC_MA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz"],
};

// ── AC µA (manual page 33, D+ freq response 40Hz–1kHz) ─────────────

pub static AC_UA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.2%+5",
        }],
    },
];

// ── AC mA (manual page 33, D+ freq response 40Hz–1kHz) ─────────────

pub static AC_MA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.5%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "1.5%+5",
        }],
    },
];

// ── DC A: UT61D+ has 20A range (manual page 32) ────────────────────
pub static DC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "10mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+5",
        }],
    },
];

pub static DC_A_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F2 10A 240V"),
    notes: &[">5A: max 10s, rest 15min"],
};

// ── AC A: UT61D+ 20A range (manual page 33) ────────────────────────
pub static AC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "2.0%+5",
        }],
    },
    SpecInfo {
        resolution: "10mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "2.0%+5",
        }],
    },
];

pub static AC_A_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F2 10A 240V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz", ">5A: max 10s, rest 15min"],
};

// ── Temperature (UT61D+ only, manual page 31) ──────────────────────

// Same sub-range accuracy structure as UT61E+, but max 230°C/446°F.
pub static TEMP_C_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.1\u{00B0}C",
    accuracy: &[
        AccuracyBand {
            freq_range: Some("-40\u{00B0}C\u{2013}0\u{00B0}C"),
            accuracy: "1.0%+3\u{00B0}C",
        },
        AccuracyBand {
            freq_range: Some("0\u{00B0}C\u{2013}230\u{00B0}C"),
            accuracy: "1.0%+2\u{00B0}C",
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
            freq_range: Some("32\u{00B0}F\u{2013}446\u{00B0}F"),
            accuracy: "1.0%+4\u{00B0}F",
        },
    ],
}];

pub static TEMP_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: None,
    notes: &["K-type thermocouple", "Max 230\u{00B0}C / 446\u{00B0}F"],
};

// ── LoZ ACV (UT61D+ only, manual page 27) ──────────────────────────

pub static LOZ_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "2.0%+5",
        }],
    },
    SpecInfo {
        resolution: "1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}1kHz"),
            accuracy: "2.0%+5",
        }],
    },
];

pub static LOZ_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("Low Z"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}1kHz"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_c_has_1_range() {
        assert_eq!(TEMP_C_SPECS.len(), 1);
    }

    #[test]
    fn loz_v_has_2_ranges() {
        assert_eq!(LOZ_V_SPECS.len(), 2);
    }

    #[test]
    fn shared_specs_accessible() {
        // Verify re-exports work
        assert_eq!(DC_V_SPECS.len(), 6);
        assert_eq!(OHM_SPECS.len(), 6);
    }

    #[test]
    fn ac_v_mode_freq_response_is_1khz() {
        // D+ should have 40Hz–1kHz, not 40Hz–500Hz like B+
        assert!(AC_V_MODE.notes.iter().any(|n| n.contains("1kHz")));
    }

    #[test]
    fn ac_a_accuracy_is_2_percent() {
        assert_eq!(AC_A_SPECS[0].accuracy[0].accuracy, "2.0%+5");
    }
}
