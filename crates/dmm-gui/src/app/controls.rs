use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;
use crate::settings::{ColorOverrides, ColorPreset, HexColor, ThemeMode};
use crate::theme::ThemeColors;

use super::{App, BigMeterMode};

/// Show a settings checkbox with a hover tooltip; returns `true` if the value changed.
fn setting_checkbox(ui: &mut Ui, value: &mut bool, label: &str, tooltip: &str) -> bool {
    ui.checkbox(value, label).on_hover_text(tooltip).changed()
}

/// Show a selectable_label with a hover tooltip; returns `true` if clicked.
fn setting_selectable(
    ui: &mut Ui,
    selected: bool,
    label: impl Into<egui::WidgetText>,
    tooltip: &str,
) -> bool {
    ui.selectable_label(selected, label)
        .on_hover_text(tooltip)
        .clicked()
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

            // Simple toggle commands: label, active flag, command, tooltip.
            // Tooltips are phrased to be device-agnostic — they describe
            // the generic DMM behavior, not model-specific details.
            for &(label, active, cmd, tooltip) in &[
                ("HOLD", hold, "hold", "Freeze the current reading"),
                (
                    "REL",
                    rel,
                    "rel",
                    "Show readings relative to the current value",
                ),
                (
                    "RANGE",
                    manual_range,
                    "range",
                    "Step to the next manual range",
                ),
                ("AUTO", auto, "auto", "Return the meter to auto-range"),
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
                let resp = ui
                    .add(egui::Button::new(text))
                    .on_hover_text(tooltip)
                    .a11y_toggled(active);
                if resp.clicked() {
                    self.send_command(cmd);
                }
            }

            // MIN/MAX and Peak: clicking always cycles (never exits), matching
            // the real device's short-press behavior. A separate "x" button
            // exits the mode (like the real device's long-press).
            for &(label, active, cycle_cmd, exit_cmd, tooltip) in &[
                (
                    "MIN/MAX",
                    min_max,
                    "minmax",
                    "exit_minmax",
                    "Record minimum, maximum, and average readings — click to cycle, × to exit",
                ),
                (
                    "PEAK",
                    peak,
                    "peak",
                    "exit_peak",
                    "Capture peak minimum and maximum — click to cycle, × to exit",
                ),
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
                let resp = ui
                    .add(egui::Button::new(text))
                    .on_hover_text(tooltip)
                    .a11y_toggled(active);
                if resp.clicked() {
                    self.send_command(cycle_cmd);
                }
                if active && has_cmd(exit_cmd) {
                    let x_text = RichText::new("x")
                        .font(egui::FontId::proportional(font_size * 0.8))
                        .color(active_color);
                    let x_btn = egui::Button::new(x_text).min_size(egui::Vec2::ZERO);
                    let exit_label = format!("Exit {label} mode");
                    let x_resp = ui
                        .add(x_btn)
                        .on_hover_text(exit_label.clone())
                        .a11y_label(&exit_label);
                    if x_resp.clicked() {
                        self.send_command(exit_cmd);
                    }
                }
            }

            // Non-toggle commands
            for &(label, cmd, tooltip) in &[
                (
                    "SELECT",
                    "select",
                    "Cycle through the secondary functions of the current dial position",
                ),
                ("LIGHT", "light", "Toggle the meter's backlight"),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(tooltip)
                    .clicked()
                {
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
                let tooltip = format!("Use {base} mode for the whole GUI");
                if setting_selectable(ui, selected, label, &tooltip) {
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
                let tooltip = match preset {
                    ColorPreset::Default => "Balanced palette tuned for everyday use",
                    ColorPreset::HighContrast => {
                        "Maximum-contrast palette for bright lighting or projectors"
                    }
                    ColorPreset::ColorblindSafe => {
                        "Palette that stays distinguishable for protan/deutan vision"
                    }
                };
                if setting_selectable(ui, selected, &label, tooltip) {
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
            let changed = setting_checkbox(
                ui,
                &mut self.settings.show_graph,
                "Graph",
                "Show the rolling time-series plot",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_stats,
                "Statistics",
                "Show Min / Max / Avg / integral for the live session",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_recording,
                "Recording",
                "Show the recording controls and sample log",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_specs,
                "Specifications",
                "Show accuracy and resolution for the current mode",
            );
            if changed {
                // Manual settings change exits big meter toggle.
                self.big_meter_mode = BigMeterMode::Off;
                self.settings.save();
            }
        });

        ui.horizontal(|ui| {
            let changed = setting_checkbox(
                ui,
                &mut self.settings.auto_connect,
                "Auto-connect on start",
                "Open the USB connection automatically when the app launches",
            ) | setting_checkbox(
                ui,
                &mut self.settings.query_device_name,
                "Show device name on connect (beeps)",
                "Query the meter's name after connecting — the meter will beep once",
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
                let tooltip = if ms == 0 {
                    "No rate limit — read as fast as the meter reports (requires reconnect)"
                        .to_string()
                } else {
                    format!("Wait {ms} ms between samples (requires reconnect)")
                };
                if setting_selectable(ui, self.settings.sample_interval_ms == ms, label, &tooltip) {
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
                let selected = self.settings.shared.device_family == device.id;
                let label = if selected && self.settings.overrides.has_device() {
                    format!("{} (--device)", device.display_name)
                } else {
                    device.display_name.to_string()
                };
                let tooltip = format!("Talk to a {} over USB", device.display_name);
                if setting_selectable(ui, selected, label, &tooltip) {
                    self.settings.shared.device_family = device.id.to_string();
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
        if registry::find_device(&self.settings.shared.device_family)
            .is_some_and(|d| d.id == "mock")
        {
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
                if setting_selectable(
                    ui,
                    auto_selected,
                    auto_label,
                    "Cycle through all synthetic modes to exercise the GUI",
                ) {
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
                let tooltip = if level == 100 {
                    "Scale the GUI to 100% (Ctrl+0, or Ctrl+/- to step)".to_string()
                } else {
                    format!("Scale the GUI to {level}% (Ctrl+/- to step)")
                };
                if setting_selectable(
                    ui,
                    self.settings.zoom_pct == level,
                    format!("{level}%"),
                    &tooltip,
                ) {
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
            if setting_checkbox(
                ui,
                &mut self.settings.always_on_top,
                "Always on top",
                "Keep the window above other desktop windows (Ctrl+T)",
            ) {
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
                "Borderless window — use Ctrl+D to toggle back",
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

        let collapsing = egui::CollapsingHeader::new("Customize colors")
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
                    if ui
                        .button("Reset all to preset")
                        .on_hover_text(
                            "Discard color customizations and return to the selected preset",
                        )
                        .clicked()
                    {
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
        // Paint an explicit focus ring on the header when Tab-focused —
        // egui's CollapsingHeader shows only a subtle highlight otherwise,
        // which is easy to miss.
        crate::a11y::paint_focus_ring(ui, &collapsing.header_response);
        collapsing
            .header_response
            .on_hover_text("Per-color overrides on top of the selected preset");
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

    // Render the swatch as a plain Button with an explicit fill, so we control
    // the open lifecycle. egui's `color_edit_button_srgba` also uses
    // `Popup::menu`, but doesn't move focus into the popup when it opens —
    // keyboard users end up stranded on the settings panel.
    let response = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let btn_size = egui::Vec2::splat(ui.spacing().interact_size.y);
        let btn = ui.add(egui::Button::new("").fill(color).min_size(btn_size));
        ui.label(RichText::new(label).small());
        btn
    });

    // The swatch's visible content is just a color, which screen readers
    // can't describe — give it the label text as its accessible name.
    let btn_response = response
        .inner
        .on_hover_text(color_edit_tooltip(field))
        .a11y_label(label);
    // The fill covers the usual button border, so paint an explicit focus
    // ring when the swatch is keyboard-focused.
    crate::a11y::paint_focus_ring(ui, &btn_response);

    let popup_id = btn_response.id.with("color_popup");
    // "This click is the one that opens the popup" — true only on the click
    // frame *and* only when the popup was closed at the start of the frame.
    // `Popup::menu` flips the memory state inside its own `show` call, so at
    // this point `is_id_open` still returns the pre-toggle value.
    let newly_opened = btn_response.clicked() && !egui::Popup::is_id_open(ui.ctx(), popup_id);

    // Popup has no built-in Esc handling (unlike egui::Modal), so consume
    // Esc manually while the popup is open.
    if egui::Popup::is_id_open(ui.ctx(), popup_id)
        && ui
            .ctx()
            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        egui::Popup::close_id(ui.ctx(), popup_id);
    }

    // Track open state across frames to detect close transitions (click
    // outside, Esc, or swatch re-click) so focus can be restored to the
    // swatch regardless of how the popup closed.
    let was_open_key = btn_response.id.with("color_popup_was_open");
    let was_open: bool = ui.ctx().data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    let mut color_changed = false;
    let hsva_cache_key = btn_response.id.with("hsva_cache");
    egui::Popup::menu(&btn_response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            // Trap Tab focus to the popup's layer. Without this, Tab cycles
            // through main-settings widgets (which are registered earlier in
            // the frame) rather than through the picker's drag values and
            // sliders.
            ui.ctx().memory_mut(|m| m.set_modal_layer(ui.layer_id()));

            // Invisible focusable anchor. When the popup first opens, focus
            // is still on the swatch — in a layer *below* the modal layer and
            // therefore no longer focusable — so Tab wouldn't advance to
            // anything. Requesting focus on this anchor puts the user inside
            // the popup's focus cycle immediately.
            let anchor =
                ui.add(egui::Label::new("").sense(egui::Sense::focusable_noninteractive()));
            if newly_opened {
                anchor.request_focus();
            }

            // HSVA is the source of truth while the popup is open.
            // Converting srgba → Hsva each frame is slightly lossy (sRGB
            // gamma), so we cache the Hsva in ctx temp data and only seed
            // from the current color on first open.
            let mut hsva: egui::ecolor::Hsva = if newly_opened {
                egui::ecolor::Hsva::from(color)
            } else {
                ui.ctx()
                    .data(|d| d.get_temp::<egui::ecolor::Hsva>(hsva_cache_key))
                    .unwrap_or_else(|| egui::ecolor::Hsva::from(color))
            };

            // Arrow-key HSV adjustment. egui's `color_slider_1d` and
            // `color_slider_2d` only handle `interact_pointer_pos` (mouse
            // drag), so Tab-focused sliders do nothing on arrow press.
            // Detect which slider currently has focus via the rect shape
            // of the focused widget (2D slider is square, hue slider is
            // wide-and-short) and apply the arrow deltas directly to
            // `hsva` before the picker renders. The size thresholds are
            // chosen to include 100-wide sliders (observed in this app's
            // theme) while excluding small toggle/drag widgets (~20 px).
            if let Some(fid) = ui.ctx().memory(|m| m.focused())
                && fid != btn_response.id
                && let Some(focused_resp) = ui.ctx().read_response(fid)
            {
                let rect = focused_resp.rect;
                let is_2d_slider =
                    rect.width() >= 50.0 && (rect.width() - rect.height()).abs() < 2.0;
                let is_hue_slider = rect.width() >= 50.0 && rect.width() > rect.height() * 3.0;

                let left = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
                let right = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
                let up = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                let down = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));

                let step = 0.02;
                let dx = if right { step } else { 0.0 } - if left { step } else { 0.0 };
                let dy = if up { step } else { 0.0 } - if down { step } else { 0.0 };

                if is_2d_slider && (dx != 0.0 || dy != 0.0) {
                    hsva.s = (hsva.s + dx).clamp(0.0, 1.0);
                    hsva.v = (hsva.v + dy).clamp(0.0, 1.0);
                    color_changed = true;
                } else if is_hue_slider && dx != 0.0 {
                    hsva.h = (hsva.h + dx).rem_euclid(1.0);
                    color_changed = true;
                }
            }

            color_changed |= egui::color_picker::color_picker_hsva_2d(
                ui,
                &mut hsva,
                egui::color_picker::Alpha::Opaque,
            );

            // Write back to Color32 and persist Hsva for next frame.
            color = egui::Color32::from(hsva);
            ui.ctx().data_mut(|d| d.insert_temp(hsva_cache_key, hsva));
        });

    let is_open_now = egui::Popup::is_id_open(ui.ctx(), popup_id);
    if is_open_now {
        // Trap arrow keys on whichever widget inside the popup currently
        // has focus. egui's color_slider_1d/2d (hue + saturation-value) are
        // focusable but don't respond to arrow keys; without trapping, the
        // first arrow press Tab-jumps focus off the slider spatially.
        // Trapping keeps focus inside the popup so the user can still Tab
        // between sliders and the RGBA drag values (which ARE keyboard-
        // adjustable via Enter-to-edit + Up/Down). See the "Known
        // limitations" note in docs/gui-reference.md on the color picker.
        if let Some(focused_id) = ui.ctx().memory(|m| m.focused())
            && focused_id != btn_response.id
        {
            ui.ctx().memory_mut(|m| {
                m.set_focus_lock_filter(
                    focused_id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
            // `set_focus_lock_filter` only takes effect on the *next*
            // frame because of its `had_focus_last_frame` gate. Cover the
            // first-frame case by also resetting `focus_direction` if any
            // arrow is held this frame.
            let any_arrow_down = ui.ctx().input(|i| {
                i.key_down(egui::Key::ArrowLeft)
                    || i.key_down(egui::Key::ArrowRight)
                    || i.key_down(egui::Key::ArrowUp)
                    || i.key_down(egui::Key::ArrowDown)
            });
            if any_arrow_down {
                ui.ctx()
                    .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }
    }
    if was_open && !is_open_now {
        // Popup just closed — put focus back on the swatch so keyboard users
        // don't get teleported to the top of the Tab order.
        ui.ctx().memory_mut(|m| m.request_focus(btn_response.id));
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(was_open_key, is_open_now));

    if color_changed {
        *override_color = Some(HexColor(color));
        return true;
    }

    false
}

/// Hover text for a color override button, keyed on the internal field name.
fn color_edit_tooltip(field: &str) -> &'static str {
    match field {
        "background" => "Panel background color",
        "text" => "Primary text color",
        "button" => "Button background color",
        "graph_line" => "Data line color on the graph",
        "graph_gap" => "Color used to mark gaps in recorded data",
        "graph_mean" => "Mean overlay line color",
        "graph_ref" => "Reference line color",
        "graph_crossing" => "Trigger-crossing marker color",
        "graph_cursor" => "Measurement cursor color",
        "graph_envelope" => "Min/Max envelope fill color",
        "plot_background" => "Graph plot area background color",
        "graph_crosshair" => "Hover crosshair color on the graph",
        "status_ok" => "\"Connected\" status color",
        "status_warning" => "Warning status color",
        "status_error" => "Error status color",
        "status_inactive" => "Inactive / disconnected status color",
        "accent" => "Accent color used by active toggles and highlights",
        "minimap_viewport" => "Minimap viewport rectangle color",
        _ => "Color override",
    }
}
