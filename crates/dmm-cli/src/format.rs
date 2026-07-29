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
            let value_str = m.value_export_str();
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
            // Built from StatusFlags::as_pairs rather than a hand-written
            // list: the old list had drifted and was missing `loz` and
            // `void`, so a VC-890 reading the meter had marked invalid was
            // indistinguishable from a good one in JSON — while the text and
            // CSV formats reported it.
            let flags: serde_json::Map<String, serde_json::Value> = m
                .flags
                .as_pairs()
                .into_iter()
                .map(|(name, set)| (name.to_string(), serde_json::json!(set)))
                .collect();
            let mut obj = serde_json::json!({
                "timestamp": timestamp_rfc3339(m, wall_clock),
                "mode": m.mode,
                "value": value,
                "unit": m.unit,
                "range": m.range_label,
                "display_raw": m.display_raw,
                "progress": m.progress,
                "experimental": experimental,
                "flags": flags,
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

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;

    fn json_for(flags: StatusFlags) -> serde_json::Value {
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", flags);
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            &m,
            &WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
        )
        .unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    /// The JSON flags object used to be hand-written and had drifted: `loz`
    /// and `void` were missing, so a VC-890 reading the meter had marked
    /// invalid looked clean to any script reading JSON.
    #[test]
    fn json_flags_cover_every_status_flag() {
        let v = json_for(StatusFlags::default());
        let obj = v["flags"].as_object().expect("flags object");
        assert_eq!(obj.len(), StatusFlags::COUNT);
        for (name, _) in StatusFlags::default().as_pairs() {
            assert!(obj.contains_key(name), "JSON flags missing {name}");
        }
    }

    #[test]
    fn json_reports_loz_and_void() {
        let v = json_for(StatusFlags {
            loz: true,
            void: true,
            ..Default::default()
        });
        assert_eq!(v["flags"]["loz"], serde_json::json!(true));
        assert_eq!(v["flags"]["void"], serde_json::json!(true));
        assert_eq!(v["flags"]["hold"], serde_json::json!(false));
    }

    /// Text output already carried these; the three formats must agree.
    #[test]
    fn text_and_json_agree_on_void() {
        let flags = StatusFlags {
            void: true,
            ..Default::default()
        };
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", flags);
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            &m,
            &WallClock::new(),
            &OutputFormat::Text,
            false,
            None,
        )
        .unwrap();
        assert!(String::from_utf8(buf).unwrap().contains("VOID"));
        assert_eq!(json_for(flags)["flags"]["void"], serde_json::json!(true));
    }
}
