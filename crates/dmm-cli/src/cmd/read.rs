//! `read`: a meter's readings, live or played back from a replay file, as
//! text, CSV, JSON or the meter's own frames, with the closing summary.

use super::{called_off, setup_ctrlc};
use crate::cli::{ClockScale, ReadArgs, TransformArgs};
use crate::format::{self, OutputFormat};
use crate::l10n::{note_label, warning_label};
use crate::open::{
    open_mock_device, open_with_help, opened_device, pinned_mock_mode, print_no_response_help,
    requires_hardware, selection_id,
};
use crate::output;
use console::style;
use dmm_lib::alarm::{Alarm, Hysteresis, Limits};
use dmm_lib::clock::ClockFlagError;
use dmm_lib::error::ErrorKind;
use dmm_lib::protocol::registry::{SelectableDevice, Selection};
use dmm_lib::stats::SeriesChange;
use dmm_lib::stream::{MeasurementStream, NO_RESPONSE_TIMEOUTS, StreamEvent};
use dmm_lib::text::TextKind;
use dmm_lib::transform::Transform;
use dmm_shared::l10n::Localizer;
use dmm_shared::reopen::{Attempt, ReopenPin, SeveralCables, listed_cables, reopen_pin};
use dmm_shared::tr;
use log::info;
use std::borrow::Cow;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A `read` run with its flags settled and checked against each other, so
/// that every refusal comes before anything opens or prints.
pub(crate) struct ReadPlan {
    selection: Selection,
    source: Source,
    interval_ms: u64,
    format: OutputFormat,
    // What the format and the `-o` file make of each other, printed once
    // the run is sure to start.
    note: Option<String>,
    // Virtual session time; real unless a --mock-clock-* flag asked otherwise.
    clock: dmm_lib::Clock,
    options: RunOptions,
}

/// Where a `read` run's readings come from.
enum Source {
    /// The meter the selection names or detection finds, or the mock.
    Meter(MeterSource),
    /// A recording played back: `--replay`, or a replay file `--import`ed.
    Replay(PathBuf),
    /// A CSV or JSON export read back.
    Import(PathBuf),
}

/// What a meter run takes that no other source does.
struct MeterSource {
    /// Reopen a link that drops; the mock has no link to lose.
    reconnect: bool,
    /// As typed: a run on the mock reads the settings file's when it opens.
    mock_mode: Option<dmm_lib::mock::MockMode>,
}

/// The `read` options every source honours: what each reading goes
/// through and where it is written.
struct RunOptions {
    destination: output::Destination,
    count: usize,
    integrate: bool,
    // Applied to every reading before anything else sees it; the identity
    // transform (no --scale/--offset/--unit) is a no-op.
    transform: Transform,
    alarm: Option<AlarmOptions>,
}

impl ReadPlan {
    /// Settle `args` for a run on `selection`, or say why they are refused.
    /// `device_named` is whether `--device` was typed, as opposed to coming
    /// from the settings file or detection.
    pub(crate) fn new(
        l: &Localizer,
        args: ReadArgs,
        selection: Selection,
        device_named: bool,
    ) -> Result<Self, Refused> {
        let ReadArgs {
            interval_ms,
            format,
            output,
            count,
            no_reconnect,
            integrate,
            transform,
            alarm,
            mock_mode,
            replay,
            import,
            mock_clock_scale,
            mock_clock_preseed,
        } = args;
        let alarm = alarm
            .limits(l)
            .map_err(Refused::Alarm)?
            .map(|(limits, hysteresis)| AlarmOptions {
                limits,
                hysteresis,
                bell: alarm.alarm_bell,
            });
        // A replay or an export names its own meter, so a `--device`
        // alongside it would either be ignored or contradict the file. Only
        // a typed flag is refused: the settings file names a meter for every
        // run, and a file must not need it edited. On the flags as typed,
        // before an imported replay file is folded into a replay.
        if device_named {
            if replay.is_some() {
                return Err(Refused::ReplayNamesItsDevice);
            }
            if import.is_some() {
                return Err(Refused::ImportNamesItsDevice);
            }
        }
        // An import names its own meter, as a replay does; a replay file
        // imported is a replay played without waiting.
        // Detected as the GUI detects it: an extensionless replay file is
        // still a replay.
        let import_replay = import.as_ref().is_some_and(|p| {
            dmm_shared::export::ExportKind::of_file(p)
                .is_ok_and(|kind| kind == dmm_shared::export::ExportKind::Replay)
        });
        let (replay, import, mock_clock_scale) = match import {
            Some(path) if import_replay => (Some(path), None, Some(ClockScale::Max)),
            other => (replay, other, mock_clock_scale),
        };
        let clock = session_clock(l, mock_clock_scale, mock_clock_preseed, replay.is_some())?;
        // Settled before anything opens, so a run that cannot write what it
        // was asked for fails with no meter attached.
        let (format, destination, note) = resolve_output(l, &format, output);
        let source = match (import, replay) {
            (Some(path), _) => {
                if let Some(refused) = refuse_for_import(format, interval_ms) {
                    return Err(refused);
                }
                Source::Import(path)
            }
            (None, replay) => {
                if let Some(refused) =
                    refuse_replay_format(format, selection, replay.is_some(), &transform, integrate)
                {
                    return Err(refused);
                }
                match replay {
                    // No cable to pace this run, so the clock flags apply as
                    // they do to the mock.
                    Some(path) => Source::Replay(path),
                    // `main` has refused the clock flags for a device on a
                    // cable.
                    None => Source::Meter(MeterSource {
                        reconnect: !no_reconnect,
                        mock_mode,
                    }),
                }
            }
        };
        Ok(Self {
            selection,
            source,
            interval_ms,
            format,
            note,
            clock,
            options: RunOptions {
                destination,
                count,
                integrate,
                transform: transform.to_transform(),
                alarm,
            },
        })
    }

    /// Run it: the note on where the output goes, then the meter, the
    /// recording or the export read. `settings` gives the mock its saved
    /// mode.
    pub(crate) fn run(
        self,
        opts: dmm_lib::OpenOptions<'_>,
        settings: Option<&dmm_shared::SharedSettings>,
    ) -> Result<(), crate::l10n::Failure> {
        let Self {
            selection,
            source,
            interval_ms,
            format,
            note,
            clock,
            options,
        } = self;
        if let Some(note) = note {
            eprintln!("{} {note}", note_label());
        }
        match source {
            Source::Import(path) => read_import(&path, format, options),
            Source::Replay(path) => read_replay(&path, interval_ms, format, options, clock),
            Source::Meter(meter) => {
                let meter = MeterSource {
                    mock_mode: pinned_mock_mode(selection, meter.mock_mode, settings),
                    ..meter
                };
                read_meter(selection, opts, interval_ms, format, options, meter, clock)
            }
        }
    }
}

/// Read the meter `selection` names, on its cable or simulated; `meter`'s
/// mock mode is pinned already.
fn read_meter(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    interval_ms: u64,
    format: OutputFormat,
    options: RunOptions,
    meter: MeterSource,
    clock: dmm_lib::Clock,
) -> Result<(), crate::l10n::Failure> {
    let MeterSource {
        reconnect,
        mock_mode,
    } = meter;
    let markers = options.alarm.is_some();
    let stop = setup_ctrlc()?;
    if requires_hardware(selection) {
        let opts = dmm_lib::OpenOptions {
            stop: Some(&stop),
            ..opts
        };
        let (mut dmm, device) = match open_with_help(selection, opts) {
            Err(e) if called_off(&e) => return Ok(()),
            opened => opened?,
        };
        // Once, before the loop: on a UT61+ asking the meter its name is a
        // command it answers with a beep, so only a replay file — whose
        // header carries the name — pays for it, and not when detection has.
        let model = (format == OutputFormat::Replay)
            .then(|| dmm.get_name().ok().flatten())
            .flatten();
        // The link these readings come over, so a session played back from
        // the file says what it was recorded on rather than nothing.
        let link = dmm.transport().link();
        let out = read_output(
            format,
            &dmm,
            &options.transform,
            options.integrate,
            markers,
            || format::ReplayHeader {
                device: crate::open::meter_listing(device).id().to_string(),
                model,
                link,
            },
        );
        // Reopened as the meter first opened, even under Auto-detect: the
        // CSV layout and the file's name are that meter's. `reopen_pin` says
        // on which link; chosen here, with this meter's cable still listed.
        let pin = reconnect.then(|| {
            reopen_pin(
                Some(device),
                opts.adapter,
                opts.bluetooth,
                link,
                dmm.transport().bluetooth_selector(),
                dmm.transport().cable_serial(),
                listed_cables,
            )
        });
        let reopen = match pin {
            Some(Ok(pin)) => Some(Reopen {
                pin,
                listed: Box::new(listed_cables),
                open: Box::new({
                    let stop = Arc::clone(&stop);
                    move |pin| dmm_lib::open_device(device, pin.options(Some(&stop)))
                }),
            }),
            // The open said how many cables there are; what is left to say
            // is that a dropped link won't come back on this one.
            Some(Err(_)) => {
                let l = crate::l10n::get();
                eprintln!("{} {}", warning_label(), l.help().cables_no_reconnect());
                None
            }
            None => None,
        };
        // After the open's own lines: the replay's name ask may have heard it.
        crate::open::warn_other_model(&dmm, device);
        info!("connected, starting measurement loop");
        run_read_loop(
            stop,
            dmm,
            interval_ms,
            options,
            out,
            crate::open::meter_name(device),
            Some(Meter {
                device,
                reopen,
                interval: dmm_shared::RECONNECT_INTERVAL,
            }),
            Vec::new(),
        )
    } else {
        let dmm = open_mock_device(selection, mock_mode, clock)?;
        info!("mock device connected, starting measurement loop");
        let interval_ms =
            dmm_lib::mock::simulated_tick(dmm.delivery(), Duration::from_millis(interval_ms))
                .as_millis() as u64;
        // `--format replay` is refused for a device that synthesises its
        // readings, so the header below is never built.
        let out = read_output(
            format,
            &dmm,
            &options.transform,
            options.integrate,
            markers,
            || format::ReplayHeader {
                device: selection_id(selection).to_string(),
                model: None,
                link: None,
            },
        );
        // Not hardware, so the selection names a registry entry — `auto` is a
        // cable to open and never lands here.
        let meter_name = opened_device(selection)
            .map_or_else(|| selection_id(selection), crate::open::meter_name);
        // No timeout to warn about, and no link to lose: the mock always
        // answers.
        run_read_loop(
            stop,
            dmm,
            interval_ms,
            options,
            out,
            meter_name,
            None,
            Vec::new(),
        )
    }
}

/// The output a `read` run writes, sized to the meter that is about to
/// answer: the CSV layout comes from its profile, and JSON flags a protocol
/// no report has confirmed.
fn read_output<T: dmm_lib::transport::Transport>(
    format: OutputFormat,
    dmm: &dmm_lib::Dmm<T>,
    transform: &Transform,
    integrate: bool,
    // Whether the run writes markers: a recording saved with some.
    markers: bool,
    replay_header: impl FnOnce() -> format::ReplayHeader,
) -> format::Output {
    // Fixed for the whole run: the CSV column layout is per meter family, so
    // a mode that reports fewer sub-values than the family can leaves its own
    // slots empty rather than shortening the row. A software transform adds
    // one more group, kept trailing, for the meter's own reading — so `Raw`
    // stays in the same columns whether or not the meter sent sub-values of
    // its own that frame.
    let layout = dmm_shared::export::CsvLayout {
        family_slots: dmm.profile().max_aux_values,
        extra_slots: transform.extra_aux_count(),
        integral: integrate,
        markers,
    };
    let experimental = !dmm.profile().stability.is_verified();
    format::Output::new(format, layout, experimental, replay_header)
}

/// The format a run writes and the note it earns: `--format` when given, else
/// what the `-o` file's extension names, else text.
///
/// The flag wins over the extension — the file is what the user asked for by
/// name — so a `.json` file holding CSV is a note rather than a refusal.
fn resolve_format(
    l: &Localizer,
    asked: Option<OutputFormat>,
    path: Option<&str>,
) -> (OutputFormat, Option<String>) {
    let named = path
        .and_then(|p| Path::new(p).extension()?.to_str())
        .and_then(OutputFormat::from_extension);
    match (asked, named, path) {
        (Some(asked), Some(named), Some(path)) if asked != named => (
            asked,
            Some(
                tr!(
                    l,
                    "read-format-mismatch",
                    format = asked.name(),
                    path = path
                )
                .into_owned(),
            ),
        ),
        (Some(asked), _, _) => (asked, None),
        (None, Some(named), _) => (named, None),
        (None, None, _) => (OutputFormat::Text, None),
    }
}

/// What `read` writes, where it goes, and the note the pair earns — printed
/// by the caller, which knows whether the run is going to happen at all.
fn resolve_output(
    l: &Localizer,
    asked: &Option<OutputFormat>,
    output: Option<Option<String>>,
) -> (OutputFormat, output::Destination, Option<String>) {
    let (format, note) = resolve_format(l, *asked, output.as_ref().and_then(|o| o.as_deref()));
    let destination = match output {
        None => output::Destination::Stdout,
        // A bare `-o`: the first reading names the file.
        Some(None) => output::Destination::Auto {
            extension: format.extension(),
        },
        Some(Some(path)) => output::Destination::Path(path.into()),
    };
    (format, destination, note)
}

/// Why `--format replay` is refused alongside the flags that re-express or
/// accumulate the reading, and on a device that synthesises its readings.
///
/// A replay file holds the meter's own frames, so what to make of them is a
/// choice for the run that plays them back.
fn refuse_replay_format(
    format: OutputFormat,
    selection: Selection,
    // A run already playing a recording has the frames to copy, whatever the
    // settings file names as the meter.
    replaying: bool,
    transform: &TransformArgs,
    integrate: bool,
) -> Option<Refused> {
    if format != OutputFormat::Replay {
        return None;
    }
    if !replaying && !requires_hardware(selection) {
        return Some(Refused::ReplayNeedsMeter(selection_id(selection)));
    }
    let flag = transform
        .flag_given()
        .or(integrate.then_some("--integrate"))?;
    Some(Refused::ReplayKeepsFrames(flag))
}

/// Why `read` refused its flags before opening anything: the one `Error:`
/// line `main` prints ([`crate::l10n::Localized`]). `Alarm` and `Clock`
/// carry text already in the run's language, so their `Display` is that
/// language too; nothing logs a refusal.
#[derive(Debug)]
pub(crate) enum Refused {
    /// The `--alarm-*` flags, as they word it.
    Alarm(String),
    /// A `--mock-clock-*` flag as typed, and why its value can't be used,
    /// worded.
    Clock(&'static str, String),
    /// `--device` alongside a `--replay`.
    ReplayNamesItsDevice,
    /// `--device` alongside an `--import`.
    ImportNamesItsDevice,
    /// `--format replay` on a device that synthesises its readings; its id.
    ReplayNeedsMeter(&'static str),
    /// `--format replay` with a flag that changes the reading; that flag.
    ReplayKeepsFrames(&'static str),
    /// `--format replay` for a CSV or JSON `--import`.
    ImportHasNoFrames,
    /// `--interval-ms` for an `--import`.
    ImportHasNoInterval,
}

impl crate::l10n::Localized for Refused {
    fn text<'a>(&'a self, l: &'a Localizer) -> Cow<'a, str> {
        match self {
            Self::Alarm(message) => Cow::Borrowed(message),
            Self::Clock(flag, reason) => tr!(
                l,
                "read-flag-refused",
                flags = *flag,
                reason = reason.as_str()
            ),
            Self::ReplayNamesItsDevice => tr!(
                l,
                "read-replay-names-its-device",
                command = "dmm-cli read --replay FILE"
            ),
            Self::ImportNamesItsDevice => tr!(l, "read-import-names-its-device"),
            Self::ReplayNeedsMeter(device) => {
                tr!(l, "read-replay-format-needs-meter", device = *device)
            }
            Self::ReplayKeepsFrames(flag) => {
                tr!(l, "read-replay-format-keeps-frames", flag = *flag)
            }
            Self::ImportHasNoFrames => tr!(l, "read-import-no-frames"),
            Self::ImportHasNoInterval => tr!(l, "read-import-no-interval"),
        }
    }
}

crate::l10n::localized_error!(Refused);

/// A recording or an export that would not read, and why: the closing
/// `Error:` line of `read --replay` or `--import`.
#[derive(Debug)]
pub(crate) struct NotReadable {
    path: std::path::PathBuf,
    error: dmm_shared::export::ReadError,
}

impl NotReadable {
    fn new(path: &Path, error: dmm_shared::export::ReadError) -> Self {
        Self {
            path: path.to_path_buf(),
            error,
        }
    }
}

impl crate::l10n::Localized for NotReadable {
    fn text<'a>(&'a self, l: &'a Localizer) -> Cow<'a, str> {
        Cow::Owned(format!(
            "{}: {}",
            self.path.display(),
            l.read_error(&self.error)
        ))
    }
}

crate::l10n::localized_error!(NotReadable);

/// Play a recording back as the session, with no meter on the cable.
///
/// The clock carries the recording's own start, so every reading exports at
/// the wall time it was measured at and two runs of the same file print the
/// same timestamps.
fn read_replay(
    path: &Path,
    interval_ms: u64,
    format: OutputFormat,
    options: RunOptions,
    clock: dmm_lib::Clock,
) -> Result<(), crate::l10n::Failure> {
    let l = crate::l10n::get();
    let replay = dmm_shared::replay::load(path)?;
    // The file names its meter, by the row it was recorded under.
    crate::open::note_picked(replay.listing, crate::open::PickedBy::Recording);
    // On screen the pace is the point; into a file it is only a wait.
    if clock.is_real() && !matches!(options.destination, output::Destination::Stdout) {
        let hint = tr!(l, "read-paced-replay", flag = "--mock-clock-scale max");
        eprintln!(
            "{}",
            style(format!("{} {hint}", tr!(l, "common-note-label"))).dim()
        );
    }
    let recorded =
        dmm_shared::replay::recorded_at(&replay).map_err(|e| NotReadable::new(path, e))?;
    let dmm = replay.open(clock.with_wall_origin(recorded.into()))?;
    // A copy is dated from its first frame, whose wall time the pinned
    // origin maps to when it was measured, so a copy that starts partway
    // through keeps the times of the session it came from.
    let out = read_output(
        format,
        &dmm,
        &options.transform,
        options.integrate,
        !replay.markers.is_empty() || options.alarm.is_some(),
        || {
            format::ReplayHeader {
                device: replay.listing.id().to_string(),
                model: replay.model.clone(),
                // The link the file recorded: re-exporting a recording must not turn
                // a Bluetooth session into a cable one.
                link: replay.link,
            }
        },
    )
    .with_view(replay.view.clone());
    // A log line, not a banner: a replay's output is what the meter's was,
    // and a note on stderr would land in every doc snippet taken from one.
    info!(
        "replaying {} ({}, recorded {})",
        path.display(),
        replay.device.id,
        replay.recorded,
    );
    // No interval floor: the file's own offsets pace the run.
    run_read_loop(
        setup_ctrlc()?,
        dmm,
        interval_ms,
        options,
        out,
        // The meter the file names, as the registry spells it — the name the
        // recording's own meter reported stays in the `# model:` line.
        replay.listing.display_name(),
        // A gap in the recording plays back as timeouts, and they are not a
        // quiet meter: there is no `--device` to check and nothing on the
        // cable to enable data transmission on. Nor is there a link to
        // reopen: the file ends the run.
        None,
        replay.markers.clone(),
    )
}

/// The session clock the `--mock-clock-*` flags ask for, or why they can't
/// be honoured, naming the flag the refusal is about.
fn session_clock(
    l: &Localizer,
    scale: Option<ClockScale>,
    preseed: Option<f64>,
    replay: bool,
) -> Result<dmm_lib::Clock, Refused> {
    const SCALE: &str = "--mock-clock-scale";
    const PRESEED: &str = "--mock-clock-preseed";
    const BOTH: &str = "--mock-clock-scale, --mock-clock-preseed";
    let refused = |flag, reason: Cow<'_, str>| Err(Refused::Clock(flag, reason.into_owned()));
    let factor = match scale {
        // A manual clock moves only when the run sleeps on it: a replay's
        // waits and the stream's pacing advance it by exactly the recorded
        // gaps, so every reading keeps its timestamp and nothing waits.
        Some(ClockScale::Max) if preseed.is_some() => {
            return refused(BOTH, tr!(l, "read-clock-max-no-preseed"));
        }
        // Only a replay ends: a live mock at `max` would print readings as
        // fast as the CPU goes until Ctrl+C, and grow a file without limit.
        Some(ClockScale::Max) if !replay => {
            return refused(SCALE, tr!(l, "read-clock-max-needs-replay"));
        }
        Some(ClockScale::Max) => return Ok(dmm_lib::Clock::manual()),
        Some(ClockScale::Factor(f)) => Some(f),
        None => None,
    };
    dmm_lib::Clock::from_flags(factor, preseed).map_err(|e| {
        let flag = match e {
            ClockFlagError::ScaleNotPositive(_) => SCALE,
            ClockFlagError::PreseedOutOfRange(_) => PRESEED,
        };
        Refused::Clock(flag, l.clock_flag(&e))
    })
}

/// One `read` run's output side, whatever the readings come from: the file
/// or stream they are written to, the statistics behind the closing summary,
/// and the transform every reading goes through first.
struct Run {
    writer: output::Writer,
    out: format::Output,
    session: dmm_lib::stats::SeriesStats,
    transform: Transform,
    integrate: bool,
    alarm: Option<Alarm>,
    // Ring the terminal bell on a breach: asked for, and stderr a terminal.
    bell: bool,
    // The number the next breach's marker takes: after every marker the
    // file being played already holds.
    next_marker: u32,
    // The idle unit the last stderr note named, so each change is said once.
    noted_idle: Option<String>,
}

impl Run {
    /// Open the output and write its header. `meter_name` is the meter's
    /// brand and model, as this run calls it: what the CSV comment and the
    /// JSON metadata carry, and, brand aside, what a file the run names
    /// itself is named after, the rule the GUI's Export… names its files by.
    fn open(
        options: RunOptions,
        meter_name: &str,
        out: format::Output,
        tick: Duration,
        next_marker: u32,
    ) -> Result<Self, crate::l10n::Failure> {
        let RunOptions {
            destination,
            count: _,
            integrate,
            transform,
            alarm,
        } = options;
        let l = crate::l10n::get();
        let mut writer = output::Writer::new(destination, meter_name)?;
        // Dim, "Note:" included: these say how the run was set up.
        let note = tr!(l, "common-note-label");
        if !transform.is_identity() {
            // On stderr so a redirected CSV or JSON stream stays
            // machine-readable, but visible: nothing in the output itself says
            // the numbers are not what the meter displayed.
            let scaled = tr!(l, "read-scaled", transform = transform.describe());
            eprintln!("{}", style(format!("{note} {scaled}")).dim());
        }
        if let Some(alarm) = &alarm {
            let on = tr!(
                l,
                "read-alarm-on",
                limits = l.alarm_limits(&alarm.limits, alarm.hysteresis)
            );
            eprintln!("{}", style(format!("{note} {on}")).dim());
        }
        if let Some(header) = out.header(meter_name) {
            write!(writer, "{header}")?;
        }
        // Min/Max/Mean and the integral are only meaningful within a single
        // mode and quantity; `SeriesStats` resets both whenever either moves,
        // and re-expresses them across an auto-range step, so the closing
        // summary only ever covers one comparable series.
        let mut session = dmm_lib::stats::SeriesStats::new(integrate);
        session.integrator.set_sample_interval(tick);
        Ok(Self {
            writer,
            out,
            session,
            transform,
            integrate,
            bell: alarm
                .as_ref()
                .is_some_and(|a| a.bell && std::io::stderr().is_terminal()),
            alarm: alarm.map(|a| Alarm::new(a.limits, a.hysteresis)),
            next_marker,
            noted_idle: None,
        })
    }

    /// Write one reading, with the marker on it if any. `file_extra` is how
    /// many of an imported reading's sub-values a scale appended when its
    /// file was written; 0 for a meter's.
    fn take(
        &mut self,
        mut m: dmm_lib::measurement::Measurement,
        file_extra: usize,
        marker: Option<(u32, &str)>,
    ) -> Result<(), crate::l10n::Failure> {
        // Before everything else: the unit-change check, the stats, the
        // integrator and the formatter must all see the same series, and after
        // a transform that series is the scaled one. (`--integrate` on a
        // relabelled clamp reading therefore integrates amps, not the
        // millivolts the meter sent.)
        self.transform.apply(&mut m);
        // Min/Max/Mean and the integral are only meaningful within a single
        // mode and quantity, and neither check subsumes the other — see
        // `SeriesStats`, which resets both accumulators and reports the
        // change, and carries them across an auto-range step without one.
        if let Some(change) = self.session.push(&m) {
            eprintln!(
                "{} {}",
                note_label(),
                series_reset(crate::l10n::get(), &change, self.integrate)
            );
        }
        let marker = self.check_alarm(&m, marker);
        let marker = marker.as_ref().map(|(n, note)| (*n, note.as_ref()));
        // Already `None` unless --integrate was given.
        let integral_display = self.session.integral_display();
        // Before the write: a file the run names itself is named after the
        // first reading, and later readings say whether the mode in that name
        // still describes the run.
        let mode = match m.value {
            dmm_lib::measurement::MeasuredValue::NoReading(_) => None,
            _ => Some(m.mode.as_ref()),
        };
        self.writer.saw(mode, m.wall_time.into())?;
        let extra_aux = file_extra + self.transform.extra_aux_count();
        self.out
            .write(&mut self.writer, &m, integral_display, extra_aux, marker)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Compare a reading against the alarm's limits, saying on stderr when it
    /// breaches one or the alarm goes idle, and return the marker the
    /// reading is written with: the file's own, a breach's, or the file's
    /// with the breach's note after its own.
    fn check_alarm<'m>(
        &mut self,
        m: &dmm_lib::measurement::Measurement,
        marker: Option<(u32, &'m str)>,
    ) -> Option<(u32, std::borrow::Cow<'m, str>)> {
        let marker = marker.map(|(n, note)| (n, std::borrow::Cow::Borrowed(note)));
        let Some(alarm) = &mut self.alarm else {
            return marker;
        };
        let l = crate::l10n::get();
        let breach = alarm.check(m);
        // Said on every change of what the readings are in, so an unattended
        // log follows each dial turn, not only the first away.
        if alarm.idle() != self.noted_idle.as_deref() {
            match (alarm.idle(), alarm.bound_unit()) {
                (Some(now), Some(watched)) => eprintln!(
                    "{} {}",
                    note_label(),
                    tr!(l, "read-alarm-idle", now = now, watched = watched),
                ),
                (None, Some(watched)) => eprintln!(
                    "{} {}",
                    note_label(),
                    tr!(l, "read-alarm-watching", watched = watched),
                ),
                _ => {}
            }
            self.noted_idle = alarm.idle().map(str::to_string);
        }
        let Some(breach) = breach else {
            return marker;
        };
        // In the run's language, on the line and in the marker the file
        // keeps, as a note the user typed would be.
        let said = l.breach_note(&breach);
        if !breach.crossing {
            // Out before the alarm saw it cross: said, not marked or rung.
            eprintln!("{} {said}", note_label());
            return marker;
        }
        eprintln!(
            "{}{} {}",
            if self.bell { "\x07" } else { "" },
            style(tr!(l, "read-alarm-label")).red().bold(),
            tr!(
                l,
                "read-alarm-breach",
                breach = said.as_str(),
                reading = format!("{} {}", m.value_display_str(), m.unit)
            ),
        );
        Some(match marker {
            Some((n, own)) => {
                let mut own = own.into_owned();
                dmm_shared::export::append_note(&mut own, &said);
                (n, own.into())
            }
            None => {
                let n = self.next_marker;
                self.next_marker += 1;
                (n, said.into())
            }
        })
    }

    /// A gap: nothing was read, so the integral does not bridge it.
    fn gap(&mut self) {
        self.session.integrator.push_gap();
    }

    /// Settle the output and print the closing summary; `fatal` is the
    /// failure that ended the run, reported once the readings it did get have
    /// been summarised and their file settled and named.
    fn close(
        mut self,
        protocol_errors: usize,
        fatal: Option<dmm_lib::error::Error>,
    ) -> Result<(), crate::l10n::Failure> {
        let l = crate::l10n::get();
        info!("shutting down");
        self.writer.flush()?;
        if protocol_errors > 0 {
            eprintln!(
                "\n{} {}",
                note_label(),
                tr!(l, "read-skipped-frames", count = protocol_errors),
            );
        }
        let session = &self.session;
        if let (Some(min), Some(max), Some(avg)) =
            (session.stats.min, session.stats.max, session.stats.avg())
        {
            // The figures are held in the latest reading's unit, every
            // sample behind them re-expressed in it across an auto-range
            // step; they reset whenever the mode or the quantity moves.
            let unit = session.unit().unwrap_or("");
            // Cyan figures, each in a range and to the resolution the meter
            // showed, with its unit after it.
            let figure = |v: f64| {
                let (number, unit) = dmm_shared::stat_figure(v, unit, Some(session.shown()));
                let number = style(number).cyan();
                if unit.is_empty() {
                    number.to_string()
                } else {
                    format!("{number} {unit}")
                }
            };
            let count = session.stats.count;
            let (min, max, mean) = (figure(min), figure(max), figure(avg));
            // One message per cell, so a new figure leaves the others'
            // translations alone. σ needs two samples; a one-reading
            // summary leaves it out.
            let mut cells = vec![
                tr!(l, "read-summary-count", count = count),
                tr!(l, "read-summary-min", min = min),
                tr!(l, "read-summary-max", max = max),
                tr!(l, "read-summary-mean", mean = mean),
            ];
            if let Some(sd) = session.stats.std_dev() {
                cells.push(tr!(l, "read-summary-sd", sd = figure(sd)));
            }
            let summary = cells.join(" | ");
            eprintln!("\n{} {summary}", style("---").dim());
            if let Some((value, disp_unit)) = session.integral_display() {
                let (number, disp_unit) = dmm_shared::stat_figure(value, disp_unit, None);
                let value = format!("{} {disp_unit}", style(number).cyan());
                let integral = match session.integrator.elapsed_secs() {
                    Some(s) => tr!(
                        l,
                        "read-integral-timed",
                        value = value,
                        seconds = style(format!("{s:.1}")).cyan().to_string(),
                    ),
                    None => tr!(l, "read-integral", value = value),
                };
                eprintln!("    {integral}");
                if session.integrator.skipped_intervals > 0 {
                    eprintln!(
                        "    {} {}",
                        note_label(),
                        tr!(
                            l,
                            "read-skipped-intervals",
                            count = session.integrator.skipped_intervals,
                            max = session.integrator.max_dt_secs().to_string(),
                        ),
                    );
                }
            }
        }
        if let Some(alarm) = &self.alarm {
            let unit = alarm
                .limits_unit(self.transform.unit.as_deref())
                .map(|u| format!(" {u}"))
                .unwrap_or_default();
            let limits = alarm.limits();
            let which = match (limits.high, limits.low) {
                (Some(_), Some(_)) => "both",
                (Some(_), None) => "high",
                _ => "low",
            };
            let limit = |v: Option<f64>| v.map(|v| format!("{v}{unit}")).unwrap_or_default();
            let counts = tr!(
                l,
                "read-alarm-counts",
                limits = which,
                above = alarm.high_count,
                high = limit(limits.high),
                below = alarm.low_count,
                low = limit(limits.low),
            );
            eprintln!("    {counts}");
        }
        // Only a file the run named itself is worth a line, either way: every
        // other destination is in the command the user typed.
        match self.writer.finish()? {
            output::Wrote::Named(path) => {
                let path = path.display().to_string();
                eprintln!("{}", style(tr!(l, "read-written-to", path = path)).dim());
            }
            // The reference promises `-o` prints the path it wrote, so a run
            // with no reading to name a file after has to say that instead of
            // nothing.
            output::Wrote::Nothing => {
                eprintln!("{}", style(tr!(l, "read-nothing-written")).dim())
            }
            output::Wrote::AsAsked => {}
        }
        match fatal {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }
}

/// The note that the statistics start again, for `change` in a run that
/// integrates the readings or doesn't.
fn series_reset<'a>(l: &'a Localizer, change: &'a SeriesChange, integrate: bool) -> Cow<'a, str> {
    let (kind, before, after) = match change {
        SeriesChange::Mode { prev, new } => (
            if integrate { "mode-integral" } else { "mode" },
            l.label(TextKind::Mode, prev),
            l.label(TextKind::Mode, new),
        ),
        SeriesChange::Unit { prev, new } => (
            if integrate { "unit-integral" } else { "unit" },
            Cow::Borrowed(prev.as_str()),
            Cow::Borrowed(new.as_str()),
        ),
    };
    tr!(
        l,
        "read-series-reset",
        change = kind,
        before = before,
        after = after
    )
}

/// What the `--alarm-*` flags ask of a run.
pub(crate) struct AlarmOptions {
    pub(crate) limits: Limits,
    pub(crate) hysteresis: Hysteresis,
    pub(crate) bell: bool,
}

/// The number a run's first breach marker takes: after the highest of the
/// markers a played-back file already holds, so none is numbered twice.
fn next_marker_number(numbers: impl Iterator<Item = u32>) -> u32 {
    numbers.max().map_or(1, |n| n + 1)
}

/// Say that a dropped link won't be reconnected, and what would let it.
fn warn_several_cables(several: SeveralCables) {
    let l = crate::l10n::get();
    eprintln!(
        "{} {} {}",
        warning_label(),
        l.help().several_cables(several),
        tr!(l, "read-several-cables-action")
    );
}

/// Opens a link as its pin says.
type OpenPinned<D> = Box<dyn Fn(&ReopenPin) -> dmm_lib::error::Result<D>>;

/// How a meter's link is opened again after it drops; `D` is the open meter.
struct Reopen<D> {
    pin: ReopenPin,
    /// What is plugged in, for each attempt to look at.
    listed: Box<dyn Fn() -> Vec<dmm_lib::DeviceInfo>>,
    open: OpenPinned<D>,
}

/// How a wait for a lost link ended.
#[derive(Debug, PartialEq)]
enum Reconnect<D> {
    Reopened(D),
    /// Ctrl+C.
    Stopped,
    Refused(SeveralCables),
}

/// What a run on a real meter has that the mock and a recording don't.
struct Meter<T: dmm_lib::transport::Transport> {
    /// The meter opened: a quiet one is shown its activation steps.
    device: &'static SelectableDevice,
    /// Brings a lost link back; `None` with `--no-reconnect` or with several
    /// cables that can't be told apart, when a lost link ends the run.
    reopen: Option<Reopen<dmm_lib::Dmm<T>>>,
    /// The wait before each attempt to reopen the link.
    interval: Duration,
}

/// Shared measurement loop for meters, the mock and recordings. `stop` is the
/// Ctrl+C flag: raised, it ends the run.
#[expect(
    clippy::too_many_arguments,
    reason = "the Ctrl+C flag comes from the caller, so a test can run the loop"
)]
fn run_read_loop<T: dmm_lib::transport::Transport>(
    stop: Arc<AtomicBool>,
    mut dmm: dmm_lib::Dmm<T>,
    interval_ms: u64,
    options: RunOptions,
    out: format::Output,
    // The meter a file the run has to name is named after; see `Run::open`.
    meter_name: &str,
    // A real meter's; `None` for the mock and a recording.
    meter: Option<Meter<T>>,
    // A recording's markers, in file order; none for a meter.
    markers: Vec<dmm_lib::replay::ReplayMarker>,
) -> Result<(), crate::l10n::Failure> {
    let count = options.count;
    let (device, mut reopen) = match meter {
        Some(Meter {
            device,
            reopen,
            interval,
        }) => (Some(device), reopen.map(|r| (r, interval))),
        None => (None, None),
    };
    // Each marker goes on the first reading played at or after its offset,
    // one per reading, so a frame `--interval-ms` or `--count` skips passes
    // its marker on rather than losing it — the rule the GUI plays by.
    let start = dmm.clock().wall_origin().map(|(start, _)| start);
    let mut markers: std::collections::VecDeque<_> = {
        let mut markers = markers;
        markers.sort_by_key(|m| m.offset);
        markers.into()
    };
    let tick = Duration::from_millis(interval_ms);
    let first_marker = next_marker_number(markers.iter().map(|m| m.number));
    // Stopped before a reading was taken: an output file named on the command
    // line hasn't been touched yet, and stays as it was.
    if stop.load(Ordering::SeqCst) {
        return Ok(());
    }
    let mut run = Run::open(options, meter_name, out, tick, first_marker)?;

    let mut i = 0usize;
    let mut protocol_errors = 0usize;
    let mut told_screen_only = false;
    let mut fatal = None;
    let mut stream = stream_over(&mut dmm, tick, &stop);

    while !stop.load(Ordering::SeqCst) && (count == 0 || i < count) {
        let event = stream.tick();
        // On any tick: a frame no parser knows may give no reading at all.
        crate::open::warn_unrecognised();
        match event {
            Ok(StreamEvent::Measurement(m)) => {
                crate::open::warn_other_data(stream.dmm());
                let due = markers.front().zip(start).is_some_and(|(next, start)| {
                    start
                        .checked_add(next.offset)
                        .is_some_and(|at| at <= m.timestamp)
                });
                let marker = if due { markers.pop_front() } else { None };
                // Once per run: the dashes on every line say the rest.
                if m.screen_only && !told_screen_only {
                    told_screen_only = true;
                    let l = crate::l10n::get();
                    eprintln!("{} {}", note_label(), l.help().screen_only_reading());
                }
                run.take(m, 0, marker.as_ref().map(|k| (k.number, k.note.as_str())))?;
                i += 1;
            }
            // A replay's last frame is behind us: the run ends with the file.
            Ok(StreamEvent::Ended) => break,
            // No wake is set: nothing else acts between readings.
            Ok(StreamEvent::Woken) => {}
            Ok(StreamEvent::Timeout { consecutive }) => {
                log::warn!("measurement timeout, retrying");
                // Nothing was read for a read timeout: no interval to bridge.
                run.gap();
                if consecutive == NO_RESPONSE_TIMEOUTS
                    && let Some(d) = device
                {
                    print_no_response_help(d);
                }
            }
            Err(e) if e.is_interrupted() => {
                // HID read returns EINTR when a signal (Ctrl-C) fires.
                // Break so the summary prints normally.
                break;
            }
            Err(e) if e.kind() == ErrorKind::Protocol => {
                // One unparseable frame must not end a long logging run: a
                // single noisy byte would throw away the rest of an overnight
                // capture even though the next request would have succeeded.
                // Report it and carry on, as `debug` does.
                //
                // Throttled: a meter parked on something no parser reads
                // fails on every sample, and an unattended run would
                // otherwise fill stderr with identical lines.
                log::warn!("protocol error: {e}");
                protocol_errors += 1;
                if protocol_errors == 1 || protocol_errors.is_multiple_of(100) {
                    let l = crate::l10n::get();
                    eprintln!(
                        "{} {}",
                        warning_label(),
                        tr!(
                            l,
                            "read-unreadable-frame",
                            error = l.error(&e),
                            count = protocol_errors
                        ),
                    );
                    // Before any reading, the frames are most often another
                    // meter's than the one picked.
                    let hint = (protocol_errors == 1 && i == 0)
                        .then(|| crate::open::wrong_pick_hint(l, &e))
                        .flatten();
                    if let Some(hint) = hint {
                        eprintln!("{}", style(hint).yellow());
                    }
                }
            }
            // A read cut short by Ctrl+C is the end of the run, not a lost
            // link.
            Err(_) if stop.load(Ordering::SeqCst) => break,
            Err(e) => {
                let Some((reopen, interval)) = &mut reopen else {
                    fatal = Some(e);
                    break;
                };
                log::warn!("link lost: {e}");
                // Nothing is read while the link is down: no interval to
                // bridge.
                run.gap();
                let l = crate::l10n::get();
                eprintln!(
                    "{} {}",
                    warning_label(),
                    tr!(l, "read-link-lost", error = l.error(&e))
                );
                // Release the old link before opening a new one: a Bluetooth
                // transport disconnects its peripheral on drop, and dropped
                // after the reopen that would cut the link just brought back
                // up on the same adapter.
                drop(dmm);
                dmm = match reconnect(reopen, &stop, *interval) {
                    Reconnect::Reopened(reopened) => reopened,
                    Reconnect::Stopped => break,
                    Reconnect::Refused(several) => {
                        warn_several_cables(several);
                        fatal = Some(e);
                        break;
                    }
                };
                eprintln!("{} {}", note_label(), tr!(l, "read-reconnected"));
                stream = stream_over(&mut dmm, tick, &stop);
            }
        }
    }
    run.close(protocol_errors, fatal)
}

/// A stream over `dmm` at `tick`, whose pacing sleep ends on the Ctrl+C flag
/// the loop checks, so a long --interval doesn't swallow the interrupt for a
/// whole tick.
fn stream_over<'a, T: dmm_lib::transport::Transport>(
    dmm: &'a mut dmm_lib::Dmm<T>,
    tick: Duration,
    stop: &Arc<AtomicBool>,
) -> MeasurementStream<'a, T> {
    let cancel = Arc::clone(stop);
    MeasurementStream::new(dmm, tick).with_cancel(move || cancel.load(Ordering::SeqCst))
}

/// How often the wait between reconnect attempts looks at the Ctrl+C flag.
const RECONNECT_POLL: Duration = Duration::from_millis(50);

/// Open `reopen`'s link every `interval` until it succeeds, Ctrl+C raises
/// `stop`, or what is plugged in refuses it. Every failed open is retried,
/// whatever its kind: an adapter that doesn't answer yet is what the wait is
/// for.
fn reconnect<D>(reopen: &mut Reopen<D>, stop: &AtomicBool, interval: Duration) -> Reconnect<D> {
    let mut attempt: u32 = 0;
    let mut told_waiting = false;
    loop {
        let until = Instant::now() + interval;
        loop {
            if stop.load(Ordering::SeqCst) {
                return Reconnect::Stopped;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            std::thread::sleep(left.min(RECONNECT_POLL));
        }
        attempt = attempt.saturating_add(1);
        match reopen.pin.check(&reopen.listed) {
            Attempt::Open => {}
            Attempt::Hold => continue,
            Attempt::WaitForOwnCable => {
                if !std::mem::replace(&mut told_waiting, true) {
                    let l = crate::l10n::get();
                    eprintln!(
                        "{} {} {}",
                        warning_label(),
                        l.help().waiting_for_own_cable(),
                        tr!(l, "read-own-cable-action")
                    );
                }
                continue;
            }
            Attempt::Refuse(several) => return Reconnect::Refused(several),
        }
        match (reopen.open)(&reopen.pin) {
            // Stopped as it ended: a link brought up meanwhile goes unused,
            // and an open called off isn't a failed attempt.
            _ if stop.load(Ordering::SeqCst) => return Reconnect::Stopped,
            Ok(reopened) => {
                info!("reconnected on attempt {attempt}");
                return Reconnect::Reopened(reopened);
            }
            Err(e) => {
                let e = reopen.pin.open_error(e);
                log::warn!("reconnect attempt {attempt} failed: {e}");
            }
        }
    }
}

/// Why `--import` refuses a flag alongside a CSV or JSON file: the readings
/// are already taken, and the file holds no frames to copy.
fn refuse_for_import(format: OutputFormat, interval_ms: u64) -> Option<Refused> {
    if format == OutputFormat::Replay {
        return Some(Refused::ImportHasNoFrames);
    }
    (interval_ms != 0).then_some(Refused::ImportHasNoInterval)
}

/// `read --import`: a CSV or JSON export read back and written out again, in
/// any format but a replay, with its markers and — into JSON — its view,
/// and the closing summary. The readings are the file's, so nothing waits.
fn read_import(
    path: &Path,
    format: OutputFormat,
    options: RunOptions,
) -> Result<(), crate::l10n::Failure> {
    use dmm_shared::export::{ExportKind, read_csv_from, read_json_from};
    let input = std::fs::File::open(path)
        .map(std::io::BufReader::new)
        .map_err(|e| crate::output::at_path(path, e))?;
    let imported = match ExportKind::of_file(path).map_err(|e| crate::output::at_path(path, e))? {
        ExportKind::Json => read_json_from(input, None),
        // A replay never reaches here: it plays as `--replay` does.
        ExportKind::Csv | ExportKind::Replay => read_csv_from(input, None),
    }
    .map_err(|e| NotReadable::new(path, e))?;
    let device = imported
        .device
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    // The file's appended groups stay trailing, and a `--scale` appends one
    // more after them.
    let layout = dmm_shared::export::CsvLayout {
        family_slots: imported.family_slots,
        extra_slots: imported.extra_slots + options.transform.extra_aux_count(),
        integral: options.integrate,
        markers: !imported.markers.is_empty() || options.alarm.is_some(),
    };
    let experimental = imported.readings.iter().any(|r| r.experimental);
    let out = format::Output::new(format, layout, experimental, unreachable_replay_header)
        .with_view(imported.view.clone());
    // The readings' gaps are the file's: judged by its median spacing, as
    // the GUI's import does, so a slow file does not break at every point.
    let offsets = imported.offsets();
    let tick = dmm_shared::export::median_spacing(&offsets);
    let first_marker = next_marker_number(imported.markers.iter().map(|m| m.number));
    let count = options.count;
    let mut run = Run::open(options, &device, out, tick, first_marker)?;
    let base = std::time::Instant::now();
    let mut markers = imported.markers.iter().peekable();
    for (i, (reading, offset)) in imported.readings.into_iter().zip(offsets).enumerate() {
        if count != 0 && i >= count {
            break;
        }
        let marker = markers.next_if(|m| m.reading == i);
        let file_extra = reading.extra_aux;
        run.take(
            reading.into_measurement(base + offset),
            file_extra,
            marker.map(|m| (m.number, m.note.as_str())),
        )?;
    }
    run.close(0, None)
}

/// `--format replay` is refused for a CSV or JSON import before the output
/// is built, so its header is never asked for.
fn unreachable_replay_header() -> format::ReplayHeader {
    format::ReplayHeader {
        device: String::new(),
        model: None,
        link: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Cmd, ReadArgs};
    use clap::Parser;
    use dmm_lib::protocol::registry;

    /// A file that would not read is worded on the terminal in the run's
    /// language, and logged in English.
    #[test]
    fn an_unreadable_file_is_logged_in_english() {
        use dmm_shared::export::{ReadError, ReadReason};
        let e = NotReadable::new(
            Path::new("x.csv"),
            ReadError {
                line: Some(1),
                reason: ReadReason::NoColumn("timestamp"),
            },
        );
        assert_eq!(e.to_string(), "x.csv: line 1: no `timestamp` column");
        let options = dmm_shared::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        let pseudo = Localizer::new(
            &dmm_shared::l10n::EN_US,
            &crate::l10n::resources(),
            &options,
        );
        let text = crate::l10n::Failure::from(e).text(&pseudo);
        assert!(text.starts_with("x.csv: ["), "{text}");
    }

    /// The English every test reads.
    fn l() -> &'static Localizer {
        crate::l10n::english()
    }

    /// A cable listed as `transport`, as `dmm_lib::list_devices` gives it.
    fn cable(transport: &'static str) -> dmm_lib::DeviceInfo {
        dmm_lib::DeviceInfo {
            path: format!("/dev/hidraw-{transport}"),
            product: None,
            serial: None,
            transport,
            not_heard: false,
        }
    }

    /// `max` is a clock that waits for nothing: an hour slept is an hour of
    /// session time at once. It hands out no burst, so a preseed is refused.
    #[test]
    fn the_max_scale_waits_for_nothing() {
        let clock = session_clock(l(), Some(ClockScale::Max), None, true).expect("max alone");
        let before = clock.now();
        clock.sleep(Duration::from_secs(3600));
        assert_eq!(clock.now(), before + Duration::from_secs(3600));
        assert!(!clock.is_real(), "refused on hardware like the other flags");
        let err = session_clock(l(), Some(ClockScale::Max), Some(10.0), true)
            .expect_err("no burst")
            .to_string();
        assert!(
            err.starts_with("--mock-clock-scale, --mock-clock-preseed: "),
            "{err}"
        );
    }

    /// A live mock never ends, so at `max` it would print as fast as the CPU
    /// goes until Ctrl+C: `max` takes a replay, and a factor does not need one.
    #[test]
    fn the_max_scale_needs_a_replay() {
        let err = session_clock(l(), Some(ClockScale::Max), None, false)
            .expect_err("no replay")
            .to_string();
        assert!(err.contains("'max'") && err.contains("--replay"), "{err}");
        assert!(session_clock(l(), Some(ClockScale::Factor(20.0)), None, false).is_ok());
    }

    /// A refused value names the flag it was typed after, not both.
    #[test]
    fn a_bad_clock_value_names_its_flag() {
        let scale = session_clock(l(), Some(ClockScale::Factor(0.0)), Some(5.0), false)
            .expect_err("zero scale")
            .to_string();
        assert_eq!(
            scale,
            "--mock-clock-scale: clock scale must be a positive number, got '0'"
        );
        let preseed = session_clock(l(), None, Some(-1.0), false)
            .expect_err("negative preseed")
            .to_string();
        assert!(preseed.starts_with("--mock-clock-preseed: "), "{preseed}");
    }

    /// Without `--format`, the file's extension says what to write; an
    /// extension nothing recognises, or no file at all, is text.
    #[test]
    fn the_output_file_extension_picks_the_format() {
        for (path, expected) in [
            ("readings.csv", OutputFormat::Csv),
            ("readings.json", OutputFormat::Json),
            ("bench.replay", OutputFormat::Replay),
            ("readings.TXT", OutputFormat::Text),
            ("readings.dat", OutputFormat::Text),
            ("readings", OutputFormat::Text),
        ] {
            let (format, note) = resolve_format(l(), None, Some(path));
            assert_eq!(format, expected, "{path}");
            assert!(note.is_none(), "{path}");
        }
        assert_eq!(resolve_format(l(), None, None).0, OutputFormat::Text);
    }

    /// `--format` wins over the name of the file it writes to — but a `.json`
    /// file holding CSV is worth a word.
    #[test]
    fn an_explicit_format_wins_over_the_extension_and_says_so() {
        let (format, note) = resolve_format(l(), Some(OutputFormat::Csv), Some("readings.json"));
        assert_eq!(format, OutputFormat::Csv);
        assert_eq!(
            note.as_deref(),
            Some("--format csv written to readings.json")
        );
        // Matching or unrecognised extensions say nothing.
        for path in ["readings.csv", "readings.dat", "readings"] {
            assert!(
                resolve_format(l(), Some(OutputFormat::Csv), Some(path))
                    .1
                    .is_none(),
                "{path}"
            );
        }
    }

    /// A replay file holds the meter's own frames, so the flags that
    /// re-express or accumulate the reading have nothing to act on.
    #[test]
    fn a_replay_run_refuses_the_flags_that_change_the_reading() {
        let read = |args: &[&str]| {
            let cli = Cli::try_parse_from(
                ["dmm-cli", "read", "--format", "replay"]
                    .iter()
                    .chain(args)
                    .copied(),
            )
            .expect("the flags parse");
            match cli.command {
                Cmd::Read(ReadArgs {
                    transform,
                    integrate,
                    ..
                }) => refuse_replay_format(
                    OutputFormat::Replay,
                    Selection::Auto,
                    false,
                    &transform,
                    integrate,
                )
                .map(|refused| refused.to_string()),
                _ => panic!("expected Read"),
            }
        };
        for (args, flag) in [
            (["--scale", "100"].as_slice(), "--scale"),
            (["--offset", "1"].as_slice(), "--offset"),
            (["--unit", "A"].as_slice(), "--unit"),
            (["--integrate"].as_slice(), "--integrate"),
        ] {
            let message = read(args).unwrap_or_else(|| panic!("{flag} should be refused"));
            assert!(message.contains(flag), "got {message}");
            assert!(message.contains("playing it back"), "got {message}");
        }
        assert!(read(&[]).is_none(), "a plain replay run is fine");
    }

    /// The mock synthesises its readings, so there are no frames to record —
    /// unless the run is copying a recording it was given.
    #[test]
    fn a_replay_run_needs_frames_to_record() {
        let mock = registry::resolve_selection("mock").expect("the mock is a registry device");
        let transform = TransformArgs {
            scale: None,
            offset: None,
            unit: None,
        };
        let message = refuse_replay_format(OutputFormat::Replay, mock, false, &transform, false)
            .expect("the mock has no frames")
            .to_string();
        assert!(
            message.contains("--format replay needs a real meter"),
            "got {message}"
        );
        assert!(
            refuse_replay_format(OutputFormat::Replay, mock, true, &transform, false).is_none(),
            "a recording being copied brings its own frames"
        );
    }

    /// A reopen with nothing pinned and nothing listed, opening with `open`.
    fn reopen_with<D>(open: impl Fn() -> dmm_lib::error::Result<D> + 'static) -> Reopen<D> {
        Reopen {
            pin: reopen_pin(None, None, true, None, None, None, Vec::new).unwrap(),
            listed: Box::new(Vec::new),
            open: Box::new(move |_| open()),
        }
    }

    /// A lost link is retried until it opens, each failure whatever its kind.
    #[test]
    fn a_lost_link_is_retried_until_it_opens() {
        let attempts = std::cell::Cell::new(0);
        let stop = AtomicBool::new(false);
        let mut reopen = reopen_with(move || {
            attempts.set(attempts.get() + 1);
            match attempts.get() {
                1 => Err(dmm_lib::error::Error::Timeout),
                2 => Err(dmm_lib::error::Error::AdapterNotFound("gone".into())),
                n => Ok(n),
            }
        });
        let reopened = reconnect(&mut reopen, &stop, Duration::from_millis(1));
        assert_eq!(reopened, Reconnect::Reopened(3));
    }

    /// Ctrl+C ends the wait for a link that never comes back.
    #[test]
    fn ctrl_c_ends_the_wait_for_a_link() {
        let stop = AtomicBool::new(true);
        let mut reopen =
            reopen_with(|| -> dmm_lib::error::Result<()> { panic!("no attempt after Ctrl+C") });
        let reopened = reconnect(&mut reopen, &stop, Duration::from_millis(1));
        assert_eq!(reopened, Reconnect::Stopped);
    }

    /// Ctrl+C during an attempt ends the wait there, whatever the attempt
    /// came to: an open it called off, or one that had already got through.
    #[test]
    fn ctrl_c_during_an_attempt_ends_the_wait() {
        let attempt = |outcome: fn() -> dmm_lib::error::Result<()>| {
            let stop = Arc::new(AtomicBool::new(false));
            let raised = Arc::clone(&stop);
            let opens = std::rc::Rc::new(std::cell::Cell::new(0));
            let count = std::rc::Rc::clone(&opens);
            let mut reopen = reopen_with(move || {
                count.set(count.get() + 1);
                raised.store(true, Ordering::SeqCst);
                outcome()
            });
            let ended = reconnect(&mut reopen, &stop, Duration::from_millis(1));
            assert_eq!(ended, Reconnect::Stopped);
            assert_eq!(opens.get(), 1, "another attempt after Ctrl+C");
        };
        attempt(|| Err(dmm_lib::error::Error::Cancelled));
        attempt(|| Ok(()));
    }

    /// A second cable found while waiting holds the attempt back, and in a
    /// row ends the wait, with nothing opened that could be the other meter.
    #[test]
    fn several_cables_at_the_reopen_refuse_it_without_opening() {
        let stop = AtomicBool::new(false);
        let mut reopen = reopen_with(|| -> dmm_lib::error::Result<()> {
            panic!("no open with several cables listed")
        });
        reopen.listed = Box::new(|| vec![cable("CP2110"), cable("CH9329")]);
        let reopened = reconnect(&mut reopen, &stop, Duration::from_millis(1));
        assert_eq!(reopened, Reconnect::Refused(SeveralCables { count: 2 }));
    }

    /// With its own cable gone and another plugged in, the wait opens nothing
    /// and goes on until its serial is back.
    #[test]
    fn a_pinned_run_waits_for_its_own_cable() {
        let serial = |transport, serial: &str| dmm_lib::DeviceInfo {
            serial: Some(serial.to_owned()),
            ..cable(transport)
        };
        let listings = std::cell::Cell::new(0);
        let stop = AtomicBool::new(false);
        let pin = reopen_pin(
            None,
            None,
            true,
            Some(dmm_lib::transport::Link::UsbCable),
            None,
            Some("OURS"),
            || vec![serial("CP2110", "OURS")],
        )
        .unwrap();
        let mut reopen = Reopen {
            pin,
            listed: Box::new(move || {
                listings.set(listings.get() + 1);
                match listings.get() {
                    1 | 2 => vec![serial("CH9329", "OTHER")],
                    _ => vec![serial("CP2110", "OURS")],
                }
            }),
            open: Box::new(|pin| Ok(pin.options(None).adapter.map(str::to_owned))),
        };
        let reopened = reconnect(&mut reopen, &stop, Duration::from_millis(1));
        assert_eq!(reopened, Reconnect::Reopened(Some("OURS".to_owned())));
    }

    /// A UT61E+ link that answers with its frames, then fails as a pulled
    /// cable does, and says when it is dropped.
    struct Unplugs {
        queue: dmm_lib::transport::mock::MockTransport,
        dropped: Arc<AtomicBool>,
    }

    impl dmm_lib::transport::Transport for Unplugs {
        fn write(&self, data: &[u8]) -> dmm_lib::error::Result<()> {
            self.queue.write(data)
        }

        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> dmm_lib::error::Result<usize> {
            self.queue.read_timeout(buf, timeout_ms)
        }

        fn link(&self) -> Option<dmm_lib::transport::Link> {
            None
        }
    }

    impl Drop for Unplugs {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    /// A UT61E+ meter on an [`Unplugs`] link that reads 5.209 V DC `readings`
    /// times before the link fails.
    fn unplugging_meter(
        readings: usize,
        dropped: &Arc<AtomicBool>,
    ) -> dmm_lib::error::Result<dmm_lib::Dmm<Unplugs>> {
        let payload = [
            0x02, 0x30, b' ', b' ', b'5', b'.', b'2', b'0', b'9', 0, 0, 0x30, 0x30, 0x30,
        ];
        let mut frame = vec![0xAB, 0xCD, (payload.len() + 2) as u8];
        frame.extend_from_slice(&payload);
        let sum = frame
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        frame.extend_from_slice(&sum.to_be_bytes());
        let device = registry::find_device("ut61eplus").expect("a registry device");
        dmm_lib::Dmm::new(
            Unplugs {
                queue: dmm_lib::transport::mock::MockTransport::new(vec![frame; readings])
                    .fail_after(readings, || dmm_lib::error::Error::LinkLost),
                dropped: Arc::clone(dropped),
            },
            (device.new_protocol)(),
        )
    }

    /// Run `count` readings from `dmm` into a CSV file, reopening its link
    /// with `reopen`, and return what the file holds, if it was written.
    fn read_through_drops<T: dmm_lib::transport::Transport>(
        dmm: dmm_lib::Dmm<T>,
        count: usize,
        stop: Arc<AtomicBool>,
        reopen: Option<Reopen<dmm_lib::Dmm<T>>>,
    ) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readings.csv");
        let transform = TransformArgs {
            scale: None,
            offset: None,
            unit: None,
        }
        .to_transform();
        let out = read_output(OutputFormat::Csv, &dmm, &transform, false, false, || {
            unreachable!("a CSV run has no replay header")
        });
        let options = RunOptions {
            destination: output::Destination::Path(path.clone()),
            count,
            integrate: false,
            transform,
            alarm: None,
        };
        let meter = Meter {
            device: registry::find_device("ut61eplus").expect("a registry device"),
            reopen,
            interval: Duration::from_millis(1),
        };
        run_read_loop(
            stop,
            dmm,
            0,
            options,
            out,
            "UNI-T UT61E+",
            Some(meter),
            Vec::new(),
        )
        .expect("a lost link that comes back is not a failure");
        std::fs::read_to_string(&path).ok()
    }

    /// Ctrl+C before a reading was taken leaves the output file alone:
    /// creating it would replace one named on the command line.
    #[test]
    fn ctrl_c_before_the_first_reading_writes_no_file() {
        let dmm = unplugging_meter(1, &Arc::new(AtomicBool::new(false))).unwrap();
        let written = read_through_drops(dmm, 1, Arc::new(AtomicBool::new(true)), None);
        assert_eq!(written, None);
    }

    /// A link that fails as Ctrl+C raises the flag, as a read cut short does.
    struct CutShort(Arc<AtomicBool>);

    impl dmm_lib::transport::Transport for CutShort {
        fn write(&self, _data: &[u8]) -> dmm_lib::error::Result<()> {
            self.0.store(true, Ordering::SeqCst);
            Err(dmm_lib::error::Error::LinkLost)
        }

        fn read_timeout(&self, _buf: &mut [u8], _ms: i32) -> dmm_lib::error::Result<usize> {
            Ok(0)
        }

        fn link(&self) -> Option<dmm_lib::transport::Link> {
            None
        }
    }

    /// With nothing to reconnect, Ctrl+C cutting a read short still ends the
    /// run cleanly: it isn't the lost link that `--no-reconnect` reports.
    #[test]
    fn a_read_cut_short_with_no_reconnect_is_not_a_failure() {
        let stop = Arc::new(AtomicBool::new(false));
        let device = registry::find_device("ut61eplus").expect("a registry device");
        let dmm = dmm_lib::Dmm::new(CutShort(Arc::clone(&stop)), (device.new_protocol)()).unwrap();
        let written = read_through_drops(dmm, 1, stop, None);
        assert!(written.is_some(), "the run opened its file and closed it");
    }

    /// The rows holding the mock meter's reading. Matches the whole CSV field:
    /// a bare `5.209` also turns up in a timestamp like `20:48:55.209881500`.
    fn readings_in(csv: &str) -> usize {
        csv.matches(",5.209,").count()
    }

    /// A link that drops is let go of, reopened, and read from again: the run
    /// carries on with the new meter's readings and ends as it would have.
    #[test]
    fn a_dropped_link_is_reopened_and_the_run_carries_on() {
        let first_dropped = Arc::new(AtomicBool::new(false));
        let dmm = unplugging_meter(1, &first_dropped).unwrap();
        let reopens = std::rc::Rc::new(std::cell::Cell::new(0));
        let reopen = reopen_with({
            let first_dropped = Arc::clone(&first_dropped);
            let reopens = std::rc::Rc::clone(&reopens);
            move || {
                assert!(
                    first_dropped.load(Ordering::SeqCst),
                    "the old link is let go of first"
                );
                reopens.set(reopens.get() + 1);
                unplugging_meter(1, &Arc::new(AtomicBool::new(false)))
            }
        });
        let stop = Arc::new(AtomicBool::new(false));
        let csv = read_through_drops(dmm, 2, stop, Some(reopen)).unwrap();
        assert_eq!(reopens.get(), 1);
        assert_eq!(readings_in(&csv), 2, "{csv}");
    }

    /// Ctrl+C while the link is down ends the run cleanly, with the readings
    /// taken before the drop.
    #[test]
    fn ctrl_c_while_the_link_is_down_ends_the_run_cleanly() {
        let dmm = unplugging_meter(1, &Arc::new(AtomicBool::new(false))).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let reopen = reopen_with({
            let stop = Arc::clone(&stop);
            move || {
                stop.store(true, Ordering::SeqCst);
                Err(dmm_lib::error::Error::LinkLost)
            }
        });
        let csv = read_through_drops(dmm, 2, stop, Some(reopen)).unwrap();
        assert_eq!(readings_in(&csv), 1, "{csv}");
    }
}
