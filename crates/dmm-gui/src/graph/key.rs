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

/// Space around the key, and between its border and its entries.
const PAD: f32 = 6.0;

/// Space between an entry's line sample and its name.
const GAP: f32 = 6.0;

/// Space between two entries of a key laid out in one row.
const ENTRY_GAP: f32 = 12.0;

/// Where the plot key goes, and how its entries are laid out in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum KeyLayout {
    /// One entry under the other, in a panel as wide as the widest.
    Column(egui::Rect),
    /// The entries side by side, for a plot too short for the column.
    Row(egui::Rect),
    /// No key: the plot has room for neither.
    Hidden,
}

/// The first shape of the key that fits in the plot's `body` with [`PAD`]
/// around it: the column, else one row, else none. A key larger than its
/// plot would be cut at the plot's edge, some of its entries with it.
///
/// `text_widths` holds the width of each entry's name and `row_height` the
/// height of one entry.
pub(super) fn key_layout(body: egui::Rect, text_widths: &[f32], row_height: f32) -> KeyLayout {
    let entry_width = |text: f32| KEY_SAMPLE_WIDTH + GAP + text;
    let at_corner =
        |size: egui::Vec2| egui::Rect::from_min_size(body.left_top() + egui::vec2(PAD, PAD), size);
    let widest = text_widths.iter().copied().fold(0.0_f32, f32::max);
    let column = at_corner(egui::vec2(
        PAD * 2.0 + entry_width(widest),
        PAD * 2.0 + row_height * text_widths.len() as f32,
    ));
    let row = at_corner(egui::vec2(
        PAD * 2.0
            + text_widths.iter().copied().map(entry_width).sum::<f32>()
            + ENTRY_GAP * text_widths.len().saturating_sub(1) as f32,
        PAD * 2.0 + row_height,
    ));
    let fits = |key: egui::Rect| body.shrink(PAD).contains_rect(key);
    if fits(column) {
        KeyLayout::Column(column)
    } else if fits(row) {
        KeyLayout::Row(row)
    } else {
        KeyLayout::Hidden
    }
}

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

    /// Paint the plot key in the top-left of the plot area, in the shape
    /// [`key_layout`] finds room for.
    ///
    /// A key, not a control: egui_plot's own `Legend` renders show/hide
    /// checkboxes, but `Plot::reset()` — which pins the view every frame while
    /// keeping pointer events — also clears the plot's `hidden_items`, so a
    /// click on one had no effect past the frame it happened in. The **Show:**
    /// chips in the toolbar are the control instead.
    ///
    /// Returns the key's rect, which the overlay labels keep off: `None`
    /// when no key is drawn, for want of entries or of room.
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

        let text_color = ui.visuals().text_color();
        let painter = ui.painter_at(plot_rect);

        let galleys: Vec<_> = entries
            .iter()
            .map(|(name, _)| painter.layout_no_wrap(name.clone(), font.clone(), text_color))
            .collect();
        let text_widths: Vec<f32> = galleys.iter().map(|g| g.size().x).collect();
        let row_height = galleys.iter().map(|g| g.size().y).fold(font.size, f32::max);
        let (rect, in_row) = match key_layout(plot_rect, &text_widths, row_height) {
            KeyLayout::Column(rect) => (rect, false),
            KeyLayout::Row(rect) => (rect, true),
            KeyLayout::Hidden => return None,
        };
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

        let mut left = rect.left() + PAD;
        for (i, ((_, style), galley)) in entries.iter().zip(&galleys).enumerate() {
            let line = if in_row { 0.0 } else { i as f32 };
            let top = rect.top() + PAD + row_height * line;
            let mid = top + row_height / 2.0;
            let (color, line_style) = match style {
                KeyStyle::Plotted => (tc.graph_line(), egui_plot::LineStyle::Solid),
                KeyStyle::Overlay(k) => Self::overlay_color_and_style(tc, *k, solid),
            };
            painter.extend(styled_line(
                egui::pos2(left, mid),
                egui::pos2(left + KEY_SAMPLE_WIDTH, mid),
                egui::Stroke::new(1.5, color),
                line_style,
            ));
            painter.galley(
                egui::pos2(left + KEY_SAMPLE_WIDTH + GAP, top),
                galley.clone(),
                text_color,
            );
            if in_row {
                left += KEY_SAMPLE_WIDTH + GAP + text_widths[i] + ENTRY_GAP;
            }
        }
        Some(rect)
    }
}
