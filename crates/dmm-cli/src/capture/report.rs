//! The capture report: its schema and serde, the trust tier the gate rules
//! on, and the report file the run reads back and writes out.

use super::input::Input;
use super::step::CaptureStep;
use crate::watch::Baseline;
use console::style;
use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::Measurement;
use serde::{Deserialize, Serialize};

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
    /// The plan file whose steps this run followed, in place of the device's
    /// own list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub plan: Option<String>,
    /// Wire events recorded before the first step: the init handshake and the
    /// name query.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub init_frames: Vec<FrameRecord>,
    /// The meter never answered the check before the first step: the report
    /// has no steps, and `init_frames` is everything the cable delivered.
    #[serde(skip_serializing_if = "is_false", default)]
    pub no_response: bool,
    /// The run was `--unverified`, so verified steps are absent by request
    /// rather than skipped by the operator.
    #[serde(skip_serializing_if = "is_false", default)]
    pub unverified_only: bool,
    /// Wire events lost to the recorder's bound, so a truncated trace is
    /// visible as one.
    #[serde(skip_serializing_if = "is_zero", default)]
    pub wire_events_dropped: u64,
    /// How far the run trusted the parser by the end of it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tier: Option<Tier>,
    /// Whether the gate steps agreed with the meter, so a reader knows
    /// whether the readings below rest on a decoder that got the basics
    /// right. Absent when the gate never finished.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub core_semantics: Option<CoreSemantics>,
    /// The gate steps that did not confirm, which is where to start reading.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub gate_failures: Vec<String>,
    /// Whether the run walked the settings itself after each mode step, so a
    /// report with no sub-steps says why it has none.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub drive: Option<crate::drive::Drive>,
    pub steps: Vec<StepResult>,
}

/// How much of what the parser says the run takes on trust, which decides how
/// each step is detected and when the operator is asked about it.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tier {
    /// Nothing: steps advance on raw byte changes and every one is confirmed.
    Sniff,
    /// The core semantics are unproven, so the gate steps decide.
    Gate,
    /// Digits, OL and sign decode correctly, so the rest is reviewed in one
    /// pass at the end.
    Trusted,
}

/// What the gate steps said about the family's core semantics.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CoreSemantics {
    Confirmed,
    Failed,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

fn is_false(b: &bool) -> bool {
    !*b
}

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
    /// Wire events the per-step cap trimmed, oldest first. Normal on a step
    /// that waited for the operator, so it is a count and not a diagnostic.
    #[serde(skip_serializing_if = "is_zero", default)]
    pub frames_dropped: u64,
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
            frames_dropped: 0,
            diagnostics: vec![],
            needs_attention: false,
        }
    }

    /// Record the operator's answer to the confirmation prompt: empty input
    /// means our reading matched, anything else is what the meter showed.
    pub(super) fn set_inline_confirmation(&mut self, input: String) {
        let confirmed = input.is_empty();
        self.confirmed = Some(confirmed);
        self.lcd = (!confirmed).then_some(input);
        self.confirmed_by = Some(ConfirmedBy::Inline);
    }

    /// Record the end-of-run review's verdict: `lcd` is what the meter showed,
    /// given only for the readings the operator listed as wrong.
    pub(super) fn set_batch_confirmation(&mut self, lcd: Option<String>) {
        self.confirmed = Some(lcd.is_none());
        self.lcd = lcd.filter(|text| !text.is_empty());
        self.confirmed_by = Some(ConfirmedBy::Batch);
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
    pub(super) tier: Tier,
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
    pub(super) fn expect(&self, step: &CaptureStep) -> Option<dmm_lib::protocol::Expect> {
        match self.tier {
            Tier::Sniff => None,
            Tier::Gate | Tier::Trusted => step.expect,
        }
    }

    /// Whether the run may drive the meter's settings itself: only once the
    /// gate has shown that mode, range and flags read back correctly.
    pub(super) fn drives(&self) -> bool {
        self.tier == Tier::Trusted
    }

    /// Whether the step is confirmed at the step itself. Gate steps always
    /// are — the rest of the run's confirmations rest on them.
    pub(super) fn confirm_inline(&self, step: &CaptureStep) -> bool {
        step.gate || self.tier != Tier::Trusted
    }

    /// Rule on the gate once every one of its steps has reported, and promote
    /// the run if the parser got the core semantics right.
    pub(super) fn update(&mut self, report: &mut CaptureReport) {
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
                style("Core semantics confirmed \u{2014} remaining steps are reviewed at the end.")
                    .green()
            );
        } else {
            report.core_semantics = Some(CoreSemantics::Failed);
            eprintln!(
                "{}",
                style(format!(
                    "Core semantics not confirmed ({}) \u{2014} every step will ask for confirmation.",
                    failures.join(", ")
                ))
                .yellow()
            );
        }
        report.gate_failures = failures;
        report.tier = Some(self.tier);
    }
}

/// Whether the report already holds samples for the step, which is what a
/// resumed run leaves alone.
pub(super) fn already_captured(report: &CaptureReport, id: &str) -> bool {
    report
        .steps
        .iter()
        .any(|s| s.id == id && s.status == StepStatus::Captured)
}

/// The baseline a captured step's samples describe.
pub(super) fn baseline_from_report(report: &CaptureReport, step_id: &str) -> Option<Baseline> {
    let step = report.steps.iter().find(|s| s.id == step_id)?;
    let payloads: Vec<Vec<u8>> = step
        .samples
        .iter()
        .filter_map(|s| hex_bytes(&s.raw_hex))
        .collect();
    Baseline::from_payloads(payloads.iter().map(Vec::as_slice))
}

/// "02 30 20" back to the payload it was written from.
fn hex_bytes(hex: &str) -> Option<Vec<u8>> {
    hex.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).ok())
        .collect()
}

pub(crate) fn save_report(
    report: &CaptureReport,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml_ng::to_string(report)?;
    // Atomic write (.tmp + fsync + rename), so a crash mid-write doesn't
    // corrupt the existing report.
    dmm_shared::write_atomic(std::path::Path::new(path), yaml.as_bytes())?;
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

/// Whether the step is worth a maintainer's attention: something didn't
/// decode, or fewer readings arrived than were asked for.
pub(crate) fn needs_attention(
    samples: &[SampleData],
    requested: usize,
    diagnostics: &[String],
) -> bool {
    !diagnostics.is_empty()
        || samples.len() < requested
        || samples.iter().any(|s| s.mode.starts_with("Unknown("))
}

/// A name reduced to what a file name can safely carry.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// The plan file's name without its directory or extension.
fn plan_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "plan".to_string())
}

/// Where the run's report goes: named after the registry id the run was
/// started for, as the no-response report already is. The meter's own name
/// would be "unknown" for every family that answers no name query, so a
/// UT804, a UT171 and a UT181A all wrote `capture-unknown.yaml`.
pub(super) fn capture_path(
    output_override: Option<String>,
    device_id: &str,
    plan_path: Option<&str>,
) -> String {
    output_override.unwrap_or_else(|| match plan_path {
        // A plan run covers a handful of steps nobody else asked for, so it
        // gets its own file rather than resuming into the full report.
        Some(plan) => format!(
            "capture-{}-{}.yaml",
            slug(device_id),
            slug(&plan_stem(plan))
        ),
        None => format!("capture-{}.yaml", slug(device_id)),
    })
}

/// Determine the output path and load an existing report (with resume/overwrite prompt)
/// or create a fresh one. Returns `None` if the user chose to abort.
pub(super) fn load_or_create_report(
    output_override: Option<String>,
    device_id: &str,
    plan_path: Option<&str>,
    input: &Input,
) -> Result<Option<(CaptureReport, String)>, Box<dyn std::error::Error>> {
    let output_path = capture_path(output_override, device_id, plan_path);

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
                let ch = input.key("r=resume, n=start fresh, q=abort: ")?;
                if ch == 'q' || ch == 'Q' {
                    eprintln!("Aborted.");
                    return Ok(None);
                }
                if ch == 'n' || ch == 'N' {
                    let confirm = input
                        .key("This will overwrite the existing capture. Are you sure? y/n: ")?;
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
                let ch = input.key("Overwrite? y=start fresh, any other key=abort: ")?;
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

/// Where the report of a meter that never answered goes: beside the path the
/// run would have written, so `-o` still picks the directory, under its own
/// name so a report in progress is never replaced by an empty one.
pub(super) fn no_response_path(
    output_override: Option<&str>,
    device_id: &str,
) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(
        output_override
            .map(str::to_string)
            .unwrap_or_else(|| format!("capture-{}.yaml", slug(device_id))),
    );
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{stem}-no-response.yaml"))
}

/// Populate report metadata (date, version, device info).
pub(super) fn populate_report_metadata(
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

/// How many of `ids` the report now holds samples for — the number the
/// epilogue reports as what this run is worth to the issue.
pub(super) fn captured_count(
    report: &CaptureReport,
    ids: &std::collections::HashSet<&str>,
) -> usize {
    report
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Captured && ids.contains(s.id.as_str()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::step::cli_step;
    use crate::watch::{STABLE_FRAMES, StateWatcher, Verdict};
    use dmm_lib::measurement::MeasuredValue;
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

    /// The next step tells a new state from the last captured one, and after
    /// a resume that state comes back out of the report's hex — the samples
    /// are all that is left of the step the run skipped as already captured.
    #[test]
    fn the_baseline_comes_back_from_the_reports_hex() {
        let mut report = CaptureReport::default();
        let samples = [b"  1.234", b"  1.298", b"  1.351"].map(|digits| {
            SampleData::from_measurement(&make_test_measurement(
                0x02,
                0x01,
                digits,
                (0x00, 0x00),
                (0x00, 0x00, 0x00),
            ))
        });
        upsert_step(
            &mut report,
            StepResult {
                samples: samples.to_vec(),
                ..StepResult::new("dcv", "Set meter to DC V", StepStatus::Captured)
            },
        );

        let baseline = baseline_from_report(&report, "dcv").expect("samples give a baseline");
        let mut watcher = StateWatcher::for_step(None, Some(&baseline), true);
        // Same mode, other digits: not a new state.
        for _ in 0..STABLE_FRAMES {
            let m = make_test_measurement(0x02, 0x01, b"  1.999", (0x00, 0x00), (0x00, 0x00, 0x00));
            assert_eq!(watcher.feed(&m), Verdict::Waiting);
        }
        // AC V is.
        let acv = make_test_measurement(0x00, 0x01, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(watcher.feed(&acv), Verdict::Waiting);
        assert_eq!(watcher.feed(&acv), Verdict::Waiting);
        assert_eq!(watcher.feed(&acv), Verdict::Ready);

        assert!(baseline_from_report(&report, "acv").is_none());
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
            "tier",
            "core_semantics",
            "gate_failures",
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
        let report: CaptureReport = serde_yaml_ng::from_str(yaml).unwrap();
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
        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        assert!(yaml.contains("tier: trusted"), "{yaml}");
        assert!(yaml.contains("core_semantics: failed"), "{yaml}");
        let parsed: CaptureReport = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.tier, Some(Tier::Trusted));
        assert_eq!(parsed.gate_failures, vec!["ohm".to_string()]);
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

    /// Coverage counts captured steps only, and only ones that were unverified.
    #[test]
    fn coverage_counts_captured_unverified_steps() {
        let mut report = CaptureReport::default();
        upsert_step(
            &mut report,
            StepResult::new("temp", "t", StepStatus::Captured),
        );
        upsert_step(
            &mut report,
            StepResult::new("dcv", "d", StepStatus::Captured),
        );
        upsert_step(&mut report, StepResult::new("hz", "h", StepStatus::Skipped));
        let unverified: std::collections::HashSet<&str> = ["temp", "hz"].into_iter().collect();
        assert_eq!(captured_count(&report, &unverified), 1);
    }

    /// The families that answer no name query — UT803/UT804, UT171, UT181A —
    /// all reported themselves as "unknown", so every one of their runs wrote
    /// `capture-unknown.yaml` and resumed into the last meter's report.
    #[test]
    fn the_report_is_named_after_the_device_the_run_was_started_for() {
        assert_eq!(capture_path(None, "ut804", None), "capture-ut804.yaml");
        assert_eq!(
            capture_path(None, "ut804", Some("plans/hz-walk.yaml")),
            "capture-ut804-hz-walk.yaml"
        );
        assert_eq!(
            capture_path(Some("runs/bench.yaml".to_string()), "ut804", None),
            "runs/bench.yaml"
        );
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
