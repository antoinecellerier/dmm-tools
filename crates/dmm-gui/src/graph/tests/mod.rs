//! Graph tests: the helpers several sections share; one file per section.

use super::view::pad_range;
use super::*;
use crate::app::test_support::key_tap;
use crate::settings::{ColorPreset, PaletteOverrides};
use std::time::Duration;

mod axes;
mod drawing;
mod figures;
mod gestures;
mod hover;
mod ingest;
mod key;
mod markers;
mod menu;
mod minimap;
mod overlays;
mod view;

/// A reading's digits, for a graph drawn without the app: none, so the
/// menu names a reading by its value on the trace.
const NO_DIGITS: ReadingDigits = &|_, _| None;

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
        main_label: Some(dmm_lib::measurement::MainLabel::Dc),
        levels: false,
        overlays: if dc.is_some() { &overlays } else { &[] },
    });
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

/// A V reading beside a frequency wandering between 49.9 and 50.1 Hz.
fn graph_with_hz() -> Graph {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for (i, hz) in [49.9, 50.1, 50.0].into_iter().enumerate() {
        push_vac_hz(
            &mut g,
            t0 + Duration::from_secs(i as u64),
            None,
            230.0,
            (hz, "Hz"),
        );
    }
    g
}

/// Names in the key, in the order they are painted.
fn key_names(g: &Graph) -> Vec<String> {
    let drawn = g.visible_overlay_traces(f64::NEG_INFINITY, f64::INFINITY);
    g.key_entries(
        &crate::l10n::english(),
        &drawn,
        !g.all_segments().is_empty(),
    )
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

/// Big enough that the plot gets a real rect. Pointer positions below are in
/// these coordinates.
fn gesture_screen() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))
}

/// Inside the plot body.
fn over_plot() -> egui::Pos2 {
    egui::pos2(400.0, 300.0)
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

/// One headless frame of the whole graph — toolbar included — with `events`
/// delivered to it.
fn graph_frame(g: &mut Graph, ctx: &egui::Context, events: Vec<egui::Event>) {
    graph_frame_in(&crate::l10n::english(), g, ctx, events);
}

/// [`graph_frame`] in `l10n`'s language.
fn graph_frame_in(l10n: &Localizer, g: &mut Graph, ctx: &egui::Context, events: Vec<egui::Event>) {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(gesture_screen()),
            events,
            ..Default::default()
        },
        |ui| {
            g.show(
                ui,
                l10n,
                &tc,
                &crate::markers::Markers::default(),
                None,
                NO_DIGITS,
            )
        },
    );
    // Nothing here paints, and TexturesDelta panics if it is dropped unapplied.
    output.textures_delta.clear();
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
