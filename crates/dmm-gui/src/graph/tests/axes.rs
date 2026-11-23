//! The axes: the tick labels the plot's own two carry, and the right axes,
//! one per sub-value unit, their room and their pins.

use super::*;
use crate::graph::render::stacks_that_fit;

/// One sample of a V reading beside a sub-value in each of `units`.
fn push_units(g: &mut Graph, t: Instant, units: &[(&'static str, &'static str)]) {
    let overlays: Vec<_> = units.iter().map(|&(l, u)| (l, u, Some(1.0))).collect();
    g.push_sample(PlotSample {
        value: Some(230.0),
        timestamp: t,
        mode: "V AC",
        unit: "V",
        display_raw: None,
        series: None,
        main_label: None,
        levels: false,
        overlays: &overlays,
    });
}

const FIVE_UNITS: [(&str, &str); 4] = [
    ("Frequency", "Hz"),
    ("Period", "ms"),
    ("Current", "A"),
    ("Impedance", "\u{2126}"),
];

/// Four units at most: a fifth is refused an axis until one is hidden, and
/// a hidden one waiting for a free axis is refused too.
#[test]
fn a_fifth_unit_waits_for_a_free_axis() {
    let mut g = Graph::new();
    push_units(&mut g, Instant::now(), &FIVE_UNITS);
    assert_eq!(g.axis_units(), vec!["V", "Hz", "ms", "A"]);
    let refusal = |g: &Graph, label: &str| {
        let o = g.overlays.iter().find(|o| o.label == label).expect("kept");
        g.axis_refusal(o)
    };
    assert_eq!(refusal(&g, "Impedance"), Some(AxisRefusal::Full));
    assert_eq!(refusal(&g, "Current"), None);
    assert!(!drawn_overlay_labels(&g).contains(&"Impedance".to_string()));

    g.toggle_overlay_hidden("Period".to_string());
    assert_eq!(g.axis_units(), vec!["V", "Hz", "A", "\u{2126}"]);
    assert_eq!(refusal(&g, "Impedance"), None);
    assert_eq!(refusal(&g, "Period"), Some(AxisRefusal::Full));
}

/// A sub-value that has stopped holds no axis once it is out of view, so a
/// live one can take it.
#[test]
fn a_stopped_sub_value_frees_its_axis() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_units(&mut g, t0, &FIVE_UNITS);
    assert_eq!(g.axis_units(), vec!["V", "Hz", "ms", "A"]);
    let rest = [FIVE_UNITS[0], FIVE_UNITS[1], FIVE_UNITS[3]];
    for s in 1..=120 {
        push_units(&mut g, t0 + Duration::from_secs(s), &rest);
    }
    assert_eq!(g.axis_units(), vec!["V", "Hz", "ms", "\u{2126}"]);
}

/// Every right axis shares one column, so a graph has room for all of them
/// or, too narrow, for the plotted unit's alone; the traces keep their place
/// for when it widens.
#[test]
fn a_narrow_graph_drops_the_right_column() {
    let mut g = Graph::new();
    push_units(&mut g, Instant::now(), &FIVE_UNITS[..3]);
    let refusal = |g: &Graph, label: &str| {
        let o = g.overlays.iter().find(|o| o.label == label).expect("kept");
        g.axis_refusal(o)
    };
    g.fit_axes_to(LEFT_AXIS_ROOM + MIN_PLOT_WIDTH + RIGHT_AXIS_ROOM);
    assert_eq!(g.drawn_units(), vec!["V", "Hz", "ms", "A"]);
    g.fit_axes_to(LEFT_AXIS_ROOM + MIN_PLOT_WIDTH + RIGHT_AXIS_ROOM - 1.0);
    assert_eq!(g.drawn_units(), vec!["V"]);
    assert!(drawn_overlay_labels(&g).is_empty());
    assert_eq!(refusal(&g, "Frequency"), Some(AxisRefusal::Narrow));
}

/// A right axis frames its own traces only, in its own unit.
#[test]
fn each_right_axis_frames_its_own_unit() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, hz) in [49.9, 50.1].into_iter().enumerate() {
        push_vac_hz(
            &mut g,
            t0 + Duration::from_secs(i as u64),
            None,
            230.0,
            (hz, "Hz"),
        );
    }
    let targets = g.secondary_targets(f64::NEG_INFINITY, f64::INFINITY);
    let units: Vec<&str> = targets.iter().map(|(u, _)| u.as_str()).collect();
    assert_eq!(units, vec!["Hz", "ms"]);
    assert_eq!(targets[0].1, (49.9, 50.1), "its data, unpadded");
}

/// Stacked right-axis labels keep to the plot's height and never overlap:
/// on a short plot a line too close to the last keeps no stack.
#[test]
fn stacked_labels_fit_or_are_left_out() {
    let room = egui::Rangef::new(100.0, 310.0);
    assert_eq!(
        stacks_that_fit(&[290.0, 110.0, 200.0], 30.0, room),
        vec![200.0, 290.0],
        "110 spills above the plot"
    );
    assert_eq!(
        stacks_that_fit(&[150.0, 170.0, 190.0], 30.0, room),
        vec![150.0, 190.0],
        "170 would overlap 150's stack"
    );
}

/// Y:Fixed pins every axis at the range it frames now — its target, not the
/// round range its axis widens that to — so the axes don't move when it is
/// pressed, nor grow at each press.
#[test]
fn fixing_y_pins_every_axis_where_it_is() {
    let mut g = graph_with_hz();
    let (x_min, x_max) = g.view_bounds();
    let before = g.secondary_targets(x_min, x_max);
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert_eq!(g.secondary_targets(x_min, x_max), before);

    g.y_axis.fixed = false;
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert_eq!(g.secondary_targets(x_min, x_max), before, "no growth");
}

/// Y:Fixed pressed in a window too narrow for the right axes still pins
/// them, for when it widens.
#[test]
fn fixing_y_in_a_narrow_window_pins_the_shed_axes() {
    let mut g = graph_with_hz();
    let (x_min, x_max) = g.view_bounds();
    let fitted = g.secondary_targets(x_min, x_max);
    g.fit_axes_to(100.0);
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert_eq!(g.y_axis.pins.len(), fitted.len());
}

/// A view restored with a fixed range fixes the left axis only; pressing
/// Y:Fixed again fixes the right axes too, keeping the restored bounds.
#[test]
fn y_fixed_after_a_restored_range_pins_the_right_axes() {
    let mut g = graph_with_hz();
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(200.0);
    g.y_axis.max.set(260.0);
    g.y_axis.fixed = false;
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert_eq!((g.y_axis.min.value(), g.y_axis.max.value()), (200.0, 260.0));
    assert!(g.y_axis.pins.contains_key("Hz"));
}

/// A sub-value stepping to another unit drops its axis's fixed range: it
/// was set for the old unit, and would be wrong if that unit came back.
#[test]
fn a_unit_step_drops_its_axis_pin() {
    let mut g = graph_with_hz();
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert!(g.y_axis.pins.contains_key("Hz"));
    push_vac_hz(
        &mut g,
        Instant::now() + Duration::from_secs(3),
        None,
        230.0,
        (1.001, "kHz"),
    );
    assert!(!g.y_axis.pins.contains_key("Hz"));
    assert!(g.y_axis.pins.contains_key("ms"), "the others keep theirs");
}

/// A decade step of the plotted series keeps no pin under its new unit,
/// which its fields frame, and gives a trace left in the old prefix the
/// range it was framed by.
#[test]
fn a_plotted_step_re_keys_the_pins() {
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
    push(
        &mut g,
        0,
        950.0,
        "mV",
        &[("AC", "V", Some(1.2)), ("Min", "mV", Some(940.0))],
    );
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(900.0);
    g.y_axis.max.set(1000.0);
    g.y_axis.pins.insert("V".into(), (1.0, 1.5));
    push(
        &mut g,
        1,
        1.01,
        "V",
        &[("AC", "V", Some(1.2)), ("Min", "mV", Some(940.0))],
    );
    assert_eq!(g.plotted_unit(), "V");
    assert_eq!(
        g.y_axis.pins.get("V"),
        None,
        "the plotted axis takes no pin"
    );
    assert_eq!(g.y_axis.pins.get("mV"), Some(&(900.0, 1000.0)));
    assert_eq!((g.y_axis.min.value(), g.y_axis.max.value()), (0.9, 1.0));
}

/// The plotted series and a kept one stepping opposite ways in one frame:
/// the kept trace keeps its own range, re-expressed, and no trace left in
/// the old prefix means no pin there.
#[test]
fn opposite_steps_in_one_frame_keep_each_axis_framed() {
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
    push(&mut g, 0, 950.0, "mV", &[("AC", "V", Some(1.2))]);
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(900.0);
    g.y_axis.max.set(1000.0);
    g.y_axis.pins.insert("V".into(), (1.0, 1.5));
    push(&mut g, 1, 1.01, "V", &[("AC", "mV", Some(1200.0))]);
    assert_eq!(g.y_axis.pins.get("mV"), Some(&(1000.0, 1500.0)));
    assert_eq!(g.y_axis.pins.get("V"), None);

    // A kept trace stepping off the plotted axis alone takes the fields'
    // range with it.
    let mut g = Graph::new();
    push(&mut g, 0, 950.0, "mV", &[("Min", "mV", Some(940.0))]);
    g.y_axis.fixed = true;
    g.y_axis.user_set = true;
    g.y_axis.min.set(900.0);
    g.y_axis.max.set(1000.0);
    push(&mut g, 1, 960.0, "mV", &[("Min", "V", Some(1.01))]);
    assert_eq!(g.y_axis.pins.get("V"), Some(&(0.9, 1.0)));

    // Nothing left in mV: no pin is made for it.
    let mut g = Graph::new();
    push(&mut g, 0, 950.0, "mV", &[("AC", "V", Some(1.2))]);
    g.y_axis.fixed = true;
    g.y_axis.min.set(900.0);
    g.y_axis.max.set(1000.0);
    push(&mut g, 1, 1.01, "V", &[("AC", "V", Some(1.2))]);
    assert!(g.y_axis.pins.is_empty(), "{:?}", g.y_axis.pins);
}

/// A box zoom pins each right axis to its share of the box, through the
/// axis's map, and Reset Zoom lets them all fit their data again.
#[test]
fn a_box_zoom_pins_every_axis_to_its_share() {
    let mut g = graph_with_hz();
    let (x_min, x_max) = g.view_bounds();
    let fitted = g.secondary_targets(x_min, x_max);
    // The plot shows 229..231 V, gridded every 0.5 V.
    g.axis_maps = fitted
        .iter()
        .map(|(unit, target)| {
            (
                unit.clone(),
                crate::graph::axes::fit_secondary(229.0, 231.0, 0.5, *target),
            )
        })
        .collect();
    let hz = g.axis_maps[0].1;

    g.apply_bbox_zoom((0.5, 230.0), (1.5, 230.5));
    let pinned = g.secondary_targets(x_min, x_max);
    assert_eq!(pinned[0].0, "Hz");
    let (lo, hi) = pinned[0].1;
    assert!((lo - hz.value_at(230.0)).abs() < 1e-9 && (hi - hz.value_at(230.5)).abs() < 1e-9);
    assert!(hi - lo < fitted[0].1.1 - fitted[0].1.0, "zoomed in");

    g.reset_view();
    assert_eq!(g.secondary_targets(x_min, x_max), fitted);
}

/// A switch of **Plot:** into another unit releases the right axes' pins
/// with the left one's: they were set against the old plotted unit.
#[test]
fn a_swap_into_another_unit_releases_every_pin() {
    let mut g = graph_with_hz();
    g.y_axis.fixed = true;
    g.pin_y_axes_to_view();
    assert!(!g.y_axis.pins.is_empty());
    push_vac_hz(
        &mut g,
        Instant::now() + Duration::from_secs(3),
        Some("Frequency"),
        230.0,
        (50.0, "Hz"),
    );
    assert!(!g.y_axis.fixed);
    assert!(g.y_axis.pins.is_empty());
}

/// The tick labels of the main plot's left axis and of its time axis, by
/// where they are drawn around `plot`.
struct TickLabels {
    left: Vec<(String, egui::Rect)>,
    time: Vec<(String, egui::Rect)>,
}

/// One frame of the graph on a `screen`-sized window, and every text it
/// drew with where.
fn frame_texts(
    g: &mut Graph,
    ctx: &egui::Context,
    screen: egui::Vec2,
) -> Vec<(String, egui::Rect)> {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(t) => found.push((
                t.galley.text().to_string(),
                egui::Rect::from_min_size(t.pos, t.galley.size()),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let markers = crate::markers::Markers::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen);
    let out = headless_frame(ctx, Some(screen), None, vec![], |ui| {
        g.show(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
    });
    let mut texts = Vec::new();
    for clipped in &out.shapes {
        walk(&clipped.shape, &mut texts);
    }
    texts
}

/// One frame of the graph on a `screen`-sized window, and the tick labels
/// it drew around its plot.
fn frame_tick_labels(g: &mut Graph, ctx: &egui::Context, screen: egui::Vec2) -> TickLabels {
    let texts = frame_texts(g, ctx, screen);
    let plot = g.plot_rect.expect("the plot is drawn");
    // egui_plot hangs a time label from the plot's bottom edge and centres
    // a left one on its gridline, right of nothing but the plot.
    let (time, rest): (Vec<_>, Vec<_>) = texts
        .into_iter()
        .partition(|(_, r)| (r.top() - plot.bottom()).abs() < 1.0);
    let left = rest
        .into_iter()
        .filter(|(_, r)| r.right() <= plot.left() && plot.y_range().contains(r.center().y))
        .collect();
    TickLabels { left, time }
}

const NARROW: egui::Vec2 = egui::vec2(454.0, 600.0);

/// A plot with no reading has no range on either axis: egui_plot draws one
/// from -1 to 1, and neither axis labels it. The time axis's row carries
/// its origin alone, under the plot's left edge, where and as the axis
/// writes its zero once readings come; they bring the labels without
/// moving the plot. The first reading alone spans no time either, so the
/// origin stays under it until the second.
#[test]
fn an_empty_plot_labels_only_the_time_axis_origin() {
    let ctx = crate::app::styled_context();
    let mut g = Graph::new();
    let mut labels = frame_tick_labels(&mut g, &ctx, NARROW);
    for _ in 0..2 {
        labels = frame_tick_labels(&mut g, &ctx, NARROW);
    }
    assert!(labels.left.is_empty(), "left labels: {:?}", labels.left);
    let [(text, origin)] = &labels.time[..] else {
        panic!("time labels: {:?}", labels.time);
    };
    assert_eq!(text, "0 s");
    let origin = *origin;
    let plot = g.plot_rect.expect("drawn");
    // Centred on the edge like any tick's label, its left half in the
    // value axis's room: inside the window, and short of the plot's right.
    assert!((origin.center().x - plot.left()).abs() < 0.5, "{origin:?}");
    assert!(origin.left() >= 0.0 && origin.right() < plot.right());

    let t0 = Instant::now();
    g.push(1.0, t0, "DC V", "V", None);
    for _ in 0..2 {
        labels = frame_tick_labels(&mut g, &ctx, NARROW);
    }
    let [(text, alone)] = &labels.time[..] else {
        panic!("with one reading: {:?}", labels.time);
    };
    let left = g.plot_rect.expect("drawn").left();
    assert_eq!(text, "0 s");
    assert!((alone.center().x - left).abs() < 0.5, "{alone:?}");

    for i in 1..=50_u64 {
        let at = t0 + Duration::from_millis(i * 100);
        g.push(1.0 + i as f64 * 0.01, at, "DC V", "V", None);
    }
    for _ in 0..3 {
        labels = frame_tick_labels(&mut g, &ctx, NARROW);
    }
    assert!(
        labels.time.iter().any(|(t, _)| t == "2 s"),
        "{:?}",
        labels.time
    );
    let zeros: Vec<_> = labels.time.iter().filter(|(t, _)| t == "0 s").collect();
    let [(_, zero)] = &zeros[..] else {
        panic!("one zero: {:?}", labels.time);
    };
    assert!(
        (zero.min - origin.min).length() < 0.5 && zero.size() == origin.size(),
        "the axis's zero at {zero:?}, the empty plot's at {origin:?}"
    );
    assert!(
        labels.left.iter().any(|(t, _)| t.contains(" V")),
        "{:?}",
        labels.left
    );
    assert_eq!(g.plot_rect, Some(plot), "the labels' row was kept for them");
}

const NO_METER: &str = "Connect a meter to plot its readings";
const WAITING: &str = "Waiting for the first reading";

/// The sentences an empty plot can show, among `texts`.
fn empty_sentences(texts: &[(String, egui::Rect)]) -> Vec<&(String, egui::Rect)> {
    texts
        .iter()
        .filter(|(t, _)| t == NO_METER || t == WAITING)
        .collect()
}

/// An empty plot says what will bring readings, in its middle and to a
/// screen reader: connecting a meter, or waiting for one that is connected.
/// Where neither is true it says nothing, and the first thing the meter
/// sends takes the sentence away.
#[test]
fn an_empty_plot_says_what_brings_readings() {
    let ctx = crate::app::styled_context();
    for (feed, sentence) in [
        (Feed::NoMeter, Some(NO_METER)),
        (Feed::Expected, Some(WAITING)),
        (Feed::Held, None),
    ] {
        let mut g = Graph::new();
        g.feed = feed;
        frame_texts(&mut g, &ctx, NARROW);
        let texts = frame_texts(&mut g, &ctx, NARROW);
        let shown = empty_sentences(&texts);
        let plot = g.plot_rect.expect("drawn");
        match sentence {
            Some(sentence) => {
                let [(text, rect)] = &shown[..] else {
                    panic!("{feed:?}: {shown:?}");
                };
                assert_eq!(text, sentence);
                assert!(plot.contains_rect(*rect), "{rect:?} in {plot:?}");
                assert!((rect.center() - plot.center()).length() < 1.0);
                assert!(
                    g.a11y_label.ends_with(&format!("{sentence}.")),
                    "{}",
                    g.a11y_label
                );
            }
            None => {
                assert!(shown.is_empty(), "{feed:?}: {shown:?}");
                assert!(g.a11y_label.ends_with("no data."), "{}", g.a11y_label);
            }
        }

        g.push(1.0, Instant::now(), "DC V", "V", None);
        let texts = frame_texts(&mut g, &ctx, NARROW);
        assert!(
            empty_sentences(&texts).is_empty(),
            "{feed:?} with a reading"
        );
        assert!(g.a11y_label.contains("last reading"), "{}", g.a11y_label);
    }

    // Over range from the start is no reading plotted, and not a wait for
    // one either.
    let mut g = Graph::new();
    g.feed = Feed::Expected;
    push_overload(&mut g, Instant::now(), "DC V", "V");
    frame_texts(&mut g, &ctx, NARROW);
    let texts = frame_texts(&mut g, &ctx, NARROW);
    assert!(empty_sentences(&texts).is_empty());
}

/// Out of live the view is a span of time whether or not anything was read
/// in it, and the axis labels it, its zero included. The empty plot's own
/// zero is for an axis without labels: here it was written a second time,
/// on the axis's. The sentence stays, the plot being as empty.
#[test]
fn an_empty_plot_out_of_live_writes_its_zero_once() {
    let ctx = crate::app::styled_context();
    let mut g = Graph::new();
    g.live = false;
    let mut texts = Vec::new();
    for _ in 0..3 {
        texts = frame_texts(&mut g, &ctx, NARROW);
    }
    let labelled = |text: &str| texts.iter().filter(|(t, _)| t == text).count();
    assert_eq!(labelled("0 s"), 1, "{texts:?}");
    assert!(
        texts.iter().any(|(t, _)| t.ends_with(" s") && t != "0 s"),
        "the axis labels its span: {texts:?}"
    );
    assert_eq!(empty_sentences(&texts).len(), 1, "{texts:?}");
}

/// The sentence stands on a patch of the plot's background, which is there
/// to keep the grid out of it. An empty plot still draws an alarm's limits,
/// and one through its middle, as a limit of 0 is, says where the alarm
/// trips: the patch used to be painted over the drawn plot and cut the
/// line. It is drawn before every line and stands under the limit, clear of
/// it, so both are whole.
#[test]
fn the_sentence_stands_clear_of_a_limit_line() {
    fn flatten<'a>(shape: &'a egui::Shape, out: &mut Vec<&'a egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    let ctx = crate::app::styled_context();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let markers = crate::markers::Markers::default();
    let below_zero = crate::graph::WatchedLimits {
        limits: dmm_lib::alarm::Limits::check(Some(0.0), None).expect("a low limit"),
        unit: dmm_lib::alarm::LimitUnit::Base,
    };
    let mut g = Graph::new();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, NARROW);
    let mut out = None;
    for _ in 0..2 {
        out = Some(headless_frame(&ctx, Some(screen), None, vec![], |ui| {
            let l10n = crate::l10n::english();
            g.show(ui, &l10n, &tc, &markers, Some(below_zero), NO_DIGITS)
        }));
    }
    let out = out.expect("a frame");
    let mut shapes = Vec::new();
    for clipped in &out.shapes {
        flatten(&clipped.shape, &mut shapes);
    }
    let plot = g.plot_rect.expect("drawn");

    let patches: Vec<(usize, egui::Rect)> = shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s {
            egui::Shape::Rect(r)
                if r.fill == tc.plot_background()
                    && r.rect != plot
                    && (r.rect.center().x - plot.center().x).abs() < 1.0 =>
            {
                Some((i, r.rect))
            }
            _ => None,
        })
        .collect();
    let [(patch_at, patch)] = patches[..] else {
        panic!("one patch: {patches:?}");
    };
    assert!(plot.contains_rect(patch), "{patch:?} in {plot:?}");
    // The limit's dots, along the plot's middle.
    let dots: Vec<(usize, egui::Pos2)> = shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s {
            egui::Shape::Circle(dot) if dot.fill == tc.status_error() => Some((i, dot.center)),
            _ => None,
        })
        .collect();
    assert!(
        dots.iter().any(|(_, at)| patch.x_range().contains(at.x)),
        "no limit line over the patch: {dots:?}"
    );
    for (i, at) in dots {
        assert!((at.y - plot.center().y).abs() < 1.0, "a limit at 0: {at:?}");
        assert!(i > patch_at, "the patch is painted after the line");
        assert!(
            patch.top() > at.y + 2.0,
            "the patch {patch:?} stands on the dot at {at:?}"
        );
    }
}

/// A plot narrower than the sentence goes without it rather than cut it or
/// let it run over the axes; the origin label stays.
#[test]
fn a_plot_too_small_for_the_sentence_goes_without() {
    let ctx = crate::app::styled_context();
    let mut g = Graph::new();
    let small = egui::vec2(220.0, 300.0);
    frame_texts(&mut g, &ctx, small);
    let texts = frame_texts(&mut g, &ctx, small);
    assert!(g.plot_rect.expect("drawn").width() < 200.0);
    assert!(empty_sentences(&texts).is_empty(), "{texts:?}");
    assert!(texts.iter().any(|(t, _)| t == "0 s"), "{texts:?}");
}

/// A time label is centred on its tick, and one whose tick is too near the
/// plot's right edge to hold its half is left out: past the edge the window
/// cuts it, or it runs under the right axes' labels. As the live view
/// scrolls a label comes in once, on its tick, and stays.
#[test]
fn no_time_label_passes_the_plots_right_edge() {
    let ctx = crate::app::styled_context();
    let (mut g, t0) = graph_over(10);
    for _ in 0..3 {
        frame_tick_labels(&mut g, &ctx, NARROW);
    }
    let plot = g.plot_rect.expect("drawn");

    // Scrolling on, the newest tick just inside the right edge at first:
    // each label, once drawn, is drawn in every frame until its tick
    // leaves on the left, and always centred on that tick.
    let mut seen: Vec<(String, Vec<usize>)> = Vec::new();
    for (frame, tenths) in (101..=160_u64).enumerate() {
        let at = t0 + Duration::from_millis(tenths * 100);
        g.push(1.0, at, "DC V", "V", None);
        let labels = frame_tick_labels(&mut g, &ctx, NARROW);
        let (view_min, view_max) = g.view_bounds();
        for (text, rect) in labels.time {
            assert!(rect.right() <= plot.right() + 0.5, "{text} at {rect:?}");
            let secs: f64 = text.trim_end_matches(" s").parse().expect("seconds");
            let tick = plot.left() as f64
                + (secs - view_min) / (view_max - view_min) * plot.width() as f64;
            assert!((f64::from(rect.center().x) - tick).abs() < 0.5, "{text}");
            match seen.iter_mut().find(|(t, _)| *t == text) {
                Some((_, frames)) => frames.push(frame),
                None => seen.push((text, vec![frame])),
            }
        }
    }
    assert!(seen.iter().any(|(t, _)| t == "15 s"), "{seen:?}");
    for (text, frames) in &seen {
        let run = frames[frames.len() - 1] - frames[0] + 1;
        assert_eq!(run, frames.len(), "{text} came and went: {frames:?}");
    }
}
