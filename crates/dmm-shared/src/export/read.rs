//! Reading an export back: the other half of this module's writers, kept
//! beside them so a column or key cannot change on one side only.
//!
//! What comes back is what the file holds. An export carries the reading as
//! the meter showed it, not the frame it came in, so a reading read back has
//! no spec, no mode or range code and no wire bytes; a word the meter showed
//! in place of a value, which exports as an empty cell, comes back as a blank
//! word. CSV also has no `progress`, `experimental` or DC flag, and writes a
//! sub-value's unit even where the meter left it to the main reading's.

use super::{ReadError, ReadReason};
use chrono::{DateTime, Local};
use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use dmm_lib::transform::RAW_LABEL;
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
    /// Sub-value groups for the meter's own sub-values: a CSV's `auxN_*`
    /// column groups, empty ones included, less [`Imported::extra_slots`];
    /// or the most any JSON reading carries — what a CSV written from it
    /// keeps.
    pub family_slots: usize,
    /// Trailing groups for the sub-values software appended (a scale's
    /// `Raw`): the most any reading carries, so a CSV written from it keeps
    /// them in the columns they had rather than packed into the meter's.
    pub extra_slots: usize,
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
    /// How long after the first reading each was taken: the sum of each
    /// reading's gap from the one before. A clock set back mid-file adds no
    /// time there and keeps the spacing of what follows, rather than sending
    /// a session's time backwards or piling the next readings onto one instant.
    pub fn offsets(&self) -> Vec<Duration> {
        let mut previous = self.readings.first().map(|r| r.wall_time);
        let mut total = Duration::ZERO;
        self.readings
            .iter()
            .map(|r| {
                total += previous
                    .and_then(|p| r.wall_time.duration_since(p).ok())
                    .unwrap_or_default();
                previous = Some(r.wall_time);
                total
            })
            .collect()
    }
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
    /// How many of `aux`, at its end, software appended: the trailing run of
    /// a scale's [`RAW_LABEL`]s, which are always appended last.
    pub extra_aux: usize,
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
            screen_only: false,
        }
    }
}

/// Why a file would not read, with the line it stopped at.
fn at(line: usize, reason: ReadReason) -> ReadError {
    ReadError::at(line, reason)
}

/// A meter's name from a file, on one line whatever the file put in it.
fn device_name(name: &str) -> Option<String> {
    let name: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    Some(name.trim().to_string()).filter(|n| !n.is_empty())
}

/// A reading's wall time as both binaries write it, in a CSV `timestamp`
/// cell or a JSON `timestamp` key: RFC 3339, which [`wall_time`] reads back.
pub fn timestamp(t: DateTime<Local>) -> String {
    t.to_rfc3339()
}

/// A timestamp cell or key: RFC 3339, as [`timestamp`] writes it.
fn wall_time(line: usize, text: &str) -> Result<SystemTime, ReadError> {
    DateTime::parse_from_rfc3339(text)
        .map(SystemTime::from)
        .map_err(|e| at(line, crate::replay::not_a_date(text, e)))
}

/// A value as a CSV cell or a JSON sub-value string writes it: the meter's
/// digits, `OL`, `NCV:n`, or empty. Empty is a word the meter showed or no
/// value at all, which the caller tells apart. A number that is not finite is
/// no reading, without its text. The digits are also returned as the display
/// text, which re-export writes back unchanged.
fn value_text(
    line: usize,
    text: &str,
) -> Result<(Option<MeasuredValue>, Option<String>), ReadError> {
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
            .map_err(|_| at(line, ReadReason::NotAnNcvLevel(text.to_string())))?;
        return Ok((Some(MeasuredValue::NcvLevel(level)), None));
    }
    let number: f64 = text
        .parse()
        .map_err(|_| at(line, ReadReason::NotAValue(text.to_string())))?;
    // A scale that overflowed once wrote `inf` here. It breaks the trace even
    // beside the Raw sub-value such a row has, and doesn't refuse the file.
    Ok(match MeasuredValue::finite(number) {
        Some(value) => (Some(value), Some(text.to_string())),
        None => (Some(MeasuredValue::NoReading("")), None),
    })
}

/// No value: a blank word in place of a reading, which breaks the trace as
/// the meter's word did, unless the reading has sub-values of the meter's own
/// — a frame that carried only its sub-values, which leaves the trace alone.
/// The `Raw` a scale appended doesn't count: it goes on every row.
fn no_value(aux: &[ImportedAux]) -> MeasuredValue {
    if aux.len() > extra_aux(aux) {
        MeasuredValue::Absent
    } else {
        MeasuredValue::NoReading("")
    }
}

/// How many of a reading's sub-values a scale appended: the run of `Raw`s
/// at its end, two when a scale went on an import of a scaled file. Exports
/// write that label untranslated, so it reads back as written.
fn extra_aux(aux: &[ImportedAux]) -> usize {
    aux.iter()
        .rev()
        .take_while(|a| a.label == RAW_LABEL)
        .count()
}

/// A CSV flags cell: the labels `StatusFlags` prints, space-separated, in
/// [`Flag::ALL`] order — which is also the order to read them in, so a
/// two-word label ("LOW BAT") and one that ends another ("MIN", "P-MIN")
/// cannot be misread.
fn flags_cell(line: usize, cell: &str) -> Result<StatusFlags, ReadError> {
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
        Err(at(line, ReadReason::NotAFlag(rest.to_string())))
    }
}

/// Read a CSV export: `#` lines, the header, then one row per reading.
///
/// Columns are found by name, so a file with integral or marker columns, or
/// any number of sub-value groups, reads the same way. The integral is left
/// out: it is worked out again from the readings by whatever reads them.
pub fn read_csv(text: &str) -> Result<Imported, ReadError> {
    read_csv_from(text.as_bytes(), None)
}

/// [`read_csv`], from a stream, keeping at most `limit` readings and
/// counting the rest in [`Imported::left_out`]: a file far larger than the
/// session can hold is read through without being held.
pub fn read_csv_from(mut input: impl BufRead, limit: Option<usize>) -> Result<Imported, ReadError> {
    let mut device = None;
    let mut first_row_line = 0;
    let mut header = String::new();
    loop {
        let mut line = String::new();
        let read = input
            .read_line(&mut line)
            .map_err(|e| ReadError::whole(ReadReason::Io(e.to_string())))?;
        if read == 0 {
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
        .map_err(|e| at(first_row_line, ReadReason::NoHeaderRow(e.to_string())))?
        .clone();
    let column = |name: &str| header.iter().position(|h| h == name);
    let required = |name: &'static str| {
        column(name).ok_or_else(|| at(first_row_line, ReadReason::NoColumn(name)))
    };
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
        family_slots: 0,
        extra_slots: 0,
        left_out: 0,
    };
    for (i, row) in reader.records().enumerate() {
        let line = first_row_line + 1 + i;
        let row = row.map_err(|e| at(line, ReadReason::Parse(e.to_string())))?;
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
            value: main.unwrap_or_else(|| no_value(&aux)),
            unit: cell(unit).to_string(),
            range: cell(range).to_string(),
            display_raw,
            progress: None,
            experimental: false,
            flags: flags_cell(line, cell(flags))?,
            extra_aux: extra_aux(&aux),
            aux,
        });
        if let Some((number, note)) = marker
            && !cell(number).is_empty()
        {
            imported.markers.push(ImportedMarker {
                reading: imported.readings.len() - 1,
                number: cell(number).parse().map_err(|_| {
                    at(line, ReadReason::NotAMarkerNumber(cell(number).to_string()))
                })?,
                note: cell(note).to_string(),
            });
        }
    }
    // The groups the file lays out, the appended ones last as the writer
    // pins them, so re-export writes the header it was read from.
    imported.extra_slots = max_extra(&imported.readings);
    imported.family_slots = aux_groups
        .len()
        .saturating_sub(imported.extra_slots)
        .max(max_family(&imported.readings));
    Ok(imported)
}

/// The most appended sub-values any reading carries.
fn max_extra(readings: &[ImportedReading]) -> usize {
    readings.iter().map(|r| r.extra_aux).max().unwrap_or(0)
}

/// The most of the meter's own sub-values any reading carries.
fn max_family(readings: &[ImportedReading]) -> usize {
    readings
        .iter()
        .map(|r| r.aux.len() - r.extra_aux)
        .max()
        .unwrap_or(0)
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
pub fn read_json(text: &str) -> Result<Imported, ReadError> {
    read_json_from(text.as_bytes(), None)
}

/// [`read_json`], from a stream, keeping at most `limit` readings and
/// counting the rest in [`Imported::left_out`].
pub fn read_json_from(input: impl BufRead, limit: Option<usize>) -> Result<Imported, ReadError> {
    let mut lines = input
        .lines()
        .enumerate()
        .map(|(i, l)| {
            l.map(|l| (i, l))
                .map_err(|e| ReadError::whole(ReadReason::Io(e.to_string())))
        })
        .filter(|l| l.as_ref().map_or(true, |(_, l)| !l.trim().is_empty()));
    let (_, first) = lines
        .next()
        .ok_or_else(|| ReadError::whole(ReadReason::Empty))??;
    let first = first.trim_start_matches('\u{feff}');
    let metadata: JsonMetadataLine =
        serde_json::from_str(first).map_err(|e| at(1, ReadReason::NoMetadata(e.to_string())))?;
    let mut imported = Imported {
        device: metadata.metadata.device.as_deref().and_then(device_name),
        view: metadata.metadata.view.map(|v| v.get().to_string()),
        readings: Vec::new(),
        markers: Vec::new(),
        family_slots: 0,
        extra_slots: 0,
        left_out: 0,
    };
    for next in lines {
        let (i, text) = next?;
        let line = i + 1;
        if limit.is_some_and(|limit| imported.readings.len() >= limit) {
            imported.left_out += 1;
            continue;
        }
        let r: JsonReadingIn =
            serde_json::from_str(&text).map_err(|e| at(line, ReadReason::Parse(e.to_string())))?;
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
                .ok_or_else(|| at(line, ReadReason::NotAValue(n.to_string())))?,
            serde_json::Value::String(s) if s == "OL" => MeasuredValue::Overload,
            serde_json::Value::Object(o) => o
                .get("ncv_level")
                .and_then(|l| l.as_u64())
                .and_then(|l| u8::try_from(l).ok())
                .map(MeasuredValue::NcvLevel)
                .ok_or_else(|| at(line, ReadReason::NotAValue(r.value.to_string())))?,
            serde_json::Value::Null => no_value(&aux),
            other => return Err(at(line, ReadReason::NotAValue(other.to_string()))),
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
            extra_aux: extra_aux(&aux),
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
    imported.family_slots = max_family(&imported.readings);
    imported.extra_slots = max_extra(&imported.readings);
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
        let rows: Vec<_> = readings
            .iter()
            .zip(marks)
            .map(|(m, k)| (m, 0, *k))
            .collect();
        csv_in(layout, &rows)
    }

    /// A reading to write, with how many of its sub-values were appended and
    /// its marker.
    type Row<'a> = (&'a Measurement, usize, Option<(u32, &'a str)>);

    /// A CSV export laid out as `layout`.
    fn csv_in(layout: CsvLayout, rows: &[Row<'_>]) -> String {
        let mut out = format!("{}\n", device_comment("UT61E+")).into_bytes();
        {
            let mut w = csv::Writer::from_writer(&mut out);
            w.write_record(layout.header().iter().map(|c| c.as_ref()))
                .expect("header");
            for &(m, extra_aux, mark) in rows {
                let ts = ts(m);
                let row = layout.row(m, &ts, None, extra_aux, mark);
                w.write_record(row.iter().map(|c| c.as_ref())).expect("row");
            }
        }
        String::from_utf8(out).expect("UTF-8")
    }

    /// A UT181A-like run scaled mid-recording: a DC row before the scale, an
    /// AC row with Frequency, Period and the scale's `Raw`, and a DC row with
    /// `Raw` alone.
    fn scaled_mid_run() -> Vec<(Measurement, usize)> {
        let (shapes, _) = shapes();
        let raw = |value: f64| AuxValue {
            label: Cow::Borrowed(RAW_LABEL),
            value: MeasuredValue::Normal(value),
            unit: Cow::Borrowed("mV"),
            display_raw: Some(format!("{value}")),
            elapsed_secs: None,
        };
        let early = shapes[0].clone();
        let mut ac = shapes[5].clone();
        ac.aux_values[1] = AuxValue {
            label: Cow::Borrowed("Period"),
            value: MeasuredValue::Normal(20.0),
            unit: Cow::Borrowed("ms"),
            display_raw: Some("20.00".to_string()),
            elapsed_secs: None,
        };
        ac.aux_values.push(raw(51.37));
        let mut dc = shapes[0].clone();
        dc.wall_time += Duration::from_secs(6);
        dc.aux_values.push(raw(16.109));
        vec![(early, 0), (ac, 1), (dc, 1)]
    }

    /// The scale's `Raw` keeps its trailing group through a CSV read back:
    /// the file exports again as the same bytes, rather than with `Raw`
    /// packed into the meter's groups beside Frequency on one row and alone
    /// in `aux1_*` on the next.
    #[test]
    fn a_csv_import_keeps_the_raw_column_in_place() {
        let layout = CsvLayout {
            family_slots: 2,
            extra_slots: 1,
            integral: false,
            markers: false,
        };
        let readings = scaled_mid_run();
        let rows: Vec<_> = readings.iter().map(|(m, e)| (m, *e, None)).collect();
        let text = csv_in(layout, &rows);
        let imported = read_csv(&text).expect("a CSV export");
        assert_eq!((imported.family_slots, imported.extra_slots), (2, 1));
        let extras: Vec<usize> = imported.readings.iter().map(|r| r.extra_aux).collect();
        assert_eq!(extras, [0, 1, 1]);
        let (again, _) = rebuilt(&imported);
        let rows: Vec<_> = again
            .iter()
            .zip(&imported.readings)
            .map(|(m, r)| (m, r.extra_aux, None))
            .collect();
        let layout_again = CsvLayout {
            family_slots: imported.family_slots,
            extra_slots: imported.extra_slots,
            ..layout
        };
        assert_eq!(csv_in(layout_again, &rows), text);
    }

    /// A scale on an import of a scaled file appends a second `Raw`: both
    /// count as appended, so the file's `Raw` stays in the third group on
    /// the DC row as on the AC one, and the file exports again as the same
    /// bytes. The early row carries only the second scale's `Raw`.
    #[test]
    fn a_csv_import_keeps_two_raw_columns_in_place() {
        let layout = CsvLayout {
            family_slots: 2,
            extra_slots: 2,
            integral: false,
            markers: false,
        };
        let second = AuxValue {
            label: Cow::Borrowed(RAW_LABEL),
            value: MeasuredValue::Normal(1.6109),
            unit: Cow::Borrowed("V"),
            display_raw: Some("1.6109".to_string()),
            elapsed_secs: None,
        };
        let readings: Vec<(Measurement, usize)> = scaled_mid_run()
            .into_iter()
            .map(|(mut m, extra)| {
                m.aux_values.push(second.clone());
                (m, extra + 1)
            })
            .collect();
        let rows: Vec<_> = readings.iter().map(|(m, e)| (m, *e, None)).collect();
        let text = csv_in(layout, &rows);
        let imported = read_csv(&text).expect("a CSV export");
        assert_eq!((imported.family_slots, imported.extra_slots), (2, 2));
        let extras: Vec<usize> = imported.readings.iter().map(|r| r.extra_aux).collect();
        assert_eq!(extras, [1, 2, 2]);
        let (again, _) = rebuilt(&imported);
        let dc = layout.row(&again[2], "ts", None, imported.readings[2].extra_aux, None);
        assert_eq!(&dc[6..12], ["", "", "", "", "", ""]);
        assert_eq!(&dc[12..15], [RAW_LABEL, "16.109", "mV"]);
        let rows: Vec<_> = again
            .iter()
            .zip(&imported.readings)
            .map(|(m, r)| (m, r.extra_aux, None))
            .collect();
        assert_eq!(csv_in(layout, &rows), text);
    }

    /// JSON doesn't lay sub-values out in groups, but a CSV written from it
    /// still keeps `Raw` in the trailing group, not the meter's.
    #[test]
    fn a_json_import_writes_raw_to_the_trailing_csv_group() {
        let readings = scaled_mid_run();
        let measurements: Vec<Measurement> = readings.iter().map(|(m, _)| m.clone()).collect();
        let text = json_of(&measurements, &vec![None; measurements.len()]);
        let imported = read_json(&text).expect("a JSON export");
        assert_eq!((imported.family_slots, imported.extra_slots), (2, 1));
        let layout = CsvLayout {
            family_slots: imported.family_slots,
            extra_slots: imported.extra_slots,
            integral: false,
            markers: false,
        };
        let (again, _) = rebuilt(&imported);
        let dc = layout.row(&again[2], "ts", None, imported.readings[2].extra_aux, None);
        assert_eq!(&dc[6..12], ["", "", "", "", "", ""]);
        assert_eq!(&dc[12..15], [RAW_LABEL, "16.109", "mV"]);
        let ac = layout.row(&again[1], "ts", None, imported.readings[1].extra_aux, None);
        assert_eq!(&ac[6..9], ["Frequency", "50.01", "Hz"]);
        assert_eq!(&ac[9..12], ["Period", "20.00", "ms"]);
        assert_eq!(&ac[12..15], [RAW_LABEL, "51.37", "mV"]);
        let early = layout.row(&again[0], "ts", None, imported.readings[0].extra_aux, None);
        assert_eq!(&early[6..15], ["", "", "", "", "", "", "", "", ""]);
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
        let message = read_csv(&text).expect_err("not a value").to_string();
        assert!(
            !message.contains('\u{1b}') && message.contains("\\u{1b}"),
            "{message}"
        );
        // A replay's header line is quoted at no line.
        let mut text = dmm_lib::replay::header("ut61eplus", "\u{1b}[2J", None, None);
        text.push_str(&dmm_lib::replay::sample_line(Duration::ZERO, &[2, 3]));
        let replay = dmm_lib::replay::Replay::parse(&text).expect("parses");
        let e = crate::replay::recorded_at(&replay).expect_err("not a date");
        let l = crate::l10n::Localizer::en_us(crate::help::RESOURCES);
        for message in [e.to_string(), l.read_error(&e)] {
            assert!(
                !message.contains('\u{1b}') && message.contains("`# recorded: \\u{1b}[2J`"),
                "{message}"
            );
        }
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

    /// A scale's `Raw` sits on every row, so it doesn't make a no-reading row
    /// a frame of sub-values, which would bridge the gap in the trace.
    #[test]
    fn a_scaled_no_reading_row_reads_back_as_no_reading() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::NoReading("Auto"),
            "V",
            StatusFlags::default(),
        );
        dmm_lib::transform::Transform::linear(2.0, 0.0, None).apply(&mut m);
        let readings = [m];
        let imported = read_csv(&csv_of(&readings, &[None])).unwrap();
        assert!(matches!(
            imported.readings[0].value,
            MeasuredValue::NoReading("")
        ));
        let imported = read_json(&json_of(&readings, &[None])).unwrap();
        assert!(matches!(
            imported.readings[0].value,
            MeasuredValue::NoReading("")
        ));
    }

    /// `str::parse::<f64>` takes these, and a scale that overflowed wrote the
    /// first. A JSON main value is a number, which has no way to spell the
    /// first three, and out of range is a parse error.
    #[test]
    fn a_value_that_is_not_finite_reads_as_no_value() {
        let (readings, marks) = shapes();
        for word in ["inf", "-infinity", "NaN", "1e999"] {
            let text = csv_of(&readings, &marks).replace("1.6109", word);
            let imported = read_csv(&text).expect(word);
            assert!(
                matches!(imported.readings[0].value, MeasuredValue::NoReading("")),
                "{word}"
            );
            let text = json_of(&readings, &marks).replace("\"0.2000\"", &format!("\"{word}\""));
            let imported = read_json(&text).expect(word);
            assert!(
                matches!(
                    imported.readings[4].aux[0].value,
                    MeasuredValue::NoReading("")
                ),
                "{word}"
            );
        }
        // Not `None`, which beside a sub-value would leave the trace unbroken.
        assert!(matches!(
            value_text(1, "inf"),
            Ok((Some(MeasuredValue::NoReading("")), None))
        ));
        let text = json_of(&readings, &marks).replace("1.6109", "1e999");
        let error = read_json(&text).expect_err("out of range");
        assert!(matches!(error.reason, ReadReason::Parse(_)), "{error}");
    }

    #[test]
    fn a_malformed_row_names_its_line() {
        let (readings, marks) = shapes();
        let text = csv_of(&readings, &marks).replace("1.6109", "1.6.109");
        let error = read_csv(&text).expect_err("a bad value");
        assert_eq!(
            error,
            ReadError::at(3, ReadReason::NotAValue("1.6.109".to_string()))
        );

        let text = json_of(&readings, &marks).replace("\"mode\"", "\"mood\"");
        let error = read_json(&text).expect_err("a missing key");
        assert!(matches!(error.reason, ReadReason::Parse(_)), "{error}");
        assert_eq!(error.line, Some(2));

        let text = csv_of(&readings, &marks).replace("HOLD", "HOLDS");
        assert!(matches!(
            read_csv(&text).expect_err("an unknown flag").reason,
            ReadReason::NotAFlag(rest) if rest.contains("HOLDS")
        ));
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

    /// A file's first reading repeated at `secs` from its wall time.
    fn imported_at(secs: &[i64]) -> Imported {
        let (shape, marks) = shapes();
        let mut imported = read_csv(&csv_of(&shape, &marks)).expect("reads");
        let first = imported.readings[0].clone();
        imported.markers.clear();
        imported.readings = secs
            .iter()
            .map(|&s| {
                let step = Duration::from_secs(s.unsigned_abs());
                let mut r = first.clone();
                r.wall_time = if s >= 0 {
                    first.wall_time + step
                } else {
                    first.wall_time - step
                };
                r
            })
            .collect();
        imported
    }

    fn secs(offsets: Vec<Duration>) -> Vec<u64> {
        offsets.iter().map(Duration::as_secs).collect()
    }

    #[test]
    fn offsets_follow_the_wall_clock() {
        assert_eq!(secs(imported_at(&[0, 1, 2, 4]).offsets()), [0, 1, 2, 4]);
    }

    /// A clock set back ten minutes adds no time at the step, and the
    /// readings after it keep their spacing instead of piling onto one
    /// instant until the clock catches up.
    #[test]
    fn a_clock_set_back_keeps_the_spacing_after_it() {
        let offsets = imported_at(&[0, 1, 2, -598, -597, -596]).offsets();
        assert_eq!(secs(offsets), [0, 1, 2, 2, 3, 4]);
    }
}
