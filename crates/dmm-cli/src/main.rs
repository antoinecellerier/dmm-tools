mod capture;
mod choice;
mod cli;
mod cmd;
mod format;
mod l10n;
mod open;
mod output;
#[cfg(test)]
mod test_fixtures;

use clap::FromArgMatches;
use clap_complete::Shell;
use cli::{CaptureArgs, Cli, Cmd};
use cmd::command::cmd_command;
use cmd::debug::cmd_debug;
use cmd::info::cmd_info;
use cmd::list::cmd_list;
use cmd::read::ReadPlan;
use cmd::settings::{cmd_get, cmd_set};
use console::style;
use dmm_lib::protocol::registry::{self, SelectableDevice, Selection};
use dmm_shared::help::LinksSearched;
use dmm_shared::l10n::{Localizer, arg_errors, help_layout};
use dmm_shared::tr;
use l10n::{Localized, error_label};
use log::error;
use open::{
    device_for_listing, open_recording_with_help, opened_device, pinned_mock_mode,
    print_no_response_help, print_setup_sections, requires_hardware, selection_id,
};

fn main() {
    // First: the language decides every line printed from here on, `--help`
    // included, so `--lang` and the settings file are read before the
    // command line is parsed. Chinese prints right on Windows 11, in Windows
    // Terminal and in the old console (cmd.exe) alike.
    let (settings, unreadable) = match dmm_shared::SharedSettings::load_if_exists() {
        Ok(settings) => (settings, None),
        Err(e) => (None, Some(e)),
    };
    let lang = cli::lang_from_argv(std::env::args_os().skip(1));
    l10n::init(lang, settings.as_ref().map_or("", |s| s.language.as_str()));
    let l = l10n::get();
    // A broken file runs as if there were none, but says so: the CLI never
    // rewrites it, so the user is the one to fix it.
    if let Some(e) = unreadable {
        eprintln!(
            "{} {}",
            l10n::warning_label(),
            tr!(l, "connect-settings-unreadable", error = e.to_string()),
        );
    }
    let cli = cli::command(l)
        .try_get_matches()
        .and_then(|mut m| Cli::from_arg_matches_mut(&mut m))
        .unwrap_or_else(|e| {
            // `--help`, `--version` and a refused command line end here,
            // before the logger below: a translator trying a broken file
            // with `--help` would otherwise never hear what loading it found.
            dmm_shared::logging::init();
            l10n::get().log_load_report();
            arg_errors::exit(e, l10n::get(), &cli::hidden_subcommands())
        });
    debug_assert_eq!(cli.lang, lang, "the pre-scan read --lang as clap does");
    // Before the settings file's meter applies: a report names its own meter,
    // and `--device` as typed overrides it, never the settings file's
    // choice, which a newer build may have written. Its logger keeps what
    // the library logs beside the frame that caused it, so it is installed
    // after parsing, which logs nothing.
    if let Cmd::Triage(args) = &cli.command {
        capture::install_triage_logger();
        if let Err(e) = capture::cmd_triage(args, cli.device.as_deref()) {
            // English, like everything triage prints.
            eprintln!("{} {e}", style("Error:").red().bold());
            std::process::exit(1);
        }
        return;
    }
    dmm_shared::logging::init();
    // Built before the logger, so what loading it found waits until now.
    l10n::get().log_load_report();

    // Whether the meter was named on the command line, as opposed to coming
    // from the settings file or from detection. Only `read --replay` and
    // `--import` ask.
    let device_named = cli.device.is_some();

    // Nothing named a meter, so none is assumed: detection is the fallback,
    // and `--device` or the settings file pins a model when the user wants
    // one.
    let device_id = dmm_shared::resolve_device_family(
        cli.device.as_deref(),
        settings.as_ref(),
        registry::AUTO_DEVICE_ID,
    );
    // A replay or an import names its own meter; a saved pick is not it.
    if !matches!(&cli.command, Cmd::Read(a) if a.replay.is_some() || a.import.is_some()) {
        let by = if device_named {
            open::PickedBy::Flag
        } else {
            open::PickedBy::Settings
        };
        open::note_picked(&device_id, by);
    }
    let Some(selection) = registry::resolve_selection(&device_id) else {
        fail(tr!(l, "connect-unknown-device", device = &device_id));
    };

    let bluetooth = bluetooth_probing(cli.no_bluetooth, settings.as_ref());
    let opts = dmm_lib::OpenOptions {
        adapter: cli.adapter.as_deref(),
        bluetooth,
    };

    // Device-independent commands — handle before mock/real split
    let result = match cli.command {
        Cmd::List => {
            if !cmd_list(bluetooth) {
                std::process::exit(1);
            }
            Ok(())
        }
        // Run and returned from before the settings file was read.
        Cmd::Triage(_) => Ok(()),
        Cmd::Completions { shell } => {
            print_completions(l, shell);
            Ok(())
        }

        // The mock is a registry device with nothing to open, so only the
        // commands that have no meaning without hardware branch on it here —
        // `read`, `command`, `get` and `set` pick the mock transport
        // themselves and take the same arm as every other device. Auto never
        // lands here: detecting a meter means opening a cable.
        Cmd::Info | Cmd::Debug { .. } if !requires_hardware(selection) => fail(tr!(
            l,
            "connect-needs-hardware",
            device = selection_id(selection)
        )),

        Cmd::Info => cmd_info(selection, opts),
        Cmd::Read(args) => match ReadPlan::new(l, args, selection, device_named) {
            Ok(plan) => plan.run(opts, settings.as_ref()),
            Err(refused) => fail(refused.text(l)),
        },
        Cmd::Command { action } => cmd_command(selection, opts, action),
        Cmd::Get {
            setting,
            format,
            mock_mode,
        } => cmd_get(
            selection,
            opts,
            setting,
            format,
            pinned_mock_mode(selection, mock_mode, settings.as_ref()),
        ),
        Cmd::Set {
            setting,
            choice,
            mock_mode,
        } => cmd_set(
            selection,
            opts,
            setting,
            choice,
            pinned_mock_mode(selection, mock_mode, settings.as_ref()),
        ),
        Cmd::Debug { count, interval_ms } => cmd_debug(selection, opts, count, interval_ms),
        Cmd::Capture(args) => run_capture(args, selection, opts),
    };

    if let Err(e) = result {
        report_failure(l, &*e, selection, cli.adapter.as_deref(), cli.no_bluetooth);
    }
}

/// Print `message` after the error label and end the run: a command line
/// refused before anything opened, said once, as clap says its own. A
/// command that ran and failed goes through [`report_failure`].
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{} {message}", error_label());
    std::process::exit(1);
}

/// `completions`: the script for `shell`, or with none named, the
/// command's own help.
fn print_completions(l: &Localizer, shell: Option<Shell>) {
    match shell {
        Some(shell) => {
            clap_complete::generate(
                shell,
                &mut cli::completion_command(),
                "dmm-cli",
                &mut std::io::stdout(),
            );
        }
        None => {
            // Built, so the usage line names the whole command as the
            // translated layout's does, in English too.
            let mut cmd = help_layout::invoked_as(cli::command(l));
            cmd.build();
            if let Some(completions) = cmd.find_subcommand_mut("completions") {
                let _ = completions.print_long_help();
            }
        }
    }
}

/// `capture`: its instructions exported or its steps listed, without
/// opening a named meter, or the guided run itself.
fn run_capture(
    args: CaptureArgs,
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = &args.export_instructions {
        // Like --list-steps: the run's steps, without opening the meter when it
        // is named.
        device_for_listing(selection, opts)
            .and_then(|device| capture::export_instructions(device, args.plan.as_deref(), path))
    } else if args.list_steps {
        // Device-scoped: the steps come from the selected device's protocol, so
        // what's listed is what `--steps` will match. With nothing selected,
        // that means asking the cable first.
        device_for_listing(selection, opts).map(|device| {
            capture::list_steps(device, args.format);
        })
    } else {
        open_recording_with_help(selection, opts).and_then(|(dmm, recorder, device, detected)| {
            // The capture's own bridge first: `auto` alone could open another
            // cable, or USB before a meter's radio.
            let reopen: capture::Reopen<'_> =
                Box::new(move |bridge| dmm_lib::open_transport(&[bridge], opts));
            capture::cmd_capture(args, dmm, recorder, device, detected, reopen)
        })
    }
}

/// Report a failed command and end the run: the error, then whatever
/// help answers it.
fn report_failure(
    l: &Localizer,
    e: &(dyn std::error::Error + 'static),
    selection: Selection,
    adapter: Option<&str>,
    no_bluetooth: bool,
) -> ! {
    error!("{e}");
    let msg = l10n::error_text(l, e);
    // The activation instructions belong to one meter, so they are only
    // printed once one is settled on: the one named, or the one detection
    // found before the meter went quiet.
    if meter_went_quiet(e)
        && let Some(device) = opened_device(selection)
    {
        print_no_response_help(device);
    } else {
        eprintln!("{} {msg}", error_label());
        if let Some(address) = unreachable_adapter(e, adapter) {
            print_setup_sections(LinksSearched::BluetoothAt(address));
        } else if let Some(device) = unreachable_bluetooth_only(e, selection) {
            print_setup_sections(LinksSearched::BluetoothOnly {
                model: &open::prose_name(open::meter_name(device)),
                activation: device.activation_instructions,
            });
        } else if bluetooth_switched_off(e) {
            eprintln!(
                "{}",
                style(l.help().cli_bluetooth_off_hint(no_bluetooth)).yellow()
            );
        }
    }
    std::process::exit(1);
}

/// Whether `error` is the meter going quiet, which its activation steps
/// answer. Matched on the variant rather than the message: hidapi names its
/// `hid_read_timeout` call in the error a pulled cable raises, and some
/// errors quote the user's own arguments back. Not `ErrorKind::Timeout`
/// either: detection finding no meter is that kind too, and prints help of
/// its own.
fn meter_went_quiet(error: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        error.downcast_ref::<dmm_lib::error::Error>(),
        Some(dmm_lib::error::Error::Timeout)
    )
}

/// The Bluetooth address `--adapter` named, when `error` is the stack failing
/// to reach it — most often a paired adapter that is asleep, which wants the
/// same steps as an address nothing answered.
fn unreachable_adapter<'a>(
    error: &(dyn std::error::Error + 'static),
    adapter: Option<&'a str>,
) -> Option<&'a str> {
    let stack_failed = matches!(
        error.downcast_ref::<dmm_lib::error::Error>(),
        Some(dmm_lib::error::Error::Bluetooth(_))
    );
    adapter.filter(|a| stack_failed && dmm_lib::is_bluetooth_selector(a))
}

/// The meter with the radio built in that `selection` named, when `error` is
/// the stack failing to reach it: the steps that switch its radio on follow
/// the stack's own words.
fn unreachable_bluetooth_only(
    error: &(dyn std::error::Error + 'static),
    selection: Selection,
) -> Option<&'static SelectableDevice> {
    let stack_failed = matches!(
        error.downcast_ref::<dmm_lib::error::Error>(),
        Some(dmm_lib::error::Error::Bluetooth(_))
    );
    match selection {
        Selection::Device(device) if stack_failed && device.bluetooth_only() => Some(device),
        _ => None,
    }
}

/// Whether `error` is a meter with the radio built in that was not looked
/// for because the search was switched off: the switch is ours to name.
fn bluetooth_switched_off(error: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        error.downcast_ref::<dmm_lib::error::Error>(),
        Some(dmm_lib::error::Error::BluetoothOnly {
            miss: dmm_lib::error::BluetoothOnlyMiss::SwitchedOff,
            ..
        })
    )
}

/// Whether this run may look for a Bluetooth adapter.
///
/// The saved setting decides, and `--no-bluetooth` overrides it for one run —
/// the file is never written back: the GUI owns it.
fn bluetooth_probing(no_bluetooth: bool, saved: Option<&dmm_shared::SharedSettings>) -> bool {
    !no_bluetooth && saved.is_none_or(|s| s.bluetooth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::*;

    /// Naming no meter, on the command line or in the settings file, means
    /// detection rather than a model nobody chose.
    #[test]
    fn no_flag_and_no_setting_detects_the_meter() {
        let id = dmm_shared::resolve_device_family(None, None, registry::AUTO_DEVICE_ID);
        assert!(matches!(
            registry::resolve_selection(&id),
            Some(Selection::Auto)
        ));
        assert!(requires_hardware(selection("auto")));
    }

    /// A saved or flagged family still pins one, and skips detection.
    #[test]
    fn a_named_family_still_wins() {
        let saved = dmm_shared::SharedSettings {
            device_family: "ut8803".to_string(),
            ..Default::default()
        };
        let id = dmm_shared::resolve_device_family(None, Some(&saved), registry::AUTO_DEVICE_ID);
        let Some(Selection::Device(device)) = registry::resolve_selection(&id) else {
            panic!("a saved family must resolve to that device");
        };
        assert_eq!(device.id, "ut8803");

        let id = dmm_shared::resolve_device_family(
            Some("ut61b+"),
            Some(&saved),
            registry::AUTO_DEVICE_ID,
        );
        assert_eq!(selection_id(selection(&id)), "ut61b+");
    }

    /// `--no-bluetooth` is one run's answer; the setting is every run's. With
    /// neither, the radio is searched — the file a v0.7 install left behind
    /// has no such field, and that version always searched.
    #[test]
    fn the_flag_overrides_the_saved_bluetooth_setting() {
        let saved = |bluetooth| dmm_shared::SharedSettings {
            bluetooth,
            ..Default::default()
        };
        assert!(bluetooth_probing(false, None));
        assert!(bluetooth_probing(false, Some(&saved(true))));
        assert!(!bluetooth_probing(true, Some(&saved(true))));
        assert!(!bluetooth_probing(false, Some(&saved(false))));
        assert!(!bluetooth_probing(true, Some(&saved(false))));
    }

    /// Only a search switched off gets the CLI's switch after the error; a
    /// meter out of range or a build without the radio has nothing to tick.
    #[test]
    fn only_a_switched_off_search_gets_the_switch() {
        use dmm_lib::error::{BluetoothOnlyMiss, Error};
        let err = |miss| -> Box<dyn std::error::Error> {
            Box::new(Error::BluetoothOnly {
                model: "UT60BT",
                activation: "",
                miss,
            })
        };
        assert!(bluetooth_switched_off(&*err(
            BluetoothOnlyMiss::SwitchedOff
        )));
        for miss in [
            BluetoothOnlyMiss::NotInRange,
            BluetoothOnlyMiss::NotBuilt,
            BluetoothOnlyMiss::UsbAdapter,
        ] {
            assert!(!bluetooth_switched_off(&*err(miss)), "{miss:?}");
        }
    }

    /// Only a meter that went quiet gets its activation steps: a pulled cable
    /// and an argument that happens to say "timeout" keep the plain error.
    #[test]
    fn only_a_quiet_meter_gets_the_activation_help() {
        use dmm_lib::error::Error;
        let boxed = |e: Error| -> Box<dyn std::error::Error> { Box::new(e) };
        assert!(meter_went_quiet(&*boxed(Error::Timeout)));
        // What hidapi's hidraw backend reports when the cable is pulled
        // mid-read.
        let pulled = Error::Hid(hidapi::HidError::HidApiError {
            message: "hid_read_timeout: unexpected poll error (device disconnected)".to_string(),
        });
        assert!(!meter_went_quiet(&*boxed(pulled)));
        let echoed: Box<dyn std::error::Error> = "unknown range: timeout".into();
        assert!(!meter_went_quiet(&*echoed));
    }

    /// A named adapter the stack could not reach is explained by its link;
    /// any other failure, or an adapter that is not a Bluetooth address, is
    /// left as the bare error.
    #[test]
    fn a_named_adapter_that_would_not_connect_gets_the_bluetooth_steps() {
        use dmm_lib::error::{BluetoothIssue, Error};
        let address = "12:34:56:78:9A:BC";
        let stack = Error::Bluetooth(BluetoothIssue::Stack("Timed out after 10s".to_string()));
        let expected = dmm_lib::is_bluetooth_selector(address).then_some(address);
        assert_eq!(unreachable_adapter(&stack, Some(address)), expected);
        assert_eq!(unreachable_adapter(&stack, None), None);
        assert_eq!(unreachable_adapter(&stack, Some("00C5B27A")), None);
        assert_eq!(unreachable_adapter(&Error::LinkLost, Some(address)), None);
    }

    /// A meter with the radio built in that the stack could not reach gets
    /// its own steps; a meter behind an adapter keeps the bare error, as it
    /// always has.
    #[test]
    fn an_unreachable_bluetooth_only_meter_gets_its_own_steps() {
        use dmm_lib::error::{BluetoothIssue, Error};
        let stack = Error::Bluetooth(BluetoothIssue::PoweredOff);
        let device = |id| Selection::Device(registry::find_device(id).expect("registry entry"));
        let found = unreachable_bluetooth_only(&stack, device("ut60bt")).map(|d| d.id);
        assert_eq!(found, Some("ut60bt"));
        assert!(unreachable_bluetooth_only(&stack, device("ut61eplus")).is_none());
        assert!(unreachable_bluetooth_only(&stack, Selection::Auto).is_none());
        assert!(unreachable_bluetooth_only(&Error::LinkLost, device("ut60bt")).is_none());
    }
}
