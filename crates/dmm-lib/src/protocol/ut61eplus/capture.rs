//! The UT61+/UT161 family's capture steps, one list for every model.

use super::mode::Mode;
use super::tables::ModelTable;
use crate::flags::Flag;
use crate::protocol::steps::{self, Ohms, Volts};
use crate::protocol::{
    CaptureStep, DeviceProfile, Expect, Need, RangeExpect, Stability, ValueExpect,
};

pub(super) fn steps(table: &ModelTable, profile: &DeviceProfile) -> Vec<CaptureStep> {
    // A model whose buttons the family list does not describe.
    if let Some(steps) = table.capture_steps {
        return steps();
    }

    // The list is shared by the whole UT61+/UT161 family; the UT61E+, the
    // UT61B+ (issue #19) and the UT161E (issue #7) have run every step of
    // it. A step a verified model has not run must not ride on this flag —
    // the two ladder steps were held back that way for the B+ until it
    // walked them on 2026-09-11.
    let hw = profile.stability == Stability::Verified;
    // Every step on this meter takes three samples. A step those models
    // have run carries their hardware history through `mark` (the gate
    // steps) or `ran` (the rest); a step none has run takes neither.
    let mark = |s: CaptureStep| s.samples(3).verified_if(hw);
    let ran = |id, instruction| mark(CaptureStep::basic(id, instruction));
    let [dcv, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", "Set meter to DC V (V\u{2393}). Leave leads open."),
        Ohms::Symbol,
        CaptureStep::basic(
            "ohm",
            "Set meter to \u{03A9}. Leave leads open (should show OL).",
        ),
    )
    .map(mark);
    let [
        hold,
        hold_off,
        rel,
        rel_off,
        minmax,
        minmax_off,
        range,
        auto,
    ] = command_steps(hw);

    let mut steps = vec![
        // The six gate steps first, both trios, so the gate is decided by
        // step six and every step after it can be driven. Split across the
        // list, as they used to be, the run reached AC V, DC mV and AC mV
        // while still ungated and swept none of them (issue #19).
        dcv,
        dcv_short,
        dcv_negative,
        ohm,
        ohm_body,
        ohm_short,
        // Gate decided. A gate step is never swept — the step after it
        // assumes the state it left — so each ladder gets a plain step of
        // its own, in the mode the run is already in.
        ran(
            "ohm_ranges",
            "Set meter to \u{03A9}. Leads open or shorted, either will do.",
        )
        .expect(Expect::mode("\u{03A9}")),
        // The rest of the resistance dial position, while the leads are
        // still there.
        ran(
            "continuity",
            "Set meter to continuity (buzzer). Touch probes together.",
        )
        .needs(&[Need::ShortedLeads])
        .expect(Expect::mode("Continuity").value(ValueExpect::Finite)),
        // Open leads read OL, and the meter refuses REL over OL in any
        // mode, so the old wording ("leave leads open") guaranteed that
        // this step's REL and MIN/MAX sub-steps said nothing about diode
        // — which is exactly what happened until 2026-09-10. Both are
        // now known dead here, so the step no longer sweeps them, but a
        // finite reading is still what verifies the decode.
        ran(
            "diode",
            "Set meter to diode. A diode across the probes if you have one, \
             otherwise leave the leads open (OL).",
        )
        .expect(Expect::mode("Diode")),
        ran("capacitance", "Set meter to capacitance. Leave leads open.")
            .expect(Expect::mode("Capacitance")),
        // Back to volts for its ladder and for the button steps below,
        // which have always assumed the dial is here.
        ran(
            "dcv_ranges",
            "Set meter to DC V (V\u{2393}). Leave leads open.",
        )
        .expect(Expect::mode("DC V")),
        hold,
        hold_off,
        rel,
        rel_off,
        minmax,
        minmax_off,
        range,
        auto,
        // The rest of each dial position's SELECT ring (spec §3.1): one
        // step per mode the ring reaches, in dial order.
        ran(
            "acdcv",
            "Set meter to V\u{2393} and press SELECT for AC+DC V.",
        )
        .expect(Expect::mode("AC+DC V")),
        ran("acv", "Set meter to AC V (V~). Leave leads open.").expect(Expect::mode("AC V")),
        ran("lpfv", "Set meter to V~ and press SELECT for LPF V.").expect(Expect::mode("LPF V")),
        ran("dcmv", "Set meter to DC mV. Leave leads open.").expect(Expect::mode("DC mV")),
        ran("acmv", "Set meter to mV and press SELECT for AC mV.").expect(Expect::mode("AC mV")),
        ran("hz", "Set meter to the Hz/% dial position.").expect(Expect::mode("Hz")),
        ran(
            "duty",
            "Hz/% position: short-press the USB button for Duty %.",
        )
        .expect(Expect::mode("Duty %")),
        ran("ncv", "Set meter to NCV. Hold near a live wire.")
            .needs(&[Need::LiveWire])
            .expect(Expect::mode("NCV").value(ValueExpect::NcvDetected)),
        ran("hfe", "Set meter to hFE (transistor test).")
            .needs(&[Need::Transistor])
            .expect(Expect::mode("hFE")),
        ran("dcua", "Set meter to DC µA.").expect(Expect::mode("DC µA")),
        ran("acua", "Set meter to µA and press SELECT for AC µA.").expect(Expect::mode("AC µA")),
        ran("dcma", "Set meter to DC mA.").expect(Expect::mode("DC mA")),
        ran("acma", "Set meter to mA and press SELECT for AC mA.").expect(Expect::mode("AC mA")),
        ran("dca", "Set meter to DC A (A\u{2393}).").expect(Expect::mode("DC A")),
        ran("aca", "Set meter to A and press SELECT for AC A.").expect(Expect::mode("AC A")),
        // Temperature needs a thermocouple, so it has never been run.
        CaptureStep::basic("temp", "Set meter to temperature (K-type thermocouple).")
            .samples(3)
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("\u{00B0}C")),
        CaptureStep::basic("tempf", "Temperature position: press SELECT for \u{00B0}F.")
            .samples(3)
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("\u{00B0}F")),
        CaptureStep::basic("loz", "Set meter to LoZ (low-impedance volts).")
            .samples(3)
            .expect(Expect::mode("LoZ V")),
    ];
    // A model whose dial differs words those steps its own way.
    for (id, instruction) in table.step_instructions {
        if let Some(step) = steps.iter_mut().find(|s| s.id == *id) {
            step.instruction = instruction;
        }
    }
    // The list names every mode in the family; a model's dial table says
    // which it reaches (spec §2.1: temperature and LoZ are UT61D+/UT161D
    // positions), so the others are not asked for. A table without a
    // dial lists its modes instead.
    let dial = table.dial;
    let reached: Vec<Mode> = if dial.is_empty() {
        table.modes.to_vec()
    } else {
        dial.iter()
            .flat_map(|p| p.modes())
            .filter_map(|m| Mode::from_byte(u8::try_from(m).ok()?).ok())
            .collect()
    };
    steps.retain(|step| {
        let Some(label) = step.expect.and_then(|e| e.mode) else {
            return true;
        };
        reached.iter().any(|mode| mode.as_static_str() == label)
    });
    // Nor is a command the model's buttons do not take (family spec §6.5).
    let commands = profile.supported_commands;
    steps.retain(|step| step.command.is_none_or(|c| commands.contains(&c)));
    steps
}

/// The command steps: HOLD, REL and MIN/MAX on and off, then RANGE and AUTO,
/// all on DC V. `hw` marks them verified, for a model that has run them.
///
/// Flags & commands run wherever the dial is, so they sit on DC V: at the end
/// of the list the dial was on DC A, a single range where RANGE and AUTO have
/// nothing to do (`auto did nothing` on hardware, 2026-09-07); DC V has four
/// rungs.
pub(super) fn command_steps(hw: bool) -> [CaptureStep; 8] {
    [
        CaptureStep::with_command(
            "hold",
            "DC V mode: press HOLD on the meter, or we will send the command.",
            "hold",
            3,
        )
        .verified_if(hw)
        .expect(Expect::new().flags(&[(Flag::Hold, true)])),
        CaptureStep::with_command("hold_off", "Press HOLD again to turn it off.", "hold", 3)
            .verified_if(hw)
            .expect(Expect::new().flags(&[(Flag::Hold, false)])),
        CaptureStep::with_command("rel", "DC V mode: we will send REL.", "rel", 3)
            .verified_if(hw)
            .expect(Expect::new().flags(&[(Flag::Rel, true)])),
        CaptureStep::with_command(
            "rel_off",
            "We will send REL again to turn it off.",
            "rel",
            3,
        )
        .verified_if(hw)
        .expect(Expect::new().flags(&[(Flag::Rel, false)])),
        // Which of MIN and MAX the first press lands on is the meter's
        // own cycle, so only the exit is asserted.
        CaptureStep::with_command("minmax", "We will send MIN/MAX.", "minmax", 3).verified_if(hw),
        CaptureStep::with_command("minmax_off", "We will exit MIN/MAX.", "exit_minmax", 3)
            .verified_if(hw)
            .expect(Expect::new().flags(&[(Flag::Min, false), (Flag::Max, false)])),
        // A single RANGE press, not a sweep: the capture walks the ladder
        // itself through `choices(Range)`, reading back each rung. A blind
        // six-press sweep was tried and removed: it filed indices 0, 2, 0,
        // 0, 0, 0 from stale frames, and a mode flip to AC+DC V that 0x46 was
        // never seen to cause (ut61-family spec §6.1).
        CaptureStep::with_command(
            "range",
            "We will send RANGE to switch to manual.",
            "range",
            3,
        )
        .verified_if(hw)
        .expect(Expect::new().range(RangeExpect::Manual)),
        CaptureStep::with_command(
            "auto",
            "We will send AUTO to return to auto-range.",
            "auto",
            3,
        )
        .verified_if(hw)
        .expect(Expect::new().range(RangeExpect::Auto)),
    ]
}
