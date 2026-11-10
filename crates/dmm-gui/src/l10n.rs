//! The resources dmm-gui shows, and the localizers built from them.
//!
//! `main` builds the session's first localizer ([`startup`]) before it
//! parses the command line, so `--help` and the errors before the window
//! opens speak the language the window will; the window then keeps it in
//! [`App`](crate::app::App) as a handle, swapped in `app/language.rs`.

use dmm_shared::l10n::{Localizer, Options, Resource};
use std::sync::{Arc, OnceLock};

/// dmm-gui's own messages, one file per screen area under
/// `i18n/<locale>/gui/`.
const GUI: &[Resource] = &[
    dmm_shared::ftl!("en-US", "gui/top-bar.ftl"),
    dmm_shared::ftl!("en-US", "gui/connection.ftl"),
    dmm_shared::ftl!("en-US", "gui/settings.ftl"),
    dmm_shared::ftl!("en-US", "gui/colors.ftl"),
    dmm_shared::ftl!("en-US", "gui/recording.ftl"),
    dmm_shared::ftl!("en-US", "gui/export-import.ftl"),
    dmm_shared::ftl!("en-US", "gui/graph.ftl"),
    dmm_shared::ftl!("en-US", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("en-US", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("en-US", "gui/reading.ftl"),
    dmm_shared::ftl!("en-US", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("en-US", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("en-US", "gui/desktop.ftl"),
    dmm_shared::ftl!("en-US", "gui/command-line.ftl"),
    dmm_shared::ftl!("fr", "gui/top-bar.ftl"),
    dmm_shared::ftl!("fr", "gui/connection.ftl"),
    dmm_shared::ftl!("fr", "gui/settings.ftl"),
    dmm_shared::ftl!("fr", "gui/colors.ftl"),
    dmm_shared::ftl!("fr", "gui/recording.ftl"),
    dmm_shared::ftl!("fr", "gui/export-import.ftl"),
    dmm_shared::ftl!("fr", "gui/graph.ftl"),
    dmm_shared::ftl!("fr", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("fr", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("fr", "gui/reading.ftl"),
    dmm_shared::ftl!("fr", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("fr", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("fr", "gui/desktop.ftl"),
    dmm_shared::ftl!("fr", "gui/command-line.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/top-bar.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/connection.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/settings.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/colors.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/recording.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/export-import.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/graph.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/reading.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/desktop.ftl"),
    dmm_shared::ftl!("pt-BR", "gui/command-line.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/top-bar.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/connection.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/settings.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/colors.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/recording.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/export-import.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/graph.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/reading.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/desktop.ftl"),
    dmm_shared::ftl!("zh-CN", "gui/command-line.ftl"),
    dmm_shared::ftl!("es", "gui/top-bar.ftl"),
    dmm_shared::ftl!("es", "gui/connection.ftl"),
    dmm_shared::ftl!("es", "gui/settings.ftl"),
    dmm_shared::ftl!("es", "gui/colors.ftl"),
    dmm_shared::ftl!("es", "gui/recording.ftl"),
    dmm_shared::ftl!("es", "gui/export-import.ftl"),
    dmm_shared::ftl!("es", "gui/graph.ftl"),
    dmm_shared::ftl!("es", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("es", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("es", "gui/reading.ftl"),
    dmm_shared::ftl!("es", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("es", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("es", "gui/desktop.ftl"),
    dmm_shared::ftl!("es", "gui/command-line.ftl"),
    dmm_shared::ftl!("ja", "gui/top-bar.ftl"),
    dmm_shared::ftl!("ja", "gui/connection.ftl"),
    dmm_shared::ftl!("ja", "gui/settings.ftl"),
    dmm_shared::ftl!("ja", "gui/colors.ftl"),
    dmm_shared::ftl!("ja", "gui/recording.ftl"),
    dmm_shared::ftl!("ja", "gui/export-import.ftl"),
    dmm_shared::ftl!("ja", "gui/graph.ftl"),
    dmm_shared::ftl!("ja", "gui/stats-markers.ftl"),
    dmm_shared::ftl!("ja", "gui/shortcuts.ftl"),
    dmm_shared::ftl!("ja", "gui/reading.ftl"),
    dmm_shared::ftl!("ja", "gui/alarm-scale.ftl"),
    dmm_shared::ftl!("ja", "gui/specs-remote.ftl"),
    dmm_shared::ftl!("ja", "gui/desktop.ftl"),
    dmm_shared::ftl!("ja", "gui/command-line.ftl"),
];

/// What each option does, as `--help` says it: generated from the doc
/// comments in `main.rs` by the `gui_help_ftl` test, so the ID scan leaves
/// it out of its unused check, as it does `lib.ftl`.
const GUI_HELP: &[Resource] = &[
    dmm_shared::ftl!("en-US", "gui-help.ftl"),
    dmm_shared::ftl!("fr", "gui-help.ftl"),
    dmm_shared::ftl!("pt-BR", "gui-help.ftl"),
    dmm_shared::ftl!("zh-CN", "gui-help.ftl"),
    dmm_shared::ftl!("es", "gui-help.ftl"),
    dmm_shared::ftl!("ja", "gui-help.ftl"),
];

/// Everything the GUI shows: the shared help, the library's text and
/// errors, its own messages and its `--help`.
pub(crate) fn resources() -> Vec<Resource> {
    [
        dmm_shared::help::RESOURCES,
        dmm_shared::l10n::LIB_RESOURCES,
        GUI,
        GUI_HELP,
    ]
    .concat()
}

/// The localizer a session starts in: the language the settings file names
/// (`setting`, empty: follow the system), else the environment's. Built once
/// by `main`, which prints `--help` in it, then handed to the window.
pub(crate) fn startup(setting: &str) -> Localizer {
    Localizer::new(
        dmm_shared::l10n::resolve_from_env(None, setting),
        &resources(),
        &Options::from_env(),
    )
}

/// [`resources`] in en-US only: what a test that makes up its own
/// translation starts from, so the shipped ones don't stand in for it.
#[cfg(test)]
pub(crate) fn english_resources() -> Vec<Resource> {
    resources()
        .into_iter()
        .filter(|r| r.locale == dmm_shared::l10n::EN_US.id)
        .collect()
}

/// The desktop entry's resources, in every language: all a localizer for
/// it needs, so writing it at launch parses one small file per language.
#[cfg(target_os = "linux")]
pub(crate) fn desktop_resources() -> Vec<Resource> {
    gui_file("gui/desktop.ftl")
}

/// The settings panel's resources, in every language: all the localizers
/// that mark each language of the **Language** list in its own words need.
pub(crate) fn settings_resources() -> Vec<Resource> {
    gui_file("gui/settings.ftl")
}

/// One of dmm-gui's files, `path` under `i18n/<locale>/`, in every language.
fn gui_file(path: &str) -> Vec<Resource> {
    GUI.iter().filter(|r| r.path == path).copied().collect()
}

/// English, shared: what tests and an app built without a window speak, and
/// what the log quotes a theme file's reason in.
pub(crate) fn english() -> Arc<Localizer> {
    static ENGLISH: OnceLock<Arc<Localizer>> = OnceLock::new();
    ENGLISH
        .get_or_init(|| Arc::new(Localizer::en_us(&resources())))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every lookup in dmm-gui names an en-US message with the variables it
    /// is given, and every message in its own files is used.
    #[test]
    fn the_messages_match_their_uses() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let owned: Vec<&str> = GUI.iter().map(|r| r.path).collect();
        let problems = dmm_shared::l10n::check::check(&src, &resources(), &owned);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A session, and its `--help`, start in the language the settings
    /// name, whatever the system speaks.
    #[test]
    fn startup_speaks_the_setting() {
        assert_eq!(startup("fr").language().id, "fr");
        assert_eq!(startup("en-US").language().id, "en-US");
    }
}
