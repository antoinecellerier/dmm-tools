use eframe::egui::{self, Ui, Vec2b};
use egui_plot::{
    AxisHints, HLine, Line, Plot, PlotBounds, PlotPoints, PlotTransform, Points, VLine,
};
use std::collections::VecDeque;
use std::time::Instant;

use crate::settings::{ColorOverrides, ColorPreset};
use crate::theme::ThemeColors;

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

/// Format a grid-mark value for the main graph's X axis, adding decimals to
/// the seconds field when the grid step is sub-second. Without this, a tight
/// zoom (e.g. a 0.5 s span) produces duplicate labels like "9 s" / "9 s"
/// because integer seconds can't distinguish adjacent gridlines.
fn format_time_axis_label(value: f64, step_size: f64) -> String {
    // When step is a sub-second power of 10 (egui_plot default log-10
    // spacer), show enough decimals to resolve adjacent marks. clamp to 1
    // so a rounding step like 0.5 (non-power-of-10) still gets at least
    // one decimal of precision.
    let sec_decimals: usize = if step_size > 0.0 && step_size < 1.0 {
        ((-step_size.log10()).round() as i64).clamp(1, 6) as usize
    } else {
        0
    };

    let s = value;
    if s < 60.0 {
        format!("{s:.sec_decimals$} s")
    } else if s < 3600.0 {
        let m = (s / 60.0).floor();
        let sec = s - m * 60.0;
        if sec_decimals == 0 && sec.abs() < 0.5 {
            format!("{m:.0} m")
        } else {
            format!("{m:.0}m {sec:.sec_decimals$}s")
        }
    } else {
        let h = (s / 3600.0).floor();
        let rem = s - h * 3600.0;
        let m = (rem / 60.0).floor();
        let sec = rem - m * 60.0;
        if sec_decimals > 0 {
            format!("{h:.0}h {m:.0}m {sec:.sec_decimals$}s")
        } else {
            format!("{h:.0}h {m:.0}m")
        }
    }
}

/// Tracks which part of the minimap the user is dragging.
#[derive(Default, Clone, Copy, PartialEq)]
enum MinimapDrag {
    #[default]
    None,
    Pan,
    ResizeLeft,
    ResizeRight,
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
    /// Show mean line overlay.
    pub show_mean: bool,
    /// Show min/max envelope band.
    pub show_envelope: bool,
    /// Envelope bucket width in seconds (user-configurable).
    envelope_window_text: String,
    envelope_window_secs: f64,
    /// Reference lines: show horizontal lines at these values.
    pub show_ref_line: bool,
    /// Show trigger crossing markers on reference lines.
    pub show_crossings: bool,
    ref_line_text: String,
    ref_line_values: Vec<f64>,
    /// Measurement cursors: two vertical lines with ΔT/ΔV readout.
    pub cursors_active: bool,
    /// Cursor positions in seconds from origin. None = not yet placed.
    cursor_a: Option<f64>,
    cursor_b: Option<f64>,
    /// Which cursor to place next on click.
    cursor_next_is_b: bool,
    /// Cached segment data, rebuilt only when history changes.
    cached_segments: Vec<Vec<[f64; 2]>>,
    cached_gaps: Vec<(f64, f64)>,
    /// Number of history entries when cache was built.
    cache_len: usize,
    /// Color preset for graph rendering.
    color_preset: ColorPreset,
    /// Per-theme color overrides for graph rendering.
    color_overrides: ColorOverrides,
    /// Current minimap drag state.
    minimap_drag: MinimapDrag,
    /// Press origin (screen pixels) when a Shift+drag bbox-zoom is in progress.
    bbox_zoom_start_px: Option<egui::Pos2>,
    /// Latest pointer position during an in-progress bbox-zoom drag. Tracked
    /// separately so the release frame still has a valid endpoint even when
    /// hover_pos()/interact_pos() momentarily return None.
    bbox_zoom_current_px: Option<egui::Pos2>,
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
            show_mean: false,
            show_envelope: false,
            envelope_window_text: "1".to_string(),
            envelope_window_secs: 1.0,
            show_ref_line: false,
            show_crossings: true,
            ref_line_text: String::new(),
            ref_line_values: Vec::new(),
            cursors_active: false,
            cursor_a: None,
            cursor_b: None,
            cursor_next_is_b: false,
            cached_segments: Vec::new(),
            cached_gaps: Vec::new(),
            cache_len: 0,
            color_preset: ColorPreset::Default,
            color_overrides: ColorOverrides::default(),
            minimap_drag: MinimapDrag::None,
            bbox_zoom_start_px: None,
            bbox_zoom_current_px: None,
        }
    }

    /// Update color configuration from settings.
    pub fn set_color_config(&mut self, preset: ColorPreset, overrides: ColorOverrides) {
        self.color_preset = preset;
        self.color_overrides = overrides;
    }

    /// Build a ThemeColors instance using this graph's color config.
    fn theme_colors(&self, dark: bool) -> ThemeColors {
        ThemeColors::new(dark, self.color_preset, self.color_overrides.for_mode(dark))
    }

    /// Update gap detection threshold based on sample interval.
    pub fn set_sample_interval_ms(&mut self, ms: u32) {
        let interval_secs = (ms as f64 / 1000.0).max(0.1); // 0ms → use ~100ms wire time
        self.gap_threshold_secs = (interval_secs * GAP_MULTIPLIER).max(GAP_MINIMUM_SECS);
    }

    pub fn push(&mut self, value: f64, timestamp: Instant, mode: &str, unit: &str) {
        let now = timestamp;

        if self.origin.is_none() {
            self.origin = Some(now);
        }

        if self.current_mode.as_deref() != Some(mode) {
            self.history.clear();
            self.current_mode = Some(mode.to_string());
            self.origin = Some(now);
            self.live = true;
            self.view_center = 0.0;
            self.cursor_a = None;
            self.cursor_b = None;
            self.cursor_next_is_b = false;
            self.bbox_zoom_start_px = None;
            self.bbox_zoom_current_px = None;
            self.invalidate_cache();
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
        self.cursor_a = None;
        self.cursor_b = None;
        self.cursor_next_is_b = false;
        self.minimap_drag = MinimapDrag::None;
        self.bbox_zoom_start_px = None;
        self.bbox_zoom_current_px = None;
        self.invalidate_cache();
    }

    /// Handle keyboard shortcuts for graph navigation.
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }

        use egui::{Key, Modifiers};

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
    fn cycle_time_window(&mut self, direction: i32) {
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
    fn scroll_view(&mut self, fraction: f64) {
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
    fn jump_to_start(&mut self) {
        let (data_min, _) = self.data_time_range();
        self.view_center = data_min + self.time_window_secs / 2.0;
        self.live = false;
    }

    fn invalidate_cache(&mut self) {
        self.cache_len = 0;
        self.cached_segments.clear();
        self.cached_gaps.clear();
    }

    /// Rebuild cached segments/gaps only if history has changed.
    fn ensure_cache(&mut self) {
        if self.cache_len != self.history.len() {
            self.cached_segments = self.build_raw_segments();
            self.cached_gaps = self.find_gap_ranges();
            self.cache_len = self.history.len();
        }
    }

    fn elapsed_secs(&self, t: Instant) -> f64 {
        match self.origin {
            // Use checked_duration_since to avoid panic if clock goes backward
            // (can happen on VM suspend/resume or NTP adjustments).
            Some(origin) => t
                .checked_duration_since(origin)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
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
                let gap = point
                    .time
                    .checked_duration_since(prev)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
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
                let gap = point
                    .time
                    .checked_duration_since(p.time)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
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

    /// Render toolbar as two rows. Row 1: time presets, LIVE, Y-axis.
    /// Row 2: overlay toggles (Mean, Min/Max, Ref, Cursors).
    /// Both use `horizontal_wrapped` so items wrap instead of clipping.
    pub fn show_toolbar(&mut self, ui: &mut Ui) {
        // Row 1: time windows + LIVE + Y-axis
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for &(secs, label) in TIME_WINDOWS {
                let tooltip = format!("Show the last {label} of samples ([ / ] to cycle)");
                if ui
                    .selectable_label((self.time_window_secs - secs).abs() < 0.1, label)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.time_window_secs = secs;
                }
            }

            ui.add_space(6.0);

            let live_color = if self.live {
                self.theme_colors(ui.visuals().dark_mode).live_green()
            } else {
                ui.visuals().weak_text_color()
            };
            if ui
                .add(egui::Button::new(
                    egui::RichText::new("LIVE").color(live_color).small(),
                ))
                .on_hover_text(
                    "Auto-follow the newest samples — off while panning (End to jump back)",
                )
                .clicked()
            {
                self.live = !self.live;
            }

            ui.add_space(6.0);

            let (y_label, y_tooltip) = if self.y_axis_fixed {
                (
                    "Y:Fixed",
                    "Using fixed Y-axis bounds — click to auto-scale to visible data",
                )
            } else {
                (
                    "Y:Auto",
                    "Auto-scaling Y to visible data — click to enter fixed bounds",
                )
            };
            if ui
                .selectable_label(self.y_axis_fixed, y_label)
                .on_hover_text(y_tooltip)
                .clicked()
            {
                if !self.y_axis_fixed && !self.y_user_set {
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
                    .add(
                        egui::TextEdit::singleline(&mut self.y_min_text).desired_width(field_width),
                    )
                    .on_hover_text("Lower bound of the fixed Y axis")
                    .changed();
                ui.label(
                    egui::RichText::new("..")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                let changed_max = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.y_max_text).desired_width(field_width),
                    )
                    .on_hover_text("Upper bound of the fixed Y axis")
                    .changed();
                if changed_min && let Ok(v) = self.y_min_text.parse::<f64>() {
                    self.y_fixed_min = v;
                    self.y_user_set = true;
                }
                if changed_max && let Ok(v) = self.y_max_text.parse::<f64>() {
                    self.y_fixed_max = v;
                    self.y_user_set = true;
                }
            }

            ui.add_space(6.0);

            let zoomed = self.is_view_zoomed();
            if ui
                .add_enabled(zoomed, egui::Button::new("Reset Zoom"))
                .on_hover_text("Return to live follow and auto Y (double-click graph)")
                .on_disabled_hover_text("Already at the default live + auto-Y view")
                .clicked()
            {
                self.reset_view();
            }
        });

        // Row 2: overlay toggles
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let dark = ui.visuals().dark_mode;

            if ui
                .selectable_label(self.show_mean, "Mean")
                .on_hover_text("Draw a horizontal line at the mean of visible samples")
                .clicked()
            {
                self.show_mean = !self.show_mean;
            }
            if ui
                .selectable_label(self.show_envelope, "Min/Max")
                .on_hover_text("Draw a shaded band between the rolling min and max")
                .clicked()
            {
                self.show_envelope = !self.show_envelope;
            }
            if self.show_envelope {
                let changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.envelope_window_text)
                            .desired_width(30.0),
                    )
                    .on_hover_text("Window size (seconds) used to compute the Min/Max envelope")
                    .changed();
                if changed
                    && let Ok(v) = self.envelope_window_text.parse::<f64>()
                    && v > 0.0
                {
                    self.envelope_window_secs = v;
                }
                ui.label(
                    egui::RichText::new("s")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if ui
                .selectable_label(self.show_ref_line, "Ref")
                .on_hover_text("Draw horizontal reference lines at the values in the next field")
                .clicked()
            {
                self.show_ref_line = !self.show_ref_line;
            }
            if self.show_ref_line {
                let changed = ui
                    .add(egui::TextEdit::singleline(&mut self.ref_line_text).desired_width(80.0))
                    .on_hover_text(
                        "Reference values, comma- or semicolon-separated (e.g. 3.3, 5, 12)",
                    )
                    .changed();
                if changed {
                    self.ref_line_values = self
                        .ref_line_text
                        .split([',', ';', ' '])
                        .filter_map(|s| s.trim().parse::<f64>().ok())
                        .collect();
                }
                if ui
                    .selectable_label(self.show_crossings, "Triggers")
                    .on_hover_text("Mark the points where the signal crosses a reference line")
                    .clicked()
                {
                    self.show_crossings = !self.show_crossings;
                }
            }
            if ui
                .selectable_label(self.cursors_active, "Cursors")
                .on_hover_text("Click the graph to place two cursors and read Δt / Δv / integral")
                .clicked()
            {
                self.cursors_active = !self.cursors_active;
                if !self.cursors_active {
                    self.cursor_a = None;
                    self.cursor_b = None;
                    self.cursor_next_is_b = false;
                }
            }
            if self.cursors_active {
                if let (Some(ta), Some(tb)) = (self.cursor_a, self.cursor_b) {
                    let dt = (tb - ta).abs();
                    let va = self.nearest_point(ta).map(|(_, v)| v);
                    let vb = self.nearest_point(tb).map(|(_, v)| v);
                    let dv = match (va, vb) {
                        (Some(a), Some(b)) => format!("{:.4}", (b - a).abs()),
                        _ => crate::NO_DATA.to_string(),
                    };
                    let unit = &self.current_unit;
                    let delta_color = self.theme_colors(dark).graph_cursor_delta();

                    let integral_str = dmm_lib::stats::integral_unit_info(unit)
                        .and_then(|(disp_unit, divisor)| {
                            self.cursor_integral(ta, tb)
                                .map(|raw| format!("  \u{222b}={:.4} {disp_unit}", raw / divisor))
                        })
                        .unwrap_or_default();

                    ui.label(
                        egui::RichText::new(format!(
                            "\u{0394}T={dt:.2} s  \u{0394}={dv} {unit}{integral_str}"
                        ))
                        .color(delta_color)
                        .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("click graph to place cursors")
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
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
        self.ensure_cache();
        let raw_segments = &self.cached_segments;
        let gap_ranges = &self.cached_gaps;
        let (view_min, view_max) = self.view_bounds();

        // Theme-aware colors from shared palette
        let tc = self.theme_colors(ui.visuals().dark_mode);
        let line_color = tc.graph_line();
        let gap_color = tc.graph_gap();
        let mean_color = tc.graph_mean();
        let ref_color = tc.graph_ref();
        let cross_color = tc.graph_crossing();
        let cursor_color = tc.graph_cursor();
        let cursor_color_dim = tc.graph_cursor_dim();
        let env_color = tc.graph_envelope();

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
            .y_range_for_view(view_min, view_max)
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
        let mean_value = self.visible_stats().map(|(_, _, avg, _)| avg);
        let visible_stats = self.visible_stats();

        let cursor_unit = self.current_unit.clone();
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

            // Min/max envelope (drawn first so it's behind the data line)
            if show_envelope && !env_min.is_empty() {
                plot_ui.line(
                    Line::new("env_max", PlotPoints::new(env_max.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.line(
                    Line::new("env_min", PlotPoints::new(env_min.clone()))
                        .color(env_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            for seg in raw_segments {
                plot_ui.line(Line::new("data", PlotPoints::new(seg.clone())).color(line_color));
            }

            for &(gap_start, gap_end) in gap_ranges {
                plot_ui.vline(
                    VLine::new("gap_start", gap_start)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.vline(
                    VLine::new("gap_end", gap_end)
                        .color(gap_color)
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
            }

            // Mean line overlay
            if show_mean && let Some((_, _, avg, _)) = visible_stats {
                plot_ui.hline(
                    HLine::new("mean", avg)
                        .color(mean_color)
                        .style(egui_plot::LineStyle::dashed_loose()),
                );
            }

            // Reference line overlays
            if show_ref {
                for &v in &ref_values {
                    plot_ui.hline(
                        HLine::new("ref", v)
                            .color(ref_color)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
            }

            // Trigger crossing markers (where data crosses reference lines)
            if !crossings.is_empty() {
                plot_ui.points(
                    Points::new("crossings", PlotPoints::new(crossings.clone()))
                        .color(cross_color)
                        .radius(4.0)
                        .shape(egui_plot::MarkerShape::Diamond),
                );
            }

            // Measurement cursors (vertical + horizontal Y-value lines)
            if cursors_active {
                if let Some(t) = cursor_a {
                    plot_ui.vline(VLine::new("cursor_a", t).color(cursor_color));
                }
                if let Some(v) = cursor_va {
                    plot_ui.hline(
                        HLine::new("cursor_va", v)
                            .color(cursor_color_dim)
                            .style(egui_plot::LineStyle::dashed_dense()),
                    );
                }
                if let Some(t) = cursor_b {
                    plot_ui.vline(VLine::new("cursor_b", t).color(cursor_color));
                }
                if let Some(v) = cursor_vb {
                    plot_ui.hline(
                        HLine::new("cursor_vb", v)
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
        self.handle_interaction(ui, &response.response, &response.transform, can_interact);
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
    fn apply_pan(&mut self, time_delta: f64) {
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
    fn bbox_to_view(p0: (f64, f64), p1: (f64, f64)) -> (f64, f64, f64, f64) {
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
    fn apply_bbox_zoom(&mut self, p0: (f64, f64), p1: (f64, f64)) {
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
    fn handle_interaction(
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
            let stroke = egui::Stroke::new(1.0, visuals.selection.stroke.color);
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

    /// Render the minimap showing full history with viewport indicator.
    pub fn show_minimap(&mut self, ui: &mut Ui) {
        if self.history.len() < 2 {
            ui.allocate_space(egui::vec2(ui.available_width(), MINIMAP_HEIGHT));
            return;
        }

        self.ensure_cache();
        let raw_segments = &self.cached_segments;
        let (data_min, data_max) = self.data_time_range();
        let (view_min, view_max) = self.view_bounds();

        let tc = self.theme_colors(ui.visuals().dark_mode);
        let line_color = tc.minimap_line();

        // Allocate rect for minimap + label space below, with margin for bracket strokes
        let label_height = 14.0;
        let margin = 4.0; // room for bracket strokes at edges
        let total_height = MINIMAP_HEIGHT + label_height + margin * 2.0;
        let (full_rect, pointer_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), total_height),
            egui::Sense::click_and_drag(),
        );
        let pointer_response = pointer_response.on_hover_text(
            "Minimap — click or drag to pan, drag the bracket edges to resize the view",
        );
        // Give the minimap an accessible label so screen readers can
        // announce it as a navigable element.
        ui.ctx()
            .accesskit_node_builder(pointer_response.id, |builder| {
                builder.set_label("Graph minimap — click or drag to navigate timeline, drag bracket edges to resize");
            });
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

        // Draw data lines
        for seg in raw_segments {
            let points: Vec<egui::Pos2> = seg
                .iter()
                .map(|&[t, v]| {
                    let x = time_to_x(t);
                    // Simple Y mapping: find Y range from all data
                    let y_frac =
                        if let Some((y_lo, y_hi)) = self.y_range_for_view(data_min, data_max) {
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
                painter.line_segment([window[0], window[1]], egui::Stroke::new(1.0, line_color));
            }
        }

        // Draw viewport indicator as [ ] bracket markers
        let vp_left = time_to_x(view_min);
        let vp_right = time_to_x(view_max);
        let vp_color = tc.minimap_viewport();
        let vp_stroke = egui::Stroke::new(2.5, vp_color);
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
                egui::Stroke::new(1.0, label_color),
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
    }

    /// Combined render: toolbar + main graph + minimap.
    pub fn show(&mut self, ui: &mut Ui) {
        self.handle_keyboard(ui.ctx());
        self.show_toolbar(ui);
        let minimap_reserve = MINIMAP_HEIGHT + 30.0;
        let main_height = (ui.available_height() - minimap_reserve).max(60.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), main_height), |ui| {
            self.show_main(ui);
        });
        ui.add_space(4.0);
        self.show_minimap(ui);
    }

    /// Compute the time-integral over the visible window, in unit·seconds.
    pub fn visible_integral(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        self.cursor_integral(x_min, x_max)
    }

    /// Elapsed time (seconds) between the first and last data point in the
    /// visible window. Returns `None` if fewer than 2 points are visible.
    pub fn visible_data_span_secs(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        let mut first: Option<f64> = None;
        let mut last: Option<f64> = None;
        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t >= x_min && t <= x_max {
                if first.is_none() {
                    first = Some(t);
                }
                last = Some(t);
            }
        }
        match (first, last) {
            (Some(f), Some(l)) if l > f => Some(l - f),
            _ => None,
        }
    }

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

    /// Build min/max envelope using a sliding window centered on each data point.
    /// Returns (min_points, max_points) as Vec<[f64; 2]>.
    /// Build min/max envelope using a trailing sliding window.
    /// At each data point time `t`, computes min/max of all points in `[t - window, t]`.
    /// This answers "what was the range over the last N seconds?" with no look-ahead.
    fn build_envelope(
        &self,
        x_min: f64,
        x_max: f64,
        window_secs: f64,
    ) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let window = window_secs.max(0.1);

        // Collect points: need data back to x_min - window for edge correctness
        let points: Vec<(f64, f64)> = self
            .history
            .iter()
            .map(|p| (self.elapsed_secs(p.time), p.value))
            .filter(|(t, _)| *t >= x_min - window && *t <= x_max)
            .collect();

        if points.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let n = points.len();
        let mut min_pts = Vec::with_capacity(n);
        let mut max_pts = Vec::with_capacity(n);
        let mut lo = 0;

        for i in 0..n {
            let (t, _) = points[i];
            // Only emit envelope points within the visible range
            if t < x_min {
                continue;
            }

            let win_start = t - window;

            // Advance lo pointer past points before the window
            while lo < n && points[lo].0 < win_start {
                lo += 1;
            }

            // Scan [lo..] for points in [t - window, t]
            let mut vmin = f64::INFINITY;
            let mut vmax = f64::NEG_INFINITY;
            for p in points.iter().take(i + 1).skip(lo) {
                vmin = vmin.min(p.1);
                vmax = vmax.max(p.1);
            }

            min_pts.push([t, vmin]);
            max_pts.push([t, vmax]);
        }

        (min_pts, max_pts)
    }

    /// Find points where the data crosses any of the given threshold values.
    /// Returns crossing points as [time, threshold_value].
    fn find_crossings(&self, thresholds: &[f64], x_min: f64, x_max: f64) -> Vec<[f64; 2]> {
        let mut crossings = Vec::new();
        let mut prev: Option<(f64, f64)> = None;

        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t < x_min || t > x_max {
                continue;
            }

            if let Some((_, prev_v)) = prev {
                for &thresh in thresholds {
                    let crossed = (prev_v <= thresh && point.value >= thresh)
                        || (prev_v >= thresh && point.value <= thresh);
                    if crossed {
                        crossings.push([t, thresh]);
                    }
                }
            }
            prev = Some((t, point.value));
        }
        crossings
    }

    /// Find the nearest data point to the given time.
    /// Returns (snapped_time, value).
    fn nearest_point(&self, t: f64) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64, f64)> = None; // (distance, time, value)
        for point in &self.history {
            let pt = self.elapsed_secs(point.time);
            let dist = (pt - t).abs();
            if best.is_none() || dist < best.unwrap().0 {
                best = Some((dist, pt, point.value));
            }
        }
        best.map(|(_, t, v)| (t, v))
    }

    /// Compute the time-integral between two cursor positions using the trapezoidal
    /// rule. Returns the raw integral in unit·seconds, or `None` if fewer than 2
    /// data points exist in the range. Skips intervals exceeding `gap_threshold_secs`.
    fn cursor_integral(&self, ta: f64, tb: f64) -> Option<f64> {
        let (t_start, t_end) = if ta <= tb { (ta, tb) } else { (tb, ta) };
        let mut integral = 0.0;
        let mut prev: Option<(f64, f64)> = None; // (time, value)
        let mut has_pair = false;

        for point in &self.history {
            let t = self.elapsed_secs(point.time);
            if t < t_start {
                continue;
            }
            if t > t_end {
                break;
            }
            if let Some((pt, pv)) = prev {
                let dt = t - pt;
                if dt <= self.gap_threshold_secs {
                    integral += (pv + point.value) / 2.0 * dt;
                    has_pair = true;
                }
            }
            prev = Some((t, point.value));
        }

        has_pair.then_some(integral)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.history.len()
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
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
        g.push(5.0, Instant::now(), "DC V", "V");
        assert_eq!(g.len(), 1);
        assert!(!g.is_empty());
        assert!(g.origin.is_some());
    }

    #[test]
    fn mode_change_clears_history() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V");
        g.push(5.1, Instant::now(), "DC V", "V");
        assert_eq!(g.len(), 2);
        g.push(100.0, Instant::now(), "Ohm", "Ω");
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn max_points_evicts_oldest() {
        let mut g = Graph::new();
        for i in 0..MAX_POINTS + 100 {
            g.push(i as f64, Instant::now(), "DC V", "V");
        }
        assert_eq!(g.len(), MAX_POINTS);
    }

    #[test]
    fn clear_resets_everything() {
        let mut g = Graph::new();
        g.push(5.0, Instant::now(), "DC V", "V");
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
        g.push(1.0, Instant::now(), "DC V", "V");
        g.push(2.0, Instant::now(), "DC V", "V");
        g.push(3.0, Instant::now(), "DC V", "V");
        let segments = g.build_raw_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn gap_detection() {
        let mut g = Graph::new();
        g.push(1.0, Instant::now(), "DC V", "V");
        g.push(2.0, Instant::now(), "DC V", "V");
        let gaps = g.find_gap_ranges();
        assert!(gaps.is_empty());
    }

    #[test]
    fn elapsed_secs_relative_to_origin() {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(1.0, t0, "DC V", "V");
        g.push(2.0, t0 + Duration::from_millis(50), "DC V", "V");
        let t = g.elapsed_secs(g.history.back().unwrap().time);
        assert!((t - 0.05).abs() < 1e-9);
    }

    #[test]
    fn live_view_bounds_follow_latest() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V");
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

    #[test]
    fn cycle_time_window_shorter() {
        let mut g = Graph::new();
        g.time_window_secs = 60.0; // 1m
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 30.0).abs() < 0.1);
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 10.0).abs() < 0.1);
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 5.0).abs() < 0.1);
        // Already at minimum preset — stays at 5s
        g.cycle_time_window(-1);
        assert!((g.time_window_secs - 5.0).abs() < 0.1);
    }

    #[test]
    fn cycle_time_window_longer() {
        let mut g = Graph::new();
        g.time_window_secs = 60.0; // 1m
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 300.0).abs() < 0.1);
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 600.0).abs() < 0.1);
        // Already at maximum preset — stays at 600s
        g.cycle_time_window(1);
        assert!((g.time_window_secs - 600.0).abs() < 0.1);
    }

    #[test]
    fn scroll_view_does_not_panic() {
        let mut g = Graph::new();
        for i in 0..20 {
            g.push(i as f64, Instant::now(), "V DC", "V");
        }
        assert!(g.live);
        // With only ~ms of real elapsed time and a 60s window, the view
        // stays pinned at the end so live remains true. This test validates
        // the method doesn't panic on minimal data spans.
        g.scroll_view(-0.25);
        g.scroll_view(0.25);
    }

    #[test]
    fn jump_to_start_exits_live() {
        let mut g = Graph::new();
        g.push(1.0, Instant::now(), "V DC", "V");
        assert!(g.live);
        g.jump_to_start();
        assert!(!g.live);
    }

    #[test]
    fn bbox_to_view_normal_drag() {
        // Top-left (t=10, v=5) to bottom-right (t=20, v=2).
        let (center, window, y_min, y_max) = Graph::bbox_to_view((10.0, 5.0), (20.0, 2.0));
        assert!((center - 15.0).abs() < 1e-9);
        assert!((window - 10.0).abs() < 1e-9);
        assert!((y_min - 2.0).abs() < 1e-9);
        assert!((y_max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bbox_to_view_reversed_drag() {
        // Bottom-right to top-left should normalise to the same bounds.
        let (center, window, y_min, y_max) = Graph::bbox_to_view((20.0, 2.0), (10.0, 5.0));
        assert!((center - 15.0).abs() < 1e-9);
        assert!((window - 10.0).abs() < 1e-9);
        assert!((y_min - 2.0).abs() < 1e-9);
        assert!((y_max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bbox_to_view_degenerate_does_not_panic() {
        // Zero-area rectangle. Helper must not produce NaN — caller gates on
        // a minimum pixel size, so a zero window reaching this helper is a
        // theoretical edge case but we still want sane arithmetic.
        let (center, window, y_min, y_max) = Graph::bbox_to_view((5.0, 3.0), (5.0, 3.0));
        assert!((center - 5.0).abs() < 1e-9);
        assert!(window.abs() < 1e-9);
        assert!((y_min - 3.0).abs() < 1e-9);
        assert!((y_max - 3.0).abs() < 1e-9);
        assert!(window.is_finite());
    }

    #[test]
    fn apply_bbox_zoom_sets_state() {
        let mut g = Graph::new();
        assert!(g.live);
        assert!(!g.y_axis_fixed);
        g.apply_bbox_zoom((10.0, 5.0), (20.0, 2.0));
        assert!(!g.live);
        assert!(g.y_axis_fixed);
        assert!(g.y_user_set);
        assert!((g.view_center - 15.0).abs() < 1e-9);
        assert!((g.time_window_secs - 10.0).abs() < 1e-9);
        assert!((g.y_fixed_min - 2.0).abs() < 1e-9);
        assert!((g.y_fixed_max - 5.0).abs() < 1e-9);
        assert_eq!(g.y_min_text, "2.0000");
        assert_eq!(g.y_max_text, "5.0000");
    }

    #[test]
    fn apply_bbox_zoom_clamps_time_window_minimum() {
        // A very narrow drag must not produce a zero-width time window.
        let mut g = Graph::new();
        g.apply_bbox_zoom((10.0, 0.0), (10.0, 1.0));
        assert!(g.time_window_secs >= 0.1);
    }

    #[test]
    fn reset_view_restores_live_and_auto_y() {
        let mut g = Graph::new();
        g.apply_bbox_zoom((10.0, 5.0), (20.0, 2.0));
        assert!(!g.live);
        assert!(g.y_axis_fixed);
        g.reset_view();
        assert!(g.live);
        assert!(!g.y_axis_fixed);
        assert!(!g.y_user_set);
        assert_eq!(g.view_center, 0.0);
    }

    #[test]
    fn apply_pan_in_browse_mode_shifts_view_center() {
        let mut g = Graph::new();
        g.live = false;
        g.view_center = 100.0;
        g.apply_pan(5.0);
        assert!((g.view_center - 95.0).abs() < 1e-9);
        assert!(!g.live);
    }

    #[test]
    fn apply_pan_in_live_mode_drops_out_of_live() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V");
        assert!(g.live);
        // Any non-zero drag while live flips us to browse mode.
        g.apply_pan(2.0);
        assert!(!g.live);
    }

    #[test]
    fn apply_pan_in_live_mode_snaps_view_center_to_end() {
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.push(1.0, Instant::now(), "DC V", "V");
        // Zero-delta pan while live still snaps view_center to the end of
        // data — so the view doesn't visibly jump on drag start.
        g.apply_pan(0.0);
        let (_, data_max) = g.data_time_range();
        let expected = data_max - g.time_window_secs / 2.0;
        assert!((g.view_center - expected).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_newer_at_live_edge_returns_to_live() {
        // Browse mode, view right-edge exactly at data_max. A drag toward
        // newer data (time_delta < 0) must snap back to live instead of
        // drifting into empty future-space.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = 50.0;
        // Fake a data_max of 55 by priming origin and history.
        g.origin = Some(Instant::now());
        // Push a point; then override the elapsed calc is hard, so use a
        // simpler setup: set view_center so right edge = 0 and data_max = 0.
        g.view_center = -5.0; // right edge = 0 = data_max (no data → data_max=0)
        g.apply_pan(-1.0); // mouse left = newer; would push right edge past 0
        assert!(g.live);
        // view_center snaps to data_max - half = 0 - 5 = -5.
        assert!((g.view_center - -5.0).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_older_never_triggers_live_snap() {
        // Dragging back into history (time_delta > 0) must never flip live
        // on, even if the starting state is at the live edge.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = -5.0; // right edge at 0 (data_max = 0 with empty history)
        g.apply_pan(3.0);
        assert!(!g.live);
        assert!((g.view_center - -8.0).abs() < 1e-9);
    }

    #[test]
    fn apply_pan_toward_newer_below_live_edge_does_not_snap() {
        // Drag toward newer but still short of the live edge — just moves
        // view_center, does not re-enter live.
        let mut g = Graph::new();
        g.time_window_secs = 10.0;
        g.live = false;
        g.view_center = -50.0; // right edge at -45, well below data_max=0
        g.apply_pan(-2.0);
        assert!(!g.live);
        assert!((g.view_center - -48.0).abs() < 1e-9);
    }

    #[test]
    fn time_axis_label_integer_seconds() {
        // Existing behaviour preserved when step ≥ 1s.
        assert_eq!(format_time_axis_label(9.0, 1.0), "9 s");
        assert_eq!(format_time_axis_label(45.0, 5.0), "45 s");
    }

    #[test]
    fn time_axis_label_subsecond_step_adds_decimals() {
        // step=0.1 → 1 decimal; step=0.01 → 2 decimals.
        assert_eq!(format_time_axis_label(9.1, 0.1), "9.1 s");
        assert_eq!(format_time_axis_label(9.25, 0.01), "9.25 s");
        assert_eq!(format_time_axis_label(9.123, 0.001), "9.123 s");
    }

    #[test]
    fn time_axis_label_integer_value_with_subsecond_step_pads_decimals() {
        // A grid mark at an integer second still gets padded when the step
        // is sub-second, so all visible labels line up at the same precision.
        assert_eq!(format_time_axis_label(9.0, 0.1), "9.0 s");
        assert_eq!(format_time_axis_label(10.0, 0.01), "10.00 s");
    }

    #[test]
    fn time_axis_label_minutes_with_subsecond_step() {
        // Zooming into a span past 1 minute while sub-second still shows
        // the decimal seconds portion.
        assert_eq!(format_time_axis_label(90.5, 0.1), "1m 30.5s");
    }

    #[test]
    fn time_axis_label_whole_minute_with_integer_step() {
        // Step ≥ 1s, exact minute → shorthand "N m".
        assert_eq!(format_time_axis_label(120.0, 1.0), "2 m");
    }

    #[test]
    fn time_axis_label_hour_integer_step() {
        assert_eq!(format_time_axis_label(3720.0, 60.0), "1h 2m");
    }

    #[test]
    fn time_axis_label_hour_subsecond_step() {
        // Unlikely in practice but the formatter should not drop the
        // seconds field when hours are involved and step is sub-second.
        let out = format_time_axis_label(3725.5, 0.1);
        assert_eq!(out, "1h 2m 5.5s");
    }

    #[test]
    fn is_view_zoomed_reflects_state() {
        let mut g = Graph::new();
        assert!(!g.is_view_zoomed());
        g.live = false;
        assert!(g.is_view_zoomed());
        g.live = true;
        g.y_axis_fixed = true;
        assert!(g.is_view_zoomed());
    }
}
