//! Keeps the README's translations, `README.<lang>.md`, and the language row
//! that links them in step with the languages the binaries ship.
//!
//! The row sits under the badges of the README and of every translation:
//! English, then each shipped language that has a translation, by its own
//! name in `languages()` order, the page's own bold and the others linked.
//! A translation records on its first line the `git hash-object` of the
//! English it was translated from; dmm-shared's ignored
//! `readme_translations_are_current` compares it at release time. Nothing
//! else checks a translation's links, so this test resolves them too: its
//! relative links, and its in-page links against its own headings.

mod common;

use common::repo_root;
use dmm_shared::l10n::{EN_US, Language, languages};

const HASH_PREFIX: &str = "<!-- translated from README.md ";

/// The file a language's README is in.
fn file_of(language: &Language) -> String {
    if language.id == EN_US.id {
        "README.md".to_string()
    } else {
        format!("README.{}.md", language.id)
    }
}

/// English, then each other language with a translation at the root.
fn with_a_readme() -> Vec<&'static Language> {
    languages()
        .iter()
        .filter(|l| l.id == EN_US.id || repo_root().join(file_of(l)).exists())
        .collect()
}

/// The row as `page` shows it.
fn row_for(page: &Language, listed: &[&Language]) -> String {
    listed
        .iter()
        .map(|l| {
            if l.id == page.id {
                format!("**{}**", l.native_name)
            } else {
                format!("[{}]({})", l.native_name, file_of(l))
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The anchor GitHub gives a heading: lowercased, spaces to hyphens, and
/// everything but letters, digits, `-` and `_` dropped, CJK included.
fn anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// The lines outside code fences.
fn prose(text: &str) -> impl Iterator<Item = &str> {
    let mut fenced = false;
    text.lines().filter(move |line| {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            return false;
        }
        !fenced
    })
}

/// The link targets outside code fences: `](target)`, images included.
fn links(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in prose(text) {
        let mut rest = line;
        while let Some(at) = rest.find("](") {
            rest = &rest[at + 2..];
            let end = rest
                .find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(rest.len());
            found.push(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    found
}

/// What is wrong with the links of one README: relative targets that don't
/// exist, and in-page anchors none of its headings makes.
fn broken_links(name: &str, text: &str) -> Vec<String> {
    let anchors: Vec<String> = prose(text)
        .filter_map(|l| l.strip_prefix('#'))
        .map(|l| anchor(l.trim_start_matches('#').trim()))
        .collect();
    let mut broken = Vec::new();
    for link in links(text) {
        if link.contains(':') {
            continue;
        }
        if let Some(fragment) = link.strip_prefix('#') {
            if !anchors.iter().any(|a| a == fragment) {
                broken.push(format!("{name}: no heading makes the anchor #{fragment}"));
            }
            continue;
        }
        let path = link.split('#').next().unwrap_or_default();
        if !repo_root().join(path).exists() {
            broken.push(format!("{name}: link to {path}, which does not exist"));
        }
    }
    broken
}

#[test]
fn every_readme_has_the_language_row() {
    let listed = with_a_readme();
    let mut problems = Vec::new();
    for page in &listed {
        let name = file_of(page);
        let text = std::fs::read_to_string(repo_root().join(&name)).expect("read the README");
        // A row of one language links nothing; the README has none until a
        // translation exists.
        if listed.len() > 1 {
            let row = row_for(page, &listed);
            if !text.lines().any(|l| l == row) {
                problems.push(format!("{name}: no line reads\n  {row}"));
            }
        }
        if page.id != EN_US.id {
            let first = text.lines().next().unwrap_or_default();
            let hash = first
                .strip_prefix(HASH_PREFIX)
                .and_then(|h| h.strip_suffix(" -->"));
            if !hash.is_some_and(|h| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit())) {
                problems.push(format!(
                    "{name}: the first line must be `{HASH_PREFIX}<git hash-object README.md> -->`, got `{first}`"
                ));
            }
        }
        problems.extend(broken_links(&name, &text));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_readme_translation_is_of_a_shipped_language() {
    let strays: Vec<String> = std::fs::read_dir(repo_root())
        .expect("read the repository root")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|name| {
            let lang = name.strip_prefix("README.")?.strip_suffix(".md")?;
            (!lang.is_empty() && !languages().iter().any(|l| l.id == lang)).then_some(name)
        })
        .collect();
    assert!(
        strays.is_empty(),
        "README translations for languages the binaries don't ship: {strays:?}"
    );
}

#[test]
fn anchors_follow_github() {
    assert_eq!(anchor("Quick start"), "quick-start");
    assert_eq!(anchor("Desktop app (`dmm-gui`)"), "desktop-app-dmm-gui");
    assert_eq!(anchor("Fonctionnalités"), "fonctionnalités");
    assert_eq!(anchor("桌面应用（`dmm-gui`）"), "桌面应用dmm-gui");
}
