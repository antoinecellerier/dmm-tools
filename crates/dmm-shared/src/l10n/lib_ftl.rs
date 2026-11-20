//! The generator of `i18n/en-US/lib.ftl` and `lib-specs.ftl`, run as a test:
//! together they must hold one message per text `dmm_lib::text::all_texts()`
//! lists, unit-only labels aside ([`is_units_only`]). The specification
//! text (bands, inputs, notes) is `lib-specs.ftl`'s, the rest `lib.ftl`'s.
//!
//! `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl` adds what is missing;
//! a plain run fails until then. The generator only adds: an existing
//! message keeps its ID and its place, and one whose English the library
//! no longer shows is an orphan the test names and leaves for the
//! maintainer — edit its English to the library's new text to keep its ID
//! and translations, or delete it. Comments are rewritten every time. A new
//! spec text worded close to a spec template is named once
//! ([`near_template`]): its author may have meant the template's wording.

use super::ftl_text::escape;
use super::lib_index::{
    GUARDED, LIB, LIB_SPECS, LibEntry, is_unit, is_units_only, kind_of_id, lib_entries,
    spec_templates, split_qualifier, unqualify,
};
use super::spec_templates::{Binding, TEXT_SLOT, Template};
use dmm_lib::protocol::registry;
use dmm_lib::text::{TextKind, all_texts};
use fluent_syntax::ast::{
    Comment, Entry, Expression, Identifier, InlineExpression, Message, Pattern, PatternElement,
    Resource,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

/// One generated file: its name under `i18n/en-US/`, the `###` comment it
/// opens with, and whether it holds the specification text ([`GUARDED`])
/// or everything else.
struct File {
    name: &'static str,
    header: &'static str,
    specs: bool,
}

const FILES: [File; 2] = [
    File {
        name: LIB,
        header: HEADER,
        specs: false,
    },
    File {
        name: LIB_SPECS,
        header: SPECS_HEADER,
        specs: true,
    },
];

impl File {
    fn path(&self) -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../i18n/en-US")).join(self.name)
    }

    /// Whether a message of `kind` is this file's.
    fn holds(&self, kind: TextKind) -> bool {
        GUARDED.contains(&kind) == self.specs
    }
}

/// Words of a text kept in its ID.
const SLUG_WORDS: usize = 6;

/// Users and spec tables named in an entry's comment before they are
/// counted instead.
const NAMED_USERS: usize = 6;
const NAMED_TABLES: usize = 3;

const HEADER: &str = "\
Text the library shows: the labels of readings and of the choices a meter
offers, meter keys and the steps that switch a meter's data transmission on.
The specification tables' text is in lib-specs.ftl.

Generated from dmm-lib by `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl`.
Entries are only ever added. The English of each one is what the library
shows and is how the app finds it: edit it only to follow a change in the
library, keeping the ID to keep its translations.

Numbers and unit symbols stay as they are in every language. Labels made of
nothing else (200mV, kΩ, °C) are not listed: they always show as they are.
Nor are modes made of a qualifier around another mode (LPF AC V): the
qualifier is listed once.";

const SPECS_HEADER: &str = "\
Text of the specification tables: accuracy bands, input ratings and notes,
from each meter's manual, as the Specifications panel shows them.

Optional: a message left out shows in English.

Generated from dmm-lib by `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl`.
Entries are only ever added. The English of each one is what the library
shows and is how the app finds it: edit it only to follow a change in the
library, keeping the ID to keep its translations.

Keep every number, sign and ± exactly; a decimal comma is fine (0,6). A
translation that changes, drops or adds a number is shown in English. Text
made of numbers and unit symbols only is not listed: it always shows as it
is.";

/// The `##` comment opening each kind's section: what it is, where it
/// shows, how to translate it.
fn section(kind: TextKind) -> &'static str {
    match kind {
        TextKind::Mode => {
            "Measurement modes: beside the reading, in the mode menu, the graph
and the statistics. Keep each as short as the meter prints it; you may keep
abbreviations such as DC V, LPF or REL as they are."
        }
        TextKind::Qualifier => {
            "Mode qualifiers: a word around a mode's name for a low-pass filter, a
held peak or a relative reading, around any of the modes above. $mode is the
mode, already translated: put it where your language puts it."
        }
        TextKind::Range => {
            "Ranges: beside the reading and in the range menu. Ranges that are a
number and a unit only are not listed."
        }
        TextKind::Aux => {
            "Sub-value labels: the meter's secondary readings beside the main one,
in the graph's legend and in the statistics. Keep them short: each sits next
to a number."
        }
        TextKind::Toggle => "On and off, for a meter setting offered as a choice.",
        TextKind::Key => {
            "Meter keys named in words for what they do. A name printed on a key is
not listed here: it stays as printed."
        }
        TextKind::KeyHover => {
            "Meter key hovers: what a remote button does, or how to do it on the
meter. Keep the meter's key names as it prints them."
        }
        TextKind::Model => {
            "Model names with words in them: the simulated meters. Keep the part
number as it is."
        }
        TextKind::Activation => {
            "Activation steps: how to switch a meter's data transmission on, shown
when a connection gets no data. Keep each step on its own line, and the
meter's key names, menu items and display text as the meter prints them."
        }
        TextKind::Band => {
            "Accuracy bands, beside an accuracy: the condition it holds under, or an
accuracy the manual gives in words."
        }
        TextKind::Input => "Inputs: a table's input impedance or overload protection.",
        TextKind::Note => {
            "Notes, under a table. Keep the exact sense, comparisons and qualifiers
such as \"typical\" included."
        }
    }
}

/// One message as the file will hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placed {
    kind: TextKind,
    id: String,
    text: String,
}

/// The file's messages after adding `wanted` (sorted by kind then text) to
/// `existing`, a new one's ID made from `slug_of` its text, or the problems
/// that stop it: orphans, IDs that name no kind, messages that don't format,
/// repeats.
fn plan(
    existing: &[LibEntry],
    wanted: &[(TextKind, &str)],
    slug_of: impl Fn(TextKind, &str) -> String,
) -> Result<Vec<Placed>, Vec<String>> {
    let wanted_set: HashSet<(TextKind, &str)> = wanted.iter().copied().collect();
    let mut problems = Vec::new();
    let mut sections: BTreeMap<TextKind, Vec<Placed>> = BTreeMap::new();
    // A template's ID is taken too: the files share one set of IDs.
    let mut taken: HashSet<String> = spec_templates().iter().map(|t| t.id.clone()).collect();
    let mut seen: BTreeMap<(TextKind, &str), &str> = BTreeMap::new();
    for e in existing {
        let id = &e.id;
        let (Some(kind), Some(text)) = (e.kind, e.text.as_deref()) else {
            problems.push(match e.kind {
                None => format!("`{id}` names no kind: start it with one of {:?}", kinds()),
                Some(_) => format!("`{id}` does not format"),
            });
            continue;
        };
        if !taken.insert(id.clone()) {
            problems.push(format!("`{id}` is defined twice"));
            continue;
        }
        if let Some(first) = seen.insert((kind, text), id) {
            problems.push(format!("`{id}` repeats `{first}`: {text:?}"));
            continue;
        }
        if !wanted_set.contains(&(kind, text)) {
            problems.push(format!(
                "`{id}` = {text:?}: not {} text the library shows any more. \
                 Edit its English to the library's new text to keep its ID and \
                 translations, or delete it, in every language: a text a spec \
                 template now reads has no message of its own.",
                kind.name()
            ));
            continue;
        }
        sections.entry(kind).or_default().push(Placed {
            kind,
            id: id.clone(),
            text: text.to_string(),
        });
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    for &(kind, text) in wanted {
        if seen.contains_key(&(kind, text)) {
            continue;
        }
        let Some(id) = new_id(kind, &slug_of(kind, text), &taken) else {
            return Err(vec![format!(
                "no free ID reads back as {kind:?} for {text:?}: is it in TextKind::ALL?"
            )]);
        };
        taken.insert(id.clone());
        let section = sections.entry(kind).or_default();
        // After the last message that sorts before it, so a new label joins
        // its neighbours.
        let at = section
            .iter()
            .rposition(|p| p.text.as_str() < text)
            .map_or(0, |i| i + 1);
        section.insert(
            at,
            Placed {
                kind,
                id,
                text: text.to_string(),
            },
        );
    }
    Ok(sections.into_values().flatten().collect())
}

fn kinds() -> Vec<&'static str> {
    TextKind::ALL.iter().map(|k| k.name()).collect()
}

/// `<kind>-<slug>`, made unique with `-2`, `-3`…; `<kind>-<n>` when there
/// is no slug, or when it would read as another kind's ID. `None` when no
/// ID reads back as `kind` ([`kind_of_id`]): a kind missing from
/// [`TextKind::ALL`]. One more number than there are IDs taken is always
/// enough, so the search ends.
fn new_id(kind: TextKind, slug: &str, taken: &HashSet<String>) -> Option<String> {
    let fits = |id: &String| !taken.contains(id) && kind_of_id(id) == Some(kind);
    let base = format!("{}-{slug}", kind.name());
    let suffixed = (2..100).map(|n| format!("{base}-{n}"));
    let slugged = std::iter::once(base.clone()).chain(suffixed);
    let numbered = (1..=taken.len() + 1).map(|n| format!("{}-{n}", kind.name()));
    (!slug.is_empty())
        .then(|| slugged.into_iter().find(fits))
        .flatten()
        .or_else(|| numbered.into_iter().find(fits))
}

/// The first [`SLUG_WORDS`] words of `text`, lowercase ASCII joined by `-`:
/// `+` reads "plus" and `%` "pct", µ, Ω, ° and β are spelled out, other
/// characters split words or drop.
fn slug(text: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let end_word = |word: &mut String, words: &mut Vec<String>| {
        if !word.is_empty() {
            words.push(std::mem::take(word));
        }
    };
    for c in text.chars() {
        match c {
            c if c.is_ascii_alphanumeric() => word.push(c.to_ascii_lowercase()),
            'µ' | 'μ' => word.push('u'),
            'Ω' | '\u{2126}' => word.push_str("ohm"),
            '°' => word.push_str("deg"),
            'β' => word.push_str("beta"),
            '+' | '%' => {
                end_word(&mut word, &mut words);
                words.push(if c == '+' { "plus" } else { "pct" }.to_string());
            }
            _ => end_word(&mut word, &mut words),
        }
    }
    end_word(&mut word, &mut words);
    words.truncate(SLUG_WORDS);
    words.join("-")
}

/// The slug of a new message's ID: its text's, but for activation steps,
/// which start alike ("1. Connect the USB cable…"), the registry id of the
/// first meter using them.
fn slug_of(
    users: &BTreeMap<(TextKind, String), BTreeSet<&'static str>>,
) -> impl Fn(TextKind, &str) -> String {
    move |kind, text| match kind {
        TextKind::Activation => users
            .get(&(kind, text.to_string()))
            .and_then(|by| by.first())
            .map_or_else(|| slug(text), |id| slug(id)),
        _ => slug(text),
    }
}

/// What `all_texts` lists that `lib.ftl` carries, with the registry ids of
/// the devices using each.
fn wanted() -> BTreeMap<(TextKind, String), BTreeSet<&'static str>> {
    let all = all_texts();
    let of = |kind| {
        all.entries()
            .filter(move |(k, _, _)| *k == kind)
            .map(|(_, text, _)| text)
    };
    let modes: HashSet<&str> = of(TextKind::Mode).collect();
    let qualifiers: Vec<(&str, &str)> = of(TextKind::Qualifier)
        .filter_map(split_qualifier)
        .collect();
    let mut wanted: BTreeMap<(TextKind, String), BTreeSet<&'static str>> = BTreeMap::new();
    for (kind, text, by) in all.entries() {
        if kind == TextKind::Mode && qualified(text, &qualifiers, &modes) {
            continue;
        }
        // A spec text a template reads is listed as the texts in its text
        // slots, used by whoever uses it.
        for listed in listed_parts(kind, text) {
            if !is_units_only(listed) {
                wanted
                    .entry((kind, listed.to_string()))
                    .or_default()
                    .extend(by);
            }
        }
    }
    wanted
}

/// What the files list for `text`: itself, or, when a spec template reads
/// it ([`spec_templates`]), what the files list for each text its text slot
/// holds; a value slot's content is listed nowhere.
///
/// Panics on a text that reads more than one way: the templates overlap.
fn listed_parts(kind: TextKind, text: &str) -> Vec<&str> {
    let parses: Vec<(&str, Binding<'_>)> = spec_templates()
        .iter()
        .filter(|t| t.kind == kind)
        .flat_map(|t| t.parses(text).into_iter().map(|b| (t.id.as_str(), b)))
        .collect();
    match &parses[..] {
        [] => vec![text],
        [(_, binding)] => binding
            .iter()
            .filter(|(slot, _)| *slot == TEXT_SLOT)
            .flat_map(|(_, held)| listed_parts(kind, held))
            .collect(),
        several => panic!("{text:?} reads more than one way: {several:?}"),
    }
}

/// Whether mode `label` is a qualifier around a mode the library lists,
/// a unit-only one, or another such: what [`Localizer::label`] can take
/// apart.
///
/// [`Localizer::label`]: crate::l10n::Localizer::label
fn qualified(label: &str, qualifiers: &[(&str, &str)], modes: &HashSet<&str>) -> bool {
    qualifiers.iter().any(|&q| {
        unqualify(q, label).is_some_and(|mode| {
            modes.contains(mode) || is_units_only(mode) || qualified(mode, qualifiers, modes)
        })
    })
}

/// The spec tables each note, input and band sits under, by name.
fn tables() -> BTreeMap<(TextKind, &'static str), BTreeSet<&'static str>> {
    let mut out: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for device in registry::DEVICES {
        for table in (device.new_protocol)().spec_sheet() {
            let mode = table.mode;
            let inputs = [mode.input_impedance, mode.overload_protection];
            let bands = table
                .rows
                .iter()
                .flat_map(|row| row.spec.accuracy)
                .flat_map(|b| b.freq_range.into_iter().chain([b.accuracy]))
                .map(|t| (TextKind::Band, t));
            let texts = inputs
                .into_iter()
                .flatten()
                .map(|t| (TextKind::Input, t))
                .chain(mode.notes.iter().map(|n| (TextKind::Note, *n)))
                .chain(bands);
            for (kind, text) in texts {
                for listed in listed_parts(kind, text) {
                    out.entry((kind, listed)).or_default().insert(table.name);
                }
            }
        }
    }
    out
}

/// Adds to `read` the templates that read `text`, and those that read what
/// their text slots hold.
fn read_by(kind: TextKind, text: &str, read: &mut HashSet<&'static str>) {
    for template in spec_templates().iter().filter(|t| t.kind == kind) {
        for binding in template.parses(text) {
            read.insert(template.id.as_str());
            for (_, held) in binding.iter().filter(|(slot, _)| *slot == TEXT_SLOT) {
                read_by(kind, held, read);
            }
        }
    }
}

/// The share of their words two texts must have in common for one to be
/// [`near_template`] the other.
const NEAR: f32 = 0.6;

/// The words of a spec text that say something: lowercased, units aside.
fn said(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty() && !is_unit(w))
        .map(str::to_lowercase)
        .collect()
}

/// The spec template of `kind` that listed text `text` nearly reads: the
/// one sharing most of its words, at least [`NEAR`] of all the words the
/// two have. A note worded a little differently from a template ("Max.
/// 600V" beside "Max { $value }") is one more message to translate, so the
/// generator names the template when it adds such a text.
fn near_template(kind: TextKind, text: &str) -> Option<&'static Template> {
    if !GUARDED.contains(&kind) {
        return None;
    }
    let words = said(text);
    let share = |t: &Template| {
        let fixed = said(&t.fixed_text());
        let both = words.intersection(&fixed).count() as f32;
        both / (words.union(&fixed).count().max(1) as f32)
    };
    spec_templates()
        .iter()
        .filter(|t| t.kind == kind)
        .map(|t| (share(t), t))
        .filter(|(share, _)| *share >= NEAR)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, t)| t)
}

/// A comma-separated list of the first `named` of `items`, or their count
/// past that.
fn listing(items: &[&str], named: usize, counted: &str, separator: &str) -> String {
    if items.len() > named {
        format!("{} {counted}", items.len())
    } else {
        items.join(separator)
    }
}

/// `file` for the messages of `placed` it holds, comments included.
fn render(
    file: &File,
    placed: &[Placed],
    users: &BTreeMap<(TextKind, String), BTreeSet<&'static str>>,
    tables: &BTreeMap<(TextKind, &'static str), BTreeSet<&'static str>>,
) -> String {
    let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut body = vec![Entry::ResourceComment(Comment {
        content: lines(file.header),
    })];
    let mut kind = None;
    for p in placed.iter().filter(|p| file.holds(p.kind)) {
        if kind != Some(p.kind) {
            kind = Some(p.kind);
            body.push(Entry::GroupComment(Comment {
                content: lines(section(p.kind)),
            }));
        }
        let mut models: Vec<&str> = users
            .get(&(p.kind, p.text.clone()))
            .into_iter()
            .flatten()
            .filter_map(|id| registry::find_device(id).map(|d| d.model))
            .collect();
        models.sort_by_key(|m| m.to_lowercase());
        let mut comment = vec![format!(
            "Used by: {}",
            listing(&models, NAMED_USERS, "meters", ", ")
        )];
        if let Some(names) = tables.get(&(p.kind, p.text.as_str())) {
            let names: Vec<&str> = names.iter().copied().collect();
            comment.push(format!(
                "In: {}",
                listing(&names, NAMED_TABLES, "tables", "; ")
            ));
        }
        let elements = match split_qualifier(&p.text) {
            Some((before, after)) if p.kind == TextKind::Qualifier => {
                comment.push("$mode is the mode, already translated.".to_string());
                qualifier_pattern(before, after)
            }
            _ => escape(&p.text),
        };
        body.push(Entry::Message(Message {
            id: Identifier { name: p.id.clone() },
            value: Some(Pattern { elements }),
            attributes: vec![],
            comment: Some(Comment { content: comment }),
        }));
    }
    fluent_syntax::serializer::serialize(&Resource { body })
}

/// A qualifier's pattern: `before`, the `$mode` variable, `after`. Both are
/// a word and a space, which need no escaping; anything else is a library
/// change this would have to learn.
fn qualifier_pattern(before: &str, after: &str) -> Vec<PatternElement<String>> {
    for part in [before, after] {
        assert!(
            !part.contains(['{', '}', '\n']),
            "qualifier text {part:?} needs escaping"
        );
    }
    assert!(!before.starts_with([' ', '[', '*', '.']) && !after.ends_with(' '));
    let text = |value: &str| {
        (!value.is_empty()).then(|| PatternElement::TextElement {
            value: value.to_string(),
        })
    };
    let mode = PatternElement::Placeable {
        expression: Expression::Inline(InlineExpression::VariableReference {
            id: Identifier {
                name: "mode".to_string(),
            },
        }),
    };
    text(before)
        .into_iter()
        .chain([mode])
        .chain(text(after))
        .collect()
}

#[test]
fn lib_ftl_lists_the_library_text() {
    let current = FILES.map(|f| std::fs::read_to_string(f.path()).unwrap_or_default());
    let existing: Vec<LibEntry> = current.iter().flat_map(|c| lib_entries(c)).collect();
    let users = wanted();
    let wanted: Vec<(TextKind, &str)> = users.keys().map(|(k, t)| (*k, t.as_str())).collect();
    let placed = match plan(&existing, &wanted, slug_of(&users)) {
        Ok(placed) => placed,
        Err(problems) => panic!("i18n/en-US/{LIB} and {LIB_SPECS}:\n{}", problems.join("\n")),
    };
    // A template no library text reads any more is left for the maintainer,
    // as an orphaned message is.
    let all = all_texts();
    let mut read = HashSet::new();
    for (kind, text, _) in all.entries() {
        read_by(kind, text, &mut read);
    }
    let unread: Vec<&str> = spec_templates()
        .iter()
        .map(|t| t.id.as_str())
        .filter(|id| !read.contains(id))
        .collect();
    assert!(
        unread.is_empty(),
        "i18n/en-US/lib-spec-templates.ftl: no library text reads {unread:?}: delete them, \
         in every language"
    );
    // New spec text that a template nearly reads: its author may have meant
    // the template's wording.
    let known: HashSet<&str> = existing.iter().map(|e| e.id.as_str()).collect();
    let near: Vec<String> = placed
        .iter()
        .filter(|p| !known.contains(p.id.as_str()))
        .filter_map(|p| {
            let t = near_template(p.kind, &p.text)?;
            Some(format!(
                "  {:?} is close to {} = {}",
                p.text,
                t.id,
                t.english()
            ))
        })
        .collect();
    let hint = if near.is_empty() {
        String::new()
    } else {
        format!(
            "\nNew spec text close to a template:\n{}\nWhere one means the same as its \
             template, word it that way in the spec table: it then needs no translation.",
            near.join("\n")
        )
    };
    let tables = tables();
    let mut stale = Vec::new();
    for (file, current) in FILES.iter().zip(&current) {
        let rendered = render(file, &placed, &users, &tables);
        if rendered == *current {
            continue;
        }
        if std::env::var_os("UPDATE_L10N").is_some() {
            crate::write_atomic(&file.path(), rendered.as_bytes()).expect("write the file");
        } else {
            let added = placed
                .iter()
                .filter(|p| file.holds(p.kind) && !current.contains(&format!("\n{} =", p.id)))
                .count();
            stale.push(format!("{} ({added} new)", file.name));
        }
    }
    assert!(
        stale.is_empty(),
        "i18n/en-US is out of date: {}; \
         run `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl` and commit it{hint}",
        stale.join(", ")
    );
    // An update that added such text says so once: the next run passes.
    assert!(
        hint.is_empty(),
        "i18n/en-US is updated.{hint}\nOtherwise run again: the files are current."
    );
}

/// Every message the generator writes formats back, through Fluent, to
/// exactly the library's text, each kind in its file.
#[test]
fn the_written_messages_read_back_as_the_library_wrote_them() {
    let users = wanted();
    let wanted: Vec<(TextKind, &str)> = users.keys().map(|(k, t)| (*k, t.as_str())).collect();
    let placed = plan(&[], &wanted, slug_of(&users)).unwrap();
    for file in &FILES {
        let rendered = render(file, &placed, &users, &tables());
        let read: Vec<(Option<TextKind>, Option<String>)> = lib_entries(&rendered)
            .into_iter()
            .map(|e| (e.kind, e.text))
            .collect();
        let expected: Vec<(Option<TextKind>, Option<String>)> = wanted
            .iter()
            .filter(|(k, _)| file.holds(*k))
            .map(|(k, t)| (Some(*k), Some(t.to_string())))
            .collect();
        assert_eq!(read, expected, "{}", file.name);
    }
}

#[test]
fn a_text_worded_almost_as_a_template_is_near_it() {
    let near = |text| near_template(TextKind::Note, text).map(|t| t.id.as_str());
    assert_eq!(near("Max. 600V"), Some("note-max-reading"));
    assert_eq!(
        near("10A: ≤5A continuous; >5A–10A max 10s, then pause >15min"),
        Some("note-current-time-limit")
    );
    // Other words, or another kind: not near.
    assert_eq!(near("Max 1000β"), None);
    assert_eq!(near("K-type thermocouple only"), None);
    assert!(near_template(TextKind::Mode, "Max").is_none());
}

#[test]
fn text_fluent_would_read_as_syntax_is_escaped() {
    let tricky = [
        "Range {auto} \"quoted\" back\\slash",
        "* starts with a star\n[x] starts with a bracket\n.starts with a dot",
        "   leading spaces\nand trailing ones   ",
        "a line\n\nafter a blank line\n  indented { brace }",
        "#not a comment\n-not a term",
    ];
    let placed: Vec<Placed> = tricky
        .iter()
        .enumerate()
        .map(|(i, t)| Placed {
            kind: TextKind::Note,
            id: format!("note-{i}"),
            text: t.to_string(),
        })
        .collect();
    let rendered = render(&FILES[1], &placed, &BTreeMap::new(), &BTreeMap::new());
    let read: Vec<String> = lib_entries(&rendered)
        .into_iter()
        .map(|e| {
            e.text
                .unwrap_or_else(|| panic!("{} fails:\n{rendered}", e.id))
        })
        .collect();
    assert_eq!(read, tricky, "{rendered}");
}

#[test]
fn ids_are_slugs_of_the_text() {
    assert_eq!(slug("AC+DC mA"), "ac-plus-dc-ma");
    assert_eq!(slug("Temp °C T1-T2"), "temp-degc-t1-t2");
    assert_eq!(slug("Duty % REL"), "duty-pct-rel");
    assert_eq!(slug("200µA"), "200ua");
    assert_eq!(slug("LoZ Ω"), "loz-ohm");
    assert_eq!(
        slug("Hold the meter's REL key (1 kHz low-pass filter)"),
        "hold-the-meter-s-rel-key"
    );
    assert_eq!(slug("ᛒ"), "");
}

fn entry(id: &str, text: &str) -> LibEntry {
    LibEntry {
        id: id.to_string(),
        kind: kind_of_id(id),
        text: Some(text.to_string()),
    }
}

#[test]
fn the_generator_only_adds() {
    let existing = [entry("mode-dc-v", "DC V"), entry("mode-ac-v", "AC V")];
    let wanted = [
        (TextKind::Mode, "AC V"),
        (TextKind::Mode, "Capacitance"),
        (TextKind::Mode, "DC V"),
        (TextKind::Mode, "DC V!"),
        (TextKind::Aux, "ᛒ"),
        (TextKind::Key, "Hover x"),
    ];
    let placed = plan(&existing, &wanted, |_, text| slug(text)).unwrap();
    let ids: Vec<&str> = placed.iter().map(|p| p.id.as_str()).collect();
    // Existing messages keep their IDs and order; a new one goes after the
    // last that sorts before it, a slug taken gets a number, and so does a
    // text with no slug or one that would read as another kind.
    assert_eq!(
        ids,
        [
            "mode-dc-v",
            "mode-ac-v",
            "mode-capacitance",
            "mode-dc-v-2",
            "aux-1",
            "key-1"
        ]
    );
}

/// The numbered search is bounded, and the bound leaves a number free.
#[test]
fn a_number_is_always_free() {
    let taken: HashSet<String> = (1..=3).map(|n| format!("aux-{n}")).collect();
    assert_eq!(new_id(TextKind::Aux, "", &taken).as_deref(), Some("aux-4"));
    let taken: HashSet<String> = (1..=2).map(|n| format!("key-{n}")).collect();
    // "key-hover-x" would read as a key hover.
    assert_eq!(
        new_id(TextKind::Key, "hover-x", &taken).as_deref(),
        Some("key-3")
    );
}

#[test]
fn an_orphan_stops_the_generator() {
    let existing = [entry("mode-dc-v", "DC V"), entry("mode-gone", "Gone")];
    let problems = plan(&existing, &[(TextKind::Mode, "DC V")], |_, t| slug(t)).unwrap_err();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("`mode-gone`"), "{problems:?}");
    let problems = plan(&[entry("nokind-x", "X")], &[], |_, t| slug(t)).unwrap_err();
    assert!(problems[0].contains("names no kind"), "{problems:?}");
}
