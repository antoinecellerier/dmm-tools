//! The status toast: the transient message an action leaves behind (export
//! result, a refused mode switch, "Recording stopped — buffer full", …),
//! floated over the window's top-right corner.
//!
//! It used to be a label appended to the top bar's status row, which meant
//! minimal mode — where there is no bar — never showed one, a narrow bar cut
//! it off, and its width pushed the bar into two rows for the seconds it
//! was up. An overlay [`egui::Area`] shows the same message in every layout.

use eframe::egui::{self, RichText};
use std::time::Duration;

use super::{App, TOAST_DURATION_SECS};
use crate::a11y::{ResponseA11yExt, UiA11yExt};

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
const ERROR_GLYPH: &str = "\u{26A0}";

/// How often a visible toast asks for a repaint. Also the cadence the app
/// already repaints at while connected.
const TOAST_REPAINT_INTERVAL: Duration = Duration::from_millis(100);

impl App {
    /// Draw the toast, if there is one, right-aligned `anchor_top` points
    /// down the window: under the top bar where one is drawn, at the window's
    /// top edge in minimal mode.
    ///
    /// Call after the panels — [`egui::Order::Foreground`] keeps it above
    /// them. A modal drawn later still covers it, which is what we want.
    pub(super) fn show_toast(&mut self, ctx: &egui::Context, anchor_top: f32) {
        let Some((message, is_error, shown_at)) = &self.toast else {
            return;
        };

        // Keep repainting while it is up. Nothing else repaints when
        // disconnected, so without this the toast would sit there until the
        // next input instead of expiring; the same repaint also lets a
        // replaced message correct its box position (an `Area` anchors from
        // the previous frame's size) before anyone sees it.
        let remaining = Duration::from_secs(TOAST_DURATION_SECS)
            .saturating_sub(shown_at.elapsed())
            .min(TOAST_REPAINT_INTERVAL);
        ctx.request_repaint_after(remaining);

        let content = ctx.content_rect();
        let max_width = (content.width() - 2.0 * TOAST_MARGIN).min(TOAST_MAX_WIDTH);
        // `Area::anchor` offsets from the content rect, not from the space the
        // panels left over, so the bar's height has to be taken off here.
        let offset = egui::vec2(-TOAST_MARGIN, anchor_top - content.top() + TOAST_MARGIN);

        let mut dismissed = false;
        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, offset)
            .show(ctx, |ui| {
                // Before anything is placed: inside an `Area` the ui's
                // `max_rect` is the previous frame's box, so the label would
                // otherwise wrap to whatever the last message needed.
                ui.set_max_width(max_width);
                let tc = self.settings.theme_colors(ui.visuals().dark_mode);
                let (color, glyph) = if *is_error {
                    (tc.status_error(), ERROR_GLYPH)
                } else {
                    (tc.status_ok(), OK_GLYPH)
                };
                egui::Frame::popup(ui.style())
                    .inner_margin(TOAST_PADDING)
                    .stroke(egui::Stroke::new(TOAST_STROKE_WIDTH, color))
                    .show(ui, |ui| {
                        // Explicit id_salt, as the top bar's status landmark:
                        // the auto-derived scope id shifts with the siblings
                        // above and AT loses the landmark.
                        ui.landmark("toast_landmark", egui::accesskit::Role::Status, |ui| {
                            // The close button goes in first, from the right,
                            // so the text wraps in what is left rather than
                            // pushing the button out of the box.
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                let close = ui
                                    .add(
                                        egui::Button::new(RichText::new("\u{00D7}").small())
                                            .frame_when_inactive(false),
                                    )
                                    .on_hover_text("Dismiss")
                                    .a11y_label("Dismiss notification");
                                dismissed = close.clicked();
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.label(RichText::new(glyph).color(color));
                                        ui.add(
                                            egui::Label::new(RichText::new(message).color(color))
                                                .wrap(),
                                        );
                                    },
                                );
                            });
                        });
                    });
            });
        if dismissed {
            self.toast = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BigMeterMode;
    use crate::settings::Settings;
    use eframe::egui::accesskit::{Node, NodeId, Role};
    use eframe::egui::{Id, Pos2, Rect, vec2};
    use std::time::Instant;

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
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            Self {
                app: App::from_settings(Settings::default(), dmm_lib::Clock::real()),
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
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(self.screen),
                    time: Some(self.seconds),
                    events,
                    ..Default::default()
                },
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
            // epaint 0.36 added a `Drop` guard on `TexturesDelta` that
            // debug-asserts the deltas were applied. This harness renders
            // without a painter, so discard them explicitly.
            out.textures_delta.clear();
            let nodes = out
                .platform_output
                .accesskit_update
                .map(|update| update.nodes)
                .unwrap_or_default();
            ToastFrame { nodes }
        }

        /// Three frames: an `Area` sizes itself from the previous one, and
        /// the bar's row decision comes from cached widths.
        fn settle(&mut self) -> ToastFrame {
            self.frame();
            self.frame();
            self.frame()
        }

        fn show_toast(&mut self, message: &str) {
            self.app.toast = Some((message.to_string(), true, Instant::now()));
        }

        /// The width the bar's left group reported last frame — the number
        /// that decides whether the bar wraps to two rows, and feeds the
        /// window's minimum width.
        fn bar_left_width(&self) -> f32 {
            self.ctx
                .data(|d| d.get_temp(Id::new("top_bar_left_w")))
                .expect("the bar caches its left width every frame")
        }

        fn toast_rect(&self) -> Rect {
            egui::AreaState::load(&self.ctx, Id::new("toast"))
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
        let b = frame
            .nodes
            .iter()
            .find(|(_, n)| n.value() == Some(text))
            .and_then(|(_, n)| n.bounds())
            .expect("the toast label is in the tree");
        Rect::from_min_max(
            Pos2::new(b.x0 as f32, b.y0 as f32),
            Pos2::new(b.x1 as f32, b.y1 as f32),
        )
    }

    /// Where the node with this accessible label was drawn.
    fn labelled_bounds(frame: &ToastFrame, label: &str) -> Rect {
        let b = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .and_then(|(_, n)| n.bounds())
            .unwrap_or_else(|| panic!("no node labelled {label:?} in the tree"));
        Rect::from_min_max(
            Pos2::new(b.x0 as f32, b.y0 as f32),
            Pos2::new(b.x1 as f32, b.y1 as f32),
        )
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
            run.app.toast.is_none(),
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
            for glyph in [OK_GLYPH, ERROR_GLYPH] {
                assert_ne!(
                    atlas_rect(ui, glyph),
                    replacement,
                    "{glyph:?} draws as the replacement box"
                );
            }
        });
        out.textures_delta.clear();
    }
}
