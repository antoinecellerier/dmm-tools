//! The UI side of the acquisition channel: opening and closing it, draining
//! the messages the device thread sends, and classifying the reason there is
//! nothing to show into the help text the reading column renders.

use dmm_lib::binary_help::{ConnectedAdapters, connected_adapters};
use dmm_lib::mock::MockMode;
use eframe::egui::{self, RichText, Ui};
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use super::connection::{
    self, DmmMessage, ThreadContext, ThreadControl, handle_thread_panic, run_device_thread,
};
use super::plot_input::{PlotInput, resolve_plot_input};
use super::{App, ConnectionState};
use crate::graph::PlotSample;

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
    /// Anything else, as reported by the acquisition thread.
    Other(String),
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

impl App {
    pub(super) fn connect(&mut self, ctx: &egui::Context) {
        self.disconnect();

        let (msg_tx, msg_rx) = mpsc::channel();
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.connection.rx = Some(msg_rx);
        self.connection.ctrl_tx = Some(ctrl_tx);
        self.connection.stop_flag = Some(Arc::clone(&stop_flag));
        self.connection.cmd_tx = Some(cmd_tx);
        let ctx_clone = ctx.clone();
        let query_name = self.settings.query_device_name;
        let sample_interval_ms = self.settings.sample_interval_ms;
        let device_entry = self.selected_device();
        self.graph.set_sample_interval_ms(sample_interval_ms);

        if !device_entry.requires_hardware {
            let mock_mode: Option<MockMode> = if self.settings.mock_mode.is_empty() {
                None
            } else {
                self.settings.mock_mode.parse().ok()
            };
            // Mock returns instantly — enforce a floor to avoid busy-looping
            let mock_interval = sample_interval_ms.max(100);
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        move || match mock_mode {
                            Some(mode) => dmm_lib::mock::open_mock_mode(mode),
                            None => dmm_lib::mock::open_mock(),
                        },
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
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
            let device_id = device_entry.id;
            let adapter = self.settings.overrides.adapter.clone();
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        move || dmm_lib::open_device_by_id_auto(device_id, adapter.as_deref()),
                        ThreadContext {
                            msg_tx,
                            ctrl_rx,
                            cmd_rx,
                            ctx: ctx_clone,
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
        self.connection.experimental = false;
        self.connection.feedback_url.clear();
        self.connection.supported_commands.clear();
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
                    experimental: exp,
                    feedback_url,
                    supported_commands: cmds,
                    max_aux_values,
                } => {
                    self.connection.state = ConnectionState::Connected;
                    self.connection.experimental = exp;
                    self.capture_layout.device_aux_slots = max_aux_values;
                    // A reconnect mid-recording is the same meter, so the
                    // in-flight capture picks the slot count back up — it was
                    // 0 before the first Connected of the session.
                    if self.recording.active {
                        self.capture_layout.aux_slots = max_aux_values;
                    }
                    self.connection.feedback_url = feedback_url;
                    self.connection.supported_commands = cmds;
                    self.connection.device_name = if name.is_empty() {
                        None
                    } else {
                        Some(name.clone())
                    };
                    self.connection.last_error = None;
                    self.connection.reconnect_attempt = 0;
                    self.connection.reconnect_last_error = None;
                    info!("UI: connected to {name}");
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
                        self.toast = Some((
                            "Recording stopped \u{2014} buffer full (500K samples)".to_string(),
                            true,
                            Instant::now(),
                        ));
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

    pub(super) fn show_connection_help(&self, ui: &mut Ui) {
        let warn_color = self
            .settings
            .theme_colors(ui.visuals().dark_mode)
            .status_warning();

        // Show waiting indicator before error threshold
        if self.connection.waiting_timeouts > 0 && self.connection.last_error.is_none() {
            ui.add_space(4.0);
            let dots = ".".repeat((self.connection.waiting_timeouts as usize % 4) + 1);
            ui.label(RichText::new(format!("Waiting for meter{dots}")).color(warn_color));
            ui.label(
                RichText::new("Check that the correct device is selected in Settings (\u{2699})")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        let Some(issue) = &self.connection.last_error else {
            return;
        };

        ui.add_space(8.0);

        if *issue == ConnectionIssue::DeviceNotFound {
            // HID device not found — dongle issue
            ui.label(RichText::new("USB cable not found").color(warn_color));
            let platform_hint = if cfg!(target_os = "linux") {
                "Check that the USB cable is plugged in and the meter is on.\n\
                 On Linux, ensure the udev rule is installed:\n\
                 sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/\n\
                 sudo udevadm control --reload-rules\n\
                 Your user must be in the plugdev group:\n\
                 sudo usermod -aG plugdev $USER\n\
                 Then log out/in and replug the cable.\n\n\
                 Click \"Connect\" after resolving the issue."
            } else if cfg!(target_os = "windows") {
                "Check that the USB cable is plugged in and the meter is on.\n\
                 Open Device Manager:\n\
                 \u{2022} 'CP2110 USB to UART Bridge' under HID devices: OK\n\
                 \u{2022} 'USB Input Device' under HID devices: OK\n\
                 \u{2022} Yellow icon under 'Other devices': install driver from\n\
                   silabs.com/developers/usb-to-uart-bridge-vcp-drivers\n\n\
                 Click \"Connect\" after resolving the issue."
            } else if cfg!(target_os = "macos") {
                "Check that the USB cable is plugged in and the meter is on.\n\
                 The cable should be recognized automatically (no driver needed).\n\
                 If not found, check System Settings > Privacy & Security > Input Monitoring.\n\n\
                 Click \"Connect\" after resolving the issue."
            } else {
                "Check that the USB cable is plugged in and the meter is on.\n\n\
                 Click \"Connect\" after resolving the issue."
            };
            ui.label(
                RichText::new(platform_hint)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            let profile = &self.selected_profile;
            if profile.stability == dmm_lib::protocol::Stability::Experimental {
                ui.hyperlink_to(
                    RichText::new(format!(
                        "{} support is experimental \u{2014} report feedback",
                        profile.model_name
                    ))
                    .small()
                    .color(warn_color),
                    profile.feedback_url(),
                )
                .on_hover_text(
                    "Opens the GitHub issue tracker to report experimental-support feedback",
                );
            }
        } else if let ConnectionIssue::AdapterNotFound { help } = issue {
            ui.label(RichText::new("Adapter not found").color(warn_color));
            ui.label(
                RichText::new(help)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        } else {
            // Dongle found but meter not responding
            ui.label(RichText::new("No response from meter").color(warn_color));
            let device_entry = self.selected_device();
            let instructions = format!(
                "The USB adapter is connected but the meter \n\
                 isn't responding ({} selected).\n\
                 \n\
                 If this is the wrong device, change it in Settings (\u{2699}).\n\
                 Otherwise, enable data transmission:\n\
                 {}",
                device_entry.display_name, device_entry.activation_instructions
            );
            ui.label(
                RichText::new(instructions)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
