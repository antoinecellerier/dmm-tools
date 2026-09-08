/// One status flag, so consumers can enumerate flags instead of hand-listing fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    Hold,
    Rel,
    AutoRange,
    Min,
    Max,
    Avg,
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
        Flag::Avg,
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
            Flag::Avg => "avg",
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
            Flag::Avg => "AVG",
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
    /// Average mode active (e.g. VC-880/VC-890 status byte 1 bit 1), the third
    /// step of the meter's MAX/MIN/AVG cycle.
    pub avg: bool,
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
    /// Number of flags in [`StatusFlags::as_pairs`] — i.e. every field.
    pub const COUNT: usize = 16;

    /// Value of a single flag. Exhaustive, so a new field fails to compile
    /// until it is wired into [`Flag`].
    pub fn get(&self, flag: Flag) -> bool {
        match flag {
            Flag::Hold => self.hold,
            Flag::Rel => self.rel,
            Flag::AutoRange => self.auto_range,
            Flag::Min => self.min,
            Flag::Max => self.max,
            Flag::Avg => self.avg,
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
            avg: true,
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
    fn display_hold_auto() {
        let flags = StatusFlags {
            hold: true,
            auto_range: true,
            ..Default::default()
        };
        assert_eq!(flags.to_string(), "HOLD AUTO");
    }

    #[test]
    fn display_empty_when_only_auto() {
        // AUTO alone shouldn't clutter display when it's the default
        let flags = StatusFlags {
            auto_range: true,
            ..Default::default()
        };
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
            "HOLD REL AUTO MIN MAX AVG LOW BAT HV! P-MAX P-MIN LEAD ERR COMP REC LoZ VOID"
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
