use crate::markers::{Marker, Markers};
use crate::settings::DEFAULT_MAX_SAMPLES;
use chrono::{DateTime, Local, SecondsFormat};
use dmm_lib::WallClock;
use dmm_lib::export::{CsvLayout, device_comment};
use dmm_lib::measurement::Measurement;
use dmm_lib::replay;
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::ops::Range;
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
///
/// `marked` are the markers on buffered samples, oldest first (see
/// [`Recording::marked`]), written when `layout` has marker columns.
pub fn render_csv(
    samples: std::collections::vec_deque::Iter<'_, Sample>,
    marked: &[&Marker],
    device_model: &str,
    layout: CsvLayout,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut marks = MarkCursor::new(marked);
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
            let marker = marks.on(s);
            let cells = layout.row(&s.measurement, &ts, None, s.extra_aux, marker);
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
///
/// `marked` are the markers on buffered samples, oldest first, as for
/// [`render_csv`].
pub(crate) fn render_json(
    samples: std::collections::vec_deque::Iter<'_, Sample>,
    marked: &[&Marker],
    device_model: &str,
    experimental: bool,
) -> String {
    let mut marks = MarkCursor::new(marked);
    let mut out = dmm_shared::export::metadata_line(device_model);
    out.push('\n');
    // A reading with no sub-values runs to roughly 400 bytes; growing from
    // there beats growing from nothing on a half-million-sample buffer.
    out.reserve(samples.len() * 400);
    for s in samples {
        let ts = s.wall_time.to_rfc3339();
        out.push_str(
            &dmm_shared::export::measurement_json(
                &s.measurement,
                &ts,
                experimental,
                None,
                marks.on(s),
            )
            .to_string(),
        );
        out.push('\n');
    }
    out
}

/// Walks the markers alongside the samples as an export writes them out, both
/// in time order, so matching them costs one pass rather than a search per
/// sample.
struct MarkCursor<'a> {
    marked: std::iter::Peekable<std::slice::Iter<'a, &'a Marker>>,
}

impl<'a> MarkCursor<'a> {
    fn new(marked: &'a [&'a Marker]) -> Self {
        Self {
            marked: marked.iter().peekable(),
        }
    }

    /// The number and note of the marker on `s`, if any.
    fn on(&mut self, s: &Sample) -> Option<(u32, &'a str)> {
        let at = s.measurement.timestamp;
        // Markers on readings before this one were on no sample of this file.
        while self.marked.next_if(|m| m.at < at).is_some() {}
        self.marked
            .next_if(|m| m.at == at)
            .map(|m| (m.number, m.note.as_str()))
    }
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
/// `link` is what the samples arrived over, so a session played back from the
/// file is on the link the meter was.
///
/// `None` when any sample has an empty payload: the mock synthesises its
/// readings, so there is no frame to hand a parser.
pub(crate) fn render_replay(
    samples: std::collections::vec_deque::Iter<'_, Sample>,
    device_id: &str,
    model: Option<&str>,
    link: Option<dmm_lib::transport::Link>,
) -> Option<String> {
    let mut rest = samples.peekable();
    let first = *rest.peek()?;
    let recorded = first
        .wall_time
        .to_rfc3339_opts(SecondsFormat::Millis, false);
    let mut out = replay::header(device_id, &recorded, model, link);
    // Offset digits and a newline, plus three characters per payload byte.
    // Frame length is fixed per family, so the first sample sizes the rest.
    out.reserve(rest.len() * (10 + 3 * first.measurement.raw_payload.len()));
    for s in rest {
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
}

/// What Export… saves: a recording, when there is one, or else the graph's
/// history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferRole {
    /// Nothing recorded: Export… saves the readings the graph holds.
    /// Disposable, as the graph is.
    History,
    /// Record was pressed: Export… saves that recording, kept until the next
    /// one starts or it is discarded.
    Recording,
}

/// A recording's samples, by the sequence number each got as it was pushed.
#[derive(Debug)]
enum Span {
    /// In the store: from `start`, up to `end` once stopped (exclusive).
    InStore { start: u64, end: Option<u64> },
    /// Stopped and left behind by the history: its samples, moved out of the
    /// store so the store can let go of what came between.
    Detached(VecDeque<Sample>),
}

/// The full readings: the graph's history, and a recording as a slice of
/// the same store.
///
/// One store, so a reading both hold is paid for once. The history is the
/// store from `history_start` on, following the graph: it restarts with the
/// graph, on Clear and on a new meter. A recording is the samples pushed
/// between Record and Stop, and carries on across those restarts. While it
/// runs, both end at the newest reading and the store is the longer of the
/// two. Once stopped and passed by the history — the graph restarted, or
/// dropped its oldest — it moves out into a buffer of its own, so memory is
/// at most the recording plus the history.
#[derive(Debug)]
pub struct Recording {
    pub active: bool,
    /// Time-ordered, as readings arrive.
    store: VecDeque<Sample>,
    /// The sequence number of `store`'s first sample: one per push, counted
    /// from the start of the session, so a position survives the front being
    /// trimmed.
    front_seq: u64,
    /// Where the history starts; `None` for "at the next reading", after
    /// Clear or a new meter.
    history_start: Option<u64>,
    span: Option<Span>,
    /// Session time the current recording started at, from the caller's
    /// [`Clock`](dmm_lib::Clock). Session time rather than wall time so a
    /// mock run on a bent clock shows a duration its samples agree with.
    pub start_time: Option<Instant>,
    /// How many of the recording's samples are known to have reached a file,
    /// of either export format: always its first ones, as a recording's front
    /// is never trimmed.
    exported_count: usize,
    /// Which recording the samples belong to, bumped whenever one starts or
    /// is discarded. An export names the epoch it rendered, so a save dialog
    /// that outlives its recording cannot mark the next one saved.
    epoch: u64,
    /// Samples the history holds, and a recording stops at, from the Buffer
    /// size setting (which bounds the graph history by the same number).
    ///
    /// A sample carrying only the meter's main reading is roughly 340 bytes —
    /// about 240 inline, plus the `display_raw` heap string and the meter's
    /// own frame — so the default 500K is on the order of 170 MB. Each
    /// sub-value the meter sends adds another ~140 bytes, which puts a
    /// four-sub-value meter (UT181A) at ~900 bytes per sample, or ~450 MB at
    /// the same bound. A stopped recording kept beside the history can take
    /// as much again.
    max_samples: usize,
    /// The markers the last CSV or JSON export of this recording wrote, each
    /// as its [`marker_key`]: a marker, or a note, missing from it exists in
    /// no file. Bounded by the markers, one per sample at most.
    saved_markers: HashSet<u64>,
}

/// One marker — its reading, number and note — as a value an export can
/// keep: a note edited back to what the file has gives the same one.
fn marker_key(m: &Marker) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (m.at, m.number, &m.note).hash(&mut h);
    h.finish()
}

/// The sample of `samples` taken at `at`.
fn find_sample(samples: &VecDeque<Sample>, at: Instant) -> Option<&Sample> {
    position(samples, at).map(|i| &samples[i])
}

/// Where in `samples` the reading taken at `at` is. Samples are in time
/// order, so this is a binary search.
fn position(samples: &VecDeque<Sample>, at: Instant) -> Option<usize> {
    let i = samples.partition_point(|s| s.measurement.timestamp < at);
    samples
        .get(i)
        .is_some_and(|s| s.measurement.timestamp == at)
        .then_some(i)
}

/// Whether `range` of `samples` holds the reading taken at `at`.
fn slice_holds(samples: &VecDeque<Sample>, range: Range<usize>, at: Instant) -> bool {
    position(samples, at).is_some_and(|i| range.contains(&i))
}

impl Recording {
    pub fn new() -> Self {
        Self {
            active: false,
            store: VecDeque::new(),
            front_seq: 0,
            history_start: None,
            span: None,
            start_time: None,
            exported_count: 0,
            epoch: 0,
            max_samples: DEFAULT_MAX_SAMPLES,
            saved_markers: HashSet::new(),
        }
    }

    pub fn role(&self) -> BufferRole {
        if self.span.is_some() {
            BufferRole::Recording
        } else {
            BufferRole::History
        }
    }

    /// The sequence number the next pushed sample gets.
    fn next_seq(&self) -> u64 {
        self.front_seq + self.store.len() as u64
    }

    /// Where `seq` sits in the store, clamped to it.
    fn index(&self, seq: u64) -> usize {
        (seq.saturating_sub(self.front_seq) as usize).min(self.store.len())
    }

    /// The recording's samples: a buffer, and the range of it that is the
    /// recording. Empty with no recording.
    pub(crate) fn recording_slice(&self) -> (&VecDeque<Sample>, Range<usize>) {
        match &self.span {
            None => (&self.store, 0..0),
            Some(Span::Detached(samples)) => (samples, 0..samples.len()),
            Some(Span::InStore { start, end }) => (
                &self.store,
                self.index(*start)..end.map_or(self.store.len(), |end| self.index(end)),
            ),
        }
    }

    /// The history's samples, as [`Recording::recording_slice`].
    pub(crate) fn history_slice(&self) -> (&VecDeque<Sample>, Range<usize>) {
        let start = self
            .history_start
            .map_or(self.store.len(), |seq| self.index(seq));
        (&self.store, start..self.store.len())
    }

    /// What Export… saves: the recording, or with none, the history.
    pub(crate) fn export_slice(&self) -> (&VecDeque<Sample>, Range<usize>) {
        match self.role() {
            BufferRole::Recording => self.recording_slice(),
            BufferRole::History => self.history_slice(),
        }
    }

    /// The recording's samples, oldest first.
    pub(crate) fn recording_samples(&self) -> std::collections::vec_deque::Iter<'_, Sample> {
        let (samples, range) = self.recording_slice();
        samples.range(range)
    }

    /// The history's samples, oldest first.
    pub(crate) fn history_samples(&self) -> std::collections::vec_deque::Iter<'_, Sample> {
        let (samples, range) = self.history_slice();
        samples.range(range)
    }

    /// The samples Export… saves, oldest first.
    pub(crate) fn export_samples(&self) -> std::collections::vec_deque::Iter<'_, Sample> {
        let (samples, range) = self.export_slice();
        samples.range(range)
    }

    fn recording_len(&self) -> usize {
        self.recording_slice().1.len()
    }

    /// Change the sample bound. Returns `true` if an active recording was
    /// stopped because it already held at least `n` samples.
    ///
    /// A recording never throws away what it has captured, so lowering the
    /// bound past a running capture ends it rather than truncating it, and a
    /// stopped one is kept whole. The history drops its oldest samples at
    /// once and gives the memory back, as the graph does.
    pub fn set_max_samples(&mut self, n: usize) -> bool {
        self.max_samples = n;
        let stopped = self.active && self.recording_len() >= n;
        if stopped {
            self.stop();
        }
        let held = self.store.len();
        self.settle();
        // Only when samples went: after a raise, giving back the spare
        // capacity would copy the store now and again as it regrows.
        if self.store.len() < held {
            self.store.shrink_to_fit();
        }
        stopped
    }

    /// Start or stop recording, `now` being the session time it happens at.
    ///
    /// Starting drops the previous recording but leaves the history alone.
    /// A recording stopped before it captured anything leaves nothing to
    /// keep, so Export… goes back to the history.
    pub fn toggle(&mut self, now: Instant) {
        if self.active {
            self.stop();
            if self.recording_len() == 0 {
                self.span = None;
            }
        } else {
            self.active = true;
            self.start_new(Some(Span::InStore {
                start: self.next_seq(),
                end: None,
            }));
            self.start_time = Some(now);
        }
        self.settle();
    }

    /// End a running recording at the samples it has.
    fn stop(&mut self) {
        self.active = false;
        let next = self.next_seq();
        if let Some(Span::InStore {
            end: end @ None, ..
        }) = &mut self.span
        {
            *end = Some(next);
        }
    }

    /// Put `span` in place of the recording, with nothing of it exported.
    fn start_new(&mut self, span: Option<Span>) {
        self.span = span;
        self.exported_count = 0;
        self.epoch += 1;
        self.saved_markers.clear();
    }

    /// Drop the recording. The history is left as it is: Export… saves the
    /// readings the graph holds, the recording's among them.
    pub fn discard(&mut self) {
        self.active = false;
        self.start_new(None);
        self.settle();
    }

    /// Restart the history at the next reading, as the graph's Clear drops
    /// its points. A recording is left alone: Clear has never discarded a
    /// capture.
    pub fn clear_history(&mut self) {
        self.history_start = None;
        self.settle();
    }

    /// Start the history no earlier than `start`, the graph's oldest point,
    /// so the history holds what the graph does. Only ever forward: a new
    /// meter restarts the history while the graph keeps its older trace, and
    /// those readings are the other meter's. A recording spans the graph's
    /// resets and is left alone.
    ///
    /// `start` itself is kept: it is the reading the graph restarted on.
    pub fn trim_before(&mut self, start: Instant) {
        let Some(current) = self.history_start else {
            return;
        };
        let at = self
            .store
            .partition_point(|s| s.measurement.timestamp < start);
        self.history_start = Some(current.max(self.front_seq + at as u64));
        self.settle();
    }

    /// Whether the history holds nothing yet: it restarts with the next
    /// reading, which is when the caller latches what it will export under.
    pub(crate) fn history_is_empty(&self) -> bool {
        self.history_slice().1.is_empty()
    }

    /// Hold the history to its bound, drop what neither the history nor the
    /// recording holds, and move a stopped recording the history has left
    /// behind out of the store.
    fn settle(&mut self) {
        let next = self.next_seq();
        if let Some(start) = &mut self.history_start {
            *start = (*start).max(next.saturating_sub(self.max_samples as u64));
        }
        let history_from = self.history_start.unwrap_or(next);
        if let Some(Span::InStore {
            start,
            end: Some(end),
        }) = self.span
            && end <= history_from
        {
            let from = self.index(start);
            let to = self.index(end);
            let recorded: VecDeque<Sample> = self.store.drain(..to).skip(from).collect();
            self.front_seq += to as u64;
            self.span = Some(Span::Detached(recorded));
            // The store grew to hold both; it holds the history alone now.
            self.store.shrink_to_fit();
        }
        let keep_from = match self.span {
            Some(Span::InStore { start, .. }) => start.min(history_from),
            _ => history_from,
        };
        let drop = self.index(keep_from);
        self.store.drain(..drop);
        self.front_seq += drop as u64;
    }

    /// Whether the history or the recording holds the reading taken at `at`.
    pub(crate) fn holds(&self, at: Instant) -> bool {
        self.sample_at(at).is_some()
    }

    /// The reading taken at `at`, if the history or the recording holds it.
    pub(crate) fn sample_at(&self, at: Instant) -> Option<&Sample> {
        find_sample(&self.store, at).or_else(|| match &self.span {
            Some(Span::Detached(samples)) => find_sample(samples, at),
            _ => None,
        })
    }

    /// Whether the recording holds the reading taken at `at`.
    pub(crate) fn in_recording(&self, at: Instant) -> bool {
        let (samples, range) = self.recording_slice();
        slice_holds(samples, range, at)
    }

    /// The markers of `markers` that sit on samples Export… saves, oldest
    /// first: what an export writes.
    pub(crate) fn marked<'a>(&self, markers: impl Iterator<Item = &'a Marker>) -> Vec<&'a Marker> {
        let (samples, range) = self.export_slice();
        markers
            .filter(|m| slice_holds(samples, range.clone(), m.at))
            .collect()
    }

    /// `marked` — the markers on buffered samples, from
    /// [`Recording::marked`] — as an export records them: see
    /// [`Recording::mark_exported`].
    pub(crate) fn marker_keys(marked: &[&Marker]) -> HashSet<u64> {
        marked.iter().map(|m| marker_key(m)).collect()
    }

    /// The recording's markers that its last CSV or JSON export doesn't hold
    /// — a marker deleted since loses nothing.
    fn unsaved_markers<'a>(&'a self, markers: &'a Markers) -> impl Iterator<Item = &'a Marker> {
        let (samples, range) = self.recording_slice();
        markers.iter().filter(move |m| {
            slice_holds(samples, range.clone(), m.at)
                && !self.saved_markers.contains(&marker_key(m))
        })
    }

    /// How many of the recording's markers, or their notes, its last CSV or
    /// JSON export doesn't hold. None with no recording.
    ///
    /// With [`Recording::unexported_count`], what starting a new recording
    /// would lose: Export… saves the recording while there is one, so all of
    /// it counts, even what the graph still holds.
    pub(crate) fn unsaved_marker_count(&self, markers: &Markers) -> usize {
        self.unsaved_markers(markers).count()
    }

    /// What Discard would lose that no file holds: the recording's unexported
    /// samples the history no longer holds, and the unsaved markers on them.
    /// The rest stays for Export… to save as the graph's readings.
    pub(crate) fn lost_on_discard(&self, markers: &Markers) -> (usize, usize) {
        let history_from = self.history_start.unwrap_or(self.next_seq());
        let left_behind = match &self.span {
            None => 0..0,
            Some(Span::Detached(samples)) => self.exported_count..samples.len(),
            Some(Span::InStore { start, end }) => {
                let end = end.unwrap_or(self.next_seq()).min(history_from);
                let first = (start + self.exported_count as u64).min(end);
                self.index(first)..self.index(end)
            }
        };
        let samples = left_behind.len();
        let markers = match &self.span {
            Some(Span::Detached(_)) => self.unsaved_markers(markers).count(),
            _ => {
                let history = self.history_slice().1.start;
                self.unsaved_markers(markers)
                    .filter(|m| position(&self.store, m.at).is_some_and(|i| i < history))
                    .count()
            }
        };
        (samples, markers)
    }

    /// The recording's current run — see `epoch`.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Most sub-values any sample Export… saves carries. The export sizes its
    /// aux columns from the device profile, but a profile is only known while
    /// connected — this is the floor that keeps a capture exportable in full
    /// after the meter is unplugged.
    pub fn max_aux_seen(&self) -> usize {
        self.export_samples()
            .map(|s| s.measurement.aux_values.len())
            .max()
            .unwrap_or(0)
    }

    /// Samples captured since the last successful export.
    ///
    /// Non-zero means starting a new recording would destroy data that
    /// exists nowhere else, which is what the Record confirmation prompt
    /// checks. Always zero with no recording: the history goes with the
    /// graph's resets unasked.
    pub fn unexported_count(&self) -> usize {
        self.recording_len().saturating_sub(self.exported_count)
    }

    /// Record that the first `count` samples of the recording of `epoch`
    /// reached a file.
    ///
    /// Takes the count that was actually written rather than the current
    /// length: samples arriving while the export ran are not in that file and
    /// must still count as unexported. An export of an earlier epoch marks
    /// nothing — the samples it wrote are gone, and the ones in their place
    /// are in no file.
    ///
    /// `markers` is [`Recording::marker_keys`] of what the export wrote, for
    /// a format that writes markers; `None` for one that doesn't, which saves
    /// none of them.
    pub fn mark_exported(&mut self, epoch: u64, count: usize, markers: Option<HashSet<u64>>) {
        if epoch == self.epoch {
            self.exported_count = count.min(self.recording_len());
            if let Some(keys) = markers {
                self.saved_markers = keys;
            }
        }
    }

    /// Push a sample. Returns `true` if the recording just became full, which
    /// stops it.
    ///
    /// Every reading joins the history; a running recording takes it too,
    /// up to the bound.
    ///
    /// `extra_aux` is the caller's current [`Sample::extra_aux`]: how many of
    /// this reading's trailing sub-values software appended.
    pub fn push(&mut self, m: &Measurement, wall_clock: &WallClock, extra_aux: usize) -> bool {
        let seq = self.next_seq();
        self.history_start.get_or_insert(seq);
        self.store
            .push_back(Sample::from_measurement(m, wall_clock, extra_aux));
        let full = self.active && self.recording_len() >= self.max_samples;
        if full {
            self.stop();
        }
        self.settle();
        full
    }

    /// Whether the recording holds as many samples as it can.
    pub fn is_full(&self) -> bool {
        self.recording_len() >= self.max_samples
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
    use std::time::Duration;

    /// A UT61E+ DC V reading on range 1 showing `display`, AUTO on.
    fn make_measurement(display: &[u8; 7]) -> Measurement {
        dmm_lib::protocol::make_test_measurement(0x02, 0x01, display, (0, 0), (0, 0, 0))
    }

    /// The GUI's export never integrates — that is a CLI-only run mode.
    fn layout(family_slots: usize, extra_slots: usize) -> CsvLayout {
        CsvLayout {
            family_slots,
            extra_slots,
            integral: false,
            markers: false,
        }
    }

    #[test]
    fn recording_inactive_by_default() {
        let r = Recording::new();
        assert!(!r.active);
        assert_eq!(r.export_samples().len(), 0);
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

    /// The history takes every sample, and keeps them through Record: the
    /// recording is the samples from there on.
    #[test]
    fn record_keeps_the_history() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        assert_eq!(r.role(), BufferRole::History);
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.history_samples().len(), 2);
        assert_eq!(r.unexported_count(), 0, "the history never prompts");

        r.toggle(Instant::now()); // start
        assert_eq!(r.role(), BufferRole::Recording);
        assert_eq!(r.recording_samples().len(), 0);
        assert_eq!(r.history_samples().len(), 2, "the graph's, still");
        r.push(&m, &wc, 0);
        assert_eq!(r.recording_samples().len(), 1);
        assert_eq!(r.history_samples().len(), 3);
    }

    /// A stopped recording is kept as it is: later readings go to the
    /// history only.
    #[test]
    fn a_stopped_recording_takes_no_samples() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now()); // start
        r.push(&m, &wc, 0);
        r.toggle(Instant::now()); // stop
        assert_eq!(r.role(), BufferRole::Recording);
        assert!(!r.push(&m, &wc, 0));
        assert_eq!(r.recording_samples().len(), 1);
        assert_eq!(r.history_samples().len(), 2, "the history takes it");
    }

    /// A recording stopped before it captured anything has nothing to keep,
    /// so Export… saves the history again.
    #[test]
    fn an_empty_recording_hands_the_buffer_back_to_the_history() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now()); // start
        r.toggle(Instant::now()); // stop, nothing captured
        assert_eq!(r.role(), BufferRole::History);
        r.push(&m, &wc, 0);
        assert_eq!(r.export_samples().len(), 1);
    }

    /// The history drops its oldest sample at the bound, as the graph does;
    /// a recording would stop there instead.
    #[test]
    fn the_history_drops_its_oldest_sample_at_the_bound() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.set_max_samples(3);
        let base = Instant::now();
        for i in 0..5 {
            let mut m = make_measurement(b"  1.234");
            m.timestamp = base + Duration::from_millis(i);
            assert!(!r.push(&m, &wc, 0), "the history never fills up");
        }
        let kept: Vec<Instant> = r
            .history_samples()
            .map(|s| s.measurement.timestamp)
            .collect();
        assert_eq!(
            kept,
            [2, 3, 4].map(|i| base + Duration::from_millis(i)),
            "the newest three"
        );
    }

    /// Lowering the bound under the history gives the samples up at once.
    #[test]
    fn lowering_the_bound_trims_the_history_now() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        for _ in 0..10 {
            r.push(&m, &wc, 0);
        }
        assert!(!r.set_max_samples(4), "no recording to stop");
        assert_eq!(r.history_samples().len(), 4);
        assert_eq!(r.store.len(), 4, "given up, not only hidden");
        assert!(!r.set_max_samples(100));
        assert_eq!(
            r.history_samples().len(),
            4,
            "raising it brings nothing back"
        );
    }

    /// The history holds what the graph does: a graph restarted on a reading
    /// takes the samples before that reading with it, and keeps that one.
    #[test]
    fn trimming_keeps_the_reading_the_graph_restarted_on() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let base = Instant::now();
        let mut wide = make_measurement(b"  1.234");
        wide.aux_values = vec![aux("Frequency", "50.01", "Hz")];
        for i in 0..4 {
            let mut m = if i == 0 {
                wide.clone()
            } else {
                make_measurement(b"  1.234")
            };
            m.timestamp = base + Duration::from_millis(i);
            r.push(&m, &wc, 0);
        }
        assert_eq!(r.max_aux_seen(), 1);
        r.trim_before(base + Duration::from_millis(2));
        assert_eq!(r.history_samples().len(), 2);
        assert_eq!(
            r.history_samples().next().map(|s| s.measurement.timestamp),
            Some(base + Duration::from_millis(2))
        );
        assert_eq!(r.max_aux_seen(), 0, "the widest sample went with the trim");
        r.trim_before(base + Duration::from_millis(10));
        assert_eq!(r.history_samples().len(), 0);
        assert!(r.store.is_empty(), "nothing else holds them");
    }

    /// Discarding a recording leaves its readings to the history, as long as
    /// the graph holds them: Export… saves them from there.
    #[test]
    fn discarding_a_recording_keeps_its_readings_in_the_history() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        r.push(&m, &wc, 0);
        r.toggle(Instant::now()); // stop, one unexported sample kept
        let epoch = r.epoch();

        r.discard();
        assert_eq!(r.role(), BufferRole::History);
        assert!(!r.active);
        assert_eq!(r.export_samples().len(), 1, "the graph's, still");
        assert_eq!(r.unexported_count(), 0);
        assert_ne!(r.epoch(), epoch, "an export of it marks nothing now");
        r.push(&m, &wc, 0);
        assert_eq!(r.export_samples().len(), 2);
    }

    /// A recording outlives the graph's resets and its Clear.
    #[test]
    fn trimming_and_clearing_leave_a_recording_alone() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        r.push(&m, &wc, 0);
        r.trim_before(m.timestamp + Duration::from_secs(1));
        r.clear_history();
        assert_eq!(r.recording_samples().len(), 1);
        let epoch = r.epoch();

        r.toggle(Instant::now()); // stop, one sample kept
        r.clear_history();
        assert_eq!(r.recording_samples().len(), 1);
        assert_eq!(r.history_samples().len(), 0);
        assert_eq!(r.epoch(), epoch);
    }

    /// Clear drops the history, and with it the export's view of it.
    #[test]
    fn clearing_the_history_empties_it() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let mut wide = make_measurement(b"  1.234");
        wide.aux_values = vec![aux("Frequency", "50.01", "Hz")];
        r.push(&wide, &wc, 0);
        r.clear_history();
        assert_eq!(r.history_samples().len(), 0);
        assert!(r.store.is_empty());
        assert_eq!(r.max_aux_seen(), 0);
    }

    /// `n` readings a millisecond apart from `base`, pushed.
    fn push_at(r: &mut Recording, base: Instant, from: u64, n: u64) {
        let wc = WallClock::new();
        for i in from..from + n {
            let mut m = make_measurement(b"  1.234");
            m.timestamp = base + Duration::from_millis(i);
            r.push(&m, &wc, 0);
        }
    }

    /// A graph restart mid-recording restarts the history; the recording
    /// keeps its whole span, and the store holds both.
    #[test]
    fn a_restart_mid_recording_keeps_the_recording_whole() {
        let mut r = Recording::new();
        let base = Instant::now();
        push_at(&mut r, base, 0, 3);
        r.toggle(Instant::now());
        push_at(&mut r, base, 3, 4);
        r.trim_before(base + Duration::from_millis(5));
        assert_eq!(r.recording_samples().len(), 4);
        assert_eq!(r.history_samples().len(), 2);
        assert_eq!(r.store.len(), 4, "the pre-Record readings went");
    }

    /// Once the history leaves a stopped recording behind, the recording
    /// moves out of the store, and the store holds the history only.
    #[test]
    fn a_stopped_recording_left_behind_moves_out() {
        let mut r = Recording::new();
        let base = Instant::now();
        r.toggle(Instant::now());
        push_at(&mut r, base, 0, 3);
        r.toggle(Instant::now()); // stop
        push_at(&mut r, base, 3, 3);
        r.trim_before(base + Duration::from_millis(4));
        assert!(matches!(r.span, Some(Span::Detached(_))));
        assert_eq!(r.recording_samples().len(), 3);
        assert_eq!(r.history_samples().len(), 2);
        assert_eq!(r.store.len(), 2, "the reading between went");
        assert!(r.holds(base + Duration::from_millis(1)));
        assert!(!r.holds(base + Duration::from_millis(3)));
        // Discarding it leaves the history as it was.
        r.discard();
        assert_eq!(r.export_samples().len(), 2);
    }

    /// Lowering the bound caps the history but keeps a stopped recording
    /// whole.
    #[test]
    fn lowering_the_bound_keeps_a_stopped_recording_whole() {
        let mut r = Recording::new();
        let base = Instant::now();
        r.toggle(Instant::now());
        push_at(&mut r, base, 0, 100);
        r.toggle(Instant::now()); // stop
        push_at(&mut r, base, 100, 20);
        assert!(!r.set_max_samples(10), "nothing running to stop");
        assert_eq!(r.recording_samples().len(), 100);
        assert_eq!(r.history_samples().len(), 10);
        assert_eq!(r.store.len(), 10);
    }

    /// Discard loses only what the history no longer holds: samples the
    /// graph still has stay for Export… to save as its readings.
    #[test]
    fn discard_loses_only_what_left_the_graph() {
        let mut r = Recording::new();
        let base = Instant::now();
        r.toggle(Instant::now());
        push_at(&mut r, base, 0, 5);
        r.toggle(Instant::now()); // stop
        let markers = Markers::default();
        assert_eq!(r.lost_on_discard(&markers), (0, 0), "the graph holds all");
        assert_eq!(r.unexported_count(), 5, "starting over would still ask");
        r.trim_before(base + Duration::from_millis(2));
        assert_eq!(r.lost_on_discard(&markers), (2, 0));
        r.mark_exported(r.epoch(), 1, None);
        assert_eq!(
            r.lost_on_discard(&markers),
            (1, 0),
            "the first is in a file"
        );
        r.trim_before(base + Duration::from_millis(9));
        assert_eq!(r.lost_on_discard(&markers), (4, 0), "moved out, all left");
    }

    /// The history only ever starts later: a graph whose oldest point is
    /// older than the history — a new meter restarted it, the graph kept
    /// its trace — takes none of the old readings back.
    #[test]
    fn the_history_only_moves_forward() {
        let mut r = Recording::new();
        let base = Instant::now();
        push_at(&mut r, base, 0, 3);
        r.clear_history();
        push_at(&mut r, base, 3, 2);
        r.trim_before(base);
        assert_eq!(r.history_samples().len(), 2);
    }

    /// An export of a discarded recording, finishing after the next one
    /// started, marks nothing of the new one.
    #[test]
    fn an_export_of_a_discarded_recording_marks_nothing() {
        let mut r = Recording::new();
        let base = Instant::now();
        r.toggle(Instant::now());
        push_at(&mut r, base, 0, 3);
        let exporting = r.epoch();
        r.toggle(Instant::now()); // stop
        r.discard();
        r.toggle(Instant::now());
        push_at(&mut r, base, 3, 2);
        r.mark_exported(exporting, 3, None);
        assert_eq!(r.unexported_count(), 2);
    }

    #[test]
    fn recording_toggle_clears_previous() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        r.toggle(Instant::now());
        let m = make_measurement(b"  1.234");
        r.push(&m, &wc, 0);
        r.push(&m, &wc, 0);
        assert_eq!(r.recording_samples().len(), 2);

        r.toggle(Instant::now()); // stop
        r.toggle(Instant::now()); // start again — should clear
        assert_eq!(r.recording_samples().len(), 0);
        assert_eq!(r.history_samples().len(), 2, "the graph's, still");
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

        r.mark_exported(r.epoch(), 3, None);
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
        r.mark_exported(r.epoch(), 5, None);
        assert_eq!(r.unexported_count(), 2);
    }

    #[test]
    fn starting_a_new_recording_resets_the_export_mark() {
        let mut r = Recording::new();
        let wc = WallClock::new();
        let m = make_measurement(b"  1.234");
        r.toggle(Instant::now());
        r.push(&m, &wc, 0);
        r.mark_exported(r.epoch(), 1, None);
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
        r.mark_exported(exporting, 3, None);
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
        r.mark_exported(r.epoch(), 99, None);
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
        assert_eq!(r.recording_samples().len(), 100);
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
        assert_eq!(r.recording_samples().len(), 100);
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
        assert_eq!(
            r.recording_samples().len(),
            50,
            "captured samples are never discarded"
        );
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
        assert_eq!(r.recording_samples().len(), 51);
    }

    #[test]
    fn sample_from_measurement() {
        let m = make_measurement(b"  5.678");
        let wc = WallClock::new();
        let s = Sample::from_measurement(&m, &wc, 0);
        assert_eq!(s.measurement.mode, "DC V");
        assert_eq!(s.measurement.value_display_str(), "5.678");
        assert_eq!(s.measurement.unit, "V");
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

        let bytes = render_csv(samples.iter(), &[], "UNI-T UT61E+", layout(0, 0)).unwrap();
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
        let bytes = render_csv(VecDeque::new().iter(), &[], "mock", layout(0, 0)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text.lines().count(), 2);
    }

    /// A single-display export is the one file both binaries write, so its
    /// header line is pinned to the same string the CLI asserts in
    /// `csv_header_names_one_group_per_aux_slot` — renaming a column in one
    /// exporter alone breaks a test.
    #[test]
    fn gui_and_cli_single_display_headers_agree() {
        let bytes = render_csv(VecDeque::new().iter(), &[], "mock", layout(0, 0)).unwrap();
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

        let bytes = render_csv(
            VecDeque::from([s]).iter(),
            &[],
            "UNI-T UT181A",
            layout(4, 0),
        )
        .unwrap();
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

        let bytes = render_csv(
            VecDeque::from([s]).iter(),
            &[],
            "UNI-T UT181A",
            layout(1, 0),
        )
        .unwrap();
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

        let bytes = render_csv(VecDeque::from([s]).iter(), &[], "mock", layout(1, 0)).unwrap();
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

        let bytes = render_csv(samples.iter(), &[], "UNI-T UT181A", layout(2, 1)).unwrap();
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
        let text = render_replay(
            samples.iter(),
            "ut61eplus",
            Some("UNI-T UT61E+"),
            Some(dmm_lib::transport::Link::Bluetooth),
        )
        .expect("frames with wire bytes");

        let replay = dmm_lib::replay::Replay::parse(&text).expect("a well-formed recording");
        assert_eq!(replay.device.id, "ut61eplus");
        assert_eq!(replay.model.as_deref(), Some("UNI-T UT61E+"));
        // The link the session was on, so playing the file back says so too.
        assert_eq!(replay.link, Some(dmm_lib::transport::Link::Bluetooth));
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
        let text = render_replay(replay_samples().iter(), "ut61eplus", None, None).expect("frames");
        assert!(!text.contains("# model:"), "{text}");
        assert!(!text.contains("# link:"), "{text}");
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
        assert!(render_replay(samples.iter(), "ut61eplus", None, None).is_none());
        assert!(render_replay(VecDeque::new().iter(), "ut61eplus", None, None).is_none());
    }

    /// The layout of a file carrying markers.
    fn marked_layout() -> CsvLayout {
        CsvLayout {
            markers: true,
            ..layout(0, 0)
        }
    }

    /// Markers on samples 2 and 3 of three, numbered 4 and 7.
    fn marked_samples() -> (VecDeque<Sample>, Vec<Marker>) {
        let samples = replay_samples();
        let marker = |i: usize, number, note: &str| Marker {
            at: samples[i].measurement.timestamp,
            number,
            note: note.to_string(),
            wall_time: samples[i].wall_time,
            reading: String::new(),
        };
        let markers = vec![marker(1, 4, "load on, 2.2 \u{3a9}"), marker(2, 7, "")];
        (samples, markers)
    }

    /// The marker columns close the header; a marked row carries its number
    /// and note (quoted, for the comma), an unmarked one leaves them empty.
    #[test]
    fn render_csv_writes_each_marker_on_its_sample() {
        let (samples, markers) = marked_samples();
        let marked: Vec<&Marker> = markers.iter().collect();
        let bytes = render_csv(samples.iter(), &marked, "UNI-T UT61E+", marked_layout()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[1],
            "timestamp,mode,value,unit,range,flags,marker,note"
        );
        assert!(lines[2].ends_with(",,"), "{:?}", lines[2]);
        assert!(
            lines[3].ends_with(",4,\"load on, 2.2 \u{3a9}\""),
            "{:?}",
            lines[3]
        );
        assert!(lines[4].ends_with(",7,"), "{:?}", lines[4]);
        let mut reader = csv::ReaderBuilder::new()
            .comment(Some(b'#'))
            .from_reader(text.as_bytes());
        let width = reader.headers().unwrap().len();
        for record in reader.records() {
            assert_eq!(
                record.unwrap().len(),
                width,
                "every row as wide as the header"
            );
        }
    }

    /// A marker on no sample of the file is skipped, not written onto the
    /// next sample.
    #[test]
    fn a_marker_between_samples_lands_on_none() {
        let (samples, mut markers) = marked_samples();
        markers[0].at -= std::time::Duration::from_millis(1);
        let marked: Vec<&Marker> = markers.iter().collect();
        let bytes = render_csv(samples.iter(), &marked, "UNI-T UT61E+", marked_layout()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[3].ends_with(",,"), "{:?}", lines[3]);
        assert!(lines[4].ends_with(",7,"), "{:?}", lines[4]);
    }

    #[test]
    fn render_json_writes_each_marker_on_its_sample() {
        let (samples, markers) = marked_samples();
        let marked: Vec<&Marker> = markers.iter().collect();
        let text = render_json(samples.iter(), &marked, "UNI-T UT61E+", false);
        let lines: Vec<serde_json::Value> = text
            .lines()
            .skip(1)
            .map(|l| serde_json::from_str(l).expect("a JSON object"))
            .collect();
        assert!(lines[0].get("marker").is_none());
        assert_eq!(lines[1]["marker"], 4);
        assert_eq!(lines[1]["note"], "load on, 2.2 \u{3a9}");
        assert_eq!(lines[2]["marker"], 7);
        assert_eq!(lines[2]["note"], "");
    }

    /// The buffer finds its samples by time, and `marked` keeps only the
    /// markers on them.
    #[test]
    fn marked_keeps_the_markers_on_buffered_samples() {
        let (samples, mut markers) = marked_samples();
        let mut r = Recording::new();
        r.store = samples;
        r.history_start = Some(0);
        markers[1].at += std::time::Duration::from_secs(9);
        let marked: Vec<u32> = r.marked(markers.iter()).iter().map(|m| m.number).collect();
        assert_eq!(marked, [4]);
    }

    /// The export and `dmm-cli read --format json` cannot drift, because both
    /// are these two calls into `dmm_shared::export` — so this checks the
    /// document the GUI builds around them, not the objects themselves.
    #[test]
    fn render_json_is_the_metadata_line_and_one_object_per_sample() {
        let mut samples = replay_samples();
        samples.truncate(2);
        let text = render_json(samples.iter(), &[], "UNI-T UT61E+", true);

        let mut expected = dmm_shared::export::metadata_line("UNI-T UT61E+");
        for s in &samples {
            expected.push('\n');
            expected.push_str(
                &dmm_shared::export::measurement_json(
                    &s.measurement,
                    &s.wall_time.to_rfc3339(),
                    true,
                    None,
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
