//! Taking readings in: pushing samples and sub-values, the breaks an
//! overload, a no-reading or a dropout leaves, restarting the trace on a
//! mode or unit change, and evicting the oldest points.

use std::collections::VecDeque;
use std::time::Instant;

use super::breaks::Breaks;
use super::minimap::MinimapDrag;
use super::{
    GAP_MINIMUM_SECS, GAP_MULTIPLIER, Graph, MAX_OVERLAYS, OverlayPoint, OverlaySeries, PlotSample,
    SERIES_DROP_FRAMES, pattern,
};

impl Graph {
    /// Update gap detection threshold based on sample interval.
    pub fn set_sample_interval_ms(&mut self, ms: u32) {
        let interval_secs = (ms as f64 / 1000.0).max(0.1); // 0ms → use ~100ms wire time
        let threshold = (interval_secs * GAP_MULTIPLIER).max(GAP_MINIMUM_SECS);
        // The history keeps its breaks: each was decided when its silence
        // ended, against the threshold then in force.
        self.gap_threshold_secs = threshold;
        self.silence_threshold_secs = self.silence_threshold_secs.max(threshold);
    }

    /// Change how many points the history keeps, dropping the oldest at once
    /// when the new bound is below what is already in it.
    ///
    /// Lowering it is a user's answer to memory pressure, so it has to take
    /// effect now rather than at the next sample — and give the memory back,
    /// which `pop_front` alone never does.
    pub fn set_max_points(&mut self, n: usize) {
        self.max_points = n;
        for o in &mut self.overlays {
            if o.points.len() > n {
                o.points.drain(..o.points.len() - n);
                o.points.shrink_to_fit();
            }
        }
        if self.history.len() <= n {
            return;
        }
        // Drop the level rather than evict into it. One eviction costs a
        // front-bucket rescan whenever the point leaving was that bucket's
        // extreme, which on a ramp is every point: draining a bucket that way
        // is quadratic, and dropping millions of points froze the settings
        // panel for half a minute. A recut is one pass, on the next frame.
        self.minimap_level = None;
        self.evict_to(n);
        self.history.shrink_to_fit();
        for o in &mut self.overlays {
            o.points.shrink_to_fit();
        }
    }

    /// Drop the oldest points until at most `keep` are left, taking the
    /// overlay points no newer than each and the minimap's bucket with them.
    ///
    /// Only an evicted point takes overlay points with it: a point of a frame
    /// that carried only sub-values, older than the whole history, stays
    /// until the history moves past it.
    ///
    /// `push_sample` asks for one below the bound — it is about to add a
    /// point — while `set_max_points` asks for the bound itself, after
    /// dropping the level. For the one point a push sheds, evicting into the
    /// level is cheaper than the full pass a recut costs.
    fn evict_to(&mut self, keep: usize) {
        while self.history.len() > keep {
            let Some(oldest) = self.history.pop_front() else {
                break;
            };
            for o in &mut self.overlays {
                while o.points.front().is_some_and(|p| p.time <= oldest.time) {
                    o.points.pop_front();
                }
            }
            while self
                .silences
                .front()
                .is_some_and(|&(_, end)| end <= oldest.time)
            {
                self.silences.pop_front();
            }
            let first_seq = self.pushed_total.saturating_sub(self.history.len() as u64);
            if let Some(level) = &mut self.minimap_level {
                level.evict(
                    oldest.value,
                    self.history.iter().map(|p| p.value),
                    first_seq,
                );
            }
        }
    }

    /// Push a single-series sample.
    ///
    /// Test-only since the App began routing every sample through
    /// `push_sample`: it keeps the graph's own tests expressing the
    /// single-display case — by far the common one — without restating the
    /// two sub-value fields each time.
    #[cfg(test)]
    pub fn push(
        &mut self,
        value: f64,
        timestamp: Instant,
        mode: &str,
        unit: &str,
        display_raw: Option<&str>,
    ) {
        self.push_sample(PlotSample {
            value: Some(value),
            timestamp,
            mode,
            unit,
            display_raw,
            series: None,
            main_label: None,
            levels: false,
            overlays: &[],
        });
    }

    /// Push a sample together with the sub-values drawn beside it.
    ///
    /// A sample without a value (a frame carrying only sub-values) records
    /// its overlay points and switches or restarts the trace on a change of
    /// mode, unit or series like any other, but leaves the history, the
    /// minimap, an open break and the spoken last reading alone. A sample
    /// with neither (an over-range reading) restarts on a new mode only.
    pub fn push_sample(&mut self, sample: PlotSample<'_>) {
        let (value, timestamp, mode, unit, display_raw) = (
            sample.value,
            sample.timestamp,
            sample.mode,
            sample.unit,
            sample.display_raw,
        );
        let now = timestamp;
        if value.is_none() && sample.overlays.is_empty() {
            // An over-range reading after a dial turn (mV to Ω with the leads
            // open) belongs to the new mode, not at the end of the old one's
            // trace. Only the mode counts: auto-range lifts an open Ω reading
            // to its top range (kΩ to MΩ), and restarting on that unit would
            // drop the trace each time the leads lift.
            if self.current_mode.as_deref() != Some(mode) {
                self.restart_trace(now, mode, unit, sample.series);
            }
            return;
        }

        if self.origin.is_none() {
            self.origin = Some(now);
        }
        self.main_label = sample.main_label;

        // A unit change is as much a scale change as a mode change. Auto-range
        // keeps the mode string fixed while the unit moves a decade (Ω→kΩ,
        // mV→V, nF→µF), so comparing only the mode let the trace collapse by
        // 1000x mid-plot with the axis relabelled and no gap to show why.
        // A change of plotted series restarts the trace for the same reason:
        // two sub-values can share a mode *and* a unit (T1 and T2 are both
        // "DC V"/"°C"), so switching from one to the other would otherwise
        // append onto the previous one's trace with nothing to mark the join —
        // unless the new series is already kept beside the old one, and the
        // two traces only change places. The kept traces take this frame's
        // units first, so a switch on the frame a unit steps lands on the one
        // trace that restarted.
        self.reconcile_overlay_units(sample.overlays);
        let incoming = sample.series.unwrap_or(self.main_name());
        self.restart_on_unit_change(incoming, unit);
        let same_mode = self.current_mode.as_deref() == Some(mode);
        let series_changed = self.current_series.as_deref() != sample.series;
        let swapped = same_mode && series_changed && self.swap_plotted_series(sample.series, unit);
        if !swapped && (!same_mode || self.current_unit != unit || series_changed) {
            self.restart_trace(now, mode, unit, sample.series);
        }
        self.heard(now);
        self.register_overlays(sample.overlays);
        match value {
            Some(value) => {
                self.levels = sample.levels;
                let last = self.history.back().map(|p| p.time);
                if self.stopped_for(self.main_missing_frames, last, now) {
                    self.breaks.gap();
                }
                self.main_missing_frames = 0;
                self.push_point(value, now, display_raw);
            }
            None => self.main_missing_frames += 1,
        }
        for i in 0..self.overlays.len() {
            let label = self.overlays[i].label.as_str();
            let Some(&(_, _, v)) = sample.overlays.iter().find(|(l, _, _)| *l == label) else {
                self.overlays[i].missing_frames += 1;
                continue;
            };
            let o = &self.overlays[i];
            let last = o.points.back().copied();
            if self.stopped_for(o.missing_frames, last.map(|p| p.time), now)
                && let Some(OverlayPoint {
                    time,
                    value: Some(_),
                }) = last
            {
                self.overlays[i].push(OverlayPoint { time, value: None }, self.max_points);
            }
            let o = &mut self.overlays[i];
            o.missing_frames = 0;
            o.push(
                OverlayPoint {
                    time: now,
                    value: v,
                },
                self.max_points,
            );
        }
    }

    /// Whether a series that last had a point at `last`, and has been missing
    /// from `missing` frames since, stopped rather than paused: its trace
    /// breaks before its next point instead of being drawn across.
    ///
    /// Frames kept coming, so no silence says so — a UT181A with REL off for
    /// half a minute, or a held UT61E+ sending only its AC component. Both a
    /// run of frames and a stretch past the gap threshold are needed, as for
    /// dropping a series option: the frames alone would break a part the
    /// meter sends every other frame at a slow interval, and the time alone
    /// one it sends every other frame at 0.67 s.
    fn stopped_for(&self, missing: u32, last: Option<Instant>, now: Instant) -> bool {
        missing >= SERIES_DROP_FRAMES
            && last.is_some_and(|last| {
                now.checked_duration_since(last)
                    .is_some_and(|d| d.as_secs_f64() > self.gap_threshold_secs)
            })
    }

    /// Note that a frame arrived at `t`, recording the silence before it if
    /// there was one worth remembering (see `silences`).
    fn heard(&mut self, t: Instant) {
        if let Some(last) = self.last_heard
            && t.checked_duration_since(last)
                .is_some_and(|d| d.as_secs_f64() > self.silence_threshold_secs)
        {
            // A stream that never grows the history is never evicted from;
            // the bound keeps it from growing without limit all the same.
            while self.silences.len() >= self.max_points.max(1) {
                self.silences.pop_front();
            }
            self.silences.push_back((last, t));
        }
        self.last_heard = Some(t);
        self.silence_threshold_secs = self.gap_threshold_secs;
    }

    /// Whether a silence (see `silences`) falls between the frames at `from`
    /// and `to`.
    pub(super) fn silent_between(&self, from: Instant, to: Instant) -> bool {
        let first = self.silences.partition_point(|&(start, _)| start < from);
        self.silences
            .range(first..)
            .take_while(|&&(_, end)| end <= to)
            .next()
            .is_some()
    }

    /// Number the history's readings from its first: it was replaced or
    /// restarted, and figures cached on the old numbers are for other
    /// readings.
    pub(super) fn restart_sequence(&mut self) {
        self.pushed_total = self.history.len() as u64;
        self.history_epoch += 1;
    }

    /// Restart each kept trace whose unit moved in this frame (a frequency
    /// auto-ranging Hz→kHz): it drops its points, as the plotted series does
    /// on a decade step, and restarts alone. The series being plotted is
    /// checked by the caller: a frame carries it apart from the others.
    fn reconcile_overlay_units(&mut self, overlays: &[(&str, &str, Option<f64>)]) {
        for &(label, unit, _) in overlays {
            self.restart_on_unit_change(label, unit);
        }
    }

    /// Start a new trace for `mode`, `unit` and `series` at `now`.
    fn restart_trace(&mut self, now: Instant, mode: &str, unit: &str, series: Option<&str>) {
        self.history.clear();
        self.overlays.clear();
        self.current_mode = Some(mode.to_string());
        self.current_unit = unit.to_string();
        self.current_series = series.map(str::to_owned);
        // The main reading's chip hid this mode's reading, not the next's.
        self.main_hidden = false;
        self.levels = false;
        self.origin = Some(now);
        self.set_live(true);
        self.view_center = 0.0;
        // Drop any pinned Y range too: it was chosen for the previous
        // mode's scale, and keeping it would plot ohms against volt bounds
        // — the trace lands far outside the plot and the graph just looks
        // empty, with the old numbers still on the axis. `clear()` and
        // `reset_view()` both release these for the same reason.
        self.y_axis.release();
        self.cursors.clear();
        self.box_zoom = None;
        self.minimap_level = None;
        self.restart_sequence();
        self.last_display_raw = None;
        self.silences.clear();
        self.main_missing_frames = 0;
        self.breaks = Breaks::default();
        self.dash_phases = pattern::DashPhases::default();
    }

    /// Restart the kept trace `label` if its unit is no longer `unit`.
    fn restart_on_unit_change(&mut self, label: &str, unit: &str) {
        if let Some(o) = self.overlays.iter_mut().find(|o| o.label == label)
            && o.unit != unit
        {
            // A range fixed for the old unit means nothing in the new one,
            // nor when the unit comes back: the axis fits its data again.
            self.y_axis.pins.remove(&o.unit);
            o.unit = unit.to_string();
            o.points.clear();
            o.missing_frames = 0;
        }
    }

    /// Start a trace for each sub-value seen for the first time. It begins at
    /// its first point: nothing is back-filled.
    ///
    /// Past [`MAX_OVERLAYS`], a trace in the plotted unit takes the place of
    /// one in another: kept Frequency and Period traces must not keep a
    /// MIN/MAX's Min off the plot. Otherwise the new one is skipped.
    fn register_overlays(&mut self, overlays: &[(&str, &str, Option<f64>)]) {
        for &(label, unit, _) in overlays {
            if self.overlays.iter().any(|o| o.label == label) {
                continue;
            }
            let trace = OverlaySeries {
                label: label.to_string(),
                unit: unit.to_string(),
                points: VecDeque::new(),
                missing_frames: 0,
            };
            if self.overlays.len() < MAX_OVERLAYS {
                self.overlays.push(trace);
            } else if unit == self.current_unit
                && let Some(i) = self
                    .overlays
                    .iter()
                    .position(|o| o.unit != self.current_unit)
            {
                self.overlays[i] = trace;
            }
        }
    }

    /// Append one point of the plotted series.
    fn push_point(&mut self, value: f64, now: Instant, display_raw: Option<&str>) {
        // Track the most recent raw display string so the a11y plot
        // summary can speak it verbatim. We update it in-place to avoid
        // allocating per push when the underlying `Cow<'static, str>` is a
        // borrowed protocol-level constant.
        match (display_raw, &mut self.last_display_raw) {
            (Some(s), Some(buf)) => {
                buf.clear();
                buf.push_str(s);
            }
            (Some(s), None) => self.last_display_raw = Some(s.to_string()),
            (None, _) => self.last_display_raw = None,
        }

        // One short of the bound: this sample is about to take the last slot.
        self.evict_to(self.max_points.saturating_sub(1));

        let point = self.breaks.point(now, value);
        // Everything the minimap's level needs, decided against the point
        // before this one exactly as `build_segments_for_range` decides it for
        // a consecutive pair — so appending and rebuilding cannot disagree.
        let seq = self.pushed_total;
        let t = self.elapsed_secs(point.time);
        let prev_time = self.history.back().map(|p| p.time);
        let break_kind = prev_time.and_then(|prev| self.breaks_before(prev, &point));
        let gaps = match (prev_time, break_kind) {
            (Some(prev), Some(kind)) => self.gap_entries(prev, &point, kind),
            _ => [None, None],
        };
        self.history.push_back(point);
        if let Some(level) = &mut self.minimap_level {
            level.append(t, value, break_kind);
            for (start, end, kind) in gaps.into_iter().flatten() {
                level.push_gap(start, end, kind, seq);
            }
        }
        self.pushed_total += 1;
    }

    /// Record that the series was interrupted — the meter reported a value
    /// that can't be plotted (an overload), so the next point starts a new
    /// segment.
    ///
    /// Needed because gap detection is otherwise purely time-based: an
    /// over-range excursion shorter than the gap threshold leaves no hole in
    /// the timestamps, so the trace would be drawn straight through it and
    /// the visible-range stats and integral would run across a value the
    /// meter never measured.
    ///
    /// Sub-value traces are not split by it: each point keeps its own time,
    /// and an over-range sub-value breaks its own trace. The App records the
    /// frame's sub-values before this, as a sample without a value, so that a
    /// frame switching the plotted series breaks the series it switched to.
    ///
    /// Arriving after a word shown instead of a reading, it starts the band
    /// here and leaves the word's stretch a gap (`gap_entries`). A word
    /// after that band is drawn into it: one interruption splits in two, not
    /// three.
    pub fn push_break(&mut self, timestamp: Instant) {
        self.heard(timestamp);
        self.breaks.overload(timestamp);
    }

    /// Record that the meter showed a word instead of a reading ("Auto" with
    /// the probes lifted): the trace breaks as for an overload, but the
    /// stretch is drawn as a gap rather than an over-range band, since the
    /// meter was not over range.
    ///
    /// Keeps the live view moving the way an overload does — the meter is
    /// still sending. Arriving during an overload, it closes the band at the
    /// last OL sample and leaves the rest a gap, through the same split a
    /// dropout mid-overload takes (`gap_entries`).
    pub fn push_no_reading(&mut self, timestamp: Instant) {
        self.heard(timestamp);
        self.breaks.no_reading(timestamp);
    }

    /// Record that data was genuinely lost — the link dropped, or
    /// acquisition was stopped — as opposed to the meter merely going quiet.
    ///
    /// The graph cannot tell those apart from timestamps. This meter pauses
    /// its output for over a second while auto-ranging (measured: 462 ms then
    /// 1153 ms stepping 2.2MΩ → 22MΩ → 220MΩ, against a 97 ms steady
    /// cadence), which by elapsed time alone is indistinguishable from an
    /// unplugged cable. The App receives the disconnect and drives pause, so
    /// it states what happened instead of leaving the graph to infer it.
    ///
    /// Every sub-value trace breaks here too, with a `None` at its own last
    /// time: a loss says nothing about when the next point comes, and with no
    /// timestamp of its own the call has no other time to put it at.
    pub fn push_data_loss(&mut self) {
        self.breaks.mark_data_loss();
        for o in &mut self.overlays {
            if let Some(&OverlayPoint {
                time,
                value: Some(_),
            }) = o.points.back()
            {
                o.push(OverlayPoint { time, value: None }, self.max_points);
            }
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
        // `selected_series` deliberately survives: Ctrl+L clears the data, not
        // the user's choice of what to plot. `series_options` and
        // `hidden_overlays` survive too — the meter is still sending, and the
        // next sample refreshes the offer.
        self.overlays.clear();
        self.current_series = None;
        self.breaks = Breaks::default();
        self.silences.clear();
        self.last_heard = None;
        self.main_missing_frames = 0;
        self.current_mode = None;
        self.current_unit.clear();
        self.levels = false;
        self.last_display_raw = None;
        self.origin = None;
        self.set_live(true);
        self.view_center = 0.0;
        self.y_axis.release();
        self.cursors.clear();
        self.minimap_drag = MinimapDrag::None;
        self.box_zoom = None;
        self.minimap_level = None;
        self.restart_sequence();
        self.dash_phases = pattern::DashPhases::default();
    }
}
