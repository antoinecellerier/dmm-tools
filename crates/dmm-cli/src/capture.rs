use crate::StepListFormat;
use crate::recording::{self, SharedRecorder, WireEvent};
use crate::watch::{Baseline, STABLE_FRAMES, StateWatcher, Verdict, enter_only};
use console::{Key, style};
use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::Need;
use dmm_lib::protocol::registry::SelectableDevice;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

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
    /// The plan file whose steps this run followed, in place of the device's
    /// own list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub plan: Option<String>,
    /// Wire events recorded before the first step: the init handshake and the
    /// name query.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub init_frames: Vec<FrameRecord>,
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
    fn set_inline_confirmation(&mut self, input: String) {
        let confirmed = input.is_empty();
        self.confirmed = Some(confirmed);
        self.lcd = (!confirmed).then_some(input);
        self.confirmed_by = Some(ConfirmedBy::Inline);
    }

    /// Record the end-of-run review's verdict: `lcd` is what the meter showed,
    /// given only for the readings the operator listed as wrong.
    fn set_batch_confirmation(&mut self, lcd: Option<String>) {
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
    tier: Tier,
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
    fn expect(&self, step: &CaptureStep) -> Option<dmm_lib::protocol::Expect> {
        match self.tier {
            Tier::Sniff => None,
            Tier::Gate | Tier::Trusted => step.expect,
        }
    }

    /// Whether the run may drive the meter's settings itself: only once the
    /// gate has shown that mode, range and flags read back correctly.
    fn drives(&self) -> bool {
        self.tier == Tier::Trusted
    }

    /// Whether the step is confirmed at the step itself. Gate steps always
    /// are — the rest of the run's confirmations rest on them.
    fn confirm_inline(&self, step: &CaptureStep) -> bool {
        step.gate || self.tier != Tier::Trusted
    }

    /// Rule on the gate once every one of its steps has reported, and promote
    /// the run if the parser got the core semantics right.
    fn update(&mut self, report: &mut CaptureReport) {
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

/// What the protocol pass left behind: whether the operator asked to finish,
/// and the steps captured without a confirmation.
struct ProtocolPass {
    quit: bool,
    to_review: Vec<String>,
}

/// Run the device's own capture steps: modes, flags, and the manual range
/// sweep, in the order the protocol declares them.
#[allow(clippy::too_many_arguments)]
fn run_protocol_capture(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    all_steps: &[CaptureStep],
    step_filter: &Option<std::collections::HashSet<String>>,
    unverified_only: bool,
    report: &mut CaptureReport,
    output_path: &str,
    input: &Input,
    trust: &mut Trust,
    driver: &mut crate::drive::Driver,
) -> Result<ProtocolPass, Box<dyn std::error::Error>> {
    // Keep only what this run selects: the checklist below covers the steps
    // that will actually run.
    let mut steps: Vec<CaptureStep> = all_steps
        .iter()
        .copied()
        .filter(|s| step_selected(s, step_filter, unverified_only))
        .collect();

    // Only what is still to do: a resumed run must not ask for the equipment
    // of a step it already has samples for, nor overwrite that step's result.
    let pending: Vec<CaptureStep> = steps
        .iter()
        .filter(|s| !already_captured(report, s.id))
        .copied()
        .collect();
    let missing = ask_missing_needs(&pending, input)?;
    let dropped = steps_without(&pending, &missing);
    for (step, label) in &dropped {
        upsert_step(
            report,
            step.empty_result(StepStatus::Skipped, Some(format!("skipped: no {label}"))),
        );
    }
    if !dropped.is_empty() {
        let ids: Vec<&str> = dropped.iter().map(|(s, _)| s.id).collect();
        steps.retain(|s| !ids.contains(&s.id));
        save_report(report, output_path)?;
    }

    eprintln!(
        "{}",
        style("\u{2501}\u{2501}\u{2501} Measurement Modes \u{2501}\u{2501}\u{2501}").bold()
    );
    eprintln!(
        "{}",
        style("each step captures itself once the meter settles \u{2014} Enter=capture now, s=skip one, q=skip to end and save").dim()
    );

    // A resumed run inherits the gate its earlier half already passed.
    trust.update(report);

    // What the meter was left showing, so the next step can tell a new state
    // from it.
    let mut prev = PrevState::default();
    let mut to_review = Vec::new();
    for step in &steps {
        let outcome = run_capture_step(
            dmm, recorder, step, report, true, input, &prev, trust, driver,
        )?;
        if outcome.to_review {
            to_review.push(step.id.to_string());
        }
        trust.update(report);
        // After the gate, and only for a step the operator set by hand: a
        // command step's own flag is what the sweep would be undoing. A step
        // the report already holds captures nothing, so a resumed run does
        // not sweep it — the dial is no longer where its sub-steps assume.
        if trust.drives()
            && !step.gate
            && step.command.is_none()
            && let Some(last) = &outcome.last
        {
            crate::drive::sweep_step(dmm, recorder, step, last, driver, report, output_path)?;
        }
        prev.last = outcome.last;
        // From the report, so a resumed run gets its baseline from the steps
        // it skipped as already captured. A step that captured nothing leaves
        // the previous baseline standing: it is still the last state the
        // meter was seen in.
        if let Some(next) = baseline_from_report(report, step.id) {
            prev.baseline = Some(next);
        }
        save_report(report, output_path)?;
        if outcome.quit {
            return Ok(ProtocolPass {
                quit: true,
                to_review,
            });
        }
    }

    Ok(ProtocolPass {
        quit: false,
        to_review,
    })
}

/// Whether the report already holds samples for the step, which is what a
/// resumed run leaves alone.
fn already_captured(report: &CaptureReport, id: &str) -> bool {
    report
        .steps
        .iter()
        .any(|s| s.id == id && s.status == StepStatus::Captured)
}

/// The baseline a captured step's samples describe.
fn baseline_from_report(report: &CaptureReport, step_id: &str) -> Option<Baseline> {
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

// --- Step definitions ---

#[derive(Clone, Copy)]
pub(crate) struct CaptureStep {
    pub id: &'static str,
    pub instruction: &'static str,
    pub command: Option<&'static str>,
    pub samples: usize,
    /// What a correct reading looks like once the instruction is carried out;
    /// `None` leaves the step watching for any new state.
    pub expect: Option<dmm_lib::protocol::Expect>,
    /// Already confirmed on real hardware — `--unverified` skips these.
    pub verified: bool,
    /// One of the steps the family's core semantics rest on; flagged in the
    /// run so an operator knows which ones must not be skipped.
    pub gate: bool,
    /// Equipment the instruction asks for, listed before the run so a step
    /// nobody can do is dropped rather than met halfway through.
    pub needs: &'static [Need],
}

impl From<&dmm_lib::protocol::CaptureStep> for CaptureStep {
    fn from(ps: &dmm_lib::protocol::CaptureStep) -> Self {
        CaptureStep {
            id: ps.id,
            instruction: ps.instruction,
            command: ps.command,
            samples: ps.samples,
            expect: ps.expect,
            verified: ps.verified,
            gate: ps.gate,
            needs: ps.needs,
        }
    }
}

/// The equipment the given steps ask for, numbered in `Need::ALL` order, and
/// the needs behind those numbers. `None` when the run needs nothing beyond
/// the meter and its leads.
fn render_needs(steps: &[CaptureStep]) -> Option<(String, Vec<Need>)> {
    let mut needs = Vec::new();
    let mut lines = String::new();
    for need in Need::ALL {
        let ids: Vec<&str> = steps
            .iter()
            .filter(|s| s.needs.contains(&need))
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            continue;
        }
        needs.push(need);
        lines.push_str(&format!(
            "  [{}] {}  (steps: {})\n",
            needs.len(),
            need.label(),
            ids.join(", ")
        ));
    }
    (!needs.is_empty()).then(|| (format!("You will need:\n{lines}"), needs))
}

/// The steps waiting on something the operator hasn't got, each with the item
/// it was waiting on.
fn steps_without<'a>(
    steps: &'a [CaptureStep],
    missing: &[Need],
) -> Vec<(&'a CaptureStep, &'static str)> {
    steps
        .iter()
        .filter_map(|s| {
            let need = missing.iter().find(|n| s.needs.contains(n))?;
            Some((s, need.label()))
        })
        .collect()
}

/// Show what the run needs and ask which of it is missing, so the bench is set
/// up once instead of a thermocouple turning up as a step in the middle.
///
/// A run with nobody to ask attempts everything: the steps are still worth
/// offering to a meter that might be sitting on the right source already.
fn ask_missing_needs(
    steps: &[CaptureStep],
    input: &Input,
) -> Result<Vec<Need>, Box<dyn std::error::Error>> {
    let Some((table, needs)) = render_needs(steps) else {
        return Ok(Vec::new());
    };
    eprintln!();
    eprint!("{table}");
    if !input.is_tty() {
        return Ok(Vec::new());
    }
    let missing = loop {
        let answer =
            input.line("Numbers of anything you don't have (Enter = have everything): ")?;
        match parse_review_indices(&answer, needs.len()) {
            Ok(indices) => break indices,
            Err(e) => eprintln!("  {}", style(e).yellow()),
        }
    };
    Ok(missing.into_iter().map(|i| needs[i]).collect())
}

impl CaptureStep {
    /// The line announcing the step. Gate steps say so: skipping one leaves
    /// the rest of the run uninterpretable.
    fn header(&self) -> String {
        let id = style(format!("[{}]", self.id)).cyan().bold();
        let gate = if self.gate {
            format!(" {}", style("(gate)").dim())
        } else {
            String::new()
        };
        format!("{id} {}{gate}", self.instruction)
    }

    /// Create a StepResult with no samples or screen capture.
    fn empty_result(&self, status: StepStatus, error: Option<String>) -> StepResult {
        StepResult {
            error,
            ..StepResult::new(self.id, self.instruction, status)
        }
    }
}

// --- Helpers ---

/// The one message for a keyboard reader that has gone away, so a dead thread
/// ends the run with a reason instead of hanging on a channel nobody feeds.
fn input_gone() -> Box<dyn std::error::Error> {
    "keyboard input stopped working; finish the capture and rerun".into()
}

/// The capture run's keyboard.
///
/// `Term::read_key` blocks, so it runs on its own thread and the watcher polls
/// the channel between readings. Every prompt goes through here too: a second
/// reader would race this one for the operator's keystrokes.
pub(crate) struct Input {
    /// `None` when stderr is not a terminal (a piped run): `read_key` needs
    /// one, so input falls back to whole lines from stdin.
    keys: Option<Receiver<Key>>,
}

impl Input {
    pub(crate) fn start() -> Self {
        if !console::Term::stderr().is_term() {
            return Input { keys: None };
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            // catch_unwind: a panic here must close the channel rather than
            // leave every later prompt waiting on a thread that is gone.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let term = console::Term::stderr();
                while let Ok(key) = term.read_key() {
                    if tx.send(key).is_err() {
                        break;
                    }
                }
            }));
        });
        Input { keys: Some(rx) }
    }

    /// Whether keys can be polled without blocking — false for a piped run,
    /// which has to be asked rather than watched.
    pub(crate) fn is_tty(&self) -> bool {
        self.keys.is_some()
    }

    /// The key waiting, if any. Never blocks, so the watcher keeps reading.
    pub(crate) fn try_key(&self) -> Result<Option<Key>, Box<dyn std::error::Error>> {
        let Some(keys) = &self.keys else {
            return Ok(None);
        };
        match keys.try_recv() {
            Ok(key) => Ok(Some(key)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(input_gone()),
        }
    }

    /// Ask for one keystroke. Enter reads as `'\n'`, as it did when this was a
    /// direct `read_char`.
    pub(crate) fn key(&self, msg: &str) -> Result<char, Box<dyn std::error::Error>> {
        eprint!("{msg}");
        std::io::stderr().flush()?;
        let Some(keys) = &self.keys else {
            // No terminal: a whole line, of which the first character answers.
            let line = read_stdin_line()?;
            return Ok(line.chars().next().unwrap_or('\n'));
        };
        loop {
            match keys.recv().map_err(|_| input_gone())? {
                Key::Char(c) => {
                    eprintln!();
                    return Ok(c);
                }
                Key::Enter => {
                    eprintln!();
                    return Ok('\n');
                }
                // Arrows and the like: keep waiting, as `read_char` did.
                _ => {}
            }
        }
    }

    /// Ask for a line, echoing it: the reader thread holds the terminal in raw
    /// mode, so nothing else will.
    pub(crate) fn line(&self, msg: &str) -> Result<String, Box<dyn std::error::Error>> {
        eprint!("{msg}");
        std::io::stderr().flush()?;
        let Some(keys) = &self.keys else {
            return read_stdin_line();
        };
        let mut out = String::new();
        loop {
            match keys.recv().map_err(|_| input_gone())? {
                Key::Enter => {
                    eprintln!();
                    return Ok(out.trim().to_string());
                }
                Key::Backspace if out.pop().is_some() => {
                    eprint!("\u{8} \u{8}");
                    std::io::stderr().flush()?;
                }
                Key::Char(c) => {
                    out.push(c);
                    eprint!("{c}");
                    std::io::stderr().flush()?;
                }
                _ => {}
            }
        }
    }
}

fn read_stdin_line() -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Parse rejections seen while a step ran, in first-seen order with a repeat
/// count: a stuck meter otherwise fills the report with the same line.
#[derive(Default)]
pub(crate) struct ErrorLog {
    entries: Vec<(String, usize)>,
}

impl ErrorLog {
    /// Echo the rejection the first time it appears — a step that waits for a
    /// state can see hundreds of them.
    fn record(&mut self, e: &dmm_lib::error::Error) {
        let text = e.to_string();
        match self.entries.iter_mut().find(|(t, _)| *t == text) {
            Some((_, count)) => *count += 1,
            None => {
                eprintln!("  error: {text}");
                self.entries.push((text, 1));
            }
        }
    }

    pub(crate) fn into_diagnostics(self) -> Vec<String> {
        self.entries
            .into_iter()
            .map(|(text, count)| {
                if count > 1 {
                    format!("{text} (x{count})")
                } else {
                    text
                }
            })
            .collect()
    }
}

/// Poll for `n` samples.
///
/// A parse rejection is recorded and polling continues: a meter whose frames
/// we misread would otherwise end the step on the first bad frame, and the
/// report showed neither a sample nor a reason.
pub(crate) fn capture_samples(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    n: usize,
    errors: &mut ErrorLog,
) -> Vec<Measurement> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => errors.record(&e),
        }
        attempts += 1;
    }
    samples
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

/// Wire events belonging to `step_id`, capped, and how many older ones the
/// cap trimmed.
///
/// The newest are kept: a step spends its wait watching the meter, and the
/// frames worth reading are the sampled ones at the end of it. The count is
/// returned rather than written into `diagnostics` because a step that waits
/// on a person passes the cap as a matter of course, and every diagnostic
/// flags the step for a maintainer to read.
pub(crate) fn frames_for_step(events: &[WireEvent], step_id: &str) -> (Vec<FrameRecord>, u64) {
    let mine: Vec<&WireEvent> = events
        .iter()
        .filter(|e| e.step.as_deref() == Some(step_id))
        .collect();
    let dropped = mine.len().saturating_sub(MAX_FRAMES_PER_STEP);
    (
        mine.into_iter()
            .skip(dropped)
            .map(FrameRecord::from)
            .collect(),
        dropped as u64,
    )
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

/// How long a step watches the meter before offering the keyboard: long
/// enough to fetch a thermocouple, short enough not to look stuck.
const STEP_TIMEOUT: Duration = Duration::from_secs(45);

/// How long a command step waits for the meter to react before calling the
/// command a no-op. The meter answers a button in a frame or two.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);

/// What the step before this one left behind.
#[derive(Default)]
pub(crate) struct PrevState {
    /// The payload bytes it held constant, so a new state can be told from
    /// it. Read back from the report, so a resumed run has one too.
    pub baseline: Option<Baseline>,
    /// Its last reading. In-memory only: `None` after a resume, and after a
    /// step the operator skipped, where the meter's state is anyone's guess.
    pub last: Option<Measurement>,
}

/// What a step left behind: whether the operator asked to finish, and the
/// reading the next step measures its own expectation against.
pub(crate) struct StepOutcome {
    quit: bool,
    last: Option<Measurement>,
    /// It captured a reading nobody was asked about, so the end-of-run review
    /// has to cover it.
    to_review: bool,
}

impl StepOutcome {
    /// The step is done and the run goes on; `last` where it captured one.
    fn done(last: Option<Measurement>, to_review: bool) -> Self {
        StepOutcome {
            quit: false,
            last,
            to_review,
        }
    }

    /// The step captured nothing — skipped, refused or ignored — so the next
    /// one has no reading to compare against.
    fn nothing(quit: bool) -> Self {
        StepOutcome {
            quit,
            last: None,
            to_review: false,
        }
    }
}

/// Why the wait for a step's state ended.
enum Watched {
    /// The state is on screen; the frame that clinched it, unless the
    /// operator's Enter cut the wait short.
    Ready(Option<Measurement>),
    /// Nothing new settled within the timeout; the last reading, for the
    /// report to say what the meter was showing instead.
    TimedOut(Option<Measurement>),
    Skip,
    Quit,
}

/// Read until the meter shows what the step asked for.
///
/// `hint` offers the keyboard once the timeout passes rather than giving up:
/// a step whose instruction takes a while to carry out must not file whatever
/// the meter happened to be showing.
fn watch_for_state(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    input: &Input,
    watcher: &mut StateWatcher,
    timeout: Duration,
    hint: bool,
    errors: &mut ErrorLog,
) -> Result<Watched, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut hinted = false;
    let mut last = None;
    loop {
        match input.try_key()? {
            Some(Key::Enter) => return Ok(Watched::Ready(None)),
            Some(Key::Char('s' | 'S')) => return Ok(Watched::Skip),
            Some(Key::Char('q' | 'Q')) => return Ok(Watched::Quit),
            _ => {}
        }

        match dmm.request_measurement() {
            Ok(m) => match watcher.feed(&m) {
                Verdict::Ready => return Ok(Watched::Ready(Some(m))),
                Verdict::Mismatch(reason) => {
                    last = Some(m);
                    eprintln!("  {}", style(format!("meter shows: {reason}")).dim());
                }
                Verdict::Waiting => last = Some(m),
            },
            Err(dmm_lib::error::Error::Timeout) => {}
            Err(e) => {
                errors.record(&e);
                watcher.feed_error();
            }
        }

        // `checked_duration_since`: a backward clock jump must not strand the
        // step in a wait that never times out.
        let waited = Instant::now()
            .checked_duration_since(start)
            .unwrap_or_default();
        if waited >= timeout {
            if !hint {
                return Ok(Watched::TimedOut(last));
            }
            if !hinted {
                hinted = true;
                eprintln!(
                    "  {}",
                    style("Press Enter when the meter is ready (s=skip, q=finish)").dim()
                );
            }
        }
    }
}

/// The wording for a command the meter ignored, in the terms `cycle.rs` uses
/// for the same failure.
fn did_nothing(command: &str, last: Option<&Measurement>) -> String {
    match last {
        Some(m) => format!(
            "{command} did nothing; the meter still shows {}",
            SampleData::from_measurement(m).summary()
        ),
        None => format!("{command} did nothing; the meter sent no reading"),
    }
}

/// What the operator typed at a step's inline confirmation prompt.
#[derive(Debug, PartialEq, Eq)]
enum Confirmation {
    /// Empty means the reading matched; anything else is what the LCD showed.
    Answer(String),
    /// Do the step over: the attempt's samples are dropped.
    Retake,
}

impl Confirmation {
    /// `r` on its own is the retake key — no meter shows a bare "r", and a
    /// typed correction that starts with one still reads as a correction.
    fn of(answer: String) -> Self {
        if answer.eq_ignore_ascii_case("r") {
            Confirmation::Retake
        } else {
            Confirmation::Answer(answer)
        }
    }
}

/// Run one capture step. Returns Ok(true) if user wants to quit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_capture_step(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step: &CaptureStep,
    report: &mut CaptureReport,
    interactive: bool,
    input: &Input,
    prev: &PrevState,
    trust: &Trust,
    driver: &mut crate::drive::Driver,
) -> Result<StepOutcome, Box<dyn std::error::Error>> {
    // Check if already captured (resume)
    if already_captured(report, step.id) {
        eprintln!("  {} already captured, skipping", style(step.id).dim());
        return Ok(StepOutcome::nothing(false));
    }

    if interactive {
        eprintln!();
    }
    eprintln!("{}", step.header());

    recording::lock(recorder).set_step(Some(step.id));
    let mut errors = ErrorLog::default();
    let expect = trust.expect(step);

    // A mode a button on the meter reaches — continuity from Ω, duty
    // from Hz — is the tool's to switch to; only the dial is the operator's.
    if interactive
        && trust.drives()
        && let Some(last) = &prev.last
    {
        crate::drive::switch_mode(dmm, step, last, driver)?;
    }

    // One pass per attempt: `r` at the confirmation prompt drops the samples
    // and runs the same wait again, on the same previous state. The recorder
    // is not drained in between, so every attempt's frames reach the report.
    let mut attempt = 0usize;
    let (sample_data, mut measurements, confirmation) = loop {
        attempt += 1;
        // The frame the watcher accepted, kept as the step's first sample: it is
        // the one reading known to be in the state the step asked for.
        let mut settled: Option<Measurement> = None;

        // A retake re-samples; it does not press the button again, which on a
        // toggle like HOLD would undo the state the first press reached.
        if let Some(cmd) = step.command.filter(|_| attempt == 1) {
            // What the meter shows before the button is pressed, so a command
            // that changes nothing can be told from one that works.
            let before = capture_samples(dmm, STABLE_FRAMES, &mut errors);
            let before = Baseline::from_payloads(before.iter().map(|m| m.raw_payload.as_slice()));

            if let Err(e) = dmm.send_command(cmd) {
                eprintln!("  {}", style(format!("Command failed: {e}")).red());
                let mut result = step.empty_result(StepStatus::Error, Some(e.to_string()));
                let mut rec = recording::lock(recorder);
                rec.set_step(None);
                (result.frames, result.frames_dropped) = frames_for_step(&rec.drain(), step.id);
                result.needs_attention = true;
                upsert_step(report, result);
                return Ok(StepOutcome::nothing(false));
            }

            let mut watcher = StateWatcher::for_step(expect, before.as_ref(), true);
            match watch_for_state(
                dmm,
                input,
                &mut watcher,
                COMMAND_TIMEOUT,
                false,
                &mut errors,
            )? {
                Watched::Ready(m) => settled = m,
                Watched::TimedOut(last) => {
                    // No samples: filing pre-command frames as the step's result
                    // is what made a dead command look like a captured state.
                    let error = did_nothing(cmd, last.as_ref());
                    eprintln!("  {}", style(&error).yellow());
                    let mut result = step.empty_result(StepStatus::Error, Some(error));
                    let mut rec = recording::lock(recorder);
                    rec.set_step(None);
                    result.diagnostics = errors.into_diagnostics();
                    (result.frames, result.frames_dropped) = frames_for_step(&rec.drain(), step.id);
                    result.needs_attention = true;
                    upsert_step(report, result);
                    return Ok(StepOutcome::nothing(false));
                }
                Watched::Skip => {
                    upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                    return Ok(StepOutcome::nothing(false));
                }
                Watched::Quit => {
                    upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                    return Ok(StepOutcome::nothing(true));
                }
            }
        } else if interactive && input.is_tty() {
            // A step whose expectation the previous reading already satisfies
            // cannot be seen arriving — DC V with the leads open and shorted
            // both read about zero — so it is Enter-only and the keyboard is
            // offered at once. A mode the tool just switched to is a change, so
            // this is false there.
            let ask = enter_only(expect, prev.last.as_ref());
            let timeout = if ask { Duration::ZERO } else { STEP_TIMEOUT };
            let mut watcher = StateWatcher::for_step(expect, prev.baseline.as_ref(), !ask);
            match watch_for_state(dmm, input, &mut watcher, timeout, true, &mut errors)? {
                Watched::Ready(m) => settled = m,
                // `hint` keeps the wait open, so the timeout never ends it.
                Watched::TimedOut(_) => {}
                Watched::Skip => {
                    upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                    return Ok(StepOutcome::nothing(false));
                }
                Watched::Quit => {
                    upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                    return Ok(StepOutcome::nothing(true));
                }
            }
        } else if interactive {
            // No terminal to poll: ask, the way this step always did.
            let ch = input.key(&format!(
                "  {} ",
                style("any key=capture, s=skip, q=finish:").dim()
            ))?;
            if ch == 'q' || ch == 'Q' {
                upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                return Ok(StepOutcome::nothing(true));
            }
            if ch == 's' || ch == 'S' {
                upsert_step(report, step.empty_result(StepStatus::Skipped, None));
                return Ok(StepOutcome::nothing(false));
            }
        }

        let mut measurements: Vec<Measurement> = settled.into_iter().collect();
        let wanted = step.samples.saturating_sub(measurements.len());
        measurements.extend(capture_samples(dmm, wanted, &mut errors));
        let sample_data: Vec<SampleData> = measurements
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

        let confirmation = if let Some(last) = sample_data.last() {
            if interactive {
                eprintln!("  We read: {}", style(last.summary()).green());
            }
            // A trusted run doesn't stop here: the reading is listed with the
            // others in the one review at the end.
            if interactive && trust.confirm_inline(step) {
                let answer = input.line(&format!(
                    "  {} ",
                    style("Enter=correct, r=retake, or type what the meter actually shows:").dim()
                ))?;
                match Confirmation::of(answer) {
                    Confirmation::Retake => {
                        eprintln!("  {}", style("retaking\u{2026}").dim());
                        continue;
                    }
                    Confirmation::Answer(answer) => Some(answer),
                }
            } else {
                None
            }
        } else {
            eprintln!("  {}", style("No response from meter.").yellow());
            None
        };

        break (sample_data, measurements, confirmation);
    };

    let diagnostics = errors.into_diagnostics();
    let (frames, frames_dropped) = {
        let mut rec = recording::lock(recorder);
        rec.set_step(None);
        frames_for_step(&rec.drain(), step.id)
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

    let to_review = confirmation.is_none() && status == StepStatus::Captured;
    let mut result = StepResult {
        needs_attention: needs_attention(&sample_data, step.samples, &diagnostics),
        samples: sample_data,
        frames,
        frames_dropped,
        diagnostics,
        ..StepResult::new(step.id, step.instruction, status)
    };
    if let Some(answer) = confirmation {
        result.set_inline_confirmation(answer);
    }

    upsert_step(report, result);
    report.wire_events_dropped = recording::lock(recorder).dropped();
    Ok(StepOutcome::done(measurements.pop(), to_review))
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
        let mut errors = ErrorLog::default();
        let samples = capture_samples(&mut dmm, 1, &mut errors);
        let diagnostics = errors.into_diagnostics();

        assert_eq!(samples.len(), 1, "polling must continue past the error");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("checksum"), "got {diagnostics:?}");
    }

    /// A meter stuck on bad frames would otherwise repeat one line per poll.
    #[test]
    fn repeated_errors_are_reported_once_with_a_count() {
        let mut bad = measurement_frame();
        let last = bad.len() - 1;
        bad[last] = bad[last].wrapping_add(1);

        let mut dmm = dmm_replaying(vec![bad.clone(), bad.clone(), bad]);
        let mut errors = ErrorLog::default();
        let samples = capture_samples(&mut dmm, 1, &mut errors);
        let diagnostics = errors.into_diagnostics();

        assert!(samples.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].ends_with("(x3)"), "got {diagnostics:?}");
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

    /// A command the meter ignored has to say what it is still showing, or
    /// the report reads as though the step never ran.
    #[test]
    fn a_dead_command_names_what_the_meter_still_shows() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(
            did_nothing("hold", Some(&m)),
            "hold did nothing; the meter still shows 5.678 V [AUTO]"
        );
        assert_eq!(
            did_nothing("hold", None),
            "hold did nothing; the meter sent no reading"
        );
    }

    /// Only the events tagged with the step, and no more than the cap — the
    /// last of them: a step that waited for the meter before sampling would
    /// otherwise fill the report with the wait and drop the samples.
    #[test]
    fn step_frames_are_filtered_and_capped_to_the_newest() {
        let event = |at_ms: u64, step: Option<&str>| WireEvent {
            at_ms,
            dir: crate::recording::Direction::Rx,
            step: step.map(str::to_string),
            bytes: vec![0xAB, 0xCD],
            feature: false,
        };
        let mut events: Vec<WireEvent> = (0..MAX_FRAMES_PER_STEP as u64 + 3)
            .map(|at_ms| event(at_ms, Some("dcv")))
            .collect();
        events.push(event(0, Some("acv")));
        events.push(event(0, None));

        let (frames, dropped) = frames_for_step(&events, "dcv");
        assert_eq!(frames.len(), MAX_FRAMES_PER_STEP);
        assert_eq!(
            frames[0].at_ms, 3,
            "the three oldest events are the dropped ones"
        );
        assert_eq!(frames[0].hex, "AB CD");
        assert_eq!(dropped, 3);
    }

    /// A step that waits on the operator — capacitance, NCV — passes the cap
    /// as a matter of course. The trim used to arrive as a diagnostic, and
    /// every diagnostic flags the step, so both were marked for a maintainer
    /// with nothing wrong in them.
    #[test]
    fn passing_the_frame_cap_does_not_flag_the_step() {
        let events: Vec<WireEvent> = (0..MAX_FRAMES_PER_STEP as u64 + 450)
            .map(|at_ms| WireEvent {
                at_ms,
                dir: crate::recording::Direction::Rx,
                step: Some("ncv".to_string()),
                bytes: vec![0xAB, 0xCD],
                feature: false,
            })
            .collect();

        let (frames, dropped) = frames_for_step(&events, "ncv");
        assert_eq!(frames.len(), MAX_FRAMES_PER_STEP);
        assert_eq!(dropped, 450);

        let samples = vec![SampleData::from_measurement(&make_test_measurement(
            0x14,
            0x00,
            b"      3",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ))];
        assert!(!needs_attention(&samples, samples.len(), &[]));
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

    /// The retake key has to be told from a reading typed at the same prompt:
    /// only a bare `r` redoes the step.
    #[test]
    fn r_alone_asks_for_a_retake() {
        assert_eq!(Confirmation::of("r".to_string()), Confirmation::Retake);
        assert_eq!(Confirmation::of("R".to_string()), Confirmation::Retake);
        for typed in ["", "5.68 V", "rel", "R 0.5"] {
            assert_eq!(
                Confirmation::of(typed.to_string()),
                Confirmation::Answer(typed.to_string()),
                "{typed:?}"
            );
        }
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

    #[test]
    fn review_indices_are_one_based_and_bounded() {
        assert_eq!(parse_review_indices("", 3), Ok(vec![]));
        assert_eq!(parse_review_indices("  ", 3), Ok(vec![]));
        assert_eq!(parse_review_indices("1,3", 3), Ok(vec![0, 2]));
        assert_eq!(parse_review_indices("3 1", 3), Ok(vec![0, 2]));
        assert_eq!(parse_review_indices("2, 2", 3), Ok(vec![1]));

        for bad in ["0", "4", "-1", "two", "1,x"] {
            assert!(
                parse_review_indices(bad, 3).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn the_review_table_numbers_every_reading() {
        let rows = vec![
            ("dcv".to_string(), "5.678 V [AUTO]".to_string()),
            ("acv".to_string(), "239.22 V [AUTO HV!]".to_string()),
        ];
        let out = console::strip_ansi_codes(&render_review_table(&rows)).into_owned();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("    1  dcv"), "{out}");
        assert!(lines[0].ends_with("5.678 V [AUTO]"), "{out}");
        assert!(lines[1].starts_with("    2  acv"), "{out}");
    }

    /// The review is the only confirmation a trusted run's readings get, so
    /// it has to record both verdicts the way the inline prompt does.
    #[test]
    fn the_review_confirms_and_corrects_the_readings_it_covers() {
        let mut report = CaptureReport::default();
        for id in ["dcv", "acv", "ohm"] {
            upsert_step(&mut report, StepResult::new(id, "t", StepStatus::Captured));
        }

        apply_review(
            &mut report,
            &[
                ("dcv".to_string(), None),
                ("acv".to_string(), Some("239.4 V".to_string())),
                // Listed as wrong but typed nothing: still a mismatch.
                ("ohm".to_string(), Some(String::new())),
            ],
        );

        assert_eq!(report.steps[0].confirmed, Some(true));
        assert_eq!(report.steps[0].lcd, None);
        assert_eq!(report.steps[0].confirmed_by, Some(ConfirmedBy::Batch));
        assert_eq!(report.steps[1].confirmed, Some(false));
        assert_eq!(report.steps[1].lcd.as_deref(), Some("239.4 V"));
        assert_eq!(report.steps[1].confirmed_by, Some(ConfirmedBy::Batch));
        assert_eq!(report.steps[2].confirmed, Some(false));
        assert_eq!(report.steps[2].lcd, None);
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

    fn cli_step(id: &'static str, verified: bool, gate: bool) -> CaptureStep {
        CaptureStep {
            id,
            instruction: "do the thing",
            command: None,
            samples: 5,
            expect: None,
            verified,
            gate,
            needs: &[],
        }
    }

    fn needy_step(id: &'static str, needs: &'static [Need]) -> CaptureStep {
        CaptureStep {
            needs,
            ..cli_step(id, false, false)
        }
    }

    /// The checklist is what the operator sets the bench up from, so it lists
    /// each item once, in one fixed order, with the steps waiting on it.
    #[test]
    fn needs_checklist_lists_each_item_once_in_need_order() {
        let steps = [
            needy_step("temp", &[Need::Thermocouple]),
            needy_step("dcv_short", &[Need::ShortedLeads]),
            needy_step("tempf", &[Need::Thermocouple]),
            needy_step("acv", &[]),
            needy_step("ohm_short", &[Need::ShortedLeads]),
        ];
        let (table, needs) = render_needs(&steps).unwrap();
        assert_eq!(needs, vec![Need::ShortedLeads, Need::Thermocouple]);
        assert_eq!(
            table,
            "You will need:\n\
             \x20 [1] shorted test leads  (steps: dcv_short, ohm_short)\n\
             \x20 [2] K-type thermocouple  (steps: temp, tempf)\n"
        );
    }

    /// Nothing to gather, nothing to ask about: a run whose steps need only
    /// the meter opens straight into the first step.
    #[test]
    fn no_needs_shows_no_checklist() {
        let steps = [needy_step("acv", &[]), needy_step("hold", &[])];
        assert!(render_needs(&steps).is_none());
    }

    /// Saying "no thermocouple" drops the temperature steps and nothing else.
    #[test]
    fn deselecting_a_need_drops_exactly_its_steps() {
        let steps = [
            needy_step("dcv_short", &[Need::ShortedLeads]),
            needy_step("temp", &[Need::Thermocouple]),
            needy_step("tempf", &[Need::Thermocouple]),
            needy_step("acv", &[]),
        ];
        let dropped = steps_without(&steps, &[Need::Thermocouple]);
        assert_eq!(
            dropped
                .iter()
                .map(|(s, label)| (s.id, *label))
                .collect::<Vec<_>>(),
            vec![
                ("temp", "K-type thermocouple"),
                ("tempf", "K-type thermocouple"),
            ]
        );
        assert!(steps_without(&steps, &[]).is_empty());
    }

    /// `--steps` and `--unverified` narrow together: a verified step named on
    /// the command line still doesn't run under `--unverified`.
    #[test]
    fn steps_and_unverified_intersect() {
        let done = cli_step("dcv", true, false);
        let todo = cli_step("temp", false, false);
        let filter: Option<std::collections::HashSet<String>> = Some(
            ["dcv".to_string(), "temp".to_string()]
                .into_iter()
                .collect(),
        );

        assert!(step_selected(&done, &None, false));
        assert!(!step_selected(&done, &None, true));
        assert!(step_selected(&todo, &None, true));

        assert!(step_selected(&done, &filter, false));
        assert!(!step_selected(&done, &filter, true));
        assert!(step_selected(&todo, &filter, true));

        let other: Option<std::collections::HashSet<String>> =
            Some(["dcv".to_string()].into_iter().collect());
        assert!(!step_selected(&todo, &other, true));
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

/// Whether the protocol pass runs this step: `--steps` and `--unverified`
/// narrow the list together, so naming a verified step under `--unverified`
/// still skips it.
fn step_selected(
    step: &CaptureStep,
    step_filter: &Option<std::collections::HashSet<String>>,
    unverified_only: bool,
) -> bool {
    step_included(step_filter, step.id) && !(unverified_only && step.verified)
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

/// Determine the output path and load an existing report (with resume/overwrite prompt)
/// or create a fresh one. Returns `None` if the user chose to abort.
fn load_or_create_report(
    output_override: Option<String>,
    device_name: &str,
    plan_path: Option<&str>,
    input: &Input,
) -> Result<Option<(CaptureReport, String)>, Box<dyn std::error::Error>> {
    let auto_path = match plan_path {
        // A plan run covers a handful of steps nobody else asked for, so it
        // gets its own file rather than resuming into the full report.
        Some(plan) => format!(
            "capture-{}-{}.yaml",
            slug(device_name),
            slug(&plan_stem(plan))
        ),
        None => format!("capture-{}.yaml", slug(device_name)),
    };
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

/// The numbered list the operator reads against the meter's screen, one line
/// per reading captured without a confirmation.
fn render_review_table(rows: &[(String, String)]) -> String {
    let mut out = String::new();
    for (i, (id, summary)) in rows.iter().enumerate() {
        out.push_str(&format!(
            "  {:>3}  {:<16} {summary}\n",
            i + 1,
            style(id).cyan()
        ));
    }
    out
}

/// The readings the operator listed as wrong, as indices into the table.
///
/// One-based on the way in, since that is what the table shows; an empty line
/// means every reading matched.
fn parse_review_indices(input: &str, len: usize) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for token in input.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let n = token
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=len).contains(n))
            .ok_or_else(|| format!("{token:?} is not a number between 1 and {len}"))?;
        if !out.contains(&(n - 1)) {
            out.push(n - 1);
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// Write the review's verdict into the report: `Some(text)` is what the meter
/// showed for a reading the operator called wrong, `None` confirms it.
fn apply_review(report: &mut CaptureReport, answers: &[(String, Option<String>)]) {
    for (id, lcd) in answers {
        if let Some(step) = report.steps.iter_mut().find(|s| s.id == *id) {
            step.set_batch_confirmation(lcd.clone());
        }
    }
}

/// Ask about every reading the run captured without stopping for it, once.
///
/// A piped run has nobody to ask, so those readings stay unconfirmed rather
/// than being recorded as agreed.
fn run_batch_review(
    report: &mut CaptureReport,
    to_review: &[String],
    output_path: &str,
    input: &Input,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<(String, String)> = to_review
        .iter()
        .filter_map(|id| {
            let step = report.steps.iter().find(|s| s.id == *id)?;
            Some((id.clone(), step.samples.last()?.summary()))
        })
        .collect();
    if rows.is_empty() || !input.is_tty() {
        return Ok(());
    }

    eprintln!(
        "\n{}",
        style("\u{2501}\u{2501}\u{2501} Review the readings \u{2501}\u{2501}\u{2501}").bold()
    );
    eprint!("{}", render_review_table(&rows));

    let wrong = loop {
        let answer = input.line(
            "Numbers of the readings that did NOT match the meter's screen (Enter = all correct): ",
        )?;
        match parse_review_indices(&answer, rows.len()) {
            Ok(indices) => break indices,
            Err(e) => eprintln!("  {}", style(e).yellow()),
        }
    };

    let mut answers: Vec<(String, Option<String>)> = Vec::with_capacity(rows.len());
    for (i, (id, _)) in rows.iter().enumerate() {
        let lcd = if wrong.contains(&i) {
            Some(input.line(&format!("[{id}] What did the meter show? "))?)
        } else {
            None
        };
        answers.push((id.clone(), lcd));
    }
    apply_review(report, &answers);
    save_report(report, output_path)?;
    Ok(())
}

/// Part 1: Run measurement mode capture steps. Returns true if user wants to quit.
/// Part 4: Freeform additional captures.
fn run_freeform_captures(
    dmm: &mut dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: &SharedRecorder,
    step_filter: &Option<std::collections::HashSet<String>>,
    report: &mut CaptureReport,
    output_path: &str,
    input: &Input,
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
        let desc = input.line(&format!(
            "[extra_{extra}] Describe what you set the meter to (or 'q' to finish): "
        ))?;
        if desc.is_empty() || desc.to_lowercase().starts_with('q') {
            break;
        }

        let step_id = format!("extra_{extra}");
        recording::lock(recorder).set_step(Some(&step_id));
        let mut errors = ErrorLog::default();
        let measurements = capture_samples(dmm, FREEFORM_SAMPLES, &mut errors);
        let diagnostics = errors.into_diagnostics();
        let sample_data: Vec<SampleData> = measurements
            .iter()
            .map(SampleData::from_measurement)
            .collect();
        let (frames, frames_dropped) = {
            let mut rec = recording::lock(recorder);
            rec.set_step(None);
            frames_for_step(&rec.drain(), &step_id)
        };

        for (i, s) in sample_data.iter().enumerate() {
            eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
        }

        let confirmation = if let Some(last) = sample_data.last() {
            Some(input.line(&format!(
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
            frames_dropped,
            diagnostics,
            ..StepResult::new(&step_id, &desc, status)
        };
        if let Some(answer) = confirmation {
            result.set_inline_confirmation(answer);
        }
        upsert_step(report, result);
        report.wire_events_dropped = recording::lock(recorder).dropped();
        save_report(report, output_path)?;
        extra += 1;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_capture(
    output_override: Option<String>,
    filter: Option<Vec<String>>,
    unverified_only: bool,
    sniff: bool,
    no_drive: bool,
    plan_path: Option<String>,
    mut dmm: dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>,
    recorder: SharedRecorder,
    device: &'static dmm_lib::protocol::registry::SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_filter: Option<std::collections::HashSet<String>> =
        filter.map(|v| v.into_iter().collect());
    // Before the meter is touched: a plan the tool can't read is the
    // reporter's typo, and they should hear about it straight away.
    let plan_steps = plan_path.as_deref().map(crate::plan::load).transpose()?;

    let (device_name, supported) = verify_meter(&mut dmm, device)?;

    let input = Input::start();
    let (mut report, output_path) =
        match load_or_create_report(output_override, &device_name, plan_path.as_deref(), &input)? {
            Some(pair) => pair,
            None => return Ok(()),
        };

    eprintln!("Output file: {output_path}\n");

    populate_report_metadata(&mut report, &mut dmm, device_name, supported);
    report.device_id = Some(device.id.to_string());
    report.plan = plan_path.clone();
    report.unverified_only = unverified_only;
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
    let unverified_ids: std::collections::HashSet<&str> = protocol_steps
        .iter()
        .filter(|s| !s.verified)
        .map(|s| s.id)
        .collect();
    // A plan replaces the device's list for the run — same watcher, tiers,
    // needs checklist and sweeps, but only the steps the maintainer wrote.
    let cli_steps: Vec<CaptureStep> = match &plan_steps {
        Some(steps) => steps.clone(),
        None => protocol_steps.iter().map(CaptureStep::from).collect(),
    };
    let mut trust = Trust::new(sniff, supported, &cli_steps);
    report.tier = Some(trust.tier);
    let mut driver = crate::drive::Driver::new(!no_drive);
    let pass = run_protocol_capture(
        &mut dmm,
        &recorder,
        &cli_steps,
        &step_filter,
        unverified_only,
        &mut report,
        &output_path,
        &input,
        &mut trust,
        &mut driver,
    )?;
    report.drive = Some(driver.state());

    run_batch_review(&mut report, &pass.to_review, &output_path, &input)?;

    if !pass.quit {
        run_freeform_captures(
            &mut dmm,
            &recorder,
            &step_filter,
            &mut report,
            &output_path,
            &input,
        )?;
    }

    report.wire_events_dropped = recording::lock(&recorder).dropped();
    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    if let Some(plan) = &plan_path {
        // The plan's steps are not in the device's unverified list, so its
        // coverage line would read zero for a run that captured everything.
        let ids: std::collections::HashSet<&str> = cli_steps.iter().map(|s| s.id).collect();
        eprintln!(
            "Plan {plan}: {} of {} steps captured",
            captured_count(&report, &ids),
            ids.len()
        );
    } else {
        let covered = captured_count(&report, &unverified_ids);
        if unverified_only && covered == 0 {
            eprintln!("No unverified step was captured, so there is nothing new to report.");
        } else {
            eprintln!(
                "Covered {covered} of {} unverified steps for {}.",
                unverified_ids.len(),
                device.display_name
            );
        }
    }
    eprintln!("Attach the report to {}", dmm.profile().feedback_url());
    Ok(())
}

/// How many of `ids` the report now holds samples for — the number the
/// epilogue reports as what this run is worth to the issue.
fn captured_count(report: &CaptureReport, ids: &std::collections::HashSet<&str>) -> usize {
    report
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Captured && ids.contains(s.id.as_str()))
        .count()
}
