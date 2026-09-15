//! `read --replay` plays a recording back as a session, and `read --record`
//! is refused where there is no meter to record from.
//!
//! The point of a replay is that it prints the same thing every time — a doc
//! snippet is generated from one — so the timestamps are asserted literally
//! rather than by shape, and a second run is compared byte for byte.
//!
//! `--record` against a meter has no test here: it needs the cable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Three real UT61E+ frames, from the golden fixtures in dmm-lib
/// (`dcv_battery`, `dcv_negative`, `dcv_negative_zero`), 100 ms apart.
const RECORDING: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
100 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
200 02 31 2D 20 30 2E 30 30 30 00 00 30 34 31
";

/// What the three frames export as, in the RFC3339 form `read` prints: the
/// `# recorded:` header plus each frame's own offset.
const TIMESTAMPS: [&str; 3] = [
    "2026-09-02T10:00:00+00:00",
    "2026-09-02T10:00:00.100+00:00",
    "2026-09-02T10:00:00.200+00:00",
];

/// Two frames six seconds apart: a meter that stopped answering part-way
/// through the recording, which plays back as a run of timeouts.
const RECORDING_WITH_A_GAP: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
6000 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
";

/// A directory of this test's own, with `text` in it as the recording.
fn recording_of(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dmm-cli-replay-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("bench.replay");
    std::fs::write(&path, text).expect("write recording");
    path
}

/// A directory of this test's own, with the three-frame recording in it.
fn recording_in(name: &str) -> PathBuf {
    recording_of(name, RECORDING)
}

/// Run the binary as a user would, with nothing of the environment left to
/// move the output: `TZ` fixes the printed offset, `NO_COLOR` the styling,
/// and an inherited `RUST_LOG` would add lines of its own.
fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_dmm-cli"))
        .args(args)
        .env("TZ", "UTC")
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

fn read_csv(path: &Path, extra: &[&str]) -> (String, String, bool) {
    let mut args = vec![
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "3",
        "--format",
        "csv",
    ];
    args.extend_from_slice(extra);
    run(&args)
}

#[test]
fn replay_exports_the_times_the_frames_were_recorded_at() {
    let path = recording_in("timestamps");
    let (stdout, _, ok) = read_csv(&path, &[]);
    assert!(ok, "replay failed: {stdout}");

    let rows: Vec<&str> = stdout.lines().skip(2).collect();
    assert_eq!(rows.len(), 3, "one row per frame: {stdout}");
    for (row, expected) in rows.iter().zip(TIMESTAMPS) {
        assert_eq!(row.split(',').next(), Some(expected), "got {row}");
    }
    // The frames decode through the family's own parser, so the values are
    // the ones the goldens carry.
    assert!(rows[0].contains(",DC V,1.6109,V,"), "got {}", rows[0]);
}

#[test]
fn replaying_twice_prints_the_same_bytes() {
    let path = recording_in("deterministic");
    let (first, _, _) = read_csv(&path, &[]);
    let (second, _, _) = read_csv(&path, &[]);
    assert_eq!(first, second);
}

/// The clock flags are the mock's, and a replay is paced the same way — a
/// preseed spends the recording's sleeps at once without moving a timestamp.
#[test]
fn replay_takes_the_clock_flags() {
    let path = recording_in("preseed");
    let (plain, _, _) = read_csv(&path, &[]);
    let (preseeded, _, ok) = read_csv(&path, &["--mock-clock-preseed", "10"]);
    assert!(ok, "preseeded replay failed: {preseeded}");
    assert_eq!(plain, preseeded);
}

/// A gap plays back as the timeouts it was, and they are not a quiet meter:
/// there is no `--device` to check and nothing on the cable to switch a USB
/// mode on. The preseed spends the six seconds of silence without waiting.
#[test]
fn a_gap_in_a_recording_does_not_print_the_no_response_help() {
    let path = recording_of("gap", RECORDING_WITH_A_GAP);
    let (stdout, stderr, ok) = run(&[
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "2",
        "--format",
        "csv",
        "--mock-clock-preseed",
        "10",
    ]);
    assert!(ok, "replay failed: {stderr}");
    assert!(
        !stderr.contains("No response from meter"),
        "the gap was reported as a quiet meter: {stderr}"
    );
    // And the frame on the far side of the gap still plays, at its own time.
    assert!(stdout.contains("2026-09-02T10:00:06+00:00"), "got {stdout}");
}

#[test]
fn record_and_replay_are_refused_together() {
    let path = recording_in("conflict");
    let (_, stderr, ok) = run(&[
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--record",
        "out.replay",
    ]);
    assert!(!ok, "expected a refusal");
    assert!(stderr.contains("--replay"), "got {stderr}");
    assert!(stderr.contains("--record"), "got {stderr}");
}

#[test]
fn recording_the_mock_is_refused() {
    let dir = std::env::temp_dir().join(format!("dmm-cli-replay-{}-mock", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let out = dir.join("out.replay");
    let (_, stderr, ok) = run(&[
        "--device",
        "mock",
        "read",
        "--record",
        out.to_str().expect("utf-8 path"),
        "--count",
        "1",
    ]);
    assert!(!ok, "expected a refusal");
    assert!(
        stderr.contains("--record needs a real meter"),
        "got {stderr}"
    );
    assert!(!out.exists(), "nothing should have been written");
}

/// The file names the meter, so a `--device` on the command line is either
/// redundant or a contradiction.
#[test]
fn naming_a_device_alongside_a_replay_is_refused() {
    let path = recording_in("device");
    let (_, stderr, ok) = run(&[
        "--device",
        "ut61eplus",
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "1",
    ]);
    assert!(!ok, "expected a refusal");
    assert!(stderr.contains("names its own meter"), "got {stderr}");
}
