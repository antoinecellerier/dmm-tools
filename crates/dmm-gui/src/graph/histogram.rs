//! The histogram pane: how the plotted series' visible readings spread over
//! the Y range on screen, docked beside the plot (sharing its Y axis) or, in
//! a graph too narrow for that, under the minimap with an axis of its own.
//!
//! The bins are cut from the Y range the plot shows, so the side strip's
//! bars sit at the heights of the values they count, and values the axis
//! leaves out are counted above and below it rather than dropped. Meter
//! readings come in steps of their last digit; a bin a whole number of steps
//! wide, with its edges halfway between two steps, never falls between
//! readings and draws empty.
//!
//! With both cursors placed, the readings between them are counted on the
//! same bins and outlined over the bars. Each bar's length is its share of
//! its own readings, so a period shorter, longer or apart from the view
//! still compares with it.

use dmm_lib::stats::RunningStats;
use eframe::egui::{self, Color32, Rect, Sense, Ui};
use egui_plot::{PlotPoint, PlotTransform};

/// Width of the side strip, bars and their σ caption, in points, until
/// dragged, and the narrowest it is drawn at: room for the caption.
pub(crate) const STRIP_WIDTH: f32 = 90.0;
pub(crate) const MIN_STRIP_WIDTH: f32 = 70.0;
/// Height of the band under the minimap, its value labels included, until
/// dragged, and the least it is drawn at.
pub(crate) const BAND_HEIGHT: f32 = 90.0;
pub(crate) const MIN_BAND_HEIGHT: f32 = 50.0;
/// Space between the plot and the strip, or the minimap and the band: the
/// handle that resizes the pane.
pub(super) const DIVIDER: f32 = 6.0;
/// The thinnest bar worth drawing, in points: fewer, wider bins below it.
const MIN_BAR: f32 = 4.0;
/// Bins kept at most, whatever the pane's size.
const MAX_BINS: usize = 200;
/// Readings sampled to find the step they come in.
const QUANTUM_SAMPLE: usize = 256;
/// The finest step looked for, as decimals.
const MAX_DECIMALS: i32 = 6;

/// `v` to `d` decimals, never as `-0`: an axis padded below zero ends a
/// hair under it.
fn fixed(v: f64, d: usize) -> String {
    let scale = 10f64.powi(d as i32);
    format!("{:.d$}", (v * scale).round() / scale + 0.0)
}

/// Where the pane sits this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dock {
    /// Right of the plot, bars growing rightwards from its Y axis heights.
    Side,
    /// Under the minimap, values across and bars growing upwards.
    Below,
}

/// The bins a `pixels`-long axis is cut into at most, [`MIN_BAR`] each.
pub(super) fn target_bins(pixels: f32) -> usize {
    ((pixels / MIN_BAR).floor() as usize).clamp(1, MAX_BINS)
}

/// Where the pane goes this frame, and how big it is across its divider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placement {
    pub(super) dock: Dock,
    /// The strip's width or the band's height, as drawn.
    pub(super) size: f32,
    /// The most a drag can make it.
    pub(super) max: f32,
}

/// Place the pane in a graph `width` wide, where a plot wide enough to keep
/// the strip beside it leaves `beside`, and `room` is the height above the
/// minimap. `sizes` are the strip width and band height as last dragged.
/// The strip goes beside the plot when it fits at its narrowest, up to half
/// the graph; else the band goes under the minimap, up to half the room so
/// it never outgrows the plot, and at its least however short the room is:
/// the column the graph sits in scrolls to make room for it.
pub(super) fn place(width: f32, beside: f32, room: f32, (strip, band): (f32, f32)) -> Placement {
    let (dock, size, min, max) = if docks_beside(beside) {
        (Dock::Side, strip, MIN_STRIP_WIDTH, beside.min(width / 2.0))
    } else {
        (Dock::Below, band, MIN_BAND_HEIGHT, room / 2.0)
    };
    let max = max.max(min);
    Placement {
        dock,
        size: size.clamp(min, max),
        max,
    }
}

/// Whether a plot that leaves the strip `beside` takes it beside, at its
/// narrowest at least.
pub(super) fn docks_beside(beside: f32) -> bool {
    beside >= MIN_STRIP_WIDTH
}

/// The divider before the pane, between the plot and the strip or the
/// minimap and the band: dragged, or focused and moved with the arrows, it
/// resizes the pane. The new size when it moved, so a window too small for
/// the size dragged draws it smaller without forgetting it.
pub(super) fn divider(ui: &Ui, rect: Rect, placement: Placement) -> Option<f32> {
    use crate::a11y::{ResizeAxis, ResponseA11yExt as _};
    // Moving the divider towards the plot grows the pane: left of the strip,
    // up over the band. `arrow_resize` counts Right and Up as positive.
    let (axis, label, toward_plot, key_growth, cursor, line, min) = match placement.dock {
        Dock::Side => (
            ResizeAxis::Horizontal,
            "Resize the histogram (Left/Right to adjust)",
            egui::vec2(-1.0, 0.0),
            -1.0,
            egui::CursorIcon::ResizeHorizontal,
            [rect.center_top(), rect.center_bottom()],
            MIN_STRIP_WIDTH,
        ),
        Dock::Below => (
            ResizeAxis::Vertical,
            "Resize the histogram (Up/Down to adjust)",
            egui::vec2(0.0, -1.0),
            1.0,
            egui::CursorIcon::ResizeVertical,
            [rect.left_center(), rect.right_center()],
            MIN_BAND_HEIGHT,
        ),
    };
    let id = ui.scope_id().with("histogram_resize");
    let response = ui.interact(rect, id, Sense::drag()).a11y_label(label);
    let keys = crate::a11y::arrow_resize(ui.ctx(), id, axis, 10.0);
    let growth = response.drag_delta().dot(toward_plot) + keys * key_growth;
    crate::a11y::paint_focus_ring(ui, &response);
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(cursor);
        ui.painter()
            .line_segment(line, ui.style().interact(&response).fg_stroke);
    }
    (growth != 0.0).then(|| (placement.size + growth).clamp(min, placement.max))
}

/// Readings counted on the histogram's bins.
#[derive(Debug, Clone)]
struct Counts {
    counts: Vec<u32>,
    /// Readings above and below the Y range shown.
    above: usize,
    below: usize,
    /// Every reading counted, in range or not.
    stats: RunningStats,
    /// Over-range readings among them, which have no value to bin.
    overloads: u32,
}

impl Counts {
    fn tallest(&self) -> u32 {
        self.counts.iter().copied().max().unwrap_or(0)
    }

    /// Bin `count`'s share of every reading counted.
    fn share(&self, count: u32) -> f64 {
        f64::from(count) / self.stats.count.max(1) as f64
    }
}

/// The visible readings binned over the Y range shown, and those between
/// the cursors on the same bins.
#[derive(Debug, Clone)]
pub(super) struct Histogram {
    /// Bin `i` holds values from `origin + (first + i)·width − quantum/2` up
    /// to the next bin's edge.
    first: i64,
    width: f64,
    /// The step the readings come in, or 0 where none was found, and where
    /// its lattice sits: readings are `origin + k·quantum`.
    quantum: f64,
    origin: f64,
    /// The Y range shown, which readings beyond count above or below.
    range: (f64, f64),
    view: Counts,
    /// The readings between the cursors, once both are placed.
    interval: Option<Counts>,
    /// Decimals the readings are written with.
    decimals: usize,
    /// Decimals σ is written with: the graph's for a mean, as Statistics
    /// writes it, finer than the readings' step it may fall under.
    sigma_decimals: usize,
}

/// The step `values` come in, as a power of ten, from an even sample of
/// them: the fewest decimals that write each to within float32 precision,
/// which is how several meters send their readings. `None` for readings with
/// more decimals than that precision or [`MAX_DECIMALS`] can tell apart — a
/// scale factor of 1/3, a computed value — which are binned on round steps
/// alone.
pub(super) fn quantum_of<I>(values: I) -> Option<(f64, usize)>
where
    I: ExactSizeIterator<Item = f64> + Clone,
{
    if values.len() == 0 {
        return None;
    }
    let stride = (values.len() / QUANTUM_SAMPLE).max(1);
    let sample = || values.clone().step_by(stride).filter(|v| v.is_finite());
    (0..=MAX_DECIMALS).find_map(|k| {
        let scale = 10f64.powi(k);
        sample()
            .all(|v| {
                let scaled = v * scale;
                // A float32 reading widened to f64 misses its step by up to
                // half a float32 ulp. Past the decimals float32 resolves,
                // every value would pass: no step can be told there.
                let slack = 1e-9 + scaled.abs() * f64::from(f32::EPSILON);
                slack < 0.05 && (scaled - scaled.round()).abs() <= slack
            })
            .then(|| (1.0 / scale, k as usize))
    })
}

/// Distinct values the sample must show before a step coarser than its
/// decimals is believed: a reading flicking between two values is no
/// evidence of one.
const MIN_LATTICE_VALUES: usize = 8;

/// The lattice readings sit on, as `(step, origin, decimals)`: readings are
/// `origin + k·step`. The step is the decimal one [`quantum_of`] finds, or a
/// whole number of it where every sampled reading is: a scale factor makes
/// 0.1 °C steps into 0.18 °F ones, written with two decimals, and bins of
/// whole 0.01 steps would catch one reading here and two there.
fn lattice_of<I>(values: I) -> Option<(f64, f64, usize)>
where
    I: ExactSizeIterator<Item = f64> + Clone,
{
    let (quantum, decimals) = quantum_of(values.clone())?;
    let stride = (values.len() / QUANTUM_SAMPLE).max(1);
    let mut units: Vec<i64> = values
        .step_by(stride)
        .filter(|v| v.is_finite())
        .map(|v| (v / quantum).round() as i64)
        .collect();
    units.sort_unstable();
    units.dedup();
    if units.len() < MIN_LATTICE_VALUES {
        return Some((quantum, 0.0, decimals));
    }
    let gcd = |mut a: i64, mut b: i64| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let steps = units.windows(2).fold(0, |g, w| gcd(g, w[1] - w[0])).max(1);
    let origin = units[0].rem_euclid(steps) as f64 * quantum;
    Some((quantum * steps as f64, origin, decimals))
}

/// Readings a bin holds on average, at least: no more bins across the
/// readings' range than they can fill. Picked against √n bins and numpy's
/// `auto` on signals from a steady reading to noise with spikes, phase-locked
/// sines and a mains recording: the fewest combs of empty bins, and a
/// meter's step still gets a bin each once there are enough readings.
const READINGS_PER_BIN: f64 = 5.0;

/// The bin width the readings call for: their range, from the even sample
/// [`quantum_of`] reads, over one bin per [`READINGS_PER_BIN`] of them.
/// `None` for readings with no spread, which the pane's size alone bins.
fn data_width<I>(values: I) -> Option<f64>
where
    I: ExactSizeIterator<Item = f64> + Clone,
{
    let n = values.len() as f64;
    let stride = (values.len() / QUANTUM_SAMPLE).max(1);
    let (low, high) = values
        .step_by(stride)
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    (high > low).then(|| READINGS_PER_BIN * (high - low) / n)
}

impl Histogram {
    /// Bin `values` over `lo..=hi`: as many bins as the readings call for
    /// (see [`data_width`]), `target` at most, each a 1, 2 or 5 multiple of
    /// the step they come in, or a round width without one. `point_decimals` writes the edges when the
    /// readings show no step, `mean_decimals` σ. `None` with no finite range
    /// to bin over.
    pub(super) fn new<I>(
        values: I,
        (lo, hi): (f64, f64),
        target: usize,
        (point_decimals, mean_decimals): (usize, usize),
    ) -> Option<Self>
    where
        I: ExactSizeIterator<Item = f64> + Clone,
    {
        if !(lo.is_finite() && hi.is_finite() && hi > lo) {
            return None;
        }
        // A step as wide as the axis (a steady reading, its range padded
        // around it) would make one bin of the whole strip: bin on round
        // steps instead, and the reading draws as one thin bar, written to
        // the graph's own decimals at least — a steady 5.000 V has none.
        let (quantum, origin, decimals) = match lattice_of(values.clone()) {
            Some((q, o, k)) if q * 2.0 <= hi - lo => (q, o, k),
            Some((_, _, k)) => (0.0, 0.0, k.max(point_decimals)),
            None => (0.0, 0.0, point_decimals),
        };
        let wanted = data_width(values.clone())
            .unwrap_or(0.0)
            .max((hi - lo) / target as f64);
        let width = if quantum > 0.0 {
            quantum * super::axes::nice_step((wanted / quantum).max(1.0))
        } else {
            super::axes::nice_step(wanted)
        };
        let first = ((lo - origin + quantum / 2.0) / width).floor() as i64;
        let mut hist = Self {
            first,
            width,
            quantum,
            origin,
            range: (lo, hi),
            view: Counts {
                counts: Vec::new(),
                above: 0,
                below: 0,
                stats: RunningStats::new(),
                overloads: 0,
            },
            interval: None,
            decimals,
            sigma_decimals: mean_decimals,
        };
        hist.view = hist.count(values);
        Some(hist)
    }

    /// Whether the readings between the cursors are counted.
    #[cfg(test)]
    pub(super) fn has_interval(&self) -> bool {
        self.interval.is_some()
    }

    /// Count the readings between the cursors on the same bins.
    pub(super) fn count_interval(&mut self, values: impl Iterator<Item = f64>, overloads: u32) {
        self.interval = Some(Counts {
            overloads,
            ..self.count(values)
        });
    }

    /// The view's over-range readings, which the meter sent no value for.
    pub(super) fn with_overloads(mut self, overloads: u32) -> Self {
        self.view.overloads = overloads;
        self
    }

    fn count(&self, values: impl Iterator<Item = f64>) -> Counts {
        let (lo, hi) = self.range;
        let index = |v: f64| ((v - self.origin + self.quantum / 2.0) / self.width).floor() as i64;
        let bins = (index(hi) - self.first + 1).clamp(1, MAX_BINS as i64 * 3) as usize;
        let mut counts = vec![0u32; bins];
        let (mut above, mut below) = (0, 0);
        let mut stats = RunningStats::new();
        // Per value: an offset from bin 0's lower edge times the inverse
        // width, cut by a cast. A value in range is at or above that edge,
        // so the cast floors it, without `floor` or a division per value.
        let base = self.origin + self.first as f64 * self.width - self.quantum / 2.0;
        let per_bin = 1.0 / self.width;
        let last = bins - 1;
        for v in values.filter(|v| v.is_finite()) {
            stats.push(v);
            if v < lo {
                below += 1;
            } else if v > hi {
                above += 1;
            } else {
                counts[(((v - base) * per_bin) as usize).min(last)] += 1;
            }
        }
        Counts {
            counts,
            above,
            below,
            stats,
            overloads: 0,
        }
    }

    /// Bin `i`'s edges, lower first.
    fn edges(&self, i: usize) -> (f64, f64) {
        let low = self.origin + (self.first + i as i64) as f64 * self.width - self.quantum / 2.0;
        (low, low + self.width)
    }

    /// The bin `value` falls in, if one is drawn.
    fn bin_of(&self, value: f64) -> Option<usize> {
        let i =
            ((value - self.origin + self.quantum / 2.0) / self.width).floor() as i64 - self.first;
        usize::try_from(i)
            .ok()
            .filter(|&i| i < self.view.counts.len())
    }

    /// The mean of every visible reading, which the Mean overlay draws.
    pub(super) fn mean(&self) -> Option<f64> {
        self.view.stats.avg()
    }

    /// The largest share a bin holds, of the view or the interval: the
    /// share a full-length bar stands for.
    fn peak_share(&self) -> f64 {
        [Some(&self.view), self.interval.as_ref()]
            .into_iter()
            .flatten()
            .map(|c| c.share(c.tallest()))
            .fold(0.0, f64::max)
    }

    /// The readings bin `i` holds, as its hover names them: one value when a
    /// bin is one step wide, else the first and last a reading can take.
    fn range_label(&self, i: usize, unit: &str) -> String {
        let d = self.decimals;
        let (low, high) = self.edges(i);
        let (first, last) = (low + self.quantum / 2.0, high - self.quantum / 2.0);
        if self.quantum > 0.0 && (last - first).abs() < self.quantum / 2.0 {
            format!("{first:.d$} {unit}")
        } else if self.quantum > 0.0 {
            format!("{first:.d$} to {last:.d$} {unit}")
        } else {
            format!("{low:.d$} to {high:.d$} {unit}")
        }
    }

    /// What hovering bin `i` says: its range, then the readings it holds of
    /// the view and of the interval, with their shares.
    fn hover_text(&self, i: usize, unit: &str) -> String {
        let held = |c: &Counts| {
            let n = c.counts[i];
            format!(
                "{n} reading{} ({:.1}%)",
                if n == 1 { "" } else { "s" },
                c.share(n) * 100.0
            )
        };
        let mut text = format!("{}: {}", self.range_label(i, unit), held(&self.view));
        if let Some(interval) = &self.interval {
            text.push_str(&format!("\nA\u{2013}B: {}", held(interval)));
        }
        text
    }

    /// The shares of readings beyond the top (`above`) or bottom of the Y
    /// range: the view's, then the interval's while it is counted, which a
    /// period apart from the view can have all of. `None` for none beyond.
    fn beyond(&self, above: bool) -> (Option<f64>, Option<f64>) {
        let share = |c: &Counts| {
            let n = if above { c.above } else { c.below };
            (n > 0).then(|| n as f64 / c.stats.count as f64)
        };
        (share(&self.view), self.interval.as_ref().and_then(share))
    }

    /// The shares of over-range readings among every reading the view and
    /// the interval took, OL included. `None` for none.
    fn overload_shares(&self) -> (Option<f64>, Option<f64>) {
        let share = |c: &Counts| {
            let total = c.stats.count + u64::from(c.overloads);
            (c.overloads > 0).then(|| f64::from(c.overloads) / total as f64)
        };
        (share(&self.view), self.interval.as_ref().and_then(share))
    }

    /// What a screen reader hears of the pane, appended to the plot's label.
    pub(super) fn spoken(&self, unit: &str) -> String {
        let d = self.sigma_decimals;
        let mut text = format!(" Histogram of {} visible samples", self.view.stats.count);
        if let Some(sd) = self.view.stats.std_dev() {
            text.push_str(&format!(", standard deviation {sd:.d$} {unit}"));
        }
        let most = self
            .view
            .counts
            .iter()
            .enumerate()
            .max_by_key(|&(i, &c)| (c, std::cmp::Reverse(i)))
            .filter(|&(_, &c)| c > 0);
        if let Some((i, &c)) = most {
            text.push_str(&format!(", most at {} ({c})", self.range_label(i, unit)));
        }
        if self.view.overloads > 0 {
            text.push_str(&format!(", {} over range", self.view.overloads));
        }
        if self.view.above + self.view.below > 0 {
            text.push_str(&format!(
                ", {} above and {} below the axis",
                self.view.above, self.view.below
            ));
        }
        if let Some(interval) = &self.interval {
            text.push_str(&format!(
                "; {} samples between the cursors",
                interval.stats.count
            ));
            if let Some(sd) = interval.stats.std_dev() {
                text.push_str(&format!(", standard deviation {sd:.d$} {unit}"));
            }
        }
        text.push('.');
        text
    }
}

/// The colours the pane draws with; each line across it brings its own.
pub(super) struct HistogramColors {
    pub(super) bar: Color32,
    /// The interval's outline and counts, as the cursor lines are drawn.
    pub(super) cursor: Color32,
    pub(super) background: Color32,
    /// Its value labels and σ, as the plot's axis labels are written.
    pub(super) text: Color32,
}

/// A line the plot draws at a value, carried across the bars as it is drawn
/// there: the mean, a reference line, an alarm limit, a cursor's reading.
pub(super) struct Line {
    pub(super) value: f64,
    pub(super) stroke: egui::Stroke,
    pub(super) style: egui_plot::LineStyle,
}

/// The span across the bars at `at` along the value axis: a row of the side
/// strip, a column of the band.
fn across(dock: Dock, bars: Rect, at: f32) -> [egui::Pos2; 2] {
    match dock {
        Dock::Side => [bars.left(), bars.right()].map(|x| egui::pos2(x, at)),
        Dock::Below => [bars.top(), bars.bottom()].map(|y| egui::pos2(at, y)),
    }
}

/// The point `length` from the bars' base at `at` along the value axis.
fn point(dock: Dock, bars: Rect, at: f32, length: f32) -> egui::Pos2 {
    match dock {
        Dock::Side => egui::pos2(bars.left() + length, at),
        Dock::Below => egui::pos2(at, bars.bottom() - length),
    }
}

/// Hover: the bin under the pointer, lit, with its readings and shares.
fn hover_bin(
    ui: &Ui,
    rect: Rect,
    hist: &Histogram,
    unit: &str,
    bin_at: impl Fn(egui::Pos2) -> Option<usize>,
) -> Option<usize> {
    let response = ui.interact(rect, ui.scope_id().with("histogram"), Sense::hover());
    let bin = response.hover_pos().and_then(bin_at)?;
    response.on_hover_text_at_pointer(hist.hover_text(bin, unit));
    Some(bin)
}

/// The bars in `bars`, growing from its left edge (side) or bottom (below),
/// each spanning the positions `along` gives its edges; faint gridlines at
/// round shares about `spacing` points apart; the interval's outline over
/// the bars; and `lines` across them. Returns the gridlines, as distances
/// from the bars' base and their labels, for the caller's scale.
#[allow(clippy::too_many_arguments)]
fn paint_bins(
    ui: &Ui,
    dock: Dock,
    bars: Rect,
    hist: &Histogram,
    along: impl Fn(f64) -> f32,
    hovered: Option<usize>,
    lines: &[Line],
    spacing: f32,
    colors: &HistogramColors,
) -> Vec<(f32, String)> {
    let painter = ui
        .painter_at(bars.expand(2.0))
        .with_clip_rect(bars.expand(2.0));
    painter.rect_filled(bars, 0.0, colors.background);
    let room = match dock {
        Dock::Side => bars.width(),
        Dock::Below => bars.height(),
    } - 2.0;
    let peak_share = hist.peak_share();
    let peak = peak_share.max(f64::MIN_POSITIVE);
    let length = |c: &Counts, count: u32| (room as f64 * c.share(count) / peak) as f32;
    // No scale over bars that hold nothing: only over-range readings, say.
    let grid: Vec<(f32, String)> = share_ticks(peak_share, room, spacing)
        .into_iter()
        .map(|(share, label)| ((f64::from(room) * share / peak) as f32, label))
        .collect();
    for (at, _) in &grid {
        let [a, b] = match dock {
            Dock::Side => [bars.top(), bars.bottom()].map(|y| egui::pos2(bars.left() + at, y)),
            Dock::Below => [bars.left(), bars.right()].map(|x| egui::pos2(x, bars.bottom() - at)),
        };
        painter.line_segment(
            [a, b],
            egui::Stroke::new(1.0, colors.text.gamma_multiply(0.3)),
        );
    }
    let pixel = 1.0 / ui.ctx().pixels_per_point();
    let snap = |x: f32| (x / pixel).round() * pixel;
    let span = |i: usize| {
        let (low, high) = hist.edges(i);
        let (a, b) = (along(low), along(high));
        (a.min(b), a.max(b))
    };
    let mut hovered_bar = None;
    for (i, &count) in hist.view.counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        // Edges on whole pixels, and one pixel of air between bars wide
        // enough to spare it: on fractional edges the gap smears over one
        // pixel here and two there, and the bars look unevenly spaced.
        let (a, b) = span(i);
        let (a, b) = (snap(a), snap(b));
        let air = if b - a >= 3.0 * pixel { pixel } else { 0.0 };
        let (a, b) = (a, (b - air).max(a + pixel));
        let len = length(&hist.view, count).max(1.0);
        let bar = Rect::from_two_pos(point(dock, bars, a, 0.0), point(dock, bars, b, len))
            .intersect(bars);
        painter.rect_filled(bar, 0.0, colors.bar);
        if hovered == Some(i) {
            hovered_bar = Some(bar);
        }
    }
    for line in lines {
        let [a, b] = across(dock, bars, along(line.value));
        painter.extend(super::render::styled_line(a, b, line.stroke, line.style));
    }
    if let Some(interval) = &hist.interval {
        // A stepped outline along the bins' ends, one line per run of
        // filled bins from the base and back, so it reads as a second
        // histogram over the bars rather than as more of them.
        let mut runs: Vec<Vec<egui::Pos2>> = vec![Vec::new()];
        for (i, &count) in interval.counts.iter().enumerate() {
            let (low, high) = hist.edges(i);
            let run = runs.last_mut().expect("never empty");
            if count == 0 {
                if !run.is_empty() {
                    runs.push(Vec::new());
                }
                continue;
            }
            let len = length(interval, count).max(1.0);
            if run.is_empty() {
                run.push(point(dock, bars, along(low), 0.0));
            } else {
                // Close the previous bin on this edge; the next step starts here.
                run.pop();
            }
            run.push(point(dock, bars, along(low), len));
            run.push(point(dock, bars, along(high), len));
            run.push(point(dock, bars, along(high), 0.0));
        }
        // Haloed in the background colour, to stand off the bars it crosses.
        for stroke in [
            egui::Stroke::new(3.5, colors.background),
            egui::Stroke::new(1.5, colors.cursor),
        ] {
            for run in runs.iter().filter(|r| !r.is_empty()) {
                painter.add(egui::Shape::line(run.clone(), stroke));
            }
        }
    }
    // Last, over the neighbouring bars, the lines and the outline, in the
    // strongest text colour, haloed in the background's: a pale theme's bars
    // can be as light as the outline, and the halo keeps it off them.
    if let Some(bar) = hovered_bar {
        for stroke in [
            egui::Stroke::new(4.0, colors.background),
            egui::Stroke::new(2.0, ui.visuals().strong_text_color()),
        ] {
            painter.rect_stroke(bar.expand(2.0), 0.0, stroke, egui::StrokeKind::Middle);
        }
    }
    grid
}

/// Round shares about `spacing` points apart along `room` points of bar, a
/// full bar standing for `peak`, from the first step up to `peak`: each
/// share with its label.
fn share_ticks(peak: f64, room: f32, spacing: f32) -> Vec<(f64, String)> {
    if !peak.is_finite() || peak <= 0.0 || room <= 0.0 {
        return Vec::new();
    }
    let step = super::axes::nice_step(peak * 100.0 * f64::from(spacing / room)) / 100.0;
    let decimals = (-(step * 100.0).log10()).ceil().max(0.0) as usize;
    (1..)
        .map(|k| f64::from(k) * step)
        .take_while(|&share| share <= peak * (1.0 + 1e-9))
        .map(|share| (share, format!("{:.decimals$}%", share * 100.0)))
        .collect()
}

/// A share of readings as the pane writes it: a decimal under 10 %, and
/// `<0.1%` for one too small to show at all.
fn percent(share: f64) -> String {
    let pct = share * 100.0;
    if pct < 0.05 {
        "<0.1%".to_string()
    } else if pct < 10.0 {
        format!("{pct:.1}%")
    } else {
        format!("{pct:.0}%")
    }
}

/// The readings beyond one end of the Y range, as shares at the end of the
/// bars they would lie past: `arrow` and the view's share of its readings,
/// then `A–B` and the interval's in the cursor colour, each boxed so it
/// reads over a bar. At the top end, the over-range readings after them,
/// `OL` with their share of every reading. Stacked from `corner` away from
/// it, aligned by `align`.
#[allow(clippy::too_many_arguments)]
fn paint_beyond(
    ui: &Ui,
    painter: &egui::Painter,
    hist: &Histogram,
    above: bool,
    arrow: char,
    corner: egui::Pos2,
    align: egui::Align2,
    colors: &HistogramColors,
) {
    let (view, interval) = hist.beyond(above);
    let (view_ol, interval_ol) = if above {
        hist.overload_shares()
    } else {
        (None, None)
    };
    let text = ui.visuals().text_color();
    let rows = [
        view.map(|s| (format!("{arrow}{}", percent(s)), text)),
        interval.map(|s| (format!("A\u{2013}B {arrow}{}", percent(s)), colors.cursor)),
        view_ol.map(|s| (format!("OL {}", percent(s)), text)),
        interval_ol.map(|s| (format!("A\u{2013}B OL {}", percent(s)), colors.cursor)),
    ];
    let font = egui::TextStyle::Small.resolve(ui.style());
    let step = ui.text_style_height(&egui::TextStyle::Small) + 2.0;
    let step = if align.y() == egui::Align::Max {
        -step
    } else {
        step
    };
    let mut at = corner;
    for (text, color) in rows.into_iter().flatten() {
        let galley = painter.layout_no_wrap(text, font.clone(), color);
        let rect = align.anchor_size(at, galley.size());
        painter.rect_filled(rect.expand(1.0), 2.0, colors.background);
        painter.galley(rect.min, galley, color);
        at.y += step;
    }
}

/// The strip right of `plot`, `area` tall, its bars at the heights
/// `transform` gives their values. The σ caption goes under the plot, in
/// the time axis's row.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_side(
    ui: &Ui,
    area: Rect,
    plot: Rect,
    transform: &PlotTransform,
    hist: &Histogram,
    lines: &[Line],
    unit: &str,
    colors: &HistogramColors,
) {
    let strip = Rect::from_x_y_ranges(area.left() + DIVIDER..=area.right(), plot.y_range());
    let x = transform.bounds().min()[0];
    let y_of = |v: f64| transform.position_from_point(&PlotPoint::new(x, v)).y;
    let hovered = hover_bin(ui, strip, hist, unit, |pos| {
        hist.bin_of(transform.value_from_position(pos).y)
    });
    let grid = paint_bins(
        ui,
        Dock::Side,
        strip,
        hist,
        y_of,
        hovered,
        lines,
        28.0,
        colors,
    );
    let painter = ui.painter_at(area);
    let inset = egui::vec2(-3.0, 3.0);
    let top = strip.right_top() + inset;
    paint_beyond(
        ui,
        &painter,
        hist,
        true,
        '\u{2191}',
        top,
        egui::Align2::RIGHT_TOP,
        colors,
    );
    let bottom = strip.right_bottom() - inset;
    let bottom = egui::pos2(top.x, bottom.y);
    paint_beyond(
        ui,
        &painter,
        hist,
        false,
        '\u{2193}',
        bottom,
        egui::Align2::RIGHT_BOTTOM,
        colors,
    );
    // The scale under the strip, in the time axis's row: each label under
    // its gridline, shifted in from the strip's right end it would overhang.
    let font = egui::TextStyle::Small.resolve(ui.style());
    let painter = ui.painter();
    for (at, label) in grid {
        let galley = painter.layout_no_wrap(label, font.clone(), colors.text);
        let x = strip.left() + at;
        let rect =
            egui::Align2::CENTER_TOP.anchor_size(egui::pos2(x, plot.bottom() + 4.0), galley.size());
        let shift = (strip.right() - rect.right()).min(0.0);
        painter.galley(rect.min + egui::vec2(shift, 0.0), galley, colors.text);
    }
}

/// The band under the minimap, values across `x_range` (the plot's own
/// width, so it lines up with the plot above) between `lo` and `hi`.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_below(
    ui: &Ui,
    area: Rect,
    x_range: egui::Rangef,
    (lo, hi): (f64, f64),
    hist: &Histogram,
    lines: &[Line],
    unit: &str,
    colors: &HistogramColors,
) {
    let font = egui::TextStyle::Small.resolve(ui.style());
    let row = ui.text_style_height(&egui::TextStyle::Small);
    let bars = Rect::from_x_y_ranges(x_range, area.top()..=area.bottom() - row - 2.0);
    let x_of = |v: f64| bars.left() + ((v - lo) / (hi - lo)) as f32 * bars.width();
    let hovered = hover_bin(ui, bars, hist, unit, |pos| {
        let share = f64::from((pos.x - bars.left()) / bars.width());
        hist.bin_of(lo + share * (hi - lo))
    });
    let grid = paint_bins(
        ui,
        Dock::Below,
        bars,
        hist,
        x_of,
        hovered,
        lines,
        24.0,
        colors,
    );
    let painter = ui.painter_at(area);
    // Values run across the band, so readings below the axis lie past its
    // left end and those above past its right.
    let left = bars.left_top() + egui::vec2(3.0, 3.0);
    paint_beyond(
        ui,
        &painter,
        hist,
        false,
        '\u{2190}',
        left,
        egui::Align2::LEFT_TOP,
        colors,
    );
    let right = bars.right_top() + egui::vec2(-3.0, 3.0);
    paint_beyond(
        ui,
        &painter,
        hist,
        true,
        '\u{2192}',
        right,
        egui::Align2::RIGHT_TOP,
        colors,
    );
    // The scale up the left side, in the room the plot keeps for its axis.
    for (at, label) in grid {
        painter.text(
            egui::pos2(bars.left() - 4.0, bars.bottom() - at),
            egui::Align2::RIGHT_CENTER,
            label,
            font.clone(),
            colors.text,
        );
    }
    // An axis of its own along the bottom, at round values with their unit
    // as the plot's axis writes them. The count and σ are the statistics
    // panel's.
    let labels_y = bars.bottom() + 2.0;
    for (value, decimals) in band_ticks(lo, hi, bars.width()) {
        let x = x_of(value);
        let text = format!("{} {unit}", fixed(value, decimals));
        let galley = painter.layout_no_wrap(text, font.clone(), colors.text);
        // Under its tick, shifted in from an end it would overhang: the ticks
        // are far enough apart that a shifted label still clears the next.
        let rect = egui::Align2::CENTER_TOP.anchor_size(egui::pos2(x, labels_y), galley.size());
        let shift = (bars.left() - rect.left()).max(0.0) + (bars.right() - rect.right()).min(0.0);
        painter.line_segment(
            [
                egui::pos2(x, bars.bottom()),
                egui::pos2(x, bars.bottom() + 2.0),
            ],
            egui::Stroke::new(1.0, colors.text),
        );
        painter.galley(rect.min + egui::vec2(shift, 0.0), galley, colors.text);
    }
}

/// Points between the band's value labels, at least.
const TICK_SPACING: f32 = 80.0;

/// The band's value ticks between `lo` and `hi` along `width` points: whole
/// multiples of a round step about [`TICK_SPACING`] apart, each with the
/// decimals the step needs.
fn band_ticks(lo: f64, hi: f64, width: f32) -> Vec<(f64, usize)> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo || width <= 0.0 {
        return Vec::new();
    }
    let step = super::axes::nice_step((hi - lo) * f64::from(TICK_SPACING / width));
    let decimals = (-step.log10()).ceil().max(0.0) as usize;
    let first = (lo / step).ceil() as i64;
    let last = (hi / step).floor() as i64;
    (first..=last)
        .map(|k| (k as f64 * step, decimals))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i(values: &[f64]) -> impl ExactSizeIterator<Item = f64> + Clone + '_ {
        values.iter().copied()
    }

    /// `values` read `times` over each, enough readings that the bins follow
    /// their steps rather than how few they are.
    fn times(values: &[f64], times: usize) -> Vec<f64> {
        values
            .iter()
            .flat_map(|&v| std::iter::repeat_n(v, times))
            .collect()
    }

    #[test]
    fn the_step_is_the_fewest_decimals_writing_every_reading() {
        assert_eq!(quantum_of(i(&[5.0021, 5.003, 5.0])), Some((1e-4, 4)));
        assert_eq!(quantum_of(i(&[1.0, 2.0, 3.0])), Some((1.0, 0)));
        assert_eq!(quantum_of(i(&[0.1 + 0.2])), Some((0.1, 1)));
        assert_eq!(quantum_of(i(&[1.0 / 3.0])), None);
        assert_eq!(quantum_of(i(&[])), None);
    }

    /// Binning walks the visible slice each time its readings change, as
    /// the visible statistics do every frame. It costs a few such passes (about 2.5 ms against
    /// 0.6 ms on a full 500K-sample window, measured 2026-10-23); this
    /// catches it growing out of proportion.
    #[test]
    #[ignore = "timing-sensitive; run with --release"]
    fn binning_costs_about_a_statistics_pass() {
        use std::hint::black_box;
        use std::time::Instant;
        let values: Vec<f64> = (0..500_000u32)
            .map(|k| 5.0 + f64::from(k.wrapping_mul(7919) % 1000) / 1e4)
            .collect();
        let start = Instant::now();
        for _ in 0..20 {
            let mut s = RunningStats::new();
            for &v in &values {
                s.push(v);
            }
            black_box(s.avg());
        }
        let stats = start.elapsed();
        let start = Instant::now();
        for _ in 0..20 {
            black_box(Histogram::new(
                i(&values),
                (5.0, 5.1),
                target_bins(600.0),
                (4, 4),
            ));
        }
        let binning = start.elapsed();
        println!(
            "500K per frame: statistics {:?}, binning {:?}",
            stats / 20,
            binning / 20
        );
        assert!(binning < stats * 8, "binning grew out of proportion");
    }

    #[test]
    fn float32_readings_keep_their_decimal_step() {
        let readings = [f64::from(239.22f32), f64::from(0.051f32)];
        assert_eq!(quantum_of(i(&readings)), Some((0.001, 3)));
    }

    /// Noise in every digit has no step, however many decimals are tried.
    #[test]
    fn computed_values_have_no_step() {
        let values: Vec<f64> = (1..50).map(|k| 11.0 + (f64::from(k) * 0.7).sin()).collect();
        assert_eq!(quantum_of(i(&values)), None);
    }

    /// A steady round reading drops its step, and is written to the
    /// graph's decimals rather than as a whole number.
    #[test]
    fn a_steady_round_reading_keeps_its_decimals() {
        let h = Histogram::new(
            i(&[5.0; 10]),
            (5.0 - 1e-7, 5.0 + 1e-7),
            target_bins(400.0),
            (4, 4),
        )
        .unwrap();
        assert_eq!(h.decimals, 4);
    }

    /// A steady reading: the axis is padded a hair around it, far narrower
    /// than a step, and the reading is one thin bar rather than one bin
    /// filling the strip.
    #[test]
    fn a_steady_reading_is_one_thin_bin() {
        let h = Histogram::new(
            i(&[5.002; 10]),
            (5.002 - 1e-7, 5.002 + 1e-7),
            target_bins(400.0),
            (4, 4),
        )
        .unwrap();
        assert_eq!(h.quantum, 0.0);
        assert!(h.width <= 2e-7 / 20.0, "{}", h.width);
        assert_eq!(h.view.counts.iter().filter(|&&c| c > 0).count(), 1);
        assert_eq!(h.decimals, 4, "the graph's decimals at least");
    }

    /// Readings a millivolt apart, binned a millivolt a bin: each reading
    /// lands in its own bin, none falls on an edge and none is empty
    /// between two that are full.
    #[test]
    fn a_bin_a_step_wide_holds_one_reading_value() {
        let values = times(&[5.001, 5.002, 5.002, 5.003, 5.003, 5.003], 10);
        let h = Histogram::new(i(&values), (5.0, 5.004), target_bins(1000.0), (4, 4)).unwrap();
        assert_eq!(h.width, 1e-3);
        assert_eq!(h.view.counts, vec![0, 10, 20, 30, 0]);
        assert_eq!(h.range_label(2, "V"), "5.002 V");
    }

    /// Too little room for a bin per step: bins a round number of steps
    /// wide, so no bin catches a step more than its neighbours.
    #[test]
    fn wider_bins_hold_whole_steps() {
        let values: Vec<f64> = (0..=100).map(|i| f64::from(i) / 1000.0).collect();
        let h = Histogram::new(i(&values), (0.0, 0.1), target_bins(40.0), (4, 4)).unwrap();
        // 10 bins of 4 points: 0.01 wide, ten steps each.
        assert!((h.width - 0.01).abs() < 1e-15);
        assert!(
            h.view.counts[..10].iter().all(|&c| c == 10),
            "{:?}",
            h.view.counts
        );
        assert_eq!(h.range_label(0, "V"), "0.000 to 0.009 V");
    }

    #[test]
    fn readings_beyond_the_axis_are_counted_apart() {
        let h = Histogram::new(
            i(&[1.0, 2.0, 3.0, 9.0, -4.0]),
            (0.0, 5.0),
            target_bins(100.0),
            (4, 4),
        )
        .unwrap();
        assert_eq!((h.view.above, h.view.below), (1, 1));
        assert_eq!(h.view.counts.iter().sum::<u32>(), 3);
        assert_eq!(h.view.stats.count, 5);
        assert_eq!(
            (h.beyond(true), h.beyond(false)),
            ((Some(0.2), None), (Some(0.2), None))
        );
    }

    /// The strip goes beside a plot that leaves it its narrowest width, up
    /// to half the graph; else the band goes under the minimap, up to half
    /// the room, and at its least in a room too short for that.
    #[test]
    fn the_pane_goes_beside_a_roomy_plot_and_below_otherwise() {
        let p = place(1000.0, 300.0, 500.0, (90.0, 90.0));
        assert_eq!((p.dock, p.size, p.max), (Dock::Side, 90.0, 300.0));
        let narrowed = place(500.0, MIN_STRIP_WIDTH, 500.0, (200.0, 90.0));
        assert_eq!(
            (narrowed.dock, narrowed.size),
            (Dock::Side, MIN_STRIP_WIDTH)
        );
        assert_eq!(place(1000.0, 900.0, 500.0, (900.0, 90.0)).max, 500.0);
        let p = place(400.0, MIN_STRIP_WIDTH - 1.0, 300.0, (90.0, 400.0));
        assert_eq!((p.dock, p.size, p.max), (Dock::Below, 150.0, 150.0));
        let short = place(400.0, 0.0, 40.0, (90.0, 90.0));
        assert_eq!((short.dock, short.size), (Dock::Below, MIN_BAND_HEIGHT));
    }

    /// A spread finer than the readings' last digit is still spoken: σ is
    /// written like Statistics writes it, not like a reading.
    #[test]
    fn sigma_is_written_finer_than_the_readings() {
        let mut values = vec![5.002; 19];
        values.push(5.003);
        let h = Histogram::new(i(&values), (5.0, 5.005), target_bins(400.0), (3, 4)).unwrap();
        assert!(h.spoken("V").contains("standard deviation 0.0002 V"));
    }

    #[test]
    fn an_edge_just_under_zero_reads_zero() {
        assert_eq!(fixed(-0.3, 0), "0");
        assert_eq!(fixed(-0.04, 1), "0.0");
        assert_eq!(fixed(-0.3, 1), "-0.3");
    }

    /// Scaled readings, 0.1 °C steps as °F, come 0.18 apart at an offset:
    /// their bins are whole steps with edges halfway between readings, so
    /// none falls empty between two that are full, where 0.01-step bins
    /// would catch one reading here and two there.
    #[test]
    fn scaled_readings_bin_on_their_own_step() {
        let celsius = (0..2000).map(|k| 25.0 + 0.1 * f64::from(k % 41 - 20));
        let values: Vec<f64> = celsius
            .map(|c| ((32.0 + 1.8 * c) * 100.0).round() / 100.0)
            .collect();
        let h = Histogram::new(i(&values), (72.9, 80.9), target_bins(600.0), (2, 2)).unwrap();
        assert!((h.quantum - 0.18).abs() < 1e-9, "{}", h.quantum);
        assert!((h.width / h.quantum - (h.width / h.quantum).round()).abs() < 1e-9);
        let counts = &h.view.counts;
        let first = counts.iter().position(|&c| c > 0).unwrap();
        let last = counts.iter().rposition(|&c| c > 0).unwrap();
        assert!(counts[first..=last].iter().all(|&c| c > 0), "{counts:?}");
        assert!(
            counts[first..=last]
                .windows(2)
                .all(|w| w[0].abs_diff(w[1]) <= 1),
            "{counts:?}"
        );
    }

    /// A reading flicking between two values, two steps apart, is no
    /// evidence of a step that coarse: it keeps its decimals' step.
    #[test]
    fn two_values_keep_their_decimal_step() {
        let values = times(&[5.002, 5.004], 50);
        let h = Histogram::new(i(&values), (5.0, 5.006), target_bins(600.0), (4, 4)).unwrap();
        assert_eq!(h.quantum, 0.001);
    }

    /// A few readings of a sine over a tall strip get as many bins as they
    /// fill: its shape, peaks at both extremes, rather than a comb of bins
    /// the pixels alone would cut, most of them empty. Its period is no
    /// whole number of readings, or it would only ever give a few values.
    #[test]
    fn few_readings_get_few_bins() {
        let values: Vec<f64> = (0..155)
            .map(|k| 120.0 + 2.0 * (f64::from(k) * 0.1 / 5.185 * std::f64::consts::TAU).sin())
            .collect();
        let h = Histogram::new(i(&values), (117.75, 122.25), target_bins(600.0), (4, 4)).unwrap();
        let counts = &h.view.counts;
        let first = counts.iter().position(|&c| c > 0).unwrap();
        let last = counts.iter().rposition(|&c| c > 0).unwrap();
        assert!(counts[first..=last].iter().all(|&c| c > 0), "{counts:?}");
        assert!(last - first < 32, "{counts:?}");
        let middle = counts[(first + last) / 2];
        assert!(
            counts[first] > middle && counts[last] > middle,
            "{counts:?}"
        );
    }

    /// The interval counts on the view's bins, its readings beyond the axis
    /// apart, and each bar is its share of its own readings: a period with
    /// all its readings in one bin draws it at full length, against the
    /// view's half.
    #[test]
    fn the_interval_counts_on_the_same_bins() {
        let values = times(&[0.0, 2.0, 2.0, 3.0], 10);
        let mut h = Histogram::new(i(&values), (0.0, 4.0), target_bins(400.0), (0, 1)).unwrap();
        assert_eq!(h.peak_share(), 0.5);
        assert_eq!(h.beyond(true), (None, None));
        h.count_interval([2.0, 2.0, 7.0, 9.0].into_iter(), 0);
        let interval = h.interval.as_ref().unwrap();
        assert_eq!(interval.counts, vec![0, 0, 2, 0, 0]);
        assert_eq!(interval.above, 2);
        assert_eq!(h.peak_share(), 0.5, "2 of the interval's 4 readings");
        assert_eq!(h.beyond(true), (None, Some(0.5)));
        assert_eq!(
            h.hover_text(2, "V"),
            "2 V: 20 readings (50.0%)\nA\u{2013}B: 2 readings (50.0%)"
        );
        assert!(
            h.spoken("V")
                .ends_with("; 4 samples between the cursors, standard deviation 3.6 V.")
        );
    }

    /// Over-range readings have no value to bin: they show as their share
    /// of every reading, OL included, the view's and the interval's apart,
    /// and the screen reader hears how many.
    #[test]
    fn overloads_show_as_their_share_of_every_reading() {
        let values = times(&[1.0, 2.0], 20);
        let mut h = Histogram::new(i(&values), (0.0, 3.0), target_bins(300.0), (0, 1))
            .unwrap()
            .with_overloads(10);
        h.count_interval([1.0, 2.0, 2.0].into_iter(), 1);
        assert_eq!(h.overload_shares(), (Some(0.2), Some(0.25)));
        assert!(
            h.spoken("V").contains(", 10 over range"),
            "{}",
            h.spoken("V")
        );
    }

    /// The share scale: round percentages along the bar, up to what a full
    /// bar stands for, with the decimals their step needs.
    #[test]
    fn share_ticks_fall_on_round_percentages() {
        let labels = |peak, room| -> Vec<String> {
            share_ticks(peak, room, 36.0)
                .into_iter()
                .map(|(_, l)| l)
                .collect()
        };
        assert_eq!(labels(0.14, 90.0), ["10%"]);
        assert_eq!(labels(0.14, 200.0), ["5%", "10%"]);
        assert_eq!(labels(0.012, 90.0), ["0.5%", "1.0%"]);
        assert!(labels(0.0, 90.0).is_empty());
        assert!(labels(f64::MIN_POSITIVE, 90.0).len() < 10);
    }

    #[test]
    fn shares_read_at_a_glance() {
        assert_eq!(percent(0.5), "50%");
        assert_eq!(percent(0.0123), "1.2%");
        assert_eq!(percent(1.0 / 500_000.0), "<0.1%");
    }

    /// The band's axis: round values about [`TICK_SPACING`] apart, with the
    /// decimals their step needs, never one past either end.
    #[test]
    fn band_ticks_fall_on_round_values() {
        let ticks = band_ticks(237.9, 240.6, 500.0);
        assert_eq!(
            ticks,
            [
                (238.0, 1),
                (238.5, 1),
                (239.0, 1),
                (239.5, 1),
                (240.0, 1),
                (240.5, 1)
            ]
        );
        assert_eq!(band_ticks(0.0, 4.0, 320.0)[..2], [(0.0, 0), (1.0, 0)]);
        assert!(band_ticks(1.0, 1.0, 300.0).is_empty());
    }

    #[test]
    fn levels_get_a_bin_each() {
        let values = times(&[0.0, 2.0, 2.0, 3.0], 10);
        let h = Histogram::new(i(&values), (0.0, 4.0), target_bins(400.0), (0, 1)).unwrap();
        assert_eq!(h.width, 1.0);
        assert_eq!(h.view.counts, vec![10, 0, 20, 10, 0]);
    }

    #[test]
    fn no_range_no_histogram() {
        assert!(Histogram::new(i(&[1.0]), (1.0, 1.0), target_bins(100.0), (4, 4)).is_none());
        assert!(
            Histogram::new(i(&[1.0]), (0.0, f64::INFINITY), target_bins(100.0), (4, 4)).is_none()
        );
    }

    #[test]
    fn an_empty_window_bins_nothing() {
        let h = Histogram::new(i(&[]), (0.0, 1.0), target_bins(100.0), (4, 4)).unwrap();
        assert_eq!(h.view.tallest(), 0);
        assert_eq!(h.spoken("V"), " Histogram of 0 visible samples.");
    }

    #[test]
    fn the_spoken_summary_names_the_fullest_bin() {
        let values = times(&[1.0, 2.0, 2.0], 10);
        let h = Histogram::new(i(&values), (0.0, 3.0), target_bins(300.0), (0, 1)).unwrap();
        assert_eq!(
            h.spoken("V"),
            " Histogram of 30 visible samples, standard deviation 0.5 V, most at 2 V (20)."
        );
    }
}
