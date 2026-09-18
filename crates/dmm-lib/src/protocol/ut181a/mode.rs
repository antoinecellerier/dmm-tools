//! UT181A dial families: which modes and ranges each one reaches.
//!
//! The dial picks a family (the mode word's top byte); SET_MODE then moves
//! between the variants *within* that family, and SET_RANGE walks the family's
//! manual range ladder. The vendor Windows app never sends a mode word from
//! another family, and neither do we — the meter would have to be refused it.
//!
//! Every word here is traced from the vendor app, not from hardware: see
//! `docs/research/ut181/reverse-engineered-protocol.md`
//! §6.1 Mode switching (SET_MODE) -- [VENDOR].

use super::parse::{decode_mode_word, lookup_range_label};
use crate::protocol::{AUTO_RANGE_ID, AUTO_RANGE_LABEL, Choice};
use std::borrow::Cow;

/// Mask selecting the dial family of a mode word: nibble 3 (function family)
/// and nibble 2 (sub-function). The low byte is the variant and REL nibbles.
const FAMILY_MASK: u16 = 0xFF00;

/// Nibble 0 of a plain mode word, and of its REL companion.
const N0_PLAIN: u16 = 0x1;
const N0_REL: u16 = 0x2;

/// One dial position.
struct Family {
    /// Mode word masked with [`FAMILY_MASK`].
    base: u16,
    /// The primary variants the vendor app offers, in its own order. All have
    /// nibble 0 = 1 except the continuity and diode pairs, where nibble 0 = 2
    /// is a second function rather than REL.
    variants: &'static [u16],
    /// Nibble-1 values that have a REL companion at nibble 0 = 2 — the
    /// "REL for n1" column of research spec §6.1, which is the enable/disable
    /// gating each primary radio's click handler applies to the secondary
    /// radios. REL is *not* uniformly available: it is withheld on every Hz
    /// variant, every Peak variant and the two differential temperature
    /// arrangements, and continuity and diode spend nibble 0 = 2 on a second
    /// function instead.
    rel_n1: &'static [u16],
    /// Length of the manual range ladder; 0 = auto only, no SET_RANGE.
    manual_ranges: u8,
}

/// Labels are not stored: `decode_mode_word` already names every word, and a
/// second table would be a second thing to keep in step with it.
const FAMILIES: &[Family] = &[
    // V AC: plain, Hz, Peak, LowPass, dBV, dBm.
    Family {
        base: 0x1100,
        variants: &[0x1111, 0x1121, 0x1131, 0x1141, 0x1151, 0x1161],
        rel_n1: &[1, 4, 5, 6],
        manual_ranges: 4,
    },
    // mV AC: plain, Hz, Peak, AC+DC. 0x2141 is offered by the vendor UI but
    // missing from its own label decoder, so its name is inferred from the
    // nibble rule alone — [UNVERIFIED] (§6.1 "Caveats").
    Family {
        base: 0x2100,
        variants: &[0x2111, 0x2121, 0x2131, 0x2141],
        rel_n1: &[1, 4],
        manual_ranges: 2,
    },
    // V DC: plain, AC+DC, Peak.
    Family {
        base: 0x3100,
        variants: &[0x3111, 0x3121, 0x3131],
        rel_n1: &[1, 2],
        manual_ranges: 4,
    },
    // mV DC: plain, Peak.
    Family {
        base: 0x4100,
        variants: &[0x4111, 0x4121],
        rel_n1: &[1],
        manual_ranges: 2,
    },
    // Temperature: nibble 1 is the probe arrangement, not a variant —
    // T1(T2), T2(T1), T1-T2, T2-T1. The two differential arrangements have no
    // REL. Fixed range.
    Family {
        base: 0x4200,
        variants: &[0x4211, 0x4221, 0x4231, 0x4241],
        rel_n1: &[1, 2],
        manual_ranges: 0,
    },
    Family {
        base: 0x4300,
        variants: &[0x4311, 0x4321, 0x4331, 0x4341],
        rel_n1: &[1, 2],
        manual_ranges: 0,
    },
    Family {
        base: 0x5100,
        variants: &[0x5111],
        rel_n1: &[1],
        manual_ranges: 6,
    },
    // Continuity: nibble 0 = 2 is the open-circuit beeper, not REL, so the
    // pair is two choices and the family has no REL toggle at all.
    Family {
        base: 0x5200,
        variants: &[0x5211, 0x5212],
        rel_n1: &[],
        manual_ranges: 0,
    },
    // nS: auto only. Its Range combo is `Visible = False` in the vendor form
    // and holds a copy of the Ohm item list — dead UI, not a range ladder
    // (research spec §7.1 "Families with no manual range").
    Family {
        base: 0x5300,
        variants: &[0x5311],
        rel_n1: &[1],
        manual_ranges: 0,
    },
    // Diode: same shape as continuity — nibble 0 = 2 is the alarm function.
    Family {
        base: 0x6100,
        variants: &[0x6111, 0x6112],
        rel_n1: &[],
        manual_ranges: 0,
    },
    Family {
        base: 0x6200,
        variants: &[0x6211],
        rel_n1: &[1],
        manual_ranges: 8,
    },
    Family {
        base: 0x7100,
        variants: &[0x7111],
        rel_n1: &[1],
        manual_ranges: 7,
    },
    Family {
        base: 0x7200,
        variants: &[0x7211],
        rel_n1: &[1],
        manual_ranges: 4,
    },
    Family {
        base: 0x7300,
        variants: &[0x7311],
        rel_n1: &[1],
        manual_ranges: 4,
    },
    // Currents. DC sub-function: plain, AC+DC, Peak — REL on the first two.
    // AC sub-function: plain, Hz, Peak — REL on the plain variant only.
    // The 10 A jack has a single fixed range.
    Family {
        base: 0x8100,
        variants: &[0x8111, 0x8121, 0x8131],
        rel_n1: &[1, 2],
        manual_ranges: 2,
    },
    Family {
        base: 0x8200,
        variants: &[0x8211, 0x8221, 0x8231],
        rel_n1: &[1],
        manual_ranges: 2,
    },
    Family {
        base: 0x9100,
        variants: &[0x9111, 0x9121, 0x9131],
        rel_n1: &[1, 2],
        manual_ranges: 2,
    },
    Family {
        base: 0x9200,
        variants: &[0x9211, 0x9221, 0x9231],
        rel_n1: &[1],
        manual_ranges: 2,
    },
    Family {
        base: 0xA100,
        variants: &[0xA111, 0xA121, 0xA131],
        rel_n1: &[1, 2],
        manual_ranges: 0,
    },
    Family {
        base: 0xA200,
        variants: &[0xA211, 0xA221, 0xA231],
        rel_n1: &[1],
        manual_ranges: 0,
    },
];

/// The dial family a mode word belongs to.
pub(crate) fn family(word: u16) -> u16 {
    word & FAMILY_MASK
}

fn lookup(word: u16) -> Option<&'static Family> {
    let base = family(word);
    FAMILIES.iter().find(|f| f.base == base)
}

/// The primary-variant nibble of a mode word.
fn variant_nibble(word: u16) -> u16 {
    (word >> 4) & 0xF
}

/// The REL companion of a mode word: nibble 0 flipped 1 <-> 2.
fn rel_partner(word: u16) -> u16 {
    word ^ 0x3
}

/// `word` with REL taken off: the variant a REL word is relative to, or
/// `word` itself. Only a REL companion is mapped, so the continuity
/// open-circuit beeper (0x5212) and the diode alarm (0x6112), whose nibble 0
/// = 2 is a function of its own, stay what they are.
pub(crate) fn plain_word(word: u16) -> u16 {
    if word & 0xF == N0_REL && rel_supported(word) {
        rel_partner(word)
    } else {
        word
    }
}

impl Family {
    /// Whether this variant of the family can be put into REL.
    fn rel_capable(&self, word: u16) -> bool {
        self.rel_n1.contains(&variant_nibble(word))
    }
}

/// Whether `word` is one of the 79 mode words of research spec §6: a
/// family's variant, or the REL companion of a variant that offers REL
/// (§6.1, "REL for n1").
pub(crate) fn is_known_word(word: u16) -> bool {
    let Some(f) = lookup(word) else {
        return false;
    };
    f.variants.contains(&word)
        || (word & 0xF == N0_REL && f.rel_capable(word) && f.variants.contains(&rel_partner(word)))
}

/// The longest range ladder of research spec §7 (capacitance).
const MAX_RANGE: u8 = 8;

/// Whether a measurement in `word` can carry range byte `range`.
///
/// Research spec §5.1 and §7: 0 is auto and 1-8 index the family's manual
/// ladder, whose length §7.1 gives per dial family.
pub(crate) fn is_known_range(word: u16, range: u8) -> bool {
    if range == 0 {
        return true;
    }
    if range > MAX_RANGE {
        return false;
    }
    let Some(f) = lookup(word) else {
        // No ladder to hold the byte against: the word is what is unknown.
        return true;
    };
    if f.manual_ranges == 0 {
        // §7.1 "Families with no manual range". A real temperature frame
        // carries range 1, so that is what a fixed range reports.
        range == 1
    } else if lookup_range_label(word, 1).is_empty() {
        // Duty and pulse width: §7.1 lists four unnamed items, and nothing
        // says which bytes the meter reports for them.
        true
    } else {
        range <= f.manual_ranges
    }
}

/// Every word [`is_known_word`] accepts, variants first, for the tests that
/// walk them all.
#[cfg(test)]
pub(crate) fn known_words() -> Vec<u16> {
    let variants = FAMILIES.iter().flat_map(|f| f.variants.iter().copied());
    let rel = FAMILIES.iter().flat_map(|f| {
        f.variants
            .iter()
            .filter(|&&w| f.rel_capable(w))
            .map(|&w| rel_partner(w))
    });
    variants.chain(rel).collect()
}

/// Modes reachable from `current_mode_raw` without moving the dial.
///
/// Empty for a mode word from no known family. REL words are not listed as
/// choices of their own: REL is a toggle on top of a variant
/// (`rel_supported`), so a meter sitting in `0x1112` reports `0x1111` — V AC —
/// as its current choice, and one in `0x1142` reports `0x1141`, V AC LPF.
pub(crate) fn mode_choices(current_mode_raw: u16) -> Vec<Choice> {
    let Some(f) = lookup(current_mode_raw) else {
        return Vec::new();
    };
    f.variants
        .iter()
        .map(|&word| {
            let rel_active = f.rel_capable(word) && rel_partner(word) == current_mode_raw;
            Choice {
                id: word,
                label: decode_mode_word(word),
                current: word == current_mode_raw || rel_active,
            }
        })
        .collect()
}

/// The ranges reachable in `word`, for `Protocol::choices`.
///
/// Auto first, then the family's manual ladder — which is what SET_RANGE
/// indexes, so the choice id *is* the byte the command takes. Empty for a
/// fixed-range family (A DC/AC, temperature, continuity, conductance, diode),
/// for a word from no known family, and for a ladder whose rungs have no
/// label to offer them by: the vendor app lists four items for duty cycle
/// and pulse width, but nothing says what the meter calls them, and a blank
/// entry cannot be picked or confirmed.
pub(crate) fn range_choices(word: u16, range_raw: u8, auto_range: bool) -> Vec<Choice> {
    let Some(f) = lookup(word) else {
        return Vec::new();
    };
    if f.manual_ranges == 0 || (1..=f.manual_ranges).any(|r| lookup_range_label(word, r).is_empty())
    {
        return Vec::new();
    }
    let mut choices = vec![Choice {
        id: AUTO_RANGE_ID,
        label: Cow::Borrowed(AUTO_RANGE_LABEL),
        // The meter says so twice — the auto-range flag, and range byte 0.
        current: auto_range || range_raw == 0,
    }];
    choices.extend((1..=f.manual_ranges).map(|rung| Choice {
        id: u16::from(rung),
        label: Cow::Borrowed(lookup_range_label(word, rung)),
        current: !auto_range && range_raw == rung,
    }));
    choices
}

/// Whether REL can be toggled from `word`.
///
/// The vendor app gates REL per primary variant, not per family: each primary
/// radio's click handler enables or disables the REL secondary radio, and it
/// is disabled on every Hz variant, every Peak variant and the differential
/// temperature arrangements (research spec §6.1, "REL for n1"). Continuity and
/// diode have no REL at all — their nibble 0 = 2 is the open-beeper and alarm
/// function. Accepts both nibble-0 values so REL can be switched back off.
pub(crate) fn rel_supported(word: u16) -> bool {
    let Some(f) = lookup(word) else {
        return false;
    };
    f.rel_capable(word) && matches!(word & 0xF, N0_PLAIN | N0_REL)
}

/// The mode word that puts `word`'s function into REL (`on`) or takes it
/// back out, or `None` when this variant has no REL companion.
///
/// REL is not its own opcode on this meter: it is SET_MODE with nibble 0
/// switched between 1 (plain) and 2 (relative), research spec §6.1. That
/// makes it absolute — the same command reaches the same state whatever the
/// meter was doing.
pub(crate) fn rel_word(word: u16, on: bool) -> Option<u16> {
    if !rel_supported(word) {
        return None;
    }
    Some((word & !0xF) | if on { N0_REL } else { N0_PLAIN })
}

/// The next manual range after `last_range` (0 = auto), or `None` when the
/// family has no manual ladder.
///
/// Steps 0 -> 1 -> ... -> n -> 1: the wrap goes back to the first manual
/// range, not to auto. Auto is its own command, as it is its own button.
pub(crate) fn next_manual_range(word: u16, last_range: u8) -> Option<u8> {
    let f = lookup(word)?;
    if f.manual_ranges == 0 {
        return None;
    }
    Some(if last_range >= f.manual_ranges {
        1
    } else {
        last_range + 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is indexed by `base`, so a duplicate would silently shadow
    /// a family, and a `base` with low bits set could never be matched.
    #[test]
    fn family_bases_are_unique_and_masked() {
        let mut seen: Vec<u16> = Vec::new();
        for f in FAMILIES {
            assert_eq!(f.base, family(f.base), "{:#06x} has low bits set", f.base);
            assert!(!seen.contains(&f.base), "{:#06x} listed twice", f.base);
            seen.push(f.base);
        }
    }

    /// Every variant must belong to the family that lists it, and must decode
    /// to a real name — a typo'd word would otherwise reach the UI as
    /// "Unknown(0x….)" and be sent to the meter anyway.
    #[test]
    fn variants_belong_to_their_family_and_have_labels() {
        for f in FAMILIES {
            assert!(!f.variants.is_empty(), "{:#06x} has no variants", f.base);
            for &word in f.variants {
                assert_eq!(
                    family(word),
                    f.base,
                    "{word:#06x} is not in {:#06x}",
                    f.base
                );
                let label = decode_mode_word(word);
                assert!(!label.starts_with("Unknown"), "{word:#06x} -> {label}");
            }
        }
    }

    /// A `rel_n1` entry naming a variant the family doesn't have would enable
    /// REL on a word the meter can never be in.
    #[test]
    fn rel_n1_entries_name_variants_the_family_has() {
        for f in FAMILIES {
            for &n1 in f.rel_n1 {
                assert!(
                    f.variants.iter().any(|&w| variant_nibble(w) == n1),
                    "{:#06x} allows REL on n1={n1} but has no such variant",
                    f.base
                );
            }
        }
    }

    /// Research spec §6 counts 79 mode words; `is_known_word` accepts those
    /// and nothing else.
    #[test]
    fn exactly_the_79_spec_words_are_known() {
        let mut words = known_words();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), 79);
        let accepted: Vec<u16> = (0..=u16::MAX).filter(|&w| is_known_word(w)).collect();
        assert_eq!(accepted, words);
        // The vendor UI's mV AC+DC is in (§6.1 "Caveats"); the alternative
        // mV DC Peak code is not (§6.1, "Agreement with the community table").
        assert!(is_known_word(0x2141));
        assert!(is_known_word(0x2142));
        assert!(!is_known_word(0x4131));
        // No REL on a Hz variant, no n0 = 3, no REL on continuity.
        for word in [0x1122, 0x1113, 0x5213, 0x6113] {
            assert!(!is_known_word(word), "{word:#06x}");
        }
    }

    #[test]
    fn mode_choices_lists_the_vac_variants() {
        let choices = mode_choices(0x1111);
        let ids: Vec<u16> = choices.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![0x1111, 0x1121, 0x1131, 0x1141, 0x1151, 0x1161]);
        assert_eq!(choices[0].label, "V AC");
        assert_eq!(choices[1].label, "V AC Hz");
        assert_eq!(choices[5].label, "V AC dBm");
    }

    #[test]
    fn mode_choices_are_empty_for_an_unknown_family() {
        assert!(mode_choices(0xFFFF).is_empty());
        assert!(mode_choices(0x0000).is_empty());
    }

    /// REL is a modifier, not a choice, so the choice list has to point at the
    /// variant underneath whichever REL word the meter is reporting.
    #[test]
    fn mode_choices_flag_the_variant_behind_an_active_rel() {
        for (rel_word, expected) in [
            (0x1112, 0x1111), // V AC REL
            (0x1142, 0x1141), // V AC LPF REL
            (0x1162, 0x1161), // V AC dBm REL
            (0x3122, 0x3121), // V DC AC+DC REL
            (0x4222, 0x4221), // °C T2 REL
        ] {
            let current: Vec<u16> = mode_choices(rel_word)
                .into_iter()
                .filter(|c| c.current)
                .map(|c| c.id)
                .collect();
            assert_eq!(current, vec![expected], "from {rel_word:#06x}");
        }
    }

    #[test]
    fn next_manual_range_wraps_to_the_first_manual_range() {
        // V DC: 4 ranges.
        assert_eq!(next_manual_range(0x3111, 0), Some(1));
        assert_eq!(next_manual_range(0x3111, 3), Some(4));
        assert_eq!(next_manual_range(0x3111, 4), Some(1));
        // Capacitance has the longest ladder.
        assert_eq!(next_manual_range(0x6211, 7), Some(8));
        assert_eq!(next_manual_range(0x6211, 8), Some(1));
    }

    #[test]
    fn next_manual_range_is_none_without_a_ladder() {
        for word in [0x4211, 0x4311, 0x5211, 0x5311, 0x6111, 0xA111, 0xA211] {
            assert_eq!(next_manual_range(word, 0), None, "{word:#06x}");
        }
        assert_eq!(next_manual_range(0xFFFF, 0), None);
    }

    /// The per-variant gating of research spec §6.1: REL is offered on the
    /// plain, AC+DC, LowPass, dBV, dBm and T1,T2 / T2,T1 variants, and
    /// withheld everywhere else.
    #[test]
    fn rel_follows_the_vendor_per_variant_gating() {
        for word in [
            0x1111, 0x1141, 0x1151, 0x1161, // V AC plain, LowPass, dBV, dBm
            0x2111, 0x2141, // mV AC plain, AC+DC
            0x3111, 0x3121, // V DC plain, AC+DC
            0x4111, // mV DC plain
            0x4211, 0x4221, 0x4311, 0x4321, // temperature T1,T2 and T2,T1
            0x5111, 0x5311, 0x6211, 0x7111, 0x7211, 0x7311, // single-variant
            0x8111, 0x8121, 0x9111, 0x9121, 0xA111, 0xA121, // DC currents
            0x8211, 0x9211, 0xA211, // AC currents, plain only
        ] {
            assert!(rel_supported(word), "{word:#06x} should offer REL");
        }

        for word in [
            0x1121, 0x1131, // V AC Hz, Peak
            0x2121, 0x2131, // mV AC Hz, Peak
            0x3131, 0x4121, // V DC Peak, mV DC Peak
            0x4231, 0x4241, 0x4331, 0x4341, // differential temperature
            0x5211, 0x5212, 0x6111, 0x6112, // continuity and diode
            0x8131, 0x9131, 0xA131, // DC current Peak
            0x8221, 0x8231, 0x9221, 0x9231, 0xA221, 0xA231, // AC current Hz/Peak
            0xFFFF, // unknown family
        ] {
            assert!(!rel_supported(word), "{word:#06x} should not offer REL");
        }
    }

    #[test]
    fn plain_word_takes_rel_off_and_nothing_else() {
        for (word, plain) in [
            (0x1112, 0x1111), // V AC REL
            (0x1142, 0x1141), // V AC LPF REL
            (0x4222, 0x4221), // °C T2 REL
            (0x1111, 0x1111),
            (0x5212, 0x5212), // continuity open, not a REL
            (0x6112, 0x6112), // diode alarm, not a REL
        ] {
            assert_eq!(plain_word(word), plain, "{word:#06x}");
        }
    }

    /// Toggling REL off has to be possible from every word toggling it on can
    /// produce, or a meter could be left stuck in REL.
    #[test]
    fn rel_round_trips_from_every_rel_capable_variant() {
        for f in FAMILIES {
            for &word in f.variants {
                if !rel_supported(word) {
                    continue;
                }
                let on = rel_partner(word);
                assert_eq!(on & 0xF, N0_REL, "{word:#06x} -> {on:#06x}");
                assert!(rel_supported(on), "{on:#06x} cannot toggle back off");
                assert_eq!(rel_partner(on), word);
            }
        }
    }
}
