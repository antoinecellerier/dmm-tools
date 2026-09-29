use super::render::{
    KeyStyle, cursor_label_rect, layout_marker_flags, level_label_rects, quantize_for_hash,
    segment_hits_rect, stepped, thin_for_drawing, whole_number_marks,
};
use super::time::format_time_axis_label;
use super::toolbar::{overlay_chip_label, series_chip_label};
use super::*;
use crate::settings::{ColorPreset, PaletteOverrides};
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

/// An over-range frame as `Capture::ingest` hands it over: no value and no
/// sub-values, then the break.
fn push_overload(g: &mut Graph, t: Instant, mode: &str, unit: &str) {
    g.push_sample(PlotSample {
        value: None,
        timestamp: t,
        mode,
        unit,
        display_raw: None,
        series: None,
        main_label: None,
        levels: false,
        overlays: &[],
    });
    g.push_break(t);
}

/// Turning the dial from mV to Ω with the leads open gives only OL: the old
/// mode's trace must not carry on with an over-range band after it.
#[test]
fn an_overload_in_a_new_mode_restarts_the_trace() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(3.9, t0, "DC mV", "mV", None);
    g.push(3.9, t0 + Duration::from_millis(100), "DC mV", "mV", None);
    push_overload(&mut g, t0 + Duration::from_millis(200), "Ω", "MΩ");

    assert!(g.is_empty(), "the mV trace is gone");
    assert_eq!(g.current_mode.as_deref(), Some("Ω"));
    assert_eq!(g.pending_overload_span(), None, "no trace to band after");

    // The first reading starts the Ω trace with no break before it.
    g.push(1.0, t0 + Duration::from_millis(300), "Ω", "kΩ", None);
    assert_eq!(g.all_segments().len(), 1);
    assert!(g.visible_gaps().is_empty());
}

/// Lifting the leads off a resistor sends OL on the top range (kΩ to MΩ) in
/// the same mode: that is an excursion to band, not a new trace.
#[test]
fn an_overload_on_another_range_of_the_same_mode_keeps_the_trace() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "Ω", "kΩ", None);
    g.push(1.1, t0 + Duration::from_millis(100), "Ω", "kΩ", None);
    push_overload(&mut g, t0 + Duration::from_millis(200), "Ω", "MΩ");
    assert!(g.pending_overload_span().is_some());

    g.push(1.0, t0 + Duration::from_millis(300), "Ω", "kΩ", None);
    assert_eq!(g.all_segments().len(), 2, "the overload splits one trace");
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

/// A meter showing "Auto" with the probes lifted is neither over range nor
/// silent: the trace breaks, the stretch is a gap rather than a band, and the
/// live view still follows the samples arriving.
#[test]
fn a_no_reading_stretch_is_a_gap_not_a_band() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    for i in 1..=3 {
        g.push_no_reading(t0 + Duration::from_millis(i * 100));
    }
    assert!(
        (g.data_time_range().1 - 0.3).abs() < 1e-9,
        "the view follows the meter"
    );
    g.push(2.0, t0 + Duration::from_millis(400), "DC V", "V", None);

    let kinds: Vec<GapKind> = g.visible_gaps().iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::NoData]);
}

/// After an overload the band ends at the last OL sample: the word that
/// follows is not over range.
#[test]
fn a_no_reading_after_an_overload_closes_the_band() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(500));
    g.push_no_reading(t0 + Duration::from_millis(600));
    g.push(2.0, t0 + Duration::from_secs(1), "DC V", "V", None);

    let gaps = g.visible_gaps();
    let kinds: Vec<GapKind> = gaps.iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload, GapKind::NoData]);
    assert!((gaps[0].1 - 0.5).abs() < 1e-9, "band ends at the last OL");
}

/// The word that follows an overload still moves the live view, though the
/// band stays at the last OL sample.
#[test]
fn a_no_reading_after_an_overload_keeps_the_view_moving() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(500));
    g.push_no_reading(t0 + Duration::from_millis(600));
    g.push_no_reading(t0 + Duration::from_millis(700));

    let live_edge = g.data_time_range().1;
    assert!(
        (live_edge - 0.7).abs() < 1e-9,
        "the view follows the meter, got {live_edge}"
    );
    let (_, band_end) = g.pending_overload_span().expect("the band is still open");
    assert!(
        (band_end - 0.5).abs() < 1e-9,
        "band ends at the last OL, got {band_end}"
    );
}

/// An overload after a word (dashes while an inrush is awaited, then OL):
/// the word's stretch stays a gap and the band starts at the first OL
/// sample, both while the overload lasts and once a reading closes it.
#[test]
fn an_overload_after_a_no_reading_starts_the_band_at_the_first_ol() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "Inrush", "A", None);
    g.push_no_reading(t0 + Duration::from_millis(200));
    g.push_break(t0 + Duration::from_millis(500));
    g.push_break(t0 + Duration::from_millis(700));

    let (start, end) = g.pending_overload_span().expect("over range now");
    assert!(
        (start - 0.5).abs() < 1e-9,
        "band starts at the first OL, got {start}"
    );
    assert!((end - 0.7).abs() < 1e-9, "got {end}");

    g.push(2.0, t0 + Duration::from_secs(1), "Inrush", "A", None);
    let gaps = g.visible_gaps();
    let kinds: Vec<GapKind> = gaps.iter().map(|&(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::NoData, GapKind::Overload]);
    assert!((gaps[0].1 - 0.5).abs() < 1e-9, "gap ends at the first OL");
    assert!(
        (gaps[1].0 - 0.5).abs() < 1e-9,
        "band starts at the first OL"
    );
    assert!((gaps[1].1 - 1.0).abs() < 1e-9, "band ends at the reading");
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

/// The minimap reads its bands from the same level it draws the trace
/// from, so the level has to carry the gaps — an earlier version of that
/// field was write-only and was removed.
#[test]
fn the_level_carries_gaps_for_the_minimap() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_millis(50));
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);

    let mut level = g.build_level(0.01);
    assert_eq!(level.runs().count(), 2, "trace splits either side");
    let kinds: Vec<GapKind> = level.gaps().map(|(_, _, k)| k).collect();
    assert_eq!(kinds, vec![GapKind::Overload]);
}

/// The level is grown a sample at a time rather than rebuilt per frame, so
/// a break arriving after it was cut has to reach it — otherwise the
/// minimap would keep drawing the set of bands it was cut with.
#[test]
fn the_level_follows_a_break_as_it_arrives() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(1.5, t0 + Duration::from_millis(10), "DC V", "V", None);
    g.ensure_level(0.01);
    assert_eq!(g.minimap_level.as_ref().expect("cut").gaps().count(), 0);

    g.push_break(t0 + Duration::from_millis(50));
    g.push(2.0, t0 + Duration::from_millis(100), "DC V", "V", None);
    let mut level = g.minimap_level.clone().expect("still cut");
    assert_eq!(level.gaps().count(), 1);
    assert_eq!(level.runs().count(), 2);
    assert_eq!(g.minimap_level, Some(g.build_level(0.01)));
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
    const KEEP: usize = 100;
    let mut g = Graph::new();
    g.set_max_points(KEEP);
    for i in 0..KEEP + 100 {
        g.push(i as f64, Instant::now(), "DC V", "V", None);
    }
    assert_eq!(g.len(), KEEP);
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
    assert!((g.y_min.value() - 2.0).abs() < 1e-9);
    assert!((g.y_max.value() - 5.0).abs() < 1e-9);
    assert_eq!(g.y_min.text(), "2.0000");
    assert_eq!(g.y_max.text(), "5.0000");
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

/// A 1 m window past the first hour has 10 s grid marks: without the seconds
/// every mark in it read "23h 59m".
#[test]
fn time_axis_label_hour_keeps_whole_seconds() {
    assert_eq!(format_time_axis_label(86350.0, 10.0), "23h 59m 10s");
    assert_eq!(format_time_axis_label(86400.0, 10.0), "24h 0m");
}

/// Push an NCV level as `Capture::ingest` hands it over.
fn push_level(g: &mut Graph, level: u8, t: Instant) {
    g.push_sample(PlotSample {
        value: Some(f64::from(level)),
        timestamp: t,
        mode: "NCV",
        unit: "",
        display_raw: None,
        series: None,
        main_label: None,
        levels: true,
        overlays: &[],
    });
}

/// A level's grid steps by whole numbers however tall the plot, and the
/// axis formatter then writes them with no decimals.
#[test]
fn an_ncv_grid_marks_whole_numbers_only() {
    for base_step_size in [0.001, 0.03, 0.4, 1.0] {
        let marks = whole_number_marks(egui_plot::GridInput {
            bounds: (-0.4, 4.4),
            base_step_size,
        });
        let values: Vec<f64> = marks.iter().map(|m| m.value).collect();
        assert_eq!(values, [0.0, 1.0, 2.0, 3.0, 4.0], "at {base_step_size}");
        assert!(marks.iter().all(|m| m.step_size >= 1.0), "{marks:?}");
    }
}

/// A level holds until the next reading, then rises or falls at once.
#[test]
fn an_ncv_trace_is_drawn_as_steps() {
    assert_eq!(
        stepped(&[[0.0, 1.0], [0.1, 1.0], [0.2, 3.0], [0.3, 2.0]]),
        [
            [0.0, 1.0],
            [0.1, 1.0],
            [0.2, 1.0],
            [0.2, 3.0],
            [0.3, 3.0],
            [0.3, 2.0]
        ]
    );
    assert!(stepped(&[]).is_empty());
}

/// A steady 0 still gets an axis from 0 to 1, and a trace of levels always
/// takes 0 in; the readouts drop the decimals for a point, keep one for a
/// mean, and go back to four once the dial leaves NCV.
#[test]
fn an_ncv_trace_reads_as_whole_numbers_from_zero() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_level(&mut g, 0, t0 + Duration::from_millis(i * 100));
    }
    assert_eq!(
        g.y_min_max_padded(0.0, 1.0, true),
        Some(pad_range(0.0, 1.0))
    );
    for i in 3..6 {
        push_level(&mut g, 3, t0 + Duration::from_millis(i * 100));
    }
    assert_eq!(
        g.y_min_max_padded(0.3, 1.0, true),
        Some(pad_range(0.0, 3.0))
    );
    assert_eq!((g.decimals(true), g.decimals(false)), (0, 1));

    g.push(1.5, t0 + Duration::from_secs(1), "DC V", "V", None);
    assert_eq!(g.len(), 1, "the dial turn restarts the trace");
    assert_eq!((g.decimals(true), g.decimals(false)), (4, 4));
}

#[test]
fn time_axis_label_hour_subsecond_step() {
    // Unlikely in practice but the formatter should not drop the
    // seconds field when hours are involved and step is sub-second.
    let out = format_time_axis_label(3725.5, 0.1);
    assert_eq!(out, "1h 2m 5.5s");
}

/// The four corners `cursor_label_rect` picks from, for a 100x20 readout at
/// (300, 200) in a 500x400 plot: right-above, left-above, right-below,
/// left-below.
fn readout_corners() -> [egui::Rect; 4] {
    use egui::{Rect, pos2};
    [
        Rect::from_min_max(pos2(304.0, 178.0), pos2(404.0, 198.0)),
        Rect::from_min_max(pos2(196.0, 178.0), pos2(296.0, 198.0)),
        Rect::from_min_max(pos2(304.0, 202.0), pos2(404.0, 222.0)),
        Rect::from_min_max(pos2(196.0, 202.0), pos2(296.0, 222.0)),
    ]
}

fn readout_rect(plot_right: f32, hits: impl Fn(egui::Rect) -> bool) -> egui::Rect {
    readout_rect_around(plot_right, &[], hits).expect("a corner inside the plot")
}

fn readout_rect_around(
    plot_right: f32,
    taken: &[egui::Rect],
    hits: impl Fn(egui::Rect) -> bool,
) -> Option<egui::Rect> {
    let plot = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(plot_right, 400.0));
    cursor_label_rect(
        egui::pos2(300.0, 200.0),
        egui::vec2(100.0, 20.0),
        plot,
        taken,
        hits,
    )
}

#[test]
fn cursor_readout_takes_the_first_clear_corner_inside_the_plot() {
    let [right_above, left_above, right_below, left_below] = readout_corners();
    // Room on the right and no trace: right-above.
    assert_eq!(readout_rect(500.0, |_| false), right_above);
    // Against the plot's right edge, the right-hand corners are out.
    assert_eq!(readout_rect(400.0, |_| false), left_above);
    // Out on the right and the trace through left-above: left-below.
    assert_eq!(readout_rect(400.0, |r| r == left_above), left_below);
    // Trace through right-above only: left-above, before right-below.
    assert_eq!(readout_rect(500.0, |r| r == right_above), left_above);
    // Trace through both above: right-below.
    assert_eq!(
        readout_rect(500.0, |r| r == right_above || r == left_above),
        right_below
    );
}

#[test]
fn cursor_readout_falls_back_when_no_corner_is_clear() {
    let [_, left_above, ..] = readout_corners();
    // The trace everywhere: the first corner inside the plot.
    assert_eq!(readout_rect(400.0, |_| true), left_above);
    // A plot narrower than the readout: no readout.
    let narrow = egui::Rect::from_min_max(egui::pos2(250.0, 0.0), egui::pos2(350.0, 400.0));
    let rect = cursor_label_rect(
        egui::pos2(300.0, 200.0),
        egui::vec2(100.0, 20.0),
        narrow,
        &[],
        |_| false,
    );
    assert_eq!(rect, None);
}

#[test]
fn cursor_readout_keeps_off_labels_placed_before_it() {
    let [right_above, left_above, right_below, left_below] = readout_corners();
    // The other readout, a mean label or the key over right-above: left-above.
    let other = right_above.translate(egui::vec2(30.0, -5.0));
    assert_eq!(
        readout_rect_around(500.0, &[other], |_| false),
        Some(left_above)
    );
    // A flag under both below corners changes nothing above.
    let flag = right_below.union(left_below);
    assert_eq!(
        readout_rect_around(500.0, &[flag], |_| false),
        Some(right_above)
    );
    // A label counts before the trace: left-above is taken, so the corner on
    // the trace (left-below) wins over it.
    assert_eq!(
        readout_rect_around(400.0, &[left_above], |r| r == left_below),
        Some(left_below)
    );
    // All four taken, as by the other readout and a flag: the same corners a
    // row further out, above first.
    let near = right_above.union(left_below);
    let row = egui::vec2(0.0, 22.0);
    assert_eq!(
        readout_rect_around(400.0, &[near], |_| false),
        Some(left_above.translate(-row))
    );
    // Every row taken: no readout rather than one on top of another label.
    let all = near.expand2(egui::vec2(0.0, 3.0 * 22.0));
    assert_eq!(readout_rect_around(400.0, &[all], |_| false), None);
}

/// A 500x400 plot, and a 100x14 label, for the mean and reference labels.
const LEVEL_PLOT: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(500.0, 400.0));
const LEVEL_SIZE: egui::Vec2 = egui::vec2(100.0, 14.0);

/// A 100x14 label ending at `right` and `bottom`; the usual right edge is 496.
fn level_at(right: f32, bottom: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(right - 100.0, bottom - 14.0),
        egui::pos2(right, bottom),
    )
}

/// The column for lines at `ys`, top to bottom.
fn column(
    ys: &[f32],
    taken: &[egui::Rect],
    line_xs: &[f32],
    hits_trace: impl Fn(egui::Rect) -> bool,
) -> Vec<egui::Rect> {
    let sizes = vec![LEVEL_SIZE; ys.len()];
    level_label_rects(ys, &sizes, LEVEL_PLOT, taken, line_xs, hits_trace)
        .into_iter()
        .map(|r| r.expect("every label placed"))
        .collect()
}

fn plain_column(ys: &[f32]) -> Vec<egui::Rect> {
    column(ys, &[], &[], |_| false)
}

/// Whether the labels run top to bottom without touching.
fn stacked(rects: &[egui::Rect]) -> bool {
    rects.windows(2).all(|w| w[0].bottom() < w[1].top())
}

#[test]
fn level_labels_sit_above_their_lines_at_the_right_edge() {
    assert_eq!(plain_column(&[200.0]), [level_at(496.0, 198.0)]);
    // Lines well apart don't disturb each other.
    assert_eq!(
        plain_column(&[100.0, 200.0, 300.0]),
        [
            level_at(496.0, 98.0),
            level_at(496.0, 198.0),
            level_at(496.0, 298.0)
        ]
    );
}

#[test]
fn close_lines_get_one_label_above_them_and_the_rest_below() {
    // Four lines within 7 px, as a mean among three close references.
    let ys = [337.0, 338.0, 343.0, 344.0];
    assert_eq!(
        plain_column(&ys),
        [
            level_at(496.0, 335.0),
            level_at(496.0, 360.0),
            level_at(496.0, 376.0),
            level_at(496.0, 392.0)
        ]
    );
}

#[test]
fn the_column_keeps_off_the_trace() {
    let ys = [337.0, 338.0, 343.0, 344.0];
    // The trace runs under the lines: the column goes above them, bottom up.
    let rects = column(&ys, &[], &[], |r| r.bottom() > 340.0);
    assert!(stacked(&rects), "{rects:?}");
    assert_eq!(rects[3], level_at(496.0, 335.0));
    // A noisy trace through every column: the first, as with no trace, not
    // whichever it crosses least this frame.
    let crossed_less_above = |r: egui::Rect| r.top() > 300.0;
    assert_eq!(column(&ys, &[], &[], crossed_less_above), plain_column(&ys));
    // A lone label with the trace through its spot goes below its line.
    assert_eq!(
        column(&[200.0], &[], &[], |r| r == level_at(496.0, 198.0)),
        [level_at(496.0, 216.0)]
    );
}

#[test]
fn level_labels_at_the_plots_edges() {
    // No room above: below the line.
    assert_eq!(plain_column(&[10.0]), [level_at(496.0, 26.0)]);
    // Lines above the view are labelled under the top edge, stacked.
    assert_eq!(
        plain_column(&[-50.0, -40.0, -30.0]),
        [
            level_at(496.0, 16.0),
            level_at(496.0, 32.0),
            level_at(496.0, 48.0)
        ]
    );
    // Close lines at the bottom: no room below them, so the column is built
    // bottom up, all above them.
    let rects = plain_column(&[390.0, 392.0, 394.0]);
    assert!(stacked(&rects), "{rects:?}");
    assert_eq!(rects[2], level_at(496.0, 388.0));
}

#[test]
fn lines_too_close_to_keep_out_of_the_text_may_cross_it() {
    // Sixteen lines 4 px apart: a column clear of them all needs more room
    // than the plot has, so the labels stack through them, still in order.
    let ys: Vec<f32> = (0..16).map(|i| 230.0 + 4.0 * i as f32).collect();
    let rects = plain_column(&ys);
    assert!(stacked(&rects), "{rects:?}");
    let crossed = |r: &egui::Rect| ys.iter().any(|&y| r.top() < y && y < r.bottom());
    assert!(rects.iter().any(crossed));
}

#[test]
fn labels_with_no_room_are_left_out_rather_than_piled() {
    // Thirty labels, room for about twenty-four: the rest are dropped, and
    // those drawn don't touch.
    let sizes = vec![LEVEL_SIZE; 30];
    let rects = level_label_rects(&[200.0; 30], &sizes, LEVEL_PLOT, &[], &[], |_| false);
    let drawn: Vec<egui::Rect> = rects.iter().flatten().copied().collect();
    assert!(drawn.len() > 20 && drawn.len() < 30, "{}", drawn.len());
    for (i, a) in drawn.iter().enumerate() {
        for b in &drawn[i + 1..] {
            assert!(!a.intersects(*b), "{a:?} overlaps {b:?}");
        }
    }
    // A plot shorter than a label has room for none.
    let short = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(500.0, 10.0));
    let rects = level_label_rects(&[5.0], &[LEVEL_SIZE], short, &[], &[], |_| false);
    assert_eq!(rects, [None]);
}

#[test]
fn a_level_label_steps_left_of_the_lines_through_it() {
    // One line through the usual spot: just left of it, on its own line.
    assert_eq!(
        column(&[200.0], &[], &[450.0], |_| false),
        [level_at(446.0, 198.0)]
    );
    // Two lines close together: left of both, not between them.
    assert_eq!(
        column(&[200.0], &[], &[450.0, 420.0], |_| false),
        [level_at(416.0, 198.0)]
    );
    // A line clear of the label changes nothing.
    assert_eq!(
        column(&[200.0], &[], &[300.0], |_| false),
        [level_at(496.0, 198.0)]
    );
    // Four lines 90 px apart: each step lands on the next one, until the
    // label is left of them all.
    assert_eq!(
        column(&[200.0], &[], &[470.0, 380.0, 290.0, 200.0], |_| false),
        [level_at(196.0, 198.0)]
    );
    // Lines all the way across: it would leave the plot, so it stays at the
    // edge.
    let across: Vec<f32> = (0..6).map(|i| 470.0 - 90.0 * i as f32).collect();
    assert_eq!(
        column(&[200.0], &[], &across, |_| false),
        [level_at(496.0, 198.0)]
    );
}

#[test]
fn a_level_label_moves_past_a_readout_on_its_spot() {
    // A readout on the spot above the line: below the line instead.
    let readout = level_at(496.0, 198.0);
    assert_eq!(
        column(&[200.0], &[readout], &[], |_| false),
        [level_at(496.0, 216.0)]
    );
    // One ending a pixel short of the spot still counts: labels keep a gap.
    let flush = readout.translate(egui::vec2(-101.0, 0.0));
    assert_eq!(
        column(&[200.0], &[flush], &[], |_| false),
        [level_at(496.0, 216.0)]
    );
}

#[test]
fn segment_hits_rect_follows_the_segment_not_its_ends() {
    use egui::{Rect, pos2};
    let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(20.0, 20.0));
    // One end inside.
    assert!(segment_hits_rect(pos2(15.0, 15.0), pos2(40.0, 40.0), rect));
    // Both ends outside, straight through: a step edge crossing a readout.
    assert!(segment_hits_rect(pos2(15.0, 0.0), pos2(15.0, 30.0), rect));
    assert!(segment_hits_rect(pos2(0.0, 12.0), pos2(30.0, 18.0), rect));
    // Beside it, parallel to an edge.
    assert!(!segment_hits_rect(pos2(25.0, 0.0), pos2(25.0, 30.0), rect));
    // Diagonal that passes the corner.
    assert!(!segment_hits_rect(pos2(0.0, 5.0), pos2(30.0, -5.0), rect));
    // Short of it.
    assert!(!segment_hits_rect(pos2(0.0, 15.0), pos2(8.0, 15.0), rect));
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
    let overlays: Vec<_> = overlays.iter().map(|&(l, v)| (l, "\u{00B0}C", v)).collect();
    g.push_sample(PlotSample {
        value: Some(value),
        timestamp: t,
        mode: "Temp",
        unit: "\u{00B0}C",
        display_raw: None,
        series,
        main_label: None,
        levels: false,
        overlays: &overlays,
    });
}

/// A UT61E+ AC+DC V frame: the DC component as the plotted value, or
/// `None` for a frame carrying only its AC component beside it.
fn push_acdc(g: &mut Graph, t: Instant, dc: Option<f64>, ac: Option<f64>) {
    let overlays = [("AC", "V", ac)];
    g.push_sample(PlotSample {
        value: dc,
        timestamp: t,
        mode: "AC+DC V",
        unit: "V",
        display_raw: None,
        series: None,
        main_label: Some("DC"),
        levels: false,
        overlays: if ac.is_some() { &overlays } else { &[] },
    });
}

/// The same frames with the AC component plotted, as the App hands them
/// over once **Plot:** AC is picked: the DC reading drawn beside it.
fn push_acdc_plotting_ac(g: &mut Graph, t: Instant, dc: Option<f64>, ac: Option<f64>) {
    let overlays = [("DC", "V", dc)];
    g.push_sample(PlotSample {
        value: ac,
        timestamp: t,
        mode: "AC+DC V",
        unit: "V",
        display_raw: None,
        series: Some("AC"),
        main_label: Some("DC"),
        levels: false,
        overlays: if dc.is_some() { &overlays } else { &[] },
    });
}

/// Every point of a sub-value trace sits at the time of the frame that
/// carried it.
#[test]
fn overlay_points_keep_their_frame_times() {
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
        g.overlay_points("T2"),
        vec![(0.0, Some(30.0)), (1.0, Some(31.0)), (2.0, Some(32.0))]
    );
}

/// The UT61E+ in AC+DC V sends its DC and AC components in turn. Each has
/// to be drawn at its own frame's time as a trace of its own, unbroken by
/// the frames of the other.
#[test]
fn alternating_component_frames_draw_two_unbroken_traces() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.6112), None);
    push_acdc(&mut g, at(667), None, Some(0.0));
    push_acdc(&mut g, at(1334), Some(1.6111), None);
    push_acdc(&mut g, at(2000), None, Some(0.0022));

    assert_eq!(
        g.len(),
        2,
        "only the DC frames are points of the plotted series"
    );
    assert_eq!(g.all_segments(), vec![vec![[0.0, 1.6112], [1.334, 1.6111]]]);
    assert_eq!(
        g.overlay_segments("AC"),
        vec![vec![[0.667, 0.0], [2.0, 0.0022]]]
    );
    assert!(g.visible_gaps().is_empty());
    assert_eq!(
        key_names(&g),
        vec!["DC", "AC"],
        "the meter's name for its reading"
    );
}

/// Picking AC under **Plot:** keeps everything already drawn: the AC trace
/// becomes the plotted one and the DC one is drawn beside it, both with
/// their past points, and picking DC again swaps them back.
#[test]
fn switching_between_same_unit_series_keeps_their_past() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.6112), None);
    push_acdc(&mut g, at(667), None, Some(0.0));
    push_acdc(&mut g, at(1334), Some(1.6111), None);
    push_acdc(&mut g, at(2000), None, Some(0.0022));

    // The first frame after the switch is a DC one: nothing for AC, the DC
    // reading beside it.
    push_acdc_plotting_ac(&mut g, at(2667), Some(1.6110), None);
    assert_eq!(g.current_series.as_deref(), Some("AC"));
    assert_eq!(g.all_segments(), vec![vec![[0.667, 0.0], [2.0, 0.0022]]]);
    assert_eq!(
        g.overlay_segments("DC"),
        vec![vec![[0.0, 1.6112], [1.334, 1.6111], [2.667, 1.611]]]
    );
    assert_eq!(key_names(&g), vec!["AC", "DC"]);
    assert_eq!(g.origin, Some(t0), "the time axis stays where it was");

    push_acdc_plotting_ac(&mut g, at(3334), None, Some(0.0031));
    push_acdc(&mut g, at(4000), Some(1.6109), None);
    assert_eq!(g.current_series, None);
    assert_eq!(
        g.len(),
        4,
        "every DC point, before and after the round trip"
    );
    assert_eq!(
        g.overlay_values("AC"),
        vec![Some(0.0), Some(0.0022), Some(0.0031)]
    );
}

/// A break in the plotted series moves with it into the trace beside it,
/// and one in a sub-value's trace becomes a gap when it is plotted.
#[test]
fn a_swap_keeps_the_breaks_of_both_traces() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.0), Some(0.1));
    g.push_break(at(300));
    push_acdc(&mut g, at(600), Some(1.0), None);
    push_acdc(&mut g, at(700), None, None);
    g.push_sample(PlotSample {
        value: None,
        timestamp: at(800),
        mode: "AC+DC V",
        unit: "V",
        display_raw: None,
        series: None,
        main_label: Some("DC"),
        levels: false,
        overlays: &[("AC", "V", None)],
    });
    push_acdc(&mut g, at(900), None, Some(0.2));

    push_acdc_plotting_ac(&mut g, at(1000), None, Some(0.3));
    assert_eq!(
        g.overlay_segments("DC"),
        vec![vec![[0.0, 1.0]], vec![[0.6, 1.0]]]
    );
    assert_eq!(
        g.all_segments(),
        vec![vec![[0.0, 0.1]], vec![[0.9, 0.2], [1.0, 0.3]]]
    );
    assert_eq!(g.visible_gaps(), vec![(0.0, 0.9, GapKind::NoData)]);
}

/// A held meter sends one component only. The AC trace is drawn alone and
/// keeps the live window moving, and the key names only what is drawn.
#[test]
fn a_stream_of_overlay_only_frames_is_drawn_and_followed() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..4 {
        push_acdc(&mut g, t0 + Duration::from_secs(i), None, Some(0.0092));
    }
    assert!(g.is_empty());
    assert_eq!(g.overlay_segments("AC").len(), 1);
    assert_eq!(g.data_time_range(), (0.0, 3.0));
    assert_eq!(g.first_point_time(), Some(t0));
    assert_eq!(key_names(&g), vec!["AC"]);
    assert_eq!(
        g.y_min_max_padded(0.0, 3.0, true),
        Some(pad_range(0.0092, 0.0092))
    );
}

/// An overlay-only stream never grows the history, so the history's
/// eviction never trims it: it has a bound of its own.
#[test]
fn an_overlay_only_stream_is_bounded() {
    const KEEP: usize = 50;
    let mut g = Graph::new();
    g.set_max_points(KEEP);
    let t0 = Instant::now();
    for i in 0..KEEP as u64 * 3 {
        push_acdc(
            &mut g,
            t0 + Duration::from_millis(i * 10),
            None,
            Some(i as f64),
        );
    }
    let values = g.overlay_values("AC");
    assert_eq!(values.len(), KEEP);
    assert_eq!(values.first().copied().flatten(), Some(100.0));
}

/// A frame carrying only sub-values in a new mode is as much a restart as a
/// point of the plotted series would be, and an overlay point that came
/// before the first plotted one survives that one being pushed.
#[test]
fn an_overlay_only_frame_restarts_a_new_mode_and_keeps_its_point() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(5.0, t0, "DC V", "V", None);
    push_acdc(&mut g, t0 + Duration::from_secs(1), None, Some(0.0));
    assert!(g.is_empty(), "the DC V trace is gone");
    assert_eq!(g.current_mode.as_deref(), Some("AC+DC V"));

    push_acdc(&mut g, t0 + Duration::from_secs(2), Some(1.6), None);
    assert_eq!(g.overlay_points("AC"), vec![(0.0, Some(0.0))]);
    assert_eq!(g.first_point_time(), Some(t0 + Duration::from_secs(1)));
}

/// A frame carrying only sub-values leaves an open over-range band on the
/// plotted series alone: the band closes at the next plotted point, not at
/// the other component's frame.
#[test]
fn an_overlay_only_frame_leaves_an_open_break_alone() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.0), None);
    g.push_break(at(500));
    push_acdc(&mut g, at(700), None, Some(0.1));
    assert_eq!(g.pending_break, Some(GapKind::Overload));
    push_acdc(&mut g, at(900), Some(1.0), None);
    assert_eq!(g.visible_gaps(), vec![(0.0, 0.9, GapKind::Overload)]);
}

/// An overlay breaks where it has no points for longer than the gap
/// threshold, as the plotted series does.
#[test]
fn an_overlay_breaks_across_a_long_silence() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_acdc(&mut g, t0, None, Some(0.1));
    push_acdc(&mut g, t0 + Duration::from_millis(500), None, Some(0.2));
    push_acdc(&mut g, t0 + Duration::from_secs(5), None, Some(0.3));
    assert_eq!(
        g.overlay_segments("AC"),
        vec![vec![[0.0, 0.1], [0.5, 0.2]], vec![[5.0, 0.3]]]
    );
}

/// A sub-value that stops while frames keep coming — REL switched off for a
/// while — breaks its trace rather than being drawn straight across the
/// stretch it was not sent in.
#[test]
fn an_overlay_that_stops_and_resumes_breaks() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_aux(&mut g, 20.0, at(0), None, &[("Reference", Some(10.0))]);
    for i in 1..20 {
        push_aux(&mut g, 20.0, at(i * 250), None, &[]);
    }
    push_aux(&mut g, 20.0, at(5000), None, &[("Reference", Some(12.0))]);
    assert_eq!(
        g.overlay_segments("Reference"),
        vec![vec![[0.0, 10.0]], vec![[5.0, 12.0]]]
    );
    assert_eq!(g.all_segments().len(), 1, "the plotted trace went on");
}

/// HOLD on the UT61E+'s AC component sends AC frames only. When DC comes
/// back its trace breaks, so neither the line nor the cursor integral runs
/// across the held stretch.
#[test]
fn the_plotted_series_breaks_across_a_run_without_it() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.6), None);
    for i in 1..=15 {
        push_acdc(&mut g, at(i * 667), None, Some(0.0092));
    }
    push_acdc(&mut g, at(16 * 667), Some(1.6), None);
    assert_eq!(g.all_segments().len(), 2);
    assert_eq!(g.visible_gaps(), vec![(0.0, 10.672, GapKind::NoData)]);
    assert_eq!(g.cursor_integral(0.0, 11.0), None);
    assert_eq!(g.overlay_segments("AC").len(), 1, "AC went on throughout");
}

/// The Buffer size estimate charges this much per overlay point.
#[test]
fn an_overlay_point_fits_its_memory_estimate() {
    assert!(std::mem::size_of::<OverlayPoint>() <= crate::settings::GRAPH_BYTES_PER_OVERLAY_POINT);
}

/// Eviction has to drop the overlay's oldest value with the point it
/// belongs to, or every overlay would drift one sample later than the
/// trace it accompanies.
#[test]
fn max_points_evicts_overlay_values_too() {
    const KEEP: usize = 100;
    let mut g = Graph::new();
    g.set_max_points(KEEP);
    let t0 = Instant::now();
    for i in 0..KEEP + 100 {
        push_aux(
            &mut g,
            i as f64,
            t0 + Duration::from_millis(i as u64 * 10),
            None,
            &[("T2", Some(i as f64 + 0.5))],
        );
    }
    let values = g.overlay_values("T2");
    assert_eq!(values.len(), KEEP);
    assert_eq!(g.len(), KEEP);
    assert_eq!(values.first().copied().flatten(), Some(100.5));
}

/// A COMP High/Low or a MIN/MAX sub-value can start mid-session. Its trace
/// begins at its first point, at that frame's time — not back at the start
/// of the plotted series.
#[test]
fn a_late_overlay_starts_at_its_first_point() {
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
    assert_eq!(g.overlay_points("Max"), vec![(2.0, Some(22.0))]);
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

/// An over-range plotted series says nothing about a sub-value beside it:
/// the plotted trace breaks, the sub-value's carries on. A lost link breaks
/// both — nothing is known about either across it.
#[test]
fn only_a_lost_link_splits_the_overlays_with_the_plotted_series() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_aux(&mut g, 20.0, at(0), None, &[("T2", Some(30.0))]);
    g.push_break(at(300));
    push_aux(&mut g, 22.0, at(600), None, &[("T2", Some(32.0))]);
    assert_eq!(g.all_segments().len(), 2);
    assert_eq!(
        g.overlay_segments("T2"),
        vec![vec![[0.0, 30.0], [0.6, 32.0]]]
    );

    g.push_data_loss();
    push_aux(&mut g, 23.0, at(900), None, &[("T2", Some(33.0))]);
    assert_eq!(g.all_segments().len(), 3);
    assert_eq!(
        g.overlay_segments("T2"),
        vec![vec![[0.0, 30.0], [0.6, 32.0]], vec![[0.9, 33.0]]]
    );
    g.push_data_loss();
    g.push_data_loss();
    assert_eq!(
        g.overlay_values("T2")
            .iter()
            .filter(|v| v.is_none())
            .count(),
        2,
        "one break per loss, however often it is reported"
    );
}

/// T1 and T2 share a mode *and* a unit, so nothing but the series label
/// distinguishes them. Switching to T2 plots its own past points, drawn
/// beside T1 until then, rather than appending onto T1's trace.
#[test]
fn a_same_unit_series_change_swaps_the_traces() {
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

    push_aux(
        &mut g,
        31.5,
        t0 + Duration::from_secs(2),
        Some("T2"),
        &[("Main", Some(21.5))],
    );
    assert_eq!(g.current_series.as_deref(), Some("T2"));
    assert_eq!(g.len(), 3, "T2's past points and the new one");
    assert_eq!(g.overlay_labels(), vec!["Main"]);
    assert_eq!(
        g.overlay_values("Main"),
        vec![Some(20.0), Some(21.0), Some(21.5)]
    );
}

/// With no reading coming (paused, a replay that has ended), a **Plot:**
/// choice kept beside the plotted series takes over at once, and the next
/// sample goes on from it; back to the main reading swaps back the same way.
#[test]
fn a_kept_series_is_plotted_at_the_click() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_acdc(&mut g, t0, Some(1.61), Some(0.02));
    push_acdc(&mut g, t0 + Duration::from_secs(1), Some(1.60), Some(0.03));

    g.selected_series = Some("AC".to_string());
    assert!(g.swap_to_selected());
    assert_eq!(g.current_series.as_deref(), Some("AC"));
    assert_eq!(g.len(), 2, "AC's own points, with nothing pushed");
    assert_eq!(g.overlay_labels(), vec!["DC"]);

    g.selected_series = None;
    assert!(g.swap_to_selected());
    assert_eq!(g.current_series, None);
    assert!(!g.swap_to_selected(), "already plotted");

    g.selected_series = Some("T2".to_string());
    assert!(!g.swap_to_selected(), "nothing kept to swap in");
}

/// A UT181A V AC + Hz frame as the App hands it over: the voltage plotted
/// with the frequency and period kept beside it in their own units, or,
/// with `series` Frequency, the frequency plotted and the voltage kept.
fn push_vac_hz(g: &mut Graph, t: Instant, series: Option<&str>, v: f64, hz: (f64, &str)) {
    let (value, unit, overlays) = match series {
        None => (
            v,
            "V",
            vec![
                ("Frequency", hz.1, Some(hz.0)),
                ("Period", "ms", Some(20.0)),
            ],
        ),
        Some(_) => (
            hz.0,
            hz.1,
            vec![("Period", "ms", Some(20.0)), ("Main", "V", Some(v))],
        ),
    };
    g.push_sample(PlotSample {
        value: Some(value),
        timestamp: t,
        mode: "V AC Hz",
        unit,
        display_raw: None,
        series,
        main_label: None,
        levels: false,
        overlays: &overlays,
    });
}

/// Frequency is kept beside the voltage although it is not drawn, so
/// picking it swaps the two traces like a same-unit switch: both keep their
/// past, and the Y axis moves to Hz, dropping a range pinned for volts.
#[test]
fn a_series_change_to_another_unit_swaps_the_traces() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |s: u64| t0 + Duration::from_secs(s);
    for i in 0..3 {
        push_vac_hz(&mut g, at(i), None, 230.0 + i as f64, (50.0, "Hz"));
    }
    assert!(key_names(&g).is_empty(), "Hz and ms are kept, not drawn");
    g.y_axis_fixed = true;
    g.y_user_set = true;

    push_vac_hz(&mut g, at(3), Some("Frequency"), 233.0, (50.1, "Hz"));
    assert_eq!(g.current_series.as_deref(), Some("Frequency"));
    assert_eq!(g.plotted_unit(), "Hz");
    assert_eq!(
        g.all_segments(),
        vec![vec![[0.0, 50.0], [1.0, 50.0], [2.0, 50.0], [3.0, 50.1]]]
    );
    assert_eq!(
        g.overlay_values("Main"),
        vec![Some(230.0), Some(231.0), Some(232.0), Some(233.0)]
    );
    assert!(key_names(&g).is_empty(), "the volts are not drawn on Hz");
    assert_eq!(g.origin, Some(t0), "the time axis stays where it was");
    assert!(
        !g.y_axis_fixed && !g.y_user_set,
        "a volt range means nothing in Hz"
    );

    push_vac_hz(&mut g, at(4), None, 234.0, (50.2, "Hz"));
    assert_eq!(g.plotted_unit(), "V");
    assert_eq!(g.len(), 5, "every voltage point, before and after");
    assert_eq!(g.overlay_values("Frequency").len(), 5);
}

/// A kept trace whose unit steps (a frequency auto-ranging Hz→kHz) starts
/// over alone; the others and the plotted series keep their past. Switching
/// on the very frame it steps swaps onto the restarted trace rather than
/// clearing everything.
#[test]
fn a_kept_trace_restarts_alone_on_its_own_unit_change() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |s: u64| t0 + Duration::from_secs(s);
    for i in 0..3 {
        push_vac_hz(&mut g, at(i), None, 230.0, (999.0, "Hz"));
    }
    push_vac_hz(&mut g, at(3), None, 230.0, (1.001, "kHz"));
    assert_eq!(g.len(), 4);
    assert_eq!(g.overlay_values("Frequency"), vec![Some(1.001)]);
    assert_eq!(g.overlay_values("Period").len(), 4);

    let mut g = Graph::new();
    for i in 0..3 {
        push_vac_hz(&mut g, at(i), None, 230.0, (999.0, "Hz"));
    }
    push_vac_hz(&mut g, at(3), Some("Frequency"), 230.0, (1.001, "kHz"));
    assert_eq!(g.plotted_unit(), "kHz");
    assert_eq!(g.len(), 1, "only the restarted frequency");
    assert_eq!(g.overlay_values("Main").len(), 4, "the voltage is kept");
    assert_eq!(g.origin, Some(t0));
}

/// A held UT61E+ in AC+DC V sends only its AC component. A switch to AC
/// partway through keeps the DC trace's count of frames without it, so DC
/// still breaks across the hold instead of being drawn straight over it.
#[test]
fn a_swap_hands_each_trace_its_run_of_missing_frames() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_acdc(&mut g, at(0), Some(1.0), Some(0.1));
    push_acdc(&mut g, at(300), None, Some(0.1));
    push_acdc(&mut g, at(600), None, Some(0.1));
    push_acdc_plotting_ac(&mut g, at(900), None, Some(0.1));
    push_acdc_plotting_ac(&mut g, at(1200), None, Some(0.1));
    push_acdc_plotting_ac(&mut g, at(1500), Some(1.0), Some(0.1));
    assert_eq!(
        g.overlay_segments("DC"),
        vec![vec![[0.0, 1.0]], vec![[1.5, 1.0]]],
        "DC stopped for the hold"
    );
}

/// A trace kept in another unit is not drawn, so it must not stretch the
/// auto Y range: a 50 Hz frequency beside a 230 V reading.
#[test]
fn a_kept_trace_in_another_unit_leaves_the_y_range_alone() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_vac_hz(
            &mut g,
            t0 + Duration::from_secs(i),
            None,
            230.0 + i as f64,
            (50.0, "Hz"),
        );
    }
    let (lo, _) = g
        .y_min_max_padded(f64::NEG_INFINITY, f64::INFINITY, true)
        .expect("a range");
    assert!(lo > 200.0, "got {lo}");
}

/// Kept traces fill the cap as they come, but must not keep a trace in the
/// plotted unit off the plot: MIN/MAX's Min, arriving with Frequency and
/// Period kept and Max and Average already in, takes an undrawn slot.
#[test]
fn a_drawn_sub_value_takes_the_slot_of_an_undrawn_one() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_vac_hz(&mut g, t0, None, 230.0, (50.0, "Hz"));
    g.push_sample(PlotSample {
        value: Some(230.0),
        timestamp: t0 + Duration::from_secs(1),
        mode: "V AC Hz",
        unit: "V",
        display_raw: None,
        series: None,
        main_label: None,
        levels: false,
        overlays: &[
            ("Max", "V", Some(231.0)),
            ("Average", "V", Some(230.0)),
            ("Min", "V", Some(229.0)),
        ],
    });
    assert_eq!(g.overlays_len(), MAX_OVERLAYS);
    let mut drawn = drawn_overlay_labels(&g);
    drawn.sort();
    assert_eq!(drawn, vec!["Average", "Max", "Min"]);
}

/// Colours and line styles go to the drawn traces in turn: the first one
/// drawn gets the first style even with traces kept in another unit ahead
/// of it.
#[test]
fn the_overlay_palette_counts_drawn_traces_only() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_vac_hz(&mut g, t0, None, 230.0, (50.0, "Hz"));
    g.push_sample(PlotSample {
        value: Some(230.0),
        timestamp: t0 + Duration::from_secs(1),
        mode: "V AC Hz",
        unit: "V",
        display_raw: None,
        series: None,
        main_label: None,
        levels: false,
        overlays: &[("Max", "V", Some(231.0))],
    });
    let drawn = g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY);
    let slots: Vec<_> = drawn
        .iter()
        .map(|(k, label, _)| (*k, label.as_str()))
        .collect();
    assert_eq!(slots, vec![(0, "Max")]);
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
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")], at(0));
    g.selected_series = Some("T2".to_string());

    g.set_series_options(&[("T1", "\u{00B0}C"), ("T2", "\u{00B0}C")], at(100));
    assert_eq!(g.selected_series(), Some("T2"));

    // A short or bit-clear frame is not a mode change: the selection has
    // to outlast one on its own, and a run of them shorter than the gap
    // threshold too.
    for i in 1..=SERIES_DROP_FRAMES as u64 {
        g.set_series_options(&[("Frequency", "Hz")], at(100 + i * 100));
        assert_eq!(g.selected_series(), Some("T2"), "dropped after {i} frames");
    }
    g.set_series_options(&[("Frequency", "Hz")], at(1200));
    assert_eq!(g.selected_series(), None);
    assert_eq!(g.series_option_labels(), vec!["Frequency"]);
}

/// The count is of *consecutive* frames: one frame that offers the label
/// again means the meter never left the mode.
#[test]
fn a_reoffered_label_restarts_the_drop_count() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    g.set_series_options(&[("T2", "\u{00B0}C")], at(0));
    g.selected_series = Some("T2".to_string());

    for i in 1..SERIES_DROP_FRAMES as u64 {
        g.set_series_options(&[("Frequency", "Hz")], at(i * 1000));
    }
    g.set_series_options(&[("T2", "\u{00B0}C")], at(3000));
    assert_eq!(g.selected_series(), Some("T2"));

    // Back to a full run: the near-miss above must not count towards it.
    for i in 1..SERIES_DROP_FRAMES as u64 {
        g.set_series_options(&[("Frequency", "Hz")], at(3000 + i * 1000));
        assert_eq!(g.selected_series(), Some("T2"), "dropped after {i} frames");
    }
    g.set_series_options(&[("Frequency", "Hz")], at(6000));
    assert_eq!(g.selected_series(), None);
}

/// The UT61E+ in AC+DC V sends its AC component every other frame, and
/// slower polling can put several DC frames in a row between two. The
/// option — and a selection of it — has to outlast those frames, or the
/// chips would come and go and the selection would be dropped.
#[test]
fn an_option_sent_every_other_frame_or_less_stays_offered() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    g.set_series_options(&[("AC", "V")], at(0));
    g.selected_series = Some("AC".to_string());
    for i in 1..=6 {
        g.set_series_options(&[], at(i * 150));
        assert_eq!(g.series_option_labels(), vec!["AC"], "frame {i}");
    }
    assert_eq!(g.selected_series_offer(), Some(("AC", "V")));
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
    g.set_series_options(&[("Frequency", "Hz")], t0);
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
    let drawn = g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY);
    g.key_entries(&drawn, !g.all_segments().is_empty())
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// Labels of the traces that are actually drawn.
fn drawn_overlay_labels(g: &Graph) -> Vec<String> {
    g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY)
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

    let drawn = g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY);
    let styles: Vec<KeyStyle> = g
        .key_entries(&drawn, true)
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
    g.set_series_options(&[("T2", "\u{00B0}C")], Instant::now());
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

// ── Minimap coordinate math ─────────────────────────────────────────────────

use super::minimap::{
    Edge, MinimapDrag, MinimapScale, ViewWindow, bucket_secs, decimate_columns, drag_target,
    level_polyline, near_a_bracket, pan, resize,
};

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
    let mut extents: std::collections::HashMap<i32, (f32, f32)> = std::collections::HashMap::new();
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

// ── Minimap bucket level ────────────────────────────────────────────────────

use super::view::pad_range;

/// A deterministic LCG. The level's contract — appending equals rebuilding —
/// only shows up over a long, messy stream, and a flaky one would be useless.
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// The whole design rests on this: a level grown one sample at a time must be
/// indistinguishable from one cut in a single pass over the same history. The
/// stream mixes steady cadence, silences past the gap threshold, overloads,
/// dropouts and a repeated timestamp, and runs long enough to evict.
#[test]
fn incremental_level_matches_a_rebuild() {
    const WIDTH: f64 = 0.05;
    const KEEP: usize = 1_000;
    let mut g = Graph::new();
    // Small enough that the run evicts thousands of times without buffering
    // the default half a million points.
    g.set_max_points(KEEP);
    let t0 = Instant::now();
    let mut t = t0;
    let mut rng = 0x1234_5678_9abc_def0_u64;

    g.push(0.0, t, "DC V", "V", None);
    g.ensure_level(WIDTH);

    for i in 0..12_000_u64 {
        let r = lcg(&mut rng);
        let step_ms = match r % 100 {
            // A silence well past the 1 s threshold: a dropout.
            0 => 1_500 + r % 500,
            // Two samples stamped alike — the meter's clock has finite
            // resolution and the bucket must not care.
            1 => 0,
            _ => 90 + r % 20,
        };
        t += Duration::from_millis(step_ms);
        match r % 211 {
            7 => g.push_break(t),
            13 => {
                g.push_break(t);
                g.push_data_loss();
            }
            _ => {}
        }
        let v = (i as f64 * 0.017).sin() * 10.0 + (r % 1_000) as f64 * 0.001;
        g.push(v, t, "DC V", "V", None);

        if i % 97 == 0 {
            assert_eq!(
                g.minimap_level,
                Some(g.build_level(WIDTH)),
                "level diverged from a rebuild after {i} samples"
            );
        }
    }
    assert!(g.len() == KEEP, "the run must have evicted");
    assert_eq!(g.minimap_level, Some(g.build_level(WIDTH)));
}

/// The level exists to draw exactly what projecting every raw point drew, for
/// a cost that follows the strip's width instead of the history length. Both
/// the polylines and the Y range the strip scales them by must come out
/// identical to the full-history path they replaced.
#[test]
fn level_polylines_match_the_raw_point_path() {
    const WIDTH: f64 = 0.5;
    let mut g = Graph::new();
    let t0 = Instant::now();
    let mut rng = 0xfeed_face_dead_beef_u64;
    let mut t = t0;
    for i in 0..2_000_u64 {
        let r = lcg(&mut rng);
        t += Duration::from_millis(if r.is_multiple_of(97) {
            1_800
        } else {
            90 + r % 20
        });
        if r.is_multiple_of(173) {
            g.push_break(t);
        }
        g.push(
            (i as f64 * 0.031).sin() * 5.0 + (r % 500) as f64 * 0.002,
            t,
            "DC V",
            "V",
            None,
        );
    }

    // The Y range: folded bucket extremes against the scan over every point.
    let mut level = g.build_level(WIDTH);
    let (data_min, data_max) = g.data_time_range();
    let padded = level.value_range().map(|(lo, hi)| pad_range(lo, hi));
    assert_eq!(padded, g.y_min_max_padded(data_min, data_max, false));

    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 5.0), egui::vec2(400.0, 60.0));
    let y_map = padded.map(|(lo, hi)| (lo, (hi - lo).max(1e-10)));
    let y_of = |v: f64| -> f32 {
        let y_frac = match y_map {
            Some((lo, range)) => ((v - lo) / range) as f32,
            None => 0.5,
        };
        rect.bottom() - y_frac * rect.height()
    };

    // The path as it was before the level: every point of every segment
    // projected and decimated.
    let oracle: Vec<Vec<egui::Pos2>> = g
        .build_segments_for_range(0, g.len())
        .0
        .iter()
        .map(|seg| {
            decimate_columns(
                seg.iter()
                    .map(|&[t, v]| egui::pos2((t / WIDTH) as f32, y_of(v))),
                1.0,
            )
        })
        .collect();
    let drawn: Vec<Vec<egui::Pos2>> = level.runs().map(|run| level_polyline(run, y_of)).collect();
    assert_eq!(drawn, oracle);
}

/// Eviction has to leave the level exactly where a rebuild of what remains
/// would: the front bucket rescanned when the sample that left was one of its
/// extremes, and a band dropped once the point that opened it is gone.
#[test]
fn eviction_trims_buckets_and_gaps_exactly() {
    const WIDTH: f64 = 0.05;
    const KEEP: usize = 500;
    let mut g = Graph::new();
    g.set_max_points(KEEP);
    let t0 = Instant::now();
    let at = |i: u64| t0 + Duration::from_millis(i * 10);

    for i in 0..KEEP as u64 {
        if i == 6 {
            g.push_break(at(i));
        }
        // A spike in the very first bucket: nothing else comes near it, so
        // the front bucket's extreme can only survive by mistake.
        let v = if i == 0 { 50.0 } else { 1.0 + (i % 7) as f64 };
        g.push(v, at(i), "DC V", "V", None);
    }
    g.ensure_level(WIDTH);
    let level = g.minimap_level.as_ref().expect("cut");
    assert_eq!(level.value_range().expect("samples").1, 50.0);
    assert_eq!(level.gaps().count(), 1);

    // One more sample evicts the spike, which forces the rescan.
    g.push(1.0, at(KEEP as u64), "DC V", "V", None);
    let level = g.minimap_level.as_ref().expect("cut");
    assert!(
        level.value_range().expect("samples").1 < 50.0,
        "the spike must leave with the point that made it"
    );
    assert_eq!(
        level.gaps().count(),
        1,
        "the band's opening point is still in"
    );
    assert_eq!(g.minimap_level, Some(g.build_level(WIDTH)));

    // Evicting the point the band hangs from takes the band with it.
    for i in 1..=5 {
        g.push(1.0, at(KEEP as u64 + i), "DC V", "V", None);
    }
    assert_eq!(
        g.minimap_level.as_ref().expect("cut").gaps().count(),
        0,
        "a band whose opening point was evicted has nothing left to hang from"
    );
    assert_eq!(g.minimap_level, Some(g.build_level(WIDTH)));
}

/// Lowering the Buffer size setting has to take the graph down to it at once,
/// and the overlay traces and the minimap's buckets with it, or the strip
/// would draw points the plot no longer has.
#[test]
fn graph_set_max_points_evicts_down_and_takes_the_overlays_with_it() {
    const WIDTH: f64 = 0.05;
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..50u64 {
        push_aux(
            &mut g,
            i as f64,
            t0 + Duration::from_millis(i * 10),
            None,
            &[("T2", Some(i as f64 + 0.5))],
        );
    }
    g.ensure_level(WIDTH);

    g.set_max_points(20);
    assert_eq!(g.len(), 20);
    let values = g.overlay_values("T2");
    assert_eq!(values.len(), 20);
    assert_eq!(
        values.first().copied().flatten(),
        Some(30.5),
        "the oldest sub-value went with the point it belonged to"
    );
    assert!(
        g.minimap_level.is_none(),
        "a bulk drop recuts the level instead of rescanning a bucket per point"
    );
    g.ensure_level(WIDTH);
    assert_eq!(
        g.minimap_level.as_ref().and_then(|l| l.value_range()),
        Some((30.0, 49.0)),
        "the recut strip covers the points that are left, and no older ones"
    );
    assert!(
        g.history.capacity() < 1024,
        "lowering the bound hands the memory back, not just the points"
    );
}

/// An overload shorter than a bucket still has to break the trace, or the
/// strip would draw straight through a stretch the meter never measured.
#[test]
fn a_break_inside_a_bucket_splits_it() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push_break(t0 + Duration::from_micros(500));
    g.push(2.0, t0 + Duration::from_micros(500), "DC V", "V", None);

    let mut level = g.build_level(0.001);
    assert_eq!(level.len(), 2, "the break opens a bucket of its own");
    let runs: Vec<Vec<i64>> = level
        .runs()
        .map(|run| run.iter().map(|b| b.key).collect())
        .collect();
    assert_eq!(
        runs,
        vec![vec![0], vec![0]],
        "two polylines, both in the same time bucket"
    );
}

/// The strip steps its bucket width geometrically as the session grows, and
/// a step recuts the level — but only a step, or every frame would pay for a
/// rebuild.
#[test]
fn a_width_step_recuts_the_level() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..40_u64 {
        g.push(
            i as f64,
            t0 + Duration::from_millis(i * 10),
            "DC V",
            "V",
            None,
        );
    }

    g.ensure_level(0.01);
    let fine = g.minimap_level.as_ref().expect("cut").len();
    g.ensure_level(0.01);
    let level = g.minimap_level.as_ref().expect("cut");
    assert_eq!(level.width(), 0.01);
    assert_eq!(level.len(), fine, "the same width keeps the level as cut");

    g.ensure_level(0.05);
    let level = g.minimap_level.as_ref().expect("recut");
    assert_eq!(level.width(), 0.05);
    assert!(level.len() < fine, "a wider bucket holds more samples");
}

/// Both start a new session: the level's buckets and its sequence numbers
/// describe history that no longer exists.
#[test]
fn clear_and_mode_change_drop_the_level() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_millis(10), "DC V", "V", None);
    g.ensure_level(0.01);
    assert!(g.minimap_level.is_some());

    g.clear();
    assert!(g.minimap_level.is_none());
    assert_eq!(g.pushed_total, 0);

    g.push(1.0, t0, "DC V", "V", None);
    g.ensure_level(0.01);
    g.push(100.0, t0 + Duration::from_millis(10), "Ohm", "Ω", None);
    assert!(
        g.minimap_level.is_none(),
        "a mode change restarts the trace"
    );
    assert_eq!(g.pushed_total, 1, "and the sequence numbers with it");
}

/// A stretch read at a slow interval keeps its line when a faster one is
/// picked: its spacing was judged against the interval it was read at. A
/// silence after the change is judged against the new one.
#[test]
fn a_shorter_interval_keeps_the_history_unbroken() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.set_sample_interval_ms(2_000);
    for i in 0..5 {
        g.push(1.0, t0 + Duration::from_secs(2 * i), "DC V", "V", None);
    }
    g.ensure_level(0.01);
    g.set_sample_interval_ms(100);
    assert!(g.visible_gaps().is_empty(), "{:?}", g.visible_gaps());
    assert!(g.minimap_level.is_some(), "the level's runs still hold");

    // The stretch across the change began at 2 s, and is judged by it.
    g.push(1.0, t0 + Duration::from_secs(10), "DC V", "V", None);
    assert!(g.visible_gaps().is_empty(), "{:?}", g.visible_gaps());
    g.push(1.0, t0 + Duration::from_secs(12), "DC V", "V", None);
    assert_eq!(
        g.visible_gaps().len(),
        1,
        "2 s of silence at 100 ms is a gap"
    );
}

/// The point of the level: neither a push nor a frame may cost more because
/// the session has been running longer. The bucket width follows the strip,
/// so both histories reduce to the same number of buckets.
#[test]
#[ignore = "timing-sensitive; run with --release"]
fn push_and_frame_cost_do_not_scale_with_history() {
    fn measure(points: u64) -> (Duration, Duration) {
        // What a ~500px strip would ask for at this session length.
        let width = points as f64 * 0.01 / 500.0;
        let mut g = Graph::new();
        // Bound at exactly what the run holds, so both sizes are measured
        // pushing into a full buffer: otherwise the short run would never
        // evict and the comparison would be push against push-plus-evict.
        g.set_max_points(points as usize);
        let t0 = Instant::now();
        let at = |i: u64| t0 + Duration::from_millis(i * 10);
        let value = |i: u64| (i as f64 * 0.017).sin() * 10.0;
        for i in 0..points {
            g.push(value(i), at(i), "DC V", "V", None);
        }
        g.ensure_level(width);

        let start = Instant::now();
        for i in points..points + 1_000 {
            g.push(value(i), at(i), "DC V", "V", None);
        }
        let push = start.elapsed();

        let start = Instant::now();
        let mut sink = 0.0_f64;
        for _ in 0..100 {
            let level = g.minimap_level.as_mut().expect("cut");
            let (lo, _) = level.value_range().expect("samples");
            sink += lo;
            for run in level.runs() {
                sink += level_polyline(run, |v| v as f32).len() as f64;
            }
        }
        let frame = start.elapsed();
        assert!(sink.is_finite());
        (push, frame)
    }

    let (push_short, frame_short) = measure(50_000);
    let (push_long, frame_long) = measure(500_000);
    let ratio =
        |short: Duration, long: Duration| long.as_secs_f64() / short.as_secs_f64().max(1e-9);
    println!(
        "50K: push {push_short:?}, frames {frame_short:?}\n\
         500K: push {push_long:?}, frames {frame_long:?}\n\
         ratios: push {:.2}x, frame {:.2}x",
        ratio(push_short, push_long),
        ratio(frame_short, frame_long),
    );
    assert!(ratio(push_short, push_long) < 2.0, "push cost grew");
    assert!(ratio(frame_short, frame_long) < 2.0, "frame cost grew");
}

// ── Pointer gestures on the main plot ───────────────────────────────────────
//
// These drive `show_main` through a real `egui::Context` so the gesture goes
// the way it does in the app: through egui's input state (which is what folds
// Ctrl+wheel into `zoom_delta` and keeps it out of `smooth_scroll_delta`) and
// through the plot's own hit testing.

/// Big enough that the plot gets a real rect. Pointer positions below are in
/// these coordinates.
fn gesture_screen() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))
}

/// Inside the plot body.
fn over_plot() -> egui::Pos2 {
    egui::pos2(400.0, 300.0)
}

/// Outside it: the Y-axis strip is at least 60 pt wide (`y_axis_min_width`),
/// so the top-left corner belongs to the axis widget, not the plot.
fn off_plot() -> egui::Pos2 {
    egui::pos2(2.0, 2.0)
}

fn wheel_event(modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        // Under 8 points, which is what makes egui apply the tick in this
        // pass instead of smoothing it out over the next few.
        delta: egui::vec2(0.0, 4.0),
        phase: egui::TouchPhase::Move,
        modifiers,
    }
}

/// Drive one headless frame of the main graph with the pointer at `pointer`
/// and `events` delivered to that frame.
///
/// egui hit-tests against the previous pass's widget rects, so the plot only
/// reports `contains_pointer` from the second frame the pointer is over it —
/// every caller runs a warm-up frame first.
fn gesture_frame(g: &mut Graph, ctx: &egui::Context, pointer: egui::Pos2, events: &[egui::Event]) {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut input = egui::RawInput {
        screen_rect: Some(gesture_screen()),
        ..Default::default()
    };
    input.events.push(egui::Event::PointerMoved(pointer));
    input.events.extend_from_slice(events);
    let mut output = ctx.run_ui(input, |ui| {
        g.show_main(ui, &tc, &crate::markers::Markers::default())
    });
    // Nothing here paints, and TexturesDelta panics if it is dropped unapplied.
    output.textures_delta.clear();
}

fn graph_with_a_minute_of_data() -> Graph {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..60_u64 {
        g.push(
            (i as f64) * 0.1,
            t0 + Duration::from_millis(i * 100),
            "DC V",
            "V",
            None,
        );
    }
    g
}

/// The panels the graph sits in need the plain wheel for scrolling, so a bare
/// tick must leave the graph exactly as it was — no zoom, and no drop out of
/// live either, which is what the old handler did on the first tick.
#[test]
fn plain_wheel_over_the_plot_leaves_the_graph_alone() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, over_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        over_plot(),
        &[wheel_event(egui::Modifiers::NONE)],
    );
    assert_eq!(g.time_window_secs, window, "plain wheel zoomed the graph");
    assert!(g.live, "plain wheel dropped out of live mode");
}

/// Ctrl+wheel is the zoom gesture. Wheel up narrows the window, and the tick
/// also has to leave live mode, or the zoom would be undone by the next
/// sample snapping the view back.
#[test]
fn ctrl_wheel_over_the_plot_zooms_and_leaves_live() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, over_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        over_plot(),
        &[wheel_event(egui::Modifiers::CTRL)],
    );
    assert!(
        g.time_window_secs < window,
        "wheel up must zoom in: window went from {window} to {}",
        g.time_window_secs
    );
    assert!(!g.live, "a zoom has to leave live mode to survive");
}

/// The gesture is the plot's, not the window's: over the toolbar, the stats
/// panel or a modal the graph must not move.
#[test]
fn ctrl_wheel_off_the_plot_changes_nothing() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, off_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        off_plot(),
        &[wheel_event(egui::Modifiers::CTRL)],
    );
    assert_eq!(
        g.time_window_secs, window,
        "zoomed without the pointer over the plot"
    );
    assert!(g.live, "left live mode without the pointer over the plot");
}

// --- Overlay shortcuts -------------------------------------------------
//
// Driven through `Graph::show`, the way the app calls it: the key handler
// runs before the toolbar is drawn, so a chip that appears in the same frame
// already shows the new state — which is what lets the `R` test read the
// caret off the field it opens.

/// A key pressed and released within one frame.
fn key_press(key: egui::Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

/// One headless frame of the whole graph — toolbar included — with `events`
/// delivered to it.
fn graph_frame(g: &mut Graph, ctx: &egui::Context, events: Vec<egui::Event>) {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(gesture_screen()),
            events,
            ..Default::default()
        },
        |ui| g.show(ui, &tc, &crate::markers::Markers::default()),
    );
    // Nothing here paints, and TexturesDelta panics if it is dropped unapplied.
    output.textures_delta.clear();
}

#[test]
fn m_and_x_toggle_the_mean_and_the_envelope() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_press(egui::Key::M));
    assert!(g.show_mean, "M did not draw the mean line");
    graph_frame(&mut g, &ctx, key_press(egui::Key::X));
    assert!(g.show_envelope, "X did not draw the min/max band");
    graph_frame(&mut g, &ctx, key_press(egui::Key::M));
    graph_frame(&mut g, &ctx, key_press(egui::Key::X));
    assert!(
        !g.show_mean && !g.show_envelope,
        "a second press left them on"
    );
}

/// The Triggers chip is drawn only while the reference lines are, so its key
/// has nothing to toggle until they are on.
#[test]
fn t_waits_for_the_reference_lines() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    let crossings = g.show_crossings;
    graph_frame(&mut g, &ctx, key_press(egui::Key::T));
    assert_eq!(
        g.show_crossings, crossings,
        "T moved the triggers with Ref off"
    );
    g.show_ref_line = true;
    graph_frame(&mut g, &ctx, key_press(egui::Key::T));
    assert_eq!(
        g.show_crossings, !crossings,
        "T left the triggers alone with Ref on"
    );
}

/// Switching the lines on from the keyboard has to land the caret in the
/// field their values are typed into — nothing is drawn until one is.
#[test]
fn r_opens_the_reference_field_and_takes_the_caret_to_it() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_press(egui::Key::R));
    assert!(g.show_ref_line, "R did not switch the reference lines on");
    assert!(
        !g.focus_ref_field,
        "the field left the focus request pending"
    );
    // The one text entry the toolbar draws here: Y:Fixed and Min/Max are off.
    assert!(ctx.text_edit_focused(), "the caret is not in a text field");
}

/// The keys belong to whatever holds the keyboard, so the caret `R` just
/// placed in the values field types there instead.
#[test]
fn a_focused_field_keeps_the_overlay_keys() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_press(egui::Key::R));
    graph_frame(&mut g, &ctx, key_press(egui::Key::M));
    assert!(!g.show_mean, "M drew the mean line from inside the field");
    graph_frame(&mut g, &ctx, key_press(egui::Key::R));
    assert!(
        g.show_ref_line,
        "R switched the lines off from their own field"
    );
}

/// Cursors off is the chip's clearing too: a pair left behind would come
/// back with the next press, measuring between two points the user has
/// forgotten placing.
#[test]
fn c_clears_the_cursors_on_the_way_off() {
    let ctx = egui::Context::default();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_press(egui::Key::C));
    assert!(g.cursors_active, "C did not switch the cursors on");
    g.cursor_a = Some(1.0);
    g.cursor_b = Some(2.0);
    g.cursor_next_is_b = true;
    graph_frame(&mut g, &ctx, key_press(egui::Key::C));
    assert!(!g.cursors_active, "C did not switch the cursors off");
    assert_eq!((g.cursor_a, g.cursor_b), (None, None), "a cursor survived");
    assert!(!g.cursor_next_is_b, "the next-click side survived");
}

/// The export history is cut to this, so it has to move exactly when the
/// graph's oldest point does: on a restart, on eviction and on Clear, and
/// not on an over-range reading, which adds no point.
#[test]
fn first_point_time_follows_the_oldest_point() {
    let mut g = Graph::new();
    assert_eq!(g.first_point_time(), None);
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);

    g.push(1.0, at(0), "DC V", "V", None);
    g.push(1.0, at(1), "DC V", "V", None);
    assert_eq!(g.first_point_time(), Some(at(0)));

    g.push_break(at(2));
    assert_eq!(g.first_point_time(), Some(at(0)), "no point, no move");

    g.push(1.0, at(3), "AC V", "V", None);
    assert_eq!(g.first_point_time(), Some(at(3)), "a mode change restarts");

    g.push(1.0, at(4), "AC V", "V", None);
    g.push(1.0, at(5), "AC V", "V", None);
    g.set_max_points(2);
    assert_eq!(g.first_point_time(), Some(at(4)), "evicted with its point");

    g.clear();
    assert_eq!(g.first_point_time(), None);
}

/// A reading a second for `secs` seconds on a 10 s window, and when it began.
fn graph_over(secs: u64) -> (Graph, Instant) {
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    let t0 = Instant::now();
    for i in 0..=secs {
        g.push(1.0, t0 + Duration::from_secs(i), "DC V", "V", None);
    }
    (g, t0)
}

fn marked(times: &[Instant]) -> crate::markers::Markers {
    let mut markers = crate::markers::Markers::default();
    for &at in times {
        markers
            .add(at, chrono::Local::now(), String::new())
            .expect("one marker per reading");
    }
    markers
}

/// Markers sit at their reading's time from the origin, and only those in
/// the window are asked for.
#[test]
fn markers_sit_at_their_readings_time() {
    let (g, t0) = graph_over(30);
    let markers = marked(&[t0 + Duration::from_secs(5), t0 + Duration::from_secs(20)]);
    let at = |x_min, x_max| -> Vec<(f64, u32)> {
        g.markers_between(&markers, x_min, x_max)
            .into_iter()
            .map(|(x, m)| (x, m.number))
            .collect()
    };
    assert_eq!(at(0.0, 30.0), [(5.0, 1), (20.0, 2)]);
    assert_eq!(at(10.0, 30.0), [(20.0, 2)]);
}

/// A restart moves the origin past the old trace: a marker on it is no
/// longer the graph's to draw, and after Clear nothing is.
#[test]
fn a_restarted_graph_shows_no_marker_from_before() {
    let (mut g, t0) = graph_over(10);
    let markers = marked(&[t0 + Duration::from_secs(5)]);
    assert!(g.holds(t0 + Duration::from_secs(5)));
    g.push(100.0, t0 + Duration::from_secs(11), "Ohm", "Ω", None);
    assert!(!g.holds(t0 + Duration::from_secs(5)));
    assert!(g.markers_between(&markers, 0.0, 1e6).is_empty());
    g.clear();
    assert!(g.markers_between(&markers, 0.0, 1e6).is_empty());
}

/// A marker out of view is brought to the middle of the window, which keeps
/// its width; one in view leaves the view alone; one at the newest reading
/// lands back in live.
#[test]
fn reveal_centres_a_marker_out_of_view() {
    let (mut g, t0) = graph_over(60);
    assert!(g.live);
    g.reveal(t0 + Duration::from_secs(55));
    assert!(g.live, "already in view");

    g.reveal(t0 + Duration::from_secs(20));
    assert!(!g.live);
    let (lo, hi) = g.view_bounds();
    assert_eq!((lo, hi), (15.0, 25.0));

    g.reveal(t0 + Duration::from_secs(22));
    assert_eq!(g.view_bounds(), (15.0, 25.0), "in view: nothing moves");

    g.reveal(t0 + Duration::from_secs(60));
    assert!(g.live, "the newest reading is the live edge");
}

/// A graph that no longer holds a reading has nowhere to show it.
#[test]
fn reveal_ignores_a_reading_the_graph_dropped() {
    let (mut g, t0) = graph_over(10);
    g.push(100.0, t0 + Duration::from_secs(11), "Ohm", "Ω", None);
    let before = (g.live, g.view_center);
    g.reveal(t0 + Duration::from_secs(5));
    assert_eq!((g.live, g.view_center), before);
}

/// Flags as a 7 px-per-character font would lay them out in a 1000 px plot.
fn flags(markers: &[(f32, u32, &str)]) -> Vec<(f32, egui::Rect, String)> {
    let plot = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1000.0, 300.0));
    layout_marker_flags(markers, plot, 19.0, |s| s.chars().count() as f32 * 7.0)
        .into_iter()
        .map(|f| (f.x, f.rect, f.label))
        .collect()
}

#[test]
fn a_flag_is_centred_on_its_line_and_cut_to_its_cap() {
    let long = "load on: 2.2 ohm across the output, supply set to 12.07 V";
    let laid = flags(&[(500.0, 3, long)]);
    let (x, rect, label) = &laid[0];
    assert_eq!(*x, 500.0);
    assert!((rect.center().x - 500.0).abs() < 0.01, "centred");
    assert!(rect.width() <= 170.0, "{rect:?}");
    assert!(
        label.starts_with("3 \u{00B7} load on") && label.ends_with('\u{2026}'),
        "{label}"
    );
    assert_eq!(rect.bottom(), 297.0, "along the bottom of the plot");
}

/// Close neighbours keep their numbers and lose their notes; closer ones
/// slide apart, and a flag pushed off its own line is left out.
#[test]
fn close_flags_keep_their_numbers() {
    let laid = flags(&[(400.0, 4, "fan on"), (430.0, 5, "load off")]);
    let labels: Vec<&str> = laid.iter().map(|(_, _, l)| l.as_str()).collect();
    assert_eq!(labels, ["4", "5"]);
    assert!(laid[0].1.right() < laid[1].1.left(), "no overlap");

    // 18 px apart, as two markers 1.2 s apart on a one-minute window: the
    // second slides right, its point still on its tag.
    let laid = flags(&[(400.0, 3, ""), (418.0, 4, "")]);
    assert_eq!(laid.len(), 2, "{laid:?}");
    assert!(laid[0].1.right() + 4.0 <= laid[1].1.left());
    assert!(laid[1].1.left() + 5.0 <= 418.0);

    let laid = flags(&[(400.0, 4, ""), (402.0, 5, "")]);
    let labels: Vec<&str> = laid.iter().map(|(_, _, l)| l.as_str()).collect();
    assert_eq!(labels, ["4"], "the second would leave its line");
}

/// At the plot's edge a flag slides inward, its line still under it, and a
/// neighbour it reaches moves over.
#[test]
fn an_edge_flag_slides_in_and_its_neighbour_moves_over() {
    let laid = flags(&[(3.0, 1, "supply warm-up done and settled"), (150.0, 2, "")]);
    assert_eq!(laid.len(), 2, "{laid:?}");
    let (x, rect, _) = &laid[0];
    assert_eq!(rect.left(), 0.0);
    assert!(rect.left() <= *x && *x <= rect.right());
    assert!(laid[0].1.right() + 4.0 <= laid[1].1.left());
    assert!(laid[1].1.left() <= 150.0 && 150.0 <= laid[1].1.right());
}

/// A marker's flag takes a click, and the graph reports whose it was.
#[test]
fn clicking_a_flag_reports_its_marker() {
    let (mut g, t0) = graph_over(30);
    let markers = marked(&[t0 + Duration::from_secs(25)]);
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    // One frame; where the flag labelled for marker 1 was drawn.
    let frame = |g: &mut Graph, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(gesture_screen()),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| g.show_main(ui, &tc, &markers));
        out.textures_delta.clear();
        out.platform_output.accesskit_update.and_then(|update| {
            update
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Write marker 1's note"))
                .and_then(|(_, n)| n.bounds())
                .map(|b| egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))
        })
    };
    // The axes settle on the first frame, and the flag with them.
    frame(&mut g, vec![]);
    let at = frame(&mut g, vec![]).expect("the flag is drawn and labelled");
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut g, vec![egui::Event::PointerMoved(at)]);
    frame(&mut g, vec![egui::Event::PointerMoved(at), button(true)]);
    frame(&mut g, vec![egui::Event::PointerMoved(at), button(false)]);
    assert_eq!(g.take_clicked_marker(), Some(1));
    assert!(
        g.cursor_a.is_none(),
        "the click is the flag's, not the plot's"
    );
}

// ── The plot's right-click menu ─────────────────────────────────────────────

/// One frame of the main graph with AccessKit on; the tree it produced.
fn menu_frame(
    g: &mut Graph,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Vec<(egui::accesskit::NodeId, egui::accesskit::Node)> {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let input = egui::RawInput {
        screen_rect: Some(gesture_screen()),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        g.show_main(ui, &tc, &crate::markers::Markers::default())
    });
    out.textures_delta.clear();
    out.platform_output
        .accesskit_update
        .map(|update| update.nodes)
        .unwrap_or_default()
}

fn node_rect(
    tree: &[(egui::accesskit::NodeId, egui::accesskit::Node)],
    label: impl Fn(&str) -> bool,
) -> Option<egui::Rect> {
    tree.iter()
        .find(|(_, n)| n.label().is_some_and(&label))
        .and_then(|(_, n)| n.bounds())
        .map(|b| {
            egui::Rect::from_min_max(
                egui::pos2(b.x0 as f32, b.y0 as f32),
                egui::pos2(b.x1 as f32, b.y1 as f32),
            )
        })
}

/// A press and release of `button` at `pos`, on frames of their own.
fn menu_click(g: &mut Graph, ctx: &egui::Context, pos: egui::Pos2, button: egui::PointerButton) {
    let event = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    menu_frame(g, ctx, vec![egui::Event::PointerMoved(pos)]);
    menu_frame(g, ctx, vec![egui::Event::PointerMoved(pos), event(true)]);
    menu_frame(g, ctx, vec![egui::Event::PointerMoved(pos), event(false)]);
}

/// Right-click at `x` across the plot (0 its left edge, 1 its right), then
/// pick "Add marker here"; what the graph asks to mark. `between` runs after
/// the menu opens, before the pick.
fn mark_from_menu(
    g: &mut Graph,
    x: f32,
    between: impl FnOnce(&mut Graph),
) -> Option<(Instant, f64)> {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    menu_frame(g, &ctx, vec![]);
    let tree = menu_frame(g, &ctx, vec![]);
    let plot = node_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    let at = egui::pos2(plot.left() + x * plot.width(), plot.center().y);
    menu_click(g, &ctx, at, egui::PointerButton::Secondary);
    assert!(egui::Popup::is_any_open(&ctx), "the menu opened");
    between(g);
    let tree = menu_frame(g, &ctx, vec![]);
    let entry = node_rect(&tree, |l| l == "Add marker here").expect("the entry");
    menu_click(g, &ctx, entry.center(), egui::PointerButton::Primary);
    assert!(!egui::Popup::is_any_open(&ctx), "picking closes it");
    g.take_mark_request()
}

/// "Add marker here" marks the reading nearest the right-click.
#[test]
fn the_menu_marks_the_reading_right_clicked() {
    let (mut g, t0) = graph_over(30);
    let (at, value) = mark_from_menu(&mut g, 0.98, |_| {}).expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(30), "the newest, at the right");
    assert_eq!(value, 1.0);
}

/// Live, the view moves on while the menu is open; the mark stays on the
/// reading that was right-clicked.
#[test]
fn a_live_view_moving_on_leaves_the_menus_reading() {
    let (mut g, t0) = graph_over(30);
    let (at, _) = mark_from_menu(&mut g, 0.98, |g| {
        for i in 31..=40 {
            g.push(2.0, t0 + Duration::from_secs(i), "DC V", "V", None);
        }
    })
    .expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(30));
}

/// A right-click names a reading in view, even when one out of view is
/// nearer the time it landed on.
#[test]
fn the_menu_marks_a_reading_in_view() {
    let (mut g, t0) = graph_over(30);
    g.live = false;
    // A view from 5.2 s to 15.2 s: at its left edge, 5 s is nearer than
    // 6 s, and out of view.
    g.view_center = 10.2;
    let (at, _) = mark_from_menu(&mut g, 0.005, |_| {}).expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(6));
}

/// Esc closes the menu and leaves the focus with nothing, so the graph's
/// keys still work.
#[test]
fn esc_closes_the_menu_and_leaves_no_focus() {
    let (mut g, _) = graph_over(30);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    menu_frame(&mut g, &ctx, vec![]);
    let tree = menu_frame(&mut g, &ctx, vec![]);
    let plot = node_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    menu_click(&mut g, &ctx, plot.center(), egui::PointerButton::Secondary);
    menu_frame(&mut g, &ctx, vec![]);
    assert!(
        ctx.memory(|m| m.focused()).is_some(),
        "the entry has the focus"
    );
    let esc = |pressed| egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    menu_frame(&mut g, &ctx, vec![esc(true), esc(false)]);
    menu_frame(&mut g, &ctx, vec![]);
    assert!(!egui::Popup::is_any_open(&ctx), "Esc closed it");
    assert!(
        ctx.memory(|m| m.focused()).is_none(),
        "nothing keeps the focus"
    );
    assert!(g.take_mark_request().is_none());
}

/// A noisy line with samples every `step` seconds, from a fixed seed.
fn noisy_line(n: usize, step: f64) -> Vec<[f64; 2]> {
    let mut seed = 12345_u64;
    (0..n)
        .map(|i| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let x = i as f64 * step;
            [x, (x * 0.01).sin() * 5.0 + (seed % 1000) as f64 / 300.0]
        })
        .collect()
}

#[test]
fn thinning_leaves_a_sparse_line_alone() {
    assert!(thin_for_drawing(&[], 0.5).is_empty());
    // One sample per bucket at most: nothing to drop.
    let sparse = noisy_line(200, 0.7);
    assert_eq!(thin_for_drawing(&sparse, 0.5), sparse);
}

/// Each bucket keeps its first, last, lowest and highest point, in time
/// order, and nothing else.
#[test]
fn thinning_keeps_each_buckets_ends_and_extremes() {
    let line = noisy_line(5000, 0.1);
    let bucket = 3.0;
    let thinned = thin_for_drawing(&line, bucket);

    assert!(
        thinned.windows(2).all(|w| w[0][0] < w[1][0]),
        "out of time order"
    );
    let key = |p: &[f64; 2]| (p[0] / bucket).floor() as i64;
    let buckets = line.last().map_or(0, |p| key(p) + 1) as usize;
    assert!(
        thinned.len() <= 4 * buckets,
        "{} points for {buckets} buckets",
        thinned.len()
    );

    for b in 0..buckets as i64 {
        let all: Vec<_> = line.iter().filter(|p| key(p) == b).copied().collect();
        let kept: Vec<_> = thinned.iter().filter(|p| key(p) == b).copied().collect();
        let lowest = all
            .iter()
            .copied()
            .reduce(|a, p| if p[1] < a[1] { p } else { a });
        let highest = all
            .iter()
            .copied()
            .reduce(|a, p| if p[1] > a[1] { p } else { a });
        for want in [all.first().copied(), all.last().copied(), lowest, highest] {
            let want = want.expect("every bucket holds samples");
            assert!(kept.contains(&want), "bucket {b} lost {want:?}");
        }
        assert!(kept.iter().all(|p| all.contains(p)));
    }
}

/// Buckets are fixed in session time, so a view sliding along the data, as
/// live view does with each sample, draws the buckets it fully covers the
/// same way every frame: nothing shimmers.
#[test]
fn thinning_does_not_change_as_the_view_slides() {
    let line = noisy_line(5000, 0.1);
    let bucket = 3.0;
    let whole = thin_for_drawing(&line, bucket);
    for start in [1, 17, 29, 30, 31, 444] {
        let slid = thin_for_drawing(&line[start..], bucket);
        // The slice's first bucket is only partly covered.
        let covered = ((line[start][0] / bucket).floor() + 1.0) * bucket;
        let after = |v: &[[f64; 2]]| -> Vec<[f64; 2]> {
            v.iter().filter(|p| p[0] >= covered).copied().collect()
        };
        assert_eq!(
            after(&slid),
            after(&whole),
            "view starting at sample {start}"
        );
    }
}

/// Zoomed out to an hour, the main graph draws about what its width holds,
/// not every sample: sixty times the samples of a one-minute view, but far
/// fewer times its vertices.
#[test]
fn a_wide_window_draws_about_as_much_as_a_narrow_one() {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let markers = crate::markers::Markers::default();
    let mut g = Graph::new();
    let t0 = Instant::now();
    for [x, v] in noisy_line(36_000, 0.1) {
        g.push(v, t0 + Duration::from_secs_f64(x), "DC V", "V", None);
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 500.0));
    let mut vertices = |window: f64| {
        g.time_window_secs = window;
        let ctx = egui::Context::default();
        let mut count = 0;
        // The axes settle on the first frame.
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| g.show_main(ui, &tc, &markers));
            out.textures_delta.clear();
            count = ctx
                .tessellate(out.shapes, out.pixels_per_point)
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum::<usize>();
        }
        count
    };
    let narrow = vertices(60.0);
    let wide = vertices(3600.0);
    // About 10 times as many; every sample drawn made it about 47.
    assert!(wide < 20 * narrow, "1 h: {wide} vertices, 1 min: {narrow}");
}
