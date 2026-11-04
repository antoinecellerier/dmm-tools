//! `dmm-cli triage`: what a maintainer reads off a capture report a reporter
//! attached, worked out with this build's parser instead of a throwaway
//! script that re-implements it.
//!
//! The output is plain text for a terminal, a file or an issue: a header,
//! findings one per line tagged by kind, each step's readings and the run's
//! stats, then with `--timeline` each step's frames in order. Only the
//! timeline carries times, so two reports of the same steps diff line by
//! line without it.

mod decode;
mod findings;
mod fixtures;
mod load;
mod log;
mod replay;
mod stats;
mod timeline;

pub(crate) use log::install as install_triage_logger;

use super::report::{CaptureReport, SampleData, StepResult, StepStatus, Tier, hex_bytes};
use super::watch::{STABLE_FRAMES, State, StateWatcher, Verdict, enter_only};
use crate::cli::TriageArgs;
use decode::{EventKind, Wire};
use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{CaptureStep, Expect, Protocol, RangeExpect, capture_reports};
use findings::reparse_findings;

pub(crate) fn cmd_triage(
    args: &TriageArgs,
    named_device: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = load::load(&args.report).map_err(|e| load::scrub(&e))?;
    let device = load::device(named_device, &report)?;
    let plan = args.plan.as_deref().map(super::plan::load).transpose()?;
    let triage = Triage::new(&report, device, plan);
    let mut text = triage.render();
    if let Some(steps) = &args.timeline {
        text.push_str(&timeline::render(&triage, steps));
    }
    if args.fixtures {
        text.push_str(&fixtures::render(&triage));
    }
    print!("{}", load::scrub(&text));
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
    /// The step's frames as the protocol decodes them; `None` with no
    /// device or no frames.
    wire: Option<Wire>,
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

    /// What the step's readings say, from the wire where its frames
    /// decoded and from its samples otherwise.
    fn all_readings(&self) -> Vec<SampleData> {
        match &self.wire {
            Some(wire) if wire.readings().next().is_some() => wire
                .readings()
                .map(|(_, m)| SampleData::from_measurement(m))
                .collect(),
            _ => self.readings(),
        }
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
        let link = match report.transport_name.as_deref() {
            Some(dmm_lib::BLUETOOTH) => dmm_lib::transport::Link::Bluetooth,
            _ => dmm_lib::transport::Link::UsbCable,
        };
        let info = device_information(report);
        let steps = report
            .steps
            .iter()
            .map(|result| {
                let mut view = parse_step(result, protocol.as_deref());
                view.wire = device
                    .filter(|_| !result.frames.is_empty())
                    .map(|d| decode::decode(&result.frames, d, Some(link), info.as_deref()));
                view
            })
            .collect();
        Triage {
            report,
            device,
            definitions,
            steps,
        }
    }

    /// The entry the meter's own name names when it is not the one the
    /// steps are decoded as: a UT217A run as `--device cms101` named itself
    /// the CMS061 (#45), whose keys and tables are its own.
    fn named_other_entry(&self) -> Option<&'static SelectableDevice> {
        let device = self.device?;
        let named = (device.new_protocol)().entry_named(&self.report.device_name)?;
        (named.id != device.id).then_some(named)
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
                "Device: {} ({}){} over {transport}; the meter named itself {:?}\n",
                d.id,
                d.display_name(),
                r.picked_as
                    .as_deref()
                    .map(|id| format!(", picked as {id}"))
                    .unwrap_or_default(),
                r.device_name
            )),
            None => out.push_str(&format!(
                "Device: none named, so nothing is parsed again (pass --device); \
                 over {transport}, the meter named itself {:?}\n",
                r.device_name
            )),
        }
        if let Some(named) = self.named_other_entry() {
            out.push_str(&format!(
                "  that name is the {} entry's: decode as it with `dmm-cli --device {} triage <report>`\n",
                named.display_name(),
                named.id
            ));
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
        let translated = r
            .steps
            .iter()
            .filter(|s| s.instruction_shown.is_some())
            .count();
        if translated > 0 {
            run.push(format!(
                "{translated} step(s) showed the reporter's own translation (instruction_shown)"
            ));
        }
        if r.no_response {
            run.push(match &r.no_response_error {
                Some(error) => format!("the meter never answered ({error})"),
                None => "the meter never answered".to_string(),
            });
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
}

/// The step with each sample parsed again by `protocol`.
fn parse_step<'a>(result: &'a StepResult, protocol: Option<&dyn Protocol>) -> StepView<'a> {
    let Some(protocol) = protocol else {
        return StepView {
            result,
            parsed: vec![],
            unrecognised: vec![],
            wire: None,
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
    // What parsing the samples logged belongs to no frame; left in the
    // buffer, it would show under the next step's first event.
    log::take();
    StepView {
        result,
        parsed,
        unrecognised,
        wire: None,
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
    // A reading with no unit leaves a double space in the summary.
    format!("{}{range}: {}", s.mode, s.summary().replace("  ", " "))
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

/// The device information the capture's link read at connect (OWON's
/// FFF2), from the note the Bluetooth transport ends `transport_info` with:
/// a protocol that reads it decodes the steps as the live run did.
fn device_information(report: &CaptureReport) -> Option<Vec<u8>> {
    let info = report.transport_info.as_deref()?;
    let (_, hex) = info.rsplit_once(", device information: ")?;
    hex_bytes(hex)
}

#[cfg(test)]
pub(super) mod tests {
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

    pub(super) fn dcv(digits: &[u8; 7]) -> SampleData {
        sample(0x02, digits)
    }

    fn acv(digits: &[u8; 7]) -> SampleData {
        sample(0x00, digits)
    }

    pub(super) fn step(id: &str, samples: Vec<SampleData>) -> StepResult {
        StepResult {
            samples,
            ..StepResult::new(id, "", StepStatus::Captured)
        }
    }

    pub(super) fn ut61eplus_report(steps: Vec<StepResult>) -> CaptureReport {
        CaptureReport {
            tool_version: "0.8.0-dev (test)".to_string(),
            device_id: Some("ut61eplus".to_string()),
            transport_name: Some("CP2110".to_string()),
            steps,
            ..CaptureReport::default()
        }
    }

    /// The FFF2 bytes come from the Bluetooth transport's note; none when
    /// it was not read, or the link reads none.
    #[test]
    fn device_information_is_read_off_the_transport_note() {
        let report = |info: &str| CaptureReport {
            transport_info: Some(info.to_string()),
            ..CaptureReport::default()
        };
        let read = report("BDM (12:34:56:78:9A:BC), device information: 3D 00 01 04 00 00");
        assert_eq!(
            device_information(&read),
            Some(vec![0x3D, 0x00, 0x01, 0x04, 0x00, 0x00])
        );
        let unread = report("BDM (12:34:56:78:9A:BC), device information: not read");
        assert_eq!(device_information(&unread), None);
        assert_eq!(device_information(&report("CP2110 (/dev/hidraw3)")), None);
        assert_eq!(device_information(&CaptureReport::default()), None);
    }

    /// A meter whose own name is another entry's is pointed at that entry,
    /// and one that names the entry decoding it is not (#45).
    #[test]
    fn a_meter_naming_another_entry_is_pointed_at_it() {
        let report = |name: &str| CaptureReport {
            device_id: Some("cms101".to_string()),
            device_name: name.to_string(),
            transport_name: Some("Bluetooth".to_string()),
            ..CaptureReport::default()
        };
        let header = |report: &CaptureReport| {
            let mut out = String::new();
            Triage::new(report, find_device("cms101"), None).header(&mut out);
            out
        };
        let other = header(&report("OWON CMS061"));
        assert!(
            other.contains(
                "that name is the OWON CMS061 entry's: decode as it with \
                 `dmm-cli --device cms061 triage <report>`"
            ),
            "{other}"
        );
        assert!(!header(&report("OWON CMS101")).contains("that name is"));
    }

    fn triage(report: &CaptureReport) -> String {
        let device = find_device("ut61eplus");
        load::scrub(&Triage::new(report, device, None).render())
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

    /// A ZT-5B report, its steps' frames as the meter put them on air.
    fn zt5b_report(steps: Vec<StepResult>) -> CaptureReport {
        CaptureReport {
            device_id: Some("zt5b".to_string()),
            transport_name: Some(dmm_lib::BLUETOOTH.to_string()),
            ..ut61eplus_report(steps)
        }
    }

    /// A captured ZT-5B key step: `before` three times, the key, `after`
    /// four times, and the last as its sample (`plain`, descrambled).
    fn key_step(id: &str, key: &str, before: &str, after: &str, plain: &str) -> StepResult {
        let mut list = vec![(0, "rx", before), (330, "rx", before), (660, "rx", before)];
        list.push((660, "tx", key));
        list.extend((1..=4).map(|i| (660 + 330 * i, "rx", after)));
        let mut step = step(id, vec![]);
        step.frames = decode::tests::frames(&list);
        let payload = hex_bytes(plain).unwrap();
        let m = (find_device("zt5b").unwrap().new_protocol)()
            .parse_payload(&payload)
            .unwrap();
        step.samples = vec![SampleData::from_measurement(&m)];
        step
    }

    /// Two keys a ZT-5B ignored passed on a flicker (issue #31); the one
    /// that took is not flagged.
    #[test]
    fn a_key_that_left_the_meter_as_it_was_is_found() {
        let dca = "1B 84 71 55 A2 21 C9 FA 22 AE";
        let fahrenheit = "1B 84 71 55 A2 21 BD FE 66 EA";
        let held = "1B 84 71 57 A2 21 BD FE 66 EA";
        let report = zt5b_report(vec![
            key_step(
                "key_capacitance",
                "EA EC 70 E5 A2 C1 32 71 64 81",
                dca,
                dca,
                "5A A5 02 00 00 E0 FB 8B 44 04",
            ),
            key_step(
                "key_hold",
                "EA EC 70 E1 A2 C1 32 71 64 85",
                fahrenheit,
                held,
                "5A A5 02 02 00 E0 8F 8F 00 40",
            ),
        ]);
        let text = load::scrub(&Triage::new(&report, find_device("zt5b"), None).render());
        assert_eq!(
            findings(&text),
            [
                "[command] key_capacitance: no new state held after the key; the meter shows DC A: 0.0 mA",
                "[expect] key_capacitance: mode is \"DC A\", want \"Capacitance\" (this build's step): DC A: 0.0 mA",
            ],
            "{text}"
        );
    }

    /// A sub-step is held to the setting its id names; a step this build
    /// dropped says so, and a plan run's steps are left alone without the
    /// plan.
    #[test]
    fn a_sub_step_must_show_its_setting() {
        let report = ut61eplus_report(vec![
            step("dcv/range:2.2V", vec![dcv(b" 0.0001")]),
            step("dcv/mode:AC V", vec![dcv(b" 0.0001")]),
            step("dcv/hold:on", vec![dcv(b" 0.0001")]),
            step("rpm", vec![dcv(b" 0.0001")]),
        ]);
        let text = triage(&report);
        assert_eq!(
            findings(&text),
            [
                "[expect] dcv/range:2.2V: range is \"22V\", want \"2.2V\" (this build's step): DC V 22V: 0.0001 V [AUTO]",
                "[expect] dcv/mode:AC V: mode is \"DC V\", want \"AC V\" (this build's step): DC V 22V: 0.0001 V [AUTO]",
                "[expect] rpm: this build has no such step",
            ],
            "{text}"
        );
        let planned = CaptureReport {
            plan: Some("plan.yaml".to_string()),
            ..report
        };
        let text = triage(&planned);
        assert_eq!(findings(&text), ["none"], "{text}");
    }

    /// A key with no reading kept before it has nothing to be measured
    /// against, and a plan run triaged without its plan has no definitions
    /// to tell a key step or a same-dial step by.
    #[test]
    fn what_cannot_be_judged_is_not() {
        let dca = "1B 84 71 55 A2 21 C9 FA 22 AE";
        let mut trimmed = key_step(
            "key_capacitance",
            "EA EC 70 E5 A2 C1 32 71 64 81",
            dca,
            dca,
            "5A A5 02 00 00 E0 FB 8B 44 04",
        );
        trimmed.frames.drain(..3);
        trimmed.samples.clear();
        let text = load::scrub(
            &Triage::new(&zt5b_report(vec![trimmed]), find_device("zt5b"), None).render(),
        );
        assert_eq!(
            findings(&text),
            ["[command] key_capacitance: no reading before its key was kept, so it is not judged"],
            "{text}"
        );

        let planned = CaptureReport {
            plan: Some("plan.yaml".to_string()),
            ..ut61eplus_report(vec![
                step("dcv", vec![dcv(b" 0.0001")]),
                step("acv", vec![dcv(b" 0.0001")]),
            ])
        };
        let text = triage(&planned);
        assert_eq!(findings(&text), ["none"], "{text}");
    }

    /// A step error spelling terminal escapes (`\e` in a YAML string is
    /// ESC) reaches the output as text: no ESC or BEL goes out.
    #[test]
    fn a_report_cannot_send_terminal_escapes() {
        let path = write_temp(
            "escapes.yaml",
            "date: x\ntool_version: 0.9.0\ndevice_name: UT61E+\nsupported: true\n\
             steps:\n- id: dcv\n  instruction: ''\n  status: error\n  \
             error: \"\\e[2J\\e]52;c;ZWNobw==\\a\"\n",
        );
        let report = load::load(&path).unwrap();
        assert_eq!(
            report.steps[0].error.as_deref(),
            Some("\u{1b}[2J\u{1b}]52;c;ZWNobw==\u{7}")
        );
        let text = triage(&report);
        assert!(!text.contains(['\u{1b}', '\u{7}']), "{text}");
        assert!(
            text.contains(r"[error] dcv: \u{1b}[2J\u{1b}]52;c;ZWNobw==\u{7}"),
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
