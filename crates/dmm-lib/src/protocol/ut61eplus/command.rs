/// Remote commands (button presses) that can be sent to the meter.
///
/// Encoding: [0xAB, 0xCD, 0x03, cmd, (cmd+379)>>8, (cmd+379)&0xFF]
///
/// Values from ljakob/unit_ut61eplus (Python), verified against real device.
///
/// The protocol deck's clamp-meter commands (0x43-0x45 and 0x4F, UT61E+ spec
/// §2.3) are left out: no UT61+ model has the features they drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Request a measurement reading.
    GetMeasurement = 0x5E,
    /// Get device name.
    GetName = 0x5F,
    /// Start the readings stream. On a cable the meter only acknowledges
    /// it; the UT-D07B Bluetooth adapter answers it by polling the meter
    /// itself and forwarding every reading
    /// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §3).
    StartStream = 0x5D,
    /// Toggle MIN/MAX mode.
    MinMax = 0x41,
    /// Exit MIN/MAX mode.
    ExitMinMax = 0x42,
    /// RANGE button.
    ///
    /// Verified on the UT61E+ (2026-09-07): the first press leaves auto on
    /// the rung the meter is already in, each further press steps one rung
    /// up, the top rung wraps to the bottom, and the mode byte never moves.
    /// Only [`Command::Auto`] returns to auto-ranging. Does nothing on the
    /// mV dial. See §6.1 of the UT61 family spec.
    Range = 0x46,
    /// Set auto-range.
    Auto = 0x47,
    /// Toggle REL (relative) mode.
    Rel = 0x48,
    /// Hz/USB SELECT button.
    Select2 = 0x49,
    /// Toggle HOLD mode.
    Hold = 0x4A,
    /// Toggle backlight.
    Light = 0x4B,
    /// Orange SELECT button (cycles modes within a dial position).
    Select = 0x4C,
    /// Toggle Peak MIN/MAX mode.
    PeakMinMax = 0x4D,
    /// Exit Peak mode.
    ExitPeak = 0x4E,
}

impl Command {
    /// Encode this command into the 6-byte wire format.
    pub fn encode(self) -> [u8; 6] {
        crate::protocol::framing::build_abcd_be16(self as u8, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_get_measurement() {
        // 0x5E + 379 = 94 + 379 = 473 = 0x01D9
        assert_eq!(
            Command::GetMeasurement.encode(),
            [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9]
        );
    }

    #[test]
    fn encode_hold() {
        // 0x4A + 379 = 74 + 379 = 453 = 0x01C5
        assert_eq!(Command::Hold.encode(), [0xAB, 0xCD, 0x03, 0x4A, 0x01, 0xC5]);
    }

    #[test]
    fn encode_light() {
        // 0x4B + 379 = 75 + 379 = 454 = 0x01C6
        assert_eq!(
            Command::Light.encode(),
            [0xAB, 0xCD, 0x03, 0x4B, 0x01, 0xC6]
        );
    }
}
