use super::{BASE, ModelTable, RangeInfo, r};
use crate::protocol::ut61eplus::mode::Mode;

/// Device table for the UNI-T UT60BT, a handheld with Bluetooth built in.
///
/// 9,999-count model. Which modes it has, how many rungs each and their
/// range-byte order and units come from UNI-T's iDMM2.0 app, which ships the
/// table as a data file. The labels are the UT60BT
/// manual's printed ranges (§X, P2) where it prints the rung, the asset's
/// otherwise. No UT60BT has been connected; see
/// `docs/research/ut61-family/reverse-engineered-protocol.md` §9.
///
/// Key differences from the UT61E+ (22,000-count):
/// - V: 999.9mV..999.9V vs 2.2V..1000V; mV: 9.999mV/99.99mV vs 220mV
/// - Resistance: 6 ranges (999.9Ω..99.99MΩ); capacitance 8 (9.999nF..99.99mF)
/// - µA: one rung; mA and A share one dial position, the A rung sitting on
///   the mA modes, so there are no separate A modes
/// - Temperature; no hFE, LoZ, LPF, AC+DC, Peak
///
/// The dial is not described, so the meter offers no remote mode selection
/// until one confirms its button codes
/// (docs/research/ut61-family/verification.md).
pub(crate) static UT60BT: ModelTable = ModelTable {
    name: "UNI-T UT60BT",
    entry,
    modes: &[
        Mode::AcV,
        Mode::AcMv,
        Mode::DcV,
        Mode::DcMv,
        Mode::Hz,
        Mode::DutyCycle,
        Mode::Ohm,
        Mode::Continuity,
        Mode::Diode,
        Mode::Capacitance,
        Mode::TempC,
        Mode::TempF,
        Mode::DcUa,
        Mode::AcUa,
        Mode::DcMa,
        Mode::AcMa,
        Mode::Ncv,
    ],
    // The bytes UNI-T's app sends this meter (family spec §6.5): HOLD,
    // RANGE and its long press, REL and SELECT. It disables MAX/MIN, and
    // sends no Peak, SELECT2 or backlight.
    commands: &["hold", "range", "auto", "rel", "select"],
    // UNI-T's app asks the name first, and a UT60BT is reported to need it
    // (family spec §6.4).
    name_before_stream: true,
    // Its dial (manual P1, Figure 1): one V position, DC with AC on
    // SELECT; the Ω position starts on continuity; the Hz/% position takes
    // SELECT, not the USB button (§VIII).
    step_instructions: &[
        (
            "ohm",
            "Set meter to the \u{03A9} position and press SELECT for \u{03A9}. \
             Leave leads open (should show OL).",
        ),
        (
            "acv",
            "Set meter to V and press SELECT for AC V. Leave leads open.",
        ),
        ("duty", "Hz/% position: press SELECT for Duty %."),
    ],
    ..BASE
};

// The V position starts at 999.9mV, not at a volts rung: the mV
// position's 9.999mV and 99.99mV are the separate mV modes.
static DC_V: [RangeInfo; 4] = [
    r("999.9mV", "mV"),
    r("9.999V", "V"),
    r("99.99V", "V"),
    r("999.9V", "V"),
];

static AC_V: [RangeInfo; 4] = [
    r("999.9mV", "mV"),
    r("9.999V", "V"),
    r("99.99V", "V"),
    r("999.9V", "V"),
];

static DC_MV: [RangeInfo; 2] = [r("9.999mV", "mV"), r("99.99mV", "mV")];
static AC_MV: [RangeInfo; 2] = [r("9.999mV", "mV"), r("99.99mV", "mV")];

// Rung 4 is 9.999MΩ in the manual; the asset's label says 9.99MΩ
// over a 9.999 bound.
static OHM: [RangeInfo; 6] = [
    r("999.9Ω", "Ω"),
    r("9.999kΩ", "kΩ"),
    r("99.99kΩ", "kΩ"),
    r("999.9kΩ", "kΩ"),
    r("9.999MΩ", "MΩ"),
    r("99.99MΩ", "MΩ"),
];

// The manual stops at 9.999mF (seven rungs); the eighth is the
// asset's alone. [UNVERIFIED]
static CAPACITANCE: [RangeInfo; 8] = [
    r("9.999nF", "nF"),
    r("99.99nF", "nF"),
    r("999.9nF", "nF"),
    r("9.999µF", "µF"),
    r("99.99µF", "µF"),
    r("999.9µF", "µF"),
    r("9.999mF", "mF"),
    r("99.99mF", "mF"),
];

// Hz: the asset's eight rungs. The manual prints a single
// "99.99Hz~9.999MHz" row, which rungs 1 and 6 bound; rungs 0 and
// 7 are the asset's alone [UNVERIFIED]. The asset's labels carry
// a fifth digit ("9.9990Hz") that its own bounds (9.999) and the
// manual's span do not, and spell MHz "mHz"; the labels here
// follow the bounds.
static HZ: [RangeInfo; 8] = [
    r("9.999Hz", "Hz"),
    r("99.99Hz", "Hz"),
    r("999.9Hz", "Hz"),
    r("9.999kHz", "kHz"),
    r("99.99kHz", "kHz"),
    r("999.9kHz", "kHz"),
    r("9.999MHz", "MHz"),
    r("99.99MHz", "MHz"),
];

static DUTY_CYCLE: [RangeInfo; 1] = [r("Duty", "%")];

// The manual's spans; the asset's single rungs say the same.
static TEMP_C: [RangeInfo; 1] = [r("-40~1000°C", "°C")];
static TEMP_F: [RangeInfo; 1] = [r("-40~1832°F", "°F")];

static DIODE: [RangeInfo; 1] = [r("Diode", "V")];
static CONTINUITY: [RangeInfo; 1] = [r("Cont", "Ω")];
static DC_UA: [RangeInfo; 1] = [r("999.9µA", "µA")];
static AC_UA: [RangeInfo; 1] = [r("999.9µA", "µA")];

// The A mA position: past 999.9mA the meter reads on its 9.999A
// rung, same position and terminal (manual §X). The manual's
// further 10.00A row has no rung in the asset.
static DC_MA: [RangeInfo; 2] = [r("999.9mA", "mA"), r("9.999A", "A")];
static AC_MA: [RangeInfo; 2] = [r("999.9mA", "mA"), r("9.999A", "A")];

fn entry(mode: Mode) -> Option<&'static [RangeInfo]> {
    Some(match mode {
        Mode::DcV => &DC_V,
        Mode::AcV => &AC_V,
        Mode::DcMv => &DC_MV,
        Mode::AcMv => &AC_MV,
        Mode::Ohm => &OHM,
        Mode::Capacitance => &CAPACITANCE,
        Mode::Hz => &HZ,
        Mode::DutyCycle => &DUTY_CYCLE,
        Mode::TempC => &TEMP_C,
        Mode::TempF => &TEMP_F,
        Mode::Diode => &DIODE,
        Mode::Continuity => &CONTINUITY,
        Mode::DcUa => &DC_UA,
        Mode::AcUa => &AC_UA,
        Mode::DcMa => &DC_MA,
        Mode::AcMa => &AC_MA,
        // NCV has no ranges; the asset lists DC A and AC A without any
        // and the manual has no A position of their own.
        Mode::Ncv
        | Mode::DcA
        | Mode::AcA
        | Mode::Hfe
        | Mode::Live
        | Mode::LozV
        | Mode::ClampAcA
        | Mode::ClampDcA
        | Mode::LpfV
        | Mode::AcDcV
        | Mode::LpfA
        | Mode::AcDcA
        | Mode::ClampLpfA
        | Mode::ClampAcDcA
        | Mode::Inrush => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(mode: Mode) -> Vec<&'static str> {
        UT60BT.ranges(mode).iter().map(|r| r.label).collect()
    }

    #[test]
    fn model_name() {
        assert_eq!(UT60BT.name, "UNI-T UT60BT");
    }

    /// Range byte 0 on the V position is millivolts, as the asset says.
    #[test]
    fn volts_start_at_999_9_millivolts() {
        let t = &UT60BT;
        for mode in [Mode::DcV, Mode::AcV] {
            let r0 = t.range_info(mode, 0).unwrap();
            assert_eq!((r0.label, r0.unit), ("999.9mV", "mV"), "{mode:?}");
            assert_eq!(t.range_info(mode, 3).unwrap().label, "999.9V");
            assert!(t.range_info(mode, 4).is_none());
        }
        assert_eq!(labels(Mode::DcMv), ["9.999mV", "99.99mV"]);
    }

    /// The A rung is range byte 1 of the mA modes.
    #[test]
    fn milliamp_modes_reach_the_amp_rung() {
        let t = &UT60BT;
        for mode in [Mode::DcMa, Mode::AcMa] {
            assert_eq!(t.range_info(mode, 0).unwrap().unit, "mA");
            let r1 = t.range_info(mode, 1).unwrap();
            assert_eq!((r1.label, r1.unit), ("9.999A", "A"), "{mode:?}");
        }
        assert_eq!(labels(Mode::DcUa), ["999.9µA"]);
    }
}
