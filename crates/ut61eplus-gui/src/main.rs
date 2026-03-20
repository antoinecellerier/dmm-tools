mod app;
mod display;
mod graph;
mod recording;
mod settings;
mod specs;
mod stats;
mod theme;

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

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dmm-tools",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
