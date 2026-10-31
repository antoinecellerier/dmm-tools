//! Why the Bluetooth stack could not do what was asked: the reason
//! [`Error::Bluetooth`] carries.
//!
//! Each reason is one sentence, with its variable parts as fields, so an app
//! can word it in the user's language; `Display` is the library's English.
//! The stack's own words (btleplug, BlueZ, WinRT, CoreBluetooth) are not the
//! library's to translate: they pass through as [`BluetoothIssue::Stack`], or
//! as the `detail` of a reason that leads in with its own sentence.
//!
//! Not cfg'd on the `bluetooth` feature, so an app's renderer compiles in
//! every build. Deliberately not `#[non_exhaustive]`: an app's exhaustive
//! `match` stops compiling when a reason is added, rather than falling back
//! to English.
//!
//! [`Error::Bluetooth`]: super::Error::Bluetooth

use std::fmt;
use thiserror::Error;

/// Why the Bluetooth stack could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BluetoothIssue {
    /// The stack's own error text, unchanged.
    #[error("{0}")]
    Stack(String),

    /// The async runtime the stack runs inside could not start; `detail` is
    /// the operating system's text.
    #[error("could not start the Bluetooth runtime: {detail}")]
    Runtime { detail: String },

    /// The computer has no Bluetooth adapter.
    #[error("not available on this computer")]
    NoAdapter,

    /// The computer's Bluetooth adapter is powered off.
    #[error("turned off on this computer")]
    PoweredOff,

    /// The peer answered but carries no profile we know: the closest one
    /// lacks its `role` characteristic.
    #[error(
        "the Bluetooth device has no data {role} characteristic we recognise — it is not a \
         supported adapter or meter, or its services never resolved"
    )]
    NoCharacteristic { role: CharacteristicRole },

    /// A Brymen meter refused the connection password, 0000. The sentence
    /// ends with the meter's reset gesture.
    #[error("{}", crate::protocol::bm78xbt::login::PASSWORD_REFUSED)]
    PasswordRefused,

    /// A Brymen meter refused the login with error `code`, any but the
    /// password's, which is [`BluetoothIssue::PasswordRefused`].
    #[error(
        "the meter refused the login: {} (code {code})",
        crate::protocol::bm78xbt::login::refusal_reason(*.code)
    )]
    LoginRefused { code: u16 },
}

/// Which of a profile's two characteristics a peer lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacteristicRole {
    /// The one the link writes commands to.
    Write,
    /// The one the meter's data arrives on.
    Notify,
}

impl fmt::Display for CharacteristicRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CharacteristicRole::Write => "write",
            CharacteristicRole::Notify => "notify",
        })
    }
}

impl From<BluetoothIssue> for super::Error {
    fn from(issue: BluetoothIssue) -> Self {
        super::Error::Bluetooth(issue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each reason reads as the sentence the library printed before reasons
    /// were enums, word for word: logs and library users see `Display`.
    #[test]
    fn each_issue_reads_as_its_sentence() {
        let issues = [
            (
                BluetoothIssue::Stack("Timed out after 10s".into()),
                "Timed out after 10s",
            ),
            (
                BluetoothIssue::Runtime {
                    detail: "Too many open files (os error 24)".into(),
                },
                "could not start the Bluetooth runtime: Too many open files (os error 24)",
            ),
            (BluetoothIssue::NoAdapter, "not available on this computer"),
            (BluetoothIssue::PoweredOff, "turned off on this computer"),
            (
                BluetoothIssue::NoCharacteristic {
                    role: CharacteristicRole::Write,
                },
                "the Bluetooth device has no data write characteristic we recognise — it is not \
                 a supported adapter or meter, or its services never resolved",
            ),
            (
                BluetoothIssue::NoCharacteristic {
                    role: CharacteristicRole::Notify,
                },
                "the Bluetooth device has no data notify characteristic we recognise — it is \
                 not a supported adapter or meter, or its services never resolved",
            ),
            (
                BluetoothIssue::PasswordRefused,
                "the meter refused the connection password 0000 — to reset it, hold the Hz \
                 button while turning the dial from OFF to capacitance within 0.6 s: the meter \
                 shows \"Org\" and its connection password and Bluetooth name are back to the \
                 factory settings",
            ),
            (
                BluetoothIssue::LoginRefused { code: 0 },
                "the meter refused the login: checksum error (code 0)",
            ),
            (
                BluetoothIssue::LoginRefused { code: 1 },
                "the meter refused the login: invalid channel ID (code 1)",
            ),
            (
                BluetoothIssue::LoginRefused { code: 2 },
                "the meter refused the login: out of setting range (code 2)",
            ),
            (
                BluetoothIssue::LoginRefused { code: 5 },
                "the meter refused the login: invalid arguments (code 5)",
            ),
            (
                BluetoothIssue::LoginRefused { code: 6 },
                "the meter refused the login: insufficient permissions (code 6)",
            ),
            (
                BluetoothIssue::LoginRefused { code: 261 },
                "the meter refused the login: unknown error (code 261)",
            ),
        ];
        for (issue, sentence) in issues {
            assert_eq!(issue.to_string(), sentence, "{issue:?}");
            let error = super::super::Error::from(issue);
            assert_eq!(error.to_string(), format!("Bluetooth: {sentence}"));
            assert!(std::error::Error::source(&error).is_none(), "{error:?}");
        }
    }
}
