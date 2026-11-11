//! The hover: the readings under the pointer, one row per drawn series,
//! with the crosshair's height tagged on each axis.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};
use egui_plot::{PlotPoint, PlotTransform};

use super::axes::AxisMap;
use super::render::OverlayTrace;
use super::{Graph, ReadingDigits};
use crate::theme::ThemeColors;
use dmm_lib::transform::PrefixRatio;
use std::time::Instant;

/// The segment spanning time `t`, from segments in time order: found by
/// its start, as the hover asks on every frame the pointer moves.
fn segment_spanning(segments: &[Vec<[f64; 2]>], t: f64) -> Option<&[[f64; 2]]> {
    let after = segments.partition_point(|s| s.first().is_none_or(|p| p[0] <= t));
    let seg = segments.get(after.checked_sub(1)?)?;
    seg.last()
        .is_some_and(|p| p[0] >= t)
        .then_some(seg.as_slice())
}

/// The reading of a drawn trace that answers for time `t`, as `[time,
/// value]`: in the segment spanning `t`, the nearest one — or, for a trace
/// drawn as steps, the level held at `t`, the last reading at or before it,
/// placed at `t` where the step is. `None` in a break between segments.
pub(super) fn reading_at(segments: &[Vec<[f64; 2]>], t: f64, stepped: bool) -> Option<[f64; 2]> {
    let seg = segment_spanning(segments, t)?;
    let i = seg.partition_point(|p| p[0] <= t);
    let before = i.checked_sub(1).and_then(|j| seg.get(j));
    let after = seg.get(i);
    match (before, after) {
        (Some(b), _) if stepped => Some([t, b[1]]),
        (Some(b), Some(a)) => Some(if t - b[0] <= a[0] - t { *b } else { *a }),
        (Some(p), None) | (None, Some(p)) => Some(*p),
        (None, None) => None,
    }
}

/// The reading the pointer at `pos` is on, as its time and its drawn height:
/// among every drawn series, the line passing nearest the pointer on screen,
/// if within `radius` points — between its readings too, as a steep stretch
/// leaves them far apart — and on it, the reading nearest the pointer. The
/// crosshair snaps onto a curve only when the pointer is on one, as
/// egui_plot's did; away from the curves it stays with the pointer, so on a
/// scrolling graph a still pointer keeps a still line. Only a reading in
/// `view` is one to land on: a line crossing the plot from a reading beyond
/// its edge snaps to its end in view, or not at all. `window` is the span of
/// times within `radius` of the pointer, and `to_screen` places a time and a
/// height on screen.
pub(super) fn reading_near(
    series: &[HoverSeries],
    (lo, hi): (f64, f64),
    (view_min, view_max): (f64, f64),
    pos: egui::Pos2,
    radius: f32,
    to_screen: impl Fn(f64, f64) -> egui::Pos2,
) -> Option<[f64; 2]> {
    let mut best: Option<([f64; 2], f32)> = None;
    for s in series {
        let drawn = |p: [f64; 2]| s.drawn(p);
        // The segments overlapping the window, found by their ends: they are
        // in time order.
        let first = s
            .segments
            .partition_point(|g| g.last().is_some_and(|p| p[0] < lo));
        let last = s
            .segments
            .partition_point(|g| g.first().is_none_or(|p| p[0] <= hi));
        for seg in s.segments.get(first..last).unwrap_or_default() {
            // The readings in reach, and one either side for the lines that
            // cross the window from beyond it.
            let from = seg.partition_point(|p| p[0] < lo).saturating_sub(1);
            let to = (seg.partition_point(|p| p[0] <= hi) + 1).min(seg.len());
            let Some(near) = seg.get(from..to) else {
                continue;
            };
            let screen = |p: [f64; 2]| to_screen(p[0], p[1]);
            let mut consider = |piece: (egui::Pos2, egui::Pos2), readings: &[[f64; 2]]| {
                let distance = segment_distance(piece, pos);
                // Off at infinity, as a Y range of "inf" puts a line, is never
                // near.
                if !distance.is_finite()
                    || distance > radius
                    || best.is_some_and(|(_, d)| d <= distance)
                {
                    return;
                }
                let in_view = |p: &[f64; 2]| p[0] >= view_min && p[0] <= view_max;
                let on = readings
                    .iter()
                    .copied()
                    .filter(in_view)
                    .map(drawn)
                    .min_by(|a, b| {
                        screen(*a)
                            .distance(pos)
                            .total_cmp(&screen(*b).distance(pos))
                    });
                if let Some(on) = on {
                    best = Some((on, distance));
                }
            };
            if let [only] = near {
                let at = screen(drawn(*only));
                consider((at, at), &[*only]);
            }
            for pair in near.windows(2) {
                let (a, b) = (drawn(pair[0]), drawn(pair[1]));
                if s.stepped {
                    // A level held, then the riser to the next one.
                    let corner = to_screen(b[0], a[1]);
                    consider((screen(a), corner), &pair[..1]);
                    consider((corner, screen(b)), &pair[1..]);
                } else {
                    consider((screen(a), screen(b)), pair);
                }
            }
        }
    }
    best.map(|(point, _)| point)
}

/// How far `p` is from the segment between `a` and `b`, on screen.
fn segment_distance((a, b): (egui::Pos2, egui::Pos2), p: egui::Pos2) -> f32 {
    let ab = b - a;
    let along = if ab.length_sq() > 0.0 {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * along).distance(p)
}

/// One series in the hover readout: its drawn segments, at their own values,
/// and its unit.
pub(super) struct HoverSeries<'a> {
    pub name: String,
    /// The sub-value its readings are, by the label the meter gives it, or
    /// `None` for the main reading: what its digits are looked up by.
    pub key: Option<String>,
    pub segments: &'a [Vec<[f64; 2]>],
    pub unit: String,
    /// Its line's colour, for its row of the readout and its dot.
    pub color: egui::Color32,
    /// Drawn as steps, a level held until the next reading.
    pub stepped: bool,
    /// Its right axis, for a unit other than the plotted one.
    pub map: Option<AxisMap>,
}

impl HoverSeries<'_> {
    /// A reading at the height its line is drawn: on its right axis's scale.
    fn drawn(&self, [t, v]: [f64; 2]) -> [f64; 2] {
        [t, self.map.map_or(v, |map| map.height_of(v))]
    }
}

/// One row of the hover readout: the index of its series, its text, and the
/// reading it gives, for the dot on the line — none for an overload.
pub(super) type HoverRow = (usize, String, Option<[f64; 2]>);

/// How a row writes a series' reading: its number and its unit.
pub(super) type ValueText<'f> = &'f dyn Fn(&HoverSeries, [f64; 2]) -> (String, String);

/// The rows of the hover readout, after the time: every series with a value
/// at `t` in its own unit, in the key's order, each named when there are
/// several — with one series the number alone says it. The plotted series (first in
/// `series`) reads "overload" inside its band.
pub(super) fn hover_lines(
    l10n: &Localizer,
    series: &[HoverSeries],
    t: f64,
    overload: bool,
    value_text: ValueText,
) -> Vec<HoverRow> {
    let named = series.len() > 1;
    series
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let name = &s.name;
            if i == 0 && overload {
                let text = if named {
                    tr!(l10n, "graph-hover-overload-series", series = name).into_owned()
                } else {
                    tr!(l10n, "graph-hover-overload").into_owned()
                };
                return Some((i, text, None));
            }
            let point = reading_at(s.segments, t, s.stepped)?;
            let (value, unit) = value_text(s, point);
            let text = if named {
                tr!(
                    l10n,
                    "graph-hover-series",
                    series = name,
                    value = value,
                    unit = unit
                )
                .into_owned()
            } else {
                format!("{value} {unit}").trim_end().to_string()
            };
            Some((i, text, Some(point)))
        })
        .collect()
}

/// The decimals a height on an axis takes: as many as one point of the plot
/// resolves, `per_point` being the axis's units per point. `None` on a span
/// with no finite size, such as a Y: Fixed range of "inf".
pub(super) fn height_decimals(per_point: f64) -> Option<usize> {
    (per_point.is_finite() && per_point > 0.0)
        .then(|| (-per_point.log10()).ceil().clamp(0.0, 6.0) as usize)
}

/// The tags where the horizontal crosshair meets the axes, for a pointer at
/// height `y`: the plotted unit's on the left, then one per right axis, each
/// with its index in `maps`.
/// `per_point` is the plotted unit per point of the plot, and `max_decimals`
/// caps the left tag's decimals (a level is a whole number). `None` where
/// the height has no finite value.
pub(super) fn height_tags(
    y: f64,
    per_point: f64,
    unit: &str,
    prefix: Option<&(String, PrefixRatio)>,
    maps: &[(String, AxisMap)],
    max_decimals: usize,
) -> Option<(String, Vec<(usize, String)>)> {
    if !y.is_finite() {
        return None;
    }
    let tag = |value: f64, decimals: usize, unit: &str| {
        let val = eframe::emath::format_with_decimals_in_range(value, decimals..=decimals);
        format!("{val} {unit}").trim_end().to_string()
    };
    // In the prefix the left axis writes its ticks in.
    let (left_y, left_unit) = super::axes::in_prefix(prefix, unit, y);
    let left_per_point = super::axes::in_prefix(prefix, unit, per_point).0.abs();
    let left = tag(
        left_y,
        height_decimals(left_per_point)?.min(max_decimals),
        left_unit,
    );
    let right = maps
        .iter()
        .enumerate()
        .filter_map(|(i, (unit, map))| {
            let value = map.value_at(y);
            let per_point = (map.value_at(y + per_point) - value).abs();
            height_decimals(per_point)
                .filter(|_| value.is_finite())
                .map(|d| (i, tag(value, d, unit)))
        })
        .collect();
    Some((left, right))
}

/// Show the hover readout beside the vertical crosshair at `at` (its x, the
/// pointer's height): the time, then a row per series in its line's colour
/// where that reads as text on the tooltip, as the axes do. To the line's
/// right, or its left where there is no room, never over it: the dots on
/// the line are what tie each row to its trace.
pub(super) fn show_hover_readout(
    plot: &egui::Response,
    at: egui::Pos2,
    time_label: &str,
    series: &[HoverSeries],
    lines: &[HoverRow],
) {
    const SIDES: [egui::RectAlign; 4] = [
        egui::RectAlign::RIGHT_START,
        egui::RectAlign::LEFT_START,
        egui::RectAlign::RIGHT_END,
        egui::RectAlign::LEFT_END,
    ];
    let mut tooltip = egui::Tooltip::always_open(
        plot.ctx.clone(),
        plot.layer_id,
        plot.id,
        egui::PopupAnchor::Position(at),
    );
    let width = plot.ctx.global_style().spacing.tooltip_width;
    tooltip.popup = tooltip
        .popup
        .width(width)
        .align(SIDES[0])
        .align_alternatives(&SIDES);
    tooltip.gap(12.0).show(|ui| {
        ui.set_max_width(width);
        ui.label(time_label);
        let ground = ui.visuals().window_fill;
        for (i, line, _) in lines {
            let color = crate::theme::legible_on(series[*i].color, ground)
                .unwrap_or(ui.visuals().text_color());
            ui.label(egui::RichText::new(line).color(color));
        }
    });
}

/// A dot on each line at the reading its row of the readout gives, ringed in
/// the plot's background so it reads over the grid and the other lines: it
/// ties the row to its line by place, not by colour alone.
pub(super) fn paint_hover_dots(
    ui: &Ui,
    body: egui::Rect,
    transform: &PlotTransform,
    series: &[HoverSeries],
    rows: &[HoverRow],
    halo: egui::Color32,
) {
    let painter = ui.painter().with_clip_rect(body);
    for (i, _, point) in rows {
        let Some(point) = *point else { continue };
        let s = &series[*i];
        let [t, height] = s.drawn(point);
        let at = transform.position_from_point(&PlotPoint::new(t, height));
        painter.circle(at, 3.5, s.color, egui::Stroke::new(1.5, halo));
    }
}

/// The tags where the horizontal crosshair meets the axes, centred on `y`
/// and kept within the plot body's height: `left` against the plot body's
/// left edge, `right` stacked against its right one, each row in its axis's
/// colour. A tag wider than its strip runs into the plot rather than out of
/// `outer`, so the strips never widen on a hover.
pub(super) fn paint_height_tags(
    ui: &Ui,
    (outer, body): (egui::Rect, egui::Rect),
    y: f32,
    left: &[(String, egui::Color32)],
    right: &[(String, egui::Color32)],
    border: egui::Color32,
) {
    const PAD: egui::Vec2 = egui::vec2(3.0, 1.0);
    let painter = ui.painter().with_clip_rect(outer);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let fill = ui.visuals().panel_fill;
    let tag = |rows: &[(String, egui::Color32)], place: &dyn Fn(f32) -> f32| {
        if rows.is_empty() {
            return;
        }
        let galleys: Vec<_> = rows
            .iter()
            .map(|(text, color)| painter.layout_no_wrap(text.clone(), font.clone(), *color))
            .collect();
        let width = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 2.0 * PAD.x;
        let height = galleys.iter().map(|g| g.size().y).sum::<f32>() + 2.0 * PAD.y;
        let top = (y - height / 2.0)
            .min(body.bottom() - height)
            .max(body.top());
        let rect =
            egui::Rect::from_min_size(egui::pos2(place(width), top), egui::vec2(width, height));
        painter.rect(
            rect,
            2.0,
            fill,
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let mut row_top = rect.top() + PAD.y;
        for (galley, (_, color)) in galleys.into_iter().zip(rows) {
            let row_height = galley.size().y;
            painter.galley(egui::pos2(rect.left() + PAD.x, row_top), galley, *color);
            row_top += row_height;
        }
    };
    tag(left, &|width| (body.left() - width).max(outer.left()));
    tag(right, &|width| body.right().min(outer.right() - width));
}

impl Graph {
    /// The series the hover lists, in the key's order: the plotted one,
    /// named `main_name`, then each drawn trace with its right axis
    /// (`maps`, by trace), every one with its unit and by its name on
    /// screen.
    pub(super) fn hover_series<'a>(
        &self,
        l10n: &Localizer,
        tc: &ThemeColors,
        main_name: &str,
        plotted: &'a [Vec<[f64; 2]>],
        traces: &'a [OverlayTrace],
        maps: &[Option<AxisMap>],
    ) -> Vec<HoverSeries<'a>> {
        let plotted = HoverSeries {
            name: main_name.to_string(),
            key: self.current_series.clone(),
            segments: plotted,
            unit: self.current_unit.clone(),
            color: tc.graph_line(),
            stepped: self.levels,
            map: None,
        };
        let traces = traces
            .iter()
            .zip(maps)
            .map(|((k, label, segments), map)| HoverSeries {
                name: self.series_text(l10n, label).into_owned(),
                key: Some(label.clone()),
                segments: segments.as_slice(),
                unit: self.overlay_unit(label).to_string(),
                color: tc.graph_overlay(*k),
                stepped: false,
                map: *map,
            });
        std::iter::once(plotted).chain(traces).collect()
    }

    /// What the hover readout writes for `series`'s reading at `point`
    /// ([`Graph::point_text`]): a reading taken before an auto-range step is
    /// held in the prefix plotted now, where four decimals would round
    /// 0.07 Ω to 0.0000 MΩ.
    pub(super) fn hover_value(
        &self,
        series: &HoverSeries,
        point: [f64; 2],
        digits: ReadingDigits,
    ) -> (String, String) {
        self.point_text(series.key.as_deref(), &series.unit, point, digits)
    }

    /// The reading of series `key` (`None` the main reading) at `point`, in
    /// `unit`, as a number and its unit: the digits the meter showed for
    /// it, else a five-digit figure in a prefix that fits it. A level is its
    /// whole number. What the hover and the cursors' labels write.
    pub(super) fn point_text(
        &self,
        key: Option<&str>,
        unit: &str,
        [t, v]: [f64; 2],
        digits: ReadingDigits,
    ) -> (String, String) {
        // A level is a whole number, with no prefix to lose; the meter's
        // text for it is the export's ("NCV:3").
        if self.levels && key == self.current_series.as_deref() {
            let digits = format!("{v:.prec$}", prec = self.decimals(true));
            return (digits, unit.to_string());
        }
        // The main reading's trace, kept while a sub-value is plotted, has
        // the main reading's digits.
        let digits_of = key.filter(|label| !self.is_main_trace(label));
        if let Some(text) = self
            .reading_time(key, t, v)
            .and_then(|at| digits(at, digits_of))
        {
            // "0.07 Ω": the export's digits keep no space.
            return match text.rsplit_once(' ') {
                Some((number, unit)) => (number.to_string(), unit.to_string()),
                None => (text, String::new()),
            };
        }
        let (number, unit) = dmm_shared::stat_figure(v, unit, None);
        (number, unit.into_owned())
    }

    /// When the reading of value `v` the hover found at `t` seconds was
    /// taken: of the plotted series for `key` the plotted one, else of the
    /// kept trace `key`. The one at or just before `t` with that value, or
    /// just after it.
    fn reading_time(&self, key: Option<&str>, t: f64, v: f64) -> Option<Instant> {
        let mut points: Vec<(Instant, Option<f64>)> = Vec::with_capacity(2);
        if key == self.current_series.as_deref() {
            let around = self.around(&self.history, |p| p.time, t);
            points.extend(self.history.range(around).map(|p| (p.time, Some(p.value))));
        } else {
            let trace = self
                .overlays
                .iter()
                .find(|o| Some(o.label.as_str()) == key)?;
            let around = self.around(&trace.points, |p| p.time, t);
            points.extend(trace.points.range(around).map(|p| (p.time, p.value)));
        }
        points
            .into_iter()
            .find(|&(_, value)| value == Some(v))
            .map(|(at, _)| at)
    }

    /// The indices of the point of `points` at or just before `t` seconds
    /// and the one just after it.
    fn around<P>(
        &self,
        points: &std::collections::VecDeque<P>,
        time: impl Fn(&P) -> Instant,
        t: f64,
    ) -> std::ops::Range<usize> {
        let i = points.partition_point(|p| self.elapsed_secs(time(p)) <= t);
        i.saturating_sub(1)..(i + 1).min(points.len())
    }
}
