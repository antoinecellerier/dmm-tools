//! Help and version text shared by the `dmm-cli` and `dmm-gui` binaries.
//!
//! These lived as byte-identical copies in both `main.rs` files. They sit here
//! rather than in either binary because the device list they render comes from
//! [`crate::protocol::registry`], which is the authority on what devices
//! exist — a device added there should reach both binaries' `--help` without
//! anyone remembering to update two lists.
//!
//! The per-crate build values (`CARGO_PKG_VERSION`, `GIT_HASH`) are passed in
//! rather than read here: `env!` would capture *this* crate's values, not the
//! binary's.

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
