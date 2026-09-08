//! Toolbar text-edit buffers paired with the values they parse to.
//!
//! Each pair used to be two `Graph` fields kept in step by convention: the
//! draft the user is typing, and the number the graph draws from. Nothing
//! stopped one being written without the other — the bbox zoom writes both,
//! and did so through four assignments — and the rule that an unparseable
//! draft leaves the last good value alone was restated at every edit site.

/// Digits the fields show when a value is written into them by the graph
/// rather than typed. Enough to keep a millivolt reading distinct, few
/// enough to stay editable.
const SET_PRECISION: usize = 4;

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
}

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

    /// Re-read the draft after an edit. Entries that don't parse are dropped
    /// rather than held, so a trailing separator or a half-typed number just
    /// doesn't draw a line yet.
    pub(super) fn parse(&mut self) {
        self.values = self
            .text
            .split([',', ';', ' '])
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
