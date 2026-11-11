//! Toolbar text-edit buffers paired with the values they parse to.
//!
//! Each pair used to be two `Graph` fields kept in step by convention: the
//! draft the user is typing, and the number the graph draws from. Nothing
//! stopped one being written without the other — the bbox zoom writes both,
//! and did so through four assignments — and the rule that an unparseable
//! draft leaves the last good value alone was restated at every edit site.
//!
//! A decade step re-expresses the value at once, but the text only once the
//! field is not being typed in: rewriting a draft under the caret would
//! change what the user is in the middle of typing. Until then a draft that
//! reads as a number is read in the prefix it was typed in, `pending` taking
//! it to the current one. A draft with no number in it (blank, a lone `-`)
//! has no prefix yet: what is typed into it is in the one the axis shows.

use super::axes::short_figure;
use crate::text_marks::{MarkStyle, TextMark};
use dmm_lib::transform::{
    PrefixRatio, base_unit, meter_unit, number_and_unit, prefix_ratio, typed_unit,
};
use std::ops::Range;

/// Digits the fields show when a value is written into them by the graph
/// rather than typed. Enough to keep a millivolt reading distinct, few
/// enough to stay editable.
const SET_PRECISION: usize = 4;

/// `text` re-expressed by `ratio` when it reads as a number, as
/// [`short_figure`] writes it; `None` when
/// it doesn't, for a draft the user is still typing to be left as it is.
fn rescaled_text(text: &str, ratio: PrefixRatio) -> Option<String> {
    Some(short_figure(ratio.apply(text.trim().parse::<f64>().ok()?)))
}

/// `pending` and then `ratio`: the step from the prefix a draft was typed in
/// to the one after this step, `None` once the steps cancel out.
fn compose(pending: Option<PrefixRatio>, ratio: PrefixRatio) -> Option<PrefixRatio> {
    let ratio = pending.map_or(ratio, |p| p.then(ratio));
    (!ratio.is_identity()).then_some(ratio)
}

/// Whether any entry of `text` reads as a number: a draft with a prefix.
fn holds_a_number(text: &str) -> bool {
    text.split(SEPARATORS)
        .any(|s| s.trim().parse::<f64>().is_ok())
}

/// A number the user types, beside the value it last parsed to.
#[derive(Debug)]
pub(super) struct NumberField {
    text: String,
    value: f64,
    /// The text is in an earlier prefix: this takes it to the value's.
    pending: Option<PrefixRatio>,
}

impl NumberField {
    /// A field showing `text`, which the caller has to make a rendering of
    /// `value`.
    pub(super) fn new(text: &str, value: f64) -> Self {
        Self {
            text: text.to_string(),
            value,
            pending: None,
        }
    }

    /// The draft, for a text edit to write into.
    pub(super) fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// The draft as it stands. Only the text edit reads it in anger, and it
    /// does that through `text_mut`.
    #[cfg(test)]
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn value(&self) -> f64 {
        self.value
    }

    /// Re-read the draft after an edit, keeping the last good value if it
    /// doesn't parse — the user may be mid-type. Returns whether the draft
    /// was taken.
    pub(super) fn parse(&mut self) -> bool {
        self.parse_if(|_| true)
    }

    /// As [`parse`](Self::parse), but only takes values `accept` allows.
    pub(super) fn parse_if(&mut self, accept: impl Fn(f64) -> bool) -> bool {
        let parsed = match self.pending {
            // As the flushed text will read.
            Some(ratio) => rescaled_text(&self.text, ratio).and_then(|t| t.parse().ok()),
            None => self.text.parse::<f64>().ok(),
        };
        if parsed.is_none() {
            // Nothing left in the draft's old prefix: what comes is typed
            // against the axis as it is.
            self.pending = None;
        }
        match parsed {
            Some(v) if accept(v) => {
                self.value = v;
                true
            }
            _ => false,
        }
    }

    /// Write a computed value into both halves, so the field shows the number
    /// the graph is using.
    pub(super) fn set(&mut self, value: f64) {
        self.value = value;
        self.text = format!("{value:.SET_PRECISION$}");
        self.pending = None;
    }

    /// Re-express the number in another prefix of its unit: the trace it is
    /// read against stepped a decade (Ω→kΩ). The text follows at
    /// [`flush`](Self::flush).
    pub(super) fn rescale(&mut self, ratio: PrefixRatio) {
        self.value = ratio.apply(self.value);
        self.pending = if holds_a_number(&self.text) {
            compose(self.pending, ratio)
        } else {
            None
        };
    }

    /// Bring the text to the value's prefix after a step, now that nobody is
    /// typing in it. A draft that doesn't read as a number is left as typed:
    /// it is read in the prefix it is shown against from now on.
    pub(super) fn flush(&mut self) {
        if let Some(ratio) = self.pending.take()
            && let Some(text) = rescaled_text(&self.text, ratio)
        {
            self.text = text;
        }
    }

    /// Put back a value a user typed and a file saved: shown as short as it
    /// reads, `60` rather than `60.0000`.
    pub(super) fn restore(&mut self, value: f64) {
        self.value = value;
        self.text = value.to_string();
        self.pending = None;
    }
}

/// What separates the numbers of a [`NumberListField`].
const SEPARATORS: [char; 3] = [',', ';', ' '];

/// One entry of a [`NumberListField`] read: its value in the plotted unit,
/// and the entry as the line's label writes it ("4.7 kΩ").
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    value: f64,
    label: String,
}

/// The words of `s`, as byte ranges: its runs of non-whitespace.
fn words(s: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut rest = 0;
    std::iter::from_fn(move || {
        let start = rest + s[rest..].find(|c: char| !c.is_whitespace())?;
        let end = s[start..]
            .find(char::is_whitespace)
            .map_or(s.len(), |e| start + e);
        rest = end;
        Some(start..end)
    })
}

/// Where the entries of `text` are, as byte ranges: split at commas,
/// semicolons and spaces, a word that doesn't start a number ("Ω", "kohm")
/// going with the number before it across a space, so "1 Ω, 4.7k 12" is
/// "1 Ω", "4.7k" and "12".
fn entry_spans(text: &str) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut chunk_start = 0;
    for chunk in text.split([',', ';']) {
        let mut in_chunk = false;
        for word in words(chunk) {
            let starts_a_number = chunk[word.clone()]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'));
            let word = chunk_start + word.start..chunk_start + word.end;
            match out.last_mut() {
                Some(last) if in_chunk && !starts_a_number => last.end = word.end,
                _ => out.push(word),
            }
            in_chunk = true;
        }
        // Both separators are one byte.
        chunk_start += chunk.len() + 1;
    }
    out
}

/// The entries of `text`, each as typed: see [`entry_spans`].
fn entries(text: &str) -> Vec<String> {
    entry_spans(text)
        .into_iter()
        .map(|r| text[r].to_string())
        .collect()
}

/// `v` and its `unit` as an entry writes them, as short as it reads.
fn entry_text(v: f64, unit: &str) -> String {
    format!("{} {unit}", short_figure(v)).trim_end().to_string()
}

/// Whether `entry` is a number with no unit after it.
fn is_bare(entry: &str) -> bool {
    number_and_unit(entry).is_some_and(|(_, _, unit)| unit.is_empty())
}

/// `entry` with `bare_unit` written after it when it is a bare number, the
/// unit it is in; as it is otherwise, or when that unit is "" (a unitless
/// reading's).
fn written_out(entry: &str, bare_unit: &str) -> String {
    if is_bare(entry) && !bare_unit.is_empty() {
        format!("{entry} {bare_unit}")
    } else {
        entry.to_string()
    }
}

/// `entry` read against the `plotted` unit, a bare number being in
/// `bare_unit`: `None` for a draft that is no number yet, or a unit of
/// another quantity.
fn read_entry(entry: &str, bare_unit: Option<&str>, plotted: &str) -> Option<Entry> {
    let (number, v, typed) = number_and_unit(entry)?;
    let unit = if typed.is_empty() {
        bare_unit?.to_string()
    } else {
        typed_unit(typed, plotted)
    };
    let ratio = prefix_ratio(&unit, plotted)?;
    Some(Entry {
        value: ratio.apply(v),
        label: format!("{number} {unit}").trim_end().to_string(),
    })
}

/// Why an entry of a [`NumberListField`] draws no line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Undrawn {
    /// It doesn't start with a number ("abc").
    NotANumber,
    /// What follows the number is no unit ("5 x").
    UnknownUnit,
    /// A unit of another quantity than the plot's ("5 V" on Ω), its base
    /// unit given ("" for a bare number in a unitless reading's): kept, and
    /// drawn once that quantity is plotted again.
    OtherQuantity(String),
}

/// An entry that draws no line, where it is in the text and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Mark {
    pub(super) span: Range<usize>,
    pub(super) why: Undrawn,
}

impl Mark {
    /// How the field marks it: one of another quantity waits for it, any
    /// other doesn't read, and may still be half typed.
    pub(super) fn text_mark(&self) -> TextMark {
        TextMark {
            span: self.span.clone(),
            style: match self.why {
                Undrawn::OtherQuantity(_) => MarkStyle::Waiting,
                Undrawn::NotANumber | Undrawn::UnknownUnit => MarkStyle::Unread,
            },
        }
    }
}

/// What a [`NumberListField`] reads its entries against, apart from its
/// text, to say which draw no line: see [`NumberListField::marker`].
pub(super) struct Marker {
    bare_unit: Option<String>,
    plotted: Option<String>,
}

impl Marker {
    /// The entries of `text` that draw no line. A unit is one of another
    /// quantity when a reading may carry it ([`meter_unit`]) or `known`
    /// names it (a meter's own, among the units the graph holds); any other
    /// word is no unit. Nothing is marked while
    /// nothing is plotted, nor a bare number still waiting for a unit.
    pub(super) fn marks(&self, text: &str, known: impl Fn(&str) -> bool) -> Vec<Mark> {
        let Some(plotted) = self.plotted.as_deref() else {
            return Vec::new();
        };
        entry_spans(text)
            .into_iter()
            .filter_map(|span| {
                let why = match number_and_unit(&text[span.clone()]) {
                    None => Undrawn::NotANumber,
                    Some((_, _, "")) => {
                        let unit = self.bare_unit.as_deref()?;
                        prefix_ratio(unit, plotted).is_none().then_some(())?;
                        Undrawn::OtherQuantity(base_unit(unit).to_string())
                    }
                    Some((_, _, typed)) => {
                        let unit = typed_unit(typed, plotted);
                        if prefix_ratio(&unit, plotted).is_some() {
                            return None;
                        }
                        if meter_unit(&unit).is_some() || known(&unit) {
                            Undrawn::OtherQuantity(base_unit(&unit).to_string())
                        } else {
                            Undrawn::UnknownUnit
                        }
                    }
                };
                Some(Mark { span, why })
            })
            .collect()
    }
}

/// A list of values the user types, separated by commas, semicolons or
/// spaces, each with a unit or bare: the reference lines. An entry keeps
/// the unit it was typed in through an auto-range step — 1 Ω stays 1 Ω
/// while the plot reads MΩ — and a bare number is in the unit plotted when
/// it was typed, written after it once the field is left. Bare numbers
/// share one unit until then: one typed after a step, before the field is
/// left, is in the draft's.
#[derive(Debug, Default)]
pub(super) struct NumberListField {
    text: String,
    /// What reads of the entries, in order.
    read: Vec<Entry>,
    /// The values of `read`, in the plotted unit.
    values: Vec<f64>,
    /// The unit a bare number in the text is in; `None` while none was
    /// plotted since it was typed, until a reading names one.
    bare_unit: Option<String>,
    /// The plotted unit the entries were last read against, `None` while
    /// nothing is plotted.
    plotted: Option<String>,
}

impl NumberListField {
    /// The draft, for a text edit to write into.
    pub(super) fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// The draft as typed.
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    /// The values the entries read, in the plotted unit.
    pub(super) fn values(&self) -> &[f64] {
        &self.values
    }

    /// The entry that reads `v`, as its line's label writes it.
    pub(super) fn label_of(&self, v: f64) -> Option<&str> {
        self.read
            .iter()
            .find(|e| e.value == v)
            .map(|e| e.label.as_str())
    }

    /// The entries as the lines' labels write them, beside [`values`](Self::values).
    pub(super) fn labels(&self) -> impl Iterator<Item = &str> {
        self.read.iter().map(|e| e.label.as_str())
    }

    /// Write `values`, in the `plotted` unit, with it after each.
    pub(super) fn set(&mut self, values: &[f64], plotted: &str) {
        self.text = values
            .iter()
            .map(|&v| entry_text(v, plotted))
            .collect::<Vec<_>>()
            .join(", ");
        self.bare_unit = Some(plotted.to_string());
        self.parse(Some(plotted));
    }

    /// Add `v`, in the `plotted` unit, at the end of the draft, keeping
    /// what is typed there as it is typed.
    pub(super) fn push(&mut self, v: f64, plotted: &str) {
        self.flush(Some(plotted));
        let kept = self.text.trim_end_matches(SEPARATORS);
        let added = entry_text(v, plotted);
        self.text = if kept.is_empty() {
            added
        } else {
            format!("{kept}, {added}")
        };
        self.parse(Some(plotted));
    }

    /// Take every entry that reads `v` out of the draft, keeping the others
    /// as they are typed, unread ones included.
    pub(super) fn remove(&mut self, v: f64, plotted: &str) {
        self.flush(Some(plotted));
        let (bare, kept) = (self.bare_unit.as_deref(), entries(&self.text));
        self.text = kept
            .iter()
            .filter(|e| read_entry(e, bare, plotted).map(|e| e.value) != Some(v))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        self.parse(Some(plotted));
    }

    /// Read the entries against `plotted` again if it moved: an auto-range
    /// step, another quantity plotted, or none (`None`). The text stays as
    /// typed.
    pub(super) fn follow(&mut self, plotted: Option<&str>) {
        if self.plotted.as_deref() != plotted {
            self.parse(plotted);
        }
    }

    /// Write the unit after each bare number, the unit it was typed in, now
    /// that nobody is typing in the field: from now on they are read as
    /// typed whatever is plotted.
    pub(super) fn flush(&mut self, plotted: Option<&str>) {
        let pieces = entries(&self.text);
        let any_bare = pieces.iter().any(|e| is_bare(e));
        if !any_bare && self.bare_unit.as_deref() == plotted && self.plotted.as_deref() == plotted {
            // Drawn every frame the field is shown: nothing to do.
            return;
        }
        // A unitless reading's bare numbers (an hFE) have no unit to write,
        // and stay in it: they draw on no other quantity. Typed with
        // nothing plotted, they wait for a reading to name theirs.
        let bare_unit = self.bare_unit.clone().unwrap_or_default();
        let written = !bare_unit.is_empty() && any_bare;
        if written {
            self.text = pieces
                .iter()
                .map(|e| written_out(e, &bare_unit))
                .collect::<Vec<_>>()
                .join(", ");
        }
        if written || !any_bare {
            self.bare_unit = plotted.map(str::to_string);
        }
        self.parse(plotted);
    }

    /// The entries as an export saves them: as typed, those that draw no
    /// line included, a bare number with the unit it is in after it.
    pub(super) fn saved_entries(&self) -> Vec<String> {
        let bare_unit = self.bare_unit.as_deref().unwrap_or_default();
        entries(&self.text)
            .iter()
            .map(|e| written_out(e, bare_unit))
            .collect()
    }

    /// Put back entries an export saved ([`saved_entries`](Self::saved_entries)),
    /// read against the `plotted` unit: a bare number left in them is in it.
    pub(super) fn restore_entries(&mut self, saved: &[String], plotted: &str) {
        self.text = saved.join(", ");
        self.bare_unit = Some(plotted.to_string());
        self.parse(Some(plotted));
    }

    /// What the entries are read against, for [`Marker::marks`] to say
    /// which draw no line in a text the field doesn't hold yet: the one
    /// being laid out under the caret.
    pub(super) fn marker(&self) -> Marker {
        Marker {
            bare_unit: self.bare_unit.clone(),
            plotted: self.plotted.clone(),
        }
    }

    /// Re-read the draft after an edit, against the `plotted` unit. Entries
    /// that don't read are dropped rather than held, so a trailing separator,
    /// a half-typed number or a unit of another quantity just doesn't draw a
    /// line.
    pub(super) fn parse(&mut self, plotted: Option<&str>) {
        let pieces = entries(&self.text);
        if self.bare_unit.is_none() || !pieces.iter().any(|e| is_bare(e)) {
            // No unit named yet, or no bare number left in an earlier one:
            // the next one typed is in what is plotted now.
            self.bare_unit = plotted.map(str::to_string);
        }
        self.plotted = plotted.map(str::to_string);
        self.read = match plotted {
            Some(plotted) => pieces
                .iter()
                .filter_map(|e| read_entry(e, self.bare_unit.as_deref(), plotted))
                .collect(),
            None => Vec::new(),
        };
        self.values = self.read.iter().map(|e| e.value).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entries of `f` that draw no line, as typed, with why.
    fn marked(f: &NumberListField, known: &[&str]) -> Vec<(String, Undrawn)> {
        let text = f.text();
        let marks = f.marker().marks(text, |u| known.contains(&u));
        marks
            .into_iter()
            .map(|m| (text[m.span].to_string(), m.why))
            .collect()
    }

    /// Each entry that draws no line is found where it was typed, extra
    /// spaces in it included, and says why: a unit of another quantity
    /// (one a reading may carry, in any case, or one the graph holds) from
    /// no unit, and from no number.
    #[test]
    fn an_entry_that_draws_no_line_says_why() {
        let other = |u: &str| Undrawn::OtherQuantity(u.to_string());
        let mut f = NumberListField::default();
        *f.text_mut() = "1 Ω, 4.7k;5  kV 3 x, abc 2 °C, 7 %, 9 mv, 4 Gizmo".into();
        f.parse(Some("kΩ"));
        assert_eq!(f.values(), [0.001, 4.7]);
        assert_eq!(
            marked(&f, &["Gizmo"]),
            [
                ("5  kV".to_string(), other("V")),
                ("3 x".to_string(), Undrawn::UnknownUnit),
                ("abc".to_string(), Undrawn::NotANumber),
                ("2 °C".to_string(), other("°C")),
                ("7 %".to_string(), other("%")),
                ("9 mv".to_string(), other("V")),
                ("4 Gizmo".to_string(), other("Gizmo")),
            ]
        );
        // Nothing is plotted: nothing can be judged.
        f.follow(None);
        assert_eq!(marked(&f, &["Gizmo"]), []);
    }

    /// An entry that may be half typed is held back while the caret is in
    /// it or right after it; one of another quantity is finished.
    #[test]
    fn the_entry_being_typed_is_held_back() {
        let mut f = NumberListField::default();
        *f.text_mut() = "1 o, 5 V".into();
        f.parse(Some("Ω"));
        let marks = f.marker().marks(f.text(), |_| false);
        let held = |caret| {
            marks
                .iter()
                .map(|m| m.text_mark().held_back(caret))
                .collect::<Vec<_>>()
        };
        assert_eq!(held(None), [false, false]);
        assert_eq!(held(Some(3)), [true, false]);
        assert_eq!(held(Some(0)), [true, false]);
        assert_eq!(held(Some(4)), [false, false]);
        assert_eq!(held(Some(8)), [false, false]);
    }

    /// A bare number typed in one quantity is marked once another is
    /// plotted before the field is left; on a unitless reading it says so.
    #[test]
    fn a_bare_number_of_another_quantity_is_marked() {
        let mut f = NumberListField::default();
        *f.text_mut() = "15".into();
        f.parse(Some("mV"));
        assert_eq!(marked(&f, &[]), []);
        f.follow(Some("Ω"));
        assert_eq!(
            marked(&f, &[]),
            [("15".to_string(), Undrawn::OtherQuantity("V".into()))]
        );
        let mut f = NumberListField::default();
        *f.text_mut() = "40".into();
        f.parse(Some(""));
        f.follow(Some("V"));
        assert_eq!(
            marked(&f, &[]),
            [("40".to_string(), Undrawn::OtherQuantity(String::new()))]
        );
    }

    /// A value added or removed leaves the rest of the draft as typed; a
    /// bare number gets the unit it was typed in once the field is left.
    #[test]
    fn a_list_keeps_its_draft_around_an_edit() {
        let mut f = NumberListField::default();
        f.push(3.29, "V");
        assert_eq!(f.text_mut().as_str(), "3.29 V");
        f.text_mut().push_str("; 5.00, x, ");
        f.parse(Some("V"));
        assert_eq!(f.values(), [3.29, 5.0]);
        f.push(12.0, "V");
        assert_eq!(f.text_mut().as_str(), "3.29 V, 5.00 V, x, 12 V");
        assert_eq!(f.values(), [3.29, 5.0, 12.0]);
        f.remove(5.0, "V");
        assert_eq!(f.text_mut().as_str(), "3.29 V, x, 12 V");
        assert_eq!(f.values(), [3.29, 12.0]);
    }

    /// An entry keeps the unit it was typed in, Latin spellings of ohms and
    /// a bare prefix included, whatever is plotted; one of another quantity
    /// draws nothing.
    #[test]
    fn an_entry_keeps_its_unit_through_a_step() {
        let mut f = NumberListField::default();
        *f.text_mut() = "1 ohm, 4.7k, 10 mOhms, 2 V".to_string();
        f.parse(Some("\u{3a9}"));
        assert_eq!(f.values(), [1.0, 4700.0, 0.01]);
        let labels: Vec<_> = f.labels().collect();
        assert_eq!(labels, ["1 \u{3a9}", "4.7 k\u{3a9}", "10 m\u{3a9}"]);
        f.follow(Some("M\u{3a9}"));
        assert_eq!(
            f.text_mut().as_str(),
            "1 ohm, 4.7k, 10 mOhms, 2 V",
            "as typed"
        );
        let close = |a: &[f64], b: &[f64]| {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(x, y)| (x - y).abs() <= 1e-15 * y.abs())
        };
        assert!(close(f.values(), &[1e-6, 0.0047, 1e-8]), "{:?}", f.values());
        assert_eq!(f.label_of(f.values()[0]), Some("1 \u{3a9}"));
    }

    /// The spellings a keyboard gives: spaces inside, the Greek mu and the
    /// ohm sign, a lowercase base, "u" for micro. Not an uppercase K for
    /// kilo, kelvin's symbol.
    #[test]
    fn units_are_read_as_keyboards_type_them() {
        let ohm = "\u{3a9}";
        assert_eq!(typed_unit("K", ohm), "K");
        assert_eq!(typed_unit("k \u{2126}", ohm), format!("k{ohm}"));
        assert_eq!(typed_unit("\u{3bc}F", "nF"), "\u{b5}F");
        assert_eq!(typed_unit("u", "nF"), "\u{b5}F");
        assert_eq!(typed_unit("mv", "V"), "mV");
        assert_eq!(typed_unit("v", "mV"), "V");
        assert_eq!(typed_unit("MHz", "Hz"), "MHz");
        let mut f = NumberListField::default();
        *f.text_mut() = "4.7 k, 2 k ohm, 1e3 \u{2126}, 3 K".to_string();
        f.parse(Some(ohm));
        assert_eq!(f.values(), [4700.0, 2000.0, 1000.0]);
    }

    /// Typed before any reading, a bare number takes the first reading's
    /// unit; typed on a unitless reading (an hFE), it stays unitless and
    /// draws on no other quantity.
    #[test]
    fn a_bare_number_waits_for_a_unit_or_stays_unitless() {
        let mut f = NumberListField::default();
        *f.text_mut() = "5".to_string();
        f.parse(None);
        assert!(f.values().is_empty(), "nothing plotted");
        f.follow(Some("V"));
        assert_eq!(f.values(), [5.0]);
        f.flush(Some("V"));
        assert_eq!(f.text_mut().as_str(), "5 V");

        let mut f = NumberListField::default();
        *f.text_mut() = "100".to_string();
        f.parse(Some(""));
        assert_eq!(f.values(), [100.0]);
        f.flush(Some(""));
        f.follow(Some("V"));
        f.flush(Some("V"));
        assert_eq!(f.text_mut().as_str(), "100");
        assert!(f.values().is_empty(), "an hFE is no voltage");
    }

    /// A bare number is in the unit plotted when it was typed, through a
    /// step while it is being typed, and is written with it once left.
    #[test]
    fn a_bare_number_is_in_the_unit_it_was_typed_in() {
        let mut f = NumberListField::default();
        *f.text_mut() = "15".to_string();
        f.parse(Some("mV"));
        f.follow(Some("V"));
        assert_eq!(f.values(), [0.015]);
        f.text_mut().push_str(", 2");
        f.parse(Some("V"));
        assert_eq!(f.values(), [0.015, 0.002], "still the draft's mV");
        f.flush(Some("V"));
        assert_eq!(f.text_mut().as_str(), "15 mV, 2 mV");
        f.text_mut().push_str(", 3");
        f.parse(Some("V"));
        assert_eq!(f.values(), [0.015, 0.002, 3.0], "typed after: in V");

        // A draft with no number yet takes what is plotted when it gets one.
        let mut f = NumberListField::default();
        *f.text_mut() = "-".to_string();
        f.parse(Some("mV"));
        f.follow(Some("V"));
        f.text_mut().push('5');
        f.parse(Some("V"));
        assert_eq!(f.values(), [-5.0]);
    }

    /// The value the graph draws from only moves when the draft is a number:
    /// clearing the field to retype it must not snap the axis to zero.
    #[test]
    fn an_unparseable_draft_keeps_the_last_good_value() {
        let mut f = NumberField::new("-1", -1.0);
        f.text_mut().clear();
        assert!(!f.parse());
        assert_eq!(f.value(), -1.0);
        f.text_mut().push_str("2.5");
        assert!(f.parse());
        assert_eq!(f.value(), 2.5);
    }

    /// A zero-width envelope window would bucket every sample together, so
    /// the field refuses it the same way it refuses letters.
    #[test]
    fn a_rejected_value_is_kept_out_the_same_way() {
        let mut f = NumberField::new("1", 1.0);
        *f.text_mut() = "0".to_string();
        assert!(!f.parse_if(|v| v > 0.0));
        assert_eq!(f.value(), 1.0);
        assert_eq!(f.text(), "0", "the draft stays as typed");
    }

    /// Writing a value in — a bbox zoom, or switching the Y axis to fixed —
    /// has to leave the text showing the number now in force.
    #[test]
    fn setting_a_value_rewrites_the_draft_to_match() {
        let mut f = NumberField::new("-1", -1.0);
        f.set(2.0);
        assert_eq!(f.value(), 2.0);
        assert_eq!(f.text(), "2.0000");
        assert!(f.parse(), "what it shows has to read back");
        assert_eq!(f.value(), 2.0);
    }

    /// A decade step re-expresses what the fields hold without float noise,
    /// and leaves a draft being typed as it is.
    #[test]
    fn a_rescale_re_expresses_the_numbers_and_keeps_the_draft() {
        use dmm_lib::transform::prefix_ratio;
        let to_mv = prefix_ratio("V", "mV").expect("one base unit");
        let to_kohm = prefix_ratio("\u{3a9}", "k\u{3a9}").expect("one base unit");

        let mut f = NumberField::new("-1", -1.0);
        f.set(219.87);
        f.rescale(to_kohm);
        f.flush();
        assert_eq!(f.text(), "0.21987");
        assert!((f.value() - 0.21987).abs() < 1e-15);
        f.text_mut().clear();
        f.rescale(to_mv);
        f.flush();
        assert_eq!(f.text(), "", "the draft stays as typed");
        assert!((f.value() - 219.87).abs() < 1e-12);
    }

    /// Until the text is flushed, it is read in the prefix it was typed in:
    /// a step, then another keystroke, and the value is still what the
    /// draft says. Two steps compose.
    #[test]
    fn a_draft_is_read_in_its_prefix_until_flushed() {
        use dmm_lib::transform::prefix_ratio;
        let to_v = prefix_ratio("mV", "V").expect("one base unit");
        let to_kv = prefix_ratio("V", "kV").expect("one base unit");
        let mut f = NumberField::new("15", 15.0);
        f.rescale(to_v);
        assert_eq!((f.text(), f.value()), ("15", 0.015));
        f.text_mut().push('0');
        assert!(f.parse());
        assert_eq!(f.value(), 0.15);
        f.rescale(to_kv);
        f.flush();
        assert_eq!(f.text(), "0.00015");
    }

    /// A draft with no number in it has no prefix: what is typed after a
    /// step is in the one the axis shows, and clearing a draft drops its old
    /// prefix too. Steps that cancel out leave the text as it was.
    #[test]
    fn a_draft_without_a_number_takes_the_new_prefix() {
        use dmm_lib::transform::prefix_ratio;
        let to_v = prefix_ratio("mV", "V").expect("one base unit");
        let to_mv = prefix_ratio("V", "mV").expect("one base unit");
        let mut f = NumberField::new("-", -1.0);
        f.rescale(to_v);
        f.text_mut().push('5');
        assert!(f.parse());
        assert_eq!(f.value(), -5.0, "typed against the axis in V");

        let mut f = NumberField::new("15", 15.0);
        f.rescale(to_v);
        f.text_mut().clear();
        assert!(!f.parse());
        f.text_mut().push('5');
        assert!(f.parse());
        assert_eq!(f.value(), 5.0, "cleared, then typed in V");

        let mut f = NumberField::new("219.8700", 219.87);
        f.rescale(to_v);
        f.rescale(to_mv);
        f.flush();
        assert_eq!(f.text(), "219.8700", "back where it was, untouched");
    }

    #[test]
    fn a_list_takes_any_of_the_three_separators() {
        let mut f = NumberListField::default();
        *f.text_mut() = "3.3, 5; 12 -1".to_string();
        f.parse(Some("V"));
        assert_eq!(f.values(), [3.3, 5.0, 12.0, -1.0]);
    }

    /// Typing "3.3, " must draw the line at 3.3 rather than nothing.
    #[test]
    fn a_half_typed_entry_does_not_drop_the_finished_ones() {
        let mut f = NumberListField::default();
        *f.text_mut() = "3.3, ".to_string();
        f.parse(Some("V"));
        assert_eq!(f.values(), [3.3]);
        *f.text_mut() = "3.3, -".to_string();
        f.parse(Some("V"));
        assert_eq!(f.values(), [3.3]);
    }
}
