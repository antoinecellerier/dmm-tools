//! The three built-in palettes as data: each colour's dark and light value
//! per preset. `ThemeColors` resolves them against the user's overrides.

use eframe::egui::Color32;

use crate::settings::ColorPreset;

/// A dark/light color pair.
pub(super) struct ColorPair(Color32, Color32);

impl ColorPair {
    const fn new(dark: Color32, light: Color32) -> Self {
        Self(dark, light)
    }

    pub(super) fn pick(&self, dark: bool) -> Color32 {
        if dark { self.0 } else { self.1 }
    }
}

/// All base colors for a preset. Each holds a dark and light variant.
pub(super) struct PresetColors {
    // -- UI chrome --
    pub(super) background: ColorPair,
    pub(super) text: ColorPair,
    /// Secondary text. Pinned per preset rather than dimmed from `text`, so
    /// that each preset's default clears AA on its own panel, faint frame and
    /// text-edit background — see `ThemeColors::weak_text`.
    pub(super) weak_text: ColorPair,
    pub(super) button: ColorPair,
    /// Separators, frames and outlines. `None` leaves them on the grey egui
    /// itself ships — see `ThemeColors::border`.
    pub(super) border: Option<ColorPair>,
    // -- Status indicators --
    pub(super) status_ok: ColorPair,
    pub(super) status_warning: ColorPair,
    pub(super) status_error: ColorPair,
    pub(super) status_inactive: ColorPair,
    pub(super) accent: ColorPair,
    // -- Graph --
    pub(super) graph_line: ColorPair,
    pub(super) graph_gap: ColorPair,
    pub(super) graph_mean: ColorPair,
    pub(super) graph_ref: ColorPair,
    pub(super) graph_crossing: ColorPair,
    pub(super) graph_cursor: ColorPair,
    pub(super) graph_envelope: ColorPair,
    pub(super) graph_overlay_1: ColorPair,
    pub(super) graph_overlay_2: ColorPair,
    pub(super) graph_overlay_3: ColorPair,
    pub(super) plot_background: ColorPair,
    pub(super) graph_crosshair: ColorPair,
    pub(super) graph_marker: ColorPair,
    // -- Minimap --
    pub(super) minimap_viewport: ColorPair,
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
    // it is the grey button captions keep (`default_button_text`), so labels
    // match the buttons beside them.
    text: ColorPair::new(Color32::from_gray(180), Color32::from_gray(80)),
    // Paired with the text above as the dimmer of two visible tiers: 135 vs
    // 180 on gray(27) reads as secondary and still clears AA at 4.79:1
    // (4.54:1 on the faint frame fill gray(32), 5.51:1 on the gray(10)
    // text-edit background). Light: 112 vs 80, 4.66:1 on gray(248), 4.87:1 on
    // gray(253), 4.95:1 on gray(255).
    weak_text: ColorPair::new(Color32::from_gray(135), Color32::from_gray(112)),
    button: ColorPair::new(Color32::from_gray(60), Color32::from_gray(230)),
    // Decorative in this preset: separators and frames stay on egui's own
    // grey rather than a copy of it.
    border: None,
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
    // A hue no other graph colour uses. Text on the plot, the panel and a
    // text field (the list's numbers, the flags' labels): 6.94:1 on gray(10),
    // 5.71:1 on the faint fill gray(32) / 7.05:1 on white, 6.63:1 on
    // gray(248).
    graph_marker: ColorPair::new(
        Color32::from_rgb(240, 100, 200),
        Color32::from_rgb(170, 0, 120),
    ),
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
    // The one preset that pins a border, and the reason the field exists:
    // egui's gray(60)/gray(190) separators sit near 1.6:1 on this preset's
    // black and white panels, so panel edges and the plot outline all but
    // vanish exactly where they are wanted most. 5.32:1 on gray(0) and
    // 4.54:1 on gray(255) — well past the 3:1 bar for a graphical element,
    // and still a line rather than a second text tier.
    border: Some(ColorPair::new(
        Color32::from_gray(128),
        Color32::from_gray(118),
    )),
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
    // Rose, beside the magenta trigger-crossing diamonds, which differ in
    // shape: 9.19:1 on gray(0), 7.35:1 on gray(255).
    graph_marker: ColorPair::new(
        Color32::from_rgb(255, 130, 190),
        Color32::from_rgb(170, 0, 90),
    ),
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
    // Same grounds as PRESET_DEFAULT, so the same decision: egui's grey.
    border: None,
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
    // Okabe–Ito's reddish purple, the eight hues being taken: the Ref lines
    // share it, but they are horizontal and markers are vertical, dotted
    // and numbered. 5.32:1 on the faint fill gray(32) / 6.71:1 on gray(248).
    graph_marker: ColorPair::new(
        Color32::from_rgb(204, 121, 167),
        Color32::from_rgb(140, 60, 100),
    ),
    minimap_viewport: ColorPair::new(
        Color32::from_rgb(86, 150, 233),
        Color32::from_rgb(0, 70, 180),
    ),
};

pub(super) fn preset_colors(preset: ColorPreset) -> &'static PresetColors {
    match preset {
        ColorPreset::Default => &PRESET_DEFAULT,
        ColorPreset::HighContrast => &PRESET_HIGH_CONTRAST,
        ColorPreset::ColorblindSafe => &PRESET_COLORBLIND_SAFE,
    }
}
