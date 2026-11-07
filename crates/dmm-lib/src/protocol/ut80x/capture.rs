//! The capture steps of the meters on this packet stream, one list with a
//! wording per model.

use super::Model::{self, Ut71Ab, Ut71Cde, Ut803, Ut804, Vc920};
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, RangeExpect};

pub(super) fn steps(model: Model) -> Vec<CaptureStep> {
    // The two parsers name the same dial position differently, and a
    // label this model cannot report leaves the step waiting for a state
    // that never arrives. Only the UT803's V and mV carry an AC/DC
    // prefix — its current modes are named by unit alone — and it has
    // neither the duty-cycle display nor the "mA%" mode, while AC+DC is
    // the UT804 layout's.
    let ut804_layout = model.ut804_layout();
    let for_model = |ut804: Option<&'static str>, ut803_label: Option<&'static str>| {
        if ut804_layout { ut804 } else { ut803_label }
    };
    // Assert the mode where this model has a label for it, and nothing
    // where it has not.
    let named = |step: CaptureStep, label: Option<&'static str>| match label {
        Some(label) => step.expect(Expect::mode(label)),
        None => step,
    };
    // The UT71 and VC9x0 handhelds reach AC and each position's other
    // functions with the blue button and AC+DC with the yellow one, and
    // turn SEND on by holding MAX MIN, or with the UT71A's SEND key (UT71
    // manual Tables 2-1 and 2-2; VC920/940/960 manual §6, §7). `say`
    // picks their wording of a step the bench meters word otherwise.
    let handheld = match model {
        Ut803 | Ut804 => false,
        Ut71Ab | Ut71Cde | Vc920 => true,
    };
    let say = |bench: &'static str, handheld_text: &'static str| {
        if handheld { handheld_text } else { bench }
    };

    // These models reach OL in a step of their own, after a plain "turn
    // the dial" one, so the resistance gate is the second of the pair
    // rather than the step called "ohm".
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", "Set meter to DC V"),
        Ohms::Word,
        CaptureStep::basic(
            "ohm_ol",
            "Set meter to Resistance (Ω) with open leads (overload)",
        ),
    );

    // Which models' dial or buttons reach a step. The UT804 has no AC mV
    // or hFE position (its manual's Table 2-1, #16); RANGE,
    // MAX MIN and REL are asked of the UT804 and the handhelds. On the
    // handhelds the blue button steps resistance to continuity and
    // diode, and Hz follows mV, whose position the UT71B/C/D and
    // VC920/VC960 share it with; a UT71E or VC940 turns ahead to its
    // °C/°F position for Hz and back. W sits between Ω and capacitance,
    // as on those two dials. Issue #16's reporter walked every UT804 step
    // by 2026-09-19; no other model has answered.
    const ALL: &[Model] = &[Ut803, Ut804, Ut71Ab, Ut71Cde, Vc920];
    const BENCH: &[Model] = &[Ut803, Ut804];
    const HANDHELD: &[Model] = &[Ut71Ab, Ut71Cde, Vc920];
    const BUTTONS: &[Model] = &[Ut804, Ut71Ab, Ut71Cde, Vc920];
    let on = |models: &[Model], step: CaptureStep| {
        models
            .contains(&model)
            .then(|| step.verified_if(model == Ut804))
    };

    [
        on(ALL, dcv),
        on(ALL, dcv_short),
        on(ALL, dcv_negative),
        on(
            ALL,
            CaptureStep::basic(
                "acv",
                say(
                    "Set meter to AC V",
                    "Set meter to AC V (on a shared V position, press the blue button for AC)",
                ),
            )
            .expect(Expect::mode("AC V")),
        ),
        // AC+DC is the AC/DC nibble's fourth value; the spec (§5) has it
        // on the UT804, behind a button of its own on the AC V position,
        // and leaves the UT803 open. The handhelds' yellow button gives
        // it on AC V (UT71 manual Table 2-2).
        on(
            ALL,
            named(
                CaptureStep::basic(
                    "acdcv",
                    match model {
                        Ut803 => "Set meter to AC+DC V (if the meter has it)",
                        Ut804 => "AC V: press the AC/AC+DC button so the display shows AC+DC",
                        Ut71Ab | Ut71Cde | Vc920 => {
                            "AC V: press the yellow AC+DC button so the display shows AC+DC"
                        }
                    },
                ),
                for_model(Some("AC+DC V"), None),
            ),
        ),
        on(
            ALL,
            CaptureStep::basic("dcmv", "Set meter to DC mV").expect(Expect::mode("DC mV")),
        ),
        on(
            &[Ut803],
            CaptureStep::basic("acmv", "Set meter to AC mV (if the meter has it)")
                .expect(Expect::mode("AC mV")),
        ),
        on(
            HANDHELD,
            CaptureStep::basic(
                "hz",
                "Set meter to Frequency (Hz): the dial position marked Hz, \
                 then the blue button until Hz shows",
            )
            .expect(Expect::mode("Frequency")),
        ),
        on(
            HANDHELD,
            CaptureStep::basic(
                "duty",
                "Frequency mode: press the blue button until % shows (duty cycle)",
            )
            .expect(Expect::mode("Duty %")),
        ),
        on(
            ALL,
            CaptureStep::basic("ohm", "Set meter to Resistance (Ω)").expect(Expect::mode("Ω")),
        ),
        on(ALL, ohm_ol),
        on(ALL, ohm_body),
        on(ALL, ohm_short),
        on(
            HANDHELD,
            CaptureStep::basic(
                "cont",
                "Resistance position: press the blue button for Continuity",
            )
            .expect(Expect::mode("Continuity")),
        ),
        on(
            HANDHELD,
            CaptureStep::basic(
                "diode",
                "Resistance position: press the blue button again for Diode",
            )
            .expect(Expect::mode("Diode")),
        ),
        // Function code E (docs/research/ut71/reverse-engineered-protocol.md
        // §3.2). The UT71E's and VC940's power adapter plugs into the
        // input jacks and into a mains outlet, and the load into it (UT71
        // manual, "Power Measurement"; VC920/940/960 manual §9 l).
        on(
            HANDHELD,
            CaptureStep::basic(
                "power",
                "Set meter to W (power, if the meter has it): its adapter into a live \
                 mains outlet, and a load into the adapter",
            )
            .needs(&[Need::PowerAdapter])
            .expect(Expect::mode("Power")),
        ),
        on(
            ALL,
            CaptureStep::basic("cap", "Set meter to Capacitance")
                .expect(Expect::mode("Capacitance")),
        ),
        // Both models name the frequency mode "Frequency", not "Hz".
        on(
            BENCH,
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)")
                .expect(Expect::mode("Frequency")),
        ),
        // The UT803's manual has no duty, tachometer or 4-20 mA position
        // (its rotary switch table), so it is asked for none of them.
        on(
            &[Ut804],
            CaptureStep::basic(
                "duty",
                "Frequency mode: switch the display to Duty Cycle (%)",
            )
            .expect(Expect::mode("Duty %")),
        ),
        on(
            BENCH,
            CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
        ),
        // #16's second run verifies this one on the UT804: in the first,
        // the reporter pressed SELECT on to Diode and Ω before the
        // samples were in.
        on(
            BENCH,
            CaptureStep::basic("cont", "Set meter to Continuity")
                .expect(Expect::mode("Continuity")),
        ),
        on(
            ALL,
            CaptureStep::basic(
                "temp",
                say(
                    "Set meter to temperature (K-type thermocouple, if available)",
                    "Set meter to temperature °C (if the meter has it; K-type \
                     thermocouple, if available)",
                ),
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("Temperature")),
        ),
        on(
            HANDHELD,
            CaptureStep::basic(
                "temp_f",
                "Temperature: press the blue button for °F (if the meter has it; K-type \
                 thermocouple, if available)",
            )
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("Temperature")),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic("dcua", "Set meter to DC µA"),
                for_model(Some("DC µA"), Some("µA")),
            ),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic(
                    "acua",
                    say(
                        "Set meter to AC µA",
                        "µA position: press the blue button for AC µA",
                    ),
                ),
                for_model(Some("AC µA"), Some("µA")),
            ),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic("dcma", "Set meter to DC mA"),
                for_model(Some("DC mA"), Some("mA")),
            ),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic(
                    "acma",
                    say(
                        "Set meter to AC mA",
                        "mA position: press the blue button for AC mA",
                    ),
                ),
                for_model(Some("AC mA"), Some("mA")),
            ),
        ),
        // The 4-20 mA loop reading in %, one more blue press on the mA
        // position. With nothing in the loop it reads LO, as the UT804's
        // did in #16, so no loop source is asked for.
        on(
            HANDHELD,
            CaptureStep::basic(
                "ma_percent",
                "mA position: press the blue button until % shows (4-20 mA loop, \
                 if the meter has it)",
            )
            .expect(Expect::mode("mA%")),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic("dca", "Set meter to DC A"),
                for_model(Some("DC A"), Some("A")),
            ),
        ),
        on(
            ALL,
            named(
                CaptureStep::basic(
                    "aca",
                    say(
                        "Set meter to AC A",
                        "A position: press the blue button for AC A",
                    ),
                ),
                for_model(Some("AC A"), Some("A")),
            ),
        ),
        // Mode 14 is hFE on the UT803 (spec §3.4), whose manual says hFE
        // is not sent to the computer: no response is a result too. The
        // transistor gates the step on Enter. Mode 15
        // is the UT804's mA position's 4-20 mA loop reading in %, whose mode
        // keeps the vendor's name "mA%".
        on(
            &[Ut803],
            CaptureStep::basic(
                "hfe",
                "Set meter to hFE, with a transistor in the socket, then Enter. \
                 If the meter sends nothing, that is a valid result. Turn the dial \
                 back to V⎓ afterwards.",
            )
            .needs(&[Need::Transistor])
            .expect(Expect::mode("hFE")),
        ),
        on(
            &[Ut804],
            CaptureStep::basic("ma_percent", "Set meter to % (4-20 mA loop)")
                .expect(Expect::mode("mA%")),
        ),
        // RANGE sets status bit 1 (Manual), MAX MIN puts nothing on the
        // wire, and REL sends the relative reading with bit 1 set (spec
        // §3.6, #16), so the steps assert nothing. MAX MIN works on a
        // manual range only (UT804 manual and UT71 manual, "Using MAX MIN"),
        // so it follows RANGE. EXIT leaves all three, and turns the
        // meter's data output off (#16), so every step that presses it
        // asks for SEND after it. Each step waits for Enter: every press,
        // and the previous step's EXIT and SEND, leaves a state the meter
        // reports, and the watcher would capture the first of them. The
        // handhelds get a step of their own to return to AUTO.
        on(
            BUTTONS,
            CaptureStep::basic("manual_range", "DC V: press RANGE, then Enter.").wait_for_enter(),
        ),
        on(
            BUTTONS,
            CaptureStep::basic(
                "max_min",
                say(
                    "DC V with AUTO off: press MAX MIN, then Enter. \
                     Press EXIT, then SEND, afterwards.",
                    "DC V, still on the manual range: press MAX MIN, then Enter.",
                ),
            )
            .wait_for_enter(),
        ),
        on(
            HANDHELD,
            CaptureStep::basic(
                "auto_range",
                "DC V: press EXIT until AUTO shows and turn SEND on again, then Enter.",
            )
            .wait_for_enter()
            .expect(Expect::new().range(RangeExpect::Auto)),
        ),
        on(
            BUTTONS,
            CaptureStep::basic(
                "rel",
                say(
                    "DC V: press REL, then Enter. Press EXIT, then SEND, afterwards.",
                    "DC V: press REL, then Enter. Press EXIT, then turn SEND on again, \
                     afterwards.",
                ),
            )
            .wait_for_enter(),
        ),
        // The HOLD wire encoding is what this step is for, so it asserts
        // nothing about the flag. The UT804 sends nothing while HOLD is
        // on (#16), which the step reports as no response only after its
        // sampling reads time out, so its instruction says that is a
        // valid result, that it can take up to a minute, and how to leave
        // HOLD for the rest of the run: EXIT turns the data output off as
        // well, and SEND turns it back on. It names the outcome rather
        // than quoting the line, which capture prints in the run's
        // language. That outcome is the result the reporter confirmed on
        // the UT804; the handhelds, whose HOLD nobody has run, get the
        // same. There the step waits for Enter, as REL's clean-up before
        // it leaves a state of its own.
        on(
            ALL,
            match model {
                Ut803 => CaptureStep::basic(
                    "hold",
                    "Press HOLD (wire encoding unknown — capture needed)",
                ),
                Ut804 => CaptureStep::basic(
                    "hold",
                    "Press HOLD on the meter, then Enter. If the meter stops sending, \
                     the tool reports no response within a minute: that is a valid \
                     result. Press EXIT, then SEND, on the meter afterwards.",
                )
                .wait_for_enter(),
                Ut71Ab | Ut71Cde | Vc920 => CaptureStep::basic(
                    "hold",
                    "Press HOLD on the meter, then Enter. If the meter stops sending, \
                     the tool reports no response within a minute: that is a valid \
                     result. Press EXIT, then turn SEND on again, afterwards.",
                )
                .wait_for_enter(),
            },
        ),
        // Holding the blue button at power-on drops every function to
        // 4000 counts until the next power cycle (UT71 manual Table 2-2;
        // VC920/940/960 manual §9), which blanks digit 5 (ut71 spec §3.1).
        // Powering on leaves SEND off.
        on(
            HANDHELD,
            CaptureStep::basic(
                "fast_mode",
                "Turn the meter off, then back on at DC V while holding the blue button \
                 (4000 counts); turn SEND on again, then Enter.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
