//! Reading an export back: the other half of this module's writers, kept
//! beside them so a column or key cannot change on one side only.
//!
//! What comes back is what the file holds. An export carries the reading as
//! the meter showed it, not the frame it came in, so a reading read back has
//! no spec, no mode or range code and no wire bytes; a word the meter showed
//! in place of a value, which exports as an empty cell, comes back as a blank
//! word. CSV also has no `progress`, `experimental` or DC flag, and writes a
//! sub-value's unit even where the meter left it to the main reading's.

use chrono::DateTime;
use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{BufRead, Read};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

/// An export, read back.
#[derive(Debug, Clone)]
pub struct Imported {
    /// The meter the file names: its `# device:` line or `_metadata`.
    pub device: Option<String>,
    /// The graph view the file was saved with, as the JSON text it was
    /// written as. Its fields are the GUI's business.
    pub view: Option<String>,
    /// The readings, in file order.
    pub readings: Vec<ImportedReading>,
    /// The markers, in file order.
    pub markers: Vec<ImportedMarker>,
    /// Sub-value groups the file lays out: a CSV's `auxN_*` column groups,
    /// empty ones included, or the most any JSON reading carries — what a
    /// CSV written from it keeps.
    pub aux_slots: usize,
    /// Readings past the limit the file was read with, counted but not kept.
    pub left_out: usize,
}

/// Which kind of export a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Csv,
    Json,
    Replay,
}

impl ExportKind {
    /// The kind `path`'s extension names, or for a file without a known one,
    /// the kind its first bytes `head` say: a replay's magic line, a JSON
    /// object, else CSV. Both binaries import by this, so they read the same
    /// files.
    pub fn detect(path: &Path, head: &str) -> Self {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("csv") => return Self::Csv,
            Some("json") => return Self::Json,
            Some("replay") => return Self::Replay,
            _ => {}
        }
        let head = head.trim_start_matches('\u{feff}').trim_start();
        if head.starts_with("# dmm-replay") {
            Self::Replay
        } else if head.starts_with('{') {
            Self::Json
        } else {
            Self::Csv
        }
    }

    /// The kind of the file at `path`, from its name and first bytes.
    pub fn of_file(path: &Path) -> std::io::Result<Self> {
        let mut head = Vec::with_capacity(256);
        std::fs::File::open(path)?
            .take(256)
            .read_to_end(&mut head)?;
        Ok(Self::detect(path, &String::from_utf8_lossy(&head)))
    }
}

impl Imported {
    /// How long after the first reading each was taken, never going back —
    /// a clock set back mid-file must not send a session's time backwards.
    pub fn offsets(&self) -> Vec<Duration> {
        let first = self.readings.first().map(|r| r.wall_time);
        let mut last = Duration::ZERO;
        self.readings
            .iter()
            .map(|r| {
                let offset = first
                    .and_then(|first| r.wall_time.duration_since(first).ok())
                    .unwrap_or_default()
                    .max(last);
                last = offset;
                offset
            })
            .collect()
    }
}

/// The median gap between consecutive `offsets`, a second when there is
/// none to measure: the spacing a file's readings came at, which a single
/// slow stretch doesn't move.
pub fn median_spacing(offsets: &[Duration]) -> Duration {
    let mut gaps: Vec<Duration> = offsets
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .filter(|g| !g.is_zero())
        .collect();
    if gaps.is_empty() {
        return Duration::from_secs(1);
    }
    let mid = gaps.len() / 2;
    *gaps.select_nth_unstable(mid).1
}

/// `text` with every control character written as its escape: file text
/// headed for a terminal or a single line, where a raw one would act.
pub fn escape_controls(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_control) {
        Cow::Owned(
            text.chars()
                .map(|c| {
                    if c.is_control() {
                        c.escape_debug().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect(),
        )
    } else {
        Cow::Borrowed(text)
    }
}

/// A marker an export carried on one of its readings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedMarker {
    /// Index into [`Imported::readings`].
    pub reading: usize,
    pub number: u32,
    pub note: String,
}

/// One reading as an export holds it.
#[derive(Debug, Clone)]
pub struct ImportedReading {
    /// When it was taken.
    pub wall_time: SystemTime,
    pub mode: String,
    pub value: MeasuredValue,
    pub unit: String,
    pub range: String,
    /// The digits as the meter showed them, which re-export writes back.
    pub display_raw: Option<String>,
    pub progress: Option<u16>,
    /// Decoded by a protocol no report has confirmed (JSON only).
    pub experimental: bool,
    pub flags: StatusFlags,
    pub aux: Vec<ImportedAux>,
}

/// One sub-value as an export holds it.
#[derive(Debug, Clone)]
pub struct ImportedAux {
    pub label: String,
    pub value: MeasuredValue,
    pub unit: String,
    pub display_raw: Option<String>,
    pub elapsed_secs: Option<u32>,
}

impl ImportedReading {
    /// The reading as a [`Measurement`] taken at session instant `timestamp`,
    /// with nothing the file lacks: no mode or range code, no spec, no frame.
    pub fn into_measurement(self, timestamp: Instant) -> Measurement {
        Measurement {
            timestamp,
            wall_time: self.wall_time,
            mode: Cow::Owned(self.mode),
            mode_raw: 0,
            range_raw: 0,
            value: self.value,
            unit: Cow::Owned(self.unit),
            range_label: Cow::Owned(self.range),
            progress: self.progress,
            display_raw: self.display_raw,
            flags: self.flags,
            aux_values: self
                .aux
                .into_iter()
                .map(|aux| AuxValue {
                    label: Cow::Owned(aux.label),
                    value: aux.value,
                    unit: Cow::Owned(aux.unit),
                    display_raw: aux.display_raw,
                    elapsed_secs: aux.elapsed_secs,
                })
                .collect(),
            main_label: None,
            raw_payload: Vec::new(),
            spec: None,
            mode_spec: None,
        }
    }
}

/// Why a file would not read, with the line it stopped at.
fn at(line: usize, message: impl std::fmt::Display) -> String {
    // Messages quote the file, and are printed.
    format!("line {line}: {}", escape_controls(&message.to_string()))
}

/// A meter's name from a file, on one line whatever the file put in it.
fn device_name(name: &str) -> Option<String> {
    let name: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    Some(name.trim().to_string()).filter(|n| !n.is_empty())
}

/// A timestamp cell or key: RFC 3339, as both binaries write it.
fn wall_time(line: usize, text: &str) -> Result<SystemTime, String> {
    DateTime::parse_from_rfc3339(text)
        .map(SystemTime::from)
        .map_err(|e| at(line, format!("`{text}` is not an RFC 3339 time ({e})")))
}

/// A value as a CSV cell or a JSON sub-value string writes it: the meter's
/// digits, `OL`, `NCV:n`, or empty. Empty is a word the meter showed or no
/// value at all, which the caller tells apart. The digits are also returned
/// as the display text, which re-export writes back unchanged.
fn value_text(line: usize, text: &str) -> Result<(Option<MeasuredValue>, Option<String>), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok((None, None));
    }
    if text == "OL" {
        return Ok((Some(MeasuredValue::Overload), None));
    }
    if let Some(level) = text.strip_prefix("NCV:") {
        let level = level
            .parse()
            .map_err(|_| at(line, format!("`{text}` is not an NCV level")))?;
        return Ok((Some(MeasuredValue::NcvLevel(level)), None));
    }
    let number: f64 = text
        .parse()
        .map_err(|_| at(line, format!("`{text}` is not a value")))?;
    Ok((Some(MeasuredValue::Normal(number)), Some(text.to_string())))
}

/// No value: a blank word in place of a reading, which breaks the trace as
/// the meter's word did, unless the reading has sub-values — a frame that
/// carried only its sub-values, which leaves the trace alone.
fn no_value(has_aux: bool) -> MeasuredValue {
    if has_aux {
        MeasuredValue::Absent
    } else {
        MeasuredValue::NoReading("")
    }
}

/// A CSV flags cell: the labels `StatusFlags` prints, space-separated, in
/// [`Flag::ALL`] order — which is also the order to read them in, so a
/// two-word label ("LOW BAT") and one that ends another ("MIN", "P-MIN")
/// cannot be misread.
fn flags_cell(line: usize, cell: &str) -> Result<StatusFlags, String> {
    let mut flags = StatusFlags::default();
    let mut rest = cell.trim();
    for flag in Flag::ALL {
        let Some(label) = flag.label() else { continue };
        if let Some(after) = rest.strip_prefix(label)
            && (after.is_empty() || after.starts_with(' '))
        {
            flags.set(flag, true);
            rest = after.trim_start();
        }
    }
    if rest.is_empty() {
        Ok(flags)
    } else {
        Err(at(line, format!("`{rest}` is not a flag")))
    }
}

/// Read a CSV export: `#` lines, the header, then one row per reading.
///
/// Columns are found by name, so a file with integral or marker columns, or
/// any number of sub-value groups, reads the same way. The integral is left
/// out: it is worked out again from the readings by whatever reads them.
pub fn read_csv(text: &str) -> Result<Imported, String> {
    read_csv_from(text.as_bytes(), None)
}

/// [`read_csv`], from a stream, keeping at most `limit` readings and
/// counting the rest in [`Imported::left_out`]: a file far larger than the
/// session can hold is read through without being held.
pub fn read_csv_from(mut input: impl BufRead, limit: Option<usize>) -> Result<Imported, String> {
    let mut device = None;
    let mut first_row_line = 0;
    let mut header = String::new();
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        first_row_line += 1;
        if first_row_line == 1 {
            // A spreadsheet saving as UTF-8 puts a byte-order mark first.
            line = line.trim_start_matches('\u{feff}').to_string();
        }
        let Some(comment) = line.trim_end().strip_prefix('#') else {
            header = line;
            break;
        };
        if let Some(model) = comment.trim().strip_prefix("device:") {
            device = device_name(model);
        }
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(std::io::Cursor::new(header.into_bytes()).chain(input));
    let header = reader
        .headers()
        .map_err(|e| at(first_row_line, format!("no header row ({e})")))?
        .clone();
    let column = |name: &str| header.iter().position(|h| h == name);
    let required =
        |name: &str| column(name).ok_or_else(|| at(first_row_line, format!("no `{name}` column")));
    let (timestamp, mode, value, unit, range, flags) = (
        required("timestamp")?,
        required("mode")?,
        required("value")?,
        required("unit")?,
        required("range")?,
        required("flags")?,
    );
    let marker = column("marker").zip(column("note"));
    // Every `auxN_label` with its value and unit, in N order.
    let mut aux_groups: Vec<(usize, usize, usize, usize)> = header
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let n: usize = h
                .strip_prefix("aux")?
                .strip_suffix("_label")?
                .parse()
                .ok()?;
            let value = column(&format!("aux{n}_value"))?;
            let unit = column(&format!("aux{n}_unit"))?;
            Some((n, i, value, unit))
        })
        .collect();
    aux_groups.sort_unstable();

    let mut imported = Imported {
        device,
        view: None,
        readings: Vec::new(),
        markers: Vec::new(),
        aux_slots: aux_groups.len(),
        left_out: 0,
    };
    for (i, row) in reader.records().enumerate() {
        let line = first_row_line + 1 + i;
        let row = row.map_err(|e| at(line, e))?;
        if limit.is_some_and(|limit| imported.readings.len() >= limit) {
            imported.left_out += 1;
            continue;
        }
        let cell = |i: usize| row.get(i).unwrap_or("");
        let mut aux = Vec::new();
        for &(_, label, value, unit) in &aux_groups {
            let (label, value_cell, unit) = (cell(label), cell(value), cell(unit));
            if label.is_empty() && value_cell.is_empty() && unit.is_empty() {
                continue; // a group this reading did not fill
            }
            let (value, display_raw) = value_text(line, value_cell)?;
            aux.push(ImportedAux {
                label: label.to_string(),
                value: value.unwrap_or(MeasuredValue::NoReading("")),
                unit: unit.to_string(),
                display_raw,
                elapsed_secs: None,
            });
        }
        let (main, display_raw) = value_text(line, cell(value))?;
        imported.readings.push(ImportedReading {
            wall_time: wall_time(line, cell(timestamp))?,
            mode: cell(mode).to_string(),
            value: main.unwrap_or_else(|| no_value(!aux.is_empty())),
            unit: cell(unit).to_string(),
            range: cell(range).to_string(),
            display_raw,
            progress: None,
            experimental: false,
            flags: flags_cell(line, cell(flags))?,
            aux,
        });
        if let Some((number, note)) = marker
            && !cell(number).is_empty()
        {
            imported.markers.push(ImportedMarker {
                reading: imported.readings.len() - 1,
                number: cell(number)
                    .parse()
                    .map_err(|_| at(line, format!("`{}` is not a marker number", cell(number))))?,
                note: cell(note).to_string(),
            });
        }
    }
    Ok(imported)
}

/// The first line of a JSON export.
#[derive(Deserialize)]
struct JsonMetadataLine {
    #[serde(rename = "_metadata")]
    metadata: JsonMetadata,
}

#[derive(Deserialize)]
struct JsonMetadata {
    #[serde(default)]
    device: Option<String>,
    /// Kept as written: its fields are the GUI's.
    #[serde(default)]
    view: Option<Box<serde_json::value::RawValue>>,
}

/// One reading line of a JSON export, as `write_measurement_json` writes it.
#[derive(Deserialize)]
struct JsonReadingIn {
    timestamp: String,
    mode: String,
    value: serde_json::Value,
    unit: String,
    range: String,
    #[serde(default)]
    display_raw: Option<String>,
    #[serde(default)]
    progress: Option<u16>,
    #[serde(default)]
    experimental: bool,
    #[serde(default)]
    flags: HashMap<String, bool>,
    #[serde(default)]
    aux: Vec<JsonAuxIn>,
    #[serde(default)]
    marker: Option<u32>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
struct JsonAuxIn {
    label: String,
    value: Option<String>,
    unit: String,
    #[serde(default)]
    elapsed_secs: Option<u32>,
}

/// Read a JSON export: the `_metadata` line, then one reading per line.
pub fn read_json(text: &str) -> Result<Imported, String> {
    read_json_from(text.as_bytes(), None)
}

/// [`read_json`], from a stream, keeping at most `limit` readings and
/// counting the rest in [`Imported::left_out`].
pub fn read_json_from(input: impl BufRead, limit: Option<usize>) -> Result<Imported, String> {
    let mut lines = input
        .lines()
        .enumerate()
        .map(|(i, l)| l.map(|l| (i, l)).map_err(|e| e.to_string()))
        .filter(|l| l.as_ref().map_or(true, |(_, l)| !l.trim().is_empty()));
    let (_, first) = lines
        .next()
        .ok_or_else(|| "the file is empty".to_string())??;
    let first = first.trim_start_matches('\u{feff}');
    let metadata: JsonMetadataLine =
        serde_json::from_str(first).map_err(|e| at(1, format!("no `_metadata` line ({e})")))?;
    let mut imported = Imported {
        device: metadata.metadata.device.as_deref().and_then(device_name),
        view: metadata.metadata.view.map(|v| v.get().to_string()),
        readings: Vec::new(),
        markers: Vec::new(),
        aux_slots: 0,
        left_out: 0,
    };
    for next in lines {
        let (i, text) = next?;
        let line = i + 1;
        if limit.is_some_and(|limit| imported.readings.len() >= limit) {
            imported.left_out += 1;
            continue;
        }
        let r: JsonReadingIn = serde_json::from_str(&text).map_err(|e| at(line, e))?;
        let mut aux = Vec::with_capacity(r.aux.len());
        for a in r.aux {
            let (value, display_raw) = value_text(line, a.value.as_deref().unwrap_or(""))?;
            aux.push(ImportedAux {
                label: a.label,
                value: value.unwrap_or(MeasuredValue::NoReading("")),
                unit: a.unit,
                display_raw,
                elapsed_secs: a.elapsed_secs,
            });
        }
        let value = match &r.value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(MeasuredValue::Normal)
                .ok_or_else(|| at(line, format!("`{n}` is not a value")))?,
            serde_json::Value::String(s) if s == "OL" => MeasuredValue::Overload,
            serde_json::Value::Object(o) => o
                .get("ncv_level")
                .and_then(|l| l.as_u64())
                .and_then(|l| u8::try_from(l).ok())
                .map(MeasuredValue::NcvLevel)
                .ok_or_else(|| at(line, format!("`{}` is not a value", r.value)))?,
            serde_json::Value::Null => no_value(!aux.is_empty()),
            other => return Err(at(line, format!("`{other}` is not a value"))),
        };
        let mut flags = StatusFlags::default();
        for flag in Flag::ALL {
            flags.set(flag, r.flags.get(flag.name()).copied().unwrap_or(false));
        }
        imported.readings.push(ImportedReading {
            wall_time: wall_time(line, &r.timestamp)?,
            mode: r.mode,
            value,
            unit: r.unit,
            range: r.range,
            display_raw: r.display_raw,
            progress: r.progress,
            experimental: r.experimental,
            flags,
            aux,
        });
        if let Some(number) = r.marker {
            imported.markers.push(ImportedMarker {
                reading: imported.readings.len() - 1,
                number,
                note: r.note.unwrap_or_default(),
            });
        }
    }
    imported.aux_slots = imported
        .readings
        .iter()
        .map(|r| r.aux.len())
        .max()
        .unwrap_or(0);
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{CsvLayout, device_comment, metadata_line, write_measurement_json};
    use chrono::{DateTime, Local};
    use std::path::Path;
    use std::time::Duration;

    /// Each reading's marker, as the writers take it.
    type Marks<'a> = Vec<Option<(u32, &'a str)>>;

    /// Readings of every shape an export holds, a second apart, with
    /// markers on the second and the last.
    fn shapes() -> (Vec<Measurement>, Marks<'static>) {
        let t0 = Instant::now();
        let wall0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_788_343_200);
        let mut flags = StatusFlags::default();
        flags.set(Flag::Hold, true);
        flags.set(Flag::LowBattery, true);
        flags.set(Flag::PeakMin, true);
        flags.set(Flag::LeadError, true);
        let aux = |label: &str, value, unit: &str, raw: Option<&str>| AuxValue {
            label: Cow::Owned(label.to_string()),
            value,
            unit: Cow::Owned(unit.to_string()),
            display_raw: raw.map(str::to_string),
            elapsed_secs: None,
        };
        let mut readings = vec![
            Measurement::test_fixture(MeasuredValue::Normal(1.6109), "V", StatusFlags::default()),
            Measurement::test_fixture(MeasuredValue::Overload, "V", flags),
            Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default()),
            Measurement::test_fixture(
                MeasuredValue::NoReading("Auto"),
                "A",
                StatusFlags::default(),
            ),
            Measurement::test_fixture(MeasuredValue::Absent, "V", StatusFlags::default()),
            Measurement::test_fixture(MeasuredValue::Normal(-0.5137), "V", StatusFlags::default()),
        ];
        readings[0].display_raw = Some(" 1.6109".to_string());
        readings[4].aux_values = vec![aux("AC", MeasuredValue::Normal(0.2), "", Some("0.2000"))];
        readings[5].display_raw = Some("-0.5137".to_string());
        readings[5].aux_values = vec![
            aux(
                "Frequency",
                MeasuredValue::Normal(50.01),
                "Hz",
                Some("50.01"),
            ),
            aux("Max", MeasuredValue::Overload, "", None),
        ];
        for (i, m) in readings.iter_mut().enumerate() {
            m.timestamp = t0 + Duration::from_secs(i as u64);
            m.wall_time = wall0 + Duration::from_secs(i as u64);
        }
        let mut marks = vec![None; readings.len()];
        marks[1] = Some((2, "load on, \"2.2\" ohm"));
        marks[5] = Some((7, ""));
        (readings, marks)
    }

    fn ts(m: &Measurement) -> String {
        DateTime::<Local>::from(m.wall_time).to_rfc3339()
    }

    fn csv_of(readings: &[Measurement], marks: &[Option<(u32, &str)>]) -> String {
        let layout = CsvLayout {
            family_slots: 2,
            extra_slots: 0,
            integral: false,
            markers: true,
        };
        let mut out = format!("{}\n", device_comment("UT61E+")).into_bytes();
        {
            let mut w = csv::Writer::from_writer(&mut out);
            w.write_record(layout.header().iter().map(|c| c.as_ref()))
                .expect("header");
            for (m, mark) in readings.iter().zip(marks) {
                let ts = ts(m);
                let row = layout.row(m, &ts, None, 0, *mark);
                w.write_record(row.iter().map(|c| c.as_ref())).expect("row");
            }
        }
        String::from_utf8(out).expect("UTF-8")
    }

    fn json_of(readings: &[Measurement], marks: &[Option<(u32, &str)>]) -> String {
        let mut out = format!("{}\n", metadata_line("UT61E+", None)).into_bytes();
        for (m, mark) in readings.iter().zip(marks) {
            write_measurement_json(&mut out, m, &ts(m), false, None, *mark).expect("a line");
            out.push(b'\n');
        }
        String::from_utf8(out).expect("UTF-8")
    }

    /// What an import rebuilds, ready to write again.
    fn rebuilt(imported: &Imported) -> (Vec<Measurement>, Marks<'_>) {
        let t0 = Instant::now();
        let readings: Vec<Measurement> = imported
            .readings
            .iter()
            .enumerate()
            .map(|(i, r)| {
                r.clone()
                    .into_measurement(t0 + Duration::from_secs(i as u64))
            })
            .collect();
        let mut marks = vec![None; readings.len()];
        for k in &imported.markers {
            marks[k.reading] = Some((k.number, k.note.as_str()));
        }
        (readings, marks)
    }

    /// A CSV export reads back into readings that export as the same bytes:
    /// values, words, overloads, NCV, sub-values, two-word flags, markers
    /// with commas and quotes.
    #[test]
    fn a_csv_export_reads_back_to_the_same_file() {
        let (readings, marks) = shapes();
        let text = csv_of(&readings, &marks);
        let imported = read_csv(&text).expect("a CSV export");
        assert_eq!(imported.device.as_deref(), Some("UT61E+"));
        assert_eq!(imported.readings.len(), 6);
        assert!(matches!(
            imported.readings[3].value,
            MeasuredValue::NoReading("")
        ));
        assert!(matches!(imported.readings[4].value, MeasuredValue::Absent));
        let flags = imported.readings[1].flags;
        assert!(flags.hold && flags.low_battery && flags.peak_min && flags.lead_error);
        assert!(!flags.min, "MIN is not read out of P-MIN");
        let (again, marks_again) = rebuilt(&imported);
        assert_eq!(csv_of(&again, &marks_again), text);
    }

    /// The same for JSON, which also keeps `progress`, `experimental` and
    /// the sub-values' elapsed times.
    #[test]
    fn a_json_export_reads_back_to_the_same_file() {
        let (mut readings, marks) = shapes();
        readings[0].progress = Some(40);
        readings[5].aux_values[1].elapsed_secs = Some(12);
        let text = json_of(&readings, &marks);
        let imported = read_json(&text).expect("a JSON export");
        assert_eq!(imported.device.as_deref(), Some("UT61E+"));
        assert_eq!(imported.view, None);
        assert_eq!(imported.readings[0].progress, Some(40));
        let (again, marks_again) = rebuilt(&imported);
        assert_eq!(json_of(&again, &marks_again), text);
    }

    /// The view goes into the metadata as an object and comes back as its
    /// text; text that is not JSON is left out rather than written.
    #[test]
    fn a_json_export_carries_the_view() {
        let line = metadata_line("UT61E+", Some(r#"{"window":30.0,"mean":true}"#));
        // As written, whatever key order a JSON value would give it.
        assert_eq!(
            line,
            r#"{"_metadata":{"device":"UT61E+","view":{"window":30.0,"mean":true}}}"#
        );
        let imported = read_json(&format!("{line}\n")).expect("reads");
        assert_eq!(
            imported.view.as_deref(),
            Some(r#"{"window":30.0,"mean":true}"#)
        );
        assert_eq!(
            metadata_line("UT61E+", Some("[1, 2]")),
            metadata_line("UT61E+", None),
            "an array is no view"
        );
        assert_eq!(
            metadata_line("UT61E+", Some("not json")),
            metadata_line("UT61E+", None)
        );
    }

    /// A CSV a spreadsheet saved as UTF-8 opens with a byte-order mark; it
    /// reads the same.
    #[test]
    fn a_byte_order_mark_is_skipped() {
        let (readings, marks) = shapes();
        let text = format!("\u{feff}{}", csv_of(&readings, &marks));
        assert_eq!(
            read_csv(&text).expect("reads").device.as_deref(),
            Some("UT61E+")
        );
        let text = format!("\u{feff}{}", json_of(&readings, &marks));
        assert_eq!(read_json(&text).expect("reads").readings.len(), 6);
    }

    /// A limit keeps the first readings and counts the rest; the markers on
    /// readings left out go with them.
    #[test]
    fn a_limit_keeps_the_first_readings() {
        let (readings, marks) = shapes();
        let csv = csv_of(&readings, &marks);
        let imported = read_csv_from(csv.as_bytes(), Some(4)).expect("reads");
        assert_eq!((imported.readings.len(), imported.left_out), (4, 2));
        assert_eq!(
            imported.markers.len(),
            1,
            "the marker on the sixth is left out"
        );
        let json = json_of(&readings, &marks);
        let imported = read_json_from(json.as_bytes(), Some(4)).expect("reads");
        assert_eq!((imported.readings.len(), imported.left_out), (4, 2));
    }

    /// A meter's name read from a file stays on one line, and file text
    /// quoted in a message cannot act on the terminal it is printed to.
    #[test]
    fn file_text_stays_inert() {
        let line = "{\"_metadata\":{\"device\":\"UT\\n61E+\"}}\n";
        assert_eq!(
            read_json(line).expect("reads").device.as_deref(),
            Some("UT 61E+")
        );
        assert_eq!(device_comment("UT\n61E+"), "# device: UT 61E+");
        let (readings, marks) = shapes();
        let text = csv_of(&readings, &marks).replace("1.6109", "\u{1b}[2J");
        let message = read_csv(&text).expect_err("not a value");
        assert!(
            !message.contains('\u{1b}') && message.contains("\\u{1b}"),
            "{message}"
        );
    }

    #[test]
    fn the_kind_comes_from_the_name_then_the_content() {
        let kind = |name: &str, head: &str| ExportKind::detect(Path::new(name), head);
        assert_eq!(kind("a.CSV", "{"), ExportKind::Csv);
        assert_eq!(kind("a.json", ""), ExportKind::Json);
        assert_eq!(kind("a.replay", ""), ExportKind::Replay);
        assert_eq!(kind("recording", "# dmm-replay 1\n"), ExportKind::Replay);
        assert_eq!(
            kind("recording", "\u{feff}{\"_metadata\""),
            ExportKind::Json
        );
        assert_eq!(kind("recording", "# device: UT61E+\n"), ExportKind::Csv);
    }

    #[test]
    fn a_malformed_row_names_its_line() {
        let (readings, marks) = shapes();
        let text = csv_of(&readings, &marks).replace("1.6109", "1.6.109");
        let message = read_csv(&text).expect_err("a bad value");
        assert!(
            message.starts_with("line 3:") && message.contains("1.6.109"),
            "{message}"
        );

        let text = json_of(&readings, &marks).replace("\"mode\"", "\"mood\"");
        let message = read_json(&text).expect_err("a missing key");
        assert!(message.starts_with("line 2:"), "{message}");

        let text = csv_of(&readings, &marks).replace("HOLD", "HOLDS");
        assert!(
            read_csv(&text)
                .expect_err("an unknown flag")
                .contains("HOLDS")
        );
    }

    /// A file whose times step backwards — the clock was set back mid-run —
    /// still reads: the times are the file's to keep.
    #[test]
    fn times_that_step_backwards_still_read() {
        let (mut readings, marks) = shapes();
        readings[2].wall_time = readings[0].wall_time - Duration::from_secs(60);
        let imported = read_csv(&csv_of(&readings, &marks)).expect("reads");
        assert!(imported.readings[2].wall_time < imported.readings[1].wall_time);
    }
}
