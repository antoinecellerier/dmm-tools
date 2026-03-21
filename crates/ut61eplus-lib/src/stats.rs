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

        // Normal interval within max_dt
        i.push(10.0, t0 + Duration::from_millis(5500));
        // (10 + 10) / 2 * 0.5 = 5.0
        assert!((i.value() - 5.0).abs() < 1e-9);
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
}
