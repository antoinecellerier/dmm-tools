//! The 121GW's capture steps, worded from its manual.
//!
//! Nothing here has run on a meter. The dial runs OFF, Low Z, V, mV, Hz, Ω,
//! mVA/VA, µA, A/mA, OFF, and MODE cycles each position's functions: V
//! DC, AC, DC+AC; mV DC, AC, temperature; Hz frequency, pulse width, duty;
//! Ω resistance, continuity, diode, capacitance; µA and A/mA DC, AC
//! (manual p.31, p.36, p.41, p.44, p.48). The manual gives no way back to
//! auto-ranging, no DC/AC choice on mVA/VA, and no µVA step on µA (manual
//! p.33, p.41, p.53), so those steps ask rather than expect. Current goes
//! in mA µA or A 500mA (p.34, p.40). The gate first, then the dial one way
//! round from Low Z; the remote keys (spec §11.1) sit in the V steps they
//! act in. Held, 1ms PEAK switches Bluetooth off (p.33), so no step holds
//! it.

use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, RangeExpect};

const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
const REL_ON: &[(Flag, bool)] = &[(Flag::Rel, true)];
const RECORD_ON: &[(Flag, bool)] = &[(Flag::Record, true)];
const PEAK_ON: &[(Flag, bool)] = &[(Flag::PeakMax, true)];
const MIN_MAX_ALL: &[(Flag, bool)] = &[(Flag::Min, true), (Flag::Max, true), (Flag::Avg, true)];
const MIN_MAX_OFF: &[(Flag, bool)] = &[(Flag::Min, false), (Flag::Max, false), (Flag::Avg, false)];

/// A remote key step: the key goes out, five samples follow.
const fn key(id: &'static str, instruction: &'static str, command: &'static str) -> CaptureStep {
    CaptureStep::with_command(id, instruction, command, 5)
}

pub(super) fn steps() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in COM and VΩ: set the dial to V and press MODE until DC shows, leads open",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to Ω and press MODE until Ω shows without the continuity, diode or \
             capacitor symbol, leads open (should show OFL)",
        ),
    );
    vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm_ol,
        ohm_body,
        ohm_short,
        CaptureStep::basic("loz", "Set the dial to Low Z, leads open")
            .expect(Expect::mode("LoZ V")),
        // REL, HOLD, MIN/MAX, MEM and SETUP each leave a state of their own,
        // and the clean-up another, so these capture on Enter.
        CaptureStep::basic(
            "rel",
            "Set the dial to V and press MODE until DC shows, then press REL, then Enter. \
             Press REL again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic("hold", "DC V: press HOLD once (HOLD shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::new().flags(HOLD_ON)),
        CaptureStep::basic(
            "a_hold",
            "DC V: press HOLD again (A-HOLD shows), then Enter. Press HOLD until A- and HOLD \
             are gone afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(HOLD_ON)),
        // The manual gives no order for the MIN, MAX and AVG presses (p.58).
        CaptureStep::basic("min_max", "DC V: press MIN/MAX once, then Enter.").wait_for_enter(),
        CaptureStep::basic(
            "min_max_all",
            "DC V: press MIN/MAX until MIN, MAX and AVG all show, then Enter. Hold MIN/MAX \
             until it beeps afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(MIN_MAX_ALL)),
        CaptureStep::basic(
            "record",
            "DC V: hold MEM until it beeps and MEM flashes (logging to the micro SD card; \
             skip if none is fitted), then Enter. Hold MEM again afterwards to stop.",
        )
        .wait_for_enter()
        .expect(Expect::new().flags(RECORD_ON)),
        CaptureStep::basic(
            "setup_temp",
            "DC V: press SETUP once (the secondary display shows the meter's internal \
             temperature), then Enter.",
        )
        .wait_for_enter(),
        CaptureStep::basic(
            "setup_battery",
            "DC V: press SETUP again until bAt shows, then Enter.",
        )
        .wait_for_enter(),
        // What the packet holds for a blank secondary display is open
        // (spec §14.14).
        CaptureStep::basic(
            "setup_blank",
            "DC V: press SETUP until the secondary display is blank (or turn the dial to \
             Low Z and back to V), then Enter.",
        )
        .wait_for_enter(),
        key(
            "key_hold",
            "DC V: we will send HOLD. Press HOLD until A- and HOLD are gone afterwards.",
            "hold",
        )
        .expect(Expect::new().flags(HOLD_ON)),
        key(
            "key_rel",
            "DC V: we will send REL. Press REL again afterwards.",
            "rel",
        )
        .expect(Expect::new().flags(REL_ON)),
        key("key_minmax", "DC V: we will send MIN/MAX.", "minmax"),
        key(
            "key_exit_minmax",
            "DC V with MIN, MAX or AVG showing (press MIN/MAX if none does): we will send a long \
             MIN/MAX to leave it.",
            "exit_minmax",
        )
        .expect(Expect::new().flags(MIN_MAX_OFF)),
        key(
            "key_light",
            "DC V: we will send a long MODE (backlight). Hold MODE afterwards to switch the \
             backlight back.",
            "light",
        ),
        CaptureStep::basic("manual_range", "DC V: press RANGE, then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").range(RangeExpect::Manual)),
        key(
            "key_range",
            "DC V, manual range: we will send RANGE.",
            "range",
        )
        .expect(Expect::mode("DC V").range(RangeExpect::Manual)),
        // The manual says how to leave auto-ranging (RANGE, p.33) but not
        // how to come back to it, so this step only asks.
        CaptureStep::basic(
            "range_auto",
            "DC V: turn the dial to Low Z and back to V, press MODE until DC shows, then \
             Enter. Does AUTO show again?",
        )
        .wait_for_enter(),
        key(
            "key_select",
            "DC V: we will send MODE (AC should show).",
            "select",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acv", "V position: press MODE until AC shows, leads open")
            .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "peak",
            "AC V: press 1ms PEAK briefly (holding it switches Bluetooth off), then Enter. \
             Afterwards press 1ms PEAK briefly, again if needed, until 1ms is gone.",
        )
        .wait_for_enter()
        .expect(Expect::mode("AC V").flags(PEAK_ON)),
        key(
            "key_peak",
            "AC V: we will send a short 1ms PEAK. Afterwards press 1ms PEAK briefly, again if \
             needed, until 1ms is gone.",
            "peak",
        )
        .expect(Expect::mode("AC V").flags(PEAK_ON)),
        CaptureStep::basic(
            "lpf",
            "AC V: hold REL until 1kHz shows, then Enter. Hold REL until 1kHz is gone \
             afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("LPF V")),
        key(
            "key_lpf",
            "AC V: we will send a long REL (1 kHz filter). Hold REL until 1kHz is gone \
             afterwards.",
            "lpf",
        )
        .expect(Expect::mode("LPF V")),
        CaptureStep::basic(
            "dbm",
            "AC V: press SETUP until dBm shows at the top right, then Enter. Press SETUP \
             until dBm is gone afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("AC V")),
        CaptureStep::basic("acdcv", "V position: press MODE until DC+AC shows")
            .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set the dial to mV and press MODE until DC shows")
            .expect(Expect::mode("DC mV")),
        CaptureStep::basic("acmv", "mV position: press MODE until AC shows")
            .expect(Expect::mode("AC mV")),
        // The c/F setting picks the unit (spec §6.4), so either may show.
        CaptureStep::basic(
            "temp",
            "mV position: press MODE until °C or °F shows (temperature; K-type thermocouple \
             in VΩ and COM)",
        )
        .needs(&[Need::Thermocouple]),
        CaptureStep::basic(
            "temp_unit",
            "Temperature: press SETUP once, hold SETUP until the display flashes, press REL \
             (▲) to switch between \"c\" (Celsius) and \"F\" (Fahrenheit), hold SETUP until it stops flashing, then Enter. \
             Afterwards switch it back the same way and press SETUP until the secondary \
             display clears.",
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter(),
        CaptureStep::basic(
            "hz",
            "Set the dial to Hz and press MODE until Hz shows (not ms or %)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "pulse_width",
            "Hz position: press MODE once for pulse width (ms)",
        )
        .expect(Expect::mode("Pulse Width")),
        CaptureStep::basic("duty", "Hz position: press MODE again for duty cycle (%)")
            .expect(Expect::mode("Duty %")),
        CaptureStep::basic(
            "cont",
            "Set the dial to Ω and press MODE until the continuity symbol shows; touch the \
             probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "diode",
            "Ω position: press MODE again for diode, leads open",
        )
        .expect(Expect::mode("Diode")),
        CaptureStep::basic(
            "diode_15v",
            "Diode: press RANGE for the 15 V test (the secondary display shows 15 V), then \
             Enter. Press RANGE again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Diode")),
        CaptureStep::basic(
            "cap",
            "Ω position: press MODE again for capacitance, leads open",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "dcva",
            "Set the dial to mVA/VA and move the red lead to A 500mA, leads open; press MODE \
             until DC shows (skip if it never does)",
        )
        .expect(Expect::mode("DC VA")),
        CaptureStep::basic(
            "acva",
            "mVA/VA position: press MODE until AC shows (skip if it never does)",
        )
        .expect(Expect::mode("AC VA")),
        CaptureStep::basic(
            "dcmva",
            "mVA/VA position: move the red lead to mA µA, leads open; press MODE until DC \
             shows (skip if it never does)",
        )
        .expect(Expect::mode("DC mVA")),
        CaptureStep::basic(
            "acmva",
            "mVA/VA position: press MODE until AC shows (skip if it never does)",
        )
        .expect(Expect::mode("AC mVA")),
        CaptureStep::basic(
            "dcua",
            "Set the dial to µA (red lead in mA µA) and press MODE until DC shows, leads open",
        )
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic(
            "burden",
            "DC µA: connect an extra lead from the mA µA jack to VΩ (skip if you have none), \
             hold SETUP until bd flashes, press REL (▲) to switch it on, hold SETUP until it \
             stops flashing, press SETUP to show the burden voltage, then Enter. Switch bd \
             off the same way and remove the extra lead afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic("acua", "µA position: press MODE until AC shows")
            .expect(Expect::mode("AC µA")),
        CaptureStep::basic(
            "dcuva",
            "µA position: press MODE until µVA and DC show (skip if it never does)",
        )
        .expect(Expect::mode("DC µVA")),
        CaptureStep::basic(
            "acuva",
            "µA position: press MODE until µVA and AC show (skip if it never does)",
        )
        .expect(Expect::mode("AC µVA")),
        CaptureStep::basic(
            "dcma",
            "Set the dial to A/mA (red lead in mA µA) and press MODE until DC shows, leads \
             open",
        )
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic("acma", "A/mA position: press MODE until AC shows")
            .expect(Expect::mode("AC mA")),
        CaptureStep::basic(
            "dca",
            "A/mA position: move the red lead to A 500mA, leads open, and press MODE until \
             DC shows",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "A/mA position: press MODE until AC shows")
            .expect(Expect::mode("AC A")),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::{COMMANDS, tables};
    use super::*;

    /// Every mode code has a step that expects it, bar temperature: it is
    /// named °C or °F after the meter's c/F setting (spec §6.4), so its
    /// step expects neither.
    #[test]
    fn every_mode_is_reached() {
        let steps = steps();
        let expected: Vec<&str> = steps.iter().filter_map(|s| s.expect?.mode).collect();
        for mode in &tables::MODES {
            if mode.name == "°C" {
                assert!(steps.iter().any(|s| s.id == "temp"));
                continue;
            }
            assert!(expected.contains(&mode.name), "{}", mode.name);
        }
        for derived in ["AC+DC V", "LPF V"] {
            assert!(expected.contains(&derived), "{derived}");
        }
    }

    #[test]
    fn key_steps_send_offered_keys_and_every_key_has_one() {
        let steps = steps();
        for step in &steps {
            if let Some(command) = step.command {
                assert!(COMMANDS.contains(&command), "{}", step.id);
            }
        }
        for command in COMMANDS {
            assert!(
                steps.iter().any(|s| s.command == Some(command)),
                "{command}"
            );
        }
    }

    #[test]
    fn the_gate_leads_and_nothing_is_verified() {
        let steps = steps();
        assert!(steps[..6].iter().all(|s| s.gate));
        assert!(steps[6..].iter().all(|s| !s.gate));
        assert!(steps.iter().all(|s| !s.verified));
        let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), steps.len(), "ids are unique");
    }

    /// Held, 1ms PEAK switches Bluetooth off (manual p.33): every step that
    /// names it says to press it briefly.
    #[test]
    fn no_step_holds_1ms_peak() {
        for step in steps() {
            if step.instruction.contains("press 1ms PEAK")
                || step.instruction.contains("Press 1ms PEAK")
            {
                assert!(step.instruction.contains("briefly"), "{}", step.id);
            }
            assert!(!step.instruction.contains("hold 1ms PEAK"), "{}", step.id);
            assert!(!step.instruction.contains("Hold 1ms PEAK"), "{}", step.id);
        }
    }
}
