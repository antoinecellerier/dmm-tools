//! The UI side of the acquisition channel: opening and closing it, draining
//! the messages the device thread sends, and classifying the reason there is
//! nothing to show into the help text the reading column renders.

use dmm_lib::binary_help::{ConnectedAdapters, connected_adapters};
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use eframe::egui::{self, RichText, Ui};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use super::connection::{
    self, DmmMessage, RemoteCommand, ThreadContext, ThreadControl, handle_thread_panic,
    run_device_thread,
};
use super::plot_input::{PlotInput, resolve_plot_input};
use super::{App, ConnectionState, named_device};
use crate::graph::PlotSample;
use crate::settings::format_sample_count;

/// Why the GUI currently has no readings to show.
///
/// Replaces a `String` that carried the sentinel `"__device_not_found__"`
/// and was probed with `contains("adapter not found")` at the render site —
/// CLAUDE.md: "Prefer enums over string-typed status/state values." The
/// acquisition thread already distinguishes these cases; this stops the
/// distinction being flattened into text and parsed back out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectionIssue {
    /// No supported USB adapter on the bus.
    DeviceNotFound,
    /// `--adapter` was given but nothing matched it. Carries the finished
    /// help text, including the connected-device list, because building it
    /// enumerates the USB bus — far too heavy for the paint path.
    AdapterNotFound { help: String },
    /// The cable is there but nothing on it answered the detection probe.
    /// Carries the finished help — what to switch on, per meter that could
    /// have been on that bridge — because it is built from the bridge the
    /// error names and the render path no longer has it.
    NotIdentified { help: String },
    /// Anything else, as reported by the acquisition thread.
    Other(String),
}

/// Which connection notice is on screen.
///
/// The big meter keys its fit cache on this: the titles differ in length, so
/// the fitted font has to be re-measured when one notice replaces another.
/// Not derived from the text, which the waiting notice changes every timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NoticeKind {
    Detecting,
    Waiting,
    DeviceNotFound,
    AdapterNotFound,
    NotIdentified,
    NoResponse,
}

/// What the reading column has to say about why there is nothing to show,
/// in pieces, so a layout can render as much of it as it has room for.
pub(super) struct ConnectionNotice {
    pub(super) kind: NoticeKind,
    /// One line naming the problem.
    pub(super) title: String,
    /// The steps that resolve it. Several hard-wrapped lines; empty while
    /// detection is still running, where there is nothing to do but wait.
    pub(super) body: String,
    /// The experimental-support feedback link, as `(text, url)`.
    pub(super) experimental_link: Option<(String, String)>,
}

impl ConnectionIssue {
    /// Classify an error the acquisition thread hands over whole.
    ///
    /// `ErrorKind` decides the not-found case; the adapter case matches its
    /// variant instead, because the help text needs the selector the user
    /// typed and the coarse kind cannot carry it. Classified once when the
    /// error arrives, not at the render site on every repaint.
    fn from_error(err: &dmm_lib::error::Error) -> Self {
        if let dmm_lib::error::Error::AdapterNotFound(selector) = err {
            return Self::AdapterNotFound {
                help: adapter_not_found_help(selector),
            };
        }
        // Also a `Timeout` kind — retrying does help once transmission is on —
        // so it is matched by variant, ahead of the kind test below.
        if let dmm_lib::error::Error::DeviceNotIdentified { bridge } = err {
            return Self::NotIdentified {
                help: not_identified_help(bridge),
            };
        }
        match err.kind() {
            dmm_lib::error::ErrorKind::DeviceNotFound => Self::DeviceNotFound,
            _ => Self::Other(err.to_string()),
        }
    }
}

/// Build the "adapter not found" help, listing what is actually on the bus.
///
/// Called once when the error arrives, not from the render path:
/// `connected_adapters()` constructs a fresh `HidApi` and walks every HID device
/// on the system, and this help stays on screen across every repaint until
/// the user reconnects. The list is a snapshot either way — the user has to
/// restart with a different `--adapter` to act on it.
fn adapter_not_found_help(selector: &str) -> String {
    let adapters = connected_adapters();
    let mut msg = format!("No device matched --adapter '{selector}'.");
    msg.push_str("\n\n");
    msg.push_str(&adapters.lines().join("\n"));
    // An empty bus is a complete answer on its own; the other two leave the
    // user with a choice to make.
    if !matches!(adapters, ConnectedAdapters::None) {
        msg.push_str("\n\nRestart with the correct --adapter value.");
    }
    msg
}

/// Build the "nothing answered" help: what the user can switch on, for every
/// meter that could have been behind that bridge.
///
/// Families share activation steps — the whole UT61+ line is one instruction —
/// so the meters are grouped by the instruction text rather than listed one by
/// one, which would repeat the same four steps six times over.
fn not_identified_help(bridge: &str) -> String {
    let mut msg = String::from(
        "The USB adapter is connected but no meter identified itself.\n\n\
         Switch on the meter's USB mode, or pick the model in Settings (\u{2699}):\n",
    );
    let mut groups: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for device in dmm_lib::devices_on_bridge(bridge) {
        match groups
            .iter_mut()
            .find(|(steps, _)| *steps == device.activation_instructions)
        {
            Some((_, names)) => names.push(device.display_name),
            None => groups.push((device.activation_instructions, vec![device.display_name])),
        }
    }
    for (steps, names) in groups {
        msg.push('\n');
        msg.push_str(&names.join(", "));
        msg.push('\n');
        msg.push_str(steps);
        msg.push('\n');
    }
    msg.push_str(
        "\nAlready transmitting and still not recognised? Pick its model in Settings \
         (\u{2699}) and please report it.\n",
    );
    msg
}

/// The meter a `Connected` identified for a session that was told none, or
/// `None` when there is nothing to tell the user.
///
/// `None` covers the two quiet cases: the settings name a meter, so the id
/// that came back is the user's own pick returning (which is also every
/// reconnect once a detected meter has been saved), and the mock, which is
/// never on the far end of a cable.
fn detected_under_auto(
    family: &str,
    device_id: Option<&'static str>,
) -> Option<&'static registry::SelectableDevice> {
    if named_device(family).is_some() {
        return None;
    }
    device_id
        .and_then(registry::find_device)
        .filter(|d| d.requires_hardware)
}

/// The id to write into `device_family` once a meter has identified itself,
/// or `None` when this connection leaves the settings file alone.
///
/// Saving is the whole point: a session that opens a named model skips the
/// detection cascade, so the `0x5F` a UT61+ beeps at is sent only if
/// `query_device_name` asks for the name — while detecting it goes out either
/// way. Two cases deliberately write nothing.
/// `--device auto` is a session-only override, and session-only
/// values never reach the file ([`crate::settings::Settings::save`] writes the
/// original back under one). And a value that already names that meter is left
/// alone, so a drop and reconnect doesn't rewrite the file each time.
fn detected_device_to_save(
    family: &str,
    overridden: bool,
    device_id: Option<&'static str>,
) -> Option<&'static str> {
    if overridden {
        return None;
    }
    detected_under_auto(family, device_id)
        .map(|d| d.id)
        .filter(|id| *id != family)
}

/// What the toast says when a meter identifies itself under Auto-detect.
///
/// `saved` splits the settings selection, which now names the meter that
/// answered, from a `--device auto` run, which detects for this session only.
fn detected_toast(display_name: &str, reported: &str, saved: bool) -> String {
    let mut msg = format!("Detected {display_name}");
    // The reported name earns its place only when it is not the name already
    // on screen: a UT61E+ answers "UT61E+". A model no entry claims falls back
    // to the UT61E+ tables, and then the name is the whole story — it is what
    // the user has to quote to get an alias added.
    if !reported.is_empty() && !reported.eq_ignore_ascii_case(display_name) {
        msg.push_str(&format!(" (the meter reports \"{reported}\")"));
    }
    if saved {
        msg.push_str(", saved as your device. Pick Auto-detect in Settings to probe again.");
    }
    msg
}

impl App {
    /// Save the meter that just identified itself as the device, and say so.
    ///
    /// Only a session that was detecting gets here: under a named model the
    /// `Connected` id is the pick the user already made. What is saved is what
    /// the *next* session opens — this one is already running that protocol,
    /// so nothing reconnects and no reading is lost.
    fn remember_detected_device(&mut self, device_id: Option<&'static str>, reported: &str) {
        let Some(device) = detected_under_auto(&self.settings.shared.device_family, device_id)
        else {
            return;
        };
        let to_save = detected_device_to_save(
            &self.settings.shared.device_family,
            self.settings.overrides.has_device(),
            device_id,
        );
        if let Some(id) = to_save {
            info!("UI: saving detected device {id} as the selected device");
            self.settings.shared.device_family = id.to_string();
            self.settings.save();
        }
        self.toast = Some((
            detected_toast(device.display_name, reported, to_save.is_some()),
            false,
            Instant::now(),
        ));
    }

    /// Say that lowering Buffer size ended the capture that was running.
    ///
    /// Deliberately not the full-buffer wording: that names the new bound,
    /// which here is *smaller* than what the buffer holds — a capture keeps
    /// every sample it took, so quoting the bound would read as if the
    /// difference had been thrown away.
    pub(super) fn buffer_shrunk_toast(&mut self) {
        let kept = format_sample_count(self.recording.samples.len());
        self.toast = Some((
            format!(
                "Recording stopped \u{2014} its {kept} samples are kept, Export CSV saves them"
            ),
            true,
            Instant::now(),
        ));
    }

    /// Say that the recording stopped because it filled the buffer, at the
    /// size the user configured.
    pub(super) fn buffer_full_toast(&mut self) {
        self.toast = Some((
            format!(
                "Recording stopped \u{2014} buffer full ({} samples)",
                format_sample_count(self.settings.max_samples)
            ),
            true,
            Instant::now(),
        ));
    }

    pub(super) fn connect(&mut self, ctx: &egui::Context) {
        self.disconnect();

        let (msg_tx, msg_rx) = mpsc::channel();
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<RemoteCommand>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.connection.rx = Some(msg_rx);
        self.connection.ctrl_tx = Some(ctrl_tx);
        self.connection.stop_flag = Some(Arc::clone(&stop_flag));
        self.connection.cmd_tx = Some(cmd_tx);
        let ctx_clone = ctx.clone();
        let query_name = self.settings.query_device_name;
        let sample_interval_ms = self.settings.sample_interval_ms;
        // `None` = Auto-detect: nothing names the meter, so the opener works
        // it out from the bytes it sends.
        let device_entry = self.selected_device();
        self.graph.set_sample_interval_ms(sample_interval_ms);

        if device_entry.is_some_and(|d| !d.requires_hardware) {
            let mock_mode: Option<MockMode> = if self.settings.mock_mode.is_empty() {
                None
            } else {
                match self.settings.mock_mode.parse() {
                    Ok(mode) => Some(mode),
                    // Only a hand-edited settings file reaches this: clap
                    // rejects a bad `--mock-mode` and the Settings row writes
                    // labels. Auto-cycling silently looked like the pin was
                    // ignored, so say so — the toast takes the first line and
                    // the log the mode list that follows it.
                    Err(message) => {
                        warn!("{message}");
                        let headline = message.lines().next().unwrap_or_default().to_string();
                        self.toast = Some((headline, true, Instant::now()));
                        None
                    }
                }
            };
            // Mock returns instantly — enforce a floor to avoid busy-looping.
            // This is session time now, which is what lets a preseed burst
            // hand out tick-spaced history without waiting for it.
            let mock_interval = sample_interval_ms.max(100);
            let clock = self.clock.clone();
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        // Cloned inside: this closure is re-run on every
                        // reconnect, and the session clock outlives each
                        // `Dmm` it opens. Nothing to detect — the mock is
                        // what it says it is.
                        move || {
                            dmm_lib::mock::open_mock_clocked(mock_mode, clock.clone())
                                .map(|dmm| (dmm, None))
                        },
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
                            selected: device_entry,
                            query_name,
                            sample_interval_ms: mock_interval,
                            stop_flag,
                        },
                    );
                }));
                if let Err(panic) = result {
                    handle_thread_panic(panic, &panic_tx, &panic_ctx);
                }
            });
        } else {
            let device_id = device_entry.map(|d| d.id);
            let adapter = self.settings.overrides.adapter.clone();
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        // Re-run on every reconnect, detection included: a
                        // meter that comes back is identified again rather
                        // than assumed to be the one that left.
                        move || match device_id {
                            Some(id) => dmm_lib::open_device_by_id_auto(id, adapter.as_deref())
                                .map(|dmm| (dmm, None)),
                            None => dmm_lib::open_auto(adapter.as_deref())
                                .map(|(dmm, detected)| (dmm, Some(detected))),
                        },
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
                            selected: device_entry,
                            query_name,
                            sample_interval_ms,
                            stop_flag,
                        },
                    );
                }));
                if let Err(panic) = result {
                    handle_thread_panic(panic, &panic_tx, &panic_ctx);
                }
            });
        }
    }

    pub(super) fn disconnect(&mut self) {
        // Data stops here. The graph keeps its history across a reconnect, so
        // the resulting hole needs marking as a genuine gap.
        self.graph.push_data_loss();
        // Raise the flag before the message: the thread may be mid-sleep, and
        // the flag is what cuts that short.
        if let Some(flag) = self.connection.stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(tx) = self.connection.ctrl_tx.take() {
            let _ = tx.send(ThreadControl::Stop);
        }
        self.connection.rx = None;
        self.connection.cmd_tx = None;
        self.connection.state = ConnectionState::Disconnected;
        self.connection.device_name = None;
        // Nothing is connected, so nothing is identified: under Auto-detect
        // the next connect asks the cable again.
        self.connection.detected = None;
        self.connection.model_name.clear();
        self.connection.stability = dmm_lib::protocol::Stability::Verified;
        self.connection.feedback_url.clear();
        self.connection.supported_commands.clear();
        self.connection.choices.clear();
        self.connection.paused = false;
        self.connection.reconnect_attempt = 0;
        self.connection.reconnect_last_error = None;
        // Otherwise only an incoming measurement clears this, and there won't
        // be one — a meter that went quiet before the user disconnected left
        // "Waiting for meter…" on screen for the whole disconnected session.
        self.connection.waiting_timeouts = 0;
    }

    /// Drop everything derived from the sample stream: graph history,
    /// session statistics, the integrator and the last reading.
    ///
    /// Shared by `Ctrl+L`, the Clear button and a change of software scale,
    /// which all mean the same thing — the numbers accumulated so far no
    /// longer describe what is being measured. The recording buffer is
    /// deliberately not touched; Clear has never discarded a capture.
    pub(super) fn clear_session(&mut self) {
        self.graph.clear();
        self.session.reset();
        self.last_measurement = None;
    }

    pub(super) fn drain_messages(&mut self) {
        // Drain with `try_recv` rather than `try_iter` so a hung-up sender is
        // distinguishable from an empty queue: the acquisition thread dropping
        // its sender is the only signal that it has died.
        let mut messages: Vec<DmmMessage> = Vec::new();
        let mut thread_gone = false;
        if let Some(rx) = self.connection.rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(msg) => messages.push(msg),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        thread_gone = true;
                        break;
                    }
                }
            }
        }

        let mut clear_channel = false;

        for msg in messages {
            match msg {
                DmmMessage::Connected {
                    name,
                    model_name,
                    device_id,
                    stability,
                    feedback_url,
                    supported_commands: cmds,
                    max_aux_values,
                } => {
                    self.connection.state = ConnectionState::Connected;
                    // Which meter this actually is. Under Auto-detect it is
                    // the only thing that knows — nothing named one.
                    self.connection.detected =
                        device_id.and_then(dmm_lib::protocol::registry::find_device);
                    self.connection.model_name = model_name;
                    self.connection.stability = stability;
                    self.capture_layout.device_aux_slots = max_aux_values;
                    // A reconnect mid-recording is the same meter, so the
                    // in-flight capture picks the slot count back up — it was
                    // 0 before the first Connected of the session.
                    if self.recording.active {
                        self.capture_layout.aux_slots = max_aux_values;
                    }
                    self.connection.feedback_url = feedback_url;
                    self.connection.supported_commands = cmds;
                    // A reconnect may find the dial elsewhere; the thread
                    // re-lists the choices with its first reading.
                    self.connection.choices.clear();
                    self.connection.device_name = if name.is_empty() {
                        None
                    } else {
                        Some(name.clone())
                    };
                    // Under Auto-detect the meter that just named itself
                    // becomes the saved device, so the next session opens it
                    // pinned instead of probing the cable again.
                    self.remember_detected_device(device_id, &name);
                    self.connection.last_error = None;
                    self.connection.reconnect_attempt = 0;
                    self.connection.reconnect_last_error = None;
                    info!(
                        "UI: connected to {} (meter reports {:?})",
                        self.connection.model_name, self.connection.device_name
                    );
                }
                DmmMessage::WaitingForMeter(count) => {
                    self.connection.waiting_timeouts = count;
                    // A timeout never raises Disconnected — the bridge is
                    // still enumerated, the meter just isn't answering (auto
                    // power-off, or unplugged at the meter end). Without this
                    // an outage during an overload would be drawn as one long
                    // band, claiming over-range for a stretch nothing was
                    // heard in. Same threshold the "no response" notice uses.
                    if count >= connection::NO_RESPONSE_TIMEOUTS {
                        self.graph.push_data_loss();
                    }
                }
                DmmMessage::Reconnecting {
                    attempt,
                    last_error,
                } => {
                    self.connection.reconnect_attempt = attempt;
                    self.connection.reconnect_last_error = last_error;
                }
                DmmMessage::Measurement(m) => {
                    self.connection.last_error = None;
                    self.connection.waiting_timeouts = 0;
                    if self.connection.paused {
                        continue;
                    }

                    // The single point a software transform is applied. Every
                    // consumer below — the session statistics, the graph's
                    // series list and plot input, the recording buffer and
                    // `last_measurement` — then sees one already-scaled
                    // reading, and none of them has to know transforms exist.
                    // No-op when identity.
                    let mut m = m;
                    self.transform.apply(&mut m);

                    // Session stats follow the meter's *main* reading whatever
                    // the graph plots: they describe the reading, not the view.
                    // `SeriesStats` resets them on a mode *or* unit change —
                    // a dial turn, or an auto-range step that moves the unit a
                    // decade (mV→V) without touching the mode — so the panel
                    // never labels volt-scale numbers with an ohms unit.
                    // `Graph::push_sample` clears its history on the same
                    // condition, and the GUI resets silently, so the returned
                    // `SeriesChange` is not needed here.
                    self.session.push(&m);

                    // Offer this frame's sub-values before resolving what to
                    // plot, so that the frame the graph finally gives the
                    // selection up on is also the one that plots the main
                    // reading again, not the one after it. Until then a frame
                    // missing the selected sub-value resolves to nothing and
                    // is skipped, leaving the trace intact.
                    let options: Vec<(&str, &str)> = m
                        .aux_values
                        .iter()
                        .map(|aux| (aux.label.as_ref(), aux.unit_or(&m.unit)))
                        .collect();
                    self.graph.set_series_options(&options);
                    match resolve_plot_input(&m, self.graph.selected_series()) {
                        Some(PlotInput {
                            value: Some(v),
                            unit,
                            display_raw,
                            series,
                            overlays,
                        }) => self.graph.push_sample(PlotSample {
                            value: v,
                            timestamp: m.timestamp,
                            mode: &m.mode,
                            unit,
                            display_raw,
                            series,
                            overlays: &overlays,
                        }),
                        // The plotted series is over range: no point, but the
                        // trace has to break so it isn't drawn straight
                        // through the excursion.
                        Some(PlotInput { value: None, .. }) => self.graph.push_break(m.timestamp),
                        None => {}
                    }

                    // `m` has already been through the transform, so the
                    // count it appended is what this sample carries.
                    let extra_aux = self.transform.extra_aux_count();
                    if self.recording.push(&m, &self.wall_clock, extra_aux) {
                        self.buffer_full_toast();
                    }

                    // Specs are attached to each measurement by `Dmm::request_measurement`;
                    // last_measurement.spec / .mode_spec is what render code reads.
                    self.last_measurement = Some(m);
                }
                DmmMessage::Disconnected(err) => {
                    info!("UI: disconnected: {err} ({:?})", err.kind());
                    self.connection.state = ConnectionState::Reconnecting;
                    // Tell the graph this was a real loss of data. It can't
                    // infer that from timestamps — the meter goes quiet for
                    // over a second while auto-ranging, which looks the same
                    // as an unplugged cable.
                    self.graph.push_data_loss();
                }
                DmmMessage::Error(e) => {
                    error!("UI: error: {e}");
                    self.connection.last_error = Some(ConnectionIssue::from_error(&e));
                    if self.connection.state == ConnectionState::Disconnected {
                        clear_channel = true;
                    }
                }
                DmmMessage::ErrorText(msg) => {
                    error!("UI: error: {msg}");
                    self.connection.last_error = Some(ConnectionIssue::Other(msg));
                    if self.connection.state == ConnectionState::Disconnected {
                        clear_channel = true;
                    }
                }
                DmmMessage::CommandFailed(msg) => {
                    self.toast = Some((msg, true, Instant::now()));
                }
                DmmMessage::Choices(setting, choices) => {
                    self.connection.choices.set(setting, choices);
                }
            }
        }

        if thread_gone && !clear_channel {
            // The acquisition thread exited on its own — it panicked, or it
            // gave up during connect. Nothing more will ever arrive on this
            // channel, so drop the connection instead of leaving a green
            // "Connected" dot and enabled controls in front of a dead thread.
            error!("UI: acquisition thread exited unexpectedly");
            if self.connection.last_error.is_none() {
                self.connection.last_error = Some(ConnectionIssue::Other(
                    "Acquisition stopped unexpectedly \u{2014} reconnect to resume".to_string(),
                ));
            }
            clear_channel = true;
        }

        if clear_channel {
            // Disconnect properly: send stop signal so the background thread exits
            self.disconnect();
        }
    }

    /// What the session currently has to say about why there are no readings,
    /// or `None` when there is nothing to report.
    ///
    /// Split from the rendering because the big-meter modes cannot draw all of
    /// it: with the reading scaled to fill the window there is no room below
    /// it, so they put `title` where the reading's placeholder goes and hand
    /// `body` to a tooltip. The normal layouts still draw every piece.
    pub(super) fn connection_notice(&self) -> Option<ConnectionNotice> {
        let notice = |kind, title: String, body: String| ConnectionNotice {
            kind,
            title,
            body,
            experimental_link: None,
        };

        // The probe is still running: nothing names the meter, the channel is
        // up, and neither a connection nor a failure has come back yet. It
        // spends up to a couple of seconds giving each family its turn to
        // answer, and an empty column for that long reads as a hang.
        if self.selected_device().is_none()
            && self.connection.state == ConnectionState::Disconnected
            && self.connection.rx.is_some()
            && self.connection.last_error.is_none()
        {
            return Some(notice(
                NoticeKind::Detecting,
                "Detecting the meter\u{2026}".to_string(),
                String::new(),
            ));
        }

        // Show waiting indicator before error threshold
        if self.connection.waiting_timeouts > 0 && self.connection.last_error.is_none() {
            let dots = ".".repeat((self.connection.waiting_timeouts as usize % 4) + 1);
            // Padded to a fixed field: in the big meter this title is measured
            // to fit the window, and a line that grows and shrinks four times
            // a second would either re-fit on every dot or overflow by three
            // characters. The padding is invisible either way.
            let title = format!("Waiting for meter{dots:<4}");
            // Under Auto-detect there is no selection to check, so the hint is
            // the other two things that make a quiet meter talk. With a family
            // selected the way out is named too: the value may have been saved
            // by a detected connect rather than picked, and the user swapping
            // meters has no reason to connect the silence to it.
            let body = if self.selected_device().is_some() {
                "Check that the correct device is selected in Settings (\u{2699}), \
                 or pick Auto-detect there"
            } else {
                "Switch on the meter's USB mode, or pick the model in Settings (\u{2699})"
            };
            return Some(notice(NoticeKind::Waiting, title, body.to_string()));
        }

        let issue = self.connection.last_error.as_ref()?;

        if *issue == ConnectionIssue::DeviceNotFound {
            // HID device not found — dongle issue
            // The hint's lines come from the library so the CLI's cable-not-found
            // help and this panel stay the same advice; only the closing line is
            // the GUI's, since the CLI has no Connect button.
            let platform_hint = format!(
                "{}\n\nClick \"Connect\" after resolving the issue.",
                dmm_lib::binary_help::transport_setup_hint().join("\n")
            );
            // Auto-detect names no meter, so there is no protocol to warn
            // about until one answers.
            let experimental_link = self
                .selected_profile
                .as_ref()
                .filter(|p| !p.stability.is_verified())
                .map(|profile| {
                    (
                        format!(
                            "{} Report feedback.",
                            dmm_lib::binary_help::experimental_warning(
                                profile.model_name,
                                profile.stability
                            )
                        ),
                        profile.feedback_url(),
                    )
                });
            Some(ConnectionNotice {
                kind: NoticeKind::DeviceNotFound,
                title: "USB cable not found".to_string(),
                body: platform_hint,
                experimental_link,
            })
        } else if let ConnectionIssue::AdapterNotFound { help } = issue {
            Some(notice(
                NoticeKind::AdapterNotFound,
                "Adapter not found".to_string(),
                help.clone(),
            ))
        } else if let ConnectionIssue::NotIdentified { help } = issue {
            // The cable is fine and the probe ran; nothing on the far end
            // spoke a protocol we know.
            Some(notice(
                NoticeKind::NotIdentified,
                "No meter answered over the USB cable".to_string(),
                help.clone(),
            ))
        } else {
            // Dongle found but meter not responding.
            // The meter this session is talking about: the one picked, or the
            // one detection found. Under Auto-detect, before anything answered,
            // there is neither a model to name nor steps to give.
            let instructions = match self.active_device() {
                Some(entry) => format!(
                    "The USB adapter is connected but the meter \n\
                     isn't responding ({} selected).\n\
                     \n\
                     If this is the wrong device, change it in Settings (\u{2699}), or pick Auto-detect there.\n\
                     Otherwise, enable data transmission:\n\
                     {}",
                    entry.display_name, entry.activation_instructions
                ),
                None => "No meter answered over the USB cable \u{2014} switch on the meter's \n\
                         USB mode, or pick the model in Settings (\u{2699})."
                    .to_string(),
            };
            Some(notice(
                NoticeKind::NoResponse,
                "No response from meter".to_string(),
                instructions,
            ))
        }
    }

    /// Draw the whole notice under the reading: title, steps, and the
    /// experimental-support link where there is one.
    ///
    /// The big-meter modes do not call this — see [`App::connection_notice`].
    pub(super) fn show_connection_help(&self, ui: &mut Ui) {
        let Some(notice) = self.connection_notice() else {
            return;
        };
        let warn_color = self
            .settings
            .theme_colors(ui.visuals().dark_mode)
            .status_warning();

        // The two transient notices sit closer to the reading than an error
        // does, as they always have: they replace themselves within seconds.
        ui.add_space(match notice.kind {
            NoticeKind::Detecting | NoticeKind::Waiting => 4.0,
            _ => 8.0,
        });
        ui.label(RichText::new(&notice.title).color(warn_color));
        if !notice.body.is_empty() {
            ui.label(
                RichText::new(&notice.body)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
        if let Some((text, url)) = &notice.experimental_link {
            ui.hyperlink_to(RichText::new(text).small().color(warn_color), url)
                .on_hover_text(
                    "Opens the GitHub issue tracker to report experimental-support feedback",
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    /// An app whose settings name `family`, with `overridden` saying whether
    /// that came from `--device` rather than the file.
    ///
    /// Nothing here may reach [`Settings::save`] — it writes the real config
    /// file — so every app-level case below is one the save decision refuses.
    fn app(family: &str, overridden: bool) -> App {
        let mut settings = Settings::default();
        settings.shared.device_family = family.to_string();
        if overridden {
            settings.overrides.device_family = Some(String::new());
        }
        App::from_settings(settings, dmm_lib::Clock::real())
    }

    fn toast_text(app: &App) -> Option<&str> {
        app.toast.as_ref().map(|(msg, _, _)| msg.as_str())
    }

    /// The notice the app would draw, with `issue` as the failure on record.
    fn notice_for(app: &mut App, issue: ConnectionIssue) -> ConnectionNotice {
        app.connection.last_error = Some(issue);
        app.connection_notice()
            .expect("a failure is always a notice")
    }

    /// A session with nothing wrong has nothing to say: the reading column
    /// draws the reading and stops there.
    #[test]
    fn a_quiet_session_has_no_notice() {
        let app = app("ut61eplus", false);
        assert!(app.connection_notice().is_none());
    }

    /// Every failure splits into a title that names the problem and a body
    /// that says what to do about it — the split the big meter needs, which
    /// has room for the title only and hands the body to a tooltip.
    #[test]
    fn each_connection_issue_gets_its_own_notice() {
        let mut app = app("ut61eplus", false);

        let n = notice_for(&mut app, ConnectionIssue::DeviceNotFound);
        assert_eq!(n.kind, NoticeKind::DeviceNotFound);
        assert_eq!(n.title, "USB cable not found");
        assert!(n.body.contains("Connect"), "got {:?}", n.body);

        let n = notice_for(
            &mut app,
            ConnectionIssue::AdapterNotFound {
                help: "no adapter matched".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::AdapterNotFound);
        assert_eq!(n.title, "Adapter not found");
        assert_eq!(n.body, "no adapter matched");

        let n = notice_for(
            &mut app,
            ConnectionIssue::NotIdentified {
                help: "switch USB mode on".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::NotIdentified);
        assert_eq!(n.title, "No meter answered over the USB cable");
        assert_eq!(n.body, "switch USB mode on");

        let n = notice_for(&mut app, ConnectionIssue::Other("timed out".to_string()));
        assert_eq!(n.kind, NoticeKind::NoResponse);
        assert_eq!(n.title, "No response from meter");
        assert!(
            n.body.contains("enable data transmission"),
            "the selected meter's steps belong in the body, got {:?}",
            n.body
        );
    }

    /// The waiting notice is measured to fit the big meter's window, so its
    /// animated dots must not change its width: the fit would otherwise be
    /// redone, or the line overflow, at every timeout.
    #[test]
    fn the_waiting_title_keeps_one_width() {
        let mut app = app("ut61eplus", false);
        let titles: Vec<String> = (1..=9)
            .map(|timeouts| {
                app.connection.waiting_timeouts = timeouts;
                let n = app.connection_notice().expect("waiting is a notice");
                assert_eq!(n.kind, NoticeKind::Waiting);
                n.title
            })
            .collect();
        let widths: Vec<usize> = titles.iter().map(|t| t.chars().count()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "the title changes width as the dots cycle: {titles:?}"
        );
        // The dots are still animating — a fixed width must not have been
        // bought by dropping them.
        assert!(
            titles
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= 4,
            "the dots stopped cycling: {titles:?}"
        );
    }

    /// The mitigation itself: the meter that answered the probe is saved, so
    /// the session after this one opens it pinned rather than walking the
    /// cascade at every connect.
    #[test]
    fn a_detected_meter_is_saved_as_the_device() {
        assert_eq!(
            detected_device_to_save(registry::AUTO_DEVICE_ID, false, Some("ut61eplus")),
            Some("ut61eplus")
        );
    }

    /// `--device auto` detects for one run. Saving under it would either be
    /// undone by `Settings::save` (which writes the file's own value back
    /// under an override) or persist a choice made for a single session.
    #[test]
    fn a_command_line_auto_detects_without_saving() {
        assert_eq!(
            detected_device_to_save(registry::AUTO_DEVICE_ID, true, Some("ut61eplus")),
            None
        );
    }

    /// The mock is a debugging aid, never the meter on the cable — detection
    /// cannot land on it, and nothing here should be what makes that true.
    #[test]
    fn a_mock_connect_is_never_saved() {
        assert_eq!(
            detected_device_to_save(registry::AUTO_DEVICE_ID, false, Some("mock")),
            None
        );
    }

    /// A drop mid-session re-probes and reports the same meter again. Writing
    /// the file on each of those would rewrite it for the life of the session.
    #[test]
    fn a_value_that_already_names_that_meter_is_not_re_saved() {
        assert_eq!(
            detected_device_to_save("ut61eplus", false, Some("ut61eplus")),
            None
        );
        // Including when the file spells it as one of the entry's aliases.
        assert_eq!(
            detected_device_to_save("ut61e", false, Some("ut61eplus")),
            None
        );
        // And a meter the user picked themselves is left as they left it.
        assert_eq!(
            detected_device_to_save("ut61b+", false, Some("ut61eplus")),
            None
        );
    }

    /// The saved case tells the user what changed and how to undo it; a
    /// UT61E+ reports its own display name, so quoting it back adds nothing.
    #[test]
    fn the_toast_says_what_was_saved() {
        assert_eq!(
            detected_toast("UT61E+", "UT61E+", true),
            "Detected UT61E+, saved as your device. \
             Pick Auto-detect in Settings to probe again."
        );
        // A family that never reports a name says only what it is.
        assert_eq!(
            detected_toast("UT8803", "", true),
            "Detected UT8803, saved as your device. \
             Pick Auto-detect in Settings to probe again."
        );
    }

    /// An unknown name falls back to the UT61E+ entry, so the toast has to
    /// carry the model the meter actually reported — it is what the user
    /// quotes when reporting it.
    #[test]
    fn the_toast_names_a_model_that_differs_from_the_entry() {
        assert_eq!(
            detected_toast("UT61E+", "UT60BT", true),
            "Detected UT61E+ (the meter reports \"UT60BT\"), saved as your device. \
             Pick Auto-detect in Settings to probe again."
        );
        assert_eq!(
            detected_toast("UT61E+", "UT60BT", false),
            "Detected UT61E+ (the meter reports \"UT60BT\")"
        );
    }

    /// Nothing was saved, so the toast must not say it was.
    #[test]
    fn the_toast_under_a_command_line_auto_only_names_the_meter() {
        let mut app = app(registry::AUTO_DEVICE_ID, true);
        app.remember_detected_device(Some("ut61eplus"), "UT61E+");
        assert_eq!(toast_text(&app), Some("Detected UT61E+"));
        assert_eq!(
            app.settings.shared.device_family,
            registry::AUTO_DEVICE_ID,
            "a session-only override must not reach the settings"
        );
        // The session is already talking to that meter; re-opening it would
        // cost a reconnect and a hole in the graph for nothing.
        assert!(!app.connection.needs_reconnect);
    }

    /// A reconnect under a saved or picked meter has nothing to announce —
    /// the user is looking at the model they chose.
    #[test]
    fn a_connect_under_a_named_meter_is_silent() {
        for family in ["ut61eplus", "mock"] {
            let mut app = app(family, false);
            app.remember_detected_device(Some(family), "UT61E+");
            assert_eq!(toast_text(&app), None, "{family} should say nothing");
        }
    }

    /// The adapter case used to be recovered at the render site with
    /// `error.contains("adapter not found")`, and "no adapter on the bus"
    /// travelled as the sentinel string "__device_not_found__".
    #[test]
    fn adapter_error_is_classified_with_its_selector() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::AdapterNotFound(
            "ABC123".to_string(),
        ));
        let ConnectionIssue::AdapterNotFound { help } = issue else {
            panic!("expected AdapterNotFound, got {issue:?}");
        };
        assert!(help.contains("ABC123"), "got {help}");
    }

    /// `DeviceNotIdentified` is an `ErrorKind::Timeout`, which would land it
    /// in `Other` and print the "no response from meter" block naming a
    /// device nobody selected. It is matched by variant for that reason.
    #[test]
    fn nothing_answering_the_probe_is_its_own_case() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::DeviceNotIdentified {
            bridge: "CP2110",
        });
        let ConnectionIssue::NotIdentified { help } = issue else {
            panic!("expected NotIdentified, got {issue:?}");
        };
        assert!(help.contains("UT61E+"), "got {help}");
        assert!(help.contains("Long press the USB/Hz button"), "got {help}");
        // gui.md: user-facing text names the cable, never the bridge chip.
        for chip in ["CP2110", "CH9329", "CH9325"] {
            assert!(!help.contains(chip), "{chip} leaked into: {help}");
        }
    }

    /// Six UT61+ models share one four-step instruction. Listing each meter
    /// separately would repeat those steps six times in a panel the reading
    /// column has to fit.
    #[test]
    fn meters_sharing_activation_steps_are_listed_together() {
        let help = not_identified_help("CP2110");
        assert!(
            help.contains("UT61E+, UT61B+, UT61D+, UT161B, UT161D, UT161E"),
            "got {help}"
        );
        assert_eq!(
            help.matches("Long press the USB/Hz button").count(),
            1,
            "the shared steps appear once: {help}"
        );
        // The mock is not on any bridge, so it is never offered as a cure for
        // a silent cable.
        assert!(!help.contains("Mock"), "got {help}");
        assert!(
            help.trim_end()
                .ends_with("Pick its model in Settings (\u{2699}) and please report it."),
            "the help must end with the way out: {help}"
        );
    }

    /// The USB-cable help used to be selected on the thread side, before the
    /// error crossed the channel; it now falls out of the error's kind.
    #[test]
    fn a_missing_adapter_is_the_device_not_found_case() {
        assert_eq!(
            ConnectionIssue::from_error(&dmm_lib::error::Error::NoTransportFound),
            ConnectionIssue::DeviceNotFound
        );
    }

    #[test]
    fn other_errors_keep_their_message() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::Timeout);
        assert_eq!(
            issue,
            ConnectionIssue::Other("timeout waiting for response".to_string())
        );
    }

    /// A message that merely mentions the phrase must not be mistaken for
    /// the adapter case — the old `contains` probe would have matched it,
    /// and the prefix probe that replaced it would have matched the same
    /// text arriving with an "adapter not found: " prefix from elsewhere.
    #[test]
    fn a_mention_of_the_phrase_is_not_the_adapter_case() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::UnknownDevice(
            "meter reports adapter not found somewhere".to_string(),
        ));
        assert!(matches!(issue, ConnectionIssue::Other(_)));
    }
}
