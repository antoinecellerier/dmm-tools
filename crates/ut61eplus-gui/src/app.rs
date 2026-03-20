use eframe::egui::{self, Color32, RichText, Ui};
use log::{error, info, warn};
use std::sync::mpsc;
use std::time::Instant;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};
use ut61eplus_lib::mock::MockMode;
use ut61eplus_lib::protocol::{DeviceFamily, Stability};
use ut61eplus_lib::transport::Transport;

use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{Settings, ThemeMode};
use crate::stats::Stats;

/// Map a device family setting value to its user-facing display name.
fn device_display_name(value: &str) -> &'static str {
    match value {
        "ut61eplus" => "UT61E+",
        "ut61b+" => "UT61B+",
        "ut61d+" => "UT61D+",
        "ut161b" => "UT161B",
        "ut161d" => "UT161D",
        "ut161e" => "UT161E",
        "ut8803" => "UT8803",
        "ut171" => "UT171",
        "ut181a" => "UT181A",
        "mock" => "Mock",
        _ => "DMM",
    }
}

/// Messages from the background thread to the UI.
pub enum DmmMessage {
    Measurement(Measurement),
    Connected {
        name: String,
        experimental: bool,
        supported_commands: Vec<String>,
    },
    Disconnected(String),
    Error(String),
    /// Waiting for meter response (consecutive timeout count).
    WaitingForMeter(u32),
}

/// Connection state.
#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connected,
    Reconnecting,
}

pub struct App {
    settings: Settings,
    settings_open: bool,

    connection_state: ConnectionState,
    device_name: Option<String>,
    /// Whether the connected protocol is experimental (unverified).
    experimental: bool,
    /// Commands supported by the connected protocol.
    supported_commands: Vec<String>,
    /// When true, incoming measurements are ignored (connection stays alive).
    paused: bool,
    last_error: Option<String>,
    /// Consecutive timeout count (0 = not waiting).
    waiting_timeouts: u32,
    last_measurement: Option<Measurement>,

    graph: Graph,
    stats: Stats,
    recording: Recording,

    rx: Option<mpsc::Receiver<DmmMessage>>,
    stop_tx: Option<mpsc::Sender<()>>,
    cmd_tx: Option<mpsc::Sender<String>>,
    first_frame: bool,
    /// Reconnect on next frame (device selection changed while connected).
    needs_reconnect: bool,
    /// OS default pixels_per_point, captured on first frame.
    os_ppp: Option<f32>,
    /// Last applied theme (to avoid re-setting every frame).
    applied_theme: Option<ThemeMode>,
    /// User-resizable recording panel height.
    recording_height: f32,
    /// Transient status toast (message, is_error, timestamp).
    toast: Option<(String, bool, Instant)>,
    /// One-shot receiver for CSV export result.
    export_result_rx: Option<mpsc::Receiver<(String, bool)>>,
    /// Cached height of non-reading content at scale=1 for big meter mode.
    meter_content_height: f32,
    /// Last window size used to compute big meter scale (recompute on change).
    meter_last_size: (u32, u32),
}

/// Run the measurement loop on a background thread, generic over transport type.
fn run_device_thread<T, F>(
    open_fn: F,
    msg_tx: mpsc::Sender<DmmMessage>,
    stop_rx: mpsc::Receiver<()>,
    cmd_rx: mpsc::Receiver<String>,
    ctx: egui::Context,
    query_name: bool,
    sample_interval_ms: u32,
) where
    T: Transport + Send + 'static,
    F: Fn() -> ut61eplus_lib::error::Result<ut61eplus_lib::Dmm<T>> + Send + 'static,
{
    info!("background thread: connecting to device");
    let mut dmm = match open_fn() {
        Ok(mut d) => {
            let profile = d.profile();
            let experimental = profile.stability == Stability::Experimental;
            let cmds: Vec<String> = profile
                .supported_commands
                .iter()
                .map(|s| s.to_string())
                .collect();
            let name = if query_name {
                d.get_name().ok().flatten().unwrap_or_default()
            } else {
                String::new()
            };
            let _ = msg_tx.send(DmmMessage::Connected {
                name,
                experimental,
                supported_commands: cmds,
            });
            ctx.request_repaint();
            d
        }
        Err(e) => {
            let _ = msg_tx.send(DmmMessage::Error(e.to_string()));
            ctx.request_repaint();
            return;
        }
    };

    let mut consecutive_timeouts: u32 = 0;
    loop {
        if stop_rx.try_recv().is_ok() {
            info!("background thread: stop signal received");
            break;
        }

        // Process any pending remote commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Err(e) = dmm.send_command(&cmd) {
                warn!("background thread: command failed: {e}");
            }
        }

        match dmm.request_measurement() {
            Ok(m) => {
                consecutive_timeouts = 0;
                if msg_tx.send(DmmMessage::Measurement(m)).is_err() {
                    break;
                }
            }
            Err(ut61eplus_lib::error::Error::Timeout) => {
                consecutive_timeouts += 1;
                warn!("background thread: measurement timeout ({consecutive_timeouts})");
                let _ = msg_tx.send(DmmMessage::WaitingForMeter(consecutive_timeouts));
                ctx.request_repaint();
                if consecutive_timeouts == 5 {
                    let _ = msg_tx.send(DmmMessage::Error(
                        "No response from meter \u{2014} check device selection and USB mode"
                            .to_string(),
                    ));
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                error!("background thread: device error: {e}");
                let _ = msg_tx.send(DmmMessage::Disconnected(e.to_string()));
                ctx.request_repaint();

                // Reconnection loop
                loop {
                    if stop_rx.try_recv().is_ok() {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    match open_fn() {
                        Ok(mut d) => {
                            let p = d.profile();
                            let exp = p.stability == Stability::Experimental;
                            let cmds: Vec<String> =
                                p.supported_commands.iter().map(|s| s.to_string()).collect();
                            let name = if query_name {
                                d.get_name().ok().flatten().unwrap_or_default()
                            } else {
                                String::new()
                            };
                            dmm = d;
                            let _ = msg_tx.send(DmmMessage::Connected {
                                name,
                                experimental: exp,
                                supported_commands: cmds,
                            });
                            ctx.request_repaint();
                            break;
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        ctx.request_repaint();
        if sample_interval_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(sample_interval_ms as u64));
        }
    }
}

fn handle_thread_panic(
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
    let _ = tx.send(DmmMessage::Error(format!("internal error: {msg}")));
    ctx.request_repaint();
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        Self {
            settings,
            settings_open: false,
            connection_state: ConnectionState::Disconnected,
            device_name: None,
            experimental: false,
            supported_commands: Vec::new(),
            paused: false,
            last_error: None,
            waiting_timeouts: 0,
            last_measurement: None,
            graph: Graph::new(),
            stats: Stats::new(),
            recording: Recording::new(),
            rx: None,
            stop_tx: None,
            cmd_tx: None,
            first_frame: true,
            needs_reconnect: false,
            os_ppp: None,
            applied_theme: None,
            recording_height: 120.0,
            toast: None,
            export_result_rx: None,
            meter_content_height: 200.0, // initial estimate, measured on first frame
            meter_last_size: (0, 0),
        }
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
        }
    }

    const ZOOM_LEVELS: &[u32] = &[
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

    fn handle_keyboard_zoom(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);
        if modifiers.command {
            // Ctrl+= or Ctrl++ (zoom in)
            if ctx.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                self.zoom_in();
            }
            // Ctrl+- (zoom out)
            if ctx.input(|i| i.key_pressed(egui::Key::Minus)) {
                self.zoom_out();
            }
            // Ctrl+0 (reset)
            if ctx.input(|i| i.key_pressed(egui::Key::Num0)) {
                self.zoom_reset();
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
        let device_family = self
            .settings
            .device_family
            .parse::<DeviceFamily>()
            .unwrap_or(DeviceFamily::Ut61EPlus);
        self.graph.set_sample_interval_ms(sample_interval_ms);

        if device_family == DeviceFamily::Mock {
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
                            Some(mode) => ut61eplus_lib::mock::open_mock_mode(mode),
                            None => ut61eplus_lib::mock::open_mock(),
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
            std::thread::spawn(move || {
                let panic_tx = msg_tx.clone();
                let panic_ctx = ctx_clone.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_device_thread(
                        move || ut61eplus_lib::open_device(device_family),
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
                    supported_commands: cmds,
                } => {
                    self.connection_state = ConnectionState::Connected;
                    self.experimental = exp;
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
                    if let MeasuredValue::Normal(v) = &m.value {
                        self.graph.push(*v, &m.mode, &m.unit);
                        self.stats.push(*v);
                    }

                    self.recording.push(&m);
                    self.last_measurement = Some(m);
                }
                DmmMessage::Disconnected(reason) => {
                    info!("UI: disconnected: {reason}");
                    self.connection_state = ConnectionState::Reconnecting;
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

    fn send_command(&self, cmd: &str) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd.to_string());
        }
    }

    fn show_remote_controls(&mut self, ui: &mut Ui, scale: f32) {
        // Only show controls when connected with measurement data and supported commands
        if self.connection_state != ConnectionState::Connected
            || self.last_measurement.is_none()
            || self.supported_commands.is_empty()
        {
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let has_cmd = |cmd: &str| self.supported_commands.iter().any(|c| c == cmd);
        let dark = ui.visuals().dark_mode;
        let active_color = if dark {
            Color32::from_rgb(100, 180, 255)
        } else {
            Color32::from_rgb(0, 100, 200)
        };

        let font_size = 12.0 * scale;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;

            let hold = flags.is_some_and(|f| f.hold);
            let rel = flags.is_some_and(|f| f.rel);
            let manual_range = flags.is_some_and(|f| !f.auto_range);
            let auto = flags.is_some_and(|f| f.auto_range);
            let min_max = flags.is_some_and(|f| f.min || f.max);
            let peak = flags.is_some_and(|f| f.peak_min || f.peak_max);

            // Commands that toggle: label, active flag, enter command, exit command.
            // Hold/rel are protocol-level toggles (same command enters and exits).
            // Min/Max and Peak have separate enter/exit wire commands — send the
            // right one based on current flag state.
            for &(label, active, enter_cmd, exit_cmd) in &[
                ("HOLD", hold, "hold", "hold"),
                ("REL", rel, "rel", "rel"),
                ("RANGE", manual_range, "range", "range"),
                ("AUTO", auto, "auto", "auto"),
                ("MIN/MAX", min_max, "minmax", "exit_minmax"),
                ("PEAK", peak, "peak", "exit_peak"),
                ("SELECT", false, "select", "select"),
                ("LIGHT", false, "light", "light"),
            ] {
                if !has_cmd(enter_cmd) {
                    continue;
                }
                let text = if active {
                    RichText::new(label)
                        .font(egui::FontId::proportional(font_size))
                        .color(active_color)
                        .strong()
                } else {
                    RichText::new(label).font(egui::FontId::proportional(font_size))
                };
                if ui.add(egui::Button::new(text)).clicked() {
                    let cmd = if active { exit_cmd } else { enter_cmd };
                    self.send_command(cmd);
                }
            }
        });
    }

    fn show_connection_help(&self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let warn_color = if dark {
            Color32::from_rgb(200, 120, 0)
        } else {
            Color32::from_rgb(180, 80, 0)
        };

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

        let is_device_not_found = error.contains("not found");

        if is_device_not_found {
            // HID device not found — dongle issue
            ui.label(RichText::new("USB adapter not found").color(warn_color));
            let platform_hint = if cfg!(target_os = "linux") {
                "Check that the CP2110 USB adapter is plugged in.\n\
                 On Linux, ensure the udev rule is installed:\n\
                 sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/\n\
                 sudo udevadm control --reload-rules\n\n\
                 Click \"Connect\" after resolving the issue."
            } else if cfg!(target_os = "windows") {
                "Check that the CP2110 USB adapter is plugged in.\n\
                 On Windows, ensure the CP2110 driver is installed.\n\
                 Download from: silabs.com/developers/usb-to-uart-bridge-vcp-drivers\n\n\
                 Click \"Connect\" after resolving the issue."
            } else {
                "Check that the CP2110 USB adapter is plugged in.\n\n\
                 Click \"Connect\" after resolving the issue."
            };
            ui.label(
                RichText::new(platform_hint)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        } else {
            // Dongle found but meter not responding
            ui.label(RichText::new("No response from meter").color(warn_color));
            let device_label = device_display_name(&self.settings.device_family);
            let family = self
                .settings
                .device_family
                .parse::<DeviceFamily>()
                .unwrap_or(DeviceFamily::Ut61EPlus);
            let instructions = format!(
                "The USB adapter is connected but the meter \n\
                 isn't responding ({device_label} selected).\n\
                 \n\
                 If this is the wrong device, change it in Settings (\u{2699}).\n\
                 Otherwise, enable data transmission:\n\
                 {}",
                family.activation_instructions()
            );
            ui.label(
                RichText::new(instructions)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }

    fn show_top_bar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            let device_label = device_display_name(&self.settings.device_family);
            ui.label(RichText::new(device_label).strong());
            ui.separator();

            match &self.connection_state {
                ConnectionState::Disconnected => {
                    if ui.button("Connect").clicked() {
                        self.connect(ctx);
                    }
                }
                ConnectionState::Connected => {
                    if ui.button("Disconnect").clicked() {
                        self.disconnect();
                    }
                    let pause_label = if self.paused {
                        "\u{25B6} Resume"
                    } else {
                        "\u{23F8} Pause"
                    };
                    if ui.button(pause_label).clicked() {
                        self.paused = !self.paused;
                    }
                    if ui.button("Clear").clicked() {
                        self.graph.clear();
                        self.stats.reset();
                        self.last_measurement = None;
                    }
                }
                ConnectionState::Reconnecting => {
                    ui.add_enabled(false, egui::Button::new("Reconnecting..."));
                }
            }

            let dark = ui.visuals().dark_mode;
            let green = if dark {
                Color32::from_rgb(60, 180, 75)
            } else {
                Color32::from_rgb(0, 140, 30)
            };
            let orange = if dark {
                Color32::from_rgb(200, 120, 0)
            } else {
                Color32::from_rgb(180, 80, 0)
            };
            let gray = if dark {
                Color32::from_rgb(150, 150, 150)
            } else {
                Color32::from_rgb(120, 120, 120)
            };

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

            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, dot_color);
            ui.label(RichText::new(status_text).small());

            if self.experimental && self.connection_state == ConnectionState::Connected {
                ui.label(RichText::new("EXPERIMENTAL").small().strong().color(orange));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("\u{2699}").clicked() {
                    self.settings_open = !self.settings_open;
                }
                ui.hyperlink_to(
                    "Help / GitHub",
                    "https://github.com/antoinecellerier/dmm-tools",
                );
                ui.label(
                    RichText::new(crate::version_label())
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                // Show toast message (export result, etc.)
                if let Some((msg, is_error, _)) = &self.toast {
                    let color = if *is_error {
                        if dark {
                            Color32::from_rgb(220, 60, 60)
                        } else {
                            Color32::from_rgb(180, 0, 0)
                        }
                    } else {
                        green
                    };
                    ui.label(RichText::new(msg).small().color(color));
                }
            });
        });
    }

    fn show_settings_panel(&mut self, ui: &mut Ui) {
        if !self.settings_open {
            return;
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Theme:");
            let mut changed = false;
            for mode in [ThemeMode::Dark, ThemeMode::Light] {
                let label = match mode {
                    ThemeMode::Dark => "Dark",
                    ThemeMode::Light => "Light",
                    ThemeMode::System => "System",
                };
                if ui
                    .selectable_label(self.settings.theme == mode, label)
                    .clicked()
                {
                    self.settings.theme = mode;
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal(|ui| {
            let mut changed = false;
            if ui
                .checkbox(&mut self.settings.show_graph, "Graph")
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(&mut self.settings.show_stats, "Statistics")
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(&mut self.settings.show_recording, "Recording")
                .changed()
            {
                changed = true;
            }
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal(|ui| {
            let mut changed = false;
            if ui
                .checkbox(&mut self.settings.auto_connect, "Auto-connect on start")
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(
                    &mut self.settings.query_device_name,
                    "Show device name on connect (beeps)",
                )
                .changed()
            {
                changed = true;
            }
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Sample interval:");
            let mut changed = false;
            for &ms in &[0u32, 100, 200, 300, 500, 1000, 2000] {
                let label = format!("{ms}ms");
                if ui
                    .selectable_label(self.settings.sample_interval_ms == ms, label)
                    .clicked()
                {
                    self.settings.sample_interval_ms = ms;
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
            }
            ui.label(
                RichText::new("(requires reconnect)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Device:");
            let mut changed = false;
            // List all supported models — each resolves to a DeviceFamily via FromStr
            for &(value, label) in &[
                ("ut61eplus", "UT61E+"),
                ("ut61b+", "UT61B+"),
                ("ut61d+", "UT61D+"),
                ("ut161b", "UT161B"),
                ("ut161d", "UT161D"),
                ("ut161e", "UT161E"),
                ("ut8803", "UT8803"),
                ("ut171", "UT171A/B/C"),
                ("ut181a", "UT181A"),
                ("mock", "Mock (simulated)"),
            ] {
                if ui
                    .selectable_label(self.settings.device_family == value, label)
                    .clicked()
                {
                    self.settings.device_family = value.to_string();
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
                // Auto-reconnect if currently connected
                if self.connection_state != ConnectionState::Disconnected {
                    self.needs_reconnect = true;
                }
            }
        });

        // Mock mode selector (only shown when mock device is selected)
        if self.settings.device_family == "mock" {
            ui.horizontal_wrapped(|ui| {
                ui.label("Mock mode:");
                let mut changed = false;
                // "Auto" = cycle through all modes
                if ui
                    .selectable_label(self.settings.mock_mode.is_empty(), "Auto (cycle)")
                    .clicked()
                {
                    self.settings.mock_mode = String::new();
                    changed = true;
                }
                for mode in MockMode::ALL {
                    let label = mode.label();
                    if ui
                        .selectable_label(self.settings.mock_mode == label, label)
                        .on_hover_text(mode.description())
                        .clicked()
                    {
                        self.settings.mock_mode = label.to_string();
                        changed = true;
                    }
                }
                if changed {
                    self.settings.save();
                    if self.connection_state != ConnectionState::Disconnected {
                        self.needs_reconnect = true;
                    }
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Zoom:");
            let mut changed = false;
            for &level in Self::ZOOM_LEVELS {
                if ui
                    .selectable_label(self.settings.zoom_pct == level, format!("{level}%"))
                    .clicked()
                {
                    self.settings.zoom_pct = level;
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+/- to adjust, Ctrl+0 = 100%)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.separator();
    }

    fn show_stats_section(&mut self, ui: &mut Ui, compact: bool, scale: f32) {
        let unit = self
            .last_measurement
            .as_ref()
            .map(|m| m.unit.as_str())
            .unwrap_or("");
        let main_font = 12.0 * scale;
        let sub_font = 11.0 * scale;

        let fmt = |v: Option<f64>| -> String {
            match v {
                Some(val) => format!("{val:>10.4} {unit}"),
                None => format!("{:>10} {unit}", "---"),
            }
        };

        // Visible window stats (from graph)
        let vis = self.graph.visible_stats();

        if compact {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Min:{}  Max:{}  Avg:{}  ({})",
                        fmt(self.stats.min),
                        fmt(self.stats.max),
                        fmt(self.stats.avg()),
                        self.stats.count,
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
                }
            });

            if let Some((vmin, vmax, vavg, vcount)) = vis {
                ui.label(
                    RichText::new(format!(
                        "Visible: Min:{} Max:{} Avg:{} ({})",
                        fmt(Some(vmin)),
                        fmt(Some(vmax)),
                        fmt(Some(vavg)),
                        vcount,
                    ))
                    .font(egui::FontId::monospace(sub_font))
                    .color(ui.visuals().weak_text_color()),
                );
            }
        } else {
            ui.label(
                RichText::new("Statistics")
                    .strong()
                    .font(egui::FontId::proportional(sub_font)),
            );
            ui.label(
                RichText::new(format!("Min:{}", fmt(self.stats.min)))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Max:{}", fmt(self.stats.max)))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Avg:{}", fmt(self.stats.avg())))
                    .font(egui::FontId::monospace(main_font)),
            );
            ui.label(
                RichText::new(format!("Count: {}", self.stats.count))
                    .font(egui::FontId::proportional(main_font)),
            );
            if ui
                .add(egui::Button::new(
                    RichText::new("Reset").font(egui::FontId::proportional(sub_font)),
                ))
                .clicked()
            {
                self.stats.reset();
            }

            // Windowed stats for visible graph interval
            if let Some((vmin, vmax, vavg, vcount)) = vis {
                ui.add_space(4.0);
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    RichText::new("Visible")
                        .strong()
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Min:{}", fmt(Some(vmin))))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Max:{}", fmt(Some(vmax))))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Avg:{}", fmt(Some(vavg))))
                        .font(egui::FontId::monospace(sub_font))
                        .color(weak),
                );
                ui.label(
                    RichText::new(format!("Count: {vcount}"))
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
            }
        }
    }

    fn show_recording_section(&mut self, ui: &mut Ui, compact: bool) {
        let btn_label = if self.recording.active {
            "\u{25A0} Stop"
        } else {
            "\u{25CF} Record"
        };

        ui.horizontal(|ui| {
            if ui.button(btn_label).clicked() {
                self.recording.toggle();
            }
            if ui.button("Export CSV").clicked() {
                self.export_csv();
            }
            let count = self.recording.samples.len();
            if self.recording.active {
                let status = format!("{count} smp | {:.0}s", self.recording.duration_secs());
                if self.recording.is_full() {
                    let dark = ui.visuals().dark_mode;
                    let warn = if dark {
                        Color32::from_rgb(230, 160, 40)
                    } else {
                        Color32::from_rgb(180, 100, 0)
                    };
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
        let (tx, rx) = std::sync::mpsc::channel::<(String, bool)>();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("measurements.csv")
                .add_filter("CSV", &["csv"])
                .save_file()
            {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let mut wtr = csv::Writer::from_path(&path)?;
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
                        let _ = tx.send((format!("Exported {} samples", samples.len()), false));
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.apply_zoom(ctx);
        self.handle_keyboard_zoom(ctx);
        self.drain_messages();
        self.poll_export_result();

        // Auto-reconnect after device selection change
        if self.needs_reconnect {
            self.needs_reconnect = false;
            self.connect(ctx);
        }

        // Expire toast after 4 seconds
        if let Some((_, _, when)) = &self.toast
            && when.elapsed().as_secs() >= 4
        {
            self.toast = None;
        }

        // Auto-connect on first frame if enabled
        if self.first_frame {
            self.first_frame = false;
            if self.settings.auto_connect {
                self.connect(ctx);
            }
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            self.show_top_bar(ui, ctx);
            self.show_settings_panel(ui);
        });

        // Determine layout mode before panels
        let wide = ctx.screen_rect().width() >= 900.0;

        let meter_only = !self.settings.show_graph && !self.settings.show_recording;

        if meter_only {
            // Big meter mode: compute scale from window size, only recalculate
            // when the window is resized to avoid frame-to-frame oscillation.
            egui::CentralPanel::default().show(ctx, |ui| {
                let size = ctx.screen_rect();
                let current_size = (size.width() as u32, size.height() as u32);
                let needs_recalc = current_size != self.meter_last_size;

                ui.centered_and_justified(|ui| {
                    ui.vertical(|ui| {
                        let scale = display::show_reading_large(
                            ui,
                            self.last_measurement.as_ref(),
                            self.meter_content_height,
                        );
                        let after_reading = ui.cursor().top();
                        self.show_remote_controls(ui, scale);
                        self.show_connection_help(ui);

                        if self.settings.show_stats {
                            ui.add_space(12.0 * scale);
                            ui.separator();
                            self.show_stats_section(ui, false, scale);
                        }

                        // Update cached height on window resize. Run twice
                        // (by not setting meter_last_size the first time) so
                        // the second pass uses the measured height from the first.
                        if needs_recalc && scale > 0.0 {
                            let total_below_reading = ui.cursor().top() - after_reading;
                            let measured = total_below_reading / scale;
                            if (self.meter_content_height - measured).abs() < 1.0 {
                                // Converged — lock in this size
                                self.meter_last_size = current_size;
                            }
                            self.meter_content_height = measured;
                        }
                    });
                });
            });
        } else if wide {
            // Wide: left side panel for reading + stats (resizable)
            egui::SidePanel::left("reading_panel")
                .default_width(240.0)
                .width_range(180.0..=400.0)
                .resizable(true)
                .show(ctx, |ui| {
                    display::show_reading(ui, self.last_measurement.as_ref());
                    self.show_remote_controls(ui, 1.0);
                    self.show_connection_help(ui);
                    ui.add_space(8.0);

                    if self.settings.show_stats {
                        ui.separator();
                        self.show_stats_section(ui, false, 1.0);
                    }
                });

            // Wide: center panel for graph + recording
            egui::CentralPanel::default().show(ctx, |ui| {
                self.show_graph_recording_split(ui, false);
            });
        } else {
            // Narrow: single column
            egui::CentralPanel::default().show(ctx, |ui| {
                display::show_reading_compact(ui, self.last_measurement.as_ref());
                self.show_remote_controls(ui, 1.0);
                self.show_connection_help(ui);

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

        if self.connection_state == ConnectionState::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
