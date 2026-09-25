//! Capture steps per layout: what each layout's annunciators can show,
//! worded for the meter the layout is named for.
//!
//! Nothing here has run on a meter. The keys are the manuals'
//! (`docs/research/zotek/reverse-engineered-protocol.md` §1 lists them).

use super::layout::Layout;
use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need};

pub(super) fn steps(layout: &Layout) -> Vec<CaptureStep> {
    match layout.type_byte {
        3 => zt300ab(),
        _ => Vec::new(),
    }
}

const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
const REL_ON: &[(Flag, bool)] = &[(Flag::Rel, true)];

/// The ZT-300AB's dial runs OFF, AUTO, V, mV, Ω, Hz %, A, mA, µA; SEL steps
/// AC/DC and each position's other functions, and a long press enters NCV
/// (ZT-300AB manual p.9, p.11-12). AUTO picks voltage, resistance or
/// continuity by what the leads touch (p.12, p.14). Hz % gives frequency,
/// then duty, on the V, mV and Hz % positions (p.10, p.18-19); held, it
/// switches Bluetooth, so no step holds it. MAX/MIN toggles MAX and MIN, and
/// a dial turn ends manual range (p.10). Temperature is SEL twice on mV, and
/// SEL again toggles °C/°F (p.19). Current goes in the 10A jack or the mAµA
/// one (p.13, p.15). The gate first, then the dial one way round.
fn zt300ab() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", "Set the dial to V and press SEL until DC shows"),
        Ohms::Word,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to Ω with open leads (should show 0L)",
        ),
    );
    vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm_ol,
        ohm_body,
        ohm_short,
        // The AUTO position shows Auto across the digits until it sees a
        // voltage or a resistance (manual p.12, p.14). Which annunciators
        // light beside it is undocumented, so only the value is checked.
        CaptureStep::basic(
            "auto_idle",
            "Set the dial to AUTO with open leads (the display shows Auto)",
        )
        .expect(Expect::new().value(crate::protocol::ValueExpect::NoReading)),
        CaptureStep::basic(
            "auto_dcv",
            "AUTO position: hold the leads on a battery or any DC source above 0.8 V \
             (DC V shows)",
        )
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "auto_ohm",
            "AUTO position: hold one probe tip between the fingers of each hand \
             (resistance shows)",
        )
        .expect(Expect::mode("Ω")),
        CaptureStep::basic(
            "auto_cont",
            "AUTO position: touch the probe tips together (continuity shows)",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        // REL, MAX/MIN, HOLD and RANGE each leave a state of their own, and
        // the next step's clean-up another, so these capture on Enter.
        CaptureStep::basic(
            "rel",
            "Set the dial to V and press SEL until DC shows, then press REL, then Enter. \
             Press REL again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(REL_ON)),
        // Which of MAX and MIN the first press lands on is not documented.
        CaptureStep::basic("max_min", "DC V: press MAX/MIN once, then Enter.").wait_for_enter(),
        CaptureStep::basic(
            "max_min_again",
            "DC V: press MAX/MIN again (it toggles MAX and MIN), then Enter. Hold MAX/MIN \
             for 2 s afterwards.",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "hold",
            "DC V: press HOLD, then Enter. Press HOLD again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press RANGE, then Enter. Turn the dial off V and back afterwards \
             (that ends manual range).",
        )
        .wait_for_enter(),
        CaptureStep::basic("acv", "V position: press SEL until AC shows")
            .expect(Expect::mode("AC V")),
        CaptureStep::basic("hz", "V position, AC: press Hz % for frequency")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic("duty", "V position: press Hz % again for duty cycle (%)")
            .expect(Expect::mode("Duty %")),
        CaptureStep::basic("dcmv", "Set the dial to mV and press SEL until DC shows")
            .expect(Expect::mode("DC mV")),
        CaptureStep::basic("acmv", "mV position: press SEL until AC shows")
            .expect(Expect::mode("AC mV")),
        CaptureStep::basic(
            "temp",
            "mV position: press SEL until °C shows (temperature; K-type thermocouple, \
             if available)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic(
            "temp_f",
            "Temperature: press SEL for °F (K-type thermocouple, if available)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°F")),
        CaptureStep::basic(
            "hz_mv",
            "mV position: press SEL until AC shows, then Hz % for frequency",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty_mv",
            "mV position: press Hz % again for duty cycle (%)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "cont",
            "Set the dial to Ω and press SEL for continuity; touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic("diode", "Ω position: press SEL again for diode, leads open")
            .expect(Expect::mode("Diode")),
        CaptureStep::basic("cap", "Ω position: press SEL again for capacitance")
            .expect(Expect::mode("Capacitance")),
        // The manual's own sequence for this position (p.18); it does not
        // say what the position shows before the presses.
        CaptureStep::basic(
            "hz_dial",
            "Set the dial to Hz %, then press SEL for AC V and Hz % for frequency",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic("duty_dial", "Hz % position: press Hz % for duty cycle (%)")
            .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "ncv",
            "Hold SEL/NCV for NCV, then hold the meter near a live mains wire",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(crate::protocol::ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "dca",
            "Set the dial to A, red lead in the 10A jack, and press SEL until DC shows",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A position: press SEL for AC").expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA, red lead in the mAµA jack, and press SEL until DC shows",
        )
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic("acma", "mA position: press SEL for AC").expect(Expect::mode("AC mA")),
        CaptureStep::basic(
            "dcua",
            "Set the dial to µA (red lead in the mAµA jack) and press SEL until DC shows",
        )
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic("acua", "µA position: press SEL for AC").expect(Expect::mode("AC µA")),
    ]
}
