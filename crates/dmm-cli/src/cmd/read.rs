//! `read`: a meter's readings, live or played back from a replay file, as
//! text, CSV, JSON or the meter's own frames, with the closing summary.

use super::setup_ctrlc;
use crate::cli::TransformArgs;
use crate::format::{self, OutputFormat};
use crate::open::{
    open_mock_device, open_with_help, opened_device, print_no_response_help, requires_hardware,
    selection_id,
};
use crate::output;
use console::style;
use dmm_lib::error::ErrorKind;
use dmm_lib::protocol::registry::{SelectableDevice, Selection};
use dmm_lib::stream::{MeasurementStream, NO_RESPONSE_TIMEOUTS, StreamEvent};
use dmm_lib::transform::Transform;
use log::info;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_read(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    interval_ms: u64,
    format: OutputFormat,
    destination: output::Destination,
    count: usize,
    integrate: bool,
    transform: &Transform,
    mock_mode: Option<String>,
    replay: Option<PathBuf>,
    // Virtual session time; real unless a --mock-clock-* flag asked otherwise.
    clock: dmm_lib::Clock,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = &replay {
        // No cable to pace this run, so the clock flags apply as they do to
        // the mock and `refuse_clock_on_hardware` is not consulted.
        return read_replay(
            path,
            interval_ms,
            format,
            destination,
            count,
            integrate,
            transform,
            clock,
        );
    }
    refuse_clock_on_hardware(selection, &clock)?;
    if requires_hardware(selection) {
        let (mut dmm, device) = open_with_help(selection, opts)?;
        // Once, before the loop: on a UT61+ asking the meter its name is a
        // command it answers with a beep, so only a replay file — whose
        // header carries the name — pays for it, and not when detection has.
        let model = (format == OutputFormat::Replay)
            .then(|| dmm.get_name().ok().flatten())
            .flatten();
        // The link these readings come over, so a session played back from
        // the file says what it was recorded on rather than nothing.
        let link = dmm.transport().link();
        let out = read_output(format, &dmm, transform, integrate, || {
            dmm_lib::replay::header(device.id, &recorded_now(), model.as_deref(), link)
        });
        info!("connected, starting measurement loop");
        run_read_loop(
            &mut dmm,
            interval_ms,
            out,
            destination,
            device.display_name,
            count,
            Some(device),
            integrate,
            transform,
            |_| false,
        )
    } else {
        let mut dmm = open_mock_device(selection, mock_mode, clock)?;
        info!("mock device connected, starting measurement loop");
        // Mock returns instantly — use 100ms floor to simulate ~10 Hz
        let interval_ms = if interval_ms == 0 { 100 } else { interval_ms };
        // `--format replay` is refused for a device that synthesises its
        // readings, so the header below is never built.
        let out = read_output(format, &dmm, transform, integrate, || {
            dmm_lib::replay::header(selection_id(selection), &recorded_now(), None, None)
        });
        // Not hardware, so the selection names a registry entry — `auto` is a
        // cable to open and never lands here.
        let meter_name = opened_device(selection)
            .map_or_else(|| selection_id(selection), |device| device.display_name);
        // No timeout to warn about: the mock always answers.
        run_read_loop(
            &mut dmm,
            interval_ms,
            out,
            destination,
            meter_name,
            count,
            None,
            integrate,
            transform,
            |_| false,
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
    replay_header: impl FnOnce() -> String,
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
        // The CLI places no markers.
        markers: false,
    };
    let experimental = !dmm.profile().stability.is_verified();
    format::Output::new(format, layout, experimental, replay_header)
}

/// When a recording being written now was made, for its header.
fn recorded_now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}

/// The format a run writes and the note it earns: `--format` when given, else
/// what the `-o` file's extension names, else text.
///
/// The flag wins over the extension — the file is what the user asked for by
/// name — so a `.json` file holding CSV is a note rather than a refusal.
fn resolve_format(
    asked: Option<OutputFormat>,
    path: Option<&str>,
) -> (OutputFormat, Option<String>) {
    let named = path
        .and_then(|p| Path::new(p).extension()?.to_str())
        .and_then(OutputFormat::from_extension);
    match (asked, named, path) {
        (Some(asked), Some(named), Some(path)) if asked != named => (
            asked,
            Some(format!("--format {} written to {path}", asked.name())),
        ),
        (Some(asked), _, _) => (asked, None),
        (None, Some(named), _) => (named, None),
        (None, None, _) => (OutputFormat::Text, None),
    }
}

/// What `read` writes, where it goes, and the note the pair earns — printed
/// by the caller, which knows whether the run is going to happen at all.
pub(crate) fn resolve_output(
    asked: &Option<OutputFormat>,
    output: Option<Option<String>>,
) -> (OutputFormat, output::Destination, Option<String>) {
    let (format, note) = resolve_format(*asked, output.as_ref().and_then(|o| o.as_deref()));
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
pub(crate) fn refuse_replay_format(
    format: OutputFormat,
    selection: Selection,
    // A run already playing a recording has the frames to copy, whatever the
    // settings file names as the meter.
    replaying: bool,
    transform: &TransformArgs,
    integrate: bool,
) -> Option<String> {
    if format != OutputFormat::Replay {
        return None;
    }
    if !replaying && !requires_hardware(selection) {
        return Some(format!(
            "--format replay needs a real meter; nothing to record from --device {}",
            selection_id(selection),
        ));
    }
    let flag = transform
        .flag_given()
        .or(integrate.then_some("--integrate"))?;
    Some(format!(
        "a replay holds the meter's own frames; pass {flag} when playing it back"
    ))
}

/// Why `--device` is refused alongside `--replay`.
pub(crate) const REPLAY_NAMES_ITS_DEVICE: &str =
    "--replay names its own meter in the file; drop --device (dmm-cli read --replay FILE)";

/// Play a recording back as the session, with no meter on the cable.
///
/// The clock carries the recording's own start, so every reading exports at
/// the wall time it was measured at and two runs of the same file print the
/// same timestamps.
#[allow(clippy::too_many_arguments)]
fn read_replay(
    path: &Path,
    interval_ms: u64,
    format: OutputFormat,
    destination: output::Destination,
    count: usize,
    integrate: bool,
    transform: &Transform,
    clock: dmm_lib::Clock,
) -> Result<(), Box<dyn std::error::Error>> {
    let replay = dmm_lib::replay::Replay::load(path)?;
    // dmm-lib has no date library, so the header line comes back as text.
    let recorded = chrono::DateTime::parse_from_rfc3339(&replay.recorded).map_err(|e| {
        format!(
            "{}: `# recorded: {}` is not an RFC3339 date ({e})",
            path.display(),
            replay.recorded,
        )
    })?;
    let mut dmm = replay.open(clock.with_wall_origin(recorded.into()))?;
    // A copy keeps the session it came from, so the frames it holds export at
    // the times they were measured at whichever file they are played from.
    let out = read_output(format, &dmm, transform, integrate, || {
        // The link the file recorded: re-exporting a recording must not turn
        // a Bluetooth session into a cable one.
        dmm_lib::replay::header(
            replay.device.id,
            &replay.recorded,
            replay.model.as_deref(),
            replay.link,
        )
    });
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
        &mut dmm,
        interval_ms,
        out,
        destination,
        // The meter the file names, as the registry spells it — the name the
        // recording's own meter reported stays in the `# model:` line.
        replay.device.display_name,
        count,
        // A gap in the recording plays back as timeouts, and they are not a
        // quiet meter: there is no `--device` to check and nothing on the
        // cable to enable data transmission on.
        None,
        integrate,
        transform,
        // The run ends with the file: past it the session only repeats the
        // last reading, which is for a GUI to keep on screen.
        dmm_lib::replay::ReplayTransport::played_out,
    )
}

/// Refuse a bent session clock on a device that is paced by USB.
///
/// Checked before the device is opened, so a hardware `--device` with the
/// clock flags fails with no meter attached and nothing to plug in — and so
/// does Auto, which has a cable to open before it knows anything at all.
fn refuse_clock_on_hardware(
    selection: Selection,
    clock: &dmm_lib::Clock,
) -> Result<(), Box<dyn std::error::Error>> {
    if requires_hardware(selection) && !clock.is_real() {
        return Err(dmm_shared::help::MOCK_CLOCK_MOCK_ONLY.into());
    }
    Ok(())
}

/// Shared measurement loop for both real and mock devices.
#[allow(clippy::too_many_arguments)]
fn run_read_loop<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    interval_ms: u64,
    mut out: format::Output,
    destination: output::Destination,
    // The meter a file the run has to name is named after: the registry's
    // name for it, whatever the meter reports and whatever format is being
    // written, so one meter's exports all sort together. The same rule the
    // GUI's Export… names its files by.
    meter_name: &str,
    count: usize,
    // When set, timeout warnings include device-specific activation instructions.
    device: Option<&'static SelectableDevice>,
    integrate: bool,
    // Applied to every reading before anything else sees it; the identity
    // transform (no --scale/--offset/--unit) is a no-op.
    transform: &Transform,
    // Whether the source has nothing more to give: a replay's end.
    played_out: impl Fn(&T) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;

    // The profile's name, which the CSV comment and the JSON metadata carry,
    // is the family's — `meter_name` is what this meter answers to.
    let model_name = dmm.profile().model_name;
    let mut writer = output::Writer::new(destination, meter_name)?;

    if !transform.is_identity() {
        // On stderr so a redirected CSV or JSON stream stays machine-readable,
        // but visible: nothing in the output itself says the numbers are not
        // what the meter displayed.
        eprintln!(
            "{}",
            style(format!(
                "Note: readings scaled in software ({})",
                transform.describe()
            ))
            .dim()
        );
    }
    if let Some(header) = out.header(model_name) {
        write!(writer, "{header}")?;
    }

    let tick = Duration::from_millis(interval_ms);
    // Session time, not wall time, is what the readings carry: a preseeded
    // run's first readings are minutes old and must export as such.
    let wall_clock = dmm_lib::WallClock::from_clock(dmm.clock());
    // Min/Max/Avg and the integral are only meaningful within a single mode
    // and unit; `SeriesStats` resets both whenever either moves, so the
    // closing summary only ever covers one comparable series.
    let mut session = dmm_lib::stats::SeriesStats::new(integrate);
    let mut i = 0usize;
    let mut protocol_errors = 0usize;
    // The failure that ended the run, reported once the readings it did get
    // have been summarised and their file settled and named.
    let mut fatal = None;
    // Give the pacing sleep the same Ctrl-C flag the loop checks, so a long
    // --interval doesn't swallow the interrupt for a whole tick.
    let cancel = running.clone();
    let mut stream =
        MeasurementStream::new(dmm, tick).with_cancel(move || !cancel.load(Ordering::SeqCst));

    while running.load(Ordering::SeqCst)
        && (count == 0 || i < count)
        && !played_out(stream.dmm().transport())
    {
        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                // Before everything else: the unit-change check, the stats,
                // the integrator and the formatter must all see the same
                // series, and after a transform that series is the scaled one.
                // (`--integrate` on a relabelled clamp reading therefore
                // integrates amps, not the millivolts the meter sent.)
                let mut m = m;
                transform.apply(&mut m);

                // Min/Max/Avg and the integral are only meaningful within a
                // single mode and unit, and neither check subsumes the other —
                // see `SeriesStats`, which resets both accumulators and reports
                // the change.
                if let Some(change) = session.push(&m) {
                    let what = if integrate {
                        "statistics and integral"
                    } else {
                        "statistics"
                    };
                    eprintln!("{} {change}, {what} reset", style("Note:").yellow());
                }

                // Already `None` unless --integrate was given.
                let integral_display = session.integral_display();

                // Before the write: a file the run names itself is named after
                // the first reading, and later readings say whether the mode
                // in that name still describes the run.
                let mode = match m.value {
                    dmm_lib::measurement::MeasuredValue::NoReading(_) => None,
                    _ => Some(m.mode.as_ref()),
                };
                writer.saw(mode, wall_clock.wall_time_for(m.timestamp).into())?;
                out.write(&mut writer, &m, &wall_clock, integral_display)?;
                writer.flush()?;
                i += 1;
            }
            Ok(StreamEvent::Timeout { consecutive }) => {
                log::warn!("measurement timeout, retrying");
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
                // Throttled: a meter parked in a mode this family's tables
                // don't cover fails on every sample, and an unattended run
                // would otherwise fill stderr with identical lines.
                log::warn!("protocol error: {e}");
                protocol_errors += 1;
                if protocol_errors == 1 || protocol_errors.is_multiple_of(100) {
                    eprintln!(
                        "{} {e} (skipped {protocol_errors} so far)",
                        style("Warning:").yellow()
                    );
                }
            }
            Err(e) => {
                fatal = Some(e);
                break;
            }
        }
    }

    info!("shutting down");
    writer.flush()?;

    if protocol_errors > 0 {
        eprintln!(
            "\n{} {protocol_errors} readings skipped (unreadable frames)",
            style("Note:").yellow(),
        );
    }

    if let (Some(min), Some(max), Some(avg)) =
        (session.stats.min, session.stats.max, session.stats.avg())
    {
        // Name the unit the figures are in. They reset whenever the mode or
        // the unit moves, so this is the unit every sample behind them was
        // measured in.
        let unit_suffix = session
            .unit()
            .filter(|u| !u.is_empty())
            .map(|u| format!(" {u}"))
            .unwrap_or_default();
        eprintln!(
            "\n{} {} samples | Min: {}{unit_suffix} | Max: {}{unit_suffix} | Avg: {}{unit_suffix}",
            style("---").dim(),
            session.stats.count,
            style(format!("{min:.4}")).cyan(),
            style(format!("{max:.4}")).cyan(),
            style(format!("{avg:.4}")).cyan(),
        );
        if let Some((value, disp_unit)) = session.integral_display() {
            let dt_str = session
                .integrator
                .elapsed_secs()
                .map(|s| format!(" ({}s)", style(format!("{s:.1}")).cyan()))
                .unwrap_or_default();
            eprintln!(
                "    Integral: {} {disp_unit}{dt_str}",
                style(format!("{value:.4}")).cyan(),
            );
            if session.integrator.skipped_intervals > 0 {
                eprintln!(
                    "    {} {} intervals skipped (sample spacing exceeds the 2 s integrator limit \u{2014} lower --interval-ms for more frequent samples or expect a partial integral)",
                    style("Note:").yellow(),
                    session.integrator.skipped_intervals,
                );
            }
        }
    }

    // Only a file the run named itself is worth a line, either way: every
    // other destination is in the command the user typed.
    match writer.finish()? {
        output::Wrote::Named(path) => {
            eprintln!("{}", style(format!("Written to {}", path.display())).dim());
        }
        // The reference promises `-o` prints the path it wrote, so a run with
        // no reading to name a file after has to say that instead of nothing.
        output::Wrote::Nothing => eprintln!(
            "{}",
            style("No readings arrived, so no file was written").dim()
        ),
        output::Wrote::AsAsked => {}
    }
    match fatal {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Cmd};
    use crate::test_fixtures::selection;
    use clap::Parser;
    use dmm_lib::protocol::registry;

    /// A bent clock on a USB-paced meter would stamp readings with instants
    /// the meter never produced, so `read` refuses before it opens anything.
    /// Auto is one of those: it has a cable to open either way.
    #[test]
    fn clock_flags_are_refused_on_a_hardware_device() {
        let mock = selection("mock");
        let virtual_clock = dmm_lib::Clock::from_flags(None, Some(90.0)).expect("valid preseed");

        for hardware in [selection("ut61eplus"), selection("auto")] {
            let err = refuse_clock_on_hardware(hardware, &virtual_clock)
                .expect_err("a hardware device must refuse a virtual clock");
            assert_eq!(err.to_string(), dmm_shared::help::MOCK_CLOCK_MOCK_ONLY);
            assert!(refuse_clock_on_hardware(hardware, &dmm_lib::Clock::real()).is_ok());
        }

        assert!(refuse_clock_on_hardware(mock, &virtual_clock).is_ok());
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
            let (format, note) = resolve_format(None, Some(path));
            assert_eq!(format, expected, "{path}");
            assert!(note.is_none(), "{path}");
        }
        assert_eq!(resolve_format(None, None).0, OutputFormat::Text);
    }

    /// `--format` wins over the name of the file it writes to — but a `.json`
    /// file holding CSV is worth a word.
    #[test]
    fn an_explicit_format_wins_over_the_extension_and_says_so() {
        let (format, note) = resolve_format(Some(OutputFormat::Csv), Some("readings.json"));
        assert_eq!(format, OutputFormat::Csv);
        assert_eq!(
            note.as_deref(),
            Some("--format csv written to readings.json")
        );
        // Matching or unrecognised extensions say nothing.
        for path in ["readings.csv", "readings.dat", "readings"] {
            assert!(
                resolve_format(Some(OutputFormat::Csv), Some(path))
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
                Cmd::Read {
                    transform,
                    integrate,
                    ..
                } => refuse_replay_format(
                    OutputFormat::Replay,
                    Selection::Auto,
                    false,
                    &transform,
                    integrate,
                ),
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
            .expect("the mock has no frames");
        assert!(
            message.contains("--format replay needs a real meter"),
            "got {message}"
        );
        assert!(
            refuse_replay_format(OutputFormat::Replay, mock, true, &transform, false).is_none(),
            "a recording being copied brings its own frames"
        );
    }
}
