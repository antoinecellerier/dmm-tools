//! `info`: the meter's name, what detection found, and the link it answered on.

use crate::open::{AUTO_DETECTED, open_with_help};
use console::style;
use dmm_lib::protocol::registry::Selection;

pub(crate) fn cmd_info(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut dmm, _device) = open_with_help(selection, opts)?;
    // The name detection got, if it ran: the session keeps it.
    match dmm.get_name()? {
        Some(ref n) => println!("Device: {}", style(n).bold()),
        None => println!("Device: {}", style("(name not supported)").dim()),
    }
    // Which tables are in use is only in question when the user named no
    // meter, so the line is only there when they didn't.
    if let Some(detected) = AUTO_DETECTED.get() {
        let reported = match &detected.reported_name {
            Some(name) if name != detected.device.display_name => {
                format!(" (the meter reports {name:?})")
            }
            _ => String::new(),
        };
        println!(
            "Detected: {}{reported}",
            style(detected.device.display_name).bold()
        );
    }

    println!("Transport: {}", dmm.transport().transport_name());
    if let Ok(info) = dmm.transport().transport_info() {
        println!("  {info}");
    }
    if let Ok(status) = dmm.transport().transport_status() {
        println!("  Status: {status}");
    }

    Ok(())
}
