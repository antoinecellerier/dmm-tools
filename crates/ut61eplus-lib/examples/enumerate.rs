//! List all connected multimeter USB cables.

fn main() {
    env_logger::init();

    match ut61eplus_lib::list_devices() {
        Ok(devices) => {
            if devices.is_empty() {
                eprintln!("No devices found.");
                eprintln!("Check USB connection.");
                #[cfg(target_os = "linux")]
                {
                    eprintln!("Ensure udev rules are installed (see udev/99-dmm-tools.rules).");
                    eprintln!(
                        "Your user must be in the plugdev group: sudo usermod -aG plugdev $USER"
                    );
                }
                #[cfg(target_os = "macos")]
                eprintln!(
                    "On macOS, the CP2110 should be recognized automatically (no driver needed)."
                );
                std::process::exit(1);
            }
            for (i, dev) in devices.iter().enumerate() {
                println!("[{i}] {dev}");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
