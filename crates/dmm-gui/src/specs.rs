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
fn compact_accuracy_str(spec: &SpecInfo) -> String {
    if spec.accuracy.len() == 1 {
        format!("\u{00B1}({})", spec.accuracy[0].accuracy)
    } else {
        let band = &spec.accuracy[0];
        let freq = band.freq_range.unwrap_or("");
        format!("\u{00B1}({}) {freq}", band.accuracy)
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
    let acc_str = compact_accuracy_str(spec);
    let mut parts = vec![
        format!("{res_label} {}", spec.resolution),
        format!("{acc_label} {acc_str}"),
    ];
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

    // Accuracy
    if spec.accuracy.len() == 1 {
        ui.label(
            RichText::new(format!("Accuracy  {}", compact_accuracy_str(spec)))
                .font(egui::FontId::proportional(main_font)),
        );
    } else {
        ui.label(RichText::new("Accuracy").font(egui::FontId::proportional(main_font)));
        for band in spec.accuracy {
            let freq = band.freq_range.unwrap_or(crate::NO_DATA);
            ui.label(
                RichText::new(format!("  {freq}  \u{00B1}({})", band.accuracy))
                    .font(egui::FontId::proportional(sub_font))
                    .color(weak),
            );
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
