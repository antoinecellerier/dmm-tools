use eframe::egui::{Color32, FontId, RichText, Ui};
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

/// Format the meter's raw 7-char display string for stable rendering.
/// Replaces leading spaces with figure spaces (U+2007) so the minus sign
/// doesn't shift digits in monospace font.
fn format_display_raw(raw: &str) -> String {
    let trimmed = raw.trim_end();
    // Pad to at least 7 chars for consistent width
    format!("{trimmed:>7}")
}

/// Render the primary reading display at the given font size.
fn show_reading_sized(ui: &mut Ui, measurement: Option<&Measurement>, value_size: f32) {
    let unit_size = value_size;
    let mode_size = (value_size * 0.4).max(12.0);

    match measurement {
        Some(m) => {
            let display_str = |m: &Measurement| -> String {
                if let Some(raw) = &m.display_raw {
                    format_display_raw(raw)
                } else {
                    // Float-based protocols: format the numeric value
                    match &m.value {
                        MeasuredValue::Normal(v) => format!("{v:>7}"),
                        MeasuredValue::Overload => format!("{:>7}", "OL"),
                        MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
                    }
                }
            };
            let (value_text, value_color) = match &m.value {
                MeasuredValue::Normal(_) => (display_str(m), ui.visuals().text_color()),
                MeasuredValue::Overload => (
                    display_str(m),
                    if ui.visuals().dark_mode {
                        Color32::from_rgb(220, 60, 60)
                    } else {
                        Color32::from_rgb(180, 0, 0)
                    },
                ),
                MeasuredValue::NcvLevel(l) => (format!("NCV {l}"), ui.visuals().text_color()),
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(
                    RichText::new(&value_text)
                        .font(FontId::monospace(value_size))
                        .color(value_color),
                );
                ui.label(
                    RichText::new(&m.unit)
                        .font(FontId::monospace(unit_size))
                        .color(ui.visuals().text_color()),
                );
            });

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new(&m.mode)
                        .font(FontId::proportional(mode_size))
                        .color(ui.visuals().weak_text_color()),
                );
                if !m.range_label.is_empty() {
                    ui.label(
                        RichText::new(&m.range_label)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                show_flags(ui, m);
            });
        }
        None => {
            ui.label(
                RichText::new("---")
                    .font(FontId::monospace(value_size))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(RichText::new("No reading").color(ui.visuals().weak_text_color()));
        }
    }
}

/// Render the large primary reading display.
pub fn show_reading(ui: &mut Ui, measurement: Option<&Measurement>) {
    show_reading_sized(ui, measurement, 36.0);
}

/// Render an extra-large reading that scales to fill available space.
/// Used when graph and recording panels are hidden ("big meter" mode).
/// Returns a scale factor (1.0 = normal) for other UI elements to match.
///
/// `base_content_height`: total height of all content below the reading
/// (buttons, stats, etc.) rendered at scale=1. The caller measures this
/// once and passes it in so we can compute the optimal scale.
pub fn show_reading_large(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    base_content_height: f32,
) -> f32 {
    let available_w = ui.available_width();
    let available_h = ui.available_height();

    // Width-based limit: 7 value chars + ~3 unit chars = 10 char widths
    let size_from_w = available_w / 10.0;

    // Height-based limit: total height scales linearly with font size.
    // At base (size=36, scale=1):
    //   reading_h ≈ 36 * 1.8 (value line + mode line)
    //   other_h   = base_content_height (buttons, stats, gaps)
    // At scale s/36:
    //   total = s * 1.8 + base_content_height * (s / 36)
    //         = s * (1.8 + base_content_height / 36)
    let height_coeff = 1.8 + base_content_height / 36.0;
    let size_from_h = available_h / height_coeff;

    let size = size_from_w.min(size_from_h).max(36.0);
    show_reading_sized(ui, measurement, size);
    size / 36.0
}

/// Render the reading as a compact single line (for narrow layout).
pub fn show_reading_compact(ui: &mut Ui, measurement: Option<&Measurement>) {
    match measurement {
        Some(m) => {
            let value_text = match &m.value {
                MeasuredValue::Normal(_) | MeasuredValue::Overload => {
                    if let Some(raw) = &m.display_raw {
                        format_display_raw(raw)
                    } else {
                        match &m.value {
                            MeasuredValue::Normal(v) => format!("{v:>7}"),
                            MeasuredValue::Overload => format!("{:>7}", "OL"),
                            _ => String::new(),
                        }
                    }
                }
                MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(RichText::new(&value_text).font(FontId::monospace(28.0)));
                ui.label(RichText::new(&m.unit).font(FontId::monospace(28.0)));
                ui.separator();
                ui.label(
                    RichText::new(&m.mode)
                        .color(ui.visuals().weak_text_color())
                        .small(),
                );
                show_flags(ui, m);
            });
        }
        None => {
            ui.label(
                RichText::new("--- No reading")
                    .font(FontId::monospace(28.0))
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

fn show_flags(ui: &mut Ui, m: &Measurement) {
    let badge = |ui: &mut Ui, label: &str, color: Color32| {
        let text = RichText::new(label).small().strong().color(color);
        ui.label(text);
    };

    let dark = ui.visuals().dark_mode;
    let accent = if dark {
        Color32::from_rgb(100, 180, 255)
    } else {
        Color32::from_rgb(0, 100, 200)
    };
    let warning = if dark {
        Color32::from_rgb(230, 160, 40)
    } else {
        Color32::from_rgb(180, 100, 0)
    };

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
}
