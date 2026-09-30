//! The auto-detection check that ends a capture run.
//!
//! A reporter names their meter with `--device`, which skips detection, so
//! the meters we don't own never go through it. The check runs last: the
//! protocol data is what the run is for, and a detection probe that upsets
//! the meter must not cost it. `docs/capture-design.md` (§B) has the why.

use super::input::Input;
use super::recording::{self, RecordingTransport, SharedRecorder};
use super::report::{FrameRecord, SampleData, tool_version};
use super::step::{DETECTION_STEP_ID, frames_for_step};
use super::watch::STABLE_FRAMES;
use console::{Key, style};
use dmm_lib::detect::Detected;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::{self, SelectableDevice};
use dmm_lib::protocol::{CaptureStep, Expect, ValueExpect};
use dmm_lib::transport::{Link, Transport};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// How long the read-back waits for the meter to settle in its mode before
/// filing what it last read.
const READ_BACK_TIMEOUT: Duration = Duration::from_secs(60);

/// How the check ended.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DetectionOutcome {
    /// Detection settled on a registry entry, the capture's or another.
    Found,
    /// The link opened and nothing on it was recognised.
    NotIdentified,
    /// The link the capture ran on could not be opened again.
    OpenFailed,
    /// The operator chose not to run it.
    Skipped,
}

/// What auto-detection made of the captured meter, filed beside the steps.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct DetectionCheck {
    pub outcome: DetectionOutcome,
    /// The registry entry detection picked.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub device_id: Option<String>,
    /// The model name the meter sent during detection.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reported_name: Option<String>,
    /// The bridge the capture ran on, which the check asks to open again.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub capture_bridge: Option<String>,
    /// The bridge the check opened.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bridge: Option<String>,
    /// The name a Bluetooth peer went by, which narrows the fingerprints run.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub advertised_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// The run itself was opened through detection: nothing was run again,
    /// and the probe is in `init_frames`.
    #[serde(skip_serializing_if = "is_false", default)]
    pub at_open: bool,
    /// The operator restarted the meter first, so detection met it as a
    /// user connecting it would — not left mid-stream by the capture.
    #[serde(default)]
    pub power_cycled: bool,
    /// The USB cable was unplugged while the meter was off, so the bridge
    /// came back empty — no frames left over from the capture — as on a
    /// first connection.
    #[serde(skip_serializing_if = "is_false", default)]
    pub cable_replugged: bool,
    /// The meter opened through what detection picked, and read in another
    /// function than the one it was detected in.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub read_back: Option<ReadBack>,
    /// How long detection took, the open excluded.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub elapsed_ms: Option<u64>,
    /// A resumed report keeps a section an older run wrote.
    #[serde(default)]
    pub tool_version: String,
    /// The probes and the replies to them.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub frames: Vec<FrameRecord>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl DetectionCheck {
    fn new(outcome: DetectionOutcome) -> Self {
        DetectionCheck {
            outcome,
            device_id: None,
            reported_name: None,
            capture_bridge: None,
            bridge: None,
            advertised_name: None,
            error: None,
            at_open: false,
            power_cycled: false,
            cable_replugged: false,
            read_back: None,
            elapsed_ms: None,
            tool_version: tool_version(),
            frames: vec![],
        }
    }

    fn found(&mut self, detected: &Detected) {
        self.device_id = Some(detected.device.id.to_string());
        self.reported_name = detected.reported_name.clone();
    }
}

/// How the read-back went.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReadBackOutcome {
    /// The meter settled in the step's mode.
    Matched,
    /// Readings came, but not the ones the step asks for: the operator took
    /// one early, or the wait ran out.
    Unexpected,
    /// Nothing decodable came.
    NoReading,
    /// The operator chose not to run it.
    Skipped,
}

/// The meter opened through detection's pick — its `init` and readings,
/// as an `auto` session gets them — and read at the family's Ω gate step.
/// Ω, where detection ran at DC V: a reading that followed the dial there
/// is live, not a frame left over from before the restart. That is also
/// where a probe from another family that switched the meter's function or
/// stopped its stream shows. Beeps and display-only changes go unseen: asking
/// the operator about them drew mostly the meter's own beep at its own probe
/// (a UT61E+ beeps at Get Name).
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReadBack {
    /// The capture step whose instruction and expectation were used.
    pub step: String,
    pub outcome: ReadBackOutcome,
    /// The reading before the operator was asked to turn the dial, which
    /// should be the DC V the meter was switched on at. One already in the
    /// step's mode means the dial was turned early, and the read-back no
    /// longer shows the readings following it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_reading: Option<SampleData>,
    /// The last reading, the one the outcome was decided on.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reading: Option<SampleData>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// The `init` exchange and the readings, newest kept.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub frames: Vec<FrameRecord>,
    #[serde(skip_serializing_if = "is_zero", default)]
    pub frames_dropped: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// Opens the link again, the way `auto` would but trying the bridge named
/// first. It can still land elsewhere; [`detect_on`] refuses that link.
pub(crate) type Reopen<'a> = Box<
    dyn FnOnce(&'static str) -> dmm_lib::error::Result<(Box<dyn Transport>, &'static str)> + 'a,
>;

/// Run the check, or file what the open already found when the run was
/// itself opened through detection. `None` leaves the report's section as it
/// is: for the mock, which has no link to detect anything on, and for a
/// resumed report whose section came from a restarted meter — that is the
/// evidence, and the operator is not asked for another restart.
///
/// `dmm` is closed before the operator restarts the meter, so a Bluetooth
/// link is let go of rather than dropped from the meter's end.
pub(super) fn run_detection_check(
    dmm: dmm_lib::Dmm<Box<dyn Transport>>,
    device: &'static SelectableDevice,
    detected_at_open: Option<&Detected>,
    previous: Option<&DetectionCheck>,
    input: &Input,
    reopen: Reopen<'_>,
) -> Result<Option<DetectionCheck>, Box<dyn std::error::Error>> {
    if !device.requires_hardware || previous.is_some_and(|p| p.power_cycled) {
        return Ok(None);
    }
    let capture_bridge = dmm.transport().transport_name();
    let capture_name = dmm.transport().advertised_name().map(str::to_string);
    let on_cable = matches!(dmm.transport().link(), Some(Link::UsbCable));
    if let Some(detected) = detected_at_open {
        let mut check = DetectionCheck::new(DetectionOutcome::Found);
        check.found(detected);
        check.at_open = true;
        check.capture_bridge = Some(capture_bridge.to_string());
        check.advertised_name = capture_name;
        return Ok(Some(check));
    }
    drop(dmm);
    // A piped run has nobody at the meter to restart it, and a check on a
    // meter the capture left mid-stream would be filed as one on a fresh one.
    if !input.is_tty() {
        return Ok(Some(DetectionCheck::new(DetectionOutcome::Skipped)));
    }

    eprintln!(
        "\n{}",
        style("\u{2501}\u{2501}\u{2501} Last check: auto-detection \u{2501}\u{2501}\u{2501}")
            .bold()
    );
    // Two prompts: on a dial meter off is a dial position, so "off and on
    // again at DC V" is not one thing to do.
    // A cable comes out too: a bridge can hold frames from the capture (the
    // CH9329 keeps them across an open), which detection would read as the
    // restarted meter's. A Bluetooth link is scanned for and joined afresh
    // anyway, and a meter with the radio built in restarts it.
    if on_cable {
        eprintln!("Turn the meter off and unplug its USB cable.");
    } else {
        eprintln!("Turn the meter off.");
    }
    if !confirmed(input, "Press Enter once it is off, or s to skip: ")? {
        return Ok(Some(DetectionCheck::new(DetectionOutcome::Skipped)));
    }
    // Some meters keep transmission on across a restart (the UT61E+), some
    // switch it off (the UT181A), so the steps are only for the second kind.
    if on_cable {
        eprintln!("Plug the cable back in and turn the meter on at DC V.");
    } else {
        eprintln!("Turn it back on at DC V.");
    }
    eprintln!("If the meter switched data transmission off, turn it back on:");
    for line in device.activation_instructions.lines() {
        eprintln!("{}", style(format!("  {line}")).dim());
    }
    if !confirmed(input, "Press Enter when it is ready, or s to skip: ")? {
        return Ok(Some(DetectionCheck::new(DetectionOutcome::Skipped)));
    }

    let (mut check, opened) = detect_on(reopen, capture_bridge, capture_name.as_deref());
    check.power_cycled = true;
    check.cable_replugged = on_cable;
    if let (Some(opened), Some(step)) = (opened, read_back_step(device)) {
        check.read_back = Some(read_back(opened, &step, input, READ_BACK_TIMEOUT));
    }
    let (line, advice) = closing_line(&check, device);
    if advice {
        eprintln!("{}", style(line).yellow());
    } else {
        eprintln!("{}", style(line).dim());
    }
    Ok(Some(check))
}

/// Enter goes on, `s` or `q` skips. Only Enter says the meter was restarted:
/// `q` ends every other prompt, and a stray key must not file a stale meter
/// as a fresh one.
fn confirmed(input: &Input, msg: &str) -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        match input.key(msg)? {
            '\n' => return Ok(true),
            's' | 'S' | 'q' | 'Q' => return Ok(false),
            _ => {}
        }
    }
}

/// A link detection identified a meter on, still open for the read-back.
struct Opened {
    transport: RecordingTransport,
    recorder: SharedRecorder,
    detected: Detected,
}

/// Open the capture's bridge again and let detection loose on it, recording
/// every byte. Nothing here fails the run: what went wrong is the finding.
///
/// A reopen that lands on another bridge, or a Bluetooth peer by another
/// name, is refused: detection would be probing some other meter, and its
/// answer would be filed as this one's.
fn detect_on(
    reopen: Reopen<'_>,
    capture_bridge: &'static str,
    capture_name: Option<&str>,
) -> (DetectionCheck, Option<Opened>) {
    eprintln!("{}", style("Checking auto-detection\u{2026}").dim());
    let mut check = DetectionCheck::new(DetectionOutcome::OpenFailed);
    check.capture_bridge = Some(capture_bridge.to_string());
    let (transport, bridge) = match reopen(capture_bridge) {
        Ok(opened) => opened,
        Err(e) => {
            check.error = Some(e.to_string());
            return (check, None);
        }
    };
    check.bridge = Some(bridge.to_string());
    check.advertised_name = transport.advertised_name().map(str::to_string);
    if bridge != capture_bridge
        || (capture_name.is_some() && check.advertised_name.as_deref() != capture_name)
    {
        check.error = Some(format!(
            "reopened {bridge}{} rather than the capture's link",
            check
                .advertised_name
                .as_deref()
                .map_or(String::new(), |name| format!(" ({name})"))
        ));
        return (check, None);
    }
    let (transport, recorder) = RecordingTransport::new(transport);
    let started = std::time::Instant::now();
    let result = dmm_lib::detect::detect_device(&transport, bridge);
    check.elapsed_ms = Some(started.elapsed().as_millis() as u64);
    check.frames = recording::lock(&recorder)
        .drain()
        .iter()
        .map(FrameRecord::from)
        .collect();
    match result {
        Ok(detected) => {
            check.outcome = DetectionOutcome::Found;
            check.found(&detected);
            let opened = Opened {
                transport,
                recorder,
                detected,
            };
            (check, Some(opened))
        }
        Err(e) => {
            check.outcome = DetectionOutcome::NotIdentified;
            check.error = Some(e.to_string());
            (check, None)
        }
    }
}

/// The step the read-back uses: the family's Ω gate step, open leads
/// reading OL. Every family's gate has one, and the words are the meter's
/// own (a dial legend, or "At Auto").
fn read_back_step(device: &SelectableDevice) -> Option<CaptureStep> {
    (device.new_protocol)()
        .capture_steps()
        .into_iter()
        .find(|s| {
            s.gate
                && s.expect
                    .is_some_and(|e| e.value == Some(ValueExpect::Overload))
        })
}

/// Open the meter through detection's pick and wait for it to settle in
/// `step`'s mode. Nothing here fails the run either.
fn read_back(opened: Opened, step: &CaptureStep, input: &Input, timeout: Duration) -> ReadBack {
    let mut out = ReadBack {
        step: step.id.to_string(),
        outcome: ReadBackOutcome::NoReading,
        start_reading: None,
        reading: None,
        error: None,
        frames: vec![],
        frames_dropped: 0,
    };
    let Opened {
        transport,
        recorder,
        detected,
    } = opened;
    recording::lock(&recorder).set_step(Some(DETECTION_STEP_ID));
    match dmm_lib::Dmm::from_detected(transport, &detected) {
        Ok(mut dmm) => {
            out.start_reading = dmm
                .request_measurement()
                .ok()
                .as_ref()
                .map(SampleData::from_measurement);
            eprintln!("Now: {}", step.instruction);
            eprintln!(
                "{}",
                style(
                    "The reading is taken once the meter settles \u{2014} Enter takes it now, \
                     s skips."
                )
                .dim()
            );
            // `read_back_step` only picks a step with an expectation.
            let expect = step.expect.unwrap_or_default();
            let (outcome, reading, error) = wait_for(&mut dmm, &expect, input, timeout);
            out.outcome = outcome;
            out.reading = reading.as_ref().map(SampleData::from_measurement);
            out.error = error;
        }
        Err(e) => out.error = Some(e.to_string()),
    }
    let events = recording::lock(&recorder).drain();
    (out.frames, out.frames_dropped) = frames_for_step(&events, DETECTION_STEP_ID);
    out
}

/// Read until `expect` holds for [`STABLE_FRAMES`] readings in a row, the
/// operator takes or skips, or `timeout` passes. Returns the outcome, the
/// last reading and the last error.
fn wait_for<T: Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    expect: &Expect,
    input: &Input,
    timeout: Duration,
) -> (ReadBackOutcome, Option<Measurement>, Option<String>) {
    let deadline = Instant::now() + timeout;
    let mut last: Option<Measurement> = None;
    let mut error: Option<String> = None;
    let mut run = 0;
    let decided = |last: Option<Measurement>, error: Option<String>| {
        let outcome = match &last {
            Some(m) if expect.check(m).is_ok() => ReadBackOutcome::Matched,
            Some(_) => ReadBackOutcome::Unexpected,
            None => ReadBackOutcome::NoReading,
        };
        (outcome, last, error)
    };
    while Instant::now() < deadline {
        match input.try_key() {
            // Only once there is a reading to take: a second Enter at the
            // ready prompt would otherwise end the wait before it starts.
            Ok(Some(Key::Enter)) if last.is_some() => break,
            Ok(Some(Key::Char('s' | 'S' | 'q' | 'Q'))) => {
                return (ReadBackOutcome::Skipped, last, error);
            }
            Ok(_) => {}
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        }
        match dmm.request_measurement() {
            Ok(m) => {
                run = if expect.check(&m).is_ok() { run + 1 } else { 0 };
                last = Some(m);
                if run >= STABLE_FRAMES {
                    return decided(last, error);
                }
            }
            Err(e) => {
                run = 0;
                error = Some(e.to_string());
                // A link that fails at once must not spin until the deadline.
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    decided(last, error)
}

/// The line the check ends on, and whether it carries advice. A pick other
/// than the capture's device — or none — means `auto` would open this meter
/// with the wrong tables or not at all, so the reporter is told what to keep
/// doing. Some of those are known and can't be fixed (a UT803 never
/// identifies, a UT71 reads as a UT804); the advice is right for them too.
fn closing_line(check: &DetectionCheck, device: &SelectableDevice) -> (String, bool) {
    // Nothing was probed, so there is nothing to say about the meter.
    if check.outcome == DetectionOutcome::OpenFailed {
        let why = check.error.as_deref().unwrap_or("no reason given");
        return (
            format!("Auto-detection was not checked: the link did not reopen ({why})."),
            true,
        );
    }
    let keep = format!(
        " \u{2014} keep using --device {}, or pick {} in dmm-gui.",
        device.id, device.display_name
    );
    let picked = check.device_id.as_deref().and_then(registry::find_device);
    match picked {
        Some(picked) if picked.id == device.id => {
            let name = picked.display_name;
            let Some(read_back) = &check.read_back else {
                return (format!("Auto-detection found {name}."), false);
            };
            let read = read_back.reading.as_ref().map(SampleData::summary);
            match (read_back.outcome, read) {
                (ReadBackOutcome::Matched, Some(read)) => (
                    format!("Auto-detection found {name} and read {read}."),
                    false,
                ),
                (ReadBackOutcome::Unexpected, Some(read)) => (
                    format!("Auto-detection found {name}, but then read {read}{keep}"),
                    true,
                ),
                (ReadBackOutcome::NoReading | ReadBackOutcome::Matched, _) => (
                    format!("Auto-detection found {name}, but then read nothing{keep}"),
                    true,
                ),
                _ => (format!("Auto-detection found {name}."), false),
            }
        }
        Some(picked) => (
            format!(
                "Auto-detection picked {} for this meter{keep}",
                picked.display_name
            ),
            true,
        ),
        None => (
            format!("Auto-detection did not find this meter{keep}"),
            true,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::Key;
    use dmm_lib::transport::mock::MockTransport;

    /// The two frames our UT61E+ answers Get Name with: the ack, then its
    /// name, each `AB CD` framed with a big-endian byte sum.
    const ACK: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];
    const NAME: [u8; 11] = [
        0xAB, 0xCD, 0x08, b'U', b'T', b'6', b'1', b'E', b'+', 0x03, 0x00,
    ];

    fn ut61eplus() -> &'static SelectableDevice {
        registry::find_device("ut61eplus").unwrap()
    }

    /// The link the capture ran on: a CP2110 nothing is read from any more.
    struct CaptureLink(Option<Link>);

    impl Transport for CaptureLink {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            Ok(0)
        }

        fn transport_name(&self) -> &'static str {
            "CP2110"
        }

        fn link(&self) -> Option<Link> {
            self.0
        }
    }

    fn capture_dmm() -> dmm_lib::Dmm<Box<dyn Transport>> {
        dmm_on(None)
    }

    fn dmm_on(link: Option<Link>) -> dmm_lib::Dmm<Box<dyn Transport>> {
        dmm_lib::Dmm::new(
            Box::new(CaptureLink(link)) as Box<dyn Transport>,
            (ut61eplus().new_protocol)(),
        )
        .unwrap()
    }

    /// A UT61E+ that answers the first probe it is sent, as ours does.
    fn answering_ut61eplus() -> Box<dyn Transport> {
        let meter = MockTransport::new(vec![]);
        meter.push_reply(ACK.to_vec());
        meter.push_reply(NAME.to_vec());
        Box::new(meter)
    }

    fn typed(keys: &[Key]) -> Input {
        let (tx, input) = Input::typed();
        for key in keys {
            tx.send(key.clone()).unwrap();
        }
        // Keep the channel open: a closed one reads as a keyboard gone away.
        std::mem::forget(tx);
        input
    }

    #[test]
    fn a_meter_detection_names_is_filed_with_its_bytes() {
        // The read-back is skipped: this meter sends no readings.
        let input = typed(&[Key::Enter, Key::Enter, Key::Char('s')]);
        let asked = std::cell::Cell::new(None);
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &input,
            Box::new(|bridge| {
                asked.set(Some(bridge));
                Ok((answering_ut61eplus(), bridge))
            }),
        )
        .unwrap()
        .unwrap();

        assert_eq!(check.outcome, DetectionOutcome::Found);
        assert_eq!(check.device_id.as_deref(), Some("ut61eplus"));
        assert_eq!(check.reported_name.as_deref(), Some("UT61E+"));
        assert_eq!(check.bridge.as_deref(), Some("CP2110"));
        assert!(check.power_cycled);
        assert!(!check.cable_replugged, "no cable to unplug");
        let read_back = check.read_back.as_ref().expect("a found meter is read");
        assert_eq!(read_back.step, "ohm");
        assert_eq!(read_back.outcome, ReadBackOutcome::Skipped);
        assert!(check.elapsed_ms.is_some());
        assert_eq!(asked.get(), Some("CP2110"), "the capture's bridge first");
        let hex: Vec<&str> = check.frames.iter().map(|f| f.hex.as_str()).collect();
        assert_eq!(hex[0], "AB CD 03 5F 01 DA", "Get Name goes out first");
        assert!(
            hex.contains(&"AB CD 04 FF 00 02 7B AB CD 08 55 54 36 31 45 2B 03 00"),
            "{hex:?}"
        );
    }

    #[test]
    fn a_silent_link_is_not_identified_and_the_run_goes_on() {
        // A stray key at a restart prompt is asked again, not taken as done.
        let input = typed(&[Key::Char('x'), Key::Enter, Key::Enter]);
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &input,
            Box::new(|bridge| Ok((Box::new(MockTransport::new(vec![])), bridge))),
        )
        .unwrap()
        .unwrap();

        assert_eq!(check.outcome, DetectionOutcome::NotIdentified);
        assert_eq!(check.device_id, None);
        assert!(check.error.is_some());
        assert!(check.read_back.is_none(), "nothing to read through");
        assert!(!check.frames.is_empty(), "the probes are kept");
    }

    #[test]
    fn a_link_that_will_not_reopen_is_recorded_without_a_question() {
        let input = typed(&[Key::Enter, Key::Enter]);
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &input,
            Box::new(|_| {
                Err(dmm_lib::error::Error::NoTransportFound {
                    cables: vec!["CP2110"],
                    bluetooth_searched: false,
                })
            }),
        )
        .unwrap()
        .unwrap();

        assert_eq!(check.outcome, DetectionOutcome::OpenFailed);
        assert!(check.error.is_some());
        assert!(check.read_back.is_none());
    }

    #[test]
    fn skipping_never_opens_the_link() {
        let input = typed(&[Key::Char('s')]);
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &input,
            Box::new(|_| panic!("a skipped check opened the link")),
        )
        .unwrap()
        .unwrap();

        assert_eq!(check.outcome, DetectionOutcome::Skipped);
        assert!(!check.power_cycled);
    }

    /// A capture over a USB cable has it replugged along with the restart.
    #[test]
    fn a_cable_capture_records_the_replug() {
        let check = run_detection_check(
            dmm_on(Some(Link::UsbCable)),
            ut61eplus(),
            None,
            None,
            &typed(&[Key::Enter, Key::Enter, Key::Char('s')]),
            Box::new(|bridge| Ok((answering_ut61eplus(), bridge))),
        )
        .unwrap()
        .unwrap();
        assert_eq!(check.outcome, DetectionOutcome::Found);
        assert!(check.power_cycled);
        assert!(check.cable_replugged);
    }

    /// The meter is off, and the operator stops there.
    #[test]
    fn skipping_once_the_meter_is_off_never_opens_the_link() {
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &typed(&[Key::Enter, Key::Char('s')]),
            Box::new(|_| panic!("a skipped check opened the link")),
        )
        .unwrap()
        .unwrap();
        assert_eq!(check.outcome, DetectionOutcome::Skipped);
        assert!(!check.power_cycled);
    }

    #[test]
    fn q_skips_as_it_ends_every_other_prompt() {
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &typed(&[Key::Char('q')]),
            Box::new(|_| panic!("q opened the link")),
        )
        .unwrap()
        .unwrap();
        assert_eq!(check.outcome, DetectionOutcome::Skipped);
    }

    /// Nobody at a piped run can restart the meter, so nothing is probed.
    #[test]
    fn a_piped_run_skips_the_check() {
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &Input::piped(),
            Box::new(|_| panic!("a piped run opened the link")),
        )
        .unwrap()
        .unwrap();
        assert_eq!(check.outcome, DetectionOutcome::Skipped);
    }

    /// A resumed report keeps the check its restarted meter answered.
    #[test]
    fn a_restarted_meters_check_is_kept_on_resume() {
        let mut previous = DetectionCheck::new(DetectionOutcome::Found);
        previous.power_cycled = true;
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            Some(&previous),
            &typed(&[]),
            Box::new(|_| panic!("the report already has the check")),
        )
        .unwrap();
        assert!(check.is_none());
    }

    /// A reopen that lands on another cable would probe another meter.
    #[test]
    fn a_reopen_on_another_bridge_is_refused() {
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            None,
            None,
            &typed(&[Key::Enter, Key::Enter]),
            Box::new(|_| Ok((answering_ut61eplus(), "CH9329"))),
        )
        .unwrap()
        .unwrap();
        assert_eq!(check.outcome, DetectionOutcome::OpenFailed);
        assert_eq!(check.device_id, None);
        assert!(check.frames.is_empty(), "nothing was probed");
        let (line, _) = closing_line(&check, ut61eplus());
        assert!(line.starts_with("Auto-detection was not checked"), "{line}");
    }

    /// A run opened through detection has already been through it: the
    /// open's result is filed, and the operator is not asked anything.
    #[test]
    fn a_run_opened_through_detection_files_the_open() {
        let input = typed(&[]);
        let detected = Detected {
            device: ut61eplus(),
            reported_name: Some("UT61E+".to_string()),
        };
        let check = run_detection_check(
            capture_dmm(),
            ut61eplus(),
            Some(&detected),
            None,
            &input,
            Box::new(|_| panic!("the open already detected the meter")),
        )
        .unwrap()
        .unwrap();

        assert_eq!(check.outcome, DetectionOutcome::Found);
        assert!(check.at_open);
        assert_eq!(check.device_id.as_deref(), Some("ut61eplus"));
        assert!(check.frames.is_empty(), "they are in init_frames");
    }

    #[test]
    fn the_mock_has_nothing_to_detect() {
        let mock = registry::find_device("mock").unwrap();
        let dmm = dmm_lib::Dmm::new(
            Box::new(dmm_lib::transport::NullTransport) as Box<dyn Transport>,
            (mock.new_protocol)(),
        )
        .unwrap();
        let check = run_detection_check(
            dmm,
            mock,
            None,
            None,
            &typed(&[]),
            Box::new(|_| panic!("the mock has no link")),
        )
        .unwrap();
        assert!(check.is_none());
    }

    #[test]
    fn the_closing_line_advises_only_when_auto_would_not_open_this_meter() {
        let mut check = DetectionCheck::new(DetectionOutcome::Found);
        check.device_id = Some("ut61eplus".to_string());
        let (line, advice) = closing_line(&check, ut61eplus());
        assert_eq!(line, "Auto-detection found UT61E+.");
        assert!(!advice);

        let ut71 = registry::find_device("ut71cde").unwrap();
        check.device_id = Some("ut804".to_string());
        let (line, advice) = closing_line(&check, ut71);
        assert!(advice);
        assert!(
            line.starts_with("Auto-detection picked UT804 for this meter"),
            "{line}"
        );
        assert!(line.contains("--device ut71cde"), "{line}");

        let (line, advice) =
            closing_line(&DetectionCheck::new(DetectionOutcome::NotIdentified), ut71);
        assert!(advice);
        assert!(
            line.starts_with("Auto-detection did not find this meter"),
            "{line}"
        );
    }

    fn simulated(mode: dmm_lib::mock::MockMode) -> dmm_lib::Dmm<dmm_lib::transport::NullTransport> {
        let mock = registry::find_device("mock").unwrap();
        dmm_lib::mock::open_simulated(mock, Some(mode), dmm_lib::Clock::real()).unwrap()
    }

    fn ohm_expect() -> Expect {
        read_back_step(ut61eplus()).unwrap().expect.unwrap()
    }

    #[test]
    fn a_meter_settled_at_ol_is_read_back() {
        let mut dmm = simulated(dmm_lib::mock::MockMode::OhmOl);
        let (outcome, reading, _) = wait_for(
            &mut dmm,
            &ohm_expect(),
            &typed(&[]),
            Duration::from_secs(10),
        );
        assert_eq!(outcome, ReadBackOutcome::Matched);
        assert!(reading.is_some());
    }

    /// A meter left at DC V never settles at Ω: the wait runs out, and the
    /// last reading is filed as not what was asked for.
    #[test]
    fn a_meter_left_in_another_mode_is_unexpected() {
        let mut dmm = simulated(dmm_lib::mock::MockMode::DcV);
        let (outcome, reading, _) = wait_for(
            &mut dmm,
            &ohm_expect(),
            &typed(&[]),
            Duration::from_millis(500),
        );
        assert_eq!(outcome, ReadBackOutcome::Unexpected);
        assert_eq!(reading.expect("the mock reads").mode, "DC V");
    }

    #[test]
    fn s_skips_the_read_back() {
        let mut dmm = simulated(dmm_lib::mock::MockMode::OhmOl);
        let (outcome, _, _) = wait_for(
            &mut dmm,
            &ohm_expect(),
            &typed(&[Key::Char('s')]),
            Duration::from_secs(10),
        );
        assert_eq!(outcome, ReadBackOutcome::Skipped);
    }

    /// Every meter detection can find has a step to read it back at.
    #[test]
    fn every_meter_has_a_read_back_step() {
        for device in registry::DEVICES.iter().filter(|d| d.requires_hardware) {
            assert!(read_back_step(device).is_some(), "{}", device.id);
            let steps = (device.new_protocol)().capture_steps();
            assert!(
                !steps.iter().any(|s| s.id == DETECTION_STEP_ID),
                "{} declares a step named {DETECTION_STEP_ID}",
                device.id
            );
        }
    }

    #[test]
    fn the_closing_line_reports_the_read_back() {
        let mut check = DetectionCheck::new(DetectionOutcome::Found);
        check.device_id = Some("ut61eplus".to_string());
        let reading = simulated(dmm_lib::mock::MockMode::OhmOl)
            .request_measurement()
            .unwrap();
        let summary = SampleData::from_measurement(&reading).summary();
        check.read_back = Some(ReadBack {
            step: "ohm".to_string(),
            outcome: ReadBackOutcome::Matched,
            start_reading: None,
            reading: Some(SampleData::from_measurement(&reading)),
            error: None,
            frames: vec![],
            frames_dropped: 0,
        });
        let (line, advice) = closing_line(&check, ut61eplus());
        assert_eq!(
            line,
            format!("Auto-detection found UT61E+ and read {summary}.")
        );
        assert!(!advice);

        check.read_back.as_mut().unwrap().outcome = ReadBackOutcome::Unexpected;
        let (line, advice) = closing_line(&check, ut61eplus());
        assert!(line.contains("but then read"), "{line}");
        assert!(line.contains("--device ut61eplus"), "{line}");
        assert!(advice);

        let read_back = check.read_back.as_mut().unwrap();
        read_back.outcome = ReadBackOutcome::NoReading;
        read_back.reading = None;
        let (line, advice) = closing_line(&check, ut61eplus());
        assert!(line.contains("read nothing"), "{line}");
        assert!(advice);
    }

    /// Reports written before the check existed still load, so they resume.
    #[test]
    fn a_report_without_the_section_still_loads() {
        let yaml = "date: '2026-01-01'\ntool_version: test\ndevice_name: UT61E+\n\
                    supported: true\nsteps: []\n";
        let report: super::super::CaptureReport = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(report.detection.is_none());
    }
}
