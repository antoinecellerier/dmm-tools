//! `info`: the meter's name, what detection found, and the link it answered on.

use crate::open::{AUTO_DETECTED, open_with_help, picked_line, print_reported_name};
use console::style;
use dmm_lib::protocol::registry::Selection;

pub(crate) fn cmd_info(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut dmm, _device) = open_with_help(selection, opts)?;
    // The name detection got, if it ran: the session keeps it.
    let reported = dmm.get_name()?;
    // Which meter the readings are decoded as: the one picked, and where,
    // or the one detection found. The meter's own name follows, the clue
    // when the two disagree.
    if let Some(line) = picked_line() {
        println!("{line}");
    } else if let Some(detected) = AUTO_DETECTED.get() {
        println!("Detected: {}", style(detected.device.display_name()).bold());
    }
    print_reported_name(reported.as_deref(), |line| println!("{line}"));

    println!("Transport: {}", dmm.transport().transport_name());
    if let Ok(info) = dmm.transport().transport_info() {
        println!("  {info}");
    }
    if let Ok(status) = dmm.transport().transport_status() {
        println!("  Status: {status}");
    }

    Ok(())
}
