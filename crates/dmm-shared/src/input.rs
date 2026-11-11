//! Why a typed number was refused, worded once for both binaries.
//!
//! The rules live in `dmm-lib` ([`Transform`](dmm_lib::transform::Transform),
//! [`Limits`](dmm_lib::alarm::Limits), [`Hysteresis`](dmm_lib::alarm::Hysteresis)); this is only
//! their wording, from `i18n/<locale>/shared.ftl`. Each function gives the
//! reason alone, lower-case and with no full stop. The framing stays with
//! the binary that knows where the number came from: the GUI's toast names
//! the field ("Invalid scale: …"), clap names the flag and the value it was
//! given.

use crate::l10n::Localizer;
use crate::tr;
use dmm_lib::alarm::{HysteresisError, LimitError, LimitParseError};
use dmm_lib::transform::FactorError;
use std::borrow::Cow;

/// `text` did not parse as a number at all.
pub fn not_a_number<'a>(l: &'a Localizer, text: &str) -> Cow<'a, str> {
    tr!(l, "input-not-a-number", text = text)
}

/// Why a scale or offset was refused.
pub fn factor_error(l: &Localizer, e: FactorError) -> Cow<'_, str> {
    match e {
        FactorError::NotFinite => tr!(l, "input-not-finite"),
        FactorError::ZeroScale => tr!(l, "input-zero-scale"),
    }
}

/// Why an alarm limit, or the pair of them, was refused.
pub fn limit_error(l: &Localizer, e: LimitError) -> Cow<'_, str> {
    match e {
        LimitError::NotFinite => tr!(l, "input-not-finite"),
        LimitError::LowNotBelowHigh => tr!(l, "input-low-not-below-high"),
        LimitError::UnitsDiffer => tr!(l, "input-limit-units-differ"),
    }
}

/// Why an alarm limit typed as `text` was refused.
pub fn limit_parse_error<'a>(l: &'a Localizer, e: LimitParseError, text: &str) -> Cow<'a, str> {
    match e {
        LimitParseError::NotAQuantity => tr!(l, "input-not-a-quantity", text = text),
        LimitParseError::Limit(e) => limit_error(l, e),
    }
}

/// Why a hysteresis band typed as `text` was refused.
pub fn hysteresis_error<'a>(l: &'a Localizer, e: HysteresisError, text: &str) -> Cow<'a, str> {
    match e {
        HysteresisError::NotANumber => tr!(l, "input-band-not-a-number", text = text),
        HysteresisError::NotABand | HysteresisError::PercentOfZero => {
            tr!(l, "input-band-negative")
        }
        HysteresisError::NotTheLimitsUnit => tr!(l, "input-band-not-the-limits-unit"),
    }
}

/// Why a percentage band can't go with limits of zero.
pub fn percent_of_zero(l: &Localizer) -> Cow<'_, str> {
    tr!(l, "input-percent-of-zero")
}
