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

/// The plotted unit's grid step for a plot spanning `[lo, hi]` with room for
/// `max_ticks` intervals.
///
/// Never more than half the span: rounding up to a round step could leave a
/// short plot with one gridline, or none, and so no scale on any axis.
pub(super) fn primary_step(lo: f64, hi: f64, max_ticks: usize) -> f64 {
    let span = hi - lo;
    let mut step = nice_step(span / max_ticks.max(1) as f64);
    while step > span / 2.0 && step > f64::MIN_POSITIVE {
        step = smaller_step(step);
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
        let step = primary_step(lo, hi, 6);
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
        let step = primary_step(lo, hi, 6);
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
        let step = primary_step(0.2, 4.6, 2);
        assert_eq!(step, 2.0);
        assert_eq!(primary_step(0.0, 1.0, 1), 0.5);
        assert_eq!(primary_step(0.0, 10.0, 1), 5.0);
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
