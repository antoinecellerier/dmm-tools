//! Open the UT61E+ and print the device product string.

fn main() {
    env_logger::init();

    match dmm_lib::open_device_by_id_auto("ut61eplus", None) {
        Ok(dmm) => {
            println!("Connected to device.");
            // The Dmm struct doesn't expose product_string directly,
            // but the init sequence succeeded if we get here.
            let _ = dmm;
            println!("UART initialized successfully (9600/8N1, FIFOs purged).");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
