//! Real-time scrolling graph: the history buffer the App pushes samples into,
//! and the widget that draws it.
//!
//! The concerns live in submodules. These add methods to the one [`Graph`]
//! declared here, so callers see a single type: [`ingest`] (taking readings
//! in), [`view`] (what slice is shown and the gestures that move it),
//! [`view_state`] (the view as an export saves it), [`toolbar`], [`render`]
//! (the main plot), [`hover`] (the readings under the pointer), [`labels`]
//! (the text painted over the plot), [`key`] (the plot key), [`menu`] (the
//! right-click menu), [`limits`] (the alarm limits drawn), [`minimap`],
//! [`analysis`] (visible-slice statistics) and [`figures`] (the caches the
//! statistics and the histogram read). These hold types and functions of
//! their own: [`breaks`] (the interruption the trace is in before its next
//! point), [`pattern`] (its dashed and dotted lines), [`axes`] (the Y axes
//! of sub-values in other units), [`level`] (the minimap's bucketed trace),
//! [`histogram`] (the histogram pane), [`field`] (the toolbar's text-edit
//! buffers and the values they parse to) and [`time`] (axis label
//! formatting).
//!
//! Drawing has two tiers, so a frame costs the same an hour into a session as
//! a minute in. The minimap shows the whole history, so it reads an
//! incremental min/max level ([`level`]) rather than the points. Everything
//! else — the main plot, the statistics, the Y range, the envelope, the
//! crossings — works on the visible slice, found by binary search, and never
//! scans the whole history.

mod analysis;
mod axes;
mod breaks;
mod field;
mod figures;
pub(crate) mod histogram;
mod hover;
mod ingest;
mod key;
mod labels;
mod level;
mod limits;
mod menu;
mod minimap;
mod pattern;
mod render;
mod time;
mod toolbar;
mod view;
mod view_state;

pub(crate) use limits::{LimitSide, WatchedLimits};
pub(crate) use menu::{MenuAction, ReadingDigits};
pub(crate) use view_state::ViewState;

#[cfg(test)]
mod tests;

use dmm_lib::measurement::MainLabel;
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};
use std::borrow::Cow;
use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::markers::{Marker, Markers};
use crate::settings::DEFAULT_MAX_SAMPLES;
use crate::theme::ThemeColors;
use axes::AxisMap;
use breaks::Breaks;
use field::{NumberField, NumberListField};
use figures::{BinnedHistogram, RangeFigures, strip_room};
use level::MinimapLevel;
use menu::MenuPoint;
use minimap::{MINIMAP_HEIGHT, MinimapDrag};
use view::{BoxZoom, YAxis};

/// Maximum number of sub-value traces the graph keeps beside the plotted
/// series, in its unit or another.
///
/// The protocols send at most four sub-values per frame (UT181A), so this is
/// the ceiling the wire imposes rather than a display choice.
pub(crate) const MAX_OVERLAYS: usize = 4;

/// Most units drawn at once, each on its own Y axis: the plotted series' on
/// the left, the others on the right. Four leaves room for a scaled
/// reading's Raw beside a UT181A's V, Hz and ms; more axes would squeeze
/// the plot.
pub(crate) const MAX_AXES: usize = 4;

/// What the meter's main reading is called when the meter gives it no name
/// of its own: on the **Plot:** chip, in the key, and as the trace drawn
/// beside a plotted sub-value. The graph's key for that trace, in English
/// whatever the language: what is drawn is [`Graph::series_text`].
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

/// Width kept for the plotted unit's axis, and the narrowest plot worth
/// giving up the right axes for, in points. Their one column is reckoned at
/// [`RIGHT_AXIS_ROOM`]: a label such as "49.98 Hz" plus margins.
const LEFT_AXIS_ROOM: f32 = 60.0;
const MIN_PLOT_WIDTH: f32 = 240.0;
const RIGHT_AXIS_ROOM: f32 = 90.0;
/// The narrowest plot the histogram strip is put beside: narrower, the strip
/// would take a fifth of a small screen's plot, and the histogram goes under
/// the minimap instead. The strip narrows to its least before it moves.
const MIN_PLOT_BESIDE_STRIP: f32 = 400.0;
// A plot roomy enough for the strip leaves the right axes their column.
const _: () = assert!(MIN_PLOT_BESIDE_STRIP >= MIN_PLOT_WIDTH + RIGHT_AXIS_ROOM);

/// Why a sub-value's unit gets no Y axis: see [`Graph::axis_refusal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisRefusal {
    /// [`MAX_AXES`] units are drawn already.
    Full,
    /// The window is too narrow for its axis.
    Narrow,
}

/// Gap threshold multiplier: gap = max(interval * multiplier, minimum), the
/// same multiple the session integral bridges.
const GAP_MULTIPLIER: f64 = dmm_lib::stats::GAP_INTERVALS;
const GAP_MINIMUM_SECS: f64 = 1.0;

/// Why the trace is interrupted over a stretch of time.
///
/// The distinction is user-visible: an overload is the meter reporting a
/// condition, while a data gap is the absence of any report at all. They are
/// drawn differently — see `MainFrame::draw_behind_trace` and
/// `MainFrame::draw_over_trace` in `render.rs`.
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
    /// Over-range readings since the trace began, up to this point: the
    /// histogram counts those between two points as the difference.
    overloads_before: u64,
}

/// One point of a sub-value trace, at the time of the frame that carried it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OverlayPoint {
    time: Instant,
    /// `None` breaks the trace here: the sub-value was over range, or the
    /// link was lost (`push_data_loss`).
    value: Option<f64>,
}

/// One sub-value trace kept beside the plotted series: drawn when it is in
/// the plotted unit, kept in the background otherwise so that **Plot:** can
/// switch to it with its past.
///
/// Its points keep their own times rather than riding on the plotted
/// series' points: a meter that sends the parts of one reading in frames of
/// their own (the UT61E+'s AC+DC V) puts each at its own moment, and a frame
/// without the plotted series still carries them. A point is appended only
/// for frames that carry the label, so the single-display meters that make up
/// most of the device table pay nothing.
struct OverlaySeries {
    label: String,
    /// What its points measure in. A trace in another unit than the plotted
    /// series is drawn on an axis of its own: see [`Graph::axis_units`].
    unit: String,
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
    /// name kept beside it, or restarts the graph when there is none: two
    /// sub-values can share a mode and a unit (T1 and T2), so nothing else
    /// would tell them apart.
    pub series: Option<&'a str>,
    /// The meter's name for its main reading ("DC" beside an "AC" part), or
    /// `None` for [`MAIN_SERIES`].
    pub main_label: Option<MainLabel>,
    /// The plotted series is a level — an NCV reading's bars — rather than a
    /// quantity: its axis steps in whole numbers and its readouts drop the
    /// decimals.
    pub levels: bool,
    /// The frame's other series, as (label, unit, value), drawn beside the
    /// plotted series — those in another unit than `unit` on axes of their
    /// own. `None` for an over-range sub-value — it breaks that trace without
    /// breaking the others.
    pub overlays: &'a [(&'a str, &'a str, Option<f64>)],
}

/// One of the two measurement cursors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cursor {
    A,
    B,
}

impl Cursor {
    /// The cursor's letter, as its line is labelled.
    fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

/// The measurement cursors: two vertical lines with a ΔT/ΔV readout, at
/// times in seconds from the origin.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MeasureCursors {
    /// A click on the plot places a cursor.
    pub(crate) active: bool,
    /// Each cursor's time; `None` until placed.
    a: Option<f64>,
    b: Option<f64>,
    /// Which cursor the next click places.
    next_is_b: bool,
}

impl MeasureCursors {
    /// Both cursors' times, once both are placed while the cursors are on.
    fn pair(&self) -> Option<(f64, f64)> {
        self.active.then_some((self.a?, self.b?))
    }

    /// The placed cursors' times, none while the cursors are off.
    fn placed(&self) -> impl Iterator<Item = f64> {
        let shown = if self.active {
            [self.a, self.b]
        } else {
            [None, None]
        };
        shown.into_iter().flatten()
    }

    /// Switch the cursors on with A and B at `a` and `b`. With only A
    /// placed, the next click places B, as if A had just been clicked.
    fn restore(&mut self, a: Option<f64>, b: Option<f64>) {
        *self = Self {
            active: true,
            a,
            b,
            next_is_b: a.is_some() && b.is_none(),
        };
    }

    /// Place the next cursor at `t`; the click after places the other.
    fn place(&mut self, t: f64) {
        if self.next_is_b {
            self.b = Some(t);
        } else {
            self.a = Some(t);
        }
        self.next_is_b = !self.next_is_b;
    }

    /// The time of cursor `which`, while the cursors are on.
    fn at(&self, which: Cursor) -> Option<f64> {
        let t = match which {
            Cursor::A => self.a,
            Cursor::B => self.b,
        };
        t.filter(|_| self.active)
    }

    /// Switch the cursors on with `which` at `t`; the next click places
    /// the other one.
    fn set(&mut self, which: Cursor, t: f64) {
        self.active = true;
        match which {
            Cursor::A => self.a = Some(t),
            Cursor::B => self.b = Some(t),
        }
        self.next_is_b = which == Cursor::A;
    }

    /// Drop cursor `which`; the next click places it again. With the last
    /// one gone the cursors go off, as the chip leaves them.
    fn remove(&mut self, which: Cursor) {
        match which {
            Cursor::A => self.a = None,
            Cursor::B => self.b = None,
        }
        self.next_is_b = which == Cursor::B;
        if self.a.is_none() && self.b.is_none() {
            *self = Self::default();
        }
    }

    /// Drop both cursors; the next click places A.
    fn clear(&mut self) {
        self.a = None;
        self.b = None;
        self.next_is_b = false;
    }

    /// Flip the cursors, dropping any placed cursor when they go off so that
    /// switching them back on starts a fresh measurement.
    fn toggle(&mut self) {
        self.active = !self.active;
        if !self.active {
            self.clear();
        }
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

/// Real-time scrolling graph with minimap navigation.
pub struct Graph {
    // -- Recorded trace --
    history: VecDeque<DataPoint>,
    /// Points the history keeps before the oldest are dropped. Shared with
    /// the recording buffer through the Buffer size setting, and changed
    /// under a running session by [`Graph::set_max_points`].
    max_points: usize,
    /// Samples pushed since the trace last restarted, so that a point keeps
    /// an identity across eviction: `history[0]` is
    /// `pushed_total - history.len()`. The level's bands are dropped by it.
    pushed_total: u64,
    /// Bumped whenever the history is replaced or restarts, when sequence
    /// numbers start over on other readings: see [`ReadingsKey`](figures::ReadingsKey).
    history_epoch: u64,
    origin: Option<Instant>,
    /// Sub-value traces, each point at its own frame's time: the ones in the
    /// plotted unit drawn, the others kept (see [`Graph::drawn`]).
    overlays: Vec<OverlaySeries>,
    /// The interruption since the last plotted point, if any, which the
    /// next one records as its break.
    breaks: Breaks,
    /// Stretches in which no frame of any kind arrived for longer than the gap
    /// threshold in force at the time, oldest first, as (last frame before,
    /// first frame after).
    ///
    /// A trace breaks for want of data only across one of these — not merely
    /// because two of its own points are far apart. A meter that sends a
    /// reading's parts in turn (the UT61E+'s AC+DC V, answering every 0.67 s)
    /// spaces each part's points wider than the threshold while it never goes
    /// quiet at all. Judged when the silence ends, against the interval the
    /// readings were taken at: a sample interval changed later does not turn
    /// the spacing of a slower stretch into gaps after the fact. There are
    /// only as many as the meter had real silences.
    silences: VecDeque<(Instant, Instant)>,
    /// Timestamp of the newest frame of any kind: a point, a frame of
    /// sub-values only, an overload or a word shown instead of a reading.
    last_heard: Option<Instant>,
    /// Gap detection threshold in seconds.
    gap_threshold_secs: f64,
    /// The threshold the silence since the last frame is judged by: the
    /// largest in force since that frame, so the stretch across a change to a
    /// shorter interval, which began at the slower one, is no gap.
    silence_threshold_secs: f64,
    /// Frames of sub-values only since the last point of the plotted series;
    /// see [`Graph::stopped_for`].
    main_missing_frames: u32,

    // -- Plotted series --
    current_mode: Option<String>,
    current_unit: String,
    /// Label of the series `history` was recorded from; `None` for the main
    /// reading. Distinct from `selected_series`: this one only moves when a
    /// sample actually arrives for the new choice.
    current_series: Option<String>,
    /// The meter's name for its main reading, from the latest sample; `None`
    /// for [`MAIN_SERIES`].
    main_label: Option<MainLabel>,
    /// The plotted series is a level, from the latest point: see
    /// [`PlotSample::levels`].
    levels: bool,
    /// Last `display_raw` string from the latest pushed measurement, kept
    /// for the a11y plot summary so screen readers hear the same digits the
    /// sighted user sees (e.g. "1.234 mV" instead of the raw f64 value
    /// printed at fixed precision, which mis-scales auto-range readings).
    last_display_raw: Option<String>,

    // -- Series choice: the toolbar's Plot: and Show: groups --
    /// Sub-values the meter is sending, with their resolved units, in the
    /// order first offered. Drives the toolbar's selector.
    series_options: Vec<SeriesOption>,
    /// Which series the toolbar is asking for. Session-only — a selection is
    /// about the meter's current mode, so persisting it across restarts would
    /// silently plot a sub-value the next session may not even have. Dropped
    /// with its option, see [`Graph::set_series_options`].
    selected_series: Option<String>,
    /// Sub-value labels switched off in the toolbar's **Show:** group, seeded
    /// from the settings at start-up (see [`Graph::take_trace_choice`]).
    /// Keyed by label rather than by index so a choice survives `clear()`, a
    /// change of plotted series, a sub-value that disappears and comes back,
    /// and a relaunch. Hidden overlays are still recorded in lockstep —
    /// re-showing one brings its history with it.
    hidden_overlays: HashSet<String>,
    /// The main reading's own trace switched off, while a sub-value is
    /// plotted. Session-only and apart from `hidden_overlays`, whose
    /// remembered labels can name another mode's main reading: a UT181A
    /// hiding its T2 sub-value must still draw T2 where T2 is the main one.
    main_hidden: bool,
    /// Sub-value lines drawn solid rather than dashed and dotted, as the
    /// user chose in Settings → Graph lines.
    pub(crate) solid_lines: bool,

    // -- View --
    /// Time window width in seconds for the main view.
    pub time_window_secs: f64,
    /// When true, main graph auto-scrolls to latest data.
    pub live: bool,
    /// User-controlled view center (seconds from origin). Used when not live.
    view_center: f64,
    /// The width turning LIVE off froze the view at, while the session was
    /// shorter than the window: live shows it without the empty rest of the
    /// window, and the frozen view, panned or not, keeps that width. A zoom,
    /// a window change or a reset clears it.
    frozen_width: Option<f64>,
    /// Fixed Y ranges, while **Y:Fixed** is on.
    y_axis: YAxis,
    pub(crate) cursors: MeasureCursors,

    // -- Guide lines --
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

    // -- Histogram pane --
    /// Show the histogram pane.
    pub show_histogram: bool,
    /// The strip's width and the band's height as last dragged. The app sets
    /// them from its settings at start, and saves them when
    /// [`Graph::take_histogram_resize`] reports a drag.
    pub(crate) histogram_sizes: (f32, f32),
    /// Where the pane goes this frame, `None` while it is off, the main
    /// trace is hidden or the window is too short: see [`histogram::place`].
    histogram_place: Option<histogram::Placement>,

    // -- Carried from the last frame --
    /// How many units' axes fit the graph's width, from the last frame: see
    /// [`Graph::axis_units`].
    axis_room: usize,
    /// This frame's right axes, by unit, for a box zoom to pin each to its
    /// share of the box: see [`Graph::apply_bbox_zoom`].
    axis_maps: Vec<(String, AxisMap)>,
    /// The main plot's area at the last frame, where the marker flags are
    /// placed while the time labels they cover are chosen.
    plot_rect: Option<egui::Rect>,
    /// The main plot's transform at the last frame, which the histogram
    /// strip reads its bar heights from.
    plot_transform: Option<egui_plot::PlotTransform>,
    /// Where the dashed and dotted lines' patterns stand, so they move with
    /// the data: see [`pattern`].
    dash_phases: pattern::DashPhases,

    // -- Caches, redone when what they cover changes --
    /// The cursor readout's figures and the visible window's, computed
    /// again only when the readings they cover change.
    cursor_figures: Option<RangeFigures>,
    visible_figures: Option<RangeFigures>,
    /// This frame's bins, cut by `update_histogram` over its Y range.
    histogram: Option<BinnedHistogram>,
    /// The minimap's whole-session trace, bucketed at whatever width its
    /// strip last asked for. `None` until the strip first draws, and again
    /// whenever the buckets stop meaning what they did: a `clear` (a trace
    /// restart included), an auto-range rescale, a swap of the plotted
    /// series, or a lower `set_max_points`.
    minimap_level: Option<MinimapLevel>,
    /// Cached AccessKit label for the plot — rebuilt only when state changes.
    a11y_label: String,
    /// Signature of the state used to build `a11y_label`, for change detection.
    a11y_label_sig: u64,

    // -- Gestures in progress --
    /// Current minimap drag state.
    minimap_drag: MinimapDrag,
    /// The Shift+drag box zoom in progress, if any.
    box_zoom: Option<BoxZoom>,
    /// The touchpad scroll in progress over the plot or the minimap.
    swipe: view::SwipeLatch,
    /// What the plot's right-click menu was opened on.
    menu: Option<MenuPoint>,
    /// Put the focus on the menu's first entry again next frame.
    menu_focus_pending: bool,

    // -- Requests for the app, collected by the take_* methods --
    /// A **Show:** chip the user clicked since the last call, with whether
    /// it is now hidden — see [`Graph::take_trace_choice`].
    trace_choice: Option<(String, bool)>,
    /// The marker whose flag was clicked this frame, for the app to open its
    /// note — see [`Graph::take_clicked_marker`].
    clicked_marker: Option<u32>,
    /// What the plot's menu asked of the app — see
    /// [`Graph::take_menu_action`].
    menu_action: Option<MenuAction>,
    histogram_resized: bool,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            // Not the bound: half a million points is 24 MB claimed up front
            // for a session that may last a minute. The deque doubles a
            // handful of times over an afternoon instead.
            history: VecDeque::with_capacity(1024),
            max_points: DEFAULT_MAX_SAMPLES,
            pushed_total: 0,
            history_epoch: 0,
            origin: None,
            overlays: Vec::new(),
            breaks: Breaks::default(),
            silences: VecDeque::new(),
            last_heard: None,
            gap_threshold_secs: GAP_MINIMUM_SECS,
            silence_threshold_secs: GAP_MINIMUM_SECS,
            main_missing_frames: 0,
            current_mode: None,
            current_unit: String::new(),
            current_series: None,
            main_label: None,
            levels: false,
            last_display_raw: None,
            series_options: Vec::new(),
            selected_series: None,
            hidden_overlays: HashSet::new(),
            main_hidden: false,
            solid_lines: false,
            time_window_secs: 60.0,
            live: true,
            view_center: 0.0,
            frozen_width: None,
            y_axis: YAxis::default(),
            cursors: MeasureCursors::default(),
            show_mean: false,
            show_envelope: false,
            envelope_window: NumberField::new("1", 1.0),
            show_ref_line: false,
            show_crossings: true,
            ref_lines: NumberListField::default(),
            focus_ref_field: false,
            show_histogram: false,
            histogram_sizes: (histogram::STRIP_WIDTH, histogram::BAND_HEIGHT),
            histogram_place: None,
            axis_room: MAX_AXES,
            axis_maps: Vec::new(),
            plot_rect: None,
            plot_transform: None,
            dash_phases: pattern::DashPhases::default(),
            cursor_figures: None,
            visible_figures: None,
            histogram: None,
            minimap_level: None,
            a11y_label: String::new(),
            a11y_label_sig: 0,
            minimap_drag: MinimapDrag::None,
            box_zoom: None,
            swipe: view::SwipeLatch::default(),
            menu: None,
            menu_focus_pending: false,
            trace_choice: None,
            clicked_marker: None,
            menu_action: None,
            histogram_resized: false,
        }
    }

    /// Sub-value traces kept beside the plotted series, drawn or not — what
    /// the Buffer size hint multiplies its per-point cost by.
    pub fn overlays_len(&self) -> usize {
        self.overlays.len()
    }

    /// What the main reading goes by: the meter's name for it, or
    /// [`MAIN_SERIES`].
    pub(crate) fn main_name(&self) -> &'static str {
        self.main_label.map_or(MAIN_SERIES, MainLabel::as_str)
    }

    /// What the main reading is called on screen: [`Graph::main_name`] in
    /// `l10n`'s language.
    pub(crate) fn main_text<'a>(&self, l10n: &'a Localizer) -> Cow<'a, str> {
        match self.main_label {
            Some(label) => l10n.main_label(label),
            None => tr!(l10n, "graph-plot-main"),
        }
    }

    /// What series `name` — [`Graph::main_name`] or a sub-value's label, as
    /// the graph keys its traces and the settings remember them — is called
    /// on screen.
    pub(crate) fn series_text<'a>(&self, l10n: &'a Localizer, name: &'a str) -> Cow<'a, str> {
        if name == self.main_name() {
            self.main_text(l10n)
        } else {
            l10n.label(TextKind::Aux, name)
        }
    }

    /// Make the **Plot:** choice the plotted series now, where its trace is
    /// kept beside the plotted one; `false` when it is not, and the next
    /// sample restarts the graph on it.
    pub(crate) fn swap_to_selected(&mut self) -> bool {
        let series = self.selected_series.clone();
        if self.current_series == series {
            return false;
        }
        let name = series.as_deref().unwrap_or(self.main_name());
        let Some(unit) = self
            .overlays
            .iter()
            .find(|o| o.label == name)
            .map(|o| o.unit.clone())
        else {
            return false;
        };
        self.swap_plotted_series(series.as_deref(), &unit)
    }

    /// Plot `series`, in `unit`, in place of the current one by swapping the
    /// two traces: the one of that name kept beside the plotted series becomes
    /// the plotted one, and the plotted one is kept beside it under its own
    /// name. `false`, touching nothing, when no such trace is kept in `unit`.
    ///
    /// The time axis and the cursors still apply. A switch to another unit
    /// moves the Y axis to it and releases a pinned Y range, chosen for the
    /// old unit's scale. A break in a sub-value's trace carries no reason, so
    /// on becoming the plotted series it is drawn as a gap, an over-range
    /// stretch included.
    fn swap_plotted_series(&mut self, series: Option<&str>, unit: &str) -> bool {
        let incoming_name = series.unwrap_or(self.main_name());
        let Some(i) = self
            .overlays
            .iter()
            .position(|o| o.label == incoming_name && o.unit == unit)
        else {
            return false;
        };
        let incoming = std::mem::take(&mut self.overlays[i].points);
        let incoming_missing = self.overlays[i].missing_frames;

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
        if let (Some(_), Some(time)) = (self.breaks.kind, prev) {
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
                    overloads_before: 0,
                }),
                None => pending = Some(GapKind::NoData),
            }
        }
        self.breaks = Breaks {
            kind: pending,
            ..Breaks::default()
        };

        self.overlays[i] = OverlaySeries {
            label: self
                .current_series
                .take()
                .unwrap_or_else(|| self.main_name().to_string()),
            unit: std::mem::replace(&mut self.current_unit, unit.to_string()),
            points: outgoing,
            // A stop in progress goes along with each trace, or a series
            // already missing for a few frames would be drawn straight across
            // the rest of its absence.
            missing_frames: self.main_missing_frames,
        };
        if self.overlays[i].unit != unit {
            self.y_axis.release();
        }
        self.main_missing_frames = incoming_missing;
        self.current_series = series.map(str::to_owned);
        self.minimap_level = None;
        self.restart_sequence();
        self.last_display_raw = None;
        true
    }

    /// The units given a Y axis, the plotted series' first, then each shown
    /// trace's in the order the meter first sent them, up to [`MAX_AXES`].
    /// A hidden trace claims none, so hiding one frees its axis for the next,
    /// and nor does one with no reading in view: a sub-value the meter
    /// stopped sending doesn't hold an axis it has nothing to draw on.
    /// Not cut to the width: see [`Graph::drawn_units`].
    fn axis_units(&self) -> Vec<&str> {
        let (x_min, x_max) = self.view_bounds();
        let mut units = vec![self.current_unit.as_str()];
        for o in &self.overlays {
            if units.len() == MAX_AXES {
                break;
            }
            if self.is_hidden(o) || units.contains(&o.unit.as_str()) {
                continue;
            }
            let (start, end) = self.time_index_range(&o.points, |p| p.time, x_min, x_max);
            if o.points.range(start..end).any(|p| p.value.is_some()) {
                units.push(&o.unit);
            }
        }
        units
    }

    /// [`Graph::axis_units`] cut to the axes the graph's width has room for:
    /// all of them, or the plotted unit's alone.
    fn drawn_units(&self) -> Vec<&str> {
        let mut units = self.axis_units();
        units.truncate(self.axis_room.max(1));
        units
    }

    /// Whether a kept trace is drawn, when shown: its unit has an axis.
    fn drawn(&self, o: &OverlaySeries) -> bool {
        self.drawn_units().contains(&o.unit.as_str())
    }

    /// Why a kept trace's unit has no axis, if it hasn't: the axes are all
    /// taken, or the window is too narrow for them. `None` while it has one,
    /// or while hidden with an axis free to take.
    fn axis_refusal(&self, o: &OverlaySeries) -> Option<AxisRefusal> {
        let units = self.axis_units();
        if units.contains(&o.unit.as_str()) {
            return (!self.drawn(o)).then_some(AxisRefusal::Narrow);
        }
        (units.len() == MAX_AXES).then_some(AxisRefusal::Full)
    }

    /// Set how many axes fit a graph `width` points wide: room for the plot
    /// beside the plotted unit's axis, then [`RIGHT_AXIS_ROOM`] for the one
    /// column every right axis shares, their labels stacked.
    fn fit_axes_to(&mut self, width: f32) {
        let spare = width - LEFT_AXIS_ROOM - MIN_PLOT_WIDTH;
        self.axis_room = if spare >= RIGHT_AXIS_ROOM {
            MAX_AXES
        } else {
            1
        };
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

    /// The plotted unit while a reading is plotted, `None` before one: a
    /// unitless reading's "" is a unit, the absence of a reading is not.
    fn plotted_unit_if_any(&self) -> Option<&str> {
        self.current_mode
            .as_ref()
            .map(|_| self.current_unit.as_str())
    }

    /// Unit of the series being plotted. The stats panel captions its
    /// visible-window block with this, which is not the meter's main unit
    /// when a sub-value is plotted.
    pub fn plotted_unit(&self) -> &str {
        &self.current_unit
    }

    /// Whether the plotted series is a level (an NCV bar), whose points
    /// are whole numbers: see [`PlotSample::levels`].
    pub fn plots_levels(&self) -> bool {
        self.levels
    }

    /// Whether the history holds the meter's main reading rather than a
    /// sub-value.
    pub fn plots_main(&self) -> bool {
        self.current_series.is_none()
    }

    /// Decimals a figure of the plotted series is written with: four, or
    /// none for one of a level's own points, which is a whole number. A
    /// figure of a level's that isn't one of its points — a mean, a typed
    /// reference line, a height on the axis — gets one.
    pub fn decimals(&self, point: bool) -> usize {
        match (self.levels, point) {
            (false, _) => 4,
            (true, true) => 0,
            (true, false) => 1,
        }
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
    /// pop_front on eviction), so `partition_point` can binary-search them.
    fn time_index_range<T>(
        &self,
        points: &VecDeque<T>,
        time: impl Fn(&T) -> Instant,
        x_min: f64,
        x_max: f64,
    ) -> (usize, usize) {
        (
            points.partition_point(|p| self.elapsed_secs(time(p)) < x_min),
            points.partition_point(|p| self.elapsed_secs(time(p)) <= x_max),
        )
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

    /// Combined render: toolbar + main graph + minimap, with `markers` on
    /// both and the alarm's `limits` on the main graph; `digits` names a
    /// reading in its menu.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        tc: &ThemeColors,
        markers: &Markers,
        limits: Option<WatchedLimits>,
        digits: ReadingDigits,
    ) {
        self.handle_keyboard(ui.ctx());
        // Before the toolbar, whose Show: chips say which units have no axis.
        let width = ui.available_width();
        self.fit_axes_to(width);
        self.show_toolbar(ui, l10n, tc);
        let room = ui.available_height() - (MINIMAP_HEIGHT + 30.0);
        // After the toolbar, so a click on the Histogram chip lays the pane
        // out in the frame it lands in.
        self.histogram_place = self
            .histogram_shown()
            .then(|| histogram::place(width, strip_room(width), room, self.histogram_sizes));
        let place = self.histogram_place;
        let band_room = match place {
            Some(p) if p.dock == histogram::Dock::Below => p.size + histogram::DIVIDER,
            _ => 0.0,
        };
        let main_height = (room - band_room).max(60.0);
        if let Some(p) = place.filter(|p| p.dock == histogram::Dock::Side) {
            let area = ui
                .horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width() - p.size, main_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.show_main(ui, l10n, tc, markers, limits, digits),
                    );
                    let (area, _) = ui
                        .allocate_exact_size(egui::vec2(p.size, main_height), egui::Sense::hover());
                    self.paint_histogram(ui, l10n, area, tc, limits);
                    area
                })
                .inner;
            // The divider spans the strip's bars, which stand beside the plot.
            let y = self.plot_rect.map_or(area.y_range(), |plot| plot.y_range());
            let rect =
                egui::Rect::from_x_y_ranges(area.left()..=area.left() + histogram::DIVIDER, y);
            // Here, after the plot and before the minimap, so Tab reaches it
            // in the order it stands on screen.
            self.histogram_divider(ui, l10n, rect, p);
        } else {
            ui.allocate_ui(egui::vec2(ui.available_width(), main_height), |ui| {
                self.show_main(ui, l10n, tc, markers, limits, digits);
            });
        }
        ui.add_space(4.0);
        self.show_minimap(ui, l10n, tc, markers);
        if let Some(p) = place.filter(|p| p.dock == histogram::Dock::Below) {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), histogram::DIVIDER),
                egui::Sense::hover(),
            );
            let (area, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), p.size),
                egui::Sense::hover(),
            );
            self.paint_histogram(ui, l10n, area, tc, limits);
            self.histogram_divider(ui, l10n, rect, p);
        }
    }

    /// The markers on readings between `x_min` and `x_max` seconds from the
    /// origin, with where they sit. None before the origin: a marker from
    /// before the graph restarted is on a trace it no longer shows.
    pub(crate) fn markers_between<'a>(
        &self,
        markers: &'a Markers,
        x_min: f64,
        x_max: f64,
    ) -> Vec<(f64, &'a Marker)> {
        let Some(origin) = self.origin else {
            return Vec::new();
        };
        let at = |secs: f64| origin + std::time::Duration::from_secs_f64(secs.max(0.0));
        markers
            .between(at(x_min), at(x_max.max(x_min)))
            .map(|m| (self.elapsed_secs(m.at), m))
            .collect()
    }

    /// The **Show:** chip the user clicked since the last call, with whether
    /// its trace is now hidden, for the settings to remember. `None` for the
    /// main reading's own chip (shown when a sub-value is plotted): kept, it
    /// would hide that reading on the next meter to plot a sub-value.
    pub(crate) fn take_trace_choice(&mut self) -> Option<(String, bool)> {
        self.trace_choice.take()
    }

    /// Whether the user switched a kept trace off: the main reading's by its
    /// own chip ([`Graph::main_hidden`]), a sub-value's by label.
    fn is_hidden(&self, o: &OverlaySeries) -> bool {
        if self.is_main_trace(&o.label) {
            self.main_hidden
        } else {
            self.hidden_overlays.contains(&o.label)
        }
    }

    /// Whether `label` names the main reading's trace, drawn beside a
    /// plotted sub-value.
    fn is_main_trace(&self, label: &str) -> bool {
        self.current_series.is_some() && label == self.main_name()
    }

    /// Hide the traces the settings remember hidden, at start-up.
    pub(crate) fn hide_overlays<'a>(&mut self, labels: impl IntoIterator<Item = &'a String>) {
        self.hidden_overlays.extend(labels.into_iter().cloned());
    }

    /// The number of the marker whose flag was clicked since the last call.
    pub(crate) fn take_clicked_marker(&mut self) -> Option<u32> {
        self.clicked_marker.take()
    }

    /// What an entry picked in the plot's menu since the last call asks of
    /// the app.
    pub(crate) fn take_menu_action(&mut self) -> Option<MenuAction> {
        self.menu_action.take()
    }

    /// Whether the graph still shows the reading taken at `at`.
    pub(crate) fn holds(&self, at: Instant) -> bool {
        self.origin.is_some_and(|origin| at >= origin)
            && self.first_point_time().is_some_and(|first| at >= first)
    }

    /// Bring the reading taken at `at` into view, centred, keeping the
    /// window's width, when it is out of view. Returns to live if that is
    /// where it lands, as scrolling there would.
    pub(crate) fn reveal(&mut self, at: Instant) {
        if !self.holds(at) {
            return;
        }
        let x = self.elapsed_secs(at);
        let (view_min, view_max) = self.view_bounds();
        if (view_min..=view_max).contains(&x) {
            return;
        }
        self.center_view_on(x);
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
    pub(crate) fn visible_gaps(&self) -> Vec<(f64, f64, GapKind)> {
        self.build_segments_for_range(0, self.history.len()).1
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.history.len()
    }

    /// Pick a series under **Plot:**, as the toolbar does.
    #[cfg(test)]
    pub(crate) fn select_series(&mut self, label: Option<&str>) {
        self.selected_series = label.map(str::to_owned);
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
    pub(crate) fn overlay_values(&self, label: &str) -> Vec<Option<f64>> {
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
