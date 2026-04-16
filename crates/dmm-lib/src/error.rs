use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("device not found (VID={vid:#06x}, PID={pid:#06x})")]
    DeviceNotFound { vid: u16, pid: u16 },

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

    #[error("unknown device: {0}")]
    UnknownDevice(String),

    #[error("adapter not found: {0}")]
    AdapterNotFound(String),

    #[error(
        "no supported USB adapter found (tried CP2110 {:#06x}:{:#06x}, CH9329 {:#06x}:{:#06x}, CH9325 {:#06x}:{:#06x})",
        0x10C4,
        0xEA80,
        0x1A86,
        0xE429,
        0x1A86,
        0xE008
    )]
    NoTransportFound,
}

impl Error {
    /// Convenience constructor for InvalidResponse with raw bytes.
    pub fn invalid_response(message: impl Into<String>, raw: &[u8]) -> Self {
        Self::InvalidResponse {
            message: message.into(),
            raw: raw.to_vec(),
        }
    }

    /// Convenience constructor for InvalidResponse without raw bytes.
    pub fn invalid_response_msg(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            message: message.into(),
            raw: Vec::new(),
        }
    }

    /// True when the error means no USB adapter was found on the bus.
    pub fn is_device_not_found(&self) -> bool {
        matches!(self, Self::DeviceNotFound { .. } | Self::NoTransportFound)
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
    /// User-side misconfiguration (unknown device ID, bad adapter selector,
    /// unsupported command). Reconnect won't help.
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
            Self::DeviceNotFound { .. } | Self::NoTransportFound => ErrorKind::DeviceNotFound,
            Self::Hid(_) => ErrorKind::Transport,
            Self::Timeout => ErrorKind::Timeout,
            Self::InvalidResponse { .. } | Self::ChecksumMismatch { .. } | Self::UnknownMode(_) => {
                ErrorKind::Protocol
            }
            Self::UnknownDevice(_) | Self::AdapterNotFound(_) | Self::UnsupportedCommand(_) => {
                ErrorKind::Configuration
            }
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

    #[test]
    fn kind_maps_not_found() {
        assert_eq!(
            Error::DeviceNotFound { vid: 0, pid: 0 }.kind(),
            ErrorKind::DeviceNotFound
        );
        assert_eq!(Error::NoTransportFound.kind(), ErrorKind::DeviceNotFound);
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
    }
}
