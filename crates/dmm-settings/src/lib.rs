//! Shared settings schema for `dmm-cli` and `dmm-gui`.
//!
//! This crate owns the schema for the fields that BOTH tools need to agree on,
//! so the contract is enforced by the Rust compiler instead of by two files
//! that happen to spell `"device_family"` the same way. GUI-only settings
//! (color overrides, panel visibility, theme, …) live in `dmm-gui` and are
//! merged into the same flat JSON on disk via `#[serde(flatten)]`.
//!
//! The canonical on-disk location is
//! `<XDG_CONFIG_HOME>/dmm-tools/settings.json` on Linux and the equivalent
//! platform-specific path on macOS and Windows (computed via `directories`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Settings fields shared between `dmm-cli` and `dmm-gui`.
///
/// Kept deliberately small. New shared fields go here; GUI-only or CLI-only
/// fields stay in their respective crates and are merged onto this struct via
/// `#[serde(flatten)]` at the call site.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SharedSettings {
    /// Device family ID from the registry (e.g. `"ut61eplus"`, `"ut8803"`).
    /// Empty string means "not set" — consumers should fall back to their own
    /// default (the CLI prints a notice; the GUI fills in from the registry).
    pub device_family: String,
}

/// Return the canonical path to the shared settings file.
///
/// `None` if the platform's config dir is unavailable (rare — weird embedded
/// or sandboxed environments).
pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "dmm-tools")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

impl SharedSettings {
    /// Load just the shared fields from the config file.
    ///
    /// Returns `None` if the file is missing, unreadable, or not valid JSON.
    /// Any GUI-only fields in the JSON are ignored (they don't appear on
    /// `SharedSettings`, and serde drops unknown fields by default). This is
    /// how `dmm-cli` reads the settings file that `dmm-gui` writes: the CLI
    /// sees only the shared slice of the schema.
    pub fn load_if_exists() -> Option<Self> {
        let path = config_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_empty_device_family() {
        let s = SharedSettings::default();
        assert_eq!(s.device_family, "");
    }

    #[test]
    fn deserializes_missing_field_as_default() {
        let s: SharedSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.device_family, "");
    }

    #[test]
    fn deserializes_device_family() {
        let s: SharedSettings = serde_json::from_str(r#"{"device_family":"ut8803"}"#).unwrap();
        assert_eq!(s.device_family, "ut8803");
    }

    #[test]
    fn ignores_unknown_gui_only_fields() {
        // Simulate a settings.json written by dmm-gui with lots of extra fields.
        // The CLI should deserialize just the shared slice without choking.
        let json = r#"{
            "device_family": "ut181a",
            "theme": "Dark",
            "show_graph": true,
            "show_stats": false,
            "color_preset": "HighContrast",
            "zoom_pct": 125
        }"#;
        let s: SharedSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.device_family, "ut181a");
    }

    #[test]
    fn serializes_to_top_level_field() {
        let s = SharedSettings {
            device_family: "vc880".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"device_family":"vc880"}"#);
    }
}
