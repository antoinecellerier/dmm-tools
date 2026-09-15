//! What an export is called and what its JSON holds, wherever it is written
//! from.
//!
//! `dmm-gui`'s Export… and `dmm-cli read` write the same objects under the
//! same names because both come through here: a script reading one file works
//! on the other, and a field added to a reading reaches both binaries at once.

use chrono::{DateTime, Local};
use dmm_lib::measurement::{MeasuredValue, Measurement};
use serde_json::{Value, json};

/// The `_metadata` object a JSON export opens with, as its line — without the
/// newline that ends it.
pub fn metadata_line(device_model: &str) -> String {
    json!({"_metadata": {"device": device_model}}).to_string()
}

/// One reading, as a JSON export writes it.
///
/// `timestamp_rfc3339` is the wall time the reading was taken at, which only
/// the caller can work out: the CLI derives it from the session's clock
/// origin, the GUI stamped it onto the sample as it arrived.
pub fn measurement_json(
    m: &Measurement,
    timestamp_rfc3339: &str,
    experimental: bool,
    integral: Option<(f64, &str)>,
) -> Value {
    let value = match &m.value {
        MeasuredValue::Normal(v) => json!(v),
        MeasuredValue::Overload => json!("OL"),
        MeasuredValue::NcvLevel(l) => json!({"ncv_level": l}),
    };
    // Built from StatusFlags::as_pairs rather than a hand-written list: the
    // old list had drifted and was missing `loz` and `void`, so a VC-890
    // reading the meter had marked invalid was indistinguishable from a good
    // one in JSON — while the text and CSV formats reported it.
    let flags: serde_json::Map<String, Value> = m
        .flags
        .as_pairs()
        .into_iter()
        .map(|(name, set)| (name.to_string(), json!(set)))
        .collect();
    let mut obj = json!({
        "timestamp": timestamp_rfc3339,
        "mode": m.mode,
        "value": value,
        "unit": m.unit,
        "range": m.range_label,
        "display_raw": m.display_raw,
        "progress": m.progress,
        "experimental": experimental,
        "flags": flags,
    });
    // Omitted entirely when there are none, so output for the families that
    // never produce sub-values is unchanged.
    if !m.aux_values.is_empty() {
        obj["aux"] = json!(
            m.aux_values
                .iter()
                .map(|aux| {
                    let unit = aux.unit_or(&m.unit);
                    json!({
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
        obj["integral"] = json!(val);
        obj["integral_unit"] = json!(unit);
    }
    obj
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
pub fn file_safe(name: &str) -> String {
    name.replace(['/', '\\', ':'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use dmm_lib::flags::StatusFlags;

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
        let line = measurement_json(&m, "2026-09-15T14:30:05+02:00", false, None).to_string();
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
            metadata_line("UNI-T UT61E+"),
            "{\"_metadata\":{\"device\":\"UNI-T UT61E+\"}}"
        );
    }

    /// A model or mode name goes into the file name as one word: the dialog
    /// opens on a name the user can save as typed, not one carrying a path
    /// separator.
    #[test]
    fn a_name_is_folded_into_one_file_name_word() {
        assert_eq!(file_safe("Mock UT61E+"), "Mock-UT61E+");
        assert_eq!(file_safe("UT61E+ / UT61B+"), "UT61E+-UT61B+");
        assert_eq!(file_safe("DC V"), "DC-V");
        assert_eq!(file_safe("\u{3a9}"), "\u{3a9}");
    }
}
