mod capture;
mod choice;
mod cli;
mod cmd;
mod format;
mod open;
mod output;
#[cfg(test)]
mod test_fixtures;

use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, Cmd, build_after_long_help, build_device_help};
use cmd::command::cmd_command;
use cmd::debug::cmd_debug;
use cmd::info::cmd_info;
use cmd::list::cmd_list;
use cmd::read::{REPLAY_NAMES_ITS_DEVICE, cmd_read, refuse_replay_format, resolve_output};
use cmd::settings::{cmd_get, cmd_set};
use console::style;
use dmm_lib::protocol::registry::{self, SelectableDevice, Selection};
use dmm_shared::help::LinksSearched;
use log::error;
use open::{
    device_for_listing, open_recording_with_help, opened_device, print_no_response_help,
    print_setup_sections, requires_hardware, selection_id,
};

fn main() {
    dmm_shared::logging::init();

    // Build CLI with registry-generated --device long_help and a dynamic
    // after_long_help that resolves the actual per-platform settings path.
    let mut cmd = Cli::command();
    let device_help = build_device_help();
    cmd = cmd.mut_arg("device", |a| a.long_help(device_help));
    cmd = cmd.after_long_help(build_after_long_help());
    let cli =
        Cli::from_arg_matches_mut(&mut cmd.get_matches()).unwrap_or_else(|e: clap::Error| e.exit());

    // Whether the meter was named on the command line, as opposed to coming
    // from the settings file or from detection. Only `read --replay` asks.
    let device_named = cli.device.is_some();

    let settings = dmm_shared::SharedSettings::load_if_exists();

    // Nothing named a meter, so none is assumed: detection is the fallback,
    // and `--device` or the settings file pins a model when the user wants
    // one.
    let device_id = dmm_shared::resolve_device_family(
        cli.device.as_deref(),
        settings.as_ref(),
        registry::AUTO_DEVICE_ID,
    );
    let selection = match registry::resolve_selection(&device_id) {
        Some(s) => s,
        None => {
            eprintln!(
                "{} unknown device: {}",
                style("Error:").red().bold(),
                device_id,
            );
            std::process::exit(1);
        }
    };

    let bluetooth = bluetooth_probing(cli.no_bluetooth, settings.as_ref());
    let opts = dmm_lib::OpenOptions {
        adapter: cli.adapter.as_deref(),
        bluetooth,
    };

    // Device-independent commands — handle before mock/real split
    let result = match cli.command {
        Cmd::List => cmd_list(bluetooth),
        Cmd::Completions { shell } => {
            match shell {
                Some(shell) => {
                    clap_complete::generate(
                        shell,
                        &mut Cli::command(),
                        "dmm-cli",
                        &mut std::io::stdout(),
                    );
                }
                None => {
                    let _ = Cli::command()
                        .find_subcommand_mut("completions")
                        .unwrap()
                        .print_long_help();
                }
            }
            Ok(())
        }

        // The mock is a registry device with nothing to open, so only the
        // commands that have no meaning without hardware branch on it here —
        // `read`, `command`, `get` and `set` pick the mock transport
        // themselves and take the same arm as every other device. Auto never
        // lands here: detecting a meter means opening a cable.
        Cmd::Info | Cmd::Debug { .. } if !requires_hardware(selection) => {
            eprintln!(
                "{} This command requires real hardware (not supported with --device {}).",
                style("Error:").red().bold(),
                selection_id(selection),
            );
            std::process::exit(1);
        }

        Cmd::Info => cmd_info(selection, opts),
        Cmd::Read {
            interval_ms,
            format,
            output,
            count,
            integrate,
            transform,
            mock_mode,
            replay,
            mock_clock_scale,
            mock_clock_preseed,
        } => {
            // `from_flags` names the offending value, not the flag it came
            // from, so that both binaries can reuse the sentence.
            let clock = match dmm_lib::Clock::from_flags(mock_clock_scale, mock_clock_preseed) {
                Ok(clock) => clock,
                Err(msg) => {
                    eprintln!(
                        "{} --mock-clock-scale/--mock-clock-preseed: {msg}",
                        style("Error:").red().bold(),
                    );
                    std::process::exit(1);
                }
            };
            // A replay file names its own meter, so a `--device` alongside it
            // would either be ignored or contradict the file. Only a typed
            // flag is refused: the settings file names a meter for every run,
            // and a replay must not need it edited.
            if replay.is_some() && device_named {
                eprintln!(
                    "{} {}",
                    style("Error:").red().bold(),
                    REPLAY_NAMES_ITS_DEVICE,
                );
                std::process::exit(1);
            }
            // Settled before anything opens, so a run that cannot write what
            // it was asked for fails with no meter attached.
            let (format, destination, note) = resolve_output(&format, output);
            match refuse_replay_format(format, selection, replay.is_some(), &transform, integrate) {
                Some(message) => Err(message.into()),
                None => {
                    // After the refusals: a note about where output goes reads
                    // as a run that started, and this one may not.
                    if let Some(note) = note {
                        eprintln!("{} {note}", style("Note:").yellow());
                    }
                    cmd_read(
                        selection,
                        opts,
                        interval_ms,
                        format,
                        destination,
                        count,
                        integrate,
                        &transform.to_transform(),
                        mock_mode,
                        replay,
                        clock,
                    )
                }
            }
        }
        Cmd::Command { action } => cmd_command(selection, opts, action),
        Cmd::Get {
            setting,
            format,
            mock_mode,
        } => cmd_get(selection, opts, setting, format, mock_mode),
        Cmd::Set {
            setting,
            choice,
            mock_mode,
        } => cmd_set(selection, opts, setting, choice, mock_mode),
        Cmd::Debug { count, interval_ms } => cmd_debug(selection, opts, count, interval_ms),
        Cmd::Capture {
            output,
            steps,
            unverified,
            plan,
            sniff,
            no_drive,
            settle,
            list_steps,
            format,
        } => {
            if list_steps {
                // Device-scoped: the steps come from the selected device's
                // protocol, so what's listed is what `--steps` will match.
                // With nothing selected, that means asking the cable first.
                device_for_listing(selection, opts).map(|device| {
                    capture::list_steps(device, format);
                })
            } else {
                open_recording_with_help(selection, opts).and_then(|(dmm, recorder, device)| {
                    capture::cmd_capture(
                        output,
                        steps,
                        unverified,
                        sniff,
                        no_drive,
                        std::time::Duration::from_millis(settle),
                        plan,
                        dmm,
                        recorder,
                        device,
                    )
                })
            }
        }
    };

    if let Err(e) = result {
        error!("{e}");
        let msg = e.to_string();
        // The activation instructions belong to one meter, so they are only
        // printed once one is settled on: the one named, or the one detection
        // found before the meter went quiet.
        if msg.contains("timeout")
            && let Some(device) = opened_device(selection)
        {
            print_no_response_help(device);
        } else {
            eprintln!("{} {msg}", style("Error:").red().bold());
            if let Some(address) = unreachable_adapter(&*e, cli.adapter.as_deref()) {
                print_setup_sections(LinksSearched::BluetoothAt(address));
            } else if let Some(device) = unreachable_bluetooth_only(&*e, selection) {
                print_setup_sections(LinksSearched::BluetoothOnly {
                    model: device.display_name,
                    activation: device.activation_instructions,
                });
            } else if bluetooth_switched_off(&*e) {
                eprintln!(
                    "{}",
                    style(dmm_shared::help::cli_bluetooth_off_hint(cli.no_bluetooth)).yellow()
                );
            }
        }
        std::process::exit(1);
    }
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

    /// A named adapter the stack could not reach is explained by its link;
    /// any other failure, or an adapter that is not a Bluetooth address, is
    /// left as the bare error.
    #[test]
    fn a_named_adapter_that_would_not_connect_gets_the_bluetooth_steps() {
        use dmm_lib::error::Error;
        let address = "12:34:56:78:9A:BC";
        let stack = Error::Bluetooth("Timed out after 10s".to_string());
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
        use dmm_lib::error::Error;
        let stack = Error::Bluetooth("Bluetooth is turned off on this computer".to_string());
        let device = |id| Selection::Device(registry::find_device(id).expect("registry entry"));
        let found = unreachable_bluetooth_only(&stack, device("ut60bt")).map(|d| d.id);
        assert_eq!(found, Some("ut60bt"));
        assert!(unreachable_bluetooth_only(&stack, device("ut61eplus")).is_none());
        assert!(unreachable_bluetooth_only(&stack, Selection::Auto).is_none());
        assert!(unreachable_bluetooth_only(&Error::LinkLost, device("ut60bt")).is_none());
    }
}
