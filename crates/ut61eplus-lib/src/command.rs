/// Remote commands (button presses) that can be sent to the meter.
///
/// Encoding: [0xAB, 0xCD, 0x03, cmd, (cmd+379)>>8, (cmd+379)&0xFF]
///
/// Values from ljakob/unit_ut61eplus (Python), verified against real device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Request a measurement reading.
    GetMeasurement = 0x5E,
    /// Get device name.
    GetName = 0x5F,
    /// Toggle MIN/MAX mode.
    MinMax = 0x41,
    /// Exit MIN/MAX mode.
    ExitMinMax = 0x42,
    /// Toggle range (auto/manual).
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
        let cmd = self as u8;
        let check = cmd as u16 + 379;
        [
            0xAB,
            0xCD,
            0x03,
            cmd,
            (check >> 8) as u8,
            (check & 0xFF) as u8,
        ]
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
