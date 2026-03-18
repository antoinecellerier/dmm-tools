mod capture;
mod format;

use clap::{Parser, Subcommand, ValueEnum};
use console::style;
use log::{error, info};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ut61eplus_lib::command::Command;
use ut61eplus_lib::measurement::MeasuredValue;

#[derive(Parser)]
#[command(name = "ut61eplus", about = "UNI-T UT61E+ multimeter tool", after_help = "Set NO_COLOR=1 to disable colored output.\n\nHelp / GitHub: https://github.com/antoinecellerier/dmm-tools")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List connected UT61E+ devices
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
    },
    /// Send a button press command to the meter
    Command {
        /// The command to send
        #[arg(value_enum)]
        action: ButtonAction,
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

#[derive(Clone, ValueEnum)]
enum ButtonAction {
    Hold,
    MinMax,
    ExitMinMax,
    Rel,
    Range,
    Auto,
    Select,
    Select2,
    Light,
    PeakMinMax,
    ExitPeak,
}

impl ButtonAction {
    fn to_command(&self) -> Command {
        match self {
            ButtonAction::Hold => Command::Hold,
            ButtonAction::MinMax => Command::MinMax,
            ButtonAction::ExitMinMax => Command::ExitMinMax,
            ButtonAction::Rel => Command::Rel,
            ButtonAction::Range => Command::Range,
            ButtonAction::Auto => Command::Auto,
            ButtonAction::Select => Command::Select,
            ButtonAction::Select2 => Command::Select2,
            ButtonAction::Light => Command::Light,
            ButtonAction::PeakMinMax => Command::PeakMinMax,
            ButtonAction::ExitPeak => Command::ExitPeak,
        }
    }
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let result = match cli.command {
        Cmd::List => cmd_list(),
        Cmd::Info => cmd_info(),
        Cmd::Read {
            interval_ms,
            format,
            output,
            count,
        } => cmd_read(interval_ms, format, output, count),
        Cmd::Command { action } => cmd_command(action),
        Cmd::Debug { count, interval_ms } => cmd_debug(count, interval_ms),
        Cmd::Capture { output, steps, list_steps } => {
            if list_steps {
                capture::list_steps();
                Ok(())
            } else {
                open_with_help().and_then(|dmm| capture::cmd_capture(output, steps, dmm))
            }
        }
    };

    if let Err(e) = result {
        error!("{e}");
        eprintln!("{} {e}", style("Error:").red().bold());
        std::process::exit(1);
    }
}

/// Open the meter with helpful error messages for common failures.
fn open_with_help() -> Result<ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>, Box<dyn std::error::Error>> {
    match ut61eplus_lib::open() {
        Ok(dmm) => Ok(dmm),
        Err(ut61eplus_lib::error::Error::DeviceNotFound { .. }) => {
            eprintln!("{}", style("USB adapter not found.").yellow().bold());
            eprintln!("Check that the CP2110 USB adapter is plugged in.");
            #[cfg(target_os = "linux")]
            {
                eprintln!("On Linux, ensure the udev rule is installed:");
                eprintln!("  {}", style("sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/").dim());
                eprintln!("  {}", style("sudo udevadm control --reload-rules").dim());
            }
            #[cfg(target_os = "windows")]
            {
                eprintln!("On Windows, ensure the CP2110 driver is installed.");
                eprintln!("Download from: {}", style("https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers").dim());
            }
            Err("device not found".into())
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let devices = ut61eplus_lib::list_devices()?;
    if devices.is_empty() {
        eprintln!("{}", style("No UT61E+ devices found.").yellow());
        eprintln!("Check USB connection and udev rules.");
        return Ok(());
    }
    for (i, dev) in devices.iter().enumerate() {
        println!("{} {dev}", style(format!("[{i}]")).cyan());
    }
    Ok(())
}

fn cmd_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_with_help()?;
    let name = dmm.get_name()?;
    println!("Device: {}", style(name).bold());
    Ok(())
}

fn cmd_read(
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut dmm = open_with_help()?;
    info!("connected, starting measurement loop");

    let mut writer: Box<dyn Write> = match &output_path {
        Some(path) => Box::new(
            std::fs::File::create(path)
                .map(std::io::BufWriter::new)?,
        ),
        None => Box::new(std::io::stdout().lock()),
    };

    // Write CSV header
    if matches!(format, OutputFormat::Csv) {
        writeln!(writer, "timestamp,mode,value,unit,range,flags")?;
    }

    let interval = Duration::from_millis(interval_ms);
    let mut stats = ReadStats::default();
    let mut i = 0usize;

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match dmm.request_measurement() {
            Ok(m) => {
                if let MeasuredValue::Normal(v) = &m.value {
                    stats.push(*v);
                }
                format::format_measurement(&mut writer, &m, &format)?;
                writer.flush()?;
                i += 1;
            }
            Err(ut61eplus_lib::error::Error::Timeout) => {
                log::warn!("measurement timeout, retrying");
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

    // Print stats summary to stderr (doesn't interfere with piped output)
    if stats.count > 0 {
        eprintln!(
            "\n{} {} samples | Min: {} | Max: {} | Avg: {}",
            style("---").dim(),
            stats.count,
            style(format!("{:.4}", stats.min)).cyan(),
            style(format!("{:.4}", stats.max)).cyan(),
            style(format!("{:.4}", stats.sum / stats.count as f64)).cyan(),
        );
    }
    Ok(())
}

#[derive(Default)]
struct ReadStats {
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
}

impl ReadStats {
    fn push(&mut self, v: f64) {
        if self.count == 0 {
            self.min = v;
            self.max = v;
        } else {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
        self.sum += v;
        self.count += 1;
    }
}

fn cmd_command(action: ButtonAction) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = open_with_help()?;
    let cmd = action.to_command();
    dmm.send_command(cmd)?;
    println!("{} {cmd:?}", style("Sent").green());
    Ok(())
}

fn cmd_debug(count: usize, interval_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut dmm = open_with_help()?;
    let interval = Duration::from_millis(interval_ms);
    let mut i = 0;

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match dmm.request_measurement() {
            Ok(m) => {
                println!(
                    "{} mode={:02X} range={:02X} display={:?} progress={} flags={} \u{2192} {}",
                    style(format!("[{i}]")).dim(),
                    m.mode as u8,
                    m.range,
                    m.display_raw,
                    m.progress,
                    m.flags,
                    style(format!("{m}")).green(),
                );
            }
            Err(e) => {
                eprintln!("{} {}", style(format!("[{i}]")).dim(), style(format!("error: {e}")).red());
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
    use ut61eplus_lib::measurement::Measurement;
    use ut61eplus_lib::tables::ut61e_plus::Ut61ePlusTable;

    /// Build a 14-byte payload and parse it into a Measurement.
    fn make_measurement(
        mode: u8,
        range: u8,
        display: &[u8; 7],
        progress: (u8, u8),
        flags: (u8, u8, u8),
    ) -> Measurement {
        let payload: Vec<u8> = vec![
            mode,               // raw, no 0x30 prefix
            range | 0x30,
            display[0], display[1], display[2], display[3],
            display[4], display[5], display[6],
            progress.0,         // raw, no 0x30 prefix
            progress.1,         // raw, no 0x30 prefix
            flags.0 | 0x30,
            flags.1 | 0x30,
            flags.2 | 0x30,
        ];
        let table = Ut61ePlusTable::new();
        Measurement::parse(&payload, &table).unwrap()
    }

    #[test]
    fn clap_parse_list() {
        let cli = Cli::try_parse_from(["ut61eplus", "list"]).unwrap();
        assert!(matches!(cli.command, Cmd::List));
    }

    #[test]
    fn clap_parse_read_defaults() {
        let cli = Cli::try_parse_from(["ut61eplus", "read"]).unwrap();
        match cli.command {
            Cmd::Read { interval_ms, format, output, count } => {
                assert_eq!(interval_ms, 0);
                assert!(matches!(format, OutputFormat::Text));
                assert!(output.is_none());
                assert_eq!(count, 0);
            }
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_read_with_args() {
        let cli = Cli::try_parse_from([
            "ut61eplus", "read",
            "--interval-ms", "100",
            "--format", "csv",
            "-o", "test.csv",
            "--count", "10",
        ]).unwrap();
        match cli.command {
            Cmd::Read { interval_ms, format, output, count } => {
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
                assert!(matches!(action, ButtonAction::Hold));
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
    fn format_text_output() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Text).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Csv).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // CSV has: timestamp,mode,value,unit,range,flags
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
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["mode"], "DC V");
        assert_eq!(parsed["value"], 5.678);
        assert_eq!(parsed["unit"], "V");
        assert_eq!(parsed["flags"]["hold"], true);
        assert_eq!(parsed["flags"]["auto_range"], true);
    }

    #[test]
    fn format_csv_overload() {
        let m = make_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Csv).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format::format_measurement(&mut buf, &m, &OutputFormat::Json).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"], "OL");
    }

    #[test]
    fn button_action_to_command() {
        assert_eq!(ButtonAction::Hold.to_command(), Command::Hold);
        assert_eq!(ButtonAction::MinMax.to_command(), Command::MinMax);
        assert_eq!(ButtonAction::ExitMinMax.to_command(), Command::ExitMinMax);
        assert_eq!(ButtonAction::Rel.to_command(), Command::Rel);
        assert_eq!(ButtonAction::Range.to_command(), Command::Range);
        assert_eq!(ButtonAction::Auto.to_command(), Command::Auto);
        assert_eq!(ButtonAction::Select.to_command(), Command::Select);
        assert_eq!(ButtonAction::Select2.to_command(), Command::Select2);
        assert_eq!(ButtonAction::Light.to_command(), Command::Light);
        assert_eq!(ButtonAction::PeakMinMax.to_command(), Command::PeakMinMax);
        assert_eq!(ButtonAction::ExitPeak.to_command(), Command::ExitPeak);
    }
}
