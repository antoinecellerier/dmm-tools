//! The application: the [`App`] state every panel reads and writes, and the
//! per-frame `update` that lays the panels out.
//!
//! The concerns live in submodules — [`appearance`] (fonts, theme, zoom),
//! [`connection`] and [`messages`] (the acquisition thread and its channel),
//! [`plot_input`], [`top_bar`], [`controls`], [`layout`] (the reading column),
//! [`meter_fit`] (the big meter's sizing arithmetic), [`stats_panel`],
//! [`recording_panel`], [`export`], [`transform_ui`], [`shortcuts`],
//! [`shortcut_help`] and [`whats_new`] — all of which add methods to the one
//! [`App`] declared here.

mod appearance;
mod connection;
mod controls;
mod export;
mod layout;
mod messages;
mod meter_fit;
mod plot_input;
mod recording_panel;
mod shortcut_help;
mod shortcuts;
mod stats_panel;
mod top_bar;
mod transform_ui;
mod whats_new;

use dmm_lib::measurement::Measurement;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::{Choice, Setting, registry};
use dmm_lib::transform::Transform;
use eframe::egui;
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
const TOAST_DURATION_SECS: u64 = 4;

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

/// Provenance and column layout the buffered capture is exported with. Taken
/// when recording starts and kept across disconnect, so a capture describes
/// the meter its samples came from rather than whatever is selected at export
/// time.
#[derive(Default)]
struct CaptureLayout {
    /// Meter the buffered recording was captured from, taken when recording
    /// started. Outlives disconnect so a capture can still be exported with
    /// the right provenance after the meter is unplugged.
    device: Option<&'static str>,
    /// Sub-value slots the connected meter family can report, from its
    /// profile. 0 until the first `Connected`, and kept on disconnect so a
    /// capture stays exportable with its full column layout.
    device_aux_slots: usize,
    /// Sub-value slots the meter itself can fill in the buffered recording,
    /// taken when recording started. Captured alongside `device` and for the
    /// same reason: the CSV column layout has to describe the meter the
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

    pub(super) fn readouts(&self) -> display::ReadoutChoices<'_> {
        display::ReadoutChoices {
            mode: &self.mode,
            range: &self.range,
        }
    }

    /// Whether the range readout is drawn as a dropdown, which is taller than
    /// the label it replaces and so changes the big meter's fitted font.
    pub(super) fn range_offered(&self) -> bool {
        display::mode_switch_offered(&self.range)
    }

    /// The same, for the mode readout.
    pub(super) fn mode_offered(&self) -> bool {
        display::mode_switch_offered(&self.mode)
    }
}

/// The live link to a meter: its state, what the connected protocol told us
/// about itself, and the channels and flags shared with the acquisition
/// thread.
pub(super) struct Connection {
    pub(super) state: ConnectionState,
    pub(super) device_name: Option<String>,
    /// Whether the connected protocol is experimental (unverified).
    pub(super) experimental: bool,
    /// URL for reporting feedback on experimental protocols.
    pub(super) feedback_url: String,
    /// Commands supported by the connected protocol.
    pub(super) supported_commands: Vec<String>,
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
            experimental: false,
            feedback_url: String::new(),
            supported_commands: Vec::new(),
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

pub struct App {
    pub(super) settings: Settings,
    pub(super) settings_open: bool,

    pub(super) connection: Connection,
    pub(super) last_measurement: Option<Measurement>,
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

    capture_layout: CaptureLayout,
    /// Profile of the selected device, refreshed only when the selection
    /// changes. Two render paths need it every frame, and building a protocol
    /// to read it allocates — the UT61E+ factory lowercases its model string,
    /// boxes a device table and reserves an rx buffer.
    selected_profile: dmm_lib::protocol::DeviceProfile,
    /// Device id the cached profile belongs to.
    selected_profile_id: &'static str,
    recording_panel: RecordingPanel,
    first_frame: bool,
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
        let mut settings = Settings::load();
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
        settings.overrides.adapter = cli.adapter;
        Self::from_settings(settings, cli.clock)
    }

    /// The app state for `settings` on `clock`, before any frame. Separate
    /// from [`App::new`] so tests can build one without an eframe context.
    fn from_settings(settings: Settings, clock: dmm_lib::Clock) -> Self {
        let graph = Graph::new();
        let initial_device = registry::resolve_device(&settings.shared.device_family)
            .unwrap_or_else(registry::default_device);
        Self {
            settings,
            settings_open: false,
            connection: Connection::default(),
            last_measurement: None,
            transform: Transform::default(),
            transform_editor: TransformEditor::default(),
            graph,
            session: SeriesStats::new(true),
            recording: Recording::new(),
            wall_clock: dmm_lib::WallClock::from_clock(&clock),
            clock,
            capture_layout: CaptureLayout::default(),
            selected_profile: *(initial_device.new_protocol)().profile(),
            selected_profile_id: initial_device.id,
            recording_panel: RecordingPanel::default(),
            first_frame: true,
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
        if self.selected_profile_id != device.id {
            self.selected_profile = *(device.new_protocol)().profile();
            self.selected_profile_id = device.id;
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
        if self.selected_device().id != "mock" || self.settings.mock_mode.is_empty() {
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

    fn selected_device(&self) -> &'static registry::SelectableDevice {
        registry::resolve_device(&self.settings.shared.device_family)
            .unwrap_or_else(registry::default_device)
    }

    fn manual_url(&self) -> Option<&'static str> {
        self.selected_device().manual_url
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
        self.drain_messages();
        self.poll_export_result();

        // Auto-reconnect after device selection change
        if self.connection.needs_reconnect {
            self.connection.needs_reconnect = false;
            self.connect(&ctx);
        }

        // Expire toast after 4 seconds
        if let Some((_, _, when)) = &self.toast
            && when.elapsed().as_secs() >= TOAST_DURATION_SECS
        {
            self.toast = None;
        }

        // Auto-connect on first frame if enabled
        if self.first_frame {
            self.first_frame = false;
            if self.settings.always_on_top {
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
        if !minimal {
            egui::Panel::top("top_bar").show(ui, |ui| {
                self.show_top_bar(ui, &ctx);
                self.show_settings_panel(ui);
            });
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
            // Big meter mode: compute scale from window size, only recalculate
            // when the window is resized to avoid frame-to-frame oscillation.
            // Shrink panel margins at small window sizes so the reading fills
            // the space tighter.
            let margin_scale = meter_fit::margin_scale(ctx.content_rect().size());
            let default_margin = ctx.global_style().spacing.window_margin;
            let frame = egui::Frame::central_panel(ctx.global_style().as_ref())
                .inner_margin(default_margin * margin_scale);
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
                    mode_offered: self.connection.choices.mode_offered(),
                    range_offered: self.connection.choices.range_offered(),
                    show_stats: self.settings.show_stats,
                    show_specs: self.settings.show_specs,
                    big_meter_mode: self.big_meter_mode,
                    transform_editor_open: self.transform_editor.open,
                    transform_is_identity: self.transform.is_identity(),
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
                        let (scale, measured_ratios, picked) = display::show_reading_large(
                            ui,
                            self.last_measurement.as_ref(),
                            content_h,
                            &self.meter_fit.reading_ratios,
                            &tc,
                            !self.transform.is_identity(),
                            self.connection.choices.readouts(),
                        );
                        if let Some((setting, id)) = picked {
                            self.select(setting, id);
                        }
                        let after_reading = ui.cursor().top();

                        if !minimal {
                            self.show_remote_controls(ui, scale);
                            self.show_transform_row(ui, scale);
                        }
                        self.show_connection_help(ui);

                        if self.big_meter_mode == BigMeterMode::Off {
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
        } else if wide {
            // Wide: left side panel for reading + stats (resizable)
            let reading_panel = egui::Panel::left("reading_panel")
                .default_size(SIDE_PANEL_DEFAULT_WIDTH)
                .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH)
                .resizable(true)
                .show(ui, |ui| {
                    self.show_reading_column(ui, ContentLayout::Wide);
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
                    self.show_graph_recording_split(ui, false);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        } else {
            // Narrow: single column
            egui::CentralPanel::default()
                .show(ui, |ui| {
                    self.show_reading_column(ui, ContentLayout::Narrow);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        }

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

    /// A pick on a real meter has nothing to do with the mock's pin.
    #[test]
    fn a_pick_on_a_real_meter_leaves_the_mock_pin_alone() {
        let mut app = app("ut181a", "temp2");
        assert!(!app.repin_mock(0x1121));
        assert_eq!(app.settings.mock_mode, "temp2");
    }
}
