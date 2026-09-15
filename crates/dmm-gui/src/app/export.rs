//! Export: rendering the recording buffer as a CSV or a replay file, running
//! the save dialog and the write off the UI thread, and folding the outcome
//! back into a toast.

use chrono::{DateTime, Local};
use dmm_lib::export::CsvLayout;
use log::{error, info, warn};
use std::path::Path;
use std::time::Instant;

use super::App;
use crate::recording::{Sample, render_csv, render_replay};

/// What the CSV's `# device:` comment says when nothing ever identified the
/// meter — a recording toggled on under Auto-detect before one answered.
const UNKNOWN_DEVICE: &str = "unknown";

/// Extension of a replay file: the dialog's filter and default name for one.
/// `--replay` reads the file's header, not its name.
const REPLAY_EXTENSION: &str = "replay";

/// Why a mock recording cannot be written as a replay file. The menu entry
/// is disabled for the mock, so this only guards the call itself.
pub(super) const NO_WIRE_FORMAT: &str =
    "The mock has no wire format to export; record a real meter";

/// Which file the export writes. Settled before the dialog opens — by the
/// Export… label (a CSV) or the menu on its arrow — because the dialog hands
/// back the path the user saved and not the file type they picked, and the
/// GTK chooser keeps the name's extension when its filter changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExportFormat {
    Csv,
    Replay,
}

impl ExportFormat {
    /// The dialog's one filter: its label and extension.
    fn filter(self) -> (&'static str, &'static str) {
        match self {
            Self::Csv => ("CSV", "csv"),
            Self::Replay => ("Replay", REPLAY_EXTENSION),
        }
    }

    /// The name the dialog opens with: the meter, the mode it stayed in and
    /// the moment the recording started, as
    /// `measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv`, so a folder of
    /// exports sorts by meter and by run. A recording that crossed a
    /// function switch has no one mode and leaves that segment out.
    fn default_name(self, model: &str, mode: Option<&str>, start: DateTime<Local>) -> String {
        let mode = mode
            .map(|m| format!("{}-", file_safe(m)))
            .unwrap_or_default();
        format!(
            "measurements-{}-{mode}{}.{}",
            file_safe(model),
            start.format("%Y-%m-%d_%H-%M-%S"),
            self.filter().1
        )
    }
}

/// A meter or mode name as one file-name word: runs of whitespace become a
/// single `-` and the separators a path could read drop out, so "Mock
/// UT61E+" exports as `Mock-UT61E+`.
fn file_safe(name: &str) -> String {
    name.replace(['/', '\\', ':'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// The mode the whole buffer stayed in, for the file name.
///
/// `None` once a recording crossed a function switch: naming that file after
/// the mode it started in would credit every later reading to it.
fn single_mode(samples: &[Sample]) -> Option<&str> {
    let first = samples.first()?.measurement.mode.as_ref();
    samples
        .iter()
        .all(|s| s.measurement.mode == first)
        .then_some(first)
}

/// Result of an export, sent from the writer thread to the UI.
pub(super) struct ExportOutcome {
    /// Toast text.
    message: String,
    is_error: bool,
    /// Samples written, on success. Drives the recording's "saved" mark, so
    /// a buffer that reached a file doesn't prompt before being discarded.
    exported: Option<usize>,
}

/// Write one export to the path the user chose and say how it went.
///
/// Writes a sibling .tmp and renames it into place, so a crash mid-export
/// can't leave a truncated file at the user-chosen path.
fn write_export(path: &Path, bytes: &[u8], sample_count: usize) -> ExportOutcome {
    match dmm_settings::write_atomic(path, bytes) {
        Ok(()) => {
            info!("exported {sample_count} samples to {}", path.display());
            // The file name, not the whole path: its extension names the
            // format written, which is the part the user needs confirmed.
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            ExportOutcome {
                message: format!("Exported {sample_count} samples to {file_name}"),
                is_error: false,
                exported: Some(sample_count),
            }
        }
        Err(e) => {
            error!("export failed: {e}");
            ExportOutcome {
                message: format!("Export failed: {e}"),
                is_error: true,
                exported: None,
            }
        }
    }
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

    pub(super) fn export_recording(&mut self, format: ExportFormat) {
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
        // Nothing named it and nothing was ever identified — a recording
        // toggled on before a meter answered — so the file says so rather
        // than crediting the samples to a model that was only selected later.
        let device_model = self
            .capture_layout
            .device
            .or_else(|| self.active_device().map(|d| d.display_name))
            .unwrap_or(UNKNOWN_DEVICE);

        // Render here and hand the bytes to the writer thread. Cloning the
        // sample buffer instead — which is what this used to do so the dialog
        // and write could run off the UI thread — duplicated every Sample,
        // each with its own heap string, roughly doubling peak memory at the
        // 500K cap. The rendered file is a fraction of that size, and building
        // it is cheaper than 500K allocations.
        let sample_count = self.recording.samples.len();
        // Built here rather than in the dialog thread, which holds only the
        // rendered bytes: the first sample is the recording's start, and the
        // buffer is known non-empty above.
        let default_name = format.default_name(
            device_model,
            single_mode(&self.recording.samples),
            self.recording.samples[0].wall_time,
        );
        let bytes = match format {
            ExportFormat::Csv => {
                match render_csv(&self.recording.samples, device_model, self.csv_layout()) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        error!("CSV export failed: {e}");
                        self.toast = Some((format!("Export failed: {e}"), true, Instant::now()));
                        return;
                    }
                }
            }
            ExportFormat::Replay => {
                let text = self
                    .replay_device_id()
                    .and_then(|id| render_replay(&self.recording.samples, id, Some(device_model)));
                match text {
                    Some(text) => text.into_bytes(),
                    None => {
                        warn!("replay export refused: the buffered samples carry no meter frames");
                        self.toast = Some((NO_WIRE_FORMAT.to_string(), true, Instant::now()));
                        return;
                    }
                }
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<ExportOutcome>();
        std::thread::spawn(move || {
            let (label, extension) = format.filter();
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter(label, &[extension])
                .save_file()
            else {
                return;
            };
            let _ = tx.send(write_export(&path, &bytes, sample_count));
        });
        self.export_result_rx = Some(rx);
    }

    /// The meter whose frames a replay file would carry: the one the
    /// recording named, else the one selected or detected now, as the CSV's
    /// provenance falls back. `None` for the mock, which has no wire format.
    pub(super) fn replay_device_id(&self) -> Option<&'static str> {
        self.capture_layout.device_id.or_else(|| {
            self.active_device()
                .filter(|d| d.requires_hardware)
                .map(|d| d.id)
        })
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
    use chrono::TimeZone;
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

    /// Each format opens the dialog on a name and a filter of its own, so
    /// a user who keeps the default gets the file the menu promised.
    #[test]
    fn each_format_names_its_own_file() {
        let start = Local
            .with_ymd_and_hms(2026, 9, 15, 14, 30, 5)
            .single()
            .expect("a fixed local timestamp");
        assert_eq!(
            ExportFormat::Csv.default_name("UT61E+", Some("DC V"), start),
            "measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv"
        );
        assert_eq!(ExportFormat::Csv.filter(), ("CSV", "csv"));
        assert_eq!(
            ExportFormat::Replay.default_name("UT61E+", Some("DC V"), start),
            "measurements-UT61E+-DC-V-2026-09-15_14-30-05.replay"
        );
        assert_eq!(ExportFormat::Replay.filter(), ("Replay", "replay"));
        // A recording with no one mode keeps meter and time, nothing between.
        assert_eq!(
            ExportFormat::Csv.default_name("UT61E+", None, start),
            "measurements-UT61E+-2026-09-15_14-30-05.csv"
        );
    }

    /// A model or mode name goes into the file name as one word: the dialog
    /// opens on a name the user can save as typed, not one carrying a path
    /// separator.
    #[test]
    fn a_name_is_folded_into_one_file_name_word() {
        assert_eq!(file_safe("Mock UT61E+"), "Mock-UT61E+");
        assert_eq!(file_safe("UT61E+ / UT61B+"), "UT61E+-UT61B+");
        assert_eq!(file_safe("DC V"), "DC-V");
        assert_eq!(file_safe("\u{3a9}"), "\u{3a9}");
    }

    /// The mode names the file only while the whole recording stayed in it —
    /// a buffer that crossed a function switch is no one mode's.
    #[test]
    fn the_mode_names_the_file_only_while_the_buffer_holds_one() {
        let mut app = app_holding(0, 0, &[0, 0]);
        assert_eq!(single_mode(&app.recording.samples), Some("DC V"));
        app.recording.samples[1].measurement.mode = "AC V".into();
        assert_eq!(single_mode(&app.recording.samples), None);
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
