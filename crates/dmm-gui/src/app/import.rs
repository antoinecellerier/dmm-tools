//! Import…: an exported CSV, JSON or replay file read back into a session of
//! its own, with its markers.
//!
//! The file is parsed on a thread of its own and its readings stamped there;
//! the frame loop then takes them through the pipeline a chunk at a time, so
//! a half-million-reading file neither freezes the window nor arrives as one
//! enormous frame. They go into a recording, stopped and marked saved: a
//! mode or quantity change inside the file restarts the graph's trace as a
//! live one would, and the recording keeps every reading for Export…
//! regardless.
//!
//! No meter is attached, so nothing new arrives; the readings, markers and
//! view can still be worked on and exported again.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use dmm_lib::measurement::Measurement;
use dmm_lib::transport::Link;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui;
use log::{error, info, warn};

use super::App;
use super::capture::CaptureLayout;
use super::task::Task;
use super::toast::Toast;
use crate::recording::Recording;

/// Readings taken through the pipeline per frame: a fraction of a frame's
/// time at release speed, so a large file fills in over a second or two with
/// the window still answering.
const CHUNK: usize = 20_000;

/// Where the session's readings come from when it is not a meter: the
/// `--replay` recording it plays, or a file Import… takes in.
#[derive(Default)]
pub(super) struct SessionSource {
    /// The recording this session plays instead of opening a meter, from
    /// `--replay`. Session-only, like the clock: the meter it names reaches
    /// the settings as an override, so nothing about a playback is saved.
    pub(super) replay: Option<crate::ReplaySource>,
    /// The `--replay` file's markers still waiting for their reading, in
    /// offset order: each goes on the first reading played at or after its
    /// offset (see `place_replay_marker`).
    pub(super) replay_markers: std::collections::VecDeque<dmm_lib::replay::ReplayMarker>,
    /// The `--replay` file's saved view, waiting until the playback reaches
    /// the furthest moment it shows (see `view_reach`).
    pub(super) replay_view: Option<(crate::graph::ViewState, Duration)>,
    /// An import cleared a `--replay` session: its next Connect queues the
    /// recording's markers and view again.
    pub(super) requeue_replay: bool,
    /// `--import`'s file, imported on the first frame.
    pub(super) cli_import: Option<PathBuf>,
    /// Import…'s open dialog, answering on its own thread: the file
    /// picked, `None` when dismissed.
    pub(super) import_dialog: Option<Task<Option<PathBuf>>>,
    /// An Import… under way: the file parsing, or its readings going in.
    pub(super) import_job: Option<ImportJob>,
    /// The file the session was imported from, once it is in.
    pub(super) imported: Option<ImportedFrom>,
    /// How far apart the imported file's readings are, in milliseconds:
    /// what the graph judges its gaps by while the session is that file's.
    pub(super) import_cadence_ms: Option<u32>,
}

impl SessionSource {
    /// The session is an imported file's, in or still coming in.
    pub(super) fn is_import(&self) -> bool {
        self.imported.is_some() || self.import_job.is_some()
    }

    /// Forget the last session's import and replay progress: an import, or
    /// the first Connect after one, starts from nothing. The `--replay`
    /// recording stays, and its markers and view go back in with its next
    /// Connect.
    fn reset(&mut self) {
        self.replay_markers.clear();
        self.replay_view = None;
        self.requeue_replay = self.replay.is_some();
        self.import_job = None;
        self.imported = None;
        self.import_cadence_ms = None;
    }
}

/// What the top bar and the recording panel say about an imported session.
#[derive(Debug, Clone)]
pub(super) struct ImportedFrom {
    pub(super) path: PathBuf,
    /// Readings the session took from the file.
    pub(super) readings: usize,
}

/// A file parsed and stamped on the loader thread.
pub(super) struct Loaded {
    path: PathBuf,
    /// The meter the file names, as it names it.
    device: Option<String>,
    /// Registry id of that meter, when the file holds its frames (a replay)
    /// and so can be exported as a replay again.
    device_id: Option<&'static str>,
    /// The name the meter gave for itself, kept by a replay file.
    model: Option<String>,
    link: Option<Link>,
    experimental: bool,
    /// Stamped `base + offset`, in file order, time never going back, each
    /// with how many of its sub-values a scale appended.
    readings: Vec<(Measurement, usize)>,
    /// `(reading index, number, note)`, in reading order.
    markers: Vec<(usize, u32, String)>,
    /// How far apart the file's readings are, for the graph's gaps.
    cadence: Duration,
    /// The CSV sub-value groups to keep: the meter's own, then those for the
    /// sub-values a scale appended.
    family_slots: usize,
    extra_slots: usize,
    /// The graph view the file was saved with, as its JSON text.
    view: Option<String>,
    /// Readings past the limit the file was read with, left out.
    left_out: usize,
}

/// Why a file did not import. `Display` is the English the log takes;
/// [`ImportError::text`] what the toast shows, in the GUI's language.
#[derive(Debug)]
pub(super) enum ImportError {
    /// The file, then why it does not read.
    Read(PathBuf, Unreadable),
    /// The loader thread panicked reading the file, saying this.
    Crashed(PathBuf, String),
}

/// Why a file does not read, kept as it came so that only the toast words
/// it in the GUI's language: the log takes its English.
#[derive(Debug)]
pub(super) enum Unreadable {
    /// The system's own words.
    Text(String),
    /// Why the export or the recording's header does not read.
    Reader(dmm_shared::export::ReadError),
    /// The library's error.
    Library(dmm_lib::error::Error),
}

impl From<dmm_shared::export::ReadError> for Unreadable {
    fn from(e: dmm_shared::export::ReadError) -> Self {
        Self::Reader(e)
    }
}

impl From<dmm_lib::error::Error> for Unreadable {
    fn from(e: dmm_lib::error::Error) -> Self {
        Self::Library(e)
    }
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Reader(e) => e.fmt(f),
            Self::Library(e) => e.fmt(f),
        }
    }
}

impl Unreadable {
    /// What the toast says after the file's name, in the GUI's language.
    fn text(&self, l10n: &Localizer) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Reader(e) => l10n.read_error(e),
            Self::Library(e) => l10n.error(e),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(path, why) => write!(f, "{}: {why}", path.display()),
            Self::Crashed(path, why) => write!(f, "{}: the import failed: {why}", path.display()),
        }
    }
}

impl ImportError {
    /// What the toast says, in the GUI's language.
    fn text(&self, l10n: &Localizer) -> String {
        match self {
            Self::Read(path, why) => {
                let path = path.display().to_string();
                tr!(l10n, "import-failed", path = path, reason = why.text(l10n))
            }
            Self::Crashed(path, _) => {
                let path = path.display().to_string();
                tr!(l10n, "import-failed-crashed", path = path)
            }
        }
        .into_owned()
    }
}

/// An import under way.
pub(super) enum ImportJob {
    /// The file, parsing on the loader thread.
    Loading(PathBuf, Task<Result<Loaded, ImportError>>),
    /// Going through the pipeline a chunk per frame.
    Ingesting(Ingest),
}

pub(super) struct Ingest {
    path: PathBuf,
    readings: std::vec::IntoIter<(Measurement, usize)>,
    total: usize,
    /// Readings taken so far.
    taken: usize,
    markers: std::collections::VecDeque<(usize, u32, String)>,
    view: Option<String>,
    /// The file's first reading, which the view's times count from.
    first: Option<Instant>,
    /// Readings the loader left out, past the limit it read the file with.
    left_out: usize,
    /// The alarm went off on the file's first reading, which put its limits
    /// in another unit: said with the result.
    alarm_off: Option<String>,
}

/// Parse `path` and stamp its readings from `base`.
///
/// The format is the extension's, or for a file without one the content's:
/// a replay's magic line, a JSON object, or else CSV.
pub(super) fn load(path: &Path, base: Instant, limit: usize) -> Result<Loaded, ImportError> {
    use dmm_shared::export::{ExportKind, read_csv_from, read_json_from};
    let fail = |e: Unreadable| ImportError::Read(path.to_path_buf(), e);
    let kind = ExportKind::of_file(path).map_err(|e| fail(Unreadable::Text(e.to_string())))?;
    // CSV and JSON are read through, keeping no more than the session can
    // hold; a replay is parsed whole, as `--replay` parses it.
    let open = || {
        std::fs::File::open(path)
            .map(std::io::BufReader::new)
            .map_err(|e| Unreadable::Text(e.to_string()))
    };
    let loaded = match kind {
        ExportKind::Replay => std::fs::read_to_string(path)
            .map_err(|e| Unreadable::Text(e.to_string()))
            .and_then(|text| load_replay(&text, base, limit)),
        ExportKind::Json => open().and_then(|file| {
            read_json_from(file, Some(limit))
                .map(|i| from_export(i, base))
                .map_err(Unreadable::from)
        }),
        ExportKind::Csv => open().and_then(|file| {
            read_csv_from(file, Some(limit))
                .map(|i| from_export(i, base))
                .map_err(Unreadable::from)
        }),
    };
    loaded
        .map(|mut l| {
            l.path = path.to_path_buf();
            l
        })
        .map_err(fail)
}

/// A CSV or JSON export, stamped by its wall times: each reading at `base`
/// plus how long after the first it was taken, never earlier than the one
/// before — a clock set back mid-file must not send time backwards.
fn from_export(imported: dmm_shared::export::Imported, base: Instant) -> Loaded {
    let experimental = imported.readings.iter().any(|r| r.experimental);
    let (family_slots, extra_slots) = (imported.family_slots, imported.extra_slots);
    let offsets = imported.offsets();
    let readings = imported
        .readings
        .into_iter()
        .zip(&offsets)
        .map(|(r, &offset)| {
            let extra_aux = r.extra_aux;
            let m = r.into_measurement(base.checked_add(offset).unwrap_or(base));
            (m, extra_aux)
        })
        .collect();
    Loaded {
        path: PathBuf::new(),
        device: imported.device,
        device_id: None,
        model: None,
        link: None,
        experimental,
        readings,
        markers: imported
            .markers
            .into_iter()
            .map(|m| (m.reading, m.number, m.note))
            .collect(),
        cadence: dmm_shared::export::median_spacing(&offsets),
        family_slots,
        extra_slots,
        view: imported.view,
        left_out: imported.left_out,
    }
}

/// A replay, decoded frame by frame through its family's parser on a clock
/// of its own that moves only as the playback sleeps on it — as fast as the
/// frames decode, with the recording's own timestamps — never the session's.
fn load_replay(text: &str, base: Instant, limit: usize) -> Result<Loaded, Unreadable> {
    use dmm_lib::stream::{MeasurementStream, StreamEvent};
    let replay = dmm_lib::replay::Replay::parse(text)?;
    let recorded = dmm_shared::replay::recorded_at(&replay)?;
    let clock = dmm_lib::Clock::manual();
    // A manual clock stands still until the playback sleeps on it: its now
    // is the instant the wall origin pins.
    let start = clock.now();
    let clock = clock.with_wall_origin(SystemTime::from(recorded));
    let mut dmm = replay.open(clock)?;
    let experimental = !dmm.profile().stability.is_verified();
    let family_slots = dmm.profile().max_aux_values;
    let mut readings = Vec::new();
    let mut offsets = Vec::new();
    let mut left_out = 0;
    {
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);
        loop {
            match stream.tick() {
                // Past the limit a frame is counted, not kept.
                Ok(StreamEvent::Measurement(_)) if readings.len() >= limit => left_out += 1,
                Ok(StreamEvent::Measurement(mut m)) => {
                    let offset = m.timestamp.saturating_duration_since(start);
                    m.timestamp = base.checked_add(offset).unwrap_or(base);
                    offsets.push(offset);
                    // Frames as the meter sent them: nothing appended.
                    readings.push((m, 0));
                }
                Ok(StreamEvent::Ended) => break,
                // No wake is set: nothing else acts between frames.
                Ok(StreamEvent::Woken) => {}
                // A gap in the recording: nothing to take.
                Ok(StreamEvent::Timeout { .. }) => {}
                // A frame the family refuses is one reading fewer, as it was
                // for the live session.
                Err(e) if e.kind() == dmm_lib::error::ErrorKind::Protocol => {
                    warn!("import: a frame did not decode: {e}");
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    // Each marker on the first reading at or after its offset, one per
    // reading, as a playback puts them.
    let mut markers = Vec::with_capacity(replay.markers.len());
    let mut pending: Vec<_> = replay.markers.clone();
    pending.sort_by_key(|m| m.offset);
    let mut next = 0;
    for m in pending {
        let from = offsets.partition_point(|&o| o < m.offset).max(next);
        if from < offsets.len() {
            markers.push((from, m.number, m.note));
            next = from + 1;
        }
    }
    Ok(Loaded {
        view: replay.view.clone(),
        path: PathBuf::new(),
        device: Some(replay.listing.display_name().to_string()),
        device_id: Some(replay.listing.id()),
        model: replay.model.clone(),
        link: replay.link,
        experimental,
        readings,
        markers,
        cadence: dmm_shared::export::median_spacing(&offsets),
        family_slots,
        extra_slots: 0,
        left_out,
    })
}

/// The file types Import… offers, as Export… writes them.
const IMPORT_EXTENSIONS: [&str; 3] = ["csv", "json", "replay"];

/// Where Import… and Export… open: the folder either last picked a file in,
/// else the user's Documents, else wherever the system's dialog starts. On
/// the dialog's thread: a stale network mount can block `is_dir`.
pub(super) fn dialog_folder(last: Option<&Path>) -> Option<PathBuf> {
    if let Some(last) = last.filter(|p| p.is_absolute() && p.is_dir()) {
        return Some(last.to_path_buf());
    }
    directories::UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(Path::to_path_buf))
        .filter(|dir| dir.is_dir())
}

impl App {
    /// Import… or `Ctrl+I`: pick a file in the system's open dialog, on a
    /// thread of its own as Export…'s save dialog is, answered next frame.
    pub(super) fn begin_import(&mut self, ctx: &egui::Context) {
        if self.source.import_dialog.is_some() {
            return;
        }
        let last = self.settings.last_folder.clone();
        let file_type = tr!(self.l10n, "import-file-type").into_owned();
        self.source.import_dialog = Some(Task::spawn("import-dialog", ctx, move || {
            let mut dialog = rfd::FileDialog::new().add_filter(file_type, &IMPORT_EXTENSIONS);
            if let Some(dir) = dialog_folder(last.as_deref()) {
                dialog = dialog.set_directory(dir);
            }
            dialog.pick_file()
        }));
    }

    /// The file the open dialog answered with, once it has.
    pub(super) fn poll_import_dialog(&mut self, ctx: &egui::Context) {
        let Some(answer) = self.source.import_dialog.as_ref().and_then(Task::poll) else {
            return;
        };
        self.source.import_dialog = None;
        match answer {
            Ok(Some(path)) => {
                self.remember_folder(&path);
                self.request_import(ctx, path);
            }
            Ok(None) => {}
            Err(why) => {
                error!("the open dialog failed: {why}");
                let message = tr!(self.l10n, "import-dialog-failed");
                self.toast = Some(Toast::error(message.into_owned()));
            }
        }
    }

    /// Keep the folder of a file Import… or Export… picked for the next
    /// dialog to open in.
    pub(super) fn remember_folder(&mut self, file: &Path) {
        let Some(folder) = file.parent().filter(|p| !p.as_os_str().is_empty()) else {
            return;
        };
        self.settings.last_folder = Some(folder.to_path_buf());
    }

    /// Import `path`, asking first if the session holds samples or markers
    /// no file does: the import replaces it.
    pub(super) fn request_import(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.ask_before_importing(path.clone()) {
            return;
        }
        self.import_file(ctx, path);
    }

    /// Import `path`: a fresh session, the meter disconnected, the file read
    /// on a thread of its own and taken in over the next frames.
    pub(super) fn import_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.disconnect();
        // The file stands in for the meter, so a failed connect has nothing
        // left to explain; the next Connect reports afresh.
        self.connection.last_error = None;
        self.reset_session_for_import();
        info!("importing {}", path.display());
        let base = self.clock.now();
        // No more than the recording it goes into can hold.
        let limit = self.settings.max_samples;
        let file = path.clone();
        let loader = Task::spawn("import-load", ctx, move || load(&file, base, limit));
        self.source.import_job = Some(ImportJob::Loading(path, loader));
    }

    /// Drop everything the last session holds: an import, or the first
    /// Connect after one, starts from nothing.
    pub(super) fn reset_session_for_import(&mut self) {
        self.clear_session();
        self.capture.recording.discard();
        self.capture.recording.clear_history();
        // The file's readings name no meter that answered.
        self.capture.source = None;
        self.markers.clear();
        self.source.reset();
        // A file, or the meter after one, is judged afresh.
        self.restart_alarm();
    }

    /// Take the next chunk of an import under way, or pick up the parsed
    /// file. Every frame; asks for another while readings are going in.
    pub(super) fn step_import(&mut self, ctx: &egui::Context) {
        let Some(job) = self.source.import_job.take() else {
            return;
        };
        let job = match job {
            ImportJob::Loading(path, loader) => match loader.poll() {
                None => Some(ImportJob::Loading(path, loader)),
                Some(loaded) => {
                    match loaded.unwrap_or_else(|panic| Err(ImportError::Crashed(path, panic))) {
                        Ok(loaded) => self.begin_ingest(loaded),
                        Err(why) => {
                            warn!("import failed: {why}");
                            self.toast = Some(Toast::error(why.text(&self.l10n)));
                            None
                        }
                    }
                }
            },
            ImportJob::Ingesting(ingest) => self.ingest_chunk(ingest),
        };
        // The loader asks for a frame when it is done.
        if matches!(job, Some(ImportJob::Ingesting(_))) {
            ctx.request_repaint();
        }
        self.source.import_job = job;
    }

    /// Latch what the recording is exported under and start it.
    fn begin_ingest(&mut self, loaded: Loaded) -> Option<ImportJob> {
        let Loaded {
            path,
            device,
            device_id,
            model,
            link,
            experimental,
            readings,
            markers,
            cadence,
            family_slots,
            extra_slots,
            view,
            left_out,
        } = loaded;
        if readings.is_empty() {
            let message = tr!(self.l10n, "import-empty", file = file_name(&path));
            self.toast = Some(Toast::info(message.into_owned()));
            return None;
        }
        self.capture.device_aux_slots = family_slots;
        self.capture.recording_layout = CaptureLayout {
            device: device.map(Into::into),
            device_id,
            model,
            experimental,
            link,
            aux_slots: family_slots,
            extra_slots,
        };
        let ms = u32::try_from(cadence.as_millis()).unwrap_or(u32::MAX);
        self.source.import_cadence_ms = Some(ms);
        self.set_gap_interval(self.settings.sample_interval_ms);
        let total = readings.len();
        let first = readings.first().map(|(m, _)| m.timestamp);
        self.markers
            .reserve(markers.iter().map(|&(_, n, _)| n).max().unwrap_or(0));
        self.capture.recording.toggle(self.clock.now());
        Some(ImportJob::Ingesting(Ingest {
            path,
            readings: readings.into_iter(),
            total,
            taken: 0,
            markers: markers.into(),
            view,
            first,
            left_out,
            alarm_off: None,
        }))
    }

    /// One chunk through the pipeline, markers put back as their readings
    /// arrive; the end of the file, or a full buffer, finishes the import.
    fn ingest_chunk(&mut self, mut ingest: Ingest) -> Option<ImportJob> {
        let mut full = false;
        let mut breaches = Vec::new();
        for _ in 0..CHUNK {
            let Some((m, extra_aux)) = ingest.readings.next() else {
                break;
            };
            full = self.capture.ingest_imported(&m, extra_aux, &mut self.graph);
            while let Some(&(index, number, _)) = ingest.markers.front()
                && index <= ingest.taken
            {
                let (_, _, note) = ingest.markers.pop_front().expect("peeked");
                if index == ingest.taken {
                    let reading = super::marker_list::log_line(&self.l10n, &m);
                    self.markers
                        .insert(m.timestamp, number, note, m.wall_time.into(), reading);
                }
            }
            // After the file's marker, as for a live reading, in the unit it
            // shows if it was scaled when written (`LimitUnit::of`).
            match self.check_imported(&m) {
                Ok(breach) => breaches.extend(breach),
                Err(off) => ingest.alarm_off = Some(off),
            }
            ingest.taken += 1;
            self.last_measurement = Some(m);
            if full {
                break;
            }
        }
        self.mark_breaches(breaches);
        if !full && ingest.taken < ingest.total {
            let percent = ingest.taken * 100 / ingest.total;
            let message = tr!(
                self.l10n,
                "import-progress",
                file = file_name(&ingest.path),
                percent = percent
            );
            self.toast = Some(Toast::info(message.into_owned()));
            return Some(ImportJob::Ingesting(ingest));
        }
        self.finish_import(ingest);
        None
    }

    /// Stop the recording, count it saved — it is in a file already, with
    /// the markers it came with — and show the whole file.
    fn finish_import(&mut self, ingest: Ingest) {
        let now = self.clock.now();
        if self.capture.recording.is_active() {
            self.capture.recording.toggle(now);
        }
        let marked = self.capture.recording.marked(self.markers.iter());
        let epoch = self.capture.recording.epoch();
        self.capture
            .recording
            .mark_exported(epoch, usize::MAX, Recording::marker_keys(&marked));
        let view = ingest.view.as_deref().and_then(parse_view);
        match (view, ingest.first) {
            (Some(view), Some(first)) => self.graph.apply_view_state(&view, first),
            _ => self.graph.show_all(),
        }
        let left_out = ingest.total - ingest.taken + ingest.left_out;
        let name = file_name(&ingest.path);
        info!(
            "imported {} readings from {} ({left_out} left out)",
            ingest.taken,
            ingest.path.display()
        );
        let message = if left_out == 0 {
            tr!(self.l10n, "import-done", count = ingest.taken, file = &name)
        } else {
            tr!(
                self.l10n,
                "import-done-partial",
                count = ingest.taken,
                left = left_out,
                file = &name
            )
        };
        self.toast = Some(match ingest.alarm_off.as_deref() {
            Some(off) => Toast::info(message.into_owned()).with_caution(off),
            None => Toast::info(message.into_owned()),
        });
        self.source.imported = Some(ImportedFrom {
            path: ingest.path,
            readings: ingest.taken,
        });
    }
}

/// A saved view's JSON text, or `None` — with a log line — for text this
/// version cannot read: the readings still import, under the default view.
pub(super) fn parse_view(json: &str) -> Option<crate::graph::ViewState> {
    serde_json::from_str(json)
        .inspect_err(|e| warn!("the saved view does not read and is left out: {e}"))
        .ok()
}

/// How far into a recording `view` reaches: the end of its window, or its
/// furthest cursor — what a playback has to have played before the view has
/// anything to show.
pub(super) fn view_reach(view: &crate::graph::ViewState) -> Duration {
    let window_end = view.start.map(|s| s + view.window.unwrap_or(0.0));
    let cursors = view.cursors.map_or([None, None], |c| [c.a, c.b]);
    let reach = std::iter::once(window_end)
        .chain(cursors)
        .flatten()
        .filter(|t| t.is_finite())
        .fold(0.0_f64, f64::max);
    Duration::try_from_secs_f64(reach).unwrap_or(Duration::ZERO)
}

/// The file's name, for a toast: the folder is in the hover.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::tests_support::{file, import};
    use super::*;
    use crate::app::connection_issue::ConnectionIssue;
    use crate::app::test_support::test_app;
    use dmm_lib::measurement::MeasuredValue;

    /// A library error stays whole until the toast words it, so the log
    /// line is its English whatever the GUI's language.
    #[test]
    fn a_library_error_is_worded_only_for_the_toast() {
        let e = ImportError::Read(
            PathBuf::from("x.dmmreplay"),
            Unreadable::Library(dmm_lib::error::Error::Timeout),
        );
        assert_eq!(e.to_string(), "x.dmmreplay: timeout waiting for response");
        assert_eq!(
            e.text(&crate::l10n::english()),
            "Import failed: x.dmmreplay: timeout waiting for response"
        );
        let options = dmm_shared::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        let pseudo = Localizer::new(
            &dmm_shared::l10n::EN_US,
            &crate::l10n::resources(),
            &options,
        );
        assert!(!e.text(&pseudo).contains("timeout waiting"));
    }

    /// Three readings, a dial turn from DC V to Ω on the third, a marker on
    /// the second.
    const CSV: &str = "\
# device: UT61E+
timestamp,mode,value,unit,range,flags,marker,note
2026-09-02T10:00:00+00:00,DC V,1.6109,V,2.2V,AUTO,,
2026-09-02T10:00:00.100+00:00,DC V,1.6110,V,2.2V,AUTO,3,\"load on, 2.2 ohm\"
2026-09-02T10:00:00.200+00:00,\u{3a9},80.45,k\u{3a9},220k\u{3a9},AUTO,,
";

    /// Every reading goes into the recording, a dial turn inside the file
    /// included, with its marker under its own number; the recording counts
    /// as saved, and the session names the file.
    #[test]
    fn a_csv_import_fills_a_saved_recording() {
        let mut app = test_app();
        let path = file("three.csv", CSV);
        import(&mut app, path.clone());
        assert_eq!(app.capture.recording.export_samples().len(), 3);
        assert_eq!(app.capture.recording.unexported_count(), 0);
        assert_eq!(app.capture.recording.unsaved_marker_count(&app.markers), 0);
        let marker = app.markers.iter().next().expect("the file's marker");
        assert_eq!(
            (marker.number, marker.note.as_str()),
            (3, "load on, 2.2 ohm")
        );
        assert_eq!(
            app.capture.recording_layout.device.as_deref(),
            Some("UT61E+")
        );
        assert_eq!(app.source.imported.as_ref().map(|f| f.readings), Some(3));
        assert!(!app.graph.live, "the whole file, not the newest reading");
        let _ = std::fs::remove_file(path);
    }

    /// An alarm set before the import judges the file's readings as the
    /// live ones: the crossing adds its note to the file's marker on it.
    #[test]
    fn an_import_is_judged_by_the_alarm() {
        let mut app = test_app();
        let limits = dmm_lib::alarm::Limits::check(None, Some(1.61095)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        let path = file("alarm.csv", CSV);
        import(&mut app, path.clone());
        let marker = app.markers.iter().next().expect("the file's marker");
        assert_eq!(
            (marker.number, marker.note.as_str()),
            (3, "load on, 2.2 ohm; Above high limit 1.61095 V")
        );
        assert_eq!(app.alarm.as_ref().map(|a| a.high_count), Some(1));
        let _ = std::fs::remove_file(path);
    }

    /// Written under a ×1000 kV scale: the meter's volts ride along as Raw.
    const KV_CSV: &str = "\
# device: UNI-T UT61E+
timestamp,mode,value,unit,range,flags,aux1_label,aux1_value,aux1_unit
2026-09-17T12:42:50.783+02:00,DC V,4000,kV,22V,AUTO,Raw,4.000,V
2026-09-17T12:42:50.879+02:00,DC V,5500,kV,22V,AUTO,Raw,5.500,V
";

    /// A file written under a ×1000 kV scale: its readings carry the meter's
    /// volts as Raw, so limits typed in kV mean kV, as they did live.
    #[test]
    fn a_scaled_files_readings_are_judged_in_the_unit_they_show() {
        let mut app = test_app();
        let limits = dmm_lib::alarm::Limits::check(None, Some(4500.0)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        let path = file("kv.csv", KV_CSV);
        import(&mut app, path.clone());
        assert_eq!(app.alarm_limit_unit(), dmm_lib::alarm::LimitUnit::Shown);
        let marker = app.markers.iter().next().expect("the breach's marker");
        assert_eq!(marker.note, "Above high limit 4500 kV");
        assert_eq!(app.alarm.as_ref().map(|a| a.high_count), Some(1));
        let _ = std::fs::remove_file(path);
    }

    /// An alarm bound to the last session's volts, carried into a file in
    /// kV, goes off: the same limits would mean a thousand times more. The
    /// result says so; a file in the same unit keeps the alarm on.
    #[test]
    fn an_import_in_another_unit_turns_the_alarm_off() {
        let mut app = test_app();
        let limits = dmm_lib::alarm::Limits::check(None, Some(5.0)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        let volts = Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.0),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        assert!(app.check_alarm(&volts).is_none());
        let path = file("kv-off.csv", KV_CSV);
        import(&mut app, path.clone());
        assert!(app.alarm.is_none());
        let toast = app.toast.as_ref().expect("the result");
        assert!(
            toast.message.ends_with(
                "\nReadings in kV \u{2014} alarm off; set its limits again in the new unit"
            ),
            "{}",
            toast.message
        );
        let _ = std::fs::remove_file(path);

        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        assert!(app.check_alarm(&volts).is_none());
        let path = file("volts.csv", CSV);
        import(&mut app, path.clone());
        assert!(app.alarm.is_some(), "volts again: the alarm stays on");
        let _ = std::fs::remove_file(path);
    }

    /// The file holds what was shown, scaled or not: a transform in force is
    /// not applied again. And the meter the file names is not saved as the
    /// user's.
    #[test]
    fn an_import_applies_no_transform_and_saves_no_device() {
        let mut app = test_app();
        let family = app.settings.shared.device_family.clone();
        app.transform = dmm_lib::transform::Transform::linear(2.0, 0.0, None);
        let path = file("scaled.csv", CSV);
        import(&mut app, path.clone());
        // Nor does the alarm judge the file's readings, or draw its limits,
        // as scaled.
        assert_eq!(app.alarm_limit_unit(), dmm_lib::alarm::LimitUnit::Base);
        let first = app
            .capture
            .recording
            .export_samples()
            .next()
            .expect("a sample");
        assert!(
            matches!(first.measurement.value, dmm_lib::measurement::MeasuredValue::Normal(v) if v == 1.6109)
        );
        assert_eq!(app.settings.shared.device_family, family);
        let _ = std::fs::remove_file(path);
    }

    /// A meter's readings never join a file's: the next Connect starts the
    /// session over.
    #[test]
    fn a_connect_after_an_import_starts_afresh() {
        let mut app = test_app();
        // The mock: Auto-detect would probe the machine's USB and Bluetooth
        // from a thread that outlives the test.
        app.settings.shared.device_family = "mock".to_string();
        let path = file("then-connect.csv", CSV);
        import(&mut app, path.clone());
        app.connect(&crate::app::styled_context());
        app.disconnect();
        assert!(app.source.imported.is_none());
        assert!(app.markers.is_empty());
        assert_eq!(app.capture.recording.export_samples().len(), 0);
        let _ = std::fs::remove_file(path);
    }

    /// A file imported after a failed connect stands in for the meter: the
    /// troubleshooting notice goes with it.
    #[test]
    fn an_import_drops_the_connection_notice() {
        let mut app = test_app();
        app.connection.last_error = Some(ConnectionIssue::of(
            dmm_lib::error::Error::NoTransportFound {
                cables: Vec::new(),
                bluetooth_searched: true,
            },
        ));
        assert!(app.connection_notice().is_some());
        let path = file("after-failure.csv", CSV);
        import(&mut app, path.clone());
        assert!(app.connection_notice().is_none());
        let _ = std::fs::remove_file(path);
    }

    /// A file longer than the Buffer size keeps its first readings, as a
    /// recording fills, and says what was left out.
    #[test]
    fn an_oversized_file_keeps_its_first_readings() {
        let mut app = test_app();
        app.capture.recording.set_max_samples(2);
        let path = file("oversized.csv", CSV);
        import(&mut app, path.clone());
        assert_eq!(app.capture.recording.export_samples().len(), 2);
        let toast = app
            .toast
            .as_ref()
            .map(|t| t.message.clone())
            .unwrap_or_default();
        assert!(
            toast.contains("first 2 readings") && toast.contains("1 reading past"),
            "{toast}"
        );
        let _ = std::fs::remove_file(path);
    }

    /// The loader keeps no more of a file than the Buffer size, however
    /// long the file, and counts what it left out.
    #[test]
    fn the_loader_keeps_no_more_than_the_buffer() {
        let mut app = test_app();
        app.settings.max_samples = 2;
        app.capture.recording.set_max_samples(2);
        let path = file("bounded.csv", CSV);
        import(&mut app, path.clone());
        assert_eq!(app.capture.recording.export_samples().len(), 2);
        let toast = app
            .toast
            .as_ref()
            .map(|t| t.message.clone())
            .unwrap_or_default();
        assert!(toast.contains("1 reading past"), "{toast}");
        let loaded = load(&path, Instant::now(), 2).expect("loads");
        assert_eq!((loaded.readings.len(), loaded.left_out), (2, 1));
        let _ = std::fs::remove_file(path);
    }

    /// One chunk per frame: a large file fills in over frames.
    #[test]
    fn a_large_file_goes_in_a_chunk_per_frame() {
        let mut text = String::from("timestamp,mode,value,unit,range,flags\n");
        for i in 0..CHUNK + 5 {
            text.push_str(&format!(
                "2026-09-02T10:{:02}:{:02}.{:03}+00:00,DC V,1.0,V,2.2V,\n",
                i / 60_000,
                (i / 1000) % 60,
                i % 1000
            ));
        }
        let mut app = test_app();
        let path = file("large.csv", &text);
        let ctx = crate::app::styled_context();
        app.import_file(&ctx, path.clone());
        while matches!(app.source.import_job, Some(ImportJob::Loading(..))) {
            app.step_import(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        app.step_import(&ctx);
        assert_eq!(app.capture.recording.export_samples().len(), CHUNK);
        app.step_import(&ctx);
        assert!(app.source.import_job.is_none());
        assert_eq!(app.capture.recording.export_samples().len(), CHUNK + 5);
        let _ = std::fs::remove_file(path);
    }

    /// JSON reads the same way, and a replay decodes through its family's
    /// parser with its markers on their readings.
    #[test]
    fn json_and_replay_files_import_too() {
        let mut app = test_app();
        let json = "{\"_metadata\":{\"device\":\"UT61E+\"}}\n\
            {\"timestamp\":\"2026-09-02T10:00:00+00:00\",\"mode\":\"DC V\",\"value\":1.6109,\"unit\":\"V\",\"range\":\"2.2V\",\"display_raw\":\"1.6109\",\"progress\":null,\"experimental\":false,\"flags\":{},\"marker\":1,\"note\":\"\"}\n";
        let path = file("one.json", json);
        import(&mut app, path.clone());
        assert_eq!(app.capture.recording.export_samples().len(), 1);
        assert_eq!(app.markers.iter().next().map(|m| m.number), Some(1));
        let _ = std::fs::remove_file(path);

        let replay = "# dmm-replay 1\n# device: ut61eplus\n# recorded: 2026-09-02T10:00:00Z\n\
            0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n\
            100 02 30 2D 30 2E 35 31 33 37 01 00 30 30 31\n# marker: 50 4 between\n";
        let path = file("two.replay", replay);
        import(&mut app, path.clone());
        assert_eq!(app.capture.recording.export_samples().len(), 2);
        assert_eq!(app.capture.recording_layout.device_id, Some("ut61eplus"));
        let marker = app.markers.iter().next().expect("the replay's marker");
        assert_eq!(marker.number, 4);
        let second = app
            .capture
            .recording
            .export_samples()
            .nth(1)
            .expect("two samples");
        assert_eq!(
            marker.at, second.measurement.timestamp,
            "at or after its offset"
        );
        let _ = std::fs::remove_file(path);
    }

    /// A saved view is put back once the readings are in: its window,
    /// overlays and cursors, times counted from the file's first reading.
    #[test]
    fn an_import_puts_the_saved_view_back() {
        let mut app = test_app();
        let json = "{\"_metadata\":{\"device\":\"UT61E+\",\"view\":{\"window\":0.1,\"start\":0.05,\"mean\":true,\"cursors\":{\"a\":0.1}}}}\n\
            {\"timestamp\":\"2026-09-02T10:00:00+00:00\",\"mode\":\"DC V\",\"value\":1.0,\"unit\":\"V\",\"range\":\"2.2V\",\"flags\":{}}\n\
            {\"timestamp\":\"2026-09-02T10:00:00.100+00:00\",\"mode\":\"DC V\",\"value\":2.0,\"unit\":\"V\",\"range\":\"2.2V\",\"flags\":{}}\n\
            {\"timestamp\":\"2026-09-02T10:00:00.200+00:00\",\"mode\":\"DC V\",\"value\":3.0,\"unit\":\"V\",\"range\":\"2.2V\",\"flags\":{}}\n";
        let path = file("viewed.json", json);
        import(&mut app, path.clone());
        assert!(app.graph.show_mean);
        assert!(!app.graph.live);
        assert!((app.graph.time_window_secs - 0.1).abs() < 1e-9);
        let _ = std::fs::remove_file(path);
    }

    /// A file logged every 10 s draws as a line: its spacing is what gaps
    /// are judged by, whatever the Sample interval is set to meanwhile.
    #[test]
    fn a_files_spacing_outlasts_a_sample_interval_change() {
        let mut app = test_app();
        let csv = "timestamp,mode,value,unit,range,flags\n\
            2026-09-02T10:00:00+00:00,DC V,1.0,V,2.2V,\n\
            2026-09-02T10:00:10+00:00,DC V,1.0,V,2.2V,\n\
            2026-09-02T10:00:20+00:00,DC V,1.0,V,2.2V,\n";
        let path = file("slow.csv", csv);
        import(&mut app, path.clone());
        app.settings.sample_interval_ms = 500;
        app.apply_sample_interval();
        assert_eq!(app.gap_interval_ms(500), 10_000);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_file_that_does_not_read_says_why() {
        let mut app = test_app();
        let path = file("broken.csv", "timestamp,mode\nnot a time,DC V\n");
        import(&mut app, path.clone());
        let toast = app
            .toast
            .as_ref()
            .map(|t| t.message.clone())
            .unwrap_or_default();
        assert!(toast.starts_with("Import failed:"), "{toast}");
        assert!(app.source.imported.is_none());
        let _ = std::fs::remove_file(path);
    }

    /// The remembered folder wins while it is there; a missing or relative
    /// one falls back to Documents, or to the system's own choice.
    #[test]
    fn a_dialog_opens_in_the_last_folder_else_documents() {
        let here = std::env::temp_dir();
        assert_eq!(dialog_folder(Some(&here)), Some(here.clone()));
        let documents = directories::UserDirs::new()
            .and_then(|d| d.document_dir().map(Path::to_path_buf))
            .filter(|d| d.is_dir());
        assert_eq!(dialog_folder(None), documents);
        let gone = here.join(format!("dmm-gui-gone-{}", std::process::id()));
        assert_eq!(dialog_folder(Some(&gone)), documents);
        assert_eq!(dialog_folder(Some(Path::new("."))), documents);
    }

    /// A picked file's folder is kept and saved; the same folder again
    /// asks for no save.
    #[test]
    fn a_picked_file_s_folder_is_remembered() {
        let mut app = test_app();
        app.remember_folder(Path::new("/data/bench/a.csv"));
        assert_eq!(
            app.settings.last_folder.as_deref(),
            Some(Path::new("/data/bench"))
        );
        assert!(app.settings_save_owed(), "saved");
        app.remember_folder(Path::new("/data/bench/b.json"));
        assert!(!app.settings_save_owed(), "unchanged");
        app.remember_folder(Path::new("bare.csv"));
        assert_eq!(
            app.settings.last_folder.as_deref(),
            Some(Path::new("/data/bench"))
        );
    }

    /// The open dialog's answer moves the folder, whatever the import
    /// makes of the file.
    #[test]
    fn the_open_dialog_s_answer_is_remembered() {
        let mut app = test_app();
        let dir = std::env::temp_dir().join(format!("dmm-gui-folder-{}", std::process::id()));
        let (tx, dialog) = Task::by_hand();
        tx.send(Ok(Some(dir.join("x.json"))))
            .expect("the app listens");
        app.source.import_dialog = Some(dialog);
        app.poll_import_dialog(&crate::app::styled_context());
        assert_eq!(app.settings.last_folder, Some(dir));
        app.source.import_job = None;
    }

    /// An import replaces the session: with unexported samples it asks
    /// first and waits, with nothing to lose it goes straight ahead.
    #[test]
    fn an_import_over_unexported_samples_asks_first() {
        let mut app = test_app();
        let path = file("ask.csv", CSV);

        app.request_import(&crate::app::styled_context(), path.clone());
        assert!(
            app.source.import_job.is_some(),
            "nothing to lose: imported at once"
        );
        app.source.import_job = None;

        app.capture.recording.toggle(Instant::now());
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.0),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        app.capture.recording.push(&m, 0);
        app.request_import(&crate::app::styled_context(), path.clone());
        assert!(app.source.import_job.is_none(), "waits for the answer");
        assert!(!app.recording_panel_idle(), "the prompt is up");
        let _ = std::fs::remove_file(path);
    }

    /// The file's marker numbers go with the file: a meter's session after
    /// it numbers its markers from 1 again.
    #[test]
    fn a_live_session_after_an_import_numbers_its_markers_from_one() {
        let mut app = test_app();
        app.settings.shared.device_family = "mock".to_string();
        let path = file("numbers.csv", CSV);
        import(&mut app, path.clone());
        app.connect(&crate::app::styled_context());
        app.disconnect_and_wait();
        app.deliver(crate::app::connection::DmmMessage::Connected(
            crate::app::connection::ConnectedMeter::test_fixture(Some("mock")),
        ));
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(1.0),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        m.timestamp = app.clock.now();
        app.deliver(crate::app::connection::DmmMessage::Measurement(m));
        app.add_marker(false);
        let numbers: Vec<u32> = app.markers.iter().map(|m| m.number).collect();
        assert_eq!(numbers, [1]);
        let _ = std::fs::remove_file(path);
    }

    /// Connect after an import asks first once markers were added to the
    /// imported session, and goes straight ahead when nothing changed.
    #[test]
    fn a_connect_over_new_markers_asks_first() {
        let mut app = test_app();
        let path = file("connect.csv", CSV);
        import(&mut app, path.clone());
        app.add_marker(false);
        app.request_connect(&crate::app::styled_context());
        assert!(!app.recording_panel_idle(), "the prompt is up");
        assert!(
            app.source.imported.is_some(),
            "the session waits for the answer"
        );
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
pub(super) mod tests_support {
    use super::App;
    use std::path::PathBuf;
    use std::time::Duration;

    /// `text` as a file named `name` of this test's own.
    pub(in crate::app) fn file(name: &str, text: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("dmm-gui-import-{}-{name}", std::process::id()));
        std::fs::write(&path, text).expect("write the test file");
        path
    }

    /// Import `path` and run frames until it is in.
    pub(in crate::app) fn import(app: &mut App, path: PathBuf) {
        let ctx = crate::app::styled_context();
        app.import_file(&ctx, path);
        for _ in 0..10_000 {
            app.step_import(&ctx);
            if app.source.import_job.is_none() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("the import never finished");
    }
}
