//! Threshold alarms: a high and a low limit on the main reading, and the
//! breaches of them.
//!
//! Limits are in the reading's **base unit**, as the transform's factors are
//! (see [`crate::transform`]): a high limit of 5 means 5 V whether the meter
//! shows 4980 mV or 5.02 V, so an auto-range step cannot move it a decade.
//! With a software transform on, the reading is already in its output unit
//! (the base unit or the user's label), and the limit is in that unit.
//!
//! An alarm binds to the quantity of the first reading it sees — the meter's
//! own base unit, read before any transform, so a `--unit` relabel that pins
//! the label cannot hide a dial turn — and stays idle while readings are of
//! another quantity. It never rebinds: limits typed for volts must not start
//! judging ohms.
//!
//! A breach is over once the reading comes back inside by a [`Hysteresis`]
//! band: a noisy reading hovering at a limit crosses it on every other
//! sample, and without the band each crossing would be a breach of its own,
//! with a marker and a line on stderr each.
//!
//! Shared by the CLI read loop and the GUI message drain, so the two agree on
//! what is a breach and on the note that marks one.

use crate::measurement::{MeasuredValue, Measurement};
use crate::transform::{RAW_LABEL, si_prefix};

/// The band [`Hysteresis::Auto`] takes, in counts of the reading's last
/// displayed digit: wider than the one- or two-count flicker of a reading
/// sitting on a limit, narrow next to any change worth an alarm.
pub(crate) const AUTO_COUNTS: f64 = 5.0;

/// How far back inside a reading has to come after a breach before the same
/// limit can raise another.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Hysteresis {
    /// A few counts of the reading's last displayed digit, so the band
    /// follows the meter's resolution through every range.
    #[default]
    Auto,
    /// A value in the limits' unit.
    Absolute(f64),
    /// A percentage of each limit's size.
    Percent(f64),
}

/// Why a hysteresis value is unusable. The wording is each binary's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HysteresisError {
    /// Not a number, or a number followed by something other than `%`.
    NotANumber,
    /// Negative, NaN or infinite: no band a reading could come back by.
    NotABand,
    /// A percentage with a limit of zero, of which every percentage is zero:
    /// the band would vanish without a word, so a value is asked for.
    PercentOfZero,
}

impl Hysteresis {
    /// `"0.05"` is a value in the limits' unit, `"1%"` a percentage of each
    /// limit; blank or `"auto"` is [`Hysteresis::Auto`].
    pub fn parse(text: &str) -> Result<Self, HysteresisError> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("auto") {
            return Ok(Self::Auto);
        }
        let (number, percent) = match text.strip_suffix('%') {
            Some(number) => (number.trim_end(), true),
            None => (text, false),
        };
        let v: f64 = number.parse().map_err(|_| HysteresisError::NotANumber)?;
        if !v.is_finite() || v < 0.0 {
            return Err(HysteresisError::NotABand);
        }
        Ok(if percent {
            Self::Percent(v)
        } else {
            Self::Absolute(v)
        })
    }

    /// `"0.05"` or `"1%"` for a banner, `None` for the automatic band.
    fn describe(&self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::Absolute(v) => Some(format!("{v}")),
            Self::Percent(v) => Some(format!("{v}%")),
        }
    }

    /// Whether this band can apply to `limits`: a percentage needs no limit
    /// of zero.
    pub fn check(self, limits: &Limits) -> Result<Self, HysteresisError> {
        if matches!(self, Self::Percent(_))
            && limits.low.into_iter().chain(limits.high).any(|v| v == 0.0)
        {
            return Err(HysteresisError::PercentOfZero);
        }
        Ok(self)
    }

    /// The band below `limit` (above it, for a low limit) a reading has to
    /// come back past, given the size of one count of its last digit.
    fn band(&self, limit: f64, count: f64) -> f64 {
        match self {
            Self::Auto => AUTO_COUNTS * count,
            Self::Absolute(v) => *v,
            Self::Percent(p) => limit.abs() * p / 100.0,
        }
    }
}

/// One count of the reading's last displayed digit, in `mult`'s base: `7.298`
/// shown in V is 0.001 V. Zero when the meter's digits are unknown, which
/// leaves [`Hysteresis::Auto`] with no band.
fn count_size(m: &Measurement, mult: f64) -> f64 {
    let Some(display) = m.display_raw.as_deref() else {
        return 0.0;
    };
    let decimals = display.split_once('.').map_or(0, |(_, frac)| {
        frac.chars().take_while(char::is_ascii_digit).count()
    });
    // `decimals` is a handful of digits, far inside i32.
    10f64.powi(-(decimals as i32)) * mult
}

/// Why a limit, or a pair of them, is unusable.
///
/// The rules are the same wherever limits are typed — the `--alarm-*` flags,
/// the GUI's Limits fields — but the wording is each binary's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitError {
    /// NaN or infinity, which no reading can cross.
    NotFinite,
    /// The low limit is not below the high one, so every reading breaches one.
    LowNotBelowHigh,
}

/// A high and a low limit, either of them optional, in base units.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Limits {
    pub low: Option<f64>,
    pub high: Option<f64>,
}

impl Limits {
    /// Check a pair of limits: each finite, and the low one below the high.
    pub fn check(low: Option<f64>, high: Option<f64>) -> Result<Self, LimitError> {
        if low.into_iter().chain(high).any(|v| !v.is_finite()) {
            return Err(LimitError::NotFinite);
        }
        if let (Some(low), Some(high)) = (low, high)
            && low >= high
        {
            return Err(LimitError::LowNotBelowHigh);
        }
        Ok(Self { low, high })
    }

    /// Whether neither limit is set, so there is nothing to watch.
    pub fn is_empty(&self) -> bool {
        self.low.is_none() && self.high.is_none()
    }

    /// The limits in `unit`, the unit a reading is shown in: `mV` turns a
    /// high limit of 5 into 5000. `scaled` readings are already in the unit
    /// the limits are, so they pass through.
    pub fn in_unit(&self, unit: &str, scaled: bool) -> Self {
        let mult = if scaled { 1.0 } else { si_prefix(unit).1 };
        Self {
            low: self.low.map(|v| v / mult),
            high: self.high.map(|v| v / mult),
        }
    }

    /// `"above 5 or below 3"`, `"above 5, hysteresis 1%"`: what both
    /// binaries say when an alarm starts. The automatic band goes unnamed.
    pub fn describe(&self, band: Hysteresis) -> String {
        let limits = match (self.high, self.low) {
            (Some(high), Some(low)) => format!("above {high} or below {low}"),
            (Some(high), None) => format!("above {high}"),
            (None, Some(low)) => format!("below {low}"),
            (None, None) => String::new(),
        };
        match band.describe() {
            Some(band) => format!("{limits}, hysteresis {band}"),
            None => limits,
        }
    }

    fn zone_of(&self, v: f64) -> Zone {
        if self.high.is_some_and(|high| v > high) {
            Zone::Above
        } else if self.low.is_some_and(|low| v < low) {
            Zone::Below
        } else {
            Zone::Inside
        }
    }
}

/// The unit a limit typed now would be in for readings like `m`: its base
/// unit, or with `scaled` the transform's output unit, which is already the
/// base or the user's label. `None` for a reading with no unit to go by: a
/// word shown instead of a reading, or a frame without a main reading.
pub fn limit_unit(m: &Measurement, scaled: bool) -> Option<&str> {
    if matches!(m.value, MeasuredValue::NoReading(_) | MeasuredValue::Absent) {
        return None;
    }
    let unit = if scaled {
        m.unit.as_ref()
    } else {
        si_prefix(&m.unit).0
    };
    (!unit.is_empty()).then_some(unit)
}

/// Where a reading lies against the limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Inside,
    Above,
    Below,
}

/// A reading out of the limits where the alarm last saw none out: a
/// crossing, or the first reading judged.
#[derive(Debug, Clone, PartialEq)]
pub struct Breach {
    /// [`Zone::Above`] or [`Zone::Below`].
    pub zone: Zone,
    /// The limit crossed.
    pub limit: f64,
    /// The unit the limit is in: the reading's base unit, or the transform's.
    pub unit: String,
    /// The reading crossed the limit. `false` when it was already out as the
    /// alarm started judging it — limits set while it was out, a cleared
    /// session, a return from another quantity: no change was seen, so
    /// nothing is counted or marked, and the binaries only say so.
    pub crossing: bool,
}

impl Breach {
    /// What the binaries say: the note a crossing's marker carries,
    /// `"Above high limit 5 V"`, or for a reading already out, `"Already
    /// below low limit 3 V; alarms start at the next crossing"`.
    pub fn note(&self) -> String {
        let what = match (self.zone, self.crossing) {
            (Zone::Below, true) => "Below low limit",
            (Zone::Below, false) => "Already below low limit",
            (Zone::Above | Zone::Inside, true) => "Above high limit",
            (Zone::Above | Zone::Inside, false) => "Already above high limit",
        };
        let note = if self.unit.is_empty() {
            format!("{what} {}", self.limit)
        } else {
            format!("{what} {} {}", self.limit, self.unit)
        };
        if self.crossing {
            note
        } else {
            format!("{note}; alarms start at the next crossing")
        }
    }
}

/// An alarm watching one run's main reading.
#[derive(Debug, Clone)]
pub struct Alarm {
    limits: Limits,
    /// The meter's base unit the alarm watches; `None` until the first reading.
    quantity: Option<String>,
    /// The base unit of the last reading of another quantity, while there is one.
    idle: Option<String>,
    /// How far back inside a breach ends.
    hysteresis: Hysteresis,
    /// The latched zone: out until a reading comes back inside by the
    /// hysteresis band. `None` until a reading is compared, so a reading
    /// already out then is found out rather than seen crossing.
    zone: Option<Zone>,
    /// Where the last compared reading lay.
    reading_zone: Option<Zone>,
    /// Breaches of the high limit.
    pub high_count: u64,
    /// Breaches of the low limit.
    pub low_count: u64,
}

impl Alarm {
    pub fn new(limits: Limits, hysteresis: Hysteresis) -> Self {
        Self {
            limits,
            quantity: None,
            idle: None,
            hysteresis,
            zone: None,
            reading_zone: None,
            high_count: 0,
            low_count: 0,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn hysteresis(&self) -> Hysteresis {
        self.hysteresis
    }

    /// The base unit the alarm watches, once a reading has bound it.
    pub fn quantity(&self) -> Option<&str> {
        self.quantity.as_deref()
    }

    /// The base unit of the readings arriving while they are of another
    /// quantity than the one the alarm watches: the alarm is idle.
    pub fn idle(&self) -> Option<&str> {
        self.idle.as_deref()
    }

    /// Where the last compared reading lay, `None` before one or while idle.
    pub fn reading_zone(&self) -> Option<Zone> {
        self.reading_zone
    }

    /// Zero the counts, keeping the quantity bound and where the reading
    /// lies: a cleared session counts afresh on the same limits, and a
    /// reading still out is the same breach, not one found anew.
    pub fn clear_counts(&mut self) {
        self.high_count = 0;
        self.low_count = 0;
    }

    /// The unit the limits are in: a scaled reading's label, `relabel`, or
    /// the base unit of the quantity watched. `None` before a reading binds.
    pub fn limits_unit<'a>(&'a self, relabel: Option<&'a str>) -> Option<&'a str> {
        relabel.or(self.quantity())
    }

    /// Compare one reading, returning the breach it starts, if any: a
    /// crossing, counted, or the reading found already out, which is not.
    ///
    /// `m` is the reading as shown, after any software transform; `scaled`
    /// says whether one was applied. Only a numeric main reading counts: an
    /// overload says only that the reading is beyond the meter's range — the
    /// resting state of resistance and diode modes, with no sign or size to
    /// compare — and no-reading words, NCV levels and frames without a main
    /// reading carry no number either, so all of them leave the alarm as it
    /// was.
    pub fn check(&mut self, m: &Measurement, scaled: bool) -> Option<Breach> {
        let v = match m.value {
            MeasuredValue::Normal(v) => v,
            // Nothing to show past a limit while the meter shows no number.
            MeasuredValue::Overload | MeasuredValue::NoReading(_) | MeasuredValue::NcvLevel(_) => {
                self.reading_zone = None;
                return None;
            }
            // The main reading carries on in frames of its own either side.
            MeasuredValue::Absent => return None,
        };
        // The meter's own unit: the main reading's, or under a transform the
        // Raw sub-value's, which carries the reading as the meter sent it.
        let meter_unit = if scaled {
            m.aux_values
                .iter()
                .rfind(|a| a.label == RAW_LABEL)
                .map_or(m.unit.as_ref(), |a| a.unit.as_ref())
        } else {
            m.unit.as_ref()
        };
        let base = si_prefix(meter_unit).0;
        match &self.quantity {
            None => self.quantity = Some(base.to_string()),
            Some(q) if q != base => {
                if self.idle.as_deref() != Some(base) {
                    self.idle = Some(base.to_string());
                }
                self.reading_zone = None;
                return None;
            }
            Some(_) => {}
        }
        if self.idle.take().is_some() {
            // Back from another quantity: whatever the reading did meanwhile
            // went unwatched, so it is judged afresh.
            self.zone = None;
        }
        // Into the limits' unit: as shown under a transform, else the base.
        let mult = if scaled { 1.0 } else { si_prefix(&m.unit).1 };
        let (value, unit) = (v * mult, if scaled { m.unit.as_ref() } else { base });
        let now = self.limits.zone_of(value);
        self.reading_zone = Some(now);
        // A breach ends once the reading is back inside by the band; short of
        // that, crossing the same limit again is the same breach.
        let count = count_size(m, mult);
        let rearmed = match self.zone {
            Some(Zone::Above) => self
                .limits
                .high
                .is_some_and(|high| value <= high - self.hysteresis.band(high, count)),
            Some(Zone::Below) => self
                .limits
                .low
                .is_some_and(|low| value >= low + self.hysteresis.band(low, count)),
            Some(Zone::Inside) | None => false,
        };
        if rearmed {
            self.zone = Some(Zone::Inside);
        }
        match now {
            Zone::Inside => {
                if self.zone.is_none() {
                    self.zone = Some(Zone::Inside);
                }
                None
            }
            out => {
                if self.zone == Some(out) {
                    return None;
                }
                let crossing = self.zone.is_some();
                self.zone = Some(out);
                let limit = match out {
                    Zone::Below => {
                        self.low_count += u64::from(crossing);
                        self.limits.low
                    }
                    Zone::Above | Zone::Inside => {
                        self.high_count += u64::from(crossing);
                        self.limits.high
                    }
                };
                Some(Breach {
                    zone: out,
                    limit: limit.unwrap_or_default(),
                    unit: unit.to_string(),
                    crossing,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::StatusFlags;
    use crate::transform::Transform;
    use std::time::Instant;

    fn reading(value: f64, unit: &'static str, at: Instant) -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(value), unit, StatusFlags::default());
        m.timestamp = at;
        m
    }

    fn alarm(low: Option<f64>, high: Option<f64>) -> Alarm {
        Alarm::new(Limits::check(low, high).unwrap(), Hysteresis::Auto)
    }

    /// A reading shown as `display`, which sets the size of one count.
    fn shown(value: f64, display: &str) -> Measurement {
        let mut m = reading(value, "V", Instant::now());
        m.display_raw = Some(display.to_string());
        m
    }

    #[test]
    fn limits_must_be_finite_and_ordered() {
        assert_eq!(
            Limits::check(Some(f64::NAN), None),
            Err(LimitError::NotFinite)
        );
        assert_eq!(
            Limits::check(None, Some(f64::INFINITY)),
            Err(LimitError::NotFinite)
        );
        assert_eq!(
            Limits::check(Some(5.0), Some(5.0)),
            Err(LimitError::LowNotBelowHigh)
        );
        assert!(Limits::check(Some(-1.0), Some(5.0)).is_ok());
        assert!(Limits::check(None, None).unwrap().is_empty());
    }

    #[test]
    fn limits_are_in_base_units_whatever_the_range() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4980.0, "mV", t), false), None);
        let breach = a.check(&reading(5.02, "V", t), false).unwrap();
        assert_eq!(breach.note(), "Above high limit 5 V");
        assert_eq!(a.high_count, 1);
    }

    #[test]
    fn the_limit_is_strict() {
        let t = Instant::now();
        let mut a = alarm(Some(3.0), Some(5.0));
        assert_eq!(a.check(&reading(5.0, "V", t), false), None);
        assert_eq!(a.check(&reading(3.0, "V", t), false), None);
        assert_eq!(a.reading_zone(), Some(Zone::Inside));
    }

    #[test]
    fn a_breach_counts_once_while_the_reading_stays_out() {
        let t = Instant::now();
        let mut a = alarm(Some(3.0), Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t), false), None);
        assert_eq!(
            a.check(&reading(2.0, "V", t), false).unwrap().note(),
            "Below low limit 3 V",
        );
        assert_eq!(a.check(&reading(1.0, "V", t), false), None);
        assert_eq!((a.low_count, a.high_count), (1, 0));
        // A jump to the other side is a breach of its own.
        assert!(a.check(&reading(6.0, "V", t), false).is_some());
        assert_eq!((a.low_count, a.high_count), (1, 1));
    }

    /// A reading flickering at the limit is one breach; coming back inside
    /// by the automatic band, five counts of its last digit, ends it.
    #[test]
    fn a_breach_ends_once_the_reading_is_back_by_the_band() {
        let mut a = alarm(None, Some(7.2975));
        assert_eq!(a.check(&shown(7.297, "7.297"), false), None);
        assert!(a.check(&shown(7.298, "7.298"), false).is_some());
        for v in [7.297, 7.298, 7.297, 7.293, 7.298] {
            let display = format!("{v:.3}");
            assert_eq!(a.check(&shown(v, &display), false), None, "{v}");
        }
        assert_eq!(a.high_count, 1);
        // 7.2975 - 5 × 0.001 = 7.2925: back past it, the breach is over.
        assert_eq!(a.check(&shown(7.292, "7.292"), false), None);
        assert!(a.check(&shown(7.298, "7.298"), false).is_some());
        assert_eq!(a.high_count, 2);
    }

    #[test]
    fn a_typed_band_is_absolute_or_a_percentage_of_the_limit() {
        let limits = Limits::check(Some(100.0), None).unwrap();
        let mut a = Alarm::new(limits, Hysteresis::Absolute(2.0));
        assert_eq!(a.check(&shown(103.0, "103.0"), false), None);
        assert!(a.check(&shown(99.0, "99.0"), false).is_some());
        assert_eq!(a.check(&shown(101.5, "101.5"), false), None);
        assert!(
            a.check(&shown(99.0, "99.0"), false).is_none(),
            "not back by 2"
        );
        assert_eq!(a.check(&shown(102.0, "102.0"), false), None);
        assert!(a.check(&shown(99.0, "99.0"), false).is_some());

        let mut a = Alarm::new(limits, Hysteresis::Percent(5.0));
        assert_eq!(a.check(&shown(106.0, "106.0"), false), None);
        assert!(a.check(&shown(99.0, "99.0"), false).is_some());
        assert_eq!(a.check(&shown(104.0, "104.0"), false), None);
        assert!(a.check(&shown(99.0, "99.0"), false).is_none(), "5 % is 5");
        assert_eq!(a.check(&shown(105.0, "105.0"), false), None);
        assert!(a.check(&shown(99.0, "99.0"), false).is_some());
    }

    #[test]
    fn a_percentage_band_needs_a_limit_that_is_not_zero() {
        let zero = Limits::check(Some(0.0), Some(5.0)).unwrap();
        assert_eq!(
            Hysteresis::Percent(1.0).check(&zero),
            Err(HysteresisError::PercentOfZero)
        );
        assert!(Hysteresis::Absolute(0.01).check(&zero).is_ok());
        assert!(Hysteresis::Auto.check(&zero).is_ok());
    }

    /// The badge goes with the number: an overload or a word shown instead
    /// has nothing past a limit to show.
    #[test]
    fn a_reading_without_a_number_shows_no_zone() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t), false), None);
        assert!(a.check(&reading(6.0, "V", t), false).is_some());
        let ol = Measurement::test_fixture(MeasuredValue::Overload, "V", StatusFlags::default());
        assert_eq!(a.check(&ol, false), None);
        assert_eq!(a.reading_zone(), None);
        // Still the same breach once the number is back.
        assert_eq!(a.check(&reading(6.0, "V", t), false), None);
        assert_eq!(a.reading_zone(), Some(Zone::Above));
    }

    #[test]
    fn hysteresis_parses_a_value_a_percentage_or_auto() {
        assert_eq!(Hysteresis::parse(" "), Ok(Hysteresis::Auto));
        assert_eq!(Hysteresis::parse("Auto"), Ok(Hysteresis::Auto));
        assert_eq!(Hysteresis::parse("0.05"), Ok(Hysteresis::Absolute(0.05)));
        assert_eq!(Hysteresis::parse("1 %"), Ok(Hysteresis::Percent(1.0)));
        assert_eq!(Hysteresis::parse("-1"), Err(HysteresisError::NotABand));
        assert_eq!(Hysteresis::parse("inf"), Err(HysteresisError::NotABand));
        assert_eq!(Hysteresis::parse("1V"), Err(HysteresisError::NotANumber));
        assert_eq!(Hysteresis::Percent(1.0).describe().as_deref(), Some("1%"));
    }

    #[test]
    fn a_limit_set_while_out_waits_for_a_crossing() {
        let mut a = alarm(None, Some(5.0));
        let found = a.check(&reading(9.0, "V", Instant::now()), false).unwrap();
        assert!(!found.crossing);
        assert_eq!(
            found.note(),
            "Already above high limit 5 V; alarms start at the next crossing"
        );
        assert_eq!(a.high_count, 0);
        assert_eq!(a.reading_zone(), Some(Zone::Above));
        assert_eq!(a.check(&reading(9.1, "V", Instant::now()), false), None);
        // Back by the band, then out: that is a crossing.
        assert_eq!(a.check(&reading(4.0, "V", Instant::now()), false), None);
        assert!(
            a.check(&reading(9.0, "V", Instant::now()), false)
                .unwrap()
                .crossing
        );
        assert_eq!(a.high_count, 1);
    }

    #[test]
    fn overload_and_wordless_readings_leave_the_alarm_alone() {
        let t = Instant::now();
        let mut a = alarm(Some(3.0), Some(5.0));
        let flags = StatusFlags::default();
        for value in [
            MeasuredValue::Overload,
            MeasuredValue::NoReading("Auto"),
            MeasuredValue::NcvLevel(3),
            MeasuredValue::Absent,
        ] {
            assert_eq!(
                a.check(&Measurement::test_fixture(value, "V", flags), false),
                None
            );
        }
        assert_eq!(a.quantity(), None);
        assert_eq!(a.check(&reading(4.0, "V", t), false), None);
        assert_eq!(a.quantity(), Some("V"));
    }

    #[test]
    fn another_quantity_leaves_the_alarm_idle() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t), false), None);
        assert_eq!(a.check(&reading(220.0, "kΩ", t), false), None);
        assert_eq!(a.idle(), Some("Ω"));
        assert_eq!(a.reading_zone(), None);
        assert_eq!(
            a.check(&reading(9.0, "V", t), false)
                .map(|b| (b.zone, b.crossing)),
            Some((Zone::Above, false)),
            "unwatched meanwhile: found out, not seen crossing"
        );
        assert_eq!(a.idle(), None);
    }

    #[test]
    fn scaled_readings_compare_as_shown_and_bind_to_the_meters_unit() {
        let t = Instant::now();
        // A 10 mV/A clamp: 12.3 mV is 1.23 A.
        let clamp = Transform::linear(100.0, 0.0, Some("A".into()));
        let mut a = alarm(None, Some(1.0));
        let mut m = reading(9.0, "mV", t);
        clamp.apply(&mut m);
        assert_eq!(a.check(&m, true), None, "0.9 A is inside");
        let mut m = reading(12.3, "mV", t);
        clamp.apply(&mut m);
        let breach = a.check(&m, true).unwrap();
        assert_eq!(breach.note(), "Above high limit 1 A");
        assert_eq!(a.quantity(), Some("V"));
        // A dial turn to Ω keeps the "A" label but not the meter's unit.
        let mut m = reading(0.5, "kΩ", t);
        clamp.apply(&mut m);
        assert_eq!(m.unit, "A");
        assert_eq!(a.check(&m, true), None);
        assert_eq!(a.idle(), Some("Ω"));
    }

    #[test]
    fn clearing_the_counts_keeps_the_quantity() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t), false), None);
        assert!(a.check(&reading(6.0, "V", t), false).is_some());
        a.clear_counts();
        assert_eq!(a.high_count, 0);
        assert_eq!(a.quantity(), Some("V"));
        // The reading still out after a clear is the breach it already was.
        assert_eq!(a.check(&reading(6.0, "V", t), false), None);
        assert_eq!(a.high_count, 0);
        assert_eq!(a.limits_unit(None), Some("V"));
        assert_eq!(a.limits_unit(Some("A")), Some("A"));
    }

    #[test]
    fn a_limit_is_in_the_readings_base_unit() {
        let t = Instant::now();
        assert_eq!(limit_unit(&reading(4980.0, "mV", t), false), Some("V"));
        assert_eq!(limit_unit(&reading(12.3, "A", t), true), Some("A"));
        let flags = StatusFlags::default();
        let auto = Measurement::test_fixture(MeasuredValue::NoReading("Auto"), "", flags);
        assert_eq!(limit_unit(&auto, false), None);
    }

    #[test]
    fn limits_convert_to_the_shown_unit() {
        let l = Limits::check(Some(0.5), Some(5.0)).unwrap();
        let mv = l.in_unit("mV", false);
        assert_eq!((mv.low, mv.high), (Some(500.0), Some(5000.0)));
        assert_eq!(l.in_unit("mV", true), l);
        assert_eq!(l.describe(Hysteresis::Auto), "above 5 or below 0.5");
        assert_eq!(
            l.describe(Hysteresis::Percent(1.0)),
            "above 5 or below 0.5, hysteresis 1%"
        );
    }
}
