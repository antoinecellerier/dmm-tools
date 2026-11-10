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
use dmm_shared::help::{Diagnosis, LinksSearched};
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
        stop: None,
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
        report_failure(l, &e, selection, cli.adapter.as_deref(), cli.no_bluetooth);
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
) -> Result<(), crate::l10n::Failure> {
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
            capture::cmd_capture(args, dmm, recorder, device, detected.as_ref(), reopen)
        })
    }
}

/// Report a failed command and end the run: the error, then whatever
/// help answers it.
fn report_failure(
    l: &Localizer,
    e: &l10n::Failure,
    selection: Selection,
    adapter: Option<&str>,
    no_bluetooth: bool,
) -> ! {
    error!("{e}");
    let extra = extra_help(e, selection, adapter);
    if let Extra::NoResponse(device) = extra {
        print_no_response_help(device);
        std::process::exit(1);
    }
    let msg = match extra {
        // By the name it was picked by, which the error doesn't carry.
        Extra::NotSearched { model, miss } => l.bluetooth_only(model, miss),
        _ => e.text(l),
    };
    eprintln!("{} {msg}", error_label());
    match extra {
        Extra::BluetoothAt(address) => {
            print_setup_sections(LinksSearched::BluetoothAt(&address));
        }
        Extra::BluetoothOnly { model, activation } => {
            print_setup_sections(LinksSearched::BluetoothOnly {
                model: &open::prose_name(model),
                activation,
            });
        }
        // The switch is ours to name.
        Extra::NotSearched {
            miss: dmm_lib::error::BluetoothOnlyMiss::SwitchedOff,
            ..
        } => eprintln!(
            "{}",
            style(l.help().cli_bluetooth_off_hint(no_bluetooth)).yellow()
        ),
        Extra::WrongPick => {
            if let Some(hint) = e.lib().and_then(|lib| open::wrong_pick_hint(l, lib)) {
                eprintln!("{}", style(hint).yellow());
            }
        }
        Extra::NotSearched { .. } | Extra::NoResponse(_) | Extra::None => {}
    }
    std::process::exit(1);
}

/// What [`report_failure`] prints besides the error, or in its place.
#[derive(Debug, PartialEq)]
enum Extra {
    /// The meter went quiet: its activation steps, in place of the error.
    NoResponse(&'static SelectableDevice),
    /// The stack could not reach the Bluetooth adapter at this address.
    BluetoothAt(String),
    /// The stack could not search for this meter with the radio built in.
    BluetoothOnly {
        model: &'static str,
        activation: &'static str,
    },
    /// Data that wouldn't read, which a wrong pick explains: which meter to
    /// pick instead, or the line naming the pick ([`open::wrong_pick_hint`]).
    WrongPick,
    /// A meter with the radio built in, not looked for: the name it was
    /// picked by, and why.
    NotSearched {
        model: &'static str,
        miss: dmm_lib::error::BluetoothOnlyMiss,
    },
    None,
}

/// The help `error` gets from a run on `selection`, given `adapter`.
///
/// The activation steps belong to one meter, so they are only printed once
/// one is settled on: the one named, or the one detection found before the
/// meter went quiet. A failure the open's own help answered never comes
/// here: it arrives as [`open::OpenFailed`].
fn extra_help(error: &l10n::Failure, selection: Selection, adapter: Option<&str>) -> Extra {
    let Some(error) = error.lib() else {
        return Extra::None;
    };
    match Diagnosis::of(error, adapter, open::picked(selection)) {
        Diagnosis::NoResponse => opened_device(selection).map_or(Extra::None, Extra::NoResponse),
        Diagnosis::AdapterUnreachable { address } => Extra::BluetoothAt(address),
        Diagnosis::BluetoothOnlyUnreachable { model, activation } => {
            Extra::BluetoothOnly { model, activation }
        }
        Diagnosis::BluetoothNotSearched { model, miss } => Extra::NotSearched { model, miss },
        Diagnosis::OtherMeter(_) | Diagnosis::BadData => Extra::WrongPick,
        _ => Extra::None,
    }
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

    /// A meter with the radio built in that was not looked for is named as
    /// this run calls it, whatever the error names; only the switch decides
    /// whether the CLI's own hint follows ([`report_failure`]). Out of range
    /// is the open's own help, and never reaches here.
    #[test]
    fn a_radio_meter_not_looked_for_is_named_as_picked() {
        use dmm_lib::error::{BluetoothOnlyMiss, Error};
        let ut60bt = selection("ut60bt");
        let Selection::Device(device) = ut60bt else {
            panic!("a named meter");
        };
        let err = |miss| Error::BluetoothOnly {
            model: device.display_name(),
            activation: "",
            miss,
        };
        for miss in [
            BluetoothOnlyMiss::SwitchedOff,
            BluetoothOnlyMiss::NotBuilt,
            BluetoothOnlyMiss::UsbAdapter,
        ] {
            assert_eq!(
                extra_help(&err(miss).into(), ut60bt, None),
                Extra::NotSearched {
                    model: open::meter_name(device),
                    miss
                },
                "{miss:?}"
            );
        }
        assert_eq!(
            extra_help(&err(BluetoothOnlyMiss::NotInRange).into(), ut60bt, None),
            Extra::None
        );
    }

    /// A meter that says it is another model gets what to pass instead,
    /// after the error.
    #[test]
    fn another_meter_gets_what_to_pass() {
        use dmm_lib::error::{Error, WrongMeter};
        let other = Error::WrongMeter {
            reason: WrongMeter::OtherSeries {
                found: "Brymen BM82x",
                opened: "Brymen BM86x",
                id: "bm82x",
            },
            raw: Vec::new(),
        };
        assert_eq!(
            extra_help(&other.into(), selection("bm86x"), None),
            Extra::WrongPick
        );
    }

    /// Data that wouldn't read gets the line naming the pick, which a saved
    /// device makes easy to forget.
    #[test]
    fn unreadable_data_names_the_pick() {
        use dmm_lib::error::Error;
        let mismatch = Error::ChecksumMismatch {
            expected: 1,
            actual: 2,
        };
        assert_eq!(
            extra_help(&mismatch.into(), selection("ut61eplus"), None),
            Extra::WrongPick
        );
    }

    /// Only a meter that went quiet gets its activation steps, and only once
    /// there is a meter to name: under Auto-detect, before detection found
    /// one, the error stands alone (no test here runs a detection, so none
    /// is on record). A pulled cable and an argument that happens to say
    /// "timeout" keep the plain error.
    #[test]
    fn only_a_quiet_meter_gets_the_activation_help() {
        use dmm_lib::error::Error;
        let ut61eplus = selection("ut61eplus");
        let Selection::Device(device) = ut61eplus else {
            panic!("a named meter");
        };
        assert_eq!(
            extra_help(&Error::Timeout.into(), ut61eplus, None),
            Extra::NoResponse(device)
        );
        assert_eq!(
            extra_help(&Error::Timeout.into(), Selection::Auto, None),
            Extra::None
        );
        assert_eq!(
            extra_help(&Error::cable_pulled().into(), ut61eplus, None),
            Extra::None
        );
        let echoed = l10n::Failure::Text("unknown range: timeout".to_string());
        assert_eq!(extra_help(&echoed, ut61eplus, None), Extra::None);
    }

    /// A named adapter the stack could not reach is explained by its link;
    /// any other failure, or an adapter that is not a Bluetooth address, is
    /// left as the bare error.
    #[test]
    fn a_named_adapter_that_would_not_connect_gets_the_bluetooth_steps() {
        use dmm_lib::error::{BluetoothIssue, Error};
        let address = "12:34:56:78:9A:BC";
        let stack = l10n::Failure::from(Error::Bluetooth(BluetoothIssue::Stack(
            "Timed out after 10s".to_string(),
        )));
        let auto = Selection::Auto;
        assert_eq!(
            extra_help(&stack, auto, Some(address)),
            Extra::BluetoothAt(address.to_string())
        );
        assert_eq!(extra_help(&stack, auto, None), Extra::None);
        assert_eq!(extra_help(&stack, auto, Some("00C5B27A")), Extra::None);
        assert_eq!(
            extra_help(&Error::LinkLost.into(), auto, Some(address)),
            Extra::None
        );
    }

    /// A meter with the radio built in that the stack could not reach gets
    /// its own steps; a meter behind an adapter, or none named, keeps the
    /// bare error, as it always has.
    #[test]
    fn an_unreachable_bluetooth_only_meter_gets_its_own_steps() {
        use dmm_lib::error::{BluetoothIssue, Error};
        let stack = l10n::Failure::from(Error::Bluetooth(BluetoothIssue::PoweredOff));
        let ut60bt = selection("ut60bt");
        let Selection::Device(device) = ut60bt else {
            panic!("a named meter");
        };
        assert_eq!(
            extra_help(&stack, ut60bt, None),
            Extra::BluetoothOnly {
                model: open::meter_name(device),
                activation: device.activation_instructions,
            }
        );
        assert_eq!(
            extra_help(&stack, selection("ut61eplus"), None),
            Extra::None
        );
        assert_eq!(extra_help(&stack, Selection::Auto, None), Extra::None);
        assert_eq!(
            extra_help(&Error::LinkLost.into(), selection("ut60bt"), None),
            Extra::None
        );
    }
}
