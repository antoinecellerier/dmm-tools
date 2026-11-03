//! Sub-value overlays, and the series the toolbar offers to plot.

use super::*;

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
        main_label: Some(dmm_lib::measurement::MainLabel::Dc),
        levels: false,
        overlays: if ac.is_some() { &overlays } else { &[] },
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
        main_label: Some(dmm_lib::measurement::MainLabel::Dc),
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
    assert_eq!(g.breaks.kind, Some(GapKind::Overload));
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
    assert_eq!(g.integral_over(0..g.history.len()), None);
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

/// Frequency is drawn beside the voltage on an axis of its own, and picking
/// it swaps the two traces like a same-unit switch: both keep their past,
/// and the left axis moves to Hz, dropping a range pinned for volts.
#[test]
fn a_series_change_to_another_unit_swaps_the_traces() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |s: u64| t0 + Duration::from_secs(s);
    for i in 0..3 {
        push_vac_hz(&mut g, at(i), None, 230.0 + i as f64, (50.0, "Hz"));
    }
    assert_eq!(
        key_names(&g),
        vec!["Main (V)", "Frequency (Hz)", "Period (ms)"],
        "Hz and ms on axes of their own"
    );
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;

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
    assert_eq!(
        key_names(&g),
        vec!["Frequency (Hz)", "Main (V)", "Period (ms)"],
        "the volts take Frequency's place, on a right axis"
    );
    assert_eq!(g.origin, Some(t0), "the time axis stays where it was");
    assert!(
        !g.y_axis.fixed && !g.y_axis.user_set,
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

/// A trace in another unit has an axis of its own, so it must not stretch
/// the plotted unit's auto Y range: a 50 Hz frequency beside a 230 V
/// reading.
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
/// Period kept and Max and Average already in, takes the slot of one in
/// another unit.
#[test]
fn a_sub_value_in_the_plotted_unit_takes_the_slot_of_one_in_another() {
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
    assert_eq!(drawn, vec!["Average", "Max", "Min", "Period"]);
}

/// Colours and line styles go to the kept traces in turn, so hiding one
/// leaves the others as they were.
#[test]
fn the_overlay_palette_follows_the_kept_order() {
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
    let slots = |g: &Graph| -> Vec<(usize, String)> {
        g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY)
            .into_iter()
            .map(|(k, label, _)| (k, label))
            .collect()
    };
    let named = |pairs: &[(usize, &str)]| -> Vec<(usize, String)> {
        pairs.iter().map(|&(k, l)| (k, l.to_string())).collect()
    };
    assert_eq!(
        slots(&g),
        named(&[(0, "Frequency"), (1, "Period"), (2, "Max")])
    );
    g.toggle_overlay_hidden("Period".to_string());
    assert_eq!(slots(&g), named(&[(0, "Frequency"), (2, "Max")]));
}

/// Solid lines take a sub-value's pattern away and keep its colour, but a
/// fourth, sharing the first one's colour, keeps its pattern; patterned
/// lines are never solid, as colour alone isn't enough.
#[test]
fn solid_lines_keep_colours_and_drop_patterns() {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    for k in 0..4 {
        let (patterned_color, patterned) = Graph::overlay_color_and_style(&tc, k, false);
        let (solid_color, solid) = Graph::overlay_color_and_style(&tc, k, true);
        assert_ne!(patterned, egui_plot::LineStyle::Solid);
        assert_eq!(solid_color, patterned_color);
        if k < 3 {
            assert_eq!(solid, egui_plot::LineStyle::Solid);
        } else {
            assert_eq!(tc.graph_overlay(k), tc.graph_overlay(0));
            assert_eq!(solid, patterned, "the fourth keeps its pattern");
        }
    }
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
