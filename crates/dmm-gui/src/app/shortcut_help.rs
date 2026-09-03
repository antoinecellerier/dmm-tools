//! The keyboard and mouse help modal: the grids it renders from
//! [`super::shortcuts::help_rows`] and the graph's own tables, and the focus
//! bookkeeping that returns the caret to the button that opened it.

use eframe::egui::{self, RichText};

use super::App;
use super::shortcuts;
use crate::a11y::ResponseA11yExt;

impl App {
    pub(super) fn show_shortcut_help(&mut self, ctx: &egui::Context) {
        if !self.shortcut_help_open {
            return;
        }

        let focus_pending = std::mem::take(&mut self.shortcut_help_focus_pending);
        let mut close_clicked = false;
        // egui::Modal (vs. egui::Window) calls `set_modal_layer`, which makes
        // Tab navigation skip widgets in the layers below — i.e. it actually
        // traps keyboard focus inside the dialog. Window does not do this.
        let modal_response =
            egui::Modal::new(egui::Id::new("shortcut_help_modal")).show(ctx, |ui| {
                ui.set_max_width(520.0);
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
                egui::Grid::new("shortcuts_app")
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("General").strong());
                        ui.end_row();
                        // Rendered from the same table `handle_keyboard_shortcuts`
                        // dispatches, so the two cannot drift.
                        for (key, action) in shortcuts::help_rows() {
                            ui.label(RichText::new(key).monospace());
                            ui.label(action);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);

                egui::Grid::new("shortcuts_graph")
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
                            ui.label(action);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);

                egui::Grid::new("gestures_graph")
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Graph (mouse)").strong());
                        ui.end_row();
                        for (gesture, action) in [
                            ("Drag", "Pan left / right through history"),
                            ("Shift + drag", "Zoom to bounding box (time & value)"),
                            ("Scroll wheel", "Zoom X axis centered on cursor"),
                            ("Double-click", "Reset to live follow + auto Y"),
                            ("Click (cursors on)", "Place cursor A / B at nearest point"),
                            ("Minimap drag", "Jump to time / resize viewport"),
                        ] {
                            ui.label(RichText::new(gesture).monospace());
                            ui.label(action);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Graph and Space shortcuts are disabled while any widget has keyboard \
                         focus — press Escape to release it.",
                    )
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
            });

        // Close on: (a) close button click, (b) Esc / backdrop click via
        // Modal::should_close, or (c) Ctrl+W (still consumed in
        // handle_keyboard_shortcuts so it works while focus is in the modal).
        if close_clicked || modal_response.should_close() {
            self.shortcut_help_open = false;
            // Defer focus restoration. On this frame `top_modal_layer` is
            // still set, and egui's `create_widget` will call
            // `surrender_focus` on every top-bar widget below the modal
            // layer on the *next* frame — including the `?` button — which
            // silently wipes any focus we set here. The deferred restore
            // fires once `top_modal_layer` has actually cleared, so the
            // target widget can keep the focus it's given.
            self.shortcut_help_restore_focus = self.shortcut_help_opener.take();
        }
    }
}
