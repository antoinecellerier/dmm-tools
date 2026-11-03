//! Markers on the plot.

use super::*;

/// The export history is cut to this, so it has to move exactly when the
/// graph's oldest point does: on a restart, on eviction and on Clear, and
/// not on an over-range reading, which adds no point.
#[test]
fn first_point_time_follows_the_oldest_point() {
    let mut g = Graph::new();
    assert_eq!(g.first_point_time(), None);
    let t0 = Instant::now();
    let at = |ms: u64| t0 + Duration::from_millis(ms);

    g.push(1.0, at(0), "DC V", "V", None);
    g.push(1.0, at(1), "DC V", "V", None);
    assert_eq!(g.first_point_time(), Some(at(0)));

    g.push_break(at(2));
    assert_eq!(g.first_point_time(), Some(at(0)), "no point, no move");

    g.push(1.0, at(3), "AC V", "V", None);
    assert_eq!(g.first_point_time(), Some(at(3)), "a mode change restarts");

    g.push(1.0, at(4), "AC V", "V", None);
    g.push(1.0, at(5), "AC V", "V", None);
    g.set_max_points(2);
    assert_eq!(g.first_point_time(), Some(at(4)), "evicted with its point");

    g.clear();
    assert_eq!(g.first_point_time(), None);
}

/// Markers sit at their reading's time from the origin, and only those in
/// the window are asked for.
#[test]
fn markers_sit_at_their_readings_time() {
    let (g, t0) = graph_over(30);
    let markers = marked(&[t0 + Duration::from_secs(5), t0 + Duration::from_secs(20)]);
    let at = |x_min, x_max| -> Vec<(f64, u32)> {
        g.markers_between(&markers, x_min, x_max)
            .into_iter()
            .map(|(x, m)| (x, m.number))
            .collect()
    };
    assert_eq!(at(0.0, 30.0), [(5.0, 1), (20.0, 2)]);
    assert_eq!(at(10.0, 30.0), [(20.0, 2)]);
}

/// A restart moves the origin past the old trace: a marker on it is no
/// longer the graph's to draw, and after Clear nothing is.
#[test]
fn a_restarted_graph_shows_no_marker_from_before() {
    let (mut g, t0) = graph_over(10);
    let markers = marked(&[t0 + Duration::from_secs(5)]);
    assert!(g.holds(t0 + Duration::from_secs(5)));
    g.push(100.0, t0 + Duration::from_secs(11), "Ohm", "Ω", None);
    assert!(!g.holds(t0 + Duration::from_secs(5)));
    assert!(g.markers_between(&markers, 0.0, 1e6).is_empty());
    g.clear();
    assert!(g.markers_between(&markers, 0.0, 1e6).is_empty());
}

/// A marker out of view is brought to the middle of the window, which keeps
/// its width; one in view leaves the view alone; one at the newest reading
/// lands back in live.
#[test]
fn reveal_centres_a_marker_out_of_view() {
    let (mut g, t0) = graph_over(60);
    assert!(g.live);
    g.reveal(t0 + Duration::from_secs(55));
    assert!(g.live, "already in view");

    g.reveal(t0 + Duration::from_secs(20));
    assert!(!g.live);
    let (lo, hi) = g.view_bounds();
    assert_eq!((lo, hi), (15.0, 25.0));

    g.reveal(t0 + Duration::from_secs(22));
    assert_eq!(g.view_bounds(), (15.0, 25.0), "in view: nothing moves");

    g.reveal(t0 + Duration::from_secs(60));
    assert!(g.live, "the newest reading is the live edge");
}

/// A graph that no longer holds a reading has nowhere to show it.
#[test]
fn reveal_ignores_a_reading_the_graph_dropped() {
    let (mut g, t0) = graph_over(10);
    g.push(100.0, t0 + Duration::from_secs(11), "Ohm", "Ω", None);
    let before = (g.live, g.view_center);
    g.reveal(t0 + Duration::from_secs(5));
    assert_eq!((g.live, g.view_center), before);
}

/// A marker's flag takes a click, and the graph reports whose it was.
#[test]
fn clicking_a_flag_reports_its_marker() {
    let (mut g, t0) = graph_over(30);
    let markers = marked(&[t0 + Duration::from_secs(25)]);
    let tc = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
    let ctx = crate::app::styled_context();
    ctx.enable_accesskit();
    // One frame; where the flag labelled for marker 1 was drawn.
    let frame = |g: &mut Graph, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(gesture_screen()),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            g.show_main(ui, &crate::l10n::english(), &tc, &markers, None, NO_DIGITS)
        });
        out.textures_delta.clear();
        out.platform_output.accesskit_update.and_then(|update| {
            update
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Write marker 1's note"))
                .and_then(|(_, n)| n.bounds())
                .map(|b| egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))
        })
    };
    // The axes settle on the first frame, and the flag with them.
    frame(&mut g, vec![]);
    let at = frame(&mut g, vec![]).expect("the flag is drawn and labelled");
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut g, vec![egui::Event::PointerMoved(at)]);
    frame(&mut g, vec![egui::Event::PointerMoved(at), button(true)]);
    frame(&mut g, vec![egui::Event::PointerMoved(at), button(false)]);
    assert_eq!(g.take_clicked_marker(), Some(1));
    assert!(
        g.cursors.a.is_none(),
        "the click is the flag's, not the plot's"
    );
}
