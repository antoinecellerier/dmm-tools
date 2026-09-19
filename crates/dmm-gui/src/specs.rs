use dmm_lib::specs::{ModeSpecInfo, SpecInfo};
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

/// An accuracy as the manual prints it: a figure as `±(0.1%+5)`, a word
/// accuracy ("Not Specified", "Only for reference") as it stands.
fn accuracy_text(accuracy: &str) -> String {
    if accuracy.starts_with(|c: char| c.is_ascii_digit()) {
        format!("\u{00B1}({accuracy})")
    } else {
        accuracy.to_string()
    }
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
        Some(accuracy_text(first.accuracy))
    } else {
        let freq = first.freq_range.unwrap_or("");
        Some(format!("{} {freq}", accuracy_text(first.accuracy)))
    }
}

/// Build the summary parts vector used by compact and inline layouts: the
/// resolution and accuracy only, as an unlabelled impedance reads as noise
/// on one line; the side panel carries the rest.
///
/// `res_label` / `acc_label` control the prefix for each field so callers can
/// choose between short (`"Res:"`) and long (`"Resolution"`) labels. A
/// reading with no range row (`spec` is `None`) has no parts.
fn build_spec_parts(spec: Option<&SpecInfo>, res_label: &str, acc_label: &str) -> Vec<String> {
    let Some(spec) = spec else {
        return Vec::new();
    };
    let mut parts = vec![format!("{res_label} {}", spec.resolution)];
    if let Some(acc_str) = compact_accuracy_str(spec) {
        parts.push(format!("{acc_label} {acc_str}"));
    }
    parts
}

/// Separates the summary from the "Manual ↗" link in both one-line layouts.
const LINK_SEPARATOR: &str = "  |  ";

/// The one-line summary: `parts` joined by `joiner`, then a separator when
/// the Manual link follows. `None` when there are no parts, so the link
/// stands alone.
fn summary_line(parts: &[String], joiner: &str, link_follows: bool) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    let mut line = parts.join(joiner);
    if link_follows {
        line.push_str(LINK_SEPARATOR);
    }
    Some(line)
}

/// Full specs panel for the wide (side panel) layout. A reading with no
/// range row (`spec` is `None`) shows its mode's impedance and notes only.
pub fn show_specs(
    ui: &mut Ui,
    spec: Option<&SpecInfo>,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
) {
    let main_font = 12.0 * scale;
    let sub_font = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();

    ui.label(
        RichText::new("Specifications")
            .font(egui::FontId::proportional(sub_font))
            .color(weak),
    );

    if let Some(spec) = spec {
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
                let figure = RichText::new(format!("Accuracy  {}", accuracy_text(single.accuracy)))
                    .font(egui::FontId::proportional(main_font));
                match single.freq_range {
                    // A lone band keeps its qualifier (LPF V's "40Hz~100Hz
                    // (LPF)"), dimmed as in the band list. One label at one
                    // size, so the row keeps a plain label's height and the
                    // two parts share a baseline.
                    Some(freq) => {
                        let mut job = egui::text::LayoutJob::default();
                        let style = ui.style();
                        let font = egui::FontSelection::Default;
                        figure.append_to(&mut job, style, font.clone(), egui::Align::BOTTOM);
                        RichText::new(format!("  {freq}"))
                            .font(egui::FontId::proportional(main_font))
                            .color(weak)
                            .append_to(&mut job, style, font, egui::Align::BOTTOM);
                        ui.label(job);
                    }
                    None => {
                        ui.label(figure);
                    }
                }
            }
            bands => {
                ui.label(RichText::new("Accuracy").font(egui::FontId::proportional(main_font)));
                for band in bands {
                    let freq = band.freq_range.unwrap_or(crate::NO_DATA);
                    ui.label(
                        RichText::new(format!("  {freq}  {}", accuracy_text(band.accuracy)))
                            .font(egui::FontId::proportional(sub_font))
                            .color(weak),
                    );
                }
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
pub fn show_specs_compact(ui: &mut Ui, spec: Option<&SpecInfo>, manual_url: Option<&str>) {
    let weak = ui.visuals().weak_text_color();
    let sub_font = 11.0;

    // Build a compact string: "Res: 0.01mV  Acc: ±(0.1%+5)"
    let parts = build_spec_parts(spec, "Res:", "Acc:");

    ui.horizontal_wrapped(|ui| {
        // The separator's own spaces set the gap before the link.
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(line) = summary_line(&parts, "  ", manual_url.is_some()) {
            ui.label(
                RichText::new(line)
                    .font(egui::FontId::proportional(sub_font))
                    .color(weak),
            );
        }
        if let Some(url) = manual_url {
            manual_link(ui, url, sub_font, weak);
        }
    });
}

/// Compact specs with the mode data and a scale parameter (both ignored) for
/// uniform callback signature.
pub fn show_specs_compact_scaled(
    ui: &mut Ui,
    spec: Option<&SpecInfo>,
    _mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    _scale: f32,
) {
    show_specs_compact(ui, spec, manual_url);
}

/// Inline pipe-separated specs for big meter mode. The mode data is not shown
/// here; the parameter keeps the callback signature uniform.
pub fn show_specs_inline(
    ui: &mut Ui,
    spec: Option<&SpecInfo>,
    _mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
) {
    let font_size = 12.0 * scale;
    let weak = ui.visuals().weak_text_color();

    let parts = build_spec_parts(spec, "Resolution", "Accuracy");

    ui.horizontal_wrapped(|ui| {
        // The separator's own spaces set the gap before the link.
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(line) = summary_line(&parts, LINK_SEPARATOR, manual_url.is_some()) {
            ui.label(
                RichText::new(line)
                    .font(egui::FontId::proportional(font_size))
                    .color(weak),
            );
        }
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
    use dmm_lib::specs::AccuracyBand;

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

    /// The text the full panel draws for `spec`.
    fn panel_texts(spec: &SpecInfo) -> Vec<String> {
        fn collect(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            show_specs(ui, Some(spec), None, None, 1.0);
        });
        out.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &out.shapes {
            collect(&clipped.shape, &mut texts);
        }
        texts
    }

    /// LPF V's one band holds only from 40Hz to 100Hz; the panel says so.
    #[test]
    fn a_lone_band_keeps_its_qualifier() {
        const LPF_BAND: &[AccuracyBand] = &[AccuracyBand {
            freq_range: Some("40Hz~100Hz (LPF)"),
            accuracy: "3.0%+50",
        }];
        let texts = panel_texts(&spec(LPF_BAND));
        assert!(
            texts
                .iter()
                .any(|t| t == "Accuracy  \u{00B1}(3.0%+50)  40Hz~100Hz (LPF)"),
            "{texts:?}"
        );
    }

    #[test]
    fn a_dc_band_shows_the_figure_alone() {
        let texts = panel_texts(&spec(DC_BAND));
        assert!(
            texts.iter().any(|t| t == "Accuracy  \u{00B1}(0.1%+5)"),
            "{texts:?}"
        );
    }

    /// Continuity and diode ship `accuracy: &[]`; the compact renderers must not
    /// index into it.
    #[test]
    fn empty_accuracy_yields_no_string() {
        assert_eq!(compact_accuracy_str(&spec(&[])), None);
    }

    #[test]
    fn empty_accuracy_omits_the_accuracy_part() {
        let parts = build_spec_parts(Some(&spec(&[])), "Res:", "Acc:");
        assert_eq!(parts, vec!["Res: 0.01mV".to_string()]);
    }

    /// A reading with no range row has no parts, so the one-line layouts
    /// show the Manual link alone.
    #[test]
    fn no_row_leaves_only_the_manual_link() {
        assert!(build_spec_parts(None, "Res:", "Acc:").is_empty());
    }

    /// The Manual link is set off from the summary, and stands alone when
    /// there is none.
    #[test]
    fn the_summary_is_separated_from_the_manual_link() {
        let parts = build_spec_parts(Some(&spec(DC_BAND)), "Res:", "Acc:");
        assert_eq!(
            summary_line(&parts, "  ", true).as_deref(),
            Some("Res: 0.01mV  Acc: \u{00B1}(0.1%+5)  |  ")
        );
        assert_eq!(
            summary_line(&parts, "  ", false).as_deref(),
            Some("Res: 0.01mV  Acc: \u{00B1}(0.1%+5)")
        );
        assert_eq!(summary_line(&[], "  ", true), None);
    }

    /// The one-line layouts carry the row's resolution and accuracy only.
    #[test]
    fn a_row_gives_resolution_and_accuracy_only() {
        let parts = build_spec_parts(Some(&spec(DC_BAND)), "Res:", "Acc:");
        assert_eq!(
            parts,
            vec![
                "Res: 0.01mV".to_string(),
                "Acc: \u{00B1}(0.1%+5)".to_string(),
            ]
        );
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

    #[test]
    fn a_figure_is_wrapped_in_plus_minus() {
        assert_eq!(accuracy_text("0.4%+30"), "\u{00B1}(0.4%+30)");
        assert_eq!(accuracy_text("2.5%"), "\u{00B1}(2.5%)");
    }

    /// "Not Specified", "Only for reference" and "reference only" are words
    /// the manual prints in the accuracy column, not a tolerance.
    #[test]
    fn a_word_accuracy_stands_as_printed() {
        for word in ["Not Specified", "Only for reference", "reference only"] {
            assert_eq!(accuracy_text(word), word);
        }
        const WORD_FIRST: &[AccuracyBand] = &[
            AccuracyBand {
                freq_range: Some("10k~100kHz"),
                accuracy: "Only for reference",
            },
            AccuracyBand {
                freq_range: Some("45~1kHz"),
                accuracy: "0.6%+30",
            },
        ];
        assert_eq!(
            compact_accuracy_str(&spec(WORD_FIRST)).as_deref(),
            Some("Only for reference 10k~100kHz")
        );
    }
}
