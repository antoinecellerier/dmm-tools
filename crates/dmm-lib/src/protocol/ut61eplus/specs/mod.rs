//! Specification data for the UT61+/UT161 family, as manual tables keyed by
//! range byte.
//!
//! One manual covers the UT61B+, UT61D+ and UT61E+: the UT61+ Series User
//! Manual, "IX. Specifications" (PDF pages 14-18, printed 25-34), prints a
//! UT61E+ table and a UT61B+/UT61D+ table for most functions, with rows and
//! notes tagged for one model. The UT61E+ tables are in `ut61e_plus`, the
//! UT61B+/UT61D+ ones in `ut61bd_plus`; the continuity and diode table,
//! which the three models share, is here.
//!
//! The UT161 Series User Manual (P/N:110401109612X) prints the same pages
//! but for the fuses of the current ranges ("9) DC Current", PDF p. 17), so
//! the UT161B/D/E take their UT61+ counterpart's tables, with twins of the
//! current parts that carry the UT161's fuses.

mod ut61bd_plus;
mod ut61e_plus;

#[cfg(test)]
mod tests;

use crate::measurement::Measurement;
use crate::protocol::ut61eplus::mode::Mode;
use crate::specs::{ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo, SpecSheetTable};

/// Which model's column of the manual a reading takes its specs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecModel {
    Ut61ePlus,
    Ut61bPlus,
    Ut61dPlus,
    Ut161e,
    Ut161b,
    Ut161d,
}

impl SpecModel {
    /// Every table of the model, in manual order.
    pub(crate) fn tables(self) -> impl Iterator<Item = &'static ModeSpecs> {
        let all = match self {
            SpecModel::Ut61ePlus | SpecModel::Ut161e => ut61e_plus::ALL,
            SpecModel::Ut61bPlus | SpecModel::Ut161b => ut61bd_plus::UT61B_PLUS,
            SpecModel::Ut61dPlus | SpecModel::Ut161d => ut61bd_plus::UT61D_PLUS,
        };
        all.iter().map(move |&table| self.own(table))
    }

    /// The manual table for reading `m`, which its mode byte picks. `None`
    /// for a mode the manual gives this model no table for.
    pub(crate) fn table(self, m: &Measurement) -> Option<&'static ModeSpecs> {
        let mode = Mode::from_byte(u8::try_from(m.mode_raw).ok()?).ok()?;
        let table = match self {
            SpecModel::Ut61ePlus | SpecModel::Ut161e => ut61e_plus::table(mode),
            SpecModel::Ut61bPlus | SpecModel::Ut161b => ut61bd_plus::ut61b_plus(mode),
            SpecModel::Ut61dPlus | SpecModel::Ut161d => ut61bd_plus::ut61d_plus(mode),
        }?;
        Some(self.own(table))
    }

    /// The manual row reading `m` takes, which its range byte picks in its
    /// table. `None` where it takes the table's mode data only.
    pub(crate) fn row(self, m: &Measurement) -> Option<&'static RangeSpec> {
        self.table(m)?.row(m.range_raw)
    }

    /// The model's spec sheet, for `Protocol::spec_sheet`.
    pub(crate) fn sheet(self) -> Vec<SpecSheetTable> {
        self.tables().map(ModeSpecs::sheet_table).collect()
    }

    /// `table` as this model's manual prints it: a UT161 takes the UT161
    /// twin of a current part, the UT61+ part itself otherwise.
    fn own(self, table: &'static ModeSpecs) -> &'static ModeSpecs {
        match self {
            SpecModel::Ut161e | SpecModel::Ut161b | SpecModel::Ut161d => ut161(table),
            SpecModel::Ut61ePlus | SpecModel::Ut61bPlus | SpecModel::Ut61dPlus => table,
        }
    }
}

/// The UT161 twin of a UT61+ current part, or `table` itself for any other
/// part.
fn ut161(table: &'static ModeSpecs) -> &'static ModeSpecs {
    ut61e_plus::UT161_FUSES
        .iter()
        .chain(ut61bd_plus::UT161_FUSES)
        .find(|(ut61, _)| std::ptr::eq(*ut61, table))
        .map_or(table, |&(_, twin)| twin)
}

/// The UT161's mA/µA fuse ("9) DC Current", UT161 manual PDF p. 17), where
/// the UT61+ has "F1 Fuse 1A 240V Φ6x25mm".
const UT161_F1: &str = "F1 Fuse 600mA 1000V Φ6x32mm";

/// The UT161's A fuse, where the UT61+ has "F2 Fuse 10A 240V Φ6x25mm".
const UT161_F2: &str = "F2 Fuse 11A 1000V Φ10x38mm";

/// `base` with the UT161 manual's `fuse` for its overload protection.
const fn ut161_fuse(base: &'static ModeSpecs, fuse: &'static str) -> ModeSpecs {
    ModeSpecs {
        name: base.name,
        page: base.page,
        ranges: base.ranges,
        mode: ModeSpecInfo {
            input_impedance: base.mode.input_impedance,
            overload_protection: Some(fuse),
            notes: base.mode.notes,
        },
    }
}

// ── 5) Continuity and Diode (manual PDF page 16) ─────────────────────────

// One table for the three models, with no accuracy column. Each row's
// remarks are its own, so each part carries its row's notes.
static CONTINUITY: ModeSpecs = ModeSpecs {
    name: "Continuity and Diode",
    page: 16,
    ranges: &[RangeSpec {
        range: None,
        label: "Continuity",
        spec: SpecInfo {
            resolution: "0.1Ω",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &["Continuity: audio/visual alarm <50Ω; no beep ≥70Ω"],
    },
};

static DIODE: ModeSpecs = ModeSpecs {
    name: "Continuity and Diode",
    page: 16,
    ranges: &[RangeSpec {
        range: None,
        label: "Diode",
        spec: SpecInfo {
            resolution: "0.001V",
            accuracy: &[],
        },
    }],
    mode: ModeSpecInfo {
        input_impedance: None,
        overload_protection: Some("1000V"),
        notes: &[
            "Diode: open circuit ≈3V",
            "Diode: one beep 0.12V–2V (normal), long beep <0.12V (short)",
        ],
    },
};
