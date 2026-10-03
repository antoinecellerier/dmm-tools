//! `--fixtures`: each distinct sample payload as a golden fixture, in the
//! shape `crates/dmm-lib/tests/golden/` holds, printed for a maintainer to
//! pick from. Nothing is written: a fixture needs a first line saying what
//! its frame shows, which only someone who read the step can write.

use super::Triage;
use crate::capture::report::SampleData;
use dmm_lib::flags::{Flag, StatusFlags};

pub(super) fn render(triage: &Triage) -> String {
    let mut out = String::from("\nFixtures\n");
    let Some(device) = triage.device else {
        out.push_str("  none: no device to parse with\n");
        return out;
    };
    let report = triage.report;
    let date = report.date.get(..10).unwrap_or(&report.date);
    let link = report.transport_name.as_deref().unwrap_or("its cable");
    // "0.8.0-dev (eea85ba)" as the fixtures cite it: "0.8.0-dev eea85ba".
    let version = report.tool_version.replace(['(', ')'], "");
    // A family that sends sub-values gets `aux: []` on a reading without
    // them, so the fixture still asserts there are none.
    let with_aux = triage
        .steps
        .iter()
        .flat_map(|s| &s.parsed)
        .any(|p| p.as_ref().is_ok_and(|m| !m.aux_values.is_empty()));
    for step in &triage.steps {
        let mut seen: Vec<String> = Vec::new();
        for (sample, parsed) in step.result.samples.iter().zip(&step.parsed) {
            if parsed.is_err() || seen.contains(&sample.raw_hex) {
                continue;
            }
            seen.push(sample.raw_hex.clone());
            let name = match seen.len() {
                1 => file_name(step.id()),
                n => format!("{}_{n}", file_name(step.id())),
            };
            let Ok(m) = parsed else { continue };
            out.push_str(&format!("--- {}/{name}.yaml\n", device.id));
            out.push_str("# TODO: what this frame shows.\n");
            out.push_str(&format!(
                "# Captured {date} from a {} over {link} (issue #TODO, {version} report, step {}).\n",
                device.display_name,
                step.id()
            ));
            out.push_str(&fixture(&SampleData::from_measurement(m), with_aux));
        }
    }
    out
}

/// A step id as a file name: `dcv/range:22V` is `dcv_range_22V`.
fn file_name(id: &str) -> String {
    id.chars()
        .map(|c| if matches!(c, '/' | ':' | ' ') { '_' } else { c })
        .collect()
}

/// The fixture's fields, the flags that are set and the sub-values, listed
/// even when there are none if `with_aux`.
fn fixture(s: &SampleData, with_aux: bool) -> String {
    let mut out = format!(
        "raw_hex: {:?}\nmode: {:?}\nvalue: {:?}\nunit: {:?}\nrange_label: {:?}\n",
        s.raw_hex, s.mode, s.value, s.unit, s.range_label
    );
    let flags = StatusFlags::from(&s.flags);
    let set: Vec<&str> = flags.active().map(Flag::name).collect();
    if set.is_empty() {
        out.push_str("flags: {}\n");
    } else {
        out.push_str("flags:\n");
        for name in set {
            out.push_str(&format!("  {name}: true\n"));
        }
    }
    if s.aux.is_empty() {
        if with_aux {
            out.push_str("aux: []\n");
        }
    } else {
        out.push_str("aux:\n");
        for a in &s.aux {
            out.push_str(&format!(
                "  - label: {:?}\n    value: {:?}\n    unit: {:?}\n",
                a.label, a.value, a.unit
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::tests::{dcv, step, ut61eplus_report};
    use super::*;
    use dmm_lib::protocol::registry::find_device;

    /// One fixture per distinct payload, numbered within the step.
    #[test]
    fn each_distinct_sample_is_a_fixture() {
        let mut report = ut61eplus_report(vec![step(
            "dcv/range:2.2V",
            vec![dcv(b" 0.0001"), dcv(b" 0.0001"), dcv(b" 0.0002")],
        )]);
        report.date = "2026-10-03T06:03:49+02:00".to_string();
        let triage = Triage::new(&report, find_device("ut61eplus"), None);
        let text = render(&triage);
        assert_eq!(text.matches("--- ").count(), 2, "{text}");
        assert!(
            text.contains(
                "--- ut61eplus/dcv_range_2.2V_2.yaml\n# TODO: what this frame shows.\n\
                 # Captured 2026-10-03 from a UT61E+ over CP2110 (issue #TODO, 0.8.0-dev test report, step dcv/range:2.2V).\n\
                 raw_hex: \"02 31 20 30 2E 30 30 30 32 00 00 30 30 30\"\n\
                 mode: \"DC V\"\nvalue: \"0.0002\"\nunit: \"V\"\nrange_label: \"22V\"\n\
                 flags:\n  auto_range: true\n"
            ),
            "{text}"
        );
    }
}
