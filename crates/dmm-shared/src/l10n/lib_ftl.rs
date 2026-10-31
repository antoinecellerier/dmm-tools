//! The generator of `i18n/en-US/lib.ftl`, run as a test: the file must hold
//! one message per text `dmm_lib::text::all_texts()` lists, unit-only
//! labels aside ([`is_units_only`]).
//!
//! `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl` adds what is missing;
//! a plain run fails until then. The generator only adds: an existing
//! message keeps its ID and its place, and one whose English the library
//! no longer shows is an orphan the test names and leaves for the
//! maintainer — edit its English to the library's new text to keep its ID
//! and translations, or delete it. Comments are rewritten every time.

use super::ftl_text::escape;
use super::lib_index::{
    LibEntry, is_units_only, kind_of_id, lib_entries, split_qualifier, unqualify,
};
use dmm_lib::protocol::registry;
use dmm_lib::text::{TextKind, all_texts};
use fluent_syntax::ast::{
    Comment, Entry, Expression, Identifier, InlineExpression, Message, Pattern, PatternElement,
    Resource,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../i18n/en-US/lib.ftl");

/// Words of a text kept in its ID.
const SLUG_WORDS: usize = 6;

/// Users and spec tables named in an entry's comment before they are
/// counted instead.
const NAMED_USERS: usize = 6;
const NAMED_TABLES: usize = 3;

const HEADER: &str = "\
Text the library shows: the labels of readings and of the choices a meter
offers, meter keys, the steps that switch a meter's data transmission on,
and the specification tables.

Generated from dmm-lib by `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl`.
Entries are only ever added. The English of each one is what the library
shows and is how the app finds it: edit it only to follow a change in the
library, keeping the ID to keep its translations.

Numbers and unit symbols stay as they are in every language. Labels made of
nothing else (200mV, kΩ, °C) are not listed: they always show as they are.
Nor are modes made of a qualifier around another mode (LPF AC V): the
qualifier is listed once.";

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
            "Meter keys: the remote panel's buttons, named after the meter's keys.
Keep a name the meter prints on its key (SEL, ZERO) as it is."
        }
        TextKind::KeyHover => {
            "Meter key hovers: how to do on the meter what a remote button does.
Keep the meter's key names as it prints them."
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
        TextKind::Table => {
            "Specification table names, in the Specifications panel, as the
meter's manual titles them. Keep the manual's numbering: (1), A."
        }
        TextKind::Input => {
            "Specification inputs: a table's input impedance or overload
protection. Every number and unit must stay: a translation that changes,
drops or adds one is shown in English."
        }
        TextKind::Note => {
            "Specification notes, under a table in the Specifications panel, from
the meter's manual. Keep the exact sense, comparisons and qualifiers such as
\"typical\" included. Every number and unit must stay: a translation that
changes, drops or adds one is shown in English."
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
    let mut taken = HashSet::new();
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
                "`{id}` = {text:?}: not {} text lib.ftl lists any more. \
                 Edit its English to the library's new text to keep its ID and \
                 translations, or delete it.",
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
    all.entries()
        .filter(|(kind, text, _)| {
            !is_units_only(text)
                && !(*kind == TextKind::Mode && qualified(text, &qualifiers, &modes))
        })
        .map(|(kind, text, by)| ((kind, text.to_string()), by.clone()))
        .collect()
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

/// The spec tables each note and input sits under, by name.
fn tables() -> BTreeMap<(TextKind, &'static str), BTreeSet<&'static str>> {
    let mut out: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for device in registry::DEVICES {
        for table in (device.new_protocol)().spec_sheet() {
            let mode = table.mode;
            let inputs = [mode.input_impedance, mode.overload_protection];
            let texts = inputs
                .into_iter()
                .flatten()
                .map(|t| (TextKind::Input, t))
                .chain(mode.notes.iter().map(|n| (TextKind::Note, *n)));
            for key in texts {
                out.entry(key).or_default().insert(table.name);
            }
        }
    }
    out
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

/// The file for `placed`, comments included.
fn render(
    placed: &[Placed],
    users: &BTreeMap<(TextKind, String), BTreeSet<&'static str>>,
    tables: &BTreeMap<(TextKind, &'static str), BTreeSet<&'static str>>,
) -> String {
    let lines = |text: &str| text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut body = vec![Entry::ResourceComment(Comment {
        content: lines(HEADER),
    })];
    let mut kind = None;
    for p in placed {
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
    let path = Path::new(PATH);
    let current = std::fs::read_to_string(path).unwrap_or_default();
    let users = wanted();
    let wanted: Vec<(TextKind, &str)> = users.keys().map(|(k, t)| (*k, t.as_str())).collect();
    let placed = match plan(&lib_entries(&current), &wanted, slug_of(&users)) {
        Ok(placed) => placed,
        Err(problems) => panic!("i18n/en-US/lib.ftl:\n{}", problems.join("\n")),
    };
    let rendered = render(&placed, &users, &tables());
    if rendered == current {
        return;
    }
    if std::env::var_os("UPDATE_L10N").is_some() {
        crate::write_atomic(path, rendered.as_bytes()).expect("write lib.ftl");
    } else {
        let added: Vec<&str> = placed
            .iter()
            .filter(|p| !current.contains(&format!("\n{} =", p.id)))
            .map(|p| p.id.as_str())
            .collect();
        panic!(
            "i18n/en-US/lib.ftl is out of date ({} new, starting {:?}); \
             run `UPDATE_L10N=1 cargo test -p dmm-shared lib_ftl` and commit it",
            added.len(),
            &added[..added.len().min(5)]
        );
    }
}

/// Every message the generator writes formats back, through Fluent, to
/// exactly the library's text.
#[test]
fn the_written_messages_read_back_as_the_library_wrote_them() {
    let users = wanted();
    let wanted: Vec<(TextKind, &str)> = users.keys().map(|(k, t)| (*k, t.as_str())).collect();
    let placed = plan(&[], &wanted, slug_of(&users)).unwrap();
    let rendered = render(&placed, &users, &tables());
    let read: Vec<(Option<TextKind>, Option<String>)> = lib_entries(&rendered)
        .into_iter()
        .map(|e| (e.kind, e.text))
        .collect();
    let expected: Vec<(Option<TextKind>, Option<String>)> = wanted
        .iter()
        .map(|(k, t)| (Some(*k), Some(t.to_string())))
        .collect();
    assert_eq!(read, expected);
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
    let rendered = render(&placed, &BTreeMap::new(), &BTreeMap::new());
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
