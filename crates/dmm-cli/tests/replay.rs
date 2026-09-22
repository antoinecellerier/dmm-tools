//! `read --replay` plays a recording back as a session, `read --format
//! replay` writes one, and `-o` decides where it lands and what shape it is.
//!
//! The point of a replay is that it prints the same thing every time — a doc
//! snippet is generated from one — so the timestamps are asserted literally
//! rather than by shape, and a second run is compared byte for byte.
//!
//! The tests that assert a timestamp, or a file name carrying one, are Linux
//! and macOS only: they pin `TZ=UTC`, which chrono's `Local` ignores on
//! Windows.
//!
//! `--format replay` against a meter has no test here: it needs the cable.

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
#[cfg(not(windows))]
const TIMESTAMPS: [&str; 3] = [
    "2026-09-02T10:00:00+00:00",
    "2026-09-02T10:00:00.100+00:00",
    "2026-09-02T10:00:00.200+00:00",
];

/// Two frames six seconds apart: a meter that stopped answering part-way
/// through the recording, which plays back as a run of timeouts.
#[cfg(not(windows))]
const RECORDING_WITH_A_GAP: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
6000 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
";

/// A session someone turned the dial during: DC V, then 80.45 kΩ across an
/// 82 kΩ resistor (the `ohm_82k` golden fixture).
#[cfg(not(windows))]
const RECORDING_ACROSS_MODES: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
100 06 33 20 20 38 30 2E 34 35 01 06 30 30 30
";

/// A recording whose only frame is truncated: it parses as a file, but the
/// family refuses every frame in it, so a run of it never gets a reading.
#[cfg(unix)]
const RECORDING_ALL_CORRUPT: &str = "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20
";

/// What a file `-o` named itself is called, for the recordings above: the
/// registry's name for the meter the file names, whatever the meter reported
/// in its `# model:` line, and the first frame's own time. Not the family
/// name the CSV comment carries.
#[cfg(not(windows))]
const AUTO_NAME_STEM: &str = "measurements-UT61E+";
#[cfg(not(windows))]
const AUTO_NAME_START: &str = "2026-09-02_10-00-00";

/// An empty directory of this test's own: what it runs in, so a file the run
/// names itself lands here.
fn dir_for(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dmm-cli-replay-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// `text` written into `dir` as the recording to play back.
fn recording_of(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("bench.replay");
    std::fs::write(&path, text).expect("write recording");
    path
}

/// The three-frame recording, in `dir`.
fn recording_in(dir: &Path) -> PathBuf {
    recording_of(dir, RECORDING)
}

/// Run the binary as a user would, with nothing of the environment left to
/// move the output: `TZ` fixes the printed offset, `NO_COLOR` the styling,
/// and an inherited `RUST_LOG` would add lines of its own.
fn run(args: &[&str]) -> (String, String, bool) {
    run_in(&std::env::temp_dir(), args)
}

/// The same, from `dir` — where a file the run names itself lands.
fn run_in(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_dmm-cli"))
        .args(args)
        .current_dir(dir)
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

#[cfg(not(windows))]
#[test]
fn replay_exports_the_times_the_frames_were_recorded_at() {
    let path = recording_in(&dir_for("timestamps"));
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
    let path = recording_in(&dir_for("deterministic"));
    let (first, _, _) = read_csv(&path, &[]);
    let (second, _, _) = read_csv(&path, &[]);
    assert_eq!(first, second);
}

/// The clock flags are the mock's, and a replay is paced the same way — a
/// preseed spends the recording's sleeps at once without moving a timestamp.
#[test]
fn replay_takes_the_clock_flags() {
    let path = recording_in(&dir_for("preseed"));
    let (plain, _, _) = read_csv(&path, &[]);
    let (preseeded, _, ok) = read_csv(&path, &["--mock-clock-preseed", "10"]);
    assert!(ok, "preseeded replay failed: {preseeded}");
    assert_eq!(plain, preseeded);
}

/// A gap plays back as the timeouts it was, and they are not a quiet meter:
/// there is no `--device` to check and nothing on the cable to switch a USB
/// mode on. The preseed spends the six seconds of silence without waiting.
#[cfg(not(windows))]
#[test]
fn a_gap_in_a_recording_does_not_print_the_no_response_help() {
    let path = recording_of(&dir_for("gap"), RECORDING_WITH_A_GAP);
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

/// A recording written from a recording is the same session: the copy carries
/// the frames and the time they were recorded at, so both play back alike.
#[test]
fn a_replay_written_from_a_replay_plays_back_identically() {
    let dir = dir_for("round-trip");
    let path = recording_in(&dir);
    let copy = dir.join("copy.replay");
    let (_, stderr, ok) = run(&[
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "3",
        "--format",
        "replay",
        "-o",
        copy.to_str().expect("utf-8 path"),
    ]);
    assert!(ok, "writing the copy failed: {stderr}");
    // The summary is the run's, whatever it wrote.
    assert!(stderr.contains("--- 3 samples"), "got {stderr}");

    let (original, _, _) = read_csv(&path, &[]);
    let (played, _, ok) = read_csv(&copy, &[]);
    assert!(ok, "the copy did not play back: {played}");
    assert_eq!(original, played);
}

/// Given no file name, `-o` builds one from the meter, the mode the run
/// stayed in and the first reading's time, and says where it went.
#[cfg(not(windows))]
#[test]
fn a_bare_output_names_the_file_after_the_meter_and_the_mode() {
    let dir = dir_for("auto-name");
    let path = recording_in(&dir);
    let (_, stderr, ok) = run_in(
        &dir,
        &[
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "3",
            "--format",
            "csv",
            "-o",
        ],
    );
    assert!(ok, "the run failed: {stderr}");

    let name = format!("{AUTO_NAME_STEM}-DC-V-{AUTO_NAME_START}.csv");
    assert!(
        stderr.contains(&format!("Written to {name}")),
        "got {stderr}"
    );
    let written = std::fs::read_to_string(dir.join(&name)).expect("the named file");
    assert!(
        written.starts_with("# device: UNI-T UT61E+\n"),
        "got {written}"
    );
    assert_eq!(written.lines().count(), 5, "comment, header, three rows");
}

/// The name carries the second the run started in, so two runs that start in
/// the same one — here, two plays of the same recording — ask for the same
/// file. The second steps aside instead of writing over the first.
#[cfg(not(windows))]
#[test]
fn two_runs_that_name_the_same_file_both_keep_their_readings() {
    let dir = dir_for("same-second");
    let path = recording_in(&dir);
    let args = [
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "3",
        "--format",
        "csv",
        "-o",
    ];
    let (_, first, ok) = run_in(&dir, &args);
    assert!(ok, "the first run failed: {first}");
    let (_, second, ok) = run_in(&dir, &args);
    assert!(ok, "the second run failed: {second}");

    let name = format!("{AUTO_NAME_STEM}-DC-V-{AUTO_NAME_START}.csv");
    let beside = format!("{AUTO_NAME_STEM}-DC-V-{AUTO_NAME_START}-2.csv");
    assert!(first.contains(&format!("Written to {name}")), "got {first}");
    assert!(
        second.contains(&format!("Written to {beside}")),
        "got {second}"
    );
    for file in [&name, &beside] {
        let written = std::fs::read_to_string(dir.join(file)).expect("the named file");
        assert_eq!(written.lines().count(), 5, "{file} holds {written}");
    }
}

/// A run that crossed a function switch is no one mode's, so the mode comes
/// back out of the name when the run ends.
#[cfg(not(windows))]
#[test]
fn a_run_that_changes_mode_drops_the_mode_from_the_name() {
    let dir = dir_for("mode-change");
    let path = recording_of(&dir, RECORDING_ACROSS_MODES);
    let (_, stderr, ok) = run_in(
        &dir,
        &[
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "2",
            "--format",
            "csv",
            "-o",
        ],
    );
    assert!(ok, "the run failed: {stderr}");

    let name = format!("{AUTO_NAME_STEM}-{AUTO_NAME_START}.csv");
    assert!(
        stderr.contains(&format!("Written to {name}")),
        "got {stderr}"
    );
    assert!(dir.join(&name).exists(), "no file at {name}");
    // Nothing is left behind under the mode the run started in.
    let strays: Vec<String> = std::fs::read_dir(&dir)
        .expect("read the run's directory")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.starts_with(AUTO_NAME_STEM) && *n != name)
        .collect();
    assert!(strays.is_empty(), "left behind {strays:?}");
}

/// The rename that drops the mode at the end of a run is a write too: the
/// name it renames onto may be a file another run already left there.
#[cfg(not(windows))]
#[test]
fn a_rename_onto_an_existing_name_steps_aside() {
    let dir = dir_for("rename");
    let path = recording_of(&dir, RECORDING_ACROSS_MODES);
    let args = [
        "read",
        "--replay",
        path.to_str().expect("utf-8 path"),
        "--count",
        "2",
        "--format",
        "csv",
        "-o",
    ];
    let (_, first, ok) = run_in(&dir, &args);
    assert!(ok, "the first run failed: {first}");
    let (_, second, ok) = run_in(&dir, &args);
    assert!(ok, "the second run failed: {second}");

    let name = format!("{AUTO_NAME_STEM}-{AUTO_NAME_START}.csv");
    let beside = format!("{AUTO_NAME_STEM}-{AUTO_NAME_START}-2.csv");
    assert!(first.contains(&format!("Written to {name}")), "got {first}");
    assert!(
        second.contains(&format!("Written to {beside}")),
        "got {second}"
    );
    for file in [&name, &beside] {
        let written = std::fs::read_to_string(dir.join(file)).expect("the named file");
        assert_eq!(written.lines().count(), 4, "{file} holds {written}");
    }
}

/// A bare `-o` promises the path it wrote, so a run that never got a reading
/// has to say there is no file rather than ending in silence.
///
/// Unix-only: nothing else ends a run with no readings to count, so the test
/// interrupts it the way a user would.
#[cfg(unix)]
#[test]
fn a_bare_output_with_no_readings_says_no_file_was_written() {
    use std::io::BufRead;

    let dir = dir_for("no-readings");
    let path = recording_of(&dir, RECORDING_ALL_CORRUPT);
    let mut child = Command::new(env!("CARGO_BIN_EXE_dmm-cli"))
        .args([
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "1",
            "--format",
            "csv",
            "-o",
        ])
        .current_dir(&dir)
        .env("TZ", "UTC")
        .env("NO_COLOR", "1")
        .env_remove("RUST_LOG")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run dmm-cli");

    // Interrupt it once the frame has been refused, so the run is past the
    // point where a file would have been named.
    let mut lines = std::io::BufReader::new(child.stderr.take().expect("piped stderr")).lines();
    let first = lines.next().expect("a line").expect("utf-8 stderr");
    assert!(first.contains("invalid response"), "got {first}");
    let killed = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send an interrupt");
    assert!(killed.success(), "could not interrupt the run");

    let rest: Vec<String> = lines.map_while(Result::ok).collect();
    let status = child.wait().expect("the run ends");
    assert!(
        status.success(),
        "an interrupted run ends cleanly: {rest:?}"
    );
    assert!(
        rest.iter()
            .any(|l| l.contains("No readings arrived, so no file was written")),
        "got {rest:?}"
    );
    let files: Vec<String> = std::fs::read_dir(&dir)
        .expect("read the run's directory")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.starts_with("measurements-"))
        .collect();
    assert!(files.is_empty(), "left behind {files:?}");
}

/// With no `--format`, the file name says what to write.
#[test]
fn the_output_file_extension_picks_the_format() {
    let dir = dir_for("inferred");
    let path = recording_in(&dir);
    for (name, starts_with) in [
        ("readings.csv", "# device: UNI-T UT61E+"),
        ("readings.json", "{\"_metadata\":"),
        ("readings.replay", "# dmm-replay 1"),
        // Nothing recognises `.dat`, so it holds what stdout would have.
        ("readings.dat", "1.6109 V"),
    ] {
        let (_, stderr, ok) = run_in(
            &dir,
            &[
                "read",
                "--replay",
                path.to_str().expect("utf-8 path"),
                "--count",
                "1",
                "-o",
                name,
            ],
        );
        assert!(ok, "{name} failed: {stderr}");
        let written = std::fs::read_to_string(dir.join(name)).expect("the file");
        assert!(written.starts_with(starts_with), "{name} holds {written}");
    }
}

/// The flag wins over the file name, and says so rather than refusing: the
/// user asked for both, and only one of them can be honoured.
#[test]
fn a_format_that_disagrees_with_the_file_name_is_noted() {
    let dir = dir_for("mismatch");
    let path = recording_in(&dir);
    let (_, stderr, ok) = run_in(
        &dir,
        &[
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "1",
            "--format",
            "csv",
            "-o",
            "readings.json",
        ],
    );
    assert!(ok, "the run failed: {stderr}");
    assert!(
        stderr.contains("Note: --format csv written to readings.json"),
        "got {stderr}"
    );
    let written = std::fs::read_to_string(dir.join("readings.json")).expect("the file");
    assert!(written.starts_with("# device:"), "got {written}");
}

/// A replay file holds the meter's own frames, so what to make of them is a
/// choice for the run that plays them back.
#[test]
fn a_replay_run_refuses_the_flags_that_change_the_reading() {
    let path = recording_in(&dir_for("refusals"));
    for (flag, value) in [
        ("--scale", Some("100")),
        ("--offset", Some("1")),
        ("--unit", Some("A")),
        ("--integrate", None),
    ] {
        let mut args = vec![
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "1",
            "--format",
            "replay",
            flag,
        ];
        args.extend(value);
        let (_, stderr, ok) = run(&args);
        assert!(!ok, "{flag} should be refused");
        assert!(stderr.contains(flag), "got {stderr}");
        assert!(stderr.contains("playing it back"), "got {stderr}");
    }
}

/// A refusal ends the run, so it comes before the note about where the output
/// would have gone: the note described a run that never happened.
#[test]
fn a_refused_run_prints_the_refusal_and_no_note() {
    let dir = dir_for("refusal-first");
    let path = recording_in(&dir);
    let (_, stderr, ok) = run_in(
        &dir,
        &[
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            "1",
            "--format",
            "replay",
            "-o",
            "readings.csv",
            "--scale",
            "2",
        ],
    );
    assert!(!ok, "--scale should be refused: {stderr}");
    assert!(stderr.contains("playing it back"), "got {stderr}");
    assert!(!stderr.contains("Note:"), "got {stderr}");
    assert!(!dir.join("readings.csv").exists(), "the run wrote a file");
}

#[test]
fn recording_the_mock_is_refused() {
    let (_, stderr, ok) = run(&[
        "--device", "mock", "read", "--format", "replay", "--count", "1",
    ]);
    assert!(!ok, "expected a refusal");
    assert!(
        stderr.contains("--format replay needs a real meter"),
        "got {stderr}"
    );
}

/// A recording is written with `--format replay` now; the flag it used to
/// take must not quietly do something else.
#[test]
fn the_old_record_flag_is_gone() {
    let (_, stderr, ok) = run(&["read", "--record", "bench.replay", "--count", "1"]);
    assert!(!ok, "expected a refusal");
    assert!(stderr.contains("--record"), "got {stderr}");
}

/// The file names the meter, so a `--device` on the command line is either
/// redundant or a contradiction.
#[test]
fn naming_a_device_alongside_a_replay_is_refused() {
    let path = recording_in(&dir_for("device"));
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

/// Every recording the repo ships is real meter data: played with the default
/// log level, none of it may be reported as unrecognised — or warn at all.
#[test]
fn the_bundled_recordings_play_without_a_warning() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/replays");
    let mut played = 0;
    for entry in std::fs::read_dir(&dir).expect("assets/replays") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|e| e != "replay") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read recording");
        let frames = text.lines().filter(|l| !l.starts_with('#')).count();
        let (_, stderr, ok) = run(&[
            "read",
            "--replay",
            path.to_str().expect("utf-8 path"),
            "--count",
            &frames.to_string(),
            "--format",
            "csv",
            "--mock-clock-preseed",
            "3600",
        ]);
        assert!(ok, "{}: {stderr}", path.display());
        assert!(!stderr.contains("WARN"), "{}: {stderr}", path.display());
        played += 1;
    }
    assert!(played > 0, "no recordings in {}", dir.display());
}

/// A display the parser can't read warns once, by default, with where to
/// report it — not once per frame.
#[test]
fn unrecognised_data_warns_once_by_default() {
    let path = recording_of(
        &dir_for("unrecognised"),
        "\
# dmm-replay 1
# device: ut61eplus
# recorded: 2026-09-02T10:00:00Z
# model: UT61E+
0 02 30 20 20 43 55 54 20 20 00 00 30 30 30
100 02 30 20 20 43 55 54 20 20 00 00 30 30 30
200 02 30 20 20 43 55 54 20 20 00 00 30 30 30
",
    );
    let (stdout, stderr, ok) = read_csv(&path, &[]);
    assert!(ok, "replay failed: {stderr}");
    assert_eq!(stdout.matches(",OL,").count(), 3, "got {stdout}");
    assert_eq!(
        stderr
            .matches("unrecognised display text: \"CUT\", shown as OL")
            .count(),
        1,
        "got {stderr}"
    );
    assert_eq!(
        stderr
            .matches("https://github.com/antoinecellerier/dmm-tools/issues")
            .count(),
        1,
        "got {stderr}"
    );
}
