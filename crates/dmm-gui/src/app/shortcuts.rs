//! Application keyboard shortcuts: the binding table, its handler, and the
//! rows the help modal shows.
//!
//! `BINDINGS` is the single source of truth. `handle_keyboard_shortcuts`
//! walks it to dispatch key presses and `help_rows` renders the same
//! shortcuts into the help modal's "General" grid, so a binding cannot be
//! added or retired in one place and forgotten in the other.
//!
//! The graph's own keys (`[`, `]`, arrows, Home/End, and the overlay
//! toggles) are handled in `graph::view` and documented by their own grid in
//! the modal.

use eframe::egui::{self, Key, Modifiers};

use super::appearance::ALWAYS_ON_TOP_WAYLAND_HINT;
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
    ToggleFullscreen,
    Minimize,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Close,
    TogglePause,
    ToggleHelp,
}

/// The machines a binding applies to.
///
/// Window keys are the one place the platforms disagree outright: `F11` is
/// Show Desktop on macOS and fullscreen everywhere else, and the macOS
/// fullscreen chord collapses to a plain `Ctrl+F` on Linux and Windows
/// (`Modifiers::COMMAND` *is* Ctrl there). Neither can simply be bound
/// everywhere, so a binding says where it is live.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Os {
    Any,
    Mac,
    NotMac,
}

impl Os {
    fn applies(self, is_mac: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Mac => is_mac,
            Self::NotMac => !is_mac,
        }
    }
}

/// One key combination and the shortcut it triggers.
struct Binding {
    modifiers: Modifiers,
    key: Key,
    shortcut: Shortcut,
    os: Os,
}

/// macOS fullscreen: `Ctrl+Cmd+F`.
///
/// `MAC_CMD` rather than `COMMAND` on purpose. `consume_key` matches with
/// `Modifiers::matches_logically`, whose `mac_cmd` branch demands the real ⌘
/// key and an exact `ctrl` match — so this fires on `Ctrl+Cmd+F` and on
/// nothing else, and `mac_cmd` is a flag egui-winit only ever sets on macOS,
/// which keeps the chord inert elsewhere even if the `Os` filter is lost.
/// `CTRL | COMMAND` would take a bare `Ctrl+F` on Linux and Windows.
const MAC_FULLSCREEN: Modifiers = Modifiers::CTRL.plus(Modifiers::MAC_CMD);

/// Handler order — Ctrl shortcuts first, then bare keys, exactly as before
/// this table existed.
///
/// `consume_key` matches modifiers *logically*: an extra Shift or Alt on the
/// pressed key is ignored, so Shift+Space would satisfy a bare Space
/// pattern. Most specific first is what keeps that from mis-firing. The help
/// modal reads in a different order — see `Shortcut::HELP_ORDER`.
const BINDINGS: &[Binding] = &[
    // --- macOS window chords ---
    // Ahead of the Command shortcuts below: both patterns are the more
    // specific of their kind, and a ⌘ press sets `command` as well as
    // `mac_cmd`, so a Command binding on the same key would swallow these.
    Binding {
        modifiers: MAC_FULLSCREEN,
        key: Key::F,
        shortcut: Shortcut::ToggleFullscreen,
        os: Os::Mac,
    },
    Binding {
        modifiers: Modifiers::MAC_CMD,
        key: Key::M,
        shortcut: Shortcut::Minimize,
        os: Os::Mac,
    },
    // --- Ctrl shortcuts ---
    // Not a Ctrl+C chord: egui-winit turns Ctrl+C (Shift held or not),
    // Ctrl+X and Ctrl+V into clipboard events before egui sees a key, so no
    // binding on them can ever fire. Ctrl+O also stays clear of TextEdit's
    // Ctrl+H/K/U/W and Ctrl+Z/Y — Ctrl+W is bound below, but only takes the
    // key when no text field has focus.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::O,
        shortcut: Shortcut::ConnectToggle,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Q,
        shortcut: Shortcut::Quit,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::L,
        shortcut: Shortcut::ClearSession,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::R,
        shortcut: Shortcut::ToggleRecording,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::B,
        shortcut: Shortcut::CycleBigMeter,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::T,
        shortcut: Shortcut::ToggleAlwaysOnTop,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::D,
        shortcut: Shortcut::ToggleDecorations,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::E,
        shortcut: Shortcut::ExportCsv,
        os: Os::Any,
    },
    // Ctrl++ and Ctrl+= both zoom in: keyboards that need Shift for `+`
    // still report the logical `=`.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Plus,
        shortcut: Shortcut::ZoomIn,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Equals,
        shortcut: Shortcut::ZoomIn,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Minus,
        shortcut: Shortcut::ZoomOut,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::Num0,
        shortcut: Shortcut::ZoomReset,
        os: Os::Any,
    },
    // The close key of every platform: it shuts the help modal if that is
    // what is open, and otherwise the window — which for a single-window
    // app is the way out. Escape closes the modal too, handled natively by
    // `egui::Modal::should_close()` inside `show_shortcut_help`, so only
    // Ctrl+W is bound here. The What's New window is a separate OS viewport
    // and handles its own close.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::W,
        shortcut: Shortcut::Close,
        os: Os::Any,
    },
    // --- Bare-key shortcuts ---
    // Space and `?` are printable, so they only fire while nothing holds
    // keyboard focus; the function keys are ours whatever has focus.
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::Space,
        shortcut: Shortcut::TogglePause,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::Questionmark,
        shortcut: Shortcut::ToggleHelp,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::F1,
        shortcut: Shortcut::ToggleHelp,
        os: Os::Any,
    },
    // F11 is Show Desktop on macOS, where `Ctrl+Cmd+F` above does this.
    Binding {
        modifiers: Modifiers::NONE,
        key: Key::F11,
        shortcut: Shortcut::ToggleFullscreen,
        os: Os::NotMac,
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
        Self::ExportCsv,
        Self::CycleBigMeter,
        Self::ToggleAlwaysOnTop,
        Self::ToggleDecorations,
        Self::ToggleFullscreen,
        Self::Minimize,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomReset,
        Self::Close,
        Self::Quit,
    ];

    /// Row in the help grid: `(keys, action, inert)`; `None` for a binding
    /// folded into another row. `inert` marks a key that does nothing in
    /// this session, so the modal can grey the row as well as say so.
    ///
    /// The keys are rendered by `ctx`, not spelled out here: the bindings
    /// use `Modifiers::COMMAND`, which is Cmd on macOS, so a literal
    /// "Ctrl+O" would name a key macOS users don't have.
    ///
    /// `on_wayland` marks the one key the compositor makes inert.
    fn help_row(
        self,
        ctx: &egui::Context,
        on_wayland: bool,
    ) -> Option<(String, &'static str, bool)> {
        let keys = |modifiers: Modifiers, key: Key| {
            ctx.format_shortcut(&egui::KeyboardShortcut::new(modifiers, key))
        };
        let (key, action) = match self {
            Self::ConnectToggle => (keys(Modifiers::COMMAND, Key::O), "Connect / Disconnect"),
            Self::TogglePause => (keys(Modifiers::NONE, Key::Space), "Pause / Resume"),
            Self::ClearSession => (keys(Modifiers::COMMAND, Key::L), "Clear graph & statistics"),
            Self::ToggleRecording => (keys(Modifiers::COMMAND, Key::R), "Toggle recording"),
            Self::CycleBigMeter => (
                keys(Modifiers::COMMAND, Key::B),
                "Cycle big meter (off / full / minimal)",
            ),
            Self::ToggleAlwaysOnTop => (
                keys(Modifiers::COMMAND, Key::T),
                if on_wayland {
                    // Same explanation as the settings caption and the
                    // toast, so the three surfaces read alike — a test
                    // pins the tail to `ALWAYS_ON_TOP_WAYLAND_HINT`.
                    "Always on top: not available on Wayland — right-click the title bar to keep the window above others"
                } else {
                    "Toggle always on top"
                },
            ),
            Self::ToggleDecorations => (
                keys(Modifiers::COMMAND, Key::D),
                "Toggle window decorations",
            ),
            Self::ExportCsv => (keys(Modifiers::COMMAND, Key::E), "Export CSV"),
            // Two bindings, one per OS — the row shows the one this machine
            // answers to rather than both.
            Self::ToggleFullscreen => (
                if ctx.os().is_mac() {
                    keys(MAC_FULLSCREEN, Key::F)
                } else {
                    keys(Modifiers::NONE, Key::F11)
                },
                "Toggle fullscreen",
            ),
            // macOS only: elsewhere the title bar's own button is the way to
            // minimise, and a row for a key that does nothing would mislead.
            Self::Minimize if !ctx.os().is_mac() => return None,
            Self::Minimize => (keys(Modifiers::MAC_CMD, Key::M), "Minimise window"),
            Self::ZoomIn => (
                format!("{}Plus/Minus", command_prefix(ctx)),
                "Zoom in / out",
            ),
            // Folded into the row above.
            Self::ZoomOut => return None,
            Self::ZoomReset => (keys(Modifiers::COMMAND, Key::Num0), "Reset zoom to 100%"),
            // Escape is egui's own (`Modal::should_close`), so it has no
            // binding to render — only the Ctrl+W half comes from the table.
            // Ctrl+W with the help closed quits, which the Quit row says.
            Self::Close => (
                format!("Esc / {}", keys(Modifiers::COMMAND, Key::W)),
                "Close this help",
            ),
            Self::Quit => (
                format!(
                    "{} / {}",
                    keys(Modifiers::COMMAND, Key::Q),
                    keys(Modifiers::COMMAND, Key::W)
                ),
                "Quit",
            ),
            // Not listed: the grid it opens *is* the documentation, and the
            // toolbar's `?` button spells the key out in its tooltip.
            Self::ToggleHelp => return None,
        };
        let inert = on_wayland && self == Self::ToggleAlwaysOnTop;
        Some((key, action, inert))
    }
}

/// The command modifier as this machine renders it, followed by whatever
/// egui puts between it and the key: `+` where the modifier is spelled out
/// (`Ctrl+`, or `Cmd+` on a macOS whose body font has no `⌘`), nothing where
/// it is drawn as a symbol (`⌘`). A row that folds two keys into one label
/// has to join them the way the single-key rows above it do.
fn command_prefix(ctx: &egui::Context) -> String {
    // `Key::Num0` renders as "0" both ways, so trimming it off a formatted
    // Ctrl+0 leaves exactly the modifier and the separator.
    let mut formatted =
        ctx.format_shortcut(&egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0));
    let prefix_len = formatted.trim_end_matches(Key::Num0.name()).len();
    formatted.truncate(prefix_len);
    formatted
}

/// The "General" grid of the shortcut help modal, in display order.
pub(super) fn help_rows(
    ctx: &egui::Context,
    on_wayland: bool,
) -> impl Iterator<Item = (String, &'static str, bool)> {
    Shortcut::HELP_ORDER
        .iter()
        .filter_map(move |s| s.help_row(ctx, on_wayland))
}

impl App {
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // `egui_wants_keyboard_input()` is any focused widget, not just a
        // TextEdit — and that is what we want for the printable keys: Space
        // has to activate the focused button rather than also toggling pause,
        // and arrow keys have to drive the focused widget rather than panning
        // the graph. The Ctrl+W guard below wants the narrower
        // `text_edit_focused()` instead, for the reason given there.
        let wants_keyboard_input = ctx.egui_wants_keyboard_input();
        let is_mac = ctx.os().is_mac();

        for binding in BINDINGS {
            if !binding.os.applies(is_mac) {
                continue;
            }
            // Guards that decide whether the key press is *consumed* at all:
            // a printable key has to stay available to the focused widget,
            // and Ctrl+W has to stay available to a focused text field.
            let ours = match binding.shortcut {
                Shortcut::TogglePause | Shortcut::ToggleHelp => match binding.key {
                    Key::Space | Key::Questionmark => !wants_keyboard_input,
                    // F1 is not a character anything can type, so it opens
                    // the help whatever has focus.
                    _ => true,
                },
                // Only a text field owns Ctrl+W, where it deletes the
                // previous word. A focused button must not block the window
                // from closing — so this is `text_edit_focused()`, not the
                // any-widget `egui_wants_keyboard_input()`.
                Shortcut::Close => self.shortcut_help.open || !ctx.text_edit_focused(),
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
                // macOS never gets here: Cmd+Q is the Quit item of winit's
                // default menu, which calls AppKit's `terminate:`
                // (winit-0.30.13 `src/platform_impl/macos/menu.rs`) without
                // the app ever seeing the key. A confirm-on-quit built on
                // `ViewportCommand::CancelClose` would have to cover that
                // path separately.
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
                    if self.on_wayland {
                        // Flipping the setting there moved a checkbox and
                        // nothing else; say so instead.
                        self.toast = Some((
                            ALWAYS_ON_TOP_WAYLAND_HINT.to_string(),
                            false,
                            std::time::Instant::now(),
                        ));
                    } else {
                        self.settings.always_on_top = !self.settings.always_on_top;
                        self.apply_always_on_top(ctx);
                        self.settings.save();
                    }
                }
                Shortcut::ToggleDecorations => {
                    self.settings.hide_decorations = !self.settings.hide_decorations;
                    self.apply_decorations(ctx);
                    self.settings.save();
                }
                Shortcut::ExportCsv => self.export_csv(),
                // Transient window state, deliberately not saved in settings:
                // a session that ended fullscreen should not reopen that way.
                Shortcut::ToggleFullscreen => {
                    let on = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!on));
                }
                // One-way: the window manager restores the window, so there
                // is no un-minimise key to pair with this.
                Shortcut::Minimize => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                Shortcut::ZoomIn => self.zoom_in(),
                Shortcut::ZoomOut => self.zoom_out(),
                Shortcut::ZoomReset => self.zoom_reset(),
                Shortcut::Close => {
                    if self.shortcut_help.open {
                        self.shortcut_help.open = false;
                        // Defer focus restoration until after top_modal_layer
                        // clears — same reason as the in-modal close path in
                        // `show_shortcut_help`.
                        self.shortcut_help.restore_focus = self.shortcut_help.opener.take();
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
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

    /// Rows are rendered per OS, so the tests that pin their text say which
    /// OS they are pinning. `Nix` is the `Ctrl` rendering — the one the docs
    /// and the literal list below describe.
    fn nix_context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx
    }

    /// The modal is where these keys are documented, so a new binding has to
    /// bring a row with it — or be one of the two deliberate omissions. The
    /// reverse direction matters just as much: `HELP_ORDER` is a second list,
    /// and a row that documents a key nothing binds is worse than no row.
    #[test]
    fn every_shortcut_has_a_help_row_or_is_folded() {
        let ctx = nix_context();
        // Rows are per OS too: the grid this asserts on is the one a Linux
        // or Windows machine draws, so the macOS-only bindings are not in it.
        for binding in BINDINGS.iter().filter(|b| b.os.applies(false)) {
            let folded = matches!(
                binding.shortcut,
                // Shares "Ctrl+Plus/Minus" with ZoomIn.
                Shortcut::ZoomOut
                // Documented by the toolbar `?` button's tooltip instead.
                    | Shortcut::ToggleHelp
            );
            assert_eq!(
                binding.shortcut.help_row(&ctx, false).is_some(),
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

    /// The table is walked in order and the first match wins, so a key bound
    /// twice on one machine gives the second binding no way to fire. The two
    /// fullscreen bindings are the case this has to allow: same key combo is
    /// what is forbidden, and those two differ in both.
    #[test]
    fn no_key_is_bound_twice_on_one_os() {
        for is_mac in [false, true] {
            let mut seen: Vec<(Modifiers, Key)> = Vec::new();
            for binding in BINDINGS.iter().filter(|b| b.os.applies(is_mac)) {
                let combo = (binding.modifiers, binding.key);
                assert!(
                    !seen.contains(&combo),
                    "{:?} + {:?} is bound twice with is_mac = {is_mac}",
                    binding.modifiers,
                    binding.key
                );
                seen.push(combo);
            }
        }
    }

    /// Pinned literally: the grid is user-facing text, and it used to be a
    /// literal list in `show_shortcut_help`.
    #[test]
    fn help_rows_are_the_documented_general_rows() {
        let ctx = nix_context();
        let rows: Vec<_> = help_rows(&ctx, false).collect();
        assert_eq!(
            rows.iter()
                .map(|(key, action, _)| (key.as_str(), *action))
                .collect::<Vec<_>>(),
            vec![
                ("Ctrl+O", "Connect / Disconnect"),
                ("Space", "Pause / Resume"),
                ("Ctrl+L", "Clear graph & statistics"),
                ("Ctrl+R", "Toggle recording"),
                ("Ctrl+E", "Export CSV"),
                ("Ctrl+B", "Cycle big meter (off / full / minimal)"),
                ("Ctrl+T", "Toggle always on top"),
                ("Ctrl+D", "Toggle window decorations"),
                ("F11", "Toggle fullscreen"),
                ("Ctrl+Plus/Minus", "Zoom in / out"),
                ("Ctrl+0", "Reset zoom to 100%"),
                ("Esc / Ctrl+W", "Close this help"),
                ("Ctrl+Q / Ctrl+W", "Quit"),
            ]
        );
    }

    /// On Wayland the grid explains the one key the compositor ignores, greys
    /// it, and changes nothing else, so the modal doesn't quietly diverge per
    /// session. The explanation is the settings caption's, word for word.
    #[test]
    fn wayland_annotates_only_the_always_on_top_row() {
        let ctx = nix_context();
        let annotated = "Always on top: not available on Wayland — right-click the title bar to keep the window above others";
        // The shared hint, minus its capital first letter.
        assert!(
            annotated.ends_with(&ALWAYS_ON_TOP_WAYLAND_HINT[1..]),
            "the Ctrl+T row has drifted from ALWAYS_ON_TOP_WAYLAND_HINT"
        );
        let expected: Vec<_> = help_rows(&ctx, false)
            .map(|(key, action, inert)| {
                assert!(!inert, "{key} is inert off Wayland");
                if key == "Ctrl+T" {
                    (key, annotated, true)
                } else {
                    (key, action, false)
                }
            })
            .collect();
        assert_eq!(help_rows(&ctx, true).collect::<Vec<_>>(), expected);
    }

    /// The bindings are `Modifiers::COMMAND`, so on macOS they are Cmd —
    /// the grid has to say so, and it has to swap the rows macOS does
    /// differently. Which of the two macOS renderings egui picks depends on
    /// whether the body font carries `⌘`, so accept either, but no row may
    /// still claim a bare Ctrl.
    #[test]
    fn mac_rows_use_cmd() {
        let ctx = egui::Context::default();
        ctx.set_os(egui::os::OperatingSystem::Mac);
        // egui asks the fonts whether they can draw `⌘`, and fonts only
        // exist inside a pass.
        let mut rows = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            rows = help_rows(ui.ctx(), false).collect::<Vec<_>>();
        });
        // The pass built a font atlas that a real app would upload; nothing
        // here paints, and TexturesDelta panics if it is dropped unapplied.
        output.textures_delta.clear();

        let (connect, _, _) = &rows[0];
        assert!(
            connect == "⌘O" || connect == "Cmd+O",
            "Connect row reads {connect:?} on macOS"
        );
        for (key, _, _) in &rows {
            // Macs do have a Control key, and the fullscreen chord uses it —
            // but only ever alongside ⌘. Ctrl on its own means a COMMAND
            // binding was rendered the Linux way.
            assert!(
                !key.contains("Ctrl") || key.contains("Cmd"),
                "{key:?} still names Ctrl on macOS"
            );
            assert!(!key.contains("F11"), "{key:?} is Show Desktop on macOS");
        }
        let minimise = rows
            .iter()
            .find(|(_, action, _)| *action == "Minimise window");
        let Some((key, _, _)) = minimise else {
            panic!("macOS has no Window menu, so the grid has to carry the minimise key");
        };
        assert!(
            key == "⌘M" || key == "Cmd+M",
            "Minimise row reads {key:?} on macOS"
        );
    }
}
