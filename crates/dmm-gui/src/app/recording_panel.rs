//! The sample buffer's UI: the Record / Export / Discard row, the recording's
//! sample log or the line saying Export… saves the graph's samples, the prompt
//! shown before a new capture or a Discard would lose unexported samples, and
//! the drag-resizable split between the graph and the recording panel.

use eframe::egui::{self, FocusDirection, Key, Modifiers, RichText, Ui};
use log::info;
use std::time::Instant;

use super::export::{ExportFormat, NO_WIRE_FORMAT};
use super::{App, ConnectionState, DEFAULT_RECORDING_HEIGHT};
use crate::a11y::ResponseA11yExt;
use crate::recording::BufferRole;

/// The arrow segment of the Export… split button (U+23F7, in egui's icon
/// font like the `⏵` its submenus use).
const EXPORT_MENU_ARROW: &str = "\u{23F7}";

/// The Export… segments' hover text, label first, then arrow: what they save
/// for the buffer's role.
fn export_tooltips(role: BufferRole) -> (&'static str, &'static str) {
    match role {
        BufferRole::Recording => (
            "Save the recording as a CSV file (Ctrl+E)",
            "Save the recording as a CSV, JSON or replay file",
        ),
        BufferRole::History => (
            "Save the samples from the graph as a CSV file (Ctrl+E)",
            "Save the samples from the graph as a CSV, JSON or replay file",
        ),
    }
}

/// `n` samples, in words.
fn sample_count(n: usize) -> String {
    format!("{n} {}", noun(n))
}

/// The line under the Record / Export row while nothing is recorded: what
/// Export saves, and what Record is for — the graph's samples go with its
/// restarts, a recording does not.
fn history_hint(samples: usize) -> String {
    format!(
        "No recording. Export saves {} from the graph. \
         Record to capture across mode changes.",
        sample_count(samples)
    )
}

/// The discard prompt's heading and text, for `unexported` samples and,
/// with `markers`, marker changes no file holds.
fn discard_prompt(action: DiscardFor, unexported: usize, markers: bool) -> (&'static str, String) {
    const MARKER_CHANGES: &str = "marker changes made since the last export";
    let lost = match (unexported, markers) {
        (0, _) => MARKER_CHANGES.to_string(),
        (n, false) => format!("{n} unexported {}", noun(n)),
        (n, true) => format!("{n} unexported {} and {MARKER_CHANGES}", noun(n)),
    };
    let text = match action {
        DiscardFor::Record => format!("Starting a new recording will discard {lost}."),
        DiscardFor::Discard => format!("Discarding the recording will lose {lost}."),
    };
    if unexported == 0 {
        (
            "Discard unexported markers?",
            format!("{text} Markers still on the graph stay there, but in no recording."),
        )
    } else {
        ("Discard unexported samples?", text)
    }
}

fn noun(n: usize) -> &'static str {
    if n == 1 { "sample" } else { "samples" }
}

/// Smallest height the graph + recording split is squeezed into; below it the
/// column that holds the split scrolls instead of shrinking it further.
///
/// The sum of the floors the split and its two halves already enforce: the
/// graph's toolbar row (~28), the plot's 60 px floor, the minimap strip
/// (`MINIMAP_HEIGHT` 60 + a 14 px label + 8 px of stroke margin), the drag
/// divider and the recording panel's 40 px floor, plus the item spacing
/// between them. `MINIMAP_HEIGHT` is private to `crate::graph`, so the sum is
/// written out here rather than derived.
pub(super) const MIN_SPLIT_HEIGHT: f32 = 240.0;

/// What the discard prompt is asking to go ahead with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscardFor {
    /// Record was pressed: a new recording replaces this one.
    Record,
    /// Discard was pressed: the buffer goes back to following the graph.
    Discard,
}

/// The recording panel's own state: how tall the user dragged it, and the
/// discard prompt that guards an unexported capture.
pub(super) struct RecordingPanel {
    /// User-resizable recording panel height.
    height: f32,
    /// Record or Discard was pressed while the recording held unexported
    /// samples; waiting for the user to confirm losing them.
    pending_discard: Option<DiscardFor>,
    confirm_discard_focus_pending: bool,
    /// The Export… menu opened this frame; its first entry still has to be
    /// given the focus — see `show_export_menu`.
    export_menu_focus_pending: bool,
}

impl Default for RecordingPanel {
    fn default() -> Self {
        Self {
            height: DEFAULT_RECORDING_HEIGHT,
            pending_discard: None,
            confirm_discard_focus_pending: false,
            export_menu_focus_pending: false,
        }
    }
}

impl App {
    /// Whether no discard prompt is waiting on the user.
    pub(super) fn recording_panel_idle(&self) -> bool {
        self.recording_panel.pending_discard.is_none()
    }

    /// Start or stop recording.
    ///
    /// Starting clears the buffer, so if it holds samples that were never
    /// exported this asks first — a second Record press (or a mistyped
    /// Ctrl+R) used to destroy an unexported capture with no prompt, no
    /// toast, and nothing in the log.
    pub(super) fn toggle_recording(&mut self) {
        if !self.recording.active && self.recording.needs_discard_prompt(&self.markers) {
            self.ask_before_discarding(DiscardFor::Record);
            return;
        }
        self.apply_recording_toggle();
    }

    /// Drop a stopped recording, asking first if it holds samples that were
    /// never exported.
    fn discard_recording(&mut self) {
        if self.recording.needs_discard_prompt(&self.markers) {
            self.ask_before_discarding(DiscardFor::Discard);
            return;
        }
        self.apply_discard();
    }

    fn ask_before_discarding(&mut self, action: DiscardFor) {
        self.recording_panel.pending_discard = Some(action);
        self.recording_panel.confirm_discard_focus_pending = true;
    }

    /// Go ahead with what the prompt was asked about.
    fn apply_pending_discard(&mut self, action: DiscardFor) {
        self.recording_panel.pending_discard = None;
        match action {
            DiscardFor::Record => self.apply_recording_toggle(),
            DiscardFor::Discard => self.apply_discard(),
        }
    }

    fn apply_discard(&mut self) {
        let count = self.recording.samples.len();
        self.recording.discard();
        info!("discarded a recording of {count} samples");
        self.toast = Some(("Recording discarded".to_string(), false, Instant::now()));
    }

    /// Flip the recording state, remembering which meter the samples came
    /// from.
    ///
    /// The device has to be captured here rather than read back at export
    /// time: the Settings selection can change while a recording is held in
    /// the buffer (it only schedules a reconnect, and neither connect nor
    /// disconnect clears the samples), so reading it later labelled the file
    /// with whatever meter happened to be picked last.
    fn apply_recording_toggle(&mut self) {
        self.recording.toggle(self.clock.now());
        if self.recording.active {
            // The meter picked, or the one detection found; under Auto-detect
            // with nothing connected there is no meter to name, and the export
            // falls back to its own placeholder.
            self.capture_layout.device = self.active_device().map(|d| d.display_name);
            // Only a meter's frames can be replayed, so the mock names no
            // device here and the export offers no replay file for it.
            self.capture_layout.device_id = self
                .active_device()
                .filter(|d| d.requires_hardware)
                .map(|d| d.id);
            // Only from a live connection: disconnected, `stability` is the
            // Verified default `disconnect()` restored, and latching that
            // marked a UT181A connected after Record as a verified protocol.
            self.capture_layout.experimental = (self.connection.state
                != ConnectionState::Disconnected)
                .then(|| !self.connection.stability.is_verified());
            // Empty while disconnected for the same reason; the export then
            // falls back to whatever link answers during the recording.
            self.capture_layout.link = self.connection.link;
            self.capture_layout.aux_slots = self.capture_layout.device_aux_slots;
            // The transform's Raw sub-value needs a fixed column of its own,
            // after the meter's — see `extra_slots`.
            self.capture_layout.extra_slots = self.transform.extra_aux_count();
        }
    }

    /// Confirmation shown when starting a recording, or discarding one,
    /// would lose samples that have not been exported.
    pub(super) fn show_discard_confirmation(&mut self, ctx: &egui::Context) {
        let Some(action) = self.recording_panel.pending_discard else {
            return;
        };
        let unexported = self.recording.unexported_count();
        let markers = self.recording.has_unsaved_markers(&self.markers);
        if unexported == 0 && !markers {
            // An export completed while the prompt was up — nothing left to
            // warn about.
            self.apply_pending_discard(action);
            return;
        }

        let focus_pending = std::mem::take(&mut self.recording_panel.confirm_discard_focus_pending);
        let mut discard = false;
        let mut cancel = false;
        // egui::Modal rather than Window: it sets the modal layer, which traps
        // keyboard focus inside the dialog.
        let modal = egui::Modal::new(egui::Id::new("confirm_discard_modal")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            let (heading, text) = discard_prompt(action, unexported, markers);
            ui.heading(heading);
            ui.add_space(4.0);
            ui.label(text);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Cancel takes focus: Enter and Space then default to the
                // non-destructive choice.
                let cancel_btn = ui.button("Cancel");
                if focus_pending {
                    cancel_btn.request_focus();
                }
                if cancel_btn.clicked() {
                    cancel = true;
                }
                let confirm = match action {
                    DiscardFor::Record => "Discard and record",
                    DiscardFor::Discard => "Discard recording",
                };
                if ui.button(confirm).clicked() {
                    discard = true;
                }
            });
        });

        if discard {
            self.apply_pending_discard(action);
        } else if cancel || modal.should_close() {
            self.recording_panel.pending_discard = None;
        }
    }

    fn show_recording_section(&mut self, ui: &mut Ui, compact: bool) {
        let (btn_label, btn_tooltip) = if self.recording.active {
            ("\u{25A0} Stop", "Stop recording (Ctrl+R)")
        } else {
            (
                "\u{25CF} Record",
                "Start writing live samples to the recording buffer (Ctrl+R)",
            )
        };

        ui.horizontal(|ui| {
            if ui.button(btn_label).on_hover_text(btn_tooltip).clicked() {
                self.toggle_recording();
            }
            self.show_export_button(ui);
            let count = self.recording.samples.len();
            let recorded = self.recording.role() == BufferRole::Recording;
            if recorded && !self.recording.active && count > 0 {
                let discard = ui.button("Discard").on_hover_text(
                    "Discard the recording; Export then saves samples from the graph",
                );
                if discard.clicked() {
                    self.discard_recording();
                }
            }
            if self.recording.active {
                let status = format!(
                    "{} | {:.0}s",
                    sample_count(count),
                    self.recording.duration_secs(self.clock.now())
                );
                if self.recording.is_full() {
                    let warn = self
                        .settings
                        .theme_colors(ui.visuals().dark_mode)
                        .recording_full_warning();
                    ui.label(RichText::new(format!("{status} (buffer full)")).color(warn));
                } else {
                    ui.label(status);
                }
            } else if recorded && count > 0 {
                ui.label(sample_count(count));
            }
        });

        if self.recording.role() == BufferRole::History && !self.recording.samples.is_empty() {
            ui.add(
                egui::Label::new(
                    RichText::new(history_hint(self.recording.samples.len()))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .wrap(),
            );
        }
        self.show_log(ui, compact);
    }

    /// The Export… split button: the label saves a CSV in one click, the
    /// arrow beside it drops a menu that also offers JSON and a replay file.
    /// The format is settled here, before the save dialog opens — see
    /// `ExportFormat` for why the dialog cannot be the one to ask.
    fn show_export_button(&mut self, ui: &mut Ui) {
        let (label_tooltip, arrow_tooltip) = export_tooltips(self.recording.role());
        ui.scope(|ui| {
            // The two segments touch, and only the outer corners are round,
            // so they read as one button.
            ui.spacing_mut().item_spacing.x = 0.0;
            let radius = ui.visuals().widgets.inactive.corner_radius;
            let main = ui
                .add(
                    egui::Button::new("Export\u{2026}").corner_radius(egui::CornerRadius {
                        ne: 0,
                        se: 0,
                        ..radius
                    }),
                )
                .on_hover_text(label_tooltip);
            if main.clicked() {
                self.export_recording(ExportFormat::Csv);
            }
            let arrow = ui
                .add(
                    egui::Button::new(EXPORT_MENU_ARROW).corner_radius(egui::CornerRadius {
                        nw: 0,
                        sw: 0,
                        ..radius
                    }),
                )
                .on_hover_text(arrow_tooltip)
                .a11y_label("Export file type");
            self.show_export_menu(ui, &arrow);
        });
    }

    /// The menu under the Export… arrow. Keyboard handling follows the
    /// readout lists in `display.rs`: focus lands on the first entry as the
    /// menu opens, the arrows move it, Enter picks, and Tab or Esc leave
    /// with the focus back on the arrow.
    fn show_export_menu(&mut self, ui: &mut Ui, arrow: &egui::Response) {
        let ctx = ui.ctx().clone();
        let popup_id = arrow.id.with("export_menu");
        // Read before `show`, which is where the arrow's click toggles it:
        // "the menu was open as this frame began".
        let was_open = egui::Popup::is_id_open(&ctx, popup_id);

        // Handled before the menu is drawn so this frame already shows it
        // closed; `move_focus(None)` cancels the jump egui queued from Tab.
        if was_open {
            let leave = ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Tab)
                    | i.consume_key(Modifiers::SHIFT, Key::Tab)
                    | i.consume_key(Modifiers::NONE, Key::Escape)
            });
            if leave {
                egui::Popup::close_id(&ctx, popup_id);
                ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
            }
        }

        let replay_possible = self.replay_device_id().is_some();
        // The click that opens the menu counts as a click outside the entry
        // that takes the focus, so egui surrenders that focus the next time
        // the entry is read back (`Context::get_response`) — which the page
        // scroller does every frame, to keep the focused widget in view. So
        // the request is repeated on the following frame, once the click is
        // gone; the opening frame is an invisible sizing pass anyway.
        let focus_again = std::mem::take(&mut self.recording_panel.export_menu_focus_pending);
        let mut picked = None;
        egui::Popup::menu(arrow).id(popup_id).show(|ui| {
            let csv = ui.button("CSV\u{2026}");
            if !was_open || focus_again {
                csv.request_focus();
            }
            // Picks count from the next frame on: the Enter that opened the
            // menu from the arrow is still "pressed" this frame, and egui
            // reads a pressed Enter on a focused button as a click — the
            // first entry took the focus just above.
            if was_open && csv.clicked() {
                picked = Some(ExportFormat::Csv);
            }
            let json = ui
                .button("JSON\u{2026}")
                .on_hover_text("One object per line, as dmm-cli read --format json prints");
            if was_open && json.clicked() {
                picked = Some(ExportFormat::Json);
            }
            let replay = ui
                .add_enabled(replay_possible, egui::Button::new("Replay\u{2026}"))
                .on_hover_text("A file dmm-gui --replay plays back as the meter")
                .on_disabled_hover_text(NO_WIRE_FORMAT);
            if was_open && replay.clicked() {
                picked = Some(ExportFormat::Replay);
            }
            // Menu entries have no frame, so egui's focus styling is a fill
            // too faint to find; ring the focused one.
            crate::a11y::paint_focus_ring(ui, &csv);
            crate::a11y::paint_focus_ring(ui, &json);
            crate::a11y::paint_focus_ring(ui, &replay);
            // A disabled entry cannot take the focus, so it is not a stop
            // for the arrows either: Down would strand the focus on it.
            let entries: Vec<_> = [csv, json, replay]
                .into_iter()
                .filter(|entry| entry.enabled())
                .collect();
            crate::display::navigate_choice_entries(ui.ctx(), &entries);
        });

        if let Some(format) = picked {
            // Enter and Space "click" without a pointer click, which is the
            // only thing a menu popup closes on by itself.
            egui::Popup::close_id(&ctx, popup_id);
            self.export_recording(format);
        }
        let is_open = egui::Popup::is_id_open(&ctx, popup_id);
        // Opened this frame: ask for the focus again next frame.
        if !was_open && is_open {
            self.recording_panel.export_menu_focus_pending = true;
        }
        // A click outside that closed the menu may have landed on another
        // widget, which took the focus as it was drawn; that click is the
        // user's choice of focus. Every other way out returns to the arrow.
        let clicked_away = picked.is_none() && ctx.input(|i| i.pointer.any_click());
        if was_open && !is_open && !clicked_away {
            ctx.memory_mut(|m| m.request_focus(arrow.id));
        }
    }

    /// The graph + recording split as the wide layout's centre column, inside
    /// the page scroller that keeps a short window from cropping it.
    ///
    /// The split sizes itself from `ui.available_height()`, which inside a
    /// scroll area is the viewport rather than what is left of the window, so
    /// the height is allocated explicitly: the rest of the viewport while the
    /// column fits — content then equals viewport and no scrollbar appears —
    /// and [`MIN_SPLIT_HEIGHT`] once it doesn't, so the graph stays usable and
    /// the column scrolls instead. `floor()` keeps a fractional viewport from
    /// spilling the content a hair past it and raising a scrollbar on a window
    /// that fits.
    ///
    /// Returns the scroller's output so tests can see whether it had to scroll.
    pub(super) fn show_graph_column(
        &mut self,
        ui: &mut Ui,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        egui::ScrollArea::vertical()
            .id_salt("graph_column")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let height = ui.available_height().max(MIN_SPLIT_HEIGHT).floor();
                ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                    self.show_graph_recording_split(ui, false);
                });
                crate::a11y::scroll_to_focus(ui);
            })
    }

    /// Render the graph+recording area with a resizable drag separator between them.
    pub(super) fn show_graph_recording_split(&mut self, ui: &mut Ui, compact: bool) {
        // Built before `self.graph` is borrowed mutably below: the palette
        // reads `self.settings`, which the borrow checker would otherwise
        // see as overlapping.
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        if self.settings.show_graph && self.settings.show_recording {
            let total = ui.available_height();
            let graph_height = (total - self.recording_panel.height).max(80.0);

            ui.allocate_ui(egui::vec2(ui.available_width(), graph_height), |ui| {
                self.graph.show(ui, &tc, &self.markers);
            });
            self.take_graph_marker_actions();

            let sep = ui.separator();
            let sep_id = ui.id().with("rec_resize");
            let sep_response = ui
                .interact(
                    sep.rect.expand2(egui::vec2(0.0, 4.0)),
                    sep_id,
                    egui::Sense::drag(),
                )
                .a11y_label("Resize recording panel (Up/Down to adjust)");
            if sep_response.dragged() {
                self.recording_panel.height = (self.recording_panel.height
                    - sep_response.drag_delta().y)
                    .clamp(40.0, (total - 80.0).max(40.0));
            }
            // Keyboard resize when focused: Up moves the divider up
            // (grows the recording panel), Down moves it down. Matches
            // mouse-drag direction.
            let delta = crate::a11y::arrow_resize(
                ui.ctx(),
                sep_id,
                crate::a11y::ResizeAxis::Vertical,
                20.0,
            );
            if delta != 0.0 {
                self.recording_panel.height =
                    (self.recording_panel.height + delta).clamp(40.0, (total - 80.0).max(40.0));
            }
            crate::a11y::paint_focus_ring(ui, &sep_response);
            if sep_response.hovered() || sep_response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }

            self.show_recording_section(ui, compact);
        } else if self.settings.show_graph {
            self.graph.show(ui, &tc, &self.markers);
            self.take_graph_marker_actions();
        } else if self.settings.show_recording {
            self.show_recording_section(ui, compact);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use eframe::egui::{Pos2, Rect, vec2};

    /// The recording row in a headless window, driven a frame at a time.
    struct MenuRun {
        app: App,
        ctx: egui::Context,
        seconds: f64,
        /// The AccessKit tree the last frame produced, and the node it named
        /// as focused. Rects are read from here rather than from
        /// `Context::read_response`: that one answers from the pass state
        /// egui swaps out at the end of a frame, so between frames it hands
        /// back the rect from two frames ago, and it surrenders the focus of
        /// the widget it is asked about when a click landed elsewhere the
        /// same frame (`Context::get_response`).
        tree: Vec<(egui::accesskit::NodeId, egui::accesskit::Node)>,
        focus: Option<egui::accesskit::NodeId>,
        /// The window's width.
        width: f32,
    }

    impl MenuRun {
        fn new() -> Self {
            let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
            // A meter's recording, so the Replay… entry is enabled.
            app.capture_layout.device_id = Some("ut61eplus");
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            Self {
                app,
                ctx,
                seconds: 0.0,
                tree: Vec::new(),
                focus: None,
                width: 800.0,
            }
        }

        /// One frame; keeps the AccessKit tree it produced.
        fn frame(&mut self, secs: f64, events: Vec<egui::Event>) {
            self.seconds += secs;
            let app = &mut self.app;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(self.width, 400.0))),
                    events,
                    time: Some(self.seconds),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx().clone();
                    app.handle_keyboard_shortcuts(&ctx);
                    // The row is drawn inside the page scroller, which reads
                    // the focused widget back to bring it into view — and that
                    // read is what takes the focus off a menu entry the
                    // opening click landed outside of.
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        app.show_recording_section(ui, false);
                        crate::a11y::scroll_to_focus(ui);
                    });
                    let ctx = ui.ctx().clone();
                    app.show_discard_confirmation(&ctx);
                },
            );
            out.textures_delta.clear();
            if let Some(update) = out.platform_output.accesskit_update {
                self.tree = update.nodes;
                self.focus = Some(update.focus);
            }
        }

        /// Move, press and release on successive frames, as a mouse does.
        fn click(&mut self, pos: Pos2) {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            self.frame(1.0, vec![egui::Event::PointerMoved(pos)]);
            self.frame(1.0, vec![egui::Event::PointerMoved(pos), button(true)]);
            self.frame(0.1, vec![egui::Event::PointerMoved(pos), button(false)]);
            self.frame(1.0, vec![]);
        }

        /// A key pressed and released within one frame, then a settling frame.
        fn key(&mut self, key: Key) {
            let events = [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                })
                .collect();
            self.frame(1.0, events);
            self.frame(1.0, vec![]);
        }

        /// Where the widget AccessKit labels `label` was drawn last frame.
        fn node_rect(&self, label: &str) -> Rect {
            self.tree
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .and_then(|(_, n)| n.bounds())
                .map(to_rect)
                .unwrap_or_else(|| panic!("no {label:?} widget in the row"))
        }

        /// Whether a widget AccessKit labels `label` was drawn last frame.
        fn shows_widget(&self, label: &str) -> bool {
            self.tree.iter().any(|(_, n)| n.label() == Some(label))
        }

        /// Whether a label reading `text` was drawn last frame.
        fn shows_text(&self, text: &str) -> bool {
            self.tree.iter().any(|(_, n)| n.value() == Some(text))
        }

        /// Where the widget holding the keyboard focus was drawn last frame.
        ///
        /// The tree's root carries no bounds, and that is what an unfocused
        /// frame names, so nothing focused reads as `None`.
        fn focused_rect(&self) -> Option<Rect> {
            let focus = self.focus?;
            self.tree
                .iter()
                .find(|(id, _)| *id == focus)
                .and_then(|(_, n)| n.bounds())
                .map(to_rect)
        }
    }

    fn to_rect(b: egui::accesskit::Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(b.x0 as f32, b.y0 as f32),
            Pos2::new(b.x1 as f32, b.y1 as f32),
        )
    }

    /// The menu's keyboard contract — the readout lists' — end to end:
    /// focus lands on CSV… as the arrow opens it, Down walks it through
    /// JSON… to Replay…, and Esc closes the menu with the focus back on the
    /// arrow.
    #[test]
    fn the_export_menu_is_keyboard_reachable() {
        let mut run = MenuRun::new();
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        let arrow = run.node_rect("Export file type");

        run.click(arrow.center());
        let csv = run.focused_rect().expect("the first entry takes the focus");
        assert!(csv.top() >= arrow.bottom(), "focus is in the menu: {csv:?}");

        let mut above = csv;
        for entry in ["JSON\u{2026}", "Replay\u{2026}"] {
            run.key(Key::ArrowDown);
            let focused = run
                .focused_rect()
                .expect("Down keeps the focus in the menu");
            assert!(focused.top() >= above.bottom(), "Down moved to {entry}");
            above = focused;
        }
        // And back up the way it came.
        run.key(Key::ArrowUp);
        let back_up = run.focused_rect().expect("Up keeps the focus in the menu");
        assert!(back_up.bottom() <= above.top(), "Up moved to JSON\u{2026}");

        run.key(Key::Escape);
        let back = run.focused_rect().expect("Esc leaves the focus somewhere");
        assert!(
            (back.center() - arrow.center()).length() < 1.0,
            "Esc returned the focus to the arrow, not {back:?}"
        );
        assert!(!egui::Popup::is_any_open(&run.ctx), "Esc closed the menu");
    }

    /// Enter on the arrow opens the menu and lands on the first entry without
    /// picking it: egui reads a pressed Enter on a focused button as a click,
    /// and the entry takes the focus in that same frame.
    #[test]
    fn enter_opens_the_menu_without_picking_an_entry() {
        let mut run = MenuRun::new();
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        let arrow = run.node_rect("Export file type");

        // Record, Export… and then the arrow.
        for _ in 0..3 {
            run.key(Key::Tab);
        }
        let focused = run.focused_rect().expect("Tab reached the arrow");
        assert!(
            (focused.center() - arrow.center()).length() < 1.0,
            "Tab stopped on the arrow, not {focused:?}"
        );

        run.key(Key::Enter);
        let csv = run.focused_rect().expect("the first entry takes the focus");
        assert!(csv.top() >= arrow.bottom(), "focus is in the menu: {csv:?}");
        // Picking CSV… on an empty buffer reports it; nothing was picked.
        assert!(
            run.app.toast.is_none(),
            "Enter did not pick the first entry"
        );
    }

    /// With no wire format to write, Replay… is disabled and Down stops at
    /// JSON… rather than stranding the focus on an entry that cannot take it.
    #[test]
    fn down_skips_a_disabled_entry() {
        let mut run = MenuRun::new();
        run.app.capture_layout.device_id = None;
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        let arrow = run.node_rect("Export file type");

        run.click(arrow.center());
        run.key(Key::ArrowDown);
        let json = run
            .focused_rect()
            .expect("Down keeps the focus in the menu");
        run.key(Key::ArrowDown);
        let after = run
            .focused_rect()
            .expect("Down keeps the focus in the menu");
        assert_eq!(after, json, "Down stayed on JSON…");
    }

    /// `Fonts::has_glyph` is a false negative for the icon font — see
    /// `.claude/rules/gui.md` — so compare the atlas rect with the
    /// replacement character's, as the toast's glyph test does.
    #[test]
    fn the_menu_arrow_has_a_glyph() {
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
            assert_ne!(
                atlas_rect(ui, EXPORT_MENU_ARROW),
                atlas_rect(ui, "\u{25FB}"),
                "the arrow draws as the replacement box"
            );
        });
        out.textures_delta.clear();
    }

    /// A 1.234 V reading, as the drain hands the buffer one.
    fn reading() -> dmm_lib::measurement::Measurement {
        dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        )
    }

    /// The line naming what Export… saves shows only while the graph's
    /// samples are what it would save; a recording shows its count and log.
    #[test]
    fn the_hint_shows_only_while_export_saves_the_graph() {
        let mut run = MenuRun::new();
        run.frame(1.0, vec![]);
        assert!(
            !run.tree
                .iter()
                .any(|(_, n)| n.value().is_some_and(|v| v.starts_with("No recording"))),
            "nothing to save, nothing to say"
        );

        for _ in 0..3 {
            let wall_clock = run.app.wall_clock;
            run.app.recording.push(&reading(), &wall_clock, 0);
        }
        run.frame(1.0, vec![]);
        assert!(run.shows_text(&history_hint(3)), "{:?}", history_hint(3));
        assert!(
            !run.shows_text("3 samples"),
            "the history has no counter of its own"
        );

        run.app.toggle_recording();
        let wall_clock = run.app.wall_clock;
        run.app.recording.push(&reading(), &wall_clock, 0);
        run.app.toggle_recording();
        run.frame(1.0, vec![]);
        assert!(!run.shows_text(&history_hint(1)));
        assert!(run.shows_text("1 sample"));
    }

    #[test]
    fn the_hint_counts_one_sample_as_one() {
        assert_eq!(
            history_hint(1),
            "No recording. Export saves 1 sample from the graph. \
             Record to capture across mode changes."
        );
        assert!(
            history_hint(1234).starts_with("No recording. Export saves 1234 samples from"),
            "{}",
            history_hint(1234)
        );
    }

    /// The hover text names what the buttons save.
    #[test]
    fn the_export_tooltips_follow_the_buffer() {
        let (label, arrow) = export_tooltips(BufferRole::History);
        assert!(label.contains("from the graph") && arrow.contains("from the graph"));
        let (label, arrow) = export_tooltips(BufferRole::Recording);
        assert!(label.contains("the recording") && arrow.contains("the recording"));
    }

    /// A stopped recording of `samples` readings, drawn once.
    fn run_with_stopped_recording(samples: usize) -> MenuRun {
        let mut run = MenuRun::new();
        run.app.toggle_recording();
        for _ in 0..samples {
            let wall_clock = run.app.wall_clock;
            run.app.recording.push(&reading(), &wall_clock, 0);
        }
        run.app.toggle_recording();
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        run
    }

    /// Discard is offered only for a recording that has stopped and holds
    /// something: the history is the graph's, and a running recording is
    /// stopped first.
    #[test]
    fn discard_shows_only_for_a_stopped_recording() {
        let mut run = MenuRun::new();
        let wall_clock = run.app.wall_clock;
        run.app.recording.push(&reading(), &wall_clock, 0);
        run.frame(1.0, vec![]);
        assert!(!run.shows_widget("Discard"), "not for the history");

        run.app.toggle_recording();
        run.app.recording.push(&reading(), &wall_clock, 0);
        run.frame(1.0, vec![]);
        assert!(!run.shows_widget("Discard"), "not while recording");

        run.app.toggle_recording();
        run.frame(1.0, vec![]);
        assert!(run.shows_widget("Discard"));
    }

    /// Discarding samples that reached no file asks first, as Record does;
    /// Cancel keeps them, confirming hands the buffer back to the graph.
    #[test]
    fn discarding_an_unexported_recording_asks_first() {
        let mut run = run_with_stopped_recording(2);
        run.click(run.node_rect("Discard").center());
        assert!(
            run.shows_text("Discarding the recording will lose 2 unexported samples."),
            "the prompt says what is lost"
        );
        run.click(run.node_rect("Cancel").center());
        assert!(!run.shows_widget("Discard recording"), "the prompt closed");
        assert_eq!(run.app.recording.samples.len(), 2, "Cancel keeps them");

        run.click(run.node_rect("Discard").center());
        run.click(run.node_rect("Discard recording").center());
        assert_eq!(run.app.recording.role(), BufferRole::History);
        assert!(run.app.recording.samples.is_empty());
        assert_eq!(
            run.app.toast.as_ref().map(|(text, _, _)| text.as_str()),
            Some("Recording discarded")
        );
        assert!(!run.shows_widget("Discard"), "nothing left to discard");
    }

    /// A recording already saved goes without a prompt.
    #[test]
    fn discarding_an_exported_recording_does_not_ask() {
        let mut run = run_with_stopped_recording(2);
        let epoch = run.app.recording.epoch();
        run.app.recording.mark_exported(epoch, 2, None);
        run.click(run.node_rect("Discard").center());
        assert!(!run.shows_widget("Discard recording"), "no prompt");
        assert_eq!(run.app.recording.role(), BufferRole::History);
        assert!(run.app.recording.samples.is_empty());
    }

    /// Mark the newest buffered sample, as `N` does with it on screen.
    fn mark_newest(run: &mut MenuRun) {
        run.app.last_measurement = run
            .app
            .recording
            .samples
            .back()
            .map(|s| s.measurement.clone());
        run.app.add_marker(false);
        run.frame(1.0, vec![]);
    }

    /// A saved recording whose markers changed since still asks, and says
    /// that the markers are what it would lose.
    #[test]
    fn discarding_unsaved_markers_asks_first() {
        let mut run = run_with_stopped_recording(2);
        let epoch = run.app.recording.epoch();
        run.app.recording.mark_exported(epoch, 2, None);
        mark_newest(&mut run);
        run.click(run.node_rect("Discard").center());
        assert!(run.shows_text("Discard unexported markers?"));
        assert!(run.shows_text(
            "Discarding the recording will lose marker changes made since the last export. \
             Markers still on the graph stay there, but in no recording."
        ));
        run.click(run.node_rect("Discard recording").center());
        assert_eq!(run.app.recording.role(), BufferRole::History);
    }

    /// The × at the end of a marker's row deletes that marker.
    #[test]
    fn the_cross_deletes_a_marker() {
        let mut run = run_with_stopped_recording(1);
        mark_newest(&mut run);
        assert!(run.shows_widget("Note for marker 1"));
        run.click(run.node_rect("Delete marker 1").center());
        assert!(run.app.markers.is_empty());
        assert!(!run.shows_widget("Note for marker 1"));
    }

    #[test]
    fn the_prompt_names_samples_and_markers_together() {
        assert_eq!(
            discard_prompt(DiscardFor::Record, 94, true),
            (
                "Discard unexported samples?",
                "Starting a new recording will discard 94 unexported samples and marker \
                 changes made since the last export."
                    .to_string()
            )
        );
    }

    /// Three minutes of readings a second apart, on the graph and in the
    /// buffer — a recording with `record` — and a marker on the one at
    /// `marked` seconds, out of the default one-minute window.
    fn run_with_a_marker_at(marked: u64, record: bool) -> MenuRun {
        run_with_markers(181, &[marked], record)
    }

    /// `count` readings a second apart, on the graph and in the buffer — a
    /// recording with `record` — with a marker on those at `marked` seconds.
    fn run_with_markers(count: u64, marked: &[u64], record: bool) -> MenuRun {
        let mut run = MenuRun::new();
        if record {
            run.app.toggle_recording();
        }
        let t0 = Instant::now();
        for i in 0..count {
            push_reading(&mut run, t0 + std::time::Duration::from_secs(i));
            if marked.contains(&i) {
                run.app.add_marker(false);
            }
        }
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        run
    }

    /// A reading taken at `at`, on the graph, in the buffer and on screen.
    fn push_reading(run: &mut MenuRun, at: Instant) {
        let mut m = reading();
        m.timestamp = at;
        let wall_clock = run.app.wall_clock;
        run.app.graph.push(1.234, m.timestamp, "DC V", "V", None);
        run.app.recording.push(&m, &wall_clock, 0);
        run.app.last_measurement = Some(m);
    }

    /// Whether the log drew the row of the sample taken at `at`.
    fn shows_sample(run: &MenuRun, at: Instant) -> bool {
        let sample = run
            .app
            .recording
            .samples
            .iter()
            .find(|s| s.measurement.timestamp == at)
            .expect("a buffered sample");
        let time = sample.wall_time.format("%H:%M:%S%.3f").to_string();
        run.tree
            .iter()
            .any(|(_, n)| n.value().is_some_and(|v| v.starts_with(&time)))
    }

    /// The label of the widget holding the keyboard focus.
    fn focused_label(run: &MenuRun) -> Option<String> {
        let focus = run.focus?;
        run.tree
            .iter()
            .find(|(id, _)| *id == focus)
            .and_then(|(_, n)| n.label().map(str::to_string))
    }

    /// The log holds the whole recording, not only its last rows: Tab
    /// reaches a marker far above the ones in view and brings it, with the
    /// samples around it, into view.
    #[test]
    fn tab_reaches_a_marker_at_the_start_of_a_long_recording() {
        let mut run = run_with_markers(2_000, &[10], true);
        let t0 = run.app.recording.samples[0].measurement.timestamp;
        let at = |i: u64| t0 + std::time::Duration::from_secs(i);
        assert!(shows_sample(&run, at(1_999)), "following the newest row");
        assert!(!shows_sample(&run, at(9)), "the start is out of view");
        for _ in 0..10 {
            if focused_label(&run).as_deref() == Some("Show marker 1 on the graph") {
                break;
            }
            run.key(Key::Tab);
        }
        assert_eq!(
            focused_label(&run).as_deref(),
            Some("Show marker 1 on the graph")
        );
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        assert!(shows_sample(&run, at(9)), "the rows around it are drawn");
        let tag = run.node_rect("Show marker 1 on the graph");
        assert!(
            tag.top() >= 0.0 && tag.bottom() <= 400.0,
            "in view: {tag:?}"
        );
    }

    /// Readings that arrive together are all drawn in the frame they arrive
    /// in, while the log follows its newest row — and so are the rows on
    /// screen, which that frame still shows before it moves down to them.
    #[test]
    fn the_newest_rows_are_drawn_as_they_arrive() {
        let mut run = run_with_markers(300, &[], true);
        let times: Vec<Instant> = run
            .app
            .recording
            .samples
            .iter()
            .map(|s| s.measurement.timestamp)
            .collect();
        let on_screen: Vec<Instant> = times
            .iter()
            .copied()
            .filter(|&at| shows_sample(&run, at))
            .collect();
        let last = *times.last().expect("samples");
        let new: Vec<Instant> = (1..=3)
            .map(|i| last + std::time::Duration::from_secs(i))
            .collect();
        for &at in &new {
            push_reading(&mut run, at);
        }
        run.frame(1.0, vec![]);
        for at in new {
            assert!(shows_sample(&run, at));
        }
        // The first is the row of margin above the view.
        for &at in &on_screen[1..] {
            assert!(shows_sample(&run, at), "a row on screen went blank");
        }
    }

    /// Readings that came in while the log wasn't drawn — big meter mode, or
    /// the panel off — cost the frame it comes back in a screenful of rows,
    /// not one row each.
    #[test]
    fn readings_that_arrive_unseen_cost_a_screenful() {
        let mut run = run_with_markers(300, &[], true);
        let last = run.app.recording.samples.back().expect("samples");
        let last = last.measurement.timestamp;
        for i in 1..=2_000 {
            push_reading(&mut run, last + std::time::Duration::from_secs(i));
        }
        run.frame(1.0, vec![]);
        let rows = run
            .tree
            .iter()
            .filter(|(_, n)| n.value().is_some_and(|v| v.contains(" V")))
            .count();
        assert!(rows < 100, "{rows} rows drawn");
        let newest = run.app.recording.samples.back().expect("samples");
        assert!(shows_sample(&run, newest.measurement.timestamp));
    }

    /// A marker's × keeps the focus while the log scrolls under it: its row
    /// keeps its id whichever rows are drawn before it.
    #[test]
    fn a_focused_cross_keeps_the_focus_as_the_log_scrolls() {
        let mut run = run_with_markers(300, &[290], true);
        for _ in 0..12 {
            if focused_label(&run).as_deref() == Some("Delete marker 1") {
                break;
            }
            run.key(Key::Tab);
        }
        assert_eq!(focused_label(&run).as_deref(), Some("Delete marker 1"));
        let over_log = run.node_rect("Note for marker 1").center();
        for delta in [-4.0, 4.0] {
            for _ in 0..30 {
                let wheel = egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, delta),
                    phase: egui::TouchPhase::Move,
                    modifiers: Modifiers::NONE,
                };
                run.frame(0.1, vec![egui::Event::PointerMoved(over_log), wheel]);
            }
            assert_eq!(focused_label(&run).as_deref(), Some("Delete marker 1"));
        }
    }

    /// A marked row is as tall as any other: its number, note and × are sized
    /// to the log's line, so the rows keep one pitch.
    #[test]
    fn a_marked_row_keeps_the_log_line_pitch() {
        let run = run_with_a_marker_at(178, true);
        let mut tops: Vec<f32> = run
            .tree
            .iter()
            .filter(|(_, n)| n.value().is_some_and(|v| v.contains(" V")))
            .filter_map(|(_, n)| n.bounds())
            .map(|b| to_rect(b).top())
            .collect();
        tops.sort_by(f32::total_cmp);
        tops.dedup();
        assert!(tops.len() > 5, "the log shows rows: {tops:?}");
        let pitch = tops[1] - tops[0];
        for pair in tops.windows(2) {
            assert!(
                (pair[1] - pair[0] - pitch).abs() < 0.5,
                "rows {pitch} apart, not {}: {tops:?}",
                pair[1] - pair[0]
            );
        }
    }

    /// In a log too narrow for a reading's line, the line is cut short
    /// rather than wrapped into the row below.
    #[test]
    fn a_narrow_log_keeps_one_line_per_row() {
        let mut run = MenuRun::new();
        run.app.toggle_recording();
        let wall_clock = run.app.wall_clock;
        let t0 = Instant::now();
        for i in 0..50 {
            let mut m = reading();
            m.timestamp = t0 + std::time::Duration::from_secs(i);
            // Sub-values long enough to need three lines at this width.
            m.aux_values = ["Max", "Min", "Average", "Peak Max", "Peak Min"]
                .into_iter()
                .map(|label| dmm_lib::measurement::AuxValue {
                    label: label.into(),
                    value: dmm_lib::measurement::MeasuredValue::Normal(1.234),
                    unit: "".into(),
                    display_raw: Some("1.234".into()),
                    elapsed_secs: None,
                })
                .collect();
            run.app.recording.push(&m, &wall_clock, 0);
        }
        run.width = 300.0;
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        let mut rows: Vec<Rect> = run
            .tree
            .iter()
            .filter(|(_, n)| n.value().is_some_and(|v| v.contains(':')))
            .filter_map(|(_, n)| n.bounds())
            .map(to_rect)
            .collect();
        rows.sort_by(|a, b| a.top().total_cmp(&b.top()));
        // A label and its text run share a rect.
        rows.dedup();
        assert!(rows.len() > 3, "{rows:?}");
        for pair in rows.windows(2) {
            assert!(pair[0].bottom() <= pair[1].top(), "rows overlap: {pair:?}");
        }
    }

    /// The marker column lines up on every row: a marker placed before
    /// Record, whose row says so, and one on a recorded sample.
    #[test]
    fn the_marker_column_lines_up() {
        let mut run = MenuRun::new();
        let t0 = Instant::now();
        let wall_clock = run.app.wall_clock;
        let mark = |run: &mut MenuRun, i: u64| {
            let mut m = reading();
            m.timestamp = t0 + std::time::Duration::from_secs(i);
            run.app.graph.push(1.234, m.timestamp, "DC V", "V", None);
            run.app.recording.push(&m, &wall_clock, 0);
            if i % 4 == 2 {
                run.app.last_measurement = Some(m);
                run.app.add_marker(false);
            }
        };
        for i in 0..4 {
            mark(&mut run, i);
        }
        run.app.toggle_recording();
        for i in 4..8 {
            mark(&mut run, i);
        }
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        for what in ["Note for marker {}", "Delete marker {}"] {
            let [before, recorded] =
                [1, 2].map(|n| run.node_rect(&what.replace("{}", &n.to_string())));
            assert!(
                (before.left() - recorded.left()).abs() < 0.5,
                "{what}: {before:?} and {recorded:?}"
            );
        }
        assert!(run.shows_text("not in the recording"), "marker 1 says so");
        // And the × stays clear of the floating scroll bar at the right.
        let bar = egui::Style::default().spacing.scroll.bar_width;
        let cross = run.node_rect("Delete marker 1");
        assert!(cross.right() + bar <= 800.0, "{cross:?}");
    }

    /// Where the log drew the row of the sample taken at `at`.
    fn sample_rect(run: &MenuRun, at: Instant) -> Rect {
        let sample = run
            .app
            .recording
            .samples
            .iter()
            .find(|s| s.measurement.timestamp == at)
            .expect("a buffered sample");
        let time = sample.wall_time.format("%H:%M:%S%.3f").to_string();
        run.tree
            .iter()
            .find(|(_, n)| n.value().is_some_and(|v| v.starts_with(&time)))
            .and_then(|(_, n)| n.bounds())
            .map(to_rect)
            .expect("the row is drawn")
    }

    /// Hovering a sample's row offers a marker; clicking it marks that
    /// sample and puts the cursor in its note.
    #[test]
    fn a_hovered_row_offers_a_marker() {
        let mut run = run_with_markers(300, &[], true);
        let at = run.app.recording.samples[295].measurement.timestamp;
        let time = run.app.recording.samples[295]
            .wall_time
            .format("%H:%M:%S%.3f")
            .to_string();
        let add = format!("Add a marker at {time}");
        assert!(!run.shows_widget(&add), "only on a hovered row");
        let row = sample_rect(&run, at);
        run.frame(1.0, vec![egui::Event::PointerMoved(row.center())]);
        run.frame(1.0, vec![egui::Event::PointerMoved(row.center())]);
        let plus = run.node_rect(&add);
        run.click(plus.center());
        let marker = run.app.markers.iter().next().expect("a marker");
        assert_eq!(marker.at, at, "on the row clicked");
        assert_eq!(focused_label(&run).as_deref(), Some("Note for marker 1"));
    }

    /// A marked row has its marker's tag, and offers no other.
    #[test]
    fn a_marked_row_offers_no_marker() {
        let mut run = run_with_markers(300, &[295], true);
        let sample = &run.app.recording.samples[295];
        let (at, time) = (
            sample.measurement.timestamp,
            sample.wall_time.format("%H:%M:%S%.3f").to_string(),
        );
        let row = sample_rect(&run, at);
        run.frame(1.0, vec![egui::Event::PointerMoved(row.center())]);
        run.frame(1.0, vec![egui::Event::PointerMoved(row.center())]);
        assert!(!run.shows_widget(&format!("Add a marker at {time}")));
    }

    /// N places no marker under the discard prompt: the prompt would be
    /// asking about something that changed behind it.
    #[test]
    fn n_waits_for_the_discard_prompt() {
        let mut run = run_with_stopped_recording(2);
        run.app.last_measurement = run
            .app
            .recording
            .samples
            .back()
            .map(|s| s.measurement.clone());
        run.click(run.node_rect("Discard").center());
        assert!(run.shows_widget("Discard recording"), "the prompt is up");
        run.key(Key::N);
        assert!(run.app.markers.is_empty());
    }

    /// The marker's number is a button: pressing it brings the marker into
    /// view.
    #[test]
    fn the_number_shows_its_marker_on_the_graph() {
        let mut run = run_with_a_marker_at(20, false);
        assert!(!run.app.graph.is_view_zoomed(), "live");
        run.click(run.node_rect("Show marker 1 on the graph").center());
        assert!(run.app.graph.is_view_zoomed(), "scrolled back to it");
    }

    /// Tab walks through a row, the number and the note included, without
    /// moving the graph.
    #[test]
    fn tabbing_through_a_row_leaves_the_graph_alone() {
        let mut run = run_with_a_marker_at(20, false);
        let number = run.node_rect("Show marker 1 on the graph");
        let note = run.node_rect("Note for marker 1");
        let mut stops = Vec::new();
        for _ in 0..6 {
            run.key(Key::Tab);
            if let Some(r) = run.focused_rect() {
                stops.push(r.center());
            }
        }
        assert!(stops.iter().any(|p| (*p - number.center()).length() < 1.0));
        assert!(stops.iter().any(|p| (*p - note.center()).length() < 1.0));
        assert!(!run.app.graph.is_view_zoomed(), "tabbing moves nothing");
    }

    /// Clicking into a note brings its marker into view; the keystrokes
    /// after leave the view to the user.
    #[test]
    fn clicking_a_note_shows_its_marker_once() {
        let mut run = run_with_a_marker_at(20, false);
        run.click(run.node_rect("Note for marker 1").center());
        assert!(run.app.graph.is_view_zoomed());
        run.app.graph.reset_view();
        run.frame(1.0, vec![egui::Event::Text("fan".into())]);
        assert!(
            !run.app.graph.is_view_zoomed(),
            "back in live, and left there"
        );
    }

    /// Ctrl+R and N in one frame: the prompt Ctrl+R asks for is not on
    /// screen yet, and N still waits for it.
    #[test]
    fn n_waits_for_a_prompt_asked_for_in_the_same_frame() {
        let mut run = run_with_stopped_recording(2);
        // Ctrl+R only answers while connected.
        run.app.connection.state = ConnectionState::Connected;
        run.app.last_measurement = run
            .app
            .recording
            .samples
            .back()
            .map(|s| s.measurement.clone());
        let press = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        run.frame(
            1.0,
            vec![
                press(Key::R, Modifiers::COMMAND),
                press(Key::N, Modifiers::NONE),
            ],
        );
        assert!(run.app.recording_panel.pending_discard.is_some());
        assert!(run.app.markers.is_empty());
    }

    /// A recording keeps a marker the graph has dropped: its row says so, and
    /// its number, with nothing to show, is greyed.
    #[test]
    fn a_marker_off_the_graph_says_so() {
        let mut run = run_with_a_marker_at(20, true);
        assert!(!run.shows_text("not on the graph"));
        run.app.graph.clear();
        run.frame(1.0, vec![]);
        run.frame(1.0, vec![]);
        assert!(run.shows_text("not on the graph"));
        let show = run
            .tree
            .iter()
            .find(|(_, n)| n.label() == Some("Show marker 1 on the graph"))
            .map(|(_, n)| n.is_disabled());
        assert_eq!(show, Some(true), "nowhere left to show it");
    }

    /// Record over an unexported recording still asks in its own words.
    #[test]
    fn recording_over_an_unexported_recording_still_asks() {
        let mut run = run_with_stopped_recording(1);
        run.click(run.node_rect("\u{25CF} Record").center());
        assert!(run.shows_text("Starting a new recording will discard 1 unexported sample."));
        run.click(run.node_rect("Discard and record").center());
        assert!(run.app.recording.active);
        assert!(run.app.recording.samples.is_empty());
    }
}
