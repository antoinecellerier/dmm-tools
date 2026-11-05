//! Helpers more than one of the GUI's test modules uses: an app before any
//! frame, a message handed to it from the acquisition thread, a key tapped
//! within one frame, and the pieces of a headless frame — running it, reading
//! its AccessKit tree, finding a widget's rect, clicking.

use super::connection::{ConnectedMeter, DmmMessage};
use super::{App, ConnectionState, Worker};
use crate::settings::Settings;
use egui::accesskit::{Node, NodeId};
use std::sync::mpsc;

/// The app on the default settings and the real clock, before any frame.
pub(crate) fn test_app() -> App {
    App::from_settings(Settings::default(), dmm_lib::Clock::real())
}

/// The app connected to a meter offering `commands`, a 1.234 reading in
/// `unit` on screen. The connected state is set by hand: no acquisition
/// thread runs.
pub(crate) fn connected_app(commands: &[&str], unit: &'static str) -> App {
    let settings = Settings {
        auto_connect: false,
        ..Settings::default()
    };
    let mut app = App::from_settings(settings, dmm_lib::Clock::real());
    app.connection.state = ConnectionState::Connected;
    app.connection.meter = Some(ConnectedMeter {
        supported_commands: commands.iter().map(|c| c.to_string()).collect(),
        ..ConnectedMeter::test_fixture(None)
    });
    app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
        dmm_lib::measurement::MeasuredValue::Normal(1.234),
        unit,
        dmm_lib::flags::StatusFlags::default(),
    ));
    app
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

/// One frame on `ctx` from `raw`, as `Context::run_ui` runs it, with the
/// frame's texture changes dropped: nothing here paints, and a
/// `TexturesDelta` dropped without being applied fails a debug assertion.
pub(crate) fn headless_run(
    ctx: &egui::Context,
    raw: egui::RawInput,
    draw: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut out = ctx.run_ui(raw, draw);
    out.textures_delta.clear();
    out
}

/// [`headless_run`] of a window `screen` covers, at input time `time`, with
/// `events` delivered to it; `None` leaves the screen or time to egui.
pub(crate) fn headless_frame(
    ctx: &egui::Context,
    screen: Option<egui::Rect>,
    time: Option<f64>,
    events: Vec<egui::Event>,
    draw: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let raw = egui::RawInput {
        screen_rect: screen,
        time,
        events,
        ..Default::default()
    };
    headless_run(ctx, raw, draw)
}

/// A frame's AccessKit tree and the node it names as focused: empty and
/// `None` unless the context enabled AccessKit. With nothing focused the
/// focus is the root, which carries no bounds.
pub(crate) fn a11y(out: &egui::FullOutput) -> (Vec<(NodeId, Node)>, Option<NodeId>) {
    out.platform_output
        .accesskit_update
        .as_ref()
        .map(|update| (update.nodes.clone(), Some(update.focus)))
        .unwrap_or_default()
}

/// Where the first node `pred` accepts was drawn, if it has bounds.
pub(crate) fn node_rect(
    nodes: &[(NodeId, Node)],
    pred: impl Fn(&Node) -> bool,
) -> Option<egui::Rect> {
    nodes
        .iter()
        .find(|(_, n)| pred(n))
        .and_then(|(_, n)| n.bounds())
        .map(to_rect)
}

/// Where the focused node was drawn; `None` when nothing is focused.
pub(crate) fn focused_rect(nodes: &[(NodeId, Node)], focus: Option<NodeId>) -> Option<egui::Rect> {
    let focus = focus?;
    nodes
        .iter()
        .find(|(id, _)| *id == focus)
        .and_then(|(_, n)| n.bounds())
        .map(to_rect)
}

/// An AccessKit rect in egui's points.
pub(crate) fn to_rect(b: egui::accesskit::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(b.x0 as f32, b.y0 as f32),
        egui::pos2(b.x1 as f32, b.y1 as f32),
    )
}

/// A click of `button` at `pos` as a mouse sends it, a frame each: the
/// pointer moves there, the button goes down, it comes up.
pub(crate) fn click_frames(pos: egui::Pos2, button: egui::PointerButton) -> [Vec<egui::Event>; 3] {
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let moved = egui::Event::PointerMoved(pos);
    [
        vec![moved.clone()],
        vec![moved.clone(), press(true)],
        vec![moved, press(false)],
    ]
}
