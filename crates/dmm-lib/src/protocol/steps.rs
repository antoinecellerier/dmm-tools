//! The gate steps every family's capture run rests on.
//!
//! Six steps — DC volts open, shorted and reversed, then resistance open,
//! through a body and shorted — pin down the mode byte, the digits, the
//! decimal point, OL and the sign. They are the same experiment on every
//! meter, so the wording lives here once instead of in each family's
//! `capture_steps`, where a fix to it used to mean editing seven files.

use crate::protocol::{CaptureStep, Expect, Need, ValueExpect};

/// Every family's mode table spells resistance the same way.
const OHMS_MODE: &str = "Ω";

/// How a family's mode table spells DC volts. An enum rather than a string
/// because [`CaptureStep::instruction`] is `&'static str`: the follow-up
/// wording has to be picked from constants, not formatted at run time.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Volts {
    /// "DC V" — the UT61+, UT80x and UT880x families, and Voltcraft.
    DcV,
    /// "V DC" — the UT171 and UT181A.
    VDc,
}

impl Volts {
    /// The mode label, as the family's own mode table spells it.
    fn label(self) -> &'static str {
        match self {
            Volts::DcV => "DC V",
            Volts::VDc => "V DC",
        }
    }

    fn shorted(self) -> &'static str {
        match self {
            Volts::DcV => "DC V mode: touch the two probe tips together.",
            Volts::VDc => "V DC mode: touch the two probe tips together.",
        }
    }

    fn reversed(self) -> &'static str {
        match self {
            Volts::DcV => "DC V mode: leads reversed on a battery or any DC source (skip if none).",
            Volts::VDc => "V DC mode: leads reversed on a battery or any DC source (skip if none).",
        }
    }
}

/// How a family's instructions name the resistance mode — the dial legend on
/// the UT61+, the word everywhere else. Same reason for the enum as [`Volts`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ohms {
    /// "Ω mode: …"
    Symbol,
    /// "Resistance mode: …"
    Word,
    /// "At Auto: …", for a meter that picks resistance by what the leads
    /// touch and has no resistance mode to set.
    Auto,
}

impl Ohms {
    fn body(self) -> &'static str {
        match self {
            Ohms::Symbol => {
                "\u{03A9} mode: hold one probe tip between the fingers of each hand \
                 (body resistance, hundreds of k\u{03A9})."
            }
            Ohms::Word => {
                "Resistance mode: hold one probe tip between the fingers of each \
                 hand (body resistance, hundreds of kΩ)."
            }
            Ohms::Auto => {
                "At Auto: hold one probe tip between the fingers of each hand \
                 (body resistance, hundreds of kΩ)."
            }
        }
    }

    fn shorted(self) -> &'static str {
        match self {
            Ohms::Symbol => "\u{03A9} mode: touch the two probe tips together.",
            Ohms::Word => "Resistance mode: touch the two probe tips together.",
            Ohms::Auto => "At Auto: touch the two probe tips together.",
        }
    }
}

/// The six gate steps, in the order they establish the semantics.
///
/// `volts_entry` and `ohms_entry` are the two "turn the dial" steps: each
/// names the meter's own panel legend and carries whatever sample count and
/// verified mark the family uses, so the caller builds them and this adds the
/// gate mark and the expectation. The result is an array so a caller can
/// destructure it and place the volts and ohms trios where its own list puts
/// them — no family has all six in a row.
pub(crate) fn gate_steps(
    volts: Volts,
    volts_entry: CaptureStep,
    ohms: Ohms,
    ohms_entry: CaptureStep,
) -> [CaptureStep; 6] {
    let v = volts.label();
    [
        volts_entry
            .gate()
            .expect(Expect::mode(v).value(ValueExpect::Finite)),
        CaptureStep::basic("dcv_short", volts.shorted())
            .gate()
            .needs(&[Need::ShortedLeads])
            .expect(Expect::mode(v).value(ValueExpect::Finite)),
        CaptureStep::basic("dcv_negative", volts.reversed())
            .gate()
            .needs(&[Need::DcSource])
            .expect(Expect::mode(v).value(ValueExpect::Negative)),
        ohms_entry
            .gate()
            .expect(Expect::mode(OHMS_MODE).value(ValueExpect::Overload)),
        // Open and shorted leads repeat one digit value, so a digit-order
        // or digit-value bug hides; a body reading spreads the digits out.
        CaptureStep::basic("ohm_body", ohms.body())
            .gate()
            .expect(Expect::mode(OHMS_MODE).value(ValueExpect::Finite)),
        CaptureStep::basic("ohm_short", ohms.shorted())
            .gate()
            .needs(&[Need::ShortedLeads])
            .expect(Expect::mode(OHMS_MODE).value(ValueExpect::Finite)),
    ]
}
