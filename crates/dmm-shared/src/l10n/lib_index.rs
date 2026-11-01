//! The library's labels in the user's language, looked up by their English
//! text and their [`TextKind`].
//!
//! `i18n/en-US/lib.ftl` is generated from `dmm_lib::text::all_texts()` (the
//! generator is the `lib_ftl` test): one message per (kind, English text),
//! whose English is the lookup key. Each language's `lib.ftl` translates
//! those IDs. Labels made only of numbers and unit symbols are left out
//! ([`is_units_only`]) and always show as the library wrote them.
//!
//! A mode label a family builds by putting a qualifier around another mode
//! ("LPF Peak AC V") is not listed either: the qualifier is, once, as a
//! message taking the translated mode as `$mode` ([`TextKind::Qualifier`]),
//! and the lookup takes the label apart ([`Localizer::label`]).

use super::{Arg, Localizer, Resource};
use dmm_lib::flags::Flag;
use dmm_lib::text::{MODE_SLOT, TextKind};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The library's text: the generated `lib.ftl` and the hand-written
/// `lib-names.ftl`, `lib-errors.ftl` and `lib-alarm.ftl`, in every language the binaries
/// ship. A binary that draws library text or errors lists these with its
/// own resources.
pub const LIB_RESOURCES: &[Resource] = &[
    crate::ftl!("en-US", "lib.ftl"),
    crate::ftl!("en-US", "lib-names.ftl"),
    crate::ftl!("en-US", "lib-errors.ftl"),
    crate::ftl!("en-US", "lib-alarm.ftl"),
    crate::ftl!("fr", "lib.ftl"),
    crate::ftl!("fr", "lib-names.ftl"),
    crate::ftl!("fr", "lib-errors.ftl"),
    crate::ftl!("fr", "lib-alarm.ftl"),
    crate::ftl!("pt-BR", "lib.ftl"),
    crate::ftl!("pt-BR", "lib-names.ftl"),
    crate::ftl!("pt-BR", "lib-errors.ftl"),
    crate::ftl!("pt-BR", "lib-alarm.ftl"),
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

/// What the embedded en-US `lib.ftl` maps English to.
#[derive(Default)]
struct Index {
    /// (kind, English) → message ID.
    labels: HashMap<TextKind, HashMap<String, String>>,
    /// Each qualifier's text before and after its mode, with its ID.
    qualifiers: Vec<(String, String, String)>,
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = Index::default();
        for entry in lib_entries(LIB_RESOURCES[0].source) {
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
    /// `lib.ftl` doesn't list. Each message is checked once, the first time
    /// any spec text is asked for, and the verdicts kept. Other kinds are
    /// [`Localizer::label`]'s.
    pub fn spec_text<'a>(&'a self, kind: TextKind, english: &'a str) -> Cow<'a, str> {
        if self.plain_english() || !GUARDED.contains(&kind) {
            return self.label(kind, english);
        }
        index()
            .labels
            .get(&kind)
            .and_then(|ids| ids.get(english))
            .and_then(|id| self.checked_spec_texts().get(id))
            .map_or(Cow::Borrowed(english), |t| Cow::Borrowed(t.as_str()))
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
        let all = dmm_lib::text::all_texts();
        for (kind, text, _) in all.entries() {
            if kind == TextKind::Qualifier {
                continue;
            }
            if kind == TextKind::Mode {
                assert_eq!(l.mode_label(text).as_deref(), Some(text));
            } else if !is_units_only(text) {
                assert_eq!(l.listed(kind, text).as_deref(), Some(text));
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

    /// A French `lib.ftl` translating spec text: some translations keep
    /// their English's numbers, some don't.
    fn french_specs() -> Localizer {
        let ids = |kind: TextKind, english: &str| index().labels[&kind][english].clone();
        let note = ids(
            TextKind::Note,
            "Lo-Z threshold: > +1.5VDC or < -1.0VDC nominal",
        );
        let open = ids(TextKind::Note, "Open circuit ≈-1.2V");
        let tempco = ids(
            TextKind::Note,
            "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)",
        );
        let input = ids(TextKind::Input, "About 10MΩ");
        let band = ids(TextKind::Band, "In REL state");
        let lpf = ids(TextKind::Band, "40Hz~100Hz (LPF)");
        let source = format!(
            "{note} = Seuil Lo-Z : > 1,5 VCC ou < -1,0 VCC nominal\n\
             {open} = Circuit ouvert ≈-1,2 V\n\
             {tempco} = 60,00 MΩ : coefficient 0,6× la précision/°C @(0 °C ~ 18 °C ou 28 °C ~ 45 °C)\n\
             {input} = Environ 10 MΩ\n\
             {band} = En mode REL\n\
             {lpf} = 40 Hz~1000 Hz (LPF)\n"
        );
        let resources = [
            &english_lib()[..],
            &[Resource {
                locale: "fr",
                path: "lib.ftl",
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
            l.spec_text(TextKind::Note, "Open circuit ≈-1.2V"),
            "Circuit ouvert ≈-1,2 V"
        );
        assert_eq!(
            l.spec_text(
                TextKind::Note,
                "60.00MΩ: tempco 0.6× accuracy/°C @(0°C ~ 18°C or 28°C ~ 45°C)"
            ),
            "60,00 MΩ : coefficient 0,6× la précision/°C @(0 °C ~ 18 °C ou 28 °C ~ 45 °C)"
        );
        assert_eq!(l.spec_text(TextKind::Input, "About 10MΩ"), "Environ 10 MΩ");
        assert_eq!(l.spec_text(TextKind::Band, "In REL state"), "En mode REL");
        // `label` takes the same guard.
        assert_eq!(l.label(TextKind::Input, "About 10MΩ"), "Environ 10 MΩ");
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
        // Text lib.ftl doesn't list, and untranslated text, stay English.
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
