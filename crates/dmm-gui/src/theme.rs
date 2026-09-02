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
    GraphOverlay1,
    GraphOverlay2,
    GraphOverlay3,
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
        PaletteField::GraphOverlay1,
        PaletteField::GraphOverlay2,
        PaletteField::GraphOverlay3,
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
            Self::GraphOverlay1 => "Overlay 1",
            Self::GraphOverlay2 => "Overlay 2",
            Self::GraphOverlay3 => "Overlay 3",
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
            Self::GraphOverlay1 => {
                "Line color of the first sub-value drawn beside the plotted series"
            }
            Self::GraphOverlay2 => {
                "Line color of the second sub-value drawn beside the plotted series"
            }
            Self::GraphOverlay3 => {
                "Line color of the third sub-value drawn beside the plotted series"
            }
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
            Self::GraphOverlay1 => &mut o.graph_overlay_1,
            Self::GraphOverlay2 => &mut o.graph_overlay_2,
            Self::GraphOverlay3 => &mut o.graph_overlay_3,
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
    /// Secondary text. Not a `PaletteField` — see `ThemeColors::weak_text`.
    weak_text: ColorPair,
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
    graph_overlay_1: ColorPair,
    graph_overlay_2: ColorPair,
    graph_overlay_3: ColorPair,
    plot_background: ColorPair,
    graph_crosshair: ColorPair,
    // -- Minimap --
    minimap_viewport: ColorPair,
}

// ── Preset definitions ──────────────────────────────────────────────────────

/// Default preset — egui's UI chrome, warm palette for data. Dark-mode text
/// is the one deliberate departure; see `text` below.
const PRESET_DEFAULT: PresetColors = PresetColors {
    // egui::Visuals::dark() / light() background defaults
    background: ColorPair::new(Color32::from_gray(27), Color32::from_gray(248)),
    // Light is egui's gray(80). Dark is *not* egui's gray(140): that sits
    // 5.12:1 on gray(27), leaving no room for a secondary tier that still
    // clears AA — 135 would be a tier in name only. gray(180) is 8.31:1, and
    // it is the grey egui already uses for button text
    // (`widgets.inactive.fg_stroke`), so labels now match the buttons beside
    // them.
    text: ColorPair::new(Color32::from_gray(180), Color32::from_gray(80)),
    // Paired with the text above as the dimmer of two visible tiers: 135 vs
    // 180 on gray(27) reads as secondary and still clears AA at 4.79:1
    // (4.54:1 on the faint frame fill gray(32), 5.51:1 on the gray(10)
    // text-edit background). Light: 112 vs 80, 4.66:1 on gray(248), 4.87:1 on
    // gray(253), 4.95:1 on gray(255).
    weak_text: ColorPair::new(Color32::from_gray(135), Color32::from_gray(112)),
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
    // Sub-value overlays. Deliberately not the mean/ref/cursor colours: those
    // are also drawn dashed, so a sub-value sharing one would be
    // indistinguishable from an overlay line. Contrast against the plot
    // background is asserted in
    // `overlay_colors_meet_graphical_contrast_on_the_plot_area`.
    graph_overlay_1: ColorPair::new(
        Color32::from_rgb(0, 200, 200),
        Color32::from_rgb(0, 115, 125),
    ),
    graph_overlay_2: ColorPair::new(
        Color32::from_rgb(190, 140, 255),
        Color32::from_rgb(105, 40, 200),
    ),
    graph_overlay_3: ColorPair::new(
        Color32::from_rgb(230, 230, 230),
        Color32::from_rgb(75, 75, 75),
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
    // 5.46:1 on gray(0), 5.30:1 on the faint frame fill gray(5) / 5.10:1 on
    // gray(255), which is also this preset's text-edit background. The wide
    // gap to the gray(220)/gray(20) body text is the point: unlike Default,
    // this preset has room for a visibly dimmer secondary tone.
    weak_text: ColorPair::new(Color32::from_gray(130), Color32::from_gray(110)),
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
    graph_overlay_1: ColorPair::new(Color32::from_gray(255), Color32::from_gray(0)),
    graph_overlay_2: ColorPair::new(
        Color32::from_rgb(200, 140, 255),
        Color32::from_rgb(100, 0, 200),
    ),
    graph_overlay_3: ColorPair::new(
        Color32::from_rgb(255, 150, 120),
        Color32::from_rgb(160, 50, 20),
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
    text: ColorPair::new(Color32::from_gray(180), Color32::from_gray(80)),
    // Same greys and ratios as PRESET_DEFAULT, including its lifted gray(180)
    // dark text — this preset shares that preset's text and background, and
    // only the hues differ.
    weak_text: ColorPair::new(Color32::from_gray(135), Color32::from_gray(112)),
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
    graph_overlay_1: ColorPair::new(Color32::from_gray(240), Color32::from_gray(60)),
    graph_overlay_2: ColorPair::new(Color32::from_rgb(213, 94, 0), Color32::from_rgb(170, 50, 0)),
    graph_overlay_3: ColorPair::new(
        Color32::from_rgb(150, 150, 255),
        Color32::from_rgb(80, 60, 220),
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

    /// Secondary text: hint captions, the mode line, sub-value labels and
    /// timestamps, toolbar group captions.
    ///
    /// No override slot and no `PaletteField`: this is derived for contrast,
    /// not a colour a user picks. It is chosen per preset to clear WCAG AA on
    /// that preset's backgrounds, and a free-form pick would quietly land
    /// under 4.5:1 — which is what egui's default does. Unset, it dims the
    /// text colour to 60% alpha: ~3.8:1 against the Default preset's dark
    /// panel and ~2.9:1 against its light one, and it was 2.7:1 dark before
    /// the primary text was lifted to gray(180). For the same reason it does
    /// not follow a `text` or `background` override, just as the status
    /// colours don't.
    pub(crate) fn weak_text(&self) -> Color32 {
        self.preset.weak_text.pick(self.dark)
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

    /// Line colour of the `k`-th sub-value drawn beside the plotted series.
    ///
    /// Three colours cycle: the graph draws at most four overlays, and the
    /// fourth is told apart by its line style (the non-colour cue every
    /// overlay carries anyway) rather than by a fourth hue that would have to
    /// stay distinct from the other three in all six preset/theme
    /// combinations.
    pub(crate) fn graph_overlay(&self, k: usize) -> Color32 {
        match k % 3 {
            0 => self.resolve(self.overrides.graph_overlay_1, &self.preset.graph_overlay_1),
            1 => self.resolve(self.overrides.graph_overlay_2, &self.preset.graph_overlay_2),
            _ => self.resolve(self.overrides.graph_overlay_3, &self.preset.graph_overlay_3),
        }
    }

    /// Border of an overload band — derives from status_error().
    ///
    /// Overload is the meter reporting a condition, not an absence of data, so
    /// it is drawn as a filled band rather than the dashed gap markers. Using
    /// the error colour ties it to the same signal the reading itself turns
    /// when the meter goes over range.
    pub(crate) fn graph_overload(&self) -> Color32 {
        self.status_error()
    }

    /// Fill of an overload band — derives from status_error(), heavily
    /// transparent so the grid and axis labels stay readable through it.
    ///
    /// `gamma_multiply` rather than the `from_rgba_premultiplied` idiom used
    /// by the other derived colours here: `Color32` is premultiplied, so
    /// pairing full-brightness RGB with a low alpha composites as an additive
    /// glow instead of a faint tint. `gamma_multiply` scales the colour and
    /// the alpha together, which is what a tint actually is.
    pub(crate) fn graph_overload_fill(&self) -> Color32 {
        // Light backgrounds need less: the same factor reads much stronger
        // against white than against the near-black plot area.
        let factor = if self.dark { 0.22 } else { 0.15 };
        self.graph_overload().gamma_multiply(factor)
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
            PaletteField::GraphOverlay1 => self.graph_overlay(0),
            PaletteField::GraphOverlay2 => self.graph_overlay(1),
            PaletteField::GraphOverlay3 => self.graph_overlay(2),
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
        assert_eq!(seen.len(), 21);
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
    fn overload_colors_derive_from_status_error() {
        let overrides = PaletteOverrides {
            status_error: Some(HexColor(Color32::from_rgb(200, 40, 40))),
            ..Default::default()
        };

        for &dark in &[true, false] {
            let tc = ThemeColors::new(dark, ColorPreset::Default, &overrides);
            assert_eq!(
                tc.graph_overload(),
                Color32::from_rgb(200, 40, 40),
                "border must be the error colour itself (dark={dark})"
            );

            // A tint, not an additive glow: every channel and the alpha scale
            // together, so the fill stays below the border on all of them.
            let fill = tc.graph_overload_fill();
            let [fr, fg, fb, fa] = fill.to_array();
            let [br, bg, bb, ba] = tc.graph_overload().to_array();
            assert!(fa < ba, "fill must be translucent (dark={dark})");
            assert!(
                fr <= br && fg <= bg && fb <= bb,
                "fill {fill:?} brighter than its border (dark={dark})"
            );
        }
    }

    /// The fill has to be visible against the plot area but weak enough to
    /// read the grid and the trace through. `luminance()` ignores alpha, so
    /// composite it by hand first — `Color32` is premultiplied, so the blend
    /// is `src + dst * (1 - a)`.
    #[test]
    fn overload_fill_is_a_visible_but_weak_tint() {
        fn composite(src: Color32, dst: Color32) -> Color32 {
            let [sr, sg, sb, sa] = src.to_array();
            let [dr, dg, db, _] = dst.to_array();
            let inv = 1.0 - (sa as f32 / 255.0);
            Color32::from_rgb(
                sr.saturating_add((dr as f32 * inv) as u8),
                sg.saturating_add((dg as f32 * inv) as u8),
                sb.saturating_add((db as f32 * inv) as u8),
            )
        }

        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for &dark in &[true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let bg = tc.plot_background();
                let banded = composite(tc.graph_overload_fill(), bg);
                let ratio = contrast(banded, bg);

                assert!(
                    ratio > 1.10,
                    "banded area {banded:?} is indistinguishable from the plot                      background {bg:?} ({ratio:.3}:1, {preset:?}, dark={dark})"
                );
                assert!(
                    ratio < 3.0,
                    "banded area {banded:?} is too strong against {bg:?}                      ({ratio:.3}:1) — the trace must stay readable through it                      ({preset:?}, dark={dark})"
                );
            }
        }
    }

    /// `.claude/rules/gui.md` requires 3:1 for graphical elements. The band's
    /// border is opaque so it can be checked directly; the fill is translucent
    /// by design and `luminance()` ignores alpha, so it is not checked here.
    #[test]
    fn overload_border_meets_graphical_contrast_on_the_plot_area() {
        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for &dark in &[true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let ratio = contrast(tc.graph_overload(), tc.plot_background());
                assert!(
                    ratio >= 3.0,
                    "overload border {:?} on plot background {:?} is {ratio:.2}:1, below 3:1 ({preset:?}, dark={dark})",
                    tc.graph_overload(),
                    tc.plot_background()
                );
            }
        }
    }

    /// Overlay traces are graphical elements on the plot area, so
    /// `.claude/rules/gui.md` asks for 3:1 against it — in every preset and
    /// both themes, the case that historically broke light mode.
    #[test]
    fn overlay_colors_meet_graphical_contrast_on_the_plot_area() {
        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for &dark in &[true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let bg = tc.plot_background();
                for k in 0..3 {
                    let c = tc.graph_overlay(k);
                    let ratio = contrast(c, bg);
                    assert!(
                        ratio >= 3.0,
                        "overlay {k} {c:?} on plot background {bg:?} is {ratio:.2}:1, below 3:1 ({preset:?}, dark={dark})"
                    );
                }
            }
        }
    }

    /// The overlays and the plotted series are drawn on the same axes at the
    /// same time. Line style tells them apart without colour, but two traces
    /// sharing a colour *and* differing only in dash length is a needlessly
    /// hard read — so the four have to be four distinct colours.
    #[test]
    fn overlay_colors_differ_from_each_other_and_the_data_line() {
        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for &dark in &[true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let line = tc.graph_line();
                for k in 0..3 {
                    assert_ne!(
                        tc.graph_overlay(k),
                        line,
                        "overlay {k} matches the data line ({preset:?}, dark={dark})"
                    );
                    for j in 0..k {
                        assert_ne!(
                            tc.graph_overlay(k),
                            tc.graph_overlay(j),
                            "overlays {j} and {k} share a colour ({preset:?}, dark={dark})"
                        );
                    }
                }
            }
        }
    }

    /// Only three overlay colours are defined; a fourth overlay wraps back to
    /// the first and is distinguished by its line style.
    #[test]
    fn overlay_colors_cycle_after_three() {
        let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        assert_eq!(tc.graph_overlay(3), tc.graph_overlay(0));
        assert_eq!(tc.graph_overlay(4), tc.graph_overlay(1));
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

    /// Secondary text has to clear AA too. `apply_color_overrides` pins
    /// `Visuals::weak_text_color` to this, replacing egui's default of the
    /// text colour at 60% alpha (~2.9:1 light on the panel; ~2.7:1 dark
    /// before the primary text was lifted, ~3.8:1 after).
    ///
    /// Three grounds, because weak text lands on all three:
    /// - `background()` — panel and window fill, where most of it is drawn.
    /// - the faint frame fill, which is `Visuals::faint_bg_color`
    ///   (`Color32::from_additive_luminance(5)`) painted over the panel, so it
    ///   composites to the background with 5 added to each channel. The
    ///   toolbar group frames in `graph.rs` use it.
    /// - `plot_background()` — the App assigns it to `extreme_bg_color`, which
    ///   is also the `TextEdit` background, and egui draws `hint_text` in the
    ///   weak colour (the Y-axis / envelope / reference fields).
    ///
    /// The plot *area* itself is not a case: `paint_plot_key` uses
    /// `text_color()` and the cursor and overlay labels use theme colours, so
    /// no weak text is painted there.
    #[test]
    fn weak_text_meets_wcag_aa_on_every_ground_it_lands_on() {
        for preset in [
            ColorPreset::Default,
            ColorPreset::HighContrast,
            ColorPreset::ColorblindSafe,
        ] {
            for dark in [true, false] {
                let tc = ThemeColors::new(dark, preset, &PaletteOverrides::default());
                let bg = tc.background();
                let faint = Color32::from_rgb(
                    bg.r().saturating_add(5),
                    bg.g().saturating_add(5),
                    bg.b().saturating_add(5),
                );
                let fg = tc.weak_text();
                for (name, ground) in [
                    ("panel background", bg),
                    ("faint frame fill", faint),
                    ("plot / text-edit background", tc.plot_background()),
                ] {
                    let ratio = contrast(fg, ground);
                    assert!(
                        ratio >= 4.5,
                        "{preset:?} {} mode: weak_text {fg:?} on {name} {ground:?} is {ratio:.2}:1, below WCAG AA 4.5:1",
                        if dark { "dark" } else { "light" },
                    );
                }
            }
        }
    }

    /// Weak text is a *derived* colour, not one the user picks: it has no
    /// `PaletteField`, and overriding text or background must not drag it off
    /// the value verified above — same rule as the status colours.
    #[test]
    fn weak_text_ignores_text_and_background_overrides() {
        let overrides = PaletteOverrides {
            background: Some(HexColor(Color32::from_rgb(10, 20, 30))),
            text: Some(HexColor(Color32::from_rgb(200, 210, 220))),
            ..Default::default()
        };
        for &dark in &[true, false] {
            let plain = ThemeColors::new(dark, ColorPreset::Default, &PaletteOverrides::default());
            let overridden = ThemeColors::new(dark, ColorPreset::Default, &overrides);
            assert_eq!(
                overridden.weak_text(),
                plain.weak_text(),
                "weak_text followed an override (dark={dark})"
            );
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
