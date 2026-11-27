//! Sizing the big meter: how small the window may become before the reading
//! stops fitting, how much margin the reading is given, and when the layout
//! splits into columns.
//!
//! Pure arithmetic, kept out of the per-frame `ui` body so the boundary
//! conditions `.claude/rules/gui.md` names — a very wide or very narrow
//! window, a quarter of the screen, maximized, high zoom — can be tested
//! without a display.

use eframe::egui;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

use super::BigMeterMode;
use super::connection_issue::NoticeKind;
use crate::display::{self, ReadingRatios};

/// Initial estimate for non-reading content height in big meter mode.
const DEFAULT_METER_CONTENT_HEIGHT: f32 = 200.0;

/// Re-measure passes before the fit is accepted as it stands. A button row
/// that wraps differently at two neighbouring scales never converges, so the
/// loop has to be capped rather than run to agreement.
pub(super) const MAX_RECALC_PASSES: u8 = 4;

/// How far apart two frames' scales may be and count as one size: a grid
/// that far out moves a measure by a twentieth of what a fit settles within.
const SAME_SCALE: f32 = 0.001;

/// Width at which the reading moves into its own column beside the graph.
const WIDE_LAYOUT_MIN_WIDTH: f32 = 900.0;

/// Window size below which the big meter starts giving up its panel margin,
/// so the reading fills a small window tighter.
const MARGIN_FULL_AT: f32 = 300.0;

/// What the window has to leave room for besides the reading.
#[derive(Clone, Copy)]
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
    /// Key of the inputs being re-measured, whose passes `recalc_passes`
    /// counts.
    pass_key: u64,
    /// Number of recalculation passes for `pass_key`.
    recalc_passes: u8,
    /// The scale the frame before drew the reading at.
    drawn_scale: f32,
    /// The other inputs a notice's line was last drawn under a reading
    /// with, by their key: the room it took is kept while they stand.
    line_drawn_with: Option<u64>,
}

impl MeterFit {
    pub(super) fn new() -> Self {
        Self {
            content_height: DEFAULT_METER_CONTENT_HEIGHT,
            reading_ratios: ReadingRatios::default(),
            cache_key: 0,
            pass_key: 0,
            recalc_passes: 0,
            drawn_scale: 0.0,
            line_drawn_with: None,
        }
    }

    /// Settle whether the fit holds room for a notice's line under the
    /// reading, in `inputs.notice_line`: while one is drawn there, and from
    /// then on until another input moves.
    ///
    /// A notice comes and goes: a meter that misses two reads and answers
    /// the third, a reconnect, a gap in a recording. Fitted with the line
    /// and again without it, the reading shrank and grew back each time.
    /// The room stays instead, so the reading changes size once, and a
    /// resize or anything else that fits it afresh gives the room back.
    /// The fit's key and the frame both read the answer from `inputs`.
    pub(super) fn keep_line_room(&mut self, inputs: &mut FitInputs, drawn: bool) {
        inputs.notice_line = false;
        let others = inputs.key();
        if drawn {
            self.line_drawn_with = Some(others);
        } else if self.line_drawn_with != Some(others) {
            self.line_drawn_with = None;
        }
        inputs.notice_line = self.line_drawn_with.is_some();
    }

    /// Whether the fit has to be re-measured for these inputs.
    pub(super) fn needs_recalc(&self, inputs: &FitInputs) -> bool {
        inputs.key() != self.cache_key
    }

    /// Fold one re-measured pass into the cache.
    ///
    /// The content below the reading and the reading's own ratios are
    /// measured at the scale they were drawn at, which the measurement then
    /// changes, so it takes more passes to settle. It has settled when a
    /// pass measures what it was drawn with: the content height alone is not
    /// enough, as a pass drawn with other ratios can match it by chance, and
    /// the next frame's larger font then wraps the buttons off the bottom.
    /// Once settled, or at the cap, the larger of each wins, so everything
    /// still fits.
    ///
    /// The cap counts passes for one set of inputs: a resize in the middle
    /// of a fit starts a new count rather than inheriting the old one's.
    ///
    /// Returns whether another frame is owed: the fit is still open, or it
    /// closed on more than this frame was drawn with. A pass settles within
    /// a tolerance, and a mode line one character longer is inside it: the
    /// fit closes at once, raised, and the frame on screen is still the one
    /// drawn before the raise.
    pub(super) fn record_pass(
        &mut self,
        inputs: &FitInputs,
        measured_content_height: f32,
        measured_ratios: ReadingRatios,
    ) -> bool {
        let key = inputs.key();
        if key != self.pass_key {
            self.pass_key = key;
            self.recalc_passes = 0;
        }
        let settled = (self.content_height - measured_content_height).abs() < 1.0
            && self.reading_ratios.settled(&measured_ratios);
        if settled || self.recalc_passes >= MAX_RECALC_PASSES {
            let raised = measured_content_height > self.content_height
                || self.reading_ratios.raised_by(&measured_ratios);
            self.content_height = self.content_height.max(measured_content_height);
            self.reading_ratios = self.reading_ratios.max(&measured_ratios);
            self.cache_key = key;
            self.recalc_passes = 0;
            raised
        } else {
            self.content_height = measured_content_height;
            self.reading_ratios = measured_ratios;
            self.recalc_passes += 1;
            true
        }
    }

    /// Note the scale a frame drew the reading at; whether the frame before
    /// drew it at the same one.
    ///
    /// The sub-values are an egui `Grid`, which draws at the column widths
    /// and row heights of the frame before. On a frame that changes the
    /// reading's size the grid is still the old size, and the frame is no
    /// measure of what the new size takes: one that follows a larger size
    /// would [`grow`](Self::grow) the fit for a grid that is not there the
    /// frame after, and the reading would stay smaller than fits.
    pub(super) fn drawn_as_before(&mut self, scale: f32) -> bool {
        let before = std::mem::replace(&mut self.drawn_scale, scale);
        (before - scale).abs() <= SAME_SCALE * scale
    }

    /// Raise a closed fit to what a frame drew, where that outgrew it.
    /// Returns whether it did, for the caller to ask for the frame that
    /// draws the reading at the size the raised fit gives.
    ///
    /// The key names what is known to move the reading's size. What it
    /// leaves out still reaches the screen — a unit gaining a character as
    /// the meter changes range — and the reading it widens or the rows it
    /// wraps would hang out of the window until the next resize.
    ///
    /// Grow-only: it never lowers a value or re-opens the fit. A row that
    /// wraps at one scale and not at the next would then be shrunk and
    /// grown for ever, the loop `MAX_RECALC_PASSES` ends. So a reading
    /// shrunk here stays smaller once its cause has gone, until a resize or
    /// a mode change fits it afresh, and the window's minimum size rises
    /// with the fit. A frame within the tolerance a pass settles by is no
    /// growth, so raising the fit to a measurement leaves the same frame
    /// nothing to raise.
    pub(super) fn grow(
        &mut self,
        measured_content_height: f32,
        measured_ratios: &ReadingRatios,
    ) -> bool {
        let outgrown = measured_content_height - self.content_height >= 1.0
            || self.reading_ratios.outgrown_by(measured_ratios);
        if outgrown {
            self.content_height = self.content_height.max(measured_content_height);
            self.reading_ratios = self.reading_ratios.max(measured_ratios);
        }
        outgrown
    }

    /// Smallest window that still fits the reading, derived from the cached
    /// ratios at the minimum big-meter font size.
    ///
    /// `bar_min_w` is the narrowest the top bar lets the window be
    /// (`top_bar::min_width`).
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
    /// The range the mode line shows. A meter stepping from 22 V to 220 V
    /// keeps its mode word and its badges, and the character the line gains
    /// is less than a frame must outgrow a closed fit by to raise it
    /// ([`MeterFit::grow`]): a line as wide as the window would stay a
    /// character past its edge.
    pub(super) range_label: Cow<'static, str>,
    /// Sub-value rows change the reading's height without changing the mode
    /// word (a UT181A entering MIN/MAX), so the fitted font has to be
    /// re-measured.
    pub(super) aux_values: usize,
    /// The meter's badges on the mode line (HOLD, REL): one more wraps the
    /// line or lengthens the row, so the reading is a little smaller while
    /// it shows and comes back when it goes.
    pub(super) badges: display::ShownBadges,
    /// The mode and range selectors are framed controls, a little taller
    /// than the plain labels they replace.
    pub(super) mode_offered: bool,
    pub(super) range_offered: bool,
    pub(super) show_stats: bool,
    pub(super) show_specs: bool,
    /// The meter-only view's spec line leaves out the fields turned off.
    pub(super) spec_fields: crate::settings::SpecFields,
    pub(super) big_meter_mode: BigMeterMode,
    /// An active scale adds the `Raw` sub-value, which changes how much room
    /// is left for the reading. (The Scale and Alarm rows never show here.)
    pub(super) transform_is_identity: bool,
    /// A set alarm reserves the limit badge's slot by the reading and adds
    /// the Alarms line to the statistics.
    pub(super) alarm_set: bool,
    /// Which connection notice, if any, is on screen, but for one that is a
    /// line under the big meter's reading (`notice_line`). In the big-meter
    /// modes its title takes the readout's place, and the titles differ in
    /// length; with the mode `Off` its whole help is drawn under the
    /// buttons. Either way the reading is fitted again when one arrives,
    /// goes or replaces another.
    pub(super) notice_kind: Option<NoticeKind>,
    /// Whether room is held under the big meter's reading for a notice's
    /// line: [`MeterFit::keep_line_room`] sets it, whatever it is built
    /// with. Whichever notice the line names, the fit counts one line that
    /// scales with the reading: a title too wide for its room is drawn
    /// smaller, and one at the smallest font wraps instead
    /// (`NoReadingText::show_under_reading`), which the fit doesn't see.
    pub(super) notice_line: bool,
    /// The late-readings notice, the capture ask and the screen-only line,
    /// drawn under the buttons with the mode `Off`: each takes room from the
    /// reading while it shows.
    pub(super) late_readings_note: bool,
    pub(super) capture_ask_note: bool,
    pub(super) screen_only_note: bool,
    /// The language the badges, the placeholder and a notice's hint are
    /// written in, whose words differ in length.
    pub(super) language: &'static str,
    /// The Statistics columns' widths: one that widens can change the
    /// table's shape, and with it the height left for the reading.
    pub(super) stats_widths: [usize; 4],
    /// The control height, in hundredths of a point: Touch's taller buttons
    /// take room from the reading at the same window size.
    pub(super) control_height: u32,
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

/// The grow request to send this frame, if any: on the frame the window
/// first falls short of its minimum, and again only when the size it should
/// grow to or the window's own size moves. A window manager that won't
/// resize a tiled window would otherwise get the request — and the app a
/// repaint — every frame. Cleared once the window fits, so the next shrink
/// asks again.
pub(super) fn grow_request(
    screen: egui::Vec2,
    min_size: egui::Vec2,
    last: &mut Option<(egui::Vec2, egui::Vec2)>,
) -> Option<egui::Vec2> {
    let Some(target) = grow_to_fit(screen, min_size) else {
        *last = None;
        return None;
    };
    if last.is_some_and(|(s, t)| !size_moved(s, screen) && !size_moved(t, target)) {
        return None;
    }
    *last = Some((screen, target));
    Some(target)
}

/// The minimum window size to send this frame, if any: on the first frame,
/// and again only when the size or the pixels per point move. egui-winit
/// turns the size into physical pixels at the scale it is sent at, so a zoom
/// or a move to a monitor with another scale needs it sent again.
pub(super) fn min_size_request(
    min_size: egui::Vec2,
    pixels_per_point: f32,
    last: &mut Option<(egui::Vec2, f32)>,
) -> Option<egui::Vec2> {
    if last.is_some_and(|(s, ppp)| !size_moved(s, min_size) && ppp == pixels_per_point) {
        return None;
    }
    *last = Some((min_size, pixels_per_point));
    Some(min_size)
}

/// Half a pixel of tolerance: sub-pixel jitter in the cached top-bar widths
/// is not a change.
pub(super) fn size_moved(a: egui::Vec2, b: egui::Vec2) -> bool {
    (a - b).abs().max_elem() > 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> FitInputs {
        FitInputs {
            width: 800,
            height: 600,
            mode_raw: 0,
            range_label: "22V".into(),
            aux_values: 0,
            badges: display::ShownBadges::default(),
            mode_offered: false,
            range_offered: false,
            show_stats: false,
            show_specs: false,
            spec_fields: crate::settings::SpecFields::default(),
            big_meter_mode: BigMeterMode::Off,
            transform_is_identity: true,
            alarm_set: false,
            notice_kind: None,
            notice_line: false,
            late_readings_note: false,
            capture_ask_note: false,
            screen_only_note: false,
            language: "en-US",
            stats_widths: [7, 2, 3, 4],
            control_height: 1800,
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

    /// A window a tiling window manager holds below its minimum is asked to
    /// grow once, not every frame; a new target, a new window size or a
    /// fresh shrink asks again.
    #[test]
    fn a_window_below_its_minimum_is_asked_to_grow_once() {
        let min = egui::vec2(400.0, 300.0);
        let small = egui::vec2(300.0, 200.0);
        let mut last = None;
        assert_eq!(grow_request(small, min, &mut last), Some(min), "too small");
        assert_eq!(grow_request(small, min, &mut last), None, "next frame");
        assert_eq!(
            grow_request(small, min + egui::vec2(0.3, 0.0), &mut last),
            None,
            "sub-pixel jitter in the minimum"
        );
        let bigger = min + egui::vec2(20.0, 0.0);
        assert_eq!(
            grow_request(small, bigger, &mut last),
            Some(bigger),
            "the minimum grew (leaving minimal mode)"
        );
        assert_eq!(
            grow_request(egui::vec2(250.0, 180.0), bigger, &mut last),
            Some(bigger),
            "the window moved, still short on both axes"
        );
        assert_eq!(
            grow_request(egui::vec2(800.0, 600.0), bigger, &mut last),
            None,
            "fits"
        );
        assert_eq!(last, None, "fitting clears the last request");
        assert_eq!(
            grow_request(small, min, &mut last),
            Some(min),
            "too small again at the first size"
        );
    }

    /// The minimum window size goes out once, then again when it moves or
    /// when the scale it was turned into pixels at changes.
    #[test]
    fn the_minimum_size_is_resent_when_the_scale_changes() {
        let min = egui::vec2(400.0, 300.0);
        let mut last = None;
        assert_eq!(
            min_size_request(min, 1.66, &mut last),
            Some(min),
            "first frame"
        );
        assert_eq!(min_size_request(min, 1.66, &mut last), None, "next frame");
        assert_eq!(
            min_size_request(min + egui::vec2(0.3, 0.0), 1.66, &mut last),
            None,
            "sub-pixel jitter in the minimum"
        );
        assert_eq!(
            min_size_request(min, 1.0, &mut last),
            Some(min),
            "dragged to a 100 % monitor"
        );
        let bigger = min + egui::vec2(20.0, 0.0);
        assert_eq!(
            min_size_request(bigger, 1.0, &mut last),
            Some(bigger),
            "the minimum grew"
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

    /// The connection issue's title stands in for the reading in the
    /// big-meter modes, and the titles differ in length — so a notice
    /// arriving, going, or being replaced has to re-open the fit even though
    /// nothing else about the window moved.
    #[test]
    fn a_new_notice_reopens_the_fit() {
        let mut fit = MeterFit::new();
        let quiet = inputs();
        fit.record_pass(&quiet, fit.content_height, ReadingRatios::default());
        assert!(!fit.needs_recalc(&quiet));

        let failed = FitInputs {
            notice_kind: Some(NoticeKind::NoResponse),
            ..inputs()
        };
        assert!(
            fit.needs_recalc(&failed),
            "the notice never reached the fit"
        );
        fit.record_pass(&failed, fit.content_height, ReadingRatios::default());
        assert!(
            fit.needs_recalc(&FitInputs {
                notice_kind: Some(NoticeKind::NoMeterFound),
                ..inputs()
            }),
            "one notice replacing another never reached the fit"
        );
    }

    /// A line under the reading opens the fit when it first shows, and its
    /// room is kept from then on: the line going, coming back and going
    /// again leave the key alone. Another input moving gives the room back,
    /// unless the line is on screen then.
    #[test]
    fn a_line_under_the_reading_keeps_its_room_until_another_input_moves() {
        let mut fit = MeterFit::new();
        let key = |fit: &mut MeterFit, mut inputs: FitInputs, drawn: bool| {
            fit.keep_line_room(&mut inputs, drawn);
            (inputs.notice_line, inputs.key())
        };
        let (room, quiet) = key(&mut fit, inputs(), false);
        assert!(!room, "room before any line");

        let (room, with_line) = key(&mut fit, inputs(), true);
        assert!(room);
        assert_ne!(with_line, quiet, "the line never reached the fit");
        for drawn in [false, true, false] {
            assert_eq!(
                key(&mut fit, inputs(), drawn),
                (true, with_line),
                "the line {} moved the fit",
                if drawn { "coming back" } else { "going" }
            );
        }

        let resized = || FitInputs {
            width: 801,
            ..inputs()
        };
        assert!(key(&mut fit, resized(), true).0, "a resize under the line");
        assert!(!key(&mut fit, inputs(), false).0, "kept through a resize");
        assert!(!key(&mut fit, resized(), false).0, "and back at that size");
    }

    /// A pass that closes the fit on more than its frame was drawn with
    /// owes the frame that draws it; one that closes on what it was drawn
    /// with owes none, and an open fit always owes its next pass.
    #[test]
    fn a_pass_that_closes_raised_owes_another_frame() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        assert!(fit.record_pass(&inputs, 40.0, ratios(5.0)), "still open");
        assert!(
            !fit.record_pass(&inputs, 40.0, ratios(5.0)),
            "closed as drawn"
        );

        let ranged = FitInputs {
            range_label: "220V".into(),
            ..self::inputs()
        };
        // One character more on the line: within what a pass settles by.
        assert!(fit.record_pass(&ranged, 40.0, ratios(5.05)));
        assert!(!fit.needs_recalc(&ranged), "it settled");
        assert_eq!(fit.reading_ratios.w, 5.05);
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

    /// The badges of a reading on HOLD.
    fn held() -> display::ShownBadges {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::{MeasuredValue, Measurement};
        let flags = StatusFlags {
            hold: true,
            ..StatusFlags::default()
        };
        display::shown_badges(&Measurement::test_fixture(
            MeasuredValue::Normal(1.0),
            "V",
            flags,
        ))
    }

    /// A badge the meter lights after the fit has closed re-opens it, and
    /// so does the badge going.
    #[test]
    fn a_badge_arriving_or_going_reopens_the_fit() {
        let mut fit = MeterFit::new();
        let plain = inputs();
        fit.record_pass(&plain, fit.content_height, ReadingRatios::default());
        let on_hold = FitInputs {
            badges: held(),
            ..inputs()
        };
        assert!(fit.needs_recalc(&on_hold), "HOLD never reached the fit");
        fit.record_pass(&on_hold, fit.content_height, ReadingRatios::default());
        assert!(fit.needs_recalc(&plain), "HOLD going never reached the fit");
    }

    /// A closed fit whose frame drew more than it assumed takes the larger
    /// of each measure and stays closed; the same frame again changes
    /// nothing, so the repaint it asked for is the last.
    #[test]
    fn a_frame_that_outgrows_the_fit_grows_it() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        fit.record_pass(&inputs, 40.0, ratios(5.0));
        fit.record_pass(&inputs, 40.0, ratios(5.0));
        assert!(!fit.needs_recalc(&inputs));

        // Within what a pass settles by: text snapping to pixels, not growth.
        assert!(!fit.grow(40.5, &ratios(5.05)));
        assert_eq!((fit.content_height, fit.reading_ratios.w), (40.0, 5.0));

        // A wrapped button row, then a unit one character longer; the
        // measure that did not grow keeps the larger value it had.
        assert!(fit.grow(72.0, &ratios(5.0)));
        assert!(fit.grow(41.0, &ratios(5.6)));
        assert_eq!((fit.content_height, fit.reading_ratios.w), (72.0, 5.6));
        assert!(!fit.needs_recalc(&inputs), "growing re-opened the fit");
        assert!(
            !fit.grow(72.0, &ratios(5.6)),
            "the same frame grew it twice"
        );
    }

    /// Growing is one-way. A fit that stopped at the pass cap holds the
    /// larger of two measures that never agreed; a frame measuring the
    /// smaller one must not lower it or re-open it, or the two would trade
    /// places for ever.
    #[test]
    fn a_smaller_frame_never_reopens_a_capped_fit() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        for i in 0..=MAX_RECALC_PASSES {
            let (height, w) = if i % 2 == 0 {
                (100.0, 5.0)
            } else {
                (400.0, 7.0)
            };
            fit.record_pass(&inputs, height, ratios(w));
        }
        assert!(!fit.needs_recalc(&inputs));
        assert_eq!((fit.content_height, fit.reading_ratios.w), (400.0, 7.0));

        assert!(!fit.grow(100.0, &ratios(5.0)));
        assert!(
            !fit.needs_recalc(&inputs),
            "a smaller frame re-opened the fit"
        );
        assert_eq!((fit.content_height, fit.reading_ratios.w), (400.0, 7.0));
    }

    /// A frame that draws the reading at another size than the one before
    /// is told apart from one that repeats it, so the caller can keep a
    /// grid still laid out at the old size out of the fit; the float noise
    /// of one size worked out twice is the same size.
    #[test]
    fn a_frame_at_a_new_size_is_told_from_one_that_repeats_it() {
        let mut fit = MeterFit::new();
        assert!(!fit.drawn_as_before(2.0), "the first frame follows none");
        assert!(fit.drawn_as_before(2.0));
        assert!(fit.drawn_as_before(2.0001));
        assert!(!fit.drawn_as_before(1.9), "a twentieth smaller");
        assert!(fit.drawn_as_before(1.9), "and then drawn again");
    }

    /// Ratios as a pass measured them, `w` wide: the reading's width per
    /// point of font.
    fn ratios(w: f32) -> ReadingRatios {
        ReadingRatios {
            w,
            ..ReadingRatios::default()
        }
    }

    /// A pass drawn with other ratios can measure the content height the
    /// cache holds by chance; the next frame, drawn with the new ratios,
    /// then wraps the buttons off the bottom. The fit stays open until the
    /// ratios settle too.
    #[test]
    fn a_pass_with_moving_ratios_keeps_the_fit_open() {
        let mut fit = MeterFit::new();
        let inputs = inputs();
        fit.record_pass(&inputs, 16.0, ratios(8.0));
        fit.record_pass(&inputs, 16.5, ratios(5.8));
        assert!(fit.needs_recalc(&inputs), "the ratios moved by a quarter");
        fit.record_pass(&inputs, 33.0, ratios(5.7));
        fit.record_pass(&inputs, 32.6, ratios(5.69));
        assert!(!fit.needs_recalc(&inputs));
        assert_eq!(fit.content_height, 33.0);
        assert_eq!(fit.reading_ratios.w, 5.7, "the larger ratio wins");
    }

    /// A resize in the middle of a fit starts a new count: carrying the old
    /// one over closed the fit at the new size after a single pass.
    #[test]
    fn a_resize_mid_fit_gets_its_own_passes() {
        let mut fit = MeterFit::new();
        let before = inputs();
        for i in 0..MAX_RECALC_PASSES {
            let alternating = if i % 2 == 0 { 100.0 } else { 400.0 };
            fit.record_pass(&before, alternating, ReadingRatios::default());
        }
        let resized = FitInputs {
            width: 450,
            height: 320,
            ..inputs()
        };
        fit.record_pass(&resized, 30.0, ReadingRatios::default());
        assert!(fit.needs_recalc(&resized), "closed on the old size's count");
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
            range_label: "220V".into(),
            ..inputs()
        });
        mutations.push(FitInputs {
            aux_values: 1,
            ..inputs()
        });
        mutations.push(FitInputs {
            badges: held(),
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
            spec_fields: crate::settings::SpecFields {
                accuracy: false,
                ..Default::default()
            },
            ..inputs()
        });
        mutations.push(FitInputs {
            big_meter_mode: BigMeterMode::Minimal,
            ..inputs()
        });
        mutations.push(FitInputs {
            transform_is_identity: false,
            ..inputs()
        });
        mutations.push(FitInputs {
            alarm_set: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            notice_kind: Some(NoticeKind::NoResponse),
            ..inputs()
        });
        mutations.push(FitInputs {
            notice_line: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            late_readings_note: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            capture_ask_note: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            screen_only_note: true,
            ..inputs()
        });
        mutations.push(FitInputs {
            language: "fr",
            ..inputs()
        });
        mutations.push(FitInputs {
            stats_widths: [8, 2, 3, 4],
            ..inputs()
        });
        mutations.push(FitInputs {
            control_height: 2800,
            ..inputs()
        });
        for (i, m) in mutations.iter().enumerate() {
            assert_ne!(m.key(), base, "input {i} does not reach the cache key");
        }
    }
}
