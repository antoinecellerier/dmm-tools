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
//! to the file, and [`load_json`], which tells a broken file from a missing
//! one); durable writes ([`write_atomic`], since settings, capture
//! reports and exports are all user data that must survive a crash
//! mid-write); [`export`], the shape and file name of what the two write
//! out, so a reading saved from the GUI and one written by the CLI read the
//! same; [`replay`], what `--replay` opens, a golden fixture included;
//! [`help`], the help and version text both print; [`logging`], the log
//! levels both start with; [`RECONNECT_INTERVAL`], how often both retry
//! a lost link; and [`stat_decimals`], how many decimals both write the
//! statistics and the integral with. The canonical settings location is
//! `<XDG_CONFIG_HOME>/dmm-tools/settings.json` on Linux and the equivalent
//! platform path on macOS and Windows (computed via `directories`, or from
//! `%APPDATA%` when set).
//!
//! What does not: device, protocol and transport code stays in `dmm-lib`, and
//! anything one binary alone needs stays in that binary — GUI-only settings
//! (color overrides, panel visibility, theme, …) live in `dmm-gui` and merge
//! into the same flat JSON on disk via `#[serde(flatten)]`.

// `pub` only on what the binaries can reach; the rest is `pub(crate)`.
#![warn(unreachable_pub)]

pub mod export;
pub mod help;
pub mod input;
pub mod l10n;
pub mod logging;
pub mod replay;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a lost link waits before each attempt to reopen it, in dmm-gui
/// and in `dmm-cli read`.
pub const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// How many decimals the statistics and the integral write a figure of
/// `magnitude` with, in `dmm-cli read`'s summary and dmm-gui's session
/// block: four, as many more as keep four significant digits below 1, so a
/// run carried into a larger prefix (219.87 Ω into MΩ) does not round to
/// nothing, and no more than twelve.
pub fn stat_decimals(magnitude: f64) -> usize {
    if !magnitude.is_finite() || magnitude == 0.0 || magnitude >= 1.0 {
        return 4;
    }
    (3 - magnitude.log10().floor() as i64).clamp(4, 12) as usize
}

/// Settings fields shared between `dmm-cli` and `dmm-gui`.
///
/// Kept deliberately small. New shared fields go here; GUI-only or CLI-only
/// fields stay in their respective crates and are merged onto this struct via
/// `#[serde(flatten)]` at the call site, while `dmm-cli` reads the file
/// straight into this struct and ignores the GUI's fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SharedSettings {
    /// Device family ID from the registry (e.g. `"ut61eplus"`, `"ut8803"`), or
    /// `"auto"` to ask the binary to work out which meter is on the cable
    /// instead of naming one.
    /// Empty string means "not set" — consumers should fall back to the default
    /// they hand [`resolve_device_family`].
    pub device_family: String,
    /// Whether an open with no cable on it may go on to look for a Bluetooth
    /// adapter. On by default; off, nothing scans the radio and no help offers
    /// it. An address named on `--adapter` is still opened.
    #[serde(default = "bluetooth_default")]
    pub bluetooth: bool,
    /// The mock scenario to pin the simulated meter to (e.g. `"dcv"`), as
    /// the GUI's Mock mode row picks it and a `--mock-mode` flag would name
    /// it. Empty means cycle through every scenario.
    #[serde(default)]
    pub mock_mode: String,
    /// The language the apps speak, as a locale id from
    /// [`l10n::languages`] (e.g. `"fr"`). Empty means follow the system; an
    /// id the running build doesn't have is followed the same way
    /// ([`l10n::resolve`]).
    #[serde(default)]
    pub language: String,
}

/// Bluetooth probing is on unless it was turned off: a settings file written
/// before the field existed comes from a version that always probed.
fn bluetooth_default() -> bool {
    true
}

impl SharedSettings {
    /// The mock scenario the file pins, `None` when it pins none. A name the
    /// mock doesn't know — only a hand-edited file has one — is the mock's
    /// error: a first line saying so, then the valid names.
    pub fn mock_mode(&self) -> Result<Option<dmm_lib::mock::MockMode>, String> {
        if self.mock_mode.is_empty() {
            return Ok(None);
        }
        self.mock_mode.parse().map(Some)
    }
}

impl Default for SharedSettings {
    fn default() -> Self {
        Self {
            device_family: String::new(),
            bluetooth: bluetooth_default(),
            mock_mode: String::new(),
            language: String::new(),
        }
    }
}

/// Which device family to use: a `--device` flag wins, then `device_family`
/// in the settings file, then `fallback`.
///
/// Here rather than in either binary because both apply the same rule to the
/// same file. `fallback` is passed in rather than looked up, so the settings
/// half of this crate does not reach into `dmm-lib`'s registry.
pub fn resolve_device_family(
    cli_device: Option<&str>,
    saved: Option<&SharedSettings>,
    fallback: &str,
) -> String {
    if let Some(device) = cli_device {
        return device.to_string();
    }
    // Empty means "not set": that is what a file written before the field
    // existed deserializes to, and what the GUI writes when nothing is picked.
    if let Some(family) = saved
        .map(|s| s.device_family.as_str())
        .filter(|f| !f.is_empty())
    {
        return family.to_string();
    }
    fallback.to_string()
}

/// Return the canonical path to the shared settings file.
///
/// On Windows, under `%APPDATA%` when it is set, as most Windows programs
/// read it: the known-folder API behind [`directories`] gives the same folder
/// but no variable moves it, so the integration tests could not keep a
/// developer's own settings out the way `XDG_CONFIG_HOME` does elsewhere.
///
/// `None` if the platform's config dir is unavailable (rare — weird embedded
/// or sandboxed environments).
pub fn config_path() -> Option<PathBuf> {
    if cfg!(windows)
        && let Some(appdata) = std::env::var_os("APPDATA").filter(|v| !v.is_empty())
    {
        return Some(appdata_config_path(Path::new(&appdata)));
    }
    directories::ProjectDirs::from("", "", "dmm-tools")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

/// The settings file under `appdata`, laid out as [`directories`] lays out
/// the roaming application data folder.
fn appdata_config_path(appdata: &Path) -> PathBuf {
    appdata
        .join("dmm-tools")
        .join("config")
        .join("settings.json")
}

/// Write `bytes` to `path` so that a crash, kill or full disk mid-write leaves
/// either the old file or the new one, never a torn file: the bytes go to
/// `<path>.<pid>.tmp` (same directory, same filesystem), are synced, then
/// renamed over `path`. The pid keeps two processes saving the same file (two
/// dmm-gui windows) off each other's half-written temp file.
///
/// The parent directory is created if missing. If anything fails after the
/// temp file was created it is removed on a best-effort basis, so a failed
/// save doesn't litter the directory with `.tmp` files. A temp file left by a
/// crashed process stays, though: the next write has another pid, so it never
/// looks at that name.
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

    // `settings.json` -> `settings.json.<pid>.tmp`: append rather than replace
    // the extension, so a name that already has one keeps it (and two
    // different files in the same directory can't collide on a shared temp
    // name).
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(tmp_name);

    fn write_and_sync(tmp: &Path, bytes: &[u8]) -> io::Result<()> {
        // A temp file already there is the leftover of a crashed process that
        // had the same pid, or something planted: a link to another file,
        // which `File::create` would follow and truncate. Remove it (the link,
        // not what it points at) and create the temp file exclusively, which
        // never follows a link and fails if one is put back in between.
        let _ = fs::remove_file(tmp);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(tmp)?;
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

/// Why a JSON file that exists could not be loaded: [`load_json`]'s error.
/// Its `Display` is English, naming the file and the reason, for a log line
/// or the end of a warning.
#[derive(Debug)]
pub enum LoadError {
    /// The file is there but could not be read.
    Read { path: PathBuf, source: io::Error },
    /// The file was read but is not valid JSON, or not of the expected shape.
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            LoadError::Parse { path, source } => {
                write!(f, "cannot parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Read `path` as JSON: `Ok(None)` when there is no such file, an error when
/// one is there but can't be read or parsed — so a broken file is reported
/// rather than mistaken for a missing one.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, LoadError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LoadError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|source| LoadError::Parse {
            path: path.to_path_buf(),
            source,
        })
}

impl SharedSettings {
    /// Load just the shared fields from the config file.
    ///
    /// `Ok(None)` if there is no config path or no file there; an error if
    /// the file can't be read or isn't valid JSON, for the caller to report.
    /// Any GUI-only fields in the JSON are ignored (they don't appear on
    /// `SharedSettings`, and serde drops unknown fields by default). This is
    /// how `dmm-cli` reads the settings file that `dmm-gui` writes: the CLI
    /// sees only the shared slice of the schema.
    pub fn load_if_exists() -> Result<Option<Self>, LoadError> {
        match config_path() {
            Some(path) => load_json(&path),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four decimals from 1 up, four significant digits below it: a run
    /// carried from Ω into MΩ keeps its figures.
    #[test]
    fn stat_figures_keep_four_significant_digits_below_one() {
        let at = |v: f64| format!("{v:.*}", stat_decimals(v.abs()));
        assert_eq!(at(1.05), "1.0500");
        assert_eq!(at(219.87), "219.8700");
        assert_eq!(at(0.95), "0.9500");
        assert_eq!(at(0.0293), "0.02930");
        assert_eq!(at(0.00021987), "0.0002199");
        assert_eq!(at(-0.00021987), "-0.0002199");
        assert_eq!(at(0.0), "0.0000");
        // Bounded: a figure that is only rounding noise stays short.
        assert_eq!(at(1e-17), "0.000000000000");
    }

    /// `%APPDATA%` leads to the file `directories` finds, so setting it
    /// changes nothing for anyone whose variable is the system's.
    #[cfg(windows)]
    #[test]
    fn appdata_leads_where_directories_does() {
        let roaming = directories::BaseDirs::new().expect("base dirs");
        let project = directories::ProjectDirs::from("", "", "dmm-tools").expect("project dirs");
        assert_eq!(
            appdata_config_path(roaming.config_dir()),
            project.config_dir().join("settings.json")
        );
    }

    /// The temp file `write_atomic` uses for `file_name` in this process.
    fn temp_name(file_name: &str) -> String {
        format!("{file_name}.{}.tmp", std::process::id())
    }

    #[test]
    fn write_atomic_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        write_atomic(&path, br#"{"device_family":"ut61eplus"}"#).unwrap();

        assert_eq!(
            fs::read(&path).unwrap(),
            br#"{"device_family":"ut61eplus"}"#
        );
        // The temp file must be gone: the rename consumed it.
        assert!(
            !dir.path().join(temp_name("settings.json")).exists(),
            "temp file should not survive a successful write"
        );
    }

    #[test]
    fn write_atomic_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.yaml");

        write_atomic(&path, b"old contents, longer than the new ones").unwrap();
        write_atomic(&path, b"new").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!dir.path().join(temp_name("report.yaml")).exists());
    }

    #[test]
    fn write_atomic_creates_missing_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
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

    /// A file written before the language existed follows the system, and
    /// a saved choice comes back as written.
    #[test]
    fn the_language_follows_the_system_unless_set() {
        let old: SharedSettings = serde_json::from_str(r#"{"device_family":"ut61eplus"}"#).unwrap();
        assert_eq!(old.language, "");
        let set: SharedSettings = serde_json::from_str(r#"{"language":"fr"}"#).unwrap();
        assert_eq!(set.language, "fr");
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
    fn load_json_reads_a_missing_file_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_json::<SharedSettings>(&dir.path().join("settings.json"));
        assert!(matches!(loaded, Ok(None)), "{loaded:?}");
    }

    #[test]
    fn load_json_reads_a_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"device_family":"ut8803","theme":"Dark"}"#).unwrap();
        let loaded = load_json::<SharedSettings>(&path).unwrap();
        assert_eq!(loaded, Some(saved("ut8803")));
    }

    #[test]
    fn load_json_reports_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // Cut short mid-write, and a shared field of the wrong type.
        for contents in [r#"{"device_family":"ut88"#, r#"{"bluetooth":"yes"}"#] {
            fs::write(&path, contents).unwrap();
            let loaded = load_json::<SharedSettings>(&path);
            assert!(
                matches!(&loaded, Err(LoadError::Parse { path: p, .. }) if *p == path),
                "{contents}: {loaded:?}"
            );
            let message = loaded.unwrap_err().to_string();
            assert!(message.contains("settings.json"), "{message}");
        }
    }

    #[test]
    fn load_json_reports_a_file_it_cannot_read() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: there, but not readable as one.
        let loaded = load_json::<SharedSettings>(dir.path());
        assert!(matches!(loaded, Err(LoadError::Read { .. })), "{loaded:?}");
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
            "ut171"
        );
    }

    #[test]
    fn the_settings_file_wins_over_the_fallback() {
        let s = saved("ut8803");
        assert_eq!(resolve_device_family(None, Some(&s), "ut61eplus"), "ut8803");
    }

    #[test]
    fn a_missing_settings_file_falls_back() {
        assert_eq!(resolve_device_family(None, None, "ut61eplus"), "ut61eplus");
    }

    /// A file written before `device_family` existed, or by a GUI where the
    /// user never picked one, deserializes to the empty string — which is
    /// "not set", not a device id.
    #[test]
    fn an_empty_device_family_falls_back() {
        let s = saved("");
        assert_eq!(
            resolve_device_family(None, Some(&s), "ut61eplus"),
            "ut61eplus"
        );
    }

    #[test]
    fn the_cli_flag_wins_with_no_settings_file() {
        assert_eq!(
            resolve_device_family(Some("ut171"), None, "ut61eplus"),
            "ut171"
        );
    }

    /// Empty pins nothing; a label or alias pins that scenario; anything
    /// else is the mock's own error, its first line naming the value.
    #[test]
    fn mock_mode_parses_the_saved_name() {
        let with = |mode: &str| SharedSettings {
            mock_mode: mode.to_string(),
            ..Default::default()
        };
        assert_eq!(with("").mock_mode(), Ok(None));
        assert_eq!(
            with("dcv").mock_mode(),
            Ok(Some(dmm_lib::mock::MockMode::DcV))
        );
        let error = with("bogus").mock_mode().unwrap_err();
        assert_eq!(error.lines().next(), Some("unknown mock mode: bogus"));
    }

    #[test]
    fn serializes_to_top_level_field() {
        let s = SharedSettings {
            device_family: "vc880".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            json,
            r#"{"device_family":"vc880","bluetooth":true,"mock_mode":"","language":""}"#
        );
    }

    /// A `.tmp` link planted beside the file — by a theme pack unpacked into
    /// the themes folder, say — must not have the write follow it into
    /// another file.
    #[cfg(unix)]
    #[test]
    fn a_planted_temp_link_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, b"keep me").unwrap();
        let path = dir.path().join("theme.json");
        std::os::unix::fs::symlink(&victim, dir.path().join(temp_name("theme.json"))).unwrap();

        write_atomic(&path, b"{}").unwrap();

        assert_eq!(fs::read(&victim).unwrap(), b"keep me");
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        assert!(
            !fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
