//! One capture step: the definition the run walks, the wait for the state it
//! asks for, and the frames that wait puts on the wire.

use super::input::{ErrorLog, Input};
use super::recording::{self, SharedRecorder, WireEvent};
use super::report::{
    CaptureReport, FrameRecord, SampleData, StepResult, StepStatus, already_captured,
    needs_attention, upsert_step,
};
use super::session::CaptureRun;
use super::watch::{Baseline, STABLE_FRAMES, StateWatcher, Verdict, enter_only};
use crate::open::BoxedDmm;
use console::{Key, style};
use dmm_lib::error::{Error, ErrorKind};
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::protocol::{Mismatch, RangeExpect, RangeShown, Shown, ValueExpect};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use std::time::{Duration, Instant};

/// Cap on wire events recorded per step, so one chatty step can't grow the
/// report without bound. The oldest are trimmed, and counted in the step's
/// `frames_dropped` (see [`frames_for_step`]).
pub(crate) const MAX_FRAMES_PER_STEP: usize = 500;

/// Filter keyword for the freeform capture pass. Not a protocol step — the
/// pass generates `extra_0`, `extra_1`, … as the user describes each capture.
pub(crate) const FREEFORM_STEP_ID: &str = "extra";

/// Filter keyword for the auto-detection check that ends a run. Not a
/// protocol step either — it files the report's `detection` section.
pub(crate) const DETECTION_STEP_ID: &str = "detect";

pub(crate) use dmm_lib::protocol::CaptureStep;

/// The line announcing the step. Gate steps say so: skipping one leaves the
/// rest of the run uninterpretable.
fn header(l: &Localizer, step: &CaptureStep) -> String {
    let id = style(format!("[{}]", step.id)).cyan().bold();
    let gate = if step.gate {
        format!(" {}", style(tr!(l, "capture-gate-mark")).dim())
    } else {
        String::new()
    };
    format!("{id} {}{gate}", super::instructions::shown(step))
}

/// Create a StepResult with no samples or screen capture.
pub(super) fn empty_result(
    step: &CaptureStep,
    status: StepStatus,
    error: Option<String>,
) -> StepResult {
    StepResult {
        error,
        ..StepResult::new(step.id, step.instruction, status)
    }
}

/// Whether `e` says the meter is out of reach — a pulled cable, a dropped
/// Bluetooth link, a Ctrl-C mid-read — rather than that one frame went
/// wrong. Every later read fails the same way at once, so the run ends on it:
/// polling on would spin and file empty steps.
pub(crate) fn link_gone(e: &Error) -> bool {
    matches!(
        e.kind(),
        ErrorKind::Transport | ErrorKind::DeviceNotFound | ErrorKind::Interrupted
    )
}

/// A failed read in a sampling loop: a timeout is skipped, a lost link
/// ([`link_gone`]) is handed back to end the run, anything else is recorded
/// and the loop goes on.
fn triage(e: Error, errors: &mut ErrorLog) -> dmm_lib::error::Result<()> {
    match e {
        Error::Timeout => Ok(()),
        e if link_gone(&e) => Err(e),
        e => {
            errors.record(&e);
            Ok(())
        }
    }
}

/// Poll for `n` samples.
///
/// A parse rejection is recorded and polling continues: a meter whose frames
/// we misread would otherwise end the step on the first bad frame, and the
/// report showed neither a sample nor a reason. A lost link ends the poll
/// ([`link_gone`]).
pub(crate) fn capture_samples(
    dmm: &mut BoxedDmm,
    n: usize,
    errors: &mut ErrorLog,
) -> dmm_lib::error::Result<Vec<Measurement>> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(e) => triage(e, errors)?,
        }
        attempts += 1;
    }
    Ok(samples)
}

/// Readings to wait through for a meter's display to come back after a
/// change. A UT181A has no reading for six or seven 100 ms frames after any
/// range, mode or dial change, and for up to 20 on some positions (its
/// research spec, §5.2); a capacitance range with nothing connected never
/// gets one, so the wait is bounded. On any meter, the first reading ends it.
const BLANK_READS: usize = 25;

/// Whether the display shows no reading.
fn is_blank(m: &Measurement) -> bool {
    matches!(m.value, MeasuredValue::NoReading(_))
}

/// The first reading that is not a blank display, or the last blank one once
/// [`BLANK_READS`] have gone by.
pub(crate) fn read_past_blank(dmm: &mut BoxedDmm) -> dmm_lib::error::Result<Measurement> {
    let mut reading = dmm.request_measurement()?;
    for _ in 1..BLANK_READS {
        if !is_blank(&reading) {
            break;
        }
        reading = dmm.request_measurement()?;
    }
    Ok(reading)
}

/// `n` samples after a switch the tool made, the first of them once the
/// display is back.
pub(crate) fn samples_after_switch(
    dmm: &mut BoxedDmm,
    n: usize,
    errors: &mut ErrorLog,
) -> dmm_lib::error::Result<Vec<Measurement>> {
    let mut samples = Vec::new();
    if n > 0 {
        match read_past_blank(dmm) {
            Ok(m) => samples.push(m),
            Err(e) => triage(e, errors)?,
        }
    }
    let rest = n.saturating_sub(samples.len());
    samples.extend(capture_samples(dmm, rest, errors)?);
    Ok(samples)
}

/// Wire events belonging to `step_id`, capped, and how many older ones the
/// cap trimmed.
///
/// The newest are kept: a step spends its wait watching the meter, and the
/// frames worth reading are the sampled ones at the end of it. The count is
/// returned rather than written into `diagnostics` because a step that waits
/// on a person passes the cap as a matter of course, and every diagnostic
/// flags the step for a maintainer to read.
pub(crate) fn frames_for_step(events: &[WireEvent], step_id: &str) -> (Vec<FrameRecord>, u64) {
    let mine: Vec<&WireEvent> = events
        .iter()
        .filter(|e| e.step.as_deref() == Some(step_id))
        .collect();
    let dropped = mine.len().saturating_sub(MAX_FRAMES_PER_STEP);
    (
        mine.into_iter()
            .skip(dropped)
            .map(FrameRecord::from)
            .collect(),
        dropped as u64,
    )
}

/// Where a step's frames come from when it is filed.
pub(crate) enum Frames<'a> {
    /// Everything the recorder holds: the step ran on its own, and nothing is
    /// tagged until the next one starts.
    Drained,
    /// Only the sub-step's own events: it is filed partway through step
    /// `parent`, whose frames stay buffered and which goes on recording.
    SubStep { parent: &'a str },
}

/// Take step `id`'s frames out of the recorder, as `from` says, capped as
/// [`frames_for_step`] caps them.
pub(crate) fn take_frames(
    recorder: &SharedRecorder,
    id: &str,
    from: Frames,
) -> (Vec<FrameRecord>, u64) {
    let mut rec = recording::lock(recorder);
    match from {
        Frames::Drained => {
            rec.set_step(None);
            frames_for_step(&rec.drain(), id)
        }
        Frames::SubStep { parent } => {
            rec.set_step(Some(parent));
            frames_for_step(&rec.take_step(id), id)
        }
    }
}

/// File `result` in the report with the frames its step left in the
/// recorder, taken as `from` says. Take them where the step finished
/// sampling: nothing may reach the wire between that and this.
pub(crate) fn file_step(
    report: &mut CaptureReport,
    recorder: &SharedRecorder,
    mut result: StepResult,
    from: Frames,
) {
    (result.frames, result.frames_dropped) = take_frames(recorder, &result.id, from);
    file_taken(report, recorder, result);
}

/// [`file_step`] for a result that already holds its frames, taken before
/// the step asked the operator anything. The report's count of wire events
/// the recorder dropped is brought up to date with it, so a run cut short
/// after any step says how many were lost.
pub(crate) fn file_taken(
    report: &mut CaptureReport,
    recorder: &SharedRecorder,
    result: StepResult,
) {
    report.wire_events_dropped = recording::lock(recorder).dropped();
    upsert_step(report, result);
}

/// How long a step watches the meter before offering the keyboard: long
/// enough to fetch a thermocouple, short enough not to look stuck.
const STEP_TIMEOUT: Duration = Duration::from_secs(45);

/// How long a command step waits for the meter to react before calling the
/// command a no-op. The meter answers a button in a frame or two.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);

/// What the step before this one left behind.
#[derive(Default)]
pub(crate) struct PrevState {
    /// The payload bytes it held constant, so a new state can be told from
    /// it. Read back from the report, so a resumed run has one too.
    pub baseline: Option<Baseline>,
    /// Its last reading. In-memory only: `None` after a resume, and after a
    /// step the operator skipped, where the meter's state is anyone's guess.
    pub last: Option<Measurement>,
}

/// What a step left behind: whether the operator asked to finish, and the
/// reading the next step measures its own expectation against.
pub(crate) struct StepOutcome {
    pub(super) quit: bool,
    pub(super) last: Option<Measurement>,
    /// It captured a reading nobody was asked about, so the end-of-run review
    /// has to cover it.
    pub(super) to_review: bool,
}

impl StepOutcome {
    /// The step is done and the run goes on; `last` where it captured one.
    fn done(last: Option<Measurement>, to_review: bool) -> Self {
        StepOutcome {
            quit: false,
            last,
            to_review,
        }
    }

    /// The step captured nothing — skipped, refused or ignored — so the next
    /// one has no reading to compare against.
    fn nothing() -> Self {
        StepOutcome {
            quit: false,
            last: None,
            to_review: false,
        }
    }
}

/// Why the wait for a step's state ended.
enum Watched {
    /// The state is on screen; the frame that clinched it, unless the
    /// operator's Enter cut the wait short.
    Ready(Option<Measurement>),
    /// Nothing new settled within the timeout; the last reading, for the
    /// report to say what the meter was showing instead.
    TimedOut(Option<Measurement>),
    Skip,
    Quit,
}

/// What a wait for a step's state does once its timeout passes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OnTimeout {
    /// End the wait: [`Watched::TimedOut`].
    GiveUp,
    /// Offer the keyboard and keep waiting: a step whose instruction takes a
    /// while to carry out must not file whatever the meter happened to be
    /// showing.
    OfferKeys,
}

/// Read until the meter shows what the step asked for.
fn watch_for_state(
    dmm: &mut BoxedDmm,
    input: &Input,
    watcher: &mut StateWatcher,
    timeout: Duration,
    on_timeout: OnTimeout,
    errors: &mut ErrorLog,
) -> Result<Watched, Box<dyn std::error::Error>> {
    let l = crate::l10n::get();
    let start = Instant::now();
    let mut hinted = false;
    let mut last = None;
    loop {
        match input.try_key()? {
            Some(Key::Enter) => return Ok(Watched::Ready(None)),
            Some(Key::Char('s' | 'S')) => return Ok(Watched::Skip),
            Some(Key::Char('q' | 'Q')) => return Ok(Watched::Quit),
            _ => {}
        }

        match dmm.request_measurement() {
            Ok(m) => match watcher.feed(&m) {
                Verdict::Ready => return Ok(Watched::Ready(Some(m))),
                Verdict::Mismatch(reason) => {
                    last = Some(m);
                    eprintln!("  {}", style(mismatch_line(l, &reason)).dim());
                }
                Verdict::Waiting => last = Some(m),
            },
            Err(Error::Timeout) => {}
            // `?` first: the wait may have no timeout, and a link that fails
            // every read at once would spin in it.
            Err(e) => {
                triage(e, errors)?;
                watcher.feed_error();
            }
        }

        // `checked_duration_since`: a backward clock jump must not strand the
        // step in a wait that never times out.
        let waited = Instant::now()
            .checked_duration_since(start)
            .unwrap_or_default();
        if waited >= timeout {
            if on_timeout == OnTimeout::GiveUp {
                return Ok(Watched::TimedOut(last));
            }
            if !hinted {
                hinted = true;
                eprintln!(
                    "  {}",
                    style(tr!(l, "capture-press-enter", skip = "s", quit = "q")).dim()
                );
            }
        }
    }
}

/// A mismatch line, in `l`'s language: what [`Mismatch`]'s `Display` says,
/// after "meter shows:".
fn mismatch_line(l: &Localizer, mismatch: &Mismatch) -> String {
    fn shown(got: &Shown) -> (&'static str, &str) {
        match got {
            Shown::Text(text) => ("value", text),
            Shown::NoMainReading => ("absent", ""),
        }
    }
    match mismatch {
        Mismatch::Mode { got, want } => tr!(
            l,
            "capture-mismatch-mode",
            got = format!("{:?}", l.label(TextKind::Mode, got)),
            want = format!("{:?}", l.label(TextKind::Mode, want)),
        ),
        Mismatch::Flag { flag, want } => tr!(
            l,
            "capture-mismatch-flag",
            flag = l
                .flag(*flag)
                .unwrap_or(std::borrow::Cow::Borrowed(flag.name())),
            want = if *want { "on" } else { "off" },
        ),
        Mismatch::Range { got, want } => {
            let (kind, range) = match got {
                RangeShown::Unlabelled => ("none", ""),
                RangeShown::AutoRung(label) => ("auto", label.as_ref()),
                RangeShown::Label(label) => ("plain", label.as_ref()),
            };
            tr!(
                l,
                "capture-mismatch-range",
                range = l.label(TextKind::Range, range),
                shown = kind,
                want = match want {
                    RangeExpect::Auto => "auto",
                    RangeExpect::Manual => "manual",
                },
            )
        }
        Mismatch::Value { got, want } => {
            let (kind, value) = shown(got);
            tr!(
                l,
                "capture-mismatch-value",
                shown = kind,
                value = value,
                want = match want {
                    ValueExpect::Overload => "overload",
                    ValueExpect::Negative => "negative",
                    ValueExpect::Finite => "finite",
                    ValueExpect::NcvDetected => "ncv",
                    ValueExpect::NoReading => "word",
                    ValueExpect::Zero => "zero",
                },
            )
        }
        Mismatch::AtLeast { got, min } => {
            let (kind, value) = shown(got);
            tr!(
                l,
                "capture-mismatch-at-least",
                shown = kind,
                value = value,
                min = min.to_string(),
            )
        }
    }
    .into_owned()
}

/// The wording for a command the meter ignored, in the terms `cycle.rs` uses
/// for the same failure: in `l`'s language on the terminal, in English in
/// the report.
fn did_nothing(l: &Localizer, command: &str, last: Option<&Measurement>) -> String {
    match last {
        Some(m) => tr!(
            l,
            "capture-did-nothing-shows",
            command = command,
            reading = SampleData::from_measurement(m).summary(),
        ),
        None => tr!(l, "capture-did-nothing-silent", command = command),
    }
    .into_owned()
}

/// What the operator answered at a confirmation prompt.
#[derive(Debug, PartialEq, Eq)]
enum Confirmation {
    /// The reading matched the meter's screen.
    Yes,
    /// It did not, so what the meter showed is asked for next.
    No,
    /// Do the step over: the attempt's samples are dropped.
    Retake,
}

impl Confirmation {
    /// The answers this prompt takes, and nothing else: `None` re-asks.
    ///
    /// A closed set, because the prompt used to take a correction in the same
    /// line — so a reporter answering "correct" had that filed as what the
    /// meter showed, marking the gate step they had just confirmed as failed.
    ///
    /// Where there is nothing to redo, `r` is not one of the answers.
    fn of(answer: &str, retake: Retake) -> Option<Self> {
        match answer.to_ascii_lowercase().as_str() {
            "" => Some(Confirmation::Yes),
            "n" | "no" => Some(Confirmation::No),
            "r" | "retake" if retake == Retake::Offered => Some(Confirmation::Retake),
            _ => None,
        }
    }
}

/// Whether a confirmation prompt offers to do the step over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Retake {
    Offered,
    NotOffered,
}

/// What a confirmation prompt settled on.
pub(super) enum Confirmed {
    /// What the meter showed when the operator said our reading was wrong —
    /// blank when they did not say — and `None` when they confirmed it.
    Reading(Option<String>),
    /// Do the step over, offered only where the caller asked for it.
    Retake,
}

/// Ask whether the reading matched the meter's screen, and what it showed when
/// it did not.
///
/// The closed question comes first and re-asks anything it doesn't recognise,
/// the way the end-of-run review re-asks a bad index: a word typed here is
/// never filed as the meter's screen.
pub(super) fn ask_confirmation(
    input: &Input,
    retake: Retake,
) -> Result<Confirmed, Box<dyn std::error::Error>> {
    let l = crate::l10n::get();
    let retake_arg = match retake {
        Retake::Offered => "yes",
        Retake::NotOffered => "no",
    };
    loop {
        let question = tr!(
            l,
            "capture-confirm",
            retake = retake_arg,
            no = "n",
            redo = "r",
        );
        let answer = input.line(&format!("  {} ", style(question).dim()))?;
        match Confirmation::of(&answer, retake) {
            Some(Confirmation::Yes) => return Ok(Confirmed::Reading(None)),
            Some(Confirmation::No) => {
                let shown =
                    input.line(&format!("  {} ", style(tr!(l, "capture-what-shown")).dim()))?;
                // Echoed because this is the answer that marks a gate step as
                // failed; a plain Enter above records nothing and says nothing.
                // Saying nothing here is still a mismatch, so it is echoed as
                // one rather than as a meter showing an empty screen.
                let recorded = match shown.as_str() {
                    "" => tr!(l, "capture-recorded-mismatch"),
                    text => tr!(l, "capture-recorded-shown", shown = format!("{text:?}")),
                };
                eprintln!("  {}", style(recorded).dim());
                return Ok(Confirmed::Reading(Some(shown)));
            }
            Some(Confirmation::Retake) => return Ok(Confirmed::Retake),
            None => eprintln!(
                "  {}",
                style(tr!(
                    l,
                    "capture-confirm-again",
                    retake = retake_arg,
                    no = "n",
                    redo = "r",
                ))
                .yellow()
            ),
        }
    }
}

/// The mode a step's samples left, and the first one they showed instead.
///
/// Only when the first sample is in the step's expected mode: the operator
/// moved on — pressed SELECT through Continuity to Diode — before sampling
/// was done, and the samples mix two modes (issue #16). An Enter capture
/// that was never in the mode is filed as it stands, and only the mode is
/// compared, so a value that wanders, such as body resistance, is no reason
/// to retake.
fn left_mode<'a>(
    expect: Option<&dmm_lib::protocol::Expect>,
    samples: &'a [Measurement],
) -> Option<(&'static str, &'a str)> {
    let want = expect?.mode?;
    let (first, rest) = samples.split_first()?;
    if first.mode != want {
        return None;
    }
    rest.iter()
        .find(|m| m.mode != want)
        .map(|m| (want, m.mode.as_ref()))
}

impl CaptureRun {
    /// File a step that ended before it sampled: the frames it did put on the
    /// wire are drained out of the recorder and into its result.
    fn finish_failed_step(&mut self, mut result: StepResult) {
        result.needs_attention = true;
        file_step(&mut self.report, &self.recorder, result, Frames::Drained);
    }

    /// File the step as skipped; the run goes on.
    fn skip(&mut self, step: &CaptureStep) -> StepOutcome {
        upsert_step(
            &mut self.report,
            empty_result(step, StepStatus::Skipped, None),
        );
        StepOutcome::nothing()
    }

    /// File the step as skipped, and stop the run here.
    fn quit(&mut self, step: &CaptureStep) -> StepOutcome {
        StepOutcome {
            quit: true,
            ..self.skip(step)
        }
    }

    /// Hold a command step until the meter shows the state it starts from, with
    /// Enter to send anyway: our reading of the state may be what is wrong.
    /// Where the run does not read the parse, only Enter can end the wait.
    fn wait_for_start(
        &mut self,
        state: dmm_lib::protocol::Expect,
        errors: &mut ErrorLog,
    ) -> Result<Watched, Box<dyn std::error::Error>> {
        let l = crate::l10n::get();
        let (mut watcher, timeout) = if self.trust.reads_parse() {
            eprintln!(
                "  {}",
                style(tr!(l, "capture-wait-start", skip = "s", quit = "q")).dim()
            );
            (
                StateWatcher::for_step(Some(state), None, true),
                STEP_TIMEOUT,
            )
        } else {
            (StateWatcher::for_step(None, None, false), Duration::ZERO)
        };
        watch_for_state(
            &mut self.dmm,
            &self.input,
            &mut watcher,
            timeout,
            OnTimeout::OfferKeys,
            errors,
        )
    }

    /// File a step the operator gave up on while it waited to start, with its
    /// frames: they show the state it never saw.
    fn skip_from_wait(&mut self, step: &CaptureStep, errors: ErrorLog, quit: bool) -> StepOutcome {
        let mut result = empty_result(step, StepStatus::Skipped, None);
        result.diagnostics = errors.into_diagnostics();
        file_step(&mut self.report, &self.recorder, result, Frames::Drained);
        StepOutcome {
            quit,
            ..StepOutcome::nothing()
        }
    }

    /// Run one capture step, and say what it left behind for the next.
    pub(super) fn capture_step(
        &mut self,
        step: &CaptureStep,
        interactive: bool,
        prev: &PrevState,
    ) -> Result<StepOutcome, Box<dyn std::error::Error>> {
        let l = crate::l10n::get();
        // Check if already captured (resume)
        if already_captured(&self.report, step.id) {
            eprintln!(
                "  {}",
                tr!(
                    l,
                    "capture-already-captured",
                    step = style(step.id).dim().to_string(),
                )
            );
            return Ok(StepOutcome::nothing());
        }

        if interactive {
            eprintln!();
        }
        eprintln!("{}", header(l, step));

        recording::lock(&self.recorder).set_step(Some(step.id));
        let mut errors = ErrorLog::default();
        let expect = self.trust.expect(step);

        // A mode a button on the meter reaches — continuity from Ω, duty
        // from Hz — is the tool's to switch to; only the dial is the operator's.
        if interactive && self.trust.drives() {
            self.switch_mode_from(step, prev.last.as_ref())?;
        }

        // One pass per attempt: `r` at the confirmation prompt drops the samples
        // and runs the same wait again, on the same previous state. The recorder
        // is not drained in between, so every attempt's frames reach the report.
        let mut attempt = 0usize;
        let (sample_data, mut measurements, confirmation) = loop {
            attempt += 1;
            // Anything typed before the step was announced answered the last
            // prompt, not this step's wait.
            self.input.drain_keys();
            // The frame the watcher accepted, kept as the step's first sample: it
            // is the one reading known to be in the state the step asked for.
            // `--settle` drops it below — there it is the transient.
            let mut settled: Option<Measurement> = None;

            // A retake re-samples; it does not press the button again, which on a
            // toggle like HOLD would undo the state the first press reached.
            if let Some(cmd) = step.command.filter(|_| attempt == 1) {
                // A piped run has no one to wait for: it sends at once.
                if interactive
                    && self.input.is_tty()
                    && let Some(state) = step.starts_from
                {
                    match self.wait_for_start(state, &mut errors)? {
                        Watched::Skip => return Ok(self.skip_from_wait(step, errors, false)),
                        Watched::Quit => return Ok(self.skip_from_wait(step, errors, true)),
                        Watched::Ready(_) | Watched::TimedOut(_) => {}
                    }
                }

                // What the meter shows before the button is pressed, so a command
                // that changes nothing can be told from one that works.
                let before = capture_samples(&mut self.dmm, STABLE_FRAMES, &mut errors)?;

                // An Enter pressed twice at the wait must not end the wait for
                // the command's effect as well, which would file the frames
                // from before it took hold.
                if step.starts_from.is_some() {
                    self.input.drain_keys();
                }

                match self.dmm.send_command(cmd) {
                    Ok(()) => {}
                    // A pulled cable is no command the meter refused: the run ends on it.
                    Err(e) if link_gone(&e) => return Err(e.into()),
                    Err(e) => {
                        eprintln!(
                            "  {}",
                            style(tr!(l, "capture-command-failed", error = l.error(&e))).red()
                        );
                        let result = empty_result(step, StepStatus::Error, Some(e.to_string()));
                        self.finish_failed_step(result);
                        return Ok(StepOutcome::nothing());
                    }
                }

                let mut watcher =
                    StateWatcher::for_command(expect, &before, self.trust.reads_parse());
                match watch_for_state(
                    &mut self.dmm,
                    &self.input,
                    &mut watcher,
                    COMMAND_TIMEOUT,
                    OnTimeout::GiveUp,
                    &mut errors,
                )? {
                    Watched::Ready(m) => {
                        self.driver.prove_command(cmd);
                        settled = m;
                    }
                    Watched::TimedOut(last) => {
                        // No samples: filing pre-command frames as the step's result
                        // is what made a dead command look like a captured state.
                        let error = did_nothing(crate::l10n::english(), cmd, last.as_ref());
                        eprintln!("  {}", style(did_nothing(l, cmd, last.as_ref())).yellow());
                        let mut result = empty_result(step, StepStatus::Error, Some(error));
                        result.diagnostics = errors.into_diagnostics();
                        self.finish_failed_step(result);
                        return Ok(StepOutcome::nothing());
                    }
                    Watched::Skip => return Ok(self.skip(step)),
                    Watched::Quit => return Ok(self.quit(step)),
                }
            } else if interactive && self.input.is_tty() {
                // A step whose expectation the previous reading already satisfies
                // cannot be seen arriving — DC V with the leads open and shorted
                // both read about zero — so it is Enter-only and the keyboard is
                // offered at once. A mode the tool just switched to is a change, so
                // this is false there. A step that declares it waits for Enter is
                // Enter-only wherever it runs.
                let ask = step.wait_for_enter || enter_only(expect, prev.last.as_ref());
                // Something has to go on the probes, and the dial is usually
                // turned first: Enter is offered at once, and the watcher only
                // captures on its own once it has seen the reading fail first.
                let gated = !step.needs.is_empty();
                let timeout = if ask || gated {
                    Duration::ZERO
                } else {
                    STEP_TIMEOUT
                };
                let mut watcher = StateWatcher::for_step(expect, prev.baseline.as_ref(), !ask);
                if gated {
                    watcher = watcher.gated();
                }
                match watch_for_state(
                    &mut self.dmm,
                    &self.input,
                    &mut watcher,
                    timeout,
                    OnTimeout::OfferKeys,
                    &mut errors,
                )? {
                    Watched::Ready(m) => settled = m,
                    // `OfferKeys` keeps the wait open, so the timeout never ends it.
                    Watched::TimedOut(_) => {}
                    Watched::Skip => return Ok(self.skip(step)),
                    Watched::Quit => return Ok(self.quit(step)),
                }
            } else if interactive {
                // No terminal to poll: ask, the way this step always did.
                let ch = self.input.key(&format!(
                    "  {} ",
                    style(tr!(l, "capture-any-key", skip = "s", quit = "q")).dim()
                ))?;
                if ch == 'q' || ch == 'Q' {
                    return Ok(self.quit(step));
                }
                if ch == 's' || ch == 'S' {
                    return Ok(self.skip(step));
                }
            }

            // The wait covers the operator's own changes as much as the tool's:
            // whatever ended this step's wait, `--settle` says the reading behind
            // it needs longer. The frame that ended the wait was read before it,
            // so a run that waits reads its whole batch afterwards.
            if self.driver.wait_to_settle() {
                settled = None;
            }
            // A display that is blank for a moment after the change is read past,
            // unless no reading is what the step is there to see.
            let expects_blank = expect.is_some_and(|e| e.value == Some(ValueExpect::NoReading));
            if !expects_blank && settled.as_ref().is_none_or(is_blank) {
                settled = match read_past_blank(&mut self.dmm) {
                    Ok(m) => Some(m),
                    Err(e) if link_gone(&e) => return Err(e.into()),
                    Err(_) => None,
                };
            }
            let mut measurements: Vec<Measurement> = settled.into_iter().collect();
            let wanted = step.samples.saturating_sub(measurements.len());
            measurements.extend(capture_samples(&mut self.dmm, wanted, &mut errors)?);

            // Only where the wait offers s and q: a meter that keeps leaving
            // would otherwise retake the step for ever.
            if interactive
                && self.input.is_tty()
                && let Some((mode, other)) = left_mode(expect.as_ref(), &measurements)
            {
                eprintln!(
                    "  {}",
                    style(tr!(
                        l,
                        "capture-left-mode",
                        mode = l.label(TextKind::Mode, mode),
                        other = l.label(TextKind::Mode, other),
                    ))
                    .yellow()
                );
                continue;
            }

            let sample_data = SampleData::from_all(&measurements);

            for (i, s) in sample_data.iter().enumerate() {
                eprintln!(
                    "    {} mode={}({}) range={} display={:?}",
                    style(format!("[{i}]")).dim(),
                    s.mode_byte,
                    s.mode,
                    s.range_label,
                    s.shown()
                );
            }

            let confirmation = if let Some(last) = sample_data.last() {
                if interactive {
                    eprintln!(
                        "  {}",
                        tr!(
                            l,
                            "capture-we-read",
                            reading = style(last.summary()).green().to_string(),
                        )
                    );
                }
                // A trusted run doesn't stop here: the reading is listed with the
                // others in the one review at the end.
                //
                // `is_tty`, as the end-of-run review does: a piped run's stdin
                // answers every prompt with the empty line EOF gives, which reads
                // as "yes, that is what the meter showed" — and a gate confirmed
                // that way promotes the run and writes `core_semantics: confirmed`
                // into the report without anyone having looked at the meter.
                if interactive && self.input.is_tty() && self.trust.confirm_inline(step) {
                    match ask_confirmation(&self.input, Retake::Offered)? {
                        Confirmed::Retake => {
                            eprintln!("  {}", style(tr!(l, "capture-retaking")).dim());
                            continue;
                        }
                        Confirmed::Reading(shown) => Some(shown),
                    }
                } else {
                    None
                }
            } else {
                eprintln!("  {}", style(tr!(l, "capture-no-response")).yellow());
                None
            };

            break (sample_data, measurements, confirmation);
        };

        let diagnostics = errors.into_diagnostics();

        let status = if sample_data.is_empty() {
            StepStatus::Timeout
        } else if sample_data.len() < step.samples {
            eprintln!(
                "  {} {}",
                style(tr!(l, "capture-few-samples-label")).yellow(),
                tr!(
                    l,
                    "capture-few-samples",
                    got = sample_data.len(),
                    wanted = step.samples,
                )
            );
            StepStatus::Captured
        } else {
            StepStatus::Captured
        };

        let to_review = confirmation.is_none() && status == StepStatus::Captured;
        let mut result = StepResult {
            needs_attention: needs_attention(&sample_data, step.samples, &diagnostics),
            samples: sample_data,
            diagnostics,
            ..StepResult::new(step.id, step.instruction, status)
        };
        if let Some(shown) = confirmation {
            result.set_inline_confirmation(shown);
        }

        file_step(&mut self.report, &self.recorder, result, Frames::Drained);
        Ok(StepOutcome::done(measurements.pop(), to_review))
    }
}

#[cfg(test)]
pub(super) fn cli_step(id: &'static str, verified: bool, gate: bool) -> CaptureStep {
    CaptureStep {
        id,
        instruction: "do the thing",
        command: None,
        samples: 5,
        expect: None,
        verified,
        gate,
        needs: &[],
        wait_for_enter: false,
        starts_from: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::make_test_measurement;
    use dmm_lib::transport::mock::MockTransport;
    use std::sync::{Arc, Mutex};

    /// A UT61E+ frame: AB CD, length, payload, 16-bit BE sum.
    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut f = vec![0xAB, 0xCD, (payload.len() + 2) as u8];
        f.extend_from_slice(payload);
        let sum = f.iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        f.extend_from_slice(&sum.to_be_bytes());
        f
    }

    /// A run on `dmm` whose frames go nowhere: the step's frames are recorded
    /// through the transport the recorder wraps, and this one only has to
    /// exist.
    fn run_on(dmm: BoxedDmm) -> CaptureRun {
        let (_unused, recorder) = crate::capture::recording::RecordingTransport::new(Box::new(
            dmm_lib::transport::NullTransport,
        ));
        CaptureRun::on(dmm, recorder)
    }

    fn measurement_frame() -> Vec<u8> {
        frame(&[
            0x02, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, 0x30, 0x30, 0x30,
        ])
    }

    /// A meter streaming `frames` one per [`StreamingTransport::PERIOD`] of
    /// real time, as a UT181A streams, rather than all queued at once.
    struct StreamingTransport {
        frames: Mutex<std::collections::VecDeque<Vec<u8>>>,
        next_due: Mutex<Instant>,
    }

    impl StreamingTransport {
        /// Quick enough for a test, apart enough to arrive one by one.
        const PERIOD: Duration = Duration::from_millis(50);

        fn new(frames: Vec<Vec<u8>>) -> Self {
            Self {
                frames: Mutex::new(frames.into()),
                next_due: Mutex::new(Instant::now()),
            }
        }
    }

    impl dmm_lib::transport::Transport for StreamingTransport {
        fn link(&self) -> Option<dmm_lib::transport::Link> {
            None
        }

        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            let mut next_due = self.next_due.lock().unwrap();
            let timeout = Duration::from_millis(timeout_ms.max(0) as u64);
            if let Some(wait) = next_due.checked_duration_since(Instant::now()) {
                if wait > timeout {
                    std::thread::sleep(timeout);
                    return Ok(0);
                }
                std::thread::sleep(wait);
            }
            let Some(next) = self.frames.lock().unwrap().pop_front() else {
                return Ok(0);
            };
            *next_due = Instant::now() + Self::PERIOD;
            let n = next.len().min(buf.len());
            buf[..n].copy_from_slice(&next[..n]);
            Ok(n)
        }
    }

    fn dmm_replaying(responses: Vec<Vec<u8>>) -> BoxedDmm {
        let transport = MockTransport::new(responses);
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        dmm_lib::Dmm::new(
            Box::new(transport) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap()
    }

    /// A UT804 sends nothing while HOLD is on (#16), and its step says that
    /// outcome is an answer. EXIT, which leaves HOLD, turns the meter's output
    /// off too, so every step that has the operator press it asks for SEND
    /// after it.
    #[test]
    fn the_ut804_hold_step_says_no_response_is_an_answer() {
        let device = dmm_lib::protocol::registry::find_device("ut804").unwrap();
        let steps = (device.new_protocol)().capture_steps();
        let hold = steps.iter().find(|s| s.id == "hold").unwrap();
        assert!(
            hold.instruction
                .contains("the tool reports no response within a minute: that is a valid result"),
            "{:?}",
            hold.instruction
        );
        for step in steps.iter().filter(|s| s.instruction.contains("EXIT")) {
            assert!(
                step.instruction.contains("EXIT, then SEND"),
                "{}: {:?}",
                step.id,
                step.instruction
            );
        }
    }

    /// Whether HOLD silences a UT71 or VC9x0 is open, so their HOLD step
    /// words the answer as the UT804's does.
    #[test]
    fn the_handheld_hold_steps_say_no_response_is_an_answer() {
        for id in ["ut71ab", "ut71cde", "vc920"] {
            let device = dmm_lib::protocol::registry::find_device(id).unwrap();
            let steps = (device.new_protocol)().capture_steps();
            let hold = steps.iter().find(|s| s.id == "hold").unwrap();
            assert!(
                hold.instruction.contains("the tool reports no response"),
                "{id}: {:?}",
                hold.instruction
            );
        }
    }

    /// The no-response line is printed in the run's language, so a step
    /// that expects it names the outcome instead of quoting the line, which
    /// would not read the same.
    #[test]
    fn no_step_quotes_the_no_response_line() {
        for device in dmm_lib::protocol::registry::DEVICES {
            for step in (device.new_protocol)().capture_steps() {
                assert!(
                    !step
                        .instruction
                        .contains(&*tr!(crate::l10n::english(), "capture-no-response")),
                    "{}/{}: {:?}",
                    device.id,
                    step.id,
                    step.instruction
                );
            }
        }
    }

    /// A rejected frame used to end the step, so the report carried neither a
    /// sample nor a reason. Keep polling and record why.
    #[test]
    fn capture_samples_reports_a_rejected_frame_and_keeps_polling() {
        let mut bad = measurement_frame();
        let last = bad.len() - 1;
        bad[last] = bad[last].wrapping_add(1);

        let mut dmm = dmm_replaying(vec![bad, measurement_frame()]);
        let mut errors = ErrorLog::default();
        let samples = capture_samples(&mut dmm, 1, &mut errors).unwrap();
        let diagnostics = errors.into_diagnostics();

        assert_eq!(samples.len(), 1, "polling must continue past the error");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("checksum"), "got {diagnostics:?}");
    }

    /// A meter stuck on bad frames would otherwise repeat one line per poll.
    #[test]
    fn repeated_errors_are_reported_once_with_a_count() {
        let mut bad = measurement_frame();
        let last = bad.len() - 1;
        bad[last] = bad[last].wrapping_add(1);

        let mut dmm = dmm_replaying(vec![bad.clone(), bad.clone(), bad]);
        let mut errors = ErrorLog::default();
        let samples = capture_samples(&mut dmm, 1, &mut errors).unwrap();
        let diagnostics = errors.into_diagnostics();

        assert!(samples.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].ends_with("(x3)"), "got {diagnostics:?}");
    }

    /// A cable pulled mid-run: every call fails at once, as an unplugged
    /// CP2110 does. The calls are counted from the moment it is pulled.
    struct Pulled {
        pulled: std::sync::Arc<std::sync::atomic::AtomicBool>,
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Pulled {
        fn fail(&self) -> dmm_lib::error::Result<usize> {
            if !self.pulled.load(std::sync::atomic::Ordering::SeqCst) {
                return Ok(0);
            }
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(Error::cable_pulled())
        }
    }

    impl dmm_lib::transport::Transport for Pulled {
        fn link(&self) -> Option<dmm_lib::transport::Link> {
            None
        }

        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            self.fail().map(|_| ())
        }

        fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            self.fail()
        }
    }

    /// Every sampling loop sorts a failed read through one place, so a new
    /// read site cannot forget the lost-link case.
    #[test]
    fn triage_skips_timeouts_ends_on_a_lost_link_and_records_the_rest() {
        let mut errors = ErrorLog::default();
        assert!(triage(Error::Timeout, &mut errors).is_ok());
        assert!(triage(Error::cable_pulled(), &mut errors).is_err_and(|e| link_gone(&e)));
        assert!(triage(Error::UnknownMode(0x7f), &mut errors).is_ok());
        assert_eq!(errors.into_diagnostics(), ["unknown mode: 0x7f"]);
    }

    /// A lost link used to be recorded as a parse rejection and polled
    /// again: an Enter-only wait spun on it at full CPU, and Enter filed an
    /// empty step. Both now end on the first failed call.
    #[test]
    fn a_lost_link_ends_the_wait_and_the_sampling_at_once() {
        use std::sync::atomic::Ordering;
        let pulled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        let mut dmm = dmm_lib::Dmm::new(
            Box::new(Pulled {
                pulled: pulled.clone(),
                calls: calls.clone(),
            }) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap();
        pulled.store(true, Ordering::SeqCst);

        let mut errors = ErrorLog::default();
        let mut watcher = StateWatcher::for_step(None, None, false);
        let watched = watch_for_state(
            &mut dmm,
            &Input::untouched(),
            &mut watcher,
            Duration::ZERO,
            OnTimeout::OfferKeys,
            &mut errors,
        );
        assert!(watched.is_err());
        assert_eq!(calls.swap(0, Ordering::SeqCst), 1);

        let sampled = capture_samples(&mut dmm, 5, &mut errors);
        assert!(sampled.is_err_and(|e| link_gone(&e)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(errors.into_diagnostics().is_empty(), "no parse rejection");
    }

    /// A lost link at the command's press used to file the step as an error,
    /// as though the meter had refused it; the run now ends there instead.
    #[test]
    fn a_lost_link_at_the_command_ends_the_run_without_filing_the_step() {
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        let dmm = dmm_lib::Dmm::new(
            // Pulled once the baseline is read: the command's write fails.
            Box::new(
                MockTransport::new(vec![measurement_frame(); STABLE_FRAMES])
                    .fail_after(STABLE_FRAMES, Error::cable_pulled),
            ) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap();
        let step = CaptureStep {
            id: "hold",
            instruction: "press HOLD",
            command: Some("hold"),
            samples: 2,
            expect: None,
            verified: true,
            gate: false,
            needs: &[],
            wait_for_enter: false,
            starts_from: None,
        };
        let mut run = run_on(dmm);
        let err = run
            .capture_step(&step, false, &PrevState::default())
            .err()
            .expect("the run ends on a lost link");
        let err = err.downcast_ref::<dmm_lib::error::Error>().unwrap();
        assert!(link_gone(err), "got {err:?}");
        assert!(run.report.steps.is_empty(), "no step filed");
    }

    /// A command the meter ignored has to say what it is still showing, or
    /// the report reads as though the step never ran.
    #[test]
    fn a_dead_command_names_what_the_meter_still_shows() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(
            did_nothing(crate::l10n::english(), "hold", Some(&m)),
            "hold did nothing; the meter still shows 5.678 V [AUTO]"
        );
        assert_eq!(
            did_nothing(crate::l10n::english(), "hold", None),
            "hold did nothing; the meter sent no reading"
        );
    }

    /// The mismatch line reads as the library words the mismatch in
    /// English, but for a flag, which goes by the label the meter's screen
    /// shows.
    #[test]
    fn a_mismatch_line_words_each_kind_of_mismatch() {
        use dmm_lib::flags::Flag;
        use dmm_lib::protocol::Expect;
        let l = crate::l10n::english();
        let dcv = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        let absent = Measurement {
            value: MeasuredValue::Absent,
            ..dcv.clone()
        };
        let hold: &[(Flag, bool)] = &[(Flag::Hold, true)];
        for (expect, m) in [
            (Expect::mode("AC V"), &dcv),
            (Expect::new().range(RangeExpect::Manual), &dcv),
            (Expect::new().value(ValueExpect::Negative), &dcv),
            (Expect::new().value(ValueExpect::NoReading), &absent),
            (Expect::new().at_least(20.0), &dcv),
            (Expect::new().at_least(1.5), &absent),
        ] {
            let mismatch = expect.check(m).unwrap_err();
            assert_eq!(
                mismatch_line(l, &mismatch),
                format!("meter shows: {mismatch}")
            );
        }
        let mismatch = Expect::new().flags(hold).check(&dcv).unwrap_err();
        assert_eq!(
            mismatch_line(l, &mismatch),
            "meter shows: HOLD is off, want on"
        );
    }

    /// Only the events tagged with the step, and no more than the cap — the
    /// last of them: a step that waited for the meter before sampling would
    /// otherwise fill the report with the wait and drop the samples.
    #[test]
    fn step_frames_are_filtered_and_capped_to_the_newest() {
        let event = |at_ms: u64, step: Option<&str>| WireEvent {
            at_ms,
            dir: crate::capture::recording::Direction::Rx,
            step: step.map(str::to_string),
            bytes: vec![0xAB, 0xCD],
            baud: None,
        };
        let mut events: Vec<WireEvent> = (0..MAX_FRAMES_PER_STEP as u64 + 3)
            .map(|at_ms| event(at_ms, Some("dcv")))
            .collect();
        events.push(event(0, Some("acv")));
        events.push(event(0, None));

        let (frames, dropped) = frames_for_step(&events, "dcv");
        assert_eq!(frames.len(), MAX_FRAMES_PER_STEP);
        assert_eq!(
            frames[0].at_ms, 3,
            "the three oldest events are the dropped ones"
        );
        assert_eq!(frames[0].hex, "AB CD");
        assert_eq!(dropped, 3);
    }

    /// A step that waits on the operator — capacitance, NCV — passes the cap
    /// as a matter of course. The trim used to arrive as a diagnostic, and
    /// every diagnostic flags the step, so both were marked for a maintainer
    /// with nothing wrong in them.
    #[test]
    fn passing_the_frame_cap_does_not_flag_the_step() {
        let events: Vec<WireEvent> = (0..MAX_FRAMES_PER_STEP as u64 + 450)
            .map(|at_ms| WireEvent {
                at_ms,
                dir: crate::capture::recording::Direction::Rx,
                step: Some("ncv".to_string()),
                bytes: vec![0xAB, 0xCD],
                baud: None,
            })
            .collect();

        let (frames, dropped) = frames_for_step(&events, "ncv");
        assert_eq!(frames.len(), MAX_FRAMES_PER_STEP);
        assert_eq!(dropped, 450);

        let samples = vec![SampleData::from_measurement(&make_test_measurement(
            0x14,
            0x00,
            b"      3",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ))];
        assert!(!needs_attention(&samples, samples.len(), &[]));
    }

    /// The prompt takes its own answers and nothing else. It used to take a
    /// correction on the same line, so a reporter's "correct" was filed as
    /// what the meter showed — failing the gate step it had just confirmed.
    #[test]
    fn the_confirmation_prompt_takes_only_its_own_answers() {
        let offered = Retake::Offered;
        assert_eq!(Confirmation::of("", offered), Some(Confirmation::Yes));
        for no in ["n", "N", "no", "No"] {
            assert_eq!(
                Confirmation::of(no, offered),
                Some(Confirmation::No),
                "{no:?}"
            );
        }
        for retake in ["r", "R", "retake", "Retake"] {
            assert_eq!(
                Confirmation::of(retake, offered),
                Some(Confirmation::Retake),
                "{retake:?}"
            );
        }
        for typed in ["correct", "yes", "y", "5.68 V", "rel", "R 0.5", "OL"] {
            assert_eq!(Confirmation::of(typed, offered), None, "{typed:?}");
        }
        // Where nothing can be redone, `r` is not one of the answers either.
        let not_offered = Retake::NotOffered;
        assert_eq!(Confirmation::of("r", not_offered), None);
        assert_eq!(Confirmation::of("", not_offered), Some(Confirmation::Yes));
        assert_eq!(Confirmation::of("n", not_offered), Some(Confirmation::No));
    }

    /// Duplicate IDs would make resume and `--steps` ambiguous, and a step
    /// could overwrite another's samples through `upsert_step`.
    #[test]
    fn every_device_has_unique_capture_step_ids() {
        for device in dmm_lib::protocol::registry::DEVICES {
            let steps = (device.new_protocol)().capture_steps();
            let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
            let len_before = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), len_before, "duplicate step ID in {}", device.id);
        }
    }

    /// Families whose gate is still split across their step list.
    ///
    /// `Trust::drives()` only turns on once every gate step has reported, and
    /// a gate step is never swept itself, so a step scheduled among them is
    /// one whose ranges and flags nobody walks. The UT61+ list lost the AC V,
    /// DC mV and AC mV ladders that way (issue #19) and has been reordered.
    ///
    /// The ones below still are, for two different reasons:
    /// - `vc880`, `vc650bt`, `vc890` declare `choices`, so they do
    ///   lose coverage. Each fix needs that family's own dial order and a
    ///   hardware run — tracked in `docs/verification-backlog.md`, "Capture
    ///   tool".
    /// - `ut8802`, `ut8803`, `ut803`, `ut804`, `ut71ab`, `ut71cde`, `vc920`,
    ///   `ut171` declare no `choices`, so nothing is swept whatever the order
    ///   and they cost nothing.
    const SPLIT_GATE: &[&str] = &[
        "ut8802", "ut8803", "ut803", "ut804", "ut71ab", "ut71cde", "vc920", "ut171", "vc880",
        "vc650bt", "vc890",
    ];

    #[test]
    fn every_device_finishes_its_gate_before_any_other_step() {
        for device in dmm_lib::protocol::registry::DEVICES {
            if SPLIT_GATE.contains(&device.id) {
                continue;
            }
            let steps = (device.new_protocol)().capture_steps();
            let Some(last_gate) = steps.iter().rposition(|s| s.gate) else {
                continue;
            };
            let stragglers: Vec<&str> = steps[..last_gate]
                .iter()
                .filter(|s| !s.gate)
                .map(|s| s.id)
                .collect();
            assert!(
                stragglers.is_empty(),
                "{}: {stragglers:?} run before the gate closes and can never be swept",
                device.id
            );
        }
    }

    /// `extra` is the freeform pass, not a protocol step. A device declaring
    /// it would make `--steps extra` ambiguous.
    #[test]
    fn no_device_claims_the_freeform_step_id() {
        for device in dmm_lib::protocol::registry::DEVICES {
            let steps = (device.new_protocol)().capture_steps();
            assert!(
                !steps.iter().any(|s| s.id == FREEFORM_STEP_ID),
                "{} declares a step named {FREEFORM_STEP_ID}",
                device.id
            );
        }
    }

    /// One RANGE press, then AUTO to restore. Not a sweep: repeated presses
    /// were tried against hardware and neither stepped the range table nor
    /// returned the meter to auto — see the Command::Range doc comment.
    #[test]
    fn ut61eplus_sends_range_once_and_restores_auto() {
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        let steps = (device.new_protocol)().capture_steps();
        let range_steps: Vec<&str> = steps
            .iter()
            .filter(|s| s.command == Some("range"))
            .map(|s| s.id)
            .collect();
        assert_eq!(
            range_steps,
            vec!["range"],
            "repeated RANGE presses produce misleading data until 0x46 is understood"
        );
        assert!(
            steps.iter().all(|s| s.id != "range_cycle"),
            "the CLI-side range_cycle step must be gone"
        );
        // RANGE engages manual ranging, so the wizard must hand the meter
        // back in auto or it leaves it stuck.
        let auto_pos = steps.iter().position(|s| s.command == Some("auto"));
        let range_pos = steps.iter().position(|s| s.command == Some("range"));
        assert!(
            matches!((range_pos, auto_pos), (Some(r), Some(a)) if a > r),
            "AUTO must come after RANGE"
        );
    }

    /// `--settle` covers a step the operator paced as much as a driven
    /// sub-step: whatever ended the wait, the reading behind it may still be
    /// moving. The frame the watcher accepted was read before the delay, so a
    /// settling run files only what it read after it.
    #[test]
    fn a_settling_step_files_only_what_it_read_after_the_wait() {
        use crate::capture::drive::Driver;

        // Same reading with HOLD on, as the meter sends it (flag nibbles
        // "201"): a state the baseline has not seen, so the watcher takes it.
        let held = |display: &[u8]| {
            let mut payload = vec![0x02, 0x30];
            payload.extend_from_slice(display);
            payload.extend_from_slice(&[0x00, 0x00, b'2', b'0', b'1']);
            frame(&payload)
        };
        let step = CaptureStep {
            id: "hold",
            instruction: "press HOLD",
            command: Some("hold"),
            samples: 2,
            expect: None,
            verified: true,
            gate: false,
            needs: &[],
            wait_for_enter: false,
            starts_from: None,
        };

        let filed = |settle: Duration| {
            let mut responses = vec![measurement_frame(); 3]; // the baseline
            responses.push(frame(&[0xFF, 0x00])); // the press's ack
            responses.extend(vec![held(b"  5.678"); 3]); // ends the watcher's wait
            responses.extend(vec![held(b"  1.234"); 2]); // what it settles to
            let mut run = run_on(dmm_replaying(responses));
            run.driver = Driver::new(false).settling(settle);
            run.capture_step(&step, false, &PrevState::default())
                .unwrap();
            let displays: Vec<String> = run.report.steps[0]
                .samples
                .iter()
                .map(|s| s.display_raw.clone())
                .collect();
            displays
        };

        // Without it the accepted frame leads, transient and all.
        let straight_off = filed(Duration::ZERO);
        assert_eq!(
            straight_off.first().map(String::as_str),
            Some("  5.678"),
            "got {straight_off:?}"
        );

        let settled = filed(Duration::from_millis(50));
        assert!(
            settled.iter().all(|d| d == "  1.234"),
            "the frame that ended the wait must not be filed: {settled:?}"
        );
        assert_eq!(settled.len(), step.samples, "got {settled:?}");
    }

    /// A UT181A frame around `payload`: 2-byte LE length, LE checksum.
    fn ut181a_frame(payload: &str) -> Vec<u8> {
        let payload: Vec<u8> = payload
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();
        let len = (payload.len() + 2) as u16;
        let mut f = vec![0xAB, 0xCD];
        f.extend_from_slice(&len.to_le_bytes());
        f.extend_from_slice(&payload);
        let sum = f[2..]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        f.extend_from_slice(&sum.to_le_bytes());
        f
    }

    /// A UT181A has no reading for about half a second after a change
    /// (issue #5, 2026-09-27). A step's samples start once its display is
    /// back, unless no reading is what the step expects.
    #[test]
    fn a_step_samples_once_the_display_is_back() {
        // Hz blank just after a switch, then 50.05 Hz — both from the capture.
        let blank = ut181a_frame(
            "02 08 00 11 71 01 00 00 00 00 33 48 7A 00 00 10 00 00 00 00 00 00 00 \
             48 7A 00 00 00 00 00 00",
        );
        let hz = ut181a_frame(
            "02 08 01 11 71 01 37 34 48 42 30 48 7A 00 00 20 00 00 00 37 34 48 42 \
             48 7A 00 00 00 00 CD CC",
        );
        let filed = |value: Option<dmm_lib::protocol::ValueExpect>| {
            let mut responses = vec![blank.clone(); 3];
            responses.extend(vec![hz.clone(); 5]);
            let device = dmm_lib::protocol::registry::find_device("ut181a").unwrap();
            let dmm = dmm_lib::Dmm::new(
                Box::new(StreamingTransport::new(responses))
                    as Box<dyn dmm_lib::transport::Transport>,
                (device.new_protocol)(),
            )
            .unwrap();
            let step = CaptureStep {
                samples: 3,
                expect: Some(dmm_lib::protocol::Expect {
                    value,
                    ..dmm_lib::protocol::Expect::mode("Hz")
                }),
                ..cli_step("hz", false, false)
            };
            let mut run = run_on(dmm);
            run.capture_step(&step, false, &PrevState::default())
                .unwrap();
            run.report.steps[0]
                .samples
                .iter()
                .map(|s| s.value.clone())
                .collect::<Vec<_>>()
        };

        let values = filed(None);
        assert_eq!(values.len(), 3);
        assert!(values.iter().all(|v| v != "----"), "got {values:?}");
        let values = filed(Some(dmm_lib::protocol::ValueExpect::NoReading));
        assert_eq!(values.first().map(String::as_str), Some("----"));
    }

    /// A UT61E+ frame in `mode` (0x07 continuity, 0x08 diode) reading 0.1.
    fn mode_frame(mode: u8) -> Vec<u8> {
        frame(&[
            mode, 0x30, b' ', b' ', b' ', b' ', b'0', b'.', b'1', 0, 0, 0x30, 0x30, 0x30,
        ])
    }

    /// Issue #16: the operator pressed SELECT on from Continuity to Diode
    /// before the samples were in, and the step filed both modes. A step
    /// whose samples leave its mode is retaken and files only its own.
    #[test]
    fn a_step_the_meter_left_while_sampling_is_retaken() {
        use crate::capture::input::Input;

        let (cont, diode) = (mode_frame(0x07), mode_frame(0x08));
        // The wait settles on three frames, then the samples: away, and back.
        let mut responses = vec![cont.clone(); 4];
        responses.extend([diode.clone(), diode, cont.clone()]);
        // The retake's wait and samples.
        responses.extend(vec![cont; 3 + 4]);
        let step = CaptureStep {
            expect: Some(dmm_lib::protocol::Expect::mode("Continuity")),
            ..cli_step("cont", false, false)
        };
        // In DC V before, so the step waits for Continuity on its own.
        let prev = PrevState {
            baseline: None,
            last: Some(make_test_measurement(
                0x02,
                0x01,
                b"  5.678",
                (0x00, 0x00),
                (0x00, 0x00, 0x00),
            )),
        };
        let mut run = CaptureRun {
            input: Input::untouched(),
            ..run_on(dmm_replaying(responses))
        };
        run.capture_step(&step, true, &prev).unwrap();

        let modes: Vec<&str> = run.report.steps[0]
            .samples
            .iter()
            .map(|s| s.mode.as_str())
            .collect();
        assert_eq!(modes, ["Continuity"; 5]);
    }

    /// Issue #16: the UT804's REL step followed MAX MIN, whose instruction
    /// ends "Press EXIT, then SEND, afterwards". The state those presses left
    /// differed from MAX MIN's, so the step captured it before REL was
    /// pressed. A step that waits for Enter files what the meter shows then.
    #[test]
    fn a_step_that_waits_for_enter_ignores_the_states_before_it() {
        /// Presses Enter as it hands out response `at`: the operator reads
        /// the new state off the LCD, then presses the key.
        struct PressesEnter {
            queue: MockTransport,
            reads: Mutex<usize>,
            at: usize,
            key: Mutex<std::sync::mpsc::Sender<Key>>,
        }

        impl dmm_lib::transport::Transport for PressesEnter {
            fn link(&self) -> Option<dmm_lib::transport::Link> {
                self.queue.link()
            }

            fn write(&self, data: &[u8]) -> dmm_lib::error::Result<()> {
                self.queue.write(data)
            }

            fn read_timeout(
                &self,
                buf: &mut [u8],
                timeout_ms: i32,
            ) -> dmm_lib::error::Result<usize> {
                let mut reads = self.reads.lock().unwrap();
                if *reads == self.at {
                    self.key.lock().unwrap().send(Key::Enter).unwrap();
                }
                *reads += 1;
                self.queue.read_timeout(buf, timeout_ms)
            }

            fn set_baud(&self, baud: u32) -> dmm_lib::error::Result<()> {
                self.queue.set_baud(baud)
            }
        }

        // DC V at one reading; only the flag bytes tell the states apart.
        let flagged = |flag1: u8, flag2: u8| {
            frame(&[
                0x02, 0x30, b' ', b' ', b'5', b'.', b'2', b'0', b'9', 0, 0, flag1, flag2, 0x30,
            ])
        };
        let max = flagged(0x38, 0x34); // MAX on a manual range
        let auto = flagged(0x30, 0x30); // after EXIT and SEND
        let rel = flagged(0x31, 0x30);

        let mut before = dmm_replaying(vec![max; 3]);
        let payloads = capture_samples(&mut before, 3, &mut ErrorLog::default()).unwrap();
        let prev = PrevState {
            baseline: Baseline::from_payloads(payloads.iter().map(|m| m.raw_payload.as_slice())),
            last: None,
        };

        // Four frames of the clean-up's state: enough to settle on.
        let mut responses = vec![auto; 4];
        responses.extend(vec![rel; 6]);
        let (key, input) = Input::typed();
        let transport = PressesEnter {
            queue: MockTransport::new(responses),
            reads: Mutex::new(0),
            at: 4,
            key: Mutex::new(key),
        };
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        let dmm = dmm_lib::Dmm::new(
            Box::new(transport) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap();
        let step = CaptureStep {
            wait_for_enter: true,
            ..cli_step("rel", false, false)
        };
        let mut run = CaptureRun {
            input,
            ..run_on(dmm)
        };
        run.capture_step(&step, true, &prev).unwrap();

        let rel: Vec<bool> = run.report.steps[0]
            .samples
            .iter()
            .map(|s| s.flags.rel)
            .collect();
        assert_eq!(rel, [true; 5]);
    }

    /// A meter on the bench for the starting-state tests: it answers a command
    /// with an ack and then its `after` frames, as sending drops whatever was
    /// already waiting. It logs how many frames had gone out at each command,
    /// and has keys typed as a given frame goes out.
    struct Bench {
        queue: MockTransport,
        after: Mutex<Vec<Vec<u8>>>,
        request: Mutex<Option<Vec<u8>>>,
        frames: Arc<Mutex<usize>>,
        commands: Arc<Mutex<Vec<usize>>>,
        keys: Vec<(usize, Key)>,
        typed: Mutex<std::sync::mpsc::Sender<Key>>,
    }

    impl dmm_lib::transport::Transport for Bench {
        fn link(&self) -> Option<dmm_lib::transport::Link> {
            self.queue.link()
        }

        fn write(&self, data: &[u8]) -> dmm_lib::error::Result<()> {
            // The first write of each test is a reading request.
            let mut request = self.request.lock().unwrap();
            if request.get_or_insert_with(|| data.to_vec()) != data {
                self.commands
                    .lock()
                    .unwrap()
                    .push(*self.frames.lock().unwrap());
                self.queue.push_response(frame(&[0xFF, 0x00])); // the ack
                for f in self.after.lock().unwrap().drain(..) {
                    self.queue.push_response(f);
                }
            }
            self.queue.write(data)
        }

        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            let mut frames = self.frames.lock().unwrap();
            for (at, key) in &self.keys {
                if at == &*frames {
                    self.typed.lock().unwrap().send(key.clone()).unwrap();
                }
            }
            let n = self.queue.read_timeout(buf, timeout_ms)?;
            if n > 0 {
                *frames += 1;
            }
            Ok(n)
        }

        fn set_baud(&self, baud: u32) -> dmm_lib::error::Result<()> {
            self.queue.set_baud(baud)
        }
    }

    /// A step's run on a [`Bench`], with its frames recorded.
    struct BenchRun {
        run: CaptureRun,
        frames: Arc<Mutex<usize>>,
        commands: Arc<Mutex<Vec<usize>>>,
    }

    impl BenchRun {
        fn new(before: Vec<Vec<u8>>, after: Vec<Vec<u8>>, keys: Vec<(usize, Key)>) -> Self {
            let (typed, input) = Input::typed();
            let frames = Arc::new(Mutex::new(0));
            let commands = Arc::new(Mutex::new(Vec::new()));
            let bench = Bench {
                queue: MockTransport::new(before),
                after: Mutex::new(after),
                request: Mutex::new(None),
                frames: Arc::clone(&frames),
                commands: Arc::clone(&commands),
                keys,
                typed: Mutex::new(typed),
            };
            let (recording, recorder) =
                crate::capture::recording::RecordingTransport::new(Box::new(bench));
            let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
            let dmm = dmm_lib::Dmm::new(
                Box::new(recording) as Box<dyn dmm_lib::transport::Transport>,
                (device.new_protocol)(),
            )
            .unwrap();
            let run = CaptureRun {
                input,
                ..CaptureRun::on(dmm, recorder)
            };
            Self {
                run,
                frames,
                commands,
            }
        }

        fn capture(&mut self, step: &CaptureStep) -> StepOutcome {
            self.run
                .capture_step(step, true, &PrevState::default())
                .unwrap()
        }

        /// How many frames had gone out at each command sent.
        fn commands(&self) -> Vec<usize> {
            self.commands.lock().unwrap().clone()
        }
    }

    /// `n` readings in Ω, away from the DC V the steps below start from.
    fn ohms(n: usize) -> Vec<Vec<u8>> {
        let f = frame(&[
            0x06, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, 0x30, 0x30, 0x30,
        ]);
        vec![f; n]
    }

    /// `n` readings in Ω with REL on, which the REL steps below expect.
    fn ohms_rel(n: usize) -> Vec<Vec<u8>> {
        let f = frame(&[
            0x06, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, 0x31, 0x30, 0x30,
        ]);
        vec![f; n]
    }

    /// `n` readings in DC V with REL `on`.
    fn volts(n: usize, on: bool) -> Vec<Vec<u8>> {
        let rel = if on { 0x31 } else { 0x30 };
        let f = frame(&[
            0x02, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, rel, 0x30, 0x30,
        ]);
        vec![f; n]
    }

    /// A REL step that starts from DC V, as a CMS key step does.
    fn rel_from_dc_v() -> CaptureStep {
        use dmm_lib::flags::Flag;
        CaptureStep::with_command("rel", "set up DC V: we will send REL", "rel", 3)
            .starts_from(dmm_lib::protocol::Expect::mode("DC V"))
            .expect(dmm_lib::protocol::Expect::new().flags(&[(Flag::Rel, true)]))
    }

    /// #45: the keys went out while the meter sat in the scope, before the
    /// operator could set it up. Nothing is sent until the state has held.
    #[test]
    fn a_command_waits_for_the_state_it_starts_from() {
        // 5 elsewhere, 3 to settle, 3 for the frames before the command.
        let before = [ohms(5), volts(6, false)].concat();
        let mut bench = BenchRun::new(before, volts(12, true), Vec::new());
        bench.capture(&rel_from_dc_v());
        assert_eq!(bench.commands(), [11]);
        let step = &bench.run.report.steps[0];
        assert_eq!(step.status, StepStatus::Captured);
        assert!(step.samples.iter().all(|s| s.flags.rel));
    }

    /// The fast path: a meter already there costs only the frames that show it.
    #[test]
    fn a_command_goes_out_at_once_from_its_starting_state() {
        let mut bench = BenchRun::new(volts(6, false), volts(12, true), Vec::new());
        bench.capture(&rel_from_dc_v());
        assert_eq!(bench.commands(), [6]);
    }

    /// Enter sends though the state never came, as our reading of it may be
    /// what is wrong; a second Enter must not end the wait for the command's
    /// effect too, or the frames from before it took hold are filed.
    #[test]
    fn enter_sends_anyway_and_a_second_one_is_dropped() {
        // Enter on the third frame, then 3 frames before the command, with
        // another Enter among them.
        let keys = vec![(2, Key::Enter), (4, Key::Enter)];
        // 2 frames before the command takes hold.
        let after = [ohms(2), ohms_rel(12)].concat();
        let mut bench = BenchRun::new(ohms(6), after, keys);
        bench.capture(&rel_from_dc_v());
        assert_eq!(bench.commands(), [6]);
        let step = &bench.run.report.steps[0];
        assert_eq!(step.status, StepStatus::Captured);
        assert!(step.samples.iter().all(|s| s.flags.rel));
    }

    /// Giving up on a state that never came keeps the frames, which show what
    /// the meter did instead, and nothing is sent.
    #[test]
    fn skipping_the_wait_files_its_frames_and_sends_nothing() {
        for (key, quits) in [('s', false), ('q', true)] {
            let keys = vec![(3, Key::Char(key))];
            let mut bench = BenchRun::new(ohms(8), Vec::new(), keys);
            let outcome = bench.capture(&rel_from_dc_v());
            assert_eq!(outcome.quit, quits);
            assert!(bench.commands().is_empty());
            let step = &bench.run.report.steps[0];
            assert_eq!(step.status, StepStatus::Skipped);
            assert!(!step.frames.is_empty(), "{key}: its frames were dropped");
        }
    }

    /// A piped run has no one to wait for, and a step that declares no state
    /// is set up by the one before: both send after the frames before it.
    #[test]
    fn with_no_one_to_wait_for_or_no_state_the_command_goes_out_at_once() {
        let mut piped = BenchRun::new(ohms(3), ohms_rel(12), Vec::new());
        piped.run.input = Input::piped();
        piped.capture(&rel_from_dc_v());
        assert_eq!(piped.commands(), [3]);

        let mut no_state = BenchRun::new(ohms(3), ohms_rel(12), Vec::new());
        no_state.capture(&CaptureStep {
            starts_from: None,
            ..rel_from_dc_v()
        });
        assert_eq!(no_state.commands(), [3]);
    }

    /// Where the run doesn't read the parse, the state can't be seen: a meter
    /// already showing it still waits for Enter.
    #[test]
    fn a_run_that_cannot_read_the_state_waits_for_enter() {
        let mut bench = BenchRun::new(volts(8, false), Vec::new(), vec![(5, Key::Enter)]);
        bench.run.trust = crate::capture::report::Trust::new(true, false, &[]);
        let state = dmm_lib::protocol::Expect::mode("DC V");
        let watched = bench
            .run
            .wait_for_start(state, &mut ErrorLog::default())
            .unwrap();
        assert!(matches!(watched, Watched::Ready(None)));
        assert_eq!(*bench.frames.lock().unwrap(), 6);
    }

    /// Only the mode is compared, and only once the first sample is in it: a
    /// wandering value is not a retake, and neither is an Enter capture that
    /// was never in the step's mode.
    #[test]
    fn only_samples_that_leave_the_steps_mode_are_retaken() {
        let reading = |mode: u8, digits: &[u8; 7]| {
            make_test_measurement(mode, 0x01, digits, (0x00, 0x00), (0x00, 0x00, 0x00))
        };
        let ohm = |digits: &[u8; 7]| reading(0x06, digits);
        let want = dmm_lib::protocol::Expect::mode("Ω");

        let body = [ohm(b" 1.2345"), ohm(b" 0.9876"), ohm(b" 1.5000")];
        assert_eq!(left_mode(Some(&want), &body), None);

        let entered_elsewhere = [reading(0x07, b"   0.1 "), ohm(b" 1.2345")];
        assert_eq!(left_mode(Some(&want), &entered_elsewhere), None);

        let left = [ohm(b" 1.2345"), reading(0x07, b"   0.1 "), ohm(b" 1.2345")];
        assert_eq!(left_mode(Some(&want), &left), Some(("Ω", "Continuity")));
        // A step that names no mode has nothing to leave.
        assert_eq!(
            left_mode(Some(&dmm_lib::protocol::Expect::new()), &left),
            None
        );
        assert_eq!(left_mode(None, &left), None);
    }
}
