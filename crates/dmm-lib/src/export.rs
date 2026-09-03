//! Tabular (CSV) export layout shared by the CLI and the GUI: one header, one
//! row builder, so the two writers cannot disagree on columns. Cells only —
//! the `csv` crate stays in the binaries.

use std::borrow::Cow;

use crate::measurement::{AUX_EXPORT_COLUMNS, Measurement};

/// The columns every exported row starts with, whatever the meter or the
/// options.
pub const CSV_BASE_COLUMNS: [&str; 6] = ["timestamp", "mode", "value", "unit", "range", "flags"];

/// The columns added when the run integrates the reading over time.
///
/// They come before the sub-value groups so `--integrate` consumers keep the
/// column positions they had before sub-values were exported.
pub const CSV_INTEGRAL_COLUMNS: [&str; 2] = ["integral", "integral_unit"];

/// Provenance comment written before the header: `# device: {model}`.
///
/// Without the terminating newline — callers `writeln!` it.
pub fn device_comment(model: &str) -> String {
    format!("# device: {model}")
}

/// Column layout fixed for a whole file.
///
/// A CSV needs one column layout for every row it contains, so the counts here
/// describe the widest row the run can produce, not what any one reading
/// carries: `family_slots` is the meter family's `max_aux_values` and
/// `extra_slots` reserves trailing groups for the sub-values software appends
/// after the meter's own (a transform's `Raw`). A reading that fills fewer
/// leaves the rest empty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsvLayout {
    /// Groups holding the meter's own sub-values.
    pub family_slots: usize,
    /// Groups reserved for sub-values software appended.
    pub extra_slots: usize,
    /// Whether the run writes [`CSV_INTEGRAL_COLUMNS`].
    pub integral: bool,
}

impl CsvLayout {
    /// Total sub-value groups the file lays out.
    pub fn aux_slots(&self) -> usize {
        self.family_slots + self.extra_slots
    }

    /// How many cells a header or a row holds.
    fn column_count(&self) -> usize {
        CSV_BASE_COLUMNS.len()
            + if self.integral {
                CSV_INTEGRAL_COLUMNS.len()
            } else {
                0
            }
            + self.aux_slots() * AUX_EXPORT_COLUMNS.len()
    }

    /// The header cells, in column order.
    pub fn header(&self) -> Vec<Cow<'static, str>> {
        let mut header: Vec<Cow<'static, str>> = Vec::with_capacity(self.column_count());
        header.extend(CSV_BASE_COLUMNS.into_iter().map(Cow::Borrowed));
        if self.integral {
            header.extend(CSV_INTEGRAL_COLUMNS.into_iter().map(Cow::Borrowed));
        }
        for i in 1..=self.aux_slots() {
            for suffix in AUX_EXPORT_COLUMNS {
                header.push(Cow::Owned(format!("aux{i}_{suffix}")));
            }
        }
        header
    }

    /// One row's cells, in the same order [`CsvLayout::header`] names them.
    ///
    /// `timestamp` is already formatted (the binaries own chrono); `integral`
    /// is the already-scaled `(value, display_unit)` pair, written as
    /// `{value:.6}` when the layout has integral columns (ignored otherwise;
    /// empty cells when the layout has them but `integral` is `None`);
    /// `extra_aux` is how many trailing sub-values of `m` were appended by
    /// software for this sample — the GUI records it per sample, the CLI
    /// passes `extra_slots` because its transform is fixed for the run.
    pub fn row<'a>(
        &self,
        m: &'a Measurement,
        timestamp: &'a str,
        integral: Option<(f64, &'a str)>,
        extra_aux: usize,
    ) -> Vec<Cow<'a, str>> {
        let mut cells: Vec<Cow<'a, str>> = Vec::with_capacity(self.column_count());
        cells.push(Cow::Borrowed(timestamp));
        cells.push(Cow::Borrowed(m.mode.as_ref()));
        cells.push(m.value_export_str());
        cells.push(Cow::Borrowed(m.unit.as_ref()));
        cells.push(Cow::Borrowed(m.range_label.as_ref()));
        cells.push(Cow::Owned(m.flags.to_string()));
        if self.integral {
            match integral {
                Some((value, unit)) => {
                    cells.push(Cow::Owned(format!("{value:.6}")));
                    cells.push(Cow::Borrowed(unit));
                }
                // The run integrates but this reading has no integral yet:
                // keep the columns, leave them blank.
                None => cells.extend(std::iter::repeat_n(Cow::Borrowed(""), 2)),
            }
        }
        // Only the extras *this* reading carries are claimed. A sample
        // recorded before a mid-recording scale has none, and claiming one
        // anyway would read its last meter sub-value as the appended one —
        // filing Frequency under the `Raw` column.
        let extra = extra_aux.min(self.extra_slots);
        // Which sub-value lands in which slot is `export_aux_slots`' business:
        // it pads the meter's own groups, pins the appended ones to the
        // trailing groups, and truncates a surplus rather than desyncing every
        // later column from the header.
        for slot in m.export_aux_slots(self.family_slots, extra) {
            match slot {
                Some(aux) => cells.extend(aux.export_cells(&m.unit)),
                None => cells.extend(std::iter::repeat_n(
                    Cow::Borrowed(""),
                    AUX_EXPORT_COLUMNS.len(),
                )),
            }
        }
        // Reserved groups this reading didn't claim.
        cells.extend(std::iter::repeat_n(
            Cow::Borrowed(""),
            (self.extra_slots - extra) * AUX_EXPORT_COLUMNS.len(),
        ));
        debug_assert_eq!(cells.len(), self.column_count());
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::StatusFlags;
    use crate::measurement::{AuxValue, MeasuredValue};

    fn layout(family_slots: usize, extra_slots: usize, integral: bool) -> CsvLayout {
        CsvLayout {
            family_slots,
            extra_slots,
            integral,
        }
    }

    fn reading() -> Measurement {
        Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default())
    }

    fn aux(label: &'static str, value: f64, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(value),
            unit: unit.into(),
            display_raw: None,
            elapsed_secs: None,
        }
    }

    fn header_line(l: CsvLayout) -> String {
        l.header().join(",")
    }

    #[test]
    fn header_names_every_column_in_order() {
        assert_eq!(
            header_line(layout(2, 1, true)),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit,\
             aux1_label,aux1_value,aux1_unit,aux2_label,aux2_value,aux2_unit,\
             aux3_label,aux3_value,aux3_unit"
        );
        assert_eq!(
            header_line(layout(0, 0, false)),
            "timestamp,mode,value,unit,range,flags"
        );
    }

    /// The header is written once at the top of the file and the rows one at a
    /// time; a mismatch would silently misalign every column.
    #[test]
    fn row_length_matches_header() {
        for l in [
            layout(0, 0, false),
            layout(2, 0, false),
            layout(2, 1, true),
            layout(4, 1, false),
        ] {
            for aux_count in [0usize, 2, 3] {
                let mut m = reading();
                m.aux_values = (0..aux_count).map(|i| aux("Aux", i as f64, "Hz")).collect();
                for extra_aux in [0usize, 1] {
                    let row = l.row(
                        &m,
                        "2026-01-01T00:00:00+00:00",
                        Some((1.5, "Vs")),
                        extra_aux,
                    );
                    assert_eq!(
                        row.len(),
                        l.header().len(),
                        "{l:?} aux_count={aux_count} extra_aux={extra_aux}"
                    );
                }
            }
        }
    }

    #[test]
    fn row_keeps_a_software_sub_value_in_its_trailing_slot() {
        let mut m = reading();
        m.aux_values = vec![
            aux("Frequency", 50.01, "Hz"),
            aux("Period", 20.0, "ms"),
            aux("Raw", 123.4, "mV"),
        ];
        let row = layout(4, 1, false).row(&m, "ts", None, 1);
        assert_eq!(&row[6..9], ["Frequency", "50.01", "Hz"]);
        assert_eq!(&row[9..12], ["Period", "20", "ms"]);
        // The meter reported fewer sub-values than the family can, so its
        // remaining groups stay empty...
        assert_eq!(&row[12..18], ["", "", "", "", "", ""]);
        // ...and the appended one keeps the group reserved for it.
        assert_eq!(&row[18..21], ["Raw", "123.4", "mV"]);
    }

    /// A sample recorded before a mid-recording scale carries no appended
    /// sub-value: the trailing group must stay empty rather than swallowing
    /// the meter's last one (Frequency filed under `Raw`).
    #[test]
    fn row_without_a_claimed_extra_leaves_the_trailing_slot_empty() {
        let mut m = reading();
        m.aux_values = vec![aux("Frequency", 50.01, "Hz"), aux("Period", 20.0, "ms")];
        let row = layout(4, 1, false).row(&m, "ts", None, 0);
        assert_eq!(&row[6..9], ["Frequency", "50.01", "Hz"]);
        assert_eq!(&row[9..12], ["Period", "20", "ms"]);
        assert_eq!(&row[18..21], ["", "", ""]);
    }

    #[test]
    fn integral_cells_follow_the_flags_column() {
        let m = reading();
        let l = layout(0, 0, true);
        let row = l.row(&m, "ts", Some((0.5, "mAh")), 0);
        assert_eq!(row[6], "0.500000");
        assert_eq!(row[7], "mAh");
        let row = l.row(&m, "ts", None, 0);
        assert_eq!(row[6], "");
        assert_eq!(row[7], "");
    }
}
