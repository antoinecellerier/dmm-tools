use crate::measurement::{MeasuredValue, Measurement};
use log::warn;
use std::time::Instant;

/// Tracks min/max/avg statistics for a series of measurements.
///
/// Used by both CLI and GUI to accumulate running statistics
/// over measurement values.
#[derive(Debug, Clone)]
pub struct RunningStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    sum: f64,
    pub count: u64,
}

impl RunningStats {
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            sum: 0.0,
            count: 0,
        }
    }

    /// Record a new value, updating min/max/sum/count.
    pub fn push(&mut self, value: f64) {
        self.min = Some(self.min.map_or(value, |m: f64| m.min(value)));
        self.max = Some(self.max.map_or(value, |m: f64| m.max(value)));
        self.sum += value;
        self.count += 1;
    }

    /// Return the average, or `None` if no values have been pushed.
    pub fn avg(&self) -> Option<f64> {
        if self.count > 0 {
            Some(self.sum / self.count as f64)
        } else {
            None
        }
    }

    /// Reset all statistics to the initial empty state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for RunningStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Default maximum interval (seconds) for the integrator to bridge.
/// Intervals larger than this are treated as gaps (pause, disconnect).
/// ~20× the typical 10 Hz sample interval.
const DEFAULT_MAX_DT_SECS: f64 = 2.0;

/// Tracks the time-integral of a measurement series using the trapezoidal rule.
///
/// For current measurements, the integral gives charge (A·s, convertible to Ah).
/// For voltage, it gives V·s. Overload values create gaps — the integral holds
/// its previous value and resumes from the next normal reading.
///
/// Intervals exceeding `max_dt_secs` are silently skipped to avoid nonsensical
/// spikes after pause or disconnect.
#[derive(Debug, Clone)]
pub struct Integrator {
    integral: f64,
    prev: Option<(f64, Instant)>,
    pub count: u64,
    pub overload_gaps: u64,
    /// Intervals skipped because `dt_secs > max_dt_secs`. Incremented every
    /// time a sample arrives too far after the previous one to contribute a
    /// sensible trapezoid — typical when the user has a low sample rate
    /// (longer than `max_dt_secs`) or after a pause/disconnect.
    pub skipped_intervals: u64,
    max_dt_secs: f64,
    first_time: Option<Instant>,
    last_time: Option<Instant>,
}

impl Integrator {
    pub fn new() -> Self {
        Self {
            integral: 0.0,
            prev: None,
            count: 0,
            overload_gaps: 0,
            skipped_intervals: 0,
            max_dt_secs: DEFAULT_MAX_DT_SECS,
            first_time: None,
            last_time: None,
        }
    }

    /// Create an integrator with a custom maximum interval threshold.
    pub fn with_max_dt(max_dt_secs: f64) -> Self {
        Self {
            max_dt_secs,
            ..Self::new()
        }
    }

    /// Record a normal measurement value, accumulating the trapezoidal area
    /// since the previous sample.
    pub fn push(&mut self, value: f64, timestamp: Instant) {
        if let Some((prev_val, prev_time)) = self.prev
            && let Some(dt) = timestamp.checked_duration_since(prev_time)
        {
            let dt_secs = dt.as_secs_f64();
            if dt_secs <= self.max_dt_secs {
                self.integral += (prev_val + value) / 2.0 * dt_secs;
            } else {
                if self.skipped_intervals == 0 {
                    warn!(
                        "integrator: skipping {dt_secs:.2}s interval (max {:.2}s); \
                         further skips will be counted in skipped_intervals",
                        self.max_dt_secs
                    );
                }
                self.skipped_intervals += 1;
            }
        }
        if self.first_time.is_none() {
            self.first_time = Some(timestamp);
        }
        self.last_time = Some(timestamp);
        self.prev = Some((value, timestamp));
        self.count += 1;
    }

    /// Record an overload reading. Breaks the integration (clears the previous
    /// sample) so the next normal reading starts a fresh interval.
    pub fn push_overload(&mut self) {
        self.prev = None;
        self.overload_gaps += 1;
    }

    /// Raw accumulated integral in unit·seconds (e.g. A·s or V·s).
    pub fn value(&self) -> f64 {
        self.integral
    }

    /// Elapsed time in seconds between the first and last sample pushed.
    /// Returns `None` if fewer than 2 samples have been pushed.
    pub fn elapsed_secs(&self) -> Option<f64> {
        match (self.first_time, self.last_time) {
            (Some(first), Some(last)) if self.count >= 2 => {
                last.checked_duration_since(first).map(|d| d.as_secs_f64())
            }
            _ => None,
        }
    }

    /// Reset all state. Preserves `max_dt_secs`.
    pub fn reset(&mut self) {
        let max_dt = self.max_dt_secs;
        *self = Self::new();
        self.max_dt_secs = max_dt;
    }
}

impl Default for Integrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns `(display_unit, divisor)` for measurement units where time-integration
/// produces a meaningful physical quantity. Divide the raw integral (unit·seconds)
/// by `divisor` to get the display value.
///
/// Returns `None` for units where integration is not meaningful (Ω, F, Hz, °C, %).
pub fn integral_unit_info(unit: &str) -> Option<(&'static str, f64)> {
    match unit {
        "A" => Some(("Ah", 3600.0)),
        "mA" => Some(("mAh", 3600.0)),
        "µA" => Some(("µAh", 3600.0)),
        "V" => Some(("V\u{00b7}s", 1.0)),
        "mV" => Some(("mV\u{00b7}s", 1.0)),
        _ => None,
    }
}

/// Why the accumulators were reset: the reading started a new series.
///
/// Mode *and* unit, because neither check subsumes the other: auto-range
/// moves the unit a decade without touching the mode (mV → V), while a
/// `--unit` relabel pins the unit across a dial turn that changes the mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesChange {
    /// The meter's mode string changed — typically the dial was turned.
    Mode { prev: String, new: String },
    /// The unit changed within the same mode — typically an auto-range step.
    Unit { prev: String, new: String },
}

impl std::fmt::Display for SeriesChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeriesChange::Mode { prev, new } => write!(f, "Mode changed ({prev} \u{2192} {new})"),
            SeriesChange::Unit { prev, new } => write!(f, "Unit changed ({prev} \u{2192} {new})"),
        }
    }
}

/// Session statistics of one comparable series of readings: min/max/avg and
/// the time integral, reset together whenever the mode or the unit changes.
///
/// Min/Max/Avg and an integral are only meaningful within a single mode and
/// unit. Auto-range moves the unit a decade without touching the mode string,
/// and turning the dial changes the mode — which `--unit` would otherwise
/// hide, as it pins the label across the change. Without both checks a run
/// that spans a dial turn averages volts with ohms and prints bare numbers,
/// so nothing on screen would reveal the mix.
///
/// Owned by the CLI read loop and the GUI message drain alike, so the two
/// cannot drift apart on what starts a new series.
#[derive(Debug, Clone)]
pub struct SeriesStats {
    pub stats: RunningStats,
    pub integrator: Integrator,
    integrate: bool,
    /// `(mode, unit)` the current series accumulates in; `None` until the
    /// first reading.
    series: Option<(String, String)>,
}

impl SeriesStats {
    /// Create an empty session.
    ///
    /// With `integrate == false` the integrator is never touched: pushing into
    /// it anyway would log its skipped-interval warning for a feature the user
    /// never asked for (the CLI only integrates under `--integrate`).
    pub fn new(integrate: bool) -> Self {
        Self {
            stats: RunningStats::new(),
            integrator: Integrator::new(),
            integrate,
            series: None,
        }
    }

    /// Record one reading, returning the [`SeriesChange`] that reset the
    /// accumulators, or `None` while the reading continues the current series.
    ///
    /// The first reading only stores the series — it starts one rather than
    /// changing it. When both mode and unit moved, the mode is reported: it
    /// names the change the user made.
    pub fn push(&mut self, m: &Measurement) -> Option<SeriesChange> {
        let change = self.series.as_ref().and_then(|(prev_mode, prev_unit)| {
            if prev_mode.as_str() != m.mode.as_ref() {
                Some(SeriesChange::Mode {
                    prev: prev_mode.clone(),
                    new: m.mode.to_string(),
                })
            } else if prev_unit.as_str() != m.unit.as_ref() {
                Some(SeriesChange::Unit {
                    prev: prev_unit.clone(),
                    new: m.unit.to_string(),
                })
            } else {
                None
            }
        });
        if change.is_some() {
            self.stats.reset();
            self.integrator.reset();
        }
        // Only re-store on a change: an unchanged series would otherwise
        // allocate two strings for every sample of the run.
        if change.is_some() || self.series.is_none() {
            self.series = Some((m.mode.to_string(), m.unit.to_string()));
        }

        match &m.value {
            MeasuredValue::Normal(v) => {
                self.stats.push(*v);
                if self.integrate {
                    self.integrator.push(*v, m.timestamp);
                }
            }
            // Breaks the integration interval; contributes nothing to min/max.
            MeasuredValue::Overload => {
                if self.integrate {
                    self.integrator.push_overload();
                }
            }
            // A detection level is not a measured quantity — neither
            // accumulator has anything to do with it.
            MeasuredValue::NcvLevel(_) => {}
        }
        change
    }

    /// Clear both accumulators, keeping the current series: the CLI's Ctrl+L
    /// and the GUI's Reset button clear the numbers, they don't forget what is
    /// being measured.
    pub fn reset(&mut self) {
        self.stats.reset();
        self.integrator.reset();
    }

    /// Mode the accumulated figures were measured in, or `None` before the
    /// first reading.
    pub fn mode(&self) -> Option<&str> {
        self.series.as_ref().map(|(mode, _)| mode.as_str())
    }

    /// Unit the accumulated figures were measured in, or `None` before the
    /// first reading.
    pub fn unit(&self) -> Option<&str> {
        self.series.as_ref().map(|(_, unit)| unit.as_str())
    }

    /// The integral scaled to its display unit, or `None` when this session
    /// does not integrate, no reading has arrived yet, or the current unit has
    /// no meaningful time-integral (see [`integral_unit_info`]).
    pub fn integral_display(&self) -> Option<(f64, &'static str)> {
        if !self.integrate {
            return None;
        }
        integral_display(self.integrator.value(), self.unit()?)
    }
}

/// Raw unit·seconds integral scaled to its display unit:
/// `(value / divisor, display_unit)`.
///
/// `None` for units where integration is not meaningful — see
/// [`integral_unit_info`].
pub fn integral_display(raw: f64, unit: &str) -> Option<(f64, &'static str)> {
    integral_unit_info(unit).map(|(display_unit, divisor)| (raw / divisor, display_unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn empty_stats() {
        let s = RunningStats::new();
        assert!(s.min.is_none());
        assert!(s.max.is_none());
        assert!(s.avg().is_none());
        assert_eq!(s.count, 0);
    }

    #[test]
    fn single_value() {
        let mut s = RunningStats::new();
        s.push(5.0);
        assert_eq!(s.min, Some(5.0));
        assert_eq!(s.max, Some(5.0));
        assert_eq!(s.avg(), Some(5.0));
        assert_eq!(s.count, 1);
    }

    #[test]
    fn multiple_values() {
        let mut s = RunningStats::new();
        s.push(1.0);
        s.push(3.0);
        s.push(5.0);
        assert_eq!(s.min, Some(1.0));
        assert_eq!(s.max, Some(5.0));
        assert_eq!(s.avg(), Some(3.0));
        assert_eq!(s.count, 3);
    }

    #[test]
    fn negative_values() {
        let mut s = RunningStats::new();
        s.push(-10.0);
        s.push(10.0);
        assert_eq!(s.min, Some(-10.0));
        assert_eq!(s.max, Some(10.0));
        assert_eq!(s.avg(), Some(0.0));
    }

    #[test]
    fn reset_clears_all() {
        let mut s = RunningStats::new();
        s.push(1.0);
        s.push(2.0);
        s.reset();
        assert!(s.min.is_none());
        assert!(s.max.is_none());
        assert!(s.avg().is_none());
        assert_eq!(s.count, 0);
    }

    // --- Integrator tests ---

    #[test]
    fn integrator_empty() {
        let i = Integrator::new();
        assert_eq!(i.value(), 0.0);
        assert_eq!(i.count, 0);
        assert_eq!(i.overload_gaps, 0);
    }

    #[test]
    fn integrator_single_sample() {
        let mut i = Integrator::new();
        i.push(5.0, Instant::now());
        // Single sample: no interval to integrate, value stays 0.
        assert_eq!(i.value(), 0.0);
        assert_eq!(i.count, 1);
    }

    #[test]
    fn integrator_constant() {
        let mut i = Integrator::new();
        let t0 = Instant::now();
        i.push(2.0, t0);
        i.push(2.0, t0 + Duration::from_secs(1));
        // Constant 2.0 over 1 second = 2.0 unit·s
        assert!((i.value() - 2.0).abs() < 1e-9);
        assert_eq!(i.count, 2);
    }

    #[test]
    fn integrator_trapezoidal() {
        let mut i = Integrator::new();
        let t0 = Instant::now();
        i.push(1.0, t0);
        i.push(3.0, t0 + Duration::from_secs(1));
        // Trapezoid: (1 + 3) / 2 * 1 = 2.0
        assert!((i.value() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn integrator_multi_step() {
        let mut i = Integrator::new();
        let t0 = Instant::now();
        i.push(0.0, t0);
        i.push(2.0, t0 + Duration::from_millis(500));
        i.push(2.0, t0 + Duration::from_millis(1000));
        // Step 1: (0 + 2) / 2 * 0.5 = 0.5
        // Step 2: (2 + 2) / 2 * 0.5 = 1.0
        // Total: 1.5
        assert!((i.value() - 1.5).abs() < 1e-9);
        assert_eq!(i.count, 3);
    }

    #[test]
    fn integrator_reset() {
        let mut i = Integrator::with_max_dt(5.0);
        let t0 = Instant::now();
        i.push(1.0, t0);
        i.push(1.0, t0 + Duration::from_secs(1));
        i.push_overload();
        i.reset();
        assert_eq!(i.value(), 0.0);
        assert_eq!(i.count, 0);
        assert_eq!(i.overload_gaps, 0);
        assert_eq!(i.skipped_intervals, 0);
        // max_dt_secs should be preserved
        assert_eq!(i.max_dt_secs, 5.0);
    }

    #[test]
    fn integrator_overload_gap() {
        let mut i = Integrator::new();
        let t0 = Instant::now();
        i.push(1.0, t0);
        i.push(1.0, t0 + Duration::from_secs(1));
        // Integral so far: 1.0
        let before = i.value();

        i.push_overload();
        assert_eq!(i.overload_gaps, 1);

        // Next normal sample starts fresh (no prev), so no area added.
        i.push(1.0, t0 + Duration::from_secs(3));
        assert!((i.value() - before).abs() < 1e-9);

        // Now a second normal sample: resumes integration.
        i.push(1.0, t0 + Duration::from_secs(4));
        // Added: (1 + 1) / 2 * 1 = 1.0
        assert!((i.value() - 2.0).abs() < 1e-9);
        assert_eq!(i.count, 4); // 4 normal pushes total
    }

    #[test]
    fn integrator_max_dt_skip() {
        let mut i = Integrator::with_max_dt(1.0);
        let t0 = Instant::now();
        i.push(10.0, t0);
        // Gap of 5 seconds > max_dt of 1 second → skipped
        i.push(10.0, t0 + Duration::from_secs(5));
        assert_eq!(i.value(), 0.0);
        assert_eq!(i.skipped_intervals, 1);

        // Normal interval within max_dt
        i.push(10.0, t0 + Duration::from_millis(5500));
        // (10 + 10) / 2 * 0.5 = 5.0
        assert!((i.value() - 5.0).abs() < 1e-9);
        assert_eq!(i.skipped_intervals, 1);
    }

    #[test]
    fn integrator_skipped_intervals_counts_every_oversize_gap() {
        let mut i = Integrator::with_max_dt(1.0);
        let t0 = Instant::now();
        i.push(10.0, t0);
        i.push(10.0, t0 + Duration::from_secs(5));
        i.push(10.0, t0 + Duration::from_secs(10));
        i.push(10.0, t0 + Duration::from_secs(15));
        assert_eq!(i.skipped_intervals, 3);
        assert_eq!(i.value(), 0.0);
    }

    #[test]
    fn integrator_clock_backward() {
        // checked_duration_since returns None if clock goes backward.
        // Integrator should silently skip that interval.
        let mut i = Integrator::new();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(1);
        i.push(1.0, t1); // "later" time first
        i.push(1.0, t0); // "earlier" time second → backward
        // No area should be added (checked_duration_since returns None).
        assert_eq!(i.value(), 0.0);
        assert_eq!(i.count, 2);
    }

    // --- integral_unit_info tests ---

    #[test]
    fn integral_unit_info_current() {
        assert_eq!(integral_unit_info("A"), Some(("Ah", 3600.0)));
        assert_eq!(integral_unit_info("mA"), Some(("mAh", 3600.0)));
        assert_eq!(integral_unit_info("µA"), Some(("µAh", 3600.0)));
    }

    #[test]
    fn integral_unit_info_voltage() {
        let (unit, div) = integral_unit_info("V").unwrap();
        assert_eq!(unit, "V\u{00b7}s");
        assert_eq!(div, 1.0);
        let (unit, div) = integral_unit_info("mV").unwrap();
        assert_eq!(unit, "mV\u{00b7}s");
        assert_eq!(div, 1.0);
    }

    #[test]
    fn integral_unit_info_none() {
        assert!(integral_unit_info("Ω").is_none());
        assert!(integral_unit_info("kΩ").is_none());
        assert!(integral_unit_info("nF").is_none());
        assert!(integral_unit_info("µF").is_none());
        assert!(integral_unit_info("Hz").is_none());
        assert!(integral_unit_info("kHz").is_none());
        assert!(integral_unit_info("°C").is_none());
        assert!(integral_unit_info("°F").is_none());
        assert!(integral_unit_info("%").is_none());
        assert!(integral_unit_info("").is_none());
    }
    // --- SeriesStats tests ---

    fn reading(mode: &'static str, unit: &'static str, value: f64, t: Instant) -> Measurement {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(value),
            unit,
            crate::flags::StatusFlags::default(),
        );
        m.mode = mode.into();
        m.timestamp = t;
        m
    }

    #[test]
    fn first_reading_starts_the_series_without_a_change() {
        let mut s = SeriesStats::new(true);
        assert_eq!(s.push(&reading("DC V", "V", 1.0, Instant::now())), None);
        assert_eq!(s.mode(), Some("DC V"));
        assert_eq!(s.unit(), Some("V"));
        assert_eq!(s.stats.count, 1);
    }

    #[test]
    fn unit_change_resets_and_reports() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("DC V", "mV", 100.0, t0));
        s.push(&reading(
            "DC V",
            "mV",
            200.0,
            t0 + Duration::from_millis(100),
        ));
        assert_eq!(s.stats.count, 2);

        let change = s.push(&reading("DC V", "V", 1.0, t0 + Duration::from_millis(200)));
        assert_eq!(
            change,
            Some(SeriesChange::Unit {
                prev: "mV".to_string(),
                new: "V".to_string(),
            })
        );
        // Reset, then the triggering reading itself is counted.
        assert_eq!(s.stats.count, 1);
        assert_eq!(s.stats.min, Some(1.0));
        assert_eq!(s.integrator.count, 1);
        assert_eq!(s.integrator.value(), 0.0);
        assert_eq!(s.unit(), Some("V"));
    }

    /// The reset used to watch the unit alone. `--unit` pins the label, so
    /// a dial turn from volts to ohms kept one series and averaged the two
    /// quantities together in silence. When both moved, name the mode: it is
    /// the change the user made.
    #[test]
    fn mode_change_takes_precedence_over_unit() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(false);
        s.push(&reading("DC V", "V", 1.0, t0));
        let change = s.push(&reading(
            "\u{3a9}",
            "k\u{3a9}",
            4.7,
            t0 + Duration::from_millis(100),
        ));
        assert_eq!(
            change,
            Some(SeriesChange::Mode {
                prev: "DC V".to_string(),
                new: "\u{3a9}".to_string(),
            })
        );
        assert_eq!(s.mode(), Some("\u{3a9}"));
        assert_eq!(s.unit(), Some("k\u{3a9}"));
    }

    #[test]
    fn overload_breaks_the_integral_but_not_the_series() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("DC A", "A", 1.0, t0));

        let mut ol = Measurement::test_fixture(
            MeasuredValue::Overload,
            "A",
            crate::flags::StatusFlags::default(),
        );
        ol.mode = "DC A".into();
        ol.timestamp = t0 + Duration::from_millis(100);
        assert_eq!(s.push(&ol), None);

        assert_eq!(
            s.push(&reading("DC A", "A", 1.0, t0 + Duration::from_millis(200))),
            None
        );
        assert_eq!(s.integrator.overload_gaps, 1);
        // Overload is not a number: it never reaches min/max/avg.
        assert_eq!(s.stats.count, 2);
        assert_eq!(s.unit(), Some("A"));
    }

    #[test]
    fn not_integrating_leaves_the_integrator_untouched() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(false);
        s.push(&reading("DC A", "A", 1.0, t0));
        s.push(&reading("DC A", "A", 1.0, t0 + Duration::from_millis(100)));
        assert_eq!(s.stats.count, 2);
        assert_eq!(s.integrator.count, 0);
        assert_eq!(s.integral_display(), None);
    }

    #[test]
    fn integral_display_scales_by_the_unit() {
        // 7200 mA·s = 2 mAh.
        assert_eq!(integral_display(7200.0, "mA"), Some((2.0, "mAh")));
        assert_eq!(integral_display(1.0, "\u{3a9}"), None);
    }

    #[test]
    fn reset_keeps_the_series() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("DC V", "V", 1.0, t0));
        s.push(&reading("DC V", "V", 3.0, t0 + Duration::from_millis(100)));
        s.reset();
        assert_eq!(s.stats.count, 0);
        assert_eq!(s.integrator.count, 0);
        assert_eq!(s.unit(), Some("V"));

        // Same series as before the reset: no spurious change note.
        assert_eq!(
            s.push(&reading("DC V", "V", 2.0, t0 + Duration::from_millis(200))),
            None
        );
        assert_eq!(s.stats.count, 1);
    }

    /// The CLI prints these verbatim; the wording predates `SeriesChange` and
    /// must survive the move into the library.
    #[test]
    fn series_change_display_matches_the_cli_wording() {
        assert_eq!(
            SeriesChange::Mode {
                prev: "DC V".to_string(),
                new: "Resistance".to_string(),
            }
            .to_string(),
            "Mode changed (DC V \u{2192} Resistance)"
        );
        assert_eq!(
            SeriesChange::Unit {
                prev: "mV".to_string(),
                new: "V".to_string(),
            }
            .to_string(),
            "Unit changed (mV \u{2192} V)"
        );
    }
}
