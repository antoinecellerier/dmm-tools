use dmm_lib::binary_help::Link;
use dmm_lib::detect::Detected;
use dmm_lib::error::ErrorKind;
use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::protocol::{Choice, MeterKeys, Setting, Stability};
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transport::Transport;
use eframe::egui;
use log::{error, info, warn};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

/// Control messages from the UI to the background thread.
pub(crate) enum ThreadControl {
    /// Exit the loop and release the device.
    Stop,
    /// Halt (`true`) or resume (`false`) acquisition. Halting stops the meter
    /// being polled at all — it is not a display-side freeze.
    SetPaused(bool),
}

/// A command the UI asks the acquisition thread to send to the meter.
pub(crate) enum RemoteCommand {
    /// A named button command (`hold`, `range`, …) from the remote controls.
    Named(String),
    /// Switch a setting to one of the values `Dmm::choices` listed, by its id.
    Select(Setting, u16),
}

impl std::fmt::Display for RemoteCommand {
    /// The subject of the failure toast: "Command 'hold' failed: …".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Named(name) => write!(f, "Command '{name}'"),
            Self::Select(Setting::Mode, _) => f.write_str("Mode switch"),
            Self::Select(Setting::Range, _) => f.write_str("Range switch"),
            Self::Select(setting, _) => write!(f, "{setting} switch"),
        }
    }
}

/// Upper bound on the configured sample interval.
///
/// `sample_interval_ms` is deserialized with `#[serde(default)]` and never
/// validated, so a hand-edited `settings.json` can ask for minutes between
/// samples. The meter then looks dead with nothing on screen explaining why.
/// The UI presets top out at 2 s; this leaves generous room above them while
/// keeping a mistyped value diagnosable.
const MAX_SAMPLE_INTERVAL_MS: u32 = 60_000;

/// Consecutive read timeouts after which the meter is treated as not
/// responding — surfaced to the user, and marked on the graph as a genuine
/// loss of data rather than a quiet meter.
pub(super) const NO_RESPONSE_TIMEOUTS: u32 = 5;

/// What the UI records as the failure once that threshold is crossed.
pub(super) const NO_RESPONSE: &str = "No response from meter \u{2014} check device selection and \
                                      data transmission";

/// How often a paused thread wakes to look for work.
///
/// Nothing is read from the meter while paused, but device commands (HOLD,
/// REL, RANGE, …) are user actions rather than acquisition, so they must still
/// reach the meter promptly instead of queueing until resume.
const PAUSE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Apply pending control messages, blocking while paused.
///
/// Returns `false` when the thread should exit: an explicit `Stop`, or a
/// hung-up channel. The hang-up case matters — the UI dropping its sender
/// without stopping us (a panic on the UI thread, or a `connect()` that
/// replaced the channel) used to be indistinguishable from "no messages", so
/// the thread kept the USB handle open and polled the meter forever.
fn handle_control(ctrl_rx: &mpsc::Receiver<ThreadControl>, paused: &mut bool) -> bool {
    loop {
        let msg = if *paused {
            match ctrl_rx.recv_timeout(PAUSE_POLL_INTERVAL) {
                Ok(m) => m,
                Err(mpsc::RecvTimeoutError::Timeout) => return true,
                Err(mpsc::RecvTimeoutError::Disconnected) => return false,
            }
        } else {
            match ctrl_rx.try_recv() {
                Ok(m) => m,
                Err(mpsc::TryRecvError::Empty) => return true,
                Err(mpsc::TryRecvError::Disconnected) => return false,
            }
        };
        match msg {
            ThreadControl::Stop => return false,
            ThreadControl::SetPaused(p) => *paused = p,
        }
    }
}

/// Messages from the background thread to the UI.
pub(crate) enum DmmMessage {
    Measurement(Measurement),
    Connected {
        name: String,
        /// Model the connected protocol reports. Names the meter in the
        /// experimental warning, where the registry entry's display name
        /// would be second-hand.
        model_name: String,
        /// Registry entry behind that protocol — what detection settled on,
        /// or the entry the user picked. `None` only if a protocol ever
        /// reports a model no entry claims.
        device_id: Option<&'static str>,
        /// How far the protocol is verified; anything short of `Verified`
        /// shows the badge, and the level words its hover text.
        stability: Stability,
        /// URL for reporting feedback on experimental protocols.
        feedback_url: String,
        /// What the meter answered over, for the status line — for a replay,
        /// the link its recording was made on. `None` for the mock, which is
        /// on no link at all.
        link: Option<Link>,
        supported_commands: Vec<String>,
        /// Function and context keys, from the profile.
        meter_keys: MeterKeys,
        /// Sub-value slots this meter family can report, from its profile.
        /// Fixes the CSV export's aux column count for the whole recording.
        max_aux_values: usize,
    },
    /// Link lost mid-acquisition; the thread is about to start reconnecting.
    Disconnected(dmm_lib::error::Error),
    /// Reconnect attempt in progress — `attempt` is 1-based.
    /// `last_error` is the most recent reconnect failure, if any.
    Reconnecting {
        attempt: u32,
        last_error: Option<String>,
    },
    /// A library failure, carried whole rather than pre-formatted: the UI
    /// classifies it by [`ErrorKind`] and by variant, which a message string
    /// can only be string-matched back into.
    Error(dmm_lib::error::Error),
    /// A failure the GUI itself diagnosed, such as a panicking thread. No
    /// library error stands behind these.
    ErrorText(String),
    /// Nothing has been read for [`NO_RESPONSE_TIMEOUTS`] polls running. Its
    /// own message rather than an [`DmmMessage::ErrorText`]: a replay's gaps
    /// come through here too, and they are not a meter to go looking for.
    NoResponse,
    /// A command the user sent was refused or could not be sent. Shown as a
    /// toast, not as a connection issue: the link is fine and the meter is
    /// still streaming, so neither the help text nor a reconnect applies.
    CommandFailed(String),
    /// Values a setting can be switched to from where the meter sits now.
    /// Sent when the reading they depend on changes and after a successful
    /// switch; empty for families that cannot drive the setting.
    Choices(Setting, Vec<Choice>),
    /// Waiting for meter response (consecutive timeout count).
    WaitingForMeter(u32),
}

/// Extract profile info from a newly opened device, optionally query its name,
/// and send a `Connected` message to the UI.
///
/// `detected` is what identified the meter when nothing named it; `selected`
/// is the entry the user picked. Exactly one of them is set, and together they
/// tell the UI which meter it is now looking at.
///
/// `recorded_link` is the link a replay file was recorded over — a playback's
/// own transport is no link at all, so the recording is the only thing that
/// can say. `None` everywhere else, where the transport answers.
fn establish_connection<T: Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    detected: Option<Detected>,
    selected: Option<&'static SelectableDevice>,
    query_name: bool,
    recorded_link: Option<Link>,
    msg_tx: &mpsc::Sender<DmmMessage>,
    ctx: &egui::Context,
) {
    let profile = dmm.profile();
    let stability = profile.stability;
    let feedback_url = profile.feedback_url();
    let cmds: Vec<String> = profile
        .supported_commands
        .iter()
        .map(|s| s.to_string())
        .collect();
    // Read before `get_name`, which borrows the device mutably.
    let max_aux_values = profile.max_aux_values;
    let meter_keys = profile.meter_keys;
    let model_name = profile.model_name.to_string();
    let link = recorded_link.or_else(|| Link::from_bridge(dmm.transport().transport_name()));
    let device_id = detected
        .as_ref()
        .map(|d| d.device)
        .or(selected)
        .map(|d| d.id);
    // A name the meter already gave on this link — to detection, say — is
    // shown either way: it costs nothing more. Asking is what beeps.
    let name = if query_name {
        dmm.get_name().ok().flatten()
    } else {
        dmm.known_name().map(str::to_owned)
    };
    let name = name.unwrap_or_default();
    let _ = msg_tx.send(DmmMessage::Connected {
        name,
        model_name,
        device_id,
        stability,
        feedback_url,
        link,
        supported_commands: cmds,
        meter_keys,
        max_aux_values,
    });
    ctx.request_repaint();
}

/// Channels and settings the acquisition thread needs, besides the opener.
pub(super) struct ThreadContext {
    pub msg_tx: mpsc::Sender<DmmMessage>,
    pub ctrl_rx: mpsc::Receiver<ThreadControl>,
    pub cmd_rx: mpsc::Receiver<RemoteCommand>,
    pub ctx: egui::Context,
    /// The meter the user picked, `None` when the opener detects it instead.
    /// The opener already knows; this is how the *reporting* side learns it,
    /// so a named meter reaches the UI as a registry entry too.
    pub selected: Option<&'static SelectableDevice>,
    pub query_name: bool,
    /// The link a replay file was recorded over, for the status line. `None`
    /// for a meter and for the mock: their transport names their link, or
    /// says there is none.
    pub recorded_link: Option<Link>,
    pub sample_interval_ms: u32,
    pub stop_flag: Arc<AtomicBool>,
}

/// The settings the acquisition thread lists for the readout dropdowns, in
/// the order they are drawn. The others stay on the remote-control buttons;
/// adding one here is what puts it on the readout.
const LISTED_SETTINGS: [Setting; 2] = [Setting::Mode, Setting::Range];

/// The lists last sent for each of [`LISTED_SETTINGS`], keyed by the reading
/// they were built from. `None` — a fresh connection, or a setting the user
/// just changed — forces a re-list.
type ListedKeys = [Option<ChoiceKey>; LISTED_SETTINGS.len()];

/// What a setting's choice list depends on in the reading it was listed for.
///
/// Re-listing is driven by this key moving rather than by every sample: the
/// list is the same for every reading in between, and building it runs on the
/// acquisition thread's hot path.
#[derive(Clone, PartialEq, Eq)]
enum ChoiceKey {
    /// The mode word plus the decoded text. The text is part of the key
    /// because the choice list is not always a pure function of the word. The
    /// mock device keeps one word per dial position and cycles between
    /// scenarios underneath it (Temp, TempDual and TempDiff all report 0x0A),
    /// and a family whose choices depend on more than the word would behave
    /// the same way. Keying on the word alone leaves the dropdown marking a
    /// mode the meter has already left, so picking the live one does nothing.
    Mode(u16, Cow<'static, str>),
    /// The mode word, the live rung, and whether the meter is auto-ranging.
    /// The ladder comes from the mode; the mark moves with the rung; and
    /// auto ↔ manual moves the mark without the rung changing at all.
    Range(u16, u8, bool),
}

impl ChoiceKey {
    /// The key `setting`'s list is built under in `m`.
    fn of(setting: Setting, m: &Measurement) -> Self {
        match setting {
            Setting::Range => Self::Range(m.mode_raw, m.range_raw, m.flags.auto_range),
            // Everything else follows the dial: what a meter can be switched
            // to is decided by the mode it is in.
            _ => Self::Mode(m.mode_raw, m.mode.clone()),
        }
    }
}

/// Whether `setting` needs re-listing for `m`, given the key its list was
/// last sent under.
fn choices_stale(last: Option<&ChoiceKey>, setting: Setting, m: &Measurement) -> bool {
    last != Some(&ChoiceKey::of(setting, m))
}

/// Send the lists whose key moved with this reading, and record the new keys.
///
/// Split out of the acquisition loop so it can be driven from a test without
/// a device on the other end.
fn send_stale_choices<T: Transport>(
    dmm: &dmm_lib::Dmm<T>,
    m: &Measurement,
    keys: &mut ListedKeys,
    msg_tx: &mpsc::Sender<DmmMessage>,
) {
    for (slot, setting) in keys.iter_mut().zip(LISTED_SETTINGS) {
        if !choices_stale(slot.as_ref(), setting, m) {
            continue;
        }
        *slot = Some(ChoiceKey::of(setting, m));
        let _ = msg_tx.send(DmmMessage::Choices(setting, dmm.choices(setting, m)));
    }
}

/// Forget the key of a setting the user just changed, so the next reading
/// re-lists it. Which entry is live has changed even where the reading has
/// not — the mock keeps one mode word per dial position.
fn invalidate(keys: &mut ListedKeys, setting: Setting) {
    if let Some(i) = LISTED_SETTINGS.iter().position(|s| *s == setting) {
        keys[i] = None;
    }
}

/// Run the measurement loop on a background thread, generic over transport type.
pub(super) fn run_device_thread<T, F>(open_fn: F, thread_ctx: ThreadContext)
where
    T: Transport + Send + 'static,
    F: Fn(Option<&str>) -> dmm_lib::error::Result<(dmm_lib::Dmm<T>, Option<Detected>)>
        + Send
        + 'static,
{
    let ThreadContext {
        msg_tx,
        ctrl_rx,
        cmd_rx,
        ctx,
        selected,
        query_name,
        recorded_link,
        sample_interval_ms,
        stop_flag,
    } = thread_ctx;

    info!("background thread: connecting to device");
    let mut dmm = match open_fn(None) {
        Ok((mut d, detected)) => {
            establish_connection(
                &mut d,
                detected,
                selected,
                query_name,
                recorded_link,
                &msg_tx,
                &ctx,
            );
            d
        }
        Err(e) => {
            let _ = msg_tx.send(DmmMessage::Error(e));
            ctx.request_repaint();
            return;
        }
    };

    // How often to re-report an ongoing protocol error to the UI. The first
    // one is always reported; repeats are throttled so a meter parked in an
    // unparseable state doesn't flood the channel.
    const PROTOCOL_ERROR_REPORT_INTERVAL: u32 = 20;

    if sample_interval_ms > MAX_SAMPLE_INTERVAL_MS {
        warn!(
            "sample_interval_ms {sample_interval_ms} exceeds the {MAX_SAMPLE_INTERVAL_MS} ms \
             maximum, clamping"
        );
    }
    let tick = Duration::from_millis(sample_interval_ms.min(MAX_SAMPLE_INTERVAL_MS) as u64);
    // Let the pacing sleep observe the stop request too. Without it a 2 s
    // interval keeps the USB handle open for the rest of the tick (plus the
    // read timeout) after the user clicks Disconnect, while the UI already
    // shows Disconnected and offers Connect again.
    // The Bluetooth adapter this session is on, which a reconnect goes back
    // to by address rather than scanning for one again.
    let mut reopen_at = bluetooth_selector(&dmm);
    let sleep_stop = Arc::clone(&stop_flag);
    let mut stream = MeasurementStream::new(&mut dmm, tick)
        .with_cancel(move || sleep_stop.load(Ordering::Relaxed));
    let mut protocol_errors: u32 = 0;
    let mut paused = false;
    let mut last_keys: ListedKeys = Default::default();
    loop {
        if stop_flag.load(Ordering::Relaxed) || !handle_control(&ctrl_rx, &mut paused) {
            info!("background thread: stopping");
            break;
        }

        // Process any pending remote commands. Goes through the stream's
        // `dmm_mut()` so the underlying `Dmm` stays owned by the stream
        // across command sends and doesn't reset its tick schedule.
        while let Ok(cmd) = cmd_rx.try_recv() {
            let result = match &cmd {
                RemoteCommand::Named(name) => stream.dmm_mut().send_command(name),
                RemoteCommand::Select(setting, id) => stream.dmm_mut().select(*setting, *id),
            };
            match (result, &cmd) {
                (Ok(()), RemoteCommand::Select(setting, _)) => {
                    invalidate(&mut last_keys, *setting);
                }
                (Ok(()), RemoteCommand::Named(_)) => {}
                (Err(e), _) => {
                    warn!("background thread: {cmd} failed: {e}");
                    let _ = msg_tx.send(DmmMessage::CommandFailed(format!("{cmd} failed: {e}")));
                    ctx.request_repaint();
                }
            }
        }

        // Pause halts acquisition itself, rather than letting the UI discard
        // measurements it asked for: the meter is not polled at all. Without
        // this the meter kept being read while "paused", so unplugging it then
        // dropped the GUI into its reconnect loop.
        if paused {
            continue;
        }

        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                protocol_errors = 0;
                send_stale_choices(stream.dmm(), &m, &mut last_keys, &msg_tx);
                if msg_tx.send(DmmMessage::Measurement(m)).is_err() {
                    break;
                }
            }
            Ok(StreamEvent::Timeout { consecutive }) => {
                warn!("background thread: measurement timeout ({consecutive})");
                let _ = msg_tx.send(DmmMessage::WaitingForMeter(consecutive));
                ctx.request_repaint();
                if consecutive == NO_RESPONSE_TIMEOUTS {
                    let _ = msg_tx.send(DmmMessage::NoResponse);
                    ctx.request_repaint();
                }
            }
            Err(e) if e.kind() == ErrorKind::Protocol => {
                // A frame we couldn't parse is not a dead link. Either line
                // noise corrupted a checksum, or the meter is in a dial
                // position this family's tables don't cover — and reconnecting
                // fixes neither. It costs a 2 s stall plus an audible
                // identification beep, and for an unknown mode byte it flaps
                // forever: connect, read, fail, repeat.
                //
                // Report it and keep reading. The next good frame clears the
                // message (`DmmMessage::Measurement` resets `last_error`), so
                // one corrupt frame is a blip rather than a hole in the graph.
                protocol_errors = protocol_errors.saturating_add(1);
                warn!("background thread: protocol error ({protocol_errors}): {e}");
                if protocol_errors == 1
                    || protocol_errors.is_multiple_of(PROTOCOL_ERROR_REPORT_INTERVAL)
                {
                    let _ = msg_tx.send(DmmMessage::Error(e));
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                error!("background thread: device error: {e}");
                let _ = msg_tx.send(DmmMessage::Disconnected(e));
                ctx.request_repaint();

                // Reconnection loop. Waits on the stop channel so disconnects
                // propagate within the retry interval instead of up to 2s later,
                // and reports each attempt to the UI so the user sees progress.
                //
                // End the stream's borrow on `dmm` before reassigning; we
                // rebuild the stream after reconnect so tick scheduling
                // restarts fresh from the post-reconnect instant.
                // (`drop()` would be clearer but clippy warns because the
                //  stream itself has no Drop impl — the borrow-release we
                //  actually need is what reassignment accomplishes here.)
                let _ = stream;
                // Release the old link before opening a new one: a Bluetooth
                // transport disconnects its peripheral on drop, and dropped
                // after the reopen that would cut the link just brought back
                // up on the same adapter.
                drop(dmm);
                let retry_interval = Duration::from_secs(2);
                let mut attempt: u32 = 0;
                let mut last_error: Option<String> = None;
                loop {
                    attempt += 1;
                    let _ = msg_tx.send(DmmMessage::Reconnecting {
                        attempt,
                        last_error: last_error.clone(),
                    });
                    ctx.request_repaint();

                    // Sleep, but wake early on a control message. A pause that
                    // arrives mid-reconnect is recorded and takes effect once
                    // the link is back: there is nothing to halt until then.
                    match ctrl_rx.recv_timeout(retry_interval) {
                        Ok(ThreadControl::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                        Ok(ThreadControl::SetPaused(p)) => paused = p,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }

                    // Re-runs the opener whole, detection included: under
                    // Auto the meter that comes back is identified again
                    // rather than assumed to be the one that went away. A
                    // Bluetooth session goes back to its own adapter, by
                    // address: a new one is the user's Disconnect/Connect.
                    // Every failure here is retried, whatever its kind — an
                    // adapter that does not answer yet is the case this loop
                    // waits out, so it only updates the Reconnecting notice.
                    match open_fn(reopen_at.as_deref()) {
                        Ok((mut d, detected)) => {
                            info!("background thread: reconnected on attempt {attempt}");
                            reopen_at = bluetooth_selector(&d);
                            establish_connection(
                                &mut d,
                                detected,
                                selected,
                                query_name,
                                recorded_link,
                                &msg_tx,
                                &ctx,
                            );
                            dmm = d;
                            break;
                        }
                        Err(err) => {
                            warn!("background thread: reconnect attempt {attempt} failed: {err}");
                            last_error = Some(err.to_string());
                        }
                    }
                }
                stream = MeasurementStream::new(&mut dmm, tick);
                protocol_errors = 0;
                // The dial may have moved while the link was down.
                last_keys = Default::default();
            }
        }

        ctx.request_repaint();
    }
}

/// The address that reopens this session's Bluetooth adapter, `None` on
/// any other link.
fn bluetooth_selector<T: Transport>(dmm: &dmm_lib::Dmm<T>) -> Option<String> {
    dmm.transport().bluetooth_selector().map(str::to_owned)
}

pub(super) fn handle_thread_panic(
    panic: Box<dyn std::any::Any + Send>,
    tx: &mpsc::Sender<DmmMessage>,
    ctx: &egui::Context,
) {
    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    };
    error!("background thread panicked: {msg}");
    let _ = tx.send(DmmMessage::ErrorText(format!("internal error: {msg}")));
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::MeasuredValue;

    #[test]
    fn no_messages_keeps_running() {
        let (_tx, rx) = mpsc::channel::<ThreadControl>();
        let mut paused = false;
        assert!(handle_control(&rx, &mut paused));
        assert!(!paused);
    }

    #[test]
    fn stop_ends_the_loop() {
        let (tx, rx) = mpsc::channel();
        tx.send(ThreadControl::Stop).unwrap();
        let mut paused = false;
        assert!(!handle_control(&rx, &mut paused));
    }

    /// The UI dropping its sender without sending Stop (a panic on the UI
    /// thread, or a reconnect that replaced the channel) has to end the
    /// thread too — otherwise it keeps the USB handle and polls forever.
    #[test]
    fn hung_up_channel_ends_the_loop() {
        let (tx, rx) = mpsc::channel::<ThreadControl>();
        drop(tx);
        let mut paused = false;
        assert!(!handle_control(&rx, &mut paused));
    }

    #[test]
    fn pause_is_recorded_and_resume_returns_immediately() {
        let (tx, rx) = mpsc::channel();
        let mut paused = false;

        tx.send(ThreadControl::SetPaused(true)).unwrap();
        // Queue the resume too, so the paused branch has a message waiting
        // and the test doesn't sit through the poll interval.
        tx.send(ThreadControl::SetPaused(false)).unwrap();
        assert!(handle_control(&rx, &mut paused));
        assert!(!paused, "resume must clear the pause");
    }

    /// While paused the thread waits rather than spinning, but it still has
    /// to notice a Stop.
    #[test]
    fn stop_is_honoured_while_paused() {
        let (tx, rx) = mpsc::channel();
        tx.send(ThreadControl::Stop).unwrap();
        let mut paused = true;
        assert!(!handle_control(&rx, &mut paused));
    }

    #[test]
    fn hung_up_channel_ends_the_loop_while_paused() {
        let (tx, rx) = mpsc::channel::<ThreadControl>();
        drop(tx);
        let mut paused = true;
        assert!(!handle_control(&rx, &mut paused));
    }

    fn reading(mode_raw: u16, mode: &'static str) -> Measurement {
        Measurement {
            mode: mode.into(),
            mode_raw,
            ..Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default())
        }
    }

    /// A reading on a given rung, auto-ranging or not.
    fn ranged(range_raw: u8, auto_range: bool) -> Measurement {
        Measurement {
            range_raw,
            ..Measurement::test_fixture(
                MeasuredValue::Normal(1.0),
                "V",
                StatusFlags {
                    auto_range,
                    ..StatusFlags::default()
                },
            )
        }
    }

    /// The re-list has to survive being run per sample: an unchanged mode must
    /// not keep rebuilding and resending the list.
    #[test]
    fn mode_choices_are_listed_once_per_mode() {
        let m = reading(0x0A, "Temperature");
        assert!(
            choices_stale(None, Setting::Mode, &m),
            "first reading must list"
        );
        let key = ChoiceKey::of(Setting::Mode, &m);
        assert!(!choices_stale(Some(&key), Setting::Mode, &m));
    }

    /// The mock cycles Temp / Temp dual / Temp diff under one mode word, and
    /// the live entry moves with each. Keying on the word alone left the
    /// dropdown marking a mode the meter had already left.
    #[test]
    fn mode_choices_are_relisted_when_only_the_mode_text_changes() {
        let key = ChoiceKey::Mode(0x0A, Cow::Borrowed("Temperature"));
        assert!(choices_stale(
            Some(&key),
            Setting::Mode,
            &reading(0x0A, "Temperature (dual)")
        ));
    }

    #[test]
    fn mode_choices_are_relisted_when_the_mode_word_changes() {
        let key = ChoiceKey::Mode(0x0A, Cow::Borrowed("Temperature"));
        assert!(choices_stale(
            Some(&key),
            Setting::Mode,
            &reading(0x00, "Temperature")
        ));
    }

    /// Stepping the meter to another rung moves the mark in the range list,
    /// so the list has to be re-sent — while an unchanged rung must not
    /// re-send it once per sample.
    #[test]
    fn range_choices_are_relisted_when_the_rung_changes() {
        let m = ranged(2, false);
        let key = ChoiceKey::of(Setting::Range, &m);
        assert!(!choices_stale(Some(&key), Setting::Range, &m));
        assert!(choices_stale(Some(&key), Setting::Range, &ranged(3, false)));
    }

    /// Leaving auto-range moves the mark from `Auto` to the rung the meter
    /// was already on, so the rung alone cannot key the list.
    #[test]
    fn range_choices_are_relisted_when_auto_range_changes() {
        let key = ChoiceKey::of(Setting::Range, &ranged(2, true));
        assert!(choices_stale(Some(&key), Setting::Range, &ranged(2, false)));
    }

    /// A mode change re-lists the ranges too: the ladder belongs to the mode.
    #[test]
    fn range_choices_are_relisted_when_the_mode_changes() {
        let key = ChoiceKey::of(Setting::Range, &ranged(2, true));
        let mut moved = ranged(2, true);
        moved.mode_raw = 0x0B;
        assert!(choices_stale(Some(&key), Setting::Range, &moved));
    }

    /// Collect the settings a batch of messages listed, with the id of the
    /// entry each list marked live.
    fn listed(rx: &mpsc::Receiver<DmmMessage>) -> Vec<(Setting, Option<u16>)> {
        rx.try_iter()
            .filter_map(|msg| match msg {
                DmmMessage::Choices(setting, choices) => {
                    Some((setting, choices.iter().find(|c| c.current).map(|c| c.id)))
                }
                _ => None,
            })
            .collect()
    }

    /// The first reading lists both settings; an unchanged one lists nothing;
    /// a rung change lists the ranges again, with the new rung marked.
    #[test]
    fn a_rung_change_sends_a_fresh_range_list() {
        let mut dmm = dmm_lib::mock::open_mock_mode(dmm_lib::mock::MockMode::DcV).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut keys: ListedKeys = Default::default();

        let m = dmm.request_measurement().unwrap();
        send_stale_choices(&dmm, &m, &mut keys, &tx);
        // The mock's DC V position is in no mode group, so its mode list is
        // empty and marks nothing; its range list is the ladder, on `Auto`.
        assert_eq!(
            listed(&rx),
            vec![(Setting::Mode, None), (Setting::Range, Some(0))],
            "the first reading lists both settings"
        );

        send_stale_choices(&dmm, &m, &mut keys, &tx);
        assert!(listed(&rx).is_empty(), "an unchanged reading lists nothing");

        dmm.select(Setting::Range, 2).unwrap();
        let m = dmm.request_measurement().unwrap();
        send_stale_choices(&dmm, &m, &mut keys, &tx);
        assert_eq!(listed(&rx), vec![(Setting::Range, Some(2))]);
    }

    /// A successful switch re-lists that setting alone, even where the
    /// reading it is keyed on has not moved — which entry is live has.
    #[test]
    fn a_successful_select_forces_that_settings_re_list() {
        let mut dmm = dmm_lib::mock::open_mock_mode(dmm_lib::mock::MockMode::DcV).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut keys: ListedKeys = Default::default();
        let m = dmm.request_measurement().unwrap();
        send_stale_choices(&dmm, &m, &mut keys, &tx);
        let _ = listed(&rx);

        invalidate(&mut keys, Setting::Range);
        send_stale_choices(&dmm, &m, &mut keys, &tx);
        assert_eq!(listed(&rx), vec![(Setting::Range, Some(0))]);
    }

    /// A setting the thread does not list must not disturb the keys of the
    /// ones it does.
    #[test]
    fn invalidating_an_unlisted_setting_changes_nothing() {
        let m = ranged(2, true);
        let mut keys: ListedKeys = [
            Some(ChoiceKey::of(Setting::Mode, &m)),
            Some(ChoiceKey::of(Setting::Range, &m)),
        ];
        invalidate(&mut keys, Setting::Hold);
        assert!(keys.iter().all(Option::is_some));
    }

    /// A paused thread with nothing to do returns to the caller so queued
    /// device commands still get sent, and stays paused.
    #[test]
    fn paused_thread_wakes_periodically_and_stays_paused() {
        let (_tx, rx) = mpsc::channel::<ThreadControl>();
        let mut paused = true;
        let start = std::time::Instant::now();
        assert!(handle_control(&rx, &mut paused));
        assert!(paused);
        assert!(
            start.elapsed() >= PAUSE_POLL_INTERVAL,
            "must wait rather than spin"
        );
    }

    /// A link that drops on the first read, counting how many are open. With
    /// a selector it is a Bluetooth link on that adapter.
    struct Dropout(Arc<std::sync::atomic::AtomicUsize>, Option<&'static str>);

    impl Dropout {
        fn open(live: &Arc<std::sync::atomic::AtomicUsize>) -> Self {
            live.fetch_add(1, Ordering::SeqCst);
            Self(Arc::clone(live), None)
        }
    }

    impl Drop for Dropout {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl Transport for Dropout {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }
        fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            Err(dmm_lib::error::Error::LinkLost)
        }
        fn send_feature_report(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            Ok(())
        }
        fn bluetooth_selector(&self) -> Option<&str> {
            self.1
        }
    }

    fn thread_context(
        msg_tx: mpsc::Sender<DmmMessage>,
        ctrl_rx: mpsc::Receiver<ThreadControl>,
        cmd_rx: mpsc::Receiver<RemoteCommand>,
    ) -> ThreadContext {
        ThreadContext {
            msg_tx,
            ctrl_rx,
            cmd_rx,
            ctx: egui::Context::default(),
            selected: None,
            query_name: false,
            recorded_link: None,
            sample_interval_ms: 10,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// A Bluetooth transport disconnects its peripheral when dropped, so a
    /// reconnect that opened the new link before dropping the old one cut
    /// the link it had just brought up. The old one has to be gone by the
    /// time the opener runs again.
    #[test]
    fn a_reconnect_releases_the_old_link_before_opening_a_new_one() {
        let live = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (seen_tx, seen_rx) = mpsc::channel();
        let (msg_tx, _msg_rx) = mpsc::channel();
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (_cmd_tx, cmd_rx) = mpsc::channel();
        let opens = std::sync::atomic::AtomicUsize::new(0);
        let entry = dmm_lib::protocol::registry::find_device("ut61eplus").expect("registry");
        let open_live = Arc::clone(&live);
        let open_fn = move |_: Option<&str>| {
            if opens.fetch_add(1, Ordering::SeqCst) > 0 {
                let _ = seen_tx.send(open_live.load(Ordering::SeqCst));
                return Err(dmm_lib::error::Error::Timeout);
            }
            dmm_lib::Dmm::new(Dropout::open(&open_live), (entry.new_protocol)()).map(|d| (d, None))
        };
        let thread = std::thread::spawn(move || {
            run_device_thread(open_fn, thread_context(msg_tx, ctrl_rx, cmd_rx))
        });
        let open_at_reopen = seen_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the thread reconnects");
        ctrl_tx.send(ThreadControl::Stop).unwrap();
        thread.join().unwrap();
        assert_eq!(open_at_reopen, 0, "the old link was still open");
    }

    /// A lost Bluetooth link is reopened at its own adapter's address, with
    /// no scan for others. That open fails with a Bluetooth error while the
    /// adapter is off — a Configuration kind, which must still be retried
    /// and shown as reconnecting rather than end the session in an error.
    #[test]
    fn a_lost_bluetooth_link_is_retried_at_its_address() {
        const ADDRESS: &str = "12:34:56:78:9A:BC";
        let live = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (seen_tx, seen_rx) = mpsc::channel();
        let (msg_tx, msg_rx) = mpsc::channel();
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (_cmd_tx, cmd_rx) = mpsc::channel();
        let opens = std::sync::atomic::AtomicUsize::new(0);
        let entry = dmm_lib::protocol::registry::find_device("ut61eplus").expect("registry");
        let timeout = || dmm_lib::error::Error::Bluetooth("connection timed out".to_string());
        assert_eq!(timeout().kind(), ErrorKind::Configuration);
        let open_fn = move |reopen_at: Option<&str>| {
            if opens.fetch_add(1, Ordering::SeqCst) > 0 {
                let _ = seen_tx.send(reopen_at.map(str::to_owned));
                return Err(timeout());
            }
            let link = Dropout(Arc::clone(&live), Some(ADDRESS));
            dmm_lib::Dmm::new(link, (entry.new_protocol)()).map(|d| (d, None))
        };
        let thread = std::thread::spawn(move || {
            run_device_thread(open_fn, thread_context(msg_tx, ctrl_rx, cmd_rx))
        });
        let reopens: Vec<Option<String>> = (0..2)
            .map(|_| {
                seen_rx
                    .recv_timeout(Duration::from_secs(10))
                    .expect("the thread keeps reconnecting")
            })
            .collect();
        ctrl_tx.send(ThreadControl::Stop).unwrap();
        thread.join().unwrap();
        assert_eq!(
            reopens,
            [Some(ADDRESS.to_string()), Some(ADDRESS.to_string())]
        );
        let messages: Vec<DmmMessage> = msg_rx.try_iter().collect();
        assert!(
            !messages.iter().any(|m| matches!(m, DmmMessage::Error(_))),
            "a failed reconnect ended in an error notice"
        );
        assert!(messages.iter().any(|m| matches!(
            m,
            DmmMessage::Reconnecting {
                attempt: 2,
                last_error: Some(_)
            }
        )));
    }
}
