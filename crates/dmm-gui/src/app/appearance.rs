//! Look and feel of the window itself: the bundled font chain and text
//! styles, the theme and colour overrides applied to egui's `Visuals`, the
//! zoom levels, and the always-on-top / decoration viewport commands.

use eframe::egui;

use super::App;
use crate::settings::ThemeMode;

/// Size of `TextStyle::Small`, in points before zoom.
///
/// egui ships 9 pt, under the 11 pt floor in `.claude/rules/gui.md`. `.small()`
/// is used across the app for real content — status line, hint captions,
/// toolbar captions, the LIVE button — so the style is raised once here rather
/// than at ~20 call sites. `apply_zoom` scales on top of this.
pub(super) const SMALL_TEXT_SIZE: f32 = 11.0;

/// Raise egui's small text style to the 11 pt floor.
///
/// `all_styles_mut`, not `style_mut`: egui 0.34 keeps a separate `Style` per
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
        let bg = tc.background();
        let text = tc.text();
        let weak_text = tc.weak_text();
        let button = tc.button();
        let plot_bg = tc.plot_background();
        let key = (bg, text, weak_text, button, plot_bg);

        if self.applied.ui_colors == Some(key) {
            return;
        }
        self.applied.ui_colors = Some(key);

        let (hover, active) = tc.button_hover_active();
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
            v.widgets.inactive.bg_fill = button;
            v.widgets.inactive.weak_bg_fill = button;
            v.widgets.hovered.bg_fill = hover;
            v.widgets.hovered.weak_bg_fill = hover;
            v.widgets.active.bg_fill = active;
            v.widgets.active.weak_bg_fill = active;
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
