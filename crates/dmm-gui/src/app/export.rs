//! Export: the sample buffer — the recording, or with nothing recorded the
//! samples the graph holds — as a CSV, a JSON document or a replay file. The
//! save dialog runs off the UI thread; the render runs once it returns, from
//! the samples pinned at the click; the write runs off the UI thread again,
//! and its outcome comes back as a toast.

use chrono::{DateTime, Local};
use dmm_lib::measurement::MeasuredValue;
use dmm_lib::transport::Link;
use dmm_shared::export::CsvLayout;
use eframe::egui;
use log::{error, info, warn};
use std::collections::HashSet;
use std::collections::vec_deque;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use super::connection::panic_text;
use super::toast::Toast;
use super::{App, ConnectionState};
use crate::markers::Marker;
use crate::recording::{BufferRole, Recording, Sample, render_csv, render_json, render_replay};

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

/// Why Export… does nothing while an earlier export waits on its dialog or
/// its write. Not about the dialog alone: a write has none to point at.
const EXPORT_IN_PROGRESS: &str = "An export is already in progress \u{2014} finish it first";

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
///
/// A word the meter shows instead of a reading ("Auto" with the probes
/// lifted) comes under a mode of its own but is no switch, so it is left out
/// — as `dmm-cli read -o` leaves it out of the name it picks.
fn single_mode(samples: std::collections::vec_deque::Iter<'_, Sample>) -> Option<&str> {
    let mut modes = samples
        .filter(|s| !matches!(s.measurement.value, MeasuredValue::NoReading(_)))
        .map(|s| s.measurement.mode.as_ref());
    let first = modes.next()?;
    modes.all(|mode| mode == first).then_some(first)
}

/// What a finished export of a recording marks saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SavedMark {
    /// The buffer epoch written from.
    epoch: u64,
    /// The recording's markers as rendered, for a format that writes
    /// markers; `None` for a replay file, which saves none.
    markers: Option<HashSet<u64>>,
}

/// What the replay export's toast adds when the buffer had markers.
const REPLAY_DROPS_MARKERS: &str =
    ". Replay files don't keep markers: export CSV or JSON to keep them.";

/// Result of an export, sent from the writer thread to the UI.
pub(super) struct ExportOutcome {
    /// Toast text.
    message: String,
    is_error: bool,
    /// On a recording's success, what was saved and the samples written.
    /// Drives the recording's "saved" mark, so a buffer that reached a file
    /// doesn't prompt before being discarded.
    exported: Option<(SavedMark, usize)>,
}

/// An export as the click asked for it, waiting on its save dialog:
/// everything its render reads besides the samples, frozen then — which
/// meter, the columns, the markers — since all of it can change while the
/// dialog is open. The samples themselves are pinned in the
/// [`Recording`].
pub(super) struct ExportRequest {
    format: ExportFormat,
    /// The name the dialog opens on.
    default_name: String,
    sample_count: usize,
    /// What to mark saved once the file is written; `None` for the history,
    /// which nothing asks about before dropping.
    mark: Option<SavedMark>,
    /// A replay file of a buffer with markers, which it leaves out: the
    /// toast says so.
    drops_markers: bool,
    device_model: &'static str,
    /// The CSV's columns, marker ones included when there are markers.
    csv_layout: CsvLayout,
    experimental: bool,
    /// A replay file's `# device:` id and link; `None` for any other format.
    replay: Option<(&'static str, Option<Link>)>,
    /// The markers on the exported samples, oldest first, as they were at
    /// the click.
    marked: Vec<Marker>,
}

impl ExportRequest {
    /// The file's bytes, from `samples`: the ones pinned at the click.
    fn render(&self, samples: vec_deque::Iter<'_, Sample>) -> Result<Vec<u8>, String> {
        let marked: Vec<&Marker> = self.marked.iter().collect();
        match self.format {
            ExportFormat::Csv => render_csv(samples, &marked, self.device_model, self.csv_layout)
                .map_err(|e| {
                    error!("CSV export failed: {e}");
                    format!("Export failed: {e}")
                }),
            ExportFormat::Json => {
                render_json(samples, &marked, self.device_model, self.experimental).map_err(|e| {
                    error!("JSON export failed: {e}");
                    format!("Export failed: {e}")
                })
            }
            ExportFormat::Replay => self
                .replay
                .and_then(|(id, link)| render_replay(samples, id, Some(self.device_model), link))
                .map(String::into_bytes)
                .ok_or_else(|| {
                    warn!("replay export refused: the buffered samples carry no meter frames");
                    NO_WIRE_FORMAT.to_string()
                }),
        }
    }
}

/// What the save dialog answered: the path picked, `None` when dismissed,
/// or the text of a panic in the dialog thread.
type DialogResult = Result<Option<PathBuf>, String>;

/// An export under way: at most one at a time.
pub(super) enum PendingExport {
    /// Its save dialog is open.
    Choosing {
        request: ExportRequest,
        rx: mpsc::Receiver<DialogResult>,
    },
    /// Rendered and being written.
    Writing(mpsc::Receiver<ExportOutcome>),
}

/// Write one export to the path the user chose and say how it went.
///
/// Writes a sibling .tmp and renames it into place, so a crash mid-export
/// can't leave a truncated file at the user-chosen path.
fn write_export(
    path: &Path,
    bytes: &[u8],
    sample_count: usize,
    mark: Option<SavedMark>,
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
    mark: Option<SavedMark>,
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
                exported: mark.map(|mark| (mark, sample_count)),
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
            family_slots: self.export_layout().aux_slots.max(
                self.capture
                    .recording
                    .max_aux_seen()
                    .saturating_sub(self.export_layout().extra_slots),
            ),
            extra_slots: self.export_layout().extra_slots,
            // Integrating is a CLI-only run mode.
            integral: false,
            // Set per export, from the markers it writes.
            markers: false,
        }
    }

    /// What an export of the buffer as `format` needs besides the samples,
    /// or the toast that says why there is nothing to save.
    ///
    /// Kept apart from the dialog so what an export writes can be checked
    /// without opening one.
    pub(super) fn prepare_export(&self, format: ExportFormat) -> Result<ExportRequest, String> {
        // A fresh walk over the samples for each pass that reads them: a
        // borrowed iterator, nothing copied.
        let samples = || self.capture.recording.export_samples();
        let role = self.capture.recording.role();
        let Some(first) = samples().next() else {
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
            .export_layout()
            .device
            .or_else(|| self.active_device().map(|d| d.display_name))
            .unwrap_or(UNKNOWN_DEVICE);
        // A replay is refused now rather than after the user picked a path:
        // the mock synthesises its readings, so a sample of it has no frame.
        let replay = match format {
            ExportFormat::Replay => {
                let id = self
                    .replay_device_id()
                    .filter(|_| samples().all(|s| !s.measurement.raw_payload.is_empty()));
                let Some(id) = id else {
                    warn!("replay export refused: the buffered samples carry no meter frames");
                    return Err(NO_WIRE_FORMAT.to_string());
                };
                // What the samples came over, latched with the rest of the
                // provenance; the live link only where a recording started
                // before a meter answered.
                Some((id, self.export_layout().link.or(self.connection.link())))
            }
            _ => None,
        };

        // The file is rendered only once the dialog returns a path, from the
        // samples pinned now: nothing is held while the dialog is open, and
        // the file still holds what the buffer held at the click, whatever
        // arrives, is dropped or discarded meanwhile.
        //
        // The name is built here, rather than in the dialog thread: the
        // first sample is where the file starts.
        let default_name =
            format.default_name(device_model, single_mode(samples()), first.wall_time);
        let marked = self.capture.recording.marked(self.markers.iter());
        Ok(ExportRequest {
            format,
            default_name,
            sample_count: samples().len(),
            mark: (role == BufferRole::Recording).then(|| SavedMark {
                epoch: self.capture.recording.epoch(),
                markers: (format != ExportFormat::Replay).then(|| Recording::marker_keys(&marked)),
            }),
            drops_markers: format == ExportFormat::Replay && !marked.is_empty(),
            device_model,
            csv_layout: CsvLayout {
                markers: !marked.is_empty(),
                ..self.csv_layout()
            },
            experimental: self.experimental(),
            replay,
            marked: marked.into_iter().cloned().collect(),
        })
    }

    /// What the samples Export… saves came from and are laid out as: the
    /// recording's, or with none, the history's.
    fn export_layout(&self) -> &super::capture::CaptureLayout {
        match self.capture.recording.role() {
            BufferRole::Recording => &self.capture.recording_layout,
            BufferRole::History => &self.capture.history_layout,
        }
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

    /// Take the click: pin the samples, and hand back the channel the save
    /// dialog answers on and the name it opens with. Refused while an
    /// earlier export is under way, or with nothing to save.
    ///
    /// Opens no dialog, so tests drive the answer by hand.
    fn begin_export(
        &mut self,
        format: ExportFormat,
    ) -> Result<(mpsc::Sender<DialogResult>, String), String> {
        if self.export.is_some() {
            return Err(EXPORT_IN_PROGRESS.to_string());
        }
        let request = self.prepare_export(format)?;
        let pinned = self.capture.recording.pin_export();
        debug_assert_eq!(pinned, request.sample_count);
        let (tx, rx) = mpsc::channel();
        info!(
            "export of {} samples: waiting for the save dialog",
            request.sample_count
        );
        let name = request.default_name.clone();
        // Stored here, with the pin, so the two cannot come apart.
        self.export = Some(PendingExport::Choosing { request, rx });
        Ok((tx, name))
    }

    pub(super) fn export_recording(&mut self, ctx: &egui::Context, format: ExportFormat) {
        let (tx, default_name) = match self.begin_export(format) {
            Ok(begun) => begun,
            Err(message) => {
                self.toast = Some(Toast::error(message));
                return;
            }
        };
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (label, extension) = format.filter();
            let answer = std::panic::catch_unwind(|| {
                rfd::FileDialog::new()
                    .set_file_name(default_name)
                    .add_filter(label, &[extension])
                    .save_file()
            })
            .map_err(|panic| panic_text(panic.as_ref()));
            let _ = tx.send(answer);
            // The answer is polled from `ui`; a paused app draws nothing
            // until asked.
            ctx.request_repaint();
        });
    }

    /// Whether the exported readings came off a protocol short of verified:
    /// what the recording latched from the meter it ran against, else the
    /// connection as it stands, as the meter's name falls back.
    pub(super) fn experimental(&self) -> bool {
        self.export_layout()
            .experimental
            .unwrap_or_else(|| !self.connection.stability().is_verified())
    }

    /// The meter whose frames a replay file would carry: the one the
    /// recording named, else the one selected or detected now, as the CSV's
    /// provenance falls back. `None` for the mock, which has no wire format.
    pub(super) fn replay_device_id(&self) -> Option<&'static str> {
        self.export_layout().device_id.or_else(|| {
            self.active_device()
                .filter(|d| d.requires_hardware)
                .map(|d| d.id)
        })
    }

    /// Fold in whatever the pending export's dialog or writer answered.
    pub(super) fn poll_export(&mut self, ctx: &egui::Context) {
        match &self.export {
            None => {}
            Some(PendingExport::Choosing { rx, .. }) => {
                let answer = match rx.try_recv() {
                    Err(mpsc::TryRecvError::Empty) => return,
                    Ok(answer) => answer,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        Err("the save dialog closed without an answer".to_string())
                    }
                };
                let Some(PendingExport::Choosing { request, .. }) = self.export.take() else {
                    return;
                };
                match answer {
                    Ok(Some(path)) => match self.render_pinned(&request) {
                        Ok(bytes) => self.spawn_write(ctx, path, bytes, request),
                        Err(message) => self.toast = Some(Toast::error(message)),
                    },
                    Ok(None) => {
                        self.capture.recording.unpin_export();
                        info!("export cancelled");
                    }
                    Err(message) => {
                        self.capture.recording.unpin_export();
                        error!("export failed: {message}");
                        self.toast = Some(Toast::error(format!("Export failed: {message}")));
                    }
                }
            }
            Some(PendingExport::Writing(rx)) => {
                let outcome = match rx.try_recv() {
                    Err(mpsc::TryRecvError::Empty) => return,
                    Ok(outcome) => outcome,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // The pin went with the render; releasing it again
                        // is a no-op, but this arm must not leave the next
                        // Export… refused as one in progress.
                        self.capture.recording.unpin_export();
                        let message = "the writer stopped without an answer";
                        error!("export failed: {message}");
                        ExportOutcome {
                            message: format!("Export failed: {message}"),
                            is_error: true,
                            exported: None,
                        }
                    }
                };
                if let Some((mark, count)) = outcome.exported {
                    // Samples that arrived while the export ran are not in
                    // that file, so mark only what was actually written — and
                    // only if the buffer still holds the recording it was
                    // written from.
                    self.capture
                        .recording
                        .mark_exported(mark.epoch, count, mark.markers);
                }
                self.toast = Some(if outcome.is_error {
                    Toast::error(outcome.message)
                } else {
                    Toast::info(outcome.message)
                });
                self.export = None;
            }
        }
    }

    /// Render `request` from the samples pinned at its click, then let them
    /// go, whatever the result.
    fn render_pinned(&mut self, request: &ExportRequest) -> Result<Vec<u8>, String> {
        let samples = self.capture.recording.pinned_samples();
        let result = if samples.len() == request.sample_count {
            request.render(samples)
        } else {
            // Never a short file under a toast claiming the full count.
            error!(
                "export failed: {} of {} pinned samples left",
                samples.len(),
                request.sample_count
            );
            Err("Export failed: the samples changed while the save dialog was open".to_string())
        };
        self.capture.recording.unpin_export();
        result
    }

    /// Write `bytes` to `path` off the UI thread; [`App::poll_export`]
    /// picks up the outcome.
    fn spawn_write(
        &mut self,
        ctx: &egui::Context,
        path: PathBuf,
        bytes: Vec<u8>,
        request: ExportRequest,
    ) {
        let (tx, rx) = mpsc::channel::<ExportOutcome>();
        let ctx = ctx.clone();
        let ExportRequest {
            sample_count,
            mark,
            drops_markers,
            ..
        } = request;
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut outcome = write_export(&path, &bytes, sample_count, mark);
                if drops_markers && !outcome.is_error {
                    outcome.message.push_str(REPLAY_DROPS_MARKERS);
                }
                outcome
            }))
            .unwrap_or_else(|panic| {
                let message = panic_text(panic.as_ref());
                error!("export failed: {message}");
                ExportOutcome {
                    message: format!("Export failed: {message}"),
                    is_error: true,
                    exported: None,
                }
            });
            let _ = tx.send(outcome);
            // The result is polled from `ui`; nothing else may be drawing.
            ctx.request_repaint();
        });
        self.export = Some(PendingExport::Writing(rx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::connection::ConnectedMeter;
    use crate::settings::Settings;
    use chrono::TimeZone;
    use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
    use std::collections::VecDeque;
    use std::time::Instant;

    /// A 1.234 V reading carrying `aux` sub-values of its own.
    fn measurement(aux: usize) -> Measurement {
        let mut m =
            dmm_lib::protocol::make_test_measurement(0x02, 0x01, b"  1.234", (0, 0), (0, 0, 0));
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
        app.capture.recording_layout.aux_slots = aux_slots;
        app.capture.recording_layout.extra_slots = extra_slots;
        app.capture.recording.toggle(std::time::Instant::now());
        for &aux in aux_counts {
            app.capture
                .recording
                .push(&measurement(aux), extra_slots.min(aux));
        }
        app
    }

    /// Readings `i` seconds after `t0`, pushed as a frame drains them.
    fn push_at(app: &mut App, t0: Instant, seconds: std::ops::Range<u64>) -> Vec<Instant> {
        seconds
            .map(|i| {
                let mut m = measurement(0);
                m.timestamp = t0 + std::time::Duration::from_secs(i);
                app.capture.recording.push(&m, 0);
                m.timestamp
            })
            .collect()
    }

    /// Discard leaves the recording's readings to the history, and Export…
    /// saves them with the ones before, markers included.
    #[test]
    fn export_after_discard_saves_the_graphs_readings_and_markers() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let t0 = Instant::now();
        let before = push_at(&mut app, t0, 0..2);
        app.markers
            .add(before[1], chrono::Local::now(), "1.234 V".into())
            .expect("a new marker");
        app.capture.recording.toggle(t0);
        push_at(&mut app, t0, 2..4);
        app.capture.recording.toggle(t0);
        app.capture.recording.discard();

        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        assert_eq!(prepared.sample_count, 4);
        assert!(prepared.mark.is_none(), "the history marks nothing saved");
        let bytes = prepared
            .render(app.capture.recording.export_samples())
            .expect("rendering the history");
        let text = String::from_utf8(bytes).expect("CSV is UTF-8");
        let mut lines = text.lines().skip(1);
        assert!(lines.next().expect("a header").ends_with(",marker,note"));
        assert_eq!(lines.filter(|l| l.ends_with(",1,")).count(), 1, "{text}");
    }

    /// A stopped recording is what Export… saves, not the readings the
    /// history took after it.
    #[test]
    fn a_stopped_recording_exports_only_itself() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let t0 = Instant::now();
        push_at(&mut app, t0, 0..3);
        app.capture.recording.toggle(t0);
        push_at(&mut app, t0, 3..5);
        app.capture.recording.toggle(t0);
        push_at(&mut app, t0, 5..8);
        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        assert_eq!(prepared.sample_count, 2);
    }

    /// The header and every row, as comma-separated cells.
    fn exported(app: &App) -> Vec<Vec<String>> {
        let bytes = render_csv(
            app.capture.recording.export_samples(),
            &[],
            "mock",
            app.csv_layout(),
        )
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
        let app = app_holding(0, 0, &[0, 0]);
        let mut samples: VecDeque<Sample> =
            app.capture.recording.export_samples().cloned().collect();
        assert_eq!(single_mode(samples.iter()), Some("DC V"));
        samples[1].measurement.mode = "AC V".into();
        assert_eq!(single_mode(samples.iter()), None);
    }

    /// A no-reading word between two readings of one mode is no switch.
    #[test]
    fn a_no_reading_leaves_the_mode_in_the_name() {
        let app = app_holding(0, 0, &[0, 0, 0]);
        let mut samples: VecDeque<Sample> =
            app.capture.recording.export_samples().cloned().collect();
        let idle = &mut samples[1].measurement;
        idle.mode = "Auto".into();
        idle.value = MeasuredValue::NoReading("Auto");
        assert_eq!(single_mode(samples.iter()), Some("DC V"));
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
            app.capture.recording_layout.experimental, None,
            "nothing was connected to take a stability from"
        );

        // The meter answers, on a protocol no report has confirmed.
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(DmmMessage::Connected(ConnectedMeter {
            name: Some("UT181A".to_string()),
            model_name: "UNI-T UT181A".to_string(),
            stability: Stability::Experimental,
            ..ConnectedMeter::test_fixture(Some("ut181a"))
        }))
        .expect("the channel is open");
        app.connection.rx = Some(rx);
        app.drain_messages();
        app.capture.recording.push(&measurement(0), 0);

        assert_eq!(app.capture.recording_layout.experimental, Some(true));
        assert!(app.experimental(), "the recording ran against that meter");
        let json = String::from_utf8(
            render_json(
                app.capture.recording.export_samples(),
                &[],
                "UNI-T UT181A",
                app.experimental(),
            )
            .unwrap(),
        )
        .unwrap();
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
        app.capture.recording.toggle(Instant::now());
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
            app.capture.recording.push(&measurement(0), 0);
        }
        let prepared = app.prepare_export(ExportFormat::Csv).expect("the history");
        assert_eq!(prepared.sample_count, 3);
        assert_eq!(prepared.mark, None);
        assert_eq!(
            prepared
                .render(app.capture.recording.export_samples())
                .expect("rendering the history"),
            render_csv(
                app.capture.recording.export_samples(),
                &[],
                UNKNOWN_DEVICE,
                app.csv_layout()
            )
            .expect("rendering the history")
        );

        app.capture.recording.toggle(Instant::now());
        for _ in 0..3 {
            app.capture.recording.push(&measurement(0), 0);
        }
        let outcome = export_outcome(
            Path::new("out.csv"),
            Ok(()),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.capture.recording.unexported_count(), 3);
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Exported 3 samples to out.csv"),
            "the user still hears the file was written"
        );
    }

    /// The prepared file is the buffer as the renderer writes it, named after
    /// its first sample.
    #[test]
    fn a_prepared_export_renders_the_buffer() {
        let app = app_holding(0, 0, &[0, 0, 0]);
        let prepared = app
            .prepare_export(ExportFormat::Csv)
            .expect("three samples to write");
        assert_eq!(prepared.sample_count, 3);
        assert_eq!(
            prepared.mark,
            Some(SavedMark {
                epoch: app.capture.recording.epoch(),
                markers: Some(Recording::marker_keys(
                    &app.capture.recording.marked(app.markers.iter())
                )),
            })
        );
        // Nothing named a meter, so the file says so.
        assert!(
            prepared
                .default_name
                .starts_with("measurements-unknown-DC-V-"),
            "{}",
            prepared.default_name
        );
        assert!(prepared.default_name.ends_with(".csv"));
        let rendered = render_csv(
            app.capture.recording.export_samples(),
            &[],
            UNKNOWN_DEVICE,
            app.csv_layout(),
        )
        .expect("rendering the fixture buffer");
        assert_eq!(
            prepared.render(app.capture.recording.export_samples()),
            Ok(rendered)
        );
    }

    /// A replay file has nowhere to put markers: it saves none of them, and
    /// its toast says so.
    #[test]
    fn a_replay_export_leaves_the_markers_out() {
        let mut app = app_holding(0, 0, &[0, 0]);
        app.capture.recording_layout.device_id = Some("ut61eplus");
        let prepared = app.prepare_export(ExportFormat::Replay).expect("frames");
        assert!(!prepared.drops_markers, "no markers to leave out");

        app.last_measurement = app
            .capture
            .recording
            .export_samples()
            .last()
            .map(|s| s.measurement.clone());
        app.add_marker(false);
        let prepared = app.prepare_export(ExportFormat::Replay).expect("frames");
        assert!(prepared.drops_markers);
        assert_eq!(prepared.mark.map(|m| m.markers), Some(None));
        let prepared = app.prepare_export(ExportFormat::Csv).expect("samples");
        assert_eq!(
            prepared.mark.map(|m| m.markers),
            Some(Some(Recording::marker_keys(
                &app.capture.recording.marked(app.markers.iter())
            )))
        );
    }

    /// Hand `outcome` to the app the way the writer thread does.
    fn deliver_outcome(app: &mut App, outcome: ExportOutcome) {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(outcome).expect("the channel is open");
        app.export = Some(PendingExport::Writing(rx));
        app.poll_export(&egui::Context::default());
    }

    /// The request of the export waiting on its dialog, taken out of the
    /// app as `poll_export` does when a path comes back.
    fn take_request(app: &mut App) -> ExportRequest {
        match app.export.take() {
            Some(PendingExport::Choosing { request, .. }) => request,
            _ => panic!("no export waiting on its dialog"),
        }
    }

    /// The file holds the samples and markers of the click, whatever the
    /// buffer went through while the dialog was open.
    #[test]
    fn an_export_writes_the_samples_at_the_click() {
        let mut app = app_holding(0, 0, &[0, 0, 0]);
        let expected = render_csv(
            app.capture.recording.export_samples(),
            &[],
            UNKNOWN_DEVICE,
            app.csv_layout(),
        )
        .expect("rendering the fixture buffer");
        app.begin_export(ExportFormat::Csv).expect("three samples");
        assert!(app.capture.recording.is_pinned());

        let t0 = Instant::now();
        push_at(&mut app, t0, 10..12);
        app.last_measurement = app
            .capture
            .recording
            .export_samples()
            .last()
            .map(|s| s.measurement.clone());
        app.add_marker(false);
        app.capture.recording.toggle(Instant::now()); // stop
        app.capture.recording.discard();

        let request = take_request(&mut app);
        assert_eq!(app.render_pinned(&request), Ok(expected));
        assert!(!app.capture.recording.is_pinned());
        assert_ne!(
            request.mark.map(|m| m.epoch),
            Some(app.capture.recording.epoch()),
            "the discarded recording's export marks nothing"
        );

        // A marker placed before the click keeps the note it had then.
        let mut app = app_holding(0, 0, &[0, 0]);
        app.last_measurement = app
            .capture
            .recording
            .export_samples()
            .last()
            .map(|s| s.measurement.clone());
        app.add_marker(false);
        let marked = app.capture.recording.marked(app.markers.iter());
        let expected = render_json(
            app.capture.recording.export_samples(),
            &marked,
            UNKNOWN_DEVICE,
            app.experimental(),
        )
        .expect("rendering the fixture buffer");
        app.begin_export(ExportFormat::Json).expect("two samples");
        for m in app.markers.iter_mut() {
            m.note = "written after the click".into();
        }
        let request = take_request(&mut app);
        let bytes = app.render_pinned(&request).expect("the pinned samples");
        assert_eq!(bytes, expected);
        assert!(
            !String::from_utf8(bytes)
                .unwrap()
                .contains("after the click")
        );
    }

    /// A second Export… while one waits on its dialog is refused, and the
    /// first keeps its samples.
    #[test]
    fn a_second_export_waits_for_the_first() {
        let mut app = app_holding(0, 0, &[0, 0]);
        app.begin_export(ExportFormat::Csv).expect("two samples");
        assert_eq!(
            app.begin_export(ExportFormat::Json).map(|_| ()),
            Err(EXPORT_IN_PROGRESS.to_string())
        );
        assert!(matches!(app.export, Some(PendingExport::Choosing { .. })));
        assert!(app.capture.recording.is_pinned());
    }

    /// A dialog dismissed, panicked or gone lets the samples go, so the
    /// next Export… opens one.
    #[test]
    fn a_dismissed_or_failed_dialog_releases_the_pin() {
        let mut app = app_holding(0, 0, &[0, 0]);
        let ctx = egui::Context::default();

        let (tx, _) = app.begin_export(ExportFormat::Csv).expect("two samples");
        tx.send(Ok(None)).expect("the app listens");
        app.poll_export(&ctx);
        assert!(!app.capture.recording.is_pinned());
        assert!(app.export.is_none());
        assert!(app.toast.is_none(), "a cancel needs no toast");

        let (tx, _) = app.begin_export(ExportFormat::Csv).expect("two samples");
        tx.send(Err("boom".into())).expect("the app listens");
        app.poll_export(&ctx);
        assert!(!app.capture.recording.is_pinned());
        assert_eq!(
            app.toast.take().map(|t| (t.message, t.is_error)),
            Some(("Export failed: boom".to_string(), true))
        );

        let (tx, _) = app.begin_export(ExportFormat::Csv).expect("two samples");
        drop(tx);
        app.poll_export(&ctx);
        assert!(!app.capture.recording.is_pinned());
        assert!(app.export.is_none());
        assert!(app.toast.is_some_and(|t| t.is_error));
    }

    /// A writer gone without an answer ends the export with an error, so
    /// the next Export… is not refused as one still in progress.
    #[test]
    fn a_writer_gone_without_an_answer_ends_the_export() {
        let mut app = app_holding(0, 0, &[0, 0]);
        let (tx, rx) = std::sync::mpsc::channel::<ExportOutcome>();
        drop(tx);
        app.export = Some(PendingExport::Writing(rx));
        app.poll_export(&egui::Context::default());
        assert!(app.export.is_none());
        assert!(app.toast.take().is_some_and(|t| t.is_error));
        assert!(app.begin_export(ExportFormat::Csv).is_ok());
    }

    /// A replay of samples with no meter frame is refused at the click, not
    /// after the user picked a path.
    #[test]
    fn a_replay_of_frameless_samples_is_refused_at_the_click() {
        let mut app = app_holding(0, 0, &[0]);
        app.capture.recording_layout.device_id = Some("ut61eplus");
        let mut frameless = measurement(0);
        frameless.raw_payload.clear();
        app.capture.recording.push(&frameless, 0);
        assert_eq!(
            app.prepare_export(ExportFormat::Replay).map(|_| ()),
            Err(NO_WIRE_FORMAT.to_string())
        );
        assert!(app.begin_export(ExportFormat::Replay).is_err());
        assert!(!app.capture.recording.is_pinned());
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
        assert_eq!(app.capture.recording.unexported_count(), 0);
        assert_eq!(
            app.toast.as_ref().map(|t| (t.message.as_str(), t.is_error)),
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
        app.capture.recording.toggle(Instant::now()); // stop
        app.capture.recording.toggle(Instant::now()); // a new recording
        app.capture.recording.push(&measurement(0), 0);
        let outcome = export_outcome(
            Path::new("out.csv"),
            Ok(()),
            prepared.sample_count,
            prepared.mark,
        );
        deliver_outcome(&mut app, outcome);
        assert_eq!(app.capture.recording.unexported_count(), 1);
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
        assert_eq!(app.capture.recording.unexported_count(), 2);
        assert_eq!(
            app.toast.as_ref().map(|t| (t.message.as_str(), t.is_error)),
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
