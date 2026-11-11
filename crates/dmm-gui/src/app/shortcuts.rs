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

use std::borrow::Cow;

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Key, Modifiers};

use super::toast::Toast;
use super::{App, ConnectionState};

/// What a key press does, independent of the keys bound to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Shortcut {
    ConnectToggle,
    Quit,
    ClearSession,
    ToggleRecording,
    CycleBigMeter,
    ToggleAlwaysOnTop,
    ToggleDecorations,
    ExportRecording,
    ImportFile,
    ToggleFullscreen,
    Minimize,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Close,
    TogglePause,
    ToggleHelp,
    AddMarker,
    AddMarkerAndNote,
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
        shortcut: Shortcut::ExportRecording,
        os: Os::Any,
    },
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::I,
        shortcut: Shortcut::ImportFile,
        os: Os::Any,
    },
    // Fires inside a note too: from one note, it moves on to the next marker.
    Binding {
        modifiers: Modifiers::COMMAND,
        key: Key::N,
        shortcut: Shortcut::AddMarkerAndNote,
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
        key: Key::N,
        shortcut: Shortcut::AddMarker,
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
        Self::ExportRecording,
        Self::ImportFile,
        Self::AddMarker,
        Self::AddMarkerAndNote,
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

    /// This shortcut's rows of [`BINDINGS`] that apply on this machine, in
    /// table order.
    fn bindings(self, is_mac: bool) -> impl Iterator<Item = &'static Binding> {
        BINDINGS
            .iter()
            .filter(move |b| b.shortcut == self && b.os.applies(is_mac))
    }

    /// The keys that trigger this shortcut, as this machine renders them —
    /// for the help grid and for every tooltip that names a shortcut.
    ///
    /// Rendered from [`BINDINGS`] by [`shortcut_text`], not spelled out: a
    /// rebinding reaches the labels, and the bindings use
    /// `Modifiers::COMMAND`, which is Cmd on macOS, so a literal "Ctrl+O"
    /// would name a key macOS users don't have.
    pub(super) fn keys(self, ctx: &egui::Context, l10n: &Localizer) -> String {
        let is_mac = ctx.os().is_mac();
        // The first row this machine answers to (F11 or the macOS chord for
        // fullscreen, `?` before F1), else the first row anywhere: Minimize
        // still names its key off macOS.
        let first = |shortcut: Self| {
            shortcut
                .bindings(is_mac)
                .next()
                .or_else(|| BINDINGS.iter().find(|b| b.shortcut == shortcut))
                .map_or_else(String::new, |b| render(ctx, l10n, b))
        };
        match self {
            // One pair, named together.
            Self::ZoomIn | Self::ZoomOut => format!(
                "{}{}/{}",
                command_prefix(ctx, l10n),
                tr!(l10n, "shortcuts-key-plus"),
                tr!(l10n, "shortcuts-key-minus")
            ),
            // Escape is egui's own (`Modal::should_close`), so it has no
            // binding to render — only the Ctrl+W half comes from the table.
            Self::Close => format!(
                "{} / {}",
                tr!(l10n, "shortcuts-key-escape"),
                first(Self::Close)
            ),
            // Ctrl+W with the help closed quits too.
            Self::Quit => format!("{} / {}", first(Self::Quit), first(Self::Close)),
            _ => first(self),
        }
    }

    /// The help's two keys, `?` and F1, for the hover that names them both.
    pub(super) fn help_keys(ctx: &egui::Context, l10n: &Localizer) -> (String, String) {
        let f1 = Self::ToggleHelp
            .bindings(ctx.os().is_mac())
            .nth(1)
            .map_or_else(String::new, |b| render(ctx, l10n, b));
        (Self::ToggleHelp.keys(ctx, l10n), f1)
    }

    /// Row in the help grid: `(keys, action, inert)`; `None` for a binding
    /// folded into another row. `inert` marks a key that does nothing in
    /// this session, so the modal can grey the row as well as say so.
    ///
    /// `on_wayland` marks the one key the compositor makes inert.
    fn help_row<'a>(
        self,
        ctx: &egui::Context,
        l10n: &'a Localizer,
        on_wayland: bool,
    ) -> Option<(String, Cow<'a, str>, bool)> {
        let action = match self {
            Self::ConnectToggle => tr!(l10n, "shortcuts-connect"),
            Self::TogglePause => tr!(l10n, "shortcuts-pause"),
            Self::ClearSession => tr!(l10n, "shortcuts-clear"),
            Self::ToggleRecording => tr!(l10n, "shortcuts-record"),
            Self::CycleBigMeter => tr!(l10n, "shortcuts-big-meter"),
            // Same explanation as the settings caption and the toast, so
            // the three surfaces read alike — a test pins the English tail
            // to the toast's.
            Self::ToggleAlwaysOnTop if on_wayland => tr!(l10n, "shortcuts-always-on-top-wayland"),
            Self::ToggleAlwaysOnTop => tr!(l10n, "shortcuts-always-on-top"),
            Self::ToggleDecorations => tr!(l10n, "shortcuts-decorations"),
            Self::ExportRecording => tr!(l10n, "shortcuts-export"),
            Self::ImportFile => tr!(l10n, "shortcuts-import"),
            Self::AddMarker => tr!(l10n, "shortcuts-add-marker"),
            Self::AddMarkerAndNote => tr!(l10n, "shortcuts-add-marker-note"),
            Self::ToggleFullscreen => tr!(l10n, "shortcuts-fullscreen"),
            // macOS only: elsewhere the title bar's own button is the way to
            // minimise, and a row for a key that does nothing would mislead.
            Self::Minimize if !ctx.os().is_mac() => return None,
            Self::Minimize => tr!(l10n, "shortcuts-minimize"),
            Self::ZoomIn => tr!(l10n, "shortcuts-zoom"),
            // Folded into the row above.
            Self::ZoomOut => return None,
            Self::ZoomReset => tr!(l10n, "shortcuts-zoom-reset"),
            // Ctrl+W with the help closed quits, which the Quit row says.
            Self::Close => tr!(l10n, "shortcuts-close-help"),
            Self::Quit => tr!(l10n, "shortcuts-quit"),
            // Not listed: the grid it opens *is* the documentation, and the
            // toolbar's `?` button spells the key out in its tooltip.
            Self::ToggleHelp => return None,
        };
        let key = self.keys(ctx, l10n);
        let inert = on_wayland && self == Self::ToggleAlwaysOnTop;
        Some((key, action, inert))
    }
}

/// `binding`'s keys as this machine writes them, by [`shortcut_text`].
fn render(ctx: &egui::Context, l10n: &Localizer, binding: &Binding) -> String {
    shortcut_text(ctx, l10n, binding.modifiers, binding.key)
}

/// `modifiers`+`key` as this machine writes a shortcut, in the window's
/// language: egui's own rendering (`Context::format_shortcut`), with the
/// modifier and key names it spells out in English looked up instead —
/// Ctrl, Shift, Alt, and the named keys of [`key_name`]. Where egui draws
/// symbols (`⌘`, `⇧` on a macOS whose font has them), the symbols stay.
pub(crate) fn shortcut_text(
    ctx: &egui::Context,
    l10n: &Localizer,
    modifiers: Modifiers,
    key: Key,
) -> String {
    let is_mac = ctx.os().is_mac();
    // egui draws symbols only on a macOS whose font has them: ask it how it
    // writes one shortcut, rather than guessing the font.
    let probe = egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);
    let symbols =
        ctx.format_shortcut(&probe) == probe.format(&egui::ModifierNames::SYMBOLS, is_mac);
    let (ctrl, shift, alt) = (
        tr!(l10n, "shortcuts-key-ctrl"),
        tr!(l10n, "shortcuts-key-shift"),
        tr!(l10n, "shortcuts-key-alt"),
    );
    let names = if symbols {
        egui::ModifierNames::SYMBOLS
    } else {
        egui::ModifierNames {
            ctrl: &ctrl,
            shift: &shift,
            alt: &alt,
            ..egui::ModifierNames::NAMES
        }
    };
    let mut text = names.format(&modifiers, is_mac);
    if !text.is_empty() {
        text.push_str(names.concat);
    }
    let symbol = (key.symbol_or_name() != key.name()).then(|| key.symbol_or_name());
    match (symbol.filter(|_| symbols), key_name(l10n, key)) {
        (Some(symbol), _) => text.push_str(symbol),
        (None, Some(name)) => text.push_str(&name),
        (None, None) => text.push_str(key.name()),
    }
    text
}

/// A named key as this language writes it — Space, Esc, the arrows, Home,
/// End, Plus, Minus, and `?` as itself rather than egui's "Questionmark" —
/// or `None` for one written as egui names it (a letter, a digit, F1).
pub(crate) fn key_name(l10n: &Localizer, key: Key) -> Option<Cow<'_, str>> {
    Some(match key {
        Key::Questionmark => Cow::Borrowed("?"),
        Key::Space => tr!(l10n, "shortcuts-key-space"),
        Key::Escape => tr!(l10n, "shortcuts-key-escape"),
        Key::ArrowLeft => tr!(l10n, "shortcuts-key-left"),
        Key::ArrowRight => tr!(l10n, "shortcuts-key-right"),
        Key::Home => tr!(l10n, "shortcuts-key-home"),
        Key::End => tr!(l10n, "shortcuts-key-end"),
        Key::Plus => tr!(l10n, "shortcuts-key-plus"),
        Key::Minus => tr!(l10n, "shortcuts-key-minus"),
        _ => return None,
    })
}

/// The command modifier as this machine renders it, followed by whatever
/// goes between it and the key: `+` where the modifier is spelled out
/// (`Ctrl+`, or `Cmd+` on a macOS whose body font has no `⌘`), nothing where
/// it is drawn as a symbol (`⌘`). A row that folds two keys into one label
/// has to join them the way the single-key rows above it do.
fn command_prefix(ctx: &egui::Context, l10n: &Localizer) -> String {
    // `Key::Num0` renders as "0" both ways, so trimming it off a formatted
    // Ctrl+0 leaves exactly the modifier and the separator.
    let mut formatted = shortcut_text(ctx, l10n, Modifiers::COMMAND, Key::Num0);
    let prefix_len = formatted.trim_end_matches(Key::Num0.name()).len();
    formatted.truncate(prefix_len);
    formatted
}

/// Consume the presses of `modifiers`+`key`, as `consume_key` does, and say
/// whether one of them was a fresh press rather than the key repeating while
/// held.
fn consume_fresh_key(ctx: &egui::Context, modifiers: Modifiers, key: Key) -> bool {
    ctx.input_mut(|i| {
        let mut fresh = false;
        i.events.retain(|event| match event {
            egui::Event::Key {
                key: k,
                pressed: true,
                repeat,
                modifiers: held,
                ..
            } if *k == key && held.matches_logically(modifiers) => {
                fresh |= !repeat;
                false
            }
            _ => true,
        });
        fresh
    })
}

/// The "General" grid of the shortcut help modal, in display order.
pub(super) fn help_rows<'a>(
    ctx: &'a egui::Context,
    l10n: &'a Localizer,
    on_wayland: bool,
) -> impl Iterator<Item = (String, Cow<'a, str>, bool)> + 'a {
    Shortcut::HELP_ORDER
        .iter()
        .filter_map(move |s| s.help_row(ctx, l10n, on_wayland))
}

impl App {
    /// Whether no modal is up or about to be: egui's modal layer is last
    /// frame's, so a discard prompt asked for earlier in this frame — Ctrl+R
    /// and N in one batch of keys — is checked for too.
    fn no_modal(&self, ctx: &egui::Context) -> bool {
        ctx.memory(|m| m.top_modal_layer()).is_none()
            && self.recording_panel_idle()
            && !self.shortcut_help.open
    }

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
                // Only a text field would type the letter. A focused button
                // has no use for it, and blocking N there would swallow the
                // press after Tab moved on from a note. Neither key reaches
                // past a modal: a marker placed under the discard prompt
                // would change what the prompt is asking about.
                Shortcut::AddMarker => !ctx.text_edit_focused() && self.no_modal(ctx),
                Shortcut::AddMarkerAndNote => self.no_modal(ctx),
                _ => true,
            };
            if !ours {
                continue;
            }
            let pressed = match binding.shortcut {
                // A held key repeats; one press places one marker.
                Shortcut::AddMarker | Shortcut::AddMarkerAndNote => {
                    consume_fresh_key(ctx, binding.modifiers, binding.key)
                }
                _ => ctx.input_mut(|i| i.consume_key(binding.modifiers, binding.key)),
            };
            if !pressed {
                continue;
            }

            // Guards below run *after* the key is consumed: the shortcut is
            // ours either way, it just does nothing while disconnected.
            let connected = self.connection.state == ConnectionState::Connected;

            match binding.shortcut {
                Shortcut::ConnectToggle => match self.connection.state {
                    ConnectionState::Disconnected => self.request_connect(ctx),
                    // Connecting cancels the open and Reconnecting the retry
                    // loop, matching the Cancel and Disconnect buttons shown
                    // in those states.
                    ConnectionState::Connecting
                    | ConnectionState::Connected
                    | ConnectionState::Reconnecting => self.disconnect(),
                },
                // A close request like the window's own, which `hold_close`
                // holds while a recording isn't exported. macOS never gets
                // here: Cmd+Q is the Quit item of winit's default menu,
                // which calls AppKit's `terminate:` (winit-0.30.13
                // `src/platform_impl/macos/menu.rs`) without the app ever
                // seeing the key, so it quits without asking.
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
                        self.toast =
                            Some(Toast::info(tr!(self.l10n, "shortcuts-always-on-top-toast")));
                    } else {
                        self.settings.always_on_top = !self.settings.always_on_top;
                        self.apply_always_on_top(ctx);
                    }
                }
                Shortcut::ToggleDecorations => {
                    self.settings.hide_decorations = !self.settings.hide_decorations;
                    self.apply_decorations(ctx);
                }
                Shortcut::ExportRecording => {
                    self.export_recording(ctx, super::export::ExportFormat::Csv)
                }
                Shortcut::ImportFile => self.begin_import(ctx),
                Shortcut::AddMarker => self.add_marker(false),
                Shortcut::AddMarkerAndNote => self.add_marker(true),
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
    use crate::app::test_support::headless_frame;

    /// Rows are rendered per OS, so the tests that pin their text say which
    /// OS they are pinning. `Nix` is the `Ctrl` rendering — the one the docs
    /// and the literal list below describe.
    fn nix_context() -> egui::Context {
        let ctx = crate::app::styled_context();
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
                binding
                    .shortcut
                    .help_row(&ctx, &crate::l10n::english(), false)
                    .is_some(),
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

    /// Every variant, as an exhaustive match and as an array, so a new
    /// shortcut fails to compile here until it is listed.
    macro_rules! every_shortcut {
        ($($shortcut:ident),* $(,)?) => {{
            let _exhaustive = |s: Shortcut| match s {
                $(Shortcut::$shortcut)|* => {}
            };
            [$(Shortcut::$shortcut),*]
        }};
    }

    /// `Shortcut::keys` names a shortcut by its first binding, so one bound
    /// to nothing would be labelled with nothing — or, for Close and Quit, a
    /// dangling ` / `. Every shortcut has one, `ToggleHelp` and any outside
    /// `HELP_ORDER` included.
    #[test]
    fn every_shortcut_has_a_binding() {
        let every = every_shortcut!(
            ConnectToggle,
            Quit,
            ClearSession,
            ToggleRecording,
            CycleBigMeter,
            ToggleAlwaysOnTop,
            ToggleDecorations,
            ExportRecording,
            ImportFile,
            ToggleFullscreen,
            Minimize,
            ZoomIn,
            ZoomOut,
            ZoomReset,
            Close,
            TogglePause,
            ToggleHelp,
            AddMarker,
            AddMarkerAndNote,
        );
        for shortcut in every {
            assert!(
                BINDINGS.iter().any(|b| b.shortcut == shortcut),
                "{shortcut:?} is bound to no key, so its label is empty"
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
        let l10n = crate::l10n::english();
        let rows: Vec<_> = help_rows(&ctx, &l10n, false).collect();
        assert_eq!(
            rows.iter()
                .map(|(key, action, _)| (key.as_str(), action.as_ref()))
                .collect::<Vec<_>>(),
            vec![
                ("Ctrl+O", "Connect / Disconnect"),
                ("Space", "Pause / Resume"),
                ("Ctrl+L", "Clear graph & statistics"),
                ("Ctrl+R", "Toggle recording"),
                ("Ctrl+E", "Export CSV\u{2026}"),
                ("Ctrl+I", "Import a file\u{2026}"),
                ("N", "Add a marker at the reading on screen"),
                ("Ctrl+N", "Add a marker and write its note"),
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
        let l10n = crate::l10n::english();
        let annotated = "Always on top: not available on Wayland — right-click the title bar to keep the window above others";
        // The toast's explanation, minus its capital first letter, and the
        // settings' word for word.
        let toast = tr!(l10n, "shortcuts-always-on-top-toast");
        assert!(
            annotated.ends_with(&toast[1..]),
            "the Ctrl+T row has drifted from the toast"
        );
        assert_eq!(tr!(l10n, "settings-always-on-top.wayland"), toast);
        assert_eq!(
            tr!(l10n, "settings-always-on-top-wayland"),
            format!("({toast})")
        );
        let expected: Vec<_> = help_rows(&ctx, &l10n, false)
            .map(|(key, action, inert)| {
                assert!(!inert, "{key} is inert off Wayland");
                if key == "Ctrl+T" {
                    (key, Cow::Borrowed(annotated), true)
                } else {
                    (key, action, false)
                }
            })
            .collect();
        assert_eq!(help_rows(&ctx, &l10n, true).collect::<Vec<_>>(), expected);
    }

    /// The help's hover names its keys as the keyboard prints them.
    #[test]
    fn the_help_keys_are_printed_as_on_the_keyboard() {
        let ctx = crate::app::styled_context();
        let l10n = crate::l10n::english();
        let mut keys = (String::new(), String::new());
        headless_frame(&ctx, None, None, vec![], |ui| {
            keys = Shortcut::help_keys(ui.ctx(), &l10n);
        });
        assert_eq!(keys, ("?".to_string(), "F1".to_string()));
    }

    /// The bindings are `Modifiers::COMMAND`, so on macOS they are Cmd —
    /// the grid has to say so, and it has to swap the rows macOS does
    /// differently. Which of the two macOS renderings egui picks depends on
    /// whether the body font carries `⌘`, so accept either, but no row may
    /// still claim a bare Ctrl.
    #[test]
    fn mac_rows_use_cmd() {
        let ctx = crate::app::styled_context();
        ctx.set_os(egui::os::OperatingSystem::Mac);
        // egui asks the fonts whether they can draw `⌘`, and fonts only
        // exist inside a pass.
        let l10n = crate::l10n::english();
        let mut rows = Vec::new();
        headless_frame(&ctx, None, None, vec![], |ui| {
            rows = help_rows(ui.ctx(), &l10n, false)
                .map(|(key, action, inert)| (key, action.into_owned(), inert))
                .collect::<Vec<_>>();
        });

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

    /// Ctrl+O while the meter opens cancels it, as the Cancel button in
    /// Connect's place does, rather than starting a second open.
    #[test]
    fn ctrl_o_cancels_a_connect() {
        let mut settings = crate::settings::Settings::default();
        // The mock: a real open would probe the machine's USB and Bluetooth
        // from a thread that outlives the test.
        settings.shared.device_family = "mock".to_string();
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        let ctx = crate::app::styled_context();
        app.connect(&ctx);
        assert_eq!(app.connection.state, ConnectionState::Connecting);
        headless_frame(
            &ctx,
            None,
            None,
            crate::app::test_support::key_tap_with(Key::O, Modifiers::COMMAND),
            |ui| app.handle_keyboard_shortcuts(ui.ctx()),
        );
        assert_eq!(app.connection.state, ConnectionState::Disconnected);
        assert!(app.connection.worker.is_none(), "no second open");
    }
}
