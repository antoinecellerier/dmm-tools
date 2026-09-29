//! What an export is called and what its CSV columns and JSON hold, wherever
//! it is written from.
//!
//! `dmm-gui`'s Export… and `dmm-cli read` write the same objects under the
//! same names because both come through here: a script reading one file works
//! on the other, and a field added to a reading reaches both binaries at once.

mod csv_layout;
mod read;

pub use csv_layout::{CsvLayout, device_comment};
pub use read::{Imported, ImportedAux, ImportedMarker, ImportedReading, read_csv, read_json};

use chrono::{DateTime, Local};
use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use serde::Serialize;
use serde_json::{Value, json};
use std::borrow::Cow;

/// The `_metadata` object a JSON export opens with, as its line — without the
/// newline that ends it.
///
/// `view` is the viewer's view of the readings, as JSON text the GUI writes
/// and reads back. It goes in as an object, written as it came — not re-keyed
/// by a JSON value, whose key order depends on how the binary was built — and
/// text that is not a one-line JSON object is left out rather than written as
/// a string nothing could read back.
pub fn metadata_line(device_model: &str, view: Option<&str>) -> String {
    let view = view.filter(|v| {
        let object = !v.contains(['\n', '\r'])
            && matches!(serde_json::from_str::<Value>(v), Ok(Value::Object(_)));
        if !object {
            log::warn!("export: the view is not a one-line JSON object and is left out");
        }
        object
    });
    match view {
        Some(view) => format!(
            "{{\"_metadata\":{{\"device\":{},\"view\":{view}}}}}",
            Value::from(device_model)
        ),
        None => json!({"_metadata": {"device": device_model}}).to_string(),
    }
}

/// Write one reading to `w` as a JSON export line, without the newline that
/// ends it.
///
/// `timestamp_rfc3339` is the wall time the reading was taken at, which only
/// the caller can work out: the CLI derives it from the session's clock
/// origin, the GUI stamped it onto the sample as it arrived. `marker` is the
/// number and note of the marker the user placed on this reading, if any.
///
/// Serialized straight from borrowed fields: a `Value` tree per reading cost
/// dozens of allocations, and a long recording's export froze the GUI.
pub fn write_measurement_json<W: std::io::Write + ?Sized>(
    w: &mut W,
    m: &Measurement,
    timestamp_rfc3339: &str,
    experimental: bool,
    integral: Option<(f64, &str)>,
    marker: Option<(u32, &str)>,
) -> std::io::Result<()> {
    let value = match &m.value {
        MeasuredValue::Normal(v) => Value::from(*v),
        MeasuredValue::Overload => json!("OL"),
        MeasuredValue::NcvLevel(l) => json!({"ncv_level": l}),
        // Null rather than a missing key or the word: every line keeps its
        // "value" key, as `display_raw` and `progress` stay present as null
        // when absent, and "mode" already carries the word. A frame without
        // a main reading has its sub-values in "aux".
        MeasuredValue::NoReading(_) | MeasuredValue::Absent => Value::Null,
    };
    let reading = JsonReading {
        timestamp: timestamp_rfc3339,
        mode: &m.mode,
        value,
        unit: &m.unit,
        range: &m.range_label,
        display_raw: m.display_raw.as_deref(),
        progress: m.progress,
        experimental,
        flags: JsonFlags(&m.flags),
        aux: m
            .aux_values
            .iter()
            .map(|aux| JsonAuxValue {
                label: &aux.label,
                // Null for a no-reading word, as for the main value.
                value: match aux.value {
                    MeasuredValue::NoReading(_) | MeasuredValue::Absent => None,
                    _ => Some(aux.value_export_str()),
                },
                unit: aux.unit_or(&m.unit),
                elapsed_secs: aux.elapsed_secs,
            })
            .collect(),
        integral: integral.map(|(v, _)| v),
        integral_unit: integral.map(|(_, u)| u),
        marker: marker.map(|(n, _)| n),
        note: marker.map(|(_, n)| n),
    };
    serde_json::to_writer(w, &reading).map_err(std::io::Error::from)
}

/// One JSON export line. The field order is the line's key order, which is
/// part of what both binaries write: a field added in the wrong place
/// reorders every export (the goldens in the tests catch it).
#[derive(Serialize)]
struct JsonReading<'a> {
    timestamp: &'a str,
    mode: &'a str,
    /// A number allocates nothing; "OL" and NCV are rare.
    value: Value,
    unit: &'a str,
    range: &'a str,
    display_raw: Option<&'a str>,
    progress: Option<u16>,
    experimental: bool,
    flags: JsonFlags<'a>,
    /// Omitted entirely when there are none, so output for the families
    /// that never produce sub-values has no "aux" key.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    aux: Vec<JsonAuxValue<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integral: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integral_unit: Option<&'a str>,
    /// Only on the marked readings, so an unmarked line is what it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    marker: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
}

/// The flags object, from [`StatusFlags::as_pairs`] rather than a
/// hand-written list: the old list had drifted and was missing `loz` and
/// `void`, so a VC-890 reading the meter had marked invalid was
/// indistinguishable from a good one in JSON — while the text and CSV formats
/// reported it.
struct JsonFlags<'a>(&'a StatusFlags);

impl Serialize for JsonFlags<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(self.0.as_pairs())
    }
}

/// One sub-value in "aux". A missing value or elapsed time stays as null.
#[derive(Serialize)]
struct JsonAuxValue<'a> {
    label: &'a str,
    value: Option<Cow<'a, str>>,
    unit: &'a str,
    elapsed_secs: Option<u32>,
}

/// The name an export opens with: the meter, the mode it stayed in and the
/// moment the recording started, as
/// `measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv`, so a folder of exports
/// sorts by meter and by run. A recording that crossed a function switch has
/// no one mode and leaves that segment out.
pub fn default_name(
    model: &str,
    mode: Option<&str>,
    start: DateTime<Local>,
    extension: &str,
) -> String {
    let mode = mode
        .map(|m| format!("{}-", file_safe(m)))
        .unwrap_or_default();
    format!(
        "measurements-{}-{mode}{}.{extension}",
        file_safe(model),
        start.format("%Y-%m-%d_%H-%M-%S"),
    )
}

/// A meter or mode name as one file-name word: runs of whitespace become a
/// single `-` and the separators a path could read drop out, so "Mock
/// UT61E+" exports as `Mock-UT61E+`.
///
/// A slash separates words rather than vanishing — deleting it ran the
/// registry's `UT171A/B/C` together as `UT171ABC`.
pub fn file_safe(name: &str) -> String {
    name.replace(':', "")
        .replace(['/', '\\'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use dmm_lib::measurement::AuxValue;

    fn start() -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, 15, 14, 30, 5)
            .single()
            .expect("a fixed local timestamp")
    }

    #[test]
    fn a_name_carries_the_meter_the_mode_and_the_start() {
        assert_eq!(
            default_name("UT61E+", Some("DC V"), start(), "csv"),
            "measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv"
        );
        // A recording with no one mode keeps meter and time, nothing between.
        assert_eq!(
            default_name("UT61E+", None, start(), "replay"),
            "measurements-UT61E+-2026-09-15_14-30-05.replay"
        );
    }

    /// One reading's export line, as both binaries write it.
    fn line(
        m: &Measurement,
        ts: &str,
        experimental: bool,
        integral: Option<(f64, &str)>,
        marker: Option<(u32, &str)>,
    ) -> String {
        let mut out = Vec::new();
        write_measurement_json(&mut out, m, ts, experimental, integral, marker)
            .expect("a Vec takes every write");
        String::from_utf8(out).expect("JSON is UTF-8")
    }

    /// The line parsed back, for tests that look at one key.
    fn parsed(line: &str) -> Value {
        serde_json::from_str(line).expect("valid JSON")
    }

    /// A "Max" sub-value.
    fn aux(value: MeasuredValue, unit: &'static str, display_raw: Option<&str>) -> AuxValue {
        AuxValue {
            label: "Max".into(),
            value,
            unit: unit.into(),
            display_raw: display_raw.map(str::to_string),
            elapsed_secs: None,
        }
    }

    /// A DC V reading of `value`, flags clear.
    fn reading(value: MeasuredValue) -> Measurement {
        Measurement::test_fixture(value, "V", StatusFlags::default())
    }

    /// One row of [`every_reading_shape_exports_byte_for_byte`].
    struct Shape {
        label: &'static str,
        m: Measurement,
        experimental: bool,
        integral: Option<(f64, &'static str)>,
        marker: Option<(u32, &'static str)>,
        expected: &'static str,
    }

    impl Shape {
        fn new(label: &'static str, m: Measurement, expected: &'static str) -> Self {
            Shape {
                label,
                m,
                experimental: false,
                integral: None,
                marker: None,
                expected,
            }
        }
    }

    /// Every shape a reading's line takes, with the whole line it exports
    /// as: key order, nulls, number formatting and string escapes. The flags
    /// object is in `Flag::ALL` order, so reordering that changes the
    /// exported bytes. The expected lines were printed by the `Value`-tree
    /// writer, not written by hand.
    #[test]
    fn every_reading_shape_exports_byte_for_byte() {
        const TS: &str = "2026-09-15T14:30:05.123456789+02:00";
        let mut rows = vec![
            Shape::new(
                "overload",
                reading(MeasuredValue::Overload),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":\"OL\",\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
            Shape::new(
                "ncv",
                reading(MeasuredValue::NcvLevel(3)),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":{\"ncv_level\":3},\"unit\":\"V\",\
                \"range\":\"22V\",\"display_raw\":\"  5.678\",\"progress\":0,\
                \"experimental\":false,\"flags\":{\"hold\":false,\"rel\":false,\
                \"auto_range\":false,\"min\":false,\"max\":false,\"avg\":false,\
                \"low_battery\":false,\"hv_warning\":false,\"peak_max\":false,\
                \"peak_min\":false,\"lead_error\":false,\"comp\":false,\"record\":false,\
                \"loz\":false,\"void\":false,\"dc\":false}}",
            ),
            Shape::new(
                "no reading",
                reading(MeasuredValue::NoReading("Auto")),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":null,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
            Shape::new(
                "absent",
                reading(MeasuredValue::Absent),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":null,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
        ];

        let mut m = reading(MeasuredValue::Normal(1.5));
        m.display_raw = None;
        m.progress = None;
        rows.push(Shape::new(
            "no display_raw, no progress",
            m,
            "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
            \"mode\":\"DC V\",\"value\":1.5,\"unit\":\"V\",\"range\":\"22V\",\
            \"display_raw\":null,\"progress\":null,\"experimental\":false,\
            \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
            \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
            \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
            \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
            \"void\":false,\"dc\":false}}",
        ));

        let every_flag = StatusFlags {
            hold: true,
            rel: true,
            min: true,
            max: true,
            avg: true,
            auto_range: true,
            low_battery: true,
            hv_warning: true,
            dc: true,
            peak_max: true,
            peak_min: true,
            lead_error: true,
            comp: true,
            record: true,
            loz: true,
            void: true,
        };
        rows.push(Shape::new(
            "every flag",
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", every_flag),
            "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
            \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
            \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
            \"flags\":{\"hold\":true,\"rel\":true,\"auto_range\":true,\"min\":true,\
            \"max\":true,\"avg\":true,\"low_battery\":true,\"hv_warning\":true,\
            \"peak_max\":true,\"peak_min\":true,\"lead_error\":true,\"comp\":true,\
            \"record\":true,\"loz\":true,\"void\":true,\"dc\":true}}",
        ));

        rows.push(Shape {
            experimental: true,
            ..Shape::new(
                "experimental",
                reading(MeasuredValue::Normal(5.678)),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":true,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            )
        });

        // Unit falling back to the main one, elapsed seconds, OL, NCV, a
        // float without digits, digits with a space after the sign, none.
        let mut m = reading(MeasuredValue::Normal(5.678));
        m.aux_values = vec![
            aux(MeasuredValue::Normal(5.7), "", Some(" 5.700")),
            AuxValue {
                elapsed_secs: Some(12),
                ..aux(MeasuredValue::Normal(0.25), "Hz", Some("0.250"))
            },
            aux(MeasuredValue::Overload, "", Some(" 9.999")),
            aux(MeasuredValue::NcvLevel(2), "", None),
            aux(MeasuredValue::Normal(0.1), "mV", None),
            aux(MeasuredValue::Normal(-1.2), "", Some("- 1.2")),
            aux(MeasuredValue::Absent, "", None),
        ];
        rows.push(Shape::new(
            "sub-values",
            m,
            "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
            \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
            \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
            \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
            \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
            \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
            \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
            \"void\":false,\"dc\":false},\"aux\":[{\"label\":\"Max\",\
            \"value\":\"5.700\",\"unit\":\"V\",\
            \"elapsed_secs\":null},{\"label\":\"Max\",\"value\":\"0.250\",\
            \"unit\":\"Hz\",\"elapsed_secs\":12},{\"label\":\"Max\",\
            \"value\":\"OL\",\"unit\":\"V\",\
            \"elapsed_secs\":null},{\"label\":\"Max\",\"value\":\"NCV:2\",\
            \"unit\":\"V\",\"elapsed_secs\":null},{\"label\":\"Max\",\
            \"value\":\"0.1\",\"unit\":\"mV\",\
            \"elapsed_secs\":null},{\"label\":\"Max\",\"value\":\"-1.2\",\
            \"unit\":\"V\",\"elapsed_secs\":null},{\"label\":\"Max\",\"value\":null,\
            \"unit\":\"V\",\"elapsed_secs\":null}]}",
        ));

        rows.push(Shape {
            integral: Some((1.5e-7, "C")),
            ..Shape::new(
                "integral",
                reading(MeasuredValue::Normal(5.678)),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false},\"integral\":1.5e-7,\
                \"integral_unit\":\"C\"}",
            )
        });
        // The key stays, its value null.
        rows.push(Shape {
            integral: Some((f64::NAN, "V·s")),
            ..Shape::new(
                "NaN integral",
                reading(MeasuredValue::Normal(5.678)),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false},\"integral\":null,\
                \"integral_unit\":\"V·s\"}",
            )
        });
        rows.push(Shape {
            marker: Some((7, "say \"hi\" \\ then\nnext\u{1}\u{2028}Ω")),
            ..Shape::new(
                "escaped note",
                reading(MeasuredValue::Normal(5.678)),
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false},\"marker\":7,\
                \"note\":\"say \\\"hi\\\" \\\\ then\\nnext\\u0001\u{2028}Ω\"}",
            )
        });

        let mut m = reading(MeasuredValue::Normal(5.678));
        m.aux_values = vec![aux(MeasuredValue::Normal(5.7), "", Some(" 5.700"))];
        rows.push(Shape {
            integral: Some((2.5, "V·s")),
            marker: Some((1, "a")),
            ..Shape::new(
                "aux, integral and marker",
                m,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5.678,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false},\"aux\":[{\"label\":\"Max\",\
                \"value\":\"5.700\",\"unit\":\"V\",\"elapsed_secs\":null}],\
                \"integral\":2.5,\"integral_unit\":\"V·s\",\"marker\":1,\"note\":\"a\"}",
            )
        });

        for (label, v, expected) in [
            (
                "-0.0",
                -0.0,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":-0.0,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":null,\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
            (
                "0.1 + 0.2",
                0.1 + 0.2,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":0.30000000000000004,\"unit\":\"V\",\
                \"range\":\"22V\",\"display_raw\":null,\"progress\":0,\
                \"experimental\":false,\"flags\":{\"hold\":false,\"rel\":false,\
                \"auto_range\":false,\"min\":false,\"max\":false,\"avg\":false,\
                \"low_battery\":false,\"hv_warning\":false,\"peak_max\":false,\
                \"peak_min\":false,\"lead_error\":false,\"comp\":false,\"record\":false,\
                \"loz\":false,\"void\":false,\"dc\":false}}",
            ),
            (
                "1e21",
                1e21,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":1e+21,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":null,\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
            (
                "5e-324",
                5e-324,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":5e-324,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":null,\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
            (
                "infinity",
                f64::INFINITY,
                "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
                \"mode\":\"DC V\",\"value\":null,\"unit\":\"V\",\"range\":\"22V\",\
                \"display_raw\":null,\"progress\":0,\"experimental\":false,\
                \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
                \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
                \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
                \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
                \"void\":false,\"dc\":false}}",
            ),
        ] {
            let mut m = reading(MeasuredValue::Normal(v));
            m.display_raw = None;
            rows.push(Shape::new(label, m, expected));
        }

        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(21.5), "°C", StatusFlags::default());
        m.mode = "Temp °C".into();
        m.range_label = "".into();
        m.aux_values = vec![aux(MeasuredValue::Normal(3.3), "µA", Some("3.3"))];
        rows.push(Shape::new(
            "°C and µA",
            m,
            "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\
            \"mode\":\"Temp °C\",\"value\":21.5,\"unit\":\"°C\",\"range\":\"\",\
            \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
            \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
            \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
            \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
            \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
            \"void\":false,\"dc\":false},\"aux\":[{\"label\":\"Max\",\
            \"value\":\"3.3\",\"unit\":\"µA\",\"elapsed_secs\":null}]}",
        ));
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.234), "kΩ", StatusFlags::default());
        m.mode = "Ω".into();
        m.range_label = "2.2kΩ".into();
        rows.push(Shape::new(
            "Ω",
            m,
            "{\"timestamp\":\"2026-09-15T14:30:05.123456789+02:00\",\"mode\":\"Ω\",\
            \"value\":1.234,\"unit\":\"kΩ\",\"range\":\"2.2kΩ\",\
            \"display_raw\":\"  5.678\",\"progress\":0,\"experimental\":false,\
            \"flags\":{\"hold\":false,\"rel\":false,\"auto_range\":false,\
            \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\
            \"hv_warning\":false,\"peak_max\":false,\"peak_min\":false,\
            \"lead_error\":false,\"comp\":false,\"record\":false,\"loz\":false,\
            \"void\":false,\"dc\":false}}",
        ));

        for row in &rows {
            let got = line(&row.m, TS, row.experimental, row.integral, row.marker);
            assert_eq!(got, row.expected, "{}", row.label);
        }

        let got = metadata_line("Say \"UT\" \\ 61", None);
        assert_eq!(
            got,
            "{\"_metadata\":{\"device\":\"Say \\\"UT\\\" \\\\ 61\"}}"
        );
    }

    /// The shape both binaries write, spelled out: the CLI's `read --format
    /// json` and the GUI's JSON export are this line, whichever wrote it.
    #[test]
    fn a_reading_exports_as_one_object_per_line() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(5.678),
            "V",
            StatusFlags {
                hold: true,
                ..Default::default()
            },
        );
        let line = line(&m, "2026-09-15T14:30:05+02:00", false, None, None);
        assert_eq!(
            line,
            "{\"timestamp\":\"2026-09-15T14:30:05+02:00\",\"mode\":\"DC V\",\"value\":5.678,\
             \"unit\":\"V\",\"range\":\"22V\",\"display_raw\":\"  5.678\",\"progress\":0,\
             \"experimental\":false,\"flags\":{\"hold\":true,\"rel\":false,\"auto_range\":false,\
             \"min\":false,\"max\":false,\"avg\":false,\"low_battery\":false,\"hv_warning\":false,\
             \"peak_max\":false,\"peak_min\":false,\"lead_error\":false,\"comp\":false,\
             \"record\":false,\"loz\":false,\"void\":false,\"dc\":false}}"
        );
        assert_eq!(
            metadata_line("UNI-T UT61E+", None),
            "{\"_metadata\":{\"device\":\"UNI-T UT61E+\"}}"
        );
    }

    /// A word the meter shows instead of a reading exports a null value, the
    /// key kept, and the word in "mode"; a sub-value showing one likewise.
    #[test]
    fn a_no_reading_exports_a_null_value() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::NoReading("Auto"), "", StatusFlags::default());
        m.mode = "Auto".into();
        m.display_raw = None;
        m.aux_values = vec![dmm_lib::measurement::AuxValue {
            label: "Raw".into(),
            value: MeasuredValue::NoReading("Auto"),
            unit: "".into(),
            display_raw: None,
            elapsed_secs: None,
        }];
        let line = line(&m, "2026-09-15T14:30:05+02:00", false, None, None);
        assert!(
            line.starts_with(
                "{\"timestamp\":\"2026-09-15T14:30:05+02:00\",\"mode\":\"Auto\",\
                 \"value\":null,\"unit\":\"\","
            ),
            "got {line}"
        );
        let v = parsed(&line);
        assert_eq!(v["aux"][0]["value"], Value::Null);
        assert_eq!(v["aux"][0]["label"], json!("Raw"));
    }

    /// A frame carrying only the AC component of an AC+DC reading: a null
    /// main value, the component in "aux", and a transform's `Raw` beside it
    /// null too.
    #[test]
    fn a_frame_without_a_main_reading_exports_a_null_value() {
        let mut m = Measurement::test_fixture(MeasuredValue::Absent, "V", StatusFlags::default());
        m.mode = "AC+DC V".into();
        m.display_raw = None;
        m.aux_values = vec![
            dmm_lib::measurement::AuxValue {
                label: "AC".into(),
                value: MeasuredValue::Normal(0.0123),
                unit: "".into(),
                display_raw: Some(" 0.0123".to_string()),
                elapsed_secs: None,
            },
            dmm_lib::measurement::AuxValue {
                label: "Raw".into(),
                value: MeasuredValue::Absent,
                unit: "V".into(),
                display_raw: None,
                elapsed_secs: None,
            },
        ];
        let v = parsed(&line(&m, "2026-09-26T14:35:03+02:00", false, None, None));
        assert_eq!(v["value"], Value::Null);
        assert_eq!(v["aux"][0]["label"], json!("AC"));
        assert_eq!(v["aux"][0]["value"], json!("0.0123"));
        assert_eq!(v["aux"][0]["unit"], json!("V"));
        assert_eq!(v["aux"][1]["value"], Value::Null);
    }

    /// A marked reading ends with its marker's number and note; an unmarked
    /// one carries neither key.
    #[test]
    fn a_marked_reading_carries_its_marker_last() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let marked = line(&m, "ts", false, None, Some((3, "load on")));
        assert!(
            marked.ends_with(",\"marker\":3,\"note\":\"load on\"}"),
            "got {marked}"
        );
        let v = parsed(&line(&m, "ts", false, None, None));
        assert!(v.get("marker").is_none() && v.get("note").is_none(), "{v}");
    }

    /// A model or mode name goes into the file name as one word: the dialog
    /// opens on a name the user can save as typed, not one carrying a path
    /// separator.
    #[test]
    fn a_name_is_folded_into_one_file_name_word() {
        assert_eq!(file_safe("Mock UT61E+"), "Mock-UT61E+");
        assert_eq!(file_safe("UT61E+ / UT61B+"), "UT61E+-UT61B+");
        // A slash with no space around it is still a word boundary: the
        // registry's own `UT171A/B/C` used to come out as `UT171ABC`.
        assert_eq!(file_safe("UT171A/B/C"), "UT171A-B-C");
        assert_eq!(file_safe("DC:V\\A"), "DCV-A");
        assert_eq!(file_safe("DC V"), "DC-V");
        assert_eq!(file_safe("\u{3a9}"), "\u{3a9}");
    }
}
