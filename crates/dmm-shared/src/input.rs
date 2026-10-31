//! Why a typed number was refused, worded once for both binaries.
//!
//! The rules live in `dmm-lib` ([`Transform`](dmm_lib::transform::Transform),
//! [`Limits`](dmm_lib::alarm::Limits), [`Hysteresis`](dmm_lib::alarm::Hysteresis)); this is only
//! their wording. Each function gives the reason alone, lower-case and with
//! no full stop. The framing stays with the binary that knows where the
//! number came from: the GUI's toast names the field ("Invalid scale: …"),
//! clap names the flag and the value it was given.

use dmm_lib::alarm::{HysteresisError, LimitError};
use dmm_lib::transform::FactorError;

/// `text` did not parse as a number at all.
pub fn not_a_number(text: &str) -> String {
    format!("\u{201c}{text}\u{201d} is not a number")
}

/// Why a scale or offset was refused.
pub fn factor_error(e: FactorError) -> &'static str {
    match e {
        FactorError::NotFinite => FINITE,
        FactorError::ZeroScale => "a scale of zero would erase the reading",
    }
}

/// Why an alarm limit, or the pair of them, was refused.
pub fn limit_error(e: LimitError) -> &'static str {
    match e {
        LimitError::NotFinite => FINITE,
        LimitError::LowNotBelowHigh => "the low limit must be below the high one",
    }
}

/// Why a hysteresis band typed as `text` was refused.
pub fn hysteresis_error(e: HysteresisError, text: &str) -> String {
    match e {
        HysteresisError::NotANumber => {
            format!("\u{201c}{text}\u{201d} is not a number or a percentage such as 1%")
        }
        HysteresisError::NotABand | HysteresisError::PercentOfZero => {
            "must be zero or more".to_string()
        }
    }
}

/// Why a percentage band can't go with limits of zero.
pub const PERCENT_OF_ZERO: &str = "a percentage of a limit of 0 is 0; give a value instead";

const FINITE: &str = "must be a finite number";
