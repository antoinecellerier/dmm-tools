use eframe::egui::{self, Color32, RichText, Ui};
use log::{error, info, warn};
use std::sync::mpsc;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

use crate::display;
use crate::graph::Graph;
use crate::recording::Recording;
use crate::settings::{GraphTimeWindow, Settings, ThemeMode};
use crate::stats::Stats;

/// Messages from the background thread to the UI.
pub enum DmmMessage {
    Measurement(Measurement),
    Connected(String), // device name
    Disconnected(String),
    Error(String),
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
    last_measurement: Option<Measurement>,

    graph: Graph,
    stats: Stats,
    recording: Recording,

    rx: Option<mpsc::Receiver<DmmMessage>>,
    stop_tx: Option<mpsc::Sender<()>>,
    first_frame: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        Self {
            settings,
            settings_open: false,
            connection_state: ConnectionState::Disconnected,
            device_name: None,
            last_measurement: None,
            graph: Graph::new(),
            stats: Stats::new(),
            recording: Recording::new(),
            rx: None,
            stop_tx: None,
            first_frame: true,
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        match self.settings.theme {
            ThemeMode::Dark => ctx.set_visuals(egui::Visuals::dark()),
            ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            ThemeMode::System => ctx.set_visuals(egui::Visuals::dark()),
        }
    }

    fn connect(&mut self, ctx: &egui::Context) {
        self.disconnect();

        let (msg_tx, msg_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();
        self.rx = Some(msg_rx);
        self.stop_tx = Some(stop_tx);
        let ctx_clone = ctx.clone();
        let query_name = self.settings.query_device_name;

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

            loop {
                if stop_rx.try_recv().is_ok() {
                    info!("background thread: stop signal received");
                    break;
                }

                match dmm.request_measurement() {
                    Ok(m) => {
                        if msg_tx.send(DmmMessage::Measurement(m)).is_err() {
                            break;
                        }
                    }
                    Err(ut61eplus_lib::error::Error::Timeout) => {
                        warn!("background thread: measurement timeout");
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
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        });
    }

    fn disconnect(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.rx = None;
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
                    // Don't clear graph/stats on reconnect — timeline is continuous.
                    // User can manually Clear if they want a fresh start.
                    info!("UI: connected to {name}");
                }
                DmmMessage::Measurement(m) => {
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
            ui.label("Graph window:");
            let mut changed = false;
            for window in GraphTimeWindow::ALL {
                if ui
                    .selectable_label(self.settings.graph_time_window == *window, window.label())
                    .clicked()
                {
                    self.settings.graph_time_window = *window;
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
                Some(val) => format!("{val:.4} {unit}"),
                None => "---".to_string(),
            }
        };

        if compact {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Min: {}  Max: {}  Avg: {}  ({})",
                    fmt(self.stats.min),
                    fmt(self.stats.max),
                    fmt(self.stats.avg()),
                    self.stats.count,
                ));
                if ui.small_button("Reset").clicked() {
                    self.stats.reset();
                }
            });
        } else {
            ui.label(RichText::new("Statistics").strong().small());
            ui.label(format!("Min: {}", fmt(self.stats.min)));
            ui.label(format!("Max: {}", fmt(self.stats.max)));
            ui.label(format!("Avg: {}", fmt(self.stats.avg())));
            ui.label(format!("Count: {}", self.stats.count));
            if ui.small_button("Reset").clicked() {
                self.stats.reset();
            }
        }
    }

    fn show_recording_section(&mut self, ui: &mut Ui, compact: bool) {
        let btn_label = if self.recording.active {
            "\u{25A0} Stop"
        } else {
            "\u{25CF} Record"
        };

        if compact {
            ui.horizontal(|ui| {
                if ui.button(btn_label).clicked() {
                    self.recording.toggle();
                }
                if ui.button("Export").clicked() {
                    self.export_csv();
                }
                ui.label(format!("{} smp", self.recording.samples.len()));
                if self.recording.active {
                    ui.label(format!("{:.0}s", self.recording.duration_secs()));
                }
            });
        } else {
            ui.horizontal(|ui| {
                if ui.button(btn_label).clicked() {
                    self.recording.toggle();
                }
                if ui.button("Export CSV").clicked() {
                    self.export_csv();
                }
            });
            ui.label(format!("Samples: {}", self.recording.samples.len()));
            if self.recording.active {
                ui.label(format!("Duration: {:.0}s", self.recording.duration_secs()));
            }
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
            // Wide: left side panel for reading + stats
            egui::SidePanel::left("reading_panel")
                .min_width(220.0)
                .max_width(280.0)
                .resizable(false)
                .show(ctx, |ui| {
                    display::show_reading(ui, self.last_measurement.as_ref());
                    ui.add_space(8.0);

                    if self.settings.show_stats {
                        ui.separator();
                        self.show_stats_section(ui, false);
                    }
                });

            // Wide: center panel for graph + recording
            egui::CentralPanel::default().show(ctx, |ui| {
                if self.settings.show_graph {
                    // Give graph most of the space
                    let graph_height = ui.available_height()
                        - if self.settings.show_recording { 60.0 } else { 0.0 };
                    let graph_height = graph_height.max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                        self.graph.show(ui, self.settings.graph_time_window.as_secs());
                    });
                }

                if self.settings.show_recording {
                    ui.separator();
                    self.show_recording_section(ui, false);
                }
            });
        } else {
            // Narrow: single column — stats under reading (they're related)
            egui::CentralPanel::default().show(ctx, |ui| {
                display::show_reading_compact(ui, self.last_measurement.as_ref());

                if self.settings.show_stats {
                    ui.separator();
                    self.show_stats_section(ui, true);
                }

                if self.settings.show_graph {
                    ui.separator();
                    let graph_height = (ui.available_height() * 0.5).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                        self.graph.show(ui, self.settings.graph_time_window.as_secs());
                    });
                }

                if self.settings.show_recording {
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
