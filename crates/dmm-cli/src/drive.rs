//! Automatic sweeps of the settings the tool can drive, run after a mode step.
//!
//! A mode step captures whatever range auto-ranging picked and whatever flags
//! the meter happened to be in. On a family implementing `choices`/`select`
//! the tool can put the meter in each of them itself, so every range and flag
//! reaches the report without asking the operator to press anything. The same
//! `select` reaches a mode a button offers on the dial position the step is
//! already at, which `switch_mode` does before the step is watched.

use crate::capture::{
    CaptureReport, CaptureStep, ErrorLog, SampleData, StepResult, StepStatus, capture_samples,
    frames_for_step, needs_attention, save_report, upsert_step,
};
use crate::recording::{self, SharedRecorder};
use console::style;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::protocol::Setting;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The settings a sweep walks, in the order it walks them.
///
/// Never `Setting::Mode`: the dial is the operator's, and a step list is
/// written around which position they were asked to turn it to. A step that
/// names a mode is switched to by `switch_mode`, one value, not swept.
const SWEPT: [Setting; 5] = [
    Setting::Range,
    Setting::Hold,
    Setting::Rel,
    Setting::MinMax,
    Setting::Peak,
];

/// The choice id every swept setting returns to: auto range, or the flag off.
const RESET_ID: u16 = 0;

/// How many refused commands end the sweeps for the rest of the run. A meter
/// whose protocol we have wrong must not be spammed with commands.
const DRIVE_FAILURE_BUDGET: u32 = 3;

/// Cap on sub-steps filed per mode step, so a family with a long ladder
/// cannot turn one step into an unbounded run.
const MAX_DRIVE_SUBSTEPS_PER_STEP: usize = 24;

/// Whether the run drove the meter's settings itself.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Drive {
    /// `--no-drive`, or a family that offers no choice to switch to.
    Off,
    On,
    /// Too many refusals: the sweeps gave up part-way through the run.
    Disabled,
}

/// The sweep's budget, kept across steps so the run gives up once rather than
/// per step.
pub(crate) struct Driver {
    enabled: bool,
    /// How long to leave the meter alone before sampling, whether the change
    /// was the tool's or the operator's. Zero unless `--settle` asked for it.
    settle: Duration,
    failures: u32,
    /// Some setting offered more than one value, so `Off` can be told from a
    /// family that simply cannot be driven.
    offered: bool,
    /// Settings the meter read a driven value back on at least once. Once a
    /// setting has worked, a later refusal is the mode lacking the function
    /// — no MIN/MAX in continuity — not the protocol being wrong about it.
    proven: Vec<Setting>,
}

impl Driver {
    pub(crate) fn new(enabled: bool) -> Self {
        Driver {
            enabled,
            settle: Duration::ZERO,
            failures: 0,
            offered: false,
            proven: Vec::new(),
        }
    }

    /// Wait this long before sampling anything.
    ///
    /// A reading can take seconds to settle after a range switch — the
    /// UT61E+'s top two Ω rungs read 50x high 200 ms after the press and come
    /// down over several seconds — and the sampler is otherwise straight off
    /// the press, or off the frame that ended a step's wait. Waiting for the
    /// reading to hold still instead would never finish on leads with nothing
    /// stable across them.
    pub(crate) fn settling(mut self, settle: Duration) -> Self {
        self.settle = settle;
        self
    }

    /// Leave the meter alone for the settle time, if one was asked for.
    /// Returns whether it waited, which is what tells a caller that a frame
    /// it read before the wait is the transient the operator asked to skip.
    pub(crate) fn wait_to_settle(&self) -> bool {
        if self.settle.is_zero() {
            return false;
        }
        std::thread::sleep(self.settle);
        true
    }

    /// Record that the setting took a value the read-back showed.
    fn prove(&mut self, setting: Setting) {
        if !self.proven.contains(&setting) {
            self.proven.push(setting);
        }
    }

    /// A command step whose flag flipped has proven its setting as surely as
    /// a sweep hit: REL refused on the next mode is then that mode's doing.
    pub(crate) fn prove_command(&mut self, command: &str) {
        let setting = match command {
            "hold" => Setting::Hold,
            "rel" => Setting::Rel,
            "minmax" | "exit_minmax" => Setting::MinMax,
            "range" | "auto" => Setting::Range,
            "peak" | "exit_peak" => Setting::Peak,
            _ => return,
        };
        self.prove(setting);
    }

    /// Whether a refusal of this setting says anything about the protocol.
    fn proven(&self, setting: Setting) -> bool {
        self.proven.contains(&setting)
    }

    /// Whether another command may be sent.
    fn active(&self) -> bool {
        self.enabled && self.failures < DRIVE_FAILURE_BUDGET
    }

    /// Count a refusal, announcing the give-up exactly once.
    fn fail(&mut self) {
        self.failures += 1;
        if self.failures == DRIVE_FAILURE_BUDGET {
            eprintln!(
                "  {}",
                style("remote control unreliable on this meter \u{2014} no more automatic sweeps")
                    .yellow()
            );
        }
    }

    /// What the report records about the sweeps.
    pub(crate) fn state(&self) -> Drive {
        if !self.enabled || !self.offered {
            Drive::Off
        } else if self.failures >= DRIVE_FAILURE_BUDGET {
            Drive::Disabled
        } else {
            Drive::On
        }
    }
}

/// The id a sub-step is filed under: `dcv/range:22V`, `dcv/hold:on`.
fn sub_step_id(step_id: &str, setting: Setting, label: &str) -> String {
    format!(
        "{step_id}/{}:{}",
        setting.name(),
        choice_slug(setting, label)
    )
}

/// The label as it reads in an id and an instruction. Range and mode labels
/// are the meter's own (`22V`, `AC V`); on/off-style labels read better
/// lowercased.
fn choice_slug(setting: Setting, label: &str) -> String {
    if matches!(setting, Setting::Range | Setting::Mode) {
        label.to_string()
    } else {
        label.to_lowercase()
    }
}

/// The instruction a sub-step carries, which says nobody was asked to do it.
fn sub_step_instruction(setting: Setting, label: &str) -> String {
    format!(
        "{} \u{2192} {} (sent by the tool)",
        setting.name(),
        choice_slug(setting, label)
    )
}

/// Whether the sample shows the setting actually took the value asked for.
///
/// Choice ids follow `Setting`'s own numbering: 0 is auto range or off, and
/// MIN/MAX and Peak number their active states from one.
fn shows_target(setting: Setting, id: u16, label: &str, s: &SampleData) -> bool {
    let f = &s.flags;
    match setting {
        // The rung label alone would pass while the meter still auto-ranges:
        // the UT61+ reports the rung auto picked, and says AUTO in a flag.
        Setting::Range if id == RESET_ID => f.auto_range,
        Setting::Range => !f.auto_range && s.range_label == label,
        Setting::Hold => f.hold == (id != RESET_ID),
        Setting::Rel => f.rel == (id != RESET_ID),
        Setting::MinMax => match id {
            0 => !f.min && !f.max && !f.avg,
            1 => f.max,
            2 => f.min,
            3 => f.avg,
            _ => false,
        },
        Setting::Peak => match id {
            0 => !f.peak_max && !f.peak_min,
            1 => f.peak_max,
            2 => f.peak_min,
            _ => false,
        },
        // Never swept, so nothing to check against.
        Setting::Mode => false,
    }
}

/// Walk every value the tool can put the meter in after `step` captured, one
/// sub-step per value, and leave the meter back on auto range with its flags
/// off.
///
/// `last` is the reading the step ended on, which is what the family's
/// `choices` are relative to.
pub(crate) fn sweep_step(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step: &CaptureStep,
    last: &Measurement,
    driver: &mut Driver,
    report: &mut CaptureReport,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !driver.active() {
        return Ok(());
    }
    let mut current = last.clone();
    let mut filed = 0usize;

    for setting in SWEPT {
        // The meter refuses REL while the reading is OL, and that refusal is
        // the meter working: spending the failure budget on it would disable
        // the sweeps for the rest of the run.
        if setting == Setting::Rel && matches!(current.value, MeasuredValue::Overload) {
            eprintln!("    {}", style("\u{21b3} rel skipped: reading is OL").dim());
            continue;
        }
        let choices = dmm.choices(setting, &current);
        // One entry is the live value alone, which is no offer at all.
        if choices.len() < 2 {
            continue;
        }
        driver.offered = true;

        let mut moved = false;
        let mut charged = false;
        let mut exhausted = false;
        for choice in choices.iter().filter(|c| !c.current) {
            if filed >= MAX_DRIVE_SUBSTEPS_PER_STEP || !driver.active() {
                exhausted = true;
                break;
            }
            let id = sub_step_id(step.id, setting, &choice.label);
            if report
                .steps
                .iter()
                .any(|s| s.id == id && s.status == StepStatus::Captured)
            {
                eprintln!("    {} already captured, skipping", style(&id).dim());
                continue;
            }
            filed += 1;
            let driven = drive_choice(
                dmm,
                recorder,
                &id,
                setting,
                choice,
                step.samples,
                driver.proven(setting),
                driver,
                report,
            )?;
            if driven.hit {
                driver.prove(setting);
            }
            // Restored after a failure too: an error can follow presses that
            // landed — issue #20's HOLD press did, and only its read-back timed
            // out — and a restore of a setting that never moved is a no-op.
            moved = true;
            if let Some(m) = driven.last {
                current = m;
            }
            save_report(report, output_path)?;
            if driven.failed {
                // A setting that has already worked this run is refused
                // because this mode has no such function, which is the meter
                // being right: only an unproven one accuses the protocol.
                if !driver.proven(setting) {
                    driver.fail();
                    charged = true;
                }
                // Its other values would fail for the same reason, so they
                // are not asked for — the restore below still runs.
                break;
            }
        }

        // Restored even once the budget is spent: leaving the meter latched in
        // HOLD or on a manual range is worse than one more command. A restore
        // that fails after a failure already charged is not charged again: a
        // meter that went quiet fails both, and that is one fault, not two.
        // After an uncharged one it is, or a quiet meter never ends the sweeps.
        if moved && let Some(m) = restore(dmm, setting, &choices, driver, !charged)? {
            current = m;
        }
        if exhausted {
            break;
        }
    }
    Ok(())
}

/// Put the meter in the mode a step asks for, when a button reaches it from
/// where the dial already sits — continuity, diode and capacitance from Ω on
/// the UT61E+, duty from Hz.
///
/// Returns whether the tool switched. False leaves the step exactly as it was
/// before: the instruction is asked, and the operator turns the dial.
/// [`switch_mode`], sourcing the reading it works from.
///
/// `prev` is what an earlier step in this run left on screen. It is empty
/// whenever there was no earlier step — the first step of any run, every step
/// of a `--steps` run, and the one after a skip or a resume — and the switch
/// used to be skipped there, so `capture --steps acdcv` asked the operator to
/// press SELECT by hand, on the very form of the command a verification issue
/// hands a reporter. The meter is right here, so ask it where it is.
pub(crate) fn switch_mode_from(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step: &CaptureStep,
    prev: Option<&Measurement>,
    driver: &mut Driver,
    report: &mut CaptureReport,
) -> Result<bool, Box<dyn std::error::Error>> {
    let last = match prev {
        Some(m) => m.clone(),
        // A meter that will not answer has nothing to switch; the step's own
        // wait reports that far better than this would.
        None => match dmm.request_measurement() {
            Ok(m) => m,
            Err(_) => return Ok(false),
        },
    };
    switch_mode(dmm, recorder, step, &last, driver, report)
}

/// The switch itself, filed as the sub-step `<step>/mode:<label>` ahead of
/// the step: its frames stay whole however long the step then waits, and a
/// refusal reaches the report instead of only the terminal.
///
/// Issue #20 is why: a UT61B+ timed out partway through a two-press Hz/%
/// walk, and the step's own wait for a hand switch would have trimmed the
/// frames that show which press went unanswered.
pub(crate) fn switch_mode(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step: &CaptureStep,
    last: &Measurement,
    driver: &mut Driver,
    report: &mut CaptureReport,
) -> Result<bool, Box<dyn std::error::Error>> {
    // A command step's own command is what puts the meter where it belongs,
    // and a step that names no mode has nothing to switch to.
    if !driver.active() || step.command.is_some() {
        return Ok(false);
    }
    let Some(want) = step.expect.and_then(|e| e.mode) else {
        return Ok(false);
    };
    let choices = dmm.choices(Setting::Mode, last);
    let Some(choice) = choices
        .iter()
        .find(|c| c.label.as_ref() == want && !c.current)
    else {
        return Ok(false);
    };

    eprintln!(
        "  {}",
        style(format!("\u{21b3} switching to {want} (sent by the tool)")).dim()
    );
    let id = sub_step_id(step.id, Setting::Mode, want);
    recording::lock(recorder).set_step(Some(&id));
    let selected = dmm.select(Setting::Mode, choice.id);
    // One reading whatever the outcome: after a refusal it is the only record
    // of where the presses left the meter, which a timeout's text never says.
    let mut errors = ErrorLog::default();
    let samples: Vec<SampleData> = capture_samples(dmm, 1, &mut errors)
        .iter()
        .map(SampleData::from_measurement)
        .collect();
    let (frames, frames_dropped) = {
        let mut rec = recording::lock(recorder);
        rec.set_step(Some(step.id));
        frames_for_step(&rec.take_step(&id), &id)
    };
    let landed = samples.last().is_some_and(|s| s.mode == want);
    let error = selected.as_ref().err().map(ToString::to_string);
    upsert_step(
        report,
        StepResult {
            // The ring said a press reaches this mode, so a refusal or a
            // meter somewhere else is the family's table or its driver
            // disagreeing with the hardware.
            needs_attention: error.is_some() || !landed,
            samples,
            frames,
            frames_dropped,
            diagnostics: errors.into_diagnostics(),
            error,
            ..StepResult::new(
                &id,
                &sub_step_instruction(Setting::Mode, want),
                if selected.is_ok() {
                    StepStatus::Captured
                } else {
                    StepStatus::Error
                },
            )
        },
    );

    match selected {
        // The family reads the mode back before returning, and the step's
        // watcher still demands its matching frames.
        Ok(()) => Ok(true),
        Err(e) => {
            eprintln!(
                "  {}",
                style(format!(
                    "\u{21b3} could not switch to {want}: {e} \u{2014} do it by hand"
                ))
                .dim()
            );
            driver.fail();
            Ok(false)
        }
    }
}

/// What one driven choice left behind: the reading the next choice is
/// computed from, and whether the command failed.
struct Driven {
    last: Option<Measurement>,
    failed: bool,
    /// The read-back showed the value, so the setting is one the meter and
    /// the protocol agree on.
    hit: bool,
}

/// Put the meter on one choice and file what it read back.
///
/// A refusal is filed as the sub-step's error rather than retried: the
/// protocol may simply be wrong about this family. `proven` says the setting
/// has already worked this run, which is what decides whether the refusal is
/// worth a maintainer's attention.
#[allow(clippy::too_many_arguments)]
fn drive_choice(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    id: &str,
    setting: Setting,
    choice: &dmm_lib::protocol::Choice,
    samples_wanted: usize,
    proven: bool,
    settle: &Driver,
    report: &mut CaptureReport,
) -> Result<Driven, Box<dyn std::error::Error>> {
    let instruction = sub_step_instruction(setting, &choice.label);
    recording::lock(recorder).set_step(Some(id));

    let selected = dmm.select(setting, choice.id);
    let mut errors = ErrorLog::default();
    let measurements = match &selected {
        Ok(()) => {
            settle.wait_to_settle();
            capture_samples(dmm, samples_wanted, &mut errors)
        }
        Err(_) => Vec::new(),
    };
    let samples: Vec<SampleData> = measurements
        .iter()
        .map(SampleData::from_measurement)
        .collect();
    let diagnostics = errors.into_diagnostics();
    let (frames, frames_dropped) = {
        let mut rec = recording::lock(recorder);
        rec.set_step(None);
        frames_for_step(&rec.drain(), id)
    };

    let hit = samples
        .iter()
        .any(|s| shows_target(setting, choice.id, &choice.label, s));
    let status = match &selected {
        Ok(()) => StepStatus::Captured,
        Err(_) => StepStatus::Error,
    };
    let error = selected.as_ref().err().map(ToString::to_string);
    match (&error, samples.last()) {
        (Some(text), _) => eprintln!(
            "    {} {}",
            style(format!("\u{21b3} {}", short_id(id))).dim(),
            style(text).yellow()
        ),
        (None, Some(s)) => eprintln!(
            "    {}",
            style(format!("\u{21b3} {}  {}", short_id(id), s.summary())).dim()
        ),
        (None, None) => eprintln!(
            "    {}",
            style(format!("\u{21b3} {}  no reading", short_id(id))).dim()
        ),
    }

    upsert_step(
        report,
        StepResult {
            // A setting that has worked elsewhere in the run is refused
            // because this mode has no such function — MIN/MAX in continuity
            // — so the error text is the whole story and nothing is flagged
            // for a maintainer to look at.
            needs_attention: match &selected {
                Err(_) => !proven,
                Ok(()) => !hit || needs_attention(&samples, samples_wanted, &diagnostics),
            },
            samples,
            frames,
            frames_dropped,
            diagnostics,
            error,
            ..StepResult::new(id, &instruction, status)
        },
    );
    Ok(Driven {
        last: measurements.into_iter().next_back(),
        failed: selected.is_err(),
        hit,
    })
}

/// The `setting:label` half of a sub-step id, for the one-line echo — the
/// mode step's id is already on screen above it.
fn short_id(id: &str) -> &str {
    id.split_once('/').map_or(id, |(_, rest)| rest)
}

/// Put the setting back to auto range or off before the next mode step, and
/// say so when the meter would not go. `charge` says whether that failure
/// spends one of the run's failures.
fn restore(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    setting: Setting,
    choices: &[dmm_lib::protocol::Choice],
    driver: &mut Driver,
    charge: bool,
) -> Result<Option<Measurement>, Box<dyn std::error::Error>> {
    let Some(off) = choices.iter().find(|c| c.id == RESET_ID) else {
        return Ok(None);
    };
    let reading = match dmm.select(setting, RESET_ID) {
        // A failed walk can leave the meter in a mode without this setting
        // (RANGE once flipped a UT61E+ into AC+DC V): nothing to put back,
        // but the next setting is offered from where the meter is now.
        Err(dmm_lib::error::Error::UnsupportedCommand(_)) => {
            return Ok(dmm.request_measurement().ok());
        }
        selected => selected.and_then(|()| dmm.request_measurement()),
    };
    if let Ok(m) = &reading
        && shows_target(
            setting,
            RESET_ID,
            &off.label,
            &SampleData::from_measurement(m),
        )
    {
        return Ok(reading.ok());
    }
    eprintln!(
        "  {}",
        style(format!(
            "{setting} could not be reset \u{2014} press the meter's button"
        ))
        .yellow()
    );
    if charge {
        driver.fail();
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;

    fn sample(flags: (u8, u8, u8)) -> SampleData {
        SampleData::from_measurement(&make_test_measurement(
            0x02,
            0x01,
            b"  1.234",
            (0x00, 0x00),
            flags,
        ))
    }

    #[test]
    fn ids_keep_range_labels_and_lowercase_the_rest() {
        assert_eq!(sub_step_id("dcv", Setting::Range, "22V"), "dcv/range:22V");
        assert_eq!(sub_step_id("dcv", Setting::Hold, "On"), "dcv/hold:on");
        assert_eq!(sub_step_id("dcv", Setting::MinMax, "MAX"), "dcv/minmax:max");
    }

    #[test]
    fn instruction_says_the_tool_sent_it() {
        assert_eq!(
            sub_step_instruction(Setting::Range, "22V"),
            "range \u{2192} 22V (sent by the tool)"
        );
    }

    #[test]
    fn short_id_drops_the_mode_step() {
        assert_eq!(short_id("dcv/range:22V"), "range:22V");
        assert_eq!(short_id("dcv"), "dcv");
    }

    #[test]
    fn range_target_is_the_label_the_meter_reports() {
        // range byte 0x01 on DC V is the 22V rung.
        let s = sample((0x00, 0x04, 0x00));
        assert!(shows_target(Setting::Range, 2, "22V", &s));
        assert!(!shows_target(Setting::Range, 3, "220V", &s));
    }

    #[test]
    fn auto_range_is_a_target_like_any_other() {
        // flag2 bit 2 clear = auto-ranging, which reports the auto label.
        let auto = sample((0x00, 0x00, 0x00));
        assert!(shows_target(Setting::Range, RESET_ID, "Auto", &auto));
        let manual = sample((0x00, 0x04, 0x00));
        assert!(!shows_target(Setting::Range, RESET_ID, "Auto", &manual));
    }

    #[test]
    fn flag_targets_check_both_directions() {
        // flag1 0x0F = REL + HOLD + MIN + MAX.
        let set = sample((0x0F, 0x04, 0x00));
        let clear = sample((0x00, 0x04, 0x00));
        assert!(shows_target(Setting::Hold, 1, "On", &set));
        assert!(!shows_target(Setting::Hold, 1, "On", &clear));
        assert!(shows_target(Setting::Hold, 0, "Off", &clear));
        assert!(shows_target(Setting::Rel, 1, "On", &set));
        assert!(shows_target(Setting::MinMax, 1, "MAX", &set));
        assert!(shows_target(Setting::MinMax, 2, "MIN", &set));
        assert!(shows_target(Setting::MinMax, 0, "Off", &clear));
        assert!(shows_target(Setting::Peak, 0, "Off", &clear));
        assert!(!shows_target(Setting::Peak, 1, "P-MAX", &clear));
    }

    #[test]
    fn the_budget_gives_up_after_three_refusals() {
        let mut d = Driver::new(true);
        d.offered = true;
        assert!(d.active());
        for _ in 0..DRIVE_FAILURE_BUDGET {
            assert!(d.active());
            d.fail();
        }
        assert!(!d.active());
        assert_eq!(d.state(), Drive::Disabled);
    }

    #[test]
    fn a_family_offering_nothing_reports_off() {
        let d = Driver::new(true);
        assert_eq!(d.state(), Drive::Off);
        let mut driven = Driver::new(true);
        driven.offered = true;
        assert_eq!(driven.state(), Drive::On);
    }

    #[test]
    fn no_drive_never_sends_anything() {
        let mut d = Driver::new(false);
        d.offered = true;
        assert!(!d.active());
        assert_eq!(d.state(), Drive::Off);
    }

    /// One sweep against the mock, which implements `choices`/`select` the
    /// way the four drivable families do.
    fn swept(mode: dmm_lib::mock::MockMode, step_id: &'static str, file: &str) -> CaptureReport {
        let mut driver = Driver::new(true);
        let report = swept_by(mock(mode), step_id, file, &mut driver);
        assert_eq!(driver.state(), Drive::On, "the mock offers choices");
        report
    }

    /// The same sweep against any protocol, with the run's budget passed in
    /// so a test can seed it and read the refusals back.
    fn swept_by(
        proto: Box<dyn dmm_lib::protocol::Protocol>,
        step_id: &'static str,
        file: &str,
        driver: &mut Driver,
    ) -> CaptureReport {
        use dmm_lib::transport::{NullTransport, Transport};

        let (transport, recorder) =
            crate::recording::RecordingTransport::new(Box::new(NullTransport));
        let mut dmm = dmm_lib::Dmm::new(Box::new(transport) as Box<dyn Transport>, proto).unwrap();
        let last = dmm.request_measurement().unwrap();

        let step = CaptureStep {
            id: step_id,
            instruction: "swept",
            command: None,
            samples: 2,
            expect: None,
            verified: false,
            gate: false,
            needs: &[],
            wait_for_enter: false,
        };
        let mut report = CaptureReport::default();
        let path = std::env::temp_dir()
            .join(file)
            .to_string_lossy()
            .to_string();

        sweep_step(
            &mut dmm,
            &recorder,
            &step,
            &last,
            driver,
            &mut report,
            &path,
        )
        .unwrap();
        let _ = std::fs::remove_file(&path);
        report
    }

    /// Every offered value has to end up filed as a sub-step whose sample
    /// shows the meter took it.
    #[test]
    fn the_sweep_files_a_sub_step_per_offered_value() {
        let report = swept(
            dmm_lib::mock::MockMode::DcV,
            "dcv",
            "dmm-cli-test-drive.yaml",
        );

        assert!(
            report.steps.iter().any(|s| s.id == "dcv/hold:on"),
            "hold was not swept: {:?}",
            report.steps.iter().map(|s| &s.id).collect::<Vec<_>>()
        );
        assert!(
            report.steps.iter().any(|s| s.id.starts_with("dcv/range:")),
            "no range rung was swept"
        );
        for s in &report.steps {
            assert_eq!(s.status, StepStatus::Captured, "{} was refused", s.id);
            assert!(!s.samples.is_empty(), "{} filed no sample", s.id);
            assert!(
                !s.needs_attention,
                "{} did not read its target back: {}",
                s.id,
                s.samples
                    .last()
                    .map(SampleData::summary)
                    .unwrap_or_default()
            );
        }
    }

    /// `--settle` leaves the meter alone before sampling; without it the
    /// sampler reads straight off the press, which on a range that takes
    /// seconds to settle files the transient (a UT61E+'s top Ω rungs read 50x
    /// high 200 ms after the press, 2026-09-10). It says whether it waited,
    /// so a caller knows its pre-wait frame is the transient.
    #[test]
    fn a_settle_time_delays_sampling_and_zero_does_not() {
        use std::time::Instant;
        let settle = Duration::from_millis(120);
        let driver = Driver::new(true).settling(settle);

        let started = Instant::now();
        assert!(
            driver.wait_to_settle(),
            "a settle time has to be waited out"
        );
        assert!(
            started.elapsed() >= settle,
            "settle time was not waited out"
        );

        // The default costs nothing: every run that does not ask pays no delay.
        let started = Instant::now();
        assert!(!Driver::new(true).wait_to_settle());
        assert!(started.elapsed() < settle);

        // `--no-drive` still honours it: nothing the tool presses is what it
        // turns off, not the wait before a reading is filed.
        assert!(Driver::new(false).settling(settle).wait_to_settle());
    }

    /// A meter on the mock's AC V ring, which offers the "AC V Hz" sub-mode
    /// the way a real dial position's button does, recorded the way a capture
    /// run records it, with an empty report to file into.
    struct Bench {
        dmm: dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
        recorder: SharedRecorder,
        last: Measurement,
        report: CaptureReport,
    }

    fn meter(proto: Box<dyn dmm_lib::protocol::Protocol>) -> Bench {
        use dmm_lib::transport::{NullTransport, Transport};
        let (transport, recorder) =
            crate::recording::RecordingTransport::new(Box::new(NullTransport));
        let mut dmm = dmm_lib::Dmm::new(Box::new(transport) as Box<dyn Transport>, proto).unwrap();
        let last = dmm.request_measurement().unwrap();
        Bench {
            dmm,
            recorder,
            last,
            report: CaptureReport::default(),
        }
    }

    impl Bench {
        fn switch(&mut self, step: &CaptureStep, driver: &mut Driver) -> bool {
            let last = self.last.clone();
            switch_mode(
                &mut self.dmm,
                &self.recorder,
                step,
                &last,
                driver,
                &mut self.report,
            )
            .unwrap()
        }

        fn mode(&mut self) -> String {
            self.dmm.request_measurement().unwrap().mode.to_string()
        }

        fn ids(&self) -> Vec<&str> {
            self.report.steps.iter().map(|s| s.id.as_str()).collect()
        }
    }

    fn mock(mode: dmm_lib::mock::MockMode) -> Box<dyn dmm_lib::protocol::Protocol> {
        Box::new(dmm_lib::mock::MockProtocol::with_mode(mode))
    }

    fn mode_step(mode: &'static str) -> CaptureStep {
        CaptureStep {
            id: "step",
            instruction: "Set the meter to it",
            command: None,
            samples: 2,
            expect: Some(dmm_lib::protocol::Expect::mode(mode)),
            verified: false,
            gate: false,
            needs: &[],
            wait_for_enter: false,
        }
    }

    /// The sub-mode is on the ring the dial already sits on, so the tool
    /// presses the button instead of asking, and files the switch as a
    /// sub-step of its own showing where the meter landed.
    #[test]
    fn a_mode_the_ring_offers_is_switched_to() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        let mut driver = Driver::new(true);
        assert!(bench.switch(&mode_step("AC V Hz"), &mut driver));
        assert_eq!(bench.mode(), "AC V Hz");
        assert_eq!(driver.failures, 0);

        assert_eq!(bench.ids(), ["step/mode:AC V Hz"]);
        let filed = &bench.report.steps[0];
        assert_eq!(filed.status, StepStatus::Captured);
        assert_eq!(
            filed.instruction,
            "mode \u{2192} AC V Hz (sent by the tool)"
        );
        assert_eq!(
            filed.samples.last().map(|s| s.mode.as_str()),
            Some("AC V Hz")
        );
        assert!(filed.error.is_none());
        assert!(!filed.needs_attention);
    }

    /// Nothing earlier in the run left a reading, which is every step of a
    /// `--steps` run. The tool takes one rather than making the operator
    /// press the button.
    #[test]
    fn a_step_with_no_earlier_reading_still_switches() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        let mut driver = Driver::new(true);
        assert!(
            switch_mode_from(
                &mut bench.dmm,
                &bench.recorder,
                &mode_step("AC V Hz"),
                None,
                &mut driver,
                &mut bench.report,
            )
            .unwrap()
        );
        assert_eq!(bench.mode(), "AC V Hz");
        assert_eq!(driver.failures, 0);
    }

    /// A mode the ring does not reach needs the dial turned, which is the
    /// operator's job: the tool must ask rather than send anything, and
    /// there is no switch to file.
    #[test]
    fn a_mode_off_the_ring_is_left_to_the_operator() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        let mut driver = Driver::new(true);
        assert!(!bench.switch(&mode_step("DC V"), &mut driver));
        assert_eq!(bench.mode(), "AC V");

        // A dial position offering no ring at all is the same case.
        let mut bench = meter(mock(dmm_lib::mock::MockMode::Ohm));
        assert!(!bench.switch(&mode_step("Capacitance"), &mut driver));
        assert_eq!(bench.mode(), "\u{03A9}");
        assert_eq!(driver.failures, 0);
        assert!(bench.report.steps.is_empty());
    }

    /// A command step sends its own command; switching the mode under it
    /// would be undoing what the step is there to test.
    #[test]
    fn a_command_step_is_never_switched() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        let step = CaptureStep {
            command: Some("hold"),
            ..mode_step("AC V Hz")
        };
        let mut driver = Driver::new(true);
        assert!(!bench.switch(&step, &mut driver));
        assert_eq!(bench.mode(), "AC V");
    }

    /// The mock, but one setting's values are refused the way a meter
    /// answers a function the mode it is in does not have. Turning the
    /// setting off still works: it is already off, which is what the
    /// families' `select` checks before it presses anything.
    /// How [`Fails`] fails its one setting.
    #[derive(Clone, Copy)]
    enum Failure {
        /// The meter says no and nothing moves.
        Refused,
        /// The value lands and only the read-back is lost, as issue #20's HOLD
        /// press was.
        TimedOutAfterLanding,
        /// The value lands and the walk still ends in a refusal, as a RANGE
        /// press that flips the mode does.
        RefusedAfterLanding,
        /// Nothing answers, putting the setting back included.
        Silent,
        /// The meter says no, and the mode it is left in has no such setting
        /// to put back.
        RefusedIntoAnotherMode,
    }

    struct Fails {
        inner: dmm_lib::mock::MockProtocol,
        setting: Setting,
        failure: Failure,
    }

    impl Fails {
        fn boxed(
            mode: dmm_lib::mock::MockMode,
            setting: Setting,
            failure: Failure,
        ) -> Box<dyn dmm_lib::protocol::Protocol> {
            Box::new(Fails {
                inner: dmm_lib::mock::MockProtocol::with_mode(mode),
                setting,
                failure,
            })
        }

        fn refusing(
            mode: dmm_lib::mock::MockMode,
            setting: Setting,
        ) -> Box<dyn dmm_lib::protocol::Protocol> {
            Self::boxed(mode, setting, Failure::Refused)
        }
    }

    impl dmm_lib::protocol::Protocol for Fails {
        fn init(&mut self, t: &dyn dmm_lib::transport::Transport) -> dmm_lib::error::Result<()> {
            self.inner.init(t)
        }
        fn request_measurement(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<Measurement> {
            self.inner.request_measurement(t)
        }
        fn parse_payload(&self, payload: &[u8]) -> dmm_lib::error::Result<Measurement> {
            self.inner.parse_payload(payload)
        }
        fn send_command(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
            command: &str,
        ) -> dmm_lib::error::Result<()> {
            self.inner.send_command(t, command)
        }
        fn get_name(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<Option<String>> {
            self.inner.get_name(t)
        }
        fn profile(&self) -> &dmm_lib::protocol::DeviceProfile {
            self.inner.profile()
        }
        fn choices(
            &self,
            setting: Setting,
            current: &Measurement,
        ) -> Vec<dmm_lib::protocol::Choice> {
            self.inner.choices(setting, current)
        }
        fn select(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
            setting: Setting,
            id: u16,
        ) -> dmm_lib::error::Result<()> {
            use dmm_lib::error::Error;
            if setting != self.setting {
                return self.inner.select(t, setting, id);
            }
            match self.failure {
                Failure::Silent => Err(Error::Timeout),
                Failure::RefusedIntoAnotherMode if id == RESET_ID => {
                    Err(Error::UnsupportedCommand("no such setting here".into()))
                }
                _ if id == RESET_ID => self.inner.select(t, setting, id),
                Failure::Refused | Failure::RefusedIntoAnotherMode => {
                    Err(Error::CommandRejected("no".into()))
                }
                Failure::TimedOutAfterLanding => {
                    self.inner.select(t, setting, id)?;
                    Err(Error::Timeout)
                }
                Failure::RefusedAfterLanding => {
                    self.inner.select(t, setting, id)?;
                    Err(Error::CommandRejected("gave up".into()))
                }
            }
        }
    }

    /// A refused switch falls back to asking, and spends one of the three
    /// refusals that end remote control for the run.
    #[test]
    fn a_refused_switch_counts_against_the_budget() {
        let mut bench = meter(Fails::refusing(dmm_lib::mock::MockMode::AcV, Setting::Mode));
        let mut driver = Driver::new(true);
        assert!(!bench.switch(&mode_step("AC V Hz"), &mut driver));
        assert_eq!(driver.failures, 1);
        assert!(driver.active());
    }

    /// The refusal is in the report, not only on the terminal, with the
    /// reading that says where the meter was left — the evidence issue #20
    /// needed and a timeout's own text does not carry.
    #[test]
    fn a_refused_switch_is_filed_with_where_the_meter_was_left() {
        let mut bench = meter(Fails::refusing(dmm_lib::mock::MockMode::AcV, Setting::Mode));
        let mut driver = Driver::new(true);
        bench.switch(&mode_step("AC V Hz"), &mut driver);

        assert_eq!(bench.ids(), ["step/mode:AC V Hz"]);
        let filed = &bench.report.steps[0];
        assert_eq!(filed.status, StepStatus::Error);
        assert_eq!(filed.error.as_deref(), Some("command rejected: no"));
        assert_eq!(filed.samples.last().map(|s| s.mode.as_str()), Some("AC V"));
        assert!(filed.needs_attention);
    }

    /// The meter refuses REL on an OL reading, so the sweep must not spend a
    /// failure on asking.
    #[test]
    fn rel_is_not_swept_while_the_reading_is_ol() {
        let report = swept(
            dmm_lib::mock::MockMode::OhmOl,
            "ohm_ol",
            "dmm-cli-test-drive-ol.yaml",
        );
        assert!(
            !report.steps.iter().any(|s| s.id.contains("/rel:")),
            "rel was swept on OL: {:?}",
            report.steps.iter().map(|s| &s.id).collect::<Vec<_>>()
        );
        assert!(
            report.steps.iter().any(|s| s.id == "ohm_ol/hold:on"),
            "the rest of the sweep still has to run"
        );
    }

    fn minmax_steps(report: &CaptureReport) -> Vec<&String> {
        report
            .steps
            .iter()
            .filter(|s| s.id.starts_with("dcv/minmax:"))
            .map(|s| &s.id)
            .collect()
    }

    /// MIN/MAX worked in an earlier mode, so continuity refusing it is the
    /// meter saying this mode has no such function — not a reason to stop
    /// driving the rest of the run.
    #[test]
    fn a_refusal_of_a_proven_setting_costs_no_budget() {
        let mut driver = Driver::new(true);
        // The MIN/MAX command step flipped the flag earlier in the run.
        driver.prove_command("minmax");
        assert!(driver.proven(Setting::MinMax));
        driver.prove_command("light");
        let report = swept_by(
            Fails::refusing(dmm_lib::mock::MockMode::DcV, Setting::MinMax),
            "dcv",
            "dmm-cli-test-drive-proven.yaml",
            &mut driver,
        );
        assert_eq!(driver.failures, 0);
        assert_eq!(driver.state(), Drive::On);
        let filed = minmax_steps(&report);
        assert_eq!(filed.len(), 1, "the refusal is still filed once: {filed:?}");
        // Nothing to look at: the error text says the mode hasn't got MIN/MAX.
        let refused = report
            .steps
            .iter()
            .find(|s| s.id.starts_with("dcv/minmax:"))
            .expect("the refusal is filed");
        assert_eq!(refused.status, StepStatus::Error);
        assert!(refused.error.is_some());
        assert!(!refused.needs_attention, "a proven setting was flagged");
    }

    /// A HOLD press that landed is put back off whatever error followed it,
    /// or the rest of the run happens under HOLD: in issue #20 a timed-out
    /// read-back left it on, and it swallowed the next step's Hz/% press.
    #[test]
    fn a_setting_that_failed_after_landing_is_still_restored() {
        for failure in [Failure::TimedOutAfterLanding, Failure::RefusedAfterLanding] {
            let mut driver = Driver::new(true);
            let report = swept_by(
                Fails::boxed(dmm_lib::mock::MockMode::DcV, Setting::Hold, failure),
                "dcv",
                "dmm-cli-test-drive-landed.yaml",
                &mut driver,
            );
            let hold = report
                .steps
                .iter()
                .find(|s| s.id == "dcv/hold:on")
                .expect("the HOLD sub-step is filed");
            assert_eq!(hold.status, StepStatus::Error);
            // HOLD drops REL, so REL landing says HOLD went back off.
            let rel = report
                .steps
                .iter()
                .find(|s| s.id == "dcv/rel:on")
                .expect("the sweep went on to REL");
            assert_eq!(rel.status, StepStatus::Captured, "{:?}", rel.error);
            assert!(rel.samples.iter().all(|s| !s.flags.hold));
        }
    }

    /// A meter that stops answering fails the setting and then its restore:
    /// one fault, so one failure. Not two, or two such settings end remote
    /// control; not zero on a proven setting, whose own failure goes
    /// uncharged, or a quiet meter is swept for the rest of the run.
    #[test]
    fn a_silent_meter_spends_one_failure() {
        for proven in [false, true] {
            let mut driver = Driver::new(true);
            if proven {
                driver.prove_command("hold");
            }
            swept_by(
                Fails::boxed(dmm_lib::mock::MockMode::DcV, Setting::Hold, Failure::Silent),
                "dcv",
                "dmm-cli-test-drive-silent.yaml",
                &mut driver,
            );
            assert_eq!(driver.failures, 1, "proven: {proven}");
        }
    }

    /// A proven setting refused is the meter being right, and a mode left
    /// with no such setting has nothing to restore: neither costs a failure.
    #[test]
    fn a_restore_with_nothing_to_restore_costs_no_failure() {
        let mut driver = Driver::new(true);
        driver.prove_command("hold");
        swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Hold,
                Failure::RefusedIntoAnotherMode,
            ),
            "dcv",
            "dmm-cli-test-drive-modeless.yaml",
            &mut driver,
        );
        assert_eq!(driver.failures, 0);
    }

    /// A setting nothing has driven yet is the case the budget exists for,
    /// and one refusal answers for every value of it.
    #[test]
    fn a_refusal_of_an_unproven_setting_spends_one_failure() {
        let mut driver = Driver::new(true);
        let report = swept_by(
            Fails::refusing(dmm_lib::mock::MockMode::DcV, Setting::MinMax),
            "dcv",
            "dmm-cli-test-drive-unproven.yaml",
            &mut driver,
        );
        assert_eq!(driver.failures, 1);
        let filed = minmax_steps(&report);
        assert_eq!(
            filed.len(),
            1,
            "the setting's other values must not be asked for: {filed:?}"
        );
        assert!(
            report.steps.iter().any(|s| s.id == "dcv/hold:on"),
            "the settings after it still have to be swept"
        );
        // This one does accuse the protocol, so it is flagged.
        assert!(
            report
                .steps
                .iter()
                .any(|s| s.id.starts_with("dcv/minmax:") && s.needs_attention),
            "an unproven setting's refusal must be flagged"
        );
    }
}
