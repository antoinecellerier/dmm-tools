//! Taking readings in: mode and unit changes, overloads, gaps and breaks.

use super::*;
use crate::graph::render::quantize_for_hash;

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

/// Auto-range crossing a decade keeps the mode string but moves the unit:
/// the trace carries on, its earlier points re-expressed in the new prefix,
/// so 219 Ω and 0.22 kΩ sit side by side rather than 1000x apart.
#[test]
fn a_decade_step_re_expresses_the_trace() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |s: u64| t0 + Duration::from_secs(s);
    g.push(150.0, at(0), "\u{3a9}", "\u{3a9}", None);
    g.push(219.0, at(1), "\u{3a9}", "\u{3a9}", None);
    let epoch = g.history_epoch;
    g.push(0.22, at(2), "\u{3a9}", "k\u{3a9}", None);
    assert_eq!(g.current_unit, "k\u{3a9}");
    assert_eq!(
        g.all_segments(),
        vec![vec![[0.0, 0.15], [1.0, 0.219], [2.0, 0.22]]]
    );
    assert_eq!(g.origin, Some(t0), "the time axis stays where it was");
    assert_ne!(g.history_epoch, epoch, "figures cached in Ω are stale");
    assert!(g.minimap_level.is_none(), "so is the minimap's level");
}

/// Every prefixed unit steps as voltage does: the trace carries on rather
/// than restarting at each step.
#[test]
fn a_step_in_any_prefixed_unit_keeps_the_trace() {
    for (low, high) in [
        ("mH", "H"),
        ("mWh", "Wh"),
        ("Wh", "kWh"),
        ("mAh", "Ah"),
        ("mW/m²", "W/m²"),
        ("m°", "°"),
    ] {
        let mut g = Graph::new();
        let t0 = Instant::now();
        g.push(900.0, t0, "L", low, None);
        g.push(1.1, t0 + Duration::from_secs(1), "L", high, None);
        assert_eq!(g.current_unit, high);
        assert_eq!(
            g.all_segments(),
            vec![vec![[0.0, 0.9], [1.0, 1.1]]],
            "{low}"
        );
    }
}

/// What the user set against the plotted axis follows a decade step: the
/// fixed Y range and the reference lines frame the same readings after it.
#[test]
fn a_decade_step_re_expresses_the_fixed_range_and_reference_lines() {
    let mut g = Graph::new();
    g.push(150.0, Instant::now(), "\u{3a9}", "\u{3a9}", None);
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(100.0);
    g.y_axis.max.set(1500.0);
    *g.ref_lines.text_mut() = "200, 950, x".to_string();
    g.ref_lines.parse(Some(&g.current_unit.clone()));
    g.push(1.05, Instant::now(), "\u{3a9}", "k\u{3a9}", None);
    assert!(g.y_axis.fixed && g.y_axis.user_set);
    assert_eq!((g.y_axis.min.value(), g.y_axis.max.value()), (0.1, 1.5));
    assert_eq!(g.ref_lines.values(), [0.2, 0.95]);
    // The Y fields' text follows once the toolbar draws them with no caret
    // in them; the references keep their numbers, now with the unit they
    // were typed in.
    g.show_ref_line = true;
    graph_frame(&mut g, &crate::app::styled_context(), Vec::new());
    assert_eq!((g.y_axis.min.text(), g.y_axis.max.text()), ("0.1", "1.5"));
    assert_eq!(
        g.ref_lines.text_mut().as_str(),
        "200 \u{3a9}, 950 \u{3a9}, x"
    );
    assert_eq!(g.ref_lines.values(), [0.2, 0.95]);
}

/// A field hidden through a step shows its text in the new prefix the first
/// time it is drawn again.
#[test]
fn a_field_hidden_through_a_step_shows_the_new_prefix() {
    let mut g = Graph::new();
    g.push(150.0, Instant::now(), "\u{3a9}", "\u{3a9}", None);
    g.y_axis.min.set(100.0);
    g.push(1.05, Instant::now(), "\u{3a9}", "k\u{3a9}", None);
    assert_eq!(g.y_axis.min.text(), "100.0000", "not drawn: Y is Auto");
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    graph_frame(&mut g, &crate::app::styled_context(), Vec::new());
    assert_eq!(g.y_axis.min.text(), "0.1");
}

/// A step while the user types a bound leaves the draft under the caret as
/// typed, read in the prefix it was typed in, and rewrites it once the
/// caret leaves.
#[test]
fn a_decade_step_leaves_a_field_being_typed_in_alone() {
    let ctx = crate::app::styled_context();
    let mut g = Graph::new();
    g.push(12.0, Instant::now(), "DC V", "mV", None);
    g.y_axis.fixed = true;
    g.y_axis.min.text_mut().clear();
    let id = egui::Id::unique(crate::graph::toolbar::Y_MIN_FIELD);
    ctx.memory_mut(|m| m.request_focus(id));
    graph_frame(&mut g, &ctx, Vec::new());
    graph_frame(&mut g, &ctx, vec![egui::Event::Text("15".into())]);
    assert_eq!((g.y_axis.min.text(), g.y_axis.min.value()), ("15", 15.0));
    g.push(1.2, Instant::now(), "DC V", "V", None);
    assert_eq!(g.y_axis.min.value(), 0.015);
    graph_frame(&mut g, &ctx, vec![egui::Event::Text("0".into())]);
    assert_eq!(g.y_axis.min.text(), "150", "the draft under the caret");
    assert_eq!(g.y_axis.min.value(), 0.15, "read in mV, as typed");
    ctx.memory_mut(|m| m.surrender_focus(id));
    graph_frame(&mut g, &ctx, Vec::new());
    assert_eq!(g.y_axis.min.text(), "0.15");
}

/// A change of quantity under one mode string is a new scale: the trace
/// restarts, and a range pinned for the old quantity is released.
#[test]
fn a_change_of_quantity_clears_the_trace_and_its_pinned_range() {
    let mut g = Graph::new();
    g.push(150.0, Instant::now(), "Temp", "\u{b0}C", None);
    g.push(151.0, Instant::now(), "Temp", "\u{b0}C", None);
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.push(302.0, Instant::now(), "Temp", "\u{b0}F", None);
    assert_eq!(g.len(), 1, "°F points must not share a trace with °C");
    assert!(!g.y_axis.fixed);
    assert!(!g.y_axis.user_set);
}

/// A frame over range in another decade ("OL MΩ" among kΩ readings) whose
/// sub-values still come leaves the trace in the prefix of its readings.
#[test]
fn an_overload_in_another_decade_leaves_the_trace_alone() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(1.05, t0, "\u{3a9}", "k\u{3a9}", None);
    let epoch = g.history_epoch;
    g.push_sample(PlotSample {
        value: None,
        timestamp: t0 + Duration::from_secs(1),
        mode: "\u{3a9}",
        unit: "M\u{3a9}",
        display_raw: None,
        series: None,
        main_label: None,
        levels: false,
        overlays: &[("Max", "k\u{3a9}", Some(1.05))],
    });
    assert_eq!(g.current_unit, "k\u{3a9}");
    assert_eq!(g.history_epoch, epoch);
    assert_eq!(g.all_segments(), vec![vec![[0.0, 1.05]]]);
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

/// Turning the dial from mV to Ω with the leads open gives only OL: the old
/// mode's trace must not carry on with an over-range band after it. The Ω
/// trace opens on the band instead.
#[test]
fn an_overload_in_a_new_mode_restarts_the_trace() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push(3.9, t0, "DC mV", "mV", None);
    g.push(3.9, t0 + Duration::from_millis(100), "DC mV", "mV", None);
    push_overload(&mut g, t0 + Duration::from_millis(200), "Ω", "MΩ");

    assert!(g.is_empty(), "the mV trace is gone");
    assert_eq!(g.current_mode.as_deref(), Some("Ω"));
    assert_eq!(
        g.pending_overload_span(),
        Some((0.0, 0.0)),
        "the band starts with the Ω trace, not on the mV one"
    );

    // The first reading closes the band before it.
    g.push(1.0, t0 + Duration::from_millis(300), "Ω", "kΩ", None);
    assert_eq!(g.all_segments().len(), 1);
    assert_gaps(&g, &[(0.0, 0.1, GapKind::Overload)]);
}

/// Asserts the graph's gaps, to the nanosecond.
fn assert_gaps(g: &Graph, expected: &[(f64, f64, GapKind)]) {
    let gaps = g.visible_gaps();
    assert_eq!(gaps.len(), expected.len(), "{gaps:?}");
    for (&(a, b, k), &(ea, eb, ek)) in gaps.iter().zip(expected) {
        assert!(
            (a - ea).abs() < 1e-9 && (b - eb).abs() < 1e-9 && k == ek,
            "{gaps:?}"
        );
    }
}

/// A session opening on OL draws its stretch as a band, like any later
/// one: open while OL lasts, closed by the first reading, the same in the
/// minimap's level, and part of what the graph holds.
#[test]
fn a_trace_opening_over_range_bands_from_its_first_ol() {
    const WIDTH: f64 = 0.01;
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_overload(&mut g, at(0), "Ω", "MΩ");
    push_overload(&mut g, at(100), "Ω", "MΩ");
    let (start, end) = g.pending_overload_span().expect("open while OL");
    assert!(start.abs() < 1e-9 && (end - 0.1).abs() < 1e-9);
    assert_eq!(g.first_point_time(), Some(at(0)));

    g.ensure_level(WIDTH);
    g.push(1.0, at(200), "Ω", "MΩ", None);
    assert_eq!(g.pending_overload_span(), None);
    assert_eq!(g.all_segments().len(), 1);
    assert_gaps(&g, &[(0.0, 0.2, GapKind::Overload)]);
    assert_eq!(g.minimap_level, Some(g.build_level(WIDTH)));
    let level = g.minimap_level.as_ref().expect("level");
    assert_eq!(level.gaps().count(), 1);
    assert!(g.data_time_range().0.abs() < 1e-9, "Home reaches the band");
    assert_eq!(g.first_point_time(), Some(at(0)));
    assert_eq!(
        g.overloads_in_view(0..1),
        2,
        "the band's OLs count with the view"
    );
}

/// A word or a lost link after the opening OL ends the band at the last OL,
/// as in the middle of a trace.
#[test]
fn a_word_or_a_loss_after_the_opening_ol_ends_its_band() {
    for word in [true, false] {
        let mut g = Graph::new();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        push_overload(&mut g, at(0), "Ω", "MΩ");
        push_overload(&mut g, at(100), "Ω", "MΩ");
        if word {
            g.push_no_reading(at(200));
        } else {
            g.push_data_loss();
        }
        g.push(1.0, at(300), "Ω", "MΩ", None);
        assert_gaps(
            &g,
            &[(0.0, 0.1, GapKind::Overload), (0.1, 0.3, GapKind::NoData)],
        );
    }
}

/// A word or a lost link before a trace's first OL has nothing to break
/// from: the stretch is one band, with no empty gap at its edge.
#[test]
fn a_word_or_a_loss_before_the_opening_ol_is_forgotten() {
    for word in [true, false] {
        let mut g = Graph::new();
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        // A frame without the plotted reading starts the trace, empty.
        push_without_value(&mut g, at(0), "DC V", "V");
        if word {
            g.push_no_reading(at(50));
        } else {
            g.push_data_loss();
        }
        push_overload(&mut g, at(100), "DC V", "V");
        assert!(g.pending_overload_span().is_some(), "over range");
        g.push(1.0, at(200), "DC V", "V", None);
        assert_gaps(&g, &[(0.1, 0.2, GapKind::Overload)]);
    }
}

/// An overload before any data starts the graph there.
#[test]
fn a_break_with_no_history_opens_the_band() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    g.push_break(t0);
    assert_eq!(g.origin, Some(t0));
    assert_eq!(g.pending_overload_span(), Some((0.0, 0.0)));
}

/// The opening band hangs from the trace's first point and goes with it,
/// in the plot and the minimap alike, and with a cleared graph.
#[test]
fn the_opening_band_goes_with_the_first_point() {
    const WIDTH: f64 = 0.01;
    let mut g = Graph::new();
    g.set_max_points(2);
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    push_overload(&mut g, at(0), "DC V", "V");
    g.ensure_level(WIDTH);
    g.push(1.0, at(100), "DC V", "V", None);
    g.push(1.0, at(105), "DC V", "V", None);
    assert_eq!(g.visible_gaps().len(), 1);
    // In the first point's bucket still: the level must not tell it apart
    // from a rebuild either.
    g.push(1.0, at(107), "DC V", "V", None);
    assert!(g.visible_gaps().is_empty());
    assert_eq!(g.first_point_time(), Some(at(105)));
    assert_eq!(g.minimap_level, Some(g.build_level(WIDTH)));

    push_overload(&mut g, at(200), "Ω", "MΩ");
    assert!(g.pending_overload_span().is_some());
    g.clear();
    assert_eq!(g.pending_overload_span(), None);
    assert_eq!(g.first_point_time(), None);
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
    assert!(g.y_axis.fixed);
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
    assert!(g.y_axis.fixed);
    assert_eq!(g.y_range_for_view(0.0, 1.0, true), Some((4.9, 5.1)));

    g.push(1000.0, Instant::now(), "Ohm", "Ω", None);
    assert!(
        !g.y_axis.fixed,
        "mode change must release the fixed Y range"
    );
    assert!(!g.y_axis.user_set);
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
    assert!(g.y_axis.fixed);
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

/// The graph breaks its trace where the session integral stops bridging:
/// a silence of a second and a half is continuous to both, one of two and a
/// half is a gap to both, at the default interval and a fast one alike.
#[test]
fn the_graph_and_the_integral_agree_on_a_gap() {
    for interval_ms in [0, 200] {
        let mut g = Graph::new();
        g.set_sample_interval_ms(interval_ms);
        let mut integral = dmm_lib::stats::Integrator::new();
        integral.set_sample_interval(Duration::from_millis(u64::from(interval_ms)));
        let t0 = Instant::now();
        for ms in [0, 200, 1700, 1900, 4400] {
            let at = t0 + Duration::from_millis(ms);
            g.push(1.0, at, "DC V", "V", None);
            integral.push(1.0, at);
        }
        let gaps: Vec<(f64, f64)> = g.visible_gaps().iter().map(|&(a, b, _)| (a, b)).collect();
        assert_eq!(gaps, [(1.9, 4.4)], "at {interval_ms} ms");
        assert_eq!(integral.skipped_intervals, 1, "at {interval_ms} ms");
    }
}

/// Every value a decade step must re-express, found by naming each of the
/// graph's fields: a field added later fails to compile here until it is
/// placed among them, and a value-bearing one checked. The structs a field
/// holds are reached through the values checked, not field by field.
#[test]
fn a_decade_step_reaches_every_value_bearing_field() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let push = |g: &mut Graph, s: u64, v: f64, unit: &'static str, overlays| {
        g.push_sample(PlotSample {
            value: Some(v),
            timestamp: t0 + Duration::from_secs(s),
            mode: "DC V",
            unit,
            display_raw: None,
            series: None,
            main_label: None,
            levels: false,
            overlays,
        });
    };
    let left = ("Min", "mV", Some(940.0));
    let before = [left, ("Hz", "Hz", Some(999.0))];
    let after = [left, ("Hz", "kHz", Some(1.001))];
    push(&mut g, 0, 950.0, "mV", &before);
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(900.0);
    g.y_axis.max.set(1000.0);
    g.ref_lines.set(&[920.0], &g.current_unit.clone());
    g.y_axis.pins.insert("Hz".into(), (900.0, 1100.0));
    let epoch = g.history_epoch;
    push(&mut g, 1, 1.01, "V", &after);

    let Graph {
        // Re-expressed by the step, each checked below.
        history,
        overlays,
        y_axis,
        ref_lines,
        current_unit,
        // Built on the old values: cleared, or keyed on the epoch or on
        // the unit and values they show.
        history_epoch,
        minimap_level,
        a11y_label: _,
        a11y_label_sig: _,
        cursor_figures: _,
        visible_figures: _,
        histogram: _,
        menu: _,
        // Rebuilt from the values above every frame.
        axis_maps: _,
        plot_transform: _,
        // Times, counts, pixels and words: no plotted unit.
        plot_rect: _,
        histogram_place: _,
        axis_room: _,
        max_points: _,
        pushed_total: _,
        origin: _,
        breaks: _,
        lead_in: _,
        silences: _,
        last_heard: _,
        gap_threshold_secs: _,
        silence_threshold_secs: _,
        main_missing_frames: _,
        current_mode: _,
        current_series: _,
        main_label: _,
        levels: _,
        last_display_raw: _,
        series_options: _,
        selected_series: _,
        hidden_overlays: _,
        main_hidden: _,
        solid_lines: _,
        time_window_secs: _,
        live: _,
        view_center: _,
        frozen_width: _,
        cursors: _,
        show_mean: _,
        show_envelope: _,
        envelope_window: _,
        show_ref_line: _,
        show_crossings: _,
        focus_ref_field: _,
        show_histogram: _,
        histogram_sizes: _,
        dash_phases: _,
        // A gesture in screen space, or a request the app takes in the
        // frame it is made.
        minimap_drag: _,
        swipe: _,
        box_zoom: _,
        menu_focus_pending: _,
        menu_action: _,
        trace_choice: _,
        clicked_marker: _,
        histogram_resized: _,
    } = &g;
    assert_eq!(current_unit, "V");
    let values: Vec<f64> = history.iter().map(|p| p.value).collect();
    assert_eq!(values, [0.95, 1.01]);
    assert_eq!((y_axis.min.value(), y_axis.max.value()), (0.9, 1.0));
    assert_eq!(ref_lines.values(), [0.92]);
    let kept = |label: &str| overlays.iter().find(|o| o.label == label).expect(label);
    assert_eq!(kept("Hz").unit, "kHz");
    assert_eq!(kept("Hz").points.front().and_then(|p| p.value), Some(0.999));
    assert_eq!(kept("Min").unit, "mV", "left in the old prefix");
    assert_eq!(y_axis.pins.get("kHz"), Some(&(0.9, 1.1)));
    assert_eq!(y_axis.pins.get("Hz"), None, "no trace left in Hz");
    assert_eq!(y_axis.pins.get("mV"), Some(&(900.0, 1000.0)));
    assert!(*history_epoch > epoch, "figures cached on the old values");
    assert!(minimap_level.is_none());
}
