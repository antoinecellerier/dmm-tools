//! The keyboard and mouse help modal: the grids it renders from
//! [`super::shortcuts::help_rows`] and the graph's own tables, the scrolling
//! that keeps it usable in a short window, and the focus bookkeeping that
//! returns the caret to the button that opened it.

use eframe::egui::{self, RichText};

use super::shortcuts;
use super::{App, HelpScroll};
use crate::a11y::ResponseA11yExt;

/// How many lines an arrow key moves the help.
const SCROLL_LINES: f32 = 3.0;

/// Gap left between the modal and the window edge, on each axis.
const SCREEN_MARGIN: f32 = 16.0;

impl App {
    /// Turn the scroll keys into a pending [`HelpScroll`] while the help is
    /// open.
    ///
    /// Called before the panels, for two reasons. `Graph::handle_keyboard`
    /// runs while they are drawn — before the modal — and would take Home/End
    /// the moment a click on the modal frame surrenders focus. And
    /// `Focus::begin_pass` has already turned an unmodified Up/Down into a
    /// focus move by the time we run, so the arrows have to cancel that too:
    /// the focus cache still holds the top-bar widgets under the modal, and
    /// the close button would lose the keyboard to one of them. The minimap
    /// cancels the same way.
    pub(super) fn handle_shortcut_help_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        if !self.shortcut_help.open {
            return;
        }

        // Every key is consumed, not only the first that matches: an arrow
        // left behind would still be there for the graph to pan with.
        let taken = |key: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let up = taken(Key::ArrowUp);
        let down = taken(Key::ArrowDown);
        let page_up = taken(Key::PageUp);
        let page_down = taken(Key::PageDown);
        let home = taken(Key::Home);
        let end = taken(Key::End);

        // First match wins, so two keys arriving in one frame still scroll
        // somewhere sensible.
        let scroll = if down {
            Some(HelpScroll::Lines(1.0))
        } else if up {
            Some(HelpScroll::Lines(-1.0))
        } else if page_down {
            Some(HelpScroll::Page(1.0))
        } else if page_up {
            Some(HelpScroll::Page(-1.0))
        } else if home {
            Some(HelpScroll::Top)
        } else if end {
            Some(HelpScroll::Bottom)
        } else {
            None
        };
        if let Some(scroll) = scroll {
            self.shortcut_help.scroll = scroll;
        }
        if up || down {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
    }

    pub(super) fn show_shortcut_help(&mut self, ctx: &egui::Context) {
        if !self.shortcut_help.open {
            return;
        }

        let focus_pending = std::mem::take(&mut self.shortcut_help.focus_pending);
        // A fresh open starts at the top: egui keeps the scroll offset
        // between shows, so the help would otherwise reopen wherever it was
        // last read.
        if focus_pending {
            self.shortcut_help.scroll = HelpScroll::Top;
        }
        let mut close_clicked = false;
        // egui::Modal (vs. egui::Window) calls `set_modal_layer`, which makes
        // Tab navigation skip widgets in the layers below — i.e. it actually
        // traps keyboard focus inside the dialog. Window does not do this.
        let modal_response =
            egui::Modal::new(egui::Id::new("shortcut_help_modal")).show(ctx, |ui| {
                // Both caps are measured against the screen rather than
                // `ui.available_*`: inside an `Area` the max rect is *last
                // frame's* area rect, which on the sizing pass is the default
                // window. And the height cap has to exist at all — egui
                // clamps the modal's sizing pass only, after which the area
                // rect is the content's min size, nudged on-screen and
                // clipped, so in a window shorter than the help the lower
                // grids were painted away with no way to reach them.
                let screen = ctx.content_rect();
                let margins = egui::Frame::popup(ui.style()).total_margin().sum();
                ui.set_max_width(520.0_f32.min(screen.width() - margins.x - SCREEN_MARGIN));
                ui.set_max_height(screen.height() - margins.y - SCREEN_MARGIN);
                ui.horizontal(|ui| {
                    ui.heading("Keyboard & Mouse");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // U+00D7: none of egui's bundled fonts has U+2715
                        // (MULTIPLICATION X), which drew a missing-glyph box.
                        let close_btn =
                            ui.button("\u{00D7}").a11y_label("Close keyboard shortcuts");
                        // First focus stop: the close button. Tab/Shift+Tab
                        // walks from here through the (non-interactive) grid
                        // labels.
                        if focus_pending {
                            close_btn.request_focus();
                        }
                        if close_btn.clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.separator();
                // The title row stays put; only the tables scroll. With room
                // for all of them the scroller shrinks to its content and the
                // modal looks exactly as it did before it had one.
                egui::ScrollArea::vertical()
                    .id_salt("shortcut_help_scroll")
                    // A key press jumps. Left animated, egui parks the delta
                    // as a target that the *next* pass turns into an offset,
                    // and it asks for no repaint once it arrives — so the
                    // rows could sit still until something else redrew.
                    .animated(false)
                    .show(ui, |ui| {
                        // `ScrollArea` handles the wheel but no keys, and the
                        // modal keeps focus on the close button, so the keys
                        // arrive here as a pending delta. A negative y moves
                        // the content up, i.e. scrolls down.
                        let line = ui.text_style_height(&egui::TextStyle::Body);
                        let delta = match std::mem::take(&mut self.shortcut_help.scroll) {
                            HelpScroll::None => None,
                            HelpScroll::Lines(dir) => Some(-dir * SCROLL_LINES * line),
                            // A line of overlap, so a page turn keeps a
                            // landmark on screen.
                            HelpScroll::Page(dir) => Some(-dir * (ui.clip_rect().height() - line)),
                            // Past either end; `ScrollArea` clamps the offset.
                            HelpScroll::Top => Some(1e4),
                            HelpScroll::Bottom => Some(-1e4),
                        };
                        if let Some(dy) = delta {
                            ui.scroll_with_delta(egui::vec2(0.0, dy));
                        }

                        // `num_columns` is what makes the wrapped action
                        // column below work: without it egui hands the last
                        // cell the previous frame's column width — the 120 pt
                        // minimum — and a wrapping label never grows past what
                        // it is given, so every row wrapped into a narrow
                        // ribbon.
                        egui::Grid::new("shortcuts_app")
                            .num_columns(2)
                            .min_col_width(120.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("General").strong());
                                ui.end_row();
                                // Rendered from the same table
                                // `handle_keyboard_shortcuts` dispatches, so
                                // the two cannot drift.
                                for (key, action, inert) in
                                    shortcuts::help_rows(ctx, self.on_wayland)
                                {
                                    let mut key = RichText::new(key).monospace();
                                    let mut action = RichText::new(action);
                                    // Greyed like the setting it mirrors; the
                                    // text still says why, so colour is only
                                    // the cue.
                                    if inert {
                                        let weak = ui.visuals().weak_text_color();
                                        key = key.color(weak);
                                        action = action.color(weak);
                                    }
                                    ui.label(key);
                                    // Wrapped, not extended: the Wayland note
                                    // on Ctrl+T is a sentence, and a grid cell
                                    // that wide would push the modal past its
                                    // max width.
                                    ui.add(egui::Label::new(action).wrap());
                                    ui.end_row();
                                }
                            });

                        ui.add_space(8.0);

                        egui::Grid::new("shortcuts_graph")
                            .num_columns(2)
                            .min_col_width(120.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Graph (keys)").strong());
                                ui.end_row();
                                for (key, action) in [
                                    ("[ / ]", "Shorter / longer time window"),
                                    ("Left / Right", "Scroll view"),
                                    ("Home", "Jump to start"),
                                    ("End", "Jump to live"),
                                ] {
                                    ui.label(RichText::new(key).monospace());
                                    ui.add(egui::Label::new(action).wrap());
                                    ui.end_row();
                                }
                            });

                        ui.add_space(8.0);

                        egui::Grid::new("gestures_graph")
                            .num_columns(2)
                            .min_col_width(120.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Graph (mouse)").strong());
                                ui.end_row();
                                for (gesture, action) in [
                                    ("Drag", "Pan left / right through history"),
                                    ("Shift + drag", "Zoom to bounding box (time & value)"),
                                    ("Ctrl + scroll wheel", "Zoom X axis centered on cursor"),
                                    ("Scroll wheel", "Scroll the panel"),
                                    ("Double-click", "Reset to live follow + auto Y"),
                                    ("Click (cursors on)", "Place cursor A / B at nearest point"),
                                    ("Minimap drag", "Jump to time / resize viewport"),
                                ] {
                                    ui.label(RichText::new(gesture).monospace());
                                    ui.add(egui::Label::new(action).wrap());
                                    ui.end_row();
                                }
                            });

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Graph and Space shortcuts are disabled while any widget has \
                                 keyboard focus — press Escape to release it. Up/Down, PgUp/PgDn \
                                 and Home/End scroll this help.",
                            )
                            .small()
                            .color(ui.visuals().weak_text_color()),
                        );
                    });
            });

        // Close on: (a) close button click, (b) Esc / backdrop click via
        // Modal::should_close, or (c) Ctrl+W (still consumed in
        // handle_keyboard_shortcuts so it works while focus is in the modal).
        if close_clicked || modal_response.should_close() {
            self.shortcut_help.open = false;
            // Defer focus restoration. On this frame `top_modal_layer` is
            // still set, and egui's `create_widget` will call
            // `surrender_focus` on every top-bar widget below the modal
            // layer on the *next* frame — including the `?` button — which
            // silently wipes any focus we set here. The deferred restore
            // fires once `top_modal_layer` has actually cleared, so the
            // target widget can keep the focus it's given.
            self.shortcut_help.restore_focus = self.shortcut_help.opener.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use eframe::egui::accesskit::{Node, NodeId};
    use eframe::egui::{Id, Pos2, Rect, vec2};

    /// Shorter than the help is tall — the window the scrolling is for.
    fn short_window() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(300.0, 200.0))
    }

    fn help_app() -> App {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.shortcut_help.open = true;
        app
    }

    /// What one headless frame left behind: the AccessKit tree and the node
    /// holding the keyboard.
    struct TestFrame {
        nodes: Vec<(NodeId, Node)>,
        focus: Option<NodeId>,
    }

    /// One frame of the modal over a stand-in for the top bar, in `App::ui`'s
    /// order: the keys are handled before any widget is drawn.
    fn run_frame(
        ctx: &egui::Context,
        app: &mut App,
        screen: Rect,
        events: Vec<egui::Event>,
    ) -> TestFrame {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx().clone();
                app.handle_shortcut_help_keys(&ctx);
                // A widget under the modal, low enough to be where an
                // unhandled ArrowDown would send the focus: egui searches its
                // focus cache, which keeps whatever registered interest
                // before the modal opened — the real top bar included.
                ui.add_space(screen.height() - 40.0);
                let _ = ui.add_sized(
                    vec2(screen.width(), 20.0),
                    egui::Button::new("under the modal"),
                );
                app.show_shortcut_help(&ctx);
            },
        );
        // epaint 0.36 added a `Drop` guard on `TexturesDelta` that
        // debug-asserts the deltas were applied. This harness renders without
        // a painter, so discard them explicitly.
        out.textures_delta.clear();
        let (nodes, focus) = out
            .platform_output
            .accesskit_update
            .map(|update| (update.nodes, Some(update.focus)))
            .unwrap_or_default();
        TestFrame { nodes, focus }
    }

    /// A key pressed and released within one frame.
    fn key(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }

    /// Where a grid heading sits on screen, as AccessKit reports it. A label
    /// carries its text as the node's *value*, not its label.
    fn row_top(frame: &TestFrame, value: &str) -> f64 {
        frame
            .nodes
            .iter()
            .find(|(_, n)| n.value() == Some(value))
            .and_then(|(_, n)| n.bounds())
            .unwrap_or_else(|| {
                let values: Vec<_> = frame.nodes.iter().filter_map(|(_, n)| n.value()).collect();
                panic!("no {value:?} row in the help; values: {values:?}")
            })
            .y0
    }

    fn modal_rect(ctx: &egui::Context) -> Rect {
        egui::AreaState::load(ctx, Id::new("shortcut_help_modal"))
            .expect("the modal registers an area")
            .rect()
    }

    /// egui clips an over-tall modal instead of shrinking it, and inside an
    /// `Area` the available height is last frame's area rect — so a help that
    /// has already been shown at its full height has to follow the window
    /// down to 300x200 and stay a dialog rather than a sliver.
    #[test]
    fn the_help_fits_a_short_window() {
        let ctx = egui::Context::default();
        let mut app = help_app();
        let roomy = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0));
        for _ in 0..3 {
            run_frame(&ctx, &mut app, roomy, vec![]);
        }
        let screen = short_window();
        // Several passes: a `Grid` hands out last pass's column widths, so
        // the shrink takes a few of them to settle.
        for _ in 0..8 {
            run_frame(&ctx, &mut app, screen, vec![]);
        }
        let rect = modal_rect(&ctx);
        assert!(
            screen.contains_rect(rect),
            "the modal {rect:?} hangs out of the {screen:?} window"
        );
        assert!(rect.height() > 100.0, "the modal collapsed to {rect:?}");
    }

    /// The modal keeps focus on its close button and `ScrollArea` handles no
    /// keys, so without this the lower grids stay unreachable.
    #[test]
    fn arrow_down_scrolls_the_help() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = help_app();
        let screen = short_window();
        run_frame(&ctx, &mut app, screen, vec![]);
        let before = row_top(&run_frame(&ctx, &mut app, screen, vec![]), "Graph (mouse)");
        run_frame(&ctx, &mut app, screen, key(egui::Key::ArrowDown));
        // The offset lands at the end of the key's own frame, so the rows
        // move on the frame after it.
        let after = row_top(&run_frame(&ctx, &mut app, screen, vec![]), "Graph (mouse)");
        assert!(
            after < before,
            "ArrowDown left the rows at {before} (now {after})"
        );
    }

    /// `Focus::begin_pass` turns an unmodified arrow into a focus move before
    /// the app runs, and the focus cache still holds the widgets under the
    /// modal — scrolling must not hand one of them the keyboard.
    #[test]
    fn arrow_keys_keep_focus_on_the_close_button() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let screen = short_window();
        // Closed for the first frame, so the widget under the modal registers
        // its interest in focus first, the way the top bar does.
        run_frame(&ctx, &mut app, screen, vec![]);
        app.shortcut_help.open = true;
        app.shortcut_help.focus_pending = true;
        run_frame(&ctx, &mut app, screen, vec![]);
        let frame = run_frame(&ctx, &mut app, screen, key(egui::Key::ArrowDown));
        let close = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Close keyboard shortcuts"))
            .map(|(id, _)| *id);
        assert!(close.is_some(), "the close button is missing from the tree");
        assert_eq!(
            frame.focus, close,
            "ArrowDown moved the keyboard off the close button"
        );
    }

    /// Given the room, the modal is what it always was: no scrolling, no
    /// scrollbar. The scroller's id is nested inside the modal and can't be
    /// rebuilt from here, so this reads the modal's height instead — on a
    /// 900 pt screen it has to be the height it takes on one that cannot
    /// clip it, and well short of the cap the screen puts on it.
    #[test]
    fn the_help_shows_no_scrollbar_when_it_fits() {
        let height_at = |screen: Rect| {
            let ctx = egui::Context::default();
            let mut app = help_app();
            for _ in 0..3 {
                run_frame(&ctx, &mut app, screen, vec![]);
            }
            modal_rect(&ctx).height()
        };
        let roomy = height_at(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0)));
        let unclipped = height_at(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 2000.0)));
        assert_eq!(roomy, unclipped, "the 900 pt window cut the help short");
        assert!(
            roomy > 400.0 && roomy < 900.0 - SCREEN_MARGIN,
            "the help is {roomy} tall: neither the whole list nor short of the cap"
        );
    }

    /// The scroll offset outlives the modal — egui stores it by id — so a
    /// help closed near its end came back there, "General" already gone.
    #[test]
    fn the_help_opens_at_the_top_again() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = help_app();
        app.shortcut_help.focus_pending = true;
        for _ in 0..3 {
            run_frame(&ctx, &mut app, short_window(), vec![]);
        }
        let first = run_frame(&ctx, &mut app, short_window(), vec![]);
        let at_top = row_top(&first, "General");
        // "General" leaves the accessibility tree once it is scrolled out of
        // view, so the scroll itself is read off a heading that stays.
        let mouse_at_top = row_top(&first, "Graph (mouse)");

        run_frame(&ctx, &mut app, short_window(), key(egui::Key::End));
        let scrolled = run_frame(&ctx, &mut app, short_window(), vec![]);
        assert!(
            row_top(&scrolled, "Graph (mouse)") < mouse_at_top,
            "End did not scroll the help"
        );

        app.shortcut_help.open = false;
        run_frame(&ctx, &mut app, short_window(), vec![]);
        app.shortcut_help.open = true;
        app.shortcut_help.focus_pending = true;
        run_frame(&ctx, &mut app, short_window(), vec![]);
        let reopened = run_frame(&ctx, &mut app, short_window(), vec![]);
        assert_eq!(
            row_top(&reopened, "General"),
            at_top,
            "the help reopened scrolled"
        );
    }
}
