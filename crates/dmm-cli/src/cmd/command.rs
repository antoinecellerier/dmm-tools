//! `command`: a button press sent to the meter, or the list of the ones its
//! family implements.
//!
//! The command words are typed by the user and stay as they are; the lines
//! around them are in the run's language (`cli/command.ftl`).

use crate::l10n::note_label;
use crate::open::{device_for_listing, open_any, profile_name, prose_name};
use console::style;
use dmm_lib::error::{Error, Unsupported};
use dmm_lib::protocol::registry::{SelectableDevice, Selection};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;

pub(crate) fn cmd_command(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    action: Option<String>,
) -> Result<(), crate::l10n::Failure> {
    let action = match action {
        Some(a) => a,
        None => return print_available_commands(device_for_listing(selection, opts)?),
    };

    let mut dmm = open_any(selection, opts, None)?;
    match dmm.send_command(&action) {
        // A name the meter doesn't have, most often mistyped: say which it
        // has, rather than send the user to look them up.
        Err(Error::UnsupportedCommand(Unsupported::Command(_))) => {
            let profile = dmm.profile();
            return Err(UnknownCommand {
                meter: profile_name(profile),
                command: action,
                commands: profile.supported_commands,
            }
            .into());
        }
        sent => sent?,
    }
    let sent = tr!(crate::l10n::get(), "command-sent-label");
    println!("{} {action}", style(sent).green());
    Ok(())
}

/// A command name the meter doesn't have, and the ones it takes, or that it
/// takes none: the closing `Error:` line.
#[derive(Debug)]
pub(crate) struct UnknownCommand {
    meter: &'static str,
    command: String,
    commands: &'static [&'static str],
}

impl crate::l10n::Localized for UnknownCommand {
    fn text<'a>(&'a self, l: &'a Localizer) -> std::borrow::Cow<'a, str> {
        let meter = l.label(TextKind::Model, self.meter);
        if self.commands.is_empty() {
            return tr!(l, "command-unknown-none", meter = meter);
        }
        tr!(
            l,
            "command-unknown",
            meter = meter,
            command = self.command.as_str(),
            commands = self.commands.join(", "),
        )
    }
}

crate::l10n::localized_error!(UnknownCommand);

/// Print supported commands for a device without connecting.
fn print_available_commands(device: &'static SelectableDevice) -> Result<(), crate::l10n::Failure> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The error names what the meter takes, or that it takes nothing.
    #[test]
    fn an_unknown_command_says_what_the_meter_takes() {
        let l = crate::l10n::english();
        let unknown = |commands| UnknownCommand {
            meter: "UNI-T UT8803",
            command: "hold".to_string(),
            commands,
        };
        assert_eq!(
            unknown(&["hold", "range"]).to_string(),
            "the UNI-T UT8803 has no 'hold' command; it takes hold, range"
        );
        assert_eq!(
            crate::l10n::Localized::text(&unknown(&[]), l),
            "the UNI-T UT8803 takes no remote commands yet"
        );
    }
}
