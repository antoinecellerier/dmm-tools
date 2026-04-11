use eframe::egui::{Color32, FontId, RichText, Ui};
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

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

            ui.horizontal(|ui| {
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
            });

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
            ui.label(
                RichText::new(crate::NO_DATA)
                    .font(FontId::monospace(value_size))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(RichText::new("No reading").color(ui.visuals().weak_text_color()));
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

            ui.horizontal(|ui| {
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
            });
        }
        None => {
            ui.label(
                RichText::new(format!("{} No reading", crate::NO_DATA))
                    .font(FontId::monospace(value_size))
                    .color(ui.visuals().weak_text_color()),
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

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(
                    RichText::new(&value_text).font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
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
            });
        }
        None => {
            ui.label(
                RichText::new(format!("{} No reading", crate::NO_DATA))
                    .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                    .color(ui.visuals().weak_text_color()),
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
