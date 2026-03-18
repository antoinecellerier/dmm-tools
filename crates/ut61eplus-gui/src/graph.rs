use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{AxisHints, Line, Plot, PlotBounds, PlotPoints, VLine};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of points to keep in the history buffer.
const MAX_POINTS: usize = 10_000;

/// Default gap threshold multiplier: gap = max(interval * multiplier, minimum).
const GAP_MULTIPLIER: f64 = 5.0;
const GAP_MINIMUM_SECS: f64 = 1.0;

/// Minimap height in logical pixels.
const MINIMAP_HEIGHT: f32 = 60.0;

/// A data point with an absolute timestamp.
#[derive(Clone, Copy)]
struct DataPoint {
    time: Instant,
    value: f64,
}

/// Time window presets.
pub const TIME_WINDOWS: &[(f64, &str)] = &[
    (5.0, "5s"),
    (10.0, "10s"),
    (30.0, "30s"),
    (60.0, "1m"),
    (300.0, "5m"),
    (600.0, "10m"),
];

/// Choose a nice round interval for time axis labels.
fn nice_time_interval(span: f64) -> f64 {
    let target_ticks = 6.0;
    let raw = span / target_ticks;
    let nice_values = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];
    for &v in &nice_values {
        if v >= raw {
            return v;
        }
    }
    raw.ceil()
}

/// Format a time value in seconds as a readable label.
fn format_time_label(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        let m = (secs / 60.0).floor() as u32;
        let s = (secs % 60.0).floor() as u32;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m{s:02}s")
        }
    } else {
        let h = (secs / 3600.0).floor() as u32;
        let m = ((secs % 3600.0) / 60.0).floor() as u32;
        format!("{h}h{m:02}m")
    }
}

/// Real-time scrolling graph with minimap navigation.
pub struct Graph {
    history: VecDeque<DataPoint>,
    current_mode: Option<String>,
    current_unit: String,
    origin: Option<Instant>,
    /// Time window width in seconds for the main view.
    pub time_window_secs: f64,
    /// When true, main graph auto-scrolls to latest data.
    pub live: bool,
    /// User-controlled view center (seconds from origin). Used when not live.
    view_center: f64,
    /// Gap detection threshold in seconds.
    gap_threshold_secs: f64,
    /// When true, Y axis uses fixed min/max instead of auto-scaling.
    pub y_axis_fixed: bool,
    /// Fixed Y-axis minimum (editable text buffer for UI).
    y_min_text: String,
    /// Fixed Y-axis maximum (editable text buffer for UI).
    y_max_text: String,
    /// Parsed fixed Y-axis min.
    y_fixed_min: f64,
    /// Parsed fixed Y-axis max.
    y_fixed_max: f64,
    /// Whether the user has manually set Y-axis values this session.
    y_user_set: bool,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_POINTS),
            current_mode: None,
            current_unit: String::new(),
            origin: None,
            time_window_secs: 60.0,
            live: true,
            view_center: 0.0,
            gap_threshold_secs: GAP_MINIMUM_SECS,
            y_axis_fixed: false,
            y_min_text: "-1".to_string(),
            y_max_text: "1".to_string(),
            y_fixed_min: -1.0,
            y_fixed_max: 1.0,
            y_user_set: false,
        }
    }

    /// Update gap detection threshold based on sample interval.
    pub fn set_sample_interval_ms(&mut self, ms: u32) {
        let interval_secs = (ms as f64 / 1000.0).max(0.1); // 0ms → use ~100ms wire time
        self.gap_threshold_secs = (interval_secs * GAP_MULTIPLIER).max(GAP_MINIMUM_SECS);
    }

    pub fn push(&mut self, value: f64, mode: &str, unit: &str) {
        let now = Instant::now();

        if self.origin.is_none() {
            self.origin = Some(now);
        }

        if self.current_mode.as_deref() != Some(mode) {
            self.history.clear();
            self.current_mode = Some(mode.to_string());
            self.origin = Some(now);
            self.live = true;
            self.view_center = 0.0;
        }
        self.current_unit = unit.to_string();

        if self.history.len() >= MAX_POINTS {
            self.history.pop_front();
        }

        self.history.push_back(DataPoint { time: now, value });
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.current_mode = None;
        self.current_unit.clear();
        self.origin = None;
        self.live = true;
        self.view_center = 0.0;
        self.y_axis_fixed = false;
        self.y_user_set = false;
    }

    fn elapsed_secs(&self, t: Instant) -> f64 {
        match self.origin {
            Some(origin) => t.duration_since(origin).as_secs_f64(),
            None => 0.0,
        }
    }

    fn data_time_range(&self) -> (f64, f64) {
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
        (x_min, x_max)
    }

    /// Current view bounds (x_min, x_max) for the main graph.
    fn view_bounds(&self) -> (f64, f64) {
        let (_, data_max) = self.data_time_range();
        let half = self.time_window_secs / 2.0;

        if self.live {
            let x_max = data_max;
            let x_min = (x_max - self.time_window_secs).max(0.0);
            (x_min, x_max)
        } else {
            let x_min = (self.view_center - half).max(0.0);
            let x_max = x_min + self.time_window_secs;
            (x_min, x_max)
        }
    }

    /// Build raw segment data as Vec<Vec<[f64;2]>> — avoids PlotPoints clone issues.
    fn build_raw_segments(&self) -> Vec<Vec<[f64; 2]>> {
        let mut segments: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut current_segment: Vec<[f64; 2]> = Vec::new();
        let mut prev_time: Option<Instant> = None;

        for point in &self.history {
            let t = self.elapsed_secs(point.time);

            if let Some(prev) = prev_time {
                let gap = point.time.duration_since(prev).as_secs_f64();
                if gap > self.gap_threshold_secs && !current_segment.is_empty() {
                    segments.push(std::mem::take(&mut current_segment));
                }
            }

            current_segment.push([t, point.value]);
            prev_time = Some(point.time);
        }

        if !current_segment.is_empty() {
            segments.push(current_segment);
        }

        segments
    }

    fn find_gap_ranges(&self) -> Vec<(f64, f64)> {
        let mut gaps = Vec::new();
        let mut prev: Option<&DataPoint> = None;

        for point in &self.history {
            if let Some(p) = prev {
                let gap = point.time.duration_since(p.time).as_secs_f64();
                if gap > self.gap_threshold_secs {
                    let t1 = self.elapsed_secs(p.time);
                    let t2 = self.elapsed_secs(point.time);
                    gaps.push((t1, t2));
                }
            }
            prev = Some(point);
        }

        gaps
    }

    /// Render toolbar (time window buttons + live toggle + Y-axis controls).
    pub fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for &(secs, label) in TIME_WINDOWS {
                if ui
                    .selectable_label(
                        (self.time_window_secs - secs).abs() < 0.1,
                        label,
                    )
                    .clicked()
                {
                    self.time_window_secs = secs;
                }
            }

            ui.separator();

            let live_color = if self.live {
                egui::Color32::from_rgb(60, 180, 75)
            } else {
                ui.visuals().weak_text_color()
            };
            if ui
                .add(egui::Button::new(
                    egui::RichText::new("LIVE").color(live_color).small(),
                ))
                .clicked()
            {
                self.live = !self.live;
            }

            ui.separator();

            // Y-axis mode toggle
            let y_label = if self.y_axis_fixed { "Y:Fixed" } else { "Y:Auto" };
            if ui.selectable_label(self.y_axis_fixed, y_label).clicked() {
                if !self.y_axis_fixed && !self.y_user_set {
                    // Switching to fixed: snapshot current auto-scaled range
                    let (view_min, view_max) = self.view_bounds();
                    if let Some((y_lo, y_hi)) = self.y_range_for_view_auto(view_min, view_max) {
                        self.y_fixed_min = y_lo;
                        self.y_fixed_max = y_hi;
                        self.y_min_text = format!("{y_lo:.4}");
                        self.y_max_text = format!("{y_hi:.4}");
                    }
                }
                self.y_axis_fixed = !self.y_axis_fixed;
            }

            if self.y_axis_fixed {
                let field_width = 50.0;
                let changed_min = ui
                    .add(egui::TextEdit::singleline(&mut self.y_min_text).desired_width(field_width))
                    .changed();
                ui.label(
                    egui::RichText::new("..").small().color(ui.visuals().weak_text_color()),
                );
                let changed_max = ui
                    .add(egui::TextEdit::singleline(&mut self.y_max_text).desired_width(field_width))
                    .changed();

                if changed_min {
                    if let Ok(v) = self.y_min_text.parse::<f64>() {
                        self.y_fixed_min = v;
                        self.y_user_set = true;
                    }
                }
                if changed_max {
                    if let Ok(v) = self.y_max_text.parse::<f64>() {
                        self.y_fixed_max = v;
                        self.y_user_set = true;
                    }
                }
            }
        });
    }

    /// Auto-scaled Y range (ignoring fixed mode setting). Used to snapshot
    /// current auto range when switching to fixed mode.
    fn y_range_for_view_auto(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t >= x_min && t <= x_max {
                y_min = y_min.min(point.value);
                y_max = y_max.max(point.value);
            }
        }
        if y_min.is_infinite() {
            return None;
        }
        let range = (y_max - y_min).max(1e-6);
        let pad = range * 0.1;
        Some((y_min - pad, y_max + pad))
    }

    /// Compute Y range from data points visible in the given X range, with padding.
    fn y_range_for_view(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
        if self.y_axis_fixed {
            return Some((self.y_fixed_min, self.y_fixed_max));
        }
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t >= x_min && t <= x_max {
                y_min = y_min.min(point.value);
                y_max = y_max.max(point.value);
            }
        }
        if y_min.is_infinite() {
            return None;
        }
        // Add 10% padding
        let range = (y_max - y_min).max(1e-6);
        let pad = range * 0.1;
        Some((y_min - pad, y_max + pad))
    }

    /// Render the main graph.
    pub fn show_main(&mut self, ui: &mut Ui) {
        let raw_segments = self.build_raw_segments();
        let gap_ranges = self.find_gap_ranges();
        let (view_min, view_max) = self.view_bounds();

        let line_color = egui::Color32::from_rgb(200, 100, 100);
        let gap_color = egui::Color32::from_rgba_premultiplied(150, 60, 60, 120);

        let can_interact = !self.live;

        // Compute Y bounds from visible data
        let y_bounds = self.y_range_for_view(view_min, view_max);

        let (y_min, y_max) = y_bounds.unwrap_or((-1.0, 1.0));

        let unit = self.current_unit.clone();
        let y_axis = AxisHints::new_y()
            .formatter(move |mark, _range| {
                let decimals = (-mark.step_size.log10().round() as usize).min(6);
                let val = eframe::emath::format_with_decimals_in_range(
                    mark.value,
                    decimals..=decimals,
                );
                if unit.is_empty() {
                    val
                } else {
                    format!("{val} {unit}  ")
                }
            });

        let x_axis = AxisHints::new_x()
            .formatter(|mark, _range| {
                let s = mark.value;
                if s < 60.0 {
                    format!("{s:.0} s")
                } else if s < 3600.0 {
                    let m = (s / 60.0).floor();
                    let sec = s % 60.0;
                    if sec.abs() < 0.5 {
                        format!("{m:.0} m")
                    } else {
                        format!("{m:.0}m {sec:.0}s")
                    }
                } else {
                    let h = (s / 3600.0).floor();
                    let m = ((s % 3600.0) / 60.0).floor();
                    format!("{h:.0}h {m:.0}m")
                }
            });

        let cursor_unit = self.current_unit.clone();
        let plot = Plot::new("main_plot")
            .height(ui.available_height().max(60.0))
            .allow_drag(Vec2b::new(can_interact, false))
            .allow_zoom(Vec2b::new(can_interact, false))
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .reset()
            .custom_x_axes(vec![x_axis])
            .custom_y_axes(vec![y_axis])
            .y_axis_min_width(60.0)
            .label_formatter(move |_name, point| {
                let t = point.x;
                let time_label = if t < 60.0 {
                    format!("{t:.1} s")
                } else {
                    let m = (t / 60.0).floor();
                    let s = t % 60.0;
                    format!("{m:.0}m {s:.1}s")
                };
                format!("{time_label}\n{:.4} {}", point.y, cursor_unit)
            });

        let response = plot.show(ui, |plot_ui| {
                // Set exact bounds: our X view range + computed Y range
                plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                    [view_min, y_min],
                    [view_max, y_max],
                ));

                for seg in &raw_segments {
                    plot_ui.line(
                        Line::new(PlotPoints::new(seg.clone())).color(line_color),
                    );
                }

                for &(gap_start, gap_end) in &gap_ranges {
                    plot_ui.vline(
                        VLine::new(gap_start)
                            .color(gap_color)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                    plot_ui.vline(
                        VLine::new(gap_end)
                            .color(gap_color)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            });

        // Handle drag: convert pixel delta to time delta
        if can_interact && response.response.dragged() {
            let drag_px = response.response.drag_delta().x;
            // Convert pixel drag to time using the transform
            let left = response.transform.value_from_position(
                response.response.rect.left_top(),
            );
            let right = response.transform.value_from_position(
                response.response.rect.right_top(),
            );
            let px_per_sec = response.response.rect.width() as f64
                / (right.x - left.x).max(1e-6);
            let time_delta = drag_px as f64 / px_per_sec;
            self.view_center -= time_delta;
        }

        // Handle scroll wheel zoom on X axis — zoom centered on cursor position
        if can_interact {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                // Find cursor X position in time coordinates for centered zoom
                if let Some(hover_pos) = response.response.hover_pos() {
                    let cursor_t = response.transform.value_from_position(hover_pos).x;
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

        // Double-click to return to live mode
        if response.response.double_clicked() {
            self.live = true;
        }
    }

    /// Render the minimap showing full history with viewport indicator.
    pub fn show_minimap(&mut self, ui: &mut Ui) {
        if self.history.len() < 2 {
            ui.allocate_space(egui::vec2(ui.available_width(), MINIMAP_HEIGHT));
            return;
        }

        let raw_segments = self.build_raw_segments();
        let (data_min, data_max) = self.data_time_range();
        let (view_min, view_max) = self.view_bounds();

        let line_color = egui::Color32::from_rgba_premultiplied(200, 100, 100, 150);

        // Allocate rect for minimap + label space below, with margin for bracket strokes
        let label_height = 14.0;
        let margin = 4.0; // room for bracket strokes at edges
        let total_height = MINIMAP_HEIGHT + label_height + margin * 2.0;
        let (full_rect, pointer_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), total_height),
            egui::Sense::click_and_drag(),
        );
        // Inset the plot area so brackets at edges have room to render
        let rect = egui::Rect::from_min_size(
            egui::pos2(full_rect.left() + margin, full_rect.top() + margin),
            egui::vec2(full_rect.width() - margin * 2.0, MINIMAP_HEIGHT),
        );

        // Use full_rect painter so nothing gets clipped
        let painter = ui.painter_at(full_rect);
        let data_span = (data_max - data_min).max(1e-6);

        let time_to_x = |t: f64| -> f32 {
            rect.left() + ((t - data_min) / data_span) as f32 * rect.width()
        };

        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        // Draw data lines
        for seg in &raw_segments {
            let points: Vec<egui::Pos2> = seg
                .iter()
                .map(|&[t, v]| {
                    let x = time_to_x(t);
                    // Simple Y mapping: find Y range from all data
                    let y_frac = if let Some((y_lo, y_hi)) = self.y_range_for_view(data_min, data_max) {
                        let range = (y_hi - y_lo).max(1e-10);
                        ((v - y_lo) / range) as f32
                    } else {
                        0.5
                    };
                    let y = rect.bottom() - y_frac * rect.height();
                    egui::pos2(x, y)
                })
                .collect();
            for window in points.windows(2) {
                painter.line_segment(
                    [window[0], window[1]],
                    egui::Stroke::new(1.0, line_color),
                );
            }
        }

        // Draw viewport indicator as [ ] bracket markers
        let vp_left = time_to_x(view_min);
        let vp_right = time_to_x(view_max);
        let vp_color = egui::Color32::from_rgb(100, 150, 255);
        let vp_stroke = egui::Stroke::new(2.5, vp_color);
        let bracket_w = 4.0_f32; // horizontal arm of the bracket

        // Left bracket [
        painter.line_segment([egui::pos2(vp_left, rect.top()), egui::pos2(vp_left, rect.bottom())], vp_stroke);
        painter.line_segment([egui::pos2(vp_left, rect.top()), egui::pos2(vp_left + bracket_w, rect.top())], vp_stroke);
        painter.line_segment([egui::pos2(vp_left, rect.bottom()), egui::pos2(vp_left + bracket_w, rect.bottom())], vp_stroke);

        // Right bracket ]
        painter.line_segment([egui::pos2(vp_right, rect.top()), egui::pos2(vp_right, rect.bottom())], vp_stroke);
        painter.line_segment([egui::pos2(vp_right, rect.top()), egui::pos2(vp_right - bracket_w, rect.top())], vp_stroke);
        painter.line_segment([egui::pos2(vp_right, rect.bottom()), egui::pos2(vp_right - bracket_w, rect.bottom())], vp_stroke);

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
                egui::FontId::proportional(9.0),
                label_color,
            );
            // Small tick mark
            painter.line_segment(
                [egui::pos2(x, rect.bottom() - 2.0), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, label_color),
            );
            t += nice_interval;
        }

        // Handle click/drag navigation
        if let Some(pos) = pointer_response.interact_pointer_pos() {
            let clicked_t = data_min + ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * data_span;
            let half = self.time_window_secs / 2.0;
            if clicked_t + half >= data_max {
                self.live = true;
            } else {
                self.view_center = clicked_t;
                self.live = false;
            }
        }
    }

    /// Combined render: toolbar + main graph + minimap.
    pub fn show(&mut self, ui: &mut Ui, _time_window_secs: f64) {
        self.show_toolbar(ui);
        let minimap_reserve = MINIMAP_HEIGHT + 30.0;
        let main_height = (ui.available_height() - minimap_reserve).max(60.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), main_height), |ui| {
            self.show_main(ui);
        });
        ui.add_space(4.0);
        self.show_minimap(ui);
    }

    /// Compute min/max/avg/count for data points visible in the current view.
    pub fn visible_stats(&self) -> Option<(f64, f64, f64, usize)> {
        let (x_min, x_max) = self.view_bounds();
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let mut count = 0usize;
        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t >= x_min && t <= x_max {
                min = min.min(point.value);
                max = max.max(point.value);
                sum += point.value;
                count += 1;
            }
        }
        if count > 0 {
            Some((min, max, sum / count as f64, count))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn new_graph_is_empty() {
        let g = Graph::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(g.live);
    }

    #[test]
    fn push_adds_point() {
        let mut g = Graph::new();
        g.push(5.0, "DC V", "V");
        assert_eq!(g.len(), 1);
        assert!(!g.is_empty());
        assert!(g.origin.is_some());
    }

    #[test]
    fn mode_change_clears_history() {
        let mut g = Graph::new();
        g.push(5.0, "DC V", "V");
        g.push(5.1, "DC V", "V");
        assert_eq!(g.len(), 2);
        g.push(100.0, "Ohm", "Ω");
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn max_points_evicts_oldest() {
        let mut g = Graph::new();
        for i in 0..MAX_POINTS + 100 {
            g.push(i as f64, "DC V", "V");
        }
        assert_eq!(g.len(), MAX_POINTS);
    }

    #[test]
    fn clear_resets_everything() {
        let mut g = Graph::new();
        g.push(5.0, "DC V", "V");
        g.live = false;
        g.clear();
        assert!(g.is_empty());
        assert_eq!(g.current_mode, None);
        assert!(g.origin.is_none());
        assert!(g.live);
    }

    #[test]
    fn segments_without_gaps() {
        let mut g = Graph::new();
        g.push(1.0, "DC V", "V");
        g.push(2.0, "DC V", "V");
        g.push(3.0, "DC V", "V");
        let segments = g.build_raw_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn gap_detection() {
        let mut g = Graph::new();
        g.push(1.0, "DC V", "V");
        g.push(2.0, "DC V", "V");
        let gaps = g.find_gap_ranges();
        assert!(gaps.is_empty());
    }

    #[test]
    fn elapsed_secs_relative_to_origin() {
        let mut g = Graph::new();
        g.push(1.0, "DC V", "V");
        sleep(Duration::from_millis(50));
        g.push(2.0, "DC V", "V");
        let t = g.elapsed_secs(g.history.back().unwrap().time);
        assert!(t >= 0.04);
    }

    #[test]
    fn live_view_bounds_follow_latest() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, "DC V", "V");
        let (vmin, vmax) = g.view_bounds();
        assert!(vmin >= 0.0);
        assert!(vmax >= vmin);
    }

    #[test]
    fn manual_view_bounds() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = 50.0;
        let (vmin, vmax) = g.view_bounds();
        assert!((vmin - 45.0).abs() < 0.1);
        assert!((vmax - 55.0).abs() < 0.1);
    }

    #[test]
    fn time_window_presets_exist() {
        assert!(TIME_WINDOWS.len() >= 3);
        assert_eq!(TIME_WINDOWS[0].1, "5s");
    }
}
