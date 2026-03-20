use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub show_graph: bool,
    pub show_stats: bool,
    pub show_recording: bool,
    /// Query device name on connect (causes a beep on the meter).
    pub query_device_name: bool,
    /// Automatically connect to the meter when the GUI starts.
    pub auto_connect: bool,
    /// UI zoom level as percentage relative to OS default (100 = OS default).
    pub zoom_pct: u32,
    /// Delay between measurement requests in milliseconds (0 = fastest possible).
    pub sample_interval_ms: u32,
    /// Device family to connect to (e.g. "ut61eplus", "ut8803", "ut171", "ut181a", "mock").
    pub device_family: String,
    /// Mock mode to pin to (e.g. "dcv", "acv"). Empty string = auto-cycle.
    /// Only meaningful when device_family is "mock".
    pub mock_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            show_graph: true,
            show_stats: true,
            show_recording: true,
            query_device_name: true,
            auto_connect: true,
            zoom_pct: 100,
            sample_interval_ms: 0,
            device_family: ut61eplus_lib::protocol::registry::default_device()
                .id
                .to_string(),
            mock_mode: String::new(),
        }
    }
}

impl Settings {
    fn config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "ut61eplus")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&path, json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert!(s.show_graph);
        assert!(s.show_stats);
        assert!(s.show_recording);
        assert!(s.query_device_name);
        assert_eq!(s.theme, ThemeMode::Dark);
    }

    #[test]
    fn settings_roundtrip() {
        let s = Settings {
            theme: ThemeMode::Light,
            show_graph: false,
            show_stats: true,
            show_recording: false,
            query_device_name: false,
            auto_connect: false,
            zoom_pct: 150,
            sample_interval_ms: 500,
            device_family: "ut8803".to_string(),
            mock_mode: "dcv".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.theme, ThemeMode::Light);
        assert!(!deserialized.show_graph);
        assert!(deserialized.show_stats);
        assert!(!deserialized.show_recording);
        assert_eq!(deserialized.zoom_pct, 150);
        assert_eq!(deserialized.sample_interval_ms, 500);
    }

    #[test]
    fn settings_deserialize_from_partial_json() {
        // Missing fields should get defaults via #[serde(default)]
        let json = r#"{"theme":"Light"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.theme, ThemeMode::Light);
        // All other fields should have default values
        assert!(s.show_graph);
        assert!(s.auto_connect);
        assert_eq!(s.zoom_pct, 100);
        assert_eq!(s.sample_interval_ms, 0);
    }
}
