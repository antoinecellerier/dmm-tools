//! The graph's key: the sample of its line each **Plot:** and **Show:** chip
//! carries, drawn as the trace is.
//!
//! The chips are the only key. One painted over the plot repeated them and
//! covered the traces, and egui_plot's own `Legend` can't stand in: it
//! renders show/hide checkboxes, but `Plot::reset()` — which pins the view
//! every frame while keeping pointer events — also clears the plot's
//! `hidden_items`, so a click on one had no effect past the frame it
//! happened in.

use eframe::egui;

use super::Graph;
use super::pattern::{period, walk_pattern};
use crate::theme::ThemeColors;

/// Which of the graph's lines a sample stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyStyle {
    /// The plotted series: solid, in the graph line colour.
    Plotted,
    /// A sub-value trace, identified by its overlay index — the same index
    /// `overlay_color_and_style` uses for the line itself.
    Overlay(usize),
}

impl KeyStyle {
    /// Colour and line style of the trace, as the plot draws it.
    pub(super) fn line(
        self,
        tc: &ThemeColors,
        solid: bool,
    ) -> (egui::Color32, egui_plot::LineStyle) {
        match self {
            KeyStyle::Plotted => (tc.graph_line(), egui_plot::LineStyle::Solid),
            KeyStyle::Overlay(k) => Graph::overlay_color_and_style(tc, k, solid),
        }
    }
}

/// Length of a line sample: room for two dashes of the loosest pattern.
pub(super) const SAMPLE_WIDTH: f32 = 24.0;

/// A line sample's stroke width.
pub(super) const SAMPLE_STROKE: f32 = 1.5;

/// A sample of a line in `color` and `style`, [`SAMPLE_WIDTH`] long from
/// `left`, walked by the plot's own pattern walker.
///
/// The pattern is centred on the sample: a dot in its middle, or the
/// middle of a gap between two dashes, so both ends look alike.
pub(super) fn line_sample(
    left: egui::Pos2,
    color: egui::Color32,
    style: egui_plot::LineStyle,
) -> Vec<egui::Shape> {
    let points = [left, left + egui::vec2(SAMPLE_WIDTH, 0.0)];
    let stroke = egui::Stroke::new(SAMPLE_STROKE, color);
    let Some(period) = period(style) else {
        return vec![egui::Shape::line_segment(points, stroke)];
    };
    // The pattern's own middle: a dot's centre, or a gap's.
    let middle = match style {
        egui_plot::LineStyle::Dashed { length } => (period + f64::from(length)) / 2.0,
        _ => 0.0,
    };
    let mut shapes = Vec::new();
    walk_pattern(
        &points,
        middle - f64::from(SAMPLE_WIDTH) / 2.0,
        style,
        stroke,
        false,
        egui::Rect::EVERYTHING,
        &mut shapes,
    );
    shapes
}
