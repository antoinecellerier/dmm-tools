use clap::{Parser, Subcommand, ValueEnum};
use console::style;
use log::{error, info};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use ut61eplus_lib::command::Command;
use ut61eplus_lib::measurement::{MeasuredValue, Measurement};

#[derive(Parser)]
#[command(name = "ut61eplus", about = "UNI-T UT61E+ multimeter tool", after_help = "Set NO_COLOR=1 to disable colored output.")]
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
enum OutputFormat {
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
        Cmd::Capture { output, steps, list_steps } => cmd_capture(output, steps, list_steps),
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
            eprintln!("On Linux, ensure the udev rule is installed:");
            eprintln!("  {}", style("sudo cp udev/99-cp2110-unit.rules /etc/udev/rules.d/").dim());
            eprintln!("  {}", style("sudo udevadm control --reload-rules").dim());
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
                format_measurement(&mut writer, &m, &format)?;
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
                    "hv_warning": m.flags.hv_warning,
                    "dc": m.flags.dc,
                    "peak_min": m.flags.peak_min,
                    "peak_max": m.flags.peak_max,
                }
            });
            writeln!(w, "{}", serde_json::to_string(&obj).unwrap())
        }
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
                    "{} mode={:02X} range={:02X} display={:?} progress={} flags={} → {}",
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

// --- Guided capture ---

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct CaptureReport {
    date: String,
    tool_version: String,
    device_name: String,
    supported: bool,
    steps: Vec<StepResult>,
}

#[derive(Serialize, Deserialize, Clone)]
struct StepResult {
    id: String,
    instruction: String,
    status: String, // "captured", "skipped", "timeout", "error"
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    samples: Vec<SampleData>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    screen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SampleData {
    /// Raw 14-byte payload as hex string (e.g. "02 30 20 30 2E 30 30 30 30 00 00 30 30 30")
    raw_hex: String,
    mode_byte: String,
    mode: String,
    range_byte: String,
    display_raw: String,
    value: String,
    unit: String,
    range_label: String,
    progress: u16,
    flags: SampleFlags,
}

#[derive(Serialize, Deserialize, Clone)]
struct SampleFlags {
    hold: bool,
    rel: bool,
    auto_range: bool,
    min: bool,
    max: bool,
    low_battery: bool,
    hv_warning: bool,
    dc: bool,
    peak_min: bool,
    peak_max: bool,
}

impl SampleData {
    fn from_measurement(m: &Measurement) -> Self {
        let value = match &m.value {
            MeasuredValue::Normal(v) => format!("{v}"),
            MeasuredValue::Overload => "OL".to_string(),
            MeasuredValue::NcvLevel(l) => format!("NCV:{l}"),
        };
        let raw_hex = m.raw_payload.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            raw_hex,
            mode_byte: format!("{:#04x}", m.mode as u8),
            mode: m.mode.to_string(),
            range_byte: format!("{:#04x}", m.range),
            display_raw: m.display_raw.clone(),
            value,
            unit: m.unit.clone(),
            range_label: m.range_label.clone(),
            progress: m.progress,
            flags: SampleFlags {
                hold: m.flags.hold,
                rel: m.flags.rel,
                auto_range: m.flags.auto_range,
                min: m.flags.min,
                max: m.flags.max,
                low_battery: m.flags.low_battery,
                hv_warning: m.flags.hv_warning,
                dc: m.flags.dc,
                peak_min: m.flags.peak_min,
                peak_max: m.flags.peak_max,
            },
        }
    }

    fn summary(&self) -> String {
        let mut flag_parts = Vec::new();
        if self.flags.auto_range { flag_parts.push("AUTO"); }
        if self.flags.hold { flag_parts.push("HOLD"); }
        if self.flags.rel { flag_parts.push("REL"); }
        if self.flags.min { flag_parts.push("MIN"); }
        if self.flags.max { flag_parts.push("MAX"); }
        format!("{} {} [{}]", self.display_raw.trim(), self.unit, flag_parts.join(" "))
    }
}

/// Definition of a capture step.
struct CaptureStep {
    id: &'static str,
    instruction: &'static str,
    command: Option<Command>,
    samples: usize,
}

fn prompt(msg: &str) -> String {
    eprint!("{msg}");
    std::io::stderr().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn prompt_key(msg: &str) -> char {
    let term = console::Term::stderr();
    eprint!("{msg}");
    std::io::stderr().flush().unwrap();
    let ch = term.read_char().unwrap_or('\n');
    eprintln!();
    ch
}

fn capture_samples(
    dmm: &mut ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
    n: usize,
) -> Vec<Measurement> {
    let mut samples = Vec::new();
    let mut attempts = 0;
    while samples.len() < n && attempts < n * 5 {
        match dmm.request_measurement() {
            Ok(m) => samples.push(m),
            Err(ut61eplus_lib::error::Error::Timeout) => {}
            Err(e) => {
                eprintln!("  error: {e}");
                break;
            }
        }
        attempts += 1;
    }
    samples
}

fn save_report(report: &CaptureReport, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(report)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

/// Insert or replace a step result in the report.
fn upsert_step(report: &mut CaptureReport, result: StepResult) {
    if let Some(pos) = report.steps.iter().position(|s| s.id == result.id) {
        report.steps[pos] = result;
    } else {
        report.steps.push(result);
    }
}

/// Run one capture step. Returns true if user wants to quit.
fn run_capture_step(
    dmm: &mut ut61eplus_lib::Dmm<ut61eplus_lib::cp2110::Cp2110>,
    step: &CaptureStep,
    report: &mut CaptureReport,
    interactive: bool,
) -> bool {
    // Check if already captured (resume)
    if report.steps.iter().any(|s| s.id == step.id && s.status == "captured") {
        eprintln!("  {} already captured, skipping", style(step.id).dim());
        return false;
    }

    if interactive {
        eprintln!();
        eprintln!("{} {}", style(format!("[{}]", step.id)).cyan().bold(), step.instruction);
        let ch = prompt_key(&format!("  {} ", style("any key=capture, s=skip, q=finish:").dim()));
        if ch == 'q' || ch == 'Q' {
            upsert_step(report, StepResult {
                id: step.id.to_string(),
                instruction: step.instruction.to_string(),
                status: "skipped".to_string(),
                samples: vec![],
                screen: None,
                error: None,
            });
            return true;
        }
        if ch == 's' || ch == 'S' {
            upsert_step(report, StepResult {
                id: step.id.to_string(),
                instruction: step.instruction.to_string(),
                status: "skipped".to_string(),
                samples: vec![],
                screen: None,
                error: None,
            });
            return false;
        }
    } else {
        eprintln!("{} {}", style(format!("[{}]", step.id)).cyan().bold(), step.instruction);
    }

    if let Some(cmd) = step.command {
        if let Err(e) = dmm.send_command(cmd) {
            eprintln!("  {}", style(format!("Command failed: {e}")).red());
            upsert_step(report, StepResult {
                id: step.id.to_string(),
                instruction: step.instruction.to_string(),
                status: "error".to_string(),
                samples: vec![],
                screen: None,
                error: Some(e.to_string()),
            });
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let raw_samples = capture_samples(dmm, step.samples);
    let sample_data: Vec<SampleData> = raw_samples.iter().map(SampleData::from_measurement).collect();

    for (i, s) in sample_data.iter().enumerate() {
        eprintln!(
            "    {} mode={}({}) range={} display={:?}",
            style(format!("[{i}]")).dim(), s.mode_byte, s.mode, s.range_byte, s.display_raw
        );
    }

    let screen = if interactive && !sample_data.is_empty() {
        let summary = sample_data.last().unwrap().summary();
        eprintln!("  We read: {}", style(&summary).green());
        let input = prompt(&format!(
            "  {} ", style("Enter=correct, or type what the meter actually shows:").dim()
        ));
        if input.is_empty() {
            Some(format!("confirmed: {summary}"))
        } else {
            Some(input)
        }
    } else if sample_data.is_empty() {
        eprintln!("  {}", style("No response from meter.").yellow());
        None
    } else {
        None
    };

    let status = if sample_data.is_empty() {
        "timeout"
    } else {
        "captured"
    };

    let result = StepResult {
        id: step.id.to_string(),
        instruction: step.instruction.to_string(),
        status: status.to_string(),
        samples: sample_data,
        screen,
        error: None,
    };

    upsert_step(report, result);
    false
}

fn all_capture_steps() -> Vec<CaptureStep> {
    vec![
        // Part 1: Modes
        CaptureStep { id: "dcv", instruction: "Set meter to DC V (V⎓). Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "dcv_short", instruction: "DC V mode: touch the two probe tips together.", command: None, samples: 3 },
        CaptureStep { id: "acv", instruction: "Set meter to AC V (V~). Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "dcmv", instruction: "Set meter to DC mV. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "ohm", instruction: "Set meter to Ω. Leave leads open (should show OL).", command: None, samples: 3 },
        CaptureStep { id: "ohm_short", instruction: "Ω mode: touch the two probe tips together.", command: None, samples: 3 },
        CaptureStep { id: "continuity", instruction: "Set meter to continuity (buzzer). Touch probes together.", command: None, samples: 3 },
        CaptureStep { id: "diode", instruction: "Set meter to diode. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "capacitance", instruction: "Set meter to capacitance (F). Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "hz", instruction: "Set meter to Hz. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "duty", instruction: "Hz mode: press USB/Hz to switch to duty cycle (%).", command: None, samples: 3 },
        CaptureStep { id: "ncv", instruction: "Set meter to NCV. Hold near a live wire if possible.", command: None, samples: 3 },
        CaptureStep { id: "hfe", instruction: "Set meter to hFE. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "dcua", instruction: "Set meter to µA. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "dcma", instruction: "Set meter to mA. Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "dca", instruction: "Set meter to A (if available). Leave leads open.", command: None, samples: 3 },
        CaptureStep { id: "temp", instruction: "Set meter to Temperature (if K-type thermocouple available).", command: None, samples: 3 },
        // Part 2: Flags
        CaptureStep { id: "hold", instruction: "Sending HOLD command.", command: Some(Command::Hold), samples: 3 },
        CaptureStep { id: "hold_off", instruction: "Toggling HOLD off.", command: Some(Command::Hold), samples: 3 },
        CaptureStep { id: "rel", instruction: "Sending REL command.", command: Some(Command::Rel), samples: 3 },
        CaptureStep { id: "rel_off", instruction: "Toggling REL off.", command: Some(Command::Rel), samples: 3 },
        CaptureStep { id: "minmax", instruction: "Sending MIN/MAX command.", command: Some(Command::MinMax), samples: 3 },
        CaptureStep { id: "minmax_off", instruction: "Exiting MIN/MAX.", command: Some(Command::ExitMinMax), samples: 3 },
        CaptureStep { id: "range", instruction: "Sending RANGE (manual range).", command: Some(Command::Range), samples: 3 },
        CaptureStep { id: "auto", instruction: "Sending AUTO (restore auto-range).", command: Some(Command::Auto), samples: 3 },
        // Part 3: Range cycle
        CaptureStep { id: "range_cycle", instruction: "Cycle through manual ranges on DC V.", command: None, samples: 0 },
    ]
}

/// IDs for part 1 (modes) vs part 2 (flags) grouping
const MODE_STEP_IDS: &[&str] = &[
    "dcv", "dcv_short", "acv", "dcmv", "ohm", "ohm_short", "continuity",
    "diode", "capacitance", "hz", "duty", "ncv", "hfe", "dcua", "dcma", "dca", "temp",
];
const FLAG_STEP_IDS: &[&str] = &[
    "hold", "hold_off", "rel", "rel_off", "minmax", "minmax_off", "range", "auto",
];

fn cmd_capture(output_override: Option<String>, filter: Option<Vec<String>>, list_steps: bool) -> Result<(), Box<dyn std::error::Error>> {
    if list_steps {
        let steps = all_capture_steps();
        eprintln!("{}", style("Available capture steps:").bold());
        eprintln!();
        eprintln!("{}", style("  Measurement modes:").cyan());
        for s in &steps {
            if MODE_STEP_IDS.contains(&s.id) {
                eprintln!("    {:<16} {}", style(s.id).bold(), s.instruction);
            }
        }
        eprintln!();
        eprintln!("{}", style("  Flags & commands:").cyan());
        for s in &steps {
            if FLAG_STEP_IDS.contains(&s.id) {
                eprintln!("    {:<16} {}", style(s.id).bold(), s.instruction);
            }
        }
        eprintln!();
        eprintln!("{}", style("  Other:").cyan());
        eprintln!("    {:<16} {}", style("range_cycle").bold(), "Cycle through manual ranges on DC V");
        eprintln!("    {:<16} {}", style("extra_N").bold(), "Freeform captures (Part 4)");
        eprintln!();
        eprintln!("Usage: {} {}", style("ut61eplus capture --steps").dim(), style("dcmv,temp,duty").dim());
        return Ok(());
    }

    let step_filter: Option<std::collections::HashSet<String>> = filter.map(|v| v.into_iter().collect());
    eprintln!("{}", style("=== UT61E+ Protocol Capture Tool ===").bold().cyan());
    eprintln!("This tool walks you through a series of steps to capture protocol");
    eprintln!("data from your meter. The output can be shared in bug reports.");
    eprintln!();

    let mut dmm = open_with_help()?;

    // Verify the meter is actually responding before proceeding
    eprintln!("{}", style("Checking meter communication...").dim());
    let device_name = match dmm.get_name() {
        Ok(name) => name,
        Err(_) => {
            // get_name failed — try a plain measurement as fallback
            match dmm.request_measurement() {
                Ok(_) => "unknown".to_string(),
                Err(_) => {
                    eprintln!();
                    eprintln!("{}", style("USB adapter found but the meter is not responding.").yellow().bold());
                    eprintln!("To enable data transmission on the UT61E+:");
                    eprintln!("  1. Insert the USB module into the meter");
                    eprintln!("  2. Turn the meter on");
                    eprintln!("  3. Long press the USB/Hz button");
                    eprintln!("  4. The S icon appears on the LCD");
                    eprintln!();
                    eprintln!("Then run this command again.");
                    return Err("meter not responding".into());
                }
            }
        }
    };

    let supported = device_name.contains("UT61E+");
    eprintln!("Device: {}", style(&device_name).bold());
    if supported {
        eprintln!("Status: {}", style("supported model").green());
    } else {
        eprintln!("Status: {}", style("UNKNOWN MODEL — captures are especially valuable!").yellow().bold());
        eprintln!("        Protocol may differ from the UT61E+. Please complete");
        eprintln!("        as many steps as possible and share the report.");
    }
    eprintln!();

    // Determine output path: explicit override, or auto-name from device
    let slug = device_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>();
    let auto_path = format!("capture-{slug}.yaml");
    let output_path = output_override.unwrap_or(auto_path);

    // Check for existing file — auto-resume or prompt to overwrite
    let mut report = match std::fs::read_to_string(&output_path) {
        Ok(contents) => {
            match serde_yaml::from_str::<CaptureReport>(&contents) {
                Ok(r) => {
                    let captured = r.steps.iter().filter(|s| s.status == "captured").count();
                    let skipped = r.steps.iter().filter(|s| s.status == "skipped").count();
                    eprintln!(
                        "Found existing capture: {output_path} ({captured} captured, {skipped} skipped)"
                    );
                    let ch = prompt_key("r=resume, n=start fresh, q=abort: ");
                    if ch == 'q' || ch == 'Q' {
                        eprintln!("Aborted.");
                        return Ok(());
                    }
                    if ch == 'n' || ch == 'N' {
                        let confirm = prompt_key("This will overwrite the existing capture. Are you sure? y/n: ");
                        if confirm != 'y' && confirm != 'Y' {
                            eprintln!("Aborted.");
                            return Ok(());
                        }
                        CaptureReport::default()
                    } else {
                        eprintln!("Resuming — already-captured steps will be skipped.\n");
                        r
                    }
                }
                Err(_) => {
                    eprintln!("Found {output_path} but couldn't parse it.");
                    let ch = prompt_key("Overwrite? y=start fresh, any other key=abort: ");
                    if ch != 'y' && ch != 'Y' {
                        eprintln!("Aborted.");
                        return Ok(());
                    }
                    CaptureReport::default()
                }
            }
        }
        Err(_) => CaptureReport::default(),
    };

    eprintln!("Output file: {output_path}\n");

    report.date = chrono::Local::now().to_rfc3339();
    report.tool_version = format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
    report.device_name = device_name;
    report.supported = supported;

    let all_steps = all_capture_steps();
    let is_filtered = step_filter.is_some();
    let include = |id: &str| -> bool {
        step_filter.as_ref().map_or(true, |f| f.contains(id))
    };

    let mut done = false;

    // --- Part 1: Modes ---
    let has_mode_steps = MODE_STEP_IDS.iter().any(|id| include(id));
    if has_mode_steps {
        eprintln!("{}", style("━━━ Part 1: Measurement Modes ━━━").bold());
        eprintln!("{}", style("any key=capture, s=skip one, q=skip to end and save").dim());

        for step in all_steps.iter().filter(|s| MODE_STEP_IDS.contains(&s.id)) {
            if done { break; }
            if !include(step.id) { continue; }
            done = run_capture_step(&mut dmm, step, &mut report, true);
            save_report(&report, &output_path)?;
        }
    }

    // --- Part 2: Flags ---
    let has_flag_steps = FLAG_STEP_IDS.iter().any(|id| include(id));
    if !done && has_flag_steps {
        eprintln!("\n{}", style("━━━ Part 2: Flags & Remote Commands ━━━").bold());
        eprintln!("Set meter to DC V mode for these tests.");
        let ch = prompt_key(&format!("\n{} ", style("Any key when ready on DC V, q=skip to end:").dim()));
        if ch == 'q' || ch == 'Q' { done = true; }
    }
    if !done && has_flag_steps {
        for step in all_steps.iter().filter(|s| FLAG_STEP_IDS.contains(&s.id)) {
            if done { break; }
            if !include(step.id) { continue; }
            done = run_capture_step(&mut dmm, step, &mut report, true);
            save_report(&report, &output_path)?;
        }
    }

    // --- Part 3: Range cycle ---
    if !done && include("range_cycle") {
        eprintln!("\n{}", style("━━━ Part 3: Range Values ━━━").bold());
        eprintln!("We'll cycle through manual ranges on DC V.");
        let ch = prompt_key(&format!("\n{} ", style("Any key to start, q=skip to end:").dim()));
        if ch != 'q' && ch != 'Q' {
            let _ = dmm.send_command(Command::Auto);
            std::thread::sleep(Duration::from_millis(200));

            let mut range_samples = Vec::new();
            for r in 0..6 {
                let _ = dmm.send_command(Command::Range);
                std::thread::sleep(Duration::from_millis(300));
                let raw = capture_samples(&mut dmm, 2);
                for m in &raw {
                    let s = SampleData::from_measurement(m);
                    eprintln!("  range_step_{r}: range={} label={}", s.range_byte, s.range_label);
                    range_samples.push(s);
                }
            }
            let _ = dmm.send_command(Command::Auto);
            std::thread::sleep(Duration::from_millis(200));

            upsert_step(&mut report, StepResult {
                id: "range_cycle".to_string(),
                instruction: "Cycle through manual ranges on DC V".to_string(),
                status: "captured".to_string(),
                samples: range_samples,
                screen: None,
                error: None,
            });
            save_report(&report, &output_path)?;
        } else {
            done = true;
        }
    }

    // --- Part 4: Freeform (skip if filtered) ---
    if !done && !is_filtered {
        eprintln!("\n{}", style("━━━ Part 4: Additional Captures (optional) ━━━").bold());
        eprintln!("Set the meter to any mode/state not covered above.\n");

        let mut extra = 0u32;
        loop {
            let desc = prompt(&format!(
                "[extra_{extra}] Describe what you set the meter to (or 'q' to finish): "
            ));
            if desc.is_empty() || desc.to_lowercase().starts_with('q') {
                break;
            }

            let raw = capture_samples(&mut dmm, 3);
            let sample_data: Vec<SampleData> = raw.iter().map(SampleData::from_measurement).collect();

            for (i, s) in sample_data.iter().enumerate() {
                eprintln!("    {} {}", style(format!("[{i}]")).dim(), s.summary());
            }

            let screen = if !sample_data.is_empty() {
                let summary = sample_data.last().unwrap().summary();
                let input = prompt(&format!(
                    "  We read: {summary}\n  Enter=correct, or type correction: "
                ));
                if input.is_empty() {
                    Some(format!("confirmed: {summary}"))
                } else {
                    Some(input)
                }
            } else {
                eprintln!("  No response from meter.");
                None
            };

            upsert_step(&mut report, StepResult {
                id: format!("extra_{extra}"),
                instruction: desc,
                status: if sample_data.is_empty() { "timeout".to_string() } else { "captured".to_string() },
                samples: sample_data,
                screen,
                error: None,
            });
            save_report(&report, &output_path)?;
            extra += 1;
        }
    }

    save_report(&report, &output_path)?;
    eprintln!();
    eprintln!("{}", style("=== Capture complete! ===").bold().green());
    eprintln!("Report saved to: {}", style(&output_path).bold());
    eprintln!("Please attach this file to your bug report or issue.");
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
        format_measurement(&mut buf, &m, &OutputFormat::Text).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("5.678"));
        assert!(output.contains("V"));
    }

    #[test]
    fn format_csv_output() {
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x00, 0x00, 0x00));
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
        // flag1=0x02 (HOLD), flag2=0x00 (AUTO on, inverted logic)
        let m = make_measurement(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
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
        let m = make_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mut buf = Vec::new();
        format_measurement(&mut buf, &m, &OutputFormat::Csv).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(",OL,"));
    }

    #[test]
    fn format_json_overload() {
        let m = make_measurement(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
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
