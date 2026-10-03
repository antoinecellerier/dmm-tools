use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;

use crate::app::App;

/// A context key's tooltip: like every remote button's, it promises a press
/// and nothing more — the key's own words when the meter has no key of that
/// name to press.
fn context_key_hover(key: &dmm_lib::protocol::MeterKey) -> String {
    key.hover.map_or_else(
        || format!("Press the meter's {} key", key.label),
        str::to_string,
    )
}

/// Width of the rule between the meter's buttons and the Scale chip, in
/// points before zoom: egui's own separator spacing.
pub(in crate::app) const SCALE_RULE_WIDTH: f32 = 6.0;

impl App {
    /// The meter's buttons, with the **Scale** chip on the end of their last
    /// line when it fits and on a line of its own otherwise. `right_reserve`
    /// is width the caller paints over at the row's right edge (the big-meter
    /// toggle), which the chip must not run under.
    pub(in crate::app) fn show_remote_controls(
        &mut self,
        ui: &mut Ui,
        scale: f32,
        right_reserve: f32,
    ) {
        use crate::app::ConnectionState;

        let font_size = 12.0 * scale;
        let spacing = 3.0 * scale;

        // Only show controls when connected with measurement data and supported commands
        if self.connection.state != ConnectionState::Connected
            || self.last_measurement.is_none()
            || self.connection.supported_commands().is_empty()
        {
            // No button row to join: Scale keeps its own, so an active
            // scale can still be turned off while disconnected.
            let clicked = ui
                .horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    self.show_scale_button(ui, font_size)
                })
                .inner;
            if clicked {
                self.toggle_transform_editor();
            }
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let has_cmd = |cmd: &str| {
            self.connection
                .supported_commands()
                .iter()
                .any(|c| c == cmd)
        };
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let active_color = tc.accent();

        // Collected rather than acted on inside the closure, which holds
        // `has_cmd`'s borrow of `self`.
        let mut scale_clicked = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            // Whether any meter button landed on the row: only then is there
            // something for the Scale chip to be set apart from.
            let mut placed = false;

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
                    "Press RANGE (manual range, one step)",
                ),
                ("AUTO", auto, "auto", "Return the meter to auto-range"),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                // `selected` announces the state and paints the selected fill
                // when on; the button keeps its frame when off so it still
                // reads as actionable, unlike `Button::selectable`.
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                let resp = ui
                    .add(egui::Button::new(text).selected(active))
                    .on_hover_text(tooltip);
                placed = true;
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
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                let resp = ui
                    .add(egui::Button::new(text).selected(active))
                    .on_hover_text(tooltip);
                placed = true;
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

            // The meter's context keys (ZOTEK's ZERO in capacitance, the
            // 121GW's 1kHz in AC, OWON's Hz/Duty, and the CM2100B's ZERO),
            // each only while the reading is one it applies to.
            let reading = self.last_measurement.as_ref();
            for key in self.connection.meter_keys().context {
                if !reading.is_some_and(|m| (key.applies)(m)) {
                    continue;
                }
                let text = RichText::new(key.label).font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(context_key_hover(key))
                    .clicked()
                {
                    self.send_command(key.command);
                }
                placed = true;
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
                placed = true;
            }

            // Scale, set apart by a rule: it changes nothing on the meter,
            // and sitting it among the buttons with no boundary would
            // suggest the meter knows about the factor. Measured as one
            // unit before placing, so the rule can never be left dangling
            // at the end of a line the chip wrapped off. On a line of its
            // own the line break is the boundary and the rule is dropped.
            let rule_width = SCALE_RULE_WIDTH * scale;
            let chip_width = Self::scale_button_width(ui, font_size);
            let needed = rule_width + spacing + chip_width + spacing + right_reserve;
            if placed && needed <= ui.available_size_before_wrap().x {
                let height = ui.cursor().height();
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(rule_width, height), egui::Sense::hover());
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
            } else if placed {
                ui.end_row();
            }
            scale_clicked = self.show_scale_button(ui, font_size);
        });
        if scale_clicked {
            self.toggle_transform_editor();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    /// A context key offered in farads only.
    const ZERO_IN_FARADS: &[dmm_lib::protocol::MeterKey] = &[dmm_lib::protocol::MeterKey {
        command: "zero",
        label: "ZERO",
        hover: None,
        applies: |m| m.unit.ends_with('F'),
    }];

    /// The labels of the buttons the connected reading column draws for a
    /// reading in `unit`, on a meter with HOLD and [`ZERO_IN_FARADS`].
    fn chips_for(unit: &'static str) -> Vec<String> {
        let settings = Settings {
            // No acquisition thread: the connected state is set by hand.
            auto_connect: false,
            ..Settings::default()
        };
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        app.connection.state = crate::app::ConnectionState::Connected;
        app.connection.meter = Some(crate::app::ConnectedMeter {
            supported_commands: vec!["hold".to_string(), "zero".to_string()],
            meter_keys: dmm_lib::protocol::MeterKeys {
                functions: &[],
                context: ZERO_IN_FARADS,
            },
            ..crate::app::ConnectedMeter::test_fixture(None)
        });
        app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            unit,
            dmm_lib::flags::StatusFlags::default(),
        ));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, crate::app::layout::ContentLayout::Wide);
            });
        });
        // This harness renders without a painter.
        out.textures_delta.clear();
        out.platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, n)| n.role() == egui::accesskit::Role::Button)
            .filter_map(|(_, n)| n.label().map(str::to_string))
            .collect()
    }

    /// A context key joins the meter's buttons only while it applies to the
    /// reading: ZERO in capacitance, and nowhere else.
    #[test]
    fn a_context_key_shows_only_while_it_applies() {
        let farads = chips_for("nF");
        assert!(farads.iter().any(|l| l == "HOLD"), "{farads:?}");
        assert!(farads.iter().any(|l| l == "ZERO"), "{farads:?}");
        let volts = chips_for("V");
        assert!(volts.iter().any(|l| l == "HOLD"), "{volts:?}");
        assert!(!volts.iter().any(|l| l == "ZERO"), "{volts:?}");
    }

    /// A context key's tooltip names the key to press, or says how the
    /// meter does it when no key carries its label.
    #[test]
    fn a_context_key_tooltip_says_how_the_meter_does_it() {
        assert_eq!(
            super::context_key_hover(&ZERO_IN_FARADS[0]),
            "Press the meter's ZERO key"
        );
        let long_press = dmm_lib::protocol::MeterKey {
            hover: Some("Hold the meter's REL key"),
            ..ZERO_IN_FARADS[0]
        };
        assert_eq!(
            super::context_key_hover(&long_press),
            "Hold the meter's REL key"
        );
    }
}
