//! What a correctly parsed reading looks like once a capture step's
//! instruction has been carried out.
//!
//! A capture step describes what the user does; an [`Expect`] describes what
//! the parser must then produce. Checking the two against each other turns a
//! capture session into a test of the protocol implementation rather than a
//! pile of samples someone has to read by eye.

use crate::flags::Flag;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::AUTO_RANGE_LABEL;
use crate::transform::si_prefix;
use std::borrow::Cow;

/// Whether the meter should be autoranging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeExpect {
    /// `range_label` is [`AUTO_RANGE_LABEL`].
    Auto,
    /// `range_label` names a specific rung — any non-empty label that is not
    /// the autorange one.
    Manual,
}

/// The class of value the reading must fall into. Deliberately coarse: the
/// exact number depends on what the user has the probes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueExpect {
    /// The meter is showing OL.
    Overload,
    /// A numeric reading below zero.
    Negative,
    /// A numeric reading — not OL, not an NCV level.
    Finite,
    /// An NCV level of at least 1: the meter found a live wire. Level 0 is
    /// what NCV shows the moment the dial reaches it.
    NcvDetected,
    /// A word the meter shows instead of a reading, such as `Auto`.
    NoReading,
    /// A numeric reading of exactly zero: what a ZERO key leaves on screen.
    Zero,
}

impl ValueExpect {
    /// How the mismatch line names this expectation.
    fn describe(self) -> &'static str {
        match self {
            ValueExpect::Overload => "OL",
            ValueExpect::Negative => "a negative reading",
            ValueExpect::Finite => "a numeric reading",
            ValueExpect::NcvDetected => "an NCV level of 1 or more",
            ValueExpect::NoReading => "a word instead of a reading",
            ValueExpect::Zero => "a reading of zero",
        }
    }

    fn matches(self, value: &MeasuredValue) -> bool {
        match (self, value) {
            (ValueExpect::Overload, MeasuredValue::Overload) => true,
            (ValueExpect::Negative, MeasuredValue::Normal(v)) => v.is_finite() && *v < 0.0,
            (ValueExpect::Finite, MeasuredValue::Normal(v)) => v.is_finite(),
            (ValueExpect::NcvDetected, MeasuredValue::NcvLevel(level)) => *level >= 1,
            // `?` is text nothing recognised, not the meter's word.
            (ValueExpect::NoReading, v @ MeasuredValue::NoReading(_)) => !v.is_unrecognised(),
            (ValueExpect::Zero, MeasuredValue::Normal(v)) => *v == 0.0,
            _ => false,
        }
    }
}

/// What a correctly parsed reading looks like for one capture step.
///
/// Every field is optional: a step asserts only what its instruction actually
/// pins down. An empty [`Expect`] asserts nothing and always passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Expect {
    /// `Measurement::mode`, matched exactly. Only set where the label can be
    /// read off the family's own mode table.
    pub mode: Option<&'static str>,
    /// Flags that must hold the given value; flags not listed are ignored.
    pub flags: &'static [(Flag, bool)],
    /// Autorange or manual, from `Measurement::range_label`.
    pub range: Option<RangeExpect>,
    /// The class the reading must fall into.
    pub value: Option<ValueExpect>,
    /// The magnitude a numeric reading must reach, in the mode's base unit
    /// whatever prefix the meter shows: what tells a battery on the probes
    /// from open-lead noise, which reads millivolts.
    pub at_least: Option<f64>,
}

impl Default for Expect {
    fn default() -> Self {
        Self::new()
    }
}

impl Expect {
    /// An expectation that asserts nothing yet — the start of a builder chain
    /// for a step that pins down flags or range but not the mode label.
    pub const fn new() -> Self {
        Self {
            mode: None,
            flags: &[],
            range: None,
            value: None,
            at_least: None,
        }
    }

    /// An expectation on the mode label, as the family's own mode table
    /// spells it.
    pub const fn mode(label: &'static str) -> Self {
        Self {
            mode: Some(label),
            ..Self::new()
        }
    }

    /// Require these flags to hold the given values.
    pub const fn flags(mut self, flags: &'static [(Flag, bool)]) -> Self {
        self.flags = flags;
        self
    }

    /// Require autorange or manual range.
    pub const fn range(mut self, range: RangeExpect) -> Self {
        self.range = Some(range);
        self
    }

    /// Require the reading to fall into `value`'s class.
    pub const fn value(mut self, value: ValueExpect) -> Self {
        self.value = Some(value);
        self
    }

    /// Require a numeric reading of at least `magnitude`, sign aside, in the
    /// mode's base unit.
    pub const fn at_least(mut self, magnitude: f64) -> Self {
        self.at_least = Some(magnitude);
        self
    }

    /// Check a reading, returning the first way it falls short.
    pub fn check(&self, m: &Measurement) -> Result<(), Mismatch> {
        if let Some(want) = self.mode
            && m.mode != want
        {
            return Err(Mismatch::Mode {
                got: m.mode.clone(),
                want,
            });
        }

        for &(flag, want) in self.flags {
            if m.flags.get(flag) != want {
                return Err(Mismatch::Flag { flag, want });
            }
        }

        if let Some(want) = self.range {
            // The UT61+ labels the ladder rung even while auto-ranging and
            // says so with the AUTO flag; other families label "Auto".
            let label = m.range_label.as_ref();
            let auto = m.flags.auto_range || label == AUTO_RANGE_LABEL;
            let ok = match want {
                RangeExpect::Auto => auto,
                RangeExpect::Manual => !auto && !label.is_empty(),
            };
            if !ok {
                let got = match (label.is_empty(), auto) {
                    (true, _) => RangeShown::Unlabelled,
                    (false, true) if label != AUTO_RANGE_LABEL => {
                        RangeShown::AutoRung(m.range_label.clone())
                    }
                    _ => RangeShown::Label(m.range_label.clone()),
                };
                return Err(Mismatch::Range { got, want });
            }
        }

        if let Some(want) = self.value
            && !want.matches(&m.value)
        {
            return Err(Mismatch::Value {
                got: shown(m),
                want,
            });
        }

        if let Some(min) = self.at_least {
            let (base, mult) = si_prefix(&m.unit);
            let big_enough = match m.value {
                MeasuredValue::Normal(v) => v.is_finite() && (v * mult).abs() >= min,
                _ => false,
            };
            if !big_enough {
                let got = match (&m.value, shown(m)) {
                    (MeasuredValue::Normal(_), Shown::Text(text)) if !m.unit.is_empty() => {
                        Shown::Text(format!("{text} {}", m.unit))
                    }
                    (_, got) => got,
                };
                return Err(Mismatch::AtLeast {
                    got,
                    min,
                    unit: base.to_string(),
                });
            }
        }

        Ok(())
    }
}

/// How a reading fell short of an [`Expect`]: the first field that did not
/// hold, with what the reading had instead. `Display` words it in English,
/// as a one-line reason; an app that speaks another language words it from
/// the fields.
#[derive(Debug, Clone, PartialEq)]
pub enum Mismatch {
    /// The mode label is `got`, not `want`.
    Mode {
        got: Cow<'static, str>,
        want: &'static str,
    },
    /// `flag` is not `want`: the reading has it the other way.
    Flag { flag: Flag, want: bool },
    /// The range, `got`, is not `want`.
    Range { got: RangeShown, want: RangeExpect },
    /// The value, `got`, is not in `want`'s class.
    Value { got: Shown, want: ValueExpect },
    /// The value, `got`, is not a number of at least `min` either way, `min`
    /// being in `unit`, the mode's base unit ("" for a reading without
    /// one). A number in `got` carries the reading's own unit.
    AtLeast { got: Shown, min: f64, unit: String },
}

/// The range as a failed check names it.
#[derive(Debug, Clone, PartialEq)]
pub enum RangeShown {
    /// The reading has no range label.
    Unlabelled,
    /// The rung autoranging picked, as a family that labels it while
    /// autoranging says (the UT61+, with its AUTO flag).
    AutoRung(Cow<'static, str>),
    /// The range label as it reads: a manual rung, or the autorange label.
    Label(Cow<'static, str>),
}

/// The value as a failed check names it.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// The export form, which reads back as the number the parser produced,
    /// or the word a meter shows instead of a reading, whose export form is
    /// empty.
    Text(String),
    /// The frame has no main reading.
    NoMainReading,
}

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shown::Text(text) => f.write_str(text),
            Shown::NoMainReading => f.write_str("no main reading"),
        }
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mismatch::Mode { got, want } => write!(f, "mode is {got:?}, want {want:?}"),
            Mismatch::Flag { flag, want } => write!(
                f,
                "{} is {}, want {}",
                flag.name(),
                on_off(!want),
                on_off(*want)
            ),
            Mismatch::Range { got, want } => {
                let want = match want {
                    RangeExpect::Auto => "auto",
                    RangeExpect::Manual => "manual",
                };
                match got {
                    RangeShown::Unlabelled => write!(f, "range is (none), want {want}"),
                    RangeShown::AutoRung(label) => {
                        write!(f, "range is {label} (auto), want {want}")
                    }
                    RangeShown::Label(label) => write!(f, "range is {label}, want {want}"),
                }
            }
            Mismatch::Value { got, want } => {
                write!(f, "value is {got}, want {}", want.describe())
            }
            Mismatch::AtLeast { got, min, unit } => {
                let min = with_unit(*min, unit);
                write!(f, "value is {got}, want at least {min} either way")
            }
        }
    }
}

/// The value as a failed check names it ([`Shown`]).
fn shown(m: &Measurement) -> Shown {
    match m.value {
        MeasuredValue::NoReading(word) => Shown::Text(word.to_string()),
        MeasuredValue::Absent => Shown::NoMainReading,
        _ => Shown::Text(m.value_export_str().into_owned()),
    }
}

/// `value` followed by `unit`, or alone when there is none.
fn with_unit(value: f64, unit: &str) -> String {
    if unit.is_empty() {
        value.to_string()
    } else {
        format!("{value} {unit}")
    }
}

fn on_off(set: bool) -> &'static str {
    if set { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::make_test_measurement;

    const MANUAL: u8 = 0x04; // flag2 bit 2: manual range
    const F_HOLD: u8 = 0x02; // flag1 bit 1: HOLD

    /// DC V, 12.345, autorange, no flags set.
    fn dcv() -> Measurement {
        make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0))
    }

    /// A check's outcome with the mismatch as `Display` words it.
    fn reason(checked: Result<(), Mismatch>) -> Result<(), String> {
        checked.map_err(|e| e.to_string())
    }

    #[test]
    fn empty_expect_accepts_anything() {
        assert_eq!(Expect::new().check(&dcv()), Ok(()));
    }

    #[test]
    fn mode_mismatch_names_both_labels() {
        assert_eq!(Expect::mode("DC V").check(&dcv()), Ok(()));
        assert_eq!(
            reason(Expect::mode("AC V").check(&dcv())),
            Err(r#"mode is "DC V", want "AC V""#.to_string())
        );
    }

    #[test]
    fn flag_mismatch_reads_on_off() {
        const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
        const HOLD_OFF: &[(Flag, bool)] = &[(Flag::Hold, false)];

        assert_eq!(Expect::new().flags(HOLD_OFF).check(&dcv()), Ok(()));
        assert_eq!(
            reason(Expect::new().flags(HOLD_ON).check(&dcv())),
            Err("hold is off, want on".to_string())
        );

        let held = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_HOLD, 0, 0));
        assert_eq!(Expect::new().flags(HOLD_ON).check(&held), Ok(()));
        assert_eq!(
            reason(Expect::new().flags(HOLD_OFF).check(&held)),
            Err("hold is on, want off".to_string())
        );
    }

    #[test]
    fn range_auto_and_manual() {
        // Families that label "Auto".
        let auto = Measurement {
            range_label: AUTO_RANGE_LABEL.into(),
            ..dcv()
        };
        assert_eq!(Expect::new().range(RangeExpect::Auto).check(&auto), Ok(()));
        assert_eq!(
            reason(Expect::new().range(RangeExpect::Manual).check(&auto)),
            Err("range is Auto, want manual".to_string())
        );

        // The UT61+ labels the rung while auto-ranging; the AUTO flag decides.
        let rung_auto = dcv();
        assert!(rung_auto.flags.auto_range);
        assert_eq!(rung_auto.range_label, "22V");
        assert_eq!(
            Expect::new().range(RangeExpect::Auto).check(&rung_auto),
            Ok(())
        );
        assert_eq!(
            reason(Expect::new().range(RangeExpect::Manual).check(&rung_auto)),
            Err("range is 22V (auto), want manual".to_string())
        );

        let manual = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, MANUAL, 0));
        assert_eq!(manual.range_label, "22V");
        assert_eq!(
            Expect::new().range(RangeExpect::Manual).check(&manual),
            Ok(())
        );
        assert_eq!(
            reason(Expect::new().range(RangeExpect::Auto).check(&manual)),
            Err("range is 22V, want auto".to_string())
        );

        let unlabelled = Measurement {
            range_label: "".into(),
            ..dcv()
        };
        assert_eq!(
            reason(Expect::new().range(RangeExpect::Manual).check(&unlabelled)),
            Err("range is (none), want manual".to_string())
        );
    }

    #[test]
    fn value_classes() {
        let finite = dcv();
        let negative = make_test_measurement(0x02, 0x01, b"-12.345", (0, 0), (0, 0, 0));
        // Ω with open leads.
        let overload = make_test_measurement(0x06, 0x01, b"     OL", (0, 0), (0, 0, 0));

        assert_eq!(
            Expect::new().value(ValueExpect::Finite).check(&finite),
            Ok(())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::Negative).check(&negative),
            Ok(())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::Overload).check(&overload),
            Ok(())
        );

        assert_eq!(
            reason(Expect::new().value(ValueExpect::Overload).check(&finite)),
            Err("value is 12.345, want OL".to_string())
        );
        assert_eq!(
            reason(Expect::new().value(ValueExpect::Negative).check(&finite)),
            Err("value is 12.345, want a negative reading".to_string())
        );
        let zero = make_test_measurement(0x02, 0x01, b" 0.0000", (0, 0), (0, 0, 0));
        assert_eq!(Expect::new().value(ValueExpect::Zero).check(&zero), Ok(()));
        assert_eq!(
            reason(Expect::new().value(ValueExpect::Zero).check(&finite)),
            Err("value is 12.345, want a reading of zero".to_string())
        );
        assert!(
            Expect::new()
                .value(ValueExpect::Zero)
                .check(&overload)
                .is_err()
        );
        assert_eq!(
            reason(Expect::new().value(ValueExpect::Finite).check(&overload)),
            Err("value is OL, want a numeric reading".to_string())
        );

        let mut idle = finite.clone();
        idle.value = MeasuredValue::NoReading("Auto");
        assert_eq!(
            reason(Expect::new().value(ValueExpect::Finite).check(&idle)),
            Err("value is Auto, want a numeric reading".to_string())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::NoReading).check(&idle),
            Ok(())
        );
        let mut unread = finite.clone();
        unread.value = MeasuredValue::UNRECOGNISED;
        assert_eq!(
            reason(Expect::new().value(ValueExpect::NoReading).check(&unread)),
            Err("value is ?, want a word instead of a reading".to_string())
        );
        assert_eq!(
            reason(Expect::new().value(ValueExpect::NoReading).check(&finite)),
            Err(format!(
                "value is {}, want a word instead of a reading",
                finite.value
            ))
        );
    }

    #[test]
    fn ncv_level_is_not_a_finite_reading() {
        // NCV level 2: not a number the value classes accept.
        let ncv = make_test_measurement(0x14, 0x00, b"     --", (0, 0), (0, 0, 0));
        assert!(matches!(ncv.value, MeasuredValue::NcvLevel(2)));
        assert_eq!(
            reason(Expect::new().value(ValueExpect::Finite).check(&ncv)),
            Err("value is NCV:2, want a numeric reading".to_string())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::NcvDetected).check(&ncv),
            Ok(())
        );

        // NCV with nothing near the probe shows "EF" (level 0); one "-"
        // segment is the meter's first detection level.
        let idle = make_test_measurement(0x14, 0x00, b"   EF  ", (0, 0), (0, 0, 0));
        assert!(matches!(idle.value, MeasuredValue::NcvLevel(0)));
        assert_eq!(
            reason(Expect::new().value(ValueExpect::NcvDetected).check(&idle)),
            Err("value is NCV:0, want an NCV level of 1 or more".to_string())
        );
        let one_bar = make_test_measurement(0x14, 0x00, b"     - ", (0, 0), (0, 0, 0));
        assert!(matches!(one_bar.value, MeasuredValue::NcvLevel(1)));
        assert_eq!(
            Expect::new()
                .value(ValueExpect::NcvDetected)
                .check(&one_bar),
            Ok(())
        );
    }

    /// Open leads read a few millivolts either way; a battery reads volts.
    #[test]
    fn at_least_tells_a_source_from_lead_noise() {
        let noise = make_test_measurement(0x02, 0x01, b"-0.0013", (0, 0), (0, 0, 0));
        let battery = make_test_measurement(0x02, 0x01, b"-1.6105", (0, 0), (0, 0, 0));
        let overload = make_test_measurement(0x06, 0x01, b"     OL", (0, 0), (0, 0, 0));
        let want = Expect::new().value(ValueExpect::Negative).at_least(1.0);
        assert_eq!(want.check(&battery), Ok(()));
        // Noise on the millivolt rung is still millivolts, however big its
        // digits.
        let millivolts = Measurement {
            value: MeasuredValue::Normal(-5.2),
            unit: "mV".into(),
            display_raw: Some("-5.2".to_string()),
            ..noise.clone()
        };
        assert_eq!(
            reason(want.check(&millivolts)),
            Err("value is -5.2 mV, want at least 1 V either way".to_string())
        );
        let kilovolt = Measurement {
            value: MeasuredValue::Normal(-0.0016),
            unit: "kV".into(),
            display_raw: Some("-0.0016".to_string()),
            ..noise.clone()
        };
        assert_eq!(want.check(&kilovolt), Ok(()));
        assert_eq!(
            reason(want.check(&noise)),
            Err("value is -0.0013 V, want at least 1 V either way".to_string())
        );
        assert_eq!(
            reason(Expect::new().at_least(1.0).check(&overload)),
            Err("value is OL, want at least 1 Ω either way".to_string())
        );
    }

    /// Each mismatch carries what the reading had, for an app to word it
    /// in its own language.
    #[test]
    fn mismatches_carry_what_the_reading_had() {
        const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
        assert_eq!(
            Expect::mode("AC V").check(&dcv()),
            Err(Mismatch::Mode {
                got: "DC V".into(),
                want: "AC V"
            })
        );
        assert_eq!(
            Expect::new().flags(HOLD_ON).check(&dcv()),
            Err(Mismatch::Flag {
                flag: Flag::Hold,
                want: true
            })
        );
        assert_eq!(
            Expect::new().range(RangeExpect::Manual).check(&dcv()),
            Err(Mismatch::Range {
                got: RangeShown::AutoRung("22V".into()),
                want: RangeExpect::Manual
            })
        );
        assert_eq!(
            Expect::new().value(ValueExpect::Negative).check(&dcv()),
            Err(Mismatch::Value {
                got: Shown::Text("12.345".to_string()),
                want: ValueExpect::Negative
            })
        );
        let absent = Measurement {
            value: MeasuredValue::Absent,
            ..dcv()
        };
        let checked = Expect::new().at_least(1.0).check(&absent);
        assert_eq!(
            checked,
            Err(Mismatch::AtLeast {
                got: Shown::NoMainReading,
                min: 1.0,
                unit: "V".to_string(),
            })
        );
        assert_eq!(
            reason(checked),
            Err("value is no main reading, want at least 1 V either way".to_string())
        );
    }

    #[test]
    fn mode_is_checked_before_value() {
        let overload = make_test_measurement(0x06, 0x01, b"     OL", (0, 0), (0, 0, 0));
        assert_eq!(
            reason(
                Expect::mode("DC V")
                    .value(ValueExpect::Finite)
                    .check(&overload)
            ),
            Err(r#"mode is "Ω", want "DC V""#.to_string())
        );
    }
}
