//! Playing a recorded meter session back as if the meter were on the cable.
//!
//! A replay file holds the payloads a meter sent and how far into the session
//! each one arrived. Playback hands those bytes to the family's real
//! [`Protocol::parse_payload`] — the same call a golden fixture goes through —
//! so a replayed reading is decoded exactly as the live one was, and nothing
//! here has to know a frame's shape, its framing or its handshake.
//!
//! What a replay is *not* is a meter: no command reaches hardware, so every
//! command and setting switch is refused rather than silently doing nothing.
//!
//! The file is line-based and hand-parsed — this crate carries no serde:
//!
//! ```text
//! # dmm-replay 1
//! # device: ut61eplus
//! # recorded: 2026-09-16T10:22:31.123+02:00
//! # model: UT61E+
//! 0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
//! 101 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
//! ```
//!
//! [`header`] and [`sample_line`] write what [`Replay::parse`] reads, so a
//! writer in another crate cannot drift from this parser.

use crate::Dmm;
use crate::clock::Clock;
use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::registry::{self, SelectableDevice};
use crate::protocol::{CaptureStep, Choice, DeviceProfile, Protocol, Setting};
use crate::specs::{ModeSpecInfo, SpecInfo};
use crate::transport::{NullTransport, Transport};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};

/// The first non-blank line of every replay file. The trailing digit is the
/// format version: a reader that does not know a version refuses the file
/// rather than guessing at its lines.
const MAGIC: &str = "# dmm-replay 1";

/// Longest a single `request_measurement` waits before reporting the quiet
/// meter that a gap in the recording was.
///
/// The caller polls again straight away, so a long gap is played back as a
/// run of timeouts. Bounding it here is what keeps Disconnect responsive:
/// the sleep happens inside the protocol, where the stream's cancel check
/// cannot see it.
const GAP_SLICE: Duration = Duration::from_secs(1);

/// Floor under the cadence derived from a file's own sample spacing, so a
/// burst of frames recorded milliseconds apart does not spin the tail.
const MIN_CADENCE: Duration = Duration::from_millis(50);

/// Cadence for a file with a single sample, which has no spacing of its own.
/// About what the polled families answer at, so a one-frame file plays back
/// as a steady reading rather than a stutter.
const LONE_SAMPLE_CADENCE: Duration = Duration::from_millis(500);

/// A recorded session, parsed and ready to open.
///
/// Cheap to keep around and open more than once: a GUI reconnect re-opens the
/// same `Replay` rather than re-reading the file.
pub struct Replay {
    /// The meter the recording was made from, resolved against the registry.
    pub device: &'static SelectableDevice,
    /// The `# recorded:` value, verbatim. This crate has no date library, so
    /// the binaries — which do — turn it into a wall time and hand it back
    /// through [`Clock::with_wall_origin`].
    pub recorded: String,
    /// The name the meter reported when the recording was made, if it has one.
    pub model: Option<String>,
    /// Non-empty, offsets non-decreasing — both enforced by the parser.
    samples: Vec<(Duration, Vec<u8>)>,
}

impl Replay {
    /// Parse a replay file's text.
    pub fn parse(text: &str) -> Result<Self> {
        let mut seen_magic = false;
        let mut device: Option<&'static SelectableDevice> = None;
        let mut recorded: Option<String> = None;
        let mut model: Option<String> = None;
        let mut samples: Vec<(Duration, Vec<u8>)> = Vec::new();

        for (index, raw) in text.lines().enumerate() {
            let line_no = index + 1;
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if !seen_magic {
                if line != MAGIC {
                    return Err(at(line_no, format!("first line must be `{MAGIC}`")));
                }
                seen_magic = true;
                continue;
            }
            if let Some(comment) = line.strip_prefix('#') {
                let comment = comment.trim();
                if let Some(value) = comment.strip_prefix("device:") {
                    device = Some(resolve_device(line_no, value.trim())?);
                } else if let Some(value) = comment.strip_prefix("recorded:") {
                    recorded = Some(value.trim().to_string());
                } else if let Some(value) = comment.strip_prefix("model:") {
                    model = Some(value.trim().to_string()).filter(|m| !m.is_empty());
                }
                // Anything else is a comment: a writer notes where a file came
                // from, and an unknown key must not strand a whole recording.
                continue;
            }
            samples.push(parse_sample(line_no, line, samples.last())?);
        }

        if !seen_magic {
            return Err(at(1, format!("first line must be `{MAGIC}`")));
        }
        let Some(device) = device else {
            return Err(Error::Replay("no `# device:` line".to_string()));
        };
        let recorded = match recorded {
            Some(r) if !r.is_empty() => r,
            _ => return Err(Error::Replay("no `# recorded:` line".to_string())),
        };
        if samples.is_empty() {
            return Err(Error::Replay("no samples".to_string()));
        }
        Ok(Self {
            device,
            recorded,
            model,
            samples,
        })
    }

    /// Read and parse a replay file.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Replay(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }

    /// Open the recording as a session, paced by `clock`.
    ///
    /// Session zero is the clock's wall origin where it has one, so every
    /// reading lands at the session time it was recorded at even when the
    /// origin was pinned before any sample was read.
    pub fn open(&self, clock: Clock) -> Result<Dmm<NullTransport>> {
        let start = clock
            .wall_origin()
            .map(|(instant, _)| instant)
            .unwrap_or_else(|| clock.now());
        let protocol = ReplayProtocol {
            inner: (self.device.new_protocol)(),
            samples: self.samples.clone(),
            model: self.model.clone(),
            cadence: cadence(&self.samples),
            clock: clock.clone(),
            start,
            next: 0,
            next_due: Duration::ZERO,
        };
        let dmm = Dmm::new(NullTransport, Box::new(protocol))?;
        Ok(dmm.with_clock(clock).with_protocol_timestamps())
    }

    /// How far into the session the last sample was recorded.
    pub fn duration(&self) -> Duration {
        self.samples
            .last()
            .map(|(offset, _)| *offset)
            .unwrap_or(Duration::ZERO)
    }
}

/// Load `path` and open it as a session in one step.
pub fn open_replay(path: &Path, clock: Clock) -> Result<Dmm<NullTransport>> {
    Replay::load(path)?.open(clock)
}

/// The header lines of a replay file, ending in a newline.
///
/// Callers append [`sample_line`]s to this.
pub fn header(device_id: &str, recorded_rfc3339: &str, model: Option<&str>) -> String {
    let mut out = format!("{MAGIC}\n# device: {device_id}\n# recorded: {recorded_rfc3339}\n");
    if let Some(model) = model {
        out.push_str("# model: ");
        out.push_str(model);
        out.push('\n');
    }
    out
}

/// One sample line: the offset in milliseconds, then the payload as
/// uppercase hex bytes.
pub fn sample_line(offset: Duration, payload: &[u8]) -> String {
    // The parser reads the offset as a `u64`; saturating keeps a line
    // readable rather than emitting a number nothing can parse back.
    let ms = u64::try_from(offset.as_millis()).unwrap_or(u64::MAX);
    let mut out = ms.to_string();
    for byte in payload {
        out.push(' ');
        out.push_str(&format!("{byte:02X}"));
    }
    out.push('\n');
    out
}

/// A parse failure, naming the line it came from.
fn at(line_no: usize, message: String) -> Error {
    Error::Replay(format!("line {line_no}: {message}"))
}

/// Resolve a `# device:` value to its registry entry.
///
/// Exact id only: a replay file is written by this project, so the aliases
/// that exist to be forgiving about a hand-typed `--device` would only widen
/// what a file can claim to be.
fn resolve_device(line_no: usize, id: &str) -> Result<&'static SelectableDevice> {
    let Some(device) = registry::find_device(id) else {
        return Err(at(line_no, format!("unknown device `{id}`")));
    };
    if !device.requires_hardware {
        // The mock synthesises its readings, so it has no `parse_payload` to
        // play bytes back through.
        return Err(at(
            line_no,
            format!("`{id}` is not a meter, so it has no frames to replay"),
        ));
    }
    Ok(device)
}

/// Parse one `<ms> <HH HH …>` line, given the sample before it.
fn parse_sample(
    line_no: usize,
    line: &str,
    previous: Option<&(Duration, Vec<u8>)>,
) -> Result<(Duration, Vec<u8>)> {
    let mut fields = line.split_whitespace();
    let Some(ms) = fields.next() else {
        return Err(at(line_no, "empty sample line".to_string()));
    };
    let Ok(ms) = u64::from_str(ms) else {
        return Err(at(line_no, format!("`{ms}` is not a millisecond offset")));
    };
    let offset = Duration::from_millis(ms);
    if let Some((last, _)) = previous
        && offset < *last
    {
        return Err(at(
            line_no,
            format!("offset {ms} ms goes back before {} ms", last.as_millis()),
        ));
    }

    let mut payload = Vec::new();
    for field in fields {
        // Two digits exactly: `from_str_radix` would take "3" or "+7" as a
        // byte, so a line truncated mid-frame would parse as a shorter one.
        let byte = match field.len() {
            2 => u8::from_str_radix(field, 16).ok(),
            _ => None,
        };
        let Some(byte) = byte else {
            return Err(at(line_no, format!("`{field}` is not a hex byte")));
        };
        payload.push(byte);
    }
    if payload.is_empty() {
        return Err(at(line_no, "sample has no payload".to_string()));
    }
    Ok((offset, payload))
}

/// How often the tail repeats the last frame: the file's own median gap,
/// floored so a dense recording does not spin.
///
/// The median rather than the mean, so one pause in an otherwise steady
/// recording does not slow the whole tail down.
fn cadence(samples: &[(Duration, Vec<u8>)]) -> Duration {
    let mut gaps: Vec<Duration> = samples
        .iter()
        .zip(samples.iter().skip(1))
        .map(|((before, _), (after, _))| after.saturating_sub(*before))
        .collect();
    if gaps.is_empty() {
        return LONE_SAMPLE_CADENCE;
    }
    gaps.sort_unstable();
    gaps.get(gaps.len() / 2)
        .copied()
        .unwrap_or(LONE_SAMPLE_CADENCE)
        .max(MIN_CADENCE)
}

/// The refusal every command path answers with, in one place so the two
/// cannot word it differently.
fn takes_no_commands(what: impl std::fmt::Display) -> Error {
    Error::CommandRejected(format!("{what}: a replay takes no commands"))
}

/// A recording driving a family's real parser.
///
/// Everything read-only delegates to the family's own protocol, so the specs
/// panel, the choice lists and the profile look exactly as they would on the
/// meter; only I/O and the device name are answered from the file.
struct ReplayProtocol {
    inner: Box<dyn Protocol>,
    samples: Vec<(Duration, Vec<u8>)>,
    model: Option<String>,
    cadence: Duration,
    clock: Clock,
    /// Session instant the recording's t=0 maps to.
    start: Instant,
    /// Index of the next sample; `samples.len()` once the tail is playing.
    next: usize,
    /// When the frame after the one just returned falls due. Only the tail
    /// reads it — until then the samples' own offsets say when they are due.
    next_due: Duration,
}

impl ReplayProtocol {
    /// Let session time reach `due`, or report the quiet meter.
    ///
    /// A recording gap *was* a meter that stopped answering, so it plays back
    /// as the timeout it was rather than as a long silent block.
    fn wait_until(&self, due: Duration, now: Duration) -> Result<()> {
        let Some(wait) = due.checked_sub(now) else {
            return Ok(());
        };
        if wait > GAP_SLICE {
            self.clock.sleep(GAP_SLICE);
            return Err(Error::Timeout);
        }
        self.clock.sleep(wait);
        Ok(())
    }
}

impl Protocol for ReplayProtocol {
    /// Nothing to bring up: the inner protocol's `init` would write a
    /// streaming trigger to a transport that answers nobody.
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        Ok(())
    }

    fn request_measurement(&mut self, _transport: &dyn Transport) -> Result<Measurement> {
        let now = self.clock.now().saturating_duration_since(self.start);
        // Of the samples already due, only the newest is still on the wire: a
        // meter that was not polled for a second does not hand over the
        // second's worth of frames it sent meanwhile.
        while self
            .samples
            .get(self.next.saturating_add(1))
            .is_some_and(|(offset, _)| *offset <= now)
        {
            self.next += 1;
        }
        // Past the end the last frame is held, one per cadence: the trace goes
        // flat and the meter stays "on", which is also what makes a one-frame
        // file usable as a steady reading.
        let (due, index) = match self.samples.get(self.next) {
            Some((offset, _)) => (*offset, self.next),
            None => {
                // Repeats already past are dropped as the recorded samples
                // are, so a caller polling slower than the cadence gets the
                // newest one rather than every one it missed. Along the grid
                // the cadence lays down, which keeps the timestamps the same
                // whenever the poll comes.
                while self.next_due.saturating_add(self.cadence) <= now {
                    self.next_due = self.next_due.saturating_add(self.cadence);
                }
                (self.next_due, self.samples.len().saturating_sub(1))
            }
        };
        self.wait_until(due, now)?;

        // Past the sample before it is parsed: a frame the family refuses is
        // skipped, the way a corrupt frame off the wire is. Leaving `next` on
        // it would hand the same bytes to the next poll, as fast as the caller
        // asks — and for ever, if it is the last frame in the file.
        self.next = self.next.saturating_add(1);
        self.next_due = due.saturating_add(self.cadence);
        let Some((_, payload)) = self.samples.get(index) else {
            return Err(Error::Replay("no samples".to_string()));
        };
        let mut m = self.inner.parse_payload(payload)?;
        // The sample's own session time, not the instant the sleep ended:
        // an export of a replay is the recording's timestamps, to the digit.
        m.timestamp = self.start.checked_add(due).unwrap_or(self.start);
        Ok(m)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        self.inner.parse_payload(payload)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        Err(takes_no_commands(command))
    }

    /// The file, not the meter: the UT61+ name query would poll a transport
    /// that never answers.
    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(self.model.clone())
    }

    fn profile(&self) -> &DeviceProfile {
        self.inner.profile()
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        self.inner.capture_steps()
    }

    fn spec_info(&self, mode_raw: u16, range_raw: u8) -> Option<&'static SpecInfo> {
        self.inner.spec_info(mode_raw, range_raw)
    }

    fn mode_spec_info(&self, mode_raw: u16) -> Option<&'static ModeSpecInfo> {
        self.inner.mode_spec_info(mode_raw)
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        self.inner.choices(setting, current)
    }

    fn select(&mut self, _transport: &dyn Transport, setting: Setting, _id: u16) -> Result<()> {
        Err(takes_no_commands(setting))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames a UT61E+ really sent, copied from the golden fixtures in
    /// `crates/dmm-lib/tests/golden/ut61eplus/`: 1.6109 V off a battery,
    /// -0.5137 V, and 80.45 kΩ across an 82 kΩ resistor.
    const DCV_BATTERY: &str = "02 30 20 31 2E 36 31 30 39 03 02 30 30 30";
    const DCV_NEGATIVE: &str = "02 30 2D 30 2E 35 31 33 37 01 00 30 30 31";
    const OHM_82K: &str = "06 33 20 20 38 30 2E 34 35 01 06 30 30 30";

    const RECORDED: &str = "2026-09-16T10:22:31.123+02:00";

    fn payload(hex: &str) -> Vec<u8> {
        hex.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).expect("test payloads are hex"))
            .collect()
    }

    /// A file written the way a recorder writes one, so the tests exercise
    /// [`header`] and [`sample_line`] rather than a hand-typed copy of them.
    fn file(frames: &[(u64, &str)]) -> String {
        let mut text = header("ut61eplus", RECORDED, Some("UT61E+"));
        for (ms, hex) in frames {
            text.push_str(&sample_line(Duration::from_millis(*ms), &payload(hex)));
        }
        text
    }

    fn three_frames() -> String {
        file(&[(0, DCV_BATTERY), (100, DCV_NEGATIVE), (300, OHM_82K)])
    }

    fn parsed(text: &str) -> Replay {
        Replay::parse(text).expect("a valid replay file")
    }

    fn rejects(text: &str) -> String {
        match Replay::parse(text) {
            Err(Error::Replay(message)) => message,
            Err(other) => panic!("expected a replay error, got {other}"),
            Ok(_) => panic!("expected a rejection, got a replay"),
        }
    }

    /// Open a three-frame replay on a clock the test drives by hand.
    fn open_manual() -> (Dmm<NullTransport>, Clock, Instant) {
        let clock = Clock::manual();
        let start = clock.now();
        let dmm = parsed(&three_frames())
            .open(clock.clone())
            .expect("the replay opens");
        (dmm, clock, start)
    }

    #[test]
    fn a_written_file_parses_back_to_what_was_written() {
        let replay = parsed(&three_frames());
        assert_eq!(replay.device.id, "ut61eplus");
        assert_eq!(replay.recorded, RECORDED);
        assert_eq!(replay.model.as_deref(), Some("UT61E+"));
        assert_eq!(replay.duration(), Duration::from_millis(300));
        assert_eq!(
            replay.samples,
            vec![
                (Duration::ZERO, payload(DCV_BATTERY)),
                (Duration::from_millis(100), payload(DCV_NEGATIVE)),
                (Duration::from_millis(300), payload(OHM_82K)),
            ]
        );
    }

    /// Blank lines and comments a writer or a human left behind are not data.
    #[test]
    fn blank_lines_and_unknown_comments_are_ignored() {
        let text = format!(
            "\n{MAGIC}\n# device: ut61eplus\n# source: issue #5\n# recorded: {RECORDED}\n\n0 {DCV_BATTERY}\n"
        );
        let replay = parsed(&text);
        assert_eq!(replay.model, None);
        assert_eq!(replay.samples.len(), 1);
    }

    #[test]
    fn a_file_that_is_not_a_replay_is_refused() {
        let message = rejects(&format!("# device: ut61eplus\n0 {DCV_BATTERY}\n"));
        assert!(message.contains("dmm-replay 1"), "got {message}");
        // An empty file is the same complaint, not a panic.
        assert!(rejects("").contains("dmm-replay 1"));
    }

    #[test]
    fn an_unknown_device_is_refused() {
        let text = three_frames().replace("# device: ut61eplus", "# device: ut99z");
        let message = rejects(&text);
        assert!(message.contains("line 2"), "got {message}");
        assert!(message.contains("ut99z"), "got {message}");
    }

    /// The mock synthesises its readings, so there is nothing for a recording
    /// of it to hand to `parse_payload`.
    #[test]
    fn a_device_without_a_wire_format_is_refused() {
        let text = three_frames().replace("# device: ut61eplus", "# device: mock");
        let message = rejects(&text);
        assert!(message.contains("mock"), "got {message}");
    }

    #[test]
    fn a_missing_recorded_line_is_refused() {
        let text = three_frames().replace(&format!("# recorded: {RECORDED}\n"), "");
        assert!(rejects(&text).contains("recorded"));
        // Present but empty is the same thing.
        let blank = three_frames().replace(&format!("# recorded: {RECORDED}"), "# recorded:");
        assert!(rejects(&blank).contains("recorded"));
    }

    #[test]
    fn a_file_with_no_samples_is_refused() {
        assert_eq!(rejects(&header("ut61eplus", RECORDED, None)), "no samples");
    }

    #[test]
    fn a_bad_sample_line_names_its_line() {
        let base = three_frames();
        for (broken, needle) in [
            (base.replace("100 02 30 2D", "100 02 ZZ 2D"), "`ZZ`"),
            (base.replace("100 02 30 2D", "100 02 3 2D"), "`3`"),
            (base.replace("100 02 30 2D", "later 02 30 2D"), "`later`"),
        ] {
            let message = rejects(&broken);
            assert!(message.contains("line 6"), "got {message}");
            assert!(message.contains(needle), "got {message}");
        }
    }

    #[test]
    fn an_offset_that_goes_backwards_is_refused() {
        let text = file(&[(300, DCV_BATTERY), (100, DCV_NEGATIVE)]);
        let message = rejects(&text);
        assert!(message.contains("line 6"), "got {message}");
        assert!(message.contains("goes back"), "got {message}");
        // Two frames at the same offset are fine: a meter can answer twice
        // inside one millisecond.
        assert_eq!(
            parsed(&file(&[(100, DCV_BATTERY), (100, DCV_NEGATIVE)]))
                .samples
                .len(),
            2
        );
    }

    #[test]
    fn a_sample_line_without_a_payload_is_refused() {
        let text = format!("{}100\n", header("ut61eplus", RECORDED, None));
        assert!(rejects(&text).contains("no payload"));
    }

    #[test]
    fn the_first_sample_is_ready_at_session_zero() {
        let (mut dmm, _clock, start) = open_manual();
        let m = dmm.request_measurement().expect("the first frame is due");
        assert_eq!(m.timestamp, start);
        assert_eq!(m.value_export_str(), "1.6109");
        assert_eq!(m.unit, "V");
    }

    /// The whole point of the replay's own timestamps: an export reads the
    /// times the recording was made at, not the instants the playback got
    /// round to each frame.
    #[test]
    fn a_replay_keeps_the_samples_own_time() {
        let (mut dmm, clock, start) = open_manual();
        dmm.request_measurement().expect("the first frame");

        // The poll comes late — the reading still carries the frame's time.
        clock.advance(Duration::from_millis(250));
        let m = dmm.request_measurement().expect("the second frame");
        assert_eq!(m.value_export_str(), "-0.5137");
        assert_eq!(m.timestamp, start + Duration::from_millis(100));
        assert_eq!(clock.now(), start + Duration::from_millis(250));
    }

    /// A sleeping caller misses frames exactly as an unpolled meter's do.
    #[test]
    fn a_clock_that_jumps_past_two_samples_returns_the_newer() {
        let (mut dmm, clock, start) = open_manual();
        dmm.request_measurement().expect("the first frame");

        clock.advance(Duration::from_millis(400));
        let m = dmm.request_measurement().expect("the newest due frame");
        assert_eq!(m.value_export_str(), "80.45");
        assert_eq!(m.timestamp, start + Duration::from_millis(300));
    }

    /// Past the end the meter is still on: the last reading repeats at the
    /// file's own cadence (here the 200 ms median of its 100/200 ms gaps).
    #[test]
    fn the_last_sample_is_held_at_the_files_cadence() {
        let (mut dmm, _clock, start) = open_manual();
        for _ in 0..3 {
            dmm.request_measurement().expect("a recorded frame");
        }
        for step in 1..=2 {
            let m = dmm.request_measurement().expect("the held frame");
            assert_eq!(m.value_export_str(), "80.45");
            assert_eq!(
                m.timestamp,
                start + Duration::from_millis(300 + 200 * step),
                "hold {step}"
            );
        }
    }

    /// A caller polling slower than the cadence — a GUI sample interval above
    /// the file's, or a session resumed after a pause — is handed the newest
    /// repeat, not the run of them it was away for.
    #[test]
    fn a_late_poll_past_the_end_gets_one_repeat_at_the_newest_grid_point() {
        let (mut dmm, clock, start) = open_manual();
        for _ in 0..3 {
            dmm.request_measurement().expect("a recorded frame");
        }
        // The file's own 200 ms cadence, ten repeats of it missed.
        let cadence = Duration::from_millis(200);
        clock.advance(cadence * 10);

        let m = dmm.request_measurement().expect("the held frame");
        assert_eq!(m.timestamp, start + Duration::from_millis(2300));
        assert_eq!(clock.now(), m.timestamp, "the newest repeat, no waiting");
        // And the grid is kept, so the next one is a cadence on.
        let next = dmm.request_measurement().expect("the next held frame");
        assert_eq!(next.timestamp, m.timestamp + cadence);
    }

    /// A frame the family refuses is reported and skipped, as a corrupt frame
    /// off the wire is. Retrying it meant the next poll failed on the same
    /// bytes at once — and for ever, had it been the last frame.
    #[test]
    fn a_frame_that_will_not_parse_is_reported_once_and_skipped() {
        let clock = Clock::manual();
        let start = clock.now();
        let mut dmm = parsed(&file(&[
            (0, DCV_BATTERY),
            (100, "02 30 20"),
            (200, OHM_82K),
        ]))
        .open(clock.clone())
        .expect("the replay opens");
        dmm.request_measurement().expect("the first frame");

        let err = dmm
            .request_measurement()
            .expect_err("a truncated frame does not parse");
        assert_eq!(err.kind(), crate::error::ErrorKind::Protocol, "got {err}");

        let m = dmm.request_measurement().expect("the frame after it");
        assert_eq!(m.value_export_str(), "80.45");
        assert_eq!(m.timestamp, start + Duration::from_millis(200));
    }

    /// A file with nothing to derive a cadence from still plays as a steady
    /// reading rather than as fast as it is polled.
    #[test]
    fn a_one_frame_file_repeats_its_reading() {
        let clock = Clock::manual();
        let start = clock.now();
        let mut dmm = parsed(&file(&[(0, DCV_BATTERY)]))
            .open(clock.clone())
            .expect("the replay opens");
        assert_eq!(
            dmm.request_measurement().expect("the frame").timestamp,
            start
        );
        assert_eq!(
            dmm.request_measurement().expect("the held frame").timestamp,
            start + LONE_SAMPLE_CADENCE
        );
    }

    /// A meter that went quiet for two seconds reads back as one: the caller
    /// sees the timeouts it saw then, and never blocks longer than a slice.
    #[test]
    fn a_gap_longer_than_a_second_reads_as_a_quiet_meter() {
        let clock = Clock::manual();
        let start = clock.now();
        let mut dmm = parsed(&file(&[(0, DCV_BATTERY), (2000, OHM_82K)]))
            .open(clock.clone())
            .expect("the replay opens");
        dmm.request_measurement().expect("the first frame");

        let err = dmm
            .request_measurement()
            .expect_err("a two-second gap is a quiet meter");
        assert!(matches!(err, Error::Timeout), "got {err}");
        assert_eq!(clock.now(), start + GAP_SLICE, "a slice, and no more");

        let m = dmm.request_measurement().expect("the frame after the gap");
        assert_eq!(m.timestamp, start + Duration::from_millis(2000));
    }

    /// Nothing is on the far end of the cable, so a command has to be refused
    /// rather than quietly doing nothing.
    #[test]
    fn a_replay_refuses_commands() {
        let (mut dmm, _clock, _start) = open_manual();
        for err in [
            dmm.send_command("hold").expect_err("no commands"),
            dmm.select(Setting::Range, 1).expect_err("no commands"),
        ] {
            assert!(matches!(err, Error::CommandRejected(_)), "got {err}");
            assert!(err.to_string().contains("a replay takes no commands"));
        }
    }

    /// The name comes from the file: the UT61+ query would poll a transport
    /// that answers nothing.
    #[test]
    fn the_model_and_profile_come_from_the_file_and_the_family() {
        let (mut dmm, _clock, _start) = open_manual();
        assert_eq!(
            dmm.get_name().expect("the file's model"),
            Some("UT61E+".to_string())
        );
        assert_eq!(dmm.profile().model_name, "UNI-T UT61E+");
        assert!(
            !dmm.capture_steps().is_empty(),
            "a replay shows the family's own capture steps"
        );

        let m = dmm.request_measurement().expect("a frame");
        assert!(m.spec.is_some(), "the specs panel sees the family's table");
        assert!(!dmm.choices(Setting::Range, &m).is_empty());
    }
}
