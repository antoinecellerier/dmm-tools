use crate::recording::{self, SharedRecorder, WireEvent};
use console::style;
use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::Measurement;
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
    /// Registry ID of the device the capture ran against — the report
    /// otherwise only holds the name the meter reports for itself.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub device_id: Option<String>,
    /// Wire events recorded before the first step: the init handshake and the
    /// name query.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub init_frames: Vec<FrameRecord>,
    /// Wire events lost to the recorder's bound, so a truncated trace is
    /// visible as one.
    #[serde(skip_serializing_if = "is_zero", default)]
    pub wire_events_dropped: u64,
    pub steps: Vec<StepResult>,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Cap on wire events recorded per step, so one chatty step can't grow the
/// report without bound. Overflow is reported in the step's diagnostics.
pub(crate) const MAX_FRAMES_PER_STEP: usize = 500;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FrameDir {
    Tx,
    Rx,
}

/// One transfer over the wire, including bytes the framing layer rejected.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct FrameRecord {
    pub at_ms: u64,
    pub dir: FrameDir,
    pub hex: String,
    /// HID feature report rather than an interrupt write.
    #[serde(skip_serializing_if = "is_false", default)]
    pub feature: bool,
}

impl From<&crate::recording::WireEvent> for FrameRecord {
    fn from(e: &crate::recording::WireEvent) -> Self {
        FrameRecord {
            at_ms: e.at_ms,
            dir: match e.dir {
                crate::recording::Direction::Tx => FrameDir::Tx,
                crate::recording::Direction::Rx => FrameDir::Rx,
            },
            hex: e
                .bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" "),
            feature: e.feature,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StepStatus {
    Captured,
    Skipped,
    Timeout,
    Error,
}

/// Where the operator's confirmation came from: the prompt shown at the step
/// itself, or a review pass over the finished report.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ConfirmedBy {
    Inline,
    Batch,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct StepResult {
    pub id: String,
    pub instruction: String,
    pub status: StepStatus,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub samples: Vec<SampleData>,
    /// Whether the operator said our reading matched the meter. `None` when
    /// they were never asked: a non-interactive run, or a step with no samples.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confirmed: Option<bool>,
    /// What the meter actually showed, recorded only when it disagreed with
    /// what we parsed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lcd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confirmed_by: Option<ConfirmedBy>,
    /// The free-text confirmation older reports stored. Read so a capture
    /// started before this split can still be resumed; never written back.
    #[serde(default, skip_serializing)]
    pub screen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Every byte exchanged while this step ran, so a step that decoded
    /// nothing still shows what the meter sent.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub frames: Vec<FrameRecord>,
    /// Parse rejections seen while sampling — checksum mismatches, unknown
    /// modes, malformed responses.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub diagnostics: Vec<String>,
    /// The step decoded incompletely: unknown mode, a parse rejection, or
    /// fewer samples than asked for. Points a maintainer at the step to read.
    #[serde(skip_serializing_if = "is_false", default)]
    pub needs_attention: bool,
}

impl StepResult {
    /// Base result for a step; the caller fills in what it actually captured.
    pub(crate) fn new(id: &str, instruction: &str, status: StepStatus) -> Self {
        StepResult {
            id: id.to_string(),
            instruction: instruction.to_string(),
            status,
            samples: vec![],
            confirmed: None,
            lcd: None,
            confirmed_by: None,
            screen: None,
            error: None,
            frames: vec![],
            diagnostics: vec![],
            needs_attention: false,
        }
    }

    /// Record the operator's answer to the confirmation prompt: empty input
    /// means our reading matched, anything else is what the meter showed.
    fn set_inline_confirmation(&mut self, input: String) {
        let confirmed = input.is_empty();
        self.confirmed = Some(confirmed);
        self.lcd = (!confirmed).then_some(input);
        self.confirmed_by = Some(ConfirmedBy::Inline);
    }

    /// Fold an older report's free-text `screen` into the structured fields,
    /// so resuming a capture started before the split keeps its confirmations.
    pub(crate) fn normalize_legacy(&mut self) {
        if self.confirmed.is_some() || self.lcd.is_some() || self.confirmed_by.is_some() {
            return;
        }
        let Some(text) = self.screen.take() else {
            return;
        };
        if text.starts_with("confirmed: ") {
            self.confirmed = Some(true);
        } else {
            self.confirmed = Some(false);
            self.lcd = Some(text);
        }
        self.confirmed_by = Some(ConfirmedBy::Inline);
    }
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
    /// secondary displays and REL/MIN-MAX/peak, UT171 frequency aux). Empty
    /// for most families, and omitted from the YAML when empty so their
    /// reports are unchanged.
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
    #[serde(default)]
    pub avg: bool,
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

/// From the library type, so a captured sample's flags are copied in one
/// place instead of field by field at the call site.
///
/// Listed field by field with no `..Default::default()`: a flag added to
/// `StatusFlags` must fail to compile here rather than silently read false.
impl From<&StatusFlags> for SampleFlags {
    fn from(f: &StatusFlags) -> Self {
        SampleFlags {
            hold: f.hold,
            rel: f.rel,
            auto_range: f.auto_range,
            min: f.min,
            max: f.max,
            avg: f.avg,
            low_battery: f.low_battery,
            hv_warning: f.hv_warning,
            dc: f.dc,
            peak_min: f.peak_min,
            peak_max: f.peak_max,
            lead_error: f.lead_error,
            comp: f.comp,
            record: f.record,
            loz: f.loz,
            void: f.void,
        }
    }
}

/// Back to the library type, so the summary line can be rendered by the one
/// `StatusFlags` `Display` every other output format already goes through.
///
/// Listed field by field with no `..Default::default()`: a flag added to
/// `StatusFlags` must fail to compile here rather than silently read false.
impl From<&SampleFlags> for StatusFlags {
    fn from(f: &SampleFlags) -> Self {
        StatusFlags {
            hold: f.hold,
            rel: f.rel,
            min: f.min,
            max: f.max,
            avg: f.avg,
            auto_range: f.auto_range,
            low_battery: f.low_battery,
            hv_warning: f.hv_warning,
            dc: f.dc,
            peak_max: f.peak_max,
            peak_min: f.peak_min,
            lead_error: f.lead_error,
            comp: f.comp,
            record: f.record,
            loz: f.loz,
            void: f.void,
        }
    }
}

impl SampleData {
    pub(crate) fn from_measurement(m: &Measurement) -> Self {
        // The parsed value, not `display_raw`: the report stores both, and
        // this column is the one a golden fixture is compared against.
        let value = m.value.to_string();
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
            flags: SampleFlags::from(&m.flags),
            aux: m
                .aux_values
                .iter()
                .map(|a| AuxSample {
                    label: a.label.to_string(),
                    value: a.value_str().into_owned(),
                    // Resolve the "empty means main unit" convention here so
                    // the report stands on its own.
                    unit: a.unit_or(&m.unit).to_string(),
                    elapsed_secs: a.elapsed_secs,
                })
                .collect(),
        }
    }

    /// The one-line rendering the operator is asked to compare against the
    /// meter's screen.
    ///
    /// Rendered through `StatusFlags`, the same way `dmm-cli read` prints a
    /// reading: this used to list only AUTO/HOLD/REL/MIN/MAX, so a UT181A
    /// sample taken on mains confirmed as `239.22 VAC [AUTO]` while the meter
    /// — and the sample's own `flags:` map — showed the high-voltage warning.
    pub(crate) fn summary(&self) -> String {
        let flags = StatusFlags::from(&self.flags).to_string();
        let value = self.display_raw.trim();
        let mut out = if flags.is_empty() {
            format!("{value} {}", self.unit)
        } else {
            format!("{value} {} [{flags}]", self.unit)
        };
        // Sub-values are on the meter's screen too, so the operator has to
        // see them to confirm the sample — a UT181A in MIN/MAX or with the
        // frequency display up otherwise confirms against half the screen.
        // Appended, so single-display meters keep the line they had, and
        // rendered by the same helper `dmm-cli read` uses so the two can't
        // describe the same reading differently.
        if !self.aux.is_empty() {
            let aux = dmm_lib::measurement::aux_summary_line(self.aux.iter().map(|a| {
                (
                    a.label.as_str(),
                    a.value.as_str(),
                    a.unit.as_str(),
                    a.elapsed_secs,
                )
            }));
            out.push_str(&format!(" ({aux})"));
        }
        out
    }
}

/// Run the device's own capture steps: modes, flags, and the manual range
/// sweep, in the order the protocol declares them.
///
/// Returns `true` if the user asked to finish early, so the caller can skip
/// the freeform pass.
fn run_protocol_capture(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
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
        if run_capture_step(dmm, recorder, step, report, true)? {
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
            error,
            ..StepResult::new(self.id, self.instruction, status)
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

/// What one sampling pass produced: the readings that parsed, and the errors
/// that stopped the others from parsing.
pub(crate) struct SampleRun {
    pub samples: Vec<Measurement>,
    pub diagnostics: Vec<String>,
}

/// Poll for `n` samples.
///
/// A parse rejection is recorded and polling continues: a meter whose frames
/// we misread would otherwise end the step on the first bad frame, and the
/// report showed neither a sample nor a reason.
pub(crate) fn capture_samples(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    n: usize,
) -> SampleRun {
    let mut samples = Vec::new();
    // First-seen order, with a repeat count: a stuck meter otherwise fills
    // the report with the same line.
    let mut errors: Vec<(String, usize)> = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => {
                let text = e.to_string();
                eprintln!("  error: {text}");
                match errors.iter_mut().find(|(t, _)| *t == text) {
                    Some((_, count)) => *count += 1,
                    None => errors.push((text, 1)),
                }
            }
        }
        attempts += 1;
    }
    SampleRun {
        samples,
        diagnostics: errors
            .into_iter()
            .map(|(text, count)| {
                if count > 1 {
                    format!("{text} (x{count})")
                } else {
                    text
                }
            })
            .collect(),
    }
}

pub(crate) fn save_report(
    report: &CaptureReport,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml_ng::to_string(report)?;
    // Atomic write (.tmp + fsync + rename), so a crash mid-write doesn't
    // corrupt the existing report.
    dmm_settings::write_atomic(std::path::Path::new(path), yaml.as_bytes())?;
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

/// Wire events belonging to `step_id`, capped, with the overflow noted.
fn frames_for_step(
    events: &[WireEvent],
    step_id: &str,
    diagnostics: &mut Vec<String>,
) -> Vec<FrameRecord> {
    let mine: Vec<&WireEvent> = events
        .iter()
        .filter(|e| e.step.as_deref() == Some(step_id))
        .collect();
    if mine.len() > MAX_FRAMES_PER_STEP {
        diagnostics.push(format!(
            "{} further wire events not recorded (cap {MAX_FRAMES_PER_STEP})",
            mine.len() - MAX_FRAMES_PER_STEP
        ));
    }
    mine.into_iter()
        .take(MAX_FRAMES_PER_STEP)
        .map(FrameRecord::from)
        .collect()
}

/// Whether the step is worth a maintainer's attention: something didn't
/// decode, or fewer readings arrived than were asked for.
fn needs_attention(samples: &[SampleData], requested: usize, diagnostics: &[String]) -> bool {
    !diagnostics.is_empty()
        || samples.len() < requested
        || samples.iter().any(|s| s.mode.starts_with("Unknown("))
}

/// Run one capture step. Returns Ok(true) if user wants to quit.
pub(crate) fn run_capture_step(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
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

    recording::lock(recorder).set_step(Some(step.id));

    if let Some(cmd) = step.command {
        if let Err(e) = dmm.send_command(cmd) {
            eprintln!("  {}", style(format!("Command failed: {e}")).red());
            let mut result = step.empty_result(StepStatus::Error, Some(e.to_string()));
            let mut rec = recording::lock(recorder);
            rec.set_step(None);
            result.frames = frames_for_step(&rec.drain(), step.id, &mut result.diagnostics);
            result.needs_attention = true;
            upsert_step(report, result);
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let run = capture_samples(dmm, step.samples);
    let mut diagnostics = run.diagnostics;
    let sample_data: Vec<SampleData> = run
        .samples
        .iter()
        .map(SampleData::from_measurement)
        .collect();

    let frames = {
        let mut rec = recording::lock(recorder);
        rec.set_step(None);
        frames_for_step(&rec.drain(), step.id, &mut diagnostics)
    };

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

    let confirmation = if let Some(last) = sample_data.last() {
        if interactive {
            eprintln!("  We read: {}", style(last.summary()).green());
            Some(prompt(&format!(
                "  {} ",
                style("Enter=correct, or type what the meter actually shows:").dim()
            ))?)
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

    let mut result = StepResult {
        needs_attention: needs_attention(&sample_data, step.samples, &diagnostics),
        samples: sample_data,
        frames,
        diagnostics,
        ..StepResult::new(step.id, step.instruction, status)
    };
    if let Some(input) = confirmation {
        result.set_inline_confirmation(input);
    }

    upsert_step(report, result);
    report.wire_events_dropped = recording::lock(recorder).dropped();
    Ok(false)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use dmm_lib::measurement::MeasuredValue;
    // `capture_steps` is a Protocol method; the trait has to be in scope to
    // call it on a concrete protocol type (not needed for `dyn Protocol`).
    use dmm_lib::protocol::Protocol;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;
    use std::sync::Mutex;

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
            avg: true,
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

    /// Neither conversion may drop a flag: a set flag that reads false again
    /// after the round-trip is a capture report — and the summary line the
    /// operator confirms — quietly disagreeing with the meter.
    #[test]
    fn sample_flags_round_trip_through_status_flags() {
        // Every field spelled out, so a new flag has to be added here too.
        let f = StatusFlags {
            hold: true,
            rel: true,
            min: true,
            max: true,
            avg: true,
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
        let s = SampleFlags::from(&f);
        assert_eq!(StatusFlags::from(&s), f);
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

    /// The summary is the line the operator confirms against the meter's
    /// screen, so it has to name every flag the meter is showing. It listed
    /// only AUTO/HOLD/REL/MIN/MAX: @diego351's UT181A mains capture (issue #5)
    /// confirmed as `239.22 VAC [AUTO]` while the same sample recorded
    /// `hv_warning: true`.
    #[test]
    fn summary_names_every_flag_the_meter_set() {
        use dmm_lib::flags::StatusFlags;

        let mut m = make_test_measurement(0x02, 0x01, b"239.22 ", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.flags = StatusFlags {
            auto_range: true,
            hv_warning: true,
            lead_error: true,
            comp: true,
            record: true,
            loz: true,
            void: true,
            avg: true,
            low_battery: true,
            peak_max: true,
            ..Default::default()
        };
        let summary = SampleData::from_measurement(&m).summary();
        for expected in [
            "AUTO", "AVG", "LOW BAT", "HV!", "P-MAX", "LEAD ERR", "COMP", "REC", "LoZ", "VOID",
        ] {
            assert!(
                summary.contains(expected),
                "summary {summary:?} drops {expected}"
            );
        }
    }

    /// The confirmation line is what the operator compares against the
    /// meter's screen, so a multi-display meter's sub-values have to be on
    /// it — a UT181A showing frequency and period confirmed as the main
    /// reading alone.
    #[test]
    fn summary_lists_sub_values() {
        use dmm_lib::measurement::AuxValue;

        let mut m = make_test_measurement(0x02, 0x01, b"239.22 ", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.aux_values = vec![
            AuxValue {
                label: "Frequency".into(),
                value: MeasuredValue::Normal(50.01),
                unit: "Hz".into(),
                display_raw: Some("50.01".to_string()),
                elapsed_secs: None,
            },
            AuxValue {
                label: "Period".into(),
                value: MeasuredValue::Normal(20.0),
                unit: "ms".into(),
                display_raw: Some("20.00".to_string()),
                elapsed_secs: None,
            },
        ];
        assert_eq!(
            SampleData::from_measurement(&m).summary(),
            "239.22 V [AUTO] (Frequency 50.01 Hz, Period 20.00 ms)"
        );
    }

    /// MIN/MAX sub-values carry the time since the mode started; that is on
    /// the meter's screen, so it belongs on the line being confirmed.
    #[test]
    fn summary_shows_minmax_timestamps() {
        use dmm_lib::measurement::AuxValue;

        let mut m = make_test_measurement(0x02, 0x01, b"  5.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.aux_values = vec![AuxValue {
            label: "Max".into(),
            value: MeasuredValue::Normal(5.0123),
            // Empty: same quantity as the main reading, resolved on capture.
            unit: "".into(),
            display_raw: Some("5.0123".to_string()),
            elapsed_secs: Some(12),
        }];
        assert_eq!(
            SampleData::from_measurement(&m).summary(),
            "5.000 V [AUTO] (Max 5.0123 V @12s)"
        );
    }

    /// Single-display meters must confirm with exactly the line they did
    /// before sub-values were recorded.
    #[test]
    fn summary_without_sub_values_is_unchanged() {
        let m = make_test_measurement(0x02, 0x01, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(SampleData::from_measurement(&m).summary(), "1.000 V [AUTO]");
    }

    /// Resuming an interrupted capture reloads the report, so sub-values have
    /// to survive the round-trip — including the optional timestamp, which is
    /// omitted from the YAML when absent so single-display reports are
    /// unchanged.
    #[test]
    fn aux_samples_survive_a_yaml_roundtrip() {
        use dmm_lib::measurement::AuxValue;

        let mut m = make_test_measurement(0x02, 0x01, b"  5.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.aux_values = vec![
            AuxValue {
                label: "Frequency".into(),
                value: MeasuredValue::Normal(50.01),
                unit: "Hz".into(),
                display_raw: Some("50.01".to_string()),
                elapsed_secs: None,
            },
            AuxValue {
                label: "Max".into(),
                value: MeasuredValue::Overload,
                unit: "".into(),
                display_raw: None,
                elapsed_secs: Some(12),
            },
        ];
        let sample = SampleData::from_measurement(&m);

        let yaml = serde_yaml_ng::to_string(&sample).unwrap();
        assert_eq!(
            yaml.matches("elapsed_secs").count(),
            1,
            "elapsed_secs: None must be omitted: {yaml}"
        );

        let parsed: SampleData = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.aux.len(), 2);
        assert_eq!(parsed.aux[0].label, "Frequency");
        assert_eq!(parsed.aux[0].value, "50.01");
        assert_eq!(parsed.aux[0].unit, "Hz");
        assert_eq!(parsed.aux[0].elapsed_secs, None);
        assert_eq!(parsed.aux[1].value, "OL");
        // The empty "same as the main reading" unit is resolved on capture.
        assert_eq!(parsed.aux[1].unit, "V");
        assert_eq!(parsed.aux[1].elapsed_secs, Some(12));
        assert_eq!(parsed.summary(), sample.summary());
    }

    /// A report written before sub-values were recorded has no `aux` key.
    #[test]
    fn reports_without_aux_still_load() {
        let sample = SampleData::from_measurement(&make_test_measurement(
            0x02,
            0x01,
            b"  1.000",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ));
        let yaml = serde_yaml_ng::to_string(&sample).unwrap();
        assert!(!yaml.contains("aux"), "empty aux must be omitted: {yaml}");
        let parsed: SampleData = serde_yaml_ng::from_str(&yaml).unwrap();
        assert!(parsed.aux.is_empty());
    }

    /// A UT61E+ frame: AB CD, length, payload, 16-bit BE sum.
    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut f = vec![0xAB, 0xCD, (payload.len() + 2) as u8];
        f.extend_from_slice(payload);
        let sum = f.iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        f.extend_from_slice(&sum.to_be_bytes());
        f
    }

    fn measurement_frame() -> Vec<u8> {
        frame(&[
            0x02, 0x30, b' ', b' ', b'5', b'.', b'6', b'7', b'8', 0, 0, 0x30, 0x30, 0x30,
        ])
    }

    /// Canned transport: hands out one queued response per read, then goes
    /// silent the way a real meter does on timeout.
    struct QueuedTransport {
        responses: Mutex<std::collections::VecDeque<Vec<u8>>>,
    }

    impl dmm_lib::transport::Transport for QueuedTransport {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            let Some(next) = self.responses.lock().unwrap().pop_front() else {
                return Ok(0);
            };
            let n = next.len().min(buf.len());
            buf[..n].copy_from_slice(&next[..n]);
            Ok(n)
        }

        fn send_feature_report(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }
    }

    fn dmm_replaying(
        responses: Vec<Vec<u8>>,
    ) -> dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>> {
        let transport = QueuedTransport {
            responses: Mutex::new(responses.into()),
        };
        let device = dmm_lib::protocol::registry::find_device("ut61eplus").unwrap();
        dmm_lib::Dmm::new(
            Box::new(transport) as Box<dyn dmm_lib::transport::Transport>,
            (device.new_protocol)(),
        )
        .unwrap()
    }

    /// A rejected frame used to end the step, so the report carried neither a
    /// sample nor a reason. Keep polling and record why.
    #[test]
    fn capture_samples_reports_a_rejected_frame_and_keeps_polling() {
        let mut bad = measurement_frame();
        let last = bad.len() - 1;
        bad[last] = bad[last].wrapping_add(1);

        let mut dmm = dmm_replaying(vec![bad, measurement_frame()]);
        let run = capture_samples(&mut dmm, 1);

        assert_eq!(run.samples.len(), 1, "polling must continue past the error");
        assert_eq!(run.diagnostics.len(), 1);
        assert!(
            run.diagnostics[0].contains("checksum"),
            "got {:?}",
            run.diagnostics
        );
    }

    /// A meter stuck on bad frames would otherwise repeat one line per poll.
    #[test]
    fn repeated_errors_are_reported_once_with_a_count() {
        let mut bad = measurement_frame();
        let last = bad.len() - 1;
        bad[last] = bad[last].wrapping_add(1);

        let mut dmm = dmm_replaying(vec![bad.clone(), bad.clone(), bad]);
        let run = capture_samples(&mut dmm, 1);

        assert!(run.samples.is_empty());
        assert_eq!(run.diagnostics.len(), 1);
        assert!(
            run.diagnostics[0].ends_with("(x3)"),
            "got {:?}",
            run.diagnostics
        );
    }

    /// An unnamed mode is exactly what a capture is run to find, so the step
    /// has to be flagged rather than read as a clean one.
    #[test]
    fn an_unknown_mode_needs_attention() {
        let mut sample = SampleData::from_measurement(&make_test_measurement(
            0x02,
            0x01,
            b"  1.000",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ));
        assert!(!needs_attention(&[sample.clone()], 1, &[]));

        sample.mode = "Unknown(0x05)".to_string();
        assert!(needs_attention(&[sample.clone()], 1, &[]));
        // So do a parse rejection and a short step.
        assert!(needs_attention(&[], 1, &[]));
        assert!(needs_attention(
            &[sample],
            1,
            &["checksum mismatch".to_string()]
        ));
    }

    /// Only the events tagged with the step, and no more than the cap.
    #[test]
    fn step_frames_are_filtered_and_capped() {
        let event = |step: Option<&str>| WireEvent {
            at_ms: 0,
            dir: crate::recording::Direction::Rx,
            step: step.map(str::to_string),
            bytes: vec![0xAB, 0xCD],
            feature: false,
        };
        let mut events: Vec<WireEvent> = (0..MAX_FRAMES_PER_STEP + 3)
            .map(|_| event(Some("dcv")))
            .collect();
        events.push(event(Some("acv")));
        events.push(event(None));

        let mut diagnostics = Vec::new();
        let frames = frames_for_step(&events, "dcv", &mut diagnostics);
        assert_eq!(frames.len(), MAX_FRAMES_PER_STEP);
        assert_eq!(frames[0].hex, "AB CD");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].starts_with("3 further wire events"));
    }

    /// Resuming an interrupted capture reloads the report, so one written
    /// before wire recording existed must still parse.
    #[test]
    fn reports_without_the_wire_fields_still_load() {
        let yaml = "date: '2026-01-01'\ntool_version: test\ndevice_name: UT61E+\n\
                    supported: true\nsteps:\n- id: dcv\n  instruction: test\n  \
                    status: captured\n";
        let report: CaptureReport = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(report.steps.len(), 1);
        assert!(report.init_frames.is_empty());
        assert!(report.steps[0].frames.is_empty());
        assert!(report.steps[0].diagnostics.is_empty());
        assert!(!report.steps[0].needs_attention);
        assert_eq!(report.wire_events_dropped, 0);
        assert_eq!(report.device_id, None);
    }

    /// The new fields must stay out of a report that has nothing to put in
    /// them, so existing captures round-trip unchanged.
    #[test]
    fn empty_wire_fields_are_omitted() {
        let report = CaptureReport {
            steps: vec![StepResult::new("dcv", "test", StepStatus::Captured)],
            ..CaptureReport::default()
        };
        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        for key in [
            "frames",
            "diagnostics",
            "needs_attention",
            "wire_events_dropped",
            "device_id",
        ] {
            assert!(!yaml.contains(key), "{key} must be omitted: {yaml}");
        }
    }

    #[test]
    fn frame_records_serialize_with_a_lowercase_direction() {
        let record = FrameRecord {
            at_ms: 12,
            dir: FrameDir::Rx,
            hex: "AB CD".to_string(),
            feature: false,
        };
        let yaml = serde_yaml_ng::to_string(&record).unwrap();
        assert!(yaml.contains("dir: rx"), "got {yaml}");
        assert!(!yaml.contains("feature"), "got {yaml}");
    }

    /// Enter at the prompt means the meter agreed with what we read, so the
    /// step records the confirmation and no screen text.
    #[test]
    fn pressing_enter_confirms_the_reading() {
        let mut step = StepResult::new("dcv", "test", StepStatus::Captured);
        step.set_inline_confirmation(String::new());
        assert_eq!(step.confirmed, Some(true));
        assert_eq!(step.lcd, None);
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
    }

    /// A typed correction is what the meter actually showed — the evidence
    /// that our parse is wrong — so it is kept apart from the confirmation.
    #[test]
    fn a_typed_correction_records_what_the_meter_showed() {
        let mut step = StepResult::new("dcv", "test", StepStatus::Captured);
        step.set_inline_confirmation("5.68 V".to_string());
        assert_eq!(step.confirmed, Some(false));
        assert_eq!(step.lcd.as_deref(), Some("5.68 V"));
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
    }

    /// Reports written before the split store the confirmation as free text.
    /// Resuming one has to recover both cases, or the operator is asked to
    /// confirm steps they already confirmed.
    #[test]
    fn legacy_screen_text_normalizes_into_the_new_fields() {
        let mut step: StepResult = serde_yaml_ng::from_str(
            "id: dcv\ninstruction: test\nstatus: captured\nscreen: 'confirmed: 5.678 V [AUTO]'\n",
        )
        .unwrap();
        step.normalize_legacy();
        assert_eq!(step.confirmed, Some(true));
        assert_eq!(step.lcd, None);
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
        assert_eq!(step.screen, None);

        let mut step: StepResult = serde_yaml_ng::from_str(
            "id: dcv\ninstruction: test\nstatus: captured\nscreen: 5.68 V\n",
        )
        .unwrap();
        step.normalize_legacy();
        assert_eq!(step.confirmed, Some(false));
        assert_eq!(step.lcd.as_deref(), Some("5.68 V"));
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
        assert_eq!(step.screen, None);
    }

    /// A step nobody was asked about — a non-interactive run, or one with no
    /// samples — serializes exactly as it did before the fields existed.
    #[test]
    fn unconfirmed_steps_omit_the_confirmation_fields() {
        let yaml = serde_yaml_ng::to_string(&StepResult::new("dcv", "test", StepStatus::Captured))
            .unwrap();
        for key in ["confirmed", "lcd", "confirmed_by", "screen"] {
            assert!(!yaml.contains(key), "{key} must be omitted: {yaml}");
        }
    }

    #[test]
    fn upsert_step_insert() {
        let mut report = CaptureReport::default();
        let result = StepResult::new("dcv", "test", StepStatus::Captured);
        upsert_step(&mut report, result);
        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].id, "dcv");
    }

    #[test]
    fn upsert_step_replace() {
        let mut report = CaptureReport::default();
        let result1 = StepResult::new("dcv", "first", StepStatus::Skipped);
        upsert_step(&mut report, result1);

        let result2 = StepResult::new("dcv", "replaced", StepStatus::Captured);
        upsert_step(&mut report, result2);

        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].instruction, "replaced");
        assert_eq!(report.steps[0].status, StepStatus::Captured);
    }

    #[test]
    fn upsert_step_multiple_ids() {
        let mut report = CaptureReport::default();
        for id in ["dcv", "acv", "ohm"] {
            upsert_step(&mut report, StepResult::new(id, id, StepStatus::Captured));
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
                samples: vec![sample],
                confirmed: Some(false),
                lcd: Some("5.68 V".to_string()),
                confirmed_by: Some(ConfirmedBy::Inline),
                ..StepResult::new("dcv", "Set meter to DC V", StepStatus::Captured)
            }],
            ..CaptureReport::default()
        };

        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        let parsed: CaptureReport = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.date, report.date);
        assert_eq!(parsed.device_name, report.device_name);
        assert!(parsed.supported);
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.steps[0].samples.len(), 1);
        assert_eq!(parsed.steps[0].samples[0].value, "5.678");
        assert_eq!(parsed.steps[0].confirmed, Some(false));
        assert_eq!(parsed.steps[0].lcd.as_deref(), Some("5.68 V"));
        assert_eq!(parsed.steps[0].confirmed_by, Some(ConfirmedBy::Inline));
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
            steps: vec![StepResult::new("dcv", "test", StepStatus::Skipped)],
            ..CaptureReport::default()
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
            ..CaptureReport::default()
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

/// Samples taken per freeform capture.
const FREEFORM_SAMPLES: usize = 3;

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
            Ok(mut r) => {
                for step in &mut r.steps {
                    step.normalize_legacy();
                }
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
    recorder: &SharedRecorder,
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

        let step_id = format!("extra_{extra}");
        recording::lock(recorder).set_step(Some(&step_id));
        let run = capture_samples(dmm, FREEFORM_SAMPLES);
        let mut diagnostics = run.diagnostics;
        let sample_data: Vec<SampleData> = run
            .samples
            .iter()
            .map(SampleData::from_measurement)
            .collect();
        let frames = {
            let mut rec = recording::lock(recorder);
            rec.set_step(None);
            frames_for_step(&rec.drain(), &step_id, &mut diagnostics)
        };

        for (i, s) in sample_data.iter().enumerate() {
            eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
        }

        let confirmation = if let Some(last) = sample_data.last() {
            Some(prompt(&format!(
                "  We read: {}\n  Enter=correct, or type correction: ",
                last.summary()
            ))?)
        } else {
            eprintln!("  No response from meter.");
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
            diagnostics,
            ..StepResult::new(&step_id, &desc, status)
        };
        if let Some(input) = confirmation {
            result.set_inline_confirmation(input);
        }
        upsert_step(report, result);
        report.wire_events_dropped = recording::lock(recorder).dropped();
        save_report(report, output_path)?;
        extra += 1;
    }

    Ok(())
}

pub(crate) fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    mut dmm: dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: SharedRecorder,
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
    report.device_id = Some(device.id.to_string());
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
    let done = run_protocol_capture(
        &mut dmm,
        &recorder,
        protocol_steps,
        &step_filter,
        &mut report,
        &output_path,
    )?;

    if !done {
        run_freeform_captures(&mut dmm, &recorder, &step_filter, &mut report, &output_path)?;
    }

    report.wire_events_dropped = recording::lock(&recorder).dropped();
    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    eprintln!("Please attach this file to your bug report or issue.");
    Ok(())
}
