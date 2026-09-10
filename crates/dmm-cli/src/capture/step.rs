//! One capture step: the definition the run walks, the wait for the state it
//! asks for, and the frames that wait puts on the wire.

use super::input::{ErrorLog, Input};
use super::report::{
    CaptureReport, FrameRecord, SampleData, StepResult, StepStatus, Trust, already_captured,
    needs_attention, upsert_step,
};
use crate::recording::{self, SharedRecorder, WireEvent};
use crate::watch::{Baseline, STABLE_FRAMES, StateWatcher, Verdict, enter_only};
use console::{Key, style};
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::Need;
use std::time::{Duration, Instant};

/// Cap on wire events recorded per step, so one chatty step can't grow the
/// report without bound. Overflow is reported in the step's diagnostics.
pub(crate) const MAX_FRAMES_PER_STEP: usize = 500;

/// Filter keyword for the freeform capture pass. Not a protocol step — the
/// pass generates `extra_0`, `extra_1`, … as the user describes each capture.
pub(crate) const FREEFORM_STEP_ID: &str = "extra";

#[derive(Clone, Copy)]
pub(crate) struct CaptureStep {
    pub id: &'static str,
    pub instruction: &'static str,
    pub command: Option<&'static str>,
    pub samples: usize,
    /// What a correct reading looks like once the instruction is carried out;
    /// `None` leaves the step watching for any new state.
    pub expect: Option<dmm_lib::protocol::Expect>,
    /// Already confirmed on real hardware — `--unverified` skips these.
    pub verified: bool,
    /// One of the steps the family's core semantics rest on; flagged in the
    /// run so an operator knows which ones must not be skipped.
    pub gate: bool,
    /// Equipment the instruction asks for, listed before the run so a step
    /// nobody can do is dropped rather than met halfway through.
    pub needs: &'static [Need],
}

impl From<&dmm_lib::protocol::CaptureStep> for CaptureStep {
    fn from(ps: &dmm_lib::protocol::CaptureStep) -> Self {
        CaptureStep {
            id: ps.id,
            instruction: ps.instruction,
            command: ps.command,
            samples: ps.samples,
            expect: ps.expect,
            verified: ps.verified,
            gate: ps.gate,
            needs: ps.needs,
        }
    }
}

impl CaptureStep {
    /// The line announcing the step. Gate steps say so: skipping one leaves
    /// the rest of the run uninterpretable.
    fn header(&self) -> String {
        let id = style(format!("[{}]", self.id)).cyan().bold();
        let gate = if self.gate {
            format!(" {}", style("(gate)").dim())
        } else {
            String::new()
        };
        format!("{id} {}{gate}", self.instruction)
    }

    /// Create a StepResult with no samples or screen capture.
    pub(super) fn empty_result(&self, status: StepStatus, error: Option<String>) -> StepResult {
        StepResult {
            error,
            ..StepResult::new(self.id, self.instruction, status)
        }
    }
}

/// Poll for `n` samples.
///
/// A parse rejection is recorded and polling continues: a meter whose frames
/// we misread would otherwise end the step on the first bad frame, and the
/// report showed neither a sample nor a reason.
pub(crate) fn capture_samples(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    n: usize,
    errors: &mut ErrorLog,
) -> Vec<Measurement> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => errors.record(&e),
        }
        attempts += 1;
    }
    samples
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
    fn nothing(quit: bool) -> Self {
        StepOutcome {
            quit,
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

/// Read until the meter shows what the step asked for.
///
/// `hint` offers the keyboard once the timeout passes rather than giving up:
/// a step whose instruction takes a while to carry out must not file whatever
/// the meter happened to be showing.
fn watch_for_state(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    input: &Input,
    watcher: &mut StateWatcher,
    timeout: Duration,
    hint: bool,
    errors: &mut ErrorLog,
) -> Result<Watched, Box<dyn std::error::Error>> {
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
                    eprintln!("  {}", style(format!("meter shows: {reason}")).dim());
                }
                Verdict::Waiting => last = Some(m),
            },
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => {
                errors.record(&e);
                watcher.feed_error();
            }
        }

        // `checked_duration_since`: a backward clock jump must not strand the
        // step in a wait that never times out.
        let waited = Instant::now()
            .checked_duration_since(start)
            .unwrap_or_default();
        if waited >= timeout {
            if !hint {
                return Ok(Watched::TimedOut(last));
            }
            if !hinted {
                hinted = true;
                eprintln!(
                    "  {}",
                    style("Press Enter when the meter is ready (s=skip, q=finish)").dim()
                );
            }
        }
    }
}

/// The wording for a command the meter ignored, in the terms `cycle.rs` uses
/// for the same failure.
fn did_nothing(command: &str, last: Option<&Measurement>) -> String {
    match last {
        Some(m) => format!(
            "{command} did nothing; the meter still shows {}",
            SampleData::from_measurement(m).summary()
        ),
        None => format!("{command} did nothing; the meter sent no reading"),
    }
}

/// What the operator typed at a step's inline confirmation prompt.
#[derive(Debug, PartialEq, Eq)]
enum Confirmation {
    /// Empty means the reading matched; anything else is what the LCD showed.
    Answer(String),
    /// Do the step over: the attempt's samples are dropped.
    Retake,
}

impl Confirmation {
    /// `r` on its own is the retake key — no meter shows a bare "r", and a
    /// typed correction that starts with one still reads as a correction.
    fn of(answer: String) -> Self {
        if answer.eq_ignore_ascii_case("r") {
            Confirmation::Retake
        } else {
            Confirmation::Answer(answer)
        }
    }
}

/// File a step that ended before it sampled: the frames it did put on the
/// wire are drained out of the recorder and into its result.
fn finish_failed_step(
    recorder: &SharedRecorder,
    report: &mut CaptureReport,
    step_id: &str,
    mut result: StepResult,
) {
    let mut rec = recording::lock(recorder);
    rec.set_step(None);
    (result.frames, result.frames_dropped) = frames_for_step(&rec.drain(), step_id);
    result.needs_attention = true;
    upsert_step(report, result);
}

/// File the step as skipped, and say whether the run stops here.
fn skipped(report: &mut CaptureReport, step: &CaptureStep, quit: bool) -> StepOutcome {
    upsert_step(report, step.empty_result(StepStatus::Skipped, None));
    StepOutcome::nothing(quit)
}

/// Run one capture step. Returns Ok(true) if user wants to quit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_capture_step(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step: &CaptureStep,
    report: &mut CaptureReport,
    interactive: bool,
    input: &Input,
    prev: &PrevState,
    trust: &Trust,
    driver: &mut crate::drive::Driver,
) -> Result<StepOutcome, Box<dyn std::error::Error>> {
    // Check if already captured (resume)
    if already_captured(report, step.id) {
        eprintln!("  {} already captured, skipping", style(step.id).dim());
        return Ok(StepOutcome::nothing(false));
    }

    if interactive {
        eprintln!();
    }
    eprintln!("{}", step.header());

    recording::lock(recorder).set_step(Some(step.id));
    let mut errors = ErrorLog::default();
    let expect = trust.expect(step);

    // A mode a button on the meter reaches — continuity from Ω, duty
    // from Hz — is the tool's to switch to; only the dial is the operator's.
    if interactive && trust.drives() {
        crate::drive::switch_mode_from(dmm, step, prev.last.as_ref(), driver)?;
    }

    // One pass per attempt: `r` at the confirmation prompt drops the samples
    // and runs the same wait again, on the same previous state. The recorder
    // is not drained in between, so every attempt's frames reach the report.
    let mut attempt = 0usize;
    let (sample_data, mut measurements, confirmation) = loop {
        attempt += 1;
        // Anything typed before the step was announced answered the last
        // prompt, not this step's wait.
        input.drain_keys();
        // The frame the watcher accepted, kept as the step's first sample: it
        // is the one reading known to be in the state the step asked for.
        // `--settle` drops it below — there it is the transient.
        let mut settled: Option<Measurement> = None;

        // A retake re-samples; it does not press the button again, which on a
        // toggle like HOLD would undo the state the first press reached.
        if let Some(cmd) = step.command.filter(|_| attempt == 1) {
            // What the meter shows before the button is pressed, so a command
            // that changes nothing can be told from one that works.
            let before = capture_samples(dmm, STABLE_FRAMES, &mut errors);
            let before = Baseline::from_payloads(before.iter().map(|m| m.raw_payload.as_slice()));

            if let Err(e) = dmm.send_command(cmd) {
                eprintln!("  {}", style(format!("Command failed: {e}")).red());
                let result = step.empty_result(StepStatus::Error, Some(e.to_string()));
                finish_failed_step(recorder, report, step.id, result);
                return Ok(StepOutcome::nothing(false));
            }

            let mut watcher = StateWatcher::for_step(expect, before.as_ref(), true);
            match watch_for_state(
                dmm,
                input,
                &mut watcher,
                COMMAND_TIMEOUT,
                false,
                &mut errors,
            )? {
                Watched::Ready(m) => {
                    driver.prove_command(cmd);
                    settled = m;
                }
                Watched::TimedOut(last) => {
                    // No samples: filing pre-command frames as the step's result
                    // is what made a dead command look like a captured state.
                    let error = did_nothing(cmd, last.as_ref());
                    eprintln!("  {}", style(&error).yellow());
                    let mut result = step.empty_result(StepStatus::Error, Some(error));
                    result.diagnostics = errors.into_diagnostics();
                    finish_failed_step(recorder, report, step.id, result);
                    return Ok(StepOutcome::nothing(false));
                }
                Watched::Skip => return Ok(skipped(report, step, false)),
                Watched::Quit => return Ok(skipped(report, step, true)),
            }
        } else if interactive && input.is_tty() {
            // A step whose expectation the previous reading already satisfies
            // cannot be seen arriving — DC V with the leads open and shorted
            // both read about zero — so it is Enter-only and the keyboard is
            // offered at once. A mode the tool just switched to is a change, so
            // this is false there.
            let ask = enter_only(expect, prev.last.as_ref());
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
            match watch_for_state(dmm, input, &mut watcher, timeout, true, &mut errors)? {
                Watched::Ready(m) => settled = m,
                // `hint` keeps the wait open, so the timeout never ends it.
                Watched::TimedOut(_) => {}
                Watched::Skip => return Ok(skipped(report, step, false)),
                Watched::Quit => return Ok(skipped(report, step, true)),
            }
        } else if interactive {
            // No terminal to poll: ask, the way this step always did.
            let ch = input.key(&format!(
                "  {} ",
                style("any key=capture, s=skip, q=finish:").dim()
            ))?;
            if ch == 'q' || ch == 'Q' {
                return Ok(skipped(report, step, true));
            }
            if ch == 's' || ch == 'S' {
                return Ok(skipped(report, step, false));
            }
        }

        // The wait covers the operator's own changes as much as the tool's:
        // whatever ended this step's wait, `--settle` says the reading behind
        // it needs longer. The frame that ended the wait was read before it,
        // so a run that waits reads its whole batch afterwards.
        if driver.wait_to_settle() {
            settled = None;
        }
        let mut measurements: Vec<Measurement> = settled.into_iter().collect();
        let wanted = step.samples.saturating_sub(measurements.len());
        measurements.extend(capture_samples(dmm, wanted, &mut errors));
        let sample_data: Vec<SampleData> = measurements
            .iter()
            .map(SampleData::from_measurement)
            .collect();

        for (i, s) in sample_data.iter().enumerate() {
            eprintln!(
                "    {} mode={}({}) range={} display={:?}",
                style(format!("[{i}]")).dim(),
                s.mode_byte,
                s.mode,
                s.range_label,
                s.display_raw
            );
        }

        let confirmation = if let Some(last) = sample_data.last() {
            if interactive {
                eprintln!("  We read: {}", style(last.summary()).green());
            }
            // A trusted run doesn't stop here: the reading is listed with the
            // others in the one review at the end.
            //
            // `is_tty`, as the end-of-run review does: a piped run's stdin
            // answers every prompt with the empty line EOF gives, which reads
            // as "the meter showed exactly this" — and a gate confirmed that
            // way promotes the run and writes `core_semantics: confirmed`
            // into the report without anyone having looked at the meter.
            if interactive && input.is_tty() && trust.confirm_inline(step) {
                let answer = input.line(&format!(
                    "  {} ",
                    style("Enter=correct, r=retake, or type what the meter actually shows:").dim()
                ))?;
                match Confirmation::of(answer) {
                    Confirmation::Retake => {
                        eprintln!("  {}", style("retaking\u{2026}").dim());
                        continue;
                    }
                    Confirmation::Answer(answer) => Some(answer),
                }
            } else {
                None
            }
        } else {
            eprintln!("  {}", style("No response from meter.").yellow());
            None
        };

        break (sample_data, measurements, confirmation);
    };

    let diagnostics = errors.into_diagnostics();
    let (frames, frames_dropped) = {
        let mut rec = recording::lock(recorder);
        rec.set_step(None);
        frames_for_step(&rec.drain(), step.id)
    };

    let status = if sample_data.is_empty() {
        StepStatus::Timeout
    } else if sample_data.len() < step.samples {
        eprintln!(
            "  {} only got {}/{} samples",
            style("warning:").yellow(),
            sample_data.len(),
            step.samples
        );
        StepStatus::Captured
    } else {
        StepStatus::Captured
    };

    let to_review = confirmation.is_none() && status == StepStatus::Captured;
    let mut result = StepResult {
        needs_attention: needs_attention(&sample_data, step.samples, &diagnostics),
        samples: sample_data,
        frames,
        frames_dropped,
        diagnostics,
        ..StepResult::new(step.id, step.instruction, status)
    };
    if let Some(answer) = confirmation {
        result.set_inline_confirmation(answer);
    }

    upsert_step(report, result);
    report.wire_events_dropped = recording::lock(recorder).dropped();
    Ok(StepOutcome::done(measurements.pop(), to_review))
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `capture_steps` is a Protocol method; the trait has to be in scope to
    // call it on a concrete protocol type (not needed for `dyn Protocol`).
    use dmm_lib::protocol::Protocol;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;
    use std::sync::Mutex;

    /// A UT61E+ frame: AB CD, length, payload, 16-bit BE sum.
    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut f = vec![0xAB, 0xCD, (payload.len() + 2) as u8];
        f.extend_from_slice(payload);
        let sum = f.iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        f.extend_from_slice(&sum.to_be_bytes());
        f
    }

    fn measurement_frame() -> Vec<u8> {
        frame(&[
            0x02, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, 0x30, 0x30, 0x30,
        ])
    }

    /// Canned transport: hands out one queued response per read, then goes
    /// silent the way a real meter does on timeout.
    struct QueuedTransport {
        responses: Mutex<std::collections::VecDeque<Vec<u8>>>,
    }

    impl dmm_lib::transport::Transport for QueuedTransport {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            let Some(next) = self.responses.lock().unwrap().pop_front() else {
                return Ok(0);
            };
            let n = next.len().min(buf.len());
            buf[..n].copy_from_slice(&next[..n]);
            Ok(n)
        }

        fn send_feature_report(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }
    }

    fn dmm_replaying(
        responses: Vec<Vec<u8>>,
    ) -> dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>> {
        let transport = QueuedTransport {
            responses: Mutex::new(responses.into()),
        };
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        dmm_lib::Dmm::new(
            Box::new(transport) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap()
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
        let samples = capture_samples(&mut dmm, 1, &mut errors);
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
        let samples = capture_samples(&mut dmm, 1, &mut errors);
        let diagnostics = errors.into_diagnostics();

        assert!(samples.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].ends_with("(x3)"), "got {diagnostics:?}");
    }

    /// A command the meter ignored has to say what it is still showing, or
    /// the report reads as though the step never ran.
    #[test]
    fn a_dead_command_names_what_the_meter_still_shows() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(
            did_nothing("hold", Some(&m)),
            "hold did nothing; the meter still shows 5.678 V [AUTO]"
        );
        assert_eq!(
            did_nothing("hold", None),
            "hold did nothing; the meter sent no reading"
        );
    }

    /// Only the events tagged with the step, and no more than the cap — the
    /// last of them: a step that waited for the meter before sampling would
    /// otherwise fill the report with the wait and drop the samples.
    #[test]
    fn step_frames_are_filtered_and_capped_to_the_newest() {
        let event = |at_ms: u64, step: Option<&str>| WireEvent {
            at_ms,
            dir: crate::recording::Direction::Rx,
            step: step.map(str::to_string),
            bytes: vec![0xAB, 0xCD],
            feature: false,
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
                dir: crate::recording::Direction::Rx,
                step: Some("ncv".to_string()),
                bytes: vec![0xAB, 0xCD],
                feature: false,
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

    /// The retake key has to be told from a reading typed at the same prompt:
    /// only a bare `r` redoes the step.
    #[test]
    fn r_alone_asks_for_a_retake() {
        assert_eq!(Confirmation::of("r".to_string()), Confirmation::Retake);
        assert_eq!(Confirmation::of("R".to_string()), Confirmation::Retake);
        for typed in ["", "5.68 V", "rel", "R 0.5"] {
            assert_eq!(
                Confirmation::of(typed.to_string()),
                Confirmation::Answer(typed.to_string()),
                "{typed:?}"
            );
        }
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
    /// The four below still are, for two different reasons:
    /// - `ut181a`, `vc880`, `vc650bt`, `vc890` declare `choices`, so they do
    ///   lose coverage. Each fix needs that family's own dial order and a
    ///   hardware run — tracked in `docs/verification-backlog.md`.
    /// - `ut8802`, `ut8803`, `ut803`, `ut804`, `ut171` declare no `choices`,
    ///   so nothing is swept whatever the order and they cost nothing.
    const SPLIT_GATE: &[&str] = &[
        "ut8802", "ut8803", "ut803", "ut804", "ut171", "ut181a", "vc880", "vc650bt", "vc890",
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
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
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
        use crate::capture::input::Input;
        use crate::drive::Driver;

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
        };

        let filed = |settle: Duration| {
            let mut responses = vec![measurement_frame(); 3]; // the baseline
            responses.extend(vec![measurement_frame(); 3]); // drained by the press
            responses.extend(vec![held(b"  5.678"); 3]); // ends the watcher's wait
            responses.extend(vec![held(b"  1.234"); 2]); // what it settles to
            let mut dmm = dmm_replaying(responses);
            // The step's frames are recorded through the transport the
            // recorder wraps; this one only has to exist.
            let (_unused, recorder) = crate::recording::RecordingTransport::new(Box::new(
                dmm_lib::transport::NullTransport,
            ));
            let mut report = CaptureReport::default();
            let mut driver = Driver::new(false).settling(settle);
            run_capture_step(
                &mut dmm,
                &recorder,
                &step,
                &mut report,
                false,
                &Input::piped(),
                &PrevState::default(),
                &Trust::new(false, true, &[]),
                &mut driver,
            )
            .unwrap();
            let displays: Vec<String> = report.steps[0]
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
}
