//! The plot key and the **Show:** toggles.

use super::*;
use crate::graph::key::KeyStyle;
use crate::graph::toolbar::{overlay_chip_label, series_chip_label};

/// The two chip rows show the same sub-value names, so their accessible
/// names have to say which group they belong to — otherwise a screen
/// reader announces the **Plot:** T2 chip and the **Show:** T2 chip
/// identically and the user cannot tell what a press will do.
#[test]
fn chip_labels_name_their_group() {
    assert_eq!(
        series_chip_label(&crate::l10n::english(), None),
        "Plot main reading"
    );
    assert_eq!(
        series_chip_label(&crate::l10n::english(), Some("T2")),
        "Plot T2"
    );
    assert_eq!(
        overlay_chip_label(&crate::l10n::english(), "T2"),
        "Show T2 trace"
    );
    assert_ne!(
        series_chip_label(&crate::l10n::english(), Some("T2")),
        overlay_chip_label(&crate::l10n::english(), "T2")
    );
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
        .key_entries(&crate::l10n::english(), &drawn, true)
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

/// A chip click is left for the settings to remember, both ways; hiding it
/// in code (Raw under a scale) is not.
#[test]
fn a_chip_click_is_left_for_the_settings() {
    let mut g = graph_with_two_overlays();
    g.hide_overlay("T2");
    assert_eq!(g.take_trace_choice(), None);

    g.click_overlay_chip("T3".to_string());
    assert_eq!(g.take_trace_choice(), Some(("T3".to_string(), true)));
    assert_eq!(g.take_trace_choice(), None, "taken once");
    g.click_overlay_chip("T3".to_string());
    assert_eq!(g.take_trace_choice(), Some(("T3".to_string(), false)));
}

/// The main reading's chip, shown while a sub-value is plotted, flips its
/// trace but isn't remembered: it would hide the main reading on the next
/// meter to plot a sub-value.
#[test]
fn the_main_reading_s_chip_is_not_remembered() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        push_aux(
            &mut g,
            50.0,
            t0 + Duration::from_secs(i),
            Some("T2"),
            &[(MAIN_SERIES, Some(20.0))],
        );
    }
    assert_eq!(drawn_overlay_labels(&g), vec![MAIN_SERIES]);

    g.click_overlay_chip(MAIN_SERIES.to_string());
    assert!(drawn_overlay_labels(&g).is_empty());
    assert_eq!(g.take_trace_choice(), None);
}

/// A remembered hidden label names a sub-value, never the main reading: a
/// UT181A that hid its T2 sub-value still draws T2 where T2 is the main
/// reading, beside a plotted T1.
#[test]
fn a_remembered_label_never_hides_the_main_reading() {
    let mut g = Graph::new();
    g.hide_overlays(&["T2".to_string()]);
    let t0 = Instant::now();
    for i in 0..3 {
        g.push_sample(PlotSample {
            value: Some(21.0),
            timestamp: t0 + Duration::from_secs(i),
            mode: "Temp",
            unit: "\u{00B0}C",
            display_raw: None,
            series: Some("T1"),
            main_label: Some(dmm_lib::measurement::MainLabel::T2),
            levels: false,
            overlays: &[("T2", "\u{00B0}C", Some(23.0))],
        });
    }
    assert_eq!(drawn_overlay_labels(&g), vec!["T2"]);
}

/// Hiding the main reading's trace lasts for the mode it was hidden in.
#[test]
fn a_hidden_main_trace_comes_back_in_the_next_mode() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    push_aux(&mut g, 50.0, t0, Some("T2"), &[(MAIN_SERIES, Some(20.0))]);
    g.click_overlay_chip(MAIN_SERIES.to_string());
    assert!(drawn_overlay_labels(&g).is_empty());

    push_vac_hz(
        &mut g,
        t0 + Duration::from_secs(1),
        Some("Frequency"),
        230.0,
        (50.0, "Hz"),
    );
    assert!(drawn_overlay_labels(&g).contains(&MAIN_SERIES.to_string()));
}

/// Traces the settings remember hidden start hidden.
#[test]
fn remembered_hidden_traces_start_hidden() {
    let mut g = Graph::new();
    g.hide_overlays(&["T3".to_string()]);
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
    assert_eq!(drawn_overlay_labels(&g), vec!["T2"]);
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
