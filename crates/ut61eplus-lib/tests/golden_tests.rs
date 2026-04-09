//! Golden file tests for measurement parsing.
//!
//! Each `.yaml` file in `tests/golden/<family>/` uses the same format as
//! capture YAML samples:
//! - `raw_hex`: hex-encoded raw measurement payload (spaces allowed)
//! - `mode`, `value`, `unit`, `range_label`, `flags`: expected parsed fields
//!
//! The `value` field is a string matching capture output:
//! - Numeric: `"5.678"`, `"-12.345"`
//! - Overload: `"OL"`
//! - NCV: `"NCV:3"`

use serde::Deserialize;
use std::path::Path;
use ut61eplus_lib::measurement::MeasuredValue;
use ut61eplus_lib::protocol::ut61eplus::parse_measurement;
use ut61eplus_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;

/// Expected flag state (same field names as capture SampleFlags).
#[derive(Debug, Deserialize)]
struct ExpectedFlags {
    hold: bool,
    rel: bool,
    auto_range: bool,
    min: bool,
    max: bool,
    low_battery: bool,
    hv_warning: bool,
    dc: bool,
    peak_min: bool,
    peak_max: bool,
    #[serde(default)]
    lead_error: bool,
    #[serde(default)]
    comp: bool,
    #[serde(default)]
    record: bool,
}

/// A golden test case in capture-compatible YAML format.
#[derive(Debug, Deserialize)]
struct GoldenTestCase {
    /// Hex-encoded payload (spaces stripped before decoding).
    raw_hex: String,
    mode: String,
    /// Value as string: "5.678", "OL", "NCV:3"
    value: String,
    unit: String,
    range_label: String,
    flags: ExpectedFlags,
}

/// Decode a hex string (with optional spaces) into bytes.
fn decode_hex(hex: &str) -> Vec<u8> {
    let clean: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        clean.len().is_multiple_of(2),
        "hex string has odd length: {}\n  cleaned hex: {clean}",
        clean.len()
    );
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .unwrap_or_else(|e| panic!("invalid hex at offset {i}: {e}\n  hex: {clean}"))
        })
        .collect()
}

/// Format a MeasuredValue as a string matching capture output.
fn format_value(v: &MeasuredValue) -> String {
    match v {
        MeasuredValue::Normal(v) => format!("{v}"),
        MeasuredValue::Overload => "OL".to_string(),
        MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
    }
}

/// Discover all `.yaml` golden files in the given directory.
fn discover_golden_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read golden dir {}: {e}", dir.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

#[test]
fn golden_ut61eplus() {
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/ut61eplus");
    let files = discover_golden_files(&golden_dir);
    assert!(
        !files.is_empty(),
        "no golden files found in {}",
        golden_dir.display()
    );

    let table = Ut61ePlusTable::new();
    let mut passed = 0;

    for path in &files {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let yaml_str = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let case: GoldenTestCase = serde_yaml::from_str(&yaml_str)
            .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()));

        let payload = decode_hex(&case.raw_hex);

        let measurement = parse_measurement(&payload, &table)
            .unwrap_or_else(|e| panic!("golden {stem}: parse failed: {e}"));

        assert_eq!(measurement.mode, case.mode, "golden {stem}: mode mismatch");

        let actual_value = format_value(&measurement.value);
        assert_eq!(actual_value, case.value, "golden {stem}: value mismatch");

        assert_eq!(measurement.unit, case.unit, "golden {stem}: unit mismatch");
        assert_eq!(
            measurement.range_label, case.range_label,
            "golden {stem}: range_label mismatch"
        );

        let f = &measurement.flags;
        let ef = &case.flags;
        assert_eq!(f.hold, ef.hold, "golden {stem}: flags.hold");
        assert_eq!(f.rel, ef.rel, "golden {stem}: flags.rel");
        assert_eq!(
            f.auto_range, ef.auto_range,
            "golden {stem}: flags.auto_range"
        );
        assert_eq!(f.min, ef.min, "golden {stem}: flags.min");
        assert_eq!(f.max, ef.max, "golden {stem}: flags.max");
        assert_eq!(
            f.low_battery, ef.low_battery,
            "golden {stem}: flags.low_battery"
        );
        assert_eq!(
            f.hv_warning, ef.hv_warning,
            "golden {stem}: flags.hv_warning"
        );
        assert_eq!(f.dc, ef.dc, "golden {stem}: flags.dc");
        assert_eq!(f.peak_min, ef.peak_min, "golden {stem}: flags.peak_min");
        assert_eq!(f.peak_max, ef.peak_max, "golden {stem}: flags.peak_max");

        passed += 1;
    }

    eprintln!("golden_ut61eplus: {passed}/{} tests passed", files.len());
}
