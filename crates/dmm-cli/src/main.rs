mod capture;
mod format;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use console::style;
use dmm_lib::binary_help::ConnectedAdapters;
use dmm_lib::error::ErrorKind;
use dmm_lib::protocol::registry::{self, SelectableDevice};
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transform::{FactorError, Transform};
use log::{error, info};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Device to connect to [ut61eplus, ut8803, ut171, ut181a, mock, ...].
    /// If omitted, falls back to `device_family` in ~/.config/dmm-tools/settings.json
    /// (written by dmm-gui), then to `ut61eplus` as a last resort.
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
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
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
        #[arg(
            long,
            long_help = "\
Pin the mock device to a specific measurement mode instead of \
auto-cycling. Only effective with --device mock.

Modes: dcv, acv, ohm, cap, hz, temp, dcma, ohm-ol, ncv

Example: --device mock read --mock-mode dcv"
        )]
        mock_mode: Option<String>,
    },
    /// Send a button press command to the meter.
    /// Run with no arguments to list available commands for the selected device.
    Command {
        /// Command name (run without arguments to see available commands)
        action: Option<String>,
    },
    /// Switch the meter's function without touching the dial.
    /// Run with no arguments to list the modes reachable from where it sits now.
    Mode {
        /// Mode label, or a unique fragment of one, from the listing (run without arguments to see them)
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
        /// List all available step IDs and exit
        #[arg(long)]
        list_steps: bool,
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

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Csv,
    Json,
}

/// Where the effective `--device` value came from. Drives the dim fallback
/// notice: we only warn when the user picked neither on the CLI nor in the
/// shared settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceSource {
    Cli,
    Settings,
    Fallback,
}

/// Resolve `--device` precedence: explicit CLI flag → `device_family` in the
/// shared settings file (written by `dmm-gui`) → registry default.
///
/// The final fallback goes through `registry::default_device()` so the CLI
/// and the registry stay in sync — there's one source of truth for "which
/// device is the default when nothing is specified".
fn resolve_device_family(cli_device: Option<&str>) -> (String, DeviceSource) {
    if let Some(d) = cli_device {
        return (d.to_string(), DeviceSource::Cli);
    }
    if let Some(s) = dmm_settings::SharedSettings::load_if_exists()
        && !s.device_family.is_empty()
    {
        return (s.device_family, DeviceSource::Settings);
    }
    (
        registry::default_device().id.to_string(),
        DeviceSource::Fallback,
    )
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

    let (device_id, device_source) = resolve_device_family(cli.device.as_deref());
    let device = match registry::resolve_device(&device_id) {
        Some(d) => d,
        None => {
            eprintln!(
                "{} unknown device: {}",
                style("Error:").red().bold(),
                device_id,
            );
            std::process::exit(1);
        }
    };

    // Dim one-line notice when the user picked neither on the CLI nor in
    // settings — nudges toward an explicit choice without blocking. Skipped
    // for commands that don't open a device.
    let opens_device = !matches!(cli.command, Cmd::List | Cmd::Completions { .. });
    if opens_device && device_source == DeviceSource::Fallback {
        eprintln!(
            "{}",
            style(format!(
                "Using default device: {} (pass --device or set device_family in dmm-gui settings to change)",
                device.id
            ))
            .dim()
        );
    }

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

        // Mock device
        Cmd::Read {
            interval_ms,
            format,
            output,
            count,
            integrate,
            transform,
            mock_mode,
        } if !device.requires_hardware => cmd_read_mock(
            interval_ms,
            format,
            output,
            count,
            integrate,
            &transform.to_transform(),
            mock_mode,
        ),
        Cmd::Command { action } if !device.requires_hardware => cmd_command(device, None, action),
        Cmd::Mode { choice, mock_mode } if !device.requires_hardware => {
            cmd_mode(device, None, choice, mock_mode)
        }
        Cmd::Info | Cmd::Debug { .. } | Cmd::Capture { .. } if !device.requires_hardware => {
            eprintln!(
                "{} This command requires real hardware (not supported with --device {}).",
                style("Error:").red().bold(),
                device.id,
            );
            std::process::exit(1);
        }

        // Real device
        Cmd::Info => cmd_info(device, adapter),
        Cmd::Read {
            interval_ms,
            format,
            output,
            count,
            integrate,
            transform,
            mock_mode: _,
        } => cmd_read(
            device,
            adapter,
            interval_ms,
            format,
            output,
            count,
            integrate,
            &transform.to_transform(),
        ),
        Cmd::Command { action } => cmd_command(device, adapter, action),
        Cmd::Mode {
            choice,
            mock_mode: _,
        } => cmd_mode(device, adapter, choice, None),
        Cmd::Debug { count, interval_ms } => cmd_debug(device, adapter, count, interval_ms),
        Cmd::Capture {
            output,
            steps,
            list_steps,
        } => {
            if list_steps {
                // Device-scoped: the steps come from the selected device's
                // protocol, so what's listed is what `--steps` will match.
                capture::list_steps(device);
                Ok(())
            } else {
                open_with_help(device, adapter)
                    .and_then(|dmm| capture::cmd_capture(output, steps, dmm, device))
            }
        }
    };

    if let Err(e) = result {
        error!("{e}");
        let msg = e.to_string();
        if msg.contains("timeout") {
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
         \x20   3. Registry default ({default})\n\
         \n\
         ENVIRONMENT:\n\
         \x20 RUST_LOG    Log filter. Use `dmm_lib=trace` for wire-level debugging.\n\
         \x20 NO_COLOR    Set to 1 to disable colored terminal output.\n\
         \n\
         Help / GitHub: https://github.com/antoinecellerier/dmm-tools",
        path = resolved_config_path_display(),
        default = registry::default_device().id,
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

/// Setup guide URL, for hints printed by binaries installed outside a checkout.
#[cfg(target_os = "linux")]
const SETUP_DOC_URL: &str = "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/setup.md";

/// Print platform-specific setup instructions when no USB cable is detected.
fn print_transport_setup_help() {
    eprintln!("Check that the USB cable is plugged in and the meter is powered on.");
    #[cfg(target_os = "linux")]
    {
        eprintln!("On Linux, ensure the udev rule is installed:");
        eprintln!(
            "  {}",
            style("sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/").dim()
        );
        eprintln!("  {}", style("sudo udevadm control --reload-rules").dim());
        eprintln!("Then replug the cable. On a headless machine, keep a group on the");
        eprintln!("rule — see {}", style(SETUP_DOC_URL).dim());
    }
    #[cfg(target_os = "windows")]
    {
        eprintln!("Open Device Manager with the cable plugged in:");
        eprintln!("  - 'CP2110 USB to UART Bridge' under HID devices: no action needed.");
        eprintln!("  - 'USB Input Device' under HID devices: no action needed.");
        eprintln!("  - Yellow warning icon under 'Other devices': install the driver from");
        eprintln!(
            "    {}",
            style("https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers").dim()
        );
        eprintln!("  - Nothing appears: try a different USB port.");
    }
    #[cfg(target_os = "macos")]
    {
        eprintln!("On macOS, the cable should be recognized automatically (no driver needed).");
        eprintln!(
            "If the device is not found, check System Settings > Privacy & Security > Input Monitoring."
        );
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

/// Open the meter with helpful error messages for common failures.
fn open_with_help(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
) -> Result<dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>, Box<dyn std::error::Error>> {
    match dmm_lib::open_device_by_id_auto(device.id, adapter) {
        Ok(dmm) => {
            let profile = dmm.profile();
            if profile.stability == dmm_lib::protocol::Stability::Experimental {
                eprintln!(
                    "{}",
                    style(format!(
                        "WARNING: {} support is EXPERIMENTAL (unverified against real hardware).",
                        profile.model_name
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
            Ok(dmm)
        }
        Err(dmm_lib::error::Error::NoTransportFound) => {
            eprintln!("{}", style("USB cable not found.").yellow().bold());
            print_transport_setup_help();
            let proto = (device.new_protocol)();
            let profile = proto.profile();
            if profile.stability == dmm_lib::protocol::Stability::Experimental {
                eprintln!(
                    "{}",
                    style(format!(
                        "{} support is experimental — report feedback: {}",
                        profile.model_name,
                        profile.feedback_url()
                    ))
                    .yellow()
                );
            }
            Err("device not found".into())
        }
        Err(dmm_lib::error::Error::AdapterNotFound(ref detail)) => {
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
            Err("adapter not found".into())
        }
        Err(e) => Err(e.into()),
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

fn cmd_info(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_with_help(device, adapter)?;
    let name = dmm.get_name()?;
    match name {
        Some(ref n) => println!("Device: {}", style(n).bold()),
        None => println!("Device: {}", style("(name not supported)").dim()),
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
    device: &'static SelectableDevice,
    adapter: Option<&str>,
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
    count: usize,
    integrate: bool,
    transform: &Transform,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_with_help(device, adapter)?;
    let experimental = dmm.profile().stability == dmm_lib::protocol::Stability::Experimental;
    info!("connected, starting measurement loop");
    run_read_loop(
        &mut dmm,
        interval_ms,
        &format,
        output_path,
        count,
        experimental,
        Some(device),
        integrate,
        transform,
    )
}

fn cmd_read_mock(
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
    count: usize,
    integrate: bool,
    transform: &Transform,
    mock_mode: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_mock_device(mock_mode)?;
    info!("mock device connected, starting measurement loop");
    // Mock returns instantly — use 100ms floor to simulate ~10 Hz
    let interval_ms = if interval_ms == 0 { 100 } else { interval_ms };
    run_read_loop(
        &mut dmm,
        interval_ms,
        &format,
        output_path,
        count,
        false,
        None,
        integrate,
        transform,
    )
}

/// Open the mock, pinned to `mock_mode` when one was given.
///
/// Shared by every subcommand that takes `--mock-mode`, so an unknown mode
/// name is rejected with the same message (and the same list of valid names)
/// wherever it is passed.
fn open_mock_device(
    mock_mode: Option<String>,
) -> Result<dmm_lib::Dmm<dmm_lib::transport::NullTransport>, Box<dyn std::error::Error>> {
    match mock_mode {
        Some(mode_str) => {
            let mode: dmm_lib::mock::MockMode = mode_str
                .parse()
                .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;
            Ok(dmm_lib::mock::open_mock_mode(mode)?)
        }
        None => Ok(dmm_lib::mock::open_mock()?),
    }
}

/// Shared measurement loop for both real and mock devices.
#[allow(clippy::too_many_arguments)]
fn run_read_loop<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    interval_ms: u64,
    format: &OutputFormat,
    output_path: Option<String>,
    count: usize,
    experimental: bool,
    // When set, timeout warnings include device-specific activation instructions.
    device: Option<&'static SelectableDevice>,
    integrate: bool,
    // Applied to every reading before anything else sees it; the identity
    // transform (no --scale/--offset/--unit) is a no-op.
    transform: &Transform,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;

    let mut writer: Box<dyn Write> = match &output_path {
        Some(path) => Box::new(std::fs::File::create(path).map(std::io::BufWriter::new)?),
        None => Box::new(std::io::stdout().lock()),
    };

    let model_name = dmm.profile().model_name;
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
    match format {
        OutputFormat::Csv => {
            writeln!(writer, "{}", dmm_lib::export::device_comment(model_name))?;
            writeln!(writer, "{}", layout.header().join(","))?;
        }
        OutputFormat::Json => {
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&serde_json::json!({"_metadata":{"device": model_name}}))
                    .map_err(std::io::Error::other)?
            )?;
        }
        OutputFormat::Text => {}
    }

    let tick = Duration::from_millis(interval_ms);
    let wall_clock = dmm_lib::WallClock::new();
    // Min/Max/Avg and the integral are only meaningful within a single mode
    // and unit; `SeriesStats` resets both whenever either moves, so the
    // closing summary only ever covers one comparable series.
    let mut session = dmm_lib::stats::SeriesStats::new(integrate);
    let mut i = 0usize;
    let mut protocol_errors = 0usize;
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

                format::format_measurement(
                    &mut writer,
                    &m,
                    &wall_clock,
                    format,
                    experimental,
                    integral_display,
                    layout,
                )?;
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
                return Err(e.into());
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
    Ok(())
}

fn cmd_command(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
    action: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = match action {
        Some(a) => a,
        None => return print_available_commands(device),
    };

    if device.requires_hardware {
        let mut dmm = open_with_help(device, adapter)?;
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

/// The one thing a user can do about a mode the meter won't take. Both
/// failures below end here, so they end in the same words.
const CHECK_DIAL_HINT: &str = "check the dial position";

/// How long to wait for a switched mode to show up in the measurement
/// stream. The meter acknowledges the command before the frame carrying the
/// new mode arrives, so "accepted" and "switched" are two separate answers.
const MODE_SWITCH_TIMEOUT: Duration = Duration::from_secs(2);

/// Gap between polls while waiting for that frame.
const MODE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// List or switch the modes the meter reaches from its current dial position.
///
/// Both paths need a reading first: the choices are relative to what the
/// meter is measuring now, so there is nothing to list or match against
/// until one frame has arrived.
fn cmd_mode(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
    choice: Option<String>,
    mock_mode: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if device.requires_hardware {
        let mut dmm = open_with_help(device, adapter)?;
        run_mode(&mut dmm, choice)
    } else {
        let mut dmm = open_mock_device(mock_mode)?;
        run_mode(&mut dmm, choice)
    }
}

/// Whether the meter has a mode to switch *to* from where its dial sits.
///
/// A single choice is the live mode on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to list, and nothing to switch.
fn offers_a_mode_switch(choices: &[dmm_lib::protocol::Choice]) -> bool {
    choices.len() > 1
}

fn run_mode<T: dmm_lib::transport::Transport>(
    dmm: &mut dmm_lib::Dmm<T>,
    choice: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let model_name = dmm.profile().model_name;
    let reading = dmm.request_measurement()?;
    let choices = dmm.choices(dmm_lib::protocol::Setting::Mode, &reading);

    if !offers_a_mode_switch(&choices) {
        eprintln!(
            "{} {model_name} has no switchable modes in {} \u{2014} use the dial.",
            style("Note:").yellow(),
            reading.mode,
        );
        return Ok(());
    }

    let Some(input) = choice else {
        print_mode_choices(model_name, &choices);
        // Name a fragment that is actually in the column above, and
        // preferably one that would change something.
        let example = choices.iter().find(|c| !c.current).unwrap_or(&choices[0]);
        eprintln!(
            "\n{}",
            style(format!(
                "Tip: the right column switches to that mode, e.g. dmm-cli mode {}",
                quote_for_shell(&shortest_fragment(&choices, example))
            ))
            .dim()
        );
        return Ok(());
    };

    let target = match resolve_mode_choice(&choices, &input) {
        Ok(target) => target,
        Err(NoModeMatch::Ambiguous(labels)) => {
            return Err(format!("ambiguous mode: {input} matches {}", labels.join(", ")).into());
        }
        Err(NoModeMatch::Unknown) => {
            print_mode_choices(model_name, &choices);
            return Err(format!("unknown mode: {input}").into());
        }
    };
    let (id, label) = (target.id, target.label.to_string());

    if target.current {
        println!("{} {label}", style("Meter is already in").green());
        return Ok(());
    }
    // Everything below reads from `choices` again, so drop the borrow.
    let mut live = choices
        .iter()
        .find(|c| c.current)
        .map_or_else(|| reading.mode.to_string(), |c| c.label.to_string());

    if let Err(e) = dmm.select(dmm_lib::protocol::Setting::Mode, id) {
        // A refusal is the meter answering, not a fault: say so, and say what
        // the user can do about it.
        return Err(match e {
            dmm_lib::error::Error::CommandRejected(detail) => {
                format!("the meter refused {label}: {detail} \u{2014} {CHECK_DIAL_HINT}").into()
            }
            other => Box::<dyn std::error::Error>::from(other),
        });
    }

    let started = std::time::Instant::now();
    loop {
        match dmm.request_measurement() {
            Ok(reading) => {
                let choices = dmm.choices(dmm_lib::protocol::Setting::Mode, &reading);
                if choices.iter().any(|c| c.id == id && c.current) {
                    println!("{} {label}", style("Meter now in").green());
                    return Ok(());
                }
                if let Some(c) = choices.iter().find(|c| c.current) {
                    live = c.label.to_string();
                }
            }
            // The frame straddling the switch can be unreadable, and the meter
            // can go quiet across it altogether — the vendor app sleeps 100 ms
            // after every SET_MODE. Neither ends the wait: the next frame
            // parses, and the 2 s deadline below is what gives up. Anything
            // else is a real fault.
            Err(e) if matches!(e.kind(), ErrorKind::Protocol | ErrorKind::Timeout) => {
                log::warn!("waiting for the mode switch: {e}");
            }
            Err(e) => return Err(e.into()),
        }
        if std::time::Instant::now()
            .checked_duration_since(started)
            .unwrap_or_default()
            >= MODE_SWITCH_TIMEOUT
        {
            break;
        }
        std::thread::sleep(MODE_POLL_INTERVAL);
    }

    Err(format!("Meter did not switch (still {live}) \u{2014} {CHECK_DIAL_HINT}").into())
}

/// One line per choice, `*` on the live one, and the least that has to be
/// typed to reach it in a second column — so the fragment form is on screen
/// rather than something to guess at.
fn print_mode_choices(model_name: &str, choices: &[dmm_lib::protocol::Choice]) {
    println!("Modes for {}:", style(model_name).bold());
    let width = choices
        .iter()
        .map(|c| c.label.chars().count())
        .max()
        .unwrap_or(0);
    for c in choices {
        // Pad the bare label: styling it first would count escape bytes
        // toward the width and misalign the column.
        let pad = " ".repeat(width - c.label.chars().count());
        println!(
            "{} {}{pad}  {}",
            if c.current {
                style("*").green().bold()
            } else {
                style(" ")
            },
            c.label,
            style(quote_for_shell(&shortest_fragment(choices, c))).dim(),
        );
    }
}

/// The shortest run of words from a choice's label that [`resolve_mode_choice`]
/// maps back to that same choice — what the listing shows as the thing to type.
///
/// Runs are tried shortest first, measured in characters and, at equal length,
/// leftmost first. A label whose every fragment is shared with a longer label
/// ("V AC" beside "V AC Hz") has no shorter form: the whole label comes back,
/// which the resolver takes as an exact match.
fn shortest_fragment(
    choices: &[dmm_lib::protocol::Choice],
    target: &dmm_lib::protocol::Choice,
) -> String {
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
        .find(|run| resolve_mode_choice(choices, run).is_ok_and(|hit| hit.id == target.id))
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
enum NoModeMatch<'a> {
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
fn resolve_mode_choice<'a>(
    choices: &'a [dmm_lib::protocol::Choice],
    input: &str,
) -> Result<&'a dmm_lib::protocol::Choice, NoModeMatch<'a>> {
    let needle = typeable(input.trim());
    if needle.is_empty() {
        return Err(NoModeMatch::Unknown);
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
        [] => Err(NoModeMatch::Unknown),
        _ => Err(NoModeMatch::Ambiguous(
            hits.iter().map(|c| c.label.as_ref()).collect(),
        )),
    }
}

fn cmd_debug(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
    count: usize,
    interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;

    let mut dmm = open_with_help(device, adapter)?;

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
    use dmm_lib::protocol::{Choice, Setting};

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
            } => {
                assert_eq!(interval_ms, 0);
                assert!(matches!(format, OutputFormat::Text));
                assert!(output.is_none());
                assert_eq!(count, 0);
                assert!(!integrate);
                // No transform flags means the identity, so `read` keeps the
                // reading and the column layout it always had.
                assert_eq!(transform.to_transform(), Transform::default());
                assert!(transform.to_transform().is_identity());
                assert!(mock_mode.is_none());
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
            } => {
                assert_eq!(interval_ms, 100);
                assert!(matches!(format, OutputFormat::Csv));
                assert_eq!(output.as_deref(), Some("test.csv"));
                assert_eq!(count, 10);
            }
            _ => panic!("expected Read"),
        }
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
                assert!(matches!(format, OutputFormat::Csv));
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
    fn clap_parse_mode() {
        let cli = Cli::try_parse_from(["dmm-cli", "mode", "V AC Hz"]).unwrap();
        match cli.command {
            Cmd::Mode { choice, .. } => assert_eq!(choice.as_deref(), Some("V AC Hz")),
            _ => panic!("expected Mode"),
        }
    }

    #[test]
    fn clap_parse_mode_no_choice_lists_modes() {
        let cli = Cli::try_parse_from(["dmm-cli", "mode"]).unwrap();
        match cli.command {
            Cmd::Mode { choice, mock_mode } => {
                assert!(choice.is_none());
                assert!(mock_mode.is_none());
            }
            _ => panic!("expected Mode"),
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
    fn resolve_mode_choice_matches_a_label_case_insensitively() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ];
        for input in ["V AC Hz", "v ac hz", "  V Ac hZ  "] {
            assert_eq!(
                resolve_mode_choice(&choices, input).ok().map(|c| c.id),
                Some(0x1121),
                "{input}"
            );
        }
    }

    /// The symbols the meters print are not on a keyboard, so their spelled
    /// out forms match too.
    #[test]
    fn resolve_mode_choice_accepts_typeable_spellings() {
        let choices = [
            mode_choice(0x06, "Ω", true),
            mode_choice(0x0C, "DC µA", false),
            mode_choice(0x14, "°C", false),
        ];
        for (input, id) in [("ohm", 0x06), ("Ω", 0x06), ("dc ua", 0x0C), ("c", 0x14)] {
            assert_eq!(
                resolve_mode_choice(&choices, input).ok().map(|c| c.id),
                Some(id),
                "{input}"
            );
        }
    }

    /// Typing a whole label is tedious, so a fragment of exactly one of them
    /// is enough.
    #[test]
    fn resolve_mode_choice_matches_a_unique_label_fragment() {
        let temps = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
        ];
        assert_eq!(
            resolve_mode_choice(&temps, "t1-t2").ok().map(|c| c.id),
            Some(0x4231)
        );
        let volts = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_mode_choice(&volts, "hz").ok().map(|c| c.id),
            Some(0x1121)
        );
    }

    /// A label that is also a fragment of longer ones stays reachable: typed
    /// in full it is an exact match, and an exact match wins outright.
    #[test]
    fn resolve_mode_choice_prefers_an_exact_label_over_a_fragment() {
        let choices = [
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
            mode_choice(0x1131, "V AC Peak", false),
        ];
        assert_eq!(
            resolve_mode_choice(&choices, "v ac").ok().map(|c| c.id),
            Some(0x1111)
        );
    }

    /// A fragment of several labels picks none of them, and says which ones
    /// it was torn between — that is what the user has to narrow down.
    #[test]
    fn resolve_mode_choice_reports_an_ambiguous_fragment() {
        let choices = [
            mode_choice(0x4211, "Temp °C", true),
            mode_choice(0x4221, "Temp °C T2", false),
            mode_choice(0x4231, "Temp °C T1-T2", false),
            mode_choice(0x4241, "Temp °C T2-T1", false),
        ];
        match resolve_mode_choice(&choices, "temp") {
            Err(NoModeMatch::Ambiguous(labels)) => assert_eq!(
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
                    resolve_mode_choice(&choices, &fragment)
                        .ok()
                        .map(|hit| hit.id),
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

    /// A meter whose dial reaches `labels`, sitting on the first of them.
    ///
    /// `post_switch_errors` are handed out in place of the readings that
    /// follow a successful switch — the meter garbling a frame or going quiet
    /// across a SET_MODE.
    struct FakeMeter {
        labels: Vec<&'static str>,
        live: usize,
        switched: bool,
        post_switch_errors: Vec<dmm_lib::error::Error>,
        /// Ids `select` was asked for, so a test can assert it was left
        /// alone.
        selected: std::sync::Arc<std::sync::Mutex<Vec<u16>>>,
    }

    impl dmm_lib::protocol::Protocol for FakeMeter {
        fn init(&mut self, _t: &dyn dmm_lib::transport::Transport) -> dmm_lib::error::Result<()> {
            Ok(())
        }

        fn request_measurement(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
        ) -> dmm_lib::error::Result<dmm_lib::measurement::Measurement> {
            if self.switched && !self.post_switch_errors.is_empty() {
                return Err(self.post_switch_errors.remove(0));
            }
            Ok(dmm_lib::measurement::Measurement {
                mode: self.labels[self.live].into(),
                mode_raw: fake_mode_id(self.live),
                ..dmm_lib::measurement::Measurement::test_fixture(
                    MeasuredValue::Normal(1.0),
                    "V",
                    dmm_lib::flags::StatusFlags::default(),
                )
            })
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
            _setting: Setting,
            _current: &dmm_lib::measurement::Measurement,
        ) -> Vec<Choice> {
            self.labels
                .iter()
                .enumerate()
                .map(|(i, label)| Choice {
                    id: fake_mode_id(i),
                    label: std::borrow::Cow::Borrowed(label),
                    current: i == self.live,
                })
                .collect()
        }

        fn select(
            &mut self,
            _t: &dyn dmm_lib::transport::Transport,
            _setting: Setting,
            id: u16,
        ) -> dmm_lib::error::Result<()> {
            self.selected.lock().expect("poisoned").push(id);
            match (0..self.labels.len()).find(|&i| fake_mode_id(i) == id) {
                Some(i) => {
                    self.live = i;
                    self.switched = true;
                    Ok(())
                }
                None => Err(dmm_lib::error::Error::UnsupportedCommand(format!(
                    "mode {id:#06x}"
                ))),
            }
        }
    }

    type SelectedIds = std::sync::Arc<std::sync::Mutex<Vec<u16>>>;

    fn fake_meter(
        labels: &[&'static str],
        post_switch_errors: Vec<dmm_lib::error::Error>,
    ) -> (dmm_lib::Dmm<dmm_lib::transport::NullTransport>, SelectedIds) {
        let selected: SelectedIds = Default::default();
        let meter = FakeMeter {
            labels: labels.to_vec(),
            live: 0,
            switched: false,
            post_switch_errors,
            selected: std::sync::Arc::clone(&selected),
        };
        let dmm = dmm_lib::Dmm::new(dmm_lib::transport::NullTransport, Box::new(meter))
            .expect("the fake meter needs no transport");
        (dmm, selected)
    }

    /// A single-variant dial (the UT181A Ohm, nS, Cap, Hz, Duty and Pulse
    /// Width positions) reports one choice: the mode the meter is already in.
    /// Listing it, and tipping the user to switch to it, is noise — it counts
    /// as nothing to switch, exactly as an empty list does.
    #[test]
    fn only_the_live_mode_is_not_a_switch_to_offer() {
        assert!(!offers_a_mode_switch(&[]));
        assert!(!offers_a_mode_switch(&[mode_choice(
            0x5111,
            "Resistance",
            true
        )]));
        assert!(offers_a_mode_switch(&[
            mode_choice(0x1111, "V AC", true),
            mode_choice(0x1121, "V AC Hz", false),
        ]));
    }

    /// And the command says so and stops: exit 0, meter untouched, whether or
    /// not a choice was asked for.
    #[test]
    fn a_dial_with_only_the_live_mode_switches_nothing() {
        for arg in [None, Some("Resistance".to_string())] {
            let (mut dmm, selected) = fake_meter(&["Resistance"], vec![]);
            run_mode(&mut dmm, arg).expect("one choice is not a failure");
            assert!(
                selected.lock().expect("poisoned").is_empty(),
                "the meter was switched"
            );
        }
    }

    /// The vendor app sleeps 100 ms after every SET_MODE, so a meter that
    /// goes quiet across the switch is expected. One timeout must not end the
    /// 2 s wait the switch just started.
    #[test]
    fn a_mode_switch_waits_through_a_quiet_meter() {
        let (mut dmm, _) = fake_meter(&["V AC", "V AC Hz"], vec![dmm_lib::error::Error::Timeout]);
        run_mode(&mut dmm, Some("V AC Hz".to_string())).expect("a timeout must not end the wait");
    }

    /// Everything that is not a garbled frame or a quiet meter still ends the
    /// wait: a dead link is not something more polling will fix.
    #[test]
    fn a_mode_switch_gives_up_on_a_lost_link() {
        let (mut dmm, _) = fake_meter(
            &["V AC", "V AC Hz"],
            vec![dmm_lib::error::Error::NoTransportFound],
        );
        assert!(run_mode(&mut dmm, Some("V AC Hz".to_string())).is_err());
    }

    #[test]
    fn resolve_mode_choice_rejects_anything_else() {
        let choices = [mode_choice(0x1111, "V AC", true)];
        // Another mode's label, the id the listing no longer prints, and an
        // empty argument — which matches nothing rather than everything.
        for input in ["V DC", "0x1111", "4369", "", "   "] {
            assert!(
                matches!(
                    resolve_mode_choice(&choices, input),
                    Err(NoModeMatch::Unknown)
                ),
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

    #[test]
    fn resolve_device_cli_takes_precedence() {
        let (id, src) = resolve_device_family(Some("ut8803"));
        assert_eq!(id, "ut8803");
        assert_eq!(src, DeviceSource::Cli);
    }

    #[test]
    fn resolve_device_fallback_when_nothing_set() {
        // Note: this test is environment-sensitive — if the test machine has
        // a real ~/.config/dmm-tools/settings.json with device_family set,
        // the resolver will return DeviceSource::Settings instead. That's
        // still a valid path; what matters is that the CLI arg is absent.
        let (id, src) = resolve_device_family(None);
        assert!(matches!(
            src,
            DeviceSource::Settings | DeviceSource::Fallback
        ));
        if src == DeviceSource::Fallback {
            assert_eq!(id, registry::default_device().id);
        }
    }

    #[test]
    fn format_text_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Text,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert!(fields.len() >= 6);
        assert_eq!(fields[1], "DC V");
        assert_eq!(fields[2], "5.678");
        assert_eq!(fields[3], "V");
        assert_eq!(fields[4], "22V");
    }

    /// Multi-display meters carry their sub-values in fixed trailing columns,
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
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
            dmm_lib::export::CsvLayout {
                family_slots: 2,
                ..Default::default()
            },
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let fields: Vec<&str> = output.trim_end().split(',').collect();
        assert_eq!(fields.len(), 6 + 2 * 3, "got {output}");
        assert_eq!(&fields[6..9], ["Frequency", "50.01", "Hz"]);
        // The unused second slot is present but empty.
        assert_eq!(&fields[9..12], ["", "", ""]);
        assert_eq!(
            dmm_lib::export::CsvLayout {
                family_slots: 2,
                ..Default::default()
            }
            .header()
            .len(),
            fields.len()
        );
    }

    /// The UT61E+ separates the sign from the digits on some ranges. That
    /// space must not reach the CSV, or the whole column parses as text.
    #[test]
    fn format_csv_negative_value_is_numeric() {
        let m = make_test_measurement(0x02, 0x01, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert_eq!(fields[2], "-55.79");
        assert_eq!(fields[2].parse::<f64>().unwrap(), -55.79);
    }

    #[test]
    fn format_json_output() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on, inverted logic)
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
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
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Json,
            true,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["experimental"], true);
    }

    #[test]
    fn format_csv_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"], "OL");
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
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Csv,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("NCV:3"));
    }

    #[test]
    fn format_json_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"]["ncv_level"], 3);
        assert_eq!(parsed["mode"], "NCV");
    }

    #[test]
    fn format_text_includes_flags() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Text,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("HOLD"));
        assert!(output.contains("REL"));
    }

    #[test]
    fn format_json_negative_value() {
        let m = make_test_measurement(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(
            &mut buf,
            &m,
            &dmm_lib::WallClock::new(),
            &OutputFormat::Json,
            false,
            None,
            dmm_lib::export::CsvLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!((parsed["value"].as_f64().unwrap() - (-12.345)).abs() < 1e-6);
    }
}
