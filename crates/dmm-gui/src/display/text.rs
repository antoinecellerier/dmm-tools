//! The reading as text: the digits drawn at a stable width, and the one-line
//! description a screen reader announces, with the fingerprint that says when
//! it has to be rebuilt.

use super::ReadingState;
use dmm_lib::alarm::Zone;
use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use std::borrow::Cow;

/// Format the meter's raw 7-char display string for stable rendering.
///
/// Right-aligns to the meter's own 7-character display width, so the reading
/// keeps a constant width as digits and the minus sign come and go — the
/// jitter `.claude/rules/gui.md` is guarding against with "display value
/// strings use `display_raw` for stable width".
///
/// Ordinary spaces suffice: every caller draws this with
/// `FontId::monospace`, where a space is already digit-width. (This comment
/// previously claimed a figure-space (U+2007) substitution, which the body
/// has never done and which would only matter in a proportional font.)
fn format_display_raw(raw: &str) -> String {
    let trimmed = raw.trim_end();
    format!("{trimmed:>7}")
}

/// Format the measurement value as a display string.
/// Uses the meter's raw 7-char display when available (UT61E+ protocol),
/// otherwise formats the numeric value for float-based protocols.
///
/// The parsed `MeasuredValue` decides first: several protocols flag overload
/// through a status bit while still sending ordinary digits in the display
/// field (UT8802 `sign_byte & 0x40`, UT8803 `payload[12] & 0x04`). Preferring
/// `display_raw` there would render an out-of-range reading as a plausible
/// number. `display_raw` still wins for normal readings, which is what keeps
/// the on-screen width steady.
pub(super) fn format_value_display(m: &Measurement) -> String {
    match &m.value {
        MeasuredValue::Normal(v) => match m.display_raw.as_deref() {
            Some(raw) => format_display_raw(raw),
            None => format!("{v:>7}"),
        },
        MeasuredValue::Overload => format!("{:>7}", "OL"),
        MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
        // Where the digits would be, like OL: never "OL", which would claim
        // an overload the meter is not showing.
        MeasuredValue::NoReading(word) => format!("{word:>7}"),
        // No main reading in this frame: blank digits at the usual width,
        // its sub-values in the rows below.
        MeasuredValue::Absent => format!("{:>7}", ""),
    }
}

/// Format a measurement as a spoken-friendly one-line description for screen
/// readers. Used as the live-region label on the primary reading. Uses the
/// same value formatting as the visible display so AT users hear exactly
/// what sighted users see.
///
/// `no_reading` is what the placeholder says when there is no measurement —
/// [`super::NO_READING_TITLE`], or the connection issue the big meter puts in the
/// readout instead, so AT hears the problem and not a bare "No reading". It
/// is ignored when a measurement is given.
pub(super) fn live_region_label(
    measurement: Option<&Measurement>,
    state: ReadingState,
    no_reading: &str,
) -> String {
    match measurement {
        Some(m) => {
            let value = match &m.value {
                MeasuredValue::Overload => "overload".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV level {l}"),
                MeasuredValue::NoReading(word) => spoken_no_reading(word),
                MeasuredValue::Normal(_) => format_value_display(m).trim().to_string(),
                // Nothing to say before the mode: the sub-values carry it.
                MeasuredValue::Absent => String::new(),
            };
            let mut parts = String::with_capacity(96);
            // Spoken where it is drawn, ahead of the digits — and, as there,
            // not for a frame without its main reading.
            if let Some(label) = m.main_label.filter(|_| m.has_main_reading()) {
                parts.push_str(label.as_str());
                parts.push(' ');
            }
            if !value.is_empty() {
                parts.push_str(&value);
                if !m.unit.is_empty() {
                    parts.push(' ');
                    parts.push_str(&spoken_unit(&m.unit));
                }
            }
            // "Auto" with no function lit is both the word and the mode;
            // saying it twice reads as two things.
            let mode_is_the_word =
                matches!(m.value, MeasuredValue::NoReading(word) if word == m.mode);
            if !m.mode.is_empty() && !mode_is_the_word {
                if !parts.is_empty() {
                    parts.push_str(", ");
                }
                parts.push_str(&m.mode);
            }
            // Sub-values sit between the mode and the flags. Whether the
            // rows are drawn under the reading or beside it depends on the
            // window, so the spoken order follows neither; it stays put
            // while the layout moves. Without them a UT181A user in MIN/MAX
            // hears only the live value and never the extremes the meter is
            // actually displaying.
            for aux in m.present_aux() {
                parts.push_str(", ");
                parts.push_str(&aux.label);
                parts.push(' ');
                parts.push_str(&spoken_aux_value(aux));
                let unit = spoken_unit(aux.unit_or(&m.unit));
                if !unit.is_empty() {
                    parts.push(' ');
                    parts.push_str(&unit);
                }
                // The visible row ends in "@12s"; spelling it out is the only
                // way an AT user learns *when* a MIN/MAX extreme was caught,
                // which is half of what those readings mean.
                if let Some(secs) = aux.elapsed_secs {
                    parts.push_str(" at ");
                    parts.push_str(&secs.to_string());
                    parts.push_str(if secs == 1 { " second" } else { " seconds" });
                }
            }
            // Speak the same status flags that the visible badge row shows.
            // Without this, a screen reader user toggling HOLD/REL/MIN/MAX/
            // AUTO via the on-device buttons hears the value change but no
            // confirmation that the mode actually flipped.
            append_flags_phrase(&mut parts, &m.flags);
            // Last, after the flags, so it reads as one more badge — which is
            // exactly where the SCALE badge sits on screen. Without it a
            // screen-reader user has no way to tell a software-scaled reading
            // from one the meter produced.
            if state.scaled {
                parts.push_str(", software scaled");
            }
            // The alarm's verdict likewise, as the HI LIMIT / LO LIMIT badge.
            match state.alarm {
                Some(Zone::Above) => parts.push_str(", above the high limit"),
                Some(Zone::Below) => parts.push_str(", below the low limit"),
                Some(Zone::Inside) | None => {}
            }
            parts
        }
        None => no_reading.to_string(),
    }
}

/// Spoken form of a sub-value: the parsed value decides, so an overloaded
/// sub-value is announced as "overload" rather than the letters "O L".
fn spoken_aux_value(aux: &AuxValue) -> Cow<'_, str> {
    match &aux.value {
        MeasuredValue::Overload => Cow::Borrowed("overload"),
        MeasuredValue::NoReading(word) => Cow::Owned(spoken_no_reading(word)),
        _ => aux.value_str(),
    }
}

/// Spoken form of a word the meter shows instead of a reading: said to be no
/// reading first, since "----" alone reads as four dashes or as nothing.
fn spoken_no_reading(word: &str) -> String {
    format!("no reading ({word})")
}

/// Spoken form of a unit string.
///
/// Only the degree symbol is rewritten: screen readers differ on whether they
/// read "°" at all, so a temperature sub-value could otherwise be announced as
/// a bare "24.1 C". The substitution is confined to the spoken label — the
/// visible rows keep the symbol.
fn spoken_unit(unit: &str) -> Cow<'_, str> {
    match unit {
        "\u{00B0}C" => Cow::Borrowed("degrees C"),
        "\u{00B0}F" => Cow::Borrowed("degrees F"),
        other => Cow::Borrowed(other),
    }
}

/// The single order the flags are presented in: `show_flags` paints the
/// badges in it and `append_flags_phrase` speaks them in it, so the two cannot
/// drift apart the way three hand-written lists did.
///
/// `Flag::HvWarning` leads — see the hazard-first comment in `show_flags`.
/// Everything else keeps [`Flag::ALL`] order, which is what the recording
/// panel and the CSV flags column already use.
pub(super) fn badge_order() -> impl Iterator<Item = Flag> {
    std::iter::once(Flag::HvWarning).chain(Flag::ALL.into_iter().filter(|f| *f != Flag::HvWarning))
}

/// Spoken form of a flag for the screen-reader label.
///
/// `None` where [`Flag::label`] is `None`: the DC/AC distinction rides on the
/// mode field, so announcing it again would be noise.
fn spoken(flag: Flag) -> Option<&'static str> {
    Some(match flag {
        Flag::HvWarning => "high voltage warning",
        Flag::AutoRange => "auto range",
        Flag::Hold => "hold",
        Flag::Rel => "relative",
        Flag::Min => "minimum",
        Flag::Max => "maximum",
        Flag::Avg => "average",
        Flag::PeakMin => "peak minimum",
        Flag::PeakMax => "peak maximum",
        Flag::LowBattery => "low battery",
        Flag::LeadError => "lead error",
        Flag::Comp => "compare",
        Flag::Record => "recording",
        Flag::LoZ => "low impedance",
        Flag::Void => "void",
        Flag::Dc => return None,
    })
}

/// Append a phrase listing the active status flags, in `badge_order()` — the
/// same iterator `show_flags` paints from, so what is heard and what is seen
/// are the same flags in the same order. Each flag is prefixed with ", " so it
/// reads naturally after the mode field. No-op if all flags are inactive.
fn append_flags_phrase(out: &mut String, flags: &StatusFlags) {
    for phrase in badge_order().filter(|f| flags.get(*f)).filter_map(spoken) {
        out.push_str(", ");
        out.push_str(phrase);
    }
}

/// Pack a `StatusFlags` into a u16 bitfield for fingerprint hashing. Bit `i`
/// is `Flag::ALL[i]`, so the packing stays stable and stays complete as flags
/// are added rather than relying on struct field order.
fn flags_bits(flags: &StatusFlags) -> u16 {
    // A seventeenth flag would shift straight out of the u16 and silently
    // leave the fingerprint, so widen the packing before adding one.
    const _: () = assert!(StatusFlags::COUNT <= u16::BITS as usize);
    Flag::ALL.iter().enumerate().fold(0u16, |bits, (i, &flag)| {
        bits | ((flags.get(flag) as u16) << i)
    })
}

/// Build a u64 fingerprint that changes whenever `live_region_label` would
/// produce different output. Lets `set_live_region_cached` skip per-frame
/// `format!`/`String` allocation when the measurement is unchanged.
pub(super) fn live_region_fingerprint(
    measurement: Option<&Measurement>,
    state: ReadingState,
    no_reading: &str,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // Toggling the transform, or the reading crossing a limit, changes the
    // spoken label without necessarily changing the reading, so both have to
    // be in the fingerprint or the cached announcement would never be rebuilt.
    state.hash(&mut h);
    match measurement {
        None => {
            0u8.hash(&mut h);
            // The placeholder's whole label: one connection issue replacing
            // another has to be announced, not swallowed by a fingerprint
            // that only ever saw "No reading".
            no_reading.hash(&mut h);
        }
        Some(m) => {
            1u8.hash(&mut h);
            match &m.value {
                MeasuredValue::Normal(v) => {
                    0u8.hash(&mut h);
                    v.to_bits().hash(&mut h);
                    // display_raw is what we actually format for Normal values,
                    // so include it so the fingerprint catches stable-string
                    // changes that don't show up in the f64 bits.
                    m.display_raw.as_deref().unwrap_or("").hash(&mut h);
                }
                MeasuredValue::Overload => 1u8.hash(&mut h),
                MeasuredValue::NcvLevel(l) => {
                    2u8.hash(&mut h);
                    l.hash(&mut h);
                }
                MeasuredValue::NoReading(word) => {
                    3u8.hash(&mut h);
                    word.hash(&mut h);
                }
                MeasuredValue::Absent => 4u8.hash(&mut h),
            }
            m.unit.hash(&mut h);
            m.mode.hash(&mut h);
            m.main_label.hash(&mut h);
            // Sub-values are part of both the spoken label and the visible
            // rows, so a MIN/MAX extreme moving (or its timestamp advancing)
            // has to invalidate the cached announcement even though the live
            // value may be unchanged.
            m.aux_values.len().hash(&mut h);
            for aux in &m.aux_values {
                aux.label.hash(&mut h);
                match &aux.value {
                    MeasuredValue::Normal(v) => {
                        0u8.hash(&mut h);
                        v.to_bits().hash(&mut h);
                        aux.display_raw.as_deref().unwrap_or("").hash(&mut h);
                    }
                    MeasuredValue::Overload => 1u8.hash(&mut h),
                    MeasuredValue::NcvLevel(l) => {
                        2u8.hash(&mut h);
                        l.hash(&mut h);
                    }
                    MeasuredValue::NoReading(word) => {
                        3u8.hash(&mut h);
                        word.hash(&mut h);
                    }
                    MeasuredValue::Absent => 4u8.hash(&mut h),
                }
                aux.unit.hash(&mut h);
                aux.elapsed_secs.hash(&mut h);
            }
            flags_bits(&m.flags).hash(&mut h);
        }
    }
    h.finish()
}

/// Right-align a sub-value to the primary reading's 7-character width.
///
/// Same reasoning as [`format_display_raw`]: every caller draws this with
/// `FontId::monospace`, so a fixed width keeps the digits from shifting
/// sideways between frames and lines the sub-value rows up with each other.
pub(super) fn format_aux_value(aux: &AuxValue) -> String {
    format!("{:>7}", aux.value_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::NO_READING_TITLE;
    use crate::display::tests::aux;

    #[test]
    fn format_display_raw_normal() {
        assert_eq!(format_display_raw("  5.678"), "  5.678");
    }

    #[test]
    fn format_display_raw_negative_with_space() {
        // "- 55.79" should be right-aligned to 7 chars
        assert_eq!(format_display_raw("- 55.79"), "- 55.79");
    }

    #[test]
    fn format_display_raw_short_value() {
        // Short values get padded to 7 chars
        assert_eq!(format_display_raw("OL"), "     OL");
    }

    #[test]
    fn format_display_raw_trailing_spaces_trimmed() {
        // Trailing spaces trimmed before alignment
        // "1.23  " → trim_end → "1.23" (4 chars) → right-align to 7
        assert_eq!(format_display_raw("1.23  "), "   1.23");
    }

    #[test]
    fn format_display_raw_full_width() {
        assert_eq!(format_display_raw("-12.345"), "-12.345");
    }

    #[test]
    fn format_display_raw_empty() {
        assert_eq!(format_display_raw(""), "       ");
    }

    /// UT8802/UT8803 flag overload through a status bit while still sending
    /// ordinary digits in the display field. Rendering those digits shows an
    /// out-of-range input as a plausible reading (a bare `0` in Ω mode is
    /// indistinguishable from a real short).
    #[test]
    fn overload_beats_display_raw_digits() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        m.display_raw = Some("    0".to_string());
        assert_eq!(format_value_display(&m).trim(), "OL");
    }

    #[test]
    fn ncv_level_beats_display_raw_digits() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        m.display_raw = Some("  1.234".to_string());
        assert_eq!(format_value_display(&m).trim(), "NCV 3");
    }

    /// Normal readings must still take the meter's own digits — that is what
    /// holds the on-screen width steady between frames.
    #[test]
    fn normal_still_prefers_display_raw() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        assert_eq!(format_value_display(&m), "  5.678");
    }

    /// The visible reading and the spoken label must agree — this pair was
    /// what made the bug user-visible in the first place.
    #[test]
    fn overload_reads_the_same_visibly_and_aloud() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        m.display_raw = Some("    0".to_string());
        assert_eq!(format_value_display(&m).trim(), "OL");
        assert!(
            live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE)
                .starts_with("overload")
        );
    }

    /// A word the meter shows instead of a reading sits where the digits do,
    /// right-aligned like OL, and is never shown or spoken as an overload.
    #[test]
    fn a_no_reading_shows_its_word_not_ol() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::NoReading("Auto"), "", StatusFlags::default());
        m.mode = "Auto".into();
        m.display_raw = Some("   Auto".to_string());
        assert_eq!(format_value_display(&m), "   Auto");
        assert_eq!(
            live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            "no reading (Auto)"
        );
        let mut in_a_mode = m.clone();
        in_a_mode.mode = "DC V".into();
        assert_eq!(
            live_region_label(Some(&in_a_mode), ReadingState::PLAIN, NO_READING_TITLE),
            "no reading (Auto), DC V"
        );

        let mut over = m.clone();
        over.value = MeasuredValue::Overload;
        assert_ne!(
            live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            live_region_fingerprint(Some(&over), ReadingState::PLAIN, NO_READING_TITLE)
        );
        let mut dashes = m.clone();
        dashes.value = MeasuredValue::NoReading("----");
        assert_ne!(
            live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            live_region_fingerprint(Some(&dashes), ReadingState::PLAIN, NO_READING_TITLE)
        );
    }

    #[test]
    fn live_region_label_includes_active_flags() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                auto_range: true,
                ..Default::default()
            },
        );
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(label.contains("V"), "got {label:?}");
        assert!(label.contains("DC V"), "got {label:?}");
        assert!(label.contains("auto range"), "got {label:?}");
        assert!(label.contains("hold"), "got {label:?}");
    }

    /// The meter's high-voltage indicator is a safety signal — a screen
    /// reader user must hear it, and hear it before the routine flags.
    #[test]
    fn live_region_label_announces_high_voltage_first() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(400.0),
            "V",
            StatusFlags {
                hv_warning: true,
                auto_range: true,
                ..Default::default()
            },
        );
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        let hv = label.find("high voltage").expect("HV must be announced");
        let auto = label
            .find("auto range")
            .expect("auto range still announced");
        assert!(hv < auto, "HV must come first, got {label:?}");
    }

    /// The big meter puts the connection issue where the reading goes, so the
    /// live region has to speak it: a screen-reader user would otherwise hear
    /// "No reading" and never learn that the meter had stopped answering.
    #[test]
    fn the_placeholder_label_carries_the_connection_issue() {
        assert_eq!(
            live_region_label(None, ReadingState::PLAIN, "No response from meter"),
            "No response from meter"
        );
        assert_eq!(
            live_region_label(None, ReadingState::PLAIN, NO_READING_TITLE),
            NO_READING_TITLE
        );
        assert_ne!(
            live_region_fingerprint(None, ReadingState::PLAIN, "No response from meter"),
            live_region_fingerprint(None, ReadingState::PLAIN, NO_READING_TITLE),
            "one placeholder replacing another has to be re-announced"
        );
    }

    #[test]
    fn live_region_label_no_flags_when_inactive() {
        let m = Measurement::test_fixture(MeasuredValue::Normal(0.0), "V", StatusFlags::default());
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        // StatusFlags::default() is all-false, so no flag phrases should
        // appear in the spoken label.
        assert!(!label.contains("hold"), "got {label:?}");
        assert!(!label.contains("relative"), "got {label:?}");
        assert!(!label.contains("auto range"), "got {label:?}");
    }

    #[test]
    fn live_region_fingerprint_changes_on_flag_toggle() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let fp1 = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        m.flags.hold = true;
        let fp2 = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_ne!(fp1, fp2, "toggling HOLD must change the fingerprint");
        m.flags.hold = false;
        m.flags.rel = true;
        let fp3 = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_ne!(fp1, fp3, "toggling REL must change the fingerprint");
        assert_ne!(fp2, fp3, "REL and HOLD must produce distinct fingerprints");
    }

    #[test]
    fn flags_bits_distinct_per_flag() {
        // Each flag must occupy a distinct bit so toggling any one of them
        // changes the packed u16. Catches accidental bit collisions.
        let names = [
            (
                "hold",
                StatusFlags {
                    hold: true,
                    ..Default::default()
                },
            ),
            (
                "rel",
                StatusFlags {
                    rel: true,
                    ..Default::default()
                },
            ),
            (
                "min",
                StatusFlags {
                    min: true,
                    ..Default::default()
                },
            ),
            (
                "max",
                StatusFlags {
                    max: true,
                    ..Default::default()
                },
            ),
            (
                "auto_range",
                StatusFlags {
                    auto_range: true,
                    ..Default::default()
                },
            ),
            (
                "low_battery",
                StatusFlags {
                    low_battery: true,
                    ..Default::default()
                },
            ),
            (
                "hv_warning",
                StatusFlags {
                    hv_warning: true,
                    ..Default::default()
                },
            ),
            (
                "dc",
                StatusFlags {
                    dc: true,
                    ..Default::default()
                },
            ),
            (
                "peak_max",
                StatusFlags {
                    peak_max: true,
                    ..Default::default()
                },
            ),
            (
                "peak_min",
                StatusFlags {
                    peak_min: true,
                    ..Default::default()
                },
            ),
            (
                "lead_error",
                StatusFlags {
                    lead_error: true,
                    ..Default::default()
                },
            ),
            (
                "comp",
                StatusFlags {
                    comp: true,
                    ..Default::default()
                },
            ),
            (
                "record",
                StatusFlags {
                    record: true,
                    ..Default::default()
                },
            ),
            (
                "avg",
                StatusFlags {
                    avg: true,
                    ..Default::default()
                },
            ),
            (
                "loz",
                StatusFlags {
                    loz: true,
                    ..Default::default()
                },
            ),
            (
                "void",
                StatusFlags {
                    void: true,
                    ..Default::default()
                },
            ),
        ];
        let mut seen = std::collections::HashSet::new();
        for (name, flags) in &names {
            let bits = flags_bits(flags);
            assert!(
                bits.count_ones() == 1,
                "{name} should set exactly one bit, got {bits:#b}"
            );
            assert!(seen.insert(bits), "{name} collides with another flag bit");
        }
    }

    #[test]
    fn badge_order_starts_with_hv_and_covers_every_flag_once() {
        let order: Vec<Flag> = badge_order().collect();
        assert_eq!(
            order.first(),
            Some(&Flag::HvWarning),
            "the hazard badge must lead the row"
        );
        assert_eq!(order.len(), StatusFlags::COUNT);

        let mut seen = std::collections::HashSet::new();
        for flag in &order {
            assert!(seen.insert(*flag), "{flag:?} appears twice in badge_order");
        }
    }

    /// The badge row and the spoken label are built from the same iterator, so
    /// anything with a badge must have a phrase and vice versa — the drift
    /// that had screen readers announcing peak flags the row never painted.
    #[test]
    fn every_labelled_flag_is_spoken() {
        for flag in Flag::ALL {
            assert_eq!(
                spoken(flag).is_some(),
                flag.label().is_some(),
                "{flag:?} must be either both labelled and spoken, or neither"
            );
        }
    }

    #[test]
    fn spoken_phrase_lists_peak_flags_the_badges_show() {
        let flags = StatusFlags {
            peak_max: true,
            peak_min: true,
            ..Default::default()
        };
        let mut phrase = String::new();
        append_flags_phrase(&mut phrase, &flags);
        assert!(phrase.contains("peak maximum"), "got {phrase:?}");
        assert!(phrase.contains("peak minimum"), "got {phrase:?}");

        let badges: Vec<&str> = badge_order()
            .filter(|f| flags.get(*f))
            .filter_map(Flag::label)
            .collect();
        assert_eq!(badges, ["P-MAX", "P-MIN"]);
    }

    /// A UT181A in V AC + Hz shows the frequency and period next to the
    /// voltage; a screen reader user has to hear them too, and hear them
    /// where they are drawn — after the mode, before the flags.
    #[test]
    fn live_region_label_lists_sub_values() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(239.22),
            "VAC",
            StatusFlags {
                auto_range: true,
                ..Default::default()
            },
        );
        m.display_raw = Some(" 239.22".to_string());
        m.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];

        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(label.contains("Frequency 50.01 Hz"), "got {label:?}");
        assert!(label.contains("Period 20.00 ms"), "got {label:?}");
        let mode = label.find("DC V").expect("mode still announced");
        let freq = label.find("Frequency").expect("sub-value announced");
        let auto = label.find("auto range").expect("flags still announced");
        assert!(mode < freq && freq < auto, "got {label:?}");
    }

    /// A frame carrying only the AC component of an AC+DC reading: blank
    /// digits at the usual width, and a spoken label that opens on the mode
    /// rather than on a lone unit, with no word for a `Raw` that has nothing.
    /// The UT61E+ names its AC+DC V reading DC beside the AC row; a screen
    /// reader hears the name where it is drawn, ahead of the digits.
    #[test]
    fn a_named_reading_is_spoken_with_its_name() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.6112), "V", StatusFlags::default());
        m.mode = "AC+DC V".into();
        m.display_raw = Some(" 1.6112".to_string());
        m.main_label = Some(dmm_lib::measurement::MainLabel::Dc);
        m.aux_values = vec![aux("AC", " 0.0123", "")];
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_eq!(label, "DC 1.6112 V, AC+DC V, AC 0.0123 V");
    }

    #[test]
    fn a_frame_without_a_main_reading_shows_blank_digits() {
        let mut m = Measurement::test_fixture(MeasuredValue::Absent, "V", StatusFlags::default());
        m.mode = "AC+DC V".into();
        m.display_raw = None;
        let mut raw = aux("Raw", "", "V");
        raw.value = MeasuredValue::Absent;
        raw.display_raw = None;
        m.aux_values = vec![aux("AC", " 0.0123", ""), raw];

        assert_eq!(format_value_display(&m), "       ");
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_eq!(label, "AC+DC V, AC 0.0123 V");

        // As the decoder sends it, named DC on both kinds of frame: with no
        // DC value there is nothing for the name to name.
        m.main_label = Some(dmm_lib::measurement::MainLabel::Dc);
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_eq!(label, "AC+DC V, AC 0.0123 V");
    }

    /// A MIN/MAX extreme is only half a reading without the moment it was
    /// caught — the visible row says "@12s", so the spoken one has to say it
    /// too. Singular for one second, since "at 1 seconds" is jarring read
    /// aloud.
    #[test]
    fn live_region_label_speaks_extreme_capture_time() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(4.9871), "V", StatusFlags::default());
        let mut max = aux("Max", "5.9010", "");
        max.elapsed_secs = Some(12);
        let mut min = aux("Min", "4.1200", "");
        min.elapsed_secs = Some(1);
        let plain = aux("Avg", "4.5000", "");
        m.aux_values = vec![max, min, plain];

        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(
            label.contains("Max 5.9010 V at 12 seconds"),
            "got {label:?}"
        );
        assert!(label.contains("Min 4.1200 V at 1 second,"), "got {label:?}");
        // A sub-value without a timestamp must not grow a phantom one.
        assert!(label.ends_with("Avg 4.5000 V"), "got {label:?}");
    }

    /// The unit falls back to the main reading's when the sub-value doesn't
    /// carry its own (MIN/MAX), and an overloaded sub-value is spoken as a
    /// word rather than as the letters "O L".
    #[test]
    fn live_region_label_speaks_aux_fallback_unit_and_overload() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(4.9871), "V", StatusFlags::default());
        let mut max = aux("Max", "5.0123", "");
        max.elapsed_secs = Some(12);
        let mut min = aux("Min", "0", "");
        min.value = MeasuredValue::Overload;
        m.aux_values = vec![max, min];

        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(label.contains("Max 5.0123 V"), "got {label:?}");
        assert!(label.contains("Min overload V"), "got {label:?}");
    }

    /// Degree symbols are spelled out only in the spoken string — screen
    /// readers differ on whether they voice "°" at all, and "24.1 C" is not
    /// a temperature. The main reading and its sub-values get the same
    /// treatment, so a dual-thermocouple reading is voiced consistently.
    #[test]
    fn live_region_label_spells_out_degrees() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(23.5),
            "\u{00B0}C",
            StatusFlags::default(),
        );
        m.display_raw = Some("   23.5".to_string());
        m.aux_values = vec![aux("T2", "24.10", "\u{00B0}C")];
        let label = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(label.starts_with("23.5 degrees C"), "got {label:?}");
        assert!(label.contains("T2 24.10 degrees C"), "got {label:?}");
        assert!(
            !label.contains('\u{00B0}'),
            "the symbol must not survive into the spoken label, got {label:?}"
        );
    }

    /// Single-display meters must be announced exactly as before sub-values
    /// existed.
    #[test]
    fn live_region_label_unchanged_without_sub_values() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                ..Default::default()
            },
        );
        assert!(m.aux_values.is_empty());
        assert_eq!(
            live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            "5.678 V, DC V, hold"
        );
    }

    /// A MIN/MAX extreme can move while the live reading is unchanged, so
    /// the cached announcement has to be invalidated by the sub-values too.
    #[test]
    fn live_region_fingerprint_changes_on_sub_value_change() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let bare = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);

        m.aux_values = vec![aux("Max", "5.0123", "")];
        let with_aux = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_ne!(bare, with_aux, "a sub-value appearing must be noticed");

        m.aux_values[0] = aux("Max", "5.0456", "");
        let moved = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_ne!(with_aux, moved, "a sub-value changing must be noticed");

        m.aux_values[0].elapsed_secs = Some(12);
        let stamped = live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert_ne!(moved, stamped, "the @Ns column changing must be noticed");

        m.aux_values[0].label = "Min".into();
        assert_ne!(
            stamped,
            live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            "a relabelled sub-value must be noticed"
        );
    }

    /// The sub-value rows are monospace, so a fixed width keeps the digits
    /// from shifting sideways as the reading changes.
    #[test]
    fn format_aux_value_pads_to_the_reading_width() {
        assert_eq!(
            format_aux_value(&aux("Frequency", "50.01", "Hz")),
            "  50.01"
        );
        let mut over = aux("Max", "0", "");
        over.value = MeasuredValue::Overload;
        assert_eq!(format_aux_value(&over), "     OL");
    }

    /// The SCALE badge is visual; the spoken label is how an AT user learns
    /// the reading has been through a software transform.
    #[test]
    fn the_live_region_label_says_when_the_reading_is_software_scaled() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(12.34), "A", StatusFlags::default());
        let plain = live_region_label(Some(&m), ReadingState::PLAIN, NO_READING_TITLE);
        assert!(!plain.contains("software scaled"), "{plain:?}");
        let scaled = live_region_label(Some(&m), ReadingState::SCALED, NO_READING_TITLE);
        assert!(scaled.ends_with(", software scaled"), "{scaled:?}");
        assert!(scaled.starts_with(&plain), "{scaled:?} vs {plain:?}");
    }

    /// The cached announcement is only rebuilt when the fingerprint moves,
    /// so toggling the transform has to move it.
    #[test]
    fn the_live_region_fingerprint_tracks_the_scaled_bit() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(12.34), "A", StatusFlags::default());
        assert_ne!(
            live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            live_region_fingerprint(Some(&m), ReadingState::SCALED, NO_READING_TITLE)
        );
    }

    /// The HI LIMIT badge is visual too, so a breach is spoken, and moves
    /// the fingerprint so the announcement is rebuilt.
    #[test]
    fn the_live_region_says_when_the_reading_is_past_a_limit() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(12.34), "A", StatusFlags::default());
        let above = ReadingState {
            scaled: false,
            alarm_set: true,
            alarm: Some(Zone::Above),
        };
        let label = live_region_label(Some(&m), above, NO_READING_TITLE);
        assert!(label.ends_with(", above the high limit"), "{label:?}");
        assert_ne!(
            live_region_fingerprint(Some(&m), ReadingState::PLAIN, NO_READING_TITLE),
            live_region_fingerprint(Some(&m), above, NO_READING_TITLE)
        );
    }

    #[test]
    fn format_display_raw_consistent_width() {
        // All outputs should be at least 7 chars wide
        let inputs = [" 0.0000", "  5.678", "-12.345", "    OL ", "- 55.79"];
        for input in &inputs {
            let output = format_display_raw(input);
            assert!(
                output.len() >= 7,
                "format_display_raw({input:?}) = {output:?} should be >= 7 chars"
            );
        }
    }
}
