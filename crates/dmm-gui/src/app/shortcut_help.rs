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

/// Width of one column of tables; a narrow window gets less.
const COLUMN_WIDTH: f32 = 520.0;

/// Space between two columns, on top of the item spacing.
const COLUMN_GAP: f32 = 24.0;

/// Space between two tables in a column, on top of the item spacing.
const SECTION_GAP: f32 = 8.0;

/// The help's sections — General, Graph (keys), Graph (mouse) with the
/// note — split into one, two or three columns. General is the tallest, so
/// it stands alone as soon as there are two. Each column is the `start..end`
/// of its sections.
const COLUMN_SECTIONS: [&[(usize, usize)]; 3] =
    [&[(0, 3)], &[(0, 1), (1, 3)], &[(0, 1), (1, 2), (2, 3)]];

/// What last frame measured of the help. The modal's width has to be set
/// before anything is placed in it, so the column count is chosen from these.
pub(super) struct HelpLayout {
    /// Each section as tall as it was drawn. Zero until measured.
    section_heights: [f32; 3],
    /// The title row and its separator, the spacing below included.
    title_height: f32,
    /// How many columns last frame was drawn in.
    columns: usize,
}

impl Default for HelpLayout {
    fn default() -> Self {
        Self {
            section_heights: [0.0; 3],
            title_height: 0.0,
            columns: 1,
        }
    }
}

/// How many columns the help takes: the fewest whose tallest column fits
/// `room`, or as many as its width holds when none does. `gap` is the space
/// between columns (x) and between sections in a column (y).
fn help_columns(heights: [f32; 3], room: egui::Vec2, gap: egui::Vec2) -> usize {
    if heights.iter().any(|h| *h <= 0.0) {
        return 1;
    }
    let fit = ((room.x + gap.x) / (COLUMN_WIDTH + gap.x)).floor() as usize;
    let most = fit.clamp(1, COLUMN_SECTIONS.len());
    (1..=most)
        .find(|&columns| tallest_column(heights, columns, gap.y) <= room.y)
        .unwrap_or(most)
}

/// The height of the tallest column when the sections are split into
/// `columns`, `gap_y` apart within a column.
fn tallest_column(heights: [f32; 3], columns: usize, gap_y: f32) -> f32 {
    COLUMN_SECTIONS[columns - 1]
        .iter()
        .map(|&(start, end)| {
            heights[start..end].iter().sum::<f32>() + gap_y * (end - start - 1) as f32
        })
        .fold(0.0, f32::max)
}

/// One heading and its table. `num_columns` is what makes the wrapped
/// action column work: without it egui hands the last cell the previous
/// frame's column width — the 120 pt minimum — and a wrapping label never
/// grows past what it is given, so every row wrapped into a narrow ribbon.
fn help_section<'a, K: Into<String>>(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    rows: impl IntoIterator<Item = (K, &'a str, bool)>,
) {
    egui::Grid::new(id)
        .num_columns(2)
        .min_col_width(120.0)
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong());
            ui.end_row();
            for (key, action, inert) in rows {
                let mut key = RichText::new(key).monospace();
                let mut action = RichText::new(action);
                // Greyed like the setting it mirrors; the text still says
                // why, so colour is only the cue.
                if inert {
                    let weak = ui.visuals().weak_text_color();
                    key = key.color(weak);
                    action = action.color(weak);
                }
                ui.label(key);
                // Wrapped, not extended: the Wayland note on Ctrl+T is a
                // sentence, and a grid cell that wide would push the modal
                // past its max width.
                ui.add(egui::Label::new(action).wrap());
                ui.end_row();
            }
        });
}

/// The graph's own rows, with no keyboard state to them.
fn plain<'a>(
    rows: &'a [(&'a str, &'a str)],
) -> impl Iterator<Item = (&'a str, &'a str, bool)> + 'a {
    rows.iter().map(|&(key, action)| (key, action, false))
}

fn graph_keys(ui: &mut egui::Ui) {
    help_section(
        ui,
        "shortcuts_graph",
        "Graph (keys)",
        plain(&[
            ("[ / ]", "Shorter / longer time window"),
            ("Left / Right", "Scroll view"),
            ("Home", "Jump to start"),
            ("End", "Jump to live"),
            ("M", "Toggle the mean line"),
            ("X", "Toggle the min/max band"),
            ("R", "Toggle reference lines, and type the values"),
            ("T", "Toggle trigger markers (with Ref on)"),
            ("C", "Toggle cursors"),
        ]),
    );
}

/// The mouse table and the closing note, which stays last in every layout.
fn graph_mouse(ui: &mut egui::Ui) {
    help_section(
        ui,
        "gestures_graph",
        "Graph (mouse)",
        plain(&[
            ("Drag", "Pan left / right through history"),
            ("Shift + drag", "Zoom to bounding box (time & value)"),
            ("Ctrl + scroll wheel", "Zoom X axis centered on cursor"),
            ("Scroll wheel", "Scroll the panel"),
            ("Double-click", "Reset to live follow + auto Y"),
            ("Click (cursors on)", "Place cursor A / B at nearest point"),
            ("Minimap drag", "Jump to time / resize viewport"),
        ]),
    );
    ui.add_space(SECTION_GAP);
    ui.label(
        RichText::new(
            "Graph and Space shortcuts are disabled while any widget has \
             keyboard focus — press Escape to release it. Up/Down, PgUp/PgDn \
             and Home/End scroll this help.",
        )
        .small()
        .color(ui.visuals().weak_text_color()),
    );
}

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
        // Both caps are measured against the screen rather than
        // `ui.available_*`: inside an `Area` the max rect is *last frame's*
        // area rect, which on the sizing pass is the default window. And the
        // height cap has to exist at all — egui clamps the modal's sizing
        // pass only, after which the area rect is the content's min size,
        // nudged on-screen and clipped, so in a window shorter than the help
        // the lower grids were painted away with no way to reach them.
        let screen = ctx.content_rect();
        let style = ctx.global_style();
        let margins = egui::Frame::popup(&style).total_margin().sum();
        let room = egui::vec2(
            screen.width() - margins.x - SCREEN_MARGIN,
            screen.height() - margins.y - SCREEN_MARGIN,
        );
        let spacing = style.spacing.item_spacing;
        let gap = spacing + egui::vec2(COLUMN_GAP, SECTION_GAP);
        // Chosen before the modal opens: the title row fills whatever width
        // it is given, so the width has to be right before anything is
        // placed, and the heights are last frame's.
        let layout = &mut self.shortcut_help.layout;
        let columns = help_columns(
            layout.section_heights,
            room - egui::vec2(0.0, layout.title_height),
            gap,
        );
        if columns != layout.columns {
            layout.columns = columns;
            // egui widens the modal from last frame's left edge, so the
            // switch would show off-centre and clipped until the next input
            // had the anchor re-centre it. It also hides the frame the tables
            // spend under new grid ids, each moved into another column, with
            // every action column at the 120 pt minimum and every row
            // wrapped into a ribbon.
            ctx.request_discard("shortcut help columns");
        }
        let column_width = COLUMN_WIDTH.min(room.x);
        let on_wayland = self.on_wayland;
        let mut measured = None;
        // egui::Modal (vs. egui::Window) calls `set_modal_layer`, which makes
        // Tab navigation skip widgets in the layers below — i.e. it actually
        // traps keyboard focus inside the dialog. Window does not do this.
        let modal_response =
            egui::Modal::new(egui::Id::new("shortcut_help_modal")).show(ctx, |ui| {
                ui.set_max_width(if columns > 1 {
                    let n = columns as f32;
                    n * COLUMN_WIDTH + (n - 1.0) * gap.x
                } else {
                    column_width
                });
                ui.set_max_height(room.y);
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
                let title_height = ui.min_rect().height() + spacing.y;
                // The title row stays put; only the tables scroll. With room
                // for all of them the scroller shrinks to its content and the
                // modal looks exactly as it did before it had one.
                let heights = egui::ScrollArea::vertical()
                    .id_salt("shortcut_help_scroll")
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

                        let section = |ui: &mut egui::Ui, index: usize| {
                            ui.vertical(|ui| match index {
                                // Rendered from the same table
                                // `handle_keyboard_shortcuts` dispatches, so
                                // the two cannot drift.
                                0 => help_section(
                                    ui,
                                    "shortcuts_app",
                                    "General",
                                    shortcuts::help_rows(ctx, on_wayland),
                                ),
                                1 => graph_keys(ui),
                                _ => graph_mouse(ui),
                            })
                            .response
                            .rect
                            .height()
                        };
                        // A grid's id comes from its parents', so a table
                        // moving into another column gets a new one; the
                        // discard on a column change hides that frame.
                        let mut heights = [0.0; 3];
                        ui.horizontal_top(|ui| {
                            for (i, &(start, end)) in
                                COLUMN_SECTIONS[columns - 1].iter().enumerate()
                            {
                                if i > 0 {
                                    ui.add_space(COLUMN_GAP);
                                }
                                ui.allocate_ui_with_layout(
                                    egui::vec2(column_width, ui.available_height()),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        for (index, height) in
                                            heights.iter_mut().enumerate().take(end).skip(start)
                                        {
                                            if index > start {
                                                ui.add_space(SECTION_GAP);
                                            }
                                            *height = section(ui, index);
                                        }
                                    },
                                );
                            }
                        });
                        heights
                    })
                    .inner;
                // The sizing pass lays out fresh grids, taller than they
                // settle; choosing from it would open in the wrong layout.
                if !ui.is_sizing_pass() {
                    measured = Some((heights, title_height));
                }
            });
        if let Some((heights, title_height)) = measured {
            let layout = &mut self.shortcut_help.layout;
            layout.section_heights = heights;
            layout.title_height = title_height;
        }

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
                time: Some(next_second()),
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

    /// A clock that jumps a second per frame, so egui's scroll animation —
    /// a few hundred milliseconds — has always finished by the next one.
    fn next_second() -> f64 {
        static SECONDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        SECONDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as f64
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
    fn row_bounds(frame: &TestFrame, value: &str) -> egui::accesskit::Rect {
        frame
            .nodes
            .iter()
            .find(|(_, n)| n.value() == Some(value))
            .and_then(|(_, n)| n.bounds())
            .unwrap_or_else(|| {
                let values: Vec<_> = frame.nodes.iter().filter_map(|(_, n)| n.value()).collect();
                panic!("no {value:?} row in the help; values: {values:?}")
            })
    }

    fn row_top(frame: &TestFrame, value: &str) -> f64 {
        row_bounds(frame, value).y0
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
        // egui animates the move and places the content a frame behind the
        // offset, so the rows have moved two frames after the key.
        run_frame(&ctx, &mut app, screen, vec![]);
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
        run_frame(&ctx, &mut app, short_window(), vec![]);
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
        run_frame(&ctx, &mut app, short_window(), vec![]);
        let reopened = run_frame(&ctx, &mut app, short_window(), vec![]);
        assert_eq!(
            row_top(&reopened, "General"),
            at_top,
            "the help reopened scrolled"
        );
    }

    const GAP: egui::Vec2 = egui::vec2(32.0, 12.0);

    #[test]
    fn the_fewest_columns_that_fit_or_as_many_as_the_width_holds() {
        let heights = [400.0, 200.0, 210.0];
        let columns = |w, h| help_columns(heights, vec2(w, h), GAP);
        assert_eq!(columns(1400.0, 900.0), 1, "fits stacked");
        assert_eq!(columns(1400.0, 600.0), 2, "General beside the graph tables");
        assert_eq!(columns(2000.0, 415.0), 3, "only three fit");
        assert_eq!(
            columns(1400.0, 415.0),
            2,
            "too narrow for three: scroll two"
        );
        assert_eq!(columns(1000.0, 600.0), 1, "too narrow for two");
        assert_eq!(
            help_columns([0.0; 3], vec2(2000.0, 10.0), GAP),
            1,
            "nothing measured yet"
        );
    }

    /// Modal heights for one, two and three columns, read off a one-column
    /// help so the windows built from them follow font changes.
    fn modal_heights() -> [f32; 3] {
        let ctx = egui::Context::default();
        let mut app = help_app();
        let tall = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 2000.0));
        for _ in 0..3 {
            run_frame(&ctx, &mut app, tall, vec![]);
        }
        let layout = &app.shortcut_help.layout;
        assert_eq!(layout.columns, 1, "a 2000 pt window split the help");
        let style = ctx.global_style();
        let chrome = layout.title_height + egui::Frame::popup(&style).total_margin().sum().y;
        let gap_y = style.spacing.item_spacing.y + SECTION_GAP;
        let heights = [1, 2, 3].map(|n| tallest_column(layout.section_heights, n, gap_y) + chrome);
        assert_eq!(
            heights[0],
            modal_rect(&ctx).height(),
            "the measured sections don't add up to the modal"
        );
        assert!(
            heights[0] > heights[1] + 40.0 && heights[1] > heights[2] + 20.0,
            "no band to test in: {heights:?}"
        );
        heights
    }

    /// A window of `width` whose height falls between what `columns` and
    /// `columns - 1` columns need.
    fn window_for(columns: usize, width: f32) -> Rect {
        let heights = modal_heights();
        let height = (heights[columns - 1] + heights[columns - 2]) / 2.0 + SCREEN_MARGIN;
        Rect::from_min_size(Pos2::ZERO, vec2(width, height))
    }

    /// A few frames from a fresh open; the last one's AccessKit tree.
    fn settled_help(ctx: &egui::Context, app: &mut App, screen: Rect) -> TestFrame {
        app.shortcut_help.focus_pending = true;
        let mut frame = run_frame(ctx, app, screen, vec![]);
        for _ in 0..3 {
            frame = run_frame(ctx, app, screen, vec![]);
        }
        frame
    }

    /// Inside the window, short of the height cap — nothing scrolls — and
    /// centred however much wider it grew.
    fn assert_whole_and_centred(ctx: &egui::Context, screen: Rect) {
        let rect = modal_rect(ctx);
        assert!(
            screen.contains_rect(rect),
            "the modal {rect:?} hangs out of the {screen:?} window"
        );
        assert!(
            rect.height() < screen.height() - SCREEN_MARGIN - 1.0,
            "the modal {rect:?} is at the height cap, so the help still scrolls"
        );
        assert!(
            (rect.center().x - screen.center().x).abs() < 1.0,
            "the widened modal {rect:?} is off centre"
        );
    }

    /// The window the help was too tall for in the user's screenshot: wide
    /// enough for two columns, so the graph tables go beside General.
    #[test]
    fn the_help_uses_two_columns_in_a_short_wide_window() {
        let screen = window_for(2, 1425.0);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = help_app();
        let frame = settled_help(&ctx, &mut app, screen);
        let general = row_bounds(&frame, "General");
        let keys = row_bounds(&frame, "Graph (keys)");
        let mouse = row_bounds(&frame, "Graph (mouse)");
        assert_eq!(keys.y0, general.y0, "the graph keys are not beside General");
        assert!(
            keys.x0 > general.x0 + f64::from(COLUMN_WIDTH) / 2.0,
            "the graph keys at {keys:?} are not right of General at {general:?}"
        );
        assert!(
            mouse.x0 == keys.x0 && mouse.y0 > keys.y0,
            "the mouse table at {mouse:?} is not under the keys at {keys:?}"
        );
        assert_whole_and_centred(&ctx, screen);
    }

    /// Shorter still, a window wide enough for three gives each table its
    /// own column.
    #[test]
    fn the_help_uses_three_columns_in_a_very_wide_window() {
        let screen = window_for(3, 2000.0);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = help_app();
        let frame = settled_help(&ctx, &mut app, screen);
        let tops = ["General", "Graph (keys)", "Graph (mouse)"].map(|t| row_bounds(&frame, t));
        for pair in tops.windows(2) {
            assert_eq!(pair[0].y0, pair[1].y0, "the tables are not side by side");
            assert!(
                pair[1].x0 > pair[0].x0 + f64::from(COLUMN_WIDTH) / 2.0,
                "{:?} is not right of {:?}",
                pair[1],
                pair[0]
            );
        }
        assert_whole_and_centred(&ctx, screen);
    }

    /// The column count is chosen from last frame's heights; it must not
    /// flip between layouts frame after frame, from a fresh open on.
    #[test]
    fn the_help_layout_settles() {
        for (columns, screen) in [(2, window_for(2, 1425.0)), (3, window_for(3, 2000.0))] {
            let ctx = egui::Context::default();
            let mut app = help_app();
            settled_help(&ctx, &mut app, screen);
            let settled = modal_rect(&ctx);
            for _ in 0..5 {
                run_frame(&ctx, &mut app, screen, vec![]);
                assert_eq!(modal_rect(&ctx), settled, "the help changed layout again");
                assert_eq!(app.shortcut_help.layout.columns, columns);
            }
        }
    }

    /// Narrowing the window folds the help into fewer columns, and widening
    /// it spreads it out again.
    #[test]
    fn the_help_reflows_on_resize() {
        let wide = window_for(3, 2000.0);
        let at_width = |width| Rect::from_min_size(Pos2::ZERO, vec2(width, wide.height()));
        let ctx = egui::Context::default();
        let mut app = help_app();
        let mut columns_at = |screen| {
            for _ in 0..3 {
                run_frame(&ctx, &mut app, screen, vec![]);
            }
            let rect = modal_rect(&ctx);
            assert!(screen.contains_rect(rect), "{rect:?} is out of {screen:?}");
            app.shortcut_help.layout.columns
        };
        assert_eq!(columns_at(wide), 3);
        assert_eq!(columns_at(at_width(1425.0)), 2);
        assert_eq!(columns_at(at_width(800.0)), 1);
        assert_eq!(columns_at(wide), 3, "widening again kept fewer columns");
    }
}
