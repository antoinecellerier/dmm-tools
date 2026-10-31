//! The library's labels in the user's language, looked up by their English
//! text and their [`TextKind`].
//!
//! `i18n/en-US/lib.ftl` is generated from `dmm_lib::text::all_texts()` (the
//! generator is the `lib_ftl` test): one message per (kind, English text),
//! whose English is the lookup key. Each language's `lib.ftl` translates
//! those IDs. Labels made only of numbers and unit symbols are left out
//! ([`is_units_only`]) and always show as the library wrote them.

use super::{Localizer, Resource};
use dmm_lib::text::TextKind;
use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The library's text: the generated `lib.ftl` and the hand-written
/// `lib-names.ftl`, in every language the binaries ship. A binary that draws
/// library text lists these with its own resources.
pub const LIB_RESOURCES: &[Resource] = &[crate::ftl!("en-US", "lib.ftl")];

/// Unit symbols, without prefix. With [`PREFIXES`] they are what stays as
/// it is in every language: a label whose letters are nothing but these
/// ("200mV", "-40~1000°C", "kΩ") is not translated, and a spec note keeps
/// every number with its unit ([`spec_numbers`]). `°C` and `°F` count as
/// one symbol each; the Ohm sign is listed beside the Greek capital omega
/// the library writes, so a translation using either still matches.
const UNITS: &[&str] = &[
    "V", "A", "Ω", "\u{2126}", "Hz", "F", "S", "W", "VA", "Wh", "Ah", "s", "dB", "dBm", "dBV",
    "°C", "°F",
];

/// The SI prefixes a [`UNITS`] symbol may take, `u` standing in for `µ`.
const PREFIXES: &[&str] = &["p", "n", "µ", "u", "m", "k", "M", "G"];

/// Whether `word` is a unit symbol, with or without an SI prefix.
fn is_unit(word: &str) -> bool {
    UNITS.contains(&word)
        || PREFIXES
            .iter()
            .any(|p| word.strip_prefix(p).is_some_and(|u| UNITS.contains(&u)))
}

/// The runs of letters in `text`, a `°` joining the letters after it.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphabetic() || c == '°'))
        .filter(|w| !w.is_empty())
}

/// Whether every word of `text` is a unit symbol ([`UNITS`]): "200mV",
/// "4-20mA", "°C". Such a label has nothing to translate, so `lib.ftl`
/// leaves it out and its lookup falls back to the English. "AC", "Auto" and
/// "DC V" are words some languages translate, so they are listed.
#[cfg(test)]
pub(crate) fn is_units_only(text: &str) -> bool {
    words(text).all(is_unit)
}

/// The numbers of `text`, each with the unit symbol or `%` right after it,
/// spacing dropped: "About 10 MΩ" gives `["10MΩ"]`, the same as
/// "About 10MΩ". A translated spec note must give the same numbers as its
/// English (in any order: compare them sorted), or it is not shown.
///
/// A number is digits with `.` decimals ("0.44"); a sign or `±` is not part
/// of it.
pub fn spec_numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        rest = &rest[start..];
        let digits = |s: &str| s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        let mut end = digits(rest);
        // `.` followed by a digit continues the number.
        while rest[end..].starts_with('.')
            && rest[end + 1..].starts_with(|c: char| c.is_ascii_digit())
        {
            end += 1 + digits(&rest[end + 1..]);
        }
        let mut number = rest[..end].to_string();
        rest = &rest[end..];
        let after_space = rest
            .strip_prefix(|c: char| c.is_whitespace())
            .unwrap_or(rest);
        if let Some(r) = after_space.strip_prefix('%') {
            number.push('%');
            rest = r;
        } else if let Some(unit) = words(after_space)
            .next()
            .filter(|w| after_space.starts_with(*w) && is_unit(w))
        {
            number.push_str(unit);
            rest = &after_space[unit.len()..];
        }
        out.push(number);
    }
    out
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

/// The messages of an en-US `lib.ftl` `source`, in file order.
pub(crate) fn lib_entries(source: &str) -> Vec<LibEntry> {
    let resource = match FluentResource::try_new(source.to_string()) {
        Ok(r) => r,
        Err((r, errors)) => {
            log::warn!("l10n: en-US/lib.ftl has syntax errors: {errors:?}");
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
    ids.into_iter()
        .map(|id| {
            let text = super::format(&bundle, &id, None).map(Cow::into_owned);
            LibEntry {
                kind: kind_of_id(&id),
                id,
                text,
            }
        })
        .collect()
}

/// (kind, English) → message ID, from the embedded en-US `lib.ftl`.
type Index = HashMap<TextKind, HashMap<String, String>>;

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = Index::new();
        for entry in lib_entries(LIB_RESOURCES[0].source) {
            if let (Some(kind), Some(text)) = (entry.kind, entry.text) {
                index.entry(kind).or_default().insert(text, entry.id);
            }
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
    pub fn label<'a>(&'a self, kind: TextKind, english: &'a str) -> Cow<'a, str> {
        if self.plain_english() {
            return Cow::Borrowed(english);
        }
        index()
            .get(&kind)
            .and_then(|by_text| by_text.get(english))
            .and_then(|id| self.lookup(id, &[]))
            .unwrap_or(Cow::Borrowed(english))
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
            ["10A", "10A", "20A", "30s", "5"]
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
            ["600Ω", "42.22", "02.22dBm"]
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
            LIB_RESOURCES,
            &[Resource {
                locale: "fr",
                path: "lib.ftl",
                source: "mode-capacitance = Capacité\n",
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

    /// A binary that doesn't list `lib.ftl` shows the English, without the
    /// missing-message assertion `tr!` makes.
    #[test]
    fn a_binary_without_lib_ftl_shows_english() {
        let l = Localizer::new(&FRENCH, &[], &Options::default());
        assert_eq!(l.label(TextKind::Mode, "Capacitance"), "Capacitance");
    }
}
