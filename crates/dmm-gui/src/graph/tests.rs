use super::render::{KeyStyle, quantize_for_hash};
use super::time::format_time_axis_label;
use super::toolbar::{overlay_chip_label, series_chip_label};
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
    g.push(5.0, Instant::now(), "DC V", "V", None);
    assert_eq!(g.len(), 1);
    assert!(!g.is_empty());
    assert!(g.origin.is_some());
}

#[test]
fn push_records_display_raw_for_a11y() {
    let mut g = Graph::new();
    g.push(0.001234, Instant::now(), "DC V", "mV", Some("  1.234"));
    assert_eq!(g.last_display_raw.as_deref(), Some("  1.234"));
    // A second push with display_raw must reuse the existing String
    // buffer, not allocate a new one.
    g.push(0.005, Instant::now(), "DC V", "mV", Some("  5.000"));
    assert_eq!(g.last_display_raw.as_deref(), Some("  5.000"));
}

#[test]
fn push_clears_display_raw_on_mode_change() {
    let mut g = Graph::new();
    g.push(0.001, Instant::now(), "DC V", "mV", Some("  1.000"));
    // Mode change clears history and the cached raw.
    g.push(100.0, Instant::now(), "Ohm", "Ω", None);
    assert!(g.last_display_raw.is_none());
}

#[test]
fn quantize_for_hash_collapses_jitter() {
    // Two values that differ by less than the quantization step (1e-3)
    // must hash-quantize to the same bucket.
    assert_eq!(quantize_for_hash(1.2345), quantize_for_hash(1.2346));
    // Values that differ by more than one bucket must not collapse.
    assert_ne!(quantize_for_hash(1.234), quantize_for_hash(1.236));
    // NaN gets a sentinel so it doesn't poison the hasher.
    assert_eq!(quantize_for_hash(f64::NAN), i64::MIN);
}

#[test]
fn mode_change_clears_history() {
    let mut g = Graph::new();
    g.push(5.0, Instant::now(), "DC V", "V", None);
    g.push(5.1, Instant::now(), "DC V", "V", None);
    assert_eq!(g.len(), 2);
    g.push(100.0, Instant::now(), "Ohm", "Ω", None);
    assert_eq!(g.len(), 1);
}

/// Auto-range crossing a decade keeps the mode string but moves the unit,
/// so 219 Ω and 0.22 kΩ would otherwise share one series — the trace
/// collapses 1000x mid-plot and the axis silently relabels.
#[test]
fn unit_change_clears_history() {
    let mut g = Graph::new();
    g.push(150.0, Instant::now(), "Ω", "Ω", None);
    g.push(219.0, Instant::now(), "Ω", "Ω", None);
    assert_eq!(g.len(), 2);
    g.push(0.22, Instant::now(), "Ω", "kΩ", None);
    assert_eq!(g.len(), 1, "kΩ points must not share a series with Ω");
    assert_eq!(g.current_unit, "kΩ");
}

/// The pinned Y range is chosen for the old decade's numbers; keeping it
/// across a unit change plots the new scale far outside the view.
#[test]
fn unit_change_releases_the_pinned_y_range() {
    let mut g = Graph::new();
    g.push(150.0, Instant::now(), "Ω", "Ω", None);
    g.y_axis_fixed = true;
    g.y_user_set = true;
    g.push(0.22, Instant::now(), "Ω", "kΩ", None);
    assert!(!g.y_axis_fixed);
    assert!(!g.y_user_set);
}

/// A brief over-range excursion leaves no hole in the timestamps, so
/// time-based gap detection can't see it: the trace was drawn straight
/// from the last good sample to the first one after, through a region the
/// meter reported as unmeasurable.
#[test]
fn overload_breaks_the_trace_without_a_time_gap() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    g.push_break(Instant::now());
    g.push(3.0, t0 + Duration::from_millis(200), "DC V", "V", None);

    // All three samples are 100ms apart — well inside the 1s minimum gap
    // threshold — so only the explicit break can split them.
    let segments = g.all_segments();
    assert_eq!(segments.len(), 2, "overload must split the trace");
    assert_eq!(segments[0].len(), 2);
    assert_eq!(segments[1].len(), 1);
}

#[test]
fn overload_is_reported_as_a_gap_range() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(Instant::now());
    g.push(3.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    assert_eq!(g.visible_gaps().len(), 1);
}

/// An overload that is still in progress has no closing sample, so the
/// paired-gap builder emits nothing and the trace just stops. The opening
/// marker has to be drawn from the pending state instead.
#[test]
fn an_unfinished_overload_still_marks_where_the_trace_stopped() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    assert_eq!(g.pending_overload_span(), None, "no break yet");

    g.push_break(Instant::now());
    // Still overloaded: no closing sample, so no paired gap exists...
    assert!(g.visible_gaps().is_empty());
    // ...but the pending span anchors to the last plotted point.
    let (start, _) = g
        .pending_overload_span()
        .expect("pending span while overloaded");
    assert!((start - 0.1).abs() < 1e-9, "got {start}");

    // Meter recovers: the pair takes over and the pending marker clears.
    g.push(3.0, t0 + Duration::from_millis(500), "DC V", "V", None);
    assert_eq!(g.pending_overload_span(), None);
    assert_eq!(g.visible_gaps().len(), 1);
}

/// The two kinds must be distinguishable by the renderer: an overload is
/// the meter reporting a condition, a time gap is the absence of any
/// report. They are drawn differently, so the builder has to say which.
#[test]
fn gap_kinds_distinguish_overload_from_data_loss() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(50));
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    // Well past the 1 s minimum gap threshold: a dropout, not an overload.
    g.push(3.0, t0 + Duration::from_secs(5), "DC V", "V", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload, GapKind::NoData]);
}

/// Losing the link mid-overload is two things end to end: a stretch the
/// meter reported over-range, then a stretch it reported nothing. Folding
/// the silence into the band would claim the meter was over range for a
/// period it never reported at all.
#[test]
fn a_dropout_during_an_overload_splits_into_band_then_gap() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    // Half a second of overload, then the link drops for 30 s.
    g.push_break(t0 + Duration::from_millis(500));
    g.push_data_loss();
    g.push(2.0, t0 + Duration::from_secs(30), "DC V", "V", None);

    let gaps = g.visible_gaps();
    assert_eq!(gaps.len(), 2, "expected band then gap, got {gaps:?}");

    let (band_start, band_end, band_kind) = gaps[0];
    assert_eq!(band_kind, GapKind::Overload);
    assert!(band_start.abs() < 1e-9);
    assert!(
        (band_end - 0.5).abs() < 1e-9,
        "band must stop at the last OL sample we heard, got {band_end}"
    );

    let (gap_start, gap_end, gap_kind) = gaps[1];
    assert_eq!(gap_kind, GapKind::NoData);
    assert!((gap_start - 0.5).abs() < 1e-9);
    assert!((gap_end - 30.0).abs() < 1e-9);
}

/// Measured on hardware: releasing the leads makes the meter step
/// 2.2MΩ → 22MΩ → 220MΩ, pausing 462 ms and then 1153 ms against a 97 ms
/// steady cadence. That silence is longer than the gap threshold but is
/// not data loss — the link never dropped — and must not be painted as a
/// dropout.
#[test]
fn an_auto_range_stutter_is_not_a_dropout() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.4934, t0, "Ω", "MΩ", None);
    g.push_break(t0 + Duration::from_millis(97)); // OL, 2.2MΩ
    g.push_break(t0 + Duration::from_millis(559)); // OL, 22MΩ — 462 ms later
    // 1153 ms later the meter reports again, still without a disconnect.
    g.push(97.14, t0 + Duration::from_millis(1712), "Ω", "MΩ", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(
        kinds,
        vec![GapKind::Overload],
        "a quiet meter is not a lost connection"
    );
}

/// A disconnect is not the only way data stops. A meter that powers off
/// mid-overload keeps its USB bridge enumerated, so nothing raises
/// Disconnected — the reads just time out. That has to split the band
/// too, or it would claim over-range for the whole outage.
#[test]
fn a_timeout_outage_during_an_overload_also_splits() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(100));
    // Reads time out until the App gives up; no Disconnected involved.
    g.push_data_loss();
    g.push(2.0, t0 + Duration::from_secs(20), "DC V", "V", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload, GapKind::NoData]);
}

/// Data loss outside an overload still breaks the trace, even when the
/// outage is shorter than the elapsed-time threshold — the App knows
/// samples are missing, so it doesn't have to be inferred.
#[test]
fn a_brief_dropout_without_an_overload_still_shows() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_data_loss();
    // Well under the 1 s threshold, so nothing would be inferred here.
    g.push(2.0, t0 + Duration::from_millis(200), "DC V", "V", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::NoData]);
}

/// While the link is up, OL samples keep arriving, so a long overload is
/// all band and no dropout — the case that must not regress.
#[test]
fn a_connected_overload_produces_no_dropout() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    // Ten seconds of overload, sampled throughout.
    for i in 1..=100 {
        g.push_break(t0 + Duration::from_millis(i * 100));
    }
    g.push(2.0, t0 + Duration::from_millis(10_100), "DC V", "V", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload]);
}

/// No minimum width is applied on the main plot: a brief excursion stays
/// sub-pixel and collapses to a line rather than being widened to
/// something legible, which would overstate how long the meter was over
/// range. With no dropout recorded the band covers the whole
/// interruption — the meter was over range across it, we simply don't
/// sample continuously.
#[test]
fn a_brief_overload_is_not_widened() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_micros(500));
    g.push(2.0, t0 + Duration::from_millis(1), "DC V", "V", None);

    let gaps = g.visible_gaps();
    assert_eq!(gaps.len(), 1, "half a millisecond is under the threshold");
    let (start, end, kind) = gaps[0];
    assert_eq!(kind, GapKind::Overload);
    let width = end - start;
    assert!(
        (width - 0.001).abs() < 1e-9,
        "band must span the real 1 ms interruption, got {width}"
    );
}

/// The minimap reads its bands from the same cache it draws the trace
/// from, so the cache has to carry gaps — an earlier version of that
/// field was write-only and was removed.
#[test]
fn the_cache_carries_gaps_for_the_minimap() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(50));
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);

    g.ensure_cache();
    assert_eq!(g.cached_segments.len(), 2, "trace splits either side");
    let kinds: Vec<GapKind> = g.cached_gaps.iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload]);
}

/// The cache is keyed on history_version; a new sample must invalidate it
/// or the minimap would keep drawing a stale set of bands.
#[test]
fn the_cache_refreshes_when_a_break_arrives() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.ensure_cache();
    assert!(g.cached_gaps.is_empty());

    g.push_break(t0 + Duration::from_millis(50));
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    g.ensure_cache();
    assert_eq!(g.cached_gaps.len(), 1);
}

/// An overload before any data has nothing to anchor to.
#[test]
fn a_break_with_no_history_marks_nothing() {
    let mut g = Graph::new();
    g.push_break(Instant::now());
    assert_eq!(g.pending_overload_span(), None);
}

/// The opening marker anchors to the last plotted point, and in live mode
/// the window used to end at that same point — so the marker landed on
/// the plot border and read as part of it. Overload samples carry
/// timestamps, so the window can follow them.
#[test]
fn the_view_follows_overload_samples() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);

    let (_, before) = g.view_bounds();
    assert!(before.abs() < 1e-9, "window ends at the only sample");

    // Two seconds of overload samples arrive, carrying timestamps.
    g.push_break(t0 + Duration::from_secs(2));
    let (_, during) = g.view_bounds();
    let (band_start, band_end) = g.pending_overload_span().expect("span while overloaded");
    assert!(
        during > band_start + 1.0,
        "view must advance past the band start: start={band_start}, x_max={during}"
    );
    assert!(
        (band_end - 2.0).abs() < 1e-9,
        "band closes at the newest overload sample, got {band_end}"
    );

    // Recovery closes the gap and hands the window back to the data.
    g.push(2.0, t0 + Duration::from_secs(3), "DC V", "V", None);
    assert_eq!(g.pending_overload_span(), None);
    let (_, after) = g.view_bounds();
    assert!((after - 3.0).abs() < 1e-9, "got {after}");
}

/// The minimap maps time to x from `data_time_range`, so if that ignored
/// overload samples the strip would stop advancing mid-excursion and the
/// band would have nowhere to grow — the same freeze the main plot had.
/// Fixed at the range itself so every consumer follows, not per call site.
#[test]
fn the_data_range_counts_overload_samples() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);

    let (_, before) = g.data_time_range();
    assert!(before.abs() < 1e-9, "only sample sits at the origin");

    g.push_break(t0 + Duration::from_secs(4));
    let (min, max) = g.data_time_range();
    assert!(min.abs() < 1e-9, "start is unaffected");
    assert!(
        (max - 4.0).abs() < 1e-9,
        "range must reach the newest overload sample, got {max}"
    );

    // And hands back to the plotted data once the meter recovers.
    g.push(2.0, t0 + Duration::from_secs(5), "DC V", "V", None);
    let (_, after) = g.data_time_range();
    assert!((after - 5.0).abs() < 1e-9, "got {after}");
}

/// A paused or disconnected meter produces no samples at all — not even
/// overload ones — so its window must hold still rather than scrolling
/// the data off screen. This is why the view follows sample timestamps
/// and not the wall clock.
#[test]
fn the_view_holds_still_without_samples() {
    let mut g = Graph::new();
    let t0 = Instant::now() - Duration::from_secs(5);
    g.push(1.0, t0, "DC V", "V", None);
    let (_, first) = g.view_bounds();
    let (_, second) = g.view_bounds();
    assert!(
        (first - second).abs() < 1e-9,
        "window drifted with no samples"
    );
}

/// Consecutive overload samples are one interruption, not several.
#[test]
fn repeated_overloads_produce_one_break() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    for _ in 0..5 {
        g.push_break(Instant::now());
    }
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    assert_eq!(g.all_segments().len(), 2);
    assert_eq!(g.visible_gaps().len(), 1);
}

/// The break must not persist past the point that consumed it.
#[test]
fn break_applies_only_to_the_next_point() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(Instant::now());
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    g.push(3.0, t0 + Duration::from_millis(200), "DC V", "V", None);
    let segments = g.all_segments();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[1].len(), 2, "samples after the break rejoin");
}

/// Clearing must drop a pending break, or the first point of the next
/// session would start orphaned.
#[test]
fn clear_discards_a_pending_break() {
    let mut g = Graph::new();
    g.push_break(Instant::now());
    g.clear();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    assert_eq!(g.all_segments().len(), 1);
}

/// Same mode and unit must not clear — otherwise the graph would reset on
/// every sample and never accumulate.
#[test]
fn steady_unit_keeps_history() {
    let mut g = Graph::new();
    for i in 0..5 {
        g.push(i as f64, Instant::now(), "DC V", "V", None);
    }
    assert_eq!(g.len(), 5);
}

#[test]
fn max_points_evicts_oldest() {
    let mut g = Graph::new();
    for i in 0..MAX_POINTS + 100 {
        g.push(i as f64, Instant::now(), "DC V", "V", None);
    }
    assert_eq!(g.len(), MAX_POINTS);
}

/// A signal sitting exactly on the reference value used to report a
/// crossing on every sample, painting a solid row of markers along the
/// reference line and hiding the trace underneath.
#[test]
fn flat_signal_on_the_reference_is_not_a_crossing() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..10 {
        g.push(5.0, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
    }
    assert!(g.find_crossings(&[5.0], 0.0, 100.0).is_empty());
}

/// The zero case matters just as much: open leads read 0.000 and Ref 0 is
/// a natural thing to set.
#[test]
fn flat_zero_signal_on_a_zero_reference_is_not_a_crossing() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..5 {
        g.push(0.0, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
    }
    assert!(g.find_crossings(&[0.0], 0.0, 100.0).is_empty());
}

#[test]
fn real_crossings_are_still_reported() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    // Rise through the threshold, then fall back through it.
    for (i, v) in [4.0, 6.0, 4.0].into_iter().enumerate() {
        g.push(
            v,
            t0 + Duration::from_millis(i as u64 * 100),
            "DC V",
            "V",
            None,
        );
    }
    assert_eq!(g.find_crossings(&[5.0], 0.0, 100.0).len(), 2);
}

/// Arriving exactly on the reference is a crossing — once, not once per
/// sample spent sitting there.
#[test]
fn touching_the_reference_marks_a_single_crossing() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, v) in [4.0, 5.0, 5.0, 5.0].into_iter().enumerate() {
        g.push(
            v,
            t0 + Duration::from_millis(i as u64 * 100),
            "DC V",
            "V",
            None,
        );
    }
    assert_eq!(g.find_crossings(&[5.0], 0.0, 100.0).len(), 1);
}

/// The minimap is a full-history overview, so it must scale to the data
/// even when the main plot's Y axis is pinned to a narrow band. Scaling by
/// the pin put points many multiples of the 60px strip's height away, and
/// they clipped to a flat line along its edges.
#[test]
fn minimap_scale_ignores_the_pinned_y_range() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, v) in [0.0, 5.0, 10.0].into_iter().enumerate() {
        g.push(
            v,
            t0 + Duration::from_millis(i as u64 * 100),
            "DC V",
            "V",
            None,
        );
    }
    // Pin a narrow band, as a Shift-drag box zoom does.
    g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
    assert!(g.y_axis_fixed);
    assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));

    let (lo, hi) = g.y_range_for_view_auto(0.0, 1.0, true).expect("auto range");
    assert!(
        lo <= 0.0 && hi >= 10.0,
        "minimap range {lo}..{hi} must cover the full 0..10 data span"
    );
}

/// A Y range pinned while measuring volts must not survive into ohms —
/// the new trace would sit far outside it and the plot would look empty.
#[test]
fn mode_change_releases_the_pinned_y_range() {
    let mut g = Graph::new();
    g.push(5.0, Instant::now(), "DC V", "V", None);
    g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
    assert!(g.y_axis_fixed);
    assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));

    g.push(1000.0, Instant::now(), "Ohm", "Ω", None);
    assert!(
        !g.y_axis_fixed,
        "mode change must release the fixed Y range"
    );
    assert!(!g.y_user_set);
    assert_ne!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));
}

/// Staying in the same mode must keep the user's zoom — otherwise every
/// incoming sample would fight the pinned view.
#[test]
fn same_mode_keeps_the_pinned_y_range() {
    let mut g = Graph::new();
    g.push(5.0, Instant::now(), "DC V", "V", None);
    g.apply_bbox_zoom((0.0, 5.1), (10.0, 4.9));
    g.push(5.05, Instant::now(), "DC V", "V", None);
    assert!(g.y_axis_fixed);
    assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));
}

#[test]
fn clear_resets_everything() {
    let mut g = Graph::new();
    g.push(5.0, Instant::now(), "DC V", "V", None);
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
    g.push(1.0, Instant::now(), "DC V", "V", None);
    g.push(2.0, Instant::now(), "DC V", "V", None);
    g.push(3.0, Instant::now(), "DC V", "V", None);
    let segments = g.all_segments();
    assert_eq!(segments.len(), 1);
}

#[test]
fn gap_detection() {
    let mut g = Graph::new();
    g.push(1.0, Instant::now(), "DC V", "V", None);
    g.push(2.0, Instant::now(), "DC V", "V", None);
    assert!(g.visible_gaps().is_empty());
}

#[test]
fn elapsed_secs_relative_to_origin() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_millis(50), "DC V", "V", None);
    let t = g.elapsed_secs(g.history.back().unwrap().time);
    assert!((t - 0.05).abs() < 1e-9);
}

#[test]
fn live_view_bounds_follow_latest() {
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    g.push(1.0, Instant::now(), "DC V", "V", None);
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
fn visible_stats_cover_only_the_visible_window() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, v) in [1.0, 2.0, 3.0, 10.0].iter().enumerate() {
        g.push(*v, t0 + Duration::from_secs(i as u64), "DC V", "V", None);
    }
    g.live = false;
    g.time_window_secs = 2.0;
    g.view_center = 2.5; // window [1.5, 3.5]: the 3.0 and 10.0 samples
    let s = g.visible_stats().expect("window holds two samples");
    assert_eq!(s.min, Some(3.0));
    assert_eq!(s.max, Some(10.0));
    assert_eq!(s.avg(), Some(6.5));
    assert_eq!(s.count, 2);
}

#[test]
fn visible_stats_none_when_window_holds_no_sample() {
    let mut g = Graph::new();
    assert!(g.visible_stats().is_none());
    g.push(1.0, Instant::now(), "DC V", "V", None);
    g.live = false;
    g.time_window_secs = 2.0;
    g.view_center = 100.0;
    assert!(g.visible_stats().is_none());
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
        g.push(i as f64, Instant::now(), "V DC", "V", None);
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
    g.push(1.0, Instant::now(), "V DC", "V", None);
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
fn visible_index_range_finds_correct_slice() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    // Push 10 points at 1-second intervals: t=0..9
    for i in 0..10 {
        g.push(i as f64, t0 + Duration::from_secs(i), "DC V", "V", None);
    }
    // Ask for points in [3.0, 6.0]
    let (start, end) = g.visible_index_range(3.0, 6.0);
    assert_eq!(start, 3);
    assert_eq!(end, 7); // half-open: indices 3,4,5,6

    // Empty range
    let (s, e) = g.visible_index_range(20.0, 30.0);
    assert_eq!(s, e);

    // Full range
    let (s, e) = g.visible_index_range(0.0, 100.0);
    assert_eq!(s, 0);
    assert_eq!(e, 10);
}

#[test]
fn nearest_point_binary_search() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    // Points at t=0, 1, 2, 3
    for i in 0..4 {
        g.push(
            (i * 10) as f64,
            t0 + Duration::from_secs(i),
            "DC V",
            "V",
            None,
        );
    }
    // Exact match
    let (pt, v) = g.nearest_point(2.0).unwrap();
    assert!((pt - 2.0).abs() < 0.01);
    assert!((v - 20.0).abs() < 0.01);
    // Between points — closer to t=1 (value=10) than t=2 (value=20)
    let (pt, v) = g.nearest_point(1.3).unwrap();
    assert!((pt - 1.0).abs() < 0.01);
    assert!((v - 10.0).abs() < 0.01);
    // Before all data
    let (pt, _) = g.nearest_point(-5.0).unwrap();
    assert!((pt - 0.0).abs() < 0.01);
    // After all data
    let (pt, _) = g.nearest_point(100.0).unwrap();
    assert!((pt - 3.0).abs() < 0.01);
}

#[test]
fn build_envelope_sliding_window_extrema() {
    // Place points one second apart so each subsequent sample is one
    // window step. With window=2.5s, the trailing window at each point
    // covers itself plus the previous two samples (gap of 2s ≤ 2.5s).
    let mut g = Graph::new();
    let t0 = Instant::now();
    let values = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
    for (i, &v) in values.iter().enumerate() {
        g.push(v, t0 + Duration::from_secs(i as u64), "DC V", "V", None);
    }

    let (min_pts, max_pts) = g.build_envelope(0.0, 7.0, 2.5);
    assert_eq!(min_pts.len(), values.len());
    assert_eq!(max_pts.len(), values.len());

    // Reference: brute-force the trailing window for every point.
    for i in 0..values.len() {
        let t = i as f64;
        let win_start = t - 2.5;
        let mut bf_min = f64::INFINITY;
        let mut bf_max = f64::NEG_INFINITY;
        for (j, &v) in values.iter().enumerate() {
            let tj = j as f64;
            if tj >= win_start && tj <= t {
                bf_min = bf_min.min(v);
                bf_max = bf_max.max(v);
            }
        }
        assert!(
            (min_pts[i][1] - bf_min).abs() < 1e-12,
            "min[{i}]={} expected {bf_min}",
            min_pts[i][1]
        );
        assert!(
            (max_pts[i][1] - bf_max).abs() < 1e-12,
            "max[{i}]={} expected {bf_max}",
            max_pts[i][1]
        );
    }
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
    g.push(1.0, Instant::now(), "DC V", "V", None);
    assert!(g.live);
    // Any non-zero drag while live flips us to browse mode.
    g.apply_pan(2.0);
    assert!(!g.live);
}

#[test]
fn apply_pan_in_live_mode_snaps_view_center_to_end() {
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    g.push(1.0, Instant::now(), "DC V", "V", None);
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

// ── Sub-value overlays ──────────────────────────────────────────────

/// Push a sample carrying sub-values. Everything but the parts under
/// test matches the single-series `push` helper.
fn push_aux(
    g: &mut Graph,
    value: f64,
    t: Instant,
    series: Option<&str>,
    overlays: &[(&str, Option<f64>)],
) {
    g.push_sample(PlotSample {
        value,
        timestamp: t,
        mode: "Temp",
        unit: "\u{00B0}C",
        display_raw: None,
        series,
        overlays,
    });
}

/// The overlay buffers are indexed by history position, so every push has
/// to extend them by exactly one — including the pushes that arrive
/// before the sub-value is first seen.
#[test]
fn overlays_stay_in_lockstep_with_history() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_aux(
            &mut g,
            20.0 + i as f64,
            t0 + Duration::from_secs(i),
            None,
            &[("T2", Some(30.0 + i as f64))],
        );
    }
    assert_eq!(g.len(), 3);
    assert_eq!(
        g.overlay_values("T2"),
        vec![Some(30.0), Some(31.0), Some(32.0)]
    );
}

/// Eviction has to drop the overlay's oldest value with the point it
/// belongs to, or every overlay would drift one sample later than the
/// trace it accompanies.
#[test]
fn max_points_evicts_overlay_values_too() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..MAX_POINTS + 100 {
        push_aux(
            &mut g,
            i as f64,
            t0 + Duration::from_millis(i as u64 * 10),
            None,
            &[("T2", Some(i as f64 + 0.5))],
        );
    }
    let values = g.overlay_values("T2");
    assert_eq!(values.len(), MAX_POINTS);
    assert_eq!(g.len(), MAX_POINTS);
    assert_eq!(values.first().copied().flatten(), Some(100.5));
}

/// A COMP High/Low or a MIN/MAX sub-value can start mid-session. Without
/// the back-fill its first value would land at index 0 and the whole
/// trace would be drawn shifted back in time.
#[test]
fn a_late_overlay_is_back_filled_with_nothing() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 20.0, t0, None, &[]);
    push_aux(&mut g, 21.0, t0 + Duration::from_secs(1), None, &[]);
    push_aux(
        &mut g,
        22.0,
        t0 + Duration::from_secs(2),
        None,
        &[("Max", Some(22.0))],
    );
    assert_eq!(g.overlay_values("Max"), vec![None, None, Some(22.0)]);
    assert_eq!(g.overlay_segments("Max"), vec![vec![[2.0, 22.0]]]);
}

/// An over-range sub-value breaks its own trace and nothing else: the
/// plotted series still has a value for that frame, so it stays whole.
#[test]
fn a_missing_overlay_value_splits_only_that_overlay() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
    push_aux(
        &mut g,
        21.0,
        t0 + Duration::from_secs(1),
        None,
        &[("T2", None)],
    );
    push_aux(
        &mut g,
        22.0,
        t0 + Duration::from_secs(2),
        None,
        &[("T2", Some(32.0))],
    );

    assert_eq!(
        g.overlay_segments("T2"),
        vec![vec![[0.0, 30.0]], vec![[2.0, 32.0]]]
    );
    assert_eq!(g.all_segments().len(), 1, "main trace must stay whole");
    assert!(g.visible_gaps().is_empty());
}

/// Overlays have to split wherever the main trace splits, or they would
/// be drawn straight across an overload the meter reported.
#[test]
fn a_break_in_the_plotted_series_splits_the_overlays() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
    g.push_break(t0 + Duration::from_millis(500));
    push_aux(
        &mut g,
        22.0,
        t0 + Duration::from_secs(1),
        None,
        &[("T2", Some(32.0))],
    );
    assert_eq!(
        g.overlay_segments("T2"),
        vec![vec![[0.0, 30.0]], vec![[1.0, 32.0]]]
    );
}

/// T1 and T2 share a mode *and* a unit, so nothing but the series label
/// distinguishes them — switching would otherwise append onto the
/// previous sub-value's trace.
#[test]
fn a_series_change_restarts_the_trace() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 20.0, t0, None, &[("T2", Some(30.0))]);
    push_aux(
        &mut g,
        21.0,
        t0 + Duration::from_secs(1),
        None,
        &[("T2", Some(31.0))],
    );
    assert_eq!(g.len(), 2);

    push_aux(
        &mut g,
        31.5,
        t0 + Duration::from_secs(2),
        Some("T2"),
        &[("Main", Some(21.5))],
    );
    assert_eq!(g.len(), 1, "switching series must clear the history");
    assert_eq!(g.overlay_labels(), vec!["Main"]);
    assert_eq!(g.current_series.as_deref(), Some("T2"));
}

/// Same series, same mode, same unit: nothing to reset.
#[test]
fn a_steady_series_keeps_its_history() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 30.0, t0, Some("T2"), &[]);
    push_aux(&mut g, 31.0, t0 + Duration::from_secs(1), Some("T2"), &[]);
    assert_eq!(g.len(), 2);
}

/// The toolbar selection survives as long as the meter keeps offering
/// that sub-value, and falls back to the main reading once it has stopped
/// for long enough — the meter left the mode that produced it.
#[test]
fn a_selection_is_dropped_once_its_label_stays_unoffered() {
    let mut g = Graph::new();
    g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")]);
    g.selected_series = Some("T2".to_string());

    g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")]);
    assert_eq!(g.selected_series(), Some("T2"));

    // A short or bit-clear frame is not a mode change: the selection has
    // to outlast one on its own.
    for i in 1..SERIES_DROP_FRAMES {
        g.set_series_options(&[("Frequency", "Hz")]);
        assert_eq!(g.selected_series(), Some("T2"), "dropped after {i} frames");
    }
    g.set_series_options(&[("Frequency", "Hz")]);
    assert_eq!(g.selected_series(), None);
}

/// The count is of *consecutive* frames: one frame that offers the label
/// again means the meter never left the mode.
#[test]
fn a_reoffered_label_restarts_the_drop_count() {
    let mut g = Graph::new();
    g.set_series_options(&[("T2", "\u{00B0}C")]);
    g.selected_series = Some("T2".to_string());

    for _ in 1..SERIES_DROP_FRAMES {
        g.set_series_options(&[("Frequency", "Hz")]);
    }
    g.set_series_options(&[("T2", "\u{00B0}C")]);
    assert_eq!(g.selected_series(), Some("T2"));

    // Back to a full run: the near-miss above must not count towards it.
    for i in 1..SERIES_DROP_FRAMES {
        g.set_series_options(&[("Frequency", "Hz")]);
        assert_eq!(g.selected_series(), Some("T2"), "dropped after {i} frames");
    }
    g.set_series_options(&[("Frequency", "Hz")]);
    assert_eq!(g.selected_series(), None);
}

/// A single-display meter keeps the two-row toolbar: the series row only
/// appears once there is a sub-value to select or a trace to draw.
#[test]
fn the_series_row_appears_only_once_there_is_something_in_it() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(20.0, t0, "DC V", "V", None);
    assert!(!g.has_series_row());

    // A selectable sub-value alone is enough...
    g.set_series_options(&[("Frequency", "Hz")]);
    assert!(g.has_series_row());

    // ...and so is a same-unit trace with nothing to select.
    let mut g = Graph::new();
    push_aux(&mut g, 20.0, t0, None, &[("T2", Some(50.0))]);
    assert!(g.series_options.is_empty());
    assert!(g.has_series_row());
}

/// A sub-value drawn outside the auto Y range would be clipped to the
/// plot edge, which reads as a flat line rather than as data.
#[test]
fn the_y_range_frames_the_visible_overlays() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_aux(
            &mut g,
            20.0,
            t0 + Duration::from_secs(i),
            None,
            &[("T2", Some(50.0))],
        );
    }
    let (lo, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
    assert!(lo <= 20.0 && hi >= 50.0, "overlay not framed: {lo}..{hi}");

    // Off-view overlay points must not stretch the axis.
    let (lo, hi) = g.y_min_max_padded(0.0, 0.5, true).expect("range");
    assert!(
        hi < 60.0,
        "range {lo}..{hi} reached beyond the visible slice"
    );
}

/// The minimap is a main-series overview and scans the whole history, so
/// it must not multiply that scan by the overlay count.
#[test]
fn the_minimap_y_range_ignores_overlays() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_aux(
            &mut g,
            20.0,
            t0 + Duration::from_secs(i),
            None,
            &[("T2", Some(50.0))],
        );
    }
    let (lo, hi) = g.y_min_max_padded(0.0, 2.0, false).expect("range");
    assert!(hi < 30.0, "minimap range {lo}..{hi} followed the overlay");
}

/// The protocols send at most four sub-values per frame; anything beyond
/// that is a bug upstream and must not grow the buffers unbounded.
#[test]
fn a_fifth_overlay_label_is_ignored() {
    let mut g = Graph::new();
    push_aux(
        &mut g,
        20.0,
        Instant::now(),
        None,
        &[
            ("A", Some(1.0)),
            ("B", Some(2.0)),
            ("C", Some(3.0)),
            ("D", Some(4.0)),
            ("E", Some(5.0)),
        ],
    );
    assert_eq!(g.overlay_labels(), vec!["A", "B", "C", "D"]);
}

// ── Plot key and Show: toggles ──────────────────────────────────────

/// The two chip rows show the same sub-value names, so their accessible
/// names have to say which group they belong to — otherwise a screen
/// reader announces the **Plot:** T2 chip and the **Show:** T2 chip
/// identically and the user cannot tell what a press will do.
#[test]
fn chip_labels_name_their_group() {
    assert_eq!(series_chip_label(None), "Plot main reading");
    assert_eq!(series_chip_label(Some("T2")), "Plot T2");
    assert_eq!(overlay_chip_label("T2"), "Show T2 trace");
    assert_ne!(series_chip_label(Some("T2")), overlay_chip_label("T2"));
}

/// Names in the key, in the order they are painted.
fn key_names(g: &Graph) -> Vec<String> {
    let drawn = g.visible_overlay_traces(0, g.len());
    g.key_entries(&drawn)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// Labels of the traces that are actually drawn.
fn drawn_overlay_labels(g: &Graph) -> Vec<String> {
    g.visible_overlay_traces(0, g.len())
        .into_iter()
        .map(|(_, label, _)| label)
        .collect()
}

/// Fill a graph with a main trace plus two same-unit sub-values.
fn graph_with_two_overlays() -> Graph {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_aux(
            &mut g,
            20.0,
            t0 + Duration::from_secs(i),
            None,
            &[("T2", Some(50.0)), ("T3", Some(60.0))],
        );
    }
    g
}

/// A single-display meter must look exactly as it did before sub-values
/// existed: no key painted over the plot at all.
#[test]
fn no_key_without_overlays() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        g.push(20.0, t0 + Duration::from_secs(i), "DC V", "V", None);
    }
    assert!(key_names(&g).is_empty());
}

/// The key names the plotted series first, then each drawn overlay in
/// the order the meter offered them.
#[test]
fn the_key_names_the_plotted_series_then_its_overlays() {
    let g = graph_with_two_overlays();
    assert_eq!(key_names(&g), vec!["Main", "T2", "T3"]);

    let drawn = g.visible_overlay_traces(0, g.len());
    let styles: Vec<KeyStyle> = g
        .key_entries(&drawn)
        .into_iter()
        .map(|(_, style)| style)
        .collect();
    assert_eq!(
        styles,
        vec![
            KeyStyle::Plotted,
            KeyStyle::Overlay(0),
            KeyStyle::Overlay(1)
        ]
    );
}

/// Hiding a trace from the Show: chips must take it off the plot, out of
/// the key, and out of the Y range — the axis stretching to frame a trace
/// that isn't drawn would flatten the one that is.
#[test]
fn hiding_an_overlay_drops_it_from_the_key_the_plot_and_the_y_range() {
    let mut g = graph_with_two_overlays();
    let (_, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
    assert!(hi >= 60.0, "both overlays framed to start with: {hi}");

    g.toggle_overlay_hidden("T3".to_string());

    assert_eq!(key_names(&g), vec!["Main", "T2"]);
    assert_eq!(drawn_overlay_labels(&g), vec!["T2"]);
    let (_, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
    assert!(hi < 60.0, "hidden overlay still stretching the axis: {hi}");
}

/// Hidden means not drawn, not not-recorded: turning a trace back on has
/// to bring its history with it. The chip flips both ways.
#[test]
fn a_hidden_overlay_is_still_recorded() {
    let mut g = graph_with_two_overlays();
    g.toggle_overlay_hidden("T3".to_string());
    assert_eq!(g.overlay_values("T3"), vec![Some(60.0); 3]);
    assert!(drawn_overlay_labels(&g).iter().all(|l| l != "T3"));

    g.toggle_overlay_hidden("T3".to_string());
    assert_eq!(g.overlay_segments("T3").len(), 1);
    assert_eq!(drawn_overlay_labels(&g), vec!["T2", "T3"]);
}

/// Every overlay hidden is the same as no overlay drawn: no key.
#[test]
fn hiding_every_overlay_removes_the_key() {
    let mut g = graph_with_two_overlays();
    g.hidden_overlays.insert("T2".to_string());
    g.hidden_overlays.insert("T3".to_string());
    assert!(key_names(&g).is_empty());
    assert!(drawn_overlay_labels(&g).is_empty());
}

/// Applying a scale hides `Raw` without a click, and does so on every
/// change of transform — so unlike the chip's toggle it must not flip a
/// trace back on when it is already hidden.
#[test]
fn hide_overlay_switches_a_trace_off_and_is_idempotent() {
    let mut g = graph_with_two_overlays();
    assert_eq!(drawn_overlay_labels(&g), vec!["T2", "T3"]);

    g.hide_overlay("T3");
    assert!(g.hidden_overlays.contains("T3"));
    assert_eq!(drawn_overlay_labels(&g), vec!["T2"]);

    g.hide_overlay("T3");
    assert_eq!(drawn_overlay_labels(&g), vec!["T2"], "still hidden");
}

/// The choice is about the sub-value, not about the buffer holding it:
/// clearing the data or switching the plotted series must not silently
/// bring a hidden trace back.
#[test]
fn hidden_overlays_survive_clear_and_a_series_change() {
    let mut g = graph_with_two_overlays();
    g.hidden_overlays.insert("T3".to_string());

    g.clear();
    assert!(g.hidden_overlays.contains("T3"));

    let t0 = Instant::now();
    push_aux(&mut g, 50.0, t0, Some("T2"), &[("T3", Some(60.0))]);
    assert!(g.hidden_overlays.contains("T3"));
    assert!(drawn_overlay_labels(&g).is_empty(), "T3 must stay hidden");
}

/// A sub-value can stop and start again (a COMP limit, a MIN/MAX reset).
/// The hidden set is keyed by label so the user's choice outlives the
/// buffer, rather than the trace reappearing the moment the meter re-sends
/// it.
#[test]
fn a_label_that_vanishes_and_returns_is_still_hidden() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 20.0, t0, None, &[("T2", Some(50.0))]);
    g.hidden_overlays.insert("T2".to_string());
    assert!(drawn_overlay_labels(&g).is_empty());

    // The meter leaves the mode: the graph resets and T2 goes away.
    g.push(1.0, t0 + Duration::from_secs(1), "DC V", "V", None);
    assert!(g.overlay_labels().is_empty());

    // It comes back later.
    push_aux(
        &mut g,
        20.0,
        t0 + Duration::from_secs(2),
        None,
        &[("T2", Some(50.0))],
    );
    assert_eq!(g.overlay_labels(), vec!["T2"]);
    assert!(
        drawn_overlay_labels(&g).is_empty(),
        "the user hid T2; it must not come back on its own"
    );
}

/// Ctrl+L clears the data, not the user's choice of what to plot — the
/// meter is still in the same mode and still sending the same sub-value.
#[test]
fn clear_drops_the_overlays_but_keeps_the_selection() {
    let mut g = Graph::new();
    g.set_series_options(&[("T2", "\u{00B0}C")]);
    g.selected_series = Some("T2".to_string());
    push_aux(
        &mut g,
        30.0,
        Instant::now(),
        Some("T2"),
        &[("Main", Some(20.0))],
    );
    assert_eq!(g.overlay_labels(), vec!["Main"]);

    g.clear();
    assert!(g.is_empty());
    assert!(g.overlay_labels().is_empty());
    assert_eq!(g.current_series, None);
    assert_eq!(g.selected_series(), Some("T2"));
}
