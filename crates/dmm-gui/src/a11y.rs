use std::sync::Arc;
use std::time::Duration;

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

/// The node a screen reader hears the reading from.
///
/// One fixed id for every layout and for the placeholder, not one derived
/// from egui's id counter, so the node keeps its identity when the layout
/// switches or the reading gives way to "No reading".
pub(crate) fn reading_region_id() -> egui::Id {
    egui::Id::unique("dmm_reading")
}

/// Lay the spoken reading over `rect`, the row that draws it: a polite live
/// region whose text is the whole reading, value, mode, flags and the app's
/// own state.
///
/// It is a leaf of its own, beside the row, not the row itself, for two
/// reasons. AccessKit drops `GenericContainer` nodes, which is what a
/// `ui.horizontal` is, so a label set on the row never reaches a screen
/// reader. And a node with no live setting of its own takes its parent's, so
/// on the row every label inside it — the value above all, new with every
/// reading — became a live region of its own, and Narrator cut itself off at
/// each one. A `Role::Label` takes its name from its value on Windows.
///
/// How the reading is paced for a screen reader ([`Announcer`]): the
/// **Announce readings** setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AnnouncePolicy {
    /// The least time between two announcements of a value: zero for every
    /// reading, `None` for none.
    pub(crate) every: Option<Duration>,
}

impl AnnouncePolicy {
    pub(crate) const DEFAULT: Self = Self::every_secs(Some(5));

    pub(crate) const fn every_secs(secs: Option<u32>) -> Self {
        Self {
            every: match secs {
                Some(s) => Some(Duration::from_secs(s as u64)),
                None => None,
            },
        }
    }
}

impl Default for AnnouncePolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the spoken reading shows a screen reader, and when it changes it.
///
/// A screen reader announces a live node each time its text changes, and
/// Narrator cuts off what it was saying to do so; a meter reads several times
/// a second. So a new value is shown at most once per [`AnnouncePolicy`]
/// interval: at once when the interval has passed since the last, otherwise
/// held, and shown when the interval runs out — the caller repaints then, so
/// the last value lands even after readings stop. A new mode (the dial
/// turned) and the placeholder (the connection state) are shown at once.
/// Flags, overload and the alarm's zone are not: they can flicker reading to
/// reading, and arrive within an interval anyway.
///
/// While values are not to be announced (`every` is `None`) the node keeps
/// its live setting off and its text paced at [`QUIET_PACE`], for a screen
/// reader that reads it on demand: Narrator drops what it was about to say
/// when the text of the item it is on changes, so a text new with every
/// reading was never read at all. Live goes back on only with a new value
/// shown: on Windows the switch alone announces the node again, and other
/// platforms ignore it.
///
/// [`Self::decide`] is pure and idempotent — egui reruns a frame it discards,
/// with the same time — so the live setting is stored, not derived from what
/// the frame did.
#[derive(Debug, Clone, Default)]
struct Announcer {
    /// The fingerprint and text of the label the node shows.
    shown: Option<(u64, Arc<str>)>,
    /// The mode of the shown label; `None` for the placeholder.
    shown_context: Option<u64>,
    /// When the shown label was shown, in egui's input time.
    shown_at: f64,
    live: bool,
}

/// How often the text changes while values are not announced: the default
/// interval, long enough for a screen reader to read the whole sentence.
const QUIET_PACE: Duration = Duration::from_secs(5);

/// What [`Announcer::decide`] makes of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Decision {
    /// Show the frame's label.
    take: bool,
    live: bool,
    /// Seconds until a held value is due.
    due_in: Option<f64>,
}

impl Announcer {
    /// What to do with a frame whose label has `fingerprint`, at `now`.
    /// `context` is the mode's hash, or `None` for the placeholder.
    fn decide(
        &self,
        now: f64,
        fingerprint: u64,
        context: Option<u64>,
        every: Option<Duration>,
    ) -> Decision {
        let changed = self.shown.as_ref().is_none_or(|(fp, _)| *fp != fingerprint);
        let keep = Decision {
            take: false,
            live: self.live,
            due_in: None,
        };
        if context.is_none() {
            return Decision {
                take: changed,
                live: changed || self.live,
                due_in: None,
            };
        }
        let wanted = every.is_some();
        let keep = Decision {
            live: self.live && wanted,
            ..keep
        };
        if !changed {
            return keep;
        }
        // A clock that went backwards counts as the interval passed.
        let elapsed = now - self.shown_at;
        let pace = every.unwrap_or(QUIET_PACE).as_secs_f64();
        if self.shown_context != context || elapsed < 0.0 || elapsed >= pace {
            Decision {
                take: true,
                live: wanted,
                due_in: None,
            }
        } else {
            Decision {
                due_in: Some(pace - elapsed),
                ..keep
            }
        }
    }

    fn apply(
        &mut self,
        d: Decision,
        now: f64,
        fingerprint: u64,
        context: Option<u64>,
        make_label: impl FnOnce() -> String,
    ) {
        if d.take {
            self.shown = Some((fingerprint, make_label().into()));
            self.shown_context = context;
            self.shown_at = now;
        }
        self.live = d.live;
    }
}

/// `make_label` is only called when the label is shown. The [`Announcer`]
/// keeps it in `ctx.data` as an `Arc<str>`, so a frame that shows nothing
/// new costs a refcount, and the node is rebuilt every frame: egui rebuilds
/// the AccessKit tree from scratch each one. A panel shown again is a new node
/// to the screen reader, which announces it once.
fn speak_reading(
    ui: &Ui,
    rect: egui::Rect,
    announce: AnnouncePolicy,
    fingerprint: u64,
    context: Option<u64>,
    make_label: impl FnOnce() -> String,
) {
    let id = reading_region_id();
    ui.interact(rect, id, egui::Sense::hover());
    let key = id.with("announcer");
    let now = ui.input(|i| i.time);
    let mut announcer: Announcer = ui.ctx().data(|d| d.get_temp(key)).unwrap_or_default();
    let decision = announcer.decide(now, fingerprint, context, announce.every);
    announcer.apply(decision, now, fingerprint, context, make_label);
    if let Some(secs) = decision.due_in {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(secs));
    }
    let label = announcer.shown.as_ref().map(|(_, l)| Arc::clone(l));
    let live = announcer.live;
    ui.ctx().data_mut(|d| d.insert_temp(key, announcer));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        if let Some(label) = label {
            node.set_value(&*label);
        }
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
        node.set_live(if live {
            egui::accesskit::Live::Polite
        } else {
            egui::accesskit::Live::Off
        });
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

    /// Draw `add_contents` in a `ui.horizontal` row and lay the spoken
    /// reading over it (see [`speak_reading`]), paced by `announce`.
    /// `fingerprint` should hash everything the spoken label says, `context`
    /// the mode (`None` for the placeholder); `make_label` is only invoked
    /// when the label is shown.
    fn spoken_reading<R>(
        &mut self,
        announce: AnnouncePolicy,
        fingerprint: u64,
        context: Option<u64>,
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

    fn spoken_reading<R>(
        &mut self,
        announce: AnnouncePolicy,
        fingerprint: u64,
        context: Option<u64>,
        make_label: impl FnOnce() -> String,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let row = self.horizontal(add_contents);
        speak_reading(
            self,
            row.response.rect,
            announce,
            fingerprint,
            context,
            make_label,
        );
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
    // Only when the widget isn't already in view: a scroll that moves
    // nothing still unsticks a `stick_to_bottom` scroller from its end.
    // The layer too: an open popup's entry can sit over the scroller's
    // rect without being in it.
    if response.gained_focus()
        && response.layer_id == ui.layer_id()
        && ui.min_rect().contains_rect(response.rect)
        && !ui.clip_rect().contains_rect(response.rect)
    {
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

    const FIVE: Option<Duration> = Some(Duration::from_secs(5));
    const DC_V: Option<u64> = Some(1);

    /// Run one frame through `a`: what it decides, applied as the app does.
    /// The label is the fingerprint's number.
    fn frame(
        a: &mut Announcer,
        now: f64,
        fp: u64,
        context: Option<u64>,
        every: Option<Duration>,
    ) -> Decision {
        let d = a.decide(now, fp, context, every);
        a.apply(d, now, fp, context, || fp.to_string());
        d
    }

    fn shown(a: &Announcer) -> Option<&str> {
        a.shown.as_ref().map(|(_, l)| &**l)
    }

    /// The first reading is shown and announced; changes inside the interval
    /// are held, with a repaint asked for when it runs out, and the latest
    /// lands then even though readings stopped.
    #[test]
    fn values_are_paced_and_the_last_one_lands() {
        let mut a = Announcer::default();
        let d = frame(&mut a, 10.0, 1, DC_V, FIVE);
        assert!(d.take && d.live, "{d:?}");
        let d = frame(&mut a, 11.0, 2, DC_V, FIVE);
        assert!(!d.take && d.live, "held, still live: {d:?}");
        assert_eq!(d.due_in, Some(4.0));
        let d = frame(&mut a, 12.5, 3, DC_V, FIVE);
        assert_eq!(d.due_in, Some(2.5));
        assert_eq!(shown(&a), Some("1"));
        // The repaint: no new reading, the held one is due.
        let d = frame(&mut a, 15.0, 3, DC_V, FIVE);
        assert!(d.take && d.live, "{d:?}");
        assert_eq!(shown(&a), Some("3"));
        // Nothing new: nothing to do.
        let d = frame(&mut a, 30.0, 3, DC_V, FIVE);
        assert!(!d.take && d.due_in.is_none(), "{d:?}");
    }

    #[test]
    fn every_reading_shows_each_change() {
        let mut a = Announcer::default();
        for (i, fp) in [1, 2, 3].into_iter().enumerate() {
            let d = frame(
                &mut a,
                10.0 + i as f64 * 0.1,
                fp,
                DC_V,
                Some(Duration::ZERO),
            );
            assert!(d.take && d.live, "{fp}: {d:?}");
        }
    }

    /// Never keeps live off and the text paced, for reading on demand: a
    /// screen reader drops what it was about to say when the text changes
    /// under it. Switching back turns live on only with a new value, and
    /// switching to Never turns it off at once.
    #[test]
    fn never_keeps_the_text_paced_and_silent() {
        let mut a = Announcer::default();
        let d = frame(&mut a, 10.0, 1, DC_V, None);
        assert!(d.take && !d.live, "{d:?}");
        let d = frame(&mut a, 11.0, 2, DC_V, None);
        assert!(!d.take && !d.live, "{d:?}");
        assert_eq!(d.due_in, Some(4.0));
        let d = frame(&mut a, 15.0, 2, DC_V, None);
        assert!(d.take && !d.live, "{d:?}");
        let d = frame(&mut a, 16.0, 2, DC_V, FIVE);
        assert!(!d.take && !d.live, "no new value, live stays off: {d:?}");
        let d = frame(&mut a, 17.0, 3, DC_V, FIVE);
        assert!(!d.take && !d.live && d.due_in == Some(3.0), "{d:?}");
        let d = frame(&mut a, 20.0, 3, DC_V, FIVE);
        assert!(d.take && d.live, "{d:?}");
        let d = frame(&mut a, 21.0, 3, DC_V, None);
        assert!(!d.take && !d.live, "{d:?}");
    }

    /// The placeholder — the connection state — is shown at once whatever
    /// the pace, Never included, and so is the first value after it.
    #[test]
    fn the_placeholder_is_shown_at_once() {
        for every in [FIVE, None] {
            let mut a = Announcer::default();
            frame(&mut a, 10.0, 1, DC_V, every);
            let d = frame(&mut a, 10.5, 2, None, every);
            assert!(d.take && d.live, "{every:?}: {d:?}");
            let d = frame(&mut a, 10.7, 3, None, every);
            assert!(d.take, "{every:?}: one notice replacing another: {d:?}");
            let d = frame(&mut a, 11.0, 4, DC_V, every);
            assert!(d.take, "{every:?}: {d:?}");
        }
    }

    #[test]
    fn a_new_mode_is_shown_at_once() {
        let mut a = Announcer::default();
        frame(&mut a, 10.0, 1, DC_V, FIVE);
        let d = frame(&mut a, 11.0, 2, Some(2), FIVE);
        assert!(d.take && d.live, "{d:?}");
    }

    /// egui reruns a discarded frame with the same time: the second run must
    /// agree with the first, live setting included.
    #[test]
    fn a_rerun_frame_decides_the_same() {
        for every in [FIVE, Some(Duration::ZERO), None] {
            let mut a = Announcer::default();
            frame(&mut a, 10.0, 1, DC_V, every);
            frame(&mut a, 12.0, 2, DC_V, every);
            let first = (shown(&a).map(str::to_owned), a.live);
            frame(&mut a, 12.0, 2, DC_V, every);
            assert_eq!((shown(&a).map(str::to_owned), a.live), first, "{every:?}");
        }
    }

    #[test]
    fn a_clock_that_went_back_counts_as_the_interval_passed() {
        let mut a = Announcer::default();
        frame(&mut a, 10.0, 1, DC_V, FIVE);
        let d = frame(&mut a, 3.0, 2, DC_V, FIVE);
        assert!(d.take, "{d:?}");
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
        let id = egui::Id::unique("handle");
        for (axis, key, want) in [
            (ResizeAxis::Vertical, egui::Key::ArrowUp, 20.0),
            (ResizeAxis::Vertical, egui::Key::ArrowDown, -20.0),
            (ResizeAxis::Vertical, egui::Key::ArrowLeft, 0.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowRight, 20.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowLeft, -20.0),
            (ResizeAxis::Horizontal, egui::Key::ArrowUp, 0.0),
        ] {
            let ctx = crate::app::styled_context();
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
        let ctx = crate::app::styled_context();
        ctx.begin_pass(egui::RawInput {
            events: vec![arrow(egui::Key::ArrowUp)],
            ..Default::default()
        });
        let delta = arrow_resize(&ctx, egui::Id::unique("handle"), ResizeAxis::Vertical, 20.0);
        assert_eq!(delta, 0.0);
    }
}
