//! Dashed and dotted main-graph lines whose pattern moves with the data.
//!
//! egui_plot starts a line's dash pattern at its first point. The main plot
//! draws only the visible slice, so in live view that first point moves
//! with every sample that scrolls off the left edge, and the whole pattern
//! crawled along the curve. Here the pattern is still one continuous walk
//! along each segment at egui_plot's dash, gap and dot lengths, but where it
//! starts is carried from fixed points in time: the phase at every anchor —
//! a multiple of [`anchor_layout`]'s spacing in session time — is
//! remembered, and the next frame starts from the leftmost one it still
//! draws. A sideways shift, live or dragged, then moves the pattern rigidly
//! with the line. A change of scale (an auto-ranged Y axis, a zoom) changes
//! the arc length between anchors, so the pattern holds at the leftmost
//! anchor and re-phases right of it, once per change.
//!
//! A horizontal line — the mean, a reference value, a cursor's level — is
//! simpler: its arc length is its run along the time axis, so its pattern
//! is set by session time alone ([`TimedHLine`]) and scrolls with the data
//! instead of standing still at the plot's left edge.
//!
//! The slice the patterned lines are built from reaches back to
//! [`anchor_layout`]'s `draw_min`: the anchor at or left of the view's edge
//! stays on the drawn line, and every thinning bucket after it is whole, so
//! the drawn points — and the arc lengths measured along them — are the
//! same from frame to frame.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Ui};
use egui_plot::{
    ClosestElem, Cursor, HLine, LabelFormatterFn, Line, LineStyle, PlotBounds, PlotConfig,
    PlotGeometry, PlotItem, PlotItemBase, PlotPoint, PlotTransform, PlotUi,
};

/// Rough spacing of the anchors, in points: a few times the longest
/// pattern, so a frame has only a handful per line to remember.
const ANCHOR_PT: f64 = 150.0;

/// egui_plot's gap after a dash of length 1: the golden ratio's fraction.
pub(super) const DASH_GAP: f32 = std::f32::consts::GOLDEN_RATIO - 1.0;

/// The anchor spacing in seconds and where the patterned lines' slice
/// starts, for a view starting at `view_min` drawn with thinning buckets of
/// `bucket` seconds, `secs_per_pt` seconds to the point.
///
/// The spacing is a whole number of buckets about [`ANCHOR_PT`] wide, so it
/// steps only when the bucket does. The slice starts a bucket before the
/// anchor at or left of `view_min`: the builders keep one point before it
/// too, alone in its partial bucket, so every bucket thinned from there on
/// is complete.
pub(super) fn anchor_layout(view_min: f64, bucket: f64, secs_per_pt: f64) -> (f64, f64) {
    let bucket_pt = bucket / secs_per_pt;
    let buckets = if bucket_pt.is_finite() && bucket_pt > 0.0 {
        (ANCHOR_PT / bucket_pt).ceil().max(1.0)
    } else {
        1.0
    };
    let anchor = buckets * bucket;
    let draw_min = (view_min / anchor).floor() * anchor - bucket;
    (anchor, draw_min)
}

/// Which patterned line a phase belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum PatternKey {
    /// A sub-value trace, by label.
    Overlay(String),
    EnvelopeMax,
    EnvelopeMin,
}

/// One line's remembered phases.
#[derive(Default, Debug)]
struct LinePhases {
    /// Pattern position, in points from the start of a period, at each
    /// anchor time (µs from the origin).
    anchors: BTreeMap<i64, f64>,
    /// Drawn since the last [`DashPhases::begin_frame`].
    drawn: bool,
}

/// Where each patterned line's pattern stands at its anchors.
#[derive(Default, Debug)]
pub(super) struct DashPhases {
    lines: HashMap<PatternKey, LinePhases>,
}

fn anchor_key(secs: f64) -> i64 {
    (secs * 1e6).round() as i64
}

impl DashPhases {
    /// Forget what this frame can no longer use: anchors outside
    /// `[from, to]` seconds, and the lines the last frame did not draw.
    pub(super) fn begin_frame(&mut self, from: f64, to: f64) {
        let range = anchor_key(from)..=anchor_key(to);
        self.lines.retain(|_, line| {
            line.anchors.retain(|k, _| range.contains(k));
            std::mem::take(&mut line.drawn)
        });
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[cfg(test)]
    pub(super) fn phase_at(&self, key: &PatternKey, secs: f64) -> Option<f64> {
        self.lines.get(key)?.anchors.get(&anchor_key(secs)).copied()
    }

    /// The phase one segment of `key` starts at, and the phases at its
    /// anchors remembered for the next frame.
    ///
    /// `times` are the segment's points' session times, `screen` the same
    /// points on screen, both in time order; `period` is the pattern's
    /// length in points. `starts_here` says the segment's first point is
    /// where it really begins, which then sits on a whole dash or dot.
    /// Otherwise the leftmost remembered anchor on the segment sets the
    /// phase, or, with none (a jump), the segment's first point does.
    pub(super) fn carry(
        &mut self,
        key: &PatternKey,
        times: &[f64],
        screen: &[Pos2],
        anchor_secs: f64,
        period: f64,
        starts_here: bool,
    ) -> f64 {
        let line = self.lines.entry(key.clone()).or_default();
        line.drawn = true;
        let (Some(&first), Some(&last)) = (times.first(), times.last()) else {
            return 0.0;
        };
        let usable = screen.iter().all(|p| p.is_finite())
            && anchor_secs > 0.0
            && period > 0.0
            && first.is_finite()
            && last.is_finite();
        if !usable {
            return 0.0;
        }
        let arc = ArcLength::new(times, screen);
        let span = anchor_key(first)..=anchor_key(last);
        let reference = if starts_here {
            None
        } else {
            line.anchors
                .range(span.clone())
                .next()
                .map(|(&k, &phase)| (k, phase))
        };
        // Anchors of an earlier spacing, before a zoom or a resize, hold
        // phases the new grid no longer updates: drop them, or one would
        // later take over as the reference with a stale phase.
        let stale: Vec<i64> = line
            .anchors
            .range(span)
            .map(|(&k, _)| k)
            .filter(|&k| reference.is_none_or(|(r, _)| r != k))
            .collect();
        for k in stale {
            line.anchors.remove(&k);
        }
        let phase0 = match reference {
            Some((k, phase)) => (phase - arc.at(k as f64 / 1e6)).rem_euclid(period),
            None => 0.0,
        };
        let (k_first, k_last) = (
            (first / anchor_secs).ceil() as i64,
            (last / anchor_secs).floor() as i64,
        );
        for k in k_first..=k_last {
            let key = anchor_key(k as f64 * anchor_secs);
            if reference.is_some_and(|(r, _)| r == key) {
                continue;
            }
            let phase = (phase0 + arc.at(key as f64 / 1e6)).rem_euclid(period);
            if phase.is_finite() {
                line.anchors.insert(key, phase);
            }
        }
        phase0
    }
}

/// Arc length along a polyline on screen, measured from its first point, at
/// a session time.
struct ArcLength<'a> {
    times: &'a [f64],
    screen: &'a [Pos2],
    /// Arc length at each point.
    sums: Vec<f64>,
}

impl<'a> ArcLength<'a> {
    fn new(times: &'a [f64], screen: &'a [Pos2]) -> Self {
        let mut sums = Vec::with_capacity(screen.len());
        let mut sum = 0.0;
        sums.push(0.0);
        for pair in screen.windows(2) {
            sum += f64::from((pair[1] - pair[0]).length());
            sums.push(sum);
        }
        Self {
            times,
            screen,
            sums,
        }
    }

    fn at(&self, t: f64) -> f64 {
        let i = self.times.partition_point(|&x| x <= t).max(1) - 1;
        let Some(&next) = self.times.get(i + 1) else {
            return self.sums[i];
        };
        let span = next - self.times[i];
        let frac = if span > 0.0 {
            ((t - self.times[i]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.sums[i] + frac * f64::from((self.screen[i + 1] - self.screen[i]).length())
    }
}

/// A pattern's period in points; `None` for a solid line.
pub(super) fn period(style: LineStyle) -> Option<f64> {
    match style {
        LineStyle::Solid => None,
        LineStyle::Dashed { length } => Some(f64::from(length * (1.0 + DASH_GAP))),
        LineStyle::Dotted { spacing } => Some(f64::from(spacing)),
    }
}

/// `phase` brought into `[0, period)`.
fn wrap(phase: f64, period: f64) -> f64 {
    let phase = phase.rem_euclid(period);
    // `rem_euclid` can round a tiny negative up to the period itself.
    if phase.is_nan() || phase >= period {
        0.0
    } else {
        phase
    }
}

/// Draw `points` in `style` with the pattern `phase0` points into its
/// period at the first point, as egui_plot draws it from phase 0: a dash
/// as one segment per stretch between points, a dot as a disc as wide as
/// the stroke, a lone point as a disc half as wide. A stretch wholly
/// outside `clip` only advances the pattern: the slice reaches left of the
/// plot, and nothing drawn there would show.
pub(super) fn walk_pattern(
    points: &[Pos2],
    phase0: f64,
    style: LineStyle,
    mut stroke: Stroke,
    highlight: bool,
    clip: Rect,
    out: &mut Vec<Shape>,
) {
    let Some(period) = period(style) else {
        return;
    };
    // A point off at infinity, as a Y range of "inf" puts it, would never be
    // walked past; egui draws nothing of such a line either.
    if points.iter().any(|p| !p.is_finite()) {
        return;
    }
    if let [only] = points {
        let mut radius = stroke.width / 2.0;
        if highlight {
            radius *= std::f32::consts::SQRT_2;
        }
        out.push(Shape::circle_filled(*only, radius, stroke.color));
        return;
    }
    let mut phase = wrap(phase0, period);
    // A repeated point has nowhere to place a dash end or a dot.
    let stretches = points
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .filter(|(a, b)| a != b);
    match style {
        LineStyle::Dashed { length } => {
            if highlight {
                stroke.width *= 2.0;
            }
            let dash = f64::from(length);
            let mut dash_from = (phase < dash).then(|| points.first().copied()).flatten();
            for (a, b) in stretches {
                let len = f64::from((b - a).length());
                if !clip.intersects(Rect::from_two_pos(a, b)) {
                    phase = wrap(phase + len, period);
                    dash_from = (phase < dash).then_some(b);
                    continue;
                }
                let mut pos = 0.0;
                loop {
                    let to_event = if phase < dash {
                        dash - phase
                    } else {
                        period - phase
                    };
                    if pos + to_event > len {
                        phase += len - pos;
                        break;
                    }
                    pos += to_event;
                    let at = a + (b - a) * (pos / len) as f32;
                    if phase < dash {
                        if let Some(from) = dash_from.take() {
                            out.push(Shape::line_segment([from, at], stroke));
                        }
                        phase = dash;
                    } else {
                        dash_from = Some(at);
                        phase = 0.0;
                    }
                }
                if let Some(from) = dash_from
                    && from != b
                {
                    out.push(Shape::line_segment([from, b], stroke));
                }
                if dash_from.is_some() {
                    dash_from = Some(b);
                }
            }
        }
        LineStyle::Dotted { .. } => {
            let mut radius = stroke.width;
            if highlight {
                radius *= std::f32::consts::SQRT_2;
            }
            if phase == 0.0
                && let Some(&first) = points.first()
            {
                out.push(Shape::circle_filled(first, radius, stroke.color));
            }
            for (a, b) in stretches {
                let len = f64::from((b - a).length());
                if !clip.intersects(Rect::from_two_pos(a, b)) {
                    phase = wrap(phase + len, period);
                    continue;
                }
                let mut pos = 0.0;
                loop {
                    let to_event = period - phase;
                    if pos + to_event > len {
                        phase += len - pos;
                        break;
                    }
                    pos += to_event;
                    let at = a + (b - a) * (pos / len) as f32;
                    out.push(Shape::circle_filled(at, radius, stroke.color));
                    phase = 0.0;
                }
            }
        }
        LineStyle::Solid => {}
    }
}

/// Where a line in `stroke` can show: the plot's frame, widened by the
/// largest dash or dot a highlighted line draws.
fn clip_for(transform: &PlotTransform, stroke: Stroke) -> Rect {
    transform.frame().expand(2.0 * stroke.width)
}

/// An egui_plot line drawn with a pattern carried by [`DashPhases`].
///
/// Everything but the drawing — hover, the crosshair's snapping, bounds —
/// is the wrapped [`Line`]'s, built as for a solid line.
struct PatternedLine<'a> {
    line: Line<'a>,
    stroke: Stroke,
    style: LineStyle,
    key: PatternKey,
    anchor_secs: f64,
    starts_here: bool,
    phases: &'a RefCell<DashPhases>,
}

/// What the main plot's lines need to carry their patterns this frame:
/// the anchor spacing from [`anchor_layout`] and the phase table.
#[derive(Clone, Copy)]
pub(super) struct Patterns<'a> {
    pub(super) anchor_secs: f64,
    pub(super) phases: &'a RefCell<DashPhases>,
}

impl<'a> Patterns<'a> {
    /// Add `line`, drawn in `color`, to the plot: as it is when `style` is
    /// solid, else carrying its pattern under `key`. See
    /// [`DashPhases::carry`] for `starts_here`.
    pub(super) fn line(
        &self,
        plot_ui: &mut PlotUi<'a>,
        line: Line<'a>,
        color: Color32,
        style: LineStyle,
        key: PatternKey,
        starts_here: bool,
    ) {
        let line = line.color(color);
        if style == LineStyle::Solid {
            plot_ui.line(line);
            return;
        }
        // `PlotUi::add` would keep an empty line, which `PlotUi::line` drops.
        if matches!(PlotItem::geometry(&line), PlotGeometry::Points(p) if p.is_empty()) {
            return;
        }
        plot_ui.add(PatternedLine {
            line,
            // egui_plot's default line width.
            stroke: Stroke::new(1.5, color),
            style,
            key,
            anchor_secs: self.anchor_secs,
            starts_here,
            phases: self.phases,
        });
    }
}

impl PlotItem for PatternedLine<'_> {
    fn shapes(&self, _ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let PlotGeometry::Points(points) = PlotItem::geometry(&self.line) else {
            return;
        };
        let Some(period) = period(self.style) else {
            return;
        };
        let screen: Vec<Pos2> = points
            .iter()
            .map(|p| transform.position_from_point(p))
            .collect();
        let times: Vec<f64> = points.iter().map(|p| p.x).collect();
        let phase0 = self.phases.borrow_mut().carry(
            &self.key,
            &times,
            &screen,
            self.anchor_secs,
            period,
            self.starts_here,
        );
        walk_pattern(
            &screen,
            phase0,
            self.style,
            self.stroke,
            PlotItem::highlighted(&self.line),
            clip_for(transform, self.stroke),
            shapes,
        );
    }

    fn initialize(&mut self, x_range: RangeInclusive<f64>) {
        PlotItem::initialize(&mut self.line, x_range);
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotItem::geometry(&self.line)
    }

    fn bounds(&self) -> PlotBounds {
        PlotItem::bounds(&self.line)
    }

    fn base(&self) -> &PlotItemBase {
        PlotItem::base(&self.line)
    }

    fn base_mut(&mut self) -> &mut PlotItemBase {
        PlotItem::base_mut(&mut self.line)
    }

    fn find_closest(&self, point: Pos2, transform: &PlotTransform) -> Option<ClosestElem> {
        PlotItem::find_closest(&self.line, point, transform)
    }

    fn on_hover(
        &self,
        plot_area_response: &egui::Response,
        elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        label_formatter: Option<&LabelFormatterFn<'_>>,
    ) {
        PlotItem::on_hover(
            &self.line,
            plot_area_response,
            elem,
            shapes,
            cursors,
            plot,
            label_formatter,
        );
    }
}

/// A horizontal line across the plot whose pattern is tied to session time:
/// at every point it stands where a pattern begun at time zero would, so it
/// scrolls with the data instead of standing still at the plot's left edge.
///
/// Everything but the drawing is the wrapped [`HLine`]'s.
pub(super) struct TimedHLine {
    line: HLine,
    y: f64,
    stroke: Stroke,
    style: LineStyle,
}

impl TimedHLine {
    pub(super) fn new(y: f64, color: Color32, style: LineStyle) -> Self {
        Self {
            line: HLine::new("", y).color(color),
            y,
            // egui_plot's default `HLine` width.
            stroke: Stroke::new(1.0, color),
            style,
        }
    }

    /// Draw the line `width` points wide.
    pub(super) fn width(mut self, width: f32) -> Self {
        self.line = self.line.width(width);
        self.stroke.width = width;
        self
    }
}

impl PlotItem for TimedHLine {
    fn shapes(&self, ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let Some(period) = period(self.style) else {
            PlotItem::shapes(&self.line, ui, transform, shapes);
            return;
        };
        let bounds = transform.bounds();
        let (left, right) = (bounds.min()[0], bounds.max()[0]);
        let px_per_s = f64::from(transform.frame().width()) / (right - left);
        let points = [
            transform.position_from_point(&PlotPoint::new(left, self.y)),
            transform.position_from_point(&PlotPoint::new(right, self.y)),
        ];
        walk_pattern(
            &points,
            (left * px_per_s).rem_euclid(period),
            self.style,
            self.stroke,
            PlotItem::highlighted(&self.line),
            clip_for(transform, self.stroke),
            shapes,
        );
    }

    fn initialize(&mut self, x_range: RangeInclusive<f64>) {
        PlotItem::initialize(&mut self.line, x_range);
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotItem::geometry(&self.line)
    }

    fn bounds(&self) -> PlotBounds {
        PlotItem::bounds(&self.line)
    }

    fn base(&self) -> &PlotItemBase {
        PlotItem::base(&self.line)
    }

    fn base_mut(&mut self) -> &mut PlotItemBase {
        PlotItem::base_mut(&mut self.line)
    }
}
