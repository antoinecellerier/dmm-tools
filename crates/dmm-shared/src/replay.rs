//! What `--replay` opens, in both binaries: a replay file, or a golden
//! fixture played as a one-frame recording.
//!
//! A golden fixture (`crates/dmm-lib/tests/golden/<device-id>/<case>.yaml`)
//! holds one real frame as `raw_hex`, and the directory it sits in names the
//! meter. That is everything a one-frame replay needs, so a fixture opens as
//! one and shows how the apps render that frame — a meter nobody here owns
//! included. The fixture's other fields are what the golden test expects the
//! frame to parse to; playback parses the frame itself.

use chrono::SecondsFormat;
use dmm_lib::error::{Error, Result};
use dmm_lib::protocol::registry;
use dmm_lib::replay::{self, Replay};
use std::path::Path;
use std::time::Duration;

/// Open `path` as a recording: a golden fixture when it is a `.yaml` file,
/// else a replay file.
pub fn load(path: &Path) -> Result<Replay> {
    if path.extension().is_none_or(|e| e != "yaml") {
        return Replay::load(path);
    }
    let fail = |message: String| Error::Replay(format!("{}: {message}", path.display()));
    let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
    let dir = path
        .canonicalize()
        .ok()
        .and_then(|p| Some(p.parent()?.file_name()?.to_string_lossy().into_owned()))
        .unwrap_or_default();
    fixture(&dir, &text).map_err(fail)
}

/// A golden fixture's text as a one-frame recording of the meter `dir_name`
/// names, recorded now: a fixture is a snapshot, and reads as one just taken.
fn fixture(dir_name: &str, text: &str) -> std::result::Result<Replay, String> {
    let device = registry::resolve_device(dir_name).ok_or_else(|| {
        format!(
            "a golden fixture's directory must be named after a device id, \
             e.g. `ut61eplus`; `{dir_name}` is not one"
        )
    })?;
    // At column 0 only: a capture report's samples carry `raw_hex` too, indented
    // under their step, and one of those is not the frame this file is about.
    let raw_hex = text
        .lines()
        .find_map(|line| line.strip_prefix("raw_hex:"))
        .ok_or("no top-level `raw_hex:` line; is this a golden fixture?")?;
    let payload = decode_hex(unquote(raw_hex.trim()))?;
    let recorded = chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false);
    // The writers `Replay::parse` is kept in step with, rather than text of
    // our own.
    let mut recording = replay::header(device.id, &recorded, None, None);
    recording.push_str(&replay::sample_line(Duration::ZERO, &payload));
    Replay::parse(&recording).map_err(|e| e.to_string())
}

/// `value` without one pair of surrounding YAML quotes.
fn unquote(value: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|&q| value.strip_prefix(q)?.strip_suffix(q))
        .unwrap_or(value)
}

/// The bytes of `hex`, spaces or not, as the golden test reads them.
fn decode_hex(hex: &str) -> std::result::Result<Vec<u8>, String> {
    let digits: Vec<char> = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.is_empty() {
        return Err("`raw_hex` is empty".to_string());
    }
    if !digits.len().is_multiple_of(2) {
        return Err(format!("`raw_hex` has an odd number of digits: {hex}"));
    }
    digits
        .chunks(2)
        .map(|pair| {
            let byte: String = pair.iter().collect();
            u8::from_str_radix(&byte, 16)
                .map_err(|_| format!("`raw_hex`: `{byte}` is not a hex byte"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::Clock;
    use std::path::PathBuf;

    /// The frame of dmm-lib's `ut61eplus/dcv_battery` fixture.
    const FIXTURE: &str = "\
# DC V across a battery
raw_hex: \"02 30 20 31 2E 36 31 30 39 03 02 30 30 30\"
mode: \"DC V\"
value: \"1.6109\"
";

    const FRAME: [u8; 14] = [
        0x02, 0x30, 0x20, 0x31, 0x2E, 0x36, 0x31, 0x30, 0x39, 0x03, 0x02, 0x30, 0x30, 0x30,
    ];

    /// The one frame a recording plays, decoded.
    fn first_reading(replay: &Replay) -> dmm_lib::measurement::Measurement {
        replay
            .open(Clock::manual())
            .expect("opens")
            .request_measurement()
            .expect("a reading")
    }

    #[test]
    fn a_fixture_plays_its_frame_as_the_meter_its_directory_names() {
        let replay = fixture("ut61eplus", FIXTURE).expect("a fixture");
        assert_eq!(replay.device.id, "ut61eplus");
        let m = first_reading(&replay);
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.value.to_string(), "1.6109");
        assert_eq!(m.raw_payload, FRAME);
    }

    /// The golden test takes `raw_hex` with or without spaces, so a fixture
    /// that passes it must open here too.
    #[test]
    fn unspaced_hex_is_the_same_frame() {
        let unspaced = FIXTURE.replace(
            "02 30 20 31 2E 36 31 30 39 03 02 30 30 30",
            "023020312E363130390302303030",
        );
        let replay = fixture("ut61eplus", &unspaced).expect("a fixture");
        assert_eq!(first_reading(&replay).raw_payload, FRAME);
    }

    #[test]
    fn a_directory_that_names_no_meter_is_refused() {
        let e = fixture("my-captures", FIXTURE).err().expect("refused");
        assert!(e.contains("`my-captures`"), "got {e}");
    }

    /// A capture report's samples carry `raw_hex` indented under their step:
    /// the file is not one frame, so it is not played as one.
    #[test]
    fn a_capture_report_is_not_a_fixture() {
        let report = "steps:\n  - id: dcv\n    samples:\n      - raw_hex: \"02 30\"\n";
        let e = fixture("ut61eplus", report).err().expect("refused");
        assert!(e.contains("no top-level `raw_hex:`"), "got {e}");
    }

    #[test]
    fn a_bad_hex_digit_is_refused_by_name() {
        let bad = FIXTURE.replace("2E", "ZE");
        let e = fixture("ut61eplus", &bad).err().expect("refused");
        assert!(e.contains("`raw_hex`: `ZE`"), "got {e}");
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn a_replay_file_loads_as_it_always_did() {
        let path = repo_root().join("assets/replays/ohm.replay");
        let (ours, plain) = (
            load(&path).expect("a replay"),
            Replay::load(&path).expect("a replay"),
        );
        assert_eq!(ours.device.id, plain.device.id);
        assert_eq!(ours.recorded, plain.recorded);
        assert_eq!(ours.duration(), plain.duration());
    }

    /// The promise: every golden fixture opens, and plays a reading.
    #[test]
    fn every_golden_fixture_opens() {
        let root = repo_root().join("crates/dmm-lib/tests/golden");
        let mut opened = 0;
        for dir in std::fs::read_dir(&root).expect("golden dir") {
            for file in std::fs::read_dir(dir.expect("entry").path()).expect("device dir") {
                let path = file.expect("entry").path();
                let replay = load(&path).unwrap_or_else(|e| panic!("{e}"));
                replay
                    .open(Clock::manual())
                    .and_then(|mut dmm| dmm.request_measurement())
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                opened += 1;
            }
        }
        assert!(opened > 0, "no fixtures in {}", root.display());
    }
}
