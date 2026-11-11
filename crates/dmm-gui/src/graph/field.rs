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
use dmm_lib::transform::PrefixRatio;

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

/// A list of numbers the user types, separated by commas, semicolons or
/// spaces.
#[derive(Debug, Default)]
pub(super) struct NumberListField {
    text: String,
    values: Vec<f64>,
    /// The text is in an earlier prefix: this takes it to the values'.
    pending: Option<PrefixRatio>,
}

impl NumberListField {
    /// The draft, for a text edit to write into.
    pub(super) fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    pub(super) fn values(&self) -> &[f64] {
        &self.values
    }

    /// Write `values` into both halves, as the graph shows them.
    pub(super) fn set(&mut self, values: &[f64]) {
        self.values = values.to_vec();
        self.pending = None;
        self.text = values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
    }

    /// Add `v` at the end of the draft, keeping what is typed there as it
    /// is typed.
    pub(super) fn push(&mut self, v: f64) {
        self.flush();
        let kept = self.text.trim_end_matches(SEPARATORS);
        self.text = if kept.is_empty() {
            v.to_string()
        } else {
            format!("{kept}, {v}")
        };
        self.parse();
    }

    /// Take every entry that reads `v` out of the draft, keeping the others
    /// as they are typed, unparsed ones included.
    pub(super) fn remove(&mut self, v: f64) {
        self.flush();
        self.text = self
            .text
            .split(SEPARATORS)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.parse::<f64>().ok() != Some(v))
            .collect::<Vec<_>>()
            .join(", ");
        self.parse();
    }

    /// Re-express every number in another prefix of its unit. The text
    /// follows at [`flush`](Self::flush).
    pub(super) fn rescale(&mut self, ratio: PrefixRatio) {
        self.pending = if holds_a_number(&self.text) {
            compose(self.pending, ratio)
        } else {
            None
        };
        // From the text, as the flushed text will read, so that the menu
        // finds each line by the number it shows.
        self.parse();
    }

    /// Bring the text to the values' prefix after a step, now that nobody
    /// is typing in it, keeping the entries that don't read as a number as
    /// they are typed, as [`push`](Self::push) and [`remove`](Self::remove)
    /// do.
    pub(super) fn flush(&mut self) {
        let Some(ratio) = self.pending.take() else {
            return;
        };
        self.text = self
            .text
            .split(SEPARATORS)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| rescaled_text(s, ratio).unwrap_or_else(|| s.to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        self.parse();
    }

    /// Re-read the draft after an edit. Entries that don't parse are dropped
    /// rather than held, so a trailing separator or a half-typed number just
    /// doesn't draw a line yet. A draft typed before a step is read in its
    /// prefix, each number as the flushed text will show it.
    pub(super) fn parse(&mut self) {
        if !holds_a_number(&self.text) {
            self.pending = None;
        }
        let pending = self.pending;
        self.values = self
            .text
            .split(SEPARATORS)
            .filter_map(|s| match pending {
                Some(ratio) => rescaled_text(s, ratio)?.parse().ok(),
                None => s.trim().parse::<f64>().ok(),
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value added or removed leaves the rest of the draft as typed.
    #[test]
    fn a_list_keeps_its_draft_around_an_edit() {
        let mut f = NumberListField::default();
        f.push(3.29);
        assert_eq!(f.text_mut().as_str(), "3.29");
        f.text_mut().push_str("; 5.00 x, ");
        f.parse();
        f.push(12.0);
        assert_eq!(f.text_mut().as_str(), "3.29; 5.00 x, 12");
        assert_eq!(f.values(), [3.29, 5.0, 12.0]);
        f.remove(5.0);
        assert_eq!(f.text_mut().as_str(), "3.29, x, 12");
        assert_eq!(f.values(), [3.29, 12.0]);
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

        let mut f = NumberListField::default();
        *f.text_mut() = "3.29; 0.1 x, -".to_string();
        f.parse();
        f.rescale(to_mv);
        assert_eq!(f.values(), [3290.0, 100.0]);
        f.flush();
        assert_eq!(f.text_mut().as_str(), "3290, 100, x, -");
        assert_eq!(f.values(), [3290.0, 100.0]);
        f.rescale(to_kohm);
        f.flush();
        assert_eq!(f.text_mut().as_str(), "3.29, 0.1, x, -");
        assert_eq!(f.values(), [3.29, 0.1], "back to the numbers typed");

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

        let mut f = NumberListField::default();
        *f.text_mut() = "3.29, 12".to_string();
        f.parse();
        f.rescale(to_v);
        f.text_mut().push_str(", 5");
        f.parse();
        assert_eq!(f.values(), [0.00329, 0.012, 0.005]);
        // A line added from the menu, in the values' prefix, flushes first.
        f.push(0.02);
        assert_eq!(f.text_mut().as_str(), "0.00329, 0.012, 0.005, 0.02");
        f.rescale(to_kv);
        f.remove(0.000012);
        assert_eq!(f.text_mut().as_str(), "0.00000329, 0.000005, 0.00002");
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

        let mut f = NumberListField::default();
        *f.text_mut() = "-".to_string();
        f.parse();
        f.rescale(to_v);
        f.text_mut().push('5');
        f.parse();
        assert_eq!(f.values(), [-5.0]);
    }

    #[test]
    fn a_list_takes_any_of_the_three_separators() {
        let mut f = NumberListField::default();
        *f.text_mut() = "3.3, 5; 12 -1".to_string();
        f.parse();
        assert_eq!(f.values(), [3.3, 5.0, 12.0, -1.0]);
    }

    /// Typing "3.3, " must draw the line at 3.3 rather than nothing.
    #[test]
    fn a_half_typed_entry_does_not_drop_the_finished_ones() {
        let mut f = NumberListField::default();
        *f.text_mut() = "3.3, ".to_string();
        f.parse();
        assert_eq!(f.values(), [3.3]);
        *f.text_mut() = "3.3, -".to_string();
        f.parse();
        assert_eq!(f.values(), [3.3]);
    }
}
