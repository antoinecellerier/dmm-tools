//! The mock reaches every command through the same dispatch arm as a real
//! meter.
//!
//! `read`, `get`, `set` and `command` used to have a second set of arms in
//! `main`, guarded on `!requires_hardware`, that called the same functions
//! with different arguments. Those functions each pick the mock transport
//! themselves, so the arms were removed — and only a run of the binary
//! covers the dispatch, which no unit test can reach.

use std::process::Command;
use std::time::Instant;

fn run(args: &[&str]) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_dmm-cli"))
        .args(args)
        .output()
        .expect("run dmm-cli");
    (
        String::from_utf8(out.stdout).expect("utf-8 stdout"),
        String::from_utf8(out.stderr).expect("utf-8 stderr"),
    )
}

#[test]
fn read_prints_one_line_per_sample_and_a_summary() {
    let (stdout, stderr) = run(&["--device", "mock", "read", "--count", "2"]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "one line per reading: {stdout}");
    for line in &lines {
        assert!(line.ends_with(" V [AUTO]"), "got {line}");
    }
    assert!(stderr.contains("2 samples | Min:"), "got {stderr}");
}

/// `--mock-mode` reaching the mock is the part the removed arms carried: the
/// surviving arm used to pass `None` here.
#[test]
fn read_honours_mock_mode() {
    let (stdout, _) = run(&[
        "--device",
        "mock",
        "read",
        "--mock-mode",
        "ohm",
        "--count",
        "1",
        "--format",
        "json",
    ]);
    let mut lines = stdout.lines();
    let header: serde_json::Value =
        serde_json::from_str(lines.next().expect("metadata line")).expect("metadata json");
    assert!(header["_metadata"]["device"].is_string(), "got {header}");
    let reading: serde_json::Value =
        serde_json::from_str(lines.next().expect("reading line")).expect("reading json");
    assert_eq!(reading["mode"], "\u{3a9}");
    assert_eq!(reading["unit"], "k\u{3a9}");
}

/// The mock answers instantly, so the read loop floors a `0` interval at
/// 100 ms; without it `--count` would spin as fast as the CPU allows.
#[test]
fn read_paces_the_mock_at_the_interval_floor() {
    let started = Instant::now();
    run(&["--device", "mock", "read", "--count", "5"]);
    assert!(
        started.elapsed().as_millis() >= 300,
        "five samples returned in {:?}",
        started.elapsed()
    );
}

#[test]
fn get_and_set_honour_mock_mode() {
    let (listing, _) = run(&["--device", "mock", "get", "range", "--mock-mode", "ohm"]);
    assert!(listing.contains("k\u{3a9}"), "got {listing}");

    let (switched, _) = run(&[
        "--device",
        "mock",
        "set",
        "range",
        "22kohm",
        "--mock-mode",
        "ohm",
    ]);
    assert!(switched.contains("22k\u{3a9}"), "got {switched}");
}

#[test]
fn command_sends_to_the_mock() {
    let (stdout, _) = run(&["--device", "mock", "command", "hold"]);
    assert_eq!(stdout.trim(), "Sent hold");
}
