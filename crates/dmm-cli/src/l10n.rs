//! The language dmm-cli speaks, chosen once at startup.
//!
//! A process-wide handle, unlike the GUI's: a run speaks one language from
//! its first line to its last, and the text is printed from every corner of
//! the command tree, clap's derive attributes included. Only `main` sets
//! it, so the unit tests, which never do, always read English.

use console::{StyledObject, style};
use dmm_shared::l10n::{Localizer, Options, Resource};
use dmm_shared::tr;
use std::borrow::Cow;
use std::sync::OnceLock;

static LOCALIZER: OnceLock<Localizer> = OnceLock::new();

/// dmm-cli's own messages, one file per part of the command tree under
/// `i18n/<locale>/cli/`.
const CLI: &[Resource] = &[
    dmm_shared::ftl!("en-US", "cli/common.ftl"),
    dmm_shared::ftl!("en-US", "cli/connect.ftl"),
    dmm_shared::ftl!("en-US", "cli/read.ftl"),
];

/// Everything dmm-cli shows: the shared help, the library's text and
/// errors, and its own messages.
pub(crate) fn resources() -> Vec<Resource> {
    [
        dmm_shared::help::RESOURCES,
        dmm_shared::l10n::LIB_RESOURCES,
        CLI,
    ]
    .concat()
}

/// Pick the language from `flag` (`--lang`) and `setting` (the settings
/// file's), else the environment, and speak it for the rest of the run.
/// Must run before anything is printed or `--help` is built.
pub(crate) fn init(flag: Option<&str>, setting: &str) {
    let language = dmm_shared::l10n::resolve_from_env(flag, setting);
    let _ = LOCALIZER.set(Localizer::new(language, &resources(), &Options::from_env()));
}

/// The run's localizer: English until [`init`] has run.
pub(crate) fn get() -> &'static Localizer {
    LOCALIZER.get_or_init(|| Localizer::en_us(&resources()))
}

/// English whatever the run speaks: what an error's `Display` writes, so
/// the log, which stays English, says the same as an English terminal.
pub(crate) fn english() -> &'static Localizer {
    static ENGLISH: OnceLock<Localizer> = OnceLock::new();
    ENGLISH.get_or_init(|| Localizer::en_us(&resources()))
}

/// `e` as the terminal shows it: a library error or one of dmm-cli's own
/// in the run's language, anything else as it reads.
pub(crate) fn error_text(e: &(dyn std::error::Error + 'static)) -> String {
    if let Some(e) = e.downcast_ref::<dmm_lib::error::Error>() {
        return get().error(e);
    }
    if let Some(e) = e.downcast_ref::<crate::open::OpenFailed>() {
        return e.text(get()).into_owned();
    }
    if let Some(e) = e.downcast_ref::<crate::cmd::read::Refused>() {
        return e.text(get()).into_owned();
    }
    e.to_string()
}

/// `Error:`, red and bold, to open the line saying why a command failed.
pub(crate) fn error_label() -> StyledObject<Cow<'static, str>> {
    style(tr!(get(), "common-error-label")).red().bold()
}

/// `Warning:`, yellow, to open a line about something off that the command
/// goes on past.
pub(crate) fn warning_label() -> StyledObject<Cow<'static, str>> {
    style(tr!(get(), "common-warning-label")).yellow()
}

/// `Note:`, yellow, to open a line about how the command runs.
pub(crate) fn note_label() -> StyledObject<Cow<'static, str>> {
    style(tr!(get(), "common-note-label")).yellow()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every lookup in dmm-cli names an en-US message with the variables
    /// it is given, and every message in its own files is used.
    #[test]
    fn the_messages_match_their_uses() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let owned: Vec<&str> = CLI.iter().map(|r| r.path).collect();
        let problems = dmm_shared::l10n::check::check(&src, &resources(), &owned);
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
