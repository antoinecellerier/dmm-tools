//! Opening the meter a command runs against — the one named, or the one
//! detection finds — and the setup help a failed open prints.

use crate::capture::recording;
use console::style;
use dmm_lib::protocol::registry::{self, SelectableDevice, Selection};
use dmm_shared::help::{ConnectedAdapters, LinksSearched};
use std::sync::OnceLock;

/// Print a "no response" warning with device-specific activation instructions.
pub(crate) fn print_no_response_help(device: &SelectableDevice) {
    eprintln!(
        "{} No response from meter. Check that --device {} is correct \
         and that data transmission is enabled.",
        style("Warning:").yellow(),
        device.id,
    );
    eprintln!("{}", style(device.activation_instructions).dim());
}

/// Print the setup help for the links that were searched, one section each.
///
/// A section opens with the link in bold, so the two are told apart at a
/// glance; its steps are indented under it, and the ones that are commands to
/// run or URLs to open are dimmed to keep the prose that explains them in the
/// foreground.
pub(crate) fn print_setup_sections(links: LinksSearched<'_>) {
    for section in links.sections() {
        eprintln!();
        eprintln!("{}: {}", style(section.link).bold(), section.check);
        for step in section.steps {
            let line = format!("  {step}");
            if step.starts_with(' ') {
                eprintln!("{}", style(line).dim());
            } else {
                eprintln!("{line}");
            }
        }
    }
}

/// Print the whole "nothing found" help: what was looked for, then what to
/// try on each link it was looked for on.
fn print_not_found_help(links: LinksSearched<'_>) {
    eprintln!(
        "{}",
        style(format!("{}.", links.not_found_title()))
            .yellow()
            .bold()
    );
    print_setup_sections(links);
}

/// The meter handle every command works through, with the transport picked
/// at runtime.
pub(crate) type BoxedDmm = dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>;

/// What detection settled on, once a [`Selection::Auto`] open has run.
///
/// The two sites that need it afterwards — the `info` listing and the timeout
/// help `main` prints once a command has already returned — are past the point
/// where the opener could hand it to them, and a run only ever opens one meter.
pub(crate) static AUTO_DETECTED: OnceLock<dmm_lib::detect::Detected> = OnceLock::new();

/// How the help for a silent cable ends: the way to bypass detection, and
/// the ask that turns an unrecognised meter into a registry fix.
const NOT_IDENTIFIED_SELF_HELP: &str = "\nIf the meter is on and transmitting but still not recognised, name it:\n  \
     dmm-cli --device <id> ...      (dmm-cli --help lists the ids)\n\
     and please report it with RUST_LOG=dmm_lib=debug output at\n  \
     https://github.com/antoinecellerier/dmm-tools/issues";

/// How both `--adapter` hints spell the value: whatever `dmm-cli list`
/// printed for that device — a USB cable's serial number or HID path, or a
/// Bluetooth adapter's address.
pub(crate) const ADAPTER_SELECTOR: &str = "--adapter <serial-path-or-address>";

/// Whether this selection means opening a USB cable. Auto does: there is
/// nothing to detect without one.
pub(crate) fn requires_hardware(selection: Selection) -> bool {
    match selection {
        Selection::Auto => true,
        Selection::Device(device) => device.requires_hardware,
    }
}

/// What the user would pass to `--device` to make this selection again.
pub(crate) fn selection_id(selection: Selection) -> &'static str {
    match selection {
        Selection::Auto => registry::AUTO_DEVICE_ID,
        Selection::Device(device) => device.id,
    }
}

/// The entry a command ran against: the one named, or the one detection found.
///
/// `None` only when Auto never got as far as identifying a meter.
pub(crate) fn opened_device(selection: Selection) -> Option<&'static SelectableDevice> {
    match selection {
        Selection::Auto => AUTO_DETECTED.get().map(|d| d.device),
        Selection::Device(device) => Some(device),
    }
}

/// Record what detection found, and say so.
///
/// The meter was picked for the user, so the name it was picked by belongs on
/// screen — dim and on stderr, like every other notice, so a redirected CSV or
/// JSON stream stays machine-readable.
fn note_detected(detected: dmm_lib::detect::Detected) -> &'static dmm_lib::detect::Detected {
    let detected = AUTO_DETECTED.get_or_init(|| detected);
    let device = detected.device;
    // A name the registry doesn't carry still identifies the family, and the
    // tables in use are then someone else's — say whose, and how to override.
    // Either way the line ends with the exact option that skips the probe next
    // time, so the user never has to look the id up.
    let note = match &detected.reported_name {
        Some(name) if name != device.display_name => format!(
            " (the meter reports {name:?}; using {} tables \u{2014} pass --device {} to keep them, \
             or another id to override)",
            device.display_name, device.id
        ),
        _ => format!(
            " (pass --device {} or pick it in dmm-gui to skip detection next time)",
            device.id
        ),
    };
    eprintln!(
        "{}",
        style(format!("Detected {}{note}", device.display_name)).dim()
    );
    detected
}

/// Open the meter with helpful error messages for common failures, and say
/// which entry answered — the one named, or the one detection found.
pub(crate) fn open_with_help(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(BoxedDmm, &'static SelectableDevice), Box<dyn std::error::Error>> {
    let (dmm, device) = match selection {
        Selection::Auto => {
            let (dmm, detected) =
                dmm_lib::open_auto(opts).map_err(|e| open_error_help(selection, e))?;
            (dmm, note_detected(detected).device)
        }
        Selection::Device(device) => (
            dmm_lib::open_device_by_id_auto(device.id, opts)
                .map_err(|e| open_error_help(selection, e))?,
            device,
        ),
    };
    warn_if_experimental(device, dmm.profile());
    Ok((dmm, device))
}

/// Open the meter with every wire byte recorded, the init handshake included,
/// for `capture` to put in its report.
pub(crate) fn open_recording_with_help(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<
    (
        BoxedDmm,
        recording::SharedRecorder,
        &'static SelectableDevice,
    ),
    Box<dyn std::error::Error>,
> {
    type Boxed = Box<dyn dmm_lib::transport::Transport>;
    let (dmm, recorder, device) = match selection {
        // Wrap first, then detect: the probe and the meter's answer to it are
        // the first bytes on the wire, and a report that starts after them
        // hides how the meter was picked.
        Selection::Auto => {
            let (transport, bridge) =
                dmm_lib::open_transport(&[], opts).map_err(|e| open_error_help(selection, e))?;
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            let transport = Box::new(transport) as Boxed;
            let detected = dmm_lib::detect::detect_device(&*transport, bridge)
                .map_err(|e| open_error_help(selection, e))?;
            let detected = note_detected(detected);
            let dmm = dmm_lib::Dmm::from_detected(transport, detected)
                .map_err(|e| open_error_help(selection, e))?;
            (dmm, recorder, detected.device)
        }
        Selection::Device(device) => {
            // The mock has no USB link to open, and none to record either — it
            // goes through the same recorder so `capture` has one code path.
            let transport: Boxed = if device.requires_hardware {
                dmm_lib::open_device_transport(device, opts)
                    .map_err(|e| open_error_help(selection, e))?
            } else {
                Box::new(dmm_lib::transport::NullTransport)
            };
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            let dmm = dmm_lib::Dmm::new(Box::new(transport) as Boxed, (device.new_protocol)())
                .map_err(|e| open_error_help(selection, e))?;
            (dmm, recorder, device)
        }
    };
    warn_if_experimental(device, dmm.profile());
    Ok((dmm, recorder, device))
}

/// Identify the meter and hand the cable straight back, for the listings that
/// need a registry entry but nothing from the meter itself.
fn detect_only(
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<&'static SelectableDevice, Box<dyn std::error::Error>> {
    let (transport, bridge) =
        dmm_lib::open_transport(&[], opts).map_err(|e| open_error_help(Selection::Auto, e))?;
    let detected = dmm_lib::detect::detect_device(&*transport, bridge)
        .map_err(|e| open_error_help(Selection::Auto, e))?;
    Ok(note_detected(detected).device)
}

/// The entry a listing describes. Nothing is read from the meter, but with
/// nothing selected there is still a meter to identify — the alternative is
/// listing another model's commands or capture steps.
pub(crate) fn device_for_listing(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<&'static SelectableDevice, Box<dyn std::error::Error>> {
    match selection {
        Selection::Auto => detect_only(opts),
        Selection::Device(device) => Ok(device),
    }
}

/// Tell the user an unverified protocol is in use and how to help fix it.
fn warn_if_experimental(
    device: &'static SelectableDevice,
    profile: &dmm_lib::protocol::DeviceProfile,
) {
    if profile.stability.is_verified() {
        return;
    }
    eprintln!(
        "{}",
        style(format!(
            "WARNING: {}",
            dmm_shared::help::experimental_warning(profile.model_name, profile.stability)
        ))
        .yellow()
        .bold()
    );
    eprintln!(
        "{}",
        style("Run 'capture' to generate a report for validation:").yellow()
    );
    eprintln!(
        "{}",
        style(format!("  dmm-cli --device {} capture", device.id)).yellow()
    );
    eprintln!(
        "{}",
        style(format!("Report feedback: {}", profile.feedback_url())).yellow()
    );
}

/// The paragraph that closes the "nothing found" help for a meter short of
/// verified. Nothing is known about the meter when it was never named and the
/// cable it would have been identified through never opened.
fn print_experimental_note(selection: Selection) {
    let Selection::Device(device) = selection else {
        return;
    };
    let proto = (device.new_protocol)();
    let profile = proto.profile();
    if !profile.stability.is_verified() {
        // Its own paragraph: the sections above end in a step, and
        // this is about the meter rather than the link.
        eprintln!();
        eprintln!(
            "{}",
            style(format!(
                "{} Report feedback: {}",
                dmm_shared::help::experimental_warning(profile.model_name, profile.stability),
                profile.feedback_url()
            ))
            .yellow()
        );
    }
}

/// Print setup help for the failures a user can act on, and return the error
/// to report.
fn open_error_help(
    selection: Selection,
    error: dmm_lib::error::Error,
) -> Box<dyn std::error::Error> {
    match error {
        dmm_lib::error::Error::NoTransportFound {
            bluetooth_searched, ..
        } => {
            print_not_found_help(LinksSearched::from_usb_failure(bluetooth_searched));
            print_experimental_note(selection);
            "device not found".into()
        }
        // A meter with the radio built in, out of range or asleep: its own
        // steps, not a cable's.
        dmm_lib::error::Error::BluetoothOnly {
            model,
            activation,
            miss: dmm_lib::error::BluetoothOnlyMiss::NotInRange,
        } => {
            print_not_found_help(LinksSearched::BluetoothOnly { model, activation });
            print_experimental_note(selection);
            "device not found".into()
        }
        // The cable is there and nothing on it spoke. Every meter it could
        // carry has something the user has to switch on, so list them.
        dmm_lib::error::Error::DeviceNotIdentified {
            bridge,
            built_in_radio,
        } => {
            eprintln!(
                "{}",
                style(format!(
                    "No meter answered over the {}.",
                    dmm_lib::transport::bridge_link_name(bridge, built_in_radio)
                ))
                .yellow()
                .bold()
            );
            for (instructions, names) in
                dmm_shared::help::activation_groups(&dmm_lib::devices_on_bridge(bridge))
            {
                eprintln!("\n{}", style(names.join(", ")).yellow());
                for line in instructions.lines() {
                    eprintln!("{}", style(format!("  {line}")).dim());
                }
            }
            for line in NOT_IDENTIFIED_SELF_HELP.lines() {
                eprintln!("{}", style(line).yellow());
            }
            "device not identified".into()
        }
        // An address nothing answered: the bus listing would be beside the
        // point, and the only thing that says which addresses are live is a
        // scan, which the open has just run.
        dmm_lib::error::Error::AdapterNotFound(ref selector)
            if dmm_lib::is_bluetooth_selector(selector) =>
        {
            print_not_found_help(LinksSearched::BluetoothAt(selector));
            "adapter not found".into()
        }
        dmm_lib::error::Error::AdapterNotFound(ref detail) => {
            eprintln!(
                "{} adapter not found: {detail}",
                style("Error:").red().bold()
            );
            // Only a real list is worth setting apart and worth a hint —
            // "nothing is connected" and "couldn't look" are complete on
            // their own.
            let adapters = dmm_shared::help::connected_adapters();
            let listed = matches!(adapters, ConnectedAdapters::Listed(_));
            if listed {
                eprintln!();
            }
            for line in adapters.lines() {
                eprintln!("{}", style(line).yellow());
            }
            if listed {
                eprintln!(
                    "\n{}",
                    style(format!("Use {ADAPTER_SELECTOR} to select one.")).dim()
                );
            }
            if let Some(hint) = dmm_shared::help::colonless_address_hint(detail) {
                eprintln!("\n{}", style(hint).yellow());
            }
            "adapter not found".into()
        }
        e => e.into(),
    }
}

/// Open the simulated device `selection` names on `clock`, the UT61E+ mock
/// pinned to `mock_mode` when one was given.
///
/// Shared by every subcommand that opens a device needing no hardware, so an
/// unknown mode name is rejected with the same message (and the same list of
/// valid names) wherever it is passed. Only `read` has clock flags; the
/// others pass [`dmm_lib::Clock::real`].
pub(crate) fn open_mock_device(
    selection: Selection,
    mock_mode: Option<String>,
    clock: dmm_lib::Clock,
) -> Result<dmm_lib::Dmm<dmm_lib::transport::NullTransport>, Box<dyn std::error::Error>> {
    let mode = match mock_mode {
        Some(mode_str) => Some(
            mode_str
                .parse::<dmm_lib::mock::MockMode>()
                .map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?,
        ),
        None => None,
    };
    // Auto is a cable to open and never lands here.
    let Selection::Device(device) = selection else {
        return Err(format!("--device {} is not simulated", selection_id(selection)).into());
    };
    Ok(dmm_lib::mock::open_simulated(device, mode, clock)?)
}
