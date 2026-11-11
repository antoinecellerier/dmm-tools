//! The statistics and histogram glue: which readings the figures and the
//! histogram strip count, the caches keyed on them, and painting the strip
//! beside the plot.

use dmm_lib::stats::RunningStats;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};
use std::borrow::Cow;

use super::{Graph, LEFT_AXIS_ROOM, MIN_PLOT_BESIDE_STRIP, WatchedLimits, histogram};
use crate::theme::ThemeColors;

/// A run of history readings, which a figure computed over them is kept
/// against while the readings stay the same: the history's epoch, then the
/// sequence numbers of the first reading and one past the last. Readings
/// only arrive at the back and leave from the front, so the same numbers in
/// the same epoch are the same readings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ReadingsKey(u64, u64, u64);

/// What a histogram is binned from: the visible readings, those between the
/// cursors, the Y range, the bins it is cut into, the decimals it writes,
/// and the language a screen reader hears it in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HistogramKey {
    pub(super) view: ReadingsKey,
    pub(super) interval: Option<ReadingsKey>,
    /// Over-range readings among the view's and the interval's, which an
    /// overload still going on adds to without a new reading.
    pub(super) overloads: (u32, Option<u32>),
    pub(super) range: (f64, f64),
    pub(super) bins: usize,
    pub(super) decimals: (usize, usize),
    /// The language's id: the spoken text is kept with the bins.
    pub(super) language: &'static str,
}

/// What a graph `width` wide leaves the histogram strip beside a plot of
/// [`MIN_PLOT_BESIDE_STRIP`].
pub(super) fn strip_room(width: f32) -> f32 {
    width - LEFT_AXIS_ROOM - MIN_PLOT_BESIDE_STRIP
}

/// A histogram kept with what it was binned from and what a screen reader
/// hears of it, both redone only when what it is binned from changes.
#[derive(Debug, Clone)]
pub(super) struct BinnedHistogram {
    pub(super) key: HistogramKey,
    /// `None` with no Y range to bin over.
    pub(super) hist: Option<histogram::Histogram>,
    pub(super) spoken: String,
}

/// Figures over a run of readings, kept with the readings they cover: their
/// statistics and integral.
#[derive(Debug, Clone)]
pub(super) struct RangeFigures {
    pub(super) key: ReadingsKey,
    pub(super) stats: RunningStats,
    pub(super) integral: Option<f64>,
}

impl Graph {
    /// The histogram's divider at `rect`, keeping the size it is dragged to.
    pub(super) fn histogram_divider(
        &mut self,
        ui: &Ui,
        l10n: &Localizer,
        rect: egui::Rect,
        place: histogram::Placement,
    ) {
        if let Some(size) = histogram::divider(ui, l10n, rect, place) {
            match place.dock {
                histogram::Dock::Side => self.histogram_sizes.0 = size,
                histogram::Dock::Below => self.histogram_sizes.1 = size,
            }
            self.histogram_resized = true;
        }
    }

    /// The histogram's sizes when a divider moved since the last call, for
    /// the settings to keep.
    pub(crate) fn take_histogram_resize(&mut self) -> Option<(f32, f32)> {
        std::mem::take(&mut self.histogram_resized).then_some(self.histogram_sizes)
    }

    /// Whether the histogram is drawn: its chip is on and the plotted trace
    /// it bins is shown.
    pub(super) fn histogram_shown(&self) -> bool {
        self.show_histogram && !self.main_hidden
    }

    /// The least height the histogram's band takes under the minimap in a
    /// graph `width` wide, divider included, and 0 with no band: the layout
    /// reserves it, so a short column scrolls rather than squash the band.
    /// Not the height dragged, which [`histogram::place`] gives up to fit.
    pub(crate) fn histogram_band_height(&self, width: f32) -> f32 {
        if self.histogram_shown() && !histogram::docks_beside(strip_room(width)) {
            histogram::MIN_BAND_HEIGHT + histogram::DIVIDER
        } else {
            0.0
        }
    }

    /// What the Histogram chip says when hovered: why nothing shows while it
    /// is on and the plotted trace is hidden.
    pub(super) fn histogram_hover<'a>(&self, l10n: &'a Localizer, keys: &str) -> Cow<'a, str> {
        let state = if self.show_histogram && self.main_hidden {
            "hidden"
        } else {
            "shown"
        };
        tr!(l10n, "graph-histogram.hover", state = state, keys = keys)
    }

    /// The key of the history readings `range`.
    pub(super) fn readings_key(&self, range: std::ops::Range<usize>) -> ReadingsKey {
        let first = self.pushed_total.saturating_sub(self.history.len() as u64);
        ReadingsKey(
            self.history_epoch,
            first + range.start as u64,
            first + range.end as u64,
        )
    }

    /// The history readings between the cursors, both included, once both
    /// are placed.
    pub(super) fn cursor_range(&self) -> Option<std::ops::Range<usize>> {
        let (a, b) = self.cursors.pair()?;
        let (start, end) = self.visible_index_range(a.min(b), a.max(b));
        Some(start..end)
    }

    /// The readings between the cursors and their integral, computed again
    /// only when those readings change: a static period, paused or behind
    /// the newest reading, costs a pass once rather than every frame the
    /// mouse moves. `None` until both cursors are placed.
    pub(super) fn cursor_figures(&mut self) -> Option<(RunningStats, Option<f64>)> {
        let range = self.cursor_range()?;
        let cached = self.cursor_figures.take();
        let figures = self.figures_over(range, cached);
        let out = (figures.stats.clone(), figures.integral);
        self.cursor_figures = Some(figures);
        Some(out)
    }

    /// The visible window's statistics and integral (in unit·seconds),
    /// computed again only when the readings in view change: the panel and
    /// the mean line read them every frame, and a still view costs one pass,
    /// not one a frame.
    pub(crate) fn visible_figures(&mut self) -> (RunningStats, Option<f64>) {
        let (x_min, x_max) = self.view_bounds();
        let (start, end) = self.visible_index_range(x_min, x_max);
        let cached = self.visible_figures.take();
        let figures = self.figures_over(start..end, cached);
        let out = (figures.stats.clone(), figures.integral);
        self.visible_figures = Some(figures);
        out
    }

    /// The figures over the history readings `range`: `cached` while it
    /// covers the same readings, else computed afresh.
    fn figures_over(
        &self,
        range: std::ops::Range<usize>,
        cached: Option<RangeFigures>,
    ) -> RangeFigures {
        let key = self.readings_key(range.clone());
        cached
            .filter(|f| f.key == key)
            .unwrap_or_else(|| RangeFigures {
                key,
                stats: self.stats_over(range.clone()),
                integral: self.integral_over(range),
            })
    }

    /// The plot's horizontal lines, carried across the histogram in the
    /// order and styles `MainFrame::draw_over_trace` draws them: the mean,
    /// the reference lines, the alarm limits, then the readings at the
    /// cursors.
    fn histogram_lines(
        &self,
        hist: &histogram::Histogram,
        tc: &ThemeColors,
        limits: Option<WatchedLimits>,
    ) -> Vec<histogram::Line> {
        use egui_plot::LineStyle;
        let line = |value, color, width, style| histogram::Line {
            value,
            stroke: egui::Stroke::new(width, color),
            style,
        };
        let mean = self.show_mean.then(|| hist.mean()).flatten();
        let refs = self
            .ref_lines
            .values()
            .iter()
            .filter(|_| self.show_ref_line);
        let cursors = self.cursors.placed().filter_map(|t| self.nearest_point(t));
        mean.map(|m| line(m, tc.graph_mean(), 1.0, LineStyle::dashed_loose()))
            .into_iter()
            .chain(refs.map(|&v| line(v, tc.graph_ref(), 1.0, LineStyle::dashed_dense())))
            .chain(
                self.limit_lines(limits)
                    .into_iter()
                    .map(|(v, _)| line(v, tc.status_error(), 2.0, LineStyle::dotted_loose())),
            )
            .chain(
                cursors
                    .map(|(_, v)| line(v, tc.graph_cursor_dim(), 1.0, LineStyle::dashed_dense())),
            )
            .collect()
    }

    /// Over-range readings among the history readings `range`: from its
    /// first reading to the one after its last, or to now after the newest.
    pub(super) fn overloads_in(&self, range: std::ops::Range<usize>) -> u32 {
        let before = |i: usize| {
            self.history
                .get(i)
                .map_or(self.breaks.overloads, |p| p.overloads_before)
        };
        let (start, end) = (before(range.start), before(range.end));
        u32::try_from(end.saturating_sub(start)).unwrap_or(u32::MAX)
    }

    /// [`Graph::overloads_in`] the readings in view, plus the over-range
    /// stretch the trace opened on when its first reading is in view: the
    /// band drawn before it. Not for the cursors: one snapped to that
    /// reading leaves the band outside its interval.
    pub(super) fn overloads_in_view(&self, range: std::ops::Range<usize>) -> u32 {
        let opening = match self.lead_in {
            Some(_) if range.start == 0 => self
                .history
                .front()
                .map_or(self.breaks.overloads, |p| p.overloads_before),
            _ => 0,
        };
        self.overloads_in(range)
            .saturating_add(u32::try_from(opening).unwrap_or(u32::MAX))
    }

    /// Bin the history readings `view` for the histogram, unless `key` is
    /// what they were last binned from, with those between the cursors.
    pub(super) fn bin_histogram(
        &mut self,
        l10n: &Localizer,
        key: HistogramKey,
        view: std::ops::Range<usize>,
    ) {
        if self.histogram.as_ref().is_some_and(|b| b.key == key) {
            return;
        }
        let values = self.history.range(view).map(|p| p.value);
        let hist =
            histogram::Histogram::new(values, key.range, key.bins, key.decimals).map(|hist| {
                let mut hist = hist.with_overloads(key.overloads.0);
                if let Some(range) = self.cursor_range() {
                    let overloads = key.overloads.1.unwrap_or(0);
                    hist.count_interval(self.history.range(range).map(|p| p.value), overloads);
                }
                hist
            });
        let spoken = hist
            .as_ref()
            .map(|h| h.spoken(l10n, &self.current_unit))
            .unwrap_or_default();
        self.histogram = Some(BinnedHistogram { key, hist, spoken });
    }

    /// This frame's histogram, while one is drawn.
    pub(super) fn histogram(&self) -> Option<&histogram::Histogram> {
        self.histogram.as_ref()?.hist.as_ref()
    }

    /// Paint this frame's histogram into `area`, in the dock picked for it.
    pub(super) fn paint_histogram(
        &self,
        ui: &Ui,
        l10n: &Localizer,
        area: egui::Rect,
        tc: &ThemeColors,
        limits: Option<WatchedLimits>,
    ) {
        let (Some(hist), Some(place), Some(plot), Some(transform)) = (
            self.histogram(),
            self.histogram_place,
            self.plot_rect,
            self.plot_transform.as_ref(),
        ) else {
            return;
        };
        let lines = self.histogram_lines(hist, tc, limits);
        let pane = histogram::Pane {
            ui,
            l10n,
            hist,
            lines: &lines,
            unit: &self.current_unit,
            colors: histogram::HistogramColors {
                bar: tc.graph_line(),
                cursor: tc.graph_cursor(),
                background: tc.plot_background(),
                text: ui.visuals().weak_text_color(),
            },
        };
        match place.dock {
            histogram::Dock::Side => pane.paint_side(area, plot, transform),
            histogram::Dock::Below => {
                let bounds = transform.bounds();
                pane.paint_below(area, plot.x_range(), (bounds.min()[1], bounds.max()[1]));
            }
        }
    }
}
