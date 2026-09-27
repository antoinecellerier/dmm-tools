//! `command`: a button press sent to the meter, or the list of the ones its
//! family implements.

use crate::open::{device_for_listing, open_mock_device, open_with_help, requires_hardware};
use console::style;
use dmm_lib::protocol::registry::{SelectableDevice, Selection};

pub(crate) fn cmd_command(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    action: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = match action {
        Some(a) => a,
        None => return print_available_commands(device_for_listing(selection, opts)?),
    };

    if requires_hardware(selection) {
        let (mut dmm, _device) = open_with_help(selection, opts)?;
        dmm.send_command(&action)?;
    } else {
        let mut dmm = open_mock_device(selection, None, dmm_lib::Clock::real())?;
        dmm.send_command(&action)?;
    }
    println!("{} {action}", style("Sent").green());
    Ok(())
}

/// Print supported commands for a device without connecting.
fn print_available_commands(
    device: &'static SelectableDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol = (device.new_protocol)();
    let profile = protocol.profile();
    if profile.supported_commands.is_empty() {
        eprintln!(
            "{} No remote commands implemented yet for {}.",
            style("Note:").yellow(),
            profile.model_name,
        );
    } else {
        println!(
            "Available commands for {}:",
            style(profile.model_name).bold()
        );
        for cmd in profile.supported_commands {
            println!("  {cmd}");
        }
    }
    Ok(())
}
