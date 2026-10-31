//! The language dmm-cli speaks, chosen once at startup.
//!
//! A process-wide handle, unlike the GUI's: a run speaks one language from
//! its first line to its last, and the text is printed from every corner of
//! the command tree, clap's derive attributes included. Only `main` sets
//! it, so the unit tests, which never do, always read English.

use dmm_shared::l10n::{Localizer, Options, Resource};
use std::sync::OnceLock;

static LOCALIZER: OnceLock<Localizer> = OnceLock::new();

/// Everything dmm-cli shows: the shared help and the library's text and
/// errors.
fn resources() -> Vec<Resource> {
    [dmm_shared::help::RESOURCES, dmm_shared::l10n::LIB_RESOURCES].concat()
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

/// `e` as the terminal shows it: a library error in the run's language,
/// anything else as it reads.
pub(crate) fn error_text(e: &(dyn std::error::Error + 'static)) -> String {
    match e.downcast_ref::<dmm_lib::error::Error>() {
        Some(e) => get().error(e),
        None => e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every lookup in dmm-cli names an en-US message with the variables
    /// it is given.
    #[test]
    fn the_messages_match_their_uses() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let problems = dmm_shared::l10n::check::check(&src, &resources(), &[]);
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
