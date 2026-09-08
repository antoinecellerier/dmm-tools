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
use report::{FrameRecord, Trust, captured_count, load_or_create_report, populate_report_metadata};
use session::{run_batch_review, run_freeform_captures, run_protocol_capture, verify_meter};

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    unverified_only: bool,
    sniff: bool,
    no_drive: bool,
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

    let (device_name, supported) = verify_meter(&mut dmm, device)?;

    let input = Input::start();
    let (mut report, output_path) =
        match load_or_create_report(output_override, &device_name, plan_path.as_deref(), &input)? {
            Some(pair) => pair,
            None => return Ok(()),
        };

    eprintln!("Output file: {output_path}\n");

    populate_report_metadata(&mut report, &mut dmm, device_name, supported);
    report.device_id = Some(device.id.to_string());
    report.plan = plan_path.clone();
    report.unverified_only = unverified_only;
    // Everything on the wire so far is the init handshake and the name query.
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
    let mut driver = crate::drive::Driver::new(!no_drive);
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
    if let Some(plan) = &plan_path {
        // The plan's steps are not in the device's unverified list, so its
        // coverage line would read zero for a run that captured everything.
        let ids: std::collections::HashSet<&str> = cli_steps.iter().map(|s| s.id).collect();
        eprintln!(
            "Plan {plan}: {} of {} steps captured",
            captured_count(&report, &ids),
            ids.len()
        );
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
    }
    eprintln!("Attach the report to {}", dmm.profile().feedback_url());
    Ok(())
}
