//! Painting the main graph: the traces, the analysis overlays, the axes and
//! the accessibility summary that describes them. The key, the hover and
//! the text labels have files of their own (`key.rs`, `hover.rs`,
//! `labels.rs`).

use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{
    AxisHints, GridInput, GridMark, Line, Plot, PlotBounds, PlotPoint, PlotPoints, PlotTransform,
    Points, Span, VLine,
};
use std::cell::RefCell;
use std::time::Instant;

use super::axes::{self, AxisMap};
use super::hover::{
    height_tags, hover_lines, paint_height_tags, paint_hover_dots, reading_near, show_hover_readout,
};
use super::labels::{FLAG_AIR, OverlayLabelData, marker_flags};
use super::menu::{MenuOpeners, plot_menu_id};
use super::minimap::bucket_secs;
use super::pattern::{DASH_GAP, PatternKey, Patterns, SegmentedLine, TimedHLine, anchor_layout};
use super::time::{MIN_GRID_SPACING, format_pointer_time, format_time_axis_label, time_marks};
use super::{GapKind, Graph, OverlaySeries, SegmentsAndGaps};
use crate::markers::Markers;
use crate::theme::ThemeColors;

/// One sub-value trace ready to draw: (overlay index, name, segments).
///
/// The index — not a colour — because it is what both the trace and its key
/// row derive their colour and line style from, so the two cannot drift.
pub(super) type OverlayTrace = (usize, String, Vec<Vec<[f64; 2]>>);

/// Shapes for a line from `a` to `b`, matching how `egui_plot` renders the
/// same `LineStyle` on the plot — otherwise the plot key, or the histogram a
/// line crosses, would promise a dash pattern the plot doesn't use.
pub(super) fn styled_line(
    a: egui::Pos2,
    b: egui::Pos2,
    stroke: egui::Stroke,
    style: egui_plot::LineStyle,
) -> Vec<egui::Shape> {
    match style {
        egui_plot::LineStyle::Solid => vec![egui::Shape::line_segment([a, b], stroke)],
        egui_plot::LineStyle::Dotted { spacing } => {
            egui::Shape::dotted_line(&[a, b], stroke.color, spacing, stroke.width)
        }
        egui_plot::LineStyle::Dashed { length } => {
            egui::Shape::dashed_line(&[a, b], stroke, length, length * DASH_GAP)
        }
    }
}

/// Quantize a `f64` to ~3 decimal digits before hashing so animated plot
/// transforms (which produce sub-pixel jitter on the y bounds) don't
/// invalidate label caches every frame. Returns an `i64` so the result is
/// `Hash`-stable and not affected by `f64::NaN` weirdness.
pub(super) fn quantize_for_hash(v: f64) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v * 1000.0).round() as i64
    }
}

/// The points of one line worth drawing: each `bucket_secs` span of session
/// time keeps its first, lowest, highest and last point, in time order.
///
/// A zoomed-out window holds tens of samples per pixel, and past a few all
/// that shows is how far the line reaches in each column, so the points
/// drawn follow the plot's width rather than the window's. A span holding
/// one sample keeps it, so a sparse window draws exactly as before.
///
/// Buckets are cut from the graph origin, as the minimap's are, never from
/// the view's edge: in live view that edge moves with every sample, and
/// screen-column buckets would change members each frame and flicker.
///
/// Input is in time order, as the segment builders produce it.
pub(super) fn thin_for_drawing(points: &[[f64; 2]], bucket_secs: f64) -> Vec<[f64; 2]> {
    let bucket = |x: f64| (x / bucket_secs).floor();
    let mut out = Vec::new();
    let mut i = 0;
    while i < points.len() {
        let key = bucket(points[i][0]);
        let first = i;
        let (mut lo, mut hi) = (i, i);
        while i < points.len() && bucket(points[i][0]) == key {
            if points[i][1] < points[lo][1] {
                lo = i;
            }
            if points[i][1] > points[hi][1] {
                hi = i;
            }
            i += 1;
        }
        let mut kept = [first, lo, hi, i - 1];
        kept.sort_unstable();
        let mut last = None;
        for k in kept {
            if last != Some(k) {
                out.push(points[k]);
                last = Some(k);
            }
        }
    }
    out
}

/// Points between the plotted unit's grid lines, at most, when right axes
/// share them: room for a label each, without crowding the grid.
const AXIS_TICK_SPACING: f32 = 50.0;

/// Points a right axis keeps beside its widest label: egui_plot's margin
/// either side of the text, plus a pixel of air.
const RIGHT_AXIS_MARGIN: f32 = 9.0;

/// Colours the palette has for sub-value traces before they repeat
/// (`ThemeColors::graph_overlay`).
const OVERLAY_COLORS: usize = 3;

/// Rows of stacked right-axis labels: a gap of this many points between
/// one gridline's stack and the next.
const STACK_GAP: f32 = 6.0;

/// One right axis's tick label at a height on the plot, and its colour.
type RightAxisLabel = (Box<dyn Fn(f64) -> String>, egui::Color32);

/// A frame's right axes: see [`Graph::right_axes`].
#[derive(Default)]
struct RightAxes {
    /// The plotted unit's grid step they share; `None` without any.
    step: Option<f64>,
    /// Each other unit's map onto the plot, in key order.
    maps: Vec<(String, AxisMap)>,
    /// The gridlines in view, in the plotted unit.
    ticks: Vec<f64>,
}

impl RightAxes {
    /// The map of `unit`'s axis, if it has one.
    fn map_of(&self, unit: &str) -> Option<AxisMap> {
        self.maps.iter().find(|(u, _)| u == unit).map(|&(_, m)| m)
    }
}

/// An axis's tick colour for a line drawn in `series`: that colour where it
/// reads as text on the panel, else the text colour. The unit on every tick
/// stays the cue that needs no colour.
fn tick_color(ui: &Ui, series: egui::Color32) -> egui::Color32 {
    crate::theme::legible_on(series, ui.visuals().panel_fill).unwrap_or(ui.visuals().text_color())
}

/// The tick label a right axis writes at a height on the plot: the whole
/// step of its unit there, then the unit — or nothing on a gridline it
/// leaves unlabelled.
fn right_axis_label(map: AxisMap, unit: String) -> impl Fn(f64) -> String {
    let decimals = map.decimals();
    move |y| {
        let v = (map.value_at(y) / map.step).round() * map.step;
        if !map.labels(v) {
            return String::new();
        }
        let val = eframe::emath::format_with_decimals_in_range(v, decimals..=decimals);
        format!("  {val} {unit}")
    }
}

/// The gridlines, by height on screen (`mids`), whose stack of labels
/// `height` tall fits: within `room`, and clear of the stack kept before
/// it. A plot too short for a stack at each line keeps every other one, or
/// none, as egui_plot drops labels it has no room for.
pub(super) fn stacks_that_fit(mids: &[f32], height: f32, room: egui::Rangef) -> Vec<f32> {
    let mut sorted = mids.to_vec();
    sorted.sort_by(f32::total_cmp);
    let mut kept: Vec<f32> = Vec::new();
    for mid in sorted {
        let (top, bottom) = (mid - height / 2.0, mid + height / 2.0);
        let clear = kept.last().is_none_or(|&last| top >= last + height / 2.0);
        if top >= room.min && bottom <= room.max && clear {
            kept.push(mid);
        }
    }
    kept
}

/// The Y grid of a level's axis: egui_plot's decade steps, the finest of
/// them never under one — a level is a whole number, and a tick at 0.5 of
/// one names a reading the meter cannot give.
pub(super) fn whole_number_marks(input: GridInput) -> Vec<GridMark> {
    egui_plot::uniform_grid_spacer(|input| {
        let step = 10f64.powf(input.base_step_size.max(1.0).log10().ceil());
        [step, step * 10.0, step * 100.0]
    })(input)
}

/// A trace of levels drawn as steps: each level held until the next reading,
/// then a riser. A slope between two readings would draw levels in between
/// that the meter never showed.
pub(super) fn stepped(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(points.len() * 2);
    for p in points {
        if let Some(&[_, held]) = out.last()
            && held != p[1]
        {
            out.push([p[0], held]);
        }
        out.push(*p);
    }
    out
}

impl Graph {
    /// Build segment and gap data for a slice of history, suitable for
    /// passing to egui_plot. Only the points in `[start, end)` are visited.
    pub(super) fn build_segments_for_range(&self, start: usize, end: usize) -> SegmentsAndGaps {
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
                // Through the same helper the minimap's level appends with,
                // so the plot's bands and the strip's cannot come to describe
                // one interruption differently.
                gaps.extend(self.gap_entries(prev, point, kind).into_iter().flatten());
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

    /// Build the drawable segments of one overlay whose points fall within
    /// `[x_min, x_max]`, plus one point either side so a line reaching the
    /// edge of the view is not clipped to nothing.
    ///
    /// Deliberately separate from `build_segments_for_range` rather than
    /// generalising it: that one also derives the gap ranges the bands and
    /// dropout markers are drawn from, and its data-loss split is subtle and
    /// tested. Overlays draw no bands and no markers. Their points keep their
    /// own times, so they break on their own: at a point with no value (over
    /// range, or a lost link) and across a silence longer than the gap
    /// threshold, as the plotted series does. Where the plotted series is over
    /// range says nothing about a sub-value beside it.
    pub(super) fn build_overlay_segments_for_range(
        &self,
        o: &OverlaySeries,
        x_min: f64,
        x_max: f64,
    ) -> Vec<Vec<[f64; 2]>> {
        let (start, end) = self.time_index_range(&o.points, |p| p.time, x_min, x_max);
        let start = start.saturating_sub(1);
        let end = (end + 1).min(o.points.len());
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for i in start..end {
            let point = o.points[i];
            if let Some(prev) = prev_time
                && self.silent_between(prev, point.time)
                && !current.is_empty()
            {
                segments.push(std::mem::take(&mut current));
            }
            prev_time = Some(point.time);

            match point.value {
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

    /// The drawn overlays the user has not switched off, paired with their
    /// index.
    ///
    /// The index is the overlay's position among the kept ones, which is what
    /// keys its colour and line style — so hiding one, or an axis shed for
    /// width, does not reshuffle the palette of the others.
    pub(super) fn shown_overlays(&self) -> impl Iterator<Item = (usize, &OverlaySeries)> {
        let units = self.drawn_units();
        self.overlays
            .iter()
            .enumerate()
            .filter(move |(_, o)| units.contains(&o.unit.as_str()) && !self.is_hidden(o))
    }

    /// The unit a kept trace is in; the plotted unit for one no longer kept.
    pub(super) fn overlay_unit(&self, label: &str) -> &str {
        self.overlays
            .iter()
            .find(|o| o.label == label)
            .map_or(self.current_unit.as_str(), |o| o.unit.as_str())
    }

    /// The sub-value traces actually drawn over `[x_min, x_max]`.
    ///
    /// Leaves out the ones the user switched off, and the ones with no points
    /// in this window — an overlay that isn't drawn must not appear in the key
    /// either.
    pub(super) fn visible_overlay_traces(&self, x_min: f64, x_max: f64) -> Vec<OverlayTrace> {
        self.shown_overlays()
            .filter_map(|(k, o)| {
                let segments = self.build_overlay_segments_for_range(o, x_min, x_max);
                (!segments.is_empty()).then(|| (k, o.label.clone(), segments))
            })
            .collect()
    }

    /// Colour and line style of one overlay trace.
    ///
    /// Keyed on the overlay's index — never on a hash of its label, which
    /// would reshuffle the palette when a sub-value appears or disappears
    /// mid-capture. The line style carries the same information without
    /// colour, as `.claude/rules/gui.md` requires — unless the user chose
    /// solid lines (`solid`, Settings → Graph lines). Even then a fourth
    /// trace, which shares the first one's colour, keeps its pattern.
    pub(super) fn overlay_color_and_style(
        tc: &ThemeColors,
        index: usize,
        solid: bool,
    ) -> (egui::Color32, egui_plot::LineStyle) {
        let style = match index % 4 {
            _ if solid && index < OVERLAY_COLORS => egui_plot::LineStyle::Solid,
            0 => egui_plot::LineStyle::dashed_loose(),
            1 => egui_plot::LineStyle::dotted_loose(),
            2 => egui_plot::LineStyle::dashed_dense(),
            _ => egui_plot::LineStyle::dotted_dense(),
        };
        (tc.graph_overlay(index), style)
    }

    /// The plotted unit's axis, on the left. On a grid shared with right axes
    /// (`grid_step`) every tick is a whole step, written to the step's
    /// decimals like theirs, and in the colour of the plotted line.
    fn left_axis(
        &self,
        ui: &Ui,
        grid_step: Option<f64>,
        line_color: egui::Color32,
    ) -> AxisHints<'static> {
        let unit = self.current_unit.clone();
        let axis = AxisHints::new_y().formatter(move |mark, _range| {
            let (value, decimals) = match grid_step {
                Some(step) => (
                    (mark.value / step).round() * step,
                    axes::step_decimals(step),
                ),
                None => (
                    mark.value,
                    (-mark.step_size.log10().round() as usize).min(6),
                ),
            };
            let val = eframe::emath::format_with_decimals_in_range(value, decimals..=decimals);
            if unit.is_empty() {
                val
            } else {
                format!("{val} {unit}  ")
            }
        });
        match grid_step {
            Some(_) => axis.tick_label_color(tick_color(ui, line_color)),
            None => axis,
        }
    }

    /// This frame's right axes: one per sub-value unit drawn beside the
    /// plotted series, aligned on a grid of round steps of the plotted unit
    /// (`axes.rs`). None — egui_plot's own grid, as a single unit always
    /// has — on a level's whole-number grid, or on a flat or non-finite range
    /// (a Y: Fixed of "inf"), which has no steps to share.
    fn right_axes(
        &self,
        ui: &Ui,
        (view_min, view_max): (f64, f64),
        (y_min, y_max): (f64, f64),
    ) -> RightAxes {
        let secondaries = self.secondary_targets(view_min, view_max);
        let shareable = !self.levels && y_min.is_finite() && y_max.is_finite() && y_max > y_min;
        if !shareable || secondaries.is_empty() {
            return RightAxes::default();
        }
        // The stacked labels need a row each between gridlines.
        let rows = secondaries.len() as f32;
        let row_height = ui.text_style_height(&egui::TextStyle::Body);
        let spacing = AXIS_TICK_SPACING.max(rows * row_height + STACK_GAP);
        let max_ticks = (ui.available_height() / spacing).floor().clamp(2.0, 10.0);
        let step = axes::primary_step(y_min, y_max, max_ticks as usize);
        RightAxes {
            step: Some(step),
            maps: secondaries
                .into_iter()
                .map(|(unit, target)| (unit, axes::fit_secondary(y_min, y_max, step, target)))
                .collect(),
            ticks: (((y_min / step).ceil() as i64)..=((y_max / step).floor() as i64))
                .map(|k| k as f64 * step)
                .collect(),
        }
    }

    /// The labels of `right`'s axes, each in the colour of the first of
    /// `traces` drawn against it, and the width of the column they share:
    /// this frame's widest label. egui_plot would otherwise size it from the
    /// previous frame's, and the plot would shift sideways as labels change
    /// length.
    fn right_axis_labels(
        &self,
        ui: &Ui,
        tc: &ThemeColors,
        right: &RightAxes,
        traces: &[OverlayTrace],
    ) -> (Vec<RightAxisLabel>, f32) {
        // The palette repeats after three colours, so a later axis whose
        // colour an earlier one already wears takes the text colour: two
        // axes in one colour would say their lines are one.
        let mut worn: Vec<egui::Color32> = Vec::new();
        let labels: Vec<RightAxisLabel> = right
            .maps
            .iter()
            .map(|(unit, map)| {
                let first = traces
                    .iter()
                    .find(|(_, label, _)| self.overlay_unit(label) == unit.as_str());
                let color = match first.map(|&(k, _, _)| tc.graph_overlay(k)) {
                    Some(c) if !worn.contains(&c) => {
                        worn.push(c);
                        tick_color(ui, c)
                    }
                    _ => ui.visuals().text_color(),
                };
                (Box::new(right_axis_label(*map, unit.clone())) as _, color)
            })
            .collect();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let width = labels
            .iter()
            .flat_map(|(label, _)| right.ticks.iter().map(move |&y| label(y)))
            .map(|text| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(text, font.clone(), egui::Color32::PLACEHOLDER)
                        .size()
                        .x
                })
            })
            .fold(0.0_f32, f32::max);
        (labels, width)
    }

    /// Paint the right axes' labels in the column right of `plot`: at each
    /// gridline, one row per axis, in key order and in its axis's colour,
    /// centred on the line as egui_plot centres its own. They sit on the same
    /// gridlines by construction, so one column holds them all.
    fn paint_stacked_ticks(
        ui: &Ui,
        plot: egui::Rect,
        transform: &PlotTransform,
        ticks: &[f64],
        rows: &[RightAxisLabel],
    ) {
        if rows.is_empty() {
            return;
        }
        let font = egui::TextStyle::Body.resolve(ui.style());
        let painter = ui.painter();
        // egui_plot's margin before a right axis's text.
        const INSET: f32 = 4.0;
        let x = transform.bounds().min()[0];
        let row_height = ui.text_style_height(&egui::TextStyle::Body);
        let mids: Vec<f32> = ticks
            .iter()
            .map(|&y| transform.position_from_point(&PlotPoint::new(x, y)).y)
            .collect();
        let height = row_height * rows.len() as f32;
        let fits = stacks_that_fit(&mids, height, plot.y_range().expand(row_height / 2.0));
        for (&y, &mid) in ticks
            .iter()
            .zip(&mids)
            .filter(|&(_, mid)| fits.contains(mid))
        {
            let galleys: Vec<_> = rows
                .iter()
                .map(|(label, color)| painter.layout_no_wrap(label(y), font.clone(), *color))
                .collect();
            let mut top = mid - height / 2.0;
            // A row a step apart, an unlabelled one included, so each unit
            // keeps its place in the stack.
            for (galley, (_, color)) in galleys.into_iter().zip(rows) {
                painter.galley(egui::pos2(plot.right() + INSET, top), galley, *color);
                top += row_height;
            }
        }
    }

    /// Render the main graph.
    pub fn show_main(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        tc: &ThemeColors,
        markers: &Markers,
        limits: Option<super::WatchedLimits>,
        digits: super::ReadingDigits,
    ) {
        let (view_min, view_max) = self.view_bounds();
        let marker_color = tc.graph_marker();
        let in_view = self.markers_between(markers, view_min, view_max);

        // Build segments and gaps for the visible slice only, plus one
        // point on each side so line segments at the view edges render
        // correctly and aren't clipped to nothing.
        let (vis_start, vis_end) = self.visible_index_range(view_min, view_max);
        let ext_start = vis_start.saturating_sub(1);
        let ext_end = (vis_end + 1).min(self.history.len());
        let (visible_segments, visible_gaps) = self.build_segments_for_range(ext_start, ext_end);
        // Drawn lines are thinned to a few points per bucket about half a
        // pixel wide; everything that answers with a value (statistics,
        // cursors, crossings, the hover, the Y range) still reads the full
        // slice. At a whole pixel, a column could fall between two buckets'
        // vertical strokes and a dense band drew with dark streaks through
        // it. The width is taken before the plot, axis included, so it errs
        // finer.
        let bucket = bucket_secs(
            (view_max - view_min)
                / f64::from(2.0 * ui.available_width() * ui.ctx().pixels_per_point()),
        );
        let thin = |points: &[[f64; 2]]| thin_for_drawing(points, bucket);
        // The dashed and dotted lines are built from further left, so the
        // pattern they carry from anchor to anchor stays on the line as the
        // view moves (`pattern.rs`).
        let (anchor_secs, draw_min) = anchor_layout(
            view_min,
            bucket,
            (view_max - view_min) / f64::from(ui.available_width()),
        );
        self.dash_phases
            .begin_frame(draw_min, view_max + anchor_secs);
        let phases = RefCell::new(std::mem::take(&mut self.dash_phases));
        let patterns = Patterns {
            anchor_secs,
            phases: &phases,
        };
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
        let line_color = tc.graph_line();
        let solid_lines = self.solid_lines;
        let gap_color = tc.graph_gap();
        let overload_edge = tc.graph_overload();
        let overload_fill = tc.graph_overload_fill();
        let mean_color = tc.graph_mean();
        let ref_color = tc.graph_ref();
        let cross_color = tc.graph_crossing();
        let cursor_color = tc.graph_cursor();
        let cursor_color_dim = tc.graph_cursor_dim();
        let env_color = tc.graph_envelope();
        let limit_color = tc.status_error();

        // The plotted series' name as the screen shows it, which the hover's
        // row names it by. The lines themselves are unnamed: nothing reads
        // their names now that the hover is ours.
        let plotted_name = self.plotted_series_name();
        let main_name = self.series_text(l10n, &plotted_name).into_owned();

        let shift_held = ui.input(|i| i.modifiers.shift);
        let bbox_active = self.box_zoom.is_some();
        // Plain drag-to-pan is allowed even in live mode — starting a drag
        // drops out of live (see handle_interaction). Bbox and shift-drag
        // always suppress the built-in pan.
        let allow_plot_x_drag = !shift_held && !bbox_active;

        // Compute Y bounds from visible data
        let (y_min, y_max) = self
            .y_range_for_view(view_min, view_max, true)
            .unwrap_or((-1.0, 1.0));

        // Binned over the Y range drawn, so the strip's bars sit at the
        // heights of the values they count, into bins sized for the plot's
        // extent they span, last frame's until it has one. Counted by bins,
        // a plot settling by a pixel doesn't bin it again.
        if let Some(place) = self.histogram_place {
            let plot = self.plot_rect.unwrap_or(ui.available_rect_before_wrap());
            let cursors = self.cursor_range();
            let key = super::HistogramKey {
                view: self.readings_key(vis_start..vis_end),
                interval: cursors.clone().map(|r| self.readings_key(r)),
                overloads: (
                    self.overloads_in(vis_start..vis_end),
                    cursors.map(|r| self.overloads_in(r)),
                ),
                range: (y_min, y_max),
                bins: super::histogram::target_bins(match place.dock {
                    super::histogram::Dock::Side => plot.height(),
                    super::histogram::Dock::Below => plot.width(),
                }),
                decimals: (self.decimals(true), self.decimals(false)),
                language: l10n.language().id,
            };
            self.bin_histogram(l10n, key, vis_start..vis_end);
        } else {
            self.histogram = None;
        }

        let right = self.right_axes(ui, (view_min, view_max), (y_min, y_max));
        self.axis_maps.clone_from(&right.maps);
        let grid_step = right.step;
        let map_of = |label: &str| right.map_of(self.overlay_unit(label));

        // Sub-value traces over the same window as the main series, minus
        // any the user switched off in the toolbar's Show: group, and any in
        // a unit with no axis in view to read them against.
        let mut overlay_traces = self.visible_overlay_traces(draw_min, view_max);
        overlay_traces.retain(|(_, label, _)| {
            self.overlay_unit(label) == self.current_unit || map_of(label).is_some()
        });
        let trace_maps: Vec<Option<AxisMap>> = overlay_traces
            .iter()
            .map(|(_, label, _)| map_of(label))
            .collect();
        let key_entries = self.key_entries(l10n, &overlay_traces, !visible_segments.is_empty());

        let mut y_axes = vec![self.left_axis(ui, grid_step, line_color)];
        // The right axes share one column, their labels painted once the plot
        // is drawn (`paint_stacked_ticks`); egui_plot only keeps its width.
        let (right_labels, column_width) = self.right_axis_labels(ui, tc, &right, &overlay_traces);
        if !right_labels.is_empty() {
            y_axes.push(
                AxisHints::new_y()
                    .placement(egui_plot::HPlacement::Right)
                    .min_thickness(column_width + RIGHT_AXIS_MARGIN)
                    .formatter(|_, _| String::new()),
            );
        }

        // The marker flags hang in the time axis's row, so the time labels
        // they would cover are left out. The formatter runs before this
        // frame's plot exists, so the flags are placed on last frame's.
        let flag_spans: Vec<egui::Rangef> = match self.plot_rect {
            Some(plot) if !in_view.is_empty() && view_max > view_min => {
                let at: Vec<(f32, u32, &str)> = in_view
                    .iter()
                    .map(|&(t, m)| {
                        let share = ((t - view_min) / (view_max - view_min)) as f32;
                        (
                            plot.left() + share * plot.width(),
                            m.number,
                            m.note.as_str(),
                        )
                    })
                    .collect();
                marker_flags(ui, &at, plot)
                    .iter()
                    .map(|f| f.rect.x_range().expand(FLAG_AIR))
                    .collect()
            }
            _ => Vec::new(),
        };
        let x_of = {
            let plot = self.plot_rect.unwrap_or(egui::Rect::NOTHING);
            move |t: f64| {
                let share = ((t - view_min) / (view_max - view_min).max(f64::EPSILON)) as f32;
                plot.left() + share * plot.width()
            }
        };
        let tick_font = egui::TextStyle::Body.resolve(ui.style());
        let ctx = ui.ctx().clone();
        let x_axis = AxisHints::new_x().formatter(move |mark, _range| {
            let text = format_time_axis_label(mark.value, mark.step_size);
            if flag_spans.is_empty() {
                return text;
            }
            let width = ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.clone(), tick_font.clone(), egui::Color32::PLACEHOLDER)
                    .size()
                    .x
            });
            let x = x_of(mark.value);
            let label = egui::Rangef::new(x - width / 2.0, x + width / 2.0);
            if flag_spans.iter().any(|f| f.intersects(label)) {
                String::new()
            } else {
                text
            }
        });

        let show_envelope = self.show_envelope;
        let (env_min, env_max) = if show_envelope {
            self.build_envelope(draw_min, view_max, self.envelope_window.value())
        } else {
            (Vec::new(), Vec::new())
        };
        let show_mean = self.show_mean;
        let show_ref = self.show_ref_line;
        let ref_values = self.ref_lines.values().to_vec();
        let limit_lines = self.limit_lines(limits);
        let show_crossings = self.show_crossings;
        let crossings = if show_ref && show_crossings && !ref_values.is_empty() {
            self.find_crossings(&ref_values, view_min, view_max)
        } else {
            Vec::new()
        };
        let cursors = self.cursors;
        let cursor_va = cursors
            .a
            .and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let cursor_vb = cursors
            .b
            .and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let mean_value = self.visible_stats().and_then(|s| s.avg());

        let (point_decimals, other_decimals) = (self.decimals(true), self.decimals(false));
        let levels = self.levels;
        // The readout under the pointer would cover the menu's entry, which
        // opens where the pointer is.
        let menu_open = egui::Popup::is_id_open(ui.ctx(), plot_menu_id());
        // The plot's whole area, axes included, as egui_plot is about to
        // allocate it: the axes' strips around the plot body take the menu's
        // right-click too.
        let plot_height = ui.available_height().max(60.0);
        let complete_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_size_before_wrap().x, plot_height),
        );
        let mut plot = Plot::new("main_plot")
            .height(plot_height)
            .allow_drag(Vec2b::new(allow_plot_x_drag, false))
            // handle_interaction owns the Ctrl+wheel zoom: it reads the same
            // `zoom_delta` egui_plot would, so leaving egui_plot's own zoom on
            // applies the tick twice — once to `time_window_secs`, once to the
            // transform the frame is drawn with. It gates nothing else; the
            // Shift+drag bbox is ours. egui_plot's own boxed zoom, on a
            // right-drag, would draw a box the view pinned below undoes;
            // the right button opens the plot's menu instead.
            .allow_zoom(Vec2b::FALSE)
            .allow_boxed_zoom(false)
            // Its drag-an-axis zoom would show a resize cursor over the axes
            // and jitter the view the per-frame `reset` pins back.
            .allow_axis_zoom_drag(false)
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .reset()
            .custom_x_axes(vec![x_axis])
            .custom_y_axes(y_axes)
            .y_axis_min_width(60.0)
            // The hover is ours, painted after `plot.show` below: egui_plot's
            // snaps to the nearest drawn point and reads the height there, so
            // one number was a reading or a height with nothing to say which.
            .show_x(false)
            .show_y(false);
        // egui_plot's own range, set here as `time_marks` reckons with it.
        plot = plot
            .grid_spacing(egui::Rangef::new(MIN_GRID_SPACING, 300.0))
            .x_grid_spacer(time_marks);
        if levels {
            plot = plot.y_grid_spacer(whole_number_marks);
        } else if let Some(step) = grid_step {
            plot = plot.y_grid_spacer(egui_plot::uniform_grid_spacer(move |_| {
                [step, step * 10.0, step * 100.0]
            }));
        }
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
                // Never from where it begins: that is the oldest point held,
                // which moves once the buffer's bound starts dropping them.
                for (edge, key) in [
                    (&env_max, PatternKey::EnvelopeMax),
                    (&env_min, PatternKey::EnvelopeMin),
                ] {
                    patterns.line(
                        plot_ui,
                        Line::new("", PlotPoints::new(thin(edge))),
                        env_color,
                        egui_plot::LineStyle::dashed_dense(),
                        key,
                        false,
                    );
                }
            }

            // Sub-values first: the plotted series goes on top of them. One in
            // another unit is drawn at its axis's heights.
            for ((k, label, segments), map) in overlay_traces.iter().zip(&trace_maps) {
                let (color, style) = Self::overlay_color_and_style(tc, *k, solid_lines);
                for (i, seg) in segments.iter().enumerate() {
                    let mut points = thin(seg);
                    if let Some(map) = map {
                        for p in &mut points {
                            p[1] = map.height_of(p[1]);
                        }
                    }
                    // A later segment begins after a break, at a fixed time.
                    // The first begins at the oldest point held, which moves
                    // once the buffer's bound starts dropping them.
                    patterns.line(
                        plot_ui,
                        Line::new("", PlotPoints::new(points)),
                        color,
                        style,
                        PatternKey::Overlay(label.clone()),
                        i > 0,
                    );
                }
            }

            let drawn_trace: Vec<Vec<[f64; 2]>> = visible_segments
                .iter()
                .map(|s| if levels { stepped(&thin(s)) } else { thin(s) })
                .collect();
            for seg in drawn_trace.iter().filter(|s| !s.is_empty()) {
                plot_ui.add(SegmentedLine::new(
                    Line::new("", PlotPoints::new(seg.clone())),
                    line_color,
                ));
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

            // Markers: dotted, apart from the dashed data-loss edges and the
            // solid cursors; each flag below carries its number.
            for &(x, _) in &in_view {
                plot_ui.vline(
                    VLine::new("", x)
                        .color(marker_color)
                        .width(1.5)
                        .style(egui_plot::LineStyle::dotted_dense()),
                );
            }

            // Mean line overlay
            if show_mean && let Some(avg) = mean_value {
                plot_ui.add(TimedHLine::new(
                    avg,
                    mean_color,
                    egui_plot::LineStyle::dashed_loose(),
                ));
            }

            // Reference line overlays
            if show_ref {
                for &v in &ref_values {
                    plot_ui.add(TimedHLine::new(
                        v,
                        ref_color,
                        egui_plot::LineStyle::dashed_dense(),
                    ));
                }
            }

            // Alarm limits, loosely dotted: the trace is solid and the mean
            // and reference lines dashed, so the limits read apart from all
            // of them without the colour — the error colour can be the
            // trace's own hue, as in the default themes. Wider than the
            // others, so the sparse dots still read as a line.
            for &(v, _) in &limit_lines {
                plot_ui.add(
                    TimedHLine::new(v, limit_color, egui_plot::LineStyle::dotted_loose())
                        .width(2.0),
                );
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
            if cursors.active {
                if let Some(t) = cursors.a {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_va {
                    plot_ui.add(TimedHLine::new(
                        v,
                        cursor_color_dim,
                        egui_plot::LineStyle::dashed_dense(),
                    ));
                }
                if let Some(t) = cursors.b {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_vb {
                    plot_ui.add(TimedHLine::new(
                        v,
                        cursor_color_dim,
                        egui_plot::LineStyle::dashed_dense(),
                    ));
                }
            }

            drawn_trace
        });

        self.dash_phases = phases.into_inner();
        self.plot_rect = Some(response.response.rect);
        self.plot_transform = Some(response.transform);
        // The hover: the pointer on screen, its height, the time it reads at
        // and whether that is inside an overload. The readout under the
        // pointer would cover the menu's entry, which opens where it is.
        let body = response.response.rect;
        let hover = response
            .response
            .hover_pos()
            .filter(|pos| !menu_open && body.contains(*pos))
            .map(|pos| {
                let transform = &response.transform;
                let series = self.hover_series(
                    l10n,
                    tc,
                    &main_name,
                    &visible_segments,
                    &overlay_traces,
                    &trace_maps,
                );
                let p = transform.value_from_position(pos);
                let radius = ui.style().interaction.interact_radius;
                let reach = |dx: f32| transform.value_from_position(pos + egui::vec2(dx, 0.0)).x;
                let [t, y] = reading_near(
                    &series,
                    (reach(-radius), reach(radius)),
                    (view_min, view_max),
                    pos,
                    radius,
                    |t, h| transform.position_from_point(&PlotPoint::new(t, h)),
                )
                .unwrap_or([p.x, p.y]);
                // Strictly inside: a band's edges are readings that keep
                // their values.
                let overload = overload_spans.iter().any(|&(a, b)| a < t && t < b);
                // A height off at infinity, as a Y range of "inf" gives, has
                // no place on screen: the crosshair stays on the pointer.
                let at = Some(transform.position_from_point(&PlotPoint::new(t, y)))
                    .filter(|at| at.is_finite())
                    .unwrap_or(pos);
                (at, y, t, overload, series)
            });
        // The crosshair under the labels, where egui_plot's sat: on the
        // reading it snapped to, or on the pointer.
        if let Some(&(at, ..)) = hover.as_ref() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            let painter = ui.painter().with_clip_rect(body);
            let stroke = egui::Stroke::new(1.0, tc.graph_crosshair());
            painter.vline(at.x, body.y_range(), stroke);
            painter.hline(body.x_range(), at.y, stroke);
        }
        Self::paint_stacked_ticks(
            ui,
            response.response.rect,
            &response.transform,
            &right.ticks,
            &right_labels,
        );

        let overlay = OverlayLabelData {
            show_mean,
            mean_value,
            show_ref,
            ref_values,
            limit_lines,
            cursors,
            cursor_va,
            cursor_vb,
            trace: response.inner,
            marker_times: in_view.iter().map(|&(t, _)| t).collect(),
            overlay_unit: self.current_unit.clone(),
            point_decimals,
            other_decimals,
            view_max,
            mean_color,
            ref_color,
            limit_color,
            cursor_color,
            halo_color: tc.plot_background(),
        };
        // The key and the flags first: the labels lay themselves out around
        // them.
        let mut taken: Vec<egui::Rect> = Self::paint_plot_key(
            ui,
            response.response.rect,
            &key_entries,
            tc,
            self.solid_lines,
        )
        .into_iter()
        .collect();
        // Before the flags, which sit in the time axis's row and must stay
        // on top of it for their clicks.
        let mut openers = Self::axis_strips(ui, complete_rect, response.response.rect);
        let (flag_click, flags) = Self::paint_marker_flags(
            ui,
            l10n,
            &response.transform,
            response.response.rect,
            &in_view,
            marker_color,
            tc.plot_background(),
        );
        self.clicked_marker = flag_click
            .filter(|&(_, button)| button == egui::PointerButton::Primary)
            .map(|(number, _)| number);
        openers.flag = flag_click
            .filter(|&(_, button)| button == egui::PointerButton::Secondary)
            .map(|(number, _)| number);
        taken.extend(flags);
        Self::paint_overlay_labels(
            ui,
            l10n,
            response.response.rect,
            &response.transform,
            &overlay,
            taken,
        );
        // The dots, the height's tags and the readout over everything else.
        if let Some((at, y, t, overload, series)) = hover {
            let rows = hover_lines(l10n, &series, t, overload, point_decimals);
            paint_hover_dots(
                ui,
                body,
                &response.transform,
                &series,
                &rows,
                tc.plot_background(),
            );
            let per_point = response.transform.dvalue_dpos()[1].abs();
            let max_decimals = if levels { other_decimals } else { usize::MAX };
            if let Some((left, right_tags)) =
                height_tags(y, per_point, &self.current_unit, &right.maps, max_decimals)
            {
                let left_color = if grid_step.is_some() {
                    tick_color(ui, line_color)
                } else {
                    ui.visuals().text_color()
                };
                let right_rows: Vec<_> = right_tags
                    .into_iter()
                    .map(|(i, text)| (text, right_labels[i].1))
                    .collect();
                paint_height_tags(
                    ui,
                    (complete_rect, body),
                    at.y,
                    &[(left, left_color)],
                    &right_rows,
                    tc.graph_crosshair(),
                );
            }
            // The time alone where no series has a reading. Not over the box
            // being drawn.
            if self.box_zoom.is_none() {
                show_hover_readout(
                    &response.response,
                    at,
                    &format_pointer_time(t),
                    &series,
                    &rows,
                );
            }
        }
        self.handle_interaction(ui, &response.response, &response.transform);
        self.show_context_menu(
            l10n,
            &response.response,
            &openers,
            &response.transform,
            &in_view,
            &overlay.limit_lines,
            digits,
        );
        self.update_plot_a11y_label(ui, l10n, response.response.id, y_min, y_max, in_view.len());
        // Draw a focus ring on the main plot body when it's keyboard-focused,
        // the plot's one Tab stop: egui_plot's axes take no focus.
        crate::a11y::paint_focus_ring(ui, &response.response);
    }

    /// Set an AccessKit label on the plot that summarizes current state so
    /// screen readers have a text alternative to the pixels. Throttled: the
    /// label is only re-formatted when the underlying state changes.
    fn update_plot_a11y_label(
        &mut self,
        ui: &Ui,
        l10n: &Localizer,
        plot_id: egui::Id,
        y_min: f64,
        y_max: f64,
        markers_in_view: usize,
    ) {
        use std::hash::{Hash, Hasher};
        let last_value = self.history.back().map(|p| p.value);
        let spoken_histogram = self.histogram.as_ref().map_or("", |b| b.spoken.as_str());
        // Only the traces actually drawn are spoken: a sub-value the user
        // switched off is not on screen, so announcing it would describe a
        // graph that isn't there.
        // Each with its unit when it is on a right axis of its own.
        let shown_labels: Vec<(&str, Option<&str>)> = self
            .shown_overlays()
            .map(|(_, o)| {
                let own_axis = o.unit != self.current_unit;
                (o.label.as_str(), own_axis.then_some(o.unit.as_str()))
            })
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
            // The words are the language's.
            l10n.language().id.hash(&mut h);
            self.live.hash(&mut h);
            last_value.map(f64::to_bits).hash(&mut h);
            self.current_unit.hash(&mut h);
            // Which series is plotted, and what is drawn beside it, are both
            // spoken below — so both have to bust the cache.
            self.current_series.as_deref().unwrap_or("").hash(&mut h);
            self.main_label.hash(&mut h);
            shown_labels.hash(&mut h);
            // Mode + display_raw drive the spoken last-reading; if either
            // changes (e.g. mode switch during paused playback) the label
            // must follow.
            self.current_mode.as_deref().unwrap_or("").hash(&mut h);
            self.last_display_raw.as_deref().unwrap_or("").hash(&mut h);
            // The over-range state is announced below, so it has to bust the
            // cache — otherwise the band appears with no spoken counterpart.
            self.breaks.kind.is_some().hash(&mut h);
            markers_in_view.hash(&mut h);
            spoken_histogram.hash(&mut h);
            h.finish()
        };
        if sig != self.a11y_label_sig {
            let also = if shown_labels.is_empty() {
                String::new()
            } else {
                let named: Vec<String> = shown_labels
                    .iter()
                    .map(|&(label, unit)| {
                        let label = self.series_text(l10n, label);
                        match unit {
                            Some(unit) => tr!(
                                l10n,
                                "graph-plot-a11y-also-axis",
                                series = label,
                                unit = crate::display::spoken_unit(l10n, unit, "")
                            )
                            .into_owned(),
                            None => label.into_owned(),
                        }
                    })
                    .collect();
                let separator = tr!(l10n, "graph-plot-a11y-also-separator");
                tr!(
                    l10n,
                    "graph-plot-a11y-also",
                    series = named.join(&*separator)
                )
                .into_owned()
            };
            self.a11y_label_sig = sig;
            let unit = if self.current_unit.is_empty() {
                ""
            } else {
                &self.current_unit
            };
            let state = tr!(
                l10n,
                "graph-plot-a11y-state",
                state = if self.live { "live" } else { "paused" }
            );
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
                _ if self.breaks.overloaded() => tr!(l10n, "graph-plot-a11y-over-range"),
                // Units by name, as the reading says them.
                (Some(raw), _) => tr!(
                    l10n,
                    "graph-plot-a11y-reading",
                    value = crate::display::spoken_amount(l10n, raw.trim(), unit)
                ),
                (None, Some(v)) => {
                    let value = format!("{v:.prec$}", prec = self.decimals(true));
                    let value = crate::display::spoken_amount(l10n, &value, unit);
                    tr!(l10n, "graph-plot-a11y-reading", value = value)
                }
                (None, None) => tr!(l10n, "graph-plot-a11y-no-data"),
            };
            // The plot draws several traces at once for a multi-display
            // meter, and which one the axis belongs to is otherwise purely
            // visual — the key painted in its corner.
            let series = match (self.current_series.as_deref(), self.main_label) {
                (Some(series), _) => Some(l10n.label(TextKind::Aux, series)),
                (None, Some(label)) => Some(l10n.main_label(label)),
                (None, None) => None,
            };
            let title = match series {
                Some(series) => tr!(l10n, "graph-plot-a11y-title-series", series = series),
                None => tr!(l10n, "graph-plot-a11y-title"),
            };
            let markers = match markers_in_view {
                0 => String::new(),
                n => tr!(l10n, "graph-plot-a11y-markers", count = n).into_owned(),
            };
            // Whole seconds, as the window is written: `{:.0}`'s rounding.
            let seconds = format!("{:.0}", self.time_window_secs)
                .parse::<i64>()
                .unwrap_or(0);
            let window = tr!(l10n, "graph-plot-a11y-window", seconds = seconds);
            let y_axis = tr!(
                l10n,
                "graph-plot-a11y-y-axis",
                min = format!("{y_min:.3}"),
                max = crate::display::spoken_amount(l10n, &format!("{y_max:.3}"), unit)
            );
            let samples = tr!(l10n, "graph-plot-a11y-samples", count = self.history.len());
            let sentences = [
                &*title,
                &*window,
                &*y_axis,
                &*samples,
                &also,
                &markers,
                spoken_histogram,
                &*state,
                &*reading,
            ];
            let separator = tr!(l10n, "graph-plot-a11y-separator");
            self.a11y_label = sentences
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(&*separator);
        }
        crate::a11y::set_accessible_label(ui, plot_id, &self.a11y_label);
    }

    /// Click areas over the left value axis and the time axis, beside and
    /// under `plot` in `complete`, for the menu to open from: egui_plot's own
    /// axis widgets only hover. Clicks only, so neither is a Tab stop.
    fn axis_strips(ui: &Ui, complete: egui::Rect, plot: egui::Rect) -> MenuOpeners {
        let strip = |rect: egui::Rect, salt: &str| {
            ui.interact(rect, ui.scope_id().with(salt), egui::Sense::CLICK)
        };
        MenuOpeners {
            value: strip(
                egui::Rect::from_min_max(
                    egui::pos2(complete.left(), plot.top()),
                    egui::pos2(plot.left(), plot.bottom()),
                ),
                "value_axis_menu",
            ),
            time: strip(
                egui::Rect::from_min_max(
                    egui::pos2(plot.left(), plot.bottom()),
                    egui::pos2(plot.right(), complete.bottom()),
                ),
                "time_axis_menu",
            ),
            flag: None,
        }
    }
}
