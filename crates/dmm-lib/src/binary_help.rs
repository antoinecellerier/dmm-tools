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
    for d in registry::DEVICES {
        let stability = (d.new_protocol)().profile().stability;
        let tag = if !d.requires_hardware {
            " (no hardware required)"
        } else if stability == Stability::Experimental {
            " (experimental)"
        } else {
            ""
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

/// Line the platform setup hint opens with, whatever the platform.
const CABLE_CHECK: &str = "Check that the USB cable is plugged in and the meter is powered on.";

#[cfg(target_os = "linux")]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
    "On Linux, ensure the udev rule is installed:",
    "  sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/",
    "  sudo udevadm control --reload-rules",
    "Then replug the cable. On a headless machine, keep a group on the",
    "rule — see the setup guide:",
    "  https://github.com/antoinecellerier/dmm-tools/blob/main/docs/setup.md",
];

#[cfg(target_os = "windows")]
const SETUP_HINT: &[&str] = &[
    CABLE_CHECK,
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
    "On macOS, the cable should be recognized automatically (no driver needed).",
    "If the device is not found, check System Settings > Privacy & Security > Input Monitoring.",
];

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const SETUP_HINT: &[&str] = &[CABLE_CHECK];

/// What to try when no USB cable was found, one line per step.
///
/// `cfg`-selected, so a binary only ever carries its own platform's steps.
/// An indented line is a command to run or a URL to open — the CLI dims those
/// to keep the prose in front; the GUI joins the lot with newlines and adds
/// its own "Click Connect" close.
pub fn transport_setup_hint() -> &'static [&'static str] {
    SETUP_HINT
}

/// The sentence both binaries use to say a protocol is unverified.
///
/// Only the claim is shared: each site appends its own call to action (the
/// CLI a `capture` command, the GUI a link), because they differ in what the
/// user can do next. Four hand-written spellings had drifted apart here.
pub fn experimental_warning(model_name: &str) -> String {
    format!("{model_name} support is experimental (unverified against real hardware).")
}

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

    #[test]
    fn experimental_warning_names_the_model() {
        assert_eq!(
            experimental_warning("UNI-T UT8803"),
            "UNI-T UT8803 support is experimental (unverified against real hardware)."
        );
    }

    /// The mock needs no hardware and several families are experimental; both
    /// tags tell the user what to expect before they plug anything in.
    #[test]
    fn device_help_tags_mock_and_experimental_devices() {
        let help = device_help("x");
        let mock_line = help
            .lines()
            .find(|l| l.trim_start().starts_with("mock"))
            .expect("mock listed");
        assert!(mock_line.contains("(no hardware required)"), "{mock_line}");
        assert!(help.contains("(experimental)"));
    }
}
