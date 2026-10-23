//! The alarm limits the graph draws, as the app's alarm hands them over.
//!
//! The alarm lives beside the reading (`app/alarm.rs`): it judges the main
//! reading whatever the graph plots. The graph draws its limits only while
//! they apply to the trace in view, and its menu asks the app to set or
//! remove one at a height (`MenuAction::SetLimit`).

use super::Graph;
use dmm_lib::alarm::Limits;

/// One of the alarm's two limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LimitSide {
    Low,
    High,
}

impl LimitSide {
    /// The limit's name, as its line's label writes it.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::High => "High",
        }
    }
}

/// The limits of an alarm bound to the main reading's quantity and judging
/// it now, handed to the graph each frame it draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WatchedLimits {
    /// In base units, or the scale's unit when `scaled`.
    pub(crate) limits: Limits,
    /// Readings go through a software transform, so the limits are already
    /// in the plotted unit.
    pub(crate) scaled: bool,
}

impl Graph {
    /// The limit lines to draw, in the plotted unit, with their names: none
    /// while a sub-value is plotted, a trace the alarm doesn't judge.
    pub(super) fn limit_lines(&self, watched: Option<WatchedLimits>) -> Vec<(f64, LimitSide)> {
        let Some(watched) = watched else {
            return Vec::new();
        };
        if self.selected_series_offer().is_some() {
            return Vec::new();
        }
        let limits = watched.limits.in_unit(&self.current_unit, watched.scaled);
        [(limits.high, LimitSide::High), (limits.low, LimitSide::Low)]
            .into_iter()
            .filter_map(|(v, side)| Some((v?, side)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_follow_the_plotted_unit() {
        let mut g = Graph::new();
        g.current_unit = "mV".to_string();
        let watched = WatchedLimits {
            limits: Limits::check(None, Some(5.0)).unwrap(),
            scaled: false,
        };
        assert!(g.limit_lines(None).is_empty());
        assert_eq!(
            g.limit_lines(Some(watched)),
            vec![(5000.0, LimitSide::High)]
        );
    }
}
