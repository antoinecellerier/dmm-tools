//! Formatting seconds-from-origin values for the time axis and the minimap,
//! and the steps the time axis is gridded in.

use egui_plot::{GridInput, GridMark};

/// Points between the time axis's labels, at least: egui_plot shows labels
/// from 60 points apart and at full strength from 80.
const TIME_LABEL_SPACING: f64 = 80.0;

/// The least spacing the main plot draws gridlines at, which egui_plot
/// hands a grid spacer as `base_step_size`: the plot sets it from here.
pub(super) const MIN_GRID_SPACING: f32 = 8.0;

/// Steps a time axis is gridded and labelled in, in seconds: 1, 2 and 5
/// times the decades under a second, then the steps a clock is read in.
fn clock_steps() -> impl Iterator<Item = f64> {
    (-3..0)
        .flat_map(|e| [1.0, 2.0, 5.0].map(|m| m * 10f64.powi(e)))
        .chain([
            1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0,
            7200.0, 10800.0, 21600.0, 43200.0, 86400.0,
        ])
}

/// The time axis's gridline steps for `seconds_per_point`, finest first:
/// labels at the first clock step [`TIME_LABEL_SPACING`] points apart, and
/// lines at the two clock steps under it that divide it evenly.
pub(super) fn time_steps(seconds_per_point: f64) -> [f64; 3] {
    let wanted = seconds_per_point * TIME_LABEL_SPACING;
    let label = clock_steps()
        .find(|&s| s >= wanted)
        .unwrap_or_else(|| (wanted / 86400.0).ceil() * 86400.0);
    let finer = |step: f64| {
        clock_steps()
            .filter(|&s| s < step * (1.0 - 1e-9))
            .filter(|&s| ((step / s) - (step / s).round()).abs() < 1e-6)
            .last()
            .unwrap_or(step)
    };
    let mid = finer(label);
    [finer(mid), mid, label]
}

/// The main plot's time grid for egui_plot, on [`time_steps`]. Its own
/// decade steps jump from 10 s to 100 s, so a window of 70–100 s on a
/// narrow plot had its 10 s labels hidden and kept one.
pub(super) fn time_marks(input: GridInput) -> Vec<GridMark> {
    // `base_step_size` spans the plot's least grid spacing.
    let seconds_per_point = input.base_step_size / f64::from(MIN_GRID_SPACING);
    egui_plot::uniform_grid_spacer(move |_| time_steps(seconds_per_point))(input)
}

/// Choose a nice round interval for time axis labels.
pub(super) fn nice_time_interval(span: f64) -> f64 {
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

/// A time on the plot as the crosshair names it, to a tenth of a second:
/// `42.3 s`, `1m 12.4s`, `2h 0m 5.0s`. Rounded before it is split, so
/// 119.97 s reads `2m 0.0s`, never `1m 60.0s`.
pub(super) fn format_pointer_time(t: f64) -> String {
    let tenths = (t * 10.0).round();
    let t = tenths / 10.0;
    if t < 60.0 {
        return format!("{t:.1} s");
    }
    let s = (tenths % 600.0) / 10.0;
    let m = (tenths / 600.0).floor();
    if m < 60.0 {
        format!("{m:.0}m {s:.1}s")
    } else {
        format!("{:.0}h {:.0}m {s:.1}s", (m / 60.0).floor(), m % 60.0)
    }
}

/// Format a time value in seconds as a readable label.
pub(super) fn format_time_label(secs: f64) -> String {
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
pub(super) fn format_time_axis_label(value: f64, step_size: f64) -> String {
    // When step is a sub-second power of 10 (egui_plot default log-10
    // spacer), show enough decimals to resolve adjacent marks. clamp to 1
    // so a rounding step like 0.5 (non-power-of-10) still gets at least
    // one decimal of precision.
    // Up, not to nearest: a 0.05 s step needs two decimals, or 9.05 s and
    // 9.10 s both read "9.1 s".
    let sec_decimals: usize = if step_size > 0.0 && step_size < 1.0 {
        ((-step_size.log10() - 1e-9).ceil() as i64).clamp(1, 6) as usize
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
        // Same rule as under an hour: the seconds go only when there are
        // none. Dropping them for every whole-second step labelled a 1 m
        // window six times over as "23h 59m" once a session ran past an
        // hour.
        if sec_decimals == 0 && sec.abs() < 0.5 {
            format!("{h:.0}h {m:.0}m")
        } else {
            format!("{h:.0}h {m:.0}m {sec:.sec_decimals$}s")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Labels at round clock steps at least 80 points apart, never the 1 s,
    /// 10 s, 100 s decades: a 97 s window on a 553-point plot is labelled
    /// every 15 s, with lines every 5 s and 1 s.
    /// Neighbouring labels never read the same: a step's decimals resolve
    /// it, 0.05 s steps included.
    #[test]
    fn sub_second_labels_tell_their_steps_apart() {
        assert_eq!(format_time_axis_label(9.05, 0.05), "9.05 s");
        assert_eq!(format_time_axis_label(9.1, 0.05), "9.10 s");
        assert_eq!(format_time_axis_label(9.5, 0.5), "9.5 s");
        assert_eq!(format_time_axis_label(9.2, 0.2), "9.2 s");
        assert_eq!(format_time_axis_label(9.1, 0.1), "9.1 s");
    }

    #[test]
    fn pointer_times_round_before_they_split() {
        assert_eq!(format_pointer_time(42.34), "42.3 s");
        assert_eq!(format_pointer_time(59.97), "1m 0.0s");
        assert_eq!(format_pointer_time(72.4), "1m 12.4s");
        assert_eq!(format_pointer_time(119.97), "2m 0.0s");
        assert_eq!(format_pointer_time(7205.0), "2h 0m 5.0s");
        assert_eq!(format_pointer_time(3599.96), "1h 0m 0.0s");
    }

    #[test]
    fn time_labels_fall_on_clock_steps() {
        assert_eq!(time_steps(97.0 / 553.0), [1.0, 5.0, 15.0]);
        assert_eq!(time_steps(60.0 / 1100.0), [0.5, 1.0, 5.0]);
        assert_eq!(time_steps(600.0 / 553.0), [30.0, 60.0, 120.0]);
        let short = time_steps(3.0 / 553.0);
        for (got, want) in short.into_iter().zip([0.05, 0.1, 0.5]) {
            assert!((got - want).abs() < 1e-12, "{short:?}");
        }
        assert_eq!(time_steps(7.0 * 86400.0 / 553.0)[2], 2.0 * 86400.0);
    }
}
