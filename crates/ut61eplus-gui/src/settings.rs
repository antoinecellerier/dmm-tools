use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorPreset {
    #[default]
    Default,
    HighContrast,
    ColorblindSafe,
}

/// A color that serializes as a hex string (`"#RRGGBB"` or `"#RRGGBBAA"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexColor(pub Color32);

impl serde::Serialize for HexColor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let [r, g, b, a] = self.0.to_array();
        if a == 255 {
            serializer.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}"))
        } else {
            serializer.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}{a:02X}"))
        }
    }
}

impl<'de> serde::Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if !s.starts_with('#') {
            return Err(serde::de::Error::custom("hex color must start with '#'"));
        }
        let hex = &s[1..];
        let parse_byte =
            |slice: &str| u8::from_str_radix(slice, 16).map_err(serde::de::Error::custom);
        match hex.len() {
            6 => {
                let r = parse_byte(&hex[0..2])?;
                let g = parse_byte(&hex[2..4])?;
                let b = parse_byte(&hex[4..6])?;
                Ok(HexColor(Color32::from_rgb(r, g, b)))
            }
            8 => {
                let r = parse_byte(&hex[0..2])?;
                let g = parse_byte(&hex[2..4])?;
                let b = parse_byte(&hex[4..6])?;
                let a = parse_byte(&hex[6..8])?;
                Ok(HexColor(Color32::from_rgba_premultiplied(r, g, b, a)))
            }
            _ => Err(serde::de::Error::custom(
                "hex color must be #RRGGBB or #RRGGBBAA",
            )),
        }
    }
}

/// Per-theme overrides for all customizable colors.
/// Fields that are `None` fall back to the active preset's default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaletteOverrides {
    // -- UI chrome --
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<HexColor>,
    // -- Graph colors --
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_line: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_gap: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_mean: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_ref: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_crossing: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_cursor: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_envelope: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot_background: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_crosshair: Option<HexColor>,
    // -- Status indicators --
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_ok: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_warning: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_error: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_inactive: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<HexColor>,
    // -- Minimap --
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimap_viewport: Option<HexColor>,
}

/// Color overrides split by theme (dark/light). Each theme's overrides
/// are independent — a dark-mode override does not affect light mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorOverrides {
    #[serde(default)]
    pub dark: PaletteOverrides,
    #[serde(default)]
    pub light: PaletteOverrides,
}

impl ColorOverrides {
    pub fn for_mode(&self, dark: bool) -> &PaletteOverrides {
        if dark { &self.dark } else { &self.light }
    }

    pub fn for_mode_mut(&mut self, dark: bool) -> &mut PaletteOverrides {
        if dark {
            &mut self.dark
        } else {
            &mut self.light
        }
    }
}

/// Tracks which settings fields are overridden by CLI arguments.
/// Overridden fields are session-only and not persisted to disk.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// Original persisted value for device_family (if overridden).
    pub device_family: Option<String>,
    /// Original persisted value for mock_mode (if overridden).
    pub mock_mode: Option<String>,
    /// Original persisted value for theme (if overridden).
    pub theme: Option<ThemeMode>,
    /// CLI-specified adapter (serial number or HID path).
    pub adapter: Option<String>,
}

impl Overrides {
    /// Returns true if the given field is CLI-overridden.
    pub fn has_device(&self) -> bool {
        self.device_family.is_some()
    }

    pub fn has_mock_mode(&self) -> bool {
        self.mock_mode.is_some()
    }

    pub fn has_theme(&self) -> bool {
        self.theme.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub show_graph: bool,
    pub show_stats: bool,
    pub show_recording: bool,
    pub show_specs: bool,
    /// Query device name on connect (causes a beep on the meter).
    pub query_device_name: bool,
    /// Automatically connect to the meter when the GUI starts.
    pub auto_connect: bool,
    /// Keep the window above all other windows.
    pub always_on_top: bool,
    /// Hide window decorations (title bar, borders).
    #[serde(default)]
    pub hide_decorations: bool,
    /// UI zoom level as percentage relative to OS default (100 = OS default).
    pub zoom_pct: u32,
    /// Delay between measurement requests in milliseconds (0 = fastest possible).
    pub sample_interval_ms: u32,
    /// Device family to connect to (e.g. "ut61eplus", "ut8803", "ut171", "ut181a", "mock").
    pub device_family: String,
    /// Mock mode to pin to (e.g. "dcv", "acv"). Empty string = auto-cycle.
    /// Only meaningful when device_family is "mock".
    pub mock_mode: String,
    /// Color palette preset.
    #[serde(default)]
    pub color_preset: ColorPreset,
    /// Per-color overrides (dark and light themes independently).
    #[serde(default)]
    pub color_overrides: ColorOverrides,
    /// CLI overrides (not serialized).
    #[serde(skip)]
    pub overrides: Overrides,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            show_graph: true,
            show_stats: true,
            show_recording: true,
            show_specs: true,
            query_device_name: true,
            auto_connect: true,
            always_on_top: false,
            hide_decorations: false,
            zoom_pct: 100,
            sample_interval_ms: 0,
            device_family: ut61eplus_lib::protocol::registry::default_device()
                .id
                .to_string(),
            mock_mode: String::new(),
            color_preset: ColorPreset::Default,
            color_overrides: ColorOverrides::default(),
            overrides: Overrides::default(),
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
            // Restore original values for CLI-overridden fields before saving.
            let mut to_save = self.clone();
            if let Some(ref original) = self.overrides.device_family {
                to_save.device_family = original.clone();
            }
            if let Some(ref original) = self.overrides.mock_mode {
                to_save.mock_mode = original.clone();
            }
            if let Some(original) = self.overrides.theme {
                to_save.theme = original;
            }
            if let Ok(json) = serde_json::to_string_pretty(&to_save) {
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
        assert!(s.show_specs);
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
            show_specs: false,
            query_device_name: false,
            auto_connect: false,
            always_on_top: true,
            hide_decorations: true,
            zoom_pct: 150,
            sample_interval_ms: 500,
            device_family: "ut8803".to_string(),
            mock_mode: "dcv".to_string(),
            color_preset: ColorPreset::HighContrast,
            color_overrides: ColorOverrides::default(),
            overrides: Overrides::default(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.theme, ThemeMode::Light);
        assert!(!deserialized.show_graph);
        assert!(deserialized.show_stats);
        assert!(!deserialized.show_recording);
        assert!(!deserialized.show_specs);
        assert!(deserialized.always_on_top);
        assert!(deserialized.hide_decorations);
        assert_eq!(deserialized.zoom_pct, 150);
        assert_eq!(deserialized.sample_interval_ms, 500);
        assert_eq!(deserialized.color_preset, ColorPreset::HighContrast);
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
        // Color fields default correctly
        assert_eq!(s.color_preset, ColorPreset::Default);
        assert_eq!(s.color_overrides, ColorOverrides::default());
    }

    #[test]
    fn hex_color_roundtrip_rgb() {
        let color = HexColor(Color32::from_rgb(0xFF, 0x88, 0x00));
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, r##""#FF8800""##);
        let parsed: HexColor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, color);
    }

    #[test]
    fn hex_color_roundtrip_rgba() {
        let color = HexColor(Color32::from_rgba_premultiplied(0x64, 0xC8, 0xFF, 0x80));
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, r##""#64C8FF80""##);
        let parsed: HexColor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, color);
    }

    #[test]
    fn hex_color_lowercase_accepted() {
        let parsed: HexColor = serde_json::from_str(r##""#ff8800""##).unwrap();
        assert_eq!(parsed.0, Color32::from_rgb(0xFF, 0x88, 0x00));
    }

    #[test]
    fn hex_color_invalid_no_hash() {
        let result: Result<HexColor, _> = serde_json::from_str(r#""FF8800""#);
        assert!(result.is_err());
    }

    #[test]
    fn hex_color_invalid_length() {
        let result: Result<HexColor, _> = serde_json::from_str(r##""#FFF""##);
        assert!(result.is_err());
    }

    #[test]
    fn color_overrides_json_roundtrip() {
        let mut overrides = ColorOverrides::default();
        overrides.dark.graph_line = Some(HexColor(Color32::from_rgb(100, 200, 255)));
        overrides.light.status_ok = Some(HexColor(Color32::from_rgb(0, 150, 50)));

        let json = serde_json::to_string_pretty(&overrides).unwrap();
        let parsed: ColorOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, overrides);
        assert_eq!(
            parsed.dark.graph_line,
            Some(HexColor(Color32::from_rgb(100, 200, 255)))
        );
        assert!(parsed.dark.graph_gap.is_none());
        assert_eq!(
            parsed.light.status_ok,
            Some(HexColor(Color32::from_rgb(0, 150, 50)))
        );
    }

    #[test]
    fn settings_with_color_preset_json() {
        let json = r#"{"color_preset":"HighContrast"}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.color_preset, ColorPreset::HighContrast);
        assert_eq!(s.color_overrides, ColorOverrides::default());
    }
}
