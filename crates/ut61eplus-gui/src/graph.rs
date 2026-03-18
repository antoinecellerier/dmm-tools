use eframe::egui::{self, Ui};
use egui_plot::{Line, Plot, PlotPoints, VLine};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of points to keep in the history buffer.
const MAX_POINTS: usize = 10_000;

/// Gap threshold: if two consecutive points are more than this apart,
/// we consider it a disconnect gap and break the line.
const GAP_THRESHOLD_SECS: f64 = 5.0;

/// A data point with an absolute timestamp.
#[derive(Clone, Copy)]
struct DataPoint {
    time: Instant,
    value: f64,
}

/// Real-time scrolling graph of measurement values.
/// Uses absolute timestamps so data persists across reconnects.
pub struct Graph {
    history: VecDeque<DataPoint>,
    /// The mode string when points were recorded. Cleared on mode change.
    current_mode: Option<String>,
    /// The time origin — first data point ever pushed.
    origin: Option<Instant>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_POINTS),
            current_mode: None,
            origin: None,
        }
    }

    /// Push a new data point. If mode changed, clear history.
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
    }

    /// Convert an Instant to seconds-from-origin for display.
    fn elapsed_secs(&self, t: Instant) -> f64 {
        match self.origin {
            Some(origin) => t.duration_since(origin).as_secs_f64(),
            None => 0.0,
        }
    }

    /// Render the plot in the given UI region.
    pub fn show(&self, ui: &mut Ui, time_window_secs: f64) {
        // Split history into segments separated by gaps (disconnects).
        // Each segment becomes its own Line so gaps show as breaks.
        let segments = self.build_segments();

        let x_max = self
            .history
            .back()
            .map(|p| self.elapsed_secs(p.time))
            .unwrap_or(0.0);
        let x_min = (x_max - time_window_secs).max(0.0);

        // Find gap boundaries for vertical markers
        let gap_times = self.find_gap_times();

        Plot::new("measurement_plot")
            .height(ui.available_height().max(80.0))
            .include_x(x_min)
            .include_x(x_max)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .x_axis_label("time (s)")
            .show(ui, |plot_ui| {
                for segment in segments {
                    plot_ui.line(Line::new(segment));
                }
                // Draw subtle vertical lines at disconnect points
                for gap_t in gap_times {
                    plot_ui.vline(
                        VLine::new(gap_t)
                            .color(egui::Color32::from_rgba_premultiplied(150, 80, 80, 100))
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            });
    }

    /// Build line segments, breaking at gaps.
    fn build_segments(&self) -> Vec<PlotPoints> {
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

        segments.into_iter().map(PlotPoints::new).collect()
    }

    /// Find the midpoint times of gaps for drawing disconnect markers.
    fn find_gap_times(&self) -> Vec<f64> {
        let mut gaps = Vec::new();
        let mut prev: Option<&DataPoint> = None;

        for point in &self.history {
            if let Some(p) = prev {
                let gap = point.time.duration_since(p.time).as_secs_f64();
                if gap > GAP_THRESHOLD_SECS {
                    // Place marker at the midpoint of the gap
                    let t1 = self.elapsed_secs(p.time);
                    let t2 = self.elapsed_secs(point.time);
                    gaps.push((t1 + t2) / 2.0);
                }
            }
            prev = Some(point);
        }

        gaps
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
        assert_eq!(g.len(), 1); // cleared + new point
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
        g.clear();
        assert!(g.is_empty());
        assert_eq!(g.current_mode, None);
        assert!(g.origin.is_none());
    }

    #[test]
    fn segments_without_gaps() {
        let mut g = Graph::new();
        g.push(1.0, "DC V");
        g.push(2.0, "DC V");
        g.push(3.0, "DC V");
        let segments = g.build_segments();
        assert_eq!(segments.len(), 1); // all one segment
    }

    #[test]
    fn gap_detection() {
        let mut g = Graph::new();
        g.push(1.0, "DC V");
        // Simulate a gap by inserting a point with a manually offset time
        // We can't easily fake Instant, but we can test that consecutive
        // points without real delay produce one segment
        g.push(2.0, "DC V");
        let gaps = g.find_gap_times();
        assert!(gaps.is_empty()); // no real gap since pushed back-to-back
    }

    #[test]
    fn elapsed_secs_relative_to_origin() {
        let mut g = Graph::new();
        g.push(1.0, "DC V");
        sleep(Duration::from_millis(50));
        g.push(2.0, "DC V");
        let t = g.elapsed_secs(g.history.back().unwrap().time);
        assert!(t >= 0.04); // at least 40ms
    }
}
