use std::fmt;

/// Measurement modes reported by the UT61+/UT161 family.
///
/// Names follow UNI-T's protocol deck, whose mode table spans meters and
/// clamps; the UT61E+ bytes are verified against real device captures. See
/// `docs/research/ut61eplus/reverse-engineered-protocol.md` §2.5.
///
/// The mode byte does NOT have a 0x30 high nibble — use the raw value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum Mode {
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
    // 0x16-0x1E: clamp functions and current variants no UT61+ dial
    // position is known to send, named after the protocol deck. The
    // vendor PC software's table has Hz and duty at 0x16/0x17 instead
    // (ut61eplus spec §2.5). The app names two more, dBV (0x1F) and
    // dBm (0x20), with no unit or range anywhere, so they stay out.
    ClampAcA = 0x16,
    ClampDcA = 0x17,
    LpfV = 0x18,
    AcDcV = 0x19,
    LpfA = 0x1A,
    AcDcA = 0x1B,
    ClampLpfA = 0x1C,
    ClampAcDcA = 0x1D,
    Inrush = 0x1E,
}

impl Mode {
    /// Every mode, indexed by its mode byte: this is the decode table, so
    /// its order must match the discriminants.
    pub(crate) const ALL: &'static [Mode] = &[
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
        Mode::ClampAcA,
        Mode::ClampDcA,
        Mode::LpfV,
        Mode::AcDcV,
        Mode::LpfA,
        Mode::AcDcA,
        Mode::ClampLpfA,
        Mode::ClampAcDcA,
        Mode::Inrush,
    ];

    pub(crate) fn from_byte(b: u8) -> Option<Self> {
        Self::ALL.get(usize::from(b)).copied()
    }
}

impl Mode {
    /// Return the mode name as a static string slice.
    ///
    /// Unlike `Display::fmt` / `.to_string()`, this avoids heap allocation
    /// and can be used directly in `Cow::Borrowed`.
    pub(crate) fn as_static_str(self) -> &'static str {
        match self {
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
            Mode::ClampAcA => "Clamp AC A",
            Mode::ClampDcA => "Clamp DC A",
            Mode::LpfV => "LPF V",
            Mode::AcDcV => "AC+DC V",
            Mode::LpfA => "LPF A",
            Mode::AcDcA => "AC+DC A",
            Mode::ClampLpfA => "Clamp LPF A",
            Mode::ClampAcDcA => "Clamp AC+DC A",
            Mode::Inrush => "Inrush",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_static_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            (0x16, Mode::ClampAcA),
            (0x17, Mode::ClampDcA),
            (0x18, Mode::LpfV),
            (0x19, Mode::AcDcV),
            (0x1A, Mode::LpfA),
            (0x1B, Mode::AcDcA),
            (0x1C, Mode::ClampLpfA),
            (0x1D, Mode::ClampAcDcA),
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
        // `from_byte` indexes `ALL` by the byte, so this is what ties each
        // position in `ALL` to its variant's discriminant.
        for mode in Mode::ALL {
            let byte = *mode as u8;
            assert_eq!(Mode::from_byte(byte).unwrap(), *mode);
        }
    }

    /// The bytes `from_byte` accepts are exactly `ALL`, in order: the specs
    /// dump walks `ALL`, so it covers every mode a frame can carry, once.
    #[test]
    fn all_holds_every_decodable_mode_in_byte_order() {
        let decoded: Vec<Mode> = (u8::MIN..=u8::MAX).filter_map(Mode::from_byte).collect();
        assert_eq!(Mode::ALL, decoded.as_slice());
    }

    #[test]
    fn from_byte_invalid() {
        for byte in [0x1F, 0xFF, 0x20] {
            assert_eq!(Mode::from_byte(byte), None, "{byte:#04x}");
        }
    }

    #[test]
    fn display_all_modes() {
        // Verify Display doesn't panic and produces non-empty strings
        for mode in Mode::ALL {
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

    /// A label names one mode: `set mode` and capture expectations match
    /// on it.
    #[test]
    fn labels_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for mode in Mode::ALL {
            assert!(
                seen.insert(mode.as_static_str()),
                "{mode:?} repeats a label"
            );
        }
    }

    #[test]
    fn mode_byte_is_raw_no_prefix() {
        // Protocol spec: mode byte does NOT have 0x30 prefix
        // All valid mode bytes should be in range 0x00..=0x1E
        for b in 0x00..=0x1E_u8 {
            assert!(
                Mode::from_byte(b).is_some(),
                "byte {b:#04x} should be a valid mode"
            );
        }
        // 0x30-prefixed versions should fail (they'd be 0x30..=0x4E)
        for b in 0x30..=0x4E_u8 {
            assert!(
                Mode::from_byte(b).is_none(),
                "byte {b:#04x} (0x30-prefixed) should not be valid"
            );
        }
    }
}
