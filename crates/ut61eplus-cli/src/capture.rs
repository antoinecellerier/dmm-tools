use console::style;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Duration;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

// --- Data types ---

#[derive(Serialize, Deserialize, Default)]
pub struct CaptureReport {
    pub date: String,
    pub tool_version: String,
    pub device_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cp2110_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cp2110_firmware: Option<u8>,
    pub supported: bool,
    pub steps: Vec<StepResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Captured,
    Skipped,
    Timeout,
    Error,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StepResult {
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
pub struct SampleData {
    /// Raw 14-byte payload as hex string (e.g. "02 30 20 30 2E 30 30 30 30 00 00 30 30 30")
    pub raw_hex: String,
    pub mode_byte: String,
    pub mode: String,
    pub range_byte: String,
    pub display_raw: String,
    pub value: String,
    pub unit: String,
    pub range_label: String,
    pub progress: u16,
    pub flags: SampleFlags,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SampleFlags {
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
}

impl SampleData {
    pub fn from_measurement(m: &Measurement) -> Self {
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
            range_byte: String::new(),
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
            },
        }
    }

    pub fn summary(&self) -> String {
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

/// Run a simplified capture using protocol-provided steps.
/// Used for experimental (non-UT61E+) protocols.
fn run_protocol_capture(
    dmm: &mut ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
    protocol_steps: Vec<ut61eplus_lib::protocol::CaptureStep>,
    step_filter: &Option<std::collections::HashSet<String>>,
    report: &mut CaptureReport,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let include = |id: &str| -> bool { step_filter.as_ref().is_none_or(|f| f.contains(id)) };

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
        if !include(step.id) {
            continue;
        }
        let done = run_capture_step(dmm, step, report, true)?;
        save_report(report, output_path)?;
        if done {
            break;
        }
    }

    eprintln!(
        "\n{} {}",
        style("Capture complete!").green().bold(),
        style(format!("Saved to {output_path}")).dim()
    );
    Ok(())
}

// --- Step definitions ---

pub struct CaptureStep {
    pub id: &'static str,
    pub instruction: &'static str,
    pub command: Option<&'static str>,
    pub samples: usize,
}

/// IDs for part 1 (modes) vs part 2 (flags) grouping.
pub const MODE_STEP_IDS: &[&str] = &[
    "dcv",
    "dcv_short",
    "acv",
    "dcmv",
    "ohm",
    "ohm_short",
    "continuity",
    "diode",
    "capacitance",
    "hz",
    "duty",
    "ncv",
    "hfe",
    "dcua",
    "dcma",
    "dca",
    "temp",
];
pub const FLAG_STEP_IDS: &[&str] = &[
    "hold",
    "hold_off",
    "rel",
    "rel_off",
    "minmax",
    "minmax_off",
    "range",
    "auto",
];

pub fn all_capture_steps() -> Vec<CaptureStep> {
    vec![
        // Part 1: Modes
        CaptureStep {
            id: "dcv",
            instruction: "Set meter to DC V (V\u{23CF}). Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "dcv_short",
            instruction: "DC V mode: touch the two probe tips together.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "acv",
            instruction: "Set meter to AC V (V~). Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "dcmv",
            instruction: "Set meter to DC mV. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "ohm",
            instruction: "Set meter to \u{03A9}. Leave leads open (should show OL).",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "ohm_short",
            instruction: "\u{03A9} mode: touch the two probe tips together.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "continuity",
            instruction: "Set meter to continuity (buzzer). Touch probes together.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "diode",
            instruction: "Set meter to diode. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "capacitance",
            instruction: "Set meter to capacitance (F). Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "hz",
            instruction: "Set meter to Hz. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "duty",
            instruction: "Hz mode: press USB/Hz to switch to duty cycle (%).",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "ncv",
            instruction: "Set meter to NCV. Hold near a live wire if possible.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "hfe",
            instruction: "Set meter to hFE. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "dcua",
            instruction: "Set meter to \u{00B5}A. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "dcma",
            instruction: "Set meter to mA. Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "dca",
            instruction: "Set meter to A (if available). Leave leads open.",
            command: None,
            samples: 3,
        },
        CaptureStep {
            id: "temp",
            instruction: "Set meter to Temperature (if K-type thermocouple available).",
            command: None,
            samples: 3,
        },
        // Part 2: Flags
        CaptureStep {
            id: "hold",
            instruction: "Sending HOLD command.",
            command: Some("hold"),
            samples: 3,
        },
        CaptureStep {
            id: "hold_off",
            instruction: "Toggling HOLD off.",
            command: Some("hold"),
            samples: 3,
        },
        CaptureStep {
            id: "rel",
            instruction: "Sending REL command.",
            command: Some("rel"),
            samples: 3,
        },
        CaptureStep {
            id: "rel_off",
            instruction: "Toggling REL off.",
            command: Some("rel"),
            samples: 3,
        },
        CaptureStep {
            id: "minmax",
            instruction: "Sending MIN/MAX command.",
            command: Some("minmax"),
            samples: 3,
        },
        CaptureStep {
            id: "minmax_off",
            instruction: "Exiting MIN/MAX.",
            command: Some("exit_minmax"),
            samples: 3,
        },
        CaptureStep {
            id: "range",
            instruction: "Sending RANGE (manual range).",
            command: Some("range"),
            samples: 3,
        },
        CaptureStep {
            id: "auto",
            instruction: "Sending AUTO (restore auto-range).",
            command: Some("auto"),
            samples: 3,
        },
        // Part 3: Range cycle
        CaptureStep {
            id: "range_cycle",
            instruction: "Cycle through manual ranges on DC V.",
            command: None,
            samples: 0,
        },
    ]
}

// --- Helpers ---

pub fn prompt(msg: &str) -> Result<String, Box<dyn std::error::Error>> {
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn prompt_key(msg: &str) -> Result<char, Box<dyn std::error::Error>> {
    let term = console::Term::stderr();
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let ch = term.read_char().unwrap_or('\n');
    eprintln!();
    Ok(ch)
}

pub fn capture_samples(
    dmm: &mut ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
    n: usize,
) -> Vec<Measurement> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(ut61eplus_lib::error::Error::Timeout) => {}
            Err(e) => {
                eprintln!("  error: {e}");
                break;
            }
        }
        attempts += 1;
    }
    samples
}

pub fn save_report(report: &CaptureReport, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let yaml = serde_yaml::to_string(report)?;
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
pub fn upsert_step(report: &mut CaptureReport, result: StepResult) {
    if let Some(pos) = report.steps.iter().position(|s| s.id == result.id) {
        report.steps[pos] = result;
    } else {
        report.steps.push(result);
    }
}

/// Run one capture step. Returns Ok(true) if user wants to quit.
pub fn run_capture_step(
    dmm: &mut ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
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
            upsert_step(
                report,
                StepResult {
                    id: step.id.to_string(),
                    instruction: step.instruction.to_string(),
                    status: StepStatus::Skipped,
                    samples: vec![],
                    screen: None,
                    error: None,
                },
            );
            return Ok(true);
        }
        if ch == 's' || ch == 'S' {
            upsert_step(
                report,
                StepResult {
                    id: step.id.to_string(),
                    instruction: step.instruction.to_string(),
                    status: StepStatus::Skipped,
                    samples: vec![],
                    screen: None,
                    error: None,
                },
            );
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
                StepResult {
                    id: step.id.to_string(),
                    instruction: step.instruction.to_string(),
                    status: StepStatus::Error,
                    samples: vec![],
                    screen: None,
                    error: Some(e.to_string()),
                },
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
            s.range_byte,
            s.display_raw
        );
    }

    let screen = if interactive && !sample_data.is_empty() {
        let summary = sample_data.last().expect("checked non-empty").summary();
        eprintln!("  We read: {}", style(&summary).green());
        let input = prompt(&format!(
            "  {} ",
            style("Enter=correct, or type what the meter actually shows:").dim()
        ))?;
        if input.is_empty() {
            Some(format!("confirmed: {summary}"))
        } else {
            Some(input)
        }
    } else if sample_data.is_empty() {
        eprintln!("  {}", style("No response from meter.").yellow());
        None
    } else {
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
mod tests {
    use super::*;
    /// Helper: build a Measurement for testing (no real device needed).
    fn make_measurement(
        mode: u8,
        range: u8,
        display: &[u8; 7],
        progress: (u8, u8),
        flags: (u8, u8, u8),
    ) -> Measurement {
        let payload: Vec<u8> = vec![
            mode,
            range | 0x30,
            display[0],
            display[1],
            display[2],
            display[3],
            display[4],
            display[5],
            display[6],
            progress.0,
            progress.1,
            flags.0 | 0x30,
            flags.1 | 0x30,
            flags.2 | 0x30,
        ];
        let table = ut61eplus_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable::new();
        ut61eplus_lib::protocol::ut61eplus::parse_measurement(&payload, &table).unwrap()
    }

    #[test]
    fn sample_data_from_normal_measurement() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x05, 0x0A), (0x00, 0x00, 0x00));
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
        let m = make_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.value, "OL");
        assert_eq!(s.mode, "Ω");
    }

    #[test]
    fn sample_data_from_ncv() {
        let m = make_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.value, "NCV:3");
        assert_eq!(s.mode, "NCV");
    }

    #[test]
    fn sample_data_raw_hex() {
        let m = make_measurement(0x02, 0x00, b" 0.0000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        // raw_hex should have 14 hex bytes separated by spaces
        let parts: Vec<&str> = s.raw_hex.split(' ').collect();
        assert_eq!(parts.len(), 14);
    }

    #[test]
    fn sample_data_flags_mapping() {
        // flag1=0x0F (REL+HOLD+MIN+MAX), flag2=0x04 (manual range), flag3=0x08 (DC)
        let m = make_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x04, 0x08));
        let s = SampleData::from_measurement(&m);
        assert!(s.flags.hold);
        assert!(s.flags.rel);
        assert!(s.flags.min);
        assert!(s.flags.max);
        assert!(!s.flags.auto_range);
        assert!(s.flags.dc);
    }

    #[test]
    fn summary_format() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        let summary = s.summary();
        assert!(summary.contains("5.678"));
        assert!(summary.contains("V"));
        assert!(summary.contains("AUTO"));
        assert!(summary.contains("HOLD"));
    }

    #[test]
    fn summary_auto_only() {
        let m = make_measurement(0x02, 0x01, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
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

    #[test]
    fn all_capture_steps_has_expected_count() {
        let steps = all_capture_steps();
        // 17 mode steps + 8 flag steps + 1 range_cycle = 26
        assert_eq!(steps.len(), 26);
    }

    #[test]
    fn all_capture_steps_unique_ids() {
        let steps = all_capture_steps();
        let mut ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
        let len_before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len_before, "step IDs must be unique");
    }

    #[test]
    fn mode_and_flag_step_ids_cover_all_steps() {
        let steps = all_capture_steps();
        for step in &steps {
            let in_modes = MODE_STEP_IDS.contains(&step.id);
            let in_flags = FLAG_STEP_IDS.contains(&step.id);
            let is_range_cycle = step.id == "range_cycle";
            assert!(
                in_modes || in_flags || is_range_cycle,
                "step {} not in any category",
                step.id
            );
        }
    }

    #[test]
    fn capture_report_serde_roundtrip() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let sample = SampleData::from_measurement(&m);

        let report = CaptureReport {
            date: "2026-01-01T00:00:00+00:00".to_string(),
            tool_version: "0.2.0-dev (abc1234)".to_string(),
            device_name: "UT61E+".to_string(),
            cp2110_part: Some("0x0a".to_string()),
            cp2110_firmware: Some(10),
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

        let yaml = serde_yaml::to_string(&report).unwrap();
        let parsed: CaptureReport = serde_yaml::from_str(&yaml).unwrap();

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
            cp2110_part: None,
            cp2110_firmware: None,
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

        let yaml = serde_yaml::to_string(&report).unwrap();
        // Optional None fields should not appear in output
        assert!(!yaml.contains("cp2110_part"));
        assert!(!yaml.contains("cp2110_firmware"));
        // Empty samples should not appear
        assert!(!yaml.contains("samples"));

        // Parse back
        let parsed: CaptureReport = serde_yaml::from_str(&yaml).unwrap();
        assert!(parsed.cp2110_part.is_none());
        assert!(parsed.cp2110_firmware.is_none());
        assert!(parsed.steps[0].samples.is_empty());
    }

    #[test]
    fn step_status_serde() {
        let yaml = serde_yaml::to_string(&StepStatus::Captured).unwrap();
        assert!(yaml.contains("captured"));

        let yaml = serde_yaml::to_string(&StepStatus::Skipped).unwrap();
        assert!(yaml.contains("skipped"));

        let yaml = serde_yaml::to_string(&StepStatus::Timeout).unwrap();
        assert!(yaml.contains("timeout"));

        let yaml = serde_yaml::to_string(&StepStatus::Error).unwrap();
        assert!(yaml.contains("error"));
    }

    #[test]
    fn save_and_load_report_file() {
        let report = CaptureReport {
            date: "2026-01-01".to_string(),
            tool_version: "test".to_string(),
            device_name: "UT61E+".to_string(),
            cp2110_part: None,
            cp2110_firmware: None,
            supported: true,
            steps: vec![],
        };

        let dir = std::env::temp_dir();
        let path = dir
            .join("ut61eplus-test-capture.yaml")
            .to_string_lossy()
            .to_string();

        save_report(&report, &path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: CaptureReport = serde_yaml::from_str(&contents).unwrap();
        assert_eq!(parsed.device_name, "UT61E+");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}

// --- Main capture command ---

pub fn list_steps() {
    let steps = all_capture_steps();
    eprintln!("{}", style("Available capture steps:").bold());
    eprintln!();
    eprintln!("{}", style("  Measurement modes:").cyan());
    for s in &steps {
        if MODE_STEP_IDS.contains(&s.id) {
            eprintln!("    {:<16} {}", style(s.id).bold(), s.instruction);
        }
    }
    eprintln!();
    eprintln!("{}", style("  Flags & commands:").cyan());
    for s in &steps {
        if FLAG_STEP_IDS.contains(&s.id) {
            eprintln!("    {:<16} {}", style(s.id).bold(), s.instruction);
        }
    }
    eprintln!();
    eprintln!("{}", style("  Other:").cyan());
    eprintln!(
        "    {:<16} Cycle through manual ranges on DC V",
        style("range_cycle").bold()
    );
    eprintln!(
        "    {:<16} Freeform captures (Part 4)",
        style("extra").bold()
    );
    eprintln!();
    eprintln!(
        "Usage: {} {}",
        style("ut61eplus capture --steps").dim(),
        style("dcmv,temp,duty").dim()
    );
}

pub fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    mut dmm: ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
    device: &'static ut61eplus_lib::protocol::registry::SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_filter: Option<std::collections::HashSet<String>> =
        filter.map(|v| v.into_iter().collect());

    // Verify the meter is actually responding before proceeding
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

    let supported = dmm.profile().stability == ut61eplus_lib::protocol::Stability::Verified;
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

    // Determine output path: explicit override, or auto-name from device
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

    // Check for existing file — auto-resume or prompt to overwrite
    let mut report = match std::fs::read_to_string(&output_path) {
        Ok(contents) => match serde_yaml::from_str::<CaptureReport>(&contents) {
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
                    return Ok(());
                }
                if ch == 'n' || ch == 'N' {
                    let confirm = prompt_key(
                        "This will overwrite the existing capture. Are you sure? y/n: ",
                    )?;
                    if confirm != 'y' && confirm != 'Y' {
                        eprintln!("Aborted.");
                        return Ok(());
                    }
                    CaptureReport::default()
                } else if ch == 'r' || ch == 'R' {
                    eprintln!("Resuming — already-captured steps will be skipped.\n");
                    r
                } else {
                    eprintln!("Aborted.");
                    return Ok(());
                }
            }
            Err(_) => {
                eprintln!("Found {output_path} but couldn't parse it.");
                let ch = prompt_key("Overwrite? y=start fresh, any other key=abort: ")?;
                if ch != 'y' && ch != 'Y' {
                    eprintln!("Aborted.");
                    return Ok(());
                }
                CaptureReport::default()
            }
        },
        Err(_) => CaptureReport::default(),
    };

    eprintln!("Output file: {output_path}\n");

    report.date = chrono::Local::now().to_rfc3339();
    report.tool_version = format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
    report.device_name = device_name;
    if let Ok(ver) = dmm.transport().version_info() {
        report.cp2110_part = Some(format!("{:#04x}", ver.part_number));
        report.cp2110_firmware = Some(ver.device_version);
    }
    report.supported = supported;

    // For non-UT61E+ protocols, use protocol-provided capture steps
    let protocol_steps = dmm.capture_steps();
    if !protocol_steps.is_empty() {
        return run_protocol_capture(
            &mut dmm,
            protocol_steps,
            &step_filter,
            &mut report,
            &output_path,
        );
    }

    let all_steps = all_capture_steps();
    let is_filtered = step_filter.is_some();
    let include = |id: &str| -> bool { step_filter.as_ref().is_none_or(|f| f.contains(id)) };

    let mut done = false;

    // --- Part 1: Modes ---
    let has_mode_steps = MODE_STEP_IDS.iter().any(|id| include(id));
    if has_mode_steps {
        eprintln!(
            "{}",
            style("\u{2501}\u{2501}\u{2501} Part 1: Measurement Modes \u{2501}\u{2501}\u{2501}")
                .bold()
        );
        eprintln!(
            "{}",
            style("any key=capture, s=skip one, q=skip to end and save").dim()
        );

        for step in all_steps.iter().filter(|s| MODE_STEP_IDS.contains(&s.id)) {
            if done {
                break;
            }
            if !include(step.id) {
                continue;
            }
            done = run_capture_step(&mut dmm, step, &mut report, true)?;
            save_report(&report, &output_path)?;
        }
    }

    // --- Part 2: Flags ---
    let has_flag_steps = FLAG_STEP_IDS.iter().any(|id| include(id));
    if !done && has_flag_steps {
        eprintln!(
            "\n{}",
            style(
                "\u{2501}\u{2501}\u{2501} Part 2: Flags & Remote Commands \u{2501}\u{2501}\u{2501}"
            )
            .bold()
        );
        eprintln!("Set meter to DC V mode for these tests.");
        let ch = prompt_key(&format!(
            "\n{} ",
            style("Any key when ready on DC V, q=skip to end:").dim()
        ))?;
        if ch == 'q' || ch == 'Q' {
            done = true;
        }
    }
    if !done && has_flag_steps {
        for step in all_steps.iter().filter(|s| FLAG_STEP_IDS.contains(&s.id)) {
            if done {
                break;
            }
            if !include(step.id) {
                continue;
            }
            done = run_capture_step(&mut dmm, step, &mut report, true)?;
            save_report(&report, &output_path)?;
        }
    }

    // --- Part 3: Range cycle ---
    if !done && include("range_cycle") {
        eprintln!(
            "\n{}",
            style("\u{2501}\u{2501}\u{2501} Part 3: Range Values \u{2501}\u{2501}\u{2501}").bold()
        );
        eprintln!("We'll cycle through manual ranges on DC V.");
        let ch = prompt_key(&format!(
            "\n{} ",
            style("Any key to start, q=skip to end:").dim()
        ))?;
        if ch != 'q' && ch != 'Q' {
            let _ = dmm.send_command("auto");
            std::thread::sleep(Duration::from_millis(200));

            let mut range_samples = Vec::new();
            for r in 0..6 {
                let _ = dmm.send_command("range");
                std::thread::sleep(Duration::from_millis(300));
                let raw = capture_samples(&mut dmm, 2);
                for m in &raw {
                    let s = SampleData::from_measurement(m);
                    eprintln!(
                        "  range_step_{r}: range={} label={}",
                        s.range_byte, s.range_label
                    );
                    range_samples.push(s);
                }
            }
            let _ = dmm.send_command("auto");
            std::thread::sleep(Duration::from_millis(200));

            upsert_step(
                &mut report,
                StepResult {
                    id: "range_cycle".to_string(),
                    instruction: "Cycle through manual ranges on DC V".to_string(),
                    status: StepStatus::Captured,
                    samples: range_samples,
                    screen: None,
                    error: None,
                },
            );
            save_report(&report, &output_path)?;
        } else {
            done = true;
        }
    }

    // --- Part 4: Freeform (skip if filtered) ---
    if !done && (!is_filtered || include("extra")) {
        eprintln!("\n{}", style("\u{2501}\u{2501}\u{2501} Part 4: Additional Captures (optional) \u{2501}\u{2501}\u{2501}").bold());
        eprintln!("Set the meter to any mode/state not covered above.\n");

        let mut extra = 0u32;
        loop {
            let desc = prompt(&format!(
                "[extra_{extra}] Describe what you set the meter to (or 'q' to finish): "
            ))?;
            if desc.is_empty() || desc.to_lowercase().starts_with('q') {
                break;
            }

            let raw = capture_samples(&mut dmm, 3);
            let sample_data: Vec<SampleData> =
                raw.iter().map(SampleData::from_measurement).collect();

            for (i, s) in sample_data.iter().enumerate() {
                eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
            }

            let screen = if !sample_data.is_empty() {
                let summary = sample_data.last().expect("checked non-empty").summary();
                let input = prompt(&format!(
                    "  We read: {summary}\n  Enter=correct, or type correction: "
                ))?;
                if input.is_empty() {
                    Some(format!("confirmed: {summary}"))
                } else {
                    Some(input)
                }
            } else {
                eprintln!("  No response from meter.");
                None
            };

            upsert_step(
                &mut report,
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
            save_report(&report, &output_path)?;
            extra += 1;
        }
    }

    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    eprintln!("Please attach this file to your bug report or issue.");
    Ok(())
}
