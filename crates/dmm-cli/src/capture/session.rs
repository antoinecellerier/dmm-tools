//! The passes a capture run makes: the meter handshake, the device's own
//! steps and the equipment they ask for, the end-of-run review, and freeform.

use super::input::{ErrorLog, Input};
use super::listing::step_included;
use super::report::{
    CaptureReport, SampleData, StepResult, StepStatus, Trust, already_captured,
    baseline_from_report, needs_attention, save_report, upsert_step,
};
use super::step::{
    CaptureStep, Confirmed, NO_RESPONSE, PrevState, WHAT_SHOWN, ask_confirmation, capture_samples,
    frames_for_step, run_capture_step,
};
use crate::recording::{self, SharedRecorder};
use console::style;
use dmm_lib::protocol::{Need, Stability};

/// What the status banner says about the model the run was started for: the
/// headline, and the line under it for a family hardware has not fully
/// confirmed.
fn status_banner(stability: Stability) -> (&'static str, Option<&'static str>) {
    match stability {
        Stability::Verified => ("supported model", None),
        Stability::PartlyVerified => (
            "partly verified \u{2014} connection and main modes confirmed",
            Some("The modes still unconfirmed are what this run is for."),
        ),
        Stability::Experimental => (
            "experimental \u{2014} the protocol is reverse-engineered, not confirmed",
            Some("Please complete as many steps as possible and share the report."),
        ),
    }
}

/// Verify that the meter is responding. Returns `(device_name, supported)` on success.
///
/// The name is what the meter calls itself, and "unknown" for the families
/// that answer no name query — the report records it as such, while the file
/// and the banner name the model the run was started for.
///
/// A name the detection probe already got is the session's: the meter has
/// answered once, so it is neither asked again nor made to beep again.
pub(super) fn verify_meter(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
) -> Result<(String, bool), Box<dyn std::error::Error>> {
    eprintln!("{}", style("Checking meter communication...").dim());
    let reported = match dmm.get_name() {
        Ok(Some(name)) => Some(name),
        Ok(None) | Err(_) => {
            // get_name failed or unsupported — try a plain measurement as fallback
            match dmm.request_measurement() {
                Ok(_) => None,
                Err(_) => {
                    eprintln!();
                    // Name the link the session is on: "adapter" now reads as
                    // the Bluetooth one, and over a cable it never was one.
                    let link = dmm_lib::binary_help::bridge_link_name(
                        dmm.transport().transport_name(),
                        device.bluetooth_only,
                    );
                    eprintln!(
                        "{}",
                        style(format!(
                            "The {link} is connected but the meter isn't responding."
                        ))
                        .yellow()
                        .bold()
                    );
                    eprintln!("To enable data transmission:");
                    for line in device.activation_instructions.lines() {
                        eprintln!("  {line}");
                    }
                    eprintln!();
                    eprintln!("Then run this command again.");
                    return Err("meter not responding".into());
                }
            }
        }
    };

    match &reported {
        Some(name) => eprintln!("Device: {}", style(name).bold()),
        // The model the run was started for, said so it is not read as the
        // meter's own answer.
        None => eprintln!(
            "Device: {} (the meter does not report a name)",
            style(device.display_name).bold()
        ),
    }
    let stability = dmm.profile().stability;
    let (headline, detail) = status_banner(stability);
    if stability.is_verified() {
        eprintln!("Status: {}", style(headline).green());
    } else {
        eprintln!("Status: {}", style(headline).yellow().bold());
    }
    if let Some(detail) = detail {
        eprintln!("        {detail}");
    }
    eprintln!();

    Ok((
        reported.unwrap_or_else(|| "unknown".to_string()),
        stability.is_verified(),
    ))
}

/// What the protocol pass left behind: whether the operator asked to finish,
/// and the steps captured without a confirmation.
pub(super) struct ProtocolPass {
    pub(super) quit: bool,
    pub(super) to_review: Vec<String>,
}

/// Run the device's own capture steps: modes, flags, and the manual range
/// sweep, in the order the protocol declares them.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_protocol_capture(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    all_steps: &[CaptureStep],
    step_filter: &Option<std::collections::HashSet<String>>,
    unverified_only: bool,
    report: &mut CaptureReport,
    output_path: &str,
    input: &Input,
    trust: &mut Trust,
    driver: &mut crate::drive::Driver,
) -> Result<ProtocolPass, Box<dyn std::error::Error>> {
    // Keep only what this run selects: the checklist below covers the steps
    // that will actually run.
    let mut steps: Vec<CaptureStep> = all_steps
        .iter()
        .copied()
        .filter(|s| step_selected(s, step_filter, unverified_only))
        .collect();

    // Only what is still to do: a resumed run must not ask for the equipment
    // of a step it already has samples for, nor overwrite that step's result.
    let pending: Vec<CaptureStep> = steps
        .iter()
        .filter(|s| !already_captured(report, s.id))
        .copied()
        .collect();
    let missing = ask_missing_needs(&pending, input)?;
    let dropped = steps_without(&pending, &missing);
    for (step, label) in &dropped {
        upsert_step(
            report,
            step.empty_result(StepStatus::Skipped, Some(format!("skipped: no {label}"))),
        );
    }
    if !dropped.is_empty() {
        let ids: Vec<&str> = dropped.iter().map(|(s, _)| s.id).collect();
        steps.retain(|s| !ids.contains(&s.id));
        save_report(report, output_path)?;
    }

    eprintln!(
        "{}",
        style("\u{2501}\u{2501}\u{2501} Measurement Modes \u{2501}\u{2501}\u{2501}").bold()
    );
    eprintln!(
        "{}",
        style("each step captures itself once the meter settles \u{2014} Enter=capture now, s=skip one, q=skip to end and save").dim()
    );

    // A resumed run inherits the gate its earlier half already passed.
    trust.update(report);

    // What the meter was left showing, so the next step can tell a new state
    // from it.
    let mut prev = PrevState::default();
    let mut to_review = Vec::new();
    for step in &steps {
        let outcome = run_capture_step(
            dmm, recorder, step, report, true, input, &prev, trust, driver,
        )?;
        if outcome.to_review {
            to_review.push(step.id.to_string());
        }
        trust.update(report);
        // After the gate, and only for a step the operator set by hand: a
        // command step's own flag is what the sweep would be undoing. A step
        // the report already holds captures nothing, so a resumed run does
        // not sweep it — the dial is no longer where its sub-steps assume.
        if trust.drives()
            && !step.gate
            && step.command.is_none()
            && let Some(last) = &outcome.last
        {
            crate::drive::sweep_step(dmm, recorder, step, last, driver, report, output_path)?;
        }
        prev.last = outcome.last;
        // From the report, so a resumed run gets its baseline from the steps
        // it skipped as already captured. A step that captured nothing leaves
        // the previous baseline standing: it is still the last state the
        // meter was seen in.
        if let Some(next) = baseline_from_report(report, step.id) {
            prev.baseline = Some(next);
        }
        save_report(report, output_path)?;
        if outcome.quit {
            return Ok(ProtocolPass {
                quit: true,
                to_review,
            });
        }
    }

    Ok(ProtocolPass {
        quit: false,
        to_review,
    })
}

/// Whether the protocol pass runs this step: `--steps` and `--unverified`
/// narrow the list together, so naming a verified step under `--unverified`
/// still skips it.
fn step_selected(
    step: &CaptureStep,
    step_filter: &Option<std::collections::HashSet<String>>,
    unverified_only: bool,
) -> bool {
    step_included(step_filter, step.id) && !(unverified_only && step.verified)
}

/// The equipment the given steps ask for, numbered in `Need::ALL` order, and
/// the needs behind those numbers. `None` when the run needs nothing beyond
/// the meter and its leads.
fn render_needs(steps: &[CaptureStep]) -> Option<(String, Vec<Need>)> {
    let mut needs = Vec::new();
    let mut lines = String::new();
    for need in Need::ALL {
        let ids: Vec<&str> = steps
            .iter()
            .filter(|s| s.needs.contains(&need))
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            continue;
        }
        needs.push(need);
        lines.push_str(&format!(
            "  [{}] {}  (steps: {})\n",
            needs.len(),
            need.label(),
            ids.join(", ")
        ));
    }
    (!needs.is_empty()).then(|| (format!("You will need:\n{lines}"), needs))
}

/// The steps waiting on something the operator hasn't got, each with the item
/// it was waiting on.
fn steps_without<'a>(
    steps: &'a [CaptureStep],
    missing: &[Need],
) -> Vec<(&'a CaptureStep, &'static str)> {
    steps
        .iter()
        .filter_map(|s| {
            let need = missing.iter().find(|n| s.needs.contains(n))?;
            Some((s, need.label()))
        })
        .collect()
}

/// Show what the run needs and ask which of it is missing, so the bench is set
/// up once instead of a thermocouple turning up as a step in the middle.
///
/// A run with nobody to ask attempts everything: the steps are still worth
/// offering to a meter that might be sitting on the right source already.
fn ask_missing_needs(
    steps: &[CaptureStep],
    input: &Input,
) -> Result<Vec<Need>, Box<dyn std::error::Error>> {
    let Some((table, needs)) = render_needs(steps) else {
        return Ok(Vec::new());
    };
    eprintln!();
    eprint!("{table}");
    if !input.is_tty() {
        return Ok(Vec::new());
    }
    let missing = loop {
        let answer =
            input.line("Numbers of anything you don't have (Enter = have everything): ")?;
        match parse_review_indices(&answer, needs.len()) {
            Ok(indices) => break indices,
            Err(e) => eprintln!("  {}", style(e).yellow()),
        }
    };
    Ok(missing.into_iter().map(|i| needs[i]).collect())
}

/// The numbered list the operator reads against the meter's screen, one line
/// per reading captured without a confirmation.
fn render_review_table(rows: &[(String, String)]) -> String {
    let mut out = String::new();
    for (i, (id, summary)) in rows.iter().enumerate() {
        out.push_str(&format!(
            "  {:>3}  {:<16} {summary}\n",
            i + 1,
            style(id).cyan()
        ));
    }
    out
}

/// The readings the operator listed as wrong, as indices into the table.
///
/// One-based on the way in, since that is what the table shows; an empty line
/// means every reading matched.
fn parse_review_indices(input: &str, len: usize) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for token in input.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let n = token
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=len).contains(n))
            .ok_or_else(|| format!("{token:?} is not a number between 1 and {len}"))?;
        if !out.contains(&(n - 1)) {
            out.push(n - 1);
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// Write the review's verdict into the report: `Some(text)` is what the meter
/// showed for a reading the operator called wrong, `None` confirms it.
fn apply_review(report: &mut CaptureReport, answers: &[(String, Option<String>)]) {
    for (id, lcd) in answers {
        if let Some(step) = report.steps.iter_mut().find(|s| s.id == *id) {
            step.set_batch_confirmation(lcd.clone());
        }
    }
}

/// Ask about every reading the run captured without stopping for it, once.
///
/// A piped run has nobody to ask, so those readings stay unconfirmed rather
/// than being recorded as agreed.
pub(super) fn run_batch_review(
    report: &mut CaptureReport,
    to_review: &[String],
    output_path: &str,
    input: &Input,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<(String, String)> = to_review
        .iter()
        .filter_map(|id| {
            let step = report.steps.iter().find(|s| s.id == *id)?;
            Some((id.clone(), step.samples.last()?.summary()))
        })
        .collect();
    if rows.is_empty() || !input.is_tty() {
        return Ok(());
    }

    eprintln!(
        "\n{}",
        style("\u{2501}\u{2501}\u{2501} Review the readings \u{2501}\u{2501}\u{2501}").bold()
    );
    eprint!("{}", render_review_table(&rows));

    let wrong = loop {
        let answer = input.line(
            "Numbers of the readings that did NOT match the meter's screen (Enter = all correct): ",
        )?;
        match parse_review_indices(&answer, rows.len()) {
            Ok(indices) => break indices,
            Err(e) => eprintln!("  {}", style(e).yellow()),
        }
    };

    let mut answers: Vec<(String, Option<String>)> = Vec::with_capacity(rows.len());
    for (i, (id, _)) in rows.iter().enumerate() {
        let lcd = if wrong.contains(&i) {
            Some(input.line(&format!("[{id}] {WHAT_SHOWN} "))?)
        } else {
            None
        };
        answers.push((id.clone(), lcd));
    }
    apply_review(report, &answers);
    save_report(report, output_path)?;
    Ok(())
}

/// Samples taken per freeform capture.
const FREEFORM_SAMPLES: usize = 3;

/// Whether what was typed at the freeform prompt ends the pass rather than
/// describing a capture. Any word starting with `q` used to end it, so
/// "quick check on Ω" finished the run.
fn ends_freeform(desc: &str) -> bool {
    desc.is_empty() || desc.eq_ignore_ascii_case("q") || desc.eq_ignore_ascii_case("quit")
}

/// Part 1: Run measurement mode capture steps. Returns true if user wants to quit.
/// Part 4: Freeform additional captures.
pub(super) fn run_freeform_captures(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step_filter: &Option<std::collections::HashSet<String>>,
    report: &mut CaptureReport,
    output_path: &str,
    input: &Input,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_filtered = step_filter.is_some();
    if is_filtered && !step_included(step_filter, "extra") {
        return Ok(());
    }

    eprintln!(
        "\n{}",
        style("\u{2501}\u{2501}\u{2501} Part 4: Additional Captures (optional) \u{2501}\u{2501}\u{2501}")
            .bold()
    );
    eprintln!("Set the meter to any mode/state not covered above.\n");

    // A resumed report keeps its freeform steps: number on from them.
    let mut extra = report
        .steps
        .iter()
        .filter_map(|s| s.id.strip_prefix("extra_")?.parse::<u32>().ok())
        .max()
        .map_or(0, |n| n + 1);
    loop {
        let desc = input.line(&format!(
            "[extra_{extra}] Describe what you set the meter to (or 'q' to finish): "
        ))?;
        if ends_freeform(&desc) {
            break;
        }

        let step_id = format!("extra_{extra}");
        recording::lock(recorder).set_step(Some(&step_id));
        let mut errors = ErrorLog::default();
        let measurements = capture_samples(dmm, FREEFORM_SAMPLES, &mut errors);
        let diagnostics = errors.into_diagnostics();
        let sample_data: Vec<SampleData> = measurements
            .iter()
            .map(SampleData::from_measurement)
            .collect();
        let (frames, frames_dropped) = {
            let mut rec = recording::lock(recorder);
            rec.set_step(None);
            frames_for_step(&rec.drain(), &step_id)
        };

        for (i, s) in sample_data.iter().enumerate() {
            eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
        }

        let confirmation = if let Some(last) = sample_data.last() {
            eprintln!("  We read: {}", style(last.summary()).green());
            match ask_confirmation(input, false)? {
                Confirmed::Reading(shown) => Some(shown),
                // No retake is offered above, so this cannot come back; a step
                // left unanswered is the one outcome that records nothing the
                // operator did not say.
                Confirmed::Retake => None,
            }
        } else {
            eprintln!("  {NO_RESPONSE}");
            None
        };

        let status = if sample_data.is_empty() {
            StepStatus::Timeout
        } else {
            StepStatus::Captured
        };
        let mut result = StepResult {
            needs_attention: needs_attention(&sample_data, FREEFORM_SAMPLES, &diagnostics),
            samples: sample_data,
            frames,
            frames_dropped,
            diagnostics,
            ..StepResult::new(&step_id, &desc, status)
        };
        if let Some(shown) = confirmation {
            result.set_inline_confirmation(shown);
        }
        upsert_step(report, result);
        report.wire_events_dropped = recording::lock(recorder).dropped();
        save_report(report, output_path)?;
        extra += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::report::ConfirmedBy;
    use crate::capture::step::cli_step;

    /// The freeform prompt takes a description, so only `q` itself finishes
    /// the run: any word starting with `q` used to, and "quick check on Ω"
    /// ended the capture.
    #[test]
    fn only_q_itself_finishes_the_freeform_pass() {
        for quit in ["", "q", "Q", "quit", "QUIT"] {
            assert!(ends_freeform(quit), "{quit:?}");
        }
        for desc in ["quick check", "Q10 range", "qualifying Ω", "dcv"] {
            assert!(!ends_freeform(desc), "{desc:?}");
        }
    }

    /// Every model is picked from the registry, so the banner's job is to say
    /// how far its protocol has been confirmed. It had two branches and told a
    /// partly verified meter's owner they had an unknown model.
    #[test]
    fn the_banner_says_how_far_the_family_is_confirmed() {
        assert_eq!(
            status_banner(Stability::Verified),
            ("supported model", None)
        );

        let (headline, detail) = status_banner(Stability::PartlyVerified);
        assert!(headline.starts_with("partly verified"), "{headline}");
        assert!(detail.unwrap().contains("still unconfirmed"));

        let (headline, detail) = status_banner(Stability::Experimental);
        assert!(headline.starts_with("experimental"), "{headline}");
        assert!(detail.unwrap().contains("as many steps as possible"));
    }

    #[test]
    fn review_indices_are_one_based_and_bounded() {
        assert_eq!(parse_review_indices("", 3), Ok(vec![]));
        assert_eq!(parse_review_indices("  ", 3), Ok(vec![]));
        assert_eq!(parse_review_indices("1,3", 3), Ok(vec![0, 2]));
        assert_eq!(parse_review_indices("3 1", 3), Ok(vec![0, 2]));
        assert_eq!(parse_review_indices("2, 2", 3), Ok(vec![1]));

        for bad in ["0", "4", "-1", "two", "1,x"] {
            assert!(
                parse_review_indices(bad, 3).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn the_review_table_numbers_every_reading() {
        let rows = vec![
            ("dcv".to_string(), "5.678 V [AUTO]".to_string()),
            ("acv".to_string(), "239.22 V [AUTO HV!]".to_string()),
        ];
        let out = console::strip_ansi_codes(&render_review_table(&rows)).into_owned();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("    1  dcv"), "{out}");
        assert!(lines[0].ends_with("5.678 V [AUTO]"), "{out}");
        assert!(lines[1].starts_with("    2  acv"), "{out}");
    }

    /// The review is the only confirmation a trusted run's readings get, so
    /// it has to record both verdicts the way the inline prompt does.
    #[test]
    fn the_review_confirms_and_corrects_the_readings_it_covers() {
        let mut report = CaptureReport::default();
        for id in ["dcv", "acv", "ohm"] {
            upsert_step(&mut report, StepResult::new(id, "t", StepStatus::Captured));
        }

        apply_review(
            &mut report,
            &[
                ("dcv".to_string(), None),
                ("acv".to_string(), Some("239.4 V".to_string())),
                // Listed as wrong but typed nothing: still a mismatch.
                ("ohm".to_string(), Some(String::new())),
            ],
        );

        assert_eq!(report.steps[0].confirmed, Some(true));
        assert_eq!(report.steps[0].lcd, None);
        assert_eq!(report.steps[0].confirmed_by, Some(ConfirmedBy::Batch));
        assert_eq!(report.steps[1].confirmed, Some(false));
        assert_eq!(report.steps[1].lcd.as_deref(), Some("239.4 V"));
        assert_eq!(report.steps[1].confirmed_by, Some(ConfirmedBy::Batch));
        assert_eq!(report.steps[2].confirmed, Some(false));
        assert_eq!(report.steps[2].lcd, None);
    }

    fn needy_step(id: &'static str, needs: &'static [Need]) -> CaptureStep {
        CaptureStep {
            needs,
            ..cli_step(id, false, false)
        }
    }

    /// The checklist is what the operator sets the bench up from, so it lists
    /// each item once, in one fixed order, with the steps waiting on it.
    #[test]
    fn needs_checklist_lists_each_item_once_in_need_order() {
        let steps = [
            needy_step("temp", &[Need::Thermocouple]),
            needy_step("dcv_short", &[Need::ShortedLeads]),
            needy_step("tempf", &[Need::Thermocouple]),
            needy_step("acv", &[]),
            needy_step("ohm_short", &[Need::ShortedLeads]),
        ];
        let (table, needs) = render_needs(&steps).unwrap();
        assert_eq!(needs, vec![Need::ShortedLeads, Need::Thermocouple]);
        assert_eq!(
            table,
            "You will need:\n\
             \x20 [1] shorted test leads  (steps: dcv_short, ohm_short)\n\
             \x20 [2] K-type thermocouple  (steps: temp, tempf)\n"
        );
    }

    /// Nothing to gather, nothing to ask about: a run whose steps need only
    /// the meter opens straight into the first step.
    #[test]
    fn no_needs_shows_no_checklist() {
        let steps = [needy_step("acv", &[]), needy_step("hold", &[])];
        assert!(render_needs(&steps).is_none());
    }

    /// Saying "no thermocouple" drops the temperature steps and nothing else.
    #[test]
    fn deselecting_a_need_drops_exactly_its_steps() {
        let steps = [
            needy_step("dcv_short", &[Need::ShortedLeads]),
            needy_step("temp", &[Need::Thermocouple]),
            needy_step("tempf", &[Need::Thermocouple]),
            needy_step("acv", &[]),
        ];
        let dropped = steps_without(&steps, &[Need::Thermocouple]);
        assert_eq!(
            dropped
                .iter()
                .map(|(s, label)| (s.id, *label))
                .collect::<Vec<_>>(),
            vec![
                ("temp", "K-type thermocouple"),
                ("tempf", "K-type thermocouple"),
            ]
        );
        assert!(steps_without(&steps, &[]).is_empty());
    }

    /// `--steps` and `--unverified` narrow together: a verified step named on
    /// the command line still doesn't run under `--unverified`.
    #[test]
    fn steps_and_unverified_intersect() {
        let done = cli_step("dcv", true, false);
        let todo = cli_step("temp", false, false);
        let filter: Option<std::collections::HashSet<String>> = Some(
            ["dcv".to_string(), "temp".to_string()]
                .into_iter()
                .collect(),
        );

        assert!(step_selected(&done, &None, false));
        assert!(!step_selected(&done, &None, true));
        assert!(step_selected(&todo, &None, true));

        assert!(step_selected(&done, &filter, false));
        assert!(!step_selected(&done, &filter, true));
        assert!(step_selected(&todo, &filter, true));

        let other: Option<std::collections::HashSet<String>> =
            Some(["dcv".to_string()].into_iter().collect());
        assert!(!step_selected(&todo, &other, true));
    }
}
