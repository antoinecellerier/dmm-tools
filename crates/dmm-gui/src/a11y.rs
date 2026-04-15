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

/// Tag `id` with an AccessKit semantic role so assistive tech can expose it
/// as a landmark (Toolbar, Main, Status, etc.) for flat-review navigation.
pub(crate) fn set_role(ui: &Ui, id: egui::Id, role: egui::accesskit::Role) {
    ui.ctx()
        .accesskit_node_builder(id, |builder| builder.set_role(role));
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

/// Mark `id` as a polite ARIA-style live region. Screen readers announce
/// updates to polite live regions at the next pause rather than interrupting
/// the user. Intended for streaming readouts (e.g. the primary measurement
/// value).
///
/// `make_label` is only called when `fingerprint` has changed since the last
/// frame, so the caller doesn't pay for `format!`/`String` allocation on
/// frames where the underlying value is the same. The resulting label is
/// cached in `ctx.data` and re-applied every frame — egui rebuilds the
/// AccessKit tree from scratch each frame, so the label has to be set on
/// every pass even when it hasn't changed.
pub(crate) fn set_live_region_cached(
    ui: &Ui,
    id: egui::Id,
    fingerprint: u64,
    make_label: impl FnOnce() -> String,
) {
    let fp_key = id.with("a11y_live_fingerprint");
    let label_key = id.with("a11y_live_label");
    let prev_fp: Option<u64> = ui.ctx().data(|d| d.get_temp(fp_key));
    if prev_fp != Some(fingerprint) {
        let new_label = make_label();
        ui.ctx().data_mut(|d| {
            d.insert_temp(fp_key, fingerprint);
            d.insert_temp(label_key, new_label);
        });
    }
    let label: Option<String> = ui.ctx().data(|d| d.get_temp(label_key));
    ui.ctx().accesskit_node_builder(id, |builder| {
        if let Some(label) = label {
            builder.set_label(label);
        }
        builder.set_live(egui::accesskit::Live::Polite);
    });
}
