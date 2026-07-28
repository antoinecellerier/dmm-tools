use dmm_lib::protocol::ut61eplus::tables::{ModeSpecInfo, SpecInfo};
use eframe::egui::{self, Color32, RichText, Ui};

const MANUAL_TOOLTIP: &str = "Open the manufacturer's manual in your browser";

/// Render a "Manual ↗" hyperlink with a consistent hover tooltip.
fn manual_link(ui: &mut Ui, url: &str, font_size: f32, color: Color32) {
    ui.hyperlink_to(
        RichText::new("Manual \u{2197}")
            .font(egui::FontId::proportional(font_size))
            .color(color),
        url,
    )
    .on_hover_text(MANUAL_TOOLTIP);
}

/// Build a compact single-line accuracy string from the spec's accuracy bands.
/// For a single band: `±(0.1%+5)`. For multiple bands: first band only with its
/// frequency range appended, e.g. `±(0.1%+5) 45Hz~1kHz`.
///
/// Returns `None` when the spec carries no accuracy bands — modes such as
/// continuity and diode have no accuracy figure in the manual and ship an
/// empty slice.
fn compact_accuracy_str(spec: &SpecInfo) -> Option<String> {
    let (first, rest) = spec.accuracy.split_first()?;
    if rest.is_empty() {
        Some(format!("\u{00B1}({})", first.accuracy))
    } else {
        let freq = first.freq_range.unwrap_or("");
        Some(format!("\u{00B1}({}) {freq}", first.accuracy))
    }
}

/// Build the summary parts vector used by compact and inline layouts.
///
/// `res_label` / `acc_label` control the prefix for each field so callers can
/// choose between short (`"Res:"`) and long (`"Resolution"`) labels.
fn build_spec_parts(
    spec: &SpecInfo,
    mode_spec: Option<&ModeSpecInfo>,
    res_label: &str,
    acc_label: &str,
) -> Vec<String> {
    let mut parts = vec![format!("{res_label} {}", spec.resolution)];
    if let Some(acc_str) = compact_accuracy_str(spec) {
        parts.push(format!("{acc_label} {acc_str}"));
    }
    if let Some(ms) = mode_spec
        && let Some(z) = ms.input_impedance
    {
        parts.push(z.to_string());
    }
    parts
}

/// Full specs panel for the wide (side panel) layout.
pub fn show_specs(
    ui: &mut Ui,
    spec: &SpecInfo,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
) {
    let main_font = 12.0 * scale;
    let sub_font = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();

    ui.label(
        RichText::new("Specifications")
            .strong()
            .font(egui::FontId::proportional(sub_font)),
    );

    // Resolution
    ui.label(
        RichText::new(format!("Resolution  {}", spec.resolution))
            .font(egui::FontId::proportional(main_font)),
    );

    // Accuracy — omitted entirely for modes that have no accuracy figure
    // (continuity, diode), which ship an empty band slice.
    match spec.accuracy {
        [] => {}
        [single] => {
            ui.label(
                RichText::new(format!("Accuracy  \u{00B1}({})", single.accuracy))
                    .font(egui::FontId::proportional(main_font)),
            );
        }
        bands => {
            ui.label(RichText::new("Accuracy").font(egui::FontId::proportional(main_font)));
            for band in bands {
                let freq = band.freq_range.unwrap_or(crate::NO_DATA);
                ui.label(
                    RichText::new(format!("  {freq}  \u{00B1}({})", band.accuracy))
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
            }
        }
    }

    // Input impedance and notes
    if let Some(ms) = mode_spec {
        if let Some(z) = ms.input_impedance {
            ui.label(
                RichText::new(format!("Input Z  {z}")).font(egui::FontId::proportional(main_font)),
            );
        }
        for note in ms.notes {
            ui.label(
                RichText::new(*note)
                    .font(egui::FontId::proportional(sub_font))
                    .color(weak),
            );
        }
    }

    // Manual link
    if let Some(url) = manual_url {
        manual_link(ui, url, sub_font, weak);
    }
}

/// Compact single-line specs for the narrow layout.
pub fn show_specs_compact(
    ui: &mut Ui,
    spec: &SpecInfo,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
) {
    let weak = ui.visuals().weak_text_color();
    let sub_font = 11.0;

    // Build a compact string: "Res: 0.01mV  Acc: ±(0.1%+5)  ~10MΩ"
    let parts = build_spec_parts(spec, mode_spec, "Res:", "Acc:");

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(parts.join("  "))
                .font(egui::FontId::proportional(sub_font))
                .color(weak),
        );
        if let Some(url) = manual_url {
            manual_link(ui, url, sub_font, weak);
        }
    });
}

/// Compact specs with a scale parameter (ignored) for uniform callback signature.
pub fn show_specs_compact_scaled(
    ui: &mut Ui,
    spec: &SpecInfo,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    _scale: f32,
) {
    show_specs_compact(ui, spec, mode_spec, manual_url);
}

/// Inline pipe-separated specs for big meter mode.
pub fn show_specs_inline(
    ui: &mut Ui,
    spec: &SpecInfo,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
) {
    let font_size = 12.0 * scale;
    let weak = ui.visuals().weak_text_color();

    let parts = build_spec_parts(spec, mode_spec, "Resolution", "Accuracy");

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(parts.join("  |  "))
                .font(egui::FontId::proportional(font_size))
                .color(weak),
        );
        if let Some(url) = manual_url {
            manual_link(ui, url, font_size, weak);
        }
    });
}

/// Render only the manual link (when no spec data is available but a URL exists).
pub fn show_manual_only(ui: &mut Ui, url: &str, scale: f32) {
    let font_size = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();
    manual_link(ui, url, font_size, weak);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::ut61eplus::tables::AccuracyBand;

    const DC_BAND: &[AccuracyBand] = &[AccuracyBand {
        freq_range: None,
        accuracy: "0.1%+5",
    }];
    const AC_BANDS: &[AccuracyBand] = &[
        AccuracyBand {
            freq_range: Some("45Hz~1kHz"),
            accuracy: "0.5%+30",
        },
        AccuracyBand {
            freq_range: Some("1kHz~10kHz"),
            accuracy: "1.5%+30",
        },
    ];

    fn spec(accuracy: &'static [AccuracyBand]) -> SpecInfo {
        SpecInfo {
            resolution: "0.01mV",
            accuracy,
        }
    }

    /// Continuity and diode ship `accuracy: &[]`; the compact renderers must not
    /// index into it.
    #[test]
    fn empty_accuracy_yields_no_string() {
        assert_eq!(compact_accuracy_str(&spec(&[])), None);
    }

    #[test]
    fn empty_accuracy_omits_the_accuracy_part() {
        let parts = build_spec_parts(&spec(&[]), None, "Res:", "Acc:");
        assert_eq!(parts, vec!["Res: 0.01mV".to_string()]);
    }

    #[test]
    fn single_band_has_no_frequency_suffix() {
        assert_eq!(
            compact_accuracy_str(&spec(DC_BAND)).as_deref(),
            Some("\u{00B1}(0.1%+5)")
        );
    }

    #[test]
    fn multi_band_appends_first_frequency_range() {
        assert_eq!(
            compact_accuracy_str(&spec(AC_BANDS)).as_deref(),
            Some("\u{00B1}(0.5%+30) 45Hz~1kHz")
        );
    }
}
