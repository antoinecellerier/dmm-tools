use std::borrow::Cow;

use dmm_lib::protocol::command;
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;

use crate::app::App;

/// A context key's tooltip: a press of the key, or the key's own words when
/// that wouldn't be true.
fn context_key_hover(l10n: &Localizer, key: &dmm_lib::protocol::MeterKey) -> String {
    match key.hover {
        Some(hover) => l10n.label(TextKind::KeyHover, hover).into_owned(),
        None => tr!(
            l10n,
            "remote-press-key",
            key = l10n.label(TextKind::Key, key.label)
        )
        .into_owned(),
    }
}

/// A shared button's tooltip: the model's own where the generic one isn't true
/// of it.
fn shared_hover<'a>(
    l10n: &'a Localizer,
    keys: &dmm_lib::protocol::MeterKeys,
    command: &str,
    generic: Cow<'a, str>,
) -> Cow<'a, str> {
    match keys.button_hover(command) {
        Some(hover) => l10n.label(TextKind::KeyHover, hover),
        None => generic,
    }
}

/// MAX/MIN's generic tooltip: it promises the × only on a model that has one.
fn min_max_hover(l10n: &Localizer, has_exit: bool) -> Cow<'_, str> {
    if has_exit {
        tr!(l10n, "remote-min-max-exit.hover")
    } else {
        tr!(l10n, "remote-min-max-no-exit.hover")
    }
}

/// PEAK's generic tooltip, as [`min_max_hover`]'s.
fn peak_hover(l10n: &Localizer, has_exit: bool) -> Cow<'_, str> {
    if has_exit {
        tr!(l10n, "remote-peak.hover")
    } else {
        tr!(l10n, "remote-peak-no-exit.hover")
    }
}

/// Width of the rule between the meter's buttons and the Scale chip, in
/// points before zoom: egui's own separator spacing.
pub(in crate::app) const SCALE_RULE_WIDTH: f32 = 6.0;

/// The caret a row chip ends with: folded, and open.
const FOLDED: &str = "\u{25B8}";
const OPEN: &str = "\u{25BE}";

/// Width of a button showing `text`, so a row can tell before placing it
/// whether it fits on the line.
pub(in crate::app) fn button_width(ui: &Ui, text: RichText) -> f32 {
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
}

/// The corners of a button and of the one `joined` to it, drawn as one shape:
/// square where they meet across [`SPLIT_GAP`].
fn split_corners(ui: &Ui, joined: bool) -> (egui::CornerRadius, egui::CornerRadius) {
    let r = ui.visuals().widgets.inactive.corner_radius;
    if joined {
        (
            egui::CornerRadius { ne: 0, se: 0, ..r },
            egui::CornerRadius { nw: 0, sw: 0, ..r },
        )
    } else {
        (r, r)
    }
}

/// One of the app's own chips beside the meter's buttons (**Scale**,
/// **Alarm**): a body that switches the feature on or off, joined to a caret
/// that shows or hides its row of fields.
///
/// The body does the same in every layout; the big meter, which has no
/// room for the row, leaves the caret out.
pub(in crate::app) struct SplitChip<'a> {
    pub(in crate::app) name: Cow<'a, str>,
    /// In force: the chip's fill.
    pub(in crate::app) active: bool,
    /// There are values to switch on, or it is on: else the body opens the
    /// row instead, or with no row to open it is greyed out.
    pub(in crate::app) ready: bool,
    /// Whether the row is open; `None` where there is no row (the big meter).
    pub(in crate::app) open: Option<bool>,
    /// The body's hover text.
    pub(in crate::app) hover: Cow<'a, str>,
    /// The caret's name for a screen reader: "Scale settings".
    pub(in crate::app) fields_name: Cow<'a, str>,
    /// The caret's hover text: what the fields take.
    pub(in crate::app) fields_hover: Cow<'a, str>,
    /// The greyed body's hover: where the row can be opened.
    pub(in crate::app) unready_hover: String,
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
        RichText::new(self.name.as_ref()).font(egui::FontId::proportional(font_size))
    }

    fn caret_label(open: bool, font_size: f32) -> RichText {
        let caret = if open { OPEN } else { FOLDED };
        RichText::new(caret).font(egui::FontId::proportional(font_size))
    }

    /// Width the chip takes, so the button row can tell before placing the
    /// app's own chips whether they fit on the line.
    pub(in crate::app) fn width(&self, ui: &Ui, font_size: f32) -> f32 {
        button_width(ui, self.body_label(font_size))
            + self.open.map_or(0.0, |open| {
                SPLIT_GAP + button_width(ui, Self::caret_label(open, font_size))
            })
    }

    /// Place the chip where the caller's row has its cursor.
    ///
    /// Spoken as two controls: the body by its name, `active` as selected,
    /// and the caret by its `fields_name`, expanded or collapsed.
    pub(in crate::app) fn show(&self, ui: &mut Ui, font_size: f32) -> ChipClick {
        let (body_radius, caret_radius) = split_corners(ui, self.open.is_some());
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
            .on_hover_text(self.hover.as_ref())
            .on_disabled_hover_text(&self.unready_hover);
        let caret = self.open.map(|open| {
            ui.spacing_mut().item_spacing.x = gap;
            let caret = ui
                .add(
                    egui::Button::new(Self::caret_label(open, font_size))
                        .selected(self.active)
                        .corner_radius(caret_radius),
                )
                .on_hover_text(self.fields_hover.as_ref())
                .a11y_label(&self.fields_name);
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

/// The hairline between a split chip's body and its caret, and between a
/// MAX/MIN or PEAK button and its "x".
const SPLIT_GAP: f32 = 1.0;

/// Width of each field in a split chip's row, in points before zoom.
const FIELD_WIDTH: f32 = 50.0;

/// One captioned field in a split chip's row (**Scale**'s factor, **Alarm**'s
/// limits), in a `horizontal_wrapped` row at `scale`: the field's response,
/// and whether Enter committed it this frame.
///
/// Enter, never `.changed()`: a value taken per keystroke would rescale the
/// readings, or raise an alarm at 1, on the way to typing 12.
pub(in crate::app) fn row_field(
    ui: &mut Ui,
    font: &egui::FontId,
    scale: f32,
    caption: &str,
    text: &mut String,
    hint: &str,
    focus: bool,
) -> (egui::Response, bool) {
    let width = FIELD_WIDTH * scale;
    // A caption and its field wrap together: a caption left at the end of a
    // line reads as labelling nothing.
    let caption = RichText::new(caption).font(font.clone());
    let caption_width = egui::WidgetText::from(caption.clone())
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Body,
        )
        .size()
        .x;
    if caption_width + ui.spacing().item_spacing.x + width > ui.available_size_before_wrap().x {
        ui.end_row();
    }
    ui.label(caption);
    let resp = ui.add(
        egui::TextEdit::singleline(text)
            .desired_width(width)
            .font(font.clone())
            .hint_text(RichText::new(hint).font(font.clone())),
    );
    if focus {
        resp.request_focus();
    }
    // egui's own focused frame is invisible under a pinned Accent; the ring
    // is the field's only keyboard cue then.
    crate::a11y::paint_focus_ring(ui, &resp);
    let committed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    (resp, committed)
}

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

        // Only show controls when connected with measurement data and supported
        // commands, in a session that draws them.
        if self.connection.state != ConnectionState::Connected
            || self.last_measurement.is_none()
            || self.connection.supported_commands().is_empty()
            || !self.draws_meter_controls()
        {
            // No button row to join: the app's chips keep their own, so an
            // active scale or alarm can still be turned off while
            // disconnected.
            let (scale_clicked, alarm_clicked) = ui
                .horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    (
                        self.scale_chip(ui.ctx()).show(ui, font_size),
                        self.alarm_chip(ui.ctx()).show(ui, font_size),
                    )
                })
                .inner;
            self.take_app_chip_clicks(scale_clicked, alarm_clicked);
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let keys = self.connection.meter_keys();
        let has_cmd = |cmd: &str| {
            self.connection
                .supported_commands()
                .iter()
                .any(|c| c == cmd)
        };
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        // What a shared button shows: the model's own word, or the generic one.
        let shown = |cmd: &str, generic: &'static str| keys.button_label(cmd).unwrap_or(generic);

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
            let min_max = flags.is_some_and(|f| f.min || f.max || f.avg);
            let peak = flags.is_some_and(|f| f.peak_min || f.peak_max);

            // The buttons that light with their state: label, active flag,
            // command, the command of the "x" that leaves the state, tooltip.
            // A tooltip here describes the generic DMM behavior; a model it
            // isn't true of says so in its `ButtonLabel`. The MAX/MIN word
            // puts MAX first, as on the meters with no label of their own.
            //
            // A click sends the key's short press, which on MAX/MIN and Peak
            // cycles on most meters and never exits there, and on the Voltcraft
            // meters' REL sets a new zero. The "x" joined to the button exits
            // the mode (like the real device's long-press).
            for (label, active, cmd, exit_cmd, tooltip) in [
                (
                    "HOLD",
                    hold,
                    command::HOLD,
                    None,
                    tr!(l10n, "remote-hold.hover"),
                ),
                (
                    "REL",
                    rel,
                    command::REL,
                    Some(command::EXIT_REL),
                    tr!(l10n, "remote-rel.hover"),
                ),
                (
                    "RANGE",
                    manual_range,
                    command::RANGE,
                    None,
                    tr!(l10n, "remote-range.hover"),
                ),
                (
                    "AUTO",
                    auto,
                    command::AUTO,
                    None,
                    tr!(l10n, "remote-auto.hover"),
                ),
                (
                    "MAX/MIN",
                    min_max,
                    command::MINMAX,
                    Some(command::EXIT_MINMAX),
                    min_max_hover(l10n, has_cmd(command::EXIT_MINMAX)),
                ),
                (
                    "PEAK",
                    peak,
                    command::PEAK,
                    Some(command::EXIT_PEAK),
                    peak_hover(l10n, has_cmd(command::EXIT_PEAK)),
                ),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                let label = shown(cmd, label);
                let font = egui::FontId::proportional(font_size);
                let body = RichText::new(label).font(font.clone());
                let exit = RichText::new("x").font(font);
                let joined_to = exit_cmd.filter(|&exit_cmd| active && has_cmd(exit_cmd));
                // `selected` announces the state and paints the selected fill
                // when on; the button keeps its frame when off so it still
                // reads as actionable, unlike `Button::selectable`.
                let mut button = egui::Button::new(body.clone()).selected(active);
                let gap = ui.spacing().item_spacing.x;
                let (body_radius, exit_radius) = split_corners(ui, true);
                if joined_to.is_some() {
                    // The two wrap as one: a joined "x" alone on a line would read
                    // as belonging to nothing. Wider than a whole line, a new line
                    // wouldn't help, and would leave an empty one above them.
                    let width = button_width(ui, body) + SPLIT_GAP + button_width(ui, exit.clone());
                    if width > ui.available_size_before_wrap().x && width <= ui.max_rect().width() {
                        ui.end_row();
                    }
                    button = button.corner_radius(body_radius);
                    // As in `SplitChip::show`: the hairline only when the "x"
                    // follows, and the "x", last, takes the row's own gap.
                    ui.spacing_mut().item_spacing.x = SPLIT_GAP;
                }
                let resp = ui
                    .add(button)
                    .on_hover_text(shared_hover(l10n, &keys, cmd, tooltip));
                placed = true;
                if resp.clicked() {
                    self.send_command(cmd);
                }
                if let Some(exit_cmd) = joined_to {
                    ui.spacing_mut().item_spacing.x = gap;
                    let x_resp = ui
                        .add(
                            egui::Button::new(exit)
                                .selected(true)
                                .corner_radius(exit_radius),
                        )
                        .on_hover_text(tr!(l10n, "remote-exit.hover", mode = label))
                        .a11y_label(&tr!(l10n, "remote-exit.a11y", mode = label));
                    // `selected` is for the shared fill: this is an action, not
                    // a toggle that a screen reader should call pressed.
                    ui.ctx()
                        .accesskit_node_builder(x_resp.id, |builder| builder.clear_toggled());
                    if x_resp.clicked() {
                        self.send_command(exit_cmd);
                    }
                }
            }

            // Non-toggle commands
            for (label, cmd, tooltip) in [
                ("SELECT", command::SELECT, tr!(l10n, "remote-select.hover")),
                ("LIGHT", command::LIGHT, tr!(l10n, "remote-light.hover")),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                let text =
                    RichText::new(shown(cmd, label)).font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(shared_hover(l10n, &keys, cmd, tooltip))
                    .clicked()
                {
                    self.send_command(cmd);
                }
                placed = true;
            }

            // The meter's context keys (ZOTEK's ZERO in capacitance, the
            // 121GW's 1kHz in AC, OWON's Hz/Duty, Inrush and the rest, the
            // CM2100B's ZERO, the UT61+'s Hz/%, the UT181A's SAVE), each only
            // while the reading is one it applies to. Last, as they come and
            // go with the mode: a SELECT press that brings one in leaves SELECT
            // where it was, for the next press.
            let reading = self.last_measurement.as_ref();
            for key in keys.context {
                if !reading.is_some_and(|m| (key.applies)(m)) {
                    continue;
                }
                let text = RichText::new(l10n.label(TextKind::Key, key.label))
                    .font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(context_key_hover(l10n, key))
                    .clicked()
                {
                    self.send_command(key.command);
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
            let scale_chip = self.scale_chip(ui.ctx());
            let alarm_chip = self.alarm_chip(ui.ctx());
            let chips_width =
                scale_chip.width(ui, font_size) + spacing + alarm_chip.width(ui, font_size);
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
            scale_clicked = scale_chip.show(ui, font_size);
            alarm_clicked = alarm_chip.show(ui, font_size);
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
    use crate::app::test_support::{
        a11y, click_frames, connected_app, connected_to, headless_frame, node_rect, test_app,
    };
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::{MeasuredValue, Measurement};
    use dmm_lib::protocol::{DeviceFamily, DeviceProfile, registry};
    use std::collections::BTreeMap;

    /// The carets draw from the bundled fonts, not as the replacement box
    /// (`Fonts::has_glyph` can't tell; see `.claude/rules/gui.md`).
    #[test]
    fn the_carets_have_glyphs() {
        let ctx = crate::app::styled_context();
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
        headless_frame(&ctx, None, None, vec![], |ui| {
            let replacement = atlas_rect(ui, "\u{25FB}");
            for caret in [FOLDED, OPEN] {
                assert_ne!(
                    atlas_rect(ui, caret),
                    replacement,
                    "{caret:?} draws as a box"
                );
            }
        });
    }

    /// The body with nothing typed opens the row to type in; with limits
    /// typed it switches the alarm on, and again off, the fields kept.
    #[test]
    fn the_alarm_body_switches_with_its_fields() {
        let mut app = test_app();
        app.toggle_alarm();
        assert!(app.alarm_editor.open && app.alarm.is_none());
        app.alarm_editor.open = false;
        app.alarm_editor.set_drafts("", "5");
        app.toggle_alarm();
        assert!(app.alarm.is_some() && !app.alarm_editor.open);
        app.toggle_alarm();
        assert!(app.alarm.is_none());
        assert!(
            app.alarm_chip(&crate::app::styled_context()).ready,
            "the limits are kept for the next click"
        );
    }

    #[test]
    fn the_scale_body_switches_with_its_fields() {
        let mut app = test_app();
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
        let mut app = test_app();
        app.big_meter_mode = crate::app::BigMeterMode::Full;
        let ctx = crate::app::styled_context();
        let chip = app.alarm_chip(&ctx);
        assert_eq!((chip.open, chip.ready), (None, false));
        app.alarm_editor.set_drafts("3", "");
        assert!(app.alarm_chip(&ctx).ready);
    }

    /// The greyed chip names the way to a row: Ctrl+B out of the big meter,
    /// but with the graph and recording hidden in Settings, Ctrl+B only
    /// cycles big-meter views that have none.
    #[test]
    fn the_greyed_chip_names_the_way_to_its_row() {
        let mut app = test_app();
        app.big_meter_mode = crate::app::BigMeterMode::Full;
        let ctx = crate::app::styled_context();
        let chip = app.scale_chip(&ctx);
        assert!(
            chip.unready_hover.contains(
                &crate::app::shortcuts::Shortcut::CycleBigMeter.keys(&ctx, &crate::l10n::english())
            ),
            "{}",
            chip.unready_hover
        );
        app.big_meter_mode = crate::app::BigMeterMode::Off;
        app.settings.show_graph = false;
        app.settings.show_recording = false;
        let chip = app.scale_chip(&ctx);
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
        let mut app = test_app();
        app.toggle_transform_editor();
        app.toggle_alarm_editor();
        assert!(app.alarm_editor.open && !app.transform_editor.open);
        app.toggle_transform_editor();
        assert!(app.transform_editor.open && !app.alarm_editor.open);
        app.toggle_transform_editor();
        assert!(!app.transform_editor.open && !app.alarm_editor.open);
    }

    /// Clicking a chip's caret opens its row with the cursor in the row's
    /// first field, in the scrolled reading column the window draws: the
    /// column's scroll-to-focus must not drop a focus the click gave.
    #[test]
    fn a_caret_opens_its_row_with_the_cursor_in_the_first_field() {
        for (caret, first_field) in [
            ("Scale settings", "Scale factor"),
            ("Alarm settings", "none"),
        ] {
            let mut app = connected_app(&["hold"], "V");
            let ctx = crate::app::styled_context();
            ctx.enable_accesskit();
            let mut frame = |events| {
                let out = headless_frame(&ctx, None, None, events, |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        app.show_reading_column_scrolled(
                            ui,
                            crate::app::layout::ContentLayout::Wide,
                        );
                    });
                });
                a11y(&out)
            };
            let (nodes, _) = frame(vec![]);
            let at = node_rect(&nodes, |n| n.label() == Some(caret))
                .unwrap_or_else(|| panic!("{caret:?} is drawn"))
                .center();
            for events in click_frames(at, egui::PointerButton::Primary) {
                frame(events);
            }
            let (nodes, focus) = frame(vec![]);
            let focused = nodes
                .iter()
                .find(|(id, _)| Some(*id) == focus)
                .map(|(_, n)| (n.role(), n.placeholder().map(str::to_string)));
            assert_eq!(
                focused,
                Some((
                    egui::accesskit::Role::TextInput,
                    Some(first_field.to_string())
                )),
                "after a click on {caret:?}"
            );
        }
    }

    /// A context key offered in farads only.
    const ZERO_IN_FARADS: &[dmm_lib::protocol::MeterKey] = &[dmm_lib::protocol::MeterKey {
        command: "zero",
        label: "ZERO",
        in_words: false,
        hover: None,
        applies: |m| m.unit.ends_with('F'),
    }];

    /// The labels of the buttons the connected reading column draws for a
    /// reading in `unit`, on a meter with HOLD and [`ZERO_IN_FARADS`].
    fn chips_for(unit: &'static str) -> Vec<String> {
        let mut app = connected_app(&["hold", "zero"], unit);
        let meter = app.connection.meter.as_mut().expect("a connected meter");
        meter.meter_keys = dmm_lib::protocol::MeterKeys {
            context: ZERO_IN_FARADS,
            ..dmm_lib::protocol::MeterKeys::NONE
        };
        buttons_of(&mut app)
    }

    /// The labels of the buttons `app`'s reading column draws.
    fn buttons_of(app: &mut App) -> Vec<String> {
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let out = headless_frame(&ctx, None, None, vec![], |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, crate::app::layout::ContentLayout::Wide);
            });
        });
        a11y(&out)
            .0
            .into_iter()
            .filter(|(_, n)| n.role() == egui::accesskit::Role::Button)
            .filter_map(|(_, n)| n.label().map(str::to_string))
            .collect()
    }

    /// What can be clicked in `app`'s single-line reading column, buttons and
    /// dropdowns, and its plain text.
    fn narrow_column(app: &mut App) -> (Vec<String>, Vec<String>) {
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let out = headless_frame(&ctx, None, None, vec![], |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, crate::app::layout::ContentLayout::Narrow);
            });
        });
        let nodes = a11y(&out).0;
        use egui::accesskit::Role;
        let of = |roles: &[Role]| -> Vec<String> {
            nodes
                .iter()
                .filter(|(_, n)| roles.contains(&n.role()))
                .filter_map(|(_, n)| n.label().or(n.value()).map(str::to_string))
                .collect()
        };
        (of(&[Role::Button, Role::ComboBox]), of(&[Role::Label]))
    }

    /// A replay refuses every press, so it draws none of the meter's buttons or
    /// dropdowns: the mode and range stay as text where they were, and the
    /// app's chips stay. The contributor flag brings them back; a live meter
    /// never loses them.
    #[test]
    fn a_replay_draws_no_meter_controls_unless_asked() {
        use dmm_lib::protocol::{Choice, Setting};
        let choices = |n: &[(&'static str, bool)]| -> Vec<Choice> {
            n.iter()
                .enumerate()
                .map(|(id, &(label, current))| Choice {
                    id: id as u16,
                    label: label.into(),
                    current,
                })
                .collect()
        };
        let app_with_lists = || {
            let mut app = connected_to("ut61eplus");
            app.connection
                .choices
                .set(Setting::Mode, choices(&[("DC V", true), ("AC V", false)]));
            app.connection
                .choices
                .set(Setting::Range, choices(&[("Auto", true), ("22V", false)]));
            let reading = app.last_measurement.as_mut().expect("a reading");
            reading.mode = "DC V".into();
            reading.range_label = "22V".into();
            app
        };
        let has = |list: &[String], text: &str| list.iter().any(|l| l.contains(text));

        let (buttons, _) = narrow_column(&mut app_with_lists());
        assert!(has(&buttons, "HOLD"), "{buttons:?}");
        assert!(has(&buttons, "Range"), "{buttons:?}");

        let mut replay = app_with_lists();
        replay.source.replay = Some(crate::ReplaySource::fixture());
        let (buttons, labels) = narrow_column(&mut replay);
        for gone in ["HOLD", "Range", "Mode"] {
            assert!(!has(&buttons, gone), "{gone}: {buttons:?}");
        }
        for kept in ["Scale", "Alarm"] {
            assert!(has(&buttons, kept), "{kept}: {buttons:?}");
        }
        for text in ["22V", "DC V"] {
            assert!(has(&labels, text), "{text}: {labels:?}");
        }

        replay.source.replay = Some(crate::ReplaySource {
            draws_keys: true,
            ..crate::ReplaySource::fixture()
        });
        let (buttons, _) = narrow_column(&mut replay);
        assert!(has(&buttons, "HOLD"), "{buttons:?}");
        assert!(has(&buttons, "Range"), "{buttons:?}");
    }

    /// A model that names a shared button shows that instead of the generic
    /// word; the rest keep it.
    #[test]
    fn a_shared_button_shows_what_the_model_calls_it() {
        for (id, shown, absent) in [
            (
                "vc880",
                ["RANGE", "AUTO", "MAX/MIN/AVG", "SHIFT"].as_slice(),
                ["MAX/MIN", "SELECT"].as_slice(),
            ),
            (
                "vc890",
                &["RANGE", "AUTO", "MAX/MIN/AVG", "SHIFT"],
                &["MAX/MIN", "SELECT"],
            ),
            (
                "vc650bt",
                &["RANGE", "AUTO", "MAX/MIN/AVG", "SHIFT"],
                &["MAX/MIN", "SELECT"],
            ),
            (
                "121gw",
                &["RANGE", "MIN/MAX/AVG", "MODE"],
                &["MAX/MIN", "SELECT"],
            ),
            ("121gw", &["HOLD/AutoHOLD", "1ms PEAK"], &["HOLD", "PEAK"]),
            ("cms101", &["MAX/MIN", "SELECT", "HOLD"], &["MaxMin"]),
            ("ut61b+", &["MAX/MIN", "SELECT"], &["PEAK"]),
            ("ut161b", &["MAX/MIN", "SELECT"], &["PEAK"]),
            (
                "ut61eplus",
                &["MAX/MIN", "SELECT"],
                &["MIN/MAX", "MAX/MIN/AVG", "MODE"],
            ),
        ] {
            let buttons = buttons_of(&mut connected_to(id));
            for label in shown {
                assert!(buttons.iter().any(|b| b == label), "{id}: {buttons:?}");
            }
            for label in absent {
                assert!(!buttons.iter().any(|b| b == label), "{id}: {buttons:?}");
            }
        }
    }

    /// The × is one shape with its button, wherever the row wraps: on the
    /// button's line, a hairline from it.
    #[test]
    fn the_exit_button_stays_joined_to_its_button() {
        let rects = |width: f32| {
            let mut app = connected_to("121gw");
            app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
                dmm_lib::measurement::MeasuredValue::Normal(1.234),
                "V",
                dmm_lib::flags::StatusFlags {
                    avg: true,
                    ..Default::default()
                },
            ));
            let ctx = crate::app::styled_context();
            ctx.enable_accesskit();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
            let out = headless_frame(&ctx, Some(screen), None, vec![], |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    app.show_reading_column(ui, crate::app::layout::ContentLayout::Wide);
                });
            });
            let (nodes, _) = a11y(&out);
            let exit = nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Exit MIN/MAX/AVG mode"));
            if let Some((_, exit)) = exit {
                assert_eq!(exit.toggled(), None, "the x is an action");
            }
            let rect = |label: &str| node_rect(&nodes, |n| n.label() == Some(label));
            (
                rect("MIN/MAX/AVG").expect("the button"),
                rect("Exit MIN/MAX/AVG mode"),
            )
        };
        for width in (200..=500).step_by(10) {
            let width = width as f32;
            let (body, exit) = rects(width);
            let exit = exit.unwrap_or_else(|| panic!("the x at {width}"));
            assert_eq!(body.min.y, exit.min.y, "the x wrapped at {width}");
            assert_eq!(body.height(), exit.height(), "at {width}");
            assert!(
                (exit.min.x - body.max.x - super::SPLIT_GAP).abs() < 0.5,
                "at {width}"
            );
        }
    }

    /// The × names the mode by the label beside it.
    #[test]
    fn the_exit_button_names_the_models_label() {
        let mut app = connected_to("vc880");
        app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            "V",
            dmm_lib::flags::StatusFlags {
                avg: true,
                ..Default::default()
            },
        ));
        let buttons = buttons_of(&mut app);
        assert!(
            buttons.iter().any(|b| b == "Exit MAX/MIN/AVG mode"),
            "{buttons:?}"
        );
    }

    /// The Voltcraft meters leave REL only with a long press, so theirs has
    /// an x, on its line, while REL is on. The other OWON meters leave it
    /// with a second press.
    #[test]
    fn rel_has_an_x_only_where_a_long_press_leaves_it() {
        let rel_on = |id: &str| {
            let mut app = connected_to(id);
            app.last_measurement = Some(Measurement::test_fixture(
                MeasuredValue::Normal(1.234),
                "V",
                StatusFlags {
                    rel: true,
                    ..Default::default()
                },
            ));
            app
        };
        let exit = "Exit REL mode".to_string();
        for id in ["vc871", "vc891", "vc915", "vc925pv"] {
            assert!(!buttons_of(&mut connected_to(id)).contains(&exit), "{id}");
            assert!(buttons_of(&mut rel_on(id)).contains(&exit), "{id}");
        }
        for id in ["ow65b", "ow67b", "ow69b", "cms061", "b35t+", "ut61eplus"] {
            assert!(!buttons_of(&mut rel_on(id)).contains(&exit), "{id}");
        }
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let mut app = rel_on("vc915");
        let out = headless_frame(&ctx, None, None, vec![], |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, crate::app::layout::ContentLayout::Wide);
            });
        });
        let (nodes, _) = a11y(&out);
        let rect = |label: &str| node_rect(&nodes, |n| n.label() == Some(label)).expect(label);
        let (body, x) = (rect("REL"), rect("Exit REL mode"));
        assert_eq!(body.min.y, x.min.y);
        assert!((x.min.x - body.max.x - SPLIT_GAP).abs() < 0.5);
    }

    /// AVG is one of the states the MAX/MIN key steps through, so the
    /// button stays on in it and keeps its way out.
    #[test]
    fn min_max_stays_on_while_the_meter_shows_avg() {
        let mut app = connected_app(&["minmax", "exit_minmax"], "V");
        let exit = "Exit MAX/MIN mode".to_string();
        assert!(!buttons_of(&mut app).contains(&exit));
        app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            "V",
            dmm_lib::flags::StatusFlags {
                avg: true,
                ..Default::default()
            },
        ));
        assert!(buttons_of(&mut app).contains(&exit));
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

    /// The UT61+'s Hz/% shows where the key does something: AC V, not DC V.
    #[test]
    fn the_hz_percent_key_follows_the_mode() {
        let at = |id: &str, mode_raw: u16| {
            let mut app = connected_to(id);
            let reading = app.last_measurement.as_mut().expect("a reading");
            reading.mode_raw = mode_raw;
            buttons_of(&mut app)
        };
        let (ac_v, dc_v) = (0x00, 0x02);
        let label = "Hz/%".to_string();
        for id in [
            "ut61eplus",
            "ut61b+",
            "ut61d+",
            "ut161b",
            "ut161d",
            "ut161e",
            "mock",
        ] {
            assert!(at(id, ac_v).contains(&label), "{id} in AC V");
            assert!(!at(id, dc_v).contains(&label), "{id} in DC V");
        }
        for id in ["ut60bt", "ut202bt"] {
            assert!(!at(id, ac_v).contains(&label), "{id} takes no select2");
        }
    }

    /// SELECT stays where it was when the mode it picks brings a key in, so a
    /// second press presses SELECT again.
    #[test]
    fn a_context_key_coming_in_leaves_select_where_it_was() {
        let select_at = |mode_raw: u16| {
            let mut app = connected_to("ut61eplus");
            let reading = app.last_measurement.as_mut().expect("a reading");
            reading.mode_raw = mode_raw;
            let ctx = crate::app::styled_context();
            ctx.enable_accesskit();
            let out = headless_frame(&ctx, None, None, vec![], |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    app.show_reading_column(ui, crate::app::layout::ContentLayout::Wide);
                });
            });
            node_rect(&a11y(&out).0, |n| n.label() == Some("SELECT")).expect("SELECT")
        };
        let (dc_mv, ac_mv) = (0x03, 0x01);
        assert_eq!(select_at(dc_mv), select_at(ac_mv));
    }

    /// The UT181A's SAVE, whatever the reading, names what it does.
    #[test]
    fn the_ut181a_offers_save() {
        assert!(buttons_of(&mut connected_to("ut181a")).contains(&"SAVE".to_string()));
        let keys = connected_to("ut181a").connection.meter_keys();
        assert_eq!(
            super::context_key_hover(&crate::l10n::english(), &keys.context[0]),
            "Save the reading to the meter's memory"
        );
    }

    /// A tooltip promises the × only where the model has one, and the average
    /// only where the meter reports one: on every model, its own words or not.
    #[test]
    fn a_hover_promises_only_what_the_model_does() {
        let l10n = crate::l10n::english();
        type Generic = for<'a> fn(&'a Localizer, bool) -> Cow<'a, str>;
        fn rel_hover(l10n: &Localizer, _has_exit: bool) -> Cow<'_, str> {
            tr!(l10n, "remote-rel.hover")
        }
        let rows: [(&str, &str, Generic); 3] = [
            (command::REL, command::EXIT_REL, rel_hover),
            (command::MINMAX, command::EXIT_MINMAX, min_max_hover),
            (command::PEAK, command::EXIT_PEAK, peak_hover),
        ];
        for device in registry::DEVICES {
            let protocol = (device.new_protocol)();
            let profile = protocol.profile();
            let has = |command: &str| profile.supported_commands.contains(&command);
            for (cycle, exit, generic) in rows {
                if !has(cycle) {
                    continue;
                }
                let hover =
                    shared_hover(&l10n, &profile.meter_keys, cycle, generic(&l10n, has(exit)));
                assert_eq!(hover.contains('×'), has(exit), "{}: {hover}", device.id);
            }
        }
        let hover = |id: &str| {
            let keys = connected_to(id).connection.meter_keys();
            shared_hover(&l10n, &keys, command::MINMAX, min_max_hover(&l10n, true)).into_owned()
        };
        assert!(!hover("ut61eplus").contains("average"));
        assert!(hover("vc880").contains("average"));
        assert!(hover("121gw").contains("average"));
        assert!(hover("ut181a").contains("average"));
        assert!(!hover("ut181a").contains("cycle"));
        assert!(!hover("cms101").contains("cycle"));
        assert!(!hover("zt5566se").contains("cycle"));
        assert!(hover("zt300ab").contains("cycle"));
    }

    /// A context key's tooltip names the key to press, or says how the
    /// meter does it when no key carries its label.
    #[test]
    fn a_context_key_tooltip_says_how_the_meter_does_it() {
        assert_eq!(
            super::context_key_hover(&crate::l10n::english(), &ZERO_IN_FARADS[0]),
            "Press the meter's ZERO key"
        );
        let long_press = dmm_lib::protocol::MeterKey {
            hover: Some("Hold the meter's REL key"),
            ..ZERO_IN_FARADS[0]
        };
        assert_eq!(
            super::context_key_hover(&crate::l10n::english(), &long_press),
            "Hold the meter's REL key"
        );
    }

    /// What a family advertises that is no keypress on the meter, so no button
    /// presses it, and why.
    const NOT_A_BUTTON: &[(DeviceFamily, &str, &str)] = &[
        (
            DeviceFamily::Ut181a,
            "monitor",
            "SET_MONITOR switches the stream on, and goes out at open",
        ),
        (
            DeviceFamily::Ut171,
            "connect",
            "starts the stream, and goes out at open",
        ),
        (
            DeviceFamily::Ut171,
            "pause",
            "stops the stream the app reads",
        ),
    ];

    /// The × that leaves a mode, drawn only with its button and the mode on:
    /// the button's command, and the flags that light it.
    fn parent_of_exit(command: &str) -> Option<(&'static str, StatusFlags)> {
        match command {
            command::EXIT_REL => Some((
                command::REL,
                StatusFlags {
                    rel: true,
                    ..Default::default()
                },
            )),
            command::EXIT_MINMAX => Some((
                command::MINMAX,
                StatusFlags {
                    max: true,
                    ..Default::default()
                },
            )),
            command::EXIT_PEAK => Some((
                command::PEAK,
                StatusFlags {
                    peak_max: true,
                    ..Default::default()
                },
            )),
            _ => None,
        }
    }

    /// Every button drawn for `commands`, the app's own included.
    fn row_buttons_for(commands: &[&str], flags: StatusFlags) -> Vec<String> {
        let mut app = connected_app(commands, "V");
        app.last_measurement = Some(Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            flags,
        ));
        buttons_of(&mut app)
    }

    /// Whether `key`'s `applies` holds for any reading: a context key that
    /// never applies is never drawn.
    fn applies_to_some_reading(key: &dmm_lib::protocol::MeterKey) -> bool {
        let units = ["V", "mV", "A", "mA", "F", "nF", "Hz", "%", "Ω", "°C"];
        (0..=u16::from(u8::MAX)).any(|mode_raw| {
            units.iter().any(|unit| {
                let mut reading = Measurement::test_fixture(
                    MeasuredValue::Normal(1.234),
                    unit,
                    StatusFlags::default(),
                );
                reading.mode_raw = mode_raw;
                (key.applies)(&reading)
            })
        })
    }

    /// Why `command`, advertised by `profile`, has no way onto the screen;
    /// `None` when it has one. `row_only` is what the row alone draws for a
    /// lone command, past the buttons it draws for none.
    fn gap(
        profile: &DeviceProfile,
        command: &str,
        row_only: &mut impl FnMut(&str) -> usize,
    ) -> Option<String> {
        let keys = profile.meter_keys;
        // A function key is drawn in the mode dropdown, unless the family also
        // offers mode choices: none that lists these keys does.
        if keys.functions.iter().any(|k| k.command == command) {
            return None;
        }
        if let Some(key) = keys.context.iter().find(|k| k.command == command) {
            return (!applies_to_some_reading(key)).then(|| "its key never applies".to_string());
        }
        if let Some((parent, flags)) = parent_of_exit(command) {
            if !profile.supported_commands.contains(&parent) {
                return Some(format!("{parent} isn't advertised, so no x is drawn"));
            }
            let exit_drawn = row_buttons_for(&[parent, command], flags)
                .iter()
                .any(|label| label.starts_with("Exit "));
            return (!exit_drawn).then(|| "no x is drawn".to_string());
        }
        match row_only(command) {
            1 => None,
            0 => Some("no button or key".to_string()),
            n => Some(format!("{n} buttons for one command")),
        }
    }

    /// Every keypress a model advertises reaches the GUI. Each command is a
    /// key the family lists, a button the row draws, the × of one, or on a
    /// list here; an entry on a list that no longer needs it fails too.
    #[test]
    fn every_advertised_command_reaches_the_gui() {
        let none = row_buttons_for(&["no_such_command"], StatusFlags::default()).len();
        let mut drawn: BTreeMap<String, usize> = BTreeMap::new();
        let mut row_only = |command: &str| {
            *drawn.entry(command.to_string()).or_insert_with(|| {
                row_buttons_for(&[command], StatusFlags::default())
                    .len()
                    .saturating_sub(none)
            })
        };
        let listed = |family: DeviceFamily, command: &str| {
            NOT_A_BUTTON
                .iter()
                .any(|&(f, c, _)| (f, c) == (family, command))
        };
        let mut gaps = Vec::new();
        let mut seen = Vec::new();
        for device in registry::DEVICES {
            let protocol = (device.new_protocol)();
            let profile = protocol.profile();
            for &command in profile.supported_commands {
                let gap = gap(profile, command, &mut row_only);
                if listed(device.family, command) {
                    seen.push((device.family, command));
                    if gap.is_none() {
                        gaps.push(format!(
                            "{}: {command} is listed, but has a way onto the screen",
                            device.id
                        ));
                    }
                } else if let Some(gap) = gap {
                    gaps.push(format!("{}: {command}: {gap}", device.id));
                }
            }
        }
        for &(family, command, _) in NOT_A_BUTTON {
            if !seen.contains(&(family, command)) {
                gaps.push(format!(
                    "{family:?} lists {command}, which nothing advertises"
                ));
            }
        }
        assert!(gaps.is_empty(), "{gaps:#?}");
    }
}
