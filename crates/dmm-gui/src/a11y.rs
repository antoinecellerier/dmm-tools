use eframe::egui::{self, InnerResponse, Response, Ui};

/// Override the AccessKit label for a widget whose visible text is not
/// descriptive (icon-only buttons, custom-painted widgets, clickable labels
/// whose literal text isn't meaningful to a screen reader).
pub(crate) fn set_accessible_label(ui: &Ui, id: egui::Id, label: &str) {
    ui.ctx()
        .accesskit_node_builder(id, |builder| builder.set_label(label));
}

/// Tag `id` with an AccessKit semantic role so assistive tech can expose it
/// as a landmark (Toolbar, Main, Status, etc.) for flat-review navigation.
/// Used by [`UiA11yExt::landmark`] for `ui.scope`-shaped regions; for an
/// existing `Response`, prefer [`ResponseA11yExt::a11y_role`].
pub(crate) fn set_role(ui: &Ui, id: egui::Id, role: egui::accesskit::Role) {
    ui.ctx()
        .accesskit_node_builder(id, |builder| builder.set_role(role));
}

/// Whichever of the two selection colours stands out against the panel.
///
/// Neither one alone works in both palettes: egui's own `selection.stroke` is
/// a pale blue that reads well on either panel while its `bg_fill` sits at
/// 2.33:1 dark / 1.55:1 light, but once the user pins an Accent the app makes
/// the fill the Accent and the stroke the *background* colour, so that the
/// caption of a selected toggle can be read on the fill — and a ring in the
/// background colour is no ring at all. Every selection-coloured cue the app
/// paints on the panel takes its colour from here for that reason, focus
/// rings and the graph's zoom rubber band alike.
pub(crate) fn focus_ring_color(visuals: &egui::Visuals) -> egui::Color32 {
    let panel = visuals.panel_fill;
    let on_panel = |c: egui::Color32| crate::theme::contrast(c, panel);
    let selection = &visuals.selection;
    if on_panel(selection.bg_fill) > on_panel(selection.stroke.color) {
        selection.bg_fill
    } else {
        selection.stroke.color
    }
}

/// Paint a high-contrast focus ring around `response` when it has keyboard
/// focus. Use for custom-painted widgets (color swatches, minimap, split
/// dividers) whose own paint overdraws the default focus rectangle, and for
/// text fields: egui draws their focused frame in `selection.stroke`, which
/// is the background colour once an Accent is pinned (`appearance.rs`).
pub(crate) fn paint_focus_ring(ui: &Ui, response: &Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            2.0,
            egui::Stroke::new(2.0_f32, focus_ring_color(ui.visuals())),
            egui::StrokeKind::Outside,
        );
    }
}

/// Axis a keyboard-resizable divider moves along.
pub(crate) enum ResizeAxis {
    /// Left/Right arrows; Right yields a positive delta.
    Horizontal,
    /// Up/Down arrows; Up yields a positive delta.
    Vertical,
}

/// Keyboard resizing for a focusable divider egui gives no keyboard action to
/// (panel resize handles, split separators).
///
/// Returns the signed delta in points — `0.0` when `handle_id` does not hold
/// keyboard focus or no arrow on `axis` was pressed. The caller applies it,
/// since dividers keep their size in different places (a struct field, an
/// [`egui::PanelState`], ...) and each has its own clamp.
pub(crate) fn arrow_resize(
    ctx: &egui::Context,
    handle_id: egui::Id,
    axis: ResizeAxis,
    step: f32,
) -> f32 {
    if ctx.memory(|m| m.focused()) != Some(handle_id) {
        return 0.0;
    }
    // Unconditional reset every frame the handle is focused — even on the
    // arrows this axis doesn't handle. egui's `Focus::begin_pass` snapshots
    // ALL arrow events into `focus_direction` before our code runs, and
    // `end_pass` would Tab-jump off the handle via `find_widget_in_direction`
    // on the unhandled axis.
    ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    let (shrink, grow) = match axis {
        ResizeAxis::Horizontal => (egui::Key::ArrowLeft, egui::Key::ArrowRight),
        ResizeAxis::Vertical => (egui::Key::ArrowDown, egui::Key::ArrowUp),
    };
    let mut delta = 0.0;
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, shrink)) {
        delta -= step;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, grow)) {
        delta += step;
    }
    delta
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

/// Chainable accessibility helpers on [`Response`]. Lets call sites attach
/// an AccessKit label or role directly onto a button/widget chain instead of
/// stashing the response and calling a separate helper:
///
/// ```ignore
/// ui.button("?")
///     .on_hover_text("Show shortcuts")
///     .a11y_label("Keyboard shortcuts");
/// ```
pub(crate) trait ResponseA11yExt {
    /// Override the AccessKit label for this response. See
    /// [`set_accessible_label`] for when to use this (icon-only buttons,
    /// custom-painted widgets, clickable labels with non-descriptive text).
    fn a11y_label(self, label: &str) -> Self;

    /// Tag this response with an AccessKit semantic role (Main, Status,
    /// Toolbar, etc.). For `ui.scope`-shaped landmarks prefer
    /// [`UiA11yExt::landmark`] which also pins a stable id_salt; use this
    /// chainable form when you have a `Response` already (e.g. the
    /// response returned from `Panel::show`).
    fn a11y_role(self, role: egui::accesskit::Role) -> Self;
}

impl ResponseA11yExt for Response {
    fn a11y_label(self, label: &str) -> Self {
        self.ctx
            .accesskit_node_builder(self.id, |builder| builder.set_label(label));
        self
    }

    fn a11y_role(self, role: egui::accesskit::Role) -> Self {
        self.ctx
            .accesskit_node_builder(self.id, |builder| builder.set_role(role));
        self
    }
}

/// Accessibility-aware scope helpers on [`Ui`].
pub(crate) trait UiA11yExt {
    /// Wrap `add_contents` in a stable-id scope tagged with an AccessKit
    /// landmark role (Toolbar, Status, Main, etc.) so assistive tech can
    /// expose it for flat-review navigation.
    ///
    /// `id_salt` must be unique within the parent — the auto-derived scope
    /// id used by `ui.scope` is a running counter and shifts whenever the
    /// parent's sibling layout changes (egui upstream gap #5), which makes
    /// AT lose the landmark.
    fn landmark<R>(
        &mut self,
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        role: egui::accesskit::Role,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;

    /// Wrap `add_contents` in `ui.horizontal` and tag the row as a polite
    /// ARIA-style live region. `fingerprint` should hash the visible value
    /// state; `make_label` is only invoked when the fingerprint changes.
    fn live_region_horizontal<R>(
        &mut self,
        fingerprint: u64,
        make_label: impl FnOnce() -> String,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;
}

impl UiA11yExt for Ui {
    fn landmark<R>(
        &mut self,
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        role: egui::accesskit::Role,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let scope = self.scope_builder(egui::UiBuilder::new().id_salt(id_salt), add_contents);
        set_role(self, scope.response.id, role);
        scope
    }

    fn live_region_horizontal<R>(
        &mut self,
        fingerprint: u64,
        make_label: impl FnOnce() -> String,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let row = self.horizontal(add_contents);
        set_live_region_cached(self, row.response.id, fingerprint, make_label);
        row
    }
}

/// Bring the control that just took keyboard focus into view.
///
/// egui scrolls to a focused widget only when assistive tech asks it to, so
/// Tab through a scrolled panel left the focused control below the fold with
/// nothing on screen to show for it. Call this last inside a `ScrollArea`'s
/// content closure: the target it sets is taken by that scroll area's `end`,
/// and the containment check keeps a scroller from answering for a widget
/// that lives in another. The move runs over egui's scroll animation, a few
/// hundred milliseconds, like every other programmatic scroll.
pub(crate) fn scroll_to_focus(ui: &Ui) {
    let ctx = ui.ctx();
    let Some(id) = ctx.memory(|m| m.focused()) else {
        return;
    };
    let Some(response) = ctx.read_response(id) else {
        return;
    };
    if response.gained_focus() && ui.min_rect().contains_rect(response.rect) {
        response.scroll_to_me(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A focus ring is a graphical element, so `.claude/rules/gui.md` asks for
    /// 3:1 on the panel it is drawn over — with egui's own selection colours
    /// and with the pair `apply_color_overrides` installs once the user pins
    /// an Accent, where the selection *stroke* is the background colour and
    /// only the fill is visible.
    #[test]
    fn the_focus_ring_stays_visible_on_the_panel() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let accent = if dark {
                egui::Color32::from_rgb(100, 180, 255)
            } else {
                egui::Color32::from_rgb(0, 100, 200)
            };
            let mut pinned = stock.clone();
            pinned.selection.bg_fill = accent;
            pinned.selection.stroke.color = pinned.panel_fill;

            for (name, visuals) in [("egui's own", &stock), ("Accent pinned", &pinned)] {
                let ring = focus_ring_color(visuals);
                let ratio = crate::theme::contrast(ring, visuals.panel_fill);
                assert!(
                    ratio >= 3.0,
                    "{name} (dark={dark}): focus ring {ring:?} on panel {:?} is {ratio:.2}:1, below 3:1",
                    visuals.panel_fill
                );
            }
        }
    }

    fn arrow(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Focused handle: the axis arrows produce ±step, off-axis arrows produce 0.
    #[test]
    fn arrow_resize_deltas() {
        let id = egui::Id::new("handle");
        for (axis, key, want) in [
            (ResizeAxis::Vertical, egui::Key::ArrowUp, 20.0),
            (ResizeAxis::Vertical, egui::Key::ArrowDown, -20.0),
            (ResizeAxis::Vertical, egui::Key::ArrowLeft, 0.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowRight, 20.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowLeft, -20.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowUp, 0.0),
        ] {
            let ctx = egui::Context::default();
            ctx.begin_pass(egui::RawInput {
                events: vec![arrow(key)],
                ..Default::default()
            });
            ctx.memory_mut(|m| m.request_focus(id));
            assert_eq!(arrow_resize(&ctx, id, axis, 20.0), want);
        }
    }

    /// Unfocused handle never resizes, whatever is pressed.
    #[test]
    fn arrow_resize_ignores_unfocused_handle() {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput {
            events: vec![arrow(egui::Key::ArrowUp)],
            ..Default::default()
        });
        let delta = arrow_resize(&ctx, egui::Id::new("handle"), ResizeAxis::Vertical, 20.0);
        assert_eq!(delta, 0.0);
    }
}
