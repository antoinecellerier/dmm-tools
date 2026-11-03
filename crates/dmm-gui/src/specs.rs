use dmm_lib::specs::{ModeSpecInfo, SpecInfo};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Color32, RichText, Ui};
use std::borrow::Cow;

use crate::settings::SpecFields;

/// Render a "Manual ↗" hyperlink with a consistent hover tooltip.
fn manual_link(ui: &mut Ui, l10n: &Localizer, url: &str, font_size: f32, color: Color32) {
    ui.hyperlink_to(
        RichText::new(tr!(l10n, "specs-manual"))
            .font(egui::FontId::proportional(font_size))
            .color(color),
        url,
    )
    .on_hover_text(tr!(l10n, "specs-manual.hover"));
}

/// Spec text `english` of `kind` in `l10n`'s language, or as the library
/// wrote it when `l10n` is `None`. A translation shows only where it keeps
/// the English's numbers ([`Localizer::spec_text`]).
fn spec_text<'a>(l10n: Option<&'a Localizer>, kind: TextKind, english: &'a str) -> Cow<'a, str> {
    match l10n {
        Some(l10n) => l10n.spec_text(kind, english),
        None => Cow::Borrowed(english),
    }
}

/// The English of spec text drawn translated, for its hover: the guard on
/// numbers can't catch a flipped comparison or a lost "typical". `None`
/// where the text is drawn as the library wrote it.
fn english_hover(shown: &str, english: &str) -> Option<String> {
    (shown != english).then(|| english.to_string())
}

/// Put `english` on `response`'s hover, where there is one.
fn hover_english(response: egui::Response, english: Option<String>) {
    if let Some(english) = english {
        response.on_hover_text(english);
    }
}

/// An accuracy as the manual prints it: a figure as `±(0.1%+5)`, a word
/// accuracy ("Not Specified", "Only for reference") as it stands, in
/// `l10n`'s language ([`spec_text`]). So does a response in dB ("3dB"): a
/// bandwidth limit, not a tolerance either side.
fn accuracy_text(l10n: Option<&Localizer>, accuracy: &str) -> String {
    if accuracy.starts_with(|c: char| c.is_ascii_digit()) && !accuracy.ends_with("dB") {
        format!("\u{00B1}({accuracy})")
    } else {
        spec_text(l10n, TextKind::Band, accuracy).into_owned()
    }
}

/// Build a compact single-line accuracy string from the spec's accuracy bands.
/// For a single band: `±(0.1%+5)`. For multiple bands: first band only with its
/// frequency range appended, e.g. `±(0.1%+5) 45Hz~1kHz`. In `l10n`'s
/// language, or as the library wrote it for `None`.
///
/// Returns `None` when the spec carries no accuracy bands — modes such as
/// continuity and diode have no accuracy figure in the manual and ship an
/// empty slice.
fn compact_accuracy_str(l10n: Option<&Localizer>, spec: &SpecInfo) -> Option<String> {
    let (first, rest) = spec.accuracy.split_first()?;
    if rest.is_empty() {
        Some(accuracy_text(l10n, first.accuracy))
    } else {
        let freq = spec_text(l10n, TextKind::Band, first.freq_range.unwrap_or(""));
        Some(format!("{} {freq}", accuracy_text(l10n, first.accuracy)))
    }
}

/// The English of the one-line summary's accuracy, for its hover, where the
/// summary shows it translated; `None` with the accuracy turned off.
fn summary_hover(l10n: &Localizer, spec: Option<&SpecInfo>, fields: SpecFields) -> Option<String> {
    let spec = spec.filter(|_| fields.accuracy)?;
    let english = compact_accuracy_str(None, spec)?;
    english_hover(&compact_accuracy_str(Some(l10n), spec)?, &english)
}

/// How a one-line summary captions its figures.
#[derive(Debug, Clone, Copy)]
enum Captions {
    /// The folded panel and the narrow layout: "Res: 0.01mV".
    Short,
    /// The big meter: "Resolution 0.01mV".
    Long,
}

/// Build the summary parts vector used by compact and inline layouts: the
/// resolution and accuracy only, as an unlabelled impedance reads as noise
/// on one line; the side panel carries the rest. A field turned off in
/// Settings is left out here too. A reading with no range row (`spec` is
/// `None`) has no parts.
fn build_spec_parts(
    l10n: &Localizer,
    spec: Option<&SpecInfo>,
    fields: SpecFields,
    captions: Captions,
) -> Vec<String> {
    let Some(spec) = spec else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    // An empty resolution is one the manual doesn't print.
    if fields.resolution && !spec.resolution.is_empty() {
        let resolution = spec.resolution;
        parts.push(
            match captions {
                Captions::Short => tr!(l10n, "specs-resolution-short", resolution = resolution),
                Captions::Long => tr!(l10n, "specs-inline-resolution", resolution = resolution),
            }
            .into_owned(),
        );
    }
    if fields.accuracy
        && let Some(accuracy) = compact_accuracy_str(Some(l10n), spec)
    {
        parts.push(
            match captions {
                Captions::Short => tr!(l10n, "specs-accuracy-short", accuracy = &accuracy),
                Captions::Long => tr!(l10n, "specs-inline-accuracy", accuracy = &accuracy),
            }
            .into_owned(),
        );
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

/// Specs panel for the wide (side panel) layout, under a heading that folds
/// it to the narrow layout's one-line summary. Unfolded, it shows the
/// `fields` the user picked in Settings; a reading with no range row (`spec`
/// is `None`) has its mode's impedance and notes only.
///
/// The caller owns `expanded`, a saved setting: this returns `true` when the
/// heading was clicked this frame, and the caller flips it. `heading` is the
/// palette's heading colour, which the title is drawn in.
#[expect(
    clippy::too_many_arguments,
    reason = "the panel draws from separate app state"
)]
pub fn show_specs(
    ui: &mut Ui,
    l10n: &Localizer,
    spec: Option<&SpecInfo>,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
    expanded: bool,
    fields: SpecFields,
    heading: Color32,
) -> bool {
    // The title where it always sat, egui's fold triangle at the column's
    // right edge, centred under the big-meter toggle, and the whole row one
    // target.
    let weak = ui.visuals().weak_text_color();
    let name = tr!(l10n, "specs-heading");
    let title = egui::WidgetText::from(
        RichText::new(name.as_ref())
            .font(egui::FontId::proportional(11.0 * scale))
            .color(heading),
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
            egui::Role::DisclosureTriangle,
            ui.is_enabled(),
            name.as_ref(),
        )
    });
    if ui.is_rect_visible(rect) {
        let title_pos = egui::pos2(rect.left(), rect.center().y - title.size().y / 2.0);
        ui.painter().galley(title_pos, title, heading);
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
        tr!(l10n, "specs-heading.hover-fold")
    } else {
        tr!(l10n, "specs-heading.hover-unfold")
    };
    let toggled = response.on_hover_text(tooltip).clicked();
    if expanded {
        show_specs_body(ui, l10n, spec, mode_spec, manual_url, scale, fields);
    } else {
        show_specs_compact(ui, l10n, spec, manual_url, fields);
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

/// The picked `fields` the unfolded panel shows under its heading, then the
/// Manual link.
fn show_specs_body(
    ui: &mut Ui,
    l10n: &Localizer,
    spec: Option<&SpecInfo>,
    mode_spec: Option<&ModeSpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
    fields: SpecFields,
) {
    let main_font = 12.0 * scale;
    let sub_font = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();

    if let Some(spec) = spec {
        if fields.resolution && !spec.resolution.is_empty() {
            ui.label(
                RichText::new(tr!(l10n, "specs-resolution", resolution = spec.resolution))
                    .font(egui::FontId::proportional(main_font)),
            );
        }

        // Accuracy — omitted when turned off in Settings, and for modes that
        // have no accuracy figure (continuity, diode), which ship an empty
        // band slice.
        match spec.accuracy {
            _ if !fields.accuracy => {}
            [] => {}
            [single] => {
                let accuracy = accuracy_text(Some(l10n), single.accuracy);
                let figure = RichText::new(tr!(l10n, "specs-accuracy", accuracy = &accuracy))
                    .font(egui::FontId::proportional(main_font));
                // The figure and its qualifier, for the hover.
                let line = |l10n| match single.freq_range {
                    Some(freq) => format!(
                        "{}  {}",
                        accuracy_text(l10n, single.accuracy),
                        spec_text(l10n, TextKind::Band, freq)
                    ),
                    None => accuracy_text(l10n, single.accuracy),
                };
                let english = english_hover(&line(Some(l10n)), &line(None));
                let response = match single.freq_range {
                    // A lone band keeps its qualifier (LPF V's "40Hz~100Hz
                    // (LPF)"), dimmed as in the band list. One label at one
                    // size, so the row keeps a plain label's height and the
                    // two parts share a baseline.
                    Some(freq) => {
                        let freq = l10n.spec_text(TextKind::Band, freq);
                        let mut job = egui::text::LayoutJob::default();
                        let style = ui.style();
                        let font = egui::FontSelection::Default;
                        figure.append_to(&mut job, style, font.clone(), egui::Align::BOTTOM);
                        RichText::new(format!("  {freq}"))
                            .font(egui::FontId::proportional(main_font))
                            .color(weak)
                            .append_to(&mut job, style, font, egui::Align::BOTTOM);
                        ui.label(job)
                    }
                    None => ui.label(figure),
                };
                hover_english(response, english);
            }
            bands => {
                ui.label(
                    RichText::new(tr!(l10n, "specs-accuracy-bands"))
                        .font(egui::FontId::proportional(main_font)),
                );
                for band in bands {
                    let line = |l10n| {
                        let freq = band.freq_range.map_or(Cow::Borrowed(crate::NO_DATA), |f| {
                            spec_text(l10n, TextKind::Band, f)
                        });
                        format!("{freq}  {}", accuracy_text(l10n, band.accuracy))
                    };
                    let shown = line(Some(l10n));
                    let english = english_hover(&shown, &line(None));
                    let response = ui.label(
                        RichText::new(format!("  {shown}"))
                            .font(egui::FontId::proportional(sub_font))
                            .color(weak),
                    );
                    hover_english(response, english);
                }
            }
        }
    }

    // Input impedance and notes
    if let Some(ms) = mode_spec {
        if fields.input_impedance
            && let Some(z) = ms.input_impedance
        {
            let shown = l10n.spec_text(TextKind::Input, z);
            let english = english_hover(&shown, z);
            let response = ui.label(
                RichText::new(tr!(l10n, "specs-input-impedance", impedance = shown))
                    .font(egui::FontId::proportional(main_font)),
            );
            hover_english(response, english);
        }
        if fields.notes {
            for note in ms.notes {
                let shown = l10n.spec_text(TextKind::Note, note);
                let english = english_hover(&shown, note);
                let response = ui.label(
                    RichText::new(shown)
                        .font(egui::FontId::proportional(sub_font))
                        .color(weak),
                );
                hover_english(response, english);
            }
        }
    }

    // Manual link
    if let Some(url) = manual_url {
        manual_link(ui, l10n, url, sub_font, weak);
    }
}

/// Compact single-line specs for the narrow layout and the folded panel.
pub fn show_specs_compact(
    ui: &mut Ui,
    l10n: &Localizer,
    spec: Option<&SpecInfo>,
    manual_url: Option<&str>,
    fields: SpecFields,
) {
    let weak = ui.visuals().weak_text_color();
    let sub_font = 11.0;

    // Build a compact string: "Res: 0.01mV  Acc: ±(0.1%+5)"
    let parts = build_spec_parts(l10n, spec, fields, Captions::Short);

    ui.horizontal_wrapped(|ui| {
        // The separator's own spaces set the gap before the link.
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(line) = summary_line(&parts, "  ", manual_url.is_some()) {
            let response = ui.label(
                RichText::new(line)
                    .font(egui::FontId::proportional(sub_font))
                    .color(weak),
            );
            hover_english(response, summary_hover(l10n, spec, fields));
        }
        if let Some(url) = manual_url {
            manual_link(ui, l10n, url, sub_font, weak);
        }
    });
}

/// Inline pipe-separated specs for big meter mode.
pub fn show_specs_inline(
    ui: &mut Ui,
    l10n: &Localizer,
    spec: Option<&SpecInfo>,
    manual_url: Option<&str>,
    scale: f32,
    fields: SpecFields,
) {
    let font_size = 12.0 * scale;
    let weak = ui.visuals().weak_text_color();

    let parts = build_spec_parts(l10n, spec, fields, Captions::Long);

    ui.horizontal_wrapped(|ui| {
        // The separator's own spaces set the gap before the link.
        ui.spacing_mut().item_spacing.x = 0.0;
        if let Some(line) = summary_line(&parts, LINK_SEPARATOR, manual_url.is_some()) {
            let response = ui.label(
                RichText::new(line)
                    .font(egui::FontId::proportional(font_size))
                    .color(weak),
            );
            hover_english(response, summary_hover(l10n, spec, fields));
        }
        if let Some(url) = manual_url {
            manual_link(ui, l10n, url, font_size, weak);
        }
    });
}

/// Render only the manual link (when no spec data is available but a URL exists).
pub fn show_manual_only(ui: &mut Ui, l10n: &Localizer, url: &str, scale: f32) {
    let font_size = 11.0 * scale;
    let weak = ui.visuals().weak_text_color();
    manual_link(ui, l10n, url, font_size, weak);
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

    /// The folded panel's summary parts, in English.
    fn short_parts(spec: Option<&SpecInfo>, fields: SpecFields) -> Vec<String> {
        build_spec_parts(&crate::l10n::english(), spec, fields, Captions::Short)
    }

    /// The side column's width at the default window size.
    const COLUMN_WIDTH: f32 = 240.0;

    /// One frame of the full panel on `ctx`, with `AC_MODE`'s impedance and
    /// note and a manual URL: the text it draws, each with where, and whether
    /// the heading was clicked.
    fn panel_frame(
        ctx: &egui::Context,
        spec: &SpecInfo,
        expanded: bool,
        fields: SpecFields,
        events: Vec<egui::Event>,
    ) -> (Vec<(String, Pos2)>, bool) {
        let english = crate::l10n::english();
        panel_frame_in(ctx, &english, spec, expanded, fields, events, 0.0)
    }

    /// [`panel_frame`] in `l10n`'s language, at `time` seconds.
    fn panel_frame_in(
        ctx: &egui::Context,
        l10n: &Localizer,
        spec: &SpecInfo,
        expanded: bool,
        fields: SpecFields,
        events: Vec<egui::Event>,
        time: f64,
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
            time: Some(time),
            ..Default::default()
        };
        let mut toggled = false;
        let mut out = ctx.run_ui(input, |ui| {
            toggled = show_specs(
                ui,
                l10n,
                Some(spec),
                Some(&AC_MODE),
                Some("https://example.com/manual"),
                1.0,
                expanded,
                fields,
                Color32::GRAY,
            );
        });
        out.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &out.shapes {
            collect(&clipped.shape, &mut texts);
        }
        (texts, toggled)
    }

    /// A French spec text translation: the note and the LPF band keep their
    /// numbers, the impedance changes its own, and a word accuracy is
    /// translated.
    fn french() -> Localizer {
        use dmm_shared::l10n::{Language, Options, Resource};
        static FRENCH: Language = Language {
            id: "fr",
            native_name: "Français",
            reviewed: true,
        };
        let lib = Resource {
            locale: "fr",
            path: "lib.ftl",
            source: "note-shorted-leads-residual-10-digits = Cordons en court-circuit : \
                     résidu ≤10 points\n\
                     input-about-10mohm = Environ 100 MΩ\n\
                     band-40hz-100hz-lpf = 40 Hz~100 Hz (filtre)\n\
                     band-only-for-reference = Pour référence\n",
        };
        let resources = [crate::l10n::english_resources(), vec![lib]].concat();
        Localizer::new(&FRENCH, &resources, &Options::default())
    }

    const LPF_BAND: &[AccuracyBand] = &[AccuracyBand {
        freq_range: Some("40Hz~100Hz (LPF)"),
        accuracy: "3.0%+50",
    }];

    /// Spec text whose translation keeps its numbers is drawn translated;
    /// one that changed a number is drawn in English.
    #[test]
    fn spec_text_is_drawn_translated_where_its_numbers_hold() {
        let ctx = crate::app::styled_context();
        let l10n = french();
        let (texts, _) = panel_frame_in(
            &ctx,
            &l10n,
            &spec(LPF_BAND),
            true,
            SpecFields::default(),
            Vec::new(),
            0.0,
        );
        let texts: Vec<String> = texts.into_iter().map(|(t, _)| t).collect();
        for shown in [
            "Cordons en court-circuit : résidu ≤10 points",
            "Input Z  About 10M\u{03A9}",
            "Accuracy  \u{00B1}(3.0%+50)  40 Hz~100 Hz (filtre)",
        ] {
            assert!(
                texts.iter().any(|t| t == shown),
                "no {shown:?} in {texts:?}"
            );
        }
    }

    /// Where the text drawn differs from the English, the English is on
    /// hover; where it doesn't, there is no hover.
    #[test]
    fn translated_spec_text_has_the_english_on_hover() {
        let ctx = crate::app::styled_context();
        let l10n = french();
        let frame = |events, time| {
            let (texts, _) = panel_frame_in(
                &ctx,
                &l10n,
                &spec(LPF_BAND),
                true,
                SpecFields::default(),
                events,
                time,
            );
            texts
        };
        let texts = frame(Vec::new(), 0.0);
        let at = |line: &str| {
            texts
                .iter()
                .find(|(text, _)| text == line)
                .map(|(_, at)| *at + vec2(4.0, 4.0))
                .unwrap_or_else(|| panic!("no {line:?} in {texts:?}"))
        };
        let note = at("Cordons en court-circuit : résidu ≤10 points");
        let input = at("Input Z  About 10M\u{03A9}");
        let hover = |pos, start: f64| {
            let mut texts = Vec::new();
            // The pointer comes to rest there; the tooltip follows once
            // it has been still a while.
            for i in 1..=4 {
                let events = if i == 1 {
                    vec![egui::Event::PointerMoved(pos)]
                } else {
                    Vec::new()
                };
                texts = frame(events, start + f64::from(i));
            }
            texts.into_iter().map(|(t, _)| t).collect::<Vec<_>>()
        };
        let shown = hover(note, 0.0);
        assert!(
            shown.iter().any(|t| t == AC_MODE.notes[0]),
            "no English note in {shown:?}"
        );
        let shown = hover(input, 10.0);
        assert!(
            !shown.iter().any(|t| t == "About 10M\u{03A9}"),
            "an English input drew its English hover: {shown:?}"
        );
    }

    /// The one-line summary carries the English of a translated band on
    /// hover, and English needs none.
    #[test]
    fn the_summary_hover_carries_the_english_band() {
        let fields = SpecFields::default();
        let l10n = french();
        // A lone band's summary is its figure alone, nothing translated.
        assert_eq!(summary_hover(&l10n, Some(&spec(LPF_BAND)), fields), None);
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
            compact_accuracy_str(Some(&l10n), &spec(WORD_FIRST)).as_deref(),
            Some("Pour référence 10k~100kHz")
        );
        assert_eq!(
            summary_hover(&l10n, Some(&spec(WORD_FIRST)), fields).as_deref(),
            Some("Only for reference 10k~100kHz")
        );
        let no_accuracy = SpecFields {
            accuracy: false,
            ..fields
        };
        assert_eq!(
            summary_hover(&l10n, Some(&spec(WORD_FIRST)), no_accuracy),
            None
        );
        let english = crate::l10n::english();
        assert_eq!(
            summary_hover(&english, Some(&spec(WORD_FIRST)), fields),
            None
        );
    }

    /// The text the full panel draws for `spec`, unfolded or folded, with
    /// `fields` picked.
    fn panel_texts(spec: &SpecInfo, expanded: bool, fields: SpecFields) -> Vec<String> {
        let (texts, _) = panel_frame(
            &crate::app::styled_context(),
            spec,
            expanded,
            fields,
            Vec::new(),
        );
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
        let texts = panel_texts(&spec(AC_BANDS), true, SpecFields::default());
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
        let texts = panel_texts(&spec(AC_BANDS), false, SpecFields::default());
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

    /// The one-line summary leaves out a field turned off in Settings, and
    /// with neither left the Manual link stands alone.
    #[test]
    fn the_summary_leaves_out_fields_turned_off() {
        let all = SpecFields::default();
        let no_accuracy = SpecFields {
            accuracy: false,
            ..all
        };
        let no_resolution = SpecFields {
            resolution: false,
            ..all
        };
        let neither = SpecFields {
            resolution: false,
            accuracy: false,
            ..all
        };
        let row = spec(AC_BANDS);
        assert_eq!(short_parts(Some(&row), no_accuracy), ["Res: 0.01mV"]);
        assert_eq!(
            short_parts(Some(&row), no_resolution),
            ["Acc: \u{00B1}(0.5%+30) 45Hz~1kHz"]
        );
        assert!(short_parts(Some(&row), neither).is_empty());
        let texts = panel_texts(&row, false, neither);
        assert!(texts.iter().any(|t| t == MANUAL), "{texts:?}");
        assert!(!texts.iter().any(|t| t.starts_with("Res:")), "{texts:?}");
    }

    /// A field turned off in Settings takes its own lines out of the unfolded
    /// panel, and nothing else; the Manual link stays.
    #[test]
    fn a_field_turned_off_hides_only_its_lines() {
        let lines = [
            "Resolution  0.01mV",
            "Accuracy",
            "  45Hz~1kHz  \u{00B1}(0.5%+30)",
            "  1kHz~10kHz  \u{00B1}(1.5%+30)",
            "Input Z  About 10M\u{03A9}",
            AC_MODE.notes[0],
        ];
        let all = SpecFields::default();
        let cases: [(SpecFields, &[&str]); 4] = [
            (
                SpecFields {
                    resolution: false,
                    ..all
                },
                &lines[0..1],
            ),
            (
                SpecFields {
                    accuracy: false,
                    ..all
                },
                &lines[1..4],
            ),
            (
                SpecFields {
                    input_impedance: false,
                    ..all
                },
                &lines[4..5],
            ),
            (
                SpecFields {
                    notes: false,
                    ..all
                },
                &lines[5..6],
            ),
        ];
        for (fields, hidden) in cases {
            let texts = panel_texts(&spec(AC_BANDS), true, fields);
            for line in lines {
                assert_eq!(
                    texts.iter().any(|t| t == line),
                    !hidden.contains(&line),
                    "{line:?} with {fields:?}: {texts:?}"
                );
            }
            assert!(texts.iter().any(|t| t == MANUAL), "{texts:?}");
        }
    }

    /// A row whose manual prints no resolution shows its accuracy alone,
    /// unfolded and on one line.
    #[test]
    fn a_row_with_no_resolution_shows_only_its_accuracy() {
        let row = SpecInfo {
            resolution: "",
            accuracy: DC_BAND,
        };
        let texts = panel_texts(&row, true, SpecFields::default());
        assert!(
            texts.iter().any(|t| t == "Accuracy  \u{00B1}(0.1%+5)"),
            "{texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.starts_with("Resolution")),
            "{texts:?}"
        );
        assert_eq!(
            short_parts(Some(&row), SpecFields::default()),
            ["Acc: \u{00B1}(0.1%+5)"]
        );
    }

    /// The panel doesn't fold itself: a click anywhere on the heading row,
    /// the title or the triangle at the far end, is reported, and the caller
    /// flips the saved setting.
    #[test]
    fn a_click_on_the_heading_is_reported() {
        let ctx = crate::app::styled_context();
        let fields = SpecFields::default();
        let (texts, toggled) = panel_frame(&ctx, &spec(DC_BAND), true, fields, Vec::new());
        assert!(!toggled);
        let (_, heading) = texts
            .iter()
            .find(|(text, _)| text == "Specifications")
            .unwrap();
        for at in [
            *heading + vec2(4.0, 4.0),
            Pos2::new(COLUMN_WIDTH - 12.0, heading.y + 4.0),
        ] {
            let (_, toggled) = panel_frame(&ctx, &spec(DC_BAND), true, fields, click_at(at));
            assert!(toggled, "a click at {at:?} was missed");
        }
    }

    /// The title sits level with the fields under it, as the Statistics
    /// heading does; the fold triangle is at the other end of the row.
    #[test]
    fn the_title_is_level_with_the_fields() {
        let (texts, _) = panel_frame(
            &crate::app::styled_context(),
            &spec(DC_BAND),
            true,
            SpecFields::default(),
            Vec::new(),
        );
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
        let texts = panel_texts(&spec(LPF_BAND), true, SpecFields::default());
        assert!(
            texts
                .iter()
                .any(|t| t == "Accuracy  \u{00B1}(3.0%+50)  40Hz~100Hz (LPF)"),
            "{texts:?}"
        );
    }

    #[test]
    fn a_dc_band_shows_the_figure_alone() {
        let texts = panel_texts(&spec(DC_BAND), true, SpecFields::default());
        assert!(
            texts.iter().any(|t| t == "Accuracy  \u{00B1}(0.1%+5)"),
            "{texts:?}"
        );
    }

    /// Continuity and diode ship `accuracy: &[]`; the compact renderers must not
    /// index into it.
    #[test]
    fn empty_accuracy_yields_no_string() {
        assert_eq!(compact_accuracy_str(None, &spec(&[])), None);
    }

    #[test]
    fn empty_accuracy_omits_the_accuracy_part() {
        let parts = short_parts(Some(&spec(&[])), SpecFields::default());
        assert_eq!(parts, vec!["Res: 0.01mV".to_string()]);
    }

    /// A reading with no range row has no parts, so the one-line layouts
    /// show the Manual link alone.
    #[test]
    fn no_row_leaves_only_the_manual_link() {
        assert!(short_parts(None, SpecFields::default()).is_empty());
    }

    /// The Manual link is set off from the summary, and stands alone when
    /// there is none.
    #[test]
    fn the_summary_is_separated_from_the_manual_link() {
        let parts = short_parts(Some(&spec(DC_BAND)), SpecFields::default());
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
        let parts = short_parts(Some(&spec(DC_BAND)), SpecFields::default());
        assert_eq!(
            parts,
            vec![
                "Res: 0.01mV".to_string(),
                "Acc: \u{00B1}(0.1%+5)".to_string(),
            ]
        );
    }

    /// The big meter's line spells its captions out.
    #[test]
    fn the_big_meter_line_has_long_captions() {
        let parts = build_spec_parts(
            &crate::l10n::english(),
            Some(&spec(AC_BANDS)),
            SpecFields::default(),
            Captions::Long,
        );
        assert_eq!(
            summary_line(&parts, LINK_SEPARATOR, true).as_deref(),
            Some("Resolution 0.01mV  |  Accuracy \u{00B1}(0.5%+30) 45Hz~1kHz  |  ")
        );
    }

    #[test]
    fn single_band_has_no_frequency_suffix() {
        assert_eq!(
            compact_accuracy_str(None, &spec(DC_BAND)).as_deref(),
            Some("\u{00B1}(0.1%+5)")
        );
    }

    #[test]
    fn multi_band_appends_first_frequency_range() {
        assert_eq!(
            compact_accuracy_str(None, &spec(AC_BANDS)).as_deref(),
            Some("\u{00B1}(0.5%+30) 45Hz~1kHz")
        );
    }

    #[test]
    fn a_figure_is_wrapped_in_plus_minus() {
        assert_eq!(accuracy_text(None, "0.4%+30"), "\u{00B1}(0.4%+30)");
        assert_eq!(accuracy_text(None, "2.5%"), "\u{00B1}(2.5%)");
    }

    /// The Brymen BM820s manual's "3dB" bounds the frequency response at
    /// the top of a band; a ± would read as a tolerance either side.
    #[test]
    fn a_db_response_stands_as_printed() {
        assert_eq!(accuracy_text(None, "3dB"), "3dB");
    }

    /// "Not Specified", "Only for reference" and "reference only" are words
    /// the manual prints in the accuracy column, not a tolerance.
    #[test]
    fn a_word_accuracy_stands_as_printed() {
        for word in ["Not Specified", "Only for reference", "reference only"] {
            assert_eq!(accuracy_text(None, word), word);
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
            compact_accuracy_str(None, &spec(WORD_FIRST)).as_deref(),
            Some("Only for reference 10k~100kHz")
        );
    }
}
