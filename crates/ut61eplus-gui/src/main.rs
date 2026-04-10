mod app;
mod display;
mod graph;
mod recording;
mod settings;
mod specs;
mod theme;

use clap::{CommandFactory, FromArgMatches, Parser};
use ut61eplus_lib::protocol::registry;

/// Placeholder string for missing/unavailable data values in the UI.
pub(crate) const NO_DATA: &str = "---";

fn version_string() -> &'static str {
    let version = env!("CARGO_PKG_VERSION");
    let hash = env!("GIT_HASH");
    if version.contains("-dev") {
        Box::leak(format!("{version} ({hash})").into_boxed_str())
    } else {
        version
    }
}

/// Version string for the app (shown in top bar, right side).
pub fn version_label() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let hash = env!("GIT_HASH");
    if version.contains("-dev") {
        format!("v{version} ({hash})")
    } else {
        format!("v{version}")
    }
}

#[derive(Parser)]
#[command(
    name = "ut61eplus-gui",
    version = version_string(),
    about = "UNI-T multimeter GUI",
    after_long_help = "Help / GitHub: https://github.com/antoinecellerier/dmm-tools"
)]
struct Args {
    /// Device to connect to [ut61eplus, ut8803, ut171, ut181a, mock, ...]
    #[arg(long)]
    device: Option<String>,

    /// Pin mock device to a specific mode (only with --device mock)
    #[arg(long)]
    mock_mode: Option<String>,

    /// Theme override [dark, light, system]
    #[arg(long)]
    theme: Option<String>,

    /// Graphics renderer [wgpu, glow]
    #[arg(long)]
    renderer: Option<String>,

    /// Select a specific USB adapter when multiple are connected.
    /// Use serial number or HID device path from 'ut61eplus list' output.
    #[arg(long, value_name = "SERIAL_OR_PATH")]
    adapter: Option<String>,
}

/// Build long help text for --device from the registry.
fn build_device_help() -> String {
    let mut help = String::from(
        "Device to connect to. Overrides saved settings for this session.\n\nDevices:\n",
    );
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
    help.push_str(
        "\nAlso accepts aliases: ut61e+, ut61b, ut171a, ut181, etc.\n\
         Quote names with special characters: --device 'ut61e+'",
    );
    help
}

/// CLI overrides to apply on top of persisted settings for this session.
pub struct CliOverrides {
    pub device: Option<String>,
    pub mock_mode: Option<String>,
    pub theme: Option<settings::ThemeMode>,
    pub renderer: Option<eframe::Renderer>,
    pub adapter: Option<String>,
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

    // Validate --mock-mode if provided
    if let Some(ref mode) = args.mock_mode
        && mode.parse::<ut61eplus_lib::mock::MockMode>().is_err()
    {
        let valid: Vec<&str> = ut61eplus_lib::mock::MockMode::ALL
            .iter()
            .map(|m| m.label())
            .collect();
        Args::command()
            .error(
                clap::error::ErrorKind::InvalidValue,
                format!(
                    "unknown mock mode '{mode}'. Valid modes: {}",
                    valid.join(", ")
                ),
            )
            .exit();
    }

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

    // --mock-mode implies --device mock
    let device = match (device, &args.mock_mode) {
        (d @ Some(_), _) => d,
        (None, Some(_)) => Some("mock".to_string()),
        (None, None) => None,
    };

    CliOverrides {
        device,
        mock_mode: args.mock_mode,
        theme,
        renderer,
        adapter: args.adapter,
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
    for (rel_path, content) in icons {
        let path = data_dir.join(rel_path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, content);
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
        let app_dir = data_dir.join("applications");
        let _ = fs::create_dir_all(&app_dir);
        let _ = fs::write(app_dir.join("dmm-tools.desktop"), desktop);
    }

    // Update the GTK icon cache so GNOME can find the icon without a session restart.
    let icon_dir = data_dir.join("icons/hicolor");
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(&icon_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    log::debug!("desktop integration files installed");
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
