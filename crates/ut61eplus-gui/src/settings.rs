use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphTimeWindow {
    Seconds30,
    Minutes1,
    Minutes5,
    Minutes10,
    Hour1,
}

impl GraphTimeWindow {
    pub fn as_secs(&self) -> f64 {
        match self {
            Self::Seconds30 => 30.0,
            Self::Minutes1 => 60.0,
            Self::Minutes5 => 300.0,
            Self::Minutes10 => 600.0,
            Self::Hour1 => 3600.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Seconds30 => "30s",
            Self::Minutes1 => "1m",
            Self::Minutes5 => "5m",
            Self::Minutes10 => "10m",
            Self::Hour1 => "1h",
        }
    }

    pub const ALL: &[Self] = &[
        Self::Seconds30,
        Self::Minutes1,
        Self::Minutes5,
        Self::Minutes10,
        Self::Hour1,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: ThemeMode,
    pub show_graph: bool,
    pub show_stats: bool,
    pub show_recording: bool,
    pub graph_time_window: GraphTimeWindow,
    /// Query device name on connect (causes a beep on the meter).
    pub query_device_name: bool,
    /// Automatically connect to the meter when the GUI starts.
    pub auto_connect: bool,
    /// UI zoom level as percentage (100 = OS default). None = use OS default.
    pub zoom_pct: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            show_graph: true,
            show_stats: true,
            show_recording: true,
            graph_time_window: GraphTimeWindow::Minutes1,
            query_device_name: true,
            auto_connect: true,
            zoom_pct: None, // OS default
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
        assert_eq!(s.graph_time_window, GraphTimeWindow::Minutes1);
    }

    #[test]
    fn settings_roundtrip() {
        let s = Settings {
            theme: ThemeMode::Light,
            show_graph: false,
            show_stats: true,
            show_recording: false,
            graph_time_window: GraphTimeWindow::Minutes5,
            query_device_name: false,
            auto_connect: false,
            zoom_pct: Some(150),
        };
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.theme, ThemeMode::Light);
        assert!(!deserialized.show_graph);
        assert!(deserialized.show_stats);
        assert!(!deserialized.show_recording);
        assert_eq!(deserialized.graph_time_window, GraphTimeWindow::Minutes5);
        assert_eq!(deserialized.zoom_pct, Some(150));
    }

    #[test]
    fn graph_time_window_values() {
        assert_eq!(GraphTimeWindow::Seconds30.as_secs(), 30.0);
        assert_eq!(GraphTimeWindow::Minutes1.as_secs(), 60.0);
        assert_eq!(GraphTimeWindow::Minutes5.as_secs(), 300.0);
        assert_eq!(GraphTimeWindow::Hour1.as_secs(), 3600.0);
    }

    #[test]
    fn graph_time_window_labels() {
        assert_eq!(GraphTimeWindow::Seconds30.label(), "30s");
        assert_eq!(GraphTimeWindow::Hour1.label(), "1h");
    }

    #[test]
    fn settings_deserialize_from_partial_json() {
        // Should fall back to defaults for missing fields
        let json = r#"{"theme":"Light"}"#;
        let result: Result<Settings, _> = serde_json::from_str(json);
        // serde won't fill defaults for missing required fields, so this should fail
        // which means load() correctly falls back to default
        assert!(result.is_err());
    }
}
