//! Specification data for the UT61+/UT161 family, as manual tables keyed by
//! range byte.
//!
//! One manual covers the UT61B+, UT61D+ and UT61E+: the UT61+ Series User
//! Manual, "IX. Specifications" (PDF pages 14-18, printed 25-34), prints a
//! UT61E+ table and a UT61B+/UT61D+ table for most functions, with rows and
//! notes tagged for one model. Each model's tables are in its file; the
//! continuity and diode table, which the three models share, is here.

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
}

impl SpecModel {
    /// Every table of the model, in manual order.
    pub(crate) fn tables(self) -> impl Iterator<Item = &'static ModeSpecs> {
        let all = match self {
            SpecModel::Ut61ePlus => ut61e_plus::ALL,
        };
        all.iter().copied()
    }

    /// The manual table for reading `m`, which its mode byte picks. `None`
    /// for a mode the manual gives this model no table for.
    pub(crate) fn table(self, m: &Measurement) -> Option<&'static ModeSpecs> {
        let mode = Mode::from_byte(u8::try_from(m.mode_raw).ok()?).ok()?;
        match self {
            SpecModel::Ut61ePlus => ut61e_plus::table(mode),
        }
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
