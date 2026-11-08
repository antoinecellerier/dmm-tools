//! View state and navigation: which slice of the history the main graph
//! shows, and the keyboard, pan and zoom gestures that move it. The key
//! handler also carries the plain keys behind the toolbar's analysis chips,
//! so that the graph takes its keyboard input in one place.

use eframe::egui::{self, Ui};
use egui_plot::PlotTransform;
use std::collections::BTreeMap;

use super::field::NumberField;
use super::{Graph, TIME_WINDOWS};

/// How far a scroll travels, in points, before its direction is decided.
const SWIPE_DECIDE_POINTS: f32 = 10.0;
/// A pause this long, in seconds, ends one scroll gesture.
const SWIPE_GAP_SECS: f64 = 0.15;

/// Whether the scroll in progress is a sideways swipe, decided once for the
/// whole gesture. Leaving live sticks, so a stray sideways frame in a page
/// scroll mustn't count, and egui's wheel smoothing flushes the two axes at
/// different rates, so one frame's x and y say little about the gesture.
#[derive(Default)]
pub(super) struct SwipeLatch {
    /// What the gesture has moved while it is undecided.
    travelled: egui::Vec2,
    /// Sideways or not, once decided.
    sideways: Option<bool>,
    /// When the last scroll arrived, in egui's input time.
    last: f64,
}

impl SwipeLatch {
    /// Take a frame's scroll delta, returning the sideways points to apply.
    /// The whole travel comes out on the frame that decides, so the first
    /// few points of a swipe aren't lost.
    fn feed(&mut self, delta: egui::Vec2, now: f64) -> Option<f32> {
        if delta == egui::Vec2::ZERO {
            return None;
        }
        // Not `is_scrolling()`: without an End phase an idle app runs no
        // frame to clear it, and the next event simply carries it on.
        if now - self.last > SWIPE_GAP_SECS {
            self.travelled = egui::Vec2::ZERO;
            self.sideways = None;
        }
        self.last = now;
        match self.sideways {
            // A frame with no sideways part isn't a pan: `apply_pan` leaves
            // live on any call.
            Some(true) => (delta.x != 0.0).then_some(delta.x),
            Some(false) => None,
            None => {
                self.travelled += delta;
                if self.travelled.length() < SWIPE_DECIDE_POINTS {
                    return None;
                }
                let sideways = self.travelled.x.abs() > 2.0 * self.travelled.y.abs();
                self.sideways = Some(sideways);
                sideways.then_some(self.travelled.x)
            }
        }
    }
}

/// The Y axes' fixed ranges, used while **Y:Fixed** is on.
pub(super) struct YAxis {
    /// The axes keep fixed ranges instead of fitting the data.
    pub(super) fixed: bool,
    /// The plotted unit's fixed bounds, as typed and as parsed.
    pub(super) min: NumberField,
    pub(super) max: NumberField,
    /// Whether the user has set the bounds this session.
    pub(super) user_set: bool,
    /// Each right axis's range while `fixed`, by unit, as its axis has to
    /// frame it: kept, like the fields, when Y:Auto is picked, and dropped
    /// by [`YAxis::release`]. A unit with none fits its data.
    pub(super) pins: BTreeMap<String, (f64, f64)>,
}

impl Default for YAxis {
    fn default() -> Self {
        Self {
            fixed: false,
            min: NumberField::new("-1", -1.0),
            max: NumberField::new("1", 1.0),
            user_set: false,
            pins: BTreeMap::new(),
        }
    }
}

impl YAxis {
    /// Back to auto Y on every axis, forgetting the fixed ranges: the scale
    /// they were chosen for is gone, or the user asked for the default view.
    pub(super) fn release(&mut self) {
        self.fixed = false;
        self.user_set = false;
        self.pins.clear();
    }
}

/// A Shift+drag box zoom in progress, in screen pixels.
#[derive(Debug, Clone, Copy)]
pub(super) struct BoxZoom {
    /// Where the drag was pressed.
    start: egui::Pos2,
    /// The pointer's latest position, kept so the release frame still has an
    /// endpoint even when hover_pos()/interact_pos() momentarily return None.
    current: egui::Pos2,
}

/// The 10% breathing room the auto Y range leaves around the data.
///
/// Shared with the minimap, which pads the extremes its bucket level already
/// carries instead of scanning the history for them — the strip's scale has to
/// come out bit-identical either way.
pub(super) fn pad_range(y_min: f64, y_max: f64) -> (f64, f64) {
    let range = (y_max - y_min).max(1e-6);
    let pad = range * 0.1;
    (y_min - pad, y_max + pad)
}

impl Graph {
    /// Handle the graph's keyboard shortcuts: the view keys, and the plain
    /// keys behind the toolbar's analysis chips.
    ///
    /// Minimap pan keys are NOT handled here — they live in `show_minimap`
    /// instead, gated on `pointer_response.has_focus()`. Doing the focus
    /// check + `move_focus(FocusDirection::None)` reset *after*
    /// `interested_in_focus` has already run on the pointer response is
    /// what lets Tab / Shift+Tab still escape the minimap. If we did the
    /// reset here at the top of the frame, it would also wipe egui's
    /// `Focus::begin_pass` snapshot of Tab into `focus_direction = Next`
    /// (egui treats Tab and arrow keys with the same field), trapping
    /// keyboard focus on the minimap.
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};

        // Only when no widget has focus (preserves the existing behaviour
        // that Tab-navigating to a button doesn't steal arrow/Home/End
        // keys from the graph's pan/jump bindings).
        if ctx.egui_wants_keyboard_input() {
            return;
        }

        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::OpenBracket)) {
            self.cycle_time_window(-1);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::CloseBracket)) {
            self.cycle_time_window(1);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft)) {
            self.scroll_view(-0.25);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight)) {
            self.scroll_view(0.25);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Home)) {
            self.jump_to_start();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::End)) {
            self.set_live(true);
        }

        // The analysis overlays. Mean, Min/Max and Histogram are plain flags; the Ref
        // and Cursors chips do more than flip one, so key and click share
        // the one method. The toolbar is drawn later in the same frame,
        // where the chips pick the new state up.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::M)) {
            self.show_mean = !self.show_mean;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::X)) {
            self.show_envelope = !self.show_envelope;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::H)) {
            self.show_histogram = !self.show_histogram;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::R)) {
            self.toggle_ref_lines();
        }
        // Consumed whether or not it does anything, like the app's own keys:
        // the Triggers chip exists only while the reference lines do, and
        // the key is inert exactly where the chip is absent.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::T)) && self.show_ref_line {
            self.show_crossings = !self.show_crossings;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::C)) {
            self.cursors.toggle();
        }
    }

    /// Cycle through TIME_WINDOWS presets. `direction`: -1 = shorter, +1 = longer.
    pub(super) fn cycle_time_window(&mut self, direction: i32) {
        if direction < 0 {
            if let Some(&(secs, _)) = TIME_WINDOWS
                .iter()
                .rev()
                .find(|&&(s, _)| s < self.time_window_secs - 0.1)
            {
                self.set_time_window(secs);
            }
        } else if let Some(&(secs, _)) = TIME_WINDOWS
            .iter()
            .find(|&&(s, _)| s > self.time_window_secs + 0.1)
        {
            self.set_time_window(secs);
        }
    }

    /// Pick a window width, the one already set included: the view shows
    /// it, not the width turning LIVE off froze.
    pub(super) fn set_time_window(&mut self, secs: f64) {
        self.time_window_secs = secs;
        self.frozen_width = None;
    }

    /// The width the view shows when not live: the window's, or the
    /// narrower one LIVE was turned off at (see `frozen_width`).
    pub(super) fn browse_width(&self) -> f64 {
        self.frozen_width.unwrap_or(self.time_window_secs)
    }

    /// Follow the newest data, or stop. Leaving live keeps the width live
    /// showed (see `frozen_width`); entering it is done with that width, and
    /// the next freeze takes its own.
    pub(super) fn set_live(&mut self, live: bool) {
        if live {
            self.frozen_width = None;
        } else if self.live {
            let (x_min, x_max) = self.view_bounds();
            let width = x_max - x_min;
            self.frozen_width = (width > 0.0 && width < self.time_window_secs).then_some(width);
        }
        self.live = live;
    }

    /// Scroll the view by a fraction of the width it shows.
    pub(super) fn scroll_view(&mut self, fraction: f64) {
        if self.live {
            self.leave_live();
        }
        self.center_view_on(self.view_center + self.browse_width() * fraction);
    }

    /// Centre the view on `center` seconds, kept from starting before the
    /// data; a view that reaches the newest data is live again.
    pub(super) fn center_view_on(&mut self, center: f64) {
        let (data_min, data_max) = self.data_time_range();
        let half = self.browse_width() / 2.0;
        self.view_center = center.max(data_min + half);
        self.set_live(self.view_center + half >= data_max);
    }

    /// Show every reading the graph holds at once, not live: a file
    /// imported whole is looked at whole.
    pub(crate) fn show_all(&mut self) {
        let (data_min, data_max) = self.data_time_range();
        self.time_window_secs = (data_max - data_min).max(1.0);
        self.view_center = data_min + self.time_window_secs / 2.0;
        self.frozen_width = None;
        self.live = false;
    }

    /// Jump view to the start of recorded data.
    pub(super) fn jump_to_start(&mut self) {
        let (data_min, _) = self.data_time_range();
        self.set_live(false);
        self.view_center = data_min + self.browse_width() / 2.0;
    }

    pub(super) fn data_time_range(&self) -> (f64, f64) {
        // Sub-value points count as data: a held meter sending only a
        // sub-value has to keep the live window moving and the minimap's
        // time mapping reaching it.
        let x_min = self
            .first_point_time()
            .map(|t| self.elapsed_secs(t))
            .unwrap_or(0.0);
        let x_max = self
            .last_point_time()
            .map(|t| self.elapsed_secs(t))
            .unwrap_or(0.0);
        // Overload samples, and words shown instead of a reading, carry
        // timestamps but no plottable value, so they never enter `history` —
        // yet they are the newest thing the meter sent. Extending the range here rather than at each call site means
        // every consumer follows them: the live window, the minimap's time
        // mapping and its drag clamping. Otherwise time visibly freezes for
        // the duration of the excursion, which is exactly when the band being
        // drawn needs somewhere to grow into.
        let x_max = match self.breaks.heard_until {
            Some(t) => x_max.max(self.elapsed_secs(t)),
            None => x_max,
        };
        (x_min, x_max)
    }

    /// Current view bounds (x_min, x_max) for the main graph.
    pub(super) fn view_bounds(&self) -> (f64, f64) {
        let (_, data_max) = self.data_time_range();

        if self.live {
            // "Live" means the window ends at the newest *sample*, and
            // `data_time_range` counts overload samples as such — so the
            // window keeps advancing through an excursion rather than
            // freezing at the last plotted point. A paused or disconnected
            // meter sends nothing at all, so its window still holds still.
            let x_max = data_max;
            let x_min = (x_max - self.time_window_secs).max(0.0);
            (x_min, x_max)
        } else {
            let width = self.browse_width();
            let x_min = (self.view_center - width / 2.0).max(0.0);
            (x_min, x_min + width)
        }
    }

    /// Compute min/max Y over the visible slice, with 10% padding.
    ///
    /// `with_overlays` includes the sub-value traces drawn beside the plotted
    /// series, so they are framed rather than clipped. The minimap doesn't
    /// come through here at all: it is a main-series overview and reads its
    /// extremes off its bucket level, which is what keeps the whole-history
    /// tier off the per-frame scan budget.
    pub(super) fn y_min_max_padded(
        &self,
        x_min: f64,
        x_max: f64,
        with_overlays: bool,
    ) -> Option<(f64, f64)> {
        let (start, end) = self.visible_index_range(x_min, x_max);
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for i in start..end {
            let v = self.history[i].value;
            y_min = y_min.min(v);
            y_max = y_max.max(v);
        }
        if with_overlays {
            // Hidden overlays are excluded: an axis stretched to frame a trace
            // the user switched off would flatten the one they are looking at.
            // So are those in another unit: they have axes of their own.
            for o in self
                .shown_overlays()
                .map(|(_, o)| o)
                .filter(|o| o.unit == self.current_unit)
            {
                let (start, end) = self.time_index_range(&o.points, |p| p.time, x_min, x_max);
                for v in o.points.range(start..end).filter_map(|p| p.value) {
                    y_min = y_min.min(v);
                    y_max = y_max.max(v);
                }
            }
        }
        if y_min.is_infinite() {
            return None;
        }
        // A level's axis starts at none and reaches at least one, so that a
        // steady 0 still draws two whole-number ticks rather than a sliver.
        if self.levels {
            y_min = y_min.min(0.0);
            y_max = y_max.max(1.0);
        }
        Some(pad_range(y_min, y_max))
    }

    /// Each unit drawn on a right axis, with the range its axis has to
    /// frame: its pin while the axes are fixed, else its data's
    /// ([`Graph::fitted_targets`]).
    pub(super) fn secondary_targets(&self, x_min: f64, x_max: f64) -> Vec<(String, (f64, f64))> {
        let mut targets = self.fitted_targets(&self.drawn_units(), x_min, x_max);
        if self.y_axis.fixed {
            for (unit, range) in &mut targets {
                if let Some(pin) = self.y_axis.pins.get(unit) {
                    *range = *pin;
                }
            }
        }
        targets
    }

    /// Each of `units` but the plotted one, with its shown traces' min/max
    /// over `[x_min, x_max]`: what its right axis has to frame. A unit with
    /// nothing in view gets no axis.
    pub(super) fn fitted_targets(
        &self,
        units: &[&str],
        x_min: f64,
        x_max: f64,
    ) -> Vec<(String, (f64, f64))> {
        let mut targets: Vec<(String, (f64, f64))> = Vec::new();
        for o in &self.overlays {
            if o.unit == self.current_unit || !units.contains(&o.unit.as_str()) || self.is_hidden(o)
            {
                continue;
            }
            let (start, end) = self.time_index_range(&o.points, |p| p.time, x_min, x_max);
            let (lo, hi) = o
                .points
                .range(start..end)
                .filter_map(|p| p.value)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                });
            if lo > hi {
                continue;
            }
            match targets.iter_mut().find(|(unit, _)| *unit == o.unit) {
                Some((_, range)) => *range = (range.0.min(lo), range.1.max(hi)),
                None => targets.push((o.unit.clone(), (lo, hi))),
            }
        }
        for (_, range) in &mut targets {
            // Not padded: the plotted unit's padding already gives the plot
            // its margin, and the fitter widens a target to a round step;
            // padding it too would often push it a step coarser and flatten
            // the trace. A steady value still needs a span to fit.
            *range = super::axes::widen_steady(range.0, range.1);
        }
        targets
    }

    /// Auto-scaled Y range (ignoring fixed mode setting). Used to snapshot
    /// current auto range when switching to fixed mode.
    pub(super) fn y_range_for_view_auto(
        &self,
        x_min: f64,
        x_max: f64,
        with_overlays: bool,
    ) -> Option<(f64, f64)> {
        self.y_min_max_padded(x_min, x_max, with_overlays)
    }

    /// Compute Y range from data points visible in the given X range, with padding.
    pub(super) fn y_range_for_view(
        &self,
        x_min: f64,
        x_max: f64,
        with_overlays: bool,
    ) -> Option<(f64, f64)> {
        if self.y_axis.fixed {
            return Some((self.y_axis.min.value(), self.y_axis.max.value()));
        }
        self.y_min_max_padded(x_min, x_max, with_overlays)
    }

    /// Return to live follow with auto Y. Shared by the double-click handler
    /// and the explicit "Reset Zoom" toolbar button.
    pub fn reset_view(&mut self) {
        self.set_live(true);
        self.view_center = 0.0;
        self.y_axis.release();
    }

    /// Fix every axis at the range it shows now, as the **Y:Fixed** chip
    /// does: the plotted unit's fields unless the user has typed or restored
    /// bounds, and each right axis without a pin yet — all of them afresh
    /// with the fields. Each takes its target rather than the round range its
    /// axis widened it to, so fitting it again gives the same axes instead of
    /// growing them at each toggle. Axes shed for width are fixed too, so
    /// they come back where they were.
    pub(super) fn pin_y_axes_to_view(&mut self) {
        let (view_min, view_max) = self.view_bounds();
        if !self.y_axis.user_set {
            // With the overlays included, so pinning the axis doesn't jump
            // the view the moment Y:Fixed is pressed.
            if let Some((y_lo, y_hi)) = self.y_range_for_view_auto(view_min, view_max, true) {
                self.y_axis.min.set(y_lo);
                self.y_axis.max.set(y_hi);
            }
            self.y_axis.pins.clear();
        }
        let targets = self.fitted_targets(&self.axis_units(), view_min, view_max);
        for (unit, target) in targets {
            self.y_axis.pins.entry(unit).or_insert(target);
        }
    }

    /// True when the view has been zoomed or panned away from the default
    /// live + auto-Y state. Used to enable/disable the Reset Zoom button.
    pub fn is_view_zoomed(&self) -> bool {
        !self.live || self.y_axis.fixed
    }

    /// Freeze the view exactly where live has it, then drop out of live.
    /// The bounds equal the live bounds — a session shorter than the window
    /// keeps the narrower width live shows it at — and the window setting
    /// is left as it was.
    pub(super) fn leave_live(&mut self) {
        let (_, x_max) = self.view_bounds();
        self.set_live(false);
        self.view_center = x_max - self.browse_width() / 2.0;
    }

    /// Shift the view by `time_delta` seconds. The sign convention matches
    /// egui's `drag_delta().x`: positive = mouse moved right, which in the
    /// pan model reveals older data (view_center decreases).
    ///
    /// If currently live, first snaps view_center to the end of data and
    /// drops out of live so drag-to-pan works from live mode without a
    /// visible jump (the snapped bounds equal the live bounds on that frame).
    ///
    /// If the drag is moving toward newer data (mouse left → `time_delta < 0`)
    /// and would push the view's right edge to or past the latest sample,
    /// snap back to live instead of letting the view drift into empty
    /// future-space.
    ///
    /// The other way it stops at the oldest sample: past it the view would
    /// show nothing new, and the way back would first have to undo the excess.
    pub(super) fn apply_pan(&mut self, time_delta: f64) {
        if self.live {
            self.leave_live();
        }
        let (data_min, data_max) = self.data_time_range();
        let half = self.browse_width() / 2.0;
        self.view_center = (self.view_center - time_delta).max(data_min + half);

        if time_delta < 0.0 && self.view_center + half >= data_max {
            self.view_center = data_max - half;
            self.set_live(true);
        }
    }

    /// Zoom the time axis by `zoom_delta` (above 1 narrows, as wheel up and
    /// a pinch open read), keeping `cursor_t` where it is on screen, or the
    /// middle without one. It zooms what the plot shows, which live and a
    /// frozen view keep narrower than the window while the session is
    /// shorter than it, and drops out of live first as a pan does. The
    /// width it picks is the window's from then on.
    pub(super) fn zoom_time(&mut self, zoom_delta: f64, cursor_t: Option<f64>) {
        if self.live {
            self.leave_live();
        }
        let (shown_min, shown_max) = self.view_bounds();
        let shown = if shown_max > shown_min {
            shown_max - shown_min
        } else {
            self.time_window_secs
        };
        let cursor_t = cursor_t.unwrap_or(shown_min + shown / 2.0);
        let rel = (cursor_t - shown_min) / shown;
        self.time_window_secs = (shown / zoom_delta).clamp(2.0, 3600.0);
        self.frozen_width = None;
        self.view_center = cursor_t - (rel - 0.5) * self.time_window_secs;
    }

    /// The sideways points of a touchpad swipe over a surface of the graph
    /// this frame, `over` saying the pointer is on that surface and it is
    /// free to move. The pointer is on one surface at most, so a frame's
    /// scroll is counted once.
    ///
    /// None while every reading is on screen: there is nowhere to pan, and a
    /// light brush of the touchpad would leave live with nothing moving and
    /// the next readings falling outside the frozen view.
    pub(super) fn swipe_x(&mut self, ui: &Ui, over: bool) -> Option<f32> {
        let (data_min, data_max) = self.data_time_range();
        let (x_min, x_max) = self.view_bounds();
        if !over || (x_min <= data_min && x_max >= data_max) {
            return None;
        }
        let (delta, now) = ui.input(|i| (i.smooth_scroll_delta(), i.time));
        self.swipe.feed(delta, now)
    }

    /// Pure helper: given the two corners of a bbox-zoom rectangle in data
    /// coordinates, return the (view_center, time_window, y_min, y_max) that
    /// zooms the view to that region. Handles reversed drags by normalising
    /// min/max on each axis.
    pub(super) fn bbox_to_view(p0: (f64, f64), p1: (f64, f64)) -> (f64, f64, f64, f64) {
        let x_min = p0.0.min(p1.0);
        let x_max = p0.0.max(p1.0);
        let y_min = p0.1.min(p1.1);
        let y_max = p0.1.max(p1.1);
        let view_center = (x_min + x_max) * 0.5;
        let time_window = x_max - x_min;
        (view_center, time_window, y_min, y_max)
    }

    /// Apply a bbox zoom to the current view state. Clamps time window to a
    /// sane minimum and mirrors the zoomed Y range into the toolbar text
    /// buffers so the numbers stay in sync.
    pub(super) fn apply_bbox_zoom(&mut self, p0: (f64, f64), p1: (f64, f64)) {
        const MIN_TIME_WINDOW_SECS: f64 = 0.1;
        let (view_center, time_window, y_min, y_max) = Self::bbox_to_view(p0, p1);
        self.live = false;
        self.view_center = view_center;
        self.frozen_width = None;
        self.time_window_secs = time_window.max(MIN_TIME_WINDOW_SECS);
        self.y_axis.fixed = true;
        self.y_axis.min.set(y_min);
        self.y_axis.max.set(y_max);
        self.y_axis.user_set = true;
        // Each right axis to its share of the box, on this frame's map.
        self.y_axis.pins = self
            .axis_maps
            .iter()
            .map(|(unit, map)| {
                let (a, b) = (map.value_at(y_min), map.value_at(y_max));
                (unit.clone(), (a.min(b), a.max(b)))
            })
            .collect();
    }

    /// Process drag, zoom, and cursor-click interactions on the plot.
    pub(super) fn handle_interaction(
        &mut self,
        ui: &Ui,
        plot_response: &egui::Response,
        transform: &PlotTransform,
    ) {
        // Bounding-box zoom (Shift + left-drag). Runs before the pan and zoom
        // branches so it can claim the gesture and short-circuit them.
        let shift_held = ui.input(|i| i.modifiers.shift);
        let (primary_pressed, primary_down) =
            ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed {
            self.box_zoom = None;
        }

        // Start: Shift held and primary just went down over the plot. We
        // intentionally do NOT gate on live mode here — shift-drag should
        // also work from live mode, and we drop out of live on release.
        if shift_held
            && primary_pressed
            && self.box_zoom.is_none()
            && let Some(pos) = plot_response.hover_pos()
        {
            self.box_zoom = Some(BoxZoom {
                start: pos,
                current: pos,
            });
        }

        // Track the pointer each frame while the drag is live. Fall back to
        // the global interact_pos when the cursor leaves the plot hover area.
        if let Some(zoom) = &mut self.box_zoom
            && let Some(pos) = plot_response
                .hover_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        {
            zoom.current = pos;
        }

        // Finish: primary released while a bbox-zoom drag was active.
        if let Some(zoom) = self.box_zoom
            && !primary_down
        {
            // Clamp to the plot rect so dragging outside the axes still
            // produces a zoom bounded by what's visible.
            let plot_rect = plot_response.rect;
            let start_c = plot_rect.clamp(zoom.start);
            let end_c = plot_rect.clamp(zoom.current);
            let rect_px = egui::Rect::from_two_pos(start_c, end_c);
            const MIN_DRAG_PX: f32 = 5.0;
            if rect_px.width() >= MIN_DRAG_PX && rect_px.height() >= MIN_DRAG_PX {
                let p0 = transform.value_from_position(rect_px.left_top());
                let p1 = transform.value_from_position(rect_px.right_bottom());
                self.apply_bbox_zoom((p0.x, p0.y), (p1.x, p1.y));
            }
            self.box_zoom = None;
            return;
        }

        // Draw the rubber-band rectangle while the drag is in progress.
        if let Some(zoom) = self.box_zoom {
            let rect = egui::Rect::from_two_pos(zoom.start, zoom.current);
            let visuals = ui.visuals();
            let fill = visuals.selection.bg_fill.linear_multiply(0.25);
            let stroke = egui::Stroke::new(1.0_f32, crate::a11y::focus_ring_color(visuals));
            ui.painter().rect_filled(rect, 0.0, fill);
            ui.painter()
                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            return;
        }

        // Handle drag: convert pixel delta to time delta. Works in live mode
        // too — apply_pan() snaps view_center to the current end of data and
        // drops out of live on the first drag frame so the pan has effect.
        let secs_of = |px: f32| {
            let left = transform.value_from_position(plot_response.rect.left_top());
            let right = transform.value_from_position(plot_response.rect.right_top());
            let px_per_sec = plot_response.rect.width() as f64 / (right.x - left.x).max(1e-6);
            px as f64 / px_per_sec
        };
        if plot_response.dragged() {
            self.apply_pan(secs_of(plot_response.drag_delta().x));
        }

        // A two-finger swipe sideways on a touchpad pans as a drag does. It
        // comes as horizontal scroll, which the graph's scroll areas, being
        // vertical, leave alone; the vertical part still scrolls the page.
        let swipe_free = plot_response.contains_pointer() && !plot_response.dragged();
        if let Some(swipe_px) = self.swipe_x(ui, swipe_free) {
            self.apply_pan(secs_of(swipe_px));
        }

        // X-axis zoom on Ctrl + wheel — and on a touch pinch, which egui folds
        // into the same signal. Both are kept out of `smooth_scroll_delta`, so
        // a plain wheel tick belongs to whatever scroll area the graph panel
        // sits in and never reaches this handler.
        //
        // The hover test is the other half of that: without it a tick anywhere
        // in the window zoomed the graph, including over the shortcut modal.
        let zoom_delta = ui.input(|i| i.zoom_delta()) as f64;
        if zoom_delta != 1.0 && plot_response.contains_pointer() {
            // Keep the time under the cursor where it is. `contains_pointer`
            // holds in cases `hover_pos` doesn't (another widget above the
            // plot has the hover), so fall back to the raw pointer position
            // the way egui_plot's own zoom does.
            let cursor_t = plot_response
                .hover_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()))
                .map(|pos| transform.value_from_position(pos).x);
            self.zoom_time(zoom_delta, cursor_t);
        }

        // Double-click to return to live mode + auto Y
        if plot_response.double_clicked() {
            self.reset_view();
        }

        // Cursor placement on click — snap to nearest data point. Not the
        // click that closes the plot's menu: that one only dismisses it.
        if self.cursors.active
            && plot_response.clicked()
            && !egui::Popup::is_id_open(ui.ctx(), super::menu::plot_menu_id())
            && let Some(pos) = plot_response.interact_pointer_pos()
        {
            let click_t = transform.value_from_position(pos).x;
            if let Some((snapped_t, _)) = self.nearest_point(click_t) {
                self.cursors.place(snapped_t);
            }
        }
    }
}
