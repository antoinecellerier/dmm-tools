//! Toolbar text-edit buffers paired with the values they parse to.
//!
//! Each pair used to be two `Graph` fields kept in step by convention: the
//! draft the user is typing, and the number the graph draws from. Nothing
//! stopped one being written without the other — the bbox zoom writes both,
//! and did so through four assignments — and the rule that an unparseable
//! draft leaves the last good value alone was restated at every edit site.

use dmm_lib::transform::PrefixRatio;

/// Digits the fields show when a value is written into them by the graph
/// rather than typed. Enough to keep a millivolt reading distinct, few
/// enough to stay editable.
const SET_PRECISION: usize = 4;

/// Significant digits a field keeps of a number it re-expresses in another
/// prefix: all a typed one has, none of the float error of the step
/// (`3.29 × 1000` is `3290.0000000000005`).
const RESCALED_DIGITS: i32 = 12;

/// `text` re-expressed by `ratio` when it reads as a number, at most
/// [`RESCALED_DIGITS`] significant digits and no trailing zeros; `None` when
/// it doesn't, for a draft the user is still typing to be left as it is.
fn rescaled_text(text: &str, ratio: PrefixRatio) -> Option<String> {
    let v = ratio.apply(text.trim().parse::<f64>().ok()?);
    if v == 0.0 || !v.is_finite() {
        return Some(if v == 0.0 { "0".into() } else { v.to_string() });
    }
    let decimals = (RESCALED_DIGITS - 1 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.decimals$}");
    Some(if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    })
}

/// A number the user types, beside the value it last parsed to.
#[derive(Debug)]
pub(super) struct NumberField {
    text: String,
    value: f64,
}

impl NumberField {
    /// A field showing `text`, which the caller has to make a rendering of
    /// `value`.
    pub(super) fn new(text: &str, value: f64) -> Self {
        Self {
            text: text.to_string(),
            value,
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
        match self.text.parse::<f64>() {
            Ok(v) if accept(v) => {
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
    }

    /// Re-express the number in another prefix of its unit: the trace it is
    /// read against stepped a decade (Ω→kΩ). A draft that doesn't read as a
    /// number is left as typed, and takes the new prefix once it does.
    pub(super) fn rescale(&mut self, ratio: PrefixRatio) {
        self.value = ratio.apply(self.value);
        if let Some(text) = rescaled_text(&self.text, ratio) {
            self.text = text;
        }
    }

    /// Put back a value a user typed and a file saved: shown as short as it
    /// reads, `60` rather than `60.0000`.
    pub(super) fn restore(&mut self, value: f64) {
        self.value = value;
        self.text = value.to_string();
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
        self.text = values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
    }

    /// Add `v` at the end of the draft, keeping what is typed there as it
    /// is typed.
    pub(super) fn push(&mut self, v: f64) {
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
        self.text = self
            .text
            .split(SEPARATORS)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.parse::<f64>().ok() != Some(v))
            .collect::<Vec<_>>()
            .join(", ");
        self.parse();
    }

    /// Re-express every number in the draft in another prefix of its unit,
    /// keeping the entries that don't read as one as they are typed, as
    /// [`push`](Self::push) and [`remove`](Self::remove) do.
    pub(super) fn rescale(&mut self, ratio: PrefixRatio) {
        self.text = self
            .text
            .split(SEPARATORS)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| rescaled_text(s, ratio).unwrap_or_else(|| s.to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        // From the text, so that the menu finds each line by the number it
        // shows.
        self.parse();
    }

    /// Re-read the draft after an edit. Entries that don't parse are dropped
    /// rather than held, so a trailing separator or a half-typed number just
    /// doesn't draw a line yet.
    pub(super) fn parse(&mut self) {
        self.values = self
            .text
            .split(SEPARATORS)
            .filter_map(|s| s.trim().parse::<f64>().ok())
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
        assert_eq!(f.text_mut().as_str(), "3290, 100, x, -");
        assert_eq!(f.values(), [3290.0, 100.0]);
        f.rescale(to_kohm);
        assert_eq!(f.text_mut().as_str(), "3.29, 0.1, x, -");
        assert_eq!(f.values(), [3.29, 0.1], "back to the numbers typed");

        let mut f = NumberField::new("-1", -1.0);
        f.set(219.87);
        f.rescale(to_kohm);
        assert_eq!(f.text(), "0.21987");
        assert!((f.value() - 0.21987).abs() < 1e-15);
        f.text_mut().clear();
        f.rescale(to_mv);
        assert_eq!(f.text(), "", "the draft stays as typed");
        assert!((f.value() - 219.87).abs() < 1e-12);
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
