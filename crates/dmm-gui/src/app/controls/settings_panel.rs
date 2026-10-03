use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use dmm_shared::help::BLUETOOTH_SETTING;
use eframe::egui::{self, RichText, Ui};

use crate::settings::{GraphLines, SpecFields, buffer_memory_estimate, format_sample_count};

use crate::app::appearance::ALWAYS_ON_TOP_WAYLAND_HINT;
use crate::app::{App, BigMeterMode};

use super::device_list::device_dropdown;
use super::{Chip, chip_row};

/// Show a settings checkbox with a hover tooltip; returns `true` if the value changed.
fn setting_checkbox(ui: &mut Ui, value: &mut bool, label: &str, tooltip: &str) -> bool {
    ui.checkbox(value, label).on_hover_text(tooltip).changed()
}

/// The **Specifications** checkbox and, while it is on, the fields the
/// panel shows after it. The group is one unit, sized before it is
/// placed so the wrapped row moves it to the next line whole, and it wraps
/// within itself only when a whole line is too narrow for it (`color_edit`
/// says why a `ui.horizontal` can't do this). Returns whether the panel
/// checkbox changed, and whether a field did.
fn specs_checkboxes(ui: &mut Ui, show: &mut bool, fields: &mut SpecFields) -> (bool, bool) {
    // Read before a click can flip it, so the frame draws what it measured.
    let on = *show;
    let panel_label = if on {
        "Specifications:"
    } else {
        "Specifications"
    };
    let SpecFields {
        resolution,
        accuracy,
        input_impedance,
        notes,
    } = fields;
    let mut field_boxes = [
        (
            resolution,
            "Resolution",
            "Show the resolution in the specifications",
        ),
        (
            accuracy,
            "Accuracy",
            "Show the accuracy in the specifications",
        ),
        (
            input_impedance,
            "Input Z",
            "Show the input impedance in the full Specifications panel",
        ),
        (
            notes,
            "Notes",
            "Show the manual's notes in the full Specifications panel",
        ),
    ];
    let shown = if on { field_boxes.len() } else { 0 };

    // A checkbox as egui lays it out: the box, the icon gap, the label.
    let checkbox_width = |label: &str| {
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        );
        ui.spacing().icon_width + ui.spacing().icon_spacing + galley.size().x
    };
    let width = std::iter::once(panel_label)
        .chain(field_boxes[..shown].iter().map(|(_, label, _)| *label))
        .map(checkbox_width)
        .sum::<f32>()
        + ui.spacing().item_spacing.x * shown as f32;
    let size = egui::vec2(
        width.min(ui.max_rect().width()),
        ui.spacing().interact_size.y,
    );
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
        |ui| {
            let panel_changed = setting_checkbox(
                ui,
                show,
                panel_label,
                "Show accuracy and resolution for the current mode",
            );
            let mut fields_changed = false;
            for (value, label, tooltip) in &mut field_boxes[..shown] {
                fields_changed |= setting_checkbox(ui, value, label, tooltip);
            }
            (panel_changed, fields_changed)
        },
    )
    .inner
}

/// A **Sample interval** chip's label: what a 0 ms interval means, and whole
/// seconds as seconds, as the **Buffer size** row beside it keeps its
/// labels short.
fn interval_label(ms: u32) -> String {
    match ms {
        0 => "Every reading".to_string(),
        ms if ms >= 1000 && ms % 1000 == 0 => format!("{}s", ms / 1000),
        ms => format!("{ms}ms"),
    }
}

/// What a **Sample interval** chip keeps, for its tooltip.
fn interval_tooltip(ms: u32) -> String {
    let every = match ms {
        0 => return "Keep every reading the meter produces, at its own pace".to_string(),
        1000 => "a second".to_string(),
        ms if ms % 1000 == 0 => format!("every {} s", ms / 1000),
        ms => format!("every {ms} ms"),
    };
    format!("At most one reading {every}: the one nearest each tick")
}

/// What a bound of `n` samples costs, as the **Buffer size** row states it:
/// the memory the graph and the sample buffer take, and how long the bound
/// lasts at the current sample interval.
fn buffer_cost(n: usize, overlays: usize, aux: usize, interval_ms: u32) -> (String, String) {
    // The same wire-time floor `Graph::set_sample_interval_ms` assumes for a
    // 0 ms interval, so the row and the gap detector agree on the rate.
    let interval_secs = (interval_ms as f64 / 1000.0).max(0.1);
    let hours = n as f64 * interval_secs / 3600.0;
    let span = if hours < 10.0 {
        format!("{hours:.1} h")
    } else {
        format!("{hours:.0} h")
    };
    (buffer_memory_estimate(n, overlays, aux), span)
}

/// What the settings panel leaves below itself for the rest of the window.
/// Raising it keeps more of the reading and the graph visible on a short
/// window, and starts scrolling the settings sooner.
const SETTINGS_RESERVE: f32 = 160.0;

/// The shortest the settings rows are ever squashed to, even when that eats
/// into [`SETTINGS_RESERVE`]: below a few lines the panel is a scrollbar
/// beside a sliver of content, and nothing can be found in it.
const SETTINGS_MIN_HEIGHT: f32 = 96.0;

/// How tall the scrolling settings rows may be, given the window height and
/// where the rows start. Floored to whole points so that a fractional
/// overflow can't raise a scrollbar beside rows that fit.
fn settings_scroll_cap(window_h: f32, top: f32) -> f32 {
    (window_h - top - SETTINGS_RESERVE)
        .max(SETTINGS_MIN_HEIGHT)
        .floor()
}

impl App {
    /// The rows below the bar row while the settings are open. Returns the
    /// scroll area's output so a test can check that nothing is scrolled when
    /// the rows fit, and `None` when the settings are closed.
    pub(in crate::app) fn show_settings_panel(
        &mut self,
        ui: &mut Ui,
    ) -> Option<egui::scroll_area::ScrollAreaOutput<()>> {
        if !self.settings_open {
            return None;
        }

        ui.separator();
        // A panel clips its content and never scrolls, so on a short window
        // the rows below would simply be cut off — Zoom, the way back from a
        // scale that made the window unusable, among them. Cap the rows and
        // scroll them instead. The cap is measured from the window rather
        // than from `available_height()`: inside a panel the content ui's
        // `max_rect` is last frame's panel rect, so the space a `ScrollArea`
        // would size itself from is stale, and often nothing at all. The cap
        // is handed over as a child ui rather than via `set_max_height`,
        // which unions the new bound with everything placed so far and then
        // moves the cursor back to the top of the panel — the rows would be
        // painted over the bar row.
        let cap = settings_scroll_cap(ui.ctx().content_rect().height(), ui.cursor().top());
        let scrolled = ui
            .allocate_ui(egui::vec2(ui.available_width(), cap), |ui| {
                // Full width, so the bar sits at the panel's edge rather than
                // at the widest row's.
                egui::ScrollArea::vertical()
                    .id_salt("settings_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.show_settings_rows(ui);
                        crate::a11y::scroll_to_focus(ui);
                    })
            })
            .inner;

        // Outside the scroller: the rule closing the panel belongs to the
        // panel, not to the last row that happens to be scrolled into view.
        ui.separator();
        Some(scrolled)
    }

    /// Why the session's meter is not the Device row's to change, when it
    /// isn't: the note the pinned row carries, saying how to get it back.
    ///
    /// A replay's frames come from the file whatever the row names — Connect
    /// re-opens the recording — and the clock flags bend session time, which
    /// only the mock can be asked to run on (`main.rs` refuses them beside a
    /// hardware `--device`), so picking a meter would leave the session timing
    /// its recording and dating its exports by a clock no meter ever ran on.
    fn device_row_pin(&self) -> Option<&'static str> {
        if self.replay.is_some() {
            Some("(restart without --replay to pick a meter)")
        } else if !self.clock.is_real() {
            Some("(restart without the clock flags to pick a meter)")
        } else {
            None
        }
    }

    /// The settings rows, top to bottom. Drawn inside the scroll area that
    /// [`Self::show_settings_panel`] caps, and kept in their own method so
    /// that the rows stay at one indentation level. The colour rows at the
    /// top live in `colors.rs`.
    fn show_settings_rows(&mut self, ui: &mut Ui) {
        self.show_theme_row(ui);

        // -- Color preset selector --
        // Presets are palettes for Dark, Light and System; a named theme
        // carries its own, so the row only shows where it applies.
        if self.settings.active_theme().is_none() {
            self.show_color_preset_row(ui);
        }

        // -- Collapsible color customization --
        self.show_color_customization(ui);

        // -- Graph line style --
        ui.horizontal_wrapped(|ui| {
            let chips = [
                (
                    GraphLines::Patterned,
                    "Patterned",
                    "Dashed and dotted sub-value lines, told apart without colour",
                ),
                (
                    GraphLines::Solid,
                    "Solid",
                    "Continuous lines, told apart by colour and the key",
                ),
            ]
            .map(|(value, label, tooltip)| Chip {
                value,
                selected: self.settings.graph_lines == value,
                label: label.to_string(),
                tooltip: tooltip.to_string(),
            });
            if let Some(lines) = chip_row(ui, "Graph lines:", chips) {
                self.settings.graph_lines = lines;
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let changed = setting_checkbox(
                ui,
                &mut self.settings.show_graph,
                "Graph",
                "Show the rolling time-series plot",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_stats,
                "Statistics",
                "Show Min / Max / Avg / integral for the live session",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_recording,
                "Recording",
                "Show the recording controls and sample log",
            );
            let (specs_changed, fields_changed) = specs_checkboxes(
                ui,
                &mut self.settings.show_specs,
                &mut self.settings.spec_fields,
            );
            if changed || specs_changed {
                // Manual settings change exits big meter toggle.
                self.big_meter_mode = BigMeterMode::Off;
            }
            if changed || specs_changed || fields_changed {
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let changed = setting_checkbox(
                ui,
                &mut self.settings.auto_connect,
                "Auto-connect on start",
                "Open the USB connection automatically when the app launches",
            ) | setting_checkbox(
                ui,
                &mut self.settings.query_device_name,
                "Show device name on connect (beeps)",
                "Query the meter's name after connecting — the meter will beep once",
            );
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let chips = [0u32, 100, 200, 300, 500, 1000, 2000]
                .into_iter()
                .map(|ms| Chip {
                    value: ms,
                    selected: self.settings.sample_interval_ms == ms,
                    label: interval_label(ms),
                    tooltip: interval_tooltip(ms),
                });
            if let Some(ms) = chip_row(ui, "Sample interval:", chips) {
                self.settings.sample_interval_ms = ms;
                self.settings.save();
                self.apply_sample_interval();
            }
        });

        ui.horizontal_wrapped(|ui| {
            // The cost of a bound depends on what this meter is sending right
            // now: a UT181A's four sub-values roughly triple a buffered
            // sample's size and add a trace to the graph's.
            let overlays = self.graph.overlays_len();
            let aux = self
                .last_measurement
                .as_ref()
                .map_or(0, |m| m.aux_values.len());
            let interval_ms = self.settings.sample_interval_ms;
            let chips = [100_000usize, 500_000, 1_000_000, 2_000_000, 5_000_000]
                .into_iter()
                .map(|n| {
                    let (memory, span) = buffer_cost(n, overlays, aux, interval_ms);
                    // Every reading comes at the meter's own pace, which the
                    // estimate cannot know: it says what it assumed.
                    let pace = if interval_ms == 0 {
                        "at 10 readings a second"
                    } else {
                        "at the current sample interval"
                    };
                    Chip {
                        value: n,
                        selected: self.settings.max_samples == n,
                        label: format_sample_count(n),
                        tooltip: format!(
                            "Keep up to {} samples in the graph and for export \u{2014} {memory}, \
                             about {span} {pace}. A stopped recording kept beside them can \
                             take as much again",
                            format_sample_count(n)
                        ),
                    }
                });
            if let Some(n) = chip_row(ui, "Buffer size:", chips) {
                self.settings.max_samples = n;
                self.settings.save();
                // Live: the graph and the history behind Export… evict down
                // to the new bound on the spot, and a recording already past
                // it stops rather than losing the samples it has.
                self.graph.set_max_points(n);
                if self.capture.recording.set_max_samples(n) {
                    self.buffer_shrunk_toast();
                }
            }
            let (memory, span) = buffer_cost(self.settings.max_samples, overlays, aux, interval_ms);
            ui.label(
                RichText::new(format!("({memory}, about {span})"))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.horizontal_wrapped(|ui| {
            let pinned = self.device_row_pin();
            let has_override = self.settings.overrides.has_device();
            // What the selection resolves to, not what the file spells: an
            // alias, or an id no entry answers to, would otherwise leave the
            // row with nothing marked while the session opened something.
            let selected_id = self
                .selected_device()
                .map_or(registry::AUTO_DEVICE_ID, |d| d.id);
            let name = registry::find_device(selected_id).map_or("Auto-detect", |d| d.display_name);
            let label = if has_override {
                format!("{name} (--device)")
            } else {
                name.to_string()
            };
            let picked = ui
                .scope(|ui| {
                    if pinned.is_some() {
                        ui.disable();
                    }
                    ui.label("Device:");
                    device_dropdown(ui, selected_id, &label)
                })
                .inner;
            if let Some(note) = pinned {
                ui.label(
                    RichText::new(note)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if let Some(id) = picked {
                self.settings.shared.device_family = id.to_string();
                // Clear the override — user explicitly chose a device
                self.settings.overrides.device_family = None;
                self.settings.save();
                // Auto-reconnect if currently connected
                if self.connection.state != crate::app::ConnectionState::Disconnected {
                    self.connection.needs_reconnect = true;
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            let label = if self.settings.overrides.has_bluetooth() {
                format!("{BLUETOOTH_SETTING} (--no-bluetooth)")
            } else {
                BLUETOOTH_SETTING.to_string()
            };
            // No reconnect: a session already running over an adapter would
            // be dropped by one, and the setting only decides what the next
            // connect looks at.
            if setting_checkbox(
                ui,
                &mut self.settings.shared.bluetooth,
                &label,
                "Look for a Bluetooth adapter or meter in range when no USB cable answers. \
                 Takes effect on the next connect.",
            ) {
                // Cleared like every other row the user sets by hand: the
                // value they picked is theirs to keep.
                self.settings.overrides.bluetooth = None;
                self.settings.save();
            }
        });

        // Mock mode selector (only shown when mock device is selected)
        if self
            .selected_device()
            .is_some_and(|d| d.id == dmm_lib::mock::MOCK.id)
        {
            ui.horizontal_wrapped(|ui| {
                let has_override = self.settings.overrides.has_mock_mode();
                // "Auto" = cycle through all modes, and leads the row.
                let auto_selected = self.settings.mock_mode.is_empty();
                let auto = std::iter::once(Chip {
                    value: String::new(),
                    selected: auto_selected,
                    label: if auto_selected && has_override {
                        "Auto (cycle) (--mock-mode)"
                    } else {
                        "Auto (cycle)"
                    }
                    .to_string(),
                    tooltip: "Cycle through all synthetic modes to exercise the GUI".to_string(),
                });
                let modes = MockMode::ALL.iter().map(|mode| {
                    let mode_label = mode.label();
                    let selected = self.settings.mock_mode == mode_label;
                    Chip {
                        value: mode_label.to_string(),
                        selected,
                        label: if selected && has_override {
                            format!("{mode_label} (--mock-mode)")
                        } else {
                            mode_label.to_string()
                        },
                        tooltip: mode.description().to_string(),
                    }
                });
                if let Some(mock_mode) = chip_row(ui, "Mock mode:", auto.chain(modes)) {
                    self.settings.mock_mode = mock_mode;
                    // Clear the override — user explicitly chose a mock mode
                    self.settings.overrides.mock_mode = None;
                    self.settings.save();
                    if self.connection.state != crate::app::ConnectionState::Disconnected {
                        self.connection.needs_reconnect = true;
                    }
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            let chips = Self::ZOOM_LEVELS.iter().map(|&level| Chip {
                value: level,
                selected: self.settings.zoom_pct == level,
                label: format!("{level}%"),
                tooltip: if level == 100 {
                    "Scale the GUI to 100% (Ctrl+0, or Ctrl+/- to step)".to_string()
                } else {
                    format!("Scale the GUI to {level}% (Ctrl+/- to step)")
                },
            });
            if let Some(level) = chip_row(ui, "Zoom:", chips) {
                self.settings.zoom_pct = level;
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+/- to adjust, Ctrl+0 = 100%)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        // The Wayland caption is a sentence, so it wraps rather than
        // running off the edge of a narrow window.
        ui.horizontal_wrapped(|ui| {
            // Greyed rather than hidden on Wayland, and the saved value is
            // left alone: a `true` written on an X11 session still applies
            // there.
            let response = ui
                .add_enabled(
                    !self.on_wayland,
                    egui::Checkbox::new(&mut self.settings.always_on_top, "Always on top"),
                )
                .on_hover_text("Keep the window above other desktop windows (Ctrl+T)")
                .on_disabled_hover_text(ALWAYS_ON_TOP_WAYLAND_HINT);
            if response.changed() {
                self.apply_always_on_top(ui.ctx());
                self.settings.save();
            }
            if self.on_wayland {
                ui.label(
                    RichText::new(format!("({ALWAYS_ON_TOP_WAYLAND_HINT})"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        ui.horizontal_wrapped(|ui| {
            if setting_checkbox(
                ui,
                &mut self.settings.hide_decorations,
                "Hide window decorations",
                "Borderless window — use Ctrl+D to toggle back",
            ) {
                self.apply_decorations(ui.ctx());
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+D to toggle)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        // Only downloaded builds ask, so only they have the row. Last, so the
        // rows above keep the places the doc screenshots click.
        if self.update_check.applies() {
            ui.horizontal_wrapped(|ui| {
                if setting_checkbox(
                    ui,
                    &mut self.settings.check_for_updates,
                    "Check for new versions",
                    "Once a day, ask GitHub whether a newer release is out. \
                     GitHub sees your IP address; nothing about your meter is sent.",
                ) {
                    self.settings.save();
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::update_check::{Tag, UpdateCheck};
    use crate::settings::Settings;
    use eframe::egui::scroll_area::ScrollAreaOutput;
    use eframe::egui::{Id, Pos2, Rect, vec2};

    /// The settings rows laid out under the real bar row.
    struct SettingsFrame {
        ctx: egui::Context,
        /// Where the bar row ends: the rows must start below it.
        bar_bottom: f32,
        scrolled: ScrollAreaOutput<()>,
        /// The AccessKit tree, empty unless the run enabled it.
        nodes: Vec<(egui::accesskit::NodeId, egui::accesskit::Node)>,
        /// The AccessKit node with keyboard focus, when the run enabled it.
        focus: Option<egui::accesskit::NodeId>,
    }

    /// An open settings panel in a `w` x `h` window, driven a frame at a
    /// time. The bar row is the app's own, since one bug this guards against
    /// is the rows being laid over it.
    struct SettingsRun {
        app: App,
        ctx: egui::Context,
        screen: Rect,
        /// Jumps a second per frame, so egui's scroll animation — a few
        /// hundred milliseconds — has always finished by the next one.
        seconds: f64,
    }

    impl SettingsRun {
        fn new(w: f32, h: f32) -> Self {
            let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
            app.settings_open = true;
            // As a downloaded build, which has one row more, and with its
            // notice on the bar — forced, so nothing is fetched or saved.
            app.update_check = UpdateCheck::new(Tag::parse("v9.9.9"));
            Self {
                app,
                ctx: egui::Context::default(),
                screen: Rect::from_min_size(Pos2::ZERO, vec2(w, h)),
                seconds: 0.0,
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) -> SettingsFrame {
            self.frame_after(1.0, events)
        }

        /// A frame `secs` after the previous one.
        fn frame_after(&mut self, secs: f64, events: Vec<egui::Event>) -> SettingsFrame {
            let mut bar_bottom = 0.0;
            let mut scrolled = None;
            let app = &mut self.app;
            self.seconds += secs;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(self.screen),
                    events,
                    time: Some(self.seconds),
                    ..Default::default()
                },
                |ui| {
                    egui::Panel::top("top_bar").show(ui, |ui| {
                        let ctx = ui.ctx().clone();
                        app.show_top_bar(ui, &ctx);
                        bar_bottom = ui.min_rect().bottom();
                        scrolled = app.show_settings_panel(ui);
                    });
                },
            );
            out.textures_delta.clear();
            let (nodes, focus) = out
                .platform_output
                .accesskit_update
                .map(|update| (update.nodes, Some(update.focus)))
                .unwrap_or_default();
            SettingsFrame {
                ctx: self.ctx.clone(),
                bar_bottom,
                scrolled: scrolled.expect("the settings are open"),
                nodes,
                focus,
            }
        }

        /// Click `pos` the way a mouse does — move, press, release on
        /// successive frames — and return the release frame. The release
        /// follows the press within egui's click window, not a second later.
        fn click(&mut self, pos: Pos2) -> SettingsFrame {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            self.frame(vec![egui::Event::PointerMoved(pos)]);
            self.frame(vec![egui::Event::PointerMoved(pos), button(true)]);
            self.frame_after(0.1, vec![egui::Event::PointerMoved(pos), button(false)])
        }
    }

    /// Where a widget sits, as AccessKit reports it: a button carries its
    /// text as the node's label, a label as its value.
    fn node_bounds(frame: &SettingsFrame, text: &str) -> egui::accesskit::Rect {
        frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(text) || n.value() == Some(text))
            .and_then(|(_, n)| n.bounds())
            .unwrap_or_else(|| panic!("no {text:?} widget in the settings"))
    }

    /// After three headless frames — a panel sizes itself from the previous
    /// frame, so the first one alone proves nothing.
    fn settings_panel(w: f32, h: f32) -> SettingsFrame {
        let mut run = SettingsRun::new(w, h);
        run.frame(vec![]);
        run.frame(vec![]);
        run.frame(vec![])
    }

    /// Tab pressed and released within one frame.
    fn tab() -> Vec<egui::Event> {
        press(egui::Key::Tab)
    }

    /// `key` pressed and released within one frame.
    fn press(key: egui::Key) -> Vec<egui::Event> {
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

    /// Whether the focused node is one named `label`: the Device box's
    /// value and its live entry's label are the same name.
    fn focused_on(frame: &SettingsFrame, label: &str) -> bool {
        frame.nodes.iter().any(|(id, n)| {
            (n.label() == Some(label) || n.value() == Some(label)) && frame.focus == Some(*id)
        })
    }

    /// Whether the open Device list is drawn: its Auto-detect entry is.
    fn device_list_open(frame: &SettingsFrame) -> bool {
        frame.nodes.iter().any(|(_, n)| {
            n.label() == Some("Auto-detect") && n.role() != egui::accesskit::Role::ComboBox
        })
    }

    /// A settings run with the **Device** list opened from the keyboard:
    /// Tab to the box, then Space, as a keyboard user opens it.
    fn open_device_list(run: &mut SettingsRun) -> SettingsFrame {
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let mut reached = false;
        for _ in 0..60 {
            let frame = run.frame(tab());
            if focused_on(&frame, "Device") {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab never reached the Device box");
        run.frame(press(egui::Key::Space));
        run.frame(vec![])
    }

    /// Opened from the keyboard, the list puts focus on the selected meter,
    /// so Enter picks it and a screen reader names it, with no Tab first.
    #[test]
    fn the_device_list_opens_on_the_selected_meter() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        let frame = open_device_list(&mut run);
        assert!(device_list_open(&frame), "Space did not open the list");
        assert!(
            focused_on(&frame, "UT61E+"),
            "focus is not on the selected meter"
        );
    }

    /// Down follows the brand's column, Right jumps to the next column over
    /// — UNI-T to Voltcraft at this width — and Left comes back.
    #[test]
    fn arrows_walk_the_device_list_down_and_across() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        let frame = run.frame(press(egui::Key::ArrowDown));
        assert!(focused_on(&frame, "UT61B+"));
        let frame = run.frame(press(egui::Key::ArrowRight));
        assert!(
            focused_on(&frame, "Voltcraft VC650BT"),
            "Right left the row"
        );
        let frame = run.frame(press(egui::Key::ArrowLeft));
        assert!(focused_on(&frame, "UT61B+"));
    }

    /// Enter picks the focused meter: saved as the device, the list closed
    /// and the keyboard back on the box.
    #[test]
    fn enter_picks_the_focused_meter() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        run.frame(press(egui::Key::ArrowDown));
        run.frame(press(egui::Key::Enter));
        let frame = run.frame(vec![]);
        assert_eq!(run.app.settings.shared.device_family, "ut61b+");
        assert!(!device_list_open(&frame), "the list stayed open");
        let (id, node) = frame
            .nodes
            .iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::ComboBox && n.label() == Some("Device")
            })
            .expect("the Device box, named for a screen reader");
        assert_eq!(node.value(), Some("UT61B+"));
        assert_eq!(frame.focus, Some(*id), "focus did not return to the box");
    }

    /// Esc leaves the list with nothing picked.
    #[test]
    fn escape_closes_the_device_list_without_a_pick() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        run.frame(press(egui::Key::ArrowDown));
        run.frame(press(egui::Key::Escape));
        let frame = run.frame(vec![]);
        assert_eq!(run.app.settings.shared.device_family, "ut61eplus");
        assert!(!device_list_open(&frame), "the list stayed open");
        assert!(
            focused_on(&frame, "UT61E+"),
            "focus did not return to the box"
        );
    }

    /// egui measures a popup once, as it opens, and only widens it after:
    /// narrowed under an open list, the list kept its four columns and ran
    /// off the window. It closes and re-opens at its new size instead.
    #[test]
    fn an_open_device_list_follows_the_window() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        open_device_list(&mut run);
        for width in [700.0, 1600.0, 700.0] {
            run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 1000.0));
            // A frame closed, one measuring, and the list is back.
            run.frame(vec![]);
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            assert!(
                device_list_open(&frame),
                "the list stayed closed at {width}"
            );
            for (_, n) in &frame.nodes {
                if let Some(b) = n.bounds() {
                    assert!(
                        b.x1 <= f64::from(width),
                        "{:?} runs off a {width} pt window: {b:?}",
                        n.label().or(n.value())
                    );
                }
            }
        }
    }

    /// Esc on the frame a resize re-shapes the list closes it for good: the
    /// re-measuring close used to queue a re-open behind the user's.
    #[test]
    fn escape_during_a_resize_keeps_the_device_list_closed() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        open_device_list(&mut run);
        run.screen = Rect::from_min_size(Pos2::ZERO, vec2(700.0, 1000.0));
        run.frame(press(egui::Key::Escape));
        for _ in 0..3 {
            let frame = run.frame(vec![]);
            assert!(!device_list_open(&frame), "the list opened again");
        }
    }

    /// The longest name, overridden, on the narrowest window: the box cuts
    /// its text short rather than running off the panel.
    #[test]
    fn a_long_device_name_stays_within_a_narrow_panel() {
        let width = 400.0;
        let mut run = SettingsRun::new(width, 900.0);
        run.app.settings.shared.device_family = "bm82x".to_string();
        run.app.settings.overrides.device_family = Some("auto".to_string());
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        let (_, node) = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Device"))
            .expect("the Device box");
        let bounds = node.bounds().expect("the box is laid out");
        let edge = frame.scrolled.inner_rect.right();
        assert!(
            bounds.x1 <= f64::from(edge),
            "the Device box runs off the {edge} pt panel: {bounds:?}"
        );
    }

    #[test]
    fn interval_chips_name_what_they_keep() {
        assert_eq!(interval_label(0), "Every reading");
        assert_eq!(interval_label(300), "300ms");
        assert_eq!(interval_label(1000), "1s");
        assert_eq!(interval_label(2000), "2s");
        assert_eq!(
            interval_tooltip(0),
            "Keep every reading the meter produces, at its own pace"
        );
        assert_eq!(
            interval_tooltip(1000),
            "At most one reading a second: the one nearest each tick"
        );
        assert_eq!(
            interval_tooltip(2000),
            "At most one reading every 2 s: the one nearest each tick"
        );
        assert_eq!(
            interval_tooltip(100),
            "At most one reading every 100 ms: the one nearest each tick"
        );
    }

    #[test]
    fn the_scroll_cap_follows_the_window_down_to_a_floor() {
        assert_eq!(
            settings_scroll_cap(900.0, 40.0),
            900.0 - 40.0 - SETTINGS_RESERVE
        );
        // Too short for the reserve: the rows keep their few lines instead.
        assert_eq!(settings_scroll_cap(220.0, 40.0), SETTINGS_MIN_HEIGHT);
    }

    /// A panel clips its content and never scrolls, so without the cap the
    /// top bar grew over the whole window and the central panel — and the
    /// reading with it — was squeezed out from below.
    #[test]
    fn a_short_window_keeps_the_settings_panel_off_the_rest_of_it() {
        let frame = settings_panel(400.0, 220.0);
        let panel = egui::PanelState::load(&frame.ctx, Id::new("top_bar")).expect("the panel ran");
        // The floor wins at this height, so the panel is the bar row, the
        // rows' cap, the two separators and the frame's margin — and the
        // rest of the window is left to the central panel.
        let bottom = panel.outer_rect.bottom();
        assert!(
            bottom <= frame.bar_bottom + SETTINGS_MIN_HEIGHT + 32.0,
            "settings panel took {bottom} pt of a 220 pt window (bar ends at {})",
            frame.bar_bottom
        );
        // And the rows do scroll: there is more than the cap can show.
        let inner = frame.scrolled.inner_rect;
        assert!(
            frame.scrolled.content_size.y > inner.height(),
            "rows {} pt tall fit a {} pt viewport, nothing to scroll",
            frame.scrolled.content_size.y,
            inner.height()
        );
    }

    /// `set_max_height` would have put the rows here: it unions the new
    /// bound with what was already placed and moves the cursor back to the
    /// top of the panel, so the rows were painted over the bar row.
    #[test]
    fn the_rows_start_below_the_bar_row() {
        for h in [220.0, 900.0] {
            let frame = settings_panel(400.0, h);
            let top = frame.scrolled.inner_rect.top();
            assert!(
                top >= frame.bar_bottom,
                "rows start at {top} pt, over a bar row ending at {} pt, in a {h} pt window",
                frame.bar_bottom
            );
        }
    }

    /// And when the window is tall enough, every row is on screen and the
    /// scroll area is invisible: it shrinks to the rows, so no bar appears
    /// and there is nothing to scroll.
    #[test]
    fn a_tall_window_shows_every_row_with_nothing_scrolled() {
        // Tall enough for every row, wrapped at this width.
        let frame = settings_panel(400.0, 1200.0);
        let panel = egui::PanelState::load(&frame.ctx, Id::new("top_bar")).expect("the panel ran");
        let height = panel.outer_rect.height();
        assert!(
            height > SETTINGS_MIN_HEIGHT + 40.0,
            "settings panel is only {height} pt tall in a 1200 pt window"
        );
        // Short of the cap, so the rows fit inside it.
        assert!(
            height < settings_scroll_cap(1200.0, 0.0),
            "settings panel is {height} pt tall, at the cap"
        );
        let inner = frame.scrolled.inner_rect;
        assert!(
            frame.scrolled.content_size.y <= inner.height() + 0.01,
            "rows {} pt tall overflow a {} pt viewport",
            frame.scrolled.content_size.y,
            inner.height()
        );
        assert_eq!(frame.scrolled.state.offset, egui::Vec2::ZERO);
    }

    /// egui scrolls to a focused widget only when assistive tech asks, so
    /// Tab walked below the fold with nothing on screen to show for it.
    #[test]
    fn tab_brings_the_focused_row_into_view() {
        let mut run = SettingsRun::new(400.0, 220.0);
        for _ in 0..3 {
            run.frame(vec![]);
        }
        let mut rows_focused = 0;
        let mut scrolled_down = false;
        for _ in 0..60 {
            run.frame(tab());
            // Focus moves within the Tab frame and the scroller is asked at
            // once, but egui animates the move and places the content a frame
            // behind the offset — a person sees it settle within two frames.
            run.frame(vec![]);
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            let Some(id) = run.ctx.memory(|m| m.focused()) else {
                continue;
            };
            let Some(response) = run.ctx.read_response(id) else {
                continue;
            };
            let inner = frame.scrolled.inner_rect;
            let content = Rect::from_min_size(
                inner.min - frame.scrolled.state.offset,
                frame.scrolled.content_size,
            );
            // Only the rows are the scroller's business; the bar row's
            // buttons are outside it, though the scrolled content's rect
            // reaches up over them once the rows are scrolled far enough.
            if !content.contains_rect(response.rect) || response.rect.bottom() <= frame.bar_bottom {
                continue;
            }
            rows_focused += 1;
            scrolled_down |= frame.scrolled.state.offset.y > 0.0;
            assert!(
                response.rect.top() >= inner.top() - 1.0
                    && response.rect.bottom() <= inner.bottom() + 1.0,
                "focused control at {:?} is outside the {:?} viewport",
                response.rect,
                inner
            );
        }
        assert!(
            rows_focused > 5,
            "Tab reached only {rows_focused} settings controls"
        );
        assert!(scrolled_down, "Tab never had to scroll the rows");
    }

    /// The scroller takes the panel's width, so its bar sits at the panel's
    /// edge — not at the widest row's, part-way across the window.
    /// Expanding **Customize colors** used to run the Graph swatches off the
    /// right edge of a narrow panel, and that overflow held every wrapped row
    /// at the overflowed width: the chip rows stopped reflowing with the
    /// window. Reported with the section opened in a wide window that was
    /// then narrowed, so the run does the same.
    #[test]
    fn the_expanded_colours_wrap_and_the_other_rows_keep_reflowing() {
        // Narrower than the Graph swatch row and than the Zoom row.
        let width = 700.0;
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        let header = node_bounds(&frame, "Customize colors");
        let centre = Pos2::new(
            ((header.x0 + header.x1) / 2.0) as f32,
            ((header.y0 + header.y1) / 2.0) as f32,
        );
        run.click(centre);
        // The body animates open; a second per frame has it fully open.
        run.frame(vec![]);
        run.frame(vec![]);
        run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 900.0));
        run.frame(vec![]);
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        // The section is open: its first swatch fits whatever the width.
        node_bounds(&frame, "Background");

        // A widget cut off at the edge leaves the AccessKit tree.
        let crosshair = frame
            .nodes
            .iter()
            .find_map(|(_, n)| {
                n.label()
                    .is_some_and(|l| l.starts_with("Crosshair"))
                    .then(|| n.bounds())
            })
            .flatten()
            .expect("the last Graph swatch is cut off by the panel edge");
        assert!(
            crosshair.x1 <= f64::from(width),
            "the last Graph swatch runs off the panel: {crosshair:?}"
        );
        let scrolled = &frame.scrolled;
        assert!(
            scrolled.content_size.x <= scrolled.inner_rect.width(),
            "the rows overflow the panel: content {} wide in {}",
            scrolled.content_size.x,
            scrolled.inner_rect.width()
        );
        for (_, node) in &frame.nodes {
            if let Some(b) = node.bounds() {
                assert!(
                    b.x1 <= f64::from(width),
                    "{:?} runs off the panel: {b:?}",
                    node.label().or(node.value())
                );
            }
        }
        // The Zoom row folded at the window, not at the swatch row.
        let first = node_bounds(&frame, "30%");
        let last = node_bounds(&frame, "(Ctrl+/- to adjust, Ctrl+0 = 100%)");
        assert!(
            last.y0 >= first.y1,
            "the Zoom row did not reflow: 30% at {first:?}, its hint at {last:?}"
        );
    }

    /// The Specifications fields follow the panel checkbox on its row, the
    /// word said once; and the group moves to the next line whole, wrapping
    /// within itself only when a line is too narrow for it.
    #[test]
    fn the_specifications_fields_wrap_as_one_group() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let wide = run.frame(vec![]);
        let graph = node_bounds(&wide, "Graph");
        let specs = node_bounds(&wide, "Specifications:");
        let notes = node_bounds(&wide, "Notes");
        assert_eq!(notes.y0, graph.y0, "the fields left the panel row");
        assert!(
            !wide
                .nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Specifications")),
            "\"Specifications\" is said twice"
        );
        // A window that just fits the row keeps it whole, so the group's
        // measure isn't over; one a little narrower moves the group down
        // whole; one narrower than the group splits it.
        let right_gap = 1200.0 - wide.scrolled.inner_rect.right();
        let group = (notes.x1 - specs.x0) as f32;
        for (width, same_row, split) in [
            ((notes.x1 as f32 + right_gap + 1.0).ceil(), true, false),
            (notes.x1 as f32 + right_gap - 20.0, false, false),
            (graph.x0 as f32 + group * 0.6 + right_gap, false, true),
        ] {
            run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 900.0));
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            let graph = node_bounds(&frame, "Graph");
            let specs = node_bounds(&frame, "Specifications:");
            let notes = node_bounds(&frame, "Notes");
            assert_eq!(specs.y0 == graph.y0, same_row, "at {width}: {specs:?}");
            assert_eq!(notes.y0 > specs.y0, split, "at {width}: {notes:?}");
            assert!(notes.x1 <= f64::from(width), "at {width}: {notes:?}");
        }
    }

    /// With the panel off, its fields are hidden and the checkbox loses the
    /// colon that led into them.
    #[test]
    fn the_specifications_fields_hide_with_the_panel() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.app.settings.show_specs = false;
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        node_bounds(&frame, "Specifications");
        assert!(
            !frame.nodes.iter().any(|(_, n)| n.label() == Some("Notes")),
            "the fields show with the panel off"
        );
    }

    /// A replay's frames come from the file whatever this row names — Connect
    /// re-opens the recording — so the row is pinned the way the clock flags
    /// pin it. Picking a meter used to save the choice and reconnect, and the
    /// session went on playing the file under the name of another meter.
    #[test]
    fn a_replay_pins_the_device_row() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.app.replay = Some(crate::ReplaySource::fixture());
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);

        let disabled = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Device"))
            .map(|(_, n)| n.is_disabled())
            .expect("the Device row is drawn");
        assert!(disabled, "the list is still pickable during a replay");
        // And the row says how to get the choice back.
        node_bounds(&frame, "(restart without --replay to pick a meter)");
    }

    #[test]
    fn the_scroller_spans_the_panel() {
        let frame = settings_panel(400.0, 220.0);
        let right = frame.scrolled.inner_rect.right();
        assert!(right >= 400.0 - 24.0, "scroller ends at {right} pt of 400");
    }
}
