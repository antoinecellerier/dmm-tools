//! The graph's view as an export saves it: which stretch is shown, the Y
//! axis, and the overlays on it.
//!
//! Every time is in seconds from the exported file's first reading, not from
//! the graph's own origin, which moves each time the trace restarts: an
//! import or a playback of the file puts that reading at its own origin, and
//! the view lands where it was.

use dmm_lib::transform::{PrefixRatio, prefix_ratio};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::minimap::MAX_WINDOW_SECS;
use super::{Graph, MeasureCursors};

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
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) histogram: bool,
    /// The Min/Max envelope's window, seconds, when it is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) envelope: Option<f64>,
    /// The reference values, in `unit`, and whether their lines are shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) references: Vec<f64>,
    /// The reference entries as typed, each with its unit ("1 Ω", "5 mV"),
    /// those that draw no line on this trace included. Read in place of
    /// `references` when present; a version from before them reads
    /// `references`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) reference_entries: Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) references_shown: bool,
    /// The trigger markers; absent keeps the graph's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) triggers: Option<bool>,
    /// The cursors, when they are on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cursors: Option<Cursors>,
    /// The trace the Y range and the references were set against: its
    /// mode, its sub-value (absent for the main reading) and its unit, which
    /// is written even with nothing plotted. A view without a unit is from
    /// before they were saved, and applies its range and references to
    /// whatever is plotted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) unit: Option<String>,
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
            // What the view shows: a frozen one narrower than the window
            // comes back as it was.
            window: Some(if self.live {
                self.time_window_secs
            } else {
                self.browse_width()
            }),
            start: (!self.live).then_some(view_min + shift),
            y: self.y_axis.fixed.then(|| YRange {
                min: self.y_axis.min.value(),
                max: self.y_axis.max.value(),
            }),
            mean: self.show_mean,
            histogram: self.show_histogram,
            envelope: self.show_envelope.then(|| self.envelope_window.value()),
            references: self.ref_lines.values().to_vec(),
            reference_entries: self.ref_lines.saved_entries(),
            references_shown: self.show_ref_line,
            triggers: Some(self.show_crossings),
            cursors: self.cursors.active.then_some(Cursors {
                a: self.cursors.a.map(|t| t + shift),
                b: self.cursors.b.map(|t| t + shift),
            }),
            mode: self.current_mode.clone(),
            series: self.current_series.clone(),
            unit: Some(self.current_unit.clone()),
        }
    }

    /// How a value of `view`'s Y range or references reads on the trace
    /// plotted now: `Some(None)` as it is (a view from before the trace was
    /// saved), `Some(Some(ratio))` through a ratio — another prefix of its
    /// unit when a replay applies its view at a reading a decade off the
    /// last —, `None` not at all: another mode, sub-value or quantity, whose
    /// numbers it is not.
    fn view_unit_step(&self, view: &ViewState) -> Option<Option<PrefixRatio>> {
        let Some(unit) = view.unit.as_deref() else {
            return Some(None);
        };
        if view.mode != self.current_mode || view.series != self.current_series {
            return None;
        }
        prefix_ratio(unit, &self.current_unit).map(Some)
    }

    /// Put back a view saved by an export whose first reading is the one
    /// taken at `first`. What makes no sense here — a window of no width, a
    /// window or cursor on none of the readings the graph holds, a Y range
    /// or references set against another trace than the one plotted — is
    /// left as it is, and a window wider than those readings and the widest
    /// zoom is narrowed to fit.
    pub(crate) fn apply_view_state(&mut self, view: &ViewState, first: Instant) {
        let step = self.view_unit_step(view);
        let here = |v: f64| step.map(|r| r.map_or(v, |r| r.apply(v)));
        let shift = self.origin_after(first);
        let (data_min, data_max) = self.data_time_range();
        let usable = |v: &f64| v.is_finite() && *v > 0.0;
        if let Some(window) = view.window.filter(usable) {
            self.time_window_secs = window.min((data_max - data_min).max(MAX_WINDOW_SECS));
        }
        self.frozen_width = None;
        let shows_data = |s: &f64| (data_min - self.time_window_secs..=data_max).contains(s);
        match view.start.map(|s| s - shift).filter(shows_data) {
            Some(start) => {
                self.view_center = start + self.time_window_secs / 2.0;
                self.live = false;
            }
            None => self.live = true,
        }
        let y = view.y.map(|YRange { min, max }| (here(min), here(max)));
        match y {
            // Set against another trace: the range in force stays.
            _ if step.is_none() => {}
            Some((Some(min), Some(max))) if min.is_finite() && max.is_finite() && min < max => {
                self.y_axis.fixed = true;
                self.y_axis.user_set = true;
                self.y_axis.min.restore(min);
                self.y_axis.max.restore(max);
                // A saved view keeps the plotted unit's range only; the
                // right axes fit their data.
                self.y_axis.pins.clear();
            }
            _ => self.y_axis.fixed = false,
        }
        self.show_mean = view.mean;
        self.show_histogram = view.histogram;
        self.show_envelope = view.envelope.is_some_and(|w| usable(&w));
        if let Some(window) = view.envelope.filter(usable) {
            self.envelope_window.restore(window);
        }
        if step.is_some() {
            if view.reference_entries.is_empty() {
                let references: Vec<f64> = view
                    .references
                    .iter()
                    .filter_map(|&v| here(v))
                    .filter(|v| v.is_finite())
                    .collect();
                self.ref_lines.set(&references, &self.current_unit);
            } else {
                self.ref_lines
                    .restore_entries(&view.reference_entries, &self.current_unit);
            }
            self.show_ref_line = view.references_shown;
        }
        if let Some(triggers) = view.triggers {
            self.show_crossings = triggers;
        }
        let held = |t: f64| (t.is_finite() && (data_min..=data_max).contains(&t)).then_some(t);
        match view.cursors {
            Some(Cursors { a, b }) => self.cursors.restore(
                a.and_then(|t| held(t - shift)),
                b.and_then(|t| held(t - shift)),
            ),
            None => self.cursors = MeasureCursors::default(),
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
        g.ref_lines.set(&[1.5, 3.0], &g.current_unit.clone());
        g.show_ref_line = true;
        g.show_crossings = false;
        g.y_axis.fixed = true;
        g.y_axis.min.set(-1.0);
        g.y_axis.max.set(70.0);
        g.cursors.active = true;
        g.cursors.a = Some(12.0);
        g.cursors.b = Some(30.0);
        let saved = serde_json::to_string(&g.view_state(t0)).expect("serialises");

        let t1 = t0 + Duration::from_secs(3600);
        let mut h = graph_from(t1, 60);
        h.apply_view_state(&serde_json::from_str(&saved).expect("reads back"), t1);
        assert_eq!(h.view_bounds(), g.view_bounds());
        assert!(!h.live);
        assert_eq!((h.cursors.a, h.cursors.b), (Some(12.0), Some(30.0)));
        assert!(h.show_mean && h.show_envelope && h.show_ref_line && !h.show_crossings);
        assert_eq!(h.envelope_window.value(), 5.0);
        assert_eq!(h.ref_lines.values(), [1.5, 3.0]);
        assert!(h.y_axis.fixed);
        assert_eq!((h.y_axis.min.value(), h.y_axis.max.value()), (-1.0, 70.0));
    }

    /// A Y range and references go back only on the trace they were set
    /// against: in another prefix of its unit they are re-expressed (a
    /// replay may apply its view at a reading a decade below the last), and
    /// on another mode they are left out, the graph's own staying.
    #[test]
    fn a_range_and_references_go_back_only_on_their_trace() {
        let t0 = Instant::now();
        let mut g = graph_from(t0, 10);
        g.y_axis.fixed = true;
        g.y_axis.min.set(0.5);
        g.y_axis.max.set(1.5);
        g.ref_lines.set(&[1.2], &g.current_unit.clone());
        let view = g.view_state(t0);
        assert_eq!(
            (
                view.mode.as_deref(),
                view.series.as_deref(),
                view.unit.as_deref()
            ),
            (Some("DC V"), None, Some("V"))
        );

        let mut mv = Graph::new();
        mv.push(950.0, t0, "DC V", "mV", None);
        mv.apply_view_state(&view, t0);
        assert!(mv.y_axis.fixed);
        assert_eq!(
            (mv.y_axis.min.value(), mv.y_axis.max.value()),
            (500.0, 1500.0)
        );
        assert_eq!(mv.ref_lines.values(), [1200.0]);

        let mut ac = Graph::new();
        ac.push(0.95, t0, "AC V", "V", None);
        ac.ref_lines.set(&[3.0], &ac.current_unit.clone());
        ac.y_axis.fixed = true;
        ac.y_axis.min.set(0.0);
        ac.apply_view_state(&view, t0);
        assert_eq!(ac.y_axis.min.value(), 0.0, "another mode's range stays out");
        assert!(ac.y_axis.fixed, "and the graph's own stays");
        assert_eq!(ac.ref_lines.values(), [3.0]);

        // Saved with nothing plotted (a Clear before the export): its range
        // belongs to no trace.
        let mut cleared = graph_from(t0, 10);
        cleared.y_axis.fixed = true;
        cleared.clear();
        let empty = cleared.view_state(t0);
        assert_eq!(
            (empty.mode.as_deref(), empty.unit.as_deref()),
            (None, Some(""))
        );
        let mut ac = Graph::new();
        ac.push(0.95, t0, "AC V", "V", None);
        ac.apply_view_state(&empty, t0);
        assert!(!ac.y_axis.fixed);

        // A view from before the trace was saved applies to what is plotted.
        let old = ViewState {
            mode: None,
            series: None,
            unit: None,
            ..view
        };
        let mut ac = Graph::new();
        ac.push(0.95, t0, "AC V", "V", None);
        ac.apply_view_state(&old, t0);
        assert!(ac.y_axis.fixed);
        assert_eq!(ac.ref_lines.values(), [1.2]);
    }

    /// References go back as they were typed, each in its unit, those of
    /// another quantity included; a file without the entries reads the
    /// values.
    #[test]
    fn references_go_back_as_typed() {
        let t0 = Instant::now();
        let mut g = Graph::new();
        g.push(4.7, t0, "\u{3a9}", "k\u{3a9}", None);
        *g.ref_lines.text_mut() = "1 \u{3a9}, 5 mV, 4.7k 2".into();
        g.ref_lines.flush(Some("k\u{3a9}"));
        let view = g.view_state(t0);
        assert_eq!(
            view.reference_entries,
            ["1 \u{3a9}", "5 mV", "4.7k", "2 k\u{3a9}"]
        );
        assert_eq!(view.references, [0.001, 4.7, 2.0]);
        let saved = serde_json::to_string(&view).expect("serialises");

        // Played back at a reading a decade up.
        let mut h = Graph::new();
        h.push(0.0047, t0, "\u{3a9}", "M\u{3a9}", None);
        h.apply_view_state(&serde_json::from_str(&saved).expect("reads back"), t0);
        assert_eq!(h.ref_lines.text(), "1 \u{3a9}, 5 mV, 4.7k, 2 k\u{3a9}");
        assert_eq!(
            h.ref_lines.labels().collect::<Vec<_>>(),
            ["1 \u{3a9}", "4.7 k\u{3a9}", "2 k\u{3a9}"]
        );

        // A file from before the entries were saved.
        let old = ViewState {
            reference_entries: Vec::new(),
            ..view
        };
        let mut h = Graph::new();
        h.push(4.7, t0, "\u{3a9}", "k\u{3a9}", None);
        h.apply_view_state(&old, t0);
        assert_eq!(h.ref_lines.values(), [0.001, 4.7, 2.0]);
    }

    /// Times count from the file's first reading, not the graph's origin: a
    /// graph that restarted part-way through the file shifts them back.
    #[test]
    fn times_count_from_the_files_first_reading() {
        let first = Instant::now();
        // The graph starts ten seconds into the file.
        let mut g = graph_from(first + Duration::from_secs(10), 30);
        g.cursors.active = true;
        g.cursors.a = Some(5.0);
        let view = g.view_state(first);
        assert_eq!(view.cursors.and_then(|c| c.a), Some(15.0));
        let mut h = graph_from(first + Duration::from_secs(10), 30);
        h.apply_view_state(&view, first);
        assert_eq!(h.cursors.a, Some(5.0));
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
        assert!(g.cursors.active);
        assert_eq!(g.cursors.a, None, "a cursor past the readings is dropped");
        assert!(g.live, "no start: the view follows the newest reading");
    }

    /// A view without cursors starts the next measurement afresh: switched
    /// back on, the first click places A even if only A was placed before.
    #[test]
    fn a_view_without_cursors_starts_the_next_at_a() {
        let t0 = Instant::now();
        let mut g = graph_from(t0, 10);
        g.cursors.toggle();
        g.cursors.place(2.0);
        g.apply_view_state(&ViewState::default(), t0);
        assert!(!g.cursors.active);
        g.cursors.toggle();
        g.cursors.place(4.0);
        assert_eq!((g.cursors.a, g.cursors.b), (Some(4.0), None));
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
            histogram: true,
            envelope: Some(1.0),
            references: vec![1.0],
            reference_entries: vec!["1 V".into()],
            references_shown: true,
            triggers: Some(true),
            cursors: Some(Cursors {
                a: Some(0.0),
                b: Some(0.0),
            }),
            mode: Some("DC V".into()),
            series: Some("AC".into()),
            unit: Some("V".into()),
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
