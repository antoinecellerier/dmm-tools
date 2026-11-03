//! The "What's New" release-notes viewport: a separate OS window rendering
//! the changelog, opened from the version label or on the first launch after
//! an upgrade, and closed back onto the widget that opened it.

use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::App;

impl App {
    pub(super) fn open_whats_new(&mut self) {
        self.whats_new.open = true;
        self.settings.last_seen_version = Some(env!("CARGO_PKG_VERSION").to_string());
    }

    pub(super) fn show_whats_new(&mut self, ctx: &egui::Context) {
        // The viewport callback signals close via an AtomicBool.
        if self.whats_new.closed.swap(false, Ordering::Relaxed) {
            self.whats_new.open = false;
            // Restore focus to the widget that opened the viewport so the
            // user's Tab position is preserved across the modal round-trip.
            if let Some(opener) = self.whats_new.opener.take() {
                ctx.memory_mut(|m| m.request_focus(opener));
            }
        }

        if !self.whats_new.open {
            return;
        }

        let version = env!("CARGO_PKG_VERSION");
        // The title is the only text of the window's own: the notes are the
        // changelog, in English. Built here in the root each frame, it
        // follows the app's language, so the window's closure needs no
        // localizer.
        let title = if version.contains("-dev") {
            dmm_shared::tr!(self.l10n, "whats-new-title-unreleased").into_owned()
        } else {
            dmm_shared::tr!(self.l10n, "whats-new-title", version = version).into_owned()
        };

        let closed = Arc::clone(&self.whats_new.closed);
        let cache = Arc::clone(&self.whats_new.cache);
        let viewport_id = egui::ViewportId::from_hash_of("whats_new");
        let viewport_builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([520.0, 480.0]);

        ctx.show_viewport_deferred(viewport_id, viewport_builder, move |ui, _class| {
            use egui::{Key, Modifiers};
            let ctx = ui.ctx().clone();
            // The window is the changelog, which is English whatever the
            // app speaks: a screen reader should read it as English.
            ctx.accesskit_node_builder(egui::accesskit_root_id(), |node| {
                node.set_language(dmm_shared::l10n::EN_US.id);
            });
            let close_requested = ctx.input(|i| i.viewport().close_requested())
                || ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Escape)
                        || i.consume_key(Modifiers::COMMAND, Key::W)
                });
            if close_requested {
                closed.store(true, Ordering::Relaxed);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                // Only the root reads `closed`, and with no reading coming in
                // (paused, disconnected) it would wait for the next input.
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut cache = cache
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    crate::changelog::show_changelog(ui, &mut cache, version);
                });
            });
        });
    }
}
