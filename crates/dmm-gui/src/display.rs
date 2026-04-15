use dmm_lib::flags::StatusFlags;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use eframe::egui::{Color32, FontId, RichText, Ui};

use crate::a11y::UiA11yExt;
use crate::settings::{ColorPreset, PaletteOverrides};
use crate::theme::ThemeColors;

/// Base font size for the primary reading in the wide (side panel) layout.
pub(crate) const BASE_READING_FONT_SIZE: f32 = 36.0;

/// Minimum font size for the big meter scaled reading. Smaller than
/// `BASE_READING_FONT_SIZE` so the window can shrink to a tiny widget.
pub(crate) const MIN_BIG_METER_FONT_SIZE: f32 = 12.0;

/// Font size for the primary reading in the compact (narrow) layout.
const COMPACT_READING_FONT_SIZE: f32 = 28.0;

/// Format the meter's raw 7-char display string for stable rendering.
/// Replaces leading spaces with figure spaces (U+2007) so the minus sign
/// doesn't shift digits in monospace font.
fn format_display_raw(raw: &str) -> String {
    let trimmed = raw.trim_end();
    // Pad to at least 7 chars for consistent width
    format!("{trimmed:>7}")
}

/// Format the measurement value as a display string.
/// Uses the meter's raw 7-char display when available (UT61E+ protocol),
/// otherwise formats the numeric value for float-based protocols.
fn format_value_display(m: &Measurement) -> String {
    if let Some(raw) = &m.display_raw {
        format_display_raw(raw)
    } else {
        match &m.value {
            MeasuredValue::Normal(v) => format!("{v:>7}"),
            MeasuredValue::Overload => format!("{:>7}", "OL"),
            MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
        }
    }
}

/// Format a measurement as a spoken-friendly one-line description for screen
/// readers. Used as the live-region label on the primary reading. Uses the
/// same value formatting as the visible display so AT users hear exactly
/// what sighted users see.
fn live_region_label(measurement: Option<&Measurement>) -> String {
    match measurement {
        Some(m) => {
            let value = match &m.value {
                MeasuredValue::Overload => "overload".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV level {l}"),
                MeasuredValue::Normal(_) => format_value_display(m).trim().to_string(),
            };
            let mut parts = String::with_capacity(96);
            parts.push_str(&value);
            if !m.unit.is_empty() {
                parts.push(' ');
                parts.push_str(&m.unit);
            }
            if !m.mode.is_empty() {
                parts.push_str(", ");
                parts.push_str(&m.mode);
            }
            // Speak the same status flags that the visible badge row shows.
            // Without this, a screen reader user toggling HOLD/REL/MIN/MAX/
            // AUTO via the on-device buttons hears the value change but no
            // confirmation that the mode actually flipped.
            append_flags_phrase(&mut parts, &m.flags);
            parts
        }
        None => "No reading".to_string(),
    }
}

/// Append a phrase listing the active status flags, in the same order as
/// `show_flags` paints them. Each flag is prefixed with ", " so it reads
/// naturally after the mode field. No-op if all flags are inactive.
fn append_flags_phrase(out: &mut String, flags: &StatusFlags) {
    let push = |label: &str, out: &mut String| {
        out.push_str(", ");
        out.push_str(label);
    };
    if flags.auto_range {
        push("auto range", out);
    }
    if flags.hold {
        push("hold", out);
    }
    if flags.rel {
        push("relative", out);
    }
    if flags.min {
        push("minimum", out);
    }
    if flags.max {
        push("maximum", out);
    }
    if flags.peak_min {
        push("peak minimum", out);
    }
    if flags.peak_max {
        push("peak maximum", out);
    }
    if flags.low_battery {
        push("low battery", out);
    }
    if flags.lead_error {
        push("lead error", out);
    }
    if flags.comp {
        push("compare", out);
    }
    if flags.record {
        push("recording", out);
    }
}

/// Pack a `StatusFlags` into a u16 bitfield for fingerprint hashing. Stable
/// across runs because we list each flag explicitly rather than relying on
/// struct field order.
fn flags_bits(flags: &StatusFlags) -> u16 {
    (flags.hold as u16)
        | ((flags.rel as u16) << 1)
        | ((flags.min as u16) << 2)
        | ((flags.max as u16) << 3)
        | ((flags.auto_range as u16) << 4)
        | ((flags.low_battery as u16) << 5)
        | ((flags.hv_warning as u16) << 6)
        | ((flags.dc as u16) << 7)
        | ((flags.peak_max as u16) << 8)
        | ((flags.peak_min as u16) << 9)
        | ((flags.lead_error as u16) << 10)
        | ((flags.comp as u16) << 11)
        | ((flags.record as u16) << 12)
}

/// Build a u64 fingerprint that changes whenever `live_region_label` would
/// produce different output. Lets `set_live_region_cached` skip per-frame
/// `format!`/`String` allocation when the measurement is unchanged.
fn live_region_fingerprint(measurement: Option<&Measurement>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    match measurement {
        None => 0u8.hash(&mut h),
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
            }
            m.unit.hash(&mut h);
            m.mode.hash(&mut h);
            flags_bits(&m.flags).hash(&mut h);
        }
    }
    h.finish()
}

/// Prepare the value text and color from a measurement.
fn value_display(ui: &Ui, m: &Measurement, tc: &ThemeColors) -> (String, Color32) {
    match &m.value {
        MeasuredValue::Normal(_) => (format_value_display(m), ui.visuals().text_color()),
        MeasuredValue::Overload => (format_value_display(m), tc.status_error()),
        MeasuredValue::NcvLevel(_) => (format_value_display(m), ui.visuals().text_color()),
    }
}

/// Render the primary reading display at the given font size (two-line layout).
fn show_reading_sized(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
) {
    let unit_size = value_size;
    let mode_size = value_size * 0.4;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(ui, m, tc);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m)),
                || live_region_label(Some(m)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(value_size))
                            .color(value_color),
                    );
                    ui.label(
                        RichText::new(&*m.unit)
                            .font(FontId::monospace(unit_size))
                            .color(ui.visuals().text_color()),
                    );
                },
            );

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.5).max(2.0);
                ui.label(
                    RichText::new(&*m.mode)
                        .font(FontId::proportional(mode_size))
                        .color(ui.visuals().weak_text_color()),
                );
                if !m.range_label.is_empty() {
                    ui.label(
                        RichText::new(&*m.range_label)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                show_flags(ui, m, mode_size, tc);
            });
        }
        None => {
            // Wrap the placeholder + caption in a horizontal scope so the
            // live-region label is attached to the scope id rather than to
            // the inner ui.label() Response. egui maps Role::Label
            // overrides to set_value, not set_label, so attaching directly
            // to the label would silently drop the live-region label.
            ui.live_region_horizontal(
                live_region_fingerprint(None),
                || live_region_label(None),
                |ui| {
                    ui.label(
                        RichText::new(crate::NO_DATA)
                            .font(FontId::monospace(value_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.label(RichText::new("No reading").color(ui.visuals().weak_text_color()));
                },
            );
        }
    }
}

/// Render the reading with value and mode on a single line (inline layout).
fn show_reading_inline(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
) {
    let unit_size = value_size;
    let mode_size = value_size * 0.4;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(ui, m, tc);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m)),
                || live_region_label(Some(m)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(value_size))
                            .color(value_color),
                    );
                    ui.label(
                        RichText::new(&*m.unit)
                            .font(FontId::monospace(unit_size))
                            .color(ui.visuals().text_color()),
                    );
                    ui.separator();
                    ui.spacing_mut().item_spacing.x = (mode_size * 0.3).max(2.0);
                    ui.label(
                        RichText::new(&*m.mode)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                    show_flags(ui, m, mode_size, tc);
                },
            );
        }
        None => {
            // See `show_reading_sized` for why the placeholder is wrapped
            // in a horizontal scope: egui Role::Label silently swallows
            // accesskit set_label overrides.
            ui.live_region_horizontal(
                live_region_fingerprint(None),
                || live_region_label(None),
                |ui| {
                    ui.label(
                        RichText::new(format!("{} No reading", crate::NO_DATA))
                            .font(FontId::monospace(value_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                },
            );
        }
    }
}

/// Render the large primary reading display.
pub fn show_reading(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    preset: ColorPreset,
    overrides: &PaletteOverrides,
) {
    let tc = ThemeColors::new(ui.visuals().dark_mode, preset, overrides);
    show_reading_sized(ui, measurement, BASE_READING_FONT_SIZE, &tc);
}

/// Cached ratios of rendered reading dimensions to font size.
/// Used by `show_reading_large` to compute the optimal font size and
/// updated by the caller only on window resize (to avoid oscillation).
#[derive(Clone)]
pub struct ReadingRatios {
    /// Two-line layout: reading width / font_size.
    pub w: f32,
    /// Two-line layout: reading height / font_size.
    pub h: f32,
    /// Inline layout: reading width / font_size.
    pub inline_w: f32,
    /// Inline layout: reading height / font_size.
    pub inline_h: f32,
}

impl Default for ReadingRatios {
    fn default() -> Self {
        Self {
            w: 6.5,
            h: 1.8,
            inline_w: 10.0,
            inline_h: 1.0,
        }
    }
}

/// Render an extra-large reading that scales to fill available space.
/// Used when graph and recording panels are hidden ("big meter" mode).
/// Returns `(scale_factor, measured_ratios)`. The caller should only
/// persist `measured_ratios` into the cached state when recalculating
/// (e.g. on window resize) to avoid frame-to-frame oscillation.
///
/// `base_content_height`: total height of all content below the reading
/// (buttons, stats, etc.) rendered at scale=1. The caller measures this
/// once and passes it in so we can compute the optimal scale.
pub fn show_reading_large(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    base_content_height: f32,
    ratios: &ReadingRatios,
    preset: ColorPreset,
    overrides: &PaletteOverrides,
) -> (f32, ReadingRatios) {
    let available_w = ui.available_width();
    let available_h = ui.available_height();

    let content_coeff = base_content_height / BASE_READING_FONT_SIZE;

    // Two-line layout: value+unit on top, mode below.
    let two_line_w = available_w / ratios.w;
    let two_line_h = available_h / (ratios.h + content_coeff);
    let two_line_size = two_line_w.min(two_line_h);

    // Inline layout: value+unit+mode all on one row.
    let inline_w = available_w / ratios.inline_w;
    let inline_h = available_h / (ratios.inline_h + content_coeff);
    let inline_size = inline_w.min(inline_h);

    // Use inline layout when it produces an equal or larger font size,
    // meaning the window is wide enough to fit everything on one line
    // without shrinking the value.
    let use_inline = inline_size >= two_line_size;
    let size = if use_inline {
        inline_size
    } else {
        two_line_size
    }
    .max(MIN_BIG_METER_FONT_SIZE);

    // Render and measure actual dimensions.
    let tc = ThemeColors::new(ui.visuals().dark_mode, preset, overrides);
    let before = ui.cursor().top();
    if use_inline {
        show_reading_inline(ui, measurement, size, &tc);
    } else {
        show_reading_sized(ui, measurement, size, &tc);
    }
    let reading_w = ui.min_rect().width();
    let reading_h = ui.cursor().top() - before;

    let mut measured = ratios.clone();
    if size > 0.0 {
        if use_inline {
            measured.inline_w = reading_w / size;
            measured.inline_h = reading_h / size;
        } else {
            measured.w = reading_w / size;
            measured.h = reading_h / size;
        }
    }

    (size / BASE_READING_FONT_SIZE, measured)
}

/// Render the reading as a compact single line (for narrow layout).
pub fn show_reading_compact(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    preset: ColorPreset,
    overrides: &PaletteOverrides,
) {
    match measurement {
        Some(m) => {
            let value_text = format_value_display(m);
            let tc = ThemeColors::new(ui.visuals().dark_mode, preset, overrides);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m)),
                || live_region_label(Some(m)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                    );
                    ui.label(
                        RichText::new(&*m.unit).font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(&*m.mode)
                            .color(ui.visuals().weak_text_color())
                            .small(),
                    );
                    show_flags(ui, m, 0.0, &tc);
                },
            );
        }
        None => {
            // See `show_reading_sized` for why the placeholder is wrapped
            // in a horizontal scope: egui Role::Label silently swallows
            // accesskit set_label overrides.
            ui.live_region_horizontal(
                live_region_fingerprint(None),
                || live_region_label(None),
                |ui| {
                    ui.label(
                        RichText::new(format!("{} No reading", crate::NO_DATA))
                            .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                            .color(ui.visuals().weak_text_color()),
                    );
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let label = live_region_label(Some(&m));
        assert!(label.contains("V"), "got {label:?}");
        assert!(label.contains("DC V"), "got {label:?}");
        assert!(label.contains("auto range"), "got {label:?}");
        assert!(label.contains("hold"), "got {label:?}");
    }

    #[test]
    fn live_region_label_no_flags_when_inactive() {
        let m = Measurement::test_fixture(MeasuredValue::Normal(0.0), "V", StatusFlags::default());
        let label = live_region_label(Some(&m));
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
        let fp1 = live_region_fingerprint(Some(&m));
        m.flags.hold = true;
        let fp2 = live_region_fingerprint(Some(&m));
        assert_ne!(fp1, fp2, "toggling HOLD must change the fingerprint");
        m.flags.hold = false;
        m.flags.rel = true;
        let fp3 = live_region_fingerprint(Some(&m));
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

fn show_flags(ui: &mut Ui, m: &Measurement, font_size: f32, tc: &ThemeColors) {
    let badge = |ui: &mut Ui, label: &str, color: Color32| {
        let mut text = RichText::new(label).strong().color(color);
        if font_size > 0.0 {
            text = text.font(FontId::proportional(font_size));
        } else {
            text = text.small();
        }
        ui.label(text);
    };

    let accent = tc.accent();
    let warning = tc.recording_full_warning();

    if m.flags.auto_range {
        badge(ui, "AUTO", accent);
    }
    if m.flags.hold {
        badge(ui, "HOLD", accent);
    }
    if m.flags.rel {
        badge(ui, "REL", accent);
    }
    if m.flags.min {
        badge(ui, "MIN", accent);
    }
    if m.flags.max {
        badge(ui, "MAX", accent);
    }
    if m.flags.low_battery {
        badge(ui, "LOW BAT", warning);
    }
    if m.flags.lead_error {
        badge(ui, "LEAD ERR", warning);
    }
    if m.flags.comp {
        badge(ui, "COMP", accent);
    }
    if m.flags.record {
        badge(ui, "REC", accent);
    }
}
