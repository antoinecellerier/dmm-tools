use eframe::egui::Color32;

/// Theme-aware color palette. All colors have dark and light variants chosen
/// for WCAG AA contrast on their respective backgrounds.
pub(crate) struct ThemeColors {
    dark: bool,
}

impl ThemeColors {
    pub(crate) fn new(dark: bool) -> Self {
        Self { dark }
    }

    // -- Status colors (used across app and graph) --

    /// Green: connected, live, success.
    pub(crate) fn green(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(60, 180, 75)
        } else {
            Color32::from_rgb(0, 140, 30)
        }
    }

    /// Orange: warnings, reconnecting, paused.
    pub(crate) fn orange(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(200, 120, 0)
        } else {
            Color32::from_rgb(180, 80, 0)
        }
    }

    /// Red: errors, toast failures.
    pub(crate) fn red(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(220, 60, 60)
        } else {
            Color32::from_rgb(180, 0, 0)
        }
    }

    /// Gray: disconnected/muted state.
    pub(crate) fn gray(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(150, 150, 150)
        } else {
            Color32::from_rgb(120, 120, 120)
        }
    }

    /// Blue accent: active flags, cursors, viewport indicators.
    pub(crate) fn blue_accent(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(100, 180, 255)
        } else {
            Color32::from_rgb(0, 100, 200)
        }
    }

    // -- Graph-specific colors --

    /// Live indicator green (slightly different light variant from status green).
    pub(crate) fn live_green(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(60, 180, 75)
        } else {
            Color32::from_rgb(0, 130, 30)
        }
    }

    /// Main data line.
    pub(crate) fn graph_line(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(220, 120, 120)
        } else {
            Color32::from_rgb(180, 40, 40)
        }
    }

    /// Gap indicator lines.
    pub(crate) fn graph_gap(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(220, 80, 80)
        } else {
            Color32::from_rgba_premultiplied(200, 0, 0, 180)
        }
    }

    /// Mean overlay line.
    pub(crate) fn graph_mean(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(100, 200, 100)
        } else {
            Color32::from_rgb(0, 120, 0)
        }
    }

    /// Reference line overlay.
    pub(crate) fn graph_ref(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(200, 200, 100)
        } else {
            Color32::from_rgb(140, 100, 0)
        }
    }

    /// Trigger crossing markers.
    pub(crate) fn graph_crossing(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(255, 220, 100)
        } else {
            Color32::from_rgb(150, 100, 0)
        }
    }

    /// Cursor lines and labels.
    pub(crate) fn graph_cursor(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(255, 180, 100)
        } else {
            Color32::from_rgb(180, 70, 0)
        }
    }

    /// Cursor dimmed variant (for horizontal Y-value lines).
    pub(crate) fn graph_cursor_dim(&self) -> Color32 {
        if self.dark {
            Color32::from_rgba_premultiplied(255, 180, 100, 80)
        } else {
            Color32::from_rgb(180, 70, 0)
        }
    }

    /// Cursor delta readout (ΔT/ΔV text).
    pub(crate) fn graph_cursor_delta(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(255, 180, 100)
        } else {
            Color32::from_rgb(180, 80, 0)
        }
    }

    /// Min/max envelope lines.
    pub(crate) fn graph_envelope(&self) -> Color32 {
        if self.dark {
            Color32::from_rgba_premultiplied(100, 150, 200, 80)
        } else {
            Color32::from_rgb(0, 60, 160)
        }
    }

    /// Minimap data line (semi-transparent).
    pub(crate) fn minimap_line(&self) -> Color32 {
        if self.dark {
            Color32::from_rgba_premultiplied(220, 120, 120, 200)
        } else {
            Color32::from_rgba_premultiplied(180, 30, 30, 220)
        }
    }

    /// Minimap viewport bracket indicator.
    pub(crate) fn minimap_viewport(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_rgb(0, 70, 200)
        }
    }

    /// Recording buffer full warning (slightly warmer orange).
    pub(crate) fn recording_full_warning(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(230, 160, 40)
        } else {
            Color32::from_rgb(180, 100, 0)
        }
    }
}
