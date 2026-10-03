//! The command line `dmm-cli` takes: the clap types, the value parsers
//! behind them, and the help text built at run time.

use crate::capture::StepListFormat;
use crate::format::OutputFormat;
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use dmm_lib::protocol::Setting;
use dmm_lib::transform::{FactorError, Transform};
use std::path::PathBuf;

/// `--mock-clock-scale`: a multiple of real time, or `max`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClockScale {
    Factor(f64),
    /// Session time moves only as the run waits on it, so a replay plays
    /// every frame at its recorded time as fast as it decodes. A large
    /// factor cannot: the real time spent decoding is scaled into session
    /// time too, and polls then skip frames.
    Max,
}

fn parse_clock_scale(s: &str) -> Result<ClockScale, String> {
    if s == "max" {
        return Ok(ClockScale::Max);
    }
    s.parse()
        .map(ClockScale::Factor)
        .map_err(|_| format!("expected a number or 'max', got '{s}'"))
}

fn version_string() -> &'static str {
    dmm_shared::help::version_string(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

#[derive(Parser)]
#[command(
    name = "dmm-cli",
    version = version_string(),
    about = "CLI tool for USB and Bluetooth digital multimeters",
    after_help = "Run with --help for the full list of supported devices and the \
                  shared-settings file path.\n\n\
                  Set NO_COLOR=1 to disable colored output.\n\
                  Help / GitHub: https://github.com/antoinecellerier/dmm-tools",
    // after_long_help is set dynamically in main() so the actual per-platform
    // settings file path appears in the output.
    after_long_help = ""
)]
pub(crate) struct Cli {
    /// Device to connect to [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...].
    /// If omitted, falls back to `device_family` in ~/.config/dmm-tools/settings.json
    /// (written by dmm-gui), then to `auto`, which detects the meter over the
    /// USB cable.
    #[arg(long)]
    pub(crate) device: Option<String>,

    /// Select a specific adapter when more than one is reachable.
    /// Use the serial number or HID path of a USB cable, or the address of a
    /// Bluetooth adapter or meter, as 'dmm-cli list' prints them.
    #[arg(long, value_name = "SERIAL_PATH_OR_ADDRESS")]
    pub(crate) adapter: Option<String>,

    /// Turn off Bluetooth scanning for this run: nothing scans for adapters or
    /// meters and 'list' shows cables only. Overrides the saved 'bluetooth' setting.
    /// An address given to --adapter is still opened.
    #[arg(long)]
    pub(crate) no_bluetooth: bool,

    #[command(subcommand)]
    pub(crate) command: Cmd,
}

#[derive(Subcommand)]
pub(crate) enum Cmd {
    /// List connected USB cables, and Bluetooth adapters and meters in range
    List,
    /// Connect and print device info
    Info,
    /// Continuously read measurements
    Read {
        /// Keep at most one reading per interval, in milliseconds (0 = every reading the meter produces)
        #[arg(long, default_value = "0")]
        interval_ms: u64,
        /// Output format [default: text, or what -o's extension names]
        #[arg(long)]
        format: Option<OutputFormat>,
        /// Output file (stdout if not specified). Given without a name, the
        /// file is named after the meter, its mode and the run's start.
        #[arg(short, long, value_name = "FILE", num_args(0..=1))]
        output: Option<Option<String>>,
        /// Number of readings (0 = unlimited: Ctrl+C stops it, or the end of a --replay file)
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
        /// Play back a file written by --format replay instead of opening a meter; ends with the file
        #[arg(long, value_name = "FILE", conflicts_with = "mock_mode")]
        replay: Option<PathBuf>,
        /// Read an exported CSV, JSON or replay file instead of opening a
        /// meter, without waiting: to convert it or print its summary
        #[arg(
            long,
            value_name = "FILE",
            conflicts_with_all = ["mock_mode", "replay", "mock_clock_scale", "mock_clock_preseed"]
        )]
        import: Option<PathBuf>,
        /// Run session time at this multiple of real time (mock only), or
        /// `max`: with --replay, every frame at its recorded time, as fast as
        /// it decodes. Hidden: a contributor tool for fast runs, not a
        /// user-facing knob.
        #[arg(long, hide = true, value_parser = parse_clock_scale)]
        mock_clock_scale: Option<ClockScale>,
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
        /// Keep at most one reading per interval, in milliseconds (0 = every reading)
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
    Capture(CaptureArgs),
    /// Read a capture report with this build's parser, no meter needed.
    /// Hidden: a maintainer tool for the reports reporters attach.
    #[command(hide = true)]
    Triage(TriageArgs),
}

/// The `triage` flags, handed to [`crate::capture::cmd_triage`] whole. The
/// meter is the report's own `device_id`, or the top-level `--device` as
/// given, which a report from before `device_id` existed needs.
#[derive(clap::Args)]
pub(crate) struct TriageArgs {
    /// The report a `capture` run wrote
    pub(crate) report: PathBuf,
    /// The plan file a `capture --plan` run followed, for its steps'
    /// expectations
    #[arg(long, value_name = "FILE")]
    pub(crate) plan: Option<String>,
    /// Also print each step's frames in order as decoded: every step, or
    /// with `=` the comma-separated step ids given
    #[arg(
        long,
        value_name = "STEPS",
        num_args = 0..=1,
        require_equals = true,
        value_delimiter = ','
    )]
    pub(crate) timeline: Option<Vec<String>>,
    /// Also print each distinct sample as a golden fixture to pick from
    #[arg(long)]
    pub(crate) fixtures: bool,
}

/// The `capture` flags, handed to [`crate::capture::cmd_capture`] whole.
#[derive(clap::Args)]
pub(crate) struct CaptureArgs {
    /// Output file (default: capture-<device>.yaml). Overrides auto-naming.
    #[arg(short, long)]
    pub(crate) output: Option<String>,
    /// Only run specific steps (comma-separated IDs, e.g. "dcmv,temp,duty")
    #[arg(long, value_delimiter = ',')]
    pub(crate) steps: Option<Vec<String>>,
    /// Only run steps not yet confirmed on real hardware, plus the freeform pass
    #[arg(long)]
    pub(crate) unverified: bool,
    /// Run the steps in a maintainer's plan file instead of the device's own list
    #[arg(long, value_name = "FILE", conflicts_with_all = ["steps", "unverified", "list_steps"])]
    pub(crate) plan: Option<String>,
    /// Trust nothing the parser says: detect steps by raw byte changes and confirm each one
    #[arg(long)]
    pub(crate) sniff: bool,
    /// Don't let the tool set ranges and flags itself after each mode step
    #[arg(long)]
    pub(crate) no_drive: bool,
    /// Wait MS before sampling any step, for readings that settle slowly
    #[arg(long, value_name = "MS", default_value_t = 0)]
    pub(crate) settle: u64,
    /// List all available step IDs and exit
    #[arg(long)]
    pub(crate) list_steps: bool,
    /// How --list-steps prints the list (md is the issue checklist)
    #[arg(long, value_enum, default_value = "text", requires = "list_steps")]
    pub(crate) format: StepListFormat,
}

/// The `read` flags that build a software [`Transform`].
///
/// Flattened into `Cmd::Read` so the three flags stay one unit here and in
/// `--help`, and so `read` keeps its identity behaviour when none are given.
#[derive(clap::Args, Clone)]
pub(crate) struct TransformArgs {
    /// Multiply the reading, taken in base units (V, A, Ω, …), by FACTOR.
    /// A 10 mV/A clamp is --scale 100; a 100:1 probe is --scale 100.
    #[arg(long, value_name = "FACTOR", allow_negative_numbers = true, value_parser = parse_scale)]
    pub(crate) scale: Option<f64>,
    /// Add VALUE after scaling (32 with --scale 1.8 turns °C into °F)
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true, value_parser = parse_offset)]
    pub(crate) offset: Option<f64>,
    /// Label the scaled reading with this unit instead of the meter's base unit
    #[arg(long, value_name = "LABEL")]
    pub(crate) unit: Option<String>,
}

impl TransformArgs {
    /// The transform these flags describe. With none given the result is the
    /// identity, which `Transform::apply` skips entirely — so an unscaled
    /// `read` is byte-for-byte what it always was.
    pub(crate) fn to_transform(&self) -> Transform {
        Transform::linear(
            self.scale.unwrap_or(1.0),
            self.offset.unwrap_or(0.0),
            self.unit.clone(),
        )
    }

    /// The first of the three flags that was given, so a refusal names the one
    /// the user typed rather than listing all three.
    pub(crate) fn flag_given(&self) -> Option<&'static str> {
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

/// What `get` and `set` name on the command line, one word per
/// [`dmm_lib::protocol::Setting`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub(crate) enum SettingArg {
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
pub(crate) enum SettingsFormat {
    Text,
    Json,
}

/// Build long help text for --device from the registry.
pub(crate) fn build_device_help() -> String {
    dmm_shared::help::device_help("Device to connect to.")
}

/// The command `completions` writes a script for: the parsing one, with the
/// device ids and mock modes a shell offers.
pub(crate) fn completion_command() -> clap::Command {
    dmm_shared::help::with_completion_values(<Cli as clap::CommandFactory>::command())
}

/// Build long help text for --mock-mode from the mock's own mode table.
fn build_mock_mode_help() -> String {
    dmm_shared::help::mock_mode_help(
        "Pin the mock device to a specific measurement mode instead of \
         auto-cycling. Only effective with --device mock.",
        "--device mock read --mock-mode dcv",
    )
}

/// Resolve the shared settings file path for display in help text.
/// Returns the platform-specific location via `dmm-shared`, or a
/// sensible placeholder if the platform config dir is unavailable.
fn resolved_config_path_display() -> String {
    dmm_shared::config_path()
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
pub(crate) fn build_after_long_help() -> String {
    format!(
        "CONFIGURATION:\n\
         \x20 Settings file (shared with dmm-gui):\n\
         \x20   {path}\n\
         \n\
         \x20 --device precedence:\n\
         \x20   1. Command-line flag\n\
         \x20   2. device_family from the settings file above\n\
         \x20   3. auto \u{2014} detect the connected meter\n\
         \n\
         ENVIRONMENT:\n\
         \x20 RUST_LOG    Log filter. Unset, dmm_lib warnings and all errors are shown.\n\
         \x20             Use `dmm_lib=trace` for wire-level debugging.\n\
         \x20 NO_COLOR    Set to 1 to disable colored terminal output.\n\
         \n\
         Help / GitHub: https://github.com/antoinecellerier/dmm-tools",
        path = resolved_config_path_display(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
                import,
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
                assert!(import.is_none());
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
                assert_eq!(mock_clock_scale, Some(ClockScale::Factor(20.0)));
                assert_eq!(mock_clock_preseed, Some(90.0));
            }
            _ => panic!("expected Read"),
        }
    }

    /// `max` is a scale of its own, and anything else must be a number.
    #[test]
    fn clap_parse_the_max_clock_scale() {
        let scale = |value: &str| {
            Cli::try_parse_from(["dmm-cli", "read", "--mock-clock-scale", value])
                .ok()
                .and_then(|cli| match cli.command {
                    Cmd::Read {
                        mock_clock_scale, ..
                    } => mock_clock_scale,
                    _ => None,
                })
        };
        assert_eq!(scale("max"), Some(ClockScale::Max));
        assert_eq!(scale("0.5"), Some(ClockScale::Factor(0.5)));
        assert_eq!(scale("fast"), None);
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
                import: _,
            } => {
                assert_eq!(interval_ms, 100);
                assert_eq!(format, Some(OutputFormat::Csv));
                assert_eq!(output, Some(Some("test.csv".to_string())));
                assert_eq!(count, 10);
            }
            _ => panic!("expected Read"),
        }
    }

    /// `auto` is a value `--device` takes, and the only one the registry does
    /// not carry — so nothing else would put it in the help.
    #[test]
    fn the_device_help_offers_auto() {
        let help = build_device_help();
        assert!(
            help.contains("auto         Detect the connected meter (default)"),
            "{help}"
        );
        assert!(
            build_after_long_help().contains("3. auto \u{2014} detect the connected meter"),
            "the precedence list still names a model as the fallback"
        );
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
    fn clap_parse_completions() {
        let cli = Cli::try_parse_from(["dmm-cli", "completions", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::Completions {
                shell: Some(Shell::Bash)
            }
        ));
    }

    /// The values are keyed on the flags' ids; a renamed field would drop
    /// them from the script without an error.
    #[test]
    fn completions_offer_devices_and_mock_modes() {
        let mut script = Vec::new();
        clap_complete::generate(
            Shell::Bash,
            &mut completion_command(),
            "dmm-cli",
            &mut script,
        );
        let script = String::from_utf8(script).unwrap();
        // The top level's `--device`, and `read --mock-mode`.
        assert!(script.contains("auto ut61eplus"), "no device ids");
        assert!(script.contains("dcv acv"), "no mock modes");
    }
}
