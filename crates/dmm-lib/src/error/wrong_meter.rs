//! Why the meter on the link cannot be read as the entry it was opened as:
//! the reason [`Error::WrongMeter`] carries.
//!
//! Each reason is one sentence, with its variable parts as fields, so an app
//! can word it in the user's language; `Display` is the library's English.
//! Model names and registry ids stay as the registry spells them. The
//! remedies name Auto-detect and `--device`, as both binaries take them.
//!
//! Deliberately not `#[non_exhaustive]`: an app's exhaustive `match` stops
//! compiling when a reason is added, rather than falling back to English.
//!
//! [`Error::WrongMeter`]: super::Error::WrongMeter

use thiserror::Error;

/// Why the meter on the link cannot be read as the entry it was opened as.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WrongMeter {
    /// An OWON meter reported model `code`, whose entry `id` (`model`)
    /// sends OWON's frame of `frame` bytes (6 or 15), not the kind of
    /// `opened`, the entry it was opened as.
    #[error(
        "the meter reports OWON model code {code} ({model}), which sends OWON's {frame}-byte \
         frame, not the {opened} it was opened as; choose Auto-detect or --device {id}"
    )]
    OtherFrame {
        code: u8,
        model: &'static str,
        frame: u8,
        opened: &'static str,
        id: &'static str,
    },

    /// An OWON meter sends frames of `sent` bytes, not the `expected` bytes
    /// of `opened`'s, and named no model code; `remedy` is what to choose.
    #[error(
        "the meter sends OWON's {sent}-byte frames, not the {expected}-byte frames of the \
         {opened} it was opened as; {remedy}"
    )]
    OtherFrameKind {
        sent: u8,
        expected: u8,
        opened: &'static str,
        remedy: FrameRemedy,
    },

    /// An OWON meter reported model `code`, which OWON's programs read but
    /// this library does not, for `why`.
    #[error("the meter reports OWON model code {code}{why}")]
    UnreadModel { code: u8, why: Unread },

    /// An OWON 6-byte entry hears bytes but never its frame: the older B35
    /// and B35T send a 14-byte ASCII frame.
    #[error(
        "the meter sends no OWON 6-byte frames; it may be an older B35 or B35T, whose 14-byte \
         frames this tool does not read yet"
    )]
    NoSixByteFrames,

    /// An OWON 15-byte entry hears bytes but never its frame.
    #[error("the meter sends no OWON 15-byte frames; choose Auto-detect")]
    NoFifteenByteFrames,

    /// No model code was read and the frames tile as neither OWON kind.
    #[error(
        "neither the meter's model code nor its frames show whether it sends OWON's 6- or \
         15-byte frames; please open an issue with this message"
    )]
    UntoldFrameKind,

    /// A Brymen meter of another series, `found`, answered on the cable
    /// opened as `opened`; `id` is the entry to choose.
    #[error(
        "the meter on the cable is a {found}, not a {opened}; choose Auto-detect or --device {id}"
    )]
    OtherSeries {
        found: &'static str,
        opened: &'static str,
        id: &'static str,
    },
}

/// What to choose for a meter that sends the other kind of OWON frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FrameRemedy {
    /// Detection opens a 6-byte stream's fallback entry.
    #[error("choose Auto-detect")]
    AutoDetect,
    /// Nothing in the frame names the model, so the user has to.
    #[error("choose the meter's own model")]
    OwnModel,
}

/// Why this library does not read an OWON model it recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Unread {
    /// A model nobody has asked for yet: an issue would.
    #[error(", which this tool does not read yet; please open an issue with this message")]
    NotYet,
    /// A 6-byte frame that takes its sign from the function word.
    #[error(
        ": it sends OWON's 6-byte frame with the sign in the function word, which this tool \
         does not read yet"
    )]
    SignInFunctionWord,
}
