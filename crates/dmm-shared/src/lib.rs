//! What `dmm-cli` and `dmm-gui` must agree on and `dmm-lib` must not carry.
//!
//! The layering is `dmm-lib` ← `dmm-shared` ← `dmm-cli`, `dmm-gui`. The meter
//! library stays self-contained — devices, protocols and transports, and no
//! opinion about config files, platform directories or what an export is
//! called — so the contracts the two binaries share sit here instead, one
//! layer below them, enforced by the Rust compiler rather than by two files
//! that happen to spell `"device_family"` the same way.
//!
//! What belongs here: the on-disk settings contract ([`SharedSettings`],
//! [`config_path`], and [`resolve_device_family`] — the precedence both apply
//! to the file); durable writes ([`write_atomic`], since settings, capture
//! reports and exports are all user data that must survive a crash
//! mid-write); [`export`], the shape and file name of what the two write
//! out, so a reading saved from the GUI and one written by the CLI read the
//! same; and [`logging`], the log levels both start with. The canonical settings location is
//! `<XDG_CONFIG_HOME>/dmm-tools/settings.json` on Linux and the equivalent
//! platform path on macOS and Windows (computed via `directories`).
//!
//! What does not: device, protocol and transport code stays in `dmm-lib`, and
//! anything one binary alone needs stays in that binary — GUI-only settings
//! (color overrides, panel visibility, theme, …) live in `dmm-gui` and merge
//! into the same flat JSON on disk via `#[serde(flatten)]`.

pub mod export;
pub mod logging;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Settings fields shared between `dmm-cli` and `dmm-gui`.
///
/// Kept deliberately small. New shared fields go here; GUI-only or CLI-only
/// fields stay in their respective crates and are merged onto this struct via
/// `#[serde(flatten)]` at the call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SharedSettings {
    /// Device family ID from the registry (e.g. `"ut61eplus"`, `"ut8803"`), or
    /// `"auto"` to ask the binary to work out which meter is on the cable
    /// instead of naming one.
    /// Empty string means "not set" — consumers should fall back to the default
    /// they hand [`resolve_device_family`] (the CLI prints a notice; the GUI
    /// fills the device picker in).
    pub device_family: String,
    /// Whether an open with no cable on it may go on to look for a Bluetooth
    /// adapter. On by default; off, nothing scans the radio and no help offers
    /// it. An address named on `--adapter` is still opened.
    #[serde(default = "bluetooth_default")]
    pub bluetooth: bool,
}

/// Bluetooth probing is on unless it was turned off: a settings file written
/// before the field existed comes from a version that always probed.
fn bluetooth_default() -> bool {
    true
}

impl Default for SharedSettings {
    fn default() -> Self {
        Self {
            device_family: String::new(),
            bluetooth: bluetooth_default(),
        }
    }
}

/// Where the device family a tool ended up with came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSource {
    /// A `--device` flag on the command line.
    Cli,
    /// `device_family` in the shared settings file.
    Settings,
    /// Neither was given, so the caller's default stands.
    Fallback,
}

/// Which device family to use, and where the choice came from: a `--device`
/// flag wins, then `device_family` in the settings file, then `fallback`.
///
/// Here rather than in either binary because both apply the same rule to the
/// same file — the CLI prints a notice when it falls through, the GUI fills the
/// device picker in. `fallback` is passed in because the registry default lives
/// in `dmm-lib`, which this crate deliberately doesn't depend on.
pub fn resolve_device_family(
    cli_device: Option<&str>,
    saved: Option<&SharedSettings>,
    fallback: &str,
) -> (String, DeviceSource) {
    if let Some(device) = cli_device {
        return (device.to_string(), DeviceSource::Cli);
    }
    // Empty means "not set": that is what a file written before the field
    // existed deserializes to, and what the GUI writes when nothing is picked.
    if let Some(family) = saved
        .map(|s| s.device_family.as_str())
        .filter(|f| !f.is_empty())
    {
        return (family.to_string(), DeviceSource::Settings);
    }
    (fallback.to_string(), DeviceSource::Fallback)
}

/// Return the canonical path to the shared settings file.
///
/// `None` if the platform's config dir is unavailable (rare — weird embedded
/// or sandboxed environments).
pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "dmm-tools")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

/// Write `bytes` to `path` so that a crash, kill or full disk mid-write leaves
/// either the old file or the new one, never a torn file: the bytes go to
/// `<path>.tmp` (same directory, same filesystem), are synced, then renamed
/// over `path`.
///
/// The parent directory is created if missing. If anything fails after the
/// temp file was created it is removed on a best-effort basis, so a failed
/// save doesn't litter the directory with `.tmp` files.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let Some(file_name) = path.file_name() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name to write to", path.display()),
        ));
    };

    // Skip the empty parent of a bare file name ("settings.json"), which
    // `create_dir_all` would reject.
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    // `settings.json` -> `settings.json.tmp`: append rather than replace the
    // extension, so a name that already has one keeps it (and two different
    // files in the same directory can't collide on a shared temp name).
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);

    fn write_and_sync(tmp: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = fs::File::create(tmp)?;
        file.write_all(bytes)?;
        // sync_all before the rename: without it the rename can land while the
        // new contents are still only in the page cache, so a power loss would
        // leave an empty or truncated file where the old one used to be.
        file.sync_all()
    }

    match write_and_sync(&tmp, bytes).and_then(|()| fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
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

    /// A private scratch directory for one test, removed on drop so a failing
    /// assertion doesn't leave files behind. Named per-process and per-test so
    /// concurrently running test binaries can't collide.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("dmm-shared-test-{}-{label}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create temp dir");
            TempDir(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn write_atomic_round_trips() {
        let dir = TempDir::new("round-trip");
        let path = dir.path().join("settings.json");

        write_atomic(&path, br#"{"device_family":"ut61eplus"}"#).unwrap();

        assert_eq!(
            fs::read(&path).unwrap(),
            br#"{"device_family":"ut61eplus"}"#
        );
        // The temp file must be gone: the rename consumed it.
        assert!(
            !dir.path().join("settings.json.tmp").exists(),
            "temp file should not survive a successful write"
        );
    }

    #[test]
    fn write_atomic_replaces_an_existing_file() {
        let dir = TempDir::new("replace");
        let path = dir.path().join("report.yaml");

        write_atomic(&path, b"old contents, longer than the new ones").unwrap();
        write_atomic(&path, b"new").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!dir.path().join("report.yaml.tmp").exists());
    }

    #[test]
    fn write_atomic_creates_missing_parent_dirs() {
        let dir = TempDir::new("parents");
        let path = dir.path().join("nested/deeper/settings.json");

        write_atomic(&path, b"{}").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"{}");
    }

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

    /// Bluetooth is probed unless the user turned it off, and a file written
    /// before the field existed came from a version that always probed — so
    /// upgrading must not quietly stop finding adapters.
    #[test]
    fn bluetooth_probing_is_on_by_default() {
        assert!(SharedSettings::default().bluetooth);
        let old: SharedSettings = serde_json::from_str(r#"{"device_family":"ut61eplus"}"#).unwrap();
        assert!(old.bluetooth);
        let off: SharedSettings = serde_json::from_str(r#"{"bluetooth":false}"#).unwrap();
        assert!(!off.bluetooth);
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

    fn saved(family: &str) -> SharedSettings {
        SharedSettings {
            device_family: family.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn the_cli_flag_wins_over_the_settings_file() {
        let s = saved("ut8803");
        assert_eq!(
            resolve_device_family(Some("ut171"), Some(&s), "ut61eplus"),
            ("ut171".to_string(), DeviceSource::Cli)
        );
    }

    #[test]
    fn the_settings_file_wins_over_the_fallback() {
        let s = saved("ut8803");
        assert_eq!(
            resolve_device_family(None, Some(&s), "ut61eplus"),
            ("ut8803".to_string(), DeviceSource::Settings)
        );
    }

    #[test]
    fn a_missing_settings_file_falls_back() {
        assert_eq!(
            resolve_device_family(None, None, "ut61eplus"),
            ("ut61eplus".to_string(), DeviceSource::Fallback)
        );
    }

    /// A file written before `device_family` existed, or by a GUI where the
    /// user never picked one, deserializes to the empty string — which is
    /// "not set", not a device id.
    #[test]
    fn an_empty_device_family_falls_back() {
        let s = saved("");
        assert_eq!(
            resolve_device_family(None, Some(&s), "ut61eplus"),
            ("ut61eplus".to_string(), DeviceSource::Fallback)
        );
    }

    #[test]
    fn the_cli_flag_wins_with_no_settings_file() {
        assert_eq!(
            resolve_device_family(Some("ut171"), None, "ut61eplus"),
            ("ut171".to_string(), DeviceSource::Cli)
        );
    }

    #[test]
    fn serializes_to_top_level_field() {
        let s = SharedSettings {
            device_family: "vc880".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"device_family":"vc880","bluetooth":true}"#);
    }
}
