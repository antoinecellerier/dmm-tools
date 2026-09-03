use chrono::{DateTime, Local};
use dmm_lib::WallClock;
use dmm_lib::measurement::{AUX_EXPORT_COLUMNS, Measurement};
use std::borrow::Cow;
use std::io::Write;

/// Render samples as a CSV document, provenance header included.
///
/// Returns the finished bytes so the caller can hand them to a writer thread
/// without duplicating the sample buffer — a full buffer is ~140 MB of
/// `Sample`s, while the rendered CSV is a fraction of that and takes one pass
/// instead of half a million allocations.
///
/// `family_slots` is the meter family's `max_aux_values`, not the count in any
/// one reading: a CSV needs one fixed column layout for the whole file, so a
/// mode that reports fewer sub-values leaves the trailing slots empty.
/// `extra_slots` reserves trailing columns for the sub-values software appends
/// after the meter's own (a transform's `Raw`), so they stay in one place
/// whatever the meter sent that frame — see [`Measurement::export_aux_slots`].
/// It is the widest any sample needs; each row claims only as many as its own
/// [`Sample::extra_aux`] says it carries, and leaves the rest empty. Pass 0
/// and 0 for a single-display meter with no transform and the file keeps the
/// six columns it always had.
pub fn render_csv(
    samples: &[Sample],
    device_model: &str,
    family_slots: usize,
    extra_slots: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let aux_slots = family_slots + extra_slots;
    // ~72 bytes covers a typical row (RFC3339 timestamp, mode, value, unit,
    // range, flags) without repeated growth on large buffers; each aux slot
    // adds roughly another 20.
    let row_bytes = 72 + aux_slots * 20;
    let mut buf: Vec<u8> = Vec::with_capacity(samples.len() * row_bytes + 128);
    writeln!(buf, "# device: {device_model}")?;
    {
        let mut wtr = csv::Writer::from_writer(&mut buf);
        let mut header: Vec<Cow<'static, str>> =
            ["timestamp", "mode", "value", "unit", "range", "flags"]
                .into_iter()
                .map(Cow::Borrowed)
                .collect();
        for i in 1..=aux_slots {
            for suffix in AUX_EXPORT_COLUMNS {
                header.push(Cow::Owned(format!("aux{i}_{suffix}")));
            }
        }
        wtr.write_record(header.iter().map(|c| c.as_ref()))?;

        let mut record: Vec<Cow<'_, str>> =
            Vec::with_capacity(6 + aux_slots * AUX_EXPORT_COLUMNS.len());
        for s in samples {
            record.clear();
            record.push(Cow::Owned(s.wall_time.to_rfc3339()));
            record.push(Cow::Borrowed(s.mode()));
            record.push(s.value_export_str());
            record.push(Cow::Borrowed(s.unit()));
            record.push(Cow::Borrowed(s.range_label()));
            record.push(Cow::Owned(s.flags_str()));
            // One fixed layout for the file: the helper pads the meter's own
            // slots, pins the appended ones to the trailing group, and
            // truncates a surplus rather than desyncing every column after it
            // from the header.
            //
            // Only the extras *this* sample carries are claimed. A row
            // recorded before a mid-recording scale has none, and letting the
            // helper claim one anyway would read its last meter sub-value as
            // the appended one — filing Frequency under the `Raw` column.
            let m = &s.measurement;
            let extra = s.extra_aux.min(extra_slots);
            for slot in m.export_aux_slots(family_slots, extra) {
                match slot {
                    Some(aux) => record.extend(aux.export_cells(&m.unit)),
                    None => {
                        for _ in 0..AUX_EXPORT_COLUMNS.len() {
                            record.push(Cow::Borrowed(""));
                        }
                    }
                }
            }
            for _ in 0..(extra_slots - extra) * AUX_EXPORT_COLUMNS.len() {
                record.push(Cow::Borrowed(""));
            }
            wtr.write_record(record.iter().map(|c| c.as_ref()))?;
        }
        wtr.flush()?;
    }
    Ok(buf)
}

/// Maximum recording samples (~14 hours at 10Hz).
///
/// A `Sample` is roughly 280 bytes — about 240 inline plus the `display_raw`
/// heap string — so a full buffer holds on the order of 140 MB. (The figure
/// quoted here used to be 22 MB, which was never achievable at this struct
/// size.)
const MAX_RECORDING_SAMPLES: usize = 500_000;

/// A single recorded sample.
///
/// Holds the underlying `Measurement` directly so both the recording panel
/// and CSV export consume exactly the same data shape the protocol produced,
/// and static-lookup-table strings (`mode`, `unit`, `range_label`) stay as
/// `Cow::Borrowed` instead of being re-cloned onto the heap for every sample.
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
        let mut measurement = m.clone();
        // Drop the debug-only wire bytes. Nothing in the GUI reads them, and
        // retaining a heap Vec per sample costs ~50 MB across a full buffer.
        measurement.raw_payload = Vec::new();
        Self {
            wall_time: wall_clock.wall_time_for(m.timestamp).into(),
            measurement,
            extra_aux,
        }
    }

    /// Display form of the measured value — see
    /// [`Measurement::value_display_str`], which this delegates to.
    ///
    /// Keeps the meter's own spacing for a steady on-screen width. Use
    /// [`Sample::value_export_str`] for CSV, where that spacing would make the
    /// column non-numeric.
    pub fn value_str(&self) -> String {
        self.measurement.value_display_str().into_owned()
    }

    /// Value formatted for CSV export — see [`Measurement::value_export_str`].
    pub fn value_export_str(&self) -> std::borrow::Cow<'_, str> {
        self.measurement.value_export_str()
    }

    pub fn mode(&self) -> &str {
        &self.measurement.mode
    }

    pub fn unit(&self) -> &str {
        &self.measurement.unit
    }

    pub fn range_label(&self) -> &str {
        &self.measurement.range_label
    }

    pub fn flags_str(&self) -> String {
        self.measurement.flags.to_string()
    }
}

/// In-memory recording buffer.
#[derive(Debug)]
pub struct Recording {
    pub active: bool,
    pub samples: Vec<Sample>,
    pub start_time: Option<DateTime<Local>>,
    /// How many samples are known to have reached a CSV file. Compared
    /// against `samples.len()` to tell whether discarding the buffer would
    /// lose anything the user hasn't saved.
    exported_count: usize,
    /// Most sub-values any buffered sample carries. The export sizes its aux
    /// columns from the device profile, but a profile is only known while
    /// connected — this is the floor that keeps a capture exportable in full
    /// after the meter is unplugged.
    max_aux_seen: usize,
}

impl Recording {
    pub fn new() -> Self {
        Self {
            active: false,
            samples: Vec::new(),
            start_time: None,
            exported_count: 0,
            max_aux_seen: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if self.active {
            self.samples.clear();
            self.exported_count = 0;
            self.max_aux_seen = 0;
            self.start_time = Some(Local::now());
        }
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

    /// Record that the first `count` samples reached a file.
    ///
    /// Takes the count that was actually written rather than the current
    /// length: samples arriving while the export ran are not in that file and
    /// must still count as unexported.
    pub fn mark_exported(&mut self, count: usize) {
        self.exported_count = count.min(self.samples.len());
    }

    /// Push a sample. Returns `true` if the buffer just became full (auto-stops recording).
    ///
    /// `extra_aux` is the caller's current [`Sample::extra_aux`]: how many of
    /// this reading's trailing sub-values software appended.
    pub fn push(&mut self, m: &Measurement, wall_clock: &WallClock, extra_aux: usize) -> bool {
        if self.active && self.samples.len() < MAX_RECORDING_SAMPLES {
            self.max_aux_seen = self.max_aux_seen.max(m.aux_values.len());
            self.samples
                .push(Sample::from_measurement(m, wall_clock, extra_aux));
            if self.samples.len() >= MAX_RECORDING_SAMPLES {
                self.active = false;
                return true;
            }
        }
        false
    }

    pub fn is_full(&self) -> bool {
        self.samples.len() >= MAX_RECORDING_SAMPLES
    }

    pub fn duration_secs(&self) -> f64 {
        self.start_time
            .map(|start| (Local::now() - start).num_milliseconds() as f64 / 1000.0)
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

    #[test]
    fn recording_inactive_by_default() {
        let r = Recording::new();
        assert!(!r.active);
        assert!(r.samples.is_empty());
    }

    #[test]
    fn recording_toggle_starts_and_stops() {
        let mut r = Recording::new();
        r.toggle();
        assert!(r.active);
        assert!(r.start_time.is_some());
        r.toggle();
        assert!(!r.active);
    }

    #[test]
    fn recording_only_captures_when_active() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.push(&m, &wc, 0);
        assert!(r.samples.is_empty());

        r.toggle(); // start
        r.push(&m, &wc, 0);
        assert_eq!(r.samples.len(), 1);
    }

    #[test]
    fn recording_toggle_clears_previous() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.samples.len(), 2);

        r.toggle(); // stop
        r.toggle(); // start again — should clear
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

        r.toggle();
        for _ in 0..3 {
            r.push(&m, &wc, 0);
        }
        assert_eq!(r.unexported_count(), 3);

        r.mark_exported(3);
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
        r.toggle();
        for _ in 0..5 {
            r.push(&m, &wc, 0);
        }
        // Export snapshots 5, two more arrive before it completes.
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        r.mark_exported(5);
        assert_eq!(r.unexported_count(), 2);
    }

    #[test]
    fn starting_a_new_recording_resets_the_export_mark() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle();
        r.push(&m, &wc, 0);
        r.mark_exported(1);
        r.toggle(); // stop
        r.toggle(); // start again — buffer cleared
        assert_eq!(r.unexported_count(), 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.unexported_count(), 1, "new samples are unexported again");
    }

    /// A stale count from a bigger previous buffer must not mask real data.
    #[test]
    fn export_mark_cannot_exceed_the_buffer() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle();
        r.push(&m, &wc, 0);
        r.mark_exported(99);
        assert_eq!(r.unexported_count(), 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.unexported_count(), 1);
    }

    #[test]
    fn recording_auto_stops_when_full() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        // Fill to one below capacity
        for _ in 0..MAX_RECORDING_SAMPLES - 1 {
            assert!(!r.push(&m, &wc, 0));
            assert!(r.active);
        }
        // The push that hits capacity should auto-stop and return true
        assert!(r.push(&m, &wc, 0));
        assert!(!r.active);
        assert_eq!(r.samples.len(), MAX_RECORDING_SAMPLES);
        assert!(r.is_full());
    }

    #[test]
    fn recording_push_after_auto_stop_is_noop() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle();
        let m = make_measurement(b"  1.234");
        for _ in 0..MAX_RECORDING_SAMPLES {
            r.push(&m, &wc, 0);
        }
        assert!(!r.active);
        // Further pushes should be no-ops
        assert!(!r.push(&m, &wc, 0));
        assert_eq!(r.samples.len(), MAX_RECORDING_SAMPLES);
    }

    #[test]
    fn sample_from_measurement() {
        let m = make_measurement(b"  5.678");
        let wc = WallClock::new();
        let s = Sample::from_measurement(&m, &wc, 0);
        assert_eq!(s.mode(), "DC V");
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
        assert_eq!(s.value_export_str(), "OL");
    }

    #[test]
    fn sample_value_str_reports_ncv_not_digits() {
        let mut m = make_measurement(b"  1.234");
        m.value = MeasuredValue::NcvLevel(2);
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);
        assert_eq!(s.value_str(), "NCV:2");
        assert_eq!(s.value_export_str(), "NCV:2");
    }

    /// The wire bytes are debug-only and nothing in the GUI reads them;
    /// keeping one heap Vec per sample cost ~50 MB across a full buffer.
    #[test]
    fn stored_samples_drop_the_debug_payload() {
        let m = make_measurement(b"  1.234");
        assert!(
            !m.raw_payload.is_empty(),
            "fixture should carry wire bytes to begin with"
        );
        let s = Sample::from_measurement(&m, &WallClock::new(), 0);
        assert!(s.measurement.raw_payload.is_empty());
    }

    #[test]
    fn render_csv_has_header_and_one_row_per_sample() {
        let wc = WallClock::new();
        let m = make_measurement(b"  5.678");
        let samples: Vec<Sample> = (0..3)
            .map(|_| Sample::from_measurement(&m, &wc, 0))
            .collect();

        let bytes = render_csv(&samples, "UNI-T UT61E+", 0, 0).unwrap();
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
        let bytes = render_csv(&[], "mock", 0, 0).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text.lines().count(), 2);
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

        let bytes = render_csv(&[s], "UNI-T UT181A", 4, 0).unwrap();
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

        let bytes = render_csv(&[s], "UNI-T UT181A", 1, 0).unwrap();
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

        let bytes = render_csv(&[s], "mock", 1, 0).unwrap();
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

        let samples: Vec<Sample> = [(&before, 0), (&bare, 1), (&wide, 1)]
            .into_iter()
            .map(|(m, extra)| Sample::from_measurement(m, &wc, extra))
            .collect();

        let bytes = render_csv(&samples, "UNI-T UT181A", 2, 1).unwrap();
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
        r.toggle();
        r.push(&plain, &wc, 0);
        assert_eq!(r.max_aux_seen(), 0);
        r.push(&wide, &wc, 0);
        assert_eq!(r.max_aux_seen(), 2);
        r.push(&plain, &wc, 0);
        assert_eq!(r.max_aux_seen(), 2, "the widest sample wins, not the last");

        r.toggle(); // stop
        assert_eq!(r.max_aux_seen(), 2, "still exportable after stopping");
        r.toggle(); // start again — buffer cleared
        assert_eq!(r.max_aux_seen(), 0);
    }

    #[test]
    fn sample_wall_time_derived_from_measurement_timestamp() {
        use std::time::Duration;
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
}
