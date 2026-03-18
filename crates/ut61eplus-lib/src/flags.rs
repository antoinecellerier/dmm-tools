/// Status flags parsed from payload bytes 11-13 (after & 0x0F masking).
///
/// Bit mapping verified against real device captures and cross-checked
/// with ljakob/unit_ut61eplus (Python).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusFlags {
    pub hold: bool,
    pub rel: bool,
    pub min: bool,
    pub max: bool,
    pub auto_range: bool,
    pub low_battery: bool,
    pub hv_warning: bool,
    pub dc: bool,
    pub peak_max: bool,
    pub peak_min: bool,
}

impl StatusFlags {
    /// Parse flags from the three flag bytes (already masked with & 0x0F).
    ///
    /// Byte 11 (flag1): bit0=REL, bit1=HOLD, bit2=MIN, bit3=MAX
    /// Byte 12 (flag2): bit0=HV warning, bit1=Low Battery, bit2=!AUTO (inverted)
    /// Byte 13 (flag3): bit0=bar polarity, bit1=Peak MIN, bit2=Peak MAX, bit3=DC
    pub fn parse(flag1: u8, flag2: u8, flag3: u8) -> Self {
        Self {
            rel: flag1 & 0x01 != 0,
            hold: flag1 & 0x02 != 0,
            min: flag1 & 0x04 != 0,
            max: flag1 & 0x08 != 0,
            hv_warning: flag2 & 0x01 != 0,
            low_battery: flag2 & 0x02 != 0,
            auto_range: flag2 & 0x04 == 0, // inverted: bit clear = auto ON
            dc: flag3 & 0x08 != 0,
            peak_max: flag3 & 0x04 != 0,
            peak_min: flag3 & 0x02 != 0,
        }
    }
}

impl std::fmt::Display for StatusFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.hold {
            parts.push("HOLD");
        }
        if self.rel {
            parts.push("REL");
        }
        if self.auto_range {
            parts.push("AUTO");
        }
        if self.min {
            parts.push("MIN");
        }
        if self.max {
            parts.push("MAX");
        }
        if self.low_battery {
            parts.push("LOW BAT");
        }
        if self.hv_warning {
            parts.push("HV!");
        }
        if self.peak_max {
            parts.push("P-MAX");
        }
        if self.peak_min {
            parts.push("P-MIN");
        }
        write!(f, "{}", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_flags_auto_on() {
        // All zero → AUTO is on (inverted logic), everything else off
        let flags = StatusFlags::parse(0x00, 0x00, 0x00);
        assert!(!flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range); // inverted: bit clear = auto ON
        assert!(!flags.min);
        assert!(!flags.max);
        assert!(!flags.low_battery);
    }

    #[test]
    fn parse_hold_with_auto() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on)
        let flags = StatusFlags::parse(0x02, 0x00, 0x00);
        assert!(flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range);
    }

    #[test]
    fn parse_manual_range() {
        // flag2=0x04 → AUTO bit set → auto_range OFF
        let flags = StatusFlags::parse(0x00, 0x04, 0x00);
        assert!(!flags.auto_range);
    }

    #[test]
    fn parse_low_battery() {
        // flag2=0x02 → LOW BAT
        let flags = StatusFlags::parse(0x00, 0x02, 0x00);
        assert!(flags.low_battery);
        assert!(flags.auto_range); // AUTO still on (bit2 is clear)
    }

    #[test]
    fn parse_min_max() {
        // flag1: bit2=MIN, bit3=MAX
        let flags = StatusFlags::parse(0x0C, 0x00, 0x00);
        assert!(flags.min);
        assert!(flags.max);
    }

    #[test]
    fn parse_all_flag1() {
        // flag1=0x0F: REL + HOLD + MIN + MAX
        let flags = StatusFlags::parse(0x0F, 0x00, 0x00);
        assert!(flags.rel);
        assert!(flags.hold);
        assert!(flags.min);
        assert!(flags.max);
    }

    #[test]
    fn parse_dc_flag() {
        // flag3=0x08 → DC
        let flags = StatusFlags::parse(0x00, 0x00, 0x08);
        assert!(flags.dc);
    }

    #[test]
    fn parse_real_device_hold() {
        // Real capture: meter on DC V with HOLD active
        // flag bytes (masked): 0x02, 0x00, 0x01
        let flags = StatusFlags::parse(0x02, 0x00, 0x01);
        assert!(flags.hold);
        assert!(!flags.rel);
        assert!(flags.auto_range);
        assert!(!flags.low_battery);
    }

    #[test]
    fn display_hold_auto() {
        let flags = StatusFlags::parse(0x02, 0x00, 0x00);
        assert_eq!(flags.to_string(), "HOLD AUTO");
    }

    #[test]
    fn display_empty_when_only_auto() {
        // AUTO alone shouldn't clutter display when it's the default
        let flags = StatusFlags::parse(0x00, 0x00, 0x00);
        assert_eq!(flags.to_string(), "AUTO");
    }
}
