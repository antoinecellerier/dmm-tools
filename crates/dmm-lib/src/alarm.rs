//! Threshold alarms: a high and a low limit on the main reading, and the
//! breaches of them.
//!
//! Limits are in the reading's **base unit**, as the transform's factors are
//! (see [`crate::transform`]): a high limit of 5 means 5 V whether the meter
//! shows 4980 mV or 5.02 V, so an auto-range step cannot move it a decade.
//! With a software transform on, the reading is already in its output unit
//! (the base unit or the user's label), and the limit is in that unit. So is
//! a file's reading that was scaled when written: its Raw sub-value says so
//! ([`LimitUnit::of`]), and the same limits mean the same on import as live.
//!
//! An alarm binds to the quantity of the first reading it sees — the meter's
//! own base unit, read before any transform, so a `--unit` relabel that pins
//! the label cannot hide a dial turn — and to the unit its limits are in. It
//! stays idle while readings are of another quantity, or in another limit
//! unit (a file recorded across a scale change), and never rebinds: limits
//! typed for volts must not start judging ohms.
//!
//! Limits typed with a unit ("4980 mV", "10 kΩ") carry its base ([`Limits::unit`])
//! and pin the alarm to it: it binds only to readings whose limits' unit is
//! of that base, and compares them in it, whatever prefix the reading or a
//! scale's label shows.
//!
//! A breach is over once the reading comes back inside by a [`Hysteresis`]
//! band: a noisy reading hovering at a limit crosses it on every other
//! sample, and without the band each crossing would be a breach of its own,
//! with a marker and a line on stderr each.
//!
//! Shared by the CLI read loop and the GUI message drain, so the two agree on
//! what is a breach and on the note that marks one.

use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::transform::{QuantityError, RAW_LABEL, read_quantity, si_prefix, with_unit};

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
    /// A unit other than the limits' ([`Limits::unit`]), or one with limits
    /// typed bare.
    NotTheLimitsUnit,
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

    /// `"0.05"` or `"1%"` for a banner, `None` for the automatic band: the
    /// text [`Hysteresis::parse`] reads back to this band.
    pub fn describe(&self) -> Option<String> {
        match self {
            Self::Auto => None,
            Self::Absolute(v) => Some(format!("{v}")),
            Self::Percent(v) => Some(format!("{v}%")),
        }
    }

    /// `text` as [`Hysteresis::parse`] reads it, or a value with a unit
    /// ("10 mV", "2k"): in the limits' `unit` it is a value in its base, and
    /// a prefix alone multiplies. A unit with limits typed bare (`unit`
    /// `None`) is refused: what it would be in is not known yet.
    pub fn parse_in(text: &str, unit: Option<&'static str>) -> Result<Self, HysteresisError> {
        match Self::parse(text) {
            Err(HysteresisError::NotANumber) => {}
            parsed => return parsed,
        }
        let v = match read_quantity(text) {
            Ok((v, None)) => v,
            Ok((v, typed)) if typed == unit => v,
            Ok(_) => return Err(HysteresisError::NotTheLimitsUnit),
            Err(_) => return Err(HysteresisError::NotANumber),
        };
        if !v.is_finite() || v < 0.0 {
            return Err(HysteresisError::NotABand);
        }
        Ok(Self::Absolute(v))
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
    let Some(decimals) = m.decimals() else {
        return 0.0;
    };
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
    /// The two limits were typed in units of different quantities.
    UnitsDiffer,
}

/// Why a limit typed as text ([`parse_limit`]) was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitParseError {
    /// Neither a number nor a number with a unit.
    NotAQuantity,
    /// A number the limits refuse.
    Limit(LimitError),
}

/// One limit as typed: a number, bare or with a unit ([`read_quantity`]),
/// in its unit's base and that base. Blank is no limit.
pub fn parse_limit(text: &str) -> Result<Option<(f64, Option<&'static str>)>, LimitParseError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let (v, unit) = read_quantity(text).map_err(|e| match e {
        QuantityError::NotANumber | QuantityError::UnknownUnit => LimitParseError::NotAQuantity,
    })?;
    Limits::check(Some(v), None).map_err(LimitParseError::Limit)?;
    Ok(Some((v, unit)))
}

/// Which unit limits are in for a reading, and so how the reading is
/// compared with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitUnit {
    /// The reading's base unit: the reading is as the meter sent it, in
    /// whichever prefix it showed, and is converted to the base (4980 mV is
    /// 4.98 against a limit in V).
    Base,
    /// The unit the reading is shown in, taken as it is: the reading went
    /// through a software transform, whose output unit is the base unit or
    /// the user's label.
    Shown,
}

impl LimitUnit {
    /// The unit limits are in for `m`: [`LimitUnit::Shown`] when it went
    /// through a software transform, which keeps the meter's own reading as
    /// a [`RAW_LABEL`] sub-value. That holds for a file's reading too: one
    /// written scaled carries its Raw column, and is judged as it was live.
    pub fn of(m: &Measurement) -> Self {
        Self::of_raw(raw_reading(m))
    }

    /// Shown when the reading carries a Raw sub-value, `raw`.
    fn of_raw(raw: Option<&AuxValue>) -> Self {
        match raw {
            Some(_) => Self::Shown,
            None => Self::Base,
        }
    }

    /// The multiplier taking a reading in `unit` into the limits' unit.
    fn mult(self, unit: &str) -> f64 {
        match self {
            Self::Base => si_prefix(unit).1,
            Self::Shown => 1.0,
        }
    }
}

/// The reading as the meter sent it, which a software transform keeps as a
/// [`RAW_LABEL`] sub-value. A file scaled when written and scaled again on
/// import carries two; the last is the file's reading, which the second
/// scale was taken from.
fn raw_reading(m: &Measurement) -> Option<&AuxValue> {
    m.aux_values.iter().rfind(|a| a.label == RAW_LABEL)
}

/// A high and a low limit, either of them optional, in base units, or in
/// the scale's unit for a scaled reading ([`LimitUnit`]).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Limits {
    pub low: Option<f64>,
    pub high: Option<f64>,
    /// The base of the unit the limits were typed in, when one was: they
    /// are in it, and the alarm watches only readings in it (see the module
    /// doc). `None` for limits typed bare.
    pub unit: Option<&'static str>,
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
        Ok(Self {
            low,
            high,
            unit: None,
        })
    }

    /// Check a pair of limits as [`parse_limit`] reads them: one typed with
    /// a unit puts both in it, a bare one being in it too, and two units
    /// must name one quantity.
    pub fn typed(
        low: Option<(f64, Option<&'static str>)>,
        high: Option<(f64, Option<&'static str>)>,
    ) -> Result<Self, LimitError> {
        let unit = match (low.and_then(|l| l.1), high.and_then(|h| h.1)) {
            (Some(a), Some(b)) if a != b => return Err(LimitError::UnitsDiffer),
            (a, b) => a.or(b),
        };
        let limits = Self::check(low.map(|l| l.0), high.map(|h| h.0))?;
        Ok(Self { unit, ..limits })
    }

    /// These limits, now in `unit` ([`Limits::unit`]).
    pub fn in_base(self, unit: Option<&'static str>) -> Self {
        Self { unit, ..self }
    }

    /// How a reading in `shown`, whose limits are in `limit_unit`, is taken
    /// into these limits' unit: under a typed unit, into the base of what
    /// it shows, else as [`LimitUnit`] says.
    fn mult(&self, shown: &str, limit_unit: LimitUnit) -> f64 {
        match self.unit {
            Some(_) => si_prefix(shown).1,
            None => limit_unit.mult(shown),
        }
    }

    /// `v` as the binaries write a limit: with the unit typed after it.
    pub fn written(&self, v: f64) -> String {
        with_unit(v, self.unit.unwrap_or_default())
    }

    /// Whether neither limit is set, so there is nothing to watch.
    pub fn is_empty(&self) -> bool {
        self.low.is_none() && self.high.is_none()
    }

    /// The limits in `unit`, the unit a reading is shown in: `mV` turns a
    /// high limit of 5 into 5000. Under [`LimitUnit::Shown`] readings are
    /// already in the unit the limits are, so they pass through.
    pub fn in_unit(&self, unit: &str, limit_unit: LimitUnit) -> Self {
        let mult = self.mult(unit, limit_unit);
        Self {
            low: self.low.map(|v| v / mult),
            high: self.high.map(|v| v / mult),
            unit: self.unit,
        }
    }

    /// `"above 5 or below 3"`, `"above 5, hysteresis 1%"`: what both
    /// binaries say when an alarm starts. The automatic band goes unnamed.
    /// Limits typed with a unit are written in its base, as the alarm reads
    /// them: `"above 4.98 V"`, so a slip of case ("mhz") shows.
    pub fn describe(&self, band: Hysteresis) -> String {
        let limits = match (self.high, self.low) {
            (Some(high), Some(low)) => {
                format!(
                    "above {} or below {}",
                    self.written(high),
                    self.written(low)
                )
            }
            (Some(high), None) => format!("above {}", self.written(high)),
            (None, Some(low)) => format!("below {}", self.written(low)),
            (None, None) => String::new(),
        };
        match self.band(band) {
            Some(band) => format!("{limits}, hysteresis {band}"),
            None => limits,
        }
    }

    /// `band` as [`Limits::describe`] writes it: a value with the limits'
    /// typed unit. `None` for the automatic band.
    pub fn band(&self, band: Hysteresis) -> Option<String> {
        match band {
            // A band in `%` reads back as a percentage of the limit: bare,
            // it is in the limits' unit all the same.
            Hysteresis::Absolute(v) if self.unit != Some("%") => Some(self.written(v)),
            band => band.describe(),
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

/// A limit at `value` in `unit`, the unit a reading is shown in, in the
/// limits' unit, as [`Limits::in_unit`] gives it back: `mV` turns 5000 into
/// 5. Rounded to 15 significant digits, past which the scaling only adds
/// noise: 3.29 mV is 0.00329, not 0.0032900000000000003.
pub fn limit_from_unit(value: f64, unit: &str, limit_unit: LimitUnit) -> f64 {
    let v = value * limit_unit.mult(unit);
    format!("{v:.14e}").parse().unwrap_or(v)
}

/// The unit a limit typed now would be in for readings like `m`: its base
/// unit, or for a scaled reading ([`LimitUnit::of`]) the transform's output
/// unit, which is already the base or the user's label. `None` for a reading with no unit
/// to go by: a word shown instead of a reading, or a frame without a main
/// reading.
pub fn limit_unit(m: &Measurement) -> Option<&str> {
    if matches!(m.value, MeasuredValue::NoReading(_) | MeasuredValue::Absent) {
        return None;
    }
    let unit = match LimitUnit::of(m) {
        LimitUnit::Shown => m.unit.as_ref(),
        LimitUnit::Base => si_prefix(&m.unit).0,
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

/// What an alarm watches: the quantity the meter measures, and the unit the
/// limits are in for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The meter's base unit, read before any transform, so a relabel that
    /// pins the label cannot hide a dial turn.
    quantity: String,
    /// The unit the limits are in: the base unit, or a scaled reading's
    /// shown unit. Two scales on one quantity (a file that spans a scale
    /// change) are two of these, and limits typed for one mean nothing in
    /// the other.
    unit: String,
    /// Whether `unit` is a scaled reading's: a scale with no label shows
    /// the base unit, whose limits mean other values than the meter's own.
    limit_unit: LimitUnit,
}

impl Binding {
    /// The unit the limits are in.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Whether the limits are in a scaled reading's unit.
    pub fn limit_unit(&self) -> LimitUnit {
        self.limit_unit
    }

    /// What moved since `before`, named as the alarm's idle state names it
    /// (see [`Alarm::idle`]); `None` when the two are one.
    pub fn moved_from(&self, before: &Binding) -> Option<&str> {
        before.moved_to(&self.quantity, &self.unit, self.limit_unit)
    }

    /// The meter's quantity when that moved, else the unit the readings
    /// are shown in; `None` when nothing did.
    fn moved_to<'a>(&self, quantity: &'a str, unit: &'a str, kind: LimitUnit) -> Option<&'a str> {
        if self.quantity != quantity {
            Some(quantity)
        } else if self.unit != unit || self.limit_unit != kind {
            Some(unit)
        } else {
            None
        }
    }
}

/// An alarm watching one run's main reading.
#[derive(Debug, Clone)]
pub struct Alarm {
    limits: Limits,
    /// What the first reading bound the alarm to; `None` until one.
    bound: Option<Binding>,
    /// What the last reading of another quantity or limit unit was in,
    /// while there is one.
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
            bound: None,
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

    /// What the first reading bound the alarm to, once one has.
    pub fn binding(&self) -> Option<&Binding> {
        self.bound.as_ref()
    }

    /// The unit the readings are judged in, once a reading has bound the
    /// alarm: the meter's base unit, or a scaled reading's shown unit. The
    /// limits are in it, or in its base when typed with a unit.
    pub fn bound_unit(&self) -> Option<&str> {
        self.bound.as_ref().map(Binding::unit)
    }

    /// What the readings arriving are in while the alarm cannot judge them:
    /// the meter's base unit after a dial turn to another quantity, else the
    /// unit they show, scaled or not, when the limits are not in it (a file
    /// recorded across a scale change). The alarm is idle.
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

    /// The unit the limits are in: the one a reading bound
    /// ([`Alarm::bound_unit`]), else the scale's label, `relabel`, which the
    /// first scaled reading will bind. `None` before a reading binds
    /// without one.
    pub fn limits_unit<'a>(&'a self, relabel: Option<&'a str>) -> Option<&'a str> {
        self.limits.unit.or(self.bound_unit()).or(relabel)
    }

    /// Compare one reading, returning the breach it starts, if any: a
    /// crossing, counted, or the reading found already out, which is not.
    ///
    /// `m` is the reading as shown, after any software transform, which
    /// [`LimitUnit::of`] tells from its Raw sub-value. Only a numeric main
    /// reading counts: an overload says only that the reading is beyond the
    /// meter's range — the resting state of resistance and diode modes, with
    /// no sign or size to compare — and no-reading words, NCV levels and
    /// frames without a main reading carry no number either, so all of them
    /// leave the alarm as it was.
    pub fn check(&mut self, m: &Measurement) -> Option<Breach> {
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
        let raw = raw_reading(m);
        let limit_unit = LimitUnit::of_raw(raw);
        let meter_unit = raw.map_or(m.unit.as_ref(), |a| a.unit.as_ref());
        let base = si_prefix(meter_unit).0;
        // Into the limits' unit: as shown under a transform, else the base.
        let unit = match limit_unit {
            LimitUnit::Shown => m.unit.as_ref(),
            LimitUnit::Base => base,
        };
        match &self.bound {
            // Typed for another quantity: wait for one in its unit.
            None if self
                .limits
                .unit
                .is_some_and(|typed| typed != si_prefix(unit).0) =>
            {
                if self.idle.as_deref() != Some(unit) {
                    self.idle = Some(unit.to_string());
                }
                self.reading_zone = None;
                return None;
            }
            None => {
                self.bound = Some(Binding {
                    quantity: base.to_string(),
                    unit: unit.to_string(),
                    limit_unit,
                });
            }
            Some(b) => {
                if let Some(now) = b.moved_to(base, unit, limit_unit) {
                    if self.idle.as_deref() != Some(now) {
                        self.idle = Some(now.to_string());
                    }
                    self.reading_zone = None;
                    return None;
                }
            }
        }
        if self.idle.take().is_some() {
            // Back from another quantity: whatever the reading did meanwhile
            // went unwatched, so it is judged afresh.
            self.zone = None;
        }
        let mult = self.limits.mult(&m.unit, limit_unit);
        let value = v * mult;
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
                    unit: self.limits.unit.unwrap_or(unit).to_string(),
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
        assert_eq!(a.check(&reading(4980.0, "mV", t)), None);
        let breach = a.check(&reading(5.02, "V", t)).unwrap();
        assert_eq!(breach.note(), "Above high limit 5 V");
        assert_eq!(a.high_count, 1);
    }

    #[test]
    fn the_limit_is_strict() {
        let t = Instant::now();
        let mut a = alarm(Some(3.0), Some(5.0));
        assert_eq!(a.check(&reading(5.0, "V", t)), None);
        assert_eq!(a.check(&reading(3.0, "V", t)), None);
        assert_eq!(a.reading_zone(), Some(Zone::Inside));
    }

    #[test]
    fn a_breach_counts_once_while_the_reading_stays_out() {
        let t = Instant::now();
        let mut a = alarm(Some(3.0), Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert_eq!(
            a.check(&reading(2.0, "V", t)).unwrap().note(),
            "Below low limit 3 V",
        );
        assert_eq!(a.check(&reading(1.0, "V", t)), None);
        assert_eq!((a.low_count, a.high_count), (1, 0));
        // A jump to the other side is a breach of its own.
        assert!(a.check(&reading(6.0, "V", t)).is_some());
        assert_eq!((a.low_count, a.high_count), (1, 1));
    }

    /// A reading flickering at the limit is one breach; coming back inside
    /// by the automatic band, five counts of its last digit, ends it.
    #[test]
    fn a_breach_ends_once_the_reading_is_back_by_the_band() {
        let mut a = alarm(None, Some(7.2975));
        assert_eq!(a.check(&shown(7.297, "7.297")), None);
        assert!(a.check(&shown(7.298, "7.298")).is_some());
        for v in [7.297, 7.298, 7.297, 7.293, 7.298] {
            let display = format!("{v:.3}");
            assert_eq!(a.check(&shown(v, &display)), None, "{v}");
        }
        assert_eq!(a.high_count, 1);
        // 7.2975 - 5 × 0.001 = 7.2925: back past it, the breach is over.
        assert_eq!(a.check(&shown(7.292, "7.292")), None);
        assert!(a.check(&shown(7.298, "7.298")).is_some());
        assert_eq!(a.high_count, 2);
    }

    #[test]
    fn a_typed_band_is_absolute_or_a_percentage_of_the_limit() {
        let limits = Limits::check(Some(100.0), None).unwrap();
        let mut a = Alarm::new(limits, Hysteresis::Absolute(2.0));
        assert_eq!(a.check(&shown(103.0, "103.0")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_some());
        assert_eq!(a.check(&shown(101.5, "101.5")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_none(), "not back by 2");
        assert_eq!(a.check(&shown(102.0, "102.0")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_some());

        let mut a = Alarm::new(limits, Hysteresis::Percent(5.0));
        assert_eq!(a.check(&shown(106.0, "106.0")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_some());
        assert_eq!(a.check(&shown(104.0, "104.0")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_none(), "5 % is 5");
        assert_eq!(a.check(&shown(105.0, "105.0")), None);
        assert!(a.check(&shown(99.0, "99.0")).is_some());
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
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert!(a.check(&reading(6.0, "V", t)).is_some());
        let ol = Measurement::test_fixture(MeasuredValue::Overload, "V", StatusFlags::default());
        assert_eq!(a.check(&ol), None);
        assert_eq!(a.reading_zone(), None);
        // Still the same breach once the number is back.
        assert_eq!(a.check(&reading(6.0, "V", t)), None);
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
        assert_eq!(
            Hysteresis::parse("1V"),
            Err(HysteresisError::NotANumber),
            "see parse_in"
        );
        assert_eq!(Hysteresis::Percent(1.0).describe().as_deref(), Some("1%"));
    }

    #[test]
    fn a_limit_set_while_out_waits_for_a_crossing() {
        let mut a = alarm(None, Some(5.0));
        let found = a.check(&reading(9.0, "V", Instant::now())).unwrap();
        assert!(!found.crossing);
        assert_eq!(
            found.note(),
            "Already above high limit 5 V; alarms start at the next crossing"
        );
        assert_eq!(a.high_count, 0);
        assert_eq!(a.reading_zone(), Some(Zone::Above));
        assert_eq!(a.check(&reading(9.1, "V", Instant::now())), None);
        // Back by the band, then out: that is a crossing.
        assert_eq!(a.check(&reading(4.0, "V", Instant::now())), None);
        assert!(
            a.check(&reading(9.0, "V", Instant::now()))
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
            assert_eq!(a.check(&Measurement::test_fixture(value, "V", flags)), None);
        }
        assert_eq!(a.bound_unit(), None);
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert_eq!(a.bound_unit(), Some("V"));
    }

    #[test]
    fn another_quantity_leaves_the_alarm_idle() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert_eq!(a.check(&reading(220.0, "kΩ", t)), None);
        assert_eq!(a.idle(), Some("Ω"));
        assert_eq!(a.reading_zone(), None);
        assert_eq!(
            a.check(&reading(9.0, "V", t)).map(|b| (b.zone, b.crossing)),
            Some((Zone::Above, false)),
            "unwatched meanwhile: found out, not seen crossing"
        );
        assert_eq!(a.idle(), None);
    }

    /// A step between two prefixes is one quantity, watched in its base unit
    /// like any other reading.
    #[test]
    fn a_step_in_any_prefixed_unit_stays_watched_in_its_base() {
        let t = Instant::now();
        for (low, high, base) in [
            ("mH", "H", "H"),
            ("mWh", "Wh", "Wh"),
            ("mAh", "Ah", "Ah"),
            ("mW/m²", "W/m²", "W/m²"),
            ("m°", "°", "°"),
        ] {
            let mut a = alarm(None, Some(1.0));
            assert_eq!(limit_unit(&reading(900.0, low, t)), Some(base));
            assert_eq!(a.check(&reading(900.0, low, t)), None);
            assert_eq!(a.bound_unit(), Some(base));
            assert_eq!(
                a.check(&reading(1.2, high, t)).map(|b| b.zone),
                Some(Zone::Above),
                "{low} to {high}"
            );
            assert_eq!(a.idle(), None);
        }
    }

    /// An import or a replay is unscaled, so a label read as picohenries
    /// would put the reading 10¹² below any limit typed for it.
    #[test]
    fn a_ph_label_is_compared_as_shown() {
        let t = Instant::now();
        let mut a = alarm(None, Some(7.0));
        assert_eq!(limit_unit(&reading(6.5, "pH", t)), Some("pH"));
        assert_eq!(a.check(&reading(6.5, "pH", t)), None);
        assert_eq!(
            a.check(&reading(7.5, "pH", t)).map(|b| b.zone),
            Some(Zone::Above)
        );
    }

    #[test]
    fn scaled_readings_compare_as_shown_and_bind_to_the_meters_quantity() {
        let t = Instant::now();
        // A 10 mV/A clamp: 12.3 mV is 1.23 A.
        let clamp = Transform::linear(100.0, 0.0, Some("A".into()));
        let mut a = alarm(None, Some(1.0));
        let mut m = reading(9.0, "mV", t);
        clamp.apply(&mut m);
        assert_eq!(a.check(&m), None, "0.9 A is inside");
        let mut m = reading(12.3, "mV", t);
        clamp.apply(&mut m);
        let breach = a.check(&m).unwrap();
        assert_eq!(breach.note(), "Above high limit 1 A");
        assert_eq!(a.bound_unit(), Some("A"));
        // A dial turn to Ω keeps the "A" label but not the meter's unit.
        let mut m = reading(0.5, "kΩ", t);
        clamp.apply(&mut m);
        assert_eq!(m.unit, "A");
        assert_eq!(a.check(&m), None);
        assert_eq!(a.idle(), Some("Ω"));
    }

    /// A limit typed with a unit is read into its base; a unit names the
    /// quantity of both, a bare one included, and two quantities are
    /// refused.
    #[test]
    fn typed_limits_are_in_their_units_base() {
        assert_eq!(parse_limit(" "), Ok(None));
        assert_eq!(parse_limit("4980mV"), Ok(Some((4.98, Some("V")))));
        assert_eq!(parse_limit("4.7k"), Ok(Some((4700.0, None))));
        assert_eq!(parse_limit("5 x"), Err(LimitParseError::NotAQuantity));
        assert_eq!(
            parse_limit("1e308 GV"),
            Err(LimitParseError::Limit(LimitError::NotFinite))
        );
        let limits = Limits::typed(Some((1.0, None)), Some((4.98, Some("V")))).unwrap();
        assert_eq!(limits.unit, Some("V"));
        assert_eq!(
            Limits::typed(Some((1.0, Some("A"))), Some((4.98, Some("V")))),
            Err(LimitError::UnitsDiffer)
        );
        assert_eq!(
            limits.describe(Hysteresis::Absolute(0.01)),
            "above 4.98 V or below 1 V, hysteresis 0.01 V"
        );
        // A band in `%` is written bare: "0.5 %" would read back as a
        // percentage of the limit.
        let duty = Limits::typed(Some((30.0, Some("%"))), Some((70.0, Some("%")))).unwrap();
        assert_eq!(duty.band(Hysteresis::Absolute(0.5)).as_deref(), Some("0.5"));
        assert_eq!(
            Hysteresis::parse_in("0.5", duty.unit),
            Ok(Hysteresis::Absolute(0.5))
        );
    }

    /// A band typed with a unit must be in the limits' one, and comes in
    /// its base.
    #[test]
    fn a_band_with_a_unit_is_in_the_limits_one() {
        assert_eq!(
            Hysteresis::parse_in("10 mV", Some("V")),
            Ok(Hysteresis::Absolute(0.01))
        );
        assert_eq!(
            Hysteresis::parse_in("1%", Some("V")),
            Ok(Hysteresis::Percent(1.0))
        );
        assert_eq!(
            Hysteresis::parse_in("2m", None),
            Ok(Hysteresis::Absolute(0.002))
        );
        for (typed, unit) in [("10 mA", Some("V")), ("10 mV", None)] {
            assert_eq!(
                Hysteresis::parse_in(typed, unit),
                Err(HysteresisError::NotTheLimitsUnit),
                "{typed}"
            );
        }
        assert_eq!(
            Hysteresis::parse_in("-1 V", Some("V")),
            Err(HysteresisError::NotABand)
        );
        assert_eq!(
            Hysteresis::parse_in("5 x", Some("V")),
            Err(HysteresisError::NotANumber)
        );
    }

    /// Limits typed in volts wait for a voltage, however many readings of
    /// another quantity come first, and judge it in its base.
    #[test]
    fn a_typed_unit_waits_for_its_quantity() {
        let t = Instant::now();
        let limits = Limits::typed(None, Some((5.0, Some("V")))).unwrap();
        let mut a = Alarm::new(limits, Hysteresis::Auto);
        assert_eq!(a.check(&reading(12.0, "k\u{3a9}", t)), None);
        assert_eq!(a.idle(), Some("\u{3a9}"));
        assert_eq!(a.bound_unit(), None);
        assert_eq!(a.limits_unit(None), Some("V"));
        assert_eq!(a.check(&reading(4980.0, "mV", t)), None);
        assert_eq!((a.idle(), a.bound_unit()), (None, Some("V")));
        let breach = a.check(&reading(5.02, "V", t)).unwrap();
        assert_eq!(breach.note(), "Above high limit 5 V");
        // Bound now: another quantity is a dial turn, as for a bare limit.
        assert_eq!(a.check(&reading(1.0, "\u{3a9}", t)), None);
        assert_eq!(a.idle(), Some("\u{3a9}"));
    }

    /// Under a scale, a typed unit is of the label's quantity, and readings
    /// are judged in its base whatever prefix the label carries.
    #[test]
    fn a_typed_unit_under_a_scale_is_the_labels_quantity() {
        let t = Instant::now();
        let limits = Limits::typed(None, Some((0.5, Some("A")))).unwrap();
        // A 10 mV/A clamp shown in mA: the factor sets the prefix.
        let clamp = Transform::linear(100_000.0, 0.0, Some("mA".into()));
        let mut a = Alarm::new(limits, Hysteresis::Auto);
        let mut m = reading(4.0, "mV", t);
        clamp.apply(&mut m);
        assert_eq!(a.check(&m), None, "400 mA is inside 0.5 A");
        let mut m = reading(6.0, "mV", t);
        clamp.apply(&mut m);
        assert_eq!(a.check(&m).unwrap().note(), "Above high limit 0.5 A");
        assert_eq!(
            limits.in_unit("mA", LimitUnit::Shown).high,
            Some(500.0),
            "the line is drawn at 500 mA"
        );

        // Shown in °F: no amps to judge.
        let to_f = Transform::linear(1.8, 32.0, Some("\u{b0}F".into()));
        let mut a = Alarm::new(limits, Hysteresis::Auto);
        let mut m = reading(20.0, "\u{b0}C", t);
        to_f.apply(&mut m);
        assert_eq!(a.check(&m), None);
        assert_eq!(a.idle(), Some("\u{b0}F"));
    }

    #[test]
    fn clearing_the_counts_keeps_the_quantity() {
        let t = Instant::now();
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert!(a.check(&reading(6.0, "V", t)).is_some());
        a.clear_counts();
        assert_eq!(a.high_count, 0);
        assert_eq!(a.bound_unit(), Some("V"));
        // The reading still out after a clear is the breach it already was.
        assert_eq!(a.check(&reading(6.0, "V", t)), None);
        assert_eq!(a.high_count, 0);
        assert_eq!(a.limits_unit(None), Some("V"));
        // Bound: what the readings bound it to, not a label passed in.
        assert_eq!(a.limits_unit(Some("A")), Some("V"));
        assert_eq!(alarm(None, Some(5.0)).limits_unit(Some("A")), Some("A"));
    }

    /// A file written under `--scale 1000 --unit kV`: its readings carry the
    /// meter's volts as Raw, and the limits are in kV, as they were live.
    #[test]
    fn a_files_scaled_reading_is_judged_as_shown() {
        let t = Instant::now();
        let kv = Transform::linear(1000.0, 0.0, Some("kV".into()));
        let mut m = reading(4.0, "V", t);
        kv.apply(&mut m);
        // Read back from the file: the same reading, no transform applied.
        assert_eq!(LimitUnit::of(&m), LimitUnit::Shown);
        assert_eq!(limit_unit(&m), Some("kV"));
        let mut a = alarm(None, Some(5000.0));
        assert_eq!(a.check(&m), None, "4000 kV is inside 5000");
        let mut m = reading(5.5, "V", t);
        kv.apply(&mut m);
        assert_eq!(a.check(&m).unwrap().note(), "Above high limit 5000 kV");
        assert_eq!(a.limits_unit(None), Some("kV"));
        // An unscaled file is judged in base units.
        let plain = reading(5500.0, "mV", t);
        assert_eq!(LimitUnit::of(&plain), LimitUnit::Base);
        let mut a = alarm(None, Some(5.0));
        assert_eq!(
            a.check(&plain).unwrap().note(),
            "Already above high limit 5 V; alarms start at the next crossing"
        );
    }

    /// A recording that went on through a scale change: limits bound by its
    /// plain volts mean nothing for its scaled amps, though the meter
    /// measured volts throughout.
    #[test]
    fn a_reading_shown_in_another_limit_unit_leaves_the_alarm_idle() {
        let t = Instant::now();
        let clamp = Transform::linear(100.0, 0.0, Some("A".into()));
        let mut a = alarm(None, Some(5.0));
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        let mut m = reading(60.0, "mV", t);
        clamp.apply(&mut m);
        assert_eq!(a.check(&m), None, "6 A against a limit in V");
        assert_eq!((a.idle(), a.bound_unit()), (Some("A"), Some("V")));
        assert_eq!(a.reading_zone(), None);
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert_eq!(a.idle(), None);
    }

    /// A scale with no label shows the base unit, but its limits mean other
    /// values than the meter's own: the two still bind apart.
    #[test]
    fn an_unlabelled_scale_binds_apart_from_the_meters_unit() {
        let t = Instant::now();
        let thousand = Transform::linear(1000.0, 0.0, None);
        let mut scaled = reading(4.0, "V", t);
        thousand.apply(&mut scaled);
        assert_eq!(scaled.unit, "V");
        let mut a = alarm(None, Some(4500.0));
        assert_eq!(a.check(&scaled), None);
        assert_eq!(a.check(&reading(4.0, "V", t)), None);
        assert_eq!(a.idle(), Some("V"), "plain volts against scaled limits");
        let mut plain = alarm(None, Some(5.0));
        plain.check(&reading(4.0, "V", t));
        let (b, s) = (plain.binding().unwrap(), a.binding().unwrap());
        assert_eq!(b.moved_from(s), Some("V"));
        assert_eq!(b.moved_from(b), None);
    }

    #[test]
    fn a_limit_is_in_the_readings_base_unit() {
        let t = Instant::now();
        assert_eq!(limit_unit(&reading(4980.0, "mV", t)), Some("V"));
        assert_eq!(limit_unit(&reading(12.3, "A", t)), Some("A"));
        let flags = StatusFlags::default();
        let auto = Measurement::test_fixture(MeasuredValue::NoReading("Auto"), "", flags);
        assert_eq!(limit_unit(&auto), None);
    }

    #[test]
    fn limits_convert_to_the_shown_unit() {
        let l = Limits::check(Some(0.5), Some(5.0)).unwrap();
        let mv = l.in_unit("mV", LimitUnit::Base);
        assert_eq!((mv.low, mv.high), (Some(500.0), Some(5000.0)));
        assert_eq!(l.in_unit("mV", LimitUnit::Shown), l);
        assert_eq!(l.describe(Hysteresis::Auto), "above 5 or below 0.5");
        assert_eq!(
            l.describe(Hysteresis::Percent(1.0)),
            "above 5 or below 0.5, hysteresis 1%"
        );
    }

    #[test]
    fn limits_convert_back_from_the_shown_unit() {
        let base = |v| limit_from_unit(v, "mV", LimitUnit::Base);
        assert_eq!((base(3.29), base(4321.0)), (0.00329, 4.321));
        assert_eq!(limit_from_unit(4.7, "kΩ", LimitUnit::Base), 4700.0);
        assert_eq!(limit_from_unit(3.29, "mV", LimitUnit::Shown), 3.29);
        let limits = Limits::check(Some(0.00329), Some(4.321)).unwrap();
        let shown = limits.in_unit("mV", LimitUnit::Base);
        assert_eq!(
            (shown.low.map(base), shown.high.map(base)),
            (limits.low, limits.high)
        );
    }
}
