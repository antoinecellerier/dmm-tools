//! The plot with nothing in it: a sentence in its middle saying what will
//! bring readings, and the time axis's origin under its left edge while the
//! axis has no labels of its own.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};
use egui_plot::{PlotBounds, PlotGeometry, PlotItem, PlotItemBase, PlotTransform};
use std::borrow::Cow;
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::Graph;
use super::time::format_time_axis_label;
use crate::theme::ThemeColors;

/// Whether readings are on their way to the graph, as the app knows it:
/// what an empty plot says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Feed {
    /// No meter is connected, and connecting one is what brings readings.
    #[default]
    NoMeter,
    /// Readings are due: a meter is connecting, connected or being
    /// reconnected to, or a file is being imported.
    Expected,
    /// Neither: none comes as things stand, and connecting a meter is not
    /// what brings one — acquisition is paused, or the session plays a
    /// recording that has ended or is not started. The plot says nothing.
    Held,
}

/// The room between the sentence and the edge of the patch it stands on.
const PATCH_MARGIN: egui::Vec2 = egui::vec2(14.0, 5.0);

/// The room between the patch and a line it has moved clear of.
const LINE_GAP: f32 = 4.0;

impl Graph {
    /// Whether the graph holds nothing: no reading of any series, and no
    /// over-range or no-reading stretch waiting for its first.
    pub(super) fn is_blank(&self) -> bool {
        self.first_point_time().is_none() && self.breaks.heard_until.is_none()
    }

    /// The sentence an empty plot shows, if it has one to show.
    fn empty_message<'a>(&self, l10n: &'a Localizer) -> Option<Cow<'a, str>> {
        match self.is_blank().then_some(self.feed)? {
            Feed::NoMeter => Some(tr!(l10n, "graph-empty-no-meter")),
            Feed::Expected => Some(tr!(l10n, "graph-empty-waiting")),
            Feed::Held => None,
        }
    }

    /// The same for a screen reader, as the last sentence of the plot's
    /// name: said whether or not the plot has room to draw it.
    pub(super) fn empty_spoken<'a>(&self, l10n: &'a Localizer) -> Option<Cow<'a, str>> {
        match self.is_blank().then_some(self.feed)? {
            Feed::NoMeter => Some(tr!(l10n, "graph-empty-no-meter.a11y")),
            Feed::Expected => Some(tr!(l10n, "graph-empty-waiting.a11y")),
            Feed::Held => None,
        }
    }

    /// The sentence of a plot with nothing in it, as the item that draws
    /// it: `None` once the graph holds something, and where there is
    /// nothing true to say. `lines` are the heights of the horizontal lines
    /// the plot draws all the same, which the sentence keeps clear of: an
    /// alarm's limits. A reference line is read against what is plotted, so
    /// a plot nothing was read on has none.
    pub(super) fn empty_note(
        &self,
        ui: &Ui,
        l10n: &Localizer,
        tc: &ThemeColors,
        lines: impl Iterator<Item = f64>,
    ) -> Option<EmptyNote> {
        let message = self.empty_message(l10n)?;
        let font = egui::TextStyle::Body.resolve(ui.style());
        let color = ui.visuals().weak_text_color();
        Some(EmptyNote {
            galley: ui
                .painter()
                .layout_no_wrap(message.into_owned(), font, color),
            color,
            background: tc.plot_background(),
            lines: lines.collect(),
            base: PlotItemBase::new(String::new()),
        })
    }
}

/// The time axis's zero under the left edge of the plot `body`, written and
/// placed as the axis writes a tick there: for a time axis that has no
/// labels of its own, its view no wider than the one reading it holds, or
/// than none.
pub(super) fn paint_time_origin(ui: &Ui, body: egui::Rect) {
    let painter = ui.painter();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let color = ui.visuals().text_color();
    let origin = painter.layout_no_wrap(format_time_axis_label(0.0, 1.0), font, color);
    let at = egui::pos2(body.left() - origin.size().x / 2.0, body.bottom());
    painter.galley(at, origin, color);
}

/// An empty plot's sentence in the middle of the plot, on a patch of the
/// plot's background so no gridline runs through it. A plot too small for
/// the patch goes without the sentence.
///
/// An empty plot still draws an alarm's limits and the crosshair, which say
/// something where they are, and a patch painted once the plot is drawn cut
/// whichever crossed its middle. So this is a plot item, added before every
/// other: the patch covers the grid and nothing else. And the sentence
/// moves down clear of a limit it would stand on, as one at 0 is in the
/// middle of an empty plot: under the line, whose label is over it.
pub(super) struct EmptyNote {
    galley: Arc<egui::Galley>,
    color: egui::Color32,
    background: egui::Color32,
    /// The heights of the horizontal lines to keep clear of.
    lines: Vec<f64>,
    base: PlotItemBase,
}

impl PlotItem for EmptyNote {
    fn shapes(&self, _ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<egui::Shape>) {
        let body = *transform.frame();
        let size = self.galley.size() + 2.0 * PATCH_MARGIN;
        let mut patch = egui::Rect::from_center_size(body.center(), size);
        // From the top down, so a line under the one moved clear of is met
        // where the patch then stands.
        let mut lines: Vec<f32> = self
            .lines
            .iter()
            .map(|&y| transform.position_from_point_y(y))
            .collect();
        lines.sort_by(f32::total_cmp);
        for y in lines {
            if patch.y_range().expand(LINE_GAP).contains(y) {
                patch = patch.translate(egui::vec2(0.0, y + LINE_GAP - patch.top()));
            }
        }
        if !body.contains_rect(patch) {
            return;
        }
        shapes.push(egui::Shape::rect_filled(patch, 0.0, self.background));
        shapes.push(egui::Shape::galley(
            patch.min + PATCH_MARGIN,
            self.galley.clone(),
            self.color,
        ));
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn color(&self) -> egui::Color32 {
        self.color
    }

    fn allow_hover(&self) -> bool {
        false
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        PlotBounds::NOTHING
    }

    fn base(&self) -> &PlotItemBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut PlotItemBase {
        &mut self.base
    }
}
