use crate::settings::DEFAULT_MAX_SAMPLES;
use chrono::{DateTime, Local, SecondsFormat};
use dmm_lib::WallClock;
use dmm_lib::export::{CsvLayout, device_comment};
use dmm_lib::measurement::Measurement;
use dmm_lib::replay;
use std::collections::VecDeque;
use std::io::Write;
use std::time::Instant;

/// Render samples as a CSV document, provenance header included.
///
/// Returns the finished bytes so the caller can hand them to a writer thread
/// without duplicating the sample buffer — a full buffer is ~170 MB of
/// `Sample`s, while the rendered CSV is a fraction of that and takes one pass
/// instead of half a million allocations.
///
/// `layout` fixes the columns for the whole file — see [`CsvLayout`] for what
/// the slot counts mean. Each row claims only as many of the reserved trailing
/// groups as its own [`Sample::extra_aux`] says it carries, because a scale
/// switched on mid-recording leaves the earlier samples without one.
pub fn render_csv(
    samples: &VecDeque<Sample>,
    device_model: &str,
    layout: CsvLayout,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // ~72 bytes covers a typical row (RFC3339 timestamp, mode, value, unit,
    // range, flags) without repeated growth on large buffers; each aux slot
    // adds roughly another 20.
    let row_bytes = 72 + layout.aux_slots() * 20;
    let mut buf: Vec<u8> = Vec::with_capacity(samples.len() * row_bytes + 128);
    writeln!(buf, "{}", device_comment(device_model))?;
    {
        let mut wtr = csv::Writer::from_writer(&mut buf);
        wtr.write_record(layout.header().iter().map(|c| c.as_ref()))?;
        for s in samples {
            let ts = s.wall_time.to_rfc3339();
            let cells = layout.row(&s.measurement, &ts, None, s.extra_aux);
            wtr.write_record(cells.iter().map(|c| c.as_ref()))?;
        }
        wtr.flush()?;
    }
    Ok(buf)
}

/// Render samples as a JSON document: the `_metadata` line naming the meter,
/// then one object per sample.
///
/// Every line comes from [`dmm_shared::export`], which is also what
/// `dmm-cli read --format json` prints, so a script reading one binary's file
/// works on the other's. `experimental` marks readings decoded by a protocol
/// no report has confirmed, as the CLI's does.
pub(crate) fn render_json(
    samples: &VecDeque<Sample>,
    device_model: &str,
    experimental: bool,
) -> String {
    let mut out = dmm_shared::export::metadata_line(device_model);
    out.push('\n');
    // A reading with no sub-values runs to roughly 400 bytes; growing from
    // there beats growing from nothing on a half-million-sample buffer.
    out.reserve(samples.len() * 400);
    for s in samples {
        let ts = s.wall_time.to_rfc3339();
        out.push_str(
            &dmm_shared::export::measurement_json(&s.measurement, &ts, experimental, None)
                .to_string(),
        );
        out.push('\n');
    }
    out
}

/// Render samples as a replay file `--replay` can play back, or `None` when
/// they carry no wire bytes to play.
///
/// Every line comes from [`dmm_lib::replay`], which also parses them, so the
/// GUI's files cannot drift from what reads them.
///
/// Offsets are measured from the first sample's frame timestamp — the session
/// instant the library stamped the frame with — rather than from wall times,
/// so a recording made on a scaled or preseeded clock plays back at the
/// spacing its readings actually arrived at. The `# recorded:` line is that
/// first sample's wall time, which is what pins playback to a clock origin.
///
/// `None` when any sample has an empty payload: the mock synthesises its
/// readings, so there is no frame to hand a parser.
pub(crate) fn render_replay(
    samples: &VecDeque<Sample>,
    device_id: &str,
    model: Option<&str>,
) -> Option<String> {
    let first = samples.front()?;
    let recorded = first
        .wall_time
        .to_rfc3339_opts(SecondsFormat::Millis, false);
    let mut out = replay::header(device_id, &recorded, model);
    // Offset digits and a newline, plus three characters per payload byte.
    // Frame length is fixed per family, so the first sample sizes the rest.
    out.reserve(samples.len() * (10 + 3 * first.measurement.raw_payload.len()));
    for s in samples {
        if s.measurement.raw_payload.is_empty() {
            return None;
        }
        let offset = s
            .measurement
            .timestamp
            .saturating_duration_since(first.measurement.timestamp);
        out.push_str(&replay::sample_line(offset, &s.measurement.raw_payload));
    }
    Some(out)
}

/// A single recorded sample.
///
/// Holds the underlying `Measurement` directly so both the recording panel
/// and the exports consume exactly the same data shape the protocol produced,
/// and static-lookup-table strings (`mode`, `unit`, `range_label`) stay as
/// `Cow::Borrowed` instead of being re-cloned onto the heap for every sample.
///
/// The meter's own frame is kept with it: it is what [`render_replay`] writes,
/// and it is the only part of a reading no decoded field can reconstruct.
#[derive(Debug, Clone)]
pub struct Sample {
    pub wall_time: DateTime<Local>,
    pub measurement: Measurement,
    /// How many trailing sub-values of `measurement` were appended by
    /// software (a transform's `Raw`) rather than sent by the meter, as of
    /// the moment this sample was recorded.
    ///
    /// Per-sample rather than per-file because a scale can be switched on
    /// mid-recording: without it the export would read the last sub-value of
    /// an earlier row as the appended one and file a meter's Frequency under
    /// the `Raw` column.
    pub extra_aux: usize,
}

impl Sample {
    pub fn from_measurement(m: &Measurement, wall_clock: &WallClock, extra_aux: usize) -> Self {
        Self {
            wall_time: wall_clock.wall_time_for(m.timestamp).into(),
            measurement: m.clone(),
            extra_aux,
        }
    }

    /// Display form of the measured value — see
    /// [`Measurement::value_display_str`], which this delegates to.
    ///
    /// Keeps the meter's own spacing for a steady on-screen width. CSV export
    /// goes through [`Measurement::value_export_str`] instead, where that
    /// spacing would make the column non-numeric.
    pub fn value_str(&self) -> String {
        self.measurement.value_display_str().into_owned()
    }

    pub fn unit(&self) -> &str {
        &self.measurement.unit
    }

    pub fn flags_str(&self) -> String {
        self.measurement.flags.to_string()
    }
}

/// In-memory recording buffer.
#[derive(Debug)]
pub struct Recording {
    pub active: bool,
    pub samples: VecDeque<Sample>,
    /// Session time the current recording started at, from the caller's
    /// [`Clock`](dmm_lib::Clock). Session time rather than wall time so a
    /// mock run on a bent clock shows a duration its samples agree with.
    pub start_time: Option<Instant>,
    /// How many samples are known to have reached a file, of either export
    /// format. Compared against `samples.len()` to tell whether discarding the
    /// buffer would lose anything the user hasn't saved.
    exported_count: usize,
    /// Which filling of the buffer the samples belong to, bumped whenever it
    /// is emptied for a new one. An export names the epoch it rendered, so a
    /// save dialog that outlives its buffer cannot mark the next one saved.
    epoch: u64,
    /// Most sub-values any buffered sample carries. The export sizes its aux
    /// columns from the device profile, but a profile is only known while
    /// connected — this is the floor that keeps a capture exportable in full
    /// after the meter is unplugged.
    max_aux_seen: usize,
    /// Samples this recording stops at, from the Buffer size setting (which
    /// bounds the graph history by the same number).
    ///
    /// A sample carrying only the meter's main reading is roughly 340 bytes —
    /// about 240 inline, plus the `display_raw` heap string and the meter's
    /// own frame — so the default 500K is on the order of 170 MB. Each
    /// sub-value the meter sends adds another ~140 bytes, which puts a
    /// four-sub-value meter (UT181A) at ~900 bytes per sample, or ~450 MB at
    /// the same bound.
    max_samples: usize,
}

impl Recording {
    pub fn new() -> Self {
        Self {
            active: false,
            samples: VecDeque::new(),
            start_time: None,
            exported_count: 0,
            epoch: 0,
            max_aux_seen: 0,
            max_samples: DEFAULT_MAX_SAMPLES,
        }
    }

    /// Change the sample bound. Returns `true` if an active recording was
    /// stopped because it already held at least `n` samples.
    ///
    /// The buffered samples are left alone: a recording never throws away
    /// what it has captured, so lowering the bound past a running capture
    /// ends it rather than truncating it.
    pub fn set_max_samples(&mut self, n: usize) -> bool {
        self.max_samples = n;
        let stopped = self.active && self.samples.len() >= n;
        if stopped {
            self.active = false;
        }
        stopped
    }

    /// Start or stop recording, `now` being the session time it happens at.
    pub fn toggle(&mut self, now: Instant) {
        self.active = !self.active;
        if self.active {
            self.samples.clear();
            self.exported_count = 0;
            self.epoch += 1;
            self.max_aux_seen = 0;
            self.start_time = Some(now);
        }
    }

    /// The buffer's current filling — see `epoch`.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Most sub-values any buffered sample carries — see `max_aux_seen`.
    pub fn max_aux_seen(&self) -> usize {
        self.max_aux_seen
    }

    /// Samples captured since the last successful export.
    ///
    /// Non-zero means clearing the buffer would destroy data that exists
    /// nowhere else, which is what the Record confirmation prompt checks.
    pub fn unexported_count(&self) -> usize {
        self.samples.len().saturating_sub(self.exported_count)
    }

    /// Record that the first `count` samples of `epoch` reached a file.
    ///
    /// Takes the count that was actually written rather than the current
    /// length: samples arriving while the export ran are not in that file and
    /// must still count as unexported. An export of an earlier epoch marks
    /// nothing — the samples it wrote are gone, and the ones in their place
    /// are in no file.
    pub fn mark_exported(&mut self, epoch: u64, count: usize) {
        if epoch == self.epoch {
            self.exported_count = count.min(self.samples.len());
        }
    }

    /// Push a sample. Returns `true` if the buffer just became full (auto-stops recording).
    ///
    /// `extra_aux` is the caller's current [`Sample::extra_aux`]: how many of
    /// this reading's trailing sub-values software appended.
    pub fn push(&mut self, m: &Measurement, wall_clock: &WallClock, extra_aux: usize) -> bool {
        if self.active && self.samples.len() < self.max_samples {
            self.max_aux_seen = self.max_aux_seen.max(m.aux_values.len());
            self.samples
                .push_back(Sample::from_measurement(m, wall_clock, extra_aux));
            if self.samples.len() >= self.max_samples {
                self.active = false;
                return true;
            }
        }
        false
    }

    pub fn is_full(&self) -> bool {
        self.samples.len() >= self.max_samples
    }

    /// How long the current recording has been running, in session seconds.
    ///
    /// `checked_duration_since` rather than subtraction: a `now` from before
    /// the start reads as zero instead of panicking.
    pub fn duration_secs(&self, now: Instant) -> f64 {
        self.start_time
            .and_then(|start| now.checked_duration_since(start))
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

impl Default for Recording {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::measurement::{AuxValue, MeasuredValue};
    use dmm_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;
    use std::time::Duration;

    fn make_measurement(display: &[u8; 7]) -> Measurement {
        let payload: Vec<u8> = vec![
            0x02, // mode: DcV (raw, no 0x30)
            0x31, // range: 1 (with 0x30 prefix)
            display[0], display[1], display[2], display[3], display[4], display[5], display[6],
            0x00, 0x00, // progress (raw)
            0x30, 0x30, 0x30, // flags (with 0x30 prefix, all zero = AUTO on)
        ];
        let table = Ut61ePlusTable::new();
        dmm_lib::protocol::ut61eplus::parse_measurement(&payload, &table).unwrap()
    }

    /// The GUI's export never integrates — that is a CLI-only run mode.
    fn layout(family_slots: usize, extra_slots: usize) -> CsvLayout {
        CsvLayout {
            family_slots,
            extra_slots,
            integral: false,
        }
    }

    #[test]
    fn recording_inactive_by_default() {
        let r = Recording::new();
        assert!(!r.active);
        assert!(r.samples.is_empty());
    }

    #[test]
    fn recording_toggle_starts_and_stops() {
        let mut r = Recording::new();
        r.toggle(Instant::now());
        assert!(r.active);
        assert!(r.start_time.is_some());
        r.toggle(Instant::now());
        assert!(!r.active);
    }

    /// The panel counts session seconds, not real ones: on a scaled or
    /// preseeded mock clock the duration has to match the samples it labels.
    #[test]
    fn duration_follows_the_instant_it_is_given() {
        let mut r = Recording::new();
        assert_eq!(r.duration_secs(Instant::now()), 0.0, "nothing recorded yet");

        let start = Instant::now();
        r.toggle(start);
        assert_eq!(r.duration_secs(start + Duration::from_secs(90)), 90.0);
        // A clock that went backwards reads as zero rather than panicking.
        assert_eq!(r.duration_secs(start - Duration::from_secs(1)), 0.0);
    }

    #[test]
    fn recording_only_captures_when_active() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.push(&m, &wc, 0);
        assert!(r.samples.is_empty());

        r.toggle(Instant::now()); // start
        r.push(&m, &wc, 0);
        assert_eq!(r.samples.len(), 1);
    }

    #[test]
    fn recording_toggle_clears_previous() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        let m = make_measurement(b"  1.234");
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.samples.len(), 2);

        r.toggle(Instant::now()); // stop
        r.toggle(Instant::now()); // start again — should clear
        assert!(r.samples.is_empty());
    }

    /// The Record button clears the buffer, so this is what decides whether
    /// pressing it would destroy data that exists nowhere else.
    #[test]
    fn unexported_count_tracks_samples_since_the_last_export() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");

        assert_eq!(r.unexported_count(), 0, "empty buffer has nothing to lose");

        r.toggle(Instant::now());
        for _ in 0..3 {
            r.push(&m, &wc, 0);
        }
        assert_eq!(r.unexported_count(), 3);

        r.mark_exported(r.epoch(), 3);
        assert_eq!(r.unexported_count(), 0);

        r.push(&m, &wc, 0);
        assert_eq!(r.unexported_count(), 1, "samples after an export count");
    }

    /// An export writes a snapshot; samples captured while it ran are not in
    /// that file and must not be counted as saved.
    #[test]
    fn samples_arriving_during_an_export_stay_unexported() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        for _ in 0..5 {
            r.push(&m, &wc, 0);
        }
        // Export snapshots 5, two more arrive before it completes.
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        r.mark_exported(r.epoch(), 5);
        assert_eq!(r.unexported_count(), 2);
    }

    #[test]
    fn starting_a_new_recording_resets_the_export_mark() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        r.push(&m, &wc, 0);
        r.mark_exported(r.epoch(), 1);
        r.toggle(Instant::now()); // stop
        r.toggle(Instant::now()); // start again — buffer cleared
        assert_eq!(r.unexported_count(), 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.unexported_count(), 1, "new samples are unexported again");
    }

    /// An export still open when the next recording started wrote the old
    /// samples, not the new ones — marking them saved would let Record
    /// discard them without asking.
    #[test]
    fn an_export_of_an_earlier_recording_marks_nothing() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        for _ in 0..3 {
            r.push(&m, &wc, 0);
        }
        let exporting = r.epoch();
        r.toggle(Instant::now()); // stop
        r.toggle(Instant::now()); // start again while the dialog is open
        assert_ne!(r.epoch(), exporting, "a new recording is a new epoch");
        for _ in 0..2 {
            r.push(&m, &wc, 0);
        }
        r.mark_exported(exporting, 3);
        assert_eq!(r.unexported_count(), 2);
    }

    /// A stale count from a bigger previous buffer must not mask real data.
    #[test]
    fn export_mark_cannot_exceed_the_buffer() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        r.push(&m, &wc, 0);
        r.mark_exported(r.epoch(), 99);
        assert_eq!(r.unexported_count(), 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.unexported_count(), 1);
    }

    #[test]
    fn recording_auto_stops_when_full() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        // A bound of its own, so the test doesn't buffer half a million
        // samples to prove the stop.
        r.set_max_samples(100);
        let m = make_measurement(b"  1.234");
        // Fill to one below capacity
        for _ in 0..99 {
            assert!(!r.push(&m, &wc, 0));
            assert!(r.active);
        }
        // The push that hits capacity should auto-stop and return true
        assert!(r.push(&m, &wc, 0));
        assert!(!r.active);
        assert_eq!(r.samples.len(), 100);
        assert!(r.is_full());
    }

    #[test]
    fn recording_push_after_auto_stop_is_noop() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        r.set_max_samples(100);
        let m = make_measurement(b"  1.234");
        for _ in 0..100 {
            r.push(&m, &wc, 0);
        }
        assert!(!r.active);
        // Further pushes should be no-ops
        assert!(!r.push(&m, &wc, 0));
        assert_eq!(r.samples.len(), 100);
    }

    /// Lowering the Buffer size setting under a running recording ends it —
    /// and keeps every sample it had already captured, which is the whole
    /// point of a recording being separate from the graph's history.
    #[test]
    fn recording_lowering_the_cap_below_the_buffer_auto_stops() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        let m = make_measurement(b"  1.234");
        for _ in 0..50 {
            r.push(&m, &wc, 0);
        }
        assert!(
            r.set_max_samples(20),
            "the running capture had to be stopped"
        );
        assert!(!r.active);
        assert_eq!(r.samples.len(), 50, "captured samples are never discarded");
        assert!(r.is_full());
    }

    #[test]
    fn recording_raising_the_cap_keeps_it_running() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        r.set_max_samples(100);
        let m = make_measurement(b"  1.234");
        for _ in 0..50 {
            r.push(&m, &wc, 0);
        }
        assert!(!r.set_max_samples(1_000));
        assert!(r.active);
        assert!(!r.is_full());
        assert!(!r.push(&m, &wc, 0));
        assert_eq!(r.samples.len(), 51);
    }

    #[test]
    fn sample_from_measurement() {
        let m = make_measurement(b"  5.678");
        let wc = WallClock::new();
        let s = Sample::from_measurement(&m, &wc, 0);
        assert_eq!(s.measurement.mode, "DC V");
        assert_eq!(s.value_str(), "5.678");
        assert_eq!(s.unit(), "V");
    }

    /// The recording panel and the CSV must never disagree about an overload:
    /// `value_export_str` has always said "OL", so `value_str` has to as well
    /// even when the protocol left digits in `display_raw`.
    #[test]
    fn sample_value_str_reports_overload_not_digits() {
        let mut m = make_measurement(b"      0");
        m.value = MeasuredValue::Overload;
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);
        assert_eq!(s.value_str(), "OL");
        assert_eq!(s.measurement.value_export_str(), "OL");
    }

    #[test]
    fn sample_value_str_reports_ncv_not_digits() {
        let mut m = make_measurement(b"  1.234");
        m.value = MeasuredValue::NcvLevel(2);
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);
        assert_eq!(s.value_str(), "NCV:2");
        assert_eq!(s.measurement.value_export_str(), "NCV:2");
    }

    /// The wire bytes are what a replay export writes, so a buffered sample
    /// has to keep the frame it was decoded from.
    #[test]
    fn stored_samples_keep_the_meters_frame() {
        let m = make_measurement(b"  1.234");
        assert!(
            !m.raw_payload.is_empty(),
            "fixture should carry wire bytes to begin with"
        );
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);
        assert_eq!(s.measurement.raw_payload, m.raw_payload);
    }

    #[test]
    fn render_csv_has_header_and_one_row_per_sample() {
        let wc = WallClock::new();
        let m = make_measurement(b"  5.678");
        let samples: VecDeque<Sample> = (0..3)
            .map(|_| Sample::from_measurement(&m, &wc, 0))
            .collect();

        let bytes = render_csv(&samples, "UNI-T UT61E+", layout(0, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(lines[0], "# device: UNI-T UT61E+");
        assert_eq!(lines[1], "timestamp,mode,value,unit,range,flags");
        assert_eq!(lines.len(), 5, "header + column row + 3 samples");
        assert!(lines[2].contains("DC V"), "got {:?}", lines[2]);
        assert!(lines[2].contains("5.678"), "got {:?}", lines[2]);
    }

    #[test]
    fn render_csv_of_an_empty_buffer_is_just_the_headers() {
        let bytes = render_csv(&VecDeque::new(), "mock", layout(0, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text.lines().count(), 2);
    }

    /// A single-display export is the one file both binaries write, so its
    /// header line is pinned to the same string the CLI asserts in
    /// `csv_header_names_one_group_per_aux_slot` — renaming a column in one
    /// exporter alone breaks a test.
    #[test]
    fn gui_and_cli_single_display_headers_agree() {
        let bytes = render_csv(&VecDeque::new(), "mock", layout(0, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text.lines().nth(1).unwrap(),
            "timestamp,mode,value,unit,range,flags"
        );
    }

    /// Sub-value fixture in the shape the protocols produce: digits in
    /// `display_raw`, unit left empty when it matches the main reading's.
    fn aux(label: &'static str, display: &str, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(display.trim().parse().unwrap_or(0.0)),
            unit: unit.into(),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
    }

    /// The column layout is fixed for the whole file, so a reading with
    /// fewer sub-values than the family can send has to leave the trailing
    /// slots empty rather than shortening the row.
    #[test]
    fn render_csv_pads_missing_aux_slots() {
        let mut m = make_measurement(b"  5.678");
        m.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);

        let bytes = render_csv(&VecDeque::from([s]), "UNI-T UT181A", layout(4, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(
            lines[1],
            "timestamp,mode,value,unit,range,flags,\
             aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,\
             aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit"
        );
        assert!(
            lines[2].ends_with(",Frequency,50.01,Hz,Period,20.00,ms,,,,,,"),
            "got {:?}",
            lines[2]
        );
        assert_eq!(
            lines[1].split(',').count(),
            lines[2].split(',').count(),
            "every row must have as many fields as the header"
        );
    }

    /// Protocols leave a sub-value's unit empty when it measures the same
    /// quantity as the main reading (MIN/MAX). The export has to fill it in,
    /// or the column reads as unitless.
    #[test]
    fn render_csv_resolves_empty_aux_unit_to_main() {
        let mut m = make_measurement(b"  5.678");
        let mut max = aux("Max", "5.9010", "");
        max.elapsed_secs = Some(12);
        m.aux_values = vec![max];
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);

        let bytes = render_csv(&VecDeque::from([s]), "UNI-T UT181A", layout(1, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let row = text.lines().nth(2).unwrap();
        assert!(row.ends_with(",Max,5.9010,V"), "got {row:?}");
    }

    /// A row carrying more sub-values than the declared slot count must be
    /// truncated, not allowed to push extra fields past the header.
    #[test]
    fn render_csv_truncates_extra_aux_values() {
        let mut m = make_measurement(b"  5.678");
        m.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);

        let bytes = render_csv(&VecDeque::from([s]), "mock", layout(1, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[2].ends_with(",Frequency,50.01,Hz"),
            "got {:?}",
            lines[2]
        );
        assert_eq!(lines[1].split(',').count(), lines[2].split(',').count());
    }

    /// A transform's `Raw` is appended after whatever sub-values the meter
    /// sent, so with a single shared slot count it slid between `aux1` and
    /// `aux3` as the meter changed mode mid-file — three quantities in one
    /// column. The trailing extra group pins it.
    ///
    /// A row recorded before the scale was switched on carries no `Raw`, and
    /// says so through its own `extra_aux`: its Frequency stays in `aux1`
    /// rather than being mistaken for the appended sub-value and filed under
    /// the `Raw` column.
    #[test]
    fn render_csv_pins_appended_sub_values_to_the_trailing_group() {
        let wc = WallClock::new();

        // Recorded before the scale: the meter's Frequency, no Raw.
        let mut before = make_measurement(b"  5.678");
        before.aux_values = vec![aux("Frequency", "50.01", "Hz")];

        // Scale on, meter in a mode with no sub-values of its own.
        let mut bare = make_measurement(b"  5.678");
        bare.aux_values = vec![aux("Raw", "0.05678", "")];

        // Scale on, meter sending both of its sub-values.
        let mut wide = make_measurement(b"  5.678");
        wide.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
            aux("Raw", "0.05678", ""),
        ];

        let samples: VecDeque<Sample> = [(&before, 0), (&bare, 1), (&wide, 1)]
            .into_iter()
            .map(|(m, extra)| Sample::from_measurement(m, &wc, extra))
            .collect();

        let bytes = render_csv(&samples, "UNI-T UT181A", layout(2, 1)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(
            lines[1],
            "timestamp,mode,value,unit,range,flags,\
             aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,\
             aux3_label,aux3_value,aux3_unit"
        );
        assert!(
            lines[2].ends_with(",Frequency,50.01,Hz,,,,,,"),
            "recorded before the scale — Frequency in aux1, Raw group empty: {:?}",
            lines[2]
        );
        assert!(
            lines[3].ends_with(",,,,,,,Raw,0.05678,V"),
            "no meter sub-values, Raw still third: {:?}",
            lines[3]
        );
        assert!(
            lines[4].ends_with(",Frequency,50.01,Hz,Period,20.00,ms,Raw,0.05678,V"),
            "two meter sub-values, Raw still third: {:?}",
            lines[4]
        );
        for row in &lines[2..] {
            assert_eq!(
                lines[1].split(',').count(),
                row.split(',').count(),
                "every row must have as many fields as the header"
            );
        }
    }

    /// The export sizes its columns from the device profile, but a capture
    /// outlives the connection — this floor keeps an unplugged meter's
    /// sub-values in the file, and must not leak into the next recording.
    #[test]
    fn max_aux_seen_tracks_the_widest_sample_and_resets_on_start() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let plain = make_measurement(b"  1.234");
        let mut wide = make_measurement(b"  1.234");
        wide.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];

        assert_eq!(r.max_aux_seen(), 0);
        r.toggle(Instant::now());
        r.push(&plain, &wc, 0);
        assert_eq!(r.max_aux_seen(), 0);
        r.push(&wide, &wc, 0);
        assert_eq!(r.max_aux_seen(), 2);
        r.push(&plain, &wc, 0);
        assert_eq!(r.max_aux_seen(), 2, "the widest sample wins, not the last");

        r.toggle(Instant::now()); // stop
        assert_eq!(r.max_aux_seen(), 2, "still exportable after stopping");
        r.toggle(Instant::now()); // start again — buffer cleared
        assert_eq!(r.max_aux_seen(), 0);
    }

    #[test]
    fn sample_wall_time_derived_from_measurement_timestamp() {
        // Build a WallClock whose origin is "now", then construct two
        // measurements with Instants 500ms apart. The first Sample's wall_time
        // should equal the WallClock's system origin; the second should be
        // exactly 500ms later, regardless of when `from_measurement` is
        // actually called.
        let wc = WallClock::new();
        let mut m1 = make_measurement(b"  1.000");
        let mut m2 = make_measurement(b"  2.000");
        m1.timestamp = std::time::Instant::now();
        m2.timestamp = m1.timestamp + Duration::from_millis(500);

        let s1 = Sample::from_measurement(&m1, &wc, 0);
        let s2 = Sample::from_measurement(&m2, &wc, 0);

        let delta = s2.wall_time.signed_duration_since(s1.wall_time);
        assert_eq!(delta.num_milliseconds(), 500);
    }

    /// Three frames 250 ms apart, as the buffer would hold them.
    fn replay_samples() -> VecDeque<Sample> {
        let wc = WallClock::new();
        let base = Instant::now();
        (0..3)
            .map(|i| {
                let mut m = make_measurement(b"  1.234");
                m.timestamp = base + Duration::from_millis(250 * i);
                Sample::from_measurement(&m, &wc, 0)
            })
            .collect()
    }

    /// What the GUI writes has to be what `--replay` reads, down to the date
    /// format the binaries turn into a clock origin — the library keeps the
    /// `# recorded:` line verbatim and never looks at it.
    #[test]
    fn render_replay_round_trips_through_the_parser() {
        let samples = replay_samples();
        let text = render_replay(&samples, "ut61eplus", Some("UNI-T UT61E+"))
            .expect("frames with wire bytes");

        let replay = dmm_lib::replay::Replay::parse(&text).expect("a well-formed recording");
        assert_eq!(replay.device.id, "ut61eplus");
        assert_eq!(replay.model.as_deref(), Some("UNI-T UT61E+"));
        assert_eq!(replay.duration(), Duration::from_millis(500));
        chrono::DateTime::parse_from_rfc3339(&replay.recorded)
            .expect("`# recorded:` is what --replay parses as a clock origin");

        // Offsets run from the first frame, whatever session time it landed
        // at, and carry the frame the meter actually sent.
        let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines.len(), 3);
        assert!(
            lines[0].starts_with("0 02 31 20 20 31 2E 32 33 34"),
            "{:?}",
            lines[0]
        );
        assert!(lines[1].starts_with("250 "), "{:?}", lines[1]);
        assert!(lines[2].starts_with("500 "), "{:?}", lines[2]);
    }

    /// The `# model:` line is optional, and a meter that never named itself
    /// must not produce an empty one the parser would have to skip.
    #[test]
    fn render_replay_leaves_out_an_unknown_model() {
        let text = render_replay(&replay_samples(), "ut61eplus", None).expect("frames");
        assert!(!text.contains("# model:"), "{text}");
        assert_eq!(
            dmm_lib::replay::Replay::parse(&text).expect("parses").model,
            None
        );
    }

    /// The mock synthesises its readings, so its samples carry no frame. A
    /// file of empty sample lines would not parse back — refuse it here, where
    /// the caller can still say so.
    #[test]
    fn render_replay_refuses_a_sample_without_a_frame() {
        let mut samples = replay_samples();
        samples[1].measurement.raw_payload = Vec::new();
        assert!(render_replay(&samples, "ut61eplus", None).is_none());
        assert!(render_replay(&VecDeque::new(), "ut61eplus", None).is_none());
    }

    /// The export and `dmm-cli read --format json` cannot drift, because both
    /// are these two calls into `dmm_shared::export` — so this checks the
    /// document the GUI builds around them, not the objects themselves.
    #[test]
    fn render_json_is_the_metadata_line_and_one_object_per_sample() {
        let mut samples = replay_samples();
        samples.truncate(2);
        let text = render_json(&samples, "UNI-T UT61E+", true);

        let mut expected = dmm_shared::export::metadata_line("UNI-T UT61E+");
        for s in &samples {
            expected.push('\n');
            expected.push_str(
                &dmm_shared::export::measurement_json(
                    &s.measurement,
                    &s.wall_time.to_rfc3339(),
                    true,
                    None,
                )
                .to_string(),
            );
        }
        expected.push('\n');
        assert_eq!(text, expected);

        // And every line of it is a JSON object, as a script reading it
        // line by line expects.
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in lines {
            let v: serde_json::Value = serde_json::from_str(line).expect("a JSON object per line");
            assert!(v.is_object(), "{line}");
        }
        assert!(lines_hold_the_reading(&text), "{text}");
    }

    /// The fields a reader of the GUI's JSON would look for, on the line the
    /// first sample wrote.
    fn lines_hold_the_reading(text: &str) -> bool {
        let Some(first) = text.lines().nth(1) else {
            return false;
        };
        let v: serde_json::Value = serde_json::from_str(first).expect("a JSON object");
        v["mode"] == "DC V" && v["value"] == 1.234 && v["unit"] == "V" && v["experimental"] == true
    }
}
