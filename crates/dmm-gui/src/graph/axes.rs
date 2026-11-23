//! Y axes for sub-values in other units, aligned on the plotted series' grid.
//!
//! egui_plot draws every Y axis from one transform and one grid spacer, so a
//! right axis can only label the tick positions of the plotted unit. Each
//! other unit is therefore mapped onto the plot as `y = offset + scale·v`,
//! chosen so those shared ticks land on round values of its own, and one set
//! of gridlines serves them all. A right axis steps 1, 2, 2.5 or 5 × 10ⁿ per
//! gridline; at 2.5 it labels every other line only, so each label it writes
//! is still a whole 1, 2 or 5 × 10ⁿ, and a trace that 2 would squeeze and 5
//! would flatten gets the height between.

use dmm_lib::transform::{PrefixRatio, prefix_for};

/// Where one unit's values sit on the plot: `y = offset + scale·v`, with a
/// gridline every `step` of the unit and a label every `label_step`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AxisMap {
    offset: f64,
    scale: f64,
    pub step: f64,
    pub label_step: f64,
}

impl AxisMap {
    /// The value's height on the plot.
    pub(super) fn height_of(self, v: f64) -> f64 {
        self.offset + self.scale * v
    }

    /// The value at a height on the plot.
    pub(super) fn value_at(self, y: f64) -> f64 {
        (y - self.offset) / self.scale
    }

    /// The decimals the axis's labels need.
    pub(super) fn decimals(self) -> usize {
        step_decimals(self.label_step)
    }

    /// Whether the gridline at `v` carries a label: every one, or every
    /// other one on a 2.5 step.
    pub(super) fn labels(self, v: f64) -> bool {
        let k = v / self.label_step;
        (k - k.round()).abs() < 1e-6
    }
}

/// The smallest 1, 2 or 5 × 10ⁿ at least `x`.
pub(super) fn nice_step(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let base = 10f64.powf(x.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * base)
        .find(|&s| s >= x * (1.0 - 1e-9))
        .unwrap_or(10.0 * base)
}

/// The smallest 1, 2, 2.5 or 5 × 10ⁿ at least `x`: a right axis's step per
/// gridline.
fn secondary_step(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let base = 10f64.powf(x.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .map(|m| m * base)
        .find(|&s| s >= x * (1.0 - 1e-9))
        .unwrap_or(10.0 * base)
}

/// The span of the plotted unit an axis writes as it is: from 0.001 to
/// four integer digits. Past either end, a trace the meter auto-ranged out of
/// would read 12000000 Ω or 0.0000 MΩ; inside it, the axis keeps the unit the
/// meter shows, and a reading wandering across a decade never flips it.
const PLAIN_SPAN: std::ops::Range<f64> = 0.001..10_000.0;

/// The prefix of `unit` an axis spanning `lo..=hi` writes its values in,
/// and the ratio taking a value in `unit` there, when `unit` writes that
/// span badly (see [`PLAIN_SPAN`]): the one its larger end reads one to
/// three digits in, 12 MΩ rather than 12000000 Ω. `None` to write them as
/// they are, as for a unit no prefix goes on or an empty span.
pub(super) fn axis_prefix(unit: &str, lo: f64, hi: f64) -> Option<(String, PrefixRatio)> {
    let largest = lo.abs().max(hi.abs());
    if !(largest.is_finite() && largest > 0.0) || PLAIN_SPAN.contains(&largest) {
        return None;
    }
    let (prefixed, ratio) = prefix_for(unit, largest.log10().floor() as i32)?;
    (prefixed != unit).then_some((prefixed, ratio))
}

/// The decimals that write in the axis's `prefix` what `decimals` wrote in
/// the plotted unit: 0.15 Ω on a MΩ axis needs six more, 1500 Ω on a kΩ
/// one three fewer.
pub(super) fn prefixed_decimals(prefix: Option<&(String, PrefixRatio)>, decimals: usize) -> usize {
    let Some((_, ratio)) = prefix else {
        return decimals;
    };
    // A power of ten, so the rounded log is exact.
    let power = ratio.apply(1.0).log10().round() as i64;
    (decimals as i64 - power).clamp(0, 12) as usize
}

/// Significant digits a number re-expressed in another prefix keeps: all a
/// typed one has, none of the float error of the step (`3.29 × 1000` is
/// `3290.0000000000005`).
const RESCALED_DIGITS: i32 = 12;

/// `v` to at most [`RESCALED_DIGITS`] significant digits with no trailing
/// zeros: a number re-expressed in another prefix without the float error of
/// the step.
pub(super) fn short_figure(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { v.to_string() };
    }
    let decimals = (RESCALED_DIGITS - 1 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// A value set at a height — a reference line, an alarm limit — as it was
/// typed: in the prefix that fits it, whatever the meter has stepped to
/// since (1 Ω, not 0.0000 MΩ), as short as it reads.
pub(super) fn set_value_text(v: f64, unit: &str) -> String {
    let fitting = (v.is_finite() && v != 0.0)
        .then(|| prefix_for(unit, v.abs().log10().floor() as i32))
        .flatten();
    let (v, unit) = match &fitting {
        Some((prefixed, ratio)) => (ratio.apply(v), prefixed.as_str()),
        None => (v, unit),
    };
    format!("{} {unit}", short_figure(v)).trim_end().to_string()
}

/// `value` in `unit`, written in the axis's `prefix` where it has one.
pub(super) fn in_prefix<'a>(
    prefix: Option<&'a (String, PrefixRatio)>,
    unit: &'a str,
    value: f64,
) -> (f64, &'a str) {
    match prefix {
        Some((prefixed, ratio)) => (ratio.apply(value), prefixed.as_str()),
        None => (value, unit),
    }
}

/// The decimals that write every multiple of `step` exactly: 0.05 needs
/// two, 0.2 one, 5 none.
pub(super) fn step_decimals(step: f64) -> usize {
    if !(step.is_finite() && step > 0.0) {
        return 0;
    }
    (-step.log10() - 1e-9).ceil().clamp(0.0, 9.0) as usize
}

/// Points of air a gridline's label keeps over its own height, for every
/// line of a grid of round steps to carry one: egui_plot leaves out a label
/// taller than its lines are apart. The height is the style's, measured
/// where the grid is chosen; in the app's own it is 15 points, a line every
/// 17 at the closest.
pub(super) const LABEL_AIR: f32 = 2.0;

/// The spacing of egui_plot's own gridlines its labels fade in over: none
/// up to the first, at an opacity of the root of the way from it to the
/// second, where they are at full strength. egui_plot's default for a value
/// axis, which the plotted unit's axis is given all the same, so the labels
/// are drawn by the range their readability is reckoned from.
pub(super) const DECADE_LABEL_FADE: (f32, f32) = (20.0, 30.0);

/// Points between egui_plot's own gridlines from which their labels count
/// as readable: two fifths of the way through [`DECADE_LABEL_FADE`], there
/// being nothing to read just past its start. At 24 the opacity is 0.63 and
/// the default themes' labels stand 4.1:1 (dark) and 3.1:1 (light) off the
/// panel, the least whole spacing that clears 3:1 on both.
pub(super) const READABLE_LABEL_SPACING: f64 =
    DECADE_LABEL_FADE.0 as f64 + (DECADE_LABEL_FADE.1 - DECADE_LABEL_FADE.0) as f64 * 2.0 / 5.0;

/// Whether egui_plot's own grid, in decade steps no finer than `finest`,
/// has two lines at least to label readably on a plot `span` high drawn
/// over `height` points, wherever the span lies. Its steps are ten times
/// apart, so a short plot can fall between two: 2 V to 8 V on fifty points
/// has its 1 V lines too close to label and no 10 V line in view.
pub(super) fn decades_label_two(span: f64, height: f32, finest: f64) -> bool {
    let readable = READABLE_LABEL_SPACING * span / f64::from(height);
    let decade = 10f64.powf(readable.log10().ceil()).max(finest);
    span >= 2.0 * decade
}

/// How far inside the spans its decades serve a plot on a grid of round
/// steps has to be before it goes back to them: a fifth of its span either
/// way. A live trace's range moves with every extreme that enters or leaves
/// the view, and one wandering across the line between the two grids
/// changed its labels at each crossing.
const DECADES_MARGIN: f64 = 0.2;

/// Whether a plot takes egui_plot's own decade grid ([`decades_label_two`])
/// rather than one of round steps, given what it was drawn on last
/// (`on_round_steps`; false for a plot not drawn yet). A plot on its decades
/// leaves them the moment they stop serving it, so its labels are always
/// readable; one on round steps, which serve any plot, stays until the
/// decades would serve a span [`DECADES_MARGIN`] smaller and larger too. So
/// a window dragged taller takes its decades a fifth later than one dragged
/// shorter gives them up, and a range within that of the line keeps its
/// grid.
pub(super) fn takes_decades(span: f64, height: f32, finest: f64, on_round_steps: bool) -> bool {
    let serve = |span: f64| decades_label_two(span, height, finest);
    serve(span)
        && (!on_round_steps
            || (serve(span * (1.0 - DECADES_MARGIN)) && serve(span * (1.0 + DECADES_MARGIN))))
}

/// The plotted unit's grid step for a plot spanning `[lo, hi]` with room for
/// `max_ticks` intervals, and never under `least`: a label's height and
/// [`LABEL_AIR`] in the plot's unit, or the finest step its values come in.
///
/// Never more than half the span, short of going under `least`: rounding up
/// to a round step could leave a short plot with one gridline, or none, and
/// so no scale on any axis. Where half the span has no room for two labels
/// the step is under 2.5 times `least`, so a plot with room for that much
/// still holds one of its lines.
pub(super) fn primary_step(lo: f64, hi: f64, max_ticks: usize, least: f64) -> f64 {
    let span = hi - lo;
    let mut step = nice_step((span / max_ticks.max(1) as f64).max(least));
    while step > span / 2.0 && step > f64::MIN_POSITIVE {
        let smaller = smaller_step(step);
        if smaller < least * (1.0 - 1e-9) {
            break;
        }
        step = smaller;
    }
    step
}

/// The round step below `step`: 1 → 0.5, 2 → 1, 5 → 2.
fn smaller_step(step: f64) -> f64 {
    let base = 10f64.powf(step.log10().floor());
    match (step / base).round() as u32 {
        1 => base / 2.0,
        2 => base,
        _ => base * 2.0,
    }
}

/// The step of the unlabelled lines between two of a round `step`'s: a
/// fifth of a 1 or a 5 (1 → 0.2, 0.5 → 0.1), half of a 2 (2 → 1), so a
/// round value itself. None where it would go under `finest`: a level has
/// no line between two whole numbers.
pub(super) fn minor_step(step: f64, finest: f64) -> Option<f64> {
    if !(step.is_finite() && step > 0.0) {
        return None;
    }
    let base = 10f64.powf(step.log10().floor());
    let minor = match (step / base).round() as u32 {
        2 => step / 2.0,
        _ => step / 5.0,
    };
    (minor >= finest * (1.0 - 1e-9)).then_some(minor)
}

/// Whether `v` is a whole multiple of `step`.
fn on_step(v: f64, step: f64) -> bool {
    let k = v / step;
    (k - k.round()).abs() < 1e-6
}

/// The gridlines of a plot spanning `[lo, hi]` on a grid of round `step`s,
/// each with the step of the coarsest level it sits on, which egui_plot
/// draws it the stronger for: the `minor` step where there is one, `step`,
/// then the powers of ten from it up. So 1 V is drawn as a 1 V line on a
/// grid of 0.5 V as it is on egui_plot's decades, and stands out from 0.5 V
/// and 1.5 V the same.
///
/// Three decades, as egui_plot has three levels: the last is a hundred
/// steps at least, past where a line gets any stronger.
pub(super) fn round_grid_marks(lo: f64, hi: f64, step: f64, minor: Option<f64>) -> Vec<(f64, f64)> {
    if !(lo.is_finite() && hi.is_finite() && step.is_finite() && step > 0.0) {
        return Vec::new();
    }
    // egui_plot's own range: a line on the upper edge is left out.
    let multiples =
        |s: f64| (((lo / s).ceil() as i64)..((hi / s).ceil() as i64)).map(move |k| k as f64 * s);
    let decade = 10f64.powf((step.log10() - 1e-9).ceil());
    let mut marks: Vec<(f64, f64)> = multiples(step)
        .map(|v| {
            let level = [decade * 100.0, decade * 10.0, decade]
                .into_iter()
                .find(|&d| on_step(v, d))
                .unwrap_or(step);
            (v, level)
        })
        .collect();
    if let Some(minor) = minor {
        marks.extend(
            multiples(minor)
                .filter(|&v| !on_step(v, step))
                .map(|v| (v, minor)),
        );
    }
    marks
}

/// A right axis's target widened to a span its unit can label sensibly: a
/// steady value (a dBm reading's 600 Ω reference, a 0 Hz frequency with no
/// signal) would otherwise be padded to a sliver and labelled to the 7th
/// decimal. The floor, 0.01 % of the value or 1 at a steady zero, stays under a
/// meter's resolution, so a real wander still fills the axis.
pub(super) fn widen_steady(lo: f64, hi: f64) -> (f64, f64) {
    let mid = (lo + hi) / 2.0;
    let magnitude = lo.abs().max(hi.abs());
    let floor = if magnitude == 0.0 {
        1.0
    } else {
        magnitude * 1e-4
    };
    if hi - lo >= floor {
        (lo, hi)
    } else {
        (mid - floor / 2.0, mid + floor / 2.0)
    }
}

/// The map that fits `target` in a plot spanning `[lo, hi]` of the plotted
/// unit, gridded every `step`: the smallest round step of the unit whose
/// ticks fall on the grid with all of `target` in view, centred as near as
/// the grid allows.
///
/// In grid steps, the plot runs from `u0 = lo/step` for `n` steps, and a
/// value `v` sits at `v/s + c` for the unit's step `s` and a whole shift
/// `c` — so its multiples of `s` land on the grid. `c` has to keep `target`
/// inside the plot; when no whole `c` does, the next round step is tried.
pub(super) fn fit_secondary(lo: f64, hi: f64, step: f64, target: (f64, f64)) -> AxisMap {
    let n = (hi - lo) / step;
    let u0 = lo / step;
    let (t0, t1) = (target.0.min(target.1), target.0.max(target.1));
    let mut s = secondary_step((t1 - t0) / n);
    for _ in 0..32 {
        // `c` at least `low` keeps `t0` in view, at most `high` keeps `t1`.
        let low = u0 - t0 / s;
        let high = u0 + n - t1 / s;
        let (c_min, c_max) = ((low - 1e-9).ceil(), (high + 1e-9).floor());
        if c_min <= c_max {
            let c = ((low + high) / 2.0).round().clamp(c_min, c_max);
            let mantissa = s / 10f64.powf(s.log10().floor());
            return AxisMap {
                offset: c * step,
                scale: step / s,
                step: s,
                label_step: if (mantissa - 2.5).abs() < 1e-6 {
                    2.0 * s
                } else {
                    s
                },
            };
        }
        s = secondary_step(s * 1.01);
    }
    // Each round step at least doubles the last, so a few dozen outgrow any
    // target on a plot of some height; the caller keeps a flat or
    // non-finite plot from getting here. Centre it anyway.
    AxisMap {
        offset: (lo + hi) / 2.0 - (t0 + t1) / 2.0,
        scale: 1.0,
        step: s,
        label_step: s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every primary tick in `[lo, hi]` lands on a whole multiple of the
    /// map's step, and the target stays in view.
    fn assert_aligned(lo: f64, hi: f64, step: f64, target: (f64, f64)) -> AxisMap {
        let map = fit_secondary(lo, hi, step, target);
        let first = (lo / step).ceil() as i64;
        let last = (hi / step).floor() as i64;
        assert!(last > first, "at least two ticks");
        for k in first..=last {
            let v = map.value_at(k as f64 * step) / map.step;
            assert!((v - v.round()).abs() < 1e-6, "tick {k} reads {v} steps");
        }
        let (v_lo, v_hi) = (map.value_at(lo), map.value_at(hi));
        assert!(
            v_lo <= target.0 + 1e-9 && v_hi >= target.1 - 1e-9,
            "{target:?} in {v_lo}..{v_hi}"
        );
        map
    }

    /// An axis keeps the plotted unit over the span it writes well, and
    /// past it takes the prefix its larger end reads in.
    #[test]
    fn an_axis_takes_another_prefix_only_past_its_plain_span() {
        let prefix = |unit, lo, hi| axis_prefix(unit, lo, hi).map(|(u, r)| (u, r.apply(1.0)));
        assert_eq!(
            prefix("\u{3a9}", -1e6, 12e6),
            Some(("M\u{3a9}".to_string(), 1e-6))
        );
        assert_eq!(
            prefix("M\u{3a9}", 0.0, 4e-7),
            Some(("m\u{3a9}".to_string(), 1e9))
        );
        assert_eq!(prefix("M\u{3a9}", 0.0, 13.0), None);
        assert_eq!(
            prefix("V", 0.0, 0.5),
            None,
            "a reading under 1 V stays in V"
        );
        assert_eq!(
            prefix("\u{3a9}", 990.0, 1010.0),
            None,
            "no flip across a decade"
        );
        assert_eq!(prefix("V", 0.0, 0.0), None);
        assert_eq!(prefix("\u{b0}C", 0.0, 1e5), None, "no prefix goes on it");
    }

    #[test]
    fn round_steps() {
        assert_eq!(nice_step(0.7), 1.0);
        assert_eq!(nice_step(1.0), 1.0);
        assert_eq!(nice_step(1.3), 2.0);
        assert_eq!(nice_step(3.0), 5.0);
        assert_eq!(nice_step(0.031), 0.05);
        assert_eq!(nice_step(7.0), 10.0);
        assert_eq!(nice_step(0.0), 1.0);
    }

    #[test]
    fn decimals_write_every_tick() {
        assert_eq!(step_decimals(0.05), 2);
        assert_eq!(step_decimals(0.2), 1);
        assert_eq!(step_decimals(0.1), 1);
        assert_eq!(step_decimals(0.01), 2);
        assert_eq!(step_decimals(1.0), 0);
        assert_eq!(step_decimals(50.0), 0);
    }

    /// Mains voltage beside its frequency and period: each right axis reads
    /// in round steps on the voltage's grid.
    #[test]
    fn mains_frequency_and_period_align_on_the_volt_grid() {
        let (lo, hi) = (229.76, 230.44);
        let step = primary_step(lo, hi, 6, 0.0);
        assert_eq!(step, 0.2);
        let hz = assert_aligned(lo, hi, step, (49.976, 50.024));
        assert_eq!(hz.step, 0.02);
        let ms = assert_aligned(lo, hi, step, (19.9904, 20.0096));
        assert_eq!(ms.step, 0.01);
    }

    /// A steady value, such as the dBm reference impedance, is widened to a
    /// span its axis can label in a few digits, and sits mid-plot.
    #[test]
    fn a_steady_value_sits_mid_plot_in_few_digits() {
        let (lo, hi) = (-10.6, 2.6);
        let step = primary_step(lo, hi, 6, 0.0);
        let map = assert_aligned(lo, hi, step, widen_steady(600.0, 600.0));
        assert!(map.decimals() <= 2, "step {}", map.step);
        let y = map.height_of(600.0);
        assert!((y - (lo + hi) / 2.0).abs() <= step, "{y}");
        let zero = fit_secondary(lo, hi, step, widen_steady(0.0, 0.0));
        assert!(zero.decimals() <= 1, "step {}", zero.step);
    }

    /// A wander either side of zero is a reading, not a steady value: it
    /// keeps its span.
    #[test]
    fn a_wander_around_zero_keeps_its_span() {
        assert_eq!(widen_steady(-0.002, 0.002), (-0.002, 0.002));
    }

    /// A short plot still gets two gridlines, so every axis has a scale.
    #[test]
    fn a_short_plot_keeps_two_gridlines() {
        let step = primary_step(0.2, 4.6, 2, 0.0);
        assert_eq!(step, 2.0);
        assert_eq!(primary_step(0.0, 1.0, 1, 0.0), 0.5);
        assert_eq!(primary_step(0.0, 10.0, 1, 0.0), 5.0);
    }

    /// Lines too close for a label each give way to one line that has the
    /// room: half of 0 to 9.9 is under 5, and 2 would put five lines where
    /// three labels fit.
    #[test]
    fn a_step_keeps_room_for_its_label() {
        assert_eq!(primary_step(0.0, 9.9, 2, 0.0), 2.0);
        assert_eq!(primary_step(0.0, 9.9, 2, 3.0), 5.0);
        assert_eq!(primary_step(0.0, 9.9, 2, 2.0), 2.0);
        // A level comes in whole numbers.
        assert_eq!(primary_step(0.0, 1.5, 2, 1.0), 1.0);
    }

    /// The lines between two steps are a round value that divides the step,
    /// and none under what the axis can show.
    #[test]
    fn a_minor_step_is_a_round_share_of_its_step() {
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < b * 1e-9);
        assert!(close(minor_step(0.5, 0.0), 0.1));
        assert!(close(minor_step(1.0, 0.0), 0.2));
        assert!(close(minor_step(2.0, 0.0), 1.0));
        assert!(close(minor_step(5.0, 0.0), 1.0));
        assert!(close(minor_step(0.02, 0.0), 0.01));
        assert!(close(minor_step(0.001, 0.0), 0.0002));
        assert!(close(minor_step(1000.0, 0.0), 200.0));
        // A level is a whole number.
        assert_eq!(minor_step(1.0, 1.0), None);
        assert_eq!(minor_step(2.0, 1.0), Some(1.0));
        assert_eq!(minor_step(5.0, 1.0), Some(1.0));
        assert_eq!(minor_step(10.0, 1.0), Some(2.0));
        assert_eq!(minor_step(0.0, 0.0), None);
    }

    /// The level of the mark at `v`, if there is one.
    fn level_at(marks: &[(f64, f64)], v: f64) -> Option<f64> {
        let found: Vec<f64> = marks
            .iter()
            .filter(|(at, _)| (at - v).abs() < 1e-9)
            .map(|&(_, level)| level)
            .collect();
        assert!(found.len() <= 1, "{v} marked {found:?}");
        found.first().copied()
    }

    /// Each line carries the coarsest level it sits on: the minor step, the
    /// step, then the powers of ten from the step up.
    #[test]
    fn a_gridline_takes_its_coarsest_round_level() {
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < b * 1e-9);
        // 0.5 V steps: 0.1 between, and 1 V a line of the 1 V level.
        let marks = round_grid_marks(-0.15, 2.05, 0.5, minor_step(0.5, 0.0));
        assert_eq!(marks.len(), 22, "{marks:?}");
        assert!(close(level_at(&marks, 0.1), 0.1));
        assert!(close(level_at(&marks, -0.1), 0.1));
        assert!(close(level_at(&marks, 0.5), 0.5));
        assert!(close(level_at(&marks, 1.5), 0.5));
        assert!(close(level_at(&marks, 1.0), 1.0));
        assert!(close(level_at(&marks, 2.0), 1.0));
        assert!(close(level_at(&marks, 0.0), 100.0));
        // 2 V steps: 1 between, then 10 and 100.
        let marks = round_grid_marks(1.5, 101.0, 2.0, minor_step(2.0, 0.0));
        assert!(close(level_at(&marks, 3.0), 1.0));
        assert!(close(level_at(&marks, 4.0), 2.0));
        assert!(close(level_at(&marks, 10.0), 10.0));
        assert!(close(level_at(&marks, 50.0), 10.0));
        assert!(close(level_at(&marks, 100.0), 100.0));
        // 5 V steps: 1 between.
        let marks = round_grid_marks(0.5, 21.0, 5.0, minor_step(5.0, 0.0));
        assert!(close(level_at(&marks, 4.0), 1.0));
        assert!(close(level_at(&marks, 5.0), 5.0));
        assert!(close(level_at(&marks, 15.0), 5.0));
        assert!(close(level_at(&marks, 20.0), 10.0));
        // 1 V steps: 0.2 between, a step is the first power of ten.
        let marks = round_grid_marks(0.5, 21.0, 1.0, minor_step(1.0, 0.0));
        assert!(close(level_at(&marks, 0.6), 0.2));
        assert!(close(level_at(&marks, 3.0), 1.0));
        assert!(close(level_at(&marks, 20.0), 10.0));
        // Levels a whole number apart have nothing between, and without a
        // minor step the grid is its steps alone.
        let marks = round_grid_marks(-0.5, 3.5, 1.0, minor_step(1.0, 1.0));
        assert_eq!(marks.len(), 4, "{marks:?}");
        // A line on the upper edge is left out, as egui_plot leaves it.
        assert_eq!(round_grid_marks(0.0, 2.0, 1.0, None).len(), 2);
        assert!(round_grid_marks(0.0, f64::INFINITY, 1.0, None).is_empty());
    }

    /// egui_plot's decades are kept where two of their lines are sure to be
    /// labelled readably, and only there.
    #[test]
    fn decades_serve_a_plot_with_two_labelled_lines() {
        // 1 V lines 38 points apart, six of them.
        assert!(decades_label_two(6.6, 250.0, 0.0));
        // 1 V lines 7 points apart, and at most one 10 V line.
        assert!(!decades_label_two(6.6, 46.0, 0.0));
        // 100 V lines 60 points apart.
        assert!(decades_label_two(1000.0, 600.0, 0.0));
        // Levels 40 points apart, but no half levels between two of them.
        assert!(decades_label_two(3.0, 120.0, 1.0));
        assert!(!decades_label_two(1.5, 120.0, 1.0));
        // 0 to 110 mA with its margins: 10 mA lines 25.6 points apart, faded
        // a little and read all the same.
        assert!(decades_label_two(130.0, 333.0, 0.0));
        // The same lines 24 points apart, and a hair under: one 100 mA line.
        assert!(decades_label_two(130.0, 312.0, 0.0));
        assert!(!decades_label_two(130.0, 311.0, 0.0));
    }

    /// The readable spacing is the whole number its doc reckons with, not
    /// a rounding off it: 130 over 312 points is on the line exactly.
    #[test]
    fn the_readable_spacing_is_two_fifths_into_the_fade() {
        assert_eq!(READABLE_LABEL_SPACING, 24.0);
    }

    /// A plot keeps the grid it has while its range is near the line
    /// between the two: it leaves its decades where they stop serving it,
    /// and comes back to them only well inside where they do.
    #[test]
    fn a_plot_near_the_line_keeps_its_grid() {
        // 100 points high: the decades serve 2 to 4.17 and 20 to 41.7.
        for (span, serve) in [(1.9, false), (2.0, true), (4.1, true), (4.2, false)] {
            assert_eq!(decades_label_two(span, 100.0, 0.0), serve, "{span}");
            // First drawn, and on its decades: as they serve it.
            assert_eq!(takes_decades(span, 100.0, 0.0, false), serve, "{span}");
        }
        // On round steps: not until a fifth clear of both lines.
        for (span, back) in [(2.0, false), (2.4, false), (2.6, true), (3.4, true)] {
            assert_eq!(takes_decades(span, 100.0, 0.0, true), back, "{span}");
        }
        assert!(!takes_decades(3.6, 100.0, 0.0, true), "too near 4.17");
        assert!(!takes_decades(1.9, 100.0, 0.0, true));
        // A window dragged taller: 0 to 130 has its 10 mA lines 24 points
        // apart at 312, where a plot on its decades keeps them, and one on
        // round steps takes them once they are a fifth further apart.
        assert!(takes_decades(130.0, 312.0, 0.0, false));
        assert!(!takes_decades(130.0, 312.0, 0.0, true));
        assert!(!takes_decades(130.0, 374.0, 0.0, true));
        assert!(takes_decades(130.0, 375.0, 0.0, true));
        // A plot tall enough for one decade or the next at any span has no
        // line to be near.
        assert!(takes_decades(4.2, 600.0, 0.0, true));
    }

    #[test]
    fn negative_and_straddling_ranges() {
        assert_aligned(-5.2, -1.1, 0.5, (-0.033, 0.021));
        assert_aligned(-1.0, 1.0, 0.5, (-120.0, -80.0));
        assert_aligned(1e-6, 9e-6, 1e-6, (3.0e3, 3.5e3));
    }

    /// A wander that a 0.02 step would squeeze and 0.05 flatten takes 0.025
    /// per gridline, labelled every other one, in whole 0.05s.
    #[test]
    fn a_half_step_labels_every_other_gridline() {
        let (lo, hi) = (117.6, 122.4);
        let map = assert_aligned(lo, hi, 1.0, (59.94, 60.06));
        assert_eq!((map.step, map.label_step), (0.025, 0.05));
        assert_eq!(map.decimals(), 2);
        let labelled: Vec<f64> = (118..=122)
            .map(|k| map.value_at(k as f64))
            .filter(|&v| map.labels(v))
            .collect();
        assert!((2..=3).contains(&labelled.len()), "{labelled:?}");
        for v in labelled {
            assert!(((v / 0.05) - (v / 0.05).round()).abs() < 1e-6, "{v}");
        }
    }

    /// A target that only just fits one step size moves up a step rather
    /// than spilling out of view.
    #[test]
    fn a_tight_target_takes_the_next_step() {
        // Four steps tall; 0.99 to 4.01 needs more than 4 × 1.
        let map = assert_aligned(0.0, 4.0, 1.0, (0.99, 4.01));
        assert_eq!(map.step, 2.0);
    }
}
