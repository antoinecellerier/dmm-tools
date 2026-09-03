//! Real-time scrolling graph: the history buffer the App pushes samples into,
//! and the widget that draws it.
//!
//! The concerns live in submodules — [`view`] (what slice is shown and the
//! gestures that move it), [`toolbar`], [`render`] (the main plot),
//! [`minimap`], [`analysis`] (visible-slice statistics) and [`time`]
//! (axis label formatting) — all of which add methods to the one [`Graph`]
//! declared here, so the type's public API is unchanged by the split.

mod analysis;
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

use crate::theme::ThemeColors;
use minimap::{MINIMAP_HEIGHT, MinimapDrag};

/// Maximum number of points to keep in the history buffer.
const MAX_POINTS: usize = 10_000;

/// Maximum number of same-unit sub-values drawn beside the plotted series.
///
/// The protocols send at most four sub-values per frame (UT181A), so this is
/// the ceiling the wire imposes rather than a display choice.
pub(crate) const MAX_OVERLAYS: usize = 4;

/// Consecutive frames without the selected sub-value before the selection is
/// dropped.
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
    /// Timestamp of the last non-plottable sample in that interruption.
    ///
    /// Bounds what we actually observed. If the meter goes over range and
    /// then the link drops, OL samples stop arriving and this stays at the
    /// last one we heard — everything after it is silence, not over-range.
    break_last_sample: Option<Instant>,
    /// The interruption included a genuine loss of data, as reported by the
    /// App — not merely a quiet meter. See `push_data_loss`.
    break_had_data_loss: bool,
}

/// One sub-value trace drawn beside the plotted series.
///
/// `values` runs in lockstep with `history`: index `i` holds the sub-value
/// that arrived with `history[i]`, or `None` when that frame carried no such
/// sub-value or reported it over range. Keeping them parallel rather than
/// widening `DataPoint` costs nothing for the single-display meters that make
/// up most of the device table, and lets `visible_index_range` index both.
struct OverlaySeries {
    label: String,
    values: VecDeque<Option<f64>>,
}

/// One measurement as the graph should plot it.
///
/// Built by the App, which decides *which* series is plotted and which
/// sub-values share its unit; the graph never sees `dmm_lib` measurement
/// types.
pub struct PlotSample<'a> {
    pub value: f64,
    pub timestamp: Instant,
    pub mode: &'a str,
    pub unit: &'a str,
    pub display_raw: Option<&'a str>,
    /// Label of the sub-value being plotted, or `None` for the meter's main
    /// reading. A change here restarts the trace: two sub-values can share a
    /// mode and a unit (T1 and T2), so nothing else would tell them apart.
    pub series: Option<&'a str>,
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
    /// Same-unit sub-value traces, in lockstep with `history`.
    overlays: Vec<OverlaySeries>,
    current_mode: Option<String>,
    current_unit: String,
    /// Label of the series `history` was recorded from; `None` for the main
    /// reading. Distinct from `selected_series`: this one only moves when a
    /// sample actually arrives for the new choice.
    current_series: Option<String>,
    /// Which series the toolbar is asking for. Session-only — a selection is
    /// about the meter's current mode, so persisting it across restarts would
    /// silently plot a sub-value the next session may not even have.
    selected_series: Option<String>,
    /// Consecutive offers that did not include `selected_series`. Debounces
    /// the drop, see [`SERIES_DROP_FRAMES`].
    series_missing_frames: u32,
    /// Sub-value labels the user has switched off in the toolbar's **Show:**
    /// group. Session-only, and deliberately keyed by label rather than by
    /// index so a choice survives `clear()`, a change of plotted series and a
    /// sub-value that disappears and comes back. Hidden overlays are still
    /// recorded in lockstep — re-showing one brings its history with it.
    hidden_overlays: HashSet<String>,
    /// Sub-values the latest sample offered, as (label, resolved unit).
    /// Drives the toolbar's selector.
    series_options: Vec<(String, String)>,
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
    /// A non-plottable sample arrived since the last plotted point, so the
    /// next one starts a new segment. Time-based gap detection can't see
    /// this: an over-range excursion shorter than the threshold leaves no
    /// time hole.
    pending_break: Option<GapKind>,
    /// A genuine dropout happened during the open interruption: the link
    /// went down, or acquisition was stopped. Distinct from the meter simply
    /// not sending — see `push_data_loss`.
    pending_data_loss: bool,
    /// Timestamp of the newest overload sample while a break is open.
    ///
    /// Overload measurements carry a timestamp like any other; they just have
    /// no plottable value. Keeping the newest one lets the live view follow
    /// the present while the meter is over range, instead of freezing at the
    /// last plotted point.
    pending_break_since: Option<Instant>,
    /// When true, Y axis uses fixed min/max instead of auto-scaling.
    pub y_axis_fixed: bool,
    /// Fixed Y-axis minimum (editable text buffer for UI).
    y_min_text: String,
    /// Fixed Y-axis maximum (editable text buffer for UI).
    y_max_text: String,
    /// Parsed fixed Y-axis min.
    y_fixed_min: f64,
    /// Parsed fixed Y-axis max.
    y_fixed_max: f64,
    /// Whether the user has manually set Y-axis values this session.
    y_user_set: bool,
    /// Show mean line overlay.
    pub show_mean: bool,
    /// Show min/max envelope band.
    pub show_envelope: bool,
    /// Envelope bucket width in seconds (user-configurable).
    envelope_window_text: String,
    envelope_window_secs: f64,
    /// Reference lines: show horizontal lines at these values.
    pub show_ref_line: bool,
    /// Show trigger crossing markers on reference lines.
    pub show_crossings: bool,
    ref_line_text: String,
    ref_line_values: Vec<f64>,
    /// Measurement cursors: two vertical lines with ΔT/ΔV readout.
    pub cursors_active: bool,
    /// Cursor positions in seconds from origin. None = not yet placed.
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    /// Which cursor to place next on click.
    cursor_next_is_b: bool,
    /// Cached segment data for minimap (full history), rebuilt only when
    /// `history_version` changes.
    cached_segments: Vec<Vec<[f64; 2]>>,
    /// Full-history gaps, for the minimap's overload bands. (An earlier
    /// version of this field was write-only and removed; the minimap now
    /// reads it.)
    cached_gaps: Vec<(f64, f64, GapKind)>,
    /// Monotonic counter incremented on every push/clear/mode-change.
    /// Used as the cache key instead of `history.len()` because a
    /// push_back+pop_front leaves the length unchanged but the data differs.
    history_version: u64,
    /// Version when the cache was last rebuilt.
    cache_version: u64,
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
            history: VecDeque::with_capacity(MAX_POINTS),
            overlays: Vec::new(),
            current_mode: None,
            current_unit: String::new(),
            current_series: None,
            selected_series: None,
            series_missing_frames: 0,
            hidden_overlays: HashSet::new(),
            series_options: Vec::new(),
            last_display_raw: None,
            origin: None,
            time_window_secs: 60.0,
            live: true,
            view_center: 0.0,
            gap_threshold_secs: GAP_MINIMUM_SECS,
            pending_break: None,
            pending_data_loss: false,
            pending_break_since: None,
            y_axis_fixed: false,
            y_min_text: "-1".to_string(),
            y_max_text: "1".to_string(),
            y_fixed_min: -1.0,
            y_fixed_max: 1.0,
            y_user_set: false,
            show_mean: false,
            show_envelope: false,
            envelope_window_text: "1".to_string(),
            envelope_window_secs: 1.0,
            show_ref_line: false,
            show_crossings: true,
            ref_line_text: String::new(),
            ref_line_values: Vec::new(),
            cursors_active: false,
            cursor_a: None,
            cursor_b: None,
            cursor_next_is_b: false,
            cached_segments: Vec::new(),
            cached_gaps: Vec::new(),
            history_version: 0,
            cache_version: 0,
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
        self.gap_threshold_secs = (interval_secs * GAP_MULTIPLIER).max(GAP_MINIMUM_SECS);
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
            value,
            timestamp,
            mode,
            unit,
            display_raw,
            series: None,
            overlays: &[],
        });
    }

    /// Push a sample together with the sub-values drawn beside it.
    pub fn push_sample(&mut self, sample: PlotSample<'_>) {
        let (value, timestamp, mode, unit, display_raw) = (
            sample.value,
            sample.timestamp,
            sample.mode,
            sample.unit,
            sample.display_raw,
        );
        let now = timestamp;

        if self.origin.is_none() {
            self.origin = Some(now);
        }

        // A unit change is as much a scale change as a mode change. Auto-range
        // keeps the mode string fixed while the unit moves a decade (Ω→kΩ,
        // mV→V, nF→µF), so comparing only the mode let the trace collapse by
        // 1000x mid-plot with the axis relabelled and no gap to show why.
        // A change of plotted series restarts the trace for the same reason:
        // two sub-values can share a mode *and* a unit (T1 and T2 are both
        // "DC V"/"°C"), so switching from one to the other would otherwise
        // append onto the previous one's trace with nothing to mark the join.
        if self.current_mode.as_deref() != Some(mode)
            || self.current_unit != unit
            || self.current_series.as_deref() != sample.series
        {
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
            self.invalidate_cache();
            self.last_display_raw = None;
        }
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

        if self.history.len() >= MAX_POINTS {
            self.history.pop_front();
            for o in &mut self.overlays {
                o.values.pop_front();
            }
        }

        // Register sub-values seen for the first time, back-filled with
        // `None` for every point already in history — a COMP High/Low that
        // appears mid-session must line up with the trace it accompanies, not
        // start at index 0.
        for &(label, _) in sample.overlays {
            if self.overlays.len() >= MAX_OVERLAYS {
                break;
            }
            if self.overlays.iter().any(|o| o.label == label) {
                continue;
            }
            self.overlays.push(OverlaySeries {
                label: label.to_string(),
                values: vec![None; self.history.len()].into(),
            });
        }

        self.history.push_back(DataPoint {
            time: now,
            value,
            break_before: self.pending_break.take(),
            break_last_sample: self.pending_break_since.take(),
            break_had_data_loss: std::mem::take(&mut self.pending_data_loss),
        });
        for o in &mut self.overlays {
            let v = sample
                .overlays
                .iter()
                .find(|(label, _)| *label == o.label)
                .and_then(|(_, v)| *v);
            o.values.push_back(v);
        }
        debug_assert!(
            self.overlays
                .iter()
                .all(|o| o.values.len() == self.history.len()),
            "overlay series out of lockstep with history"
        );
        self.history_version += 1;
    }

    /// Offer the sub-values the meter is currently sending, as
    /// (label, resolved unit), for the toolbar's series selector.
    ///
    /// A selection whose label stops being offered for [`SERIES_DROP_FRAMES`]
    /// consecutive frames falls back to the main reading — the meter left the
    /// mode that produced it. Dropping it on the first frame instead would
    /// throw the trace away every time one reply arrives short or with the
    /// sub-value bit clear; a single such frame is skipped rather than
    /// plotted, so nothing is lost while the count runs.
    pub fn set_series_options(&mut self, options: &[(&str, &str)]) {
        if let Some(sel) = &self.selected_series {
            if options.iter().any(|(label, _)| *label == sel.as_str()) {
                self.series_missing_frames = 0;
            } else {
                self.series_missing_frames += 1;
                if self.series_missing_frames >= SERIES_DROP_FRAMES {
                    self.selected_series = None;
                    self.series_missing_frames = 0;
                }
            }
        }
        // Called on every sample; only pay for the strings when the offer
        // actually changed.
        let unchanged = self.series_options.len() == options.len()
            && self
                .series_options
                .iter()
                .zip(options)
                .all(|((l, u), (nl, nu))| l.as_str() == *nl && u.as_str() == *nu);
        if !unchanged {
            self.series_options = options
                .iter()
                .map(|(l, u)| ((*l).to_string(), (*u).to_string()))
                .collect();
        }
    }

    /// The sub-value label currently selected for plotting, or `None` for the
    /// meter's main reading.
    pub fn selected_series(&self) -> Option<&str> {
        self.selected_series.as_deref()
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
    /// The frame's sub-values are dropped with it: overlays are indexed by
    /// history position, and an over-range plotted series adds no position.
    /// They resume at the next plotted point, split by this break like the
    /// main trace.
    pub fn push_break(&mut self, timestamp: Instant) {
        // Updated on every overload sample, not just the first: it is what
        // advances the live view for as long as the meter stays over range.
        self.pending_break_since = Some(timestamp);
        if self.pending_break.is_some() {
            return;
        }
        self.pending_break = Some(GapKind::Overload);
        // The break only becomes visible once the next point lands, but the
        // caches key on this counter — bump it so a stale segment list built
        // before the overload isn't reused.
        self.invalidate_cache();
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
    pub fn push_data_loss(&mut self) {
        self.pending_data_loss = true;
        if self.pending_break.is_none() {
            self.pending_break = Some(GapKind::NoData);
            self.invalidate_cache();
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
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.history_version += 1;
    }

    /// Rebuild the cached full-history segments and gaps (for the minimap)
    /// only if history has changed since the last rebuild.
    ///
    /// The main graph builds its own over the visible slice; this is the
    /// whole-history pass the minimap needs.
    fn ensure_cache(&mut self) {
        if self.cache_version != self.history_version {
            let (segments, gaps) = self.build_segments_for_range(0, self.history.len());
            self.cached_segments = segments;
            self.cached_gaps = gaps;
            self.cache_version = self.history_version;
        }
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
    ///
    /// `self.history` is time-ordered (push_back only, pop_front on eviction),
    /// so we can binary-search via `partition_point`. `VecDeque` doesn't expose
    /// a single contiguous slice, but its two halves from `as_slices()` are
    /// each sorted, so we search both and combine the results.
    fn visible_index_range(&self, x_min: f64, x_max: f64) -> (usize, usize) {
        let (a, b) = self.history.as_slices();
        let a_len = a.len();

        // Find first index with elapsed_secs >= x_min.
        let start_a = a.partition_point(|p| self.elapsed_secs(p.time) < x_min);
        let start = if start_a < a_len {
            start_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(p.time) < x_min)
        };

        // Find first index with elapsed_secs > x_max (i.e. one past the last visible).
        let end_a = a.partition_point(|p| self.elapsed_secs(p.time) <= x_max);
        let end = if end_a < a_len {
            end_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(p.time) <= x_max)
        };

        (start, end)
    }

    /// Why the line breaks between `prev` and `point`, or `None` if it
    /// doesn't. A recorded break wins over the elapsed-time test: an overload
    /// that lasted less than the gap threshold is still an overload, not a
    /// dropout.
    fn breaks_before(&self, prev: Instant, point: &DataPoint) -> Option<GapKind> {
        if let Some(kind) = point.break_before {
            return Some(kind);
        }
        let elapsed = point
            .time
            .checked_duration_since(prev)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        (elapsed > self.gap_threshold_secs).then_some(GapKind::NoData)
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
        let o = self
            .overlays
            .iter()
            .find(|o| o.label == label)
            .unwrap_or_else(|| panic!("no overlay {label:?}"));
        self.build_overlay_segments_for_range(o, 0, self.history.len())
    }

    #[cfg(test)]
    fn overlay_values(&self, label: &str) -> Vec<Option<f64>> {
        self.overlays
            .iter()
            .find(|o| o.label == label)
            .unwrap_or_else(|| panic!("no overlay {label:?}"))
            .values
            .iter()
            .copied()
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
