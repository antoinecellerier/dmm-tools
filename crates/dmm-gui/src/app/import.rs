//! Import…: an exported CSV, JSON or replay file read back into a session of
//! its own, with its markers.
//!
//! The file is parsed on a thread of its own and its readings stamped there;
//! the frame loop then takes them through the pipeline a chunk at a time, so
//! a half-million-reading file neither freezes the window nor arrives as one
//! enormous frame. They go into a recording, stopped and marked saved: a
//! mode or unit change inside the file restarts the graph's trace as a live
//! one would, and the recording keeps every reading for Export… regardless.
//!
//! No meter is attached, so nothing new arrives; the readings, markers and
//! view can still be worked on and exported again.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use dmm_lib::measurement::Measurement;
use dmm_lib::transport::Link;
use eframe::egui;
use log::{info, warn};

use super::App;
use super::capture::CaptureLayout;
use super::toast::Toast;
use crate::recording::Recording;

/// Readings taken through the pipeline per frame: a fraction of a frame's
/// time at release speed, so a large file fills in over a second or two with
/// the window still answering.
const CHUNK: usize = 20_000;

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
    link: Option<Link>,
    experimental: bool,
    /// Stamped `base + offset`, in file order, time never going back.
    readings: Vec<Measurement>,
    /// `(reading index, number, note)`, in reading order.
    markers: Vec<(usize, u32, String)>,
    /// How far apart the file's readings are, for the graph's gaps.
    cadence: Duration,
    /// The most sub-values any reading carries: the CSV columns to keep.
    aux_slots: usize,
    /// The graph view the file was saved with, as its JSON text.
    view: Option<String>,
}

/// An import under way.
pub(super) enum ImportJob {
    /// Parsing on the loader thread.
    Loading(mpsc::Receiver<Result<Loaded, String>>),
    /// Going through the pipeline a chunk per frame.
    Ingesting(Ingest),
}

pub(super) struct Ingest {
    path: PathBuf,
    readings: std::vec::IntoIter<Measurement>,
    total: usize,
    /// Readings taken so far.
    taken: usize,
    markers: std::collections::VecDeque<(usize, u32, String)>,
    view: Option<String>,
    /// The file's first reading, which the view's times count from.
    first: Option<Instant>,
}

/// Parse `path` and stamp its readings from `base`.
///
/// The format is the extension's, or for a file without one the content's:
/// a replay's magic line, a JSON object, or else CSV.
pub(super) fn load(path: &Path, base: Instant) -> Result<Loaded, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let head = text.trim_start();
    let is_replay = extension.as_deref() == Some("replay")
        || (extension.is_none() && head.starts_with("# dmm-replay"));
    let is_json =
        extension.as_deref() == Some("json") || (extension.is_none() && head.starts_with('{'));
    let loaded = if is_replay {
        load_replay(&text, base)
    } else {
        let imported = if is_json {
            dmm_shared::export::read_json(&text)
        } else {
            dmm_shared::export::read_csv(&text)
        }?;
        Ok(from_export(imported, base))
    };
    loaded
        .map(|mut l| {
            l.path = path.to_path_buf();
            l
        })
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// A CSV or JSON export, stamped by its wall times: each reading at `base`
/// plus how long after the first it was taken, never earlier than the one
/// before — a clock set back mid-file must not send time backwards.
fn from_export(imported: dmm_shared::export::Imported, base: Instant) -> Loaded {
    let first = imported.readings.first().map(|r| r.wall_time);
    let experimental = imported.readings.iter().any(|r| r.experimental);
    let aux_slots = imported
        .readings
        .iter()
        .map(|r| r.aux.len())
        .max()
        .unwrap_or(0);
    let mut last = Duration::ZERO;
    let mut offsets = Vec::with_capacity(imported.readings.len());
    let readings = imported
        .readings
        .into_iter()
        .map(|r| {
            let offset = first
                .and_then(|first| r.wall_time.duration_since(first).ok())
                .unwrap_or(Duration::ZERO)
                .max(last);
            last = offset;
            offsets.push(offset);
            r.into_measurement(base.checked_add(offset).unwrap_or(base))
        })
        .collect();
    Loaded {
        path: PathBuf::new(),
        device: imported.device,
        device_id: None,
        link: None,
        experimental,
        readings,
        markers: imported
            .markers
            .into_iter()
            .map(|m| (m.reading, m.number, m.note))
            .collect(),
        cadence: median_spacing(&offsets),
        aux_slots,
        view: imported.view,
    }
}

/// A replay, decoded frame by frame through its family's parser on a clock
/// of its own that moves only as the playback sleeps on it — as fast as the
/// frames decode, with the recording's own timestamps — never the session's.
fn load_replay(text: &str, base: Instant) -> Result<Loaded, String> {
    use dmm_lib::stream::{MeasurementStream, StreamEvent};
    let replay = dmm_lib::replay::Replay::parse(text).map_err(|e| e.to_string())?;
    let recorded = chrono::DateTime::parse_from_rfc3339(&replay.recorded).map_err(|e| {
        format!(
            "`# recorded: {}` is not an RFC 3339 date: {e}",
            replay.recorded
        )
    })?;
    let clock = dmm_lib::Clock::manual().with_wall_origin(SystemTime::from(recorded));
    let (start, _) = clock
        .wall_origin()
        .ok_or("the replay clock has no origin")?;
    let mut dmm = replay.open(clock).map_err(|e| e.to_string())?;
    let experimental = !dmm.profile().stability.is_verified();
    let aux_slots = dmm.profile().max_aux_values;
    let mut readings = Vec::new();
    let mut offsets = Vec::new();
    {
        let mut stream = MeasurementStream::new(&mut dmm, Duration::ZERO);
        loop {
            match stream.tick() {
                Ok(StreamEvent::Measurement(mut m)) => {
                    let offset = m.timestamp.saturating_duration_since(start);
                    m.timestamp = base.checked_add(offset).unwrap_or(base);
                    offsets.push(offset);
                    readings.push(m);
                }
                Ok(StreamEvent::Ended) => break,
                // A gap in the recording: nothing to take.
                Ok(StreamEvent::Timeout { .. }) => {}
                // A frame the family refuses is one reading fewer, as it was
                // for the live session.
                Err(e) if e.kind() == dmm_lib::error::ErrorKind::Protocol => {
                    warn!("import: a frame did not decode: {e}");
                }
                Err(e) => return Err(e.to_string()),
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
        device: Some(replay.device.display_name.to_string()),
        device_id: Some(replay.device.id),
        link: replay.link,
        experimental,
        readings,
        markers,
        cadence: median_spacing(&offsets),
        aux_slots,
    })
}

/// The median gap between consecutive offsets, a second when there is none
/// to measure: the spacing a file's readings came at, which a single slow
/// stretch doesn't move.
fn median_spacing(offsets: &[Duration]) -> Duration {
    let mut gaps: Vec<Duration> = offsets
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .filter(|g| !g.is_zero())
        .collect();
    if gaps.is_empty() {
        return Duration::from_secs(1);
    }
    let mid = gaps.len() / 2;
    *gaps.select_nth_unstable(mid).1
}

/// The file types Import… offers, as Export… writes them.
const IMPORT_EXTENSIONS: [&str; 3] = ["csv", "json", "replay"];

impl App {
    /// Import… or `Ctrl+I`: pick a file in the system's open dialog, on a
    /// thread of its own as Export…'s save dialog is, answered next frame.
    pub(super) fn begin_import(&mut self) {
        if self.import_dialog.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let picked = std::panic::catch_unwind(|| {
                rfd::FileDialog::new()
                    .add_filter("Exported readings", &IMPORT_EXTENSIONS)
                    .pick_file()
            })
            .unwrap_or_else(|_| {
                warn!("the open dialog failed");
                None
            });
            let _ = tx.send(picked);
        });
        self.import_dialog = Some(rx);
    }

    /// The file the open dialog answered with, once it has.
    pub(super) fn poll_import_dialog(&mut self) {
        let Some(rx) = &self.import_dialog else {
            return;
        };
        match rx.try_recv() {
            Ok(picked) => {
                self.import_dialog = None;
                if let Some(path) = picked {
                    self.request_import(path);
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => self.import_dialog = None,
        }
    }

    /// Import `path`, asking first if the session holds samples or markers
    /// no file does: the import replaces it.
    pub(super) fn request_import(&mut self, path: PathBuf) {
        if self.ask_before_importing(path.clone()) {
            return;
        }
        self.import_file(path);
    }

    /// Import `path`: a fresh session, the meter disconnected, the file read
    /// on a thread of its own and taken in over the next frames.
    pub(super) fn import_file(&mut self, path: PathBuf) {
        self.disconnect();
        self.reset_session_for_import();
        info!("importing {}", path.display());
        let base = self.clock.now();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let loaded = std::panic::catch_unwind(|| load(&path, base))
                .unwrap_or_else(|_| Err(format!("{}: the import failed", path.display())));
            let _ = tx.send(loaded);
        });
        self.import_job = Some(ImportJob::Loading(rx));
    }

    /// Drop everything the last session holds: an import, or the first
    /// Connect after one, starts from nothing.
    pub(super) fn reset_session_for_import(&mut self) {
        self.clear_session();
        self.capture.recording.discard();
        self.capture.recording.clear_history();
        self.markers.clear();
        self.replay_markers.clear();
        self.import_cadence_ms = None;
        self.replay_view = None;
        // A `--replay` session's markers and view go back in with its next
        // Connect.
        self.requeue_replay = self.replay.is_some();
        self.imported = None;
        self.import_job = None;
    }

    /// Take the next chunk of an import under way, or pick up the parsed
    /// file. Every frame; asks for another while there is more.
    pub(super) fn step_import(&mut self, ctx: &egui::Context) {
        let Some(job) = self.import_job.take() else {
            return;
        };
        let job = match job {
            ImportJob::Loading(rx) => match rx.try_recv() {
                Ok(Ok(loaded)) => self.begin_ingest(loaded),
                Ok(Err(message)) => {
                    warn!("import failed: {message}");
                    self.toast = Some(Toast::error(format!("Import failed: {message}")));
                    None
                }
                Err(mpsc::TryRecvError::Empty) => Some(ImportJob::Loading(rx)),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.toast = Some(Toast::error("Import failed".to_string()));
                    None
                }
            },
            ImportJob::Ingesting(ingest) => self.ingest_chunk(ingest),
        };
        if job.is_some() {
            ctx.request_repaint();
        }
        self.import_job = job;
    }

    /// Latch what the recording is exported under and start it.
    fn begin_ingest(&mut self, loaded: Loaded) -> Option<ImportJob> {
        let Loaded {
            path,
            device,
            device_id,
            link,
            experimental,
            readings,
            markers,
            cadence,
            aux_slots,
            view,
        } = loaded;
        if readings.is_empty() {
            self.toast = Some(Toast::info(format!(
                "{} holds no readings",
                file_name(&path)
            )));
            return None;
        }
        self.capture.device_aux_slots = aux_slots;
        self.capture.recording_layout = CaptureLayout {
            device: device.map(Into::into),
            device_id,
            experimental: Some(experimental),
            link,
            aux_slots,
            extra_slots: 0,
        };
        let ms = u32::try_from(cadence.as_millis()).unwrap_or(u32::MAX);
        self.import_cadence_ms = Some(ms);
        self.set_gap_interval(self.settings.sample_interval_ms);
        let total = readings.len();
        let first = readings.first().map(|m| m.timestamp);
        self.capture.recording.toggle(self.clock.now());
        Some(ImportJob::Ingesting(Ingest {
            path,
            readings: readings.into_iter(),
            total,
            taken: 0,
            markers: markers.into(),
            view,
            first,
        }))
    }

    /// One chunk through the pipeline, markers put back as their readings
    /// arrive; the end of the file, or a full buffer, finishes the import.
    fn ingest_chunk(&mut self, mut ingest: Ingest) -> Option<ImportJob> {
        let mut full = false;
        for _ in 0..CHUNK {
            let Some(m) = ingest.readings.next() else {
                break;
            };
            full = self.capture.ingest_imported(&m, &mut self.graph);
            while let Some(&(index, number, _)) = ingest.markers.front()
                && index <= ingest.taken
            {
                let (_, _, note) = ingest.markers.pop_front().expect("peeked");
                if index == ingest.taken {
                    let reading = super::marker_list::log_line(&m);
                    self.markers
                        .insert(m.timestamp, number, note, m.wall_time.into(), reading);
                }
            }
            ingest.taken += 1;
            self.last_measurement = Some(m);
            if full {
                break;
            }
        }
        if !full && ingest.taken < ingest.total {
            let percent = ingest.taken * 100 / ingest.total;
            self.toast = Some(Toast::info(format!(
                "Importing {}… {percent}%",
                file_name(&ingest.path)
            )));
            return Some(ImportJob::Ingesting(ingest));
        }
        self.finish_import(ingest);
        None
    }

    /// Stop the recording, count it saved — it is in a file already, with
    /// the markers it came with — and show the whole file.
    fn finish_import(&mut self, ingest: Ingest) {
        let now = self.clock.now();
        if self.capture.recording.active {
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
        let left_out = ingest.total - ingest.taken;
        let name = file_name(&ingest.path);
        info!(
            "imported {} readings from {} ({left_out} left out)",
            ingest.taken,
            ingest.path.display()
        );
        self.toast = Some(Toast::info(if left_out == 0 {
            format!("Imported {} from {name}", readings_noun(ingest.taken))
        } else {
            format!(
                "Imported the first {} from {name}; {} past the Buffer size in Settings were left out",
                readings_noun(ingest.taken),
                readings_noun(left_out)
            )
        }));
        self.imported = Some(ImportedFrom {
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

/// `n readings`, singular for one.
pub(super) fn readings_noun(n: usize) -> String {
    if n == 1 {
        "1 reading".to_string()
    } else {
        format!("{n} readings")
    }
}

/// The file's name, for a toast: the folder is in the hover.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    /// Three readings, a dial turn from DC V to Ω on the third, a marker on
    /// the second.
    const CSV: &str = "\
# device: UT61E+
timestamp,mode,value,unit,range,flags,marker,note
2026-09-02T10:00:00+00:00,DC V,1.6109,V,2.2V,AUTO,,
2026-09-02T10:00:00.100+00:00,DC V,1.6110,V,2.2V,AUTO,3,\"load on, 2.2 ohm\"
2026-09-02T10:00:00.200+00:00,\u{3a9},80.45,k\u{3a9},220k\u{3a9},AUTO,,
";

    fn app() -> App {
        App::from_settings(Settings::default(), dmm_lib::Clock::real())
    }

    /// `text` as a file named `name` of this test's own.
    fn file(name: &str, text: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("dmm-gui-import-{}-{name}", std::process::id()));
        std::fs::write(&path, text).expect("write the test file");
        path
    }

    /// Import `path` and run frames until it is in.
    fn import(app: &mut App, path: PathBuf) {
        let ctx = egui::Context::default();
        app.import_file(path);
        for _ in 0..10_000 {
            app.step_import(&ctx);
            if app.import_job.is_none() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("the import never finished");
    }

    /// Every reading goes into the recording, a dial turn inside the file
    /// included, with its marker under its own number; the recording counts
    /// as saved, and the session names the file.
    #[test]
    fn a_csv_import_fills_a_saved_recording() {
        let mut app = app();
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
        assert_eq!(app.imported.as_ref().map(|f| f.readings), Some(3));
        assert!(!app.graph.live, "the whole file, not the newest reading");
        let _ = std::fs::remove_file(path);
    }

    /// The file holds what was shown, scaled or not: a transform in force is
    /// not applied again. And the meter the file names is not saved as the
    /// user's.
    #[test]
    fn an_import_applies_no_transform_and_saves_no_device() {
        let mut app = app();
        let family = app.settings.shared.device_family.clone();
        app.transform = dmm_lib::transform::Transform::linear(2.0, 0.0, None);
        let path = file("scaled.csv", CSV);
        import(&mut app, path.clone());
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
        let mut app = app();
        let path = file("then-connect.csv", CSV);
        import(&mut app, path.clone());
        app.connect(&egui::Context::default());
        app.disconnect();
        assert!(app.imported.is_none());
        assert!(app.markers.is_empty());
        assert_eq!(app.capture.recording.export_samples().len(), 0);
        let _ = std::fs::remove_file(path);
    }

    /// A file longer than the Buffer size keeps its first readings, as a
    /// recording fills, and says what was left out.
    #[test]
    fn an_oversized_file_keeps_its_first_readings() {
        let mut app = app();
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
        let mut app = app();
        let path = file("large.csv", &text);
        let ctx = egui::Context::default();
        app.import_file(path.clone());
        while matches!(app.import_job, Some(ImportJob::Loading(_))) {
            app.step_import(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        app.step_import(&ctx);
        assert_eq!(app.capture.recording.export_samples().len(), CHUNK);
        app.step_import(&ctx);
        assert!(app.import_job.is_none());
        assert_eq!(app.capture.recording.export_samples().len(), CHUNK + 5);
        let _ = std::fs::remove_file(path);
    }

    /// JSON reads the same way, and a replay decodes through its family's
    /// parser with its markers on their readings.
    #[test]
    fn json_and_replay_files_import_too() {
        let mut app = app();
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
        let mut app = app();
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
        let mut app = app();
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
        let mut app = app();
        let path = file("broken.csv", "timestamp,mode\nnot a time,DC V\n");
        import(&mut app, path.clone());
        let toast = app
            .toast
            .as_ref()
            .map(|t| t.message.clone())
            .unwrap_or_default();
        assert!(toast.starts_with("Import failed:"), "{toast}");
        assert!(app.imported.is_none());
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod prompt_tests {
    use super::*;
    use crate::settings::Settings;
    use dmm_lib::measurement::MeasuredValue;

    /// An import replaces the session: with unexported samples it asks
    /// first and waits, with nothing to lose it goes straight ahead.
    #[test]
    fn an_import_over_unexported_samples_asks_first() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let path =
            std::env::temp_dir().join(format!("dmm-gui-import-{}-ask.csv", std::process::id()));
        std::fs::write(&path, super::tests_support::CSV).expect("write");

        app.request_import(path.clone());
        assert!(
            app.import_job.is_some(),
            "nothing to lose: imported at once"
        );
        app.import_job = None;

        app.capture.recording.toggle(Instant::now());
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.0),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        app.capture.recording.push(&m, 0);
        app.request_import(path.clone());
        assert!(app.import_job.is_none(), "waits for the answer");
        assert!(!app.recording_panel_idle(), "the prompt is up");
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod connect_tests {
    use super::*;
    use crate::settings::Settings;

    /// Connect after an import asks first once markers were added to the
    /// imported session, and goes straight ahead when nothing changed.
    #[test]
    fn a_connect_over_new_markers_asks_first() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let path =
            std::env::temp_dir().join(format!("dmm-gui-import-{}-connect.csv", std::process::id()));
        std::fs::write(&path, super::tests_support::CSV).expect("write");
        let ctx = egui::Context::default();
        app.import_file(path.clone());
        while app.import_job.is_some() {
            app.step_import(&ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
        app.add_marker(false);
        app.request_connect(&ctx);
        assert!(!app.recording_panel_idle(), "the prompt is up");
        assert!(app.imported.is_some(), "the session waits for the answer");
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests_support {
    /// A small CSV export, for tests outside `tests`.
    pub(super) const CSV: &str = "\
timestamp,mode,value,unit,range,flags
2026-09-02T10:00:00+00:00,DC V,1.6109,V,2.2V,AUTO
";
}
