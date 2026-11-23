//! The **Plot:** and **Show:** chips: the graph's key, and its toggles.

use super::*;
use crate::graph::key::{KeyStyle, SAMPLE_WIDTH};
use crate::graph::toolbar::{overlay_chip_label, series_chip_label};
use crate::settings::{Settings, ThemeMode};
use crate::theme::contrast;

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
/// existed: one line, and no chips to name it.
#[test]
fn a_meter_without_sub_values_has_no_chips() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        g.push(20.0, t0 + Duration::from_secs(i), "DC V", "V", None);
    }
    assert!(!g.has_series_row());
    assert!(drawn_overlay_labels(&g).is_empty());
}

/// Hiding a trace from the Show: chips must take it off the plot and out of
/// the Y range — the axis stretching to frame a trace that isn't drawn would
/// flatten the one that is.
#[test]
fn hiding_an_overlay_drops_it_from_the_plot_and_the_y_range() {
    let mut g = graph_with_two_overlays();
    let (_, hi) = g.y_min_max_padded(0.0, 2.0, true).expect("range");
    assert!(hi >= 60.0, "both overlays framed to start with: {hi}");

    g.toggle_overlay_hidden("T3".to_string());

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

/// Every overlay hidden leaves the plotted series alone on the plot.
#[test]
fn hiding_every_overlay_draws_none() {
    let mut g = graph_with_two_overlays();
    g.hidden_overlays.insert("T2".to_string());
    g.hidden_overlays.insert("T3".to_string());
    assert_eq!(drawn_names(&g), vec!["Main"]);
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

/// A graph with a main trace and four sub-values in units of their own:
/// the most chips the toolbar draws, five under **Plot:** and four under
/// **Show:**.
fn graph_with_four_overlays() -> Graph {
    let mut g = Graph::new();
    let t0 = Instant::now();
    for i in 0..3 {
        g.push_sample(PlotSample {
            value: Some(230.0),
            timestamp: t0 + Duration::from_secs(i),
            mode: "V AC",
            unit: "V",
            display_raw: None,
            series: None,
            main_label: None,
            levels: false,
            overlays: &[
                ("Frequency", "Hz", Some(50.0)),
                ("Period", "ms", Some(20.0)),
                ("Maximum", "V", Some(231.0)),
                ("Minimum", "V", Some(229.0)),
            ],
        });
    }
    g
}

/// One frame of the toolbar alone in a `width`-wide window: the rect it
/// took, the width it was given, and what it painted.
fn toolbar_frame(
    g: &mut Graph,
    ctx: &egui::Context,
    tc: &ThemeColors,
    width: f32,
) -> (egui::Rect, f32, Vec<egui::Shape>) {
    fn flatten(shape: egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.into_iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 400.0));
    let mut taken = (egui::Rect::NOTHING, 0.0);
    let mut shapes = Vec::new();
    // The second frame: the first lays out the fonts.
    for _ in 0..2 {
        let out = headless_frame(ctx, Some(screen), None, vec![], |ui| {
            let room = ui.available_width();
            let rect = ui
                .scope(|ui| g.show_toolbar(ui, &crate::l10n::english(), tc))
                .response
                .rect;
            taken = (rect, room);
        });
        shapes.clear();
        for clipped in out.shapes {
            flatten(clipped.shape, &mut shapes);
        }
    }
    (taken.0, taken.1, shapes)
}

/// How many dashes or dots of a line sample in `color` the frame painted.
fn sample_marks(shapes: &[egui::Shape], color: egui::Color32) -> usize {
    shapes
        .iter()
        .filter(|shape| match shape {
            egui::Shape::LineSegment { points, stroke } => {
                stroke.color == color
                    && points[0].y == points[1].y
                    && (points[1].x - points[0].x).abs() <= SAMPLE_WIDTH
            }
            egui::Shape::Circle(dot) => dot.fill == color,
            _ => false,
        })
        .count()
}

/// The chips are the graph's key: the selected **Plot:** chip carries the
/// plotted line's sample, each **Show:** chip its trace's, switched off or
/// on, and no other **Plot:** chip carries one.
#[test]
fn the_chips_carry_their_lines_samples() {
    let (ctx, tc) = themed_context(Settings::default());
    let mut g = graph_with_two_overlays();
    g.set_series_options(&[("T2", "\u{00B0}C"), ("T3", "\u{00B0}C")], Instant::now());
    let lines = [
        KeyStyle::Plotted.line(&tc, false).0,
        KeyStyle::Overlay(0).line(&tc, false).0,
        KeyStyle::Overlay(1).line(&tc, false).0,
    ];
    let marks = |g: &mut Graph| {
        let (_, _, shapes) = toolbar_frame(g, &ctx, &tc, 800.0);
        lines.map(|color| sample_marks(&shapes, color))
    };
    let on = marks(&mut g);
    assert_eq!(on[0], 1, "one solid sample, on the selected Plot: chip");
    assert!(
        on[1] >= 2 && on[2] >= 2,
        "a pattern on each Show: chip: {on:?}"
    );

    g.toggle_overlay_hidden("T3".to_string());
    assert_eq!(marks(&mut g), on, "a chip switched off keeps its sample");
}

/// A chip names its unit only where the meter's series are in several.
#[test]
fn a_chip_names_its_unit_where_the_series_have_several() {
    use crate::graph::toolbar::chip_text;
    let l10n = crate::l10n::english();
    assert_eq!(chip_text(&l10n, "Frequency", "Hz", true), "Frequency (Hz)");
    assert_eq!(chip_text(&l10n, "T2", "\u{00B0}C", false), "T2");
    assert_eq!(chip_text(&l10n, "Duty", "", true), "Duty");

    let mut g = graph_with_two_overlays();
    g.set_series_options(&[("T2", "\u{00B0}C"), ("T3", "\u{00B0}C")], Instant::now());
    assert!(!g.series_in_several_units(), "every series in one unit");
    g.set_series_options(&[("T2", "\u{00B0}C"), ("Frequency", "Hz")], Instant::now());
    assert!(g.series_in_several_units());
}

/// A choice the graph can't draw yet, as while paused, leaves the sample on
/// the chip of the series the plotted line still is.
#[test]
fn the_plotted_lines_sample_stays_on_the_series_drawn() {
    let (ctx, tc) = themed_context(Settings::default());
    let mut g = graph_with_two_overlays();
    g.set_series_options(
        &[("T2", "\u{00B0}C"), ("Late", "\u{00B0}C")],
        Instant::now(),
    );
    let plotted = KeyStyle::Plotted.line(&tc, false).0;
    let sample_left = |g: &mut Graph| {
        let (_, _, shapes) = toolbar_frame(g, &ctx, &tc, 800.0);
        shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::LineSegment { points, stroke } if stroke.color == plotted => {
                    Some(points[0].x)
                }
                _ => None,
            })
            .expect("a plotted sample")
    };
    let on_main = sample_left(&mut g);

    // "Late" has no trace kept beside the plotted one: nothing to swap to.
    g.selected_series = Some("Late".to_string());
    assert!(!g.swap_to_selected());
    assert_eq!(g.current_series, None);
    assert_eq!(sample_left(&mut g), on_main);
}

/// The solid-lines setting reaches the chips as it does the plot.
#[test]
fn the_chips_samples_follow_the_solid_lines_setting() {
    let (ctx, tc) = themed_context(Settings::default());
    let mut g = graph_with_two_overlays();
    g.solid_lines = true;
    let (_, _, shapes) = toolbar_frame(&mut g, &ctx, &tc, 800.0);
    for k in 0..2 {
        let (color, style) = KeyStyle::Overlay(k).line(&tc, true);
        assert_eq!(style, egui_plot::LineStyle::Solid);
        assert_eq!(sample_marks(&shapes, color), 1, "overlay {k}");
    }
}

/// The two groups' frames in a frame's shapes, **Plot:** first.
fn group_frames(ctx: &egui::Context, shapes: &[egui::Shape]) -> Vec<egui::Rect> {
    let fill = ctx.global_style().visuals.faint_bg_color;
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
            _ => None,
        })
        .collect()
}

/// The four sub-values of [`graph_with_four_overlays`], offered under
/// **Plot:** too.
fn graph_with_every_chip() -> Graph {
    let mut g = graph_with_four_overlays();
    g.set_series_options(
        &[
            ("Frequency", "Hz"),
            ("Period", "ms"),
            ("Maximum", "V"),
            ("Minimum", "V"),
        ],
        Instant::now(),
    );
    g
}

/// The **Show:** group sits beside **Plot:** while the row holds both whole,
/// to the point: one more than the two frames and the gap between them. A
/// point narrower it has a row to itself, and is no wider than it was — not
/// squeezed into what **Plot:** left.
#[test]
fn the_show_group_takes_its_own_row_when_the_plot_row_is_full() {
    let (ctx, tc) = themed_context(Settings::default());
    let frames = |width: f32| {
        let (_, room, shapes) = toolbar_frame(&mut graph_with_every_chip(), &ctx, &tc, width);
        (group_frames(&ctx, &shapes), room)
    };
    let (wide, _) = frames(1200.0);
    let [plot, show] = wide[..] else {
        panic!("two groups: {wide:?}");
    };
    assert_eq!(plot.top(), show.top(), "one row while there is room");
    let both = show.right() - plot.left();

    let margins = 1200.0 - frames(1200.0).1;
    let (fits, _) = frames((both + margins).ceil());
    assert_eq!(fits[0].top(), fits[1].top(), "{fits:?}");
    assert_eq!(fits[1].width(), show.width(), "{fits:?}");

    let (wrapped, room) = frames((both + margins).ceil() - 2.0);
    assert!(wrapped[0].bottom() <= wrapped[1].top(), "{wrapped:?}");
    assert_eq!(wrapped[1].left(), wrapped[0].left(), "{wrapped:?}");
    assert_eq!(wrapped[1].width(), show.width(), "{wrapped:?}");
    assert!(wrapped[1].right() <= room, "{wrapped:?} in {room}");
}

/// Measuring the **Show:** group lays it out a second time, unseen, and a
/// screen reader still finds each chip once.
#[test]
fn a_screen_reader_finds_each_show_chip_once() {
    let (ctx, tc) = themed_context(Settings::default());
    ctx.enable_accesskit();
    let mut g = graph_with_every_chip();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 400.0));
    let mut frame = || {
        headless_frame(&ctx, Some(screen), None, vec![], |ui| {
            g.show_toolbar(ui, &crate::l10n::english(), &tc)
        })
    };
    frame();
    let (nodes, _) = a11y(&frame());
    for series in ["Frequency", "Period", "Maximum", "Minimum"] {
        let name = format!("Show {series} trace");
        let named = nodes.iter().filter(|(_, n)| n.label() == Some(&name));
        assert_eq!(named.count(), 1, "{name}");
    }
}

/// With every chip it can have, the toolbar stays inside the width it is
/// given down to the narrowest window the app allows, and a little under: a
/// chip with its sample moves to the next row whole, and narrower costs
/// rows, a few of them.
#[test]
fn the_toolbar_with_sampled_chips_wraps_inside_its_width() {
    let (_, tc) = themed_context(Settings::default());
    let mut heights = Vec::new();
    for width in [1200.0, 600.0, 454.0, 396.0, 380.0] {
        // A context each: an overflow would widen the next frames' rows.
        let (ctx, _) = themed_context(Settings::default());
        let (rect, room, shapes) = toolbar_frame(&mut graph_with_every_chip(), &ctx, &tc, width);
        assert!(
            rect.width() <= room,
            "{width}: the toolbar took {} of {room}",
            rect.width()
        );
        for group in group_frames(&ctx, &shapes) {
            assert!(group.right() <= room + 0.5, "{width}: {group:?} in {room}");
        }
        heights.push(rect.height());
    }
    assert!(
        heights.windows(2).all(|w| w[0] <= w[1]),
        "narrower costs rows, never width: {heights:?}"
    );
    let row = heights[1] - heights[0];
    assert!(row > 0.0, "{heights:?}");
    assert!(heights[4] <= heights[0] + 3.0 * row, "{heights:?}");
}

/// Every built-in palette: the three presets in both modes, and the named
/// themes.
fn every_palette() -> Vec<(String, Settings)> {
    let presets = [
        ColorPreset::Default,
        ColorPreset::HighContrast,
        ColorPreset::ColorblindSafe,
    ]
    .into_iter()
    .flat_map(|color_preset| {
        [ThemeMode::Dark, ThemeMode::Light].map(|theme| {
            let settings = Settings {
                theme,
                color_preset,
                ..Settings::default()
            };
            (format!("{color_preset:?} {theme:?}"), settings)
        })
    });
    let themes = crate::theme::named::builtin().iter().map(|theme| {
        let settings = Settings {
            named_theme: Some(theme.name.clone()),
            ..Settings::default()
        };
        (theme.name.clone(), settings)
    });
    presets.chain(themes).collect()
}

/// A chip's sample shows on its ground, the plot's background, at the 3:1
/// a graphic needs under every built-in palette — whatever fill the chip's
/// state gives it. `cargo test chips_samples_show -- --nocapture` lists each
/// ratio.
#[test]
fn the_chips_samples_show_on_their_ground() {
    for (name, settings) in every_palette() {
        let (_, tc) = themed_context(settings);
        let ground = tc.plot_background();
        let lines = [KeyStyle::Plotted]
            .into_iter()
            .chain((0..3).map(KeyStyle::Overlay));
        for line in lines {
            let ratio = contrast(line.line(&tc, false).0, ground);
            println!("{name:22} {line:?}: {ratio:.2}:1");
            assert!(ratio >= 3.0, "{name}, {line:?}: {ratio:.2}:1");
        }
    }
}

/// The toolbar's accessible names, laid out in `width` under Touch or
/// Compact; a few frames, so the first row's measured rest settles.
fn toolbar_names(g: &mut Graph, width: f32, touch: bool) -> Vec<String> {
    let ctx = if touch {
        crate::app::styled_context_touch()
    } else {
        crate::app::styled_context()
    };
    ctx.enable_accesskit();
    let (_, tc) = themed_context(Settings::default());
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 400.0));
    let mut out = None;
    for _ in 0..3 {
        out = Some(headless_frame(&ctx, Some(screen), None, vec![], |ui| {
            g.show_toolbar(ui, &crate::l10n::english(), &tc);
        }));
    }
    let (nodes, _) = a11y(&out.expect("drawn"));
    nodes
        .iter()
        .filter_map(|(_, n)| n.label().map(str::to_string))
        .collect()
}

/// The time windows keep their chips while the first row holds them and
/// become one list where it would wrap: a phone in portrait under Touch.
#[test]
fn the_time_windows_collapse_only_where_their_row_would_wrap() {
    let has = |names: &[String], name: &str| names.iter().any(|n| n == name);
    let wide = toolbar_names(&mut Graph::new(), 1200.0, true);
    assert!(has(&wide, "5s") && !has(&wide, "Time window"), "{wide:?}");
    let phone = toolbar_names(&mut Graph::new(), 395.0, true);
    assert!(
        has(&phone, "Time window") && !has(&phone, "5s"),
        "{phone:?}"
    );
    // Compact's row fits the same room: the room decides, not the density.
    let compact = toolbar_names(&mut Graph::new(), 395.0, false);
    assert!(
        has(&compact, "5s") && !has(&compact, "Time window"),
        "{compact:?}"
    );
}
