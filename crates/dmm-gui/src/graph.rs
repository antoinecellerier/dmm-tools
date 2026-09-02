use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{
    AxisHints, HLine, Line, Plot, PlotBounds, PlotPoints, PlotTransform, Points, Span, VLine,
};
use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::a11y::ResponseA11yExt;
use crate::settings::{ColorOverrides, ColorPreset};
use crate::theme::ThemeColors;

/// Maximum number of points to keep in the history buffer.
const MAX_POINTS: usize = 10_000;

/// Maximum number of same-unit sub-values drawn beside the plotted series.
///
/// The protocols send at most four sub-values per frame (UT181A), so this is
/// the ceiling the wire imposes rather than a display choice.
pub(crate) const MAX_OVERLAYS: usize = 4;

/// Default gap threshold multiplier: gap = max(interval * multiplier, minimum).
const GAP_MULTIPLIER: f64 = 5.0;
const GAP_MINIMUM_SECS: f64 = 1.0;

/// Minimap height in logical pixels.
const MINIMAP_HEIGHT: f32 = 60.0;

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

/// One sub-value trace ready to draw: (overlay index, name, segments).
///
/// The index — not a colour — because it is what both the trace and its key
/// row derive their colour and line style from, so the two cannot drift.
type OverlayTrace = (usize, String, Vec<Vec<[f64; 2]>>);

/// How one row of the plot key draws its line sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyStyle {
    /// The plotted series: solid, in the graph line colour.
    Plotted,
    /// A sub-value trace, identified by its overlay index — the same index
    /// `overlay_color_and_style` uses for the line itself.
    Overlay(usize),
}

/// Minimum font size for the plot key, per `.claude/rules/gui.md`. The body
/// text style is normally larger; this only bites if a user's style shrinks it.
const MIN_KEY_FONT_SIZE: f32 = 11.0;

/// Width of the line sample drawn at the left of each key row.
const KEY_SAMPLE_WIDTH: f32 = 24.0;

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

/// Shapes for one key row's line sample, matching how `egui_plot` renders the
/// same `LineStyle` on the plot — otherwise the key would promise a dash
/// pattern the trace doesn't use.
fn key_line_sample(
    a: egui::Pos2,
    b: egui::Pos2,
    color: egui::Color32,
    style: egui_plot::LineStyle,
) -> Vec<egui::Shape> {
    const WIDTH: f32 = 1.5;
    match style {
        egui_plot::LineStyle::Solid => {
            vec![egui::Shape::line_segment(
                [a, b],
                egui::Stroke::new(WIDTH, color),
            )]
        }
        egui_plot::LineStyle::Dotted { spacing } => {
            egui::Shape::dotted_line(&[a, b], color, spacing, WIDTH)
        }
        egui_plot::LineStyle::Dashed { length } => {
            // The same golden-ratio gap `egui_plot::LineStyle::style_line` uses.
            const GOLDEN_RATIO: f32 = 0.618_034;
            egui::Shape::dashed_line(
                &[a, b],
                egui::Stroke::new(WIDTH, color),
                length,
                length * GOLDEN_RATIO,
            )
        }
    }
}

/// Quantize a `f64` to ~3 decimal digits before hashing so animated plot
/// transforms (which produce sub-pixel jitter on the y bounds) don't
/// invalidate label caches every frame. Returns an `i64` so the result is
/// `Hash`-stable and not affected by `f64::NaN` weirdness.
fn quantize_for_hash(v: f64) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v * 1000.0).round() as i64
    }
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

/// Choose a nice round interval for time axis labels.
fn nice_time_interval(span: f64) -> f64 {
    let target_ticks = 6.0;
    let raw = span / target_ticks;
    let nice_values = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    for &v in &nice_values {
        if v >= raw {
            return v;
        }
    }
    raw.ceil()
}

/// Format a time value in seconds as a readable label.
fn format_time_label(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        let m = (secs / 60.0).floor() as u32;
        let s = (secs % 60.0).floor() as u32;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m{s:02}s")
        }
    } else {
        let h = (secs / 3600.0).floor() as u32;
        let m = ((secs % 3600.0) / 60.0).floor() as u32;
        format!("{h}h{m:02}m")
    }
}

/// Format a grid-mark value for the main graph's X axis, adding decimals to
/// the seconds field when the grid step is sub-second. Without this, a tight
/// zoom (e.g. a 0.5 s span) produces duplicate labels like "9 s" / "9 s"
/// because integer seconds can't distinguish adjacent gridlines.
fn format_time_axis_label(value: f64, step_size: f64) -> String {
    // When step is a sub-second power of 10 (egui_plot default log-10
    // spacer), show enough decimals to resolve adjacent marks. clamp to 1
    // so a rounding step like 0.5 (non-power-of-10) still gets at least
    // one decimal of precision.
    let sec_decimals: usize = if step_size > 0.0 && step_size < 1.0 {
        ((-step_size.log10()).round() as i64).clamp(1, 6) as usize
    } else {
        0
    };

    let s = value;
    if s < 60.0 {
        format!("{s:.sec_decimals$} s")
    } else if s < 3600.0 {
        let m = (s / 60.0).floor();
        let sec = s - m * 60.0;
        if sec_decimals == 0 && sec.abs() < 0.5 {
            format!("{m:.0} m")
        } else {
            format!("{m:.0}m {sec:.sec_decimals$}s")
        }
    } else {
        let h = (s / 3600.0).floor();
        let rem = s - h * 3600.0;
        let m = (rem / 60.0).floor();
        let sec = rem - m * 60.0;
        if sec_decimals > 0 {
            format!("{h:.0}h {m:.0}m {sec:.sec_decimals$}s")
        } else {
            format!("{h:.0}h {m:.0}m")
        }
    }
}

/// Tracks which part of the minimap the user is dragging.
#[derive(Default, Clone, Copy, PartialEq)]
enum MinimapDrag {
    #[default]
    None,
    Pan,
    ResizeLeft,
    ResizeRight,
}

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
    /// Color preset for graph rendering.
    color_preset: ColorPreset,
    /// Per-theme color overrides for graph rendering.
    color_overrides: ColorOverrides,
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

/// Pre-computed data needed by `paint_overlay_labels` to draw text labels
/// for mean, reference, and cursor overlays after the plot has been rendered.
struct OverlayLabelData {
    show_mean: bool,
    mean_value: Option<f64>,
    show_ref: bool,
    ref_values: Vec<f64>,
    cursors_active: bool,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    cursor_va: Option<f64>,
    cursor_vb: Option<f64>,
    overlay_unit: String,
    view_max: f64,
    mean_color: egui::Color32,
    ref_color: egui::Color32,
    cursor_color: egui::Color32,
}

/// Faint container for one labelled toolbar group (**Plot:**, **Show:**).
///
/// The two groups do different things — one picks the series the graph plots,
/// the other picks which sub-value traces are drawn over it — and inline chips
/// gave a first-time user nothing to tell them apart, or apart from the
/// analysis toggles beside them. Boxing each group makes that grouping
/// visible.
///
/// Both colours come from `Visuals`, so the box follows the theme. The fill is
/// `faint_bg_color`, well clear of `selection.bg_fill`: the frame is only a
/// container, and a selected chip inside it must still read as selected.
fn group_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::new()
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
}

/// Caption naming one toolbar group. Body size, not `.small()` — egui's small
/// style lands under the 11 pt floor in `.claude/rules/gui.md`, and these are
/// group labels rather than annotations.
fn group_caption(ui: &Ui, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .strong()
        .color(ui.visuals().weak_text_color())
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
            color_preset: ColorPreset::Default,
            color_overrides: ColorOverrides::default(),
            minimap_drag: MinimapDrag::None,
            bbox_zoom_start_px: None,
            bbox_zoom_current_px: None,
            a11y_label: String::new(),
            a11y_label_sig: 0,
        }
    }

    /// Update color configuration from settings.
    pub fn set_color_config(&mut self, preset: ColorPreset, overrides: ColorOverrides) {
        self.color_preset = preset;
        self.color_overrides = overrides;
    }

    /// Build a ThemeColors instance using this graph's color config.
    fn theme_colors(&self, dark: bool) -> ThemeColors {
        ThemeColors::new(dark, self.color_preset, self.color_overrides.for_mode(dark))
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
    /// A selection whose label is no longer offered falls back to the main
    /// reading — the meter left the mode that produced it.
    pub fn set_series_options(&mut self, options: &[(&str, &str)]) {
        if let Some(sel) = &self.selected_series
            && !options.iter().any(|(label, _)| *label == sel.as_str())
        {
            self.selected_series = None;
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

    /// Handle keyboard shortcuts for graph navigation.
    ///
    /// Minimap pan keys are NOT handled here — they live in `show_minimap`
    /// instead, gated on `pointer_response.has_focus()`. Doing the focus
    /// check + `move_focus(FocusDirection::None)` reset *after*
    /// `interested_in_focus` has already run on the pointer response is
    /// what lets Tab / Shift+Tab still escape the minimap. If we did the
    /// reset here at the top of the frame, it would also wipe egui's
    /// `Focus::begin_pass` snapshot of Tab into `focus_direction = Next`
    /// (egui treats Tab and arrow keys with the same field), trapping
    /// keyboard focus on the minimap.
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        // Only when no widget has focus (preserves the existing behaviour
        // that Tab-navigating to a button doesn't steal arrow/Home/End
        // keys from the graph's pan/jump bindings).
        if ctx.egui_wants_keyboard_input() {
            return;
        }

        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::OpenBracket)) {
            self.cycle_time_window(-1);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::CloseBracket)) {
            self.cycle_time_window(1);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft)) {
            self.scroll_view(-0.25);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight)) {
            self.scroll_view(0.25);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Home)) {
            self.jump_to_start();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::End)) {
            self.live = true;
        }
    }

    /// Cycle through TIME_WINDOWS presets. `direction`: -1 = shorter, +1 = longer.
    fn cycle_time_window(&mut self, direction: i32) {
        if direction < 0 {
            if let Some(&(secs, _)) = TIME_WINDOWS
                .iter()
                .rev()
                .find(|&&(s, _)| s < self.time_window_secs - 0.1)
            {
                self.time_window_secs = secs;
            }
        } else if let Some(&(secs, _)) = TIME_WINDOWS
            .iter()
            .find(|&&(s, _)| s > self.time_window_secs + 0.1)
        {
            self.time_window_secs = secs;
        }
    }

    /// Scroll the view by a fraction of the current window width.
    fn scroll_view(&mut self, fraction: f64) {
        let delta = self.time_window_secs * fraction;
        let (data_min, data_max) = self.data_time_range();
        let half = self.time_window_secs / 2.0;

        if self.live {
            self.view_center = data_max - half;
            self.live = false;
        }

        self.view_center = (self.view_center + delta).max(data_min + half);

        if self.view_center + half >= data_max {
            self.live = true;
        }
    }

    /// Jump view to the start of recorded data.
    fn jump_to_start(&mut self) {
        let (data_min, _) = self.data_time_range();
        self.view_center = data_min + self.time_window_secs / 2.0;
        self.live = false;
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

    fn data_time_range(&self) -> (f64, f64) {
        let x_min = self
            .history
            .front()
            .map(|p| self.elapsed_secs(p.time))
            .unwrap_or(0.0);
        let x_max = self
            .history
            .back()
            .map(|p| self.elapsed_secs(p.time))
            .unwrap_or(0.0);
        // Overload samples carry timestamps but no plottable value, so they
        // never enter `history` — yet they are the newest thing the meter
        // sent. Extending the range here rather than at each call site means
        // every consumer follows them: the live window, the minimap's time
        // mapping and its drag clamping. Otherwise time visibly freezes for
        // the duration of the excursion, which is exactly when the band being
        // drawn needs somewhere to grow into.
        let x_max = match self.pending_break_since {
            Some(t) => x_max.max(self.elapsed_secs(t)),
            None => x_max,
        };
        (x_min, x_max)
    }

    /// Current view bounds (x_min, x_max) for the main graph.
    fn view_bounds(&self) -> (f64, f64) {
        let (_, data_max) = self.data_time_range();
        let half = self.time_window_secs / 2.0;

        if self.live {
            // "Live" means the window ends at the newest *sample*, and
            // `data_time_range` counts overload samples as such — so the
            // window keeps advancing through an excursion rather than
            // freezing at the last plotted point. A paused or disconnected
            // meter sends nothing at all, so its window still holds still.
            let x_max = data_max;
            let x_min = (x_max - self.time_window_secs).max(0.0);
            (x_min, x_max)
        } else {
            let x_min = (self.view_center - half).max(0.0);
            let x_max = x_min + self.time_window_secs;
            (x_min, x_max)
        }
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

    /// Whether the toolbar's series row has anything to show: a sub-value to
    /// pick from (**Plot:**) or a trace to draw beside the plotted series
    /// (**Show:**). False for single-display meters, which keep the two-row
    /// toolbar they had before sub-values existed.
    fn has_series_row(&self) -> bool {
        !self.series_options.is_empty() || !self.overlays.is_empty()
    }

    /// Render the toolbar. Row 1: time presets, LIVE, Y-axis, Reset Zoom.
    /// Row 2, present only for meters that send sub-values: the boxed
    /// **Plot:** series selector and **Show:** trace toggles. Row 3: the
    /// analysis overlays (Mean, Min/Max, Ref, Cursors).
    ///
    /// The series controls get a row of their own rather than sharing one
    /// with the analysis toggles: inline, nothing distinguished the two kinds
    /// of control for a user who didn't already know them. The rows now read
    /// view → what is plotted → what is drawn over it. Each uses
    /// `horizontal_wrapped` so items wrap instead of clipping.
    pub fn show_toolbar(&mut self, ui: &mut Ui) {
        // Row 1: time windows + LIVE + Y-axis
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for &(secs, label) in TIME_WINDOWS {
                let tooltip = format!("Show the last {label} of samples ([ / ] to cycle)");
                if ui
                    .selectable_label((self.time_window_secs - secs).abs() < 0.1, label)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.time_window_secs = secs;
                }
            }

            ui.add_space(6.0);

            let live_color = if self.live {
                self.theme_colors(ui.visuals().dark_mode).live_green()
            } else {
                ui.visuals().weak_text_color()
            };
            let live_resp = ui
                .add(egui::Button::new(
                    egui::RichText::new("LIVE").color(live_color).small(),
                ))
                .on_hover_text(
                    "Auto-follow the newest samples — off while panning (End to jump back)",
                )
                .a11y_toggled(self.live);
            if live_resp.clicked() {
                self.live = !self.live;
            }

            ui.add_space(6.0);

            let (y_label, y_tooltip) = if self.y_axis_fixed {
                (
                    "Y:Fixed",
                    "Using fixed Y-axis bounds — click to auto-scale to visible data",
                )
            } else {
                (
                    "Y:Auto",
                    "Auto-scaling Y to visible data — click to enter fixed bounds",
                )
            };
            if ui
                .selectable_label(self.y_axis_fixed, y_label)
                .on_hover_text(y_tooltip)
                .clicked()
            {
                if !self.y_axis_fixed && !self.y_user_set {
                    let (view_min, view_max) = self.view_bounds();
                    // Snapshot with the overlays included, so pinning the axis
                    // doesn't jump the view the moment Y:Fixed is pressed.
                    if let Some((y_lo, y_hi)) = self.y_range_for_view_auto(view_min, view_max, true)
                    {
                        self.y_fixed_min = y_lo;
                        self.y_fixed_max = y_hi;
                        self.y_min_text = format!("{y_lo:.4}");
                        self.y_max_text = format!("{y_hi:.4}");
                    }
                }
                self.y_axis_fixed = !self.y_axis_fixed;
            }
            if self.y_axis_fixed {
                let field_width = 50.0;
                let changed_min = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.y_min_text)
                            .desired_width(field_width)
                            .hint_text("Y axis minimum"),
                    )
                    .on_hover_text("Lower bound of the fixed Y axis")
                    .changed();
                ui.label(
                    egui::RichText::new("..")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                let changed_max = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.y_max_text)
                            .desired_width(field_width)
                            .hint_text("Y axis maximum"),
                    )
                    .on_hover_text("Upper bound of the fixed Y axis")
                    .changed();
                if changed_min && let Ok(v) = self.y_min_text.parse::<f64>() {
                    self.y_fixed_min = v;
                    self.y_user_set = true;
                }
                if changed_max && let Ok(v) = self.y_max_text.parse::<f64>() {
                    self.y_fixed_max = v;
                    self.y_user_set = true;
                }
            }

            ui.add_space(6.0);

            let zoomed = self.is_view_zoomed();
            if ui
                .add_enabled(zoomed, egui::Button::new("Reset Zoom"))
                .on_hover_text("Return to live follow and auto Y (double-click graph)")
                .on_disabled_hover_text("Already at the default live + auto-Y view")
                .clicked()
            {
                self.reset_view();
            }
        });

        // Row 2: what is plotted, and what is drawn beside it. Skipped
        // entirely for single-display meters, which keep the two-row toolbar.
        if self.has_series_row() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                self.show_series_selector(ui);
                self.show_overlay_toggles(ui);
            });
        }

        // Row 3: analysis overlays
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let dark = ui.visuals().dark_mode;

            if ui
                .selectable_label(self.show_mean, "Mean")
                .on_hover_text("Draw a horizontal line at the mean of visible samples")
                .clicked()
            {
                self.show_mean = !self.show_mean;
            }
            if ui
                .selectable_label(self.show_envelope, "Min/Max")
                .on_hover_text("Draw a shaded band between the rolling min and max")
                .clicked()
            {
                self.show_envelope = !self.show_envelope;
            }
            if self.show_envelope {
                let changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.envelope_window_text)
                            .desired_width(30.0)
                            .hint_text("Min/Max window, seconds"),
                    )
                    .on_hover_text("Window size (seconds) used to compute the Min/Max envelope")
                    .changed();
                if changed
                    && let Ok(v) = self.envelope_window_text.parse::<f64>()
                    && v > 0.0
                {
                    self.envelope_window_secs = v;
                }
                ui.label(
                    egui::RichText::new("s")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if ui
                .selectable_label(self.show_ref_line, "Ref")
                .on_hover_text("Draw horizontal reference lines at the values in the next field")
                .clicked()
            {
                self.show_ref_line = !self.show_ref_line;
            }
            if self.show_ref_line {
                let changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.ref_line_text)
                            .desired_width(80.0)
                            .hint_text("Reference values"),
                    )
                    .on_hover_text(
                        "Reference values, comma- or semicolon-separated (e.g. 3.3, 5, 12)",
                    )
                    .changed();
                if changed {
                    self.ref_line_values = self
                        .ref_line_text
                        .split([',', ';', ' '])
                        .filter_map(|s| s.trim().parse::<f64>().ok())
                        .collect();
                }
                if ui
                    .selectable_label(self.show_crossings, "Triggers")
                    .on_hover_text("Mark the points where the signal crosses a reference line")
                    .clicked()
                {
                    self.show_crossings = !self.show_crossings;
                }
            }
            if ui
                .selectable_label(self.cursors_active, "Cursors")
                .on_hover_text("Click the graph to place two cursors and read Δt / Δv / integral")
                .clicked()
            {
                self.cursors_active = !self.cursors_active;
                if !self.cursors_active {
                    self.cursor_a = None;
                    self.cursor_b = None;
                    self.cursor_next_is_b = false;
                }
            }
            if self.cursors_active {
                if let (Some(ta), Some(tb)) = (self.cursor_a, self.cursor_b) {
                    let dt = (tb - ta).abs();
                    let va = self.nearest_point(ta).map(|(_, v)| v);
                    let vb = self.nearest_point(tb).map(|(_, v)| v);
                    let dv = match (va, vb) {
                        (Some(a), Some(b)) => format!("{:.4}", (b - a).abs()),
                        _ => crate::NO_DATA.to_string(),
                    };
                    let unit = &self.current_unit;
                    let delta_color = self.theme_colors(dark).graph_cursor_delta();

                    let integral_str = dmm_lib::stats::integral_unit_info(unit)
                        .and_then(|(disp_unit, divisor)| {
                            self.cursor_integral(ta, tb)
                                .map(|raw| format!("  \u{222b}={:.4} {disp_unit}", raw / divisor))
                        })
                        .unwrap_or_default();

                    ui.label(
                        egui::RichText::new(format!(
                            "\u{0394}T={dt:.2} s  \u{0394}={dv} {unit}{integral_str}"
                        ))
                        .color(delta_color)
                        .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("click graph to place cursors")
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            }
        });
    }

    /// Chips choosing which series the graph plots: the meter's main reading
    /// or any sub-value it is currently sending.
    ///
    /// Only rendered when the meter sends sub-values, so single-display
    /// meters see the toolbar exactly as before. `selectable_label` rather
    /// than a combo box because that is the toolbar's idiom throughout — and
    /// with at most five choices the chips stay readable.
    fn show_series_selector(&mut self, ui: &mut Ui) {
        if self.series_options.is_empty() {
            return;
        }

        // `None` = the main reading, `Some(i)` = `series_options[i]`. Applied
        // after the loop so the click handler doesn't need a mutable borrow
        // while the option list is being read.
        let mut choice: Option<Option<usize>> = None;

        let frame = group_frame(ui);
        frame.show(ui, |ui| {
            ui.label(group_caption(ui, "Plot:"));

            let main_selected = self.selected_series.is_none();
            if ui
                .selectable_label(main_selected, "Main")
                .on_hover_text("Plot the meter's main reading")
                .a11y_toggled(main_selected)
                .clicked()
            {
                choice = Some(None);
            }

            for (i, (label, unit)) in self.series_options.iter().enumerate() {
                let selected = self.selected_series.as_deref() == Some(label.as_str());
                if ui
                    .selectable_label(selected, label.as_str())
                    .on_hover_text(format!(
                        "Plot the {label} sub-value ({unit}) \u{2014} a different unit restarts the graph"
                    ))
                    .a11y_toggled(selected)
                    .clicked()
                {
                    choice = Some(Some(i));
                }
            }
        });

        // The switch takes effect at the next sample: `push_sample` sees the
        // new series and clears through the same branch a mode change uses,
        // releasing the pinned Y range, the cursors and any bbox state.
        match choice {
            Some(None) => self.selected_series = None,
            Some(Some(i)) => self.selected_series = Some(self.series_options[i].0.clone()),
            None => {}
        }

        ui.add_space(6.0);
    }

    /// Chips choosing which same-unit sub-value traces are drawn beside the
    /// plotted series.
    ///
    /// The plot key is a key, not a control (`Plot::reset()` wipes egui_plot's
    /// own legend state every frame), so the show/hide affordance lives here
    /// in the toolbar with the rest of them. Hiding a trace stops it being
    /// drawn but not recorded — turning it back on brings its history with it.
    fn show_overlay_toggles(&mut self, ui: &mut Ui) {
        if self.overlays.is_empty() {
            return;
        }

        // Applied after the loop: the click handler would otherwise need a
        // mutable borrow while the overlay list is being read.
        let mut toggled: Option<String> = None;

        let frame = group_frame(ui);
        frame.show(ui, |ui| {
            ui.label(group_caption(ui, "Show:"));

            for o in &self.overlays {
                let label = o.label.as_str();
                let shown = !self.hidden_overlays.contains(&o.label);
                let hover = if shown {
                    format!("Hide the {label} trace")
                } else {
                    format!("Draw the {label} trace beside the plotted series")
                };
                if ui
                    .selectable_label(shown, label)
                    .on_hover_text(hover)
                    .a11y_toggled(shown)
                    .clicked()
                {
                    toggled = Some(o.label.clone());
                }
            }
        });

        if let Some(label) = toggled {
            self.toggle_overlay_hidden(label);
        }

        ui.add_space(6.0);
    }

    /// Flip one sub-value trace between drawn and hidden.
    fn toggle_overlay_hidden(&mut self, label: String) {
        if !self.hidden_overlays.remove(&label) {
            self.hidden_overlays.insert(label);
        }
    }

    /// Compute min/max Y over the visible slice, with 10% padding.
    ///
    /// `with_overlays` includes the sub-value traces drawn beside the plotted
    /// series, so they are framed rather than clipped. The minimap passes
    /// `false`: it is a main-series overview, and its scan already covers the
    /// whole history — multiplying it by the overlay count is the O(n) budget
    /// the two-tier rendering contract exists to protect.
    fn y_min_max_padded(&self, x_min: f64, x_max: f64, with_overlays: bool) -> Option<(f64, f64)> {
        let (start, end) = self.visible_index_range(x_min, x_max);
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for i in start..end {
            let v = self.history[i].value;
            y_min = y_min.min(v);
            y_max = y_max.max(v);
        }
        if with_overlays {
            // Hidden overlays are excluded: an axis stretched to frame a trace
            // the user switched off would flatten the one they are looking at.
            for o in self.shown_overlays().map(|(_, o)| o) {
                for i in start..end {
                    if let Some(v) = o.values.get(i).copied().flatten() {
                        y_min = y_min.min(v);
                        y_max = y_max.max(v);
                    }
                }
            }
        }
        if y_min.is_infinite() {
            return None;
        }
        let range = (y_max - y_min).max(1e-6);
        let pad = range * 0.1;
        Some((y_min - pad, y_max + pad))
    }

    /// Auto-scaled Y range (ignoring fixed mode setting). Used to snapshot
    /// current auto range when switching to fixed mode.
    fn y_range_for_view_auto(
        &self,
        x_min: f64,
        x_max: f64,
        with_overlays: bool,
    ) -> Option<(f64, f64)> {
        self.y_min_max_padded(x_min, x_max, with_overlays)
    }

    /// Compute Y range from data points visible in the given X range, with padding.
    fn y_range_for_view(&self, x_min: f64, x_max: f64, with_overlays: bool) -> Option<(f64, f64)> {
        if self.y_axis_fixed {
            return Some((self.y_fixed_min, self.y_fixed_max));
        }
        self.y_min_max_padded(x_min, x_max, with_overlays)
    }

    /// Build segment and gap data for a slice of history, suitable for
    /// passing to egui_plot. Only the points in `[start, end)` are visited.
    fn build_segments_for_range(&self, start: usize, end: usize) -> SegmentsAndGaps {
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut gaps: Vec<(f64, f64, GapKind)> = Vec::new();
        let mut current_segment: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);

            if let Some(prev) = prev_time
                && let Some(kind) = self.breaks_before(prev, point)
                && !current_segment.is_empty()
            {
                let gap_start = self.elapsed_secs(prev);
                // An interruption can be two things end to end: a stretch the
                // meter reported on, then a stretch it didn't. Losing the link
                // mid-overload is exactly that, and folding the silence into
                // the band would claim the meter was over range for a period
                // it never reported at all.
                match (kind, point.break_last_sample) {
                    (GapKind::Overload, Some(last)) if point.break_had_data_loss => {
                        let heard_until = self.elapsed_secs(last);
                        gaps.push((gap_start, heard_until, GapKind::Overload));
                        gaps.push((heard_until, t, GapKind::NoData));
                    }
                    _ => gaps.push((gap_start, t, kind)),
                }
                segments.push(std::mem::take(&mut current_segment));
            }

            current_segment.push([t, point.value]);
            prev_time = Some(point.time);
        }

        if !current_segment.is_empty() {
            segments.push(current_segment);
        }

        (segments, gaps)
    }

    /// Build the drawable segments of one overlay over `[start, end)`.
    ///
    /// Deliberately separate from `build_segments_for_range` rather than
    /// generalising it: that one also derives the gap ranges the bands and
    /// dropout markers are drawn from, and its data-loss split is subtle and
    /// tested. Overlays draw no bands and no markers — they only need to stop
    /// wherever the main trace stops (time gap, overload, data loss) plus
    /// wherever the sub-value itself is missing.
    fn build_overlay_segments_for_range(
        &self,
        o: &OverlaySeries,
        start: usize,
        end: usize,
    ) -> Vec<Vec<[f64; 2]>> {
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for i in start..end {
            let point = &self.history[i];
            if let Some(prev) = prev_time
                && self.breaks_before(prev, point).is_some()
                && !current.is_empty()
            {
                segments.push(std::mem::take(&mut current));
            }
            prev_time = Some(point.time);

            match o.values.get(i).copied().flatten() {
                Some(v) => current.push([self.elapsed_secs(point.time), v]),
                None if !current.is_empty() => segments.push(std::mem::take(&mut current)),
                None => {}
            }
        }

        if !current.is_empty() {
            segments.push(current);
        }
        segments
    }

    /// The overlays the user has not switched off, paired with their index.
    ///
    /// The index is the overlay's position in `self.overlays`, which is what
    /// keys its colour and line style — stable for the life of the session, so
    /// hiding one does not reshuffle the palette of the others.
    fn shown_overlays(&self) -> impl Iterator<Item = (usize, &OverlaySeries)> {
        self.overlays
            .iter()
            .enumerate()
            .filter(|(_, o)| !self.hidden_overlays.contains(&o.label))
    }

    /// The sub-value traces actually drawn over `[start, end)`.
    ///
    /// Leaves out the ones the user switched off, and the ones with no points
    /// in this window — an overlay that isn't drawn must not appear in the key
    /// either.
    fn visible_overlay_traces(&self, start: usize, end: usize) -> Vec<OverlayTrace> {
        self.shown_overlays()
            .filter_map(|(k, o)| {
                let segments = self.build_overlay_segments_for_range(o, start, end);
                (!segments.is_empty()).then(|| (k, o.label.clone(), segments))
            })
            .collect()
    }

    /// Colour and line style of one overlay trace.
    ///
    /// Keyed on the overlay's index — never on a hash of its label, which
    /// would reshuffle the palette when a sub-value appears or disappears
    /// mid-capture. The line style carries the same information without
    /// colour, as `.claude/rules/gui.md` requires.
    fn overlay_color_and_style(
        tc: &ThemeColors,
        index: usize,
    ) -> (egui::Color32, egui_plot::LineStyle) {
        let style = match index % 4 {
            0 => egui_plot::LineStyle::dashed_loose(),
            1 => egui_plot::LineStyle::dotted_loose(),
            2 => egui_plot::LineStyle::dashed_dense(),
            _ => egui_plot::LineStyle::dotted_dense(),
        };
        (tc.graph_overlay(index), style)
    }

    /// Name the plotted series goes by on the plot and in the key.
    fn plotted_series_name(&self) -> String {
        self.current_series
            .clone()
            .unwrap_or_else(|| "Main".to_string())
    }

    /// Rows of the plot key: the plotted series, then each drawn overlay in
    /// `self.overlays` order.
    ///
    /// Empty when nothing is overlaid — a single-series graph gets no key, and
    /// looks exactly as it did before sub-values existed. Takes the traces
    /// `show_main` is about to draw rather than recomputing them, so the key
    /// cannot list a line that isn't there.
    fn key_entries(&self, drawn: &[OverlayTrace]) -> Vec<(String, KeyStyle)> {
        if drawn.is_empty() {
            return Vec::new();
        }
        let mut entries = Vec::with_capacity(drawn.len() + 1);
        entries.push((self.plotted_series_name(), KeyStyle::Plotted));
        entries.extend(
            drawn
                .iter()
                .map(|(k, label, _)| (label.clone(), KeyStyle::Overlay(*k))),
        );
        entries
    }

    /// Paint the plot key in the top-left of the plot area.
    ///
    /// A key, not a control: egui_plot's own `Legend` renders show/hide
    /// checkboxes, but `Plot::reset()` — which pins the view every frame while
    /// keeping pointer events — also clears the plot's `hidden_items`, so a
    /// click on one had no effect past the frame it happened in. The **Show:**
    /// chips in the toolbar are the control instead.
    fn paint_plot_key(
        ui: &Ui,
        plot_rect: egui::Rect,
        entries: &[(String, KeyStyle)],
        tc: &ThemeColors,
    ) {
        if entries.is_empty() {
            return;
        }
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = font.size.max(MIN_KEY_FONT_SIZE);

        const PAD: f32 = 6.0;
        const GAP: f32 = 6.0;
        let text_color = ui.visuals().text_color();
        let painter = ui.painter_at(plot_rect);

        // Measure first so the panel is exactly as wide as its widest row.
        let galleys: Vec<_> = entries
            .iter()
            .map(|(name, _)| painter.layout_no_wrap(name.clone(), font.clone(), text_color))
            .collect();
        let text_width = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max);
        let row_height = galleys.iter().map(|g| g.size().y).fold(font.size, f32::max);
        let width = PAD * 2.0 + KEY_SAMPLE_WIDTH + GAP + text_width;
        let height = PAD * 2.0 + row_height * entries.len() as f32;
        let rect = egui::Rect::from_min_size(
            plot_rect.left_top() + egui::vec2(PAD, PAD),
            egui::vec2(width, height),
        );
        // `window_fill`, not `extreme_bg_color`: the App assigns the latter
        // the plot's own background, so a key painted in it would be invisible
        // apart from its border. `window_fill` is the panel ground either
        // theme pairs with `text_color`, so the names keep their normal
        // contrast, and 85% alpha keeps the grid faintly readable underneath
        // instead of punching a hole in the plot.
        painter.rect_filled(rect, 4.0, ui.visuals().window_fill.gamma_multiply(0.85));
        painter.rect_stroke(
            rect,
            4.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );

        for (i, ((_, style), galley)) in entries.iter().zip(&galleys).enumerate() {
            let top = rect.top() + PAD + row_height * i as f32;
            let mid = top + row_height / 2.0;
            let (color, line_style) = match style {
                KeyStyle::Plotted => (tc.graph_line(), egui_plot::LineStyle::Solid),
                KeyStyle::Overlay(k) => Self::overlay_color_and_style(tc, *k),
            };
            painter.extend(key_line_sample(
                egui::pos2(rect.left() + PAD, mid),
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH, mid),
                color,
                line_style,
            ));
            painter.galley(
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH + GAP, top),
                galley.clone(),
                text_color,
            );
        }
    }

    /// Render the main graph.
    pub fn show_main(&mut self, ui: &mut Ui) {
        let (view_min, view_max) = self.view_bounds();

        // Build segments and gaps for the visible slice only, plus one
        // point on each side so line segments at the view edges render
        // correctly and aren't clipped to nothing.
        let (vis_start, vis_end) = self.visible_index_range(view_min, view_max);
        let ext_start = vis_start.saturating_sub(1);
        let ext_end = (vis_end + 1).min(self.history.len());
        let (visible_segments, visible_gaps) = self.build_segments_for_range(ext_start, ext_end);
        // Overload spans, including one still in progress. Used both to draw
        // the bands and to answer the crosshair tooltip, which is the only
        // non-visual cue available — `Span` is never a hover target.
        let overload_spans: Vec<(f64, f64)> = visible_gaps
            .iter()
            .filter(|(_, _, kind)| *kind == GapKind::Overload)
            .map(|&(a, b, _)| (a, b))
            .chain(self.pending_overload_span())
            .collect();

        // Theme-aware colors from shared palette
        let tc = self.theme_colors(ui.visuals().dark_mode);
        let line_color = tc.graph_line();
        let gap_color = tc.graph_gap();
        let overload_edge = tc.graph_overload();
        let overload_fill = tc.graph_overload_fill();
        let mean_color = tc.graph_mean();
        let ref_color = tc.graph_ref();
        let cross_color = tc.graph_crossing();
        let cursor_color = tc.graph_cursor();
        let cursor_color_dim = tc.graph_cursor_dim();
        let env_color = tc.graph_envelope();

        // Sub-value traces over the same visible slice as the main series,
        // minus any the user switched off in the toolbar's Show: group.
        let overlay_traces = self.visible_overlay_traces(ext_start, ext_end);
        let key_entries = self.key_entries(&overlay_traces);
        let multi_series = !overlay_traces.is_empty();
        // Every drawn line carries the name of its series, which is what the
        // hover label reports; every helper item is named "" and falls through
        // to the plain form.
        let main_name = self.plotted_series_name();

        let can_interact = !self.live;
        let shift_held = ui.input(|i| i.modifiers.shift);
        let bbox_active = self.bbox_zoom_start_px.is_some();
        // Plain drag-to-pan is allowed even in live mode — starting a drag
        // drops out of live (see handle_interaction). Scroll-zoom stays gated
        // on !live: the first scroll exits live without zooming, the second
        // zooms. Bbox and shift-drag always suppress the built-in pan.
        let allow_plot_x_drag = !shift_held && !bbox_active;
        let allow_plot_x_zoom = can_interact && !bbox_active;

        // Compute Y bounds from visible data
        let (y_min, y_max) = self
            .y_range_for_view(view_min, view_max, true)
            .unwrap_or((-1.0, 1.0));

        let unit = self.current_unit.clone();
        let y_axis = AxisHints::new_y().formatter(move |mark, _range| {
            let decimals = (-mark.step_size.log10().round() as usize).min(6);
            let val = eframe::emath::format_with_decimals_in_range(mark.value, decimals..=decimals);
            if unit.is_empty() {
                val
            } else {
                format!("{val} {unit}  ")
            }
        });

        let x_axis = AxisHints::new_x()
            .formatter(|mark, _range| format_time_axis_label(mark.value, mark.step_size));

        let show_envelope = self.show_envelope;
        let (env_min, env_max) = if show_envelope {
            self.build_envelope(view_min, view_max, self.envelope_window_secs)
        } else {
            (Vec::new(), Vec::new())
        };
        let show_mean = self.show_mean;
        let show_ref = self.show_ref_line;
        let ref_values = self.ref_line_values.clone();
        let show_crossings = self.show_crossings;
        let crossings = if show_ref && show_crossings && !ref_values.is_empty() {
            self.find_crossings(&ref_values, view_min, view_max)
        } else {
            Vec::new()
        };
        let cursors_active = self.cursors_active;
        let cursor_a = self.cursor_a;
        let cursor_b = self.cursor_b;
        let cursor_va = cursor_a.and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let cursor_vb = cursor_b.and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let visible_stats = self.visible_stats();
        let mean_value = visible_stats.map(|(_, _, avg, _)| avg);

        let cursor_unit = self.current_unit.clone();
        // Moved into the label_formatter closure, which is rebuilt each frame.
        let tooltip_spans = overload_spans.clone();
        let plot = Plot::new("main_plot")
            .height(ui.available_height().max(60.0))
            .allow_drag(Vec2b::new(allow_plot_x_drag, false))
            .allow_zoom(Vec2b::new(allow_plot_x_zoom, false))
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .reset()
            .custom_x_axes(vec![x_axis])
            .custom_y_axes(vec![y_axis])
            .y_axis_min_width(60.0)
            .cursor_color(tc.graph_crosshair())
            .label_formatter(move |name, point| {
                let t = point.x;
                let time_label = if t < 60.0 {
                    format!("{t:.1} s")
                } else {
                    let m = (t / 60.0).floor();
                    let s = t % 60.0;
                    format!("{m:.0}m {s:.1}s")
                };
                // Inside a band there is no measured value to report, and
                // `Span` can't be hovered itself (its geometry is None), so
                // this is where the condition gets named. It is also the only
                // cue that isn't visual.
                if tooltip_spans.iter().any(|&(a, b)| t >= a && t <= b) {
                    return format!("{time_label}\noverload");
                }
                // With several traces on the same axes the number alone is
                // ambiguous, so name the one being hovered. Helper items carry
                // an empty name and fall through to the plain form.
                if multi_series && !name.is_empty() {
                    return format!("{time_label}\n{name}: {:.4} {cursor_unit}", point.y);
                }
                format!("{time_label}\n{:.4} {}", point.y, cursor_unit)
            });
        let response = plot.show(ui, |plot_ui| {
            // Set exact bounds: our X view range + computed Y range
            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                [view_min, y_min],
                [view_max, y_max],
            ));

            // Overload bands, before everything else so they sit behind the
            // data. An overload is the meter reporting a condition, not an
            // absence of one, so it reads as a filled region rather than the
            // dashed edges used for a dropout — a distinction that survives
            // without colour.
            //
            // Drawn at true duration with no minimum width: a brief excursion
            // collapses to a line rather than overstating how long the meter
            // was over range. `Span` clamps itself to the visible range, so a
            // band wider than the window still fills the plot — the case where
            // two edge markers would both be off-screen and show nothing.
            for &(start, end) in &overload_spans {
                plot_ui.span(
                    // Empty name: `Span` renders its name as a label inside
                    // the band, which would collide at narrow widths.
                    Span::new("", start..=end)
                        .fill(overload_fill)
                        .border_color(overload_edge)
                        .border_style(egui_plot::LineStyle::Solid),
                );
            }

            // Min/max envelope (drawn first so it's behind the data line)
            if show_envelope && !env_min.is_empty() {
                plot_ui.line(
                    Line::new("", PlotPoints::new(env_max.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.line(
                    Line::new("", PlotPoints::new(env_min.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Sub-values first: the plotted series goes on top of them.
            for (k, label, segments) in &overlay_traces {
                let (color, style) = Self::overlay_color_and_style(&tc, *k);
                for seg in segments {
                    plot_ui.line(
                        Line::new(label.clone(), PlotPoints::new(seg.clone()))
                            .color(color)
                            .style(style),
                    );
                }
            }

            for seg in &visible_segments {
                plot_ui.line(
                    Line::new(main_name.clone(), PlotPoints::new(seg.clone())).color(line_color),
                );
            }

            // No data: two dashed edges, no fill.
            for &(gap_start, gap_end, kind) in &visible_gaps {
                if kind != GapKind::NoData {
                    continue;
                }
                plot_ui.vline(
                    VLine::new("", gap_start)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.vline(
                    VLine::new("", gap_end)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Mean line overlay
            if show_mean && let Some((_, _, avg, _)) = visible_stats {
                plot_ui.hline(
                    HLine::new("", avg)
                        .color(mean_color)
                        .style(egui_plot::LineStyle::dashed_loose()),
                );
            }

            // Reference line overlays
            if show_ref {
                for &v in &ref_values {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(ref_color)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            }

            // Trigger crossing markers (where data crosses reference lines)
            if !crossings.is_empty() {
                plot_ui.points(
                    Points::new("", PlotPoints::new(crossings.clone()))
                        .color(cross_color)
                        .radius(4.0_f32)
                        .shape(egui_plot::MarkerShape::Diamond),
                );
            }

            // Measurement cursors (vertical + horizontal Y-value lines)
            if cursors_active {
                if let Some(t) = cursor_a {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_va {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(cursor_color_dim)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
                if let Some(t) = cursor_b {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_vb {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(cursor_color_dim)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            }
        });

        let overlay = OverlayLabelData {
            show_mean,
            mean_value,
            show_ref,
            ref_values,
            cursors_active,
            cursor_a,
            cursor_b,
            cursor_va,
            cursor_vb,
            overlay_unit: self.current_unit.clone(),
            view_max,
            mean_color,
            ref_color,
            cursor_color,
        };
        Self::paint_overlay_labels(ui, &response.response, &response.transform, &overlay);
        // Top-left, where `paint_overlay_labels` never draws — the Mean/Ref
        // and cursor labels are anchored to the right edge.
        Self::paint_plot_key(ui, response.response.rect, &key_entries, &tc);
        self.handle_interaction(ui, &response.response, &response.transform, can_interact);
        self.update_plot_a11y_label(ui, response.response.id, y_min, y_max);
        // Draw a focus ring on the main plot body when it's keyboard-focused.
        // Note: egui_plot also allocates separate focusable responses for the
        // X and Y axes — those receive Tab but don't draw a focus indicator.
        // Making those invisible to Tab would require patching egui_plot.
        crate::a11y::paint_focus_ring(ui, &response.response);
    }

    /// Set an AccessKit label on the plot that summarizes current state so
    /// screen readers have a text alternative to the pixels. Throttled: the
    /// label is only re-formatted when the underlying state changes.
    fn update_plot_a11y_label(&mut self, ui: &Ui, plot_id: egui::Id, y_min: f64, y_max: f64) {
        use std::hash::{Hash, Hasher};
        let last_value = self.history.back().map(|p| p.value);
        // Only the traces actually drawn are spoken: a sub-value the user
        // switched off is not on screen, so announcing it would describe a
        // graph that isn't there.
        let shown_labels: Vec<&str> = self
            .shown_overlays()
            .map(|(_, o)| o.label.as_str())
            .collect();
        let sig = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.time_window_secs.to_bits().hash(&mut h);
            // Quantize y bounds before hashing. Otherwise sub-pixel jitter
            // from animated auto-fit (and last-bit f64 noise from plot
            // transforms) busts the cache every frame, defeating the
            // throttle and forcing a fresh format! per render.
            quantize_for_hash(y_min).hash(&mut h);
            quantize_for_hash(y_max).hash(&mut h);
            self.history.len().hash(&mut h);
            self.live.hash(&mut h);
            last_value.map(f64::to_bits).hash(&mut h);
            self.current_unit.hash(&mut h);
            // Which series is plotted, and what is drawn beside it, are both
            // spoken below — so both have to bust the cache.
            self.current_series.as_deref().unwrap_or("").hash(&mut h);
            shown_labels.hash(&mut h);
            // Mode + display_raw drive the spoken last-reading; if either
            // changes (e.g. mode switch during paused playback) the label
            // must follow.
            self.current_mode.as_deref().unwrap_or("").hash(&mut h);
            self.last_display_raw.as_deref().unwrap_or("").hash(&mut h);
            // The over-range state is announced below, so it has to bust the
            // cache — otherwise the band appears with no spoken counterpart.
            self.pending_break.is_some().hash(&mut h);
            h.finish()
        };
        if sig != self.a11y_label_sig {
            let also = if shown_labels.is_empty() {
                String::new()
            } else {
                format!(" Also showing {}.", shown_labels.join(", "))
            };
            self.a11y_label_sig = sig;
            let unit = if self.current_unit.is_empty() {
                ""
            } else {
                &self.current_unit
            };
            let state = if self.live { "live" } else { "paused" };
            // Prefer the meter's own raw display string for the spoken
            // reading — it already encodes range/scale (e.g. "  1.234"
            // for a 22 V range vs " 12.34" for a 220 V range), so AT
            // users hear what sighted users see. Fall back to the f64
            // value only if the protocol doesn't provide display_raw.
            let reading = match (self.last_display_raw.as_deref(), last_value) {
                // Over range is the present state, so it outranks the last
                // plotted value — which is stale by definition while the
                // meter has nothing to measure. The band is otherwise a
                // purely visual cue.
                _ if self.pending_break == Some(GapKind::Overload) => {
                    "currently over range".to_string()
                }
                (Some(raw), _) => format!("last reading {} {unit}", raw.trim()),
                (None, Some(v)) => format!("last reading {v:.4} {unit}"),
                (None, None) => "no data".to_string(),
            };
            // The plot draws several traces at once for a multi-display
            // meter, and which one the axis belongs to is otherwise purely
            // visual — the key painted in its corner.
            let of_series = match self.current_series.as_deref() {
                Some(label) => format!(" of {label}"),
                None => String::new(),
            };
            self.a11y_label = format!(
                "Measurement plot{of_series}. {:.0} second window. Y axis {:.3} to {:.3} {unit}. {} samples.{also} {}. {}.",
                self.time_window_secs,
                y_min,
                y_max,
                self.history.len(),
                state,
                reading,
            );
        }
        crate::a11y::set_accessible_label(ui, plot_id, &self.a11y_label);
    }

    /// Paint text labels for overlays (mean, reference lines, cursors) using the
    /// UI painter so they render outside the plot's clip rect.
    fn paint_overlay_labels(
        ui: &Ui,
        plot_response: &egui::Response,
        transform: &PlotTransform,
        data: &OverlayLabelData,
    ) {
        let painter = ui.painter();
        let label_font = egui::FontId::proportional(12.0);
        let plot_rect = plot_response.rect;

        // Mean line label — anchored to right edge of plot rect
        if data.show_mean
            && let Some(avg) = data.mean_value
        {
            let y_pos = transform
                .position_from_point(&egui_plot::PlotPoint::new(data.view_max, avg))
                .y
                .clamp(plot_rect.top() + 12.0, plot_rect.bottom() - 2.0);
            painter.text(
                egui::pos2(plot_rect.right() - 4.0, y_pos - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("Mean: {avg:.4} {}", data.overlay_unit),
                label_font.clone(),
                data.mean_color,
            );
        }

        // Reference line labels
        if data.show_ref {
            for &v in &data.ref_values {
                let y_pos = transform
                    .position_from_point(&egui_plot::PlotPoint::new(data.view_max, v))
                    .y
                    .clamp(plot_rect.top() + 12.0, plot_rect.bottom() - 2.0);
                painter.text(
                    egui::pos2(plot_rect.right() - 4.0, y_pos - 2.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{v:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.ref_color,
                );
            }
        }

        // Cursor labels
        if data.cursors_active {
            if let Some(t) = data.cursor_a {
                let y_val = data.cursor_va.unwrap_or(0.0);
                let pos = transform.position_from_point(&egui_plot::PlotPoint::new(t, y_val));
                painter.text(
                    egui::pos2(pos.x + 4.0, pos.y - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("A: {t:.2} s / {y_val:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.cursor_color,
                );
            }
            if let Some(t) = data.cursor_b {
                let y_val = data.cursor_vb.unwrap_or(0.0);
                let pos = transform.position_from_point(&egui_plot::PlotPoint::new(t, y_val));
                painter.text(
                    egui::pos2(pos.x + 4.0, pos.y - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("B: {t:.2} s / {y_val:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.cursor_color,
                );
            }
        }
    }

    /// Return to live follow with auto Y. Shared by the double-click handler
    /// and the explicit "Reset Zoom" toolbar button.
    pub fn reset_view(&mut self) {
        self.live = true;
        self.view_center = 0.0;
        self.y_axis_fixed = false;
        self.y_user_set = false;
    }

    /// True when the view has been zoomed or panned away from the default
    /// live + auto-Y state. Used to enable/disable the Reset Zoom button.
    pub fn is_view_zoomed(&self) -> bool {
        !self.live || self.y_axis_fixed
    }

    /// Shift the view by `time_delta` seconds. The sign convention matches
    /// egui's `drag_delta().x`: positive = mouse moved right, which in the
    /// pan model reveals older data (view_center decreases).
    ///
    /// If currently live, first snaps view_center to the end of data and
    /// drops out of live so drag-to-pan works from live mode without a
    /// visible jump (the snapped bounds equal the live bounds on that frame).
    ///
    /// If the drag is moving toward newer data (mouse left → `time_delta < 0`)
    /// and would push the view's right edge to or past the latest sample,
    /// snap back to live instead of letting the view drift into empty
    /// future-space.
    fn apply_pan(&mut self, time_delta: f64) {
        if self.live {
            let (_, data_max) = self.data_time_range();
            self.view_center = data_max - self.time_window_secs / 2.0;
            self.live = false;
        }
        self.view_center -= time_delta;

        if time_delta < 0.0 {
            let (_, data_max) = self.data_time_range();
            let half = self.time_window_secs / 2.0;
            if self.view_center + half >= data_max {
                self.view_center = data_max - half;
                self.live = true;
            }
        }
    }

    /// Pure helper: given the two corners of a bbox-zoom rectangle in data
    /// coordinates, return the (view_center, time_window, y_min, y_max) that
    /// zooms the view to that region. Handles reversed drags by normalising
    /// min/max on each axis.
    fn bbox_to_view(p0: (f64, f64), p1: (f64, f64)) -> (f64, f64, f64, f64) {
        let x_min = p0.0.min(p1.0);
        let x_max = p0.0.max(p1.0);
        let y_min = p0.1.min(p1.1);
        let y_max = p0.1.max(p1.1);
        let view_center = (x_min + x_max) * 0.5;
        let time_window = x_max - x_min;
        (view_center, time_window, y_min, y_max)
    }

    /// Apply a bbox zoom to the current view state. Clamps time window to a
    /// sane minimum and mirrors the zoomed Y range into the toolbar text
    /// buffers so the numbers stay in sync.
    fn apply_bbox_zoom(&mut self, p0: (f64, f64), p1: (f64, f64)) {
        const MIN_TIME_WINDOW_SECS: f64 = 0.1;
        let (view_center, time_window, y_min, y_max) = Self::bbox_to_view(p0, p1);
        self.live = false;
        self.view_center = view_center;
        self.time_window_secs = time_window.max(MIN_TIME_WINDOW_SECS);
        self.y_axis_fixed = true;
        self.y_fixed_min = y_min;
        self.y_fixed_max = y_max;
        self.y_min_text = format!("{y_min:.4}");
        self.y_max_text = format!("{y_max:.4}");
        self.y_user_set = true;
    }

    /// Process drag, scroll, zoom, and cursor-click interactions on the plot.
    fn handle_interaction(
        &mut self,
        ui: &Ui,
        plot_response: &egui::Response,
        transform: &PlotTransform,
        can_interact: bool,
    ) {
        // Bounding-box zoom (Shift + left-drag). Runs before the pan/scroll
        // branches so it can claim the gesture and short-circuit them.
        let shift_held = ui.input(|i| i.modifiers.shift);
        let (primary_pressed, primary_down) =
            ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if self.bbox_zoom_start_px.is_some() && escape_pressed {
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
        }

        // Start: Shift held and primary just went down over the plot. We
        // intentionally do NOT gate on can_interact here — shift-drag should
        // also work from live mode, and we drop out of live on release.
        if shift_held
            && primary_pressed
            && self.bbox_zoom_start_px.is_none()
            && let Some(pos) = plot_response.hover_pos()
        {
            self.bbox_zoom_start_px = Some(pos);
            self.bbox_zoom_current_px = Some(pos);
        }

        // Track the pointer each frame while the drag is live. Fall back to
        // the global interact_pos when the cursor leaves the plot hover area.
        if self.bbox_zoom_start_px.is_some()
            && let Some(pos) = plot_response
                .hover_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        {
            self.bbox_zoom_current_px = Some(pos);
        }

        // Finish: primary released while a bbox-zoom drag was active.
        if self.bbox_zoom_start_px.is_some() && !primary_down {
            if let (Some(start), Some(end)) = (self.bbox_zoom_start_px, self.bbox_zoom_current_px) {
                // Clamp to the plot rect so dragging outside the axes still
                // produces a zoom bounded by what's visible.
                let plot_rect = plot_response.rect;
                let start_c = plot_rect.clamp(start);
                let end_c = plot_rect.clamp(end);
                let rect_px = egui::Rect::from_two_pos(start_c, end_c);
                const MIN_DRAG_PX: f32 = 5.0;
                if rect_px.width() >= MIN_DRAG_PX && rect_px.height() >= MIN_DRAG_PX {
                    let p0 = transform.value_from_position(rect_px.left_top());
                    let p1 = transform.value_from_position(rect_px.right_bottom());
                    self.apply_bbox_zoom((p0.x, p0.y), (p1.x, p1.y));
                }
            }
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
            return;
        }

        // Draw the rubber-band rectangle while the drag is in progress.
        if let (Some(start), Some(current)) = (self.bbox_zoom_start_px, self.bbox_zoom_current_px) {
            let rect = egui::Rect::from_two_pos(start, current);
            let visuals = ui.visuals();
            let fill = visuals.selection.bg_fill.linear_multiply(0.25);
            let stroke = egui::Stroke::new(1.0_f32, visuals.selection.stroke.color);
            ui.painter().rect_filled(rect, 0.0, fill);
            ui.painter()
                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            return;
        }

        // Handle drag: convert pixel delta to time delta. Works in live mode
        // too — apply_pan() snaps view_center to the current end of data and
        // drops out of live on the first drag frame so the pan has effect.
        if plot_response.dragged() {
            let drag_px = plot_response.drag_delta().x;
            let left = transform.value_from_position(plot_response.rect.left_top());
            let right = transform.value_from_position(plot_response.rect.right_top());
            let px_per_sec = plot_response.rect.width() as f64 / (right.x - left.x).max(1e-6);
            let time_delta = drag_px as f64 / px_per_sec;
            self.apply_pan(time_delta);
        }

        // Handle scroll wheel zoom on X axis — zoom centered on cursor position
        if can_interact {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                // Find cursor X position in time coordinates for centered zoom
                if let Some(hover_pos) = plot_response.hover_pos() {
                    let cursor_t = transform.value_from_position(hover_pos).x;
                    let old_half = self.time_window_secs / 2.0;
                    self.time_window_secs = (self.time_window_secs * factor).clamp(2.0, 3600.0);
                    let new_half = self.time_window_secs / 2.0;
                    // Adjust center so cursor stays at same relative position
                    let rel = (cursor_t - (self.view_center - old_half)) / (old_half * 2.0);
                    self.view_center = cursor_t - (rel - 0.5) * new_half * 2.0;
                } else {
                    self.time_window_secs = (self.time_window_secs * factor).clamp(2.0, 3600.0);
                }
            }
        }

        // Scroll while in live mode → exit live mode to browse
        if self.live {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let (_, data_max) = self.data_time_range();
                self.view_center = data_max - self.time_window_secs / 2.0;
                self.live = false;
            }
        }

        // Double-click to return to live mode + auto Y
        if plot_response.double_clicked() {
            self.reset_view();
        }

        // Cursor placement on click — snap to nearest data point
        if self.cursors_active
            && plot_response.clicked()
            && let Some(pos) = plot_response.interact_pointer_pos()
        {
            let click_t = transform.value_from_position(pos).x;
            if let Some((snapped_t, _)) = self.nearest_point(click_t) {
                if self.cursor_next_is_b {
                    self.cursor_b = Some(snapped_t);
                } else {
                    self.cursor_a = Some(snapped_t);
                }
                self.cursor_next_is_b = !self.cursor_next_is_b;
            }
        }
    }

    /// Render the minimap showing full history with viewport indicator.
    pub fn show_minimap(&mut self, ui: &mut Ui) {
        if self.history.len() < 2 {
            ui.allocate_space(egui::vec2(ui.available_width(), MINIMAP_HEIGHT));
            return;
        }

        self.ensure_cache();
        let raw_segments = &self.cached_segments;
        let (data_min, data_max) = self.data_time_range();
        let (view_min, view_max) = self.view_bounds();

        let tc = self.theme_colors(ui.visuals().dark_mode);
        let line_color = tc.minimap_line();
        let overload_fill = tc.graph_overload_fill();
        // Same spans the main plot bands, including one still open.
        let overload_spans: Vec<(f64, f64)> = self
            .cached_gaps
            .iter()
            .filter(|(_, _, kind)| *kind == GapKind::Overload)
            .map(|&(a, b, _)| (a, b))
            .chain(self.pending_overload_span())
            .collect();

        // Allocate rect for minimap + label space below, with margin for bracket strokes
        let label_height = 14.0;
        let margin = 4.0; // room for bracket strokes at edges
        let total_height = MINIMAP_HEIGHT + label_height + margin * 2.0;
        let (full_rect, pointer_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), total_height),
            egui::Sense::click_and_drag(),
        );
        let pointer_response = pointer_response
            .on_hover_text(
                "Minimap — click or drag to pan, drag the bracket edges to resize the view",
            )
            .a11y_label(
                "Graph minimap — click or drag to navigate timeline (Left/Right to pan), drag bracket edges to resize",
            );
        // Inset the plot area so brackets at edges have room to render
        let rect = egui::Rect::from_min_size(
            egui::pos2(full_rect.left() + margin, full_rect.top() + margin),
            egui::vec2(full_rect.width() - margin * 2.0, MINIMAP_HEIGHT),
        );

        // Use full_rect painter so nothing gets clipped
        let painter = ui.painter_at(full_rect);
        let data_span = (data_max - data_min).max(1e-6);

        let time_to_x =
            |t: f64| -> f32 { rect.left() + ((t - data_min) / data_span) as f32 * rect.width() };

        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        // Overload bands, matching the main plot so the two read the same way.
        // Behind the trace, like the background.
        //
        // Widened to one pixel when the span is narrower. Unlike the main
        // plot — where a floor would overstate the duration against a legible
        // time axis — the minimap compresses the whole session into a strip,
        // so sub-pixel is the normal case and the alternative is the band
        // silently not existing.
        for &(start, end) in &overload_spans {
            let x0 = time_to_x(start);
            let x1 = time_to_x(end).max(x0 + 1.0);
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom())),
                0.0,
                overload_fill,
            );
        }

        // Draw data lines.
        //
        // The Y range is identical for every point in every segment because it
        // covers the whole history (`data_min..data_max`). Compute it once
        // before the loop — pulling this call inside the per-point closure
        // turned the minimap into an O(n²) hot spot, which dominated frame
        // time once the history filled.
        //
        // Deliberately the *auto* range, not the main plot's: a pinned Y range
        // (a Shift-drag box zoom, or a user-entered fixed range) is chosen to
        // frame a detail of the main view, and scaling full history by it puts
        // most points far outside this 60px strip. They clip to a flat line
        // along the edges and overdraw the time axis below — the overview the
        // minimap exists to give disappears exactly when zooming in makes it
        // most useful.
        let y_map = self
            .y_range_for_view_auto(data_min, data_max, false)
            .map(|(lo, hi)| {
                let range = (hi - lo).max(1e-10);
                (lo, range)
            });
        for seg in raw_segments {
            let points: Vec<egui::Pos2> = seg
                .iter()
                .map(|&[t, v]| {
                    let x = time_to_x(t);
                    let y_frac = match y_map {
                        Some((y_lo, range)) => ((v - y_lo) / range) as f32,
                        None => 0.5,
                    };
                    let y = rect.bottom() - y_frac * rect.height();
                    egui::pos2(x, y)
                })
                .collect();
            for window in points.windows(2) {
                painter.line_segment(
                    [window[0], window[1]],
                    egui::Stroke::new(1.0_f32, line_color),
                );
            }
        }

        // Draw viewport indicator as [ ] bracket markers
        let vp_left = time_to_x(view_min);
        let vp_right = time_to_x(view_max);
        let vp_color = tc.minimap_viewport();
        let vp_stroke = egui::Stroke::new(2.5_f32, vp_color);
        let bracket_w = 4.0_f32; // horizontal arm of the bracket

        // Left bracket [
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.top()),
                egui::pos2(vp_left, rect.bottom()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.top()),
                egui::pos2(vp_left + bracket_w, rect.top()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.bottom()),
                egui::pos2(vp_left + bracket_w, rect.bottom()),
            ],
            vp_stroke,
        );

        // Right bracket ]
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.top()),
                egui::pos2(vp_right, rect.bottom()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.top()),
                egui::pos2(vp_right - bracket_w, rect.top()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.bottom()),
                egui::pos2(vp_right - bracket_w, rect.bottom()),
            ],
            vp_stroke,
        );

        // Draw X-axis time labels
        let label_color = ui.visuals().weak_text_color();
        let nice_interval = nice_time_interval(data_span);
        let mut t = (data_min / nice_interval).ceil() * nice_interval;
        while t <= data_max {
            let x = time_to_x(t);
            let label = format_time_label(t);
            painter.text(
                egui::pos2(x, rect.bottom() + 2.0),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::proportional(11.0),
                label_color,
            );
            // Small tick mark
            painter.line_segment(
                [
                    egui::pos2(x, rect.bottom() - 2.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, label_color),
            );
            t += nice_interval;
        }

        // Handle click/drag navigation with bracket resize handles
        let handle_half = 8.0_f32; // half-width of the resize hit zone in pixels

        // Cursor feedback: force resize icon during active resize drag,
        // otherwise show it on hover near bracket edges.
        if matches!(
            self.minimap_drag,
            MinimapDrag::ResizeLeft | MinimapDrag::ResizeRight
        ) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if let Some(hover_pos) = pointer_response.hover_pos() {
            let dl = (hover_pos.x - vp_left).abs();
            let dr = (hover_pos.x - vp_right).abs();
            if dl <= handle_half || dr <= handle_half {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }

        // Lock in drag mode on the mouse-down frame — bracket positions and click
        // position are from the same frame, so the hit-test is consistent even if
        // brackets shift later (live data arriving, resize in progress).
        if self.minimap_drag == MinimapDrag::None
            && pointer_response.is_pointer_button_down_on()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let dl = (origin.x - vp_left).abs();
            let dr = (origin.x - vp_right).abs();
            // When brackets are close, pick the nearest edge
            if dl <= handle_half && dl <= dr {
                self.minimap_drag = MinimapDrag::ResizeLeft;
            } else if dr <= handle_half {
                self.minimap_drag = MinimapDrag::ResizeRight;
            } else {
                self.minimap_drag = MinimapDrag::Pan;
            }
        }

        // Apply drag — resize uses per-frame delta, pan uses absolute position.
        // When the window is wider than the data, the brackets are clamped to
        // the data edges. On the first resize drag frame we snap the window to
        // the visible (clamped) span so the drag feels 1:1 with what's on screen.
        let time_per_px = data_span / rect.width() as f64;
        match self.minimap_drag {
            MinimapDrag::ResizeLeft => {
                let drag_px = pointer_response.drag_delta().x;
                if drag_px.abs() > 0.1 {
                    // Snap to visible span if window extends before data start
                    if self.time_window_secs > data_span + 0.1 {
                        self.time_window_secs = data_span;
                        self.view_center = data_min + data_span / 2.0;
                        self.live = false;
                    }
                    let dt = drag_px as f64 * time_per_px;
                    let right_edge = self.view_center + self.time_window_secs / 2.0;
                    self.time_window_secs = (self.time_window_secs - dt).clamp(2.0, 3600.0);
                    self.view_center = right_edge - self.time_window_secs / 2.0;
                    self.live = false;
                }
            }
            MinimapDrag::ResizeRight => {
                let drag_px = pointer_response.drag_delta().x;
                if drag_px.abs() > 0.1 {
                    // Snap to visible span if window extends past data end
                    if self.time_window_secs > data_span + 0.1 {
                        self.time_window_secs = data_span;
                        self.view_center = data_min + data_span / 2.0;
                        self.live = false;
                    }
                    let dt = drag_px as f64 * time_per_px;
                    let left_edge = (self.view_center - self.time_window_secs / 2.0).max(0.0);
                    self.time_window_secs = (self.time_window_secs + dt).clamp(2.0, 3600.0);
                    self.view_center = left_edge + self.time_window_secs / 2.0;
                    self.live = self.view_center + self.time_window_secs / 2.0 >= data_max;
                }
            }
            MinimapDrag::Pan => {
                if let Some(pos) = pointer_response.interact_pointer_pos() {
                    let pos_t = data_min
                        + ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * data_span;
                    let half = self.time_window_secs / 2.0;
                    if pos_t + half >= data_max {
                        self.live = true;
                    } else {
                        self.view_center = pos_t;
                        self.live = false;
                    }
                }
            }
            MinimapDrag::None => {}
        }

        // Reset drag state when pointer is released
        if !pointer_response.is_pointer_button_down_on() {
            self.minimap_drag = MinimapDrag::None;
        }

        // Keyboard pan when the minimap holds focus. Has to run *after*
        // `allocate_exact_size` above (which runs `interested_in_focus`
        // on the pointer response) so that on a Tab press focus has
        // already advanced away and `pointer_response.has_focus()`
        // returns false. Otherwise the `move_focus(FocusDirection::None)`
        // below would also wipe egui's `Focus::begin_pass` snapshot of
        // Tab into `focus_direction = Next` (egui stores arrow keys and
        // Tab in the same field), trapping focus on the minimap.
        if pointer_response.has_focus() {
            use egui::{Key, Modifiers};
            let left = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft));
            let right = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight));
            // Up/Down get consumed too even though they don't pan —
            // without that, `end_pass` would walk
            // `find_widget_in_direction` on the begin_pass snapshot and
            // Tab-jump focus to the spatially nearest widget on every
            // Up/Down press.
            let up = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
            let down = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
            if left {
                self.scroll_view(-0.25);
            }
            if right {
                self.scroll_view(0.25);
            }
            if left || right || up || down {
                ui.ctx()
                    .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }

        // Paint focus ring last so it sits on top of the minimap content
        // (background, brackets, time labels) rather than being overdrawn.
        crate::a11y::paint_focus_ring(ui, &pointer_response);
    }

    /// Combined render: toolbar + main graph + minimap.
    pub fn show(&mut self, ui: &mut Ui) {
        self.handle_keyboard(ui.ctx());
        self.show_toolbar(ui);
        let minimap_reserve = MINIMAP_HEIGHT + 30.0;
        let main_height = (ui.available_height() - minimap_reserve).max(60.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), main_height), |ui| {
            self.show_main(ui);
        });
        ui.add_space(4.0);
        self.show_minimap(ui);
    }

    /// Compute the time-integral over the visible window, in unit·seconds.
    pub fn visible_integral(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        self.cursor_integral(x_min, x_max)
    }

    /// Elapsed time (seconds) between the first and last data point in the
    /// visible window. Returns `None` if fewer than 2 points are visible.
    pub fn visible_data_span_secs(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        let (start, end) = self.visible_index_range(x_min, x_max);
        if end - start < 2 {
            return None;
        }
        let first = self.elapsed_secs(self.history[start].time);
        let last = self.elapsed_secs(self.history[end - 1].time);
        if last > first {
            Some(last - first)
        } else {
            None
        }
    }

    pub fn visible_stats(&self) -> Option<(f64, f64, f64, usize)> {
        let (x_min, x_max) = self.view_bounds();
        let (start, end) = self.visible_index_range(x_min, x_max);
        if start >= end {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let count = end - start;
        for i in start..end {
            let v = self.history[i].value;
            min = min.min(v);
            max = max.max(v);
            sum += v;
        }
        Some((min, max, sum / count as f64, count))
    }

    /// Build min/max envelope using a trailing sliding window.
    /// At each data point time `t`, computes min/max of all points in `[t - window, t]`.
    /// This answers "what was the range over the last N seconds?" with no look-ahead.
    ///
    /// Sliding-window min/max via two monotonic deques (front holds the
    /// current extremum). Each index is pushed and popped at most once, so
    /// the whole pass is O(n) instead of the previous O(n²) which re-scanned
    /// the window for every point.
    fn build_envelope(
        &self,
        x_min: f64,
        x_max: f64,
        window_secs: f64,
    ) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let window = window_secs.max(0.1);

        // Collect points: need data back to x_min - window for edge correctness.
        // Use visible_index_range to skip the bulk of the history.
        let (start, end) = self.visible_index_range(x_min - window, x_max);
        let points: Vec<(f64, f64)> = (start..end)
            .map(|i| {
                let p = &self.history[i];
                (self.elapsed_secs(p.time), p.value)
            })
            .collect();

        if points.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let n = points.len();
        let mut min_pts = Vec::with_capacity(n);
        let mut max_pts = Vec::with_capacity(n);
        // Deques hold indices into `points`, with values monotonically
        // increasing (min) or decreasing (max). The front is always the
        // extremum of the current window.
        let mut min_deque: VecDeque<usize> = VecDeque::new();
        let mut max_deque: VecDeque<usize> = VecDeque::new();

        for (i, &(t, v)) in points.iter().enumerate() {
            // Maintain the monotonic invariant by popping any back entries
            // that the new value supersedes.
            while min_deque.back().is_some_and(|&b| points[b].1 >= v) {
                min_deque.pop_back();
            }
            min_deque.push_back(i);
            while max_deque.back().is_some_and(|&b| points[b].1 <= v) {
                max_deque.pop_back();
            }
            max_deque.push_back(i);

            // Drop fronts that have fallen out of the trailing window.
            let win_start = t - window;
            while min_deque.front().is_some_and(|&f| points[f].0 < win_start) {
                min_deque.pop_front();
            }
            while max_deque.front().is_some_and(|&f| points[f].0 < win_start) {
                max_deque.pop_front();
            }

            // Only emit envelope points within the visible range. Points
            // before `x_min` still feed the deques so the first visible
            // sample sees the correct trailing window.
            if t < x_min {
                continue;
            }

            let vmin = points[*min_deque.front().expect("min deque non-empty after push")].1;
            let vmax = points[*max_deque.front().expect("max deque non-empty after push")].1;
            min_pts.push([t, vmin]);
            max_pts.push([t, vmax]);
        }

        (min_pts, max_pts)
    }

    /// Find points where the data crosses any of the given threshold values.
    /// Returns crossing points as [time, threshold_value].
    fn find_crossings(&self, thresholds: &[f64], x_min: f64, x_max: f64) -> Vec<[f64; 2]> {
        let (start, end) = self.visible_index_range(x_min, x_max);
        let mut crossings = Vec::new();
        let mut prev: Option<f64> = None;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);
            if let Some(prev_v) = prev {
                for &thresh in thresholds {
                    // Strict on the "coming from" side. With `<=` on both,
                    // prev_v == value == thresh satisfied *both* arms, so a
                    // signal resting exactly on the reference (a steady 5.000 V
                    // against Ref 5, or open leads reading 0.000 against Ref 0)
                    // emitted a marker for every sample and painted a solid row
                    // over the trace. A reading that arrives at the threshold
                    // is still marked once, from whichever side it came.
                    let crossed = (prev_v < thresh && point.value >= thresh)
                        || (prev_v > thresh && point.value <= thresh);
                    if crossed {
                        crossings.push([t, thresh]);
                    }
                }
            }
            prev = Some(point.value);
        }
        crossings
    }

    /// Find the nearest data point to the given time via binary search.
    /// Returns (snapped_time, value).
    fn nearest_point(&self, t: f64) -> Option<(f64, f64)> {
        if self.history.is_empty() {
            return None;
        }
        // Find the insertion point — the first index whose time > t.
        let (a, b) = self.history.as_slices();
        let a_len = a.len();
        let pos_a = a.partition_point(|p| self.elapsed_secs(p.time) <= t);
        let pos = if pos_a < a_len {
            pos_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(p.time) <= t)
        };

        // The nearest point is either at `pos` or `pos - 1`. Check both.
        let mut best: Option<(f64, f64, f64)> = None; // (dist, time, value)
        for &idx in &[pos.wrapping_sub(1), pos] {
            if idx < self.history.len() {
                let pt = self.elapsed_secs(self.history[idx].time);
                let dist = (pt - t).abs();
                if best.is_none_or(|(best_dist, _, _)| dist < best_dist) {
                    best = Some((dist, pt, self.history[idx].value));
                }
            }
        }
        best.map(|(_, t, v)| (t, v))
    }

    /// Compute the time-integral between two cursor positions using the trapezoidal
    /// rule. Returns the raw integral in unit·seconds, or `None` if fewer than 2
    /// data points exist in the range. Skips intervals exceeding
    /// `gap_threshold_secs`, and intervals interrupted by an overload.
    fn cursor_integral(&self, ta: f64, tb: f64) -> Option<f64> {
        let (t_start, t_end) = if ta <= tb { (ta, tb) } else { (tb, ta) };
        let (start, end) = self.visible_index_range(t_start, t_end);
        let mut integral = 0.0;
        let mut prev: Option<(f64, f64)> = None; // (time, value)
        let mut has_pair = false;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);
            // An overload breaks the series even when the samples either side
            // of it are adjacent in time — integrating across it would credit
            // the area under a value the meter never measured.
            if let Some((pt, pv)) = prev {
                let dt = t - pt;
                if dt <= self.gap_threshold_secs && point.break_before.is_none() {
                    integral += (pv + point.value) / 2.0 * dt;
                    has_pair = true;
                }
            }
            prev = Some((t, point.value));
        }

        has_pair.then_some(integral)
    }

    /// Span of an interruption that hasn't closed yet, as `(start, end)`.
    ///
    /// `build_segments_for_range` pairs consecutive points, so it can only
    /// emit a gap once a sample arrives *after* the interruption. While an
    /// overload is still in progress no such sample exists, and the trace
    /// would simply stop with nothing to say why until the meter came back.
    ///
    /// Anchored to the last plotted point — where the trace ends — and closed
    /// at the newest overload sample rather than at "now", so a meter that
    /// disconnects mid-overload leaves a band covering the samples we actually
    /// received instead of one that grows forever.
    fn pending_overload_span(&self) -> Option<(f64, f64)> {
        if self.pending_break != Some(GapKind::Overload) {
            return None;
        }
        let start = self.history.back().map(|p| self.elapsed_secs(p.time))?;
        let end = self
            .pending_break_since
            .map(|t| self.elapsed_secs(t))
            .unwrap_or(start);
        Some((start, end.max(start)))
    }

    /// Gap ranges across the whole history, through the same builder the main
    /// graph renders from — so these tests exercise the path that actually
    /// draws the gap markers.
    /// Full-history segments, through the same builder the minimap caches.
    #[cfg(test)]
    fn all_segments(&self) -> Vec<Vec<[f64; 2]>> {
        self.build_segments_for_range(0, self.history.len()).0
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn new_graph_is_empty() {
        let g = Graph::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(g.live);
    }

    #[test]
    fn push_adds_point() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V", None);
        assert_eq!(g.len(), 1);
        assert!(!g.is_empty());
        assert!(g.origin.is_some());
    }

    #[test]
    fn push_records_display_raw_for_a11y() {
        let mut g = Graph::new();
        g.push(0.001234, Instant::now(), "DC V", "mV", Some("  1.234"));
        assert_eq!(g.last_display_raw.as_deref(), Some("  1.234"));
        // A second push with display_raw must reuse the existing String
        // buffer, not allocate a new one.
        g.push(0.005, Instant::now(), "DC V", "mV", Some("  5.000"));
        assert_eq!(g.last_display_raw.as_deref(), Some("  5.000"));
    }

    #[test]
    fn push_clears_display_raw_on_mode_change() {
        let mut g = Graph::new();
        g.push(0.001, Instant::now(), "DC V", "mV", Some("  1.000"));
        // Mode change clears history and the cached raw.
        g.push(100.0, Instant::now(), "Ohm", "Ω", None);
        assert!(g.last_display_raw.is_none());
    }

    #[test]
    fn quantize_for_hash_collapses_jitter() {
        // Two values that differ by less than the quantization step (1e-3)
        // must hash-quantize to the same bucket.
        assert_eq!(quantize_for_hash(1.2345), quantize_for_hash(1.2346));
        // Values that differ by more than one bucket must not collapse.
        assert_ne!(quantize_for_hash(1.234), quantize_for_hash(1.236));
        // NaN gets a sentinel so it doesn't poison the hasher.
        assert_eq!(quantize_for_hash(f64::NAN), i64::MIN);
    }

    #[test]
    fn mode_change_clears_history() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V", None);
        g.push(5.1, Instant::now(), "DC V", "V", None);
        assert_eq!(g.len(), 2);
        g.push(100.0, Instant::now(), "Ohm", "Ω", None);
        assert_eq!(g.len(), 1);
    }

    /// Auto-range crossing a decade keeps the mode string but moves the unit,
    /// so 219 Ω and 0.22 kΩ would otherwise share one series — the trace
    /// collapses 1000x mid-plot and the axis silently relabels.
    #[test]
    fn unit_change_clears_history() {
        let mut g = Graph::new();
        g.push(150.0, Instant::now(), "Ω", "Ω", None);
        g.push(219.0, Instant::now(), "Ω", "Ω", None);
        assert_eq!(g.len(), 2);
        g.push(0.22, Instant::now(), "Ω", "kΩ", None);
        assert_eq!(g.len(), 1, "kΩ points must not share a series with Ω");
        assert_eq!(g.current_unit, "kΩ");
    }

    /// The pinned Y range is chosen for the old decade's numbers; keeping it
    /// across a unit change plots the new scale far outside the view.
    #[test]
    fn unit_change_releases_the_pinned_y_range() {
        let mut g = Graph::new();
        g.push(150.0, Instant::now(), "Ω", "Ω", None);
        g.y_axis_fixed = true;
        g.y_user_set = true;
        g.push(0.22, Instant::now(), "Ω", "kΩ", None);
        assert!(!g.y_axis_fixed);
        assert!(!g.y_user_set);
    }

    /// A brief over-range excursion leaves no hole in the timestamps, so
    /// time-based gap detection can't see it: the trace was drawn straight
    /// from the last good sample to the first one after, through a region the
    /// meter reported as unmeasurable.
    #[test]
    fn overload_breaks_the_trace_without_a_time_gap() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        g.push_break(Instant::now());
        g.push(3.0, t0 + Duration::from_millis(200), "DC V", "V", None);

        // All three samples are 100ms apart — well inside the 1s minimum gap
        // threshold — so only the explicit break can split them.
        let segments = g.all_segments();
        assert_eq!(segments.len(), 2, "overload must split the trace");
        assert_eq!(segments[0].len(), 2);
        assert_eq!(segments[1].len(), 1);
    }

    #[test]
    fn overload_is_reported_as_a_gap_range() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(Instant::now());
        g.push(3.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        assert_eq!(g.visible_gaps().len(), 1);
    }

    /// An overload that is still in progress has no closing sample, so the
    /// paired-gap builder emits nothing and the trace just stops. The opening
    /// marker has to be drawn from the pending state instead.
    #[test]
    fn an_unfinished_overload_still_marks_where_the_trace_stopped() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        assert_eq!(g.pending_overload_span(), None, "no break yet");

        g.push_break(Instant::now());
        // Still overloaded: no closing sample, so no paired gap exists...
        assert!(g.visible_gaps().is_empty());
        // ...but the pending span anchors to the last plotted point.
        let (start, _) = g
            .pending_overload_span()
            .expect("pending span while overloaded");
        assert!((start - 0.1).abs() < 1e-9, "got {start}");

        // Meter recovers: the pair takes over and the pending marker clears.
        g.push(3.0, t0 + Duration::from_millis(500), "DC V", "V", None);
        assert_eq!(g.pending_overload_span(), None);
        assert_eq!(g.visible_gaps().len(), 1);
    }

    /// The two kinds must be distinguishable by the renderer: an overload is
    /// the meter reporting a condition, a time gap is the absence of any
    /// report. They are drawn differently, so the builder has to say which.
    #[test]
    fn gap_kinds_distinguish_overload_from_data_loss() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(t0 + Duration::from_millis(50));
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        // Well past the 1 s minimum gap threshold: a dropout, not an overload.
        g.push(3.0, t0 + Duration::from_secs(5), "DC V", "V", None);

        let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
        assert_eq!(kinds, vec![GapKind::Overload, GapKind::NoData]);
    }

    /// Losing the link mid-overload is two things end to end: a stretch the
    /// meter reported over-range, then a stretch it reported nothing. Folding
    /// the silence into the band would claim the meter was over range for a
    /// period it never reported at all.
    #[test]
    fn a_dropout_during_an_overload_splits_into_band_then_gap() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        // Half a second of overload, then the link drops for 30 s.
        g.push_break(t0 + Duration::from_millis(500));
        g.push_data_loss();
        g.push(2.0, t0 + Duration::from_secs(30), "DC V", "V", None);

        let gaps = g.visible_gaps();
        assert_eq!(gaps.len(), 2, "expected band then gap, got {gaps:?}");

        let (band_start, band_end, band_kind) = gaps[0];
        assert_eq!(band_kind, GapKind::Overload);
        assert!(band_start.abs() < 1e-9);
        assert!(
            (band_end - 0.5).abs() < 1e-9,
            "band must stop at the last OL sample we heard, got {band_end}"
        );

        let (gap_start, gap_end, gap_kind) = gaps[1];
        assert_eq!(gap_kind, GapKind::NoData);
        assert!((gap_start - 0.5).abs() < 1e-9);
        assert!((gap_end - 30.0).abs() < 1e-9);
    }

    /// Measured on hardware: releasing the leads makes the meter step
    /// 2.2MΩ → 22MΩ → 220MΩ, pausing 462 ms and then 1153 ms against a 97 ms
    /// steady cadence. That silence is longer than the gap threshold but is
    /// not data loss — the link never dropped — and must not be painted as a
    /// dropout.
    #[test]
    fn an_auto_range_stutter_is_not_a_dropout() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.4934, t0, "Ω", "MΩ", None);
        g.push_break(t0 + Duration::from_millis(97)); // OL, 2.2MΩ
        g.push_break(t0 + Duration::from_millis(559)); // OL, 22MΩ — 462 ms later
        // 1153 ms later the meter reports again, still without a disconnect.
        g.push(97.14, t0 + Duration::from_millis(1712), "Ω", "MΩ", None);

        let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
        assert_eq!(
            kinds,
            vec![GapKind::Overload],
            "a quiet meter is not a lost connection"
        );
    }

    /// A disconnect is not the only way data stops. A meter that powers off
    /// mid-overload keeps its USB bridge enumerated, so nothing raises
    /// Disconnected — the reads just time out. That has to split the band
    /// too, or it would claim over-range for the whole outage.
    #[test]
    fn a_timeout_outage_during_an_overload_also_splits() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(t0 + Duration::from_millis(100));
        // Reads time out until the App gives up; no Disconnected involved.
        g.push_data_loss();
        g.push(2.0, t0 + Duration::from_secs(20), "DC V", "V", None);

        let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
        assert_eq!(kinds, vec![GapKind::Overload, GapKind::NoData]);
    }

    /// Data loss outside an overload still breaks the trace, even when the
    /// outage is shorter than the elapsed-time threshold — the App knows
    /// samples are missing, so it doesn't have to be inferred.
    #[test]
    fn a_brief_dropout_without_an_overload_still_shows() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_data_loss();
        // Well under the 1 s threshold, so nothing would be inferred here.
        g.push(2.0, t0 + Duration::from_millis(200), "DC V", "V", None);

        let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
        assert_eq!(kinds, vec![GapKind::NoData]);
    }

    /// While the link is up, OL samples keep arriving, so a long overload is
    /// all band and no dropout — the case that must not regress.
    #[test]
    fn a_connected_overload_produces_no_dropout() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        // Ten seconds of overload, sampled throughout.
        for i in 1..=100 {
            g.push_break(t0 + Duration::from_millis(i * 100));
        }
        g.push(2.0, t0 + Duration::from_millis(10_100), "DC V", "V", None);

        let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
        assert_eq!(kinds, vec![GapKind::Overload]);
    }

    /// No minimum width is applied on the main plot: a brief excursion stays
    /// sub-pixel and collapses to a line rather than being widened to
    /// something legible, which would overstate how long the meter was over
    /// range. With no dropout recorded the band covers the whole
    /// interruption — the meter was over range across it, we simply don't
    /// sample continuously.
    #[test]
    fn a_brief_overload_is_not_widened() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(t0 + Duration::from_micros(500));
        g.push(2.0, t0 + Duration::from_millis(1), "DC V", "V", None);

        let gaps = g.visible_gaps();
        assert_eq!(gaps.len(), 1, "half a millisecond is under the threshold");
        let (start, end, kind) = gaps[0];
        assert_eq!(kind, GapKind::Overload);
        let width = end - start;
        assert!(
            (width - 0.001).abs() < 1e-9,
            "band must span the real 1 ms interruption, got {width}"
        );
    }

    /// The minimap reads its bands from the same cache it draws the trace
    /// from, so the cache has to carry gaps — an earlier version of that
    /// field was write-only and was removed.
    #[test]
    fn the_cache_carries_gaps_for_the_minimap() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(t0 + Duration::from_millis(50));
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);

        g.ensure_cache();
        assert_eq!(g.cached_segments.len(), 2, "trace splits either side");
        let kinds: Vec<GapKind> = g.cached_gaps.iter().map(|&(_, _, k)| k).collect();
        assert_eq!(kinds, vec![GapKind::Overload]);
    }

    /// The cache is keyed on history_version; a new sample must invalidate it
    /// or the minimap would keep drawing a stale set of bands.
    #[test]
    fn the_cache_refreshes_when_a_break_arrives() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.ensure_cache();
        assert!(g.cached_gaps.is_empty());

        g.push_break(t0 + Duration::from_millis(50));
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        g.ensure_cache();
        assert_eq!(g.cached_gaps.len(), 1);
    }

    /// An overload before any data has nothing to anchor to.
    #[test]
    fn a_break_with_no_history_marks_nothing() {
        let mut g = Graph::new();
        g.push_break(Instant::now());
        assert_eq!(g.pending_overload_span(), None);
    }

    /// The opening marker anchors to the last plotted point, and in live mode
    /// the window used to end at that same point — so the marker landed on
    /// the plot border and read as part of it. Overload samples carry
    /// timestamps, so the window can follow them.
    #[test]
    fn the_view_follows_overload_samples() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);

        let (_, before) = g.view_bounds();
        assert!(before.abs() < 1e-9, "window ends at the only sample");

        // Two seconds of overload samples arrive, carrying timestamps.
        g.push_break(t0 + Duration::from_secs(2));
        let (_, during) = g.view_bounds();
        let (band_start, band_end) = g.pending_overload_span().expect("span while overloaded");
        assert!(
            during > band_start + 1.0,
            "view must advance past the band start: start={band_start}, x_max={during}"
        );
        assert!(
            (band_end - 2.0).abs() < 1e-9,
            "band closes at the newest overload sample, got {band_end}"
        );

        // Recovery closes the gap and hands the window back to the data.
        g.push(2.0, t0 + Duration::from_secs(3), "DC V", "V", None);
        assert_eq!(g.pending_overload_span(), None);
        let (_, after) = g.view_bounds();
        assert!((after - 3.0).abs() < 1e-9, "got {after}");
    }

    /// The minimap maps time to x from `data_time_range`, so if that ignored
    /// overload samples the strip would stop advancing mid-excursion and the
    /// band would have nowhere to grow — the same freeze the main plot had.
    /// Fixed at the range itself so every consumer follows, not per call site.
    #[test]
    fn the_data_range_counts_overload_samples() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);

        let (_, before) = g.data_time_range();
        assert!(before.abs() < 1e-9, "only sample sits at the origin");

        g.push_break(t0 + Duration::from_secs(4));
        let (min, max) = g.data_time_range();
        assert!(min.abs() < 1e-9, "start is unaffected");
        assert!(
            (max - 4.0).abs() < 1e-9,
            "range must reach the newest overload sample, got {max}"
        );

        // And hands back to the plotted data once the meter recovers.
        g.push(2.0, t0 + Duration::from_secs(5), "DC V", "V", None);
        let (_, after) = g.data_time_range();
        assert!((after - 5.0).abs() < 1e-9, "got {after}");
    }

    /// A paused or disconnected meter produces no samples at all — not even
    /// overload ones — so its window must hold still rather than scrolling
    /// the data off screen. This is why the view follows sample timestamps
    /// and not the wall clock.
    #[test]
    fn the_view_holds_still_without_samples() {
        let mut g = Graph::new();
        let t0 = Instant::now() - Duration::from_secs(5);
        g.push(1.0, t0, "DC V", "V", None);
        let (_, first) = g.view_bounds();
        let (_, second) = g.view_bounds();
        assert!(
            (first - second).abs() < 1e-9,
            "window drifted with no samples"
        );
    }

    /// Consecutive overload samples are one interruption, not several.
    #[test]
    fn repeated_overloads_produce_one_break() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        for _ in 0..5 {
            g.push_break(Instant::now());
        }
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        assert_eq!(g.all_segments().len(), 2);
        assert_eq!(g.visible_gaps().len(), 1);
    }

    /// The break must not persist past the point that consumed it.
    #[test]
    fn break_applies_only_to_the_next_point() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push_break(Instant::now());
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        g.push(3.0, t0 + Duration::from_millis(200), "DC V", "V", None);
        let segments = g.all_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].len(), 2, "samples after the break rejoin");
    }

    /// Clearing must drop a pending break, or the first point of the next
    /// session would start orphaned.
    #[test]
    fn clear_discards_a_pending_break() {
        let mut g = Graph::new();
        g.push_break(Instant::now());
        g.clear();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
        assert_eq!(g.all_segments().len(), 1);
    }

    /// Same mode and unit must not clear — otherwise the graph would reset on
    /// every sample and never accumulate.
    #[test]
    fn steady_unit_keeps_history() {
        let mut g = Graph::new();
        for i in 0..5 {
            g.push(i as f64, Instant::now(), "DC V", "V", None);
        }
        assert_eq!(g.len(), 5);
    }

    #[test]
    fn max_points_evicts_oldest() {
        let mut g = Graph::new();
        for i in 0..MAX_POINTS + 100 {
            g.push(i as f64, Instant::now(), "DC V", "V", None);
        }
        assert_eq!(g.len(), MAX_POINTS);
    }

    /// A signal sitting exactly on the reference value used to report a
    /// crossing on every sample, painting a solid row of markers along the
    /// reference line and hiding the trace underneath.
    #[test]
    fn flat_signal_on_the_reference_is_not_a_crossing() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..10 {
            g.push(5.0, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
        }
        assert!(g.find_crossings(&[5.0], 0.0, 100.0).is_empty());
    }

    /// The zero case matters just as much: open leads read 0.000 and Ref 0 is
    /// a natural thing to set.
    #[test]
    fn flat_zero_signal_on_a_zero_reference_is_not_a_crossing() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..5 {
            g.push(0.0, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
        }
        assert!(g.find_crossings(&[0.0], 0.0, 100.0).is_empty());
    }

    #[test]
    fn real_crossings_are_still_reported() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        // Rise through the threshold, then fall back through it.
        for (i, v) in [4.0, 6.0, 4.0].into_iter().enumerate() {
            g.push(
                v,
                t0 + Duration::from_millis(i as u64 * 100),
                "DC V",
                "V",
                None,
            );
        }
        assert_eq!(g.find_crossings(&[5.0], 0.0, 100.0).len(), 2);
    }

    /// Arriving exactly on the reference is a crossing — once, not once per
    /// sample spent sitting there.
    #[test]
    fn touching_the_reference_marks_a_single_crossing() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for (i, v) in [4.0, 5.0, 5.0, 5.0].into_iter().enumerate() {
            g.push(
                v,
                t0 + Duration::from_millis(i as u64 * 100),
                "DC V",
                "V",
                None,
            );
        }
        assert_eq!(g.find_crossings(&[5.0], 0.0, 100.0).len(), 1);
    }

    /// The minimap is a full-history overview, so it must scale to the data
    /// even when the main plot's Y axis is pinned to a narrow band. Scaling by
    /// the pin put points many multiples of the 60px strip's height away, and
    /// they clipped to a flat line along its edges.
    #[test]
    fn minimap_scale_ignores_the_pinned_y_range() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for (i, v) in [0.0, 5.0, 10.0].into_iter().enumerate() {
            g.push(
                v,
                t0 + Duration::from_millis(i as u64 * 100),
                "DC V",
                "V",
                None,
            );
        }
        // Pin a narrow band, as a Shift-drag box zoom does.
        g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
        assert!(g.y_axis_fixed);
        assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));

        let (lo, hi) = g.y_range_for_view_auto(0.0, 1.0, true).expect("auto range");
        assert!(
            lo <= 0.0 && hi >= 10.0,
            "minimap range {lo}..{hi} must cover the full 0..10 data span"
        );
    }

    /// A Y range pinned while measuring volts must not survive into ohms —
    /// the new trace would sit far outside it and the plot would look empty.
    #[test]
    fn mode_change_releases_the_pinned_y_range() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V", None);
        g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
        assert!(g.y_axis_fixed);
        assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));

        g.push(1000.0, Instant::now(), "Ohm", "Ω", None);
        assert!(
            !g.y_axis_fixed,
            "mode change must release the fixed Y range"
        );
        assert!(!g.y_user_set);
        assert_ne!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));
    }

    /// Staying in the same mode must keep the user's zoom — otherwise every
    /// incoming sample would fight the pinned view.
    #[test]
    fn same_mode_keeps_the_pinned_y_range() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V", None);
        g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
        g.push(5.05, Instant::now(), "DC V", "V", None);
        assert!(g.y_axis_fixed);
        assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));
    }

    #[test]
    fn clear_resets_everything() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V", None);
        g.live = false;
        g.clear();
        assert!(g.is_empty());
        assert_eq!(g.current_mode, None);
        assert!(g.origin.is_none());
        assert!(g.live);
    }

    #[test]
    fn segments_without_gaps() {
        let mut g = Graph::new();
        g.push(1.0, Instant::now(), "DC V", "V", None);
        g.push(2.0, Instant::now(), "DC V", "V", None);
        g.push(3.0, Instant::now(), "DC V", "V", None);
        let segments = g.all_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn gap_detection() {
        let mut g = Graph::new();
        g.push(1.0, Instant::now(), "DC V", "V", None);
        g.push(2.0, Instant::now(), "DC V", "V", None);
        assert!(g.visible_gaps().is_empty());
    }

    #[test]
    fn elapsed_secs_relative_to_origin() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V", None);
        g.push(2.0, t0 + Duration::from_millis(50), "DC V", "V", None);
        let t = g.elapsed_secs(g.history.back().unwrap().time);
        assert!((t - 0.05).abs() < 1e-9);
    }

    #[test]
    fn live_view_bounds_follow_latest() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V", None);
        let (vmin, vmax) = g.view_bounds();
        assert!(vmin >= 0.0);
        assert!(vmax >= vmin);
    }

    #[test]
    fn manual_view_bounds() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = 50.0;
        let (vmin, vmax) = g.view_bounds();
        assert!((vmin - 45.0).abs() < 0.1);
        assert!((vmax - 55.0).abs() < 0.1);
    }

    #[test]
    fn time_window_presets_exist() {
        assert!(TIME_WINDOWS.len() >= 3);
        assert_eq!(TIME_WINDOWS[0].1, "5s");
    }

    #[test]
    fn cycle_time_window_shorter() {
        let mut g = Graph::new();
        g.time_window_secs = 60.0; // 1m
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 30.0).abs() < 0.1);
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 10.0).abs() < 0.1);
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 5.0).abs() < 0.1);
        // Already at minimum preset — stays at 5s
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 5.0).abs() < 0.1);
    }

    #[test]
    fn cycle_time_window_longer() {
        let mut g = Graph::new();
        g.time_window_secs = 60.0; // 1m
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 300.0).abs() < 0.1);
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 600.0).abs() < 0.1);
        // Already at maximum preset — stays at 600s
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 600.0).abs() < 0.1);
    }

    #[test]
    fn scroll_view_does_not_panic() {
        let mut g = Graph::new();
        for i in 0..20 {
            g.push(i as f64, Instant::now(), "V DC", "V", None);
        }
        assert!(g.live);
        // With only ~ms of real elapsed time and a 60s window, the view
        // stays pinned at the end so live remains true. This test validates
        // the method doesn't panic on minimal data spans.
        g.scroll_view(-0.25);
        g.scroll_view(0.25);
    }

    #[test]
    fn jump_to_start_exits_live() {
        let mut g = Graph::new();
        g.push(1.0, Instant::now(), "V DC", "V", None);
        assert!(g.live);
        g.jump_to_start();
        assert!(!g.live);
    }

    #[test]
    fn bbox_to_view_normal_drag() {
        // Top-left (t=10, v=5) to bottom-right (t=20, v=2).
        let (center, window, y_min, y_max) = Graph::bbox_to_view((10.0, 5.0), (20.0, 2.0));
        assert!((center - 15.0).abs() < 1e-9);
        assert!((window - 10.0).abs() < 1e-9);
        assert!((y_min - 2.0).abs() < 1e-9);
        assert!((y_max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bbox_to_view_reversed_drag() {
        // Bottom-right to top-left should normalise to the same bounds.
        let (center, window, y_min, y_max) = Graph::bbox_to_view((20.0, 2.0), (10.0, 5.0));
        assert!((center - 15.0).abs() < 1e-9);
        assert!((window - 10.0).abs() < 1e-9);
        assert!((y_min - 2.0).abs() < 1e-9);
        assert!((y_max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bbox_to_view_degenerate_does_not_panic() {
        // Zero-area rectangle. Helper must not produce NaN — caller gates on
        // a minimum pixel size, so a zero window reaching this helper is a
        // theoretical edge case but we still want sane arithmetic.
        let (center, window, y_min, y_max) = Graph::bbox_to_view((5.0, 3.0), (5.0, 3.0));
        assert!((center - 5.0).abs() < 1e-9);
        assert!(window.abs() < 1e-9);
        assert!((y_min - 3.0).abs() < 1e-9);
        assert!((y_max - 3.0).abs() < 1e-9);
        assert!(window.is_finite());
    }

    #[test]
    fn apply_bbox_zoom_sets_state() {
        let mut g = Graph::new();
        assert!(g.live);
        assert!(!g.y_axis_fixed);
        g.apply_bbox_zoom((10.0, 5.0), (20.0, 2.0));
        assert!(!g.live);
        assert!(g.y_axis_fixed);
        assert!(g.y_user_set);
        assert!((g.view_center - 15.0).abs() < 1e-9);
        assert!((g.time_window_secs - 10.0).abs() < 1e-9);
        assert!((g.y_fixed_min - 2.0).abs() < 1e-9);
        assert!((g.y_fixed_max - 5.0).abs() < 1e-9);
        assert_eq!(g.y_min_text, "2.0000");
        assert_eq!(g.y_max_text, "5.0000");
    }

    #[test]
    fn apply_bbox_zoom_clamps_time_window_minimum() {
        // A very narrow drag must not produce a zero-width time window.
        let mut g = Graph::new();
        g.apply_bbox_zoom((10.0, 0.0), (10.0, 1.0));
        assert!(g.time_window_secs >= 0.1);
    }

    #[test]
    fn reset_view_restores_live_and_auto_y() {
        let mut g = Graph::new();
        g.apply_bbox_zoom((10.0, 5.0), (20.0, 2.0));
        assert!(!g.live);
        assert!(g.y_axis_fixed);
        g.reset_view();
        assert!(g.live);
        assert!(!g.y_axis_fixed);
        assert!(!g.y_user_set);
        assert_eq!(g.view_center, 0.0);
    }

    #[test]
    fn visible_index_range_finds_correct_slice() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        // Push 10 points at 1-second intervals: t=0..9
        for i in 0..10 {
            g.push(i as f64, t0 + Duration::from_secs(i), "DC V", "V", None);
        }
        // Ask for points in [3.0, 6.0]
        let (start, end) = g.visible_index_range(3.0, 6.0);
        assert_eq!(start, 3);
        assert_eq!(end, 7); // half-open: indices 3,4,5,6

        // Empty range
        let (s, e) = g.visible_index_range(20.0, 30.0);
        assert_eq!(s, e);

        // Full range
        let (s, e) = g.visible_index_range(0.0, 100.0);
        assert_eq!(s, 0);
        assert_eq!(e, 10);
    }

    #[test]
    fn nearest_point_binary_search() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        // Points at t=0, 1, 2, 3
        for i in 0..4 {
            g.push(
                (i * 10) as f64,
                t0 + Duration::from_secs(i),
                "DC V",
                "V",
                None,
            );
        }
        // Exact match
        let (pt, v) = g.nearest_point(2.0).unwrap();
        assert!((pt - 2.0).abs() < 0.01);
        assert!((v - 20.0).abs() < 0.01);
        // Between points — closer to t=1 (value=10) than t=2 (value=20)
        let (pt, v) = g.nearest_point(1.3).unwrap();
        assert!((pt - 1.0).abs() < 0.01);
        assert!((v - 10.0).abs() < 0.01);
        // Before all data
        let (pt, _) = g.nearest_point(-5.0).unwrap();
        assert!((pt - 0.0).abs() < 0.01);
        // After all data
        let (pt, _) = g.nearest_point(100.0).unwrap();
        assert!((pt - 3.0).abs() < 0.01);
    }

    #[test]
    fn build_envelope_sliding_window_extrema() {
        // Place points one second apart so each subsequent sample is one
        // window step. With window=2.5s, the trailing window at each point
        // covers itself plus the previous two samples (gap of 2s ≤ 2.5s).
        let mut g = Graph::new();
        let t0 = Instant::now();
        let values = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        for (i, &v) in values.iter().enumerate() {
            g.push(v, t0 + Duration::from_secs(i as u64), "DC V", "V", None);
        }

        let (min_pts, max_pts) = g.build_envelope(0.0, 7.0, 2.5);
        assert_eq!(min_pts.len(), values.len());
        assert_eq!(max_pts.len(), values.len());

        // Reference: brute-force the trailing window for every point.
        for i in 0..values.len() {
            let t = i as f64;
            let win_start = t - 2.5;
            let mut bf_min = f64::INFINITY;
            let mut bf_max = f64::NEG_INFINITY;
            for (j, &v) in values.iter().enumerate() {
                let tj = j as f64;
                if tj >= win_start && tj <= t {
                    bf_min = bf_min.min(v);
                    bf_max = bf_max.max(v);
                }
            }
            assert!(
                (min_pts[i][1] - bf_min).abs() < 1e-12,
                "min[{i}]={} expected {bf_min}",
                min_pts[i][1]
            );
            assert!(
                (max_pts[i][1] - bf_max).abs() < 1e-12,
                "max[{i}]={} expected {bf_max}",
                max_pts[i][1]
            );
        }
    }

    #[test]
    fn apply_pan_in_browse_mode_shifts_view_center() {
        let mut g = Graph::new();
        g.live = false;
        g.view_center = 100.0;
        g.apply_pan(5.0);
        assert!((g.view_center - 95.0).abs() < 1e-9);
        assert!(!g.live);
    }

    #[test]
    fn apply_pan_in_live_mode_drops_out_of_live() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V", None);
        assert!(g.live);
        // Any non-zero drag while live flips us to browse mode.
        g.apply_pan(2.0);
        assert!(!g.live);
    }

    #[test]
    fn apply_pan_in_live_mode_snaps_view_center_to_end() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V", None);
        // Zero-delta pan while live still snaps view_center to the end of
        // data — so the view doesn't visibly jump on drag start.
        g.apply_pan(0.0);
        let (_, data_max) = g.data_time_range();
        let expected = data_max - g.time_window_secs / 2.0;
        assert!((g.view_center - expected).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_newer_at_live_edge_returns_to_live() {
        // Browse mode, view right-edge exactly at data_max. A drag toward
        // newer data (time_delta < 0) must snap back to live instead of
        // drifting into empty future-space.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = 50.0;
        // Fake a data_max of 55 by priming origin and history.
        g.origin = Some(Instant::now());
        // Push a point; then override the elapsed calc is hard, so use a
        // simpler setup: set view_center so right edge = 0 and data_max = 0.
        g.view_center = -5.0; // right edge = 0 = data_max (no data → data_max=0)
        g.apply_pan(-1.0); // mouse left = newer; would push right edge past 0
        assert!(g.live);
        // view_center snaps to data_max - half = 0 - 5 = -5.
        assert!((g.view_center - -5.0).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_older_never_triggers_live_snap() {
        // Dragging back into history (time_delta > 0) must never flip live
        // on, even if the starting state is at the live edge.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = -5.0; // right edge at 0 (data_max = 0 with empty history)
        g.apply_pan(3.0);
        assert!(!g.live);
        assert!((g.view_center - -8.0).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_newer_below_live_edge_does_not_snap() {
        // Drag toward newer but still short of the live edge — just moves
        // view_center, does not re-enter live.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = -50.0; // right edge at -45, well below data_max=0
        g.apply_pan(-2.0);
        assert!(!g.live);
        assert!((g.view_center - -48.0).abs() < 1e-9);
    }

    #[test]
    fn time_axis_label_integer_seconds() {
        // Existing behaviour preserved when step ≥ 1s.
        assert_eq!(format_time_axis_label(9.0, 1.0), "9 s");
        assert_eq!(format_time_axis_label(45.0, 5.0), "45 s");
    }

    #[test]
    fn time_axis_label_subsecond_step_adds_decimals() {
        // step=0.1 → 1 decimal; step=0.01 → 2 decimals.
        assert_eq!(format_time_axis_label(9.1, 0.1), "9.1 s");
        assert_eq!(format_time_axis_label(9.25, 0.01), "9.25 s");
        assert_eq!(format_time_axis_label(9.123, 0.001), "9.123 s");
    }

    #[test]
    fn time_axis_label_integer_value_with_subsecond_step_pads_decimals() {
        // A grid mark at an integer second still gets padded when the step
        // is sub-second, so all visible labels line up at the same precision.
        assert_eq!(format_time_axis_label(9.0, 0.1), "9.0 s");
        assert_eq!(format_time_axis_label(10.0, 0.01), "10.00 s");
    }

    #[test]
    fn time_axis_label_minutes_with_subsecond_step() {
        // Zooming into a span past 1 minute while sub-second still shows
        // the decimal seconds portion.
        assert_eq!(format_time_axis_label(90.5, 0.1), "1m 30.5s");
    }

    #[test]
    fn time_axis_label_whole_minute_with_integer_step() {
        // Step ≥ 1s, exact minute → shorthand "N m".
        assert_eq!(format_time_axis_label(120.0, 1.0), "2 m");
    }

    #[test]
    fn time_axis_label_hour_integer_step() {
        assert_eq!(format_time_axis_label(3720.0, 60.0), "1h 2m");
    }

    #[test]
    fn time_axis_label_hour_subsecond_step() {
        // Unlikely in practice but the formatter should not drop the
        // seconds field when hours are involved and step is sub-second.
        let out = format_time_axis_label(3725.5, 0.1);
        assert_eq!(out, "1h 2m 5.5s");
    }

    #[test]
    fn is_view_zoomed_reflects_state() {
        let mut g = Graph::new();
        assert!(!g.is_view_zoomed());
        g.live = false;
        assert!(g.is_view_zoomed());
        g.live = true;
        g.y_axis_fixed = true;
        assert!(g.is_view_zoomed());
    }

    // ── Sub-value overlays ──────────────────────────────────────────────

    /// Push a sample carrying sub-values. Everything but the parts under
    /// test matches the single-series `push` helper.
    fn push_aux(
        g: &mut Graph,
        value: f64,
        t: Instant,
        series: Option<&str>,
        overlays: &[(&str, Option<f64>)],
    ) {
        g.push_sample(PlotSample {
            value,
            timestamp: t,
            mode: "Temp",
            unit: "\u{00B0}C",
            display_raw: None,
            series,
            overlays,
        });
    }

    /// The overlay buffers are indexed by history position, so every push has
    /// to extend them by exactly one — including the pushes that arrive
    /// before the sub-value is first seen.
    #[test]
    fn overlays_stay_in_lockstep_with_history() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..3 {
            push_aux(
                &mut g,
                20.0 + i as f64,
                t0 + Duration::from_secs(i),
                None,
                &[("T2", Some(30.0 + i as f64))],
            );
        }
        assert_eq!(g.len(), 3);
        assert_eq!(
            g.overlay_values("T2"),
            vec![Some(30.0), Some(31.0), Some(32.0)]
        );
    }

    /// Eviction has to drop the overlay's oldest value with the point it
    /// belongs to, or every overlay would drift one sample later than the
    /// trace it accompanies.
    #[test]
    fn max_points_evicts_overlay_values_too() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..MAX_POINTS + 100 {
            push_aux(
                &mut g,
                i as f64,
                t0 + Duration::from_millis(i as u64 * 10),
                None,
                &[("T2", Some(i as f64 + 0.5))],
            );
        }
        let values = g.overlay_values("T2");
        assert_eq!(values.len(), MAX_POINTS);
        assert_eq!(g.len(), MAX_POINTS);
        assert_eq!(values.first().copied().flatten(), Some(100.5));
    }

    /// A COMP High/Low or a MIN/MAX sub-value can start mid-session. Without
    /// the back-fill its first value would land at index 0 and the whole
    /// trace would be drawn shifted back in time.
    #[test]
    fn a_late_overlay_is_back_filled_with_nothing() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 20.0, t0, None, &[]);
        push_aux(&mut g, 21.0, t0 + Duration::from_secs(1), None, &[]);
        push_aux(
            &mut g,
            22.0,
            t0 + Duration::from_secs(2),
            None,
            &[("Max", Some(22.0))],
        );
        assert_eq!(g.overlay_values("Max"), vec![None, None, Some(22.0)]);
        assert_eq!(g.overlay_segments("Max"), vec![vec![[2.0, 22.0]]]);
    }

    /// An over-range sub-value breaks its own trace and nothing else: the
    /// plotted series still has a value for that frame, so it stays whole.
    #[test]
    fn a_missing_overlay_value_splits_only_that_overlay() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
        push_aux(
            &mut g,
            21.0,
            t0 + Duration::from_secs(1),
            None,
            &[("T2", None)],
        );
        push_aux(
            &mut g,
            22.0,
            t0 + Duration::from_secs(2),
            None,
            &[("T2", Some(32.0))],
        );

        assert_eq!(
            g.overlay_segments("T2"),
            vec![vec![[0.0, 30.0]], vec![[2.0, 32.0]]]
        );
        assert_eq!(g.all_segments().len(), 1, "main trace must stay whole");
        assert!(g.visible_gaps().is_empty());
    }

    /// Overlays have to split wherever the main trace splits, or they would
    /// be drawn straight across an overload the meter reported.
    #[test]
    fn a_break_in_the_plotted_series_splits_the_overlays() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
        g.push_break(t0 + Duration::from_millis(500));
        push_aux(
            &mut g,
            22.0,
            t0 + Duration::from_secs(1),
            None,
            &[("T2", Some(32.0))],
        );
        assert_eq!(
            g.overlay_segments("T2"),
            vec![vec![[0.0, 30.0]], vec![[1.0, 32.0]]]
        );
    }

    /// T1 and T2 share a mode *and* a unit, so nothing but the series label
    /// distinguishes them — switching would otherwise append onto the
    /// previous sub-value's trace.
    #[test]
    fn a_series_change_restarts_the_trace() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
        push_aux(
            &mut g,
            21.0,
            t0 + Duration::from_secs(1),
            None,
            &[("T2", Some(31.0))],
        );
        assert_eq!(g.len(), 2);

        push_aux(
            &mut g,
            31.5,
            t0 + Duration::from_secs(2),
            Some("T2"),
            &[("Main", Some(21.5))],
        );
        assert_eq!(g.len(), 1, "switching series must clear the history");
        assert_eq!(g.overlay_labels(), vec!["Main"]);
        assert_eq!(g.current_series.as_deref(), Some("T2"));
    }

    /// Same series, same mode, same unit: nothing to reset.
    #[test]
    fn a_steady_series_keeps_its_history() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 30.0, t0, Some("T2"), &[]);
        push_aux(&mut g, 31.0, t0 + Duration::from_secs(1), Some("T2"), &[]);
        assert_eq!(g.len(), 2);
    }

    /// The toolbar selection survives as long as the meter keeps offering
    /// that sub-value, and falls back to the main reading when it stops —
    /// the meter left the mode that produced it.
    #[test]
    fn a_selection_is_dropped_once_its_label_is_no_longer_offered() {
        let mut g = Graph::new();
        g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")]);
        g.selected_series = Some("T2".to_string());

        g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")]);
        assert_eq!(g.selected_series(), Some("T2"));

        g.set_series_options(&[("Frequency", "Hz")]);
        assert_eq!(g.selected_series(), None);
    }

    /// A single-display meter keeps the two-row toolbar: the series row only
    /// appears once there is a sub-value to select or a trace to draw.
    #[test]
    fn the_series_row_appears_only_once_there_is_something_in_it() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(20.0, t0, "DC V", "V", None);
        assert!(!g.has_series_row());

        // A selectable sub-value alone is enough...
        g.set_series_options(&[("Frequency", "Hz")]);
        assert!(g.has_series_row());

        // ...and so is a same-unit trace with nothing to select.
        let mut g = Graph::new();
        push_aux(&mut g, 20.0, t0, None, &[("T2", Some(50.0))]);
        assert!(g.series_options.is_empty());
        assert!(g.has_series_row());
    }

    /// A sub-value drawn outside the auto Y range would be clipped to the
    /// plot edge, which reads as a flat line rather than as data.
    #[test]
    fn the_y_range_frames_the_visible_overlays() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..3 {
            push_aux(
                &mut g,
                20.0,
                t0 + Duration::from_secs(i),
                None,
                &[("T2", Some(50.0))],
            );
        }
        let (lo, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
        assert!(lo <= 20.0 && hi >= 50.0, "overlay not framed: {lo}..{hi}");

        // Off-view overlay points must not stretch the axis.
        let (lo, hi) = g.y_min_max_padded(0.0, 0.5, true).expect("range");
        assert!(
            hi < 60.0,
            "range {lo}..{hi} reached beyond the visible slice"
        );
    }

    /// The minimap is a main-series overview and scans the whole history, so
    /// it must not multiply that scan by the overlay count.
    #[test]
    fn the_minimap_y_range_ignores_overlays() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..3 {
            push_aux(
                &mut g,
                20.0,
                t0 + Duration::from_secs(i),
                None,
                &[("T2", Some(50.0))],
            );
        }
        let (lo, hi) = g.y_min_max_padded(0.0, 2.0, false).expect("range");
        assert!(hi < 30.0, "minimap range {lo}..{hi} followed the overlay");
    }

    /// The protocols send at most four sub-values per frame; anything beyond
    /// that is a bug upstream and must not grow the buffers unbounded.
    #[test]
    fn a_fifth_overlay_label_is_ignored() {
        let mut g = Graph::new();
        push_aux(
            &mut g,
            20.0,
            Instant::now(),
            None,
            &[
                ("A", Some(1.0)),
                ("B", Some(2.0)),
                ("C", Some(3.0)),
                ("D", Some(4.0)),
                ("E", Some(5.0)),
            ],
        );
        assert_eq!(g.overlay_labels(), vec!["A", "B", "C", "D"]);
    }

    // ── Plot key and Show: toggles ──────────────────────────────────────

    /// Names in the key, in the order they are painted.
    fn key_names(g: &Graph) -> Vec<String> {
        let drawn = g.visible_overlay_traces(0, g.len());
        g.key_entries(&drawn)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    /// Labels of the traces that are actually drawn.
    fn drawn_overlay_labels(g: &Graph) -> Vec<String> {
        g.visible_overlay_traces(0, g.len())
            .into_iter()
            .map(|(_, label, _)| label)
            .collect()
    }

    /// Fill a graph with a main trace plus two same-unit sub-values.
    fn graph_with_two_overlays() -> Graph {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..3 {
            push_aux(
                &mut g,
                20.0,
                t0 + Duration::from_secs(i),
                None,
                &[("T2", Some(50.0)), ("T3", Some(60.0))],
            );
        }
        g
    }

    /// A single-display meter must look exactly as it did before sub-values
    /// existed: no key painted over the plot at all.
    #[test]
    fn no_key_without_overlays() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        for i in 0..3 {
            g.push(20.0, t0 + Duration::from_secs(i), "DC V", "V", None);
        }
        assert!(key_names(&g).is_empty());
    }

    /// The key names the plotted series first, then each drawn overlay in
    /// the order the meter offered them.
    #[test]
    fn the_key_names_the_plotted_series_then_its_overlays() {
        let g = graph_with_two_overlays();
        assert_eq!(key_names(&g), vec!["Main", "T2", "T3"]);

        let drawn = g.visible_overlay_traces(0, g.len());
        let styles: Vec<KeyStyle> = g
            .key_entries(&drawn)
            .into_iter()
            .map(|(_, style)| style)
            .collect();
        assert_eq!(
            styles,
            vec![
                KeyStyle::Plotted,
                KeyStyle::Overlay(0),
                KeyStyle::Overlay(1)
            ]
        );
    }

    /// Hiding a trace from the Show: chips must take it off the plot, out of
    /// the key, and out of the Y range — the axis stretching to frame a trace
    /// that isn't drawn would flatten the one that is.
    #[test]
    fn hiding_an_overlay_drops_it_from_the_key_the_plot_and_the_y_range() {
        let mut g = graph_with_two_overlays();
        let (_, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
        assert!(hi >= 60.0, "both overlays framed to start with: {hi}");

        g.toggle_overlay_hidden("T3".to_string());

        assert_eq!(key_names(&g), vec!["Main", "T2"]);
        assert_eq!(drawn_overlay_labels(&g), vec!["T2"]);
        let (_, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
        assert!(hi < 60.0, "hidden overlay still stretching the axis: {hi}");
    }

    /// Hidden means not drawn, not not-recorded: turning a trace back on has
    /// to bring its history with it. The chip flips both ways.
    #[test]
    fn a_hidden_overlay_is_still_recorded() {
        let mut g = graph_with_two_overlays();
        g.toggle_overlay_hidden("T3".to_string());
        assert_eq!(g.overlay_values("T3"), vec![Some(60.0); 3]);
        assert!(drawn_overlay_labels(&g).iter().all(|l| l != "T3"));

        g.toggle_overlay_hidden("T3".to_string());
        assert_eq!(g.overlay_segments("T3").len(), 1);
        assert_eq!(drawn_overlay_labels(&g), vec!["T2", "T3"]);
    }

    /// Every overlay hidden is the same as no overlay drawn: no key.
    #[test]
    fn hiding_every_overlay_removes_the_key() {
        let mut g = graph_with_two_overlays();
        g.hidden_overlays.insert("T2".to_string());
        g.hidden_overlays.insert("T3".to_string());
        assert!(key_names(&g).is_empty());
        assert!(drawn_overlay_labels(&g).is_empty());
    }

    /// The choice is about the sub-value, not about the buffer holding it:
    /// clearing the data or switching the plotted series must not silently
    /// bring a hidden trace back.
    #[test]
    fn hidden_overlays_survive_clear_and_a_series_change() {
        let mut g = graph_with_two_overlays();
        g.hidden_overlays.insert("T3".to_string());

        g.clear();
        assert!(g.hidden_overlays.contains("T3"));

        let t0 = Instant::now();
        push_aux(&mut g, 50.0, t0, Some("T2"), &[("T3", Some(60.0))]);
        assert!(g.hidden_overlays.contains("T3"));
        assert!(drawn_overlay_labels(&g).is_empty(), "T3 must stay hidden");
    }

    /// A sub-value can stop and start again (a COMP limit, a MIN/MAX reset).
    /// The hidden set is keyed by label so the user's choice outlives the
    /// buffer, rather than the trace reappearing the moment the meter re-sends
    /// it.
    #[test]
    fn a_label_that_vanishes_and_returns_is_still_hidden() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        push_aux(&mut g, 20.0, t0, None, &[("T2", Some(50.0))]);
        g.hidden_overlays.insert("T2".to_string());
        assert!(drawn_overlay_labels(&g).is_empty());

        // The meter leaves the mode: the graph resets and T2 goes away.
        g.push(1.0, t0 + Duration::from_secs(1), "DC V", "V", None);
        assert!(g.overlay_labels().is_empty());

        // It comes back later.
        push_aux(
            &mut g,
            20.0,
            t0 + Duration::from_secs(2),
            None,
            &[("T2", Some(50.0))],
        );
        assert_eq!(g.overlay_labels(), vec!["T2"]);
        assert!(
            drawn_overlay_labels(&g).is_empty(),
            "the user hid T2; it must not come back on its own"
        );
    }

    /// Ctrl+L clears the data, not the user's choice of what to plot — the
    /// meter is still in the same mode and still sending the same sub-value.
    #[test]
    fn clear_drops_the_overlays_but_keeps_the_selection() {
        let mut g = Graph::new();
        g.set_series_options(&[("T2", "\u{00B0}C")]);
        g.selected_series = Some("T2".to_string());
        push_aux(
            &mut g,
            30.0,
            Instant::now(),
            Some("T2"),
            &[("Main", Some(20.0))],
        );
        assert_eq!(g.overlay_labels(), vec!["Main"]);

        g.clear();
        assert!(g.is_empty());
        assert!(g.overlay_labels().is_empty());
        assert_eq!(g.current_series, None);
        assert_eq!(g.selected_series(), Some("T2"));
    }
}
