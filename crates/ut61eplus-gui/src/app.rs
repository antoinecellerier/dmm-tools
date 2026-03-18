use eframe::egui::{self, Color32, RichText, Ui};
use log::{error, info, warn};
use std::sync::mpsc;
use ut61eplus_lib::command::Command;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{Settings, ThemeMode};
use crate::stats::Stats;

/// Messages from the background thread to the UI.
pub enum DmmMessage {
    Measurement(Measurement),
    Connected(String), // device name
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
    last_error: Option<String>,
    /// Consecutive timeout count (0 = not waiting).
    waiting_timeouts: u32,
    last_measurement: Option<Measurement>,

    graph: Graph,
    stats: Stats,
    recording: Recording,

    rx: Option<mpsc::Receiver<DmmMessage>>,
    stop_tx: Option<mpsc::Sender<()>>,
    cmd_tx: Option<mpsc::Sender<Command>>,
    first_frame: bool,
    /// OS default pixels_per_point, captured on first frame.
    os_ppp: Option<f32>,
    /// Last applied theme (to avoid re-setting every frame).
    applied_theme: Option<ThemeMode>,
    /// User-resizable recording panel height.
    recording_height: f32,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        Self {
            settings,
            settings_open: false,
            connection_state: ConnectionState::Disconnected,
            device_name: None,
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
            os_ppp: None,
            applied_theme: None,
            recording_height: 120.0,
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

    const ZOOM_LEVELS: &[u32] = &[30, 50, 67, 80, 90, 100, 110, 120, 133, 150, 170, 200, 240, 300];

    fn apply_zoom(&mut self, ctx: &egui::Context) {
        // Capture OS default pixels_per_point on first call
        if self.os_ppp.is_none() {
            self.os_ppp = Some(ctx.pixels_per_point());
        }
        let os_ppp = self.os_ppp.unwrap();
        let target_ppp = os_ppp * self.settings.zoom_pct as f32 / 100.0;
        // Only update when changed — setting ppp every frame resets panel resize state
        if (ctx.pixels_per_point() - target_ppp).abs() > 0.001 {
            ctx.set_pixels_per_point(target_ppp);
        }
    }

    fn zoom_in(&mut self) {
        if let Some(&next) = Self::ZOOM_LEVELS.iter().find(|&&z| z > self.settings.zoom_pct) {
            self.settings.zoom_pct = next;
            self.settings.save();
        }
    }

    fn zoom_out(&mut self) {
        if let Some(&prev) = Self::ZOOM_LEVELS.iter().rev().find(|&&z| z < self.settings.zoom_pct) {
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
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        self.rx = Some(msg_rx);
        self.stop_tx = Some(stop_tx);
        self.cmd_tx = Some(cmd_tx);
        let ctx_clone = ctx.clone();
        let query_name = self.settings.query_device_name;
        let sample_interval_ms = self.settings.sample_interval_ms;

        std::thread::spawn(move || {
            info!("background thread: connecting to device");
            let mut dmm = match ut61eplus_lib::open() {
                Ok(mut d) => {
                    let name = if query_name {
                        d.get_name().unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let _ = msg_tx.send(DmmMessage::Connected(name));
                    ctx_clone.request_repaint();
                    d
                }
                Err(e) => {
                    let _ = msg_tx.send(DmmMessage::Error(e.to_string()));
                    ctx_clone.request_repaint();
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
                    if let Err(e) = dmm.send_command(cmd) {
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
                        ctx_clone.request_repaint();
                        if consecutive_timeouts >= 5 {
                            let _ = msg_tx.send(DmmMessage::Error(
                                "No response from meter — is USB mode enabled?".to_string(),
                            ));
                            ctx_clone.request_repaint();
                        }
                    }
                    Err(e) => {
                        error!("background thread: device error: {e}");
                        let _ = msg_tx.send(DmmMessage::Disconnected(e.to_string()));
                        ctx_clone.request_repaint();

                        // Reconnection loop
                        loop {
                            if stop_rx.try_recv().is_ok() {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            match ut61eplus_lib::open() {
                                Ok(mut d) => {
                                    let name = if query_name {
                                        d.get_name().unwrap_or_default()
                                    } else {
                                        String::new()
                                    };
                                    dmm = d;
                                    let _ = msg_tx.send(DmmMessage::Connected(name));
                                    ctx_clone.request_repaint();
                                    break;
                                }
                                Err(_) => continue,
                            }
                        }
                    }
                }

                ctx_clone.request_repaint();
                if sample_interval_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(
                        sample_interval_ms as u64,
                    ));
                }
            }
        });
    }

    fn disconnect(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.rx = None;
        self.cmd_tx = None;
        self.connection_state = ConnectionState::Disconnected;
        self.device_name = None;
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
                DmmMessage::Connected(name) => {
                    self.connection_state = ConnectionState::Connected;
                    self.device_name = if name.is_empty() { None } else { Some(name.clone()) };
                    self.last_error = None;
                    info!("UI: connected to {name}");
                }
                DmmMessage::WaitingForMeter(count) => {
                    self.waiting_timeouts = count;
                }
                DmmMessage::Measurement(m) => {
                    self.last_error = None;
                    self.waiting_timeouts = 0;
                    if let MeasuredValue::Normal(v) = &m.value {
                        self.graph.push(*v, &m.mode.to_string());
                        self.stats.push(*v);
                    }

                    self.recording.push(&m);
                    self.last_measurement = Some(m);
                }
                DmmMessage::Disconnected(reason) => {
                    warn!("UI: disconnected: {reason}");
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
            self.rx = None;
            self.stop_tx = None;
        }
    }

    fn send_command(&self, cmd: Command) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd);
        }
    }

    fn show_remote_controls(&mut self, ui: &mut Ui) {
        // Only show controls when we have actual measurement data
        if self.connection_state != ConnectionState::Connected || self.last_measurement.is_none() {
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let active_color = Color32::from_rgb(100, 180, 255);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;

            let hold = flags.is_some_and(|f| f.hold);
            let rel = flags.is_some_and(|f| f.rel);
            let manual_range = flags.is_some_and(|f| !f.auto_range);
            let auto = flags.is_some_and(|f| f.auto_range);
            let min_max = flags.is_some_and(|f| f.min || f.max);
            let peak = flags.is_some_and(|f| f.peak_min || f.peak_max);

            // Buttons with protocol feedback (flag-based state)
            for &(label, active, cmd) in &[
                ("HOLD", hold, Command::Hold),
                ("REL", rel, Command::Rel),
                ("RANGE", manual_range, Command::Range),
                ("AUTO", auto, Command::Auto),
                ("MIN/MAX", min_max, Command::MinMax),
                ("PEAK", peak, Command::PeakMinMax),
            ] {
                let text = if active {
                    RichText::new(label).small().color(active_color).strong()
                } else {
                    RichText::new(label).small()
                };
                if ui.add(egui::Button::new(text)).clicked() {
                    self.send_command(cmd);
                }
            }

            // SELECT: mode cycle — no toggle state, mode is visible in reading
            if ui
                .add(egui::Button::new(RichText::new("SELECT").small()))
                .clicked()
            {
                self.send_command(Command::Select);
            }

            // LIGHT: no protocol feedback for backlight state
            if ui
                .add(egui::Button::new(RichText::new("LIGHT").small()))
                .clicked()
            {
                self.send_command(Command::Light);
            }
        });
    }

    fn show_connection_help(&self, ui: &mut Ui) {
        // Show waiting indicator before error threshold
        if self.waiting_timeouts > 0 && self.last_error.is_none() {
            ui.add_space(4.0);
            let dots = ".".repeat((self.waiting_timeouts as usize % 4) + 1);
            ui.label(
                RichText::new(format!("Waiting for meter{dots}"))
                    .color(Color32::from_rgb(230, 160, 40)),
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
            ui.label(
                RichText::new("USB adapter not found")
                    .color(Color32::from_rgb(230, 160, 40)),
            );
            ui.label(
                RichText::new(
                    "Check that the CP2110 USB adapter is plugged in.\n\
                     On Linux, ensure the udev rule is installed:\n\
                     sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/\n\
                     sudo udevadm control --reload-rules\n\n\
                     Click \"Connect\" after resolving the issue."
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        } else {
            // Dongle found but meter not responding
            ui.label(
                RichText::new("No response from meter")
                    .color(Color32::from_rgb(230, 160, 40)),
            );
            ui.label(
                RichText::new(
                    "The USB adapter is connected but the meter \n\
                     isn't responding. To enable data transmission:\n\
                     1. Insert the USB module into the meter\n\
                     2. Turn the meter on\n\
                     3. Long press the USB/Hz button\n\
                     4. The S icon appears on the LCD"
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        }
    }

    fn show_top_bar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("UT61E+").strong());
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

            let (dot_color, status_text) = match &self.connection_state {
                ConnectionState::Connected => {
                    let name = self.device_name.as_deref().unwrap_or("Connected");
                    (Color32::from_rgb(60, 180, 75), name.to_string())
                }
                ConnectionState::Disconnected => {
                    (Color32::from_rgb(150, 150, 150), "Disconnected".to_string())
                }
                ConnectionState::Reconnecting => {
                    (Color32::from_rgb(230, 160, 40), "Reconnecting...".to_string())
                }
            };

            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, dot_color);
            ui.label(RichText::new(status_text).small());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("\u{2699}").clicked() {
                    self.settings_open = !self.settings_open;
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
                if ui.selectable_label(self.settings.theme == mode, label).clicked() {
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
            if ui.checkbox(&mut self.settings.show_graph, "Graph").changed() {
                changed = true;
            }
            if ui.checkbox(&mut self.settings.show_stats, "Statistics").changed() {
                changed = true;
            }
            if ui.checkbox(&mut self.settings.show_recording, "Recording").changed() {
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

        ui.horizontal(|ui| {
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

        ui.horizontal(|ui| {
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

    fn show_stats_section(&mut self, ui: &mut Ui, compact: bool) {
        let unit = self
            .last_measurement
            .as_ref()
            .map(|m| m.unit.as_str())
            .unwrap_or("");

        let fmt = |v: Option<f64>| -> String {
            match v {
                Some(val) => format!("{val:>10.4} {unit}"),
                None => format!("{:>10} {unit}", "---"),
            }
        };

        // Visible window stats (from graph)
        let vis = self.graph.visible_stats();

        if compact {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Min:{}  Max:{}  Avg:{}  ({})",
                        fmt(self.stats.min),
                        fmt(self.stats.max),
                        fmt(self.stats.avg()),
                        self.stats.count,
                    ))
                    .font(egui::FontId::monospace(12.0)),
                );
                if ui.small_button("Reset").clicked() {
                    self.stats.reset();
                }
            });

            if let Some((vmin, vmax, vavg, vcount)) = vis {
                ui.label(
                    RichText::new(format!(
                        "View: Min:{} Max:{} Avg:{} ({})",
                        fmt(Some(vmin)),
                        fmt(Some(vmax)),
                        fmt(Some(vavg)),
                        vcount,
                    ))
                    .font(egui::FontId::monospace(10.0))
                    .color(ui.visuals().weak_text_color()),
                );
            }
        } else {
            ui.label(RichText::new("Statistics").strong().small());
            ui.label(
                RichText::new(format!("Min:{}", fmt(self.stats.min)))
                    .font(egui::FontId::monospace(12.0)),
            );
            ui.label(
                RichText::new(format!("Max:{}", fmt(self.stats.max)))
                    .font(egui::FontId::monospace(12.0)),
            );
            ui.label(
                RichText::new(format!("Avg:{}", fmt(self.stats.avg())))
                    .font(egui::FontId::monospace(12.0)),
            );
            ui.label(format!("Count: {}", self.stats.count));
            if ui.small_button("Reset").clicked() {
                self.stats.reset();
            }

            // Windowed stats for visible graph interval
            if let Some((vmin, vmax, vavg, vcount)) = vis {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Visible window")
                        .strong()
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                ui.label(
                    RichText::new(format!(
                        "Min:{} Max:{} Avg:{} ({})",
                        fmt(Some(vmin)),
                        fmt(Some(vmax)),
                        fmt(Some(vavg)),
                        vcount,
                    ))
                    .font(egui::FontId::monospace(10.0))
                    .color(ui.visuals().weak_text_color()),
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
                ui.label(format!("{count} smp | {:.0}s", self.recording.duration_secs()));
            } else if count > 0 {
                ui.label(format!("{count} smp"));
            }
        });

        // Scrollable sample log
        if !self.recording.samples.is_empty() {
            let max_height = if compact { 80.0 } else { ui.available_height().max(60.0) };
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

    fn export_csv(&self) {
        if self.recording.samples.is_empty() {
            return;
        }
        // Clone samples so the file dialog + write runs on a separate thread
        // without blocking the UI.
        let samples = self.recording.samples.clone();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("measurements.csv")
                .add_filter("CSV", &["csv"])
                .save_file()
            {
                let mut wtr = match csv::Writer::from_path(&path) {
                    Ok(w) => w,
                    Err(e) => {
                        error!("CSV export failed: {e}");
                        return;
                    }
                };
                if wtr
                    .write_record(["timestamp", "mode", "value", "unit", "range", "flags"])
                    .is_err()
                {
                    return;
                }
                for s in &samples {
                    let _ = wtr.write_record([
                        &s.wall_time.to_rfc3339(),
                        &s.mode,
                        &s.value_str,
                        &s.unit,
                        &s.range_label,
                        &s.flags,
                    ]);
                }
                let _ = wtr.flush();
                info!("exported {} samples to {}", samples.len(), path.display());
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.apply_zoom(ctx);
        self.handle_keyboard_zoom(ctx);
        self.drain_messages();

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

        if wide {
            // Wide: left side panel for reading + stats (resizable)
            egui::SidePanel::left("reading_panel")
                .default_width(240.0)
                .width_range(180.0..=400.0)
                .resizable(true)
                .show(ctx, |ui| {
                    display::show_reading(ui, self.last_measurement.as_ref());
                    self.show_remote_controls(ui);
                    self.show_connection_help(ui);
                    ui.add_space(8.0);

                    if self.settings.show_stats {
                        ui.separator();
                        self.show_stats_section(ui, false);
                    }
                });

            // Wide: center panel for graph + recording
            egui::CentralPanel::default().show(ctx, |ui| {
                if self.settings.show_graph && self.settings.show_recording {
                    // Split: graph on top, recording on bottom with drag separator
                    let total = ui.available_height();
                    let graph_height = (total - self.recording_height).max(80.0);

                    ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                        self.graph.show(ui, 0.0);
                    });

                    // Drag handle separator
                    let sep = ui.separator();
                    let sep_id = ui.id().with("rec_resize");
                    let sep_response = ui.interact(sep.rect.expand2(egui::vec2(0.0, 4.0)), sep_id, egui::Sense::drag());
                    if sep_response.dragged() {
                        self.recording_height = (self.recording_height - sep_response.drag_delta().y)
                            .clamp(40.0, total - 80.0);
                    }
                    if sep_response.hovered() || sep_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    self.show_recording_section(ui, false);
                } else if self.settings.show_graph {
                    self.graph.show(ui, 0.0);
                } else if self.settings.show_recording {
                    self.show_recording_section(ui, false);
                }
            });
        } else {
            // Narrow: single column
            egui::CentralPanel::default().show(ctx, |ui| {
                display::show_reading_compact(ui, self.last_measurement.as_ref());
                self.show_remote_controls(ui);
                self.show_connection_help(ui);

                if self.settings.show_stats {
                    ui.separator();
                    self.show_stats_section(ui, true);
                }

                if self.settings.show_graph && self.settings.show_recording {
                    let total = ui.available_height();
                    let graph_height = (total - self.recording_height).max(80.0);

                    ui.separator();
                    ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                        self.graph.show(ui, 0.0);
                    });

                    let sep = ui.separator();
                    let sep_id = ui.id().with("rec_resize_narrow");
                    let sep_response = ui.interact(sep.rect.expand2(egui::vec2(0.0, 4.0)), sep_id, egui::Sense::drag());
                    if sep_response.dragged() {
                        self.recording_height = (self.recording_height - sep_response.drag_delta().y)
                            .clamp(40.0, total - 80.0);
                    }
                    if sep_response.hovered() || sep_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                    }

                    self.show_recording_section(ui, true);
                } else if self.settings.show_graph {
                    ui.separator();
                    self.graph.show(ui, 0.0);
                } else if self.settings.show_recording {
                    ui.separator();
                    self.show_recording_section(ui, true);
                }
            });
        }

        if self.connection_state == ConnectionState::Connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
