use eframe::egui::{self, RichText, Ui};
use ut61eplus_lib::mock::MockMode;
use ut61eplus_lib::protocol::registry;

use crate::settings::{ColorOverrides, ColorPreset, HexColor, ThemeMode};
use crate::theme::ThemeColors;

use super::{App, BigMeterMode};

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
        let dark = ui.visuals().dark_mode;
        let tc = ThemeColors::new(
            dark,
            self.settings.color_preset,
            self.settings.color_overrides.for_mode(dark),
        );
        let active_color = tc.accent();

        let font_size = 12.0 * scale;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;

            let hold = flags.is_some_and(|f| f.hold);
            let rel = flags.is_some_and(|f| f.rel);
            let manual_range = flags.is_some_and(|f| !f.auto_range);
            let auto = flags.is_some_and(|f| f.auto_range);
            let min_max = flags.is_some_and(|f| f.min || f.max);
            let peak = flags.is_some_and(|f| f.peak_min || f.peak_max);

            // Simple toggle commands: label, active flag, command.
            for &(label, active, cmd) in &[
                ("HOLD", hold, "hold"),
                ("REL", rel, "rel"),
                ("RANGE", manual_range, "range"),
                ("AUTO", auto, "auto"),
            ] {
                if !has_cmd(cmd) {
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
                    self.send_command(cmd);
                }
            }

            // MIN/MAX and Peak: clicking always cycles (never exits), matching
            // the real device's short-press behavior. A separate "x" button
            // exits the mode (like the real device's long-press).
            for &(label, active, cycle_cmd, exit_cmd) in &[
                ("MIN/MAX", min_max, "minmax", "exit_minmax"),
                ("PEAK", peak, "peak", "exit_peak"),
            ] {
                if !has_cmd(cycle_cmd) {
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
                    self.send_command(cycle_cmd);
                }
                if active && has_cmd(exit_cmd) {
                    let x_text = RichText::new("x")
                        .font(egui::FontId::proportional(font_size * 0.8))
                        .color(active_color);
                    let x_btn = egui::Button::new(x_text).min_size(egui::Vec2::ZERO);
                    if ui
                        .add(x_btn)
                        .on_hover_text(format!("Exit {label}"))
                        .clicked()
                    {
                        self.send_command(exit_cmd);
                    }
                }
            }

            // Non-toggle commands
            for &(label, cmd) in &[("SELECT", "select"), ("LIGHT", "light")] {
                if !has_cmd(cmd) {
                    continue;
                }
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                if ui.add(egui::Button::new(text)).clicked() {
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
                let selected = self.settings.theme == mode;
                let base = match mode {
                    ThemeMode::Dark => "Dark",
                    ThemeMode::Light => "Light",
                    ThemeMode::System => "System",
                };
                let label = if selected && self.settings.overrides.has_theme() {
                    format!("{base} (--theme)")
                } else {
                    base.to_string()
                };
                if ui.selectable_label(selected, label).clicked() {
                    self.settings.theme = mode;
                    // Clear the override — user explicitly chose a theme
                    self.settings.overrides.theme = None;
                    changed = true;
                }
            }
            if changed {
                self.settings.save();
            }
        });

        // -- Color preset selector --
        let has_overrides = self.settings.color_overrides != ColorOverrides::default();
        ui.horizontal_wrapped(|ui| {
            ui.label("Colors:");
            let mut changed = false;
            for preset in [
                ColorPreset::Default,
                ColorPreset::HighContrast,
                ColorPreset::ColorblindSafe,
            ] {
                let selected = self.settings.color_preset == preset;
                let base = match preset {
                    ColorPreset::Default => "Default",
                    ColorPreset::HighContrast => "High Contrast",
                    ColorPreset::ColorblindSafe => "Colorblind",
                };
                let label = if selected && has_overrides {
                    format!("{base} (customized)")
                } else {
                    base.to_string()
                };
                if ui.selectable_label(selected, &label).clicked() {
                    self.settings.color_preset = preset;
                    // Clear all overrides when switching presets.
                    self.settings.color_overrides = ColorOverrides::default();
                    self.applied_ui_colors = None; // force reapply
                    changed = true;
                }
            }
            if changed {
                self.sync_graph_colors();
                self.settings.save();
            }
            if has_overrides {
                ui.label(
                    RichText::new("Selecting a preset will clear customizations")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        // -- Collapsible color customization --
        self.show_color_customization(ui);

        ui.horizontal(|ui| {
            let changed = setting_checkbox(ui, &mut self.settings.show_graph, "Graph")
                | setting_checkbox(ui, &mut self.settings.show_stats, "Statistics")
                | setting_checkbox(ui, &mut self.settings.show_recording, "Recording")
                | setting_checkbox(ui, &mut self.settings.show_specs, "Specifications");
            if changed {
                // Manual settings change exits big meter toggle.
                self.big_meter_mode = BigMeterMode::Off;
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
                let selected = self.settings.device_family == device.id;
                let label = if selected && self.settings.overrides.has_device() {
                    format!("{} (--device)", device.display_name)
                } else {
                    device.display_name.to_string()
                };
                if ui.selectable_label(selected, label).clicked() {
                    self.settings.device_family = device.id.to_string();
                    // Clear the override — user explicitly chose a device
                    self.settings.overrides.device_family = None;
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
                let has_override = self.settings.overrides.has_mock_mode();
                // "Auto" = cycle through all modes
                let auto_selected = self.settings.mock_mode.is_empty();
                let auto_label = if auto_selected && has_override {
                    "Auto (cycle) (--mock-mode)"
                } else {
                    "Auto (cycle)"
                };
                if ui.selectable_label(auto_selected, auto_label).clicked() {
                    self.settings.mock_mode = String::new();
                    changed = true;
                }
                for mode in MockMode::ALL {
                    let mode_label = mode.label();
                    let selected = self.settings.mock_mode == mode_label;
                    let label = if selected && has_override {
                        format!("{mode_label} (--mock-mode)")
                    } else {
                        mode_label.to_string()
                    };
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(mode.description())
                        .clicked()
                    {
                        self.settings.mock_mode = mode_label.to_string();
                        changed = true;
                    }
                }
                if changed {
                    // Clear the override — user explicitly chose a mock mode
                    self.settings.overrides.mock_mode = None;
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

        ui.horizontal(|ui| {
            if setting_checkbox(ui, &mut self.settings.always_on_top, "Always on top") {
                self.apply_always_on_top(ui.ctx());
                self.settings.save();
            }
            if Self::is_wayland() {
                ui.label(
                    RichText::new("(on Wayland, use the title bar menu or launch with WAYLAND_DISPLAY= to force X11)")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        ui.horizontal(|ui| {
            if setting_checkbox(
                ui,
                &mut self.settings.hide_decorations,
                "Hide window decorations",
            ) {
                self.apply_decorations(ui.ctx());
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+D to toggle)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.separator();
    }

    /// Sync graph color config from settings.
    fn sync_graph_colors(&mut self) {
        self.graph.set_color_config(
            self.settings.color_preset,
            self.settings.color_overrides.clone(),
        );
    }

    /// Show the collapsible color customization section.
    fn show_color_customization(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;

        egui::CollapsingHeader::new("Customize colors")
            .default_open(false)
            .show(ui, |ui| {
                let theme_label = if dark { "dark" } else { "light" };
                ui.label(
                    RichText::new(format!("(editing {theme_label} theme colors)"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );

                let mut changed = false;
                let preset = self.settings.color_preset;
                let overrides = self.settings.color_overrides.for_mode_mut(dark);

                // UI chrome
                ui.horizontal_wrapped(|ui| {
                    ui.label("UI:");
                    changed |= color_edit(
                        ui,
                        "Background",
                        &mut overrides.background,
                        preset,
                        dark,
                        "background",
                    );
                    changed |= color_edit(ui, "Text", &mut overrides.text, preset, dark, "text");
                    changed |=
                        color_edit(ui, "Button", &mut overrides.button, preset, dark, "button");
                });

                // Graph colors
                ui.horizontal_wrapped(|ui| {
                    ui.label("Graph:");
                    changed |= color_edit(
                        ui,
                        "Data line",
                        &mut overrides.graph_line,
                        preset,
                        dark,
                        "graph_line",
                    );
                    changed |= color_edit(
                        ui,
                        "Gap",
                        &mut overrides.graph_gap,
                        preset,
                        dark,
                        "graph_gap",
                    );
                    changed |= color_edit(
                        ui,
                        "Mean",
                        &mut overrides.graph_mean,
                        preset,
                        dark,
                        "graph_mean",
                    );
                    changed |= color_edit(
                        ui,
                        "Ref",
                        &mut overrides.graph_ref,
                        preset,
                        dark,
                        "graph_ref",
                    );
                    changed |= color_edit(
                        ui,
                        "Crossing",
                        &mut overrides.graph_crossing,
                        preset,
                        dark,
                        "graph_crossing",
                    );
                    changed |= color_edit(
                        ui,
                        "Cursor",
                        &mut overrides.graph_cursor,
                        preset,
                        dark,
                        "graph_cursor",
                    );
                    changed |= color_edit(
                        ui,
                        "Envelope",
                        &mut overrides.graph_envelope,
                        preset,
                        dark,
                        "graph_envelope",
                    );
                    changed |= color_edit(
                        ui,
                        "Plot bg",
                        &mut overrides.plot_background,
                        preset,
                        dark,
                        "plot_background",
                    );
                    changed |= color_edit(
                        ui,
                        "Crosshair",
                        &mut overrides.graph_crosshair,
                        preset,
                        dark,
                        "graph_crosshair",
                    );
                });

                // Status indicators
                ui.horizontal_wrapped(|ui| {
                    ui.label("Status:");
                    changed |= color_edit(
                        ui,
                        "Connected",
                        &mut overrides.status_ok,
                        preset,
                        dark,
                        "status_ok",
                    );
                    changed |= color_edit(
                        ui,
                        "Warning",
                        &mut overrides.status_warning,
                        preset,
                        dark,
                        "status_warning",
                    );
                    changed |= color_edit(
                        ui,
                        "Error",
                        &mut overrides.status_error,
                        preset,
                        dark,
                        "status_error",
                    );
                    changed |= color_edit(
                        ui,
                        "Inactive",
                        &mut overrides.status_inactive,
                        preset,
                        dark,
                        "status_inactive",
                    );
                    changed |=
                        color_edit(ui, "Accent", &mut overrides.accent, preset, dark, "accent");
                });

                // Minimap
                ui.horizontal_wrapped(|ui| {
                    ui.label("Minimap:");
                    changed |= color_edit(
                        ui,
                        "Viewport",
                        &mut overrides.minimap_viewport,
                        preset,
                        dark,
                        "minimap_viewport",
                    );
                });

                // Reset button
                ui.horizontal(|ui| {
                    if ui.button("Reset all to preset").clicked() {
                        self.settings.color_overrides = ColorOverrides::default();
                        changed = true;
                    }
                });

                if changed {
                    self.sync_graph_colors();
                    self.applied_ui_colors = None; // force reapply
                    self.settings.save();
                }
            });
    }
}

/// Render a color edit button with label. Returns true if the color was changed.
fn color_edit(
    ui: &mut Ui,
    label: &str,
    override_color: &mut Option<HexColor>,
    preset: ColorPreset,
    dark: bool,
    field: &str,
) -> bool {
    use crate::settings::PaletteOverrides;

    // Get the effective color (override or preset default).
    let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
    let default = tc.effective_color(field);
    let mut color = override_color.map(|h| h.0).unwrap_or(default);

    let response = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let btn = egui::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            egui::color_picker::Alpha::Opaque,
        );
        ui.label(RichText::new(label).small());
        btn
    });

    let btn_response = response.inner;
    if btn_response.changed() {
        *override_color = Some(HexColor(color));
        return true;
    }

    false
}
