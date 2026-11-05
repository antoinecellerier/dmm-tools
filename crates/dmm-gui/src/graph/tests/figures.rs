//! The histogram strip and the cursor figures.

use super::*;

/// The pane is placed only while its chip is on and the plotted trace is
/// drawn, and the chip says why when it is on and nothing shows.
#[test]
fn the_histogram_shows_while_on_and_its_trace_is_drawn() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(g.histogram_place, None, "off until asked for");
    g.show_histogram = true;
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(
        g.histogram_place.map(|p| p.dock),
        Some(crate::graph::histogram::Dock::Side)
    );
    assert_eq!(
        g.histogram_hover(&crate::l10n::english(), "H"),
        "Show how the visible readings spread (H)"
    );
    g.main_hidden = true;
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(g.histogram_place, None, "no bars for a trace not drawn");
    assert!(
        g.histogram_hover(&crate::l10n::english(), "H")
            .starts_with("Show the plotted trace")
    );
}

/// What a screen reader hears of the histogram is redone in a new
/// language, though the readings it is binned from are the same.
#[test]
fn the_spoken_histogram_follows_the_language() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    g.show_histogram = true;
    graph_frame(&mut g, &ctx, vec![]);
    let english = g.histogram.as_ref().expect("binned").spoken.clone();
    assert!(english.contains("Histogram of"), "{english}");
    static OTHER: dmm_shared::l10n::Language = dmm_shared::l10n::Language {
        id: "x-test",
        native_name: "Test",
        reviewed: true,
    };
    let options = dmm_shared::l10n::Options {
        pseudo: true,
        ..Default::default()
    };
    let other = Localizer::new(&OTHER, &crate::l10n::resources(), &options);
    graph_frame_in(&other, &mut g, &ctx, vec![]);
    let spoken = &g.histogram.as_ref().expect("binned").spoken;
    assert_ne!(*spoken, english, "rebuilt in the new language");
}

/// The cursor readout covers the readings at both cursors and between
/// them, whichever cursor is placed first.
#[test]
fn the_cursor_readout_covers_the_readings_between_the_cursors() {
    let mut g = graph_with_a_minute_of_data();
    g.cursors.active = true;
    g.cursors.a = Some(2.0);
    assert_eq!(
        g.cursor_readout(&crate::l10n::english()),
        None,
        "one cursor placed"
    );
    g.cursors.b = Some(1.0);
    let figures = g
        .cursor_readout(&crate::l10n::english())
        .expect("both cursors placed");
    assert_eq!(figures[0], "\u{0394}T=1.00 s");
    assert_eq!(
        figures[figures.len() - 3..],
        ["Mean=1.5000 V", "\u{3c3}=0.3317 V", "Readings=11"]
    );
}

/// Over-range readings count with the readings they fell between: after a
/// range's first reading up to the one after its last, or up to now after
/// the newest, never the stretch before the first.
#[test]
fn overloads_are_counted_where_they_fell() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);
    g.push(1.0, at(0), "DC V", "V", None);
    for ms in [100, 200, 300] {
        g.push_break(at(ms));
    }
    g.push(1.0, at(400), "DC V", "V", None);
    g.push(1.0, at(500), "DC V", "V", None);
    g.push_break(at(600));
    g.push_break(at(700));
    let len = g.history.len();
    assert_eq!(
        g.overloads_in(0..len),
        5,
        "three before, two still going on"
    );
    assert_eq!(g.overloads_in(0..1), 3, "after the first, before the next");
    assert_eq!(g.overloads_in(1..2), 0, "the stretch before it is not its");
    assert_eq!(g.overloads_in(2..len), 2);
}

/// The cursor figures are kept while the readings between the cursors stay
/// the same, a reading arriving after them included, and computed again for
/// a reading inside the period or a trace restarted on the same timestamps.
#[test]
fn cursor_figures_follow_the_readings_between_the_cursors() {
    let mut g = Graph::new();
    let t0 = Instant::now();
    let push = |g: &mut Graph, i: u64, v: f64| {
        g.push(v, t0 + Duration::from_millis(i * 100), "DC V", "V", None);
    };
    let place_cursors = |g: &mut Graph, a: f64, b: f64| {
        g.cursors.active = true;
        g.cursors.a = Some(a);
        g.cursors.b = Some(b);
    };
    let stats = |g: &mut Graph| g.cursor_figures().expect("both cursors placed").0;
    let key = |g: &Graph| g.cursor_figures.as_ref().map(|f| f.key);
    for i in 0..30 {
        push(&mut g, i, 1.0);
    }
    place_cursors(&mut g, 1.0, 2.0);
    let s = stats(&mut g);
    assert_eq!((s.count, s.avg()), (11, Some(1.0)));
    let before = key(&g);
    push(&mut g, 30, 5.0);
    stats(&mut g);
    assert_eq!(key(&g), before, "a reading after the period leaves them be");
    place_cursors(&mut g, 1.0, 1.5);
    assert_eq!(stats(&mut g).count, 6, "the period moved");

    g.clear();
    for i in 0..30 {
        push(&mut g, i, 2.0);
    }
    place_cursors(&mut g, 1.0, 2.0);
    assert_eq!(
        stats(&mut g).avg(),
        Some(2.0),
        "the same times, other readings"
    );
}

/// The histogram is binned again only when what it is binned from changes:
/// a frame with nothing new keeps it, a reading in view or a cursor placed
/// does not.
#[test]
fn the_histogram_is_binned_again_only_when_its_readings_change() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    g.show_histogram = true;
    graph_frame(&mut g, &ctx, vec![]);
    graph_frame(&mut g, &ctx, vec![]);
    let key = g.histogram.as_ref().map(|b| b.key);
    assert!(key.is_some());
    graph_frame(&mut g, &ctx, vec![]);
    assert_eq!(g.histogram.as_ref().map(|b| b.key), key, "nothing changed");

    g.cursors.active = true;
    g.cursors.a = Some(1.0);
    g.cursors.b = Some(2.0);
    graph_frame(&mut g, &ctx, vec![]);
    assert_ne!(
        g.histogram.as_ref().map(|b| b.key),
        key,
        "the cursors' period is counted"
    );
    assert!(g.histogram().is_some_and(|h| h.has_interval()));

    let key = g.histogram.as_ref().map(|b| b.key);
    g.push(3.0, Instant::now(), "DC V", "V", None);
    graph_frame(&mut g, &ctx, vec![]);
    assert_ne!(
        g.histogram.as_ref().map(|b| b.key),
        key,
        "a new reading in view"
    );
}

/// Tab reaches the strip's divider where it stands on screen: after the
/// plot and before the minimap under both.
#[test]
fn tab_reaches_the_strip_divider_before_the_minimap() {
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut g = graph_with_a_minute_of_data();
    g.show_histogram = true;
    let markers = crate::markers::Markers::default();
    let mut order: Vec<String> = Vec::new();
    for k in 0..40 {
        let events = if k < 2 {
            Vec::new()
        } else {
            key_tap(egui::Key::Tab)
        };
        let out = headless_frame(&ctx, Some(gesture_screen()), None, events, |ui| {
            g.show(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
        });
        let (nodes, focus) = a11y(&out);
        let focused = nodes
            .iter()
            .find(|(id, _)| Some(*id) == focus)
            .and_then(|(_, node)| node.label().map(str::to_owned));
        order.extend(focused);
    }
    let at = |prefix: &str| order.iter().position(|l| l.starts_with(prefix));
    let (divider, minimap) = (at("Resize the histogram"), at("Graph minimap"));
    assert!(divider.is_some() && minimap.is_some(), "{order:?}");
    assert!(divider < minimap, "{order:?}");
}

/// Dragging the divider left of the strip widens it, up to half the graph,
/// down to its least; a window too narrow for the width dragged draws it
/// narrower without forgetting it.
#[test]
fn dragging_the_divider_resizes_the_histogram() {
    use crate::graph::histogram::{MIN_STRIP_WIDTH, STRIP_WIDTH};
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut g = graph_with_a_minute_of_data();
    g.show_histogram = true;
    // One frame `width` wide; where the divider was drawn.
    let frame = |g: &mut Graph, width: f32, events: Vec<egui::Event>| {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
        let markers = crate::markers::Markers::default();
        let out = headless_frame(&ctx, Some(screen), None, events, |ui| {
            g.show(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
        });
        node_rect(&a11y(&out).0, |n| {
            n.label() == Some("Resize the histogram (Left/Right to adjust)")
        })
        .map(|r| r.center())
    };
    let drag = |g: &mut Graph, dx: f32| {
        frame(g, 800.0, vec![]);
        let at = frame(g, 800.0, vec![]).expect("the divider is drawn and labelled");
        let to = at + egui::vec2(dx, 0.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(g, 800.0, vec![egui::Event::PointerMoved(at)]);
        frame(g, 800.0, vec![button(at, true)]);
        frame(g, 800.0, vec![egui::Event::PointerMoved(to)]);
        frame(g, 800.0, vec![button(to, false)]);
    };
    drag(&mut g, -30.0);
    assert_eq!(g.histogram_sizes.0, STRIP_WIDTH + 30.0);
    drag(&mut g, -1000.0);
    let half = g.histogram_sizes.0;
    assert!(half > STRIP_WIDTH + 30.0 && half <= 400.0, "{half}");
    drag(&mut g, 1000.0);
    assert_eq!(g.histogram_sizes.0, MIN_STRIP_WIDTH);
    drag(&mut g, -50.0);
    frame(&mut g, 560.0, vec![]);
    assert_eq!(
        g.histogram_sizes.0,
        MIN_STRIP_WIDTH + 50.0,
        "a narrow window keeps the width dragged"
    );
}
