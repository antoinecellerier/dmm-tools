//! Which language the apps speak: the ones they ship, how a locale tag
//! matches one, and the order the choice is made in.

use std::sync::OnceLock;

/// A language the apps can speak.
#[derive(Debug, PartialEq, Eq)]
pub struct Language {
    /// The locale its resources live under: `i18n/<id>/`.
    pub id: &'static str,
    /// Its name in itself, as a language list shows it.
    pub native_name: &'static str,
    /// Read by a native speaker. A machine-drafted language says so where
    /// it is offered, until someone has.
    pub reviewed: bool,
}

/// The reference language: every message exists in it, and every other
/// language falls back to it one message at a time.
pub const EN_US: Language = Language {
    id: "en-US",
    native_name: "English",
    reviewed: true,
};

/// The languages the binaries ship. A language is added here in the commit
/// that completes its resources, so a half-translated one is never chosen.
pub(super) const SHIPPED: &[Language] = &[EN_US];

/// The shipped languages, plus any [`L10N_DIR_VAR`] holds a folder for.
///
/// Built once: the variable is read at the first call, as a translator
/// sets it before starting the app.
pub fn languages() -> &'static [Language] {
    static ALL: OnceLock<Vec<Language>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut all: Vec<Language> = SHIPPED.iter().map(copy).collect();
        if let Some(dir) = std::env::var_os(super::L10N_DIR_VAR) {
            all.extend(extra_languages(std::path::Path::new(&dir), &all));
        }
        all
    })
}

fn copy(l: &Language) -> Language {
    Language { ..*l }
}

/// A folder under the override directory that names no shipped language: a
/// translation being started. Its id doubles as its name, and it is not
/// reviewed. Leaked, as the list lives as long as the process.
fn extra_languages(dir: &std::path::Path, known: &[Language]) -> Vec<Language> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|id| id.parse::<unic_langid::LanguageIdentifier>().is_ok())
        .filter(|id| !known.iter().any(|l| l.id.eq_ignore_ascii_case(id)))
        .collect();
    ids.sort();
    ids.into_iter()
        .map(|id| {
            let id: &'static str = Box::leak(id.into_boxed_str());
            Language {
                id,
                native_name: id,
                reviewed: false,
            }
        })
        .collect()
}

/// The language a locale tag asks for, if the apps have it.
///
/// Takes what users and systems write: `fr`, `fr-FR`, `fr_FR.UTF-8@euro`,
/// `zh-Hans-CN`. An exact match wins; otherwise the language subtag decides,
/// except that Chinese must also be in the same script: `Hans` or `Hant`
/// where the tag names one, else traditional for Taiwan, Hong Kong and
/// Macau and simplified elsewhere. So `zh-Hans-HK` is simplified, and
/// `zh-TW` or `zh-Hant-CN` never matches the simplified language.
pub fn find(tag: &str) -> Option<&'static Language> {
    find_in(languages(), tag)
}

/// [`find`] among `languages`: what an app offers, which a test may narrow
/// or widen.
pub fn find_in<'a>(languages: &'a [Language], tag: &str) -> Option<&'a Language> {
    let tag = tag.split(['.', '@']).next().unwrap_or("").replace('_', "-");
    if tag.is_empty() {
        return None;
    }
    if let Some(l) = languages.iter().find(|l| l.id.eq_ignore_ascii_case(&tag)) {
        return Some(l);
    }
    let primary = tag.split('-').next()?;
    languages.iter().find(|l| {
        let id_primary = l.id.split('-').next().unwrap_or("");
        id_primary.eq_ignore_ascii_case(primary)
            && (!primary.eq_ignore_ascii_case("zh") || traditional(l.id) == traditional(&tag))
    })
}

/// Whether a Chinese tag is in traditional script: its script subtag
/// decides, else its region.
fn traditional(tag: &str) -> bool {
    let rest: Vec<String> = tag
        .split('-')
        .skip(1)
        .map(str::to_ascii_lowercase)
        .collect();
    if rest.iter().any(|r| r == "hans") {
        return false;
    }
    rest.iter()
        .any(|r| matches!(r.as_str(), "hant" | "tw" | "hk" | "mo"))
}

/// Pick the language, in this order:
///
/// 1. `flag`, the command line's `--lang`;
/// 2. `setting`, the settings file's `language` (empty: follow the system);
/// 3. the C locale: if the first non-empty of `LC_ALL`, `LC_MESSAGES` and
///    `LANG` is `C` or `POSIX` (`C.UTF-8` included), English, whatever
///    `LANGUAGE` says — scripts and cron jobs set `LC_ALL=C` and expect
///    English (one that must get English whatever the setting says passes
///    `--lang en-US`), and Debian and Ubuntu desktops set `LANGUAGE`
///    anyway. Where those variables are the system's language (Unix but macOS), all
///    three unset is the C locale too, as gettext reads it;
/// 4. `system`, the system's preferred languages in order
///    (`sys_locale::get_locales()`), skipping `C` and `POSIX`;
/// 5. English.
///
/// A flag or setting naming a language the apps don't have falls through to
/// the next step. `var` reads the environment; it is passed in so tests
/// don't have to set process-wide variables.
pub fn resolve(
    flag: Option<&str>,
    setting: &str,
    var: impl Fn(&str) -> Option<String>,
    system: &[String],
) -> &'static Language {
    let gettext = cfg!(all(unix, not(target_os = "macos")));
    resolve_in(languages(), flag, setting, var, system, gettext)
}

/// [`resolve`], `gettext` saying whether the locale variables are the
/// system's language, unset meaning the C locale.
fn resolve_in<'a>(
    languages: &'a [Language],
    flag: Option<&str>,
    setting: &str,
    var: impl Fn(&str) -> Option<String>,
    system: &[String],
    gettext: bool,
) -> &'a Language {
    let english = || {
        languages
            .iter()
            .find(|l| l.id == EN_US.id)
            .expect("en-US ships")
    };
    if let Some(l) = flag
        .into_iter()
        .chain([setting])
        .find_map(|t| find_in(languages, t))
    {
        return l;
    }
    let posix = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(&var)
        .find(|v| !v.is_empty());
    if posix.as_deref().map_or(gettext, is_c_locale) {
        return english();
    }
    system
        .iter()
        .filter(|tag| !is_c_locale(tag))
        .find_map(|tag| find_in(languages, tag))
        .unwrap_or_else(english)
}

/// The C locale, which means "no translation": `C`, `POSIX`, `C.UTF-8`.
fn is_c_locale(value: &str) -> bool {
    value == "C" || value == "POSIX" || value.starts_with("C.")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANGS: &[Language] = &[
        EN_US,
        Language {
            id: "fr",
            native_name: "Français",
            reviewed: true,
        },
        Language {
            id: "pt-BR",
            native_name: "Português (Brasil)",
            reviewed: false,
        },
        Language {
            id: "zh-CN",
            native_name: "简体中文",
            reviewed: false,
        },
    ];

    fn id(tag: &str) -> Option<&'static str> {
        find_in(LANGS, tag).map(|l| l.id)
    }

    #[test]
    fn tags_match_by_language() {
        assert_eq!(id("fr"), Some("fr"));
        assert_eq!(id("fr-CA"), Some("fr"));
        assert_eq!(id("fr_FR.UTF-8@euro"), Some("fr"));
        assert_eq!(id("pt"), Some("pt-BR"));
        assert_eq!(id("pt_PT"), Some("pt-BR"));
        assert_eq!(id("pt-PT"), Some("pt-BR"));
        assert_eq!(id("EN-gb"), Some("en-US"));
        assert_eq!(id("en-US@posix"), Some("en-US"));
        assert_eq!(id("de-DE"), None);
        assert_eq!(id("C.UTF-8"), None);
        assert_eq!(id(""), None);
    }

    #[test]
    fn traditional_chinese_never_matches_simplified() {
        for tag in [
            "zh",
            "zh-CN",
            "zh-Hans",
            "zh-Hans-CN",
            "zh-Hans-HK",
            "zh-Hans-TW",
            "zh-SG",
            "zh_CN.UTF-8",
        ] {
            assert_eq!(id(tag), Some("zh-CN"), "{tag}");
        }
        for tag in [
            "zh-TW",
            "zh-HK",
            "zh-Hant",
            "zh-Hant-TW",
            "zh-Hant-CN",
            "zh_MO",
        ] {
            assert_eq!(id(tag), None, "{tag}");
        }
    }

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    fn pick(
        flag: Option<&str>,
        setting: &str,
        vars: &[(&str, &str)],
        system: &[&str],
    ) -> &'static str {
        let system: Vec<String> = system.iter().map(|s| s.to_string()).collect();
        resolve_in(LANGS, flag, setting, env(vars), &system, false).id
    }

    #[test]
    fn the_flag_beats_the_setting_which_beats_the_system() {
        assert_eq!(pick(Some("zh-CN"), "fr", &[], &["pt-BR"]), "zh-CN");
        assert_eq!(pick(None, "fr", &[], &["pt-BR"]), "fr");
        assert_eq!(pick(None, "", &[], &["pt-BR"]), "pt-BR");
        assert_eq!(pick(None, "", &[], &[]), "en-US");
        // A setting the apps no longer have falls through.
        assert_eq!(pick(None, "de", &[], &["fr-FR"]), "fr");
    }

    #[test]
    fn the_c_locale_means_english() {
        let c = [("LC_ALL", "C.UTF-8"), ("LANGUAGE", "fr")];
        assert_eq!(pick(None, "", &c, &["fr", "C"]), "en-US");
        assert_eq!(pick(None, "", &[("LANG", "POSIX")], &["fr"]), "en-US");
        // Only the first non-empty of the three counts.
        let vars = [
            ("LC_ALL", ""),
            ("LC_MESSAGES", "fr_FR.UTF-8"),
            ("LANG", "C"),
        ];
        assert_eq!(pick(None, "", &vars, &["fr-FR", "C"]), "fr");
        // The flag and the setting still win.
        assert_eq!(pick(None, "fr", &c, &[]), "fr");
    }

    /// On Linux, glibc with none of the three set is in the C locale and
    /// ignores `LANGUAGE`, which `sys_locale` would report. On macOS and
    /// Windows the system list is the source and the variables are rarely
    /// set.
    #[test]
    fn no_locale_variable_is_the_c_locale_where_gettext_reads_them() {
        let system: Vec<String> = vec!["fr".into()];
        let only_language = [("LANGUAGE", "fr"), ("LC_ALL", ""), ("LANG", "")];
        let gettext =
            |vars: &[(&str, &str)]| resolve_in(LANGS, None, "", env(vars), &system, true).id;
        assert_eq!(gettext(&only_language), "en-US");
        assert_eq!(gettext(&[]), "en-US");
        assert_eq!(gettext(&[("LANG", "fr_FR.UTF-8")]), "fr");
        // The setting still wins.
        let set = resolve_in(LANGS, None, "fr", env(&only_language), &system, true);
        assert_eq!(set.id, "fr");
        assert_eq!(pick(None, "", &only_language, &["fr"]), "fr");
    }

    #[test]
    fn the_system_list_is_walked_in_order_past_c() {
        assert_eq!(pick(None, "", &[], &["C", "de", "pt-BR", "fr"]), "pt-BR");
        assert_eq!(pick(None, "", &[], &["POSIX", "zh-TW", "zh-Hans"]), "zh-CN");
    }

    #[test]
    fn a_folder_under_the_override_names_a_new_language() {
        let dir = std::env::temp_dir().join(format!("dmm-l10n-langs-{}", std::process::id()));
        for sub in ["de", "fr", "not a tag", "en-US"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let extra = extra_languages(&dir, LANGS);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(extra.len(), 1);
        assert_eq!((extra[0].id, extra[0].reviewed), ("de", false));
    }
}
