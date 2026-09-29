//! The graph's view as an export saves it: which stretch is shown, the Y
//! axis, and the overlays on it.
//!
//! Every time is in seconds from the exported file's first reading, not from
//! the graph's own origin, which moves each time the trace restarts: an
//! import or a playback of the file puts that reading at its own origin, and
//! the view lands where it was.

use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::Graph;

/// What an export saves of the view. Every field is optional on the way in,
/// so a view from an older or newer version still applies what it can.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ViewState {
    /// Width of the time window, seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) window: Option<f64>,
    /// Where the window starts; absent while it followed the newest reading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start: Option<f64>,
    /// A fixed Y axis; absent for Y:Auto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) y: Option<YRange>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) mean: bool,
    /// The Min/Max envelope's window, seconds, when it is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) envelope: Option<f64>,
    /// The reference values, and whether their lines are shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) references: Vec<f64>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) references_shown: bool,
    /// The trigger markers; absent keeps the graph's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) triggers: Option<bool>,
    /// The cursors, when they are on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cursors: Option<Cursors>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct YRange {
    pub(crate) min: f64,
    pub(crate) max: f64,
}

/// Cursor positions, each absent until placed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Cursors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) b: Option<f64>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// `a - b` in seconds, either way round.
fn seconds_between(a: Instant, b: Instant) -> f64 {
    match a.checked_duration_since(b) {
        Some(d) => d.as_secs_f64(),
        None => -b.duration_since(a).as_secs_f64(),
    }
}

impl Graph {
    /// Seconds from `first`, the file's first reading, to the graph's origin:
    /// what a time on the graph is shifted by to be a time in the file.
    fn origin_after(&self, first: Instant) -> f64 {
        self.origin
            .map_or(0.0, |origin| seconds_between(origin, first))
    }

    /// The view as an export whose first reading was taken at `first` saves
    /// it.
    pub(crate) fn view_state(&self, first: Instant) -> ViewState {
        let shift = self.origin_after(first);
        let (view_min, _) = self.view_bounds();
        ViewState {
            window: Some(self.time_window_secs),
            start: (!self.live).then_some(view_min + shift),
            y: self.y_axis_fixed.then(|| YRange {
                min: self.y_min.value(),
                max: self.y_max.value(),
            }),
            mean: self.show_mean,
            envelope: self.show_envelope.then(|| self.envelope_window.value()),
            references: self.ref_lines.values().to_vec(),
            references_shown: self.show_ref_line,
            triggers: Some(self.show_crossings),
            cursors: self.cursors_active.then_some(Cursors {
                a: self.cursor_a.map(|t| t + shift),
                b: self.cursor_b.map(|t| t + shift),
            }),
        }
    }

    /// Put back a view saved by an export whose first reading is the one
    /// taken at `first`. What makes no sense here — a window of no width, a
    /// cursor past the readings the graph holds — is left as it is.
    pub(crate) fn apply_view_state(&mut self, view: &ViewState, first: Instant) {
        let shift = self.origin_after(first);
        let usable = |v: &f64| v.is_finite() && *v > 0.0;
        if let Some(window) = view.window.filter(usable) {
            self.time_window_secs = window;
        }
        match view.start.filter(|s| s.is_finite()) {
            Some(start) => {
                self.view_center = start - shift + self.time_window_secs / 2.0;
                self.live = false;
            }
            None => self.live = true,
        }
        match view.y {
            Some(YRange { min, max }) if min.is_finite() && max.is_finite() && min < max => {
                self.y_axis_fixed = true;
                self.y_user_set = true;
                self.y_min.restore(min);
                self.y_max.restore(max);
            }
            _ => self.y_axis_fixed = false,
        }
        self.show_mean = view.mean;
        self.show_envelope = view.envelope.is_some_and(|w| usable(&w));
        if let Some(window) = view.envelope.filter(usable) {
            self.envelope_window.restore(window);
        }
        let references: Vec<f64> = view
            .references
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect();
        self.ref_lines.set(&references);
        self.show_ref_line = view.references_shown;
        if let Some(triggers) = view.triggers {
            self.show_crossings = triggers;
        }
        let (data_min, data_max) = self.data_time_range();
        let held = |t: f64| (t.is_finite() && (data_min..=data_max).contains(&t)).then_some(t);
        match view.cursors {
            Some(Cursors { a, b }) => {
                self.cursors_active = true;
                self.cursor_a = a.and_then(|t| held(t - shift));
                self.cursor_b = b.and_then(|t| held(t - shift));
                self.cursor_next_is_b = self.cursor_a.is_some() && self.cursor_b.is_none();
            }
            None => {
                self.cursors_active = false;
                self.cursor_a = None;
                self.cursor_b = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A graph of `n` readings a second apart from `t0`.
    fn graph_from(t0: Instant, n: u64) -> Graph {
        let mut g = Graph::new();
        for i in 0..n {
            g.push(i as f64, t0 + Duration::from_secs(i), "DC V", "V", None);
        }
        g
    }

    /// A view saved from one session lands where it was in another, whose
    /// clock put the same readings at other instants.
    #[test]
    fn a_view_lands_where_it_was_in_another_session() {
        let t0 = Instant::now();
        let mut g = graph_from(t0, 60);
        g.time_window_secs = 20.0;
        g.center_view_on(25.0);
        g.show_mean = true;
        g.show_envelope = true;
        g.envelope_window.set(5.0);
        g.ref_lines.set(&[1.5, 3.0]);
        g.show_ref_line = true;
        g.show_crossings = false;
        g.y_axis_fixed = true;
        g.y_min.set(-1.0);
        g.y_max.set(70.0);
        g.cursors_active = true;
        g.cursor_a = Some(12.0);
        g.cursor_b = Some(30.0);
        let saved = serde_json::to_string(&g.view_state(t0)).expect("serialises");

        let t1 = t0 + Duration::from_secs(3600);
        let mut h = graph_from(t1, 60);
        h.apply_view_state(&serde_json::from_str(&saved).expect("reads back"), t1);
        assert_eq!(h.view_bounds(), g.view_bounds());
        assert!(!h.live);
        assert_eq!((h.cursor_a, h.cursor_b), (Some(12.0), Some(30.0)));
        assert!(h.show_mean && h.show_envelope && h.show_ref_line && !h.show_crossings);
        assert_eq!(h.envelope_window.value(), 5.0);
        assert_eq!(h.ref_lines.values(), [1.5, 3.0]);
        assert!(h.y_axis_fixed);
        assert_eq!((h.y_min.value(), h.y_max.value()), (-1.0, 70.0));
    }

    /// Times count from the file's first reading, not the graph's origin: a
    /// graph that restarted part-way through the file shifts them back.
    #[test]
    fn times_count_from_the_files_first_reading() {
        let first = Instant::now();
        // The graph starts ten seconds into the file.
        let mut g = graph_from(first + Duration::from_secs(10), 30);
        g.cursors_active = true;
        g.cursor_a = Some(5.0);
        let view = g.view_state(first);
        assert_eq!(view.cursors.and_then(|c| c.a), Some(15.0));
        let mut h = graph_from(first + Duration::from_secs(10), 30);
        h.apply_view_state(&view, first);
        assert_eq!(h.cursor_a, Some(5.0));
    }

    /// A view from another version reads: unknown fields are skipped,
    /// missing ones leave the default; nonsense is left out.
    #[test]
    fn a_view_applies_what_it_can() {
        let view: ViewState =
            serde_json::from_str(r#"{"window":-3,"mean":true,"plotted":"AC","cursors":{"a":1e9}}"#)
                .expect("unknown fields are skipped");
        let t0 = Instant::now();
        let mut g = graph_from(t0, 10);
        let window = g.time_window_secs;
        g.apply_view_state(&view, t0);
        assert_eq!(
            g.time_window_secs, window,
            "a window of no width is left out"
        );
        assert!(g.show_mean);
        assert!(g.cursors_active);
        assert_eq!(g.cursor_a, None, "a cursor past the readings is dropped");
        assert!(g.live, "no start: the view follows the newest reading");
    }
}

#[cfg(test)]
mod screenshot_views {
    use super::*;

    /// Every key `ViewState` writes, and so reads back.
    fn known_keys() -> Vec<String> {
        let full = ViewState {
            window: Some(1.0),
            start: Some(0.0),
            y: Some(YRange { min: 0.0, max: 1.0 }),
            mean: true,
            envelope: Some(1.0),
            references: vec![1.0],
            references_shown: true,
            triggers: Some(true),
            cursors: Some(Cursors {
                a: Some(0.0),
                b: Some(0.0),
            }),
        };
        match serde_json::to_value(full).expect("serialises") {
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            _ => unreachable!("a struct serialises as an object"),
        }
    }

    /// The views the doc screenshots load are typed by hand into the script,
    /// and a view skips what it doesn't know: a misspelt key would quietly
    /// leave a picture with the default view. Every key must be one the app
    /// reads, and every value must read.
    #[test]
    fn the_screenshot_views_are_all_read() {
        let script = include_str!("../../../../scripts/doc-screenshots.sh");
        let known = known_keys();
        let views: Vec<&str> = script
            .lines()
            .filter_map(|l| l.split_once("_VIEW='# view: "))
            .map(|(_, rest)| rest.trim_end_matches('\''))
            .collect();
        assert!(views.len() >= 5, "found {views:?}");
        for view in views {
            let value: serde_json::Value = serde_json::from_str(view).expect(view);
            let serde_json::Value::Object(map) = &value else {
                panic!("not an object: {view}")
            };
            for key in map.keys() {
                assert!(known.contains(key), "`{key}` in {view} is not a view field");
            }
            let _: ViewState = serde_json::from_value(value).expect(view);
        }
    }
}
