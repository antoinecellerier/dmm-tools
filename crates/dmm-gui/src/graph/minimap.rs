//! The minimap strip below the main graph: the full-history overview and the
//! drag gestures that move and resize the view window over it.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};

use super::level::Bucket;
use super::time::{format_time_label, nice_time_interval};
use super::view::pad_range;
use super::{GapKind, Graph};
use crate::a11y::ResponseA11yExt;
use crate::markers::Markers;
use crate::theme::ThemeColors;

/// Minimap height in logical pixels.
pub(super) const MINIMAP_HEIGHT: f32 = 60.0;

/// Half-width of a bracket's resize hit zone, in pixels.
const HANDLE_HALF: f32 = 8.0;

/// Drag distance below which a resize is treated as noise rather than intent.
const RESIZE_DEADZONE_PX: f32 = 0.1;

/// How far the view window may be zoomed in and out, in seconds.
const MIN_WINDOW_SECS: f64 = 2.0;
pub(super) const MAX_WINDOW_SECS: f64 = 3600.0;

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

/// One run of the bucket level, projected to the strip's x/y grid.
///
/// Each bucket contributes its two extremes at the centre of its own column,
/// which is where `decimate_columns` would have put the samples themselves —
/// it keeps nothing but a column's extent, so handing it the extents directly
/// draws the same polyline for a cost that follows the strip's width instead
/// of the session's length. `y_of` projects a value to screen y; x stays in
/// bucket keys for the caller to map back to time.
pub(super) fn level_polyline(run: &[Bucket], y_of: impl Fn(f64) -> f32) -> Vec<egui::Pos2> {
    decimate_columns(
        run.iter().flat_map(|b| {
            let x = b.key as f32 + 0.5;
            [egui::pos2(x, y_of(b.y_min)), egui::pos2(x, y_of(b.y_max))]
        }),
        1.0,
    )
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

/// The arm of a viewport bracket: how far its top and bottom strokes reach
/// into the view.
const BRACKET_ARM: f32 = 4.0;

/// One of the viewport's brackets, `[` at `x` with `arm` positive and `]`
/// with it negative: a stroke down `rect` and one along each edge.
fn paint_bracket(
    painter: &egui::Painter,
    rect: egui::Rect,
    x: f32,
    arm: f32,
    stroke: egui::Stroke,
) {
    painter.line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x + arm, rect.top())],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x, rect.bottom()),
            egui::pos2(x + arm, rect.bottom()),
        ],
        stroke,
    );
}

/// The time labels under the strip, each with a tick, at round steps from
/// `data_min` to `data_max`.
fn paint_time_axis(
    painter: &egui::Painter,
    scale: &MinimapScale,
    rect: egui::Rect,
    (data_min, data_max): (f64, f64),
    label_color: egui::Color32,
) {
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
}

impl Graph {
    /// Render the minimap showing full history with viewport indicator.
    pub fn show_minimap(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        tc: &ThemeColors,
        markers: &Markers,
    ) {
        if self.history.len() < 2 {
            ui.allocate_space(egui::vec2(ui.available_width(), MINIMAP_HEIGHT));
            return;
        }

        let (data_min, data_max) = self.data_time_range();
        let (view_min, view_max) = self.view_bounds();

        // Allocate rect for minimap + label space below, with margin for bracket strokes
        let label_height = 14.0;
        let margin = 4.0; // room for bracket strokes at edges
        let total_height = MINIMAP_HEIGHT + label_height + margin * 2.0;
        let (full_rect, pointer_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), total_height),
            egui::Sense::click_and_drag(),
        );
        let pointer_response = pointer_response
            .on_hover_text(tr!(l10n, "graph-minimap.hover"))
            .a11y_label(&tr!(l10n, "graph-minimap.a11y"));
        // Inset the plot area so brackets at edges have room to render
        let rect = egui::Rect::from_min_size(
            egui::pos2(full_rect.left() + margin, full_rect.top() + margin),
            egui::vec2(full_rect.width() - margin * 2.0, MINIMAP_HEIGHT),
        );

        // Use full_rect painter so nothing gets clipped
        let painter = ui.painter_at(full_rect);
        let scale = MinimapScale::new(rect, data_min, data_max);

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
        self.ensure_level(bucket);

        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        // Behind the trace, like the background.
        self.paint_overload_bands(&painter, &scale, rect, tc.graph_overload_fill());
        self.paint_trace(&painter, &scale, rect, bucket, tc.minimap_line());

        // A marker is a small upward point on the bottom edge — its flag's
        // point on the main plot. One per pixel at most: a burst of markers
        // would otherwise paint the same spot over and over.
        let marker_color = tc.graph_marker();
        let mut last_x = f32::NEG_INFINITY;
        for (x, _) in self.markers_between(markers, data_min, data_max) {
            let x = scale.x_of(x);
            if x - last_x < 1.0 {
                continue;
            }
            last_x = x;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x - 4.5, rect.bottom()),
                    egui::pos2(x, rect.bottom() - 7.0),
                    egui::pos2(x + 4.5, rect.bottom()),
                ],
                marker_color,
                egui::Stroke::NONE,
            ));
        }

        // Draw viewport indicator as [ ] bracket markers
        let vp_left = scale.x_of(view_min);
        let vp_right = scale.x_of(view_max);
        let vp_color = tc.minimap_viewport();
        let vp_stroke = egui::Stroke::new(2.5_f32, vp_color);

        paint_bracket(&painter, rect, vp_left, BRACKET_ARM, vp_stroke);
        paint_bracket(&painter, rect, vp_right, -BRACKET_ARM, vp_stroke);

        paint_time_axis(
            &painter,
            &scale,
            rect,
            (data_min, data_max),
            ui.visuals().weak_text_color(),
        );

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
            // What the plot shows: live keeps a session shorter than the
            // window narrower than it, as a frozen view does.
            width: {
                let (x_min, x_max) = self.view_bounds();
                x_max - x_min
            },
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
            // A pan keeps the width shown, a frozen one included; a resize
            // picks the window's.
            if next.width != window.width {
                self.time_window_secs = next.width;
                self.frozen_width = None;
            }
            self.set_live(next.live);
            self.view_center = next.center;
        }

        // A two-finger swipe on a touchpad moves the viewport the way the
        // scroll goes, as dragging it does: the opposite sense to the plot,
        // where the trace moves and the view goes against it.
        let swipe_free =
            pointer_response.contains_pointer() && self.minimap_drag == MinimapDrag::None;
        if let Some(swipe_px) = self.swipe_x(ui, swipe_free) {
            self.apply_pan(-swipe_px as f64 * scale.time_per_px());
        }

        // Reset drag state when pointer is released
        if !pointer_response.is_pointer_button_down_on() {
            self.minimap_drag = MinimapDrag::None;
        }

        // After `allocate_exact_size`: see `handle_minimap_keys`.
        self.handle_minimap_keys(ui, &pointer_response);

        // Paint focus ring last so it sits on top of the minimap content
        // (background, brackets, time labels) rather than being overdrawn.
        crate::a11y::paint_focus_ring(ui, &pointer_response);
    }

    /// The overload bands, matching the main plot's so the two read the same
    /// way: the level's spans and the one still open.
    fn paint_overload_bands(
        &self,
        painter: &egui::Painter,
        scale: &MinimapScale,
        rect: egui::Rect,
        fill: egui::Color32,
    ) {
        let overload_spans: Vec<(f64, f64)> = self
            .minimap_level
            .iter()
            .flat_map(|level| level.gaps())
            .filter(|(_, _, kind)| *kind == GapKind::Overload)
            .map(|(a, b, _)| (a, b))
            .chain(self.pending_overload_span())
            .collect();
        for &(start, end) in &overload_spans {
            let (x0, x1) = scale.band_x(start, end);
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom())),
                0.0,
                fill,
            );
        }
    }

    /// The trace: the level's buckets of `bucket` seconds, a polyline per run
    /// between interruptions.
    fn paint_trace(
        &mut self,
        painter: &egui::Painter,
        scale: &MinimapScale,
        rect: egui::Rect,
        bucket: f64,
        color: egui::Color32,
    ) {
        // The Y range covers the whole history, so it is the same for every
        // bucket and the level already carries the extremes it needs — the
        // strip pays no scan for it at all. Recomputing it inside the
        // projection once turned the minimap into an O(n²) hot spot that
        // dominated frame time.
        //
        // Deliberately the *auto* range, not the main plot's: a pinned Y range
        // (a Shift-drag box zoom, or a user-entered fixed range) is chosen to
        // frame a detail of the main view, and scaling full history by it puts
        // most points far outside this 60px strip. They clip to a flat line
        // along the edges and overdraw the time axis below — the overview the
        // minimap exists to give disappears exactly when zooming in makes it
        // most useful.
        let y_map = self
            .minimap_level
            .as_ref()
            .and_then(|level| level.value_range())
            .map(|(lo, hi)| pad_range(lo, hi))
            .map(|(lo, hi)| {
                let range = (hi - lo).max(1e-10);
                (lo, range)
            });
        let y_of = |v: f64| -> f32 {
            let y_frac = match y_map {
                Some((y_lo, range)) => ((v - y_lo) / range) as f32,
                None => 0.5,
            };
            rect.bottom() - y_frac * rect.height()
        };
        // Each run of buckets between two interruptions is drawn as a single
        // polyline. Drawn point by point instead, once the history passed a
        // sample per pixel the semi-transparent segments blended twice at their
        // joints and thinned to nothing between them — the trace read as beads
        // and dashes. One path feathers once, joins properly, and the
        // per-bucket extremes keep the spikes.
        if let Some(level) = self.minimap_level.as_mut() {
            for run in level.runs() {
                let mut points = level_polyline(run, y_of);
                if points.len() < 2 {
                    continue;
                }
                for p in &mut points {
                    p.x = scale.x_of(p.x as f64 * bucket);
                }
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, color)));
            }
        }
    }

    /// Keyboard pan when the minimap holds focus.
    ///
    /// Has to run *after* the strip's `allocate_exact_size` (which runs
    /// `interested_in_focus` on `pointer_response`) so that on a Tab press
    /// focus has already advanced away and `pointer_response.has_focus()`
    /// returns false. Otherwise the `move_focus(FocusDirection::None)` below
    /// would also wipe egui's `Focus::begin_pass` snapshot of Tab into
    /// `focus_direction = Next` (egui stores arrow keys and Tab in the same
    /// field), trapping focus on the minimap.
    fn handle_minimap_keys(&mut self, ui: &Ui, pointer_response: &egui::Response) {
        if pointer_response.has_focus() {
            // Up/Down get consumed too even though they don't pan — without
            // that, `end_pass` would walk `find_widget_in_direction` on the
            // begin_pass snapshot and Tab-jump focus to the spatially nearest
            // widget on every Up/Down press.
            let [left, right, up, down] = crate::a11y::consume_arrows(ui.ctx());
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Minimap coordinate math ─────────────────────────────────────────────────

    /// A 400px-wide strip starting at x=100, over a 200-second session.
    fn strip() -> MinimapScale {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 60.0));
        MinimapScale::new(rect, 0.0, 200.0)
    }

    #[test]
    fn the_strip_maps_the_whole_session_across_its_width() {
        let s = strip();
        assert_eq!(s.x_of(0.0), 100.0);
        assert_eq!(s.x_of(200.0), 500.0);
        assert_eq!(s.x_of(100.0), 300.0);
        assert_eq!(s.time_at(300.0), 100.0);
    }

    /// A pointer dragged off either end keeps pushing the view to that end
    /// rather than jumping to the far side.
    #[test]
    fn a_time_read_off_the_strip_clamps_to_its_ends() {
        let s = strip();
        assert_eq!(s.time_at(-5000.0), 0.0);
        assert_eq!(s.time_at(5000.0), 200.0);
    }

    /// A session with no measurable duration must not divide by zero and put
    /// every point at the same x.
    #[test]
    fn an_instantaneous_session_still_has_a_span() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 60.0));
        let s = MinimapScale::new(rect, 12.0, 12.0);
        assert!(s.span() > 0.0);
        assert!(s.x_of(12.0).is_finite());
    }

    /// Whole sessions compress into 400px, so most overload spans are narrower
    /// than a pixel. Drawing them at their true width would draw nothing.
    #[test]
    fn a_sub_pixel_overload_band_is_widened_to_one_pixel() {
        let s = strip();
        let (x0, x1) = s.band_x(100.0, 100.001);
        assert_eq!(x1 - x0, 1.0);
        let (x0, x1) = s.band_x(0.0, 100.0);
        assert_eq!((x0, x1), (100.0, 300.0), "a wide band keeps its width");
    }

    #[test]
    fn a_press_grabs_the_bracket_it_lands_on_and_pans_elsewhere() {
        assert!(drag_target(200.0, 200.0, 400.0) == MinimapDrag::ResizeLeft);
        assert!(drag_target(406.0, 200.0, 400.0) == MinimapDrag::ResizeRight);
        assert!(drag_target(300.0, 200.0, 400.0) == MinimapDrag::Pan);
        // Brackets on top of each other: the nearest edge wins, and a tie goes
        // to the left one.
        assert!(drag_target(203.0, 200.0, 210.0) == MinimapDrag::ResizeLeft);
        assert!(drag_target(207.0, 200.0, 210.0) == MinimapDrag::ResizeRight);
        assert!(drag_target(205.0, 200.0, 210.0) == MinimapDrag::ResizeLeft);

        assert!(near_a_bracket(408.0, 200.0, 400.0));
        assert!(!near_a_bracket(300.0, 200.0, 400.0));
    }

    #[test]
    fn a_drag_shorter_than_the_deadzone_leaves_the_view_alone() {
        let w = ViewWindow {
            center: 100.0,
            width: 60.0,
            live: false,
        };
        assert_eq!(resize(w, Edge::Left, 0.05, &strip(), 200.0), w);
    }

    /// Dragging the left bracket right narrows the window and pins its right
    /// edge, so the data under the right bracket does not move.
    #[test]
    fn dragging_the_left_bracket_pins_the_right_edge() {
        let w = ViewWindow {
            center: 100.0,
            width: 60.0,
            live: true,
        };
        // 0.5 s per pixel over this strip: 20px narrows the window by 10 s.
        let next = resize(w, Edge::Left, 20.0, &strip(), 200.0);
        assert!((next.width - 50.0).abs() < 1e-9);
        assert!((next.center + next.width / 2.0 - 130.0).abs() < 1e-9);
        assert!(!next.live, "resizing takes the view off live follow");
    }

    /// Widening the right bracket past the newest sample is how the user asks
    /// to follow the present again.
    #[test]
    fn widening_the_right_bracket_onto_the_newest_sample_resumes_live() {
        let w = ViewWindow {
            center: 100.0,
            width: 60.0,
            live: false,
        };
        let next = resize(w, Edge::Right, 200.0, &strip(), 200.0);
        assert!(
            (next.center - next.width / 2.0 - 70.0).abs() < 1e-9,
            "left edge pinned"
        );
        assert!(next.live);
        // Short of the end it stays parked.
        assert!(!resize(w, Edge::Right, 20.0, &strip(), 200.0).live);
    }

    /// The window is clamped at both ends of its zoom range, and a drag past
    /// the clamp still leaves the pinned edge where it was.
    #[test]
    fn a_resize_clamps_to_the_zoom_range() {
        let w = ViewWindow {
            center: 100.0,
            width: 60.0,
            live: false,
        };
        let narrow = resize(w, Edge::Left, 10_000.0, &strip(), 200.0);
        assert_eq!(narrow.width, 2.0);
        assert!((narrow.center + narrow.width / 2.0 - 130.0).abs() < 1e-9);
        let wide = resize(w, Edge::Right, 10_000.0, &strip(), 200.0);
        assert_eq!(wide.width, 3600.0);
    }

    /// A window wider than the session has its brackets clamped to the strip's
    /// ends, so a drag has to start from the span the user can actually see or
    /// the first frame jumps.
    #[test]
    fn a_window_wider_than_the_session_snaps_before_resizing() {
        let w = ViewWindow {
            center: 5000.0,
            width: 4000.0,
            live: false,
        };
        let next = resize(w, Edge::Left, 20.0, &strip(), 200.0);
        assert!(
            (next.width - 190.0).abs() < 1e-9,
            "snapped to 200 s, then -10 s"
        );
        assert!((next.center + next.width / 2.0 - 200.0).abs() < 1e-9);
    }

    #[test]
    fn panning_centres_the_window_on_the_pointer_until_it_reaches_the_end() {
        let w = ViewWindow {
            center: 0.0,
            width: 60.0,
            live: true,
        };
        let next = pan(w, 300.0, &strip(), 200.0);
        assert_eq!(next.center, 100.0);
        assert_eq!(next.width, 60.0);
        assert!(
            !next.live,
            "panning back into the history stops live follow"
        );

        // Far enough right that the window's own right edge covers the newest
        // sample: resume live rather than parking just short of it.
        let at_end = pan(w, 500.0, &strip(), 200.0);
        assert!(at_end.live);
        assert_eq!(at_end.center, w.center, "live follow picks its own centre");
    }

    // ── Minimap trace decimation ────────────────────────────────────────────────

    /// A whole session compresses into a 640px strip, so a long run puts
    /// thousands of samples on one pixel column. All that can be seen of them is
    /// how far up and down the column they reach.
    #[test]
    fn a_dense_column_collapses_to_its_extremes() {
        let dense = (0..1000).map(|i| egui::pos2(0.3, (i % 100) as f32));
        let out = decimate_columns(dense, 1.0);

        assert_eq!(out.len(), 2, "one vertical run, not a point per sample");
        assert!(out.iter().any(|p| p.y == 0.0), "column minimum survives");
        assert!(out.iter().any(|p| p.y == 99.0), "column maximum survives");
        assert!(
            out.iter().all(|p| p.x == 0.5),
            "every point sits on the column centre"
        );

        let flat = decimate_columns((0..1000).map(|_| egui::pos2(0.3, 7.0)), 1.0);
        assert_eq!(flat, vec![egui::pos2(0.5, 7.0)], "a flat column is a point");
    }

    /// The overview exists to show the excursions, so a single sample far from
    /// its neighbours must not be averaged or dropped away.
    #[test]
    fn a_lone_spike_in_a_flat_run_survives() {
        let flat_with_spike = (0..200).map(|i| {
            let y = if i == 137 { 99.0 } else { 10.0 };
            egui::pos2(i as f32 * 0.01, y)
        });
        let out = decimate_columns(flat_with_spike, 1.0);

        assert!(out.iter().any(|p| p.y == 99.0), "the spike is still drawn");
    }

    /// A short session has fewer samples than columns; nothing may be lost, and
    /// each point moves only onto its column's centre.
    #[test]
    fn one_point_per_column_passes_through_snapped_to_the_column_centres() {
        let sparse = (0..5).map(|i| egui::pos2(i as f32 + 0.2, i as f32 * 3.0));
        let out = decimate_columns(sparse, 1.0);

        assert_eq!(out.len(), 5);
        let xs: Vec<f32> = out.iter().map(|p| p.x).collect();
        let ys: Vec<f32> = out.iter().map(|p| p.y).collect();
        assert_eq!(xs, vec![0.5, 1.5, 2.5, 3.5, 4.5]);
        assert_eq!(ys, vec![0.0, 3.0, 6.0, 9.0, 12.0]);
    }

    /// On a HiDPI screen a logical pixel is two physical ones, so the columns
    /// halve — and the polyline still has to come out left to right.
    #[test]
    fn columns_follow_the_physical_pixel_grid_and_stay_in_order() {
        let out = decimate_columns(
            [
                egui::pos2(0.2, 1.0),
                egui::pos2(0.4, 2.0),
                egui::pos2(0.7, 3.0),
            ]
            .into_iter(),
            2.0,
        );

        let xs: Vec<f32> = out.iter().map(|p| p.x).collect();
        assert_eq!(
            xs,
            vec![0.25, 0.25, 0.75],
            "0.2 and 0.4 share a half-pixel column, 0.7 opens the next"
        );
        assert!(
            out.windows(2).all(|w| w[0].x <= w[1].x),
            "columns come out left to right"
        );
        assert_eq!(out[2].y, 3.0);
    }

    /// egui reports the pixel density from the window; a missing or nonsense one
    /// must not fold the whole trace into a single column.
    #[test]
    fn a_nonsense_pixel_density_falls_back_to_logical_pixels() {
        let points = [egui::pos2(0.5, 1.0), egui::pos2(1.5, 2.0)];
        for ppp in [0.0, -2.0, f32::NAN] {
            let out = decimate_columns(points.into_iter(), ppp);
            assert_eq!(out.len(), 2, "ppp {ppp}");
            assert_eq!(out[0].x, 0.5);
            assert_eq!(out[1].x, 1.5);
        }
    }

    #[test]
    fn an_empty_history_gives_no_points_and_a_single_sample_gives_one() {
        assert!(decimate_columns(std::iter::empty(), 1.0).is_empty());

        let one = decimate_columns(std::iter::once(egui::pos2(7.3, 4.0)), 1.0);
        assert_eq!(one, vec![egui::pos2(7.5, 4.0)]);
    }

    /// epaint tessellates a corner between two exactly opposite segments into a
    /// twisted, half-lit strip, so the polyline must never double back on
    /// itself — nor repeat a point, which is the same corner with no direction
    /// at all. Dense noisy data is where the old time-ordered output hit this on
    /// nearly every column.
    #[test]
    fn the_polyline_never_doubles_back() {
        let sample = |i: i32| {
            let x = i as f32 * 200.0 / 3000.0;
            // A one-sample spike every few hundred samples, on top of a sine
            // dense enough to put ~15 samples in every column.
            let spike = if i % 613 == 0 { 45.0 } else { 0.0 };
            egui::pos2(x, 60.0 + 20.0 * (x * 0.15).sin() + spike)
        };
        let out = decimate_columns((0..3000).map(sample), 1.0);

        // Two segments can only be exactly opposite when both are vertical, and
        // a column contributes at most one vertical run, so the invariant to
        // hold is that no three points in a row share an x — plus no repeated
        // point, which is a corner with no direction at all.
        for w in out.windows(3) {
            assert!(
                w[0].x != w[1].x || w[1].x != w[2].x,
                "two vertical segments in a row at x {}",
                w[1].x
            );
        }
        for w in out.windows(2) {
            assert!(w[0] != w[1], "zero-length segment at {:?}", w[0]);
        }

        // Nothing was lost on the way: every column still reaches as far up and
        // down as its samples did.
        let mut extents: std::collections::HashMap<i32, (f32, f32)> =
            std::collections::HashMap::new();
        for p in (0..3000).map(sample) {
            let e = extents.entry(p.x.floor() as i32).or_insert((p.y, p.y));
            e.0 = e.0.min(p.y);
            e.1 = e.1.max(p.y);
        }
        for (col, (y_min, y_max)) in extents {
            let x = col as f32 + 0.5;
            assert!(out.contains(&egui::pos2(x, y_min)), "column {col} minimum");
            assert!(out.contains(&egui::pos2(x, y_max)), "column {col} maximum");
        }
    }

    /// Which end of a column's vertical run comes second decides how the step to
    /// the next column runs: leaving on the end facing that column keeps the
    /// step short and stops the path from reversing into it.
    #[test]
    fn the_exit_end_faces_the_next_column() {
        let column = |x: f32, y_min: f32, y_max: f32| {
            [egui::pos2(x + 0.1, y_min), egui::pos2(x + 0.2, y_max)].into_iter()
        };

        let down = decimate_columns(
            column(0.0, 10.0, 12.0)
                .chain(column(1.0, 30.0, 40.0))
                .chain(column(2.0, 50.0, 52.0)),
            1.0,
        );
        assert_eq!(
            (down[2].y, down[3].y),
            (30.0, 40.0),
            "the next column is further down the screen"
        );

        let up = decimate_columns(
            column(0.0, 10.0, 12.0)
                .chain(column(1.0, 30.0, 40.0))
                .chain(column(2.0, 0.0, 2.0)),
            1.0,
        );
        assert_eq!(
            (up[2].y, up[3].y),
            (40.0, 30.0),
            "the next column is further up the screen"
        );
    }

    /// A bucket is never narrower than the pixel it is drawn in — that would put
    /// two extents in one column and bring the beads back — and never so wide
    /// that the trace goes coarse.
    #[test]
    fn a_bucket_is_at_least_a_pixel_and_less_than_a_step_wider() {
        let mut secs_per_px = 0.002;
        while secs_per_px < 100.0 {
            let bucket = bucket_secs(secs_per_px);
            assert!(bucket >= secs_per_px, "{bucket} < {secs_per_px}");
            assert!(bucket < secs_per_px * 1.25, "{bucket} vs {secs_per_px}");
            secs_per_px *= 1.03;
        }
    }

    /// The whole point: while the session grows within a step, the bucket width
    /// does not move, so the buckets keep their members and the trace slides
    /// instead of flickering.
    #[test]
    fn the_bucket_width_holds_still_between_steps() {
        let mut widths = std::collections::BTreeSet::new();
        let mut secs_per_px = 0.1;
        while secs_per_px <= 1.0 {
            widths.insert(bucket_secs(secs_per_px).to_bits());
            secs_per_px += 0.001;
        }
        // A decade at ×1.25 per step is log(10)/log(1.25) ≈ 10.3 steps.
        assert!(
            widths.len() <= 12,
            "{} distinct widths over a decade",
            widths.len()
        );
    }

    #[test]
    fn a_degenerate_scale_falls_back_to_the_finest_bucket() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(bucket_secs(bad), bucket_secs(0.0), "{bad}");
        }
        assert!(bucket_secs(0.0) > 0.0);
    }

    /// Adding a sample must not recompose the buckets already on screen: keyed
    /// on time rather than on screen column, the earlier output is a prefix of
    /// the later one, save for the bucket the new sample joins.
    #[test]
    fn a_new_sample_leaves_earlier_buckets_untouched() {
        let bucket = bucket_secs(0.15);
        let sample = |i: usize| {
            let t = i as f64 * 0.1;
            egui::pos2((t / bucket) as f32, (i as f32 * 0.7).sin() * 20.0)
        };
        let before = decimate_columns((0..600).map(sample), 1.0);
        let after = decimate_columns((0..601).map(sample), 1.0);
        // Everything up to the last bucket of `before` is reproduced exactly.
        let last_x = before.last().map(|p| p.x).expect("non-empty");
        let stable = before.iter().take_while(|p| p.x < last_x).count();
        assert!(stable > 300, "prefix of only {stable} points");
        assert_eq!(&before[..stable], &after[..stable]);
    }
}
