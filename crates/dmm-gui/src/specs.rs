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

const FOLD_TOOLTIP: &str = "Show only the resolution and accuracy, on one line";
const UNFOLD_TOOLTIP: &str = "Show the full specifications";

/// Specs panel for the wide (side panel) layout, under a heading that folds
/// it to the narrow layout's one-line summary. A reading with no range row
/// (`spec` is `None`) shows its mode's impedance and notes only.
///
/// The caller owns `expanded`, a saved setting: this returns `true` when the
/// heading was clicked this frame, and the caller flips it.
pub fn show_specs(
    ui: &mut Ui,
    spec: Option<&SpecInfo>,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
    expanded: bool,
) -> bool {
    // The title where it always sat, egui's fold triangle at the column's
    // right edge, centred under the big-meter toggle, and the whole row one
    // target.
    let weak = ui.visuals().weak_text_color();
    let title = egui::WidgetText::from(
        RichText::new("Specifications")
            .font(egui::FontId::proportional(11.0 * scale))
            .color(weak),
    )
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontSelection::Default,
    );
    // The title's own height, as the plain label it replaces: the row is
    // already the column's width, and a button-height row left a gap above
    // the first line.
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), title.size().y),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::CollapsingHeader,
            ui.is_enabled(),
            "Specifications",
        )
    });
    if ui.is_rect_visible(rect) {
        let title_pos = egui::pos2(rect.left(), rect.center().y - title.size().y / 2.0);
        ui.painter().galley(title_pos, title, weak);
        // Customize colors' triangle, in the big-meter toggle's muted colour
        // until the row is hovered or focused.
        let size = ui.spacing().icon_width_inner;
        let centre = egui::pos2(
            rect.right() - crate::app::App::big_meter_toggle_width(ui) / 2.0,
            rect.center().y,
        );
        let color = if response.hovered() || response.has_focus() {
            ui.style().interact(&response).fg_stroke.color
        } else {
            weak
        };
        ui.painter().add(egui::Shape::convex_polygon(
            fold_triangle(centre, size, expanded).to_vec(),
            color,
            egui::Stroke::NONE,
        ));
    }
    // The same explicit focus ring as the Customize colors header.
    crate::a11y::paint_focus_ring(ui, &response);
    let tooltip = if expanded {
        FOLD_TOOLTIP
    } else {
        UNFOLD_TOOLTIP
    };
    let toggled = response.on_hover_text(tooltip).clicked();
    if expanded {
        show_specs_body(ui, spec, mode_spec, manual_url, scale);
    } else {
        show_specs_compact(ui, spec, manual_url);
    }
    toggled
}

/// The triangle egui's `paint_default_icon` draws for a collapsing header
/// in a `size` square: pointing down when `open`, right when not. Drawn here
/// because that painter takes its colour from the widget's interaction state.
fn fold_triangle(centre: egui::Pos2, size: f32, open: bool) -> [egui::Pos2; 3] {
    let r = egui::Rect::from_center_size(centre, egui::Vec2::splat(size * 0.75));
    let down = [r.left_top(), r.right_top(), r.center_bottom()];
    if open {
        down
    } else {
        let quarter = egui::emath::Rot2::from_angle(-std::f32::consts::FRAC_PI_2);
        down.map(|p| centre + quarter * (p - centre))
    }
}

/// Everything the unfolded panel shows under its heading.
fn show_specs_body(
    ui: &mut Ui,
    spec: Option<&SpecInfo>,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
) {
    let main_font = 12.0 * scale;
    let sub_font = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();

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

/// Compact single-line specs for the narrow layout and the folded panel.
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
    use egui::{Pos2, vec2};

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

    const AC_MODE: ModeSpecInfo = ModeSpecInfo {
        input_impedance: Some("About 10M\u{03A9}"),
        overload_protection: Some("1000V"),
        notes: &["Shorted leads: residual \u{2264}10 digits"],
    };

    fn spec(accuracy: &'static [AccuracyBand]) -> SpecInfo {
        SpecInfo {
            resolution: "0.01mV",
            accuracy,
        }
    }

    const MANUAL: &str = "Manual \u{2197}";

    /// The side column's width at the default window size.
    const COLUMN_WIDTH: f32 = 240.0;

    /// One frame of the full panel on `ctx`, with `AC_MODE`'s impedance and
    /// note and a manual URL: the text it draws, each with where, and whether
    /// the heading was clicked.
    fn panel_frame(
        ctx: &egui::Context,
        spec: &SpecInfo,
        expanded: bool,
        events: Vec<egui::Event>,
    ) -> (Vec<(String, Pos2)>, bool) {
        fn collect(shape: &egui::Shape, out: &mut Vec<(String, Pos2)>) {
            match shape {
                egui::Shape::Text(t) => out.push((t.galley.text().to_string(), t.pos)),
                egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
                _ => {}
            }
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                vec2(COLUMN_WIDTH, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut toggled = false;
        let mut out = ctx.run_ui(input, |ui| {
            toggled = show_specs(
                ui,
                Some(spec),
                Some(&AC_MODE),
                Some("https://example.com/manual"),
                1.0,
                expanded,
            );
        });
        out.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &out.shapes {
            collect(&clipped.shape, &mut texts);
        }
        (texts, toggled)
    }

    /// The text the full panel draws for `spec`, unfolded or folded.
    fn panel_texts(spec: &SpecInfo, expanded: bool) -> Vec<String> {
        let (texts, _) = panel_frame(&egui::Context::default(), spec, expanded, Vec::new());
        texts.into_iter().map(|(text, _)| text).collect()
    }

    /// A press and release at `pos`, as the pointer would deliver them.
    fn click_at(pos: Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    /// Unfolded, the panel shows every field under its heading.
    #[test]
    fn an_unfolded_panel_shows_every_field() {
        let texts = panel_texts(&spec(AC_BANDS), true);
        for shown in [
            "Specifications",
            "Resolution  0.01mV",
            "Accuracy",
            "  45Hz~1kHz  \u{00B1}(0.5%+30)",
            "  1kHz~10kHz  \u{00B1}(1.5%+30)",
            "Input Z  About 10M\u{03A9}",
            AC_MODE.notes[0],
            MANUAL,
        ] {
            assert!(
                texts.iter().any(|t| t == shown),
                "no {shown:?} in {texts:?}"
            );
        }
    }

    /// Folded, the heading stays and the narrow layout's one-line summary
    /// replaces the fields.
    #[test]
    fn a_folded_panel_shows_the_one_line_summary() {
        let texts = panel_texts(&spec(AC_BANDS), false);
        for shown in [
            "Specifications",
            "Res: 0.01mV  Acc: \u{00B1}(0.5%+30) 45Hz~1kHz  |  ",
            MANUAL,
        ] {
            assert!(
                texts.iter().any(|t| t == shown),
                "no {shown:?} in {texts:?}"
            );
        }
        for gone in [
            "Resolution  0.01mV",
            "Accuracy",
            "  1kHz~10kHz  \u{00B1}(1.5%+30)",
            "Input Z  About 10M\u{03A9}",
            AC_MODE.notes[0],
        ] {
            assert!(!texts.iter().any(|t| t == gone), "{gone:?} in {texts:?}");
        }
    }

    /// The panel doesn't fold itself: a click anywhere on the heading row,
    /// the title or the triangle at the far end, is reported, and the caller
    /// flips the saved setting.
    #[test]
    fn a_click_on_the_heading_is_reported() {
        let ctx = egui::Context::default();
        let (texts, toggled) = panel_frame(&ctx, &spec(DC_BAND), true, Vec::new());
        assert!(!toggled);
        let (_, heading) = texts
            .iter()
            .find(|(text, _)| text == "Specifications")
            .unwrap();
        for at in [
            *heading + vec2(4.0, 4.0),
            Pos2::new(COLUMN_WIDTH - 12.0, heading.y + 4.0),
        ] {
            let (_, toggled) = panel_frame(&ctx, &spec(DC_BAND), true, click_at(at));
            assert!(toggled, "a click at {at:?} was missed");
        }
    }

    /// The title sits level with the fields under it, as the Statistics
    /// heading does; the fold triangle is at the other end of the row.
    #[test]
    fn the_title_is_level_with_the_fields() {
        let (texts, _) = panel_frame(&egui::Context::default(), &spec(DC_BAND), true, Vec::new());
        let x = |line: &str| {
            texts
                .iter()
                .find(|(text, _)| text == line)
                .map(|(_, at)| at.x)
                .unwrap_or_else(|| panic!("no {line:?} in {texts:?}"))
        };
        assert_eq!(x("Specifications"), x("Resolution  0.01mV"));
    }

    /// LPF V's one band holds only from 40Hz to 100Hz; the panel says so.
    #[test]
    fn a_lone_band_keeps_its_qualifier() {
        const LPF_BAND: &[AccuracyBand] = &[AccuracyBand {
            freq_range: Some("40Hz~100Hz (LPF)"),
            accuracy: "3.0%+50",
        }];
        let texts = panel_texts(&spec(LPF_BAND), true);
        assert!(
            texts
                .iter()
                .any(|t| t == "Accuracy  \u{00B1}(3.0%+50)  40Hz~100Hz (LPF)"),
            "{texts:?}"
        );
    }

    #[test]
    fn a_dc_band_shows_the_figure_alone() {
        let texts = panel_texts(&spec(DC_BAND), true);
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
