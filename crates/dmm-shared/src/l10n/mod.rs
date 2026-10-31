//! Translation: the apps' words in the user's language, through
//! [Fluent](https://projectfluent.org/).
//!
//! Canonical English stays the data. File contents, settings, logs and the
//! library's own strings never change language; text is translated as it
//! reaches the screen or the terminal. The design and its reasons are in
//! `docs/localization-design.md`.
//!
//! - Resources live in `i18n/<locale>/` at the workspace root, `en-US` being
//!   the reference, and are compiled in with [`ftl!`], each binary taking only
//!   the files it shows.
//! - A [`Localizer`] is a handle, not a global: the GUI keeps one in its app
//!   state and swaps it when the language changes, the CLI builds one at
//!   startup, and each test builds its own.
//! - Messages are looked up with [`tr!`](crate::tr), whose message id must be
//!   a literal so a test can check every id used against the en-US files.
//! - A message missing from a language, or failing to format in it, falls
//!   back to en-US on its own.

#[cfg(any(test, feature = "test-support"))]
pub mod check;
mod errors;
mod lang;
#[cfg(test)]
mod lib_ftl;
mod lib_index;
mod lib_names;
mod pseudo;

pub use lang::{EN_US, Language, find, languages, resolve};
pub use lib_index::{LIB_RESOURCES, spec_numbers};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Developer switch for the padding pseudo-locale (any non-empty value): it
/// lengthens every message to find layouts that can't take a longer
/// language, and brackets each one to show text that bypassed the
/// localizer. Never offered in a language list.
pub const PSEUDO_VAR: &str = "DMM_TOOLS_PSEUDO_LOCALE";

/// A folder whose `<locale>/…` files are loaded over the built-in ones, so
/// a translation can be tried in a release binary without rebuilding. A
/// folder for a locale the apps don't ship makes that language selectable.
pub const L10N_DIR_VAR: &str = "DMM_TOOLS_L10N_DIR";

/// [`resolve`] against this process: its locale variables and the system's
/// preferred languages.
pub fn resolve_from_env(flag: Option<&str>, setting: &str) -> &'static Language {
    let system: Vec<String> = sys_locale::get_locales().collect();
    resolve(flag, setting, |k| std::env::var(k).ok(), &system)
}

/// One `.ftl` file compiled into a binary.
#[derive(Debug, Clone, Copy)]
pub struct Resource {
    /// The locale it belongs to: `en-US`, `fr`, …
    pub locale: &'static str,
    /// Its path under `i18n/<locale>/`, e.g. `gui/top-bar.ftl`.
    pub path: &'static str,
    /// Its text.
    pub source: &'static str,
}

/// Compile `i18n/<locale>/<path>` into the binary as a [`Resource`].
#[macro_export]
macro_rules! ftl {
    ($locale:literal, $path:literal) => {
        $crate::l10n::Resource {
            locale: $locale,
            path: $path,
            source: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../i18n/",
                $locale,
                "/",
                $path
            )),
        }
    };
}

/// How a [`Localizer`] is built beyond its language: what the environment
/// asked for at startup. Default for tests: no pseudo-locale, no override.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The pseudo-locale (see [`PSEUDO_VAR`]).
    pub pseudo: bool,
    /// Files loaded over the built-in ones (see [`L10N_DIR_VAR`]).
    pub override_dir: Option<PathBuf>,
}

impl Options {
    /// What the process environment asks for.
    pub fn from_env() -> Self {
        Self {
            pseudo: std::env::var_os(PSEUDO_VAR).is_some_and(|v| !v.is_empty()),
            override_dir: std::env::var_os(L10N_DIR_VAR)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        }
    }
}

/// A value for a message's variable.
///
/// Numbers go in as integers, for plural selection, or as text already
/// formatted the app's way: a raw `f64` would lose its precision ("5.0"
/// prints as "5"), so there is deliberately no float variant.
#[derive(Debug, Clone)]
pub enum Arg<'a> {
    Str(Cow<'a, str>),
    Int(i64),
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(s: &'a str) -> Self {
        Self::Str(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for Arg<'a> {
    fn from(s: &'a String) -> Self {
        Self::Str(Cow::Borrowed(s))
    }
}

impl From<String> for Arg<'_> {
    fn from(s: String) -> Self {
        Self::Str(Cow::Owned(s))
    }
}

impl<'a> From<Cow<'a, str>> for Arg<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::Str(s)
    }
}

macro_rules! int_arg {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg<'_> {
            fn from(n: $t) -> Self {
                Self::Int(i64::try_from(n).unwrap_or(i64::MAX))
            }
        }
    )*};
}
int_arg!(i32, i64, u8, u16, u32, u64, usize);

/// Look up a message: `tr!(l10n, "id")`, or with variables
/// `tr!(l10n, "id", name = value, count = n)`. An attribute is
/// `"id.attribute"`. Returns a `Cow<str>`, borrowed when the message has no
/// variables.
///
/// The id must be a literal, so a test can find every use and check it
/// against the en-US resources, variables included.
#[macro_export]
macro_rules! tr {
    ($l10n:expr, $id:literal $(,)?) => {
        $l10n.message($id, &[])
    };
    ($l10n:expr, $id:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $l10n.message(
            $id,
            &[$((stringify!($name), $crate::l10n::Arg::from($value))),+],
        )
    };
}

/// The messages of one language, falling back to en-US one message at a
/// time.
pub struct Localizer {
    language: &'static Language,
    /// The language's bundle, then en-US's; just en-US for English.
    bundles: Vec<FluentBundle<FluentResource>>,
    /// What loading the resources found, for [`Localizer::log_load_report`].
    load_report: Vec<(log::Level, String)>,
    /// The level a message failing to format logs at: a translator trying
    /// files from the override folder sees it without asking.
    format_failure: log::Level,
    pseudo: bool,
}

impl Localizer {
    /// `language`'s messages from `resources`, which list every locale's
    /// files; those of other locales are ignored.
    pub fn new(language: &'static Language, resources: &[Resource], options: &Options) -> Self {
        let mut bundles = Vec::with_capacity(2);
        let mut load_report = Vec::new();
        if language.id != EN_US.id {
            bundles.push(bundle(language.id, resources, options, &mut load_report));
        }
        bundles.push(bundle(EN_US.id, resources, options, &mut load_report));
        if options.pseudo {
            // Every bundle: a message the language lacks comes from en-US's
            // and must be padded too.
            for b in &mut bundles {
                b.set_transform(Some(pseudo::transform));
            }
        }
        Self {
            language,
            bundles,
            load_report,
            format_failure: problem_level(options),
            pseudo: options.pseudo,
        }
    }

    /// English, as tests pin it.
    pub fn en_us(resources: &[Resource]) -> Self {
        Self::new(&EN_US, resources, &Options::default())
    }

    /// The language this localizer speaks.
    pub fn language(&self) -> &'static Language {
        self.language
    }

    /// Log what loading the resources found: each override file read, and
    /// the syntax errors and redefinitions in any file. Call it once, when
    /// the logger is installed; dmm-cli builds its localizer before that,
    /// since the language must be known before `--help` is.
    pub fn log_load_report(&self) {
        for (level, line) in &self.load_report {
            log::log!(*level, "{line}");
        }
    }

    /// The text of message `id` (or `id.attribute`) with `args`. Use
    /// [`tr!`](crate::tr), which keeps ids literal.
    ///
    /// A message the language lacks, or one that fails to format there,
    /// comes from en-US. One missing from en-US too is a bug the id scan
    /// catches; release builds show the id rather than nothing.
    #[doc(hidden)]
    pub fn message<'a>(&'a self, id: &str, args: &[(&str, Arg<'_>)]) -> Cow<'a, str> {
        self.lookup(id, args).unwrap_or_else(|| {
            debug_assert!(
                false,
                "`{id}`: no en-US message, or it failed to format (see the log)"
            );
            Cow::Owned(id.to_string())
        })
    }

    /// [`Localizer::message`], `None` where en-US lacks the message too or
    /// fails to format it.
    fn lookup<'a>(&'a self, id: &str, args: &[(&str, Arg<'_>)]) -> Option<Cow<'a, str>> {
        let args = fluent_args(args);
        let text = self
            .bundles
            .iter()
            .find_map(|b| format_at(b, id, args.as_ref(), self.format_failure))?;
        Some(if self.pseudo {
            Cow::Owned(pseudo::bracket(&text))
        } else {
            text
        })
    }

    /// Whether this localizer shows the English as written: en-US without
    /// the pseudo-locale.
    fn plain_english(&self) -> bool {
        self.language.id == EN_US.id && !self.pseudo
    }
}

/// `id`'s text in `bundle`, `None` when the bundle lacks it or it fails to
/// format there.
fn format<'b>(
    bundle: &'b FluentBundle<FluentResource>,
    id: &str,
    args: Option<&FluentArgs<'_>>,
) -> Option<Cow<'b, str>> {
    format_at(bundle, id, args, log::Level::Warn)
}

/// [`format`], logging a failure at `level`, once per id and process: a
/// message drawn every frame would otherwise flood the log.
fn format_at<'b>(
    bundle: &'b FluentBundle<FluentResource>,
    id: &str,
    args: Option<&FluentArgs<'_>>,
    level: log::Level,
) -> Option<Cow<'b, str>> {
    let (name, attribute) = match id.split_once('.') {
        Some((name, attribute)) => (name, Some(attribute)),
        None => (id, None),
    };
    let message = bundle.get_message(name)?;
    let pattern = match attribute {
        Some(a) => message.get_attribute(a)?.value(),
        None => message.value()?,
    };
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    if errors.is_empty() {
        return Some(text);
    }
    use std::collections::BTreeSet;
    use std::sync::Mutex;
    static REPORTED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    if REPORTED.lock().is_ok_and(|mut r| r.insert(id.to_string())) {
        let locale = &bundle.locales[0];
        log::log!(level, "l10n: {locale}: `{id}` failed to format: {errors:?}");
    }
    None
}

/// The level of what may be a translator's mistake: a problem in an
/// override file, or a message failing to format. Error with an override
/// folder: a translator who set it is trying their files and must see a
/// mistake in them at the default log level (which shows errors only), or
/// the message silently falls back to English. Otherwise a warning: the
/// built-in files are checked by tests, and an end user can do nothing
/// about them.
fn problem_level(options: &Options) -> log::Level {
    if options.override_dir.is_some() {
        log::Level::Error
    } else {
        log::Level::Warn
    }
}

fn fluent_args<'a>(args: &'a [(&'a str, Arg<'a>)]) -> Option<FluentArgs<'a>> {
    if args.is_empty() {
        return None;
    }
    let mut out = FluentArgs::with_capacity(args.len());
    for (name, value) in args {
        let value = match value {
            Arg::Str(s) => FluentValue::from(s.as_ref()),
            Arg::Int(n) => FluentValue::from(*n),
        };
        out.set(*name, value);
    }
    Some(out)
}

/// A bundle of `locale`'s resources, with the override folder's copy of a
/// file loaded over the built-in one: its messages replace those with the
/// same ids, and the rest stay. A locale nothing is compiled in for (a
/// language being started under the override) takes en-US's file list.
///
/// A mistake in an override file is reported at [`problem_level`], one in
/// a built-in file as a warning.
fn bundle(
    locale: &'static str,
    resources: &[Resource],
    options: &Options,
    report: &mut Vec<(log::Level, String)>,
) -> FluentBundle<FluentResource> {
    let langid = locale
        .parse()
        .unwrap_or_else(|_| EN_US.id.parse().expect("en-US is a language id"));
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    let paths = resources
        .iter()
        .filter(|r| r.locale == EN_US.id)
        .map(|r| r.path);
    for path in paths {
        let embedded = resources
            .iter()
            .find(|r| r.locale == locale && r.path == path)
            .map(|r| (r.source.to_string(), false));
        let over = options
            .override_dir
            .as_deref()
            .and_then(|dir| read_override(dir, locale, path, report))
            .map(|source| (source, true));
        for (source, overriding) in embedded.into_iter().chain(over) {
            let level = if overriding {
                problem_level(options)
            } else {
                log::Level::Warn
            };
            let resource = match FluentResource::try_new(source) {
                Ok(r) => r,
                Err((r, errors)) => {
                    report.push((
                        level,
                        format!("l10n: {locale}/{path} has syntax errors: {errors:?}"),
                    ));
                    r
                }
            };
            if overriding {
                bundle.add_resource_overriding(resource);
            } else if let Err(errors) = bundle.add_resource(resource) {
                report.push((
                    level,
                    format!("l10n: {locale}/{path} redefines messages: {errors:?}"),
                ));
            }
        }
    }
    bundle
}

fn read_override(
    dir: &Path,
    locale: &str,
    path: &str,
    report: &mut Vec<(log::Level, String)>,
) -> Option<String> {
    let file = dir.join(locale).join(path);
    match std::fs::read_to_string(&file) {
        Ok(text) => {
            report.push((log::Level::Info, format!("l10n: loaded {}", file.display())));
            Some(text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            let line = format!("l10n: can't read {}: {e}", file.display());
            report.push((log::Level::Error, line));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "\
greeting = Hello
connect = Connect { $model } now
samples = { $n ->
    [one] One sample
   *[other] { $n } samples
}
button = Clear
    .hover = Clear the graph
only-english = English only
";

    const FR: &str = "\
greeting = Bonjour
connect = Connecter { $model } maintenant
samples = { $n ->
    [one] Un échantillon
   *[other] { $n } échantillons
}
button = Effacer
    .hover = Effacer le graphique
";

    const RESOURCES: &[Resource] = &[
        Resource {
            locale: "en-US",
            path: "test.ftl",
            source: EN,
        },
        Resource {
            locale: "fr",
            path: "test.ftl",
            source: FR,
        },
    ];

    const FRENCH: Language = Language {
        id: "fr",
        native_name: "Français",
        reviewed: true,
    };

    #[test]
    fn english_messages_borrow_and_carry_no_isolation_marks() {
        let l = Localizer::en_us(RESOURCES);
        assert!(matches!(tr!(l, "greeting"), Cow::Borrowed("Hello")));
        assert_eq!(tr!(l, "connect", model = "UT61E+"), "Connect UT61E+ now");
        assert_eq!(tr!(l, "button.hover"), "Clear the graph");
    }

    #[test]
    fn plurals_select_on_integers() {
        let l = Localizer::en_us(RESOURCES);
        assert_eq!(tr!(l, "samples", n = 1usize), "One sample");
        assert_eq!(tr!(l, "samples", n = 3u32), "3 samples");
    }

    #[test]
    fn a_missing_translation_falls_back_to_english_alone() {
        static FR_LANG: Language = FRENCH;
        let l = Localizer::new(&FR_LANG, RESOURCES, &Options::default());
        assert_eq!(tr!(l, "greeting"), "Bonjour");
        assert_eq!(tr!(l, "button.hover"), "Effacer le graphique");
        assert_eq!(tr!(l, "samples", n = 2usize), "2 échantillons");
        assert_eq!(tr!(l, "only-english"), "English only");
    }

    #[test]
    fn the_pseudo_locale_pads_text_but_not_values() {
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let l = Localizer::new(&EN_US, RESOURCES, &options);
        assert_eq!(
            tr!(l, "connect", model = "UT61E+"),
            "[Çóññéçţ~~~ UT61E+ ñóŵ~~]"
        );
    }

    /// An English fallback in a translated pseudo run is padded like the
    /// rest.
    #[test]
    fn the_pseudo_locale_pads_english_fallbacks() {
        static FR_LANG: Language = FRENCH;
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let l = Localizer::new(&FR_LANG, RESOURCES, &options);
        assert_eq!(
            tr!(l, "only-english"),
            tr!(Localizer::new(&EN_US, RESOURCES, &options), "only-english")
        );
        assert_ne!(tr!(l, "only-english"), "[English only]");
    }

    #[test]
    fn the_override_folder_s_messages_win() {
        static FR_LANG: Language = FRENCH;
        let dir = std::env::temp_dir().join(format!("dmm-l10n-override-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("fr")).unwrap();
        std::fs::write(dir.join("fr/test.ftl"), "greeting = Salut\n").unwrap();
        let options = Options {
            override_dir: Some(dir.clone()),
            ..Options::default()
        };
        let l = Localizer::new(&FR_LANG, RESOURCES, &options);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(tr!(l, "greeting"), "Salut");
        // What the override lacks stays as built in.
        assert_eq!(tr!(l, "button"), "Effacer");
    }

    /// A partial en-US file under the override leaves English, the
    /// fallback of every language, whole.
    #[test]
    fn a_partial_english_override_keeps_the_rest() {
        let dir = std::env::temp_dir().join(format!("dmm-l10n-en-override-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("en-US")).unwrap();
        std::fs::write(dir.join("en-US/test.ftl"), "greeting = Hi\n").unwrap();
        let options = Options {
            override_dir: Some(dir.clone()),
            ..Options::default()
        };
        let l = Localizer::new(&EN_US, RESOURCES, &options);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(tr!(l, "greeting"), "Hi");
        assert_eq!(tr!(l, "only-english"), "English only");
        assert_eq!(tr!(l, "button.hover"), "Clear the graph");
    }

    /// The report waits for the logger, and a mistake in an override file
    /// is an error, which the default log level shows.
    #[test]
    fn an_override_file_s_mistakes_are_reported_as_errors() {
        static FR_LANG: Language = FRENCH;
        let dir = std::env::temp_dir().join(format!("dmm-l10n-report-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("fr")).unwrap();
        std::fs::write(dir.join("fr/test.ftl"), "greeting = Salut\nbroken = {\n").unwrap();
        let options = Options {
            override_dir: Some(dir.clone()),
            ..Options::default()
        };
        let l = Localizer::new(&FR_LANG, RESOURCES, &options);
        std::fs::remove_dir_all(&dir).unwrap();
        let levels: Vec<log::Level> = l.load_report.iter().map(|(level, _)| *level).collect();
        assert_eq!(
            levels,
            [log::Level::Info, log::Level::Error],
            "{:?}",
            l.load_report
        );
        assert!(l.load_report[1].1.contains("fr/test.ftl has syntax errors"));
        assert!(Localizer::en_us(RESOURCES).load_report.is_empty());
    }

    #[test]
    fn the_localizer_crosses_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Localizer>();
    }
}
