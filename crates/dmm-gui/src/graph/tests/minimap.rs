//! The minimap bucket level.

use super::*;
use crate::graph::minimap::{decimate_columns, level_polyline};

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
