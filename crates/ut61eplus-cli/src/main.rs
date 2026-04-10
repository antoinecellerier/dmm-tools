mod capture;
mod format;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use console::style;
use log::{error, info};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use ut61eplus_lib::measurement::MeasuredValue;
use ut61eplus_lib::protocol::registry::{self, SelectableDevice};

fn version_string() -> &'static str {
    let version = env!("CARGO_PKG_VERSION");
    let hash = env!("GIT_HASH");
    if version.contains("-dev") {
        Box::leak(format!("{version} ({hash})").into_boxed_str())
    } else {
        version
    }
}

#[derive(Parser)]
#[command(
    name = "ut61eplus",
    version = version_string(),
    about = "UNI-T UT61E+ multimeter tool",
    after_help = "Run with --help for the full list of supported devices.\n\nSet NO_COLOR=1 to disable colored output.\nHelp / GitHub: https://github.com/antoinecellerier/dmm-tools",
    after_long_help = "Set NO_COLOR=1 to disable colored output.\nHelp / GitHub: https://github.com/antoinecellerier/dmm-tools"
)]
struct Cli {
    /// Device to connect to [ut61eplus, ut8803, ut171, ut181a, mock, ...]
    #[arg(long, default_value = "ut61eplus")]
    device: String,

    /// Select a specific USB adapter when multiple are connected.
    /// Use serial number or HID device path from 'ut61eplus list' output.
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
  bash:  ut61eplus completions bash > ~/.local/share/bash-completion/completions/ut61eplus
  zsh:   ut61eplus completions zsh > ~/.zfunc/_ut61eplus
  fish:  ut61eplus completions fish > ~/.config/fish/completions/ut61eplus.fish
  pwsh:  ut61eplus completions powershell >> $PROFILE")]
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

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Csv,
    Json,
}

fn main() {
    env_logger::init();

    // Build CLI with registry-generated --device long_help
    let mut cmd = Cli::command();
    let device_help = build_device_help();
    cmd = cmd.mut_arg("device", |a| a.long_help(device_help));
    let cli =
        Cli::from_arg_matches_mut(&mut cmd.get_matches()).unwrap_or_else(|e: clap::Error| e.exit());

    let device = match registry::resolve_device(&cli.device) {
        Some(d) => d,
        None => {
            eprintln!(
                "{} unknown device: {}",
                style("Error:").red().bold(),
                cli.device,
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
                        "ut61eplus",
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
            mock_mode,
        } if !device.requires_hardware => {
            cmd_read_mock(interval_ms, format, output, count, integrate, mock_mode)
        }
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
            mock_mode: _,
        } => cmd_read(
            device,
            adapter,
            interval_ms,
            format,
            output,
            count,
            integrate,
        ),
        Cmd::Command { action } => cmd_command(device, adapter, action),
        Cmd::Debug { count, interval_ms } => cmd_debug(device, adapter, count, interval_ms),
        Cmd::Capture {
            output,
            steps,
            list_steps,
        } => {
            if list_steps {
                capture::list_steps();
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
    let mut help = String::from("Device to connect to.\n\nDevices:\n");
    for d in registry::DEVICES {
        let stability = (d.new_protocol)().profile().stability;
        let tag = if !d.requires_hardware {
            " (no hardware required)"
        } else if stability == ut61eplus_lib::protocol::Stability::Experimental {
            " (experimental)"
        } else {
            ""
        };
        help.push_str(&format!("  {:<12} {}{}\n", d.id, d.display_name, tag));
    }
    help.push_str("\nAlso accepts aliases: ut61e+, ut61b, ut171a, ut181, etc.\nQuote names with special characters: --device 'ut61e+'");
    help
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
        eprintln!("Then unplug and replug the cable.");
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
) -> Result<
    ut61eplus_lib::Dmm<Box<dyn ut61eplus_lib::transport::Transport>>,
    Box<dyn std::error::Error>,
> {
    match ut61eplus_lib::open_device_by_id_auto(device.id, adapter) {
        Ok(dmm) => {
            let profile = dmm.profile();
            if profile.stability == ut61eplus_lib::protocol::Stability::Experimental {
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
                    style(format!("  ut61eplus --device {} capture", device.id)).yellow()
                );
                eprintln!(
                    "{}",
                    style(format!("Report feedback: {}", profile.feedback_url())).yellow()
                );
            }
            Ok(dmm)
        }
        Err(ut61eplus_lib::error::Error::DeviceNotFound { .. })
        | Err(ut61eplus_lib::error::Error::NoTransportFound) => {
            eprintln!("{}", style("USB cable not found.").yellow().bold());
            print_transport_setup_help();
            let proto = (device.new_protocol)();
            let profile = proto.profile();
            if profile.stability == ut61eplus_lib::protocol::Stability::Experimental {
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
        Err(ut61eplus_lib::error::Error::AdapterNotFound(ref detail)) => {
            eprintln!(
                "{} adapter not found: {detail}",
                style("Error:").red().bold()
            );
            eprintln!(
                "{}",
                style("Run 'ut61eplus list' to see connected devices.").yellow()
            );
            Err("adapter not found".into())
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let devices = ut61eplus_lib::list_devices()?;
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

fn cmd_read(
    device: &'static SelectableDevice,
    adapter: Option<&str>,
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
    count: usize,
    integrate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_with_help(device, adapter)?;
    let experimental = dmm.profile().stability == ut61eplus_lib::protocol::Stability::Experimental;
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
    )
}

fn cmd_read_mock(
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
    count: usize,
    integrate: bool,
    mock_mode: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = match mock_mode {
        Some(mode_str) => {
            let mode: ut61eplus_lib::mock::MockMode = mode_str
                .parse()
                .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;
            ut61eplus_lib::mock::open_mock_mode(mode)?
        }
        None => ut61eplus_lib::mock::open_mock()?,
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
    )
}

/// Shared measurement loop for both real and mock devices.
#[allow(clippy::too_many_arguments)]
fn run_read_loop<T: ut61eplus_lib::transport::Transport>(
    dmm: &mut ut61eplus_lib::Dmm<T>,
    interval_ms: u64,
    format: &OutputFormat,
    output_path: Option<String>,
    count: usize,
    experimental: bool,
    // When set, timeout warnings include device-specific activation instructions.
    device: Option<&'static SelectableDevice>,
    integrate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrlc()?;

    let mut writer: Box<dyn Write> = match &output_path {
        Some(path) => Box::new(std::fs::File::create(path).map(std::io::BufWriter::new)?),
        None => Box::new(std::io::stdout().lock()),
    };

    if matches!(format, OutputFormat::Csv) {
        if integrate {
            writeln!(
                writer,
                "timestamp,mode,value,unit,range,flags,integral,integral_unit"
            )?;
        } else {
            writeln!(writer, "timestamp,mode,value,unit,range,flags")?;
        }
    }

    let interval = Duration::from_millis(interval_ms);
    let mut stats = ut61eplus_lib::stats::RunningStats::default();
    let mut integrator = ut61eplus_lib::stats::Integrator::new();
    let mut integral_unit: Option<String> = None;
    let mut i = 0usize;
    let mut consecutive_timeouts: u32 = 0;

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match dmm.request_measurement() {
            Ok(m) => {
                consecutive_timeouts = 0;
                if let MeasuredValue::Normal(v) = &m.value {
                    stats.push(*v);
                }

                // Integration tracking
                let integral_display = if integrate {
                    let current_unit: &str = &m.unit;
                    if let Some(prev_unit) = &integral_unit
                        && prev_unit != current_unit
                    {
                        eprintln!(
                            "{} Unit changed ({prev_unit} \u{2192} {current_unit}), integral reset",
                            style("Note:").yellow(),
                        );
                        integrator.reset();
                    }
                    integral_unit = Some(current_unit.to_string());

                    match &m.value {
                        MeasuredValue::Normal(v) => integrator.push(*v, m.timestamp),
                        MeasuredValue::Overload => integrator.push_overload(),
                        _ => {}
                    }

                    ut61eplus_lib::stats::integral_unit_info(current_unit)
                        .map(|(disp_unit, divisor)| (integrator.value() / divisor, disp_unit))
                } else {
                    None
                };

                format::format_measurement(
                    &mut writer,
                    &m,
                    format,
                    experimental,
                    integral_display,
                )?;
                writer.flush()?;
                i += 1;
            }
            Err(ut61eplus_lib::error::Error::Timeout) => {
                consecutive_timeouts += 1;
                log::warn!("measurement timeout, retrying");
                if consecutive_timeouts == 5
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
            Err(e) => {
                return Err(e.into());
            }
        }
        if interval_ms > 0 && (count == 0 || i < count) {
            std::thread::sleep(interval);
        }
    }

    info!("shutting down");
    writer.flush()?;

    if stats.count > 0 {
        eprintln!(
            "\n{} {} samples | Min: {} | Max: {} | Avg: {}",
            style("---").dim(),
            stats.count,
            style(format!("{:.4}", stats.min.unwrap())).cyan(),
            style(format!("{:.4}", stats.max.unwrap())).cyan(),
            style(format!("{:.4}", stats.avg().unwrap())).cyan(),
        );
        if integrate
            && let Some(unit_str) = &integral_unit
            && let Some((disp_unit, divisor)) = ut61eplus_lib::stats::integral_unit_info(unit_str)
        {
            let dt_str = integrator
                .elapsed_secs()
                .map(|s| format!(" ({}s)", style(format!("{s:.1}")).cyan()))
                .unwrap_or_default();
            eprintln!(
                "    Integral: {} {disp_unit}{dt_str}",
                style(format!("{:.4}", integrator.value() / divisor)).cyan(),
            );
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
        let mut dmm = ut61eplus_lib::mock::open_mock()?;
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

    let interval = Duration::from_millis(interval_ms);
    let mut i = 0;

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match dmm.request_measurement() {
            Ok(m) => {
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
        if count == 0 || i < count {
            std::thread::sleep(interval);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ut61eplus_lib::protocol::ut61eplus::make_test_measurement;

    #[test]
    fn clap_parse_list() {
        let cli = Cli::try_parse_from(["ut61eplus", "list"]).unwrap();
        assert!(matches!(cli.command, Cmd::List));
    }

    #[test]
    fn clap_parse_read_defaults() {
        let cli = Cli::try_parse_from(["ut61eplus", "read"]).unwrap();
        match cli.command {
            Cmd::Read {
                interval_ms,
                format,
                output,
                count,
                integrate,
                mock_mode,
            } => {
                assert_eq!(interval_ms, 0);
                assert!(matches!(format, OutputFormat::Text));
                assert!(output.is_none());
                assert_eq!(count, 0);
                assert!(!integrate);
                assert!(mock_mode.is_none());
            }
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_read_with_args() {
        let cli = Cli::try_parse_from([
            "ut61eplus",
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
            } => {
                assert_eq!(interval_ms, 100);
                assert!(matches!(format, OutputFormat::Csv));
                assert_eq!(output.as_deref(), Some("test.csv"));
                assert_eq!(count, 10);
            }
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_command() {
        let cli = Cli::try_parse_from(["ut61eplus", "command", "hold"]).unwrap();
        match cli.command {
            Cmd::Command { action } => {
                assert_eq!(action.as_deref(), Some("hold"));
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn clap_parse_command_no_action_lists_commands() {
        let cli = Cli::try_parse_from(["ut61eplus", "command"]).unwrap();
        match cli.command {
            Cmd::Command { action } => {
                assert!(action.is_none());
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn clap_parse_debug() {
        let cli = Cli::try_parse_from(["ut61eplus", "debug", "--count", "5"]).unwrap();
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
        let cli = Cli::try_parse_from(["ut61eplus", "--device", "ut8803", "list"]).unwrap();
        assert_eq!(cli.device, "ut8803");
    }

    #[test]
    fn format_text_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Text, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Csv, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let fields: Vec<&str> = output.trim().split(',').collect();
        assert!(fields.len() >= 6);
        assert_eq!(fields[1], "DC V");
        assert_eq!(fields[2], "5.678");
        assert_eq!(fields[3], "V");
        assert_eq!(fields[4], "22V");
    }

    #[test]
    fn format_json_output() {
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on, inverted logic)
        let m = make_test_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json, false, None).unwrap();
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
        format::format_measurement(&mut buf, &m, &OutputFormat::Json, true, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["experimental"], true);
    }

    #[test]
    fn format_csv_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Csv, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_test_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"], "OL");
    }

    #[test]
    fn clap_parse_completions() {
        let cli = Cli::try_parse_from(["ut61eplus", "completions", "bash"]).unwrap();
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
        format::format_measurement(&mut buf, &m, &OutputFormat::Csv, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("NCV:3"));
    }

    #[test]
    fn format_json_ncv() {
        let m = make_test_measurement(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"]["ncv_level"], 3);
        assert_eq!(parsed["mode"], "NCV");
    }

    #[test]
    fn format_text_includes_flags() {
        let m = make_test_measurement(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x0F, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Text, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("HOLD"));
        assert!(output.contains("REL"));
    }

    #[test]
    fn format_json_negative_value() {
        let m = make_test_measurement(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json, false, None).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!((parsed["value"].as_f64().unwrap() - (-12.345)).abs() < 1e-6);
    }
}
