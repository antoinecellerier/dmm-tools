//! The trust tier the gate rules on: what a run starts at, and how the
//! gate steps' outcome promotes it or keeps every step confirming.

use super::{CaptureReport, CoreSemantics, StepStatus, Tier};
use crate::capture::step::CaptureStep;
use console::style;
use dmm_shared::tr;

/// The gate steps that did not confirm the parser, or `None` while any of
/// them is still to run. An empty list is a gate that passed.
fn gate_failures(report: &CaptureReport, gate_ids: &[&str]) -> Option<Vec<String>> {
    let mut failed = Vec::new();
    for id in gate_ids {
        let step = report.steps.iter().find(|s| s.id == *id)?;
        if step.status != StepStatus::Captured || step.confirmed != Some(true) {
            failed.push((*id).to_string());
        }
    }
    Some(failed)
}

/// How much the run trusts the parser, and what the gate has decided about it.
pub(crate) struct Trust {
    pub(in crate::capture) tier: Tier,
    /// Every gate step the device declares, not just the selected ones: the
    /// gate is only decided once the whole block has reported.
    gate_ids: Vec<&'static str>,
    /// The gate has been ruled on, so a failed one says so once.
    decided: bool,
}

impl Trust {
    /// A family whose readings hardware has confirmed is trusted from the
    /// start; `--sniff` distrusts even a family that has been.
    pub(crate) fn new(sniff: bool, verified: bool, steps: &[CaptureStep]) -> Self {
        Trust {
            tier: match (sniff, verified) {
                (true, _) => Tier::Sniff,
                (false, true) => Tier::Trusted,
                (false, false) => Tier::Gate,
            },
            gate_ids: steps.iter().filter(|s| s.gate).map(|s| s.id).collect(),
            decided: false,
        }
    }

    /// What the step's detector may assume. Sniff assumes nothing: the step
    /// advances on the payload bytes changing, whatever the parse made of it.
    pub(in crate::capture) fn expect(
        &self,
        step: &CaptureStep,
    ) -> Option<dmm_lib::protocol::Expect> {
        self.reads_parse().then_some(step.expect).flatten()
    }

    /// Whether a step may judge the meter by the parse at all: not in Sniff.
    pub(in crate::capture) fn reads_parse(&self) -> bool {
        self.tier != Tier::Sniff
    }

    /// Whether the run may drive the meter's settings itself: only once the
    /// gate has shown that mode, range and flags read back correctly.
    pub(in crate::capture) fn drives(&self) -> bool {
        self.tier == Tier::Trusted
    }

    /// Whether the step is confirmed at the step itself. Gate steps always
    /// are — the rest of the run's confirmations rest on them.
    pub(in crate::capture) fn confirm_inline(&self, step: &CaptureStep) -> bool {
        step.gate || self.tier != Tier::Trusted
    }

    /// Rule on the gate once every one of its steps has reported, and promote
    /// the run if the parser got the core semantics right.
    pub(in crate::capture) fn update(&mut self, report: &mut CaptureReport) {
        if self.decided || self.tier != Tier::Gate || self.gate_ids.is_empty() {
            return;
        }
        let Some(failures) = gate_failures(report, &self.gate_ids) else {
            return;
        };
        self.decided = true;
        if failures.is_empty() {
            self.tier = Tier::Trusted;
            report.core_semantics = Some(CoreSemantics::Confirmed);
            eprintln!(
                "{}",
                style(tr!(crate::l10n::get(), "capture-core-confirmed")).green()
            );
        } else {
            report.core_semantics = Some(CoreSemantics::Failed);
            eprintln!(
                "{}",
                style(tr!(
                    crate::l10n::get(),
                    "capture-core-not-confirmed",
                    steps = failures.join(", "),
                ))
                .yellow()
            );
        }
        report.gate_failures = failures;
        report.tier = Some(self.tier);
    }
}
#[cfg(test)]
mod tests {
    use super::super::file::to_yaml;
    use super::*;
    use crate::capture::report::{StepResult, from_yaml, upsert_step};
    use crate::capture::step::cli_step;

    /// A report holding one result per gate step, as the run would have left
    /// it.
    fn gate_report(results: &[(&str, StepStatus, Option<bool>)]) -> CaptureReport {
        let mut report = CaptureReport::default();
        for (id, status, confirmed) in results {
            upsert_step(
                &mut report,
                StepResult {
                    confirmed: *confirmed,
                    ..StepResult::new(id, "do the thing", status.clone())
                },
            );
        }
        report
    }

    fn gate_steps() -> Vec<CaptureStep> {
        vec![
            cli_step("dcv", false, true),
            cli_step("ohm", false, true),
            cli_step("temp", false, false),
        ]
    }

    /// The gate is what says the parser reads digits, OL and sign correctly,
    /// so every one of its steps has to have been captured and confirmed.
    #[test]
    fn a_gate_passes_only_when_every_step_confirmed() {
        let ids = ["dcv", "ohm"];
        let passed = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Captured, Some(true)),
        ]);
        assert_eq!(gate_failures(&passed, &ids), Some(vec![]));

        let corrected = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Captured, Some(false)),
        ]);
        assert_eq!(
            gate_failures(&corrected, &ids),
            Some(vec!["ohm".to_string()])
        );

        let skipped = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Skipped, None),
        ]);
        assert_eq!(gate_failures(&skipped, &ids), Some(vec!["ohm".to_string()]));

        // Nothing to rule on until the whole block has reported.
        let half = gate_report(&[("dcv", StepStatus::Captured, Some(true))]);
        assert_eq!(gate_failures(&half, &ids), None);
    }

    /// A confirmed gate is what buys the rest of the run its deferred review.
    #[test]
    fn a_passed_gate_promotes_the_run() {
        let mut trust = Trust::new(false, false, &gate_steps());
        assert_eq!(trust.tier, Tier::Gate);
        assert!(trust.confirm_inline(&cli_step("temp", false, false)));

        let mut report = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Captured, Some(true)),
        ]);
        trust.update(&mut report);

        assert_eq!(trust.tier, Tier::Trusted);
        assert_eq!(report.tier, Some(Tier::Trusted));
        assert_eq!(report.core_semantics, Some(CoreSemantics::Confirmed));
        assert!(report.gate_failures.is_empty());
        assert!(!trust.confirm_inline(&cli_step("temp", false, false)));
        // Gate steps are confirmed at the step whatever the tier.
        assert!(trust.confirm_inline(&cli_step("dcv", false, true)));
    }

    /// A gate step the meter disagreed with leaves every later reading worth
    /// asking about.
    #[test]
    fn a_failed_gate_keeps_confirming_every_step() {
        let mut trust = Trust::new(false, false, &gate_steps());
        let mut report = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Captured, Some(false)),
        ]);
        trust.update(&mut report);

        assert_eq!(trust.tier, Tier::Gate);
        assert_eq!(report.core_semantics, Some(CoreSemantics::Failed));
        assert_eq!(report.gate_failures, vec!["ohm".to_string()]);
        assert!(trust.confirm_inline(&cli_step("temp", false, false)));
    }

    /// `--sniff` is for a parser nobody trusts, so a passed gate must not
    /// hand it the benefit of the doubt.
    #[test]
    fn sniff_never_promotes_and_never_reads_the_expectation() {
        let mut trust = Trust::new(true, true, &gate_steps());
        assert_eq!(trust.tier, Tier::Sniff);

        let mut report = gate_report(&[
            ("dcv", StepStatus::Captured, Some(true)),
            ("ohm", StepStatus::Captured, Some(true)),
        ]);
        trust.update(&mut report);

        assert_eq!(trust.tier, Tier::Sniff);
        assert_eq!(report.core_semantics, None);
        assert!(trust.confirm_inline(&cli_step("temp", false, false)));

        let mut step = cli_step("dcv", false, true);
        step.expect = Some(dmm_lib::protocol::Expect::mode("DC V"));
        assert!(trust.expect(&step).is_none(), "sniff detects on raw bytes");
    }

    /// Hardware has already confirmed a Verified family's readings, so its
    /// run starts where a passed gate would leave it.
    #[test]
    fn a_verified_family_starts_trusted() {
        let trust = Trust::new(false, true, &gate_steps());
        assert_eq!(trust.tier, Tier::Trusted);
        assert!(!trust.confirm_inline(&cli_step("temp", false, false)));

        let mut step = cli_step("dcv", false, true);
        step.expect = Some(dmm_lib::protocol::Expect::mode("DC V"));
        assert!(trust.expect(&step).is_some());
    }

    /// Resuming an interrupted capture reloads the report, so one written
    /// before the trust tiers existed must still parse.
    #[test]
    fn reports_without_the_trust_fields_still_load() {
        let yaml = "date: '2026-01-01'\ntool_version: test\ndevice_name: UT61E+\n\
                    supported: true\nsteps:\n- id: dcv\n  instruction: test\n  \
                    status: captured\n";
        let report: CaptureReport = from_yaml(yaml).unwrap();
        assert_eq!(report.tier, None);
        assert_eq!(report.core_semantics, None);
        assert!(report.gate_failures.is_empty());
    }

    #[test]
    fn the_tier_and_gate_outcome_serialize_in_lowercase() {
        let report = CaptureReport {
            tier: Some(Tier::Trusted),
            core_semantics: Some(CoreSemantics::Failed),
            gate_failures: vec!["ohm".to_string()],
            ..CaptureReport::default()
        };
        let yaml = to_yaml(&report).unwrap();
        assert!(yaml.contains("tier: trusted"), "{yaml}");
        assert!(yaml.contains("core_semantics: failed"), "{yaml}");
        let parsed: CaptureReport = from_yaml(&yaml).unwrap();
        assert_eq!(parsed.tier, Some(Tier::Trusted));
        assert_eq!(parsed.gate_failures, vec!["ohm".to_string()]);
    }
}
