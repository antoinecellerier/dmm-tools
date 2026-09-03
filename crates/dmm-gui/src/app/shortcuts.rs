//! Application keyboard shortcuts: the binding table, its handler, and the
//! rows the help modal shows.
//!
//! `BINDINGS` is the single source of truth. `handle_keyboard_shortcuts`
//! walks it to dispatch key presses and `help_rows` renders the same
//! shortcuts into the help modal's "General" grid, so a binding cannot be
//! added or retired in one place and forgotten in the other.
//!
//! The graph's own keys (`[`, `]`, arrows, Home/End) are handled in
//! `graph.rs` and documented by their own grid in the modal.

use eframe::egui::{self, Key, Modifiers};

use super::{App, ConnectionState};

/// What a key press does, independent of the keys bound to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shortcut {
    ConnectToggle,
    Quit,
    ClearSession,
    ToggleRecording,
    CycleBigMeter,
    ToggleAlwaysOnTop,
    ToggleDecorations,
    ExportCsv,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    CloseHelp,
    TogglePause,
    ToggleHelp,
}

/// One key combination and the shortcut it triggers.
struct Binding {
    modifiers: Modifiers,
    key: Key,
    shortcut: Shortcut,
}

/// Handler order — Ctrl shortcuts first, then bare keys, exactly as before
/// this table existed.
///
/// `consume_key` matches modifiers *logically*: an extra Shift or Alt on the
/// pressed key is ignored, so Shift+Space would satisfy a bare Space
/// pattern. Most specific first is what keeps that from mis-firing. The help
/// modal reads in a different order — see `Shortcut::HELP_ORDER`.
const BINDINGS: &[Binding] = &[
    // --- Ctrl shortcuts ---
    // Not a Ctrl+C chord: egui-winit turns Ctrl+C (Shift held or not),
    // Ctrl+X and Ctrl+V into clipboard events before egui sees a key, so no
    // binding on them can ever fire. Ctrl+O also stays clear of TextEdit's
    // Ctrl+H/K/U/W and Ctrl+Z/Y.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::O,
        shortcut: Shortcut::ConnectToggle,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Q,
        shortcut: Shortcut::Quit,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::L,
        shortcut: Shortcut::ClearSession,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::R,
        shortcut: Shortcut::ToggleRecording,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::B,
        shortcut: Shortcut::CycleBigMeter,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::T,
        shortcut: Shortcut::ToggleAlwaysOnTop,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::D,
        shortcut: Shortcut::ToggleDecorations,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::E,
        shortcut: Shortcut::ExportCsv,
    },
    // Ctrl++ and Ctrl+= both zoom in: keyboards that need Shift for `+`
    // still report the logical `=`.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Plus,
        shortcut: Shortcut::ZoomIn,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Equals,
        shortcut: Shortcut::ZoomIn,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Minus,
        shortcut: Shortcut::ZoomOut,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Num0,
        shortcut: Shortcut::ZoomReset,
    },
    // Escape is handled natively by `egui::Modal::should_close()` inside
    // `show_shortcut_help`, so only Ctrl+W is bound here. The What's New
    // window is a separate OS viewport and handles its own close.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::W,
        shortcut: Shortcut::CloseHelp,
    },
    // --- Bare-key shortcuts (only when nothing holds keyboard focus) ---
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::Space,
        shortcut: Shortcut::TogglePause,
    },
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::Questionmark,
        shortcut: Shortcut::ToggleHelp,
    },
];

impl Shortcut {
    /// Display order of the help modal's "General" grid.
    ///
    /// Reading order, not handler order: Pause sits next to Connect because
    /// that is the pair a user reaches for first, and Quit is last because
    /// it is the way out.
    const HELP_ORDER: &'static [Self] = &[
        Self::ConnectToggle,
        Self::TogglePause,
        Self::ClearSession,
        Self::ToggleRecording,
        Self::CycleBigMeter,
        Self::ToggleAlwaysOnTop,
        Self::ToggleDecorations,
        Self::ExportCsv,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomReset,
        Self::CloseHelp,
        Self::Quit,
    ];

    /// Row in the help grid: `(keys, action)`; `None` for a binding folded
    /// into another row.
    fn help_row(self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            Self::ConnectToggle => ("Ctrl+O", "Connect / Disconnect"),
            Self::TogglePause => ("Space", "Pause / Resume"),
            Self::ClearSession => ("Ctrl+L", "Clear graph & statistics"),
            Self::ToggleRecording => ("Ctrl+R", "Toggle recording"),
            Self::CycleBigMeter => ("Ctrl+B", "Cycle big meter (off / full / minimal)"),
            Self::ToggleAlwaysOnTop => ("Ctrl+T", "Toggle always on top"),
            Self::ToggleDecorations => ("Ctrl+D", "Toggle window decorations"),
            Self::ExportCsv => ("Ctrl+E", "Export CSV"),
            Self::ZoomIn => ("Ctrl+Plus/Minus", "Zoom in / out"),
            // Folded into the row above.
            Self::ZoomOut => return None,
            Self::ZoomReset => ("Ctrl+0", "Reset zoom to 100%"),
            Self::CloseHelp => ("Esc / Ctrl+W", "Close this help"),
            Self::Quit => ("Ctrl+Q", "Quit"),
            // Not listed: the grid it opens *is* the documentation, and the
            // toolbar's `?` button spells the key out in its tooltip.
            Self::ToggleHelp => return None,
        })
    }
}

/// The "General" grid of the shortcut help modal, in display order.
pub(super) fn help_rows() -> impl Iterator<Item = (&'static str, &'static str)> {
    Shortcut::HELP_ORDER.iter().filter_map(|s| s.help_row())
}

impl App {
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // `egui_wants_keyboard_input()` is any focused widget, not just a
        // TextEdit — and that is what we want: Space has to activate the
        // focused button rather than also toggling pause, and arrow keys have
        // to drive the focused widget rather than panning the graph.
        // `text_edit_focused()` would be the text-only predicate.
        let wants_keyboard_input = ctx.egui_wants_keyboard_input();

        for binding in BINDINGS {
            // Guards that decide whether the key press is *consumed* at all:
            // a bare key has to stay available to the focused widget, and
            // Ctrl+W must fall through while the help modal is closed.
            let ours = match binding.shortcut {
                Shortcut::TogglePause | Shortcut::ToggleHelp => !wants_keyboard_input,
                Shortcut::CloseHelp => self.shortcut_help.open,
                _ => true,
            };
            if !ours || !ctx.input_mut(|i| i.consume_key(binding.modifiers, binding.key)) {
                continue;
            }

            // Guards below run *after* the key is consumed: the shortcut is
            // ours either way, it just does nothing while disconnected.
            let connected = self.connection.state == ConnectionState::Connected;

            match binding.shortcut {
                Shortcut::ConnectToggle => match self.connection.state {
                    ConnectionState::Disconnected => self.connect(ctx),
                    // Reconnecting cancels the retry loop, matching the
                    // Disconnect button shown in that state.
                    ConnectionState::Connected | ConnectionState::Reconnecting => self.disconnect(),
                },
                Shortcut::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Shortcut::ClearSession => {
                    if connected {
                        self.clear_session();
                    }
                }
                Shortcut::ToggleRecording => {
                    if connected {
                        self.toggle_recording();
                    }
                }
                Shortcut::CycleBigMeter => self.cycle_big_meter(),
                Shortcut::ToggleAlwaysOnTop => {
                    self.settings.always_on_top = !self.settings.always_on_top;
                    self.apply_always_on_top(ctx);
                    self.settings.save();
                }
                Shortcut::ToggleDecorations => {
                    self.settings.hide_decorations = !self.settings.hide_decorations;
                    self.apply_decorations(ctx);
                    self.settings.save();
                }
                Shortcut::ExportCsv => self.export_csv(),
                Shortcut::ZoomIn => self.zoom_in(),
                Shortcut::ZoomOut => self.zoom_out(),
                Shortcut::ZoomReset => self.zoom_reset(),
                Shortcut::CloseHelp => {
                    self.shortcut_help.open = false;
                    // Defer focus restoration until after top_modal_layer
                    // clears — same reason as the in-modal close path in
                    // `show_shortcut_help`.
                    self.shortcut_help.restore_focus = self.shortcut_help.opener.take();
                }
                Shortcut::TogglePause => {
                    if connected {
                        self.set_paused(!self.connection.paused);
                    }
                }
                Shortcut::ToggleHelp => {
                    let will_open = !self.shortcut_help.open;
                    self.shortcut_help.open = will_open;
                    if will_open {
                        // Capture whatever widget currently has focus so we
                        // can restore to it when the modal closes. Can't rely
                        // on "egui will retain focus" — Focus::begin_pass
                        // clears focused_widget unconditionally when it sees
                        // Escape, so without an explicit opener the next Tab
                        // lands on the first widget in the top bar instead of
                        // the one the user was on.
                        self.shortcut_help.opener = ctx.memory(|m| m.focused());
                        self.shortcut_help.focus_pending = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The modal is where these keys are documented, so a new binding has to
    /// bring a row with it — or be one of the two deliberate omissions. The
    /// reverse direction matters just as much: `HELP_ORDER` is a second list,
    /// and a row that documents a key nothing binds is worse than no row.
    #[test]
    fn every_shortcut_has_a_help_row_or_is_folded() {
        for binding in BINDINGS {
            let folded = matches!(
                binding.shortcut,
                // Shares "Ctrl+Plus/Minus" with ZoomIn.
                Shortcut::ZoomOut
                // Documented by the toolbar `?` button's tooltip instead.
                    | Shortcut::ToggleHelp
            );
            assert_eq!(
                binding.shortcut.help_row().is_some(),
                !folded,
                "{:?} help row disagrees with the folded list",
                binding.shortcut
            );
            if !folded {
                assert!(
                    Shortcut::HELP_ORDER.contains(&binding.shortcut),
                    "{:?} has a help row but never reaches the grid",
                    binding.shortcut
                );
            }
        }

        for shortcut in Shortcut::HELP_ORDER {
            assert!(
                BINDINGS.iter().any(|b| b.shortcut == *shortcut),
                "{shortcut:?} is documented but bound to no key"
            );
            assert_eq!(
                Shortcut::HELP_ORDER
                    .iter()
                    .filter(|s| *s == shortcut)
                    .count(),
                1,
                "{shortcut:?} is listed twice in the help grid"
            );
        }
    }

    /// Pinned literally: the grid is user-facing text, and it used to be a
    /// literal list in `show_shortcut_help`.
    #[test]
    fn help_rows_are_the_twelve_documented_general_rows() {
        assert_eq!(
            help_rows().collect::<Vec<_>>(),
            vec![
                ("Ctrl+O", "Connect / Disconnect"),
                ("Space", "Pause / Resume"),
                ("Ctrl+L", "Clear graph & statistics"),
                ("Ctrl+R", "Toggle recording"),
                ("Ctrl+B", "Cycle big meter (off / full / minimal)"),
                ("Ctrl+T", "Toggle always on top"),
                ("Ctrl+D", "Toggle window decorations"),
                ("Ctrl+E", "Export CSV"),
                ("Ctrl+Plus/Minus", "Zoom in / out"),
                ("Ctrl+0", "Reset zoom to 100%"),
                ("Esc / Ctrl+W", "Close this help"),
                ("Ctrl+Q", "Quit"),
            ]
        );
    }
}
