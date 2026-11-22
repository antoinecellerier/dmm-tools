//! The status toasts: the transient messages actions leave behind (export
//! result, a refused mode switch, "Recording stopped — buffer full", …),
//! stacked over the window's top-right corner with the newest on top.
//!
//! They float in an overlay [`egui::Area`] rather than sit in the top bar's
//! status row, so minimal mode — where there is no bar — shows them, a narrow
//! bar neither cuts them off nor wraps to hold them.
//!
//! Each message is a card of its own, with its own clock and close button,
//! so one arriving a second after another covers nothing. [`Toasts::push`]
//! keeps the stack honest: a repeat or a newer word from the same control
//! takes its card's place, and the stack is bounded by a count and by the
//! height it may cover. While the pointer is over the stack it holds still
//! (see [`Toasts::push`]), so the × under the pointer stays the one aimed at.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, RichText};
use std::time::{Duration, Instant};

use super::App;
use crate::a11y::{ResponseA11yExt, UiA11yExt};

/// How long a toast stays up unless the user closes it first.
const TOAST_DURATION: Duration = Duration::from_secs(8);

/// How long a caution stays up: it is longer to read, and nothing on screen
/// explains it.
const CAUTION_DURATION: Duration = Duration::from_secs(15);

/// Widest a toast box gets (logical points). Long export paths would
/// otherwise stretch it across the whole window.
const TOAST_MAX_WIDTH: f32 = 420.0;

/// Gap between the toast and the window edge, and between it and whatever it
/// is anchored under.
const TOAST_MARGIN: f32 = 8.0;

/// Room between the box's border and its contents, sideways and vertically.
/// Logical points, like everything egui lays out, so the box grows with the
/// zoom setting and the screen's scale factor rather than staying a fixed
/// number of pixels.
const TOAST_PADDING: egui::Margin = egui::Margin::symmetric(12, 8);

/// The border, in the status colour: what makes the box read as a
/// notification from across the room, where the text colour alone did not.
const TOAST_STROKE_WIDTH: f32 = 1.5;

/// The glyph in front of the message, so the kind of news does not rest on
/// colour alone. The heavy check mark, not U+2713: the bundled fonts have no
/// glyph for the light one and draw a box — `the_glyphs_have_fonts` below
/// keeps it that way.
const OK_GLYPH: &str = "\u{2714}";
const CAUTION_GLYPH: &str = "\u{2139}";
pub(super) const ERROR_GLYPH: &str = "\u{26A0}";

/// The × that closes a notice, a toast or one drawn in a panel.
pub(super) fn dismiss_button(ui: &mut egui::Ui, l10n: &Localizer) -> egui::Response {
    ui.add(egui::Button::new(RichText::new("\u{00D7}").small()).frame_when_inactive(false))
        .on_hover_text(tr!(l10n, "connection-dismiss.hover"))
        .a11y_label(&tr!(l10n, "connection-dismiss.a11y"))
}

/// How far the ring showing a toast's time left stands off its ×.
const RING_CLEARANCE: f32 = 2.0;

/// Draw the time a toast has left as a ring around its × at `close`: whole
/// when the toast goes up, running out clockwise from the top, so a card is
/// seen to be on its way out rather than vanishing unannounced. `left` is the
/// share of its time remaining.
fn paint_time_left(ui: &egui::Ui, close: egui::Rect, left: f32, color: egui::Color32) {
    use std::f32::consts::TAU;
    let radius = close.size().max_elem() / 2.0 + RING_CLEARANCE;
    // Segments short enough to read as a curve at this size.
    let steps = (48.0 * left).ceil().max(1.0) as usize;
    let gone = TAU * (1.0 - left);
    let points = (0..=steps)
        .map(|i| {
            // From where the time already gone ends, round to the top.
            let angle = gone + (TAU - gone) * i as f32 / steps as f32 - TAU / 4.0;
            close.center() + radius * egui::Vec2::angled(angle)
        })
        .collect();
    let stroke = egui::Stroke::new(TOAST_STROKE_WIDTH, color);
    ui.painter().add(egui::Shape::line(points, stroke));
}

/// How often a visible toast asks for a repaint. Also the cadence the app
/// already repaints at while connected.
const TOAST_REPAINT_INTERVAL: Duration = Duration::from_millis(100);

/// What kind of news a toast is, which its colour and glyph say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastKind {
    Ok,
    /// Nothing went wrong, but something the user may want to act on.
    Caution,
    Error,
}

/// The most cards on screen at once, however tall the window.
const MAX_TOASTS: usize = 4;

/// The share of the height under the top bar the stack may cover: what is
/// under it — the reading, in minimal mode — stays mostly in view.
const STACK_HEIGHT_SHARE: f32 = 0.5;

/// The order cards go in when the stack is full: plain news first, a failure
/// last.
const EVERY_KIND: [ToastKind; 3] = [ToastKind::Ok, ToastKind::Caution, ToastKind::Error];

/// Gap between two cards.
const TOAST_GAP: f32 = 4.0;

/// A control whose every toast replaces its last one: each says where the
/// control stands now, and the one before it no longer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastSlot {
    /// An import's progress and its result.
    Import,
    Alarm,
    Scale,
}

/// A transient status message: what it says, what kind of news it is, and
/// when it went up. Timed on real time, not the session clock — it is UI
/// cadence, and a scaled mock clock must not hold one on screen for hours.
pub(super) struct Toast {
    pub(super) message: String,
    pub(super) kind: ToastKind,
    slot: Option<ToastSlot>,
    /// Names its card's widgets, given by [`Toasts::push`]: a card keeps its
    /// close button's id as others come and go around it.
    id: u64,
    /// When it was first drawn. A toast raised while the window is hidden
    /// (readings drain from `App::logic` then) has its clock start when the
    /// window comes back, so it is not gone before anyone could read it.
    shown_at: Option<Instant>,
    /// How tall its card was drawn, once it has been.
    height: Option<f32>,
}

impl Toast {
    /// Good or neutral news, up for its full time from its first frame.
    pub(super) fn info(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastKind::Ok)
    }

    /// Worth knowing, with nothing gone wrong yet: up for its full time from
    /// its first frame.
    pub(super) fn caution(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastKind::Caution)
    }

    /// A failure, up for its full time from its first frame.
    pub(super) fn error(message: impl Into<String>) -> Self {
        Self::new(message.into(), ToastKind::Error)
    }

    fn new(message: String, kind: ToastKind) -> Self {
        Self {
            message,
            kind,
            slot: None,
            id: 0,
            shown_at: None,
            height: None,
        }
    }

    /// This toast in `slot`, where it takes the place of the slot's last one.
    pub(super) fn replacing(mut self, slot: ToastSlot) -> Self {
        self.slot = Some(slot);
        self
    }

    /// Whether it is a failure.
    #[cfg(test)]
    pub(super) fn is_error(&self) -> bool {
        self.kind == ToastKind::Error
    }

    /// Whether it is a caution.
    #[cfg(test)]
    pub(super) fn is_caution(&self) -> bool {
        self.kind == ToastKind::Caution
    }

    /// How long it stays up from its first frame.
    fn duration(&self) -> Duration {
        match self.kind {
            ToastKind::Caution => CAUTION_DURATION,
            ToastKind::Ok | ToastKind::Error => TOAST_DURATION,
        }
    }

    /// How long it has left on screen: all of it until it is first drawn.
    fn remaining(&self) -> Duration {
        self.shown_at.map_or(self.duration(), |at| {
            self.duration().saturating_sub(at.elapsed())
        })
    }

    /// The share of its time it has left, from 1 down to 0.
    fn left(&self) -> f32 {
        self.remaining().as_secs_f32() / self.duration().as_secs_f32()
    }

    /// Whether it has been up its full time.
    fn expired(&self) -> bool {
        self.remaining().is_zero()
    }

    /// Whether `newer` makes this card redundant: it speaks for the same
    /// control, or says the same thing again.
    fn superseded_by(&self, newer: &Toast) -> bool {
        (newer.slot.is_some() && self.slot == newer.slot)
            || (self.kind == newer.kind && self.message == newer.message)
    }
}

/// The toasts on screen, oldest first. Everything that raises one goes
/// through [`Toasts::push`].
#[derive(Default)]
pub(super) struct Toasts {
    cards: Vec<Toast>,
    next_id: u64,
    /// When the stack was last drawn with the pointer over it, while it
    /// still is: the stack is held, so nothing in it moves or runs out
    /// under a click on its way to a ×.
    held_at: Option<Instant>,
}

impl Toasts {
    /// Put `toast` up as the newest card.
    ///
    /// It takes the place of the cards it supersedes — the last word from
    /// the same [`ToastSlot`], or the same message still on screen — so a
    /// repeated failure or a control toggled back and forth holds one card,
    /// with a fresh clock, however often it is raised. Past [`MAX_TOASTS`]
    /// an older card makes room, whatever its kind (see `evict`).
    ///
    /// A held stack keeps its cards where they are: the toast goes where
    /// the card it supersedes was, or under the others, out of order. Only
    /// a full stack still moves, to stay in its bound.
    pub(super) fn push(&mut self, mut toast: Toast) {
        // Here too, not only on a drawn frame: toasts are also raised while
        // the window is hidden, and a card past its time must neither absorb
        // a repeat nor count against the bound.
        self.expire();
        // Its card's id and height with it: an import's progress is raised
        // afresh every frame, and its close button has to stay one widget
        // and its card one the height bound counts.
        let first = self.cards.iter().position(|c| c.superseded_by(&toast));
        let mut taken = None;
        self.cards.retain(|card| {
            let superseded = card.superseded_by(&toast);
            if superseded && taken.is_none() {
                taken = Some((card.id, card.height));
            }
            !superseded
        });
        (toast.id, toast.height) = taken.unwrap_or_else(|| {
            self.next_id += 1;
            (self.next_id, None)
        });
        match (self.held_at, first) {
            (None, _) => {
                self.cards.push(toast);
                while self.cards.len() > MAX_TOASTS && self.evict(&EVERY_KIND) {}
            }
            (Some(_), Some(at)) => self.cards.insert(at, toast),
            (Some(_), None) => {
                while self.cards.len() >= MAX_TOASTS && self.evict(&EVERY_KIND) {}
                self.cards.insert(0, toast);
            }
        }
    }

    /// Take down what `slot`'s control last said: something else is about
    /// to say where it stands now.
    pub(super) fn retire(&mut self, slot: ToastSlot) {
        self.cards.retain(|card| card.slot != Some(slot));
    }

    /// Drop the cards that have been up their full time, unless the stack
    /// is held: its clocks are stopped, and are put back by the next frame.
    pub(super) fn expire(&mut self) {
        if self.held_at.is_none() {
            self.cards.retain(|toast| !toast.expired());
        }
    }

    /// Drop one card to make room, and say whether one went: the oldest of
    /// the first of `kinds` there is a card of — never the newest, which is
    /// why room is being made. Dropped rather than held back until there is
    /// room: news shown tens of seconds late reads as if it had just
    /// happened.
    fn evict(&mut self, kinds: &[ToastKind]) -> bool {
        let Some((_newest, older)) = self.cards.split_last() else {
            return false;
        };
        let at = kinds
            .iter()
            .find_map(|kind| older.iter().position(|card| card.kind == *kind));
        if let Some(at) = at {
            self.cards.remove(at);
        }
        at.is_some()
    }

    /// Keep the stack within `budget` points of height where dropping older
    /// news does it, by the heights the cards were last drawn at. A caution
    /// or a failure stays: many are said once, and covering more of the
    /// window for a few seconds costs less than losing one. A card not
    /// drawn yet counts from the next frame.
    fn fit(&mut self, budget: f32) {
        let height = |cards: &[Toast]| {
            let drawn = cards.iter().filter_map(|c| c.height);
            drawn.clone().sum::<f32>() + TOAST_GAP * drawn.count().saturating_sub(1) as f32
        };
        while height(&self.cards) > budget && self.evict(&[ToastKind::Ok]) {}
    }

    /// The toast raised last, if it is still up.
    #[cfg(test)]
    pub(super) fn latest(&self) -> Option<&Toast> {
        self.cards.last()
    }

    /// Take down the toast raised last, and hand it over.
    #[cfg(test)]
    pub(super) fn take_latest(&mut self) -> Option<Toast> {
        self.cards.pop()
    }

    /// What the cards say, oldest first.
    #[cfg(test)]
    pub(super) fn messages(&self) -> Vec<&str> {
        self.cards.iter().map(|c| c.message.as_str()).collect()
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        self.cards.clear();
    }
}

impl App {
    /// Draw the toasts, if there are any, right-aligned `anchor_top` points
    /// down the window: under the top bar where one is drawn, at the window's
    /// top edge in minimal mode.
    ///
    /// Call after the panels — [`egui::Order::Foreground`] keeps it above
    /// them. A modal drawn later still covers it, which is what we want.
    pub(super) fn show_toast(&mut self, ctx: &egui::Context, anchor_top: f32) {
        let content = ctx.content_rect();
        let now = Instant::now();
        match self.toasts.held_at {
            // Held: the time since the last frame did not count.
            Some(last) => {
                let paused = now.saturating_duration_since(last);
                for card in &mut self.toasts.cards {
                    card.shown_at = card.shown_at.map(|at| at.checked_add(paused).unwrap_or(at));
                }
            }
            None => self
                .toasts
                .fit((content.bottom() - anchor_top) * STACK_HEIGHT_SHARE),
        }
        let cards = &mut self.toasts.cards;
        let Some(soonest) = cards.iter().map(Toast::remaining).min() else {
            self.toasts.held_at = None;
            return;
        };

        // Keep repainting while one is up. Nothing else repaints when
        // disconnected or paused, so without this a toast would sit there
        // until the next input instead of expiring; the same repaint also lets
        // a changed stack correct its position (an `Area` anchors from the
        // previous frame's size) before anyone sees it.
        ctx.request_repaint_after(soonest.min(TOAST_REPAINT_INTERVAL));

        let max_width = (content.width() - 2.0 * TOAST_MARGIN).min(TOAST_MAX_WIDTH);
        // `Area::anchor` offsets from the content rect, not from the space the
        // panels left over, so the bar's height has to be taken off here.
        let offset = egui::vec2(-TOAST_MARGIN, anchor_top - content.top() + TOAST_MARGIN);
        let tc = self
            .settings
            .theme_colors(ctx.global_style().visuals.dark_mode);
        let l10n = &self.l10n;

        let mut dismissed = None;
        let stack =
        // One area for the whole stack: egui keeps an area's state for good,
        // so one per card would leave a little behind for every toast raised.
        egui::Area::new(egui::Id::unique("toast"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, offset)
            // A stack taller than the window runs off its bottom edge with
            // the oldest cards; constrained, it would slide up over the bar.
            .constrain(false)
            .show(ctx, |ui| {
                // Before anything is placed: inside an `Area` the ui's
                // `max_rect` is the previous frame's box, so the labels would
                // otherwise wrap to whatever the last stack needed.
                ui.set_max_width(max_width);
                ui.spacing_mut().item_spacing.y = TOAST_GAP;
                for toast in cards.iter_mut().rev() {
                    toast.shown_at.get_or_insert_with(Instant::now);
                    let (color, glyph) = match toast.kind {
                        ToastKind::Ok => (tc.status_ok(), OK_GLYPH),
                        ToastKind::Caution => (tc.status_warning(), CAUTION_GLYPH),
                        ToastKind::Error => (tc.status_error(), ERROR_GLYPH),
                    };
                    // Under the card's own id, so its widgets keep theirs
                    // when a card above it goes.
                    let card = ui.push_id(toast.id, |ui| {
                        egui::Frame::popup(ui.style())
                            .inner_margin(TOAST_PADDING)
                            .stroke(egui::Stroke::new(TOAST_STROKE_WIDTH, color))
                            .show(ui, |ui| {
                                // Explicit id_salt, as the top bar's status
                                // landmark: the auto-derived scope id shifts
                                // with the siblings above and AT loses the
                                // landmark.
                                let role = egui::accesskit::Role::Status;
                                ui.landmark("toast_landmark", role, |ui| {
                                    // The close button goes in first, from the
                                    // right, so the text wraps in what is left
                                    // rather than pushing the button out of
                                    // the box.
                                    let rtl = egui::Layout::right_to_left(egui::Align::TOP);
                                    ui.with_layout(rtl, |ui| {
                                        let close = dismiss_button(ui, l10n);
                                        if close.clicked() {
                                            dismissed = Some(toast.id);
                                        }
                                        paint_time_left(ui, close.rect, toast.left(), color);
                                        let ltr = egui::Layout::left_to_right(egui::Align::TOP);
                                        ui.with_layout(ltr, |ui| {
                                            ui.label(RichText::new(glyph).color(color));
                                            let text = RichText::new(&toast.message).color(color);
                                            ui.add(egui::Label::new(text).wrap());
                                        });
                                    });
                                });
                            });
                    });
                    toast.height = Some(card.response.rect.height());
                }
            });
        if let Some(id) = dismissed {
            cards.retain(|toast| toast.id != id);
        }
        // Held while the pointer is anywhere over the stack, the gaps
        // between cards included: on its way from one × to the next.
        let over = ctx
            .pointer_hover_pos()
            .is_some_and(|pos| stack.response.rect.contains(pos));
        self.toasts.held_at = over.then_some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BigMeterMode;
    use crate::app::test_support::{a11y, headless_frame, node_rect, test_app, to_rect};
    use eframe::egui::accesskit::{Node, NodeId, Role};
    use eframe::egui::{Id, Pos2, Rect, vec2};

    /// A message long enough to need wrapping in any window this app opens,
    /// and long enough that appending it to the top bar's status row would
    /// have widened the row well past the window.
    const LONG_MESSAGE: &str = "Nothing to export: record some samples first, \
        then choose where the CSV file should be written to on disk.";

    /// What one headless frame left behind.
    struct ToastFrame {
        nodes: Vec<(NodeId, Node)>,
    }

    /// The top bar and the toast over it, in `App::ui`'s order, driven a
    /// frame at a time in a `w` x `h` window.
    struct ToastRun {
        app: App,
        ctx: egui::Context,
        screen: Rect,
        /// Jumps a second per frame, so egui's fade-in — a fraction of a
        /// second — has always finished by the next one.
        seconds: f64,
    }

    impl ToastRun {
        fn new(w: f32, h: f32) -> Self {
            let ctx = crate::app::styled_context();
            ctx.enable_accesskit();
            Self {
                app: test_app(),
                ctx,
                screen: Rect::from_min_size(Pos2::ZERO, vec2(w, h)),
                seconds: 0.0,
            }
        }

        /// The panels-then-overlay order `App::ui` uses, including the
        /// anchor it hands the toast.
        fn frame(&mut self) -> ToastFrame {
            self.frame_with(Vec::new())
        }

        /// One frame with `events` delivered to it.
        fn frame_with(&mut self, events: Vec<egui::Event>) -> ToastFrame {
            self.seconds += 1.0;
            let app = &mut self.app;
            let minimal = app.big_meter_mode == BigMeterMode::Minimal;
            let out = headless_frame(
                &self.ctx,
                Some(self.screen),
                Some(self.seconds),
                events,
                |ui| {
                    let ctx = ui.ctx().clone();
                    let mut anchor_top = ctx.content_rect().top();
                    if !minimal {
                        let top = egui::Panel::top("top_bar").show(ui, |ui| {
                            app.show_top_bar(ui, &ctx);
                        });
                        anchor_top = top.response.rect.bottom();
                    }
                    app.show_toast(&ctx, anchor_top);
                },
            );
            ToastFrame {
                nodes: a11y(&out).0,
            }
        }

        /// Three frames: an `Area` sizes itself from the previous one, and
        /// the bar's row decision comes from cached widths.
        fn settle(&mut self) -> ToastFrame {
            self.frame();
            self.frame();
            self.frame()
        }

        fn show_toast(&mut self, message: &str) {
            self.app.toasts.push(Toast::error(message));
        }

        /// The space the stack may take: half of what is under the bar.
        fn stack_budget(&self) -> f32 {
            (self.screen.bottom() - self.toast_rect().top() + TOAST_MARGIN) * STACK_HEIGHT_SHARE
        }

        /// The window width the bar's left group fitted last frame — the
        /// number that decides whether the bar wraps to two rows, and feeds
        /// the window's minimum width.
        fn bar_left_width(&self) -> f32 {
            crate::app::top_bar::first_row_window_width(&self.ctx)
        }

        fn toast_rect(&self) -> Rect {
            egui::AreaState::load(&self.ctx, Id::unique("toast"))
                .expect("a visible toast registers an area")
                .rect()
        }
    }

    /// Every node under `root`, however deep.
    fn descendants(nodes: &[(NodeId, Node)], root: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            if let Some((_, n)) = nodes.iter().find(|(nid, _)| *nid == id) {
                for child in n.children() {
                    out.push(*child);
                    todo.push(*child);
                }
            }
        }
        out
    }

    /// The label a screen reader finds under a `Role::Status` landmark. A
    /// label carries its text as the node's value, not its label.
    fn status_landmark_holding(frame: &ToastFrame, text: &str) -> Option<NodeId> {
        frame
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == Role::Status)
            .find(|(id, _)| {
                descendants(&frame.nodes, *id).iter().any(|child| {
                    frame
                        .nodes
                        .iter()
                        .any(|(nid, n)| nid == child && n.value() == Some(text))
                })
            })
            .map(|(id, _)| *id)
    }

    /// Where the toast's own text sits, as AccessKit reports it. A label
    /// carries its text as the node's value, and only the widget nodes carry
    /// bounds — the landmark scope around it has none.
    fn text_bounds(frame: &ToastFrame, text: &str) -> Rect {
        node_rect(&frame.nodes, |n| n.value() == Some(text))
            .expect("the toast label is in the tree")
    }

    /// Where the node with this accessible label was drawn.
    fn labelled_bounds(frame: &ToastFrame, label: &str) -> Rect {
        node_rect(&frame.nodes, |n| n.label() == Some(label))
            .unwrap_or_else(|| panic!("no node labelled {label:?} in the tree"))
    }

    /// Where each card's close button was drawn, top to bottom.
    fn close_buttons(frame: &ToastFrame) -> Vec<Rect> {
        let mut rects: Vec<Rect> = frame
            .nodes
            .iter()
            .filter(|(_, n)| n.label() == Some("Dismiss notification"))
            .filter_map(|(_, n)| n.bounds())
            .map(to_rect)
            .collect();
        rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
        rects
    }

    /// A card that has been up its full time, or `None` where the monotonic
    /// clock started less than that ago.
    fn stale(toast: Toast) -> Option<Toast> {
        let long_ago = Instant::now().checked_sub(CAUTION_DURATION)?;
        Some(Toast {
            shown_at: Some(long_ago),
            ..toast
        })
    }

    /// Mashing a button that fails raises the same failure each time: one
    /// card says it, as the newest, with its full time again.
    #[test]
    fn a_repeat_takes_its_cards_place() {
        let mut toasts = Toasts::default();
        toasts.push(Toast::error("HOLD failed"));
        toasts.push(Toast::info("Copied"));
        toasts.cards[0].shown_at = Some(Instant::now());
        let id = toasts.cards[0].id;
        toasts.push(Toast::error("HOLD failed"));
        assert_eq!(toasts.messages(), ["Copied", "HOLD failed"]);
        let repeat = toasts.latest().expect("the repeat");
        assert_eq!(repeat.id, id, "the card's widgets changed id");
        assert_eq!(repeat.remaining(), TOAST_DURATION);
        // The same words as another kind of news are other news.
        toasts.push(Toast::info("HOLD failed"));
        assert_eq!(toasts.messages().len(), 3);
    }

    /// A control says where it stands now: its last word goes, whatever it
    /// said, and other news stays.
    #[test]
    fn a_slot_holds_one_card() {
        let mut toasts = Toasts::default();
        toasts.push(Toast::info("Alarm on").replacing(ToastSlot::Alarm));
        let id = toasts.cards[0].id;
        toasts.push(Toast::info("Scale on").replacing(ToastSlot::Scale));
        toasts.push(Toast::info("Exported"));
        toasts.push(Toast::error("Low limit above high").replacing(ToastSlot::Alarm));
        assert_eq!(
            toasts.messages(),
            ["Scale on", "Exported", "Low limit above high"]
        );
        assert_eq!(toasts.latest().map(|t| t.id), Some(id));
        // A card it also repeats goes with the slot's.
        toasts.push(Toast::info("Alarm off"));
        toasts.push(Toast::info("Alarm on").replacing(ToastSlot::Alarm));
        toasts.push(Toast::info("Alarm off").replacing(ToastSlot::Alarm));
        assert_eq!(toasts.messages(), ["Scale on", "Exported", "Alarm off"]);
        // Another control taking the alarm down takes its word down too.
        toasts.retire(ToastSlot::Alarm);
        assert_eq!(toasts.messages(), ["Scale on", "Exported"]);
    }

    /// Past the bound the oldest plain news goes, then the oldest caution;
    /// with only failures behind the newest, the oldest of them.
    #[test]
    fn a_full_stack_drops_old_news_before_a_failure() {
        let mut toasts = Toasts::default();
        toasts.push(Toast::error("first failure"));
        toasts.push(Toast::info("news"));
        for n in 2..=MAX_TOASTS {
            toasts.push(Toast::error(format!("failure {n}")));
        }
        assert_eq!(toasts.messages().len(), MAX_TOASTS);
        assert_eq!(toasts.messages()[0], "first failure", "news goes first");
        assert!(!toasts.messages().contains(&"news"));
        toasts.cards[1] = Toast::caution("a caution");
        toasts.push(Toast::error("one more failure"));
        assert!(!toasts.messages().contains(&"a caution"));
        assert_eq!(toasts.messages()[1], "failure 3");
        toasts.push(Toast::info("the newest stays"));
        assert_eq!(toasts.messages().len(), MAX_TOASTS);
        assert_eq!(toasts.messages()[0], "failure 3");
        assert_eq!(
            toasts.latest().map(|t| &*t.message),
            Some("the newest stays")
        );
    }

    /// Each card runs its own clock: one past its time goes and the others
    /// stay, and a push sweeps too — toasts are raised while the window is
    /// hidden — so a card past its time does not absorb a repeat.
    #[test]
    fn a_card_expires_on_its_own() {
        let Some(old) = stale(Toast::info("Exported")) else {
            return;
        };
        let mut toasts = Toasts::default();
        toasts.push(Toast::error("Export failed"));
        toasts.cards.push(old);
        toasts.expire();
        assert_eq!(toasts.messages(), ["Export failed"]);

        let Some(old) = stale(Toast::info("Exported")) else {
            return;
        };
        toasts.cards.insert(0, Toast { id: 99, ..old });
        toasts.push(Toast::info("Exported"));
        assert_eq!(toasts.messages(), ["Export failed", "Exported"]);
        assert_ne!(toasts.latest().map(|t| t.id), Some(99));
    }

    /// Two messages a second apart are both on screen, each a landmark of
    /// its own, the newest where the single toast has always been and the
    /// older one under it at the same width.
    #[test]
    fn toasts_stack_newest_on_top_in_every_layout() {
        for mode in [BigMeterMode::Off, BigMeterMode::Minimal] {
            let mut run = ToastRun::new(900.0, 600.0);
            run.app.big_meter_mode = mode;
            run.app.toasts.push(Toast::info("Exported 1234 samples"));
            let frame = run.settle();
            let alone = text_bounds(&frame, "Exported 1234 samples");
            run.show_toast(LONG_MESSAGE);
            let frame = run.settle();
            for text in ["Exported 1234 samples", LONG_MESSAGE] {
                assert!(
                    status_landmark_holding(&frame, text).is_some(),
                    "{mode:?}: no Status landmark holds {text:?}"
                );
            }
            let (older, newer) = (
                text_bounds(&frame, "Exported 1234 samples"),
                text_bounds(&frame, LONG_MESSAGE),
            );
            assert_eq!(newer.top(), alone.top(), "{mode:?}: the newest moved");
            assert!(
                newer.bottom() < older.top(),
                "{mode:?}: {newer:?} {older:?}"
            );
            assert_eq!(
                newer.left(),
                older.left(),
                "{mode:?}: cards differ in width"
            );
            let closes = close_buttons(&frame);
            assert_eq!(closes.len(), 2, "{mode:?}");
            assert_eq!(closes[0].right(), closes[1].right(), "{mode:?}");
        }
    }

    /// A card's × closes that card and no other.
    #[test]
    fn a_close_button_dismisses_its_own_card() {
        let mut run = ToastRun::new(900.0, 600.0);
        for message in ["oldest", "middle", "newest"] {
            run.show_toast(message);
        }
        let frame = run.settle();
        let closes = close_buttons(&frame);
        assert_eq!(closes.len(), 3);
        run.frame_with(click_at(closes[1].center()));
        assert_eq!(run.app.toasts.messages(), ["oldest", "newest"]);
        // The card under it kept its button: one more click closes it.
        let frame = run.settle();
        let closes = close_buttons(&frame);
        run.frame_with(click_at(closes[1].center()));
        assert_eq!(run.app.toasts.messages(), ["newest"]);
    }

    /// With the pointer over the stack nothing in it moves or runs out: a
    /// click on its way to a × lands on the card it was aimed at. A new
    /// message goes under the others, a repeat stays where its card was, and
    /// the clocks wait until the pointer leaves.
    #[test]
    fn a_stack_under_the_pointer_holds_still() {
        let mut run = ToastRun::new(900.0, 600.0);
        for message in ["oldest", "middle", "newest"] {
            run.show_toast(message);
        }
        let frame = run.settle();
        let closes = close_buttons(&frame);
        let aim = closes[1].center();
        run.frame_with(vec![egui::Event::PointerMoved(aim)]);
        assert!(run.app.toasts.held_at.is_some(), "the pointer is over it");

        let clock = run.app.toasts.cards[1].shown_at;
        run.app.toasts.push(Toast::info("news"));
        run.show_toast("oldest");
        let frame = run.settle();
        assert_eq!(
            run.app.toasts.messages(),
            ["news", "oldest", "middle", "newest"]
        );
        assert_eq!(close_buttons(&frame)[..3], closes[..]);
        assert!(run.app.toasts.cards[2].shown_at > clock, "its clock ran");

        // A card past its time stays while the stack is held, and its ×
        // still closes the card it is on.
        let Some(long_ago) = Instant::now().checked_sub(CAUTION_DURATION) else {
            return;
        };
        run.app.toasts.cards[3].shown_at = Some(long_ago);
        run.app.toasts.expire();
        run.frame_with(click_at(aim));
        assert_eq!(run.app.toasts.messages(), ["news", "oldest", "newest"]);
        run.frame_with(vec![egui::Event::PointerGone]);
        assert!(run.app.toasts.held_at.is_none());
        run.app.toasts.expire();
        assert_eq!(run.app.toasts.messages(), ["news", "oldest"]);
    }

    /// A held stack that is full still keeps to its bound.
    #[test]
    fn a_held_stack_stays_within_its_bound() {
        let mut toasts = Toasts {
            held_at: Some(Instant::now()),
            ..Toasts::default()
        };
        for n in 1..=MAX_TOASTS + 2 {
            toasts.push(Toast::info(format!("news {n}")));
        }
        assert_eq!(toasts.messages().len(), MAX_TOASTS);
        assert_eq!(toasts.messages()[0], format!("news {}", MAX_TOASTS + 2));
    }

    /// A card raised later starts its own clock at its own first frame.
    #[test]
    fn each_card_runs_its_clock_from_its_first_frame() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.show_toast("first");
        run.frame();
        let first = run.app.toasts.cards[0].shown_at;
        run.show_toast("second");
        assert_eq!(run.app.toasts.cards[1].remaining(), TOAST_DURATION);
        let before = Instant::now();
        run.frame();
        assert_eq!(run.app.toasts.cards[0].shown_at, first);
        assert!(
            run.app.toasts.cards[1]
                .shown_at
                .is_some_and(|at| at >= before)
        );
    }

    /// In a short window the stack keeps to half the height under the bar,
    /// so what is under it stays mostly in view: older news goes and the
    /// newest stays, as it does when it alone is taller than that.
    #[test]
    fn the_stack_keeps_to_half_the_height_under_the_bar() {
        let mut run = ToastRun::new(420.0, 300.0);
        for n in 1..=MAX_TOASTS {
            let message = format!("Exported run {n} to the measurements folder");
            run.app.toasts.push(Toast::info(message));
            // As an import's progress is, raised afresh before each frame.
            run.app
                .toasts
                .push(Toast::info("Importing").replacing(ToastSlot::Import));
            run.frame();
        }
        run.settle();
        let left = run.app.toasts.messages().len();
        assert!((1..MAX_TOASTS).contains(&left), "{left} cards left");
        assert!(
            run.toast_rect().height() <= run.stack_budget(),
            "the stack {:?} is over its {} pt",
            run.toast_rect(),
            run.stack_budget()
        );
        assert_eq!(run.app.toasts.messages().last(), Some(&"Importing"));
    }

    /// A failure or a caution is not what makes room: the settings file that
    /// could not be read stays when detection answers a moment later, in a
    /// window where half the height holds one card.
    #[test]
    fn a_short_window_keeps_a_failure_under_newer_news() {
        let mut run = ToastRun::new(300.0, 160.0);
        run.app.toasts.push(Toast::info("an earlier export"));
        run.show_toast(LONG_MESSAGE);
        run.app.toasts.push(Toast::caution("Two USB cables"));
        run.app.toasts.push(Toast::info("Detected the meter"));
        run.settle();
        assert_eq!(
            run.app.toasts.messages(),
            [LONG_MESSAGE, "Two USB cables", "Detected the meter"]
        );
    }

    /// A tall window holds the whole stack, and it starts under the bar
    /// rather than sliding up over it.
    #[test]
    fn a_full_stack_fits_a_tall_window_under_the_bar() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.show_toast("one");
        run.settle();
        let top = run.toast_rect().top();
        for n in 2..=MAX_TOASTS {
            run.show_toast(&format!("failure {n}: {LONG_MESSAGE}"));
        }
        run.settle();
        assert_eq!(run.app.toasts.messages().len(), MAX_TOASTS);
        assert_eq!(run.toast_rect().top(), top);
    }

    /// A press and release at `pos`, as the pointer would deliver them.
    fn click_at(pos: Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    /// The whole point of moving the toast off the status row: its text no
    /// longer counts towards the cached left-group width, which decides
    /// whether the bar wraps to two rows and how narrow the window may get.
    #[test]
    fn a_toast_does_not_widen_the_top_bar() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.settle();
        let quiet = run.bar_left_width();
        run.show_toast(LONG_MESSAGE);
        run.settle();
        assert_eq!(
            run.bar_left_width(),
            quiet,
            "the toast widened the bar's left group"
        );
    }

    /// Assistive tech still finds the message: a Status landmark carries it,
    /// in the normal layout and in minimal mode, where there is no bar to
    /// hang it on at all.
    #[test]
    fn the_toast_is_a_status_landmark_in_every_layout() {
        for mode in [BigMeterMode::Off, BigMeterMode::Minimal] {
            let mut run = ToastRun::new(900.0, 600.0);
            run.app.big_meter_mode = mode;
            run.show_toast("Exported 1234 samples");
            let frame = run.settle();
            assert!(
                status_landmark_holding(&frame, "Exported 1234 samples").is_some(),
                "{mode:?}: no Status landmark holds the toast text"
            );
        }
    }

    /// The toast floats below the bar at the window's right edge, not over
    /// the bar and not off the edge, and a long message stays a box rather
    /// than a ribbon across a wide window.
    #[test]
    fn the_toast_sits_under_the_top_bar() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.show_toast(LONG_MESSAGE);
        run.settle();
        let toast = run.toast_rect();
        assert!(
            toast.width() <= TOAST_MAX_WIDTH,
            "the toast {toast:?} is wider than the {TOAST_MAX_WIDTH} pt cap"
        );
        assert!(
            toast.right() <= run.screen.right() && toast.right() > run.screen.right() - 32.0,
            "the toast {toast:?} is not at the right edge of {:?}",
            run.screen
        );
        // The default top bar is one row of buttons; the toast starts below
        // it, whatever that row measures.
        assert!(
            toast.top() > 24.0,
            "the toast {toast:?} overlaps the top bar"
        );
    }

    /// A long message wraps instead of running off the edge of a small
    /// window — the failure the status row had, where the row's horizontal
    /// layout extends rather than wraps.
    #[test]
    fn a_long_toast_wraps_into_a_small_window() {
        let mut run = ToastRun::new(300.0, 240.0);
        run.show_toast(LONG_MESSAGE);
        let frame = run.settle();
        assert!(
            status_landmark_holding(&frame, LONG_MESSAGE).is_some(),
            "no Status landmark holds the toast text"
        );
        let bounds = text_bounds(&frame, LONG_MESSAGE);
        assert!(
            run.screen.contains_rect(bounds),
            "the toast text {bounds:?} hangs out of the {:?} window",
            run.screen
        );
        assert!(
            run.screen.contains_rect(run.toast_rect()),
            "the toast box {:?} hangs out of the {:?} window",
            run.toast_rect(),
            run.screen
        );
    }

    /// A toast is up for its whole duration and gone after it, whatever
    /// its kind.
    #[test]
    fn a_toast_expires_after_its_duration() {
        let fresh = Toast {
            shown_at: Some(Instant::now()),
            ..Toast::info("Exported 1234 samples")
        };
        assert!(!fresh.expired());
        assert!(fresh.remaining() > TOAST_DURATION - Duration::from_secs(1));
        let Some(long_ago) = Instant::now().checked_sub(TOAST_DURATION) else {
            return; // the monotonic clock started less than that ago
        };
        let old = Toast {
            shown_at: Some(long_ago),
            ..Toast::error("Export failed")
        };
        assert!(old.expired());
        assert_eq!(old.remaining(), Duration::ZERO);
        // The ring around the × is whole before the first frame and gone
        // at the end.
        assert_eq!(Toast::caution("Two cables").left(), 1.0);
        assert!(fresh.left() > 0.8 && fresh.left() <= 1.0);
        assert_eq!(old.left(), 0.0);
    }

    /// A toast raised while the window is hidden waits for its first frame:
    /// however long ago it was built, it has its full time once drawn.
    #[test]
    fn a_toast_runs_its_clock_from_its_first_frame() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.show_toast("Command failed");
        let toast = run.app.toasts.latest().expect("the toast was set");
        assert!(!toast.expired(), "an undrawn toast expired");
        assert_eq!(toast.remaining(), TOAST_DURATION);

        let before = Instant::now();
        run.frame();
        let shown_at = run.app.toasts.latest().and_then(|t| t.shown_at);
        assert!(
            shown_at.is_some_and(|at| at >= before),
            "drawing the toast did not start its clock"
        );
        let started = shown_at;
        run.frame();
        assert_eq!(
            run.app.toasts.latest().and_then(|t| t.shown_at),
            started,
            "a later frame restarted the clock"
        );
    }

    /// Eight seconds is long enough to read a buffer-full warning and too
    /// long to sit under an export path already read: the × closes it.
    #[test]
    fn the_close_button_dismisses_the_toast() {
        let mut run = ToastRun::new(900.0, 600.0);
        run.show_toast(LONG_MESSAGE);
        let frame = run.settle();
        let close = labelled_bounds(&frame, "Dismiss notification");
        assert!(
            run.toast_rect().contains_rect(close),
            "the close button {close:?} is outside the toast {:?}",
            run.toast_rect()
        );
        run.frame_with(click_at(close.center()));
        assert!(
            run.app.toasts.is_empty(),
            "the toast survived its close button"
        );
    }

    /// A glyph the fonts lack is drawn as a box, which says nothing about
    /// the news; egui's bundled fonts have no light check mark, only the
    /// heavy one. Not `Fonts::has_glyph`: epaint's replacement character
    /// lives in the emoji font, and that check reports every glyph of the
    /// replacement's face as missing — see `.claude/rules/gui.md`. What
    /// matters is whether the glyph lands on the replacement's atlas rect.
    #[test]
    fn the_glyphs_have_fonts() {
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
            for glyph in [OK_GLYPH, CAUTION_GLYPH, ERROR_GLYPH] {
                assert_ne!(
                    atlas_rect(ui, glyph),
                    replacement,
                    "{glyph:?} draws as the replacement box"
                );
            }
        });
    }
}
