//! Painting the main graph: the traces, the analysis overlays, the plot key
//! and the accessibility summary that describes them.

use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{
    AxisHints, GridInput, GridMark, HLine, HoverPosition, Line, Plot, PlotBounds, PlotPoints,
    PlotTransform, Points, Span, VLine,
};
use std::time::Instant;

use super::minimap::bucket_secs;
use super::time::format_time_axis_label;
use super::{GapKind, Graph, OverlaySeries, SegmentsAndGaps};
use crate::markers::Markers;
use crate::theme::ThemeColors;

/// One sub-value trace ready to draw: (overlay index, name, segments).
///
/// The index — not a colour — because it is what both the trace and its key
/// row derive their colour and line style from, so the two cannot drift.
pub(super) type OverlayTrace = (usize, String, Vec<Vec<[f64; 2]>>);

/// How one row of the plot key draws its line sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyStyle {
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
pub(super) fn quantize_for_hash(v: f64) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v * 1000.0).round() as i64
    }
}

/// Widest a marker's flag grows, its note included.
const FLAG_MAX_WIDTH: f32 = 170.0;
/// Room left between a flag and its neighbours' lines.
const FLAG_AIR: f32 = 4.0;
/// Space between a flag's edge and its text.
const FLAG_PAD: f32 = 5.0;
/// Half the width of a flag's point, and its height.
const FLAG_TIP: egui::Vec2 = egui::vec2(5.0, 6.0);

/// The plot's right-click menu. One plot, so one id, known before the plot
/// is drawn.
fn plot_menu_id() -> egui::Id {
    egui::Id::new("main_plot_menu")
}

/// One marker's flag on the plot: the line it points at, where its tag is
/// drawn, and what the tag says.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Flag {
    pub(super) number: u32,
    pub(super) x: f32,
    pub(super) rect: egui::Rect,
    pub(super) label: String,
}

/// The flags of the markers at screen `x` (ascending), each labelled with
/// its number and note, along the bottom of `plot`.
///
/// A flag is centred on its line. Its note gets no more than the gap to the
/// nearer neighbouring line, so notes are cut before they cover a neighbour;
/// the number always stays. A flag that would still touch the one before it
/// slides right, and one that would slide off its own line is left out — its
/// line still shows. Near a plot edge a flag slides inward, its point staying
/// on the line. `width_of` measures a label.
pub(super) fn layout_marker_flags(
    markers: &[(f32, u32, &str)],
    plot: egui::Rect,
    height: f32,
    width_of: impl Fn(&str) -> f32,
) -> Vec<Flag> {
    let mut flags: Vec<Flag> = Vec::new();
    for (i, &(x, number, note)) in markers.iter().enumerate() {
        let before = i.checked_sub(1).map_or(f32::INFINITY, |j| x - markers[j].0);
        let after = markers.get(i + 1).map_or(f32::INFINITY, |next| next.0 - x);
        let room = FLAG_MAX_WIDTH.min(before - FLAG_AIR).min(after - FLAG_AIR);
        let label = fit_flag_label(number, note, room - 2.0 * FLAG_PAD, &width_of);
        let width = width_of(&label) + 2.0 * FLAG_PAD;
        let mut left = (x - width / 2.0).min(plot.right() - width).max(plot.left());
        if let Some(prev) = flags.last() {
            left = left.max(prev.rect.right() + FLAG_AIR);
        }
        // The point has to leave from the tag.
        if x < left || x > left + width || left + width > plot.right() {
            continue;
        }
        let bottom = plot.bottom() - 3.0;
        let rect = egui::Rect::from_min_max(
            egui::pos2(left, bottom - height),
            egui::pos2(left + width, bottom),
        );
        flags.push(Flag {
            number,
            x,
            rect,
            label,
        });
    }
    flags
}

/// The longest of "3 note" and "3 no…" that fits `room`, else "3".
fn fit_flag_label(number: u32, note: &str, room: f32, width_of: &impl Fn(&str) -> f32) -> String {
    let number = number.to_string();
    let note = note.trim();
    if note.is_empty() {
        return number;
    }
    // The top bar's separator, so a numeric note stays apart from the number.
    let full = format!("{number} \u{00B7} {note}");
    if width_of(&full) <= room {
        return full;
    }
    // Widths grow with the characters kept, so search for the most that fit
    // rather than measuring every length.
    let chars: Vec<char> = note.chars().collect();
    let cut = |k: usize| {
        let kept: String = chars[..k].iter().collect();
        format!("{number} \u{00B7} {}\u{2026}", kept.trim_end())
    };
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width_of(&cut(mid)) <= room {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo > 0 { cut(lo) } else { number }
}

/// How far a cursor's readout stands off its point: clear of the vertical
/// line and of the dashed level line.
const CURSOR_LABEL_OFFSET: egui::Vec2 = egui::vec2(4.0, 2.0);

/// Whether `rect` keeps a label's gap from every `taken` one, so two labels
/// never sit flush.
fn is_clear(taken: &[egui::Rect], rect: egui::Rect) -> bool {
    let gap = LEVEL_LABEL_GAP;
    taken.iter().all(|t| {
        rect.left() >= t.right() + gap
            || rect.right() + gap <= t.left()
            || rect.top() >= t.bottom() + gap
            || rect.bottom() + gap <= t.top()
    })
}

/// Rows a cursor's readout may move further from its point when all four
/// corners next to it are taken.
const CURSOR_LABEL_ROWS: usize = 2;

/// Where a cursor's readout goes around its point: the first of right-above,
/// left-above, right-below and left-below that stays inside the plot, clear
/// of the `taken` labels and off the trace, then the first inside and clear
/// of `taken`. When all four are taken — the other readout, a marker's
/// flag — the same corners a row further out, up to [`CURSOR_LABEL_ROWS`].
/// Failing all of those, None: a readout on top of another label is no
/// easier to read than none.
pub(super) fn cursor_label_rect(
    point: egui::Pos2,
    size: egui::Vec2,
    plot: egui::Rect,
    taken: &[egui::Rect],
    hits_trace: impl Fn(egui::Rect) -> bool,
) -> Option<egui::Rect> {
    use egui::{Align2, vec2};
    let (dx, dy) = (CURSOR_LABEL_OFFSET.x, CURSOR_LABEL_OFFSET.y);
    let corners = |away: f32| {
        [
            Align2::LEFT_BOTTOM.anchor_size(point + vec2(dx, -dy - away), size),
            Align2::RIGHT_BOTTOM.anchor_size(point + vec2(-dx, -dy - away), size),
            Align2::LEFT_TOP.anchor_size(point + vec2(dx, dy + away), size),
            Align2::RIGHT_TOP.anchor_size(point + vec2(-dx, dy + away), size),
        ]
    };
    let clear = |r: &egui::Rect| is_clear(taken, *r);
    for row in 0..=CURSOR_LABEL_ROWS {
        let row = corners(row as f32 * (size.y + dy));
        let inside = || row.iter().copied().filter(|r| plot.contains_rect(*r));
        // `clear` first: it is cheap, and the trace walk isn't.
        let found = inside()
            .filter(clear)
            .find(|r| !hits_trace(*r))
            .or_else(|| inside().find(clear));
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Where an overlay label's rim is drawn around its text: a pixel out on
/// every side, so the lines under it stop short of the letters.
const LABEL_HALO: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (0.0, 1.0),
    (1.0, 1.0),
];

/// Space between a mean or reference label and its line, and between two
/// labels stacked in a column.
const LEVEL_LABEL_GAP: f32 = 2.0;
/// How far a mean or reference label ends short of the plot's right edge,
/// or of a cursor or marker line it steps left of.
const LEVEL_LABEL_INSET: f32 = 4.0;

/// Where the labels of the mean and reference lines at screen `ys` go, given
/// the labels already placed (`taken`) and the vertical cursor and marker
/// lines at `line_xs`. `ys` runs top to bottom, and `sizes` matches it.
///
/// The labels form one column at the plot's right edge, in their lines'
/// order. Each sits just above its line — below it when there is no room
/// above — unless the label before it is in the way; then it goes right under
/// that one. A label never has a mean or reference line through its text:
/// it moves past the line instead, so lines closer together than a label is
/// tall get one label above them and the rest stacked below them. A label
/// steps left past the cursor and marker lines that would cross it, giving
/// up at the plot's left edge, and moves past a taken spot. The column is built top down and bottom up, with the
/// labels above their lines and then below them; the first of those clear of
/// the trace wins, else the first that fits. When no column fits, the lines
/// may cross the text; when none fits even so, each label goes where it
/// can and the rest are left out (None). A line above or below the view is
/// labelled at that edge.
///
/// In live view a cursor or marker line scrolling left through a label takes
/// it along until it has passed, then the label springs back to the edge,
/// and crowded labels can hop between columns as the trace moves under
/// them.
pub(super) fn level_label_rects(
    ys: &[f32],
    sizes: &[egui::Vec2],
    plot: egui::Rect,
    taken: &[egui::Rect],
    line_xs: &[f32],
    hits_trace: impl Fn(egui::Rect) -> bool,
) -> Vec<Option<egui::Rect>> {
    let gap = LEVEL_LABEL_GAP;
    // The lines drawn, which no label's text may cross.
    let drawn: Vec<f32> = ys
        .iter()
        .copied()
        .filter(|&y| plot.y_range().contains(y))
        .collect();
    let ys: Vec<f32> = ys
        .iter()
        .map(|y| y.max(plot.top()).min(plot.bottom()))
        .collect();
    // Above the line, or below it when asked or when above has no room.
    let usual_top = |y: f32, h: f32, below: bool| {
        let (above, under) = (y - gap - h, y + gap);
        if (below && under + h <= plot.bottom()) || above < plot.top() {
            under
        } else {
            above
        }
    };

    // Each label at its usual spot, or past the one before it: top down,
    // or bottom up. `level_ys` are the lines kept out of the text.
    let column = |down: bool, below: bool, level_ys: &[f32]| {
        let mut rects: Vec<egui::Rect> = Vec::with_capacity(ys.len());
        let order: Vec<usize> = if down {
            (0..ys.len()).collect()
        } else {
            (0..ys.len()).rev().collect()
        };
        for i in order {
            let (y, size) = (ys[i], sizes[i]);
            let top = match rects.last() {
                None => usual_top(y, size.y, below),
                Some(r) if down => usual_top(y, size.y, below).max(r.bottom() + gap),
                Some(r) => usual_top(y, size.y, below).min(r.top() - gap - size.y),
            };
            let rect = fit_level_label(top, size, down, plot, taken, level_ys, line_xs)?;
            rects.push(rect);
        }
        if !down {
            rects.reverse();
        }
        Some(rects)
    };
    // The first of: above the lines top down, bottom up, then below them
    // top down, bottom up — the first clear of the trace, if one is. Only
    // a clear column counts: a noisy trace crosses every column by a count
    // that changes each frame, and chasing the least would move the labels
    // with it. Lines too close for any column to keep out of the text may
    // cross it.
    for level_ys in [&drawn[..], &[]] {
        let columns: Vec<Vec<egui::Rect>> =
            [(true, false), (false, false), (true, true), (false, true)]
                .into_iter()
                .filter_map(|(down, below)| column(down, below, level_ys))
                .collect();
        if let Some(rects) = columns
            .iter()
            .find(|rects| !rects.iter().any(|r| hits_trace(*r)))
            .or(columns.first())
        {
            return rects.iter().copied().map(Some).collect();
        }
    }

    // No room for them all: each in turn where it fits, and none for the
    // rest rather than a pile no one can read.
    let mut placed: Vec<egui::Rect> = taken.to_vec();
    ys.iter()
        .zip(sizes)
        .map(|(&y, &size)| {
            let top = usual_top(y, size.y, false);
            let rect = fit_level_label(top, size, true, plot, &placed, &[], line_xs)
                .or_else(|| fit_level_label(top, size, false, plot, &placed, &[], line_xs))?;
            placed.push(rect);
            Some(rect)
        })
        .collect()
}

/// The first spot for a mean or reference label at or past `top`, moving
/// down or up: clear of the `level_ys` lines and of `taken`, stepped left
/// past the `line_xs` that would cross it. None once it leaves the plot.
fn fit_level_label(
    mut top: f32,
    size: egui::Vec2,
    down: bool,
    plot: egui::Rect,
    taken: &[egui::Rect],
    level_ys: &[f32],
    line_xs: &[f32],
) -> Option<egui::Rect> {
    let (gap, h) = (LEVEL_LABEL_GAP, size.y);
    let at =
        |top: f32, right: f32| egui::Rect::from_min_size(egui::pos2(right - size.x, top), size);
    let edge = plot.right() - LEVEL_LABEL_INSET;
    // Every pass moves past a line or a taken spot, so it ends.
    for _ in 0..=level_ys.len() + taken.len() {
        if top < plot.top() || top + h > plot.bottom() {
            return None;
        }
        // A mean or reference line through the text: past it.
        let through = level_ys
            .iter()
            .copied()
            .filter(|&y| top - gap < y && y < top + h + gap);
        if let Some(y) = if down {
            through.reduce(f32::max)
        } else {
            through.reduce(f32::min)
        } {
            top = if down { y + gap } else { y - gap - h };
            continue;
        }
        // Along the edge, then left of each line crossing it.
        let mut right = edge;
        let mut first_clear = None;
        // Each step passes at least one line.
        for _ in 0..=line_xs.len() {
            let rect = at(top, right);
            if !plot.contains_rect(rect) {
                break;
            }
            let clear = is_clear(taken, rect);
            let crossing = line_xs
                .iter()
                .copied()
                .filter(|&x| rect.left() < x && x < rect.right())
                .reduce(f32::min);
            match crossing {
                None if clear => return Some(rect),
                None => break,
                Some(x) => {
                    if clear && first_clear.is_none() {
                        first_clear = Some(rect);
                    }
                    right = x - LEVEL_LABEL_INSET;
                }
            }
        }
        if first_clear.is_some() {
            return first_clear;
        }
        // Taken at the edge: past whatever is there.
        let rect = at(top, edge);
        let blockers = taken.iter().filter(|t| !is_clear(&[**t], rect));
        top = if down {
            blockers.map(|t| t.bottom()).reduce(f32::max)? + gap
        } else {
            blockers.map(|t| t.top()).reduce(f32::min)? - gap - h
        };
    }
    None
}

/// Whether the straight segment from `a` to `b` passes through `rect`
/// (Liang–Barsky: clip the segment's parameter range against each edge).
pub(super) fn segment_hits_rect(a: egui::Pos2, b: egui::Pos2, rect: egui::Rect) -> bool {
    let d = b - a;
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-d.x, a.x - rect.left()),
        (d.x, rect.right() - a.x),
        (-d.y, a.y - rect.top()),
        (d.y, rect.bottom() - a.y),
    ] {
        if p == 0.0 {
            // Parallel to this edge: outside it means outside the rect.
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                enter = enter.max(r);
            } else {
                leave = leave.min(r);
            }
            if enter > leave {
                return false;
            }
        }
    }
    true
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
    /// The plotted series' visible segments as drawn, which cursor readouts
    /// keep off.
    trace: Vec<Vec<[f64; 2]>>,
    /// The times of the markers in view, whose lines the mean and reference
    /// labels step aside from.
    marker_times: Vec<f64>,
    overlay_unit: String,
    /// [`Graph::decimals`] for the cursors' values, which are points, and
    /// for the mean and the reference lines, which aren't.
    point_decimals: usize,
    other_decimals: usize,
    view_max: f64,
    mean_color: egui::Color32,
    ref_color: egui::Color32,
    cursor_color: egui::Color32,
    /// The plot's background, which rims each label so grid lines, the
    /// trace and other lines stop short of its letters.
    halo_color: egui::Color32,
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
    /// The index is the overlay's position among the drawn ones, which is what
    /// keys its colour and line style — stable while the plotted unit is, so
    /// hiding one does not reshuffle the palette of the others, and traces
    /// kept in another unit take no colour from the ones drawn.
    pub(super) fn shown_overlays(&self) -> impl Iterator<Item = (usize, &OverlaySeries)> {
        self.overlays
            .iter()
            .filter(|o| self.drawn(o))
            .enumerate()
            .filter(|(_, o)| !self.hidden_overlays.contains(&o.label))
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
            .unwrap_or_else(|| self.main_name().to_string())
    }

    /// Rows of the plot key: the plotted series when it is drawn, then each
    /// drawn overlay in `self.overlays` order.
    ///
    /// Empty when nothing is overlaid — a single-series graph gets no key, and
    /// looks exactly as it did before sub-values existed. Takes the traces
    /// `show_main` is about to draw rather than recomputing them, so the key
    /// cannot list a line that isn't there — the plotted series included,
    /// which has no line while a held meter sends only a sub-value.
    pub(super) fn key_entries(
        &self,
        drawn: &[OverlayTrace],
        plotted_drawn: bool,
    ) -> Vec<(String, KeyStyle)> {
        if drawn.is_empty() {
            return Vec::new();
        }
        let mut entries = Vec::with_capacity(drawn.len() + 1);
        if plotted_drawn {
            entries.push((self.plotted_series_name(), KeyStyle::Plotted));
        }
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
    ///
    /// Returns the key's rect, which the overlay labels keep off.
    fn paint_plot_key(
        ui: &Ui,
        plot_rect: egui::Rect,
        entries: &[(String, KeyStyle)],
        tc: &ThemeColors,
    ) -> Option<egui::Rect> {
        if entries.is_empty() {
            return None;
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
        Some(rect)
    }

    /// Render the main graph.
    pub fn show_main(&mut self, ui: &mut Ui, tc: &ThemeColors, markers: &Markers) {
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
        // cursors, crossings, the Y range) still reads the full slice. At a
        // whole pixel, a column could fall between two buckets' vertical
        // strokes and a dense band drew with dark streaks through it. The
        // width is taken before the plot, axis included, so it errs finer.
        let bucket = bucket_secs(
            (view_max - view_min)
                / f64::from(2.0 * ui.available_width() * ui.ctx().pixels_per_point()),
        );
        let thin = |points: &[[f64; 2]]| thin_for_drawing(points, bucket);
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
        let gap_color = tc.graph_gap();
        let overload_edge = tc.graph_overload();
        let overload_fill = tc.graph_overload_fill();
        let mean_color = tc.graph_mean();
        let ref_color = tc.graph_ref();
        let cross_color = tc.graph_crossing();
        let cursor_color = tc.graph_cursor();
        let cursor_color_dim = tc.graph_cursor_dim();
        let env_color = tc.graph_envelope();

        // Sub-value traces over the same window as the main series, minus
        // any the user switched off in the toolbar's Show: group.
        let overlay_traces = self.visible_overlay_traces(view_min, view_max);
        let key_entries = self.key_entries(&overlay_traces, !visible_segments.is_empty());
        let multi_series = !overlay_traces.is_empty();
        // Every drawn line carries the name of its series, which is what the
        // hover label reports; every helper item is named "" and falls through
        // to the plain form.
        let main_name = self.plotted_series_name();

        let shift_held = ui.input(|i| i.modifiers.shift);
        let bbox_active = self.bbox_zoom_start_px.is_some();
        // Plain drag-to-pan is allowed even in live mode — starting a drag
        // drops out of live (see handle_interaction). Bbox and shift-drag
        // always suppress the built-in pan.
        let allow_plot_x_drag = !shift_held && !bbox_active;

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
            self.build_envelope(view_min, view_max, self.envelope_window.value())
        } else {
            (Vec::new(), Vec::new())
        };
        let show_mean = self.show_mean;
        let show_ref = self.show_ref_line;
        let ref_values = self.ref_lines.values().to_vec();
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
        let mean_value = self.visible_stats().and_then(|s| s.avg());

        let cursor_unit = self.current_unit.clone();
        let (point_decimals, other_decimals) = (self.decimals(true), self.decimals(false));
        let levels = self.levels;
        // Moved into the label_formatter closure, which is rebuilt each frame.
        let tooltip_spans = overload_spans.clone();
        // The readout under the pointer would cover the menu's entry, which
        // opens where the pointer is.
        let menu_open = egui::Popup::is_id_open(ui.ctx(), plot_menu_id());
        let mut plot = Plot::new("main_plot")
            .height(ui.available_height().max(60.0))
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
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .reset()
            .custom_x_axes(vec![x_axis])
            .custom_y_axes(vec![y_axis])
            .y_axis_min_width(60.0)
            .cursor_color(tc.graph_crosshair())
            .label_formatter(move |pos| {
                // 0.37 replaced the `(name, point)` pair with `HoverPosition`.
                // A hover that isn't near a data point used to arrive as an
                // empty name, so map it back to one and keep the branches below
                // unchanged. The data points are the drawn ones: zoomed out, a
                // pixel column keeps its first, last and extremes, and the
                // pointer between them reads as Elsewhere.
                let (name, point) = match pos {
                    HoverPosition::NearDataPoint {
                        plot_name,
                        position,
                        ..
                    } => (*plot_name, position),
                    HoverPosition::Elsewhere { position } => ("", position),
                };
                if menu_open {
                    return None;
                }
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
                    return Some(format!("{time_label}\noverload"));
                }
                // Only a named item is a trace, so only its y is a point; a
                // helper's, or the pointer's own, is a height on the axis.
                let decimals = if name.is_empty() {
                    other_decimals
                } else {
                    point_decimals
                };
                // With several traces on the same axes the number alone is
                // ambiguous, so name the one being hovered. Helper items carry
                // an empty name and fall through to the plain form.
                if multi_series && !name.is_empty() {
                    return Some(format!(
                        "{time_label}\n{name}: {:.decimals$} {cursor_unit}",
                        point.y
                    ));
                }
                Some(format!(
                    "{time_label}\n{:.decimals$} {cursor_unit}",
                    point.y
                ))
            });
        if levels {
            plot = plot.y_grid_spacer(whole_number_marks);
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
                plot_ui.line(
                    Line::new("", PlotPoints::new(thin(&env_max)))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.line(
                    Line::new("", PlotPoints::new(thin(&env_min)))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Sub-values first: the plotted series goes on top of them.
            for (k, label, segments) in &overlay_traces {
                let (color, style) = Self::overlay_color_and_style(tc, *k);
                for seg in segments {
                    plot_ui.line(
                        Line::new(label.clone(), PlotPoints::new(thin(seg)))
                            .color(color)
                            .style(style),
                    );
                }
            }

            let drawn_trace: Vec<Vec<[f64; 2]>> = visible_segments
                .iter()
                .map(|s| if levels { stepped(&thin(s)) } else { thin(s) })
                .collect();
            for seg in &drawn_trace {
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

            drawn_trace
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
            trace: response.inner,
            marker_times: in_view.iter().map(|&(t, _)| t).collect(),
            overlay_unit: self.current_unit.clone(),
            point_decimals,
            other_decimals,
            view_max,
            mean_color,
            ref_color,
            cursor_color,
            halo_color: tc.plot_background(),
        };
        // The key and the flags first: the labels lay themselves out around
        // them.
        let mut taken: Vec<egui::Rect> =
            Self::paint_plot_key(ui, response.response.rect, &key_entries, tc)
                .into_iter()
                .collect();
        let (clicked_marker, flags) = Self::paint_marker_flags(
            ui,
            &response.transform,
            response.response.rect,
            &in_view,
            marker_color,
            tc.plot_background(),
        );
        self.clicked_marker = clicked_marker;
        taken.extend(flags);
        Self::paint_overlay_labels(
            ui,
            response.response.rect,
            &response.transform,
            &overlay,
            taken,
        );
        self.handle_interaction(ui, &response.response, &response.transform);
        self.show_context_menu(&response.response, &response.transform);
        self.update_plot_a11y_label(ui, response.response.id, y_min, y_max, in_view.len());
        // Draw a focus ring on the main plot body when it's keyboard-focused.
        // Note: egui_plot also allocates separate focusable responses for the
        // X and Y axes — those receive Tab but don't draw a focus indicator.
        // Making those invisible to Tab would require patching egui_plot.
        crate::a11y::paint_focus_ring(ui, &response.response);
    }

    /// The plot's right-click menu. What it offers to mark is the reading
    /// nearest the right-click, fixed as the menu opens: by the time the
    /// entry is picked the pointer is on the menu, and a live view has moved.
    ///
    /// Keyboard handling follows the Export… menu: the entry takes the focus
    /// as the menu opens, and Tab or Esc close it. The focus then goes
    /// nowhere rather than to the plot, where it would turn the graph's keys
    /// off.
    fn show_context_menu(&mut self, plot: &egui::Response, transform: &PlotTransform) {
        let ctx = plot.ctx.clone();
        if plot.secondary_clicked()
            && let Some(pos) = plot.interact_pointer_pos()
        {
            self.menu_reading = self.nearest_reading_in_view(transform.value_from_position(pos).x);
        }
        let popup_id = plot_menu_id();
        let was_open = egui::Popup::is_id_open(&ctx, popup_id);
        if was_open {
            let leave = ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                    | i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            });
            if leave {
                egui::Popup::close_id(&ctx, popup_id);
                ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }
        // Asked again the frame after it opens: the right-click that opened
        // it is a click outside the entry, which surrenders its focus.
        let focus_again = std::mem::take(&mut self.menu_focus_pending);
        let can_mark = self.menu_reading.is_some();
        let mut entry_id = None;
        let mut picked = false;
        egui::Popup::context_menu(plot).id(popup_id).show(|ui| {
            let add = ui
                .add_enabled(can_mark, egui::Button::new("Add marker here"))
                .on_disabled_hover_text("No reading in view to mark");
            if !was_open || focus_again {
                add.request_focus();
            }
            picked = was_open && add.clicked();
            crate::a11y::paint_focus_ring(ui, &add);
            entry_id = Some(add.id);
        });
        if picked {
            // Enter and Space click without a pointer click, which is the
            // only thing a menu closes on by itself.
            egui::Popup::close_id(&ctx, popup_id);
            self.mark_request = self.menu_reading.take();
        }
        let is_open = egui::Popup::is_id_open(&ctx, popup_id);
        if !was_open && is_open {
            self.menu_focus_pending = true;
        }
        if was_open
            && !is_open
            && let Some(id) = entry_id
        {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }

    /// Set an AccessKit label on the plot that summarizes current state so
    /// screen readers have a text alternative to the pixels. Throttled: the
    /// label is only re-formatted when the underlying state changes.
    fn update_plot_a11y_label(
        &mut self,
        ui: &Ui,
        plot_id: egui::Id,
        y_min: f64,
        y_max: f64,
        markers_in_view: usize,
    ) {
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
            self.main_label.hash(&mut h);
            shown_labels.hash(&mut h);
            // Mode + display_raw drive the spoken last-reading; if either
            // changes (e.g. mode switch during paused playback) the label
            // must follow.
            self.current_mode.as_deref().unwrap_or("").hash(&mut h);
            self.last_display_raw.as_deref().unwrap_or("").hash(&mut h);
            // The over-range state is announced below, so it has to bust the
            // cache — otherwise the band appears with no spoken counterpart.
            self.pending_break.is_some().hash(&mut h);
            markers_in_view.hash(&mut h);
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
                (None, Some(v)) => {
                    format!("last reading {v:.prec$} {unit}", prec = self.decimals(true))
                }
                (None, None) => "no data".to_string(),
            };
            // The plot draws several traces at once for a multi-display
            // meter, and which one the axis belongs to is otherwise purely
            // visual — the key painted in its corner.
            let of_series = match self.current_series.as_deref().or(self.main_label) {
                Some(label) => format!(" of {label}"),
                None => String::new(),
            };
            let markers = match markers_in_view {
                0 => String::new(),
                1 => " 1 marker in view.".to_string(),
                n => format!(" {n} markers in view."),
            };
            self.a11y_label = format!(
                "Measurement plot{of_series}. {:.0} second window. Y axis {:.3} to {:.3} {unit}. {} samples.{also}{markers} {}. {}.",
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

    /// Each marker's flag at the bottom of the plot: a tag in the marker
    /// colour pointing up at its line, with the number and as much of the
    /// note as fits, in the plot background's colour. Returns the number of
    /// the marker whose flag was clicked, and each flag's rect with its
    /// point, which the overlay labels keep off.
    ///
    /// A flag takes clicks but not the keyboard focus: the plot would gain a
    /// Tab stop per marker, and the Recording panel's log already has one.
    fn paint_marker_flags(
        ui: &Ui,
        transform: &PlotTransform,
        plot_rect: egui::Rect,
        in_view: &[(f64, &crate::markers::Marker)],
        color: egui::Color32,
        text_color: egui::Color32,
    ) -> (Option<u32>, Vec<egui::Rect>) {
        if in_view.is_empty() {
            return (None, Vec::new());
        }
        let mut clicked = None;
        let mut rects = Vec::new();
        let painter = ui.painter();
        let font = egui::FontId::proportional(12.0);
        let height = painter.fonts_mut(|f| f.row_height(&font)) + 4.0;
        let at: Vec<(f32, u32, &str)> = in_view
            .iter()
            .map(|&(x, m)| {
                let px = transform
                    .position_from_point(&egui_plot::PlotPoint::new(x, 0.0))
                    .x;
                (px, m.number, m.note.as_str())
            })
            .collect();
        let width_of = |s: &str| {
            painter
                .layout_no_wrap(s.to_string(), font.clone(), text_color)
                .size()
                .x
        };
        for flag in layout_marker_flags(&at, plot_rect, height, width_of) {
            let top = flag.rect.top();
            // The point's base stays on the tag, even for one slid to an edge.
            let base = |dx: f32| (flag.x + dx).clamp(flag.rect.left(), flag.rect.right());
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(base(-FLAG_TIP.x), top),
                    egui::pos2(flag.x, top - FLAG_TIP.y),
                    egui::pos2(base(FLAG_TIP.x), top),
                ],
                color,
                egui::Stroke::NONE,
            ));
            painter.rect_filled(flag.rect, 3.0, color);
            painter.text(
                flag.rect.center(),
                egui::Align2::CENTER_CENTER,
                &flag.label,
                font.clone(),
                text_color,
            );
            let edit = format!("Write marker {}'s note", flag.number);
            let response = ui
                .interact(
                    flag.rect,
                    ui.id().with(("marker_flag", flag.number)),
                    egui::Sense::CLICK,
                )
                .on_hover_text(&edit)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &edit));
            if response.clicked() {
                clicked = Some(flag.number);
            }
            rects.push(flag.rect.with_min_y(top - FLAG_TIP.y));
        }
        (clicked, rects)
    }

    /// Paint text labels for overlays (mean, reference lines, cursors) using the
    /// UI painter so they render outside the plot's clip rect.
    ///
    /// Each label keeps off `taken` — the key and the marker flags — and off
    /// the labels placed before it. The cursor readouts go first, as they
    /// have only four corners to choose from; the mean and reference labels
    /// then form one column around them (see [`level_label_rects`]).
    fn paint_overlay_labels(
        ui: &Ui,
        plot_rect: egui::Rect,
        transform: &PlotTransform,
        data: &OverlayLabelData,
        mut taken: Vec<egui::Rect>,
    ) {
        let painter = ui.painter();
        let label_font = egui::FontId::proportional(12.0);
        let screen =
            |t: f64, v: f64| transform.position_from_point(&egui_plot::PlotPoint::new(t, v));
        let layout = |text: String, color: egui::Color32| {
            painter.layout_no_wrap(text, label_font.clone(), color)
        };
        let paint = |rect: egui::Rect, galley: std::sync::Arc<egui::Galley>, color| {
            for (dx, dy) in LABEL_HALO {
                let at = rect.min + egui::vec2(dx, dy);
                painter.galley_with_override_text_color(at, galley.clone(), data.halo_color);
            }
            painter.galley(rect.min, galley, color);
        };

        // The vertical lines a mean or reference label steps aside from.
        let cursor_times = [data.cursor_a, data.cursor_b];
        let line_xs: Vec<f32> = data
            .marker_times
            .iter()
            .copied()
            .chain(
                cursor_times
                    .into_iter()
                    .flatten()
                    .filter(|_| data.cursors_active),
            )
            .map(|t| screen(t, 0.0).x)
            .filter(|&x| plot_rect.x_range().contains(x))
            .collect();

        // Whether the trace runs through a label. Only the stretch under the
        // label's time span can touch it.
        let hits_trace = |rect: egui::Rect| {
            let t_lo = transform.value_from_position(rect.left_top()).x;
            let t_hi = transform.value_from_position(rect.right_top()).x;
            data.trace.iter().any(|segment| {
                segment.windows(2).any(|w| {
                    w[1][0] >= t_lo
                        && w[0][0] <= t_hi
                        && segment_hits_rect(
                            screen(w[0][0], w[0][1]),
                            screen(w[1][0], w[1][1]),
                            rect,
                        )
                })
            })
        };

        let level_text = |v: f64| {
            format!(
                "{v:.prec$} {}",
                data.overlay_unit,
                prec = data.other_decimals
            )
        };
        // Cursor labels
        if data.cursors_active {
            for (name, t, value) in [
                ("A", data.cursor_a, data.cursor_va),
                ("B", data.cursor_b, data.cursor_vb),
            ] {
                let Some(t) = t else { continue };
                let y_val = value.unwrap_or(0.0);
                let pos = screen(t, y_val);
                // Scrolled out of view: no line to label. A reading above or
                // below the Y axis still has its line in view, so its readout
                // goes against that edge.
                if !plot_rect.x_range().contains(pos.x) {
                    continue;
                }
                let pos = egui::pos2(pos.x, pos.y.max(plot_rect.top()).min(plot_rect.bottom()));
                let galley = layout(
                    format!(
                        "{name}: {t:.2} s / {y_val:.prec$} {}",
                        data.overlay_unit,
                        prec = data.point_decimals
                    ),
                    data.cursor_color,
                );
                if let Some(rect) =
                    cursor_label_rect(pos, galley.size(), plot_rect, &taken, hits_trace)
                {
                    paint(rect, galley, data.cursor_color);
                    taken.push(rect);
                }
            }
        }

        // The mean and reference labels, one column in their lines' order:
        // the mean first among equals, one label for a value entered twice.
        let mut levels: Vec<(f64, String, egui::Color32)> = Vec::new();
        if data.show_mean
            && let Some(avg) = data.mean_value
        {
            levels.push((avg, format!("Mean: {}", level_text(avg)), data.mean_color));
        }
        if data.show_ref {
            let mut refs = data.ref_values.clone();
            refs.sort_by(|a, b| b.total_cmp(a));
            refs.dedup();
            levels.extend(refs.into_iter().map(|v| (v, level_text(v), data.ref_color)));
        }
        let mut levels: Vec<_> = levels
            .into_iter()
            .map(|(v, text, color)| (screen(data.view_max, v).y, layout(text, color), color))
            .collect();
        levels.sort_by(|a, b| a.0.total_cmp(&b.0));
        let ys: Vec<f32> = levels.iter().map(|l| l.0).collect();
        let sizes: Vec<egui::Vec2> = levels.iter().map(|l| l.1.size()).collect();
        let rects = level_label_rects(&ys, &sizes, plot_rect, &taken, &line_xs, hits_trace);
        for ((_, galley, color), rect) in levels.into_iter().zip(rects) {
            if let Some(rect) = rect {
                paint(rect, galley, color);
            }
        }
    }
}
