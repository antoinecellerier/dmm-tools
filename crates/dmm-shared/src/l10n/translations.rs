//! The checks every translation in `i18n/` passes against en-US, the
//! reference, so a translator's slip fails the build rather than reaching a
//! screen: each file parses and holds only messages and attributes its
//! en-US file has (terms of its own aside), showing the variables the
//! English shows; plurals have every form the language's rules name, and
//! only those; and what must stay as it is — `code` spans,
//! `--options`, a spec note's numbers — stays, with none the English lacks.
//!
//! Any language may leave messages out, which show in English. Before a
//! language ships (`lang::SHIPPED`), and before each release for those that
//! do, [`shipped_languages_are_complete`] wants every file and every message
//! but the specification text and `capture`'s text ([`optional`]): text
//! added in between shows in English until then, so a change to the English
//! never waits on its translations.

use super::lib_index::{GUARDED, kind_of_id, same_numbers};
use dmm_lib::text::TextKind;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use fluent_syntax::ast;
use intl_pluralrules::{PluralCategory, PluralRuleType, PluralRules};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The workspace's `i18n/`.
fn i18n_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../i18n")
}

/// Every `.ftl` under `dir`, by its path relative to it, `/`-separated.
fn ftl_files(dir: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir)
            .expect("readable i18n folder")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.extension().is_some_and(|e| e == "ftl") {
                let rel = path.strip_prefix(root).expect("under root");
                let rel = rel.to_string_lossy().replace('\\', "/");
                out.insert(rel, std::fs::read_to_string(&path).expect("UTF-8 .ftl"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The locales `i18n/` holds besides en-US.
fn translations() -> Vec<String> {
    let mut locales: Vec<String> = std::fs::read_dir(i18n_dir())
        .expect("i18n/")
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|l| l != super::EN_US.id)
        .collect();
    locales.sort();
    locales
}

/// What a check compares in one pattern: a message's value or attribute.
#[derive(Debug, Default)]
struct Shape {
    variables: BTreeSet<String>,
    /// The variables shown in the text, as opposed to only selected on: a
    /// language with one plural form needs no `$count` selector.
    shown: BTreeSet<String>,
    /// Each plural selector's keys, by the variable it selects on.
    plurals: Vec<(String, BTreeSet<String>)>,
    /// Every selector on a variable, with its named keys.
    selectors: Vec<(String, BTreeSet<String>)>,
    /// The text outside placeables, every variant's, for the verbatim
    /// checks.
    text: String,
}

/// Messages and terms of one file: `id` or `id.attribute` → its shape,
/// plus problems the parse found.
fn shapes(source: &str) -> (BTreeMap<String, Shape>, Vec<String>) {
    let mut problems = Vec::new();
    let resource = match fluent_syntax::parser::parse(source) {
        Ok(r) => r,
        Err((r, errors)) => {
            for e in errors {
                let line = source[..e.pos.start.min(source.len())].lines().count() + 1;
                problems.push(format!("line {line}: syntax error {:?}", e.kind));
            }
            r
        }
    };
    let mut out = BTreeMap::new();
    for entry in &resource.body {
        let (id, value, attributes) = match entry {
            ast::Entry::Message(m) => (m.id.name.to_string(), m.value.as_ref(), &m.attributes),
            ast::Entry::Term(t) => (format!("-{}", t.id.name), Some(&t.value), &t.attributes),
            ast::Entry::Junk { content } => {
                let first = content.lines().next().unwrap_or("");
                problems.push(format!("unparsed text: {first}"));
                continue;
            }
            _ => continue,
        };
        if out.contains_key(&id) {
            problems.push(format!("`{id}` is defined twice"));
            continue;
        }
        let mut keys = vec![(id.clone(), value)];
        keys.extend(
            attributes
                .iter()
                .map(|a| (format!("{id}.{}", a.id.name), Some(&a.value))),
        );
        for (key, pattern) in keys {
            let mut shape = Shape::default();
            let has_value = pattern.is_some();
            if let Some(p) = pattern {
                walk_pattern(p, &mut shape);
            }
            if has_value || key != id {
                out.insert(key, shape);
            } else {
                // A message with attributes only: its own key marks it.
                out.insert(key, Shape::default());
            }
        }
    }
    (out, problems)
}

fn walk_pattern(p: &ast::Pattern<&str>, shape: &mut Shape) {
    for element in &p.elements {
        match element {
            ast::PatternElement::TextElement { value } => shape.text.push_str(value),
            ast::PatternElement::Placeable { expression } => {
                walk_expression(expression, shape);
                // Keep the pieces either side of a placeable apart.
                shape.text.push('\u{1}');
            }
        }
    }
}

const PLURAL_KEYS: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];

fn walk_expression(e: &ast::Expression<&str>, shape: &mut Shape) {
    match e {
        ast::Expression::Select { selector, variants } => {
            match selector {
                ast::InlineExpression::VariableReference { id } => {
                    shape.variables.insert(id.name.to_string());
                }
                other => walk_inline(other, shape),
            }
            let keys: BTreeSet<String> = variants
                .iter()
                .filter_map(|v| match &v.key {
                    ast::VariantKey::Identifier { name } => Some(name.to_string()),
                    ast::VariantKey::NumberLiteral { .. } => None,
                })
                .collect();
            if let ast::InlineExpression::VariableReference { id } = selector {
                if !keys.is_empty() && keys.iter().all(|k| PLURAL_KEYS.contains(&k.as_str())) {
                    shape.plurals.push((id.name.to_string(), keys.clone()));
                }
                shape.selectors.push((id.name.to_string(), keys));
            }
            for v in variants {
                walk_pattern(&v.value, shape);
                shape.text.push('\u{1}');
            }
        }
        ast::Expression::Inline(i) => walk_inline(i, shape),
    }
}

fn walk_inline(i: &ast::InlineExpression<&str>, shape: &mut Shape) {
    match i {
        ast::InlineExpression::VariableReference { id } => {
            shape.variables.insert(id.name.to_string());
            shape.shown.insert(id.name.to_string());
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for p in &arguments.positional {
                walk_inline(p, shape);
            }
            for n in &arguments.named {
                walk_inline(&n.value, shape);
            }
        }
        ast::InlineExpression::Placeable { expression } => walk_expression(expression, shape),
        _ => {}
    }
}

/// What must reach the translation as the English has it: `code` spans and
/// `--options`, with how often each occurs.
fn verbatim(text: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    // Between the first tick and the second, the third and the fourth, …;
    // an unclosed tick opens nothing.
    let segments: Vec<&str> = text.split('`').collect();
    for code in segments
        .iter()
        .skip(1)
        .step_by(2)
        .take((segments.len() - 1) / 2)
    {
        if !code.is_empty() && !code.contains('\u{1}') {
            *out.entry(format!("`{code}`")).or_default() += 1;
        }
    }
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 2 < chars.len() {
        let starts = chars[i] == '-'
            && chars[i + 1] == '-'
            && chars[i + 2].is_ascii_lowercase()
            && (i == 0 || !chars[i - 1].is_ascii_alphanumeric() && chars[i - 1] != '-');
        if starts {
            let mut j = i + 2;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '-') {
                j += 1;
            }
            *out.entry(chars[i..j].iter().collect()).or_default() += 1;
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// The plural forms `locale` distinguishes for the integers the apps pass
/// (`Arg::Int`), as Fluent selects them: the rules have no list of their
/// own, so the forms are read off a range of counts.
fn plural_forms(locale: &str) -> BTreeSet<String> {
    // As fluent-bundle negotiates them: the locale's own rules, else its
    // language's (zh-CN reads zh's, pt-BR pt's).
    let id: unic_langid::LanguageIdentifier = locale.parse().expect("a locale id");
    let language: unic_langid::LanguageIdentifier =
        id.language.as_str().parse().expect("a language id");
    let rules = PluralRules::create(id, PluralRuleType::CARDINAL)
        .or_else(|_| PluralRules::create(language, PluralRuleType::CARDINAL))
        .expect("plural rules");
    (0..=1000)
        .chain([1_000_000, 10_000_000])
        .map(|n: u64| {
            match rules.select(n).expect("an integer selects") {
                PluralCategory::ZERO => "zero",
                PluralCategory::ONE => "one",
                PluralCategory::TWO => "two",
                PluralCategory::FEW => "few",
                PluralCategory::MANY => "many",
                PluralCategory::OTHER => "other",
            }
            .to_string()
        })
        .collect()
}

/// Each spec note, input and band of `lib.ftl` whose translation drops,
/// changes or adds a number: the app would show it in English.
fn spec_number_problems(english: &str, translated: &str, locale: &str) -> Vec<String> {
    let bundle = |source: &str, locale: &str| {
        let mut b = FluentBundle::new_concurrent(vec![locale.parse().expect("a locale id")]);
        b.set_use_isolating(false);
        let r = FluentResource::try_new(source.to_string()).unwrap_or_else(|(r, _)| r);
        let _ = b.add_resource(r);
        b
    };
    let en = bundle(english, super::EN_US.id);
    let tr = bundle(translated, locale);
    let mut problems = Vec::new();
    for (id, _) in shapes(english).0 {
        if !kind_of_id(&id).is_some_and(|k| GUARDED.contains(&k)) {
            continue;
        }
        let (Some(e), Some(t)) = (super::format(&en, &id, None), super::format(&tr, &id, None))
        else {
            continue;
        };
        if !same_numbers(&e, &t) {
            problems.push(format!(
                "`{id}`: numbers differ from the English «{e}»: «{t}»"
            ));
        }
    }
    problems
}

/// Whether message `key` of file `path` may be left out of a shipped
/// language: the specification text of `lib.ftl` (table titles, notes,
/// input ratings, accuracy bands) and every message of `cli/capture.ftl`
/// (the guided run behind a device report, whose report stays English),
/// polish that shows in English one message at a time until translated.
/// Everything that drives the meter or the apps — modes, ranges, keys,
/// activation steps, errors — is required.
fn optional(path: &str, key: &str) -> bool {
    path == CAPTURE
        || path == "lib.ftl"
            && kind_of_id(key.split('.').next().unwrap_or(key))
                .is_some_and(|k| k == TextKind::Table || GUARDED.contains(&k))
}

/// `capture`'s own text, every message of it [`optional`].
const CAPTURE: &str = "cli/capture.ftl";

/// What `locale` lacks of en-US, files then messages: all of it with
/// `optional_too`, else all but the [`optional`] messages.
fn gaps(locale: &str, optional_too: bool) -> Vec<String> {
    let english = ftl_files(&i18n_dir().join(super::EN_US.id));
    let translated = ftl_files(&i18n_dir().join(locale));
    let mut gaps = Vec::new();
    for (path, english_source) in &english {
        let Some(source) = translated.get(path) else {
            // A file of optional messages only may be missing as a whole.
            if optional_too || path != CAPTURE {
                gaps.push(format!("{locale}/{path}: missing"));
            }
            continue;
        };
        let (en, tr) = (shapes(english_source).0, shapes(source).0);
        for key in en
            .keys()
            .filter(|k| (optional_too || !optional(path, k)) && !tr.contains_key(*k))
        {
            gaps.push(format!("{locale}/{path}: `{key}` is missing"));
        }
    }
    gaps
}

/// Every problem `locale`'s files have against en-US. A message left out
/// is not one: see [`gaps`].
fn problems(locale: &str) -> Vec<String> {
    let english = ftl_files(&i18n_dir().join(super::EN_US.id));
    let translated = ftl_files(&i18n_dir().join(locale));
    let forms = plural_forms(locale);
    let mut problems = Vec::new();
    for (path, source) in &translated {
        let at = |what: String| format!("{locale}/{path}: {what}");
        let Some(english_source) = english.get(path) else {
            problems.push(at("has no en-US file".into()));
            continue;
        };
        let (en, _) = shapes(english_source);
        let (tr, parse) = shapes(source);
        problems.extend(parse.into_iter().map(at));
        // A term (`-name`, its attributes) may be the language's own: a
        // grammatical gender, say.
        for key in tr
            .keys()
            .filter(|k| !en.contains_key(*k) && !k.starts_with('-'))
        {
            problems.push(at(format!("`{key}` is not in en-US")));
        }
        for (key, t) in &tr {
            let Some(e) = en.get(key) else { continue };
            // Every variable the English shows, and no other; one it only
            // selects on may go, as in a language with one plural form.
            if !t.variables.is_subset(&e.variables) || !e.shown.is_subset(&t.variables) {
                problems.push(at(format!(
                    "`{key}` uses {:?}, en-US {:?}",
                    t.variables, e.variables
                )));
            }
            // A key that isn't a plural form, where the English selects on
            // that variable by plural form, would turn the check off.
            for (variable, keys) in &t.selectors {
                let english_plural = e.plurals.iter().any(|(v, _)| v == variable);
                let odd: Vec<_> = keys
                    .iter()
                    .filter(|k| !PLURAL_KEYS.contains(&k.as_str()))
                    .collect();
                if english_plural && !odd.is_empty() {
                    problems.push(at(format!(
                        "`{key}`: the plural on ${variable} has keys that are no plural form: {odd:?}"
                    )));
                }
            }
            for (variable, keys) in &t.plurals {
                let missing: Vec<_> = forms.difference(keys).collect();
                if !missing.is_empty() {
                    problems.push(at(format!(
                        "`{key}`: the plural on ${variable} lacks {missing:?}"
                    )));
                }
            }
            let (want, have) = (verbatim(&e.text), verbatim(&t.text));
            for (token, n) in &want {
                if have.get(token).copied().unwrap_or(0) < *n {
                    problems.push(at(format!("`{key}` lost {token}, which stays as it is")));
                }
            }
            for token in have.keys().filter(|t| !want.contains_key(*t)) {
                problems.push(at(format!("`{key}` has {token}, which its English lacks")));
            }
        }
        if path == "lib.ftl" {
            problems.extend(
                spec_number_problems(english_source, source, locale)
                    .into_iter()
                    .map(at),
            );
        }
    }
    problems
}

#[test]
fn translations_match_their_english() {
    let mut all = Vec::new();
    for locale in translations() {
        assert!(
            locale.parse::<unic_langid::LanguageIdentifier>().is_ok(),
            "i18n/{locale} is not a locale id"
        );
        all.extend(problems(&locale));
    }
    assert!(all.is_empty(), "{}", all.join("\n"));
}

/// What the shipped languages lack, [`gaps`] each.
fn shipped_gaps(optional_too: bool) -> Vec<String> {
    super::lang::SHIPPED
        .iter()
        .filter(|l| l.id != super::EN_US.id)
        .flat_map(|l| gaps(l.id, optional_too))
        .collect()
}

/// Every shipped language has all its messages but the [`optional`] ones:
/// run before a release, and in the commit that ships a language
/// (`cargo test -p dmm-shared shipped_languages_are_complete -- --ignored`).
#[test]
#[ignore = "before a release: text added since shows in English until then"]
fn shipped_languages_are_complete() {
    let gaps = shipped_gaps(false);
    assert!(gaps.is_empty(), "{}", gaps.join("\n"));
}

/// Every shipped language has the [`optional`] messages too: what the
/// translate skill's fill-gaps workflow drafts with `optional: true`
/// (`cargo test -p dmm-shared shipped_languages_have_the_optional_text --
/// --ignored`).
#[test]
#[ignore = "polish: optional text shows in English until translated"]
fn shipped_languages_have_the_optional_text() {
    let gaps = shipped_gaps(true);
    assert!(gaps.is_empty(), "{}", gaps.join("\n"));
}

/// Every shipped language has a translation of the README, `README.<lang>.md`,
/// made from the README as it is now: its first line holds the
/// `git hash-object` of the English it was translated from, and `git
/// cat-file blob <hash>` gives that English back to diff against. A copy is
/// refreshed at release, not with each English change (`docs/development.md`,
/// release step 5): `cargo test -p dmm-shared readme_translations_are_current
/// -- --ignored`. The always-run checks on the copies are in dmm-cli's
/// `readme_languages.rs`.
#[test]
#[ignore = "before a release: a README translation lags the English until then"]
fn readme_translations_are_current() {
    let root = i18n_dir().join("..");
    let out = std::process::Command::new("git")
        .args(["hash-object", "README.md"])
        .current_dir(&root)
        .output()
        .expect("run git hash-object");
    assert!(out.status.success(), "git hash-object README.md failed");
    let english = String::from_utf8(out.stdout).expect("utf-8 hash");
    let wanted = format!("<!-- translated from README.md {} -->", english.trim());
    let mut stale = Vec::new();
    for l in super::lang::SHIPPED
        .iter()
        .filter(|l| l.id != super::EN_US.id)
    {
        let name = format!("README.{}.md", l.id);
        match std::fs::read_to_string(root.join(&name)) {
            Err(_) => stale.push(format!("{name}: missing")),
            Ok(text) if text.lines().next() != Some(wanted.as_str()) => stale.push(format!(
                "{name}: translated from another README.md ({})",
                text.lines().next().unwrap_or_default()
            )),
            Ok(_) => {}
        }
    }
    assert!(
        stale.is_empty(),
        "README.md is now {}\n{}",
        english.trim(),
        stale.join("\n")
    );
}

#[test]
fn every_shipped_language_has_a_folder() {
    for l in super::lang::SHIPPED {
        assert!(i18n_dir().join(l.id).is_dir(), "i18n/{} is missing", l.id);
    }
}

/// The checks catch what they are for, on a made-up French file.
#[test]
fn the_checks_catch_a_translators_slips() {
    let english = "\
a = { $count ->
    [one] One reading
   *[other] { $count } readings
}
b = Pass `--lang en-US` or --device auto
    .hover = Hover { $x }
c = Plain
";
    let (en, _) = shapes(english);
    let french = "\
a = { $count ->
   *[other] { $count } mesures
}
b = Passez --device auto --format csv
    .hover = Survol { $y }
d = Extra
";
    let (tr, _) = shapes(french);
    assert!(tr.contains_key("d") && !en.contains_key("d"));
    assert_ne!(tr["b.hover"].variables, en["b.hover"].variables);
    let forms = plural_forms("fr");
    assert_eq!(forms, BTreeSet::from(["one".into(), "other".into()]));
    assert!(!forms.is_subset(&tr["a"].plurals[0].1));
    let lost: Vec<_> = verbatim(&en["b"].text)
        .into_keys()
        .filter(|t| !verbatim(&tr["b"].text).contains_key(t))
        .collect();
    assert_eq!(lost, ["--lang", "`--lang en-US`"]);
    let added: Vec<_> = verbatim(&tr["b"].text)
        .into_keys()
        .filter(|t| !verbatim(&en["b"].text).contains_key(t))
        .collect();
    assert_eq!(added, ["--format"]);
    assert_eq!(plural_forms("zh-CN"), BTreeSet::from(["other".into()]));
    // Specification text is the one thing a shipped language may lack.
    assert!(optional("lib.ftl", "note-hold-sel-ncv-for-ncv"));
    assert!(optional("lib.ftl", "table-resistance"));
    assert!(!optional("lib.ftl", "activation-ut8802"));
    assert!(!optional("lib.ftl", "mode-capacitance"));
    assert!(optional("cli/capture.ftl", "capture-no-response"));
    assert!(optional("cli/capture.ftl", "capture-mismatch-mode"));
    assert!(!optional("cli/read.ftl", "read-alarm-on"));
    assert!(!optional("gui/settings.ftl", "note-x"));
    assert_eq!(plural_forms("pt-BR"), forms);
    // A variable the English only selects on isn't one it shows; a
    // misspelt plural key is still a selector on it.
    // "{ $count } readings" shows $count, so no language may drop it.
    assert!(en["a"].shown.contains("count"));
    let (typo, _) = shapes("a = { $count ->\n    [on] Une\n   *[other] { $count }\n}\n");
    assert_eq!(typo["a"].plurals, vec![]);
    assert_eq!(
        typo["a"].selectors,
        vec![(
            "count".to_string(),
            BTreeSet::from(["on".into(), "other".into()])
        )]
    );
    let (selector_only, _) = shapes("b = { $n ->\n    [one] One\n   *[other] Many\n}\n");
    assert!(selector_only["b"].variables.contains("n") && selector_only["b"].shown.is_empty());
    // An option right after a Han character is still one.
    assert!(verbatim("请使用--adapter指定").contains_key("--adapter"));
}

/// Short labels that grew past 1.5 times their English, for a reviewer to
/// look at in the window: `cargo test -p dmm-shared -- --ignored
/// --nocapture long_labels`.
#[test]
#[ignore = "a report, not a check"]
fn long_labels() {
    for locale in translations() {
        let english = ftl_files(&i18n_dir().join(super::EN_US.id));
        for (path, source) in ftl_files(&i18n_dir().join(&locale)) {
            let Some(en) = english.get(&path) else {
                continue;
            };
            let (en, _) = shapes(en);
            for (key, t) in shapes(&source).0 {
                let Some(e) = en.get(&key) else { continue };
                let (e_len, t_len) = (e.text.chars().count(), t.text.chars().count());
                if (1..=24).contains(&e_len) && t_len * 2 > e_len * 3 {
                    println!("{locale}/{path} {key}: {e_len} → {t_len} «{}»", t.text);
                }
            }
        }
    }
}
