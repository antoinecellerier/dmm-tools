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
    frame_text_colors(g, ctx, screen)
        .into_iter()
        .map(|(text, rect, _)| (text, rect))
        .collect()
}

/// [`frame_texts`], each with the colour it was drawn in.
fn frame_text_colors(
    g: &mut Graph,
    ctx: &egui::Context,
    screen: egui::Vec2,
) -> Vec<(String, egui::Rect, egui::Color32)> {
    texts_in(&frame_output(g, ctx, screen))
}

/// Every text a frame drew, with where and in which colour.
fn texts_in(out: &egui::FullOutput) -> Vec<(String, egui::Rect, egui::Color32)> {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect, egui::Color32)>) {
        match shape {
            egui::Shape::Text(t) => found.push((
                t.galley.text().to_string(),
                egui::Rect::from_min_size(t.pos, t.galley.size()),
                t.fallback_color,
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for clipped in &out.shapes {
        walk(&clipped.shape, &mut texts);
    }
    texts
}

/// One frame of the graph on a `screen`-sized window.
fn frame_output(g: &mut Graph, ctx: &egui::Context, screen: egui::Vec2) -> egui::FullOutput {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let markers = crate::markers::Markers::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen);
    headless_frame(ctx, Some(screen), None, vec![], |ui| {
        g.show(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
    })
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

/// The sentence keeps clear of an alarm's limits and of nothing else, there
/// being no other line on a plot nothing was read on: a reference line is
/// read against what is plotted, so the Ref field's entries draw none
/// there, typed bare or with a unit, put back by a saved view or added from
/// the plot's menu. The sentence stays in the plot's middle.
#[test]
fn an_empty_plot_draws_no_reference_line() {
    let ctx = crate::app::styled_context();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    type Fill = fn(&mut Graph);
    let entries: [(&str, Fill); 4] = [
        ("typed bare", |g| {
            *g.ref_lines.text_mut() = "0".to_string();
            g.ref_lines.parse(None);
        }),
        ("typed with a unit", |g| {
            *g.ref_lines.text_mut() = "0 V".to_string();
            g.ref_lines.parse(None);
        }),
        ("a saved view's", |g| g.ref_lines.set(&[0.0], "")),
        ("the menu's", |g| g.add_ref_line(0.0)),
    ];
    for (name, fill) in entries {
        let mut g = Graph::new();
        g.show_ref_line = true;
        frame_output(&mut g, &ctx, NARROW);
        fill(&mut g);
        for _ in 0..3 {
            let out = frame_output(&mut g, &ctx, NARROW);
            let plot = g.plot_rect.expect("drawn");
            let dashes = super::drawing::shapes_in(&out.shapes, tc.graph_ref(), plot.top());
            assert!(dashes.is_empty(), "{name}: {} dashes", dashes.len());
            let texts: Vec<_> = texts_in(&out).into_iter().map(|(t, r, _)| (t, r)).collect();
            let [(_, sentence)] = &empty_sentences(&texts)[..] else {
                panic!("{name}: {texts:?}");
            };
            assert!((sentence.center() - plot.center()).length() < 1.0, "{name}");
        }
    }
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

/// Sixty seconds of a slow wave between `lo` and `hi`, as `sample` pushes
/// each reading.
fn wave(lo: f64, hi: f64, sample: impl Fn(&mut Graph, Instant, f64)) -> Graph {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..=600_u64 {
        let phase = (i as f64 / 600.0 * std::f64::consts::TAU * 3.0).sin();
        let v = lo + (hi - lo) * (phase + 1.0) / 2.0;
        sample(&mut g, t0 + Duration::from_millis(i * 100), v);
    }
    g
}

/// The value axis's labels on a plot of `g` in a window `height` tall, by
/// where and in which colour they are drawn, and the plot's body.
fn value_labels(
    g: &mut Graph,
    ctx: &egui::Context,
    height: f32,
) -> (Vec<(String, egui::Rect, egui::Color32)>, egui::Rect) {
    let screen = egui::vec2(396.0, height);
    for _ in 0..2 {
        frame_text_colors(g, ctx, screen);
    }
    let texts = frame_text_colors(g, ctx, screen);
    let plot = g.plot_rect.expect("the plot is drawn");
    let mut left: Vec<_> = texts
        .into_iter()
        .filter(|(_, r, _)| {
            r.right() <= plot.left()
                && r.intersects(plot.expand2(egui::vec2(f32::INFINITY, 0.0)))
                && (r.top() - plot.bottom()).abs() >= 1.0
        })
        .collect();
    left.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
    (left, plot)
}

/// The least alpha egui_plot draws a readable label in: it fades one by the
/// root of how far its lines' spacing is from 20 points to 30.
fn readable_alpha() -> u8 {
    let strength = (crate::graph::axes::READABLE_LABEL_SPACING - 20.0) / 10.0;
    (255.0 * strength.sqrt()).floor() as u8
}

/// A plot with readings keeps a scale on its value axis however short it
/// is: at least one label on a gridline inside it, readable, a round
/// value, and clear of the next. egui_plot's decade grid had none for
/// a short plot of 2 V to 8 V, the 1 V lines too close to label and no 10 V
/// line in view; nor had the grid shared with right axes, its lines closer
/// than egui_plot labels.
#[test]
fn a_short_plot_keeps_a_label_on_its_value_axis() {
    let ctx = crate::app::styled_context();
    let plain = |g: &mut Graph, t, v| g.push(v, t, "DC V", "V", None);
    let with_hz = |g: &mut Graph, t, v| push_vac_hz(g, t, None, v, (50.0 + v % 0.1, "Hz"));
    let level = |g: &mut Graph, t, v: f64| {
        g.push_sample(PlotSample {
            value: Some(v.round()),
            timestamp: t,
            mode: "NCV",
            unit: "",
            display_raw: None,
            series: None,
            main_label: None,
            levels: true,
            overlays: &[],
        });
    };
    let ranges = [
        (2.0, 8.0),
        (0.1, 0.4),
        (118.2, 118.4),
        (0.0, 1000.0),
        (-3.0, 5.0),
        (0.0, 9.9),
        (0.0, 4.0),
    ];
    let mut failures = Vec::new();
    for (lo, hi) in ranges {
        for (name, mut g) in [
            ("plain", wave(lo, hi, plain)),
            ("beside Hz", wave(lo, hi, with_hz)),
            ("levels", wave(lo, hi, level)),
        ] {
            let mut least = f32::INFINITY;
            for height in (100..=700).step_by(7) {
                let (labels, plot) = value_labels(&mut g, &ctx, height as f32);
                least = least.min(plot.height());
                let case = format!("{name} {lo}..{hi}, plot {} pt", plot.height());
                let inside = labels.iter().filter(|(_, r, color)| {
                    plot.y_range().contains(r.center().y) && color.a() >= readable_alpha()
                });
                if inside.count() == 0 {
                    failures.push(format!("{case}: no label"));
                }
                for pair in labels.windows(2) {
                    if pair[0].1.bottom() > pair[1].1.top() {
                        failures.push(format!("{case}: {} over {}", pair[0].0, pair[1].0));
                    }
                }
                // Round values: whole multiples of a 1, 2 or 5 × 10ⁿ step.
                let values: Vec<f64> = labels
                    .iter()
                    .map(|(text, _, _)| {
                        let number = text.trim().split(' ').next().expect("a number");
                        number.parse().expect("a number")
                    })
                    .collect();
                for pair in values.windows(2) {
                    let step = pair[0] - pair[1];
                    let lead = step / 10f64.powf(step.log10().floor());
                    let round = [1.0, 2.0, 5.0, 10.0]
                        .iter()
                        .any(|m| (lead - m).abs() < 1e-6);
                    let on_it = ((pair[0] / step).round() - pair[0] / step).abs() < 1e-6;
                    let whole = name != "levels" || step.fract() == 0.0;
                    if !(round && on_it && whole) {
                        failures.push(format!("{case}: {} then {}", pair[0], pair[1]));
                    }
                }
            }
            // The graph's least: two thirds of what it shares with the minimap.
            assert!(least < 75.0, "the least plot was {least} pt tall");
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A plot tall enough for egui_plot's decades to be read keeps them, faded
/// or not: 0 to 110 mA on some 330 points had a label on every 10 mA line,
/// 25 points apart, and three labels in their place say less. The round
/// steps take over only where those labels would be too faint to read.
#[test]
fn a_plot_keeps_the_decades_it_can_read() {
    let ctx = crate::app::styled_context();
    let readable = crate::graph::axes::READABLE_LABEL_SPACING as f32;
    let mut g = wave(0.0, 110.0, |g, t, v| g.push(v, t, "DC mA", "mA", None));
    // The spacings of the 10 mA lines seen on either grid.
    let (mut kept, mut replaced) = (Vec::new(), Vec::new());
    for height in (300..=560).step_by(2) {
        // As a plot first drawn at each height: one resized keeps the grid
        // it has near the line (`a_resized_plot_keeps_its_grid_near_the_line`).
        g.on_round_steps = false;
        let (labels, plot) = value_labels(&mut g, &ctx, height as f32);
        let case = format!("plot {} pt", plot.height());
        let at: Vec<(f32, f32)> = labels
            .iter()
            .map(|(text, r, _)| {
                let number = text.trim().split(' ').next().expect("a number");
                (number.parse().expect("a number"), r.center().y)
            })
            .collect();
        let (top, bottom) = (at.first().expect(&case), at.last().expect(&case));
        assert!(top.0 > bottom.0, "{case}: {labels:?}");
        let spacing = 10.0 * (bottom.1 - top.1) / (top.0 - bottom.0);
        let steps: Vec<f32> = at.windows(2).map(|pair| pair[0].0 - pair[1].0).collect();
        if spacing >= readable + 0.1 {
            // egui_plot's grid: every 10 mA line in view, each readable.
            assert!(steps.iter().all(|&s| s == 10.0), "{case}: {steps:?}");
            let lines = (plot.height() / spacing).floor() as usize;
            assert!(labels.len() >= lines, "{case}: {labels:?}");
            for (text, _, color) in &labels {
                assert!(color.a() >= readable_alpha(), "{case}: {text} {color:?}");
            }
            kept.push(spacing);
        } else if spacing <= readable - 0.1 {
            // The round steps, each line labelled at full strength.
            assert!(
                steps.iter().all(|&s| s == 20.0 || s == 50.0),
                "{case}: {steps:?}"
            );
            assert!(labels.iter().all(|(_, _, c)| c.is_opaque()), "{case}");
            replaced.push(spacing);
        }
    }
    // Both sides of the line were seen, close to it: where egui_plot fades
    // its labels, and where it would still have drawn them.
    assert!(kept.iter().any(|&s| s < readable + 1.0), "{kept:?}");
    assert!(replaced.iter().any(|&s| s > readable - 1.0), "{replaced:?}");
}

/// A live trace's range follows every extreme that enters or leaves the
/// view, and one wandering either side of the line between the two grids
/// changed its labels at each crossing: 0 to 1.6 V is 1.92 V with its
/// margins, short of the 2 V two labelled 1 V lines take, and a reading of
/// 1.7 V in view makes it 2.04 V. The plot keeps the round steps it has
/// until the decades would serve it with a margin.
#[test]
fn a_range_wandering_across_the_line_keeps_its_grid() {
    let ctx = crate::app::styled_context();
    let screen = egui::vec2(396.0, 330.0);
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    let t0 = Instant::now();
    let (mut served, mut drawn) = (Vec::new(), Vec::new());
    // The 1.7 V reading is in view ten seconds in every fifteen.
    for i in 0..520_u64 {
        let v = match i {
            _ if i % 150 == 0 => 1.7,
            _ if i % 2 == 0 => 0.0,
            _ => 1.6,
        };
        g.push(v, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
        frame_output(&mut g, &ctx, screen);
        let (from, to) = g.view_bounds();
        let (lo, hi) = g.y_range_for_view(from, to, true).expect("readings");
        let height = g.plot_rect.expect("drawn").height();
        served.push(crate::graph::axes::decades_label_two(hi - lo, height, 0.0));
        drawn.push(!g.on_round_steps);
    }
    // Once the first 1.7 V has left: the line is crossed five times more.
    let changes = |grids: &[bool]| grids[110..].windows(2).filter(|p| p[0] != p[1]).count();
    assert_eq!(changes(&served), 5, "the range crosses the line");
    assert_eq!(changes(&drawn), 0, "the grid changed with it");
    // With 1.7 V in view at the end, where the decades would serve.
    assert_eq!(served.last(), Some(&true));
    assert!(g.on_round_steps);
}

/// A window dragged shorter gives up its decades where their labels stop
/// being readable, and one dragged taller takes them back a fifth later:
/// between the two a plot keeps the grid it has, and outside them the grid
/// is the one a plot first drawn at that height takes.
#[test]
fn a_resized_plot_keeps_its_grid_near_the_line() {
    let ctx = crate::app::styled_context();
    let readable = crate::graph::axes::READABLE_LABEL_SPACING as f32;
    let mut g = wave(0.0, 110.0, |g, t, v| g.push(v, t, "DC mA", "mA", None));
    // Whether the plot is on its decades in a window `height` tall, and
    // how far apart its 10 mA lines are: 132 mA with the margins.
    let mut at = |height: u32| {
        for _ in 0..3 {
            frame_output(&mut g, &ctx, egui::vec2(396.0, height as f32));
        }
        let plot = g.plot_rect.expect("drawn").height();
        (!g.on_round_steps, 10.0 * plot / 132.0)
    };
    let heights: Vec<u32> = (300..=700).step_by(2).collect();
    assert!(!at(heights[0]).0, "round steps on the shortest");
    let taller: Vec<(bool, f32)> = heights.iter().map(|&h| at(h)).collect();
    let shorter: Vec<(bool, f32)> = heights.iter().rev().map(|&h| at(h)).collect();
    // One change each way, and the spacing it came at.
    let turn = |seen: &[(bool, f32)]| {
        let turns: Vec<f32> = seen
            .windows(2)
            .filter(|pair| pair[0].0 != pair[1].0)
            .map(|pair| pair[0].1.max(pair[1].1))
            .collect();
        let [at] = turns[..] else {
            panic!("one change of grid: {turns:?}");
        };
        at
    };
    let (took, gave_up) = (turn(&taller), turn(&shorter));
    assert!(
        (readable * 1.2..readable * 1.2 + 0.4).contains(&took),
        "taken with the lines {took} points apart"
    );
    assert!(
        (readable..readable + 0.4).contains(&gave_up),
        "kept down to {gave_up} points apart"
    );
}

/// The room a round step leaves each label is the label's own height in
/// the style it is drawn in: egui_plot leaves out a label taller than its
/// lines are apart, and in 20-point text, 23 points high, the lines of a
/// short plot were closer than that and its axis had none.
#[test]
fn a_short_plot_keeps_a_label_in_larger_text() {
    let ctx = crate::app::styled_context();
    ctx.all_styles_mut(|style| {
        let body = egui::FontId::proportional(20.0);
        style.text_styles.insert(egui::TextStyle::Body, body);
    });
    let plain = |g: &mut Graph, t, v| g.push(v, t, "DC V", "V", None);
    let mut failures = Vec::new();
    let mut shortest = f32::INFINITY;
    for (lo, hi) in [(2.0, 8.0), (0.0, 9.9), (118.2, 118.4), (-3.0, 5.0)] {
        let mut g = wave(lo, hi, plain);
        for height in (150..=500).step_by(7) {
            let (labels, plot) = value_labels(&mut g, &ctx, height as f32);
            // A plot with room for one line at least, whatever its range:
            // two and a half labels with their air.
            if plot.height() < 63.0 {
                continue;
            }
            shortest = shortest.min(plot.height());
            let case = format!("{lo}..{hi}, plot {} pt", plot.height());
            let inside = labels.iter().filter(|(_, r, color)| {
                plot.y_range().contains(r.center().y) && color.a() >= readable_alpha()
            });
            if inside.count() == 0 {
                failures.push(format!("{case}: no label"));
            }
            for pair in labels.windows(2) {
                if pair[0].1.bottom() > pair[1].1.top() {
                    failures.push(format!("{case}: {} over {}", pair[0].0, pair[1].0));
                }
            }
        }
    }
    assert!(shortest < 80.0, "the shortest plot was {shortest} pt");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A settled frame's value grid: its lines across the plot and the labels
/// either side of it.
struct ValueGrid {
    /// Each gridline's value, height on screen and colour.
    lines: Vec<(f64, f32, egui::Color32)>,
    /// The left axis's labels, and the right axes' stacked ones.
    left: Vec<(String, egui::Rect)>,
    right: Vec<(String, egui::Rect)>,
    /// Points a unit of the plotted value takes.
    points_per_unit: f64,
}

impl ValueGrid {
    /// The colour of the line at `v`.
    fn color_at(&self, v: f64) -> egui::Color32 {
        let found = self.lines.iter().find(|(at, _, _)| (at - v).abs() < 1e-3);
        found.unwrap_or_else(|| panic!("no line at {v}")).2
    }
}

/// The value grid `g` settles on in a `screen`-sized window.
fn value_grid(g: &mut Graph, ctx: &egui::Context, screen: egui::Vec2) -> ValueGrid {
    fn walk(shape: &egui::Shape, plot: egui::Rect, found: &mut Vec<(f32, egui::Color32)>) {
        match shape {
            egui::Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y
                    && plot.y_range().contains(points[0].y)
                    && (points[0].x - points[1].x).abs() >= plot.width() - 1.0 =>
            {
                found.push((points[0].y, stroke.color));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, plot, found)),
            _ => {}
        }
    }
    for _ in 0..2 {
        frame_output(g, ctx, screen);
    }
    let out = frame_output(g, ctx, screen);
    let plot = g.plot_rect.expect("the plot is drawn");
    let transform = g.plot_transform.expect("the plot is drawn");
    let mut drawn = Vec::new();
    for clipped in &out.shapes {
        walk(&clipped.shape, plot, &mut drawn);
    }
    let lines = drawn
        .into_iter()
        .map(|(y, color)| {
            let at = transform.value_from_position(egui::pos2(plot.center().x, y));
            (at.y, y, color)
        })
        .collect();
    let beside: Vec<_> = frame_texts(g, ctx, screen)
        .into_iter()
        .filter(|(_, r)| plot.y_range().contains(r.center().y))
        .collect();
    let side = |keep: &dyn Fn(&egui::Rect) -> bool| -> Vec<(String, egui::Rect)> {
        let kept = beside.iter().filter(|(_, r)| keep(r));
        kept.cloned().collect()
    };
    ValueGrid {
        lines,
        left: side(&|r| r.right() <= plot.left()),
        right: side(&|r| r.left() >= plot.right()),
        points_per_unit: transform.dpos_dvalue()[1].abs(),
    }
}

/// The number a tick label writes.
fn label_value(text: &str) -> f64 {
    let number = text.trim().split(' ').next().expect("a number");
    number.parse().expect("a number")
}

/// Two colours a step apart at most in each channel: the test reads the
/// spacing back off heights on screen, and a rounding off egui_plot's own
/// can move a faded line's alpha by one.
fn same_color(a: egui::Color32, b: egui::Color32) -> bool {
    a.to_array()
        .iter()
        .zip(b.to_array())
        .all(|(&x, y)| x.abs_diff(y) <= 1)
}

/// The colour egui_plot draws a gridline in whose level's lines are
/// `spacing` points apart: the text colour, faded by the root of how far
/// the spacing is from the least it draws to 300 points.
fn gridline_color(ctx: &egui::Context, spacing: f64) -> egui::Color32 {
    let least = f64::from(crate::graph::time::MIN_GRID_SPACING);
    let strength = ((spacing - least) / (300.0 - least)).clamp(0.0, 1.0) as f32;
    let text = ctx.global_style().visuals.text_color();
    text.gamma_multiply(strength.sqrt())
}

/// The grid of round steps has a fainter line between two steps, as
/// egui_plot's decades have their tenths: at the minor step, drawn only
/// once those lines are far enough apart, and never labelled. Each line is
/// as strong as the coarsest round level it sits on, so 1 V stands out on
/// a grid of 0.5 V as it does on the decades. The labels stay on every
/// whole step and nowhere else, the right axes' stacks centred on them.
#[test]
fn a_round_grid_has_unlabelled_lines_between_its_steps() {
    use crate::graph::axes::minor_step;
    let ctx = crate::app::styled_context();
    let plain = |g: &mut Graph, t, v| g.push(v, t, "DC V", "V", None);
    let with_hz = |g: &mut Graph, t, v| push_vac_hz(g, t, None, v, (50.0 + v % 0.1, "Hz"));
    let level = |g: &mut Graph, t, v: f64| {
        g.push_sample(PlotSample {
            value: Some(v.round()),
            timestamp: t,
            mode: "NCV",
            unit: "",
            display_raw: None,
            series: None,
            main_label: None,
            levels: true,
            overlays: &[],
        });
    };
    let least = f64::from(crate::graph::time::MIN_GRID_SPACING);
    // A line's value is read back off its height on screen.
    let on = |v: f64, step: f64| ((v / step).round() - v / step).abs() < 1e-3;
    // (name, graph, window, the step its labels are on, right axes)
    let cases = [
        (
            "2 to 8 V, short",
            wave(2.0, 8.0, plain),
            (396.0, 230.0),
            2.0,
            0,
        ),
        ("0 to 1.5 V", wave(0.0, 1.5, plain), (454.0, 330.0), 0.5, 0),
        (
            "0 to 1.5 V, short",
            wave(0.0, 1.5, plain),
            (454.0, 200.0),
            0.5,
            0,
        ),
        (
            "beside Hz",
            wave(2.0, 8.0, with_hz),
            (1000.0, 700.0),
            1.0,
            2,
        ),
        (
            "beside Hz, short",
            wave(2.0, 8.0, with_hz),
            (1000.0, 270.0),
            2.0,
            2,
        ),
        ("levels", wave(0.0, 4.0, level), (396.0, 200.0), 2.0, 0),
        (
            "levels, whole",
            wave(0.0, 1.0, level),
            (396.0, 200.0),
            1.0,
            0,
        ),
        (
            "levels, coarse",
            wave(0.0, 40.0, level),
            (396.0, 330.0),
            10.0,
            0,
        ),
    ];
    // Both sides of the least spacing were seen, and both kinds of step.
    let (mut with_minor, mut without) = (0, 0);
    for (name, mut g, (width, height), step, right_axes) in cases {
        let grid = value_grid(&mut g, &ctx, egui::vec2(width, height));
        let finest = if name.starts_with("levels") { 1.0 } else { 0.0 };
        let case = format!("{name}: lines {:?}", grid.lines);
        type Line = (f64, f32, egui::Color32);
        let (steps, between): (Vec<Line>, Vec<Line>) =
            grid.lines.iter().partition(|l| on(l.0, step));
        assert!(steps.len() >= 2, "{case}");

        // The labels: one on every whole step, centred on its line, and
        // none anywhere else.
        assert_eq!(grid.left.len(), steps.len(), "{case}: {:?}", grid.left);
        for (text, rect) in &grid.left {
            let v = label_value(text);
            assert!(on(v, step), "{case}: a label at {v}");
            let line = steps.iter().find(|l| (l.0 - v).abs() < 1e-3);
            let y = line.unwrap_or_else(|| panic!("{case}: no line at {v}")).1;
            assert!(
                (rect.center().y - y).abs() <= 1.0,
                "{case}: {text} off its line"
            );
        }
        // The right axes' stacks: a row per axis, centred on a whole step.
        assert_eq!(grid.right.is_empty(), right_axes == 0, "{case}");
        for &(_, y, _) in &steps {
            let stack: Vec<f32> = grid
                .right
                .iter()
                .filter(|(_, r)| (r.center().y - y).abs() < 17.0)
                .map(|(_, r)| r.center().y)
                .collect();
            if !stack.is_empty() {
                assert_eq!(stack.len(), right_axes, "{case}: {:?}", grid.right);
                let mid = stack.iter().sum::<f32>() / stack.len() as f32;
                assert!((mid - y).abs() <= 1.0, "{case}: a stack off its line");
            }
        }
        let stacked = grid.right.len() / right_axes.max(1);
        assert!(right_axes == 0 || stacked >= 2, "{case}: {:?}", grid.right);

        // The lines between: at the minor step, every one of them once
        // they are far enough apart to draw, fainter than a step's.
        let minor = minor_step(step, finest);
        let drawn = minor.filter(|m| m * grid.points_per_unit > least);
        match drawn {
            Some(minor) => {
                with_minor += 1;
                let per_step = (step / minor).round() as usize - 1;
                assert!(between.len() >= per_step * (steps.len() - 1), "{case}");
                for &(v, _, color) in &between {
                    assert!(on(v, minor), "{case}: a line at {v}");
                    let expected = gridline_color(&ctx, minor * grid.points_per_unit);
                    assert!(
                        same_color(color, expected),
                        "{case}: the line at {v}: {color:?}, not {expected:?}"
                    );
                    assert!(steps.iter().all(|l| l.2.a() > color.a()), "{case}: {v}");
                }
            }
            None => {
                without += 1;
                assert!(between.is_empty(), "{case}");
            }
        }
        // Each step's line is as strong as its coarsest round level.
        for &(v, _, color) in &steps {
            let decade = 10f64.powf(step.log10().ceil());
            let level = [decade * 100.0, decade * 10.0, decade, step]
                .into_iter()
                .find(|&d| on(v, d))
                .expect("a step");
            let expected = gridline_color(&ctx, level * grid.points_per_unit);
            assert!(
                same_color(color, expected),
                "{case}: the line at {v}, level {level}: {color:?}, not {expected:?}"
            );
        }
        if name == "0 to 1.5 V" {
            // 1 V is a 1 V line, twice as far from the next as 0.5 V is.
            let (half, one) = (grid.color_at(0.5), grid.color_at(1.0));
            assert!(one.a() > half.a(), "{case}");
            assert_eq!(half, grid.color_at(1.5), "{case}");
        }
    }
    assert!(
        with_minor >= 4 && without >= 2,
        "{with_minor} and {without}"
    );
}
