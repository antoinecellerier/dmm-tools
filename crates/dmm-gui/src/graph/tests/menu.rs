//! The plot's right-click menu.

use super::*;

/// The markers, the alarm limits and the meter's digits for every main
/// reading a menu test's graph is drawn with.
type MenuScene = (
    crate::markers::Markers,
    Option<crate::graph::WatchedLimits>,
    Option<String>,
);

thread_local! {
    /// What the menu tests' graph is drawn with, set by the test that needs
    /// it: the helpers below draw a frame each from several places.
    static MENU_SCENE: std::cell::RefCell<MenuScene> = std::cell::RefCell::default();
}

/// One frame of the main graph with AccessKit on; the tree it produced.
fn menu_frame(
    g: &mut Graph,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Vec<(egui::accesskit::NodeId, egui::accesskit::Node)> {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let out = MENU_SCENE.with_borrow(|(markers, limits, shown)| {
        // The meter's digits for its main reading; none for a sub-value.
        let digits = |_, series: Option<&str>| series.is_none().then(|| shown.clone()).flatten();
        let digits: ReadingDigits = &digits;
        headless_frame(ctx, Some(gesture_screen()), None, events, |ui| {
            g.show_main(ui, &crate::l10n::english(), &tc, markers, *limits, digits)
        })
    });
    a11y(&out).0
}

/// Where the first node whose label `label` accepts was drawn.
fn labelled_rect(
    tree: &[(egui::accesskit::NodeId, egui::accesskit::Node)],
    label: impl Fn(&str) -> bool,
) -> Option<egui::Rect> {
    node_rect(tree, |n| n.label().is_some_and(&label))
}

/// A press and release of `button` at `pos`, on frames of their own.
fn menu_click(g: &mut Graph, ctx: &egui::Context, pos: egui::Pos2, button: egui::PointerButton) {
    for events in click_frames(pos, button) {
        menu_frame(g, ctx, events);
    }
}

/// Right-click at `x` across the plot (0 its left edge, 1 its right), then
/// pick "Mark the reading at …"; what the graph asks to mark. `between` runs after
/// the menu opens, before the pick.
fn mark_from_menu(
    g: &mut Graph,
    x: f32,
    between: impl FnOnce(&mut Graph),
) -> Option<(Instant, f64)> {
    let ctx = open_menu(g, |plot| {
        egui::pos2(plot.left() + x * plot.width(), plot.center().y)
    });
    between(g);
    pick_entry(g, &ctx, |l| l.starts_with("Mark the reading at "));
    g.take_menu_action().map(|action| match action {
        crate::graph::MenuAction::Mark(at, value) => (at, value),
        other => panic!("not a mark: {other:?}"),
    })
}

/// Right-click the plot at the point `at` picks in its rect; the context,
/// with the menu open.
fn open_menu(g: &mut Graph, at: impl FnOnce(egui::Rect) -> egui::Pos2) -> egui::Context {
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    menu_frame(g, &ctx, vec![]);
    let tree = menu_frame(g, &ctx, vec![]);
    let plot = labelled_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    menu_click(g, &ctx, at(plot), egui::PointerButton::Secondary);
    assert!(egui::Popup::is_any_open(&ctx), "the menu opened");
    ctx
}

/// The labels of the open menu's entries, top to bottom.
fn menu_entries(g: &mut Graph, ctx: &egui::Context) -> Vec<String> {
    let mut entries: Vec<(f64, String)> = menu_frame(g, ctx, vec![])
        .iter()
        .filter(|(_, n)| n.role() == egui::accesskit::Role::Button)
        .filter_map(|(_, n)| Some((n.bounds()?.y0, n.label()?.to_owned())))
        .collect();
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    entries.into_iter().map(|(_, label)| label).collect()
}

/// Click the open menu's entry whose label passes `label`.
fn pick_entry(g: &mut Graph, ctx: &egui::Context, label: impl Fn(&str) -> bool) {
    let tree = menu_frame(g, ctx, vec![]);
    let entry = labelled_rect(&tree, label).expect("the entry");
    menu_click(g, ctx, entry.center(), egui::PointerButton::Primary);
    assert!(!egui::Popup::is_any_open(ctx), "picking closes it");
}

/// The value the plot's Y axis has at `pos`, as last drawn.
fn height_at(g: &Graph, pos: egui::Pos2) -> f64 {
    g.plot_transform.expect("drawn").value_from_position(pos).y
}

/// "Mark the reading" marks the reading nearest the right-click.
#[test]
fn the_menu_marks_the_reading_right_clicked() {
    let (mut g, t0) = graph_over(30);
    let (at, value) = mark_from_menu(&mut g, 0.98, |_| {}).expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(30), "the newest, at the right");
    assert_eq!(value, 1.0);
}

/// Live, the view moves on while the menu is open; the mark stays on the
/// reading that was right-clicked.
#[test]
fn a_live_view_moving_on_leaves_the_menus_reading() {
    let (mut g, t0) = graph_over(30);
    let (at, _) = mark_from_menu(&mut g, 0.98, |g| {
        for i in 31..=40 {
            g.push(2.0, t0 + Duration::from_secs(i), "DC V", "V", None);
        }
    })
    .expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(30));
}

/// A right-click names a reading in view, even when one out of view is
/// nearer the time it landed on.
#[test]
fn the_menu_marks_a_reading_in_view() {
    let (mut g, t0) = graph_over(30);
    g.live = false;
    // A view from 5.2 s to 15.2 s: at its left edge, 5 s is nearer than
    // 6 s, and out of view.
    g.view_center = 10.2;
    let (at, _) = mark_from_menu(&mut g, 0.005, |_| {}).expect("a mark");
    assert_eq!(at, t0 + Duration::from_secs(6));
}

/// Esc closes the menu and leaves the focus with nothing, so the graph's
/// keys still work.
#[test]
fn esc_closes_the_menu_and_leaves_no_focus() {
    let (mut g, _) = graph_over(30);
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    menu_frame(&mut g, &ctx, vec![]);
    let tree = menu_frame(&mut g, &ctx, vec![]);
    let plot = labelled_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    menu_click(&mut g, &ctx, plot.center(), egui::PointerButton::Secondary);
    menu_frame(&mut g, &ctx, vec![]);
    assert!(
        ctx.memory(|m| m.focused()).is_some(),
        "the entry has the focus"
    );
    menu_frame(&mut g, &ctx, key_tap(egui::Key::Escape));
    menu_frame(&mut g, &ctx, vec![]);
    assert!(!egui::Popup::is_any_open(&ctx), "Esc closed it");
    assert!(
        ctx.memory(|m| m.focused()).is_none(),
        "nothing keeps the focus"
    );
    assert!(g.take_menu_action().is_none());
}

/// The reading's entries name it by its time, and "Copy" by the digits it
/// copies: the meter's where the app has them, else the trace's value.
#[test]
fn the_menu_names_the_reading() {
    let (mut g, _) = graph_over(30);
    let right = |plot: egui::Rect| egui::pos2(plot.left() + 0.98 * plot.width(), plot.center().y);
    let ctx = open_menu(&mut g, right);
    let entries = menu_entries(&mut g, &ctx);
    for entry in [
        "Mark the reading at 30.0 s",
        "Copy 1.0000 V",
        "Put a cursor on the reading at 30.0 s",
    ] {
        assert!(
            entries.contains(&entry.to_string()),
            "{entry} in {entries:?}"
        );
    }

    MENU_SCENE.set((Default::default(), None, Some(" 1.000".to_string())));
    let ctx = open_menu(&mut g, right);
    pick_entry(&mut g, &ctx, |l| l == "Copy 1.000");
    assert_eq!(
        g.take_menu_action(),
        Some(crate::graph::MenuAction::Copy("1.000".to_string()))
    );

    // A sub-value chosen, but no reading of it plotted yet: the trace is
    // still the main reading's, and so are the digits.
    g.set_series_options(&[("AC", "V")], Instant::now());
    g.selected_series = Some("AC".to_string());
    let ctx = open_menu(&mut g, right);
    let entries = menu_entries(&mut g, &ctx);
    assert!(entries.contains(&"Copy 1.000".to_string()), "{entries:?}");
}

/// Without the meter's digits, Copy writes the point as a figure: in a
/// prefix that fits it, not a megohm reading the trace re-expressed in Ω
/// after an auto-range step.
#[test]
fn copy_without_digits_writes_a_fitting_prefix() {
    let mut g = Graph::new();
    g.time_window_secs = 10.0;
    let t0 = Instant::now();
    for i in 0..=30 {
        g.push(
            1.2345,
            t0 + Duration::from_secs(i),
            "\u{3a9}",
            "M\u{3a9}",
            None,
        );
    }
    g.push(
        980.0,
        t0 + Duration::from_secs(31),
        "\u{3a9}",
        "\u{3a9}",
        None,
    );
    let middle = |plot: egui::Rect| egui::pos2(plot.left() + 0.5 * plot.width(), plot.center().y);
    let ctx = open_menu(&mut g, middle);
    let entries = menu_entries(&mut g, &ctx);
    assert!(
        entries.contains(&"Copy 1.2345 M\u{3a9}".to_string()),
        "{entries:?}"
    );
}

/// "Add reference line" draws one at the height right-clicked, rounded, and
/// switches the lines on without taking the caret into their field.
#[test]
fn the_menu_adds_a_reference_line_where_clicked() {
    let (mut g, _) = graph_over(30);
    let mut at = egui::Pos2::ZERO;
    let ctx = open_menu(&mut g, |plot| {
        at = plot.center() - egui::vec2(0.0, plot.height() / 4.0);
        at
    });
    pick_entry(&mut g, &ctx, |l| l.starts_with("Add reference line at "));
    assert!(g.show_ref_line);
    assert!(!g.focus_ref_field, "the value is chosen already");
    let &[v] = g.ref_lines.values() else {
        panic!("one line: {:?}", g.ref_lines.values());
    };
    let pixel = (height_at(&g, at) - height_at(&g, at + egui::vec2(0.0, 1.0))).abs();
    assert!(
        (v - height_at(&g, at)).abs() <= pixel,
        "{v} is where clicked"
    );
    assert_eq!(g.ref_lines.text_mut().as_str(), format!("{v} V"));
    assert!(g.take_menu_action().is_none());
}

/// Switched off with the chip, the lines come back with the one added.
#[test]
fn a_line_added_brings_back_the_hidden_ones() {
    let (mut g, _) = graph_over(30);
    g.ref_lines.set(&[0.5], &g.current_unit.clone());
    let ctx = open_menu(&mut g, |plot| plot.center());
    pick_entry(&mut g, &ctx, |l| l.starts_with("Add reference line at "));
    assert!(g.show_ref_line);
    assert_eq!(g.ref_lines.values().len(), 2);
    assert_eq!(g.ref_lines.values()[0], 0.5);
}

/// "Remove reference line" is offered over a line, and only there.
#[test]
fn the_menu_removes_the_reference_line_under_the_pointer() {
    let (mut g, _) = graph_over(30);
    g.show_ref_line = true;
    let ctx = crate::app::styled_context();
    menu_frame(&mut g, &ctx, vec![]);
    let at = egui::pos2(400.0, 250.0);
    let v = height_at(&g, at);
    g.ref_lines.set(&[v, v + 1000.0], &g.current_unit.clone());

    let ctx = open_menu(&mut g, |_| at + egui::vec2(0.0, 3.0));
    let entries = menu_entries(&mut g, &ctx);
    let remove: Vec<_> = entries
        .iter()
        .filter(|l| l.starts_with("Remove reference line"))
        .collect();
    let entry = crate::graph::axes::short_figure(v);
    assert_eq!(remove, [&format!("Remove reference line {entry} V")]);
    pick_entry(&mut g, &ctx, |l| l.starts_with("Remove reference line"));
    let &[kept] = g.ref_lines.values() else {
        panic!("one line left: {:?}", g.ref_lines.values());
    };
    assert!((kept - (v + 1000.0)).abs() < 1e-6, "{kept}");

    // A line added from the menu: right-clicked again in the same place,
    // the menu offers no second one at its height.
    let ctx = open_menu(&mut g, |_| at);
    pick_entry(&mut g, &ctx, |l| l.starts_with("Add reference line"));
    let ctx = open_menu(&mut g, |_| at);
    let entries = menu_entries(&mut g, &ctx);
    assert!(
        entries
            .iter()
            .any(|l| l.starts_with("Remove reference line"))
    );
    assert!(
        !entries.iter().any(|l| l.starts_with("Add reference line")),
        "{entries:?}"
    );

    let ctx = open_menu(&mut g, |_| at + egui::vec2(0.0, 20.0));
    assert!(
        !menu_entries(&mut g, &ctx)
            .iter()
            .any(|l| l.starts_with("Remove reference line")),
        "nothing under the pointer"
    );
}

/// The cursor entries follow the cursors placed: one puts the first, then
/// one puts the missing cursor and one moves each placed one, so picking the
/// same entry twice makes a pair rather than moving the cursor just put.
#[test]
fn the_menus_cursor_entries_follow_the_cursors_placed() {
    let (mut g, _) = graph_over(30);
    let right = |plot: egui::Rect| egui::pos2(plot.left() + 0.98 * plot.width(), plot.center().y);
    let left = |plot: egui::Rect| egui::pos2(plot.left() + 0.02 * plot.width(), plot.center().y);
    let cursor_entries = |g: &mut Graph, at: fn(egui::Rect) -> egui::Pos2| {
        let ctx = open_menu(g, at);
        let entries = menu_entries(g, &ctx)
            .into_iter()
            .filter(|l| l.contains("cursor"))
            .collect::<Vec<_>>();
        (ctx, entries)
    };

    let (ctx, entries) = cursor_entries(&mut g, right);
    assert_eq!(entries, ["Put a cursor on the reading at 30.0 s"]);
    pick_entry(&mut g, &ctx, |l| l.starts_with("Put a cursor"));
    assert!(g.cursors.active);
    assert_eq!((g.cursors.a, g.cursors.b), (Some(30.0), None));

    let (ctx, entries) = cursor_entries(&mut g, left);
    assert_eq!(
        entries,
        [
            "Put cursor B on the reading at 20.0 s",
            "Move cursor A to the reading at 20.0 s",
            "Remove cursors",
        ]
    );
    pick_entry(&mut g, &ctx, |l| l.starts_with("Put cursor B"));
    assert_eq!((g.cursors.a, g.cursors.b), (Some(30.0), Some(20.0)));
    assert!(!g.cursors.next_is_b, "a click moves the other one, A");

    let (ctx, entries) = cursor_entries(&mut g, |plot| plot.center());
    assert_eq!(
        entries,
        [
            "Move cursor A to the reading at 25.0 s",
            "Move cursor B to the reading at 25.0 s",
            "Remove cursors",
        ]
    );
    pick_entry(&mut g, &ctx, |l| l.starts_with("Move cursor A"));
    assert_eq!((g.cursors.a, g.cursors.b), (Some(25.0), Some(20.0)));

    let ctx = open_menu(&mut g, right);
    pick_entry(&mut g, &ctx, |l| l == "Remove cursors");
    assert!(!g.cursors.active);
    assert_eq!((g.cursors.a, g.cursors.b), (None, None));
}

/// "Remove cursor" is offered over a cursor's line; the last one removed
/// switches the cursors off.
#[test]
fn the_menu_removes_the_cursor_under_the_pointer() {
    let (mut g, _) = graph_over(30);
    g.cursors.set(crate::graph::Cursor::A, 25.0);
    g.cursors.set(crate::graph::Cursor::B, 22.0);
    let ctx = open_menu(&mut g, |plot| plot.center() + egui::vec2(3.0, 0.0));
    let entries = menu_entries(&mut g, &ctx);
    assert!(
        entries.contains(&"Move cursor B to the reading at 25.0 s".to_string()),
        "{entries:?}"
    );
    assert!(
        !entries.iter().any(|l| l.starts_with("Move cursor A")),
        "A is on that reading already: {entries:?}"
    );
    pick_entry(&mut g, &ctx, |l| l == "Remove cursor A");
    assert!(g.cursors.active);
    assert_eq!((g.cursors.a, g.cursors.b), (None, Some(22.0)));
    assert!(!g.cursors.next_is_b, "a click puts A back");

    let ctx = open_menu(&mut g, |plot| plot.center());
    assert!(
        !menu_entries(&mut g, &ctx)
            .iter()
            .any(|l| l.starts_with("Remove cursor ")),
        "no cursor under the pointer"
    );
    g.cursors.remove(crate::graph::Cursor::B);
    assert!(!g.cursors.active, "the last one gone");
}

/// "Delete marker" is offered over a marker's line, and only there.
#[test]
fn the_menu_deletes_the_marker_under_the_pointer() {
    let (mut g, t0) = graph_over(30);
    MENU_SCENE.set((marked(&[t0 + Duration::from_secs(25)]), None, None));
    // The 10 s window from 20 s to 30 s: the marker halfway across.
    let ctx = open_menu(&mut g, |plot| plot.center() + egui::vec2(3.0, 0.0));
    pick_entry(&mut g, &ctx, |l| l == "Delete marker 1");
    assert_eq!(
        g.take_menu_action(),
        Some(crate::graph::MenuAction::DeleteMarker(1))
    );

    let ctx = open_menu(&mut g, |plot| plot.center() + egui::vec2(20.0, 0.0));
    assert!(
        !menu_entries(&mut g, &ctx)
            .iter()
            .any(|l| l.starts_with("Delete marker")),
        "no marker under the pointer"
    );
}

/// With the cursors on, the click that closes the menu places none; the
/// next click does.
#[test]
fn the_click_closing_the_menu_places_no_cursor() {
    let (mut g, _) = graph_over(30);
    g.cursors.active = true;
    let ctx = open_menu(&mut g, |plot| plot.center());
    let tree = menu_frame(&mut g, &ctx, vec![]);
    let plot = labelled_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    let elsewhere = egui::pos2(plot.left() + 0.8 * plot.width(), plot.top() + 10.0);
    menu_click(&mut g, &ctx, elsewhere, egui::PointerButton::Primary);
    assert!(!egui::Popup::is_any_open(&ctx), "the click closed it");
    assert_eq!(g.cursors.at(crate::graph::Cursor::A), None);
    menu_click(&mut g, &ctx, elsewhere, egui::PointerButton::Primary);
    assert!(g.cursors.at(crate::graph::Cursor::A).is_some());
}

/// The alarm's entries set a limit at the height clicked, and remove the
/// limit under the pointer.
#[test]
fn the_menu_sets_and_removes_alarm_limits() {
    let (mut g, _) = graph_over(30);
    let ctx = open_menu(&mut g, |plot| plot.center());
    pick_entry(&mut g, &ctx, |l| l.starts_with("Set alarm high at "));
    let Some(crate::graph::MenuAction::SetLimit(crate::graph::LimitSide::High, v)) =
        g.take_menu_action()
    else {
        panic!("a high limit");
    };
    let watched = crate::graph::WatchedLimits {
        limits: dmm_lib::alarm::Limits::check(None, Some(v)).unwrap(),
        unit: dmm_lib::alarm::LimitUnit::Base,
    };
    MENU_SCENE.set((crate::markers::Markers::default(), Some(watched), None));
    let ctx = open_menu(&mut g, |plot| plot.center() + egui::vec2(0.0, 2.0));
    pick_entry(&mut g, &ctx, |l| l == "Remove alarm high");
    assert_eq!(
        g.take_menu_action(),
        Some(crate::graph::MenuAction::ClearLimit(
            crate::graph::LimitSide::High
        ))
    );
}

/// The alarm judges the main reading: no limit is set from a sub-value's
/// trace, nor from a level's.
#[test]
fn no_alarm_entries_off_the_main_reading() {
    let has_alarm = |g: &mut Graph| {
        let ctx = open_menu(g, |plot| plot.center());
        menu_entries(g, &ctx)
            .iter()
            .any(|l| l.starts_with("Set alarm"))
    };
    let (mut g, t0) = graph_over(30);
    assert!(has_alarm(&mut g));
    g.set_series_options(&[("AC", "V")], t0 + Duration::from_secs(30));
    g.selected_series = Some("AC".to_string());
    assert!(
        has_alarm(&mut g),
        "picked, but the trace is the main one till AC comes"
    );
    push_acdc_plotting_ac(&mut g, t0 + Duration::from_secs(31), None, Some(0.5));
    assert!(!has_alarm(&mut g), "a sub-value");
    g.selected_series = None;
    assert!(
        !has_alarm(&mut g),
        "the main reading picked back, its trace not in yet"
    );

    let (mut g, _) = graph_over(30);
    g.levels = true;
    assert!(!has_alarm(&mut g), "a level");
}

/// Right-clicked on an axis, the menu offers what that axis measures: the
/// value axis what is done at a value, the time axis what is done to a
/// reading.
#[test]
fn an_axis_opens_the_menu_for_what_it_measures() {
    let (mut g, _) = graph_over(30);
    let on_value_axis = |plot: egui::Rect| egui::pos2(plot.left() - 20.0, plot.center().y);
    let ctx = open_menu(&mut g, on_value_axis);
    let entries = menu_entries(&mut g, &ctx);
    assert!(
        entries
            .iter()
            .any(|l| l.starts_with("Add reference line at "))
    );
    assert!(entries.iter().any(|l| l.starts_with("Set alarm high at ")));
    assert!(
        !entries.iter().any(|l| l.contains("the reading")),
        "{entries:?}"
    );
    pick_entry(&mut g, &ctx, |l| l.starts_with("Add reference line at "));
    assert_eq!(g.ref_lines.values().len(), 1);

    let on_time_axis = |plot: egui::Rect| egui::pos2(plot.center().x, plot.bottom() + 8.0);
    let ctx = open_menu(&mut g, on_time_axis);
    let entries = menu_entries(&mut g, &ctx);
    assert!(entries.contains(&"Mark the reading at 25.0 s".to_string()));
    assert!(
        !entries
            .iter()
            .any(|l| l.contains("reference line") || l.contains("alarm")),
        "{entries:?}"
    );
}

/// The axes are no zoom handles: egui_plot's drag-an-axis zoom is off, so
/// hovering one shows no resize cursor and dragging it leaves the view.
#[test]
fn dragging_an_axis_leaves_the_view() {
    let (mut g, _) = graph_over(30);
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let markers = crate::markers::Markers::default();
    let frame = |g: &mut Graph, events: Vec<egui::Event>| {
        headless_frame(&ctx, Some(gesture_screen()), None, events, |ui| {
            g.show_main(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
        })
        .platform_output
    };
    frame(&mut g, vec![]);
    let tree = frame(&mut g, vec![])
        .accesskit_update
        .map(|update| update.nodes)
        .unwrap_or_default();
    let plot = labelled_rect(&tree, |l| l.contains("last reading")).expect("the plot");
    let bounds = *g.plot_transform.expect("drawn").bounds();
    for (axis, from, to) in [
        (
            "value",
            egui::pos2(plot.left() - 20.0, plot.center().y),
            egui::pos2(plot.left() - 20.0, plot.center().y + 40.0),
        ),
        (
            "time",
            egui::pos2(plot.center().x, plot.bottom() + 8.0),
            egui::pos2(plot.center().x + 40.0, plot.bottom() + 8.0),
        ),
    ] {
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let hover = frame(&mut g, vec![egui::Event::PointerMoved(from)]);
        assert!(
            !matches!(
                hover.cursor_icon,
                egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeVertical
            ),
            "{axis} axis: {:?}",
            hover.cursor_icon
        );
        frame(&mut g, vec![button(from, true)]);
        frame(&mut g, vec![egui::Event::PointerMoved(to)]);
        assert_eq!(
            *g.plot_transform.expect("drawn").bounds(),
            bounds,
            "mid-drag on the {axis} axis"
        );
        frame(&mut g, vec![button(to, false)]);
    }
}

/// Right-clicked, a marker's flag offers that marker's deletion alone; a
/// click still opens its note.
#[test]
fn a_flag_opens_the_menu_for_its_marker() {
    let (mut g, t0) = graph_over(30);
    MENU_SCENE.set((marked(&[t0 + Duration::from_secs(25)]), None, None));
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    menu_frame(&mut g, &ctx, vec![]);
    let tree = menu_frame(&mut g, &ctx, vec![]);
    let flag = labelled_rect(&tree, |l| l == "Write marker 1's note").expect("the flag");
    menu_click(&mut g, &ctx, flag.center(), egui::PointerButton::Secondary);
    let entries: Vec<String> = menu_entries(&mut g, &ctx)
        .into_iter()
        .filter(|l| !l.starts_with("Write marker"))
        .collect();
    assert_eq!(entries, ["Delete marker 1"], "the flag's own button aside");
    pick_entry(&mut g, &ctx, |l| l == "Delete marker 1");
    assert_eq!(
        g.take_menu_action(),
        Some(crate::graph::MenuAction::DeleteMarker(1))
    );
    assert_eq!(g.take_clicked_marker(), None, "not the note's click");
}

/// The arrows walk the entries that can be picked and Enter picks one.
#[test]
fn the_menu_answers_the_arrows() {
    let (mut g, _) = graph_over(30);
    let ctx = open_menu(&mut g, |plot| plot.center());
    menu_frame(&mut g, &ctx, vec![]);
    menu_frame(&mut g, &ctx, key_tap(egui::Key::ArrowDown));
    menu_frame(&mut g, &ctx, vec![]);
    menu_frame(&mut g, &ctx, key_tap(egui::Key::Enter));
    menu_frame(&mut g, &ctx, vec![]);
    assert!(!egui::Popup::is_any_open(&ctx), "Enter picked");
    assert!(
        matches!(
            g.take_menu_action(),
            Some(crate::graph::MenuAction::Copy(..))
        ),
        "the second entry"
    );
}

/// A trace re-expressed under the open menu — here a decade step — takes
/// the menu with it: its heights were in the old prefix.
#[test]
fn a_restart_closes_the_menu() {
    let (mut g, t0) = graph_over(30);
    let ctx = open_menu(&mut g, |plot| plot.center());
    g.push(1.0, t0 + Duration::from_secs(31), "DC V", "mV", None);
    menu_frame(&mut g, &ctx, vec![]);
    assert!(!egui::Popup::is_any_open(&ctx));
    assert!(g.menu.is_none());
}
