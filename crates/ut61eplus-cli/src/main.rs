use clap::{Parser, Subcommand, ValueEnum};
use log::{error, info};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ut61eplus_lib::command::Command;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

#[derive(Parser)]
#[command(name = "ut61eplus", about = "UNI-T UT61E+ multimeter tool")]
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
        /// Interval between readings in milliseconds
        #[arg(long, default_value = "500")]
        interval_ms: u64,
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
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
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Csv,
    Json,
}

#[derive(Clone, ValueEnum)]
enum ButtonAction {
    Hold,
    MinMax,
    Rel,
    Range,
    Select,
    Light,
}

impl ButtonAction {
    fn to_command(&self) -> Command {
        match self {
            ButtonAction::Hold => Command::Hold,
            ButtonAction::MinMax => Command::MinMax,
            ButtonAction::Rel => Command::Rel,
            ButtonAction::Range => Command::Range,
            ButtonAction::Select => Command::Select,
            ButtonAction::Light => Command::Light,
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
        } => cmd_read(interval_ms, format, output),
        Cmd::Command { action } => cmd_command(action),
        Cmd::Debug { count, interval_ms } => cmd_debug(count, interval_ms),
    };

    if let Err(e) = result {
        error!("{e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let devices = ut61eplus_lib::list_devices()?;
    if devices.is_empty() {
        eprintln!("No UT61E+ devices found.");
        eprintln!("Check USB connection and udev rules.");
        return Ok(());
    }
    for (i, dev) in devices.iter().enumerate() {
        println!("[{i}] {dev}");
    }
    Ok(())
}

fn cmd_info() -> Result<(), Box<dyn std::error::Error>> {
    let _dmm = ut61eplus_lib::open()?;
    println!("Connected to UNI-T UT61E+");
    println!("UART initialized (9600/8N1)");
    Ok(())
}

fn cmd_read(
    interval_ms: u64,
    format: OutputFormat,
    output_path: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut dmm = ut61eplus_lib::open()?;
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

    while running.load(Ordering::SeqCst) {
        match dmm.request_measurement() {
            Ok(m) => {
                format_measurement(&mut writer, &m, &format)?;
                writer.flush()?;
            }
            Err(ut61eplus_lib::error::Error::Timeout) => {
                log::warn!("measurement timeout, retrying");
            }
            Err(e) => {
                return Err(e.into());
            }
        }
        std::thread::sleep(interval);
    }

    info!("shutting down");
    writer.flush()?;
    Ok(())
}

fn format_measurement(
    w: &mut dyn Write,
    m: &Measurement,
    format: &OutputFormat,
) -> std::io::Result<()> {
    match format {
        OutputFormat::Text => {
            writeln!(w, "{m}")
        }
        OutputFormat::Csv => {
            let value_str = match &m.value {
                MeasuredValue::Normal(v) => v.to_string(),
                MeasuredValue::Overload => "OL".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
            };
            writeln!(
                w,
                "{},{},{},{},{},{}",
                chrono::Local::now().to_rfc3339(),
                m.mode,
                value_str,
                m.unit,
                m.range_label,
                m.flags,
            )
        }
        OutputFormat::Json => {
            let value = match &m.value {
                MeasuredValue::Normal(v) => serde_json::json!(v),
                MeasuredValue::Overload => serde_json::json!("OL"),
                MeasuredValue::NcvLevel(l) => serde_json::json!({"ncv_level": l}),
            };
            let obj = serde_json::json!({
                "timestamp": chrono::Local::now().to_rfc3339(),
                "mode": m.mode.to_string(),
                "value": value,
                "unit": m.unit,
                "range": m.range_label,
                "display_raw": m.display_raw,
                "progress": m.progress,
                "flags": {
                    "hold": m.flags.hold,
                    "rel": m.flags.rel,
                    "auto_range": m.flags.auto_range,
                    "min": m.flags.min,
                    "max": m.flags.max,
                    "low_battery": m.flags.low_battery,
                }
            });
            writeln!(w, "{}", serde_json::to_string(&obj).unwrap())
        }
    }
}

fn cmd_command(action: ButtonAction) -> Result<(), Box<dyn std::error::Error>> {
    let mut dmm = ut61eplus_lib::open()?;
    let cmd = action.to_command();
    dmm.send_command(cmd)?;
    println!("Sent {cmd:?}");
    Ok(())
}

fn cmd_debug(count: usize, interval_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut dmm = ut61eplus_lib::open()?;
    let interval = Duration::from_millis(interval_ms);
    let mut i = 0;

    while running.load(Ordering::SeqCst) && (count == 0 || i < count) {
        match dmm.request_measurement() {
            Ok(m) => {
                println!(
                    "[{i}] mode={:02X} range={:02X} display={:?} progress={} flags={} → {}",
                    m.mode as u8,
                    m.range,
                    m.display_raw,
                    m.progress,
                    m.flags,
                    m,
                );
            }
            Err(e) => {
                eprintln!("[{i}] error: {e}");
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
            mode | 0x30,
            range | 0x30,
            display[0], display[1], display[2], display[3],
            display[4], display[5], display[6],
            progress.0 | 0x30,
            progress.1 | 0x30,
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
            Cmd::Read { interval_ms, format, output } => {
                assert_eq!(interval_ms, 500);
                assert!(matches!(format, OutputFormat::Text));
                assert!(output.is_none());
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
        ]).unwrap();
        match cli.command {
            Cmd::Read { interval_ms, format, output } => {
                assert_eq!(interval_ms, 100);
                assert!(matches!(format, OutputFormat::Csv));
                assert_eq!(output.as_deref(), Some("test.csv"));
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
        let m = make_measurement(0x00, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x01, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Text).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_measurement(0x00, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x01, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Csv).unwrap();
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
        let m = make_measurement(0x00, 0x01, b"  5.678", (0x00, 0x00), (0x01, 0x01, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Json).unwrap();
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
        let m = make_measurement(0x04, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x01, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Csv).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_measurement(0x04, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x01, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Json).unwrap();
        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["value"], "OL");
    }

    #[test]
    fn button_action_to_command() {
        assert_eq!(ButtonAction::Hold.to_command(), Command::Hold);
        assert_eq!(ButtonAction::MinMax.to_command(), Command::MinMax);
        assert_eq!(ButtonAction::Rel.to_command(), Command::Rel);
        assert_eq!(ButtonAction::Range.to_command(), Command::Range);
        assert_eq!(ButtonAction::Select.to_command(), Command::Select);
        assert_eq!(ButtonAction::Light.to_command(), Command::Light);
    }
}
