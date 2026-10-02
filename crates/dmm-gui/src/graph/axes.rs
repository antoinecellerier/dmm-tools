//! Y axes for sub-values in other units, aligned on the plotted series' grid.
//!
//! egui_plot draws every Y axis from one transform and one grid spacer, so a
//! right axis can only label the tick positions of the plotted unit. Each
//! other unit is therefore mapped onto the plot as `y = offset + scale·v`,
//! chosen so those shared ticks land on round values of its own: every axis
//! reads in steps of 1, 2 or 5 × 10ⁿ, and one set of gridlines serves them
//! all.

/// Where one unit's values sit on the plot: `y = offset + scale·v`, with a
/// tick every `step` of the unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AxisMap {
    offset: f64,
    scale: f64,
    pub step: f64,
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

    /// The decimals the axis's ticks need.
    pub(super) fn decimals(self) -> usize {
        step_decimals(self.step)
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
    let mut s = nice_step((t1 - t0) / n);
    for _ in 0..32 {
        // `c` at least `low` keeps `t0` in view, at most `high` keeps `t1`.
        let low = u0 - t0 / s;
        let high = u0 + n - t1 / s;
        let (c_min, c_max) = ((low - 1e-9).ceil(), (high + 1e-9).floor());
        if c_min <= c_max {
            let c = ((low + high) / 2.0).round().clamp(c_min, c_max);
            return AxisMap {
                offset: c * step,
                scale: step / s,
                step: s,
            };
        }
        s = nice_step(s * 1.5);
    }
    // Each round step at least doubles the last, so a few dozen outgrow any
    // target on a plot of some height; the caller keeps a flat or
    // non-finite plot from getting here. Centre it anyway.
    AxisMap {
        offset: (lo + hi) / 2.0 - (t0 + t1) / 2.0,
        scale: 1.0,
        step: s,
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

    /// A target that only just fits one step size moves up a step rather
    /// than spilling out of view.
    #[test]
    fn a_tight_target_takes_the_next_step() {
        // Four steps tall; 0.99 to 4.01 needs more than 4 × 1.
        let map = assert_aligned(0.0, 4.0, 1.0, (0.99, 4.01));
        assert_eq!(map.step, 2.0);
    }
}
