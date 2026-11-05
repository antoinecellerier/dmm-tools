//! `read --import` reads an export back: it converts it, or summarises it,
//! with its markers, without waiting.

mod common;

use common::dmm_cli;
use std::path::{Path, PathBuf};

/// Three UT61E+ frames 100 ms apart with a marker on the second.
const RECORDING: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
100 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
# marker: 100 2 load on, 2.2 ohm
200 02 31 2D 20 30 2E 30 30 30 00 00 30 34 31
";

/// A directory of this test's own, removed when it passes.
struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("dmm-cli-import-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn run(args: &[&str]) -> (String, String, bool) {
    let out = dmm_cli()
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("RUST_LOG")
        .output()
        .expect("run dmm-cli");
    (
        String::from_utf8(out.stdout).expect("utf-8 stdout"),
        String::from_utf8(out.stderr).expect("utf-8 stderr"),
        out.status.success(),
    )
}

fn p(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

/// The recording converted to `format` by `read --import`, the replay way.
fn converted(dir: &TestDir, format: &str) -> PathBuf {
    let replay = dir.path("bench.replay");
    std::fs::write(&replay, RECORDING).expect("write recording");
    let out = dir.path(&format!("bench.{format}"));
    let (_, err, ok) = run(&["read", "--import", p(&replay), "-o", p(&out)]);
    assert!(ok, "the replay converts: {err}");
    out
}

/// A CSV or JSON export read back and written in its own format again comes
/// out byte for byte, marker included.
#[test]
fn an_export_reads_back_to_the_same_bytes() {
    let dir = TestDir::new("same-bytes");
    for format in ["csv", "json"] {
        let first = converted(&dir, format);
        let again = dir.path(&format!("again.{format}"));
        let (_, err, ok) = run(&["read", "--import", p(&first), "-o", p(&again)]);
        assert!(ok, "{format} imports: {err}");
        let (first, again) = (
            std::fs::read_to_string(first).expect("first"),
            std::fs::read_to_string(again).expect("again"),
        );
        assert!(first.contains("load on, 2.2 ohm"), "{first}");
        assert_eq!(again, first, "{format}");
    }
}

/// CSV to JSON keeps the readings and the marker, and the summary closes the
/// run as it does a meter's.
#[test]
fn a_csv_export_converts_to_json() {
    let dir = TestDir::new("csv-to-json");
    let csv = converted(&dir, "csv");
    let (json, err, ok) = run(&["read", "--import", p(&csv), "--format", "json"]);
    assert!(ok, "converts: {err}");
    let lines: Vec<serde_json::Value> = json
        .lines()
        .map(|l| serde_json::from_str(l).expect("a JSON line"))
        .collect();
    assert_eq!(lines.len(), 4, "metadata and three readings: {json}");
    assert_eq!(lines[2]["marker"], 2);
    assert_eq!(lines[2]["note"], "load on, 2.2 ohm");
    assert!(err.contains("3 readings"), "the summary: {err}");
}

/// A CSV holds no frames, so it cannot become a replay; and its readings are
/// all there is, so nothing is kept per interval.
#[test]
fn a_csv_import_refuses_what_it_cannot_do() {
    let dir = TestDir::new("refusals");
    let csv = converted(&dir, "csv");
    let (_, err, ok) = run(&["read", "--import", p(&csv), "--format", "replay"]);
    assert!(!ok && err.contains("replay"), "{err}");
    let (_, err, ok) = run(&["read", "--import", p(&csv), "--interval-ms", "500"]);
    assert!(!ok && err.contains("--interval-ms"), "{err}");
}

/// A replay imported is a replay played without waiting: the same output as
/// `--replay` at the unpaced clock.
#[test]
fn a_replay_import_is_an_unpaced_replay() {
    let dir = TestDir::new("replay");
    let replay = dir.path("bench.replay");
    std::fs::write(&replay, RECORDING).expect("write recording");
    let (imported, _, ok) = run(&["read", "--import", p(&replay), "--format", "csv"]);
    assert!(ok);
    let (replayed, _, ok) = run(&[
        "read",
        "--replay",
        p(&replay),
        "--mock-clock-scale",
        "max",
        "--format",
        "csv",
    ]);
    assert!(ok);
    assert_eq!(imported, replayed);
}

/// A replay without its extension is still a replay, found by its first
/// line, as the GUI finds it.
#[test]
fn an_extensionless_replay_imports() {
    let dir = TestDir::new("extensionless");
    let replay = dir.path("recording");
    std::fs::write(&replay, RECORDING).expect("write recording");
    let (out, err, ok) = run(&["read", "--import", p(&replay), "--format", "csv"]);
    assert!(ok, "imports: {err}");
    assert_eq!(
        out.lines().count(),
        5,
        "comment, header and three rows: {out}"
    );
}

/// A CSV written with a scale on part-way: a DC row before it, an AC row
/// with Frequency, Period and the scale's `Raw`, a DC row with `Raw` alone.
const SCALED_CSV: &str = "\
# device: UT181A
timestamp,mode,value,unit,range,flags,aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,aux3_label,aux3_value,aux3_unit
2026-09-02T10:00:00+00:00,DC V,1.6109,V,2.2V,AUTO,,,,,,,,,
2026-09-02T10:00:00.100+00:00,AC V,0.5137,A,2.2V,AUTO,Frequency,50.01,Hz,Period,20.00,ms,Raw,51.37,mV
2026-09-02T10:00:00.200+00:00,DC V,0.1611,A,2.2V,AUTO,,,,,,,Raw,16.11,mV
";

/// The cells of each line after the provenance comment, the timestamp left
/// out: it is written in local time.
fn cells_past_timestamp(csv: &str) -> Vec<Vec<String>> {
    csv.lines()
        .skip(1)
        .map(|l| l.split(',').skip(1).map(str::to_string).collect())
        .collect()
}

/// The scale's `Raw` keeps its trailing column through an import: not
/// packed into the meter's columns, beside Frequency on one row and alone
/// under `aux1_*` on the next.
#[test]
fn a_scaled_csv_keeps_the_raw_column_in_place() {
    let dir = TestDir::new("scaled");
    let csv = dir.path("scaled.csv");
    std::fs::write(&csv, SCALED_CSV).expect("write the CSV");
    let (out, err, ok) = run(&["read", "--import", p(&csv), "--format", "csv"]);
    assert!(ok, "imports: {err}");
    assert_eq!(cells_past_timestamp(&out), cells_past_timestamp(SCALED_CSV));

    // A scale on the import appends a second `Raw`, which takes the last
    // column on every row, the file's staying beside it where it was.
    let (out, err, ok) = run(&[
        "read",
        "--import",
        p(&csv),
        "--format",
        "csv",
        "--scale",
        "10",
    ]);
    assert!(ok, "imports scaled: {err}");
    let rows = cells_past_timestamp(&out);
    assert!(rows[0].ends_with(&["aux4_label", "aux4_value", "aux4_unit"].map(String::from)));
    for row in &rows[1..] {
        assert_eq!(row.len(), rows[0].len());
        assert_eq!(row[14], "Raw", "{row:?}");
    }
    assert_eq!(rows[1][11..14], ["", "", ""]);
    assert_eq!(rows[2][11..14], ["Raw", "51.37", "mV"]);
    assert_eq!(rows[3][11..14], ["Raw", "16.11", "mV"]);
}

/// A second at 900 mV, then the meter auto-ranges to V and reads 1.1 V for
/// two seconds.
const STEP_CSV: &str = "\
# device: UT61E+
timestamp,mode,value,unit,range,flags
2026-09-02T10:00:00+00:00,DC V,900.0,mV,2.2V,AUTO
2026-09-02T10:00:01+00:00,DC V,900.0,mV,2.2V,AUTO
2026-09-02T10:00:02+00:00,DC V,1.100,V,22V,AUTO
2026-09-02T10:00:03+00:00,DC V,1.100,V,22V,AUTO
";

/// An auto-range step from mV to V carries the statistics and the integral
/// on, in volts: no reset note, each figure in a range the file showed to
/// that range's decimals, and the integral column continues across it.
#[test]
fn an_auto_range_step_carries_the_statistics_and_integral_on() {
    let dir = TestDir::new("decade-step");
    let csv = dir.path("step.csv");
    std::fs::write(&csv, STEP_CSV).expect("write the CSV");
    let (out, err, ok) = run(&[
        "read",
        "--import",
        p(&csv),
        "--format",
        "csv",
        "--integrate",
    ]);
    assert!(ok, "imports: {err}");
    assert!(!err.contains("changed"), "no reset note: {err}");
    assert!(
        err.contains(
            "4 readings | Min: 900.0 mV | Max: 1.100 V | Mean: 1.000 V | \u{3c3}: 115.5 mV"
        ),
        "the summary: {err}"
    );
    assert!(err.contains("Integral: 3.0000 V\u{b7}s (3.0s)"), "{err}");
    // 0.9 V·s, then 1.0 V·s over the second that straddles the step, then
    // 1.1 V·s: each row in the unit of its own reading.
    let integrals: Vec<(String, String)> = cells_past_timestamp(&out)[1..]
        .iter()
        .map(|row| (row[5].clone(), row[6].clone()))
        .collect();
    let mv_s = "mV\u{b7}s".to_string();
    let v_s = "V\u{b7}s".to_string();
    assert_eq!(
        integrals,
        [
            ("0.000000".to_string(), mv_s.clone()),
            ("900.000000".to_string(), mv_s),
            ("1.900000".to_string(), v_s.clone()),
            ("3.000000".to_string(), v_s),
        ],
        "{out}"
    );
}
