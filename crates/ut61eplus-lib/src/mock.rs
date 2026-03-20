use crate::Dmm;
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::{NullTransport, Transport};
use std::borrow::Cow;
use std::time::Instant;

const MOCK_COMMANDS: &[&str] = &[
    "hold",
    "minmax",
    "exit_minmax",
    "range",
    "auto",
    "rel",
    "select2",
    "select",
    "light",
    "peak",
    "exit_peak",
];

/// Short identifier for a mock scenario, usable from CLI `--mock-mode` and GUI selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockMode {
    DcV,
    AcV,
    Ohm,
    Capacitance,
    Hz,
    Temp,
    DcMa,
    OhmOl,
    Ncv,
}

impl MockMode {
    /// All available modes in scenario order.
    pub const ALL: &[MockMode] = &[
        MockMode::DcV,
        MockMode::AcV,
        MockMode::Ohm,
        MockMode::Capacitance,
        MockMode::Hz,
        MockMode::Temp,
        MockMode::DcMa,
        MockMode::OhmOl,
        MockMode::Ncv,
    ];

    /// Short string label for CLI and display.
    pub fn label(self) -> &'static str {
        match self {
            MockMode::DcV => "dcv",
            MockMode::AcV => "acv",
            MockMode::Ohm => "ohm",
            MockMode::Capacitance => "cap",
            MockMode::Hz => "hz",
            MockMode::Temp => "temp",
            MockMode::DcMa => "dcma",
            MockMode::OhmOl => "ohm-ol",
            MockMode::Ncv => "ncv",
        }
    }

    /// Human-readable description.
    pub fn description(self) -> &'static str {
        match self {
            MockMode::DcV => "DC Voltage (sine wave around 5V)",
            MockMode::AcV => "AC Voltage (sine wave around 120V)",
            MockMode::Ohm => "Resistance (step 1-10 kΩ)",
            MockMode::Capacitance => "Capacitance (ramp 1-20 µF)",
            MockMode::Hz => "Frequency (sine wave around 60Hz)",
            MockMode::Temp => "Temperature (ramp 20-30°C)",
            MockMode::DcMa => "DC mA (sine wave around 50mA)",
            MockMode::OhmOl => "Resistance overload (OL)",
            MockMode::Ncv => "NCV (cycling levels 0-4)",
        }
    }
}

impl std::str::FromStr for MockMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dcv" | "dc-v" | "dc_v" => Ok(MockMode::DcV),
            "acv" | "ac-v" | "ac_v" => Ok(MockMode::AcV),
            "ohm" | "ohms" | "resistance" => Ok(MockMode::Ohm),
            "cap" | "capacitance" => Ok(MockMode::Capacitance),
            "hz" | "freq" | "frequency" => Ok(MockMode::Hz),
            "temp" | "temperature" => Ok(MockMode::Temp),
            "dcma" | "dc-ma" | "dc_ma" | "ma" => Ok(MockMode::DcMa),
            "ohm-ol" | "ohm_ol" | "ol" | "overload" => Ok(MockMode::OhmOl),
            "ncv" => Ok(MockMode::Ncv),
            _ => {
                let valid: Vec<&str> = MockMode::ALL.iter().map(|m| m.label()).collect();
                Err(format!(
                    "unknown mock mode: {s}\nValid modes: {}",
                    valid.join(", ")
                ))
            }
        }
    }
}

impl std::fmt::Display for MockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A scenario defines a measurement mode with a time-varying value pattern.
struct Scenario {
    id: MockMode,
    mode: &'static str,
    mode_raw: u16,
    range_raw: u8,
    unit: &'static str,
    range_label: &'static str,
    range_max: f64,
    samples: u64,
    value_fn: fn(u64) -> MeasuredValue,
}

fn dcv_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(5.0 + 3.0 * (step as f64 / 15.0).sin())
}

fn acv_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(120.0 + 2.0 * (step as f64 / 10.0).sin())
}

fn ohm_value(step: u64) -> MeasuredValue {
    let half = 50;
    let v = if step < half {
        1.0 + (step as f64 / half as f64) * 9.0
    } else {
        10.0 - ((step - half) as f64 / half as f64) * 9.0
    };
    MeasuredValue::Normal(v)
}

fn cap_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(1.0 + (step as f64 / 100.0) * 19.0)
}

fn hz_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(60.0 + 0.5 * (step as f64 / 20.0).sin())
}

fn temp_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(20.0 + (step as f64 / 80.0) * 10.0)
}

fn dcma_value(step: u64) -> MeasuredValue {
    MeasuredValue::Normal(50.0 + 5.0 * (step as f64 / 8.0).sin())
}

fn ohm_ol_value(_step: u64) -> MeasuredValue {
    MeasuredValue::Overload
}

fn ncv_value(step: u64) -> MeasuredValue {
    const LEVELS: [u8; 8] = [0, 1, 2, 3, 4, 3, 2, 1];
    let idx = (step as usize / 5) % LEVELS.len();
    MeasuredValue::NcvLevel(LEVELS[idx])
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: MockMode::DcV,
            mode: "DC V",
            mode_raw: 0x02,
            range_raw: 1, // 22V range
            unit: "V",
            range_label: "22V",
            range_max: 22.0,
            samples: 100,
            value_fn: dcv_value,
        },
        Scenario {
            id: MockMode::AcV,
            mode: "AC V",
            mode_raw: 0x00,
            range_raw: 2, // 220V range
            unit: "V",
            range_label: "220V",
            range_max: 220.0,
            samples: 100,
            value_fn: acv_value,
        },
        Scenario {
            id: MockMode::Ohm,
            mode: "\u{03A9}",
            mode_raw: 0x06,
            range_raw: 2, // 22kΩ range
            unit: "k\u{03A9}",
            range_label: "22k\u{03A9}",
            range_max: 22.0,
            samples: 100,
            value_fn: ohm_value,
        },
        Scenario {
            id: MockMode::Capacitance,
            mode: "Capacitance",
            mode_raw: 0x09,
            range_raw: 3, // 22µF range
            unit: "\u{00B5}F",
            range_label: "22\u{00B5}F",
            range_max: 22.0,
            samples: 100,
            value_fn: cap_value,
        },
        Scenario {
            id: MockMode::Hz,
            mode: "Hz",
            mode_raw: 0x04,
            range_raw: 1, // 220Hz range
            unit: "Hz",
            range_label: "220Hz",
            range_max: 220.0,
            samples: 80,
            value_fn: hz_value,
        },
        Scenario {
            id: MockMode::Temp,
            mode: "Temp \u{00B0}C",
            mode_raw: 0x0A,
            range_raw: 0,
            unit: "\u{00B0}C",
            range_label: "",
            range_max: 400.0,
            samples: 80,
            value_fn: temp_value,
        },
        Scenario {
            id: MockMode::DcMa,
            mode: "DC mA",
            mode_raw: 0x0E,
            range_raw: 1, // 220mA range
            unit: "mA",
            range_label: "220mA",
            range_max: 220.0,
            samples: 80,
            value_fn: dcma_value,
        },
        Scenario {
            id: MockMode::OhmOl,
            mode: "\u{03A9}",
            mode_raw: 0x06,
            range_raw: 5, // 22MΩ range
            unit: "M\u{03A9}",
            range_label: "22M\u{03A9}",
            range_max: 22.0,
            samples: 20,
            value_fn: ohm_ol_value,
        },
        Scenario {
            id: MockMode::Ncv,
            mode: "NCV",
            mode_raw: 0x14,
            range_raw: 0,
            unit: "",
            range_label: "",
            range_max: 4.0,
            samples: 40,
            value_fn: ncv_value,
        },
    ]
}

/// Mock protocol that generates synthetic measurements without hardware.
pub struct MockProtocol {
    scenarios: Vec<Scenario>,
    current_scenario: usize,
    step: u64,
    /// When false, stays on the current scenario indefinitely (step counter still resets).
    auto_cycle: bool,
    hold: bool,
    held_value: Option<MeasuredValue>,
    rel: bool,
    rel_base: Option<f64>,
    auto_range: bool,
    min_flag: bool,
    max_flag: bool,
    peak_min: bool,
    peak_max: bool,
    profile: DeviceProfile,
}

impl MockProtocol {
    /// Create a mock protocol that auto-cycles through all scenarios.
    pub fn new() -> Self {
        Self {
            scenarios: scenarios(),
            current_scenario: 0,
            step: 0,
            auto_cycle: true,
            hold: false,
            held_value: None,
            rel: false,
            rel_base: None,
            auto_range: true,
            min_flag: false,
            max_flag: false,
            peak_min: false,
            peak_max: false,
            profile: DeviceProfile {
                family_name: "mock",
                model_name: "Mock UT61E+",
                // Verified so the GUI doesn't show the EXPERIMENTAL badge —
                // mock behavior is deterministic and needs no hardware validation.
                stability: Stability::Verified,
                supported_commands: MOCK_COMMANDS,
            },
        }
    }

    /// Create a mock protocol pinned to a specific mode.
    /// The mode repeats indefinitely; use `select`/`select2` commands to switch manually.
    pub fn with_mode(mode: MockMode) -> Self {
        let all = scenarios();
        let idx = all
            .iter()
            .position(|s| s.id == mode)
            .expect("MockMode must have a matching scenario");
        let mut proto = Self::new();
        proto.current_scenario = idx;
        proto.auto_cycle = false;
        proto
    }

    /// Return the current scenario's `MockMode`.
    pub fn current_mode(&self) -> MockMode {
        self.scenarios[self.current_scenario].id
    }

    fn current_scenario(&self) -> &Scenario {
        &self.scenarios[self.current_scenario]
    }

    fn advance_scenario(&mut self) {
        self.current_scenario = (self.current_scenario + 1) % self.scenarios.len();
        self.step = 0;
    }

    /// Format a value as a 7-char right-justified display string matching real meter format.
    fn format_display(value: &MeasuredValue) -> Option<String> {
        match value {
            MeasuredValue::Normal(v) => {
                // Format with enough decimals to fill 7 chars
                let abs = v.abs();
                let decimals = if abs >= 100.0 {
                    2
                } else if abs >= 10.0 {
                    3
                } else {
                    4
                };
                let mut s = format!("{v:>7.*}", decimals);
                // Clamp to exactly 7 chars — real meter display is always 7
                s.truncate(7);
                Some(s)
            }
            MeasuredValue::Overload => Some("    OL ".to_string()),
            MeasuredValue::NcvLevel(_) => None,
        }
    }

    /// Compute progress bar value (0-800) proportional to value/range_max.
    fn compute_progress(value: &MeasuredValue, range_max: f64) -> Option<u16> {
        match value {
            MeasuredValue::Normal(v) => {
                let ratio = v.abs() / range_max;
                Some((ratio * 800.0).min(800.0) as u16)
            }
            MeasuredValue::Overload => Some(800),
            MeasuredValue::NcvLevel(_) => None,
        }
    }
}

impl Default for MockProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Protocol for MockProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        Ok(())
    }

    fn request_measurement(&mut self, _transport: &dyn Transport) -> Result<Measurement> {
        let scenario = self.current_scenario();
        let raw_value = (scenario.value_fn)(self.step);

        // Apply hold: freeze the value
        let value = if self.hold {
            self.held_value.clone().unwrap_or(raw_value.clone())
        } else {
            raw_value.clone()
        };

        // Apply rel: subtract baseline
        let value = if self.rel {
            if let (MeasuredValue::Normal(v), Some(base)) = (&value, self.rel_base) {
                MeasuredValue::Normal(v - base)
            } else {
                value
            }
        } else {
            value
        };

        let mode: Cow<'static, str> = Cow::Borrowed(scenario.mode);
        let mode_raw = scenario.mode_raw;
        let range_raw = scenario.range_raw;
        let unit: Cow<'static, str> = Cow::Borrowed(scenario.unit);
        let range_label: Cow<'static, str> = Cow::Borrowed(scenario.range_label);
        let range_max = scenario.range_max;
        let samples = scenario.samples;

        let display_raw = Self::format_display(&value);
        let progress = Self::compute_progress(&value, range_max);

        let flags = StatusFlags {
            hold: self.hold,
            rel: self.rel,
            auto_range: self.auto_range,
            min: self.min_flag,
            max: self.max_flag,
            peak_min: self.peak_min,
            peak_max: self.peak_max,
            ..Default::default()
        };

        let measurement = Measurement {
            timestamp: Instant::now(),
            mode,
            mode_raw,
            range_raw,
            value,
            unit,
            range_label,
            progress,
            display_raw,
            flags,
            raw_payload: vec![],
        };

        self.step += 1;
        if self.step >= samples {
            if self.auto_cycle {
                self.advance_scenario();
            } else {
                self.step = 0; // loop the pattern without changing mode
            }
        }

        Ok(measurement)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        match command {
            "hold" => {
                self.hold = !self.hold;
                if self.hold {
                    let scenario = self.current_scenario();
                    self.held_value = Some((scenario.value_fn)(self.step));
                } else {
                    self.held_value = None;
                }
            }
            "rel" => {
                self.rel = !self.rel;
                if self.rel {
                    let scenario = self.current_scenario();
                    if let MeasuredValue::Normal(v) = (scenario.value_fn)(self.step) {
                        self.rel_base = Some(v);
                    }
                } else {
                    self.rel_base = None;
                }
            }
            "range" => {
                self.auto_range = false;
            }
            "auto" => {
                self.auto_range = true;
            }
            "minmax" => {
                self.min_flag = true;
                self.max_flag = true;
            }
            "exit_minmax" => {
                self.min_flag = false;
                self.max_flag = false;
            }
            "peak" => {
                self.peak_min = true;
                self.peak_max = true;
            }
            "exit_peak" => {
                self.peak_min = false;
                self.peak_max = false;
            }
            "select" | "select2" => {
                self.advance_scenario();
            }
            "light" => { /* no-op */ }
            _ => return Err(Error::UnsupportedCommand(command.to_string())),
        }
        Ok(())
    }

    fn get_name(&mut self, _transport: &dyn Transport) -> Result<Option<String>> {
        Ok(Some("Mock UT61E+".to_string()))
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }
}

/// Create a mock Dmm instance that auto-cycles through all scenarios.
pub fn open_mock() -> Result<Dmm<NullTransport>> {
    Dmm::new(NullTransport, Box::new(MockProtocol::new()))
}

/// Create a mock Dmm instance pinned to a specific mode.
pub fn open_mock_mode(mode: MockMode) -> Result<Dmm<NullTransport>> {
    Dmm::new(NullTransport, Box::new(MockProtocol::with_mode(mode)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_produces_measurements() {
        let mut dmm = open_mock().unwrap();
        for _ in 0..5 {
            let m = dmm.request_measurement().unwrap();
            assert_eq!(m.mode, "DC V");
            assert_eq!(m.unit, "V");
            assert!(m.flags.auto_range);
        }
    }

    #[test]
    fn test_mode_cycling() {
        let mut dmm = open_mock().unwrap();
        let first_mode = dmm.request_measurement().unwrap().mode.clone();
        // Run through enough samples to change scenario (first scenario = 100 samples)
        for _ in 0..100 {
            let _ = dmm.request_measurement().unwrap();
        }
        let new_mode = dmm.request_measurement().unwrap().mode;
        assert_ne!(first_mode, new_mode);
    }

    #[test]
    fn test_hold_command() {
        let mut dmm = open_mock().unwrap();
        let m1 = dmm.request_measurement().unwrap();
        dmm.send_command("hold").unwrap();
        // Read several more — value should be frozen
        let m2 = dmm.request_measurement().unwrap();
        let m3 = dmm.request_measurement().unwrap();
        assert!(m2.flags.hold);
        assert!(m3.flags.hold);
        // Value should be the same
        if let (MeasuredValue::Normal(v2), MeasuredValue::Normal(v3)) = (&m2.value, &m3.value) {
            assert!((v2 - v3).abs() < 1e-10, "held values should be identical");
        }
        // Turn hold off
        dmm.send_command("hold").unwrap();
        let m4 = dmm.request_measurement().unwrap();
        assert!(!m4.flags.hold);
        let _ = m1; // used to establish initial state
    }

    #[test]
    fn test_rel_command() {
        let mut dmm = open_mock().unwrap();
        let _ = dmm.request_measurement().unwrap();
        dmm.send_command("rel").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.rel);
        // REL should produce a delta (value should be smaller than raw)
        if let MeasuredValue::Normal(v) = &m.value {
            // The delta should be close to zero for the first reading after baseline
            assert!(v.abs() < 10.0, "rel delta should be reasonable");
        }
        dmm.send_command("rel").unwrap();
        let m2 = dmm.request_measurement().unwrap();
        assert!(!m2.flags.rel);
    }

    #[test]
    fn test_select_advances() {
        let mut dmm = open_mock().unwrap();
        let m1 = dmm.request_measurement().unwrap();
        assert_eq!(m1.mode, "DC V");
        dmm.send_command("select").unwrap();
        let m2 = dmm.request_measurement().unwrap();
        assert_eq!(m2.mode, "AC V");
    }

    #[test]
    fn test_unsupported_command() {
        let mut dmm = open_mock().unwrap();
        let result = dmm.send_command("nonexistent");
        assert!(matches!(result, Err(Error::UnsupportedCommand(_))));
    }

    #[test]
    fn test_get_name() {
        let mut dmm = open_mock().unwrap();
        let name = dmm.get_name().unwrap();
        assert_eq!(name, Some("Mock UT61E+".to_string()));
    }

    #[test]
    fn test_profile() {
        let dmm = open_mock().unwrap();
        let profile = dmm.profile();
        assert_eq!(profile.family_name, "mock");
        assert_eq!(profile.model_name, "Mock UT61E+");
        assert_eq!(profile.stability, Stability::Verified);
        assert!(profile.supported_commands.contains(&"hold"));
        assert!(profile.supported_commands.contains(&"rel"));
        assert!(profile.supported_commands.contains(&"range"));
    }

    #[test]
    fn test_open_mock() {
        let mut dmm = open_mock().unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.mode.is_empty());
    }

    #[test]
    fn test_display_raw_format() {
        let mut dmm = open_mock().unwrap();
        let m = dmm.request_measurement().unwrap();
        // DC V scenario should have a display_raw
        let display = m.display_raw.as_ref().unwrap();
        assert_eq!(display.len(), 7, "display_raw should be 7 chars");
    }

    #[test]
    fn test_overload_display() {
        let display = MockProtocol::format_display(&MeasuredValue::Overload);
        assert_eq!(display, Some("    OL ".to_string()));
    }

    #[test]
    fn test_ncv_no_display() {
        let display = MockProtocol::format_display(&MeasuredValue::NcvLevel(3));
        assert!(display.is_none());
    }

    #[test]
    fn test_minmax_flags() {
        let mut dmm = open_mock().unwrap();
        dmm.send_command("minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.min);
        assert!(m.flags.max);
        dmm.send_command("exit_minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.min);
        assert!(!m.flags.max);
    }

    #[test]
    fn test_with_mode_pins_scenario() {
        let mut dmm = open_mock_mode(MockMode::Hz).unwrap();
        // Should start in Hz mode
        let m1 = dmm.request_measurement().unwrap();
        assert_eq!(m1.mode, "Hz");
        // After many readings, should still be Hz (no auto-cycle)
        for _ in 0..100 {
            let _ = dmm.request_measurement().unwrap();
        }
        let m2 = dmm.request_measurement().unwrap();
        assert_eq!(m2.mode, "Hz");
    }

    #[test]
    fn test_with_mode_select_still_advances() {
        let mut dmm = open_mock_mode(MockMode::DcV).unwrap();
        let m1 = dmm.request_measurement().unwrap();
        assert_eq!(m1.mode, "DC V");
        // Manual select should still advance
        dmm.send_command("select").unwrap();
        let m2 = dmm.request_measurement().unwrap();
        assert_eq!(m2.mode, "AC V");
    }

    #[test]
    fn test_mock_mode_from_str() {
        assert_eq!("dcv".parse::<MockMode>().unwrap(), MockMode::DcV);
        assert_eq!("ohm-ol".parse::<MockMode>().unwrap(), MockMode::OhmOl);
        assert_eq!("temp".parse::<MockMode>().unwrap(), MockMode::Temp);
        assert_eq!("ncv".parse::<MockMode>().unwrap(), MockMode::Ncv);
        assert!("invalid".parse::<MockMode>().is_err());
    }

    #[test]
    fn test_mock_mode_label_roundtrip() {
        for mode in MockMode::ALL {
            let label = mode.label();
            let parsed: MockMode = label.parse().unwrap();
            assert_eq!(*mode, parsed);
        }
    }

    #[test]
    fn test_open_mock_mode_all_modes() {
        for mode in MockMode::ALL {
            let mut dmm = open_mock_mode(*mode).unwrap();
            let m = dmm.request_measurement().unwrap();
            assert!(
                !m.mode.is_empty(),
                "mode {:?} produced empty mode string",
                mode
            );
        }
    }

    #[test]
    fn test_peak_flags() {
        let mut dmm = open_mock().unwrap();
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.peak_min);
        assert!(m.flags.peak_max);
        dmm.send_command("exit_peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.peak_min);
        assert!(!m.flags.peak_max);
    }

    #[test]
    fn test_commands_match_profile() {
        // Every command in MOCK_COMMANDS must be accepted by send_command,
        // and every accepted command should be listed in MOCK_COMMANDS.
        let mut proto = MockProtocol::new();
        let transport = NullTransport;
        for &cmd in MOCK_COMMANDS {
            assert!(
                proto.send_command(&transport, cmd).is_ok(),
                "MOCK_COMMANDS lists '{cmd}' but send_command rejects it"
            );
        }
        // Verify unlisted commands are rejected
        assert!(proto.send_command(&transport, "nonexistent").is_err());
    }
}
