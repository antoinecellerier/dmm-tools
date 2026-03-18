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

/// Render the large primary reading display.
pub fn show_reading(ui: &mut Ui, measurement: Option<&Measurement>) {
    match measurement {
        Some(m) => {
            // Use the meter's 7-char display string for stable formatting.
            // display_raw preserves leading spaces (sign placeholder) and
            // fixed decimal places per range, so numbers don't jump around.
            let (value_text, value_color) = match &m.value {
                MeasuredValue::Normal(_) => {
                    (format_display_raw(&m.display_raw), ui.visuals().text_color())
                }
                MeasuredValue::Overload => {
                    (format_display_raw(&m.display_raw), if ui.visuals().dark_mode { Color32::from_rgb(220, 60, 60) } else { Color32::from_rgb(180, 0, 0) })
                }
                MeasuredValue::NcvLevel(l) => {
                    (format!("NCV {l}"), ui.visuals().text_color())
                }
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(
                    RichText::new(&value_text)
                        .font(FontId::monospace(36.0))
                        .color(value_color),
                );
                ui.label(
                    RichText::new(m.unit)
                        .font(FontId::monospace(20.0))
                        .color(ui.visuals().text_color()),
                );
            });

            // Mode, range, flags
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new(m.mode.to_string())
                        .color(ui.visuals().weak_text_color()),
                );
                if !m.range_label.is_empty() {
                    ui.label(
                        RichText::new(m.range_label)
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                show_flags(ui, m);
            });
        }
        None => {
            ui.label(
                RichText::new("---")
                    .font(FontId::monospace(36.0))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(
                RichText::new("No reading")
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }
}

/// Render the reading as a compact single line (for narrow layout).
pub fn show_reading_compact(ui: &mut Ui, measurement: Option<&Measurement>) {
    match measurement {
        Some(m) => {
            let value_text = match &m.value {
                MeasuredValue::Normal(_) | MeasuredValue::Overload => {
                    format_display_raw(&m.display_raw)
                }
                MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(
                    RichText::new(&value_text)
                        .font(FontId::monospace(28.0)),
                );
                ui.label(
                    RichText::new(m.unit)
                        .font(FontId::monospace(16.0)),
                );
                ui.separator();
                ui.label(
                    RichText::new(m.mode.to_string())
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

fn show_flags(ui: &mut Ui, m: &Measurement) {
    let badge = |ui: &mut Ui, label: &str, color: Color32| {
        let text = RichText::new(label).small().strong().color(color);
        ui.label(text);
    };

    let dark = ui.visuals().dark_mode;
    let accent = if dark { Color32::from_rgb(100, 180, 255) } else { Color32::from_rgb(0, 100, 200) };
    let warning = if dark { Color32::from_rgb(230, 160, 40) } else { Color32::from_rgb(180, 100, 0) };

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
