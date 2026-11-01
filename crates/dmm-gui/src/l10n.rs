//! The resources dmm-gui shows, and the localizers built from them.
//!
//! The app keeps its localizer in [`App`](crate::app::App) as a handle,
//! built and swapped in `app/language.rs`; text drawn outside it (`--help`,
//! errors before the window opens) is English.

use dmm_shared::l10n::{Localizer, Resource};
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
];

/// Everything the GUI shows: the shared help, the library's text and
/// errors, and its own messages.
pub(crate) fn resources() -> Vec<Resource> {
    [
        dmm_shared::help::RESOURCES,
        dmm_shared::l10n::LIB_RESOURCES,
        GUI,
    ]
    .concat()
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

/// English, shared: what tests and an app built without a window speak.
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
}
