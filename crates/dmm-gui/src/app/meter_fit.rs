//! Sizing the big meter: how small the window may become before the reading
//! stops fitting, how much margin the reading is given, and when the layout
//! splits into columns.
//!
//! Pure arithmetic, kept out of the per-frame `ui` body so the boundary
//! conditions `.claude/rules/gui.md` names — a very wide or very narrow
//! window, a quarter of the screen, maximized, high zoom — can be tested
//! without a display.

use eframe::egui;
use std::hash::{Hash, Hasher};

use super::BigMeterMode;
use crate::display::{self, ReadingRatios};

/// Initial estimate for non-reading content height in big meter mode.
const DEFAULT_METER_CONTENT_HEIGHT: f32 = 200.0;

/// Re-measure passes before the fit is accepted as it stands. A button row
/// that wraps differently at two neighbouring scales never converges, so the
/// loop has to be capped rather than run to agreement.
const MAX_RECALC_PASSES: u8 = 4;

/// Width at which the reading moves into its own column beside the graph.
const WIDE_LAYOUT_MIN_WIDTH: f32 = 900.0;

/// Window size below which the big meter starts giving up its panel margin,
/// so the reading fills a small window tighter.
const MARGIN_FULL_AT: f32 = 300.0;

/// What the window has to leave room for besides the reading.
pub(super) enum WindowContent {
    /// Just the reading — no top bar, no buttons.
    ReadingOnly,
    /// Reading + buttons + top bar.
    Meter,
    /// Full layout: the top bar constrains width, the panels need height.
    Panels,
}

/// Cached inputs of the big-meter fit solver, which sizes the reading to the
/// window and re-measures only when one of its inputs changes.
pub(super) struct MeterFit {
    /// Cached height of non-reading content at scale=1 for big meter mode.
    pub(super) content_height: f32,
    /// Cached reading dimension ratios for big meter mode.
    pub(super) reading_ratios: ReadingRatios,
    /// Key of the inputs the cache was built from.
    cache_key: u64,
    /// Number of recalculation passes since the last cache key change.
    recalc_passes: u8,
}

impl MeterFit {
    pub(super) fn new() -> Self {
        Self {
            content_height: DEFAULT_METER_CONTENT_HEIGHT,
            reading_ratios: ReadingRatios::default(),
            cache_key: 0,
            recalc_passes: 0,
        }
    }

    /// Whether the fit has to be re-measured for these inputs.
    pub(super) fn needs_recalc(&self, inputs: &FitInputs) -> bool {
        inputs.key() != self.cache_key
    }

    /// Fold one re-measured pass into the cache.
    ///
    /// The content below the reading is measured at the scale it was drawn
    /// at, which the measurement itself then changes, so it takes a second
    /// pass to settle. Once it does — or the cap is reached — the larger of
    /// the two heights wins, so everything still fits.
    pub(super) fn record_pass(
        &mut self,
        inputs: &FitInputs,
        measured_content_height: f32,
        measured_ratios: ReadingRatios,
    ) {
        if (self.content_height - measured_content_height).abs() < 1.0
            || self.recalc_passes >= MAX_RECALC_PASSES
        {
            self.content_height = self.content_height.max(measured_content_height);
            self.cache_key = inputs.key();
            self.recalc_passes = 0;
        } else {
            self.content_height = measured_content_height;
            self.recalc_passes += 1;
        }
        self.reading_ratios = measured_ratios;
    }

    /// Smallest window that still fits the reading, derived from the cached
    /// ratios at the minimum big-meter font size.
    ///
    /// `bar_min_w` is the wider of the top bar's two groups as measured on
    /// the previous frame, plus its spacing.
    pub(super) fn min_window_size(&self, content: WindowContent, bar_min_w: f32) -> egui::Vec2 {
        let min_font = display::MIN_BIG_METER_FONT_SIZE;
        let min_scale = min_font / display::BASE_READING_FONT_SIZE;
        let reading_w = self.reading_ratios.w * min_font;
        let reading_h = self.reading_ratios.h * min_font + self.content_height * min_scale;
        match content {
            WindowContent::ReadingOnly => egui::vec2(reading_w, reading_h),
            WindowContent::Meter => egui::vec2(reading_w.max(bar_min_w), reading_h),
            WindowContent::Panels => egui::vec2(bar_min_w, reading_h),
        }
    }
}

/// Everything that changes how much room the reading gets. When any of it
/// moves, the fit is re-measured.
#[derive(Hash)]
pub(super) struct FitInputs {
    /// Window size, in whole logical pixels — sub-pixel drift is not a
    /// reason to re-measure.
    pub(super) width: u32,
    pub(super) height: u32,
    /// The mode word, which the reading is laid out around.
    pub(super) mode_raw: u16,
    /// Sub-value rows change the reading's height without changing the mode
    /// word (a UT181A entering MIN/MAX), so the fitted font has to be
    /// re-measured.
    pub(super) aux_values: usize,
    /// The mode and range selectors are framed controls, a little taller
    /// than the plain labels they replace.
    pub(super) mode_offered: bool,
    pub(super) range_offered: bool,
    pub(super) show_stats: bool,
    pub(super) show_specs: bool,
    pub(super) big_meter_mode: BigMeterMode,
    /// The Scale row adds a button row, and opening its editor adds a second
    /// one — both change how much room is left for the reading.
    pub(super) transform_editor_open: bool,
    pub(super) transform_is_identity: bool,
}

impl FitInputs {
    fn key(&self) -> u64 {
        let mut h = std::hash::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

/// Whether the window is wide enough for the reading to sit in its own
/// column beside the graph.
pub(super) fn is_wide(content_width: f32) -> bool {
    content_width >= WIDE_LAYOUT_MIN_WIDTH
}

/// Fraction of the panel's normal margin the big meter keeps.
pub(super) fn margin_scale(screen: egui::Vec2) -> f32 {
    (screen.x.min(screen.y) / MARGIN_FULL_AT).clamp(0.1, 1.0)
}

/// The size to grow the window to when it no longer meets its own minimum —
/// leaving minimal mode, say — or `None` when it already fits.
pub(super) fn grow_to_fit(screen: egui::Vec2, min_size: egui::Vec2) -> Option<egui::Vec2> {
    (screen.x < min_size.x || screen.y < min_size.y)
        .then(|| egui::vec2(screen.x.max(min_size.x), screen.y.max(min_size.y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> FitInputs {
        FitInputs {
            width: 800,
            height: 600,
            mode_raw: 0,
            aux_values: 0,
            mode_offered: false,
            range_offered: false,
            show_stats: false,
            show_specs: false,
            big_meter_mode: BigMeterMode::Off,
            transform_editor_open: false,
            transform_is_identity: true,
        }
    }

    /// Minimal mode has no top bar, so the bar's width must not hold the
    /// window open — that is the whole point of shrinking to a tiny widget.
    #[test]
    fn minimal_mode_ignores_the_top_bar_width() {
        let fit = MeterFit::new();
        let size = fit.min_window_size(WindowContent::ReadingOnly, 4000.0);
        assert!(size.x < 200.0, "{size:?} was held open by the top bar");
        assert_eq!(size, fit.min_window_size(WindowContent::ReadingOnly, 0.0));
    }

    /// With the top bar on screen the window has to fit whichever of the two
    /// is wider — the reading on a narrow bar, the bar on a narrow reading.
    #[test]
    fn the_meter_window_fits_the_wider_of_reading_and_top_bar() {
        let fit = MeterFit::new();
        let wide_bar = fit.min_window_size(WindowContent::Meter, 500.0);
        let narrow_bar = fit.min_window_size(WindowContent::Meter, 10.0);
        assert_eq!(wide_bar.x, 500.0);
        assert_eq!(
            narrow_bar.x,
            fit.reading_ratios.w * display::MIN_BIG_METER_FONT_SIZE
        );
        assert_eq!(wide_bar.y, narrow_bar.y, "height does not follow the bar");
    }

    /// The full layout's own panels set the width; only the reading's height
    /// still has to be reserved.
    #[test]
    fn the_full_layout_takes_its_width_from_the_top_bar() {
        let fit = MeterFit::new();
        let size = fit.min_window_size(WindowContent::Panels, 316.0);
        assert_eq!(size.x, 316.0);
        assert_eq!(size.y, fit.min_window_size(WindowContent::Meter, 316.0).y);
    }

    /// Taller content below the reading pushes the minimum height up, scaled
    /// down to the minimum font — not added at full size.
    #[test]
    fn content_below_the_reading_raises_the_minimum_height() {
        let mut fit = MeterFit::new();
        let before = fit.min_window_size(WindowContent::Meter, 0.0).y;
        fit.content_height += 300.0;
        let after = fit.min_window_size(WindowContent::Meter, 0.0).y;
        let scale = display::MIN_BIG_METER_FONT_SIZE / display::BASE_READING_FONT_SIZE;
        assert!((after - before - 300.0 * scale).abs() < 1e-3);
    }

    /// A maximized window keeps the full margin; a quarter-screen or
    /// high-zoom one gives some back, and a sliver still keeps a tenth so
    /// the reading never touches the frame.
    #[test]
    fn the_margin_shrinks_with_the_window_and_stops_at_a_tenth() {
        assert_eq!(margin_scale(egui::vec2(3840.0, 2160.0)), 1.0, "maximized");
        assert_eq!(margin_scale(egui::vec2(1920.0, 300.0)), 1.0, "at the knee");
        // A quarter of a 1280x800 screen, and the same window at 200% zoom.
        assert_eq!(margin_scale(egui::vec2(640.0, 400.0)), 1.0);
        assert_eq!(margin_scale(egui::vec2(320.0, 200.0)), 200.0 / 300.0);
        assert_eq!(margin_scale(egui::vec2(2000.0, 20.0)), 0.1, "a sliver");
    }

    /// The reading only gets its own column past the threshold; a very
    /// narrow window keeps the single column however tall it is.
    #[test]
    fn the_layout_splits_into_columns_only_past_the_threshold() {
        assert!(is_wide(3840.0), "very wide");
        assert!(is_wide(900.0), "exactly at the threshold");
        assert!(!is_wide(899.0));
        assert!(!is_wide(320.0), "very narrow");
    }

    #[test]
    fn a_window_short_on_either_axis_grows_only_that_axis() {
        let min = egui::vec2(400.0, 300.0);
        assert_eq!(grow_to_fit(egui::vec2(800.0, 600.0), min), None);
        assert_eq!(
            grow_to_fit(egui::vec2(400.0, 300.0), min),
            None,
            "exact fit"
        );
        assert_eq!(
            grow_to_fit(egui::vec2(200.0, 600.0), min),
            Some(egui::vec2(400.0, 600.0))
        );
        assert_eq!(
            grow_to_fit(egui::vec2(200.0, 100.0), min),
            Some(egui::vec2(400.0, 300.0))
        );
    }

    /// A measurement within a pixel of the cache is the fit settling: adopt
    /// it and stop re-measuring.
    #[test]
    fn a_converged_pass_closes_the_cache() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        assert!(fit.needs_recalc(&inputs));
        fit.record_pass(&inputs, fit.content_height + 0.5, ReadingRatios::default());
        assert!(!fit.needs_recalc(&inputs));
    }

    /// A content height that keeps moving would re-measure forever. After
    /// the cap the largest height seen wins, so nothing is clipped.
    #[test]
    fn an_oscillating_fit_stops_at_the_pass_cap() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        for i in 0..MAX_RECALC_PASSES {
            let alternating = if i % 2 == 0 { 100.0 } else { 400.0 };
            fit.record_pass(&inputs, alternating, ReadingRatios::default());
            assert!(fit.needs_recalc(&inputs), "gave up after {i} passes");
        }
        fit.record_pass(&inputs, 100.0, ReadingRatios::default());
        assert!(!fit.needs_recalc(&inputs));
        assert_eq!(fit.content_height, 400.0, "the larger height has to win");
    }

    /// Every input is one the reading's size depends on, so none may be
    /// hashed away.
    #[test]
    fn each_input_moves_the_key() {
        let base = inputs().key();
        let mut mutations: Vec<FitInputs> = Vec::new();
        mutations.push(FitInputs {
            width: 801,
            ..inputs()
        });
        mutations.push(FitInputs {
            height: 601,
            ..inputs()
        });
        mutations.push(FitInputs {
            mode_raw: 1,
            ..inputs()
        });
        mutations.push(FitInputs {
            aux_values: 1,
            ..inputs()
        });
        mutations.push(FitInputs {
            mode_offered: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            range_offered: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            show_stats: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            show_specs: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            big_meter_mode: BigMeterMode::Minimal,
            ..inputs()
        });
        mutations.push(FitInputs {
            transform_editor_open: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            transform_is_identity: false,
            ..inputs()
        });
        for (i, m) in mutations.iter().enumerate() {
            assert_ne!(m.key(), base, "input {i} does not reach the cache key");
        }
    }
}
