//! Specification data for the UNI-T UT61B+ (6,000 counts).
//! Transcribed from references/ut61eplus/ut61e_manual.pdf, section IX.2.

use super::{AccuracyBand, ModeSpecInfo, SpecInfo};

// ── DC Voltage (manual page 26) ─────────────────────────────────────────

// Range order matches ut61b_plus.rs: 60mV, 600mV, 6V, 60V, 600V, 1000V
pub static DC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.8%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.8%+5",
        }],
    },
    SpecInfo {
        resolution: "0.001V",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+3",
        }],
    },
    SpecInfo {
        resolution: "0.01V",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+3",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.5%+3",
        }],
    },
    SpecInfo {
        resolution: "1V",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+3",
        }],
    },
];

pub static DC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &[],
};

// ── AC Voltage (manual page 27) ─────────────────────────────────────────

// Range order: 60mV, 600mV, 6V, 60V, 600V, 750V
// UT61B+ frequency response: 40Hz–500Hz (single band)
pub static AC_V_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.001V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "0.01V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "0.1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "1V",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
];

pub static AC_V_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz"],
};

// ── DC mV (manual page 26) ──────────────────────────────────────────────

// Range order: 60mV, 600mV
// ±(0.8%+5) merged for 60mV/600mV (same as DC V table rows 1-2)
pub static DC_MV_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.8%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.8%+5",
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
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mV",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
];

pub static AC_MV_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: Some("~10 M\u{2126}"),
    overload_protection: Some("1000V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz"],
};

// ── Resistance (manual page 29) ─────────────────────────────────────────

// Range order: 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ
// PDF page 29: 600Ω ±(1.2%+2), 6k–600k ±(1.0%+2), 6M ±(1.2%+2), 60M ±(2.0%+5)
pub static OHM_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+2",
        }],
    },
    SpecInfo {
        resolution: "1\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+2",
        }],
    },
    SpecInfo {
        resolution: "10\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+2",
        }],
    },
    SpecInfo {
        resolution: "100\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+2",
        }],
    },
    SpecInfo {
        resolution: "1k\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.2%+2",
        }],
    },
    SpecInfo {
        resolution: "10k\u{2126}",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "2.0%+5",
        }],
    },
];

pub static OHM_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Open circuit ~1V"],
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

// ── Capacitance (manual page 31) ────────────────────────────────────────

// Range order: 60nF, 600nF, 6µF, 60µF, 600µF, 6mF, 60mF
// ±(3%+5) merged for 60nF/600nF/6µF/60µF/600µF; ±(10%+5) merged for 6mF/60mF
pub static CAP_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10pF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3%+5",
        }],
    },
    SpecInfo {
        resolution: "100pF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3%+5",
        }],
    },
    SpecInfo {
        resolution: "1nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3%+5",
        }],
    },
    SpecInfo {
        resolution: "10nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3%+5",
        }],
    },
    SpecInfo {
        resolution: "100nF",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "3%+5",
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
            accuracy: "10%+5",
        }],
    },
];

pub static CAP_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Use REL for \u{2264}1\u{00B5}F"],
};

// ── DC µA (manual page 32) ──────────────────────────────────────────────

// Range order: 600µA, 6000µA
// PDF page 32: both share ±(1.0%+2)
pub static DC_UA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+2",
        }],
    },
    SpecInfo {
        resolution: "1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+2",
        }],
    },
];

pub static DC_UA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &[],
};

// ── AC µA (manual page 33) ──────────────────────────────────────────────

// PDF page 33: both share ±(1.2%+5)
pub static AC_UA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
    SpecInfo {
        resolution: "1\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.2%+5",
        }],
    },
];

pub static AC_UA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz"],
};

// ── DC mA (manual page 32) ──────────────────────────────────────────────

// Range order: 60mA, 600mA
// PDF page 32: both share ±(1.0%+3)
pub static DC_MA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+3",
        }],
    },
    SpecInfo {
        resolution: "0.1mA",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "1.0%+3",
        }],
    },
];

pub static DC_MA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &[],
};

// ── AC mA (manual page 33) ──────────────────────────────────────────────

// PDF page 33: both share ±(1.5%+5)
pub static AC_MA_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "10\u{00B5}A",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.5%+5",
        }],
    },
    SpecInfo {
        resolution: "0.1mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "1.5%+5",
        }],
    },
];

pub static AC_MA_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F1 1A 240V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz"],
};

// ── DC A (manual page 32) ───────────────────────────────────────────────

// Range order: 6A, 10A
// PDF page 32: both share ±(1.2%+5)
pub static DC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1mA",
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

// ── AC A (manual page 33) ───────────────────────────────────────────────

// PDF page 33: both share ±(2.0%+5)
pub static AC_A_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "1mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "2.0%+5",
        }],
    },
    SpecInfo {
        resolution: "10mA",
        accuracy: &[AccuracyBand {
            freq_range: Some("40Hz\u{2013}500Hz"),
            accuracy: "2.0%+5",
        }],
    },
];

pub static AC_A_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("F2 10A 240V"),
    notes: &["True RMS", "40Hz\u{2013}500Hz", ">5A: max 10s, rest 15min"],
};

// ── Hz (manual page 34) ─────────────────────────────────────────────────

// Range order: 60Hz, 600Hz, 6kHz, 60kHz, 600kHz
pub static HZ_SPECS: &[SpecInfo] = &[
    SpecInfo {
        resolution: "0.01Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+4",
        }],
    },
    SpecInfo {
        resolution: "0.1Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+4",
        }],
    },
    SpecInfo {
        resolution: "1Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+4",
        }],
    },
    SpecInfo {
        resolution: "10Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+4",
        }],
    },
    SpecInfo {
        resolution: "100Hz",
        accuracy: &[AccuracyBand {
            freq_range: None,
            accuracy: "0.1%+4",
        }],
    },
];

pub static HZ_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["10Hz\u{2013}10MHz"],
};

// ── Duty % (manual page 34) ─────────────────────────────────────────────

pub static DUTY_SPECS: &[SpecInfo] = &[SpecInfo {
    resolution: "0.1%",
    accuracy: &[AccuracyBand {
        freq_range: None,
        accuracy: "2.0%+5",
    }],
}];

pub static DUTY_MODE: ModeSpecInfo = ModeSpecInfo {
    input_impedance: None,
    overload_protection: Some("1000V"),
    notes: &["Square waves only"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_v_has_6_ranges() {
        assert_eq!(DC_V_SPECS.len(), 6);
    }

    #[test]
    fn ac_v_has_6_ranges_single_band() {
        assert_eq!(AC_V_SPECS.len(), 6);
        for spec in AC_V_SPECS {
            assert_eq!(spec.accuracy.len(), 1, "UT61B+ has single freq band");
        }
    }

    #[test]
    fn ohm_has_6_ranges() {
        assert_eq!(OHM_SPECS.len(), 6);
    }

    #[test]
    fn cap_has_7_ranges() {
        assert_eq!(CAP_SPECS.len(), 7);
    }
}
