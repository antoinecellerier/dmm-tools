//! Real-time scrolling graph: the history buffer the App pushes samples into,
//! and the widget that draws it.
//!
//! The concerns live in submodules — [`view`] (what slice is shown and the
//! gestures that move it), [`toolbar`], [`render`] (the main plot),
//! [`minimap`], [`level`] (the minimap's bucketed trace), [`analysis`]
//! (visible-slice statistics), [`field`] (the toolbar's text-edit buffers and
//! the values they parse to) and [`time`] (axis label formatting) — all of
//! which add methods to the one [`Graph`] declared here, so the type's public
//! API is unchanged by the split.

mod analysis;
mod field;
mod level;
mod minimap;
mod render;
mod time;
mod toolbar;
mod view;

#[cfg(test)]
mod tests;

use eframe::egui::{self, Ui};
use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::settings::DEFAULT_MAX_SAMPLES;
use crate::theme::ThemeColors;
use field::{NumberField, NumberListField};
use level::MinimapLevel;
use minimap::{MINIMAP_HEIGHT, MinimapDrag};

/// Maximum number of same-unit sub-values drawn beside the plotted series.
///
/// The protocols send at most four sub-values per frame (UT181A), so this is
/// the ceiling the wire imposes rather than a display choice.
pub(crate) const MAX_OVERLAYS: usize = 4;

/// What the meter's main reading is called when the meter gives it no name
/// of its own: on the **Plot:** chip, in the key, and as the trace drawn
/// beside a plotted sub-value.
pub(crate) const MAIN_SERIES: &str = "Main";

/// Consecutive frames without a sub-value before it is no longer offered —
/// and, if it was selected, before the selection is dropped. The gap
/// threshold has to pass too: see [`Graph::set_series_options`].
///
/// A single short or bit-clear frame from the UT181A isn't a mode change: it
/// gates its sub-values on both a status bit and the frame being long enough,
/// so one truncated reply omits them without the meter having moved. Three
/// consecutive frames without the label is.
const SERIES_DROP_FRAMES: u32 = 3;

/// Default gap threshold multiplier: gap = max(interval * multiplier, minimum).
const GAP_MULTIPLIER: f64 = 5.0;
const GAP_MINIMUM_SECS: f64 = 1.0;

/// Why the trace is interrupted over a stretch of time.
///
/// The distinction is user-visible: an overload is the meter reporting a
/// condition, while a data gap is the absence of any report at all. They are
/// drawn differently — see `show_main`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GapKind {
    /// No samples arrived: disconnect, pause, or a sample interval longer
    /// than the gap threshold.
    NoData,
    /// The meter reported over-range. Carries no plottable value, but is a
    /// measurement, not a dropout.
    Overload,
}

/// Segments (contiguous runs of [time, value] points) paired with the gaps
/// between them: (start_time, end_time, why).
type SegmentsAndGaps = (Vec<Vec<[f64; 2]>>, Vec<(f64, f64, GapKind)>);

/// A data point with an absolute timestamp.
#[derive(Clone, Copy)]
struct DataPoint {
    time: Instant,
    value: f64,
    /// The series was interrupted immediately before this point, and why —
    /// an overload produces no plottable value, so the line must break here
    /// even though the two neighbouring samples are adjacent in time.
    /// `None` when this point continues the previous one.
    break_before: Option<GapKind>,
    /// Timestamp of the last non-plottable sample in that interruption, or
    /// with `break_band_late` the first OL sample.
    ///
    /// Bounds what we actually observed. If the meter goes over range and
    /// then the link drops, OL samples stop arriving and this stays at the
    /// last one we heard — everything after it is silence, not over-range.
    break_last_sample: Option<Instant>,
    /// The interruption included a genuine loss of data, as reported by the
    /// App — not merely a quiet meter. See `push_data_loss`.
    break_had_data_loss: bool,
    /// The meter showed a word instead of a reading before going over range:
    /// the band starts at `break_last_sample`, a gap before it. See
    /// `push_break`.
    break_band_late: bool,
}

/// One point of a sub-value trace, at the time of the frame that carried it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OverlayPoint {
    time: Instant,
    /// `None` breaks the trace here: the sub-value was over range, or the
    /// link was lost (`push_data_loss`).
    value: Option<f64>,
}

/// One sub-value trace drawn beside the plotted series.
///
/// Its points keep their own times rather than riding on the plotted
/// series' points: a meter that sends the parts of one reading in frames of
/// their own (the UT61E+'s AC+DC V) puts each at its own moment, and a frame
/// without the plotted series still carries them. A point is appended only
/// for frames that carry the label, so the single-display meters that make up
/// most of the device table pay nothing.
struct OverlaySeries {
    label: String,
    points: VecDeque<OverlayPoint>,
    /// Frames since the last one that carried this sub-value; see
    /// [`Graph::stopped_for`].
    missing_frames: u32,
}

impl OverlaySeries {
    /// Append a point, dropping the oldest once `max_points` are held — the
    /// bound that keeps a stream carrying only this sub-value, which never
    /// grows the history, from growing without limit.
    fn push(&mut self, point: OverlayPoint, max_points: usize) {
        while self.points.len() >= max_points.max(1) {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }
}

/// A sub-value the meter has offered for plotting, kept while it is sent.
struct SeriesOption {
    label: String,
    unit: String,
    /// Consecutive offers that did not include it.
    missing_frames: u32,
    /// Timestamp of the last frame that did.
    last_seen: Instant,
}

/// One measurement as the graph should plot it.
///
/// Built by the App, which decides *which* series is plotted and which
/// sub-values share its unit; the graph never sees `dmm_lib` measurement
/// types.
pub struct PlotSample<'a> {
    /// The plotted series' value, or `None` for a frame that carries only
    /// sub-values: its overlays are recorded and nothing else is touched.
    pub value: Option<f64>,
    pub timestamp: Instant,
    pub mode: &'a str,
    pub unit: &'a str,
    pub display_raw: Option<&'a str>,
    /// Label of the sub-value being plotted, or `None` for the meter's main
    /// reading. A change here swaps the plotted trace with the one of that
    /// name beside it, or restarts the graph when there is none: two
    /// sub-values can share a mode and a unit (T1 and T2), so nothing else
    /// would tell them apart.
    pub series: Option<&'a str>,
    /// The meter's name for its main reading ("DC" beside an "AC" part), or
    /// `None` for [`MAIN_SERIES`].
    pub main_label: Option<&'static str>,
    /// Sub-values sharing the plotted series' unit, as (label, value).
    /// `None` for an over-range sub-value — it breaks that trace without
    /// breaking the others.
    pub overlays: &'a [(&'a str, Option<f64>)],
}

/// Time window presets.
pub const TIME_WINDOWS: &[(f64, &str)] = &[
    (5.0, "5s"),
    (10.0, "10s"),
    (30.0, "30s"),
    (60.0, "1m"),
    (300.0, "5m"),
    (600.0, "10m"),
];

/// Real-time scrolling graph with minimap navigation.
pub struct Graph {
    history: VecDeque<DataPoint>,
    /// Points the history keeps before the oldest are dropped. Shared with
    /// the recording buffer through the Buffer size setting, and changed
    /// under a running session by [`Graph::set_max_points`].
    max_points: usize,
    /// Same-unit sub-value traces, each point at its own frame's time.
    overlays: Vec<OverlaySeries>,
    current_mode: Option<String>,
    current_unit: String,
    /// The meter's name for its main reading, from the latest sample; `None`
    /// for [`MAIN_SERIES`].
    main_label: Option<&'static str>,
    /// Label of the series `history` was recorded from; `None` for the main
    /// reading. Distinct from `selected_series`: this one only moves when a
    /// sample actually arrives for the new choice.
    current_series: Option<String>,
    /// Which series the toolbar is asking for. Session-only — a selection is
    /// about the meter's current mode, so persisting it across restarts would
    /// silently plot a sub-value the next session may not even have. Dropped
    /// with its option, see [`Graph::set_series_options`].
    selected_series: Option<String>,
    /// Sub-value labels the user has switched off in the toolbar's **Show:**
    /// group. Session-only, and deliberately keyed by label rather than by
    /// index so a choice survives `clear()`, a change of plotted series and a
    /// sub-value that disappears and comes back. Hidden overlays are still
    /// recorded in lockstep — re-showing one brings its history with it.
    hidden_overlays: HashSet<String>,
    /// Sub-values the meter is sending, with their resolved units, in the
    /// order first offered. Drives the toolbar's selector.
    series_options: Vec<SeriesOption>,
    /// Last `display_raw` string from the latest pushed measurement, kept
    /// for the a11y plot summary so screen readers hear the same digits the
    /// sighted user sees (e.g. "1.234 mV" instead of the raw f64 value
    /// printed at fixed precision, which mis-scales auto-range readings).
    last_display_raw: Option<String>,
    origin: Option<Instant>,
    /// Time window width in seconds for the main view.
    pub time_window_secs: f64,
    /// When true, main graph auto-scrolls to latest data.
    pub live: bool,
    /// User-controlled view center (seconds from origin). Used when not live.
    view_center: f64,
    /// Gap detection threshold in seconds.
    gap_threshold_secs: f64,
    /// Stretches longer than [`GAP_MINIMUM_SECS`] in which no frame of any
    /// kind arrived, oldest first, as (last frame before, first frame after).
    ///
    /// A trace breaks for want of data only across one of these longer than
    /// the gap threshold — not merely because two of its own points are far
    /// apart. A meter that sends a reading's parts in turn (the UT61E+'s
    /// AC+DC V, answering every 0.67 s) spaces each part's points wider than
    /// the threshold while it never goes quiet at all. Kept down to the
    /// minimum threshold so a threshold change is only a new question asked of
    /// them; there are only as many as the meter had real silences.
    silences: VecDeque<(Instant, Instant)>,
    /// Timestamp of the newest frame of any kind: a point, a frame of
    /// sub-values only, an overload or a word shown instead of a reading.
    last_heard: Option<Instant>,
    /// Frames of sub-values only since the last point of the plotted series;
    /// see [`Graph::stopped_for`].
    main_missing_frames: u32,
    /// A non-plottable sample arrived since the last plotted point, so the
    /// next one starts a new segment. Time-based gap detection can't see
    /// this: an over-range excursion shorter than the threshold leaves no
    /// time hole.
    pending_break: Option<GapKind>,
    /// A genuine dropout happened during the open interruption: the link
    /// went down, or acquisition was stopped. Distinct from the meter simply
    /// not sending — see `push_data_loss`.
    pending_data_loss: bool,
    /// Timestamp of the newest overload sample while a break is open: where
    /// the over-range band ends.
    pending_break_since: Option<Instant>,
    /// Timestamp of the newest sample while a break is open, an overload or
    /// a word shown instead of a reading.
    ///
    /// These carry a timestamp like any other; they just have no plottable
    /// value. Keeping the newest one lets the live view follow the present
    /// while the meter sends them, instead of freezing at the last plotted
    /// point.
    pending_heard_until: Option<Instant>,
    /// Timestamp of the first overload sample of a break that began with a
    /// word shown instead of a reading: where the band starts.
    pending_band_from: Option<Instant>,
    /// When true, Y axis uses fixed min/max instead of auto-scaling.
    pub y_axis_fixed: bool,
    /// Fixed Y-axis bounds, as typed and as parsed.
    y_min: NumberField,
    y_max: NumberField,
    /// Whether the user has manually set Y-axis values this session.
    y_user_set: bool,
    /// Show mean line overlay.
    pub show_mean: bool,
    /// Show min/max envelope band.
    pub show_envelope: bool,
    /// Envelope bucket width in seconds (user-configurable).
    envelope_window: NumberField,
    /// Reference lines: show horizontal lines at these values.
    pub show_ref_line: bool,
    /// Show trigger crossing markers on reference lines.
    pub show_crossings: bool,
    /// Values the reference lines are drawn at, as typed and as parsed.
    ref_lines: NumberListField,
    /// Put the caret in the reference-values field the next time it is drawn.
    /// Set when the lines are switched on, by the chip or by its key: the
    /// field appears empty, and nothing is drawn until a value is typed.
    focus_ref_field: bool,
    /// Measurement cursors: two vertical lines with ΔT/ΔV readout.
    pub cursors_active: bool,
    /// Cursor positions in seconds from origin. None = not yet placed.
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    /// Which cursor to place next on click.
    cursor_next_is_b: bool,
    /// The minimap's whole-session trace, bucketed at whatever width its
    /// strip last asked for. `None` until the strip first draws, and again
    /// whenever the buckets stop meaning what they did — see `clear` and
    /// `set_sample_interval_ms`.
    minimap_level: Option<MinimapLevel>,
    /// Samples pushed since the trace last restarted, so that a point keeps
    /// an identity across eviction: `history[0]` is
    /// `pushed_total - history.len()`. The level's bands are dropped by it.
    pushed_total: u64,
    /// Current minimap drag state.
    minimap_drag: MinimapDrag,
    /// Press origin (screen pixels) when a Shift+drag bbox-zoom is in progress.
    bbox_zoom_start_px: Option<egui::Pos2>,
    /// Latest pointer position during an in-progress bbox-zoom drag. Tracked
    /// separately so the release frame still has a valid endpoint even when
    /// hover_pos()/interact_pos() momentarily return None.
    bbox_zoom_current_px: Option<egui::Pos2>,
    /// Cached AccessKit label for the plot — rebuilt only when state changes.
    a11y_label: String,
    /// Signature of the state used to build `a11y_label`, for change detection.
    a11y_label_sig: u64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            // Not the bound: half a million points is 24 MB claimed up front
            // for a session that may last a minute. The deque doubles a
            // handful of times over an afternoon instead.
            history: VecDeque::with_capacity(1024),
            max_points: DEFAULT_MAX_SAMPLES,
            overlays: Vec::new(),
            current_mode: None,
            current_unit: String::new(),
            current_series: None,
            main_label: None,
            selected_series: None,
            hidden_overlays: HashSet::new(),
            series_options: Vec::new(),
            last_display_raw: None,
            origin: None,
            time_window_secs: 60.0,
            live: true,
            view_center: 0.0,
            gap_threshold_secs: GAP_MINIMUM_SECS,
            silences: VecDeque::new(),
            last_heard: None,
            main_missing_frames: 0,
            pending_break: None,
            pending_data_loss: false,
            pending_break_since: None,
            pending_heard_until: None,
            pending_band_from: None,
            y_axis_fixed: false,
            y_min: NumberField::new("-1", -1.0),
            y_max: NumberField::new("1", 1.0),
            y_user_set: false,
            show_mean: false,
            show_envelope: false,
            envelope_window: NumberField::new("1", 1.0),
            show_ref_line: false,
            show_crossings: true,
            ref_lines: NumberListField::default(),
            focus_ref_field: false,
            cursors_active: false,
            cursor_a: None,
            cursor_b: None,
            cursor_next_is_b: false,
            minimap_level: None,
            pushed_total: 0,
            minimap_drag: MinimapDrag::None,
            bbox_zoom_start_px: None,
            bbox_zoom_current_px: None,
            a11y_label: String::new(),
            a11y_label_sig: 0,
        }
    }

    /// Update gap detection threshold based on sample interval.
    pub fn set_sample_interval_ms(&mut self, ms: u32) {
        let interval_secs = (ms as f64 / 1000.0).max(0.1); // 0ms → use ~100ms wire time
        let threshold = (interval_secs * GAP_MULTIPLIER).max(GAP_MINIMUM_SECS);
        if threshold != self.gap_threshold_secs {
            self.gap_threshold_secs = threshold;
            // Called on every connect, and history survives a reconnect — but
            // where the trace breaks was decided against the old threshold, so
            // the level's segments no longer match what the main plot draws.
            self.minimap_level = None;
        }
    }

    /// Change how many points the history keeps, dropping the oldest at once
    /// when the new bound is below what is already in it.
    ///
    /// Lowering it is a user's answer to memory pressure, so it has to take
    /// effect now rather than at the next sample — and give the memory back,
    /// which `pop_front` alone never does.
    pub fn set_max_points(&mut self, n: usize) {
        self.max_points = n;
        for o in &mut self.overlays {
            if o.points.len() > n {
                o.points.drain(..o.points.len() - n);
                o.points.shrink_to_fit();
            }
        }
        if self.history.len() <= n {
            return;
        }
        // Drop the level rather than evict into it. One eviction costs a
        // front-bucket rescan whenever the point leaving was that bucket's
        // extreme, which on a ramp is every point: draining a bucket that way
        // is quadratic, and dropping millions of points froze the settings
        // panel for half a minute. A recut is one pass, on the next frame.
        self.minimap_level = None;
        self.evict_to(n);
        self.history.shrink_to_fit();
        for o in &mut self.overlays {
            o.points.shrink_to_fit();
        }
    }

    /// Drop the oldest points until at most `keep` are left, taking the
    /// overlay points no newer than each and the minimap's bucket with them.
    ///
    /// Only an evicted point takes overlay points with it: a point of a frame
    /// that carried only sub-values, older than the whole history, stays
    /// until the history moves past it.
    ///
    /// `push_sample` asks for one below the bound — it is about to add a
    /// point — while `set_max_points` asks for the bound itself, after
    /// dropping the level. For the one point a push sheds, evicting into the
    /// level is cheaper than the full pass a recut costs.
    fn evict_to(&mut self, keep: usize) {
        while self.history.len() > keep {
            let Some(oldest) = self.history.pop_front() else {
                break;
            };
            for o in &mut self.overlays {
                while o.points.front().is_some_and(|p| p.time <= oldest.time) {
                    o.points.pop_front();
                }
            }
            while self
                .silences
                .front()
                .is_some_and(|&(_, end)| end <= oldest.time)
            {
                self.silences.pop_front();
            }
            let first_seq = self.pushed_total.saturating_sub(self.history.len() as u64);
            if let Some(level) = &mut self.minimap_level {
                level.evict(
                    oldest.value,
                    self.history.iter().map(|p| p.value),
                    first_seq,
                );
            }
        }
    }

    /// Sub-value traces drawn beside the plotted series — what the Buffer
    /// size hint multiplies its per-point cost by.
    pub fn overlays_len(&self) -> usize {
        self.overlays.len()
    }

    /// Push a single-series sample.
    ///
    /// Test-only since the App began routing every sample through
    /// `push_sample`: it keeps the graph's own tests expressing the
    /// single-display case — by far the common one — without restating the
    /// two sub-value fields each time.
    #[cfg(test)]
    pub fn push(
        &mut self,
        value: f64,
        timestamp: Instant,
        mode: &str,
        unit: &str,
        display_raw: Option<&str>,
    ) {
        self.push_sample(PlotSample {
            value: Some(value),
            timestamp,
            mode,
            unit,
            display_raw,
            series: None,
            main_label: None,
            overlays: &[],
        });
    }

    /// Push a sample together with the sub-values drawn beside it.
    ///
    /// A sample without a value (a frame carrying only sub-values) records
    /// its overlay points and restarts the trace on a change of mode, unit or
    /// series like any other, but leaves the history, the minimap, an open
    /// break and the spoken last reading alone.
    pub fn push_sample(&mut self, sample: PlotSample<'_>) {
        let (value, timestamp, mode, unit, display_raw) = (
            sample.value,
            sample.timestamp,
            sample.mode,
            sample.unit,
            sample.display_raw,
        );
        let now = timestamp;
        if value.is_none() && sample.overlays.is_empty() {
            return;
        }

        if self.origin.is_none() {
            self.origin = Some(now);
        }
        self.main_label = sample.main_label;

        // A unit change is as much a scale change as a mode change. Auto-range
        // keeps the mode string fixed while the unit moves a decade (Ω→kΩ,
        // mV→V, nF→µF), so comparing only the mode let the trace collapse by
        // 1000x mid-plot with the axis relabelled and no gap to show why.
        // A change of plotted series restarts the trace for the same reason:
        // two sub-values can share a mode *and* a unit (T1 and T2 are both
        // "DC V"/"°C"), so switching from one to the other would otherwise
        // append onto the previous one's trace with nothing to mark the join —
        // unless the new series is already drawn beside the old one, in the
        // same unit, and the two traces only change places.
        let same_scale = self.current_mode.as_deref() == Some(mode) && self.current_unit == unit;
        let series_changed = self.current_series.as_deref() != sample.series;
        let swapped = same_scale && series_changed && self.swap_plotted_series(sample.series);
        if !swapped && (!same_scale || series_changed) {
            self.history.clear();
            self.overlays.clear();
            self.current_mode = Some(mode.to_string());
            self.current_unit = unit.to_string();
            self.current_series = sample.series.map(str::to_owned);
            self.origin = Some(now);
            self.live = true;
            self.view_center = 0.0;
            // Drop any pinned Y range too: it was chosen for the previous
            // mode's scale, and keeping it would plot ohms against volt bounds
            // — the trace lands far outside the plot and the graph just looks
            // empty, with the old numbers still on the axis. `clear()` and
            // `reset_view()` both release these for the same reason.
            self.y_axis_fixed = false;
            self.y_user_set = false;
            self.cursor_a = None;
            self.cursor_b = None;
            self.cursor_next_is_b = false;
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
            self.minimap_level = None;
            self.pushed_total = 0;
            self.last_display_raw = None;
            self.silences.clear();
            self.main_missing_frames = 0;
        }
        self.heard(now);
        self.register_overlays(sample.overlays);
        match value {
            Some(value) => {
                let last = self.history.back().map(|p| p.time);
                if self.stopped_for(self.main_missing_frames, last, now)
                    && self.pending_break.is_none()
                {
                    self.pending_break = Some(GapKind::NoData);
                }
                self.main_missing_frames = 0;
                self.push_point(value, now, display_raw);
            }
            None => self.main_missing_frames += 1,
        }
        for i in 0..self.overlays.len() {
            let label = self.overlays[i].label.as_str();
            let Some(&(_, v)) = sample.overlays.iter().find(|(l, _)| *l == label) else {
                self.overlays[i].missing_frames += 1;
                continue;
            };
            let o = &self.overlays[i];
            let last = o.points.back().copied();
            if self.stopped_for(o.missing_frames, last.map(|p| p.time), now)
                && let Some(OverlayPoint {
                    time,
                    value: Some(_),
                }) = last
            {
                self.overlays[i].push(OverlayPoint { time, value: None }, self.max_points);
            }
            let o = &mut self.overlays[i];
            o.missing_frames = 0;
            o.push(
                OverlayPoint {
                    time: now,
                    value: v,
                },
                self.max_points,
            );
        }
    }

    /// Whether a series that last had a point at `last`, and has been missing
    /// from `missing` frames since, stopped rather than paused: its trace
    /// breaks before its next point instead of being drawn across.
    ///
    /// Frames kept coming, so no silence says so — a UT181A with REL off for
    /// half a minute, or a held UT61E+ sending only its AC component. Both a
    /// run of frames and a stretch past the gap threshold are needed, as for
    /// dropping a series option: the frames alone would break a part the
    /// meter sends every other frame at a slow interval, and the time alone
    /// one it sends every other frame at 0.67 s.
    fn stopped_for(&self, missing: u32, last: Option<Instant>, now: Instant) -> bool {
        missing >= SERIES_DROP_FRAMES
            && last.is_some_and(|last| {
                now.checked_duration_since(last)
                    .is_some_and(|d| d.as_secs_f64() > self.gap_threshold_secs)
            })
    }

    /// Note that a frame arrived at `t`, recording the silence before it if
    /// there was one worth remembering (see `silences`).
    fn heard(&mut self, t: Instant) {
        if let Some(last) = self.last_heard
            && t.checked_duration_since(last)
                .is_some_and(|d| d.as_secs_f64() > GAP_MINIMUM_SECS)
        {
            // A stream that never grows the history is never evicted from;
            // the bound keeps it from growing without limit all the same.
            while self.silences.len() >= self.max_points.max(1) {
                self.silences.pop_front();
            }
            self.silences.push_back((last, t));
        }
        self.last_heard = Some(t);
    }

    /// Whether no frame at all arrived for longer than the gap threshold
    /// somewhere between the frames at `from` and `to`.
    fn silent_between(&self, from: Instant, to: Instant) -> bool {
        let first = self.silences.partition_point(|&(start, _)| start < from);
        self.silences
            .range(first..)
            .take_while(|&&(_, end)| end <= to)
            .any(|&(start, end)| {
                end.checked_duration_since(start)
                    .is_some_and(|d| d.as_secs_f64() > self.gap_threshold_secs)
            })
    }

    /// What the main reading goes by: the meter's name for it, or
    /// [`MAIN_SERIES`].
    pub(crate) fn main_name(&self) -> &'static str {
        self.main_label.unwrap_or(MAIN_SERIES)
    }

    /// Plot `series` in place of the current one by swapping the two traces:
    /// the one of that name drawn beside the plotted series becomes the
    /// plotted one, and the plotted one is drawn beside it under its own
    /// name. `false`, touching nothing, when no such trace is drawn — a
    /// series in another unit, whose past values were never kept.
    ///
    /// The time axis, the unit, the Y range and the cursors all still apply.
    /// A break in a sub-value's trace carries no reason, so on becoming the
    /// plotted series it is drawn as a gap, an over-range stretch included.
    fn swap_plotted_series(&mut self, series: Option<&str>) -> bool {
        let incoming_name = series.unwrap_or(self.main_name());
        let Some(i) = self.overlays.iter().position(|o| o.label == incoming_name) else {
            return false;
        };
        let incoming = std::mem::take(&mut self.overlays[i].points);

        let mut outgoing: VecDeque<OverlayPoint> = VecDeque::with_capacity(self.history.len());
        let mut prev: Option<Instant> = None;
        for p in self.history.drain(..) {
            if let (Some(_), Some(time)) = (p.break_before, prev) {
                outgoing.push_back(OverlayPoint { time, value: None });
            }
            outgoing.push_back(OverlayPoint {
                time: p.time,
                value: Some(p.value),
            });
            prev = Some(p.time);
        }
        if let (Some(_), Some(time)) = (self.pending_break, prev) {
            outgoing.push_back(OverlayPoint { time, value: None });
        }
        while outgoing.len() > self.max_points.max(1) {
            outgoing.pop_front();
        }

        let mut pending = None;
        for p in incoming {
            match p.value {
                Some(value) => self.history.push_back(DataPoint {
                    time: p.time,
                    value,
                    break_before: pending.take(),
                    break_last_sample: None,
                    break_had_data_loss: false,
                    break_band_late: false,
                }),
                None => pending = Some(GapKind::NoData),
            }
        }
        self.pending_break = pending;
        self.pending_data_loss = false;
        self.pending_break_since = None;
        self.pending_heard_until = None;
        self.pending_band_from = None;

        self.overlays[i] = OverlaySeries {
            label: self
                .current_series
                .take()
                .unwrap_or_else(|| self.main_name().to_string()),
            points: outgoing,
            missing_frames: 0,
        };
        self.main_missing_frames = 0;
        self.current_series = series.map(str::to_owned);
        self.minimap_level = None;
        self.pushed_total = self.history.len() as u64;
        self.last_display_raw = None;
        true
    }

    /// Start a trace for each sub-value seen for the first time, up to
    /// [`MAX_OVERLAYS`]. It begins at its first point: nothing is back-filled.
    fn register_overlays(&mut self, overlays: &[(&str, Option<f64>)]) {
        for &(label, _) in overlays {
            if self.overlays.len() >= MAX_OVERLAYS {
                break;
            }
            if self.overlays.iter().any(|o| o.label == label) {
                continue;
            }
            self.overlays.push(OverlaySeries {
                label: label.to_string(),
                points: VecDeque::new(),
                missing_frames: 0,
            });
        }
    }

    /// Append one point of the plotted series.
    fn push_point(&mut self, value: f64, now: Instant, display_raw: Option<&str>) {
        // Track the most recent raw display string so the a11y plot
        // summary can speak it verbatim. We update it in-place to avoid
        // allocating per push when the underlying `Cow<'static, str>` is a
        // borrowed protocol-level constant.
        match (display_raw, &mut self.last_display_raw) {
            (Some(s), Some(buf)) => {
                buf.clear();
                buf.push_str(s);
            }
            (Some(s), None) => self.last_display_raw = Some(s.to_string()),
            (None, _) => self.last_display_raw = None,
        }

        // One short of the bound: this sample is about to take the last slot.
        self.evict_to(self.max_points.saturating_sub(1));

        let band_from = self.pending_band_from.take();
        let last_overload = self.pending_break_since.take();
        let point = DataPoint {
            time: now,
            value,
            break_before: self.pending_break.take(),
            break_last_sample: band_from.or(last_overload),
            break_had_data_loss: std::mem::take(&mut self.pending_data_loss),
            break_band_late: band_from.is_some(),
        };
        self.pending_heard_until = None;
        // Everything the minimap's level needs, decided against the point
        // before this one exactly as `build_segments_for_range` decides it for
        // a consecutive pair — so appending and rebuilding cannot disagree.
        let seq = self.pushed_total;
        let t = self.elapsed_secs(point.time);
        let prev_time = self.history.back().map(|p| p.time);
        let break_kind = prev_time.and_then(|prev| self.breaks_before(prev, &point));
        let gaps = match (prev_time, break_kind) {
            (Some(prev), Some(kind)) => self.gap_entries(prev, &point, kind),
            _ => [None, None],
        };
        self.history.push_back(point);
        if let Some(level) = &mut self.minimap_level {
            level.append(t, value, break_kind);
            for (start, end, kind) in gaps.into_iter().flatten() {
                level.push_gap(start, end, kind, seq);
            }
        }
        self.pushed_total += 1;
    }

    /// Offer the sub-values this frame carries, as (label, resolved unit),
    /// for the toolbar's series selector. `now` is the frame's timestamp.
    ///
    /// An option outlives frames that lack it: it is dropped only once
    /// [`SERIES_DROP_FRAMES`] consecutive frames went without it *and* it has
    /// not been seen for longer than the gap threshold. The frame count alone
    /// would drop a sub-value the meter sends every other frame (the UT61E+'s
    /// AC+DC V) or less; the time alone would drop every option across a
    /// pause. A selection whose option is dropped falls back to the main
    /// reading — the meter left the mode that produced it. Dropping it on the
    /// first frame instead would throw the trace away every time one reply
    /// arrives short or with the sub-value bit clear.
    pub fn set_series_options(&mut self, options: &[(&str, &str)], now: Instant) {
        for o in &mut self.series_options {
            match options.iter().find(|(label, _)| *label == o.label) {
                Some(&(_, unit)) => {
                    o.missing_frames = 0;
                    o.last_seen = now;
                    if o.unit != unit {
                        o.unit = unit.to_string();
                    }
                }
                None => o.missing_frames += 1,
            }
        }
        let threshold = self.gap_threshold_secs;
        self.series_options.retain(|o| {
            let unseen = now
                .checked_duration_since(o.last_seen)
                .is_some_and(|d| d.as_secs_f64() > threshold);
            o.missing_frames < SERIES_DROP_FRAMES || !unseen
        });
        for &(label, unit) in options {
            if !self.series_options.iter().any(|o| o.label == label) {
                self.series_options.push(SeriesOption {
                    label: label.to_string(),
                    unit: unit.to_string(),
                    missing_frames: 0,
                    last_seen: now,
                });
            }
        }
        if let Some(sel) = &self.selected_series
            && !self.series_options.iter().any(|o| &o.label == sel)
        {
            self.selected_series = None;
        }
    }

    /// The sub-value label currently selected for plotting, or `None` for the
    /// meter's main reading.
    #[cfg(test)]
    pub fn selected_series(&self) -> Option<&str> {
        self.selected_series.as_deref()
    }

    /// The selected sub-value with the unit it was last offered in, or
    /// `None` while the main reading is plotted.
    pub fn selected_series_offer(&self) -> Option<(&str, &str)> {
        let sel = self.selected_series.as_deref()?;
        self.series_options
            .iter()
            .find(|o| o.label == sel)
            .map(|o| (o.label.as_str(), o.unit.as_str()))
    }

    /// Mode of the trace being drawn, or `None` before the first sample.
    pub fn plotted_mode(&self) -> Option<&str> {
        self.current_mode.as_deref()
    }

    /// Unit of the series being plotted. The stats panel captions its
    /// visible-window block with this, which is not the meter's main unit
    /// when a sub-value is plotted.
    pub fn plotted_unit(&self) -> &str {
        &self.current_unit
    }

    /// Record that the series was interrupted — the meter reported a value
    /// that can't be plotted (an overload), so the next point starts a new
    /// segment.
    ///
    /// Needed because gap detection is otherwise purely time-based: an
    /// over-range excursion shorter than the gap threshold leaves no hole in
    /// the timestamps, so the trace would be drawn straight through it and
    /// the visible-range stats and integral would run across a value the
    /// meter never measured.
    ///
    /// Sub-value traces are not split by it: each point keeps its own time,
    /// and an over-range sub-value breaks its own trace. The App records the
    /// frame's sub-values after this, as a sample without a value.
    ///
    /// Arriving after a word shown instead of a reading, it starts the band
    /// here and leaves the word's stretch a gap (`gap_entries`). A word
    /// after that band is drawn into it: one interruption splits in two, not
    /// three.
    pub fn push_break(&mut self, timestamp: Instant) {
        self.heard(timestamp);
        // Updated on every overload sample, not just the first: they close
        // the band and advance the live view for as long as the meter stays
        // over range.
        self.pending_break_since = Some(timestamp);
        self.pending_heard_until = Some(timestamp);
        match self.pending_break {
            None => self.pending_break = Some(GapKind::Overload),
            // Only `push_data_loss` marks a loss, so this gap is the word's.
            Some(GapKind::NoData) if !self.pending_data_loss => {
                self.pending_break = Some(GapKind::Overload);
                self.pending_band_from = Some(timestamp);
            }
            Some(_) => {}
        }
    }

    /// Record that the meter showed a word instead of a reading ("Auto" with
    /// the probes lifted): the trace breaks as for an overload, but the
    /// stretch is drawn as a gap rather than an over-range band, since the
    /// meter was not over range.
    ///
    /// Keeps the live view moving the way an overload does — the meter is
    /// still sending. Arriving during an overload, it closes the band at the
    /// last OL sample and leaves the rest a gap, through the same split a
    /// dropout mid-overload takes (`gap_entries`).
    pub fn push_no_reading(&mut self, timestamp: Instant) {
        self.heard(timestamp);
        self.pending_heard_until = Some(timestamp);
        if self.pending_break == Some(GapKind::Overload) {
            self.pending_data_loss = true;
            return;
        }
        self.pending_break = Some(GapKind::NoData);
    }

    /// Record that data was genuinely lost — the link dropped, or
    /// acquisition was stopped — as opposed to the meter merely going quiet.
    ///
    /// The graph cannot tell those apart from timestamps. This meter pauses
    /// its output for over a second while auto-ranging (measured: 462 ms then
    /// 1153 ms stepping 2.2MΩ → 22MΩ → 220MΩ, against a 97 ms steady
    /// cadence), which by elapsed time alone is indistinguishable from an
    /// unplugged cable. The App receives the disconnect and drives pause, so
    /// it states what happened instead of leaving the graph to infer it.
    ///
    /// Every sub-value trace breaks here too, with a `None` at its own last
    /// time: a loss says nothing about when the next point comes, and with no
    /// timestamp of its own the call has no other time to put it at.
    pub fn push_data_loss(&mut self) {
        self.pending_data_loss = true;
        if self.pending_break.is_none() {
            self.pending_break = Some(GapKind::NoData);
        }
        for o in &mut self.overlays {
            if let Some(&OverlayPoint {
                time,
                value: Some(_),
            }) = o.points.back()
            {
                o.push(OverlayPoint { time, value: None }, self.max_points);
            }
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
        // `selected_series` deliberately survives: Ctrl+L clears the data, not
        // the user's choice of what to plot. `series_options` and
        // `hidden_overlays` survive too — the meter is still sending, and the
        // next sample refreshes the offer.
        self.overlays.clear();
        self.current_series = None;
        self.pending_break = None;
        self.pending_data_loss = false;
        self.pending_break_since = None;
        self.pending_heard_until = None;
        self.pending_band_from = None;
        self.silences.clear();
        self.last_heard = None;
        self.main_missing_frames = 0;
        self.current_mode = None;
        self.current_unit.clear();
        self.last_display_raw = None;
        self.origin = None;
        self.live = true;
        self.view_center = 0.0;
        self.y_axis_fixed = false;
        self.y_user_set = false;
        self.cursor_a = None;
        self.cursor_b = None;
        self.cursor_next_is_b = false;
        self.minimap_drag = MinimapDrag::None;
        self.bbox_zoom_start_px = None;
        self.bbox_zoom_current_px = None;
        self.minimap_level = None;
        self.pushed_total = 0;
    }

    /// When the oldest point still in the history or a sub-value trace was
    /// sampled, or `None` while there is none: where what the graph holds
    /// begins.
    pub fn first_point_time(&self) -> Option<Instant> {
        self.history
            .front()
            .map(|p| p.time)
            .into_iter()
            .chain(
                self.overlays
                    .iter()
                    .filter_map(|o| o.points.front().map(|p| p.time)),
            )
            .min()
    }

    /// When the newest point in the history or a sub-value trace was
    /// sampled.
    fn last_point_time(&self) -> Option<Instant> {
        self.history
            .back()
            .map(|p| p.time)
            .into_iter()
            .chain(
                self.overlays
                    .iter()
                    .filter_map(|o| o.points.back().map(|p| p.time)),
            )
            .max()
    }

    /// Cut the minimap's level to `width`, keeping the one already cut when
    /// the strip's scale has not stepped past it.
    ///
    /// A recut is one pass over the history — what the strip used to pay
    /// every frame — and falls only on the ~70 geometric width steps a
    /// session takes and on window resizes. Because it runs the same `append`
    /// the per-sample path does, a recut level and a grown one are identical
    /// by construction.
    pub(super) fn ensure_level(&mut self, width: f64) {
        if self.minimap_level.as_ref().map(|l| l.width()) != Some(width) {
            self.minimap_level = Some(self.build_level(width));
        }
    }

    /// Bucket the whole history at `width`, from scratch.
    fn build_level(&self, width: f64) -> MinimapLevel {
        let mut level = MinimapLevel::new(width);
        let first_seq = self.pushed_total.saturating_sub(self.history.len() as u64);
        let mut prev_time: Option<Instant> = None;
        for (i, point) in self.history.iter().enumerate() {
            let break_kind = prev_time.and_then(|prev| self.breaks_before(prev, point));
            level.append(self.elapsed_secs(point.time), point.value, break_kind);
            if let (Some(prev), Some(kind)) = (prev_time, break_kind) {
                for (start, end, k) in self.gap_entries(prev, point, kind).into_iter().flatten() {
                    level.push_gap(start, end, k, first_seq + i as u64);
                }
            }
            prev_time = Some(point.time);
        }
        level
    }

    fn elapsed_secs(&self, t: Instant) -> f64 {
        match self.origin {
            // Use checked_duration_since to avoid panic if clock goes backward
            // (can happen on VM suspend/resume or NTP adjustments).
            Some(origin) => t
                .checked_duration_since(origin)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            None => 0.0,
        }
    }

    /// Return the half-open index range `[start, end)` of history points
    /// whose elapsed time falls within `[x_min, x_max]`.
    fn visible_index_range(&self, x_min: f64, x_max: f64) -> (usize, usize) {
        self.time_index_range(&self.history, |p| p.time, x_min, x_max)
    }

    /// The half-open index range `[start, end)` of `points` whose elapsed
    /// time falls within `[x_min, x_max]`.
    ///
    /// The history and every overlay are time-ordered (push_back only,
    /// pop_front on eviction), so we can binary-search via `partition_point`.
    /// `VecDeque` doesn't expose a single contiguous slice, but its two halves
    /// from `as_slices()` are each sorted, so we search both and combine the
    /// results.
    fn time_index_range<T>(
        &self,
        points: &VecDeque<T>,
        time: impl Fn(&T) -> Instant,
        x_min: f64,
        x_max: f64,
    ) -> (usize, usize) {
        let (a, b) = points.as_slices();
        let a_len = a.len();

        // Find first index with elapsed_secs >= x_min.
        let start_a = a.partition_point(|p| self.elapsed_secs(time(p)) < x_min);
        let start = if start_a < a_len {
            start_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(time(p)) < x_min)
        };

        // Find first index with elapsed_secs > x_max (i.e. one past the last visible).
        let end_a = a.partition_point(|p| self.elapsed_secs(time(p)) <= x_max);
        let end = if end_a < a_len {
            end_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(time(p)) <= x_max)
        };

        (start, end)
    }

    /// Why the line breaks between `prev` and `point`, or `None` if it
    /// doesn't. A recorded break wins over the silence test: an overload
    /// that lasted less than the gap threshold is still an overload, not a
    /// dropout.
    fn breaks_before(&self, prev: Instant, point: &DataPoint) -> Option<GapKind> {
        if let Some(kind) = point.break_before {
            return Some(kind);
        }
        self.silent_between(prev, point.time)
            .then_some(GapKind::NoData)
    }

    /// The band(s) one interruption draws, between `prev` and `point`.
    ///
    /// An interruption can be two things end to end: a stretch the meter
    /// reported on, then a stretch it didn't. Losing the link mid-overload is
    /// exactly that, and folding the silence into the band would claim the
    /// meter was over range for a period it never reported at all.
    ///
    /// Shared by the main graph's segment builder and the minimap level's
    /// per-sample append, so the two cannot come to different conclusions
    /// about the same interruption.
    fn gap_entries(
        &self,
        prev: Instant,
        point: &DataPoint,
        kind: GapKind,
    ) -> [Option<(f64, f64, GapKind)>; 2] {
        let start = self.elapsed_secs(prev);
        let end = self.elapsed_secs(point.time);
        match (kind, point.break_last_sample) {
            // A word shown instead of a reading, then over range.
            (GapKind::Overload, Some(first)) if point.break_band_late => {
                let band_from = self.elapsed_secs(first);
                [
                    Some((start, band_from, GapKind::NoData)),
                    Some((band_from, end, GapKind::Overload)),
                ]
            }
            (GapKind::Overload, Some(last)) if point.break_had_data_loss => {
                let heard_until = self.elapsed_secs(last);
                [
                    Some((start, heard_until, GapKind::Overload)),
                    Some((heard_until, end, GapKind::NoData)),
                ]
            }
            _ => [Some((start, end, kind)), None],
        }
    }

    /// Combined render: toolbar + main graph + minimap.
    pub fn show(&mut self, ui: &mut Ui, tc: &ThemeColors) {
        self.handle_keyboard(ui.ctx());
        self.show_toolbar(ui, tc);
        let minimap_reserve = MINIMAP_HEIGHT + 30.0;
        let main_height = (ui.available_height() - minimap_reserve).max(60.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), main_height), |ui| {
            self.show_main(ui, tc);
        });
        ui.add_space(4.0);
        self.show_minimap(ui, tc);
    }

    /// Full-history segments, through the same builder the minimap caches.
    #[cfg(test)]
    fn all_segments(&self) -> Vec<Vec<[f64; 2]>> {
        self.build_segments_for_range(0, self.history.len()).0
    }

    /// Gap ranges across the whole history, through the same builder the main
    /// graph renders from — so these tests exercise the path that actually
    /// draws the gap markers.
    #[cfg(test)]
    fn visible_gaps(&self) -> Vec<(f64, f64, GapKind)> {
        self.build_segments_for_range(0, self.history.len()).1
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.history.len()
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Full-history segments of one overlay, through the same builder the
    /// main graph renders from.
    #[cfg(test)]
    fn overlay_segments(&self, label: &str) -> Vec<Vec<[f64; 2]>> {
        self.build_overlay_segments_for_range(self.overlay(label), f64::NEG_INFINITY, f64::INFINITY)
    }

    #[cfg(test)]
    fn overlay_values(&self, label: &str) -> Vec<Option<f64>> {
        self.overlay(label).points.iter().map(|p| p.value).collect()
    }

    /// An overlay's points as (seconds from origin, value).
    #[cfg(test)]
    fn overlay_points(&self, label: &str) -> Vec<(f64, Option<f64>)> {
        self.overlay(label)
            .points
            .iter()
            .map(|p| (self.elapsed_secs(p.time), p.value))
            .collect()
    }

    #[cfg(test)]
    fn overlay(&self, label: &str) -> &OverlaySeries {
        self.overlays
            .iter()
            .find(|o| o.label == label)
            .unwrap_or_else(|| panic!("no overlay {label:?}"))
    }

    /// Labels the series selector offers, in order.
    #[cfg(test)]
    fn series_option_labels(&self) -> Vec<&str> {
        self.series_options
            .iter()
            .map(|o| o.label.as_str())
            .collect()
    }

    #[cfg(test)]
    fn overlay_labels(&self) -> Vec<&str> {
        self.overlays.iter().map(|o| o.label.as_str()).collect()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
