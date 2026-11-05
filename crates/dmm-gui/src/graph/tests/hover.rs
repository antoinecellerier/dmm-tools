//! The hover: the readings it lists and the heights it tags.

use super::*;
use crate::graph::hover::{
    HoverSeries, height_decimals, height_tags, hover_lines, reading_at, reading_near,
};

/// The hover lists every series at its time in the key's order, named
/// when there are several, and leaves out one in a break.
#[test]
fn the_hover_lists_every_series_at_its_time() {
    let volts = vec![vec![[0.0, 230.0], [2.0, 231.0]]];
    let hz = vec![vec![[0.0, 50.0], [0.9, 50.1]], vec![[1.8, 49.9]]];
    let series = [
        HoverSeries {
            name: "Main".into(),
            segments: &volts,
            unit: "V".into(),
            color: egui::Color32::RED,
            stepped: false,
            map: None,
        },
        HoverSeries {
            name: "Frequency".into(),
            segments: &hz,
            unit: "Hz".into(),
            color: egui::Color32::BLUE,
            stepped: false,
            map: None,
        },
    ];
    let rows = |series: &[HoverSeries], t, overload| -> Vec<(usize, String)> {
        hover_lines(&crate::l10n::english(), series, t, overload, 1)
            .into_iter()
            .map(|(i, text, _)| (i, text))
            .collect()
    };
    assert_eq!(
        rows(&series, 0.5, false),
        vec![
            (0, "Main: 230.0 V".to_string()),
            (1, "Frequency: 50.1 Hz".to_string())
        ]
    );
    assert_eq!(
        rows(&series, 1.2, false),
        vec![(0, "Main: 231.0 V".to_string())],
        "Frequency is in a break at 1.2 s"
    );
    assert_eq!(
        rows(&series, 0.2, true),
        vec![
            (0, "Main: overload".to_string()),
            (1, "Frequency: 50.0 Hz".to_string())
        ]
    );
    // One series: the number alone.
    assert_eq!(
        rows(&series[..1], 0.5, false),
        vec![(0, "230.0 V".to_string())]
    );
    assert_eq!(
        rows(&series[..1], 0.5, true),
        vec![(0, "overload".to_string())]
    );
    // Each row's reading is the one it shows, for the dot on its line.
    let points: Vec<_> = hover_lines(&crate::l10n::english(), &series, 0.5, false, 1)
        .into_iter()
        .map(|(_, _, p)| p)
        .collect();
    assert_eq!(points, vec![Some([0.0, 230.0]), Some([0.9, 50.1])]);
}

/// A level is held until the next reading: between two, the hover reads
/// the one before, as the steps are drawn, not the nearer one after.
#[test]
fn a_level_reads_the_one_held() {
    let levels = vec![vec![[0.0, 2.0], [1.0, 3.0]]];
    assert_eq!(reading_at(&levels, 0.8, true), Some([0.8, 2.0]));
    assert_eq!(reading_at(&levels, 1.0, true), Some([1.0, 3.0]));
    assert_eq!(reading_at(&levels, 0.8, false), Some([1.0, 3.0]));
    assert_eq!(reading_at(&levels, 1.5, true), None, "past the trace");
}

/// On a steep stretch the readings are far apart on screen: the pointer on
/// the line midway still snaps, and on a level's step, to the level held.
#[test]
fn the_hover_snaps_along_a_steep_line_and_a_step() {
    let line = vec![vec![[0.0, 0.0], [1.0, 1.0]]];
    let series = |segments, stepped| HoverSeries {
        name: "Main".into(),
        segments,
        unit: "V".into(),
        color: egui::Color32::RED,
        stepped,
        map: None,
    };
    // 10 points a second across, 100 points a unit up: steep.
    let to_screen = |t: f64, h: f64| egui::pos2((t * 10.0) as f32, (400.0 - h * 100.0) as f32);
    let near = |series: &[HoverSeries], x: f32, y: f32| {
        let window = (f64::from(x - 5.0) / 10.0, f64::from(x + 5.0) / 10.0);
        reading_near(
            series,
            window,
            (0.0, 10.0),
            egui::pos2(x, y),
            5.0,
            to_screen,
        )
    };
    let sloped = [series(&line, false)];
    assert_eq!(near(&sloped, 4.0, 360.0), Some([0.0, 0.0]));
    assert_eq!(near(&sloped, 6.0, 340.0), Some([1.0, 1.0]));
    assert_eq!(near(&sloped, 5.0, 250.0), None, "off the line");
    // Drawn as a step: held at 0 until 1 s, then a riser to 1.
    let steps = [series(&line, true)];
    assert_eq!(near(&steps, 8.0, 401.0), Some([0.0, 0.0]), "on the level");
    assert_eq!(near(&steps, 10.0, 330.0), Some([1.0, 1.0]), "on the riser");
    assert_eq!(near(&steps, 3.0, 330.0), None, "where a slope would be");
    // The nearer end out of view: the one in view, or none.
    let window = (0.1, 1.1);
    let in_view = |view| {
        reading_near(
            &sloped,
            window,
            view,
            egui::pos2(4.0, 360.0),
            5.0,
            to_screen,
        )
    };
    assert_eq!(in_view((0.2, 10.0)), Some([1.0, 1.0]));
    assert_eq!(in_view((0.2, 0.8)), None);
    // A line off at infinity is never near.
    let off = |t: f64, h: f64| egui::pos2((t * 10.0) as f32, (h * f64::INFINITY) as f32);
    assert_eq!(
        reading_near(
            &sloped,
            window,
            (0.0, 10.0),
            egui::pos2(4.0, 360.0),
            5.0,
            off
        ),
        None
    );
}

/// The crosshair snaps to a reading only when the pointer is on a curve,
/// any drawn one at its drawn height, anywhere along it, and stays with the
/// pointer elsewhere: on a scrolling graph a still pointer keeps a still line.
#[test]
fn the_hover_snaps_only_near_a_curve() {
    let volts = vec![vec![[0.0, 1.0], [1.0, 1.0], [2.0, 1.0]]];
    let hz = vec![vec![[0.5, 50.0], [1.5, 50.0]]];
    let map = crate::graph::axes::fit_secondary(0.0, 4.0, 1.0, (40.0, 60.0));
    let series = [
        HoverSeries {
            name: "Main".into(),
            segments: &volts,
            unit: "V".into(),
            color: egui::Color32::RED,
            stepped: false,
            map: None,
        },
        HoverSeries {
            name: "Frequency".into(),
            segments: &hz,
            unit: "Hz".into(),
            color: egui::Color32::BLUE,
            stepped: false,
            map: Some(map),
        },
    ];
    // 100 points a second across, 100 points a unit up.
    let to_screen = |t: f64, h: f64| egui::pos2((t * 100.0) as f32, (400.0 - h * 100.0) as f32);
    let near = |x: f32, y: f32| {
        let window = (f64::from(x - 5.0) / 100.0, f64::from(x + 5.0) / 100.0);
        reading_near(
            &series,
            window,
            (0.0, 10.0),
            egui::pos2(x, y),
            5.0,
            to_screen,
        )
    };
    assert_eq!(
        near(102.0, 300.0),
        Some([1.0, 1.0]),
        "on Main's reading at 1 s"
    );
    assert_eq!(near(102.0, 250.0), None, "half a unit above it");
    assert_eq!(
        near(130.0, 300.0),
        Some([1.0, 1.0]),
        "on the line between two readings: the nearer one"
    );
    assert_eq!(near(170.0, 300.0), Some([2.0, 1.0]));
    let hz_height = (400.0 - map.height_of(50.0) * 100.0) as f32;
    assert_eq!(
        near(151.0, hz_height),
        Some([1.5, map.height_of(50.0)]),
        "on Frequency's reading, at its drawn height"
    );
}

/// A height reads to the decimals one point of the plot resolves, on each
/// axis, and not at all on an axis with no finite span.
#[test]
fn the_height_tags_resolve_a_point() {
    // 4 V over 300 points: 0.013 V a point.
    assert_eq!(height_decimals(4.0 / 300.0), Some(2));
    assert_eq!(height_decimals(400.0 / 300.0), Some(0));
    assert_eq!(height_decimals(f64::INFINITY), None);
    assert_eq!(height_decimals(0.0), None);

    let (left, right) = height_tags(121.8, 4.0 / 300.0, "V", &[], usize::MAX).unwrap();
    assert_eq!(left, "121.80 V");
    assert!(right.is_empty());
    // A level is capped at a height's single decimal.
    let (left, _) = height_tags(2.37, 5.0 / 300.0, "", &[], 1).unwrap();
    assert_eq!(left, "2.4");
    assert_eq!(height_tags(f64::NAN, 0.01, "V", &[], usize::MAX), None);
    // A right axis reads the same height in its own unit, to the decimals a
    // point resolves in that unit.
    let map = crate::graph::axes::fit_secondary(118.0, 122.0, 1.0, (59.95, 60.05));
    let maps = [("Hz".to_string(), map)];
    let (_, right) = height_tags(121.8, 4.0 / 300.0, "V", &maps, usize::MAX).unwrap();
    let per_point = (map.value_at(121.8 + 4.0 / 300.0) - map.value_at(121.8)).abs();
    let decimals = height_decimals(per_point).unwrap();
    assert_eq!(
        right,
        vec![(0, format!("{:.decimals$} Hz", map.value_at(121.8)))]
    );
    assert_eq!(height_tags(1.0, f64::INFINITY, "V", &[], usize::MAX), None);
}

/// The text a frame of the main plot draws with the pointer at `pointer`,
/// after warm-up frames there: the hover's tooltip and tags among it.
fn hover_texts(g: &mut Graph, pointer: egui::Pos2) -> Vec<String> {
    fn walk(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, texts)),
            _ => {}
        }
    }
    let ctx = crate::app::styled_context();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut texts = Vec::new();
    // A tooltip shows from its second frame: the first only sizes it.
    for _ in 0..3 {
        let output = headless_frame(
            &ctx,
            Some(gesture_screen()),
            None,
            vec![egui::Event::PointerMoved(pointer)],
            |ui| {
                g.show_main(
                    ui,
                    &crate::l10n::english(),
                    &tc,
                    &crate::markers::Markers::default(),
                    None,
                    NO_DIGITS,
                )
            },
        );
        texts.clear();
        for clipped in &output.shapes {
            walk(&clipped.shape, &mut texts);
        }
    }
    texts
}

/// The pointer on screen over the plotted reading at `t` seconds, `dy`
/// points above it, from the last frame's plot.
fn over_reading(g: &Graph, t: f64, dy: f32) -> egui::Pos2 {
    let transform = g.plot_transform.expect("a plot was drawn");
    let (_, v) = g.nearest_point(t).expect("a reading");
    transform.position_from_point(&egui_plot::PlotPoint::new(t, v)) - egui::vec2(0.0, dy)
}

/// One series: the tooltip gives the time and the reading alone, and the
/// left axis tags the pointer's height.
#[test]
fn the_hover_reads_one_series_and_tags_the_height() {
    let mut g = graph_with_a_minute_of_data();
    hover_texts(&mut g, over_plot());
    // Above the reading at 3 s (3 V), off the line: the time still snaps
    // to it, and the height is the pointer's.
    let pointer = over_reading(&g, 3.0, 20.0);
    let texts = hover_texts(&mut g, pointer);
    assert!(texts.contains(&"3.0000 V".to_string()), "{texts:?}");
    let tag = texts.iter().find(|t| {
        t.strip_suffix(" V")
            .and_then(|v| v.parse::<f64>().ok())
            .is_some_and(|v| v > 3.0)
    });
    assert!(tag.is_some(), "no height tag above 3 V: {texts:?}");
}

/// Several series: a named row each in the key's order, and a tag on each
/// right axis.
#[test]
fn the_hover_lists_every_series_and_tags_every_axis() {
    let mut g = graph_with_hz();
    hover_texts(&mut g, over_plot());
    let pointer = over_reading(&g, 1.0, 0.0);
    let texts = hover_texts(&mut g, pointer);
    let at = |prefix: &str| texts.iter().position(|t| t.starts_with(prefix));
    let (main, hz, period) = (at("Main: "), at("Frequency: "), at("Period: "));
    assert!(
        main.is_some() && hz.is_some() && period.is_some(),
        "{texts:?}"
    );
    assert!(
        main < hz && hz < period,
        "rows out of the key's order: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.ends_with(" Hz") && !t.starts_with("Frequency")),
        "no Hz tag: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.ends_with(" ms") && !t.starts_with("Period")),
        "no ms tag: {texts:?}"
    );
}

/// Under a Y: Fixed range of "inf" the hover reads without a non-finite
/// number reaching the screen.
#[test]
fn the_hover_survives_an_infinite_y_range() {
    let mut g = graph_with_a_minute_of_data();
    g.y_axis.fixed = true;
    g.y_axis.max = crate::graph::NumberField::new("inf", f64::INFINITY);
    hover_texts(&mut g, over_plot());
    let texts = hover_texts(&mut g, over_plot());
    assert!(
        !texts.iter().any(|t| t.contains("NaN") || t.contains("inf")),
        "{texts:?}"
    );
}
