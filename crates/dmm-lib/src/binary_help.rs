//! Help and version text shared by the `dmm-cli` and `dmm-gui` binaries.
//!
//! The lists come from the authorities on what exists (the registry for
//! devices, [`MockMode::ALL`] for mock scenarios), so an addition there
//! reaches both binaries' `--help`. The prose (setup hint, experimental
//! warning) is here because the two copies had already drifted apart.
//!
//! The per-crate build values (`CARGO_PKG_VERSION`, `GIT_HASH`) are passed in
//! rather than read here: `env!` would capture *this* crate's values, not the
//! binary's.

use crate::list_devices;
use crate::mock::MockMode;
use crate::protocol::registry::SelectableDevice;
use crate::protocol::{Stability, registry};

/// Version text for `--version`, with the git hash appended on dev builds.
///
/// Leaks on the dev path because clap's `version` attribute needs a
/// `&'static str`; it happens once per process.
pub fn version_string(version: &'static str, git_hash: &str) -> &'static str {
    if version.contains("-dev") {
        Box::leak(format!("{version} ({git_hash})").into_boxed_str())
    } else {
        version
    }
}

/// Short version label for display in a UI, e.g. `v0.6.0-dev (abc1234)`.
pub fn version_label(version: &str, git_hash: &str) -> String {
    if version.contains("-dev") {
        format!("v{version} ({git_hash})")
    } else {
        format!("v{version}")
    }
}

/// Long help for a `--device` flag: `intro`, then one line per known device.
///
/// `intro` is the only part that differs between the binaries — the CLI's is
/// bare, the GUI's notes that the flag overrides saved settings.
pub fn device_help(intro: &str) -> String {
    let mut help = String::with_capacity(intro.len() + registry::DEVICES.len() * 48 + 160);
    help.push_str(intro);
    help.push_str("\n\nDevices:\n");
    // Leads the list: it is what the flag does when nothing names a meter, and
    // the one value the registry does not carry.
    help.push_str(&format!(
        "  {:<12} Detect the connected meter (default)\n",
        registry::AUTO_DEVICE_ID
    ));
    for d in registry::DEVICES {
        let stability = (d.new_protocol)().profile().stability;
        let tag = if !d.requires_hardware {
            " (no hardware required)".to_string()
        } else if stability.is_verified() {
            String::new()
        } else {
            format!(" ({})", stability.label())
        };
        help.push_str(&format!("  {:<12} {}{}\n", d.id, d.display_name, tag));
    }
    help.push_str(
        "\nAlso accepts aliases: ut61e+, ut61b, ut171a, ut181, etc.\n\
         Quote names with special characters: --device 'ut61e+'",
    );
    help
}

/// Long help for a `--mock-mode` flag: `intro`, the mode list, then `example`.
///
/// The list is rendered from [`MockMode::ALL`] for the same reason the device
/// list comes from the registry — the CLI's hardcoded copy had fallen four
/// modes behind the mock. `intro` and `example` differ between the binaries:
/// the CLI's flag sits on a subcommand, the GUI's implies `--device mock`.
pub fn mock_mode_help(intro: &str, example: &str) -> String {
    format!(
        "{intro}\n\nModes: {}\n\nExample: {example}",
        MockMode::label_list()
    )
}

/// The link a meter is on, as the user knows it.
///
/// Never the bridge chip: someone plugged in a USB cable or switched a
/// Bluetooth adapter on, and has no reason to know which chip is inside it.
/// The error text, the CLI help and the GUI's connection messages all take
/// their wording from here so the three cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Link {
    UsbCable,
    Bluetooth,
}

impl Link {
    /// The link a bridge is on, by the name its transport gives, and `None`
    /// for a transport with nothing on the far end: the mock and a replay
    /// answer from inside the process.
    pub fn from_bridge(bridge: &str) -> Option<Self> {
        match bridge {
            crate::transport::NO_LINK => None,
            crate::BLUETOOTH => Some(Self::Bluetooth),
            _ => Some(Self::UsbCable),
        }
    }

    /// The name for a status line that already names the meter, and what a
    /// replay file's `# link:` line records.
    ///
    /// "Bluetooth adapter" doubles the width of a UT61E+ label for a word the
    /// label around it no longer needs.
    pub fn short_name(self) -> &'static str {
        match self {
            Self::UsbCable => USB_CABLE,
            Self::Bluetooth => BLUETOOTH_LINK,
        }
    }

    /// The name for text with the room to spell it out — an error, or a
    /// hover where the bar had to shorten or drop it.
    ///
    /// `built_in_radio` is a meter with Bluetooth built in on the far end
    /// (a `bluetooth_only` registry entry, or a peer advertising the name of
    /// one): there is no adapter to name.
    pub fn full_name(self, built_in_radio: bool) -> &'static str {
        match self {
            Self::UsbCable => USB_CABLE,
            Self::Bluetooth if built_in_radio => BLUETOOTH_BUILT_IN,
            Self::Bluetooth => BLUETOOTH_ADAPTER,
        }
    }

    /// A link read back from a file by its [`Link::short_name`].
    ///
    /// `None` for anything else, so a recording made by a version that knows
    /// a link this one does not still plays — it just says nothing about the
    /// link.
    pub(crate) fn from_short_name(name: &str) -> Option<Self> {
        [Self::UsbCable, Self::Bluetooth]
            .into_iter()
            .find(|link| link.short_name() == name)
    }
}

/// The full name of the link `bridge` is on, and "link" for a transport with
/// none — an error about it still reads as a sentence. `built_in_radio` as
/// for [`Link::full_name`].
pub fn bridge_link_name(bridge: &str, built_in_radio: bool) -> &'static str {
    Link::from_bridge(bridge).map_or("link", |link| link.full_name(built_in_radio))
}

/// The meters on a bridge, grouped by the steps that switch their
/// transmission on.
///
/// Several entries share one instruction block — every UT61+/UT161 model, the
/// UT8802 and the UT8803 — so a list per device would print the same four
/// lines six times over. Registry order is kept, and a group is named by the
/// display names that share it.
pub fn activation_groups(
    devices: &[&'static SelectableDevice],
) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut groups: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for device in devices {
        match groups
            .iter_mut()
            .find(|(instructions, _)| *instructions == device.activation_instructions)
        {
            Some((_, names)) => names.push(device.display_name),
            None => groups.push((device.activation_instructions, vec![device.display_name])),
        }
    }
    groups
}

/// The cable link, in both the long and the short form.
const USB_CABLE: &str = "USB cable";

/// The radio link where the words around it already say what it is.
const BLUETOOTH_LINK: &str = "Bluetooth";

/// The radio link where they don't.
const BLUETOOTH_ADAPTER: &str = "Bluetooth adapter";

/// The same, for a meter with the radio built in, which has no adapter.
const BLUETOOTH_BUILT_IN: &str = "Bluetooth link";

/// What a recording with no link recorded is played back as.
///
/// Every replay file written before the link was recorded came off a cable,
/// and a session that says nothing about its link is less use than one that
/// says the thing all of them had in common.
pub(crate) const RECORDED_LINK_DEFAULT: Option<Link> = Some(Link::UsbCable);

/// What the USB label line says to check, whatever the platform.
const CABLE_CHECK: &str = "check it is plugged in and the meter is powered on.";

/// What the Bluetooth label line says to check. Generic on purpose: which
/// adapters we speak to is the catalog's business, not a message's.
const BLUETOOTH_CHECK: &str =
    "turn on Bluetooth on this computer and data transmission on the meter.";

#[cfg(target_os = "linux")]
const USB_STEPS: &[&str] = &[
    "Ensure the udev rule is installed:",
    "  sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/",
    "  sudo udevadm control --reload-rules",
    "Then replug the cable. On a headless machine, keep a group on the rule:",
    "  https://github.com/antoinecellerier/dmm-tools/blob/main/docs/setup.md",
];

#[cfg(target_os = "windows")]
const USB_STEPS: &[&str] = &[
    "Open Device Manager with the cable plugged in:",
    "- 'CP2110 USB to UART Bridge' under HID devices: no action needed.",
    "- 'USB Input Device' under HID devices: no action needed.",
    "- Yellow warning icon under 'Other devices': install the driver from",
    "  https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers",
    "- Nothing appears: try a different USB port.",
];

#[cfg(target_os = "macos")]
const USB_STEPS: &[&str] = &[
    "The cable should be recognized automatically (no driver needed).",
    "If the device is not found, check System Settings > Privacy & Security > Input Monitoring.",
];

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const USB_STEPS: &[&str] = &[];

/// The GUI setting that lets an open search the radio, as its checkbox
/// reads. The CLI reads the same saved setting, so its help names it too.
pub const BLUETOOTH_SETTING: &str = "Look for Bluetooth devices";

/// What turns the radio search back on in the CLI, for a meter with the
/// radio built in that was not looked for: `flag_given` when
/// `--no-bluetooth` switched it off, else the saved setting did.
pub fn cli_bluetooth_off_hint(flag_given: bool) -> String {
    if flag_given {
        "Leave out --no-bluetooth.".to_string()
    } else {
        format!("Tick \"{BLUETOOTH_SETTING}\" in dmm-gui's settings, which dmm-cli reads too.")
    }
}

/// The same in the GUI, where ticking the setting also clears a
/// `--no-bluetooth` given at start.
pub fn gui_bluetooth_off_hint() -> String {
    format!("Tick \"{BLUETOOTH_SETTING}\" in Settings (\u{2699}).")
}

/// The step that wakes an adapter: it stops advertising in standby, and its
/// own power switch wakes it
/// (docs/research/ut-d07b/reverse-engineered-protocol.md §4).
const BLUETOOTH_ADAPTER_STEPS: &[&str] = &["If both are on, switch the adapter off and on."];

/// What this computer's radio may need, adapter or not.
const BLUETOOTH_HOST_STEPS: &[&str] = &[
    #[cfg(target_os = "macos")]
    "Allow Bluetooth in System Settings > Privacy & Security.",
];

/// Where the adapters and the meters that take one are listed, for the help
/// that has no address to go on.
const BLUETOOTH_CATALOG: &[&str] = &[
    "Supported adapters and meters are listed in the device catalog:",
    "  https://github.com/antoinecellerier/dmm-tools/blob/main/docs/supported-devices.md",
];

/// What to do instead when an address was named and nothing answered it: the
/// scan is the only thing that says which addresses are live.
const BLUETOOTH_SCAN: &[&str] = &["Run 'dmm-cli list' to scan for adapters and meters in range."];

/// One link's worth of "nothing found" help: a label line, then its steps.
///
/// Keyed by the link the user knows it by, so the two links never interleave
/// and a third (LAN, for the bench meters) is another section rather than
/// more lines in this one. The CLI prints `link` bold and dims the steps that
/// are commands or URLs; the GUI draws the label line bold above them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupSection {
    /// The link, as the user names it: "USB cable", "Bluetooth".
    pub link: &'static str,
    /// The rest of the label line, after `<link>: `.
    pub check: &'static str,
    /// One step per line, which the renderer indents. A step that starts with
    /// a space of its own is a command to run or a URL to open.
    pub steps: Vec<&'static str>,
}

impl SetupSection {
    /// The label line, `<link>: <check>`, for a renderer that draws it whole.
    pub fn label(&self) -> String {
        format!("{}: {}", self.link, self.check)
    }

    /// The section as plain text, the steps indented under the label.
    pub fn text(&self) -> String {
        let mut out = self.label();
        for step in &self.steps {
            out.push_str("\n  ");
            out.push_str(step);
        }
        out
    }
}

/// Which links a failed open looked at — what the help is allowed to talk
/// about, and what its title says was not found.
///
/// Narrowed by what the user asked for: a meter that only ships with a cable,
/// or Bluetooth switched off, gets the cable section alone, and an address on
/// `--adapter` gets the radio alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinksSearched<'a> {
    /// The USB cables alone.
    Usb,
    /// Both links, which is what an unnamed meter gets.
    UsbAndBluetooth,
    /// One Bluetooth adapter, named by address or peripheral id.
    BluetoothAt(&'a str),
    /// The radio alone, for a meter with Bluetooth built in: its display
    /// name and the registry's steps that switch its radio on.
    BluetoothOnly {
        model: &'a str,
        activation: &'static str,
    },
}

impl LinksSearched<'_> {
    /// What a failed open searched, from whether it reached the radio.
    pub fn from_usb_failure(bluetooth_searched: bool) -> Self {
        if bluetooth_searched {
            Self::UsbAndBluetooth
        } else {
            Self::Usb
        }
    }

    /// The line that says what was not found.
    ///
    /// No full stop: the CLI ends the sentence, the GUI's notice titles carry
    /// none.
    pub fn not_found_title(&self) -> String {
        match self {
            Self::Usb => "No USB cable found".to_string(),
            Self::UsbAndBluetooth => "No meter found over USB or Bluetooth".to_string(),
            Self::BluetoothAt(address) => format!("No Bluetooth device found at {address}"),
            Self::BluetoothOnly { model, .. } => format!("No {model} found over Bluetooth"),
        }
    }

    /// The sections to print under that title, in the order they are printed.
    pub fn sections(&self) -> Vec<SetupSection> {
        match self {
            Self::Usb => vec![usb_section()],
            Self::UsbAndBluetooth => vec![usb_section(), bluetooth_section(BLUETOOTH_CATALOG)],
            // No cable was looked at, so nothing about cables applies — and
            // the address the user typed is the only thing that did not
            // answer.
            Self::BluetoothAt(_) => vec![bluetooth_section(BLUETOOTH_SCAN)],
            // No adapter to wake, and the meter named: its own steps say how
            // to switch its radio on.
            Self::BluetoothOnly { activation, .. } => vec![SetupSection {
                link: BLUETOOTH_LINK,
                check: BLUETOOTH_CHECK,
                steps: activation
                    .lines()
                    .chain(BLUETOOTH_HOST_STEPS.iter().copied())
                    .collect(),
            }],
        }
    }
}

fn usb_section() -> SetupSection {
    SetupSection {
        link: USB_CABLE,
        check: CABLE_CHECK,
        steps: USB_STEPS.to_vec(),
    }
}

fn bluetooth_section(closing: &'static [&'static str]) -> SetupSection {
    SetupSection {
        link: BLUETOOTH_LINK,
        check: BLUETOOTH_CHECK,
        steps: BLUETOOTH_ADAPTER_STEPS
            .iter()
            .chain(BLUETOOTH_HOST_STEPS)
            .chain(closing)
            .copied()
            .collect(),
    }
}

/// The sentence both binaries use to say a protocol is not fully verified.
///
/// Only the claim is shared: each site appends its own call to action (the
/// CLI a `capture` command, the GUI a link), because they differ in what the
/// user can do next. Four hand-written spellings had drifted apart here.
pub fn experimental_warning(model_name: &str, stability: Stability) -> String {
    let detail = match stability {
        Stability::Verified => "verified against real hardware",
        Stability::PartlyVerified => "some modes and commands unconfirmed on real hardware",
        Stability::Experimental => "unverified against real hardware",
    };
    format!("{model_name} support is {} ({detail}).", stability.label())
}

/// What both binaries answer when the clock flags meet a hardware device.
///
/// `--mock-clock-scale` and `--mock-clock-preseed` bend session time, and a
/// real meter is paced by USB: honouring them there would stamp readings with
/// instants the meter never produced. A replay takes them too — its readings
/// are already recorded, so time can be run through them at any speed. Shared
/// so the CLI and the GUI refuse with the same sentence. The name predates
/// the replay and is left alone: both binaries print it by it.
pub const MOCK_CLOCK_MOCK_ONLY: &str = "--mock-clock-scale and --mock-clock-preseed \
                                        only apply to the mock device and to a replay. \
                                        Re-run with --device mock or --replay, or \
                                        without the clock flags.";

/// What the bus held when an `--adapter` selector matched nothing.
///
/// Both binaries print the same lines but only attach their "pick another
/// adapter" hint when devices were actually listed. Naming the three answers
/// keeps that decision off a line count.
pub enum ConnectedAdapters {
    /// Nothing is plugged in.
    None,
    /// One `  [i] dev` line per device, without the header.
    Listed(Vec<String>),
    /// HID enumeration itself failed, so there is nothing to list.
    Unavailable,
}

/// What is on the bus, for the "adapter not found" error both binaries print.
///
/// Walks every HID device on the system, so call it once when the error
/// arrives rather than from a render path. The answer is a snapshot either
/// way: acting on it means restarting with a different selector.
pub fn connected_adapters() -> ConnectedAdapters {
    match list_devices() {
        Ok(devices) if devices.is_empty() => ConnectedAdapters::None,
        Ok(devices) => ConnectedAdapters::Listed(
            devices
                .iter()
                .enumerate()
                .map(|(i, dev)| format!("  [{i}] {dev}"))
                .collect(),
        ),
        // The HID API itself failed, so there is nothing to list; `list` says
        // the same thing with the setup help attached.
        Err(_) => ConnectedAdapters::Unavailable,
    }
}

impl ConnectedAdapters {
    /// The lines both binaries print, header included.
    ///
    /// No styling and no trailing hint — the CLI tells the user to re-run with
    /// a different `--adapter`, the GUI to restart with one, and only the list
    /// of what is plugged in is common to both.
    pub fn lines(&self) -> Vec<String> {
        match self {
            Self::None => vec!["No devices currently connected.".to_string()],
            Self::Listed(devices) => {
                let mut lines = Vec::with_capacity(devices.len() + 1);
                lines.push("Connected devices:".to_string());
                lines.extend(devices.iter().cloned());
                lines
            }
            Self::Unavailable => {
                vec!["Run 'dmm-cli list' to see connected devices.".to_string()]
            }
        }
    }
}

/// The line both binaries add when an `--adapter` value nothing matched is
/// twelve hex digits — a Bluetooth address without its colons, the way
/// Windows' Device Manager shows one.
///
/// Only a hint: a USB cable's serial number can be the same twelve digits,
/// so the value is not opened as an address. `None` for any other value, and
/// in a build without the radio.
pub fn colonless_address_hint(selector: &str) -> Option<String> {
    if !cfg!(feature = "bluetooth")
        || selector.len() != 12
        || !selector.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let pairs: Vec<String> = selector
        .as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8_lossy(pair).to_ascii_uppercase())
        .collect();
    Some(format!(
        "If that is a Bluetooth address, write it {}.",
        pairs.join(":")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_version_has_no_hash() {
        assert_eq!(version_string("0.5.0", "abc1234"), "0.5.0");
        assert_eq!(version_label("0.5.0", "abc1234"), "v0.5.0");
    }

    #[test]
    fn dev_version_carries_the_hash() {
        assert_eq!(
            version_string("0.6.0-dev", "abc1234"),
            "0.6.0-dev (abc1234)"
        );
        assert_eq!(
            version_label("0.6.0-dev", "abc1234"),
            "v0.6.0-dev (abc1234)"
        );
    }

    #[test]
    fn device_help_lists_every_registry_device() {
        let help = device_help("Device to connect to.");
        assert!(help.starts_with("Device to connect to.\n\nDevices:\n"));
        // Auto is not a registry entry, so nothing below would list it — and
        // it leads, being what a user who names nothing gets.
        let listing = help.split("Devices:\n").nth(1).expect("device list");
        assert!(
            listing.starts_with("  auto         Detect the connected meter (default)\n"),
            "{listing}"
        );
        for d in registry::DEVICES {
            assert!(help.contains(d.id), "missing {}", d.id);
            assert!(help.contains(d.display_name), "missing {}", d.display_name);
        }
        assert!(help.contains("Also accepts aliases"));
    }

    /// Both binaries render these lines verbatim, so the header and the empty
    /// and unavailable wordings are part of their output, not an internal
    /// detail.
    #[test]
    fn connected_adapter_lines_match_each_outcome() {
        assert_eq!(
            ConnectedAdapters::None.lines(),
            ["No devices currently connected."]
        );
        assert_eq!(
            ConnectedAdapters::Listed(vec!["  [0] one".into(), "  [1] two".into()]).lines(),
            ["Connected devices:", "  [0] one", "  [1] two"]
        );
        assert_eq!(
            ConnectedAdapters::Unavailable.lines(),
            ["Run 'dmm-cli list' to see connected devices."]
        );
    }

    /// A Bluetooth address typed the way Device Manager shows it gets the
    /// colon form back; anything else that is not twelve hex digits gets
    /// nothing.
    #[cfg(feature = "bluetooth")]
    #[test]
    fn a_colonless_address_is_shown_its_colon_form() {
        assert_eq!(
            colonless_address_hint("123456789abc").as_deref(),
            Some("If that is a Bluetooth address, write it 12:34:56:78:9A:BC.")
        );
        assert!(colonless_address_hint("12:34:56:78:9A:BC").is_none());
        assert!(colonless_address_hint("123456789AB").is_none());
        assert!(colonless_address_hint("123456789ABCD").is_none());
        assert!(colonless_address_hint("12345678ZABC").is_none());
        assert!(colonless_address_hint("/dev/hidraw0").is_none());
        // Twelve bytes but not hex: no pair may split a character.
        assert!(colonless_address_hint("éééééé").is_none());
    }

    /// The CLI's copy of this list went four modes stale; rendering it from
    /// the table is only worth it if every mode really reaches the help.
    #[test]
    fn mock_mode_help_lists_every_mode() {
        let help = mock_mode_help("Pin the mock.", "--mock-mode dcv");
        assert!(help.starts_with("Pin the mock.\n\nModes: "));
        for mode in MockMode::ALL {
            assert!(help.contains(mode.label()), "missing {}", mode.label());
        }
        assert!(help.ends_with("Example: --mock-mode dcv"));
    }

    /// Both binaries render these lines verbatim: the CLI dims the indented
    /// ones, the GUI joins them with newlines, so a blank line would show up
    /// as a gap in the middle of a section.
    #[test]
    fn every_section_is_printable_on_every_platform() {
        for links in [
            LinksSearched::Usb,
            LinksSearched::UsbAndBluetooth,
            LinksSearched::BluetoothAt("12:34:56:78:9A:BC"),
            ut60bt(),
        ] {
            for section in links.sections() {
                assert!(!section.link.is_empty());
                assert!(section.check.ends_with('.'), "{}", section.check);
                assert!(section.steps.iter().all(|line| !line.trim().is_empty()));
            }
        }
    }

    /// The help covers the links the open path actually tried, and no others:
    /// a cable-only meter, a build without the feature and a switched-off
    /// radio all reach the cable section alone.
    #[test]
    fn the_sections_are_the_links_that_were_searched() {
        let links = |l: LinksSearched| -> Vec<&'static str> {
            l.sections().iter().map(|s| s.link).collect()
        };
        assert_eq!(links(LinksSearched::Usb), ["USB cable"]);
        assert_eq!(
            links(LinksSearched::UsbAndBluetooth),
            ["USB cable", "Bluetooth"]
        );
        assert_eq!(links(LinksSearched::BluetoothAt("4C:3C")), ["Bluetooth"]);
        assert_eq!(links(ut60bt()), ["Bluetooth"]);
    }

    /// A meter with the radio built in, as the binaries build its help from
    /// its registry entry.
    fn ut60bt() -> LinksSearched<'static> {
        let device = registry::find_device("ut60bt").expect("registry entry");
        LinksSearched::BluetoothOnly {
            model: device.display_name,
            activation: device.activation_instructions,
        }
    }

    /// A meter with the radio built in is sent to its own switch, not to an
    /// adapter's, and not to the catalog: the user already named it.
    #[test]
    fn a_bluetooth_only_meter_gets_its_own_steps() {
        let links = ut60bt();
        assert_eq!(links.not_found_title(), "No UT60BT found over Bluetooth");
        let text = links.sections()[0].text();
        assert!(
            text.starts_with(
                "Bluetooth: turn on Bluetooth on this computer and data transmission on the meter.\n  \
                 1. Turn the meter on\n  2. Long press SEL"
            ),
            "{text}"
        );
        assert!(!text.contains("adapter"), "{text}");
        assert!(!text.contains("supported-devices.md"), "{text}");
    }

    /// The titles are the user's first line of both binaries' help, and the
    /// GUI's notice title on top of that.
    #[test]
    fn the_title_names_what_was_looked_for() {
        assert_eq!(LinksSearched::Usb.not_found_title(), "No USB cable found");
        assert_eq!(
            LinksSearched::UsbAndBluetooth.not_found_title(),
            "No meter found over USB or Bluetooth"
        );
        assert_eq!(
            LinksSearched::BluetoothAt("12:34:56:78:9A:BC").not_found_title(),
            "No Bluetooth device found at 12:34:56:78:9A:BC"
        );
        assert_eq!(
            LinksSearched::from_usb_failure(true),
            LinksSearched::UsbAndBluetooth
        );
        assert_eq!(LinksSearched::from_usb_failure(false), LinksSearched::Usb);
    }

    /// An address nobody answered is answered with the scan, not with the
    /// catalog: the user already knows which adapter they meant.
    #[test]
    fn a_named_address_is_sent_to_the_scan() {
        let selected = LinksSearched::BluetoothAt("4C:3C").sections();
        let text = selected[0].text();
        assert!(text.contains("dmm-cli list"), "{text}");
        assert!(!text.contains("supported-devices.md"), "{text}");

        let unnamed = LinksSearched::UsbAndBluetooth.sections();
        let text = unnamed[1].text();
        assert!(text.contains("supported-devices.md"), "{text}");
        assert!(!text.contains("dmm-cli list"), "{text}");
    }

    /// No product name in either binary: which adapters work is the catalog's
    /// business, and a message that names one dates the moment a second is
    /// supported.
    #[test]
    fn the_sections_name_no_product() {
        for links in [
            LinksSearched::Usb,
            LinksSearched::UsbAndBluetooth,
            LinksSearched::BluetoothAt("4C:3C"),
            ut60bt(),
        ] {
            for section in links.sections() {
                let text = section.text();
                assert!(!text.contains("UT-D07"), "{text}");
            }
        }
    }

    /// The user plugged in a cable or switched an adapter on; either way the
    /// bridge chip must stay out of what they read.
    #[test]
    fn a_link_is_named_cable_or_adapter_never_the_chip() {
        assert_eq!(
            bridge_link_name(crate::BLUETOOTH, false),
            "Bluetooth adapter"
        );
        for bridge in ["CP2110", "CH9329", "CH9325"] {
            assert_eq!(Link::from_bridge(bridge), Some(Link::UsbCable));
            for built_in in [false, true] {
                assert_eq!(bridge_link_name(bridge, built_in), "USB cable");
            }
        }
    }

    /// A meter with the radio built in has no adapter for the text to name.
    #[test]
    fn a_built_in_radio_is_no_adapter() {
        assert_eq!(bridge_link_name(crate::BLUETOOTH, true), "Bluetooth link");
        assert_eq!(Link::Bluetooth.full_name(true), "Bluetooth link");
    }

    /// Each binary names its own way to turn the search back on, and the
    /// GUI's checkbox by the label it draws.
    #[test]
    fn each_binary_names_its_own_bluetooth_switch() {
        assert_eq!(cli_bluetooth_off_hint(true), "Leave out --no-bluetooth.");
        assert_eq!(
            cli_bluetooth_off_hint(false),
            "Tick \"Look for Bluetooth devices\" in dmm-gui's settings, which dmm-cli reads too."
        );
        assert_eq!(
            gui_bluetooth_off_hint(),
            "Tick \"Look for Bluetooth devices\" in Settings (\u{2699})."
        );
    }

    /// A transport with nothing on the far end, such as the mock's, is on no
    /// link — not on a cable by default.
    #[test]
    fn a_transport_with_no_link_names_none() {
        use crate::transport::Transport;
        let bridge = crate::transport::NullTransport.transport_name();
        assert_eq!(Link::from_bridge(bridge), None);
        assert_eq!(bridge_link_name(bridge, false), "link");
    }

    /// The short form drops the word the status line no longer needs; the
    /// full form is the one the errors use.
    #[test]
    fn the_two_forms_of_a_link_name() {
        assert_eq!(Link::Bluetooth.short_name(), "Bluetooth");
        assert_eq!(Link::Bluetooth.full_name(false), "Bluetooth adapter");
        assert_eq!(Link::UsbCable.short_name(), "USB cable");
        assert_eq!(Link::UsbCable.full_name(false), "USB cable");
    }

    /// A replay file records the short name, so reading it back must give
    /// the same link, and a name this version does not write gives none.
    #[test]
    fn a_short_name_reads_back_as_its_link() {
        for link in [Link::UsbCable, Link::Bluetooth] {
            assert_eq!(Link::from_short_name(link.short_name()), Some(link));
        }
        assert_eq!(Link::from_short_name("Bluetooth adapter"), None);
        assert_eq!(Link::from_short_name("carrier pigeon"), None);
    }

    /// The "no meter answered" help lists what to switch on, once per set of
    /// steps: every UT61+/UT161 model shares one block, and so do the UT8802
    /// and the UT8803.
    #[test]
    fn activation_help_lists_each_set_of_steps_once() {
        let devices = crate::devices_on_bridge("CP2110");
        assert!(devices.len() > 1, "CP2110 carries several meters");
        let groups = activation_groups(&devices);
        assert!(groups.len() < devices.len(), "nothing was grouped");

        let instructions: Vec<&str> = groups.iter().map(|(i, _)| *i).collect();
        let mut unique = instructions.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), instructions.len(), "a block is listed twice");

        // Every meter is named exactly once, under its own block.
        let named: Vec<&str> = groups.iter().flat_map(|(_, names)| names.clone()).collect();
        assert_eq!(named.len(), devices.len());
        let ut61 = groups
            .iter()
            .find(|(_, names)| names.contains(&"UT61E+"))
            .expect("the UT61E+ is on the CP2110");
        assert!(ut61.1.contains(&"UT61B+"), "{:?}", ut61.1);
        assert!(ut61.0.contains("USB/Hz"), "{}", ut61.0);
    }

    #[test]
    fn experimental_warning_names_the_model_and_the_level() {
        assert_eq!(
            experimental_warning("UNI-T UT8803", Stability::Experimental),
            "UNI-T UT8803 support is experimental (unverified against real hardware)."
        );
        assert_eq!(
            experimental_warning("UNI-T UT181A", Stability::PartlyVerified),
            "UNI-T UT181A support is partly verified (some modes and commands unconfirmed on real hardware)."
        );
    }

    /// The mock needs no hardware and the families short of verified carry
    /// their level; the tags tell the user what to expect before they plug
    /// anything in.
    #[test]
    fn device_help_tags_mock_and_unverified_devices() {
        let help = device_help("x");
        let mock_line = help
            .lines()
            .find(|l| l.trim_start().starts_with("mock"))
            .expect("mock listed");
        assert!(mock_line.contains("(no hardware required)"), "{mock_line}");
        assert!(help.contains("(experimental)"));
        assert!(help.contains("(partly verified)"));
    }
}
