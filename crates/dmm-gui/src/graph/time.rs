//! Formatting seconds-from-origin values for the time axis and the minimap.

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
    let sec_decimals: usize = if step_size > 0.0 && step_size < 1.0 {
        ((-step_size.log10()).round() as i64).clamp(1, 6) as usize
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
        if sec_decimals > 0 {
            format!("{h:.0}h {m:.0}m {sec:.sec_decimals$}s")
        } else {
            format!("{h:.0}h {m:.0}m")
        }
    }
}
