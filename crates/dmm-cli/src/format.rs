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
            // Through the csv crate rather than hand-joined with commas, as
            // the GUI export already does. Several of these fields carry
            // device-derived text: UT181A units come from `parse_unit_string`,
            // which maps raw frame bytes to chars with no character-set
            // validation, and an unrecognised mode byte becomes
            // `Unknown(0x..)`. One comma or quote in there and every
            // downstream column shifts.
            let value_str = m.value_export_str();
            let ts = timestamp_rfc3339(m, wall_clock);
            let flags = m.flags.to_string();
            let mut wtr = csv::WriterBuilder::new()
                // One row per call, so the default 8 KiB buffer is dead
                // weight — a row is well under this.
                .buffer_capacity(256)
                .from_writer(w);
            let mut record: Vec<&str> = vec![
                &ts,
                &m.mode,
                value_str.as_ref(),
                &m.unit,
                &m.range_label,
                &flags,
            ];
            let integral_str;
            if let Some((val, unit)) = integral {
                integral_str = format!("{val:.6}");
                record.push(&integral_str);
                record.push(unit);
            }
            wtr.write_record(&record).map_err(std::io::Error::other)?;
            wtr.flush()
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

    fn csv_for(m: &Measurement) -> String {
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            m,
            &WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
        )
        .unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// A comma in any device-derived field used to shift every column after
    /// it. UT181A units come straight off the wire.
    #[test]
    fn csv_quotes_a_field_containing_a_comma() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        m.mode = "Unknown(0x05), odd".into();
        let line = csv_for(&m);
        assert!(
            line.contains("\"Unknown(0x05), odd\""),
            "mode should be quoted, got {line}"
        );
        // Six fields, so five separating commas outside the quoted one.
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        let rec = rdr.records().next().unwrap().unwrap();
        assert_eq!(rec.len(), 6);
        assert_eq!(&rec[1], "Unknown(0x05), odd");
    }

    #[test]
    fn csv_escapes_quotes_and_newlines() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        m.unit = "a\"b\nc".into();
        let line = csv_for(&m);
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        let rec = rdr.records().next().unwrap().unwrap();
        assert_eq!(rec.len(), 6);
        assert_eq!(&rec[3], "a\"b\nc");
    }

    /// Ordinary rows must stay unquoted — the column layout is documented
    /// and consumed by spreadsheets.
    #[test]
    fn csv_leaves_plain_fields_unquoted() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let line = csv_for(&m);
        assert!(!line.contains('"'), "got {line}");
        assert!(line.contains(",DC V,5.678,V,22V,"), "got {line}");
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
