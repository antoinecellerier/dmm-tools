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

/// One physical pixel column of the trace, reduced to the vertical extent of
/// the samples that landed in it.
#[derive(Clone, Copy)]
struct Column {
    key: f32,
    y_min: f32,
    y_max: f32,
}

impl Column {
    fn new(key: f32, y: f32) -> Self {
        Self {
            key,
            y_min: y,
            y_max: y,
        }
    }

    fn add(&mut self, y: f32) {
        self.y_min = self.y_min.min(y);
        self.y_max = self.y_max.max(y);
    }

    fn mid_y(&self) -> f32 {
        (self.y_min + self.y_max) / 2.0
    }

    /// Whether `y_min` is the extreme closer to `y`.
    fn min_is_nearer(&self, y: f32) -> bool {
        (self.y_min - y).abs() <= (self.y_max - y).abs()
    }

    /// Push the column's extent as one vertical run, leaving on `y_min` when
    /// `exit_min` (the other extreme is where the path enters). Returns the y
    /// the path leaves the column on.
    fn push(&self, exit_min: bool, ppp: f32, out: &mut Vec<egui::Pos2>) -> f32 {
        let x = (self.key + 0.5) / ppp;
        if self.y_min == self.y_max {
            out.push(egui::pos2(x, self.y_min));
            return self.y_min;
        }
        let (enter, exit) = if exit_min {
            (self.y_max, self.y_min)
        } else {
            (self.y_min, self.y_max)
        };
        out.push(egui::pos2(x, enter));
        out.push(egui::pos2(x, exit));
        exit
    }
}

/// Ratio between consecutive minimap bucket widths.
///
/// Buckets are re-cut only when the session grows past a step, and between
/// steps they are between one and this many physical pixels wide. Closer to
/// one would re-cut more often for a trace that is never noticeably coarser.
const BUCKET_STEP: f64 = 1.25;

/// Finest bucket ever cut. Below this the session is too short to fill a
/// strip, so a coarser bucket loses nothing.
const BUCKET_BASE_SECS: f64 = 1e-3;

/// Width in seconds of the minimap's time buckets for a strip where one
/// physical pixel spans `secs_per_px`.
///
/// The smallest `BUCKET_BASE_SECS × BUCKET_STEP^k` that is at least a pixel
/// wide. Stepping the width geometrically rather than tracking the scale
/// exactly is what keeps bucket membership fixed while the session grows: a
/// point's bucket depends only on its time and the current width, so a new
/// sample moves the buckets on screen without recomposing them. A degenerate
/// scale falls back to the base width.
pub(super) fn bucket_secs(secs_per_px: f64) -> f64 {
    if !secs_per_px.is_finite() || secs_per_px <= BUCKET_BASE_SECS {
        return BUCKET_BASE_SECS;
    }
    let k = (secs_per_px / BUCKET_BASE_SECS).log(BUCKET_STEP).ceil();
    let width = BUCKET_BASE_SECS * BUCKET_STEP.powf(k);
    // Rounding in the logarithm can land one step short of the pixel.
    if width < secs_per_px {
        width * BUCKET_STEP
    } else {
        width
    }
}

/// Condense screen-space points to the vertical extent of each physical pixel
/// column: two points, its lowest and highest sample, or one when the column
/// is flat.
///
/// The strip holds the whole session, so past a sample per pixel there is
/// nothing left to draw between neighbours — only how far the trace reaches
/// in each column, which keeps every spike at full height while cutting the
/// point count to twice the strip's pixel width. Columns are keyed on the
/// physical pixel grid (`x * pixels_per_point`) and x is snapped to the
/// column centre, so a column is one lit pixel rather than a smear across
/// two.
///
/// The two points are ordered so the path leaves each column on the extreme
/// nearer the next column, never on the far one: the path then alternates a
/// vertical run with a rightward step, and no two consecutive segments can
/// point in exactly opposite directions. That matters because epaint's join
/// (`Path::add_open_points`) normalises the sum of the two segment normals,
/// which for an exact reversal is the zero vector, and the corner tessellates
/// into a twisted, half-lit strip. Emitting the samples in time order put an
/// exact reversal in every column holding a local extremum — a crest drew
/// dim and thin. Time order within a column is not preserved, which costs
/// nothing: at pixel resolution only the column's extent is visible.
///
/// Input is assumed to be in time order, which is how the segment cache
/// builds it.
pub(super) fn decimate_columns(
    points: impl Iterator<Item = egui::Pos2>,
    pixels_per_point: f32,
) -> Vec<egui::Pos2> {
    // A zero, negative or non-finite scale would put every point in one
    // column (or none at all); logical pixels are the sane fallback.
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };

    let mut out = Vec::new();
    // A column can only be emitted once the *next* one is complete, since
    // that is what decides which way round its two points go — so one
    // finished column waits in `pending` while `cur` fills.
    let mut pending: Option<Column> = None;
    let mut cur: Option<Column> = None;
    let mut exit_y: Option<f32> = None;

    for p in points {
        let key = (p.x * ppp).floor();
        if let Some(c) = cur.as_mut()
            && c.key == key
        {
            c.add(p.y);
            continue;
        }
        let done = cur.replace(Column::new(key, p.y));
        if let (Some(prev), Some(done)) = (pending, done) {
            exit_y = Some(prev.push(prev.min_is_nearer(done.mid_y()), ppp, &mut out));
        }
        pending = done;
    }

    if let Some(last) = cur {
        if let Some(prev) = pending {
            exit_y = Some(prev.push(prev.min_is_nearer(last.mid_y()), ppp, &mut out));
        }
        // Nothing follows the last column, so it enters on the extreme
        // nearer where the path came from and leaves on the far one. A lone
        // column has no incoming step and can go either way round.
        let exit_min = exit_y.is_some_and(|y| !last.min_is_nearer(y));
        last.push(exit_min, ppp, &mut out);
    }
    out
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
        // Each segment is thinned to one min/max column per time bucket and
        // drawn as a single polyline. Drawn point by point instead, once the
        // history passed a sample per pixel the semi-transparent segments
        // blended twice at their joints and thinned to nothing between them —
        // the trace read as beads and dashes. One path feathers once, joins
        // properly, and the per-column extremes keep the spikes.
        //
        // The buckets are fixed spans of session time, not screen columns.
        // The strip maps the whole session, so every sample shrinks the scale
        // and slides each older point left by an amount proportional to its
        // age; bucketed by screen column, points hopped columns at different
        // moments and the column extents flickered frame to frame — the trace
        // visibly wobbled. Bucketed by time, a bucket's extent is fixed and
        // only its position slides, smoothly, under the antialiasing. The
        // width is stepped so it stays between one and `BUCKET_STEP` physical
        // pixels: never denser than a column, and re-bucketed only at a step.
        let pixels_per_point = ui.ctx().pixels_per_point();
        let bucket = bucket_secs(scale.time_per_px() / pixels_per_point.max(0.1) as f64);
        for seg in raw_segments {
            let projected = seg.iter().map(|&[t, v]| {
                let x = (t / bucket) as f32;
                let y_frac = match y_map {
                    Some((y_lo, range)) => ((v - y_lo) / range) as f32,
                    None => 0.5,
                };
                let y = rect.bottom() - y_frac * rect.height();
                egui::pos2(x, y)
            });
            let mut points = decimate_columns(projected, 1.0);
            if points.len() < 2 {
                continue;
            }
            for p in &mut points {
                p.x = scale.x_of(p.x as f64 * bucket);
            }
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5_f32, line_color),
            ));
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
