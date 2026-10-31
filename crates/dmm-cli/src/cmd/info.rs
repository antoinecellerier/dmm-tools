//! `info`: the meter's name, what detection found, and the link it answered on.
//!
//! The labels are in the run's language (`cli/info.ftl`); the link details
//! after them are the library's, in English.

use crate::open::{AUTO_DETECTED, open_with_help, picked_line, print_reported_name, prose_name};
use console::style;
use dmm_lib::protocol::registry::Selection;
use dmm_shared::tr;

pub(crate) fn cmd_info(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let l = crate::l10n::get();
    let (mut dmm, _device) = open_with_help(selection, opts)?;
    // The name detection got, if it ran: the session keeps it.
    let reported = dmm.get_name()?;
    // Which meter the readings are decoded as: the one picked, and where,
    // or the one detection found. The meter's own name follows, the clue
    // when the two disagree.
    if let Some(line) = picked_line(l) {
        println!("{line}");
    } else if let Some(detected) = AUTO_DETECTED.get() {
        let meter = style(prose_name(detected.device.display_name())).bold();
        println!("{}", tr!(l, "info-detected", meter = meter.to_string()));
    }
    print_reported_name(l, reported.as_deref(), |line| println!("{line}"));

    let transport = dmm.transport().transport_name();
    println!("{}", tr!(l, "info-transport", transport = transport));
    if let Ok(info) = dmm.transport().transport_info() {
        println!("  {info}");
    }
    if let Ok(status) = dmm.transport().transport_status() {
        println!("  {}", tr!(l, "info-status", status = status));
    }

    Ok(())
}
