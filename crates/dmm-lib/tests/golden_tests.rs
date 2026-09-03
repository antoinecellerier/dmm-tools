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
//!
//! `flags` is a map of snake_case flag name (the names [`Flag::name`] returns)
//! to the expected bool. A fixture may list only the flags it cares about:
//! every name it omits is expected to be false. An unknown name fails the
//! test rather than being ignored, so a typo can't silently check nothing.

use dmm_lib::flags::Flag;
use dmm_lib::protocol::ut61eplus::parse_measurement;
use dmm_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

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
    /// Expected flags by snake_case name; omitted names expect false.
    flags: BTreeMap<String, bool>,
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

/// Fail on a fixture flag name no `Flag` answers to.
///
/// Omitted names expect false, so without this a misspelled key would just be
/// ignored — the fixture would look like it checked a flag it never did.
fn assert_known_flag_names(stem: &str, flags: &BTreeMap<String, bool>) {
    for key in flags.keys() {
        assert!(
            Flag::ALL.iter().any(|f| f.name() == key),
            "golden {stem}: unknown flag name {key:?}"
        );
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
        let case: GoldenTestCase = serde_yaml_ng::from_str(&yaml_str)
            .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()));

        let payload = decode_hex(&case.raw_hex);

        let measurement = parse_measurement(&payload, &table)
            .unwrap_or_else(|e| panic!("golden {stem}: parse failed: {e}"));

        assert_eq!(measurement.mode, case.mode, "golden {stem}: mode mismatch");

        let actual_value = measurement.value.to_string();
        assert_eq!(actual_value, case.value, "golden {stem}: value mismatch");

        assert_eq!(measurement.unit, case.unit, "golden {stem}: unit mismatch");
        assert_eq!(
            measurement.range_label, case.range_label,
            "golden {stem}: range_label mismatch"
        );

        // Driven by `as_pairs`, so a flag added to `StatusFlags` is checked by
        // every fixture from the moment it exists — the hand-written list this
        // replaced had never gained `loz` or `void`.
        for (name, actual) in measurement.flags.as_pairs() {
            assert_eq!(
                actual,
                case.flags.get(name).copied().unwrap_or(false),
                "golden {stem}: flags.{name}"
            );
        }
        assert_known_flag_names(&stem, &case.flags);

        passed += 1;
    }

    eprintln!("golden_ut61eplus: {passed}/{} tests passed", files.len());
}

/// A fixture that misspells a flag name must fail rather than quietly expect
/// nothing.
#[test]
#[should_panic(expected = "unknown flag name \"auto_rnage\"")]
fn unknown_fixture_flag_names_fail() {
    let case: GoldenTestCase = serde_yaml_ng::from_str(
        "raw_hex: \"02 31 20 20 35 2E 36 37 38 00 00 30 30 30\"\n\
         mode: DC V\n\
         value: \"5.678\"\n\
         unit: V\n\
         range_label: 22V\n\
         flags:\n  auto_rnage: true\n",
    )
    .expect("synthetic case must parse");
    assert_known_flag_names("typo", &case.flags);
}
