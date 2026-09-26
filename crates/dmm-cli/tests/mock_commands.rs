//! The mock reaches every command through the same dispatch arm as a real
//! meter.
//!
//! `read`, `get`, `set` and `command` used to have a second set of arms in
//! `main`, guarded on `!requires_hardware`, that called the same functions
//! with different arguments. Those functions each pick the mock transport
//! themselves, so the arms were removed — and only a run of the binary
//! covers the dispatch, which no unit test can reach.

mod common;

use common::dmm_cli;
use std::time::Instant;

fn run(args: &[&str]) -> (String, String) {
    let out = dmm_cli().args(args).output().expect("run dmm-cli");
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

/// `auto` is a `--device` value like any other, and the one the registry does
/// not carry: it has to reach the open path rather than be rejected as an
/// unknown device.
///
/// Pinned to an adapter that cannot exist so the run stays hermetic — without
/// it, a machine with a meter plugged in would have that meter probed by the
/// test suite.
#[test]
fn auto_is_a_device_value_and_reaches_the_usb_cable() {
    let (_, stderr) = run(&["--device", "auto", "--adapter", "no-such-adapter", "info"]);
    assert!(
        !stderr.contains("unknown device"),
        "auto was rejected before anything was opened: {stderr}"
    );
    assert!(stderr.contains("adapter not found"), "got {stderr}");
}

/// Naming the mock still picks it, rather than probing for a meter: the
/// dispatch every command goes through now resolves `auto` too.
#[test]
fn a_named_device_is_not_detected() {
    let (stdout, stderr) = run(&["--device", "mock", "command", "hold"]);
    assert_eq!(stdout.trim(), "Sent hold");
    assert!(!stderr.contains("Detected"), "got {stderr}");
    assert!(!stderr.contains("Auto-detecting"), "got {stderr}");
}

/// The ZT-5B mock opens as itself: its own keys, its AUTO word, and a
/// refusal the ZOTEK driver makes rather than the UT61E+ mock's.
#[test]
fn the_zt5b_mock_opens_as_its_own_device() {
    let (stdout, _) = run(&["--device", "mock-zt5b", "read", "--count", "1"]);
    assert_eq!(stdout.trim(), "Auto", "got {stdout}");
    let (stdout, _) = run(&["--device", "mock-zt5b", "command", "volts"]);
    assert_eq!(stdout.trim(), "Sent volts");
    let (_, stderr) = run(&["--device", "mock-zt5b", "command", "zero"]);
    assert!(
        stderr.contains("ZERO works in capacitance only"),
        "got {stderr}"
    );
}
