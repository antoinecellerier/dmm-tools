use crate::measurement::{MeasuredValue, Measurement, decimals_in};
use crate::transform::{PrefixRatio, prefix_power, prefix_ratio, re_express, si_prefix};
use log::debug;
use std::time::{Duration, Instant};

/// Tracks min/max/avg/standard-deviation statistics for a series of
/// measurements.
///
/// Used by both CLI and GUI to accumulate running statistics
/// over measurement values.
#[derive(Debug, Clone)]
pub struct RunningStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub count: u64,
    /// Sums of each value's offset from the first, and of its square, which
    /// keep the mean and variance accurate for a small spread on a large
    /// offset.
    shift: f64,
    shifted_sum: f64,
    shifted_squares: f64,
}

impl RunningStats {
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            count: 0,
            shift: 0.0,
            shifted_sum: 0.0,
            shifted_squares: 0.0,
        }
    }

    /// Record a new value, updating min/max/count and the sums. A value
    /// that is not finite is left out: it has no place in a mean or a spread,
    /// and as the first value, the offset every other is taken from, it would
    /// turn them all to NaN.
    /// Inlined: the GUI calls it per sample over a whole visible window.
    #[inline]
    pub fn push(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.min = Some(self.min.map_or(value, |m: f64| m.min(value)));
        self.max = Some(self.max.map_or(value, |m: f64| m.max(value)));
        if self.count == 0 {
            self.shift = value;
        }
        self.count += 1;
        let d = value - self.shift;
        self.shifted_sum += d;
        self.shifted_squares += d * d;
    }

    /// Return the average, or `None` if no values have been pushed.
    pub fn avg(&self) -> Option<f64> {
        if self.count > 0 {
            Some(self.shift + self.shifted_sum / self.count as f64)
        } else {
            None
        }
    }

    /// Return the sample standard deviation (n − 1), or `None` below two
    /// values.
    pub fn std_dev(&self) -> Option<f64> {
        (self.count >= 2).then(|| {
            let n = self.count as f64;
            let m2 = self.shifted_squares - self.shifted_sum * self.shifted_sum / n;
            (m2.max(0.0) / (n - 1.0)).sqrt()
        })
    }

    /// Every figure in another prefix of the same unit, as when the meter
    /// auto-ranged from mV to V: the sums scale with the values, the sum of
    /// squares with their square, so the mean and spread read the same.
    pub(crate) fn re_expressed(&self, ratio: PrefixRatio) -> Self {
        Self {
            min: self.min.map(|v| ratio.apply(v)),
            max: self.max.map(|v| ratio.apply(v)),
            count: self.count,
            shift: ratio.apply(self.shift),
            shifted_sum: ratio.apply(self.shifted_sum),
            shifted_squares: ratio.apply(ratio.apply(self.shifted_squares)),
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
/// ~20× the typical 10 Hz sample interval, and the floor under
/// [`Integrator::set_sample_interval`].
const DEFAULT_MAX_DT_SECS: f64 = 2.0;

/// How many sample intervals without a reading make a gap: where the GUI's
/// graph breaks its trace, and where the integral stops bridging, so the
/// two agree on what is continuous.
pub const GAP_INTERVALS: f64 = 5.0;

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
    /// A larger limit in force since the last reading, which the interval in
    /// progress is still judged by: it began at the slower sample interval.
    carried_max_dt_secs: f64,
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
            carried_max_dt_secs: 0.0,
            first_time: None,
            last_time: None,
        }
    }

    /// Bridge up to [`GAP_INTERVALS`] sample intervals between readings, and
    /// never less than the 2 s default: at a sample interval of its own, a
    /// streaming meter's readings come a frame early or late, so a fixed
    /// limit near the interval skipped half of them.
    pub fn set_sample_interval(&mut self, interval: Duration) {
        self.carried_max_dt_secs = self.carried_max_dt_secs.max(self.max_dt_secs);
        self.max_dt_secs = (interval.as_secs_f64() * GAP_INTERVALS).max(DEFAULT_MAX_DT_SECS);
    }

    /// The longest interval between readings that is integrated, in seconds.
    pub fn max_dt_secs(&self) -> f64 {
        self.max_dt_secs
    }

    /// Record a normal measurement value, accumulating the trapezoidal area
    /// since the previous sample. A value that is not finite breaks the
    /// integration, as an overload does but without counting as one: once in
    /// the sum, it would stay there until the next reset.
    pub fn push(&mut self, value: f64, timestamp: Instant) {
        if !value.is_finite() {
            self.push_gap();
            return;
        }
        if let Some((prev_val, prev_time)) = self.prev
            && let Some(dt) = timestamp.checked_duration_since(prev_time)
        {
            let dt_secs = dt.as_secs_f64();
            let limit = self.max_dt_secs.max(self.carried_max_dt_secs);
            if dt_secs <= limit {
                self.integral += (prev_val + value) / 2.0 * dt_secs;
            } else {
                // The binaries show the skip count; this only says which gap.
                debug!("integrator: skipping {dt_secs:.2}s interval (max {limit:.2}s)");
                self.skipped_intervals += 1;
            }
        }
        if self.first_time.is_none() {
            self.first_time = Some(timestamp);
        }
        self.last_time = Some(timestamp);
        self.prev = Some((value, timestamp));
        self.carried_max_dt_secs = 0.0;
        self.count += 1;
    }

    /// Record an overload reading. Breaks the integration (clears the previous
    /// sample) so the next normal reading starts a fresh interval.
    pub(crate) fn push_overload(&mut self) {
        self.push_gap();
        self.overload_gaps += 1;
    }

    /// Break the integration without counting an overload: the next normal
    /// reading starts a fresh interval instead of bridging this one with a
    /// trapezoid, which would integrate a value nobody measured. The binaries
    /// call it at a pause, a disconnect and a timeout.
    pub fn push_gap(&mut self) {
        self.prev = None;
    }

    /// Raw accumulated integral in unit·seconds (e.g. A·s or V·s). Within
    /// the crate only: a series' integral is in its anchor's prefix, read
    /// through [`SeriesStats::integral_display`].
    pub(crate) fn value(&self) -> f64 {
        self.integral
    }

    /// Elapsed time in seconds between the first and last finite sample pushed.
    /// Returns `None` if fewer than 2 such samples have been pushed.
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
pub(crate) fn integral_unit_info(unit: &str) -> Option<(&'static str, f64)> {
    // The prefix carries over, whichever one the meter auto-ranged to: nA
    // integrates to nAh, kV to kV·s.
    const UNITS: [(f64, &str, &str); 8] = [
        (1e-12, "pAh", "pV\u{00b7}s"),
        (1e-9, "nAh", "nV\u{00b7}s"),
        (1e-6, "µAh", "µV\u{00b7}s"),
        (1e-3, "mAh", "mV\u{00b7}s"),
        (1.0, "Ah", "V\u{00b7}s"),
        (1e3, "kAh", "kV\u{00b7}s"),
        (1e6, "MAh", "MV\u{00b7}s"),
        (1e9, "GAh", "GV\u{00b7}s"),
    ];
    let (base, mult) = si_prefix(unit);
    let &(_, charge, volt_seconds) = UNITS.iter().find(|(m, _, _)| *m == mult)?;
    match base {
        "A" => Some((charge, 3600.0)),
        "V" => Some((volt_seconds, 1.0)),
        _ => None,
    }
}

/// What the meter showed one series of readings in: each prefix of its unit
/// it used, and the most decimals it showed in that prefix — 0.05 Ω on the
/// UT61E+'s 220 Ω range is two, 1.2341 MΩ four.
///
/// The statistics are written from it ([`ShownDigits::layout`]), so a
/// figure keeps to a range and a resolution the meter used rather than
/// digits it never showed. Keyed by the prefix's power of ten, not by the
/// unit the figures are held in, so an auto-range step that re-expresses
/// those leaves it standing.
#[derive(Debug, Clone, Default)]
pub struct ShownDigits {
    /// The base unit the prefixes go on ("Ω"), set by the first reading.
    base: String,
    /// `(power of ten of the prefix, decimals)`, at most one per prefix.
    seen: Vec<(i32, usize)>,
}

impl ShownDigits {
    /// Note a reading of `value` in `unit` the meter showed as `display`.
    ///
    /// The digits count only when they spell `value`: the meter's text
    /// can be something else (a pattern, a word, segments read as digits
    /// the value was not taken from), and decimals counted from it would
    /// then be made up.
    fn record(&mut self, value: f64, unit: &str, display: &str) {
        // Spaces apart, the UT61E+'s "- 55.79" or a padded " 1.6112" is a
        // number; allocating only for the one with a space inside.
        let trimmed = display.trim();
        let number: std::borrow::Cow<'_, str> = if trimmed.contains(' ') {
            trimmed
                .chars()
                .filter(|c| *c != ' ')
                .collect::<String>()
                .into()
        } else {
            trimmed.into()
        };
        let Ok(shown) = number.parse::<f64>() else {
            return;
        };
        let decimals = decimals_in(&number);
        // Within half the last digit shown: a float-based family's text is
        // its f32 rounded, the value that f32 widened.
        let half_step = 0.5 * 10f64.powi(-(decimals as i32));
        if !shown.is_finite() || (shown - value).abs() > half_step || decimals > 9 {
            return;
        }
        let (base, power) = prefix_power(unit);
        if self.seen.is_empty() {
            base.clone_into(&mut self.base);
        } else if self.base != base {
            return;
        }
        match self.seen.iter_mut().find(|(p, _)| *p == power) {
            Some((_, most)) => *most = (*most).max(decimals),
            None => {
                self.seen.push((power, decimals));
                self.seen.sort_unstable_by_key(|&(p, _)| p);
            }
        }
    }

    fn clear(&mut self) {
        self.seen.clear();
    }

    /// How to write a figure of `value` `unit` as the meter would: the unit
    /// under one of the prefixes it showed, the ratio that re-expresses the
    /// figure in it, and the decimals it showed there. The prefix is the
    /// largest one the figure reaches 1 in, else the smallest: 31.3 MΩ in
    /// MΩ, 0.04 Ω in Ω beside them both.
    ///
    /// `None` when nothing was shown yet, or `unit` is another quantity's
    /// than the one shown (a plotted sub-value's Hz beside a V reading).
    pub fn layout(&self, value: f64, unit: &str) -> Option<(String, PrefixRatio, usize)> {
        let (base, from) = prefix_power(unit);
        if base != self.base {
            return None;
        }
        let (power, decimals) = self
            .seen
            .iter()
            .rev()
            .copied()
            .find(|&(p, _)| value.abs() * 10f64.powi(from - p) >= 1.0)
            .or_else(|| self.seen.first().copied())?;
        let (unit, ratio) = re_express(unit, power)?;
        Some((unit, ratio, decimals))
    }
}

/// Why the accumulators were reset: the reading started a new series.
///
/// Mode *and* unit, because neither check subsumes the other: a `--unit`
/// relabel pins the unit across a dial turn that changes the mode, and a
/// mode string may cover more than one quantity. An auto-range step that
/// only moves the prefix (mV → V) is neither: the series carries on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesChange {
    /// The meter's mode string changed — typically the dial was turned.
    Mode { prev: String, new: String },
    /// The unit changed to another quantity within the same mode, not merely
    /// another prefix of the same one.
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
/// the time integral, reset together whenever the mode or the quantity
/// changes, and carried on when the meter auto-ranges.
///
/// Min/Max/Avg and an integral are only meaningful within a single mode and
/// quantity. Turning the dial changes the mode — which `--unit` would
/// otherwise hide, as it pins the label across the change. Without both
/// checks a run that spans a dial turn averages volts with ohms and prints
/// bare numbers, so nothing on screen would reveal the mix. Auto-range moves
/// the unit a decade without touching the mode string (the UT61E+ shows
/// "219.0 Ω" then "0.2200 kΩ" in one mode). Every value is accumulated in
/// the prefix the series' first reading had, its anchor, so the two decades
/// never mix and a step changes nothing held; the figures are read in the
/// unit the meter shows last. A run without a step is then summed exactly as
/// it arrived, and one with steps depends on their order only by float
/// rounding.
///
/// Owned by the CLI read loop and the GUI message drain alike, so the two
/// cannot drift apart on what starts a new series.
#[derive(Debug, Clone)]
pub struct SeriesStats {
    /// In the anchor's prefix; read through [`SeriesStats::stats`].
    stats: RunningStats,
    /// Its integral is in the anchor's prefix; read it through
    /// [`SeriesStats::integral_display`].
    pub integrator: Integrator,
    integrate: bool,
    /// `(mode, unit)` the current series accumulates in; `None` until the
    /// first reading.
    series: Option<(String, String)>,
    /// The series began on an overload, whose unit may be a decade the meter
    /// names only out of range: the first reading with a value sets it, with
    /// nothing accumulated to reset.
    unit_unread: bool,
    /// The prefixes and decimals the meter showed the series in.
    shown: ShownDigits,
    /// The unit the figures are accumulated in: the first value's since the
    /// series began or was reset, so a series that began on an overload in a
    /// decade the meter only names out of range takes its readings' own.
    anchor: Option<String>,
}

impl SeriesStats {
    /// Create an empty session.
    ///
    /// With `integrate == false` the integrator is never touched: pushing into
    /// it anyway would log its skipped-interval debug line for a feature the
    /// user never asked for (the CLI only integrates under `--integrate`).
    pub fn new(integrate: bool) -> Self {
        Self {
            stats: RunningStats::new(),
            integrator: Integrator::new(),
            integrate,
            series: None,
            unit_unread: false,
            shown: ShownDigits::default(),
            anchor: None,
        }
    }

    /// Record one reading, returning the [`SeriesChange`] that reset the
    /// accumulators, or `None` while the reading continues the current series.
    ///
    /// The first reading only stores the series — it starts one rather than
    /// changing it. When both mode and unit moved, the mode is reported: it
    /// names the change the user made.
    ///
    /// A reading in another prefix of the same unit (mV → V, Ω → kΩ) is no
    /// change: min/max/avg and the integral carry on, read in its prefix,
    /// and [`SeriesStats::unit`] follows it.
    ///
    /// An overload's unit never moves the series: it may be a decade the
    /// meter only names while out of range ("OL MΩ" between kΩ readings).
    /// Its mode does — a dial turn with the probes open starts the new
    /// series there, as the graph's trace does.
    ///
    /// A [`MeasuredValue::NoReading`] neither continues nor starts a series:
    /// its mode can be the word the meter shows ("Auto" while the probes are
    /// lifted), so treating it as one would reset MIN/MAX/AVG on every lift.
    /// It leaves the series and min/max/avg alone and only breaks the
    /// integration interval, as an overload does: the time the meter
    /// measured nothing has no value to integrate, and bridging it with a
    /// trapezoid between the readings either side would make one up.
    ///
    /// A [`MeasuredValue::Absent`] frame is skipped outright: the main
    /// reading went on in frames of its own either side of it, so the series,
    /// the figures and the integration interval all carry on across it.
    pub fn push(&mut self, m: &Measurement) -> Option<SeriesChange> {
        if let MeasuredValue::Absent = m.value {
            return None;
        }
        if let MeasuredValue::NoReading(_) = m.value {
            if self.integrate {
                self.integrator.push_gap();
            }
            return None;
        }
        let overload = matches!(m.value, MeasuredValue::Overload);
        // Only stored when it moves: an unchanged series would otherwise
        // allocate two strings for every sample of the run.
        let change = match self.series.as_mut() {
            None => {
                self.series = Some((m.mode.to_string(), m.unit.to_string()));
                self.unit_unread = overload;
                None
            }
            Some((prev_mode, _)) if prev_mode.as_str() != m.mode.as_ref() => {
                Some(SeriesChange::Mode {
                    prev: prev_mode.clone(),
                    new: m.mode.to_string(),
                })
            }
            Some(_) if overload => None,
            Some((_, prev_unit)) if self.unit_unread => {
                *prev_unit = m.unit.to_string();
                None
            }
            Some((_, prev_unit)) if prev_unit.as_str() != m.unit.as_ref() => {
                match prefix_ratio(prev_unit, &m.unit) {
                    Some(_) => {
                        *prev_unit = m.unit.to_string();
                        None
                    }
                    None => Some(SeriesChange::Unit {
                        prev: prev_unit.clone(),
                        new: m.unit.to_string(),
                    }),
                }
            }
            Some(_) => None,
        };
        if change.is_some() {
            self.stats.reset();
            self.integrator.reset();
            self.shown.clear();
            self.anchor = None;
            self.series = Some((m.mode.to_string(), m.unit.to_string()));
            self.unit_unread = overload;
        } else if !overload {
            self.unit_unread = false;
        }

        match &m.value {
            MeasuredValue::Normal(v) => {
                let anchor = self.anchor.get_or_insert_with(|| m.unit.to_string());
                // The series carried on, so the two are one unit's prefixes.
                let held = if m.unit.as_ref() == anchor.as_str() {
                    *v
                } else {
                    let ratio = prefix_ratio(&m.unit, anchor);
                    debug_assert!(ratio.is_some(), "{} beside {anchor}", m.unit);
                    ratio.map_or(*v, |r| r.apply(*v))
                };
                self.stats.push(held);
                if let Some(display) = m.display_raw.as_deref() {
                    self.shown.record(*v, &m.unit, display);
                }
                if self.integrate {
                    self.integrator.push(held, m.timestamp);
                }
            }
            // Breaks the integration interval; contributes nothing to min/max.
            MeasuredValue::Overload => {
                if self.integrate {
                    self.integrator.push_overload();
                }
            }
            // A detection level is not a measured quantity — neither
            // accumulator has anything to do with it. A no-reading word and a
            // frame without a main reading returned above and never get here.
            MeasuredValue::NcvLevel(_) | MeasuredValue::NoReading(_) | MeasuredValue::Absent => {}
        }
        change
    }

    /// Clear both accumulators, keeping the current series: the CLI's Ctrl+L
    /// and the GUI's Reset button clear the numbers, they don't forget what is
    /// being measured.
    pub fn reset(&mut self) {
        self.stats.reset();
        self.integrator.reset();
        self.shown.clear();
        self.anchor = None;
    }

    /// Min/max/avg and the spread, in [`SeriesStats::unit`].
    pub fn stats(&self) -> RunningStats {
        match self.anchor_ratio() {
            Some(ratio) => self.stats.re_expressed(ratio),
            None => self.stats.clone(),
        }
    }

    /// The ratio from the anchor to the unit the figures are read in, when
    /// the meter has stepped away from it.
    fn anchor_ratio(&self) -> Option<PrefixRatio> {
        let anchor = self.anchor.as_deref()?;
        let unit = self.unit()?;
        (anchor != unit)
            .then(|| prefix_ratio(anchor, unit))
            .flatten()
    }

    /// The prefixes and decimals the meter showed the series in, which the
    /// binaries write its figures with.
    pub fn shown(&self) -> &ShownDigits {
        &self.shown
    }

    /// Mode the accumulated figures were measured in, or `None` before the
    /// first reading.
    pub fn mode(&self) -> Option<&str> {
        self.series.as_ref().map(|(mode, _)| mode.as_str())
    }

    /// Unit the figures are read in — the latest reading's prefix, after an
    /// auto-range step — or `None` before the first reading.
    pub fn unit(&self) -> Option<&str> {
        self.series.as_ref().map(|(_, unit)| unit.as_str())
    }

    /// The integral scaled to its display unit, or `None` when this session
    /// does not integrate, no reading has arrived yet, or the current unit has
    /// no meaningful time-integral (see [`integral_display`]).
    pub fn integral_display(&self) -> Option<(f64, &'static str)> {
        if !self.integrate {
            return None;
        }
        let raw = self.integrator.value();
        let raw = self.anchor_ratio().map_or(raw, |ratio| ratio.apply(raw));
        integral_display(raw, self.unit()?)
    }
}

/// Raw unit·seconds integral scaled to its display unit:
/// `(value / divisor, display_unit)`.
///
/// `None` for units where integration is not meaningful (Ω, F, Hz, °C, %).
pub fn integral_display(raw: f64, unit: &str) -> Option<(f64, &'static str)> {
    integral_unit_info(unit).map(|(display_unit, divisor)| (raw / divisor, display_unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// An integrator with a custom maximum interval threshold.
    fn with_max_dt(max_dt_secs: f64) -> Integrator {
        Integrator {
            max_dt_secs,
            ..Integrator::new()
        }
    }

    #[test]
    fn empty_stats() {
        let s = RunningStats::new();
        assert!(s.min.is_none());
        assert!(s.max.is_none());
        assert!(s.avg().is_none());
        assert_eq!(s.count, 0);
    }

    #[test]
    fn std_dev_needs_two_values() {
        let mut s = RunningStats::new();
        assert!(s.std_dev().is_none());
        s.push(5.0);
        assert!(s.std_dev().is_none());
        s.push(5.0);
        assert_eq!(s.std_dev(), Some(0.0));
    }

    #[test]
    fn std_dev_is_the_sample_deviation() {
        let mut s = RunningStats::new();
        for v in [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0] {
            s.push(v);
        }
        // Sum of squared deviations from the mean 5 is 32, over n − 1 = 7.
        let expected = (32.0f64 / 7.0).sqrt();
        assert!((s.std_dev().unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn std_dev_holds_on_a_large_offset() {
        // A 5 V rail moving in the fifth digit.
        let mut s = RunningStats::new();
        for i in 0..10_000 {
            s.push(5.0 + if i % 2 == 0 { 1e-5 } else { -1e-5 });
        }
        let sd = s.std_dev().unwrap();
        assert!((sd - 1e-5).abs() < 1e-9, "{sd}");
    }

    #[test]
    fn non_finite_values_are_left_out() {
        let mut s = RunningStats::new();
        s.push(f64::INFINITY);
        s.push(f64::NAN);
        s.push(1.0);
        s.push(3.0);
        assert_eq!(s.count, 2);
        assert_eq!(s.avg(), Some(2.0));
        assert!((s.std_dev().unwrap() - 2f64.sqrt()).abs() < 1e-12);
    }

    /// A run in mV re-expressed in V holds the figures a run read in V from
    /// the start would.
    #[test]
    fn re_expressed_figures_match_a_run_in_the_other_prefix() {
        let millivolts = [898.7, 901.2, 899.9, 900.4, 1100.3];
        let mut stepped = RunningStats::new();
        for v in millivolts {
            stepped.push(v);
        }
        let mut stepped = stepped.re_expressed(crate::transform::prefix_ratio("mV", "V").unwrap());
        stepped.push(1.0996);
        let mut volts = RunningStats::new();
        for v in millivolts.iter().map(|v| v / 1000.0).chain([1.0996]) {
            volts.push(v);
        }
        assert_eq!(stepped.count, 6);
        assert_eq!(stepped.min, volts.min);
        assert_eq!(stepped.max, volts.max);
        let close = |a: Option<f64>, b: Option<f64>| (a.unwrap() - b.unwrap()).abs() < 1e-12;
        assert!(close(stepped.avg(), volts.avg()));
        assert!(close(stepped.std_dev(), volts.std_dev()));
    }

    #[test]
    fn re_expressing_empty_stats_leaves_them_empty() {
        let s =
            RunningStats::new().re_expressed(crate::transform::prefix_ratio("Ω", "kΩ").unwrap());
        assert_eq!(s.count, 0);
        assert!(s.min.is_none() && s.avg().is_none() && s.std_dev().is_none());
    }

    #[test]
    fn reset_clears_std_dev() {
        let mut s = RunningStats::new();
        s.push(1.0);
        s.push(3.0);
        s.reset();
        assert!(s.std_dev().is_none());
        s.push(10.0);
        s.push(10.0);
        assert_eq!(s.std_dev(), Some(0.0));
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
        let mut i = with_max_dt(5.0);
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
    fn integrator_breaks_at_a_value_that_is_not_finite() {
        for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let mut i = Integrator::new();
            let t0 = Instant::now();
            i.push(1.0, t0);
            i.push(1.0, t0 + Duration::from_secs(1));

            i.push(bad, t0 + Duration::from_secs(2));
            assert_eq!(i.value(), 1.0, "{bad}");
            assert_eq!(i.overload_gaps, 0, "{bad}");
            assert_eq!(i.count, 2, "{bad}");

            // The next normal sample starts fresh, bridging nothing.
            i.push(1.0, t0 + Duration::from_secs(3));
            assert_eq!(i.value(), 1.0, "{bad}");
            i.push(1.0, t0 + Duration::from_secs(4));
            assert_eq!(i.value(), 2.0, "{bad}");
        }
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

    /// The limit follows the sample interval, never below 2 s, and a
    /// shorter interval leaves the interval in progress to the longer one.
    #[test]
    fn the_limit_follows_the_sample_interval() {
        let t0 = Instant::now();
        let at = |s: f64| t0 + Duration::from_secs_f64(s);
        let mut i = Integrator::new();
        i.set_sample_interval(Duration::from_millis(100));
        assert_eq!(i.max_dt_secs(), 2.0);
        i.set_sample_interval(Duration::from_secs(2));
        assert_eq!(i.max_dt_secs(), 10.0);
        i.push(1.0, at(0.0));
        i.push(1.0, at(2.3));
        assert_eq!(i.skipped_intervals, 0, "a frame late at 2 s is no gap");
        i.set_sample_interval(Duration::from_millis(100));
        i.push(1.0, at(4.5));
        assert_eq!(i.skipped_intervals, 0, "the stretch across the change");
        i.push(1.0, at(7.0));
        assert_eq!(i.skipped_intervals, 1, "2.5 s at 100 ms is a gap");
    }

    #[test]
    fn integrator_max_dt_skip() {
        let mut i = with_max_dt(1.0);
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
        let mut i = with_max_dt(1.0);
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
        assert_eq!(integral_unit_info("uA"), Some(("µAh", 3600.0)));
        assert_eq!(integral_unit_info("nA"), Some(("nAh", 3600.0)));
    }

    #[test]
    fn integral_unit_info_voltage() {
        let (unit, div) = integral_unit_info("V").unwrap();
        assert_eq!(unit, "V\u{00b7}s");
        assert_eq!(div, 1.0);
        let (unit, div) = integral_unit_info("mV").unwrap();
        assert_eq!(unit, "mV\u{00b7}s");
        assert_eq!(div, 1.0);
        assert_eq!(integral_unit_info("kV"), Some(("kV\u{00b7}s", 1.0)));
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
        assert!(integral_unit_info("mVA").is_none());
        assert!(integral_unit_info("ms").is_none());
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
        assert_eq!(s.stats().count, 1);
    }

    /// A mode whose readings move to another quantity, as a frequency mode
    /// showing the duty cycle does, starts a new series.
    #[test]
    fn a_change_of_quantity_resets_and_reports() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("Hz", "Hz", 50.0, t0));
        s.push(&reading("Hz", "Hz", 50.1, t0 + Duration::from_millis(100)));
        assert_eq!(s.stats().count, 2);

        let change = s.push(&reading("Hz", "%", 49.9, t0 + Duration::from_millis(200)));
        assert_eq!(
            change,
            Some(SeriesChange::Unit {
                prev: "Hz".to_string(),
                new: "%".to_string(),
            })
        );
        // Reset, then the triggering reading itself is counted.
        assert_eq!(s.stats().count, 1);
        assert_eq!(s.stats().min, Some(49.9));
        assert_eq!(s.integrator.count, 1);
        assert_eq!(s.integrator.value(), 0.0);
        assert_eq!(s.unit(), Some("%"));
    }

    /// An auto-range step from mV to V carries the figures and the integral
    /// on, re-expressed in volts: no change to report.
    #[test]
    fn a_decade_step_carries_the_series_on() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("DC V", "mV", 900.0, t0));
        s.push(&reading("DC V", "mV", 900.0, t0 + Duration::from_secs(1)));
        assert_eq!(
            s.push(&reading("DC V", "V", 1.1, t0 + Duration::from_secs(2))),
            None
        );
        assert_eq!(s.unit(), Some("V"));
        assert_eq!(s.stats().count, 3);
        assert_eq!(s.stats().min, Some(0.9));
        assert_eq!(s.stats().max, Some(1.1));
        assert!((s.stats().avg().unwrap() - 2.9 / 3.0).abs() < 1e-12);
        // 0.9 V·s over the first second, then (0.9 + 1.1) / 2 over the
        // second, which straddles the step.
        assert_eq!(s.integrator.count, 3);
        let (value, unit) = s.integral_display().unwrap();
        assert!((value - 1.9).abs() < 1e-12);
        assert_eq!(unit, "V\u{00b7}s");
    }

    /// A reading flickering across a decade, mV and V in turn, is held as
    /// the same readings in mV are, to the bit: a step changes nothing held.
    #[test]
    fn a_reading_flickering_across_a_decade_holds_as_one_prefix() {
        let t0 = Instant::now();
        let at = |i: u64| t0 + Duration::from_millis(500 * i);
        let mut flicker = SeriesStats::new(true);
        let mut millivolts = SeriesStats::new(true);
        for i in 0..400 {
            // 1.5 V is 1500 mV exactly, so nothing but the steps could part them.
            let (shown, unit, mv) = if i % 2 == 0 {
                (999.5, "mV", 999.5)
            } else {
                (1.5, "V", 1500.0)
            };
            assert_eq!(flicker.push(&reading("DC V", unit, shown, at(i))), None);
            millivolts.push(&reading("DC V", "mV", mv, at(i)));
        }
        let held = |s: &SeriesStats| {
            let r = &s.stats;
            let bits = [
                r.shift,
                r.shifted_sum,
                r.shifted_squares,
                s.integrator.value(),
            ];
            (r.count, r.min, r.max, bits.map(f64::to_bits))
        };
        assert_eq!(held(&flicker), held(&millivolts));
        assert_eq!(flicker.unit(), Some("V"));
        // Read in V: 1.5 V and 0.9995 V, and (1.5 + 0.9995) / 2 V over each
        // half second of the 199.5 s.
        let stats = flicker.stats();
        assert_eq!((stats.min, stats.max), (Some(0.9995), Some(1.5)));
        let (integral, unit) = flicker.integral_display().unwrap();
        assert_eq!(unit, "V\u{00b7}s");
        assert!((integral - 1.24975 * 199.5).abs() < 1e-9, "{integral}");
    }

    /// A step that comes after a gap, an overload between amps and
    /// milliamps: the integral holds through both and goes on in mA.
    #[test]
    fn a_step_after_a_gap_keeps_the_integral() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("DC A", "A", 2.0, t0));
        s.push(&reading("DC A", "A", 2.0, t0 + Duration::from_secs(1)));
        let mut ol = reading("DC A", "A", 0.0, t0 + Duration::from_secs(2));
        ol.value = MeasuredValue::Overload;
        s.push(&ol);
        s.push(&reading("DC A", "mA", 1500.0, t0 + Duration::from_secs(3)));
        s.push(&reading("DC A", "mA", 1500.0, t0 + Duration::from_secs(4)));
        // 2 A·s, then nothing across the overload, then 1.5 A·s.
        let (integral, unit) = s.integral_display().unwrap();
        assert_eq!(unit, "mAh");
        assert!((integral - 3500.0 / 3600.0).abs() < 1e-12, "{integral}");
    }

    /// Without a step the figures are summed as the readings came, to the
    /// bit: from the first reading with a value, after an overload in
    /// another decade, and again after a reset.
    #[test]
    fn a_run_without_a_step_is_summed_as_it_came() {
        let t0 = Instant::now();
        let at = |i: u64| t0 + Duration::from_millis(100 * i);
        let kohms = [219.97, 219.93, 220.04, 219.99];
        let mut plain = SeriesStats::new(true);
        for (i, v) in kohms.iter().enumerate() {
            plain.push(&reading("\u{3a9}", "k\u{3a9}", *v, at(i as u64)));
        }
        let mut s = SeriesStats::new(true);
        let mut ol = reading("\u{3a9}", "M\u{3a9}", 0.0, t0);
        ol.value = MeasuredValue::Overload;
        s.push(&ol);
        for (i, v) in kohms.iter().enumerate() {
            s.push(&reading("\u{3a9}", "k\u{3a9}", *v, at(i as u64)));
        }
        assert_eq!(s.anchor.as_deref(), Some("k\u{3a9}"));
        let same = |a: &SeriesStats, b: &SeriesStats| {
            let (x, y) = (a.stats(), b.stats());
            assert_eq!((x.count, x.min, x.max), (y.count, y.min, y.max));
            assert_eq!(x.avg().map(f64::to_bits), y.avg().map(f64::to_bits));
            assert_eq!(x.std_dev().map(f64::to_bits), y.std_dev().map(f64::to_bits));
        };
        same(&s, &plain);
        // Anchored in MΩ, reset, then readings in kΩ: those anchor.
        let mut s = SeriesStats::new(true);
        s.push(&reading("\u{3a9}", "M\u{3a9}", 1.2, at(5)));
        s.reset();
        assert_eq!(s.anchor, None);
        for (i, v) in kohms.iter().enumerate() {
            s.push(&reading("\u{3a9}", "k\u{3a9}", *v, at(i as u64 + 6)));
        }
        same(&s, &plain);
    }

    /// Every prefixed unit steps as voltage does, and what was shown in each
    /// prefix is kept for the figures.
    #[test]
    fn a_step_in_any_prefixed_unit_carries_the_series_on() {
        let t0 = Instant::now();
        for (mode, low, high) in [
            ("L", "mH", "H"),
            ("Power Wh", "mWh", "Wh"),
            ("Power Ah", "mAh", "Ah"),
            ("Solar", "mW/m²", "W/m²"),
            ("Angle", "m°", "°"),
        ] {
            let mut s = SeriesStats::new(false);
            let mut before = reading(mode, low, 900.0, t0);
            before.display_raw = Some("900.0".to_string());
            let mut after = reading(mode, high, 1.1, t0);
            after.display_raw = Some("1.100".to_string());
            s.push(&before);
            assert_eq!(s.push(&after), None, "{low} to {high}");
            assert_eq!(s.unit(), Some(high));
            assert_eq!(s.stats().count, 2);
            assert_eq!(s.stats().min, Some(0.9));
            assert_eq!(s.stats().max, Some(1.1));
            let (unit, _, decimals) = s.shown().layout(0.9, high).unwrap();
            assert_eq!((unit.as_str(), decimals), (low, 1));
            let (unit, _, decimals) = s.shown().layout(1.1, high).unwrap();
            assert_eq!((unit.as_str(), decimals), (high, 3));
        }
    }

    /// The UT61E+ reports mode "Ohm" as both "219.0 Ω" and "0.2200 kΩ".
    /// Across that boundary the figures hold one scale, whichever decade
    /// each reading came in: never a max three decades out.
    #[test]
    fn readings_either_side_of_a_decade_never_mix() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(false);
        for (i, (unit, value)) in [
            ("\u{3a9}", 219.0),
            ("k\u{3a9}", 0.2200),
            ("k\u{3a9}", 0.2210),
            ("\u{3a9}", 219.5),
        ]
        .into_iter()
        .enumerate()
        {
            let at = t0 + Duration::from_millis(100 * i as u64);
            assert_eq!(
                s.push(&reading("Ohm", unit, value, at)),
                None,
                "{value} {unit}"
            );
        }
        assert_eq!(s.unit(), Some("\u{3a9}"));
        assert_eq!(s.stats().count, 4);
        let close = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-9;
        assert!(close(s.stats().min, 219.0), "{:?}", s.stats().min);
        assert!(close(s.stats().max, 221.0), "{:?}", s.stats().max);
        assert!(close(s.stats().avg(), 219.875), "{:?}", s.stats().avg());
    }

    /// An overload names whatever decade the meter shows while out of range;
    /// it moves neither the prefix nor, after a dial turn, the series: the
    /// figures stay in the unit of the readings behind them.
    #[test]
    fn an_overloads_unit_leaves_the_prefix_and_its_mode_starts_a_series() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("Ohm", "k\u{3a9}", 1.05, t0));
        let overload = |mode: &'static str, unit: &'static str, ms: u64| {
            let mut ol = Measurement::test_fixture(
                MeasuredValue::Overload,
                unit,
                crate::flags::StatusFlags::default(),
            );
            ol.mode = mode.into();
            ol.timestamp = t0 + Duration::from_millis(ms);
            ol
        };
        assert_eq!(s.push(&overload("Ohm", "M\u{3a9}", 100)), None);
        assert_eq!(s.unit(), Some("k\u{3a9}"));
        assert_eq!(
            s.push(&reading(
                "Ohm",
                "k\u{3a9}",
                1.06,
                t0 + Duration::from_millis(200)
            )),
            None
        );
        assert_eq!(s.stats().count, 2);
        assert_eq!(s.stats().max, Some(1.06));
        assert_eq!(s.integrator.overload_gaps, 1);

        // The dial turned to DC V with the probes open: the Ω figures are
        // done with, as the graph's Ω trace is.
        assert!(matches!(
            s.push(&overload("DC V", "V", 300)),
            Some(SeriesChange::Mode { .. })
        ));
        assert_eq!(s.mode(), Some("DC V"));
        assert_eq!(s.stats().count, 0);
        assert_eq!(
            s.push(&reading("DC V", "V", 1.5, t0 + Duration::from_millis(400))),
            None
        );
        assert_eq!(s.stats().count, 1);

        // An overload's unit that is no prefix of the reading's (none at
        // all) is taken from the first reading, not reported as a change.
        assert!(matches!(
            s.push(&overload("Ohm", "", 500)),
            Some(SeriesChange::Mode { .. })
        ));
        assert_eq!(
            s.push(&reading(
                "Ohm",
                "\u{3a9}",
                220.0,
                t0 + Duration::from_millis(600)
            )),
            None
        );
        assert_eq!(s.unit(), Some("\u{3a9}"));
        assert_eq!(s.stats().count, 1);
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
        assert_eq!(s.stats().count, 2);
        assert_eq!(s.unit(), Some("A"));
    }

    /// A ZOTEK meter shows "Auto" under a mode of that name whenever the
    /// probes are lifted. Taken as a series, every lift would reset MIN/MAX/AVG.
    #[test]
    fn a_no_reading_neither_continues_nor_starts_a_series() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        assert_eq!(s.push(&reading("DC V", "V", 1.0, t0)), None);

        let mut idle = Measurement::test_fixture(
            MeasuredValue::NoReading("Auto"),
            "",
            crate::flags::StatusFlags::default(),
        );
        idle.mode = "Auto".into();
        idle.timestamp = t0 + Duration::from_millis(100);
        assert_eq!(s.push(&idle), None);
        assert_eq!(s.mode(), Some("DC V"));
        assert_eq!(s.unit(), Some("V"));

        assert_eq!(
            s.push(&reading("DC V", "V", 3.0, t0 + Duration::from_millis(200))),
            None
        );
        assert_eq!(s.stats().count, 2);
        assert_eq!(s.stats().min, Some(1.0));
        assert_eq!(s.stats().max, Some(3.0));
        assert_eq!(s.stats().avg(), Some(2.0));
        // The stretch with no reading is not bridged: nothing is integrated
        // across it, and it is not counted as an overload.
        assert_eq!(s.integrator.count, 2);
        assert_eq!(s.integrator.value(), 0.0);
        assert_eq!(s.integrator.overload_gaps, 0);
    }

    /// A UT61E+ in AC+DC V sends its DC and AC components in turn, the AC
    /// frames without a main reading. Those frames are not a break: the DC
    /// series, its figures and the integral carry on across them.
    #[test]
    fn a_frame_without_a_main_reading_is_skipped_outright() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(true);
        s.push(&reading("AC+DC V", "V", 1.0, t0));

        let mut ac = Measurement::test_fixture(
            MeasuredValue::Absent,
            "V",
            crate::flags::StatusFlags::default(),
        );
        ac.mode = "AC+DC V".into();
        ac.timestamp = t0 + Duration::from_millis(500);
        assert_eq!(s.push(&ac), None);

        s.push(&reading(
            "AC+DC V",
            "V",
            3.0,
            t0 + Duration::from_millis(1000),
        ));
        assert_eq!(s.stats().count, 2);
        assert_eq!(s.stats().avg(), Some(2.0));
        // Bridged DC to DC: one trapezoid over the whole second.
        assert_eq!(s.integrator.count, 2);
        assert!((s.integrator.value() - 2.0).abs() < 1e-9);
        assert_eq!(s.integrator.overload_gaps, 0);
    }

    /// A no-reading word ahead of the first reading leaves the series for
    /// that reading to start.
    #[test]
    fn a_no_reading_before_the_first_reading_starts_nothing() {
        let mut s = SeriesStats::new(false);
        let mut idle = Measurement::test_fixture(
            MeasuredValue::NoReading("Auto"),
            "",
            crate::flags::StatusFlags::default(),
        );
        idle.mode = "Auto".into();
        assert_eq!(s.push(&idle), None);
        assert_eq!(s.mode(), None);
        assert_eq!(s.push(&reading("DC V", "V", 1.0, Instant::now())), None);
        assert_eq!(s.mode(), Some("DC V"));
    }

    #[test]
    fn not_integrating_leaves_the_integrator_untouched() {
        let t0 = Instant::now();
        let mut s = SeriesStats::new(false);
        s.push(&reading("DC A", "A", 1.0, t0));
        s.push(&reading("DC A", "A", 1.0, t0 + Duration::from_millis(100)));
        assert_eq!(s.stats().count, 2);
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
        assert_eq!(s.stats().count, 0);
        assert_eq!(s.integrator.count, 0);
        assert_eq!(s.unit(), Some("V"));

        // Same series as before the reset: no spurious change note.
        assert_eq!(
            s.push(&reading("DC V", "V", 2.0, t0 + Duration::from_millis(200))),
            None
        );
        assert_eq!(s.stats().count, 1);
    }

    /// A reading the meter showed as `display`, its value read off it.
    fn shown(mode: &'static str, unit: &'static str, display: &str) -> Measurement {
        let value = display.replace(' ', "").parse().unwrap();
        let mut m = reading(mode, unit, value, Instant::now());
        m.display_raw = Some(display.to_string());
        m
    }

    /// The unit and decimals `value` `unit` is written with, if any.
    fn written(s: &SeriesStats, value: f64, unit: &str) -> Option<(String, usize)> {
        s.shown()
            .layout(value, unit)
            .map(|(unit, _, decimals)| (unit, decimals))
    }

    /// Each prefix the meter used, to the most decimals it showed there:
    /// the largest a figure reaches 1 in, else the smallest.
    #[test]
    fn the_meters_prefixes_and_decimals_are_kept() {
        let mut s = SeriesStats::new(false);
        for (unit, display) in [
            ("M\u{3a9}", " 31.300"),
            ("k\u{3a9}", " 5.6351"),
            // A float-based family's text drops a trailing zero: the most
            // decimals shown in a prefix stand.
            ("\u{3a9}", " 0.1"),
            ("\u{3a9}", "- 0.05"),
        ] {
            s.push(&shown("\u{3a9}", unit, display));
        }
        let ohms = |v: f64| written(&s, v, "\u{3a9}");
        let at = |unit: &str, decimals| Some((unit.to_string(), decimals));
        assert_eq!(ohms(0.04), at("\u{3a9}", 2));
        assert_eq!(ohms(999.9), at("\u{3a9}", 2));
        assert_eq!(ohms(1_000.0), at("k\u{3a9}", 4));
        assert_eq!(ohms(31.3e6), at("M\u{3a9}", 3));
        assert_eq!(ohms(-5.6351e6), at("M\u{3a9}", 3));
        assert_eq!(ohms(0.0), at("\u{3a9}", 2));
        // Re-expressed into the unit picked.
        let (_, ratio, _) = s.shown().layout(5_635.1, "\u{3a9}").unwrap();
        assert!((ratio.apply(5_635.1) - 5.6351).abs() < 1e-12);
    }

    /// Keyed by the prefix, not by the unit the figures are held in: the
    /// series moving from MΩ to Ω writes the same figure the same way.
    #[test]
    fn the_shown_prefixes_survive_an_auto_range_step() {
        let mut s = SeriesStats::new(false);
        s.push(&shown("\u{3a9}", "M\u{3a9}", " 1.2341"));
        assert_eq!(
            written(&s, 1.2341, "M\u{3a9}"),
            Some(("M\u{3a9}".into(), 4))
        );
        s.push(&shown("\u{3a9}", "\u{3a9}", "219.82"));
        assert_eq!(s.unit(), Some("\u{3a9}"));
        assert_eq!(
            written(&s, 1_234_100.0, "\u{3a9}"),
            Some(("M\u{3a9}".into(), 4))
        );
        assert_eq!(written(&s, 219.82, "\u{3a9}"), Some(("\u{3a9}".into(), 2)));
        // Another quantity's figure (a plotted sub-value) is not written
        // from them.
        assert_eq!(written(&s, 50.0, "Hz"), None);
    }

    /// A new series and the Reset button forget what the meter showed.
    #[test]
    fn a_new_series_and_reset_forget_the_shown_prefixes() {
        let mut s = SeriesStats::new(false);
        s.push(&shown("DC V", "V", " 1.6112"));
        s.reset();
        assert_eq!(written(&s, 1.6, "V"), None);
        s.push(&shown("DC V", "V", " 1.6112"));
        // The dial turned to mV with the probes open: a new series, and an
        // overload shows no digits.
        let mut ol = reading("DC mV", "mV", 0.0, Instant::now());
        ol.value = MeasuredValue::Overload;
        ol.display_raw = Some("    OL".into());
        assert!(s.push(&ol).is_some());
        assert_eq!(written(&s, 1.6, "V"), None);
        s.push(&shown("DC mV", "mV", " 12.345"));
        assert_eq!(written(&s, 0.012, "V"), Some(("mV".into(), 3)));
    }

    /// Digits that do not spell the value, a word in their place and a
    /// reading without them count for nothing.
    #[test]
    fn only_digits_that_spell_the_value_count() {
        let mut s = SeriesStats::new(false);
        // test_fixture's "  5.678" beside a value of 1.
        s.push(&reading("DC V", "V", 1.0, Instant::now()));
        let mut none = reading("DC V", "V", 1.0, Instant::now());
        none.display_raw = None;
        s.push(&none);
        let mut pattern = reading("DC V", "V", 1.0, Instant::now());
        pattern.display_raw = Some("1.2.3.4".into());
        s.push(&pattern);
        let mut word = reading("DC V", "V", 1.0, Instant::now());
        word.value = MeasuredValue::NoReading("Auto");
        word.display_raw = Some(" 0.000".into());
        s.push(&word);
        assert_eq!(written(&s, 1.0, "V"), None);
        // A float-based family's text, its f32 widened beside it.
        let mut f32_reading = reading("DC V", "V", f64::from(12.345f32), Instant::now());
        f32_reading.display_raw = Some(format!("{}", 12.345f32));
        s.push(&f32_reading);
        assert_eq!(written(&s, 1.0, "V"), Some(("V".into(), 3)));
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
