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

/// What to call the link a meter is on, for messages the user reads.
///
/// Never the bridge chip: someone plugged in a USB cable or switched a
/// Bluetooth adapter on, and has no reason to know which chip is inside it.
/// The error text, the CLI help and the GUI's connection messages all take
/// their wording from here so the three cannot drift.
pub fn link_name(bridge: &str) -> &'static str {
    if bridge == crate::BLUETOOTH {
        BLUETOOTH_ADAPTER
    } else {
        USB_CABLE
    }
}

/// The cable link, in both the long and the short form.
const USB_CABLE: &str = "USB cable";

/// The radio link where the words around it already say what it is.
const BLUETOOTH_LINK: &str = "Bluetooth";

/// The radio link where they don't.
const BLUETOOTH_ADAPTER: &str = "Bluetooth adapter";

/// The full name of a link given in its short form, for text with the room
/// to spell it out — a hover, where the bar had to shorten or drop it.
///
/// Takes what [`short_link_name`] returns, so the two forms of one link
/// cannot come from different wordings.
pub fn full_link_name(short: &str) -> &'static str {
    if short == BLUETOOTH_LINK {
        BLUETOOTH_ADAPTER
    } else {
        USB_CABLE
    }
}

/// The same link, shortened for a status line that already names the meter,
/// and `None` where there is no link at all.
///
/// "Bluetooth adapter" doubles the width of a UT61E+ label for a word the
/// label around it no longer needs. The mock answers from inside the process,
/// so it gets no link name.
pub fn short_link_name(bridge: &str) -> Option<&'static str> {
    match bridge {
        crate::transport::NO_LINK => None,
        crate::BLUETOOTH => Some(BLUETOOTH_LINK),
        _ => Some(USB_CABLE),
    }
}

/// A link name read back from a file, matched against the ones we write.
///
/// `None` for anything else, so a recording made by a version that knows a
/// link this one does not still plays — it just says nothing about the link.
pub(crate) fn link_from_name(name: &str) -> Option<&'static str> {
    [USB_CABLE, BLUETOOTH_LINK]
        .into_iter()
        .find(|known| *known == name)
}

/// What a recording with no link recorded is played back as.
///
/// Every replay file written before the link was recorded came off a cable,
/// and a session that says nothing about its link is less use than one that
/// says the thing all of them had in common.
pub(crate) const RECORDED_LINK_DEFAULT: Option<&'static str> = Some(USB_CABLE);

/// Line the platform setup hint opens with, whatever the platform.
const CABLE_CHECK: &str = "Check that the USB cable is plugged in and the meter is powered on.";

/// First half of the Bluetooth check, shared by every platform. The adapter
/// stops advertising when it goes to sleep, and only the meter can wake it
/// (docs/research/ut-d07b/reverse-engineered-protocol.md §4).
#[cfg(feature = "bluetooth")]
const BLUETOOTH_CHECK: &str =
    "For a UT-D07B adapter, turn on Bluetooth here and data transmission on the meter.";

#[cfg(target_os = "linux")]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
    #[cfg(feature = "bluetooth")]
    BLUETOOTH_CHECK,
    #[cfg(feature = "bluetooth")]
    "If the adapter is never found, switch the meter's data transmission off and on.",
    "Ensure the udev rule is installed:",
    "  sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/",
    "  sudo udevadm control --reload-rules",
    "Then replug the cable. On a headless machine, keep a group on the",
    "rule — see the setup guide:",
    "  https://github.com/antoinecellerier/dmm-tools/blob/main/docs/setup.md",
];

#[cfg(target_os = "windows")]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
    #[cfg(feature = "bluetooth")]
    BLUETOOTH_CHECK,
    #[cfg(feature = "bluetooth")]
    "If the adapter is never found, switch the meter's data transmission off and on.",
    "Open Device Manager with the cable plugged in:",
    "- 'CP2110 USB to UART Bridge' under HID devices: no action needed.",
    "- 'USB Input Device' under HID devices: no action needed.",
    "- Yellow warning icon under 'Other devices': install the driver from",
    "  https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers",
    "- Nothing appears: try a different USB port.",
];

#[cfg(target_os = "macos")]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
    #[cfg(feature = "bluetooth")]
    BLUETOOTH_CHECK,
    #[cfg(feature = "bluetooth")]
    "If the adapter is never found, switch the meter's data transmission off and on,",
    #[cfg(feature = "bluetooth")]
    "and allow Bluetooth in System Settings > Privacy & Security.",
    "The cable should be recognized automatically (no driver needed).",
    "If the device is not found, check System Settings > Privacy & Security > Input Monitoring.",
];

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
    #[cfg(feature = "bluetooth")]
    BLUETOOTH_CHECK,
    #[cfg(feature = "bluetooth")]
    "If the adapter is never found, switch the meter's data transmission off and on.",
];

/// What to try when no USB cable was found, one line per step.
///
/// `cfg`-selected, so a binary only ever carries its own platform's steps.
/// An indented line is a command to run or a URL to open — the CLI dims those
/// to keep the prose in front; the GUI joins the lot with newlines and adds
/// its own "Click Connect" close.
pub fn transport_setup_hint() -> &'static [&'static str] {
    SETUP_HINT
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
    /// as a gap in the middle of the hint.
    #[test]
    fn setup_hint_is_printable_on_every_platform() {
        let hint = transport_setup_hint();
        assert_eq!(hint.first(), Some(&CABLE_CHECK));
        assert!(hint.iter().all(|line| !line.trim().is_empty()));
    }

    /// The hint covers whichever links the build can open, and never promises
    /// a Bluetooth adapter a feature-off build cannot reach.
    #[test]
    fn setup_hint_covers_bluetooth_when_the_build_does() {
        let mentions = transport_setup_hint()
            .iter()
            .any(|line| line.contains("UT-D07B"));
        assert_eq!(mentions, cfg!(feature = "bluetooth"));
    }

    /// The user plugged in a cable or switched an adapter on; either way the
    /// bridge chip must stay out of what they read.
    #[test]
    fn link_name_says_cable_or_adapter_never_the_chip() {
        assert_eq!(link_name(crate::BLUETOOTH), "Bluetooth adapter");
        for bridge in ["CP2110", "CH9329", "CH9325"] {
            assert_eq!(link_name(bridge), "USB cable");
        }
    }

    /// The short form keeps the same two links and answers `None` for a
    /// transport with nothing on the far end, such as the mock's.
    #[test]
    fn short_link_name_drops_the_adapter_and_the_link_that_isnt_one() {
        use crate::transport::Transport;
        assert_eq!(short_link_name(crate::BLUETOOTH), Some("Bluetooth"));
        for bridge in ["CP2110", "CH9329", "CH9325"] {
            assert_eq!(short_link_name(bridge), Some("USB cable"));
        }
        assert_eq!(
            short_link_name(crate::transport::NullTransport.transport_name()),
            None
        );
    }

    /// Both forms of a link have to name the same thing: the short one goes
    /// on a status line, the full one in the hover that spells it out.
    #[test]
    fn the_full_form_of_a_short_link_name_is_the_one_the_errors_use() {
        for bridge in [crate::BLUETOOTH, "CP2110", "CH9329", "CH9325"] {
            let short = short_link_name(bridge).expect("a link");
            assert_eq!(full_link_name(short), link_name(bridge));
        }
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
