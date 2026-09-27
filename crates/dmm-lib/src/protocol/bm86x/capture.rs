//! The capture steps, worded from the BM860s manual and the BM820s/BM520s
//! manual.
//!
//! Nothing here has run on a meter. The BM860s manual draws the BM869s
//! (p.1); its steps follow the dial from VFD round to the current
//! positions (p.6-12), with the keys (p.13-14) at DCV, the gate first and
//! the Beep-Jack warning (p.14) last. The BM820s/BM520s manual draws the
//! BM829s and a logging model (p.1, p.4); their steps follow the dial from
//! Auto Check round to µA (p.6-12), with EF and the keys at DCV, the
//! Beep-Jack warning after the current steps and, on a logging model, the
//! logging last (p.14-16). The current steps go µA, mA, A, the order the
//! leads move in: µA and mA share a jack (BM860s manual p.1, BM820s manual
//! p.4). A function only some models of an entry have says which, and to
//! skip it (spec §11.1). No step touches mains: the AC steps read open
//! leads or a transformer's output, and EF holds the meter near a wire.

use super::Series;
use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
const REL_ON: &[(Flag, bool)] = &[(Flag::Rel, true)];
const MANUAL_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, false)];
const AUTO_RANGE: &[(Flag, bool)] = &[(Flag::AutoRange, true)];
const CREST_MAX: &[(Flag, bool)] = &[(Flag::PeakMax, true), (Flag::PeakMin, false)];
const CREST_MIN: &[(Flag, bool)] = &[(Flag::PeakMin, true), (Flag::PeakMax, false)];
const CREST_MAX_MIN: &[(Flag, bool)] = &[(Flag::PeakMax, true), (Flag::PeakMin, true)];
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
const REC_MAX_MIN: &[(Flag, bool)] = &[
    (Flag::Record, true),
    (Flag::Max, true),
    (Flag::Min, true),
    (Flag::Avg, false),
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

/// The BM829s, BM827s, BM822s and BM821s.
pub(super) fn bm82x() -> Vec<CaptureStep> {
    bm820s(Series::Bm82x)
}

/// The BM525s and BM521s.
pub(super) fn bm52x() -> Vec<CaptureStep> {
    bm820s(Series::Bm52x)
}

/// The two series of the BM820s/BM520s manual: one dial, one keypad but
/// for the logging keys, and functions by model (spec §11.1, §11.3). The
/// logging models have AutoCheck, DC+AC V and mV, nS, T1, REC and CREST
/// on both; no dBm and no EF; REC without AVG; T2 on the BM525s alone.
fn bm820s(series: Series) -> Vec<CaptureStep> {
    let logs = series == Series::Bm52x;
    // The wording for each series, where the models that have a function
    // differ.
    let pick = |bm82x: &'static str, bm52x: &'static str| if logs { bm52x } else { bm82x };
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
            "Set the dial to Ω, leads open, and press SELECT if needed until Ω shows without \
             the continuity symbol or nS (over-range, 0L shows)",
        ),
    );
    let mut list = vec![
        dcv,
        dcv_short,
        dcv_negative,
        ohm_ol,
        ohm_body,
        ohm_short,
        // AutoCheck, below OFF (p.6-7).
        CaptureStep::basic(
            "autocheck",
            pick(
                "Set the dial to Auto Check, leads open (Auto shows; BM829s only; skip if your \
                 meter lacks it)",
                "Set the dial to Auto Check, leads open (Auto shows)",
            ),
        )
        .expect(Expect::mode("Auto V").value(ValueExpect::NoReading)),
        CaptureStep::basic(
            "autocheck_dc",
            pick(
                "Auto Check: leads on a DC source above 1.5 V, such as a 9 V battery (BM829s \
                 only; skip if your meter lacks it)",
                "Auto Check: leads on a DC source above 1.5 V, such as a 9 V battery",
            ),
        )
        .needs(&[Need::DcSource])
        .expect(Expect::mode("LoZ DC V")),
        CaptureStep::basic(
            "autocheck_ac",
            pick(
                "Auto Check: leads on a low-voltage AC source above 3 V, such as a transformer's \
                 output (BM829s only; skip if your meter lacks it, or if you have none)",
                "Auto Check: leads on a low-voltage AC source above 3 V, such as a transformer's \
                 output (skip if none)",
            ),
        )
        .expect(Expect::mode("LoZ AC V")),
        CaptureStep::basic(
            "autocheck_ohm",
            pick(
                "Auto Check: hold one probe tip between the fingers of each hand (BM829s only; \
                 skip if your meter lacks it)",
                "Auto Check: hold one probe tip between the fingers of each hand",
            ),
        )
        .expect(Expect::mode("LoZ Ω")),
        // ACV, dBm, Hz (p.7-8).
        CaptureStep::basic(
            "acv",
            "Set the dial to Hz V~, leads open, and press SELECT until the main display shows ~ \
             and V and the small one Hz",
        )
        .expect(Expect::mode("AC V")),
    ];
    if !logs {
        list.push(
            CaptureStep::basic(
                "dbm",
                "Hz dBm V~ position: press SELECT until dBm shows (the reference impedance shows \
                 for a second first; BM829s only; skip if your meter lacks it)",
            )
            .expect(Expect::mode("dBm")),
        );
    }
    list.extend([
        CaptureStep::basic(
            "hz_acv",
            "Hz V~ position: press SELECT until the main display shows Hz and the small one V",
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
            pick(
                "V⎓ position: press SELECT until the main display shows ⎓ and ~ both (BM829s \
                 only; skip if your meter lacks it)",
                "V⎓ position: press SELECT until the main display shows ⎓ and ~ both",
            ),
        )
        .expect(Expect::mode("AC+DC V")),
    ]);
    if !logs {
        // EF, non-contact only: the probe-contact mode touches a live
        // conductor (p.12-13).
        list.extend([
            CaptureStep::basic(
                "ef",
                "V⎓ position: hold HOLD (EF) for one second or more, away from any wire (E.F. \
                 shows), then Enter (BM827s and BM829s only; skip if your meter lacks it).",
            )
            .wait_for_enter()
            .expect(Expect::mode("EF")),
            CaptureStep::basic(
                "ncv",
                "EF: hold the top of the meter near a live wire, probe tips away from it, \
                 without touching it (dashes show), then Enter. Hold HOLD for one second or more \
                 afterwards to leave EF (BM827s and BM829s only; skip if your meter lacks it).",
            )
            .needs(&[Need::LiveWire])
            .wait_for_enter()
            .expect(Expect::mode("EF").value(ValueExpect::NcvDetected)),
        ]);
    }
    // The keys, at DCV (p.13-14).
    list.extend([
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
    ]);
    if logs {
        list.extend([
            CaptureStep::basic(
                "rec",
                "DCV: press REC briefly (R and MAX MIN show), then Enter.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MAX_MIN)),
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
                "rec_max_min",
                "REC: press REC briefly again (MAX-MIN shows), then Enter. Hold REC for one \
                 second or more afterwards to leave it.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MAX_MIN)),
        ]);
    } else {
        list.extend([
            CaptureStep::basic(
                "rec",
                "DCV: press REC briefly (R and MAX MIN AVG show), then Enter (BM827s and \
                 BM829s only; skip this and the REC steps after it if your meter lacks it).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_ALL)),
            CaptureStep::basic(
                "rec_max",
                "REC: press REC briefly (MAX shows), then Enter (BM827s and BM829s only).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MAX)),
            CaptureStep::basic(
                "rec_min",
                "REC: press REC briefly again (MIN shows), then Enter (BM827s and BM829s only).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MIN)),
            CaptureStep::basic(
                "rec_max_min",
                "REC: press REC briefly again (MAX-MIN shows), then Enter (BM827s and BM829s \
                 only).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_MAX_MIN)),
            CaptureStep::basic(
                "rec_avg",
                "REC: press REC briefly again (AVG shows), then Enter. Hold REC for one second \
                 or more afterwards to leave it (BM827s and BM829s only).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V").flags(REC_AVG)),
        ]);
    }
    list.extend([
        CaptureStep::basic(
            "crest_max",
            pick(
                "DCV: press CREST briefly (C and MAX show), then Enter (BM827s and BM829s \
                 only; skip this and the next two steps if your meter lacks it).",
                "DCV: press CREST briefly (C and MAX show), then Enter.",
            ),
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(CREST_MAX)),
        CaptureStep::basic(
            "crest_min",
            pick(
                "CREST: press CREST briefly again (MIN shows), then Enter (BM827s and BM829s \
                 only).",
                "CREST: press CREST briefly again (MIN shows), then Enter.",
            ),
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(CREST_MIN)),
        CaptureStep::basic(
            "crest_max_min",
            pick(
                "CREST: press CREST briefly again (MAX-MIN shows), then Enter. Hold CREST for \
                 one second or more afterwards to leave it (BM827s and BM829s only).",
                "CREST: press CREST briefly again (MAX-MIN shows), then Enter. Hold CREST for \
                 one second or more afterwards to leave it.",
            ),
        )
        .wait_for_enter()
        .expect(Expect::mode("DC V").flags(CREST_MAX_MIN)),
        // mV⎓, logic Hz, duty (p.8-9).
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
            pick(
                "mV⎓ position: press SELECT until the main display shows ⎓ and ~ both (BM829s \
                 only; skip if your meter lacks it)",
                "mV⎓ position: press SELECT until the main display shows ⎓ and ~ both",
            ),
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
            "Set the dial to Hz mV~, leads open, and press SELECT until the main display shows \
             ~ and mV and the small one Hz",
        )
        .expect(Expect::mode("AC mV")),
    ]);
    if !logs {
        list.push(
            CaptureStep::basic(
                "dbm_mv",
                "Hz dBm mV~ position: press SELECT until dBm shows (the reference impedance shows \
                 for a second first; BM829s only; skip if your meter lacks it)",
            )
            .expect(Expect::mode("dBm")),
        );
    }
    list.extend([
        CaptureStep::basic(
            "hz_acmv",
            "Hz mV~ position: press SELECT until the main display shows Hz and the small one mV",
        )
        .expect(Expect::mode("Hz")),
        // Continuity, nS (p.10).
        CaptureStep::basic(
            "cont",
            pick(
                "Set the dial to the continuity symbol, or to Ω and press SELECT until the \
                 continuity symbol shows; touch the probe tips together",
                "Set the dial to Ω and press SELECT until the continuity symbol shows; touch \
                 the probe tips together",
            ),
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity")),
        CaptureStep::basic(
            "ns",
            pick(
                "Ω position: press SELECT until nS shows, leads open (BM827s and BM829s only; \
                 skip if your meter lacks it)",
                "Ω position: press SELECT until nS shows, leads open",
            ),
        )
        .expect(Expect::mode("nS")),
        // Temperature (p.10-11). What an open input shows is in no manual
        // (spec §11.6), so only the function is expected there.
        CaptureStep::basic(
            "t1_open",
            pick(
                "Unplug the test leads, then set the dial to T1 T2; on a BM829s press RANGE \
                 until T1 shows with nothing on the small display (BM827s and BM829s only; \
                 skip this and the temperature steps after it if your meter lacks it)",
                "Unplug the test leads, then set the dial to T1 T2; on a BM525s press RANGE \
                 until T1 shows with nothing on the small display",
            ),
        )
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t1",
            pick(
                "Temperature: plug a type-K thermocouple into ΩV (+) and COM (−) (BM827s and \
                 BM829s only)",
                "Temperature: plug a type-K thermocouple into ΩV (+) and COM (−)",
            ),
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "temp_unit",
            pick(
                "Temperature T1: press SELECT briefly to switch between °C and °F, then Enter. \
                 Press SELECT briefly again afterwards (BM827s and BM829s only).",
                "Temperature T1: press SELECT briefly to switch between °C and °F, then Enter. \
                 Press SELECT briefly again afterwards.",
            ),
        )
        .needs(&[Need::Thermocouple])
        .wait_for_enter()
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t2",
            pick(
                "Temperature: move the thermocouple to mA µA (+) and A (−) and press RANGE \
                 until T2 shows (BM829s only; skip if your meter lacks it)",
                "Temperature: move the thermocouple to mA µA (+) and A (−) and press RANGE \
                 until T2 shows (BM525s only; skip if your meter lacks it)",
            ),
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T2")),
        CaptureStep::basic(
            "t1_t2",
            pick(
                "Temperature: add a second thermocouple in ΩV (+) and COM (−) and press RANGE \
                 until the main display shows T1 and the small one T2 (BM829s only; skip if \
                 your meter lacks it, or with only one)",
                "Temperature: add a second thermocouple in ΩV (+) and COM (−) and press RANGE \
                 until the main display shows T1 and the small one T2 (BM525s only; skip if \
                 your meter lacks it, or with only one)",
            ),
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1")),
        CaptureStep::basic(
            "t1_minus_t2",
            pick(
                "Temperature, both thermocouples in: press RANGE until T1-T2 shows (BM829s \
                 only; skip if your meter lacks it, or with only one)",
                "Temperature, both thermocouples in: press RANGE until T1-T2 shows (BM525s \
                 only; skip if your meter lacks it, or with only one)",
            ),
        )
        .needs(&[Need::Thermocouple])
        .expect(Expect::mode("T1-T2")),
        // Capacitance and diode (p.11).
        CaptureStep::basic(
            "cap",
            "Unplug any thermocouple and put the leads back in ΩV and COM, leads open; set the \
             dial to capacitance and press SELECT until diod is gone. A reading can take \
             seconds.",
        )
        .expect(Expect::mode("Capacitance")),
        CaptureStep::basic(
            "diode",
            "Capacitance position: press SELECT until diod shows on the small display, leads \
             open",
        )
        .expect(Expect::mode("Diode")),
        // µA, mA, A (p.12).
        CaptureStep::basic(
            "dcua",
            "Move the red lead to mA µA, leads open; set the dial to µA and press SELECT until \
             ⎓ shows with nothing on the small display, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC µA")),
        CaptureStep::basic(
            "acdcua",
            pick(
                "µA position: press SELECT until the main display shows ⎓ and ~ both (skip if \
                 SELECT never shows it; it may be missing on the BM821s and BM822s)",
                "µA position: press SELECT until the main display shows ⎓ and ~ both",
            ),
        )
        .expect(Expect::mode("AC+DC µA")),
        CaptureStep::basic(
            "acua",
            "µA position: press SELECT until the main display shows ~ and the small one Hz",
        )
        .expect(Expect::mode("AC µA")),
        CaptureStep::basic(
            "dcma",
            "Red lead still in mA µA: set the dial to A mA and press SELECT until ⎓ shows with \
             nothing on the small display, then Enter.",
        )
        .wait_for_enter()
        .expect(Expect::mode("DC mA")),
        CaptureStep::basic(
            "acdcma",
            pick(
                "A mA position, lead in mA µA: press SELECT until the main display shows ⎓ and \
                 ~ both (skip if SELECT never shows it; it may be \
                 missing on the BM821s and BM822s)",
                "A mA position, lead in mA µA: press SELECT until the main display shows ⎓ and \
                 ~ both",
            ),
        )
        .expect(Expect::mode("AC+DC mA")),
        CaptureStep::basic(
            "acma",
            "A mA position, lead in mA µA: press SELECT until the main display shows ~ and the \
             small one Hz",
        )
        .expect(Expect::mode("AC mA")),
        CaptureStep::basic(
            "dca",
            "Move the red lead to A, leads open; A mA position: press SELECT until ⎓ shows with \
             nothing on the small display",
        )
        .expect(Expect::mode("DC A")),
        CaptureStep::basic(
            "acdca",
            pick(
                "A mA position, lead in A: press SELECT until the main display shows ⎓ and ~ \
                 both (skip if SELECT never shows it; it may be \
                 missing on the BM821s and BM822s)",
                "A mA position, lead in A: press SELECT until the main display shows ⎓ and ~ \
                 both",
            ),
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
    ]);
    if logs {
        // Data logging and Recall (p.14-16), last: the Yes ▲ key starts,
        // pauses, continues and stops a session. ▼ Erase at the start
        // prompt erases every logged session; with Yes ▲ at the same moment
        // it enters Recall. Each step waits for Enter: the words show for
        // half a second, or the screen before the press matches.
        list.extend([
            CaptureStep::basic(
                "log_left",
                "Move the red lead back to ΩV, leads open, dial at V⎓. Hold Yes ▲ for one \
                 second or more: LEFt shows, then the memory points left, then Enter. Do not \
                 press ▼ Erase, which erases every logged session (skip this and the logging \
                 steps after it if the meter holds logs you want to keep).",
            )
            .wait_for_enter(),
            CaptureStep::basic(
                "log_start",
                "Logging: press Yes ▲ briefly to start a session without erasing the others \
                 (t0.05 shows, then Strt, then the live reading returns), then Enter; pressing \
                 ▼ Erase instead erases every logged session (skip if the meter holds logs you \
                 want to keep).",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
            CaptureStep::basic(
                "log_pause",
                "Logging: press Yes ▲ briefly to pause (PAUS shows), then Enter (skip if the \
                 meter holds logs you want to keep).",
            )
            .wait_for_enter()
            .expect(Expect::new().value(ValueExpect::NoReading)),
            CaptureStep::basic(
                "log_continue",
                "Logging: press Yes ▲ briefly again to continue (Cont shows), then Enter (skip \
                 if the meter holds logs you want to keep).",
            )
            .wait_for_enter(),
            CaptureStep::basic(
                "log_stop",
                "Logging: hold Yes ▲ for one second or more to stop (StoP shows), then Enter \
                 (skip if the meter holds logs you want to keep).",
            )
            .wait_for_enter(),
            CaptureStep::basic(
                "recall",
                "Press Yes ▲ and ▼ Erase at the same moment for Recall: the session page P.nnn, \
                 then the last logged reading with R and C, then Enter. Turn the dial afterwards \
                 to leave Recall (skip if the meter holds logs you want to keep).",
            )
            .wait_for_enter()
            .expect(Expect::mode("Recall").value(ValueExpect::NoReading)),
        ]);
    }
    list
}

#[cfg(test)]
mod tests {
    use super::super::decode::MODES;
    use super::*;

    /// Each series' steps, and the modes the decoder names that no step of
    /// it expects: the dBm reference impedance, which shows for a second
    /// (BM860s manual p.7, BM820s manual p.8), and the functions the series
    /// lacks (spec §11.1).
    fn series() -> [(Vec<CaptureStep>, &'static [&'static str]); 3] {
        [
            (
                bm86x(),
                &[
                    "dBm reference",
                    "Recall",
                    "Auto V",
                    "EF",
                    "LoZ DC V",
                    "LoZ AC V",
                    "LoZ Ω",
                ],
            ),
            (bm82x(), &["dBm reference", "Recall", "VFD Hz", "VFD AC V"]),
            (
                bm52x(),
                &["dBm reference", "dBm", "EF", "VFD Hz", "VFD AC V"],
            ),
        ]
    }

    /// Every mode the decoder names has a step that expects it, on each
    /// series that has the function.
    #[test]
    fn every_mode_is_reached() {
        for (steps, lacking) in series() {
            let expected: Vec<&str> = steps.iter().filter_map(|s| s.expect?.mode).collect();
            for mode in MODES {
                assert_eq!(
                    expected.contains(&mode),
                    !lacking.contains(&mode),
                    "{mode} in {:?}",
                    steps.iter().map(|s| s.id).collect::<Vec<_>>()
                );
            }
            for mode in &expected {
                assert!(MODES.contains(mode), "{mode}");
            }
        }
    }

    #[test]
    fn the_gate_leads_and_nothing_is_verified() {
        for (steps, _) in series() {
            assert!(steps[..6].iter().all(|s| s.gate));
            assert!(steps[6..].iter().all(|s| !s.gate));
            assert!(steps.iter().all(|s| !s.verified));
            assert!(steps.iter().all(|s| s.command.is_none()), "no remote keys");
            let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), steps.len(), "ids are unique");
        }
    }

    /// Never the leads on mains: a live wire is for the non-contact EF
    /// step alone, on the BM82x.
    #[test]
    fn only_ef_needs_a_live_wire() {
        let live = |steps: Vec<CaptureStep>| -> Vec<&'static str> {
            steps
                .iter()
                .filter(|s| s.needs.contains(&Need::LiveWire))
                .map(|s| s.id)
                .collect()
        };
        assert!(live(bm86x()).is_empty());
        assert_eq!(live(bm82x()), ["ncv"]);
        assert!(live(bm52x()).is_empty());
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

    /// A function only some models of an entry have names them (spec
    /// §11.1); the BM52x's AutoCheck is on both its models.
    #[test]
    fn model_only_steps_say_so() {
        let says = |steps: &[CaptureStep], id: &str, models: &str| {
            let step = steps.iter().find(|s| s.id == id).expect(id);
            assert!(
                step.instruction.contains(models),
                "{id}: {}",
                step.instruction
            );
        };
        let bm82x = bm82x();
        for id in [
            "autocheck",
            "autocheck_dc",
            "autocheck_ac",
            "autocheck_ohm",
            "dbm",
            "acdcv",
            "acdcmv",
            "dbm_mv",
            "t2",
            "t1_t2",
            "t1_minus_t2",
        ] {
            says(&bm82x, id, "BM829s only; skip if your meter lacks it");
        }
        for id in ["ef", "ncv", "rec", "crest_max", "ns", "t1_open"] {
            says(&bm82x, id, "BM827s and BM829s only; skip");
        }
        let bm52x = bm52x();
        for id in ["t2", "t1_t2", "t1_minus_t2"] {
            says(&bm52x, id, "BM525s only; skip if your meter lacks it");
        }
        for step in bm52x.iter().filter(|s| s.id.starts_with("autocheck")) {
            assert!(!step.instruction.contains("skip if your"), "{}", step.id);
        }
    }

    /// The logging steps close the BM52x's run, each saying to skip it
    /// where logs are to be kept. ▼ Erase at the start prompt erases the
    /// memory (BM820s manual p.15): the two steps there warn against it,
    /// and only Recall presses it, with Yes ▲.
    #[test]
    fn logging_goes_last_and_no_step_presses_erase_alone() {
        let steps = bm52x();
        let first = steps.iter().position(|s| s.id == "log_left").unwrap();
        let logging: Vec<&str> = steps[first..].iter().map(|s| s.id).collect();
        assert_eq!(
            logging,
            [
                "log_left",
                "log_start",
                "log_pause",
                "log_continue",
                "log_stop",
                "recall"
            ]
        );
        for step in &steps[first..] {
            assert!(
                step.instruction
                    .contains("if the meter holds logs you want to keep"),
                "{}",
                step.id
            );
            assert!(step.wait_for_enter, "{}", step.id);
            let warns = step.instruction.contains("erases every logged session");
            assert_eq!(
                warns,
                ["log_left", "log_start"].contains(&step.id),
                "{}",
                step.id
            );
        }
        let recall = steps.iter().find(|s| s.id == "recall").unwrap();
        assert!(
            recall
                .instruction
                .contains("Yes ▲ and ▼ Erase at the same moment")
        );
        for (steps, _) in series() {
            for step in steps {
                assert_eq!(
                    step.instruction.contains('▼'),
                    ["log_left", "log_start", "recall"].contains(&step.id),
                    "{}",
                    step.id
                );
            }
        }
        assert!(
            bm82x()
                .iter()
                .all(|s| !s.id.starts_with("log_") && s.id != "recall")
        );
    }
}
