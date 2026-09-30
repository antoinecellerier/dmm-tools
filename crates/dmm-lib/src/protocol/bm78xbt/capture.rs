//! The BM78xBT's capture steps, worded from the BM788BT manual.
//!
//! Nothing here has run on a meter. Neither manual draws the BM788BT's or
//! the BM787BT's dial (spec §9: their figures show a representative model),
//! so the steps follow the manual's sections in order — AutoV, ACV, DCV, mV,
//! Ω, capacitance, temperature, A/mA, µA (BM788BT manual p.5-15) — with the
//! keys' own sections (p.15-18) in the voltage steps they act in, the gate
//! first and the Beep-Jack warning (p.18) last. SELECT steps through a
//! position's functions. Holding Δ switches Bluetooth off (p.19), so no step
//! holds it. "Skip on a BM787BT" marks what only the BM788BT has, T2, T1-T2
//! and %4-20mA; nS, which the BM787BT manual gives both ways (spec §9.5), is
//! skipped if it never shows.

use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
const REL_ON: &[(Flag, bool)] = &[(Flag::Rel, true)];
const MANUAL_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, false)];
const AUTO_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, true)];
const CREST_MAX: &[(Flag, bool)] = &[(Flag::PeakMax, true), (Flag::PeakMin, false)];
const CREST_MIN: &[(Flag, bool)] = &[(Flag::PeakMin, true), (Flag::PeakMax, false)];
const REC_ALL: &[(Flag, bool)] = &[
    (Flag::Record, true),
    (Flag::Max, true),
    (Flag::Min, true),
    (Flag::Avg, true),
];
const REC_MAX: &[(Flag, bool)] = &[
    (Flag::Record, true),
    (Flag::Max, true),
    (Flag::Min, false),
    (Flag::Avg, false),
];
const REC_MIN: &[(Flag, bool)] = &[
    (Flag::Record, true),
    (Flag::Max, false),
    (Flag::Min, true),
    (Flag::Avg, false),
];
const REC_AVG: &[(Flag, bool)] = &[
    (Flag::Record, true),
    (Flag::Max, false),
    (Flag::Min, false),
    (Flag::Avg, true),
];
const INPUT_WARNING: &[(Flag, bool)] = &[(Flag::LeadError, true)];

pub(super) fn steps() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in V and COM: set the dial to V⎓ and press SELECT until ⎓ shows without \
             ~, leads open",
        ),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm_ol",
            "Set the dial to Ω and press SELECT until Ω shows without the continuity symbol \
             or nS, leads open (over-range, 0L shows)",
        ),
    );
    vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm_ol,
        ohm_body,
        ohm_short,
        // AutoV.
        CaptureStep::basic(
            "autov",
            "Set the dial to Auto V/LoZ, leads open (Auto shows)",
        )
        .expect(Expect::mode("Auto V").value(ValueExpect::NoReading)),
        CaptureStep::basic(
            "autov_dc",
            "Auto V/LoZ: leads on a battery or other DC source above 1 V (skip if none)",
        )
        .needs(&[Need::DcSource])
        .expect(Expect::mode("LoZ DC V")),
        CaptureStep::basic(
            "autov_ac",
            "Auto V/LoZ: leads on a low-voltage AC source above 1 V, such as a transformer's \
             output (skip if none)",
        )
        .expect(Expect::mode("LoZ AC V")),
        // ACV, VFD-ACV, ~Hz, CREST, EF.
        CaptureStep::basic(
            "acv",
            "Set the dial to V~ and press SELECT until ~ shows without VFD, leads open",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "line_hz",
            "ACV: press ~Hz briefly (Hz shows), then Enter. Hold ~Hz for one second or more \
             afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Line Hz")),
        CaptureStep::basic(
            "crest_max",
            "ACV: press CREST briefly (C and MAX show), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("AC V").flags(CREST_MAX)),
        CaptureStep::basic(
            "crest_min",
            "CREST: press CREST briefly again (C and MIN show), then Enter. Hold CREST for one \
             second or more afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("AC V").flags(CREST_MIN)),
        CaptureStep::basic("vfd_acv", "V~ position: press SELECT until VFD shows")
            .expect(Expect::mode("VFD AC V")),
        CaptureStep::basic(
            "vfd_hz",
            "VFD-ACV: press ~Hz briefly (Hz shows), then Enter. Hold ~Hz for one second or \
             more afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("VFD Hz")),
        CaptureStep::basic(
            "ef_h",
            "Press EF briefly, away from any wire (EF-H shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("EF-H")),
        // Non-contact only: the single-probe contact mode touches a live
        // conductor (BM788BT manual p.15-16).
        CaptureStep::basic(
            "ncv",
            "EF-H: hold the meter's top-right corner near a live wire without touching it \
             (dashes show), then Enter.",
        )
        .needs(&[Need::LiveWire])
        .wait_for_enter()
        .expect(Expect::mode("EF-H").value(ValueExpect::NcvDetected)),
        CaptureStep::basic(
            "ef_l",
            "EF-H: press EF briefly, away from any wire (EF-L shows), then Enter. Hold EF for \
             one second or more afterwards to leave EF-Detection.",
        )
        .wait_for_enter()
        .expect(Expect::mode("EF-L")),
        // DCV, DC+ACV, and the keys that act there.
        CaptureStep::basic(
            "acdcv",
            "Set the dial to V⎓ and press SELECT until ⎓ and ~ both show",
        )
        .expect(Expect::mode("AC+DC V")),
        CaptureStep::basic(
            "rel",
            "V⎓ position: press SELECT until ⎓ shows without ~, then press Δ briefly (Δ \
             shows), then Enter. Press Δ briefly again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
        CaptureStep::basic(
            "hold",
            "DCV: press HOLD briefly (H shows), then Enter. Press HOLD briefly again \
             afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "autohold",
            "DCV, leads open: hold HOLD for one second or more (A-H and - - - - - show), then \
             Enter. Hold HOLD for one second or more again afterwards.",
        )
        .wait_for_enter()
        .expect(
            Expect::mode("DC V")
                .flags(HOLD_ON)
                .value(ValueExpect::NoReading),
        ),
        CaptureStep::basic(
            "manual_range",
            "DCV: press RANGE briefly (AUTO goes off), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "range_auto",
            "DCV: hold RANGE for one second or more (AUTO shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(AUTO_RANGE)),
        CaptureStep::basic(
            "rec",
            "DCV: press REC briefly (R, MAX, AVG and MIN show), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REC_ALL)),
        CaptureStep::basic("rec_max", "REC: press REC briefly (MAX shows), then Enter.")
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MAX)),
        CaptureStep::basic(
            "rec_min",
            "REC: press REC briefly again (MIN shows), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REC_MIN)),
        CaptureStep::basic(
            "rec_avg",
            "REC: press REC briefly again (AVG shows), then Enter. Hold REC for one second \
             or more afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REC_AVG)),
        // mV.
        CaptureStep::basic(
            "dcmv",
            "Set the dial to mV and press SELECT until ⎓ shows without ~",
        )
        .expect(Expect::mode("DC mV")),
        CaptureStep::basic("acmv", "mV position: press SELECT until ~ shows without ⎓")
            .expect(Expect::mode("AC mV")),
        CaptureStep::basic(
            "acdcmv",
            "mV position: press SELECT until ⎓ and ~ both show",
        )
        .expect(Expect::mode("AC+DC mV")),
        CaptureStep::basic(
            "logic_hz",
            "mV position: press SELECT until Hz shows (logic-level Hz)",
        )
        .expect(Expect::mode("Logic Hz")),
        CaptureStep::basic(
            "duty",
            "mV position: press SELECT until % shows (logic-level duty %)",
        )
        .expect(Expect::mode("Duty %")),
        // Ω.
        CaptureStep::basic(
            "cont",
            "Set the dial to Ω and press SELECT until the continuity symbol shows; touch the \
             probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "ns",
            "Ω position: press SELECT until nS shows, leads open (skip if it never shows)",
        )
        .expect(Expect::mode("nS")),
        // Capacitance.
        CaptureStep::basic(
            "cap",
            "Set the dial to capacitance and press SELECT until the diode symbol is gone, \
             leads open",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "diode",
            "Capacitance position: press SELECT for diode, leads open",
        )
        .expect(Expect::mode("Diode")),
        // Temperature.
        CaptureStep::basic(
            "t1",
            "Set the dial to temperature (T1 T2) with a type-K thermocouple in V (+) and COM (−), and press \
             RANGE until T1 shows",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "temp_unit",
            "Temperature T1: press SELECT briefly to switch between °C and °F (if both are enabled), then \
             Enter. Press SELECT briefly again afterwards.",
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter()
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t2",
            "Temperature T1: move the thermocouple to µA mA (+) and A (−) and press RANGE until T2 shows \
             (skip on a BM787BT)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T2")),
        CaptureStep::basic(
            "t1_t2",
            "Temperature T2: add a second thermocouple in V (+) and COM (−) and press RANGE until T1-T2 \
             shows (skip on a BM787BT)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1-T2")),
        // A.
        CaptureStep::basic(
            "dca",
            "Set the dial to A/mA with the red lead in A and the black in COM, leads open; \
             press SELECT until ⎓ shows without ~",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "aca",
            "A/mA position, lead in A: press SELECT until ~ shows without ⎓",
        )
        .expect(Expect::mode("AC A")),
        CaptureStep::basic(
            "acdca",
            "A/mA position, lead in A: press SELECT until ⎓ and ~ both show",
        )
        .expect(Expect::mode("AC+DC A")),
        CaptureStep::basic(
            "line_hz_a",
            "A/mA position, lead in A: press ~Hz briefly (Hz shows), then Enter. Hold ~Hz for \
             one second or more afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("Line Hz")),
        // µA.
        CaptureStep::basic(
            "dcua",
            "Set the dial to µA and move the red lead to µA mA, leads open; press SELECT \
             until ⎓ shows without ~",
        )
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic("acua", "µA position: press SELECT until ~ shows without ⎓")
            .expect(Expect::mode("AC µA")),
        CaptureStep::basic(
            "acdcua",
            "µA position: press SELECT until ⎓ and ~ both show",
        )
        .expect(Expect::mode("AC+DC µA")),
        // mA.
        CaptureStep::basic(
            "dcma",
            "Set the dial back to A/mA, red lead still in µA mA, leads open; press SELECT \
             until ⎓ shows without ~",
        )
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic(
            "acma",
            "A/mA position, lead in µA mA: press SELECT until ~ shows without ⎓",
        )
        .expect(Expect::mode("AC mA")),
        CaptureStep::basic(
            "acdcma",
            "A/mA position, lead in µA mA: press SELECT until ⎓ and ~ both show",
        )
        .expect(Expect::mode("AC+DC mA")),
        CaptureStep::basic(
            "ma420",
            "A/mA position, lead in µA mA: press SELECT until %4-20mA shows (skip on a \
             BM787BT)",
        )
        .expect(Expect::mode("% 4-20mA")),
        // Beep-Jack input warning.
        CaptureStep::basic(
            "iner",
            "Red lead still in µA mA: set the dial to V⎓, leads open (the meter beeps and \
             shows InEr)",
        )
        .expect(
            Expect::new()
                .flags(INPUT_WARNING)
                .value(ValueExpect::NoReading),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::tables;
    use super::*;

    /// Every function the table holds has a step that expects it, but LoZ
    /// Ω, the app's code no manual function matches (spec §6.5, §9.5).
    #[test]
    fn every_mode_is_reached() {
        let steps = steps();
        let expected: Vec<&str> = steps.iter().filter_map(|s| s.expect?.mode).collect();
        for name in tables::names() {
            if name == "LoZ Ω" {
                continue;
            }
            assert!(expected.contains(&name), "{name}");
        }
    }

    #[test]
    fn the_gate_leads_and_nothing_is_verified() {
        let steps = steps();
        assert!(steps[..6].iter().all(|s| s.gate));
        assert!(steps[6..].iter().all(|s| !s.gate));
        assert!(steps.iter().all(|s| !s.verified));
        assert!(steps.iter().all(|s| s.command.is_none()), "no remote keys");
        let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), steps.len(), "ids are unique");
    }

    /// Held, Δ switches Bluetooth off (manual p.19): every step that names
    /// it says to press it briefly.
    #[test]
    fn no_step_holds_delta() {
        for step in steps() {
            if step.instruction.contains('Δ') {
                assert!(step.instruction.contains("Δ briefly"), "{}", step.id);
            }
            assert!(!step.instruction.contains("hold Δ"), "{}", step.id);
            assert!(!step.instruction.contains("Hold Δ"), "{}", step.id);
        }
    }

    /// Never the leads on mains: live wires are for the non-contact EF
    /// step alone.
    #[test]
    fn only_the_non_contact_step_needs_a_live_wire() {
        let live: Vec<&str> = steps()
            .iter()
            .filter(|s| s.needs.contains(&Need::LiveWire))
            .map(|s| s.id)
            .collect();
        assert_eq!(live, ["ncv"]);
    }
}
