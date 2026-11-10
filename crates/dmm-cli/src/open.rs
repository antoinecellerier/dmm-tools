//! Opening the meter a command runs against — the one named, or the one
//! detection finds — and the setup help a failed open prints.
//!
//! Everything printed here is in the run's language
//! (`i18n/<locale>/cli/connect.ftl`), on stderr like every notice, and the
//! library text in it — model names, activation steps — goes through the
//! localizer's library lookups. The ids and commands it names stay as typed.

use crate::capture::recording;
use crate::l10n::{error_label, warning_label};
use console::style;
use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry::{self, SelectableDevice, Selection};
use dmm_lib::text::TextKind;
use dmm_shared::help::{Diagnosis, LinksSearched};
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use std::borrow::Cow;
use std::sync::OnceLock;

/// Print a "no response" warning with device-specific activation instructions.
pub(crate) fn print_no_response_help(device: &'static SelectableDevice) {
    let l = crate::l10n::get();
    eprintln!(
        "{} {}",
        warning_label(),
        tr!(
            l,
            "connect-no-response",
            title = l.help().no_response(),
            meter = prose_name(meter_name(device)),
        ),
    );
    print_picked();
    let steps = l.label(TextKind::Activation, device.activation_instructions);
    eprintln!("{}", style(steps).dim());
}

/// Print the setup help for the links that were searched, one section each.
///
/// A section opens with the link in bold, so the two are told apart at a
/// glance; its steps are indented under it, and the ones that are commands to
/// run or URLs to open are dimmed to keep the prose that explains them in the
/// foreground.
pub(crate) fn print_setup_sections(links: LinksSearched<'_>) {
    let help = crate::l10n::get().help();
    for section in help.sections(&links) {
        eprintln!();
        let link = style(&section.link).bold().to_string();
        eprintln!("{}", help.section_label(&link, &section.check));
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
    let l = crate::l10n::get();
    let title = l.help().not_found_title(&links);
    eprintln!(
        "{}",
        style(tr!(l, "connect-not-found", title = title))
            .yellow()
            .bold()
    );
    print_picked();
    print_setup_sections(links);
}

/// The meter handle every command works through, with the transport picked
/// at runtime.
pub(crate) type BoxedDmm = dmm_lib::Dmm<Box<dyn dmm_lib::transport::Transport>>;

/// The entry detection settled on, once a [`Selection::Auto`] open has run.
///
/// For [`opened_device`]: the help a failed open or command prints, past the
/// point where the opener could hand the entry over. A run detects once:
/// [`note_detected`] asserts it, as a second detection would leave this
/// naming the first meter.
static DETECTED_DEVICE: OnceLock<&'static SelectableDevice> = OnceLock::new();

/// Where the meter a run opens was named.
#[derive(Clone, Copy)]
pub(crate) enum PickedBy {
    /// `--device` on the command line.
    Flag,
    /// The settings file's `device_family`, which dmm-gui writes when a
    /// device is picked or Auto-detect finds one.
    Settings,
    /// The `# device:` line of the file being played back.
    Recording,
}

/// Whether `--no-bluetooth` is what switched the radio's search off, as
/// opposed to the saved setting: the way to switch it on differs. Set once,
/// by `main`.
static NO_BLUETOOTH_FLAG: OnceLock<bool> = OnceLock::new();

/// Record whether `--no-bluetooth` was given, for the help a failed open
/// prints.
pub(crate) fn note_no_bluetooth_flag(given: bool) {
    let _ = NO_BLUETOOTH_FLAG.set(given);
}

/// How this run switches the radio's search back on, by what switched it
/// off: the flag, or the saved setting. In `l`'s language.
pub(crate) fn bluetooth_off_hint(l: &Localizer) -> Cow<'_, str> {
    l.help()
        .cli_bluetooth_off_hint(NO_BLUETOOTH_FLAG.get().copied().unwrap_or(false))
}

/// The row the user picked, and where; unset under Auto-detect. Set once,
/// before anything is opened.
static PICKED: OnceLock<(registry::Listing, PickedBy)> = OnceLock::new();

/// Record the pick `device` names, so the run calls the meter by it.
pub(crate) fn note_picked(device: &str, by: PickedBy) {
    if let Some(listing) = registry::resolve_listing(device) {
        let _ = PICKED.set((listing, by));
    }
}

/// The row this run calls `device` by: the user's pick when it opens
/// `device`, else the maker's ([`registry::connected_listing`]).
pub(crate) fn meter_listing(device: &'static SelectableDevice) -> registry::Listing {
    let picked = PICKED.get().map(|&(listing, _)| listing);
    registry::connected_listing(picked, device)
}

/// The line that says which meter the run was told to open, and where it was
/// told: a meter saved by dmm-gui opens with no `--device` in sight, and a
/// wrong one would otherwise only show as a cable or meter that never
/// answers. `None` under Auto-detect. In `l`'s language.
pub(crate) fn picked_line(l: &Localizer) -> Option<String> {
    let &(listing, by) = PICKED.get()?;
    Some(selected_line(l, listing, by))
}

/// [`picked_line`] for `listing`, picked `by`.
fn selected_line(l: &Localizer, listing: registry::Listing, by: PickedBy) -> String {
    let by = match by {
        PickedBy::Flag => "flag",
        PickedBy::Settings => "settings",
        PickedBy::Recording => "recording",
    };
    tr!(
        l,
        "connect-selected",
        by = by,
        meter = l.label(TextKind::Model, listing.display_name()),
        device = listing.id(),
    )
    .into_owned()
}

/// Print the name the meter reported for itself through `print`, or that it
/// reported none — many meters never do, so that is no fault. In `l`'s
/// language, as [`picked_line`].
pub(crate) fn print_reported_name(l: &Localizer, reported: Option<&str>, print: impl Fn(String)) {
    let name = match reported.filter(|n| !n.is_empty()) {
        // The meter's own text, escaped as `note_detected` writes it.
        Some(name) => style(escaped(name)).bold().to_string(),
        None => style(tr!(l, "connect-reported-name-none"))
            .dim()
            .to_string(),
    };
    print(tr!(l, "connect-reported-name", name = name).into_owned());
}

/// The row the user picked, on the command line or in the settings file
/// (not a replay's own); `None` under Auto-detect.
pub(crate) fn picked_row() -> Option<registry::Listing> {
    match PICKED.get() {
        Some(&(_, PickedBy::Recording)) | None => None,
        Some(&(listing, _)) => Some(listing),
    }
}

/// What data that won't read, before any reading came in, says about the
/// pick: the meter to pick, when the meter names one, else the line naming
/// the pick, which a saved device makes easy to forget. `None` for any other
/// failure, under Auto-detect for unreadable data, and for a recording,
/// which names its own meter. In `l`'s language.
pub(crate) fn wrong_pick_hint(l: &Localizer, error: &dmm_lib::error::Error) -> Option<String> {
    pick_hint(l, error, PICKED.get().copied())
}

/// [`wrong_pick_hint`] for the row `pick` names, and where it was picked.
fn pick_hint(
    l: &Localizer,
    error: &dmm_lib::error::Error,
    pick: Option<(registry::Listing, PickedBy)>,
) -> Option<String> {
    let picked = match pick {
        Some((_, PickedBy::Recording)) => return None,
        Some((listing, _)) => Some(listing),
        None => None,
    };
    match Diagnosis::of(error, None, picked) {
        Diagnosis::OtherMeter(choice) => Some(l.help().cli_choose_meter(choice, picked)),
        Diagnosis::BadData => pick.map(|(listing, by)| selected_line(l, listing, by)),
        _ => None,
    }
}

/// Print [`picked_line`], when there is one, under a failed open's headline.
fn print_picked() {
    if let Some(line) = picked_line(crate::l10n::get()) {
        eprintln!("{line}");
    }
}

/// Brand and model, as this run calls `device`.
pub(crate) fn meter_name(device: &'static SelectableDevice) -> &'static str {
    meter_listing(device).display_name()
}

/// [`meter_name`] of the entry `profile` was built for.
pub(crate) fn profile_name(profile: &dmm_lib::protocol::DeviceProfile) -> &'static str {
    registry::find_device(profile.id).map_or_else(|| profile.display_name(), meter_name)
}

/// A meter's name as a sentence shows it: [`meter_name`] and the like stay
/// English for the files they head, and only a simulated meter's name has
/// words to translate ("Mock UT61E+"); brands and models are as they are.
pub(crate) fn prose_name(name: &'static str) -> Cow<'static, str> {
    crate::l10n::get().label(TextKind::Model, name)
}

/// How the help for a silent cable ends: the way to bypass detection, and
/// the ask that turns an unrecognised meter into a registry fix. One
/// paragraph, a line per terminal line, the commands on lines of their own.
fn not_identified_self_help(l: &Localizer) -> Cow<'_, str> {
    tr!(
        l,
        "connect-not-identified-name-it",
        command = "dmm-cli --device <id> ...",
        help = "dmm-cli --help",
        logging = "RUST_LOG=dmm_lib=debug",
        url = format!("{}/issues", dmm_shared::help::PROJECT_URL),
    )
}

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
        Selection::Auto => DETECTED_DEVICE.get().copied(),
        Selection::Device(device) => Some(device),
    }
}

/// Record what detection found, and say so.
///
/// The meter was picked for the user, so the name it was picked by belongs on
/// screen — dim and on stderr, like every other notice, so a redirected CSV or
/// JSON stream stays machine-readable.
fn note_detected(detected: &dmm_lib::detect::Detected) {
    let device = detected.device;
    let first = DETECTED_DEVICE.set(device).is_ok();
    debug_assert!(first, "a run detects its meter once");
    let l = crate::l10n::get();
    let meter = prose_name(device.display_name());
    // A name the registry doesn't carry still identifies the family, and the
    // tables in use are then someone else's — say whose, and how to override.
    // Either way the line ends with the exact option that skips the probe next
    // time, so the user never has to look the id up.
    let line = match &detected.reported_name {
        Some(name) if !device.answers_to(name) => tr!(
            l,
            "connect-detected-reports",
            meter = meter.as_ref(),
            reported = escaped(name),
            device = device.id,
        ),
        _ => tr!(
            l,
            "connect-detected",
            meter = meter.as_ref(),
            device = device.id
        ),
    };
    eprintln!("{}", style(line).dim());
}

/// `text` escaped as `{:?}` writes it, without the quotes, which the
/// message supplies: a name with control characters or quotes in it shows
/// what it holds.
fn escaped(text: &str) -> String {
    let quoted = format!("{text:?}");
    quoted[1..quoted.len() - 1].to_string()
}

/// Open the meter with helpful error messages for common failures, and say
/// which entry answered — the one named, or the one detection found.
pub(crate) fn open_with_help(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<(BoxedDmm, &'static SelectableDevice), crate::l10n::Failure> {
    let (dmm, device) = match selection {
        Selection::Auto => {
            let (dmm, detected) = unless_stopped(dmm_lib::open_auto(opts), opts.stop)
                .map_err(|e| open_error_help(selection, opts, e))?;
            note_detected(&detected);
            (dmm, detected.device)
        }
        Selection::Device(device) => (
            unless_stopped(dmm_lib::open_device(device, opts), opts.stop)
                .map_err(|e| open_error_help(selection, opts, e))?,
            device,
        ),
    };
    warn_if_experimental(device, dmm.profile());
    warn_cables_at_open(selection, opts, dmm.transport());
    Ok((dmm, device))
}

/// Say that other cables were there to open and this run took the first, as
/// dmm-gui does at its first open. Nothing for a meter opened over
/// Bluetooth, or a cable the only one of its kind.
fn warn_cables_at_open(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    transport: &dyn dmm_lib::transport::Transport,
) {
    // A simulated meter, or one over Bluetooth, took no cable.
    if transport.link() != Some(dmm_lib::transport::Link::UsbCable) {
        return;
    }
    // Every cable kind under Auto-detect, as its open looked at them all.
    let device = match selection {
        Selection::Auto => None,
        Selection::Device(device) => Some(device),
    };
    let pin = dmm_shared::reopen::reopen_pin(
        device,
        opts.adapter,
        opts.bluetooth,
        transport.link(),
        transport.bluetooth_selector(),
        transport.cable_serial(),
        dmm_shared::reopen::listed_cables,
    );
    if let Some((several, _)) = dmm_shared::reopen::at_open(pin.as_ref().map(Some).map_err(|s| *s))
    {
        let l = crate::l10n::get();
        eprintln!("{} {}", warning_label(), l.help().cables_at_open(several));
    }
}

/// An open `stop` overtook comes to [`Error::Cancelled`], whatever it came to
/// by itself: a USB open and the detection after it run to their end, and what
/// a stopped run does next is close, with no help or note for anything found.
/// With a flag to raise, a read the signal cut short counts too: the handler
/// raises it on a thread of its own, which may not have run yet.
///
/// [`Error::Cancelled`]: dmm_lib::error::Error::Cancelled
fn unless_stopped<T>(
    opened: dmm_lib::error::Result<T>,
    stop: Option<&std::sync::atomic::AtomicBool>,
) -> dmm_lib::error::Result<T> {
    let Some(stop) = stop else {
        return opened;
    };
    match opened {
        _ if stop.load(std::sync::atomic::Ordering::SeqCst) => {
            Err(dmm_lib::error::Error::Cancelled)
        }
        Err(e) if e.is_interrupted() => Err(dmm_lib::error::Error::Cancelled),
        opened => opened,
    }
}

/// A meter opened for `capture`: the handle, its wire recorder, the entry it
/// runs as, and what detection found when the open ran it.
pub(crate) type RecordingOpen = (
    BoxedDmm,
    recording::SharedRecorder,
    &'static SelectableDevice,
    Option<dmm_lib::detect::Detected>,
);

/// Open the meter with every wire byte recorded, the init handshake included,
/// for `capture` to put in its report.
pub(crate) fn open_recording_with_help(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<RecordingOpen, crate::l10n::Failure> {
    type Boxed = Box<dyn dmm_lib::transport::Transport>;
    let (dmm, recorder, device, detected) = match selection {
        // Wrap first, then detect: the probe and the meter's answer to it are
        // the first bytes on the wire, and a report that starts after them
        // hides how the meter was picked.
        Selection::Auto => {
            let (transport, bridge) = dmm_lib::open_transport(&[], opts)
                .map_err(|e| open_error_help(selection, opts, e))?;
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            let transport = Box::new(transport) as Boxed;
            let detected = dmm_lib::detect::detect_device(&*transport, bridge)
                .map_err(|e| open_error_help(selection, opts, e))?;
            note_detected(&detected);
            let dmm = dmm_lib::Dmm::from_detected(transport, &detected)
                .map_err(|e| open_error_help(selection, opts, e))?;
            (dmm, recorder, detected.device, Some(detected))
        }
        Selection::Device(device) => {
            // The mock has no USB link to open, and none to record either — it
            // goes through the same recorder so `capture` has one code path.
            let transport: Boxed = if device.requires_hardware {
                dmm_lib::open_device_transport(device, opts)
                    .map_err(|e| open_error_help(selection, opts, e))?
            } else {
                Box::new(dmm_lib::transport::NullTransport)
            };
            let (transport, recorder) = recording::RecordingTransport::new(transport);
            let dmm = dmm_lib::Dmm::new(Box::new(transport) as Boxed, (device.new_protocol)())
                .map_err(|e| open_error_help(selection, opts, e))?;
            (dmm, recorder, device, None)
        }
    };
    warn_if_experimental(device, dmm.profile());
    warn_cables_at_open(selection, opts, dmm.transport());
    Ok((dmm, recorder, device, detected))
}

/// Identify the meter and hand the cable straight back, for the listings that
/// need a registry entry but nothing from the meter itself.
fn detect_only(
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<&'static SelectableDevice, crate::l10n::Failure> {
    let (transport, bridge) = dmm_lib::open_transport(&[], opts)
        .map_err(|e| open_error_help(Selection::Auto, opts, e))?;
    let detected = dmm_lib::detect::detect_device(&*transport, bridge)
        .map_err(|e| open_error_help(Selection::Auto, opts, e))?;
    note_detected(&detected);
    Ok(detected.device)
}

/// The entry a listing describes. Nothing is read from the meter, but with
/// nothing selected there is still a meter to identify — the alternative is
/// listing another model's commands or capture steps.
pub(crate) fn device_for_listing(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
) -> Result<&'static SelectableDevice, crate::l10n::Failure> {
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
    let l = crate::l10n::get();
    let warning = l
        .help()
        .stability_warning(&prose_name(profile_name(profile)), profile.stability);
    eprintln!(
        "{}",
        style(tr!(l, "connect-unverified-warning", warning = warning))
            .yellow()
            .bold()
    );
    eprintln!("{}", style(tr!(l, "connect-unverified-capture")).yellow());
    eprintln!(
        "{}",
        style(format!(
            "  dmm-cli --device {} capture",
            meter_listing(device).id()
        ))
        .yellow()
    );
    eprintln!(
        "{}",
        style(tr!(
            l,
            "connect-report-feedback",
            url = profile.feedback_url()
        ))
        .yellow()
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
        let l = crate::l10n::get();
        let warning = l
            .help()
            .stability_warning(&prose_name(profile_name(profile)), profile.stability);
        eprintln!();
        eprintln!(
            "{}",
            style(tr!(
                l,
                "connect-unverified-note",
                warning = warning,
                url = profile.feedback_url()
            ))
            .yellow()
        );
    }
}

/// Why an open failed, once the help printed above it has said what to try:
/// the closing `Error:` line ([`crate::l10n::Localized`]).
#[derive(Debug, Clone, Copy)]
pub(crate) enum OpenFailed {
    NotFound,
    NotIdentified,
    AdapterNotFound,
    /// `--device` named a meter on a cable where only a simulated one
    /// could open; the id it named.
    NotSimulated(&'static str),
}

impl crate::l10n::Localized for OpenFailed {
    fn text<'a>(&'a self, l: &'a Localizer) -> Cow<'a, str> {
        match *self {
            Self::NotFound => tr!(l, "connect-error-not-found"),
            Self::NotIdentified => tr!(l, "connect-error-not-identified"),
            Self::AdapterNotFound => tr!(l, "connect-error-adapter-not-found"),
            Self::NotSimulated(id) => tr!(l, "connect-error-not-simulated", device = id),
        }
    }
}

crate::l10n::localized_error!(OpenFailed);

/// The meter the user picked, for [`Diagnosis::of`]: the one named, as this
/// run calls it. `None` under Auto-detect, which names none.
pub(crate) fn picked(selection: Selection) -> Option<registry::Listing> {
    match selection {
        Selection::Auto => None,
        Selection::Device(device) => Some(meter_listing(device)),
    }
}

/// Print setup help for the failures a user can act on, and return the error
/// to report. The rest go back as they came, for [`crate::report_failure`].
fn open_error_help(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    error: dmm_lib::error::Error,
) -> crate::l10n::Failure {
    match Diagnosis::of(&error, opts.adapter, picked(selection)) {
        Diagnosis::NotFound { bluetooth_searched } => {
            // The radio skipped only for its switch: say how to turn it on.
            let hint = bluetooth_off_hint(crate::l10n::get());
            let off = !bluetooth_searched
                && dmm_shared::help::bluetooth_switched_off(
                    opts.bluetooth,
                    opts.adapter,
                    opened_device(selection),
                );
            print_not_found_help(if off {
                LinksSearched::UsbBluetoothOff { hint: &hint }
            } else {
                LinksSearched::from_usb_failure(bluetooth_searched)
            });
            print_experimental_note(selection);
            OpenFailed::NotFound.into()
        }
        // A meter with the radio built in, out of range or asleep: its own
        // steps, not a cable's.
        Diagnosis::BluetoothOnlyNotFound { model, activation } => {
            print_not_found_help(LinksSearched::BluetoothOnly { model, activation });
            print_experimental_note(selection);
            OpenFailed::NotFound.into()
        }
        // The cable is there and nothing on it spoke. Every meter it could
        // carry has something the user has to switch on, so list them.
        Diagnosis::NotIdentified {
            bridge,
            built_in_radio,
        } => {
            let l = crate::l10n::get();
            let headline = tr!(
                l,
                "connect-not-identified",
                link = dmm_shared::l10n::link_key(
                    Some(dmm_lib::transport::Link::of_bridge(bridge)),
                    built_in_radio
                ),
                title = l.help().no_meter_answered(),
            );
            eprintln!("{}", style(headline).yellow().bold());
            print_picked();
            for (instructions, names) in
                dmm_shared::help::activation_groups(&dmm_lib::devices_on_bridge(bridge))
            {
                let names: Vec<Cow<'_, str>> = names
                    .into_iter()
                    .map(|name| l.label(TextKind::Model, name))
                    .collect();
                eprintln!("\n{}", style(names.join(", ")).yellow());
                for line in l.label(TextKind::Activation, instructions).lines() {
                    eprintln!("{}", style(format!("  {line}")).dim());
                }
            }
            eprintln!();
            for line in not_identified_self_help(l).lines() {
                eprintln!("{}", style(line).yellow());
            }
            OpenFailed::NotIdentified.into()
        }
        // An address nothing answered: the bus listing would be beside the
        // point, and the only thing that says which addresses are live is a
        // scan, which the open has just run.
        Diagnosis::NothingAtAddress { address } => {
            print_not_found_help(LinksSearched::BluetoothAt(&address));
            OpenFailed::AdapterNotFound.into()
        }
        Diagnosis::AdapterNotFound => {
            let l = crate::l10n::get();
            let detail = l.adapter_named(&error).unwrap_or_default();
            eprintln!("{} {}", error_label(), l.error(&error));
            // Only a real list is worth setting apart and worth a hint.
            let adapters = dmm_shared::help::connected_adapters();
            let listed = adapters.offers_choice();
            if listed {
                eprintln!();
            }
            for line in l.help().adapter_lines(&adapters) {
                eprintln!("{}", style(line).yellow());
            }
            if listed {
                let hint = tr!(l, "connect-adapter-select", selector = ADAPTER_SELECTOR);
                eprintln!("\n{}", style(hint).dim());
            }
            if let Some(hint) = l.help().colonless_address_hint(&detail) {
                eprintln!("\n{}", style(hint).yellow());
            }
            OpenFailed::AdapterNotFound.into()
        }
        _ => error.into(),
    }
}

/// The mock scenario a run on `selection` pins: `--mock-mode` when given,
/// else the settings file's `mock_mode`, which the GUI's Mock mode row
/// writes. `None` for a meter on a cable, and for a saved name the mock
/// doesn't know, which is warned about so the mock cycling isn't a mystery —
/// the GUI does the same with it.
pub(crate) fn pinned_mock_mode(
    selection: Selection,
    flag: Option<MockMode>,
    saved: Option<&dmm_shared::SharedSettings>,
) -> Option<MockMode> {
    if flag.is_some() || requires_hardware(selection) {
        return flag;
    }
    match saved?.mock_mode() {
        Ok(mode) => mode,
        Err(message) => {
            let error = message.lines().next().unwrap_or_default();
            eprintln!(
                "{} {}",
                warning_label(),
                tr!(crate::l10n::get(), "connect-saved-mock-mode", error = error),
            );
            None
        }
    }
}

/// Open the meter `selection` names for a command that has no clock flags:
/// on its cable, with the setup help a failed open prints, or the simulated
/// one, pinned to `mock_mode`.
pub(crate) fn open_any(
    selection: Selection,
    opts: dmm_lib::OpenOptions<'_>,
    mock_mode: Option<MockMode>,
) -> Result<BoxedDmm, crate::l10n::Failure> {
    if requires_hardware(selection) {
        Ok(open_with_help(selection, opts)?.0)
    } else {
        Ok(open_mock_device(selection, mock_mode, dmm_lib::Clock::real())?.boxed())
    }
}

/// Open the simulated device `selection` names on `clock`, the UT61E+ mock
/// pinned to `mock_mode` when one was given.
pub(crate) fn open_mock_device(
    selection: Selection,
    mock_mode: Option<MockMode>,
    clock: dmm_lib::Clock,
) -> Result<dmm_lib::Dmm<dmm_lib::transport::NullTransport>, crate::l10n::Failure> {
    // Auto is a cable to open and never lands here.
    let Selection::Device(device) = selection else {
        return Err(OpenFailed::NotSimulated(selection_id(selection)).into());
    };
    Ok(dmm_lib::mock::open_simulated(device, mock_mode, clock)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A raised flag settles any open: one that got through, and one that
    /// failed with help of its own, end alike and say nothing. Down or absent,
    /// the open's own result goes through.
    #[test]
    fn a_raised_stop_flag_overtakes_any_open() {
        use dmm_lib::error::Error;
        use std::sync::atomic::AtomicBool;
        let (raised, down) = (AtomicBool::new(true), AtomicBool::new(false));
        assert!(matches!(
            unless_stopped(Ok(1), Some(&raised)),
            Err(Error::Cancelled)
        ));
        assert!(matches!(
            unless_stopped::<()>(Err(Error::Timeout), Some(&raised)),
            Err(Error::Cancelled)
        ));
        assert!(matches!(unless_stopped(Ok(1), Some(&down)), Ok(1)));
        // The signal's read error can come before its handler has run.
        assert!(matches!(
            unless_stopped::<()>(Err(Error::interrupted()), Some(&down)),
            Err(Error::Cancelled)
        ));
        assert!(matches!(
            unless_stopped::<()>(Err(Error::interrupted()), None),
            Err(Error::Hid(_))
        ));
        assert!(matches!(unless_stopped(Ok(1), None), Ok(1)));
        assert!(matches!(
            unless_stopped::<()>(Err(Error::Timeout), None),
            Err(Error::Timeout)
        ));
        // Reaches its caller as it left, so the command can tell.
        let failure = open_error_help(
            Selection::Auto,
            dmm_lib::OpenOptions::new(),
            Error::Cancelled,
        );
        assert!(matches!(failure.lib(), Some(Error::Cancelled)));
    }

    /// The line names the meter by the name it was picked by, and says
    /// where: a pick saved by dmm-gui is the one a user forgets.
    /// Data that won't read names the pick, or the meter to pick when the
    /// meter names one; nothing under Auto-detect for plain bad data, and
    /// nothing for a recording, which names its own meter.
    #[test]
    fn unreadable_data_points_at_the_pick() {
        use dmm_lib::error::Error;
        let l = crate::l10n::english();
        let mismatch = Error::ChecksumMismatch {
            expected: 1,
            actual: 2,
        };
        let other = Error::WrongMeter {
            reason: dmm_lib::error::WrongMeter::OtherSeries {
                found: "Brymen BM82x",
                opened: "Brymen BM86x",
                id: "bm82x",
            },
            raw: Vec::new(),
        };
        let saved = registry::resolve_listing("bm86x").map(|l| (l, PickedBy::Settings));
        let hint = pick_hint(l, &mismatch, saved).expect("the pick is named");
        assert!(hint.contains("saved by dmm-gui"), "{hint}");
        assert_eq!(pick_hint(l, &mismatch, None), None);
        assert_eq!(
            pick_hint(l, &other, saved).as_deref(),
            Some("Pass --device bm82x, or --device auto.")
        );
        assert_eq!(
            pick_hint(l, &other, None).as_deref(),
            Some("Pass --device bm82x.")
        );
        let recorded = registry::resolve_listing("bm86x").map(|l| (l, PickedBy::Recording));
        assert_eq!(pick_hint(l, &mismatch, recorded), None);
        assert_eq!(pick_hint(l, &Error::LinkLost, saved), None);
    }

    #[test]
    fn the_selected_line_says_where_the_meter_was_picked() {
        let et2517a = registry::resolve_listing("et-2517a").expect("a registry row");
        assert_eq!(
            selected_line(crate::l10n::english(), et2517a, PickedBy::Flag),
            "Selected: Minipa ET-2517A (--device et2517a)"
        );
        assert_eq!(
            selected_line(crate::l10n::english(), et2517a, PickedBy::Settings),
            "Selected: Minipa ET-2517A (saved by dmm-gui; pass --device auto to detect the meter)"
        );
        assert_eq!(
            selected_line(crate::l10n::english(), et2517a, PickedBy::Recording),
            "Selected: Minipa ET-2517A (named by the recording)"
        );
    }

    /// The paragraph that closes the help for a silent cable keeps its
    /// layout: the commands indented under the prose they belong to.
    #[test]
    fn the_name_it_paragraph_indents_its_commands() {
        assert_eq!(
            not_identified_self_help(crate::l10n::get()),
            "If the meter is on and transmitting but still not recognised, name it:\n  \
             dmm-cli --device <id> ...      (dmm-cli --help lists the ids)\n\
             and please report it with RUST_LOG=dmm_lib=debug output at\n  \
             https://github.com/antoinecellerier/dmm-tools/issues"
        );
    }

    /// A reported name shows what it holds, as `{:?}` would, inside the
    /// message's own quotes.
    #[test]
    fn a_reported_name_is_escaped_without_its_quotes() {
        assert_eq!(escaped("UT216XD"), "UT216XD");
        assert_eq!(escaped("a\"b\n\u{1}"), "a\\\"b\\n\\u{1}");
        let line = std::cell::RefCell::new(String::new());
        print_reported_name(crate::l10n::english(), Some("UT\u{1b}[2J"), |l| {
            *line.borrow_mut() = l
        });
        let line = line.into_inner();
        assert!(
            line.contains("UT\\u{1b}[2J") && !line.contains("\u{1b}[2J"),
            "{line}"
        );
    }
}
