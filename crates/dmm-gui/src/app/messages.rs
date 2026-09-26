//! The UI side of the acquisition channel: opening and closing it, draining
//! the messages the device thread sends, and classifying the reason there is
//! nothing to show into the help text the reading column renders.

use dmm_lib::binary_help::{ConnectedAdapters, LinksSearched, SetupSection, connected_adapters};
use dmm_lib::measurement::Measurement;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::{MeterKeys, registry};
use eframe::egui::{self, RichText, Ui};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Instant, SystemTime};

use super::connection::{
    self, DmmMessage, RemoteCommand, ThreadContext, ThreadControl, handle_thread_panic,
    run_device_thread,
};
use super::plot_input::{PlotInput, resolve_plot_input};
use super::{App, ConnectionState, named_device};
use crate::graph::PlotSample;
use crate::recording::BufferRole;
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
    /// Nothing answered on the links that were searched. `bluetooth_searched`
    /// is the open path's own answer to whether the radio got a turn, which
    /// decides the title and the sections the help shows.
    DeviceNotFound { bluetooth_searched: bool },
    /// `--adapter` named a USB device and nothing matched it. Carries the
    /// finished help text, including the connected-device list, because
    /// building it enumerates the USB bus — far too heavy for the paint path.
    AdapterNotFound { help: String },
    /// `--adapter` named a Bluetooth adapter and nothing answered at that
    /// address. The scan the open just ran is the only thing that would say
    /// which addresses are live, so nothing is enumerated here.
    BluetoothNotFound { address: String },
    /// `--adapter` named a Bluetooth adapter the stack could not connect to —
    /// most often one that is paired but asleep. `error` is what the stack
    /// said, shown above the same steps an address nothing answered gets.
    BluetoothUnreachable { address: String, error: String },
    /// A meter with the radio built in was searched for and nothing in range
    /// carried its name. Its display name and the registry's steps that
    /// switch its radio on, from the error.
    BluetoothOnlyNotFound {
        model: &'static str,
        activation: &'static str,
    },
    /// The stack could not search for a meter with the radio built in —
    /// radio off, permission denied. `error` is what the stack said, shown
    /// above the meter's own steps.
    BluetoothOnlyUnreachable {
        model: &'static str,
        activation: &'static str,
        error: String,
    },
    /// A meter with the radio built in, and the radio was not searched: the
    /// setting is off, the build has no Bluetooth, or `--adapter` names a USB
    /// device. `message` is the library's, which says which, finished with
    /// the GUI's own switch when the setting is off.
    BluetoothNotSearched { message: String },
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
    /// Nothing on the USB bus, which is the only link that was searched.
    UsbNotFound,
    /// Nothing on either link.
    NoMeterFound,
    /// Nothing at the Bluetooth address `--adapter` named.
    BluetoothNotFound,
    /// The adapter at that address would not connect, or the stack could
    /// not search for a meter with the radio built in.
    BluetoothUnreachable,
    /// Nothing in range carried the name of a meter with the radio built in.
    BluetoothOnlyNotFound,
    /// A meter with the radio built in, and the radio was not searched.
    BluetoothNotSearched,
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
    /// The steps to try, one section per link, each drawn under its own bold
    /// label. Empty for a notice that is about no link in particular.
    pub(super) sections: Vec<SetupSection>,
    /// The prose under them, or the whole of a sectionless notice's steps.
    /// Several hard-wrapped lines; empty while detection is still running,
    /// where there is nothing to do but wait.
    pub(super) body: String,
    /// The experimental-support feedback link, as `(text, url)`.
    pub(super) experimental_link: Option<(String, String)>,
}

impl ConnectionNotice {
    /// Everything under the title as one string, for the big-meter modes:
    /// they have room for the title alone and hand this to a tooltip.
    pub(super) fn help_text(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            out.push_str(&section.text());
            out.push_str("\n\n");
        }
        out.push_str(&self.body);
        out
    }
}

impl ConnectionIssue {
    /// Classify an error the acquisition thread hands over whole.
    ///
    /// Matched by variant rather than by [`dmm_lib::error::ErrorKind`]: the
    /// help needs what each one carries — the selector the user typed, the
    /// links that were searched, the bridge nothing answered on — and the
    /// coarse kind carries none of it. Classified once when the error
    /// arrives, not at the render site on every repaint. `adapter` is the
    /// `--adapter` value the open was given, which a stack failure is
    /// explained by when it names a Bluetooth adapter; `selected` the meter
    /// picked in Settings, which explains one when it has the radio built in.
    fn from_error(
        err: &dmm_lib::error::Error,
        adapter: Option<&str>,
        selected: Option<&'static registry::SelectableDevice>,
    ) -> Self {
        if let dmm_lib::error::Error::AdapterNotFound(selector) = err {
            // An address is explained by the link it named, not by the USB
            // bus — nothing on the bus could have answered it.
            if dmm_lib::is_bluetooth_selector(selector) {
                return Self::BluetoothNotFound {
                    address: selector.clone(),
                };
            }
            return Self::AdapterNotFound {
                help: adapter_not_found_help(selector),
            };
        }
        // The open path already worked out which links it searched; carrying
        // the answer keeps the help from deriving it again from the build,
        // the settings and the selected meter.
        if let dmm_lib::error::Error::NoTransportFound { bluetooth_searched } = err {
            return Self::DeviceNotFound {
                bluetooth_searched: *bluetooth_searched,
            };
        }
        // Also a `Timeout` kind — retrying does help once transmission is on —
        // so it is matched by variant, ahead of the kind test below.
        if let dmm_lib::error::Error::DeviceNotIdentified {
            bridge,
            built_in_radio,
        } = err
        {
            return Self::NotIdentified {
                help: not_identified_help(bridge, *built_in_radio),
            };
        }
        if let dmm_lib::error::Error::Bluetooth(_) = err
            && let Some(address) = adapter.filter(|a| dmm_lib::is_bluetooth_selector(a))
        {
            return Self::BluetoothUnreachable {
                address: address.to_string(),
                error: err.to_string(),
            };
        }
        if let dmm_lib::error::Error::Bluetooth(_) = err
            && let Some(device) = selected.filter(|d| d.bluetooth_only)
        {
            return Self::BluetoothOnlyUnreachable {
                model: device.display_name,
                activation: device.activation_instructions,
                error: err.to_string(),
            };
        }
        if let dmm_lib::error::Error::BluetoothOnly {
            model,
            activation,
            miss,
        } = err
        {
            return match miss {
                dmm_lib::error::BluetoothOnlyMiss::NotInRange => {
                    Self::BluetoothOnlyNotFound { model, activation }
                }
                // The remedy is the GUI's own switch; the library's sentence
                // names none.
                dmm_lib::error::BluetoothOnlyMiss::SwitchedOff => Self::BluetoothNotSearched {
                    message: format!("{err}. {}", dmm_lib::binary_help::gui_bluetooth_off_hint()),
                },
                _ => Self::BluetoothNotSearched {
                    message: format!("{err}."),
                },
            };
        }
        Self::Other(err.to_string())
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
    if let Some(hint) = dmm_lib::binary_help::colonless_address_hint(selector) {
        msg.push_str("\n\n");
        msg.push_str(&hint);
    }
    msg
}

/// Build the "nothing answered" help: what the user can switch on, for every
/// meter that could have been behind that bridge.
///
/// Families share activation steps — the whole UT61+ line is one instruction —
/// so the meters are grouped by the instruction text rather than listed one by
/// one, which would repeat the same four steps six times over.
fn not_identified_help(bridge: &str, built_in_radio: bool) -> String {
    let mut msg = format!(
        "The {} is connected but no meter identified itself.\n\n\
         Switch on the meter's data transmission, or pick the model in \
         Settings (\u{2699}):\n",
        dmm_lib::binary_help::bridge_link_name(bridge, built_in_radio)
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
                "Recording stopped \u{2014} its {kept} samples are kept, Export\u{2026} saves them"
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

    /// Make this Connect the session's zero, the first time a recording is
    /// opened.
    ///
    /// Playback measures every frame's offset from the origin, so pinning it
    /// while the arguments were parsed dropped whatever fell due while the
    /// window and the GPU were starting — and with auto-connect off, a Connect
    /// past the recording's length found nothing left but the held last frame.
    /// A later Disconnect/Connect keeps the origin, so the recording resumes
    /// where the session has got to.
    fn pin_replay_origin(&mut self, recorded: SystemTime) {
        if self.clock.wall_origin().is_some() {
            return;
        }
        self.clock = self.clock.clone().with_wall_origin(recorded);
        // The pair the recording and its exports date their samples by was
        // captured before there was an origin to take.
        self.wall_clock = dmm_lib::WallClock::from_clock(&self.clock);
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

        let source = self
            .replay
            .as_ref()
            .map(|source| (Arc::clone(&source.replay), source.recorded));
        if let Some((replay, recorded)) = source {
            self.pin_replay_origin(recorded);
            // The file says which meter its frames came from, so that entry is
            // reported rather than whatever the Settings row currently names.
            let selected = Some(replay.device);
            // And which link they came over: playback is on none of its own.
            let recorded_link = replay.link;
            let clock = self.clock.clone();
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        // A replay cannot fail once it is open, so the retry
                        // loop never re-runs this; a manual Disconnect then
                        // Connect does. The origin the first Connect pinned
                        // stands for the rest of the session, so re-opening
                        // picks the recording up where the session has got to
                        // instead of starting it again. Nothing to detect —
                        // the file names it.
                        move |_| replay.open(clock.clone()).map(|dmm| (dmm, None)),
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
                            selected,
                            query_name,
                            recorded_link,
                            // No floor: the recording's own spacing is the
                            // cadence, and the protocol sleeps until each
                            // frame is due rather than returning at once.
                            sample_interval_ms,
                            stop_flag,
                        },
                    );
                }));
                if let Err(panic) = result {
                    handle_thread_panic(panic, &panic_tx, &panic_ctx);
                }
            });
        } else if let Some(device) = device_entry.filter(|d| !d.requires_hardware) {
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
                        move |_| {
                            dmm_lib::mock::open_simulated(device, mock_mode, clock.clone())
                                .map(|dmm| (dmm, None))
                        },
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
                            selected: device_entry,
                            query_name,
                            recorded_link: None,
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
            let bluetooth = self.settings.shared.bluetooth;
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        // Re-run on every reconnect, detection included: a
                        // meter that comes back is identified again rather
                        // than assumed to be the one that left. `reopen_at`
                        // is the Bluetooth adapter a lost link was on, opened
                        // by address like `--adapter`, which wins if given.
                        move |reopen_at| {
                            let opts = dmm_lib::OpenOptions {
                                adapter: adapter.as_deref().or(reopen_at),
                                bluetooth,
                            };
                            match device_id {
                                Some(id) => {
                                    dmm_lib::open_device_by_id_auto(id, opts).map(|dmm| (dmm, None))
                                }
                                None => dmm_lib::open_auto(opts)
                                    .map(|(dmm, detected)| (dmm, Some(detected))),
                            }
                        },
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
                            selected: device_entry,
                            query_name,
                            recorded_link: None,
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
        self.connection.link = None;
        self.connection.supported_commands.clear();
        self.connection.meter_keys = MeterKeys::NONE;
        self.connection.choices.clear();
        self.connection.paused = false;
        self.connection.reconnect_attempt = 0;
        self.connection.reconnect_last_error = None;
        // Otherwise only an incoming measurement clears this, and there won't
        // be one — a meter that went quiet before the user disconnected left
        // "Waiting for meter…" on screen for the whole disconnected session.
        self.connection.waiting_timeouts = 0;
    }

    /// Drop everything derived from the sample stream: graph history and
    /// the samples Export… would save from it, session statistics, the
    /// integrator and the last reading.
    ///
    /// Shared by `Ctrl+L`, the Clear button and a change of software scale,
    /// which all mean the same thing — the numbers accumulated so far no
    /// longer describe what is being measured. A recording is deliberately
    /// not touched; Clear has never discarded a capture.
    pub(super) fn clear_session(&mut self) {
        self.graph.clear();
        self.recording.clear_history();
        self.session.reset();
        self.last_measurement = None;
    }

    /// Keep a reading in the sample buffer: the recording's next sample, or
    /// — with nothing recorded — the history Export… falls back to, cut to
    /// what the graph holds.
    ///
    /// Runs after the graph has taken the reading, so a mode change the graph
    /// restarted on already shows as its first point. An empty graph cuts
    /// nothing: NCV and over-range readings add no point, and are still
    /// readings to export.
    fn keep_sample(&mut self, m: &Measurement, extra_aux: usize) {
        if self.recording.role() == BufferRole::History {
            // `detected`, not the selection: a device picked in Settings
            // takes effect at the next connect, and this reading may still
            // be the old meter's. A file names one meter, so another one
            // starts another history.
            let meter = self.connection.detected.map(|d| d.display_name);
            if meter != self.capture_layout.device {
                self.recording.clear_history();
            }
            if let Some(start) = self.graph.first_point_time() {
                self.recording.trim_before(start);
            }
            if self.recording.samples.is_empty() {
                self.latch_history_layout();
            }
        }
        if self.recording.push(m, &self.wall_clock, extra_aux) {
            self.buffer_full_toast();
        }
    }

    /// Take the export's provenance and columns for a history about to start,
    /// from the connection its first reading arrived on — the history's
    /// counterpart of what Record latches.
    fn latch_history_layout(&mut self) {
        let meter = self.connection.detected;
        self.capture_layout.device = meter.map(|d| d.display_name);
        // Only a meter's frames can be replayed; the mock has none.
        self.capture_layout.device_id = meter.filter(|d| d.requires_hardware).map(|d| d.id);
        // A reading only arrives on a live connection, so this stability and
        // this link are the meter's rather than the defaults a disconnect
        // leaves behind.
        self.capture_layout.experimental = Some(!self.connection.stability.is_verified());
        self.capture_layout.link = self.connection.link;
        self.capture_layout.aux_slots = self.capture_layout.device_aux_slots;
        // A scale change clears the history, so the transform in force now is
        // the one every sample in it went through.
        self.capture_layout.extra_slots = self.transform.extra_aux_count();
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
                    link,
                    supported_commands: cmds,
                    meter_keys,
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
                        // The meter Record was pressed ahead of: the first one
                        // to answer during this recording is the one its
                        // samples come from, and the only one the export can
                        // still name once the cable is out.
                        self.capture_layout
                            .experimental
                            .get_or_insert(!stability.is_verified());
                    }
                    self.connection.feedback_url = feedback_url;
                    self.connection.link = link;
                    self.connection.supported_commands = cmds;
                    self.connection.meter_keys = meter_keys;
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
                            ..
                        }) => self.graph.push_sample(PlotSample {
                            value: v,
                            timestamp: m.timestamp,
                            mode: &m.mode,
                            unit,
                            display_raw,
                            series,
                            overlays: &overlays,
                        }),
                        // A word instead of a reading ("Auto" with the probes
                        // lifted): a break too, but not an over-range one.
                        // It never reaches `push_sample`, so its mode and unit
                        // cannot restart the trace either.
                        Some(PlotInput {
                            value: None,
                            no_reading: true,
                            ..
                        }) => self.graph.push_no_reading(m.timestamp),
                        // The plotted series is over range: no point, but the
                        // trace has to break so it isn't drawn straight
                        // through the excursion.
                        Some(PlotInput { value: None, .. }) => self.graph.push_break(m.timestamp),
                        None => {}
                    }

                    // `m` has already been through the transform, so the
                    // count it appended is what this sample carries.
                    self.keep_sample(&m, self.transform.extra_aux_count());

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
                    self.connection.last_error = Some(ConnectionIssue::from_error(
                        &e,
                        self.settings.overrides.adapter.as_deref(),
                        self.selected_device(),
                    ));
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
                DmmMessage::NoResponse => {
                    // A gap in a recording plays back as timeouts and reaches
                    // this threshold too. It is not a quiet meter: there is no
                    // device selection to check, no USB mode to switch on, and
                    // the file carries on by itself once the gap is over.
                    if self.replay.is_none() {
                        error!("UI: error: {}", connection::NO_RESPONSE);
                        self.connection.last_error =
                            Some(ConnectionIssue::Other(connection::NO_RESPONSE.to_string()));
                        if self.connection.state == ConnectionState::Disconnected {
                            clear_channel = true;
                        }
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
            sections: Vec::new(),
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
            // A stretch of the recording with nothing in it, which no meter
            // and no cable can be asked about. Said plainly and left there:
            // the file plays on by itself once the gap is over.
            if self.replay.is_some() {
                return Some(notice(
                    NoticeKind::Waiting,
                    "No frames in the recording here".to_string(),
                    String::new(),
                ));
            }
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
                "Switch on the meter's data transmission, or pick the model in Settings (\u{2699})"
            };
            return Some(notice(NoticeKind::Waiting, title, body.to_string()));
        }

        let issue = self.connection.last_error.as_ref()?;

        if let ConnectionIssue::DeviceNotFound { bluetooth_searched } = issue {
            // Nothing answered on the links the open path searched. The
            // sections come from the library so the CLI's help and this panel
            // stay the same advice; only the closing line is the GUI's, since
            // the CLI has no Connect button.
            let links = LinksSearched::from_usb_failure(*bluetooth_searched);
            Some(ConnectionNotice {
                kind: if *bluetooth_searched {
                    NoticeKind::NoMeterFound
                } else {
                    NoticeKind::UsbNotFound
                },
                title: links.not_found_title(),
                sections: links.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: self.experimental_link(),
            })
        } else if let ConnectionIssue::BluetoothOnlyNotFound { model, activation } = issue {
            // The same notice, on the radio alone and with the meter's own
            // steps: it has no cable, and no adapter to wake.
            let links = LinksSearched::BluetoothOnly { model, activation };
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothOnlyNotFound,
                title: links.not_found_title(),
                sections: links.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: self.experimental_link(),
            })
        } else if let ConnectionIssue::BluetoothOnlyUnreachable {
            model,
            activation,
            error,
        } = issue
        {
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: error.clone(),
                sections: LinksSearched::BluetoothOnly { model, activation }.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::BluetoothNotSearched { message } = issue {
            Some(notice(
                NoticeKind::BluetoothNotSearched,
                "Bluetooth not searched".to_string(),
                message.clone(),
            ))
        } else if let ConnectionIssue::BluetoothNotFound { address } = issue {
            let links = LinksSearched::BluetoothAt(address);
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothNotFound,
                title: links.not_found_title(),
                sections: links.sections(),
                // The GUI cannot scan, so the way back in is a restart with
                // the address the scan printed.
                body: "Restart with --adapter set to the address it prints.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::BluetoothUnreachable { address, error } = issue {
            // Found, or remembered, but the link would not come up: the
            // stack's own words first, then the steps that wake an adapter.
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: error.clone(),
                sections: LinksSearched::BluetoothAt(address).sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::AdapterNotFound { help } = issue {
            Some(notice(
                NoticeKind::AdapterNotFound,
                "Adapter not found".to_string(),
                help.clone(),
            ))
        } else if let ConnectionIssue::NotIdentified { help } = issue {
            // The link is fine and the probe ran; nothing on the far end
            // spoke a protocol we know. Which link it was is in the body —
            // the title stays one string per notice, which is what the big
            // meter's fit cache keys on.
            Some(notice(
                NoticeKind::NotIdentified,
                "No meter answered".to_string(),
                help.clone(),
            ))
        } else {
            // Dongle found but meter not responding.
            // The meter this session is talking about: the one picked, or the
            // one detection found. Under Auto-detect, before anything answered,
            // there is neither a model to name nor steps to give.
            // Name the link the session is on: "adapter" now reads as the
            // Bluetooth one, and over a cable it never was one.
            let built_in_radio = self.active_device().is_some_and(|d| d.bluetooth_only);
            let link = self
                .connection
                .link
                .map_or("link", |link| link.full_name(built_in_radio));
            let instructions = match self.active_device() {
                Some(entry) => format!(
                    "The {link} is connected but the meter \n\
                     isn't responding ({} selected).\n\
                     \n\
                     If this is the wrong device, change it in Settings (\u{2699}), or pick Auto-detect there.\n\
                     Otherwise, enable data transmission:\n\
                     {}",
                    entry.display_name, entry.activation_instructions
                ),
                None => "No meter answered \u{2014} switch on the meter's data \n\
                         transmission, or pick the model in Settings (\u{2699})."
                    .to_string(),
            };
            Some(notice(
                NoticeKind::NoResponse,
                "No response from meter".to_string(),
                instructions,
            ))
        }
    }

    /// The experimental-support line a "nothing found" notice ends with, for
    /// a picked meter short of verified. Auto-detect names no meter, so there
    /// is no protocol to warn about until one answers.
    fn experimental_link(&self) -> Option<(String, String)> {
        self.selected_profile
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
            })
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
        // One section per link, each under its own bold label, so the cable
        // steps and the Bluetooth steps are never read as one list.
        for section in &notice.sections {
            ui.add_space(4.0);
            ui.label(RichText::new(section.label()).small().strong());
            ui.label(
                RichText::new(
                    section
                        .steps
                        .iter()
                        .map(|step| format!("  {step}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        }
        if !notice.body.is_empty() {
            if !notice.sections.is_empty() {
                ui.add_space(4.0);
            }
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
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::MeasuredValue;
    use dmm_lib::protocol::Stability;
    use std::time::Duration;

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

        let n = notice_for(
            &mut app,
            ConnectionIssue::DeviceNotFound {
                bluetooth_searched: false,
            },
        );
        assert_eq!(n.kind, NoticeKind::UsbNotFound);
        assert_eq!(n.title, "No USB cable found");
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
        assert_eq!(n.title, "No meter answered");
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

    /// The title says which links were looked at, and the sections say what
    /// to do on each — the cable steps and the radio steps never run into one
    /// another.
    #[test]
    fn the_not_found_notice_is_grouped_by_link() {
        let mut app = app("ut61eplus", false);

        let n = notice_for(
            &mut app,
            ConnectionIssue::DeviceNotFound {
                bluetooth_searched: true,
            },
        );
        assert_eq!(n.kind, NoticeKind::NoMeterFound);
        assert_eq!(n.title, "No meter found over USB or Bluetooth");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link).collect();
        assert_eq!(links, ["USB cable", "Bluetooth"]);
        // The big meter shows the title alone and hands the rest to a
        // tooltip, so every step has to reach that one string.
        let help = n.help_text();
        assert!(help.contains("USB cable: check it is plugged in"), "{help}");
        assert!(help.contains("Bluetooth: turn on Bluetooth"), "{help}");
        assert!(help.ends_with("Click \"Connect\" after resolving the issue."));
    }

    /// An address nothing answered is about that link alone: no cable steps,
    /// and no USB bus listing — nothing on the bus could have answered it.
    #[test]
    fn a_bluetooth_address_is_answered_on_its_own_link() {
        let mut app = app("ut61eplus", false);
        let n = notice_for(
            &mut app,
            ConnectionIssue::BluetoothNotFound {
                address: "12:34:56:78:9A:BC".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::BluetoothNotFound);
        assert_eq!(n.title, "No Bluetooth device found at 12:34:56:78:9A:BC");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link).collect();
        assert_eq!(links, ["Bluetooth"]);
        assert!(n.help_text().contains("dmm-cli list"), "{}", n.help_text());
        assert_eq!(
            n.body,
            "Restart with --adapter set to the address it prints."
        );
    }

    /// A named adapter the stack could not reach — paired but asleep — gets
    /// the stack's own words as the title and the same Bluetooth steps as an
    /// address nothing answered, not a silent-meter notice.
    #[test]
    fn an_unreachable_named_adapter_gets_the_bluetooth_steps() {
        let err = dmm_lib::error::Error::Bluetooth("Timed out after 10s".to_string());
        let issue = ConnectionIssue::from_error(&err, Some("12:34:56:78:9A:BC"), None);
        if !dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            // A build without the radio has no address to explain it by.
            assert!(matches!(issue, ConnectionIssue::Other(_)));
            return;
        }
        let mut app = app("ut61eplus", false);
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: Timed out after 10s");
        assert_eq!(
            n.sections,
            LinksSearched::BluetoothAt("12:34:56:78:9A:BC").sections()
        );
        assert_eq!(n.body, "Click \"Connect\" after resolving the issue.");

        // Without an address, or with a USB one, it stays the bare error.
        assert!(matches!(
            ConnectionIssue::from_error(&err, None, None),
            ConnectionIssue::Other(_)
        ));
        assert!(matches!(
            ConnectionIssue::from_error(&err, Some("00C5B27A"), None),
            ConnectionIssue::Other(_)
        ));
    }

    /// A meter with the radio built in is answered on the radio alone, with
    /// its own steps, whichever way the open failed — never with the cable
    /// help. A meter behind an adapter keeps the bare stack error.
    #[test]
    fn a_bluetooth_only_meter_gets_its_own_notice() {
        use dmm_lib::error::{BluetoothOnlyMiss, Error};
        let ut60bt = registry::find_device("ut60bt").expect("registry entry");
        let own_steps = LinksSearched::BluetoothOnly {
            model: ut60bt.display_name,
            activation: ut60bt.activation_instructions,
        };
        let missed = |miss| Error::BluetoothOnly {
            model: ut60bt.display_name,
            activation: ut60bt.activation_instructions,
            miss,
        };
        let mut app = app("ut60bt", false);

        let issue = ConnectionIssue::from_error(&missed(BluetoothOnlyMiss::NotInRange), None, None);
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothOnlyNotFound);
        assert_eq!(n.title, "No UT60BT found over Bluetooth");
        assert_eq!(n.sections, own_steps.sections());
        assert!(
            n.help_text().contains("Long press SEL"),
            "{}",
            n.help_text()
        );
        assert!(n.experimental_link.is_some());

        let stack = Error::Bluetooth("turned off on this computer".to_string());
        let n = notice_for(
            &mut app,
            ConnectionIssue::from_error(&stack, None, Some(ut60bt)),
        );
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: turned off on this computer");
        assert_eq!(n.sections, own_steps.sections());

        let issue =
            ConnectionIssue::from_error(&missed(BluetoothOnlyMiss::SwitchedOff), None, None);
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothNotSearched);
        assert_eq!(n.title, "Bluetooth not searched");
        assert_eq!(
            n.body,
            "UT60BT connects over Bluetooth only, and Bluetooth is switched off. \
             Tick \"Look for Bluetooth devices\" in Settings (\u{2699})."
        );
        assert!(n.sections.is_empty());

        let ut61eplus = registry::find_device("ut61eplus").expect("registry entry");
        assert!(matches!(
            ConnectionIssue::from_error(&stack, None, Some(ut61eplus)),
            ConnectionIssue::Other(_)
        ));
    }

    /// Hand `app` one message from the acquisition thread. The sender is
    /// still alive while it drains, so the channel is not taken for one whose
    /// thread has died.
    fn deliver(app: &mut App, msg: DmmMessage) {
        let (tx, rx) = mpsc::channel();
        tx.send(msg).expect("the channel is open");
        app.connection.rx = Some(rx);
        app.drain_messages();
    }

    /// A gap in a recording plays back as timeouts, and they are not a quiet
    /// meter: nothing is on the cable to select and no USB mode to switch on.
    /// The gap says what it is, and the help stays away.
    #[test]
    fn a_replay_gap_is_not_a_quiet_meter() {
        let mut app = app("ut61eplus", false);
        app.replay = Some(crate::ReplaySource::fixture());
        app.connection.waiting_timeouts = connection::NO_RESPONSE_TIMEOUTS;
        deliver(&mut app, DmmMessage::NoResponse);

        assert!(app.connection.last_error.is_none(), "no failure on record");
        let n = app.connection_notice().expect("the gap is still reported");
        assert_eq!(n.title, "No frames in the recording here");
        assert!(n.body.is_empty(), "got {:?}", n.body);
    }

    /// And a meter that really did go quiet still gets the steps it always
    /// did.
    #[test]
    fn a_quiet_meter_still_gets_the_no_response_help() {
        let mut app = app("ut61eplus", false);
        app.connection.waiting_timeouts = connection::NO_RESPONSE_TIMEOUTS;
        deliver(&mut app, DmmMessage::NoResponse);

        let n = app.connection_notice().expect("a quiet meter is a failure");
        assert_eq!(n.kind, NoticeKind::NoResponse);
        assert!(
            n.body.contains("enable data transmission"),
            "got {:?}",
            n.body
        );
    }

    /// Session zero is the Connect, not the launch: playback measures every
    /// frame's offset from the origin, so pinning it while the arguments were
    /// parsed dropped whatever fell due while the window was starting.
    #[test]
    fn a_replay_pins_session_zero_at_the_first_connect() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.replay = Some(crate::ReplaySource::fixture());
        let recorded = app.replay.as_ref().expect("the recording").recorded;
        assert!(
            app.clock.wall_origin().is_none(),
            "nothing pinned at launch"
        );

        // Whatever the window and the GPU spent starting is behind us.
        let connected_at = Instant::now();
        app.connect(&egui::Context::default());
        let (origin, at) = app.clock.wall_origin().expect("the Connect pins it");
        assert_eq!(at, recorded);
        assert!(
            origin >= connected_at,
            "the recording starts at the Connect"
        );
        // The pair the recording and its exports date samples by sees it too.
        assert_eq!(app.wall_clock.wall_time_for(origin), recorded);

        // A Disconnect/Connect keeps the origin, so the recording resumes
        // where the session has got to rather than starting again.
        app.disconnect();
        app.connect(&egui::Context::default());
        assert_eq!(app.clock.wall_origin().expect("still pinned").0, origin);
        app.disconnect();
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
            detected_toast("UT61E+", "UT216XD", true),
            "Detected UT61E+ (the meter reports \"UT216XD\"), saved as your device. \
             Pick Auto-detect in Settings to probe again."
        );
        assert_eq!(
            detected_toast("UT61E+", "UT216XD", false),
            "Detected UT61E+ (the meter reports \"UT216XD\")"
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
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::AdapterNotFound("ABC123".to_string()),
            None,
            None,
        );
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
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::DeviceNotIdentified {
                bridge: "CP2110",
                built_in_radio: false,
            },
            None,
            None,
        );
        let ConnectionIssue::NotIdentified { help } = issue else {
            panic!("expected NotIdentified, got {issue:?}");
        };
        assert!(help.contains("UT61E+"), "got {help}");
        assert!(help.contains("Long press the USB/Hz button"), "got {help}");
        // gui.md: user-facing text names the cable, never the bridge chip.
        assert!(help.contains("USB cable is connected"), "got {help}");
        for chip in ["CP2110", "CH9329", "CH9325"] {
            assert!(!help.contains(chip), "{chip} leaked into: {help}");
        }
    }

    /// The same probe over the radio: the user switched an adapter on, so the
    /// help must not send them looking at a cable.
    #[test]
    fn the_silent_link_is_named_as_the_user_sees_it() {
        let help = not_identified_help(dmm_lib::BLUETOOTH, false);
        assert!(
            help.starts_with("The Bluetooth adapter is connected"),
            "got {help}"
        );
        // The meters UNI-T lists on the adapter, with their steps. The steps
        // themselves are the meter's own and name its USB socket, which is
        // where the adapter plugs in.
        assert!(help.contains("UT61E+"), "got {help}");
        // A meter with the radio built in answered the open: no adapter.
        let help = not_identified_help(dmm_lib::BLUETOOTH, true);
        assert!(
            help.starts_with("The Bluetooth link is connected but no meter identified itself."),
            "got {help}"
        );
    }

    /// A quiet meter over the radio: an adapter for a meter behind one, the
    /// link for a meter with the radio built in.
    #[test]
    fn a_quiet_meter_names_its_bluetooth_link() {
        for (id, link) in [("ut61eplus", "adapter"), ("ut60bt", "link")] {
            let mut app = app(id, false);
            app.connection.link = Some(dmm_lib::binary_help::Link::Bluetooth);
            let n = notice_for(&mut app, ConnectionIssue::Other("timed out".to_string()));
            assert_eq!(n.kind, NoticeKind::NoResponse);
            assert!(
                n.body
                    .starts_with(&format!("The Bluetooth {link} is connected but the meter")),
                "{id}: {:?}",
                n.body
            );
        }
    }

    /// Six UT61+ models share one four-step instruction. Listing each meter
    /// separately would repeat those steps six times in a panel the reading
    /// column has to fit.
    #[test]
    fn meters_sharing_activation_steps_are_listed_together() {
        let help = not_identified_help("CP2110", false);
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

    /// The help used to be selected on the thread side, before the error
    /// crossed the channel; it now falls out of the error, which carries the
    /// links the open path searched.
    #[test]
    fn a_missing_adapter_is_the_device_not_found_case() {
        for bluetooth_searched in [false, true] {
            assert_eq!(
                ConnectionIssue::from_error(
                    &dmm_lib::error::Error::NoTransportFound { bluetooth_searched },
                    None,
                    None,
                ),
                ConnectionIssue::DeviceNotFound { bluetooth_searched }
            );
        }
    }

    /// A Bluetooth address takes the link's own case, not the bus listing:
    /// `connected_adapters()` would walk every HID device for an answer that
    /// could not contain the address anyway.
    #[test]
    fn an_unanswered_address_is_classified_as_a_bluetooth_failure() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::AdapterNotFound("12:34:56:78:9A:BC".to_string()),
            None,
            None,
        );
        let expected = if dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            ConnectionIssue::BluetoothNotFound {
                address: "12:34:56:78:9A:BC".to_string(),
            }
        } else {
            // A build without the radio has no link that value could name.
            ConnectionIssue::AdapterNotFound {
                help: adapter_not_found_help("12:34:56:78:9A:BC"),
            }
        };
        assert_eq!(issue, expected);
    }

    #[test]
    fn other_errors_keep_their_message() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::Timeout, None, None);
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
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::UnknownDevice(
                "meter reports adapter not found somewhere".to_string(),
            ),
            None,
            None,
        );
        assert!(matches!(issue, ConnectionIssue::Other(_)));
    }

    /// A reading in `mode` stamped `at`, as the acquisition thread sends one.
    fn reading(mode: &'static str, value: MeasuredValue, at: Instant) -> Measurement {
        let mut m = Measurement::test_fixture(value, "V", StatusFlags::default());
        m.mode = mode.into();
        m.timestamp = at;
        m
    }

    /// The thread's hello for `device_id`.
    fn connected(
        device_id: &'static str,
        stability: Stability,
        max_aux_values: usize,
    ) -> DmmMessage {
        DmmMessage::Connected {
            name: String::new(),
            model_name: String::new(),
            device_id: Some(device_id),
            stability,
            feedback_url: String::new(),
            link: None,
            supported_commands: Vec::new(),
            meter_keys: MeterKeys::NONE,
            max_aux_values,
        }
    }

    /// Deliver one reading per entry of `modes`, each stamped as it is sent.
    fn deliver_readings(app: &mut App, modes: &[(&'static str, MeasuredValue)]) {
        for (mode, value) in modes {
            let m = reading(mode, value.clone(), Instant::now());
            deliver(app, DmmMessage::Measurement(m));
        }
    }

    /// A UT61E+ connected and sending, with nothing recorded.
    fn connected_app() -> App {
        let mut app = app("ut61eplus", false);
        deliver(&mut app, connected("ut61eplus", Stability::Verified, 0));
        app
    }

    const DC: (&str, MeasuredValue) = ("DC V", MeasuredValue::Normal(1.0));
    const AC: (&str, MeasuredValue) = ("AC V", MeasuredValue::Normal(1.0));

    fn modes(app: &App) -> Vec<&str> {
        app.recording
            .samples
            .iter()
            .map(|s| s.measurement.mode.as_ref())
            .collect()
    }

    /// With nothing recorded the buffer holds what the graph does: a turn of
    /// the dial restarts both.
    #[test]
    fn the_history_restarts_with_the_graph() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, DC, DC]);
        assert_eq!(modes(&app), ["DC V"; 3]);
        deliver_readings(&mut app, &[AC, AC]);
        assert_eq!(modes(&app), ["AC V"; 2]);
    }

    /// An over-range reading adds no graph point but is still a reading.
    #[test]
    fn an_over_range_reading_stays_in_the_history() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, ("DC V", MeasuredValue::Overload), DC]);
        assert_eq!(app.recording.samples.len(), 3);
        assert_eq!(
            app.recording.samples[1].measurement.value_export_str(),
            "OL"
        );
    }

    /// "Auto" with the probes lifted comes under a mode of its own, but it is
    /// no dial turn: the graph, the history and the statistics carry on.
    #[test]
    fn a_no_reading_between_readings_restarts_nothing() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC]);
        let first = app.graph.first_point_time();
        let mut idle = reading("Auto", MeasuredValue::NoReading("Auto"), Instant::now());
        idle.unit = "".into();
        deliver(&mut app, DmmMessage::Measurement(idle));
        deliver_readings(&mut app, &[DC]);
        assert_eq!(modes(&app), ["DC V", "Auto", "DC V"]);
        assert!(first.is_some());
        assert_eq!(app.graph.first_point_time(), first);
        assert_eq!(app.session.stats.count, 2);
        assert_eq!(app.recording.samples[1].measurement.value_export_str(), "");
    }

    /// NCV readings are never plotted: an empty graph cuts nothing.
    #[test]
    fn ncv_readings_are_kept_with_nothing_plotted() {
        let mut app = connected_app();
        let ncv = ("NCV", MeasuredValue::NcvLevel(2));
        deliver_readings(&mut app, &[ncv.clone(), ncv.clone(), ncv]);
        assert_eq!(app.graph.first_point_time(), None);
        assert_eq!(modes(&app), ["NCV"; 3]);
    }

    /// Clear drops the history with the graph, and never a recording.
    #[test]
    fn clear_drops_the_history_but_not_a_recording() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, DC]);
        app.clear_session();
        assert!(app.recording.samples.is_empty());

        app.toggle_recording();
        deliver_readings(&mut app, &[DC, DC]);
        app.clear_session();
        assert_eq!(app.recording.samples.len(), 2);
    }

    /// A recording spans the dial turns the graph restarts on.
    #[test]
    fn a_recording_keeps_every_mode() {
        let mut app = connected_app();
        app.toggle_recording();
        deliver_readings(&mut app, &[DC, DC, AC, AC]);
        assert_eq!(modes(&app), ["DC V", "DC V", "AC V", "AC V"]);
    }

    /// Pause halts acquisition: whatever was still queued is not kept.
    #[test]
    fn a_paused_session_keeps_nothing() {
        let mut app = connected_app();
        app.connection.paused = true;
        deliver_readings(&mut app, &[DC]);
        assert!(app.recording.samples.is_empty());
    }

    /// The history's file names the meter it came from, as a recording's
    /// does, and keeps naming it once the cable is out.
    #[test]
    fn the_history_names_the_meter_it_came_from() {
        let mut app = app("ut181a", false);
        deliver(&mut app, connected("ut181a", Stability::Experimental, 4));
        deliver_readings(&mut app, &[DC]);

        let ut181a = registry::find_device("ut181a").expect("a registry entry");
        assert_eq!(app.capture_layout.device, Some(ut181a.display_name));
        assert_eq!(app.capture_layout.device_id, Some("ut181a"));
        assert_eq!(app.capture_layout.experimental, Some(true));
        assert_eq!(app.capture_layout.aux_slots, 4);

        app.disconnect();
        assert!(app.experimental(), "the samples are still that meter's");
        assert_eq!(app.replay_device_id(), Some("ut181a"));
    }

    /// The mock sends no frames, so its history offers no replay file.
    #[test]
    fn a_mock_history_has_no_replay_file() {
        let mut app = app("mock", false);
        deliver(&mut app, connected("mock", Stability::Verified, 0));
        deliver_readings(&mut app, &[DC]);
        assert_eq!(app.capture_layout.device_id, None);
        assert_eq!(app.replay_device_id(), None);
    }

    /// A file names one meter: another one answering starts another history,
    /// while the same one coming back continues it.
    #[test]
    fn another_meter_starts_another_history() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, DC]);

        deliver(
            &mut app,
            DmmMessage::Disconnected(dmm_lib::error::Error::Timeout),
        );
        deliver(&mut app, connected("ut61eplus", Stability::Verified, 0));
        deliver_readings(&mut app, &[DC]);
        assert_eq!(app.recording.samples.len(), 3, "the same meter came back");

        deliver(&mut app, connected("ut181a", Stability::Experimental, 4));
        deliver_readings(&mut app, &[DC]);
        assert_eq!(app.recording.samples.len(), 1);
        let ut181a = registry::find_device("ut181a").expect("a registry entry");
        assert_eq!(app.capture_layout.device, Some(ut181a.display_name));
    }

    /// A device picked in Settings only takes effect at the next connect;
    /// readings still queued are the old meter's and stay under its name.
    #[test]
    fn a_device_pick_does_not_relabel_queued_readings() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC]);
        app.settings.shared.device_family = "ut181a".to_string();
        deliver_readings(&mut app, &[DC]);

        assert_eq!(app.recording.samples.len(), 2);
        let ut61eplus = registry::find_device("ut61eplus").expect("a registry entry");
        assert_eq!(app.capture_layout.device, Some(ut61eplus.display_name));
    }

    /// With nothing recorded every reading is also kept for export — cut to
    /// the graph and dropping its oldest at the bound — so a reading must not
    /// cost more the longer the session has run. Measured over the whole
    /// drain: stats, graph and sample buffer.
    #[test]
    #[ignore = "timing-sensitive; run with --release"]
    fn a_reading_costs_the_same_however_long_the_history() {
        fn measure(points: u64) -> Duration {
            let mut app = connected_app();
            // Bound at exactly what the run holds, so both sizes push into a
            // full buffer, as the graph's own cost test does.
            app.graph.set_max_points(points as usize);
            app.recording.set_max_samples(points as usize);
            let t0 = Instant::now();
            let send = |app: &mut App, range: std::ops::Range<u64>| {
                let (tx, rx) = mpsc::channel();
                for i in range {
                    let value = MeasuredValue::Normal((i as f64 * 0.017).sin() * 10.0);
                    let at = t0 + Duration::from_millis(i * 10);
                    tx.send(DmmMessage::Measurement(reading("DC V", value, at)))
                        .expect("the channel is open");
                }
                app.connection.rx = Some(rx);
                let start = Instant::now();
                app.drain_messages();
                let elapsed = start.elapsed();
                drop(tx);
                elapsed
            };
            send(&mut app, 0..points);
            assert_eq!(app.recording.samples.len(), points as usize);
            let elapsed = send(&mut app, points..points + 1_000);
            assert_eq!(
                app.recording.samples.len(),
                points as usize,
                "still bounded"
            );
            elapsed
        }

        let short = measure(50_000);
        let long = measure(500_000);
        let ratio = long.as_secs_f64() / short.as_secs_f64().max(1e-9);
        println!("1K readings: 50K history {short:?}, 500K history {long:?}, ratio {ratio:.2}x");
        assert!(ratio < 2.0, "a reading costs more in a longer session");
    }
}
