//! `info`: the meter's name, what detection found, and the link it answered on.
//!
//! The labels are in the run's language (`cli/info.ftl`); the link details
//! after them are the library's, in English.

use crate::open::{open_with_help, picked_line, print_reported_name, prose_name};
use console::style;
use dmm_lib::protocol::registry::Selection;
use dmm_shared::tr;

pub(crate) fn cmd_info(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(), crate::l10n::Failure> {
    let l = crate::l10n::get();
    let (mut dmm, device) = open_with_help(selection, opts)?;
    // The name detection got, if it ran: the session keeps it.
    let reported = dmm.get_name()?;
    // Which meter the readings are decoded as: the one picked, and where,
    // or the one detection found. The meter's own name follows, the clue
    // when the two disagree.
    if let Some(line) = picked_line(l) {
        println!("{line}");
    } else if matches!(selection, Selection::Auto) {
        let meter = style(prose_name(device.display_name())).bold();
        println!("{}", tr!(l, "info-detected", meter = meter.to_string()));
    }
    print_reported_name(l, reported.as_deref(), |line| println!("{line}"));
    crate::open::warn_other_model(&dmm, device);

    let transport = dmm.transport().transport_name();
    println!("{}", tr!(l, "info-transport", transport = transport));
    match dmm.transport().transport_info() {
        Ok(Some(info)) => println!("  {info}"),
        Ok(None) => {}
        // The chip's identity has no label of its own; a failure needs one.
        Err(e) => println!(
            "  {}",
            tr!(l, "info-bridge", bridge = super::unavailable(l, &e))
        ),
    }
    if let Some(status) = super::link_detail(l, dmm.transport().transport_status()) {
        println!("  {}", tr!(l, "info-status", status = status));
    }

    Ok(())
}
