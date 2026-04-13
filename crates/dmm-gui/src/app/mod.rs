mod connection;
mod controls;

use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use dmm_lib::protocol::ut61eplus::tables::{ModeSpecInfo, SpecInfo};
use eframe::egui::{self, Color32, RichText, Ui};
use log::{error, info};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{Settings, ThemeMode};
use crate::specs;
use crate::theme::ThemeColors;
use dmm_lib::stats::{self, Integrator, RunningStats};

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

use connection::{DmmMessage, handle_thread_panic, run_device_thread};

/// Pre-formatted min/max/avg/count strings for a single stats group.
struct FormattedStatsGroup {
    min: String,
    max: String,
    avg: String,
    count: usize,
    integral: Option<String>,
}

/// Pre-formatted statistics for the stats section, shared by both layout modes.
struct FormattedStats {
    min: String,
    max: String,
    avg: String,
    count: u64,
    /// Formatted integral string (e.g. "   0.2925 mAh"), or None if not integrable.
    integral: Option<String>,
    /// Visible-window stats, if available.
    visible: Option<FormattedStatsGroup>,
}

impl FormattedStats {
    fn new(
        stats: &RunningStats,
        visible: Option<(f64, f64, f64, usize)>,
        unit: &str,
        integral: Option<(f64, &str, f64, Option<f64>)>,
        visible_integral: Option<(f64, &str, f64, Option<f64>)>,
    ) -> Self {
        let fmt = |v: Option<f64>| -> String {
            match v {
                Some(val) => format!("{val:>10.4} {unit}"),
                None => format!("{:>10} {unit}", crate::NO_DATA),
            }
        };
        let fmt_integral = |info: Option<(f64, &str, f64, Option<f64>)>| -> Option<String> {
            info.map(|(raw, disp_unit, divisor, dt)| {
                let val = raw / divisor;
                match dt {
                    Some(secs) => format!("{val:>10.4} {disp_unit} ({secs:.0}s)"),
                    None => format!("{val:>10.4} {disp_unit}"),
                }
            })
        };
        Self {
            min: fmt(stats.min),
            max: fmt(stats.max),
            avg: fmt(stats.avg()),
            count: stats.count,
            integral: fmt_integral(integral),
            visible: visible.map(|(vmin, vmax, vavg, vcount)| FormattedStatsGroup {
                min: fmt(Some(vmin)),
                max: fmt(Some(vmax)),
                avg: fmt(Some(vavg)),
                count: vcount,
                integral: fmt_integral(visible_integral),
            }),
        }
    }
}

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
    pub(super) last_error: Option<String>,
    /// Consecutive timeout count (0 = not waiting).
    pub(super) waiting_timeouts: u32,
    pub(super) last_measurement: Option<Measurement>,

    /// Cached per-range spec for current measurement (avoids lookup per frame).
    pub(super) cached_spec: Option<&'static SpecInfo>,
    /// Cached per-mode spec for current measurement.
    pub(super) cached_mode_spec: Option<&'static ModeSpecInfo>,
    /// Key used to invalidate spec cache: (mode_raw, range_raw).
    cached_spec_key: (u16, u8),

    graph: Graph,
    stats: RunningStats,
    integrator: Integrator,
    recording: Recording,

    rx: Option<mpsc::Receiver<DmmMessage>>,
    stop_tx: Option<mpsc::Sender<()>>,
    pub(super) cmd_tx: Option<mpsc::Sender<String>>,
    first_frame: bool,
    /// Reconnect on next frame (device selection changed while connected).
    pub(super) needs_reconnect: bool,
    /// OS default pixels_per_point, captured on first frame.
    os_ppp: Option<f32>,
    /// Last applied theme (to avoid re-setting every frame).
    applied_theme: Option<ThemeMode>,
    /// Last applied UI chrome colors (bg, text, button, plot_bg) to avoid per-frame Visuals mutation.
    applied_ui_colors: Option<(Color32, Color32, Color32, Color32)>,
    /// User-resizable recording panel height.
    recording_height: f32,
    /// Transient status toast (message, is_error, timestamp).
    toast: Option<(String, bool, Instant)>,
    /// One-shot receiver for CSV export result.
    export_result_rx: Option<mpsc::Receiver<(String, bool)>>,
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
    /// Whether the "What's New" changelog window is open.
    whats_new_open: bool,
    /// Set by the viewport callback when the user closes the changelog window.
    whats_new_closed: Arc<AtomicBool>,
    /// Shared commonmark cache for the changelog viewport.
    whats_new_cache: Arc<Mutex<egui_commonmark::CommonMarkCache>>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, cli: crate::CliOverrides) -> Self {
        let mut settings = Settings::load();
        if let Some(device) = cli.device {
            settings.overrides.device_family = Some(settings.device_family.clone());
            settings.device_family = device;
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
        let mut graph = Graph::new();
        graph.set_color_config(settings.color_preset, settings.color_overrides.clone());
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
            last_measurement: None,
            cached_spec: None,
            cached_mode_spec: None,
            cached_spec_key: (u16::MAX, u8::MAX),
            graph,
            stats: RunningStats::new(),
            integrator: Integrator::new(),
            recording: Recording::new(),
            rx: None,
            stop_tx: None,
            cmd_tx: None,
            first_frame: true,
            needs_reconnect: false,
            os_ppp: None,
            applied_theme: None,
            applied_ui_colors: None,
            recording_height: DEFAULT_RECORDING_HEIGHT,
            toast: None,
            export_result_rx: None,
            meter_content_height: DEFAULT_METER_CONTENT_HEIGHT,
            meter_reading_ratios: display::ReadingRatios::default(),
            meter_cache_key: 0,
            meter_recalc_passes: 0,
            big_meter_mode: BigMeterMode::Off,
            shortcut_help_open: false,
            whats_new_open: false,
            whats_new_closed: Arc::new(AtomicBool::new(false)),
            whats_new_cache: Arc::new(Mutex::new(egui_commonmark::CommonMarkCache::default())),
        }
    }

    /// Build a ThemeColors instance from current settings and dark mode state.
    fn theme_colors(&self, dark: bool) -> ThemeColors {
        ThemeColors::new(
            dark,
            self.settings.color_preset,
            self.settings.color_overrides.for_mode(dark),
        )
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let target = match self.settings.theme {
            ThemeMode::Dark | ThemeMode::System => ThemeMode::Dark,
            ThemeMode::Light => ThemeMode::Light,
        };
        if self.applied_theme != Some(target) {
            match target {
                ThemeMode::Dark | ThemeMode::System => ctx.set_visuals(egui::Visuals::dark()),
                ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            }
            self.applied_theme = Some(target);
            self.applied_ui_colors = None; // force reapply on top of new base
        }
    }

    /// Apply background, text, and button color overrides to egui Visuals.
    fn apply_color_overrides(&mut self, ctx: &egui::Context) {
        let dark = matches!(self.settings.theme, ThemeMode::Dark | ThemeMode::System);
        let tc = self.theme_colors(dark);
        let bg = tc.background();
        let text = tc.text();
        let button = tc.button();
        let plot_bg = tc.plot_background();
        let key = (bg, text, button, plot_bg);

        if self.applied_ui_colors == Some(key) {
            return;
        }
        self.applied_ui_colors = Some(key);

        let (hover, active) = tc.button_hover_active();
        ctx.global_style_mut(|style| {
            let v = &mut style.visuals;
            v.panel_fill = bg;
            v.window_fill = bg;
            // Plot background and minimap background use extreme_bg_color.
            v.extreme_bg_color = plot_bg;
            v.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(v.widgets.noninteractive.fg_stroke.width, text);
            v.widgets.inactive.bg_fill = button;
            v.widgets.inactive.weak_bg_fill = button;
            v.widgets.hovered.bg_fill = hover;
            v.widgets.hovered.weak_bg_fill = hover;
            v.widgets.active.bg_fill = active;
            v.widgets.active.weak_bg_fill = active;
        });
    }

    pub(super) const ZOOM_LEVELS: &[u32] = &[
        30, 50, 67, 80, 90, 100, 110, 120, 133, 150, 170, 200, 240, 300,
    ];

    fn apply_zoom(&mut self, ctx: &egui::Context) {
        // Capture OS default pixels_per_point on first call
        if self.os_ppp.is_none() {
            self.os_ppp = Some(ctx.pixels_per_point());
        }
        let Some(os_ppp) = self.os_ppp else { return };
        let target_ppp = os_ppp * self.settings.zoom_pct as f32 / 100.0;
        // Only update when changed — setting ppp every frame resets panel resize state
        if (ctx.pixels_per_point() - target_ppp).abs() > 0.001 {
            ctx.set_pixels_per_point(target_ppp);
        }
    }

    fn zoom_in(&mut self) {
        if let Some(&next) = Self::ZOOM_LEVELS
            .iter()
            .find(|&&z| z > self.settings.zoom_pct)
        {
            self.settings.zoom_pct = next;
            self.settings.save();
        }
    }

    fn zoom_out(&mut self) {
        if let Some(&prev) = Self::ZOOM_LEVELS
            .iter()
            .rev()
            .find(|&&z| z < self.settings.zoom_pct)
        {
            self.settings.zoom_pct = prev;
            self.settings.save();
        }
    }

    fn zoom_reset(&mut self) {
        self.settings.zoom_pct = 100;
        self.settings.save();
    }

    pub(super) fn apply_always_on_top(&self, ctx: &egui::Context) {
        let level = if self.settings.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    pub(super) fn apply_decorations(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
            !self.settings.hide_decorations,
        ));
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        // --- Ctrl+Shift shortcuts (most specific first) ---

        // Ctrl+Shift+C: Connect/Disconnect
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::C)) {
            match self.connection_state {
                ConnectionState::Disconnected => self.connect(ctx),
                ConnectionState::Connected => self.disconnect(),
                ConnectionState::Reconnecting => {}
            }
        }

        // --- Ctrl shortcuts ---

        // Ctrl+Q: Quit
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Q)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Ctrl+L: Clear graph & statistics
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::L))
            && self.connection_state == ConnectionState::Connected
        {
            self.graph.clear();
            self.stats.reset();
            self.integrator.reset();
            self.last_measurement = None;
        }

        // Ctrl+R: Toggle recording
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::R))
            && self.connection_state == ConnectionState::Connected
        {
            self.recording.toggle();
        }

        // Ctrl+B: Cycle big meter mode (off -> full -> minimal -> off)
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::B)) {
            self.cycle_big_meter();
        }

        // Ctrl+T: Toggle always on top
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::T)) {
            self.settings.always_on_top = !self.settings.always_on_top;
            self.apply_always_on_top(ctx);
            self.settings.save();
        }

        // Ctrl+D: Toggle window decorations
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::D)) {
            self.settings.hide_decorations = !self.settings.hide_decorations;
            self.apply_decorations(ctx);
            self.settings.save();
        }

        // Ctrl+E: Export CSV
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::E)) {
            self.export_csv();
        }

        // Ctrl+= or Ctrl++: Zoom in
        if ctx.input_mut(|i| {
            i.consume_key(Modifiers::COMMAND, Key::Plus)
                || i.consume_key(Modifiers::COMMAND, Key::Equals)
        }) {
            self.zoom_in();
        }

        // Ctrl+-: Zoom out
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Minus)) {
            self.zoom_out();
        }

        // Ctrl+0: Reset zoom
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Num0)) {
            self.zoom_reset();
        }

        // Escape / Ctrl+W: Close shortcut help overlay (if open).
        // Note: the What's New window is a separate OS viewport and handles
        // its own close via the window's X button.
        if self.shortcut_help_open
            && ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Escape)
                    || i.consume_key(Modifiers::COMMAND, Key::W)
            })
        {
            self.shortcut_help_open = false;
        }

        // --- Bare-key shortcuts (only when no text field has focus) ---
        if !ctx.egui_wants_keyboard_input() {
            // Space: Pause/Resume
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space))
                && self.connection_state == ConnectionState::Connected
            {
                self.paused = !self.paused;
            }

            // ?: Toggle shortcut help
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Questionmark)) {
                self.shortcut_help_open = !self.shortcut_help_open;
            }
        }
    }

    fn connect(&mut self, ctx: &egui::Context) {
        self.disconnect();

        let (msg_tx, msg_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>();
        self.rx = Some(msg_rx);
        self.stop_tx = Some(stop_tx);
        self.cmd_tx = Some(cmd_tx);
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
                        msg_tx,
                        stop_rx,
                        cmd_rx,
                        ctx_clone,
                        query_name,
                        mock_interval,
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
                        msg_tx,
                        stop_rx,
                        cmd_rx,
                        ctx_clone,
                        query_name,
                        sample_interval_ms,
                    );
                }));
                if let Err(panic) = result {
                    handle_thread_panic(panic, &panic_tx, &panic_ctx);
                }
            });
        }
    }

    fn disconnect(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.rx = None;
        self.cmd_tx = None;
        self.connection_state = ConnectionState::Disconnected;
        self.device_name = None;
        self.experimental = false;
        self.feedback_url.clear();
        self.supported_commands.clear();
        self.paused = false;
    }

    fn drain_messages(&mut self) {
        let messages: Vec<DmmMessage> = self
            .rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();

        let mut clear_channel = false;

        for msg in messages {
            match msg {
                DmmMessage::Connected {
                    name,
                    experimental: exp,
                    feedback_url,
                    supported_commands: cmds,
                } => {
                    self.connection_state = ConnectionState::Connected;
                    self.experimental = exp;
                    self.feedback_url = feedback_url;
                    self.supported_commands = cmds;
                    self.device_name = if name.is_empty() {
                        None
                    } else {
                        Some(name.clone())
                    };
                    self.last_error = None;
                    info!("UI: connected to {name}");
                }
                DmmMessage::WaitingForMeter(count) => {
                    self.waiting_timeouts = count;
                }
                DmmMessage::Measurement(m) => {
                    self.last_error = None;
                    self.waiting_timeouts = 0;
                    if self.paused {
                        continue;
                    }

                    // Reset integrator on mode change (units become incompatible).
                    if let Some(prev) = &self.last_measurement
                        && prev.mode != m.mode
                    {
                        self.integrator.reset();
                    }

                    match &m.value {
                        MeasuredValue::Normal(v) => {
                            self.graph.push(*v, &m.mode, &m.unit);
                            self.stats.push(*v);
                            self.integrator.push(*v, m.timestamp);
                        }
                        MeasuredValue::Overload => {
                            self.integrator.push_overload();
                        }
                        _ => {}
                    }

                    if self.recording.push(&m) {
                        self.toast = Some((
                            "Recording stopped \u{2014} buffer full (500K samples)".to_string(),
                            true,
                            Instant::now(),
                        ));
                    }

                    // Update spec cache if mode/range changed
                    let new_key = (m.mode_raw, m.range_raw);
                    if new_key != self.cached_spec_key {
                        self.cached_spec_key = new_key;
                        use dmm_lib::protocol::ut61eplus::tables;
                        let device_id = &self.settings.device_family;
                        self.cached_spec = tables::lookup_spec(device_id, m.mode_raw, m.range_raw);
                        self.cached_mode_spec = tables::lookup_mode_spec(device_id, m.mode_raw);
                    }

                    self.last_measurement = Some(m);
                }
                DmmMessage::Disconnected(reason) => {
                    info!("UI: disconnected: {reason}");
                    self.connection_state = ConnectionState::Reconnecting;
                }
                DmmMessage::DeviceNotFound => {
                    error!("UI: USB cable not found");
                    self.last_error = Some("__device_not_found__".to_string());
                    if self.connection_state == ConnectionState::Disconnected {
                        clear_channel = true;
                    }
                }
                DmmMessage::Error(e) => {
                    error!("UI: error: {e}");
                    self.last_error = Some(e);
                    if self.connection_state == ConnectionState::Disconnected {
                        clear_channel = true;
                    }
                }
            }
        }

        if clear_channel {
            // Disconnect properly: send stop signal so the background thread exits
            self.disconnect();
        }
    }

    pub(super) fn send_command(&self, cmd: &str) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd.to_string());
        }
    }

    fn show_connection_help(&self, ui: &mut Ui) {
        let warn_color = self.theme_colors(ui.visuals().dark_mode).status_warning();

        // Show waiting indicator before error threshold
        if self.waiting_timeouts > 0 && self.last_error.is_none() {
            ui.add_space(4.0);
            let dots = ".".repeat((self.waiting_timeouts as usize % 4) + 1);
            ui.label(RichText::new(format!("Waiting for meter{dots}")).color(warn_color));
            ui.label(
                RichText::new("Check that the correct device is selected in Settings (\u{2699})")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        let error = match &self.last_error {
            Some(e) => e.clone(),
            None => return,
        };

        ui.add_space(8.0);

        let is_device_not_found = error == "__device_not_found__";

        if is_device_not_found {
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
            let device_entry = self.selected_device();
            let proto = (device_entry.new_protocol)();
            let profile = proto.profile();
            if profile.stability == dmm_lib::protocol::Stability::Experimental {
                ui.hyperlink_to(
                    RichText::new(format!(
                        "{} support is experimental \u{2014} report feedback",
                        profile.model_name
                    ))
                    .small()
                    .color(warn_color),
                    profile.feedback_url(),
                );
            }
        } else if error.contains("adapter not found") {
            // --adapter specified but no matching device
            ui.label(RichText::new("Adapter not found").color(warn_color));
            let detail = error.strip_prefix("adapter not found: ").unwrap_or(&error);
            let mut msg = format!("No device matched --adapter '{detail}'.");
            match dmm_lib::list_devices() {
                Ok(devices) if devices.is_empty() => {
                    msg.push_str("\n\nNo devices currently connected.");
                }
                Ok(devices) => {
                    msg.push_str("\n\nConnected devices:");
                    for (i, dev) in devices.iter().enumerate() {
                        msg.push_str(&format!("\n  [{i}] {dev}"));
                    }
                    msg.push_str("\n\nRestart with the correct --adapter value.");
                }
                Err(_) => {
                    msg.push_str(
                        "\n\nRun 'dmm-cli list' to see connected devices,\n\
                         then restart with the correct --adapter value.",
                    );
                }
            }
            ui.label(
                RichText::new(msg)
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

    /// Render the top bar: controls on the left, info/links on the right.
    ///
    /// Adaptive layout: when the window is wide enough, everything fits on
    /// a single row. When it isn't (narrow window or high zoom), the right
    /// group (version, Help, ?, settings) wraps to a second row to avoid
    /// clipping. The decision uses cached widget widths from the previous
    /// frame (egui Discussion #3468 pattern) — converges in one frame,
    /// imperceptible to the user.
    ///
    /// The right group is rendered via `show_top_bar_right` in left-to-right
    /// order so that Tab key navigation follows visual reading order
    /// (Help → ? → ⚙) rather than the reverse.
    fn show_top_bar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let tc = self.theme_colors(ui.visuals().dark_mode);
        let green = tc.status_ok();
        let orange = tc.status_warning();
        let gray = tc.status_inactive();

        // Cache left/right group widths from the previous frame to decide
        // whether both fit on one row.
        let left_id = egui::Id::new("top_bar_left_w");
        let right_id = egui::Id::new("top_bar_right_w");
        let cached_left: f32 = ui.data(|d| d.get_temp(left_id)).unwrap_or(300.0);
        let cached_right: f32 = ui.data(|d| d.get_temp(right_id)).unwrap_or(200.0);
        let spacing = ui.spacing().item_spacing.x;
        let one_row = cached_left + cached_right + spacing < ui.available_width();

        // Row 1: device label, action buttons, status indicator
        ui.horizontal(|ui| {
            let left_start = ui.cursor().left();

            let device_label = registry::find_device(&self.settings.device_family)
                .map(|d| d.display_name)
                .unwrap_or("DMM");
            ui.label(RichText::new(device_label).strong());
            ui.separator();

            match &self.connection_state {
                ConnectionState::Disconnected => {
                    if ui.button("Connect").on_hover_text("Ctrl+Shift+C").clicked() {
                        self.connect(ctx);
                    }
                }
                ConnectionState::Connected => {
                    if ui
                        .button("Disconnect")
                        .on_hover_text("Ctrl+Shift+C")
                        .clicked()
                    {
                        self.disconnect();
                    }
                    let pause_label = if self.paused {
                        "\u{25B6} Resume"
                    } else {
                        "\u{23F8} Pause"
                    };
                    if ui.button(pause_label).on_hover_text("Space").clicked() {
                        self.paused = !self.paused;
                    }
                    if ui.button("Clear").on_hover_text("Ctrl+L").clicked() {
                        self.graph.clear();
                        self.stats.reset();
                        self.integrator.reset();
                        self.last_measurement = None;
                    }
                }
                ConnectionState::Reconnecting => {
                    ui.add_enabled(false, egui::Button::new("Reconnecting..."));
                }
            }

            let (dot_color, status_text) = match &self.connection_state {
                ConnectionState::Connected => {
                    let name = self.device_name.as_deref().unwrap_or("Connected");
                    if self.paused {
                        (orange, format!("{name} (paused)"))
                    } else {
                        (green, name.to_string())
                    }
                }
                ConnectionState::Disconnected => (gray, "Disconnected".to_string()),
                ConnectionState::Reconnecting => (orange, "Reconnecting...".to_string()),
            };

            // Decorative status dot — not interactive or focusable.
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, dot_color);
            ui.label(RichText::new(status_text).small());

            // Show EXPERIMENTAL badge based on connected state or selected device.
            let device_entry = self.selected_device();
            let proto = (device_entry.new_protocol)();
            let profile = proto.profile();
            let is_experimental = if self.connection_state == ConnectionState::Connected {
                self.experimental
            } else {
                profile.stability == dmm_lib::protocol::Stability::Experimental
            };
            if is_experimental {
                let url = if self.connection_state == ConnectionState::Connected
                    && !self.feedback_url.is_empty()
                {
                    self.feedback_url.clone()
                } else {
                    profile.feedback_url()
                };
                ui.hyperlink_to(
                    RichText::new("EXPERIMENTAL").small().strong().color(orange),
                    url,
                )
                .on_hover_text(format!(
                    "{} support is experimental \u{2014} click to report feedback",
                    profile.model_name
                ));
            }

            // Toast inline on this row
            if let Some((msg, is_error, _)) = &self.toast {
                let color = if *is_error { tc.status_error() } else { green };
                ui.label(RichText::new(msg).small().color(color));
            }

            let left_width = ui.min_rect().right() - left_start;
            ui.data_mut(|d| d.insert_temp(left_id, left_width));

            // If wide enough, render right-side items on the same row
            if one_row {
                self.show_top_bar_right(ui, right_id);
            }
        });

        // If not wide enough, render right-side items on a second row
        if !one_row {
            ui.horizontal(|ui| {
                self.show_top_bar_right(ui, right_id);
            });
        }
    }

    /// Right side of the top bar: version label, Help/GitHub link, keyboard
    /// shortcut help button, and settings button.
    ///
    /// Items are added left-to-right so that egui's Tab order matches the
    /// visual reading direction. A cached-width spacer right-aligns the
    /// group without needing a right-to-left layout (which would reverse
    /// tab order). The cached width comes from the previous frame and
    /// self-corrects in one frame.
    fn show_top_bar_right(&mut self, ui: &mut Ui, cache_id: egui::Id) {
        let cached_width: f32 = ui.data(|d| d.get_temp(cache_id)).unwrap_or(200.0);
        let spacer = (ui.available_width() - cached_width).max(0.0);
        ui.add_space(spacer);
        let before = ui.cursor().left();

        let version_resp = ui.add(
            egui::Label::new(
                RichText::new(crate::version_label())
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .sense(egui::Sense::click()),
        );
        if version_resp.clicked() {
            if self.whats_new_open {
                self.whats_new_open = false;
            } else {
                self.open_whats_new();
            }
        }
        version_resp
            .on_hover_text("What's New")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.hyperlink_to(
            "Help / GitHub",
            "https://github.com/antoinecellerier/dmm-tools",
        );
        let shortcuts_btn = ui.button("?").on_hover_text("Keyboard shortcuts");
        if shortcuts_btn.clicked() {
            self.shortcut_help_open = !self.shortcut_help_open;
        }
        Self::set_accessible_label(ui, shortcuts_btn.id, "Keyboard shortcuts");

        let settings_btn = ui.button("\u{2699}").on_hover_text("Settings");
        if settings_btn.clicked() {
            self.settings_open = !self.settings_open;
        }
        Self::set_accessible_label(ui, settings_btn.id, "Settings");

        let actual_width = ui.min_rect().right() - before;
        ui.data_mut(|d| d.insert_temp(cache_id, actual_width));
    }

    /// Returns true if the app is running on a native Wayland session.
    fn is_wayland() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
    }

    /// Override the AccessKit label for a widget whose visible text is not
    /// descriptive (e.g. icon-only buttons like "⚙" or "?"). This ensures
    /// screen readers announce a meaningful name instead of the raw symbol.
    fn set_accessible_label(ui: &Ui, id: egui::Id, label: &str) {
        ui.ctx()
            .accesskit_node_builder(id, |builder| builder.set_label(label));
    }

    fn show_stats_section(&mut self, ui: &mut Ui, compact: bool, scale: f32) {
        let unit = self
            .last_measurement
            .as_ref()
            .map(|m| &*m.unit)
            .unwrap_or("");
        let integral_info = stats::integral_unit_info(unit).map(|(disp_unit, divisor)| {
            (
                self.integrator.value(),
                disp_unit,
                divisor,
                self.integrator.elapsed_secs(),
            )
        });
        let visible_integral = stats::integral_unit_info(unit).and_then(|(disp_unit, divisor)| {
            self.graph
                .visible_integral()
                .map(|raw| (raw, disp_unit, divisor, self.graph.visible_data_span_secs()))
        });
        let formatted = FormattedStats::new(
            &self.stats,
            self.graph.visible_stats(),
            unit,
            integral_info,
            visible_integral,
        );
        let main_font = 12.0 * scale;
        let sub_font = 11.0 * scale;

        if compact {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Min:{}  Max:{}  Avg:{}  ({})",
                        formatted.min, formatted.max, formatted.avg, formatted.count,
                    ))
                    .font(egui::FontId::monospace(main_font)),
                );
                if ui
                    .add(egui::Button::new(
                        RichText::new("Reset").font(egui::FontId::proportional(sub_font)),
                    ))
                    .clicked()
                {
                    self.stats.reset();
                    self.integrator.reset();
                }
            });

            if let Some(vis) = &formatted.visible {
                let vis_line = if let Some(vint) = &vis.integral {
                    format!(
                        "Visible: Min:{} Max:{} Avg:{} ({})  \u{222b}:{vint}",
                        vis.min, vis.max, vis.avg, vis.count,
                    )
                } else {
                    format!(
                        "Visible: Min:{} Max:{} Avg:{} ({})",
                        vis.min, vis.max, vis.avg, vis.count,
                    )
                };
                ui.label(
                    RichText::new(vis_line)
                        .font(egui::FontId::monospace(sub_font))
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if let Some(int) = &formatted.integral {
                ui.label(
                    RichText::new(format!("\u{222b}:{int}"))
                        .font(egui::FontId::monospace(main_font)),
                );
            }
        } else {
            ui.label(
                RichText::new("Statistics")
                    .strong()
                    .font(egui::FontId::proportional(sub_font)),
            );
            ui.label(
                RichText::new(format!("Min:{}", formatted.min))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Max:{}", formatted.max))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Avg:{}", formatted.avg))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Count: {}", formatted.count))
                    .font(egui::FontId::proportional(main_font)),
            );
            if let Some(int) = &formatted.integral {
                ui.label(
                    RichText::new(format!("\u{222b}:{int}"))
                        .font(egui::FontId::monospace(main_font)),
                );
            }
            if ui
                .add(egui::Button::new(
                    RichText::new("Reset").font(egui::FontId::proportional(sub_font)),
                ))
                .clicked()
            {
                self.stats.reset();
                self.integrator.reset();
            }

            // Windowed stats for visible graph interval
            if let Some(vis) = &formatted.visible {
                ui.add_space(4.0);
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    RichText::new("Visible")
                        .strong()
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Min:{}", vis.min))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Max:{}", vis.max))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Avg:{}", vis.avg))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Count: {}", vis.count))
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
                if let Some(vint) = &vis.integral {
                    ui.label(
                        RichText::new(format!("\u{222b}:{vint}"))
                            .font(egui::FontId::monospace(sub_font))
                            .color(weak),
                    );
                }
            }
        }
    }

    fn selected_device(&self) -> &'static registry::SelectableDevice {
        registry::resolve_device(&self.settings.device_family)
            .unwrap_or_else(registry::default_device)
    }

    fn manual_url(&self) -> Option<&'static str> {
        registry::find_device(&self.settings.device_family).and_then(|d| d.manual_url)
    }

    /// Render a specs section, calling `render_fn` when spec data is available,
    /// or showing a manual-only link as fallback.
    fn show_specs_with(
        &self,
        ui: &mut Ui,
        scale: f32,
        render_fn: fn(
            &mut Ui,
            &'static SpecInfo,
            Option<&'static ModeSpecInfo>,
            Option<&'static str>,
            f32,
        ),
    ) {
        if !self.settings.show_specs {
            return;
        }
        let manual_url = self.manual_url();
        if let Some(spec) = self.cached_spec {
            render_fn(ui, spec, self.cached_mode_spec, manual_url, scale);
        } else if let Some(url) = manual_url {
            specs::show_manual_only(ui, url, scale);
        }
    }

    /// Render specs for the wide (side panel) layout.
    fn show_specs_section(&self, ui: &mut Ui, scale: f32) {
        self.show_specs_with(ui, scale, specs::show_specs);
    }

    /// Render specs for big meter mode (pipe-separated inline).
    fn show_specs_section_inline(&self, ui: &mut Ui, scale: f32) {
        self.show_specs_with(ui, scale, specs::show_specs_inline);
    }

    /// Render specs for the narrow (compact single-line) layout.
    fn show_specs_section_compact(&self, ui: &mut Ui) {
        self.show_specs_with(ui, 1.0, specs::show_specs_compact_scaled);
    }

    fn show_recording_section(&mut self, ui: &mut Ui, compact: bool) {
        let btn_label = if self.recording.active {
            "\u{25A0} Stop"
        } else {
            "\u{25CF} Record"
        };

        ui.horizontal(|ui| {
            if ui.button(btn_label).on_hover_text("Ctrl+R").clicked() {
                self.recording.toggle();
            }
            if ui.button("Export CSV").on_hover_text("Ctrl+E").clicked() {
                self.export_csv();
            }
            let count = self.recording.samples.len();
            if self.recording.active {
                let status = format!("{count} smp | {:.0}s", self.recording.duration_secs());
                if self.recording.is_full() {
                    let warn = self
                        .theme_colors(ui.visuals().dark_mode)
                        .recording_full_warning();
                    ui.label(RichText::new(format!("{status} (buffer full)")).color(warn));
                } else {
                    ui.label(status);
                }
            } else if count > 0 {
                ui.label(format!("{count} smp"));
            }
        });

        // Scrollable sample log
        if !self.recording.samples.is_empty() {
            let max_height = if compact {
                80.0
            } else {
                ui.available_height().max(60.0)
            };
            egui::ScrollArea::vertical()
                .max_height(max_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let start = self.recording.samples.len().saturating_sub(500);
                    for s in &self.recording.samples[start..] {
                        let time = s.wall_time.format("%H:%M:%S%.3f");
                        let flags = if s.flags.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", s.flags)
                        };
                        ui.label(
                            RichText::new(format!(
                                "{time}  {val:>10} {unit}{flags}",
                                val = s.value_str,
                                unit = s.unit,
                            ))
                            .font(egui::FontId::monospace(11.0)),
                        );
                    }
                });
        }
    }

    /// Render the graph+recording area with a resizable drag separator between them.
    fn show_graph_recording_split(&mut self, ui: &mut Ui, compact: bool) {
        if self.settings.show_graph && self.settings.show_recording {
            let total = ui.available_height();
            let graph_height = (total - self.recording_height).max(80.0);

            ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                self.graph.show(ui);
            });

            let sep = ui.separator();
            let sep_id = ui.id().with("rec_resize");
            let sep_response = ui.interact(
                sep.rect.expand2(egui::vec2(0.0, 4.0)),
                sep_id,
                egui::Sense::drag(),
            );
            if sep_response.dragged() {
                self.recording_height = (self.recording_height - sep_response.drag_delta().y)
                    .clamp(40.0, (total - 80.0).max(40.0));
            }
            if sep_response.hovered() || sep_response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }

            self.show_recording_section(ui, compact);
        } else if self.settings.show_graph {
            self.graph.show(ui);
        } else if self.settings.show_recording {
            self.show_recording_section(ui, compact);
        }
    }

    fn export_csv(&mut self) {
        if self.recording.samples.is_empty() {
            return;
        }
        // Clone samples so the file dialog + write runs on a separate thread
        // without blocking the UI.
        let samples = self.recording.samples.clone();
        let device_model = self.selected_device().display_name;
        let (tx, rx) = std::sync::mpsc::channel::<(String, bool)>();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("measurements.csv")
                .add_filter("CSV", &["csv"])
                .save_file()
            {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let mut file = std::fs::File::create(&path)?;
                    writeln!(file, "# device: {device_model}")?;
                    let mut wtr = csv::Writer::from_writer(file);
                    wtr.write_record(["timestamp", "mode", "value", "unit", "range", "flags"])?;
                    for s in &samples {
                        wtr.write_record([
                            &s.wall_time.to_rfc3339(),
                            &s.mode,
                            &s.value_str,
                            &s.unit,
                            &s.range_label,
                            &s.flags,
                        ])?;
                    }
                    wtr.flush()?;
                    Ok(())
                })();
                match result {
                    Ok(()) => {
                        info!("exported {} samples to {}", samples.len(), path.display());
                        let file_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        let _ = tx.send((
                            format!("Exported {} samples to {file_name}", samples.len()),
                            false,
                        ));
                    }
                    Err(e) => {
                        error!("CSV export failed: {e}");
                        let _ = tx.send((format!("Export failed: {e}"), true));
                    }
                }
            }
        });
        self.export_result_rx = Some(rx);
    }

    fn poll_export_result(&mut self) {
        if let Some(rx) = &self.export_result_rx
            && let Ok((msg, is_error)) = rx.try_recv()
        {
            self.toast = Some((msg, is_error, Instant::now()));
            self.export_result_rx = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
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
            egui::CentralPanel::default()
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
                        self.settings.show_stats.hash(&mut h);
                        self.settings.show_specs.hash(&mut h);
                        self.big_meter_mode.hash(&mut h);
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
                            let dark = ui.visuals().dark_mode;
                            let (scale, measured_ratios) = display::show_reading_large(
                                ui,
                                self.last_measurement.as_ref(),
                                content_h,
                                &self.meter_reading_ratios,
                                self.settings.color_preset,
                                self.settings.color_overrides.for_mode(dark),
                            );
                            let after_reading = ui.cursor().top();

                            if !minimal {
                                self.show_remote_controls(ui, scale);
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
        } else if wide {
            // Wide: left side panel for reading + stats (resizable)
            egui::Panel::left("reading_panel")
                .default_size(SIDE_PANEL_DEFAULT_WIDTH)
                .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH)
                .resizable(true)
                .show_inside(ui, |ui| {
                    let dark = ui.visuals().dark_mode;
                    display::show_reading(
                        ui,
                        self.last_measurement.as_ref(),
                        self.settings.color_preset,
                        self.settings.color_overrides.for_mode(dark),
                    );
                    let controls_top = ui.cursor().top();
                    self.show_remote_controls(ui, 1.0);
                    let controls_bottom = ui.cursor().top();
                    // Overlay toggle on the last controls row, right-aligned.
                    let toggle_rect = egui::Rect::from_min_max(
                        egui::pos2(ui.max_rect().left(), controls_top),
                        egui::pos2(ui.max_rect().right(), controls_bottom),
                    );
                    self.show_big_meter_toggle_at(ui, toggle_rect);
                    self.show_connection_help(ui);
                    ui.add_space(8.0);

                    if self.cached_spec.is_some() || self.manual_url().is_some() {
                        ui.separator();
                        self.show_specs_section(ui, 1.0);
                    }

                    if self.settings.show_stats {
                        ui.separator();
                        self.show_stats_section(ui, false, 1.0);
                    }
                });

            // Wide: center panel for graph + recording
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.show_graph_recording_split(ui, false);
            });
        } else {
            // Narrow: single column
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let dark = ui.visuals().dark_mode;
                display::show_reading_compact(
                    ui,
                    self.last_measurement.as_ref(),
                    self.settings.color_preset,
                    self.settings.color_overrides.for_mode(dark),
                );
                let controls_top = ui.cursor().top();
                self.show_remote_controls(ui, 1.0);
                let controls_bottom = ui.cursor().top();
                let toggle_rect = egui::Rect::from_min_max(
                    egui::pos2(ui.max_rect().left(), controls_top),
                    egui::pos2(ui.max_rect().right(), controls_bottom),
                );
                self.show_big_meter_toggle_at(ui, toggle_rect);
                self.show_connection_help(ui);
                self.show_specs_section_compact(ui);

                if self.settings.show_stats {
                    ui.separator();
                    self.show_stats_section(ui, true, 1.0);
                }

                if self.settings.show_graph || self.settings.show_recording {
                    ui.separator();
                    self.show_graph_recording_split(ui, true);
                }
            });
        }

        self.show_shortcut_help(&ctx);
        self.show_whats_new(&ctx);

        if self.connection_state == ConnectionState::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl App {
    /// Paint the big meter toggle button at a given rect (overlay, no layout impact).
    fn show_big_meter_toggle_at(&mut self, ui: &mut Ui, rect: egui::Rect) {
        let (icon, tooltip) = match self.big_meter_mode {
            BigMeterMode::Off => ("\u{229E}", "Big meter mode (Ctrl+B)"),
            BigMeterMode::Full | BigMeterMode::Minimal => ("\u{229F}", "Exit big meter (Ctrl+B)"),
        };
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
            let color = ui.visuals().weak_text_color();
            let btn = egui::Button::new(RichText::new(icon).size(14.0).color(color));
            let response = ui.add(btn).on_hover_text(tooltip);
            Self::set_accessible_label(ui, response.id, tooltip);
            if response.clicked() {
                if self.big_meter_mode == BigMeterMode::Off {
                    // Enter big meter — use cycle_big_meter() to handle
                    // the "already_big" restore-all-panels case.
                    self.cycle_big_meter();
                } else {
                    self.big_meter_mode = BigMeterMode::Off;
                }
            }
        });
    }

    fn cycle_big_meter(&mut self) {
        match self.big_meter_mode {
            BigMeterMode::Off => {
                let already_big = !self.settings.show_graph
                    && !self.settings.show_recording
                    && !self.settings.show_stats
                    && !self.settings.show_specs;
                if already_big {
                    // All panels already hidden via settings — restore them all.
                    self.settings.show_graph = true;
                    self.settings.show_recording = true;
                    self.settings.show_stats = true;
                    self.settings.show_specs = true;
                    self.settings.save();
                } else {
                    self.big_meter_mode = BigMeterMode::Full;
                }
            }
            BigMeterMode::Full => {
                self.big_meter_mode = BigMeterMode::Minimal;
            }
            BigMeterMode::Minimal => {
                self.big_meter_mode = BigMeterMode::Off;
            }
        }
    }

    fn show_shortcut_help(&mut self, ctx: &egui::Context) {
        if !self.shortcut_help_open {
            return;
        }

        let mut open = self.shortcut_help_open;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_app")
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("General").strong());
                        ui.end_row();
                        for (key, action) in [
                            ("Ctrl+Shift+C", "Connect / Disconnect"),
                            ("Space", "Pause / Resume"),
                            ("Ctrl+L", "Clear graph & statistics"),
                            ("Ctrl+R", "Toggle recording"),
                            ("Ctrl+B", "Cycle big meter (off / full / minimal)"),
                            ("Ctrl+T", "Toggle always on top"),
                            ("Ctrl+D", "Toggle window decorations"),
                            ("Ctrl+E", "Export CSV"),
                            ("Ctrl+Plus/Minus", "Zoom in / out"),
                            ("Ctrl+0", "Reset zoom to 100%"),
                            ("Esc / Ctrl+W", "Close this help"),
                            ("Ctrl+Q", "Quit"),
                        ] {
                            ui.label(RichText::new(key).monospace());
                            ui.label(action);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);

                egui::Grid::new("shortcuts_graph")
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Graph").strong());
                        ui.end_row();
                        for (key, action) in [
                            ("[ / ]", "Shorter / longer time window"),
                            ("Left / Right", "Scroll view"),
                            ("Home", "Jump to start"),
                            ("End", "Jump to live"),
                        ] {
                            ui.label(RichText::new(key).monospace());
                            ui.label(action);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Graph and Space shortcuts are disabled when a text field has focus.",
                    )
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
            });
        self.shortcut_help_open = open;
    }

    fn open_whats_new(&mut self) {
        self.whats_new_open = true;
        self.settings.last_seen_version = Some(env!("CARGO_PKG_VERSION").to_string());
        self.settings.save();
    }

    fn show_whats_new(&mut self, ctx: &egui::Context) {
        // The viewport callback signals close via an AtomicBool.
        if self.whats_new_closed.swap(false, Ordering::Relaxed) {
            self.whats_new_open = false;
        }

        if !self.whats_new_open {
            return;
        }

        let version = env!("CARGO_PKG_VERSION");
        let title = if version.contains("-dev") {
            "What's New (Unreleased)".to_string()
        } else {
            format!("What's New in v{version}")
        };

        let closed = Arc::clone(&self.whats_new_closed);
        let cache = Arc::clone(&self.whats_new_cache);
        let viewport_id = egui::ViewportId::from_hash_of("whats_new");
        let viewport_builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([520.0, 480.0]);

        ctx.show_viewport_deferred(viewport_id, viewport_builder, move |ui, _class| {
            use egui::{Key, Modifiers};
            let ctx = ui.ctx().clone();
            let close_requested = ctx.input(|i| i.viewport().close_requested())
                || ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Escape)
                        || i.consume_key(Modifiers::COMMAND, Key::W)
                });
            if close_requested {
                closed.store(true, Ordering::Relaxed);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::changelog::show_changelog(ui, &mut cache.lock().unwrap());
                });
            });
        });
    }
}
