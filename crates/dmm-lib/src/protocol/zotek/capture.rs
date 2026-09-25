//! Capture steps per layout: what each layout's annunciators can show,
//! worded for the meter the layout is named for.
//!
//! Nothing here has run on a meter. The keys are the manuals'
//! (`docs/research/zotek/reverse-engineered-protocol.md` §1 lists them).

use super::layout::Layout;
use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

pub(super) fn steps(layout: &Layout) -> Vec<CaptureStep> {
    match layout.type_byte {
        3 => zt300ab(),
        4 => zt5566se(),
        1 => zt5bq(),
        2 => zt5b(),
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

/// The ZT-5566SE has a button per function rather than a dial: V~/⎓ Hz,
/// mV~/⎓ Hz, Ω, capacitance, diode/continuity and A~/⎓ mA~/⎓, then HOLD,
/// MAX/MIN, REL and VOL/RANGE, whose knob picks a range (ZT-5566SE manual
/// p.11-12). Its secondary display shows the frequency in V and the duty
/// cycle in Hz; the V button reaches frequency for AC above 36 V, the mV
/// button below it (p.11, p.21). MAX/MIN records both, and a long press
/// leaves it; no way to show MIN alone is documented (p.12). It has no
/// temperature, NCV or peak key, though the layout has a PEAK bit (spec
/// §7.4); its °C/°F button is for the knob's room temperature (p.13).
fn zt5566se() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", "Press the V~/⎓ Hz button until DC V shows"),
        Ohms::Word,
        CaptureStep::basic(
            "ohm_ol",
            "Press the Ω button with open leads (should show 0L)",
        ),
    );
    vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm_ol,
        ohm_body,
        ohm_short,
        CaptureStep::basic(
            "acv",
            "Press the V~/⎓ Hz button until AC V shows; the secondary display shows the frequency",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "hz",
            "Press the V~/⎓ Hz button until Hz shows; the secondary display shows the duty cycle",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "rel",
            "Press the V~/⎓ Hz button until DC V shows, then press REL, then Enter. Press \
             REL again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(REL_ON)),
        CaptureStep::basic(
            "max_min",
            "DC V: press MAX/MIN, then Enter. Long-press MAX/MIN afterwards.",
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
            "DC V: press VOL/RANGE and turn the knob one step, then Enter.",
        )
        .wait_for_enter(),
        CaptureStep::basic("dcmv", "Press the mV~/⎓ Hz button until DC mV shows")
            .expect(Expect::mode("DC mV")),
        CaptureStep::basic("acmv", "Press the mV~/⎓ Hz button until AC mV shows")
            .expect(Expect::mode("AC mV")),
        CaptureStep::basic(
            "hz_mv",
            "Press the mV~/⎓ Hz button until Hz shows (for AC below 36 V); the secondary \
             display shows the duty cycle",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "cont",
            "Press the diode/continuity button for continuity; touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "Press the diode/continuity button again for diode, leads open",
        )
        .expect(Expect::mode("Diode")),
        CaptureStep::basic("cap", "Press the capacitance button")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "dca",
            "Press the A~/⎓ mA~/⎓ button until DC A shows (leads in the A jack)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "Press the A~/⎓ mA~/⎓ button until AC A shows")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Press the A~/⎓ mA~/⎓ button until DC mA shows (leads in the mA jack)",
        )
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic("acma", "Press the A~/⎓ mA~/⎓ button until AC mA shows")
            .expect(Expect::mode("AC mA")),
    ]
}

/// The ZT-5BQ clamp is auto-ranging: with open leads it shows Auto, and it
/// takes V above 0.8 V and Ω or continuity by what the leads touch; its
/// panel legend lists continuity under AUTO (ZT-5BQ manual p.1/-1-, -4-,
/// p.2/-5-). Its jaw is the only current input, AC only (p.2/-5-).
/// Power/Select steps continuity/diode, then capacitance (p.2/-5-); the
/// temperature section says one press, then one more for °F (p.2/-6-), so
/// that step presses until °C shows. The manual never says how to get back
/// to Auto from these, so they run together and the Auto steps start "back
/// at Auto". Hz/NCV gives frequency, and held over 2 s NCV, which one page
/// ends on release and the other toggles (p.1/-4-, p.2/-6-); in NCV the
/// red probe tells the live wire from neutral (p.2/-6-), which no step asks
/// for: it means touching a mains conductor. The side button
/// gives HOLD, then INRUSH, and PEAK HOLD with the leads in, on one press
/// or two as the pages disagree (p.1/-4-, p.2/-6-). The layout's REL and %
/// have no documented key.
///
/// Shorted leads read as continuity and open leads as Auto, so the gate
/// keeps DC V, the reversed source and the body resistance, and takes its
/// OL from the diode with open leads.
fn zt5bq() -> Vec<CaptureStep> {
    let [dcv, _dcv_short, dcv_negative, _ohm_ol, ohm_body, _ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Hold the leads on a battery or any DC source above 0.8 V (DC V shows)",
        ),
        Ohms::Auto,
        CaptureStep::basic("ohm_ol", "unused"),
    );
    vec![
        dcv,
        dcv_negative,
        ohm_body,
        CaptureStep::basic(
            "diode_ol",
            "Press Power/Select once for continuity/diode, leads open (should show 0L)",
        )
        .gate()
        .expect(Expect::new().value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "Continuity/diode (Power/Select once from Auto): touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads]),
        CaptureStep::basic(
            "cap",
            "Press Power/Select once more for capacitance (twice from Auto), leads open",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "temp",
            "Press Power/Select until °C shows (temperature; K-type thermocouple, if available)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic(
            "temp_f",
            "Temperature: press Power/Select for °F (K-type thermocouple, if available)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°F")),
        CaptureStep::basic(
            "auto_idle",
            "Leads open, back at Auto: the display shows Auto",
        )
        .expect(Expect::new().value(ValueExpect::NoReading)),
        // The legend is the only source for this, and too coarse a print
        // to pin the annunciators on.
        CaptureStep::basic(
            "auto_cont",
            "At Auto: touch the probe tips together (continuity is picked by itself)",
        )
        .needs(&[Need::ShortedLeads]),
        CaptureStep::basic(
            "acv",
            "Hold the leads on a low-voltage AC source above 0.8 V, such as a transformer's \
             output (AC V shows)",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "hz",
            "Leads on the AC source: press Hz/NCV once for its frequency",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "ncv",
            "Leads off the source: hold Hz/NCV down over 2 s for NCV and keep holding it \
             near a live mains wire",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "aca",
            "Leads out: clamp the jaw around one wire carrying AC current",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "hz_a",
            "Jaw on the AC current, leads out: press Hz/NCV once for its frequency",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic("hold", "AC A: press the side HOLD button once, then Enter.")
            .wait_for_enter()
            .expect(Expect::new().flags(HOLD_ON)),
        CaptureStep::basic(
            "inrush",
            "Press the side HOLD button again for INRUSH, leads out (dashes until a motor \
             starts), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Inrush")),
        CaptureStep::basic(
            "peak",
            "Leads on a DC source above 0.8 V: press the side HOLD button until PEAK HOLD \
             shows, then Enter. Press it until PEAK HOLD clears afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V peak")),
    ]
}

/// The ZT-5B is auto-only: with open leads it shows Auto, and it takes V
/// above 0.8 V, Ω and current by what the leads touch (ZT-5B manual
/// p.1/-1-, -3-, -4-, p.2/-5-). SEL steps continuity/diode, capacitance,
/// frequency and temperature, one to four presses (p.1/-4-, p.2/-5-, -6-);
/// held, it gives NCV while held, never with a lead in the A mA jack
/// (p.1/-3-, p.2/-5-). H/ZERO holds, and in capacitance clears the reading
/// (p.1/-3-). The manual has no °F key, and never says how to get back to
/// Auto from a SEL mode, so the SEL steps run together and the Auto steps
/// start "back at Auto". The gate as the ZT-5BQ's, for the same reason.
/// No step checks the over-voltage warning (spec §7.3): community captures
/// have it from 180 V AC, and no step puts the leads on mains.
fn zt5b() -> Vec<CaptureStep> {
    let [dcv, _dcv_short, dcv_negative, _ohm_ol, ohm_body, _ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Hold the leads on a battery or any DC source above 0.8 V (DC V shows)",
        ),
        Ohms::Auto,
        CaptureStep::basic("ohm_ol", "unused"),
    );
    vec![
        dcv,
        dcv_negative,
        ohm_body,
        CaptureStep::basic(
            "diode_ol",
            "Press SEL once for continuity/diode, leads open (should show 0L)",
        )
        .gate()
        .expect(Expect::new().value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "Continuity/diode (SEL once from Auto): touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads]),
        CaptureStep::basic(
            "cap",
            "Press SEL once more for capacitance (twice from Auto), leads open",
        )
        .expect(Expect::mode("Capacitance")),
        // Whether the press also lights HOLD is not documented.
        CaptureStep::basic(
            "cap_zero",
            "Capacitance, leads open: press H/ZERO once (in capacitance it clears the \
             reading), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "hz",
            "Press SEL once more for frequency (three times from Auto), leads on a low-voltage \
             AC source",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "temp",
            "Press SEL once more for temperature (four times from Auto; K-type \
             thermocouple, if available)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic(
            "auto_idle",
            "Leads open, back at Auto: the display shows Auto",
        )
        .expect(Expect::new().value(ValueExpect::NoReading)),
        CaptureStep::basic(
            "acv",
            "Hold the leads on a low-voltage AC source above 0.8 V, such as a transformer's \
             output (AC V shows)",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "hold",
            "Leads on a DC source above 0.8 V (DC V shows): press H/ZERO once, then Enter. \
             Press it again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(HOLD_ON)),
        CaptureStep::basic(
            "ncv",
            "Leads out of the A mA jack: hold SEL/NCV down for NCV and keep holding it \
             near a live mains wire",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "dca",
            "Red lead in the A mA jack, in series with a battery and a load such as a \
             resistor or LED (DC A shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "aca",
            "Red lead in the A mA jack, in series with a low-voltage AC load such as a \
             transformer's output and a resistor (skip if none)",
        )
        .expect(Expect::mode("AC A")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::zotek::layout::LAYOUTS;

    /// Every layout has its steps, a gate among them, and none claims a
    /// hardware run.
    #[test]
    fn every_layout_has_steps_in_its_own_modes() {
        for layout in LAYOUTS {
            let steps = steps(layout);
            assert!(steps.iter().any(|s| s.gate), "{}", layout.id);
            assert!(steps.iter().all(|s| !s.verified), "{}", layout.id);
        }
    }
}
