//! What `--list-steps` prints for a device, and the check that `--steps`
//! only names steps that device declares.

use super::step::FREEFORM_STEP_ID;
use crate::StepListFormat;
use console::style;
use dmm_lib::protocol::registry::SelectableDevice;

/// Print the step IDs `--steps` accepts for the selected device.
///
/// The steps come from the device's own protocol, so what this prints is
/// exactly what will run. It used to list a table held in this file, which no
/// device had used since protocols started declaring their own steps: the IDs
/// shown matched nothing, so `--steps` filtered everything out and wrote an
/// empty report.
pub(crate) fn list_steps(device: &'static SelectableDevice, format: StepListFormat) {
    let protocol = (device.new_protocol)();
    let steps = protocol.capture_steps();
    match format {
        StepListFormat::Text => eprint!("{}", render_step_list(device, &steps)),
        // stdout: the checklist is meant to be piped or pasted into the issue.
        StepListFormat::Md => print!("{}", render_step_checklist(device, &steps)),
    }
}

/// The one-line ask a reporter runs to cover what hardware has not confirmed.
fn unverified_ask(device: &'static SelectableDevice) -> String {
    format!("dmm-cli --device {} capture --unverified", device.id)
}

/// How many of `steps` still lack hardware evidence.
fn unverified_count(steps: &[dmm_lib::protocol::CaptureStep]) -> usize {
    steps.iter().filter(|s| !s.verified).count()
}

/// The human listing: a mark per step, so reporter and maintainer read the
/// same verification state, and the ask that covers what is left.
fn render_step_list(
    device: &'static SelectableDevice,
    steps: &[dmm_lib::protocol::CaptureStep],
) -> String {
    let mut out = format!(
        "{} {}\n\n",
        style("Available capture steps for").bold(),
        style(device.display_name).bold().cyan()
    );
    if steps.is_empty() {
        out.push_str("  This device declares no capture steps.\n");
    }
    for s in steps {
        let mark = if s.verified {
            style("\u{2713}").green()
        } else {
            style("\u{b7}").dim()
        };
        let gate = if s.gate { "gate" } else { "" };
        out.push_str(&format!(
            "  {mark} {:<16} {gate:<5} {}\n",
            style(s.id).bold(),
            s.instruction
        ));
    }
    out.push_str(&format!("\n{}\n", style("  Always available:").cyan()));
    out.push_str(&format!(
        "    {:<16} Freeform captures — describe any mode not covered above\n\n",
        style(FREEFORM_STEP_ID).bold()
    ));
    out.push_str(&format!(
        "Usage: {} {}\n\n",
        style("dmm-cli capture --steps").dim(),
        style("dcmv,temp,duty").dim()
    ));
    let unverified = unverified_count(steps);
    out.push_str(&format!(
        "{unverified} of {} steps still unverified.\n",
        steps.len()
    ));
    if unverified > 0 {
        out.push_str(&format!(
            "Run only those with: {}\n",
            unverified_ask(device)
        ));
    }
    out
}

/// The checklist the device verification issues carry, generated so the issue
/// and the step list cannot drift. No colour: it is pasted into GitHub.
fn render_step_checklist(
    device: &'static SelectableDevice,
    steps: &[dmm_lib::protocol::CaptureStep],
) -> String {
    let mut out = String::from("## What needs verification\n\n");
    for s in steps {
        let mark = if s.verified { 'x' } else { ' ' };
        out.push_str(&format!("- [{mark}] `{}` — {}\n", s.id, s.instruction));
    }
    out.push_str(&format!("\n```bash\n{}\n```\n", unverified_ask(device)));
    out
}

/// Reject `--steps` IDs that no step will match.
///
/// Without this an unknown ID silently filtered every step out, leaving a
/// report with `steps: []` that the CLI still announced as "Capture
/// complete!" — so the user attached an empty file to their bug report.
pub(super) fn validate_step_filter(
    step_filter: &Option<std::collections::HashSet<String>>,
    steps: &[dmm_lib::protocol::CaptureStep],
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(filter) = step_filter else {
        return Ok(());
    };
    let known: std::collections::HashSet<&str> = steps
        .iter()
        .map(|s| s.id)
        .chain(std::iter::once(FREEFORM_STEP_ID))
        .collect();
    let mut unknown: Vec<&str> = filter
        .iter()
        .map(String::as_str)
        .filter(|id| !known.contains(id))
        .collect();
    if unknown.is_empty() {
        return Ok(());
    }
    unknown.sort_unstable();
    Err(format!(
        "unknown capture step(s): {}\nRun `dmm-cli capture --list-steps` to see the steps this device supports.",
        unknown.join(", ")
    )
    .into())
}

/// Returns true if the given step ID is included by the filter (or if there is no filter).
pub(super) fn step_included(
    step_filter: &Option<std::collections::HashSet<String>>,
    id: &str,
) -> bool {
    step_filter.as_ref().is_none_or(|f| f.contains(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    // `capture_steps` is a Protocol method; the trait has to be in scope to
    // call it on a concrete protocol type (not needed for `dyn Protocol`).
    use dmm_lib::protocol::Protocol;

    #[test]
    fn unknown_step_ids_are_rejected() {
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
        let filter: Option<std::collections::HashSet<String>> = Some(
            ["dcv".to_string(), "range_cycle".to_string()]
                .into_iter()
                .collect(),
        );
        let err = validate_step_filter(&filter, &steps)
            .unwrap_err()
            .to_string();
        assert!(err.contains("range_cycle"), "got {err}");
        assert!(
            !err.contains("dcv"),
            "known IDs must not be reported: {err}"
        );
    }

    #[test]
    fn known_step_ids_and_the_freeform_keyword_pass() {
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
        let filter: Option<std::collections::HashSet<String>> = Some(
            [
                "dcv".to_string(),
                "range".to_string(),
                FREEFORM_STEP_ID.to_string(),
            ]
            .into_iter()
            .collect(),
        );
        assert!(validate_step_filter(&filter, &steps).is_ok());
    }

    fn lib_step(id: &'static str, verified: bool, gate: bool) -> dmm_lib::protocol::CaptureStep {
        let step = dmm_lib::protocol::CaptureStep::basic(id, "do the thing");
        let step = if verified { step.verified() } else { step };
        if gate { step.gate() } else { step }
    }

    fn ut61eplus() -> &'static SelectableDevice {
        dmm_lib::protocol::registry::find_device("ut61eplus").expect("registry has the UT61E+")
    }

    /// The issue checklist is generated, so its shape is asserted line by line.
    #[test]
    fn markdown_listing_is_the_issue_checklist() {
        let steps = vec![lib_step("dcv", true, true), lib_step("temp", false, false)];
        assert_eq!(
            render_step_checklist(ut61eplus(), &steps),
            "## What needs verification\n\
             \n\
             - [x] `dcv` — do the thing\n\
             - [ ] `temp` — do the thing\n\
             \n\
             ```bash\n\
             dmm-cli --device ut61eplus capture --unverified\n\
             ```\n"
        );
    }

    #[test]
    fn text_listing_marks_each_step_and_asks_for_what_is_left() {
        let steps = vec![lib_step("dcv", true, true), lib_step("temp", false, false)];
        let out = console::strip_ansi_codes(&render_step_list(ut61eplus(), &steps)).into_owned();
        assert!(out.contains("\u{2713} dcv"), "{out}");
        assert!(out.contains("\u{b7} temp"), "{out}");
        assert!(out.contains("gate"), "{out}");
        assert!(out.contains("1 of 2 steps still unverified."), "{out}");
        assert!(
            out.contains("dmm-cli --device ut61eplus capture --unverified"),
            "{out}"
        );
    }

    /// A fully verified list has nothing to ask for.
    #[test]
    fn text_listing_omits_the_ask_when_nothing_is_unverified() {
        let steps = vec![lib_step("dcv", true, false)];
        let out = console::strip_ansi_codes(&render_step_list(ut61eplus(), &steps)).into_owned();
        assert!(out.contains("0 of 1 steps still unverified."), "{out}");
        assert!(!out.contains("--unverified\n"), "{out}");
    }

    /// The real list is the one reporters see: the UT61E+ is verified hardware,
    /// so `--unverified` must leave it something smaller than the whole run.
    #[test]
    fn unverified_count_is_a_subset_of_the_ut61eplus_steps() {
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
        let unverified = unverified_count(&steps);
        assert!(unverified < steps.len(), "{unverified} of {}", steps.len());
    }

    #[test]
    fn no_filter_accepts_everything() {
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
        assert!(validate_step_filter(&None, &steps).is_ok());
    }
}
