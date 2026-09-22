//! Export: rendering the sample buffer — the recording, or with nothing
//! recorded the samples the graph holds — as a CSV, a JSON document or a
//! replay file, running the save dialog and the write off the UI thread, and
//! folding the outcome back into a toast.

use chrono::{DateTime, Local};
use dmm_lib::export::CsvLayout;
use log::{error, info, warn};
use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

use super::{App, ConnectionState};
use crate::recording::{BufferRole, Sample, render_csv, render_json, render_replay};

/// What the CSV's `# device:` comment says when nothing ever identified the
/// meter — a recording toggled on under Auto-detect before one answered.
const UNKNOWN_DEVICE: &str = "unknown";

/// Extension of a replay file: the dialog's filter and default name for one.
/// `--replay` reads the file's header, not its name.
const REPLAY_EXTENSION: &str = "replay";

/// Why the mock's samples cannot be written as a replay file. The menu entry
/// is disabled for the mock, so this only guards the call itself.
pub(super) const NO_WIRE_FORMAT: &str =
    "The mock has no wire format to export; connect a real meter";

/// Which file the export writes. Settled before the dialog opens — by the
/// Export… label (a CSV) or the menu on its arrow — because the dialog hands
/// back the path the user saved and not the file type they picked, and the
/// GTK chooser keeps the name's extension when its filter changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExportFormat {
    Csv,
    Json,
    Replay,
}

impl ExportFormat {
    /// The dialog's one filter: its label and extension.
    fn filter(self) -> (&'static str, &'static str) {
        match self {
            Self::Csv => ("CSV", "csv"),
            Self::Json => ("JSON", "json"),
            Self::Replay => ("Replay", REPLAY_EXTENSION),
        }
    }

    /// The name the dialog opens with. Shared with `dmm-cli read -o`, so an
    /// export saved here and one the CLI wrote sort together.
    fn default_name(self, model: &str, mode: Option<&str>, start: DateTime<Local>) -> String {
        dmm_shared::export::default_name(model, mode, start, self.filter().1)
    }
}

/// The mode the whole buffer stayed in, for the file name.
///
/// `None` once a recording crossed a function switch: naming that file after
/// the mode it started in would credit every later reading to it.
fn single_mode(samples: &VecDeque<Sample>) -> Option<&str> {
    let first = samples.front()?.measurement.mode.as_ref();
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
    /// On a recording's success, the buffer epoch written from and the
    /// samples written. Drives the recording's "saved" mark, so a buffer that
    /// reached a file doesn't prompt before being discarded.
    exported: Option<(u64, usize)>,
}

/// An export rendered and waiting for its save dialog: everything the writer
/// thread needs, none of it borrowed from the buffer.
pub(super) struct PreparedExport {
    format: ExportFormat,
    /// The name the dialog opens on.
    default_name: String,
    bytes: Vec<u8>,
    sample_count: usize,
    /// The buffer epoch to mark saved once the file is written; `None` for
    /// the history, which nothing asks about before dropping.
    mark: Option<u64>,
}

/// Write one export to the path the user chose and say how it went.
///
/// Writes a sibling .tmp and renames it into place, so a crash mid-export
/// can't leave a truncated file at the user-chosen path.
fn write_export(
    path: &Path,
    bytes: &[u8],
    sample_count: usize,
    mark: Option<u64>,
) -> ExportOutcome {
    export_outcome(
        path,
        dmm_shared::write_atomic(path, bytes),
        sample_count,
        mark,
    )
}

/// What the toast says, and what the recording marks saved, once the write
/// to `path` finished with `result`.
fn export_outcome(
    path: &Path,
    result: std::io::Result<()>,
    sample_count: usize,
    mark: Option<u64>,
) -> ExportOutcome {
    match result {
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
                exported: mark.map(|epoch| (epoch, sample_count)),
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
    /// Columns the buffered samples are written with.
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

    /// Render the buffer as `format` for the save dialog, or the toast that
    /// says why there is nothing to save.
    ///
    /// Kept apart from the dialog so what an export writes can be checked
    /// without opening one.
    pub(super) fn prepare_export(&self, format: ExportFormat) -> Result<PreparedExport, String> {
        let samples = &self.recording.samples;
        let role = self.recording.role();
        let Some(first) = samples.front() else {
            // Returning silently made the button and Ctrl+E look broken:
            // no file dialog, no message, nothing in the log. Say why.
            info!("export skipped: sample buffer is empty");
            return Err(self.nothing_to_export(role).to_string());
        };
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
        //
        // The name is built here too, rather than in the dialog thread: the
        // first sample is where the file starts.
        let default_name = format.default_name(device_model, single_mode(samples), first.wall_time);
        let bytes = match format {
            ExportFormat::Csv => {
                render_csv(samples, device_model, self.csv_layout()).map_err(|e| {
                    error!("CSV export failed: {e}");
                    format!("Export failed: {e}")
                })?
            }
            ExportFormat::Json => {
                render_json(samples, device_model, self.experimental()).into_bytes()
            }
            ExportFormat::Replay => self
                .replay_device_id()
                .and_then(|id| {
                    // What the samples came over, latched with the rest of
                    // the provenance; the live link only where a recording
                    // started before a meter answered.
                    let link = self.capture_layout.link.or(self.connection.link);
                    render_replay(samples, id, Some(device_model), link)
                })
                .ok_or_else(|| {
                    warn!("replay export refused: the buffered samples carry no meter frames");
                    NO_WIRE_FORMAT.to_string()
                })?
                .into_bytes(),
        };
        Ok(PreparedExport {
            format,
            default_name,
            bytes,
            sample_count: samples.len(),
            mark: (role == BufferRole::Recording).then(|| self.recording.epoch()),
        })
    }

    /// Why an empty buffer has nothing to save, as what to do about it.
    fn nothing_to_export(&self, role: BufferRole) -> &'static str {
        match role {
            BufferRole::Recording => "Nothing to export \u{2014} the recording has no samples yet",
            BufferRole::History if self.connection.state == ConnectionState::Disconnected => {
                "Nothing to export \u{2014} connect a meter first"
            }
            BufferRole::History => "Nothing to export \u{2014} no readings yet",
        }
    }

    pub(super) fn export_recording(&mut self, format: ExportFormat) {
        let prepared = match self.prepare_export(format) {
            Ok(prepared) => prepared,
            Err(message) => {
                self.toast = Some((message, true, Instant::now()));
                return;
            }
        };
        let (tx, rx) = std::sync::mpsc::channel::<ExportOutcome>();
        std::thread::spawn(move || {
            let PreparedExport {
                format,
                default_name,
                bytes,
                sample_count,
                mark,
            } = prepared;
            let (label, extension) = format.filter();
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter(label, &[extension])
                .save_file()
            else {
                return;
            };
            let _ = tx.send(write_export(&path, &bytes, sample_count, mark));
        });
        self.export_result_rx = Some(rx);
    }

    /// Whether the exported readings came off a protocol short of verified:
    /// what the recording latched from the meter it ran against, else the
    /// connection as it stands, as the meter's name falls back.
    pub(super) fn experimental(&self) -> bool {
        self.capture_layout
            .experimental
            .unwrap_or_else(|| !self.connection.stability.is_verified())
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
            if let Some((epoch, count)) = outcome.exported {
                // Samples that arrived while the export ran are not in that
                // file, so mark only what was actually written — and only if
                // the buffer still holds the recording it was written from.
                self.recording.mark_exported(epoch, count);
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
            ExportFormat::Json.default_name("UT61E+", Some("DC V"), start),
            "measurements-UT61E+-DC-V-2026-09-15_14-30-05.json"
        );
        assert_eq!(ExportFormat::Json.filter(), ("JSON", "json"));
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

    /// The mode names the file only while the whole recording stayed in it —
    /// a buffer that crossed a function switch is no one mode's.
    #[test]
    fn the_mode_names_the_file_only_while_the_buffer_holds_one() {
        let mut app = app_holding(0, 0, &[0, 0]);
        assert_eq!(single_mode(&app.recording.samples), Some("DC V"));
        app.recording.samples[1].measurement.mode = "AC V".into();
        assert_eq!(single_mode(&app.recording.samples), None);
    }

    /// Record works with nothing connected, and `disconnect()` puts the
    /// stability back to Verified — so a recording started before the meter
    /// answered exported a UT181A's readings as a verified protocol.
    #[test]
    fn a_recording_started_before_the_meter_answered_is_still_experimental() {
        use crate::app::connection::DmmMessage;
        use dmm_lib::protocol::Stability;

        // A named meter, so the Connected message below cannot reach the
        // detected-device save and write the real config file.
        let mut settings = Settings::default();
        settings.shared.device_family = "ut181a".to_string();
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());

        app.toggle_recording();
        assert_eq!(
            app.capture_layout.experimental, None,
            "nothing was connected to take a stability from"
        );

        // The meter answers, on a protocol no report has confirmed.
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(DmmMessage::Connected {
            name: "UT181A".to_string(),
            model_name: "UNI-T UT181A".to_string(),
            device_id: Some("ut181a"),
            stability: Stability::Experimental,
            feedback_url: String::new(),
            link: None,
            supported_commands: Vec::new(),
            max_aux_values: 0,
        })
        .expect("the channel is open");
        app.connection.rx = Some(rx);
        app.drain_messages();
        app.recording.push(&measurement(0), &app.wall_clock, 0);

        assert_eq!(app.capture_layout.experimental, Some(true));
        assert!(app.experimental(), "the recording ran against that meter");
        let json = render_json(&app.recording.samples, "UNI-T UT181A", app.experimental());
        let readings: Vec<&str> = json.lines().skip(1).collect();
        assert!(!readings.is_empty(), "got {json}");
        assert!(
            readings.iter().all(|l| l.contains("\"experimental\":true")),
            "got {json}"
        );

        // And it survives the cable coming out, which is why it is latched.
        app.disconnect();
        assert!(app.experimental(), "the samples are still that meter's");
    }

    /// Nothing buffered: no dialog, and a toast saying what to do instead.
    #[test]
    fn an_empty_buffer_prepares_no_export() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let message = |app: &App| match app.prepare_export(ExportFormat::Csv) {
            Ok(_) => panic!("an empty buffer has nothing to write"),
            Err(message) => message,
        };
        assert_eq!(
            message(&app),
            "Nothing to export \u{2014} connect a meter first"
        );
        app.connection.state = ConnectionState::Connected;
        assert_eq!(message(&app), "Nothing to export \u{2014} no readings yet");
        app.recording.toggle(Instant::now());
        assert_eq!(
            message(&app),
            "Nothing to export \u{2014} the recording has no samples yet"
        );
    }

    /// With nothing recorded, Export… saves the history the buffer holds —
    /// and marks nothing, so a recording started while its dialog is open
    /// still asks before it is discarded.
    #[test]
    fn a_history_export_saves_the_buffer_and_marks_nothing() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        for _ in 0..3 {
            app.recording.push(&measurement(0), &app.wall_clock, 0);
        }
        let prepared = app.prepare_export(ExportFormat::Csv).expect("the history");
        assert_eq!(prepared.sample_count, 3);
        assert_eq!(prepared.mark, None);
        assert_eq!(
            prepared.bytes,
            render_csv(&app.recording.samples, UNKNOWN_DEVICE, app.csv_layout())
                .expect("rendering the history")
        );

        app.recording.toggle(Instant::now());
        for _ in 0..3 {
            app.recording.push(&measurement(0), &app.wall_clock, 0);
        }
        let outcome = export_outcome(
            Path::new("out.csv"),
            Ok(()),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.recording.unexported_count(), 3);
        assert_eq!(
            app.toast.as_ref().map(|(text, _, _)| text.as_str()),
            Some("Exported 3 samples to out.csv"),
            "the user still hears the file was written"
        );
    }

    /// The prepared file is the buffer as the renderer writes it, named after
    /// its first sample.
    #[test]
    fn a_prepared_export_holds_the_rendered_buffer() {
        let app = app_holding(0, 0, &[0, 0, 0]);
        let prepared = app
            .prepare_export(ExportFormat::Csv)
            .expect("three samples to write");
        assert_eq!(prepared.sample_count, 3);
        assert_eq!(prepared.mark, Some(app.recording.epoch()));
        // Nothing named a meter, so the file says so.
        assert!(
            prepared
                .default_name
                .starts_with("measurements-unknown-DC-V-"),
            "{}",
            prepared.default_name
        );
        assert!(prepared.default_name.ends_with(".csv"));
        let rendered = render_csv(&app.recording.samples, UNKNOWN_DEVICE, app.csv_layout())
            .expect("rendering the fixture buffer");
        assert_eq!(prepared.bytes, rendered);
    }

    /// Hand `outcome` to the app the way the writer thread does.
    fn deliver_outcome(app: &mut App, outcome: ExportOutcome) {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(outcome).expect("the channel is open");
        app.export_result_rx = Some(rx);
        app.poll_export_result();
    }

    /// A written file marks what it holds, so Record doesn't ask about it.
    #[test]
    fn a_finished_export_marks_its_samples_saved() {
        let mut app = app_holding(0, 0, &[0, 0, 0]);
        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        let outcome = export_outcome(
            Path::new("out.csv"),
            Ok(()),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.recording.unexported_count(), 0);
        assert_eq!(
            app.toast
                .as_ref()
                .map(|(text, is_error, _)| (text.as_str(), *is_error)),
            Some(("Exported 3 samples to out.csv", false))
        );
    }

    /// The save dialog leaves the window live, so a new recording can start
    /// before the file is written. That file holds the old samples: the new
    /// ones must still count as unexported, or Record discards them unasked.
    #[test]
    fn a_finished_export_does_not_mark_a_recording_started_after_it() {
        let mut app = app_holding(0, 0, &[0, 0, 0]);
        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        app.recording.toggle(Instant::now()); // stop
        app.recording.toggle(Instant::now()); // a new recording
        app.recording.push(&measurement(0), &app.wall_clock, 0);
        let outcome = export_outcome(
            Path::new("out.csv"),
            Ok(()),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.recording.unexported_count(), 1);
    }

    /// A failed write marks nothing and says why.
    #[test]
    fn a_failed_export_marks_nothing() {
        let mut app = app_holding(0, 0, &[0, 0]);
        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        let outcome = export_outcome(
            Path::new("out.csv"),
            Err(std::io::Error::other("disk full")),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.recording.unexported_count(), 2);
        assert_eq!(
            app.toast
                .as_ref()
                .map(|(text, is_error, _)| (text.as_str(), *is_error)),
            Some(("Export failed: disk full", true))
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
