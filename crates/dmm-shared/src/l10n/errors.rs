//! The library's errors in the user's language, by variant: an exhaustive
//! `match` over [`Error`] and each reason it carries, so a variant added in
//! dmm-lib fails to compile here until it has a message in
//! `i18n/<locale>/lib-errors.ftl`. A test checks each en-US rendering
//! against the library's `Display`, which logs keep using.
//!
//! What the library did not word passes through as it is: the operating
//! system's and the Bluetooth stack's text, file paths, values the user
//! typed, hexadecimal codes. So do the developer diagnostics whose detail is
//! built from open-ended parts: [`Error::InvalidResponse`]'s message and
//! [`Error::Replay`]'s, which name a line of a file and quote serde, chrono
//! or io errors. Their lead-in translates; the detail stays English, and
//! the replay one, quoting the file, has its control characters escaped. Mode,
//! range and state names go through [`Localizer::label`] by the kind each
//! field names.

use super::Localizer;
use crate::export::escape_controls;
use crate::tr;
use dmm_lib::error::{
    BluetoothIssue, BluetoothOnlyMiss, CharacteristicRole, Error, FrameRemedy, Rejected, Sent,
    Unread, Unsupported, WrongMeter,
};
use dmm_lib::text::TextKind;
use dmm_lib::transport::{LateReadings, Link};
use std::borrow::Cow;

/// A hexadecimal code as the library prints it: `0x3111`.
fn word(code: u16) -> String {
    format!("{code:#06x}")
}

impl Localizer {
    /// `e` in this language: the text a GUI notice or toast and the CLI's
    /// `Error:` line show.
    pub fn error(&self, e: &Error) -> String {
        match e {
            Error::Hid(hid) => tr!(self, "error-hid", detail = hid.to_string()),
            Error::InvalidResponse { message, .. } => {
                tr!(self, "error-invalid-response", detail = message)
            }
            Error::WrongMeter { reason, .. } => {
                tr!(self, "error-wrong-meter", reason = self.wrong_meter(reason))
            }
            Error::ChecksumMismatch { expected, actual } => tr!(
                self,
                "error-checksum-mismatch",
                expected = word(*expected),
                actual = word(*actual)
            ),
            Error::Timeout => tr!(self, "error-timeout"),
            Error::UnknownMode(code) => {
                tr!(self, "error-unknown-mode", code = format!("{code:#04x}"))
            }
            Error::UnsupportedCommand(reason) => tr!(
                self,
                "error-unsupported-command",
                reason = self.unsupported(reason)
            ),
            Error::CommandRejected(reason) => tr!(
                self,
                "error-command-rejected",
                reason = self.rejected(reason)
            ),
            Error::UnknownDevice(device) => tr!(self, "error-unknown-device", device = device),
            Error::NotSimulated(device) => tr!(self, "error-not-simulated", device = *device),
            // It quotes the file, so its control characters are escaped.
            Error::Replay(detail) => tr!(
                self,
                "error-replay",
                detail = escape_controls(detail).into_owned()
            ),
            Error::AdapterNotFound(adapter) => {
                tr!(self, "error-adapter-not-found", adapter = adapter)
            }
            Error::NotAnAdapter(path) => tr!(
                self,
                "error-adapter-not-found",
                adapter = tr!(self, "error-not-an-adapter", path = path)
            ),
            Error::WrongCable { cable, model } => tr!(
                self,
                "error-wrong-cable",
                link = if *cable == dmm_lib::BLUETOOTH {
                    "bluetooth"
                } else {
                    "cable"
                },
                cable = *cable,
                model = *model
            ),
            Error::LinkLost => tr!(self, "error-link-lost"),
            Error::Bluetooth(issue) => {
                tr!(self, "error-bluetooth", reason = self.bluetooth(issue))
            }
            Error::DeviceNotIdentified {
                bridge,
                built_in_radio,
            } => {
                let link = if *bridge == dmm_lib::BLUETOOTH {
                    Link::Bluetooth
                } else {
                    Link::UsbCable
                };
                tr!(
                    self,
                    "error-device-not-identified",
                    link = link_key(Some(link), *built_in_radio)
                )
            }
            Error::NoTransportFound {
                cables,
                bluetooth_searched,
            } => {
                let bluetooth = if *bluetooth_searched {
                    "searched"
                } else {
                    "not-searched"
                };
                let ids = dmm_lib::error::cable_ids(cables);
                if ids.is_empty() {
                    tr!(self, "error-no-transport-found", bluetooth = bluetooth)
                } else {
                    tr!(
                        self,
                        "error-no-transport-found-looked-for",
                        bluetooth = bluetooth,
                        cables = ids
                    )
                }
            }
            Error::BluetoothOnly { model, miss, .. } => {
                let model = *model;
                match miss {
                    BluetoothOnlyMiss::NotInRange => {
                        tr!(self, "error-bluetooth-only-not-in-range", model = model)
                    }
                    BluetoothOnlyMiss::SwitchedOff => {
                        tr!(self, "error-bluetooth-only-switched-off", model = model)
                    }
                    BluetoothOnlyMiss::NotBuilt => {
                        tr!(self, "error-bluetooth-only-not-built", model = model)
                    }
                    BluetoothOnlyMiss::UsbAdapter => {
                        tr!(self, "error-bluetooth-only-usb-adapter", model = model)
                    }
                }
            }
        }
        .into_owned()
    }

    /// What `--adapter` named, as an adapter-not-found error and the
    /// help beside it quote it: the selector, or a USB device's path with
    /// why it is no cable. `None` for any other error.
    pub fn adapter_named<'a>(&'a self, e: &'a Error) -> Option<Cow<'a, str>> {
        match e {
            Error::AdapterNotFound(selector) => Some(Cow::Borrowed(selector.as_str())),
            Error::NotAnAdapter(path) => Some(tr!(self, "error-not-an-adapter", path = path)),
            _ => None,
        }
    }

    /// Why the meter on the link is not one the meter chosen can read, as
    /// [`Error::WrongMeter`] carries it. Model names and ids stay as the
    /// registry spells them.
    fn wrong_meter<'a>(&'a self, reason: &'a WrongMeter) -> Cow<'a, str> {
        match reason {
            WrongMeter::OtherFrame {
                code,
                model,
                frame,
                opened,
                id,
            } => tr!(
                self,
                "error-wrong-meter-other-frame",
                code = *code,
                model = *model,
                frame = *frame,
                opened = *opened,
                id = *id
            ),
            WrongMeter::OtherFrameKind {
                sent,
                expected,
                opened,
                remedy,
            } => tr!(
                self,
                "error-wrong-meter-other-frame-kind",
                sent = *sent,
                expected = *expected,
                opened = *opened,
                remedy = match remedy {
                    FrameRemedy::AutoDetect => "auto-detect",
                    FrameRemedy::OwnModel => "own-model",
                }
            ),
            WrongMeter::UnreadModel { code, why } => match why {
                Unread::NotYet => tr!(self, "error-wrong-meter-not-yet", code = *code),
                Unread::SignInFunctionWord => {
                    tr!(
                        self,
                        "error-wrong-meter-sign-in-function-word",
                        code = *code
                    )
                }
            },
            WrongMeter::OtherSeries { found, opened, id } => tr!(
                self,
                "error-wrong-meter-other-series",
                found = *found,
                opened = *opened,
                id = *id
            ),
            WrongMeter::NoSixByteFrames => tr!(self, "error-wrong-meter-no-six-byte-frames"),
            WrongMeter::NoFifteenByteFrames => {
                tr!(self, "error-wrong-meter-no-fifteen-byte-frames")
            }
            WrongMeter::UntoldFrameKind => tr!(self, "error-wrong-meter-untold-frame-kind"),
        }
    }

    /// Why a command was refused before it was sent, as
    /// [`Error::UnsupportedCommand`] carries it.
    fn unsupported<'a>(&'a self, reason: &'a Unsupported) -> Cow<'a, str> {
        let mode = |m: &'a str| self.label(TextKind::Mode, m);
        match reason {
            // A command word or a setting and a code: nothing to word.
            Unsupported::Command(command) => Cow::Borrowed(command.as_str()),
            Unsupported::NoSuchChoice { .. } => Cow::Owned(reason.to_string()),
            Unsupported::Setting(setting) => {
                tr!(self, "error-unsupported-setting", setting = setting.name())
            }
            Unsupported::SeveralPositions { mode: m } => {
                tr!(self, "error-unsupported-several-positions", mode = mode(m))
            }
            Unsupported::NoPosition { mode: m } => {
                tr!(self, "error-unsupported-no-position", mode = mode(m))
            }
            Unsupported::ModeUnreachable { target, from } => tr!(
                self,
                "error-unsupported-mode-unreachable",
                target = mode(target),
                from = mode(from)
            ),
            Unsupported::WordUnreachable {
                target,
                from,
                from_word,
            } => tr!(
                self,
                "error-unsupported-word-unreachable",
                target = word(*target),
                from = mode(from),
                from_word = word(*from_word)
            ),
            Unsupported::NoRanges { mode: m } => {
                tr!(self, "error-unsupported-no-ranges", mode = mode(m))
            }
            Unsupported::NoSuchRange { id, count, mode: m } => tr!(
                self,
                "error-unsupported-no-such-range",
                id = *id,
                count = *count,
                mode = mode(m)
            ),
            Unsupported::RangeNotListed { id, count, mode: m } => tr!(
                self,
                "error-unsupported-range-not-listed",
                id = *id,
                count = *count,
                mode = mode(m)
            ),
            Unsupported::AutoRangeNotListed { count, mode: m } => tr!(
                self,
                "error-unsupported-auto-range-not-listed",
                count = *count,
                mode = mode(m)
            ),
            Unsupported::FixedRange { mode: m, word: w } => tr!(
                self,
                "error-unsupported-fixed-range",
                mode = mode(m),
                word = word(*w)
            ),
            Unsupported::SettingNotInMode { setting, mode: m } => tr!(
                self,
                "error-unsupported-setting-not-in-mode",
                setting = setting.name(),
                mode = mode(m)
            ),
            Unsupported::SettingNotInWord {
                setting,
                mode: m,
                word: w,
            } => tr!(
                self,
                "error-unsupported-setting-not-in-word",
                setting = setting.name(),
                mode = mode(m),
                word = word(*w)
            ),
            Unsupported::NoSuchState {
                setting,
                id,
                mode: m,
                offered,
            } => {
                let offered: Vec<Cow<'_, str>> = offered
                    .iter()
                    .map(|s| self.label(TextKind::Toggle, s))
                    .collect();
                tr!(
                    self,
                    "error-unsupported-no-such-state",
                    setting = setting.name(),
                    id = *id,
                    mode = mode(m),
                    offered = offered.join(", ")
                )
            }
            Unsupported::RefusedInPeak {
                command,
                mode: m,
                word: w,
            } => tr!(
                self,
                "error-unsupported-refused-in-peak",
                command = command,
                mode = mode(m),
                word = word(*w)
            ),
            Unsupported::NoRel { mode: m, word: w } => tr!(
                self,
                "error-unsupported-no-rel",
                mode = mode(m),
                word = word(*w)
            ),
            Unsupported::NoButton { model, button } => tr!(
                self,
                "error-unsupported-no-button",
                model = *model,
                button = *button
            ),
            Unsupported::RequestLength { cable, len, got } => tr!(
                self,
                "error-unsupported-request-length",
                cable = *cable,
                len = *len,
                got = *got
            ),
            // The library names the link without the meter, so a Bluetooth
            // link is the adapter.
            Unsupported::Baud { link, baud } => tr!(
                self,
                "error-unsupported-baud",
                link = link_key(*link, false),
                baud = *baud
            ),
            Unsupported::NoWireFormat => tr!(self, "error-unsupported-no-wire-format"),
        }
    }

    /// Why the meter did not do a command, as [`Error::CommandRejected`]
    /// carries it; what the CLI's `set` repeats after its own lead-in.
    pub fn rejected<'a>(&'a self, reason: &'a Rejected) -> Cow<'a, str> {
        match reason {
            Rejected::DidNothing {
                button,
                state,
                kind,
            } => tr!(
                self,
                "error-rejected-did-nothing",
                button = *button,
                state = self.label(*kind, state)
            ),
            Rejected::NeverAppeared { target, now, kind } => tr!(
                self,
                "error-rejected-never-appeared",
                target = self.label(*kind, target),
                now = self.label(*kind, now)
            ),
            Rejected::GaveUp {
                presses,
                button,
                state,
                kind,
            } => tr!(
                self,
                "error-rejected-gave-up",
                presses = *presses,
                button = *button,
                state = self.label(*kind, state)
            ),
            Rejected::ModeChanged { mode } => tr!(
                self,
                "error-rejected-mode-changed",
                mode = self.label(TextKind::Mode, mode)
            ),
            Rejected::StartedIn { reason, mode } => tr!(
                self,
                "error-rejected-started-in",
                reason = self.rejected(reason),
                mode = self.label(TextKind::Mode, mode)
            ),
            Rejected::AutoDidNothing { range } => tr!(
                self,
                "error-rejected-auto-did-nothing",
                range = self.label(TextKind::Range, range)
            ),
            Rejected::ExitDidNothing { button, state } => tr!(
                self,
                "error-rejected-exit-did-nothing",
                button = *button,
                state = self.label(TextKind::Toggle, state)
            ),
            Rejected::SwitchDidNothing {
                setting,
                target,
                now,
            } => tr!(
                self,
                "error-rejected-switch-did-nothing",
                setting = setting.name(),
                target = self.label(TextKind::Toggle, target),
                now = self.label(TextKind::Toggle, now)
            ),
            Rejected::ByMeter(sent) => {
                tr!(self, "error-rejected-by-meter", sent = self.sent(sent))
            }
            Rejected::ZeroNeedsCapacitance => tr!(self, "error-rejected-zero-needs-capacitance"),
            Rejected::OtherModel {
                code,
                model,
                opened,
                id,
            } => tr!(
                self,
                "error-rejected-other-model",
                code = *code,
                model = *model,
                opened = *opened,
                id = *id
            ),
            Rejected::UnknownModel { code } => {
                tr!(self, "error-rejected-unknown-model", code = *code)
            }
            Rejected::ModelUnread { command } => {
                tr!(self, "error-rejected-model-unread", command = *command)
            }
            Rejected::MinMaxLocksRange => tr!(self, "error-rejected-min-max-locks-range"),
            Rejected::Replay(command) => tr!(self, "error-rejected-replay", command = command),
        }
    }

    /// What a refused command sent, as [`Rejected::ByMeter`] names it.
    fn sent<'a>(&'a self, sent: &'a Sent) -> Cow<'a, str> {
        match sent {
            Sent::Command(command) => Cow::Borrowed(command.as_str()),
            Sent::Mode(mode) => tr!(
                self,
                "error-sent-mode",
                mode = self.label(TextKind::Mode, mode)
            ),
            Sent::Range(range) => tr!(
                self,
                "error-sent-range",
                range = self.label(TextKind::Range, range)
            ),
            // A setting's name and a state: nothing to word.
            Sent::State(setting, state) => {
                Cow::Owned(format!("{setting} {}", self.label(TextKind::Toggle, state)))
            }
        }
    }

    /// What the Bluetooth stack could not do, as [`Error::Bluetooth`]
    /// carries it.
    fn bluetooth<'a>(&'a self, issue: &'a BluetoothIssue) -> Cow<'a, str> {
        match issue {
            // The stack's own words, in its own language.
            BluetoothIssue::Stack(text) => Cow::Borrowed(text.as_str()),
            BluetoothIssue::Runtime { detail } => {
                tr!(self, "error-bluetooth-runtime", detail = detail)
            }
            BluetoothIssue::NoAdapter => tr!(self, "error-bluetooth-no-adapter"),
            BluetoothIssue::PoweredOff => tr!(self, "error-bluetooth-powered-off"),
            BluetoothIssue::NoCharacteristic { role } => tr!(
                self,
                "error-bluetooth-no-characteristic",
                role = match role {
                    CharacteristicRole::Write => "write",
                    CharacteristicRole::Notify => "notify",
                }
            ),
            BluetoothIssue::PasswordRefused => tr!(self, "error-bluetooth-password-refused"),
            BluetoothIssue::LoginRefused { code } => {
                tr!(self, "error-bluetooth-login-refused", code = *code)
            }
        }
    }
}

impl Localizer {
    /// The title of the notice for a link that hands over readings two at
    /// a time ([`LateReadings::TITLE`]).
    pub fn late_readings_title(&self) -> Cow<'_, str> {
        tr!(self, "late-readings-title")
    }

    /// What happens and whether there is a command to run, as
    /// [`LateReadings::advice`] words it; the command itself is not text.
    pub fn late_readings_advice(&self, notice: &LateReadings) -> Cow<'_, str> {
        let fix = if notice.command.is_some() {
            "command"
        } else {
            "none"
        };
        tr!(self, "late-readings-advice", fix = fix)
    }
}

/// The key the messages select a link's name by: `usb`, `adapter` (a
/// Bluetooth adapter on the meter), `radio` (the meter's own Bluetooth) or
/// `none`, as [`Link::full_name`] names them.
fn link_key(link: Option<Link>, built_in_radio: bool) -> &'static str {
    match link {
        Some(Link::UsbCable) => "usb",
        Some(Link::Bluetooth) if built_in_radio => "radio",
        Some(Link::Bluetooth) => "adapter",
        None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::LIB_RESOURCES;
    use dmm_lib::protocol::Setting;

    fn s(text: &'static str) -> Cow<'static, str> {
        Cow::Borrowed(text)
    }

    /// Every sample `next` chains: `next(None)` gives the first variant's,
    /// `next(Some(last))` those of the variant after `last`'s, and an empty
    /// list ends the chain. Each `next` is one exhaustive `match` over its
    /// enum, so a variant added in dmm-lib fails to compile there until the
    /// chain gives it samples.
    fn chain<E: std::fmt::Debug>(next: impl Fn(Option<&E>) -> Vec<E>) -> Vec<E> {
        let mut all: Vec<E> = Vec::new();
        let mut seen = Vec::new();
        loop {
            let more = next(all.last());
            let Some(first) = more.first() else {
                return all;
            };
            let variant = std::mem::discriminant(first);
            assert!(!seen.contains(&variant), "{first:?} comes round again");
            assert!(
                more.iter().all(|e| std::mem::discriminant(e) == variant),
                "{more:?} mixes variants"
            );
            seen.push(variant);
            all.extend(more);
        }
    }

    fn unsupported() -> Vec<Unsupported> {
        use Unsupported as U;
        chain(|last: Option<&U>| match last {
            None => vec![U::Command("light".into())],
            Some(U::Command(_)) => vec![U::Setting(Setting::Peak)],
            Some(U::Setting(_)) => vec![U::NoSuchChoice {
                setting: Setting::Mode,
                id: 0x12,
            }],
            Some(U::NoSuchChoice { .. }) => vec![U::SeveralPositions { mode: s("DC V") }],
            Some(U::SeveralPositions { .. }) => vec![U::NoPosition { mode: s("DC V") }],
            Some(U::NoPosition { .. }) => vec![U::ModeUnreachable {
                target: s("Ω"),
                from: s("DC V"),
            }],
            Some(U::ModeUnreachable { .. }) => vec![U::WordUnreachable {
                target: 0x4111,
                from: s("VDC"),
                from_word: 0x3111,
            }],
            Some(U::WordUnreachable { .. }) => vec![U::NoRanges { mode: s("Diode") }],
            Some(U::NoRanges { .. }) => vec![U::NoSuchRange {
                id: 9,
                count: 5,
                mode: s("DC V"),
            }],
            Some(U::NoSuchRange { .. }) => vec![U::RangeNotListed {
                id: 9,
                count: 1,
                mode: s("VDC"),
            }],
            Some(U::RangeNotListed { .. }) => vec![U::AutoRangeNotListed {
                count: 2,
                mode: s("VDC Peak"),
            }],
            Some(U::AutoRangeNotListed { .. }) => vec![U::FixedRange {
                mode: s("Diode"),
                word: 0x5111,
            }],
            Some(U::FixedRange { .. }) => vec![U::SettingNotInMode {
                setting: Setting::Rel,
                mode: s("Diode"),
            }],
            Some(U::SettingNotInMode { .. }) => vec![U::SettingNotInWord {
                setting: Setting::Rel,
                mode: s("Diode"),
                word: 0x5111,
            }],
            Some(U::SettingNotInWord { .. }) => vec![U::NoSuchState {
                setting: Setting::MinMax,
                id: 7,
                mode: s("DC V"),
                offered: vec![s("off"), s("MAX"), s("MIN")],
            }],
            Some(U::NoSuchState { .. }) => vec![U::RefusedInPeak {
                command: "auto".into(),
                mode: s("VDC Peak"),
                word: 0x3113,
            }],
            Some(U::RefusedInPeak { .. }) => vec![U::NoRel {
                mode: s("Diode"),
                word: 0x5111,
            }],
            Some(U::NoRel { .. }) => vec![U::NoButton {
                model: "VC-880",
                button: "Hz/%",
            }],
            Some(U::NoButton { .. }) => vec![U::RequestLength {
                cable: "BU-86X",
                len: 3,
                got: 6,
            }],
            Some(U::RequestLength { .. }) => [Some(Link::UsbCable), Some(Link::Bluetooth), None]
                .into_iter()
                .map(|link| U::Baud { link, baud: 19200 })
                .collect(),
            Some(U::Baud { .. }) => vec![U::NoWireFormat],
            Some(U::NoWireFormat) => vec![],
        })
    }

    fn sent() -> Vec<Sent> {
        chain(|last: Option<&Sent>| match last {
            None => vec![Sent::Command("auto".into())],
            Some(Sent::Command(_)) => vec![Sent::Mode(s("VDC"))],
            Some(Sent::Mode(_)) => vec![Sent::Range(s("60V"))],
            Some(Sent::Range(_)) => vec![Sent::State(Setting::MinMax, s("on"))],
            Some(Sent::State(..)) => vec![],
        })
    }

    fn rejected() -> Vec<Rejected> {
        use Rejected as R;
        let did_nothing = || R::DidNothing {
            button: "SELECT",
            state: s("AC V"),
            kind: TextKind::Mode,
        };
        chain(|last: Option<&R>| match last {
            None => vec![did_nothing()],
            Some(R::DidNothing { .. }) => vec![R::NeverAppeared {
                target: s("Hz"),
                now: s("AC V"),
                kind: TextKind::Mode,
            }],
            Some(R::NeverAppeared { .. }) => vec![R::GaveUp {
                presses: 4,
                button: "RANGE",
                state: s("22V"),
                kind: TextKind::Range,
            }],
            Some(R::GaveUp { .. }) => vec![R::ModeChanged { mode: s("AC+DC V") }],
            Some(R::ModeChanged { .. }) => vec![R::StartedIn {
                reason: Box::new(did_nothing()),
                mode: s("DC V"),
            }],
            Some(R::StartedIn { .. }) => vec![R::AutoDidNothing { range: s("22V") }],
            Some(R::AutoDidNothing { .. }) => vec![R::ExitDidNothing {
                button: "MAX/MIN",
                state: s("MAX"),
            }],
            Some(R::ExitDidNothing { .. }) => vec![R::SwitchDidNothing {
                setting: Setting::Rel,
                target: s("on"),
                now: s("off"),
            }],
            Some(R::SwitchDidNothing { .. }) => sent().into_iter().map(R::ByMeter).collect(),
            Some(R::ByMeter(_)) => vec![R::ZeroNeedsCapacitance],
            Some(R::ZeroNeedsCapacitance) => vec![R::OtherModel {
                code: 41,
                model: "OWON B41T+",
                opened: "OWON B35T+",
                id: "b41t+",
            }],
            Some(R::OtherModel { .. }) => vec![R::UnknownModel { code: 223 }],
            Some(R::UnknownModel { .. }) => vec![R::ModelUnread {
                command: "exit_rel",
            }],
            Some(R::ModelUnread { .. }) => vec![R::MinMaxLocksRange],
            Some(R::MinMaxLocksRange) => vec![R::Replay("hold".into())],
            Some(R::Replay(_)) => vec![],
        })
    }

    fn bluetooth() -> Vec<BluetoothIssue> {
        use BluetoothIssue as B;
        chain(|last: Option<&B>| match last {
            None => vec![B::Stack("Timed out after 10s".into())],
            Some(B::Stack(_)) => vec![B::Runtime {
                detail: "Too many open files (os error 24)".into(),
            }],
            Some(B::Runtime { .. }) => vec![B::NoAdapter],
            Some(B::NoAdapter) => vec![B::PoweredOff],
            Some(B::PoweredOff) => chain(|last: Option<&CharacteristicRole>| match last {
                None => vec![CharacteristicRole::Write],
                Some(CharacteristicRole::Write) => vec![CharacteristicRole::Notify],
                Some(CharacteristicRole::Notify) => vec![],
            })
            .into_iter()
            .map(|role| B::NoCharacteristic { role })
            .collect(),
            Some(B::NoCharacteristic { .. }) => vec![B::PasswordRefused],
            Some(B::PasswordRefused) => (0..8)
                .chain([261])
                .map(|code| B::LoginRefused { code })
                .collect(),
            Some(B::LoginRefused { .. }) => vec![],
        })
    }

    fn wrong_meter() -> Vec<WrongMeter> {
        use WrongMeter as W;
        chain(|last: Option<&W>| match last {
            None => vec![W::OtherFrame {
                code: 87,
                model: "Voltcraft VC871",
                frame: 15,
                opened: "OWON B35T+",
                id: "vc871",
            }],
            Some(W::OtherFrame { .. }) => chain(|last: Option<&FrameRemedy>| match last {
                None => vec![FrameRemedy::AutoDetect],
                Some(FrameRemedy::AutoDetect) => vec![FrameRemedy::OwnModel],
                Some(FrameRemedy::OwnModel) => vec![],
            })
            .into_iter()
            .map(|remedy| W::OtherFrameKind {
                sent: 6,
                expected: 15,
                opened: "Voltcraft VC871",
                remedy,
            })
            .collect(),
            Some(W::OtherFrameKind { .. }) => chain(|last: Option<&Unread>| match last {
                None => vec![Unread::NotYet],
                Some(Unread::NotYet) => vec![Unread::SignInFunctionWord],
                Some(Unread::SignInFunctionWord) => vec![],
            })
            .into_iter()
            .map(|why| W::UnreadModel { code: 83, why })
            .collect(),
            Some(W::UnreadModel { .. }) => vec![W::OtherSeries {
                found: "Brymen BM82x",
                opened: "Brymen BM86x",
                id: "bm82x",
            }],
            Some(W::OtherSeries { .. }) => vec![W::NoSixByteFrames],
            Some(W::NoSixByteFrames) => vec![W::NoFifteenByteFrames],
            Some(W::NoFifteenByteFrames) => vec![W::UntoldFrameKind],
            Some(W::UntoldFrameKind) => vec![],
        })
    }

    fn bluetooth_only_misses() -> Vec<BluetoothOnlyMiss> {
        use BluetoothOnlyMiss as M;
        chain(|last: Option<&M>| match last {
            None => vec![M::NotInRange],
            Some(M::NotInRange) => vec![M::SwitchedOff],
            Some(M::SwitchedOff) => vec![M::NotBuilt],
            Some(M::NotBuilt) => vec![M::UsbAdapter],
            Some(M::UsbAdapter) => vec![],
        })
    }

    fn errors() -> Vec<Error> {
        chain(|last: Option<&Error>| match last {
            None => vec![Error::Hid(hidapi::HidError::HidApiError {
                message: "hid_read_timeout: device disconnected".into(),
            })],
            Some(Error::Hid(_)) => vec![Error::InvalidResponse {
                message: "frame too short".into(),
                raw: vec![0xAB],
            }],
            Some(Error::InvalidResponse { .. }) => wrong_meter()
                .into_iter()
                .map(|reason| Error::WrongMeter {
                    reason,
                    raw: vec![0x2B],
                })
                .collect(),
            Some(Error::WrongMeter { .. }) => vec![Error::ChecksumMismatch {
                expected: 0x01a3,
                actual: 0x0003,
            }],
            Some(Error::ChecksumMismatch { .. }) => vec![Error::Timeout],
            Some(Error::Timeout) => vec![Error::UnknownMode(0x1f)],
            Some(Error::UnknownMode(_)) => unsupported().into_iter().map(Error::from).collect(),
            Some(Error::UnsupportedCommand(_)) => rejected().into_iter().map(Error::from).collect(),
            Some(Error::CommandRejected(_)) => vec![Error::UnknownDevice("ut99".into())],
            Some(Error::UnknownDevice(_)) => vec![Error::NotSimulated("ut61eplus")],
            Some(Error::NotSimulated(_)) => {
                vec![Error::Replay("line 3: `ZZ` is not a hex byte".into())]
            }
            Some(Error::Replay(_)) => vec![Error::AdapterNotFound("12:34:56:78:9A:BC".into())],
            Some(Error::AdapterNotFound(_)) => vec![Error::NotAnAdapter("/dev/hidraw3".into())],
            Some(Error::NotAnAdapter(_)) => ["BU-86X", dmm_lib::BLUETOOTH]
                .into_iter()
                .map(|cable| Error::WrongCable {
                    cable,
                    model: "UT61E+",
                })
                .collect(),
            Some(Error::WrongCable { .. }) => vec![Error::LinkLost],
            Some(Error::LinkLost) => bluetooth().into_iter().map(Error::from).collect(),
            Some(Error::Bluetooth(_)) => ["CH9329", dmm_lib::BLUETOOTH]
                .into_iter()
                .flat_map(|bridge| {
                    [false, true].map(|built_in_radio| Error::DeviceNotIdentified {
                        bridge,
                        built_in_radio,
                    })
                })
                .collect(),
            Some(Error::DeviceNotIdentified { .. }) => {
                [vec![], vec!["BU-86X"], vec!["CP2110", "CH9329", "nope"]]
                    .into_iter()
                    .flat_map(|cables| {
                        [false, true].map(|bluetooth_searched| Error::NoTransportFound {
                            cables: cables.clone(),
                            bluetooth_searched,
                        })
                    })
                    .collect()
            }
            Some(Error::NoTransportFound { .. }) => bluetooth_only_misses()
                .into_iter()
                .map(|miss| Error::BluetoothOnly {
                    model: "UT60BT",
                    activation: "",
                    miss,
                })
                .collect(),
            Some(Error::BluetoothOnly { .. }) => vec![],
        })
    }

    /// English is the library's own sentence, byte for byte, for every
    /// variant and every reason.
    #[test]
    fn english_errors_read_as_the_librarys() {
        let l = Localizer::en_us(LIB_RESOURCES);
        for e in errors() {
            assert_eq!(l.error(&e), e.to_string(), "{e:?}");
        }
        for reason in rejected() {
            assert_eq!(l.rejected(&reason), reason.to_string(), "{reason:?}");
        }
    }

    /// A replay file's text quoted in a refusal can't act on the terminal
    /// it is printed to.
    #[test]
    fn replay_file_text_stays_inert() {
        let l = Localizer::en_us(LIB_RESOURCES);
        let mut text =
            dmm_lib::replay::header("ut61eplus", "2026-10-31T10:00:00+01:00", None, None);
        text.push_str("0 02 \u{1b}[2J\n");
        let e = dmm_lib::replay::Replay::parse(&text)
            .err()
            .expect("not a hex byte");
        let message = l.error(&e);
        assert!(
            !message.contains('\u{1b}') && message.contains("`\\u{1b}[2J` is not a hex byte"),
            "{message}"
        );
    }

    /// The late-readings notice reads as the library's, with and without
    /// a command.
    #[test]
    fn english_late_readings_read_as_the_librarys() {
        let l = Localizer::en_us(LIB_RESOURCES);
        assert_eq!(l.late_readings_title(), LateReadings::TITLE);
        for command in [None, Some("sudo fix".to_string())] {
            let notice = LateReadings { command };
            assert_eq!(l.late_readings_advice(&notice), notice.advice());
        }
    }

    /// The help beside an adapter-not-found error quotes what `--adapter`
    /// named as the error does.
    #[test]
    fn adapter_named_quotes_the_error() {
        let l = Localizer::en_us(LIB_RESOURCES);
        for e in [
            Error::AdapterNotFound("12:34:56:78:9A:BC".into()),
            Error::NotAnAdapter("/dev/hidraw3".into()),
        ] {
            let named = l.adapter_named(&e).expect("an adapter error");
            assert_eq!(format!("adapter not found: {named}"), e.to_string());
        }
        assert!(l.adapter_named(&Error::LinkLost).is_none());
    }

    /// A MIN/MAX walk's states are flag names, translated as the flags
    /// are; on and off are `lib.ftl`'s.
    #[test]
    fn flag_named_states_translate_as_their_flag() {
        static FRENCH: crate::l10n::Language = crate::l10n::Language {
            id: "fr",
            native_name: "Français",
            reviewed: true,
        };
        let resources = [
            &crate::l10n::lib_index::english_lib()[..],
            &[
                crate::l10n::Resource {
                    locale: "fr",
                    path: "lib-names.ftl",
                    source: "flag-max = MAXI\nflag-min = MINI\n",
                },
                crate::l10n::Resource {
                    locale: "fr",
                    path: "lib.ftl",
                    source: "toggle-off = arrêt\n",
                },
            ],
        ]
        .concat();
        let l = Localizer::new(&FRENCH, &resources, &crate::l10n::Options::default());
        let e = Error::from(Unsupported::NoSuchState {
            setting: Setting::MinMax,
            id: 7,
            mode: s("DC V"),
            offered: vec![s("off"), s("MAX"), s("MIN")],
        });
        let text = l.error(&e);
        assert!(text.contains("arrêt, MAXI, MINI"), "{text}");
        let e = Error::from(Rejected::ExitDidNothing {
            button: "MAX/MIN",
            state: s("MAX"),
        });
        let text = l.error(&e);
        assert!(text.contains("MAXI") && text.contains("MAX/MIN"), "{text}");
    }

    /// The pseudo-locale reaches every message, and leaves what passes
    /// through as it is.
    #[test]
    fn errors_go_through_the_messages() {
        let options = crate::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        let l = Localizer::new(&crate::l10n::EN_US, LIB_RESOURCES, &options);
        for e in errors() {
            let text = l.error(&e);
            assert!(text.starts_with('['), "{e:?}: {text}");
        }
        let stack = Error::from(BluetoothIssue::Stack("Timed out after 10s".into()));
        assert!(l.error(&stack).contains("Timed out after 10s"));
    }
}
