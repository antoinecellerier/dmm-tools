use super::{BASE, GAP, ModelTable, RangeInfo, r};
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::ut61eplus::capture::command_steps;
use crate::protocol::ut61eplus::mode::Mode;
use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

/// Where peak capture works: "P-MAX and P-MIN (Only for ACV/ACA)" in the
/// manual's button table (P8/13), and its AC V (P8/14) and AC A (P11/20)
/// instructions. AC A under both codes the table answers to. Which flag bits
/// the meter sets there is unconfirmed.
const PEAK_MODES: &[Mode] = &[Mode::AcV, Mode::AcA, Mode::ClampAcA];

/// Device table for the UNI-T UT202BT, a clamp meter with Bluetooth built in.
///
/// 9,999-count model with no dial: colour-coded buttons pick the function.
/// Which modes it has, how many rungs each and their range-byte order and
/// units come from UNI-T's iDMM2.0 app, which ships the table as a data
/// file. The labels are the UT202T/UT202BT manual's printed
/// ranges (spec tables, P14/26 to P17/31) where it prints the rung, the
/// asset's otherwise. No UT202BT has been connected; see
/// `docs/research/ut61-family/reverse-engineered-protocol.md` §9.
///
/// - V and clamp A: 9.999, 99.99 and 600.0, with LPF and inrush
/// - Resistance from 99.99Ω (7 ranges), capacitance to 99.9mF, temperature
/// - No mV, µA, mA, DC A, diode, duty cycle, hFE, REL or MAX/MIN
/// - Peak capture (P-MAX, P-MIN) in AC V and AC A only
///
/// AC A is listed under both 0x11 and 0x16, the two codes the app names
/// "ACA"; which one the meter sends is unknown. The app's LPF, LPF A and
/// °C rows start above range byte 0, and the missing bytes are [`GAP`]s.
///
/// The meter's secondary display (frequency in AC V and AC A, °F beside °C)
/// comes in frames of its own, read with this same table (family spec §2.3).
pub(crate) static UT202BT: ModelTable = ModelTable {
    name: "UNI-T UT202BT",
    entry,
    modes: &[
        Mode::AcV,
        Mode::DcV,
        Mode::Hz,
        Mode::Ohm,
        Mode::Continuity,
        Mode::Capacitance,
        Mode::TempC,
        Mode::TempF,
        Mode::AcA,
        Mode::Ncv,
        Mode::ClampAcA,
        Mode::LpfV,
        Mode::ClampLpfA,
        Mode::Inrush,
    ],
    peak_modes: PEAK_MODES,
    // Of the commands here, the only ones UNI-T's app sends this meter
    // (family spec §6.5). Its other bytes, 0x31-0x37, have no command here;
    // Peak stays off until 0x37 or 0x4D is known to start it.
    commands: &["hold", "range"],
    // Frequency beside AC V and AC A, °F beside °C (family spec §2.3).
    secondary_display: true,
    // UNI-T's app asks the name first, and a UT60BT is reported to need it
    // (family spec §6.4).
    name_before_stream: true,
    // No dial and colour-coded buttons: a list of its own.
    capture_steps: Some(capture_steps),
    ..BASE
};

static DC_V: [RangeInfo; 3] = [r("9.999V", "V"), r("99.99V", "V"), r("600.0V", "V")];
static AC_V: [RangeInfo; 3] = [r("9.999V", "V"), r("99.99V", "V"), r("600.0V", "V")];

// Listed at range byte 2 only, by the asset and the manual alike.
static LPF_V: [RangeInfo; 3] = [GAP, GAP, r("600.0V", "V")];

static AC_A: [RangeInfo; 3] = [r("9.999A", "A"), r("99.99A", "A"), r("600.0A", "A")];
static LPF_A: [RangeInfo; 3] = [GAP, GAP, r("600.0A", "A")];

// The manual's inrush table has 99.99A and 600A; rung 0 is the
// asset's alone. [UNVERIFIED]
static INRUSH: [RangeInfo; 3] = [r("9.999A", "A"), r("99.99A", "A"), r("600.0A", "A")];

// The UT202BT's own table (P16/29); the UT202T's starts at 999.9Ω.
static OHM: [RangeInfo; 7] = [
    r("99.99Ω", "Ω"),
    r("999.9Ω", "Ω"),
    r("9.999kΩ", "kΩ"),
    r("99.99kΩ", "kΩ"),
    r("999.9kΩ", "kΩ"),
    r("9.999MΩ", "MΩ"),
    r("99.99MΩ", "MΩ"),
];

// The top rung is printed "99.9mF" in both manuals; the asset
// says 105mF.
static CAPACITANCE: [RangeInfo; 7] = [
    r("99.99nF", "nF"),
    r("999.9nF", "nF"),
    r("9.999µF", "µF"),
    r("99.99µF", "µF"),
    r("999.9µF", "µF"),
    r("9.999mF", "mF"),
    r("99.9mF", "mF"),
];

// The asset's rungs, MHz for its "mHz". The manual prints no
// frequency table: frequency is on the auxiliary display in AC V
// and AC A. [UNVERIFIED]
static HZ: [RangeInfo; 6] = [
    r("99.99Hz", "Hz"),
    r("999.9Hz", "Hz"),
    r("9.999kHz", "kHz"),
    r("99.99kHz", "kHz"),
    r("999.9kHz", "kHz"),
    r("9.999MHz", "MHz"),
];

// The manual prints one 999.9Ω range; the asset lists it at
// range bytes 0 and 1.
static CONTINUITY: [RangeInfo; 2] = [r("999.9Ω", "Ω"), r("999.9Ω", "Ω")];

// The asset lists °C at range byte 1 and °F at byte 0; the
// labels are the manual's spans (P17/31).
static TEMP_C: [RangeInfo; 2] = [GAP, r("-40~1000°C", "°C")];
static TEMP_F: [RangeInfo; 1] = [r("-40~1832°F", "°F")];

fn entry(mode: Mode) -> Option<&'static [RangeInfo]> {
    Some(match mode {
        Mode::DcV => &DC_V,
        Mode::AcV => &AC_V,
        Mode::LpfV => &LPF_V,
        Mode::AcA | Mode::ClampAcA => &AC_A,
        Mode::ClampLpfA => &LPF_A,
        Mode::Inrush => &INRUSH,
        Mode::Ohm => &OHM,
        Mode::Capacitance => &CAPACITANCE,
        Mode::Hz => &HZ,
        Mode::Continuity => &CONTINUITY,
        Mode::TempC => &TEMP_C,
        Mode::TempF => &TEMP_F,
        // NCV has no ranges; the rest are not on this meter.
        Mode::Ncv
        | Mode::AcMv
        | Mode::DcMv
        | Mode::DutyCycle
        | Mode::Diode
        | Mode::DcUa
        | Mode::AcUa
        | Mode::DcMa
        | Mode::AcMa
        | Mode::DcA
        | Mode::Hfe
        | Mode::Live
        | Mode::LozV
        | Mode::ClampDcA
        | Mode::AcDcV
        | Mode::LpfA
        | Mode::AcDcA
        | Mode::ClampAcDcA => return None,
    })
}

/// The UT202BT's capture steps, from its manual's button table (P7/12,
/// P8/13): the red V button cycles AC V, AC V LPF and DC V; the yellow A
/// button AC A and AC A LPF, a long press inrush; the blue one Ω, continuity,
/// capacitance and temperature; NCV/PEAK is NCV, a long press peak capture.
/// Of the command steps only HOLD and RANGE are asked for: they are the only
/// ones UNI-T's app sends this meter (family spec §6.5). Nothing here has run
/// on a meter.
fn capture_steps() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Press the red V button until DC V shows. Leave leads open.",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm",
            "Press the blue \u{03A9} button until \u{03A9} shows. Leave leads open \
             (should show OL).",
        ),
    )
    .map(|s| s.samples(3));
    let [
        hold,
        hold_off,
        _rel,
        _rel_off,
        _minmax,
        _minmax_off,
        range,
        _auto,
    ] = command_steps(false);
    let step = |id, instruction| CaptureStep::basic(id, instruction).samples(3);
    vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm,
        ohm_body,
        ohm_short,
        step(
            "ohm_ranges",
            "Press the blue \u{03A9} button until \u{03A9} shows. Leads open or shorted, \
             either will do.",
        )
        .expect(Expect::mode("\u{03A9}")),
        step(
            "continuity",
            "Press the blue button once more for continuity (buzzer). Touch probes together.",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity").value(ValueExpect::Finite)),
        step(
            "capacitance",
            "Press the blue button once more for capacitance. Leave leads open.",
        )
        .expect(Expect::mode("Capacitance")),
        step(
            "temp",
            "Press the blue button once more for temperature (thermocouple).",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("\u{00B0}C")),
        step(
            "dcv_ranges",
            "Press the red V button until DC V shows. Leave leads open.",
        )
        .expect(Expect::mode("DC V")),
        hold,
        hold_off,
        range,
        step(
            "acv",
            "Press the red V button until AC V shows. Leave leads open.",
        )
        .expect(Expect::mode("AC V")),
        // What peak capture sets in the flag bytes is unknown on this meter.
        step("peak", "AC V: long-press NCV/PEAK to turn peak capture on."),
        step("peak_off", "Long-press NCV/PEAK again to turn it off."),
        step(
            "lpfv",
            "Press the red V button until AC V LPF shows. Leave leads open.",
        )
        .expect(Expect::mode("LPF V")),
        // The app names 0x11 and 0x16 alike, so which one AC A sends is open
        // and the step asserts neither.
        step(
            "aca",
            "Press the yellow A button until AC A shows. Leave the jaws empty.",
        ),
        step(
            "lpfa",
            "Press the yellow A button until AC A LPF shows. Leave the jaws empty.",
        )
        .expect(Expect::mode("Clamp LPF A")),
        step(
            "inrush",
            "AC A: long-press the yellow A button for inrush. Leave the jaws empty.",
        )
        .expect(Expect::mode("Inrush")),
        step(
            "ncv",
            "Short-press NCV/PEAK for NCV. Hold near a live wire.",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::tables::range_ladder;

    fn label(mode: Mode, range: u8) -> Option<&'static str> {
        UT202BT.range_info(mode, range).map(|r| r.label)
    }

    #[test]
    fn model_name() {
        assert_eq!(UT202BT.name, "UNI-T UT202BT");
    }

    /// LPF and °C exist only above range byte 0; the bytes below read as no
    /// range, and no ladder is offered over them.
    #[test]
    fn sparse_rows_read_only_their_listed_range_bytes() {
        let t = &UT202BT;
        for (mode, unit) in [(Mode::LpfV, "V"), (Mode::ClampLpfA, "A")] {
            assert_eq!(label(mode, 0), None, "{mode:?}");
            assert_eq!(label(mode, 1), None, "{mode:?}");
            assert_eq!(t.range_info(mode, 2).unwrap().unit, unit, "{mode:?}");
            assert!(range_ladder(t, mode).is_empty(), "{mode:?}");
        }
        assert_eq!(label(Mode::TempC, 0), None);
        assert_eq!(label(Mode::TempC, 1), Some("-40~1000°C"));
        assert_eq!(label(Mode::TempF, 0), Some("-40~1832°F"));
        assert!(range_ladder(t, Mode::TempC).is_empty());
    }

    /// Peak capture is AC V and AC A only (manual P8/13), so a frame with
    /// P-MAX or P-MIN set there is expected, not unrecognised.
    #[test]
    fn peak_is_ac_volts_and_amps_only() {
        let t = &UT202BT;
        for mode in [Mode::AcV, Mode::AcA, Mode::ClampAcA] {
            assert!(t.peak_modes.contains(&mode), "{mode:?}");
        }
        for mode in [Mode::DcV, Mode::LpfV, Mode::ClampLpfA, Mode::Inrush] {
            assert!(!t.peak_modes.contains(&mode), "{mode:?}");
        }
    }
}
