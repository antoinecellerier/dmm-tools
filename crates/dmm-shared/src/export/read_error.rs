//! Why an export or a recording's header did not read: the reasons
//! [`read_csv_from`](super::read_csv_from),
//! [`read_json_from`](super::read_json_from) and
//! [`recorded_at`](crate::replay::recorded_at) give.
//!
//! Each reason is one sentence, with its variable parts as fields, so an app
//! words it in the user's language ([`Localizer::read_error`]); `Display` is
//! the English the logs and `dmm-gui`'s startup errors keep. What the system
//! or a parser (csv, serde, chrono) said passes through as it is, in English.
//!
//! Deliberately not `#[non_exhaustive]`: the renderer's exhaustive `match`
//! stops compiling when a reason is added, rather than falling back to
//! English.

use super::escape_controls;
use crate::l10n::Localizer;
use crate::tr;
use std::borrow::Cow;

/// Why a file did not read, at the line it stopped at where there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    /// The line of the file, counting from 1.
    pub line: Option<usize>,
    pub reason: ReadReason,
}

/// What was wrong with the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadReason {
    /// The system could not read it: its words.
    Io(String),
    /// A JSON export without a single line.
    Empty,
    /// A CSV export without its header row: the CSV reader's words.
    NoHeaderRow(String),
    /// A CSV export without one of the columns every export has.
    NoColumn(&'static str),
    /// A JSON export whose first line is not its `_metadata` line: the JSON
    /// parser's words.
    NoMetadata(String),
    /// A cell or field that should hold a reading's value.
    NotAValue(String),
    /// An `NCV:` cell whose level is no number.
    NotAnNcvLevel(String),
    /// The words of a flags cell that name no flag.
    NotAFlag(String),
    /// A marker cell that is no marker number.
    NotAMarkerNumber(String),
    /// A timestamp, or a replay's `# recorded:` line, that is no RFC 3339
    /// date: the text, then the date parser's words.
    NotADate { text: String, detail: String },
    /// A row or a line its parser refused: the parser's words.
    Parse(String),
}

impl ReadError {
    /// `reason`, at no line.
    pub(crate) fn whole(reason: ReadReason) -> Self {
        Self { line: None, reason }
    }

    /// `reason`, at `line`.
    pub(crate) fn at(line: usize, reason: ReadReason) -> Self {
        Self {
            line: Some(line),
            reason,
        }
    }
}

impl std::fmt::Display for ReadReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(detail) | Self::Parse(detail) => f.write_str(detail),
            Self::Empty => f.write_str("the file is empty"),
            Self::NoHeaderRow(detail) => write!(f, "no header row ({detail})"),
            Self::NoColumn(name) => write!(f, "no `{name}` column"),
            Self::NoMetadata(detail) => write!(f, "no `_metadata` line ({detail})"),
            Self::NotAValue(text) => write!(f, "`{text}` is not a value"),
            Self::NotAnNcvLevel(text) => write!(f, "`{text}` is not an NCV level"),
            Self::NotAFlag(text) => write!(f, "`{text}` is not a flag"),
            Self::NotAMarkerNumber(text) => write!(f, "`{text}` is not a marker number"),
            Self::NotADate { text, detail } => {
                write!(f, "`{text}` is not an RFC 3339 date: {detail}")
            }
        }
    }
}

/// The reason, at its line where there is one, with its control characters
/// escaped: it quotes the file, and is printed.
impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = self.reason.to_string();
        let reason = escape_controls(&reason);
        match self.line {
            Some(line) => write!(f, "line {line}: {reason}"),
            None => f.write_str(&reason),
        }
    }
}

impl std::error::Error for ReadError {}

impl Localizer {
    /// `e` in this language: what follows the file's name in `dmm-gui`'s
    /// import toast and `dmm-cli`'s `Error:` line.
    pub fn read_error(&self, e: &ReadError) -> String {
        // Text from the file is escaped as `Display` escapes it.
        let reason = self.read_reason(&e.reason, |t| escape_controls(t).into_owned());
        match e.line {
            Some(line) => {
                tr!(self, "read-error-at-line", line = line, reason = reason).into_owned()
            }
            None => reason,
        }
    }

    /// `reason` in this language, each part from the file through `quote`.
    fn read_reason(&self, reason: &ReadReason, quote: impl Fn(&str) -> String) -> String {
        let text: Cow<'_, str> = match reason {
            ReadReason::Io(detail) | ReadReason::Parse(detail) => return quote(detail),
            ReadReason::Empty => tr!(self, "read-error-empty"),
            ReadReason::NoHeaderRow(detail) => {
                tr!(self, "read-error-no-header-row", detail = quote(detail))
            }
            ReadReason::NoColumn(name) => tr!(self, "read-error-no-column", column = *name),
            ReadReason::NoMetadata(detail) => {
                tr!(self, "read-error-no-metadata", detail = quote(detail))
            }
            ReadReason::NotAValue(text) => tr!(self, "read-error-not-a-value", text = quote(text)),
            ReadReason::NotAnNcvLevel(text) => {
                tr!(self, "read-error-not-an-ncv-level", text = quote(text))
            }
            ReadReason::NotAFlag(text) => tr!(self, "read-error-not-a-flag", text = quote(text)),
            ReadReason::NotAMarkerNumber(text) => {
                tr!(self, "read-error-not-a-marker-number", text = quote(text))
            }
            ReadReason::NotADate { text, detail } => tr!(
                self,
                "read-error-not-a-date",
                text = quote(text),
                detail = quote(detail)
            ),
        };
        text.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason after `last` in a chain that lists every reason once:
    /// one exhaustive `match`, so a reason added fails to compile here
    /// until it has a sample.
    fn after(last: Option<&ReadReason>) -> Option<ReadReason> {
        use ReadReason as R;
        let s = |t: &str| t.to_string();
        Some(match last {
            None => R::Io(s("Permission denied (os error 13)")),
            Some(R::Io(_)) => R::Empty,
            Some(R::Empty) => R::NoHeaderRow(s("stream did not contain valid UTF-8")),
            Some(R::NoHeaderRow(_)) => R::NoColumn("timestamp"),
            Some(R::NoColumn(_)) => R::NoMetadata(s("expected value at line 1 column 1")),
            Some(R::NoMetadata(_)) => R::NotAValue(s("1.2.3")),
            Some(R::NotAValue(_)) => R::NotAnNcvLevel(s("NCV:x")),
            Some(R::NotAnNcvLevel(_)) => R::NotAFlag(s("HOLD\u{7}")),
            Some(R::NotAFlag(_)) => R::NotAMarkerNumber(s("-1")),
            Some(R::NotAMarkerNumber(_)) => R::NotADate {
                text: s("# recorded: yesterday"),
                detail: s("input contains invalid characters"),
            },
            Some(R::NotADate { .. }) => R::Parse(s("found record with 3 fields, but \u{1b}[31m")),
            Some(R::Parse(_)) => return None,
        })
    }

    /// English is `Display`, byte for byte, for every reason, at a line and
    /// at none, control characters escaped either way.
    #[test]
    fn english_reads_as_display() {
        let l = Localizer::en_us(crate::help::RESOURCES);
        let mut reason = after(None);
        let mut seen = 0;
        while let Some(r) = reason {
            for e in [ReadError::whole(r.clone()), ReadError::at(3, r.clone())] {
                assert_eq!(l.read_error(&e), e.to_string(), "{e:?}");
            }
            reason = after(Some(&r));
            seen += 1;
        }
        assert_eq!(seen, 11);
        let escaped = ReadError::at(2, ReadReason::NotAFlag("A\u{7}".into()));
        assert_eq!(escaped.to_string(), "line 2: `A\\u{7}` is not a flag");
        let escaped = ReadError::whole(ReadReason::NotAFlag("A\u{7}".into()));
        assert_eq!(escaped.to_string(), "`A\\u{7}` is not a flag");
    }
}
