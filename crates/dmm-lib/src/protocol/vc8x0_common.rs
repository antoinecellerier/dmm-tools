//! Shared helpers for the Voltcraft VC-880 and VC-890 protocol families.
//!
//! Both use AB CD framing with BE16 checksums and share identical command
//! byte assignments and DeviceID retrieval logic.

use crate::error::{Error, Result};
use crate::protocol::CaptureStep;
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::transport::Transport;
use log::debug;

/// Build a command frame: `[0xAB, 0xCD, 0x03, cmd, chk_hi, chk_lo]`.
pub(crate) fn build_command(cmd: u8) -> Vec<u8> {
    let mut frame = vec![0xAB, 0xCD, 0x03, cmd];
    let sum: u16 = frame.iter().map(|&b| b as u16).sum();
    frame.push((sum >> 8) as u8);
    frame.push((sum & 0xFF) as u8);
    frame
}

/// Map a command name to its byte value.
///
/// Command bytes are identical for VC-880 and VC-890.
pub(crate) fn command_byte(command: &str) -> Result<u8> {
    match command {
        "hold" => Ok(0x4A),
        "rel" => Ok(0x48),
        "max_min_avg" => Ok(0x49),
        "exit_max_min_avg" => Ok(0x43),
        "range_auto" => Ok(0x47),
        "range_manual" => Ok(0x46),
        "light" => Ok(0x4B),
        "select" => Ok(0x4C),
        _ => Err(Error::UnsupportedCommand(command.to_string())),
    }
}

/// Send the GetDeviceID command (0x00) and read the 20-byte ASCII name.
pub(crate) fn read_device_name(
    rx_buf: &mut Vec<u8>,
    transport: &dyn Transport,
    label: &str,
) -> Result<Option<String>> {
    let frame = build_command(0x00);
    debug!("{label}: sending GetDeviceID command");
    transport.write(&frame)?;

    match framing::read_frame(
        rx_buf,
        transport,
        framing::extract_frame_abcd_be16,
        |p| !p.is_empty() && p[0] == 0x00, // DeviceID type
        FrameErrorRecovery::SkipAndRetry,
        &format!("{label}-id"),
        &framing::HEADER,
    ) {
        Ok(payload) if payload.len() >= 21 => {
            let name = String::from_utf8_lossy(&payload[1..21]).trim().to_string();
            debug!("{label}: device name: {name}");
            if name.is_empty() {
                Ok(None)
            } else {
                Ok(Some(name))
            }
        }
        Ok(_) => {
            debug!("{label}: DeviceID response too short");
            Ok(None)
        }
        Err(e) => {
            debug!("{label}: failed to read DeviceID: {e}");
            Ok(None)
        }
    }
}

/// Button commands, identical on both meters.
pub(crate) const COMMANDS: &[&str] = &[
    "hold",
    "rel",
    "max_min_avg",
    "exit_max_min_avg",
    "range_auto",
    "range_manual",
    "light",
    "select",
];

/// Capture steps shared by the VC-880 and VC-890.
///
/// Both meters expose the same dial positions, so the list lived as ~120
/// duplicated lines in each protocol. Device-specific steps are appended by
/// the caller.
pub(crate) fn capture_steps() -> Vec<CaptureStep> {
    vec![
        CaptureStep {
            id: "dcv",
            instruction: "Set meter to DC V",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "acv",
            instruction: "Set meter to AC V",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "acdcv",
            instruction: "Set meter to AC+DC V",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "dcmv",
            instruction: "Set meter to DC mV",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "dcua",
            instruction: "Set meter to DC µA",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "acua",
            instruction: "Set meter to AC µA",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "dcma",
            instruction: "Set meter to DC mA",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "acma",
            instruction: "Set meter to AC mA",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "dca",
            instruction: "Set meter to DC A",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "aca",
            instruction: "Set meter to AC A",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "ohm",
            instruction: "Set meter to Resistance (Ω)",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "cont",
            instruction: "Set meter to Continuity",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "diode",
            instruction: "Set meter to Diode",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "cap",
            instruction: "Set meter to Capacitance",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "hz",
            instruction: "Set meter to Frequency (Hz)",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "duty",
            instruction: "Set meter to Duty Cycle (%)",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "tempc",
            instruction: "Set meter to Temperature °C",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "tempf",
            instruction: "Set meter to Temperature °F",
            command: None,
            samples: 5,
        },
        CaptureStep {
            id: "lpf",
            instruction: "Set meter to ACV Low-Pass Filter",
            command: None,
            samples: 5,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_checksum() {
        let frame = build_command(0x4A);
        assert_eq!(frame.len(), 6);
        assert_eq!(&frame[..4], &[0xAB, 0xCD, 0x03, 0x4A]);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    #[test]
    fn command_byte_known() {
        assert_eq!(command_byte("hold").unwrap(), 0x4A);
        assert_eq!(command_byte("rel").unwrap(), 0x48);
        assert_eq!(command_byte("light").unwrap(), 0x4B);
    }

    #[test]
    fn command_byte_unknown() {
        assert!(command_byte("nonexistent").is_err());
    }
}
