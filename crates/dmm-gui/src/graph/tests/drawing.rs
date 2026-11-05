//! Drawing the traces: solid lines, thinning, dash patterns; the toolbar
//! height and the spoken label.

use super::*;
use crate::graph::pattern::tests::ramp;
use crate::graph::pattern::{DashPhases, PatternKey};
use crate::graph::render::thin_for_drawing;

/// A solid line is drawn a segment at a time, so a staircase too fine for
/// epaint's mitred joins keeps its width: one segment per straight run, a
/// dot rounding each corner, and a dot for a lone point, as egui_plot draws
/// it.
#[test]
fn a_solid_line_draws_one_segment_per_straight_run() {
    use crate::graph::pattern::SegmentedLine;
    use egui_plot::{Line, PlotBounds, PlotItem, PlotPoints, PlotTransform};
    let frame = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0));
    let bounds = PlotBounds::from_min_max([0.0, 0.0], [10.0, 10.0]);
    let transform = PlotTransform::new(frame, bounds, false);
    let ctx = crate::app::styled_context();
    headless_frame(&ctx, None, None, vec![], |ui| {
        let shapes_of = |points: Vec<[f64; 2]>| {
            let mut shapes = Vec::new();
            let line =
                SegmentedLine::new(Line::new("", PlotPoints::new(points)), egui::Color32::RED);
            line.shapes(ui, &transform, &mut shapes);
            shapes
        };
        let staircase = shapes_of(vec![[1.0, 1.0], [1.1, 1.0], [1.2, 2.0], [1.3, 2.0]]);
        let count = |is: fn(&egui::Shape) -> bool| staircase.iter().filter(|s| is(s)).count();
        let segments = count(|s| matches!(s, egui::Shape::LineSegment { .. }));
        let joins = count(|s| matches!(s, egui::Shape::Circle(_)));
        assert_eq!((segments, joins, staircase.len()), (3, 2, 5));
        let flat = shapes_of(vec![[0.0, 1.0], [1.0, 1.0], [2.0, 1.0], [3.0, 1.0]]);
        assert_eq!(flat.len(), 1, "a straight run is one segment, no corner");
        let back = shapes_of(vec![[1.0, 1.0], [3.0, 1.0], [2.0, 1.0]]);
        assert_eq!(back.len(), 3, "doubling back keeps its corner");
        assert!(matches!(
            shapes_of(vec![[5.0, 5.0]])[..],
            [egui::Shape::Circle(_)]
        ));
    });
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
        let ctx = crate::app::styled_context();
        let mut count = 0;
        // The axes settle on the first frame.
        for _ in 0..2 {
            let out = headless_frame(&ctx, Some(screen), None, vec![], |ui| {
                g.show_main(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
            });
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

/// A frame forgets the anchors outside its slice and the lines the last
/// frame did not draw; clearing the graph forgets them all.
#[test]
fn dash_phases_are_pruned_and_cleared() {
    let mut phases = DashPhases::default();
    let times: Vec<f64> = (0..=40).map(|i| f64::from(i) * 0.25).collect();
    let screen = ramp(&times, 30.0, 0.0, 0.0);
    let (a, b) = (PatternKey::EnvelopeMax, PatternKey::EnvelopeMin);
    phases.carry(&a, &times, &screen, 2.0, 16.0, false);
    phases.carry(&b, &times, &screen, 2.0, 16.0, false);
    phases.begin_frame(3.0, 20.0);
    assert_eq!(phases.phase_at(&a, 2.0), None);
    assert!(phases.phase_at(&a, 4.0).is_some());
    phases.carry(&a, &times, &screen, 2.0, 16.0, false);
    phases.begin_frame(3.0, 20.0);
    assert!(phases.phase_at(&a, 4.0).is_some());
    assert_eq!(phases.phase_at(&b, 4.0), None, "not drawn last frame");
    phases.begin_frame(3.0, 20.0);
    assert!(phases.is_empty());

    let (mut g, _) = graph_over(30);
    g.dash_phases.carry(&a, &times, &screen, 2.0, 16.0, false);
    assert!(!g.dash_phases.is_empty());
    g.clear();
    assert!(g.dash_phases.is_empty());
}

/// The envelope reaches one point back past its range, with that point's
/// own trailing window.
#[test]
fn build_envelope_emits_the_point_before_its_range() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let values = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
    for (i, &v) in values.iter().enumerate() {
        g.push(v, t0 + Duration::from_secs(i as u64), "DC V", "V", None);
    }
    let (min_pts, max_pts) = g.build_envelope(3.5, 7.0, 2.5);
    // t = 3 sees 1.0, 4.0 and 1.0 over [0.5, 3].
    assert_eq!(min_pts[0], [3.0, 1.0]);
    assert_eq!(max_pts[0], [3.0, 4.0]);
    assert_eq!(min_pts.len(), 5);
}

/// The main graph's shapes in `color` below `below`, clear of the key, each
/// as its points: a dash's two ends, or a dot's centre.
fn shapes_in(
    out: &[egui::epaint::ClippedShape],
    color: egui::Color32,
    below: f32,
) -> Vec<Vec<egui::Pos2>> {
    out.iter()
        .filter_map(|c| match &c.shape {
            // A sliver of a dash past a bend comes and goes with rounding.
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == color && (points[1] - points[0]).length() > 0.05 =>
            {
                Some(points.to_vec())
            }
            egui::Shape::Circle(d) if d.fill == color => Some(vec![d.center]),
            _ => None,
        })
        .filter(|p| p.iter().all(|p| p.y > below))
        .collect()
}

/// A graph drawing a sine with two sub-values, one dashed and one dotted,
/// its min/max envelope and a reference line, on a pinned Y axis, with `samples` of them
/// 40 ms apart.
fn patterned_graph(samples: u32) -> (Graph, Instant) {
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    g.show_envelope = true;
    g.show_ref_line = true;
    g.ref_lines.set(&[0.25]);
    let t0 = Instant::now();
    push_patterned(&mut g, t0, 0, samples);
    // After the first sample, whose new mode lets go of a pinned axis.
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(-2.0);
    // The traces stay in the lower half, clear of the key.
    g.y_axis.max.set(6.0);
    (g, t0)
}

fn push_patterned(g: &mut Graph, t0: Instant, from: u32, to: u32) {
    for i in from..to {
        let t = f64::from(i) * 0.04;
        push_aux(
            g,
            (t * 1.3).sin(),
            t0 + Duration::from_millis(u64::from(i) * 40),
            None,
            &[
                ("T1", Some((t * 0.9).sin() * 1.5)),
                ("T2", Some((t * 2.1).cos())),
            ],
        );
    }
}

/// Each patterned line's shapes in one frame of the main graph, and the
/// plot's area.
type PatternFrame = ([Vec<Vec<egui::Pos2>>; 4], egui::Rect);

/// Draw the main graph and keep the shapes of its two sub-values, its
/// envelope's edges and its reference line, below the key.
fn pattern_frame(g: &mut Graph, ctx: &egui::Context, tc: &ThemeColors) -> PatternFrame {
    let markers = crate::markers::Markers::default();
    let mut shapes = Vec::new();
    // Twice: the axes settle on the first, and a discarded frame is run
    // again just so.
    for _ in 0..2 {
        let out = headless_frame(ctx, Some(gesture_screen()), None, vec![], |ui| {
            g.show_main(ui, &crate::l10n::english(), tc, &markers, None, NO_DIGITS)
        });
        shapes = out.shapes;
    }
    let plot = g.plot_rect.expect("drawn");
    let below = plot.top() + plot.height() * 0.4;
    let colors = [
        tc.graph_overlay(0),
        tc.graph_overlay(1),
        tc.graph_envelope(),
        tc.graph_ref(),
    ];
    (colors.map(|c| shapes_in(&shapes, c, below)), plot)
}

/// Every dash and dot `now` shows that `before` showed too is where it
/// was, `dx` px to the left.
fn assert_moved(now: &PatternFrame, before: &PatternFrame, dx: f32) {
    let plot = now.1;
    // Clear of the edges, and of where the older frame's lines ended.
    let lo = plot.left() + 20.0 + (-dx).max(0.0);
    let hi = plot.right() - 20.0 - dx.max(0.0);
    for (k, (now, before)) in now.0.iter().zip(&before.0).enumerate() {
        let shown: Vec<_> = now
            .iter()
            .filter(|p| p.iter().all(|p| (lo..hi).contains(&p.x)))
            .collect();
        assert!(shown.len() > 20, "line {k}: {} shapes", shown.len());
        for shape in shown {
            let moved = before.iter().any(|b| {
                b.len() == shape.len()
                    && b.iter()
                        .zip(shape)
                        .all(|(b, s)| (b.x - dx - s.x).abs() < 0.02 && (b.y - s.y).abs() < 0.02)
            });
            assert!(
                moved,
                "line {k}: {shape:?} was not drawn {dx} px to its right"
            );
        }
    }
}

/// Live view: as readings arrive, every dash and dot of the sub-values
/// and the envelope moves left with the data, by just what the time it
/// was drawn at did. Before, each sample leaving the left edge moved the
/// pattern's start and it crawled along the curve.
#[test]
fn dashes_move_with_the_data_in_live_view() {
    let (mut g, t0) = patterned_graph(751);
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = crate::app::styled_context();
    let mut last = pattern_frame(&mut g, &ctx, &tc);
    // Steps of one sample and of several, past a few anchors.
    for (step, n) in [(1, 1), (1, 7), (7, 6), (13, 3)] {
        for _ in 0..n {
            let from = (g.data_time_range().1 / 0.04).round() as u32 + 1;
            push_patterned(&mut g, t0, from, from + step);
            let next = pattern_frame(&mut g, &ctx, &tc);
            let px_per_s = f64::from(next.1.width()) / g.time_window_secs;
            assert_moved(&next, &last, (f64::from(step) * 0.04 * px_per_s) as f32);
            last = next;
        }
    }
}

/// Dragging the view either way moves the pattern with the line, and
/// coming back draws it as it was.
#[test]
fn dashes_move_with_the_view_both_ways() {
    let (mut g, _) = patterned_graph(1500);
    let tc = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = crate::app::styled_context();
    g.live = false;
    g.view_center = 30.0;
    let start = pattern_frame(&mut g, &ctx, &tc);
    let mut last = (30.0, start.clone());
    for center in [30.3, 31.1, 33.7, 31.9, 30.6, 30.0] {
        g.view_center = center;
        let next = pattern_frame(&mut g, &ctx, &tc);
        let px_per_s = f64::from(next.1.width()) / g.time_window_secs;
        assert_moved(&next, &last.1, ((center - last.0) * px_per_s) as f32);
        last = (center, next);
    }
    assert_moved(&last.1, &start, 0.0);
}

/// The toolbar keeps its height as its fields come and go, so switching
/// Min/Max, Ref or Y:Fixed on doesn't move the plot below it — which would
/// rescale the graph and shift every line's pattern.
#[test]
fn toolbar_fields_leave_its_height_alone() {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = crate::app::styled_context();
    let height = |g: &mut Graph| {
        let mut h = 0.0;
        // The second frame: the first lays out the fonts.
        for _ in 0..2 {
            headless_frame(&ctx, Some(gesture_screen()), None, vec![], |ui| {
                h = ui
                    .scope(|ui| g.show_toolbar(ui, &crate::l10n::english(), &tc))
                    .response
                    .rect
                    .height();
            });
        }
        h
    };
    let (mut g, _) = graph_over(30);
    let plain = height(&mut g);
    g.show_envelope = true;
    assert_eq!(height(&mut g), plain, "Min/Max window field");
    g.show_ref_line = true;
    assert_eq!(height(&mut g), plain, "reference values field");
    g.y_axis.fixed = true;
    assert_eq!(height(&mut g), plain, "fixed Y bounds fields");
}

/// The plot's name for a screen reader, whole, its units said by name.
#[test]
fn the_plots_spoken_label_reads_whole() {
    let (mut g, _) = graph_over(30);
    let ctx = crate::app::styled_context();
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(
        g.a11y_label,
        "Measurement plot. 10 second window. Y axis 1.000 to 1.000 volts. 31 samples. live. \
         last reading 1.0000 volts."
    );
    g.show_histogram = true;
    g.live = false;
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(
        g.a11y_label,
        "Measurement plot. 10 second window. Y axis 1.000 to 1.000 volts. 31 samples. \
         Histogram of 11 visible readings, standard deviation 0.0000 volts, most at 1.0000 to \
         1.0000 volts (11). paused. last reading 1.0000 volts."
    );

    // Traces drawn beside it, one on an axis of its own, and the readings
    // between the cursors.
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    let t0 = Instant::now();
    for i in 0..=10u32 {
        let overlays = [
            ("T2", "\u{00B0}C", Some(30.0)),
            ("Frequency", "Hz", Some(50.0 + f64::from(i % 2))),
        ];
        g.push_sample(PlotSample {
            value: Some(20.0 + f64::from(i % 3)),
            timestamp: t0 + Duration::from_secs(u64::from(i)),
            mode: "Temp",
            unit: "\u{00B0}C",
            display_raw: None,
            series: None,
            main_label: None,
            levels: false,
            overlays: &overlays,
        });
    }
    g.show_histogram = true;
    g.cursors.active = true;
    g.cursors.a = Some(2.0);
    g.cursors.b = Some(5.0);
    graph_frame(&mut g, &ctx, vec![]);
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(
        g.a11y_label,
        "Measurement plot. 10 second window. Y axis 19.000 to 31.000 degrees C. 11 samples. \
         Also showing T2, Frequency in hertz on a right axis. Histogram of 11 visible readings, \
         standard deviation 0.8312 degrees C, most at 20 degrees C (4); 4 readings between the \
         cursors, standard deviation 0.9574 degrees C. live. last reading 21.0000 degrees C."
    );
}
