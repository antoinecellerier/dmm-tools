mod capture;
mod format;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use console::style;
use dmm_lib::error::ErrorKind;
use dmm_lib::measurement::MeasuredValue;
use dmm_lib::protocol::registry::{self, SelectableDevice};
use dmm_lib::stream::{MeasurementStream, StreamEvent};
use dmm_lib::transform::Transform;
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

/// Parse a transform flag's number, rejecting NaN and infinity.
///
/// Neither is caught anywhere downstream: they propagate through every
/// reading into the stats and the integral, whose summary then reads `NaN`
/// with nothing to say where it came from. `flag` names the offending flag so
/// `--scale` and `--offset` cannot word the same rejection two ways.
fn parse_finite(flag: &str, s: &str) -> Result<f64, String> {
    let value: f64 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    if !value.is_finite() {
        return Err(format!("{flag} must be a finite number, got `{s}`"));
    }
    Ok(value)
}

/// Reject the two scale factors that destroy the reading rather than
/// re-express it: zero collapses every sample onto the offset, and NaN/inf
/// poison the stats and the integral.
fn parse_scale(s: &str) -> Result<f64, String> {
    let value = parse_finite("scale", s)?;
    if value == 0.0 {
        return Err("scale must not be zero — it would flatten every reading to the offset".into());
    }
    Ok(value)
}

/// Any finite shift is a meaningful offset — zero and negatives included — so
/// only NaN and infinity are rejected, on the same grounds as `--scale`.
fn parse_offset(s: &str) -> Result<f64, String> {
    parse_finite("offset", s)
}

/// The note to print when a reading starts a new statistics series, or `None`
/// while it continues the current one.
///
/// Mode *and* unit, not the unit alone: `--unit` pins the label, so a dial
/// turn from V to Ω would otherwise leave the unit identical and average
/// volts with ohms in silence. Auto-range moves the unit without touching the
/// mode (mV → V), so neither check subsumes the other. Same condition the GUI
/// resets its stats and integrator on.
fn series_change(
    (prev_mode, prev_unit): (&str, &str),
    (mode, unit): (&str, &str),
) -> Option<String> {
    if prev_mode != mode {
        Some(format!("Mode changed ({prev_mode} \u{2192} {mode})"))
    } else if prev_unit != unit {
        Some(format!("Unit changed ({prev_unit} \u{2192} {unit})"))
    } else {
        None
    }
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

/// Print platform-specific setup instructions when no USB cable is detected.
fn print_transport_setup_help() {
    eprintln!("Check that the USB cable is plugged in and the meter is powered on.");
    #[cfg(target_os = "linux")]
    {
        eprintln!("On Linux, ensure the udev rule is installed:");
        eprintln!(
            "  {}",
            style("sudo cp udev/99-dmm-tools.rules /etc/udev/rules.d/").dim()
        );
        eprintln!("  {}", style("sudo udevadm control --reload-rules").dim());
        eprintln!(
            "Your user must be in the plugdev group: {}",
            style("sudo usermod -aG plugdev $USER").dim()
        );
        eprintln!("Then log out/in and replug the cable.");
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
        Err(dmm_lib::error::Error::DeviceNotFound { .. })
        | Err(dmm_lib::error::Error::NoTransportFound) => {
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
            match dmm_lib::list_devices() {
                Ok(devices) if devices.is_empty() => {
                    eprintln!("{}", style("No devices currently connected.").yellow());
                }
                Ok(devices) => {
                    eprintln!("\n{}:", style("Connected devices").yellow());
                    for (i, dev) in devices.iter().enumerate() {
                        eprintln!("  {} {dev}", style(format!("[{i}]")).cyan());
                    }
                    eprintln!(
                        "\n{}",
                        style("Use --adapter <serial-or-path> to select one.").dim()
                    );
                }
                Err(_) => {
                    eprintln!(
                        "{}",
                        style("Run 'dmm-cli list' to see connected devices.").yellow()
                    );
                }
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
    let mut dmm = match mock_mode {
        Some(mode_str) => {
            let mode: dmm_lib::mock::MockMode = mode_str
                .parse()
                .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;
            dmm_lib::mock::open_mock_mode(mode)?
        }
        None => dmm_lib::mock::open_mock()?,
    };
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
    let aux = format::AuxLayout {
        family: dmm.profile().max_aux_values,
        extra: transform.extra_aux_count(),
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
            writeln!(writer, "# device: {model_name}")?;
            writeln!(writer, "{}", format::csv_header(integrate, aux.total()))?;
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
    let mut stats = dmm_lib::stats::RunningStats::default();
    let mut integrator = dmm_lib::stats::Integrator::new();
    // Mode and unit the current stats/integral series accumulates in. A
    // change to either resets both, so the closing summary only ever covers
    // one comparable series.
    let mut series: Option<(String, String)> = None;
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
                // single mode and unit. Auto-range moves the unit a decade
                // without touching the mode string, and turning the dial
                // changes the mode — which `--unit` would otherwise hide, as
                // it pins the label across the change. Without both checks the
                // closing summary averages volts with ohms and prints bare
                // numbers, so nothing on screen would reveal the mix.
                let current_unit: &str = &m.unit;
                if let Some((prev_mode, prev_unit)) = &series
                    && let Some(note) =
                        series_change((prev_mode, prev_unit), (&m.mode, current_unit))
                {
                    let what = if integrate {
                        "statistics and integral"
                    } else {
                        "statistics"
                    };
                    eprintln!("{} {note}, {what} reset", style("Note:").yellow());
                    stats.reset();
                    integrator.reset();
                }
                series = Some((m.mode.to_string(), current_unit.to_string()));

                if let MeasuredValue::Normal(v) = &m.value {
                    stats.push(*v);
                }

                // Integration tracking
                let integral_display = if integrate {
                    match &m.value {
                        MeasuredValue::Normal(v) => integrator.push(*v, m.timestamp),
                        MeasuredValue::Overload => integrator.push_overload(),
                        _ => {}
                    }

                    dmm_lib::stats::integral_unit_info(current_unit)
                        .map(|(disp_unit, divisor)| (integrator.value() / divisor, disp_unit))
                } else {
                    None
                };

                format::format_measurement(
                    &mut writer,
                    &m,
                    &wall_clock,
                    format,
                    experimental,
                    integral_display,
                    aux,
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

    if let (Some(min), Some(max), Some(avg)) = (stats.min, stats.max, stats.avg()) {
        // Name the unit the figures are in. They reset whenever the mode or
        // the unit moves, so this is the unit every sample behind them was
        // measured in.
        let unit_suffix = series
            .as_ref()
            .map(|(_, unit)| unit.as_str())
            .filter(|u| !u.is_empty())
            .map(|u| format!(" {u}"))
            .unwrap_or_default();
        eprintln!(
            "\n{} {} samples | Min: {}{unit_suffix} | Max: {}{unit_suffix} | Avg: {}{unit_suffix}",
            style("---").dim(),
            stats.count,
            style(format!("{min:.4}")).cyan(),
            style(format!("{max:.4}")).cyan(),
            style(format!("{avg:.4}")).cyan(),
        );
        if integrate
            && let Some((_, unit_str)) = &series
            && let Some((disp_unit, divisor)) = dmm_lib::stats::integral_unit_info(unit_str)
        {
            let dt_str = integrator
                .elapsed_secs()
                .map(|s| format!(" ({}s)", style(format!("{s:.1}")).cyan()))
                .unwrap_or_default();
            eprintln!(
                "    Integral: {} {disp_unit}{dt_str}",
                style(format!("{:.4}", integrator.value() / divisor)).cyan(),
            );
            if integrator.skipped_intervals > 0 {
                eprintln!(
                    "    {} {} intervals skipped (sample spacing exceeds the 2 s integrator limit \u{2014} lower --interval-ms for more frequent samples or expect a partial integral)",
                    style("Note:").yellow(),
                    integrator.skipped_intervals,
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

    /// The stats/integral reset used to watch the unit alone. `--unit` pins
    /// the label, so a dial turn from volts to ohms kept one series and
    /// averaged the two quantities together in silence.
    #[test]
    fn series_change_watches_the_mode_as_well_as_the_unit() {
        assert_eq!(series_change(("DC V", "V"), ("DC V", "V")), None);
        assert_eq!(
            series_change(("DC V", "A"), ("Resistance", "A")).as_deref(),
            Some("Mode changed (DC V \u{2192} Resistance)")
        );
        assert_eq!(
            series_change(("DC V", "mV"), ("DC V", "V")).as_deref(),
            Some("Unit changed (mV \u{2192} V)")
        );
        // Both moved: name the mode, the change the user made.
        assert_eq!(
            series_change(("DC V", "mV"), ("Resistance", "k\u{3a9}")).as_deref(),
            Some("Mode changed (DC V \u{2192} Resistance)")
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout {
                family: 2,
                extra: 0,
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
            format::csv_header(false, 2).split(',').count(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
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
            format::AuxLayout::default(),
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!((parsed["value"].as_f64().unwrap() - (-12.345)).abs() < 1e-6);
    }
}
