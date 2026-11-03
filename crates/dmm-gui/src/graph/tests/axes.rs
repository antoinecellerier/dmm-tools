//! The right axes: one per sub-value unit, their room and their pins.

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
