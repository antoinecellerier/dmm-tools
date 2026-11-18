//! The spec-table lookup shared by the UNI-T UT71 entries.
//!
//! The UT71A/B/C/D/E operating manual (P/N:110401111273X, 2022.8.18 REV.3)
//! prints one set of tables, A to L, with a column per model: UT71A, UT71B
//! and UT71C/D/E. Each model's file (`specs_ut71a`, `specs_ut71b`,
//! `specs_ut71cde`) holds its column in the same parts, split as the UT804's
//! are: a table whose rows belong to different modes or need a different
//! overload protection is split into parts of the same name.
//!
//! Every UT71 table is the display's full count: 40000 on the UT71C/D/E,
//! 20000 on the UT71A/B. The manual prints nothing for the 4000-count display
//! the blue key held at power-on gives (ut71 spec §3.1), so a reading on it
//! takes its table's mode data but no row.

use super::{CountMode, Coupling};
use crate::specs::ModeSpecs;

/// One model's column of the manual's tables.
pub(super) struct Ut71Tables {
    /// Every part, in manual order.
    pub(super) all: &'static [&'static ModeSpecs],
    pub(super) dc_mv: &'static ModeSpecs,
    pub(super) dc_v: &'static ModeSpecs,
    pub(super) ac_v: &'static ModeSpecs,
    pub(super) dc_ua: &'static ModeSpecs,
    pub(super) dc_ma: &'static ModeSpecs,
    pub(super) dc_a: &'static ModeSpecs,
    pub(super) ac_ua: &'static ModeSpecs,
    pub(super) ac_ma: &'static ModeSpecs,
    pub(super) ac_a: &'static ModeSpecs,
    pub(super) resistance: &'static ModeSpecs,
    pub(super) continuity: &'static ModeSpecs,
    pub(super) diode: &'static ModeSpecs,
    pub(super) capacitance: &'static ModeSpecs,
    pub(super) frequency: &'static ModeSpecs,
    /// J. Temperature, °C and °F: the UT71B/C/D/E's.
    pub(super) temp_c: Option<&'static ModeSpecs>,
    pub(super) temp_f: Option<&'static ModeSpecs>,
    /// K. 4~20 mA loop current: the UT71B/C/D/E's.
    pub(super) loop_current: Option<&'static ModeSpecs>,
    /// L. Power Measurement: the UT71E's.
    pub(super) power: Option<&'static ModeSpecs>,
}

/// The table in `t` for a reading with mode code `mode`, its coupling and
/// whether it is a duty cycle (`Ut804Fields`).
///
/// `None` where the manual has no table: duty cycle (the frequency table
/// prints none), AC or AC+DC millivolts (no UT71 document has an AC mV
/// function, ut71 spec §3.2), and temperature, 4-20 mA and power on a model
/// whose column lacks them.
pub(super) fn table(
    t: &Ut71Tables,
    mode: u8,
    coupling: Option<Coupling>,
    duty: bool,
) -> Option<&'static ModeSpecs> {
    use Coupling::{Ac, AcDc, Dc};
    Some(match (mode, coupling) {
        (0xC, _) if duty => return None,
        (0x1 | 0x2, Some(Dc)) => t.dc_v,
        (0x1 | 0x2, Some(Ac | AcDc)) => t.ac_v,
        (0x3, Some(Dc)) => t.dc_mv,
        (0x7, Some(Dc)) => t.dc_ua,
        (0x8, Some(Dc)) => t.dc_ma,
        (0x9, Some(Dc)) => t.dc_a,
        (0x7, Some(Ac | AcDc)) => t.ac_ua,
        (0x8, Some(Ac | AcDc)) => t.ac_ma,
        (0x9, Some(Ac | AcDc)) => t.ac_a,
        (0x4, _) => t.resistance,
        (0xA, _) => t.continuity,
        (0xB, _) => t.diode,
        (0x5, _) => t.capacitance,
        (0xC, _) => t.frequency,
        (0x6, _) => t.temp_c?,
        (0xD, _) => t.temp_f?,
        (0xF, _) => t.loop_current?,
        (0xE, _) => t.power?,
        _ => return None,
    })
}

/// Whether a reading with this coupling and count mode takes its row's
/// accuracy. An AC+DC reading takes B's or D's mode data only: their remarks
/// add (1%+35 digits) to the table's accuracy, a sum the manual does not
/// print. Nor does it print the 4000-count display, and an overload or LO
/// packet does not say which display it came from.
pub(super) fn has_rows(coupling: Option<Coupling>, counts: Option<CountMode>) -> bool {
    coupling != Some(Coupling::AcDc) && counts == Some(CountMode::Full)
}
