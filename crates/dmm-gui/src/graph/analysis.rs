//! Statistics computed over the visible slice: mean/min/max, the integral,
//! the min/max envelope, reference-line crossings and cursor readouts.

use std::collections::VecDeque;

use super::{GapKind, Graph};

impl Graph {
    /// Compute the time-integral over the visible window, in unit·seconds.
    pub fn visible_integral(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        self.cursor_integral(x_min, x_max)
    }

    /// Elapsed time (seconds) between the first and last data point in the
    /// visible window. Returns `None` if fewer than 2 points are visible.
    pub fn visible_data_span_secs(&self) -> Option<f64> {
        let (x_min, x_max) = self.view_bounds();
        let (start, end) = self.visible_index_range(x_min, x_max);
        if end - start < 2 {
            return None;
        }
        let first = self.elapsed_secs(self.history[start].time);
        let last = self.elapsed_secs(self.history[end - 1].time);
        if last > first {
            Some(last - first)
        } else {
            None
        }
    }

    pub fn visible_stats(&self) -> Option<(f64, f64, f64, usize)> {
        let (x_min, x_max) = self.view_bounds();
        let (start, end) = self.visible_index_range(x_min, x_max);
        if start >= end {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let count = end - start;
        for i in start..end {
            let v = self.history[i].value;
            min = min.min(v);
            max = max.max(v);
            sum += v;
        }
        Some((min, max, sum / count as f64, count))
    }

    /// Build min/max envelope using a trailing sliding window.
    /// At each data point time `t`, computes min/max of all points in `[t - window, t]`.
    /// This answers "what was the range over the last N seconds?" with no look-ahead.
    ///
    /// Sliding-window min/max via two monotonic deques (front holds the
    /// current extremum). Each index is pushed and popped at most once, so
    /// the whole pass is O(n) instead of the previous O(n²) which re-scanned
    /// the window for every point.
    pub(super) fn build_envelope(
        &self,
        x_min: f64,
        x_max: f64,
        window_secs: f64,
    ) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let window = window_secs.max(0.1);

        // Collect points: need data back to x_min - window for edge correctness.
        // Use visible_index_range to skip the bulk of the history.
        let (start, end) = self.visible_index_range(x_min - window, x_max);
        let points: Vec<(f64, f64)> = (start..end)
            .map(|i| {
                let p = &self.history[i];
                (self.elapsed_secs(p.time), p.value)
            })
            .collect();

        if points.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let n = points.len();
        let mut min_pts = Vec::with_capacity(n);
        let mut max_pts = Vec::with_capacity(n);
        // Deques hold indices into `points`, with values monotonically
        // increasing (min) or decreasing (max). The front is always the
        // extremum of the current window.
        let mut min_deque: VecDeque<usize> = VecDeque::new();
        let mut max_deque: VecDeque<usize> = VecDeque::new();

        for (i, &(t, v)) in points.iter().enumerate() {
            // Maintain the monotonic invariant by popping any back entries
            // that the new value supersedes.
            while min_deque.back().is_some_and(|&b| points[b].1 >= v) {
                min_deque.pop_back();
            }
            min_deque.push_back(i);
            while max_deque.back().is_some_and(|&b| points[b].1 <= v) {
                max_deque.pop_back();
            }
            max_deque.push_back(i);

            // Drop fronts that have fallen out of the trailing window.
            let win_start = t - window;
            while min_deque.front().is_some_and(|&f| points[f].0 < win_start) {
                min_deque.pop_front();
            }
            while max_deque.front().is_some_and(|&f| points[f].0 < win_start) {
                max_deque.pop_front();
            }

            // Only emit envelope points within the visible range. Points
            // before `x_min` still feed the deques so the first visible
            // sample sees the correct trailing window.
            if t < x_min {
                continue;
            }

            let vmin = points[*min_deque.front().expect("min deque non-empty after push")].1;
            let vmax = points[*max_deque.front().expect("max deque non-empty after push")].1;
            min_pts.push([t, vmin]);
            max_pts.push([t, vmax]);
        }

        (min_pts, max_pts)
    }

    /// Find points where the data crosses any of the given threshold values.
    /// Returns crossing points as [time, threshold_value].
    pub(super) fn find_crossings(
        &self,
        thresholds: &[f64],
        x_min: f64,
        x_max: f64,
    ) -> Vec<[f64; 2]> {
        let (start, end) = self.visible_index_range(x_min, x_max);
        let mut crossings = Vec::new();
        let mut prev: Option<f64> = None;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);
            if let Some(prev_v) = prev {
                for &thresh in thresholds {
                    // Strict on the "coming from" side. With `<=` on both,
                    // prev_v == value == thresh satisfied *both* arms, so a
                    // signal resting exactly on the reference (a steady 5.000 V
                    // against Ref 5, or open leads reading 0.000 against Ref 0)
                    // emitted a marker for every sample and painted a solid row
                    // over the trace. A reading that arrives at the threshold
                    // is still marked once, from whichever side it came.
                    let crossed = (prev_v < thresh && point.value >= thresh)
                        || (prev_v > thresh && point.value <= thresh);
                    if crossed {
                        crossings.push([t, thresh]);
                    }
                }
            }
            prev = Some(point.value);
        }
        crossings
    }

    /// Find the nearest data point to the given time via binary search.
    /// Returns (snapped_time, value).
    pub(super) fn nearest_point(&self, t: f64) -> Option<(f64, f64)> {
        if self.history.is_empty() {
            return None;
        }
        // Find the insertion point — the first index whose time > t.
        let (a, b) = self.history.as_slices();
        let a_len = a.len();
        let pos_a = a.partition_point(|p| self.elapsed_secs(p.time) <= t);
        let pos = if pos_a < a_len {
            pos_a
        } else {
            a_len + b.partition_point(|p| self.elapsed_secs(p.time) <= t)
        };

        // The nearest point is either at `pos` or `pos - 1`. Check both.
        let mut best: Option<(f64, f64, f64)> = None; // (dist, time, value)
        for &idx in &[pos.wrapping_sub(1), pos] {
            if idx < self.history.len() {
                let pt = self.elapsed_secs(self.history[idx].time);
                let dist = (pt - t).abs();
                if best.is_none_or(|(best_dist, _, _)| dist < best_dist) {
                    best = Some((dist, pt, self.history[idx].value));
                }
            }
        }
        best.map(|(_, t, v)| (t, v))
    }

    /// Compute the time-integral between two cursor positions using the trapezoidal
    /// rule. Returns the raw integral in unit·seconds, or `None` if fewer than 2
    /// data points exist in the range. Skips intervals exceeding
    /// `gap_threshold_secs`, and intervals interrupted by an overload.
    pub(super) fn cursor_integral(&self, ta: f64, tb: f64) -> Option<f64> {
        let (t_start, t_end) = if ta <= tb { (ta, tb) } else { (tb, ta) };
        let (start, end) = self.visible_index_range(t_start, t_end);
        let mut integral = 0.0;
        let mut prev: Option<(f64, f64)> = None; // (time, value)
        let mut has_pair = false;

        for i in start..end {
            let point = &self.history[i];
            let t = self.elapsed_secs(point.time);
            // An overload breaks the series even when the samples either side
            // of it are adjacent in time — integrating across it would credit
            // the area under a value the meter never measured.
            if let Some((pt, pv)) = prev {
                let dt = t - pt;
                if dt <= self.gap_threshold_secs && point.break_before.is_none() {
                    integral += (pv + point.value) / 2.0 * dt;
                    has_pair = true;
                }
            }
            prev = Some((t, point.value));
        }

        has_pair.then_some(integral)
    }

    /// Span of an interruption that hasn't closed yet, as `(start, end)`.
    ///
    /// `build_segments_for_range` pairs consecutive points, so it can only
    /// emit a gap once a sample arrives *after* the interruption. While an
    /// overload is still in progress no such sample exists, and the trace
    /// would simply stop with nothing to say why until the meter came back.
    ///
    /// Anchored to the last plotted point — where the trace ends — and closed
    /// at the newest overload sample rather than at "now", so a meter that
    /// disconnects mid-overload leaves a band covering the samples we actually
    /// received instead of one that grows forever.
    pub(super) fn pending_overload_span(&self) -> Option<(f64, f64)> {
        if self.pending_break != Some(GapKind::Overload) {
            return None;
        }
        let start = self.history.back().map(|p| self.elapsed_secs(p.time))?;
        let end = self
            .pending_break_since
            .map(|t| self.elapsed_secs(t))
            .unwrap_or(start);
        Some((start, end.max(start)))
    }
}
