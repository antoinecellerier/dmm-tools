use std::io::Write;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

use crate::OutputFormat;

pub fn format_measurement(
    w: &mut dyn Write,
    m: &Measurement,
    format: &OutputFormat,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => {
            writeln!(w, "{m}")
        }
        OutputFormat::Csv => {
            let value_str = match &m.value {
                MeasuredValue::Normal(v) => v.to_string(),
                MeasuredValue::Overload => "OL".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
            };
            writeln!(
                w,
                "{},{},{},{},{},{}",
                chrono::Local::now().to_rfc3339(),
                m.mode,
                value_str,
                m.unit,
                m.range_label,
                m.flags,
            )
        }
        OutputFormat::Json => {
            let value = match &m.value {
                MeasuredValue::Normal(v) => serde_json::json!(v),
                MeasuredValue::Overload => serde_json::json!("OL"),
                MeasuredValue::NcvLevel(l) => serde_json::json!({"ncv_level": l}),
            };
            let obj = serde_json::json!({
                "timestamp": chrono::Local::now().to_rfc3339(),
                "mode": m.mode.to_string(),
                "value": value,
                "unit": m.unit,
                "range": m.range_label,
                "display_raw": m.display_raw,
                "progress": m.progress,
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
                }
            });
            writeln!(
                w,
                "{}",
                serde_json::to_string(&obj).map_err(std::io::Error::other)?
            )
        }
    }
}
