use eframe::egui::{self, Response, Ui};

/// Override the AccessKit label for a widget whose visible text is not
/// descriptive (icon-only buttons, custom-painted widgets, clickable labels
/// whose literal text isn't meaningful to a screen reader).
pub(crate) fn set_accessible_label(ui: &Ui, id: egui::Id, label: &str) {
    ui.ctx()
        .accesskit_node_builder(id, |builder| builder.set_label(label));
}

/// Tag a button with on/off state for screen readers without emitting a
/// duplicate `OutputEvent::Clicked`. Use this on plain `egui::Button`s that
/// behave as toggles (HOLD, REL, LIVE, etc.) — the button's color change
/// alone does not communicate the state to AT users.
///
/// Do not use `Response::widget_info(WidgetInfo::selected(...))` for the same
/// purpose: the Button widget already calls `widget_info` internally with a
/// labeled (non-selected) info, and a second `widget_info` call pushes a
/// second `OutputEvent::Clicked` on the click frame — causing AT to announce
/// the click twice.
pub(crate) fn set_toggled(ui: &Ui, id: egui::Id, selected: bool) {
    use egui::accesskit::Toggled;
    ui.ctx().accesskit_node_builder(id, |builder| {
        builder.set_toggled(if selected {
            Toggled::True
        } else {
            Toggled::False
        });
    });
}

/// Paint a high-contrast focus ring around `response` when it has keyboard
/// focus. Use for custom-painted widgets (color swatches, minimap, split
/// dividers) whose own paint overdraws the default focus rectangle.
pub(crate) fn paint_focus_ring(ui: &Ui, response: &Response) {
    if response.has_focus() {
        let stroke_color = ui.visuals().selection.stroke.color;
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            2.0,
            egui::Stroke::new(2.0, stroke_color),
            egui::StrokeKind::Outside,
        );
    }
}
