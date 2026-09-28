use chrono::{DateTime, Local};
use clap::ValueEnum;
use dmm_lib::measurement::Measurement;
use dmm_lib::transport::Link;
use dmm_shared::export::CsvLayout;
use std::io::Write;
use std::time::Instant;

/// The reading's wall time as RFC3339: when it was taken, not when the
/// formatter ran.
fn timestamp_rfc3339(m: &Measurement) -> String {
    let dt: DateTime<Local> = m.wall_time.into();
    dt.to_rfc3339()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Csv,
    Json,
    /// The meter's own frames, for --replay to play back
    Replay,
}

impl OutputFormat {
    /// What `--format` calls this format, for a message that quotes the flag
    /// back at the user.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Replay => "replay",
        }
    }

    /// The extension a file of this format carries: what a bare `-o` names its
    /// file with, and what picks the format when `-o` names a file and
    /// `--format` doesn't.
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Replay => "replay",
        }
    }

    /// The format a file extension names, if it names one.
    pub(crate) fn from_extension(extension: &str) -> Option<Self> {
        [Self::Text, Self::Csv, Self::Json, Self::Replay]
            .into_iter()
            .find(|f| f.extension().eq_ignore_ascii_case(extension))
    }
}

/// What a `read` run writes, with whatever that format needs for the whole
/// run: [`OutputFormat`] is what the user asked for, this is what the loop
/// writes through.
///
/// Three of the four render the reading; `Replay` writes the frame the meter
/// sent instead, so a recording can be played back through the family's own
/// parser.
pub enum Output {
    Text,
    /// The column layout is fixed for the run, so a reading that fills fewer
    /// sub-value slots than the meter can send leaves the rest empty rather
    /// than shortening its row.
    Csv(CsvLayout),
    /// `experimental` marks readings decoded by a protocol no report has
    /// confirmed, so a script can tell them apart.
    Json {
        experimental: bool,
    },
    /// The meter's own frames, under the header naming the meter they came
    /// from — only the caller knows which meter that is. The header goes out
    /// with the first frame, dated from it.
    Replay {
        header: ReplayHeader,
        /// When the first frame arrived. Offsets are measured from it, so a
        /// recording starts at zero however long the meter took to answer.
        first: Option<Instant>,
    },
}

/// What a replay file's header names besides when it was recorded, which is
/// the first frame's wall time, known once it arrives: the offsets count from
/// that frame, so the header and the offsets agree however long the meter
/// took to answer, and a copy of a replay keeps the times it was measured at.
pub struct ReplayHeader {
    pub device: String,
    pub model: Option<String>,
    pub link: Option<Link>,
}

impl Output {
    /// The output for `format`, sized to the meter that is about to answer.
    ///
    /// `replay_header` is only called for `--format replay`: on a UT61+ the
    /// name it carries costs a command the meter answers with a beep.
    pub fn new(
        format: OutputFormat,
        layout: CsvLayout,
        experimental: bool,
        replay_header: impl FnOnce() -> ReplayHeader,
    ) -> Self {
        match format {
            OutputFormat::Text => Self::Text,
            OutputFormat::Csv => Self::Csv(layout),
            OutputFormat::Json => Self::Json { experimental },
            OutputFormat::Replay => Self::Replay {
                header: replay_header(),
                first: None,
            },
        }
    }

    /// What opens the file, for the formats with a header known up front.
    /// `model_name` is the meter's, as the CSV comment and the JSON metadata
    /// name it. A replay's header goes out with its first frame.
    pub fn header(&self, model_name: &str) -> Option<String> {
        match self {
            Self::Text => None,
            Self::Csv(layout) => Some(format!(
                "{}\n{}\n",
                dmm_shared::export::device_comment(model_name),
                layout.header().join(","),
            )),
            Self::Json { .. } => Some(format!(
                "{}\n",
                dmm_shared::export::metadata_line(model_name)
            )),
            Self::Replay { .. } => None,
        }
    }

    /// Write one reading.
    pub fn write(
        &mut self,
        w: &mut dyn Write,
        m: &Measurement,
        integral: Option<(f64, &str)>,
    ) -> std::io::Result<()> {
        match self {
            Self::Text => format_text(w, m, integral),
            Self::Csv(layout) => format_csv(w, m, integral, *layout),
            Self::Json { experimental } => format_json(w, m, *experimental, integral),
            Self::Replay { header, first } => {
                // The payload as the meter sent it: a `--scale` is a choice
                // the run that plays the file back makes for itself, and this
                // is one of the reasons it is refused alongside this format.
                //
                // Through `dmm_lib::replay`, which also parses these lines, so
                // the two ends of a recording cannot drift apart.
                if m.raw_payload.is_empty() {
                    // A reading with no frame behind it would be written as a
                    // bare offset, which `Replay::parse` refuses — the same
                    // reading the GUI's replay export refuses to render.
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("a {} reading carries no meter frame to record", m.mode),
                    ));
                }
                if first.is_none() {
                    let recorded: DateTime<Local> = m.wall_time.into();
                    let recorded = recorded.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
                    w.write_all(
                        dmm_lib::replay::header(
                            &header.device,
                            &recorded,
                            header.model.as_deref(),
                            header.link,
                        )
                        .as_bytes(),
                    )?;
                }
                let first = *first.get_or_insert(m.timestamp);
                let offset = m
                    .timestamp
                    .checked_duration_since(first)
                    .unwrap_or_default();
                w.write_all(dmm_lib::replay::sample_line(offset, &m.raw_payload).as_bytes())
            }
        }
    }
}

fn format_text(
    w: &mut dyn Write,
    m: &Measurement,
    integral: Option<(f64, &str)>,
) -> std::io::Result<()> {
    if let Some((val, unit)) = integral {
        writeln!(w, "{m} [\u{222b} {val:.4} {unit}]")?;
    } else {
        writeln!(w, "{m}")?;
    }
    // Sub-values, indented under the reading they belong to. The UT181A
    // produces these in REL (Reference/Absolute), MIN/MAX (Max/Average/Min
    // with timestamps) and peak modes, and the UT171 for the AC frequency
    // aux; before this they were parsed and discarded. A frame without a main
    // reading already printed its sub-values in the value's place.
    if !m.has_main_reading() {
        return Ok(());
    }
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

fn format_csv(
    w: &mut dyn Write,
    m: &Measurement,
    integral: Option<(f64, &str)>,
    layout: CsvLayout,
) -> std::io::Result<()> {
    // Through the csv crate rather than hand-joined with commas, as the GUI
    // export already does. Several of these fields carry device-derived text:
    // UT181A units come from `parse_unit_string`, which maps raw frame bytes
    // to chars with no character-set validation, and an unrecognised mode byte
    // becomes `Unknown(0x..)`. One comma or quote in there and every
    // downstream column shifts.
    let ts = timestamp_rfc3339(m);
    // Cells resolved ahead of the writer so the borrowed ones outlive the
    // record. `--scale` is fixed for the run, so every row carries the full
    // extra count the layout reserves.
    // The CLI places no markers.
    let cells = layout.row(m, &ts, integral, layout.extra_slots, None);
    let mut wtr = csv::WriterBuilder::new()
        // One row per call, so the default 8 KiB buffer is dead weight — a row
        // is well under this.
        .buffer_capacity(256)
        .from_writer(w);
    wtr.write_record(cells.iter().map(|c| c.as_ref()))
        .map_err(std::io::Error::other)?;
    wtr.flush()
}

/// One reading as a JSON object, through the writer the GUI's JSON export
/// also calls — the two cannot drift, because there is only one of them.
///
/// Rendered whole, then written once: stdout's line buffering would otherwise
/// flush the object and its newline separately.
fn format_json(
    w: &mut dyn Write,
    m: &Measurement,
    experimental: bool,
    integral: Option<(f64, &str)>,
) -> std::io::Result<()> {
    let mut line = Vec::with_capacity(512);
    dmm_shared::export::write_measurement_json(
        &mut line,
        m,
        &timestamp_rfc3339(m),
        experimental,
        integral,
        None,
    )?;
    line.push(b'\n');
    w.write_all(&line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::{AuxValue, MeasuredValue};
    use dmm_lib::protocol::make_test_measurement;

    /// One reading, as `output` writes it.
    fn rendered(mut output: Output, m: &Measurement, integral: Option<(f64, &str)>) -> String {
        let mut buf = Vec::new();
        output.write(&mut buf, m, integral).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn text_for(m: &Measurement) -> String {
        rendered(Output::Text, m, None)
    }

    fn json_for(flags: StatusFlags) -> serde_json::Value {
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", flags);
        serde_json::from_str(&rendered(
            Output::Json {
                experimental: false,
            },
            &m,
            None,
        ))
        .unwrap()
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

    /// A word the meter shows instead of a reading prints as the word, never
    /// as OL, and leaves the CSV value cell empty.
    #[test]
    fn a_no_reading_prints_its_word() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::NoReading("----"),
            "A",
            StatusFlags::default(),
        );
        m.mode = "AC A".into();
        assert_eq!(text_for(&m), "---- A\n");
        let csv = rendered(Output::Csv(CsvLayout::default()), &m, None);
        let cells: Vec<&str> = csv.trim_end().split(',').collect();
        assert_eq!(&cells[1..4], ["AC A", "", "A"]);
    }

    /// A frame carrying only the AC component of an AC+DC reading prints the
    /// component where the value goes, once — not again as an indented
    /// sub-value line.
    #[test]
    fn a_frame_without_a_main_reading_prints_one_line() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Absent,
            "V",
            StatusFlags {
                auto_range: true,
                ..Default::default()
            },
        );
        m.mode = "AC+DC V".into();
        m.display_raw = None;
        m.aux_values = vec![AuxValue {
            label: "AC".into(),
            value: MeasuredValue::Normal(0.0),
            unit: "".into(),
            display_raw: Some(" 0.0000".to_string()),
            elapsed_secs: None,
        }];
        assert_eq!(text_for(&m), "AC 0.0000 V [AUTO]\n");
    }

    /// UT181A REL/MIN-MAX sub-values were parsed and then discarded by every
    /// output format.
    #[test]
    fn text_output_lists_aux_values() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_aux(&mut m);
        let out = text_for(&m);
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
        let v: serde_json::Value = serde_json::from_str(&rendered(
            Output::Json {
                experimental: false,
            },
            &m,
            None,
        ))
        .unwrap();
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
        assert_eq!(text_for(&m).lines().count(), 1);
        assert!(json_for(StatusFlags::default()).get("aux").is_none());
    }

    fn csv_for(m: &Measurement) -> String {
        rendered(Output::Csv(CsvLayout::default()), m, None)
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

    fn layout_of(integral: bool, family: usize, extra: usize) -> CsvLayout {
        CsvLayout {
            family_slots: family,
            extra_slots: extra,
            integral,
            markers: false,
        }
    }

    fn csv_with(
        m: &Measurement,
        integral: Option<(f64, &str)>,
        family: usize,
        extra: usize,
    ) -> String {
        rendered(
            Output::Csv(layout_of(integral.is_some(), family, extra)),
            m,
            integral,
        )
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

    /// The header `run_read_loop` writes at the top of the file, for the
    /// layouts the CLI's own options produce.
    #[test]
    fn csv_header_names_one_group_per_aux_slot() {
        assert_eq!(
            layout_of(false, 0, 0).header().join(","),
            "timestamp,mode,value,unit,range,flags"
        );
        assert_eq!(
            layout_of(true, 0, 0).header().join(","),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit"
        );
        assert_eq!(
            layout_of(false, 4, 0).header().join(","),
            "timestamp,mode,value,unit,range,flags,\
             aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,\
             aux3_label,aux3_value,aux3_unit,aux4_label,aux4_value,aux4_unit"
        );
        // Integral columns first, so existing --integrate consumers keep
        // their column positions.
        assert_eq!(
            layout_of(true, 0, 1).header().join(","),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit,\
             aux1_label,aux1_value,aux1_unit"
        );
    }

    /// The header is written once at the top of the file and the rows one at a
    /// time; a mismatch would silently misalign every column.
    #[test]
    fn csv_header_and_row_have_the_same_field_count() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        for integrate in [false, true] {
            for slots in [0usize, 1, 4] {
                let integral = integrate.then_some((1.5, "Vs"));
                let row = csv_fields(&csv_with(&m, integral, slots, 0));
                let header = layout_of(integrate, slots, 0).header();
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
        let line = csv_with(&m, None, 4, 0);
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
        let fields = csv_fields(&csv_with(&m, None, 2, 0));
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
        assert_eq!(csv_fields(&csv_with(&m, None, 0, 0)).len(), 6);
    }

    #[test]
    fn csv_integral_columns_come_before_the_aux_columns() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        with_freq_aux(&mut m);
        let fields = csv_fields(&csv_with(&m, Some((1.5, "Vs")), 2, 0));
        assert_eq!(&fields[6..8], ["1.500000", "Vs"]);
        assert_eq!(&fields[8..11], ["Frequency", "50.01", "Hz"]);
    }

    /// A reading a software `--scale` has already been applied to: the main
    /// value is in the relabelled unit and the meter's own reading trails as
    /// the `Raw` sub-value.
    fn scaled_reading() -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(123.4), "mV", StatusFlags::default());
        m.display_raw = Some("123.4".to_string());
        m
    }

    /// `run_read_loop` sizes the run's slots as family slots + 1 when a
    /// transform is on. Header and row must still agree, and the meter's own
    /// sub-values must keep the columns they had.
    #[test]
    fn csv_row_with_a_transform_keeps_the_header_field_count() {
        use dmm_lib::transform::Transform;
        let mut m = scaled_reading();
        with_freq_aux(&mut m); // two sub-values from the meter itself
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        t.apply(&mut m);
        let extra = t.extra_aux_count();

        let row = csv_fields(&csv_with(&m, None, 2, extra));
        assert_eq!(layout_of(false, 2, extra).header().len(), row.len());
        assert_eq!(&row[2..4], ["12.34", "A"]);
        assert_eq!(
            &row[6..12],
            ["Frequency", "50.01", "Hz", "Period", "20.00", "ms"]
        );
        // Raw takes the trailing group reserved for it.
        assert_eq!(&row[12..15], ["Raw", "123.4", "mV"]);
    }

    /// The same run, one frame later, with the dial on a mode the meter sends
    /// no sub-values for: `Raw` must stay in the third group rather than
    /// sliding into `aux1_*` and mixing millivolts into the frequency column.
    #[test]
    fn csv_raw_holds_its_group_when_the_meter_sends_no_sub_values() {
        use dmm_lib::transform::Transform;
        let mut m = scaled_reading();
        let t = Transform::linear(100.0, 0.0, Some("A".to_string()));
        t.apply(&mut m);
        let extra = t.extra_aux_count();

        let row = csv_fields(&csv_with(&m, None, 2, extra));
        assert_eq!(layout_of(false, 2, extra).header().len(), row.len());
        // The meter's two groups stay empty...
        assert_eq!(&row[6..12], ["", "", "", "", "", ""]);
        // ...and Raw keeps the same columns it had on the frame above.
        assert_eq!(&row[12..15], ["Raw", "123.4", "mV"]);
    }

    #[test]
    fn text_output_lists_the_raw_sub_value_under_a_scaled_reading() {
        use dmm_lib::transform::Transform;
        let mut m = scaled_reading();
        Transform::linear(100.0, 0.0, Some("A".to_string())).apply(&mut m);
        let out = text_for(&m);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "reading plus the Raw sub-value: {out}");
        assert!(lines[0].starts_with("12.34 A"), "got {}", lines[0]);
        assert_eq!(lines[1], "  Raw  123.4 mV");
    }

    /// Text output already carried these; the three formats must agree.
    #[test]
    fn text_and_json_agree_on_void() {
        let flags = StatusFlags {
            void: true,
            ..Default::default()
        };
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", flags);
        assert!(text_for(&m).contains("VOID"));
        assert_eq!(json_for(flags)["flags"]["void"], serde_json::json!(true));
    }

    /// The one check that writer and parser agree, short of a meter: a bench
    /// recording has to come back as the session it was.
    #[test]
    fn a_replay_run_writes_a_file_that_parses_back_as_a_replay() {
        use dmm_lib::protocol::make_test_measurement;
        use dmm_lib::replay::Replay;
        use std::time::Duration;

        // The `dcv_battery` golden frame, 1.6109 V on the 2.2V range.
        let mut m = make_test_measurement(0x02, 0x30, b" 1.6109", (0x03, 0x02), (0x30, 0x30, 0x30));
        let first = m.timestamp;
        let recorded = std::time::SystemTime::UNIX_EPOCH + Duration::from_millis(1_790_000_000_123);
        m.wall_time = recorded;
        let mut output = Output::new(OutputFormat::Replay, CsvLayout::default(), false, || {
            ReplayHeader {
                device: "ut61eplus".to_string(),
                model: Some("UT61E+".to_string()),
                link: Some(Link::Bluetooth),
            }
        });
        // Nothing up front: the header is dated from the first frame.
        assert!(output.header("UNI-T UT61E+").is_none());

        let mut file = Vec::new();
        output.write(&mut file, &m, None).unwrap();
        m.timestamp = first + Duration::from_millis(250);
        output.write(&mut file, &m, None).unwrap();

        let text = String::from_utf8(file).expect("a replay file is UTF-8");
        let replay = Replay::parse(&text).expect("parses as a replay");
        assert_eq!(replay.device.id, "ut61eplus");
        assert_eq!(replay.model.as_deref(), Some("UT61E+"));
        // The link the run was on reaches the file and comes back.
        assert_eq!(replay.link, Some(dmm_lib::transport::Link::Bluetooth));
        // Offsets run from the first frame, not from wherever the session was.
        assert_eq!(replay.duration(), Duration::from_millis(250));
        // And the header is that frame's wall time.
        let dated = chrono::DateTime::parse_from_rfc3339(&replay.recorded).unwrap();
        assert_eq!(std::time::SystemTime::from(dated), recorded);
    }

    /// A reading with no frame behind it would go out as a bare offset, which
    /// `Replay::parse` refuses. The families that can be recorded all carry
    /// their payload, so this is the writer refusing to produce a file its own
    /// parser would reject — the check the GUI's replay export already makes.
    #[test]
    fn a_reading_without_a_frame_is_not_written_as_a_replay() {
        // The fixture carries no payload, which is exactly the case: a
        // reading that reached the writer without the frame it was decoded
        // from.
        let m = Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let mut output = Output::new(OutputFormat::Replay, CsvLayout::default(), false, || {
            ReplayHeader {
                device: "ut61eplus".to_string(),
                model: None,
                link: None,
            }
        });

        let mut file = Vec::new();
        let e = output
            .write(&mut file, &m, None)
            .expect_err("a frameless reading has nothing to record");
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
        assert!(e.to_string().contains("DC V"), "got {e}");
        assert!(file.is_empty(), "nothing is written: {file:?}");
    }

    fn csv_of(m: &dmm_lib::measurement::Measurement) -> String {
        rendered(Output::Csv(CsvLayout::default()), m, None)
    }

    fn json_of(m: &dmm_lib::measurement::Measurement, experimental: bool) -> serde_json::Value {
        serde_json::from_str(&rendered(Output::Json { experimental }, m, None)).unwrap()
    }

    #[test]
    fn format_text_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = rendered(Output::Text, &m, None);
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = csv_of(&m);
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert!(fields.len() >= 6);
        assert_eq!(fields[1], "DC V");
        assert_eq!(fields[2], "5.678");
        assert_eq!(fields[3], "V");
    }

    /// A meter that can report sub-values gets one column group per slot,
    /// sized by the family's `max_aux_values` so every row of a file lines up
    /// even when a mode reports fewer than the family can.
    #[test]
    fn format_csv_with_aux_slots() {
        use dmm_lib::measurement::AuxValue;

        let mut m = make_test_measurement(0x02, 0x01, b"239.22 ", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.aux_values = vec![AuxValue {
            label: "Frequency".into(),
            value: MeasuredValue::Normal(50.01),
            unit: "Hz".into(),
            display_raw: Some("50.01".to_string()),
            elapsed_secs: None,
        }];
        let layout = dmm_shared::export::CsvLayout {
            family_slots: 2,
            ..Default::default()
        };
        let output = rendered(Output::Csv(layout), &m, None);
        let fields: Vec<&str> = output.trim_end().split(',').collect();
        assert_eq!(fields.len(), 6 + 2 * 3, "got {output}");
        assert_eq!(&fields[6..9], ["Frequency", "50.01", "Hz"]);
        // The unused second slot is present but empty.
        assert_eq!(&fields[9..12], ["", "", ""]);
        assert_eq!(layout.header().len(), fields.len());
    }

    /// The UT61E+ separates the sign from the digits on some ranges. That
    /// space must not reach the CSV, or the whole column parses as text.
    #[test]
    fn format_csv_negative_value_is_numeric() {
        let m = make_test_measurement(0x02, 0x01, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = csv_of(&m);
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert_eq!(fields[2], "-55.79");
        assert_eq!(fields[2].parse::<f64>().unwrap(), -55.79);
    }

    #[test]
    fn format_json_output() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on, inverted logic)
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert_eq!(parsed["mode"], "DC V");
        assert_eq!(parsed["value"], 5.678);
        assert_eq!(parsed["unit"], "V");
        assert_eq!(parsed["flags"]["hold"], true);
        assert_eq!(parsed["flags"]["auto_range"], true);
        assert_eq!(parsed["experimental"], false);
    }

    #[test]
    fn format_json_experimental_flag() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(json_of(&m, true)["experimental"], true);
    }

    #[test]
    fn format_csv_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert!(csv_of(&m).contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(json_of(&m, false)["value"], "OL");
    }

    #[test]
    fn format_csv_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert!(csv_of(&m).contains("NCV:3"));
    }

    #[test]
    fn format_json_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert_eq!(parsed["value"]["ncv_level"], 3);
        assert_eq!(parsed["mode"], "NCV");
    }

    #[test]
    fn format_text_includes_flags() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x00, 0x00));
        let output = rendered(Output::Text, &m, None);
        assert!(output.contains("HOLD"));
        assert!(output.contains("REL"));
    }

    #[test]
    fn format_json_negative_value() {
        let m = make_test_measurement(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert!((parsed["value"].as_f64().unwrap() - (-12.345)).abs() < 1e-6);
    }

    /// Every format names the file it writes, and every one of those names
    /// picks it back out of an `-o` file name.
    #[test]
    fn a_format_and_its_file_extension_name_each_other() {
        for format in [
            OutputFormat::Text,
            OutputFormat::Csv,
            OutputFormat::Json,
            OutputFormat::Replay,
        ] {
            assert_eq!(
                OutputFormat::from_extension(format.extension()),
                Some(format),
                "{}",
                format.name()
            );
            // The name a message quotes back is the one `--format` takes.
            assert_eq!(
                format
                    .to_possible_value()
                    .expect("a --format value")
                    .get_name(),
                format.name()
            );
        }
        assert_eq!(OutputFormat::from_extension("dat"), None);
    }
}
