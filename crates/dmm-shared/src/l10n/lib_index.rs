//! The library's labels in the user's language, looked up by their English
//! text and their [`TextKind`].
//!
//! `i18n/en-US/lib.ftl` and `lib-specs.ftl` (the specification text) are
//! generated from `dmm_lib::text::all_texts()` (the generator is the
//! `lib_ftl` test): one message per (kind, English text), whose English is
//! the lookup key. Each language's files translate those IDs. Labels made only of numbers and unit symbols are left out
//! ([`is_units_only`]) and always show as the library wrote them.
//!
//! A mode label a family builds by putting a qualifier around another mode
//! ("LPF Peak AC V") is not listed either: the qualifier is, once, as a
//! message taking the translated mode as `$mode` ([`TextKind::Qualifier`]),
//! and the lookup takes the label apart ([`Localizer::label`]).

use super::spec_templates::{LIB_SPEC_TEMPLATES, TEXT_SLOT, Template, templates};
use super::{Arg, Localizer, Resource};
use dmm_lib::flags::Flag;
use dmm_lib::text::{MODE_SLOT, TextKind};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The generated file of the library's labels, keys and activation steps.
pub(crate) const LIB: &str = "lib.ftl";

/// The generated file of the specification text ([`GUARDED`]): optional in
/// a translation, shown in English where left out.
pub(crate) const LIB_SPECS: &str = "lib-specs.ftl";

/// The library's text: the generated `lib.ftl` and `lib-specs.ftl` and the
/// hand-written `lib-spec-templates.ftl`, `lib-names.ftl`, `lib-errors.ftl` and `lib-alarm.ftl`, in every language the binaries
/// ship. A binary that draws library text or errors lists these with its
/// own resources.
pub const LIB_RESOURCES: &[Resource] = &[
    crate::ftl!("en-US", "lib.ftl"),
    crate::ftl!("en-US", "lib-specs.ftl"),
    crate::ftl!("en-US", "lib-spec-templates.ftl"),
    crate::ftl!("en-US", "lib-names.ftl"),
    crate::ftl!("en-US", "lib-errors.ftl"),
    crate::ftl!("en-US", "lib-alarm.ftl"),
    crate::ftl!("fr", "lib.ftl"),
    crate::ftl!("fr", "lib-specs.ftl"),
    crate::ftl!("fr", "lib-spec-templates.ftl"),
    crate::ftl!("fr", "lib-names.ftl"),
    crate::ftl!("fr", "lib-errors.ftl"),
    crate::ftl!("fr", "lib-alarm.ftl"),
    crate::ftl!("pt-BR", "lib.ftl"),
    crate::ftl!("pt-BR", "lib-specs.ftl"),
    crate::ftl!("pt-BR", "lib-spec-templates.ftl"),
    crate::ftl!("pt-BR", "lib-names.ftl"),
    crate::ftl!("pt-BR", "lib-errors.ftl"),
    crate::ftl!("pt-BR", "lib-alarm.ftl"),
    crate::ftl!("zh-CN", "lib.ftl"),
    crate::ftl!("zh-CN", "lib-specs.ftl"),
    crate::ftl!("zh-CN", "lib-spec-templates.ftl"),
    crate::ftl!("zh-CN", "lib-names.ftl"),
    crate::ftl!("zh-CN", "lib-errors.ftl"),
    crate::ftl!("zh-CN", "lib-alarm.ftl"),
    crate::ftl!("es", "lib.ftl"),
    crate::ftl!("es", "lib-specs.ftl"),
    crate::ftl!("es", "lib-spec-templates.ftl"),
    crate::ftl!("es", "lib-names.ftl"),
    crate::ftl!("es", "lib-errors.ftl"),
    crate::ftl!("es", "lib-alarm.ftl"),
    crate::ftl!("ja", "lib.ftl"),
    crate::ftl!("ja", "lib-specs.ftl"),
    crate::ftl!("ja", "lib-spec-templates.ftl"),
    crate::ftl!("ja", "lib-names.ftl"),
    crate::ftl!("ja", "lib-errors.ftl"),
    crate::ftl!("ja", "lib-alarm.ftl"),
];

/// The en-US half of [`LIB_RESOURCES`]: what a test that makes up its own
/// translation starts from, so the shipped ones don't stand in for it.
#[cfg(test)]
pub(crate) fn english_lib() -> Vec<Resource> {
    LIB_RESOURCES
        .iter()
        .filter(|r| r.locale == super::EN_US.id)
        .copied()
        .collect()
}

/// Unit symbols, without prefix. With [`PREFIXES`] they are what stays as
/// it is in every language: a label whose letters are nothing but these
/// ("200mV", "-40~1000°C", "kΩ") is not translated, and a spec text keeps
/// every number with its unit ([`spec_numbers`]). `°C` and `°F` count as
/// one symbol each; the Ohm sign is listed beside the Greek capital omega
/// the library writes, and [`spec_numbers`] reads it as that omega, so a
/// translation using either still matches.
const UNITS: &[&str] = &[
    "V", "A", "Ω", "\u{2126}", "Hz", "F", "S", "W", "VA", "Wh", "Ah", "s", "dB", "dBm", "dBV",
    "°C", "°F",
];

/// The SI prefixes a [`UNITS`] symbol may take, `u` standing in for `µ`.
const PREFIXES: &[&str] = &["p", "n", "µ", "u", "m", "k", "M", "G"];

/// Time units a spec text writes after a number, beside [`UNITS`]' `s`:
/// "30 s", "5 min", "2 h". `sec` reads as `s`. Units after a number only,
/// as [`spec_unit`] reads them: a label of these is still words.
const TIME_UNITS: &[&str] = &["min", "h", "sec"];

/// What a unit after a number may end in to say which current it is:
/// "1.5VDC", "3VAC", or as French writes them "V CC", "V CA". A spec text
/// compares the unit without it, so "1.5VDC" and "1,5 V CC" agree.
const CURRENT_KINDS: &[&str] = &["DC", "AC", "CC", "CA"];

/// Whether `word` is a unit symbol, with or without an SI prefix.
fn is_symbol(word: &str) -> bool {
    UNITS.contains(&word)
        || PREFIXES
            .iter()
            .any(|p| word.strip_prefix(p).is_some_and(|u| UNITS.contains(&u)))
}

/// The unit `word` is when it follows a number, as a spec text compares
/// it: a unit symbol with or without an SI prefix, or a time unit
/// ([`TIME_UNITS`], `sec` read as `s`), less any current kind written onto
/// it ([`CURRENT_KINDS`]): "mVDC" is `mV`. `None` for any other word.
fn spec_unit(word: &str) -> Option<&str> {
    fn unit(w: &str) -> Option<&str> {
        if w == "sec" {
            Some("s")
        } else {
            (is_symbol(w) || TIME_UNITS.contains(&w)).then_some(w)
        }
    }
    unit(word).or_else(|| {
        CURRENT_KINDS
            .iter()
            .find_map(|k| word.strip_suffix(k).and_then(unit))
    })
}

/// Whether `word` is a unit when it follows a number ([`spec_unit`]): the
/// pseudo-locale leaves such a word as it is written.
pub(super) fn is_unit(word: &str) -> bool {
    spec_unit(word).is_some()
}

/// The runs of letters in `text`, a `°` joining the letters after it.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphabetic() || c == '°'))
        .filter(|w| !w.is_empty())
}

/// Whether every word of `text` is a unit symbol ([`UNITS`]) or a bare SI
/// prefix whose unit comes later: "200mV", "4-20mA", "°C", "10k~100kHz".
/// Such a label has nothing to translate, so `lib.ftl` leaves it out and its
/// lookup falls back to the English. "AC", "Auto" and "DC V" are words some
/// languages translate, so they are listed.
pub fn is_units_only(text: &str) -> bool {
    words(text).all(|w| is_symbol(w) || PREFIXES.contains(&w))
}

/// Whether `text` is one value as a spec text writes it: one number with
/// its sign, a comparator before it and a unit after it ("4.000", ">5A",
/// "+400 °C", "≈-1.2V"), and nothing that separates clauses. What a value
/// slot of a spec template holds ([`super::spec_templates`]).
pub(super) fn is_value(text: &str) -> bool {
    !text.is_empty()
        && text.trim() == text
        && !text.contains([':', ',', ';', '(', ')'])
        && spec_numbers(text).len() == 1
        && words(text).all(|w| spec_unit(w).is_some() || PREFIXES.contains(&w))
}

/// How many composed spec texts a [`Localizer`] keeps: several times what
/// the library's tables hold.
const COMPOSED_CAP: usize = 4096;

/// The kinds of spec text a translation may only show with the English's
/// numbers ([`Localizer::spec_text`]).
pub(super) const GUARDED: [TextKind; 3] = [TextKind::Band, TextKind::Input, TextKind::Note];

/// The numbers of `text`, each with its sign and the unit or `%` right
/// after it, spacing dropped: "About 10 MΩ" gives `["10MΩ"]`, the same as
/// "About 10MΩ".
///
/// - A number is digits with decimals after a `.` or, as many languages
///   write it, a `,` followed by a digit: "0,6" gives "0.6". No separator
///   groups thousands: "1 000" is two numbers, "1" and "000".
/// - A `-`, `−` or `+` right before the digits is its sign when nothing
///   word-like comes before it: "-40°C", "≈-1.2V" and "> +1.5V" carry one,
///   while "4-20mA" is a span and "0.5%+3" a sum.
/// - A `±` belongs to the number after it, spaces allowed ("± 0.5" gives
///   "±0.5"); one before anything else ("±(0.5%+3)") is a number of its own.
/// - The unit is the SI symbol or time unit [`spec_unit`] reads: "1.5VDC"
///   gives "1.5V", as do "1.5 V DC" and "1,5 V CC" (the current's word
///   after the unit is a word of its own, not compared), and "5 min" gives
///   "5min", which "5 h" does not.
/// - The Ohm sign reads as the Greek capital omega.
fn spec_numbers(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '±' {
            let mut j = i + 1;
            while chars.get(j).is_some_and(|c| c.is_whitespace()) {
                j += 1;
            }
            if !digit(j) {
                out.push("±".to_string());
            }
            i += 1;
            continue;
        }
        if !digit(i) {
            i += 1;
            continue;
        }
        let mut number = String::new();
        // The sign or ± before the digits, read back past any spaces a
        // ± may have.
        let mut back = i;
        while back > 0 && chars[back - 1].is_whitespace() {
            back -= 1;
        }
        if back > 0 && chars[back - 1] == '±' {
            number.push('±');
        } else if i > 0 && matches!(chars[i - 1], '-' | '\u{2212}' | '+') {
            let spans = i > 1
                && (chars[i - 2].is_alphanumeric() || matches!(chars[i - 2], '%' | ')' | '.'));
            if !spans {
                number.push(if chars[i - 1] == '+' { '+' } else { '-' });
            }
        }
        while digit(i) {
            number.push(chars[i]);
            i += 1;
            // A point or comma followed by a digit continues the number.
            if matches!(chars.get(i), Some('.' | ',')) && digit(i + 1) {
                number.push('.');
                i += 1;
            }
        }
        let mut after = i;
        while chars.get(after).is_some_and(|c| c.is_whitespace()) {
            after += 1;
        }
        if chars.get(after) == Some(&'%') {
            number.push('%');
            i = after + 1;
        } else {
            let rest: String = chars[after..].iter().collect();
            if let Some((word, unit)) = words(&rest)
                .next()
                .filter(|w| rest.starts_with(*w))
                .and_then(|w| spec_unit(w).map(|u| (w, u)))
            {
                number.push_str(&unit.replace('\u{2126}', "Ω"));
                i = after + word.chars().count();
            }
        }
        out.push(number);
    }
    out
}

/// Whether `translated` carries the numbers of `english` ([`spec_numbers`]),
/// in any order.
pub(super) fn same_numbers(english: &str, translated: &str) -> bool {
    let sorted = |text| {
        let mut numbers = spec_numbers(text);
        numbers.sort();
        numbers
    };
    sorted(english) == sorted(translated)
}

/// The kind an ID names: its prefix before the first `-` that leaves a
/// [`TextKind::name`], the longest one winning (`key-hover-…` over `key-…`).
pub(crate) fn kind_of_id(id: &str) -> Option<TextKind> {
    TextKind::ALL
        .into_iter()
        .filter(|k| {
            id.strip_prefix(k.name())
                .is_some_and(|rest| rest.starts_with('-'))
        })
        .max_by_key(|k| k.name().len())
}

/// One message of an en-US `lib.ftl`.
#[derive(Debug)]
pub(crate) struct LibEntry {
    pub id: String,
    /// `None` for an ID that names no kind.
    pub kind: Option<TextKind>,
    /// Its English as Fluent formats it, `None` when it fails to.
    pub text: Option<String>,
}

/// The messages of an en-US `lib.ftl` or `lib-specs.ftl` `source`, in file
/// order.
pub(crate) fn lib_entries(source: &str) -> Vec<LibEntry> {
    let resource = match FluentResource::try_new(source.to_string()) {
        Ok(r) => r,
        Err((r, errors)) => {
            log::warn!("l10n: an en-US library file has syntax errors: {errors:?}");
            r
        }
    };
    let ids: Vec<String> = resource
        .entries()
        .filter_map(|e| match e {
            fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_string()),
            _ => None,
        })
        .collect();
    let mut bundle = FluentBundle::new_concurrent(vec![
        super::EN_US.id.parse().expect("en-US is a language id"),
    ]);
    bundle.set_use_isolating(false);
    // A repeated ID keeps its first message, as everywhere else.
    let _ = bundle.add_resource(resource);
    // A qualifier's English is its pattern, the mode slot standing in for
    // `$mode`.
    let mut slot = FluentArgs::new();
    slot.set("mode", MODE_SLOT);
    ids.into_iter()
        .map(|id| {
            let args = (kind_of_id(&id) == Some(TextKind::Qualifier)).then_some(&slot);
            let text = super::format(&bundle, &id, args).map(Cow::into_owned);
            LibEntry {
                kind: kind_of_id(&id),
                id,
                text,
            }
        })
        .collect()
}

/// A qualifier pattern's text before and after its mode slot, `None` for
/// a pattern without one.
pub(crate) fn split_qualifier(pattern: &str) -> Option<(&str, &str)> {
    pattern.split_once(MODE_SLOT)
}

/// The mode `label` puts qualifier `(before, after)` around, `None` when
/// it doesn't.
pub(crate) fn unqualify<'t>((before, after): (&str, &str), label: &'t str) -> Option<&'t str> {
    let mode = label.strip_prefix(before)?.strip_suffix(after)?;
    (!mode.is_empty()).then_some(mode)
}

/// What the embedded en-US `lib.ftl` and `lib-specs.ftl` map English to.
#[derive(Default)]
struct Index {
    /// (kind, English) → message ID.
    labels: HashMap<TextKind, HashMap<String, String>>,
    /// Each qualifier's text before and after its mode, with its ID.
    qualifiers: Vec<(String, String, String)>,
    /// The spec templates, in file order.
    templates: Vec<Template>,
}

/// The spec templates of the embedded en-US `lib-spec-templates.ftl`.
#[cfg(test)]
pub(crate) fn spec_templates() -> &'static [Template] {
    &index().templates
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = Index::default();
        let generated = LIB_RESOURCES
            .iter()
            .filter(|r| r.locale == super::EN_US.id && [LIB, LIB_SPECS].contains(&r.path));
        for entry in generated.flat_map(|r| lib_entries(r.source)) {
            let (Some(kind), Some(text)) = (entry.kind, entry.text) else {
                continue;
            };
            if kind == TextKind::Qualifier
                && let Some((before, after)) = split_qualifier(&text)
            {
                let id = entry.id.clone();
                index.qualifiers.push((before.into(), after.into(), id));
            }
            index.labels.entry(kind).or_default().insert(text, entry.id);
        }
        let hand_written = LIB_RESOURCES
            .iter()
            .find(|r| r.locale == super::EN_US.id && r.path == LIB_SPEC_TEMPLATES);
        match hand_written.map(|r| templates(r.source)) {
            Some(Ok(templates)) => index.templates = templates,
            Some(Err(problems)) => {
                log::warn!("l10n: en-US/{LIB_SPEC_TEMPLATES}: {problems:?}");
            }
            None => {}
        }
        index
    })
}

impl Localizer {
    /// Library text `english` of `kind` in this language: a mode, range or
    /// sub-value label, a key name, an activation step, a spec table's text.
    ///
    /// English borrows `english` without a lookup. Text `lib.ftl` doesn't
    /// list (unit-only labels, the library's `Unknown(0x..)` fallbacks) and
    /// text this binary has no messages for come back unchanged.
    ///
    /// A mode `lib.ftl` lacks is tried as a qualifier around a mode it has,
    /// or around a unit-only one, which stays as it is: "LPF Peak AC V" is
    /// the LPF qualifier around the Peak qualifier around "AC V".
    ///
    /// A [`TextKind::Toggle`] state named after a flag (MAX, P-MIN) is that
    /// flag's name ([`Localizer::flag`]): `lib.ftl` lists only on and off.
    /// Spec notes, inputs and bands go through [`Localizer::spec_text`].
    pub fn label<'a>(&'a self, kind: TextKind, english: &'a str) -> Cow<'a, str> {
        if self.plain_english() {
            return Cow::Borrowed(english);
        }
        let found = match kind {
            kind if GUARDED.contains(&kind) => return self.spec_text(kind, english),
            TextKind::Mode => self.mode_label(english),
            TextKind::Toggle => Flag::ALL
                .into_iter()
                .find(|f| f.label() == Some(english))
                .and_then(|f| self.flag(f))
                .or_else(|| self.listed(kind, english)),
            TextKind::Aux if english == dmm_lib::transform::RAW_LABEL => Some(self.raw_label()),
            kind => self.listed(kind, english),
        };
        found.unwrap_or(Cow::Borrowed(english))
    }

    /// Spec text `english` of `kind` in this language: a note, an input
    /// (impedance or overload protection) or an accuracy band.
    ///
    /// A translation shows only if it carries the English's numbers, each
    /// with its sign, ± and unit ([`spec_numbers`]), in any order: one that
    /// changes, drops or adds a number is shown in English, as is text
    /// neither `lib-specs.ftl` lists nor a spec template reads. Each
    /// message is checked once, the first time any spec text is asked for,
    /// and each composed text the first time it is, and the verdicts kept.
    /// Other kinds are [`Localizer::label`]'s.
    pub fn spec_text<'a>(&'a self, kind: TextKind, english: &'a str) -> Cow<'a, str> {
        if self.plain_english() || !GUARDED.contains(&kind) {
            return self.label(kind, english);
        }
        if let Some(id) = index().labels.get(&kind).and_then(|ids| ids.get(english)) {
            return self
                .checked_spec_texts()
                .get(id)
                .map_or(Cow::Borrowed(english), |t| Cow::Borrowed(t.as_str()));
        }
        // Numbers and units alone are never translated, in a template or
        // out of one.
        if is_units_only(english) {
            return Cow::Borrowed(english);
        }
        self.composed(kind, english)
            .map_or(Cow::Borrowed(english), Cow::Owned)
    }

    /// [`Localizer::compose`]'s text for `english` where it keeps the
    /// numbers, from the cache after the first time. The cache holds the
    /// library's worded spec texts, which are what is asked for, and
    /// [`COMPOSED_CAP`] texts at most: past it a text is composed each time.
    fn composed(&self, kind: TextKind, english: &str) -> Option<String> {
        let cache = || {
            self.composed_spec_texts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        };
        if let Some(known) = cache().get(&kind).and_then(|texts| texts.get(english)) {
            return known.clone();
        }
        let made = match self.compose(kind, english) {
            Some(Ok(text)) => Some(text),
            Some(Err(text)) => {
                log::warn!(
                    "l10n: {}: {text:?} changes the numbers of {english:?}: \
                     showing the English",
                    self.language.id
                );
                None
            }
            None => None,
        };
        let mut cache = cache();
        if cache.values().map(HashMap::len).sum::<usize>() < COMPOSED_CAP {
            cache
                .entry(kind)
                .or_default()
                .insert(english.to_string(), made.clone());
        }
        made
    }

    /// `english` through the spec template that reads it: `None` when none
    /// does, `Err` with the text when it changes the English's numbers
    /// ([`same_numbers`]).
    pub(crate) fn compose(&self, kind: TextKind, english: &str) -> Option<Result<String, String>> {
        let raw = self.compose_raw(kind, english)?;
        let shown = if self.pseudo {
            super::pseudo::bracket(&raw)
        } else {
            raw
        };
        Some(if same_numbers(english, &shown) {
            Ok(shown)
        } else {
            Err(shown)
        })
    }

    /// [`Localizer::compose`] before the pseudo-locale's brackets, which go
    /// around the whole text once. Only through a template the language
    /// has: with en-US's a text would show half translated, where one
    /// without a translation shows in English.
    fn compose_raw(&self, kind: TextKind, english: &str) -> Option<String> {
        let templates = index()
            .templates
            .iter()
            .filter(|t| t.kind == kind && self.has_own(&t.id));
        for template in templates {
            for binding in template.parses(english) {
                let args: Option<Vec<(&str, Arg<'_>)>> = binding
                    .into_iter()
                    .map(|(slot, held)| {
                        let held = if slot == TEXT_SLOT {
                            Cow::Owned(self.spec_part(kind, held)?)
                        } else {
                            Cow::Borrowed(held)
                        };
                        Some((slot, Arg::Str(held)))
                    })
                    .collect();
                if let Some(args) = args {
                    return self.lookup_raw(&template.id, &args).map(Cow::into_owned);
                }
            }
        }
        None
    }

    /// What a template's text slot shows for `english`: its message, what
    /// another template makes of it, or itself when it is numbers and units
    /// only. `None` for anything else: the template doesn't read the text.
    fn spec_part(&self, kind: TextKind, english: &str) -> Option<String> {
        if let Some(id) = index().labels.get(&kind).and_then(|ids| ids.get(english)) {
            let text = if self.pseudo {
                self.lookup_raw(id, &[])?
            } else {
                self.checked_spec_texts()
                    .get(id)
                    .map_or(Cow::Borrowed(english), |t| Cow::Borrowed(t.as_str()))
            };
            return Some(text.into_owned());
        }
        self.compose_raw(kind, english)
            .or_else(|| is_units_only(english).then(|| english.to_string()))
    }

    /// Each [`GUARDED`] message's text in this language, by ID, where it
    /// differs from the English and keeps its numbers.
    fn checked_spec_texts(&self) -> &HashMap<String, String> {
        self.spec_texts.get_or_init(|| {
            let mut out = HashMap::new();
            for kind in GUARDED {
                for (english, id) in index().labels.get(&kind).into_iter().flatten() {
                    let Some(text) = self.lookup(id, &[]) else {
                        continue;
                    };
                    if text == english.as_str() {
                        continue;
                    }
                    if same_numbers(english, &text) {
                        out.insert(id.clone(), text.into_owned());
                    } else {
                        log::warn!(
                            "l10n: {}: `{id}` changes the numbers of {english:?}: \
                             showing the English",
                            self.language.id
                        );
                    }
                }
            }
            out
        })
    }

    /// `english`'s own message in this language, `None` when `lib.ftl` lacks
    /// it or this binary has no messages for it.
    fn listed(&self, kind: TextKind, english: &str) -> Option<Cow<'_, str>> {
        let id = index().labels.get(&kind)?.get(english)?;
        self.lookup(id, &[])
    }

    /// Mode `english` in this language, through qualifiers where it has no
    /// message of its own; `None` when it doesn't come apart into messages
    /// and unit-only modes.
    fn mode_label<'a>(&'a self, english: &'a str) -> Option<Cow<'a, str>> {
        if let Some(found) = self.listed(TextKind::Mode, english) {
            return Some(found);
        }
        for (before, after, id) in &index().qualifiers {
            if let Some(mode) = unqualify((before, after), english)
                && let Some(mode) = self.mode_label(mode)
            {
                return self.lookup(id, &[("mode", Arg::from(mode))]);
            }
        }
        is_units_only(english).then_some(Cow::Borrowed(english))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::{EN_US, Language, Options};

    #[test]
    fn unit_symbols_and_numbers_alone_are_not_translated() {
        for text in [
            "200mV",
            "-40~1000°C",
            "1000V",
            "100nF",
            "Hz",
            "mA",
            "V",
            "kΩ",
            "4-20mA",
            "°C",
            "2.2MΩ",
            "60nS",
            "mA%",
            "% 4-20mA",
            "25000µVA",
            "dBm",
        ] {
            assert!(is_units_only(text), "{text}");
        }
        for text in [
            "AC",
            "DC",
            "AC+DC",
            "REL",
            "Auto",
            "Peak",
            "A AC",
            "AC+DC mA",
            "DC V",
            "Temp °C",
            "1000β",
            "LoZ V~",
            "Hz/Duty",
            "hFE",
            "Fuse 0.5A 250V",
            "250V rms",
        ] {
            assert!(!is_units_only(text), "{text}");
        }
    }

    #[test]
    fn spec_numbers_keep_their_units_and_drop_spacing() {
        assert_eq!(spec_numbers("About 10MΩ"), ["10MΩ"]);
        assert_eq!(spec_numbers("Environ 10\u{202f}MΩ"), ["10MΩ"]);
        assert_eq!(
            spec_numbers("10MΩ, 50pF nominal"),
            spec_numbers("10 MΩ, 50 pF nominal")
        );
        assert_eq!(
            spec_numbers("10A continuous; >10A to 20A: 30s max, 5 min cool-down"),
            ["10A", "10A", "20A", "30s", "5min"]
        );
        assert_eq!(
            spec_numbers("60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)"),
            ["60.00MΩ", "0.6", "0°C", "18°C", "28°C", "45°C"]
        );
        assert_eq!(
            spec_numbers("60.00mV, 600.0mV, 1kHz ~ 20kHz: specified 30% to 100% of range"),
            ["60.00mV", "600.0mV", "1kHz", "20kHz", "30%", "100%"]
        );
        assert_eq!(
            spec_numbers("600Ω ref, typical: ACmV -42.22 to -02.22dBm"),
            ["600Ω", "-42.22", "-02.22dBm"]
        );
        assert_eq!(
            spec_numbers("4mA shows 0 %, 20mA shows 100 %"),
            ["4mA", "0%", "20mA", "100%"]
        );
        // A word that only starts like a unit is not one.
        assert_eq!(spec_numbers("2 digits, 3 Max"), ["2", "3"]);
        assert!(spec_numbers("no numbers. Here").is_empty());
    }

    #[test]
    fn spec_numbers_keep_signs_and_plus_minus() {
        assert_eq!(spec_numbers("-40°C to 0°C"), ["-40°C", "0°C"]);
        assert_eq!(spec_numbers("−40 °C"), ["-40°C"]);
        assert_eq!(spec_numbers("Open circuit ≈-1.2V"), ["-1.2V"]);
        assert_eq!(spec_numbers("> +1.5VDC or < -1.0VDC"), ["+1.5V", "-1.0V"]);
        assert_eq!(spec_numbers("-40~-10°C"), ["-40", "-10°C"]);
        assert_eq!(spec_numbers("±0.5% typical"), ["±0.5%"]);
        assert_eq!(spec_numbers("± 0.5 %"), ["±0.5%"]);
        assert_eq!(spec_numbers("±(0.5%+3)"), ["±", "0.5%", "3"]);
        // A dash between numbers is a span, a plus after one a sum.
        assert_eq!(spec_numbers("4-20mA"), ["4", "20mA"]);
        assert_eq!(spec_numbers("0.5%+3"), ["0.5%", "3"]);
        assert_eq!(spec_numbers("T1-T2"), ["1", "2"]);
    }

    #[test]
    fn spec_numbers_read_a_decimal_comma_and_the_ohm_sign() {
        assert_eq!(spec_numbers("0,6× la précision"), ["0.6"]);
        assert_eq!(spec_numbers("Environ 10 M\u{2126}"), ["10MΩ"]);
        // A comma before a space separates, and nothing groups thousands.
        assert_eq!(spec_numbers("60.00mV, 600.0mV"), ["60.00mV", "600.0mV"]);
        assert_eq!(spec_numbers("1 000V"), ["1", "000V"]);
    }

    #[test]
    fn a_translation_must_keep_the_numbers() {
        let english = "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or -10°C)";
        for kept in [
            "60,00 MΩ : coefficient 0,6× la précision/°C @(0 °C ~ 18 °C ou -10 °C)",
            "(-10°C, 18°C ~ 0°C) 0.6× 60.00MΩ",
        ] {
            assert!(same_numbers(english, kept), "{kept}");
        }
        for changed in [
            "60.00MΩ: 0.5× @(0°C ~ 18°C or -10°C)",
            "60.00MΩ: 0.6× @(0°C ~ 18°C or 10°C)",
            "60.00MΩ: 0.6× @(0°C ~ 18°C)",
            "60.00MΩ: 0.6× @(0°C ~ 18°C or -10°C) 2",
            "60.00kΩ: 0.6× @(0°C ~ 18°C or -10°C)",
        ] {
            assert!(!same_numbers(english, changed), "{changed}");
        }
        assert!(!same_numbers("±0.5%", "0.5%"));
        assert!(!same_numbers("1000V", "1 000V"));
    }

    /// The unit is the SI symbol: a current's DC or AC, run onto it or a
    /// word after it, is not compared.
    #[test]
    fn a_unit_compares_without_its_current() {
        let english = "> +1.5VDC or < -1.0VDC, 3VAC, 200mVDC";
        for kept in [
            "> +1,5 V CC ou < -1,0 V CC, 3 V CA, 200 mV CC",
            "> +1.5 V DC or < -1.0 V DC, 3 V AC, 200 mV DC",
            "> +1,5 VCC ou < -1,0 VCC, 3 VCA, 200 mVCC",
        ] {
            assert!(same_numbers(english, kept), "{kept}");
        }
        assert_eq!(spec_numbers("3 VCA"), ["3V"]);
        assert!(!same_numbers("3VAC", "3 mV CA"));
        assert!(!same_numbers("3VAC", "3 A CA"));
        // A word that only ends like a current is not a unit.
        assert_eq!(spec_numbers("2 DC"), ["2"]);
    }

    /// Time units count as units, so a translation can't change one for
    /// another.
    #[test]
    fn time_units_are_units() {
        assert_eq!(
            spec_numbers("10min, 2 h, 30 s, 5 ms"),
            ["10min", "2h", "30s", "5ms"]
        );
        assert!(same_numbers("10 min", "10min"));
        assert!(same_numbers("30 sec", "30 s"));
        assert!(!same_numbers("10min", "10 h"));
        assert!(!same_numbers("30 s", "30 ms"));
        assert!(!same_numbers("10min", "10"));
    }

    #[test]
    fn ids_name_their_kind() {
        assert_eq!(kind_of_id("mode-dc-v"), Some(TextKind::Mode));
        assert_eq!(kind_of_id("key-hover-press"), Some(TextKind::KeyHover));
        assert_eq!(kind_of_id("key-sel"), Some(TextKind::Key));
        assert_eq!(kind_of_id("modes-x"), None);
        assert_eq!(kind_of_id("mode"), None);
    }

    static FRENCH: Language = Language {
        id: "fr",
        native_name: "Français",
        reviewed: true,
    };

    fn french() -> Localizer {
        let resources = [
            &english_lib()[..],
            &[Resource {
                locale: "fr",
                path: "lib.ftl",
                source: "mode-capacitance = Capacité\n",
            }],
            &[Resource {
                locale: "fr",
                path: "lib-names.ftl",
                source: "raw-label = Brut\n",
            }],
        ]
        .concat();
        Localizer::new(&FRENCH, &resources, &Options::default())
    }

    #[test]
    fn labels_are_looked_up_by_kind_and_english() {
        let l = french();
        assert_eq!(l.label(TextKind::Mode, "Capacitance"), "Capacité");
        // Another kind with the same English is another message.
        assert_eq!(l.label(TextKind::Aux, "Capacitance"), "Capacitance");
        // A scaled reading's raw sub-value is named, not listed.
        assert_eq!(l.label(TextKind::Aux, "Raw"), "Brut");
        // Untranslated, unlisted and unknown text stays English.
        assert_eq!(l.label(TextKind::Mode, "Diode"), "Diode");
        assert_eq!(l.label(TextKind::Range, "200mV"), "200mV");
        assert_eq!(l.label(TextKind::Mode, "Unknown(0x05)"), "Unknown(0x05)");
    }

    #[test]
    fn english_borrows_its_input() {
        let l = Localizer::en_us(LIB_RESOURCES);
        let english = String::from("Capacitance");
        assert!(matches!(
            l.label(TextKind::Mode, &english),
            Cow::Borrowed(s) if std::ptr::eq(s, english.as_str())
        ));
    }

    #[test]
    fn the_pseudo_locale_reaches_listed_labels_only() {
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let l = Localizer::new(&EN_US, LIB_RESOURCES, &options);
        let pseudo = l.label(TextKind::Mode, "Capacitance");
        assert!(pseudo.starts_with('[') && pseudo.ends_with(']'), "{pseudo}");
        assert_eq!(l.label(TextKind::Range, "200mV"), "200mV");
    }

    /// A language without its own `lib.ftl`: every lookup goes through the
    /// messages, en-US's.
    fn untranslated() -> Localizer {
        Localizer::new(&FRENCH, &english_lib(), &Options::default())
    }

    /// Every label the library lists comes back from the en-US messages as
    /// the library wrote it, a composed mode through its qualifiers.
    #[test]
    fn every_label_reads_back_through_the_messages() {
        let l = untranslated();
        // Templates compose only in a language that has them.
        let english = Localizer::en_us(LIB_RESOURCES);
        let all = dmm_lib::text::all_texts();
        for (kind, text, _) in all.entries() {
            if kind == TextKind::Qualifier {
                continue;
            }
            if kind == TextKind::Mode {
                assert_eq!(l.mode_label(text).as_deref(), Some(text));
            } else if !is_units_only(text) {
                // Its own message, or the spec template that reads it.
                let read = l
                    .listed(kind, text)
                    .map(Cow::into_owned)
                    .or_else(|| english.compose_raw(kind, text));
                assert_eq!(read.as_deref(), Some(text));
            }
        }
    }

    /// A band a template reads is made of the template and of what its
    /// text slot holds, each in the language; numbers and units stay.
    #[test]
    fn a_template_composes_spec_text_from_its_parts() {
        let resources = [
            &english_lib()[..],
            &[
                Resource {
                    locale: "fr",
                    path: "lib-spec-templates.ftl",
                    source: "band-at-4000-counts = { $text } (4000 points)\n\
                             band-temperature-from-to = { $from } à { $to }\n",
                },
                Resource {
                    locale: "fr",
                    path: "lib-specs.ftl",
                    source: "band-30-100-pct-of-mr = 30 - 100% de la plage\n",
                },
            ],
        ]
        .concat();
        let l = Localizer::new(&FRENCH, &resources, &Options::default());
        let band = |english| l.spec_text(TextKind::Band, english).into_owned();
        assert_eq!(
            band("30 - 100% of MR (4000 counts)"),
            "30 - 100% de la plage (4000 points)"
        );
        assert_eq!(band("45Hz~1kHz (4000 counts)"), "45Hz~1kHz (4000 points)");
        // A template's text may be another template's.
        assert_eq!(
            band("+40 to +400 °C (4000 counts)"),
            "+40 à +400 °C (4000 points)"
        );
        // A part that is a message without a translation stays English
        // inside the template.
        assert_eq!(band("3d/kHz+2d (4000 counts)"), "3d/kHz+2d (4000 points)");
        // A part read by a template the language lacks: English whole.
        assert_eq!(
            band("250kHz~400kHz, 10 - 30% of MR (4000 counts)"),
            "250kHz~400kHz, 10 - 30% of MR (4000 counts)"
        );
        // Text no template reads stays English whole.
        assert_eq!(band("Sometimes: 4.000"), "Sometimes: 4.000");
    }

    /// A template whose translation changes a written number shows the
    /// English, as a message does.
    #[test]
    fn a_template_that_changes_a_number_shows_english() {
        let resources = [
            &english_lib()[..],
            &[Resource {
                locale: "fr",
                path: "lib-spec-templates.ftl",
                source: "band-at-4000-counts = { $text } (400 points)\n",
            }],
        ]
        .concat();
        let l = Localizer::new(&FRENCH, &resources, &Options::default());
        assert_eq!(
            l.compose(TextKind::Band, "45Hz~1kHz (4000 counts)"),
            Some(Err("45Hz~1kHz (400 points)".to_string()))
        );
        assert_eq!(
            l.spec_text(TextKind::Band, "45Hz~1kHz (4000 counts)"),
            "45Hz~1kHz (4000 counts)"
        );
    }

    /// No shipped language's templates change a number of any spec text
    /// the library shows: the app would show those in English.
    #[test]
    fn shipped_templates_keep_the_numbers() {
        let all = dmm_lib::text::all_texts();
        for language in crate::l10n::lang::SHIPPED {
            let l = Localizer::new(language, LIB_RESOURCES, &Options::default());
            for (kind, text, _) in all.entries().filter(|(k, _, _)| GUARDED.contains(k)) {
                if let Some(Err(shown)) = l.compose(kind, text) {
                    panic!("{}: {shown:?} changes the numbers of {text:?}", language.id);
                }
            }
        }
    }

    #[test]
    fn qualifiers_translate_once_and_may_move() {
        let resources = [
            &english_lib()[..],
            &[Resource {
                locale: "fr",
                path: "lib.ftl",
                source: "mode-ac-v = V CA\nqualifier-lpf-mode = { $mode } filtré\n\
                         qualifier-peak-mode = { $mode } crête\n",
            }],
        ]
        .concat();
        let l = Localizer::new(&FRENCH, &resources, &Options::default());
        assert_eq!(l.label(TextKind::Mode, "LPF AC V"), "V CA filtré");
        assert_eq!(
            l.label(TextKind::Mode, "LPF Peak AC V"),
            "V CA crête filtré"
        );
        // A unit-only mode stays; a qualifier this language lacks is English.
        assert_eq!(l.label(TextKind::Mode, "Peak °C"), "°C crête");
        assert_eq!(l.label(TextKind::Mode, "AC V peak"), "V CA peak");
        // Text that doesn't come apart stays English whole.
        assert_eq!(l.label(TextKind::Mode, "LPF Nonsense"), "LPF Nonsense");
    }

    /// A French `lib-specs.ftl`: some translations keep their English's
    /// numbers, some don't.
    fn french_specs() -> Localizer {
        let ids = |kind: TextKind, english: &str| index().labels[&kind][english].clone();
        let note = ids(
            TextKind::Note,
            "Lo-Z threshold: > +1.5VDC or < -1.0VDC nominal",
        );
        let probe = ids(TextKind::Note, "Supplied wire thermocouple: max. +230°C");
        let tempco = ids(
            TextKind::Note,
            "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)",
        );
        let input = ids(TextKind::Input, "250V rms");
        let band = ids(TextKind::Band, "In REL mode");
        let lpf = ids(TextKind::Band, "40Hz~100Hz (LPF)");
        let source = format!(
            "{note} = Seuil Lo-Z : > 1,5 VCC ou < -1,0 VCC nominal\n\
             {probe} = Thermocouple filaire fourni : max. +230 °C\n\
             {tempco} = 60,00 MΩ : coefficient 0,6× la précision/°C @(0 °C ~ 18 °C ou 28 °C ~ 45 °C)\n\
             {input} = 250 V eff.\n\
             {band} = En mode REL\n\
             {lpf} = 40 Hz~1000 Hz (LPF)\n"
        );
        let resources = [
            &english_lib()[..],
            &[Resource {
                locale: "fr",
                path: "lib-specs.ftl",
                source: Box::leak(source.into_boxed_str()),
            }],
        ]
        .concat();
        Localizer::new(&FRENCH, &resources, &Options::default())
    }

    #[test]
    fn spec_text_shows_a_translation_that_keeps_its_numbers() {
        let l = french_specs();
        assert_eq!(
            l.spec_text(TextKind::Note, "Supplied wire thermocouple: max. +230°C"),
            "Thermocouple filaire fourni : max. +230 °C"
        );
        assert_eq!(
            l.spec_text(
                TextKind::Note,
                "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)"
            ),
            "60,00 MΩ : coefficient 0,6× la précision/°C @(0 °C ~ 18 °C ou 28 °C ~ 45 °C)"
        );
        let input = "250V rms";
        assert_eq!(l.spec_text(TextKind::Input, input), "250 V eff.");
        assert_eq!(l.spec_text(TextKind::Band, "In REL mode"), "En mode REL");
        // `label` takes the same guard.
        assert_eq!(l.label(TextKind::Input, input), "250 V eff.");
    }

    #[test]
    fn spec_text_shows_the_english_where_a_number_changed() {
        let l = french_specs();
        // The + of +1.5 is gone.
        let english = "Lo-Z threshold: > +1.5VDC or < -1.0VDC nominal";
        assert_eq!(l.spec_text(TextKind::Note, english), english);
        assert_eq!(l.label(TextKind::Note, english), english);
        // 100Hz became 1000 Hz.
        assert_eq!(
            l.spec_text(TextKind::Band, "40Hz~100Hz (LPF)"),
            "40Hz~100Hz (LPF)"
        );
        // Text lib-specs.ftl doesn't list, and untranslated text, stay English.
        assert_eq!(l.spec_text(TextKind::Band, "45Hz~1kHz"), "45Hz~1kHz");
        assert_eq!(l.spec_text(TextKind::Input, "1000V"), "1000V");
    }

    #[test]
    fn english_spec_text_borrows_its_input() {
        let l = Localizer::en_us(LIB_RESOURCES);
        let english = String::from("About 10MΩ");
        assert!(matches!(
            l.spec_text(TextKind::Input, &english),
            Cow::Borrowed(s) if std::ptr::eq(s, english.as_str())
        ));
    }

    /// The pseudo-locale keeps numbers and the units after them, so every
    /// listed spec text passes the guard and shows bracketed.
    #[test]
    fn pseudo_spec_text_passes_the_guard() {
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let l = Localizer::new(&EN_US, LIB_RESOURCES, &options);
        for kind in GUARDED {
            for english in index().labels[&kind].keys() {
                let shown = l.spec_text(kind, english);
                assert!(shown.starts_with('['), "{kind:?} {english:?}: {shown}");
            }
        }
    }

    /// A binary that doesn't list `lib.ftl` shows the English, without the
    /// missing-message assertion `tr!` makes.
    #[test]
    fn a_binary_without_lib_ftl_shows_english() {
        let l = Localizer::new(&FRENCH, &[], &Options::default());
        assert_eq!(l.label(TextKind::Mode, "Capacitance"), "Capacitance");
    }
}
