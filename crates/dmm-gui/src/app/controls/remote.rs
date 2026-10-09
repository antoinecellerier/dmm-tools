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

/// The caret a row chip ends with: folded, and open.
const FOLDED: &str = "\u{25B8}";
const OPEN: &str = "\u{25BE}";

/// One of the app's own chips beside the meter's buttons (**Scale**,
/// **Alarm**): a body that switches the feature on or off, joined to a caret
/// that shows or hides its row of fields.
///
/// The body does the same in every layout; the big meter, which has no
/// room for the row, leaves the caret out.
pub(in crate::app) struct SplitChip<'a> {
    pub(in crate::app) name: &'a str,
    /// In force: the chip's fill.
    pub(in crate::app) active: bool,
    /// There are values to switch on, or it is on: else the body opens the
    /// row instead, or with no row to open it is greyed out.
    pub(in crate::app) ready: bool,
    /// Whether the row is open; `None` where there is no row (the big meter).
    pub(in crate::app) open: Option<bool>,
    /// The body's hover text.
    pub(in crate::app) hover: &'a str,
    /// The caret's hover text: what the fields take.
    pub(in crate::app) fields_hover: &'a str,
    /// The greyed body's hover: where the row can be opened.
    pub(in crate::app) unready_hover: &'a str,
}

/// What a click on a [`SplitChip`] asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ChipClick {
    None,
    /// The body: switch on or off.
    Body,
    /// The caret: show or hide the row.
    Caret,
}

impl SplitChip<'_> {
    fn body_label(&self, font_size: f32) -> RichText {
        RichText::new(self.name).font(egui::FontId::proportional(font_size))
    }

    fn caret_label(open: bool, font_size: f32) -> RichText {
        let caret = if open { OPEN } else { FOLDED };
        RichText::new(caret).font(egui::FontId::proportional(font_size))
    }

    /// Width the chip takes, so the button row can tell before placing the
    /// app's own chips whether they fit on the line.
    pub(in crate::app) fn width(&self, ui: &Ui, font_size: f32) -> f32 {
        let width = |text: RichText| {
            egui::WidgetText::from(text)
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Button,
                )
                .size()
                .x
                + 2.0 * ui.spacing().button_padding.x
        };
        width(self.body_label(font_size))
            + self.open.map_or(0.0, |open| {
                SPLIT_GAP + width(Self::caret_label(open, font_size))
            })
    }

    /// Place the chip where the caller's row has its cursor.
    ///
    /// Spoken as two controls: the body by its name, `active` as selected,
    /// and the caret as "<name> settings", expanded or collapsed.
    pub(in crate::app) fn show(&self, ui: &mut Ui, font_size: f32) -> ChipClick {
        let r = ui.visuals().widgets.inactive.corner_radius;
        let (body_radius, caret_radius) = if self.open.is_some() {
            (
                egui::CornerRadius { ne: 0, se: 0, ..r },
                egui::CornerRadius { nw: 0, sw: 0, ..r },
            )
        } else {
            (r, r)
        };
        // egui spaces each widget from the one after it by the spacing in
        // force as it is placed: the body takes the hairline only when the
        // caret follows it, and the caret, last, the row's own gap.
        let gap = ui.spacing().item_spacing.x;
        if self.open.is_some() {
            ui.spacing_mut().item_spacing.x = SPLIT_GAP;
        }
        // `selected` puts the state in the widget info for AT users, whom
        // the fill alone doesn't reach; the frame stays when off so the
        // chip still reads as actionable.
        let body = ui
            .add_enabled(
                self.ready || self.open.is_some(),
                egui::Button::new(self.body_label(font_size))
                    .selected(self.active)
                    .corner_radius(body_radius),
            )
            .on_hover_text(self.hover)
            .on_disabled_hover_text(self.unready_hover);
        let caret = self.open.map(|open| {
            ui.spacing_mut().item_spacing.x = gap;
            let caret = ui
                .add(
                    egui::Button::new(Self::caret_label(open, font_size))
                        .selected(self.active)
                        .corner_radius(caret_radius),
                )
                .on_hover_text(self.fields_hover)
                .a11y_label(&format!("{} settings", self.name));
            ui.ctx()
                .accesskit_node_builder(caret.id, |builder| builder.set_expanded(open));
            caret
        });
        if body.clicked() {
            ChipClick::Body
        } else if caret.is_some_and(|c| c.clicked()) {
            ChipClick::Caret
        } else {
            ChipClick::None
        }
    }
}

/// The hairline between a split chip's body and its caret.
const SPLIT_GAP: f32 = 1.0;

impl App {
    /// The meter's buttons, with the **Scale** and **Alarm** chips on the end
    /// of their last line when they fit and on a line of their own otherwise.
    /// `right_reserve` is width the caller paints over at the row's right
    /// edge (the big-meter toggle), which the chips must not run under.
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
            // No button row to join: the app's chips keep their own, so an
            // active scale or alarm can still be turned off while
            // disconnected.
            let (scale_clicked, alarm_clicked) = ui
                .horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    (
                        self.scale_chip().show(ui, font_size),
                        self.alarm_chip().show(ui, font_size),
                    )
                })
                .inner;
            self.take_app_chip_clicks(scale_clicked, alarm_clicked);
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
        let mut scale_clicked = ChipClick::None;
        let mut alarm_clicked = ChipClick::None;
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

            // Scale and Alarm, set apart by a rule: they change nothing on
            // the meter, and sitting them among the buttons with no
            // boundary would suggest the meter knows about the factor or
            // the limits. Measured as one unit before placing, so the rule
            // can never be left dangling at the end of a line the chips
            // wrapped off. On a line of their own the line break is the
            // boundary and the rule is dropped.
            let rule_width = SCALE_RULE_WIDTH * scale;
            let chips_width = self.scale_chip().width(ui, font_size)
                + spacing
                + self.alarm_chip().width(ui, font_size);
            let needed = rule_width + spacing + chips_width + spacing + right_reserve;
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
            scale_clicked = self.scale_chip().show(ui, font_size);
            alarm_clicked = self.alarm_chip().show(ui, font_size);
        });
        self.take_app_chip_clicks(scale_clicked, alarm_clicked);
    }

    /// Act on a click on the Scale or Alarm chip — switch it, or show or
    /// hide its row — once the button row's closure has let go of `self`.
    fn take_app_chip_clicks(&mut self, scale: ChipClick, alarm: ChipClick) {
        match scale {
            ChipClick::Body => self.toggle_scale(),
            ChipClick::Caret => self.toggle_transform_editor(),
            ChipClick::None => {}
        }
        match alarm {
            ChipClick::Body => self.toggle_alarm(),
            ChipClick::Caret => self.toggle_alarm_editor(),
            ChipClick::None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    /// The carets draw from the bundled fonts, not as the replacement box
    /// (`Fonts::has_glyph` can't tell; see `.claude/rules/gui.md`).
    #[test]
    fn the_carets_have_glyphs() {
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::app::appearance::font_definitions());
        let font = egui::FontId::proportional(14.0);
        let atlas_rect = |ui: &egui::Ui, text: &str| {
            let galley = ui.painter().layout_no_wrap(
                text.to_string(),
                font.clone(),
                egui::Color32::PLACEHOLDER,
            );
            galley.rows[0].row.glyphs[0].uv_rect
        };
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let replacement = atlas_rect(ui, "\u{25FB}");
            for caret in [FOLDED, OPEN] {
                assert_ne!(
                    atlas_rect(ui, caret),
                    replacement,
                    "{caret:?} draws as a box"
                );
            }
        });
        out.textures_delta.clear();
    }

    /// The body with nothing typed opens the row to type in; with limits
    /// typed it switches the alarm on, and again off, the fields kept.
    #[test]
    fn the_alarm_body_switches_with_its_fields() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.toggle_alarm();
        assert!(app.alarm_editor.open && app.alarm.is_none());
        app.alarm_editor.open = false;
        app.alarm_editor.set_drafts("", "5");
        app.toggle_alarm();
        assert!(app.alarm.is_some() && !app.alarm_editor.open);
        app.toggle_alarm();
        assert!(app.alarm.is_none());
        assert!(
            app.alarm_chip().ready,
            "the limits are kept for the next click"
        );
    }

    #[test]
    fn the_scale_body_switches_with_its_fields() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.toggle_scale();
        assert!(app.transform_editor.open && app.transform.is_identity());
        app.transform_editor.scale = "100".to_string();
        app.toggle_scale();
        assert!(!app.transform.is_identity());
        app.toggle_scale();
        assert!(app.transform.is_identity());
        assert_eq!(app.transform_editor.scale, "100");
    }

    /// The big meter has no row: no caret, and nothing to switch on greys
    /// the body out rather than opening a row it can't show.
    #[test]
    fn the_big_meter_chips_have_no_caret() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.big_meter_mode = crate::app::BigMeterMode::Full;
        let chip = app.alarm_chip();
        assert_eq!((chip.open, chip.ready), (None, false));
        app.alarm_editor.set_drafts("3", "");
        assert!(app.alarm_chip().ready);
    }

    /// The greyed chip names the way to a row: Ctrl+B out of the big meter,
    /// but with the graph and recording hidden in Settings, Ctrl+B only
    /// cycles big-meter views that have none.
    #[test]
    fn the_greyed_chip_names_the_way_to_its_row() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.big_meter_mode = crate::app::BigMeterMode::Full;
        assert!(app.scale_chip().unready_hover.contains("Ctrl+B"));
        app.big_meter_mode = crate::app::BigMeterMode::Off;
        app.settings.show_graph = false;
        app.settings.show_recording = false;
        let chip = app.scale_chip();
        assert_eq!(chip.open, None);
        assert!(
            chip.unready_hover.contains("Settings"),
            "{}",
            chip.unready_hover
        );
    }

    /// One row under the chips: opening either closes the other.
    #[test]
    fn opening_a_row_closes_the_other() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.toggle_transform_editor();
        app.toggle_alarm_editor();
        assert!(app.alarm_editor.open && !app.transform_editor.open);
        app.toggle_transform_editor();
        assert!(app.transform_editor.open && !app.alarm_editor.open);
        app.toggle_transform_editor();
        assert!(!app.transform_editor.open && !app.alarm_editor.open);
    }

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
