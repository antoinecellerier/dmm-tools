//! The language dmm-cli speaks, chosen once at startup.
//!
//! A process-wide handle, unlike the GUI's: a run speaks one language from
//! its first line to its last, and the text is printed from every corner of
//! the command tree, clap's derive attributes included. Only `main` sets
//! it, so the unit tests, which never do, always read English.

use crate::cli::Lang;
use console::{StyledObject, style};
use dmm_shared::l10n::{Language, Localizer, Options, Resource};
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
    dmm_shared::ftl!("en-US", "cli/capture.ftl"),
    dmm_shared::ftl!("en-US", "cli/help.ftl"),
    dmm_shared::ftl!("fr", "cli/common.ftl"),
    dmm_shared::ftl!("fr", "cli/connect.ftl"),
    dmm_shared::ftl!("fr", "cli/read.ftl"),
    dmm_shared::ftl!("fr", "cli/list.ftl"),
    dmm_shared::ftl!("fr", "cli/info.ftl"),
    dmm_shared::ftl!("fr", "cli/debug.ftl"),
    dmm_shared::ftl!("fr", "cli/command.ftl"),
    dmm_shared::ftl!("fr", "cli/settings.ftl"),
    dmm_shared::ftl!("fr", "cli/capture.ftl"),
    dmm_shared::ftl!("fr", "cli/help.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/common.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/connect.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/read.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/list.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/info.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/debug.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/command.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/settings.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/capture.ftl"),
    dmm_shared::ftl!("pt-BR", "cli/help.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/common.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/connect.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/read.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/list.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/info.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/debug.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/command.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/settings.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/capture.ftl"),
    dmm_shared::ftl!("zh-CN", "cli/help.ftl"),
    dmm_shared::ftl!("es", "cli/common.ftl"),
    dmm_shared::ftl!("es", "cli/connect.ftl"),
    dmm_shared::ftl!("es", "cli/read.ftl"),
    dmm_shared::ftl!("es", "cli/list.ftl"),
    dmm_shared::ftl!("es", "cli/info.ftl"),
    dmm_shared::ftl!("es", "cli/debug.ftl"),
    dmm_shared::ftl!("es", "cli/command.ftl"),
    dmm_shared::ftl!("es", "cli/settings.ftl"),
    dmm_shared::ftl!("es", "cli/capture.ftl"),
    dmm_shared::ftl!("es", "cli/help.ftl"),
    dmm_shared::ftl!("ja", "cli/common.ftl"),
    dmm_shared::ftl!("ja", "cli/connect.ftl"),
    dmm_shared::ftl!("ja", "cli/read.ftl"),
    dmm_shared::ftl!("ja", "cli/list.ftl"),
    dmm_shared::ftl!("ja", "cli/info.ftl"),
    dmm_shared::ftl!("ja", "cli/debug.ftl"),
    dmm_shared::ftl!("ja", "cli/command.ftl"),
    dmm_shared::ftl!("ja", "cli/settings.ftl"),
    dmm_shared::ftl!("ja", "cli/capture.ftl"),
    dmm_shared::ftl!("ja", "cli/help.ftl"),
];

/// What each command and option does, as `--help` says it: generated from
/// the doc comments in `cli.rs` by the `cli_help_ftl` test, so the ID scan
/// leaves it out of its unused check, as it does `lib.ftl`.
const CLI_HELP: &[Resource] = &[
    dmm_shared::ftl!("en-US", "cli-help.ftl"),
    dmm_shared::ftl!("fr", "cli-help.ftl"),
    dmm_shared::ftl!("pt-BR", "cli-help.ftl"),
    dmm_shared::ftl!("zh-CN", "cli-help.ftl"),
    dmm_shared::ftl!("es", "cli-help.ftl"),
    dmm_shared::ftl!("ja", "cli-help.ftl"),
];

/// Everything dmm-cli shows: the shared help, the library's text and
/// errors, its own messages and its `--help`.
pub(crate) fn resources() -> Vec<Resource> {
    [
        dmm_shared::help::RESOURCES,
        dmm_shared::l10n::LIB_RESOURCES,
        CLI,
        CLI_HELP,
    ]
    .concat()
}

/// Pick the language from `flag` (`--lang`) and `setting` (the settings
/// file's), else the environment, and speak it for the rest of the run.
/// Must run once, before anything is printed or `--help` is built.
pub(crate) fn init(flag: Option<Lang>, setting: &str) {
    let fresh = LOCALIZER
        .set(Localizer::new(
            choose(flag, setting),
            &resources(),
            &Options::from_env(),
        ))
        .is_ok();
    debug_assert!(fresh, "the language was chosen, or read, before init");
}

/// The language a run speaks: `--lang`'s, else the settings file's, else
/// the environment's ([`dmm_shared::l10n::resolve`]). `--lang system`
/// skips the settings file.
fn choose(flag: Option<Lang>, setting: &str) -> &'static Language {
    match flag {
        Some(Lang::Language(language)) => language,
        Some(Lang::System) => dmm_shared::l10n::resolve_from_env(None, ""),
        None => dmm_shared::l10n::resolve_from_env(None, setting),
    }
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
/// log gets, and a way into [`Failure`], which shows [`Localized::text`] in
/// the run's language.
pub(crate) trait Localized: std::error::Error + 'static {
    /// The error in `l`'s language.
    fn text<'a>(&'a self, l: &'a Localizer) -> Cow<'a, str>;
}

/// `Display`, in English, and `Error` for a [`Localized`] type, and its
/// conversion into a [`Failure`].
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

        impl From<$t> for $crate::l10n::Failure {
            fn from(e: $t) -> Self {
                Self::Ours(Box::new(e))
            }
        }
    };
}
pub(crate) use localized_error;

/// Why a command failed, as it travels to the `Error:` line, kept by what
/// words it, so the line is in the run's language whatever kind it is.
///
/// Not an [`std::error::Error`] itself, so nothing can box one and hide
/// which it is; a new [`Localized`] type comes in through its
/// [`localized_error!`] or does not compile at its `?`. The one way round
/// that is inside an `io::Error`, which only the replay writer's frameless
/// reading takes (see the `From<std::io::Error>`).
#[derive(Debug)]
pub(crate) enum Failure {
    /// The library's, worded by variant.
    Lib(dmm_lib::error::Error),
    /// One of dmm-cli's own.
    Ours(Box<dyn Localized>),
    /// The system's, whose text is its own.
    Io(std::io::Error),
    /// Text with nothing left to word: a file's own error, a parser's.
    Text(String),
}

impl Failure {
    /// The failure as the terminal shows it, in `l`'s language.
    pub(crate) fn text(&self, l: &Localizer) -> String {
        match self {
            Self::Lib(e) => l.error(e),
            Self::Ours(e) => e.text(l).into_owned(),
            Self::Io(e) => e.to_string(),
            Self::Text(text) => text.clone(),
        }
    }

    /// The library's error, when it is one.
    pub(crate) fn lib(&self) -> Option<&dmm_lib::error::Error> {
        match self {
            Self::Lib(e) => Some(e),
            _ => None,
        }
    }
}

/// In English, for the log.
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lib(e) => e.fmt(f),
            Self::Ours(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::Text(text) => f.write_str(text),
        }
    }
}

impl From<dmm_lib::error::Error> for Failure {
    fn from(e: dmm_lib::error::Error) -> Self {
        Self::Lib(e)
    }
}

/// A writer returns an [`std::io::Error`], and one of ours can come inside
/// it: a replay's line that has no meter frame to record.
impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        match e.downcast::<crate::format::NoFrame>() {
            Ok(ours) => ours.into(),
            Err(e) => Self::Io(e),
        }
    }
}

impl From<String> for Failure {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

/// The handler the run installs for Ctrl-C could not be set.
impl From<ctrlc::Error> for Failure {
    fn from(e: ctrlc::Error) -> Self {
        Self::Text(e.to_string())
    }
}

/// A value that would not serialize, for `--format json`.
impl From<serde_json::Error> for Failure {
    fn from(e: serde_json::Error) -> Self {
        Self::Text(e.to_string())
    }
}

/// A capture report that would not serialize.
impl From<serde_saphyr::SerializeError> for Failure {
    fn from(e: serde_saphyr::SerializeError) -> Self {
        Self::Text(e.to_string())
    }
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

    /// Each kind of failure reaches the terminal through the messages, and
    /// the log in English. (One of ours inside a writer's `io::Error`: the
    /// replay writer's test in `format.rs`.)
    #[test]
    fn a_failure_is_worded_in_the_runs_language() {
        let options = dmm_shared::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        let pseudo = Localizer::new(&dmm_shared::l10n::EN_US, &resources(), &options);
        let ours = Failure::from(crate::open::OpenFailed::NotFound);
        let lib = Failure::from(dmm_lib::error::Error::Timeout);
        for failure in [&ours, &lib] {
            let text = failure.text(&pseudo);
            assert!(text.starts_with('['), "{text}");
            assert!(!failure.to_string().starts_with('['), "{failure}");
        }
        // The system's own text is all there is to show.
        let io = Failure::from(std::io::Error::other("disk full"));
        assert_eq!(io.text(&pseudo), "disk full");
    }

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
