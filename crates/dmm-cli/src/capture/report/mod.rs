//! The capture report: its schema and serde, the trust tier the gate rules
//! on, and the report file the run reads back and writes out.

use super::watch::Baseline;
use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::Measurement;
use serde::{Deserialize, Serialize};

mod file;
mod trust;

pub(crate) use file::{from_yaml, save_report};
pub(super) use file::{
    hex_bytes, hex_string, load_or_create_report, no_response_path, populate_report_metadata,
    tool_version,
};
pub(crate) use trust::Trust;

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
    /// The id the run was picked by when it names the meter under another
    /// brand's name (`ut217a` for a run on the `cms061` entry); absent when
    /// it is `device_id`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub picked_as: Option<String>,
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
    /// Why the check gave up when `no_response` is set: a timeout, or the
    /// error the meter's answer drew.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub no_response_error: Option<String>,
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
    pub drive: Option<super::drive::Drive>,
    pub steps: Vec<StepResult>,
    /// What auto-detection made of the meter, checked once the steps are done.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub detection: Option<super::detection::DetectionCheck>,
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
    /// Link set-up rather than a transfer: a baud rate change (`baud`), or
    /// in older reports a HID feature report (`hex`).
    #[serde(skip_serializing_if = "is_false", default)]
    pub feature: bool,
    /// The rate the link was set to.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub baud: Option<u32>,
}

impl From<&super::recording::WireEvent> for FrameRecord {
    fn from(e: &super::recording::WireEvent) -> Self {
        FrameRecord {
            at_ms: e.at_ms,
            dir: match e.dir {
                super::recording::Direction::Tx => FrameDir::Tx,
                super::recording::Direction::Rx => FrameDir::Rx,
            },
            hex: hex_string(&e.bytes),
            feature: e.baud.is_some(),
            baud: e.baud,
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
    /// The instruction the operator read instead, from their own
    /// translation (`capture --instructions`), so a maintainer can see what
    /// they were asked to do.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instruction_shown: Option<String>,
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
    /// Read errors seen while sampling that left the link up — checksum
    /// mismatches, unknown modes, malformed responses.
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
            instruction_shown: super::instructions::translated(id, instruction).map(str::to_string),
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

    /// Record a verdict on a captured reading: `None` confirms it, `Some(text)`
    /// is a mismatch and what the meter showed, which the operator may leave
    /// blank.
    fn set_confirmation(&mut self, lcd: Option<String>, by: ConfirmedBy) {
        self.confirmed = Some(lcd.is_none());
        self.lcd = lcd.filter(|text| !text.is_empty());
        self.confirmed_by = Some(by);
    }

    /// Record the operator's answer to the prompt at the step itself.
    pub(super) fn set_inline_confirmation(&mut self, lcd: Option<String>) {
        self.set_confirmation(lcd, ConfirmedBy::Inline);
    }

    /// Record the end-of-run review's verdict: `lcd` is what the meter showed,
    /// given only for the readings the operator listed as wrong.
    pub(super) fn set_batch_confirmation(&mut self, lcd: Option<String>) {
        self.set_confirmation(lcd, ConfirmedBy::Batch);
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
    /// The reading's name beside sub-values of the same quantity ("DC" in
    /// the UT61E+'s AC+DC V, "T1" or "Peak Max" on a UT181A), omitted when
    /// it has none.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub main_label: Option<String>,
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
        let raw_hex = hex_string(&m.raw_payload);
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
            main_label: m.main_label.map(|l| l.as_str().to_string()),
        }
    }

    /// The main reading as the meter's screen shows it: the meter's own
    /// digits, or the value's rendering (`OL`, `----`) where the protocol
    /// sends none. A UT181A overload carries no digits, and its confirmation
    /// line read ` MΩ [AUTO]`, which the operator rightly refused against an
    /// LCD showing OL — failing the gate (issue #5).
    pub(crate) fn shown(&self) -> &str {
        match self.display_raw.trim() {
            "" => &self.value,
            digits => digits,
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
        // Sub-values are on the meter's screen too, so the operator has to
        // see them to confirm the sample — a UT181A in MIN/MAX or with the
        // frequency display up otherwise confirms against half the screen.
        // Appended, so single-display meters keep the line they had, and
        // rendered by the same helper `dmm-cli read` uses so the two can't
        // describe the same reading differently.
        let aux = (!self.aux.is_empty()).then(|| {
            dmm_lib::measurement::aux_summary_line(self.aux.iter().map(|a| {
                (
                    a.label.as_str(),
                    a.value.as_str(),
                    a.unit.as_str(),
                    a.elapsed_secs,
                )
            }))
        });
        // A frame without a main reading: its sub-values take the value's
        // place, as `dmm-cli read` prints them.
        let (mut out, aux) = match aux {
            Some(aux) if self.value.is_empty() => (aux, None),
            aux => {
                let name = self
                    .main_label
                    .as_deref()
                    .map(|l| format!("{l} "))
                    .unwrap_or_default();
                (format!("{name}{} {}", self.shown(), self.unit), aux)
            }
        };
        if !flags.is_empty() {
            out.push_str(&format!(" [{flags}]"));
        }
        if let Some(aux) = aux {
            out.push_str(&format!(" ({aux})"));
        }
        out
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
        || samples
            .iter()
            .any(|s| dmm_lib::protocol::is_unknown_mode(&s.mode))
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
    use super::file::to_yaml;
    use super::*;
    use crate::capture::watch::{STABLE_FRAMES, StateWatcher, Verdict};
    use dmm_lib::measurement::MeasuredValue;
    use dmm_lib::protocol::make_test_measurement;

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

    /// A UT8802 or UT8803 overload from its status bit comes with digits on
    /// the wire; the confirmation line has to read OL, as the meter does,
    /// not those digits.
    #[test]
    fn status_bit_overload_is_shown_as_ol() {
        for (id, payload) in [
            // DC V, digits 1234.5, sign byte bit 6 (OL).
            ("ut8802", vec![0x05, 0x45, 0x23, 0x01, 0x01, 0x00, 0x40]),
            // Ω, display "    0", frame byte 14 bit 2 (OL).
            (
                "ut8803",
                vec![
                    0x00, 0x02, 0x08, 0x30, 0x00, b' ', b' ', b' ', b' ', b'0', 0x00, 0x00, 0x04,
                    0x00, 0x00, 0x00, 0x00,
                ],
            ),
        ] {
            let device = dmm_lib::protocol::registry::find_device(id).unwrap();
            let m = (device.new_protocol)().parse_payload(&payload).unwrap();
            let s = SampleData::from_measurement(&m);
            assert_eq!(s.shown(), "OL", "{id}");
        }
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
        // AC+DC V: flag1=0x0F (REL+HOLD+MIN+MAX), flag2=0x04 (manual range),
        // flag3=0x00 (the DC component)
        let m = make_test_measurement(0x19, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x04, 0x00));
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

        let yaml = to_yaml(&SampleData::from_measurement(&m).flags).unwrap();
        let parsed: std::collections::BTreeMap<String, bool> = from_yaml(&yaml).unwrap();

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
        let flags: SampleFlags = from_yaml(yaml).unwrap();
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

    /// A UT61E+ AC+DC V frame carrying only the AC component is confirmed
    /// against the component, not a blank value and a lone unit.
    #[test]
    fn summary_of_a_frame_without_a_main_reading() {
        let m = make_test_measurement(0x19, 0x00, b" 0.0000", (0x00, 0x00), (0x00, 0x00, 0x08));
        assert!(matches!(m.value, MeasuredValue::Absent));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.summary(), "AC 0.0000 V [AUTO]");
    }

    /// The DC frame of the same pair is confirmed against its name too, as
    /// `dmm-cli read` prints it.
    #[test]
    fn summary_names_a_named_reading() {
        let m = make_test_measurement(0x19, 0x00, b" 1.6113", (0x00, 0x00), (0x00, 0x00, 0x00));
        let s = SampleData::from_measurement(&m);
        assert_eq!(s.summary(), "DC 1.6113 V [AUTO]");
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

    /// A reading without digits of its own — the UT181A's overload and its
    /// blank after a switch — confirms against what the meter shows instead
    /// of a lone unit: @diego351 refused ` MΩ [AUTO]` against an LCD showing
    /// OL, which failed the gate and left the whole run undriven (issue #5).
    #[test]
    fn summary_shows_a_reading_without_digits() {
        use dmm_lib::measurement::MeasuredValue;

        let auto = StatusFlags {
            auto_range: true,
            ..Default::default()
        };
        for (value, expected) in [
            (MeasuredValue::Overload, "OL MΩ [AUTO]"),
            (MeasuredValue::NoReading("----"), "---- MΩ [AUTO]"),
        ] {
            let mut m = Measurement::test_fixture(value, "MΩ", auto);
            m.display_raw = None;
            let sample = SampleData::from_measurement(&m);
            assert_eq!(sample.summary(), expected);
            assert_eq!(sample.display_raw, "", "the report keeps the raw field");
        }
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

        let yaml = to_yaml(&sample).unwrap();
        assert_eq!(
            yaml.matches("elapsed_secs").count(),
            1,
            "elapsed_secs: None must be omitted: {yaml}"
        );

        let parsed: SampleData = from_yaml(&yaml).unwrap();
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
        let yaml = to_yaml(&sample).unwrap();
        assert!(!yaml.contains("aux"), "empty aux must be omitted: {yaml}");
        let parsed: SampleData = from_yaml(&yaml).unwrap();
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
        let report: CaptureReport = from_yaml(yaml).unwrap();
        assert_eq!(report.steps.len(), 1);
        assert!(report.init_frames.is_empty());
        assert!(report.steps[0].frames.is_empty());
        assert!(report.steps[0].diagnostics.is_empty());
        assert!(!report.steps[0].needs_attention);
        assert_eq!(report.wire_events_dropped, 0);
        assert_eq!(report.device_id, None);
        assert_eq!(report.picked_as, None);
    }

    /// A run picked under another brand's name keeps that name beside the
    /// entry it decoded with.
    #[test]
    fn a_picked_name_round_trips() {
        let report = CaptureReport {
            device_id: Some("cms061".to_string()),
            picked_as: Some("ut217a".to_string()),
            ..CaptureReport::default()
        };
        let back: CaptureReport = from_yaml(&to_yaml(&report).unwrap()).unwrap();
        assert_eq!(back.picked_as.as_deref(), Some("ut217a"));
        assert_eq!(back.device_id.as_deref(), Some("cms061"));
    }

    /// The new fields must stay out of a report that has nothing to put in
    /// them, so existing captures round-trip unchanged.
    #[test]
    fn empty_wire_fields_are_omitted() {
        let report = CaptureReport {
            steps: vec![StepResult::new("dcv", "test", StepStatus::Captured)],
            ..CaptureReport::default()
        };
        let yaml = to_yaml(&report).unwrap();
        for key in [
            "frames",
            "diagnostics",
            "needs_attention",
            "wire_events_dropped",
            "device_id",
            "picked_as",
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
            baud: None,
        };
        let yaml = to_yaml(&record).unwrap();
        assert!(yaml.contains("dir: rx"), "got {yaml}");
        assert!(!yaml.contains("feature"), "got {yaml}");
        assert!(!yaml.contains("baud"), "got {yaml}");
    }

    /// A rate change is filed as link set-up carrying its rate, and a
    /// report from before rate changes were recorded still loads.
    #[test]
    fn a_rate_change_is_a_feature_record_with_its_rate() {
        let event = crate::capture::recording::WireEvent {
            at_ms: 3,
            dir: crate::capture::recording::Direction::Tx,
            step: None,
            bytes: Vec::new(),
            baud: Some(19200),
        };
        let yaml = to_yaml(&FrameRecord::from(&event)).unwrap();
        assert!(yaml.contains("feature: true"), "got {yaml}");
        assert!(yaml.contains("baud: 19200"), "got {yaml}");

        let old: FrameRecord =
            from_yaml("at_ms: 3\ndir: tx\nhex: 00 00 4B 00 00 03 00 00 00 00\nfeature: true\n")
                .unwrap();
        assert!(old.feature);
        assert_eq!(old.baud, None);
    }

    /// Enter at the prompt means the meter agreed with what we read, so the
    /// step records the confirmation and no screen text.
    #[test]
    fn pressing_enter_confirms_the_reading() {
        let mut step = StepResult::new("dcv", "test", StepStatus::Captured);
        step.set_inline_confirmation(None);
        assert_eq!(step.confirmed, Some(true));
        assert_eq!(step.lcd, None);
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
    }

    /// A typed correction is what the meter actually showed — the evidence
    /// that our parse is wrong — so it is kept apart from the confirmation.
    #[test]
    fn a_typed_correction_records_what_the_meter_showed() {
        let mut step = StepResult::new("dcv", "test", StepStatus::Captured);
        step.set_inline_confirmation(Some("5.68 V".to_string()));
        assert_eq!(step.confirmed, Some(false));
        assert_eq!(step.lcd.as_deref(), Some("5.68 V"));
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));

        // Said to be wrong but typed nothing: still a mismatch, as it is in
        // the end-of-run review.
        let mut step = StepResult::new("dcv", "test", StepStatus::Captured);
        step.set_inline_confirmation(Some(String::new()));
        assert_eq!(step.confirmed, Some(false));
        assert_eq!(step.lcd, None);
    }

    /// Reports written before the split store the confirmation as free text.
    /// Resuming one has to recover both cases, or the operator is asked to
    /// confirm steps they already confirmed.
    #[test]
    fn legacy_screen_text_normalizes_into_the_new_fields() {
        let mut step: StepResult = from_yaml(
            "id: dcv\ninstruction: test\nstatus: captured\nscreen: 'confirmed: 5.678 V [AUTO]'\n",
        )
        .unwrap();
        step.normalize_legacy();
        assert_eq!(step.confirmed, Some(true));
        assert_eq!(step.lcd, None);
        assert_eq!(step.confirmed_by, Some(ConfirmedBy::Inline));
        assert_eq!(step.screen, None);

        let mut step: StepResult =
            from_yaml("id: dcv\ninstruction: test\nstatus: captured\nscreen: 5.68 V\n").unwrap();
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
        let yaml = to_yaml(&StepResult::new("dcv", "test", StepStatus::Captured)).unwrap();
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

    #[test]
    fn capture_report_serde_roundtrip() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let sample = SampleData::from_measurement(&m);
        let long_hex = vec!["AB"; 70].join(" ");

        let report = CaptureReport {
            date: "2026-01-01T00:00:00+00:00".to_string(),
            tool_version: "0.2.0-dev (abc1234)".to_string(),
            device_name: "UT61E+".to_string(),
            transport_name: Some("CP2110".to_string()),
            // Strings a plain scalar would turn into null, a boolean or a
            // folded block come back as written.
            transport_info: Some(String::new()),
            supported: true,
            init_frames: vec![FrameRecord {
                at_ms: 0,
                dir: FrameDir::Rx,
                hex: long_hex.clone(),
                feature: false,
                baud: None,
            }],
            drive: Some(super::super::drive::Drive::On),
            steps: vec![StepResult {
                samples: vec![sample],
                confirmed: Some(false),
                lcd: Some("~".to_string()),
                confirmed_by: Some(ConfirmedBy::Inline),
                error: Some("null".to_string()),
                ..StepResult::new("dcv", "Set meter to DC V", StepStatus::Captured)
            }],
            ..CaptureReport::default()
        };

        let yaml = to_yaml(&report).unwrap();
        let parsed: CaptureReport = from_yaml(&yaml).unwrap();

        assert_eq!(parsed.date, report.date);
        assert_eq!(parsed.device_name, report.device_name);
        assert!(parsed.supported);
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.steps[0].samples.len(), 1);
        assert_eq!(parsed.steps[0].samples[0].value, "5.678");
        assert_eq!(parsed.steps[0].samples[0].mode_byte, "0x02");
        assert_eq!(parsed.steps[0].samples[0].display_raw, "  5.678");
        assert_eq!(parsed.steps[0].confirmed, Some(false));
        assert_eq!(parsed.steps[0].lcd.as_deref(), Some("~"));
        assert_eq!(parsed.steps[0].error.as_deref(), Some("null"));
        assert_eq!(parsed.steps[0].confirmed_by, Some(ConfirmedBy::Inline));
        assert_eq!(parsed.transport_info.as_deref(), Some(""));
        assert_eq!(parsed.drive, Some(super::super::drive::Drive::On));
        assert!(yaml.contains(&format!("hex: {long_hex}\n")), "{yaml}");
        assert_eq!(to_yaml(&parsed).unwrap(), yaml);
    }

    /// A long run's report, every step at its frame cap, reads back: past
    /// the parser's default 250 000 nodes.
    #[test]
    fn a_full_report_reads_back() {
        let frames: Vec<FrameRecord> = (0..super::super::step::MAX_FRAMES_PER_STEP as u64)
            .map(|at_ms| FrameRecord {
                at_ms,
                dir: FrameDir::Rx,
                hex: "AB CD 10".to_string(),
                feature: false,
                baud: None,
            })
            .collect();
        let report = CaptureReport {
            steps: (0..100)
                .map(|i| StepResult {
                    frames: frames.clone(),
                    ..StepResult::new(&format!("s{i}"), "x", StepStatus::Captured)
                })
                .collect(),
            ..CaptureReport::default()
        };
        let parsed: CaptureReport = from_yaml(&to_yaml(&report).unwrap()).unwrap();
        assert_eq!(parsed.steps.len(), 100);
        assert_eq!(parsed.steps[99].frames.len(), frames.len());
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

        let yaml = to_yaml(&report).unwrap();
        // Optional None fields should not appear in output
        assert!(!yaml.contains("transport_name"));
        assert!(!yaml.contains("transport_info"));
        // Empty samples should not appear
        assert!(!yaml.contains("samples"));

        // Parse back
        let parsed: CaptureReport = from_yaml(&yaml).unwrap();
        assert!(parsed.transport_name.is_none());
        assert!(parsed.transport_info.is_none());
        assert!(parsed.steps[0].samples.is_empty());
    }

    #[test]
    fn step_status_serde() {
        let yaml = to_yaml(&StepStatus::Captured).unwrap();
        assert!(yaml.contains("captured"));

        let yaml = to_yaml(&StepStatus::Skipped).unwrap();
        assert!(yaml.contains("skipped"));

        let yaml = to_yaml(&StepStatus::Timeout).unwrap();
        assert!(yaml.contains("timeout"));

        let yaml = to_yaml(&StepStatus::Error).unwrap();
        assert!(yaml.contains("error"));
    }
}
