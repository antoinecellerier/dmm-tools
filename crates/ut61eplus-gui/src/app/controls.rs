use eframe::egui::{self, RichText, Ui};
use ut61eplus_lib::mock::MockMode;
use ut61eplus_lib::protocol::registry;

use crate::settings::ThemeMode;
use crate::theme::ThemeColors;

use super::App;

/// Show a settings checkbox; returns `true` if the value changed.
fn setting_checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> bool {
    ui.checkbox(value, label).changed()
}

impl App {
    pub(super) fn show_remote_controls(&mut self, ui: &mut Ui, scale: f32) {
        use super::ConnectionState;

        // Only show controls when connected with measurement data and supported commands
        if self.connection_state != ConnectionState::Connected
            || self.last_measurement.is_none()
            || self.supported_commands.is_empty()
        {
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let has_cmd = |cmd: &str| self.supported_commands.iter().any(|c| c == cmd);
        let tc = ThemeColors::new(ui.visuals().dark_mode);
        let active_color = tc.blue_accent();

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

    pub(super) fn show_settings_panel(&mut self, ui: &mut Ui) {
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
            let changed = setting_checkbox(ui, &mut self.settings.show_graph, "Graph")
                | setting_checkbox(ui, &mut self.settings.show_stats, "Statistics")
                | setting_checkbox(ui, &mut self.settings.show_recording, "Recording")
                | setting_checkbox(ui, &mut self.settings.show_specs, "Specifications");
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal(|ui| {
            let changed =
                setting_checkbox(ui, &mut self.settings.auto_connect, "Auto-connect on start")
                    | setting_checkbox(
                        ui,
                        &mut self.settings.query_device_name,
                        "Show device name on connect (beeps)",
                    );
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
            for device in registry::DEVICES {
                if ui
                    .selectable_label(
                        self.settings.device_family == device.id,
                        device.display_name,
                    )
                    .clicked()
                {
                    self.settings.device_family = device.id.to_string();
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
                // Auto-reconnect if currently connected
                if self.connection_state != super::ConnectionState::Disconnected {
                    self.needs_reconnect = true;
                }
            }
        });

        // Mock mode selector (only shown when mock device is selected)
        if registry::find_device(&self.settings.device_family).is_some_and(|d| d.id == "mock") {
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
                    if self.connection_state != super::ConnectionState::Disconnected {
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
}
