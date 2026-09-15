mod capture;
mod drive;
mod format;
mod output;
mod plan;
mod recording;
mod watch;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use console::style;
use dmm_lib::binary_help::ConnectedAdapters;
use dmm_lib::error::ErrorKind;
use dmm_lib::protocol::registry::{self, SelectableDevice, Selection};
use dmm_lib::protocol::{Choice, Setting};
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transform::{FactorError, Transform};
use log::{error, info};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

fn version_string() -> &'static str {
    dmm_lib::binary_help::version_string(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

#[derive(Parser)]
#[command(
    name = "dmm-cli",
    version = version_string(),
    about = "CLI tool for UNI-T and Voltcraft digital multimeters",
    after_help = "Run with --help for the full list of supported devices and the \
                  shared-settings file path.\n\n\
                  Set NO_COLOR=1 to disable colored output.\n\
                  Help / GitHub: https://github.com/antoinecellerier/dmm-tools",
    // after_long_help is set dynamically in main() so the actual per-platform
    // settings file path appears in the output.
    after_long_help = ""
)]
struct Cli {
    /// Device to connect to [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...].
    /// If omitted, falls back to `device_family` in ~/.config/dmm-tools/settings.json
    /// (written by dmm-gui), then to `auto`, which detects the meter over the
    /// USB cable.
    #[arg(long)]
    device: Option<String>,

    /// Select a specific USB adapter when multiple are connected.
    /// Use serial number or HID device path from 'dmm-cli list' output.
    #[arg(long, value_name = "SERIAL_OR_PATH")]
    adapter: Option<String>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List connected CP2110 devices
    List,
    /// Connect and print device info
    Info,
    /// Continuously read measurements
    Read {
        /// Interval between readings in milliseconds (0 = fastest, ~10 Hz)
        #[arg(long, default_value = "0")]
        interval_ms: u64,
        /// Output format [default: text, or what -o's extension names]
        #[arg(long)]
        format: Option<OutputFormat>,
        /// Output file (stdout if not specified). Given without a name, the
        /// file is named after the meter, its mode and the run's start.
        #[arg(short, long, value_name = "FILE", num_args(0..=1))]
        output: Option<Option<String>>,
        /// Number of readings (0 = unlimited, Ctrl+C to stop)
        #[arg(long, default_value = "0")]
        count: usize,
        /// Show cumulative time-integral (charge for current modes, V·s for voltage)
        #[arg(long)]
        integrate: bool,
        #[command(flatten)]
        transform: TransformArgs,
        /// Pin mock device to a specific mode (only with --device mock).
        /// Without this, mock cycles through all modes automatically.
        #[arg(long, long_help = build_mock_mode_help())]
        mock_mode: Option<String>,
        /// Play back a file written by --format replay instead of opening a meter
        #[arg(long, value_name = "FILE", conflicts_with = "mock_mode")]
        replay: Option<PathBuf>,
        /// Run session time at this multiple of real time (mock only).
        /// Hidden: a contributor tool for fast runs, not a user-facing knob.
        #[arg(long, hide = true)]
        mock_clock_scale: Option<f64>,
        /// Start the run with this many seconds of readings already behind it,
        /// produced as fast as the mock answers (mock only). Hidden, as above.
        #[arg(long, hide = true)]
        mock_clock_preseed: Option<f64>,
    },
    /// Send a button press command to the meter.
    /// Run with no arguments to list available commands for the selected device.
    Command {
        /// Command name (run without arguments to see available commands)
        action: Option<String>,
    },
    /// List what the meter's settings can be switched to from where it sits now.
    /// Run with no argument for every setting that offers a choice.
    Get {
        /// Setting to list (omit for all of them)
        #[arg(value_enum)]
        setting: Option<SettingArg>,
        /// Output format
        #[arg(long, default_value = "text")]
        format: SettingsFormat,
        /// Pin mock device to a specific mode (only with --device mock).
        /// Without this, mock cycles through all modes automatically.
        #[arg(long)]
        mock_mode: Option<String>,
    },
    /// Switch one of the meter's settings without touching the meter.
    /// Run without a choice to list what that setting reaches from here.
    Set {
        /// Setting to switch
        #[arg(value_enum)]
        setting: SettingArg,
        /// Value label, or a unique fragment of one, from the listing (run without it to see them)
        choice: Option<String>,
        /// Pin mock device to a specific mode (only with --device mock).
        /// Without this, mock cycles through all modes automatically.
        #[arg(long)]
        mock_mode: Option<String>,
    },
    /// Raw hex dump mode for protocol debugging
    Debug {
        /// Number of requests to send (0 = unlimited)
        #[arg(long, default_value = "1")]
        count: usize,
        /// Interval between requests in milliseconds
        #[arg(long, default_value = "500")]
        interval_ms: u64,
    },
    /// Generate shell completions
    #[command(after_help = "\
Install completions for your shell:
  bash:  dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli
  zsh:   dmm-cli completions zsh > ~/.zfunc/_dmm-cli
  fish:  dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish
  pwsh:  dmm-cli completions powershell >> $PROFILE")]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
    /// Guided protocol capture for bug reports and verification
    Capture {
        /// Output file (default: capture-<device>.yaml). Overrides auto-naming.
        #[arg(short, long)]
        output: Option<String>,
        /// Only run specific steps (comma-separated IDs, e.g. "dcmv,temp,duty")
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        /// Only run steps not yet confirmed on real hardware, plus the freeform pass
        #[arg(long)]
        unverified: bool,
        /// Run the steps in a maintainer's plan file instead of the device's own list
        #[arg(long, value_name = "FILE", conflicts_with_all = ["steps", "unverified", "list_steps"])]
        plan: Option<String>,
        /// Trust nothing the parser says: detect steps by raw byte changes and confirm each one
        #[arg(long)]
        sniff: bool,
        /// Don't let the tool set ranges and flags itself after each mode step
        #[arg(long)]
        no_drive: bool,
        /// Wait MS before sampling any step, for readings that settle slowly
        #[arg(long, value_name = "MS", default_value_t = 0)]
        settle: u64,
        /// List all available step IDs and exit
        #[arg(long)]
        list_steps: bool,
        /// How --list-steps prints the list (md is the issue checklist)
        #[arg(long, value_enum, default_value = "text", requires = "list_steps")]
        format: StepListFormat,
    },
}

/// The `read` flags that build a software [`Transform`].
///
/// Flattened into `Cmd::Read` so the three flags stay one unit here and in
/// `--help`, and so `read` keeps its identity behaviour when none are given.
#[derive(clap::Args, Clone)]
struct TransformArgs {
    /// Multiply the reading, taken in base units (V, A, Ω, …), by FACTOR.
    /// A 10 mV/A clamp is --scale 100; a 100:1 probe is --scale 100.
    #[arg(long, value_name = "FACTOR", allow_negative_numbers = true, value_parser = parse_scale)]
    scale: Option<f64>,
    /// Add VALUE after scaling (32 with --scale 1.8 turns °C into °F)
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true, value_parser = parse_offset)]
    offset: Option<f64>,
    /// Label the scaled reading with this unit instead of the meter's base unit
    #[arg(long, value_name = "LABEL")]
    unit: Option<String>,
}

impl TransformArgs {
    /// The transform these flags describe. With none given the result is the
    /// identity, which `Transform::apply` skips entirely — so an unscaled
    /// `read` is byte-for-byte what it always was.
    fn to_transform(&self) -> Transform {
        Transform::linear(
            self.scale.unwrap_or(1.0),
            self.offset.unwrap_or(0.0),
            self.unit.clone(),
        )
    }

    /// The first of the three flags that was given, so a refusal names the one
    /// the user typed rather than listing all three.
    fn flag_given(&self) -> Option<&'static str> {
        match self {
            Self { scale: Some(_), .. } => Some("--scale"),
            Self {
                offset: Some(_), ..
            } => Some("--offset"),
            Self { unit: Some(_), .. } => Some("--unit"),
            _ => None,
        }
    }
}

/// Parse a transform flag's number and check it against the rules
/// [`Transform`] sets for that flag.
///
/// The rules are shared with the GUI's Scale row; the wording is not, so
/// each [`FactorError`] is turned into a message here. `flag` names the
/// offending flag so `--scale` and `--offset` cannot word the same rejection
/// two ways.
fn parse_factor(
    flag: &str,
    s: &str,
    check: fn(f64) -> Result<f64, FactorError>,
) -> Result<f64, String> {
    let value: f64 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    check(value).map_err(|e| match e {
        FactorError::NotFinite => format!("{flag} must be a finite number, got `{s}`"),
        FactorError::ZeroScale => {
            "scale must not be zero — it would flatten every reading to the offset".to_string()
        }
    })
}

/// Reject the two scale factors that destroy the reading rather than
/// re-express it: zero collapses every sample onto the offset, and NaN/inf
/// poison the stats and the integral.
fn parse_scale(s: &str) -> Result<f64, String> {
    parse_factor("scale", s, Transform::check_scale)
}

/// Any finite shift is a meaningful offset — zero and negatives included — so
/// only NaN and infinity are rejected, on the same grounds as `--scale`.
fn parse_offset(s: &str) -> Result<f64, String> {
    parse_factor("offset", s, Transform::check_offset)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Csv,
    Json,
    /// The meter's own frames, for --replay to play back
    Replay,
}

impl OutputFormat {
    /// What `--format` calls this format, for a message that quotes the flag
    /// back at the user.
    fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Replay => "replay",
        }
    }

    /// The extension a file of this format carries: what a bare `-o` names its
    /// file with, and what picks the format when `-o` names a file and
    /// `--format` doesn't.
    fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Replay => "replay",
        }
    }

    /// The format a file extension names, if it names one.
    fn from_extension(extension: &str) -> Option<Self> {
        [Self::Text, Self::Csv, Self::Json, Self::Replay]
            .into_iter()
            .find(|f| f.extension().eq_ignore_ascii_case(extension))
    }
}

/// What `get` and `set` name on the command line, one word per
/// [`dmm_lib::protocol::Setting`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum SettingArg {
    Mode,
    Range,
    Hold,
    Rel,
    Minmax,
    Peak,
}

impl From<SettingArg> for Setting {
    fn from(arg: SettingArg) -> Self {
        match arg {
            SettingArg::Mode => Setting::Mode,
            SettingArg::Range => Setting::Range,
            SettingArg::Hold => Setting::Hold,
            SettingArg::Rel => Setting::Rel,
            SettingArg::Minmax => Setting::MinMax,
            SettingArg::Peak => Setting::Peak,
        }
    }
}

/// How `get` prints a listing. Separate from [`OutputFormat`] because a
/// settings listing has no CSV form — reusing that enum would take
/// `--format csv` and then have nothing to do with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum SettingsFormat {
    Text,
    Json,
}

/// How `capture --list-steps` prints the step list. `Md` is the checklist the
/// device verification issues carry, so the issue and the code can't drift.
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub(crate) enum StepListFormat {
    Text,
    Md,
}

fn main() {
    env_logger::init();

    // Build CLI with registry-generated --device long_help and a dynamic
    // after_long_help that resolves the actual per-platform settings path.
    let mut cmd = Cli::command();
    let device_help = build_device_help();
    cmd = cmd.mut_arg("device", |a| a.long_help(device_help));
    cmd = cmd.after_long_help(build_after_long_help());
    let cli =
        Cli::from_arg_matches_mut(&mut cmd.get_matches()).unwrap_or_else(|e: clap::Error| e.exit());

    // Whether the meter was named on the command line, as opposed to coming
    // from the settings file or from detection. Only `read --replay` asks.
    let device_named = cli.device.is_some();

    // Nothing named a meter, so none is assumed: detection is the fallback,
    // and `--device` or the settings file pins a model when the user wants
    // one.
    let (device_id, _source) = dmm_settings::resolve_device_family(
        cli.device.as_deref(),
        dmm_settings::SharedSettings::load_if_exists().as_ref(),
        registry::AUTO_DEVICE_ID,
    );
    let selection = match registry::resolve_selection(&device_id) {
        Some(s) => s,
        None => {
            eprintln!(
                "{} unknown device: {}",
                style("Error:").red().bold(),
                device_id,
            );
            std::process::exit(1);
        }
    };

    let adapter = cli.adapter.as_deref();

    // Device-independent commands — handle before mock/real split
    let result = match cli.command {
        Cmd::List => cmd_list(),
        Cmd::Completions { shell } => {
            match shell {
                Some(shell) => {
                    clap_complete::generate(
                        shell,
                        &mut Cli::command(),
                        "dmm-cli",
                        &mut std::io::stdout(),
                    );
                }
                None => {
                    let _ = Cli::command()
                        .find_subcommand_mut("completions")
                        .unwrap()
                        .print_long_help();
                }
            }
            Ok(())
        }

        // The mock is a registry device with nothing to open, so only the
        // commands that have no meaning without hardware branch on it here —
        // `read`, `command`, `get` and `set` pick the mock transport
        // themselves and take the same arm as every other device. Auto never
        // lands here: detecting a meter means opening a cable.
        Cmd::Info | Cmd::Debug { .. } if !requires_hardware(selection) => {
            eprintln!(
                "{} This command requires real hardware (not supported with --device {}).",
                style("Error:").red().bold(),
                selection_id(selection),
            );
            std::process::exit(1);
        }

        Cmd::Info => cmd_info(selection, adapter),
        Cmd::Read {
            interval_ms,
            format,
            output,
            count,
            integrate,
            transform,
            mock_mode,
            replay,
            mock_clock_scale,
            mock_clock_preseed,
        } => {
            // `from_flags` names the offending value, not the flag it came
            // from, so that both binaries can reuse the sentence.
            let clock = match dmm_lib::Clock::from_flags(mock_clock_scale, mock_clock_preseed) {
                Ok(clock) => clock,
                Err(msg) => {
                    eprintln!(
                        "{} --mock-clock-scale/--mock-clock-preseed: {msg}",
                        style("Error:").red().bold(),
                    );
                    std::process::exit(1);
                }
            };
            // A replay file names its own meter, so a `--device` alongside it
            // would either be ignored or contradict the file. Only a typed
            // flag is refused: the settings file names a meter for every run,
            // and a replay must not need it edited.
            if replay.is_some() && device_named {
                eprintln!(
                    "{} {}",
                    style("Error:").red().bold(),
                    REPLAY_NAMES_ITS_DEVICE,
                );
                std::process::exit(1);
            }
            // Settled before anything opens, so a run that cannot write what
            // it was asked for fails with no meter attached.
            let (format, destination) = resolve_output(&format, output);
            match refuse_replay_format(format, selection, replay.is_some(), &transform, integrate) {
                Some(message) => Err(message.into()),
                None => cmd_read(
                    selection,
                    adapter,
                    interval_ms,
                    format,
                    destination,
                    count,
                    integrate,
                    &transform.to_transform(),
                    mock_mode,
                    replay,
                    clock,
                ),
            }
        }
        Cmd::Command { action } => cmd_command(selection, adapter, action),
        Cmd::Get {
            setting,
            format,
            mock_mode,
        } => cmd_get(selection, adapter, setting, format, mock_mode),
        Cmd::Set {
            setting,
            choice,
            mock_mode,
        } => cmd_set(selection, adapter, setting, choice, mock_mode),
        Cmd::Debug { count, interval_ms } => cmd_debug(selection, adapter, count, interval_ms),
        Cmd::Capture {
            output,
            steps,
            unverified,
            plan,
            sniff,
            no_drive,
            settle,
            list_steps,
            format,
        } => {
            if list_steps {
                // Device-scoped: the steps come from the selected device's
                // protocol, so what's listed is what `--steps` will match.
                // With nothing selected, that means asking the cable first.
                device_for_listing(selection, adapter).map(|device| {
                    capture::list_steps(device, format);
                })
            } else {
                open_recording_with_help(selection, adapter).and_then(|(dmm, recorder, device)| {
                    capture::cmd_capture(
                        output,
                        steps,
                        unverified,
                        sniff,
                        no_drive,
                        std::time::Duration::from_millis(settle),
                        plan,
                        dmm,
                        recorder,
                        device,
                        detected_name(),
                    )
                })
            }
        }
    };

    if let Err(e) = result {
        error!("{e}");
        let msg = e.to_string();
        // The activation instructions belong to one meter, so they are only
        // printed once one is settled on: the one named, or the one detection
        // found before the meter went quiet.
        if msg.contains("timeout")
            && let Some(device) = opened_device(selection)
        {
            print_no_response_help(device);
        } else {
            eprintln!("{} {msg}", style("Error:").red().bold());
        }
        std::process::exit(1);
    }
}

/// Build long help text for --device from the registry.
fn build_device_help() -> String {
    dmm_lib::binary_help::device_help("Device to connect to.")
}

/// Build long help text for --mock-mode from the mock's own mode table.
fn build_mock_mode_help() -> String {
    dmm_lib::binary_help::mock_mode_help(
        "Pin the mock device to a specific measurement mode instead of \
         auto-cycling. Only effective with --device mock.",
        "--device mock read --mock-mode dcv",
    )
}

/// Resolve the shared settings file path for display in help text.
/// Returns the platform-specific location via `dmm-settings`, or a
/// sensible placeholder if the platform config dir is unavailable.
fn resolved_config_path_display() -> String {
    dmm_settings::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/dmm-tools/settings.json".to_string())
}

/// Build the `after_long_help` text shown by `dmm-cli --help` (long form).
/// Dynamic so the actual resolved settings path appears per-platform.
///
/// Each line is a standalone item rather than wrapped prose — clap doesn't
/// re-wrap `after_long_help` text, so hardcoded mid-sentence line breaks
/// read poorly. A table-style layout with short, self-contained lines
/// avoids the issue on any terminal width.
fn build_after_long_help() -> String {
    format!(
        "CONFIGURATION:\n\
         \x20 Settings file (shared with dmm-gui):\n\
         \x20   {path}\n\
         \n\
         \x20 --device precedence:\n\
         \x20   1. Command-line flag\n\
         \x20   2. device_family from the settings file above\n\
         \x20   3. auto \u{2014} detect the meter over the USB cable\n\
         \n\
         ENVIRONMENT:\n\
         \x20 RUST_LOG    Log filter. Use `dmm_lib=trace` for wire-level debugging.\n\
         \x20 NO_COLOR    Set to 1 to disable colored terminal output.\n\
         \n\
         Help / GitHub: https://github.com/antoinecellerier/dmm-tools",
        path = resolved_config_path_display(),
    )
}

/// Print a "no response" warning with device-specific activation instructions.
fn print_no_response_help(device: &SelectableDevice) {
    eprintln!(
        "{} No response from meter. Check that --device {} is correct \
         and that data transmission is enabled.",
        style("Warning:").yellow(),
        device.id,
    );
    eprintln!("{}", style(device.activation_instructions).dim());
}

/// Print platform-specific setup instructions when no USB cable is detected.
///
/// The hint's indented lines are commands to run or URLs to open; dimming
/// them keeps the prose that explains them in the foreground.
fn print_transport_setup_help() {
    for line in dmm_lib::binary_help::transport_setup_hint() {
        if line.starts_with(' ') {
            eprintln!("{}", style(line).dim());
        } else {
            eprintln!("{line}");
        }
    }
}

/// Set up a Ctrl+C handler that clears the returned flag when triggered.
fn setup_ctrlc() -> Result<Arc<AtomicBool>, Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    Ok(running)
}

/// The meter handle every command works through, with the transport picked
/// at runtime.
type BoxedDmm = dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>;

/// What detection settled on, once a [`Selection::Auto`] open has run.
///
/// The two sites that need it afterwards — the `info` listing and the timeout
/// help `main` prints once a command has already returned — are past the point
/// where the opener could hand it to them, and a run only ever opens one meter.
static AUTO_DETECTED: OnceLock<dmm_lib::detect::Detected> = OnceLock::new();

/// How the help for a silent cable ends: the way to bypass detection, and
/// the ask that turns an unrecognised meter into a registry fix.
const NOT_IDENTIFIED_SELF_HELP: &str = "\nIf the meter is on and transmitting but still not recognised, name it:\n  \
     dmm-cli --device <id> ...      (dmm-cli --help lists the ids)\n\
     and please report it with RUST_LOG=dmm_lib=debug output at\n  \
     https://github.com/antoinecellerier/dmm-tools/issues";

/// Whether this selection means opening a USB cable. Auto does: there is
/// nothing to detect without one.
fn requires_hardware(selection: Selection) -> bool {
    match selection {
        Selection::Auto => true,
        Selection::Device(device) => device.requires_hardware,
    }
}

/// What the user would pass to `--device` to make this selection again.
fn selection_id(selection: Selection) -> &'static str {
    match selection {
        Selection::Auto => registry::AUTO_DEVICE_ID,
        Selection::Device(device) => device.id,
    }
}

/// The entry a command ran against: the one named, or the one detection found.
///
/// `None` only when Auto never got as far as identifying a meter.
fn opened_device(selection: Selection) -> Option<&'static SelectableDevice> {
    match selection {
        Selection::Auto => AUTO_DETECTED.get().map(|d| d.device),
        Selection::Device(device) => Some(device),
    }
}

/// Record what detection found, and say so.
///
/// The meter was picked for the user, so the name it was picked by belongs on
/// screen — dim and on stderr, like every other notice, so a redirected CSV or
/// JSON stream stays machine-readable.
fn note_detected(detected: dmm_lib::detect::Detected) -> &'static SelectableDevice {
    let detected = AUTO_DETECTED.get_or_init(|| detected);
    let device = detected.device;
    // A name the registry doesn't carry still identifies the family, and the
    // tables in use are then someone else's — say whose, and how to override.
    // Either way the line ends with the exact option that skips the probe next
    // time, so the user never has to look the id up.
    let note = match &detected.reported_name {
        Some(name) if name != device.display_name => format!(
            " (the meter reports {name:?}; using {} tables \u{2014} pass --device {} to keep them, \
             or another id to override)",
            device.display_name, device.id
        ),
        _ => format!(
            " (pass --device {} or pick it in dmm-gui to skip detection next time)",
            device.id
        ),
    };
    eprintln!(
        "{}",
        style(format!("Detected {}{note}", device.display_name)).dim()
    );
    device
}

/// Open the meter with helpful error messages for common failures, and say
/// which entry answered — the one named, or the one detection found.
fn open_with_help(
    selection: Selection,
    adapter: Option<&str>,
) -> Result<(BoxedDmm, &'static SelectableDevice), Box<dyn std::error::Error>> {
    let (dmm, device) = match selection {
        Selection::Auto => {
            let (dmm, detected) =
                dmm_lib::open_auto(adapter).map_err(|e| open_error_help(selection, e))?;
            let device = note_detected(detected);
            (dmm, device)
        }
        Selection::Device(device) => (
            dmm_lib::open_device_by_id_auto(device.id, adapter)
                .map_err(|e| open_error_help(selection, e))?,
            device,
        ),
    };
    warn_if_experimental(device, dmm.profile());
    Ok((dmm, device))
}

/// Open the meter with every wire byte recorded, the init handshake included,
/// for `capture` to put in its report.
fn open_recording_with_help(
    selection: Selection,
    adapter: Option<&str>,
) -> Result<
    (
        BoxedDmm,
        recording::SharedRecorder,
        &'static SelectableDevice,
    ),
    Box<dyn std::error::Error>,
> {
    type Boxed = Box<dyn dmm_lib::transport::Transport>;
    let (transport, recorder, device, protocol): (Boxed, _, _, _) = match selection {
        // Wrap first, then detect: the probe and the meter's answer to it are
        // the first bytes on the wire, and a report that starts after them
        // hides how the meter was picked.
        Selection::Auto => {
            let (transport, bridge) =
                dmm_lib::open_transport(&[], adapter).map_err(|e| open_error_help(selection, e))?;
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            let transport = Box::new(transport) as Boxed;
            let detected = dmm_lib::detect::detect_device(&*transport, bridge)
                .map_err(|e| open_error_help(selection, e))?;
            let device = note_detected(detected);
            (transport, recorder, device, (device.new_protocol)())
        }
        Selection::Device(device) => {
            // The mock has no USB link to open, and none to record either — it
            // goes through the same recorder so `capture` has one code path.
            let (transport, protocol): (Boxed, _) = if device.requires_hardware {
                dmm_lib::open_transport_by_id_auto(device.id, adapter)
                    .map_err(|e| open_error_help(selection, e))?
            } else {
                (
                    Box::new(dmm_lib::transport::NullTransport),
                    (device.new_protocol)(),
                )
            };
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            (Box::new(transport) as Boxed, recorder, device, protocol)
        }
    };
    let dmm = dmm_lib::Dmm::new(transport, protocol).map_err(|e| open_error_help(selection, e))?;
    warn_if_experimental(device, dmm.profile());
    Ok((dmm, recorder, device))
}

/// Identify the meter and hand the cable straight back, for the listings that
/// need a registry entry but nothing from the meter itself.
fn detect_only(
    adapter: Option<&str>,
) -> Result<&'static SelectableDevice, Box<dyn std::error::Error>> {
    let (transport, bridge) =
        dmm_lib::open_transport(&[], adapter).map_err(|e| open_error_help(Selection::Auto, e))?;
    let detected = dmm_lib::detect::detect_device(&*transport, bridge)
        .map_err(|e| open_error_help(Selection::Auto, e))?;
    Ok(note_detected(detected))
}

/// The entry a listing describes. Nothing is read from the meter, but with
/// nothing selected there is still a meter to identify — the alternative is
/// listing another model's commands or capture steps.
fn device_for_listing(
    selection: Selection,
    adapter: Option<&str>,
) -> Result<&'static SelectableDevice, Box<dyn std::error::Error>> {
    match selection {
        Selection::Auto => detect_only(adapter),
        Selection::Device(device) => Ok(device),
    }
}

/// Tell the user an unverified protocol is in use and how to help fix it.
fn warn_if_experimental(
    device: &'static SelectableDevice,
    profile: &dmm_lib::protocol::DeviceProfile,
) {
    if profile.stability.is_verified() {
        return;
    }
    eprintln!(
        "{}",
        style(format!(
            "WARNING: {}",
            dmm_lib::binary_help::experimental_warning(profile.model_name, profile.stability)
        ))
        .yellow()
        .bold()
    );
    eprintln!(
        "{}",
        style("Run 'capture' to generate a report for validation:").yellow()
    );
    eprintln!(
        "{}",
        style(format!("  dmm-cli --device {} capture", device.id)).yellow()
    );
    eprintln!(
        "{}",
        style(format!("Report feedback: {}", profile.feedback_url())).yellow()
    );
}

/// The meters on a bridge, grouped by the steps that switch their
/// transmission on.
///
/// Several entries share one instruction block — every UT61+/UT161 model, the
/// UT8802 and the UT8803 — so a list per device would print the same four
/// lines six times over. Registry order is kept, and a group is named by the
/// display names that share it.
fn activation_groups(
    devices: &[&'static SelectableDevice],
) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut groups: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for device in devices {
        match groups
            .iter_mut()
            .find(|(instructions, _)| *instructions == device.activation_instructions)
        {
            Some((_, names)) => names.push(device.display_name),
            None => groups.push((device.activation_instructions, vec![device.display_name])),
        }
    }
    groups
}

/// Print setup help for the failures a user can act on, and return the error
/// to report.
fn open_error_help(
    selection: Selection,
    error: dmm_lib::error::Error,
) -> Box<dyn std::error::Error> {
    match error {
        dmm_lib::error::Error::NoTransportFound => {
            eprintln!("{}", style("USB cable not found.").yellow().bold());
            print_transport_setup_help();
            // Nothing is known about the meter when it was never named and the
            // cable it would have been identified through never opened.
            if let Selection::Device(device) = selection {
                let proto = (device.new_protocol)();
                let profile = proto.profile();
                if !profile.stability.is_verified() {
                    eprintln!(
                        "{}",
                        style(format!(
                            "{} Report feedback: {}",
                            dmm_lib::binary_help::experimental_warning(
                                profile.model_name,
                                profile.stability
                            ),
                            profile.feedback_url()
                        ))
                        .yellow()
                    );
                }
            }
            "device not found".into()
        }
        // The cable is there and nothing on it spoke. Every meter it could
        // carry has something the user has to switch on, so list them.
        dmm_lib::error::Error::DeviceNotIdentified { bridge } => {
            eprintln!(
                "{}",
                style("No meter answered over the USB cable.")
                    .yellow()
                    .bold()
            );
            for (instructions, names) in activation_groups(&dmm_lib::devices_on_bridge(bridge)) {
                eprintln!("\n{}", style(names.join(", ")).yellow());
                for line in instructions.lines() {
                    eprintln!("{}", style(format!("  {line}")).dim());
                }
            }
            for line in NOT_IDENTIFIED_SELF_HELP.lines() {
                eprintln!("{}", style(line).yellow());
            }
            "device not identified".into()
        }
        dmm_lib::error::Error::AdapterNotFound(ref detail) => {
            eprintln!(
                "{} adapter not found: {detail}",
                style("Error:").red().bold()
            );
            // Only a real list is worth setting apart and worth a hint —
            // "nothing is connected" and "couldn't look" are complete on
            // their own.
            let adapters = dmm_lib::binary_help::connected_adapters();
            let listed = matches!(adapters, ConnectedAdapters::Listed(_));
            if listed {
                eprintln!();
            }
            for line in adapters.lines() {
                eprintln!("{}", style(line).yellow());
            }
            if listed {
                eprintln!(
                    "\n{}",
                    style("Use --adapter <serial-or-path> to select one.").dim()
                );
            }
            "adapter not found".into()
        }
        e => e.into(),
    }
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let devices = dmm_lib::list_devices()?;
    if devices.is_empty() {
        eprintln!("{}", style("No devices found.").yellow());
        print_transport_setup_help();
        return Ok(());
    }
    for (i, dev) in devices.iter().enumerate() {
        println!("{} {dev}", style(format!("[{i}]")).cyan());
    }
    if devices.len() > 1 {
        eprintln!(
            "\n{}",
            style("Tip: use --adapter <serial-or-path> to select a specific device").dim()
        );
    }
    Ok(())
}

/// The name the detection probe already got from the meter, if it ran and
/// the meter gave one: asking again would spend a second round trip — and on
/// a UT61+ a second beep — on a name we have.
fn detected_name() -> Option<String> {
    AUTO_DETECTED.get().and_then(|d| d.reported_name.clone())
}

fn cmd_info(selection: Selection, adapter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut dmm, _device) = open_with_help(selection, adapter)?;
    let name = match detected_name() {
        Some(name) => Some(name),
        None => dmm.get_name()?,
    };
    match name {
        Some(ref n) => println!("Device: {}", style(n).bold()),
        None => println!("Device: {}", style("(name not supported)").dim()),
    }
    // Which tables are in use is only in question when the user named no
    // meter, so the line is only there when they didn't.
    if let Some(detected) = AUTO_DETECTED.get() {
        let reported = match &detected.reported_name {
            Some(name) if name != detected.device.display_name => {
                format!(" (the meter reports {name:?})")
            }
            _ => String::new(),
        };
        println!(
            "Detected: {}{reported}",
            style(detected.device.display_name).bold()
        );
    }

    println!("Transport: {}", dmm.transport().transport_name());
    if let Ok(info) = dmm.transport().transport_info() {
        println!("  {info}");
    }
    if let Ok(status) = dmm.transport().transport_status() {
        println!("  Status: {status}");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_read(
    selection: Selection,
    adapter: Option<&str>,
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
        let (mut dmm, device) = open_with_help(selection, adapter)?;
        // Once, before the loop: on a UT61+ asking the meter its name is a
        // command it answers with a beep, so only a replay file — whose
        // header carries the name — pays for it.
        let model = (format == OutputFormat::Replay)
            .then(|| dmm.get_name().ok().flatten())
            .flatten();
        let out = read_output(format, &dmm, transform, integrate, || {
            dmm_lib::replay::header(device.id, &recorded_now(), model.as_deref())
        });
        info!("connected, starting measurement loop");
        // The name the meter gave, where the run already asked for one.
        let meter_name = model.as_deref().unwrap_or(device.display_name);
        run_read_loop(
            &mut dmm,
            interval_ms,
            out,
            destination,
            meter_name,
            count,
            Some(device),
            integrate,
            transform,
        )
    } else {
        let mut dmm = open_mock_device(mock_mode, clock)?;
        info!("mock device connected, starting measurement loop");
        // Mock returns instantly — use 100ms floor to simulate ~10 Hz
        let interval_ms = if interval_ms == 0 { 100 } else { interval_ms };
        // `--format replay` is refused for a device that synthesises its
        // readings, so the header below is never built.
        let out = read_output(format, &dmm, transform, integrate, || {
            dmm_lib::replay::header(selection_id(selection), &recorded_now(), None)
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
    let layout = dmm_lib::export::CsvLayout {
        family_slots: dmm.profile().max_aux_values,
        extra_slots: transform.extra_aux_count(),
        integral: integrate,
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

/// What `read` writes and where it goes.
fn resolve_output(
    asked: &Option<OutputFormat>,
    output: Option<Option<String>>,
) -> (OutputFormat, output::Destination) {
    let (format, note) = resolve_format(*asked, output.as_ref().and_then(|o| o.as_deref()));
    if let Some(note) = note {
        eprintln!("{} {note}", style("Note:").yellow());
    }
    let destination = match output {
        None => output::Destination::Stdout,
        // A bare `-o`: the first reading names the file.
        Some(None) => output::Destination::Auto {
            extension: format.extension(),
        },
        Some(Some(path)) => output::Destination::Path(path.into()),
    };
    (format, destination)
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
const REPLAY_NAMES_ITS_DEVICE: &str =
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
        dmm_lib::replay::header(replay.device.id, &replay.recorded, replay.model.as_deref())
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
        // The name the meter reported when the recording was made.
        replay
            .model
            .as_deref()
            .unwrap_or(replay.device.display_name),
        count,
        // A gap in the recording plays back as timeouts, and they are not a
        // quiet meter: there is no `--device` to check and nothing on the
        // cable to enable data transmission on.
        None,
        integrate,
        transform,
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
        return Err(dmm_lib::binary_help::MOCK_CLOCK_MOCK_ONLY.into());
    }
    Ok(())
}

/// Open the mock on `clock`, pinned to `mock_mode` when one was given.
///
/// Shared by every subcommand that takes `--mock-mode`, so an unknown mode
/// name is rejected with the same message (and the same list of valid names)
/// wherever it is passed. Only `read` has clock flags; the others pass
/// [`dmm_lib::Clock::real`].
fn open_mock_device(
    mock_mode: Option<String>,
    clock: dmm_lib::Clock,
) -> Result<dmm_lib::Dmm<dmm_lib::transport::NullTransport>, Box<dyn std::error::Error>> {
    let mode = match mock_mode {
        Some(mode_str) => Some(
            mode_str
                .parse::<dmm_lib::mock::MockMode>()
                .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?,
        ),
        None => None,
    };
    Ok(dmm_lib::mock::open_mock_clocked(mode, clock)?)
}

/// Shared measurement loop for both real and mock devices.
#[allow(clippy::too_many_arguments)]
fn run_read_loop<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    interval_ms: u64,
    mut out: format::Output,
    destination: output::Destination,
    // What the meter calls itself, for a file the run has to name: the name it
    // reported where the run already has one, else the registry's. The same
    // rule the GUI's Export… names its files by, so the two agree.
    meter_name: &str,
    count: usize,
    // When set, timeout warnings include device-specific activation instructions.
    device: Option<&'static SelectableDevice>,
    integrate: bool,
    // Applied to every reading before anything else sees it; the identity
    // transform (no --scale/--offset/--unit) is a no-op.
    transform: &Transform,
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

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
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
                writer.saw(&m.mode, wall_clock.wall_time_for(m.timestamp).into())?;
                out.write(&mut writer, &m, &wall_clock, integral_display)?;
                writer.flush()?;
                i += 1;
            }
            Ok(StreamEvent::Timeout { consecutive }) => {
                log::warn!("measurement timeout, retrying");
                if consecutive == 5
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

    // Only a file the run named itself is worth a line: every other
    // destination is in the command the user typed.
    if let Some(path) = writer.finish()? {
        eprintln!("{}", style(format!("Written to {}", path.display())).dim());
    }
    match fatal {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

fn cmd_command(
    selection: Selection,
    adapter: Option<&str>,
    action: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = match action {
        Some(a) => a,
        None => return print_available_commands(device_for_listing(selection, adapter)?),
    };

    if requires_hardware(selection) {
        let (mut dmm, _device) = open_with_help(selection, adapter)?;
        dmm.send_command(&action)?;
    } else {
        let mut dmm = dmm_lib::mock::open_mock()?;
        dmm.send_command(&action)?;
    }
    println!("{} {action}", style("Sent").green());
    Ok(())
}

/// Print supported commands for a device without connecting.
fn print_available_commands(
    device: &'static SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol = (device.new_protocol)();
    let profile = protocol.profile();
    if profile.supported_commands.is_empty() {
        eprintln!(
            "{} No remote commands implemented yet for {}.",
            style("Note:").yellow(),
            profile.model_name,
        );
    } else {
        println!(
            "Available commands for {}:",
            style(profile.model_name).bold()
        );
        for cmd in profile.supported_commands {
            println!("  {cmd}");
        }
    }
    Ok(())
}

/// The one thing a user can do about a value the meter won't take. Every
/// failure below ends here, so they end in the same words.
const CHECK_DIAL_HINT: &str = "check the dial position";

/// How long to wait for a switched setting to show up in the measurement
/// stream. The meter acknowledges the command before the frame carrying the
/// new value arrives, so "accepted" and "switched" are two separate answers.
const SWITCH_TIMEOUT: Duration = Duration::from_secs(2);

/// Gap between polls while waiting for that frame.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The [`Setting::Range`] choice id the library gives autorange. The CLI needs
/// it to know which row carries the live-range note and which sentence a
/// switch to it gets.
const AUTO_RANGE_ID: u16 = 0;

/// Shown in place of the live value when the meter is on something the list
/// does not name — a manual range outside the family's ladder, say.
const UNKNOWN_LIVE: &str = "?";

/// How a setting is named in a listing header and in the lines `set` prints:
/// Title case, or the capitals the meter's own buttons carry.
fn setting_title(setting: Setting) -> &'static str {
    match setting {
        Setting::Mode => "Mode",
        Setting::Range => "Range",
        Setting::Hold => "HOLD",
        Setting::Rel => "REL",
        Setting::MinMax => "MIN/MAX",
        Setting::Peak => "Peak",
    }
}

/// The same name where the sentence wants a plural ("has no switchable
/// modes"). The button settings read as themselves.
fn setting_plural(setting: Setting) -> &'static str {
    match setting {
        Setting::Mode => "modes",
        Setting::Range => "ranges",
        _ => setting_title(setting),
    }
}

/// What a user can do about a value the meter won't take. A range is refused
/// for one reason the dial does not cover: the reading is off the end of it.
fn switch_hint(setting: Setting) -> &'static str {
    match setting {
        Setting::Range => "check the dial position and that the input is within the range",
        _ => CHECK_DIAL_HINT,
    }
}

/// Whether the meter has a value to switch *to* from where it sits.
///
/// A single choice is the live value on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to list, and nothing to switch.
fn offers_a_switch(choices: &[Choice]) -> bool {
    choices.len() > 1
}

/// The value the meter sits on, or [`UNKNOWN_LIVE`] when the list marks none.
fn live_label(choices: &[Choice]) -> &str {
    choices
        .iter()
        .find(|c| c.current)
        .map_or(UNKNOWN_LIVE, |c| c.label.as_ref())
}

/// The rung autoranging picked, for the note beside an Auto row — "Auto" on
/// its own never says what the meter settled on. `None` unless this is the
/// range list and the meter is autoranging.
fn auto_range_rung<'a>(
    setting: Setting,
    choices: &[Choice],
    reading: &'a dmm_lib::measurement::Measurement,
) -> Option<&'a str> {
    (setting == Setting::Range && choices.iter().any(|c| c.current && c.id == AUTO_RANGE_ID))
        .then(|| reading.range_label.as_ref())
}

/// The note a setting — or, with `None`, the whole meter — with nothing to
/// switch prints before exiting 0. Only the dial changes what modes and
/// ranges a position offers; a button setting the meter lacks is not
/// something the dial can fix, so it gets no such advice.
fn print_nothing_to_switch(model_name: &str, setting: Option<Setting>, mode: &str) {
    let what = setting.map_or("settings", setting_plural);
    let advice = match setting {
        None | Some(Setting::Mode | Setting::Range) => " \u{2014} use the dial",
        Some(_) => "",
    };
    eprintln!(
        "{} {model_name} has no switchable {what} in {mode}{advice}.",
        style("Note:").yellow(),
    );
}

/// The dim line under a listing. Both listings end in one, so both name a
/// switch that can actually be made — and preferably one that would change
/// something.
fn print_tip(lead: &str, setting: Setting, choices: &[Choice]) {
    let example = choices.iter().find(|c| !c.current).unwrap_or(&choices[0]);
    eprintln!(
        "\n{}",
        style(format!(
            "Tip: {lead}, e.g. dmm-cli set {} {}",
            setting.name(),
            quote_for_shell(&shortest_fragment(choices, example))
        ))
        .dim()
    );
}

/// List what one setting, or every setting, reaches from where the meter sits.
///
/// Both need a reading first: the choices are relative to what the meter is
/// measuring now, so there is nothing to list until one frame has arrived.
fn cmd_get(
    selection: Selection,
    adapter: Option<&str>,
    setting: Option<SettingArg>,
    format: SettingsFormat,
    mock_mode: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let setting = setting.map(Setting::from);
    if requires_hardware(selection) {
        let (mut dmm, _device) = open_with_help(selection, adapter)?;
        run_get(&mut dmm, setting, format)
    } else {
        let mut dmm = open_mock_device(mock_mode, dmm_lib::Clock::real())?;
        run_get(&mut dmm, setting, format)
    }
}

/// Switch one setting, or list what it reaches when no choice was named.
fn cmd_set(
    selection: Selection,
    adapter: Option<&str>,
    setting: SettingArg,
    choice: Option<String>,
    mock_mode: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let setting = Setting::from(setting);
    if requires_hardware(selection) {
        let (mut dmm, _device) = open_with_help(selection, adapter)?;
        run_set(&mut dmm, setting, choice)
    } else {
        let mut dmm = open_mock_device(mock_mode, dmm_lib::Clock::real())?;
        run_set(&mut dmm, setting, choice)
    }
}

fn run_get<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    setting: Option<Setting>,
    format: SettingsFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = dmm.profile().model_name;
    let reading = dmm.request_measurement()?;

    let Some(setting) = setting else {
        let offered = offered_settings(dmm, &reading);
        return match format {
            SettingsFormat::Json => print_json(settings_json(model_name, &reading, &offered)),
            SettingsFormat::Text => {
                if offered.is_empty() {
                    print_nothing_to_switch(model_name, None, &reading.mode);
                    return Ok(());
                }
                print_settings_table(model_name, &reading, &offered);
                let (setting, choices) = offered
                    .iter()
                    .find(|(_, choices)| choices.iter().any(|c| !c.current))
                    .unwrap_or(&offered[0]);
                print_tip("switch one by name", *setting, choices);
                Ok(())
            }
        };
    };

    let choices = dmm.choices(setting, &reading);
    match format {
        SettingsFormat::Json => {
            print_json(one_setting_json(model_name, &reading, setting, &choices))
        }
        SettingsFormat::Text => {
            if !offers_a_switch(&choices) {
                print_nothing_to_switch(model_name, Some(setting), &reading.mode);
                return Ok(());
            }
            print_choices(model_name, setting, &reading, &choices);
            print_tip("the right column switches to it", setting, &choices);
            Ok(())
        }
    }
}

fn run_set<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    setting: Setting,
    choice: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = dmm.profile().model_name;
    let reading = dmm.request_measurement()?;
    let choices = dmm.choices(setting, &reading);

    if !offers_a_switch(&choices) {
        print_nothing_to_switch(model_name, Some(setting), &reading.mode);
        return Ok(());
    }

    let Some(input) = choice else {
        print_choices(model_name, setting, &reading, &choices);
        print_tip("the right column switches to it", setting, &choices);
        return Ok(());
    };

    let target = match resolve_choice(&choices, &input) {
        Ok(target) => target,
        Err(NoMatch::Ambiguous(labels)) => {
            return Err(
                format!("ambiguous {setting}: {input} matches {}", labels.join(", ")).into(),
            );
        }
        Err(NoMatch::Unknown) => {
            print_choices(model_name, setting, &reading, &choices);
            return Err(format!("unknown {setting}: {input}").into());
        }
    };
    let (id, label) = (target.id, target.label.to_string());

    if target.current {
        println!(
            "{}",
            style(switch_message(
                false,
                setting,
                id,
                &label,
                &reading.range_label
            ))
            .green()
        );
        return Ok(());
    }
    // Everything below reads from `choices` again, so drop the borrow.
    let mut live = choices
        .iter()
        .find(|c| c.current)
        .map(|c| c.label.to_string());

    if let Err(e) = dmm.select(setting, id) {
        // A refusal is the meter answering, not a fault: say so, and say what
        // the user can do about it.
        return Err(match e {
            dmm_lib::error::Error::CommandRejected(detail) => format!(
                "the meter refused {label}: {detail} \u{2014} {}",
                switch_hint(setting)
            )
            .into(),
            other => Box::<dyn std::error::Error>::from(other),
        });
    }

    let started = std::time::Instant::now();
    loop {
        match dmm.request_measurement() {
            Ok(reading) => {
                let choices = dmm.choices(setting, &reading);
                if choices.iter().any(|c| c.id == id && c.current) {
                    println!(
                        "{}",
                        style(switch_message(
                            true,
                            setting,
                            id,
                            &label,
                            &reading.range_label
                        ))
                        .green()
                    );
                    return Ok(());
                }
                if let Some(c) = choices.iter().find(|c| c.current) {
                    live = Some(c.label.to_string());
                }
            }
            // The frame straddling the switch can be unreadable, and the meter
            // can go quiet across it altogether — the vendor app sleeps 100 ms
            // after every SET_MODE. Neither ends the wait: the next frame
            // parses, and the 2 s deadline below is what gives up. Anything
            // else is a real fault.
            Err(e) if matches!(e.kind(), ErrorKind::Protocol | ErrorKind::Timeout) => {
                log::warn!("waiting for the {setting} switch: {e}");
            }
            Err(e) => return Err(e.into()),
        }
        if std::time::Instant::now()
            .checked_duration_since(started)
            .unwrap_or_default()
            >= SWITCH_TIMEOUT
        {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    let still = live.map_or_else(String::new, |l| format!(" (still {l})"));
    Err(format!(
        "Meter did not switch{still} \u{2014} {}",
        switch_hint(setting)
    )
    .into())
}

/// What `set` prints once the meter is on `label`, in that setting's own
/// words. `now` picks between the line a switch prints and the one a meter
/// that was already there prints, so the pair cannot drift apart.
fn switch_message(now: bool, setting: Setting, id: u16, label: &str, range_label: &str) -> String {
    let lead = if now { "Meter now" } else { "Meter is already" };
    match setting {
        Setting::Mode => format!("{lead} in {label}"),
        // "Auto" alone leaves the user guessing which rung that is.
        Setting::Range if id == AUTO_RANGE_ID => format!("{lead} auto-ranging ({range_label})"),
        Setting::Range => format!("{lead} in {label} (manual range)"),
        Setting::Hold | Setting::Rel => format!("{lead} {} {label}", setting_title(setting)),
        // Off is not a state the meter is "in", it is one it has left.
        Setting::MinMax | Setting::Peak if id == 0 => {
            format!("{lead} out of {}", setting_title(setting))
        }
        Setting::MinMax | Setting::Peak => format!("{lead} in {label}"),
    }
}

/// The listing a single setting gets: a header naming the meter and, for
/// everything but the mode itself, what it is measuring.
fn choices_listing(
    model_name: &str,
    setting: Setting,
    reading: &dmm_lib::measurement::Measurement,
    choices: &[Choice],
) -> Vec<String> {
    let header = match setting {
        Setting::Mode => format!("Modes for {}:", style(model_name).bold()),
        _ => format!(
            "{} for {} in {}:",
            setting_title(setting),
            style(model_name).bold(),
            reading.mode
        ),
    };
    let rung = auto_range_rung(setting, choices, reading);
    let width = choices
        .iter()
        .map(|c| c.label.chars().count())
        .max()
        .unwrap_or(0);
    let rows = choices.iter().map(|c| {
        // Pad the bare label: styling it first would count escape bytes
        // toward the width and misalign the column.
        let pad = " ".repeat(width - c.label.chars().count());
        let note = match rung {
            Some(rung) if c.id == AUTO_RANGE_ID => format!("  (now {rung})"),
            _ => String::new(),
        };
        format!(
            "{} {}{pad}  {}{}",
            if c.current {
                style("*").green().bold()
            } else {
                style(" ")
            },
            c.label,
            style(quote_for_shell(&shortest_fragment(choices, c))).dim(),
            style(note).dim(),
        )
    });
    std::iter::once(header).chain(rows).collect()
}

/// One line per choice, `*` on the live one, and the least that has to be
/// typed to reach it in a second column — so the fragment form is on screen
/// rather than something to guess at.
fn print_choices(
    model_name: &str,
    setting: Setting,
    reading: &dmm_lib::measurement::Measurement,
    choices: &[Choice],
) {
    for line in choices_listing(model_name, setting, reading, choices) {
        println!("{line}");
    }
}

/// The whole-meter listing: one row per setting, its name, the live value
/// behind a `*`, then what else it reaches. The labels themselves stand in
/// for the per-setting listing's fragment column.
fn settings_listing(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) -> Vec<String> {
    let header = format!(
        "Settings for {} ({}):",
        style(model_name).bold(),
        reading.mode
    );
    let name_width = offered
        .iter()
        .map(|(s, _)| s.name().chars().count())
        .max()
        .unwrap_or(0);
    let live_width = offered
        .iter()
        .map(|(_, choices)| live_label(choices).chars().count())
        .max()
        .unwrap_or(0);
    let rows = offered.iter().map(|(setting, choices)| {
        let live = live_label(choices);
        let others: Vec<&str> = choices
            .iter()
            .filter(|c| !c.current)
            .map(|c| c.label.as_ref())
            .collect();
        let note = match auto_range_rung(*setting, choices, reading) {
            Some(rung) => format!("  (auto-ranging in {rung})"),
            None => String::new(),
        };
        format!(
            "  {:name_width$}  {} {}{}  {}{}",
            setting.name(),
            if choices.iter().any(|c| c.current) {
                style("*").green().bold()
            } else {
                style(" ")
            },
            live,
            " ".repeat(live_width - live.chars().count()),
            others.join("  "),
            style(note).dim(),
        )
    });
    std::iter::once(header).chain(rows).collect()
}

fn print_settings_table(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) {
    for line in settings_listing(model_name, reading, offered) {
        println!("{line}");
    }
}

/// Every setting the meter offers a real choice in, in [`Setting::ALL`]
/// order. One that offers nothing — an unimplemented Peak, a dial with a
/// single mode — is simply absent, from both the table and the JSON.
fn offered_settings<T: dmm_lib::transport::Transport>(
    dmm: &dmm_lib::Dmm<T>,
    reading: &dmm_lib::measurement::Measurement,
) -> Vec<(Setting, Vec<Choice>)> {
    Setting::ALL
        .iter()
        .map(|&s| (s, dmm.choices(s, reading)))
        .filter(|(_, choices)| offers_a_switch(choices))
        .collect()
}

/// The three fields every `get --format json` object leads with: which meter
/// answered, and what it was measuring when it did.
fn json_header(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
) -> serde_json::Map<String, serde_json::Value> {
    let mut header = serde_json::Map::new();
    header.insert("device".into(), model_name.into());
    header.insert("mode".into(), reading.mode.as_ref().into());
    header.insert("range".into(), reading.range_label.as_ref().into());
    header
}

/// One setting's block, used flat for `get <SETTING>` and as an element of
/// the `settings` array for `get`.
fn setting_json(setting: Setting, choices: &[Choice]) -> serde_json::Value {
    serde_json::json!({
        "setting": setting.name(),
        "current": choices.iter().find(|c| c.current).map(|c| c.label.as_ref()),
        "choices": choices
            .iter()
            .map(|c| serde_json::json!({"label": c.label, "current": c.current}))
            .collect::<Vec<_>>(),
    })
}

/// What `get <SETTING> --format json` prints. Flat, not nested: the query
/// asked about one setting, so its fields sit beside the header's.
fn one_setting_json(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    setting: Setting,
    choices: &[Choice],
) -> serde_json::Map<String, serde_json::Value> {
    let mut doc = json_header(model_name, reading);
    if let serde_json::Value::Object(fields) = setting_json(setting, choices) {
        doc.extend(fields);
    }
    doc
}

/// What `get --format json` prints: the header, then a block per setting the
/// meter offers a choice in.
fn settings_json(
    model_name: &str,
    reading: &dmm_lib::measurement::Measurement,
    offered: &[(Setting, Vec<Choice>)],
) -> serde_json::Map<String, serde_json::Value> {
    let mut doc = json_header(model_name, reading);
    doc.insert(
        "settings".into(),
        offered
            .iter()
            .map(|(s, choices)| setting_json(*s, choices))
            .collect(),
    );
    doc
}

/// One object per invocation, and nothing else on stdout — `get --format json`
/// answers a question rather than streaming, unlike `read`.
fn print_json(
    doc: serde_json::Map<String, serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(doc))?
    );
    Ok(())
}
/// The shortest run of words from a choice's label that [`resolve_choice`]
/// maps back to that same choice — what the listing shows as the thing to type.
///
/// Runs are tried shortest first, measured in characters and, at equal length,
/// leftmost first. A label whose every fragment is shared with a longer label
/// ("V AC" beside "V AC Hz") has no shorter form: the whole label comes back,
/// which the resolver takes as an exact match.
fn shortest_fragment(choices: &[Choice], target: &Choice) -> String {
    let label = typeable(&target.label);
    let words: Vec<&str> = label.split_whitespace().collect();
    let mut runs: Vec<(usize, usize, String)> = Vec::new();
    for len in 1..=words.len() {
        for start in 0..=words.len() - len {
            let run = words[start..start + len].join(" ");
            runs.push((run.chars().count(), start, run));
        }
    }
    runs.sort_by_key(|(chars, start, _)| (*chars, *start));
    runs.into_iter()
        .map(|(_, _, run)| run)
        .find(|run| resolve_choice(choices, run).is_ok_and(|hit| hit.id == target.id))
        // Only reachable for a label the runs above cannot reproduce (empty,
        // or oddly spaced); the label itself is always an exact match.
        .unwrap_or(label)
}

/// A label as it can be typed on any keyboard: lower-case, with the symbols
/// the meters use spelled out (Ω → ohm, µ → u) or dropped (°). Both sides of
/// a match go through this, so "ohm" finds "Ω" and "temp c" finds "Temp °C".
fn typeable(label: &str) -> String {
    label
        .to_lowercase()
        .chars()
        .filter(|&c| c != '°')
        .map(|c| match c {
            // Both U+03A9 (Greek omega) and U+2126 (ohm sign) lower-case to ω.
            'ω' => "ohm".to_string(),
            'µ' | 'μ' => "u".to_string(),
            other => other.to_string(),
        })
        .collect()
}

/// A label or fragment as it has to be typed back on a shell command line:
/// bare when the shell would leave it alone, double-quoted otherwise.
fn quote_for_shell(text: &str) -> String {
    let bare = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if bare {
        text.to_string()
    } else {
        format!("\"{text}\"")
    }
}

/// Why a `mode` argument picked out no single choice.
#[derive(Debug)]
enum NoMatch<'a> {
    /// The input is a fragment of these labels, and equal to none of them.
    Ambiguous(Vec<&'a str>),
    /// The input is a fragment of no label at all.
    Unknown,
}

/// Resolve a `mode` argument against the choices the meter just reported: a
/// label, case-insensitively, or a fragment of exactly one of them.
///
/// An exact match wins outright — a label that is also a substring of longer
/// ones ("V AC" beside "V AC Hz") stays reachable by typing it in full.
fn resolve_choice<'a>(choices: &'a [Choice], input: &str) -> Result<&'a Choice, NoMatch<'a>> {
    let needle = typeable(input.trim());
    if needle.is_empty() {
        return Err(NoMatch::Unknown);
    }
    if let Some(exact) = choices.iter().find(|c| typeable(&c.label) == needle) {
        return Ok(exact);
    }
    let hits: Vec<_> = choices
        .iter()
        .filter(|c| typeable(&c.label).contains(&needle))
        .collect();
    match hits[..] {
        [one] => Ok(one),
        [] => Err(NoMatch::Unknown),
        _ => Err(NoMatch::Ambiguous(
            hits.iter().map(|c| c.label.as_ref()).collect(),
        )),
    }
}

fn cmd_debug(
    selection: Selection,
    adapter: Option<&str>,
    count: usize,
    interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;

    let (mut dmm, _device) = open_with_help(selection, adapter)?;

    // Show transport info before entering measurement loop
    eprintln!(
        "{} {}",
        style("transport:").dim(),
        dmm.transport().transport_name()
    );
    if let Ok(info) = dmm.transport().transport_info() {
        eprintln!("{} {info}", style("bridge:").dim());
    }
    if let Ok(status) = dmm.transport().transport_status() {
        eprintln!("{} {status}", style("status:").dim());
    }

    let tick = Duration::from_millis(interval_ms);
    let mut i = 0;
    let cancel = running.clone();
    let mut stream =
        MeasurementStream::new(&mut dmm, tick).with_cancel(move || !cancel.load(Ordering::SeqCst));

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match stream.tick() {
            Ok(StreamEvent::Measurement(m)) => {
                let display = m.display_raw.as_deref().unwrap_or("(none)");
                println!(
                    "{} mode_raw={:04X} display={:?} progress={:?} flags={} raw={:02X?} \u{2192} {}",
                    style(format!("[{i}]")).dim(),
                    m.mode_raw,
                    display,
                    m.progress,
                    m.flags,
                    m.raw_payload,
                    style(format!("{m}")).green(),
                );
                // The secondary displays a UT181A or UT171 sends alongside
                // the reading; nothing else in the debug line shows them.
                if !m.aux_values.is_empty() {
                    println!("    {} {}", style("sub-values:").dim(), m.aux_summary());
                }
            }
            Ok(StreamEvent::Timeout { .. }) => {
                eprintln!(
                    "{} {}",
                    style(format!("[{i}]")).dim(),
                    style("error: timeout").red()
                );
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    style(format!("[{i}]")).dim(),
                    style(format!("error: {e}")).red()
                );
            }
        }
        i += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::measurement::MeasuredValue;
    use dmm_lib::protocol::ut61eplus::make_test_measurement;

    #[test]
    fn clap_parse_list() {
        let cli = Cli::try_parse_from(["dmm-cli", "list"]).unwrap();
        assert!(matches!(cli.command, Cmd::List));
    }

    #[test]
    fn clap_parse_read_defaults() {
        let cli = Cli::try_parse_from(["dmm-cli", "read"]).unwrap();
        match cli.command {
            Cmd::Read {
                interval_ms,
                format,
                output,
                count,
                integrate,
                transform,
                mock_mode,
                replay,
                mock_clock_scale,
                mock_clock_preseed,
            } => {
                assert_eq!(interval_ms, 0);
                // Neither given, so the run prints text on stdout.
                assert!(format.is_none());
                assert!(output.is_none());
                assert_eq!(count, 0);
                assert!(!integrate);
                // No transform flags means the identity, so `read` keeps the
                // reading and the column layout it always had.
                assert_eq!(transform.to_transform(), Transform::default());
                assert!(transform.to_transform().is_identity());
                assert!(mock_mode.is_none());
                // Nothing to play back: `read` opens the meter.
                assert!(replay.is_none());
                // No flag means the wall clock, so `read` paces as it always did.
                assert!(mock_clock_scale.is_none());
                assert!(mock_clock_preseed.is_none());
            }
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_read_replay() {
        let playback = Cli::try_parse_from(["dmm-cli", "read", "--replay", "bench.replay"])
            .expect("--replay parses");
        match playback.command {
            Cmd::Read { replay, .. } => {
                assert_eq!(replay.as_deref(), Some(Path::new("bench.replay")));
            }
            _ => panic!("expected Read"),
        }
    }

    /// `read` has no positional argument, so `-o` can take its file name or
    /// leave it to the run — and the flag after a bare one is still a flag.
    #[test]
    fn clap_parse_read_output_with_and_without_a_name() {
        let output_of = |args: &[&str]| {
            let cli = Cli::try_parse_from(["dmm-cli", "read"].iter().chain(args).copied())
                .expect("-o parses");
            match cli.command {
                Cmd::Read { output, count, .. } => (output, count),
                _ => panic!("expected Read"),
            }
        };
        assert_eq!(
            output_of(&["-o", "bench.csv"]),
            (Some(Some("bench.csv".to_string())), 0)
        );
        assert_eq!(output_of(&["-o", "--count", "3"]), (Some(None), 3));
        assert_eq!(output_of(&["--count", "3", "-o"]), (Some(None), 3));
    }

    /// A run gives frames back or takes them from the meter, never both.
    #[test]
    fn clap_refuses_replay_with_mock_mode() {
        assert!(
            Cli::try_parse_from(["dmm-cli", "read", "--replay", "b", "--mock-mode", "dcv"])
                .is_err()
        );
    }

    /// The flag a recording used to be written with; it is `--format replay`
    /// now, and nothing should quietly accept the old spelling.
    #[test]
    fn clap_refuses_the_old_record_flag() {
        assert!(Cli::try_parse_from(["dmm-cli", "read", "--record", "bench.replay"]).is_err());
    }

    /// Hidden, but they still have to parse — nothing in `--help` would catch
    /// a rename, and the screenshot and perf runs depend on both.
    #[test]
    fn clap_parse_read_clock_flags() {
        let cli = Cli::try_parse_from([
            "dmm-cli",
            "read",
            "--mock-clock-scale",
            "20",
            "--mock-clock-preseed",
            "90",
        ])
        .unwrap();
        match cli.command {
            Cmd::Read {
                mock_clock_scale,
                mock_clock_preseed,
                ..
            } => {
                assert_eq!(mock_clock_scale, Some(20.0));
                assert_eq!(mock_clock_preseed, Some(90.0));
            }
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_read_with_args() {
        let cli = Cli::try_parse_from([
            "dmm-cli",
            "read",
            "--interval-ms",
            "100",
            "--format",
            "csv",
            "-o",
            "test.csv",
            "--count",
            "10",
        ])
        .unwrap();
        match cli.command {
            Cmd::Read {
                interval_ms,
                format,
                output,
                count,
                mock_mode: _,
                integrate: _,
                transform: _,
                replay: _,
                mock_clock_scale: _,
                mock_clock_preseed: _,
            } => {
                assert_eq!(interval_ms, 100);
                assert_eq!(format, Some(OutputFormat::Csv));
                assert_eq!(output, Some(Some("test.csv".to_string())));
                assert_eq!(count, 10);
            }
            _ => panic!("expected Read"),
        }
    }

    /// A device string, resolved as `main` resolves it.
    fn selection(s: &str) -> Selection {
        registry::resolve_selection(s).unwrap_or_else(|| panic!("{s} must resolve"))
    }

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
            assert_eq!(err.to_string(), dmm_lib::binary_help::MOCK_CLOCK_MOCK_ONLY);
            assert!(refuse_clock_on_hardware(hardware, &dmm_lib::Clock::real()).is_ok());
        }

        assert!(refuse_clock_on_hardware(mock, &virtual_clock).is_ok());
    }

    /// Naming no meter, on the command line or in the settings file, means
    /// detection rather than a model nobody chose.
    #[test]
    fn no_flag_and_no_setting_detects_the_meter() {
        let (id, source) =
            dmm_settings::resolve_device_family(None, None, registry::AUTO_DEVICE_ID);
        assert_eq!(source, dmm_settings::DeviceSource::Fallback);
        assert!(matches!(
            registry::resolve_selection(&id),
            Some(Selection::Auto)
        ));
        assert!(requires_hardware(selection("auto")));
    }

    /// A saved or flagged family still pins one, and skips detection.
    #[test]
    fn a_named_family_still_wins() {
        let saved = dmm_settings::SharedSettings {
            device_family: "ut8803".to_string(),
        };
        let (id, source) =
            dmm_settings::resolve_device_family(None, Some(&saved), registry::AUTO_DEVICE_ID);
        assert_eq!(source, dmm_settings::DeviceSource::Settings);
        let Some(Selection::Device(device)) = registry::resolve_selection(&id) else {
            panic!("a saved family must resolve to that device");
        };
        assert_eq!(device.id, "ut8803");

        let (id, source) = dmm_settings::resolve_device_family(
            Some("ut61b+"),
            Some(&saved),
            registry::AUTO_DEVICE_ID,
        );
        assert_eq!(source, dmm_settings::DeviceSource::Cli);
        assert_eq!(selection_id(selection(&id)), "ut61b+");
    }

    /// `auto` is a value `--device` takes, and the only one the registry does
    /// not carry — so nothing else would put it in the help.
    #[test]
    fn the_device_help_offers_auto() {
        let help = build_device_help();
        assert!(
            help.contains("auto         Detect the meter over the USB cable (default)"),
            "{help}"
        );
        assert!(
            build_after_long_help()
                .contains("3. auto \u{2014} detect the meter over the USB cable"),
            "the precedence list still names a model as the fallback"
        );
    }

    /// The "no meter answered" help lists what to switch on, once per set of
    /// steps: every UT61+/UT161 model shares one block, and so do the UT8802
    /// and the UT8803.
    #[test]
    fn activation_help_lists_each_set_of_steps_once() {
        let devices = dmm_lib::devices_on_bridge("CP2110");
        assert!(devices.len() > 1, "CP2110 carries several meters");
        let groups = activation_groups(&devices);
        assert!(groups.len() < devices.len(), "nothing was grouped");

        let instructions: Vec<&str> = groups.iter().map(|(i, _)| *i).collect();
        let mut unique = instructions.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), instructions.len(), "a block is listed twice");

        // Every meter is named exactly once, under its own block.
        let named: Vec<&str> = groups.iter().flat_map(|(_, names)| names.clone()).collect();
        assert_eq!(named.len(), devices.len());
        let ut61 = groups
            .iter()
            .find(|(_, names)| names.contains(&"UT61E+"))
            .expect("the UT61E+ is on the CP2110");
        assert!(ut61.1.contains(&"UT61B+"), "{:?}", ut61.1);
        assert!(ut61.0.contains("USB/Hz"), "{}", ut61.0);
    }

    fn read_transform(args: &[&str]) -> Transform {
        let mut argv = vec!["dmm-cli", "read"];
        argv.extend_from_slice(args);
        match Cli::try_parse_from(argv).unwrap().command {
            Cmd::Read { transform, .. } => transform.to_transform(),
            _ => panic!("expected Read"),
        }
    }

    /// `--unit` takes exactly one value, so a `--format` after it must still
    /// be parsed as a flag rather than swallowed as part of the label.
    #[test]
    fn clap_parse_read_scale_and_unit() {
        let cli = Cli::try_parse_from([
            "dmm-cli", "read", "--scale", "100", "--unit", "A", "--format", "csv",
        ])
        .unwrap();
        match cli.command {
            Cmd::Read {
                transform, format, ..
            } => {
                assert_eq!(
                    transform.to_transform(),
                    Transform::linear(100.0, 0.0, Some("A".to_string()))
                );
                assert_eq!(format, Some(OutputFormat::Csv));
            }
            _ => panic!("expected Read"),
        }
    }

    /// Without `allow_negative_numbers` clap reads `-3` as an unknown flag.
    #[test]
    fn clap_parse_read_accepts_negative_scale_and_offset() {
        assert_eq!(
            read_transform(&["--offset", "-3"]),
            Transform::linear(1.0, -3.0, None)
        );
        assert_eq!(
            read_transform(&["--scale", "-1"]),
            Transform::linear(-1.0, 0.0, None)
        );
    }

    #[test]
    fn clap_parse_read_celsius_to_fahrenheit() {
        assert_eq!(
            read_transform(&["--scale", "1.8", "--offset", "32", "--unit", "°F"]),
            Transform::linear(1.8, 32.0, Some("°F".to_string()))
        );
    }

    /// `Cli` is not `Debug`, so the rejection has to be matched rather than
    /// unwrapped.
    fn flag_error(flag: &str, value: &str) -> String {
        match Cli::try_parse_from(["dmm-cli", "read", flag, value]) {
            Ok(_) => panic!("{flag} {value} should have been rejected"),
            Err(e) => e.to_string(),
        }
    }

    /// A zero or non-finite factor would destroy the reading rather than
    /// re-express it, and would poison the stats and the integral with NaN.
    #[test]
    fn clap_rejects_a_zero_or_non_finite_scale() {
        for bad in ["0", "-0", "nan", "inf"] {
            let msg = flag_error("--scale", bad);
            assert!(msg.contains("scale must"), "--scale {bad} said: {msg}");
        }
        let msg = flag_error("--scale", "abc");
        assert!(msg.contains("is not a number"), "got {msg}");
    }

    /// `--offset` had no parser at all, so NaN and infinity went straight
    /// through into every reading and only surfaced as `NaN` in the closing
    /// summary. Any *finite* shift stays legal, zero and negatives included.
    #[test]
    fn clap_rejects_a_non_finite_offset() {
        for bad in ["nan", "inf"] {
            let msg = flag_error("--offset", bad);
            assert!(msg.contains("offset must"), "--offset {bad} said: {msg}");
        }
        let msg = flag_error("--offset", "abc");
        assert!(msg.contains("is not a number"), "got {msg}");
        assert_eq!(
            read_transform(&["--offset", "-3"]),
            Transform::linear(1.0, -3.0, None)
        );
        assert_eq!(
            read_transform(&["--offset", "0"]),
            Transform::linear(1.0, 0.0, None)
        );
    }

    #[test]
    fn clap_parse_command() {
        let cli = Cli::try_parse_from(["dmm-cli", "command", "hold"]).unwrap();
        match cli.command {
            Cmd::Command { action } => {
                assert_eq!(action.as_deref(), Some("hold"));
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn clap_parse_command_no_action_lists_commands() {
        let cli = Cli::try_parse_from(["dmm-cli", "command"]).unwrap();
        match cli.command {
            Cmd::Command { action } => {
                assert!(action.is_none());
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn clap_parse_set() {
        let cli = Cli::try_parse_from(["dmm-cli", "set", "mode", "V AC Hz"]).unwrap();
        match cli.command {
            Cmd::Set {
                setting, choice, ..
            } => {
                assert_eq!(setting, SettingArg::Mode);
                assert_eq!(choice.as_deref(), Some("V AC Hz"));
            }
            _ => panic!("expected Set"),
        }
    }

    #[test]
    fn clap_parse_set_no_choice_lists_them() {
        let cli = Cli::try_parse_from(["dmm-cli", "set", "minmax"]).unwrap();
        match cli.command {
            Cmd::Set {
                setting,
                choice,
                mock_mode,
            } => {
                assert_eq!(setting, SettingArg::Minmax);
                assert!(choice.is_none());
                assert!(mock_mode.is_none());
            }
            _ => panic!("expected Set"),
        }
    }

    /// Every setting is nameable, and every name maps to the library's own.
    #[test]
    fn clap_parse_get_takes_each_setting_name() {
        for (word, setting) in [
            ("mode", Setting::Mode),
            ("range", Setting::Range),
            ("hold", Setting::Hold),
            ("rel", Setting::Rel),
            ("minmax", Setting::MinMax),
            ("peak", Setting::Peak),
        ] {
            let cli = Cli::try_parse_from(["dmm-cli", "get", word]).unwrap();
            match cli.command {
                Cmd::Get {
                    setting: Some(arg),
                    format,
                    ..
                } => {
                    assert_eq!(Setting::from(arg), setting, "{word}");
                    assert_eq!(format, SettingsFormat::Text, "{word}");
                }
                _ => panic!("expected Get {word}"),
            }
            assert_eq!(setting.name(), word);
        }
    }

    /// A settings listing has no CSV form, so `--format csv` is rejected
    /// rather than silently printing text.
    #[test]
    fn clap_parse_get_takes_only_text_and_json() {
        assert!(Cli::try_parse_from(["dmm-cli", "get", "--format", "json"]).is_ok());
        assert!(Cli::try_parse_from(["dmm-cli", "get", "--format", "csv"]).is_err());
    }

    #[test]
    fn clap_parse_get_no_setting_lists_everything() {
        let cli = Cli::try_parse_from(["dmm-cli", "get"]).unwrap();
        match cli.command {
            Cmd::Get { setting, .. } => assert!(setting.is_none()),
            _ => panic!("expected Get"),
        }
    }

    fn mode_choice(id: u16, label: &'static str, current: bool) -> Choice {
        Choice {
            id,
            label: std::borrow::Cow::Borrowed(label),
            current,
        }
    }

    /// A user retypes what the listing printed, in whatever case they like.
    #[test]
    fn resolve_choice_matches_a_label_case_insensitively() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ];
        for input in ["V AC Hz", "v ac hz", "  V Ac hZ  "] {
            assert_eq!(
                resolve_choice(&choices, input).ok().map(|c| c.id),
                Some(0x1121),
                "{input}"
            );
        }
    }

    /// The symbols the meters print are not on a keyboard, so their spelled
    /// out forms match too.
    #[test]
    fn resolve_choice_accepts_typeable_spellings() {
        let choices = [
            mode_choice(0x06, "Ω", true),
            mode_choice(0x0C, "DC µA", false),
            mode_choice(0x14, "°C", false),
        ];
        for (input, id) in [("ohm", 0x06), ("Ω", 0x06), ("dc ua", 0x0C), ("c", 0x14)] {
            assert_eq!(
                resolve_choice(&choices, input).ok().map(|c| c.id),
                Some(id),
                "{input}"
            );
        }
    }

    /// Typing a whole label is tedious, so a fragment of exactly one of them
    /// is enough.
    #[test]
    fn resolve_choice_matches_a_unique_label_fragment() {
        let temps = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
        ];
        assert_eq!(
            resolve_choice(&temps, "t1-t2").ok().map(|c| c.id),
            Some(0x4231)
        );
        let volts = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_choice(&volts, "hz").ok().map(|c| c.id),
            Some(0x1121)
        );
    }

    /// A label that is also a fragment of longer ones stays reachable: typed
    /// in full it is an exact match, and an exact match wins outright.
    #[test]
    fn resolve_choice_prefers_an_exact_label_over_a_fragment() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_choice(&choices, "v ac").ok().map(|c| c.id),
            Some(0x1111)
        );
    }

    /// A fragment of several labels picks none of them, and says which ones
    /// it was torn between — that is what the user has to narrow down.
    #[test]
    fn resolve_choice_reports_an_ambiguous_fragment() {
        let choices = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
            mode_choice(0x4241, "Temp °C T2-T1", false),
        ];
        match resolve_choice(&choices, "temp") {
            Err(NoMatch::Ambiguous(labels)) => assert_eq!(
                labels,
                ["Temp °C", "Temp °C T2", "Temp °C T1-T2", "Temp °C T2-T1"]
            ),
            other => panic!("expected an ambiguity, got {other:?}"),
        }
    }

    /// Label groups a meter really offers, each with the fragment the listing
    /// is expected to print beside every one of its labels: the mock's
    /// temperature dial, the UT181A's V AC and temperature dials, and the
    /// mock's AC V dial.
    fn fragment_cases() -> [(&'static [&'static str], &'static [&'static str]); 5] {
        [
            (
                &[
                    "Temp °C",
                    "Temp °C T1 (T2)",
                    "Temp °C T1-T2",
                    "Temp °C T2-T1",
                ],
                &["temp c", "(t2)", "t1-t2", "t2-t1"],
            ),
            (
                &[
                    "V AC",
                    "V AC Hz",
                    "V AC Peak",
                    "V AC LPF",
                    "V AC dBV",
                    "V AC dBm",
                ],
                &["v ac", "hz", "peak", "lpf", "dbv", "dbm"],
            ),
            (
                &["°C", "°C T2", "°C T1-T2", "°C T2-T1"],
                &["c", "c t2", "t1-t2", "t2-t1"],
            ),
            (&["AC V", "AC V Hz"], &["ac v", "hz"]),
            (
                &["Ω", "Continuity", "Diode", "Capacitance"],
                &["ohm", "continuity", "diode", "capacitance"],
            ),
        ]
    }

    fn fragment_choices(labels: &[&'static str]) -> Vec<Choice> {
        labels
            .iter()
            .enumerate()
            .map(|(i, label)| mode_choice(fake_mode_id(i), label, i == 0))
            .collect()
    }

    /// What the second column of the listing says, group by group.
    #[test]
    fn shortest_fragment_is_the_least_that_picks_a_mode_out() {
        for (labels, expected) in fragment_cases() {
            let choices = fragment_choices(labels);
            let fragments: Vec<_> = choices
                .iter()
                .map(|c| shortest_fragment(&choices, c))
                .collect();
            assert_eq!(fragments, expected, "{labels:?}");
        }
    }

    /// The column is only useful if what it prints comes back to the line it
    /// is printed on — including for a label every fragment of which is
    /// shared, where the whole label is the answer.
    #[test]
    fn shortest_fragment_always_selects_its_own_mode() {
        // The groups above, plus the shorter lists the resolver tests use.
        let extra: [&[&str]; 3] = [
            &["V AC", "V AC Hz"],
            &["Temp °C", "Temp °C T2", "Temp °C T1-T2"],
            &["V AC"],
        ];
        for labels in fragment_cases().iter().map(|(l, _)| *l).chain(extra) {
            let choices = fragment_choices(labels);
            for c in &choices {
                let fragment = shortest_fragment(&choices, c);
                assert_eq!(
                    resolve_choice(&choices, &fragment).ok().map(|hit| hit.id),
                    Some(c.id),
                    "{fragment:?} for {}",
                    c.label
                );
            }
        }
    }

    /// A fragment is meant to be typed back, so anything a shell would split
    /// or mangle is printed quoted.
    #[test]
    fn quote_for_shell_quotes_what_a_shell_would_not_take_bare() {
        for bare in ["hz", "t1-t2", "dbv", "peak_2", "1.5"] {
            assert_eq!(quote_for_shell(bare), bare, "{bare}");
        }
        for quoted in ["ac v", "(t2)", "temp °c", "°c", "ac+dc", ""] {
            assert_eq!(quote_for_shell(quoted), format!("\"{quoted}\""), "{quoted}");
        }
    }

    /// Ids the fake meter below gives its choices, spaced like the UT181A's
    /// variant nibble so what `select` is asked for is realistic.
    fn fake_mode_id(index: usize) -> u16 {
        0x1111 + (index as u16) * 0x10
    }

    static FAKE_PROFILE: dmm_lib::protocol::DeviceProfile = dmm_lib::protocol::DeviceProfile {
        family_name: "Fake",
        model_name: "Fake meter",
        stability: dmm_lib::protocol::Stability::Experimental,
        supported_commands: &[],
        max_aux_values: 0,
        verification_issue: None,
    };

    /// The ids a family gives a setting's choices. Mode ids are the family's
    /// own, spaced like the UT181A's variant nibble; every other setting
    /// counts from zero, where zero is off or auto.
    fn fake_choice_id(setting: Setting, index: usize) -> u16 {
        match setting {
            Setting::Mode => fake_mode_id(index),
            _ => index as u16,
        }
    }

    /// One setting the fake meter offers: its labels, and which of them it
    /// sits on.
    type FakeList = (Setting, &'static [&'static str], usize);

    /// A meter offering exactly `lists`, each sitting where the list says.
    struct FakeMeter {
        lists: Vec<FakeList>,
        live: std::collections::HashMap<Setting, usize>,
        switched: bool,
        quirks: Quirks,
        /// What `select` was asked for, so a test can assert the meter was
        /// left alone.
        selected: SelectedIds,
    }

    /// How the fake meter misbehaves after a `select`. The default is a meter
    /// that simply works.
    #[derive(Default)]
    struct Quirks {
        /// Handed out in place of the readings that follow a successful
        /// switch — a garbled frame, or a meter gone quiet across it.
        post_switch_errors: Vec<dmm_lib::error::Error>,
        /// What every `select` answers instead of switching.
        refusal: Option<&'static str>,
        /// Takes the command and then never reports the new value.
        deaf: bool,
    }

    impl FakeMeter {
        fn labels(&self, setting: Setting) -> Option<&'static [&'static str]> {
            self.lists
                .iter()
                .find(|(s, _, _)| *s == setting)
                .map(|(_, labels, _)| *labels)
        }

        fn live_index(&self, setting: Setting) -> usize {
            self.live.get(&setting).copied().unwrap_or(0)
        }
    }

    impl dmm_lib::protocol::Protocol for FakeMeter {
        fn init(&mut self, _t: &dyn dmm_lib::transport::Transport) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn request_measurement(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<dmm_lib::measurement::Measurement> {
            if self.switched && !self.quirks.post_switch_errors.is_empty() {
                return Err(self.quirks.post_switch_errors.remove(0));
            }
            let mode_index = self.live_index(Setting::Mode);
            let mode = self
                .labels(Setting::Mode)
                .map_or("DC V", |labels| labels[mode_index]);
            // Autoranging settled on 22V; a manual rung reports itself.
            let range_index = self.live_index(Setting::Range);
            let range = match self.labels(Setting::Range) {
                Some(labels) if range_index != 0 => labels[range_index],
                _ => "22V",
            };
            Ok(dmm_lib::measurement::Measurement {
                mode: mode.into(),
                mode_raw: fake_mode_id(mode_index),
                range_label: range.into(),
                ..dmm_lib::measurement::Measurement::test_fixture(
                    MeasuredValue::Normal(1.0),
                    "V",
                    dmm_lib::flags::StatusFlags::default(),
                )
            })
        }

        fn parse_payload(
            &self,
            _payload: &[u8],
        ) -> dmm_lib::error::Result<dmm_lib::measurement::Measurement> {
            Err(dmm_lib::error::Error::UnsupportedCommand(
                "parse_payload: the fake meter has no wire format".to_string(),
            ))
        }

        fn send_command(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
            command: &str,
        ) -> dmm_lib::error::Result<()> {
            Err(dmm_lib::error::Error::UnsupportedCommand(
                command.to_string(),
            ))
        }

        fn get_name(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<Option<String>> {
            Ok(None)
        }

        fn profile(&self) -> &dmm_lib::protocol::DeviceProfile {
            &FAKE_PROFILE
        }

        fn choices(
            &self,
            setting: Setting,
            _current: &dmm_lib::measurement::Measurement,
        ) -> Vec<Choice> {
            let Some(labels) = self.labels(setting) else {
                return Vec::new();
            };
            let live = self.live_index(setting);
            labels
                .iter()
                .enumerate()
                .map(|(i, label)| Choice {
                    id: fake_choice_id(setting, i),
                    label: std::borrow::Cow::Borrowed(label),
                    current: i == live,
                })
                .collect()
        }

        fn select(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
            setting: Setting,
            id: u16,
        ) -> dmm_lib::error::Result<()> {
            self.selected.lock().expect("poisoned").push((setting, id));
            if let Some(detail) = self.quirks.refusal {
                return Err(dmm_lib::error::Error::CommandRejected(detail.to_string()));
            }
            let Some(len) = self.labels(setting).map(<[&str]>::len) else {
                return Err(dmm_lib::error::Error::UnsupportedCommand(format!(
                    "{setting} cannot be set on this meter"
                )));
            };
            match (0..len).find(|&i| fake_choice_id(setting, i) == id) {
                Some(i) => {
                    if !self.quirks.deaf {
                        self.live.insert(setting, i);
                    }
                    self.switched = true;
                    Ok(())
                }
                None => Err(dmm_lib::error::Error::UnsupportedCommand(format!(
                    "{setting} {id:#06x}"
                ))),
            }
        }
    }

    type SelectedIds = std::sync::Arc<std::sync::Mutex<Vec<(Setting, u16)>>>;
    type FakeDmm = dmm_lib::Dmm<dmm_lib::transport::NullTransport>;

    fn fake_meter(lists: &[FakeList]) -> (FakeDmm, SelectedIds) {
        fake_meter_with(lists, Quirks::default())
    }

    fn fake_meter_with(lists: &[FakeList], quirks: Quirks) -> (FakeDmm, SelectedIds) {
        let selected: SelectedIds = Default::default();
        let meter = FakeMeter {
            lists: lists.to_vec(),
            live: lists.iter().map(|&(s, _, live)| (s, live)).collect(),
            switched: false,
            quirks,
            selected: std::sync::Arc::clone(&selected),
        };
        let dmm = dmm_lib::Dmm::new(dmm_lib::transport::NullTransport, Box::new(meter))
            .expect("the fake meter needs no transport");
        (dmm, selected)
    }

    /// A meter on the V AC dial: two modes, four ranges, HOLD, and no Peak.
    fn a_full_meter() -> [FakeList; 4] {
        [
            (Setting::Mode, &["V AC", "V AC Hz"], 0),
            (Setting::Range, &["Auto", "2.2V", "22V", "220V"], 0),
            (Setting::Hold, &["off", "on"], 0),
            (Setting::MinMax, &["off", "max", "min"], 0),
        ]
    }

    /// The lines a listing is made of, with the styling stripped so the test
    /// reads the same whether or not colour is on.
    fn plain(lines: Vec<String>) -> Vec<String> {
        lines
            .into_iter()
            .map(|l| console::strip_ansi_codes(&l).into_owned())
            .collect()
    }

    /// A single-variant dial (the UT181A Ohm, nS, Cap, Hz, Duty and Pulse
    /// Width positions) reports one choice: the value the meter is already
    /// on. Listing it, and tipping the user to switch to it, is noise — it
    /// counts as nothing to switch, exactly as an empty list does.
    #[test]
    fn only_the_live_value_is_not_a_switch_to_offer() {
        assert!(!offers_a_switch(&[]));
        assert!(!offers_a_switch(&[mode_choice(0x5111, "Resistance", true)]));
        assert!(offers_a_switch(&[
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ]));
    }

    /// And both commands say so and stop: exit 0, meter untouched, whether or
    /// not a choice was asked for.
    #[test]
    fn a_setting_with_only_the_live_value_switches_nothing() {
        let lists: [FakeList; 1] = [(Setting::Mode, &["Resistance"], 0)];
        for arg in [None, Some("Resistance".to_string())] {
            let (mut dmm, selected) = fake_meter(&lists);
            run_set(&mut dmm, Setting::Mode, arg).expect("one choice is not a failure");
            assert!(
                selected.lock().expect("poisoned").is_empty(),
                "the meter was switched"
            );
        }
        let (mut dmm, selected) = fake_meter(&lists);
        run_get(&mut dmm, Some(Setting::Mode), SettingsFormat::Text).expect("nothing to list");
        run_get(&mut dmm, None, SettingsFormat::Text).expect("nothing to list");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// The whole-meter listing: one row per setting that offers a choice, the
    /// live value behind a `*`, and the rung autoranging picked.
    #[test]
    fn the_settings_table_lists_every_offered_setting() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let offered = offered_settings(&dmm, &reading);
        assert_eq!(
            offered.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            [
                Setting::Mode,
                Setting::Range,
                Setting::Hold,
                Setting::MinMax
            ],
            "Peak offers nothing, so it is absent"
        );
        let lines = plain(settings_listing("Fake meter", &reading, &offered));
        assert_eq!(lines[0], "Settings for Fake meter (V AC):");
        assert_eq!(
            &lines[1..],
            [
                "  mode    * V AC  V AC Hz",
                "  range   * Auto  2.2V  22V  220V  (auto-ranging in 22V)",
                "  hold    * off   on",
                "  minmax  * off   max  min",
            ]
        );
    }

    /// The per-setting listing keeps the mode header it always had, names the
    /// live mode for every other setting, and says which rung Auto picked.
    #[test]
    fn a_setting_listing_names_the_meter_and_the_live_mode() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");

        let modes = dmm.choices(Setting::Mode, &reading);
        let lines = plain(choices_listing(
            "Fake meter",
            Setting::Mode,
            &reading,
            &modes,
        ));
        assert_eq!(lines[0], "Modes for Fake meter:");
        assert_eq!(&lines[1..], ["* V AC     \"v ac\"", "  V AC Hz  hz"]);

        let ranges = dmm.choices(Setting::Range, &reading);
        let lines = plain(choices_listing(
            "Fake meter",
            Setting::Range,
            &reading,
            &ranges,
        ));
        assert_eq!(lines[0], "Range for Fake meter in V AC:");
        assert_eq!(lines[1], "* Auto  auto  (now 22V)");
    }

    /// `get <SETTING> --format json` is one flat object: which meter, what it
    /// is measuring, and the setting's own choices by label — the label is
    /// what `set` takes, so the library's ids stay out of the contract.
    #[test]
    fn one_setting_json_carries_the_choices_by_label() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let choices = dmm.choices(Setting::Range, &reading);
        let doc = serde_json::Value::Object(one_setting_json(
            "Fake meter",
            &reading,
            Setting::Range,
            &choices,
        ));
        let parsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(parsed["device"], "Fake meter");
        assert_eq!(parsed["mode"], "V AC");
        assert_eq!(parsed["range"], "22V");
        assert_eq!(parsed["setting"], "range");
        let printed = serde_json::to_string(&doc).unwrap();
        assert!(
            printed.starts_with(r#"{"device":"Fake meter","mode":"V AC","range":"22V","setting""#),
            "the header leads, as the reference shows: {printed}"
        );
        assert_eq!(parsed["current"], "Auto");
        assert!(parsed["choices"][0].get("id").is_none(), "no ids: {parsed}");
        assert_eq!(parsed["choices"][0]["label"], "Auto");
        assert_eq!(parsed["choices"][0]["current"], true);
        assert_eq!(parsed["choices"][2]["label"], "22V");
        assert_eq!(parsed["choices"][2]["current"], false);
        assert_eq!(parsed["choices"].as_array().unwrap().len(), 4);
    }

    /// `get --format json` is one object per invocation, not one per line —
    /// and a setting the meter offers nothing in is absent, not empty.
    #[test]
    fn the_settings_json_omits_a_setting_with_nothing_to_offer() {
        let (mut dmm, _) = fake_meter(&a_full_meter());
        let reading = dmm.request_measurement().expect("the fake meter answers");
        let offered = offered_settings(&dmm, &reading);
        let printed = serde_json::to_string(&serde_json::Value::Object(settings_json(
            "Fake meter",
            &reading,
            &offered,
        )))
        .unwrap();
        assert_eq!(printed.lines().count(), 1, "one object, not a stream");
        let parsed: serde_json::Value = serde_json::from_str(&printed).unwrap();
        assert_eq!(parsed["device"], "Fake meter");
        let settings = parsed["settings"].as_array().unwrap();
        let names: Vec<&str> = settings
            .iter()
            .map(|s| s["setting"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["mode", "range", "hold", "minmax"]);
        assert!(!printed.contains("peak"), "{printed}");
        assert_eq!(settings[2]["current"], "off");
        assert_eq!(settings[2]["choices"][1]["label"], "on");
    }

    /// `set hold on`: the meter is asked for id 1 and confirms it.
    #[test]
    fn set_switches_a_button_setting_by_label() {
        let (mut dmm, selected) = fake_meter(&a_full_meter());
        run_set(&mut dmm, Setting::Hold, Some("on".to_string())).expect("the meter takes it");
        assert_eq!(*selected.lock().expect("poisoned"), [(Setting::Hold, 1)]);
    }

    /// Asking for what the meter is already on costs no command at all.
    #[test]
    fn set_to_the_live_value_touches_nothing() {
        let (mut dmm, selected) = fake_meter(&a_full_meter());
        run_set(&mut dmm, Setting::Hold, Some("off".to_string())).expect("already there");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// A refusal is the meter answering: the error repeats what it said and
    /// what the user can do about it.
    #[test]
    fn a_refused_switch_repeats_the_meters_reason() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                refusal: Some("HOLD is locked out"),
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Hold, Some("on".to_string()))
            .expect_err("the meter refused")
            .to_string();
        assert!(err.contains("the meter refused on"), "{err}");
        assert!(err.contains("HOLD is locked out"), "{err}");
        assert!(err.contains(CHECK_DIAL_HINT), "{err}");
    }

    /// A meter that takes the command and never reports the new value fails
    /// after the deadline, naming what it is still on.
    #[test]
    fn a_switch_the_meter_never_confirms_fails() {
        let (mut dmm, selected) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                deaf: true,
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Hold, Some("on".to_string()))
            .expect_err("never confirmed")
            .to_string();
        assert!(err.contains("Meter did not switch (still off)"), "{err}");
        assert!(err.contains(CHECK_DIAL_HINT), "{err}");
        assert_eq!(*selected.lock().expect("poisoned"), [(Setting::Hold, 1)]);
    }

    /// A range is refused for one reason the dial does not cover, so its hint
    /// names that reason too.
    #[test]
    fn a_range_that_will_not_take_says_to_check_the_input() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                refusal: Some("out of range"),
                ..Default::default()
            },
        );
        let err = run_set(&mut dmm, Setting::Range, Some("22V".to_string()))
            .expect_err("the meter refused")
            .to_string();
        assert!(err.contains("within the range"), "{err}");
    }

    /// `set range auto` from a manual rung: Auto is id 0, and the meter is
    /// asked for exactly that.
    #[test]
    fn set_range_auto_asks_for_the_autorange_id() {
        let lists: [FakeList; 1] = [(Setting::Range, &["Auto", "2.2V", "22V"], 2)];
        let (mut dmm, selected) = fake_meter(&lists);
        run_set(&mut dmm, Setting::Range, Some("auto".to_string())).expect("the meter takes it");
        assert_eq!(
            *selected.lock().expect("poisoned"),
            [(Setting::Range, AUTO_RANGE_ID)]
        );
    }

    /// Each setting says what the meter now is in its own words, and the
    /// "already" line mirrors it word for word.
    #[test]
    fn switch_messages_speak_each_settings_own_language() {
        for (setting, id, label, expected) in [
            (Setting::Mode, 0x1121, "V AC Hz", "in V AC Hz"),
            (Setting::Range, AUTO_RANGE_ID, "Auto", "auto-ranging (22V)"),
            (Setting::Range, 2, "22V", "in 22V (manual range)"),
            (Setting::Hold, 1, "on", "HOLD on"),
            (Setting::Rel, 0, "off", "REL off"),
            (Setting::MinMax, 1, "max", "in max"),
            (Setting::MinMax, 0, "off", "out of MIN/MAX"),
            (Setting::Peak, 2, "P-MIN", "in P-MIN"),
            (Setting::Peak, 0, "off", "out of Peak"),
        ] {
            assert_eq!(
                switch_message(true, setting, id, label, "22V"),
                format!("Meter now {expected}")
            );
            assert_eq!(
                switch_message(false, setting, id, label, "22V"),
                format!("Meter is already {expected}")
            );
        }
    }

    /// A choice that matches several, or none, names the setting it was
    /// asked about — the wording is shared by all six.
    #[test]
    fn an_unresolvable_choice_names_the_setting() {
        let lists: [FakeList; 1] = [(
            Setting::Mode,
            &["Temp °C", "Temp °C T2", "Temp °C T1-T2"],
            0,
        )];
        let (mut dmm, selected) = fake_meter(&lists);
        let err = run_set(&mut dmm, Setting::Mode, Some("temp".to_string()))
            .expect_err("several match")
            .to_string();
        assert!(
            err.starts_with("ambiguous mode: temp matches Temp"),
            "{err}"
        );

        let (mut dmm, _) = fake_meter(&a_full_meter());
        let err = run_set(&mut dmm, Setting::Range, Some("500V".to_string()))
            .expect_err("none match")
            .to_string();
        assert_eq!(err, "unknown range: 500V");
        assert!(selected.lock().expect("poisoned").is_empty());
    }

    /// The vendor app sleeps 100 ms after every SET_MODE, so a meter that
    /// goes quiet across the switch is expected. One timeout must not end the
    /// 2 s wait the switch just started.
    #[test]
    fn a_switch_waits_through_a_quiet_meter() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                post_switch_errors: vec![dmm_lib::error::Error::Timeout],
                ..Default::default()
            },
        );
        run_set(&mut dmm, Setting::Mode, Some("V AC Hz".to_string()))
            .expect("a timeout must not end the wait");
    }

    /// Everything that is not a garbled frame or a quiet meter still ends the
    /// wait: a dead link is not something more polling will fix.
    #[test]
    fn a_switch_gives_up_on_a_lost_link() {
        let (mut dmm, _) = fake_meter_with(
            &a_full_meter(),
            Quirks {
                post_switch_errors: vec![dmm_lib::error::Error::NoTransportFound],
                ..Default::default()
            },
        );
        assert!(run_set(&mut dmm, Setting::Mode, Some("V AC Hz".to_string())).is_err());
    }

    #[test]
    fn resolve_choice_rejects_anything_else() {
        let choices = [mode_choice(0x1111, "V AC", true)];
        // Another mode's label, the id the listing no longer prints, and an
        // empty argument — which matches nothing rather than everything.
        for input in ["V DC", "0x1111", "4369", "", "   "] {
            assert!(
                matches!(resolve_choice(&choices, input), Err(NoMatch::Unknown)),
                "{input}"
            );
        }
    }

    #[test]
    fn clap_parse_debug() {
        let cli = Cli::try_parse_from(["dmm-cli", "debug", "--count", "5"]).unwrap();
        match cli.command {
            Cmd::Debug { count, interval_ms } => {
                assert_eq!(count, 5);
                assert_eq!(interval_ms, 500);
            }
            _ => panic!("expected Debug"),
        }
    }

    #[test]
    fn clap_parse_device_flag() {
        let cli = Cli::try_parse_from(["dmm-cli", "--device", "ut8803", "list"]).unwrap();
        assert_eq!(cli.device.as_deref(), Some("ut8803"));
    }

    #[test]
    fn clap_parse_device_flag_omitted() {
        let cli = Cli::try_parse_from(["dmm-cli", "list"]).unwrap();
        assert_eq!(cli.device, None);
    }

    /// One reading, as `output` writes it.
    fn rendered(mut output: format::Output, m: &dmm_lib::measurement::Measurement) -> String {
        let mut buf = Vec::new();
        output
            .write(&mut buf, m, &dmm_lib::WallClock::new(), None)
            .unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn csv_of(m: &dmm_lib::measurement::Measurement) -> String {
        rendered(
            format::Output::Csv(dmm_lib::export::CsvLayout::default()),
            m,
        )
    }

    fn json_of(m: &dmm_lib::measurement::Measurement, experimental: bool) -> serde_json::Value {
        serde_json::from_str(&rendered(format::Output::Json { experimental }, m)).unwrap()
    }

    #[test]
    fn format_text_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = rendered(format::Output::Text, &m);
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = csv_of(&m);
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert!(fields.len() >= 6);
        assert_eq!(fields[1], "DC V");
        assert_eq!(fields[2], "5.678");
        assert_eq!(fields[3], "V");
    }

    /// A meter that can report sub-values gets one column group per slot,
    /// sized by the family's `max_aux_values` so every row of a file lines up
    /// even when a mode reports fewer than the family can.
    #[test]
    fn format_csv_with_aux_slots() {
        use dmm_lib::measurement::AuxValue;

        let mut m = make_test_measurement(0x02, 0x01, b"239.22 ", (0x00, 0x00), (0x00, 0x00, 0x00));
        m.aux_values = vec![AuxValue {
            label: "Frequency".into(),
            value: MeasuredValue::Normal(50.01),
            unit: "Hz".into(),
            display_raw: Some("50.01".to_string()),
            elapsed_secs: None,
        }];
        let layout = dmm_lib::export::CsvLayout {
            family_slots: 2,
            ..Default::default()
        };
        let output = rendered(format::Output::Csv(layout), &m);
        let fields: Vec<&str> = output.trim_end().split(',').collect();
        assert_eq!(fields.len(), 6 + 2 * 3, "got {output}");
        assert_eq!(&fields[6..9], ["Frequency", "50.01", "Hz"]);
        // The unused second slot is present but empty.
        assert_eq!(&fields[9..12], ["", "", ""]);
        assert_eq!(layout.header().len(), fields.len());
    }

    /// The UT61E+ separates the sign from the digits on some ranges. That
    /// space must not reach the CSV, or the whole column parses as text.
    #[test]
    fn format_csv_negative_value_is_numeric() {
        let m = make_test_measurement(0x02, 0x01, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let output = csv_of(&m);
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert_eq!(fields[2], "-55.79");
        assert_eq!(fields[2].parse::<f64>().unwrap(), -55.79);
    }

    #[test]
    fn format_json_output() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on, inverted logic)
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert_eq!(parsed["mode"], "DC V");
        assert_eq!(parsed["value"], 5.678);
        assert_eq!(parsed["unit"], "V");
        assert_eq!(parsed["flags"]["hold"], true);
        assert_eq!(parsed["flags"]["auto_range"], true);
        assert_eq!(parsed["experimental"], false);
    }

    #[test]
    fn format_json_experimental_flag() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(json_of(&m, true)["experimental"], true);
    }

    #[test]
    fn format_csv_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert!(csv_of(&m).contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert_eq!(json_of(&m, false)["value"], "OL");
    }

    #[test]
    fn clap_parse_completions() {
        let cli = Cli::try_parse_from(["dmm-cli", "completions", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::Completions {
                shell: Some(Shell::Bash)
            }
        ));
    }

    #[test]
    fn format_csv_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        assert!(csv_of(&m).contains("NCV:3"));
    }

    #[test]
    fn format_json_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert_eq!(parsed["value"]["ncv_level"], 3);
        assert_eq!(parsed["mode"], "NCV");
    }

    #[test]
    fn format_text_includes_flags() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x00, 0x00));
        let output = rendered(format::Output::Text, &m);
        assert!(output.contains("HOLD"));
        assert!(output.contains("REL"));
    }

    #[test]
    fn format_json_negative_value() {
        let m = make_test_measurement(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let parsed = json_of(&m, false);
        assert!((parsed["value"].as_f64().unwrap() - (-12.345)).abs() < 1e-6);
    }

    /// Every format names the file it writes, and every one of those names
    /// picks it back out of an `-o` file name.
    #[test]
    fn a_format_and_its_file_extension_name_each_other() {
        for format in [
            OutputFormat::Text,
            OutputFormat::Csv,
            OutputFormat::Json,
            OutputFormat::Replay,
        ] {
            assert_eq!(
                OutputFormat::from_extension(format.extension()),
                Some(format),
                "{}",
                format.name()
            );
            // The name a message quotes back is the one `--format` takes.
            assert_eq!(
                format
                    .to_possible_value()
                    .expect("a --format value")
                    .get_name(),
                format.name()
            );
        }
        assert_eq!(OutputFormat::from_extension("dat"), None);
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
