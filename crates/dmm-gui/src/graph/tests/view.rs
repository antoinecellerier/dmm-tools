//! The view: its bounds, time windows, zoom and pan, and the NCV levels.

use super::*;
use crate::graph::render::{stepped, whole_number_marks};

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

/// The visible window's figures are kept while the readings in view stay
/// the same, the view moving over them included, and computed again when
/// one enters it.
#[test]
fn visible_figures_follow_the_readings_in_view() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, v) in [1.0, 2.0, 3.0].iter().enumerate() {
        g.push(*v, t0 + Duration::from_secs(i as u64), "DC V", "V", None);
    }
    g.live = false;
    g.time_window_secs = 10.0;
    g.view_center = 5.0;
    assert_eq!(g.visible_stats().map(|s| s.count), Some(3));
    let key = g.visible_figures.as_ref().map(|f| f.key);
    g.view_center = 4.5;
    assert_eq!(g.visible_stats().map(|s| s.count), Some(3));
    assert_eq!(
        g.visible_figures.as_ref().map(|f| f.key),
        key,
        "same readings"
    );
    g.push(4.0, t0 + Duration::from_secs(3), "DC V", "V", None);
    let s = g.visible_stats().expect("four in view");
    assert_eq!((s.count, s.max), (4, Some(4.0)));
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
    assert!(!g.y_axis.fixed);
    g.apply_bbox_zoom((10.0, 5.0), (20.0, 2.0));
    assert!(!g.live);
    assert!(g.y_axis.fixed);
    assert!(g.y_axis.user_set);
    assert!((g.view_center - 15.0).abs() < 1e-9);
    assert!((g.time_window_secs - 10.0).abs() < 1e-9);
    assert!((g.y_axis.min.value() - 2.0).abs() < 1e-9);
    assert!((g.y_axis.max.value() - 5.0).abs() < 1e-9);
    assert_eq!(g.y_axis.min.text(), "2.0000");
    assert_eq!(g.y_axis.max.text(), "5.0000");
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
    assert!(g.y_axis.fixed);
    g.reset_view();
    assert!(g.live);
    assert!(!g.y_axis.fixed);
    assert!(!g.y_axis.user_set);
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

/// A graph whose history has wrapped round its ring buffer, so
/// `as_slices` returns two halves of four points each: a bounded history
/// evicts from the front as it takes new points at the back.
fn wrapped_graph() -> Graph {
    let mut g = Graph::new();
    g.set_max_points(8);
    let t0 = Instant::now();
    for i in 0..10_000 {
        g.push(i as f64, t0 + Duration::from_secs(i), "DC V", "V", None);
        let (a, b) = g.history.as_slices();
        if (a.len(), b.len()) == (4, 4) {
            return g;
        }
    }
    panic!("the history never wrapped");
}

/// Times from before the first point to past the last, in quarter seconds,
/// landing on points and between them in both halves of the ring.
fn query_times(g: &Graph) -> Vec<f64> {
    let first = g.elapsed_secs(g.history.front().unwrap().time);
    let last = g.elapsed_secs(g.history.back().unwrap().time);
    let steps = ((last - first + 4.0) * 4.0) as usize;
    (0..=steps).map(|k| first - 2.0 + k as f64 * 0.25).collect()
}

#[test]
fn visible_index_range_on_a_wrapped_history_matches_a_linear_scan() {
    let g = wrapped_graph();
    let times: Vec<f64> = g.history.iter().map(|p| g.elapsed_secs(p.time)).collect();
    let queries = query_times(&g);
    for &x_min in &queries {
        for &x_max in queries.iter().filter(|&&x| x >= x_min) {
            let start = times
                .iter()
                .position(|&t| t >= x_min)
                .unwrap_or(times.len());
            let end = times.iter().position(|&t| t > x_max).unwrap_or(times.len());
            assert_eq!(
                g.visible_index_range(x_min, x_max),
                (start, end),
                "[{x_min}, {x_max}]"
            );
        }
    }
}

#[test]
fn nearest_point_on_a_wrapped_history_matches_a_linear_scan() {
    let g = wrapped_graph();
    for t in query_times(&g) {
        let expected = g
            .history
            .iter()
            .map(|p| (g.elapsed_secs(p.time), p.value))
            .min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs()));
        assert_eq!(g.nearest_point(t), expected, "t = {t}");
    }
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
fn leaving_live_freezes_the_view_where_it_was() {
    // Ten minutes on a 60 s window, with the centre where a reset left it:
    // turning LIVE off must hold the window live was showing, not jump to
    // the session's start.
    let mut g = Graph::new();
    g.time_window_secs = 60.0;
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_secs(600), "DC V", "V", None);
    g.view_center = 0.0;
    let live_bounds = g.view_bounds();
    g.leave_live();
    assert!(!g.live);
    let (x_min, x_max) = g.view_bounds();
    assert!(
        (x_min - live_bounds.0).abs() < 1e-9,
        "{x_min} vs {live_bounds:?}"
    );
    assert!(
        (x_max - live_bounds.1).abs() < 1e-9,
        "{x_max} vs {live_bounds:?}"
    );
}

#[test]
fn leaving_live_before_the_window_fills_keeps_the_view_exactly() {
    // 20 s of readings on a 60 s window: live shows 0–20 s, and so must the
    // frozen view, with the window setting untouched and later readings
    // left out of it.
    let mut g = Graph::new();
    g.time_window_secs = 60.0;
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_secs(20), "DC V", "V", None);
    let live_bounds = g.view_bounds();
    assert_eq!(live_bounds, (0.0, 20.0));
    g.leave_live();
    assert_eq!(g.view_bounds(), live_bounds);
    assert_eq!(g.time_window_secs, 60.0);
    g.push(3.0, t0 + Duration::from_secs(30), "DC V", "V", None);
    assert_eq!(g.view_bounds(), live_bounds);
    // A pan slides the frozen width over the later readings.
    g.apply_pan(-5.0);
    assert_eq!(g.view_bounds(), (5.0, 25.0));
    assert_eq!(g.time_window_secs, 60.0);
    // An export saves the view as shown.
    assert_eq!(g.view_state(t0).window, Some(20.0));
}

/// 20 s of readings on a 60 s window, LIVE turned off, then 20 s more.
fn frozen_short_session() -> Graph {
    let mut g = Graph::new();
    g.time_window_secs = 60.0;
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_secs(20), "DC V", "V", None);
    g.leave_live();
    g.push(3.0, t0 + Duration::from_secs(40), "DC V", "V", None);
    g
}

#[test]
fn a_window_change_lets_go_of_the_frozen_width_for_good() {
    // Back on the frozen window and centre, the later readings still show.
    let mut g = frozen_short_session();
    g.cycle_time_window(-1);
    g.cycle_time_window(1);
    assert_eq!(g.time_window_secs, 60.0);
    assert_eq!(g.view_bounds(), (0.0, 60.0));
}

#[test]
fn a_pan_there_and_back_keeps_the_frozen_width() {
    let mut g = frozen_short_session();
    g.apply_pan(-5.0);
    g.apply_pan(5.0);
    g.scroll_view(0.25);
    assert!(!g.live);
    assert_eq!(g.view_bounds(), (5.0, 25.0));
}

#[test]
fn picking_the_current_window_shows_it() {
    let mut g = frozen_short_session();
    g.set_time_window(60.0);
    assert_eq!(g.view_bounds(), (0.0, 60.0));
}

#[test]
fn a_pan_through_live_and_back_keeps_the_width_live_showed() {
    // Data now 0–40 s on a 60 s window: live shows all 40 s, and a pan back
    // out of it freezes that width rather than jumping to the window's.
    let mut g = frozen_short_session();
    g.apply_pan(-30.0);
    assert!(g.live);
    assert_eq!(g.view_bounds(), (0.0, 40.0));
    g.apply_pan(5.0);
    assert!(!g.live);
    assert_eq!(g.view_bounds(), (0.0, 40.0));
    g.apply_pan(-2.0);
    assert_eq!(g.view_bounds(), (0.0, 40.0));
}

#[test]
fn going_live_forgets_the_frozen_width() {
    // Live again, then out of it by Home a while later: the window's width,
    // not the one the first freeze took.
    let mut g = frozen_short_session();
    g.set_live(true);
    g.push(
        4.0,
        g.origin.unwrap() + Duration::from_secs(600),
        "DC V",
        "V",
        None,
    );
    g.jump_to_start();
    assert_eq!(g.view_bounds(), (0.0, 60.0));
}

#[test]
fn zooming_in_on_a_frozen_short_session_narrows_what_it_shows() {
    // The plot shows 0–20 s: one notch in at 10 s narrows that, around 10 s.
    let mut g = frozen_short_session();
    g.zoom_time(1.1, Some(10.0));
    let (lo, hi) = g.view_bounds();
    assert!((hi - lo - 20.0 / 1.1).abs() < 1e-9, "{lo}..{hi}");
    assert!(((lo + hi) / 2.0 - 10.0).abs() < 1e-9, "{lo}..{hi}");
}

#[test]
fn zooming_in_from_live_on_a_short_session_narrows_what_it_shows() {
    let mut g = Graph::new();
    g.time_window_secs = 60.0;
    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    g.push(2.0, t0 + Duration::from_secs(20), "DC V", "V", None);
    g.zoom_time(2.0, Some(20.0));
    assert!(!g.live);
    assert_eq!(g.view_bounds(), (10.0, 20.0));
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
    let mut g = graph_over(200).0;
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
    let mut g = graph_over(200).0;
    g.live = false;
    g.view_center = 195.0; // right edge at data_max
    g.apply_pan(3.0);
    assert!(!g.live);
    assert!((g.view_center - 192.0).abs() < 1e-9);
}

#[test]
fn apply_pan_toward_newer_below_live_edge_does_not_snap() {
    // Drag toward newer but still short of the live edge — just moves
    // view_center, does not re-enter live.
    let mut g = graph_over(200).0;
    g.live = false;
    g.view_center = 100.0; // right edge at 105, well below data_max
    g.apply_pan(-2.0);
    assert!(!g.live);
    assert!((g.view_center - 102.0).abs() < 1e-9);
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
fn is_view_zoomed_reflects_state() {
    let mut g = Graph::new();
    assert!(!g.is_view_zoomed());
    g.live = false;
    assert!(g.is_view_zoomed());
    g.live = true;
    g.y_axis.fixed = true;
    assert!(g.is_view_zoomed());
}

/// A view from a file that lands on none of the readings, or is wider than
/// any the graph shows, still draws: its start is left out and its window
/// narrowed.
#[test]
fn a_view_past_the_readings_still_draws() {
    let (mut g, t0) = graph_over(30);
    let markers = marked(&[t0 + Duration::from_secs(5)]);
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = crate::app::styled_context();
    for saved in [
        r#"{"start": 1e20, "window": 1e20}"#,
        r#"{"start": -1e20}"#,
        r#"{"start": 0, "window": 1e19}"#,
    ] {
        let view = serde_json::from_str(saved).expect("a view");
        g.apply_view_state(&view, t0);
        assert!(
            g.time_window_secs <= super::super::minimap::MAX_WINDOW_SECS,
            "{saved}"
        );
        headless_frame(&ctx, None, None, vec![], |ui| {
            g.show_main(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
        });
    }
    // A start on the readings still lands, its window kept.
    g.apply_view_state(
        &serde_json::from_str(r#"{"start": 3, "window": 20}"#).expect("a view"),
        t0,
    );
    assert_eq!((g.live, g.view_bounds()), (false, (3.0, 23.0)));
}
