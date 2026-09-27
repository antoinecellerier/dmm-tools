//! Tabular (CSV) export layout shared by the CLI and the GUI: one header, one
//! row builder, so the two writers cannot disagree on columns. Cells only —
//! the `csv` crate stays in the binaries.

use std::borrow::Cow;

use dmm_lib::measurement::{AuxValue, Measurement};

/// The columns every exported row starts with, whatever the meter or the
/// options.
pub const CSV_BASE_COLUMNS: [&str; 6] = ["timestamp", "mode", "value", "unit", "range", "flags"];

/// The columns added when the run integrates the reading over time.
///
/// They come before the sub-value groups so `--integrate` consumers keep the
/// column positions they had before sub-values were exported.
pub const CSV_INTEGRAL_COLUMNS: [&str; 2] = ["integral", "integral_unit"];

/// The columns added when the file carries markers: the marker's number and
/// its note.
///
/// Last, after the sub-value groups, so a file with markers keeps every
/// other column where a file without them has it.
pub(crate) const CSV_MARKER_COLUMNS: [&str; 2] = ["marker", "note"];

/// The columns one sub-value slot contributes, as header suffixes:
/// `aux1_label,aux1_value,aux1_unit`.
///
/// [`aux_cells`] returns an array exactly this long, so a column cannot be
/// added here without the row builder failing to compile.
pub(crate) const AUX_EXPORT_COLUMNS: [&str; 3] = ["label", "value", "unit"];

/// The cells one exported slot writes, in [`AUX_EXPORT_COLUMNS`] order.
///
/// `main_unit` is the parent reading's unit, used when the sub-value leaves
/// its own empty (see [`AuxValue::unit_or`]).
fn aux_cells<'a>(
    aux: &'a AuxValue,
    main_unit: &'a str,
) -> [Cow<'a, str>; AUX_EXPORT_COLUMNS.len()] {
    [
        Cow::Borrowed(aux.label.as_ref()),
        aux.value_export_str(),
        Cow::Borrowed(aux.unit_or(main_unit)),
    ]
}

/// Lay the sub-values of `m` out for a fixed-column export.
///
/// The first `family_slots` entries hold the meter's own sub-values in
/// order, padded with `None`; the following `extra_slots` entries hold the
/// last `extra_slots` sub-values — the ones software appended after the
/// meter's (a transform's `Raw`), so they keep a fixed column whatever the
/// meter sent that frame. Always returns exactly
/// `family_slots + extra_slots` entries; surplus meter sub-values are
/// truncated rather than shifting later columns.
///
/// Without the split, a UT181A run crossing from DC V (no sub-values) to
/// AC V (frequency and period) would move `Raw` from `aux1_*` to `aux3_*`
/// mid-file, mixing three quantities into one column.
fn aux_slots(m: &Measurement, family_slots: usize, extra_slots: usize) -> Vec<Option<&AuxValue>> {
    let n = m.aux_values.len();
    // A frame carrying fewer sub-values than `extra_slots` promises is one
    // where the meter sent none of its own: take what is there as the
    // appended ones and leave the meter's slots empty.
    let extra = extra_slots.min(n);
    let mut slots = Vec::with_capacity(family_slots + extra_slots);
    slots.extend(
        m.aux_values[..n - extra]
            .iter()
            .take(family_slots)
            .map(Some),
    );
    slots.resize(family_slots, None);
    slots.extend(m.aux_values[n - extra..].iter().map(Some));
    slots.resize(family_slots + extra_slots, None);
    slots
}

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
    /// Whether the file writes [`CSV_MARKER_COLUMNS`].
    pub markers: bool,
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
            + if self.markers {
                CSV_MARKER_COLUMNS.len()
            } else {
                0
            }
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
        if self.markers {
            header.extend(CSV_MARKER_COLUMNS.into_iter().map(Cow::Borrowed));
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
    /// passes `extra_slots` because its transform is fixed for the run;
    /// `marker` is the number and note of the marker on this reading, written
    /// when the layout has marker columns (empty cells when it is `None`).
    pub fn row<'a>(
        &self,
        m: &'a Measurement,
        timestamp: &'a str,
        integral: Option<(f64, &'a str)>,
        extra_aux: usize,
        marker: Option<(u32, &'a str)>,
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
        // Which sub-value lands in which slot is `aux_slots`' business:
        // it pads the meter's own groups, pins the appended ones to the
        // trailing groups, and truncates a surplus rather than desyncing every
        // later column from the header.
        for slot in aux_slots(m, self.family_slots, extra) {
            match slot {
                Some(aux) => cells.extend(aux_cells(aux, &m.unit)),
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
        if self.markers {
            match marker {
                Some((number, note)) => {
                    cells.push(Cow::Owned(number.to_string()));
                    cells.push(Cow::Borrowed(note));
                }
                None => cells.extend(std::iter::repeat_n(
                    Cow::Borrowed(""),
                    CSV_MARKER_COLUMNS.len(),
                )),
            }
        }
        debug_assert_eq!(cells.len(), self.column_count());
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::MeasuredValue;

    fn layout(family_slots: usize, extra_slots: usize, integral: bool) -> CsvLayout {
        CsvLayout {
            family_slots,
            extra_slots,
            integral,
            markers: false,
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
        assert_eq!(
            header_line(CsvLayout {
                markers: true,
                ..layout(1, 0, true)
            }),
            "timestamp,mode,value,unit,range,flags,integral,integral_unit,\
             aux1_label,aux1_value,aux1_unit,marker,note"
        );
    }

    /// The header is written once at the top of the file and the rows one at a
    /// time; a mismatch would silently misalign every column.
    #[test]
    fn row_length_matches_header() {
        let layouts = [
            layout(0, 0, false),
            layout(2, 0, false),
            layout(2, 1, true),
            layout(4, 1, false),
        ];
        for l in layouts
            .into_iter()
            .flat_map(|l| [false, true].map(|markers| CsvLayout { markers, ..l }))
        {
            for aux_count in [0usize, 2, 3] {
                let mut m = reading();
                m.aux_values = (0..aux_count).map(|i| aux("Aux", i as f64, "Hz")).collect();
                for extra_aux in [0usize, 1] {
                    for marker in [None, Some((3, "load on"))] {
                        let row = l.row(
                            &m,
                            "2026-01-01T00:00:00+00:00",
                            Some((1.5, "Vs")),
                            extra_aux,
                            marker,
                        );
                        assert_eq!(
                            row.len(),
                            l.header().len(),
                            "{l:?} aux_count={aux_count} extra_aux={extra_aux} {marker:?}"
                        );
                    }
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
        let row = layout(4, 1, false).row(&m, "ts", None, 1, None);
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
        let row = layout(4, 1, false).row(&m, "ts", None, 0, None);
        assert_eq!(&row[6..9], ["Frequency", "50.01", "Hz"]);
        assert_eq!(&row[9..12], ["Period", "20", "ms"]);
        assert_eq!(&row[18..21], ["", "", ""]);
    }

    /// A word the meter shows instead of a reading leaves the value cell
    /// empty: the mode column carries the word, and the value column stays
    /// numeric.
    #[test]
    fn a_no_reading_row_has_an_empty_value() {
        let mut m = reading();
        m.mode = "Auto".into();
        m.value = MeasuredValue::NoReading("Auto");
        m.unit = "".into();
        m.range_label = "".into();
        let row = layout(0, 0, false).row(&m, "ts", None, 0, None);
        assert_eq!(row, ["ts", "Auto", "", "", "", ""]);
    }

    /// A frame carrying only the AC component of an AC+DC reading: the value
    /// cell stays empty and the component fills its sub-value slot, so each
    /// row still stands for one frame at its own time.
    #[test]
    fn a_frame_without_a_main_reading_fills_only_its_sub_value_slot() {
        let mut m = reading();
        m.mode = "AC+DC V".into();
        m.value = MeasuredValue::Absent;
        m.display_raw = None;
        m.range_label = "2.2V".into();
        m.aux_values = vec![aux("AC", 0.0123, "")];
        let row = layout(1, 0, false).row(&m, "ts", None, 0, None);
        assert_eq!(
            row,
            ["ts", "AC+DC V", "", "V", "2.2V", "", "AC", "0.0123", "V"]
        );
    }

    /// The marker cells close the row, after the sub-value groups; a reading
    /// without a marker leaves them empty, and a layout without marker
    /// columns writes neither.
    #[test]
    fn marker_cells_come_last() {
        let mut m = reading();
        m.aux_values = vec![aux("Frequency", 50.01, "Hz")];
        let l = CsvLayout {
            markers: true,
            ..layout(1, 0, false)
        };
        let row = l.row(&m, "ts", None, 0, Some((3, "load on, 2.2 Ω")));
        assert_eq!(
            &row[6..],
            ["Frequency", "50.01", "Hz", "3", "load on, 2.2 Ω"]
        );
        let row = l.row(&m, "ts", None, 0, None);
        assert_eq!(&row[9..], ["", ""]);
        let row = layout(1, 0, false).row(&m, "ts", None, 0, Some((3, "load on")));
        assert_eq!(row.len(), 9, "no marker columns to write it in");
    }

    #[test]
    fn integral_cells_follow_the_flags_column() {
        let m = reading();
        let l = layout(0, 0, true);
        let row = l.row(&m, "ts", Some((0.5, "mAh")), 0, None);
        assert_eq!(row[6], "0.500000");
        assert_eq!(row[7], "mAh");
        let row = l.row(&m, "ts", None, 0, None);
        assert_eq!(row[6], "");
        assert_eq!(row[7], "");
    }

    /// A sub-value as a meter sends it: parsed value and display digits.
    fn shown(label: &'static str, display: &str, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(display.trim().parse().unwrap_or(0.0)),
            unit: unit.into(),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
    }

    #[test]
    fn an_aux_no_reading_exports_an_empty_value() {
        let mut a = shown("Max", "9.999", "");
        a.value = MeasuredValue::NoReading("Auto");
        assert_eq!(aux_cells(&a, "V"), ["Max", "", "V"]);
    }

    /// Labels of an export layout, `""` for an empty slot.
    fn slot_labels<'a>(slots: &[Option<&'a AuxValue>]) -> Vec<&'a str> {
        slots
            .iter()
            .map(|slot| slot.map_or("", |aux| aux.label.as_ref()))
            .collect()
    }

    #[test]
    fn export_slots_pad_a_reading_without_sub_values() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let slots = aux_slots(&m, 2, 0);
        assert_eq!(slots.len(), 2);
        assert!(slots.iter().all(Option::is_none));
    }

    /// With nothing appended by software the layout is the plain
    /// "first `family_slots` sub-values, then padding" it always was.
    #[test]
    fn export_slots_without_extras_keep_the_meter_order() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(230.0), "V", StatusFlags::default());
        m.aux_values = vec![
            shown("Frequency", "50.01", "Hz"),
            shown("Period", "20.00", "ms"),
        ];
        assert_eq!(
            slot_labels(&aux_slots(&m, 4, 0)),
            ["Frequency", "Period", "", ""]
        );
    }

    /// A UT181A AC V frame (4 meter slots) with a transform's `Raw` appended:
    /// the meter's two sub-values keep the first columns and `Raw` takes the
    /// fifth.
    #[test]
    fn an_appended_sub_value_takes_the_extra_slot() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(230.0), "V", StatusFlags::default());
        m.aux_values = vec![
            shown("Frequency", "50.01", "Hz"),
            shown("Period", "20.00", "ms"),
            shown("Raw", "230.0", "V"),
        ];
        let slots = aux_slots(&m, 4, 1);
        assert_eq!(slot_labels(&slots), ["Frequency", "Period", "", "", "Raw"]);
        assert_eq!(slots[4].map(|aux| aux.unit.as_ref()), Some("V"));
    }

    /// The next frame of that same run, after the dial moved to DC V: the
    /// meter sends no sub-values, and `Raw` must not slide into `aux1_*`.
    #[test]
    fn an_appended_sub_value_holds_its_column_when_the_meter_sends_none() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(230.0), "V", StatusFlags::default());
        m.aux_values = vec![shown("Raw", "230.0", "V")];
        assert_eq!(slot_labels(&aux_slots(&m, 4, 1)), ["", "", "", "", "Raw"]);
    }

    /// A reading with more sub-values than the family profile promised is cut
    /// short — the appended one still gets its own column.
    #[test]
    fn surplus_meter_sub_values_are_truncated_not_shifted() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(230.0), "V", StatusFlags::default());
        m.aux_values = vec![
            shown("Max", "5.01", ""),
            shown("Average", "4.99", ""),
            shown("Min", "4.96", ""),
            shown("Raw", "230.0", "V"),
        ];
        assert_eq!(slot_labels(&aux_slots(&m, 2, 1)), ["Max", "Average", "Raw"]);
    }

    /// A transform is on, but this frame carries nothing at all: every slot,
    /// the extra one included, stays empty rather than borrowing a neighbour.
    #[test]
    fn an_empty_frame_leaves_the_extra_slot_empty() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let slots = aux_slots(&m, 4, 1);
        assert_eq!(slots.len(), 5);
        assert!(slots.iter().all(Option::is_none));
    }

    /// A UT61E+ AC+DC V frame carrying only the AC component, scaled: the Raw
    /// the transform appends has no value either, and still keeps the
    /// trailing slot instead of sliding into the meter's own.
    #[test]
    fn a_raw_on_a_frame_without_a_main_reading_keeps_its_slot() {
        let t = dmm_lib::transform::Transform::linear(10.0, 0.0, None);
        let mut m = Measurement::test_fixture(MeasuredValue::Absent, "V", StatusFlags::default());
        m.display_raw = None;
        m.aux_values.push(shown("AC", " 0.1234", ""));
        t.apply(&mut m);

        let slots = aux_slots(&m, 1, t.extra_aux_count());
        assert_eq!(slots[0].map(|a| a.label.as_ref()), Some("AC"));
        assert_eq!(
            slots[1].map(|a| a.label.as_ref()),
            Some(dmm_lib::transform::RAW_LABEL)
        );
    }
}
