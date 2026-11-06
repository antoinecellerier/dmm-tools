//! The application: the [`App`] state every panel reads and writes, and the
//! per-frame `ui` that lays the panels out.
//!
//! The concerns live in submodules — [`appearance`] (fonts, theme, zoom),
//! [`connection`] and [`messages`] (the acquisition thread and its channel),
//! [`connection_issue`] (why there is nothing to read, as text),
//! [`capture`] (the reading pipeline and the stores it fills),
//! [`held_reading`], [`plot_input`], [`top_bar`], [`toast`], [`controls`],
//! [`layout`] (the reading column), [`meter_fit`] (the big meter's sizing
//! arithmetic), [`stats_panel`], [`recording_panel`], [`marker_list`],
//! [`export`], [`import`], [`transform_ui`], [`shortcuts`],
//! [`shortcut_help`], [`update_check`] and [`whats_new`] — all of which add
//! methods to the one [`App`] declared here, so no panel owns state of its
//! own.

mod alarm;
mod appearance;
mod capture;
mod connection;
mod connection_issue;
mod controls;
mod export;
mod held_reading;
mod import;
mod language;
mod layout;
mod marker_list;
mod messages;
mod meter_fit;
mod plot_input;
mod recording_panel;
mod shortcut_help;
mod shortcuts;
pub(crate) use shortcuts::key_name;
mod stats_panel;
mod system_fonts;
mod task;
#[cfg(test)]
pub(crate) mod test_support;
mod toast;
mod top_bar;
mod transform_ui;
pub(crate) mod update_check;
mod whats_new;

use dmm_lib::measurement::Measurement;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::{Choice, MeterKeys, Setting, registry};
use dmm_lib::transform::Transform;
use eframe::egui;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::a11y::ResponseA11yExt;
use crate::display;
use crate::graph::Graph;
use crate::settings::{
    DeferredSave, SIDE_PANEL_MAX_WIDTH, SIDE_PANEL_MIN_WIDTH, SaveDue, Settings, ThemeChoice,
    ThemeMode,
};
use appearance::{UiColorKey, font_definitions, frame_panel, install_styles, on_frame};
use capture::Capture;
use connection::RemoteCommand;
use connection_issue::ConnectionIssue;
use export::PendingExport;
use layout::ContentLayout;
use meter_fit::{FitInputs, MeterFit, WindowContent};
use recording_panel::RecordingPanel;
use toast::Toast;
use transform_ui::TransformEditor;

/// How often a recording redraws for its elapsed-time label when nothing else
/// does (paused, or a silent meter). Real time, not the session clock: the
/// label rounds to whole seconds, and a quarter-second tick keeps it at most
/// that late whatever the clock's scale.
const RECORDING_LABEL_TICK: std::time::Duration = std::time::Duration::from_millis(250);

use connection::{ConnectedMeter, DmmMessage, ThreadControl};

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
    /// The acquisition thread is opening the meter: detecting it under
    /// Auto-detect, or connecting to the one picked. Not connected: nothing
    /// is identified yet, and a failure drops back to `Disconnected`.
    Connecting,
    Connected,
    Reconnecting,
}

impl ConnectionState {
    /// Whether a meter has answered on this connection: connected, or
    /// retrying a link that was up. Not while the first open runs.
    pub(super) fn is_linked(&self) -> bool {
        matches!(self, Self::Connected | Self::Reconnecting)
    }
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
    /// What last frame measured of the help, for this one to pick its
    /// columns from.
    layout: shortcut_help::HelpLayout,
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
    /// Last applied theme (to avoid re-setting every frame).
    theme: Option<ThemeMode>,
    /// Last applied UI chrome colors, to avoid per-frame Visuals mutation.
    ui_colors: Option<UiColorKey>,
    /// Last minimum window size pushed to the windowing system, and the
    /// pixels per point it was sent at, so the viewport command is only
    /// re-sent when either changes.
    min_size: Option<(egui::Vec2, f32)>,
    /// Window size and grow target of the last request sent while the window
    /// was below its minimum.
    grow_request: Option<(egui::Vec2, egui::Vec2)>,
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
    // `auto` names no row, so Auto-detect resolves to none.
    registry::resolve_listing(family).map(|l| l.device)
}

/// The selected device as `device_family` names it, resolved once per value
/// of that field.
///
/// Keyed on the value itself, so any write to the field, wherever it
/// happens, is picked up by the next read: nothing has to remember to
/// refresh it, and it cannot go stale. A read costs comparing that short
/// string; the registry and the protocol factory run only when it changed.
#[derive(Default)]
struct SelectedDevice {
    family: String,
    listing: Option<registry::Listing>,
    /// Two render paths need it every frame, and building a protocol to read
    /// it allocates — the UT61E+ factory boxes a device table and reserves an
    /// rx buffer.
    profile: Option<dmm_lib::protocol::DeviceProfile>,
}

impl SelectedDevice {
    fn for_family(family: &str) -> Self {
        // `auto` names no row, so Auto-detect resolves to none.
        let listing = registry::resolve_listing(family);
        Self {
            family: family.to_string(),
            listing,
            profile: listing.map(|l| *(l.device.new_protocol)().profile()),
        }
    }
}

/// The live link to a meter: its state, what the connected protocol told us
/// about itself, and the channels and flags shared with the acquisition
/// thread.
pub(super) struct Connection {
    pub(super) state: ConnectionState,
    /// What the connected meter told us about itself. Kept through a lost
    /// link until the reconnect reports afresh; `None` once the user
    /// disconnects, so under Auto-detect the next connect asks the cable
    /// again.
    pub(super) meter: Option<ConnectedMeter>,
    /// Values the meter can be switched to for each setting the readout
    /// draws, as last listed by the acquisition thread.
    pub(super) choices: SettingChoices,
    /// Acquisition is halted: the thread stops reading the meter, though
    /// device commands still reach it ([`App::set_paused`]). Not the
    /// live-view scroll-lock.
    pub(super) paused: bool,
    pub(super) last_error: Option<ConnectionIssue>,
    /// Consecutive timeout count (0 = not waiting).
    pub(super) waiting_timeouts: u32,
    /// Reconnect attempt count (0 = not reconnecting). Populated from the
    /// background thread while the state is `Reconnecting`.
    pub(super) reconnect_attempt: u32,
    /// Last reconnect failure message, if any, worded as it arrived.
    pub(super) reconnect_last_error: Option<String>,
    /// The last reconnect attempt found another cable, not this session's.
    pub(super) waiting_for_own_cable: bool,
    /// The acquisition thread's channels; `None` while none runs.
    worker: Option<Worker>,
    /// The message channel of the last thread stopped, which may still be
    /// finishing an open: the next thread waits for it to close before
    /// opening, and quitting waits for it too.
    stopping: Option<mpsc::Receiver<DmmMessage>>,
    /// Reconnect on next frame (device selection changed while connected).
    pub(super) needs_reconnect: bool,
    /// The replay handed out its last frame; nothing more will come until
    /// the next connect.
    pub(super) ended: bool,
    /// The acquisition thread was told no meter, so every open, reconnects
    /// included, identifies whichever meter answers.
    pub(super) detecting: bool,
    /// What to tell the user about readings arriving two at a time, until
    /// dismissed or the link goes ([`App::show_late_readings`]).
    pub(super) late_readings: Option<dmm_lib::transport::LateReadings>,
}

/// The app's ends of a running acquisition thread's channels, set up
/// together on Connect and dropped together on Disconnect.
struct Worker {
    rx: mpsc::Receiver<DmmMessage>,
    ctrl_tx: mpsc::Sender<ThreadControl>,
    /// Stop request, readable without consuming a channel message so the
    /// acquisition thread's pacing sleep can bail out on it mid-tick.
    /// `ctrl_tx` stays the control path; this is the wake signal.
    stop_flag: Arc<AtomicBool>,
    cmd_tx: mpsc::Sender<RemoteCommand>,
}

impl Worker {
    /// Raise the flag, then hang up: the thread may be mid-sleep, and the
    /// flag is what cuts that short; dropping the control sender ends any
    /// wait on the channel and the loop itself. The message receiver comes
    /// back, for a caller that waits for the thread to end.
    fn stop(self) -> mpsc::Receiver<DmmMessage> {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.rx
    }

    /// A worker that only hands over messages: its control and command
    /// channels lead nowhere.
    #[cfg(test)]
    fn reading(rx: mpsc::Receiver<DmmMessage>) -> Self {
        Self {
            rx,
            ctrl_tx: mpsc::channel().0,
            stop_flag: Arc::default(),
            cmd_tx: mpsc::channel().0,
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            meter: None,
            choices: SettingChoices::default(),
            paused: false,
            last_error: None,
            waiting_timeouts: 0,
            reconnect_attempt: 0,
            reconnect_last_error: None,
            waiting_for_own_cable: false,
            worker: None,
            stopping: None,
            needs_reconnect: false,
            ended: false,
            detecting: false,
            late_readings: None,
        }
    }
}

impl Connection {
    /// Stop the acquisition thread and forget everything about the link.
    /// Nothing is connected, so nothing is identified: under Auto-detect the
    /// next connect asks the cable again. The waiting count goes too: only an
    /// incoming measurement clears it otherwise, and there won't be one.
    /// Only the error that ended the link, a pending reconnect and the
    /// stopped thread's channel survive.
    fn hang_up(&mut self) {
        let stopping = match self.worker.take() {
            Some(worker) => Some(worker.stop()),
            None => self.stopping.take(),
        };
        *self = Self {
            last_error: self.last_error.take(),
            needs_reconnect: self.needs_reconnect,
            stopping,
            ..Self::default()
        };
    }

    /// What the readout dropdowns offer: the listed choices, and the
    /// meter's function keys.
    pub(super) fn readouts(&self) -> display::ReadoutChoices<'_> {
        display::ReadoutChoices {
            mode: &self.choices.mode,
            range: &self.choices.range,
            keys: self.meter_keys().functions,
        }
    }

    /// The registry entry actually connected — the one the user named, or
    /// the one detection settled on. `None` while nothing is connected.
    pub(super) fn detected(&self) -> Option<&'static registry::SelectableDevice> {
        self.meter.as_ref().and_then(|m| m.device)
    }

    /// The row the connected meter goes by ([`ConnectedMeter::listing`]).
    pub(super) fn listing(&self) -> Option<registry::Listing> {
        self.meter.as_ref().and_then(|m| m.listing)
    }

    /// How far the connected protocol is verified; `Verified` while nothing
    /// is connected, so no badge shows.
    pub(super) fn stability(&self) -> dmm_lib::protocol::Stability {
        self.meter
            .as_ref()
            .map_or(dmm_lib::protocol::Stability::Verified, |m| m.stability)
    }

    /// What the meter is answering over. `None` while disconnected, and for
    /// the mock.
    pub(super) fn link(&self) -> Option<dmm_lib::transport::Link> {
        self.meter.as_ref().and_then(|m| m.link)
    }

    /// Commands the connected protocol supports; none while disconnected.
    pub(super) fn supported_commands(&self) -> &[String] {
        self.meter
            .as_ref()
            .map_or(&[], |m| m.supported_commands.as_slice())
    }

    /// The connected protocol's function and context keys; none while
    /// disconnected.
    pub(super) fn meter_keys(&self) -> MeterKeys {
        self.meter
            .as_ref()
            .map_or(MeterKeys::NONE, |m| m.meter_keys)
    }
}

pub struct App {
    pub(super) settings: Settings,
    /// The language the window speaks. An `Arc` so a deferred viewport
    /// (What's New) can carry it to its own thread.
    pub(crate) l10n: std::sync::Arc<dmm_shared::l10n::Localizer>,
    /// The languages on offer and what follows the one spoken.
    speech: language::Speech,
    pub(super) settings_open: bool,
    /// Reads of the user's themes folder, started when Settings opens.
    theme_scan: appearance::ThemeScan,
    /// A settings write waiting for a control to stop moving (colour edits).
    settings_save: DeferredSave,

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
    /// The alarm judging every reading against the **Alarm** row's limits,
    /// while any are set. Session-only, as the transform is.
    alarm: Option<dmm_lib::alarm::Alarm>,
    /// Draft text for the **Alarm** row.
    alarm_editor: alarm::AlarmEditor,

    graph: Graph,
    /// The session statistics, the sample buffer and its export layouts:
    /// what [`capture::Capture::ingest`] fills beside the graph.
    capture: Capture,
    /// Markers the user placed on readings, kept while the graph or the
    /// sample buffer holds their reading.
    markers: crate::markers::Markers,
    /// The `--replay` recording or the imported file the session holds
    /// instead of a meter's readings.
    source: import::SessionSource,
    marker_list: marker_list::MarkerList,
    /// Time base the session's readings are stamped with — real unless a
    /// `--mock-clock-*` flag was given. Cloned into the acquisition thread so
    /// the mock's waveform, the pacing loop and anything here that measures
    /// session age (the recording duration) agree. UI cadence — toasts,
    /// repaint, control-channel waits — stays on real time.
    clock: dmm_lib::Clock,

    /// What `settings.shared.device_family` names, read through
    /// [`Self::selection`].
    selection: std::cell::RefCell<SelectedDevice>,
    recording_panel: RecordingPanel,
    first_frame: bool,
    /// A `ui` pass ran since the last `logic` call; see [`Self::logic_pass`].
    ui_ran: bool,
    /// The window is a native Wayland surface, where the compositor owns
    /// stacking: xdg-shell has no keep-above request, so winit drops
    /// `WindowLevel` commands and "Always on top" cannot work.
    on_wayland: bool,
    applied: AppliedChrome,
    /// Transient status toast.
    toast: Option<Toast>,
    /// The Save as theme row under Customize colors.
    theme_save: controls::theme_save::ThemeSave,
    /// The export waiting on its save dialog or its write, if any.
    export: Option<PendingExport>,
    meter_fit: MeterFit,
    /// Transient big meter mode (not persisted to settings).
    big_meter_mode: BigMeterMode,
    shortcut_help: ShortcutHelp,
    whats_new: WhatsNew,
    /// Whether a newer release is out; off unless [`App::new`] turns it on.
    update_check: update_check::UpdateCheck,
}

impl App {
    /// `settings` are loaded by `main`, which sizes the window from them
    /// before this runs, and so is `l10n`, the language they choose, which
    /// `main` prints `--help` in.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        cli: crate::CliOverrides,
        l10n: Arc<dmm_shared::l10n::Localizer>,
    ) -> Self {
        install_styles(&cc.egui_ctx);
        cc.egui_ctx.set_fonts(font_definitions());
        // Asked once, here: the display handle says which backend the window
        // actually got, where `WAYLAND_DISPLAY` only says which one is on
        // offer.
        let on_wayland = cc
            .display_handle()
            .is_ok_and(|handle| matches!(handle.as_raw(), RawDisplayHandle::Wayland(_)));
        let update_notice = cli.update_notice.clone();
        let mut app = Self::from_cli(settings, cli);
        app.on_wayland = on_wayland;
        // Here and not in `from_settings`, so tests speak English whatever
        // the desktop does, and install no system fonts.
        app.start_speaking(&cc.egui_ctx, l10n);
        // Here and not in `from_settings`, so a test build can never reach
        // the network or the cache file.
        app.update_check = update_check::UpdateCheck::new(update_notice);
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
            settings.overrides.mock_mode = Some(settings.shared.mock_mode.clone());
            // The label, not what was typed: the Settings row matches on
            // labels, so an alias (`--mock-mode temp_dual`) would leave the
            // row showing no selection at all.
            settings.shared.mock_mode = mock_mode.label().to_string();
        }
        if let Some(theme) = cli.theme {
            settings.overrides.theme = Some((settings.theme, settings.named_theme.clone()));
            match theme {
                ThemeChoice::Mode(mode) => {
                    settings.theme = mode;
                    // A named theme wins over the mode, so asking for a mode
                    // has to set it aside for the session.
                    settings.named_theme = None;
                }
                ThemeChoice::Named(name) => settings.named_theme = Some(name),
            }
        }
        // Only the off switch is a flag: there is nothing to force on, the
        // setting already is.
        if cli.no_bluetooth {
            settings.overrides.bluetooth = Some(settings.shared.bluetooth);
            settings.shared.bluetooth = false;
        }
        settings.overrides.adapter = cli.adapter;
        let mut app = Self::from_settings(settings, cli.clock);
        app.source.replay = cli.replay;
        app.source.cli_import = cli.import;
        app
    }

    /// The app state for `settings` on `clock`, before any frame. Separate
    /// from [`App::new`] so tests can build one without an eframe context.
    fn from_settings(settings: Settings, clock: dmm_lib::Clock) -> Self {
        let mut graph = Graph::new();
        // One setting bounds both stores of the sample stream.
        graph.set_max_points(settings.max_samples);
        graph.hide_overlays(&settings.hidden_series);
        graph.histogram_sizes = (settings.histogram_width, settings.histogram_height);
        let capture = Capture::new(settings.max_samples);
        let selection = SelectedDevice::for_family(&settings.shared.device_family);
        let settings_save = DeferredSave::new(&settings);
        Self {
            settings,
            l10n: crate::l10n::english(),
            speech: language::Speech::default(),
            settings_open: false,
            theme_scan: appearance::ThemeScan::default(),
            settings_save,
            connection: Connection::default(),
            last_measurement: None,
            held: held_reading::HeldReading::default(),
            transform: Transform::default(),
            transform_editor: TransformEditor::default(),
            alarm: None,
            alarm_editor: alarm::AlarmEditor::default(),
            graph,
            capture,
            markers: crate::markers::Markers::default(),
            source: import::SessionSource::default(),
            marker_list: marker_list::MarkerList::default(),
            clock,
            selection: std::cell::RefCell::new(selection),
            recording_panel: RecordingPanel::default(),
            first_frame: true,
            ui_ran: false,
            // Only `App::new` has a window to ask; tests build without one.
            on_wayland: false,
            applied: AppliedChrome::default(),
            toast: None,
            theme_save: Default::default(),
            export: None,
            meter_fit: MeterFit::new(),
            big_meter_mode: BigMeterMode::Off,
            shortcut_help: ShortcutHelp::default(),
            whats_new: WhatsNew::default(),
            update_check: update_check::UpdateCheck::default(),
        }
    }

    /// The selected device's row and profile, resolved again only when
    /// `device_family` changed since the last read ([`SelectedDevice`]).
    fn selection(
        &self,
    ) -> (
        Option<registry::Listing>,
        Option<dmm_lib::protocol::DeviceProfile>,
    ) {
        let family = &self.settings.shared.device_family;
        let mut selection = self.selection.borrow_mut();
        if selection.family != *family {
            *selection = SelectedDevice::for_family(family);
        }
        (selection.listing, selection.profile)
    }

    /// Profile of the selected device. `None` under Auto-detect: no meter is
    /// named, so there is no profile to describe until one answers — what
    /// the connection then reports stands in ([`Connection::detected`]).
    fn selected_profile(&self) -> Option<dmm_lib::protocol::DeviceProfile> {
        self.selection().1
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
            self.mark_data_loss();
            self.held.clear();
        }
        if let Some(worker) = &self.connection.worker {
            let _ = worker.ctrl_tx.send(ThreadControl::SetPaused(paused));
        }
    }

    pub(super) fn send_command(&self, cmd: &str) {
        if let Some(worker) = &self.connection.worker {
            let _ = worker.cmd_tx.send(RemoteCommand::Named(cmd.to_string()));
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

    /// Whether the window shows the meter alone, sized to fill it: the big
    /// meter, or the graph and recording both hidden in Settings.
    pub(super) fn meter_only(&self) -> bool {
        self.big_meter_mode != BigMeterMode::Off
            || (!self.settings.show_graph && !self.settings.show_recording)
    }

    /// Where a Scale or Alarm chip greyed out in a meter-only view can be
    /// set up: the layout with a row for its fields.
    pub(super) fn chip_setup_hint(&self, ctx: &egui::Context) -> String {
        if !self.settings.show_graph && !self.settings.show_recording {
            dmm_shared::tr!(self.l10n, "alarm-scale-setup-in-settings").into_owned()
        } else {
            let keys = shortcuts::Shortcut::CycleBigMeter.keys(ctx, &self.l10n);
            dmm_shared::tr!(self.l10n, "alarm-scale-setup-in-normal-layout", keys = keys)
                .into_owned()
        }
    }

    /// Ask the meter to switch `setting` to one of the values
    /// `connection.choices` listed. A refusal comes back as a toast.
    pub(super) fn select(&mut self, setting: Setting, id: u16) {
        if let Some(worker) = &self.connection.worker {
            let _ = worker.cmd_tx.send(RemoteCommand::Select(setting, id));
        }
        // Only the mode pin: `settings.shared.mock_mode` names a scenario, and a
        // range pick leaves the mock in the scenario it is already pinned to.
        if setting == Setting::Mode {
            self.repin_mock(id);
        }
    }

    /// Keep the Settings row's mock pin truthful after a dropdown pick.
    ///
    /// `settings.shared.mock_mode` is the scenario the mock is pinned to at connect;
    /// a pick moves the mock's live scenario without a reconnect, so a pinned
    /// mock is re-pinned to the scenario picked. No `needs_reconnect`: the mock has
    /// already switched, and a reconnect would restart it. An auto-cycling
    /// mock (empty pin) is left alone — it carries on cycling from the picked
    /// scenario, so the row stays right. The SELECT button has the same
    /// desync and is left alone: the GUI cannot learn which scenario the
    /// mock cycled to.
    fn repin_mock(&mut self, id: u16) {
        if self
            .selected_device()
            .is_none_or(|d| d.id != dmm_lib::mock::MOCK.id)
            || self.settings.shared.mock_mode.is_empty()
        {
            return;
        }
        let Some(mode) = MockMode::from_choice_id(id) else {
            return;
        };
        self.settings.shared.mock_mode = mode.label().to_string();
        // An explicit choice, as in the Settings row: it replaces a
        // `--mock-mode` override rather than being saved under it.
        self.settings.overrides.mock_mode = None;
    }

    /// The meter the user named, or `None` when the one on the cable is to be
    /// identified instead.
    fn selected_device(&self) -> Option<&'static registry::SelectableDevice> {
        self.selection().0.map(|l| l.device)
    }

    /// [`Self::selected_device`] by the name the user picked it by: the UNI-T
    /// UT217A, not the OWON CMS061 entry it opens.
    fn picked_listing(&self) -> Option<registry::Listing> {
        self.selection().0
    }

    /// The row the session calls its meter by, the counterpart of
    /// [`Self::active_device`]: the pick, else the connected meter's.
    fn active_listing(&self) -> Option<registry::Listing> {
        self.picked_listing().or(self.connection.listing())
    }

    /// The meter this session is actually talking about: the one the user
    /// named, else the one detection found. `None` until a meter answers
    /// under Auto-detect.
    fn active_device(&self) -> Option<&'static registry::SelectableDevice> {
        self.selected_device().or(self.connection.detected())
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
        let l10n = &self.l10n;
        match self.connection.reconnect_attempt {
            0 => dmm_shared::tr!(l10n, "top-bar-reconnecting"),
            attempt => dmm_shared::tr!(l10n, "top-bar-reconnecting-attempt", attempt = attempt),
        }
        .into_owned()
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
        // The sections and the prose under them as one string: the tooltip is
        // the only place a big-meter session can read them.
        let tooltip = notice.as_ref().map(|n| n.help_text(&self.l10n));
        // The keys rendered by the context rather than spelled out: the
        // binding uses `Modifiers::COMMAND`, which is Cmd on macOS.
        let hint = notice.is_some().then(|| {
            let keys = shortcuts::Shortcut::CycleBigMeter.keys(ctx, &self.l10n);
            dmm_shared::tr!(self.l10n, "reading-big-meter-hint", keys = keys).into_owned()
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
                transform_is_identity: self.transform.is_identity(),
                alarm_set: self.alarm.is_some(),
                notice_kind: notice.as_ref().map(|n| n.kind),
                language: self.l10n.language().id,
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
                    let cx = display::ReadingCtx {
                        l10n: &self.l10n,
                        tc: &tc,
                        state: self.reading_state(),
                        choices: self.connection.readouts(),
                    };
                    let (scale, measured_ratios, picked) = display::show_reading_large(
                        ui,
                        &cx,
                        self.last_measurement.as_ref(),
                        display::ReadingFit {
                            base_content_height: content_h,
                            ratios: &self.meter_fit.reading_ratios,
                        },
                        no_reading,
                    );
                    if let Some(pick) = picked {
                        self.apply_pick(pick);
                    }
                    let after_reading = ui.cursor().top();

                    if !minimal {
                        // The big-meter toggle sits in the panel corner
                        // here, not on the row: nothing to keep clear of.
                        // No Scale or Alarm row: a row of fields drawn at the
                        // big meter's size takes room the fit gave the
                        // reading. Their chips switch them on and off.
                        self.show_remote_controls(ui, scale, 0.0);
                    }

                    if self.big_meter_mode == BigMeterMode::Off {
                        // Only here: in Full and Minimal the notice is
                        // already in the readout, and the help drawn under a
                        // window-filling reading would land off-screen.
                        self.show_connection_help(ui);
                        self.show_late_readings(ui);
                        self.show_specs_section_inline(ui, scale);

                        if self.settings.show_stats {
                            ui.add_space(12.0 * scale);
                            ui.separator();
                            self.show_stats_section(ui, false, scale);
                        }
                    }

                    // Re-measure until the fit settles, a pass a frame, each
                    // drawn with what the one before measured. Each asks for
                    // the next frame: a paused meter sends no readings to
                    // draw one, and a fit left halfway can clip the buttons.
                    if needs_recalc && scale > 0.0 {
                        let total_below_reading = ui.cursor().top() - after_reading;
                        self.meter_fit.record_pass(
                            &fit_inputs,
                            total_below_reading / scale,
                            measured_ratios,
                        );
                        if self.meter_fit.needs_recalc(&fit_inputs) {
                            ui.ctx().request_repaint();
                        }
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

    /// The first frame's one-off work: the window's always-on-top and
    /// maximized state, the import `--import` named or the auto-connect,
    /// What's New after an upgrade, and the toast for a settings file that
    /// could not be loaded.
    fn on_first_frame(&mut self, ctx: &egui::Context) {
        if self.settings.always_on_top && !self.on_wayland {
            self.apply_always_on_top(ctx);
        }
        // `main` builds the window without its title bar and maximized.
        // Maximize again all the same: eframe creates the window hidden,
        // and X11 window managers ignore a maximize request made before
        // the window is shown.
        if self.settings.window_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        if let Some(path) = self.source.cli_import.take() {
            // The file is the session: no meter is opened beside it.
            self.import_file(ctx, path);
        } else if self.settings.auto_connect {
            self.connect(ctx);
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
        // Last, so a toast raised above can't replace it.
        self.settings_load_toast();
    }

    /// Tell the user, once, that their settings file could not be loaded and
    /// the defaults are in use — the log alone would go unseen until the
    /// first save had written over it.
    fn settings_load_toast(&mut self) {
        let Some(problem) = self.settings.load_problem.take() else {
            return;
        };
        let message = match problem.kept {
            Some(copy) => {
                let path = copy.display().to_string();
                dmm_shared::tr!(self.l10n, "settings-load-failed-kept", path = path)
            }
            None => {
                let path = problem.path.display().to_string();
                dmm_shared::tr!(self.l10n, "settings-load-failed", path = path)
            }
        };
        self.toast = Some(Toast::error(message));
    }

    /// Keep the window's minimum size, and its size, fitting what this frame
    /// shows: `minimal` big meter, the meter alone, or the panels.
    fn update_min_window_size(&mut self, ctx: &egui::Context, minimal: bool, meter_only: bool) {
        // Reading dimensions come from cached ratios × minimum big meter
        // font size; top bar widths come from previous-frame measurements.
        let bar_left_w: f32 = ctx.data(|d| d.get_temp(top_bar::left_width_id()).unwrap_or(300.0));
        let bar_right_w: f32 = ctx.data(|d| d.get_temp(top_bar::right_width_id()).unwrap_or(120.0));
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
        // set_zoom_factor().
        if let Some(min) = meter_fit::min_size_request(
            min_size,
            ctx.pixels_per_point(),
            &mut self.applied.min_size,
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));
        }
        // If the window is smaller than the new minimum (e.g. after exiting
        // minimal mode), grow it to fit. Once per change: a tiling window
        // manager may refuse, and every viewport command asks for another
        // frame, so an unguarded request redraws at the refresh rate.
        if let Some(grown) = meter_fit::grow_request(
            ctx.content_rect().size(),
            min_size,
            &mut self.applied.grow_request,
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(grown));
        }
    }
}

impl App {
    /// What `App::logic` does. eframe calls `logic` before every `ui`, and
    /// alone while the window is minimized or covered, when it skips `ui`.
    /// Two `logic` calls in a row mean no frame is drawing, so this one takes
    /// the readings in: the channel stays short and the recording bounded.
    /// After a `ui` pass it leaves them to the next frame's, which drains
    /// after the shortcuts so `N` marks the reading on screen.
    fn logic_pass(&mut self) {
        if !std::mem::replace(&mut self.ui_ran, false) {
            self.drain_messages();
            self.trim_markers();
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.logic_pass();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui_ran = true;
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
        self.tell_screen_reader_language(&ctx);
        self.poll_font_check();
        self.poll_theme_scan();
        self.poll_theme_save();
        self.apply_theme(&ctx);
        self.apply_color_overrides(&ctx);
        self.apply_zoom(&ctx);
        self.handle_keyboard_shortcuts(&ctx);
        self.handle_shortcut_help_keys(&ctx);
        // Shortcuts first: `N` marks the reading the user saw, not one that
        // arrives with this frame.
        self.drain_messages();
        self.poll_import_dialog(&ctx);
        self.step_import(&ctx);
        self.trim_markers();
        self.poll_export(&ctx);
        // From a frame, not from `App::new`: the wgpu-to-glow fallback builds
        // the App twice, and only the one that draws should ask.
        self.update_check
            .poll(&ctx, self.settings.check_for_updates);

        // Auto-reconnect after device selection change
        // Not over an imported session: a device picked in Settings
        // applies at the next Connect, which asks about unsaved markers.
        if self.connection.needs_reconnect {
            self.connection.needs_reconnect = false;
            if !self.source.is_import() {
                self.connect(&ctx);
            }
        }

        // Expire the toast, unless the user closed it first
        if self.toast.as_ref().is_some_and(Toast::expired) {
            self.toast = None;
        }

        if self.first_frame {
            self.first_frame = false;
            self.on_first_frame(&ctx);
        }

        let minimal = self.big_meter_mode == BigMeterMode::Minimal;
        // Where the toast hangs from: under the bar (and under the settings
        // rows it opens), at the window's top edge when there is no bar.
        let mut toast_top = ctx.content_rect().top();
        // The top bar and the wide layout's reading panel sit on the
        // palette's Frame surface, around the graph column.
        let frame_fill = self.settings.theme_colors(ui.visuals().dark_mode).frame();
        if !minimal {
            let top = egui::Panel::top("top_bar")
                .frame(frame_panel(ui.style(), frame_fill))
                .show(ui, |ui| {
                    on_frame(ui, frame_fill);
                    self.show_top_bar(ui, &ctx);
                    self.show_settings_panel(ui);
                });
            toast_top = top.response.rect.bottom();
        }

        // Determine layout mode before panels
        let wide = meter_fit::is_wide(ctx.content_rect().width());

        let meter_only = self.meter_only();

        self.update_min_window_size(&ctx, minimal, meter_only);

        if meter_only {
            self.show_meter_only(ui, &ctx, minimal);
        } else if wide {
            // Wide: left side panel for reading + stats (resizable)
            let reading_panel = egui::Panel::left(layout::READING_PANEL)
                .default_size(self.settings.reading_panel_width)
                .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH)
                .resizable(true)
                .frame(frame_panel(ui.style(), frame_fill))
                .show(ui, |ui| {
                    on_frame(ui, frame_fill);
                    self.show_reading_column_scrolled(ui, ContentLayout::Wide);
                });
            self.reading_panel_resize_a11y(ui, reading_panel.response.rect);

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

        // The recording's elapsed time is the only thing on screen that moves
        // with time alone. Readings, connection notices and export results
        // each ask for their frame; toasts schedule their own.
        if self.capture.recording.is_active() {
            ctx.request_repaint_after(RECORDING_LABEL_TICK);
        }
        self.track_layout(ui);
        // Last, so a change made this frame is seen this frame. A paused or
        // disconnected app draws nothing by itself, so a pending save asks
        // for the frame it is due in.
        let now = std::time::Instant::now();
        self.settings_save.check(&self.settings, now);
        match self.settings_save.poll(now) {
            SaveDue::Idle => {}
            SaveDue::Wait(delay) => ctx.request_repaint_after(delay),
            SaveDue::Now => self.settings.save(),
        }
    }

    /// Quitting inside a deferred save's delay still saves. Not `Drop`: tests
    /// build and drop `App`s, and must never write the real settings file.
    /// The save goes first, so a slow disconnect never loses it.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings_save
            .check(&self.settings, std::time::Instant::now());
        if self.settings_save.take_pending() {
            self.settings.save();
        }
        // `main` returns, and the process exits, as soon as this does.
        self.disconnect_and_wait();
    }
}

/// A context with the app's own styles, for headless tests: laid out on
/// egui's defaults instead, a test measures spacing and text sizes the
/// window never shows.
///
/// It writes shortcuts as Linux does, whatever the host: on a Mac, egui
/// asks the fonts whether they draw `⌘`, and outside a pass there are no
/// fonts yet. A test about macOS sets the OS itself and runs in a pass, as
/// `mac_rows_use_cmd` does.
#[cfg(test)]
pub(crate) fn styled_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_os(egui::os::OperatingSystem::Nix);
    appearance::install_styles(&ctx);
    ctx
}

#[cfg(test)]
impl App {
    /// Whether the settings changed since the last check, as the end of a
    /// frame would find, and so a save is owed; taken.
    fn settings_save_owed(&mut self) -> bool {
        self.settings_save
            .check(&self.settings, std::time::Instant::now());
        self.settings_save.take_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::{a11y, click_frames, headless_frame, node_rect, to_rect};
    use egui::accesskit::{Node, NodeId};

    fn app(device: &str, mock_mode: &str) -> App {
        let mut settings = Settings::default();
        settings.shared.device_family = device.to_string();
        settings.shared.mock_mode = mock_mode.to_string();
        // As after `--mock-mode`: the pin on screen is an override.
        settings.overrides.mock_mode = Some(String::new());
        App::from_settings(settings, dmm_lib::Clock::real())
    }

    /// A settings file that could not be loaded raises one error toast,
    /// naming the copy kept when there is one, else the file.
    #[test]
    fn a_settings_load_problem_is_toasted_once() {
        let file = std::path::PathBuf::from("/config/settings.json");
        let copy = std::path::PathBuf::from("/config/settings.json.bad");
        for (kept, expected) in [
            (
                Some(copy),
                "Your settings file could not be read, so the defaults are in use. \
                 A copy of it is kept as /config/settings.json.bad.",
            ),
            (
                None,
                "Your settings file /config/settings.json could not be read, \
                 so the defaults are in use.",
            ),
        ] {
            let settings = Settings {
                load_problem: Some(crate::settings::LoadProblem {
                    path: file.clone(),
                    kept,
                }),
                ..Settings::default()
            };
            let mut app = App::from_settings(settings, dmm_lib::Clock::real());
            app.settings_load_toast();
            let toast = app.toast.take().expect("no toast");
            assert!(toast.is_error());
            assert_eq!(toast.message, expected);
            app.settings_load_toast();
            assert!(app.toast.is_none(), "raised twice");
        }
    }

    /// The most strings the main scenes may draw without going through the
    /// localizer. Comes down as each panel's text moves into `i18n/`, and
    /// ends at what is meant to stay as it is: brand and model names, key
    /// names, theme names, the version, numbers formatted in Rust (`500K`),
    /// the commands and links a notice quotes, a spec text's English on the
    /// hover of its translation, and egui's own colour picker (Hue, R, G, B,
    /// U8). Two above the Linux count: macOS draws a Minimise row in the
    /// shortcut help.
    const UNTRANSLATED_CEILING: usize = 49;

    /// Every distinct text the whole app draws or names to a screen reader
    /// in `scene`, under the pseudo-locale, that has a word outside the
    /// brackets the pseudo-locale puts around each message.
    ///
    /// Numbers and unit symbols are never translated — a reading's "22V"
    /// range, its "V", a "0.5 s" tick — and the library's labels made of
    /// nothing else come back from the localizer as they are, unbracketed
    /// ([`dmm_shared::l10n::is_units_only`]); text whose words outside the
    /// brackets are all unit symbols is not counted ([`units_only`]).
    ///
    /// Blind spot: English passed into a message as a variable sits inside
    /// its brackets, so a caller that hands a message an English word, or
    /// a whole English sentence, is not counted. Only reading the call
    /// sites finds those.
    fn untranslated_texts(scene: impl Fn(&mut App)) -> std::collections::BTreeSet<String> {
        untranslated_after_clicks(scene, &[], &[])
    }

    /// Where a scene clicks once its first frames have settled, and with
    /// which button, found in the frame before: `None` fails the test.
    type Click = dyn Fn(&App, &egui::FullOutput) -> Option<(egui::Pos2, egui::PointerButton)>;

    /// [`untranslated_texts`] once `clicks` have been made, in order, on a
    /// frame that shows each message of `shows`: what the clicks opened.
    fn untranslated_after_clicks(
        scene: impl Fn(&mut App),
        clicks: &[&Click],
        shows: &[&str],
    ) -> std::collections::BTreeSet<String> {
        let mut settings = Settings {
            auto_connect: false,
            ..Settings::default()
        };
        settings.shared.device_family = "ut61eplus".to_string();
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        let options = dmm_shared::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        app.l10n = std::sync::Arc::new(dmm_shared::l10n::Localizer::new(
            &dmm_shared::l10n::EN_US,
            &crate::l10n::resources(),
            &options,
        ));
        scene(&mut app);
        let mut texts = drawn_texts(&frame_after_clicks(&mut app, clicks));
        for id in shows {
            let shown = app.l10n.message(id, &[]);
            assert!(
                texts.iter().any(|t| t.contains(&*shown)),
                "no {shown:?} drawn: {texts:#?}"
            );
        }
        texts.retain(|t| {
            let mut depth = 0usize;
            let outside: String = t
                .chars()
                .filter(|&c| match c {
                    '[' => {
                        depth += 1;
                        false
                    }
                    ']' => {
                        depth = depth.saturating_sub(1);
                        false
                    }
                    _ => depth == 0,
                })
                .collect();
            outside.chars().any(|c| c.is_ascii_alphabetic()) && !units_only(&outside)
        });
        texts
    }

    /// Whether `text` is nothing but numbers and units, as
    /// [`dmm_shared::l10n::is_units_only`] reads it, except for a lone
    /// letter: an SI prefix or a one-letter unit (M, k, F, S) alone is more
    /// likely a key's name than a unit, so it counts only right after a
    /// number or the `---` of a missing one ("5 V", "2 M", "--- V"), or as
    /// the whole text when it is a unit (a reading's "V").
    fn units_only(text: &str) -> bool {
        const PREFIXES: [&str; 8] = ["p", "n", "\u{b5}", "u", "m", "k", "M", "G"];
        let mut words = Vec::new();
        let (mut before, mut start) = (None, None);
        for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
            if c.is_alphabetic() || c == '\u{b0}' {
                start.get_or_insert((i, before));
                continue;
            }
            if let Some((from, before)) = start.take() {
                words.push((&text[from..i], before));
            }
            if !c.is_whitespace() {
                before = Some(c);
            }
        }
        let lone = |word: &str, before: Option<char>| {
            word.chars().count() == 1
                && !before.is_some_and(|c| c.is_ascii_digit() || c == '-')
                && !(text.trim() == word && !PREFIXES.contains(&word))
        };
        dmm_shared::l10n::is_units_only(text) && !words.iter().any(|&(w, b)| lone(w, b))
    }

    #[test]
    fn a_lone_letter_is_a_unit_only_after_a_number() {
        for units in [
            "22V", "5 V", "0.5 s", "2 M", "100kHz", "\u{b0}C", "V", "--- V",
        ] {
            assert!(units_only(units), "{units}");
        }
        for words in ["M", "k", "F11", "Ctrl+F", "S / V"] {
            assert!(!units_only(words), "{words}");
        }
    }

    /// The fourth frame of `app` in a 1600×1000 window, AccessKit on: by
    /// then the layout has settled.
    fn last_frame(app: &mut App) -> egui::FullOutput {
        frame_after_clicks(app, &[])
    }

    /// [`last_frame`], then each of `clicks` — a press and a release on
    /// frames of their own — and the frames that settle after each.
    fn frame_after_clicks(app: &mut App, clicks: &[&Click]) -> egui::FullOutput {
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let mut frame = eframe::Frame::_new_kittest();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
        let mut run = |app: &mut App, events: Vec<egui::Event>| {
            headless_frame(&ctx, Some(screen), None, events, |ui| {
                eframe::App::ui(app, ui, &mut frame)
            })
        };
        let mut out = run(app, vec![]);
        for _ in 0..3 {
            out = run(app, vec![]);
        }
        for (i, click) in clicks.iter().enumerate() {
            let (pos, button) = click(app, &out).unwrap_or_else(|| panic!("click {i}: no target"));
            let [moved, pressed, released] = click_frames(pos, button);
            run(app, moved);
            run(app, pressed);
            out = run(app, released);
            for _ in 0..4 {
                out = run(app, vec![]);
            }
        }
        out
    }

    /// The centre of the first AccessKit node whose label `label` accepts.
    fn node_centre(out: &egui::FullOutput, label: impl Fn(&str) -> bool) -> Option<egui::Pos2> {
        node_rect(&a11y(out).0, |n| n.label().is_some_and(&label)).map(|r| r.center())
    }

    /// Connected to the mock UT61E+, with a few seconds of its readings
    /// through the pipeline — the graph, the statistics, the
    /// Specifications panel — and a marker on the last.
    fn with_readings(app: &mut App) {
        app.connection.state = ConnectionState::Connected;
        app.connection.meter = Some(ConnectedMeter::test_fixture(Some("mock")));
        let mut dmm =
            dmm_lib::mock::open_mock_mode(dmm_lib::mock::MockMode::DcV).expect("the mock opens");
        let start = app.clock.now();
        for i in 0..50 {
            let mut m = dmm.request_measurement().expect("the mock reads");
            m.timestamp = start + std::time::Duration::from_millis(100 * i);
            let (m, _) = app
                .capture
                .ingest(m, &app.transform, &mut app.graph, &app.connection);
            app.last_measurement = Some(m);
        }
        app.add_marker(false);
        assert!(!app.markers.is_empty(), "the last reading took a marker");
    }

    /// Every distinct text a frame draws or names to a screen reader.
    fn drawn_texts(out: &egui::FullOutput) -> std::collections::BTreeSet<String> {
        fn walk(shape: &egui::Shape, texts: &mut std::collections::BTreeSet<String>) {
            match shape {
                egui::Shape::Text(t) => {
                    texts.insert(t.galley.text().to_string());
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, texts)),
                _ => {}
            }
        }
        let mut texts = std::collections::BTreeSet::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut texts);
        }
        for (_, node) in a11y(out).0 {
            texts.extend(node.label().map(str::to_string));
            texts.extend(node.value().map(str::to_string));
            texts.extend(node.description().map(str::to_string));
        }
        texts
    }

    /// Text that never went through the localizer shows unbracketed under
    /// the pseudo-locale; its count may only come down.
    #[test]
    fn untranslated_text_only_comes_down() {
        let mut found = untranslated_texts(|_| {});
        found.extend(untranslated_texts(|app| app.settings_open = true));
        found.extend(untranslated_texts(|app| app.shortcut_help.open = true));
        found.extend(untranslated_texts(|app| {
            app.last_measurement = Some(labelled_reading());
        }));
        found.extend(untranslated_after_clicks(
            |app| {
                app.connection.last_error = Some(ConnectionIssue::DeviceNotFound {
                    bluetooth_searched: false,
                });
            },
            &[],
            &["not-found-usb"],
        ));
        found.extend(untranslated_after_clicks(
            App::buffer_full_toast,
            &[],
            &["connection-dismiss.a11y"],
        ));
        found.extend(untranslated_after_clicks(
            with_readings,
            &[],
            &["stats-heading", "specs-heading"],
        ));
        // The plot's right-click menu, over the readings.
        let plot: &Click = &|app, out| {
            let title = dmm_shared::tr!(app.l10n, "graph-plot-a11y-title");
            let at = node_centre(out, |l| l.starts_with(&*title))?;
            Some((at, egui::PointerButton::Secondary))
        };
        found.extend(untranslated_after_clicks(
            with_readings,
            &[plot],
            &["graph-menu-mark"],
        ));
        // A colour's editor, open from the Settings panel's Customize
        // colours section.
        let customize: &Click = &|app, out| {
            let header = dmm_shared::tr!(app.l10n, "colors-customize");
            let at = node_centre(out, |l| l == header)?;
            Some((at, egui::PointerButton::Primary))
        };
        let swatch: &Click = &|app, out| {
            let label = crate::theme::PaletteField::Background.label(&app.l10n);
            let at = node_centre(out, |l| l == label)?;
            Some((at, egui::PointerButton::Primary))
        };
        found.extend(untranslated_after_clicks(
            |app| app.settings_open = true,
            &[customize, swatch],
            &["colors-hex"],
        ));
        assert!(
            found.len() <= UNTRANSLATED_CEILING,
            "{} untranslated strings, the ceiling is {UNTRANSLATED_CEILING}: {found:#?}",
            found.len()
        );
        assert!(
            found.len() + 20 > UNTRANSLATED_CEILING,
            "{} untranslated strings: lower UNTRANSLATED_CEILING to match",
            found.len()
        );
    }

    /// A reading with a library label in every place the display has one: a
    /// mode, a named main reading, sub-values with MIN/MAX timestamps, flags,
    /// and a range of digits and a unit only.
    fn labelled_reading() -> Measurement {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::{AuxValue, MainLabel, MeasuredValue};
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                max: true,
                low_battery: true,
                ..StatusFlags::default()
            },
        );
        m.mode = "AC+DC V".into();
        m.range_label = "22V".into();
        m.main_label = Some(MainLabel::Dc);
        let aux = |label: &'static str, secs| AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(0.5),
            unit: "".into(),
            display_raw: Some("0.500".to_string()),
            elapsed_secs: secs,
        };
        m.aux_values = vec![aux("AC", None), aux("Max", Some(3))];
        m
    }

    /// A language whose files translate a few of the library's labels, as
    /// a translator's `lib.ftl` and `lib-names.ftl` would.
    fn french() -> std::sync::Arc<dmm_shared::l10n::Localizer> {
        use dmm_shared::l10n::{Language, Localizer, Options, Resource};
        static FRENCH: Language = Language {
            id: "fr",
            native_name: "Français",
            reviewed: true,
        };
        let lib = Resource {
            locale: "fr",
            path: "lib.ftl",
            source: "mode-v-dc = V CC\nqualifier-mode-rel = { $mode } RÉL\n\
                     aux-max = Maxi\nkey-current = COURANT\n",
        };
        let names = Resource {
            locale: "fr",
            path: "lib-names.ftl",
            source: "flag-hold = MAINTIEN\nmain-label-dc = CC\n",
        };
        let resources = [crate::l10n::english_resources(), vec![lib, names]].concat();
        std::sync::Arc::new(Localizer::new(&FRENCH, &resources, &Options::default()))
    }

    /// An overload in another decade ("OL MΩ" after kΩ readings) leaves the
    /// statistics in kΩ, and the panel says so: their caption is the unit
    /// they are held in, not the one the latest frame names.
    #[test]
    fn the_statistics_keep_their_unit_through_an_overload() {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::MeasuredValue;
        let mut app = App::from_settings(
            Settings {
                auto_connect: false,
                ..Settings::default()
            },
            dmm_lib::Clock::real(),
        );
        app.connection.state = ConnectionState::Connected;
        app.connection.meter = Some(ConnectedMeter::test_fixture(None));
        for (value, unit) in [
            (MeasuredValue::Normal(1.05), "k\u{3a9}"),
            (MeasuredValue::Overload, "M\u{3a9}"),
        ] {
            let mut m = Measurement::test_fixture(value, unit, StatusFlags::default());
            m.mode = "Ohm".into();
            app.deliver(connection::DmmMessage::Measurement(m));
        }
        let texts = drawn_texts(&last_frame(&mut app));
        assert!(
            texts.iter().any(|t| t.ends_with("1.0500 k\u{3a9}")),
            "no kΩ figure in {texts:#?}"
        );
        assert!(
            !texts.iter().any(|t| t.ends_with("1.0500 M\u{3a9}")),
            "a kΩ figure captioned MΩ in {texts:#?}"
        );
    }

    /// The library's labels are drawn in the app's language — a mode, one
    /// the library composed around a qualifier, a flag, the main reading's
    /// name, a sub-value, a key — while a trace hidden under its English
    /// label in the settings stays hidden, and stays saved as it was.
    #[test]
    fn library_labels_are_drawn_in_the_apps_language() {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::{AuxValue, MainLabel, MeasuredValue};
        use dmm_lib::protocol::{MeterKey, MeterKeys};
        const CURRENT: &[MeterKey] = &[MeterKey {
            command: "zero",
            label: "Current",
            in_words: true,
            hover: None,
            applies: |_| true,
        }];
        let mut settings = Settings {
            auto_connect: false,
            ..Settings::default()
        };
        settings.hidden_series.insert("Max".to_string());
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        app.l10n = french();
        app.connection.state = ConnectionState::Connected;
        app.connection.meter = Some(ConnectedMeter {
            supported_commands: vec!["hold".to_string(), "zero".to_string()],
            meter_keys: MeterKeys {
                context: CURRENT,
                ..MeterKeys::NONE
            },
            ..ConnectedMeter::test_fixture(None)
        });
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                ..StatusFlags::default()
            },
        );
        m.mode = "V DC REL".into();
        m.range_label = "6V".into();
        m.main_label = Some(MainLabel::Dc);
        m.aux_values = vec![AuxValue {
            label: "Max".into(),
            value: MeasuredValue::Normal(9.876),
            unit: "".into(),
            display_raw: Some(" 9.876".to_string()),
            elapsed_secs: None,
        }];
        app.deliver(connection::DmmMessage::Measurement(m));

        let out = last_frame(&mut app);
        let texts = drawn_texts(&out);
        for drawn in ["V CC RÉL", "MAINTIEN", "Maxi", "COURANT", "6V"] {
            assert!(texts.contains(drawn), "no {drawn:?} in {texts:#?}");
        }
        assert!(
            texts.iter().any(|t| t.starts_with("CC ")),
            "no main reading named CC in {texts:#?}"
        );
        // HOLD stays: the remote button is the app's own key symbol.
        for english in ["V DC REL", "Max", "Current", "DC"] {
            assert!(!texts.contains(english), "{english:?} drawn in {texts:#?}");
        }
        let chip = a11y(&out)
            .0
            .into_iter()
            .find(|(_, n)| n.label() == Some("Show Maxi trace"))
            .map(|(_, n)| n.toggled())
            .expect("a Show: chip for the sub-value");
        assert_eq!(chip, Some(egui::accesskit::Toggled::False));
        assert_eq!(
            app.settings.hidden_series,
            std::collections::BTreeSet::from(["Max".to_string()])
        );
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
        app.repin_mock(choice_id_of(MockMode::TempDiff));
        assert_eq!(app.settings.shared.mock_mode, "temp-diff");
        assert_eq!(
            app.settings.overrides.mock_mode, None,
            "the pick replaces a --mock-mode override"
        );
        assert!(!app.connection.needs_reconnect);
    }

    /// A change made outside the Settings panel is saved too, with no call
    /// of its own: the end of the frame finds it.
    #[test]
    fn a_change_anywhere_is_owed_a_save() {
        let mut app = app("mock", "temp2");
        assert!(!app.settings_save_owed(), "as loaded");
        app.zoom_in();
        assert!(app.settings_save_owed(), "a zoom shortcut");
        app.select(Setting::Mode, choice_id_of(MockMode::TempDiff));
        assert!(app.settings_save_owed(), "a mock re-pin");
        app.select(Setting::Mode, choice_id_of(MockMode::TempDiff));
        assert!(!app.settings_save_owed(), "the same pick again");
    }

    /// An auto-cycling mock keeps cycling from the picked scenario, so the
    /// row's "Auto (cycle)" stays true and is left alone.
    #[test]
    fn a_pick_leaves_an_auto_cycling_mock_unpinned() {
        let mut app = app("mock", "");
        app.repin_mock(choice_id_of(MockMode::TempDiff));
        assert_eq!(app.settings.shared.mock_mode, "");
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
                no_bluetooth: true,
                ..crate::CliOverrides::none()
            },
        );
        assert!(!app.settings.shared.bluetooth, "this session skips it");
        assert_eq!(app.settings.overrides.bluetooth, Some(true));
    }

    /// `--theme` picks a named theme or a mode for one session. A mode sets
    /// a saved named theme aside, since the theme would otherwise win; both
    /// saved values are kept for [`Settings::save`] to write back.
    #[test]
    fn the_theme_flag_does_not_reach_the_settings_file() {
        let session = |saved: Option<&str>, flag: ThemeChoice| {
            let settings = Settings {
                auto_connect: false,
                theme: ThemeMode::Light,
                named_theme: saved.map(str::to_string),
                ..Settings::default()
            };
            App::from_cli(
                settings,
                crate::CliOverrides {
                    theme: Some(flag),
                    ..crate::CliOverrides::none()
                },
            )
        };

        let app = session(None, ThemeChoice::Named("Midnight".to_string()));
        assert_eq!(app.settings.active_theme().unwrap().name, "Midnight");
        assert_eq!(app.settings.overrides.theme, Some((ThemeMode::Light, None)));

        let app = session(Some("Desert"), ThemeChoice::Mode(ThemeMode::Dark));
        assert_eq!(app.settings.theme, ThemeMode::Dark);
        assert_eq!(app.settings.active_theme(), None);
        assert_eq!(
            app.settings.overrides.theme,
            Some((ThemeMode::Light, Some("Desert".to_string())))
        );
    }

    /// A pick on a real meter has nothing to do with the mock's pin.
    #[test]
    fn a_pick_on_a_real_meter_leaves_the_mock_pin_alone() {
        let mut app = app("ut181a", "temp2");
        app.repin_mock(0x1121);
        assert_eq!(app.settings.shared.mock_mode, "temp2");
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
                replay: Some(crate::ReplaySource {
                    replay: Arc::new(replay),
                    path: std::path::PathBuf::from("dcv-steps.replay"),
                    recorded: std::time::SystemTime::UNIX_EPOCH,
                }),
                ..crate::CliOverrides::none()
            },
        );

        assert_eq!(app.settings.shared.device_family, "ut61eplus");
        assert_eq!(
            app.settings.overrides.device_family.as_deref(),
            Some("mock"),
            "the saved device is the one Settings::save puts back"
        );
        assert!(
            app.source.replay.is_some(),
            "the session plays the recording"
        );
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

    /// A meter picked by another brand's name is called by it, before it
    /// answers and once it has.
    #[test]
    fn a_meter_picked_by_another_brands_name_keeps_it() {
        let mut app = app("ut217a", "");
        assert_eq!(app.selected_device().map(|d| d.id), Some("cms061"));
        let name = |app: &App| app.active_listing().map(|l| l.display_name());
        assert_eq!(name(&app), Some("UNI-T UT217A"));
        let cms061 = registry::find_device("cms061").expect("a registry entry");
        app.connection.meter = Some(ConnectedMeter {
            listing: Some(registry::connected_listing(app.picked_listing(), cms061)),
            ..ConnectedMeter::test_fixture(Some("cms061"))
        });
        assert_eq!(name(&app), Some("UNI-T UT217A"));

        // Picked anew while that meter is still connected: the pick names
        // the session, as it does for `active_device`.
        app.settings.shared.device_family = "ut61eplus".to_string();
        assert_eq!(name(&app), Some("UNI-T UT61E+"));
    }

    /// Auto-detect has no profile of its own, so nothing may assume one: the
    /// stability badge and the connection help both read it every frame.
    #[test]
    fn auto_detect_carries_no_profile_until_a_meter_answers() {
        let mut app = app(registry::AUTO_DEVICE_ID, "");
        assert!(app.selected_device().is_none());
        assert!(app.selected_profile().is_none());
        assert!(app.active_device().is_none(), "nothing connected yet");

        // What `DmmMessage::Connected` does: the meter that answered is the
        // one the top bar names from then on.
        app.connection.meter = Some(ConnectedMeter::test_fixture(Some("ut8803")));
        assert_eq!(app.active_device().map(|d| d.id), Some("ut8803"));

        // And picking a model back out of the picker restores its profile.
        app.settings.shared.device_family = "ut61b+".to_string();
        assert_eq!(
            app.selected_profile().map(|p| p.display_name()),
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
        app.connection.last_error = Some(ConnectionIssue::NoResponse);
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
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
        let mut nodes = Vec::new();
        for _ in 0..6 {
            let out = headless_frame(&ctx, Some(screen), None, vec![], |ui| draw(app, ui));
            nodes = a11y(&out).0;
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
        Some(to_rect(node.bounds().expect("a drawn label has bounds")))
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

    /// Bounds of the plain label whose text is exactly `text`: the live
    /// region's sentence and the top bar's longer strings never match.
    fn label_exact(nodes: &[(NodeId, Node)], text: &str) -> Option<egui::Rect> {
        let (_, node) = nodes.iter().find(|(_, n)| n.value() == Some(text))?;
        Some(to_rect(node.bounds().expect("a drawn label has bounds")))
    }

    /// The user's case: a T2-T1 difference with T1 and T2 under it, in a
    /// wide, short minimal-mode window. Through the real fit loop the
    /// sub-values settle beside the value, before the mode line, and the
    /// row's far end — the AUTO badge — stays inside the window; the same
    /// with a UT181A's four MIN/MAX rows, which outgrow the value.
    #[test]
    fn a_wide_minimal_window_settles_with_the_sub_values_beside_the_value() {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
        let row = |label: &'static str, digits: &str| AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(digits.trim().parse().unwrap_or(0.0)),
            unit: "".into(),
            display_raw: Some(digits.to_string()),
            elapsed_secs: None,
        };
        let cases = [
            vec![row("T1", "20.500"), row("T2", "22.382")],
            vec![
                row("Max", "24.117"),
                row("Average", "22.050"),
                row("Min", "20.003"),
                row("Raw", "21.996"),
            ],
        ];
        for (w, h) in [(1998.0, 275.0), (1200.0, 200.0)] {
            for rows in &cases {
                let case = format!("{} rows at {w}x{h}", rows.len());
                let mut app = App::from_settings(
                    Settings {
                        auto_connect: false,
                        ..Settings::default()
                    },
                    dmm_lib::Clock::real(),
                );
                app.big_meter_mode = BigMeterMode::Minimal;
                let mut m = Measurement::test_fixture(
                    MeasuredValue::Normal(1.8812),
                    "\u{00B0}C",
                    StatusFlags {
                        auto_range: true,
                        ..StatusFlags::default()
                    },
                );
                m.display_raw = Some(" 1.8812".to_string());
                m.mode = "Temp \u{00B0}C T2-T1".into();
                m.aux_values = rows.clone();
                app.last_measurement = Some(m);

                let nodes = meter_nodes(&mut app, w, h);
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
                let find = |text: &str| {
                    label_exact(&nodes, text).unwrap_or_else(|| panic!("{case}: no {text:?}"))
                };
                let value = find(" 1.8812");
                let first = find(rows[0].label.as_ref());
                let mode = find("Temp \u{00B0}C T2-T1");
                let auto = find("AUTO");
                assert!(
                    value.right() < first.left() && first.right() < mode.left(),
                    "{case}: value {value:?}, sub-values {first:?}, mode {mode:?} not side by side"
                );
                assert!(
                    screen.contains_rect(auto),
                    "{case}: AUTO {auto:?} hangs out of {screen:?}"
                );
            }
        }
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
