//! View state and navigation: which slice of the history the main graph
//! shows, and the keyboard, pan and zoom gestures that move it.

use eframe::egui::{self, Ui};
use egui_plot::PlotTransform;

use super::{Graph, TIME_WINDOWS};

impl Graph {
    /// Handle keyboard shortcuts for graph navigation.
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
            self.live = true;
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
                self.time_window_secs = secs;
            }
        } else if let Some(&(secs, _)) = TIME_WINDOWS
            .iter()
            .find(|&&(s, _)| s > self.time_window_secs + 0.1)
        {
            self.time_window_secs = secs;
        }
    }

    /// Scroll the view by a fraction of the current window width.
    pub(super) fn scroll_view(&mut self, fraction: f64) {
        let delta = self.time_window_secs * fraction;
        let (data_min, data_max) = self.data_time_range();
        let half = self.time_window_secs / 2.0;

        if self.live {
            self.view_center = data_max - half;
            self.live = false;
        }

        self.view_center = (self.view_center + delta).max(data_min + half);

        if self.view_center + half >= data_max {
            self.live = true;
        }
    }

    /// Jump view to the start of recorded data.
    pub(super) fn jump_to_start(&mut self) {
        let (data_min, _) = self.data_time_range();
        self.view_center = data_min + self.time_window_secs / 2.0;
        self.live = false;
    }

    pub(super) fn data_time_range(&self) -> (f64, f64) {
        let x_min = self
            .history
            .front()
            .map(|p| self.elapsed_secs(p.time))
            .unwrap_or(0.0);
        let x_max = self
            .history
            .back()
            .map(|p| self.elapsed_secs(p.time))
            .unwrap_or(0.0);
        // Overload samples carry timestamps but no plottable value, so they
        // never enter `history` — yet they are the newest thing the meter
        // sent. Extending the range here rather than at each call site means
        // every consumer follows them: the live window, the minimap's time
        // mapping and its drag clamping. Otherwise time visibly freezes for
        // the duration of the excursion, which is exactly when the band being
        // drawn needs somewhere to grow into.
        let x_max = match self.pending_break_since {
            Some(t) => x_max.max(self.elapsed_secs(t)),
            None => x_max,
        };
        (x_min, x_max)
    }

    /// Current view bounds (x_min, x_max) for the main graph.
    pub(super) fn view_bounds(&self) -> (f64, f64) {
        let (_, data_max) = self.data_time_range();
        let half = self.time_window_secs / 2.0;

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
            let x_min = (self.view_center - half).max(0.0);
            let x_max = x_min + self.time_window_secs;
            (x_min, x_max)
        }
    }

    /// Compute min/max Y over the visible slice, with 10% padding.
    ///
    /// `with_overlays` includes the sub-value traces drawn beside the plotted
    /// series, so they are framed rather than clipped. The minimap passes
    /// `false`: it is a main-series overview, and its scan already covers the
    /// whole history — multiplying it by the overlay count is the O(n) budget
    /// the two-tier rendering contract exists to protect.
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
            for o in self.shown_overlays().map(|(_, o)| o) {
                for i in start..end {
                    if let Some(v) = o.values.get(i).copied().flatten() {
                        y_min = y_min.min(v);
                        y_max = y_max.max(v);
                    }
                }
            }
        }
        if y_min.is_infinite() {
            return None;
        }
        let range = (y_max - y_min).max(1e-6);
        let pad = range * 0.1;
        Some((y_min - pad, y_max + pad))
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
        if self.y_axis_fixed {
            return Some((self.y_fixed_min, self.y_fixed_max));
        }
        self.y_min_max_padded(x_min, x_max, with_overlays)
    }

    /// Return to live follow with auto Y. Shared by the double-click handler
    /// and the explicit "Reset Zoom" toolbar button.
    pub fn reset_view(&mut self) {
        self.live = true;
        self.view_center = 0.0;
        self.y_axis_fixed = false;
        self.y_user_set = false;
    }

    /// True when the view has been zoomed or panned away from the default
    /// live + auto-Y state. Used to enable/disable the Reset Zoom button.
    pub fn is_view_zoomed(&self) -> bool {
        !self.live || self.y_axis_fixed
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
    pub(super) fn apply_pan(&mut self, time_delta: f64) {
        if self.live {
            let (_, data_max) = self.data_time_range();
            self.view_center = data_max - self.time_window_secs / 2.0;
            self.live = false;
        }
        self.view_center -= time_delta;

        if time_delta < 0.0 {
            let (_, data_max) = self.data_time_range();
            let half = self.time_window_secs / 2.0;
            if self.view_center + half >= data_max {
                self.view_center = data_max - half;
                self.live = true;
            }
        }
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
        self.time_window_secs = time_window.max(MIN_TIME_WINDOW_SECS);
        self.y_axis_fixed = true;
        self.y_fixed_min = y_min;
        self.y_fixed_max = y_max;
        self.y_min_text = format!("{y_min:.4}");
        self.y_max_text = format!("{y_max:.4}");
        self.y_user_set = true;
    }

    /// Process drag, scroll, zoom, and cursor-click interactions on the plot.
    pub(super) fn handle_interaction(
        &mut self,
        ui: &Ui,
        plot_response: &egui::Response,
        transform: &PlotTransform,
        can_interact: bool,
    ) {
        // Bounding-box zoom (Shift + left-drag). Runs before the pan/scroll
        // branches so it can claim the gesture and short-circuit them.
        let shift_held = ui.input(|i| i.modifiers.shift);
        let (primary_pressed, primary_down) =
            ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if self.bbox_zoom_start_px.is_some() && escape_pressed {
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
        }

        // Start: Shift held and primary just went down over the plot. We
        // intentionally do NOT gate on can_interact here — shift-drag should
        // also work from live mode, and we drop out of live on release.
        if shift_held
            && primary_pressed
            && self.bbox_zoom_start_px.is_none()
            && let Some(pos) = plot_response.hover_pos()
        {
            self.bbox_zoom_start_px = Some(pos);
            self.bbox_zoom_current_px = Some(pos);
        }

        // Track the pointer each frame while the drag is live. Fall back to
        // the global interact_pos when the cursor leaves the plot hover area.
        if self.bbox_zoom_start_px.is_some()
            && let Some(pos) = plot_response
                .hover_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        {
            self.bbox_zoom_current_px = Some(pos);
        }

        // Finish: primary released while a bbox-zoom drag was active.
        if self.bbox_zoom_start_px.is_some() && !primary_down {
            if let (Some(start), Some(end)) = (self.bbox_zoom_start_px, self.bbox_zoom_current_px) {
                // Clamp to the plot rect so dragging outside the axes still
                // produces a zoom bounded by what's visible.
                let plot_rect = plot_response.rect;
                let start_c = plot_rect.clamp(start);
                let end_c = plot_rect.clamp(end);
                let rect_px = egui::Rect::from_two_pos(start_c, end_c);
                const MIN_DRAG_PX: f32 = 5.0;
                if rect_px.width() >= MIN_DRAG_PX && rect_px.height() >= MIN_DRAG_PX {
                    let p0 = transform.value_from_position(rect_px.left_top());
                    let p1 = transform.value_from_position(rect_px.right_bottom());
                    self.apply_bbox_zoom((p0.x, p0.y), (p1.x, p1.y));
                }
            }
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
            return;
        }

        // Draw the rubber-band rectangle while the drag is in progress.
        if let (Some(start), Some(current)) = (self.bbox_zoom_start_px, self.bbox_zoom_current_px) {
            let rect = egui::Rect::from_two_pos(start, current);
            let visuals = ui.visuals();
            let fill = visuals.selection.bg_fill.linear_multiply(0.25);
            let stroke = egui::Stroke::new(1.0_f32, visuals.selection.stroke.color);
            ui.painter().rect_filled(rect, 0.0, fill);
            ui.painter()
                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            return;
        }

        // Handle drag: convert pixel delta to time delta. Works in live mode
        // too — apply_pan() snaps view_center to the current end of data and
        // drops out of live on the first drag frame so the pan has effect.
        if plot_response.dragged() {
            let drag_px = plot_response.drag_delta().x;
            let left = transform.value_from_position(plot_response.rect.left_top());
            let right = transform.value_from_position(plot_response.rect.right_top());
            let px_per_sec = plot_response.rect.width() as f64 / (right.x - left.x).max(1e-6);
            let time_delta = drag_px as f64 / px_per_sec;
            self.apply_pan(time_delta);
        }

        // Handle scroll wheel zoom on X axis — zoom centered on cursor position
        if can_interact {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                // Find cursor X position in time coordinates for centered zoom
                if let Some(hover_pos) = plot_response.hover_pos() {
                    let cursor_t = transform.value_from_position(hover_pos).x;
                    let old_half = self.time_window_secs / 2.0;
                    self.time_window_secs = (self.time_window_secs * factor).clamp(2.0, 3600.0);
                    let new_half = self.time_window_secs / 2.0;
                    // Adjust center so cursor stays at same relative position
                    let rel = (cursor_t - (self.view_center - old_half)) / (old_half * 2.0);
                    self.view_center = cursor_t - (rel - 0.5) * new_half * 2.0;
                } else {
                    self.time_window_secs = (self.time_window_secs * factor).clamp(2.0, 3600.0);
                }
            }
        }

        // Scroll while in live mode → exit live mode to browse
        if self.live {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let (_, data_max) = self.data_time_range();
                self.view_center = data_max - self.time_window_secs / 2.0;
                self.live = false;
            }
        }

        // Double-click to return to live mode + auto Y
        if plot_response.double_clicked() {
            self.reset_view();
        }

        // Cursor placement on click — snap to nearest data point
        if self.cursors_active
            && plot_response.clicked()
            && let Some(pos) = plot_response.interact_pointer_pos()
        {
            let click_t = transform.value_from_position(pos).x;
            if let Some((snapped_t, _)) = self.nearest_point(click_t) {
                if self.cursor_next_is_b {
                    self.cursor_b = Some(snapped_t);
                } else {
                    self.cursor_a = Some(snapped_t);
                }
                self.cursor_next_is_b = !self.cursor_next_is_b;
            }
        }
    }
}
