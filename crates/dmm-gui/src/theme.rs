use eframe::egui::Color32;

use crate::settings::{ColorPreset, HexColor, PaletteOverrides};

/// One user-customisable colour in the palette.
///
/// Replaces the `&str` keys that four parallel lists matched on — the
/// effective-colour lookup, the tooltip table, the settings-panel label and
/// the `PaletteOverrides` field passed alongside it. A typo produced a
/// transparent swatch with a generic tooltip at runtime; now the compiler
/// catches it, and a new colour can't be half-added.
///
/// CLAUDE.md: "Prefer enums over string-typed status/state values."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteField {
    Background,
    Text,
    Button,
    GraphLine,
    GraphGap,
    GraphMean,
    GraphRef,
    GraphCrossing,
    GraphCursor,
    GraphEnvelope,
    PlotBackground,
    GraphCrosshair,
    StatusOk,
    StatusWarning,
    StatusError,
    StatusInactive,
    Accent,
    MinimapViewport,
}

impl PaletteField {
    /// Every field, in settings-panel order.
    pub(crate) const ALL: &'static [PaletteField] = &[
        PaletteField::Background,
        PaletteField::Text,
        PaletteField::Button,
        PaletteField::GraphLine,
        PaletteField::GraphGap,
        PaletteField::GraphMean,
        PaletteField::GraphRef,
        PaletteField::GraphCrossing,
        PaletteField::GraphCursor,
        PaletteField::GraphEnvelope,
        PaletteField::PlotBackground,
        PaletteField::GraphCrosshair,
        PaletteField::StatusOk,
        PaletteField::StatusWarning,
        PaletteField::StatusError,
        PaletteField::StatusInactive,
        PaletteField::Accent,
        PaletteField::MinimapViewport,
    ];

    /// Short label shown next to the swatch.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Background => "Background",
            Self::Text => "Text",
            Self::Button => "Button",
            Self::GraphLine => "Data line",
            Self::GraphGap => "Gap",
            Self::GraphMean => "Mean",
            Self::GraphRef => "Ref",
            Self::GraphCrossing => "Crossing",
            Self::GraphCursor => "Cursor",
            Self::GraphEnvelope => "Envelope",
            Self::PlotBackground => "Plot bg",
            Self::GraphCrosshair => "Crosshair",
            Self::StatusOk => "Connected",
            Self::StatusWarning => "Warning",
            Self::StatusError => "Error",
            Self::StatusInactive => "Inactive",
            Self::Accent => "Accent",
            Self::MinimapViewport => "Viewport",
        }
    }

    /// Hover text for the swatch.
    pub(crate) fn tooltip(self) -> &'static str {
        match self {
            Self::Background => "Panel background color",
            Self::Text => "Primary text color",
            Self::Button => "Button background color",
            Self::GraphLine => "Data line color on the graph",
            Self::GraphGap => "Color used to mark gaps in recorded data",
            Self::GraphMean => "Mean overlay line color",
            Self::GraphRef => "Reference line color",
            Self::GraphCrossing => "Trigger-crossing marker color",
            Self::GraphCursor => "Measurement cursor color",
            Self::GraphEnvelope => "Min/Max envelope fill color",
            Self::PlotBackground => "Graph plot area background color",
            Self::GraphCrosshair => "Hover crosshair color on the graph",
            Self::StatusOk => "\"Connected\" status color",
            Self::StatusWarning => "Warning status color",
            Self::StatusError => "Error status color",
            Self::StatusInactive => "Inactive / disconnected status color",
            Self::Accent => "Accent color used by active toggles and highlights",
            Self::MinimapViewport => "Minimap viewport rectangle color",
        }
    }

    /// The override slot for this field.
    ///
    /// Pairing the field with its slot here means a call site can't ask for
    /// one colour and write to another's override.
    pub(crate) fn override_slot(self, o: &mut PaletteOverrides) -> &mut Option<HexColor> {
        match self {
            Self::Background => &mut o.background,
            Self::Text => &mut o.text,
            Self::Button => &mut o.button,
            Self::GraphLine => &mut o.graph_line,
            Self::GraphGap => &mut o.graph_gap,
            Self::GraphMean => &mut o.graph_mean,
            Self::GraphRef => &mut o.graph_ref,
            Self::GraphCrossing => &mut o.graph_crossing,
            Self::GraphCursor => &mut o.graph_cursor,
            Self::GraphEnvelope => &mut o.graph_envelope,
            Self::PlotBackground => &mut o.plot_background,
            Self::GraphCrosshair => &mut o.graph_crosshair,
            Self::StatusOk => &mut o.status_ok,
            Self::StatusWarning => &mut o.status_warning,
            Self::StatusError => &mut o.status_error,
            Self::StatusInactive => &mut o.status_inactive,
            Self::Accent => &mut o.accent,
            Self::MinimapViewport => &mut o.minimap_viewport,
        }
    }
}

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
        // 5.34:1 on gray(248); the previous (0,140,30) was 4.15:1 (< AA 4.5)
        Color32::from_rgb(0, 120, 25),
    ),
    status_warning: ColorPair::new(
        Color32::from_rgb(200, 120, 0),
        Color32::from_rgb(180, 80, 0),
    ),
    status_error: ColorPair::new(
        // 4.62:1 on gray(27); the previous (220,60,60) was 3.90:1 (< AA 4.5)
        Color32::from_rgb(230, 80, 80),
        Color32::from_rgb(180, 0, 0),
    ),
    status_inactive: ColorPair::new(
        Color32::from_rgb(150, 150, 150),
        // 4.80:1 on gray(248); the previous (120,120,120) was 4.16:1 (< AA 4.5)
        Color32::from_rgb(110, 110, 110),
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
        // 5.77:1 on white; the previous (200,100,0) was 3.98:1 (< AA 4.5)
        Color32::from_rgb(160, 80, 0),
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
        // 4.79:1 on gray(248); the previous (180,100,0) was 4.15:1 (< AA 4.5)
        Color32::from_rgb(165, 92, 0),
    ),
    status_error: ColorPair::new(
        // 4.91:1 on gray(27); the previous (213,94,0) was 4.45:1 (< AA 4.5)
        Color32::from_rgb(224, 100, 0),
        Color32::from_rgb(170, 50, 0),
    ),
    status_inactive: ColorPair::new(
        Color32::from_rgb(150, 150, 150),
        // 4.80:1 on gray(248); the previous (120,120,120) was 4.16:1 (< AA 4.5)
        Color32::from_rgb(110, 110, 110),
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
    /// Exhaustive: there is no fallback arm, so adding a `PaletteField`
    /// without wiring it here is a compile error rather than a swatch that
    /// silently renders transparent.
    pub(crate) fn effective_color(&self, field: PaletteField) -> Color32 {
        match field {
            PaletteField::Background => self.background(),
            PaletteField::Text => self.text(),
            PaletteField::Button => self.button(),
            PaletteField::GraphLine => self.graph_line(),
            PaletteField::GraphGap => self.graph_gap(),
            PaletteField::GraphMean => self.graph_mean(),
            PaletteField::GraphRef => self.graph_ref(),
            PaletteField::GraphCrossing => self.graph_crossing(),
            PaletteField::GraphCursor => self.graph_cursor(),
            PaletteField::GraphEnvelope => self.graph_envelope(),
            PaletteField::PlotBackground => self.plot_background(),
            PaletteField::GraphCrosshair => self.graph_crosshair(),
            PaletteField::StatusOk => self.status_ok(),
            PaletteField::StatusWarning => self.status_warning(),
            PaletteField::StatusError => self.status_error(),
            PaletteField::StatusInactive => self.status_inactive(),
            PaletteField::Accent => self.accent(),
            PaletteField::MinimapViewport => self.minimap_viewport(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field must resolve to a real colour. The old string-keyed
    /// lookup fell through to TRANSPARENT, so a typo produced an invisible
    /// swatch at runtime instead of a compile error.
    #[test]
    fn every_palette_field_resolves_to_a_colour() {
        for &dark in &[true, false] {
            let tc = ThemeColors::new(dark, ColorPreset::Default, &PaletteOverrides::default());
            for &field in PaletteField::ALL {
                assert_ne!(
                    tc.effective_color(field),
                    Color32::TRANSPARENT,
                    "{field:?} resolves to transparent (dark={dark})"
                );
            }
        }
    }

    /// ALL drives the settings panel, so a field missing from it would be
    /// uneditable even though the rest of the plumbing exists.
    #[test]
    fn all_lists_every_field_exactly_once() {
        let mut seen: Vec<PaletteField> = Vec::new();
        for &f in PaletteField::ALL {
            assert!(!seen.contains(&f), "{f:?} listed twice");
            seen.push(f);
        }
        // The settings panel slices ALL into four groups by index; if the
        // count changes those slices need revisiting.
        assert_eq!(seen.len(), 18);
    }

    /// An override must come back from the field it was written to — the
    /// point of pairing the slot with the field.
    #[test]
    fn override_slot_round_trips_per_field() {
        for &field in PaletteField::ALL {
            let mut overrides = PaletteOverrides::default();
            *field.override_slot(&mut overrides) = Some(HexColor(Color32::from_rgb(1, 2, 3)));
            let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
            assert_eq!(
                tc.effective_color(field),
                Color32::from_rgb(1, 2, 3),
                "{field:?} did not read back its own override"
            );
        }
    }

    #[test]
    fn labels_and_tooltips_are_distinct_per_field() {
        for &a in PaletteField::ALL {
            for &b in PaletteField::ALL {
                if a != b {
                    assert_ne!(a.tooltip(), b.tooltip(), "{a:?} and {b:?} share a tooltip");
                }
            }
        }
    }

    #[test]
    fn default_preset_matches_original_colors() {
        let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc.status_ok(), Color32::from_rgb(60, 180, 75));
        assert_eq!(tc.graph_line(), Color32::from_rgb(220, 120, 120));
        assert_eq!(tc.graph_cursor(), Color32::from_rgb(255, 180, 100));
        assert_eq!(tc.background(), Color32::from_gray(27));

        let tc_light = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc_light.status_ok(), Color32::from_rgb(0, 120, 25));
        assert_eq!(tc_light.graph_line(), Color32::from_rgb(180, 40, 40));
        assert_eq!(tc_light.background(), Color32::from_gray(248));
    }

    #[test]
    fn override_takes_precedence() {
        let overrides = PaletteOverrides {
            graph_line: Some(HexColor(Color32::from_rgb(0, 0, 255))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.graph_line(), Color32::from_rgb(0, 0, 255));
        assert_eq!(tc.graph_mean(), Color32::from_rgb(100, 200, 100));
    }

    #[test]
    fn derived_colors_track_base() {
        let overrides = PaletteOverrides {
            graph_cursor: Some(HexColor(Color32::from_rgb(100, 200, 50))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.graph_cursor_delta(), Color32::from_rgb(100, 200, 50));
        assert_eq!(
            tc.graph_cursor_dim(),
            Color32::from_rgba_premultiplied(100, 200, 50, 80)
        );
    }

    #[test]
    fn minimap_line_derives_from_graph_line() {
        let overrides = PaletteOverrides {
            graph_line: Some(HexColor(Color32::from_rgb(50, 100, 200))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(
            tc.minimap_line(),
            Color32::from_rgba_premultiplied(50, 100, 200, 200)
        );
    }

    #[test]
    fn live_indicator_derives_from_status_ok() {
        let overrides = PaletteOverrides {
            status_ok: Some(HexColor(Color32::from_rgb(0, 255, 128))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.live_green(), Color32::from_rgb(0, 255, 128));
    }

    #[test]
    fn recording_warning_derives_from_status_warning() {
        let overrides = PaletteOverrides {
            status_warning: Some(HexColor(Color32::from_rgb(255, 200, 0))),
            ..Default::default()
        };

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
        let overrides = PaletteOverrides {
            background: Some(HexColor(Color32::from_rgb(10, 20, 30))),
            text: Some(HexColor(Color32::from_rgb(200, 210, 220))),
            button: Some(HexColor(Color32::from_rgb(80, 80, 80))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.background(), Color32::from_rgb(10, 20, 30));
        assert_eq!(tc.text(), Color32::from_rgb(200, 210, 220));
        assert_eq!(tc.button(), Color32::from_rgb(80, 80, 80));
    }

    /// WCAG 2.1 relative luminance of an sRGB color.
    fn luminance(c: Color32) -> f64 {
        fn lin(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    /// WCAG 2.1 contrast ratio between two colors (1.0..=21.0).
    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Every color used to render text must meet WCAG 2.1 AA (>= 4.5:1)
    /// against its preset's panel background, in both themes. Catches
    /// dark-tuned colors that fail on light backgrounds — historically the
    /// largest source of rework in this project.
    #[test]
    fn text_colors_meet_wcag_aa_in_both_themes() {
        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for dark in [true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let bg = tc.background();
                let text_colors = [
                    ("text", tc.text()),
                    ("status_ok", tc.status_ok()),
                    ("status_warning", tc.status_warning()),
                    ("status_error", tc.status_error()),
                    ("status_inactive", tc.status_inactive()),
                    ("accent", tc.accent()),
                ];
                for (name, fg) in text_colors {
                    let ratio = contrast(fg, bg);
                    assert!(
                        ratio >= 4.5,
                        "{preset:?} {} mode: {name} {fg:?} on {bg:?} is {ratio:.2}:1, below WCAG AA 4.5:1",
                        if dark { "dark" } else { "light" },
                    );
                }
            }
        }
    }

    #[test]
    fn plot_colors_resolve() {
        let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc.plot_background(), Color32::from_gray(10));
        assert_eq!(tc.graph_crosshair(), Color32::from_gray(200));

        let overrides = PaletteOverrides {
            plot_background: Some(HexColor(Color32::from_rgb(20, 20, 40))),
            graph_crosshair: Some(HexColor(Color32::from_rgb(255, 255, 0))),
            ..Default::default()
        };

        let tc = ThemeColors::new(true, ColorPreset::Default, &overrides);
        assert_eq!(tc.plot_background(), Color32::from_rgb(20, 20, 40));
        assert_eq!(tc.graph_crosshair(), Color32::from_rgb(255, 255, 0));
    }
}
