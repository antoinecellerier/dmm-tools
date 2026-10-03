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
                let base = match mode {
                    ThemeMode::Dark => "Dark",
                    ThemeMode::Light => "Light",
                    ThemeMode::System => "System",
                };
                Chip {
                    value: ThemeChoice::Mode(mode),
                    selected,
                    // Dark and Light keep their own customizations while a
                    // named theme is on; say so on the chip, as a theme's
                    // chip does, picked or not, so a customized Light isn't
                    // taken for plain Light.
                    label: if selected && from_flag {
                        format!("{base} (--theme)")
                    } else if customized(mode) {
                        format!("{base} (customized)")
                    } else {
                        base.to_string()
                    },
                    tooltip: match mode {
                        ThemeMode::System => {
                            "Follow the desktop's light/dark setting (Dark if it reports none)"
                                .to_string()
                        }
                        _ => format!("Use {base} mode for the whole GUI"),
                    },
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
                let suffix = match (selected && from_flag, customized) {
                    (true, _) => " (--theme)",
                    (false, true) => " (customized)",
                    (false, false) => "",
                };
                Chip {
                    value: ThemeChoice::Named(theme.name.clone()),
                    selected,
                    label: format!("{}{suffix}", theme.name),
                    tooltip: {
                        let mode = if theme.dark { "Dark" } else { "Light" };
                        // A user's theme names its file in full, so it can be
                        // found to edit, share or delete.
                        match (&theme.file, &themes_dir) {
                            (Some(file), Some(dir)) => format!(
                                "{mode} theme, from {}. Delete the file to remove it.",
                                dir.join(file).display()
                            ),
                            (Some(file), None) => format!("{mode} theme, from {file}"),
                            (None, _) => format!("Built-in {} theme", mode.to_lowercase()),
                        }
                    },
                }
            });
            if let Some(choice) = chip_row(ui, "Theme:", modes.into_iter().chain(named)) {
                match choice {
                    ThemeChoice::Mode(mode) => {
                        self.settings.theme = mode;
                        self.settings.named_theme = None;
                    }
                    ThemeChoice::Named(name) => {
                        // Its own mode is the fallback should its file go.
                        if let Some(theme) = named::find(&name, &self.settings.user_themes.themes) {
                            self.settings.theme = if theme.dark {
                                ThemeMode::Dark
                            } else {
                                ThemeMode::Light
                            };
                        }
                        self.settings.named_theme = Some(name);
                    }
                }
                // Clear the override — user explicitly chose a theme
                self.settings.overrides.theme = None;
                self.settings.save();
            }
            let skipped = &self.settings.user_themes.skipped;
            if !skipped.is_empty() {
                let mut list: Vec<String> = themes_dir
                    .iter()
                    .map(|dir| format!("In {}:", dir.display()))
                    .collect();
                list.extend(
                    skipped
                        .iter()
                        .map(|(file, reason)| format!("{file}: {reason}")),
                );
                let caption = match skipped.len() {
                    1 => "1 theme file skipped".to_string(),
                    n => format!("{n} theme files skipped"),
                };
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
        ui.horizontal_wrapped(|ui| {
            let chips = [
                ColorPreset::Default,
                ColorPreset::HighContrast,
                ColorPreset::ColorblindSafe,
            ]
            .into_iter()
            .map(|preset| {
                let selected = self.settings.color_preset == preset;
                let base = match preset {
                    ColorPreset::Default => "Default",
                    ColorPreset::HighContrast => "High Contrast",
                    ColorPreset::ColorblindSafe => "Colorblind",
                };
                Chip {
                    value: preset,
                    selected,
                    label: if selected && has_overrides {
                        format!("{base} (customized)")
                    } else {
                        base.to_string()
                    },
                    tooltip: match preset {
                        ColorPreset::Default => "Balanced palette tuned for everyday use",
                        ColorPreset::HighContrast => {
                            "Maximum-contrast palette for bright lighting or projectors"
                        }
                        ColorPreset::ColorblindSafe => {
                            "Palette that stays distinguishable for protan/deutan vision"
                        }
                    }
                    .to_string(),
                }
            });
            if let Some(preset) = chip_row(ui, "Colors:", chips) {
                self.settings.color_preset = preset;
                // Clear the presets' overrides when switching presets.
                self.settings.color_overrides.dark = PaletteOverrides::default();
                self.settings.color_overrides.light = PaletteOverrides::default();
                self.applied.ui_colors = None; // force reapply
                self.settings.save();
            }
            if has_overrides {
                ui.label(
                    RichText::new("Selecting a preset will clear customizations")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });
    }

    /// Show the collapsible color customization section.
    pub(super) fn show_color_customization(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;

        let collapsing = egui::CollapsingHeader::new("Customize colors")
            .default_open(false)
            .show(ui, |ui| {
                // A named theme's own palette only in its own mode, as
                // `Settings::color_tweaks` has it: on the frame a chip is
                // picked, the UI may still be in the other.
                let named = self
                    .settings
                    .active_theme()
                    .filter(|t| t.dark == dark)
                    .map(|t| t.name.clone());
                let editing = match &named {
                    Some(name) => format!("(editing {name} colors)"),
                    None if dark => "(editing dark theme colors)".to_string(),
                    None => "(editing light theme colors)".to_string(),
                };
                ui.label(
                    RichText::new(editing)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );

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
                for &(caption, row) in PANEL_ROWS {
                    match row {
                        PanelRow::Family(anchor) => {
                            // One row per family, the settings panel being
                            // short on height: the anchor, an arrow, the
                            // colours that follow it — dashed while they do.
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, caption);
                                changed |= color_edit(ui, anchor, overrides, &current, &base);
                                ui.label(
                                    RichText::new("\u{2192}").color(ui.visuals().weak_text_color()),
                                )
                                .on_hover_text(format!(
                                    "These follow {}, keeping their offset from it, until \
                                     you pick one on its own",
                                    anchor.label()
                                ));
                                for field in links::followers(anchor) {
                                    changed |= color_edit(ui, field, overrides, &current, &base);
                                }
                            });
                        }
                        PanelRow::Fields(fields) => {
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, caption);
                                for &field in fields {
                                    changed |= color_edit(ui, field, overrides, &current, &base);
                                }
                            });
                        }
                    }
                }
                if changed && let Some(name) = &named {
                    self.settings.color_overrides.set_for_theme(name, tweaks);
                }

                // Reset button
                ui.horizontal(|ui| {
                    if ui
                        .button("Reset colors")
                        .on_hover_text(match &named {
                            Some(name) => format!("Discard your changes to {name}'s colors"),
                            None => "Discard your changes to the colors of this mode".to_string(),
                        })
                        .clicked()
                    {
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
                    self.show_theme_save_row(ui, dark);
                });

                if changed {
                    self.applied.ui_colors = None; // force reapply
                    // Deferred: a drag or a held arrow key changes the
                    // colour every frame, and each save is an fsync.
                    self.settings_save.schedule(std::time::Instant::now());
                }
            });
        // Paint an explicit focus ring on the header when Tab-focused —
        // egui's CollapsingHeader shows only a subtle highlight otherwise,
        // which is easy to miss.
        crate::a11y::paint_focus_ring(ui, &collapsing.header_response);
        collapsing
            .header_response
            .on_hover_text("Per-color overrides on top of the selected preset or theme");
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

/// The swatch rows, top to bottom, under their captions. Every palette
/// field is on exactly one: as an anchor, as one of its followers, or in a
/// row of its own (`panel_rows_list_every_field_once`).
const PANEL_ROWS: &[(&str, PanelRow)] = &[
    ("UI:", PanelRow::Family(PaletteField::Background)),
    ("", PanelRow::Family(PaletteField::Text)),
    ("", PanelRow::Fields(&[PaletteField::Accent])),
    ("Graph:", PanelRow::Family(PaletteField::GraphLine)),
    (
        "",
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
        "Status:",
        PanelRow::Fields(&[
            PaletteField::StatusOk,
            PaletteField::StatusWarning,
            PaletteField::StatusError,
            PaletteField::StatusInactive,
        ]),
    ),
];

/// A row's caption, at one width so the swatches line up beneath it.
fn caption_cell(ui: &mut Ui, caption: &str) {
    let width = 56.0;
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
    egui::Id::new("color_relink_refocus")
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
    field: PaletteField,
    overrides: &mut PaletteOverrides,
    current: &ThemeColors,
    base: &ThemeColors,
) -> bool {
    let label = field.label();
    let anchor = links::anchor(field);
    let set = field.override_slot(overrides).is_some();
    let linked = anchor.is_some() && !set;
    // The colour on screen: this field's override, or where the preset, the
    // theme or its anchor put it.
    let mut color = current.effective_color(field);

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
    let text = egui::WidgetText::from(RichText::new(label).small());
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
        .on_hover_text(field.tooltip())
        .a11y_label(&match anchor {
            Some(anchor) if linked => format!("{label}, follows {}", anchor.label()),
            _ => label.to_string(),
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
    // An outline on every swatch, so one the colour of the panel it sits
    // on still shows; dashed while it follows its anchor.
    let rect = btn_response.rect;
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
    // The fill covers the usual button border, so paint an explicit focus
    // ring when the swatch is keyboard-focused.
    crate::a11y::paint_focus_ring(ui, &btn_response);

    let mut relinked = false;
    if let Some(anchor) = anchor
        && set
        && ui
            .small_button("\u{21BA} relink")
            .on_hover_text(format!("Follow {} again", anchor.label()))
            .a11y_label(&format!("Relink {label} to {}", anchor.label()))
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
    // Only where the user's changes brought it under: the presets' own
    // decorative borders sit under the graphical floor on purpose. A linked
    // colour warns too, when no lightness could keep it readable.
    if let Some((ratio, ground, floor)) = links::floor_failure(current, field)
        && links::floor_failure(base, field).is_none()
    {
        ui.label(
            RichText::new(format!(
                "{} {ratio:.1}:1 on {}",
                crate::app::toast::ERROR_GLYPH,
                ground.label()
            ))
            .small()
            .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text(format!(
            "Under the {floor}:1 contrast this color needs on {}",
            ground.label()
        ));
    }

    // Graph colours drawn together have to read apart; a turn of the line
    // can bring an overlay onto a colour that stayed put. Said only where
    // the user's changes did it, as the contrast warning is.
    if let Some((other, d, floor)) = links::too_close(current, field)
        && links::too_close(base, field).is_none()
    {
        ui.label(
            RichText::new(format!(
                "{} close to {}",
                crate::app::toast::ERROR_GLYPH,
                other.label()
            ))
            .small()
            .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text(format!(
            "Only {d:.0} apart (\u{0394}E) where graph colors need {floor:.0} to read apart"
        ));
    }

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

    // Track open state across frames to detect close transitions (click
    // outside, Esc, or swatch re-click) so focus can be restored to the
    // swatch regardless of how the popup closed.
    let was_open_key = btn_response.id.with("color_popup_was_open");
    let was_open: bool = ui.ctx().data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    let mut color_changed = false;
    let hsva_cache_key = btn_response.id.with("hsva_cache");
    egui::Popup::menu(&btn_response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            // Trap Tab focus to the popup's layer. Without this, Tab cycles
            // through main-settings widgets (which are registered earlier in
            // the frame) rather than through the picker's drag values and
            // sliders.
            ui.ctx().memory_mut(|m| m.set_modal_layer(ui.layer_id()));

            // Invisible focusable anchor. When the popup first opens, focus
            // is still on the swatch — in a layer *below* the modal layer and
            // therefore no longer focusable — so Tab wouldn't advance to
            // anything. Requesting focus on this anchor puts the user inside
            // the popup's focus cycle immediately.
            // Zero-sized and placed with `interact` rather than added, so it
            // takes no row of its own: an empty label here left a blank line
            // at the top of the picker.
            let focus_anchor = ui.interact(
                egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::ZERO),
                btn_response.id.with("popup_focus_anchor"),
                egui::Sense::focusable_noninteractive(),
            );
            if newly_opened {
                focus_anchor.request_focus();
            }

            // HSVA is the source of truth while the popup is open.
            // Converting srgba → Hsva each frame is slightly lossy (sRGB
            // gamma), so we cache the Hsva in ctx temp data and only seed
            // from the current color on first open.
            let mut hsva: egui::ecolor::Hsva = if newly_opened {
                egui::ecolor::Hsva::from(color)
            } else {
                ui.ctx()
                    .data(|d| d.get_temp::<egui::ecolor::Hsva>(hsva_cache_key))
                    .unwrap_or_else(|| egui::ecolor::Hsva::from(color))
            };

            // Arrow-key HSV adjustment. egui's `color_slider_1d` and
            // `color_slider_2d` only handle `interact_pointer_pos` (mouse
            // drag), so Tab-focused sliders do nothing on arrow press.
            // Detect which slider currently has focus via the rect shape
            // of the focused widget (2D slider is square, hue slider is
            // wide-and-short) and apply the arrow deltas directly to
            // `hsva` before the picker renders. The size thresholds are
            // chosen to include 100-wide sliders (observed in this app's
            // theme) while excluding small toggle/drag widgets (~20 px).
            let hex_id = btn_response.id.with("hex_field");
            if let Some(fid) = ui.ctx().memory(|m| m.focused())
                && fid != btn_response.id
                // The hex field is as wide and short as the hue slider,
                // and its arrows move the caret.
                && fid != hex_id
                && let Some(focused_resp) = ui.ctx().read_response(fid)
            {
                let rect = focused_resp.rect;
                let is_2d_slider =
                    rect.width() >= 50.0 && (rect.width() - rect.height()).abs() < 2.0;
                let is_hue_slider = rect.width() >= 50.0 && rect.width() > rect.height() * 3.0;

                let left = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
                let right = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
                let up = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                let down = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));

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

            color_changed |= egui::color_picker::color_picker_hsva_2d(
                ui,
                &mut hsva,
                egui::color_picker::Alpha::Opaque,
            );

            // The colour as text, to copy from one swatch and paste into
            // another rather than retyping three channels. It shows the
            // colour while not being edited; while it is, a valid entry
            // applies at once.
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
                let caption = ui.label("Hex");
                let field = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .id(hex_id)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(80.0),
                    )
                    .labelled_by(caption.id);
                let field = field.on_hover_text("Copy this color, or paste one as #RRGGBB");
                if field.changed()
                    && let Some(pasted) = parse_hex(&text)
                {
                    hsva = egui::ecolor::Hsva::from(pasted);
                    color_changed = true;
                }
            });
            ui.ctx().data_mut(|d| d.insert_temp(text_key, text));

            // Write back to Color32 and persist Hsva for next frame.
            color = egui::Color32::from(hsva);
            ui.ctx().data_mut(|d| d.insert_temp(hsva_cache_key, hsva));
        });

    let is_open_now = egui::Popup::is_id_open(ui.ctx(), popup_id);
    if is_open_now {
        // Trap arrow keys on whichever widget inside the popup currently
        // has focus. egui's color_slider_1d/2d (hue + saturation-value) are
        // focusable but don't respond to arrow keys; without trapping, the
        // first arrow press Tab-jumps focus off the slider spatially.
        // Trapping keeps focus inside the popup so the user can still Tab
        // between sliders and the RGBA drag values (which ARE keyboard-
        // adjustable via Enter-to-edit + Up/Down). See the "Known
        // limitations" note in docs/gui-reference.md on the color picker.
        if let Some(focused_id) = ui.ctx().memory(|m| m.focused())
            && focused_id != btn_response.id
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
            // `set_focus_lock_filter` only takes effect on the *next*
            // frame because of its `had_focus_last_frame` gate. Cover the
            // first-frame case by also resetting `focus_direction` if any
            // arrow is held this frame.
            let any_arrow_down = ui.ctx().input(|i| {
                i.key_down(egui::Key::ArrowLeft)
                    || i.key_down(egui::Key::ArrowRight)
                    || i.key_down(egui::Key::ArrowUp)
                    || i.key_down(egui::Key::ArrowDown)
            });
            if any_arrow_down {
                ui.ctx()
                    .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }
    }
    if was_open && !is_open_now {
        // Popup just closed — put focus back on the swatch so keyboard users
        // don't get teleported to the top of the Tab order.
        ui.ctx().memory_mut(|m| m.request_focus(btn_response.id));
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(was_open_key, is_open_now));

    if color_changed {
        *field.override_slot(overrides) = Some(HexColor(color));
        return true;
    }

    relinked
}

#[cfg(test)]
mod tests {
    use super::*;

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
