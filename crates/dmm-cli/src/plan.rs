//! A step list a maintainer writes for one investigation, read from a YAML
//! file.
//!
//! The shipped steps cover what a family is known to do; an edge case found in
//! an issue does not wait for a release. The maintainer pastes a few steps into
//! the thread and the reporter runs them with `capture --plan`.

use crate::capture::{CaptureStep, FREEFORM_STEP_ID};
use dmm_lib::flags::Flag;
use dmm_lib::protocol::{Expect, Need, RangeExpect, ValueExpect};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

/// Samples a step takes when it doesn't say, matching `CaptureStep::basic`.
const DEFAULT_SAMPLES: usize = 5;

/// A plan file: nothing but the steps to run, in order.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    steps: Vec<PlanStep>,
}

/// One step, with the fields a protocol's own `CaptureStep` carries that a
/// plan may set. `verified` and `gate` are not among them: a plan step has no
/// hardware evidence behind it and nothing else in the run rests on it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanStep {
    id: String,
    instruction: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    samples: Option<usize>,
    #[serde(default)]
    expect: Option<PlanExpect>,
    #[serde(default)]
    needs: Vec<String>,
}

/// What a correctly parsed reading looks like, in the names the report and
/// `--list-steps` already use.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanExpect {
    #[serde(default)]
    mode: Option<String>,
    /// Flag name to the value it must hold; a flag left out is not checked.
    #[serde(default)]
    flags: BTreeMap<String, bool>,
    #[serde(default)]
    range: Option<String>,
    #[serde(default)]
    value: Option<String>,
    /// Magnitude a numeric reading must reach, sign aside.
    #[serde(default)]
    at_least: Option<f64>,
}

/// The steps in `path`, ready for the capture loop.
pub(crate) fn load(path: &str) -> Result<Vec<CaptureStep>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(parse(path, &text)?)
}

fn parse(path: &str, yaml: &str) -> Result<Vec<CaptureStep>, String> {
    let plan: Plan = serde_yaml_ng::from_str(yaml).map_err(|e| format!("{path}: {e}"))?;
    if plan.steps.is_empty() {
        return Err(format!("{path}: the plan lists no steps"));
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        if step.id == FREEFORM_STEP_ID {
            return Err(format!(
                "{path}: step id {FREEFORM_STEP_ID:?} is reserved for the freeform pass"
            ));
        }
        if !seen.insert(&step.id) {
            return Err(problem(path, &step.id, "listed twice"));
        }
        out.push(step.to_capture_step(path)?);
    }
    Ok(out)
}

/// How every plan error names where it came from.
fn problem(path: &str, id: &str, what: impl std::fmt::Display) -> String {
    format!("{path}: step {id:?}: {what}")
}

impl PlanStep {
    fn to_capture_step(&self, path: &str) -> Result<CaptureStep, String> {
        let mut needs = Vec::with_capacity(self.needs.len());
        for name in &self.needs {
            needs.push(need(name).ok_or_else(|| {
                problem(
                    path,
                    &self.id,
                    format!("unknown need {name:?} (valid: {})", need_names()),
                )
            })?);
        }
        Ok(CaptureStep {
            id: leak_str(&self.id),
            instruction: leak_str(&self.instruction),
            command: self.command.as_deref().map(leak_str),
            samples: self.samples.unwrap_or(DEFAULT_SAMPLES),
            expect: self
                .expect
                .as_ref()
                .map(|e| e.to_expect(path, &self.id))
                .transpose()?,
            verified: false,
            gate: false,
            needs: leak_slice(needs),
        })
    }
}

impl PlanExpect {
    fn to_expect(&self, path: &str, id: &str) -> Result<Expect, String> {
        let mut out = match &self.mode {
            Some(mode) => Expect::mode(leak_str(mode)),
            None => Expect::new(),
        };
        if !self.flags.is_empty() {
            let mut pairs = Vec::with_capacity(self.flags.len());
            for (name, &want) in &self.flags {
                let flag = flag(name).ok_or_else(|| {
                    problem(
                        path,
                        id,
                        format!("unknown flag {name:?} (valid: {})", flag_names()),
                    )
                })?;
                pairs.push((flag, want));
            }
            out = out.flags(leak_slice(pairs));
        }
        if let Some(range) = &self.range {
            out = out.range(match range.as_str() {
                "auto" => RangeExpect::Auto,
                "manual" => RangeExpect::Manual,
                other => {
                    return Err(problem(
                        path,
                        id,
                        format!("unknown range {other:?} (valid: auto, manual)"),
                    ));
                }
            });
        }
        if let Some(value) = &self.value {
            out = out.value(match value.as_str() {
                "overload" => ValueExpect::Overload,
                "negative" => ValueExpect::Negative,
                "finite" => ValueExpect::Finite,
                "ncv" => ValueExpect::NcvDetected,
                other => {
                    return Err(problem(
                        path,
                        id,
                        format!("unknown value {other:?} (valid: overload, negative, finite, ncv)"),
                    ));
                }
            });
        }
        if let Some(min) = self.at_least {
            out = out.at_least(min);
        }
        Ok(out)
    }
}

/// What a plan calls this need: the variant name in snake_case. A `match`, so
/// a new [`Need`] has to be named here rather than silently be unwritable.
fn need_name(need: Need) -> &'static str {
    match need {
        Need::ShortedLeads => "shorted_leads",
        Need::DcSource => "dc_source",
        Need::Thermocouple => "thermocouple",
        Need::LiveWire => "live_wire",
        Need::Transistor => "transistor",
        Need::Scr => "scr",
    }
}

fn need(name: &str) -> Option<Need> {
    Need::ALL.into_iter().find(|n| need_name(*n) == name)
}

fn flag(name: &str) -> Option<Flag> {
    Flag::ALL.into_iter().find(|f| f.name() == name)
}

/// The spellings a rejected name is listed against.
fn need_names() -> String {
    join(Need::ALL.into_iter().map(need_name))
}

fn flag_names() -> String {
    join(Flag::ALL.into_iter().map(Flag::name))
}

fn join<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.collect::<Vec<_>>().join(", ")
}

/// The plan's own strings outlive the run: `CaptureStep` holds `&'static`
/// data, and a plan is loaded once per process, so the leak is bounded by the
/// size of the file.
fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// As `leak_str`, for the flag and need lists a step carries.
fn leak_slice<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan a maintainer would paste into an issue.
    const EDGE: &str = "\
steps:
  - id: dcv_hold
    instruction: Set the meter to DC V, then press HOLD
    command: hold
    samples: 3
    expect:
      mode: DC V
      flags:
        hold: true
      value: finite
    needs:
      - dc_source
  - id: dcv_open
    instruction: Leave the probes open on DC V
";

    fn err(yaml: &str) -> String {
        parse("edge.yaml", yaml)
            .err()
            .expect("the plan should have been rejected")
    }

    #[test]
    fn minimal_plan_parses() {
        let steps = parse("edge.yaml", EDGE).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id, "dcv_hold");
        assert_eq!(steps[0].command, Some("hold"));
        assert_eq!(steps[0].samples, 3);
        assert_eq!(steps[0].needs, [Need::DcSource]);
        assert!(!steps[0].gate, "a plan step never gates the run");
        // Left out, so the defaults stand.
        assert_eq!(steps[1].samples, DEFAULT_SAMPLES);
        assert_eq!(steps[1].command, None);
        assert!(steps[1].expect.is_none());
    }

    /// A typo must stop the run rather than quietly drop the field the
    /// maintainer wrote the plan for.
    #[test]
    fn unknown_key_is_rejected() {
        let e = err("steps:\n  - id: a\n    instruction: b\n    sampls: 2\n");
        assert!(e.starts_with("edge.yaml: "), "{e}");
        assert!(e.contains("sampls"), "{e}");
    }

    #[test]
    fn unknown_flag_names_the_valid_ones() {
        let e = err(
            "steps:\n  - id: a\n    instruction: b\n    expect:\n      flags:\n        holdd: true\n",
        );
        assert_eq!(
            e,
            format!(
                "edge.yaml: step \"a\": unknown flag \"holdd\" (valid: {})",
                flag_names()
            )
        );
        assert!(e.contains("hold, rel, auto_range"), "{e}");
    }

    #[test]
    fn unknown_need_names_the_valid_ones() {
        let e = err("steps:\n  - id: a\n    instruction: b\n    needs: [battery]\n");
        assert!(e.contains("unknown need \"battery\""), "{e}");
        assert!(e.contains("dc_source"), "{e}");
    }

    /// Two steps with one id would overwrite each other in the report.
    #[test]
    fn duplicate_id_is_rejected() {
        let e = err("steps:\n  - id: a\n    instruction: b\n  - id: a\n    instruction: c\n");
        assert_eq!(e, "edge.yaml: step \"a\": listed twice");
    }

    #[test]
    fn freeform_id_is_reserved() {
        let e = err("steps:\n  - id: extra\n    instruction: b\n");
        assert!(e.contains("reserved for the freeform pass"), "{e}");
    }

    #[test]
    fn range_and_value_words_convert() {
        let yaml = "steps:\n  - id: a\n    instruction: b\n    expect:\n      range: manual\n      value: overload\n";
        let expect = parse("edge.yaml", yaml).unwrap()[0].expect.unwrap();
        assert_eq!(expect.range, Some(RangeExpect::Manual));
        assert_eq!(expect.value, Some(ValueExpect::Overload));

        let yaml = "steps:\n  - id: a\n    instruction: b\n    expect:\n      value: ncv\n";
        let expect = parse("edge.yaml", yaml).unwrap()[0].expect.unwrap();
        assert_eq!(expect.value, Some(ValueExpect::NcvDetected));

        let yaml = "steps:\n  - id: a\n    instruction: b\n    expect:\n      value: negative\n      at_least: 1.0\n";
        let expect = parse("edge.yaml", yaml).unwrap()[0].expect.unwrap();
        assert_eq!(expect.at_least, Some(1.0));

        let e = err("steps:\n  - id: a\n    instruction: b\n    expect:\n      range: fixed\n");
        assert_eq!(
            e,
            "edge.yaml: step \"a\": unknown range \"fixed\" (valid: auto, manual)"
        );
    }

    #[test]
    fn empty_plan_is_rejected() {
        assert_eq!(err("steps: []\n"), "edge.yaml: the plan lists no steps");
    }

    /// The point of the conversion: what the maintainer wrote has to decide
    /// whether a reading off the meter satisfies the step.
    #[test]
    fn expect_checks_a_reading() {
        use dmm_lib::protocol::ut61eplus::make_test_measurement;

        let steps = parse("edge.yaml", EDGE).unwrap();
        let expect = steps[0].expect.unwrap();
        // DC V, autorange, HOLD set (flag1 bit 1).
        let held = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0x02, 0, 0));
        assert_eq!(expect.check(&held), Ok(()));

        let running = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        assert_eq!(
            expect.check(&running),
            Err("hold is off, want on".to_string())
        );
    }
}
