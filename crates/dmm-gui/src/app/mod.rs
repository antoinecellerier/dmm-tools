//! The application: the [`App`] state every panel reads and writes, and the
//! per-frame `update` that lays the panels out.
//!
//! The concerns live in submodules — [`appearance`] (fonts, theme, zoom),
//! [`connection`] and [`messages`] (the acquisition thread and its channel),
//! [`plot_input`], [`top_bar`], [`controls`], [`layout`] (the reading column),
//! [`stats_panel`], [`recording_panel`], [`export`], [`transform_ui`],
//! [`shortcuts`], [`shortcut_help`] and [`whats_new`] — all of which add
//! methods to the one [`App`] declared here.

mod appearance;
mod connection;
mod controls;
mod export;
mod layout;
mod messages;
mod plot_input;
mod recording_panel;
mod shortcut_help;
mod shortcuts;
mod stats_panel;
mod top_bar;
mod transform_ui;
mod whats_new;

use dmm_lib::measurement::Measurement;
use dmm_lib::protocol::registry;
use dmm_lib::transform::Transform;
use eframe::egui::{self, Color32};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::a11y::ResponseA11yExt;
use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{Settings, ThemeMode};
use appearance::{font_definitions, install_text_styles};
use dmm_lib::stats::SeriesStats;
use export::ExportOutcome;
use layout::ContentLayout;
use messages::ConnectionIssue;
use transform_ui::TransformEditor;

/// How long a toast message stays visible (seconds).
const TOAST_DURATION_SECS: u64 = 4;

/// Default height of the recording panel (logical pixels).
const DEFAULT_RECORDING_HEIGHT: f32 = 120.0;

/// Initial estimate for non-reading content height in big meter mode.
const DEFAULT_METER_CONTENT_HEIGHT: f32 = 200.0;

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

pub struct App {
    pub(super) settings: Settings,
    pub(super) settings_open: bool,

    pub(super) connection_state: ConnectionState,
    pub(super) device_name: Option<String>,
    /// Whether the connected protocol is experimental (unverified).
    pub(super) experimental: bool,
    /// URL for reporting feedback on experimental protocols.
    pub(super) feedback_url: String,
    /// Commands supported by the connected protocol.
    pub(super) supported_commands: Vec<String>,
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

    rx: Option<mpsc::Receiver<DmmMessage>>,
    /// Meter the buffered recording was captured from, taken when recording
    /// started. Outlives disconnect so a capture can still be exported with
    /// the right provenance after the meter is unplugged.
    recording_device: Option<&'static str>,
    /// Sub-value slots the connected meter family can report, from its
    /// profile. 0 until the first `Connected`, and kept on disconnect so a
    /// capture stays exportable with its full column layout.
    device_aux_slots: usize,
    /// Sub-value slots the meter itself can fill in the buffered recording,
    /// taken when recording started. Captured alongside `recording_device`
    /// and for the same reason: the CSV column layout has to describe the
    /// meter the samples came from, not whatever is selected at export time.
    recording_aux_slots: usize,
    /// Extra sub-value slots the export reserves *after* the meter's own, for
    /// the ones software appends (a transform's `Raw`). Kept apart from
    /// `recording_aux_slots` so `Raw` gets a fixed trailing column instead of
    /// sliding forward whenever the meter sends fewer sub-values. Only ever
    /// grows during a recording — turning a scale off mid-capture leaves the
    /// trailing group empty rather than renumbering the columns already
    /// written into the user's mental model of the file.
    recording_extra_slots: usize,
    /// Profile of the selected device, refreshed only when the selection
    /// changes. Two render paths need it every frame, and building a protocol
    /// to read it allocates — the UT61E+ factory lowercases its model string,
    /// boxes a device table and reserves an rx buffer.
    selected_profile: dmm_lib::protocol::DeviceProfile,
    /// Device id the cached profile belongs to.
    selected_profile_id: &'static str,
    /// Record was pressed while the buffer held unexported samples; waiting
    /// for the user to confirm discarding them.
    confirm_discard_open: bool,
    confirm_discard_focus_pending: bool,
    ctrl_tx: Option<mpsc::Sender<ThreadControl>>,
    /// Stop request, readable without consuming a channel message so the
    /// acquisition thread's pacing sleep can bail out on it mid-tick.
    /// `ctrl_tx` stays the control path; this is the wake signal.
    stop_flag: Option<Arc<AtomicBool>>,
    pub(super) cmd_tx: Option<mpsc::Sender<String>>,
    first_frame: bool,
    /// Reconnect on next frame (device selection changed while connected).
    pub(super) needs_reconnect: bool,
    /// OS default pixels_per_point, captured on first frame.
    os_ppp: Option<f32>,
    /// Last applied theme (to avoid re-setting every frame).
    applied_theme: Option<ThemeMode>,
    /// Last applied UI chrome colors (bg, text, weak_text, button, plot_bg) to
    /// avoid per-frame Visuals mutation.
    applied_ui_colors: Option<(Color32, Color32, Color32, Color32, Color32)>,
    /// Last minimum window size pushed to the windowing system, so the
    /// viewport command is only re-sent when it actually changes.
    applied_min_size: Option<egui::Vec2>,
    /// User-resizable recording panel height.
    recording_height: f32,
    /// Transient status toast (message, is_error, timestamp).
    toast: Option<(String, bool, Instant)>,
    /// One-shot receiver for CSV export result.
    export_result_rx: Option<mpsc::Receiver<ExportOutcome>>,
    /// Cached height of non-reading content at scale=1 for big meter mode.
    meter_content_height: f32,
    /// Cached reading dimension ratios for big meter mode.
    meter_reading_ratios: display::ReadingRatios,
    /// Cache key for big meter scale. Recalculate when any input changes.
    meter_cache_key: u64,
    /// Number of recalculation passes since last cache key change.
    meter_recalc_passes: u8,
    /// Transient big meter mode (not persisted to settings).
    big_meter_mode: BigMeterMode,
    /// Whether the keyboard shortcut help overlay is open.
    shortcut_help_open: bool,
    /// Widget id that opened the shortcut help window — focus is restored to
    /// this widget when the window closes so keyboard users don't lose place.
    shortcut_help_opener: Option<egui::Id>,
    /// Pending focus target to restore after the shortcut help modal closes.
    /// The restore is deferred until `top_modal_layer` has actually cleared —
    /// otherwise egui's `create_widget` calls `surrender_focus` on widgets
    /// below the modal layer (still committed from the close frame) and
    /// wipes any focus we set in the close path.
    shortcut_help_restore_focus: Option<egui::Id>,
    /// Set on the frame the shortcut help window is opened so the next frame
    /// can focus the first widget inside it (one-shot trigger).
    shortcut_help_focus_pending: bool,
    /// Whether the "What's New" changelog window is open.
    whats_new_open: bool,
    /// Widget id that opened the What's New viewport — focus is restored to
    /// this widget when the viewport closes.
    whats_new_opener: Option<egui::Id>,
    /// Set by the viewport callback when the user closes the changelog window.
    whats_new_closed: Arc<AtomicBool>,
    /// Shared commonmark cache for the changelog viewport.
    whats_new_cache: Arc<Mutex<egui_commonmark::CommonMarkCache>>,
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
            settings.mock_mode = mock_mode;
        }
        if let Some(theme) = cli.theme {
            settings.overrides.theme = Some(settings.theme);
            settings.theme = theme;
        }
        settings.overrides.adapter = cli.adapter;
        let graph = Graph::new();
        let initial_device = registry::resolve_device(&settings.shared.device_family)
            .unwrap_or_else(registry::default_device);
        Self {
            settings,
            settings_open: false,
            connection_state: ConnectionState::Disconnected,
            device_name: None,
            experimental: false,
            feedback_url: String::new(),
            supported_commands: Vec::new(),
            paused: false,
            last_error: None,
            waiting_timeouts: 0,
            reconnect_attempt: 0,
            reconnect_last_error: None,
            last_measurement: None,
            transform: Transform::default(),
            transform_editor: TransformEditor::default(),
            graph,
            session: SeriesStats::new(true),
            recording: Recording::new(),
            wall_clock: dmm_lib::WallClock::new(),
            rx: None,
            recording_device: None,
            device_aux_slots: 0,
            recording_aux_slots: 0,
            recording_extra_slots: 0,
            selected_profile: *(initial_device.new_protocol)().profile(),
            selected_profile_id: initial_device.id,
            confirm_discard_open: false,
            confirm_discard_focus_pending: false,
            ctrl_tx: None,
            stop_flag: None,
            cmd_tx: None,
            first_frame: true,
            needs_reconnect: false,
            os_ppp: None,
            applied_theme: None,
            applied_ui_colors: None,
            applied_min_size: None,
            recording_height: DEFAULT_RECORDING_HEIGHT,
            toast: None,
            export_result_rx: None,
            meter_content_height: DEFAULT_METER_CONTENT_HEIGHT,
            meter_reading_ratios: display::ReadingRatios::default(),
            meter_cache_key: 0,
            meter_recalc_passes: 0,
            big_meter_mode: BigMeterMode::Off,
            shortcut_help_open: false,
            shortcut_help_opener: None,
            shortcut_help_restore_focus: None,
            shortcut_help_focus_pending: false,
            whats_new_open: false,
            whats_new_opener: None,
            whats_new_closed: Arc::new(AtomicBool::new(false)),
            whats_new_cache: Arc::new(Mutex::new(egui_commonmark::CommonMarkCache::default())),
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
        self.paused = paused;
        if paused {
            // Acquisition stops, so the samples that would have covered this
            // stretch never exist — a data gap the graph should show even if
            // the pause is shorter than its elapsed-time threshold.
            self.graph.push_data_loss();
        }
        if let Some(tx) = &self.ctrl_tx {
            let _ = tx.send(ThreadControl::SetPaused(paused));
        }
    }

    pub(super) fn send_command(&self, cmd: &str) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd.to_string());
        }
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
        if self.reconnect_attempt > 0 {
            format!("Reconnecting (attempt {})...", self.reconnect_attempt)
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
        if let Some(target) = self.shortcut_help_restore_focus
            && ctx.memory(|m| m.top_modal_layer()).is_none()
        {
            ctx.memory_mut(|m| m.request_focus(target));
            self.shortcut_help_restore_focus = None;
        }
        self.refresh_selected_profile();
        self.apply_theme(&ctx);
        self.apply_color_overrides(&ctx);
        self.apply_zoom(&ctx);
        self.handle_keyboard_shortcuts(&ctx);
        self.drain_messages();
        self.poll_export_result();

        // Auto-reconnect after device selection change
        if self.needs_reconnect {
            self.needs_reconnect = false;
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
            egui::Panel::top("top_bar").show_inside(ui, |ui| {
                self.show_top_bar(ui, &ctx);
                self.show_settings_panel(ui);
            });
        }

        // Determine layout mode before panels
        let wide = ctx.content_rect().width() >= 900.0;

        let meter_only = self.big_meter_mode != BigMeterMode::Off
            || (!self.settings.show_graph && !self.settings.show_recording);

        // Dynamic minimum window size derived from actual rendered content.
        // Reading dimensions come from cached ratios × minimum big meter
        // font size; top bar widths come from previous-frame measurements.
        let min_font = display::MIN_BIG_METER_FONT_SIZE;
        let ratios = &self.meter_reading_ratios;
        let min_scale = min_font / display::BASE_READING_FONT_SIZE;
        let reading_w = ratios.w * min_font;
        let reading_h = ratios.h * min_font + self.meter_content_height * min_scale;
        let bar_left_w: f32 =
            ctx.data(|d| d.get_temp(egui::Id::new("top_bar_left_w")).unwrap_or(300.0));
        let bar_right_w: f32 = ctx.data(|d| {
            d.get_temp(egui::Id::new("top_bar_right_w"))
                .unwrap_or(120.0)
        });
        let bar_min_w = bar_left_w.max(bar_right_w) + 16.0;

        let min_size = if minimal {
            // Just the reading — no top bar, no buttons.
            egui::vec2(reading_w, reading_h)
        } else if meter_only {
            // Reading + buttons + top bar.
            egui::vec2(reading_w.max(bar_min_w), reading_h)
        } else {
            // Full layout: top bar constrains width, panels need height.
            egui::vec2(bar_min_w, reading_h)
        };
        // Only when it changes. Its inputs — cached top-bar widths, meter
        // ratios, big-meter mode — are stable across the vast majority of
        // frames, and a viewport command sent every repaint is the same class
        // of mistake .claude/rules/gui.md calls out for set_visuals() and
        // set_pixels_per_point(). Half a pixel of tolerance keeps sub-pixel
        // jitter in the cached widths from re-triggering it.
        if self
            .applied_min_size
            .is_none_or(|prev| (prev - min_size).abs().max_elem() > 0.5)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
            self.applied_min_size = Some(min_size);
        }
        // If the window is smaller than the new minimum (e.g. after exiting
        // minimal mode), grow it to fit.
        let screen = ctx.content_rect();
        if screen.width() < min_size.x || screen.height() < min_size.y {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                screen.width().max(min_size.x),
                screen.height().max(min_size.y),
            )));
        }

        if meter_only {
            // Big meter mode: compute scale from window size, only recalculate
            // when the window is resized to avoid frame-to-frame oscillation.
            // Shrink panel margins at small window sizes so the reading fills
            // the space tighter.
            let screen = ctx.content_rect();
            let margin_scale = (screen.width().min(screen.height()) / 300.0).clamp(0.1, 1.0);
            let default_margin = ctx.global_style().spacing.window_margin;
            let frame = egui::Frame::central_panel(ctx.global_style().as_ref())
                .inner_margin(default_margin * margin_scale);
            let main = egui::CentralPanel::default()
                .frame(frame)
                .show_inside(ui, |ui| {
                    let size = ctx.content_rect();
                    use std::hash::{Hash, Hasher};
                    let cache_key = {
                        let mut h = std::hash::DefaultHasher::new();
                        (size.width() as u32).hash(&mut h);
                        (size.height() as u32).hash(&mut h);
                        self.last_measurement
                            .as_ref()
                            .map_or(0u16, |m| m.mode_raw)
                            .hash(&mut h);
                        // Sub-value rows change the reading's height without
                        // changing the mode word (a UT181A entering MIN/MAX),
                        // so the fitted font has to be re-measured.
                        self.last_measurement
                            .as_ref()
                            .map_or(0usize, |m| m.aux_values.len())
                            .hash(&mut h);
                        self.settings.show_stats.hash(&mut h);
                        self.settings.show_specs.hash(&mut h);
                        self.big_meter_mode.hash(&mut h);
                        // The Scale row adds a button row, and opening its
                        // editor adds a second one — both change how much
                        // room is left for the reading.
                        self.transform_editor.open.hash(&mut h);
                        self.transform.is_identity().hash(&mut h);
                        h.finish()
                    };
                    let needs_recalc = cache_key != self.meter_cache_key;

                    let panel_rect = ui.max_rect();
                    let mut add_content = |ui: &mut egui::Ui| {
                        ui.vertical(|ui| {
                            // In minimal mode there's nothing below the reading,
                            // so pass 0 to let the reading fill all available space.
                            let content_h = if minimal {
                                0.0
                            } else {
                                self.meter_content_height
                            };
                            let tc = self.settings.theme_colors(ui.visuals().dark_mode);
                            let (scale, measured_ratios) = display::show_reading_large(
                                ui,
                                self.last_measurement.as_ref(),
                                content_h,
                                &self.meter_reading_ratios,
                                &tc,
                                !self.transform.is_identity(),
                            );
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
                            // (by not setting meter_last_size the first time) so
                            // the second pass uses the measured values from the first.
                            if needs_recalc && scale > 0.0 {
                                let total_below_reading = ui.cursor().top() - after_reading;
                                let measured = total_below_reading / scale;
                                if (self.meter_content_height - measured).abs() < 1.0
                                    || self.meter_recalc_passes >= 4
                                {
                                    // Converged, or max passes reached (e.g. button
                                    // row wrapping oscillation). Use the larger height
                                    // so everything fits.
                                    self.meter_content_height =
                                        self.meter_content_height.max(measured);
                                    self.meter_cache_key = cache_key;
                                    self.meter_recalc_passes = 0;
                                } else {
                                    self.meter_content_height = measured;
                                    self.meter_recalc_passes += 1;
                                }
                                self.meter_reading_ratios = measured_ratios;
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
                .show_inside(ui, |ui| {
                    self.show_reading_column(ui, ContentLayout::Wide);
                });
            // egui's `Panel::left(..).resizable(true)` allocates a
            // drag-sense resize handle at its right edge, which is focusable
            // but has no visible focus indicator of its own and no keyboard
            // action. Paint a focus ring and wire up Left/Right arrow keys
            // to resize the panel, consistent with the recording-panel
            // divider. The handle id is derived from the panel id — see
            // `panel.rs:847` in egui 0.34 for the `__resize` salt.
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
                    let new_width = (state.rect.width() + delta)
                        .clamp(SIDE_PANEL_MIN_WIDTH, SIDE_PANEL_MAX_WIDTH);
                    state.rect.max.x = state.rect.min.x + new_width;
                    ctx.data_mut(|d| d.insert_persisted(reading_panel_id, state));
                }
                // Paint a 3px focus indicator on the panel's right edge —
                // the standard focus ring is invisible on the thin vline
                // egui uses to draw the panel boundary.
                let panel_rect = reading_panel.response.rect;
                let stroke_color = ui.visuals().selection.stroke.color;
                ui.painter().vline(
                    panel_rect.right(),
                    panel_rect.y_range(),
                    egui::Stroke::new(3.0_f32, stroke_color),
                );
            }

            // Wide: center panel for graph + recording
            egui::CentralPanel::default()
                .show_inside(ui, |ui| {
                    self.show_graph_recording_split(ui, false);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        } else {
            // Narrow: single column
            egui::CentralPanel::default()
                .show_inside(ui, |ui| {
                    self.show_reading_column(ui, ContentLayout::Narrow);
                })
                .response
                .a11y_role(egui::accesskit::Role::Main);
        }

        self.show_shortcut_help(&ctx);
        self.show_discard_confirmation(&ctx);
        self.show_whats_new(&ctx);

        if self.connection_state == ConnectionState::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
