use crate::error::Error;
use std::fmt;

/// Measurement modes reported by the UT61E+.
///
/// Values verified against real device captures and cross-checked with
/// ljakob/unit_ut61eplus (Python) and mwuertinger/ut61ep (Go).
///
/// The mode byte does NOT have a 0x30 high nibble — use the raw value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Mode {
    AcV = 0x00,
    AcMv = 0x01,
    DcV = 0x02,
    DcMv = 0x03,
    Hz = 0x04,
    DutyCycle = 0x05,
    Ohm = 0x06,
    Continuity = 0x07,
    Diode = 0x08,
    Capacitance = 0x09,
    TempC = 0x0A,
    TempF = 0x0B,
    DcUa = 0x0C,
    AcUa = 0x0D,
    DcMa = 0x0E,
    AcMa = 0x0F,
    DcA = 0x10,
    AcA = 0x11,
    Hfe = 0x12,
    Live = 0x13,
    Ncv = 0x14,
    LozV = 0x15,
    LozV2 = 0x16,
    Lpf = 0x17,
    LpfV = 0x18,
    AcDcV = 0x19,
    LpfMv = 0x1A,
    AcDcMv = 0x1B,
    LpfA = 0x1C,
    AcDcA2 = 0x1D,
    Inrush = 0x1E,
}

impl Mode {
    pub fn from_byte(b: u8) -> Result<Self, Error> {
        match b {
            0x00 => Ok(Mode::AcV),
            0x01 => Ok(Mode::AcMv),
            0x02 => Ok(Mode::DcV),
            0x03 => Ok(Mode::DcMv),
            0x04 => Ok(Mode::Hz),
            0x05 => Ok(Mode::DutyCycle),
            0x06 => Ok(Mode::Ohm),
            0x07 => Ok(Mode::Continuity),
            0x08 => Ok(Mode::Diode),
            0x09 => Ok(Mode::Capacitance),
            0x0A => Ok(Mode::TempC),
            0x0B => Ok(Mode::TempF),
            0x0C => Ok(Mode::DcUa),
            0x0D => Ok(Mode::AcUa),
            0x0E => Ok(Mode::DcMa),
            0x0F => Ok(Mode::AcMa),
            0x10 => Ok(Mode::DcA),
            0x11 => Ok(Mode::AcA),
            0x12 => Ok(Mode::Hfe),
            0x13 => Ok(Mode::Live),
            0x14 => Ok(Mode::Ncv),
            0x15 => Ok(Mode::LozV),
            0x16 => Ok(Mode::LozV2),
            0x17 => Ok(Mode::Lpf),
            0x18 => Ok(Mode::LpfV),
            0x19 => Ok(Mode::AcDcV),
            0x1A => Ok(Mode::LpfMv),
            0x1B => Ok(Mode::AcDcMv),
            0x1C => Ok(Mode::LpfA),
            0x1D => Ok(Mode::AcDcA2),
            0x1E => Ok(Mode::Inrush),
            _ => Err(Error::UnknownMode(b)),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Mode::AcV => "AC V",
            Mode::AcMv => "AC mV",
            Mode::DcV => "DC V",
            Mode::DcMv => "DC mV",
            Mode::Hz => "Hz",
            Mode::DutyCycle => "Duty %",
            Mode::Ohm => "Ω",
            Mode::Continuity => "Continuity",
            Mode::Diode => "Diode",
            Mode::Capacitance => "Capacitance",
            Mode::TempC => "°C",
            Mode::TempF => "°F",
            Mode::DcUa => "DC µA",
            Mode::AcUa => "AC µA",
            Mode::DcMa => "DC mA",
            Mode::AcMa => "AC mA",
            Mode::DcA => "DC A",
            Mode::AcA => "AC A",
            Mode::Hfe => "hFE",
            Mode::Live => "Live",
            Mode::Ncv => "NCV",
            Mode::LozV => "LoZ V",
            Mode::LozV2 => "LoZ V",
            Mode::Lpf => "LPF",
            Mode::LpfV => "LPF V",
            Mode::AcDcV => "AC+DC V",
            Mode::LpfMv => "LPF mV",
            Mode::AcDcMv => "AC+DC mV",
            Mode::LpfA => "LPF A",
            Mode::AcDcA2 => "AC+DC A",
            Mode::Inrush => "Inrush",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn from_byte_all_valid_modes() {
        let expected = [
            (0x00, Mode::AcV),
            (0x01, Mode::AcMv),
            (0x02, Mode::DcV),
            (0x03, Mode::DcMv),
            (0x04, Mode::Hz),
            (0x05, Mode::DutyCycle),
            (0x06, Mode::Ohm),
            (0x07, Mode::Continuity),
            (0x08, Mode::Diode),
            (0x09, Mode::Capacitance),
            (0x0A, Mode::TempC),
            (0x0B, Mode::TempF),
            (0x0C, Mode::DcUa),
            (0x0D, Mode::AcUa),
            (0x0E, Mode::DcMa),
            (0x0F, Mode::AcMa),
            (0x10, Mode::DcA),
            (0x11, Mode::AcA),
            (0x12, Mode::Hfe),
            (0x13, Mode::Live),
            (0x14, Mode::Ncv),
            (0x15, Mode::LozV),
            (0x16, Mode::LozV2),
            (0x17, Mode::Lpf),
            (0x18, Mode::LpfV),
            (0x19, Mode::AcDcV),
            (0x1A, Mode::LpfMv),
            (0x1B, Mode::AcDcMv),
            (0x1C, Mode::LpfA),
            (0x1D, Mode::AcDcA2),
            (0x1E, Mode::Inrush),
        ];
        for (byte, mode) in &expected {
            assert_eq!(
                Mode::from_byte(*byte).unwrap(),
                *mode,
                "byte {byte:#04x} should map to {mode:?}"
            );
        }
    }

    #[test]
    fn from_byte_roundtrip_via_repr() {
        // Verify that from_byte(variant as u8) == variant for all variants
        let all_modes = [
            Mode::AcV,
            Mode::AcMv,
            Mode::DcV,
            Mode::DcMv,
            Mode::Hz,
            Mode::DutyCycle,
            Mode::Ohm,
            Mode::Continuity,
            Mode::Diode,
            Mode::Capacitance,
            Mode::TempC,
            Mode::TempF,
            Mode::DcUa,
            Mode::AcUa,
            Mode::DcMa,
            Mode::AcMa,
            Mode::DcA,
            Mode::AcA,
            Mode::Hfe,
            Mode::Live,
            Mode::Ncv,
            Mode::LozV,
            Mode::LozV2,
            Mode::Lpf,
            Mode::LpfV,
            Mode::AcDcV,
            Mode::LpfMv,
            Mode::AcDcMv,
            Mode::LpfA,
            Mode::AcDcA2,
            Mode::Inrush,
        ];
        for mode in &all_modes {
            let byte = *mode as u8;
            assert_eq!(Mode::from_byte(byte).unwrap(), *mode);
        }
    }

    #[test]
    fn from_byte_invalid() {
        assert!(matches!(
            Mode::from_byte(0x1F),
            Err(Error::UnknownMode(0x1F))
        ));
        assert!(matches!(
            Mode::from_byte(0xFF),
            Err(Error::UnknownMode(0xFF))
        ));
        assert!(matches!(
            Mode::from_byte(0x20),
            Err(Error::UnknownMode(0x20))
        ));
    }

    #[test]
    fn display_all_modes() {
        // Verify Display doesn't panic and produces non-empty strings
        let all_modes = [
            Mode::AcV,
            Mode::AcMv,
            Mode::DcV,
            Mode::DcMv,
            Mode::Hz,
            Mode::DutyCycle,
            Mode::Ohm,
            Mode::Continuity,
            Mode::Diode,
            Mode::Capacitance,
            Mode::TempC,
            Mode::TempF,
            Mode::DcUa,
            Mode::AcUa,
            Mode::DcMa,
            Mode::AcMa,
            Mode::DcA,
            Mode::AcA,
            Mode::Hfe,
            Mode::Live,
            Mode::Ncv,
            Mode::LozV,
            Mode::LozV2,
            Mode::Lpf,
            Mode::LpfV,
            Mode::AcDcV,
            Mode::LpfMv,
            Mode::AcDcMv,
            Mode::LpfA,
            Mode::AcDcA2,
            Mode::Inrush,
        ];
        for mode in &all_modes {
            let s = mode.to_string();
            assert!(!s.is_empty(), "{mode:?} should have non-empty display");
        }
    }

    #[test]
    fn display_specific_labels() {
        assert_eq!(Mode::AcV.to_string(), "AC V");
        assert_eq!(Mode::DcV.to_string(), "DC V");
        assert_eq!(Mode::Ohm.to_string(), "Ω");
        assert_eq!(Mode::Capacitance.to_string(), "Capacitance");
        assert_eq!(Mode::TempC.to_string(), "°C");
        assert_eq!(Mode::TempF.to_string(), "°F");
        assert_eq!(Mode::DcUa.to_string(), "DC µA");
        assert_eq!(Mode::Hfe.to_string(), "hFE");
        assert_eq!(Mode::Ncv.to_string(), "NCV");
        assert_eq!(Mode::LozV.to_string(), "LoZ V");
        assert_eq!(Mode::Inrush.to_string(), "Inrush");
    }

    #[test]
    fn mode_byte_is_raw_no_prefix() {
        // Protocol spec: mode byte does NOT have 0x30 prefix
        // All valid mode bytes should be in range 0x00..=0x1E
        for b in 0x00..=0x1E_u8 {
            assert!(
                Mode::from_byte(b).is_ok(),
                "byte {b:#04x} should be a valid mode"
            );
        }
        // 0x30-prefixed versions should fail (they'd be 0x30..=0x4E)
        for b in 0x30..=0x4E_u8 {
            assert!(
                Mode::from_byte(b).is_err(),
                "byte {b:#04x} (0x30-prefixed) should not be valid"
            );
        }
    }
}
