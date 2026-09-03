//! The minimap strip below the main graph: the full-history overview and the
//! drag gestures that move and resize the view window over it.

use eframe::egui::{self, Ui};

use super::time::{format_time_label, nice_time_interval};
use super::{GapKind, Graph};
use crate::a11y::ResponseA11yExt;
use crate::theme::ThemeColors;

/// Minimap height in logical pixels.
pub(super) const MINIMAP_HEIGHT: f32 = 60.0;

/// Tracks which part of the minimap the user is dragging.
#[derive(Default, Clone, Copy, PartialEq)]
pub(super) enum MinimapDrag {
    #[default]
    None,
    Pan,
    ResizeLeft,
    ResizeRight,
}

impl Graph {
    /// Render the minimap showing full history with viewport indicator.
    pub fn show_minimap(&mut self, ui: &mut Ui, tc: &ThemeColors) {
        if self.history.len() < 2 {
            ui.allocate_space(egui::vec2(ui.available_width(), MINIMAP_HEIGHT));
            return;
        }

        self.ensure_cache();
        let raw_segments = &self.cached_segments;
        let (data_min, data_max) = self.data_time_range();
        let (view_min, view_max) = self.view_bounds();

        let line_color = tc.minimap_line();
        let overload_fill = tc.graph_overload_fill();
        // Same spans the main plot bands, including one still open.
        let overload_spans: Vec<(f64, f64)> = self
            .cached_gaps
            .iter()
            .filter(|(_, _, kind)| *kind == GapKind::Overload)
            .map(|&(a, b, _)| (a, b))
            .chain(self.pending_overload_span())
            .collect();

        // Allocate rect for minimap + label space below, with margin for bracket strokes
        let label_height = 14.0;
        let margin = 4.0; // room for bracket strokes at edges
        let total_height = MINIMAP_HEIGHT + label_height + margin * 2.0;
        let (full_rect, pointer_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), total_height),
            egui::Sense::click_and_drag(),
        );
        let pointer_response = pointer_response
            .on_hover_text(
                "Minimap — click or drag to pan, drag the bracket edges to resize the view",
            )
            .a11y_label(
                "Graph minimap — click or drag to navigate timeline (Left/Right to pan), drag bracket edges to resize",
            );
        // Inset the plot area so brackets at edges have room to render
        let rect = egui::Rect::from_min_size(
            egui::pos2(full_rect.left() + margin, full_rect.top() + margin),
            egui::vec2(full_rect.width() - margin * 2.0, MINIMAP_HEIGHT),
        );

        // Use full_rect painter so nothing gets clipped
        let painter = ui.painter_at(full_rect);
        let data_span = (data_max - data_min).max(1e-6);

        let time_to_x =
            |t: f64| -> f32 { rect.left() + ((t - data_min) / data_span) as f32 * rect.width() };

        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        // Overload bands, matching the main plot so the two read the same way.
        // Behind the trace, like the background.
        //
        // Widened to one pixel when the span is narrower. Unlike the main
        // plot — where a floor would overstate the duration against a legible
        // time axis — the minimap compresses the whole session into a strip,
        // so sub-pixel is the normal case and the alternative is the band
        // silently not existing.
        for &(start, end) in &overload_spans {
            let x0 = time_to_x(start);
            let x1 = time_to_x(end).max(x0 + 1.0);
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom())),
                0.0,
                overload_fill,
            );
        }

        // Draw data lines.
        //
        // The Y range is identical for every point in every segment because it
        // covers the whole history (`data_min..data_max`). Compute it once
        // before the loop — pulling this call inside the per-point closure
        // turned the minimap into an O(n²) hot spot, which dominated frame
        // time once the history filled.
        //
        // Deliberately the *auto* range, not the main plot's: a pinned Y range
        // (a Shift-drag box zoom, or a user-entered fixed range) is chosen to
        // frame a detail of the main view, and scaling full history by it puts
        // most points far outside this 60px strip. They clip to a flat line
        // along the edges and overdraw the time axis below — the overview the
        // minimap exists to give disappears exactly when zooming in makes it
        // most useful.
        let y_map = self
            .y_range_for_view_auto(data_min, data_max, false)
            .map(|(lo, hi)| {
                let range = (hi - lo).max(1e-10);
                (lo, range)
            });
        for seg in raw_segments {
            let points: Vec<egui::Pos2> = seg
                .iter()
                .map(|&[t, v]| {
                    let x = time_to_x(t);
                    let y_frac = match y_map {
                        Some((y_lo, range)) => ((v - y_lo) / range) as f32,
                        None => 0.5,
                    };
                    let y = rect.bottom() - y_frac * rect.height();
                    egui::pos2(x, y)
                })
                .collect();
            for window in points.windows(2) {
                painter.line_segment(
                    [window[0], window[1]],
                    egui::Stroke::new(1.0_f32, line_color),
                );
            }
        }

        // Draw viewport indicator as [ ] bracket markers
        let vp_left = time_to_x(view_min);
        let vp_right = time_to_x(view_max);
        let vp_color = tc.minimap_viewport();
        let vp_stroke = egui::Stroke::new(2.5_f32, vp_color);
        let bracket_w = 4.0_f32; // horizontal arm of the bracket

        // Left bracket [
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.top()),
                egui::pos2(vp_left, rect.bottom()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.top()),
                egui::pos2(vp_left + bracket_w, rect.top()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_left, rect.bottom()),
                egui::pos2(vp_left + bracket_w, rect.bottom()),
            ],
            vp_stroke,
        );

        // Right bracket ]
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.top()),
                egui::pos2(vp_right, rect.bottom()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.top()),
                egui::pos2(vp_right - bracket_w, rect.top()),
            ],
            vp_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(vp_right, rect.bottom()),
                egui::pos2(vp_right - bracket_w, rect.bottom()),
            ],
            vp_stroke,
        );

        // Draw X-axis time labels
        let label_color = ui.visuals().weak_text_color();
        let nice_interval = nice_time_interval(data_span);
        let mut t = (data_min / nice_interval).ceil() * nice_interval;
        while t <= data_max {
            let x = time_to_x(t);
            let label = format_time_label(t);
            painter.text(
                egui::pos2(x, rect.bottom() + 2.0),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::proportional(11.0),
                label_color,
            );
            // Small tick mark
            painter.line_segment(
                [
                    egui::pos2(x, rect.bottom() - 2.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0_f32, label_color),
            );
            t += nice_interval;
        }

        // Handle click/drag navigation with bracket resize handles
        let handle_half = 8.0_f32; // half-width of the resize hit zone in pixels

        // Cursor feedback: force resize icon during active resize drag,
        // otherwise show it on hover near bracket edges.
        if matches!(
            self.minimap_drag,
            MinimapDrag::ResizeLeft | MinimapDrag::ResizeRight
        ) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if let Some(hover_pos) = pointer_response.hover_pos() {
            let dl = (hover_pos.x - vp_left).abs();
            let dr = (hover_pos.x - vp_right).abs();
            if dl <= handle_half || dr <= handle_half {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }

        // Lock in drag mode on the mouse-down frame — bracket positions and click
        // position are from the same frame, so the hit-test is consistent even if
        // brackets shift later (live data arriving, resize in progress).
        if self.minimap_drag == MinimapDrag::None
            && pointer_response.is_pointer_button_down_on()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let dl = (origin.x - vp_left).abs();
            let dr = (origin.x - vp_right).abs();
            // When brackets are close, pick the nearest edge
            if dl <= handle_half && dl <= dr {
                self.minimap_drag = MinimapDrag::ResizeLeft;
            } else if dr <= handle_half {
                self.minimap_drag = MinimapDrag::ResizeRight;
            } else {
                self.minimap_drag = MinimapDrag::Pan;
            }
        }

        // Apply drag — resize uses per-frame delta, pan uses absolute position.
        // When the window is wider than the data, the brackets are clamped to
        // the data edges. On the first resize drag frame we snap the window to
        // the visible (clamped) span so the drag feels 1:1 with what's on screen.
        let time_per_px = data_span / rect.width() as f64;
        match self.minimap_drag {
            MinimapDrag::ResizeLeft => {
                let drag_px = pointer_response.drag_delta().x;
                if drag_px.abs() > 0.1 {
                    // Snap to visible span if window extends before data start
                    if self.time_window_secs > data_span + 0.1 {
                        self.time_window_secs = data_span;
                        self.view_center = data_min + data_span / 2.0;
                        self.live = false;
                    }
                    let dt = drag_px as f64 * time_per_px;
                    let right_edge = self.view_center + self.time_window_secs / 2.0;
                    self.time_window_secs = (self.time_window_secs - dt).clamp(2.0, 3600.0);
                    self.view_center = right_edge - self.time_window_secs / 2.0;
                    self.live = false;
                }
            }
            MinimapDrag::ResizeRight => {
                let drag_px = pointer_response.drag_delta().x;
                if drag_px.abs() > 0.1 {
                    // Snap to visible span if window extends past data end
                    if self.time_window_secs > data_span + 0.1 {
                        self.time_window_secs = data_span;
                        self.view_center = data_min + data_span / 2.0;
                        self.live = false;
                    }
                    let dt = drag_px as f64 * time_per_px;
                    let left_edge = (self.view_center - self.time_window_secs / 2.0).max(0.0);
                    self.time_window_secs = (self.time_window_secs + dt).clamp(2.0, 3600.0);
                    self.view_center = left_edge + self.time_window_secs / 2.0;
                    self.live = self.view_center + self.time_window_secs / 2.0 >= data_max;
                }
            }
            MinimapDrag::Pan => {
                if let Some(pos) = pointer_response.interact_pointer_pos() {
                    let pos_t = data_min
                        + ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * data_span;
                    let half = self.time_window_secs / 2.0;
                    if pos_t + half >= data_max {
                        self.live = true;
                    } else {
                        self.view_center = pos_t;
                        self.live = false;
                    }
                }
            }
            MinimapDrag::None => {}
        }

        // Reset drag state when pointer is released
        if !pointer_response.is_pointer_button_down_on() {
            self.minimap_drag = MinimapDrag::None;
        }

        // Keyboard pan when the minimap holds focus. Has to run *after*
        // `allocate_exact_size` above (which runs `interested_in_focus`
        // on the pointer response) so that on a Tab press focus has
        // already advanced away and `pointer_response.has_focus()`
        // returns false. Otherwise the `move_focus(FocusDirection::None)`
        // below would also wipe egui's `Focus::begin_pass` snapshot of
        // Tab into `focus_direction = Next` (egui stores arrow keys and
        // Tab in the same field), trapping focus on the minimap.
        if pointer_response.has_focus() {
            use egui::{Key, Modifiers};
            let left = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft));
            let right = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight));
            // Up/Down get consumed too even though they don't pan —
            // without that, `end_pass` would walk
            // `find_widget_in_direction` on the begin_pass snapshot and
            // Tab-jump focus to the spatially nearest widget on every
            // Up/Down press.
            let up = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
            let down = ui
                .ctx()
                .input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
            if left {
                self.scroll_view(-0.25);
            }
            if right {
                self.scroll_view(0.25);
            }
            if left || right || up || down {
                ui.ctx()
                    .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }

        // Paint focus ring last so it sits on top of the minimap content
        // (background, brackets, time labels) rather than being overdrawn.
        crate::a11y::paint_focus_ring(ui, &pointer_response);
    }
}
