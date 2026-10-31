//! The resources dmm-gui shows, and the localizers built from them.
//!
//! The app keeps its localizer in [`App`](crate::app::App) as a handle; text
//! drawn outside it (`--help`, errors before the window opens) is English.

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

/// The localizer for `setting` (the settings file's `language`, empty to
/// follow the system), in this process's environment.
pub(crate) fn from_env(setting: &str) -> Arc<Localizer> {
    let language = dmm_shared::l10n::resolve_from_env(None, setting);
    Arc::new(Localizer::new(language, &resources(), &Options::from_env()))
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
