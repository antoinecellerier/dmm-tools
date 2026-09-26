//! Golden file tests for measurement parsing.
//!
//! Every subdirectory of `tests/golden/` is named after a registry device id
//! (`ut61eplus`, `ut804`, …) and its `.yaml` files are parsed by that device's
//! [`Protocol::parse_payload`]. The format is a capture report's sample
//! format, so a sample can be copied out of a report unchanged:
//! - `raw_hex`: the sample's `raw_hex` — the payload the reading was parsed from
//! - `mode`, `value`, `unit`, `range_label`, `flags`: expected parsed fields
//!
//! A fixture must also parse without anything reported as unrecognised.
//!
//! The `value` field is a string matching capture output:
//! - Numeric: `"5.678"`, `"-12.345"`
//! - Overload: `"OL"`
//! - NCV: `"NCV:3"`
//! - No main reading in this frame, only sub-values: `""`
//!
//! `aux`, when present, lists the sub-values the reading must carry — no more,
//! so `aux: []` checks there are none — as the capture report stores them:
//! `label`, `value` (the display form), and `unit` resolved against the
//! reading's.
//!
//! `resolution`, when present, is the resolution of the manual row
//! [`Protocol::spec_info`] finds for the reading.
//!
//! `flags` is a map of snake_case flag name (the names [`Flag::name`] returns)
//! to the expected bool. A fixture may list only the flags it cares about:
//! every name it omits is expected to be false. An unknown name fails the
//! test rather than being ignored, so a typo can't silently check nothing.

use dmm_lib::flags::Flag;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::resolve_device;
use dmm_lib::protocol::{Protocol, capture_reports};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Resolution of the reading's spec row, checked when given.
    #[serde(default)]
    resolution: Option<String>,
    /// The reading's sub-values, checked when given.
    #[serde(default)]
    aux: Option<Vec<GoldenAux>>,
}

/// One expected sub-value, in the capture report's shape.
#[derive(Debug, Deserialize, PartialEq)]
struct GoldenAux {
    label: String,
    value: String,
    unit: String,
}

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
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

/// The device id each golden subdirectory is named after, sorted.
fn golden_device_dirs() -> Vec<(String, PathBuf)> {
    let root = golden_root();
    let mut dirs: Vec<_> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read golden dir {}: {e}", root.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?.to_string();
            path.is_dir().then_some((name, path))
        })
        .collect();
    dirs.sort();
    dirs
}

/// Discover all `.yaml` golden files in the given directory.
fn discover_golden_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read golden dir {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().and_then(|s| s.to_str()) == Some("yaml")).then_some(path)
        })
        .collect();
    files.sort();
    files
}

/// Read one fixture file.
fn load_case(path: &Path) -> GoldenTestCase {
    let yaml_str = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_yaml_ng::from_str(&yaml_str)
        .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

/// Check one fixture against what its device's parser produces.
fn check_fixture(protocol: &dyn Protocol, stem: &str, path: &Path) {
    let case = load_case(path);
    let measurement = check_reading(protocol, stem, &case);
    if let Some(resolution) = &case.resolution {
        assert_eq!(
            protocol.spec_info(&measurement).map(|s| s.resolution),
            Some(resolution.as_str()),
            "golden {stem}: resolution mismatch"
        );
    }
}

/// Check a fixture's reading, everything but the resolution, and return it.
fn check_reading(protocol: &dyn Protocol, stem: &str, case: &GoldenTestCase) -> Measurement {
    let payload = decode_hex(&case.raw_hex);
    let (parsed, reports) = capture_reports(|| protocol.parse_payload(&payload));
    let measurement = parsed.unwrap_or_else(|e| panic!("golden {stem}: parse failed: {e}"));
    // A captured frame is known data by definition: a report here means a
    // parser check is wrong, and would warn every user who sends this frame.
    assert!(
        reports.is_empty(),
        "golden {stem}: reported as unrecognised: {reports:?}"
    );

    assert_eq!(measurement.mode, case.mode, "golden {stem}: mode mismatch");
    assert_eq!(
        measurement.value.to_string(),
        case.value,
        "golden {stem}: value mismatch"
    );
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
    assert_known_flag_names(stem, &case.flags);
    if let Some(aux) = &case.aux {
        let got: Vec<GoldenAux> = measurement
            .aux_values
            .iter()
            .map(|a| GoldenAux {
                label: a.label.to_string(),
                value: a.value_str().into_owned(),
                unit: a.unit_or(&measurement.unit).to_string(),
            })
            .collect();
        assert_eq!(&got, aux, "golden {stem}: aux mismatch");
    }
    measurement
}

#[test]
fn golden_fixtures_parse_as_recorded() {
    let dirs = golden_device_dirs();
    assert!(
        !dirs.is_empty(),
        "no golden directories in {}",
        golden_root().display()
    );

    let mut passed = 0;
    for (id, dir) in &dirs {
        let device = resolve_device(id).unwrap_or_else(|| {
            panic!("golden directory {id:?} is not a device id in the registry")
        });
        let protocol = (device.new_protocol)();

        let files = discover_golden_files(dir);
        assert!(!files.is_empty(), "no golden files in {}", dir.display());
        for path in &files {
            let stem = format!("{id}/{}", path.file_stem().unwrap().to_string_lossy());
            check_fixture(protocol.as_ref(), &stem, path);
            passed += 1;
        }
    }

    eprintln!(
        "golden: {passed} fixtures passed across {} devices",
        dirs.len()
    );
}

/// The UT71C/D/E sends the UT804's packets and shows the UT804's range
/// labels (docs/research/ut71/reverse-engineered-protocol.md §2, §3.5), so
/// every UT804 fixture reads the same under it. It has no spec tables, so
/// the resolution is left out.
#[test]
fn ut804_fixtures_read_the_same_as_a_ut71cde() {
    let device = resolve_device("ut71cde").expect("ut71cde is in the registry");
    let protocol = (device.new_protocol)();
    let files = discover_golden_files(&golden_root().join("ut804"));
    assert!(!files.is_empty(), "no UT804 golden files");
    for path in &files {
        let stem = format!(
            "ut804/{} as ut71cde",
            path.file_stem().unwrap().to_string_lossy()
        );
        check_reading(protocol.as_ref(), &stem, &load_case(path));
    }
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
