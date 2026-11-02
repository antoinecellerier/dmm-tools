//! The command line `dmm-cli` takes: the clap types, the value parsers
//! behind them, and the help text built at run time.

use crate::capture::StepListFormat;
use crate::format::OutputFormat;
use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser, ValueParser};
use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use dmm_lib::alarm::{Hysteresis, Limits};
use dmm_lib::protocol::Setting;
use dmm_lib::transform::{FactorError, Transform};
use dmm_shared::input;
use dmm_shared::l10n::{self, Language, Localizer, clap_help};
use dmm_shared::tr;
use std::borrow::Cow;
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
        .map_err(|_| tr!(crate::l10n::get(), "read-clock-scale-invalid", value = s).into_owned())
}

fn version_string() -> &'static str {
    dmm_shared::help::version_string(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

/// `--lang`: a language the build has, or the system's whatever the
/// settings file says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Lang {
    System,
    Language(&'static Language),
}

/// What `--lang` names: `system`, or a tag [`l10n::find`] matches, so
/// `fr`, `fr_FR` and `pt-br` are taken as the settings file's `language`
/// and the system's locale are. `None` for anything else.
pub(crate) fn parse_lang(s: &str) -> Option<Lang> {
    if s.eq_ignore_ascii_case("system") {
        return Some(Lang::System);
    }
    l10n::find(s).map(Lang::Language)
}

/// The values `--lang` lists: each language's id, then `system`.
fn lang_values() -> impl Iterator<Item = &'static str> {
    l10n::languages().iter().map(|l| l.id).chain(["system"])
}

/// [`parse_lang`] as clap takes it: an unknown value gets clap's own
/// "invalid value" error, with the values it takes.
#[derive(Clone)]
struct LangParser;

impl TypedValueParser for LangParser {
    type Value = Lang;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Lang, clap::Error> {
        let given = value.to_string_lossy();
        parse_lang(&given).ok_or_else(|| {
            use clap::error::{ContextKind, ContextValue, ErrorKind};
            let mut e = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
            let arg = arg.map_or_else(|| "--lang".to_string(), ToString::to_string);
            e.insert(ContextKind::InvalidArg, ContextValue::String(arg));
            e.insert(
                ContextKind::InvalidValue,
                ContextValue::String(given.into_owned()),
            );
            e.insert(
                ContextKind::ValidValue,
                ContextValue::Strings(lang_values().map(String::from).collect()),
            );
            e
        })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(lang_values().map(PossibleValue::new)))
    }
}

/// The `--lang` of a command line, read before the command tree is built,
/// since its help is built in that language: `--lang X` or `--lang=X`, the
/// last before any `--`, as clap keeps a command's over the top level's. A
/// value [`parse_lang`] refuses is `None`, so the run goes on in the
/// language it would speak without, and clap reports the value.
pub(crate) fn lang_from_argv<I, S>(args: I) -> Option<Lang>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut args = args.into_iter();
    let mut found = None;
    while let Some(arg) = args.next() {
        let arg = arg.as_ref().to_string_lossy();
        if arg == "--" {
            break;
        }
        if arg == "--lang" {
            found = Some(
                args.next()
                    .and_then(|v| parse_lang(&v.as_ref().to_string_lossy())),
            );
        } else if let Some(value) = arg.strip_prefix("--lang=") {
            found = Some(parse_lang(value));
        }
    }
    found.flatten()
}

#[derive(Parser)]
#[command(
    name = "dmm-cli",
    version = version_string(),
    about = "CLI tool for USB and Bluetooth digital multimeters"
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

    /// Language of messages and help, or `system` for the system's. If
    /// omitted, falls back to `language` in the settings file (written by
    /// dmm-gui's Language row), then to the system's.
    // Global, as it is read wherever it stands; listed last.
    #[arg(
        long,
        global = true,
        display_order = 100,
        value_name = "LANG",
        value_parser = LangParser
    )]
    pub(crate) lang: Option<Lang>,

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
        #[command(flatten)]
        alarm: AlarmArgs,
        /// Pin mock device to a specific mode (only with --device mock).
        /// Without this, the settings file's mock mode applies, else the
        /// mock cycles through all modes.
        #[arg(long)]
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
        /// Without this, the settings file's mock mode applies, else the
        /// mock cycles through all modes.
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
        /// Without this, the settings file's mock mode applies, else the
        /// mock cycles through all modes.
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
    // The line above is the --help text, so `<device>` stays as users read it.
    #[allow(rustdoc::invalid_html_tags)]
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
    // Last of capture's own, so they sit beside the global --lang.
    /// Write the step instructions to FILE, to translate for --instructions,
    /// and exit
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with_all = ["instructions", "steps", "unverified", "list_steps"]
    )]
    pub(crate) export_instructions: Option<String>,
    /// Show the step instructions from FILE, translated from what
    /// --export-instructions wrote
    #[arg(long, value_name = "FILE")]
    pub(crate) instructions: Option<String>,
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
/// The rules and their wording are shared with the GUI's Scale row
/// ([`dmm_shared::input`]); clap's own message already names the flag and
/// the value.
fn parse_factor(s: &str, check: fn(f64) -> Result<f64, FactorError>) -> Result<f64, String> {
    let l = crate::l10n::get();
    let value: f64 = s
        .parse()
        .map_err(|_| input::not_a_number(l, s).into_owned())?;
    check(value).map_err(|e| input::factor_error(l, e).into_owned())
}

/// Reject the two scale factors that destroy the reading rather than
/// re-express it: zero collapses every sample onto the offset, and NaN/inf
/// poison the stats and the integral.
fn parse_scale(s: &str) -> Result<f64, String> {
    parse_factor(s, Transform::check_scale)
}

/// Any finite shift is a meaningful offset — zero and negatives included — so
/// only NaN and infinity are rejected, on the same grounds as `--scale`.
fn parse_offset(s: &str) -> Result<f64, String> {
    parse_factor(s, Transform::check_offset)
}

/// The `read` flags that set a threshold alarm.
#[derive(clap::Args, Clone)]
pub(crate) struct AlarmArgs {
    /// Warn when the reading rises above VALUE, in base units (V, A, Ω, …),
    /// or in the --unit of a scaled reading
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true, value_parser = parse_limit)]
    pub(crate) alarm_high: Option<f64>,
    /// Warn when the reading falls below VALUE, as --alarm-high
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true, value_parser = parse_limit)]
    pub(crate) alarm_low: Option<f64>,
    /// How far back inside a reading must come before the same limit alarms
    /// again: a value in the limits' unit, or a percentage of the limit
    /// (`1%`) [default: a few counts of the meter's last digit]
    #[arg(long, value_name = "BAND", allow_negative_numbers = true, value_parser = parse_hysteresis)]
    pub(crate) alarm_hysteresis: Option<Hysteresis>,
    /// Ring the terminal bell on each alarm
    #[arg(long)]
    pub(crate) alarm_bell: bool,
}

impl AlarmArgs {
    /// The limits these flags set and their band, `None` without any limit,
    /// or why the flags are unusable together.
    pub(crate) fn limits(&self, l: &Localizer) -> Result<Option<(Limits, Hysteresis)>, String> {
        let refused = |flags: &str, reason: Cow<'_, str>| {
            tr!(l, "read-flag-refused", flags = flags, reason = reason).into_owned()
        };
        match Limits::check(self.alarm_low, self.alarm_high) {
            Ok(limits) if limits.is_empty() && self.alarm_bell => {
                Err(tr!(l, "read-alarm-bell-needs-limit").into_owned())
            }
            Ok(limits) if limits.is_empty() && self.alarm_hysteresis.is_some() => {
                Err(tr!(l, "read-alarm-hysteresis-needs-limit").into_owned())
            }
            Ok(limits) if limits.is_empty() => Ok(None),
            Ok(limits) => self
                .alarm_hysteresis
                .unwrap_or_default()
                .check(&limits)
                .map(|band| Some((limits, band)))
                .map_err(|_| refused("--alarm-hysteresis", input::percent_of_zero(l))),
            Err(e) => Err(refused(
                "--alarm-low, --alarm-high",
                input::limit_error(l, e),
            )),
        }
    }
}

/// One alarm limit: a finite number.
fn parse_limit(s: &str) -> Result<f64, String> {
    let l = crate::l10n::get();
    let value: f64 = s
        .parse()
        .map_err(|_| input::not_a_number(l, s).into_owned())?;
    Limits::check(Some(value), None).map_err(|e| input::limit_error(l, e).into_owned())?;
    Ok(value)
}

/// `--alarm-hysteresis`: a value, or a percentage with `%`.
fn parse_hysteresis(s: &str) -> Result<Hysteresis, String> {
    Hysteresis::parse(s).map_err(|e| input::hysteresis_error(crate::l10n::get(), e, s).into_owned())
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

/// The command line as `l` words its help: the doc comments above through
/// `cli-help.ftl` ([`dmm_shared::l10n::clap_help`]), then the help built at
/// run time. What `main` parses with and prints `--help` from.
pub(crate) fn command(l: &Localizer) -> clap::Command {
    let cmd = clap_help::translate(<Cli as clap::CommandFactory>::command(), l, &values_parser);
    crate::help_layout::localize(with_run_time_help(cmd, l), l)
}

/// The parser of an argument whose possible values' help is translated: the
/// values as `PossibleValuesParser` takes them, mapped back to the
/// argument's type, which clap can't recover from the derive's parser. An
/// argument that gains described values needs a line here, or the
/// `every_described_value_has_a_parser` test fails.
fn values_parser(id: &str, values: Vec<PossibleValue>, ignore_case: bool) -> Option<ValueParser> {
    fn of<E: ValueEnum + Clone + Send + Sync + 'static>(
        values: Vec<PossibleValue>,
        ignore_case: bool,
    ) -> ValueParser {
        PossibleValuesParser::new(values)
            .try_map(move |s| E::from_str(&s, ignore_case))
            .into()
    }
    match id {
        "read--format" => Some(of::<OutputFormat>(values, ignore_case)),
        _ => None,
    }
}

/// The help made as the run starts, in `l`'s language: the device and mock
/// mode lists, which come from the registry and the mock, and the notes
/// after the options, one of which names this platform's settings path.
fn with_run_time_help(cmd: clap::Command, l: &Localizer) -> clap::Command {
    cmd.after_help(build_after_help(l))
        .after_long_help(build_after_long_help(l))
        .mut_arg("device", |a| a.long_help(build_device_help(l)))
        .mut_subcommand("read", |c| {
            c.mut_arg("mock_mode", |a| a.long_help(build_mock_mode_help(l)))
        })
        .mut_subcommand("completions", |c| {
            c.after_help(build_completions_after_help(l))
        })
}

/// Build long help text for --device from the registry.
fn build_device_help(l: &Localizer) -> String {
    l.help().device_help(&tr!(l, "help-device-intro"))
}

/// The command `completions` writes a script for: the parsing one, with the
/// device ids and mock modes a shell offers. English whatever the run
/// speaks, as a script installed once serves every later run.
pub(crate) fn completion_command() -> clap::Command {
    dmm_shared::help::with_completion_values(<Cli as clap::CommandFactory>::command())
}

/// Build long help text for --mock-mode from the mock's own mode table.
fn build_mock_mode_help(l: &Localizer) -> String {
    l.help().mock_mode_help(
        &tr!(l, "help-mock-mode-intro"),
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

/// The notes after `dmm-cli -h`'s options.
fn build_after_help(l: &Localizer) -> String {
    format!(
        "{}\n\n{}\n{}",
        tr!(l, "help-see-long-help"),
        tr!(l, "help-no-color", setting = "NO_COLOR=1"),
        l.help().project_link_line()
    )
}

/// Build the `after_long_help` text shown by `dmm-cli --help` (long form).
/// Dynamic so the actual resolved settings path appears per-platform.
///
/// Each line is a standalone item rather than wrapped prose — clap doesn't
/// re-wrap `after_long_help` text, so hardcoded mid-sentence line breaks
/// read poorly. A table-style layout with short, self-contained lines
/// avoids the issue on any terminal width.
fn build_after_long_help(l: &Localizer) -> String {
    let mut out = format!(
        "{}\n  {}\n    {}\n\n  {}\n",
        tr!(l, "help-configuration"),
        tr!(l, "help-settings-file"),
        resolved_config_path_display(),
        tr!(l, "help-device-precedence"),
    );
    let precedence = [
        tr!(l, "help-precedence-flag"),
        tr!(l, "help-precedence-settings"),
        tr!(l, "help-precedence-auto"),
    ];
    for (n, line) in precedence.iter().enumerate() {
        out.push_str(&format!("    {}. {line}\n", n + 1));
    }
    out.push_str(&format!("\n{}\n", tr!(l, "help-environment")));
    let variables = [
        (
            "RUST_LOG",
            tr!(l, "help-env-rust-log", filter = "dmm_lib=trace"),
        ),
        ("NO_COLOR", tr!(l, "help-env-no-color")),
        ("LANGUAGE", tr!(l, "help-env-language")),
        ("LC_ALL", tr!(l, "help-env-lc-all")),
    ];
    for (name, text) in &variables {
        // The text's lines under each other, past the longest name.
        for (i, line) in text.lines().enumerate() {
            let name = if i == 0 { *name } else { "" };
            out.push_str(&format!("  {name:<10}  {line}\n"));
        }
    }
    out.push_str(&format!("\n{}", l.help().project_link_line()));
    out
}

/// The notes after `dmm-cli completions --help`'s options: how to install
/// each shell's script.
fn build_completions_after_help(l: &Localizer) -> String {
    let mut out = tr!(l, "help-completions-install").into_owned();
    for (shell, line) in [
        (
            "bash",
            "dmm-cli completions bash > ~/.local/share/bash-completion/completions/dmm-cli",
        ),
        ("zsh", "dmm-cli completions zsh > ~/.zfunc/_dmm-cli"),
        (
            "fish",
            "dmm-cli completions fish > ~/.config/fish/completions/dmm-cli.fish",
        ),
        ("pwsh", "dmm-cli completions powershell >> $PROFILE"),
    ] {
        out.push_str(&format!("\n  {:<6} {line}", format!("{shell}:")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::FromArgMatches;
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
                alarm: _,
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
                alarm: _,
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
        let help = build_device_help(crate::l10n::english());
        assert!(
            help.contains("auto          Detect the connected meter (default)"),
            "{help}"
        );
        assert!(
            build_after_long_help(crate::l10n::english())
                .contains("3. auto \u{2014} detect the connected meter"),
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

    fn read_alarm(args: &[&str]) -> Result<Option<(Limits, Hysteresis)>, String> {
        let mut argv = vec!["dmm-cli", "read"];
        argv.extend_from_slice(args);
        match Cli::try_parse_from(argv).unwrap().command {
            Cmd::Read { alarm, .. } => alarm.limits(crate::l10n::english()),
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn clap_parse_read_alarm_limits() {
        assert_eq!(read_alarm(&[]), Ok(None));
        let (limits, band) = read_alarm(&["--alarm-low", "-1.5", "--alarm-high", "5"])
            .unwrap()
            .unwrap();
        assert_eq!((limits.low, limits.high), (Some(-1.5), Some(5.0)));
        assert_eq!(band, Hysteresis::Auto);
        let message = read_alarm(&["--alarm-low", "0", "--alarm-hysteresis", "1%"]).unwrap_err();
        assert!(message.contains("give a value"), "got {message}");
        assert_eq!(
            read_alarm(&["--alarm-low", "5", "--alarm-high", "3"]),
            Err("--alarm-low, --alarm-high: the low limit must be below the high one".to_string()),
        );
        assert!(read_alarm(&["--alarm-bell"]).is_err());
        assert!(read_alarm(&["--alarm-hysteresis", "1%"]).is_err());
        let msg = flag_error("--alarm-hysteresis", "-1");
        assert!(msg.contains("zero or more"), "got {msg}");
        let msg = flag_error("--alarm-high", "nan");
        assert!(msg.contains("'nan' for '--alarm-high"), "got {msg}");
        assert!(msg.contains("must be a finite number"), "got {msg}");
    }

    /// A zero or non-finite factor would destroy the reading rather than
    /// re-express it, and would poison the stats and the integral with NaN.
    #[test]
    fn clap_rejects_a_zero_or_non_finite_scale() {
        for bad in ["0", "-0", "nan", "inf"] {
            let msg = flag_error("--scale", bad);
            assert!(
                msg.contains("scale of zero") || msg.contains("finite number"),
                "--scale {bad} said: {msg}"
            );
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
            assert!(msg.contains("finite number"), "--offset {bad} said: {msg}");
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

    fn english() -> Lang {
        Lang::Language(l10n::find("en-US").unwrap())
    }

    #[test]
    fn lang_takes_a_language_or_system() {
        for tag in ["en-US", "en", "EN_gb", "en_US.UTF-8"] {
            assert_eq!(parse_lang(tag), Some(english()), "{tag}");
        }
        assert_eq!(parse_lang("system"), Some(Lang::System));
        assert_eq!(parse_lang("System"), Some(Lang::System));
        for tag in ["", "xx", "C", "POSIX"] {
            assert_eq!(parse_lang(tag), None, "{tag}");
        }
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        command(crate::l10n::english())
            .try_get_matches_from(std::iter::once("dmm-cli").chain(args.iter().copied()))
            .and_then(|mut m| Cli::from_arg_matches_mut(&mut m))
    }

    /// The pre-scan finds `--lang` where clap does: before or after the
    /// command, with or without `=`, never past `--`.
    #[test]
    fn the_lang_pre_scan_reads_what_clap_reads() {
        for (args, wanted) in [
            (&["--lang", "en", "list"][..], Some(english())),
            (&["--lang=en-us", "list"], Some(english())),
            (
                &["read", "--lang", "system", "--count", "1"],
                Some(Lang::System),
            ),
            (
                &["--device", "mock", "read", "--lang=system"],
                Some(Lang::System),
            ),
            (
                &["--lang", "system", "read", "--lang", "en"],
                Some(english()),
            ),
            (&["list"], None),
        ] {
            assert_eq!(lang_from_argv(args), wanted, "{args:?}");
            assert_eq!(parse(args).unwrap().lang, wanted, "{args:?}");
        }
        assert_eq!(lang_from_argv(["command", "--", "--lang", "en"]), None);
        assert_eq!(lang_from_argv(["--lang"]), None);
    }

    /// A value the pre-scan passes over is clap's to refuse, with the
    /// values `--lang` takes.
    #[test]
    fn an_unknown_lang_gets_claps_error() {
        assert_eq!(lang_from_argv(["--lang", "xx", "list"]), None);
        let e = parse(&["--lang", "xx", "list"]).err().unwrap();
        assert_eq!(e.kind(), clap::error::ErrorKind::InvalidValue);
        let text = e.to_string();
        assert!(
            text.contains("invalid value 'xx' for '--lang <LANG>'"),
            "{text}"
        );
        assert!(
            text.contains("[possible values: en-US, es, fr, pt-BR, ja, zh-CN, system]"),
            "{text}"
        );
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

    const CLI_HELP_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../i18n/en-US/cli-help.ftl");

    const CLI_HELP_HEADER: &str = "\
dmm-cli's --help: what each command and option does.

Generated from the doc comments in crates/dmm-cli/src/cli.rs by
`UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl`: edit those, not this
file. The run names each ID whose English changed. When the meaning changed
too, delete that message from every language's cli-help.ftl, so it shows in
English until it is translated again.

IDs follow the command tree: read.about is what the read command does,
read--interval-ms.help what its --interval-ms option does. clap wraps each
paragraph to the terminal's width: keep a paragraph on one line. Option
names (--device), values (auto, mock) and command lines ('dmm-cli list')
stay as they are. clap's own words around these texts (Usage:, Options:,
[default: …], [possible values: …]) are in cli/help.ftl.";

    /// `cli-help.ftl` holds the doc comments as they are now.
    #[test]
    fn cli_help_ftl_follows_the_doc_comments() {
        use std::io::Write;
        let path = Path::new(CLI_HELP_PATH);
        let current = std::fs::read_to_string(path).unwrap_or_default();
        let rendered =
            clap_help::render(&<Cli as clap::CommandFactory>::command(), CLI_HELP_HEADER);
        if rendered == current {
            return;
        }
        let changes = clap_help::changes(&current, &rendered);
        let summary = format!(
            "English changed: {:?}\nadded: {:?}\nremoved: {:?}",
            changes.changed, changes.added, changes.removed
        );
        if std::env::var_os("UPDATE_L10N").is_some() {
            dmm_shared::write_atomic(path, rendered.as_bytes()).expect("write cli-help.ftl");
            // Past the test harness's capture, so a passing run shows it.
            let _ = writeln!(
                std::io::stderr(),
                "i18n/en-US/cli-help.ftl regenerated.\n{summary}"
            );
        } else {
            panic!(
                "i18n/en-US/cli-help.ftl is out of date; run \
                 `UPDATE_L10N=1 cargo test -p dmm-cli cli_help_ftl` and commit it.\n{summary}"
            );
        }
    }

    /// Every option whose values say what they do has a parser the walk
    /// can rebuild with their translated help.
    #[test]
    fn every_described_value_has_a_parser() {
        let cmd = <Cli as clap::CommandFactory>::command();
        let unmapped = clap_help::unmapped_values(cmd, &values_parser);
        assert!(
            unmapped.is_empty(),
            "add {unmapped:?} to values_parser in cli.rs"
        );
    }

    /// Every help page `cmd` prints: `-h` and `--help` at the top and for
    /// each subcommand, hidden ones included, and `help <subcommand>`.
    fn help_pages(cmd: &clap::Command) -> Vec<(String, String)> {
        let names: Vec<String> = cmd
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        let mut argvs: Vec<Vec<&str>> = vec![vec!["-h"], vec!["--help"], vec!["help"]];
        for name in &names {
            argvs.push(vec![name, "-h"]);
            argvs.push(vec![name, "--help"]);
            argvs.push(vec!["help", name]);
        }
        argvs
            .into_iter()
            .map(|argv| {
                let line = format!("dmm-cli {}", argv.join(" "));
                let full = std::iter::once("dmm-cli").chain(argv);
                match cmd.clone().try_get_matches_from(full) {
                    Ok(_) => panic!("{line} printed no help"),
                    Err(e) => (line, e.render().to_string()),
                }
            })
            .collect()
    }

    /// Walked in English, the tree prints every help page as the doc
    /// comments alone do, byte for byte.
    #[test]
    fn the_english_walk_leaves_every_help_page_as_it_was() {
        let l = crate::l10n::english();
        let unwalked = help_pages(&with_run_time_help(
            <Cli as clap::CommandFactory>::command(),
            l,
        ));
        let walked = help_pages(&command(l));
        assert_eq!(unwalked.len(), walked.len());
        for (before, after) in unwalked.iter().zip(&walked) {
            assert_eq!(before, after);
        }
    }

    /// The values whose help translates still parse to their type, and
    /// refuse what they always refused.
    #[test]
    fn a_translated_value_list_still_parses() {
        let parse = |format: &str| {
            command(crate::l10n::english())
                .try_get_matches_from(["dmm-cli", "read", "--format", format])
                .and_then(|mut m| Cli::from_arg_matches_mut(&mut m))
        };
        match parse("replay").map(|cli| cli.command) {
            Ok(Cmd::Read { format, .. }) => assert_eq!(format, Some(OutputFormat::Replay)),
            _ => panic!("expected Read"),
        }
        let refused = parse("yaml").err().unwrap().to_string();
        assert!(
            refused.contains("[possible values: text, csv, json, replay]"),
            "{refused}"
        );
    }

    /// Under the pseudo-locale every text the help shows went through the
    /// translations: each walked text is one bracketed message, each text
    /// built at run time starts with one.
    #[test]
    fn the_pseudo_locale_brackets_every_help_text() {
        use dmm_shared::l10n::{EN_US, Options};
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let pseudo = Localizer::new(&EN_US, &crate::l10n::resources(), &options);
        let mut problems = Vec::new();
        let mut check = |at: String, text: Option<&clap::builder::StyledStr>, whole: bool| {
            let Some(text) = text.map(|t| t.to_string()) else {
                return;
            };
            if !text.starts_with('[') || (whole && !text.ends_with(']')) {
                problems.push(format!("{at}: {text}"));
            }
        };
        let cmd = command(&pseudo);
        check("after_help".into(), cmd.get_after_help(), false);
        check("after_long_help".into(), cmd.get_after_long_help(), false);
        let mut stack = vec![(String::from("dmm-cli"), &cmd)];
        while let Some((line, c)) = stack.pop() {
            check(format!("{line} about"), c.get_about(), true);
            check(format!("{line} long_about"), c.get_long_about(), true);
            for a in c.get_arguments().filter(|a| !a.is_hide_set()) {
                let at = format!("{line} {}", a.get_id());
                check(format!("{at} help"), a.get_help(), true);
                let composed = ["device", "mock_mode"].contains(&a.get_id().as_str());
                check(format!("{at} long_help"), a.get_long_help(), !composed);
                for v in a.get_possible_values() {
                    check(format!("{at} {}", v.get_name()), v.get_help(), !composed);
                }
            }
            for sub in c.get_subcommands().filter(|s| !s.is_hide_set()) {
                check(
                    format!("{line} {} after_help", sub.get_name()),
                    sub.get_after_help(),
                    false,
                );
                stack.push((format!("{line} {}", sub.get_name()), sub));
            }
        }
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
