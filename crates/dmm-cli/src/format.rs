use chrono::{DateTime, Local};
use dmm_lib::WallClock;
use dmm_lib::measurement::{AUX_EXPORT_COLUMNS, MeasuredValue, Measurement};
use std::borrow::Cow;
use std::io::Write;

use crate::OutputFormat;

/// The CSV header for a run, matching what [`format_measurement`] writes for
/// the same `integrate`/`aux_slots` pair.
///
/// `aux_slots` is the meter family's `max_aux_values`, not the count in any
/// one reading: the column layout has to be fixed for the whole file, so a
/// mode that reports fewer sub-values leaves the trailing slots empty. The
/// aux groups come last, after the integral columns, so `--integrate`
/// consumers keep the positions they had before sub-values were exported.
pub fn csv_header(integrate: bool, aux_slots: usize) -> String {
    let mut header = String::from("timestamp,mode,value,unit,range,flags");
    if integrate {
        header.push_str(",integral,integral_unit");
    }
    for i in 1..=aux_slots {
        for suffix in AUX_EXPORT_COLUMNS {
            header.push_str(&format!(",aux{i}_{suffix}"));
        }
    }
    header
}

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
    // Sub-value columns to write in CSV, from the family's `max_aux_values`.
    // Ignored by the text and JSON arms, which size themselves per reading.
    aux_slots: usize,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => {
            if let Some((val, unit)) = integral {
                writeln!(w, "{m} [\u{222b} {val:.4} {unit}]")?;
            } else {
                writeln!(w, "{m}")?;
            }
            // Sub-values, indented under the reading they belong to. The
            // UT181A produces these in REL (Reference/Absolute), MIN/MAX
            // (Max/Average/Min with timestamps) and peak modes, and the UT171
            // for the AC frequency aux; before this they were parsed and
            // discarded.
            let label_w = m
                .aux_values
                .iter()
                .map(|a| a.label.chars().count())
                .max()
                .unwrap_or(0);
            for aux in &m.aux_values {
                let unit = aux.unit_or(&m.unit);
                let elapsed = aux
                    .elapsed_secs
                    .map(|s| format!(" @{s}s"))
                    .unwrap_or_default();
                writeln!(
                    w,
                    "  {:<label_w$}  {} {unit}{elapsed}",
                    aux.label,
                    aux.value_str()
                )?;
            }
            Ok(())
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
            // Resolved ahead of the record so the borrowed cells outlive it.
            // `take` guards the layout: a reading with more sub-values than
            // the profile promised is truncated rather than shifting every
            // later column (or panicking) mid-file.
            let aux_cells: Vec<[Cow<'_, str>; AUX_EXPORT_COLUMNS.len()]> = m
                .aux_values
                .iter()
                .take(aux_slots)
                .map(|aux| aux.export_cells(&m.unit))
                .collect();
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
            // Every slot is written whether or not this reading filled it —
            // the file's column layout is fixed for the whole run.
            for slot in 0..aux_slots {
                match aux_cells.get(slot) {
                    Some(cells) => record.extend(cells.iter().map(|c| c.as_ref())),
                    None => record.extend(std::iter::repeat_n("", AUX_EXPORT_COLUMNS.len())),
                }
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
            // Omitted entirely when there are none, so output for the
            // families that never produce sub-values is unchanged.
            if !m.aux_values.is_empty() {
                obj["aux"] = serde_json::json!(
                    m.aux_values
                        .iter()
                        .map(|aux| {
                            let unit = aux.unit_or(&m.unit);
                            serde_json::json!({
                                "label": aux.label,
                                "value": aux.value_str(),
                                "unit": unit,
                                "elapsed_secs": aux.elapsed_secs,
                            })
                        })
                        .collect::<Vec<_>>()
                );
            }
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
            0,
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

    fn with_aux(m: &mut Measurement) {
        use dmm_lib::measurement::AuxValue;
        m.aux_values = vec![
            AuxValue {
                label: "Reference".into(),
                value: MeasuredValue::Normal(1.234),
                unit: "".into(), // empty = same as the main reading
                display_raw: Some("1.2340".to_string()),
                elapsed_secs: None,
            },
            AuxValue {
                label: "Max".into(),
                value: MeasuredValue::Overload,
                unit: "mV".into(),
                display_raw: None,
                elapsed_secs: Some(42),
            },
        ];
    }

    /// UT181A REL/MIN-MAX sub-values were parsed and then discarded by every
    /// output format.
    #[test]
    fn text_output_lists_aux_values() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_aux(&mut m);
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            &m,
            &WallClock::new(),
            &OutputFormat::Text,
            false,
            None,
            0,
        )
        .unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "reading plus two sub-values: {out}");
        assert!(lines[1].contains("Reference"), "got {}", lines[1]);
        // Empty aux unit falls back to the main reading's unit.
        assert!(lines[1].trim().ends_with("1.2340 V"), "got {}", lines[1]);
        // Overloaded sub-value reads OL, and carries its timestamp.
        assert!(lines[2].contains("OL mV @42s"), "got {}", lines[2]);
    }

    #[test]
    fn json_output_includes_aux_values() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_aux(&mut m);
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            &m,
            &WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
            0,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let aux = v["aux"].as_array().expect("aux array");
        assert_eq!(aux.len(), 2);
        assert_eq!(aux[0]["label"], "Reference");
        assert_eq!(aux[0]["unit"], "V");
        assert_eq!(aux[1]["value"], "OL");
        assert_eq!(aux[1]["elapsed_secs"], 42);
    }

    /// Families that report no sub-values must produce byte-identical output
    /// to before.
    #[test]
    fn no_aux_means_no_change_to_either_format() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            &m,
            &WallClock::new(),
            &OutputFormat::Text,
            false,
            None,
            0,
        )
        .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap().lines().count(), 1);
        assert!(json_for(StatusFlags::default()).get("aux").is_none());
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
            0,
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

    fn csv_with(m: &Measurement, integral: Option<(f64, &str)>, aux_slots: usize) -> String {
        let mut buf = Vec::new();
        format_measurement(
            &mut buf,
            m,
            &WallClock::new(),
            &OutputFormat::Csv,
            false,
            integral,
            aux_slots,
        )
        .unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn csv_fields(line: &str) -> Vec<String> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        rdr.records()
            .next()
            .unwrap()
            .unwrap()
            .iter()
            .map(str::to_string)
            .collect()
    }

    /// A UT181A frequency/period pair: two sub-values with units of their own.
    fn with_freq_aux(m: &mut Measurement) {
        use dmm_lib::measurement::AuxValue;
        m.aux_values = vec![
            AuxValue {
                label: "Frequency".into(),
                value: MeasuredValue::Normal(50.01),
                unit: "Hz".into(),
                display_raw: Some("50.01".to_string()),
                elapsed_secs: None,
            },
            AuxValue {
                label: "Period".into(),
                value: MeasuredValue::Normal(20.0),
                unit: "ms".into(),
                display_raw: Some("20.00".to_string()),
                elapsed_secs: None,
            },
        ];
    }

    #[test]
    fn csv_header_names_one_group_per_aux_slot() {
        assert_eq!(
            csv_header(false, 0),
            "timestamp,mode,value,unit,range,flags"
        );
        assert_eq!(
            csv_header(true, 0),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit"
        );
        assert_eq!(
            csv_header(false, 4),
            "timestamp,mode,value,unit,range,flags,\
             aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,\
             aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit"
        );
        // Integral columns first, so existing --integrate consumers keep
        // their column positions.
        assert_eq!(
            csv_header(true, 1),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit,\
             aux1_label,aux1_value,aux1_unit"
        );
    }

    /// The header is written once at the top of the file and the rows by a
    /// different function; a mismatch would silently misalign every column.
    #[test]
    fn csv_header_and_row_have_the_same_field_count() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        for integrate in [false, true] {
            for slots in [0usize, 1, 4] {
                let integral = integrate.then_some((1.5, "Vs"));
                let row = csv_fields(&csv_with(&m, integral, slots));
                let header = csv_fields(&csv_header(integrate, slots));
                assert_eq!(
                    header.len(),
                    row.len(),
                    "integrate={integrate} slots={slots}"
                );
            }
        }
    }

    /// The column layout is fixed per family, not per mode, so a reading that
    /// fills fewer slots than the family can report leaves the rest empty.
    #[test]
    fn csv_pads_unused_aux_slots() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        let line = csv_with(&m, None, 4);
        assert!(
            line.trim_end()
                .ends_with(",Frequency,50.01,Hz,Period,20.00,ms,,,,,,"),
            "got {line}"
        );
        assert_eq!(csv_fields(&line).len(), 6 + 4 * 3);
    }

    #[test]
    fn csv_aux_falls_back_to_the_main_unit_and_reports_overload() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_aux(&mut m);
        let fields = csv_fields(&csv_with(&m, None, 2));
        // Empty aux unit means "same as the main reading".
        assert_eq!(&fields[6..9], ["Reference", "1.2340", "V"]);
        // Overloaded sub-value exports OL, like the main value does.
        assert_eq!(&fields[9..12], ["Max", "OL", "mV"]);
    }

    /// A family with no sub-values must produce exactly the columns it did
    /// before aux export existed.
    #[test]
    fn csv_zero_slots_adds_no_columns() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        assert_eq!(csv_fields(&csv_with(&m, None, 0)).len(), 6);
    }

    #[test]
    fn csv_integral_columns_come_before_the_aux_columns() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        let fields = csv_fields(&csv_with(&m, Some((1.5, "Vs")), 2));
        assert_eq!(&fields[6..8], ["1.500000", "Vs"]);
        assert_eq!(&fields[8..11], ["Frequency", "50.01", "Hz"]);
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
            0,
        )
        .unwrap();
        assert!(String::from_utf8(buf).unwrap().contains("VOID"));
        assert_eq!(json_for(flags)["flags"]["void"], serde_json::json!(true));
    }
}
