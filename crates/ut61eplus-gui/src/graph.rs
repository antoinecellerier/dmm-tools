use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{Line, Plot, PlotBounds, PlotPoints, VLine};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of points to keep in the history buffer.
const MAX_POINTS: usize = 10_000;

/// Gap threshold: if two consecutive points are more than this apart,
/// we consider it a disconnect gap and break the line.
const GAP_THRESHOLD_SECS: f64 = 5.0;

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
    (10.0, "10s"),
    (30.0, "30s"),
    (60.0, "1m"),
    (300.0, "5m"),
    (600.0, "10m"),
];

/// Real-time scrolling graph with minimap navigation.
pub struct Graph {
    history: VecDeque<DataPoint>,
    current_mode: Option<String>,
    origin: Option<Instant>,
    /// Time window width in seconds for the main view.
    pub time_window_secs: f64,
    /// When true, main graph auto-scrolls to latest data.
    pub live: bool,
    /// User-controlled view center (seconds from origin). Used when not live.
    view_center: f64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_POINTS),
            current_mode: None,
            origin: None,
            time_window_secs: 60.0,
            live: true,
            view_center: 0.0,
        }
    }

    pub fn push(&mut self, value: f64, mode: &str) {
        let now = Instant::now();

        if self.origin.is_none() {
            self.origin = Some(now);
        }

        if self.current_mode.as_deref() != Some(mode) {
            self.history.clear();
            self.current_mode = Some(mode.to_string());
            self.origin = Some(now);
        }

        if self.history.len() >= MAX_POINTS {
            self.history.pop_front();
        }

        self.history.push_back(DataPoint { time: now, value });
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.current_mode = None;
        self.origin = None;
        self.live = true;
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
                if gap > GAP_THRESHOLD_SECS && !current_segment.is_empty() {
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
                if gap > GAP_THRESHOLD_SECS {
                    let t1 = self.elapsed_secs(p.time);
                    let t2 = self.elapsed_secs(point.time);
                    gaps.push((t1, t2));
                }
            }
            prev = Some(point);
        }

        gaps
    }

    /// Render toolbar (time window buttons + live toggle).
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
        });
    }

    /// Compute Y range from data points visible in the given X range, with padding.
    fn y_range_for_view(&self, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
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

        let mut plot = Plot::new("main_plot")
            .height(ui.available_height().max(60.0))
            .allow_drag(Vec2b::new(can_interact, false))
            .allow_zoom(Vec2b::new(can_interact, false))
            .allow_scroll(Vec2b::new(false, false))
            .allow_double_click_reset(false)
            .auto_bounds(Vec2b::new(false, false))
            .include_x(view_min)
            .include_x(view_max)
            .x_axis_label("time (s)");

        if let Some((y_min, y_max)) = y_bounds {
            plot = plot.include_y(y_min).include_y(y_max);
        }

        // Reset plot memory in live mode so it doesn't fight our bounds
        if self.live {
            plot = plot.reset();
        }

        let response = plot.show(ui, |plot_ui| {
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

        // If user dragged/zoomed, capture the new view
        if can_interact {
            let bounds = response.transform.bounds();
            self.view_center = (bounds.min()[0] + bounds.max()[0]) / 2.0;
            self.time_window_secs = bounds.max()[0] - bounds.min()[0];
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
        let viewport_color = egui::Color32::from_rgba_premultiplied(100, 150, 255, 80);

        let response = Plot::new("minimap_plot")
            .height(MINIMAP_HEIGHT)
            .include_x(data_min)
            .include_x(data_max)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .auto_bounds(Vec2b::new(false, true))
            .show_axes(Vec2b::new(true, false))
            .show_grid(false)
            .reset()
            .show(ui, |plot_ui| {
                for seg in &raw_segments {
                    plot_ui.line(
                        Line::new(PlotPoints::new(seg.clone())).color(line_color),
                    );
                }

                // Viewport indicator as two vertical lines
                plot_ui.vline(VLine::new(view_min).color(viewport_color).width(2.0));
                plot_ui.vline(VLine::new(view_max).color(viewport_color).width(2.0));
            });

        // Click or drag on minimap to navigate
        if response.response.dragged() || response.response.clicked() {
            if let Some(pos) = response.response.interact_pointer_pos() {
                let clicked_point = response.transform.value_from_position(pos);
                let half = self.time_window_secs / 2.0;
                // If clicked area would include the latest data point, go live
                if clicked_point.x + half >= data_max {
                    self.live = true;
                } else {
                    self.view_center = clicked_point.x;
                    self.live = false;
                }
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
        g.push(5.0, "DC V");
        assert_eq!(g.len(), 1);
        assert!(!g.is_empty());
        assert!(g.origin.is_some());
    }

    #[test]
    fn mode_change_clears_history() {
        let mut g = Graph::new();
        g.push(5.0, "DC V");
        g.push(5.1, "DC V");
        assert_eq!(g.len(), 2);
        g.push(100.0, "Ohm");
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn max_points_evicts_oldest() {
        let mut g = Graph::new();
        for i in 0..MAX_POINTS + 100 {
            g.push(i as f64, "DC V");
        }
        assert_eq!(g.len(), MAX_POINTS);
    }

    #[test]
    fn clear_resets_everything() {
        let mut g = Graph::new();
        g.push(5.0, "DC V");
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
        g.push(1.0, "DC V");
        g.push(2.0, "DC V");
        g.push(3.0, "DC V");
        let segments = g.build_raw_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn gap_detection() {
        let mut g = Graph::new();
        g.push(1.0, "DC V");
        g.push(2.0, "DC V");
        let gaps = g.find_gap_ranges();
        assert!(gaps.is_empty());
    }

    #[test]
    fn elapsed_secs_relative_to_origin() {
        let mut g = Graph::new();
        g.push(1.0, "DC V");
        sleep(Duration::from_millis(50));
        g.push(2.0, "DC V");
        let t = g.elapsed_secs(g.history.back().unwrap().time);
        assert!(t >= 0.04);
    }

    #[test]
    fn live_view_bounds_follow_latest() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, "DC V");
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
        assert_eq!(TIME_WINDOWS[0].1, "10s");
    }
}
