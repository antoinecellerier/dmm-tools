use chrono::{DateTime, Local};
use dmm_lib::WallClock;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use std::io::Write;

use crate::OutputFormat;

/// Derive a wall-clock RFC3339 string from the measurement's monotonic
/// timestamp using the session's `WallClock` origin. Keeps exported
/// timestamps aligned with when the device produced the reading rather than
/// when the formatter ran.
fn timestamp_rfc3339(m: &Measurement, wall_clock: &WallClock) -> String {
    let sys_time = wall_clock.wall_time_for(m.timestamp);
    let dt: DateTime<Local> = sys_time.into();
    dt.to_rfc3339()
}

pub fn format_measurement(
    w: &mut dyn Write,
    m: &Measurement,
    wall_clock: &WallClock,
    format: &OutputFormat,
    experimental: bool,
    integral: Option<(f64, &str)>,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => {
            if let Some((val, unit)) = integral {
                writeln!(w, "{m} [\u{222b} {val:.4} {unit}]")
            } else {
                writeln!(w, "{m}")
            }
        }
        OutputFormat::Csv => {
            let value_str = match &m.value {
                MeasuredValue::Normal(v) => m
                    .display_raw
                    .as_deref()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| v.to_string()),
                MeasuredValue::Overload => "OL".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
            };
            let ts = timestamp_rfc3339(m, wall_clock);
            if let Some((val, unit)) = integral {
                writeln!(
                    w,
                    "{},{},{},{},{},{},{:.6},{unit}",
                    ts, m.mode, value_str, m.unit, m.range_label, m.flags, val,
                )
            } else {
                writeln!(
                    w,
                    "{},{},{},{},{},{}",
                    ts, m.mode, value_str, m.unit, m.range_label, m.flags,
                )
            }
        }
        OutputFormat::Json => {
            let value = match &m.value {
                MeasuredValue::Normal(v) => serde_json::json!(v),
                MeasuredValue::Overload => serde_json::json!("OL"),
                MeasuredValue::NcvLevel(l) => serde_json::json!({"ncv_level": l}),
            };
            let mut obj = serde_json::json!({
                "timestamp": timestamp_rfc3339(m, wall_clock),
                "mode": m.mode,
                "value": value,
                "unit": m.unit,
                "range": m.range_label,
                "display_raw": m.display_raw,
                "progress": m.progress,
                "experimental": experimental,
                "flags": {
                    "hold": m.flags.hold,
                    "rel": m.flags.rel,
                    "auto_range": m.flags.auto_range,
                    "min": m.flags.min,
                    "max": m.flags.max,
                    "low_battery": m.flags.low_battery,
                    "hv_warning": m.flags.hv_warning,
                    "dc": m.flags.dc,
                    "peak_min": m.flags.peak_min,
                    "peak_max": m.flags.peak_max,
                    "lead_error": m.flags.lead_error,
                    "comp": m.flags.comp,
                    "record": m.flags.record,
                }
            });
            if let Some((val, unit)) = integral {
                obj["integral"] = serde_json::json!(val);
                obj["integral_unit"] = serde_json::json!(unit);
            }
            writeln!(
                w,
                "{}",
                serde_json::to_string(&obj).map_err(std::io::Error::other)?
            )
        }
    }
}
