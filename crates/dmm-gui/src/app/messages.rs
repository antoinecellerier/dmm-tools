//! The UI side of the acquisition channel: opening and closing it, draining
//! the messages the device thread sends, and drawing the connection help the
//! reading column shows when there is nothing to read.

use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use eframe::egui::{self, RichText, Ui};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::SystemTime;

use super::connection::{
    self, DmmMessage, RECONNECT_INTERVAL, RemoteCommand, ThreadContext, spawn_acquisition,
};
use super::connection_issue::{ConnectionIssue, NoticeKind};
use super::toast::{ERROR_GLYPH, Toast, dismiss_button};
use super::{App, ConnectionState, named_device};
use crate::settings::format_sample_count;

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
        self.toast = Some(Toast::info(detected_toast(
            device.display_name,
            reported,
            to_save.is_some(),
        )));
    }

    /// Say that lowering Buffer size ended the capture that was running.
    ///
    /// Deliberately not the full-buffer wording: that names the new bound,
    /// which here is *smaller* than what the buffer holds — a capture keeps
    /// every sample it took, so quoting the bound would read as if the
    /// difference had been thrown away.
    pub(super) fn buffer_shrunk_toast(&mut self) {
        let kept = format_sample_count(self.capture.recording.recording_samples().len());
        self.toast = Some(Toast::error(format!(
            "Recording stopped \u{2014} its {kept} samples are kept, Export\u{2026} saves them"
        )));
    }

    /// Say that the recording stopped because it filled the buffer, at the
    /// size the user configured.
    pub(super) fn buffer_full_toast(&mut self) {
        self.toast = Some(Toast::error(format!(
            "Recording stopped \u{2014} buffer full ({} samples)",
            format_sample_count(self.settings.max_samples)
        )));
    }

    /// The interval the graph judges its gaps by for a sample interval of
    /// `ms`: an imported file's or a replay's readings come no more often
    /// than the file has them, and judged by the viewer's interval alone a
    /// recording spaced wider than it broke at every point.
    pub(super) fn gap_interval_ms(&self, ms: u32) -> u32 {
        let file = self.import_cadence_ms.or_else(|| {
            self.replay.as_ref().map(|source| {
                u32::try_from(source.replay.cadence().as_millis()).unwrap_or(u32::MAX)
            })
        });
        file.map_or(ms, |cadence| ms.max(cadence))
    }

    /// Tell the graph and the session integral how far apart readings come
    /// at a sample interval of `ms`, so both judge a gap alike.
    pub(super) fn set_gap_interval(&mut self, ms: u32) {
        let gap_ms = self.gap_interval_ms(ms);
        self.graph.set_sample_interval_ms(gap_ms);
        self.capture
            .session
            .integrator
            .set_sample_interval(std::time::Duration::from_millis(u64::from(gap_ms)));
    }

    /// Readings stopped (a pause, a lost link, a meter gone quiet): the graph
    /// marks the gap, and the session integral does not bridge it.
    pub(super) fn mark_data_loss(&mut self) {
        self.graph.push_data_loss();
        self.capture.session.integrator.push_gap();
    }

    /// Put the **Sample interval** in Settings into effect: the graph's gap
    /// threshold, and a running session's acquisition, at once.
    pub(super) fn apply_sample_interval(&mut self) {
        let ms = self.settings.sample_interval_ms;
        self.set_gap_interval(ms);
        if let Some(tx) = &self.connection.ctrl_tx {
            let _ = tx.send(connection::ThreadControl::SetInterval(ms));
        }
    }

    /// Make this Connect the session's zero, the first time a recording is
    /// opened.
    ///
    /// Playback measures every frame's offset from the origin, so pinning it
    /// while the arguments were parsed dropped whatever fell due while the
    /// window and the GPU were starting — and with auto-connect off, a Connect
    /// past the recording's length found it already ended. A later
    /// Disconnect/Connect keeps the origin, so the recording resumes where the
    /// session has got to.
    ///
    /// `true` when this call pinned it: the recording starts from the top.
    fn pin_replay_origin(&mut self, recorded: SystemTime) -> bool {
        if self.clock.wall_origin().is_some() {
            return false;
        }
        self.clock = self.clock.clone().with_wall_origin(recorded);
        true
    }

    pub(super) fn connect(&mut self, ctx: &egui::Context) {
        self.disconnect();
        // An imported session's readings are the file's, stamped on a time
        // base of their own: a meter's never join them.
        if self.imported.is_some() || self.import_job.is_some() {
            self.reset_session_for_import();
        }

        // Unbounded: `drain_messages` empties the message channel every
        // frame, so it only ever holds what arrived since the last one.
        let (msg_tx, msg_rx) = mpsc::channel();
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<RemoteCommand>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.connection.rx = Some(msg_rx);
        self.connection.ctrl_tx = Some(ctrl_tx);
        self.connection.stop_flag = Some(Arc::clone(&stop_flag));
        self.connection.cmd_tx = Some(cmd_tx);
        let sample_interval_ms = self.settings.sample_interval_ms;
        // `None` = Auto-detect: nothing names the meter, so the opener works
        // it out from the bytes it sends.
        let device_entry = self.selected_device();
        self.set_gap_interval(sample_interval_ms);
        let mut thread_ctx = ThreadContext {
            msg_tx,
            ctrl_rx,
            cmd_rx,
            ctx: ctx.clone(),
            selected: device_entry,
            query_name: self.settings.query_device_name,
            sample_interval_ms,
            simulated: false,
            reconnect_interval: RECONNECT_INTERVAL,
            stop_flag,
        };

        let source = self
            .replay
            .as_ref()
            .map(|source| (Arc::clone(&source.replay), source.recorded));
        if let Some((replay, recorded)) = source {
            // Queued at the first Connect, and again after an import cleared
            // the session; a plain Disconnect/Connect resumes the playback,
            // and the markers already played are on their readings.
            if self.pin_replay_origin(recorded) || std::mem::take(&mut self.requeue_replay) {
                let mut markers = replay.markers.clone();
                markers.sort_by_key(|m| m.offset);
                self.markers
                    .reserve(markers.iter().map(|m| m.number).max().unwrap_or(0));
                self.replay_markers = markers.into();
                self.replay_view = replay
                    .view
                    .as_deref()
                    .and_then(super::import::parse_view)
                    .map(|view| {
                        let reach = super::import::view_reach(&view);
                        (view, reach)
                    });
            }
            self.drop_replay_markers_passed(replay.cadence());
            // The file says which meter its frames came from, so that entry is
            // reported rather than whatever the Settings row currently names.
            // No interval floor: the protocol sleeps until each frame is due,
            // and the Sample interval keeps one frame per tick of the file's
            // as it does a live meter's.
            thread_ctx.selected = Some(replay.device);
            let clock = self.clock.clone();
            spawn_acquisition(
                // A replay cannot fail once it is open, so the retry loop
                // never re-runs this; a manual Disconnect then Connect does.
                // The origin the first Connect pinned stands for the rest of
                // the session, so re-opening picks the recording up where the
                // session has got to instead of starting it again. Nothing to
                // detect — the file names it.
                move |_| replay.open(clock.clone()).map(|dmm| (dmm, None)),
                thread_ctx,
            );
        } else if let Some(device) = device_entry.filter(|d| !d.requires_hardware) {
            let mock_mode: Option<MockMode> = if self.settings.shared.mock_mode.is_empty() {
                None
            } else {
                match self.settings.shared.mock_mode.parse() {
                    Ok(mode) => Some(mode),
                    // Only a hand-edited settings file reaches this: clap
                    // rejects a bad `--mock-mode` and the Settings row writes
                    // labels. Auto-cycling silently looked like the pin was
                    // ignored, so say so — the toast takes the first line and
                    // the log the mode list that follows it.
                    Err(message) => {
                        warn!("{message}");
                        let headline = message.lines().next().unwrap_or_default().to_string();
                        self.toast = Some(Toast::error(headline));
                        None
                    }
                }
            };
            thread_ctx.simulated = true;
            let clock = self.clock.clone();
            spawn_acquisition(
                // Cloned inside: this closure is re-run on every reconnect,
                // and the session clock outlives each `Dmm` it opens. Nothing
                // to detect — the mock is what it says it is.
                move |_| {
                    dmm_lib::mock::open_simulated(device, mock_mode, clock.clone())
                        .map(|dmm| (dmm, None))
                },
                thread_ctx,
            );
        } else {
            let device_id = device_entry.map(|d| d.id);
            let adapter = self.settings.overrides.adapter.clone();
            let bluetooth = self.settings.shared.bluetooth;
            spawn_acquisition(
                // Re-run on every reconnect, detection included: a meter that
                // comes back is identified again rather than assumed to be the
                // one that left. `reopen_at` is the Bluetooth adapter a lost
                // link was on, opened by address like `--adapter`, which wins
                // if given.
                move |reopen_at| {
                    let opts = dmm_lib::OpenOptions {
                        adapter: adapter.as_deref().or(reopen_at),
                        bluetooth,
                    };
                    match device_id {
                        Some(id) => {
                            dmm_lib::open_device_by_id_auto(id, opts).map(|dmm| (dmm, None))
                        }
                        None => {
                            dmm_lib::open_auto(opts).map(|(dmm, detected)| (dmm, Some(detected)))
                        }
                    }
                },
                thread_ctx,
            );
        }
    }

    pub(super) fn disconnect(&mut self) {
        // Data stops here. The graph keeps its history across a reconnect, so
        // the resulting hole needs marking as a genuine gap.
        self.mark_data_loss();
        // Raise the flag, then hang up: the thread may be mid-sleep, and the
        // flag is what cuts that short; dropping the control sender ends any
        // wait on the channel and the loop itself.
        if let Some(flag) = self.connection.stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        self.connection.ctrl_tx = None;
        self.connection.rx = None;
        self.connection.cmd_tx = None;
        self.connection.state = ConnectionState::Disconnected;
        // Nothing is connected, so nothing is identified: under Auto-detect
        // the next connect asks the cable again.
        self.connection.meter = None;
        self.connection.choices.clear();
        self.connection.paused = false;
        self.connection.reconnect_attempt = 0;
        self.connection.reconnect_last_error = None;
        // Otherwise only an incoming measurement clears this, and there won't
        // be one — a meter that went quiet before the user disconnected left
        // "Waiting for meter…" on screen for the whole disconnected session.
        self.connection.waiting_timeouts = 0;
        self.connection.ended = false;
        self.connection.late_readings = None;
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
        self.capture.recording.clear_history();
        self.capture.session.reset();
        if let Some(alarm) = &mut self.alarm {
            alarm.clear_counts();
        }
        self.last_measurement = None;
        self.held.clear();
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
        let mut breaches = Vec::new();
        for msg in messages {
            match msg {
                DmmMessage::Connected(meter) => {
                    self.connection.state = ConnectionState::Connected;
                    self.held.clear();
                    // A new link: it says again if its readings come in pairs.
                    self.connection.late_readings = None;
                    self.capture.device_aux_slots = meter.max_aux_values;
                    // A reconnect mid-recording is the same meter, so the
                    // in-flight capture picks the slot count back up — it was
                    // 0 before the first Connected of the session.
                    if self.capture.recording.active {
                        self.capture.recording_layout.aux_slots = meter.max_aux_values;
                        // The meter Record was pressed ahead of: the first one
                        // to answer during this recording is the one its
                        // samples come from, and the only one the export can
                        // still name once the cable is out.
                        self.capture
                            .recording_layout
                            .experimental
                            .get_or_insert(!meter.stability.is_verified());
                    }
                    // A reconnect may find the dial elsewhere; the thread
                    // re-lists the choices with its first reading.
                    self.connection.choices.clear();
                    let device_id = meter.device.map(|d| d.id);
                    let reported = meter.name.clone().unwrap_or_default();
                    self.connection.meter = Some(meter);
                    // Under Auto-detect the meter that just named itself
                    // becomes the saved device, so the next session opens it
                    // pinned instead of probing the cable again.
                    self.remember_detected_device(device_id, &reported);
                    self.connection.last_error = None;
                    self.connection.reconnect_attempt = 0;
                    self.connection.reconnect_last_error = None;
                    if let Some(meter) = &self.connection.meter {
                        info!(
                            "UI: connected to {} (meter reports {:?})",
                            meter.model_name, meter.name
                        );
                    }
                }
                DmmMessage::Ended => {
                    self.connection.ended = true;
                }
                DmmMessage::WaitingForMeter(count) => {
                    self.connection.waiting_timeouts = count;
                    // A timeout never raises Disconnected — the bridge is
                    // still enumerated, the meter just isn't answering (auto
                    // power-off, or unplugged at the meter end). Without this
                    // an outage during an overload would be drawn as one long
                    // band, claiming over-range for a stretch nothing was
                    // heard in. Same threshold the "no response" notice uses.
                    // No reading for a read timeout: nothing to integrate
                    // across, whatever the interval.
                    self.capture.session.integrator.push_gap();
                    if count >= dmm_lib::stream::NO_RESPONSE_TIMEOUTS {
                        self.mark_data_loss();
                    }
                    // Crossing the threshold is a failure, recorded once. A
                    // gap in a recording plays back as timeouts and reaches
                    // it too. It is not a quiet meter: there is no device
                    // selection to check, no USB mode to switch on, and the
                    // file carries on by itself once the gap is over.
                    if count == dmm_lib::stream::NO_RESPONSE_TIMEOUTS && self.replay.is_none() {
                        error!("UI: error: {}", connection::NO_RESPONSE);
                        self.connection.last_error =
                            Some(ConnectionIssue::Other(connection::NO_RESPONSE.to_string()));
                        if self.connection.state == ConnectionState::Disconnected {
                            clear_channel = true;
                        }
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

                    let (m, filled) =
                        self.capture
                            .ingest(m, &self.transform, &mut self.graph, &self.connection);
                    if filled {
                        self.buffer_full_toast();
                    }

                    // Specs are attached to each measurement by `Dmm::request_measurement`;
                    // last_measurement.spec / .mode_spec is what render code reads.
                    // Filled in from the frames before it when the meter sends
                    // a reading's parts in frames of their own.
                    self.place_replay_marker(&m);
                    // After the file's marker: a breach on the same reading
                    // adds its note to that marker rather than pushing it on.
                    breaches.extend(self.check_alarm(&m, self.alarm_scaled()));
                    self.apply_replay_view(&m);
                    let shown = self.held.fill_in(self.last_measurement.as_ref(), m);
                    self.last_measurement = Some(shown);
                }
                DmmMessage::Disconnected(err) => {
                    info!("UI: disconnected: {err} ({:?})", err.kind());
                    self.connection.state = ConnectionState::Reconnecting;
                    self.held.clear();
                    // Its command names a link that is gone.
                    self.connection.late_readings = None;
                    // Tell the graph this was a real loss of data. It can't
                    // infer that from timestamps — the meter goes quiet for
                    // over a second while auto-ranging, which looks the same
                    // as an unplugged cable.
                    self.mark_data_loss();
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
                DmmMessage::CommandFailed(msg) => {
                    self.toast = Some(Toast::error(msg));
                }
                DmmMessage::Choices(setting, choices) => {
                    self.connection.choices.set(setting, choices);
                }
                DmmMessage::LateReadings(notice) => {
                    self.connection.late_readings = Some(notice);
                }
            }
        }

        self.mark_breaches(breaches);
        self.sync_alarm_view();

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

    /// Draw the notice for readings arriving two at a time under the
    /// reading, until dismissed: the title, what happens and, where there
    /// is one, the command that fixes the link, with a button to copy it.
    ///
    /// Not drawn in the big-meter modes, as the connection help is not.
    pub(super) fn show_late_readings(&mut self, ui: &mut Ui) {
        let Some(notice) = &self.connection.late_readings else {
            return;
        };
        let warn_color = self
            .settings
            .theme_colors(ui.visuals().dark_mode)
            .status_warning();
        let mut dismissed = false;
        let mut copied = false;
        ui.add_space(8.0);
        // The close button goes in first, from the right, so the title wraps
        // in what is left, as in a toast; inset by a scrollbar's width, as
        // the column's bar floats over its right edge when it scrolls.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            let scroll = &ui.spacing().scroll;
            ui.add_space(scroll.bar_width + scroll.bar_outer_margin);
            dismissed = dismiss_button(ui);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "{ERROR_GLYPH} {}",
                            dmm_lib::transport::LateReadings::TITLE
                        ))
                        .color(warn_color),
                    )
                    .wrap(),
                );
            });
        });
        ui.add(egui::Label::new(RichText::new(notice.advice()).small()).wrap());
        if let Some(command) = &notice.command {
            ui.add_space(4.0);
            // Copy follows the command's last word, on that line when there
            // is room for it.
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(command)
                            .small()
                            .family(egui::FontFamily::Monospace),
                    )
                    .wrap()
                    .selectable(true),
                );
                copied = ui.small_button("Copy").clicked();
            });
            if copied {
                ui.ctx().copy_text(command.clone());
            }
        }
        if dismissed {
            self.connection.late_readings = None;
        }
        if copied {
            self.toast = Some(Toast::info("Command copied"));
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
            NoticeKind::Detecting | NoticeKind::Waiting | NoticeKind::Ended => 4.0,
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
    use crate::app::connection::ConnectedMeter;
    use crate::settings::Settings;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::{MeasuredValue, Measurement};
    use dmm_lib::protocol::Stability;
    use std::time::{Duration, Instant};

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
        app.toast.as_ref().map(|t| t.message.as_str())
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

    /// The notice for readings arriving in pairs stays until the link goes:
    /// its command names that link, and a new link says so again itself.
    #[test]
    fn the_late_readings_notice_lasts_as_long_as_its_link() {
        let notice = || dmm_lib::transport::LateReadings {
            command: Some("sudo hcitool lecup --handle 2048".to_string()),
        };
        let mut app = app("ut61eplus", false);
        deliver(&mut app, DmmMessage::LateReadings(notice()));
        assert_eq!(app.connection.late_readings, Some(notice()));
        deliver(
            &mut app,
            DmmMessage::Disconnected(dmm_lib::error::Error::LinkLost),
        );
        assert_eq!(app.connection.late_readings, None, "the link went");

        deliver(&mut app, DmmMessage::LateReadings(notice()));
        app.disconnect();
        assert_eq!(app.connection.late_readings, None, "disconnected");
    }

    /// A Sample interval picked while connected goes to the running session
    /// at once, with no reconnect.
    #[test]
    fn a_new_sample_interval_reaches_the_session() {
        let mut app = app("ut61eplus", false);
        let (tx, rx) = mpsc::channel();
        app.connection.ctrl_tx = Some(tx);
        app.settings.sample_interval_ms = 1000;
        app.apply_sample_interval();
        assert!(matches!(
            rx.try_recv(),
            Ok(connection::ThreadControl::SetInterval(1000))
        ));
    }

    /// A replay that played to its end says so, and keeps its last reading;
    /// a Disconnect clears it for the next Connect.
    #[test]
    fn a_replay_that_ended_says_so() {
        let mut app = app("ut61eplus", false);
        app.replay = Some(crate::ReplaySource::fixture());
        let reading = Measurement::test_fixture(
            MeasuredValue::Normal(1.0),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        app.last_measurement = Some(reading);
        deliver(&mut app, DmmMessage::Ended);
        let n = app.connection_notice().expect("a notice");
        assert_eq!(n.kind, NoticeKind::Ended);
        assert_eq!(n.title, "The recording has ended");
        assert!(app.last_measurement.is_some(), "the last reading stays");
        app.disconnect();
        assert!(app.connection_notice().is_none());
    }

    /// A gap in a recording plays back as timeouts, and they are not a quiet
    /// meter: nothing is on the cable to select and no USB mode to switch on.
    /// The gap says what it is, and the help stays away.
    #[test]
    fn a_replay_gap_is_not_a_quiet_meter() {
        let mut app = app("ut61eplus", false);
        app.replay = Some(crate::ReplaySource::fixture());
        deliver(
            &mut app,
            DmmMessage::WaitingForMeter(dmm_lib::stream::NO_RESPONSE_TIMEOUTS),
        );

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
        deliver(
            &mut app,
            DmmMessage::WaitingForMeter(dmm_lib::stream::NO_RESPONSE_TIMEOUTS),
        );

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
        // The wall time of a moment with no reading, a marker's, maps from it too.
        assert_eq!(app.clock.wall_time_for(origin), recorded);

        // A Disconnect/Connect keeps the origin, so the recording resumes
        // where the session has got to rather than starting again.
        app.disconnect();
        app.connect(&egui::Context::default());
        assert_eq!(app.clock.wall_origin().expect("still pinned").0, origin);
        app.disconnect();
    }

    /// A replay's markers go back on their readings as playback reaches
    /// them, under their own numbers: each on the first reading played at or
    /// after its offset, one per reading, so a marker whose frame never plays
    /// lands on the next one. The first Connect queues them; a later one
    /// resumes the playback and queues nothing again.
    #[test]
    fn a_replays_markers_go_on_their_readings() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let mut source = crate::ReplaySource::fixture();
        source.replay = Arc::new(
            dmm_lib::replay::Replay::parse(
                "# dmm-replay 1\n\
                 # device: ut61eplus\n\
                 # recorded: 2026-09-02T10:00:00Z\n\
                 0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n\
                 # marker: 0 3 probes on\n\
                 # marker: 150 5 skipped\n\
                 # marker: 200 6 \n",
            )
            .expect("a well-formed recording"),
        );
        app.replay = Some(source);
        let ctx = egui::Context::default();
        app.connect(&ctx);
        app.disconnect();
        assert_eq!(app.replay_markers.len(), 3);

        let (start, _) = app.clock.wall_origin().expect("the Connect pins it");
        let play = |app: &mut App, ms| {
            let mut m = Measurement::test_fixture(
                MeasuredValue::Normal(1.0),
                "V",
                dmm_lib::flags::StatusFlags::default(),
            );
            m.timestamp = start + Duration::from_millis(ms);
            deliver(app, DmmMessage::Measurement(m));
        };
        for ms in [0, 100, 250, 300] {
            play(&mut app, ms);
        }
        let placed: Vec<(u128, u32, &str)> = app
            .markers
            .iter()
            .map(|m| {
                (
                    m.at.duration_since(start).as_millis(),
                    m.number,
                    m.note.as_str(),
                )
            })
            .collect();
        assert_eq!(
            placed,
            [(0, 3, "probes on"), (250, 5, "skipped"), (300, 6, "")]
        );

        app.connect(&ctx);
        app.disconnect();
        assert!(app.replay_markers.is_empty(), "queued once per session");
        // A marker placed by hand numbers on past the file's.
        play(&mut app, 400);
        app.add_marker(false);
        assert_eq!(app.markers.iter().last().map(|m| m.number), Some(7));
    }

    /// A Connect that resumes a playback drops the markers the playback has
    /// passed, rather than piling them onto its next readings; after an
    /// import cleared the session, the next Connect queues the recording's
    /// markers and view again, less the ones passed.
    #[test]
    fn a_resumed_replay_drops_the_markers_it_passed() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::manual());
        let mut source = crate::ReplaySource::fixture();
        source.replay = Arc::new(
            dmm_lib::replay::Replay::parse(
                "# dmm-replay 1\n\
                 # device: ut61eplus\n\
                 # recorded: 2026-09-02T10:00:00Z\n\
                 # view: {\"mean\":true}\n\
                 0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n\
                 # marker: 1000 1 early\n\
                 # marker: 90000 2 passed\n\
                 # marker: 120000 3 ahead\n",
            )
            .expect("a well-formed recording"),
        );
        app.replay = Some(source);
        let ctx = egui::Context::default();
        app.connect(&ctx);
        app.disconnect();
        assert_eq!(app.replay_markers.len(), 3);

        app.clock.sleep(Duration::from_secs(100));
        app.connect(&ctx);
        app.disconnect();
        let left: Vec<u32> = app.replay_markers.iter().map(|m| m.number).collect();
        assert_eq!(left, [3], "the playback resumed past the first two");

        app.reset_session_for_import();
        assert!(app.replay_markers.is_empty() && app.replay_view.is_none());
        app.connect(&ctx);
        app.disconnect();
        let left: Vec<u32> = app.replay_markers.iter().map(|m| m.number).collect();
        assert_eq!(left, [3], "queued again, less the ones passed");
        assert!(app.replay_view.is_some(), "the view is queued again");
    }

    /// A replay's saved view waits until the playback reaches the furthest
    /// moment it shows, then goes in once.
    #[test]
    fn a_replays_view_waits_for_its_readings() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let mut source = crate::ReplaySource::fixture();
        source.replay = Arc::new(
            dmm_lib::replay::Replay::parse(
                "# dmm-replay 1\n\
                 # device: ut61eplus\n\
                 # recorded: 2026-09-02T10:00:00Z\n\
                 # view: {\"mean\":true,\"cursors\":{\"a\":0.1}}\n\
                 0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n",
            )
            .expect("a well-formed recording"),
        );
        app.replay = Some(source);
        let ctx = egui::Context::default();
        app.connect(&ctx);
        app.disconnect();
        let (start, _) = app.clock.wall_origin().expect("the Connect pins it");
        let play = |app: &mut App, ms| {
            let mut m = Measurement::test_fixture(
                MeasuredValue::Normal(1.0),
                "V",
                dmm_lib::flags::StatusFlags::default(),
            );
            m.timestamp = start + Duration::from_millis(ms);
            deliver(app, DmmMessage::Measurement(m));
        };
        play(&mut app, 0);
        assert!(!app.graph.show_mean, "the cursor's reading has not played");
        play(&mut app, 100);
        assert!(app.graph.show_mean);
        assert!(app.graph.cursors_active);
        assert!(app.replay_view.is_none(), "applied once");
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
        DmmMessage::Connected(ConnectedMeter {
            stability,
            max_aux_values,
            ..ConnectedMeter::test_fixture(Some(device_id))
        })
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
        app.capture
            .recording
            .export_samples()
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
        assert_eq!(app.capture.recording.export_samples().len(), 3);
        assert_eq!(
            app.capture
                .recording
                .export_samples()
                .nth(1)
                .expect("a sample")
                .measurement
                .value_export_str(),
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
        assert_eq!(app.capture.session.stats.count, 2);
        assert_eq!(
            app.capture
                .recording
                .export_samples()
                .nth(1)
                .expect("a sample")
                .measurement
                .value_export_str(),
            ""
        );
    }

    /// A UT61E+ in AC+DC V sends its DC and AC components in turn, the AC
    /// frames without a main reading. Every frame is kept, the AC component
    /// becomes a trace beside the DC one, and the statistics follow DC alone.
    #[test]
    fn alternating_component_frames_keep_both_and_plot_ac_beside_dc() {
        let mut app = connected_app();
        let t0 = Instant::now();
        for i in 0..4u64 {
            let at = t0 + std::time::Duration::from_millis(i * 667);
            let m = if i % 2 == 0 {
                reading("AC+DC V", MeasuredValue::Normal(1.6112), at)
            } else {
                let mut ac = reading("AC+DC V", MeasuredValue::Absent, at);
                ac.display_raw = None;
                ac.aux_values = vec![dmm_lib::measurement::AuxValue {
                    label: "AC".into(),
                    value: MeasuredValue::Normal(0.0),
                    unit: "".into(),
                    display_raw: Some(" 0.0000".to_string()),
                    elapsed_secs: None,
                }];
                ac
            };
            deliver(&mut app, DmmMessage::Measurement(m));
        }
        assert_eq!(modes(&app), ["AC+DC V"; 4]);
        assert_eq!(app.graph.overlays_len(), 1, "AC drawn beside DC");
        assert_eq!(app.graph.first_point_time(), Some(t0));
        assert_eq!(app.capture.session.stats.count, 2);
        assert_eq!(app.capture.session.stats.min, Some(1.6112));
    }

    /// A UT181A in V AC + Hz: picking Frequency under **Plot:** keeps the
    /// voltage's past and plots the frequency's, an over-range frequency on
    /// the switching frame included, and the graph gives the voltage back
    /// with its past once the meter stops sending the frequency.
    #[test]
    fn switching_plot_to_another_unit_keeps_both_series() {
        use crate::graph::GapKind;
        let mut app = app("ut181a", false);
        deliver(&mut app, connected("ut181a", Stability::PartlyVerified, 4));
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let frame = |ms: u64, hz: Option<MeasuredValue>| {
            let mut m = reading("V AC Hz", MeasuredValue::Normal(230.0), at(ms));
            if let Some(hz) = hz {
                m.aux_values = ["Frequency", "Period"]
                    .into_iter()
                    .zip([(hz, "Hz"), (MeasuredValue::Normal(20.0), "ms")])
                    .map(|(label, (value, unit))| dmm_lib::measurement::AuxValue {
                        label: label.into(),
                        value,
                        unit: unit.into(),
                        display_raw: None,
                        elapsed_secs: None,
                    })
                    .collect();
            }
            DmmMessage::Measurement(m)
        };
        for i in 0..3 {
            deliver(&mut app, frame(i * 100, Some(MeasuredValue::Normal(50.0))));
        }

        app.graph.select_series(Some("Frequency"));
        deliver(&mut app, frame(300, Some(MeasuredValue::Overload)));
        deliver(&mut app, frame(400, Some(MeasuredValue::Normal(50.1))));
        assert_eq!(app.graph.plotted_unit(), "Hz");
        assert_eq!(app.graph.len(), 4, "the kept frequency and the new point");
        assert_eq!(app.graph.overlay_values("Main").len(), 5);
        assert_eq!(
            app.graph.visible_gaps(),
            vec![(0.2, 0.4, GapKind::Overload)],
            "the over-range frame breaks the frequency, not the voltage"
        );

        // The meter stops sending the frequency: once the graph gives the
        // selection up, the voltage comes back with its past.
        for i in 5..=20 {
            deliver(&mut app, frame(i * 100, None));
        }
        assert_eq!(app.graph.plotted_unit(), "V");
        assert!(app.graph.len() > 5, "got {}", app.graph.len());
        assert_eq!(app.graph.first_point_time(), Some(t0));
    }

    /// Turning the dial to NCV restarts the graph on the levels, as any
    /// other mode does — the amps before it are not left on screen — and the
    /// history with it.
    #[test]
    fn turning_to_ncv_restarts_the_graph_on_its_levels() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, DC]);
        assert_eq!(app.graph.decimals(true), 4);
        let ncv = |l| ("NCV", MeasuredValue::NcvLevel(l));
        deliver_readings(&mut app, &[ncv(0), ncv(2), ncv(3)]);
        assert_eq!(app.graph.plotted_mode(), Some("NCV"));
        assert_eq!(app.graph.len(), 3);
        assert_eq!(app.graph.decimals(true), 0, "written as the level it is");
        assert_eq!(modes(&app), ["NCV"; 3]);
    }

    /// Clear drops the history with the graph, and never a recording.
    #[test]
    fn clear_drops_the_history_but_not_a_recording() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC, DC]);
        app.clear_session();
        assert!((app.capture.recording.export_samples().len() == 0));

        app.toggle_recording();
        deliver_readings(&mut app, &[DC, DC]);
        app.clear_session();
        assert_eq!(app.capture.recording.export_samples().len(), 2);
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
        assert!((app.capture.recording.export_samples().len() == 0));
    }

    /// The history's file names the meter it came from, as a recording's
    /// does, and keeps naming it once the cable is out.
    #[test]
    fn the_history_names_the_meter_it_came_from() {
        let mut app = app("ut181a", false);
        deliver(&mut app, connected("ut181a", Stability::Experimental, 4));
        deliver_readings(&mut app, &[DC]);

        let ut181a = registry::find_device("ut181a").expect("a registry entry");
        assert_eq!(
            app.capture.history_layout.device.as_deref(),
            Some(ut181a.display_name)
        );
        assert_eq!(app.capture.history_layout.device_id, Some("ut181a"));
        assert_eq!(app.capture.history_layout.experimental, Some(true));
        assert_eq!(app.capture.history_layout.aux_slots, 4);

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
        assert_eq!(app.capture.history_layout.device_id, None);
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
        assert_eq!(
            app.capture.recording.export_samples().len(),
            3,
            "the same meter came back"
        );

        deliver(&mut app, connected("ut181a", Stability::Experimental, 4));
        deliver_readings(&mut app, &[DC]);
        assert_eq!(app.capture.recording.export_samples().len(), 1);
        let ut181a = registry::find_device("ut181a").expect("a registry entry");
        assert_eq!(
            app.capture.history_layout.device.as_deref(),
            Some(ut181a.display_name)
        );
    }

    /// Another meter answering mid-recording restarts the history under its
    /// name; the recording carries on under the one it started with.
    #[test]
    fn a_new_meter_mid_recording_restarts_only_the_history() {
        let mut app = connected_app();
        app.toggle_recording();
        deliver_readings(&mut app, &[DC]);
        deliver(&mut app, connected("ut181a", Stability::Experimental, 4));
        deliver_readings(&mut app, &[DC]);

        let ut61eplus = registry::find_device("ut61eplus").expect("a registry entry");
        let ut181a = registry::find_device("ut181a").expect("a registry entry");
        assert_eq!(
            app.capture.recording_layout.device.as_deref(),
            Some(ut61eplus.display_name)
        );
        assert_eq!(
            app.capture.history_layout.device.as_deref(),
            Some(ut181a.display_name)
        );
        assert_eq!(app.capture.recording.recording_samples().len(), 2);
        assert_eq!(app.capture.recording.history_samples().len(), 1);
    }

    /// A device picked in Settings only takes effect at the next connect;
    /// readings still queued are the old meter's and stay under its name.
    #[test]
    fn a_device_pick_does_not_relabel_queued_readings() {
        let mut app = connected_app();
        deliver_readings(&mut app, &[DC]);
        app.settings.shared.device_family = "ut181a".to_string();
        deliver_readings(&mut app, &[DC]);

        assert_eq!(app.capture.recording.export_samples().len(), 2);
        let ut61eplus = registry::find_device("ut61eplus").expect("a registry entry");
        assert_eq!(
            app.capture.history_layout.device.as_deref(),
            Some(ut61eplus.display_name)
        );
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
            app.capture.recording.set_max_samples(points as usize);
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
            assert_eq!(
                app.capture.recording.export_samples().len(),
                points as usize
            );
            let elapsed = send(&mut app, points..points + 1_000);
            assert_eq!(
                app.capture.recording.export_samples().len(),
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
