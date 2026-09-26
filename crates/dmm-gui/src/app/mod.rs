//! The application: the [`App`] state every panel reads and writes, and the
//! per-frame `update` that lays the panels out.
//!
//! The concerns live in submodules — [`appearance`] (fonts, theme, zoom),
//! [`connection`] and [`messages`] (the acquisition thread and its channel),
//! [`plot_input`], [`top_bar`], [`toast`], [`controls`], [`layout`] (the
//! reading column), [`meter_fit`] (the big meter's sizing arithmetic),
//! [`stats_panel`], [`recording_panel`], [`export`], [`transform_ui`],
//! [`shortcuts`], [`shortcut_help`] and [`whats_new`] — all of which add
//! methods to the one [`App`] declared here.

mod appearance;
mod connection;
mod controls;
mod export;
mod held_reading;
mod layout;
mod messages;
mod meter_fit;
mod plot_input;
mod recording_panel;
mod shortcut_help;
mod shortcuts;
mod stats_panel;
mod toast;
mod top_bar;
mod transform_ui;
mod whats_new;

use dmm_lib::measurement::Measurement;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::{Choice, MeterKeys, Setting, registry};
use dmm_lib::transform::Transform;
use eframe::egui;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::a11y::ResponseA11yExt;
use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{Settings, ThemeMode};
use appearance::{UiColorKey, font_definitions, install_text_styles};
use connection::RemoteCommand;
use dmm_lib::stats::SeriesStats;
use export::ExportOutcome;
use layout::ContentLayout;
use messages::ConnectionIssue;
use meter_fit::{FitInputs, MeterFit, WindowContent};
use recording_panel::RecordingPanel;
use transform_ui::TransformEditor;

/// How long a toast message stays visible (seconds).
const TOAST_DURATION_SECS: u64 = 8;

/// Default height of the recording panel (logical pixels).
const DEFAULT_RECORDING_HEIGHT: f32 = 120.0;

/// Default width for the side panel in wide layout (logical pixels).
const SIDE_PANEL_DEFAULT_WIDTH: f32 = 240.0;

/// Allowed range for the resizable side panel.
const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
const SIDE_PANEL_MAX_WIDTH: f32 = 400.0;

use connection::{DmmMessage, ThreadControl};

/// Big meter display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
enum BigMeterMode {
    #[default]
    Off,
    /// Value + mode line + command buttons (no graph/stats/specs).
    Full,
    /// Value + mode line only (no top bar, no buttons).
    Minimal,
}

/// Connection state.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ConnectionState {
    Disconnected,
    Connected,
    Reconnecting,
}

/// Keyboard shortcut help overlay: whether it is showing, and the focus
/// bookkeeping that returns the keyboard where it came from.
#[derive(Default)]
struct ShortcutHelp {
    /// Whether the keyboard shortcut help overlay is open.
    open: bool,
    /// Widget id that opened the shortcut help window — focus is restored to
    /// this widget when the window closes so keyboard users don't lose place.
    opener: Option<egui::Id>,
    /// Pending focus target to restore after the shortcut help modal closes.
    /// The restore is deferred until `top_modal_layer` has actually cleared —
    /// otherwise egui's `create_widget` calls `surrender_focus` on widgets
    /// below the modal layer (still committed from the close frame) and
    /// wipes any focus we set in the close path.
    restore_focus: Option<egui::Id>,
    /// Set on the frame the shortcut help window is opened so the next frame
    /// can focus the first widget inside it (one-shot trigger).
    focus_pending: bool,
    /// Scroll the help's scroller is owed this frame, from a key the modal
    /// has to handle itself. Set by `handle_shortcut_help_keys` before the
    /// panels and spent when the modal draws.
    scroll: HelpScroll,
}

/// A pending keyboard scroll of the shortcut help.
///
/// `Lines` and `Page` carry a direction: `1.0` towards the end of the list,
/// `-1.0` towards its start. How far a line or a page is depends on the text
/// style and the viewport, so only the modal can turn these into pixels.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
enum HelpScroll {
    #[default]
    None,
    Lines(f32),
    Page(f32),
    Top,
    Bottom,
}

/// The "What's New" changelog viewport: whether it is showing, the focus to
/// return on close, and the state shared with the viewport callback.
#[derive(Default)]
struct WhatsNew {
    /// Whether the "What's New" changelog window is open.
    open: bool,
    /// Widget id that opened the What's New viewport — focus is restored to
    /// this widget when the viewport closes.
    opener: Option<egui::Id>,
    /// Set by the viewport callback when the user closes the changelog window.
    closed: Arc<AtomicBool>,
    /// Shared commonmark cache for the changelog viewport.
    cache: Arc<Mutex<egui_commonmark::CommonMarkCache>>,
}

/// Last-applied window chrome, kept so the per-frame paths can skip work that
/// egui and the windowing system charge for on every call.
#[derive(Default)]
struct AppliedChrome {
    /// OS default pixels_per_point, captured on first frame.
    os_ppp: Option<f32>,
    /// Last applied theme (to avoid re-setting every frame).
    theme: Option<ThemeMode>,
    /// Last applied UI chrome colors, to avoid per-frame Visuals mutation.
    ui_colors: Option<UiColorKey>,
    /// Last minimum window size pushed to the windowing system, so the
    /// viewport command is only re-sent when it actually changes.
    min_size: Option<egui::Vec2>,
}

/// Provenance and column layout the sample buffer is exported with. Taken
/// when recording starts — or, for the history, from the connection its first
/// reading arrived on — and kept across disconnect, so a file describes the
/// meter its samples came from rather than whatever is selected at export
/// time.
#[derive(Default)]
struct CaptureLayout {
    /// Meter the buffered samples came from. Outlives disconnect so a capture
    /// can still be exported with the right provenance after the meter is
    /// unplugged.
    device: Option<&'static str>,
    /// Registry id of that meter, for a replay file's `# device:` line, taken
    /// at the same moment and for the same reason as `device`.
    ///
    /// `None` for the mock, whose readings are synthesised rather than decoded
    /// from frames — there is nothing to play back.
    device_id: Option<&'static str>,
    /// Whether that meter's protocol was short of verified, for the JSON
    /// export's `experimental` field. Taken alongside `device` and for the
    /// same reason: disconnecting clears the connection's stability, so read
    /// at export time it would mark an unplugged UT181A's readings verified.
    ///
    /// `None` until a meter has been connected during the recording — Record
    /// works while disconnected, and the stability read then is the default
    /// the disconnect left behind, not a meter's. The export falls back to the
    /// live connection, as `device` and `device_id` do.
    experimental: Option<bool>,
    /// The link those samples came over, for the replay export's `# link:`
    /// line. Taken alongside `device_id` and for the same reason: a
    /// disconnect clears the connection's link, and a file exported after
    /// unplugging would then claim the cable every unmarked file is read as.
    link: Option<dmm_lib::binary_help::Link>,
    /// Sub-value slots the connected meter family can report, from its
    /// profile. 0 until the first `Connected`, and kept on disconnect so a
    /// capture stays exportable with its full column layout.
    device_aux_slots: usize,
    /// Sub-value slots the meter itself can fill in the buffered samples,
    /// taken alongside `device` and for the same reason: the CSV column layout has to describe the meter the
    /// samples came from, not whatever is selected at export time.
    aux_slots: usize,
    /// Extra sub-value slots the export reserves *after* the meter's own, for
    /// the ones software appends (a transform's `Raw`). Kept apart from
    /// `aux_slots` so `Raw` gets a fixed trailing column instead of sliding
    /// forward whenever the meter sends fewer sub-values. Only ever grows
    /// during a recording — turning a scale off mid-capture leaves the
    /// trailing group empty rather than renumbering the columns already
    /// written into the user's mental model of the file.
    extra_slots: usize,
}

/// The choice lists the readout dropdowns draw, one per setting the
/// acquisition thread lists. Adding a setting is a field and two match arms.
#[derive(Default)]
pub(super) struct SettingChoices {
    mode: Vec<Choice>,
    range: Vec<Choice>,
}

impl SettingChoices {
    /// Store the list the acquisition thread sent for `setting`. Settings the
    /// readout does not draw are dropped rather than kept unused.
    pub(super) fn set(&mut self, setting: Setting, choices: Vec<Choice>) {
        match setting {
            Setting::Mode => self.mode = choices,
            Setting::Range => self.range = choices,
            _ => {}
        }
    }

    /// Forget every list — the dial may be anywhere by the time the meter is
    /// back, so nothing here survives a connect or a disconnect.
    pub(super) fn clear(&mut self) {
        self.mode.clear();
        self.range.clear();
    }

    /// Whether the range readout is drawn as a dropdown, which is taller than
    /// the label it replaces and so changes the big meter's fitted font.
    pub(super) fn range_offered(&self) -> bool {
        display::mode_switch_offered(&self.range)
    }
}

/// The meter a saved `device_family` names, or `None` when it names none.
///
/// `None` covers both [`registry::AUTO_DEVICE_ID`] and a value no registry
/// entry answers to — a hand-edited settings file, or a model removed since
/// it was written. Both mean "we were not told which meter this is", and
/// detection is a better answer to that than quietly opening the meter whose
/// tables happen to be the library's fallback.
fn named_device(family: &str) -> Option<&'static registry::SelectableDevice> {
    match registry::resolve_selection(family) {
        Some(registry::Selection::Device(d)) => Some(d),
        Some(registry::Selection::Auto) | None => None,
    }
}

/// The live link to a meter: its state, what the connected protocol told us
/// about itself, and the channels and flags shared with the acquisition
/// thread.
pub(super) struct Connection {
    pub(super) state: ConnectionState,
    pub(super) device_name: Option<String>,
    /// The registry entry actually connected — the one the user named, or the
    /// one detection settled on. `None` while nothing is connected, so under
    /// Auto-detect this is the only thing that knows which meter is on the
    /// cable. Cleared on disconnect.
    pub(super) detected: Option<&'static registry::SelectableDevice>,
    /// Model name the connected protocol reports, for the text that has to
    /// name it (the experimental warning). Empty while disconnected.
    pub(super) model_name: String,
    /// How far the connected protocol is verified; the badge shows for
    /// anything short of `Verified`.
    pub(super) stability: dmm_lib::protocol::Stability,
    /// URL for reporting feedback on experimental protocols.
    pub(super) feedback_url: String,
    /// What the meter is answering over — for a replay, what its recording
    /// was made over. `None` while disconnected, and for the mock, which is
    /// on no link at all.
    pub(super) link: Option<dmm_lib::binary_help::Link>,
    /// Commands supported by the connected protocol.
    pub(super) supported_commands: Vec<String>,
    /// The function and context keys the connected protocol lists, drawn
    /// as the mode readout's list and as chips beside HOLD.
    pub(super) meter_keys: MeterKeys,
    /// Values the meter can be switched to for each setting the readout
    /// draws, as last listed by the acquisition thread.
    pub(super) choices: SettingChoices,
    /// When true, incoming measurements are ignored (connection stays alive).
    pub(super) paused: bool,
    pub(super) last_error: Option<ConnectionIssue>,
    /// Consecutive timeout count (0 = not waiting).
    pub(super) waiting_timeouts: u32,
    /// Reconnect attempt count (0 = not reconnecting). Populated from the
    /// background thread while the state is `Reconnecting`.
    pub(super) reconnect_attempt: u32,
    /// Last reconnect failure message, if any.
    pub(super) reconnect_last_error: Option<String>,
    rx: Option<mpsc::Receiver<DmmMessage>>,
    ctrl_tx: Option<mpsc::Sender<ThreadControl>>,
    /// Stop request, readable without consuming a channel message so the
    /// acquisition thread's pacing sleep can bail out on it mid-tick.
    /// `ctrl_tx` stays the control path; this is the wake signal.
    stop_flag: Option<Arc<AtomicBool>>,
    pub(super) cmd_tx: Option<mpsc::Sender<RemoteCommand>>,
    /// Reconnect on next frame (device selection changed while connected).
    pub(super) needs_reconnect: bool,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            device_name: None,
            detected: None,
            model_name: String::new(),
            stability: dmm_lib::protocol::Stability::Verified,
            feedback_url: String::new(),
            link: None,
            supported_commands: Vec::new(),
            meter_keys: MeterKeys::NONE,
            choices: SettingChoices::default(),
            paused: false,
            last_error: None,
            waiting_timeouts: 0,
            reconnect_attempt: 0,
            reconnect_last_error: None,
            rx: None,
            ctrl_tx: None,
            stop_flag: None,
            cmd_tx: None,
            needs_reconnect: false,
        }
    }
}

impl Connection {
    /// What the readout dropdowns offer: the listed choices, and the
    /// meter's function keys.
    pub(super) fn readouts(&self) -> display::ReadoutChoices<'_> {
        display::ReadoutChoices {
            mode: &self.choices.mode,
            range: &self.choices.range,
            keys: self.meter_keys.functions,
        }
    }
}

pub struct App {
    pub(super) settings: Settings,
    pub(super) settings_open: bool,

    pub(super) connection: Connection,
    pub(super) last_measurement: Option<Measurement>,
    /// Parts of the reading on screen that came in frames of their own; see
    /// `held_reading`.
    held: held_reading::HeldReading,
    /// Software transform applied to every incoming reading. Session-only:
    /// see [`transform_ui`] for why it is never written to settings.
    transform: Transform,
    /// Draft text for the **Scale** row, separate from `transform` so a
    /// half-typed number never reaches the reading.
    transform_editor: TransformEditor,

    graph: Graph,
    /// Min/max/avg and the running integral of the current series. The GUI
    /// always integrates: the stats panel shows the integral whenever the
    /// current unit has a meaningful one.
    session: SeriesStats,
    recording: Recording,
    /// Session-long `(Instant, SystemTime)` origin pair used to map
    /// `m.timestamp` (monotonic) onto wall-clock timestamps for recording and
    /// export. Captured once at construction so every sample across the
    /// session is translated against the same origin.
    wall_clock: dmm_lib::WallClock,
    /// Time base the session's readings are stamped with — real unless a
    /// `--mock-clock-*` flag was given. Cloned into the acquisition thread so
    /// the mock's waveform, the pacing loop and anything here that measures
    /// session age (the recording duration) agree. UI cadence — toasts,
    /// repaint, control-channel waits — stays on real time.
    clock: dmm_lib::Clock,
    /// The recording this session plays instead of opening a meter, from
    /// `--replay`. Session-only, like the clock: the meter it names reaches
    /// the settings as an override, so nothing about a playback is saved.
    replay: Option<crate::ReplaySource>,

    capture_layout: CaptureLayout,
    /// Profile of the selected device, refreshed only when the selection
    /// changes. Two render paths need it every frame, and building a protocol
    /// to read it allocates — the UT61E+ factory lowercases its model string,
    /// boxes a device table and reserves an rx buffer.
    ///
    /// `None` under Auto-detect: no meter is named, so there is no profile to
    /// describe until one answers — what the connection then reports stands
    /// in ([`Connection::detected`]).
    selected_profile: Option<dmm_lib::protocol::DeviceProfile>,
    /// Device id the cached profile belongs to, [`registry::AUTO_DEVICE_ID`]
    /// when none is selected.
    selected_profile_id: &'static str,
    recording_panel: RecordingPanel,
    first_frame: bool,
    /// The window is a native Wayland surface, where the compositor owns
    /// stacking: xdg-shell has no keep-above request, so winit drops
    /// `WindowLevel` commands and "Always on top" cannot work.
    on_wayland: bool,
    applied: AppliedChrome,
    /// Transient status toast (message, is_error, timestamp).
    toast: Option<(String, bool, Instant)>,
    /// One-shot receiver for CSV export result.
    export_result_rx: Option<mpsc::Receiver<ExportOutcome>>,
    meter_fit: MeterFit,
    /// Transient big meter mode (not persisted to settings).
    big_meter_mode: BigMeterMode,
    shortcut_help: ShortcutHelp,
    whats_new: WhatsNew,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, cli: crate::CliOverrides) -> Self {
        install_text_styles(&cc.egui_ctx);
        cc.egui_ctx.set_fonts(font_definitions());
        // Asked once, here: the display handle says which backend the window
        // actually got, where `WAYLAND_DISPLAY` only says which one is on
        // offer.
        let on_wayland = cc
            .display_handle()
            .is_ok_and(|handle| matches!(handle.as_raw(), RawDisplayHandle::Wayland(_)));
        let mut app = Self::from_cli(Settings::load(), cli);
        app.on_wayland = on_wayland;
        app
    }

    /// The session `settings` and the flags that override them describe.
    /// Separate from [`App::new`] so tests can build one without an eframe
    /// context.
    ///
    /// Every overridden field is recorded in `settings.overrides`, which is
    /// what [`Settings::save`] puts back before writing: a flag changes this
    /// session, never the file.
    fn from_cli(mut settings: Settings, cli: crate::CliOverrides) -> Self {
        if let Some(device) = cli.device {
            settings.overrides.device_family = Some(settings.shared.device_family.clone());
            settings.shared.device_family = device;
        }
        if let Some(mock_mode) = cli.mock_mode {
            settings.overrides.mock_mode = Some(settings.mock_mode.clone());
            // The label, not what was typed: the Settings row matches on
            // labels, so an alias (`--mock-mode temp_dual`) would leave the
            // row showing no selection at all.
            settings.mock_mode = mock_mode.label().to_string();
        }
        if let Some(theme) = cli.theme {
            settings.overrides.theme = Some(settings.theme);
            settings.theme = theme;
        }
        // Only the off switch is a flag: there is nothing to force on, the
        // setting already is.
        if cli.no_bluetooth {
            settings.overrides.bluetooth = Some(settings.shared.bluetooth);
            settings.shared.bluetooth = false;
        }
        settings.overrides.adapter = cli.adapter;
        let mut app = Self::from_settings(settings, cli.clock);
        app.replay = cli.replay;
        app
    }

    /// The app state for `settings` on `clock`, before any frame. Separate
    /// from [`App::new`] so tests can build one without an eframe context.
    fn from_settings(settings: Settings, clock: dmm_lib::Clock) -> Self {
        let mut graph = Graph::new();
        // One setting bounds both stores of the sample stream.
        graph.set_max_points(settings.max_samples);
        let mut recording = Recording::new();
        // A fresh buffer holds nothing, so this cannot stop anything.
        recording.set_max_samples(settings.max_samples);
        let initial_device = named_device(&settings.shared.device_family);
        Self {
            settings,
            settings_open: false,
            connection: Connection::default(),
            last_measurement: None,
            held: held_reading::HeldReading::default(),
            transform: Transform::default(),
            transform_editor: TransformEditor::default(),
            graph,
            session: SeriesStats::new(true),
            recording,
            wall_clock: dmm_lib::WallClock::from_clock(&clock),
            clock,
            replay: None,
            capture_layout: CaptureLayout::default(),
            selected_profile: initial_device.map(|d| *(d.new_protocol)().profile()),
            selected_profile_id: initial_device.map_or(registry::AUTO_DEVICE_ID, |d| d.id),
            recording_panel: RecordingPanel::default(),
            first_frame: true,
            // Only `App::new` has a window to ask; tests build without one.
            on_wayland: false,
            applied: AppliedChrome::default(),
            toast: None,
            export_result_rx: None,
            meter_fit: MeterFit::new(),
            big_meter_mode: BigMeterMode::Off,
            shortcut_help: ShortcutHelp::default(),
            whats_new: WhatsNew::default(),
        }
    }

    /// Re-read the selected device's profile if the selection changed.
    ///
    /// Called once per frame instead of at each use: the two render paths
    /// that need it (`show_connection_help`, the status landmark) run every
    /// repaint, and `(new_protocol)()` allocates.
    fn refresh_selected_profile(&mut self) {
        let device = self.selected_device();
        let id = device.map_or(registry::AUTO_DEVICE_ID, |d| d.id);
        if self.selected_profile_id != id {
            self.selected_profile = device.map(|d| *(d.new_protocol)().profile());
            self.selected_profile_id = id;
        }
    }

    /// Set the pause state and tell the acquisition thread about it.
    ///
    /// Pause halts acquisition — the meter stops being polled entirely. The
    /// live-view toggle is the separate scroll-lock that freezes the view
    /// while data keeps arriving.
    pub(super) fn set_paused(&mut self, paused: bool) {
        self.connection.paused = paused;
        if paused {
            // Acquisition stops, so the samples that would have covered this
            // stretch never exist — a data gap the graph should show even if
            // the pause is shorter than its elapsed-time threshold.
            self.graph.push_data_loss();
            self.held.clear();
        }
        if let Some(tx) = &self.connection.ctrl_tx {
            let _ = tx.send(ThreadControl::SetPaused(paused));
        }
    }

    pub(super) fn send_command(&self, cmd: &str) {
        if let Some(tx) = &self.connection.cmd_tx {
            let _ = tx.send(RemoteCommand::Named(cmd.to_string()));
        }
    }

    /// Act on a readout dropdown's pick: a switch, or a key press. Either
    /// one's refusal comes back as a toast; the stream shows the outcome.
    pub(super) fn apply_pick(&mut self, pick: display::ReadoutPick) {
        match pick {
            display::ReadoutPick::Select(setting, id) => self.select(setting, id),
            display::ReadoutPick::Press(command) => self.send_command(command),
        }
    }

    /// Ask the meter to switch `setting` to one of the values
    /// `connection.choices` listed. A refusal comes back as a toast.
    pub(super) fn select(&mut self, setting: Setting, id: u16) {
        if let Some(tx) = &self.connection.cmd_tx {
            let _ = tx.send(RemoteCommand::Select(setting, id));
        }
        // Only the mode pin: `settings.mock_mode` names a scenario, and a
        // range pick leaves the mock in the scenario it is already pinned to.
        if setting == Setting::Mode && self.repin_mock(id) {
            self.settings.save();
        }
    }

    /// Keep the Settings row's mock pin truthful after a dropdown pick.
    ///
    /// `settings.mock_mode` is the scenario the mock is pinned to at connect;
    /// a pick moves the mock's live scenario without a reconnect, so a pinned
    /// mock is re-pinned to the scenario picked. Returns whether the settings
    /// changed (and so need saving). No `needs_reconnect`: the mock has
    /// already switched, and a reconnect would restart it. An auto-cycling
    /// mock (empty pin) is left alone — it carries on cycling from the picked
    /// scenario, so the row stays right. The SELECT button has the same
    /// desync and is left alone: the GUI cannot learn which scenario the
    /// mock cycled to.
    fn repin_mock(&mut self, id: u16) -> bool {
        if self.selected_device().is_none_or(|d| d.id != "mock")
            || self.settings.mock_mode.is_empty()
        {
            return false;
        }
        let Some(mode) = MockMode::from_choice_id(id) else {
            return false;
        };
        self.settings.mock_mode = mode.label().to_string();
        // An explicit choice, as in the Settings row: it replaces a
        // `--mock-mode` override rather than being saved under it.
        self.settings.overrides.mock_mode = None;
        true
    }

    /// The meter the user named, or `None` when the one on the cable is to be
    /// identified instead.
    fn selected_device(&self) -> Option<&'static registry::SelectableDevice> {
        named_device(&self.settings.shared.device_family)
    }

    /// The meter this session is actually talking about: the one the user
    /// named, else the one detection found. `None` until a meter answers
    /// under Auto-detect.
    fn active_device(&self) -> Option<&'static registry::SelectableDevice> {
        self.selected_device().or(self.connection.detected)
    }

    fn manual_url(&self) -> Option<&'static str> {
        self.active_device().and_then(|d| d.manual_url)
    }

    /// Status text while the acquisition thread is retrying.
    ///
    /// One copy: the disabled connect button and the status indicator both
    /// show this, and they were two independent formattings of the same
    /// user-facing string.
    fn reconnecting_label(&self) -> String {
        if self.connection.reconnect_attempt > 0 {
            format!(
                "Reconnecting (attempt {})...",
                self.connection.reconnect_attempt
            )
        } else {
            "Reconnecting...".to_string()
        }
    }

    /// The big meter: one reading scaled to fill the window, with the command
    /// buttons under it — and, when the mode is `Off` and it is only the
    /// hidden panels that left the reading alone, the connection help, specs
    /// and statistics too.
    ///
    /// Its own method rather than a block in [`eframe::App::ui`] so a headless
    /// test can drive exactly what a frame draws here.
    fn show_meter_only(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, minimal: bool) {
        // Big meter mode: compute scale from window size, only recalculate
        // when the window is resized to avoid frame-to-frame oscillation.
        // Shrink panel margins at small window sizes so the reading fills
        // the space tighter.
        let margin_scale = meter_fit::margin_scale(ctx.content_rect().size());
        let default_margin = ctx.global_style().spacing.window_margin;
        let frame = egui::Frame::central_panel(ctx.global_style().as_ref())
            .inner_margin(default_margin * margin_scale);

        // Full and Minimal fill the window with the reading, leaving nowhere
        // to draw the connection help: the issue takes the reading's place
        // instead, and the steps become its hover text. Built here, outside
        // the closure that borrows `self` mutably.
        let notice = (self.big_meter_mode != BigMeterMode::Off)
            .then(|| self.connection_notice())
            .flatten();
        // Rendered by the context rather than spelled out: the binding uses
        // `Modifiers::COMMAND`, which is Cmd on macOS.
        // The sections and the prose under them as one string: the tooltip is
        // the only place a big-meter session can read them.
        let tooltip = notice.as_ref().map(|n| n.help_text());
        let hint = notice.is_some().then(|| {
            format!(
                "{} for details",
                ctx.format_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::B
                ))
            )
        });

        let main = egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let size = ctx.content_rect();
            let fit_inputs = FitInputs {
                width: size.width() as u32,
                height: size.height() as u32,
                mode_raw: self.last_measurement.as_ref().map_or(0, |m| m.mode_raw),
                aux_values: self
                    .last_measurement
                    .as_ref()
                    .map_or(0, |m| m.aux_values.len()),
                mode_offered: self.connection.readouts().mode_offered(),
                range_offered: self.connection.choices.range_offered(),
                show_stats: self.settings.show_stats,
                show_specs: self.settings.show_specs,
                spec_fields: self.settings.spec_fields,
                big_meter_mode: self.big_meter_mode,
                transform_editor_open: self.transform_editor.open,
                transform_is_identity: self.transform.is_identity(),
                notice_kind: notice.as_ref().map(|n| n.kind),
            };
            let needs_recalc = self.meter_fit.needs_recalc(&fit_inputs);

            let panel_rect = ui.max_rect();
            let mut add_content = |ui: &mut egui::Ui| {
                ui.vertical(|ui| {
                    // In minimal mode there's nothing below the reading,
                    // so pass 0 to let the reading fill all available space.
                    let content_h = if minimal {
                        0.0
                    } else {
                        self.meter_fit.content_height
                    };
                    let tc = self.settings.theme_colors(ui.visuals().dark_mode);
                    let no_reading = match (notice.as_ref(), hint.as_deref(), tooltip.as_deref()) {
                        (Some(n), Some(hint), Some(tooltip)) => display::NoReadingText::Notice {
                            title: &n.title,
                            hint,
                            tooltip,
                            color: tc.status_warning(),
                        },
                        _ => display::NoReadingText::Plain,
                    };
                    let (scale, measured_ratios, picked) = display::show_reading_large(
                        ui,
                        self.last_measurement.as_ref(),
                        display::ReadingFit {
                            base_content_height: content_h,
                            ratios: &self.meter_fit.reading_ratios,
                        },
                        &tc,
                        !self.transform.is_identity(),
                        self.connection.readouts(),
                        no_reading,
                    );
                    if let Some(pick) = picked {
                        self.apply_pick(pick);
                    }
                    let after_reading = ui.cursor().top();

                    if !minimal {
                        // The big-meter toggle sits in the panel corner
                        // here, not on the row: nothing to keep clear of.
                        self.show_remote_controls(ui, scale, 0.0);
                        self.show_transform_editor(ui, scale);
                    }

                    if self.big_meter_mode == BigMeterMode::Off {
                        // Only here: in Full and Minimal the notice is
                        // already in the readout, and the help drawn under a
                        // window-filling reading would land off-screen.
                        self.show_connection_help(ui);
                        self.show_specs_section_inline(ui, scale);

                        if self.settings.show_stats {
                            ui.add_space(12.0 * scale);
                            ui.separator();
                            self.show_stats_section(ui, false, scale);
                        }
                    }

                    // Update cached dimensions on window resize. Run twice
                    // (by not closing the cache the first time) so the
                    // second pass uses the measured values from the first.
                    if needs_recalc && scale > 0.0 {
                        let total_below_reading = ui.cursor().top() - after_reading;
                        self.meter_fit.record_pass(
                            &fit_inputs,
                            total_below_reading / scale,
                            measured_ratios,
                        );
                    }
                });
            };
            if minimal {
                add_content(ui);
            } else {
                ui.centered_and_justified(add_content);
            }
            // Overlay toggle button in the bottom-right, outside the
            // measured content so it doesn't affect scaling convergence.
            // Hide when the panel is too small to avoid overlapping the reading.
            if panel_rect.width() > 100.0 && panel_rect.height() > 80.0 {
                let btn_rect = egui::Rect::from_min_size(
                    egui::pos2(panel_rect.right() - 32.0, panel_rect.bottom() - 32.0),
                    egui::vec2(28.0, 28.0),
                );
                self.show_big_meter_toggle_at(ui, btn_rect);
            }
        });
        main.response.a11y_role(egui::accesskit::Role::Main);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Deferred focus restoration after the shortcut help modal closes.
        // We only run this once `top_modal_layer` has actually cleared —
        // otherwise egui's `create_widget` would `surrender_focus` the
        // target widget while rendering the top bar, because it's below
        // the still-committed modal layer.
        if let Some(target) = self.shortcut_help.restore_focus
            && ctx.memory(|m| m.top_modal_layer()).is_none()
        {
            ctx.memory_mut(|m| m.request_focus(target));
            self.shortcut_help.restore_focus = None;
        }
        self.refresh_selected_profile();
        self.apply_theme(&ctx);
        self.apply_color_overrides(&ctx);
        self.apply_zoom(&ctx);
        self.handle_keyboard_shortcuts(&ctx);
        self.handle_shortcut_help_keys(&ctx);
        self.drain_messages();
        self.poll_export_result();

        // Auto-reconnect after device selection change
        if self.connection.needs_reconnect {
            self.connection.needs_reconnect = false;
            self.connect(&ctx);
        }

        // Expire the toast, unless the user closed it first
        if let Some((_, _, when)) = &self.toast
            && when.elapsed().as_secs() >= TOAST_DURATION_SECS
        {
            self.toast = None;
        }

        // Auto-connect on first frame if enabled
        if self.first_frame {
            self.first_frame = false;
            if self.settings.always_on_top && !self.on_wayland {
                self.apply_always_on_top(&ctx);
            }
            if self.settings.hide_decorations {
                self.apply_decorations(&ctx);
            }
            if self.settings.auto_connect {
                self.connect(&ctx);
            }
            // Show "What's New" on first launch after a release upgrade.
            // Dev builds (-dev suffix) never auto-open to avoid annoyance.
            let current_version = env!("CARGO_PKG_VERSION");
            if !current_version.contains("-dev")
                && self.settings.last_seen_version.as_deref() != Some(current_version)
                && crate::changelog::has_version_section(current_version)
            {
                self.open_whats_new();
            }
        }

        let minimal = self.big_meter_mode == BigMeterMode::Minimal;
        // Where the toast hangs from: under the bar (and under the settings
        // rows it opens), at the window's top edge when there is no bar.
        let mut toast_top = ctx.content_rect().top();
        if !minimal {
            let top = egui::Panel::top("top_bar").show(ui, |ui| {
                self.show_top_bar(ui, &ctx);
                self.show_settings_panel(ui);
            });
            toast_top = top.response.rect.bottom();
        }

        // Determine layout mode before panels
        let wide = meter_fit::is_wide(ctx.content_rect().width());

        let meter_only = self.big_meter_mode != BigMeterMode::Off
            || (!self.settings.show_graph && !self.settings.show_recording);

        // Dynamic minimum window size derived from actual rendered content.
        // Reading dimensions come from cached ratios × minimum big meter
        // font size; top bar widths come from previous-frame measurements.
        let bar_left_w: f32 =
            ctx.data(|d| d.get_temp(egui::Id::new("top_bar_left_w")).unwrap_or(300.0));
        let bar_right_w: f32 = ctx.data(|d| {
            d.get_temp(egui::Id::new("top_bar_right_w"))
                .unwrap_or(120.0)
        });
        let bar_min_w = bar_left_w.max(bar_right_w) + 16.0;

        let window_content = if minimal {
            WindowContent::ReadingOnly
        } else if meter_only {
            WindowContent::Meter
        } else {
            WindowContent::Panels
        };
        let min_size = self.meter_fit.min_window_size(window_content, bar_min_w);
        // Only when it changes. Its inputs — cached top-bar widths, meter
        // ratios, big-meter mode — are stable across the vast majority of
        // frames, and a viewport command sent every repaint is the same class
        // of mistake .claude/rules/gui.md calls out for set_visuals() and
        // set_pixels_per_point(). Half a pixel of tolerance keeps sub-pixel
        // jitter in the cached widths from re-triggering it.
        if self
            .applied
            .min_size
            .is_none_or(|prev| (prev - min_size).abs().max_elem() > 0.5)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
            self.applied.min_size = Some(min_size);
        }
        // If the window is smaller than the new minimum (e.g. after exiting
        // minimal mode), grow it to fit.
        if let Some(grown) = meter_fit::grow_to_fit(ctx.content_rect().size(), min_size) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(grown));
        }

        if meter_only {
            self.show_meter_only(ui, &ctx, minimal);
        } else if wide {
            // Wide: left side panel for reading + stats (resizable)
            let reading_panel = egui::Panel::left("reading_panel")
                .default_size(SIDE_PANEL_DEFAULT_WIDTH)
                .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH)
                .resizable(true)
                .show(ui, |ui| {
                    self.show_reading_column_scrolled(ui, ContentLayout::Wide);
                });
            // egui's `Panel::left(..).resizable(true)` allocates a
            // drag-sense resize handle at its right edge, which is focusable
            // but has no visible focus indicator of its own and no keyboard
            // action. Paint a focus ring and wire up Left/Right arrow keys
            // to resize the panel, consistent with the recording-panel
            // divider. The handle id is derived from the panel id — see
            // `resize_widget_id` at `panel.rs:36` in egui 0.36 for the salt.
            let reading_panel_id = egui::Id::new("reading_panel");
            let reading_panel_resize_id = reading_panel_id.with("__resize");
            crate::a11y::set_accessible_label(
                ui,
                reading_panel_resize_id,
                "Resize reading panel (Left/Right to adjust)",
            );
            if ctx.memory(|m| m.focused()) == Some(reading_panel_resize_id) {
                let delta = crate::a11y::arrow_resize(
                    &ctx,
                    reading_panel_resize_id,
                    crate::a11y::ResizeAxis::Horizontal,
                    20.0,
                );
                if delta != 0.0
                    && let Some(mut state) = egui::PanelState::load(&ctx, reading_panel_id)
                {
                    let new_width = (state.outer_rect.width() + delta)
                        .clamp(SIDE_PANEL_MIN_WIDTH, SIDE_PANEL_MAX_WIDTH);
                    state.outer_rect.max.x = state.outer_rect.min.x + new_width;
                    ctx.data_mut(|d| d.insert_persisted(reading_panel_id, state));
                }
                // Paint a 3px focus indicator on the panel's right edge —
                // the standard focus ring is invisible on the thin vline
                // egui uses to draw the panel boundary.
                let panel_rect = reading_panel.response.rect;
                let stroke_color = crate::a11y::focus_ring_color(ui.visuals());
                ui.painter().vline(
                    panel_rect.right(),
                    panel_rect.y_range(),
                    egui::Stroke::new(3.0_f32, stroke_color),
                );
            }

            // Wide: center panel for graph + recording
            egui::CentralPanel::default()
                .show(ui, |ui| {
                    self.show_graph_column(ui);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        } else {
            // Narrow: single column
            egui::CentralPanel::default()
                .show(ui, |ui| {
                    self.show_reading_column_scrolled(ui, ContentLayout::Narrow);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        }

        self.show_toast(&ctx, toast_top);
        self.show_shortcut_help(&ctx);
        self.show_discard_confirmation(&ctx);
        self.show_whats_new(&ctx);

        if self.connection.state == ConnectionState::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Node, NodeId};

    fn app(device: &str, mock_mode: &str) -> App {
        let mut settings = Settings::default();
        settings.shared.device_family = device.to_string();
        settings.mock_mode = mock_mode.to_string();
        // As after `--mock-mode`: the pin on screen is an override.
        settings.overrides.mock_mode = Some(String::new());
        App::from_settings(settings, dmm_lib::Clock::real())
    }

    fn choice_id_of(mode: MockMode) -> u16 {
        (0..u16::MAX)
            .find(|&id| MockMode::from_choice_id(id) == Some(mode))
            .expect("every mock mode has a choice id")
    }

    /// The Settings row pins the mock at connect; a dropdown pick moves the
    /// mock without a reconnect, so the pin has to follow it or the row lies.
    #[test]
    fn a_pick_re_pins_a_pinned_mock_without_reconnecting() {
        let mut app = app("mock", "temp2");
        assert!(app.repin_mock(choice_id_of(MockMode::TempDiff)));
        assert_eq!(app.settings.mock_mode, "temp-diff");
        assert_eq!(
            app.settings.overrides.mock_mode, None,
            "the pick replaces a --mock-mode override"
        );
        assert!(!app.connection.needs_reconnect);
    }

    /// An auto-cycling mock keeps cycling from the picked scenario, so the
    /// row's "Auto (cycle)" stays true and is left alone.
    #[test]
    fn a_pick_leaves_an_auto_cycling_mock_unpinned() {
        let mut app = app("mock", "");
        assert!(!app.repin_mock(choice_id_of(MockMode::TempDiff)));
        assert_eq!(app.settings.mock_mode, "");
        assert!(!app.connection.needs_reconnect);
    }

    /// `--no-bluetooth` is one session's answer: no scanning
    /// here, and the saved value is what [`Settings::save`] writes back.
    #[test]
    fn the_bluetooth_flag_does_not_reach_the_settings_file() {
        let settings = Settings {
            auto_connect: false,
            ..Settings::default()
        };
        assert!(settings.shared.bluetooth, "probing is on by default");
        let app = App::from_cli(
            settings,
            crate::CliOverrides {
                device: None,
                mock_mode: None,
                theme: None,
                renderer: None,
                adapter: None,
                no_bluetooth: true,
                clock: dmm_lib::Clock::real(),
                replay: None,
            },
        );
        assert!(!app.settings.shared.bluetooth, "this session skips it");
        assert_eq!(app.settings.overrides.bluetooth, Some(true));
    }

    /// A pick on a real meter has nothing to do with the mock's pin.
    #[test]
    fn a_pick_on_a_real_meter_leaves_the_mock_pin_alone() {
        let mut app = app("ut181a", "temp2");
        assert!(!app.repin_mock(0x1121));
        assert_eq!(app.settings.mock_mode, "temp2");
    }

    /// `--replay` runs the session as the meter the recording came from —
    /// for this session only. The meter the user picked is kept in
    /// `overrides`, which is what [`Settings::save`] writes back, so a
    /// playback cannot leave a device behind in the settings file.
    #[test]
    fn a_replay_device_never_reaches_the_saved_settings() {
        let replay = dmm_lib::replay::Replay::parse(
            "# dmm-replay 1\n\
             # device: ut61eplus\n\
             # recorded: 2026-09-16T10:22:31.123+02:00\n\
             0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n",
        )
        .expect("a well-formed recording");
        let mut settings = Settings {
            // No acquisition thread: this is about the settings only.
            auto_connect: false,
            ..Settings::default()
        };
        settings.shared.device_family = "mock".to_string();

        let app = App::from_cli(
            settings,
            crate::CliOverrides {
                // What `parse_args` resolves a `--replay` file to.
                device: Some(replay.device.id.to_string()),
                mock_mode: None,
                theme: None,
                renderer: None,
                adapter: None,
                no_bluetooth: false,
                clock: dmm_lib::Clock::real(),
                replay: Some(crate::ReplaySource {
                    replay: Arc::new(replay),
                    path: std::path::PathBuf::from("dcv-steps.replay"),
                    recorded: std::time::SystemTime::UNIX_EPOCH,
                }),
            },
        );

        assert_eq!(app.settings.shared.device_family, "ut61eplus");
        assert_eq!(
            app.settings.overrides.device_family.as_deref(),
            Some("mock"),
            "the saved device is the one Settings::save puts back"
        );
        assert!(app.replay.is_some(), "the session plays the recording");
    }

    /// A named meter still resolves to its entry, aliases included; `auto`
    /// names none. The unknown case is the one that used to bite: it fell
    /// back to the UT61E+, so a typo in the settings file opened a meter the
    /// user had never chosen and failed on the first frame it parsed.
    #[test]
    fn only_a_named_meter_resolves_to_an_entry() {
        assert_eq!(named_device("ut61b+").map(|d| d.id), Some("ut61b+"));
        assert_eq!(named_device("UT61E").map(|d| d.id), Some("ut61eplus"));
        assert!(named_device(registry::AUTO_DEVICE_ID).is_none());
        assert!(named_device("AUTO").is_none());
        assert!(named_device("no such meter").is_none());
        assert!(named_device("").is_none());
    }

    /// Auto-detect has no profile of its own, so nothing may assume one: the
    /// experimental badge and the connection help both read it every frame.
    #[test]
    fn auto_detect_carries_no_profile_until_a_meter_answers() {
        let mut app = app(registry::AUTO_DEVICE_ID, "");
        assert!(app.selected_device().is_none());
        assert!(app.selected_profile.is_none());
        assert_eq!(app.selected_profile_id, registry::AUTO_DEVICE_ID);
        assert!(app.active_device().is_none(), "nothing connected yet");

        // What `DmmMessage::Connected` does: the meter that answered is the
        // one the top bar names from then on.
        app.connection.detected = registry::find_device("ut8803");
        assert_eq!(app.active_device().map(|d| d.id), Some("ut8803"));

        // And picking a model back out of the picker restores its profile.
        app.settings.shared.device_family = "ut61b+".to_string();
        app.refresh_selected_profile();
        assert_eq!(app.selected_profile_id, "ut61b+");
        assert_eq!(
            app.selected_profile.map(|p| p.model_name),
            Some("UNI-T UT61B+"),
            "the picked model outranks the connected one"
        );
        assert_eq!(app.active_device().map(|d| d.id), Some("ut61b+"));
    }

    /// The distinctive sentence out of the "No response from meter" body —
    /// the steps the big meter has no room to draw.
    const BODY_PHRASE: &str = "enable data transmission";

    /// An app with a meter selected, no reading, and the acquisition thread
    /// reporting silence: the state the readout's placeholder speaks for.
    fn stalled_app(mode: BigMeterMode) -> App {
        let mut settings = Settings {
            // No acquisition thread: the failure is set by hand.
            auto_connect: false,
            ..Settings::default()
        };
        settings.shared.device_family = "ut61eplus".to_string();
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        app.big_meter_mode = mode;
        app.connection.last_error = Some(ConnectionIssue::Other("timed out".to_string()));
        app
    }

    /// Run `draw` in a `w` x `h` window and return the last frame's
    /// accessibility nodes. Several frames, because the big meter's fit
    /// re-measures itself from the one before.
    fn frame_nodes(
        app: &mut App,
        w: f32,
        h: f32,
        mut draw: impl FnMut(&mut App, &mut egui::Ui),
    ) -> Vec<(NodeId, Node)> {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
        let mut nodes = Vec::new();
        for _ in 0..6 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| draw(app, ui),
            );
            // epaint 0.36 debug-asserts on dropping unapplied texture deltas;
            // this harness renders without a painter.
            out.textures_delta.clear();
            nodes = out
                .platform_output
                .accesskit_update
                .map(|update| update.nodes)
                .unwrap_or_default();
        }
        nodes
    }

    /// The meter-only branch in [`eframe::App::ui`]'s order: the top bar where
    /// there is one, then the big meter.
    fn meter_nodes(app: &mut App, w: f32, h: f32) -> Vec<(NodeId, Node)> {
        frame_nodes(app, w, h, |app, ui| {
            let ctx = ui.ctx().clone();
            let minimal = app.big_meter_mode == BigMeterMode::Minimal;
            if !minimal {
                egui::Panel::top("top_bar").show(ui, |ui| app.show_top_bar(ui, &ctx));
            }
            app.show_meter_only(ui, &ctx, minimal);
        })
    }

    /// The text AccessKit reports for a node: a plain label carries it as the
    /// node's value, a button or a link as its label.
    fn node_text(node: &Node) -> Option<&str> {
        node.value().or_else(|| node.label())
    }

    fn shows_text(nodes: &[(NodeId, Node)], needle: &str) -> bool {
        nodes
            .iter()
            .any(|(_, n)| node_text(n).is_some_and(|t| t.contains(needle)))
    }

    /// Where the first node whose text contains `needle` was drawn.
    fn text_bounds(nodes: &[(NodeId, Node)], needle: &str) -> Option<egui::Rect> {
        let (_, node) = nodes
            .iter()
            .find(|(_, n)| node_text(n).is_some_and(|t| t.contains(needle)))?;
        let b = node.bounds().expect("a drawn label has bounds");
        Some(egui::Rect::from_min_max(
            egui::pos2(b.x0 as f32, b.y0 as f32),
            egui::pos2(b.x1 as f32, b.y1 as f32),
        ))
    }

    /// The big meter fills the window with the reading, so the connection
    /// help drawn under it used to overflow the bottom edge or miss the
    /// window entirely. The issue now takes the reading's place: the title is
    /// on screen whatever shape the window is, the steps are not drawn there
    /// at all, and a hint line says how to get them back.
    /// The fit re-measures only the layout it drew, so the notice used to be
    /// judged against a cached default tuned to a seven-character value and
    /// lost the stacked layout in windows that plainly had the height for it.
    /// A 3:2 window stacks the title under the dashes; a strip lays them out
    /// side by side.
    #[test]
    fn the_notice_stacks_unless_the_window_is_a_strip() {
        let placed = |w: f32, h: f32| {
            let mut app = stalled_app(BigMeterMode::Full);
            let nodes = meter_nodes(&mut app, w, h);
            let bars = text_bounds(&nodes, crate::NO_DATA).expect("the dashes are drawn");
            let title = text_bounds(&nodes, "No response from meter").expect("the title is drawn");
            (bars, title)
        };

        let (bars, title) = placed(1920.0, 1280.0);
        assert!(
            title.top() >= bars.bottom(),
            "3:2 window: the title {title:?} should sit under the dashes {bars:?}"
        );

        let (bars, title) = placed(1920.0, 435.0);
        assert!(
            title.left() >= bars.right(),
            "strip window: the title {title:?} should sit beside the dashes {bars:?}"
        );
    }

    #[test]
    fn the_big_meter_puts_the_connection_issue_in_the_readout() {
        for mode in [BigMeterMode::Full, BigMeterMode::Minimal] {
            for (w, h) in [(1920.0, 1280.0), (1920.0, 435.0), (400.0, 300.0)] {
                let case = format!("{mode:?} at {w}x{h}");
                let mut app = stalled_app(mode);
                let nodes = meter_nodes(&mut app, w, h);
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));

                let title = text_bounds(&nodes, "No response from meter")
                    .unwrap_or_else(|| panic!("{case}: the issue never reached the readout"));
                assert!(
                    screen.contains_rect(title),
                    "{case}: the title {title:?} hangs out of {screen:?}"
                );
                assert!(
                    !shows_text(&nodes, BODY_PHRASE),
                    "{case}: the steps were drawn under a window-filling reading"
                );
                let hint = text_bounds(&nodes, "for details")
                    .unwrap_or_else(|| panic!("{case}: nothing says how to reach the steps"));
                assert!(
                    screen.contains_rect(hint),
                    "{case}: the hint {hint:?} hangs out of {screen:?}"
                );
            }
        }
    }

    /// With the big meter off, the readout and the help below it are exactly
    /// what they always were — the placeholder says "No reading" and the
    /// steps are on screen, not hidden behind a hover. Both layouts that keep
    /// the help: the reading column, and the meter-only branch the hidden
    /// panels also lead to.
    #[test]
    fn the_normal_layout_still_draws_the_whole_help() {
        let mut app = stalled_app(BigMeterMode::Off);
        let column = frame_nodes(&mut app, 1000.0, 700.0, |app, ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, ContentLayout::Wide);
            });
        });

        // The case the meter-only branch also serves: both panels hidden.
        app.settings.show_graph = false;
        app.settings.show_recording = false;
        let meter = meter_nodes(&mut app, 1000.0, 700.0);

        for (name, nodes) in [("the reading column", column), ("the big meter off", meter)] {
            assert!(
                shows_text(&nodes, "No reading"),
                "{name}: the readout stopped saying \"No reading\""
            );
            assert!(
                shows_text(&nodes, "No response from meter"),
                "{name}: the help lost its title"
            );
            assert!(
                shows_text(&nodes, BODY_PHRASE),
                "{name}: the help lost its steps"
            );
        }
    }
}
