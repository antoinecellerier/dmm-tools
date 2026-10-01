//! Capture steps per model, each worded from its own manual.
//!
//! Nothing here has run on a meter. Each list is the gate, then the dial one
//! way round from its first position with the lead changes grouped, then the
//! remote keys the entry offers (spec §7.3). A key leaves the meter wherever
//! it put it, which no manual step starts from, so the keys go last, each
//! starting from the mode it acts in.
//!
//! A long press of the meter's Bluetooth key — △/ᛒ on the B series,
//! Hz/Duty△/ᛒ on the OW16 and OW18, ZERO/ᛒ on the CM2100 (spec §7.1, §9.1) —
//! switches Bluetooth, so every step that presses it says to press it
//! briefly, and the remote keys never send it long.
//!
//! Manual cites are `<manual> p.PDF/printed`, with the spec's short names:
//! B35-UM (35 Series & B41T), B33-UM, OW18-UM, OW16-UM, CM2100-UM.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::owon::model::MODELS;

    /// The six model codes and the remote commands each entry offers.
    fn entries() -> impl Iterator<Item = (u8, &'static [&'static str])> {
        MODELS.iter().map(|m| (m.code, m.commands))
    }

    /// The Bluetooth key's label on each model's panel.
    fn bluetooth_key(code: u8) -> &'static str {
        match code {
            21 => "ZERO",
            _ => "△/ᛒ",
        }
    }

    #[test]
    fn an_unknown_code_has_no_steps() {
        assert!(steps(0).is_empty());
        assert!(steps(55).is_empty());
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
    /// step that names it presses it briefly or sends it short.
    #[test]
    fn no_step_holds_the_bluetooth_key() {
        for (code, _) in entries() {
            let label = bluetooth_key(code);
            for step in steps(code) {
                let text = step.instruction;
                if text.contains(label) {
                    assert!(
                        text.contains("briefly") || text.contains("short"),
                        "{code} {}",
                        step.id
                    );
                }
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
                18 | 20 | 21 => &["ncv"],
                _ => &[],
            };
            assert_eq!(live, want, "{code}");
        }
    }
}
