//! The `capture` command: the guided run that walks a meter through its
//! protocol's steps and writes the report a device report is built on.

mod input;
mod listing;
mod report;
mod session;
mod step;

pub(crate) use input::{ErrorLog, Input};
pub(crate) use listing::list_steps;
pub(crate) use report::{
    CaptureReport, SampleData, StepResult, StepStatus, needs_attention, save_report, upsert_step,
};
pub(crate) use step::{CaptureStep, FREEFORM_STEP_ID, capture_samples, frames_for_step};

use crate::recording::{self, SharedRecorder};
use console::style;
use listing::validate_step_filter;
use report::{
    FrameRecord, Trust, captured_count, load_or_create_report, no_response_path,
    populate_report_metadata,
};
use session::{run_batch_review, run_freeform_captures, run_protocol_capture, verify_meter};

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    unverified_only: bool,
    sniff: bool,
    no_drive: bool,
    settle: std::time::Duration,
    plan_path: Option<String>,
    mut dmm: dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: SharedRecorder,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_filter: Option<std::collections::HashSet<String>> =
        filter.map(|v| v.into_iter().collect());
    // Before the meter is touched: a plan the tool can't read is the
    // reporter's typo, and they should hear about it straight away.
    let plan_steps = plan_path.as_deref().map(crate::plan::load).transpose()?;

    let (device_name, supported) = match verify_meter(&mut dmm, device) {
        Ok(verified) => verified,
        Err(e) => {
            match save_no_response(&mut dmm, &recorder, device, output_override.as_deref()) {
                Ok((path, rx_bytes)) => {
                    eprintln!(
                        "Bytes received: {rx_bytes}, saved to {}",
                        style(&path).bold()
                    );
                    eprintln!(
                        "If the meter is on and still not answering, attach that file to {}",
                        dmm.profile().feedback_url()
                    );
                }
                Err(save) => eprintln!("Could not save the bytes received: {save}"),
            }
            return Err(e);
        }
    };

    let input = Input::start();
    let (mut report, output_path) =
        match load_or_create_report(output_override, device.id, plan_path.as_deref(), &input)? {
            Some(pair) => pair,
            None => return Ok(()),
        };

    eprintln!("Output file: {output_path}\n");

    populate_report_metadata(&mut report, &mut dmm, device_name, supported);
    report.device_id = Some(device.id.to_string());
    report.plan = plan_path.clone();
    report.unverified_only = unverified_only;
    // Everything on the wire so far is the detection probe (when the meter
    // was not named), the init handshake and the name query.
    report.init_frames = recording::lock(&recorder)
        .drain()
        .iter()
        .map(FrameRecord::from)
        .collect();

    validate_step_filter(&step_filter, &dmm.capture_steps())?;

    // The device's own steps cover modes, flags and button commands;
    // the freeform pass afterwards is device-agnostic and always offered,
    // since it is the only way to capture a mode the step list doesn't
    // anticipate — and the only step that records what the meter's screen
    // actually said next to what we parsed.
    let protocol_steps = dmm.capture_steps();
    let unverified_ids: std::collections::HashSet<&str> = protocol_steps
        .iter()
        .filter(|s| !s.verified)
        .map(|s| s.id)
        .collect();
    // A plan replaces the device's list for the run — same watcher, tiers,
    // needs checklist and sweeps, but only the steps the maintainer wrote.
    let cli_steps: Vec<CaptureStep> = match &plan_steps {
        Some(steps) => steps.clone(),
        None => protocol_steps.iter().map(CaptureStep::from).collect(),
    };
    // `--unverified` leaves out the steps hardware has already confirmed,
    // which is the same evidence the gate is looking for: counting one of
    // those as a gate step it never sees would leave the gate undecided for
    // the whole run — no deferred review, no sweeps — on the very command
    // `--list-steps` tells a reporter to run.
    let gate_scope: Vec<CaptureStep> = cli_steps
        .iter()
        .copied()
        .filter(|s| !(unverified_only && s.verified))
        .collect();
    let mut trust = Trust::new(sniff, supported, &gate_scope);
    report.tier = Some(trust.tier);
    let mut driver = crate::drive::Driver::new(!no_drive).settling(settle);
    let pass = run_protocol_capture(
        &mut dmm,
        &recorder,
        &cli_steps,
        &step_filter,
        unverified_only,
        &mut report,
        &output_path,
        &input,
        &mut trust,
        &mut driver,
    )?;
    report.drive = Some(driver.state());

    run_batch_review(&mut report, &pass.to_review, &output_path, &input)?;

    if !pass.quit {
        run_freeform_captures(
            &mut dmm,
            &recorder,
            &step_filter,
            &mut report,
            &output_path,
            &input,
        )?;
    }

    report.wire_events_dropped = recording::lock(&recorder).dropped();
    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    let (covered, total) = if let Some(plan) = &plan_path {
        // The plan's steps are not in the device's unverified list, so its
        // coverage line would read zero for a run that captured everything.
        let ids: std::collections::HashSet<&str> = cli_steps.iter().map(|s| s.id).collect();
        let covered = captured_count(&report, &ids);
        eprintln!("Plan {plan}: {covered} of {} steps captured", ids.len());
        (covered, ids.len())
    } else {
        let covered = captured_count(&report, &unverified_ids);
        if unverified_only && covered == 0 {
            eprintln!("No unverified step was captured, so there is nothing new to report.");
        } else {
            eprintln!(
                "Covered {covered} of {} unverified steps for {}.",
                unverified_ids.len(),
                device.display_name
            );
        }
        (covered, unverified_ids.len())
    };
    if let Some(hint) = resume_hint(covered, total, plan_path.is_some()) {
        eprintln!("{hint}");
    }
    eprintln!("Attach the report to {}", dmm.profile().feedback_url());
    Ok(())
}

/// The way back into a run that left steps undone. A capture that ended on the
/// first `q` signs off with the same "Capture complete!" as one that walked
/// every step, and said nothing about the report being resumable.
fn resume_hint(covered: usize, total: usize, plan: bool) -> Option<String> {
    if covered >= total {
        return None;
    }
    let mut hint =
        "Not finished \u{2014} run the same command again and answer r to resume.".to_string();
    // `--steps` conflicts with `--plan`, so it is not on offer for a plan run.
    if !plan {
        hint.push_str("\n--steps <ids> runs named steps on their own.");
    }
    Some(hint)
}

/// Keep what the cable delivered when the meter never answered: on a meter
/// being brought up, bytes that did not decode are all the evidence there is.
/// Returns the file written and how many bytes it holds.
fn save_no_response(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
    output_override: Option<&str>,
) -> Result<(String, usize), Box<dyn std::error::Error>> {
    let mut report = CaptureReport::default();
    let supported = dmm.profile().stability.is_verified();
    // `verify_meter` only fails when the meter gave no name, so there is none
    // to put here; "unknown" is what a nameless meter's report says too.
    populate_report_metadata(&mut report, dmm, "unknown".to_string(), supported);
    report.device_id = Some(device.id.to_string());
    report.no_response = true;
    let events = {
        let mut recorder = recording::lock(recorder);
        report.wire_events_dropped = recorder.dropped();
        recorder.drain()
    };
    let rx_bytes = events
        .iter()
        .filter(|e| e.dir == recording::Direction::Rx && !e.feature)
        .map(|e| e.bytes.len())
        .sum();
    report.init_frames = events.iter().map(FrameRecord::from).collect();

    // Reserve a fresh name, then write over the reservation atomically.
    let (path, _) = crate::output::create_new(&no_response_path(output_override, device.id))?;
    let path = path.to_string_lossy().into_owned();
    save_report(&report, &path)?;
    Ok((path, rx_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::transport::Transport;
    use std::cell::RefCell;

    /// Hands up `chunks` one read at a time, then nothing.
    struct Chunks(RefCell<Vec<Vec<u8>>>);

    impl Transport for Chunks {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            let mut chunks = self.0.borrow_mut();
            if chunks.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(10));
                return Ok(0);
            }
            let chunk = chunks.remove(0);
            buf[..chunk.len()].copy_from_slice(&chunk);
            Ok(chunk.len())
        }

        fn send_feature_report(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }
    }

    /// A UT804 that sends bytes the decoder never frames: the run stops at
    /// the meter check, and the bytes land in a report of their own beside
    /// the one `-o` names, which a later run can still resume.
    #[test]
    fn a_meter_that_never_answers_leaves_its_bytes_beside_the_report() {
        let dir = std::env::temp_dir().join(format!("dmm-no-response-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let output = dir.join("capture.yaml");
        std::fs::write(&output, "steps: []\n").unwrap();
        let taken = dir.join("capture-no-response.yaml");
        std::fs::write(&taken, "an earlier run\n").unwrap();

        let device = dmm_lib::protocol::registry::find_device("ut804").unwrap();
        let wire = Chunks(RefCell::new(vec![vec![0x55, 0xAA, 0x01], vec![0x02]]));
        let (transport, recorder) = crate::recording::RecordingTransport::new(Box::new(wire));
        let dmm = dmm_lib::Dmm::new(
            Box::new(transport) as Box<dyn Transport>,
            (device.new_protocol)(),
        )
        .unwrap();

        let result = cmd_capture(
            Some(output.to_string_lossy().into_owned()),
            None,
            false,
            false,
            false,
            std::time::Duration::ZERO,
            None,
            dmm,
            recorder,
            device,
        );
        assert_eq!(result.unwrap_err().to_string(), "meter not responding");

        assert_eq!(std::fs::read_to_string(&output).unwrap(), "steps: []\n");
        assert_eq!(std::fs::read_to_string(&taken).unwrap(), "an earlier run\n");
        let saved = dir.join("capture-no-response-2.yaml");
        let report: CaptureReport =
            serde_yaml_ng::from_str(&std::fs::read_to_string(&saved).unwrap()).unwrap();
        assert!(report.no_response);
        assert!(report.steps.is_empty());
        assert_eq!(report.device_id.as_deref(), Some("ut804"));
        let hex: Vec<&str> = report.init_frames.iter().map(|f| f.hex.as_str()).collect();
        assert_eq!(hex, ["55 AA 01 02"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A run that stopped early has to say it can be picked up; one that
    /// covered everything has nothing to add.
    #[test]
    fn an_unfinished_run_says_how_to_resume() {
        let hint = resume_hint(1, 27, false).expect("26 steps left");
        assert!(hint.contains("run the same command again"), "{hint}");
        assert!(hint.contains("--steps"), "{hint}");

        // `--steps` conflicts with `--plan`, so a plan run is not sent to it.
        let hint = resume_hint(1, 2, true).expect("1 step left");
        assert!(hint.contains("run the same command again"), "{hint}");
        assert!(!hint.contains("--steps"), "{hint}");

        assert_eq!(resume_hint(27, 27, false), None);
        assert_eq!(resume_hint(0, 0, false), None);
    }

    #[test]
    fn the_no_response_report_is_named_after_the_device_or_the_output() {
        assert_eq!(
            no_response_path(None, "ut804"),
            std::path::PathBuf::from("capture-ut804-no-response.yaml")
        );
        assert_eq!(
            no_response_path(Some("runs/bench.yaml"), "ut804"),
            std::path::PathBuf::from("runs/bench-no-response.yaml")
        );
    }
}
