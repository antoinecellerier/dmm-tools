//! CSV export: rendering the recording buffer, running the save dialog and
//! the write off the UI thread, and folding the outcome back into a toast.

use dmm_lib::export::CsvLayout;
use log::{error, info};
use std::time::Instant;

use super::App;
use crate::recording::render_csv;

/// Result of a CSV export, sent from the writer thread to the UI.
pub(super) struct ExportOutcome {
    /// Toast text.
    message: String,
    is_error: bool,
    /// Samples written, on success. Drives the recording's "saved" mark, so
    /// a buffer that reached a file doesn't prompt before being discarded.
    exported: Option<usize>,
}

impl App {
    /// Columns the buffered recording is written with.
    ///
    /// The profile's slot count fixes the layout, with the widest sample
    /// actually buffered as a floor: a recording can outlive the connection
    /// that declared the profile. The floor counts the meter's own
    /// sub-values, so the appended ones come off it first — otherwise a
    /// transform would widen the meter's group by one as well as adding its
    /// own trailing column.
    fn csv_layout(&self) -> CsvLayout {
        CsvLayout {
            family_slots: self.capture_layout.aux_slots.max(
                self.recording
                    .max_aux_seen()
                    .saturating_sub(self.capture_layout.extra_slots),
            ),
            extra_slots: self.capture_layout.extra_slots,
            // Integrating is a CLI-only run mode.
            integral: false,
        }
    }

    pub(super) fn export_csv(&mut self) {
        if self.recording.samples.is_empty() {
            // Returning silently made the button and Ctrl+E look broken:
            // no file dialog, no message, nothing in the log. Say why.
            info!("export skipped: recording buffer is empty");
            self.toast = Some((
                "Nothing to export \u{2014} press Record to capture samples first".to_string(),
                true,
                Instant::now(),
            ));
            return;
        }
        // The meter these samples came from, not whatever is selected now.
        let device_model = self
            .capture_layout
            .device
            .unwrap_or_else(|| self.selected_device().display_name);

        // Render here and hand the bytes to the writer thread. Cloning the
        // sample buffer instead — which is what this used to do so the dialog
        // and write could run off the UI thread — duplicated every Sample,
        // each with its own heap string, roughly doubling peak memory at the
        // 500K cap. The rendered CSV is a fraction of that size, and building
        // it is cheaper than 500K allocations.
        let sample_count = self.recording.samples.len();
        let csv_bytes = match render_csv(&self.recording.samples, device_model, self.csv_layout()) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("CSV export failed: {e}");
                self.toast = Some((format!("Export failed: {e}"), true, Instant::now()));
                return;
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<ExportOutcome>();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("measurements.csv")
                .add_filter("CSV", &["csv"])
                .save_file()
            {
                // Writes a sibling .tmp and renames it into place, so a crash
                // mid-export can't leave a truncated file at the user-chosen
                // path.
                match dmm_settings::write_atomic(&path, &csv_bytes) {
                    Ok(()) => {
                        info!("exported {sample_count} samples to {}", path.display());
                        let file_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        let _ = tx.send(ExportOutcome {
                            message: format!("Exported {sample_count} samples to {file_name}"),
                            is_error: false,
                            exported: Some(sample_count),
                        });
                    }
                    Err(e) => {
                        error!("CSV export failed: {e}");
                        let _ = tx.send(ExportOutcome {
                            message: format!("Export failed: {e}"),
                            is_error: true,
                            exported: None,
                        });
                    }
                }
            }
        });
        self.export_result_rx = Some(rx);
    }

    pub(super) fn poll_export_result(&mut self) {
        if let Some(rx) = &self.export_result_rx
            && let Ok(outcome) = rx.try_recv()
        {
            if let Some(count) = outcome.exported {
                // Samples that arrived while the export ran are not in that
                // file, so mark only what was actually written.
                self.recording.mark_exported(count);
            }
            self.toast = Some((outcome.message, outcome.is_error, Instant::now()));
            self.export_result_rx = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
    use dmm_lib::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;

    /// A 1.234 V reading carrying `aux` sub-values of its own.
    fn measurement(aux: usize) -> Measurement {
        let payload: Vec<u8> = vec![
            0x02, // mode: DcV
            0x31, // range: 1
            b' ', b' ', b'1', b'.', b'2', b'3', b'4', // display
            0x00, 0x00, // progress
            0x30, 0x30, 0x30, // flags
        ];
        let mut m =
            dmm_lib::protocol::ut61eplus::parse_measurement(&payload, &Ut61ePlusTable::new())
                .expect("the fixture payload parses");
        m.aux_values = (0..aux)
            .map(|i| AuxValue {
                label: format!("sub{i}").into(),
                value: MeasuredValue::Normal(i as f64),
                unit: "V".into(),
                display_raw: Some(format!("{i}")),
                elapsed_secs: None,
            })
            .collect();
        m
    }

    /// An app whose profile declared `aux_slots` sub-value slots and reserved
    /// `extra_slots` trailing ones, holding one buffered sample per entry of
    /// `aux_counts`.
    fn app_holding(aux_slots: usize, extra_slots: usize, aux_counts: &[usize]) -> App {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        app.capture_layout.aux_slots = aux_slots;
        app.capture_layout.extra_slots = extra_slots;
        app.recording.toggle(std::time::Instant::now());
        for &aux in aux_counts {
            app.recording
                .push(&measurement(aux), &app.wall_clock, extra_slots.min(aux));
        }
        app
    }

    /// The header and every row, as comma-separated cells.
    fn exported(app: &App) -> Vec<Vec<String>> {
        let bytes = render_csv(&app.recording.samples, "mock", app.csv_layout())
            .expect("rendering the fixture buffer");
        String::from_utf8(bytes)
            .expect("CSV is UTF-8")
            .lines()
            .skip(1) // the provenance comment
            .map(|l| l.split(',').map(str::to_string).collect())
            .collect()
    }

    /// The column layout is fixed for the whole file: a sample with fewer
    /// sub-values than the widest one has to pad, not shorten its row, or
    /// every later column is read under the wrong heading.
    #[test]
    fn rows_line_up_with_the_header_across_0_1_and_2_sub_values() {
        let app = app_holding(0, 0, &[0, 1, 2]);
        let rows = exported(&app);
        let header = &rows[0];
        assert_eq!(
            header[6..],
            [
                "aux1_label",
                "aux1_value",
                "aux1_unit",
                "aux2_label",
                "aux2_value",
                "aux2_unit"
            ],
            "the widest buffered sample sets the slot count"
        );
        for (i, row) in rows.iter().enumerate().skip(1) {
            assert_eq!(row.len(), header.len(), "row {i} does not fill the header");
        }
        assert_eq!(rows[1][6..], ["", "", "", "", "", ""], "no sub-values");
        assert_eq!(
            rows[2][6..],
            ["sub0", "0", "V", "", "", ""],
            "one sub-value"
        );
        assert_eq!(
            rows[3][6..],
            ["sub0", "0", "V", "sub1", "1", "V"],
            "two sub-values"
        );
    }

    /// A meter that can send four sub-values keeps all four columns even
    /// while it is sending fewer, so a file doesn't change shape with the
    /// mode the meter happened to be in.
    #[test]
    fn the_profile_holds_its_columns_open_past_the_widest_sample() {
        let app = app_holding(4, 0, &[0, 1, 2]);
        let rows = exported(&app);
        assert_eq!(rows[0].len(), 6 + 4 * 3);
        assert_eq!(
            rows[3][6..],
            ["sub0", "0", "V", "sub1", "1", "V", "", "", "", "", "", ""]
        );
    }

    /// A transform's appended sub-value gets the trailing group, and comes
    /// off the floor the buffered samples set rather than widening the
    /// meter's own group as well.
    #[test]
    fn an_appended_sub_value_takes_the_trailing_group_not_a_second_one() {
        let app = app_holding(0, 1, &[0, 1, 2]);
        let rows = exported(&app);
        assert_eq!(
            rows[0].len(),
            6 + 2 * 3,
            "one group for the meter, one for the appended value"
        );
        // The two-sub-value sample carries one of its own plus the appended
        // one, which is pinned to the trailing group.
        assert_eq!(rows[3][6..], ["sub0", "0", "V", "sub1", "1", "V"]);
        // The one-sub-value sample's is the appended one, so the meter's
        // group stays empty rather than claiming it.
        assert_eq!(rows[2][6..], ["", "", "", "sub0", "0", "V"]);
    }
}
