//! `dmm-cli triage`: what a maintainer reads off a capture report a reporter
//! attached, worked out with this build's parser instead of a throwaway
//! script that re-implements it.
//!
//! The output is plain text for a terminal, a file or an issue: a header,
//! findings one per line tagged by kind, each step's distinct readings and
//! the run's stats. Nothing in it carries a time, so two reports of the
//! same steps diff line by line.

mod load;

use super::report::{CaptureReport, SampleData, StepResult, StepStatus, hex_bytes};
use super::watch::{State, enter_only};
use crate::cli::TriageArgs;
use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{CaptureStep, Protocol, capture_reports};

pub(crate) fn cmd_triage(
    args: TriageArgs,
    named_device: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = load::load(&args.report)?;
    let device = load::device(named_device, &report)?;
    let plan = args.plan.as_deref().map(super::plan::load).transpose()?;
    let triage = Triage::new(&report, device, plan);
    print!("{}", load::scrub_addresses(&triage.render()));
    Ok(())
}

/// A report read through one build's parser.
struct Triage<'a> {
    report: &'a CaptureReport,
    device: Option<&'static SelectableDevice>,
    /// The step definitions the report's steps are judged against: the
    /// plan's, or the device's own in this build. `None` for a plan run
    /// whose plan was not given, whose ids may mean other steps here.
    definitions: Option<Vec<CaptureStep>>,
    steps: Vec<StepView<'a>>,
}

/// One step of the report, its samples parsed again.
struct StepView<'a> {
    result: &'a StepResult,
    /// Each sample as this build parses its payload; empty with no device.
    parsed: Vec<Result<Measurement, String>>,
    /// What the parser called unrecognised while parsing them.
    unrecognised: Vec<String>,
}

impl StepView<'_> {
    fn id(&self) -> &str {
        &self.result.id
    }

    /// A sub-step the tool drove (`dcv/range:6V`).
    fn is_sub_step(&self) -> bool {
        self.result.id.contains('/')
    }

    fn captured(&self) -> bool {
        self.result.status == StepStatus::Captured
    }

    /// The step's recorded clock span, first frame to last.
    fn clock(&self) -> Option<(u64, u64)> {
        let first = self.result.frames.first()?.at_ms;
        let last = self.result.frames.last()?.at_ms;
        Some((first, last))
    }

    /// Each sample as this build reads it, falling back to what the report
    /// recorded where it no longer parses or no device was given.
    fn readings(&self) -> Vec<SampleData> {
        self.result
            .samples
            .iter()
            .enumerate()
            .map(|(i, recorded)| match self.parsed.get(i) {
                Some(Ok(m)) => SampleData::from_measurement(m),
                _ => recorded.clone(),
            })
            .collect()
    }
}

impl<'a> Triage<'a> {
    fn new(
        report: &'a CaptureReport,
        device: Option<&'static SelectableDevice>,
        plan: Option<Vec<CaptureStep>>,
    ) -> Self {
        let protocol = device.map(|d| (d.new_protocol)());
        let definitions = match (plan, &report.plan, &protocol) {
            (Some(plan), _, _) => Some(plan),
            (None, Some(_), _) => None,
            (None, None, protocol) => protocol.as_ref().map(|p| p.capture_steps()),
        };
        let steps = report
            .steps
            .iter()
            .map(|result| parse_step(result, protocol.as_deref()))
            .collect();
        Triage {
            report,
            device,
            definitions,
            steps,
        }
    }

    /// The definition a step ran from: a sub-step's is its base step's.
    fn definition(&self, id: &str) -> Option<&CaptureStep> {
        let base = id.split('/').next().unwrap_or(id);
        self.definitions.as_ref()?.iter().find(|s| s.id == base)
    }

    fn render(&self) -> String {
        let mut out = String::new();
        self.header(&mut out);
        out.push_str("\nFindings\n");
        let findings = self.findings();
        if findings.is_empty() {
            out.push_str("  none\n");
        }
        for line in findings {
            out.push_str(&format!("  {line}\n"));
        }
        let reparsed: Vec<String> = self.steps.iter().flat_map(reparse_findings).collect();
        if !reparsed.is_empty() {
            out.push_str("\nRead differently by this build\n");
            for line in reparsed {
                out.push_str(&format!("  {line}\n"));
            }
        }
        out.push_str("\nSteps\n");
        self.summary(&mut out);
        out.push_str("\nStats\n");
        self.stats(&mut out);
        out
    }

    fn header(&self, out: &mut String) {
        let r = self.report;
        out.push_str(&format!("Tool: {}, run {}\n", r.tool_version, r.date));
        let transport = r.transport_name.as_deref().unwrap_or("unknown link");
        match self.device {
            Some(d) => out.push_str(&format!(
                "Device: {} ({}) over {transport}; the meter named itself {:?}\n",
                d.id, d.display_name, r.device_name
            )),
            None => out.push_str(&format!(
                "Device: none named, so nothing is parsed again (pass --device); \
                 over {transport}, the meter named itself {:?}\n",
                r.device_name
            )),
        }

        let mut run = Vec::new();
        if let Some(tier) = r.tier {
            run.push(format!("tier {}", format!("{tier:?}").to_lowercase()));
        }
        if let Some(core) = r.core_semantics {
            run.push(format!(
                "core semantics {}",
                format!("{core:?}").to_lowercase()
            ));
        }
        if !r.gate_failures.is_empty() {
            run.push(format!("gate failures: {}", r.gate_failures.join(", ")));
        }
        if let Some(drive) = r.drive {
            run.push(format!("drive {}", format!("{drive:?}").to_lowercase()));
        }
        if r.unverified_only {
            run.push("--unverified".to_string());
        }
        if let Some(plan) = &r.plan {
            run.push(format!("plan {plan}"));
        }
        if r.no_response {
            run.push("the meter never answered".to_string());
        }
        if !run.is_empty() {
            out.push_str(&format!("Run: {}\n", run.join("; ")));
        }
        if r.plan.is_some() && self.definitions.is_none() {
            out.push_str("Steps: a plan run, so no expectations; pass --plan with its file\n");
        }

        let count = |status: StepStatus| r.steps.iter().filter(|s| s.status == status).count();
        let sub_steps = self.steps.iter().filter(|s| s.is_sub_step()).count();
        out.push_str(&format!(
            "Steps: {} ({} captured, {} error, {} timeout, {} skipped), {sub_steps} of them sub-steps\n",
            r.steps.len(),
            count(StepStatus::Captured),
            count(StepStatus::Error),
            count(StepStatus::Timeout),
            count(StepStatus::Skipped),
        ));
        if let Some(d) = &r.detection {
            out.push_str(&format!("Detection: {}\n", detection_line(d)));
        }

        // A resumed run's clock starts again at zero, and a retaken step
        // keeps its place in the file with the new run's clock.
        let resets = self
            .steps
            .iter()
            .filter_map(StepView::clock)
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[1].0 < w[0].1)
            .count();
        if resets > 0 {
            out.push_str(&format!(
                "Clock: goes back {resets} time(s) in file order: a resumed run or a retaken step\n"
            ));
        }

        let mut truncated = Vec::new();
        if r.wire_events_dropped > 0 {
            truncated.push(format!(
                "{} wire events dropped by the session bound",
                r.wire_events_dropped
            ));
        }
        let dropped: Vec<String> = r
            .steps
            .iter()
            .filter(|s| s.frames_dropped > 0)
            .map(|s| format!("{} {}", s.id, s.frames_dropped))
            .collect();
        if !dropped.is_empty() {
            truncated.push(format!(
                "oldest frames trimmed per step: {}",
                dropped.join(", ")
            ));
        }
        if !truncated.is_empty() {
            out.push_str(&format!("Truncated: {}\n", truncated.join("; ")));
        }

        let diagnostics = tally(r.steps.iter().flat_map(|s| s.diagnostics.iter().cloned()));
        for (text, n) in diagnostics {
            out.push_str(&format!("Diagnostic: {n}\u{d7} {text}\n"));
        }
    }

    fn findings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for step in &self.steps {
            let r = step.result;
            match r.status {
                StepStatus::Error | StepStatus::Timeout => out.push(format!(
                    "[{}] {}: {}",
                    if r.status == StepStatus::Error {
                        "error"
                    } else {
                        "timeout"
                    },
                    r.id,
                    r.error.as_deref().unwrap_or("no reason recorded")
                )),
                _ if r.needs_attention => {
                    out.push(format!("[attention] {}: flagged by the capture", r.id))
                }
                _ => {}
            }
        }
        for step in &self.steps {
            if step.result.confirmed == Some(false) {
                out.push(lcd_finding(step));
            }
        }
        out.extend(self.stale_starts());
        if let Some(line) = self.reparse_summary() {
            out.push(line);
        }
        for step in &self.steps {
            for (text, n) in tally(step.unrecognised.iter().cloned()) {
                out.push(format!("[unrecognised] {}: {n}\u{d7} {text}", step.id()));
            }
        }
        out
    }

    /// One line for every sample this build reads differently, naming the
    /// fields: a report from before a parser fix differs on most steps, and
    /// the details have a section of their own.
    fn reparse_summary(&self) -> Option<String> {
        let mut steps = 0;
        let mut fields: Vec<(&str, usize)> = Vec::new();
        for step in &self.steps {
            let mut differs = false;
            for (recorded, parsed) in step.result.samples.iter().zip(&step.parsed) {
                let names = match parsed {
                    Ok(m) => differences(recorded, &SampleData::from_measurement(m))
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect(),
                    Err(_) => vec!["no longer parses"],
                };
                differs |= !names.is_empty();
                for name in names {
                    match fields.iter_mut().find(|(f, _)| *f == name) {
                        Some((_, n)) => *n += 1,
                        None => fields.push((name, 1)),
                    }
                }
            }
            steps += usize::from(differs);
        }
        (steps > 0).then(|| {
            let fields: Vec<String> = fields
                .iter()
                .map(|(f, n)| format!("{f} in {n} samples"))
                .collect();
            format!(
                "[reparse] {steps} steps read differently by this build: {}",
                fields.join(", ")
            )
        })
    }

    /// Steps whose first sample shows the state the step before them ended
    /// in, as if the operator's action never reached the meter: the UT804's
    /// `rel` filed what `max_min` left (issue #16).
    ///
    /// "Before" is the previous step the operator ran: sub-steps are the
    /// tool's, and it puts the meter back where their base step was. Steps
    /// the tool commanded are judged on their command, and a step at the
    /// dial position the previous one ended at shows nothing arriving
    /// whatever was done ([`enter_only`]).
    fn stale_starts(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut previous: Option<&StepView> = None;
        for step in self.steps.iter().filter(|s| !s.is_sub_step()) {
            let prev = previous.replace(step);
            let Some(prev) = prev.filter(|p| p.captured()) else {
                continue;
            };
            // Without its definition, a key step or a step at the previous
            // one's dial position cannot be told apart: a plan run triaged
            // without its plan, a freeform step, a step this build dropped.
            let Some(definition) = self.definition(step.id()) else {
                continue;
            };
            if !step.captured() || definition.command.is_some() {
                continue;
            }
            // Out of order on one clock: a retake or a resume put them here.
            if let (Some(p), Some(s)) = (prev.clock(), step.clock())
                && p.1 > s.0
            {
                continue;
            }
            let (Some(Ok(last)), Some(Ok(first))) = (prev.parsed.last(), step.parsed.first())
            else {
                continue;
            };
            if enter_only(definition.expect, Some(last)) {
                continue;
            }
            // The sub-values' numbers too: a step that only changes a
            // secondary display (a 121GW's setup steps) did arrive.
            let sub_values = |m: &Measurement| -> Vec<String> {
                m.aux_values
                    .iter()
                    .map(|a| a.value_str().into_owned())
                    .collect()
            };
            if State::of(first) == State::of(last) && sub_values(first) == sub_values(last) {
                let digits = if first.display_raw == last.display_raw {
                    ", digits too"
                } else {
                    ""
                };
                out.push(format!(
                    "[stale] {}: starts in the state {} ended in{digits}: {}",
                    step.id(),
                    prev.id(),
                    describe(&SampleData::from_measurement(first))
                ));
            }
        }
        out
    }

    fn summary(&self, out: &mut String) {
        for step in &self.steps {
            let r = step.result;
            let status = format!("{:?}", r.status).to_lowercase();
            out.push_str(&format!(
                "  {} [{status}, {} samples, {} frames]\n",
                r.id,
                r.samples.len(),
                r.frames.len()
            ));
            for (text, n) in tally(step.readings().iter().map(describe)) {
                out.push_str(&format!("      {text} \u{d7}{n}\n"));
            }
        }
    }

    fn stats(&self, out: &mut String) {
        let readings: Vec<SampleData> = self.steps.iter().flat_map(StepView::readings).collect();
        if readings.is_empty() {
            out.push_str("  no readings\n");
            return;
        }
        let flags: Vec<StatusFlags> = readings
            .iter()
            .map(|s| StatusFlags::from(&s.flags))
            .collect();
        let names = |pick: &dyn Fn(Flag) -> bool| {
            let names: Vec<&str> = Flag::ALL
                .into_iter()
                .filter(|&f| pick(f))
                .map(Flag::name)
                .collect();
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        };
        out.push_str(&format!(
            "  flags never set: {}\n",
            names(&|f| flags.iter().all(|s| !s.get(f)))
        ));
        out.push_str(&format!(
            "  flags always set: {}\n",
            names(&|f| flags.iter().all(|s| s.get(f)))
        ));

        // Mode by mode, in the order the run met them.
        let mut modes: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
        for s in &readings {
            let i = match modes.iter().position(|(m, _, _)| *m == s.mode) {
                Some(i) => i,
                None => {
                    modes.push((s.mode.clone(), vec![], vec![]));
                    modes.len() - 1
                }
            };
            let (_, units, ranges) = &mut modes[i];
            if !units.contains(&s.unit) {
                units.push(s.unit.clone());
            }
            if !s.range_label.is_empty() && !ranges.contains(&s.range_label) {
                ranges.push(s.range_label.clone());
            }
        }
        for (mode, units, ranges) in modes {
            let ranges = if ranges.is_empty() {
                String::new()
            } else {
                format!("; ranges {}", ranges.join(" "))
            };
            out.push_str(&format!(
                "  mode {mode}: units {}{ranges}\n",
                units.join(" ")
            ));
        }
    }
}

/// The step with each sample parsed again by `protocol`.
fn parse_step<'a>(result: &'a StepResult, protocol: Option<&dyn Protocol>) -> StepView<'a> {
    let Some(protocol) = protocol else {
        return StepView {
            result,
            parsed: vec![],
            unrecognised: vec![],
        };
    };
    let (parsed, unrecognised) = capture_reports(|| {
        result
            .samples
            .iter()
            .map(|s| match hex_bytes(&s.raw_hex) {
                Some(payload) => protocol.parse_payload(&payload).map_err(|e| e.to_string()),
                None => Err(format!("not hex: {:?}", s.raw_hex)),
            })
            .collect()
    });
    StepView {
        result,
        parsed,
        unrecognised,
    }
}

/// A reading on one line: mode, range, then what the confirmation line
/// showed the operator.
fn describe(s: &SampleData) -> String {
    let range = if s.range_label.is_empty() {
        String::new()
    } else {
        format!(" {}", s.range_label)
    };
    format!("{}{range}: {}", s.mode, s.summary())
}

/// The operator said the reading was wrong: what they saw beside what this
/// build makes of the step's last sample.
fn lcd_finding(step: &StepView) -> String {
    let id = step.id();
    let lcd = step.result.lcd.as_deref();
    let shown = lcd.map_or("(nothing typed)".to_string(), |t| format!("{t:?}"));
    let Some(reading) = step.readings().pop() else {
        return format!("[lcd] {id}: meter showed {shown}; no samples");
    };
    let tag = match lcd {
        Some(t) if same_words(t, &reading.summary()) => " (matches this build)",
        Some(t) if unit_only(t, &reading) => " (same digits, unit differs)",
        _ => "",
    };
    format!(
        "[lcd] {id}: meter showed {shown}; decoded {}{tag}",
        describe(&reading)
    )
}

/// Whether what the operator typed says what the reading says: the same
/// value and unit, and only flags the reading has. v0.6.0 reports filed
/// whatever the operator typed, a matching reading included, often without
/// the AUTO they took for granted.
fn same_words(typed: &str, summary: &str) -> bool {
    let (typed_main, typed_flags) = words_and_flags(typed);
    let (main, flags) = words_and_flags(summary);
    typed_main == main && typed_flags.iter().all(|f| flags.contains(f))
}

/// The words outside brackets, sorted, and the flag words inside them.
fn words_and_flags(text: &str) -> (Vec<&str>, Vec<&str>) {
    let (mut main, mut flags) = (Vec::new(), Vec::new());
    let mut inside = false;
    for part in text.split_inclusive(['[', ']']) {
        let words = part.trim_end_matches(['[', ']']).split_whitespace();
        if inside {
            flags.extend(words);
        } else {
            main.extend(words);
        }
        inside = part.ends_with('[');
    }
    main.sort_unstable();
    (main, flags)
}

/// Whether what the operator typed has the reading's digits and not its
/// unit: the 1000x mistake, a range labelled mV on a meter showing V
/// (issue #19).
fn unit_only(lcd: &str, reading: &SampleData) -> bool {
    let digits = |t: &str| {
        t.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .trim_start_matches('0')
            .to_string()
    };
    let want = digits(reading.shown());
    // The units typed: the letters of each word, "0.037V" and "0.037 V"
    // alike. Nothing typed is no disagreement.
    let (words, _) = words_and_flags(lcd);
    let units: Vec<&str> = words
        .iter()
        .flat_map(|w| w.split(|c: char| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')))
        .filter(|u| !u.is_empty())
        .collect();
    !want.is_empty()
        && digits(lcd) == want
        && !units.is_empty()
        && !units.contains(&reading.unit.as_str())
}

/// Where this build reads a step's samples differently from the build that
/// wrote the report, one line per distinct set of differences.
fn reparse_findings(step: &StepView) -> Vec<String> {
    let total = step.parsed.len();
    let lines =
        step.result.samples.iter().zip(&step.parsed).filter_map(
            |(recorded, parsed)| match parsed {
                Ok(m) => {
                    let diffs = differences(recorded, &SampleData::from_measurement(m));
                    (!diffs.is_empty()).then(|| {
                        diffs
                            .into_iter()
                            .map(|(_, d)| d)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                }
                Err(e) => Some(format!("no longer parses ({e}): {}", recorded.raw_hex)),
            },
        );
    tally(lines)
        .into_iter()
        .map(|(line, n)| format!("{} ({n} of {total} samples): {line}", step.id()))
        .collect()
}

/// The fields that differ: each one's name, and `name "before" → "after"`.
fn differences(recorded: &SampleData, now: &SampleData) -> Vec<(&'static str, String)> {
    let mut diffs = Vec::new();
    let mut field = |name: &'static str, a: &str, b: &str| {
        if a != b {
            diffs.push((name, format!("{name} {a:?} \u{2192} {b:?}")));
        }
    };
    field("mode", &recorded.mode, &now.mode);
    field("mode byte", &recorded.mode_byte, &now.mode_byte);
    field("display", &recorded.display_raw, &now.display_raw);
    field("value", &recorded.value, &now.value);
    field("unit", &recorded.unit, &now.unit);
    field("range", &recorded.range_label, &now.range_label);
    field(
        "progress",
        &recorded.progress.to_string(),
        &now.progress.to_string(),
    );
    field(
        "flags",
        &flag_names(&StatusFlags::from(&recorded.flags)),
        &flag_names(&StatusFlags::from(&now.flags)),
    );
    field(
        "label",
        recorded.main_label.as_deref().unwrap_or(""),
        now.main_label.as_deref().unwrap_or(""),
    );
    // A UT61+ secondary display arrives with the next frame, which a
    // payload parsed on its own never has.
    if !now.aux.is_empty() {
        let aux = |s: &SampleData| {
            s.aux
                .iter()
                .map(|a| format!("{} {} {}", a.label, a.value, a.unit))
                .collect::<Vec<_>>()
                .join(", ")
        };
        field("sub-values", &aux(recorded), &aux(now));
    }
    diffs
}

fn flag_names(flags: &StatusFlags) -> String {
    flags.active().map(Flag::name).collect::<Vec<_>>().join(" ")
}

fn detection_line(d: &super::detection::DetectionCheck) -> String {
    let mut parts = vec![format!("{:?}", d.outcome).to_lowercase()];
    if let Some(id) = &d.device_id {
        parts.push(format!("as {id}"));
    }
    if let Some(bridge) = &d.bridge {
        parts.push(format!("over {bridge}"));
    }
    if let Some(ms) = d.elapsed_ms {
        parts.push(format!("in {ms} ms"));
    }
    if d.power_cycled {
        parts.push("after a restart".to_string());
    }
    if d.cable_replugged {
        parts.push("cable replugged".to_string());
    }
    if let Some(rb) = &d.read_back {
        parts.push(format!(
            "read-back {} at {}",
            format!("{:?}", rb.outcome).to_lowercase(),
            rb.step
        ));
    }
    if let Some(e) = &d.error {
        parts.push(format!("error: {e}"));
    }
    parts.join(", ")
}

/// Each distinct item with how many times it came, in first-seen order.
fn tally(items: impl IntoIterator<Item = String>) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for item in items {
        match out.iter_mut().find(|(seen, _)| *seen == item) {
            Some((_, n)) => *n += 1,
            None => out.push((item, 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::make_test_measurement;
    use dmm_lib::protocol::registry::find_device;

    /// A UT61E+ sample: mode byte, autorange, the digits given.
    fn sample(mode: u8, digits: &[u8; 7]) -> SampleData {
        SampleData::from_measurement(&make_test_measurement(
            mode,
            0x01,
            digits,
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ))
    }

    fn dcv(digits: &[u8; 7]) -> SampleData {
        sample(0x02, digits)
    }

    fn acv(digits: &[u8; 7]) -> SampleData {
        sample(0x00, digits)
    }

    fn step(id: &str, samples: Vec<SampleData>) -> StepResult {
        StepResult {
            samples,
            ..StepResult::new(id, "", StepStatus::Captured)
        }
    }

    fn ut61eplus_report(steps: Vec<StepResult>) -> CaptureReport {
        CaptureReport {
            tool_version: "0.8.0-dev (test)".to_string(),
            device_id: Some("ut61eplus".to_string()),
            transport_name: Some("CP2110".to_string()),
            steps,
            ..CaptureReport::default()
        }
    }

    fn triage(report: &CaptureReport) -> String {
        let device = find_device("ut61eplus");
        load::scrub_addresses(&Triage::new(report, device, None).render())
    }

    fn findings(text: &str) -> Vec<&str> {
        text.lines()
            .skip_while(|l| *l != "Findings")
            .skip(1)
            .take_while(|l| !l.is_empty())
            .map(str::trim)
            .collect()
    }

    /// AC V filing the DC V reading the step before ended on is the
    /// operator's turn of the dial never arriving; DC V shorted after DC V
    /// open is the same dial position, where nothing shows arriving.
    #[test]
    fn a_step_that_starts_where_the_previous_ended_is_found() {
        let report = ut61eplus_report(vec![
            step("dcv", vec![dcv(b" 0.0002"), dcv(b" 0.0001")]),
            step("dcv_short", vec![dcv(b" 0.0001")]),
            step("acv", vec![dcv(b" 0.0001"), acv(b" 0.0001")]),
        ]);
        let text = triage(&report);
        assert_eq!(
            findings(&text),
            [
                "[stale] acv: starts in the state dcv_short ended in, digits too: DC V 22V: 0.0001 V [AUTO]"
            ],
            "{text}"
        );
    }

    /// The reading this build makes of a sample is set beside what the
    /// report's build filed, field by field.
    #[test]
    fn a_sample_read_differently_is_listed() {
        let mut old = dcv(b" 0.0371");
        old.unit = "mV".to_string();
        let report = ut61eplus_report(vec![step("dcv", vec![old, dcv(b" 0.0372")])]);
        let text = triage(&report);
        assert!(
            text.contains("[reparse] 1 steps read differently by this build: unit in 1 samples"),
            "{text}"
        );
        assert!(
            text.contains("dcv (1 of 2 samples): unit \"mV\" \u{2192} \"V\""),
            "{text}"
        );
    }

    /// The operator's text with the reading's digits and another unit is
    /// the 1000x mistake; one that says what the reading says is not.
    #[test]
    fn an_lcd_mismatch_in_the_unit_alone_is_tagged() {
        let mut wrong = step("dcv", vec![dcv(b" 0.0371")]);
        wrong.confirmed = Some(false);
        wrong.lcd = Some("0.0371 mV".to_string());
        let mut same = step("acv", vec![acv(b" 0.0371")]);
        same.confirmed = Some(false);
        same.lcd = Some("0.0371 V".to_string());
        // No unit typed, or one typed against the digits, is no
        // disagreement.
        let mut bare = step("dcmv", vec![dcv(b" 0.0371")]);
        bare.confirmed = Some(false);
        bare.lcd = Some("0.0371".to_string());
        let mut glued = step("acmv", vec![dcv(b" 0.0371")]);
        glued.confirmed = Some(false);
        glued.lcd = Some("0.0371V MAX".to_string());
        let text = triage(&ut61eplus_report(vec![wrong, same, bare, glued]));
        let found = findings(&text);
        assert!(found[0].ends_with("(same digits, unit differs)"), "{text}");
        assert!(found[1].ends_with("(matches this build)"), "{text}");
        assert!(found[2].ends_with("[AUTO]"), "{text}");
        assert!(found[3].ends_with("[AUTO]"), "{text}");
    }

    /// A resumed run, trimmed frames and dropped events show in the header,
    /// and an address in any free text is masked.
    #[test]
    fn the_header_says_what_the_report_is_missing() {
        let frame = |at_ms| crate::capture::report::FrameRecord {
            at_ms,
            dir: crate::capture::report::FrameDir::Rx,
            hex: "AB CD".to_string(),
            feature: false,
            baud: None,
        };
        let mut first = step("dcv", vec![dcv(b" 0.0001")]);
        first.frames = vec![frame(5000), frame(6000)];
        first.frames_dropped = 12;
        let mut resumed = StepResult::new("hold", "", StepStatus::Error);
        resumed.frames = vec![frame(100)];
        resumed.error = Some("link to 00:11:22:33:44:55 lost".to_string());
        let mut report = ut61eplus_report(vec![first, resumed]);
        report.wire_events_dropped = 7;
        report.transport_info = Some("Bluetooth DMM (00:11:22:33:44:55)".to_string());
        let text = triage(&report);
        assert!(text.contains("Clock: goes back 1 time(s)"), "{text}");
        assert!(
            text.contains("Truncated: 7 wire events dropped by the session bound; oldest frames trimmed per step: dcv 12"),
            "{text}"
        );
        assert!(
            text.contains("[error] hold: link to XX:XX:XX:XX:XX:XX lost"),
            "{text}"
        );
        assert!(!text.contains("00:11:22"), "{text}");
    }

    fn write_temp(name: &str, text: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dmm-triage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A v0.6.0 report has no `device_id` and a free-text `screen`, and a
    /// terminal may have escaped its Ω: it loads, and `--device` decodes it.
    #[test]
    fn an_old_escaped_report_loads() {
        let ohm = dcv(b" 0.0001").raw_hex;
        let path = write_temp(
            "v060.yaml",
            &format!(
                "date: x\ntool_version: 0.6.0 (0eea1c6)\ndevice_name: UT61E+\nsupported: true\n\
                 steps:\n- id: ohm_ranges/range:600\\CE\\A9\n  instruction: ''\n  status: captured\n  \
                 screen: 0.0001 V\n  samples:\n  - raw_hex: {ohm}\n    mode_byte: '0x02'\n    \
                 mode: DC V\n    display_raw: ' 0.0001'\n    value: '0.0001'\n    unit: V\n    \
                 range_label: 22V\n    progress: 0\n    flags:\n      hold: false\n      \
                 rel: false\n      auto_range: true\n      min: false\n      max: false\n      \
                 low_battery: false\n      hv_warning: false\n      dc: false\n      \
                 peak_min: false\n      peak_max: false\n"
            ),
        );
        let report = load::load(&path).unwrap();
        assert_eq!(report.steps[0].id, "ohm_ranges/range:600Ω");
        assert_eq!(report.steps[0].lcd.as_deref(), Some("0.0001 V"));
        assert!(load::device(None, &report).unwrap().is_none());
        let device = load::device(Some("ut61eplus"), &report).unwrap();
        let text = Triage::new(&report, device, None).render();
        assert!(
            text.contains("[lcd] ohm_ranges/range:600Ω: meter showed \"0.0001 V\"; decoded DC V 22V: 0.0001 V [AUTO] (matches this build)"),
            "{text}"
        );
    }

    #[test]
    fn a_plan_is_not_a_report() {
        let path = write_temp(
            "plan.yaml",
            "steps:\n- id: hz\n  instruction: x\n  command: hz\n",
        );
        let error = load::load(&path).err().unwrap();
        assert!(error.contains("a capture plan, not a report"), "{error}");
        let path = write_temp("trace.txt", "[DEBUG dmm_lib] read 4 bytes\n");
        let error = load::load(&path).err().unwrap();
        assert!(error.ends_with("not a capture report"), "{error}");
    }
}
