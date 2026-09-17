use crate::theme::ThemeColors;
use dmm_shared::SharedSettings;
use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Samples the graph history and the sample buffer each keep by default: ~14
/// hours at 10 Hz, which is as long as most bench sessions run.
pub(crate) const DEFAULT_MAX_SAMPLES: usize = 500_000;

/// Floor for [`Settings::max_samples`]. Below a few thousand points the graph
/// evicts faster than the user can look away, so a hand-edited zero would
/// leave the app looking broken rather than frugal.
pub(crate) const MIN_MAX_SAMPLES: usize = 1_000;

/// Ceiling for [`Settings::max_samples`], ten times the largest size the row
/// offers.
///
/// Not a judgement about what is sensible — a hand-edited file is welcome to
/// ask for far more than the chips do. It is there so a slipped digit or a
/// corrupted file asks for a buffer some machine could hold, rather than one
/// that fills memory until the process is killed.
pub(crate) const MAX_MAX_SAMPLES: usize = 50_000_000;

fn default_max_samples() -> usize {
    DEFAULT_MAX_SAMPLES
}

/// Bytes one point costs in the graph's history: a `DataPoint` plus the
/// `VecDeque` slack it sits in.
const GRAPH_BYTES_PER_POINT: usize = 48;
/// Bytes one point costs per sub-value trace drawn beside the plotted one
/// (`Option<f64>`, kept in lockstep with the history).
const GRAPH_BYTES_PER_OVERLAY_POINT: usize = 16;
/// Bytes one buffered sample costs for the meter's main reading: about 240
/// inline, plus its `display_raw` heap string and the meter's own frame — 14
/// to 57 bytes across the families, kept so a recording can be exported as a
/// replay file.
const RECORDING_BYTES_PER_SAMPLE: usize = 340;
/// Bytes each sub-value adds to a buffered sample: an `AuxValue`'s two `Cow`
/// strings, its own `display_raw` and the value.
const RECORDING_BYTES_PER_AUX: usize = 140;

/// `n` as the settings row writes it: "500K", "1.5M", "2M".
pub(crate) fn format_sample_count(n: usize) -> String {
    let (scaled, suffix) = if n >= 1_000_000 {
        (n as f64 / 1_000_000.0, "M")
    } else if n >= 1_000 {
        (n as f64 / 1_000.0, "K")
    } else {
        return n.to_string();
    };
    let s = format!("{scaled:.1}");
    format!("{}{suffix}", s.strip_suffix(".0").unwrap_or(&s))
}

/// What a bound of `n` samples costs in memory, for a meter currently sending
/// `aux` sub-values with `overlays` of them drawn beside the plotted series.
///
/// Both copies of the stream are counted, and both are always kept: the
/// graph's history, and the sample buffer Export… saves — the graph's samples
/// in full, or a recording. The sample buffer dominates, and it grows with
/// the meter: a UT181A's four sub-values roughly triple its per-sample cost.
///
/// Decimal MB, matching the figures quoted in the docs.
pub(crate) fn buffer_memory_estimate(n: usize, overlays: usize, aux: usize) -> String {
    let per_point = GRAPH_BYTES_PER_POINT + overlays * GRAPH_BYTES_PER_OVERLAY_POINT;
    let per_sample = RECORDING_BYTES_PER_SAMPLE + aux * RECORDING_BYTES_PER_AUX;
    let bytes = n.saturating_mul(per_point + per_sample);
    format!("\u{2248}{} MB", bytes.div_ceil(1_000_000))
}

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
        let Some(hex) = s.strip_prefix('#') else {
            return Err(serde::de::Error::custom("hex color must start with '#'"));
        };
        // The digits are sliced by byte index below, so reject anything that is
        // not an ASCII hex digit first: a multi-byte character would otherwise
        // be split mid-codepoint and panic instead of erroring. This also
        // rejects the leading `+` that `from_str_radix` would accept.
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(serde::de::Error::custom(
                "hex color must contain only hex digits",
            ));
        }
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
    pub weak_text: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<HexColor>,
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
    pub graph_overlay_1: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_overlay_2: Option<HexColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_overlay_3: Option<HexColor>,
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
    /// Schema shared with `dmm-cli` via the `dmm-shared` crate.
    /// Flattened so fields (currently just `device_family`) appear at the
    /// top level of the JSON file, preserving the existing on-disk shape.
    #[serde(flatten)]
    pub shared: SharedSettings,
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
    /// Samples to keep, bounding the graph history and the sample buffer
    /// (the samples Export… saves, or a recording) alike —
    /// they hold the same stream, so one number is what the user has to
    /// reason about. Applied live; see `MIN_MAX_SAMPLES` for the floor.
    #[serde(default = "default_max_samples")]
    pub max_samples: usize,
    /// Mock mode to pin to (e.g. "dcv", "acv"). Empty string = auto-cycle.
    /// Only meaningful when device_family is "mock".
    pub mock_mode: String,
    /// Color palette preset.
    #[serde(default)]
    pub color_preset: ColorPreset,
    /// Per-color overrides (dark and light themes independently).
    #[serde(default)]
    pub color_overrides: ColorOverrides,
    /// Last version the user has seen the "What's New" dialog for.
    /// `None` means the user has never dismissed it (new install or pre-feature upgrade).
    #[serde(default)]
    pub last_seen_version: Option<String>,
    /// CLI overrides (not serialized).
    #[serde(skip)]
    pub overrides: Overrides,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shared: SharedSettings {
                // No meter named: a fresh install works out which one is on
                // the cable rather than assuming the one we own.
                device_family: dmm_lib::protocol::registry::AUTO_DEVICE_ID.to_string(),
            },
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
            max_samples: DEFAULT_MAX_SAMPLES,
            mock_mode: String::new(),
            color_preset: ColorPreset::Default,
            color_overrides: ColorOverrides::default(),
            last_seen_version: None,
            overrides: Overrides::default(),
        }
    }
}

impl Settings {
    /// Build the palette for one theme mode from the preset and that mode's
    /// overrides.
    ///
    /// The single place those three fields meet: every widget that needs a
    /// color goes through here, so a preset switch or an override edit can't
    /// reach one part of the UI and miss another.
    pub fn theme_colors(&self, dark: bool) -> ThemeColors {
        ThemeColors::new(dark, self.color_preset, self.color_overrides.for_mode(dark))
    }

    fn config_path() -> Option<PathBuf> {
        dmm_shared::config_path()
    }

    pub fn load() -> Self {
        let mut s: Settings = Self::config_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
        // Through the shared resolver so the GUI and the CLI read the same
        // file the same way. There is no `--device` to weigh here — that
        // override is applied in `App::new` — so this is the settings-then-
        // fallback half. A saved id keeps naming its meter; an older config
        // or a fresh install falls through to detection instead of to a
        // model the user never picked.
        let (family, _) = dmm_shared::resolve_device_family(
            None,
            Some(&s.shared),
            dmm_lib::protocol::registry::AUTO_DEVICE_ID,
        );
        s.shared.device_family = family;
        s.sanitize();
        s
    }

    /// Pull values a hand-edited config file could put out of range back in.
    ///
    /// Separate from `load()` so it is testable without a config file on
    /// disk, and so every future clamp has one place to live.
    fn sanitize(&mut self) {
        self.max_samples = self.max_samples.clamp(MIN_MAX_SAMPLES, MAX_MAX_SAMPLES);
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            // Restore original values for CLI-overridden fields before saving.
            let mut to_save = self.clone();
            if let Some(ref original) = self.overrides.device_family {
                to_save.shared.device_family = original.clone();
            }
            if let Some(ref original) = self.overrides.mock_mode {
                to_save.mock_mode = original.clone();
            }
            if let Some(original) = self.overrides.theme {
                to_save.theme = original;
            }
            if let Ok(json) = serde_json::to_string_pretty(&to_save) {
                // Atomic (.tmp + fsync + rename) so a kill or disk-full
                // mid-write can't corrupt the existing config file.
                if let Err(e) = dmm_shared::write_atomic(&path, json.as_bytes()) {
                    log::warn!("failed to save settings to {}: {e}", path.display());
                }
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

    /// A fresh install names no meter: the device it opens is the one that
    /// answers, not the family whose tables happen to be the fallback.
    #[test]
    fn the_default_device_is_auto_detect() {
        assert_eq!(
            Settings::default().shared.device_family,
            dmm_lib::protocol::registry::AUTO_DEVICE_ID
        );
        // And a config file written before the field existed lands there too,
        // rather than on a model the user never picked.
        let s: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
        let (family, _) = dmm_shared::resolve_device_family(
            None,
            Some(&s.shared),
            dmm_lib::protocol::registry::AUTO_DEVICE_ID,
        );
        assert_eq!(family, dmm_lib::protocol::registry::AUTO_DEVICE_ID);
    }

    #[test]
    fn shared_field_serializes_at_top_level() {
        // Guardrail against accidental #[serde(flatten)] removal: the
        // on-disk JSON must keep `device_family` as a top-level field so
        // dmm-cli's SharedSettings deserialize continues to work.
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            json.contains("\"device_family\":"),
            "device_family must be a top-level field, got: {json}"
        );
        assert!(
            !json.contains("\"shared\":"),
            "shared substruct should be flattened away, got: {json}"
        );
    }

    #[test]
    fn shared_settings_crate_can_read_gui_written_json() {
        // The core of the shared-schema contract: a JSON blob written by
        // the GUI must deserialize cleanly into dmm_shared::SharedSettings
        // with the correct device_family. If the contract drifts, this test
        // fails loudly instead of the CLI silently falling through to the
        // default device.
        let s = Settings {
            shared: SharedSettings {
                device_family: "vc880".to_string(),
            },
            ..Default::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        let shared: dmm_shared::SharedSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(shared.device_family, "vc880");
    }

    #[test]
    fn settings_roundtrip() {
        let s = Settings {
            shared: SharedSettings {
                device_family: "ut8803".to_string(),
            },
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
            max_samples: 2_000_000,
            mock_mode: "dcv".to_string(),
            color_preset: ColorPreset::HighContrast,
            color_overrides: ColorOverrides::default(),
            last_seen_version: Some("0.3.0".to_string()),
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
        assert_eq!(deserialized.max_samples, 2_000_000);
        assert_eq!(deserialized.color_preset, ColorPreset::HighContrast);
        assert_eq!(deserialized.shared.device_family, "ut8803");
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
        // A config file written before the buffer became configurable must
        // land on the default bound, not on zero.
        assert_eq!(s.max_samples, DEFAULT_MAX_SAMPLES);
        // Color fields default correctly
        assert_eq!(s.color_preset, ColorPreset::Default);
        assert_eq!(s.color_overrides, ColorOverrides::default());
        // New optional fields default to None
        assert!(s.last_seen_version.is_none());
    }

    /// The settings row only offers sane sizes, but the file is editable by
    /// hand and a two-digit bound would look like a broken graph.
    #[test]
    fn a_hand_edited_buffer_size_is_floored() {
        let mut s: Settings = serde_json::from_str(r#"{"max_samples":5}"#).unwrap();
        assert_eq!(s.max_samples, 5, "serde takes the file at its word");
        s.sanitize();
        assert_eq!(s.max_samples, MIN_MAX_SAMPLES);
    }

    /// The other end of the same problem: a slipped digit asking for a buffer
    /// no machine holds would fill memory until the process is killed.
    #[test]
    fn a_hand_edited_buffer_size_is_capped() {
        let mut s: Settings = serde_json::from_str(r#"{"max_samples":100000000000}"#).unwrap();
        s.sanitize();
        assert_eq!(s.max_samples, MAX_MAX_SAMPLES);
        // A size beyond the chips but within reason is left alone.
        let mut s: Settings = serde_json::from_str(r#"{"max_samples":8000000}"#).unwrap();
        s.sanitize();
        assert_eq!(s.max_samples, 8_000_000);
    }

    #[test]
    fn sample_counts_read_as_the_chips_label_them() {
        assert_eq!(format_sample_count(999), "999");
        assert_eq!(format_sample_count(1_000), "1K");
        assert_eq!(format_sample_count(100_000), "100K");
        assert_eq!(format_sample_count(500_000), "500K");
        assert_eq!(format_sample_count(1_000_000), "1M");
        assert_eq!(format_sample_count(1_500_000), "1.5M");
        assert_eq!(format_sample_count(5_000_000), "5M");
    }

    /// Both copies of the stream are counted, and the recording's share grows
    /// with the meter's sub-values: 500K × (48 + 340 + 4 × 140) bytes.
    #[test]
    fn memory_estimate_counts_graph_and_recording() {
        assert_eq!(buffer_memory_estimate(500_000, 0, 4), "\u{2248}474 MB");
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

    /// A multi-byte character can make the string six *bytes* long while being
    /// shorter in characters; slicing it by byte index used to panic.
    #[test]
    fn hex_color_non_ascii_errors_instead_of_panicking() {
        for json in [r##""#€345""##, r##""#€€""##, r##""#ÿÿÿÿ""##] {
            let result: Result<HexColor, _> = serde_json::from_str(json);
            assert!(result.is_err(), "{json} should be rejected");
        }
    }

    #[test]
    fn hex_color_rejects_signed_digits() {
        let result: Result<HexColor, _> = serde_json::from_str(r##""#+1+2+3""##);
        assert!(result.is_err());
    }

    /// A bad color value must surface as a deserialization error so `load()`
    /// can fall back to defaults, rather than unwinding out of serde.
    #[test]
    fn settings_with_invalid_color_errors_rather_than_panicking() {
        let json = r##"{"color_overrides":{"dark":{"graph_line":"#€345"}}}"##;
        let parsed: Result<Settings, _> = serde_json::from_str(json);
        assert!(parsed.is_err());
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
