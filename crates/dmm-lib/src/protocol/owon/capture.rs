//! Capture steps per model, each worded from its own manual.
//!
//! Nothing here has run on a meter. Each list is the gate, then the dial one
//! way round from its first position with the lead changes grouped, then the
//! remote keys the entry offers (spec §7.3, §10.8). A key leaves the meter
//! wherever it put it, which no manual step starts from, so the keys go last,
//! each starting from the mode it acts in.
//!
//! A long press of the meter's Bluetooth key — △/ᛒ on the B series,
//! Hz/Duty△/ᛒ on the OW16 and OW18, ZERO/ᛒ on the CM2100 (spec §7.1, §9.1),
//! Tab⇌ on the CMS, SETUP on the OW65B and < on the OW67B, OW69B and the
//! Voltcraft meters (spec §9.4) — switches Bluetooth, so every step that
//! presses it says to press it briefly, and the remote keys never send it
//! long.
//!
//! No step puts the leads on mains: the high-voltage, Motor, inrush and AC
//! power modes are captured with nothing connected, and only their live
//! reading is left out.
//!
//! Manual cites are `<manual> p.PDF/printed`, with the spec's short names:
//! B35-UM (35 Series & B41T), B33-UM, OW18-UM, OW16-UM, CM2100-UM,
//! CMS101-UM, OW65-UM, OW67-UM, OW69-UM; the Voltcraft manuals, VC871-UM,
//! VC891-UM, VC915-UM and VC925-UM, are cited `p.PDF`, their printed page.

use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

pub(super) fn steps(code: u8) -> Vec<CaptureStep> {
    let (mut steps, keys) = match code {
        18 => (
            ow18b(),
            ow_keys(
                "DC V: we will send ☀/H (H or HOLD should show). Press ☀/H briefly afterwards.",
            ),
        ),
        20 => (
            ow18e(),
            ow_keys("DC V: we will send ☀/H (HOLD should show). Press ☀/H briefly afterwards."),
        ),
        33 => (b33(), b33_keys()),
        35 => (b35t_plus(), b35_b41_keys()),
        41 => (b41t_plus(), b35_b41_keys()),
        21 => (cm2100b(), cm2100b_keys()),
        101 | 61 => (cms(), cms_keys()),
        65 => (ow65b(), ow65b_keys()),
        67 => (ow67b(), ow67b_keys()),
        69 => (ow69b(), ow69b_keys()),
        87 => (vc871(), vc871_keys()),
        89 => (vc891(), vc891_keys()),
        91 => (vc915(), vc915_keys()),
        92 => (vc925pv(), vc925pv_keys()),
        _ => return Vec::new(),
    };
    steps.extend(keys);
    steps
}

const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
const REL_ON: &[(Flag, bool)] = &[(Flag::Rel, true)];
const MANUAL_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, false)];
const AUTO_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, true)];
const MAX_ON: &[(Flag, bool)] = &[(Flag::Max, true), (Flag::Min, false)];
const MIN_ON: &[(Flag, bool)] = &[(Flag::Min, true), (Flag::Max, false)];
const MAX_MIN_OFF: &[(Flag, bool)] = &[(Flag::Max, false), (Flag::Min, false)];

/// A remote key step: the key goes out, five samples follow.
const fn key(id: &'static str, instruction: &'static str, command: &'static str) -> CaptureStep {
    CaptureStep::with_command(id, instruction, command, 5)
}

/// The B35T+'s dial runs OFF, ≈V, ≈mV, •)))⊣⊢→|Ω, Hz%, hFE, TEMP, µA≈, mA≈,
/// A≈ (B35-UM p.13/8-14/9). Select picks AC over the default DC, ℃ or ℉ on
/// TEMP, and on the combined position diode, continuity and capacitance on
/// one, two and three presses from resistance (p.14/9, p.19/14-22/17). The
/// thermocouple, the hFE socket and the µA and mA leads go in the mA TEMP
/// µA hFE jack, 20 A in the 20A jack (p.17/12-18/13). The ≈V steps and the
/// keys are the B41T+'s too: [`b35_b41_volts`], [`b35_b41_keys`].
fn b35t_plus() -> Vec<CaptureStep> {
    let mut steps = b35_b41_gate(CaptureStep::basic(
        "ohm_ol",
        "Set the dial to •)))⊣⊢→|Ω, leads open (Ω without the diode or continuity symbol or \
         F; should show OL)",
    ));
    steps.extend(b35_b41_volts());
    steps.extend([
        CaptureStep::basic(
            "diode",
            "Set the dial to •)))⊣⊢→|Ω and press Select once for diode (the diode symbol \
             shows), leads open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "•)))⊣⊢→|Ω position: press Select again for continuity (the continuity symbol \
             shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "cap",
            "•)))⊣⊢→|Ω position: press Select again for capacitance (F shows), leads open",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set the dial to Hz%, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press Hz/Duty for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "hfe",
            "Remove the test leads, set the dial to hFE, plug the multi-functional test \
             socket's + plug into mA TEMP µA hFE and its COM plug into COM, and seat a \
             transistor in it (skip if you have none)",
        )
        .needs(&[Need::Transistor])
        .expect(Expect::mode("hFE")),
        CaptureStep::basic(
            "temp_c",
            "Unplug the test socket or the test leads, set the dial to TEMP with a K-type \
             thermocouple's red connection in mA TEMP µA hFE and its black one in COM, and \
             press Select until ℃ shows (temperature)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic("temp_f", "Temperature: press Select until ℉ shows")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
        CaptureStep::basic(
            "dcua",
            "Unplug the thermocouple, set the dial to µA≈ with the test leads in COM and mA TEMP µA \
             hFE, \
             leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(b35_b41_current());
    steps
}

/// The B41T+ as the B35T+, from the same manual, but its dial has no hFE
/// and carries capacitance on a ⊣⊢ position of its own after Hz%, and its
/// Select cycles only resistance, diode and continuity (B35-UM p.14/9,
/// p.17/12-18/13). The capacitance procedure contradicts both, sending a
/// B41T(+) to ∘)))→|Ω (p.21/16), so that step names the ⊣⊢ position and
/// falls back to Select on ∘)))→|Ω. Its thermocouple and µA and mA leads go
/// in the mA µA TEMP jack (p.17/12).
fn b41t_plus() -> Vec<CaptureStep> {
    let mut steps = b35_b41_gate(CaptureStep::basic(
        "ohm_ol",
        "Set the dial to ∘)))→|Ω, leads open (Ω without the diode or continuity symbol; \
         should show OL)",
    ));
    steps.extend(b35_b41_volts());
    steps.extend([
        CaptureStep::basic(
            "diode",
            "Set the dial to ∘)))→|Ω and press Select once for diode (the diode symbol shows), \
             leads open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "∘)))→|Ω position: press Select again for continuity (the continuity symbol \
             shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic("hz", "Set the dial to Hz%, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press Hz/Duty for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "cap",
            "Set the dial to ⊣⊢, leads open (F shows; if the meter has no ⊣⊢ position, set it \
             to ∘)))→|Ω and press Select until F shows; the manual disagrees with itself \
             here)",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "temp_c",
            "Remove the test leads, set the dial to TEMP with a K-type thermocouple's red \
             connection in mA µA TEMP and its black one in COM, and press Select until ℃ \
             shows (temperature)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic("temp_f", "Temperature: press Select until ℉ shows")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
        CaptureStep::basic(
            "dcua",
            "Unplug the thermocouple, set the dial to µA≈ with the test leads in COM and mA µA \
             TEMP, \
             leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(b35_b41_current());
    steps
}

/// The B35T+ and B41T+ gate: ≈V defaults to DC (B35-UM p.19/14), and the
/// resistance position to resistance (p.19/14-20/15).
fn b35_b41_gate(ohm_ol: CaptureStep) -> Vec<CaptureStep> {
    steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ→|: set the dial to ≈V, leads open (DC shows)",
        ),
        Ohms::Symbol,
        ohm_ol,
    )
    .to_vec()
}

/// The ≈V and ≈mV positions of the B35T+ and B41T+, and the keys that act
/// there (B35-UM p.12/7, p.14/9, p.19/14, p.21/16, p.23/18-24/19): ☀/H
/// holds on a short press and lights the backlight on a long one; Range
/// held over 2 s goes back to auto; △/ᛒ is REL and forces manual range;
/// Max/Min cycles MAX and MIN, forces manual range, and held over 2 s
/// leaves; Hz/Duty on AC V cycles frequency, duty and back. Which of MAX
/// and MIN the first press shows is not stated, so the steps press until
/// each shows.
fn b35_b41_volts() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic(
            "hold",
            "Set the dial back to ≈V (DC shows), leads open: press ☀/H briefly (H shows; \
             holding it lights the backlight), then Enter. Press ☀/H briefly again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press Range once (AUTO goes off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DC V: hold Range for more than 2 seconds (AUTO shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "rel",
            "DC V: press △/ᛒ briefly (don't hold it: holding it is the Bluetooth key; △ \
             shows), then Enter. Press △/ᛒ briefly again afterwards, then, if AUTO is off, \
             hold Range for more than 2 seconds (AUTO shows).",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic("max", "DC V: press Max/Min until MAX shows, then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MAX_ON)),
        CaptureStep::basic("min", "DC V: press Max/Min until MIN shows, then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MIN_ON)),
        CaptureStep::basic(
            "minmax_exit",
            "DC V: hold Max/Min for more than 2 seconds (MAX and MIN go off), then Enter. \
             Afterwards, if AUTO is off, hold Range for more than 2 seconds (AUTO shows).",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        CaptureStep::basic(
            "acv",
            "≈V position: press Select for AC (AC shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_hz",
            "AC V: press Hz/Duty once for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acv_duty",
            "AC V: press Hz/Duty again for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        // The frame names no mV: the function is DC or AC V with the
        // milli prefix (spec §6.2-6.3).
        CaptureStep::basic("dcmv", "Set the dial to ≈mV, leads open (DC shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "≈mV position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC V")),
    ]
}

/// The B35T+ and B41T+ current steps after DC µA, whose jack differs:
/// µA≈, mA≈ and A≈ each default to DC, Select gives AC, and Hz/Duty on AC
/// current cycles frequency and duty (B35-UM p.21/16-22/17).
fn b35_b41_current() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("acua", "µA≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≈, red lead still in the same jack, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acma", "mA≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "acma_hz",
            "AC mA: press Hz/Duty once for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acma_duty",
            "AC mA: press Hz/Duty again for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "dca",
            "Set the dial to A≈ and move the red lead to 20A, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
    ]
}

/// The B35T+ and B41T+ entries' keys: Select, Range, a long Range (auto),
/// ☀/H, a long ☀/H (backlight), △/ᛒ (REL), Hz/Duty, Max/Min and a long
/// Max/Min (spec §7.3; B35-UM p.12/7, p.23/18). Which of MAX and MIN a press
/// shows is not stated, so that step expects nothing; the backlight is not
/// in the frame, so neither does the light step.
fn b35_b41_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to ≈V and move the red lead back to VΩ→|, leads open, DC showing \
             (press Select if AC shows): we will send Select (AC should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hz_duty",
            "AC V: we will send Hz/Duty (Hz should show). Press Hz/Duty twice afterwards to \
             get back to AC V, then Select for DC.",
            "hz_duty",
        )
        .expect(Expect::mode("Hz")),
        key(
            "key_hold",
            "DC V: we will send ☀/H (H should show). Press ☀/H briefly afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send a short △/ᛒ press (△ should show). Afterwards press △/ᛒ \
             briefly (don't hold it: holding it is the Bluetooth key), then, if AUTO is off, \
             hold Range for more than 2 seconds (AUTO shows).",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "DC V, AUTO showing: we will send Range (AUTO should go off).",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_auto",
            "DC V, AUTO off (press Range if it shows): we will send a long Range (AUTO should \
             show).",
            "auto",
        )
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        key(
            "key_minmax",
            "DC V: we will send Max/Min (MAX or MIN should show).",
            "minmax",
        ),
        key(
            "key_exit_minmax",
            "DC V, MAX or MIN showing (press Max/Min if neither does): we will send a long \
             Max/Min to leave it. Afterwards, if AUTO is off, hold Range for more than 2 \
             seconds (AUTO shows).",
            "exit_minmax",
        )
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        key(
            "key_light",
            "DC V: we will send a long ☀/H (the backlight, which lasts one minute).",
            "light",
        ),
    ]
}

/// The B33's dial runs OFF, ≈V, ∘)))→|Ω, ⊣⊢, Hz, ℃, µA≈, mA≈, A≈, OFF
/// (B33-UM p.12/7-13/8); it has no mV position, no ℉ and no MAX/MIN. Its
/// keys are Select, Range, Hz/Duty, Hold, ☀ and △/ᛒ (p.13/8-14/9). Select
/// picks AC over the default DC, and diode and continuity on one and two
/// presses from resistance (p.17/12-18/13); Hz/Duty toggles frequency and
/// duty on Hz and cycles frequency, duty and back on AC V and AC A
/// (p.19/14); Range held over 2 s goes back to auto (p.12/7); △/ᛒ is REL
/// and forces manual range (p.21/16). The thermocouple and the µA and mA
/// leads go in mA µA TEMP, up to 10 A in 10A (p.16/11).
fn b33() -> Vec<CaptureStep> {
    let mut steps = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ→|: set the dial to ≈V, leads open (DC shows)",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to ∘)))→|Ω, leads open (Ω without the diode or continuity symbol; \
             should show OL)",
        ),
    )
    .to_vec();
    steps.extend([
        CaptureStep::basic(
            "hold",
            "Set the dial back to ≈V (DC shows), leads open: press Hold (H shows), then \
             Enter. Press Hold again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press Range once (AUTO goes off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DC V: hold Range for more than 2 seconds (AUTO shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "rel",
            "DC V: press △/ᛒ briefly (holding it switches Bluetooth; △ shows), then Enter. \
             Press △/ᛒ briefly again afterwards, then, if AUTO is off, hold Range for more \
             than 2 seconds (AUTO shows).",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "acv",
            "≈V position: press Select for AC (AC shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_hz",
            "AC V: press Hz/Duty once for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acv_duty",
            "AC V: press Hz/Duty again for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "diode",
            "Set the dial to ∘)))→|Ω and press Select once for diode (the diode symbol shows), \
             leads open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "∘)))→|Ω position: press Select again for continuity (the continuity symbol \
             shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic("cap", "Set the dial to ⊣⊢, leads open (F shows)")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set the dial to Hz, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz position: press Hz/Duty for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "temp_c",
            "Remove the test leads, set the dial to ℃ with a K-type thermocouple's red \
             connection in mA µA TEMP and its black one in COM (temperature)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("°C")),
        CaptureStep::basic(
            "dcua",
            "Unplug the thermocouple, set the dial to µA≈ with the test leads in COM and mA µA \
             TEMP, \
             leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≈, red lead still in mA µA TEMP, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acma", "mA≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "acma_hz",
            "AC mA: press Hz/Duty once for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acma_duty",
            "AC mA: press Hz/Duty again for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "dca",
            "Set the dial to A≈ and move the red lead to 10A, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≈ position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// The B33 entry's keys: Select, Range, a long Range (auto), Hold, △/ᛒ (REL)
/// and Hz/Duty (spec §7.3). Its backlight is the ☀ key, which no program
/// sends, so there is no light step.
fn b33_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to ≈V and move the red lead back to VΩ→|, leads open, DC showing \
             (press Select if AC shows): we will send Select (AC should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hz_duty",
            "AC V: we will send Hz/Duty (Hz should show). Press Hz/Duty twice afterwards to \
             get back to AC V, then Select for DC.",
            "hz_duty",
        )
        .expect(Expect::mode("Hz")),
        key(
            "key_hold",
            "DC V: we will send Hold (H should show). Press Hold afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send a short △/ᛒ press (△ should show). Afterwards press △/ᛒ \
             briefly (holding it switches Bluetooth), then, if AUTO is off, hold Range for more \
             than 2 seconds (AUTO shows).",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "DC V, AUTO showing: we will send Range (AUTO should go off).",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_auto",
            "DC V, AUTO off (press Range if it shows): we will send a long Range (AUTO should \
             show).",
            "auto",
        )
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
    ]
}

/// The OW18B entry, which the OW16B shares (spec §1), worded from the OW18
/// manual with the OW16's differences marked. The OW18's dial runs OFF, ≈V,
/// ≈mV (only on units without hFE), Ω→|∘))), ⊣⊢, Hz%, ℃/℉, NCV, hFE (only on
/// some units), ≈µA, ≈mA, ≈A (OW18-UM p.13/8-14/9); the OW16's has no mV
/// and one position that is hFE or µA≈ by unit (OW16-UM p.13/8). Which
/// units carry hFE is not stated per model letter, so mV, hFE and µA steps
/// say "if the meter has it". On an OW18 the µA and mA leads go in µA mA and
/// 20 A in 20A (OW18-UM p.16/11); on an OW16 mA shares the VΩ jack and the
/// high-current jack is 10A (OW16-UM p.16/11, p.20/15). The thermocouple
/// goes in the VΩ jack on both. The ≈V steps, the gate, the NCV and
/// temperature steps and the keys are the OW18E's too: [`ow_gate`],
/// [`ow_volts`], [`ow_middle`], [`ow_keys`].
fn ow18b() -> Vec<CaptureStep> {
    let mut steps = ow_gate();
    steps.extend(ow_volts(
        "Set the dial back to ≈V (DC shows), leads open: press ☀/H briefly (H or HOLD shows; \
         holding it lights the backlight and, on an OW18B, the flashlight), then Enter. Press \
         ☀/H briefly again afterwards.",
    ));
    steps.push(
        CaptureStep::basic(
            "dcmv",
            "Set the dial to ≈mV if the meter has it (an OW18B without hFE; an OW16B has \
             none, skip then), leads open (DC shows)",
        )
        .expect(Expect::mode("DC V")),
    );
    steps.push(
        CaptureStep::basic(
            "acmv",
            "≈mV position, if the meter has it: press Select for AC (AC shows); skip otherwise",
        )
        .expect(Expect::mode("AC V")),
    );
    steps.extend(ow_middle());
    steps.extend([
        CaptureStep::basic(
            "dcua",
            "Set the dial to ≈µA (µA≈ on an OW16B) if the meter has it (an OW16B with hFE has \
             none, skip then) with the test leads in COM and µA mA (on an OW16B, the VΩ jack \
             marked mA), leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "acua",
            "≈µA position, if the meter has it: press Select for AC (AC shows); skip otherwise",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to ≈mA (mA≈ on an OW16B), red lead still in µA mA (on an OW16B, the \
             VΩ jack marked mA), leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(ow_ac_ma());
    steps.extend([
        CaptureStep::basic(
            "dca",
            "Set the dial to ≈A (A≈ on an OW16B) and move the red lead to 20A (10A on an \
             OW16B), leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "≈A position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// The OW18E, from the OW18 manual: the OW18B's dial and keys, with mV up
/// to 200 mV on units without hFE, and µA and mA up to 2000 µA and 200 mA
/// (OW18-UM p.13/8-14/9). Which OW18E units carry hFE is not stated, so the
/// mV and hFE steps say "if the meter has it". Leads as the OW18B's.
fn ow18e() -> Vec<CaptureStep> {
    let mut steps = ow_gate();
    steps.extend(ow_volts(
        "Set the dial back to ≈V (DC shows), leads open: press ☀/H briefly (HOLD shows; holding \
         it lights the backlight and the flashlight), then Enter. Press ☀/H briefly again \
         afterwards.",
    ));
    steps.push(
        CaptureStep::basic(
            "dcmv",
            "Set the dial to ≈mV if the meter has it (units without hFE; skip if not), leads \
             open (DC shows)",
        )
        .expect(Expect::mode("DC V")),
    );
    steps.push(
        CaptureStep::basic(
            "acmv",
            "≈mV position, if the meter has it: press Select for AC (AC shows); skip otherwise",
        )
        .expect(Expect::mode("AC V")),
    );
    steps.extend(ow_middle());
    steps.extend([
        CaptureStep::basic(
            "dcua",
            "Set the dial to ≈µA with the test leads in COM and µA mA, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "≈µA position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to ≈mA, red lead still in µA mA, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(ow_ac_ma());
    steps.extend([
        CaptureStep::basic(
            "dca",
            "Set the dial to ≈A and move the red lead to 20A, leads open (DC shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "≈A position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// The OW16 and OW18 gate: ≈V defaults to DC, Ω→|∘))) to resistance
/// (OW18-UM p.17/12).
fn ow_gate() -> Vec<CaptureStep> {
    steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ: set the dial to ≈V, leads open (DC shows)",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to Ω→|∘))), leads open (Ω without the continuity or diode symbol; \
             should show OL)",
        ),
    )
    .to_vec()
}

/// The ≈V position of the OW16 and OW18 and the keys that act there
/// (OW18-UM p.12/7, p.14/9, p.17/12, p.22/17; OW16-UM p.14/9, p.22/17):
/// ☀/H holds on a short press and lights the backlight (and the OW18's
/// flashlight) on a long one; Range held over 2 s goes back to auto.
/// Hz/Duty△/ᛒ is one key: REL, which forces manual range, outside AC and
/// frequency, and frequency, duty and back on AC V. The hold step's
/// wording is the entry's own: the OW18D/E display shows HOLD (OW18-UM
/// p.14/9-15/10), the Data Hold section a boxed H (p.22/17).
fn ow_volts(hold: &'static str) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("hold", hold)
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press Range once (AUTO goes off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DC V: hold Range for more than 2 seconds (AUTO shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "rel",
            "DC V: press Hz/Duty△/ᛒ briefly (don't hold it: holding it is the Bluetooth key; \
             REL shows), then Enter. Press Hz/Duty△/ᛒ briefly again afterwards, then, if AUTO \
             is off, hold Range for more than 2 seconds (AUTO shows).",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "acv",
            "≈V position: press Select for AC (AC shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_hz",
            "AC V: press Hz/Duty△/ᛒ briefly (don't hold it: holding it is the Bluetooth key) \
             for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acv_duty",
            "AC V: press Hz/Duty△/ᛒ briefly again (don't hold it: holding it is the Bluetooth \
             key) for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
    ]
}

/// The OW16 and OW18 positions between mV and the current ones: Select on
/// Ω→|∘))) gives continuity on one press and diode on two (OW18-UM
/// p.18/13); Hz/Duty△/ᛒ toggles frequency and duty on Hz% (p.19/14). How ℃
/// or ℉ is chosen on the ℃/℉ position is not stated in either manual, so
/// the temperature step expects no unit and the next asks for the other
/// unit only if the meter offers one. NCV lights the LED above the display
/// and beeps, near the top of the meter (p.19/14-20/15). The transistor's
/// leads go in the test holes on the panel (p.20/15; OW16-UM p.20/15).
fn ow_middle() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic(
            "cont",
            "Set the dial to Ω→|∘))) and press Select once for continuity (the continuity \
             symbol shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "Ω→|∘))) position: press Select again for diode (the diode symbol shows), leads \
             open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic("cap", "Set the dial to ⊣⊢, leads open (F shows)")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set the dial to Hz%, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press Hz/Duty△/ᛒ briefly (don't hold it: holding it is the \
             Bluetooth key) for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "temp",
            "Remove the test leads, set the dial to ℃/℉ with a K-type thermocouple's red \
             connection in the VΩ jack and its black one in COM (temperature; ℃ or ℉ shows)",
        )
        .needs(&[Need::Thermocouple]),
        CaptureStep::basic(
            "temp_unit",
            "Temperature: if the meter offers a way to show the other unit (the manual names \
             none), switch to it, then Enter. Skip if it offers none.",
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter(),
        CaptureStep::basic(
            "ncv_idle",
            "Unplug the thermocouple and set the dial to NCV, away from any \
             wiring",
        )
        .expect(Expect::mode("NCV")),
        CaptureStep::basic(
            "ncv",
            "NCV: hold the top of the meter very close to a live wire without touching it (the \
             LED above the display flashes and the meter beeps)",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "hfe",
            "Set the dial to hFE if the meter has it, and insert a transistor's emitter, base \
             and collector in the matching test holes on the panel (skip if the meter has no \
             hFE position or you have no transistor)",
        )
        .needs(&[Need::Transistor])
        .expect(Expect::mode("hFE")),
    ]
}

/// AC on the OW16 and OW18 mA position, where Hz/Duty△/ᛒ cycles frequency,
/// duty and back (OW18-UM p.21/16).
fn ow_ac_ma() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("acma", "≈mA position: press Select for AC (AC shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "acma_hz",
            "AC mA: press Hz/Duty△/ᛒ briefly (don't hold it: holding it is the Bluetooth key) \
             for frequency (Hz shows)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "acma_duty",
            "AC mA: press Hz/Duty△/ᛒ briefly again (don't hold it: holding it is the \
             Bluetooth key) for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
    ]
}

/// The OW18B and OW18E entries' keys: Select, Range, a long Range (auto),
/// ☀/H, a long ☀/H (backlight and flashlight; held 2 s again turns them off,
/// OW18-UM p.11/6), and the one Hz/Duty△/ᛒ key twice, as REL on DC V and as
/// Hz/Duty on AC V (spec §7.3). The backlight is not in the frame, so the
/// light step expects nothing. The hold step's wording is the entry's own,
/// as in [`ow_volts`].
fn ow_keys(hold: &'static str) -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to ≈V and move the red lead back to the VΩ jack, leads open, DC \
             showing (press Select if AC shows): we will send Select (AC should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hz_duty",
            "AC V: we will send a short Hz/Duty△/ᛒ press (Hz should show). Afterwards press \
             Hz/Duty△/ᛒ briefly twice (don't hold it: holding it is the Bluetooth key) to get \
             back to AC V, then Select for DC.",
            "hz_duty",
        )
        .expect(Expect::mode("Hz")),
        key("key_hold", hold, "hold").expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send a short Hz/Duty△/ᛒ press (REL should show). Afterwards press \
             Hz/Duty△/ᛒ briefly (don't hold it: holding it is the Bluetooth key), then, if \
             AUTO is off, hold Range for more than 2 seconds (AUTO shows).",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "DC V, AUTO showing: we will send Range (AUTO should go off).",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_auto",
            "DC V, AUTO off (press Range if it shows): we will send a long Range (AUTO should \
             show).",
            "auto",
        )
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        key(
            "key_light",
            "DC V: we will send a long ☀/H (the backlight, and the flashlight on an OW18). \
             Hold ☀/H for more than 2 seconds afterwards to switch them off.",
            "light",
        ),
    ]
}

/// The CM2100B's dial runs OFF, V≃, 2A≃, 20A≃, 100A≃, NCV, ⊣⊢Ω→|∘))), Hz%,
/// OFF; its keys are ZERO/ᛒ, HOLD (held about 2 s, the backlight) and
/// SELECT (held 2 s on AC, VFC) (CM2100-UM p.9/6-10/7). SELECT steps each
/// position's functions in an order the manual does not give, so the steps
/// press until the function shows. Current is the jaws only, chosen by the
/// dial, with the leads out (p.11/8-12/9; auto range excludes current,
/// p.33/30); ZERO zeroes DC A before a reading (p.11/8) and is relative for
/// voltage and capacitance (p.9/6), ZERO showing on the display (p.10/7).
/// NCV shows EF away from a field and one to four dashes near one, the
/// antenna 8-15 mm from it (p.13/10). VFC is held SELECT on AC voltage and
/// AC current (p.9/6-10/7); no step expects a mode for it, nor a flag for
/// ZERO: what the frame carries for either is not documented.
fn cm2100b() -> Vec<CaptureStep> {
    let mut steps = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and the red V→|Ω jack: set the dial to V≃ and press SELECT until DC \
             shows, leads open",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to ⊣⊢Ω→|∘))) and press SELECT until Ω shows without the diode or \
             continuity symbol or F, leads open (should show OL)",
        ),
    )
    .to_vec();
    steps.extend([
        CaptureStep::basic(
            "acv",
            "Set the dial back to V≃ and press SELECT until AC shows, leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "vfc",
            "AC V: hold SELECT for about 2 seconds until VFC shows, then Enter. Hold SELECT \
             for about 2 seconds again afterwards (VFC goes off).",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "hold",
            "V≃ position: press SELECT until DC shows, then press HOLD briefly (H shows; \
             holding it switches the backlight), then Enter. Press HOLD briefly again \
             afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "zero_v",
            "V≃ position, DC, leads open: press ZERO briefly for relative voltage (holding it \
             switches Bluetooth off; ZERO should show), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "dca_2a",
            "Remove the test leads and set the dial to 2A≃ with nothing in the jaws; press \
             SELECT until DC shows",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "zero",
            "2A≃ position, DC, nothing in the jaws: press ZERO briefly (holding it switches \
             Bluetooth off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "aca_2a",
            "2A≃ position: press SELECT until AC shows, nothing in the jaws",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "vfc_a",
            "AC A, nothing in the jaws: hold SELECT for about 2 seconds until VFC shows, then \
             Enter. Hold SELECT for about 2 seconds again afterwards (VFC goes off).",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "dca_20a",
            "Set the dial to 20A≃ and press SELECT until DC shows, nothing in the jaws",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "aca_20a",
            "20A≃ position: press SELECT until AC shows, nothing in the jaws",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dca_100a",
            "Set the dial to 100A≃ and press SELECT until DC shows, nothing in the jaws",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "aca_100a",
            "100A≃ position: press SELECT until AC shows, nothing in the jaws",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "ncv_ef",
            "Set the dial to NCV, test leads out, away from any wiring (EF shows)",
        )
        .expect(Expect::mode("NCV")),
        CaptureStep::basic(
            "ncv",
            "NCV: hold the tip of the clamp head about 8-15 mm from a live wire without \
             touching it (dashes show and the NCV indicator flashes)",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "diode",
            "Leads back in COM and V→|Ω: set the dial to ⊣⊢Ω→|∘))) and press SELECT until the \
             diode symbol shows, leads open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic(
            "cont",
            "⊣⊢Ω→|∘))) position: press SELECT until the continuity symbol shows; touch the \
             probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "cap",
            "⊣⊢Ω→|∘))) position: press SELECT until F shows, leads open",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "hz",
            "Set the dial to Hz% and press SELECT until Hz shows, leads open",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic("duty", "Hz% position: press SELECT until % shows")
            .expect(Expect::mode("Duty %")),
    ]);
    steps
}

/// The CM2100B entry's keys: SELECT, HOLD, a long HOLD (backlight) and ZERO
/// (spec §7.3). SELECT on V≃ picks ACV or DCV (CM2100-UM p.9/6), so from DC
/// it expects AC. What ZERO sets in the status word is not documented, so
/// its step expects only the mode.
fn cm2100b_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to V≃, leads open, DC showing (press SELECT if not): we will send \
             SELECT (AC should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hold",
            "V≃ position, DC showing (press SELECT if not): we will send HOLD (H should \
             show). Press HOLD briefly afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_light",
            "V≃ position: we will send a long HOLD (backlight). Hold HOLD for about 2 seconds \
             afterwards to switch the backlight off.",
            "light",
        ),
        key(
            "key_zero",
            "Remove the test leads, set the dial to 2A≃ and press SELECT until DC shows, \
             nothing in the jaws: we will send a short ZERO press.",
            "zero",
        )
        .expect(Expect::mode("DC A")),
    ]
}

const REL_OFF: &[(Flag, bool)] = &[(Flag::Rel, false)];
const LEAD_ERROR: &[(Flag, bool)] = &[(Flag::LeadError, true)];

/// The CMS101 and CMS061, from their manuals, the same text but for the
/// clamp, 1000 A against 600 A (CMS101-UM p.12/7). There is no dial: on the
/// first menu page F1 picks current (DC, AC), F2 voltage (DC and AC, V and
/// mV), F3 Res, Cont and Diode, and F4 Cap, Freq and NCV, each press the
/// next, the status bar naming the function (p.14/9-15/10, p.17/12-25/20).
/// Tab⇌, which carries BLE, steps the three menu pages; the second holds
/// MaxMin, Rel, Inrush and Hz/A on AC A, and Hz/V on AC V (p.14/9,
/// p.27/22). ▲ and ▼ go to manual range and A back to auto (p.11/6). Current
/// is the jaws only; HOLD held zeroes DC A (p.13/8, p.18/13). The power key
/// switches between the multimeter and the oscilloscope (p.13/8); whether
/// Bluetooth streams there is open (spec §9.4).
fn cms() -> Vec<CaptureStep> {
    let mut steps = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and the V/Ω jack: press F2 until the status bar shows V,DC (not \
             mV,DC), leads open",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Press F3 until the status bar shows Res, leads open (should show OL)",
        ),
    )
    .to_vec();
    steps.extend([
        CaptureStep::basic(
            "hold",
            "Press F2 until V,DC shows, leads open: press HOLD briefly (HOLD shows; holding it \
             zeroes DC A), then Enter. Press HOLD briefly again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press ▲ once (Manu shows in place of Auto), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic("range_auto", "DC V: press A (Auto shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "rel",
            "DC V: press Tab⇌ briefly for the second menu page, then F2 (Rel; △ shows), then \
             Enter. Press F2 again afterwards, then A if Manu shows.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "maxmin",
            "DC V, second menu page: press F1 (MaxMin; Min, Max and Avg show under the \
             reading), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "maxmin_exit",
            "DC V: press F1 (MaxMin) again to leave it (Auto shows again), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "dcmv",
            "Press Tab⇌ briefly until the first menu page shows, then F2 until mV,DC shows, \
             leads open",
        )
        .expect(Expect::mode("DC V")),
        CaptureStep::basic("acv", "Press F2 until V,AC shows, leads open")
            .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_hz",
            "AC V: press Tab⇌ briefly for the second menu page, then F4 (Hz/V) for the \
             frequency, then Enter. Press F4 again afterwards.",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "acmv",
            "Press Tab⇌ briefly until the first menu page shows, then F2 until mV,AC shows",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "cont",
            "Press F3 until Cont shows; touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic("diode", "Press F3 until Diode shows, leads open (OL shows)")
            .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic("cap", "Press F4 until Cap shows, leads open")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Press F4 until Freq shows, leads open")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Freq: press Tab⇌ briefly for the second menu page, then F4 (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "ncv_ef",
            "Press Tab⇌ briefly until the first menu page shows, then F4 until NCV shows; \
             remove the test leads and keep the clamp away from any wiring (EF shows)",
        )
        .expect(Expect::mode("NCV")),
        CaptureStep::basic(
            "ncv",
            "NCV: bring the front end of the clamp head close to a live conductor without \
             touching it (dashes show, the NCV LED lights and the buzzer sounds)",
        )
        .needs(&[Need::LiveWire])
        .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        CaptureStep::basic("dca", "Press F1 until A,DC shows, nothing in the jaws")
            .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "dca_zero",
            "DC A, nothing in the jaws: hold HOLD to zero DC A, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "Press F1 until A,AC shows, nothing in the jaws")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "aca_hz",
            "AC A, nothing in the jaws: press Tab⇌ briefly for the second menu page, then F4 \
             (Hz/A), then Enter. Press F4 again afterwards.",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "inrush",
            "AC A, second menu page, nothing in the jaws: press F3 (Inrush; INRUSH shows and \
             the meter waits for a current), then Enter. Press F3 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Inrush")),
        CaptureStep::basic(
            "scope",
            "Leads open: press the power key briefly to switch to the oscilloscope (holding it \
             switches the meter off), then Enter. This step records whether readings keep \
             coming; none is fine. Press the power key briefly again afterwards.",
        )
        .wait_for_enter(),
    ]);
    steps
}

/// The CMS entries' keys: Select, Range, Hold, Rel, Hz/Duty, Max/Min and
/// Inrush, all taps (spec §10.8). The CMS has no SELECT key and no key
/// named Hz/Duty, so those steps expect nothing.
fn cms_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Press the power key briefly if the oscilloscope shows, Tab⇌ briefly until the \
             first menu page shows, then F2 until V,DC shows, leads open: we will send Select, \
             which the CMS has no key for.",
            "select",
        ),
        key(
            "key_hold",
            "V,DC: we will send Hold (HOLD should show). Press HOLD briefly afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "V,DC: we will send Rel (△ should show). Afterwards press Tab⇌ briefly for the \
             second menu page, F2 (Rel) to leave it, then A if Manu shows.",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "V,DC, Auto showing: we will send Range (Manu should show). Press A afterwards.",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_minmax",
            "V,DC: we will send Max/Min (Min, Max and Avg should show). Afterwards press Tab⇌ \
             briefly for the second menu page and F1 (MaxMin) to leave it.",
            "minmax",
        ),
        key(
            "key_hz_duty",
            "Press Tab⇌ briefly until the first menu page shows, then F4 until Freq shows, \
             leads open: we will send Hz/Duty. If % shows afterwards, press Tab⇌ briefly for \
             the second menu page and F4.",
            "hz_duty",
        ),
        key(
            "key_inrush",
            "Press Tab⇌ briefly until the first menu page shows, then F1 until A,AC shows, \
             nothing in the jaws: we will send Inrush (INRUSH should show). Afterwards press \
             Tab⇌ briefly for the second menu page and F3 (Inrush) to leave it.",
            "inrush",
        ),
    ]
}

/// The OW65B's dial runs OFF, V≅, mV≅, →|·))Ω, ⊣⊢, Hz%, °C°F, µA≅, mA≅,
/// A≅, OFF; F1-F4 are RANGE, MAX/MIN, REL and HOLD, beside SELECT, SETUP
/// (BLE above it), the torch key and LoZ (OW65-UM p.13/8-14/9). SELECT gives
/// AC over the default DC, continuity and diode on one and two presses from
/// resistance, and duty on Hz% (p.19/14-24/19), the display showing ⎓ or ~
/// (p.14/9). The manual names no way back to auto range or out of MAX/MIN
/// (p.26/21), and no ℃/℉ choice (p.24/19), so those steps turn the dial or
/// ask. Its 4-20 mA note names < and > keys this panel lacks (p.25/20). µA
/// and mA go in mAµA, up to 10 A in 10A (p.24/19-25/20). MAX/MIN shows the
/// live value and REL the reference at the top left (p.26/21-27/22).
fn ow65b() -> Vec<CaptureStep> {
    let mut steps = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ: set the dial to V≅, leads open (⎓ shows)",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to →|·))Ω, leads open (Ω without the continuity or diode symbol; \
             should show OL)",
        ),
    )
    .to_vec();
    steps.extend([
        CaptureStep::basic(
            "hold",
            "Set the dial back to V≅ (⎓ shows), leads open: press F4 (HOLD; HOLD shows at the \
             top right), then Enter. Press F4 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press F1 (RANGE) once for manual range, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DC V: turn the dial to mV≅ and back to V≅ (the manual names no key back to auto \
             range), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "rel",
            "DC V: press F3 (REL; Rel shows at the top left, with the reference), then Enter. \
             Press F3 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "max",
            "DC V: press F2 (MAX/MIN) once (MAX and the live reading show at the top left), \
             then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_ON)),
        CaptureStep::basic("min", "DC V: press F2 again (MIN shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MIN_ON)),
        CaptureStep::basic(
            "minmax_exit",
            "DC V: turn the dial to mV≅ and back to V≅ to leave MAX/MIN (the manual names no \
             other way), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        CaptureStep::basic(
            "loz",
            "DC V, leads open: hold LoZ (no more than 3 seconds; Loz shows) and press Enter \
             while holding it.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(LOZ_ON)),
        CaptureStep::basic(
            "acv",
            "V≅ position: press SELECT for AC (~ shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(ow6x_middle(
        "Remove the test leads, set the dial to °C°F with a K-type thermocouple's red \
         connection in VΩ and its black one in COM (temperature; ℃ or ℉ shows)",
    ));
    steps.extend([
        CaptureStep::basic(
            "dcua",
            "Unplug the thermocouple, set the dial to µA≅ with the test leads in COM and mAµA, \
             leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≅, red lead still in mAµA, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acma", "mA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "loop",
            "mA≅ position, leads open: press SELECT for DC, then switch to 4-20 mA if the \
             meter offers a way (the manual says < or >, keys the OW65 lacks), then Enter. \
             Skip if it offers none.",
        )
        .wait_for_enter()
        .expect(Expect::mode("4-20mA")),
        CaptureStep::basic(
            "dca",
            "Set the dial to A≅ and move the red lead to 10A, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// The OW65B, OW67B and OW69B positions from →|·))Ω to °C°F, all three
/// manuals the same (OW65-UM p.20/15-24/19; OW67-UM p.21/16-25/20; OW69-UM
/// p.20/15-24/19): SELECT gives continuity on one press and diode on two,
/// and duty on Hz%; none says how ℃ or ℉ is chosen. `temp` is the model's
/// thermocouple step, whose jack differs.
fn ow6x_middle(temp: &'static str) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic(
            "cont",
            "Set the dial to →|·))Ω and press SELECT once for continuity (the continuity \
             symbol shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "→|·))Ω position: press SELECT again for diode (the diode symbol shows), leads open \
             (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic("cap", "Set the dial to ⊣⊢, leads open")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set the dial to Hz%, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press SELECT for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
        CaptureStep::basic("temp", temp).needs(&[Need::Thermocouple]),
        CaptureStep::basic(
            "temp_unit",
            "Temperature: if the meter offers a way to show the other unit (the manual names \
             none), switch to it, then Enter. Skip if it offers none.",
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter(),
    ]
}

/// The OW65B entry's keys: Select, Range, Hold, Rel and Max/Min, all taps
/// (spec §10.8); a first MAX/MIN press shows MAX (OW65-UM p.26/21).
fn ow65b_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to V≅ and move the red lead back to VΩ, leads open, ⎓ showing (press \
             SELECT if ~ shows): we will send Select (~ should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hold",
            "Press SELECT for DC: we will send Hold (HOLD should show). Press F4 afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send Rel (Rel should show). Press F3 afterwards.",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "DC V: we will send Range (manual range). Afterwards turn the dial to mV≅ and back \
             to V≅.",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_minmax",
            "DC V: we will send Max/Min (MAX should show). Afterwards turn the dial to mV≅ and \
             back to V≅.",
            "minmax",
        )
        .expect(Expect::mode("DC V").flags(MAX_ON)),
    ]
}

/// The OW67B, from its manual: the OW65B's dial and keys with a W cosφ
/// position in place of the last OFF, and the menu on F1-F4 paged by < and >
/// (< carries BLE): page 1 RANGE, MAX/MIN, REL, HOLD, or on AC V, reached
/// by < or >, Lo, PEAK and FREQ (OW67-UM p.13/8-14/9, p.29/24-32/27). PEAK
/// shows MAX, then Min on a second F2 press, and leaves on F2 held; FREQ
/// swaps V and Hz between the main and sub-display; Lo is not explained,
/// the VC871's manual calling it a 1 kHz low-pass (p.32/27). < or > gives
/// 4-20 mA on mA (p.26/21). On W cosφ, AC power is the default, SELECT gives
/// DC power and twice USB power, through optional modules for those two; F1
/// cycles the displays, or toggles V and W, and F4 times USB charge
/// (p.26/21-28/23). As on the OW65B, no way back to auto range or out of
/// MAX/MIN is named.
fn ow67b() -> Vec<CaptureStep> {
    let mut steps = ow6x_gate();
    steps.extend(ow6x_volts(OW6X_HOLD));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅ position: press SELECT for AC (~ shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acv_freq", OW6X_ACV_FREQ).wait_for_enter(),
        CaptureStep::basic("acv_peak", OW6X_ACV_PEAK)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic("acv_peak_min", OW6X_ACV_PEAK_MIN)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic(
            "acv_lpf",
            "AC V, Lo, PEAK and FREQ over F1-F3: press F1 (Lo; the VC871's manual calls it a \
             1 kHz low-pass filter), then Enter. Press F1 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("LPF AC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(ow6x_middle(
        "Remove the test leads, set the dial to °C°F with a K-type thermocouple's red \
         connection in VΩ and its black one in COM (temperature; ℃ or ℉ shows)",
    ));
    steps.extend(ow6x_current());
    steps.extend(power_steps(
        "Remove the test leads and set the dial to W cosφ, nothing in the input jacks (AC \
         power: W and VA show)",
        "AC power: press F1 (Display) once (V and A show)",
        "AC power: press F1 (Display) again (PF and Hz show)",
        "W cosφ position: press SELECT once for DC power (⎓ shows), nothing in the input jacks",
        "DC power: plug the DC power measurement module into 10A, COM and VΩ, a DC supply on \
         its INPUT and a load on its OUTPUT, and switch both on (W and A show)",
        "Unplug the module, then press SELECT again (twice from AC power) for USB power, \
         nothing in the input jacks",
        "USB power: plug the USB power measurement module into 10A, COM and VΩ, a USB \
         charger on its INPUT and a device to charge on its OUTPUT (W and A show)",
    ));
    steps
}

/// The OW67B and OW69B gate: V≅ defaults to DC, →|·))Ω to resistance
/// (OW67-UM p.20/15-21/16; OW69-UM p.19/14-20/15).
fn ow6x_gate() -> Vec<CaptureStep> {
    steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ: set the dial to V≅, leads open (⎓ shows)",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to →|·))Ω, leads open (Ω without the continuity or diode symbol; \
             should show OL)",
        ),
    )
    .to_vec()
}

/// The OW67B's and OW69B's hold step, the same in both manuals (OW67-UM
/// p.30/25; OW69-UM p.27/22).
const OW6X_HOLD: &str = "Set the dial back to V≅ (⎓ shows), leads open: press < or > \
     briefly until the first menu page (RANGE, MAX/MIN, REL, HOLD) shows, then F4 (HOLD; HOLD \
     shows at the top right), then Enter. Press F4 again afterwards.";

/// The OW67B's and OW69B's AC V menu steps: < or > to the page holding Lo,
/// PEAK and FREQ, which SELECT does not land on; PEAK shows MAX, a second F2
/// press Min, and F2 held leaves it (OW67-UM p.32/27; OW69-UM p.29/24).
const OW6X_ACV_FREQ: &str = "AC V: press < or > briefly until Lo, PEAK and FREQ show over \
     F1-F3, then F3 (FREQ; V and Hz swap between the main and sub-display), then Enter. Press \
     F3 again afterwards.";
const OW6X_ACV_PEAK: &str = "AC V, Lo, PEAK and FREQ over F1-F3: press F2 (PEAK; MAX and the \
     live reading show at the top left), then Enter.";
const OW6X_ACV_PEAK_MIN: &str = "AC V, PEAK on: press F2 again (Min shows), then Enter. Hold F2 \
     afterwards to leave PEAK.";

/// The 15-byte meters' LoZ step: LoZ held, which the status word's bit 8
/// shows (spec §10.6).
const LOZ_ON: &[(Flag, bool)] = &[(Flag::LoZ, true)];

/// The OW67B's and OW69B's V≅ steps on the first menu page (OW67-UM
/// p.29/24-30/25; OW69-UM p.26/21-27/22): RANGE, MAX/MIN, REL and HOLD on
/// F1-F4, and LoZ held.
fn ow6x_volts(hold: &'static str) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("hold", hold)
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press F1 (RANGE) once for manual range, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DC V: turn the dial to mV≅ and back to V≅ (the manual names no key back to auto \
             range), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "rel",
            "DC V: press < or > briefly until the first menu page shows, then F3 (REL; Rel and \
             the reference show at the top left), then Enter. Press F3 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "max",
            "DC V: press F2 (MAX/MIN) once (MAX and the live reading show at the top left), \
             then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_ON)),
        CaptureStep::basic("min", "DC V: press F2 again (MIN shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MIN_ON)),
        CaptureStep::basic(
            "minmax_exit",
            "DC V: turn the dial to mV≅ and back to V≅ to leave MAX/MIN (the manual names no \
             other way), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        CaptureStep::basic(
            "loz",
            "DC V, leads open: hold LoZ (no more than 3 seconds; Loz shows) and press Enter \
             while holding it.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(LOZ_ON)),
    ]
}

/// The OW67B's and OW69B's current positions (OW67-UM p.25/20-26/21;
/// OW69-UM p.24/19-25/20): DC by default, SELECT for AC, µA and mA in mAµA,
/// up to 10 A in 10A, and < or > for 4-20 mA on mA.
fn ow6x_current() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic(
            "dcua",
            "Unplug the thermocouple, set the dial to µA≅ with the test leads in COM and mAµA, \
             leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≅, red lead still in mAµA, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acma", "mA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "loop",
            "mA≅ position, leads open: press SELECT for DC, then < or > briefly to switch to \
             4-20 mA, then Enter. Afterwards press < or > briefly to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("4-20mA")),
        CaptureStep::basic(
            "dca",
            "Set the dial to A≅ and move the red lead to 10A, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
    ]
}

/// The power steps of the OW67B and the VC871, worded from each manual: AC
/// power with nothing connected and its two other displays, then DC and
/// USB power, each first with nothing connected and then through its
/// optional module, F1 toggling V and W (OW67-UM p.28/23; VC871-UM p.86,
/// p.88). The VC871's captures settle AC power's V and PF functions and the
/// DC power context (spec §14.3 D11); what USB power's W display sends is
/// open, so it expects no mode.
fn power_steps(
    ac: &'static str,
    ac_v: &'static str,
    ac_pf: &'static str,
    dc: &'static str,
    dc_module: &'static str,
    usb: &'static str,
    usb_module: &'static str,
) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("power_ac", ac).expect(Expect::mode("AC power W")),
        CaptureStep::basic("power_ac_v", ac_v).expect(Expect::mode("AC power V")),
        CaptureStep::basic("power_ac_pf", ac_pf).expect(Expect::mode("Power factor")),
        CaptureStep::basic("power_dc", dc).expect(Expect::mode("DC power W")),
        CaptureStep::basic("power_dc_module", dc_module)
            .needs(&[Need::PowerModule])
            .expect(Expect::mode("DC power W")),
        CaptureStep::basic(
            "power_dc_v",
            "DC power, module still connected: press F1 (V shows in place of W), then Enter. \
             Press F1 again afterwards.",
        )
        .needs(&[Need::PowerModule])
        .wait_for_enter()
        .expect(Expect::mode("DC power V")),
        CaptureStep::basic("power_usb", usb),
        CaptureStep::basic("power_usb_module", usb_module).needs(&[Need::PowerModule]),
        CaptureStep::basic(
            "power_usb_v",
            "USB power, module still connected: press F1 (V shows in place of W), then Enter. \
             Press F1 again afterwards.",
        )
        .needs(&[Need::PowerModule])
        .wait_for_enter()
        .expect(Expect::mode("USB power V")),
        CaptureStep::basic(
            "usb_timer",
            "USB power, module still connected: press F4 to start counting elapsed time (the \
             totals show), then Enter. Press F4 again afterwards.",
        )
        .needs(&[Need::PowerModule])
        .wait_for_enter(),
    ]
}

/// The OW67B entry's keys: Select, Range, Hold, Rel, Max/Min, Peak, 4~20mA
/// and Display, all taps (spec §10.8). What Peak, 4~20mA and Display do as
/// remote keys is not stated, so they expect nothing.
fn ow67b_keys() -> Vec<CaptureStep> {
    let mut steps = ow6x_keys();
    steps.extend([
        key(
            "key_peak",
            "V≅ position, ~ showing (press SELECT if ⎓ shows): we will send Peak. Afterwards \
             press < or > briefly until the first menu page shows, and hold F2 if PEAK is on.",
            "peak",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards press < or > briefly to leave 4-20 mA if it shows.",
            "current_loop",
        ),
        key(
            "key_display",
            "Remove the test leads and set the dial to W cosφ, nothing in the input jacks, AC \
             power showing: we will send Display.",
            "display",
        ),
    ]);
    steps
}

/// The OW67B's and OW69B's V≅ keys: Select, Hold, Rel, Range and Max/Min
/// (spec §10.8), the first MAX/MIN press showing MAX (OW67-UM p.29/24;
/// OW69-UM p.26/21).
fn ow6x_keys() -> Vec<CaptureStep> {
    vec![
        key(
            "key_select",
            "Set the dial to V≅ with the leads in COM and VΩ, leads open, ⎓ showing (press \
             SELECT if ~ shows): we will send Select (~ should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        key(
            "key_hold",
            "Press SELECT for DC: we will send Hold (HOLD should show). Press F4 on the first \
             menu page afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send Rel (Rel should show). Press F3 on the first menu page \
             afterwards.",
            "rel",
        )
        .expect(Expect::mode("DC V").flags(REL_ON)),
        key(
            "key_range",
            "DC V: we will send Range (manual range). Afterwards turn the dial to mV≅ and back \
             to V≅.",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_minmax",
            "DC V: we will send Max/Min (MAX should show). Afterwards turn the dial to mV≅ and \
             back to V≅.",
            "minmax",
        )
        .expect(Expect::mode("DC V").flags(MAX_ON)),
    ]
}

/// The OW69B, from its manual: the OW67B's dial with OFF in place of W
/// cosφ, and V≅ also AC+DC, whose SELECT order is not stated (OW69-UM
/// p.13/8-14/9). Lo is a low-pass filter on AC V, F1 on the page < or >
/// reaches, PEAK F2 and FREQ F3 as on the OW67B (p.28/23-29/24).
fn ow69b() -> Vec<CaptureStep> {
    let mut steps = ow6x_gate();
    steps.extend(ow6x_volts(OW6X_HOLD));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅ position: press SELECT until ~ shows without AC+DC, leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acv_freq", OW6X_ACV_FREQ).wait_for_enter(),
        CaptureStep::basic("acv_peak", OW6X_ACV_PEAK)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic("acv_peak_min", OW6X_ACV_PEAK_MIN)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic(
            "acv_lpf",
            "AC V, Lo, PEAK and FREQ over F1-F3: press F1 (Lo, the low-pass filter; Lo shows at \
             the bottom left), then Enter. Press F1 again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("LPF AC V")),
        CaptureStep::basic(
            "acdcv",
            "V≅ position: press SELECT until AC+DC shows, leads open",
        )
        .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(ow6x_middle(
        "Remove the test leads, set the dial to °C°F with a K-type thermocouple's red \
         connection in VΩ and its black one in COM (temperature; ℃ or ℉ shows)",
    ));
    steps.extend(ow6x_current());
    steps
}

/// The OW69B entry's keys: Select, Range, Hold, Rel, Hz/Duty, Max/Min, LPF,
/// Peak and 4~20mA, all taps (spec §10.8). Hz/Duty, LPF, Peak and 4~20mA
/// expect nothing: what they do as remote keys is not stated.
fn ow69b_keys() -> Vec<CaptureStep> {
    let mut steps = ow6x_keys();
    steps.extend([
        key(
            "key_hz_duty",
            "Set the dial to Hz%, leads open, Hz showing: we will send Hz/Duty. Afterwards \
             press SELECT if % shows.",
            "hz_duty",
        ),
        key(
            "key_lpf",
            "Set the dial to V≅, ~ showing (press SELECT until it does), leads open: we will \
             send LPF. Afterwards press < or > briefly until the first menu page shows, and F1 \
             if Lo shows.",
            "lpf",
        ),
        key(
            "key_peak",
            "AC V: we will send Peak. Afterwards press < or > briefly until the first menu page \
             shows, and hold F2 if PEAK is on.",
            "peak",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards press < or > briefly to leave 4-20 mA if it shows.",
            "current_loop",
        ),
    ]);
    steps
}

/// The VC871, from its manual, the OW67B's twin by panel and menus (spec
/// §9.4) but worded by Voltcraft: its dial runs OFF, V≅, mV≅, →|·))Ω, ⊣⊢,
/// Hz%, °C°F, µA≅, mA≅, A≅, W cosφ, and RANGE, MAX/MIN, REL and HOLD are
/// labels over F1-F4, paged by < (BLE) and > (VC871-UM p.62, p.69-71). SELECT
/// gives AC over DC, continuity and diode on one and two presses from Ω, %
/// on Hz, ℉ on °C (p.73-82). RANGE held about 1 s goes back to auto, REL
/// held leaves REL; MAX/MIN held "activates MAX/MIN again" and shows AUTO
/// (p.88-89). PEAK, FREQ, Lo (a 1 kHz low-pass on AC V) and 4-20mA (DC mA)
/// are menu labels whose page the manual does not give (p.71), so those
/// steps page until the label shows; the manual gives no way out of PEAK.
/// A lead in the wrong socket shows "Check inPut" (p.73), which a VC871
/// sends as status bit 16 (spec §14.4). Power as the OW67B (p.84-88).
fn vc871() -> Vec<CaptureStep> {
    let mut steps = vc_gate("Leads in COM and VΩ: set the dial to V≅, leads open (⎓ shows)");
    steps.extend(vc_volts(Vc::Vc871));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅ position: press SELECT for AC (~ shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acv_freq", VC8X1_ACV_FREQ).wait_for_enter(),
        CaptureStep::basic("acv_peak", VC8X1_ACV_PEAK)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic("acv_lpf", VC8X1_ACV_LPF)
            .wait_for_enter()
            .expect(Expect::mode("LPF AC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(vc_middle());
    steps.extend(vc_temperature(
        "Remove the test leads, set the dial to °C°F with the K-type wire probe in °C (+) and \
         COM (-) (temperature)",
    ));
    steps.extend(vc8x1_current(true));
    steps.extend(power_steps(
        "Remove the test leads and set the dial to W cosφ, nothing in the input sockets (AC \
         power: W and VA show)",
        "AC power: press F1 (Display) once (V and A show)",
        "AC power: press F1 (Display) again (PF and Hz show)",
        "W cosφ position: press SELECT once for DC power (⎓ shows), nothing in the input \
         sockets",
        "DC power: plug the DC power measurement module into 10A, COM and VΩ, a DC supply on \
         its INPUT and a load on its OUTPUT, and switch both on (W and A show)",
        "Unplug the module, then press SELECT again for USB power (the USB symbol shows), \
         nothing in the input sockets",
        "USB power: plug the USB power measurement module into 10A, COM and VΩ, a USB charger \
         on its INPUT and a device to charge on its OUTPUT (W and A show)",
    ));
    steps
}

/// The Voltcraft meter a shared step list is worded for, where their
/// manuals differ: the VC915's and VC925 PV's dial reads V≅% (VC915-UM p.60;
/// VC925-UM p.72), and the VC925 PV's manual holds keys with no time given
/// and names MAX/MIN held as the way out (VC925-UM p.86-87).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vc {
    Vc871,
    Vc891,
    Vc915,
    Vc925,
}

/// The VC871's and VC891's AC V menu steps: PEAK, FREQ and Lo (the 1 kHz
/// low-pass) are labels on a page the manuals do not give (VC871-UM p.71;
/// VC891-UM p.65), and neither manual gives a way out of PEAK.
const VC8X1_ACV_FREQ: &str = "AC V: press < or > briefly until FREQ shows over an F key, press \
     that key, then Enter. Press it again afterwards.";
const VC8X1_ACV_PEAK: &str = "AC V: press < or > briefly until PEAK shows over an F key, press \
     that key, then Enter. Afterwards leave PEAK (the manual gives no way: try holding its key, \
     else turn the dial to mV≅ and back to V≅).";
const VC8X1_ACV_LPF: &str = "AC V: press < or > briefly until Lo shows over an F key, press that \
     key (the 1 kHz low-pass filter), then Enter. Press it again afterwards.";

/// The Voltcraft gate: V≅ defaults to DC (VC871-UM p.74; VC891-UM p.68;
/// VC915-UM p.71; VC925-UM p.74), and →|·))Ω to resistance.
fn vc_gate(dcv: &'static str) -> Vec<CaptureStep> {
    steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", dcv),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to →|·))Ω, leads open (Ω without the continuity or diode symbol; \
             should show OL)",
        ),
    )
    .to_vec()
}

/// The Voltcraft meters' V≅ steps (VC871-UM p.88-89; VC891-UM p.78-79;
/// VC915-UM p.86; VC925-UM p.86-87): RANGE, MAX/MIN, REL and HOLD on the
/// first menu page, RANGE and REL held, and LoZ held up to 3 s. Which of
/// MAX and MIN a press shows first is not stated. MAX/MIN held leaves it on
/// the VC925 PV; the others' manuals say it "activates MAX/MIN again".
fn vc_volts(vc: Vc) -> Vec<CaptureStep> {
    let hold = match vc {
        Vc::Vc871 | Vc::Vc891 => {
            "Set the dial back to V≅ (⎓ shows), leads open: press < or > briefly until RANGE, \
             MAX/MIN, REL and HOLD show over F1-F4, then press HOLD (HOLD shows), then Enter. \
             Press HOLD again afterwards."
        }
        Vc::Vc915 => {
            "Set the dial back to V≅% (⎓ shows), leads open: press < or > briefly until RANGE, \
             MAX/MIN, REL and HOLD show over F1-F4, then press HOLD (HOLD shows), then Enter. \
             Press HOLD again afterwards."
        }
        Vc::Vc925 => {
            "Swap the orange lead for the red one in VΩ, set the dial back to V≅% (⎓ shows), \
             leads open: press < or > briefly until RANGE, MAX/MIN, REL and HOLD show over \
             F1-F4, then press HOLD (HOLD shows), then Enter. Press HOLD again afterwards."
        }
    };
    let (range_auto, rel_exit) = if vc == Vc::Vc925 {
        (
            "DC V: hold RANGE (AUTO shows), then Enter.",
            "DC V: hold REL (Δ goes off), then Enter. Afterwards, if AUTO is off, hold RANGE.",
        )
    } else {
        (
            "DC V: hold RANGE for about 1 second (AUTO shows), then Enter.",
            "DC V: hold REL for about 1 second (Δ goes off), then Enter. Afterwards, if AUTO \
             is off, hold RANGE for about 1 second.",
        )
    };
    let minmax = match vc {
        Vc::Vc925 => CaptureStep::basic(
            "minmax_exit",
            "DC V: hold MAX/MIN to leave it (AUTO shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        Vc::Vc915 => CaptureStep::basic(
            "minmax_hold",
            "DC V: hold MAX/MIN for about 1 second, then Enter (the manual says AUTO shows). \
             Afterwards, if MAX or MIN still shows, turn the dial to mV≅ and back to V≅%.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        Vc::Vc871 | Vc::Vc891 => CaptureStep::basic(
            "minmax_hold",
            "DC V: hold MAX/MIN for about 1 second, then Enter (the manual says AUTO shows). \
             Afterwards, if MAX or MIN still shows, turn the dial to mV≅ and back to V≅.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
    };
    vec![
        CaptureStep::basic("hold", hold)
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "manual_range",
            "DC V: press RANGE once (AUTO goes off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic("range_auto", range_auto)
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic("rel", "DC V: press REL (Δ shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic("rel_exit", rel_exit)
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REL_OFF)),
        CaptureStep::basic("max", "DC V: press MAX/MIN until MAX shows, then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MAX_ON)),
        CaptureStep::basic("min", "DC V: press MAX/MIN until MIN shows, then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(MIN_ON)),
        minmax,
        CaptureStep::basic(
            "loz",
            "DC V, leads open: hold LoZ (no more than 3 seconds; Loz shows) and press Enter \
             while holding it.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(LOZ_ON)),
    ]
}

/// The Voltcraft meters' →|·))Ω, ⊣⊢ and Hz% positions (VC871-UM p.78-83;
/// VC891-UM p.73-77; VC915-UM p.80-84): continuity and diode on one and two
/// SELECT presses, duty on the sub-display beside Hz, and SELECT swapping
/// Hz and %.
fn vc_middle() -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic(
            "cont",
            "Set the dial to →|·))Ω and press SELECT once for continuity (the continuity \
             symbol shows); touch the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "→|·))Ω position: press SELECT again for diode (the diode symbol shows), leads open \
             (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic("cap", "Set the dial to ⊣⊢, leads open (nF shows)")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "hz",
            "Set the dial to Hz%, leads open (Hz shows, and % on the sub-display)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press SELECT to swap the displays (% on the main display)",
        )
        .expect(Expect::mode("Duty %")),
    ]
}

/// °C°F on the Voltcraft meters: SELECT switches ℃ and ℉ (VC871-UM p.83;
/// VC891-UM p.78; VC915-UM p.85; VC925-UM p.83). `temp_c` is the model's
/// probe step.
fn vc_temperature(temp_c: &'static str) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("temp_c", temp_c)
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C")),
        CaptureStep::basic("temp_f", "Temperature: press SELECT for ℉ (℉ shows)")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
    ]
}

/// The VC871's and VC891's current positions (VC871-UM p.76-77; VC891-UM
/// p.71-73): DC by default, SELECT for AC, µA and mA in mAµA, up to 10 A in
/// 10A; 4-20mA on DC mA (p.71). The A≅ position is
/// first set with the red lead still in mAµA, for "Check inPut" (VC871-UM
/// p.73; VC891-UM p.67), which a VC871 sends as status bit 16 (spec
/// §14.4), so on the VC871 (`vc871`) that step expects the lead error.
fn vc8x1_current(vc871: bool) -> Vec<CaptureStep> {
    let mut steps = vec![
        CaptureStep::basic(
            "dcua",
            "Remove the probe, set the dial to µA≅ with the test leads in COM and mAµA, leads \
             open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≅, red lead still in mAµA, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "loop",
            "DC mA, leads open: press < or > briefly until 4-20mA shows over an F key, press \
             that key, then Enter. Press it again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("4-20mA")),
        CaptureStep::basic("acma", "mA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
    ];
    // No Enter: the flag shows only briefly after the dial moves (spec
    // §14.4), and the expectation catches it as it comes.
    let check_input = CaptureStep::basic(
        "check_input",
        "Set the dial to A≅ with the red lead still in mAµA (a lead in the wrong socket shows \
         Check inPut with a beep).",
    );
    steps.push(if vc871 {
        check_input.expect(Expect::mode("DC A").flags(LEAD_ERROR))
    } else {
        check_input
    });
    steps.extend([
        CaptureStep::basic(
            "dca",
            "A≅ position: move the red lead to 10A, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// The Voltcraft meters' common keys: Select, Range, a long RANGE (auto),
/// Hold, Rel and a long REL (leave REL), and Max/Min (spec §10.8). A long
/// REL is code 4, the B series' Bluetooth key, so it goes out only once
/// FFF2 named the model.
fn vc_keys(vc: Vc) -> Vec<CaptureStep> {
    let select = match vc {
        Vc::Vc871 | Vc::Vc891 => {
            "Set the dial to V≅ with the leads in COM and VΩ, leads open, ⎓ showing (press \
             SELECT until it does): we will send Select (~ should show)."
        }
        Vc::Vc915 | Vc::Vc925 => {
            "Set the dial to V≅% with the leads in COM and VΩ, leads open, ⎓ showing (press \
             SELECT until it does): we will send Select (~ should show)."
        }
    };
    let exit_rel = if vc == Vc::Vc925 {
        "DC V, Δ showing: we will send a long REL (Δ should go off). Afterwards, if AUTO is \
         off, hold RANGE."
    } else {
        "DC V, Δ showing: we will send a long REL (Δ should go off). Afterwards, if AUTO is \
         off, hold RANGE for about 1 second."
    };
    let minmax = match vc {
        Vc::Vc871 | Vc::Vc891 => {
            "DC V: we will send Max/Min (MAX or MIN should show). Afterwards hold MAX/MIN for \
             about 1 second, and turn the dial to mV≅ and back to V≅ if MAX or MIN still shows."
        }
        Vc::Vc915 => {
            "DC V: we will send Max/Min (MAX or MIN should show). Afterwards hold MAX/MIN for \
             about 1 second, and turn the dial to mV≅ and back to V≅% if MAX or MIN still \
             shows."
        }
        // Its long MAX/MIN step follows and leaves it.
        Vc::Vc925 => "DC V: we will send Max/Min (MAX or MIN should show).",
    };
    vec![
        key("key_select", select, "select").expect(Expect::mode("AC V")),
        key(
            "key_hold",
            "Press SELECT until ⎓ shows: we will send Hold (HOLD should show). Press HOLD \
             afterwards.",
            "hold",
        )
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        key("key_rel", "DC V: we will send Rel (Δ should show).", "rel")
            .expect(Expect::mode("DC V").flags(REL_ON)),
        key("key_exit_rel", exit_rel, "exit_rel").expect(Expect::mode("DC V").flags(REL_OFF)),
        key(
            "key_range",
            "DC V, AUTO showing: we will send Range (AUTO should go off).",
            "range",
        )
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        key(
            "key_auto",
            "DC V, AUTO off (press RANGE if it shows): we will send a long RANGE (AUTO should \
             show).",
            "auto",
        )
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        key("key_minmax", minmax, "minmax"),
    ]
}

/// The VC871 entry's keys: the Voltcraft ones, then Peak, 4~20mA and
/// Display (spec §10.8), which expect nothing: what they do as remote keys
/// is not stated.
fn vc871_keys() -> Vec<CaptureStep> {
    let mut steps = vc_keys(Vc::Vc871);
    steps.extend([
        key(
            "key_peak",
            "V≅ position, ~ showing (press SELECT if ⎓ shows): we will send Peak. Afterwards, \
             if PEAK is on, leave it (the manual gives no way: press < or > briefly until PEAK \
             shows over an F key and try holding that key, else turn the dial to mV≅ and back \
             to V≅).",
            "peak",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards leave 4-20 mA through its F key if it shows.",
            "current_loop",
        ),
        key(
            "key_display",
            "Remove the test leads and set the dial to W cosφ, nothing in the input sockets, AC \
             power showing: we will send Display.",
            "display",
        ),
    ]);
    steps
}

/// The VC891, from its manual, the OW69B's twin (spec §9.4): the VC871's
/// dial with OFF in place of W cosφ, V≅ also AC+DC, reached by a press its
/// manual calls "press the dial" (VC891-UM p.56, p.70); the VC915's manual
/// gives SELECT for the same step (VC915-UM p.73). Lo is the 1 kHz low-pass
/// on AC V (p.65); the manual gives no way out of PEAK.
fn vc891() -> Vec<CaptureStep> {
    let mut steps = vc_gate("Leads in COM and VΩ: set the dial to V≅, leads open (⎓ shows)");
    steps.extend(vc_volts(Vc::Vc891));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅ position: press SELECT until ~ shows without the AC+DC icon, leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acv_freq", VC8X1_ACV_FREQ).wait_for_enter(),
        CaptureStep::basic("acv_peak", VC8X1_ACV_PEAK)
            .wait_for_enter()
            .expect(Expect::mode("Peak AC V")),
        CaptureStep::basic("acv_lpf", VC8X1_ACV_LPF)
            .wait_for_enter()
            .expect(Expect::mode("LPF AC V")),
        CaptureStep::basic(
            "acdcv",
            "V≅ position: press SELECT until the AC+DC icon shows (the manual says \"press the \
             dial\"), leads open",
        )
        .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(vc_middle());
    steps.extend(vc_temperature(
        "Remove the test leads, set the dial to °C°F with the K-type wire probe in °C (+) and \
         COM (-) (temperature)",
    ));
    steps.extend(vc8x1_current(false));
    steps
}

/// The VC891 entry's keys: the Voltcraft ones, then Hz/Duty, LPF, Peak and
/// 4~20mA (spec §10.8), which expect nothing.
fn vc891_keys() -> Vec<CaptureStep> {
    let mut steps = vc_keys(Vc::Vc891);
    steps.extend([
        key(
            "key_hz_duty",
            "Set the dial to Hz%, leads open, Hz showing: we will send Hz/Duty. Afterwards \
             press SELECT if % is on the main display.",
            "hz_duty",
        ),
        key(
            "key_lpf",
            "Set the dial to V≅, ~ showing (press SELECT until it does), leads open: we will \
             send LPF. Afterwards press < or > briefly until Lo shows over an F key, and press \
             it if Lo is on.",
            "lpf",
        ),
        key(
            "key_peak",
            "AC V: we will send Peak. Afterwards, if PEAK is on, leave it (the manual gives no \
             way: press < or > briefly until PEAK shows over an F key and try holding that key, \
             else turn the dial to mV≅ and back to V≅).",
            "peak",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards leave 4-20 mA through its F key if it shows.",
            "current_loop",
        ),
    ]);
    steps
}

/// The VC915's dial runs OFF, V≅% (also AC+DC), mV≅, →|·))Ω, ⊣⊢, Hz%,
/// °C°F, µA≅, mA≅, A≅, OFF (VC915-UM p.60). On V≅ SELECT gives AC on one
/// press and AC+DC on two; the menu, paged by < (BLE) and >, holds DISPLAY
/// (F3 swaps Hz/% and V), LPF (1 kHz), MOTOR and, on AC+DC, AC/DC (F1
/// switches the sub-display between VAC and VDC) (p.72-76). Motor's
/// rotation needs three live phases, so its step stops at the waiting
/// screen with the leads open, L3 and L1 flashing; RANGE, MAX/MIN and REL
/// are off there (p.75). The menu's items besides DISPLAY and AC/DC have no
/// F key named, so those steps name the key by the label over it. Current: SELECT between DC and AC, µA and mA in mAµA, up to 20 A
/// in 10A; 4-20mA from the menu on mA, in series with a 15-48 V supply
/// (p.77-79).
fn vc915() -> Vec<CaptureStep> {
    let mut steps = vc_gate("Leads in COM and VΩ: set the dial to V≅%, leads open (⎓ shows)");
    steps.extend(vc_volts(Vc::Vc915));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅% position: press SELECT once for AC (~ shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_display",
            "AC V: press < or > briefly until DISPLAY shows in the menu, then F3 (Hz/% and V \
             swap between the main and sub-display), then Enter. Press F3 again afterwards.",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "acv_lpf",
            "AC V: press < briefly until LPF shows in the menu, then the F key under it (the \
             LPF icon shows), then Enter. Press that key again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("LPF AC V")),
        CaptureStep::basic(
            "motor",
            "AC V, leads open: press < briefly until MOTOR shows in the menu, then the F key \
             under it (L3 and L1 flash), then Enter. Press that key again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Motor").value(ValueExpect::NoReading)),
        CaptureStep::basic(
            "acdcv",
            "V≅% position: press SELECT until the AC+DC symbol shows (two presses from ⎓), \
             leads open",
        )
        .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic(
            "acdcv_sub",
            "AC+DC V: press < briefly until AC/DC shows in the menu, then F1 (the sub-display \
             switches between VAC and VDC), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
    ]);
    steps.extend(vc_middle());
    steps.extend(vc_temperature(
        "Remove the test leads, set the dial to °C°F with the K-type adapter in °C (+) and COM \
         (-) (temperature)",
    ));
    steps.extend([
        CaptureStep::basic(
            "dcua",
            "Remove the adapter, set the dial to µA≅ with the test leads in COM and mAµA, leads \
             open, and press SELECT until ⎓ shows",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≅, red lead still in mAµA, leads open, and press SELECT until \
             ⎓ shows",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(vc9x5_loop(
        "DC mA, leads open: press < briefly until 4-20mA shows in the menu, then the F key \
         under it, then Enter.",
        "4-20 mA: connect the leads in series with a 4-20 mA loop and its 15-48 V DC supply, \
         then switch the supply on",
    ));
    steps.extend([
        CaptureStep::basic(
            "acma",
            "Switch the loop supply off and remove the leads from it, press < briefly until \
             4-20mA shows in the menu and press the F key under it to leave 4-20 mA, then press \
             SELECT for AC (~ shows)",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dca",
            "Set the dial to A≅ and move the red lead to 10A, leads open, and press SELECT \
             until ⎓ shows",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
    ]);
    steps
}

/// 4-20 mA on the VC915 and VC925 PV: chosen from the < menu on mA, then in
/// series with the loop and a 15-48 V DC supply (VC915-UM p.79; VC925-UM
/// p.79).
fn vc9x5_loop(open: &'static str, live: &'static str) -> Vec<CaptureStep> {
    vec![
        CaptureStep::basic("loop", open)
            .wait_for_enter()
            .expect(Expect::mode("4-20mA")),
        CaptureStep::basic("loop_live", live).needs(&[Need::LoopSource]),
    ]
}

/// The VC915 entry's keys: the Voltcraft ones, then LPF, Compare, AC/DC,
/// Motor, 4~20mA and Display (spec §10.8), which expect nothing.
fn vc915_keys() -> Vec<CaptureStep> {
    let mut steps = vc_keys(Vc::Vc915);
    steps.extend([
        key(
            "key_display",
            "V≅% position, ~ showing (press SELECT until it does), leads open: we will send \
             Display. Afterwards press F3 on the DISPLAY menu page if Hz/% is on the main \
             display.",
            "display",
        ),
        key(
            "key_lpf",
            "AC V: we will send LPF. Afterwards, if the LPF icon shows, press < briefly until \
             LPF shows in the menu and press the F key under it.",
            "lpf",
        ),
        key(
            "key_motor",
            "AC V, leads open: we will send Motor. Afterwards, if the motor screen shows, press \
             < briefly until MOTOR shows in the menu and press the F key under it.",
            "motor",
        ),
        key(
            "key_ac_dc",
            "V≅% position: press SELECT until the AC+DC symbol shows: we will send AC/DC.",
            "ac_dc",
        ),
        key(
            "key_compare",
            "Press SELECT until ⎓ shows: we will send Compare. Afterwards turn the dial away and \
             back if COMP shows.",
            "compare",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards, if 4-20 mA shows, press < briefly until 4-20mA shows in \
             the menu and press the F key under it.",
            "current_loop",
        ),
    ]);
    steps
}

/// The VC925 PV's dial runs OFF, 2kV⎓/1.5kV~, V≅%, mV≅, →|·))Ω, ⊣⊢, Hz%,
/// °C°F, µA≅, mA≅, PV: no A position or 10 A jack, high voltage on its own
/// orange-lead jack (VC925-UM p.68, p.72). SELECT gives AC on V and mA,
/// AC/DC on the high-voltage position, diode and continuity in an order
/// the manual does not give, Hz/% and ℃/℉ (p.75-85); DISPLAY (F1) swaps
/// Hz/% and V on AC V (p.75). PV reads the LX-925 irradiance adapter once
/// linked from its Find page, the LX-925 switched on with POWER and its
/// Bluetooth on with its own BLE button (Status LED red); F1 Display swaps
/// the main and sub-display (p.69, p.89-92). The adapter link beside the
/// phone's is open (spec §9.4), so the PV steps record it.
fn vc925pv() -> Vec<CaptureStep> {
    let mut steps = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ: set the dial to V≅%, leads open (⎓ shows)",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to →|·))Ω and press SELECT until Ω shows without the diode or \
             continuity symbol, leads open (should show OL)",
        ),
    )
    .to_vec();
    steps.extend([
        CaptureStep::basic(
            "hv_dc",
            "Remove the red lead, put the orange lead in the 2kV⎓ 1.5kV~ jack, leads open, set \
             the dial to 2kV⎓/1.5kV~ and press SELECT until ⎓ shows",
        )
        .expect(Expect::mode("HV DC V")),
        CaptureStep::basic(
            "hv_ac",
            "2kV⎓/1.5kV~ position, leads open: press SELECT until ~ shows",
        )
        .expect(Expect::mode("HV AC V")),
    ]);
    steps.extend(vc_volts(Vc::Vc925));
    steps.extend([
        CaptureStep::basic(
            "acv",
            "V≅% position: press SELECT for AC (~ shows), leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "acv_display",
            "AC V: press < or > briefly until DISPLAY shows in the menu, then F1 (Hz/% and V \
             swap), then Enter. Press F1 again afterwards.",
        )
        .wait_for_enter(),
        CaptureStep::basic("dcmv", "Set the dial to mV≅, leads open (⎓ shows)")
            .expect(Expect::mode("DC V")),
        CaptureStep::basic("acmv", "mV≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "cont",
            "Set the dial to →|·))Ω and press SELECT until the continuity symbol shows; touch \
             the probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "→|·))Ω position: press SELECT until the diode symbol shows, leads open (OL shows)",
        )
        .expect(Expect::mode("Diode").value(ValueExpect::Overload)),
        CaptureStep::basic("cap", "Set the dial to ⊣⊢, leads open (nF shows)")
            .expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set the dial to Hz%, leads open (Hz shows)")
            .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "Hz% position: press SELECT for duty cycle (% shows)",
        )
        .expect(Expect::mode("Duty %")),
    ]);
    steps.extend(vc_temperature(
        "Remove the test leads, set the dial to °C°F with the K-type adapter in °C (+) and COM \
         (-) (temperature)",
    ));
    steps.extend([
        CaptureStep::basic(
            "dcua",
            "Remove the adapter, set the dial to µA≅ with the test leads in COM and mAµA, leads \
             open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("acua", "µA≅ position: press SELECT for AC (~ shows)")
            .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to mA≅, red lead still in mAµA, leads open (⎓ shows)",
        )
        .expect(Expect::mode("DC A")),
    ]);
    steps.extend(vc9x5_loop(
        "DC mA, leads open: press < briefly until 4-20mA shows in the menu, then the F key \
         under it, then Enter.",
        "4-20 mA: connect the leads in series with a 4-20 mA loop and its 15-48 V DC supply, \
         then switch the supply on",
    ));
    steps.extend([
        CaptureStep::basic(
            "acma",
            "Switch the loop supply off and remove the leads from it, press < briefly until \
             4-20mA shows in the menu and press the F key under it to leave 4-20 mA, then press \
             SELECT for AC (~ shows)",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "pv_link",
            "Switch the LX-925 on (POWER) and its Bluetooth on with its own BLE button (Status \
             LED red). Remove the test leads, set the dial to PV and link the LX-925: press < \
             briefly to the first page, F1 Find, F1 Find again, pick \"PV adaptor VC92\" and \
             press F4 CONNECT (VC92 shows), then Enter. This step records whether readings keep \
             coming.",
        )
        .needs(&[Need::PvAdapter])
        .wait_for_enter(),
        CaptureStep::basic(
            "pv",
            "PV, LX-925 linked: hold its sensor towards a light, then tilt and turn it (W/m², \
             the angle and the compass direction show)",
        )
        .needs(&[Need::PvAdapter]),
        CaptureStep::basic(
            "pv_display",
            "PV, LX-925 linked: press F1 (Display) to swap the main and sub-display, then \
             Enter.",
        )
        .needs(&[Need::PvAdapter])
        .wait_for_enter(),
    ]);
    steps
}

/// The VC925 PV entry's keys: the Voltcraft ones, a long MAX/MIN (leave
/// MAX/MIN, VC925-UM p.87), then Compare, 4~20mA and Display (spec §10.8),
/// which expect nothing.
fn vc925pv_keys() -> Vec<CaptureStep> {
    let mut steps = vc_keys(Vc::Vc925);
    steps.extend([
        key(
            "key_exit_minmax",
            "DC V, MAX or MIN showing (press MAX/MIN if neither does): we will send a long \
             MAX/MIN to leave it (AUTO should show).",
            "exit_minmax",
        )
        .expect(Expect::mode("DC V").flags(MAX_MIN_OFF)),
        key(
            "key_display",
            "Press SELECT for AC: we will send Display. Afterwards press F1 on the DISPLAY menu \
             page if Hz/% is on the main display.",
            "display",
        ),
        key(
            "key_compare",
            "Press SELECT for DC: we will send Compare. Afterwards press COMP again, or turn \
             the dial away and back, if COMP shows.",
            "compare",
        ),
        key(
            "key_current_loop",
            "Set the dial to mA≅ with the red lead in mAµA, leads open, ⎓ showing: we will \
             send 4~20mA. Afterwards, if 4-20 mA shows, press < briefly until 4-20mA shows in \
             the menu and press the F key under it.",
            "current_loop",
        ),
    ]);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::owon::model::MODELS;

    /// Every entry's model code and the remote commands it offers.
    fn entries() -> impl Iterator<Item = (u8, &'static [&'static str])> {
        MODELS.iter().map(|m| (m.code, m.commands))
    }

    /// The Bluetooth key's label on each model's panel, and the name the
    /// Voltcraft manuals also give it, "BLE" (spec §9.4).
    fn bluetooth_key(code: u8) -> &'static [&'static str] {
        match code {
            21 => &["ZERO"],
            101 | 61 => &["Tab⇌"],
            65 => &["SETUP"],
            67 | 69 => &["<"],
            87 | 89 | 91 | 92 => &["<", "BLE"],
            _ => &["△/ᛒ"],
        }
    }

    #[test]
    fn an_unknown_code_has_no_steps() {
        for code in [0, 55, 83, 85, 223] {
            assert!(steps(code).is_empty(), "{code}");
        }
        assert_eq!(entries().count(), 15);
    }

    #[test]
    fn the_gate_leads_and_nothing_is_verified() {
        for (code, _) in entries() {
            let steps = steps(code);
            assert!(steps[..6].iter().all(|s| s.gate), "{code}");
            assert!(steps[6..].iter().all(|s| !s.gate), "{code}");
            assert!(steps.iter().all(|s| !s.verified), "{code}");
            let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), steps.len(), "{code}: ids are unique");
        }
    }

    /// Each entry sends only the keys it offers, every one of them, and
    /// the key steps come last.
    #[test]
    fn key_steps_send_offered_keys_and_every_key_has_one() {
        for (code, commands) in entries() {
            let steps = steps(code);
            for step in &steps {
                if let Some(command) = step.command {
                    assert!(commands.contains(&command), "{code} {}", step.id);
                }
            }
            for command in commands {
                assert!(
                    steps.iter().any(|s| s.command == Some(command)),
                    "{code} {command}"
                );
            }
            let first_key = steps.iter().position(|s| s.command.is_some()).unwrap();
            assert!(
                steps[first_key..].iter().all(|s| s.command.is_some()),
                "{code}: keys last"
            );
        }
    }

    /// Held, the Bluetooth key switches Bluetooth (spec §7.1, §9.1): every
    /// step that names it by its panel label presses it briefly or sends it
    /// short, and no step holds it by either name. "BLE" alone is also the
    /// LX-925 adapter's own Bluetooth button (`pv_link`).
    #[test]
    fn no_step_holds_the_bluetooth_key() {
        for (code, _) in entries() {
            let labels = bluetooth_key(code);
            for step in steps(code) {
                let text = step.instruction;
                if text.contains(labels[0]) {
                    assert!(
                        text.contains("briefly") || text.contains("short"),
                        "{code} {}",
                        step.id
                    );
                }
                for label in labels {
                    for verb in ["hold", "Hold", "long"] {
                        assert!(
                            !text.contains(&format!("{verb} {label}")),
                            "{code} {}",
                            step.id
                        );
                    }
                }
            }
        }
    }

    /// Never the leads on mains: a live wire is for the NCV step alone,
    /// on the models with NCV.
    #[test]
    fn only_the_ncv_step_needs_a_live_wire() {
        for (code, _) in entries() {
            let live: Vec<&str> = steps(code)
                .iter()
                .filter(|s| s.needs.contains(&Need::LiveWire))
                .map(|s| s.id)
                .collect();
            let want: &[&str] = match code {
                18 | 20 | 21 | 101 | 61 => &["ncv"],
                _ => &[],
            };
            assert_eq!(live, want, "{code}");
        }
    }
}
