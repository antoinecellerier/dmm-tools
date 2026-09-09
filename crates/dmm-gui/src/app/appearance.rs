//! Look and feel of the window itself: the bundled font chain and text
//! styles, the theme and colour overrides applied to egui's `Visuals`, the
//! zoom levels, and the always-on-top / decoration viewport commands.

use eframe::egui;

use super::App;
use crate::settings::ThemeMode;

/// Memo key for [`App::apply_color_overrides`]: every colour it pins into
/// egui's `Visuals`, plus whether the palette pins a Border and a selection
/// colour and whether Background and Button are overridden.
///
/// Those four are not implied by the colours: they decide whether the open
/// combo box, the hover/press border strokes, the selection fill and the
/// scrollbar trough follow the palette or keep egui's own values, and a user
/// may override a field to the value it already had. Border and selection are
/// carried as `Option`s rather than a colour and a flag each; `None` means
/// egui's own value, which depends only on the mode, and a mode change clears
/// the memo outright. Text needs no such flag — the caption colours already
/// carry the fallback, since they resolve to egui's own values when Text is
/// not overridden.
///
/// A struct rather than the tuple this used to be: the tuple had reached the
/// twelve-element ceiling `PartialEq` is implemented up to, and named fields
/// make a mismatch between the key and what the function actually assigns
/// findable.
#[derive(PartialEq, Clone, Copy)]
pub(super) struct UiColorKey {
    background: egui::Color32,
    text: egui::Color32,
    weak_text: egui::Color32,
    button: egui::Color32,
    /// `None` while the border is egui's own grey.
    border: Option<egui::Color32>,
    /// `None` while the selection is egui's own blue.
    selection: Option<egui::Color32>,
    plot_background: egui::Color32,
    warning: egui::Color32,
    error: egui::Color32,
    button_text: egui::Color32,
    strong_text: egui::Color32,
    background_overridden: bool,
    button_overridden: bool,
}

/// Size of `TextStyle::Small`, in points before zoom.
///
/// egui ships 9 pt, under the 11 pt floor in `.claude/rules/gui.md`. `.small()`
/// is used across the app for real content — status line, hint captions,
/// toolbar captions, the LIVE button — so the style is raised once here rather
/// than at ~20 call sites. `apply_zoom` scales on top of this.
pub(super) const SMALL_TEXT_SIZE: f32 = 11.0;

/// Raise egui's small text style to the 11 pt floor.
///
/// `all_styles_mut`, not `style_mut`: egui 0.36 keeps a separate `Style` per
/// theme, and `apply_theme` switches between them with `set_visuals`, which
/// replaces only `style.visuals` and leaves `text_styles` alone. Setting just
/// the active theme's style would leave the other theme at 9 pt.
pub(super) fn install_text_styles(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(SMALL_TEXT_SIZE, egui::FontFamily::Proportional),
        );
    });
}

/// Font definitions with the monospace face added to the proportional
/// fallback chain.
///
/// egui's proportional chain is Ubuntu-Light, then the two emoji fonts, and
/// Ubuntu-Light has no U+2192: the Scale row's arrow separator and the arrow
/// in a transform's description both rendered as tofu boxes. Hack is already
/// bundled (it is the monospace face the reading itself uses) and covers the
/// arrows and the rest of the maths block.
///
/// It goes at the *end* of the chain, behind egui's emoji faces. Ahead of
/// them it also re-resolved every other glyph Ubuntu-Light lacks — the Manual
/// link's U+2197, Resume's U+25B6, Stop's U+25A0, the graph zoom's
/// U+229E/U+229F — away from the emoji fonts that had been drawing them,
/// changing their weight. The glyphs that actually need Hack (U+2192, U+25CF)
/// are in no other bundled face, so last in the chain still reaches them.
pub(super) fn font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        proportional.push("Hack".to_owned());
    }
    fonts
}

impl App {
    /// Whether the UI should render dark, resolving `System` against the OS.
    ///
    /// `system_theme()` returns `None` when the platform reports no
    /// preference; Dark is the app's default, so that's the fallback.
    fn resolve_dark(&self, ctx: &egui::Context) -> bool {
        match self.settings.theme {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => !matches!(ctx.system_theme(), Some(egui::Theme::Light)),
        }
    }

    pub(super) fn apply_theme(&mut self, ctx: &egui::Context) {
        // `applied.theme` holds the *resolved* mode, never `System`. That way
        // an OS theme flip while set to System changes the target here and
        // repaints, instead of comparing System to System and doing nothing.
        let dark = self.resolve_dark(ctx);
        let target = if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        if self.applied.theme != Some(target) {
            // Only on change: set_visuals every frame resets egui's internal
            // panel state (resize positions, scroll offsets).
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            self.applied.theme = Some(target);
            self.applied.ui_colors = None; // force reapply on top of new base
        }
    }

    /// Apply background, text, and button color overrides to egui Visuals.
    pub(super) fn apply_color_overrides(&mut self, ctx: &egui::Context) {
        let dark = self.resolve_dark(ctx);
        let tc = self.settings.theme_colors(dark);
        let overrides = self.settings.color_overrides.for_mode(dark);
        let bg_overridden = overrides.background.is_some();
        let button_overridden = overrides.button.is_some();
        let accent_overridden = overrides.accent.is_some();
        let bg = tc.background();
        let text = tc.text();
        let weak_text = tc.weak_text();
        let button = tc.button();
        let border = tc.border();
        let border_pinned = tc.border_pinned();
        let plot_bg = tc.plot_background();
        let warning = tc.status_warning();
        let error = tc.status_error();
        let button_text = tc.button_text();
        let strong_text = tc.strong_text();
        let accent = tc.accent();
        let key = UiColorKey {
            background: bg,
            text,
            weak_text,
            button,
            border: border_pinned.then_some(border),
            selection: accent_overridden.then_some(accent),
            plot_background: plot_bg,
            warning,
            error,
            button_text,
            strong_text,
            background_overridden: bg_overridden,
            button_overridden,
        };

        if self.applied.ui_colors == Some(key) {
            return;
        }
        self.applied.ui_colors = Some(key);

        let (hover, active) = tc.button_hover_active();
        // Two fills egui paints from its own palette follow the palette only
        // when the field driving them is overridden: the scrollbar trough
        // (`noninteractive`) follows Background, the open combo box (`open`)
        // follows Button. Otherwise they keep the value egui ships — read
        // from `Visuals` rather than copied as a literal, so it keeps
        // tracking egui across upgrades, and assigned either way so that
        // clearing an override restores it without a theme switch.
        let stock = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let (trough, trough_weak) = if bg_overridden {
            (bg, bg)
        } else {
            let w = &stock.widgets.noninteractive;
            (w.bg_fill, w.weak_bg_fill)
        };
        let (open, open_weak) = if button_overridden {
            (button, button)
        } else {
            let w = &stock.widgets.open;
            (w.bg_fill, w.weak_bg_fill)
        };
        // Three more strokes on the same rule. `border()` is already egui's
        // separator grey while nothing pins one, and that is exactly what
        // `noninteractive.bg_stroke` and `window_stroke` ship as, so those two
        // take it unconditionally. The open combo box's outline and the
        // emphasis egui puts on a hovered or pressed widget are *not* that
        // grey — egui darkens them in light mode — so they wait for a border
        // the palette actually pins, and take the emphasised text colour when
        // pressed.
        let (open_stroke, hover_stroke, active_stroke) = if border_pinned {
            (border, border, strong_text)
        } else {
            let w = &stock.widgets;
            (
                w.open.bg_stroke.color,
                w.hovered.bg_stroke.color,
                w.active.bg_stroke.color,
            )
        };
        // The same rule again for `selection`, driven by Accent. egui fills a
        // selected `Button`/`selectable_label` — the meter's HOLD/REL/AUTO
        // toggles, the settings and graph chips — with `selection.bg_fill` and
        // redraws its caption in `selection.stroke.color`; the text-selection
        // highlight reads the same pair, and the app's own focus ring takes
        // whichever of the two is visible on the panel
        // (`a11y::focus_ring_color`). Selected text goes on the background
        // colour rather than the text colour: it sits *on* the accent, so it
        // needs the contrast the palette already guarantees between those two
        // (`text_colors_meet_wcag_aa_in_both_themes`). The stroke width is
        // egui's.
        //
        // egui strokes the frame of a focused `TextEdit` with the same
        // colour, and that one sits on the panel rather than on the accent,
        // so it disappears here: the graph toolbar and Scale fields paint
        // `a11y::paint_focus_ring` over it for a cue that survives an Accent.
        let (selection_fill, selection_text) = if accent_overridden {
            (accent, bg)
        } else {
            (stock.selection.bg_fill, stock.selection.stroke.color)
        };
        ctx.global_style_mut(|style| {
            let v = &mut style.visuals;
            v.panel_fill = bg;
            v.window_fill = bg;
            // Plot background and minimap background use extreme_bg_color.
            v.extreme_bg_color = plot_bg;
            v.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(v.widgets.noninteractive.fg_stroke.width, text);
            // Pin the secondary text colour instead of letting egui derive it.
            // Unset, `weak_text_color()` is the text colour at
            // `weak_text_alpha` = 0.6, which lands around 3.8:1 dark / 2.9:1
            // light on the Default preset's panel — under the 4.5:1 AA bar,
            // and it is used for real information (mode line, sub-value labels
            // and timestamps, toolbar captions, hint captions).
            v.weak_text_color = Some(weak_text);
            // egui draws warnings and errors from these two fields; the app
            // has the same two colours in the palette, and the stock light
            // orange is 2.8:1 on the light panel.
            v.warn_fg_color = warning;
            v.error_fg_color = error;
            // The text colour reached labels only. Button captions and the
            // caption of a hovered or pressed widget — which is also what
            // `.strong()` text is drawn with — come from these four strokes,
            // so a customised text colour never got to them. Both accessors
            // return egui's own value while Text is unset, so the assignment
            // is unconditional. Only `.color` is set: the widths are egui's
            // emphasis on hover (1.5) and press (2.0).
            v.widgets.inactive.fg_stroke.color = button_text;
            v.widgets.open.fg_stroke.color = button_text;
            v.widgets.hovered.fg_stroke.color = strong_text;
            v.widgets.active.fg_stroke.color = strong_text;
            v.widgets.inactive.bg_fill = button;
            v.widgets.inactive.weak_bg_fill = button;
            v.widgets.hovered.bg_fill = hover;
            v.widgets.hovered.weak_bg_fill = hover;
            v.widgets.active.bg_fill = active;
            v.widgets.active.weak_bg_fill = active;
            v.widgets.noninteractive.bg_fill = trough;
            v.widgets.noninteractive.weak_bg_fill = trough_weak;
            v.widgets.open.bg_fill = open;
            v.widgets.open.weak_bg_fill = open_weak;
            // Separators, panel edges, the toolbar group boxes and the plot
            // outline all come off `noninteractive.bg_stroke`; `window_stroke`
            // frames the colour picker, the help modal and the discard modal.
            // Only `.color` again: the widths are egui's, including the 0 on
            // `inactive` that leaves a resting button unoutlined.
            v.widgets.noninteractive.bg_stroke.color = border;
            v.window_stroke.color = border;
            v.widgets.open.bg_stroke.color = open_stroke;
            v.widgets.hovered.bg_stroke.color = hover_stroke;
            v.widgets.active.bg_stroke.color = active_stroke;
            v.selection.bg_fill = selection_fill;
            v.selection.stroke.color = selection_text;
        });
    }

    pub(super) const ZOOM_LEVELS: &[u32] = &[
        30, 50, 67, 80, 90, 100, 110, 120, 133, 150, 170, 200, 240, 300,
    ];

    pub(super) fn apply_zoom(&mut self, ctx: &egui::Context) {
        // Capture OS default pixels_per_point on first call
        if self.applied.os_ppp.is_none() {
            self.applied.os_ppp = Some(ctx.pixels_per_point());
        }
        let Some(os_ppp) = self.applied.os_ppp else {
            return;
        };
        let target_ppp = os_ppp * self.settings.zoom_pct as f32 / 100.0;
        // Only update when changed — setting ppp every frame resets panel resize state
        if (ctx.pixels_per_point() - target_ppp).abs() > 0.001 {
            ctx.set_pixels_per_point(target_ppp);
        }
    }

    pub(super) fn zoom_in(&mut self) {
        if let Some(&next) = Self::ZOOM_LEVELS
            .iter()
            .find(|&&z| z > self.settings.zoom_pct)
        {
            self.settings.zoom_pct = next;
            self.settings.save();
        }
    }

    pub(super) fn zoom_out(&mut self) {
        if let Some(&prev) = Self::ZOOM_LEVELS
            .iter()
            .rev()
            .find(|&&z| z < self.settings.zoom_pct)
        {
            self.settings.zoom_pct = prev;
            self.settings.save();
        }
    }

    pub(super) fn zoom_reset(&mut self) {
        self.settings.zoom_pct = 100;
        self.settings.save();
    }

    pub(super) fn apply_always_on_top(&self, ctx: &egui::Context) {
        let level = if self.settings.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    pub(super) fn apply_decorations(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
            !self.settings.hide_decorations,
        ));
    }

    /// Returns true if the app is running on a native Wayland session.
    pub(super) fn is_wayland() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{HexColor, PaletteOverrides, Settings};
    use eframe::egui::Color32;

    /// The state the first frame leaves in `Visuals`: an app whose current
    /// mode carries `overrides`, after one theme and one override pass on a
    /// fresh context.
    fn themed_app(dark: bool, overrides: PaletteOverrides) -> (egui::Context, App) {
        let mut settings = Settings {
            theme: if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            ..Settings::default()
        };
        *settings.color_overrides.for_mode_mut(dark) = overrides;
        let mut app = App::from_settings(settings);
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        app.apply_color_overrides(&ctx);
        (ctx, app)
    }

    /// The colours egui draws warning and error text with are the palette's,
    /// so the stats panel's "⚠ N gaps skipped" is the Warning colour: egui's
    /// own orange measures 2.8:1 on the light panel, under the AA bar.
    #[test]
    fn warning_and_error_text_use_the_status_colours() {
        for dark in [true, false] {
            let (ctx, app) = themed_app(dark, PaletteOverrides::default());
            let tc = app.settings.theme_colors(dark);
            let visuals = &ctx.global_style().visuals;
            assert_eq!(visuals.warn_fg_color, tc.status_warning(), "dark={dark}");
            assert_eq!(visuals.error_fg_color, tc.status_error(), "dark={dark}");
        }
    }

    /// The scrollbar trough stays on egui's fill until Background is
    /// customised. Compared against `Visuals`, never a literal, so the
    /// fallback keeps tracking whatever egui ships.
    #[test]
    fn the_scrollbar_trough_tracks_egui_until_the_background_is_overridden() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let (ctx, _) = themed_app(dark, PaletteOverrides::default());
            assert_eq!(
                ctx.global_style().visuals.widgets.noninteractive.bg_fill,
                stock.widgets.noninteractive.bg_fill,
                "dark={dark}"
            );

            let picked = Color32::from_rgb(0x21, 0x30, 0x40);
            let (ctx, _) = themed_app(
                dark,
                PaletteOverrides {
                    background: Some(HexColor(picked)),
                    ..Default::default()
                },
            );
            assert_eq!(
                ctx.global_style().visuals.widgets.noninteractive.bg_fill,
                picked,
                "dark={dark}"
            );
        }
    }

    /// Same rule for the fill an open combo box draws itself with: egui's
    /// until Button is customised, then the palette's.
    #[test]
    fn the_open_combo_box_tracks_egui_until_the_button_is_overridden() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let (ctx, _) = themed_app(dark, PaletteOverrides::default());
            assert_eq!(
                ctx.global_style().visuals.widgets.open.weak_bg_fill,
                stock.widgets.open.weak_bg_fill,
                "dark={dark}"
            );

            let picked = Color32::from_rgb(0x40, 0x30, 0x21);
            let (ctx, _) = themed_app(
                dark,
                PaletteOverrides {
                    button: Some(HexColor(picked)),
                    ..Default::default()
                },
            );
            assert_eq!(
                ctx.global_style().visuals.widgets.open.weak_bg_fill,
                picked,
                "dark={dark}"
            );
        }
    }

    /// Button captions and `.strong()` text keep egui's own colours until Text
    /// is customised, then follow it. The stroke *widths* are egui's emphasis
    /// on hover and press and must survive the recolour.
    #[test]
    fn button_captions_track_egui_until_the_text_is_overridden() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let (ctx, _) = themed_app(dark, PaletteOverrides::default());
            let style = ctx.global_style();
            let w = &style.visuals.widgets;
            assert_eq!(
                w.inactive.fg_stroke.color, stock.widgets.inactive.fg_stroke.color,
                "dark={dark}"
            );
            assert_eq!(
                w.active.fg_stroke.color, stock.widgets.active.fg_stroke.color,
                "dark={dark}"
            );

            let picked = Color32::from_rgb(0x8A, 0xC0, 0xE0);
            let (ctx, app) = themed_app(
                dark,
                PaletteOverrides {
                    text: Some(HexColor(picked)),
                    ..Default::default()
                },
            );
            let tc = app.settings.theme_colors(dark);
            let style = ctx.global_style();
            let w = &style.visuals.widgets;
            assert_eq!(w.inactive.fg_stroke.color, picked, "dark={dark}");
            assert_eq!(w.open.fg_stroke.color, picked, "dark={dark}");
            assert_eq!(w.hovered.fg_stroke.color, tc.strong_text(), "dark={dark}");
            assert_eq!(w.active.fg_stroke.color, tc.strong_text(), "dark={dark}");
            for (name, got, want) in [
                (
                    "inactive",
                    w.inactive.fg_stroke.width,
                    stock.widgets.inactive.fg_stroke.width,
                ),
                (
                    "hovered",
                    w.hovered.fg_stroke.width,
                    stock.widgets.hovered.fg_stroke.width,
                ),
                (
                    "active",
                    w.active.fg_stroke.width,
                    stock.widgets.active.fg_stroke.width,
                ),
            ] {
                assert_eq!(got, want, "{name} stroke width changed (dark={dark})");
            }
        }
    }

    /// Separators, panel edges and window frames stay on egui's grey under the
    /// Default preset and follow the Border colour once it is customised —
    /// hover emphasis included. The stroke *widths* are egui's and must
    /// survive the recolour, the 0 on `inactive` above all: widening it would
    /// put an outline round every resting button.
    #[test]
    fn borders_track_egui_until_the_border_is_overridden() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let (ctx, _) = themed_app(dark, PaletteOverrides::default());
            let style = ctx.global_style();
            let v = &style.visuals;
            assert_eq!(
                v.widgets.noninteractive.bg_stroke.color,
                stock.widgets.noninteractive.bg_stroke.color,
                "dark={dark}"
            );
            assert_eq!(
                v.window_stroke.color, stock.window_stroke.color,
                "dark={dark}"
            );
            assert_eq!(
                v.widgets.open.bg_stroke.color, stock.widgets.open.bg_stroke.color,
                "dark={dark}"
            );
            assert_eq!(
                v.widgets.hovered.bg_stroke.color, stock.widgets.hovered.bg_stroke.color,
                "dark={dark}"
            );
            drop(style);

            let picked = Color32::from_rgb(0x30, 0x90, 0xC0);
            let (ctx, app) = themed_app(
                dark,
                PaletteOverrides {
                    border: Some(HexColor(picked)),
                    ..Default::default()
                },
            );
            let tc = app.settings.theme_colors(dark);
            let style = ctx.global_style();
            let v = &style.visuals;
            assert_eq!(
                v.widgets.noninteractive.bg_stroke.color, picked,
                "dark={dark}"
            );
            assert_eq!(v.window_stroke.color, picked, "dark={dark}");
            assert_eq!(v.widgets.open.bg_stroke.color, picked, "dark={dark}");
            assert_eq!(v.widgets.hovered.bg_stroke.color, picked, "dark={dark}");
            assert_eq!(
                v.widgets.active.bg_stroke.color,
                tc.strong_text(),
                "dark={dark}"
            );
            for (name, got, want) in [
                (
                    "noninteractive",
                    v.widgets.noninteractive.bg_stroke.width,
                    stock.widgets.noninteractive.bg_stroke.width,
                ),
                (
                    "inactive",
                    v.widgets.inactive.bg_stroke.width,
                    stock.widgets.inactive.bg_stroke.width,
                ),
                (
                    "hovered",
                    v.widgets.hovered.bg_stroke.width,
                    stock.widgets.hovered.bg_stroke.width,
                ),
                ("window", v.window_stroke.width, stock.window_stroke.width),
            ] {
                assert_eq!(got, want, "{name} stroke width changed (dark={dark})");
            }
        }
    }

    /// The fill behind a selected toggle or chip, and the colour egui redraws
    /// its caption in, stay on egui's blue until Accent is customised. The
    /// stroke *width* is egui's and must survive the recolour.
    #[test]
    fn the_selection_tracks_egui_until_the_accent_is_overridden() {
        for dark in [true, false] {
            let stock = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let (ctx, _) = themed_app(dark, PaletteOverrides::default());
            let style = ctx.global_style();
            let selection = &style.visuals.selection;
            assert_eq!(selection.bg_fill, stock.selection.bg_fill, "dark={dark}");
            assert_eq!(
                selection.stroke.color, stock.selection.stroke.color,
                "dark={dark}"
            );
            drop(style);

            let picked = Color32::from_rgb(0x22, 0xC5, 0x5E);
            let (ctx, app) = themed_app(
                dark,
                PaletteOverrides {
                    accent: Some(HexColor(picked)),
                    ..Default::default()
                },
            );
            let tc = app.settings.theme_colors(dark);
            let style = ctx.global_style();
            let selection = &style.visuals.selection;
            assert_eq!(selection.bg_fill, picked, "dark={dark}");
            assert_eq!(selection.stroke.color, tc.background(), "dark={dark}");
            assert_eq!(
                selection.stroke.width, stock.selection.stroke.width,
                "selection stroke width changed (dark={dark})"
            );
        }
    }

    /// Ubuntu-Light has no rightwards arrow, so without the monospace face in
    /// the chain the Scale row and its toast render tofu boxes. It has to sit
    /// *behind* the emoji fonts: ahead of them it also captured the symbols
    /// they were already drawing (↗, ▶, ■, ⊞, ⊟) and changed their weight.
    #[test]
    fn the_proportional_family_falls_back_to_the_monospace_face_last() {
        let fonts = font_definitions();
        let proportional = fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .expect("egui always defines the proportional family");
        let hack = proportional
            .iter()
            .position(|name| name == "Hack")
            .expect("monospace face is in the chain");
        let emoji = proportional
            .iter()
            .rposition(|name| name.contains("moji"))
            .expect("egui's defaults include the emoji fonts");
        assert!(hack > emoji, "got {proportional:?}");
        assert_eq!(hack, proportional.len() - 1, "got {proportional:?}");
        assert_eq!(
            proportional.first().map(String::as_str),
            Some("Ubuntu-Light")
        );
    }

    /// Both themes, because egui keeps a `Style` per theme and `apply_theme`
    /// swaps between them: raising only the active one would leave `.small()`
    /// at egui's 9 pt after the first theme switch.
    #[test]
    fn small_text_style_meets_the_font_size_floor_in_both_themes() {
        let ctx = egui::Context::default();
        install_text_styles(&ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let style = ctx.style_of(theme);
            assert_eq!(
                style.text_styles[&egui::TextStyle::Small].size,
                SMALL_TEXT_SIZE,
                "{theme:?} small text style is below the 11 pt floor"
            );
        }
    }
}
