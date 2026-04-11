use eframe::egui::Color32;

use crate::settings::{ColorPreset, HexColor, PaletteOverrides};

/// A dark/light color pair.
struct ColorPair(Color32, Color32);

impl ColorPair {
    const fn new(dark: Color32, light: Color32) -> Self {
        Self(dark, light)
    }

    fn pick(&self, dark: bool) -> Color32 {
        if dark { self.0 } else { self.1 }
    }
}

/// All base colors for a preset. Each holds a dark and light variant.
struct PresetColors {
    // -- UI chrome --
    background: ColorPair,
    text: ColorPair,
    button: ColorPair,
    // -- Status indicators --
    status_ok: ColorPair,
    status_warning: ColorPair,
    status_error: ColorPair,
    status_inactive: ColorPair,
    accent: ColorPair,
    // -- Graph --
    graph_line: ColorPair,
    graph_gap: ColorPair,
    graph_mean: ColorPair,
    graph_ref: ColorPair,
    graph_crossing: ColorPair,
    graph_cursor: ColorPair,
    graph_envelope: ColorPair,
    plot_background: ColorPair,
    graph_crosshair: ColorPair,
    // -- Minimap --
    minimap_viewport: ColorPair,
}

// ── Preset definitions ──────────────────────────────────────────────────────

/// Default preset — matches egui defaults for UI chrome, warm palette for data.
const PRESET_DEFAULT: PresetColors = PresetColors {
    // egui::Visuals::dark() / light() defaults
    background: ColorPair::new(Color32::from_gray(27), Color32::from_gray(248)),
    text: ColorPair::new(Color32::from_gray(140), Color32::from_gray(80)),
    button: ColorPair::new(Color32::from_gray(60), Color32::from_gray(230)),
    status_ok: ColorPair::new(
        Color32::from_rgb(60, 180, 75),
        Color32::from_rgb(0, 140, 30),
    ),
    status_warning: ColorPair::new(
        Color32::from_rgb(200, 120, 0),
        Color32::from_rgb(180, 80, 0),
    ),
    status_error: ColorPair::new(Color32::from_rgb(220, 60, 60), Color32::from_rgb(180, 0, 0)),
    status_inactive: ColorPair::new(
        Color32::from_rgb(150, 150, 150),
        Color32::from_rgb(120, 120, 120),
    ),
    accent: ColorPair::new(
        Color32::from_rgb(100, 180, 255),
        Color32::from_rgb(0, 100, 200),
    ),
    graph_line: ColorPair::new(
        Color32::from_rgb(220, 120, 120),
        Color32::from_rgb(180, 40, 40),
    ),
    graph_gap: ColorPair::new(
        Color32::from_rgb(220, 80, 80),
        Color32::from_rgba_premultiplied(200, 0, 0, 180),
    ),
    graph_mean: ColorPair::new(
        Color32::from_rgb(100, 200, 100),
        Color32::from_rgb(0, 120, 0),
    ),
    graph_ref: ColorPair::new(
        Color32::from_rgb(200, 200, 100),
        Color32::from_rgb(140, 100, 0),
    ),
    graph_crossing: ColorPair::new(
        Color32::from_rgb(255, 220, 100),
        Color32::from_rgb(150, 100, 0),
    ),
    graph_cursor: ColorPair::new(
        Color32::from_rgb(255, 180, 100),
        Color32::from_rgb(180, 70, 0),
    ),
    graph_envelope: ColorPair::new(
        Color32::from_rgba_premultiplied(100, 150, 200, 80),
        Color32::from_rgb(0, 60, 160),
    ),
    // egui extreme_bg_color defaults: dark=10, light=255
    plot_background: ColorPair::new(Color32::from_gray(10), Color32::from_gray(255)),
    graph_crosshair: ColorPair::new(Color32::from_gray(200), Color32::from_gray(60)),
    minimap_viewport: ColorPair::new(
        Color32::from_rgb(100, 150, 255),
        Color32::from_rgb(0, 70, 200),
    ),
};

/// High-contrast preset — bolder colors, darker/lighter backgrounds.
const PRESET_HIGH_CONTRAST: PresetColors = PresetColors {
    background: ColorPair::new(Color32::from_gray(0), Color32::from_gray(255)),
    text: ColorPair::new(Color32::from_gray(220), Color32::from_gray(20)),
    button: ColorPair::new(Color32::from_gray(50), Color32::from_gray(215)),
    status_ok: ColorPair::new(Color32::from_rgb(0, 230, 0), Color32::from_rgb(0, 130, 0)),
    status_warning: ColorPair::new(
        Color32::from_rgb(255, 160, 0),
        Color32::from_rgb(200, 100, 0),
    ),
    status_error: ColorPair::new(Color32::from_rgb(255, 40, 40), Color32::from_rgb(200, 0, 0)),
    status_inactive: ColorPair::new(
        Color32::from_rgb(180, 180, 180),
        Color32::from_rgb(100, 100, 100),
    ),
    accent: ColorPair::new(
        Color32::from_rgb(80, 180, 255),
        Color32::from_rgb(0, 80, 200),
    ),
    graph_line: ColorPair::new(Color32::from_rgb(0, 230, 230), Color32::from_rgb(0, 0, 180)),
    graph_gap: ColorPair::new(Color32::from_rgb(255, 60, 60), Color32::from_rgb(200, 0, 0)),
    graph_mean: ColorPair::new(Color32::from_rgb(0, 255, 0), Color32::from_rgb(0, 120, 0)),
    graph_ref: ColorPair::new(
        Color32::from_rgb(255, 255, 0),
        Color32::from_rgb(160, 120, 0),
    ),
    graph_crossing: ColorPair::new(
        Color32::from_rgb(255, 100, 255),
        Color32::from_rgb(140, 0, 140),
    ),
    graph_cursor: ColorPair::new(
        Color32::from_rgb(255, 160, 0),
        Color32::from_rgb(180, 80, 0),
    ),
    graph_envelope: ColorPair::new(
        Color32::from_rgba_premultiplied(80, 160, 255, 100),
        Color32::from_rgb(0, 60, 180),
    ),
    plot_background: ColorPair::new(Color32::from_gray(0), Color32::from_gray(255)),
    graph_crosshair: ColorPair::new(Color32::from_gray(240), Color32::from_gray(20)),
    minimap_viewport: ColorPair::new(
        Color32::from_rgb(100, 160, 255),
        Color32::from_rgb(0, 60, 200),
    ),
};

/// Colorblind-safe preset — avoids red-green confusion (deuteranopia/protanopia safe).
const PRESET_COLORBLIND_SAFE: PresetColors = PresetColors {
    background: ColorPair::new(Color32::from_gray(27), Color32::from_gray(248)),
    text: ColorPair::new(Color32::from_gray(140), Color32::from_gray(80)),
    button: ColorPair::new(Color32::from_gray(60), Color32::from_gray(230)),
    status_ok: ColorPair::new(
        Color32::from_rgb(0, 180, 160),
        Color32::from_rgb(0, 120, 100),
    ),
    status_warning: ColorPair::new(
        Color32::from_rgb(230, 159, 0),
        Color32::from_rgb(180, 100, 0),
    ),
    status_error: ColorPair::new(Color32::from_rgb(213, 94, 0), Color32::from_rgb(170, 50, 0)),
    status_inactive: ColorPair::new(
        Color32::from_rgb(150, 150, 150),
        Color32::from_rgb(120, 120, 120),
    ),
    accent: ColorPair::new(
        Color32::from_rgb(86, 180, 233),
        Color32::from_rgb(0, 100, 200),
    ),
    graph_line: ColorPair::new(
        Color32::from_rgb(86, 180, 233),
        Color32::from_rgb(0, 80, 160),
    ),
    graph_gap: ColorPair::new(
        Color32::from_rgb(213, 94, 0),
        Color32::from_rgba_premultiplied(170, 50, 0, 180),
    ),
    graph_mean: ColorPair::new(
        Color32::from_rgb(230, 159, 0),
        Color32::from_rgb(160, 100, 0),
    ),
    graph_ref: ColorPair::new(
        Color32::from_rgb(204, 121, 167),
        Color32::from_rgb(140, 60, 100),
    ),
    graph_crossing: ColorPair::new(
        Color32::from_rgb(240, 228, 66),
        Color32::from_rgb(140, 120, 0),
    ),
    graph_cursor: ColorPair::new(
        Color32::from_rgb(0, 158, 115),
        Color32::from_rgb(0, 110, 80),
    ),
    graph_envelope: ColorPair::new(
        Color32::from_rgba_premultiplied(86, 150, 200, 80),
        Color32::from_rgb(0, 60, 140),
    ),
    plot_background: ColorPair::new(Color32::from_gray(10), Color32::from_gray(255)),
    graph_crosshair: ColorPair::new(Color32::from_gray(200), Color32::from_gray(60)),
    minimap_viewport: ColorPair::new(
        Color32::from_rgb(86, 150, 233),
        Color32::from_rgb(0, 70, 180),
    ),
};

fn preset_colors(preset: ColorPreset) -> &'static PresetColors {
    match preset {
        ColorPreset::Default => &PRESET_DEFAULT,
        ColorPreset::HighContrast => &PRESET_HIGH_CONTRAST,
        ColorPreset::ColorblindSafe => &PRESET_COLORBLIND_SAFE,
    }
}

// ── ThemeColors ─────────────────────────────────────────────────────────────

/// Theme-aware color palette. Resolves colors from: override → preset → default.
/// All preset colors have dark and light variants chosen for WCAG AA contrast
/// on their respective backgrounds.
pub(crate) struct ThemeColors {
    dark: bool,
    preset: &'static PresetColors,
    overrides: PaletteOverrides,
}

impl ThemeColors {
    pub(crate) fn new(dark: bool, preset: ColorPreset, overrides: &PaletteOverrides) -> Self {
        Self {
            dark,
            preset: preset_colors(preset),
            overrides: overrides.clone(),
        }
    }

    /// Resolve a color: override wins, then preset default.
    fn resolve(&self, over: Option<HexColor>, pair: &ColorPair) -> Color32 {
        if let Some(h) = over {
            h.0
        } else {
            pair.pick(self.dark)
        }
    }

    // -- UI chrome --

    /// Panel/window background.
    pub(crate) fn background(&self) -> Color32 {
        self.resolve(self.overrides.background, &self.preset.background)
    }

    /// Primary text color for labels and values.
    pub(crate) fn text(&self) -> Color32 {
        self.resolve(self.overrides.text, &self.preset.text)
    }

    /// Button/widget fill color.
    pub(crate) fn button(&self) -> Color32 {
        self.resolve(self.overrides.button, &self.preset.button)
    }

    /// Derive hover/active button states from the base button color.
    pub(crate) fn button_hover_active(&self) -> (Color32, Color32) {
        let base = self.button();
        let [r, g, b, a] = base.to_array();
        let adj = |c: u8, d: i16| (c as i16 + d).clamp(0, 255) as u8;
        if self.dark {
            // hover: lighter, active: slightly darker
            (
                Color32::from_rgba_premultiplied(adj(r, 10), adj(g, 10), adj(b, 10), a),
                Color32::from_rgba_premultiplied(adj(r, -5), adj(g, -5), adj(b, -5), a),
            )
        } else {
            // hover: slightly darker, active: much darker
            (
                Color32::from_rgba_premultiplied(adj(r, -10), adj(g, -10), adj(b, -10), a),
                Color32::from_rgba_premultiplied(adj(r, -65), adj(g, -65), adj(b, -65), a),
            )
        }
    }

    // -- Status indicators --

    /// Connected, live, success.
    pub(crate) fn status_ok(&self) -> Color32 {
        self.resolve(self.overrides.status_ok, &self.preset.status_ok)
    }

    /// Warnings, reconnecting, paused.
    pub(crate) fn status_warning(&self) -> Color32 {
        self.resolve(self.overrides.status_warning, &self.preset.status_warning)
    }

    /// Errors, toast failures.
    pub(crate) fn status_error(&self) -> Color32 {
        self.resolve(self.overrides.status_error, &self.preset.status_error)
    }

    /// Disconnected/muted state.
    pub(crate) fn status_inactive(&self) -> Color32 {
        self.resolve(self.overrides.status_inactive, &self.preset.status_inactive)
    }

    /// Active flags, cursors, viewport indicators.
    pub(crate) fn accent(&self) -> Color32 {
        self.resolve(self.overrides.accent, &self.preset.accent)
    }

    // -- Graph colors --

    /// Live indicator — derives from status_ok().
    pub(crate) fn live_green(&self) -> Color32 {
        self.status_ok()
    }

    /// Main data line.
    pub(crate) fn graph_line(&self) -> Color32 {
        self.resolve(self.overrides.graph_line, &self.preset.graph_line)
    }

    /// Gap indicator lines.
    pub(crate) fn graph_gap(&self) -> Color32 {
        self.resolve(self.overrides.graph_gap, &self.preset.graph_gap)
    }

    /// Mean overlay line.
    pub(crate) fn graph_mean(&self) -> Color32 {
        self.resolve(self.overrides.graph_mean, &self.preset.graph_mean)
    }

    /// Reference line overlay.
    pub(crate) fn graph_ref(&self) -> Color32 {
        self.resolve(self.overrides.graph_ref, &self.preset.graph_ref)
    }

    /// Trigger crossing markers.
    pub(crate) fn graph_crossing(&self) -> Color32 {
        self.resolve(self.overrides.graph_crossing, &self.preset.graph_crossing)
    }

    /// Cursor lines and labels.
    pub(crate) fn graph_cursor(&self) -> Color32 {
        self.resolve(self.overrides.graph_cursor, &self.preset.graph_cursor)
    }

    /// Cursor dimmed variant — derives from graph_cursor() with reduced alpha.
    pub(crate) fn graph_cursor_dim(&self) -> Color32 {
        let base = self.graph_cursor();
        let [r, g, b, _] = base.to_array();
        if self.dark {
            Color32::from_rgba_premultiplied(r, g, b, 80)
        } else {
            base
        }
    }

    /// Cursor delta readout (ΔT/ΔV text) — derives from graph_cursor().
    pub(crate) fn graph_cursor_delta(&self) -> Color32 {
        self.graph_cursor()
    }

    /// Plot area background fill.
    pub(crate) fn plot_background(&self) -> Color32 {
        self.resolve(self.overrides.plot_background, &self.preset.plot_background)
    }

    /// Plot crosshair / pointer tracking lines.
    pub(crate) fn graph_crosshair(&self) -> Color32 {
        self.resolve(self.overrides.graph_crosshair, &self.preset.graph_crosshair)
    }

    /// Min/max envelope lines.
    pub(crate) fn graph_envelope(&self) -> Color32 {
        self.resolve(self.overrides.graph_envelope, &self.preset.graph_envelope)
    }

    /// Minimap data line — derives from graph_line() with semi-transparency.
    pub(crate) fn minimap_line(&self) -> Color32 {
        let base = self.graph_line();
        let [r, g, b, _] = base.to_array();
        let alpha = if self.dark { 200 } else { 220 };
        Color32::from_rgba_premultiplied(r, g, b, alpha)
    }

    /// Minimap viewport bracket indicator.
    pub(crate) fn minimap_viewport(&self) -> Color32 {
        self.resolve(
            self.overrides.minimap_viewport,
            &self.preset.minimap_viewport,
        )
    }

    /// Recording buffer full warning — derives from status_warning().
    pub(crate) fn recording_full_warning(&self) -> Color32 {
        self.status_warning()
    }

    /// Return the effective color for a given field, for use in the settings UI.
    pub(crate) fn effective_color(&self, field: &str) -> Color32 {
        match field {
            "background" => self.background(),
            "text" => self.text(),
            "button" => self.button(),
            "graph_line" => self.graph_line(),
            "graph_gap" => self.graph_gap(),
            "graph_mean" => self.graph_mean(),
            "graph_ref" => self.graph_ref(),
            "graph_crossing" => self.graph_crossing(),
            "graph_cursor" => self.graph_cursor(),
            "graph_envelope" => self.graph_envelope(),
            "plot_background" => self.plot_background(),
            "graph_crosshair" => self.graph_crosshair(),
            "status_ok" => self.status_ok(),
            "status_warning" => self.status_warning(),
            "status_error" => self.status_error(),
            "status_inactive" => self.status_inactive(),
            "accent" => self.accent(),
            "minimap_viewport" => self.minimap_viewport(),
            _ => Color32::TRANSPARENT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_matches_original_colors() {
        let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc.status_ok(), Color32::from_rgb(60, 180, 75));
        assert_eq!(tc.graph_line(), Color32::from_rgb(220, 120, 120));
        assert_eq!(tc.graph_cursor(), Color32::from_rgb(255, 180, 100));
        assert_eq!(tc.background(), Color32::from_gray(27));

        let tc_light = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc_light.status_ok(), Color32::from_rgb(0, 140, 30));
        assert_eq!(tc_light.graph_line(), Color32::from_rgb(180, 40, 40));
        assert_eq!(tc_light.background(), Color32::from_gray(248));
    }

    #[test]
    fn override_takes_precedence() {
        let mut overrides = PaletteOverrides::default();
        overrides.graph_line = Some(HexColor(Color32::from_rgb(0, 0, 255)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.graph_line(), Color32::from_rgb(0, 0, 255));
        assert_eq!(tc.graph_mean(), Color32::from_rgb(100, 200, 100));
    }

    #[test]
    fn derived_colors_track_base() {
        let mut overrides = PaletteOverrides::default();
        overrides.graph_cursor = Some(HexColor(Color32::from_rgb(100, 200, 50)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.graph_cursor_delta(), Color32::from_rgb(100, 200, 50));
        assert_eq!(
            tc.graph_cursor_dim(),
            Color32::from_rgba_premultiplied(100, 200, 50, 80)
        );
    }

    #[test]
    fn minimap_line_derives_from_graph_line() {
        let mut overrides = PaletteOverrides::default();
        overrides.graph_line = Some(HexColor(Color32::from_rgb(50, 100, 200)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(
            tc.minimap_line(),
            Color32::from_rgba_premultiplied(50, 100, 200, 200)
        );
    }

    #[test]
    fn live_indicator_derives_from_status_ok() {
        let mut overrides = PaletteOverrides::default();
        overrides.status_ok = Some(HexColor(Color32::from_rgb(0, 255, 128)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.live_green(), Color32::from_rgb(0, 255, 128));
    }

    #[test]
    fn recording_warning_derives_from_status_warning() {
        let mut overrides = PaletteOverrides::default();
        overrides.status_warning = Some(HexColor(Color32::from_rgb(255, 200, 0)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.recording_full_warning(), Color32::from_rgb(255, 200, 0));
    }

    #[test]
    fn different_presets_produce_different_colors() {
        let default = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        let high_contrast = ThemeColors::new(
            true,
            ColorPreset::HighContrast,
            &PaletteOverrides::default(),
        );
        let colorblind = ThemeColors::new(
            true,
            ColorPreset::ColorblindSafe,
            &PaletteOverrides::default(),
        );

        assert_ne!(default.graph_line(), high_contrast.graph_line());
        assert_ne!(default.graph_line(), colorblind.graph_line());
        assert_ne!(high_contrast.graph_line(), colorblind.graph_line());
    }

    #[test]
    fn ui_chrome_colors_resolve() {
        let mut overrides = PaletteOverrides::default();
        overrides.background = Some(HexColor(Color32::from_rgb(10, 20, 30)));
        overrides.text = Some(HexColor(Color32::from_rgb(200, 210, 220)));
        overrides.button = Some(HexColor(Color32::from_rgb(80, 80, 80)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.background(), Color32::from_rgb(10, 20, 30));
        assert_eq!(tc.text(), Color32::from_rgb(200, 210, 220));
        assert_eq!(tc.button(), Color32::from_rgb(80, 80, 80));
    }

    #[test]
    fn plot_colors_resolve() {
        let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc.plot_background(), Color32::from_gray(10));
        assert_eq!(tc.graph_crosshair(), Color32::from_gray(200));

        let mut overrides = PaletteOverrides::default();
        overrides.plot_background = Some(HexColor(Color32::from_rgb(20, 20, 40)));
        overrides.graph_crosshair = Some(HexColor(Color32::from_rgb(255, 255, 0)));

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.plot_background(), Color32::from_rgb(20, 20, 40));
        assert_eq!(tc.graph_crosshair(), Color32::from_rgb(255, 255, 0));
    }
}
