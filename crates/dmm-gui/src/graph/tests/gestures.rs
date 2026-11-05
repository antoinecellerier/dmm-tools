//! Pointer gestures on the main plot, and the overlay shortcuts.

use super::*;

// These drive `show_main` through a real `egui::Context` so the gesture goes
// the way it does in the app: through egui's input state (which is what folds
// Ctrl+wheel into `zoom_delta` and keeps it out of `smooth_scroll_delta`) and
// through the plot's own hit testing.

/// Outside the plot: the Y-axis strip is at least 60 pt wide
/// (`y_axis_min_width`), so the top-left corner belongs to the axis widget,
/// not the plot.
fn off_plot() -> egui::Pos2 {
    egui::pos2(2.0, 2.0)
}

fn wheel_event(modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        // Under 8 points, which is what makes egui apply the tick in this
        // pass instead of smoothing it out over the next few.
        delta: egui::vec2(0.0, 4.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers,
    }
}

/// Drive one headless frame of the main graph with the pointer at `pointer`
/// and `events` delivered to that frame.
///
/// egui hit-tests against the previous pass's widget rects, so the plot only
/// reports `contains_pointer` from the second frame the pointer is over it —
/// every caller runs a warm-up frame first.
fn gesture_frame(g: &mut Graph, ctx: &egui::Context, pointer: egui::Pos2, events: &[egui::Event]) {
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let mut input = vec![egui::Event::PointerMoved(pointer)];
    input.extend_from_slice(events);
    headless_frame(ctx, Some(gesture_screen()), None, input, |ui| {
        g.show_main(
            ui,
            &crate::l10n::english(),
            &tc,
            &crate::markers::Markers::default(),
            None,
            NO_DIGITS,
        )
    });
}

/// The panels the graph sits in need the plain wheel for scrolling, so a bare
/// tick must leave the graph exactly as it was — no zoom, and no drop out of
/// live either, which is what the old handler did on the first tick.
#[test]
fn plain_wheel_over_the_plot_leaves_the_graph_alone() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, over_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        over_plot(),
        &[wheel_event(egui::Modifiers::NONE)],
    );
    assert_eq!(g.time_window_secs, window, "plain wheel zoomed the graph");
    assert!(g.live, "plain wheel dropped out of live mode");
}

/// Ctrl+wheel is the zoom gesture. Wheel up narrows the window, and the tick
/// also has to leave live mode, or the zoom would be undone by the next
/// sample snapping the view back.
#[test]
fn ctrl_wheel_over_the_plot_zooms_and_leaves_live() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, over_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        over_plot(),
        &[wheel_event(egui::Modifiers::CTRL)],
    );
    assert!(
        g.time_window_secs < window,
        "wheel up must zoom in: window went from {window} to {}",
        g.time_window_secs
    );
    assert!(!g.live, "a zoom has to leave live mode to survive");
}

/// The gesture is the plot's, not the window's: over the toolbar, the stats
/// panel or a modal the graph must not move.
#[test]
fn ctrl_wheel_off_the_plot_changes_nothing() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    let window = g.time_window_secs;
    gesture_frame(&mut g, &ctx, off_plot(), &[]);
    gesture_frame(
        &mut g,
        &ctx,
        off_plot(),
        &[wheel_event(egui::Modifiers::CTRL)],
    );
    assert_eq!(
        g.time_window_secs, window,
        "zoomed without the pointer over the plot"
    );
    assert!(g.live, "left live mode without the pointer over the plot");
}

// --- Overlay shortcuts -------------------------------------------------
//
// Driven through `Graph::show`, the way the app calls it: the key handler
// runs before the toolbar is drawn, so a chip that appears in the same frame
// already shows the new state — which is what lets the `R` test read the
// caret off the field it opens.

#[test]
fn m_and_x_toggle_the_mean_and_the_envelope() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_tap(egui::Key::M));
    assert!(g.show_mean, "M did not draw the mean line");
    graph_frame(&mut g, &ctx, key_tap(egui::Key::X));
    assert!(g.show_envelope, "X did not draw the min/max band");
    graph_frame(&mut g, &ctx, key_tap(egui::Key::M));
    graph_frame(&mut g, &ctx, key_tap(egui::Key::X));
    assert!(
        !g.show_mean && !g.show_envelope,
        "a second press left them on"
    );
}

/// The Triggers chip is drawn only while the reference lines are, so its key
/// has nothing to toggle until they are on.
#[test]
fn t_waits_for_the_reference_lines() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    let crossings = g.show_crossings;
    graph_frame(&mut g, &ctx, key_tap(egui::Key::T));
    assert_eq!(
        g.show_crossings, crossings,
        "T moved the triggers with Ref off"
    );
    g.show_ref_line = true;
    graph_frame(&mut g, &ctx, key_tap(egui::Key::T));
    assert_eq!(
        g.show_crossings, !crossings,
        "T left the triggers alone with Ref on"
    );
}

/// Switching the lines on from the keyboard has to land the caret in the
/// field their values are typed into — nothing is drawn until one is.
#[test]
fn r_opens_the_reference_field_and_takes_the_caret_to_it() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_tap(egui::Key::R));
    assert!(g.show_ref_line, "R did not switch the reference lines on");
    assert!(
        !g.focus_ref_field,
        "the field left the focus request pending"
    );
    // The one text entry the toolbar draws here: Y:Fixed and Min/Max are off.
    assert!(ctx.text_edit_focused(), "the caret is not in a text field");
}

/// The keys belong to whatever holds the keyboard, so the caret `R` just
/// placed in the values field types there instead.
#[test]
fn a_focused_field_keeps_the_overlay_keys() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_tap(egui::Key::R));
    graph_frame(&mut g, &ctx, key_tap(egui::Key::M));
    assert!(!g.show_mean, "M drew the mean line from inside the field");
    graph_frame(&mut g, &ctx, key_tap(egui::Key::R));
    assert!(
        g.show_ref_line,
        "R switched the lines off from their own field"
    );
}

/// Cursors off is the chip's clearing too: a pair left behind would come
/// back with the next press, measuring between two points the user has
/// forgotten placing.
#[test]
fn c_clears_the_cursors_on_the_way_off() {
    let ctx = crate::app::styled_context();
    let mut g = graph_with_a_minute_of_data();
    graph_frame(&mut g, &ctx, key_tap(egui::Key::C));
    assert!(g.cursors.active, "C did not switch the cursors on");
    g.cursors.a = Some(1.0);
    g.cursors.b = Some(2.0);
    g.cursors.next_is_b = true;
    graph_frame(&mut g, &ctx, key_tap(egui::Key::C));
    assert!(!g.cursors.active, "C did not switch the cursors off");
    assert_eq!(
        (g.cursors.a, g.cursors.b),
        (None, None),
        "a cursor survived"
    );
    assert!(!g.cursors.next_is_b, "the next-click side survived");
}
