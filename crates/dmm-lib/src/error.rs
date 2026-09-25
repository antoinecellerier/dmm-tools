use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("invalid response: {message}")]
    InvalidResponse {
        message: String,
        /// Raw bytes that caused the error, for remote debugging.
        raw: Vec<u8>,
    },

    #[error("checksum mismatch: expected {expected:#06x}, got {actual:#06x}")]
    ChecksumMismatch { expected: u16, actual: u16 },

    #[error("timeout waiting for response")]
    Timeout,

    #[error("unknown mode: {0:#04x}")]
    UnknownMode(u8),

    #[error("unsupported command: {0}")]
    UnsupportedCommand(String),

    /// The meter received the command and refused it — typically because the
    /// dial isn't where the command needs it. Retrying the same command won't
    /// help; the user has to change something.
    #[error("command rejected: {0}")]
    CommandRejected(String),

    #[error("unknown device: {0}")]
    UnknownDevice(String),

    /// A replay file could not be read or makes no sense. The message names
    /// the offending line, so a hand-edited recording says where it broke.
    #[error("replay file: {0}")]
    Replay(String),

    #[error("adapter not found: {0}")]
    AdapterNotFound(String),

    /// The Bluetooth link went away mid-session — out of range, meter powered
    /// off, adapter asleep. The same shape as a pulled USB cable, so consumers
    /// reconnect from it.
    #[error("Bluetooth link lost")]
    LinkLost,

    /// The Bluetooth stack cannot do what was asked: no adapter, powered off,
    /// permission denied, the device carries no UART. Retrying never helps —
    /// the message carries what the user has to change.
    #[error("Bluetooth: {0}")]
    Bluetooth(String),

    /// Auto-detection ran its whole probe cascade and nothing on the far end
    /// of the cable answered with a frame we recognise.
    ///
    /// The message names the *link*, never the bridge chip: the user plugged
    /// in a USB cable or switched a Bluetooth adapter on, and has no reason to
    /// know whether it carries a CP2110, a CH9329 or a CH9325. `bridge` is
    /// carried for the logs and for help text that lists the meters reachable
    /// over that bridge. `built_in_radio` says the peer's advertised name is
    /// a registry meter's (`built_in_meters()` in `lib.rs`): a meter with the
    /// radio built in has no adapter for the message to name.
    #[error(
        "no meter answered over the {}",
        crate::binary_help::bridge_link_name(.bridge, *.built_in_radio)
    )]
    DeviceNotIdentified {
        bridge: &'static str,
        built_in_radio: bool,
    },

    /// The IDs come from the transport modules themselves rather than being
    /// spelled out here, so a corrected PID or a fourth bridge can't leave
    /// this message describing adapters we no longer look for.
    ///
    /// `bluetooth_searched` says whether the radio got a turn as well, which
    /// is what the binaries' help titles itself on — they would otherwise
    /// have to work the answer out again from the build, the settings and the
    /// selected meter.
    #[error(
        "no supported USB adapter found (tried CP2110 {:#06x}:{:#06x}, CH9329 {:#06x}:{:#06x}, CH9325 {:#06x}:{:#06x}){}",
        crate::transport::cp2110::VID,
        crate::transport::cp2110::PID,
        crate::transport::ch9329::VID,
        crate::transport::ch9329::PID,
        crate::transport::ch9325::VID,
        crate::transport::ch9325::PID,
        bluetooth_clause(.bluetooth_searched)
    )]
    NoTransportFound { bluetooth_searched: bool },

    /// A meter with the radio built in, which has no cable, was not opened:
    /// nothing in range carried its name, or the radio was not searched.
    ///
    /// `model` and `activation` are the registry entry's display name and
    /// activation steps, which the binaries' help is built from.
    #[error("{}", bluetooth_only_message(.model, .miss))]
    BluetoothOnly {
        model: &'static str,
        activation: &'static str,
        miss: BluetoothOnlyMiss,
    },
}

/// Why a meter with the radio built in was not opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothOnlyMiss {
    /// The radio was searched and nothing in range carried the meter's name.
    NotInRange,
    /// The Bluetooth setting is off, or `--no-bluetooth` was given.
    SwitchedOff,
    /// This build has no Bluetooth support.
    NotBuilt,
    /// `--adapter` names a USB device, which this meter cannot be behind.
    UsbAdapter,
}

/// The sentence both binaries print for a [`Error::BluetoothOnly`].
///
/// Switched off, it names no remedy: the switch is each binary's own
/// (`binary_help::cli_bluetooth_off_hint`, `gui_bluetooth_off_hint`).
fn bluetooth_only_message(model: &str, miss: &BluetoothOnlyMiss) -> String {
    let why = match miss {
        BluetoothOnlyMiss::NotInRange => return format!("no {model} found in Bluetooth range"),
        BluetoothOnlyMiss::SwitchedOff => "and Bluetooth is switched off",
        BluetoothOnlyMiss::NotBuilt => "and this build has no Bluetooth support",
        BluetoothOnlyMiss::UsbAdapter => {
            "and --adapter names a USB device: pass the meter's Bluetooth address, \
             or leave --adapter out"
        }
    };
    format!("{model} connects over Bluetooth only, {why}")
}

/// The rest of the "nothing found" message when the radio was searched too.
/// Saying so keeps the user from hunting for a cable fault that isn't there.
fn bluetooth_clause(searched: &bool) -> &'static str {
    if *searched {
        ", nor a Bluetooth device in range"
    } else {
        ""
    }
}

impl Error {
    /// Convenience constructor for InvalidResponse with raw bytes.
    pub(crate) fn invalid_response(message: impl Into<String>, raw: &[u8]) -> Self {
        Self::InvalidResponse {
            message: message.into(),
            raw: raw.to_vec(),
        }
    }

    /// Convenience constructor for InvalidResponse without raw bytes.
    pub(crate) fn invalid_response_msg(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            message: message.into(),
            raw: Vec::new(),
        }
    }

    /// True when the underlying cause is an interrupted system call (EINTR),
    /// which typically means a signal (e.g. Ctrl-C) arrived mid-read.
    pub fn is_interrupted(&self) -> bool {
        match self {
            Self::Hid(hidapi::HidError::HidApiError { message }) => message.contains("Interrupted"),
            Self::Hid(hidapi::HidError::IoError { error }) => {
                error.kind() == std::io::ErrorKind::Interrupted
            }
            _ => false,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Coarse classification of an [`Error`] for consumers that need to branch
/// on the failure category without matching every variant. Small and
/// `Copy` so it can be sent across channels or stored in UI state without
/// moving the underlying [`Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// USB adapter not found on the bus (unplugged, permissions, wrong VID/PID).
    /// Reconnect is expected to succeed once the hardware comes back.
    DeviceNotFound,
    /// I/O failure underneath the HID layer.
    Transport,
    /// Meter didn't respond in time.
    Timeout,
    /// Response arrived but couldn't be parsed (checksum, invalid response,
    /// unknown mode byte). Reconnect alone won't help if the protocol is wrong.
    Protocol,
    /// Something only the user can fix (unknown device ID, bad adapter
    /// selector, unsupported command, a command the meter refused).
    /// Reconnect won't help — report it and carry on streaming.
    Configuration,
    /// Interrupted system call — typically a Ctrl-C signal mid-read.
    Interrupted,
}

impl Error {
    /// Classify this error into a coarse [`ErrorKind`] for consumers that
    /// want to branch on the failure category.
    pub fn kind(&self) -> ErrorKind {
        if self.is_interrupted() {
            return ErrorKind::Interrupted;
        }
        match self {
            Self::NoTransportFound { .. }
            | Self::BluetoothOnly {
                miss: BluetoothOnlyMiss::NotInRange,
                ..
            } => ErrorKind::DeviceNotFound,
            // A lost link is the Bluetooth spelling of a pulled cable.
            Self::Hid(_) | Self::LinkLost => ErrorKind::Transport,
            // Nothing answered the probes — the same shape as a timeout, and
            // the same cure: reconnect once transmission is enabled.
            Self::Timeout | Self::DeviceNotIdentified { .. } => ErrorKind::Timeout,
            Self::InvalidResponse { .. } | Self::ChecksumMismatch { .. } | Self::UnknownMode(_) => {
                ErrorKind::Protocol
            }
            Self::UnknownDevice(_)
            | Self::AdapterNotFound(_)
            | Self::UnsupportedCommand(_)
            | Self::CommandRejected(_)
            | Self::Bluetooth(_)
            | Self::BluetoothOnly { .. }
            | Self::Replay(_) => ErrorKind::Configuration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_maps_timeout() {
        assert_eq!(Error::Timeout.kind(), ErrorKind::Timeout);
    }

    /// The GUI reconnect loop keys off `Timeout`: a meter whose
    /// transmission mode was still off answers the next probe, so this must
    /// not land in `Configuration` (which stops retrying). The message must
    /// stay chip-free — the user plugged in a cable, not a CH9329.
    #[test]
    fn kind_maps_not_identified() {
        let err = Error::DeviceNotIdentified {
            bridge: "CH9329",
            built_in_radio: false,
        };
        assert_eq!(err.kind(), ErrorKind::Timeout);
        let msg = err.to_string();
        assert!(msg.contains("USB cable"), "got {msg}");
        assert!(!msg.contains("CH9329"), "got {msg}");
    }

    /// The same error, over the other link: the user switched a Bluetooth
    /// adapter on, so the message must not send them looking at a cable.
    #[test]
    fn not_identified_names_the_link_it_happened_on() {
        let err = |built_in_radio| Error::DeviceNotIdentified {
            bridge: crate::BLUETOOTH,
            built_in_radio,
        };
        let msg = err(false).to_string();
        assert_eq!(msg, "no meter answered over the Bluetooth adapter");
        // A meter with the radio built in is no adapter.
        let msg = err(true).to_string();
        assert_eq!(msg, "no meter answered over the Bluetooth link");
    }

    /// The error a failed open carries, with `bluetooth_searched` as the
    /// open path would have set it.
    fn not_found(bluetooth_searched: bool) -> Error {
        Error::NoTransportFound { bluetooth_searched }
    }

    #[test]
    fn kind_maps_not_found() {
        assert_eq!(not_found(false).kind(), ErrorKind::DeviceNotFound);
        assert_eq!(not_found(true).kind(), ErrorKind::DeviceNotFound);
    }

    /// A lost link is what a pulled cable is: the GUI reconnects from both.
    /// A stack that cannot do what was asked is not — retrying a powered-off
    /// adapter forever would hide the one thing the user has to change.
    #[test]
    fn kind_separates_a_lost_link_from_a_misconfigured_stack() {
        assert_eq!(Error::LinkLost.kind(), ErrorKind::Transport);
        assert_eq!(
            Error::Bluetooth("turned off on this computer".into()).kind(),
            ErrorKind::Configuration
        );
    }

    /// A meter with the radio built in says what kept it from the radio, and
    /// only a meter out of range is the not-found the GUI waits out.
    #[test]
    fn a_bluetooth_only_meter_says_why_it_was_not_opened() {
        let err = |miss| Error::BluetoothOnly {
            model: "UT60BT",
            activation: "",
            miss,
        };
        assert_eq!(
            err(BluetoothOnlyMiss::NotInRange).to_string(),
            "no UT60BT found in Bluetooth range"
        );
        assert_eq!(
            err(BluetoothOnlyMiss::NotInRange).kind(),
            ErrorKind::DeviceNotFound
        );
        // No remedy: each binary adds its own switch.
        assert_eq!(
            err(BluetoothOnlyMiss::SwitchedOff).to_string(),
            "UT60BT connects over Bluetooth only, and Bluetooth is switched off"
        );
        for (miss, names) in [
            (BluetoothOnlyMiss::SwitchedOff, "switched off"),
            (BluetoothOnlyMiss::NotBuilt, "no Bluetooth support"),
            (BluetoothOnlyMiss::UsbAdapter, "--adapter"),
        ] {
            let msg = err(miss).to_string();
            assert!(
                msg.starts_with("UT60BT connects over Bluetooth only, and "),
                "{msg}"
            );
            assert!(msg.contains(names), "{msg}");
            assert!(!msg.contains("USB cable"), "{msg}");
            assert_eq!(err(miss).kind(), ErrorKind::Configuration);
        }
    }

    /// The GUI hands whole errors from the acquisition thread to the UI
    /// thread, so a variant that isn't `Send` would break that channel at a
    /// call site far from here.
    #[test]
    fn error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn kind_maps_protocol() {
        assert_eq!(
            Error::ChecksumMismatch {
                expected: 1,
                actual: 2
            }
            .kind(),
            ErrorKind::Protocol
        );
        assert_eq!(Error::UnknownMode(0xFF).kind(), ErrorKind::Protocol);
    }

    #[test]
    fn kind_maps_config() {
        assert_eq!(
            Error::UnknownDevice("foo".into()).kind(),
            ErrorKind::Configuration
        );
        assert_eq!(
            Error::UnsupportedCommand("bar".into()).kind(),
            ErrorKind::Configuration
        );
        // A bad replay file is a file the user has to fix, not a meter that
        // might answer the next poll.
        assert_eq!(
            Error::Replay("line 3: `ZZ` is not a hex byte".into()).kind(),
            ErrorKind::Configuration
        );
        // A refused command must not look like a transport fault: the GUI
        // reconnects on those, and reconnecting can't move the meter's dial.
        assert_eq!(
            Error::CommandRejected("baz".into()).kind(),
            ErrorKind::Configuration
        );
    }

    /// The message must name the adapters the code actually scans for. It
    /// used to hardcode all six literals, so a corrected PID would have left
    /// it advertising the old one.
    #[test]
    fn no_transport_message_uses_the_transport_constants() {
        let msg = not_found(false).to_string();
        for (vid, pid) in [
            (crate::transport::cp2110::VID, crate::transport::cp2110::PID),
            (crate::transport::ch9329::VID, crate::transport::ch9329::PID),
            (crate::transport::ch9325::VID, crate::transport::ch9325::PID),
        ] {
            assert!(msg.contains(&format!("{vid:#06x}:{pid:#06x}")), "got {msg}");
        }
    }

    /// An open that reached the radio says so; one that never did must not
    /// promise a search it never ran. Which it was is the open path's answer
    /// (see `bluetooth_is_next`), and it names no product — the adapters we
    /// speak to are the catalog's business.
    #[test]
    fn no_transport_message_names_every_link_tried() {
        let msg = not_found(true).to_string();
        assert!(
            msg.contains(", nor a Bluetooth device in range"),
            "got {msg}"
        );
        assert!(!msg.contains("UT-D07"), "got {msg}");
        assert!(
            !not_found(false).to_string().contains("Bluetooth"),
            "got {msg}"
        );
    }
}
