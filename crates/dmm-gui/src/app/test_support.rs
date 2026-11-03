//! Helpers more than one of the GUI's test modules uses: an app before any
//! frame, a message handed to it from the acquisition thread, and a key
//! tapped within one frame.

use super::connection::DmmMessage;
use super::{App, Worker};
use crate::settings::Settings;
use std::sync::mpsc;

/// The app on the default settings and the real clock, before any frame.
pub(crate) fn test_app() -> App {
    App::from_settings(Settings::default(), dmm_lib::Clock::real())
}

impl App {
    /// Hand the app one message from the acquisition thread. The sender is
    /// never dropped, so neither this drain nor a later frame's takes the
    /// channel for one whose thread has died.
    pub(crate) fn deliver(&mut self, msg: DmmMessage) {
        let (tx, rx) = mpsc::channel();
        tx.send(msg).expect("the channel is open");
        std::mem::forget(tx);
        self.connection.worker = Some(Worker::reading(rx));
        self.drain_messages();
    }
}

/// `key` pressed and released within one frame.
pub(crate) fn key_tap(key: egui::Key) -> Vec<egui::Event> {
    key_tap_with(key, egui::Modifiers::NONE)
}

/// `key` pressed and released within one frame, with `modifiers` held.
pub(crate) fn key_tap_with(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}
