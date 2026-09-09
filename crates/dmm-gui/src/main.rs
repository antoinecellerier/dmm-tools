mod a11y;
mod app;
mod changelog;
mod display;
mod graph;
mod recording;
mod settings;
mod specs;
mod theme;

use clap::{CommandFactory, FromArgMatches, Parser};
use dmm_lib::protocol::registry;

/// Placeholder string for missing/unavailable data values in the UI.
pub(crate) const NO_DATA: &str = "---";

fn version_string() -> &'static str {
    dmm_lib::binary_help::version_string(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

/// Version string for the app (shown in top bar, right side).
pub fn version_label() -> String {
    dmm_lib::binary_help::version_label(env!("CARGO_PKG_VERSION"), env!("GIT_HASH"))
}

#[derive(Parser)]
#[command(
    name = "dmm-gui",
    version = version_string(),
    about = "GUI application for UNI-T and Voltcraft digital multimeters",
    after_long_help = "Help / GitHub: https://github.com/antoinecellerier/dmm-tools"
)]
struct Args {
    /// Device to connect to [ut61eplus, ut8803, ut171, ut181a, mock, ...]
    #[arg(long)]
    device: Option<String>,

    /// Pin mock device to a specific mode (implies --device mock)
    #[arg(long, long_help = build_mock_mode_help())]
    mock_mode: Option<String>,

    /// Theme override [dark, light, system]
    #[arg(long)]
    theme: Option<String>,

    /// Graphics renderer [wgpu, glow]
    #[arg(long)]
    renderer: Option<String>,

    /// Select a specific USB adapter when multiple are connected.
    /// Use serial number or HID device path from 'dmm-cli list' output.
    #[arg(long, value_name = "SERIAL_OR_PATH")]
    adapter: Option<String>,

    /// Run session time at FACTOR times real time (mock only, implies
    /// --device mock). Hidden: a contributor tool for screenshots and
    /// performance runs, documented in docs/development.md.
    #[arg(long, hide = true, value_name = "FACTOR")]
    mock_clock_scale: Option<f64>,

    /// Start the session with SECS of history, produced instantly (mock
    /// only, implies --device mock). Hidden for the same reason as
    /// --mock-clock-scale.
    #[arg(long, hide = true, value_name = "SECS")]
    mock_clock_preseed: Option<f64>,
}

/// Build long help text for --device from the registry.
fn build_device_help() -> String {
    dmm_lib::binary_help::device_help(
        "Device to connect to. Overrides saved settings for this session.",
    )
}

/// Build long help text for --mock-mode from the mock's own mode table.
fn build_mock_mode_help() -> String {
    dmm_lib::binary_help::mock_mode_help(
        "Pin the mock device to a specific measurement mode instead of \
         auto-cycling. Implies --device mock, and overrides the saved Mock \
         mode setting for this session.",
        "dmm-gui --mock-mode dcv",
    )
}

/// CLI overrides to apply on top of persisted settings for this session.
pub struct CliOverrides {
    pub device: Option<String>,
    pub mock_mode: Option<dmm_lib::mock::MockMode>,
    pub theme: Option<settings::ThemeMode>,
    pub renderer: Option<eframe::Renderer>,
    pub adapter: Option<String>,
    /// Time base for this session's readings: real unless a `--mock-clock-*`
    /// flag was given.
    pub clock: dmm_lib::Clock,
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
/// was not given.
fn resolve_device_and_clock(
    device: Option<String>,
    mock_mode_given: bool,
    scale: Option<f64>,
    preseed: Option<f64>,
) -> Result<(Option<String>, dmm_lib::Clock), String> {
    let clock = dmm_lib::Clock::from_flags(scale, preseed)
        .map_err(|e| format!("--mock-clock-scale / --mock-clock-preseed: {e}"))?;

    let hardware = device
        .as_deref()
        .and_then(registry::resolve_device)
        .is_some_and(|d| d.requires_hardware);
    if !clock.is_real() && hardware {
        return Err(dmm_lib::binary_help::MOCK_CLOCK_MOCK_ONLY.to_string());
    }

    // --mock-mode and either clock flag each imply --device mock, so
    // `dmm-gui --mock-clock-preseed 90` is a complete invocation.
    let device = match device {
        d @ Some(_) => d,
        None if mock_mode_given || !clock.is_real() => Some("mock".to_string()),
        None => None,
    };
    Ok((device, clock))
}

fn parse_args() -> CliOverrides {
    let mut cmd = Args::command();
    let device_help = build_device_help();
    cmd = cmd.mut_arg("device", |a| a.long_help(device_help));
    let args = Args::from_arg_matches_mut(&mut cmd.get_matches()).unwrap_or_else(|e| e.exit());

    // Validate and canonicalize --device if provided
    let device = args.device.map(|raw| match registry::resolve_device(&raw) {
        Some(d) => d.id.to_string(),
        None => {
            Args::command()
                .error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("unknown device '{raw}'. Run with --help to see available devices."),
                )
                .exit();
        }
    });

    // Parse --theme if provided
    let theme = args.theme.as_deref().map(|t| match t {
        "dark" => settings::ThemeMode::Dark,
        "light" => settings::ThemeMode::Light,
        "system" => settings::ThemeMode::System,
        other => {
            Args::command()
                .error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("unknown theme '{other}'. Valid options: dark, light, system"),
                )
                .exit();
        }
    });

    // Parse --mock-mode once and carry the mode, not the string: the settings
    // field is a string only because it is persisted, and re-parsing it later
    // has to invent an answer for a value already rejected here. The rejection
    // is the mock's own message, so it names the modes the mock really has.
    let mock_mode = args.mock_mode.as_deref().map(|raw| {
        raw.parse::<dmm_lib::mock::MockMode>().unwrap_or_else(|e| {
            Args::command()
                .error(clap::error::ErrorKind::InvalidValue, e)
                .exit()
        })
    });

    // Parse --renderer if provided
    let renderer = args.renderer.as_deref().map(|r| match r {
        "wgpu" => eframe::Renderer::Wgpu,
        "glow" => eframe::Renderer::Glow,
        other => {
            Args::command()
                .error(
                    clap::error::ErrorKind::InvalidValue,
                    format!("unknown renderer '{other}'. Valid options: wgpu, glow"),
                )
                .exit();
        }
    });

    let (device, clock) = resolve_device_and_clock(
        device,
        mock_mode.is_some(),
        args.mock_clock_scale,
        args.mock_clock_preseed,
    )
    .unwrap_or_else(|message| {
        Args::command()
            .error(clap::error::ErrorKind::InvalidValue, message)
            .exit()
    });

    CliOverrides {
        device,
        mock_mode,
        theme,
        renderer,
        adapter: args.adapter,
        clock,
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
        let desktop = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=dmm-tools\n\
             Comment=Real-time display and plotting for UNI-T multimeters\n\
             Exec={}\n\
             Icon=dmm-tools\n\
             Categories=Utility;Electronics;\n\
             StartupWMClass=dmm-tools\n",
            exe_path.display()
        );
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
    env_logger::init();

    #[cfg(target_os = "linux")]
    install_desktop_integration();

    let overrides = parse_args();

    // Embedded icon for the window titlebar / taskbar. On Windows and macOS this
    // is all that's needed. On Linux/Wayland the icon is looked up from the
    // .desktop file instead — see install_desktop_integration().
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon-256.png"))
        .expect("failed to load app icon");

    let explicit_renderer = overrides.renderer.is_some();
    let renderer = overrides.renderer.unwrap_or(eframe::Renderer::Wgpu);

    let viewport = eframe::egui::ViewportBuilder::default()
        .with_app_id("dmm-tools")
        .with_icon(std::sync::Arc::new(icon))
        .with_inner_size([960.0, 640.0])
        .with_min_inner_size([200.0, 150.0]);

    let options = eframe::NativeOptions {
        viewport: viewport.clone(),
        renderer,
        ..Default::default()
    };

    let result = eframe::run_native(
        "dmm-tools",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, overrides)))),
    );

    // If wgpu failed and wasn't explicitly requested, retry with glow
    if result.is_err() && !explicit_renderer {
        log::warn!("wgpu renderer failed, falling back to glow");
        let fallback_options = eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        };
        let fallback_overrides = parse_args();
        return eframe::run_native(
            "dmm-tools",
            fallback_options,
            Box::new(move |cc| Ok(Box::new(app::App::new(cc, fallback_overrides)))),
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(
        device: Option<&str>,
        scale: Option<f64>,
        preseed: Option<f64>,
    ) -> Result<(Option<String>, dmm_lib::Clock), String> {
        resolve_device_and_clock(device.map(str::to_string), false, scale, preseed)
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
            dmm_lib::binary_help::MOCK_CLOCK_MOCK_ONLY
        );
        assert_eq!(
            resolve(Some("ut61eplus"), Some(30.0), None).unwrap_err(),
            dmm_lib::binary_help::MOCK_CLOCK_MOCK_ONLY
        );
        // Without the flags the same device is of course fine.
        assert!(resolve(Some("ut61eplus"), None, None).is_ok());
    }

    /// The message clap prints has to say which flag was wrong; the value
    /// itself comes from the shared validator.
    #[test]
    fn a_bad_value_is_reported_against_its_flag() {
        let err = resolve(None, Some(0.0), None).unwrap_err();
        assert!(
            err.starts_with("--mock-clock-scale / --mock-clock-preseed: "),
            "{err}"
        );
        assert!(err.contains("got '0'"), "{err}");
    }
}
