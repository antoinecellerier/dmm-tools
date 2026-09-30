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
//! # link: USB cable
//! # view: {"window":30.0,"mean":true}
//! 0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30
//! # marker: 0 1 probes on
//! 101 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31
//! ```
//!
//! A `# marker:` line — offset in milliseconds, number, note — marks the
//! first reading played at or after its offset, wherever the line sits: a
//! playback that skips the marked frame puts the marker on the next one
//! rather than losing it.
//!
//! A `# view:` line carries the viewer's view of the recording as text this
//! crate keeps but does not read.
//!
//! [`header`], [`sample_line`], [`marker_line`] and [`view_line`] write what
//! [`Replay::parse`] reads, so a writer in another crate cannot drift from
//! this parser.

use crate::Dmm;
use crate::clock::Clock;
use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::registry::{self, SelectableDevice};
use crate::protocol::{CaptureStep, Choice, DeviceProfile, Protocol, Setting};
use crate::specs::{ModeSpecInfo, SpecInfo};
use crate::transport::{Link, Transport};
use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
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

/// The `# link:` value for a cable recording. Files carry it, so it is part
/// of the format: it stays spelled this way whatever the apps call the link.
const LINK_USB_CABLE: &str = "USB cable";

/// The `# link:` value for a Bluetooth recording; fixed like [`LINK_USB_CABLE`].
const LINK_BLUETOOTH: &str = "Bluetooth";

/// What a recording with no link recorded is played back as.
///
/// Every replay file written before the link was recorded came off a cable,
/// and a session that says nothing about its link is less use than one that
/// says the thing all of them had in common.
const RECORDED_LINK_DEFAULT: Option<Link> = Some(Link::UsbCable);

/// The `# link:` value `link` is written as.
fn link_token(link: Link) -> &'static str {
    match link {
        Link::UsbCable => LINK_USB_CABLE,
        Link::Bluetooth => LINK_BLUETOOTH,
    }
}

/// The link a `# link:` value names.
///
/// `None` for anything else, so a recording made by a version that knows a
/// link this one does not still plays — it just says nothing about the link.
fn link_from_token(token: &str) -> Option<Link> {
    [Link::UsbCable, Link::Bluetooth]
        .into_iter()
        .find(|link| link_token(*link) == token)
}

/// A recorded session, parsed and ready to open.
///
/// Cheap to keep around and open more than once: a GUI reconnect re-opens the
/// same `Replay` rather than re-reading the file, and every open shares the
/// same frames rather than copying them.
pub struct Replay {
    /// The meter the recording was made from, resolved against the registry.
    pub device: &'static SelectableDevice,
    /// The `# recorded:` value, verbatim. This crate has no date library, so
    /// the binaries — which do — turn it into a wall time and hand it back
    /// through [`Clock::with_wall_origin`].
    pub recorded: String,
    /// The name the meter reported when the recording was made, if it has one.
    pub model: Option<String>,
    /// The link the recording came over — what a played-back session says
    /// it is on, since the playback itself has no cable or radio. A file with
    /// no `# link:` line is a cable recording; one naming a link this version
    /// does not know says nothing.
    pub link: Option<Link>,
    /// The markers the recording was saved with, in file order.
    pub markers: Vec<ReplayMarker>,
    /// The graph view the recording was saved with, as the `# view:` line's
    /// text: its fields are the viewer's business, not this crate's.
    pub view: Option<String>,
    /// Non-empty, offsets non-decreasing — both enforced by the parser.
    ///
    /// An `Arc<Vec>` rather than an `Arc<[_]>`: converting to a slice copies
    /// every slot while the file text is still alive, raising the load peak.
    samples: Arc<Vec<(Duration, Vec<u8>)>>,
    /// The [`cadence`], worked out when first asked rather than in `parse`,
    /// where its gap list would sit on top of the file text.
    cadence: OnceLock<Duration>,
}

/// A marker saved in a replay file: a `# marker:` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayMarker {
    /// How far into the session the marked reading was recorded.
    pub offset: Duration,
    /// The number the marker was shown and exported with.
    pub number: u32,
    /// Its note, possibly empty.
    pub note: String,
}

impl Replay {
    /// Parse a replay file's text.
    pub fn parse(text: &str) -> Result<Self> {
        let mut seen_magic = false;
        let mut device: Option<&'static SelectableDevice> = None;
        let mut recorded: Option<String> = None;
        let mut model: Option<String> = None;
        let mut link = RECORDED_LINK_DEFAULT;
        let mut samples: Vec<(Duration, Vec<u8>)> = Vec::new();
        let mut markers: Vec<ReplayMarker> = Vec::new();
        let mut view: Option<String> = None;

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
                } else if let Some(value) = comment.strip_prefix("link:") {
                    // A link we don't know is not a reason to refuse a
                    // recording: the frames are the file, the link is a label.
                    link = link_from_token(value.trim());
                } else if let Some(value) = comment.strip_prefix("view:") {
                    view = Some(value.trim().to_string()).filter(|v| !v.is_empty());
                } else if comment.starts_with("marker:") {
                    // From the untrimmed line: a note keeps its own spaces.
                    markers.push(parse_marker(line_no, raw)?);
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
        // Kept for the session: the growth slack would be kept with it.
        samples.shrink_to_fit();
        Ok(Self {
            device,
            recorded,
            model,
            link,
            markers,
            view,
            samples: Arc::new(samples),
            cadence: OnceLock::new(),
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
    /// origin was pinned before any sample was read. The session's transport
    /// reports the link the file was recorded over, so a caller names it the
    /// way it names a live meter's.
    pub fn open(&self, clock: Clock) -> Result<Dmm<ReplayTransport>> {
        let start = clock
            .wall_origin()
            .map(|(instant, _)| instant)
            .unwrap_or_else(|| clock.now());
        // Session time already gone by is history: a re-open onto an origin
        // pinned earlier — the GUI's Disconnect then Connect — picks the
        // recording up where the session has got to rather than from the top.
        let elapsed = clock.now().saturating_duration_since(start);
        let next = if elapsed > self.duration() && elapsed > GAP_SLICE {
            // Nothing recorded is still to come, so the session opens already
            // ended. Landing on the last frame instead would stamp it at its
            // own offset, before the readings the session has already handed
            // out — backwards on the graph, a duplicate after a quick
            // Disconnect then Connect, and an export of that buffer is a
            // replay file `parse` refuses. Only an open in the session's first
            // moments, a one-frame file's the moment its origin was pinned,
            // still plays the frame.
            self.samples.len()
        } else {
            // Of the samples already due only the newest is still on the wire,
            // the same rule `request_measurement` plays a file by.
            self.samples
                .partition_point(|(offset, _)| *offset <= elapsed)
                .saturating_sub(1)
        };
        let protocol = ReplayProtocol {
            inner: (self.device.new_protocol)(),
            samples: Arc::clone(&self.samples),
            model: self.model.clone(),
            clock: clock.clone(),
            start,
            next,
        };
        let transport = ReplayTransport { link: self.link };
        let dmm = Dmm::new(transport, Box::new(protocol))?;
        Ok(dmm.with_clock(clock).with_protocol_timestamps())
    }

    /// How often the recording has a frame, as a sample interval would say
    /// it: what a player measures its gaps against, rather than a viewer's
    /// own interval.
    pub fn cadence(&self) -> Duration {
        *self.cadence.get_or_init(|| cadence(&self.samples))
    }

    /// How far into the session the last sample was recorded.
    pub fn duration(&self) -> Duration {
        self.samples
            .last()
            .map(|(offset, _)| *offset)
            .unwrap_or(Duration::ZERO)
    }
}

/// What a replay session reads through: nothing, like the mock's transport —
/// the frames come from the file, through the protocol — except that it
/// reports the link the file was recorded over, where a live transport
/// reports its own.
pub struct ReplayTransport {
    link: Option<Link>,
}

impl Transport for ReplayTransport {
    fn write(&self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        Ok(0)
    }

    fn set_baud(&self, _baud: u32) -> Result<()> {
        Ok(())
    }

    fn link(&self) -> Option<Link> {
        self.link
    }
}

/// The header lines of a replay file, ending in a newline.
///
/// `link` is the link the readings came over; left out, the file plays back
/// as a cable recording, which every file written before the line existed
/// was. `recorded_rfc3339` is the first reading's wall time, which playback
/// pins the clock's wall origin to, so every writer must pass that one.
///
/// Callers append [`sample_line`]s to this.
pub fn header(
    device_id: &str,
    recorded_rfc3339: &str,
    model: Option<&str>,
    link: Option<Link>,
) -> String {
    let mut out = format!("{MAGIC}\n# device: {device_id}\n# recorded: {recorded_rfc3339}\n");
    if let Some(model) = model {
        out.push_str("# model: ");
        out.push_str(model);
        out.push('\n');
    }
    if let Some(link) = link {
        out.push_str("# link: ");
        out.push_str(link_token(link));
        out.push('\n');
    }
    out
}

/// One marker line: the marked reading's offset in milliseconds, the
/// marker's number and its note, which the line break would end — a line
/// break in it is written as a space.
pub fn marker_line(offset: Duration, number: u32, note: &str) -> String {
    let ms = u64::try_from(offset.as_millis()).unwrap_or(u64::MAX);
    let note: String = note
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();
    format!("# marker: {ms} {number} {note}\n")
}

/// The line saving the viewer's view of the recording, `view` being its
/// one-line text; a line break in it is written as a space.
pub fn view_line(view: &str) -> String {
    let view: String = view
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();
    format!("# view: {view}\n")
}

/// One sample line: the offset in milliseconds, then the payload as
/// uppercase hex bytes.
pub fn sample_line(offset: Duration, payload: &[u8]) -> String {
    // The parser reads the offset as a `u64`; saturating keeps a line
    // readable rather than emitting a number nothing can parse back.
    let ms = u64::try_from(offset.as_millis()).unwrap_or(u64::MAX);
    let mut out = ms.to_string();
    for byte in payload {
        // Formatted into the line rather than through a `String` per byte: a
        // render walks every sample of a buffer up to the half-million bound.
        let _ = write!(out, " {byte:02X}");
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

/// A `# marker:` line, from the line as written so the note keeps its own
/// spaces: `# marker: <offset ms> <number> <note>`, the note running to the
/// end of the line.
fn parse_marker(line_no: usize, raw: &str) -> Result<ReplayMarker> {
    let raw = raw.strip_suffix('\r').unwrap_or(raw);
    let rest = raw
        .split_once("marker:")
        .map_or("", |(_, rest)| rest)
        .trim_start();
    let (ms, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let (number, note) = rest.split_once(' ').unwrap_or((rest, ""));
    let Ok(ms) = u64::from_str(ms) else {
        return Err(at(
            line_no,
            format!("`{ms}` is not a marker's millisecond offset"),
        ));
    };
    let Ok(number) = u32::from_str(number) else {
        return Err(at(line_no, format!("`{number}` is not a marker number")));
    };
    Ok(ReplayMarker {
        offset: Duration::from_millis(ms),
        number,
        note: note.to_string(),
    })
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

/// The recording's usual spacing: its median gap, floored.
///
/// The median rather than the mean, so one pause in an otherwise steady
/// recording does not stretch it.
fn cadence(samples: &[(Duration, Vec<u8>)]) -> Duration {
    let mut gaps: Vec<Duration> = samples
        .iter()
        .zip(samples.iter().skip(1))
        .map(|((before, _), (after, _))| after.saturating_sub(*before))
        .collect();
    if gaps.is_empty() {
        return LONE_SAMPLE_CADENCE;
    }
    // The element a sort would put in the middle, without the sort.
    let middle = gaps.len() / 2;
    (*gaps.select_nth_unstable(middle).1).max(MIN_CADENCE)
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
    /// The [`Replay`]'s own frames, shared.
    samples: Arc<Vec<(Duration, Vec<u8>)>>,
    model: Option<String>,
    clock: Clock,
    /// Session instant the recording's t=0 maps to.
    start: Instant,
    /// Index of the next sample; `samples.len()` once the recording ended.
    next: usize,
}

impl ReplayProtocol {
    fn skip_to_newest_due(&mut self, now: Duration) {
        while self
            .samples
            .get(self.next.saturating_add(1))
            .is_some_and(|(offset, _)| *offset <= now)
        {
            self.next += 1;
        }
    }

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
    // Hands out every frame in order, each when it falls due, as a streaming
    // meter sends them; the stream keeps what the sample interval asks for.
    fn delivery(&self) -> crate::protocol::Delivery {
        crate::protocol::Delivery::Streamed
    }

    /// Nothing to bring up: the inner protocol's `init` would write a
    /// streaming trigger to a transport that answers nobody.
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        Ok(())
    }

    /// Skip every sample already due but the newest: what a meter nobody
    /// read meanwhile still has on the wire.
    fn discard_input(&mut self, _transport: &dyn Transport) -> Result<()> {
        let now = self.clock.now().saturating_duration_since(self.start);
        self.skip_to_newest_due(now);
        Ok(())
    }

    fn ended(&self) -> bool {
        self.next >= self.samples.len()
    }

    fn request_measurement(&mut self, _transport: &dyn Transport) -> Result<Measurement> {
        let now = self.clock.now().saturating_duration_since(self.start);
        let samples = Arc::clone(&self.samples);
        let Some((due, payload)) = samples.get(self.next) else {
            return Err(Error::Replay("the recording has ended".to_string()));
        };
        let due = *due;
        self.wait_until(due, now)?;
        // Past the sample before it is parsed: a frame the family refuses is
        // skipped, the way a corrupt frame off the wire is, rather than handed
        // to the next request again.
        self.next += 1;
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

    fn spec_info(&self, m: &Measurement) -> Option<&'static SpecInfo> {
        self.inner.spec_info(m)
    }

    fn mode_spec_info(&self, m: &Measurement) -> Option<&'static ModeSpecInfo> {
        self.inner.mode_spec_info(m)
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
    use std::time::SystemTime;

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
        let mut text = header("ut61eplus", RECORDED, Some("UT61E+"), None);
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
    fn open_manual() -> (Dmm<ReplayTransport>, Clock, Instant) {
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
            *replay.samples,
            vec![
                (Duration::ZERO, payload(DCV_BATTERY)),
                (Duration::from_millis(100), payload(DCV_NEGATIVE)),
                (Duration::from_millis(300), payload(OHM_82K)),
            ]
        );
    }

    /// The link is written and read back whole, a file that names none plays
    /// as the cable recording every early file was, and a link this version
    /// does not know costs the frames nothing.
    #[test]
    fn the_recorded_link_round_trips_and_falls_back_to_the_cable() {
        let mut text = header("ut61eplus", RECORDED, Some("UT61E+"), Some(Link::Bluetooth));
        // The on-disk spelling is what older and newer versions read back.
        assert!(text.contains("\n# link: Bluetooth\n"), "{text}");
        text.push_str(&sample_line(Duration::ZERO, &payload(DCV_BATTERY)));
        assert_eq!(parsed(&text).link, Some(Link::Bluetooth));

        assert_eq!(parsed(&three_frames()).link, Some(Link::UsbCable));
        let cable = three_frames().replacen('\n', "\n# link: USB cable\n", 1);
        assert_eq!(parsed(&cable).link, Some(Link::UsbCable));

        let unknown = text.replace("# link: Bluetooth", "# link: carrier pigeon");
        assert_eq!(parsed(&unknown).link, None);
    }

    /// A played-back session is on the link its file names, as far as
    /// anything asking the transport can tell, and on none when the file
    /// names one this version does not know.
    #[test]
    fn an_opened_replay_reports_the_recorded_link() {
        let mut text = header("ut61eplus", RECORDED, None, Some(Link::Bluetooth));
        text.push_str(&sample_line(Duration::ZERO, &payload(DCV_BATTERY)));
        let link = |text: &str| {
            let dmm = parsed(text)
                .open(Clock::manual())
                .expect("the replay opens");
            dmm.transport().link()
        };
        assert_eq!(link(&text), Some(Link::Bluetooth));
        assert_eq!(link(&three_frames()), Some(Link::UsbCable));
        let unknown = text.replace("# link: Bluetooth", "# link: carrier pigeon");
        assert_eq!(link(&unknown), None);
    }

    /// The `# link:` values are the file format: files already written carry
    /// these exact spellings, so the header keeps writing them byte for byte
    /// and each reads back as its link. The apps' own word for a link — the
    /// status line's "Bluetooth adapter", say — is not one of them.
    #[test]
    fn the_link_tokens_are_fixed() {
        let head = |link| header("ut61eplus", RECORDED, Some("UT61E+"), Some(link));
        let lead =
            format!("{MAGIC}\n# device: ut61eplus\n# recorded: {RECORDED}\n# model: UT61E+\n");
        assert_eq!(head(Link::UsbCable), format!("{lead}# link: USB cable\n"));
        assert_eq!(head(Link::Bluetooth), format!("{lead}# link: Bluetooth\n"));
        for link in [Link::UsbCable, Link::Bluetooth] {
            assert_eq!(link_from_token(link_token(link)), Some(link));
        }
        assert_eq!(link_from_token("Bluetooth adapter"), None);
        assert_eq!(link_from_token("carrier pigeon"), None);
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

    /// A marker line reads back as written, wherever it sits and whatever
    /// its note holds; a line break in the note is written as a space.
    #[test]
    fn marker_lines_round_trip() {
        let notes = ["probes on", "", "  # 3.3 V rail, see #2  ", "two\r\nlines"];
        let mut text = format!("{MAGIC}\n# device: ut61eplus\n# recorded: {RECORDED}\n");
        text.push_str(&marker_line(Duration::from_millis(250), 2, notes[1]));
        text.push_str(&format!("0 {DCV_BATTERY}\n"));
        text.push_str(&marker_line(Duration::ZERO, 1, notes[0]));
        text.push_str(&format!("500 {DCV_BATTERY}\n"));
        text.push_str(&marker_line(Duration::from_millis(500), 7, notes[2]));
        text.push_str(&marker_line(Duration::from_millis(500), 8, notes[3]));
        let replay = parsed(&text);
        let marker = |ms, number, note: &str| ReplayMarker {
            offset: Duration::from_millis(ms),
            number,
            note: note.to_string(),
        };
        assert_eq!(
            replay.markers,
            [
                marker(250, 2, ""),
                marker(0, 1, "probes on"),
                marker(500, 7, "  # 3.3 V rail, see #2  "),
                marker(500, 8, "two  lines"),
            ]
        );
        // A file written with Windows line ends reads the same.
        assert_eq!(parsed(&text.replace('\n', "\r\n")).markers, replay.markers);
        // A marker line trimmed of its trailing space still has an empty note.
        let bare = text.replace("# marker: 250 2 \n", "# marker: 250 2\n");
        assert_eq!(parsed(&bare).markers[0], marker(250, 2, ""));
    }

    /// The view line is kept as written, for the viewer to read.
    #[test]
    fn the_view_line_is_kept_as_text() {
        let mut text = format!("{MAGIC}\n# device: ut61eplus\n# recorded: {RECORDED}\n");
        text.push_str(&view_line("{\"window\":30.0,\n\"mean\":true}"));
        text.push_str(&format!("0 {DCV_BATTERY}\n"));
        assert_eq!(
            parsed(&text).view.as_deref(),
            Some("{\"window\":30.0, \"mean\":true}")
        );
        assert_eq!(parsed(&three_frames()).view, None);
    }

    #[test]
    fn a_malformed_marker_line_names_its_line() {
        let text = format!(
            "{MAGIC}\n# device: ut61eplus\n# recorded: {RECORDED}\n# marker: soon 1 x\n0 {DCV_BATTERY}\n"
        );
        let message = rejects(&text);
        assert!(
            message.contains("line 4") && message.contains("soon"),
            "got {message}"
        );
        let text = text.replace("soon 1", "10 first");
        assert!(rejects(&text).contains("`first` is not a marker number"));
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
        assert_eq!(
            rejects(&header("ut61eplus", RECORDED, None, None)),
            "no samples"
        );
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
        let text = format!("{}100\n", header("ut61eplus", RECORDED, None, None));
        assert!(rejects(&text).contains("no payload"));
    }

    /// The replay hands its readings to `Dmm::request_measurement`, which
    /// attaches the specs from the payload the reading carries: a UT804
    /// picks its table from bits that are only in the payload.
    #[test]
    fn a_replayed_reading_carries_its_specs() {
        // AC V on the 400V range, from `tests/golden/ut804/acv_mains.yaml`.
        const UT804_ACV_MAINS: &str = "32 32 37 32 B0 B3 32 31 31 0D 8A";
        let mut text = header("ut804", RECORDED, Some("UT804"), None);
        text.push_str(&sample_line(Duration::ZERO, &payload(UT804_ACV_MAINS)));
        let mut dmm = parsed(&text)
            .open(Clock::manual())
            .expect("the replay opens");
        let m = dmm.request_measurement().expect("the first frame is due");
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.spec.map(|s| s.resolution), Some("0.01V"));
        assert!(m.mode_spec.is_some_and(|ms| ms.input_impedance.is_some()));
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

    /// A caller away longer than `Dmm`'s resync bound misses frames exactly
    /// as an unread meter's are dropped.
    #[test]
    fn a_clock_that_jumps_past_two_samples_returns_the_newer() {
        let (mut dmm, clock, start) = open_manual();
        dmm.request_measurement().expect("the first frame");

        clock.advance(Duration::from_millis(400));
        let m = dmm.request_measurement().expect("the newest due frame");
        assert_eq!(m.value_export_str(), "80.45");
        assert_eq!(m.timestamp, start + Duration::from_millis(300));
    }

    /// The session says when the file has played out: after the last
    /// recorded frame, and from then on nothing more is read.
    #[test]
    fn the_session_says_when_the_recording_has_ended() {
        let (mut dmm, _clock, _start) = open_manual();
        for _ in 0..2 {
            dmm.request_measurement().expect("a recorded frame");
            assert!(!dmm.ended());
        }
        dmm.request_measurement().expect("the last frame");
        assert!(dmm.ended());
        let err = dmm.request_measurement().expect_err("nothing past the end");
        assert!(matches!(err, Error::Replay(_)), "got {err}");
    }

    /// Through the stream a replay plays as a streaming meter does: every
    /// frame at 0 ms, then the end, reported again on every later tick.
    #[test]
    fn the_stream_plays_every_frame_then_ends() {
        use crate::stream::{MeasurementStream, StreamEvent};
        let (mut dmm, _clock, start) = open_manual();
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);
        let mut stamps = Vec::new();
        for _ in 0..3 {
            match stream.tick().expect("a frame") {
                StreamEvent::Measurement(m) => stamps.push(m.timestamp - start),
                other => panic!("expected a frame, got {other:?}"),
            }
        }
        assert_eq!(stamps, [0, 100, 300].map(Duration::from_millis));
        assert!(matches!(stream.tick(), Ok(StreamEvent::Ended)));
        assert!(matches!(stream.tick(), Ok(StreamEvent::Ended)));
    }

    /// At an interval the recording's last frame is kept, due or not: it is
    /// what the meter showed when the recording stopped.
    #[test]
    fn an_interval_keeps_the_last_frame() {
        use crate::stream::{MeasurementStream, StreamEvent};
        let (mut dmm, _clock, _start) = open_manual();
        let mut stream = MeasurementStream::new(&mut dmm, Duration::from_secs(10));
        assert!(matches!(stream.tick(), Ok(StreamEvent::Measurement(_))));
        match stream.tick().expect("the last frame") {
            StreamEvent::Measurement(m) => assert_eq!(m.value_export_str(), "80.45"),
            other => panic!("expected the last frame, got {other:?}"),
        }
        assert!(matches!(stream.tick(), Ok(StreamEvent::Ended)));
    }

    /// A last frame the family refuses ends the file too.
    #[test]
    fn a_refused_last_frame_plays_the_recording_out() {
        let clock = Clock::manual();
        let mut dmm = parsed(&file(&[(0, DCV_BATTERY), (100, "02 30 20")]))
            .open(clock.clone())
            .expect("the replay opens");
        dmm.request_measurement().expect("the first frame");
        assert!(!dmm.ended());
        dmm.request_measurement()
            .expect_err("a truncated frame does not parse");
        assert!(dmm.ended());
    }

    /// A one-frame file opened on a clock already just past its only offset
    /// still plays that frame before it ends.
    #[test]
    fn a_one_frame_file_opened_late_plays_its_frame_first() {
        // The origin pinned before the open, as `read --replay` pins it; the
        // few moments between put the session past the file's only offset.
        let clock = Clock::manual().with_wall_origin(SystemTime::now());
        clock.advance(Duration::from_millis(5));
        let mut dmm = parsed(&file(&[(0, DCV_BATTERY)]))
            .open(clock.clone())
            .expect("the replay opens");
        assert!(!dmm.ended());
        let m = dmm.request_measurement().expect("the frame");
        assert_eq!(m.value_export_str(), "1.6109");
        assert!(dmm.ended());
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

    /// A GUI Disconnect then Connect re-opens the same recording on the same
    /// clock, origin and all. The session time that passed meanwhile is
    /// history, so a re-open well past the end has ended: its last frame again
    /// would be dated before the readings already taken, backwards on the
    /// graph and in an exported replay.
    #[test]
    fn a_reopen_past_the_end_has_ended() {
        let clock = Clock::manual().with_wall_origin(SystemTime::now());
        let replay = parsed(&three_frames());
        let mut dmm = replay.open(clock.clone()).expect("the replay opens");
        for _ in 0..3 {
            dmm.request_measurement().expect("a recorded frame");
        }
        drop(dmm);

        clock.advance(Duration::from_millis(7_400));
        let dmm = replay.open(clock.clone()).expect("the replay re-opens");
        assert!(dmm.ended());
    }

    /// A quick Disconnect then Connect after the end does not hand the last
    /// frame out again, at a timestamp already delivered.
    #[test]
    fn a_quick_reopen_after_the_end_has_ended() {
        let clock = Clock::manual().with_wall_origin(SystemTime::now());
        let replay = parsed(&file(&[(0, DCV_BATTERY), (900, OHM_82K)]));
        let mut dmm = replay.open(clock.clone()).expect("the replay opens");
        for _ in 0..2 {
            dmm.request_measurement().expect("a recorded frame");
        }
        drop(dmm);
        clock.advance(Duration::from_millis(200));
        assert!(
            replay
                .open(clock.clone())
                .expect("the replay re-opens")
                .ended()
        );
    }

    /// Re-opening partway through the recording resumes at the newest frame
    /// due, not at the top of the file.
    #[test]
    fn a_reopen_partway_through_resumes_at_the_newest_frame_due() {
        let clock = Clock::manual().with_wall_origin(SystemTime::now());
        let start = clock.now();
        let replay = parsed(&three_frames());
        drop(replay.open(clock.clone()).expect("the replay opens"));

        clock.advance(Duration::from_millis(150));
        let mut dmm = replay.open(clock.clone()).expect("the replay re-opens");
        let m = dmm.request_measurement().expect("the newest frame due");
        assert_eq!(m.value_export_str(), "-0.5137");
        assert_eq!(m.timestamp, start + Duration::from_millis(100));
    }

    /// Every open shares the parsed frames: a session holds them while it
    /// runs, and lets them go when it ends.
    #[test]
    fn a_reopen_shares_the_frames() {
        let clock = Clock::manual().with_wall_origin(SystemTime::now());
        let replay = parsed(&three_frames());
        assert_eq!(Arc::strong_count(&replay.samples), 1);
        let dmm = replay.open(clock.clone()).expect("the replay opens");
        assert_eq!(Arc::strong_count(&replay.samples), 2);
        drop(dmm);
        assert_eq!(Arc::strong_count(&replay.samples), 1);
    }

    /// The cadence is the median gap, odd or even in number, and the
    /// same element a sort would pick.
    #[test]
    fn the_cadence_is_the_median_gap() {
        let at = |ms: &[u64]| -> Vec<(Duration, Vec<u8>)> {
            ms.iter()
                .map(|&ms| (Duration::from_millis(ms), Vec::new()))
                .collect()
        };
        let sorted_median = |samples: &[(Duration, Vec<u8>)]| {
            let mut gaps: Vec<Duration> = samples
                .windows(2)
                .map(|w| w[1].0.saturating_sub(w[0].0))
                .collect();
            gaps.sort_unstable();
            gaps[gaps.len() / 2].max(MIN_CADENCE)
        };
        // Gaps 300, 100, 200: odd count.
        let odd = at(&[0, 300, 400, 600]);
        assert_eq!(cadence(&odd), Duration::from_millis(200));
        assert_eq!(cadence(&odd), sorted_median(&odd));
        // Gaps 400, 100, 300, 200: even count, the upper middle.
        let even = at(&[0, 400, 500, 800, 1000]);
        assert_eq!(cadence(&even), Duration::from_millis(300));
        assert_eq!(cadence(&even), sorted_median(&even));
        // A dense recording is floored.
        assert_eq!(cadence(&at(&[0, 1, 2, 3])), MIN_CADENCE);
        assert_eq!(cadence(&at(&[0])), LONE_SAMPLE_CADENCE);
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

        // DC V on the 2.2V range: the manual's 2.2000V row.
        let m = dmm.request_measurement().expect("a frame");
        assert_eq!(m.spec.map(|s| s.resolution), Some("0.1mV"));
        assert_eq!(
            m.mode_spec.and_then(|ms| ms.input_impedance),
            Some("About 10MΩ")
        );
        assert!(!dmm.choices(Setting::Range, &m).is_empty());
    }
}
