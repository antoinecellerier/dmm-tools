use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;
use crate::settings::{ColorPreset, HexColor, PaletteOverrides, ThemeChoice, ThemeMode};
use crate::theme::links;
use crate::theme::named;
use crate::theme::{PaletteField, ThemeColors};

use crate::app::App;

use super::{Chip, chip_row};

impl App {
    /// The **Theme** row: Dark, Light and System, then the named themes,
    /// and a caption naming any theme file that was skipped.
    pub(super) fn show_theme_row(&mut self, ui: &mut Ui) {
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        ui.horizontal_wrapped(|ui| {
            let active = self.settings.active_theme().map(|t| t.name.as_str());
            let from_flag = self.settings.overrides.has_theme();
            let overrides = &self.settings.color_overrides;
            let customized = |mode: ThemeMode| match mode {
                ThemeMode::Dark => overrides.dark != PaletteOverrides::default(),
                ThemeMode::Light => overrides.light != PaletteOverrides::default(),
                ThemeMode::System => false,
            };
            let modes = [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System].map(|mode| {
                let selected = active.is_none() && self.settings.theme == mode;
                // Dark and Light keep their own customizations while a
                // named theme is on; say so on the chip, as a theme's chip
                // does, picked or not, so a customized Light isn't taken
                // for plain Light.
                let state = if selected && from_flag {
                    "pinned"
                } else if customized(mode) {
                    "customized"
                } else {
                    "plain"
                };
                let (label, tooltip) = match mode {
                    ThemeMode::Dark => (
                        tr!(l10n, "colors-theme-dark", state = state),
                        tr!(l10n, "colors-theme-dark.hover"),
                    ),
                    ThemeMode::Light => (
                        tr!(l10n, "colors-theme-light", state = state),
                        tr!(l10n, "colors-theme-light.hover"),
                    ),
                    ThemeMode::System => (
                        tr!(l10n, "colors-theme-system", state = state),
                        tr!(l10n, "colors-theme-system.hover"),
                    ),
                };
                Chip {
                    value: ThemeChoice::Mode(mode),
                    selected,
                    label: label.into_owned(),
                    tooltip: tooltip.into_owned(),
                }
            });
            let user = self.settings.user_themes.clone();
            let themes_dir = named::user_dir();
            let named = named::listed(&user.themes).into_iter().map(|theme| {
                let selected = active == Some(theme.name.as_str());
                let customized = self
                    .settings
                    .color_overrides
                    .for_theme(&theme.name)
                    .is_some_and(|t| *t != PaletteOverrides::default());
                // Customized whether picked or not, as Dark and Light are.
                let name = theme.name.as_str();
                let label = match (selected && from_flag, customized) {
                    (true, _) => tr!(l10n, "colors-theme-named-pinned", name = name).into_owned(),
                    (false, true) => {
                        tr!(l10n, "colors-theme-named-customized", name = name).into_owned()
                    }
                    (false, false) => theme.name.clone(),
                };
                let mode = if theme.dark { "dark" } else { "light" };
                // A user's theme names its file in full, so it can be found
                // to edit, share or delete.
                let tooltip = match (&theme.file, &themes_dir) {
                    (Some(file), Some(dir)) => tr!(
                        l10n,
                        "colors-theme-file.hover",
                        mode = mode,
                        path = dir.join(file).display().to_string(),
                    ),
                    (Some(file), None) => tr!(
                        l10n,
                        "colors-theme-file-unplaced.hover",
                        mode = mode,
                        file = file,
                    ),
                    (None, _) => tr!(l10n, "colors-theme-builtin.hover", mode = mode),
                };
                Chip {
                    value: ThemeChoice::Named(theme.name.clone()),
                    selected,
                    label,
                    tooltip: tooltip.into_owned(),
                }
            });
            let caption = tr!(l10n, "colors-theme");
            if let Some(choice) = chip_row(ui, &caption, modes.into_iter().chain(named)) {
                self.settings.pick_theme(choice);
            }
            let skipped = &self.settings.user_themes.skipped;
            if !skipped.is_empty() {
                let mut list: Vec<String> = themes_dir
                    .iter()
                    .map(|dir| {
                        let folder = dir.display().to_string();
                        tr!(l10n, "colors-skipped-folder", folder = folder).into_owned()
                    })
                    .collect();
                list.extend(skipped.iter().map(|(file, reason)| reason.line(l10n, file)));
                let caption = tr!(l10n, "colors-skipped", count = skipped.len());
                ui.label(
                    RichText::new(caption)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .on_hover_text(list.join("\n"));
            }
        });
    }

    /// The **Colors** row: the palette presets for Dark, Light and System.
    pub(super) fn show_color_preset_row(&mut self, ui: &mut Ui) {
        // Only the presets' own customizations: a named theme's are kept
        // when the preset changes, so they don't count here.
        let has_overrides = self.settings.color_overrides.dark != PaletteOverrides::default()
            || self.settings.color_overrides.light != PaletteOverrides::default();
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        ui.horizontal_wrapped(|ui| {
            let chips = [
                ColorPreset::Default,
                ColorPreset::HighContrast,
                ColorPreset::ColorblindSafe,
            ]
            .into_iter()
            .map(|preset| {
                let selected = self.settings.color_preset == preset;
                let state = if selected && has_overrides {
                    "customized"
                } else {
                    "plain"
                };
                let (label, tooltip) = match preset {
                    ColorPreset::Default => (
                        tr!(l10n, "colors-preset-default", state = state),
                        tr!(l10n, "colors-preset-default.hover"),
                    ),
                    ColorPreset::HighContrast => (
                        tr!(l10n, "colors-preset-high-contrast", state = state),
                        tr!(l10n, "colors-preset-high-contrast.hover"),
                    ),
                    ColorPreset::ColorblindSafe => (
                        tr!(l10n, "colors-preset-colorblind", state = state),
                        tr!(l10n, "colors-preset-colorblind.hover"),
                    ),
                };
                Chip {
                    value: preset,
                    selected,
                    label: label.into_owned(),
                    tooltip: tooltip.into_owned(),
                }
            });
            let caption = tr!(l10n, "colors-colors");
            // The preset in use clicked again keeps its customizations: only
            // a switch to another preset clears them.
            if let Some(preset) = chip_row(ui, &caption, chips)
                && preset != self.settings.color_preset
            {
                self.settings.color_preset = preset;
                // Clear the presets' overrides when switching presets.
                self.settings.color_overrides.dark = PaletteOverrides::default();
                self.settings.color_overrides.light = PaletteOverrides::default();
                self.applied.ui_colors = None; // force reapply
            }
            if has_overrides {
                ui.label(
                    RichText::new(tr!(l10n, "colors-other-preset-clears"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });
    }

    /// Show the collapsible color customization section.
    pub(super) fn show_color_customization(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();

        // A named theme's own palette only in its own mode, as
        // `Settings::color_tweaks` has it: on the frame a chip is picked, the
        // UI may still be in the other.
        let named = self
            .settings
            .active_theme()
            .filter(|t| t.dark == dark)
            .map(|t| t.name.clone());
        let editing = match &named {
            Some(name) => tr!(l10n, "colors-editing-named", name = name),
            None => tr!(
                l10n,
                "colors-editing",
                mode = if dark { "dark" } else { "light" }
            ),
        };

        // The header's id, kept from the frame before: `CollapsingHeader`
        // makes it inside a `ui.vertical` of its own, so it can't be worked
        // out here, but it doesn't change from one frame to the next.
        let header_key = ui.scope_id().with("customize_colors_header");
        let header_id: Option<egui::Id> = ui.data(|d| d.get_temp(header_key));
        let clip = ui.clip_rect();
        // Its id from a salt of its own rather than from its text, which
        // changes with the language.
        let collapsing = egui::CollapsingHeader::new(tr!(l10n, "colors-customize"))
            .id_salt("Customize colors")
            .default_open(false)
            .show(ui, |ui| {
                // Beside the header rather than on a line of its own under
                // it, but created first in the body: a screen reader then
                // reaches it straight after the header, before the swatches.
                // In a child the body never allocates for, as allocating
                // would move the cursor back up; clipped to the panel rather
                // than to the body, which starts below the header's line.
                if let Some(header) = header_id.and_then(|id| ui.ctx().read_response(id)) {
                    let header = header.rect;
                    let beside = egui::Rect::from_min_max(
                        egui::pos2(header.right() + ui.spacing().item_spacing.x, header.top()),
                        egui::pos2(ui.max_rect().right(), header.bottom()),
                    );
                    let mut caption = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(beside)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    caption.set_clip_rect(beside.intersect(clip));
                    let weak = caption.visuals().weak_text_color();
                    caption.add(
                        egui::Label::new(RichText::new(&*editing).small().color(weak)).truncate(),
                    );
                }

                let mut changed = false;
                // What every swatch shows: followers already moved with
                // their anchors.
                let current = self.settings.theme_colors(dark);
                // And what it is before the user's changes: a warning is for
                // what they changed, not what the palette chose.
                let base = self.settings.uncustomized_colors(dark);
                // A named theme's tweaks are edited on a copy and put back
                // only when they change, so that just showing the swatches
                // doesn't leave an empty entry behind for every theme.
                let mut tweaks = named
                    .as_ref()
                    .and_then(|name| self.settings.color_overrides.for_theme(name))
                    .cloned()
                    .unwrap_or_default();
                let overrides = match named {
                    Some(_) => &mut tweaks,
                    None => self.settings.color_overrides.for_mode_mut(dark),
                };

                // In families: an anchor on its row, and the colours that
                // follow it on an indented row beneath, so what moves with
                // what shows without hovering. Each swatch's label, tooltip
                // and override slot come from the enum, so a colour can't be
                // listed with another's tooltip or wired to its override.
                // The captions' column as wide as the widest of them, so the
                // swatches line up whatever the language makes of them.
                let width = [Group::Ui, Group::Graph, Group::Status]
                    .map(|group| {
                        egui::WidgetText::from(group.caption(l10n))
                            .into_galley(
                                ui,
                                Some(egui::TextWrapMode::Extend),
                                f32::INFINITY,
                                egui::TextStyle::Body,
                            )
                            .size()
                            .x
                            + ui.spacing().item_spacing.x
                    })
                    .into_iter()
                    .fold(CAPTION_WIDTH, f32::max);
                for &(group, row) in PANEL_ROWS {
                    let caption = group.map_or("".into(), |g| g.caption(l10n));
                    match row {
                        PanelRow::Family(anchor) => {
                            // One row per family, the settings panel being
                            // short on height: the anchor, an arrow, the
                            // colours that follow it — dashed while they do.
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, &caption, width);
                                changed |= color_edit(ui, l10n, anchor, overrides, &current, &base);
                                ui.label(
                                    RichText::new("\u{2192}").color(ui.visuals().weak_text_color()),
                                )
                                .on_hover_text(tr!(
                                    l10n,
                                    "colors-follow-arrow.hover",
                                    anchor = anchor.label(l10n)
                                ));
                                for field in links::followers(anchor) {
                                    changed |=
                                        color_edit(ui, l10n, field, overrides, &current, &base);
                                }
                            });
                        }
                        PanelRow::Fields(fields) => {
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, &caption, width);
                                for &field in fields {
                                    changed |=
                                        color_edit(ui, l10n, field, overrides, &current, &base);
                                }
                            });
                        }
                    }
                }
                if changed && let Some(name) = &named {
                    self.settings.color_overrides.set_for_theme(name, tweaks);
                }

                // Only once there is something to reset or save.
                if self.has_color_changes(dark) {
                    ui.horizontal(|ui| {
                        let reset =
                            ui.button(tr!(l10n, "colors-reset"))
                                .on_hover_text(match &named {
                                    Some(name) => {
                                        tr!(l10n, "colors-reset-named.hover", name = name)
                                    }
                                    None => tr!(l10n, "colors-reset.hover"),
                                });
                        if reset.clicked() {
                            // The button goes with the changes it resets;
                            // focus left on it would fall back to the top of
                            // the window, so hand it to the section's header.
                            if let Some(header) = header_id.filter(|_| reset.has_focus()) {
                                ui.memory_mut(|m| m.request_focus(header));
                            }
                            match &named {
                                Some(name) => self
                                    .settings
                                    .color_overrides
                                    .set_for_theme(name, PaletteOverrides::default()),
                                None => {
                                    *self.settings.color_overrides.for_mode_mut(dark) =
                                        PaletteOverrides::default()
                                }
                            }
                            changed = true;
                        }
                        // On the same row: the section is tall enough already.
                        self.show_theme_save_row(ui);
                    });
                }

                if changed {
                    self.applied.ui_colors = None; // force reapply
                }
            });
        let header_id = collapsing.header_response.id;
        ui.data_mut(|d| d.insert_temp(header_key, header_id));
        // Near the foot of the panel, the section opens below the fold of a
        // short window. Once it has finished opening, scroll as much of it
        // into view as fits, the header first.
        let header = &collapsing.header_response;
        let reveal = header.id.with("reveal");
        if header.clicked() {
            let opened = egui::collapsing_header::CollapsingState::load(ui.ctx(), header.id)
                .is_some_and(|state| state.is_open());
            ui.data_mut(|d| {
                if opened {
                    d.insert_temp(reveal, true);
                } else {
                    d.remove::<bool>(reveal);
                }
            });
        }
        if collapsing.fully_open() && ui.data_mut(|d| d.remove_temp::<bool>(reveal).is_some()) {
            let body = collapsing
                .body_response
                .as_ref()
                .map_or(header.rect, |b| b.rect);
            ui.scroll_to_rect(header.rect.union(body), None);
        }
        // Paint an explicit focus ring on the header when Tab-focused —
        // egui's CollapsingHeader shows only a subtle highlight otherwise,
        // which is easy to miss.
        crate::a11y::paint_focus_ring(ui, &collapsing.header_response);
        collapsing
            .header_response
            .on_hover_text(tr!(l10n, "colors-customize.hover"));
    }
}

/// One row of the Customize colors swatches.
#[derive(Clone, Copy)]
enum PanelRow {
    /// An anchor, then the colours that follow it, on one row.
    Family(PaletteField),
    /// Colours that follow nothing.
    Fields(&'static [PaletteField]),
}

/// The caption a group of swatch rows starts with.
#[derive(Clone, Copy)]
enum Group {
    Ui,
    Graph,
    Status,
}

impl Group {
    fn caption(self, l10n: &Localizer) -> std::borrow::Cow<'_, str> {
        match self {
            Group::Ui => tr!(l10n, "colors-group-ui"),
            Group::Graph => tr!(l10n, "colors-group-graph"),
            Group::Status => tr!(l10n, "colors-group-status"),
        }
    }
}

/// The least width of the swatch rows' captions.
const CAPTION_WIDTH: f32 = 56.0;

/// The swatch rows, top to bottom, the first of each group under its
/// caption. Every palette field is on exactly one: as an anchor, as one of
/// its followers, or in a row of its own
/// (`panel_rows_list_every_field_once`).
const PANEL_ROWS: &[(Option<Group>, PanelRow)] = &[
    (Some(Group::Ui), PanelRow::Family(PaletteField::Background)),
    (None, PanelRow::Family(PaletteField::Text)),
    (None, PanelRow::Fields(&[PaletteField::Accent])),
    (
        Some(Group::Graph),
        PanelRow::Family(PaletteField::GraphLine),
    ),
    (
        None,
        PanelRow::Fields(&[
            PaletteField::GraphGap,
            PaletteField::GraphMean,
            PaletteField::GraphRef,
            PaletteField::GraphCrossing,
            PaletteField::GraphCursor,
            PaletteField::GraphMarker,
        ]),
    ),
    (
        Some(Group::Status),
        PanelRow::Fields(&[
            PaletteField::StatusOk,
            PaletteField::StatusWarning,
            PaletteField::StatusError,
            PaletteField::StatusInactive,
        ]),
    ),
];

/// A row's caption, at one width so the swatches line up beneath it.
fn caption_cell(ui: &mut Ui, caption: &str, width: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.label(caption);
        },
    );
}

/// A colour as the picker's hex field shows it.
fn hex_text(c: egui::Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// A colour typed or pasted as `#RRGGBB`, the `#` optional.
fn parse_hex(text: &str) -> Option<egui::Color32> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(egui::Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// Where a swatch whose relink button was just pressed waits to take focus
/// back on the next frame.
fn relink_refocus_id() -> egui::Id {
    egui::Id::unique("color_relink_refocus")
}

/// Render a color edit button with label. Returns true if the color was
/// changed, or a follower set on its own was linked again.
///
/// A follower its anchor still moves has a dashed outline; one set on its
/// own a solid one and a relink button. A colour under its contrast floor
/// that the user is answerable for — set it, or set its ground — carries a
/// warning with the ratio; a linked follower never does, it keeps its floor.
fn color_edit(
    ui: &mut Ui,
    l10n: &Localizer,
    field: PaletteField,
    overrides: &mut PaletteOverrides,
    current: &ThemeColors,
    base: &ThemeColors,
) -> bool {
    let label = field.label(l10n);
    let anchor = links::anchor(field);
    let set = field.override_slot(overrides).is_some();
    let linked = anchor.is_some() && !set;
    // The colour on screen: this field's override, or where the preset, the
    // theme or its anchor put it.
    let color = current.effective_color(field);

    // Render the swatch as a plain Button with an explicit fill, so we control
    // the open lifecycle. egui's `color_edit_button_srgba` also uses
    // `Popup::menu`, but doesn't move focus into the popup when it opens —
    // keyboard users end up stranded on the settings panel.
    //
    // The swatch and its label are one unit, sized before it is placed so the
    // wrapped group row can move it to the next line. A `ui.horizontal` here
    // claimed the rest of the row and never wrapped, and its overflow widened
    // the whole settings panel — every other row then kept folding at that
    // width instead of the window's.
    let gap = 2.0;
    let btn_size = egui::Vec2::splat(ui.spacing().interact_size.y);
    let text = egui::WidgetText::from(RichText::new(&*label).small());
    let galley = text.clone().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    let unit = egui::vec2(
        btn_size.x + gap + galley.size().x,
        btn_size.y.max(galley.size().y),
    );
    let response = ui.allocate_ui_with_layout(
        unit,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let btn = ui.add(egui::Button::new("").fill(color).min_size(btn_size));
            ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
            btn
        },
    );

    // The swatch's visible content is just a color, which screen readers
    // can't describe — give it the label text as its accessible name.
    let btn_response = response
        .inner
        .on_hover_text(field.tooltip(l10n))
        .a11y_label(&match anchor {
            Some(anchor) if linked => tr!(
                l10n,
                "colors-swatch-linked",
                color = &*label,
                anchor = anchor.label(l10n)
            ),
            _ => label.clone(),
        });
    if ui
        .ctx()
        .data(|d| d.get_temp::<egui::Id>(relink_refocus_id()))
        == Some(btn_response.id)
    {
        ui.ctx()
            .data_mut(|d| d.remove::<egui::Id>(relink_refocus_id()));
        btn_response.request_focus();
    }
    paint_swatch_outline(ui, &btn_response, linked);

    let mut relinked = false;
    if let Some(anchor) = anchor
        && set
        && ui
            .small_button(tr!(l10n, "colors-relink"))
            .on_hover_text(tr!(
                l10n,
                "colors-relink.hover",
                anchor = anchor.label(l10n)
            ))
            .a11y_label(&tr!(
                l10n,
                "colors-relink.a11y",
                color = &*label,
                anchor = anchor.label(l10n)
            ))
            .clicked()
    {
        *field.override_slot(overrides) = None;
        relinked = true;
        // The button goes with the override; keep keyboard focus on the
        // colour it belonged to rather than dropping it to the window's
        // first widget. Next frame: egui hands focus to the button it was
        // clicked on after this runs, and the button is gone by then.
        ui.ctx()
            .data_mut(|d| d.insert_temp(relink_refocus_id(), btn_response.id));
        ui.ctx().request_repaint();
    }
    show_change_warnings(ui, l10n, field, current, base);

    let popup_id = btn_response.id.with("color_popup");
    // "This click is the one that opens the popup" — true only on the click
    // frame *and* only when the popup was closed at the start of the frame.
    // `Popup::menu` flips the memory state inside its own `show` call, so at
    // this point `is_id_open` still returns the pre-toggle value.
    let newly_opened = btn_response.clicked() && !egui::Popup::is_id_open(ui.ctx(), popup_id);

    // Popup has no built-in Esc handling (unlike egui::Modal), so consume
    // Esc manually while the popup is open.
    if egui::Popup::is_id_open(ui.ctx(), popup_id)
        && ui
            .ctx()
            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        egui::Popup::close_id(ui.ctx(), popup_id);
    }

    let picked = egui::Popup::menu(&btn_response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| picker_popup_body(ui, l10n, btn_response.id, newly_opened, color))
        .and_then(|shown| shown.inner);

    keep_focus_with_popup(ui, popup_id, btn_response.id);

    if let Some(color) = picked {
        *field.override_slot(overrides) = Some(HexColor(color));
        return true;
    }

    relinked
}

/// The outline every swatch has, so one the colour of the panel it sits on
/// still shows — dashed while it follows its anchor — and its focus ring.
fn paint_swatch_outline(ui: &Ui, response: &egui::Response, linked: bool) {
    let rect = response.rect;
    let stroke = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    if linked {
        let r = rect.expand(1.0);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ];
        ui.painter()
            .extend(egui::Shape::dashed_line(&corners, stroke, 3.0, 2.0));
    } else {
        ui.painter()
            .rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
    }
    // The fill covers the usual button border, so paint an explicit focus ring
    // when the swatch is keyboard-focused.
    crate::a11y::paint_focus_ring(ui, response);
}

/// The warnings under a swatch whose colour the user's changes brought under
/// its contrast floor or too close to another graph colour.
fn show_change_warnings(
    ui: &mut Ui,
    l10n: &Localizer,
    field: PaletteField,
    current: &ThemeColors,
    base: &ThemeColors,
) {
    // Only where the user's changes brought it under: the presets' own
    // decorative borders sit under the graphical floor on purpose. A linked
    // colour warns too, when no lightness could keep it readable.
    if let Some((ratio, ground, floor)) = links::floor_failure(current, field)
        && links::floor_failure(base, field).is_none()
    {
        let warning = tr!(
            l10n,
            "colors-low-contrast",
            ratio = format!("{ratio:.1}"),
            ground = ground.label(l10n)
        );
        let hover = tr!(
            l10n,
            "colors-low-contrast.hover",
            floor = floor.to_string(),
            ground = ground.label(l10n)
        );
        warning_label(ui, &warning, &hover);
    }

    // Graph colours drawn together have to read apart; a turn of the line can
    // bring an overlay onto a colour that stayed put. Said only where the
    // user's changes did it, as the contrast warning is.
    if let Some((other, d, floor)) = links::too_close(current, field)
        && links::too_close(base, field).is_none()
    {
        let warning = tr!(l10n, "colors-too-close", other = other.label(l10n));
        let hover = tr!(
            l10n,
            "colors-too-close.hover",
            distance = format!("{d:.0}"),
            floor = format!("{floor:.0}")
        );
        warning_label(ui, &warning, &hover);
    }
}

/// One of a swatch's warnings: the error glyph and `text`, small, in the
/// warning colour, with `hover` to explain it.
fn warning_label(ui: &mut Ui, text: &str, hover: &str) {
    ui.label(
        RichText::new(format!("{} {text}", crate::app::toast::ERROR_GLYPH))
            .small()
            .color(ui.visuals().warn_fg_color),
    )
    .on_hover_text(hover);
}

/// The body of the swatch `btn_id`'s picker popup, showing `color`; called
/// inside `Popup::show`. Traps Tab in the popup, focuses its anchor and
/// seeds the HSV from `color` when `newly_opened`, steps the sliders on
/// arrow keys, and shows the hex field. The new colour, if it changed this
/// frame.
fn picker_popup_body(
    ui: &mut Ui,
    l10n: &Localizer,
    btn_id: egui::Id,
    newly_opened: bool,
    color: egui::Color32,
) -> Option<egui::Color32> {
    let mut color_changed = false;
    let hsva_cache_key = btn_id.with("hsva_cache");
    // Trap Tab focus to the popup's layer. Without this, Tab cycles through
    // main-settings widgets (which are registered earlier in the frame) rather
    // than through the picker's drag values and sliders.
    ui.ctx().memory_mut(|m| m.set_modal_layer(ui.layer_id()));

    // Invisible focusable anchor. When the popup first opens, focus is still on
    // the swatch — in a layer *below* the modal layer and therefore no longer
    // focusable — so Tab wouldn't advance to anything. Requesting focus on this
    // anchor puts the user inside the popup's focus cycle immediately.
    // Zero-sized and placed with `interact` rather than added, so it takes no
    // row of its own: an empty label here left a blank line at the top of the
    // picker.
    let focus_anchor = ui.interact(
        egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::ZERO),
        btn_id.with("popup_focus_anchor"),
        egui::Sense::focusable_noninteractive(),
    );
    if newly_opened {
        focus_anchor.request_focus();
    }

    // HSVA is the source of truth while the popup is open. Converting srgba →
    // Hsva each frame is slightly lossy (sRGB gamma), so we cache the Hsva in
    // ctx temp data and only seed from the current color on first open.
    let mut hsva: egui::ecolor::Hsva = if newly_opened {
        egui::ecolor::Hsva::from(color)
    } else {
        ui.ctx()
            .data(|d| d.get_temp::<egui::ecolor::Hsva>(hsva_cache_key))
            .unwrap_or_else(|| egui::ecolor::Hsva::from(color))
    };

    // Arrow-key HSV adjustment. egui's `color_slider_1d` and `color_slider_2d`
    // only handle `interact_pointer_pos` (mouse drag), so Tab-focused sliders
    // do nothing on arrow press. Detect which slider currently has focus via
    // the rect shape of the focused widget (2D slider is square, hue slider is
    // wide-and-short) and apply the arrow deltas directly to `hsva` before the
    // picker renders. The size thresholds are chosen to include 100-wide
    // sliders (observed in this app's theme) while excluding small toggle/drag
    // widgets (~20 px).
    let hex_id = btn_id.with("hex_field");
    if let Some(fid) = ui.ctx().memory(|m| m.focused())
        && fid != btn_id
        // The hex field is as wide and short as the hue slider, and its arrows
        // move the caret.
        && fid != hex_id
        && let Some(focused_resp) = ui.ctx().read_response(fid)
    {
        let rect = focused_resp.rect;
        let is_2d_slider = rect.width() >= 50.0 && (rect.width() - rect.height()).abs() < 2.0;
        let is_hue_slider = rect.width() >= 50.0 && rect.width() > rect.height() * 3.0;

        let [left, right, up, down] = crate::a11y::consume_arrows(ui.ctx());

        let step = 0.02;
        let dx = if right { step } else { 0.0 } - if left { step } else { 0.0 };
        let dy = if up { step } else { 0.0 } - if down { step } else { 0.0 };

        if is_2d_slider && (dx != 0.0 || dy != 0.0) {
            hsva.s = (hsva.s + dx).clamp(0.0, 1.0);
            hsva.v = (hsva.v + dy).clamp(0.0, 1.0);
            color_changed = true;
        } else if is_hue_slider && dx != 0.0 {
            hsva.h = (hsva.h + dx).rem_euclid(1.0);
            color_changed = true;
        }
    }

    color_changed |=
        egui::color_picker::color_picker_hsva_2d(ui, &mut hsva, egui::color_picker::Alpha::Opaque);

    // The colour as text, to copy from one swatch and paste into another rather
    // than retyping three channels. It shows the colour while not being edited;
    // while it is, a valid entry applies at once.
    let text_key = hex_id.with("text");
    let editing = ui.ctx().memory(|m| m.has_focus(hex_id));
    let mut text = if editing {
        ui.ctx()
            .data(|d| d.get_temp::<String>(text_key))
            .unwrap_or_else(|| hex_text(egui::Color32::from(hsva)))
    } else {
        hex_text(egui::Color32::from(hsva))
    };
    ui.horizontal(|ui| {
        let caption = ui.label(tr!(l10n, "colors-hex"));
        let field = ui
            .add(
                egui::TextEdit::singleline(&mut text)
                    .id(hex_id)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(80.0),
            )
            .labelled_by(caption.id);
        let field = field.on_hover_text(tr!(l10n, "colors-hex.hover"));
        if field.changed()
            && let Some(pasted) = parse_hex(&text)
        {
            hsva = egui::ecolor::Hsva::from(pasted);
            color_changed = true;
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(text_key, text));

    // Persist Hsva for next frame.
    ui.ctx().data_mut(|d| d.insert_temp(hsva_cache_key, hsva));

    // The focus lock filter `keep_focus_with_popup` sets after `show` only
    // takes effect on the *next* frame because of its `had_focus_last_frame`
    // gate. Cover the first frame by resetting `focus_direction` if any arrow
    // is held — here, before the popup's own menu arrow navigation runs after
    // this body, which a reset after `show` came too late to stop.
    let any_arrow_down = ui.ctx().input(|i| {
        i.key_down(egui::Key::ArrowLeft)
            || i.key_down(egui::Key::ArrowRight)
            || i.key_down(egui::Key::ArrowUp)
            || i.key_down(egui::Key::ArrowDown)
    });
    let focused = ui.ctx().memory(|m| m.focused());
    if any_arrow_down && focused.is_some_and(|id| id != btn_id) {
        ui.ctx()
            .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }

    color_changed.then(|| egui::Color32::from(hsva))
}

/// Keep the keyboard focus with the swatch `btn_id`'s popup `popup_id`:
/// trapped on the widget it is on while the popup is open, and back on the
/// swatch once it closes. Called after `Popup::show`, which opens and closes
/// it.
fn keep_focus_with_popup(ui: &Ui, popup_id: egui::Id, btn_id: egui::Id) {
    // Track open state across frames to detect close transitions (click
    // outside, Esc, or swatch re-click) so focus can be restored to the swatch
    // regardless of how the popup closed.
    let was_open_key = btn_id.with("color_popup_was_open");
    let was_open: bool = ui.ctx().data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    let is_open_now = egui::Popup::is_id_open(ui.ctx(), popup_id);
    if is_open_now {
        // Trap arrow keys on whichever widget inside the popup currently has
        // focus. egui's color_slider_1d/2d (hue + saturation-value) are
        // focusable but don't respond to arrow keys; without trapping, the
        // first arrow press Tab-jumps focus off the slider spatially. Trapping
        // keeps focus inside the popup so the user can still Tab between
        // sliders and the RGBA drag values (which ARE keyboard-adjustable via
        // Enter-to-edit + Up/Down). See the "Known limitations" note in
        // docs/gui-reference.md on the color picker.
        if let Some(focused_id) = ui.ctx().memory(|m| m.focused())
            && focused_id != btn_id
        {
            ui.ctx().memory_mut(|m| {
                m.set_focus_lock_filter(
                    focused_id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
        }
    }
    if was_open && !is_open_now {
        // Popup just closed — put focus back on the swatch so keyboard users
        // don't get teleported to the top of the Tab order.
        ui.ctx().memory_mut(|m| m.request_focus(btn_id));
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(was_open_key, is_open_now));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::{headless_frame, key_tap};

    /// A lone Background swatch, one frame at a time, a second apart.
    struct Swatch {
        ctx: egui::Context,
        overrides: PaletteOverrides,
        colors: ThemeColors,
        seconds: f64,
    }

    impl Swatch {
        fn frame(&mut self, events: Vec<egui::Event>) {
            self.seconds += 1.0;
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
            let (overrides, colors) = (&mut self.overrides, &self.colors);
            headless_frame(&self.ctx, Some(screen), Some(self.seconds), events, |ui| {
                let l10n = &crate::l10n::english();
                color_edit(
                    ui,
                    l10n,
                    PaletteField::Background,
                    overrides,
                    colors,
                    colors,
                );
            });
        }

        fn focused_rect(&self) -> Option<egui::Rect> {
            let id = self.ctx.memory(|m| m.focused())?;
            Some(self.ctx.read_response(id)?.rect)
        }
    }

    /// An arrow held on the frame a picker slider takes focus adjusts the
    /// slider and leaves focus on it: egui's own arrow navigation, in the
    /// popup and in menus, must not move it to a neighbour.
    #[test]
    fn an_arrow_on_a_newly_focused_slider_keeps_focus_there() {
        let mut swatch = Swatch {
            ctx: crate::app::styled_context(),
            overrides: PaletteOverrides::default(),
            colors: crate::settings::Settings::default().theme_colors(true),
            seconds: 0.0,
        };
        swatch.frame(vec![]);
        swatch.frame(key_tap(egui::Key::Tab));
        swatch.frame(key_tap(egui::Key::Enter));
        swatch.frame(vec![]);
        // Tab to the saturation-value square.
        let mut square = None;
        for _ in 0..12 {
            swatch.frame(key_tap(egui::Key::Tab));
            if let Some(rect) = swatch.focused_rect()
                && rect.width() >= 50.0
                && (rect.width() - rect.height()).abs() < 2.0
            {
                square = Some(rect);
                break;
            }
        }
        let square = square.expect("Tab reaches the saturation-value square");
        // The arrow held over a frame: pressed, then released two frames on.
        let [press, release]: [egui::Event; 2] = key_tap(egui::Key::ArrowUp)
            .try_into()
            .expect("a press and a release");
        swatch.frame(vec![press]);
        swatch.frame(vec![]);
        swatch.frame(vec![release]);
        swatch.frame(vec![]);
        assert_eq!(swatch.focused_rect(), Some(square));
        assert_ne!(
            swatch.overrides,
            PaletteOverrides::default(),
            "the arrow moved the colour"
        );
    }

    #[test]
    fn the_hex_field_reads_what_it_writes_and_what_is_pasted() {
        let c = egui::Color32::from_rgb(0xB8, 0x00, 0x96);
        assert_eq!(hex_text(c), "#B80096");
        assert_eq!(parse_hex("#B80096"), Some(c));
        assert_eq!(parse_hex("  b80096 "), Some(c));
        for bad in ["#B8009", "#B800966", "#G80096", "", "#"] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
    }

    /// Every palette field has one swatch: as an anchor, as one of its
    /// followers, or in a row of its own — and a follower sits under its own
    /// anchor, not in a flat row.
    #[test]
    fn panel_rows_list_every_field_once() {
        let mut listed = Vec::new();
        for &(_, row) in PANEL_ROWS {
            match row {
                PanelRow::Family(anchor) => {
                    assert_eq!(links::anchor(anchor), None, "{anchor:?}");
                    listed.push(anchor);
                    listed.extend(links::followers(anchor));
                }
                PanelRow::Fields(fields) => {
                    for &f in fields {
                        assert_eq!(links::anchor(f), None, "{f:?} belongs under its anchor");
                        listed.push(f);
                    }
                }
            }
        }
        for &f in PaletteField::ALL {
            assert_eq!(listed.iter().filter(|&&g| g == f).count(), 1, "{f:?}");
        }
        assert_eq!(listed.len(), PaletteField::ALL.len());
    }
}
