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
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let overrides = parse_args();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dmm-tools",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, overrides)))),
    )
}
