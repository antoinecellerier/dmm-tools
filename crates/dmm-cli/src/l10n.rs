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
    dmm_shared::ftl!("en-US", "cli/list.ftl"),
    dmm_shared::ftl!("en-US", "cli/info.ftl"),
    dmm_shared::ftl!("en-US", "cli/debug.ftl"),
    dmm_shared::ftl!("en-US", "cli/command.ftl"),
    dmm_shared::ftl!("en-US", "cli/settings.ftl"),
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

/// An error dmm-cli words itself: the closing `Error:` line of a command.
/// [`localized_error!`] gives it a `Display` in English, which is what the
/// log gets; the terminal gets [`Localized::text`] in the run's language,
/// once the type is listed in [`ours`].
pub(crate) trait Localized: std::error::Error + 'static {
    /// The error in `l`'s language.
    fn text<'a>(&'a self, l: &'a Localizer) -> Cow<'a, str>;
}

/// `Display`, in English, and `Error` for a [`Localized`] type.
macro_rules! localized_error {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&$crate::l10n::Localized::text(
                    self,
                    $crate::l10n::english(),
                ))
            }
        }

        impl std::error::Error for $t {}
    };
}
pub(crate) use localized_error;

/// `e` as one of dmm-cli's [`Localized`] errors: a line per type.
fn ours<'e>(e: &'e (dyn std::error::Error + 'static)) -> Option<&'e dyn Localized> {
    fn of<'e, T: Localized>(e: &'e (dyn std::error::Error + 'static)) -> Option<&'e dyn Localized> {
        e.downcast_ref::<T>().map(|t| t as &dyn Localized)
    }
    of::<crate::open::OpenFailed>(e)
        .or_else(|| of::<crate::cmd::read::Refused>(e))
        .or_else(|| of::<crate::cmd::settings::SetFailed>(e))
        .or_else(|| of::<crate::format::NoFrame>(e))
}

/// `e` as the terminal shows it: a library error or one of dmm-cli's own
/// in `l`'s language, anything else as it reads. One of ours may come
/// inside an `io::Error`, as a writer returns it.
pub(crate) fn error_text(l: &Localizer, e: &(dyn std::error::Error + 'static)) -> String {
    if let Some(e) = e.downcast_ref::<dmm_lib::error::Error>() {
        return l.error(e);
    }
    let inner = e
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::get_ref)
        .map(|inner| inner as &(dyn std::error::Error + 'static));
    match ours(inner.unwrap_or(e)) {
        Some(ours) => ours.text(l).into_owned(),
        None => e.to_string(),
    }
}

/// `Error:`, red and bold, to open the line saying why a command failed.
pub(crate) fn error_label() -> StyledObject<Cow<'static, str>> {
    error_label_in(get())
}

/// [`error_label`] in `l`'s language: capture's stays English.
pub(crate) fn error_label_in(l: &'static Localizer) -> StyledObject<Cow<'static, str>> {
    style(tr!(l, "common-error-label")).red().bold()
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
