/// One status flag, so consumers can enumerate flags instead of hand-listing fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    Hold,
    Rel,
    AutoRange,
    Min,
    Max,
    LowBattery,
    HvWarning,
    PeakMax,
    PeakMin,
    LeadError,
    Comp,
    Record,
    LoZ,
    Void,
    Dc,
}

impl Flag {
    /// Every flag, in the order [`StatusFlags`]'s `Display` prints them; `Dc`
    /// last because it has no label and is never printed.
    pub const ALL: [Flag; StatusFlags::COUNT] = [
        Flag::Hold,
        Flag::Rel,
        Flag::AutoRange,
        Flag::Min,
        Flag::Max,
        Flag::LowBattery,
        Flag::HvWarning,
        Flag::PeakMax,
        Flag::PeakMin,
        Flag::LeadError,
        Flag::Comp,
        Flag::Record,
        Flag::LoZ,
        Flag::Void,
        Flag::Dc,
    ];

    /// Machine-readable snake_case name — the JSON/YAML key for this flag.
    /// Renaming one is a breaking change for downstream consumers.
    pub fn name(self) -> &'static str {
        match self {
            Flag::Hold => "hold",
            Flag::Rel => "rel",
            Flag::AutoRange => "auto_range",
            Flag::Min => "min",
            Flag::Max => "max",
            Flag::LowBattery => "low_battery",
            Flag::HvWarning => "hv_warning",
            Flag::PeakMax => "peak_max",
            Flag::PeakMin => "peak_min",
            Flag::LeadError => "lead_error",
            Flag::Comp => "comp",
            Flag::Record => "record",
            Flag::LoZ => "loz",
            Flag::Void => "void",
            Flag::Dc => "dc",
        }
    }

    /// Short label as printed by [`StatusFlags`]'s `Display`.
    ///
    /// `None` for `Dc`: the DC/AC distinction is carried by the measurement
    /// mode, so it is never printed as a flag.
    pub fn label(self) -> Option<&'static str> {
        Some(match self {
            Flag::Hold => "HOLD",
            Flag::Rel => "REL",
            Flag::AutoRange => "AUTO",
            Flag::Min => "MIN",
            Flag::Max => "MAX",
            Flag::LowBattery => "LOW BAT",
            Flag::HvWarning => "HV!",
            Flag::PeakMax => "P-MAX",
            Flag::PeakMin => "P-MIN",
            Flag::LeadError => "LEAD ERR",
            Flag::Comp => "COMP",
            Flag::Record => "REC",
            Flag::LoZ => "LoZ",
            Flag::Void => "VOID",
            Flag::Dc => return None,
        })
    }
}

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
    pub lead_error: bool,
    pub comp: bool,
    pub record: bool,
    /// Low-impedance voltage measurement active (e.g. VC-890 byte 59 bit 2).
    pub loz: bool,
    /// Reading marked invalid by the meter (e.g. VC-890 byte 59 bit 3, via
    /// misplug / reference-disconnect detection).
    pub void: bool,
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
            // Inverted: bit2 of flag2 is the MANUAL range indicator.
            // When clear (0), the meter is in auto-range mode.
            auto_range: flag2 & 0x04 == 0,
            dc: flag3 & 0x08 != 0,
            peak_max: flag3 & 0x04 != 0,
            peak_min: flag3 & 0x02 != 0,
            ..Default::default()
        }
    }

    /// Number of flags in [`StatusFlags::as_pairs`] — i.e. every field.
    pub const COUNT: usize = 15;

    /// Value of a single flag. Exhaustive, so a new field fails to compile
    /// until it is wired into [`Flag`].
    pub fn get(&self, flag: Flag) -> bool {
        match flag {
            Flag::Hold => self.hold,
            Flag::Rel => self.rel,
            Flag::AutoRange => self.auto_range,
            Flag::Min => self.min,
            Flag::Max => self.max,
            Flag::LowBattery => self.low_battery,
            Flag::HvWarning => self.hv_warning,
            Flag::PeakMax => self.peak_max,
            Flag::PeakMin => self.peak_min,
            Flag::LeadError => self.lead_error,
            Flag::Comp => self.comp,
            Flag::Record => self.record,
            Flag::LoZ => self.loz,
            Flag::Void => self.void,
            Flag::Dc => self.dc,
        }
    }

    /// The flags that are currently set, in [`Flag::ALL`] order.
    pub fn active(&self) -> impl Iterator<Item = Flag> + '_ {
        Flag::ALL.into_iter().filter(|&flag| self.get(flag))
    }

    /// Every flag as a `(machine-readable name, value)` pair, from [`Flag::ALL`].
    ///
    /// Exists so consumers that need to enumerate the flags — the CLI's JSON
    /// output, the capture report — don't each hand-maintain their own list.
    /// Those lists had already drifted: JSON was missing `loz` and `void`,
    /// and the capture report was missing five flags, so a VC-890 reading the
    /// meter had marked VOID looked clean in both.
    ///
    /// Names are snake_case and are part of those output formats; renaming one
    /// is a breaking change for downstream consumers.
    ///
    /// The element order is [`Flag::ALL`]'s, not the struct field order, and no
    /// consumer observes it: the CLI's JSON arm collects the pairs into a
    /// `serde_json::Map` (a `BTreeMap`, which re-sorts by key), and the tests
    /// only check that names are present.
    pub fn as_pairs(&self) -> [(&'static str, bool); Self::COUNT] {
        Flag::ALL.map(|flag| (flag.name(), self.get(flag)))
    }
}

impl std::fmt::Display for StatusFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for label in self.active().filter_map(Flag::label) {
            if !first {
                f.write_str(" ")?;
            }
            f.write_str(label)?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field set, spelled out as a full struct literal so that adding a
    /// field to `StatusFlags` breaks this helper until the flag is wired up.
    fn all_set() -> StatusFlags {
        StatusFlags {
            hold: true,
            rel: true,
            min: true,
            max: true,
            auto_range: true,
            low_battery: true,
            hv_warning: true,
            dc: true,
            peak_max: true,
            peak_min: true,
            lead_error: true,
            comp: true,
            record: true,
            loz: true,
            void: true,
        }
    }

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

    #[test]
    fn new_flags_default_false() {
        let flags = StatusFlags::default();
        assert!(!flags.lead_error);
        assert!(!flags.comp);
        assert!(!flags.record);
    }

    #[test]
    fn display_lead_error() {
        let flags = StatusFlags {
            lead_error: true,
            ..Default::default()
        };
        assert!(flags.to_string().contains("LEAD ERR"));
    }

    #[test]
    fn display_comp_and_record() {
        let flags = StatusFlags {
            comp: true,
            record: true,
            ..Default::default()
        };
        let s = flags.to_string();
        assert!(s.contains("COMP"));
        assert!(s.contains("REC"));
    }

    #[test]
    fn all_flags_set_prints_every_label_in_order() {
        assert_eq!(
            all_set().to_string(),
            "HOLD REL AUTO MIN MAX LOW BAT HV! P-MAX P-MIN LEAD ERR COMP REC LoZ VOID"
        );
    }

    #[test]
    fn active_covers_every_field() {
        assert_eq!(all_set().active().count(), StatusFlags::COUNT);

        let mut names: Vec<&str> = Flag::ALL.iter().map(|flag| flag.name()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate flag names in Flag::ALL");
    }

    #[test]
    fn as_pairs_names_match_flag_names() {
        let pairs = StatusFlags::default().as_pairs();
        let names = Flag::ALL.map(Flag::name);
        for (pair, name) in pairs.iter().zip(names.iter()) {
            assert_eq!(pair.0, *name);
        }

        for name in names {
            assert!(
                !name.is_empty()
                    && !name.starts_with('_')
                    && !name.ends_with('_')
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{name} is not snake_case ASCII"
            );
        }
    }

    #[test]
    fn dc_has_no_label_and_is_last() {
        assert!(Flag::Dc.label().is_none());
        assert_eq!(Flag::ALL.last(), Some(&Flag::Dc));
    }
}
