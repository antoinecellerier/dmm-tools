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
}

impl ValueExpect {
    /// How the mismatch line names this expectation.
    fn describe(self) -> &'static str {
        match self {
            ValueExpect::Overload => "OL",
            ValueExpect::Negative => "a negative reading",
            ValueExpect::Finite => "a numeric reading",
            ValueExpect::NcvDetected => "an NCV level of 1 or more",
        }
    }

    fn matches(self, value: &MeasuredValue) -> bool {
        match (self, value) {
            (ValueExpect::Overload, MeasuredValue::Overload) => true,
            (ValueExpect::Negative, MeasuredValue::Normal(v)) => v.is_finite() && *v < 0.0,
            (ValueExpect::Finite, MeasuredValue::Normal(v)) => v.is_finite(),
            (ValueExpect::NcvDetected, MeasuredValue::NcvLevel(level)) => *level >= 1,
            _ => false,
        }
    }
}

/// What a correctly parsed reading looks like for one capture step.
///
/// Every field is optional: a step asserts only what its instruction actually
/// pins down. An empty [`Expect`] asserts nothing and always passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        }
    }

    /// An expectation on the mode label, as the family's own mode table
    /// spells it.
    pub const fn mode(label: &'static str) -> Self {
        Self {
            mode: Some(label),
            flags: &[],
            range: None,
            value: None,
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

    /// Check a reading, returning a one-line reason for the first mismatch.
    pub fn check(&self, m: &Measurement) -> Result<(), String> {
        if let Some(want) = self.mode
            && m.mode != want
        {
            return Err(format!("mode is {:?}, want {:?}", m.mode, want));
        }

        for &(flag, want) in self.flags {
            let got = m.flags.get(flag);
            if got != want {
                return Err(format!(
                    "{} is {}, want {}",
                    flag.name(),
                    on_off(got),
                    on_off(want)
                ));
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
                    (true, _) => "(none)".to_string(),
                    (false, true) if label != AUTO_RANGE_LABEL => format!("{label} (auto)"),
                    _ => label.to_string(),
                };
                let want = match want {
                    RangeExpect::Auto => "auto",
                    RangeExpect::Manual => "manual",
                };
                return Err(format!("range is {got}, want {want}"));
            }
        }

        if let Some(want) = self.value
            && !want.matches(&m.value)
        {
            return Err(format!(
                "value is {}, want {}",
                m.value_export_str(),
                want.describe()
            ));
        }

        Ok(())
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

    #[test]
    fn empty_expect_accepts_anything() {
        assert_eq!(Expect::new().check(&dcv()), Ok(()));
    }

    #[test]
    fn mode_mismatch_names_both_labels() {
        assert_eq!(Expect::mode("DC V").check(&dcv()), Ok(()));
        assert_eq!(
            Expect::mode("AC V").check(&dcv()),
            Err(r#"mode is "DC V", want "AC V""#.to_string())
        );
    }

    #[test]
    fn flag_mismatch_reads_on_off() {
        const HOLD_ON: &[(Flag, bool)] = &[(Flag::Hold, true)];
        const HOLD_OFF: &[(Flag, bool)] = &[(Flag::Hold, false)];

        assert_eq!(Expect::new().flags(HOLD_OFF).check(&dcv()), Ok(()));
        assert_eq!(
            Expect::new().flags(HOLD_ON).check(&dcv()),
            Err("hold is off, want on".to_string())
        );

        let held = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_HOLD, 0, 0));
        assert_eq!(Expect::new().flags(HOLD_ON).check(&held), Ok(()));
        assert_eq!(
            Expect::new().flags(HOLD_OFF).check(&held),
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
            Expect::new().range(RangeExpect::Manual).check(&auto),
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
            Expect::new().range(RangeExpect::Manual).check(&rung_auto),
            Err("range is 22V (auto), want manual".to_string())
        );

        let manual = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, MANUAL, 0));
        assert_eq!(manual.range_label, "22V");
        assert_eq!(
            Expect::new().range(RangeExpect::Manual).check(&manual),
            Ok(())
        );
        assert_eq!(
            Expect::new().range(RangeExpect::Auto).check(&manual),
            Err("range is 22V, want auto".to_string())
        );

        let unlabelled = Measurement {
            range_label: "".into(),
            ..dcv()
        };
        assert_eq!(
            Expect::new().range(RangeExpect::Manual).check(&unlabelled),
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
            Expect::new().value(ValueExpect::Overload).check(&finite),
            Err("value is 12.345, want OL".to_string())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::Negative).check(&finite),
            Err("value is 12.345, want a negative reading".to_string())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::Finite).check(&overload),
            Err("value is OL, want a numeric reading".to_string())
        );
    }

    #[test]
    fn ncv_level_is_not_a_finite_reading() {
        // NCV level 2: not a number the value classes accept.
        let ncv = make_test_measurement(0x14, 0x00, b"      2", (0, 0), (0, 0, 0));
        assert!(matches!(ncv.value, MeasuredValue::NcvLevel(2)));
        assert_eq!(
            Expect::new().value(ValueExpect::Finite).check(&ncv),
            Err("value is NCV:2, want a numeric reading".to_string())
        );
        assert_eq!(
            Expect::new().value(ValueExpect::NcvDetected).check(&ncv),
            Ok(())
        );

        // NCV with nothing near the probe: the display carries no digit, so
        // the parser reports level 0 and the step has not been carried out.
        for display in [b"     - ", b"   EF  "] {
            let idle = make_test_measurement(0x14, 0x00, display, (0, 0), (0, 0, 0));
            assert!(matches!(idle.value, MeasuredValue::NcvLevel(0)));
            assert_eq!(
                Expect::new().value(ValueExpect::NcvDetected).check(&idle),
                Err("value is NCV:0, want an NCV level of 1 or more".to_string())
            );
        }
    }

    #[test]
    fn mode_is_checked_before_value() {
        let overload = make_test_measurement(0x06, 0x01, b"     OL", (0, 0), (0, 0, 0));
        assert_eq!(
            Expect::mode("DC V")
                .value(ValueExpect::Finite)
                .check(&overload),
            Err(r#"mode is "Ω", want "DC V""#.to_string())
        );
    }
}
