//! The Linux desktop entry dmm-gui installs for itself: the shipped
//! `assets/dmm-tools.desktop`, with its `Exec` pointed at the running binary
//! and its `Comment` in every language the app speaks.

use dmm_shared::l10n::{EN_US, Language, Localizer, Options, Resource};
use std::borrow::Cow;

/// The entry as it ships: the en-US text, and the source of every key.
const SHIPPED: &str = include_str!("../../../assets/dmm-tools.desktop");

/// The entry for the binary at `exe`.
///
/// Each language other than en-US whose own `desktop-comment` differs from
/// the English adds a `Comment[<locale>]` line. Only the translations built
/// into the binary count: a draft under the override folder stays out of
/// the user's application list.
pub(crate) fn contents(exe: &str) -> String {
    let resources = crate::l10n::desktop_resources();
    let comments = comments(dmm_shared::l10n::languages(), &resources);
    render(SHIPPED, exe, &comments)
}

/// `(xdg locale, comment)` for each of `languages` with its own comment.
fn comments(languages: &'static [Language], resources: &[Resource]) -> Vec<(String, String)> {
    let english = Localizer::en_us(resources);
    let english = dmm_shared::tr!(english, "desktop-comment");
    languages
        .iter()
        .filter(|l| l.id != EN_US.id)
        .filter_map(|l| {
            let localizer = Localizer::new(l, resources, &Options::default());
            let text = dmm_shared::tr!(localizer, "desktop-comment");
            (text != english).then(|| (l.id.replace('-', "_"), text.into_owned()))
        })
        .collect()
}

/// `template` with its `Exec` value replaced by `exe` and the `comments`
/// after its `Comment` line.
fn render(template: &str, exe: &str, comments: &[(String, String)]) -> String {
    let mut out = String::with_capacity(template.len() + exe.len());
    for line in template.lines() {
        if line.starts_with("Exec=") {
            out.push_str("Exec=");
            out.push_str(exe);
        } else {
            out.push_str(line);
        }
        out.push('\n');
        if line.starts_with("Comment=") {
            for (locale, text) in comments {
                out.push_str(&format!("Comment[{locale}]={}\n", escape(text)));
            }
        }
    }
    out
}

/// A desktop-entry string value: backslash, newline, tab and carriage
/// return escaped, so the value stays on its line.
fn escape(text: &str) -> Cow<'_, str> {
    if !text.contains(['\\', '\n', '\t', '\r']) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = "/opt/dmm-tools/dmm-gui";

    /// The keys of `entry`, checking it parses as a desktop entry: the
    /// `[Desktop Entry]` group first, every other line `key=value`, no key
    /// twice.
    fn keys(entry: &str) -> Vec<&str> {
        assert!(entry.ends_with('\n'), "{entry:?}");
        let mut lines = entry.lines();
        assert_eq!(lines.next(), Some("[Desktop Entry]"));
        let mut keys = Vec::new();
        for line in lines {
            let (key, _) = line.split_once('=').unwrap_or_else(|| panic!("{line:?}"));
            assert!(!keys.contains(&key), "`{key}` twice in {entry:?}");
            keys.push(key);
        }
        keys
    }

    #[test]
    fn the_english_entry_is_the_one_written_before_translation() {
        let before = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=dmm-tools\n\
             Comment=Real-time display and plotting for digital multimeters\n\
             Exec={EXE}\n\
             Icon=dmm-tools\n\
             Categories=Utility;Electronics;\n\
             StartupWMClass=dmm-tools\n"
        );
        assert_eq!(contents(EXE), before);
    }

    #[test]
    fn the_written_entry_parses_and_runs_this_binary() {
        let entry = contents(EXE);
        let keys = keys(&entry);
        for key in ["Type", "Name", "Comment", "Exec", "Icon"] {
            assert!(keys.contains(&key), "no {key} in {entry:?}");
        }
        assert!(entry.contains(&format!("\nExec={EXE}\n")), "{entry:?}");
    }

    #[test]
    fn the_shipped_comment_is_the_english_message() {
        let english = Localizer::en_us(&crate::l10n::desktop_resources());
        let comment = format!(
            "\nComment={}\n",
            dmm_shared::tr!(english, "desktop-comment")
        );
        assert!(SHIPPED.contains(&comment), "{SHIPPED:?}");
    }

    #[test]
    fn a_translated_comment_follows_the_english_one_escaped() {
        static LANGS: &[Language] = &[
            EN_US,
            Language {
                id: "pt-BR",
                native_name: "Português (Brasil)",
                reviewed: false,
            },
            Language {
                id: "fr",
                native_name: "Français",
                reviewed: true,
            },
            Language {
                id: "de",
                native_name: "Deutsch",
                reviewed: false,
            },
        ];
        // The made-up translations stand alone: the shipped ones are left out.
        let mut resources: Vec<Resource> = crate::l10n::desktop_resources()
            .into_iter()
            .filter(|r| r.locale == dmm_shared::l10n::EN_US.id)
            .collect();
        resources.push(Resource {
            locale: "pt-BR",
            path: "gui/desktop.ftl",
            source: "desktop-comment = Leitura { \"\\\\\" } e{ \"\\u0009\" }gráfico\n",
        });
        resources.push(Resource {
            locale: "fr",
            path: "gui/desktop.ftl",
            source: "desktop-comment = Affichage\n    sur deux lignes\n",
        });
        // German has no file: it falls back to English and adds no line.
        let comments = comments(LANGS, &resources);
        let entry = render(SHIPPED, EXE, &comments);
        keys(&entry);
        let comment_lines: Vec<&str> = entry
            .lines()
            .skip_while(|l| !l.starts_with("Comment="))
            .take_while(|l| l.starts_with("Comment"))
            .skip(1)
            .collect();
        assert_eq!(
            comment_lines,
            [
                "Comment[pt_BR]=Leitura \\\\ e\\tgráfico",
                "Comment[fr]=Affichage\\nsur deux lignes",
            ]
        );
    }
}
