//! Painting the main graph: the traces, the analysis overlays, the plot key
//! and the accessibility summary that describes them.

use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{
    AxisHints, HLine, Line, Plot, PlotBounds, PlotPoints, PlotTransform, Points, Span, VLine,
};
use std::time::Instant;

use super::time::format_time_axis_label;
use super::{GapKind, Graph, OverlaySeries, SegmentsAndGaps};
use crate::theme::ThemeColors;

/// One sub-value trace ready to draw: (overlay index, name, segments).
///
/// The index — not a colour — because it is what both the trace and its key
/// row derive their colour and line style from, so the two cannot drift.
pub(super) type OverlayTrace = (usize, String, Vec<Vec<[f64; 2]>>);

/// How one row of the plot key draws its line sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyStyle {
    /// The plotted series: solid, in the graph line colour.
    Plotted,
    /// A sub-value trace, identified by its overlay index — the same index
    /// `overlay_color_and_style` uses for the line itself.
    Overlay(usize),
}

/// Minimum font size for the plot key, per `.claude/rules/gui.md`. The body
/// text style is normally larger; this only bites if a user's style shrinks it.
const MIN_KEY_FONT_SIZE: f32 = 11.0;

/// Width of the line sample drawn at the left of each key row.
const KEY_SAMPLE_WIDTH: f32 = 24.0;

/// Shapes for one key row's line sample, matching how `egui_plot` renders the
/// same `LineStyle` on the plot — otherwise the key would promise a dash
/// pattern the trace doesn't use.
fn key_line_sample(
    a: egui::Pos2,
    b: egui::Pos2,
    color: egui::Color32,
    style: egui_plot::LineStyle,
) -> Vec<egui::Shape> {
    const WIDTH: f32 = 1.5;
    match style {
        egui_plot::LineStyle::Solid => {
            vec![egui::Shape::line_segment(
                [a, b],
                egui::Stroke::new(WIDTH, color),
            )]
        }
        egui_plot::LineStyle::Dotted { spacing } => {
            egui::Shape::dotted_line(&[a, b], color, spacing, WIDTH)
        }
        egui_plot::LineStyle::Dashed { length } => {
            // The same golden-ratio gap `egui_plot::LineStyle::style_line` uses.
            const GOLDEN_RATIO: f32 = 0.618_034;
            egui::Shape::dashed_line(
                &[a, b],
                egui::Stroke::new(WIDTH, color),
                length,
                length * GOLDEN_RATIO,
            )
        }
    }
}

/// Quantize a `f64` to ~3 decimal digits before hashing so animated plot
/// transforms (which produce sub-pixel jitter on the y bounds) don't
/// invalidate label caches every frame. Returns an `i64` so the result is
/// `Hash`-stable and not affected by `f64::NaN` weirdness.
pub(super) fn quantize_for_hash(v: f64) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v * 1000.0).round() as i64
    }
}

/// Pre-computed data needed by `paint_overlay_labels` to draw text labels
/// for mean, reference, and cursor overlays after the plot has been rendered.
struct OverlayLabelData {
    show_mean: bool,
    mean_value: Option<f64>,
    show_ref: bool,
    ref_values: Vec<f64>,
    cursors_active: bool,
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    cursor_va: Option<f64>,
    cursor_vb: Option<f64>,
    overlay_unit: String,
    view_max: f64,
    mean_color: egui::Color32,
    ref_color: egui::Color32,
    cursor_color: egui::Color32,
}

impl Graph {
    /// Build segment and gap data for a slice of history, suitable for
    /// passing to egui_plot. Only the points in `[start, end)` are visited.
    pub(super) fn build_segments_for_range(&self, start: usize, end: usize) -> SegmentsAndGaps {
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut gaps: Vec<(f64, f64, GapKind)> = Vec::new();
        let mut current_segment: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);

            if let Some(prev) = prev_time
                && let Some(kind) = self.breaks_before(prev, point)
                && !current_segment.is_empty()
            {
                let gap_start = self.elapsed_secs(prev);
                // An interruption can be two things end to end: a stretch the
                // meter reported on, then a stretch it didn't. Losing the link
                // mid-overload is exactly that, and folding the silence into
                // the band would claim the meter was over range for a period
                // it never reported at all.
                match (kind, point.break_last_sample) {
                    (GapKind::Overload, Some(last)) if point.break_had_data_loss => {
                        let heard_until = self.elapsed_secs(last);
                        gaps.push((gap_start, heard_until, GapKind::Overload));
                        gaps.push((heard_until, t, GapKind::NoData));
                    }
                    _ => gaps.push((gap_start, t, kind)),
                }
                segments.push(std::mem::take(&mut current_segment));
            }

            current_segment.push([t, point.value]);
            prev_time = Some(point.time);
        }

        if !current_segment.is_empty() {
            segments.push(current_segment);
        }

        (segments, gaps)
    }

    /// Build the drawable segments of one overlay over `[start, end)`.
    ///
    /// Deliberately separate from `build_segments_for_range` rather than
    /// generalising it: that one also derives the gap ranges the bands and
    /// dropout markers are drawn from, and its data-loss split is subtle and
    /// tested. Overlays draw no bands and no markers — they only need to stop
    /// wherever the main trace stops (time gap, overload, data loss) plus
    /// wherever the sub-value itself is missing.
    pub(super) fn build_overlay_segments_for_range(
        &self,
        o: &OverlaySeries,
        start: usize,
        end: usize,
    ) -> Vec<Vec<[f64; 2]>> {
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for i in start..end {
            let point = &self.history[i];
            if let Some(prev) = prev_time
                && self.breaks_before(prev, point).is_some()
                && !current.is_empty()
            {
                segments.push(std::mem::take(&mut current));
            }
            prev_time = Some(point.time);

            match o.values.get(i).copied().flatten() {
                Some(v) => current.push([self.elapsed_secs(point.time), v]),
                None if !current.is_empty() => segments.push(std::mem::take(&mut current)),
                None => {}
            }
        }

        if !current.is_empty() {
            segments.push(current);
        }
        segments
    }

    /// The overlays the user has not switched off, paired with their index.
    ///
    /// The index is the overlay's position in `self.overlays`, which is what
    /// keys its colour and line style — stable for the life of the session, so
    /// hiding one does not reshuffle the palette of the others.
    pub(super) fn shown_overlays(&self) -> impl Iterator<Item = (usize, &OverlaySeries)> {
        self.overlays
            .iter()
            .enumerate()
            .filter(|(_, o)| !self.hidden_overlays.contains(&o.label))
    }

    /// The sub-value traces actually drawn over `[start, end)`.
    ///
    /// Leaves out the ones the user switched off, and the ones with no points
    /// in this window — an overlay that isn't drawn must not appear in the key
    /// either.
    pub(super) fn visible_overlay_traces(&self, start: usize, end: usize) -> Vec<OverlayTrace> {
        self.shown_overlays()
            .filter_map(|(k, o)| {
                let segments = self.build_overlay_segments_for_range(o, start, end);
                (!segments.is_empty()).then(|| (k, o.label.clone(), segments))
            })
            .collect()
    }

    /// Colour and line style of one overlay trace.
    ///
    /// Keyed on the overlay's index — never on a hash of its label, which
    /// would reshuffle the palette when a sub-value appears or disappears
    /// mid-capture. The line style carries the same information without
    /// colour, as `.claude/rules/gui.md` requires.
    fn overlay_color_and_style(
        tc: &ThemeColors,
        index: usize,
    ) -> (egui::Color32, egui_plot::LineStyle) {
        let style = match index % 4 {
            0 => egui_plot::LineStyle::dashed_loose(),
            1 => egui_plot::LineStyle::dotted_loose(),
            2 => egui_plot::LineStyle::dashed_dense(),
            _ => egui_plot::LineStyle::dotted_dense(),
        };
        (tc.graph_overlay(index), style)
    }

    /// Name the plotted series goes by on the plot and in the key.
    fn plotted_series_name(&self) -> String {
        self.current_series
            .clone()
            .unwrap_or_else(|| "Main".to_string())
    }

    /// Rows of the plot key: the plotted series, then each drawn overlay in
    /// `self.overlays` order.
    ///
    /// Empty when nothing is overlaid — a single-series graph gets no key, and
    /// looks exactly as it did before sub-values existed. Takes the traces
    /// `show_main` is about to draw rather than recomputing them, so the key
    /// cannot list a line that isn't there.
    pub(super) fn key_entries(&self, drawn: &[OverlayTrace]) -> Vec<(String, KeyStyle)> {
        if drawn.is_empty() {
            return Vec::new();
        }
        let mut entries = Vec::with_capacity(drawn.len() + 1);
        entries.push((self.plotted_series_name(), KeyStyle::Plotted));
        entries.extend(
            drawn
                .iter()
                .map(|(k, label, _)| (label.clone(), KeyStyle::Overlay(*k))),
        );
        entries
    }

    /// Paint the plot key in the top-left of the plot area.
    ///
    /// A key, not a control: egui_plot's own `Legend` renders show/hide
    /// checkboxes, but `Plot::reset()` — which pins the view every frame while
    /// keeping pointer events — also clears the plot's `hidden_items`, so a
    /// click on one had no effect past the frame it happened in. The **Show:**
    /// chips in the toolbar are the control instead.
    fn paint_plot_key(
        ui: &Ui,
        plot_rect: egui::Rect,
        entries: &[(String, KeyStyle)],
        tc: &ThemeColors,
    ) {
        if entries.is_empty() {
            return;
        }
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = font.size.max(MIN_KEY_FONT_SIZE);

        const PAD: f32 = 6.0;
        const GAP: f32 = 6.0;
        let text_color = ui.visuals().text_color();
        let painter = ui.painter_at(plot_rect);

        // Measure first so the panel is exactly as wide as its widest row.
        let galleys: Vec<_> = entries
            .iter()
            .map(|(name, _)| painter.layout_no_wrap(name.clone(), font.clone(), text_color))
            .collect();
        let text_width = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max);
        let row_height = galleys.iter().map(|g| g.size().y).fold(font.size, f32::max);
        let width = PAD * 2.0 + KEY_SAMPLE_WIDTH + GAP + text_width;
        let height = PAD * 2.0 + row_height * entries.len() as f32;
        let rect = egui::Rect::from_min_size(
            plot_rect.left_top() + egui::vec2(PAD, PAD),
            egui::vec2(width, height),
        );
        // `window_fill`, not `extreme_bg_color`: the App assigns the latter
        // the plot's own background, so a key painted in it would be invisible
        // apart from its border. `window_fill` is the panel ground either
        // theme pairs with `text_color`, so the names keep their normal
        // contrast, and 85% alpha keeps the grid faintly readable underneath
        // instead of punching a hole in the plot.
        painter.rect_filled(rect, 4.0, ui.visuals().window_fill.gamma_multiply(0.85));
        painter.rect_stroke(
            rect,
            4.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );

        for (i, ((_, style), galley)) in entries.iter().zip(&galleys).enumerate() {
            let top = rect.top() + PAD + row_height * i as f32;
            let mid = top + row_height / 2.0;
            let (color, line_style) = match style {
                KeyStyle::Plotted => (tc.graph_line(), egui_plot::LineStyle::Solid),
                KeyStyle::Overlay(k) => Self::overlay_color_and_style(tc, *k),
            };
            painter.extend(key_line_sample(
                egui::pos2(rect.left() + PAD, mid),
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH, mid),
                color,
                line_style,
            ));
            painter.galley(
                egui::pos2(rect.left() + PAD + KEY_SAMPLE_WIDTH + GAP, top),
                galley.clone(),
                text_color,
            );
        }
    }

    /// Render the main graph.
    pub fn show_main(&mut self, ui: &mut Ui, tc: &ThemeColors) {
        let (view_min, view_max) = self.view_bounds();

        // Build segments and gaps for the visible slice only, plus one
        // point on each side so line segments at the view edges render
        // correctly and aren't clipped to nothing.
        let (vis_start, vis_end) = self.visible_index_range(view_min, view_max);
        let ext_start = vis_start.saturating_sub(1);
        let ext_end = (vis_end + 1).min(self.history.len());
        let (visible_segments, visible_gaps) = self.build_segments_for_range(ext_start, ext_end);
        // Overload spans, including one still in progress. Used both to draw
        // the bands and to answer the crosshair tooltip, which is the only
        // non-visual cue available — `Span` is never a hover target.
        let overload_spans: Vec<(f64, f64)> = visible_gaps
            .iter()
            .filter(|(_, _, kind)| *kind == GapKind::Overload)
            .map(|&(a, b, _)| (a, b))
            .chain(self.pending_overload_span())
            .collect();

        // Theme-aware colors from shared palette
        let line_color = tc.graph_line();
        let gap_color = tc.graph_gap();
        let overload_edge = tc.graph_overload();
        let overload_fill = tc.graph_overload_fill();
        let mean_color = tc.graph_mean();
        let ref_color = tc.graph_ref();
        let cross_color = tc.graph_crossing();
        let cursor_color = tc.graph_cursor();
        let cursor_color_dim = tc.graph_cursor_dim();
        let env_color = tc.graph_envelope();

        // Sub-value traces over the same visible slice as the main series,
        // minus any the user switched off in the toolbar's Show: group.
        let overlay_traces = self.visible_overlay_traces(ext_start, ext_end);
        let key_entries = self.key_entries(&overlay_traces);
        let multi_series = !overlay_traces.is_empty();
        // Every drawn line carries the name of its series, which is what the
        // hover label reports; every helper item is named "" and falls through
        // to the plain form.
        let main_name = self.plotted_series_name();

        let can_interact = !self.live;
        let shift_held = ui.input(|i| i.modifiers.shift);
        let bbox_active = self.bbox_zoom_start_px.is_some();
        // Plain drag-to-pan is allowed even in live mode — starting a drag
        // drops out of live (see handle_interaction). Scroll-zoom stays gated
        // on !live: the first scroll exits live without zooming, the second
        // zooms. Bbox and shift-drag always suppress the built-in pan.
        let allow_plot_x_drag = !shift_held && !bbox_active;
        let allow_plot_x_zoom = can_interact && !bbox_active;

        // Compute Y bounds from visible data
        let (y_min, y_max) = self
            .y_range_for_view(view_min, view_max, true)
            .unwrap_or((-1.0, 1.0));

        let unit = self.current_unit.clone();
        let y_axis = AxisHints::new_y().formatter(move |mark, _range| {
            let decimals = (-mark.step_size.log10().round() as usize).min(6);
            let val = eframe::emath::format_with_decimals_in_range(mark.value, decimals..=decimals);
            if unit.is_empty() {
                val
            } else {
                format!("{val} {unit}  ")
            }
        });

        let x_axis = AxisHints::new_x()
            .formatter(|mark, _range| format_time_axis_label(mark.value, mark.step_size));

        let show_envelope = self.show_envelope;
        let (env_min, env_max) = if show_envelope {
            self.build_envelope(view_min, view_max, self.envelope_window_secs)
        } else {
            (Vec::new(), Vec::new())
        };
        let show_mean = self.show_mean;
        let show_ref = self.show_ref_line;
        let ref_values = self.ref_line_values.clone();
        let show_crossings = self.show_crossings;
        let crossings = if show_ref && show_crossings && !ref_values.is_empty() {
            self.find_crossings(&ref_values, view_min, view_max)
        } else {
            Vec::new()
        };
        let cursors_active = self.cursors_active;
        let cursor_a = self.cursor_a;
        let cursor_b = self.cursor_b;
        let cursor_va = cursor_a.and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let cursor_vb = cursor_b.and_then(|t| self.nearest_point(t).map(|(_, v)| v));
        let mean_value = self.visible_stats().and_then(|s| s.avg());

        let cursor_unit = self.current_unit.clone();
        // Moved into the label_formatter closure, which is rebuilt each frame.
        let tooltip_spans = overload_spans.clone();
        let plot = Plot::new("main_plot")
            .height(ui.available_height().max(60.0))
            .allow_drag(Vec2b::new(allow_plot_x_drag, false))
            .allow_zoom(Vec2b::new(allow_plot_x_zoom, false))
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .reset()
            .custom_x_axes(vec![x_axis])
            .custom_y_axes(vec![y_axis])
            .y_axis_min_width(60.0)
            .cursor_color(tc.graph_crosshair())
            .label_formatter(move |name, point| {
                let t = point.x;
                let time_label = if t < 60.0 {
                    format!("{t:.1} s")
                } else {
                    let m = (t / 60.0).floor();
                    let s = t % 60.0;
                    format!("{m:.0}m {s:.1}s")
                };
                // Inside a band there is no measured value to report, and
                // `Span` can't be hovered itself (its geometry is None), so
                // this is where the condition gets named. It is also the only
                // cue that isn't visual.
                if tooltip_spans.iter().any(|&(a, b)| t >= a && t <= b) {
                    return format!("{time_label}\noverload");
                }
                // With several traces on the same axes the number alone is
                // ambiguous, so name the one being hovered. Helper items carry
                // an empty name and fall through to the plain form.
                if multi_series && !name.is_empty() {
                    return format!("{time_label}\n{name}: {:.4} {cursor_unit}", point.y);
                }
                format!("{time_label}\n{:.4} {}", point.y, cursor_unit)
            });
        let response = plot.show(ui, |plot_ui| {
            // Set exact bounds: our X view range + computed Y range
            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                [view_min, y_min],
                [view_max, y_max],
            ));

            // Overload bands, before everything else so they sit behind the
            // data. An overload is the meter reporting a condition, not an
            // absence of one, so it reads as a filled region rather than the
            // dashed edges used for a dropout — a distinction that survives
            // without colour.
            //
            // Drawn at true duration with no minimum width: a brief excursion
            // collapses to a line rather than overstating how long the meter
            // was over range. `Span` clamps itself to the visible range, so a
            // band wider than the window still fills the plot — the case where
            // two edge markers would both be off-screen and show nothing.
            for &(start, end) in &overload_spans {
                plot_ui.span(
                    // Empty name: `Span` renders its name as a label inside
                    // the band, which would collide at narrow widths.
                    Span::new("", start..=end)
                        .fill(overload_fill)
                        .border_color(overload_edge)
                        .border_style(egui_plot::LineStyle::Solid),
                );
            }

            // Min/max envelope (drawn first so it's behind the data line)
            if show_envelope && !env_min.is_empty() {
                plot_ui.line(
                    Line::new("", PlotPoints::new(env_max.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.line(
                    Line::new("", PlotPoints::new(env_min.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Sub-values first: the plotted series goes on top of them.
            for (k, label, segments) in &overlay_traces {
                let (color, style) = Self::overlay_color_and_style(tc, *k);
                for seg in segments {
                    plot_ui.line(
                        Line::new(label.clone(), PlotPoints::new(seg.clone()))
                            .color(color)
                            .style(style),
                    );
                }
            }

            for seg in &visible_segments {
                plot_ui.line(
                    Line::new(main_name.clone(), PlotPoints::new(seg.clone())).color(line_color),
                );
            }

            // No data: two dashed edges, no fill.
            for &(gap_start, gap_end, kind) in &visible_gaps {
                if kind != GapKind::NoData {
                    continue;
                }
                plot_ui.vline(
                    VLine::new("", gap_start)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.vline(
                    VLine::new("", gap_end)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Mean line overlay
            if show_mean && let Some(avg) = mean_value {
                plot_ui.hline(
                    HLine::new("", avg)
                        .color(mean_color)
                        .style(egui_plot::LineStyle::dashed_loose()),
                );
            }

            // Reference line overlays
            if show_ref {
                for &v in &ref_values {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(ref_color)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            }

            // Trigger crossing markers (where data crosses reference lines)
            if !crossings.is_empty() {
                plot_ui.points(
                    Points::new("", PlotPoints::new(crossings.clone()))
                        .color(cross_color)
                        .radius(4.0_f32)
                        .shape(egui_plot::MarkerShape::Diamond),
                );
            }

            // Measurement cursors (vertical + horizontal Y-value lines)
            if cursors_active {
                if let Some(t) = cursor_a {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_va {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(cursor_color_dim)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
                if let Some(t) = cursor_b {
                    plot_ui.vline(VLine::new("", t).color(cursor_color));
                }
                if let Some(v) = cursor_vb {
                    plot_ui.hline(
                        HLine::new("", v)
                            .color(cursor_color_dim)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            }
        });

        let overlay = OverlayLabelData {
            show_mean,
            mean_value,
            show_ref,
            ref_values,
            cursors_active,
            cursor_a,
            cursor_b,
            cursor_va,
            cursor_vb,
            overlay_unit: self.current_unit.clone(),
            view_max,
            mean_color,
            ref_color,
            cursor_color,
        };
        Self::paint_overlay_labels(ui, &response.response, &response.transform, &overlay);
        // Top-left, where `paint_overlay_labels` never draws — the Mean/Ref
        // and cursor labels are anchored to the right edge.
        Self::paint_plot_key(ui, response.response.rect, &key_entries, tc);
        self.handle_interaction(ui, &response.response, &response.transform, can_interact);
        self.update_plot_a11y_label(ui, response.response.id, y_min, y_max);
        // Draw a focus ring on the main plot body when it's keyboard-focused.
        // Note: egui_plot also allocates separate focusable responses for the
        // X and Y axes — those receive Tab but don't draw a focus indicator.
        // Making those invisible to Tab would require patching egui_plot.
        crate::a11y::paint_focus_ring(ui, &response.response);
    }

    /// Set an AccessKit label on the plot that summarizes current state so
    /// screen readers have a text alternative to the pixels. Throttled: the
    /// label is only re-formatted when the underlying state changes.
    fn update_plot_a11y_label(&mut self, ui: &Ui, plot_id: egui::Id, y_min: f64, y_max: f64) {
        use std::hash::{Hash, Hasher};
        let last_value = self.history.back().map(|p| p.value);
        // Only the traces actually drawn are spoken: a sub-value the user
        // switched off is not on screen, so announcing it would describe a
        // graph that isn't there.
        let shown_labels: Vec<&str> = self
            .shown_overlays()
            .map(|(_, o)| o.label.as_str())
            .collect();
        let sig = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.time_window_secs.to_bits().hash(&mut h);
            // Quantize y bounds before hashing. Otherwise sub-pixel jitter
            // from animated auto-fit (and last-bit f64 noise from plot
            // transforms) busts the cache every frame, defeating the
            // throttle and forcing a fresh format! per render.
            quantize_for_hash(y_min).hash(&mut h);
            quantize_for_hash(y_max).hash(&mut h);
            self.history.len().hash(&mut h);
            self.live.hash(&mut h);
            last_value.map(f64::to_bits).hash(&mut h);
            self.current_unit.hash(&mut h);
            // Which series is plotted, and what is drawn beside it, are both
            // spoken below — so both have to bust the cache.
            self.current_series.as_deref().unwrap_or("").hash(&mut h);
            shown_labels.hash(&mut h);
            // Mode + display_raw drive the spoken last-reading; if either
            // changes (e.g. mode switch during paused playback) the label
            // must follow.
            self.current_mode.as_deref().unwrap_or("").hash(&mut h);
            self.last_display_raw.as_deref().unwrap_or("").hash(&mut h);
            // The over-range state is announced below, so it has to bust the
            // cache — otherwise the band appears with no spoken counterpart.
            self.pending_break.is_some().hash(&mut h);
            h.finish()
        };
        if sig != self.a11y_label_sig {
            let also = if shown_labels.is_empty() {
                String::new()
            } else {
                format!(" Also showing {}.", shown_labels.join(", "))
            };
            self.a11y_label_sig = sig;
            let unit = if self.current_unit.is_empty() {
                ""
            } else {
                &self.current_unit
            };
            let state = if self.live { "live" } else { "paused" };
            // Prefer the meter's own raw display string for the spoken
            // reading — it already encodes range/scale (e.g. "  1.234"
            // for a 22 V range vs " 12.34" for a 220 V range), so AT
            // users hear what sighted users see. Fall back to the f64
            // value only if the protocol doesn't provide display_raw.
            let reading = match (self.last_display_raw.as_deref(), last_value) {
                // Over range is the present state, so it outranks the last
                // plotted value — which is stale by definition while the
                // meter has nothing to measure. The band is otherwise a
                // purely visual cue.
                _ if self.pending_break == Some(GapKind::Overload) => {
                    "currently over range".to_string()
                }
                (Some(raw), _) => format!("last reading {} {unit}", raw.trim()),
                (None, Some(v)) => format!("last reading {v:.4} {unit}"),
                (None, None) => "no data".to_string(),
            };
            // The plot draws several traces at once for a multi-display
            // meter, and which one the axis belongs to is otherwise purely
            // visual — the key painted in its corner.
            let of_series = match self.current_series.as_deref() {
                Some(label) => format!(" of {label}"),
                None => String::new(),
            };
            self.a11y_label = format!(
                "Measurement plot{of_series}. {:.0} second window. Y axis {:.3} to {:.3} {unit}. {} samples.{also} {}. {}.",
                self.time_window_secs,
                y_min,
                y_max,
                self.history.len(),
                state,
                reading,
            );
        }
        crate::a11y::set_accessible_label(ui, plot_id, &self.a11y_label);
    }

    /// Paint text labels for overlays (mean, reference lines, cursors) using the
    /// UI painter so they render outside the plot's clip rect.
    fn paint_overlay_labels(
        ui: &Ui,
        plot_response: &egui::Response,
        transform: &PlotTransform,
        data: &OverlayLabelData,
    ) {
        let painter = ui.painter();
        let label_font = egui::FontId::proportional(12.0);
        let plot_rect = plot_response.rect;

        // Mean line label — anchored to right edge of plot rect
        if data.show_mean
            && let Some(avg) = data.mean_value
        {
            let y_pos = transform
                .position_from_point(&egui_plot::PlotPoint::new(data.view_max, avg))
                .y
                .clamp(plot_rect.top() + 12.0, plot_rect.bottom() - 2.0);
            painter.text(
                egui::pos2(plot_rect.right() - 4.0, y_pos - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("Mean: {avg:.4} {}", data.overlay_unit),
                label_font.clone(),
                data.mean_color,
            );
        }

        // Reference line labels
        if data.show_ref {
            for &v in &data.ref_values {
                let y_pos = transform
                    .position_from_point(&egui_plot::PlotPoint::new(data.view_max, v))
                    .y
                    .clamp(plot_rect.top() + 12.0, plot_rect.bottom() - 2.0);
                painter.text(
                    egui::pos2(plot_rect.right() - 4.0, y_pos - 2.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{v:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.ref_color,
                );
            }
        }

        // Cursor labels
        if data.cursors_active {
            if let Some(t) = data.cursor_a {
                let y_val = data.cursor_va.unwrap_or(0.0);
                let pos = transform.position_from_point(&egui_plot::PlotPoint::new(t, y_val));
                painter.text(
                    egui::pos2(pos.x + 4.0, pos.y - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("A: {t:.2} s / {y_val:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.cursor_color,
                );
            }
            if let Some(t) = data.cursor_b {
                let y_val = data.cursor_vb.unwrap_or(0.0);
                let pos = transform.position_from_point(&egui_plot::PlotPoint::new(t, y_val));
                painter.text(
                    egui::pos2(pos.x + 4.0, pos.y - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("B: {t:.2} s / {y_val:.4} {}", data.overlay_unit),
                    label_font.clone(),
                    data.cursor_color,
                );
            }
        }
    }
}
