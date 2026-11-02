//! The plot key: one row per drawn series, its line sample styled as the
//! trace is.

use dmm_shared::l10n::Localizer;
use eframe::egui::{self, Ui};

use super::Graph;
use super::render::{OverlayTrace, styled_line};
use crate::theme::ThemeColors;

/// How one row of the plot key draws its line sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyStyle {
    /// The plotted series: solid, in the graph line colour.
    Plotted,
    /// A sub-value trace, identified by its overlay index — the same index
    /// `overlay_color_and_style` uses for the line itself.
    Overlay(usize),
}

/// Minimum font size for the plot key, per `.claude/rules/gui.md`. The body
/// text style is normally larger; this only bites if a user's style shrinks it.
const MIN_KEY_FONT_SIZE: f32 = 11.0;

/// Width of the line sample drawn at the left of each key row.
const KEY_SAMPLE_WIDTH: f32 = 24.0;

impl Graph {
    /// Name the plotted series goes by on the plot and in the key.
    pub(super) fn plotted_series_name(&self) -> String {
        self.current_series
            .clone()
            .unwrap_or_else(|| self.main_name().to_string())
    }

    /// Rows of the plot key: the plotted series when it is drawn, then each
    /// drawn overlay in `self.overlays` order.
    ///
    /// Empty when nothing is overlaid — a single-series graph gets no key, and
    /// looks exactly as it did before sub-values existed. Takes the traces
    /// `show_main` is about to draw rather than recomputing them, so the key
    /// cannot list a line that isn't there — the plotted series included,
    /// which has no line while a held meter sends only a sub-value.
    ///
    /// With a trace on an axis of its own, every row names its unit, which is
    /// what ties a line to its axis without leaning on colour.
    pub(super) fn key_entries(
        &self,
        l10n: &Localizer,
        drawn: &[OverlayTrace],
        plotted_drawn: bool,
    ) -> Vec<(String, KeyStyle)> {
        if drawn.is_empty() {
            return Vec::new();
        }
        let with_units = drawn
            .iter()
            .any(|(_, label, _)| self.overlay_unit(label) != self.current_unit);
        let name = |label: &str, unit: &str| {
            let label = self.series_text(l10n, label);
            if with_units && !unit.is_empty() {
                format!("{label} ({unit})")
            } else {
                label.into_owned()
            }
        };
        let mut entries = Vec::with_capacity(drawn.len() + 1);
        if plotted_drawn {
            entries.push((
                name(&self.plotted_series_name(), &self.current_unit),
                KeyStyle::Plotted,
            ));
        }
        entries.extend(
            drawn.iter().map(|(k, label, _)| {
                (name(label, self.overlay_unit(label)), KeyStyle::Overlay(*k))
            }),
        );
        entries
    }

    /// Paint the plot key in the top-left of the plot area.
    ///
    /// A key, not a control: egui_plot's own `Legend` renders show/hide
    /// checkboxes, but `Plot::reset()` — which pins the view every frame while
    /// keeping pointer events — also clears the plot's `hidden_items`, so a
    /// click on one had no effect past the frame it happened in. The **Show:**
    /// chips in the toolbar are the control instead.
    ///
    /// Returns the key's rect, which the overlay labels keep off.
    pub(super) fn paint_plot_key(
        ui: &Ui,
        plot_rect: egui::Rect,
        entries: &[(String, KeyStyle)],
        tc: &ThemeColors,
        solid: bool,
    ) -> Option<egui::Rect> {
        if entries.is_empty() {
            return None;
        }
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = font.size.max(MIN_KEY_FONT_SIZE);

        const PAD: f32 = 6.0;
        const GAP: f32 = 6.0;
        let text_color = ui.visuals().text_color();
        let painter = ui.painter_at(plot_rect);

        // Measure first so the panel is exactly as wide as its widest row.
        let galleys: Vec<_> = entries
            .iter()
            .map(|(name, _)| painter.layout_no_wrap(name.clone(), font.clone(), text_color))
            .collect();
        let text_width = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max);
        let row_height = galleys.iter().map(|g| g.size().y).fold(font.size, f32::max);
        let width = PAD * 2.0 + KEY_SAMPLE_WIDTH + GAP + text_width;
        let height = PAD * 2.0 + row_height * entries.len() as f32;
        let rect = egui::Rect::from_min_size(
            plot_rect.left_top() + egui::vec2(PAD, PAD),
            egui::vec2(width, height),
        );
        // `window_fill`, not `extreme_bg_color`: the App assigns the latter
        // the plot's own background, so a key painted in it would be invisible
        // apart from its border. `window_fill` is the panel ground either
        // theme pairs with `text_color`, so the names keep their normal
        // contrast, and 85% alpha keeps the grid faintly readable underneath
        // instead of punching a hole in the plot.
        painter.rect_filled(rect, 4.0, ui.visuals().window_fill.gamma_multiply(0.85));
        painter.rect_stroke(
            rect,
            4.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );

        for (i, ((_, style), galley)) in entries.iter().zip(&galleys).enumerate() {
            let top = rect.top() + PAD + row_height * i as f32;
            let mid = top + row_height / 2.0;
            let (color, line_style) = match style {
                KeyStyle::Plotted => (tc.graph_line(), egui_plot::LineStyle::Solid),
                KeyStyle::Overlay(k) => Self::overlay_color_and_style(tc, *k, solid),
            };
            painter.extend(styled_line(
                egui::pos2(rect.left() + PAD, mid),
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH, mid),
                egui::Stroke::new(1.5, color),
                line_style,
            ));
            painter.galley(
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH + GAP, top),
                galley.clone(),
                text_color,
            );
        }
        Some(rect)
    }
}
