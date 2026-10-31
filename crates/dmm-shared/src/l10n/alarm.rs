//! What the library says about a threshold alarm, in the user's language:
//! the limits it watches ([`Limits::describe`]) and the limit a reading
//! breached ([`Breach::note`]), from `i18n/<locale>/lib-alarm.ftl`. A test
//! checks the en-US text against the library's own.
//!
//! The library's English stays the data: a breach's marker note is written
//! into recordings and exports as [`Breach::note`] words it. These are for
//! the lines and notices that say it.

use super::Localizer;
use crate::tr;
use dmm_lib::alarm::{Breach, Hysteresis, Limits, Zone};

impl Localizer {
    /// `limits` and their `band`, as [`Limits::describe`] words them:
    /// "above 5 or below 3, hysteresis 1%".
    pub fn alarm_limits(&self, limits: &Limits, band: Hysteresis) -> String {
        let which = match (limits.high, limits.low) {
            (Some(_), Some(_)) => "both",
            (Some(_), None) => "high",
            (None, Some(_)) => "low",
            (None, None) => return String::new(),
        };
        let number = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
        let watched = tr!(
            self,
            "alarm-limits",
            limits = which,
            high = number(limits.high),
            low = number(limits.low),
        );
        match band.describe() {
            Some(band) => tr!(self, "alarm-hysteresis", limits = watched, band = band).into_owned(),
            None => watched.into_owned(),
        }
    }

    /// The limit `breach` is past, as [`Breach::note`] words it: "Above
    /// high limit 5 V", or for a reading already out when the alarm started
    /// watching it, "Already below low limit 3 V; alarms start at the next
    /// crossing".
    pub fn breach_note(&self, breach: &Breach) -> String {
        let side = match breach.zone {
            Zone::Below => "low",
            Zone::Above | Zone::Inside => "high",
        };
        let limit = if breach.unit.is_empty() {
            breach.limit.to_string()
        } else {
            format!("{} {}", breach.limit, breach.unit)
        };
        if breach.crossing {
            tr!(self, "alarm-breach", side = side, limit = limit).into_owned()
        } else {
            tr!(self, "alarm-breach-already", side = side, limit = limit).into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::LIB_RESOURCES;

    #[test]
    fn english_alarm_text_is_the_librarys() {
        let l = Localizer::en_us(LIB_RESOURCES);
        // An alarm has a limit: without one there is nothing to describe.
        for (low, high) in [
            (Some(-0.25), None),
            (None, Some(5.0)),
            (Some(3.0), Some(4.0005)),
        ] {
            let limits = Limits::check(low, high).expect("valid limits");
            for band in [
                Hysteresis::Auto,
                Hysteresis::Absolute(0.01),
                Hysteresis::Percent(1.0),
            ] {
                assert_eq!(l.alarm_limits(&limits, band), limits.describe(band));
            }
        }
        for zone in [Zone::Above, Zone::Below, Zone::Inside] {
            for crossing in [true, false] {
                for unit in ["", "V"] {
                    let breach = Breach {
                        zone,
                        limit: 1.5,
                        unit: unit.to_string(),
                        crossing,
                    };
                    assert_eq!(l.breach_note(&breach), breach.note());
                }
            }
        }
    }
}
