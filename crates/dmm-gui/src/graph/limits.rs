//! The alarm limits the graph draws, as the app's alarm hands them over.
//!
//! The limits are set beside the reading (`app/alarm.rs`), not here: the
//! alarm judges the main reading whatever the graph plots. The graph only
//! draws them, and only while they apply to the trace in view.

use super::Graph;
use dmm_lib::alarm::Limits;

/// What the app's alarm is watching, for the graph's limit lines.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AlarmView {
    /// The limits in force, in base units; empty while no alarm is set.
    pub(crate) limits: Limits,
    /// The base unit the alarm watches, once a reading has bound it.
    pub(crate) watched: Option<String>,
    /// The base unit of readings of another quantity, while they arrive.
    pub(crate) idle: Option<String>,
    /// Readings go through a software transform, so the limits are already
    /// in the plotted unit.
    pub(crate) scaled: bool,
    /// The unit the limits are in: the watched base unit, or the Scale
    /// row's output unit.
    pub(crate) unit: Option<String>,
}

impl Graph {
    /// The limit lines to draw, in the plotted unit, with their names: none
    /// unless the alarm is watching the plotted series' quantity — a
    /// sub-value plotted, or readings of another quantity, would put the
    /// lines on a trace the alarm doesn't judge.
    pub(super) fn limit_lines(&self) -> Vec<(f64, &'static str)> {
        let view = &self.alarm_view;
        if view.limits.is_empty()
            || view.watched.is_none()
            || view.idle.is_some()
            || self.selected_series_offer().is_some()
        {
            return Vec::new();
        }
        let limits = view.limits.in_unit(&self.current_unit, view.scaled);
        [(limits.high, "High"), (limits.low, "Low")]
            .into_iter()
            .filter_map(|(v, name)| Some((v?, name)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_follow_the_plotted_unit_and_only_while_watched() {
        let mut g = Graph::new();
        g.alarm_view.limits = Limits::check(None, Some(5.0)).unwrap();
        g.current_unit = "mV".to_string();
        assert!(g.limit_lines().is_empty(), "nothing bound yet");
        g.alarm_view.watched = Some("V".to_string());
        assert_eq!(g.limit_lines(), vec![(5000.0, "High")]);
        g.alarm_view.idle = Some("Ω".to_string());
        assert!(g.limit_lines().is_empty());
    }
}
