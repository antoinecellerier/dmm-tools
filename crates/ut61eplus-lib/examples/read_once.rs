//! Request a single measurement from the UT61E+ and print the result.

fn main() {
    env_logger::init();

    let mut dmm = match ut61eplus_lib::open_device_by_id_auto("ut61eplus") {
        Ok(dmm) => dmm,
        Err(e) => {
            eprintln!("Error opening device: {e}");
            std::process::exit(1);
        }
    };

    match dmm.request_measurement() {
        Ok(m) => {
            println!("{m}");
            println!("  Mode:  {}", m.mode);
            println!("  Range: {}", m.range_label);
            println!("  Raw:   {:?}", m.display_raw.as_deref().unwrap_or(""));
            println!("  Flags: {}", m.flags);
        }
        Err(e) => {
            eprintln!("Error reading measurement: {e}");
            std::process::exit(1);
        }
    }
}
