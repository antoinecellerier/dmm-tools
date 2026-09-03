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
        // The profile's slot count fixes the layout, with the widest sample
        // actually buffered as a floor: a recording can outlive the
        // connection that declared the profile. The floor counts the meter's
        // own sub-values, so the appended ones come off it first — otherwise
        // a transform would widen the meter's group by one as well as adding
        // its own trailing column.
        let family_slots = self.capture_layout.aux_slots.max(
            self.recording
                .max_aux_seen()
                .saturating_sub(self.capture_layout.extra_slots),
        );
        let layout = CsvLayout {
            family_slots,
            extra_slots: self.capture_layout.extra_slots,
            integral: false,
        };
        let csv_bytes = match render_csv(&self.recording.samples, device_model, layout) {
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
