//! Automatic sweeps of the settings the tool can drive, run after a mode step.
//!
//! A mode step captures whatever range auto-ranging picked and whatever flags
//! the meter happened to be in. On a family implementing `choices`/`select`
//! the tool can put the meter in each of them itself, so every range and flag
//! reaches the report without asking the operator to press anything. The same
//! `select` reaches a mode a button offers on the dial position the step is
//! already at, which `switch_mode` does before the step is watched.

use super::recording;
use super::report::already_captured;
use super::session::CaptureRun;
use super::step::{Frames, file_step};
use super::{
    CaptureStep, ErrorLog, SampleData, StepResult, StepStatus, link_gone, needs_attention,
    read_past_blank, samples_after_switch, save_report,
};
use console::style;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::protocol::Setting;
use dmm_lib::text::TextKind;
use dmm_shared::tr;
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
/// Not the library's `AUTO_RANGE_ID`, which names the range case only.
const RESET_ID: u16 = 0;

/// How many refused commands end the sweeps for the rest of the run. A meter
/// whose protocol we have wrong must not be spammed with commands.
const DRIVE_FAILURE_BUDGET: u32 = 3;

/// Cap on sub-steps filed per mode step, so a family with a long ladder
/// cannot turn one step into an unbounded run.
const MAX_DRIVE_SUBSTEPS_PER_STEP: usize = 24;

/// Whether the run drove the meter's settings itself.
///
/// Declared from least to most telling, which is the order [`Drive::merged`]
/// keeps across a resumed run.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Drive {
    /// `--no-drive`, or a family that offers no choice to switch to.
    Off,
    On,
    /// Too many refusals: the sweeps gave up part-way through the run.
    Disabled,
}

impl Drive {
    /// What a report says once another run has resumed it: a sweep that gave
    /// up earlier still explains the steps it left unswept, however the
    /// later run went (issue #5: a `--steps` re-run overwrote `disabled`
    /// with `on`).
    pub(crate) fn merged(earlier: Option<Drive>, this_run: Drive) -> Drive {
        earlier.map_or(this_run, |earlier| earlier.max(this_run))
    }
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
    /// the press (past any blank display), or off the frame that ended a
    /// step's wait. Waiting for the reading to hold still instead would never
    /// finish on leads with nothing stable across them.
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
                style(tr!(crate::l10n::get(), "capture-drive-unreliable")).yellow()
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

/// The id a setting's restore after its sweep is filed under:
/// `dcv/range~restore`. No `:`, so triage never takes `restore` for a
/// choice the meter should show, and no choice label can collide with it.
fn restore_id(step_id: &str, setting: Setting) -> String {
    format!("{step_id}/{}~restore", setting.name())
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

/// What one driven choice left behind: the reading the next choice is
/// computed from, and whether the command failed.
struct Driven {
    last: Option<Measurement>,
    failed: bool,
    /// The read-back showed the value, so the setting is one the meter and
    /// the protocol agree on.
    hit: bool,
}

/// The `setting:label` half of a sub-step id, for the one-line echo — the
/// mode step's id is already on screen above it.
fn short_id(id: &str) -> &str {
    id.split_once('/').map_or(id, |(_, rest)| rest)
}

impl CaptureRun {
    /// Walk every value the tool can put the meter in after `step` captured, one
    /// sub-step per value, and leave the meter back on auto range with its flags
    /// off — or on the rung it started on, where the mode has no auto range.
    ///
    /// `last` is the reading the step ended on, which is what the family's
    /// `choices` are relative to.
    pub(super) fn sweep_step(
        &mut self,
        step: &CaptureStep,
        last: &Measurement,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.driver.active() {
            return Ok(());
        }
        let l = crate::l10n::get();
        let mut current = last.clone();
        let mut filed = 0usize;

        for setting in SWEPT {
            // The meter refuses REL while the reading is OL, and that refusal is
            // the meter working: spending the failure budget on it would disable
            // the sweeps for the rest of the run. A display still blank after the
            // wait has no reading to take a reference from either.
            if setting == Setting::Rel {
                let skipped = match current.value {
                    MeasuredValue::Overload => Some(tr!(
                        l,
                        "capture-drive-rel-skipped-ol",
                        setting = setting.name(),
                    )),
                    MeasuredValue::NoReading(_) => Some(tr!(
                        l,
                        "capture-drive-rel-skipped-blank",
                        setting = setting.name(),
                    )),
                    _ => None,
                };
                if let Some(skipped) = skipped {
                    eprintln!("    {}", style(format!("\u{21b3} {skipped}")).dim());
                    continue;
                }
            }
            let choices = self.dmm.choices(setting, &current);
            // One entry is the live value alone, which is no offer at all.
            if choices.len() < 2 {
                continue;
            }
            self.driver.offered = true;

            let mut moved = false;
            let mut charged = false;
            let mut exhausted = false;
            for choice in choices.iter().filter(|c| !c.current) {
                if filed >= MAX_DRIVE_SUBSTEPS_PER_STEP || !self.driver.active() {
                    exhausted = true;
                    break;
                }
                let id = sub_step_id(step.id, setting, &choice.label);
                if already_captured(&self.report, &id) {
                    eprintln!(
                        "    {}",
                        tr!(
                            l,
                            "capture-already-captured",
                            step = style(&id).dim().to_string(),
                        )
                    );
                    continue;
                }
                filed += 1;
                let driven = self.drive_choice(&id, setting, choice, step.samples)?;
                if driven.hit {
                    self.driver.prove(setting);
                }
                // Restored after a failure too: an error can follow presses that
                // landed — issue #20's HOLD press did, and only its read-back timed
                // out — and a restore of a setting that never moved is a no-op.
                moved = true;
                if let Some(m) = driven.last {
                    current = m;
                }
                save_report(&self.report, &self.output_path)?;
                if driven.failed {
                    // A setting that has already worked this run is refused
                    // because this mode has no such function, which is the meter
                    // being right: only an unproven one accuses the protocol.
                    if !self.driver.proven(setting) {
                        self.driver.fail();
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
            // Not counted against the cap: it puts the meter back, and the cap
            // is about how many values one step asks for.
            if moved && let Some(m) = self.restore(step.id, setting, &choices, !charged)? {
                current = m;
            }
            if exhausted {
                break;
            }
        }
        Ok(())
    }

    /// [`Self::switch_mode`], sourcing the reading it works from.
    ///
    /// `prev` is what an earlier step in this run left on screen. It is empty
    /// whenever there was no earlier step — the first step of any run, every step
    /// of a `--steps` run, and the one after a skip or a resume — and the switch
    /// used to be skipped there, so `capture --steps acdcv` asked the operator to
    /// press SELECT by hand, on the very form of the command a verification issue
    /// hands a reporter. The meter is right here, so ask it where it is.
    pub(super) fn switch_mode_from(
        &mut self,
        step: &CaptureStep,
        prev: Option<&Measurement>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let last = match prev {
            Some(m) => m.clone(),
            // A meter that will not answer has nothing to switch; the step's own
            // wait reports that far better than this would.
            None => match self.dmm.request_measurement() {
                Ok(m) => m,
                Err(_) => return Ok(false),
            },
        };
        self.switch_mode(step, &last)
    }

    /// Put the meter in the mode a step asks for, when a button reaches it from
    /// where the dial already sits — continuity, diode and capacitance from Ω on
    /// the UT61E+, duty from Hz.
    ///
    /// Returns whether the tool switched. False leaves the step exactly as it was
    /// before: the instruction is asked, and the operator turns the dial.
    ///
    /// The switch is filed as the sub-step `<step>/mode:<label>` ahead of
    /// the step: its frames stay whole however long the step then waits, and a
    /// refusal reaches the report instead of only the terminal.
    ///
    /// Issue #20 is why: a UT61B+ timed out partway through a two-press Hz/%
    /// walk, and the step's own wait for a hand switch would have trimmed the
    /// frames that show which press went unanswered.
    pub(super) fn switch_mode(
        &mut self,
        step: &CaptureStep,
        last: &Measurement,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // A command step's own command is what puts the meter where it belongs,
        // and a step that names no mode has nothing to switch to.
        if !self.driver.active() || step.command.is_some() {
            return Ok(false);
        }
        let Some(want) = step.expect.and_then(|e| e.mode) else {
            return Ok(false);
        };
        let choices = self.dmm.choices(Setting::Mode, last);
        let Some(choice) = choices
            .iter()
            .find(|c| c.label.as_ref() == want && !c.current)
        else {
            return Ok(false);
        };

        let l = crate::l10n::get();
        let mode = l.label(TextKind::Mode, want);
        eprintln!(
            "  {}",
            style(format!(
                "\u{21b3} {}",
                tr!(l, "capture-drive-switching", mode = &*mode)
            ))
            .dim()
        );
        let id = sub_step_id(step.id, Setting::Mode, want);
        recording::lock(&self.recorder).set_step(Some(&id));
        let selected = match self.dmm.select(Setting::Mode, choice.id) {
            // A lost link is no meter refusing the mode: the run ends on it.
            Err(e) if link_gone(&e) => return Err(e.into()),
            selected => selected,
        };
        // One reading whatever the outcome: after a refusal it is the only record
        // of where the presses left the meter, which a timeout's text never says.
        // Taken once the display is back, so the step's own samples start there
        // too.
        let mut errors = ErrorLog::default();
        let samples = SampleData::from_all(&samples_after_switch(&mut self.dmm, 1, &mut errors)?);
        let landed = samples.last().is_some_and(|s| s.mode == want);
        let error = selected.as_ref().err().map(ToString::to_string);
        file_step(
            &mut self.report,
            &self.recorder,
            StepResult {
                // The ring said a press reaches this mode, so a refusal or a
                // meter somewhere else is the family's table or its driver
                // disagreeing with the hardware.
                needs_attention: error.is_some() || !landed,
                samples,
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
            Frames::SubStep { parent: step.id },
        );

        match selected {
            // The family reads the mode back before returning, and the step's
            // watcher still demands its matching frames.
            Ok(()) => Ok(true),
            Err(e) => {
                eprintln!(
                    "  {}",
                    style(format!(
                        "\u{21b3} {}",
                        tr!(
                            l,
                            "capture-drive-switch-failed",
                            mode = &*mode,
                            error = l.error(&e),
                        )
                    ))
                    .dim()
                );
                self.driver.fail();
                Ok(false)
            }
        }
    }

    /// Put the meter on one choice and file what it read back.
    ///
    /// A refusal is filed as the sub-step's error rather than retried: the
    /// protocol may simply be wrong about this family. Whether the setting has
    /// already worked this run is what decides whether the refusal is worth a
    /// maintainer's attention.
    fn drive_choice(
        &mut self,
        id: &str,
        setting: Setting,
        choice: &dmm_lib::protocol::Choice,
        samples_wanted: usize,
    ) -> Result<Driven, Box<dyn std::error::Error>> {
        let instruction = sub_step_instruction(setting, &choice.label);
        let proven = self.driver.proven(setting);
        recording::lock(&self.recorder).set_step(Some(id));

        let selected = match self.dmm.select(setting, choice.id) {
            // A lost link is no meter refusing the value: the run ends on it,
            // with nothing filed and no failure spent.
            Err(e) if link_gone(&e) => return Err(e.into()),
            selected => selected,
        };
        let mut errors = ErrorLog::default();
        let measurements = match &selected {
            Ok(()) => {
                self.driver.wait_to_settle();
                samples_after_switch(&mut self.dmm, samples_wanted, &mut errors)?
            }
            Err(_) => Vec::new(),
        };
        let samples = SampleData::from_all(&measurements);
        let diagnostics = errors.into_diagnostics();

        let hit = samples
            .iter()
            .any(|s| shows_target(setting, choice.id, &choice.label, s));
        let status = match &selected {
            Ok(()) => StepStatus::Captured,
            Err(_) => StepStatus::Error,
        };
        let error = selected.as_ref().err().map(ToString::to_string);
        let l = crate::l10n::get();
        match (selected.as_ref().err(), samples.last()) {
            (Some(e), _) => eprintln!(
                "    {} {}",
                style(format!("\u{21b3} {}", short_id(id))).dim(),
                style(l.error(e)).yellow()
            ),
            (None, Some(s)) => eprintln!(
                "    {}",
                style(format!("\u{21b3} {}  {}", short_id(id), s.summary())).dim()
            ),
            (None, None) => eprintln!(
                "    {}",
                style(format!(
                    "\u{21b3} {}  {}",
                    short_id(id),
                    tr!(l, "capture-drive-no-reading")
                ))
                .dim()
            ),
        }

        file_step(
            &mut self.report,
            &self.recorder,
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
                diagnostics,
                error,
                ..StepResult::new(id, &instruction, status)
            },
            Frames::Drained,
        );
        Ok(Driven {
            last: measurements.into_iter().next_back(),
            failed: selected.is_err(),
            hit,
        })
    }

    /// Put the setting back to auto range or off before the next mode step, and
    /// say so when the meter would not go. `charge` says whether that failure
    /// spends one of the run's failures.
    ///
    /// Filed as the sub-step `<step>/<setting>~restore` whatever the outcome,
    /// with its frames and the reading it left: a restore that failed leaves
    /// HOLD latched or a manual range under every later step, and in issue #20
    /// the report had nothing to explain the step that then went wrong.
    fn restore(
        &mut self,
        step_id: &str,
        setting: Setting,
        choices: &[dmm_lib::protocol::Choice],
        charge: bool,
    ) -> Result<Option<Measurement>, Box<dyn std::error::Error>> {
        // Back to auto range or off — or, on a setting with no such value (a
        // UT181A in Peak has no auto range), to the value the sweep started on.
        let Some(off) = choices
            .iter()
            .find(|c| c.id == RESET_ID)
            .or_else(|| choices.iter().find(|c| c.current))
        else {
            return Ok(None);
        };
        let id = restore_id(step_id, setting);
        recording::lock(&self.recorder).set_step(Some(&id));
        let selected = self.dmm.select(setting, off.id);
        // A failed walk can leave the meter in a mode without this setting
        // (RANGE once flipped a UT61E+ into AC+DC V): nothing to put back, but
        // the next setting is offered from where the meter is now.
        let gone_elsewhere = matches!(selected, Err(dmm_lib::error::Error::UnsupportedCommand(_)));
        let reading = if gone_elsewhere {
            self.dmm.request_measurement()
        } else {
            selected.and_then(|()| read_past_blank(&mut self.dmm))
        };
        // A lost link is no meter refusing to go back: the run ends on it.
        let reading = match reading {
            Err(e) if link_gone(&e) => return Err(e.into()),
            reading => reading,
        };
        let sample = reading.as_ref().ok().map(SampleData::from_measurement);
        let failure = match (&reading, &sample) {
            (Err(e), _) => Some(e.to_string()),
            _ if gone_elsewhere => None,
            (Ok(_), Some(s)) if shows_target(setting, off.id, &off.label, s) => None,
            (Ok(_), _) => Some(format!(
                "the meter did not read back {} {}",
                setting.name(),
                choice_slug(setting, &off.label)
            )),
        };
        // Filed as a driven choice is: an error only when a command or read
        // failed; a read-back that missed is a captured reading to look at.
        let errored = reading.is_err();
        file_step(
            &mut self.report,
            &self.recorder,
            StepResult {
                needs_attention: failure.is_some(),
                samples: sample.into_iter().collect(),
                error: failure.clone(),
                ..StepResult::new(
                    &id,
                    &sub_step_instruction(setting, &off.label),
                    if errored {
                        StepStatus::Error
                    } else {
                        StepStatus::Captured
                    },
                )
            },
            Frames::Drained,
        );
        // Saved now: a run cut short before the next step's save would lose
        // where the sweep left the meter.
        save_report(&self.report, &self.output_path)?;
        if failure.is_none() {
            return Ok(reading.ok());
        }
        eprintln!(
            "  {}",
            style(tr!(
                crate::l10n::get(),
                "capture-drive-reset-failed",
                setting = setting.name(),
            ))
            .yellow()
        );
        if charge {
            self.driver.fail();
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureReport;
    use dmm_lib::protocol::make_test_measurement;

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
    fn a_resumed_run_keeps_the_most_telling_drive_state() {
        assert_eq!(Drive::merged(None, Drive::On), Drive::On);
        assert_eq!(
            Drive::merged(Some(Drive::Disabled), Drive::On),
            Drive::Disabled
        );
        assert_eq!(
            Drive::merged(Some(Drive::Disabled), Drive::Off),
            Drive::Disabled
        );
        assert_eq!(Drive::merged(Some(Drive::On), Drive::Off), Drive::On);
        assert_eq!(
            Drive::merged(Some(Drive::On), Drive::Disabled),
            Drive::Disabled
        );
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
    fn swept(mode: dmm_lib::mock::MockMode, step_id: &'static str) -> CaptureReport {
        let (report, driver) = swept_by(mock(mode), step_id, Driver::new(true));
        assert_eq!(driver.state(), Drive::On, "the mock offers choices");
        report
    }

    /// The same sweep against any protocol, with the run's budget passed in
    /// and handed back so a test can seed it and read the refusals back.
    fn swept_by(
        proto: Box<dyn dmm_lib::protocol::Protocol>,
        step_id: &'static str,
        driver: Driver,
    ) -> (CaptureReport, Driver) {
        let mut run = CaptureRun {
            driver,
            ..run_on(proto)
        };
        let last = run.dmm.request_measurement().unwrap();

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
            starts_from: None,
        };
        let dir = tempfile::tempdir().unwrap();
        run.output_path = dir
            .path()
            .join("capture.yaml")
            .to_string_lossy()
            .to_string();

        run.sweep_step(&step, &last).unwrap();
        (run.report, run.driver)
    }

    /// Every offered value has to end up filed as a sub-step whose sample
    /// shows the meter took it.
    #[test]
    fn the_sweep_files_a_sub_step_per_offered_value() {
        let report = swept(dmm_lib::mock::MockMode::DcV, "dcv");

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

    /// A run on `proto`, recorded the way a capture run records it, with an
    /// empty report to file into and the sweeps on.
    fn run_on(proto: Box<dyn dmm_lib::protocol::Protocol>) -> CaptureRun {
        use dmm_lib::transport::{NullTransport, Transport};
        let (transport, recorder) =
            crate::capture::recording::RecordingTransport::new(Box::new(NullTransport));
        let dmm = dmm_lib::Dmm::new(Box::new(transport) as Box<dyn Transport>, proto).unwrap();
        CaptureRun {
            driver: Driver::new(true),
            ..CaptureRun::on(dmm, recorder)
        }
    }

    /// A meter on the mock's AC V ring, which offers the "AC V Hz" sub-mode
    /// the way a real dial position's button does, and the reading it shows.
    struct Bench {
        run: CaptureRun,
        last: Measurement,
    }

    fn meter(proto: Box<dyn dmm_lib::protocol::Protocol>) -> Bench {
        let mut run = run_on(proto);
        let last = run.dmm.request_measurement().unwrap();
        Bench { run, last }
    }

    impl Bench {
        fn switch(&mut self, step: &CaptureStep) -> bool {
            let last = self.last.clone();
            self.run.switch_mode(step, &last).unwrap()
        }

        fn mode(&mut self) -> String {
            self.run.dmm.request_measurement().unwrap().mode.to_string()
        }

        fn ids(&self) -> Vec<&str> {
            self.run
                .report
                .steps
                .iter()
                .map(|s| s.id.as_str())
                .collect()
        }

        fn failures(&self) -> u32 {
            self.run.driver.failures
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
            starts_from: None,
        }
    }

    /// The sub-mode is on the ring the dial already sits on, so the tool
    /// presses the button instead of asking, and files the switch as a
    /// sub-step of its own showing where the meter landed.
    #[test]
    fn a_mode_the_ring_offers_is_switched_to() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        assert!(bench.switch(&mode_step("AC V Hz")));
        assert_eq!(bench.mode(), "AC V Hz");
        assert_eq!(bench.failures(), 0);

        assert_eq!(bench.ids(), ["step/mode:AC V Hz"]);
        let filed = &bench.run.report.steps[0];
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

    /// A sub-step brings the report's count of dropped wire events up to
    /// date, as every step does: a run cut short after it still says how many
    /// events the recorder's bounded buffer lost.
    #[test]
    fn a_sub_step_files_the_dropped_wire_event_count() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        for _ in 0..recording::MAX_WIRE_EVENTS + 5 {
            bench.run.dmm.transport().write(&[0x01]).unwrap();
        }
        assert!(bench.switch(&mode_step("AC V Hz")));
        let dropped = recording::lock(&bench.run.recorder).dropped();
        assert!(dropped >= 5, "the buffer overflowed: {dropped}");
        assert_eq!(bench.run.report.wire_events_dropped, dropped);
    }

    /// Nothing earlier in the run left a reading, which is every step of a
    /// `--steps` run. The tool takes one rather than making the operator
    /// press the button.
    #[test]
    fn a_step_with_no_earlier_reading_still_switches() {
        let mut run = run_on(mock(dmm_lib::mock::MockMode::AcV));
        assert!(run.switch_mode_from(&mode_step("AC V Hz"), None).unwrap());
        assert_eq!(
            run.dmm.request_measurement().unwrap().mode.as_ref(),
            "AC V Hz"
        );
        assert_eq!(run.driver.failures, 0);
    }

    /// A mode the ring does not reach needs the dial turned, which is the
    /// operator's job: the tool must ask rather than send anything, and
    /// there is no switch to file.
    #[test]
    fn a_mode_off_the_ring_is_left_to_the_operator() {
        let mut bench = meter(mock(dmm_lib::mock::MockMode::AcV));
        assert!(!bench.switch(&mode_step("DC V")));
        assert_eq!(bench.mode(), "AC V");
        assert_eq!(bench.failures(), 0);
        assert!(bench.ids().is_empty());

        // A dial position offering no ring at all is the same case.
        let mut bench = meter(mock(dmm_lib::mock::MockMode::Ohm));
        assert!(!bench.switch(&mode_step("Capacitance")));
        assert_eq!(bench.mode(), "\u{03A9}");
        assert_eq!(bench.failures(), 0);
        assert!(bench.ids().is_empty());
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
        assert!(!bench.switch(&step));
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
        /// Every value lands, and the display then has no reading for this
        /// many reads, as a UT181A's does after a switch.
        Blanks(usize),
        /// The setting offers no reset value, as a UT181A's range in Peak
        /// offers no auto.
        NoReset,
        /// The link drops at the press, as a Bluetooth meter walking off does.
        LinkLost,
        /// Every value lands, and the press back to off or auto is accepted
        /// and does nothing.
        IgnoresReset,
    }

    struct Fails {
        inner: dmm_lib::mock::MockProtocol,
        setting: Setting,
        failure: Failure,
        /// Reads left before the display comes back, under `Blanks`.
        blank_reads: usize,
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
                blank_reads: 0,
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
        fn delivery(&self) -> dmm_lib::protocol::Delivery {
            self.inner.delivery()
        }
        fn discard_input(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<()> {
            self.inner.discard_input(t)
        }
        fn init(&mut self, t: &dyn dmm_lib::transport::Transport) -> dmm_lib::error::Result<()> {
            self.inner.init(t)
        }
        fn request_measurement(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<Measurement> {
            let mut reading = self.inner.request_measurement(t)?;
            if self.blank_reads > 0 {
                self.blank_reads -= 1;
                reading.value = MeasuredValue::NoReading("----");
                reading.display_raw = None;
            }
            Ok(reading)
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
            let mut choices = self.inner.choices(setting, current);
            if setting == self.setting && matches!(self.failure, Failure::NoReset) {
                choices.retain(|c| c.id != RESET_ID);
            }
            choices
        }
        fn select(
            &mut self,
            t: &dyn dmm_lib::transport::Transport,
            setting: Setting,
            id: u16,
        ) -> dmm_lib::error::Result<()> {
            use dmm_lib::error::{Error, Rejected, Unsupported};
            use dmm_lib::text::TextKind;
            // A press on the wire, so a sub-step's frames show it was sent.
            t.write(&[0xAB, 0xCD, id as u8])?;
            if setting != self.setting {
                return self.inner.select(t, setting, id);
            }
            match self.failure {
                Failure::NoReset => self.inner.select(t, setting, id),
                Failure::Blanks(reads) => {
                    self.inner.select(t, setting, id)?;
                    self.blank_reads = reads;
                    Ok(())
                }
                Failure::Silent => Err(Error::Timeout),
                Failure::LinkLost => Err(Error::cable_pulled()),
                Failure::RefusedIntoAnotherMode if id == RESET_ID => {
                    Err(Unsupported::Setting(setting).into())
                }
                Failure::IgnoresReset if id == RESET_ID => Ok(()),
                _ if id == RESET_ID => self.inner.select(t, setting, id),
                Failure::IgnoresReset => self.inner.select(t, setting, id),
                Failure::Refused | Failure::RefusedIntoAnotherMode => Err(Rejected::DidNothing {
                    button: "SELECT",
                    state: "AC V".into(),
                    kind: TextKind::Mode,
                }
                .into()),
                Failure::TimedOutAfterLanding => {
                    self.inner.select(t, setting, id)?;
                    Err(Error::Timeout)
                }
                Failure::RefusedAfterLanding => {
                    self.inner.select(t, setting, id)?;
                    Err(Rejected::GaveUp {
                        presses: 5,
                        button: "RANGE",
                        state: "2.2V".into(),
                        kind: TextKind::Range,
                    }
                    .into())
                }
            }
        }
    }

    /// A refused switch falls back to asking, and spends one of the three
    /// refusals that end remote control for the run.
    #[test]
    fn a_refused_switch_counts_against_the_budget() {
        let mut bench = meter(Fails::refusing(dmm_lib::mock::MockMode::AcV, Setting::Mode));
        assert!(!bench.switch(&mode_step("AC V Hz")));
        assert_eq!(bench.failures(), 1);
        assert!(bench.run.driver.active());
    }

    /// The refusal is in the report, not only on the terminal, with the
    /// reading that says where the meter was left — the evidence issue #20
    /// needed and a timeout's own text does not carry.
    #[test]
    fn a_refused_switch_is_filed_with_where_the_meter_was_left() {
        let mut bench = meter(Fails::refusing(dmm_lib::mock::MockMode::AcV, Setting::Mode));
        bench.switch(&mode_step("AC V Hz"));

        assert_eq!(bench.ids(), ["step/mode:AC V Hz"]);
        let filed = &bench.run.report.steps[0];
        assert_eq!(filed.status, StepStatus::Error);
        assert_eq!(
            filed.error.as_deref(),
            Some("command rejected: SELECT did nothing in AC V")
        );
        assert_eq!(filed.samples.last().map(|s| s.mode.as_str()), Some("AC V"));
        assert!(filed.needs_attention);
    }

    /// The meter refuses REL on an OL reading, so the sweep must not spend a
    /// failure on asking.
    #[test]
    fn rel_is_not_swept_while_the_reading_is_ol() {
        let report = swept(dmm_lib::mock::MockMode::OhmOl, "ohm_ol");
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

    /// A driven range is sampled once the meter's display is back, not in
    /// the moment it has no reading, which was all @diego351's UT181A
    /// capture filed for its range sub-steps (issue #5).
    #[test]
    fn a_driven_setting_is_sampled_once_the_display_is_back() {
        let (report, _) = swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Range,
                Failure::Blanks(3),
            ),
            "dcv",
            Driver::new(true),
        );
        let ranges: Vec<_> = report
            .steps
            .iter()
            .filter(|s| s.id.contains("/range:"))
            .collect();
        assert!(!ranges.is_empty());
        for step in ranges {
            assert!(
                step.samples.iter().all(|s| s.value != "----"),
                "{} sampled the blank display",
                step.id
            );
        }
        assert!(report.steps.iter().any(|s| s.id.contains("/rel:")));
    }

    /// A display that stays blank has no reading for REL to take as its
    /// reference, so REL is skipped the way it is on OL.
    #[test]
    fn rel_is_not_swept_while_the_display_stays_blank() {
        let (report, _) = swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Range,
                Failure::Blanks(1000),
            ),
            "dcv",
            Driver::new(true),
        );
        assert!(!report.steps.iter().any(|s| s.id.contains("/rel:")));
        assert!(report.steps.iter().any(|s| s.id == "dcv/hold:on"));
    }

    /// A switch leaves the step to sample after the blank, so the step's
    /// own samples are real readings.
    #[test]
    fn a_switch_waits_for_the_display_to_come_back() {
        let mut bench = meter(Fails::boxed(
            dmm_lib::mock::MockMode::AcV,
            Setting::Mode,
            Failure::Blanks(3),
        ));
        assert!(bench.switch(&mode_step("AC V Hz")));
        let filed = &bench.run.report.steps[0];
        assert_ne!(filed.samples.last().map(|s| s.value.as_str()), Some("----"));
        assert!(!matches!(
            bench.run.dmm.request_measurement().unwrap().value,
            MeasuredValue::NoReading(_)
        ));
    }

    /// A ladder with no auto range — a UT181A's in Peak — is put back on the
    /// rung the sweep started from, with no failure spent on a reset that
    /// does not exist.
    #[test]
    fn a_ladder_without_auto_goes_back_to_its_rung() {
        let mut run = run_on(Fails::boxed(
            dmm_lib::mock::MockMode::DcV,
            Setting::Range,
            Failure::NoReset,
        ));
        run.dmm.select(Setting::Range, 2).unwrap();
        let last = run.dmm.request_measurement().unwrap();
        let start = last.range_label.to_string();
        let dir = tempfile::tempdir().unwrap();
        run.output_path = dir
            .path()
            .join("capture.yaml")
            .to_string_lossy()
            .to_string();
        let step = CaptureStep {
            samples: 1,
            ..mode_step("DC V")
        };
        run.sweep_step(&step, &last).unwrap();

        assert!(run.report.steps.iter().any(|s| s.id.contains("/range:")));
        let now = run.dmm.request_measurement().unwrap();
        assert!(!now.flags.auto_range);
        assert_eq!(now.range_label, start);
        assert_eq!(run.driver.failures, 0);
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
        let (report, driver) = swept_by(
            Fails::refusing(dmm_lib::mock::MockMode::DcV, Setting::MinMax),
            "dcv",
            driver,
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
            let (report, _) = swept_by(
                Fails::boxed(dmm_lib::mock::MockMode::DcV, Setting::Hold, failure),
                "dcv",
                Driver::new(true),
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
            let (_, driver) = swept_by(
                Fails::boxed(dmm_lib::mock::MockMode::DcV, Setting::Hold, Failure::Silent),
                "dcv",
                driver,
            );
            assert_eq!(driver.failures, 1, "proven: {proven}");
        }
    }

    fn filed<'a>(report: &'a CaptureReport, id: &str) -> &'a StepResult {
        report
            .steps
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("{id} is not filed"))
    }

    /// A clean sweep files where each setting was left: a reading showing
    /// auto range and HOLD off, nothing flagged.
    #[test]
    fn a_clean_sweep_files_where_each_setting_was_left() {
        let report = swept(dmm_lib::mock::MockMode::DcV, "dcv");
        let range = filed(&report, "dcv/range~restore");
        let hold = filed(&report, "dcv/hold~restore");
        for restore in [range, hold] {
            assert_eq!(restore.status, StepStatus::Captured, "{:?}", restore.error);
            assert!(!restore.needs_attention, "{} was flagged", restore.id);
            assert!(restore.error.is_none());
        }
        assert_eq!(range.instruction, "range \u{2192} Auto (sent by the tool)");
        assert!(range.samples.last().is_some_and(|s| s.flags.auto_range));
        assert_eq!(hold.instruction, "hold \u{2192} off (sent by the tool)");
        assert!(hold.samples.last().is_some_and(|s| !s.flags.hold));
    }

    /// A restore the meter would not take reaches the report flagged — an
    /// error when the meter didn't answer, a captured reading when it
    /// answered from where it was left — with the frames of the presses: the
    /// evidence issue #20 lacked.
    #[test]
    fn a_failed_restore_is_filed_and_flagged() {
        let (report, _) = swept_by(
            Fails::boxed(dmm_lib::mock::MockMode::DcV, Setting::Hold, Failure::Silent),
            "dcv",
            Driver::new(true),
        );
        let restore = filed(&report, "dcv/hold~restore");
        assert_eq!(restore.status, StepStatus::Error);
        assert!(restore.needs_attention);
        assert!(restore.error.is_some());
        assert!(!restore.frames.is_empty(), "the press was not kept");

        let (report, _) = swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Hold,
                Failure::IgnoresReset,
            ),
            "dcv",
            Driver::new(true),
        );
        let restore = filed(&report, "dcv/hold~restore");
        assert_eq!(restore.status, StepStatus::Captured);
        assert!(restore.needs_attention);
        assert_eq!(
            restore.error.as_deref(),
            Some("the meter did not read back hold off")
        );
        assert!(!restore.frames.is_empty(), "the press was not kept");
        assert!(restore.samples.last().is_some_and(|s| s.flags.hold));
    }

    /// A press that landed before its error is still put back, and the
    /// restore says so with the reading it left.
    #[test]
    fn a_restore_after_a_landed_failure_shows_the_setting_back_off() {
        let (report, _) = swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Hold,
                Failure::TimedOutAfterLanding,
            ),
            "dcv",
            Driver::new(true),
        );
        let restore = filed(&report, "dcv/hold~restore");
        assert_eq!(restore.status, StepStatus::Captured, "{:?}", restore.error);
        assert!(!restore.needs_attention);
        assert!(!restore.frames.is_empty(), "the press was not kept");
        assert!(restore.samples.last().is_some_and(|s| !s.flags.hold));
    }

    /// A resumed run sweeps nothing its report already holds, so it neither
    /// retakes a restore nor files a second one.
    #[test]
    fn a_resumed_sweep_leaves_the_restores_alone() {
        let mut run = run_on(mock(dmm_lib::mock::MockMode::DcV));
        let last = run.dmm.request_measurement().unwrap();
        let dir = tempfile::tempdir().unwrap();
        run.output_path = dir
            .path()
            .join("capture.yaml")
            .to_string_lossy()
            .to_string();
        let step = CaptureStep {
            id: "dcv",
            ..mode_step("DC V")
        };
        run.sweep_step(&step, &last).unwrap();
        let before = serde_json::to_string(&run.report.steps).unwrap();
        assert!(before.contains("dcv/hold~restore"));

        run.driver = Driver::new(true);
        run.sweep_step(&step, &last).unwrap();
        assert_eq!(serde_json::to_string(&run.report.steps).unwrap(), before);
    }

    /// A proven setting refused is the meter being right, and a mode left
    /// with no such setting has nothing to restore: neither costs a failure,
    /// and the restore files the reading of where the meter went.
    #[test]
    fn a_restore_with_nothing_to_restore_costs_no_failure() {
        let mut driver = Driver::new(true);
        driver.prove_command("hold");
        let (report, driver) = swept_by(
            Fails::boxed(
                dmm_lib::mock::MockMode::DcV,
                Setting::Hold,
                Failure::RefusedIntoAnotherMode,
            ),
            "dcv",
            driver,
        );
        assert_eq!(driver.failures, 0);
        let restore = filed(&report, "dcv/hold~restore");
        assert_eq!(restore.status, StepStatus::Captured, "{:?}", restore.error);
        assert!(!restore.needs_attention);
        assert_eq!(restore.samples.len(), 1);
    }

    /// A link lost at a press is no refusal: the run ends there, with no
    /// sub-step filed against the protocol and no failure spent.
    #[test]
    fn a_lost_link_at_a_press_ends_the_run_without_filing_it() {
        let mut run = run_on(Fails::boxed(
            dmm_lib::mock::MockMode::DcV,
            Setting::Hold,
            Failure::LinkLost,
        ));
        let last = run.dmm.request_measurement().unwrap();
        let dir = tempfile::tempdir().unwrap();
        run.output_path = dir
            .path()
            .join("capture.yaml")
            .to_string_lossy()
            .to_string();
        let step = CaptureStep {
            id: "dcv",
            samples: 1,
            ..mode_step("DC V")
        };
        assert!(run.sweep_step(&step, &last).is_err());
        assert!(
            !run.report
                .steps
                .iter()
                .any(|s| s.id.starts_with("dcv/hold:"))
        );
        assert_eq!(run.driver.failures, 0);

        let mut bench = meter(Fails::boxed(
            dmm_lib::mock::MockMode::AcV,
            Setting::Mode,
            Failure::LinkLost,
        ));
        let last = bench.last.clone();
        assert!(bench.run.switch_mode(&mode_step("AC V Hz"), &last).is_err());
        assert!(bench.ids().is_empty());
        assert_eq!(bench.failures(), 0);
    }

    /// A setting nothing has driven yet is the case the budget exists for,
    /// and one refusal answers for every value of it.
    #[test]
    fn a_refusal_of_an_unproven_setting_spends_one_failure() {
        let (report, driver) = swept_by(
            Fails::refusing(dmm_lib::mock::MockMode::DcV, Setting::MinMax),
            "dcv",
            Driver::new(true),
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
