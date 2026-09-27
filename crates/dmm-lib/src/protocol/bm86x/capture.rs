//! The capture steps, worded from the BM860s manual.
//!
//! Nothing here has run on a meter. The manual draws the BM869s (p.1); the
//! steps follow its dial from VFD round to the current positions (p.6-12),
//! with the keys (p.13-14) at DCV, the gate first and the Beep-Jack warning
//! (p.14) last. The current steps go µA, mA, A, the order the leads move
//! in: µA and mA share a jack (p.1). "Skip on a BM867s" marks what only the
//! BM869s has, VFD and temperature (spec §11.1). No step touches mains: the
//! AC steps read open leads.

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

/// The BM869s and BM867s.
pub(super) fn bm86x() -> Vec<CaptureStep> {
    let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic(
            "dcv",
            "Leads in ΩV and COM: set the dial to V⎓ and press SELECT until ⎓ shows with \
             nothing on the small display, leads open",
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
        // VFD (p.6).
        CaptureStep::basic(
            "vfd_acv",
            "Set the dial to VFD, leads open, and press SELECT until the main display shows V \
             and the small one Hz (skip on a BM867s)",
        )
        .expect(Expect::mode("VFD AC V").flags(MANUAL_RANGE)),
        CaptureStep::basic(
            "vfd_hz",
            "VFD position: press SELECT until the main display shows Hz and the small one V \
             (skip on a BM867s)",
        )
        .expect(Expect::mode("VFD Hz")),
        // ACV, dBm, Hz (p.7).
        CaptureStep::basic(
            "acv",
            "Set the dial to Hz dBm V~, leads open, and press SELECT until the main display \
             shows ~ and V and the small one Hz",
        )
        .expect(Expect::mode("AC V")),
        CaptureStep::basic(
            "dbm",
            "Hz dBm V~ position: press SELECT until dBm shows (the reference impedance shows \
             for a second first)",
        )
        .expect(Expect::mode("dBm")),
        CaptureStep::basic(
            "hz_acv",
            "Hz dBm V~ position: press SELECT until the main display shows Hz and the small \
             one V",
        )
        .expect(Expect::mode("Hz")),
        // DCV +ACV, DC+ACV +ACV (p.8).
        CaptureStep::basic(
            "dcv_acv",
            "Set the dial to V⎓, leads open, and press SELECT until the main display shows ⎓ \
             and the small one ~ and V, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V")),
        CaptureStep::basic(
            "acdcv",
            "V⎓ position: press SELECT until the main display shows ⎓ and ~ both",
        )
        .expect(Expect::mode("AC+DC V")),
        // The keys, at DCV (p.13-14).
        CaptureStep::basic(
            "hold",
            "V⎓ position: press SELECT until ⎓ shows with nothing on the small display, then \
             press HOLD briefly (H shows), then Enter. Press HOLD briefly again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(HOLD_ON)),
        CaptureStep::basic(
            "rel",
            "DCV: press Δ briefly (Δ shows), then Enter. Press Δ briefly again afterwards.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(REL_ON)),
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
            "DCV: press REC briefly (R and MAX MIN AVG show), then Enter.",
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
        CaptureStep::basic(
            "crest_max",
            "DCV: press CREST briefly (C and MAX show), then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(CREST_MAX)),
        CaptureStep::basic(
            "crest_min",
            "CREST: press CREST briefly again (MIN shows), then Enter. Hold CREST for one \
             second or more afterwards to leave it.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(CREST_MIN)),
        CaptureStep::basic(
            "counts_500000",
            "DCV: hold Δ for one second or more until six digits show (readings slow to 1.25 \
             a second), then Enter. Hold Δ for one second or more again afterwards to return \
             to 50000 counts.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").value(ValueExpect::Finite)),
        // mV⎓, logic Hz, duty (p.8).
        CaptureStep::basic(
            "dcmv",
            "Set the dial to mV⎓, leads open, and press SELECT until ⎓ shows with nothing on \
             the small display, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC mV")),
        CaptureStep::basic(
            "dcmv_acmv",
            "mV⎓ position: press SELECT until the main display shows ⎓ and the small one ~ \
             and mV",
        )
        .expect(Expect::mode("DC mV")),
        CaptureStep::basic(
            "acdcmv",
            "mV⎓ position: press SELECT until the main display shows ⎓ and ~ both",
        )
        .expect(Expect::mode("AC+DC mV")),
        CaptureStep::basic(
            "logic_hz",
            "mV⎓ position: press SELECT until Hz shows (logic-level Hz)",
        )
        .expect(Expect::mode("Hz")),
        CaptureStep::basic(
            "duty",
            "mV⎓ position: press SELECT until D% shows (duty cycle)",
        )
        .expect(Expect::mode("Duty %")),
        // mV~, dBm, Hz (p.9).
        CaptureStep::basic(
            "acmv",
            "Set the dial to Hz dBm mV~, leads open, and press SELECT until the main display \
             shows ~ and mV and the small one Hz",
        )
        .expect(Expect::mode("AC mV")),
        CaptureStep::basic(
            "dbm_mv",
            "Hz dBm mV~ position: press SELECT until dBm shows (the reference impedance shows \
             for a second first)",
        )
        .expect(Expect::mode("dBm")),
        CaptureStep::basic(
            "hz_acmv",
            "Hz dBm mV~ position: press SELECT until the main display shows Hz and the small \
             one mV",
        )
        .expect(Expect::mode("Hz")),
        // Temperature (p.9). What an open input shows is in no manual
        // (spec §11.6), so only the function is expected there.
        CaptureStep::basic(
            "t1_open",
            "Unplug the test leads, then set the dial to T1 T2 and press RANGE until T1 shows \
             with nothing on the small display (skip on a BM867s)",
        )
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t1",
            "Temperature: plug a type-K thermocouple into ΩV (+) and COM (−) (skip on a \
             BM867s)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "temp_unit",
            "Temperature T1: press SELECT briefly to switch between °C and °F, then Enter. \
             Press SELECT briefly again afterwards (skip on a BM867s).",
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter()
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t2",
            "Temperature: move the thermocouple to mA µA (+) and A (−) and press RANGE until \
             T2 shows (skip on a BM867s)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T2")),
        CaptureStep::basic(
            "t1_t2",
            "Temperature: add a second thermocouple in ΩV (+) and COM (−) and press RANGE \
             until the main display shows T1 and the small one T2 (skip on a BM867s, or with \
             only one)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t1_minus_t2",
            "Temperature, both thermocouples in: press RANGE until T1-T2 shows (skip on a \
             BM867s, or with only one)",
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1-T2")),
        // Capacitance and diode (p.10).
        CaptureStep::basic(
            "cap",
            "Unplug the thermocouples and put the leads back in ΩV and COM, leads open; set \
             the dial to capacitance (on a BM869s, press SELECT until diod is gone). A reading \
             can take seconds.",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "diode",
            "Diode, leads open: on a BM869s press SELECT (diod shows on the small display); on \
             a BM867s turn the dial to diode",
        )
        .expect(Expect::mode("Diode")),
        // Ω, continuity, nS (p.11).
        CaptureStep::basic(
            "cont",
            "Set the dial to Ω and press SELECT until the continuity symbol shows; touch the \
             probe tips together",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic("ns", "Ω position: press SELECT until nS shows, leads open")
            .expect(Expect::mode("nS")),
        // µA (p.11-12).
        CaptureStep::basic(
            "dcua",
            "Move the red lead to mA µA, leads open; set the dial to µA and press SELECT until \
             ⎓ shows with nothing on the small display, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic(
            "acdcua",
            "µA position: press SELECT until the main display shows ⎓ and ~ both",
        )
        .expect(Expect::mode("AC+DC µA")),
        CaptureStep::basic(
            "acua",
            "µA position: press SELECT until the main display shows ~ and the small one Hz",
        )
        .expect(Expect::mode("AC µA")),
        // mA, with the loop percentage on DC.
        CaptureStep::basic(
            "dcma",
            "Red lead still in mA µA: set the dial to A mA and press SELECT until ⎓ shows and \
             the small display shows %4-20mA, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic(
            "acdcma",
            "A mA position, lead in mA µA: press SELECT until the main display shows ⎓ and ~ \
             both",
        )
        .expect(Expect::mode("AC+DC mA")),
        CaptureStep::basic(
            "acma",
            "A mA position, lead in mA µA: press SELECT until the main display shows ~ and \
             the small one Hz",
        )
        .expect(Expect::mode("AC mA")),
        // A.
        CaptureStep::basic(
            "dca",
            "Move the red lead to A, leads open; A mA position: press SELECT until ⎓ shows \
             with nothing on the small display",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "acdca",
            "A mA position, lead in A: press SELECT until the main display shows ⎓ and ~ both",
        )
        .expect(Expect::mode("AC+DC A")),
        CaptureStep::basic(
            "aca",
            "A mA position, lead in A: press SELECT until the main display shows ~ and the \
             small one Hz",
        )
        .expect(Expect::mode("AC A")),
        // Beep-Jack input warning (p.14).
        CaptureStep::basic(
            "iner",
            "Red lead still in A: set the dial to V⎓, leads open (the meter beeps and shows \
             InEr)",
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
    use super::super::decode::MODES;
    use super::*;

    /// Every mode the decoder names has a step that expects it, but the
    /// dBm reference impedance, which shows for a second (BM860s manual
    /// p.7).
    #[test]
    fn every_mode_is_reached() {
        let steps = bm86x();
        let expected: Vec<&str> = steps.iter().filter_map(|s| s.expect?.mode).collect();
        for mode in MODES {
            if mode == "dBm reference" {
                continue;
            }
            assert!(expected.contains(&mode), "{mode}");
        }
        for mode in &expected {
            assert!(MODES.contains(mode), "{mode}");
        }
    }

    #[test]
    fn the_gate_leads_and_nothing_is_verified() {
        let steps = bm86x();
        assert!(steps[..6].iter().all(|s| s.gate));
        assert!(steps[6..].iter().all(|s| !s.gate));
        assert!(steps.iter().all(|s| !s.verified));
        assert!(steps.iter().all(|s| s.command.is_none()), "no remote keys");
        let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), steps.len(), "ids are unique");
    }

    /// Never the leads on mains, and no live wire.
    #[test]
    fn no_step_needs_a_live_wire() {
        assert!(bm86x().iter().all(|s| !s.needs.contains(&Need::LiveWire)));
    }

    /// The BM867s has no VFD or temperature function (spec §11.1): every
    /// step at those positions says to skip it there.
    #[test]
    fn bm869s_only_steps_say_so() {
        for step in bm86x() {
            let at = step.instruction;
            if at.contains("VFD") || at.contains("T1 T2") || at.starts_with("Temperature") {
                assert!(at.contains("skip on a BM867s"), "{}", step.id);
            }
        }
    }
}
