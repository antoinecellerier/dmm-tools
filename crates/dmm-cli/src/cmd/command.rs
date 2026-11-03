//! `command`: a button press sent to the meter, or the list of the ones its
//! family implements.
//!
//! The command words are typed by the user and stay as they are; the lines
//! around them are in the run's language (`cli/command.ftl`).

use crate::l10n::note_label;
use crate::open::{device_for_listing, open_any, profile_name, prose_name};
use console::style;
use dmm_lib::protocol::registry::{SelectableDevice, Selection};
use dmm_shared::tr;

pub(crate) fn cmd_command(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    action: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = match action {
        Some(a) => a,
        None => return print_available_commands(device_for_listing(selection, opts)?),
    };

    open_any(selection, opts, None)?.send_command(&action)?;
    let sent = tr!(crate::l10n::get(), "command-sent-label");
    println!("{} {action}", style(sent).green());
    Ok(())
}

/// Print supported commands for a device without connecting.
fn print_available_commands(
    device: &'static SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol = (device.new_protocol)();
    let profile = protocol.profile();
    let l = crate::l10n::get();
    let meter = prose_name(profile_name(profile));
    if profile.supported_commands.is_empty() {
        eprintln!("{} {}", note_label(), tr!(l, "command-none", meter = meter));
    } else {
        let meter = style(meter).bold().to_string();
        println!("{}", tr!(l, "command-available", meter = meter));
        for cmd in profile.supported_commands {
            println!("  {cmd}");
        }
    }
    Ok(())
}
