//! The minimap strip below the main graph: the full-history overview and the
//! drag gestures that move and resize the view window over it.

use eframe::egui::{self, Ui};

use super::time::{format_time_label, nice_time_interval};
use super::{GapKind, Graph};
use crate::a11y::ResponseA11yExt;
use crate::theme::ThemeColors;

/// Minimap height in logical pixels.
pub(super) const MINIMAP_HEIGHT: f32 = 60.0;

/// Half-width of a bracket's resize hit zone, in pixels.
const HANDLE_HALF: f32 = 8.0;

/// Drag distance below which a resize is treated as noise rather than intent.
const RESIZE_DEADZONE_PX: f32 = 0.1;

/// How far the view window may be zoomed in and out, in seconds.
const MIN_WINDOW_SECS: f64 = 2.0;
const MAX_WINDOW_SECS: f64 = 3600.0;

/// Tracks which part of the minimap the user is dragging.
#[derive(Default, Clone, Copy, PartialEq)]
pub(super) enum MinimapDrag {
    #[default]
    None,
    Pan,
    ResizeLeft,
    ResizeRight,
}

/// Maps between times in the history and x positions on the minimap strip.
///
/// The strip always shows the whole session, so this is the one mapping the
/// trace, the overload bands, the viewport brackets, the time labels and
/// every drag gesture share.
pub(super) struct MinimapScale {
    left: f32,
    width: f32,
    data_min: f64,
    data_span: f64,
}

impl MinimapScale {
    pub(super) fn new(rect: egui::Rect, data_min: f64, data_max: f64) -> Self {
        Self {
            left: rect.left(),
            width: rect.width(),
            data_min,
            // A session shorter than a microsecond would otherwise divide by
            // zero and put every point at the same x.
            data_span: (data_max - data_min).max(1e-6),
        }
    }

    pub(super) fn x_of(&self, t: f64) -> f32 {
        self.left + ((t - self.data_min) / self.data_span) as f32 * self.width
    }

    /// The time under a pointer, clamped to the strip: a drag that leaves the
    /// minimap keeps pushing the view to the end it left by.
    pub(super) fn time_at(&self, x: f32) -> f64 {
        self.data_min + ((x - self.left) / self.width).clamp(0.0, 1.0) as f64 * self.data_span
    }

    /// Session length, floored so a degenerate span never divides by zero.
    pub(super) fn span(&self) -> f64 {
        self.data_span
    }

    fn time_per_px(&self) -> f64 {
        self.data_span / self.width as f64
    }

    /// Left and right edges of one overload band.
    ///
    /// Widened to one pixel when the span is narrower. Unlike the main plot —
    /// where a floor would overstate the duration against a legible time axis
    /// — the minimap compresses the whole session into a strip, so sub-pixel
    /// is the normal case and the alternative is the band silently not
    /// existing.
    pub(super) fn band_x(&self, start: f64, end: f64) -> (f32, f32) {
        let x0 = self.x_of(start);
        (x0, self.x_of(end).max(x0 + 1.0))
    }
}

/// The slice of the history the main graph shows, as the minimap's drags
/// move it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ViewWindow {
    /// Centre, in seconds from the origin.
    pub(super) center: f64,
    /// Width, in seconds.
    pub(super) width: f64,
    /// Whether the view follows the newest samples.
    pub(super) live: bool,
}

/// Which bracket a resize drag is holding.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Edge {
    Left,
    Right,
}

/// Which part of the minimap a press at `origin_x` grabs.
///
/// Decided on the mouse-down frame, so the hit-test uses the bracket
/// positions from that same frame and stays consistent even if the brackets
/// shift later (live data arriving, resize in progress). When the brackets
/// are close together the nearest edge wins.
pub(super) fn drag_target(origin_x: f32, vp_left: f32, vp_right: f32) -> MinimapDrag {
    let dl = (origin_x - vp_left).abs();
    let dr = (origin_x - vp_right).abs();
    if dl <= HANDLE_HALF && dl <= dr {
        MinimapDrag::ResizeLeft
    } else if dr <= HANDLE_HALF {
        MinimapDrag::ResizeRight
    } else {
        MinimapDrag::Pan
    }
}

/// Whether the pointer is close enough to a bracket for the resize cursor.
pub(super) fn near_a_bracket(x: f32, vp_left: f32, vp_right: f32) -> bool {
    (x - vp_left).abs() <= HANDLE_HALF || (x - vp_right).abs() <= HANDLE_HALF
}

/// The view window after dragging one bracket by `drag_px`.
///
/// A window wider than the data is snapped to the data span first: its real
/// edges are off the strip, so without the snap the first drag frame would
/// jump by the difference instead of tracking the bracket under the pointer.
pub(super) fn resize(
    window: ViewWindow,
    edge: Edge,
    drag_px: f32,
    scale: &MinimapScale,
    data_max: f64,
) -> ViewWindow {
    if drag_px.abs() <= RESIZE_DEADZONE_PX {
        return window;
    }
    let data_min = scale.data_min;
    let data_span = scale.data_span;
    let mut window = window;
    if window.width > data_span + 0.1 {
        window = ViewWindow {
            center: data_min + data_span / 2.0,
            width: data_span,
            live: false,
        };
    }
    let dt = drag_px as f64 * scale.time_per_px();
    match edge {
        // Dragging the left bracket pins the right edge, and vice versa.
        Edge::Left => {
            let right_edge = window.center + window.width / 2.0;
            let width = (window.width - dt).clamp(MIN_WINDOW_SECS, MAX_WINDOW_SECS);
            ViewWindow {
                center: right_edge - width / 2.0,
                width,
                live: false,
            }
        }
        Edge::Right => {
            let left_edge = (window.center - window.width / 2.0).max(0.0);
            let width = (window.width + dt).clamp(MIN_WINDOW_SECS, MAX_WINDOW_SECS);
            let center = left_edge + width / 2.0;
            ViewWindow {
                center,
                width,
                // Widening the right edge past the newest sample is how the
                // user asks to follow the present again.
                live: center + width / 2.0 >= data_max,
            }
        }
    }
}

/// The view window after a pan to `pointer_x`.
pub(super) fn pan(
    window: ViewWindow,
    pointer_x: f32,
    scale: &MinimapScale,
    data_max: f64,
) -> ViewWindow {
    let center = scale.time_at(pointer_x);
    if center + window.width / 2.0 >= data_max {
        // Panning onto the newest samples resumes live follow rather than
        // parking the view just short of the end.
        ViewWindow {
            live: true,
            ..window
        }
    } else {
        ViewWindow {
            center,
            live: false,
            ..window
        }
    }
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
        let scale = MinimapScale::new(rect, data_min, data_max);

        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        // Overload bands, matching the main plot so the two read the same way.
        // Behind the trace, like the background.
        for &(start, end) in &overload_spans {
            let (x0, x1) = scale.band_x(start, end);
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
                    let x = scale.x_of(t);
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
        let vp_left = scale.x_of(view_min);
        let vp_right = scale.x_of(view_max);
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
        let nice_interval = nice_time_interval(scale.span());
        let mut t = (data_min / nice_interval).ceil() * nice_interval;
        while t <= data_max {
            let x = scale.x_of(t);
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

        // Handle click/drag navigation with bracket resize handles.
        //
        // Cursor feedback: force resize icon during active resize drag,
        // otherwise show it on hover near bracket edges.
        if matches!(
            self.minimap_drag,
            MinimapDrag::ResizeLeft | MinimapDrag::ResizeRight
        ) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if let Some(hover_pos) = pointer_response.hover_pos()
            && near_a_bracket(hover_pos.x, vp_left, vp_right)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // Lock in drag mode on the mouse-down frame.
        if self.minimap_drag == MinimapDrag::None
            && pointer_response.is_pointer_button_down_on()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            self.minimap_drag = drag_target(origin.x, vp_left, vp_right);
        }

        // Apply drag — resize uses per-frame delta, pan uses absolute position.
        let window = ViewWindow {
            center: self.view_center,
            width: self.time_window_secs,
            live: self.live,
        };
        let dragged = match self.minimap_drag {
            MinimapDrag::ResizeLeft => Some(resize(
                window,
                Edge::Left,
                pointer_response.drag_delta().x,
                &scale,
                data_max,
            )),
            MinimapDrag::ResizeRight => Some(resize(
                window,
                Edge::Right,
                pointer_response.drag_delta().x,
                &scale,
                data_max,
            )),
            MinimapDrag::Pan => pointer_response
                .interact_pointer_pos()
                .map(|pos| pan(window, pos.x, &scale, data_max)),
            MinimapDrag::None => None,
        };
        if let Some(next) = dragged {
            self.view_center = next.center;
            self.time_window_secs = next.width;
            self.live = next.live;
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
