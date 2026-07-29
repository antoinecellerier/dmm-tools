use console::style;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::protocol::registry::SelectableDevice;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Duration;

// --- Data types ---

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CaptureReport {
    pub date: String,
    pub tool_version: String,
    pub device_name: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        alias = "cp2110_part"
    )]
    pub transport_name: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        alias = "cp2110_firmware"
    )]
    pub transport_info: Option<String>,
    pub supported: bool,
    pub steps: Vec<StepResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StepStatus {
    Captured,
    Skipped,
    Timeout,
    Error,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct StepResult {
    pub id: String,
    pub instruction: String,
    pub status: StepStatus,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub samples: Vec<SampleData>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub screen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SampleData {
    /// Raw 14-byte payload as hex string (e.g. "02 30 20 30 2E 30 30 30 30 00 00 30 30 30")
    pub raw_hex: String,
    pub mode_byte: String,
    pub mode: String,
    pub display_raw: String,
    pub value: String,
    pub unit: String,
    pub range_label: String,
    pub progress: u16,
    pub flags: SampleFlags,
    /// Sub-values the meter reported alongside the main reading (UT181A
    /// REL/MIN-MAX/peak, UT171 frequency aux). Empty for most families, and
    /// omitted from the YAML when empty so their reports are unchanged.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aux: Vec<AuxSample>,
}

/// One sub-value in a captured sample.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AuxSample {
    pub label: String,
    pub value: String,
    pub unit: String,
    /// Seconds since the mode started, for the UT181A's MIN/MAX timestamps.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub elapsed_secs: Option<u32>,
}

/// Status flags recorded per sample.
///
/// Must cover every `StatusFlags` field — a capture report is the evidence a
/// maintainer works from, and a flag missing here reads as "the meter didn't
/// set it". `capture_report_covers_every_status_flag` enforces that.
///
/// `#[serde(default)]` on the fields added after the first release keeps
/// older reports loadable, which `load_or_create_report` relies on to resume
/// an interrupted capture.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SampleFlags {
    pub hold: bool,
    pub rel: bool,
    pub auto_range: bool,
    pub min: bool,
    pub max: bool,
    pub low_battery: bool,
    pub hv_warning: bool,
    pub dc: bool,
    pub peak_min: bool,
    pub peak_max: bool,
    #[serde(default)]
    pub lead_error: bool,
    #[serde(default)]
    pub comp: bool,
    #[serde(default)]
    pub record: bool,
    #[serde(default)]
    pub loz: bool,
    #[serde(default)]
    pub void: bool,
}

impl SampleData {
    pub(crate) fn from_measurement(m: &Measurement) -> Self {
        let value = match &m.value {
            MeasuredValue::Normal(v) => format!("{v}"),
            MeasuredValue::Overload => "OL".to_string(),
            MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
        };
        let raw_hex = m
            .raw_payload
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            raw_hex,
            mode_byte: format!("{:#04x}", m.mode_raw),
            mode: m.mode.to_string(),
            display_raw: m.display_raw.clone().unwrap_or_default(),
            value,
            unit: m.unit.to_string(),
            range_label: m.range_label.to_string(),
            progress: m.progress.unwrap_or(0),
            flags: SampleFlags {
                hold: m.flags.hold,
                rel: m.flags.rel,
                auto_range: m.flags.auto_range,
                min: m.flags.min,
                max: m.flags.max,
                low_battery: m.flags.low_battery,
                hv_warning: m.flags.hv_warning,
                dc: m.flags.dc,
                peak_min: m.flags.peak_min,
                peak_max: m.flags.peak_max,
                lead_error: m.flags.lead_error,
                comp: m.flags.comp,
                record: m.flags.record,
                loz: m.flags.loz,
                void: m.flags.void,
            },
            aux: m
                .aux_values
                .iter()
                .map(|a| AuxSample {
                    label: a.label.to_string(),
                    value: a.value_str().into_owned(),
                    // An empty aux unit means "same as the main reading";
                    // resolve it here so the report stands on its own.
                    unit: if a.unit.is_empty() {
                        m.unit.to_string()
                    } else {
                        a.unit.to_string()
                    },
                    elapsed_secs: a.elapsed_secs,
                })
                .collect(),
        }
    }

    pub(crate) fn summary(&self) -> String {
        let mut flag_parts = Vec::new();
        if self.flags.auto_range {
            flag_parts.push("AUTO");
        }
        if self.flags.hold {
            flag_parts.push("HOLD");
        }
        if self.flags.rel {
            flag_parts.push("REL");
        }
        if self.flags.min {
            flag_parts.push("MIN");
        }
        if self.flags.max {
            flag_parts.push("MAX");
        }
        format!(
            "{} {} [{}]",
            self.display_raw.trim(),
            self.unit,
            flag_parts.join(" ")
        )
    }
}

/// Run the device's own capture steps: modes, flags, and the manual range
/// sweep, in the order the protocol declares them.
///
/// Returns `true` if the user asked to finish early, so the caller can skip
/// the freeform pass.
fn run_protocol_capture(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    protocol_steps: Vec<dmm_lib::protocol::CaptureStep>,
    step_filter: &Option<std::collections::HashSet<String>>,
    report: &mut CaptureReport,
    output_path: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Convert protocol steps to CLI steps
    let steps: Vec<CaptureStep> = protocol_steps
        .iter()
        .map(|ps| CaptureStep {
            id: ps.id,
            instruction: ps.instruction,
            command: ps.command,
            samples: ps.samples,
        })
        .collect();

    eprintln!(
        "{}",
        style("\u{2501}\u{2501}\u{2501} Measurement Modes \u{2501}\u{2501}\u{2501}").bold()
    );
    eprintln!(
        "{}",
        style("any key=capture, s=skip one, q=skip to end and save").dim()
    );

    for step in &steps {
        if !step_included(step_filter, step.id) {
            continue;
        }
        if run_capture_step(dmm, step, report, true)? {
            return Ok(true);
        }
        save_report(report, output_path)?;
    }

    Ok(false)
}

// --- Step definitions ---

pub(crate) struct CaptureStep {
    pub id: &'static str,
    pub instruction: &'static str,
    pub command: Option<&'static str>,
    pub samples: usize,
}

impl CaptureStep {
    /// Create a StepResult with no samples or screen capture.
    fn empty_result(&self, status: StepStatus, error: Option<String>) -> StepResult {
        StepResult {
            id: self.id.to_string(),
            instruction: self.instruction.to_string(),
            status,
            samples: vec![],
            screen: None,
            error,
        }
    }
}

// --- Helpers ---

pub(crate) fn prompt(msg: &str) -> Result<String, Box<dyn std::error::Error>> {
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub(crate) fn prompt_key(msg: &str) -> Result<char, Box<dyn std::error::Error>> {
    let term = console::Term::stderr();
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let ch = term.read_char().unwrap_or('\n');
    eprintln!();
    Ok(ch)
}

pub(crate) fn capture_samples(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    n: usize,
) -> Vec<Measurement> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => {
                eprintln!("  error: {e}");
                break;
            }
        }
        attempts += 1;
    }
    samples
}

pub(crate) fn save_report(
    report: &CaptureReport,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let yaml = serde_yaml_ng::to_string(report)?;
    // Atomic write: write to temp file then rename, so a crash mid-write
    // doesn't corrupt the existing report.
    let tmp_path = format!("{path}.tmp");
    let mut f = std::fs::File::create(&tmp_path)?;
    f.write_all(yaml.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Insert or replace a step result in the report.
pub(crate) fn upsert_step(report: &mut CaptureReport, result: StepResult) {
    if let Some(pos) = report.steps.iter().position(|s| s.id == result.id) {
        report.steps[pos] = result;
    } else {
        report.steps.push(result);
    }
}

/// Run one capture step. Returns Ok(true) if user wants to quit.
pub(crate) fn run_capture_step(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    step: &CaptureStep,
    report: &mut CaptureReport,
    interactive: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Check if already captured (resume)
    if report
        .steps
        .iter()
        .any(|s| s.id == step.id && s.status == StepStatus::Captured)
    {
        eprintln!("  {} already captured, skipping", style(step.id).dim());
        return Ok(false);
    }

    if interactive {
        eprintln!();
        eprintln!(
            "{} {}",
            style(format!("[{}]", step.id)).cyan().bold(),
            step.instruction
        );
        let ch = prompt_key(&format!(
            "  {} ",
            style("any key=capture, s=skip, q=finish:").dim()
        ))?;
        if ch == 'q' || ch == 'Q' {
            upsert_step(report, step.empty_result(StepStatus::Skipped, None));
            return Ok(true);
        }
        if ch == 's' || ch == 'S' {
            upsert_step(report, step.empty_result(StepStatus::Skipped, None));
            return Ok(false);
        }
    } else {
        eprintln!(
            "{} {}",
            style(format!("[{}]", step.id)).cyan().bold(),
            step.instruction
        );
    }

    if let Some(cmd) = step.command {
        if let Err(e) = dmm.send_command(cmd) {
            eprintln!("  {}", style(format!("Command failed: {e}")).red());
            upsert_step(
                report,
                step.empty_result(StepStatus::Error, Some(e.to_string())),
            );
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let raw_samples = capture_samples(dmm, step.samples);
    let sample_data: Vec<SampleData> = raw_samples
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

    let screen = if let Some(last) = sample_data.last() {
        if interactive {
            let summary = last.summary();
            eprintln!("  We read: {}", style(&summary).green());
            let input = prompt(&format!(
                "  {} ",
                style("Enter=correct, or type what the meter actually shows:").dim()
            ))?;
            Some(if input.is_empty() {
                format!("confirmed: {summary}")
            } else {
                input
            })
        } else {
            None
        }
    } else {
        eprintln!("  {}", style("No response from meter.").yellow());
        None
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

    let result = StepResult {
        id: step.id.to_string(),
        instruction: step.instruction.to_string(),
        status,
        samples: sample_data,
        screen,
        error: None,
    };

    upsert_step(report, result);
    Ok(false)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    // `capture_steps` is a Protocol method; the trait has to be in scope to
    // call it on a concrete protocol type (not needed for `dyn Protocol`).
    use dmm_lib::protocol::Protocol;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;

    #[test]
    fn sample_data_from_normal_measurement() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x09), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.mode_byte, "0x02");
        assert_eq!(s.mode, "DC V");
        assert_eq!(s.unit, "V");
        assert_eq!(s.range_label, "22V");
        assert_eq!(s.value, "5.678");
        assert!(s.flags.auto_range);
        assert!(!s.flags.hold);
    }

    #[test]
    fn sample_data_from_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.value, "OL");
        assert_eq!(s.mode, "Ω");
    }

    #[test]
    fn sample_data_from_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.value, "NCV:3");
        assert_eq!(s.mode, "NCV");
    }

    #[test]
    fn sample_data_raw_hex() {
        let m = make_test_measurement(0x02, 0x00, b" 0.0000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        // raw_hex should have 14 hex bytes separated by spaces
        let parts: Vec<&str> = s.raw_hex.split(' ').collect();
        assert_eq!(parts.len(), 14);
    }

    #[test]
    fn sample_data_flags_mapping() {
        // flag1=0x0F (REL+HOLD+MIN+MAX), flag2=0x04 (manual range), flag3=0x08 (DC)
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x04, 0x08));
        let s = SampleData::from_measurement(&m);
        assert!(s.flags.hold);
        assert!(s.flags.rel);
        assert!(s.flags.min);
        assert!(s.flags.max);
        assert!(!s.flags.auto_range);
        assert!(s.flags.dc);
    }

    /// A capture report is the evidence a maintainer works from, so every
    /// flag the meter can set has to reach it. Five were missing —
    /// lead_error, comp, record, loz and void — so a VC-890 capture taken
    /// with VOID lit arrived showing all-false.
    ///
    /// Serialize a fully-set SampleFlags and check the YAML has one key per
    /// StatusFlags field, all true: that catches both a field never added
    /// here and one added but left unassigned in `from_measurement`.
    #[test]
    fn capture_report_covers_every_status_flag() {
        use dmm_lib::flags::StatusFlags;

        let all_set = StatusFlags {
            hold: true,
            rel: true,
            min: true,
            max: true,
            auto_range: true,
            low_battery: true,
            hv_warning: true,
            dc: true,
            peak_max: true,
            peak_min: true,
            lead_error: true,
            comp: true,
            record: true,
            loz: true,
            void: true,
        };
        let mut m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.flags = all_set;

        let yaml = serde_yaml_ng::to_string(&SampleData::from_measurement(&m).flags).unwrap();
        let parsed: std::collections::BTreeMap<String, bool> =
            serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.len(), StatusFlags::COUNT);
        for (name, _) in all_set.as_pairs() {
            assert_eq!(
                parsed.get(name),
                Some(&true),
                "capture report drops or clears {name}"
            );
        }
    }

    /// Reports written before the five extra flags existed must still load,
    /// or resuming an interrupted capture would fail.
    #[test]
    fn older_reports_without_the_new_flags_still_load() {
        let yaml = "hold: true\nrel: false\nauto_range: true\nmin: false\nmax: false\n\
                    low_battery: false\nhv_warning: false\ndc: true\npeak_min: false\n\
                    peak_max: false\n";
        let flags: SampleFlags = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(flags.hold);
        assert!(!flags.void, "missing fields default to false");
    }

    #[test]
    fn summary_format() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        let summary = s.summary();
        assert!(summary.contains("5.678"));
        assert!(summary.contains("V"));
        assert!(summary.contains("AUTO"));
        assert!(summary.contains("HOLD"));
    }

    #[test]
    fn summary_auto_only() {
        let m = make_test_measurement(0x02, 0x01, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        let summary = s.summary();
        assert!(summary.contains("[AUTO]"));
    }

    #[test]
    fn upsert_step_insert() {
        let mut report = CaptureReport::default();
        let result = StepResult {
            id: "dcv".to_string(),
            instruction: "test".to_string(),
            status: StepStatus::Captured,
            samples: vec![],
            screen: None,
            error: None,
        };
        upsert_step(&mut report, result);
        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].id, "dcv");
    }

    #[test]
    fn upsert_step_replace() {
        let mut report = CaptureReport::default();
        let result1 = StepResult {
            id: "dcv".to_string(),
            instruction: "first".to_string(),
            status: StepStatus::Skipped,
            samples: vec![],
            screen: None,
            error: None,
        };
        upsert_step(&mut report, result1);

        let result2 = StepResult {
            id: "dcv".to_string(),
            instruction: "replaced".to_string(),
            status: StepStatus::Captured,
            samples: vec![],
            screen: None,
            error: None,
        };
        upsert_step(&mut report, result2);

        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].instruction, "replaced");
        assert_eq!(report.steps[0].status, StepStatus::Captured);
    }

    #[test]
    fn upsert_step_multiple_ids() {
        let mut report = CaptureReport::default();
        for id in ["dcv", "acv", "ohm"] {
            upsert_step(
                &mut report,
                StepResult {
                    id: id.to_string(),
                    instruction: id.to_string(),
                    status: StepStatus::Captured,
                    samples: vec![],
                    screen: None,
                    error: None,
                },
            );
        }
        assert_eq!(report.steps.len(), 3);
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

    #[test]
    fn no_filter_accepts_everything() {
        let steps = dmm_lib::protocol::ut61eplus::Ut61PlusProtocol::new().capture_steps();
        assert!(validate_step_filter(&None, &steps).is_ok());
    }

    #[test]
    fn capture_report_serde_roundtrip() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let sample = SampleData::from_measurement(&m);

        let report = CaptureReport {
            date: "2026-01-01T00:00:00+00:00".to_string(),
            tool_version: "0.2.0-dev (abc1234)".to_string(),
            device_name: "UT61E+".to_string(),
            transport_name: Some("CP2110".to_string()),
            transport_info: Some("CP2110 part=0x0a firmware=10".to_string()),
            supported: true,
            steps: vec![StepResult {
                id: "dcv".to_string(),
                instruction: "Set meter to DC V".to_string(),
                status: StepStatus::Captured,
                samples: vec![sample],
                screen: Some("confirmed: 5.678 V [AUTO]".to_string()),
                error: None,
            }],
        };

        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        let parsed: CaptureReport = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.date, report.date);
        assert_eq!(parsed.device_name, report.device_name);
        assert!(parsed.supported);
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.steps[0].samples.len(), 1);
        assert_eq!(parsed.steps[0].samples[0].value, "5.678");
        assert_eq!(parsed.steps[0].screen, report.steps[0].screen);
    }

    #[test]
    fn capture_report_optional_fields_skip() {
        let report = CaptureReport {
            date: "2026-01-01".to_string(),
            tool_version: "0.2.0".to_string(),
            device_name: "UT61E+".to_string(),
            transport_name: None,
            transport_info: None,
            supported: true,
            steps: vec![StepResult {
                id: "dcv".to_string(),
                instruction: "test".to_string(),
                status: StepStatus::Skipped,
                samples: vec![],
                screen: None,
                error: None,
            }],
        };

        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        // Optional None fields should not appear in output
        assert!(!yaml.contains("transport_name"));
        assert!(!yaml.contains("transport_info"));
        // Empty samples should not appear
        assert!(!yaml.contains("samples"));

        // Parse back
        let parsed: CaptureReport = serde_yaml_ng::from_str(&yaml).unwrap();
        assert!(parsed.transport_name.is_none());
        assert!(parsed.transport_info.is_none());
        assert!(parsed.steps[0].samples.is_empty());
    }

    #[test]
    fn step_status_serde() {
        let yaml = serde_yaml_ng::to_string(&StepStatus::Captured).unwrap();
        assert!(yaml.contains("captured"));

        let yaml = serde_yaml_ng::to_string(&StepStatus::Skipped).unwrap();
        assert!(yaml.contains("skipped"));

        let yaml = serde_yaml_ng::to_string(&StepStatus::Timeout).unwrap();
        assert!(yaml.contains("timeout"));

        let yaml = serde_yaml_ng::to_string(&StepStatus::Error).unwrap();
        assert!(yaml.contains("error"));
    }

    #[test]
    fn save_and_load_report_file() {
        let report = CaptureReport {
            date: "2026-01-01".to_string(),
            tool_version: "test".to_string(),
            device_name: "UT61E+".to_string(),
            transport_name: None,
            transport_info: None,
            supported: true,
            steps: vec![],
        };

        let dir = std::env::temp_dir();
        let path = dir
            .join("dmm-cli-test-capture.yaml")
            .to_string_lossy()
            .to_string();

        save_report(&report, &path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: CaptureReport = serde_yaml_ng::from_str(&contents).unwrap();
        assert_eq!(parsed.device_name, "UT61E+");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}

// --- Main capture command ---

/// Filter keyword for the freeform capture pass. Not a protocol step — the
/// pass generates `extra_0`, `extra_1`, … as the user describes each capture.
pub(crate) const FREEFORM_STEP_ID: &str = "extra";

/// Print the step IDs `--steps` accepts for the selected device.
///
/// The steps come from the device's own protocol, so what this prints is
/// exactly what will run. It used to list a table held in this file, which no
/// device had used since protocols started declaring their own steps: the IDs
/// shown matched nothing, so `--steps` filtered everything out and wrote an
/// empty report.
pub(crate) fn list_steps(device: &'static SelectableDevice) {
    let protocol = (device.new_protocol)();
    let steps = protocol.capture_steps();

    eprintln!(
        "{} {}",
        style("Available capture steps for").bold(),
        style(device.display_name).bold().cyan()
    );
    eprintln!();
    if steps.is_empty() {
        eprintln!("  This device declares no capture steps.");
    }
    for s in &steps {
        eprintln!("    {:<16} {}", style(s.id).bold(), s.instruction);
    }
    eprintln!();
    eprintln!("{}", style("  Always available:").cyan());
    eprintln!(
        "    {:<16} Freeform captures — describe any mode not covered above",
        style(FREEFORM_STEP_ID).bold()
    );
    eprintln!();
    eprintln!(
        "Usage: {} {}",
        style("dmm-cli capture --steps").dim(),
        style("dcmv,temp,duty").dim()
    );
}

/// Reject `--steps` IDs that no step will match.
///
/// Without this an unknown ID silently filtered every step out, leaving a
/// report with `steps: []` that the CLI still announced as "Capture
/// complete!" — so the user attached an empty file to their bug report.
fn validate_step_filter(
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
fn step_included(step_filter: &Option<std::collections::HashSet<String>>, id: &str) -> bool {
    step_filter.as_ref().is_none_or(|f| f.contains(id))
}

/// Verify that the meter is responding. Returns `(device_name, supported)` on success.
fn verify_meter(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
) -> Result<(String, bool), Box<dyn std::error::Error>> {
    eprintln!("{}", style("Checking meter communication...").dim());
    let device_name = match dmm.get_name() {
        Ok(Some(name)) => name,
        Ok(None) | Err(_) => {
            // get_name failed or unsupported — try a plain measurement as fallback
            match dmm.request_measurement() {
                Ok(_) => "unknown".to_string(),
                Err(_) => {
                    eprintln!();
                    eprintln!(
                        "{}",
                        style("USB adapter found but the meter is not responding.")
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

    let supported = dmm.profile().stability == dmm_lib::protocol::Stability::Verified;
    eprintln!("Device: {}", style(&device_name).bold());
    if supported {
        eprintln!("Status: {}", style("supported model").green());
    } else {
        eprintln!(
            "Status: {}",
            style("UNKNOWN MODEL — captures are especially valuable!")
                .yellow()
                .bold()
        );
        eprintln!("        Protocol may differ from the UT61E+. Please complete");
        eprintln!("        as many steps as possible and share the report.");
    }
    eprintln!();

    Ok((device_name, supported))
}

/// Determine the output path and load an existing report (with resume/overwrite prompt)
/// or create a fresh one. Returns `None` if the user chose to abort.
fn load_or_create_report(
    output_override: Option<String>,
    device_name: &str,
) -> Result<Option<(CaptureReport, String)>, Box<dyn std::error::Error>> {
    let slug = device_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let auto_path = format!("capture-{slug}.yaml");
    let output_path = output_override.unwrap_or(auto_path);

    let report = match std::fs::read_to_string(&output_path) {
        Ok(contents) => match serde_yaml_ng::from_str::<CaptureReport>(&contents) {
            Ok(r) => {
                let captured = r
                    .steps
                    .iter()
                    .filter(|s| s.status == StepStatus::Captured)
                    .count();
                let skipped = r
                    .steps
                    .iter()
                    .filter(|s| s.status == StepStatus::Skipped)
                    .count();
                eprintln!(
                    "Found existing capture: {output_path} ({captured} captured, {skipped} skipped)"
                );
                let ch = prompt_key("r=resume, n=start fresh, q=abort: ")?;
                if ch == 'q' || ch == 'Q' {
                    eprintln!("Aborted.");
                    return Ok(None);
                }
                if ch == 'n' || ch == 'N' {
                    let confirm = prompt_key(
                        "This will overwrite the existing capture. Are you sure? y/n: ",
                    )?;
                    if confirm != 'y' && confirm != 'Y' {
                        eprintln!("Aborted.");
                        return Ok(None);
                    }
                    CaptureReport::default()
                } else if ch == 'r' || ch == 'R' {
                    eprintln!("Resuming — already-captured steps will be skipped.\n");
                    r
                } else {
                    eprintln!("Aborted.");
                    return Ok(None);
                }
            }
            Err(_) => {
                eprintln!("Found {output_path} but couldn't parse it.");
                let ch = prompt_key("Overwrite? y=start fresh, any other key=abort: ")?;
                if ch != 'y' && ch != 'Y' {
                    eprintln!("Aborted.");
                    return Ok(None);
                }
                CaptureReport::default()
            }
        },
        Err(_) => CaptureReport::default(),
    };

    Ok(Some((report, output_path)))
}

/// Populate report metadata (date, version, device info).
fn populate_report_metadata(
    report: &mut CaptureReport,
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    device_name: String,
    supported: bool,
) {
    report.date = chrono::Local::now().to_rfc3339();
    report.tool_version = format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
    report.device_name = device_name;
    report.transport_name = Some(dmm.transport().transport_name().to_string());
    if let Ok(info) = dmm.transport().transport_info() {
        report.transport_info = Some(info);
    }
    report.supported = supported;
}

/// Part 1: Run measurement mode capture steps. Returns true if user wants to quit.
/// Part 4: Freeform additional captures.
fn run_freeform_captures(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    step_filter: &Option<std::collections::HashSet<String>>,
    report: &mut CaptureReport,
    output_path: &str,
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

    let mut extra = 0u32;
    loop {
        let desc = prompt(&format!(
            "[extra_{extra}] Describe what you set the meter to (or 'q' to finish): "
        ))?;
        if desc.is_empty() || desc.to_lowercase().starts_with('q') {
            break;
        }

        let raw = capture_samples(dmm, 3);
        let sample_data: Vec<SampleData> = raw.iter().map(SampleData::from_measurement).collect();

        for (i, s) in sample_data.iter().enumerate() {
            eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
        }

        let screen = if let Some(last) = sample_data.last() {
            let summary = last.summary();
            let input = prompt(&format!(
                "  We read: {summary}\n  Enter=correct, or type correction: "
            ))?;
            Some(if input.is_empty() {
                format!("confirmed: {summary}")
            } else {
                input
            })
        } else {
            eprintln!("  No response from meter.");
            None
        };

        upsert_step(
            report,
            StepResult {
                id: format!("extra_{extra}"),
                instruction: desc,
                status: if sample_data.is_empty() {
                    StepStatus::Timeout
                } else {
                    StepStatus::Captured
                },
                samples: sample_data,
                screen,
                error: None,
            },
        );
        save_report(report, output_path)?;
        extra += 1;
    }

    Ok(())
}

pub(crate) fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    mut dmm: dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_filter: Option<std::collections::HashSet<String>> =
        filter.map(|v| v.into_iter().collect());

    let (device_name, supported) = verify_meter(&mut dmm, device)?;

    let (mut report, output_path) = match load_or_create_report(output_override, &device_name)? {
        Some(pair) => pair,
        None => return Ok(()),
    };

    eprintln!("Output file: {output_path}\n");

    populate_report_metadata(&mut report, &mut dmm, device_name, supported);

    validate_step_filter(&step_filter, &dmm.capture_steps())?;

    // The device's own steps cover modes, flags and button commands;
    // the freeform pass afterwards is device-agnostic and always offered,
    // since it is the only way to capture a mode the step list doesn't
    // anticipate — and the only step that records what the meter's screen
    // actually said next to what we parsed.
    let protocol_steps = dmm.capture_steps();
    let done = run_protocol_capture(
        &mut dmm,
        protocol_steps,
        &step_filter,
        &mut report,
        &output_path,
    )?;

    if !done {
        run_freeform_captures(&mut dmm, &step_filter, &mut report, &output_path)?;
    }

    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    eprintln!("Please attach this file to your bug report or issue.");
    Ok(())
}
