mod a11y;
mod app;
mod changelog;
#[cfg(target_os = "linux")]
mod desktop_entry;
mod display;
mod graph;
mod l10n;
mod listbox;
mod markers;
mod recording;
mod settings;
mod specs;
mod theme;

use clap::builder::{PossibleValue, TypedValueParser, ValueParser};
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use dmm_lib::protocol::registry;
use dmm_lib::replay::Replay;
use dmm_shared::l10n::{Localizer, arg_errors, clap_help, help_layout};
use dmm_shared::tr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// Placeholder string for missing/unavailable data values in the UI.
pub(crate) const NO_DATA: &str = "---";

fn version_string() -> &'static str {
    dmm_shared::help::version_string(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

/// Version string for the app (shown in top bar, right side).
pub fn version_label() -> String {
    dmm_shared::help::version_label(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

/// The command line. Its doc comments are `--help`'s English, which
/// `gui-help.ftl` carries for translation; the help built as the run starts
/// is added by [`with_run_time_help`], never here, or it would land in that
/// file.
#[derive(Parser)]
#[command(
    name = "dmm-gui",
    version = version_string(),
    about = "GUI application for USB and Bluetooth digital multimeters"
)]
struct Args {
    /// Device to connect to [auto, ut61eplus, ut8803, ut171, ut181a, mock, ...]
    #[arg(long)]
    device: Option<String>,

    /// Pin mock device to a specific mode (implies --device mock)
    #[arg(long, value_parser = dmm_shared::help::MockModeParser)]
    mock_mode: Option<dmm_lib::mock::MockMode>,

    /// Theme override
    #[arg(long)]
    theme: Option<String>,

    /// Graphics renderer
    #[arg(long, value_enum)]
    renderer: Option<RendererFlag>,

    /// Select a specific adapter when more than one is reachable.
    /// Use the serial number or HID path of a USB cable, or the address of a
    /// Bluetooth adapter or meter, as 'dmm-cli list' prints them.
    #[arg(long, value_name = "SERIAL_PATH_OR_ADDRESS")]
    adapter: Option<String>,

    /// Turn off Bluetooth scanning for this session.
    /// Overrides the "Look for Bluetooth devices" setting. An address given
    /// to --adapter is still opened.
    #[arg(long)]
    no_bluetooth: bool,

    /// Play back a file written by 'dmm-cli read --format replay' or by
    /// Export… → Replay…, instead of connecting to a meter
    #[arg(long, value_name = "FILE", conflicts_with_all = ["device", "mock_mode"])]
    replay: Option<PathBuf>,

    /// Open an exported CSV, JSON or replay file, with its markers, instead
    /// of connecting to a meter
    #[arg(long, value_name = "FILE", conflicts_with_all = ["device", "mock_mode", "replay"])]
    import: Option<PathBuf>,

    /// Print a shell completion script and exit
    #[arg(long, value_enum, value_name = "SHELL", exclusive = true)]
    completions: Option<clap_complete::Shell>,

    /// Run session time at FACTOR times real time (mock only, implies
    /// --device mock). Hidden: a contributor tool for screenshots and
    /// performance runs, documented in docs/development.md.
    #[arg(long, hide = true, value_name = "FACTOR", value_parser = parse_clock_scale)]
    mock_clock_scale: Option<f64>,

    /// Start the session with SECS of history, produced instantly (mock
    /// only, implies --device mock). Hidden for the same reason as
    /// --mock-clock-scale.
    #[arg(long, hide = true, value_name = "SECS")]
    mock_clock_preseed: Option<f64>,

    /// Show the update notice for TAG (`v0.8.0` or `dev-<hash>`) without
    /// asking GitHub or saving anything. Hidden: a contributor tool for
    /// screenshots, documented in docs/development.md.
    #[arg(long, hide = true, value_name = "TAG", value_parser = parse_update_notice)]
    update_notice: Option<app::update_check::Tag>,
}

/// What `--renderer` takes.
#[derive(Clone, Copy, ValueEnum)]
enum RendererFlag {
    Wgpu,
    Glow,
}

fn parse_update_notice(s: &str) -> Result<app::update_check::Tag, String> {
    app::update_check::Tag::parse(s).ok_or_else(|| "expected vX.Y.Z or dev-<hash>".to_string())
}

/// `--mock-clock-scale`'s factor. `max`, which `dmm-cli read` takes, is
/// refused by name rather than as a bad number: it would race a replay
/// through to its end before the window drew.
fn parse_clock_scale(s: &str) -> Result<f64, String> {
    if s == "max" {
        return Err("'max' is for converting a replay with dmm-cli read; \
                    the GUI would play the whole file at once, so give a factor"
            .to_string());
    }
    s.parse()
        .map_err(|_| format!("expected a number, got '{s}'"))
}

/// The command line as `l` words its help: the doc comments through
/// `gui-help.ftl` ([`clap_help`]), then the help built at run time, and
/// clap's own words ([`help_layout`]). What [`parse_args`] parses with and
/// prints `--help` from.
fn command(l: &Localizer) -> clap::Command {
    let cmd = clap_help::translate(Args::command(), l, &no_described_values);
    help_layout::localize(with_run_time_help(cmd, l), l)
}

/// The values parser [`clap_help::translate`] asks for: none, as the only
/// values dmm-gui lists, the shells, say nothing about themselves.
fn no_described_values(_: &str, _: Vec<PossibleValue>, _: bool) -> Option<ValueParser> {
    None
}

/// The help made as the run starts, in `l`'s language: the device and mock
/// mode lists, which come from the registry and the mock, the themes, and
/// the project link after the options.
fn with_run_time_help(cmd: clap::Command, l: &Localizer) -> clap::Command {
    cmd.after_long_help(l.help().project_link_line())
        .mut_arg("device", |a| a.long_help(build_device_help(l)))
        .mut_arg("mock_mode", |a| a.long_help(build_mock_mode_help(l)))
        .mut_arg("theme", with_theme_list)
}

/// `--theme`'s help followed by the themes, which only exist at run time:
/// `--help` shows them, and the completion scripts offer them.
fn with_theme_list(arg: clap::Arg) -> clap::Arg {
    let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
    arg.help(format!("{help} [{}]", theme_values(&[]).join(", ")))
}

/// Build long help text for --device from the registry.
fn build_device_help(l: &Localizer) -> String {
    l.help().device_help(&tr!(l, "command-line-device-intro"))
}

/// What `--theme` accepts, as `--help` and the error for a bad value list it:
/// the modes, then the themes as the Theme row lists them, `user` files
/// included and each name once.
fn theme_values(user: &[theme::named::NamedTheme]) -> Vec<String> {
    let named = theme::named::listed(user)
        .into_iter()
        .map(|t| theme::named::flag_name(&t.name));
    ["dark", "light", "system"]
        .map(String::from)
        .into_iter()
        .chain(named)
        .collect()
}

/// Build long help text for --mock-mode from the mock's own mode table.
fn build_mock_mode_help(l: &Localizer) -> String {
    l.help().mock_mode_help(
        &tr!(l, "command-line-mock-mode-intro"),
        "dmm-gui --mock-mode dcv",
    )
}

/// `value` of the option `id`, refused after parsing for `reason`: worded
/// as clap words a value its own parser refuses, so the shared renderer
/// puts it in the user's language.
fn refused(cmd: &clap::Command, id: &str, value: &str, reason: String) -> clap::Error {
    let parser = move |_: &str| Err::<(), String>(reason.clone());
    let arg = cmd.get_arguments().find(|a| a.get_id() == id);
    parser
        .parse_ref(cmd, arg, std::ffi::OsStr::new(value))
        .err()
        .unwrap_or_else(|| clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd))
}

/// `value` of the option `id`, refused after parsing as none of `values`:
/// worded as clap words a value outside the ones it lists.
fn not_one_of(cmd: &clap::Command, id: &str, value: &str, values: Vec<String>) -> clap::Error {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    let arg = cmd
        .get_arguments()
        .find(|a| a.get_id() == id)
        .map_or_else(|| format!("--{id}"), ToString::to_string);
    let mut e = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
    e.insert(ContextKind::InvalidArg, ContextValue::String(arg));
    e.insert(
        ContextKind::InvalidValue,
        ContextValue::String(value.to_string()),
    );
    e.insert(ContextKind::ValidValue, ContextValue::Strings(values));
    e
}

/// An option's value refused after parsing: the option's id, and why in
/// the user's language, but for the hidden clock flags' reasons.
#[derive(Debug, PartialEq)]
struct Refusal {
    id: &'static str,
    reason: String,
}

/// CLI overrides to apply on top of persisted settings for this session.
pub struct CliOverrides {
    pub device: Option<String>,
    pub mock_mode: Option<dmm_lib::mock::MockMode>,
    pub theme: Option<settings::ThemeChoice>,
    /// The user's themes folder, when `--theme` named a theme and so read it.
    pub(crate) user_themes: Option<theme::named::ThemeCatalog>,
    pub renderer: Option<eframe::Renderer>,
    pub adapter: Option<String>,
    /// `--no-bluetooth`: no Bluetooth scanning for this session, whatever the
    /// saved setting says.
    pub no_bluetooth: bool,
    /// Time base for this session's readings: real unless a `--mock-clock-*`
    /// flag was given. Under `--replay` the first Connect pins its origin to
    /// the recording's own start.
    pub clock: dmm_lib::Clock,
    /// The recording this session plays instead of opening a meter.
    pub replay: Option<ReplaySource>,
    /// `--import`: the file this session opens instead of a meter.
    pub import: Option<PathBuf>,
    /// `--update-notice`: the release the top bar links to, in place of
    /// asking GitHub.
    pub(crate) update_notice: Option<app::update_check::Tag>,
}

/// A recording to play back, and the file it came from.
///
/// The parsed recording is shared rather than re-read: a Disconnect/Connect
/// re-opens it. The path is carried alongside because only the file name says
/// *which* session is on screen, and the recording itself does not know it.
pub struct ReplaySource {
    pub replay: Arc<Replay>,
    pub path: PathBuf,
    /// Wall time the recording started at, which the first Connect makes this
    /// session's zero.
    pub recorded: SystemTime,
}

#[cfg(test)]
impl ReplaySource {
    /// A one-frame UT61E+ recording, for tests that only need the session to
    /// be a playback.
    pub(crate) fn fixture() -> Self {
        Self {
            replay: Arc::new(
                Replay::parse(
                    "# dmm-replay 1\n\
                     # device: ut61eplus\n\
                     # recorded: 2026-09-02T10:00:00Z\n\
                     0 02 30 20 31 2E 36 31 30 39 03 02 30 30 30\n",
                )
                .expect("a well-formed recording"),
            ),
            path: PathBuf::from("bench.replay"),
            // The `# recorded:` line above, as the loader would parse it.
            recorded: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_788_343_200),
        }
    }
}

/// Load `--replay`'s file, with the wall time its header says the recording
/// started at; why not, in `l`'s language, if it won't load.
///
/// A file that will not load is a bad flag value, reported the way a bad
/// `--device` is: at the command line, before a window exists to show it in.
fn load_replay(path: PathBuf, l: &Localizer) -> Result<ReplaySource, String> {
    let replay = dmm_shared::replay::load(&path).map_err(|e| l.error(&e))?;
    let recorded = dmm_shared::replay::recorded_at(&replay).map_err(|e| l.read_error(&e))?;
    Ok(ReplaySource {
        replay: Arc::new(replay),
        path,
        recorded: recorded.into(),
    })
}

/// Resolve the device and the session clock from the flags that decide them.
///
/// Split out of [`parse_args`] because it holds three rules worth testing on
/// their own: the clock flags are validated like `--theme`, they imply
/// `--device mock` the way `--mock-mode` does, and they are refused outright
/// on a hardware device, where bent session time would stamp readings with
/// instants a USB-paced meter never produced.
///
/// `device` is the canonical id `--device` resolved to, `None` when the flag
/// was not given. `replay_device` is the meter a `--replay` file names, which
/// clap has already refused `--device` alongside. A refusal is worded in
/// `l`'s language, but for the clock flags' own reasons.
fn resolve_device_and_clock(
    l: &Localizer,
    device: Option<String>,
    replay_device: Option<&'static str>,
    mock_mode_given: bool,
    scale: Option<f64>,
    preseed: Option<f64>,
) -> Result<(Option<String>, dmm_lib::Clock), Refusal> {
    // The scale alone first, so a refusal names the flag it is about.
    dmm_lib::Clock::from_flags(scale, None).map_err(|reason| Refusal {
        id: "mock_clock_scale",
        reason,
    })?;
    let clock = dmm_lib::Clock::from_flags(scale, preseed).map_err(|reason| Refusal {
        id: "mock_clock_preseed",
        reason,
    })?;

    // Auto counts as hardware: it names no meter, but the meter it finds is a
    // real one, and bending session time under it would stamp readings with
    // instants no USB-paced meter produced. A replay names a hardware meter
    // too, but nothing is on the cable and the file paces itself, so the
    // clock flags apply to it as they do to the mock.
    let hardware = replay_device.is_none()
        && match device.as_deref().and_then(registry::resolve_selection) {
            Some(registry::Selection::Auto) => true,
            Some(registry::Selection::Device(d)) => d.requires_hardware,
            None => false,
        };
    if !clock.is_real() && hardware {
        return Err(Refusal {
            id: "device",
            reason: l.help().mock_clock_mock_only().into_owned(),
        });
    }

    // A replay is the session's device; failing that, --mock-mode and either
    // clock flag each imply --device mock, so `dmm-gui --mock-clock-preseed
    // 90` is a complete invocation.
    let device = match (replay_device, device) {
        (Some(id), _) => Some(id.to_string()),
        (None, d @ Some(_)) => d,
        (None, None) if mock_mode_given || !clock.is_real() => {
            Some(dmm_lib::mock::MOCK.id.to_string())
        }
        (None, None) => None,
    };
    Ok((device, clock))
}

/// The command `--completions` writes a script for: the parsing one, with
/// the device ids and mock modes a shell offers.
fn completion_command() -> clap::Command {
    dmm_shared::help::with_completion_values(Args::command()).mut_arg("theme", with_theme_list)
}

/// The id `--device <raw>` is stored under, or `None` for an unknown name:
/// the listing row's id, so a sold-as name (`ut217a`) keeps its own row
/// rather than the maker's, as the Settings row and `dmm-cli` do; and
/// `auto` as a choice rather than an unknown device.
fn canonical_device(raw: &str) -> Option<String> {
    match registry::resolve_selection(raw)? {
        registry::Selection::Auto => Some(registry::AUTO_DEVICE_ID.to_string()),
        registry::Selection::Device(_) => {
            registry::resolve_listing(raw).map(|l| l.id().to_string())
        }
    }
}

/// The command line, worded in `l`'s language: its `--help`, and the
/// errors that end the run before a window opens.
fn parse_args(l: &Localizer) -> CliOverrides {
    let mut cmd = command(l);
    let matches = cmd
        .try_get_matches_from_mut(std::env::args_os())
        .unwrap_or_else(|e| arg_errors::exit(e, l, &[]));
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| arg_errors::exit(e, l, &[]));
    let refuse = |e: clap::Error| -> ! { arg_errors::exit(e, l, &[]) };

    if let Some(shell) = args.completions {
        clap_complete::generate(
            shell,
            &mut completion_command(),
            "dmm-gui",
            &mut std::io::stdout(),
        );
        std::process::exit(0);
    }

    // Validate and canonicalize --device if provided.
    let device = args.device.map(|raw| {
        canonical_device(&raw).unwrap_or_else(|| {
            let reason = tr!(l, "command-line-unknown-device").into_owned();
            refuse(refused(&cmd, "device", &raw, reason))
        })
    });

    // Parse --theme if provided. A named theme reads the user's themes
    // folder, whose files may replace a built-in; the modes never do.
    let mut user_themes = None;
    let theme = args.theme.as_deref().map(|t| {
        use settings::{ThemeChoice, ThemeMode};
        match t {
            "dark" => ThemeChoice::Mode(ThemeMode::Dark),
            "light" => ThemeChoice::Mode(ThemeMode::Light),
            "system" => ThemeChoice::Mode(ThemeMode::System),
            other => {
                let catalog = theme::named::discover_user();
                let found = theme::named::find(other, &catalog.themes).map(|t| t.name.clone());
                let Some(name) = found else {
                    refuse(not_one_of(
                        &cmd,
                        "theme",
                        other,
                        theme_values(&catalog.themes),
                    ))
                };
                user_themes = Some(catalog);
                ThemeChoice::Named(name)
            }
        }
    });

    let renderer = args.renderer.map(|r| match r {
        RendererFlag::Wgpu => eframe::Renderer::Wgpu,
        RendererFlag::Glow => eframe::Renderer::Glow,
    });

    let replay = args.replay.map(|path| {
        let typed = path.display().to_string();
        load_replay(path, l)
            .unwrap_or_else(|reason| refuse(refused(&cmd, "replay", &typed, reason)))
    });

    let (device, clock) = resolve_device_and_clock(
        l,
        device,
        replay.as_ref().map(|source| source.replay.listing.id()),
        args.mock_mode.is_some(),
        args.mock_clock_scale,
        args.mock_clock_preseed,
    )
    .unwrap_or_else(|r| {
        let typed = matches
            .get_raw(r.id)
            .into_iter()
            .flatten()
            .next()
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_default();
        refuse(refused(&cmd, r.id, &typed, r.reason))
    });

    CliOverrides {
        device,
        mock_mode: args.mock_mode,
        theme,
        user_themes,
        renderer,
        adapter: args.adapter,
        no_bluetooth: args.no_bluetooth,
        clock,
        replay,
        import: args.import,
        update_notice: args.update_notice,
    }
}

/// Install icon and .desktop file so Wayland compositors (GNOME, etc.) can show
/// the app icon in alt-tab and the task bar. Runs once per launch; overwrites
/// stale files from older versions.
///
/// The .desktop `Exec` line is set to the current binary path so the entry is
/// valid whether the app is run via `cargo run` or from an installed location.
/// GNOME's `g_app_info_get_all()` silently drops entries whose `Exec` binary
/// doesn't exist, which would make the icon invisible.
#[cfg(target_os = "linux")]
fn install_desktop_integration() {
    use directories::BaseDirs;
    use std::fs;

    let Some(base_dirs) = BaseDirs::new() else {
        log::debug!("could not determine XDG base directories, skipping desktop integration");
        return;
    };
    let data_dir = base_dirs.data_dir();

    // Icon files (embedded at compile time).
    let icons: &[(&str, &[u8])] = &[
        (
            "icons/hicolor/256x256/apps/dmm-tools.png",
            include_bytes!("../../../assets/icon-256.png"),
        ),
        (
            "icons/hicolor/scalable/apps/dmm-tools.svg",
            include_bytes!("../../../assets/icon.svg"),
        ),
    ];
    // Only rewrite what actually differs. This runs before the window is
    // created on every launch, and unconditionally writing a 256x256 PNG, an
    // SVG and the .desktop file — then spawning gtk-update-icon-cache and
    // waiting for it to exit — is a visible blank period at startup, paid
    // every time to reproduce bytes that are already on disk.
    let mut icons_changed = false;
    for (rel_path, content) in icons {
        icons_changed |= write_if_changed(&data_dir.join(rel_path), content);
    }

    // Desktop entry — Exec points to whatever binary is currently running.
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| fs::canonicalize(p).ok());
    if let Some(exe_path) = exe {
        let desktop = desktop_entry::contents(&exe_path.display().to_string());
        let desktop_path = data_dir.join("applications/dmm-tools.desktop");
        write_if_changed(&desktop_path, desktop.as_bytes());
    }

    // Update the GTK icon cache so GNOME can find the icon without a session
    // restart — but only when an icon actually changed. This spawns a process
    // and blocks until it exits, which is the bulk of the startup cost.
    if icons_changed {
        let icon_dir = data_dir.join("icons/hicolor");
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .args(["-f", "-t"])
            .arg(&icon_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        log::debug!("desktop integration files installed, icon cache refreshed");
    } else {
        log::debug!("desktop integration already up to date");
    }
}

/// Write `content` to `path` unless it is already exactly that.
///
/// Returns whether anything was written. Creates parent directories on
/// demand. Errors are ignored: desktop integration is best-effort, and a
/// read-only or absent data dir must not stop the GUI starting.
#[cfg(target_os = "linux")]
fn write_if_changed(path: &std::path::Path, content: &[u8]) -> bool {
    use std::fs;
    if fs::read(path).is_ok_and(|existing| existing == content) {
        return false;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, content).is_ok()
}

fn main() -> eframe::Result<()> {
    dmm_shared::logging::init();

    // Before the command line, so `--help` and its errors speak the language
    // the window will: the settings file's, else the system's. Loaded here,
    // not in `App::new`, as the window is built from them too. Loading
    // writes nothing: a file that won't load is reported, and kept as
    // `.bad`, once the command line has parsed.
    let (mut settings, unreadable) = settings::Settings::load();
    // The window keeps this one, so what loading it found is logged once.
    let l10n = Arc::new(l10n::startup(&settings.shared.language));
    l10n.log_load_report();
    let overrides = parse_args(&l10n);

    // After parsing: `--help`, `--version` and a bad flag exit there, so
    // running them leaves no `.bad` copy and, from a build tree, the desktop
    // entry alone.
    settings.load_problem = unreadable.map(settings::Unreadable::keep);
    #[cfg(target_os = "linux")]
    install_desktop_integration();

    // Embedded icon for the window titlebar / taskbar. On Windows and macOS this
    // is all that's needed. On Linux/Wayland the icon is looked up from the
    // .desktop file instead — see install_desktop_integration().
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon-256.png"))
        .expect("failed to load app icon");

    let explicit_renderer = overrides.renderer.is_some();
    let renderer = overrides.renderer.unwrap_or(eframe::Renderer::Wgpu);

    // The themes folder is read before the first frame only when this
    // session draws a named theme, which may be one of its files, so that
    // frame is already in the right colours. Otherwise opening Settings is
    // the first read.
    let named = match &overrides.theme {
        Some(choice) => matches!(choice, settings::ThemeChoice::Named(_)),
        None => settings.named_theme.is_some(),
    };
    if named {
        let catalog = overrides
            .user_themes
            .clone()
            .unwrap_or_else(theme::named::discover_user);
        theme::named::log_skipped(&catalog, None);
        settings.user_themes = std::sync::Arc::new(catalog);
    }
    // eframe clamps the size to the largest monitor. The title bar is
    // hidden here rather than by the first frame's command, which on
    // Windows keeps the outer size and so grows the kept inner size on every
    // launch.
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_app_id("dmm-tools")
        .with_icon(std::sync::Arc::new(icon))
        .with_inner_size(
            settings
                .window_size
                .unwrap_or(settings::DEFAULT_WINDOW_SIZE),
        )
        .with_min_inner_size(settings::MIN_WINDOW_SIZE)
        .with_maximized(settings.window_maximized)
        .with_decorations(!settings.hide_decorations);
    let fallback_settings = settings.clone();
    let fallback_l10n = l10n.clone();

    let options = eframe::NativeOptions {
        viewport: viewport.clone(),
        renderer,
        // The app installs its own, picking shapes by its language rather
        // than the system's (`app/language.rs`).
        system_font_fallback: false,
        ..Default::default()
    };

    let result = eframe::run_native(
        "dmm-tools",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, settings, overrides, l10n)))),
    );

    // If wgpu failed and wasn't explicitly requested, retry with glow
    if result.is_err() && !explicit_renderer {
        log::warn!("wgpu renderer failed, falling back to glow");
        let fallback_options = eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            system_font_fallback: false,
            ..Default::default()
        };
        let fallback_overrides = parse_args(&fallback_l10n);
        return eframe::run_native(
            "dmm-tools",
            fallback_options,
            Box::new(move |cc| {
                Ok(Box::new(app::App::new(
                    cc,
                    fallback_settings,
                    fallback_overrides,
                    fallback_l10n,
                )))
            }),
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_names_keep_their_listing_row() {
        assert_eq!(canonical_device("ut217a").as_deref(), Some("ut217a"));
        assert_eq!(canonical_device("UT61E+").as_deref(), Some("ut61eplus"));
        assert_eq!(canonical_device("cms061").as_deref(), Some("cms061"));
        assert_eq!(canonical_device("auto").as_deref(), Some("auto"));
        assert_eq!(canonical_device("no-such-meter"), None);
    }

    fn resolve(
        device: Option<&str>,
        scale: Option<f64>,
        preseed: Option<f64>,
    ) -> Result<(Option<String>, dmm_lib::Clock), Refusal> {
        let l = crate::l10n::english();
        resolve_device_and_clock(&l, device.map(str::to_string), None, false, scale, preseed)
    }

    /// As [`resolve`], for a session playing back a recording of `device`.
    fn resolve_replay(
        device: &'static str,
        preseed: Option<f64>,
    ) -> Result<(Option<String>, dmm_lib::Clock), Refusal> {
        let l = crate::l10n::english();
        resolve_device_and_clock(&l, None, Some(device), false, None, preseed)
    }

    /// The refusal of the clock flags on a hardware device.
    fn mock_only() -> Refusal {
        Refusal {
            id: "device",
            reason: crate::l10n::english()
                .help()
                .mock_clock_mock_only()
                .into_owned(),
        }
    }

    #[test]
    fn without_the_clock_flags_the_session_runs_on_wall_time() {
        let (device, clock) = resolve(None, None, None).expect("no flags");
        assert_eq!(device, None, "no flag implies no device");
        assert!(clock.is_real());
    }

    /// `dmm-gui --mock-clock-preseed 90` has to be a complete invocation, the
    /// way `--mock-mode dcv` is.
    #[test]
    fn a_clock_flag_alone_implies_the_mock_device() {
        for (scale, preseed) in [(Some(30.0), None), (None, Some(90.0))] {
            let (device, clock) = resolve(None, scale, preseed).expect("clock flag");
            assert_eq!(device.as_deref(), Some("mock"));
            assert!(!clock.is_real());
        }
    }

    #[test]
    fn an_explicit_mock_device_takes_the_clock_flags() {
        let (device, clock) = resolve(Some("mock"), None, Some(90.0)).expect("mock device");
        assert_eq!(device.as_deref(), Some("mock"));
        assert_eq!(clock.preseed(), std::time::Duration::from_secs(90));
    }

    /// Virtual time on a USB-paced meter would stamp readings with instants it
    /// never produced, so the flags are refused rather than ignored.
    #[test]
    fn a_hardware_device_refuses_the_clock_flags() {
        assert_eq!(
            resolve(Some("ut61eplus"), None, Some(90.0)).unwrap_err(),
            mock_only()
        );
        assert_eq!(
            resolve(Some("ut61eplus"), Some(30.0), None).unwrap_err(),
            mock_only()
        );
        // Without the flags the same device is of course fine.
        assert!(resolve(Some("ut61eplus"), None, None).is_ok());
    }

    /// `auto` names no meter, but the one it finds is real hardware — so it
    /// takes the refusal, and is not mistaken for an unknown device that
    /// silently leaves the flags alone.
    #[test]
    fn auto_detect_is_hardware_for_the_clock_flags() {
        assert_eq!(
            resolve(Some("auto"), None, Some(90.0)).unwrap_err(),
            mock_only()
        );
        let (device, clock) = resolve(Some("auto"), None, None).expect("no clock flags");
        assert_eq!(device.as_deref(), Some("auto"));
        assert!(clock.is_real());
    }

    /// A recording is a hardware meter's frames, but no meter is on the cable
    /// and the file paces its own playback — so the clock flags apply, and a
    /// preseed can put a whole recording on screen at launch.
    #[test]
    fn a_replay_takes_the_clock_flags() {
        let (device, clock) = resolve_replay("ut61eplus", Some(90.0)).expect("replay");
        assert_eq!(device.as_deref(), Some("ut61eplus"));
        assert_eq!(clock.preseed(), std::time::Duration::from_secs(90));
    }

    /// The file says which meter it was recorded from, so that is the meter
    /// the session runs as — clap refuses a `--device` that would disagree.
    #[test]
    fn a_replay_names_its_own_device() {
        let (device, clock) = resolve_replay("ut181a", None).expect("replay");
        assert_eq!(device.as_deref(), Some("ut181a"));
        assert!(clock.is_real());
    }

    /// And the refusal is clap's, so it is spelled out before anything opens.
    #[test]
    fn a_replay_refuses_a_second_answer_to_which_meter() {
        for extra in [["--device", "mock"], ["--mock-mode", "dcv"]] {
            let err = Args::try_parse_from(
                ["dmm-gui", "--replay", "session.replay"]
                    .into_iter()
                    .chain(extra),
            )
            // `.err()`, not `expect_err`: `Args` is not `Debug`.
            .err()
            .expect("the recording already names the meter");
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{err}"
            );
        }
    }

    #[test]
    fn the_args_are_well_formed() {
        Args::command().debug_assert();
    }

    /// `--completions` prints a script and exits, so a flag given with it
    /// would be silently dropped; clap refuses the pair instead.
    #[test]
    fn completions_take_no_other_flag() {
        let err = Args::try_parse_from(["dmm-gui", "--completions", "zsh", "--device", "mock"])
            .err()
            .expect("--completions stands alone");
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "{err}"
        );
    }

    /// The values are keyed on the flags' ids; a renamed field would drop
    /// them from the script without an error.
    #[test]
    fn completions_offer_devices_and_mock_modes() {
        let mut script = Vec::new();
        clap_complete::generate(
            clap_complete::Shell::Bash,
            &mut completion_command(),
            "dmm-gui",
            &mut script,
        );
        let script = String::from_utf8(script).unwrap();
        assert!(script.contains("auto ut61eplus"), "no device ids");
        assert!(script.contains("dcv acv"), "no mock modes");
    }

    /// `max` converts a replay in `dmm-cli read`; the GUI says so rather
    /// than calling it a bad number.
    #[test]
    fn the_max_clock_scale_is_refused_by_name() {
        let err = Args::try_parse_from(["dmm-gui", "--mock-clock-scale", "max"])
            .err()
            .expect("the GUI takes a factor");
        assert!(err.to_string().contains("dmm-cli read"), "{err}");
        assert!(Args::try_parse_from(["dmm-gui", "--mock-clock-scale", "20"]).is_ok());
    }

    /// The message clap prints has to say which flag was wrong; the reason
    /// comes from the shared validator.
    #[test]
    fn a_bad_value_is_reported_against_its_flag() {
        let err = resolve(None, Some(0.0), None).unwrap_err();
        assert_eq!(err.id, "mock_clock_scale");
        assert!(err.reason.contains("got '0'"), "{err:?}");
        let err = resolve(None, Some(2.0), Some(-1.0)).unwrap_err();
        assert_eq!(err.id, "mock_clock_preseed");
        assert!(err.reason.contains("got '-1'"), "{err:?}");
    }

    const GUI_HELP_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../i18n/en-US/gui-help.ftl");

    const GUI_HELP_HEADER: &str = "\
dmm-gui's --help: what the program and each option do.

Generated from the doc comments in crates/dmm-gui/src/main.rs by
`UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl`: edit those, not this
file. The run names each ID whose English changed. When the meaning changed
too, delete that message from every language's gui-help.ftl, so it shows in
English until it is translated again.

IDs follow the options: dmm-gui--replay.help is what --replay does. clap
wraps each paragraph to the terminal's width: keep a paragraph on one line.
Option names (--device), values (auto, mock) and command lines ('dmm-cli
list') stay as they are. The help built as the program runs is in
gui/command-line.ftl, and clap's own words around these texts (Usage:,
Options:, [possible values: …]) are in shared.ftl.";

    /// `gui-help.ftl` holds the doc comments as they are now.
    #[test]
    fn gui_help_ftl_follows_the_doc_comments() {
        use std::io::Write;
        let path = std::path::Path::new(GUI_HELP_PATH);
        let current = std::fs::read_to_string(path).unwrap_or_default();
        let rendered = clap_help::render(&Args::command(), GUI_HELP_HEADER);
        if rendered == current {
            return;
        }
        let changes = clap_help::changes(&current, &rendered);
        let summary = format!(
            "English changed: {:?}\nadded: {:?}\nremoved: {:?}",
            changes.changed, changes.added, changes.removed
        );
        if std::env::var_os("UPDATE_L10N").is_some() {
            dmm_shared::write_atomic(path, rendered.as_bytes()).expect("write gui-help.ftl");
            // Past the test harness's capture, so a passing run shows it.
            let _ = writeln!(
                std::io::stderr(),
                "i18n/en-US/gui-help.ftl regenerated.\n{summary}"
            );
        } else {
            panic!(
                "i18n/en-US/gui-help.ftl is out of date; run \
                 `UPDATE_L10N=1 cargo test -p dmm-gui gui_help_ftl` and commit it.\n{summary}"
            );
        }
    }

    /// No option's values say what they do, so none needs a parser of its
    /// own to carry their translated help.
    #[test]
    fn no_value_is_described() {
        let unmapped = clap_help::unmapped_values(Args::command(), &no_described_values);
        assert!(unmapped.is_empty(), "{unmapped:?}");
    }

    /// `-h` or `--help`, as `cmd` prints it.
    fn page(cmd: clap::Command, flag: &str) -> String {
        match cmd.try_get_matches_from(["dmm-gui", flag]) {
            Ok(_) => panic!("{flag} printed no help"),
            Err(e) => e.render().to_string(),
        }
    }

    /// Walked in English, the help reads as the doc comments and the run
    /// time's help alone write it, byte for byte.
    #[test]
    fn the_english_walk_leaves_the_help_as_it_was() {
        let l = crate::l10n::english();
        for flag in ["-h", "--help"] {
            assert_eq!(
                page(command(&l), flag),
                page(with_run_time_help(Args::command(), &l), flag),
                "{flag}"
            );
        }
    }

    /// Laid out by the shared help layout in English, the help reads
    /// exactly as clap's own.
    #[test]
    fn english_layout_is_claps() {
        let l = crate::l10n::english();
        let differences = help_layout::english_layout_differences(command(&l), &l);
        assert!(differences.is_empty(), "{}", differences.join("\n\n"));
    }

    /// Under the pseudo-locale, which marks every translated word, no
    /// heading or note of clap's is left in English.
    #[test]
    fn no_english_layout_is_left() {
        use dmm_shared::l10n::{EN_US, Options};
        let options = Options {
            pseudo: true,
            ..Options::default()
        };
        let l = Localizer::new(&EN_US, &crate::l10n::resources(), &options);
        let left = help_layout::english_left(command(&l), &l);
        assert!(left.is_empty(), "{}", left.join("\n\n"));
    }

    /// What clap prints for `args`, and what the shared renderer makes of
    /// the same error in English.
    fn both(args: &[&str]) -> (String, String) {
        let l = crate::l10n::english();
        let e = command(&l)
            .try_get_matches_from(args)
            .expect_err("an error");
        let ours = arg_errors::render(&e, &l, &[]).to_string();
        (e.render().to_string(), ours)
    }

    /// Mistakes users make on dmm-gui read as clap words them.
    #[test]
    fn errors_match_clap() {
        for args in [
            &["dmm-gui", "--renderer", "vulkan"][..],
            &["dmm-gui", "--rendrer", "glow"],
            &["dmm-gui", "--renderer"],
            &["dmm-gui", "--mock-mode", "volts"],
            &["dmm-gui", "--replay", "a.replay", "--device", "mock"],
            &["dmm-gui", "--completions", "fish", "--no-bluetooth"],
            &["dmm-gui", "--mock-clock-scale", "max"],
            &["dmm-gui", "extra"],
        ] {
            let (clap, ours) = both(args);
            assert_eq!(ours, clap, "{args:?}");
        }
    }

    /// A value refused after parsing reads as one clap's own parser
    /// refuses, and in English as clap words it.
    #[test]
    fn a_value_refused_after_parsing_reads_as_claps() {
        let l = crate::l10n::english();
        let mut cmd = command(&l);
        cmd.build();
        for (e, first) in [
            (
                refused(&cmd, "device", "ut62", "why".to_string()),
                "error: invalid value 'ut62' for '--device <DEVICE>': why",
            ),
            (
                not_one_of(&cmd, "theme", "sepia", theme_values(&[])),
                "error: invalid value 'sepia' for '--theme <THEME>'",
            ),
        ] {
            let ours = arg_errors::render(&e, &l, &[]).to_string();
            assert_eq!(ours, e.render().to_string());
            assert!(ours.starts_with(first), "{ours}");
        }
    }
}
