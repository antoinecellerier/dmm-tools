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
}

pub type Result<T> = std::result::Result<T, Error>;
