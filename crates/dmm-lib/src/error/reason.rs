//! Why a command was not carried out: the reasons [`Error::UnsupportedCommand`]
//! and [`Error::CommandRejected`] carry.
//!
//! Each reason is one sentence, with its variable parts as fields, so an app
//! can word it in the user's language; `Display` is the library's English.
//! Mode, range and state labels are the library's English, as
//! `Measurement::mode` and the choice lists carry them: each field says the
//! [`TextKind`] an app looks it up as. Button names stay as the meter prints
//! them, and command words and setting names as the CLI takes them.
//!
//! Deliberately not `#[non_exhaustive]`: an app's exhaustive `match` stops
//! compiling when a reason is added, rather than falling back to English.
//!
//! [`Error::UnsupportedCommand`]: super::Error::UnsupportedCommand
//! [`Error::CommandRejected`]: super::Error::CommandRejected

use crate::protocol::Setting;
use crate::text::TextKind;
use crate::transport::Link;
use std::borrow::Cow;
use thiserror::Error;

/// Why a command was refused before it reached the meter: the meter, the
/// link or the dial position cannot do it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unsupported {
    /// A command word the family does not take.
    #[error("{0}")]
    Command(String),

    /// A setting the family cannot drive at all.
    #[error("{0} cannot be set on this meter")]
    Setting(Setting),

    /// A choice id the setting does not list.
    #[error("{setting} {id:#06x}")]
    NoSuchChoice { setting: Setting, id: u16 },

    /// The mode is on more than one dial position, so which one the dial is
    /// on is unknown. `mode` is a [`TextKind::Mode`].
    #[error(
        "{mode} is reported from more than one dial position — take a reading in another mode \
         first"
    )]
    SeveralPositions { mode: Cow<'static, str> },

    /// The mode is on no dial position of the family's table. `mode` is a
    /// [`TextKind::Mode`].
    #[error("{mode} is on no known dial position")]
    NoPosition { mode: Cow<'static, str> },

    /// `target` is on another dial position than `from`. Both are
    /// [`TextKind::Mode`].
    #[error("mode {target} is not reachable from {from} — turn the dial first")]
    ModeUnreachable {
        target: Cow<'static, str>,
        from: Cow<'static, str>,
    },

    /// As [`Unsupported::ModeUnreachable`], from a meter whose modes are
    /// words (UT181A): `target` and `from_word` are mode words, `from` is
    /// `from_word`'s [`TextKind::Mode`].
    #[error(
        "mode {target:#06x} is not reachable from {from} ({from_word:#06x}) — turn the dial first"
    )]
    WordUnreachable {
        target: u16,
        from: Cow<'static, str>,
        from_word: u16,
    },

    /// The mode has no range ladder. `mode` is a [`TextKind::Mode`].
    #[error("{mode} has no range to choose on this meter")]
    NoRanges { mode: Cow<'static, str> },

    /// Rung `id` is past the `count` rungs of the mode's ladder. `mode` is
    /// a [`TextKind::Mode`].
    #[error("range {id} is not one of the {count} {mode} offers")]
    NoSuchRange {
        id: u16,
        count: usize,
        mode: Cow<'static, str>,
    },

    /// As [`Unsupported::NoSuchRange`], from a meter that takes ranges
    /// absolutely (UT181A). `mode` is a [`TextKind::Mode`].
    #[error("range {id} is not one of the {count} ranges {mode} offers")]
    RangeNotListed {
        id: u16,
        count: usize,
        mode: Cow<'static, str>,
    },

    /// Auto range is not among the mode's ranges (UT181A in Peak). `mode` is
    /// a [`TextKind::Mode`].
    #[error("auto range is not one of the {count} ranges {mode} offers")]
    AutoRangeNotListed {
        count: usize,
        mode: Cow<'static, str>,
    },

    /// The mode has no range ladder (UT181A). `mode` is `word`'s
    /// [`TextKind::Mode`].
    #[error("range in {mode} ({word:#06x}): fixed-range mode")]
    FixedRange { mode: Cow<'static, str>, word: u16 },

    /// The setting is not offered in this mode. `mode` is a
    /// [`TextKind::Mode`].
    #[error("{setting} cannot be set in {mode} on this meter")]
    SettingNotInMode {
        setting: Setting,
        mode: Cow<'static, str>,
    },

    /// As [`Unsupported::SettingNotInMode`], from a meter whose modes are
    /// words (UT181A). `mode` is `word`'s [`TextKind::Mode`].
    #[error("{setting} cannot be set in {mode} ({word:#06x}) on this meter")]
    SettingNotInWord {
        setting: Setting,
        mode: Cow<'static, str>,
        word: u16,
    },

    /// State `id` is not one the setting offers in this mode. `mode` is a
    /// [`TextKind::Mode`], each of `offered` a [`TextKind::Toggle`].
    #[error("{setting} has no state {id}; {mode} offers {}", .offered.join(", "))]
    NoSuchState {
        setting: Setting,
        id: u16,
        mode: Cow<'static, str>,
        offered: Vec<Cow<'static, str>>,
    },

    /// The meter refuses `command` in a Peak mode (UT181A). `mode` is
    /// `word`'s [`TextKind::Mode`].
    #[error("{command} in {mode} ({word:#06x}): the meter refuses it in Peak")]
    RefusedInPeak {
        command: String,
        mode: Cow<'static, str>,
        word: u16,
    },

    /// The mode has no relative reading (UT181A). `mode` is `word`'s
    /// [`TextKind::Mode`].
    #[error("rel in {mode} ({word:#06x})")]
    NoRel { mode: Cow<'static, str>, word: u16 },

    /// The meter has no such button. `model` is its part number.
    #[error("the {model} has no {button} button")]
    NoButton {
        model: &'static str,
        button: &'static str,
    },

    /// A request of another length than the cable carries: a meter of
    /// another family on this cable. `cable` is the cable's name.
    #[error("the {cable} cable takes {len}-byte requests, not {got} bytes")]
    RequestLength {
        cable: &'static str,
        len: usize,
        got: usize,
    },

    /// The link cannot change the meter's serial rate; `None` for a link
    /// that does not say what it is.
    #[error(
        "the {} cannot change its rate to {baud} baud",
        .link.map_or("link", |link| link.full_name(false))
    )]
    Baud { link: Option<Link>, baud: u32 },

    /// `Protocol::parse_payload` on a simulated meter, which has no bytes.
    #[error("parse_payload: the mock has no wire format")]
    NoWireFormat,
}

/// Why a command the meter was sent did not take, or was refused unsent
/// because the meter's state says it cannot: typically the dial is not where
/// the command needs it. Retrying the same command won't help.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Rejected {
    /// A press of `button` left `state` as it was. `state` is a `kind`
    /// label: the mode, range or setting state being walked.
    #[error("{button} did nothing in {state}")]
    DidNothing {
        button: &'static str,
        state: Cow<'static, str>,
        kind: TextKind,
    },

    /// The walk came back around to `now` without `target` showing. Both are
    /// `kind` labels, as for [`Rejected::DidNothing`].
    #[error("{target} never appeared; the meter is back in {now}")]
    NeverAppeared {
        target: Cow<'static, str>,
        now: Cow<'static, str>,
        kind: TextKind,
    },

    /// `presses` presses of `button` did not reach the target. `state` is a
    /// `kind` label, as for [`Rejected::DidNothing`].
    #[error("gave up after {presses} presses of {button}; the meter is in {state}")]
    GaveUp {
        presses: usize,
        button: &'static str,
        state: Cow<'static, str>,
        kind: TextKind,
    },

    /// A range walk saw the mode change under it, and stopped pressing
    /// RANGE. `mode` is a [`TextKind::Mode`].
    #[error(
        "the mode changed to {mode}; stopped pressing {}",
        crate::protocol::cycle::RANGE_BUTTON_NAME
    )]
    ModeChanged { mode: Cow<'static, str> },

    /// `reason` ended a walk after an earlier one had moved the meter out of
    /// `mode`, a [`TextKind::Mode`].
    #[error("{reason}; started in {mode}")]
    StartedIn {
        reason: Box<Rejected>,
        mode: Cow<'static, str>,
    },

    /// The auto-range command left the meter on `range`, a
    /// [`TextKind::Range`].
    #[error("AUTO did nothing; the meter is still in {range}")]
    AutoDidNothing { range: Cow<'static, str> },

    /// The command that leaves a setting left it in `state`, a
    /// [`TextKind::Toggle`]. `button` is the setting's button.
    #[error("EXIT {button} did nothing; the meter is still in {state}")]
    ExitDidNothing {
        button: &'static str,
        state: Cow<'static, str>,
    },

    /// Switching `setting` to `target` left it in `now`; both are
    /// [`TextKind::Toggle`] (UT181A).
    #[error("{setting} {target} did nothing; the meter is still in {now}")]
    SwitchDidNothing {
        setting: Setting,
        target: Cow<'static, str>,
        now: Cow<'static, str>,
    },

    /// The meter answered the command with a refusal of its own (UT181A).
    #[error("{0} rejected by the meter")]
    ByMeter(Sent),

    /// ZERO is taken only in capacitance (ZOTEK).
    #[error("ZERO works in capacitance only")]
    ZeroNeedsCapacitance,

    /// REL takes the reading on the display as its reference, and the
    /// display shows no number (UT171).
    #[error("REL needs a number on the display")]
    RelNeedsReading,

    /// REL is on, and the display shows the difference, not the reading a
    /// new reference would take (UT171).
    #[error("REL is already on; leave REL first")]
    RelIsOn,

    /// The meter reports another model than the one it was opened as, so its
    /// keys stay off (OWON). `model` and `opened` are the entries' display
    /// names, `id` the reported entry's registry id.
    #[error(
        "the meter reports OWON model code {code} ({model}), not the {opened} it was opened as, \
         so remote keys are off; choose Auto-detect or --device {id}"
    )]
    OtherModel {
        code: u8,
        model: &'static str,
        opened: &'static str,
        id: &'static str,
    },

    /// The meter reports a model code no entry carries, so its keys stay off
    /// (OWON).
    #[error(
        "the meter reports OWON model code {code}, which this tool does not know yet, so remote \
         keys are off"
    )]
    UnknownModel { code: u8 },

    /// The model code was not read, and `command` waits for it (OWON).
    #[error(
        "the meter's model code was not read, so {command} is off: hold REL on the meter to leave REL"
    )]
    ModelUnread { command: &'static str },

    /// MIN/MAX recording holds the range (the mock).
    #[error("MIN/MAX locks the range; leave MIN/MAX before changing it")]
    MinMaxLocksRange,

    /// A replay was sent a command, or asked to switch a setting: what is
    /// the command word or the setting name.
    #[error("{0}: a replay takes no commands")]
    Replay(String),
}

/// What a command sent the meter, as [`Rejected::ByMeter`] names it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Sent {
    /// A command word, as the CLI takes it.
    #[error("{0}")]
    Command(String),
    /// A switch to a mode, a [`TextKind::Mode`].
    #[error("mode {0}")]
    Mode(Cow<'static, str>),
    /// A switch to a range, a [`TextKind::Range`].
    #[error("range {0}")]
    Range(Cow<'static, str>),
    /// A switch of a setting to a state, a [`TextKind::Toggle`].
    #[error("{0} {1}")]
    State(Setting, Cow<'static, str>),
}

impl From<Unsupported> for super::Error {
    fn from(reason: Unsupported) -> Self {
        super::Error::UnsupportedCommand(reason)
    }
}

impl From<Rejected> for super::Error {
    fn from(reason: Rejected) -> Self {
        super::Error::CommandRejected(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &'static str) -> Cow<'static, str> {
        Cow::Borrowed(text)
    }

    /// Each reason reads as the sentence the library printed before reasons
    /// were enums, word for word: logs and library users see `Display`.
    #[test]
    fn each_reason_reads_as_its_sentence() {
        let unsupported = [
            (Unsupported::Command("light".into()), "light"),
            (
                Unsupported::Setting(Setting::Peak),
                "peak cannot be set on this meter",
            ),
            (
                Unsupported::NoSuchChoice {
                    setting: Setting::Mode,
                    id: 0x12,
                },
                "mode 0x0012",
            ),
            (
                Unsupported::SeveralPositions { mode: s("DC V") },
                "DC V is reported from more than one dial position — take a reading in another \
                 mode first",
            ),
            (
                Unsupported::NoPosition { mode: s("DC V") },
                "DC V is on no known dial position",
            ),
            (
                Unsupported::ModeUnreachable {
                    target: s("Ω"),
                    from: s("DC V"),
                },
                "mode Ω is not reachable from DC V — turn the dial first",
            ),
            (
                Unsupported::WordUnreachable {
                    target: 0x4111,
                    from: s("VDC"),
                    from_word: 0x3111,
                },
                "mode 0x4111 is not reachable from VDC (0x3111) — turn the dial first",
            ),
            (
                Unsupported::NoRanges { mode: s("Diode") },
                "Diode has no range to choose on this meter",
            ),
            (
                Unsupported::NoSuchRange {
                    id: 9,
                    count: 5,
                    mode: s("DC V"),
                },
                "range 9 is not one of the 5 DC V offers",
            ),
            (
                Unsupported::RangeNotListed {
                    id: 9,
                    count: 4,
                    mode: s("VDC"),
                },
                "range 9 is not one of the 4 ranges VDC offers",
            ),
            (
                Unsupported::AutoRangeNotListed {
                    count: 2,
                    mode: s("VDC Peak"),
                },
                "auto range is not one of the 2 ranges VDC Peak offers",
            ),
            (
                Unsupported::FixedRange {
                    mode: s("Diode"),
                    word: 0x5111,
                },
                "range in Diode (0x5111): fixed-range mode",
            ),
            (
                Unsupported::SettingNotInMode {
                    setting: Setting::Rel,
                    mode: s("Diode"),
                },
                "rel cannot be set in Diode on this meter",
            ),
            (
                Unsupported::SettingNotInWord {
                    setting: Setting::Rel,
                    mode: s("Diode"),
                    word: 0x5111,
                },
                "rel cannot be set in Diode (0x5111) on this meter",
            ),
            (
                Unsupported::NoSuchState {
                    setting: Setting::MinMax,
                    id: 7,
                    mode: s("DC V"),
                    offered: vec![s("off"), s("MAX"), s("MIN")],
                },
                "minmax has no state 7; DC V offers off, MAX, MIN",
            ),
            (
                Unsupported::RefusedInPeak {
                    command: "auto".into(),
                    mode: s("VDC Peak"),
                    word: 0x3113,
                },
                "auto in VDC Peak (0x3113): the meter refuses it in Peak",
            ),
            (
                Unsupported::NoRel {
                    mode: s("Diode"),
                    word: 0x5111,
                },
                "rel in Diode (0x5111)",
            ),
            (
                Unsupported::NoButton {
                    model: "VC-880",
                    button: "Hz/%",
                },
                "the VC-880 has no Hz/% button",
            ),
            (
                Unsupported::RequestLength {
                    cable: "BU-86X",
                    len: 3,
                    got: 6,
                },
                "the BU-86X cable takes 3-byte requests, not 6 bytes",
            ),
            (
                Unsupported::Baud {
                    link: Some(Link::UsbCable),
                    baud: 19200,
                },
                "the USB cable cannot change its rate to 19200 baud",
            ),
            (
                Unsupported::Baud {
                    link: None,
                    baud: 19200,
                },
                "the link cannot change its rate to 19200 baud",
            ),
            (
                Unsupported::NoWireFormat,
                "parse_payload: the mock has no wire format",
            ),
        ];
        let did_nothing = Rejected::DidNothing {
            button: "SELECT",
            state: s("AC V"),
            kind: TextKind::Mode,
        };
        let rejected = [
            (did_nothing.clone(), "SELECT did nothing in AC V"),
            (
                Rejected::NeverAppeared {
                    target: s("Hz"),
                    now: s("AC V"),
                    kind: TextKind::Mode,
                },
                "Hz never appeared; the meter is back in AC V",
            ),
            (
                Rejected::GaveUp {
                    presses: 4,
                    button: "RANGE",
                    state: s("22V"),
                    kind: TextKind::Range,
                },
                "gave up after 4 presses of RANGE; the meter is in 22V",
            ),
            (
                Rejected::ModeChanged { mode: s("AC+DC V") },
                "the mode changed to AC+DC V; stopped pressing RANGE",
            ),
            (
                Rejected::StartedIn {
                    reason: Box::new(did_nothing),
                    mode: s("DC V"),
                },
                "SELECT did nothing in AC V; started in DC V",
            ),
            (
                Rejected::AutoDidNothing { range: s("22V") },
                "AUTO did nothing; the meter is still in 22V",
            ),
            (
                Rejected::ExitDidNothing {
                    button: "MAX/MIN",
                    state: s("MAX"),
                },
                "EXIT MAX/MIN did nothing; the meter is still in MAX",
            ),
            (
                Rejected::SwitchDidNothing {
                    setting: Setting::Rel,
                    target: s("on"),
                    now: s("off"),
                },
                "rel on did nothing; the meter is still in off",
            ),
            (
                Rejected::ByMeter(Sent::Command("auto".into())),
                "auto rejected by the meter",
            ),
            (
                Rejected::ByMeter(Sent::Mode(s("VDC"))),
                "mode VDC rejected by the meter",
            ),
            (
                Rejected::ByMeter(Sent::Range(s("60V"))),
                "range 60V rejected by the meter",
            ),
            (
                Rejected::ByMeter(Sent::State(Setting::MinMax, s("on"))),
                "minmax on rejected by the meter",
            ),
            (
                Rejected::ZeroNeedsCapacitance,
                "ZERO works in capacitance only",
            ),
            (
                Rejected::RelNeedsReading,
                "REL needs a number on the display",
            ),
            (Rejected::RelIsOn, "REL is already on; leave REL first"),
            (
                Rejected::OtherModel {
                    code: 41,
                    model: "OWON B41T+",
                    opened: "OWON B35T+",
                    id: "b41t+",
                },
                "the meter reports OWON model code 41 (OWON B41T+), not the OWON B35T+ it was \
                 opened as, so remote keys are off; choose Auto-detect or --device b41t+",
            ),
            (
                Rejected::UnknownModel { code: 223 },
                "the meter reports OWON model code 223, which this tool does not know yet, so \
                 remote keys are off",
            ),
            (
                Rejected::ModelUnread {
                    command: "exit_rel",
                },
                "the meter's model code was not read, so exit_rel is off: hold REL on the meter \
                 to leave REL",
            ),
            (
                Rejected::MinMaxLocksRange,
                "MIN/MAX locks the range; leave MIN/MAX before changing it",
            ),
            (
                Rejected::Replay("hold".into()),
                "hold: a replay takes no commands",
            ),
        ];
        for (reason, sentence) in unsupported {
            assert_eq!(reason.to_string(), sentence, "{reason:?}");
            assert_eq!(
                super::super::Error::from(reason).to_string(),
                format!("unsupported command: {sentence}")
            );
        }
        for (reason, sentence) in rejected {
            assert_eq!(reason.to_string(), sentence, "{reason:?}");
            assert_eq!(
                super::super::Error::from(reason).to_string(),
                format!("command rejected: {sentence}")
            );
        }
    }
}
