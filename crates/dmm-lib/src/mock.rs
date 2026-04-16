use crate::Dmm;
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::{NullTransport, Transport};
use std::borrow::Cow;
use std::time::{Duration, Instant};

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
///
/// Values are a pure function of elapsed seconds since the scenario started,
/// so displayed waveforms trace smooth curves regardless of read cadence or
/// scheduling jitter.
struct Scenario {
    id: MockMode,
    mode: &'static str,
    mode_raw: u16,
    range_raw: u8,
    unit: &'static str,
    range_label: &'static str,
    range_max: f64,
    duration_secs: f64,
    value_fn: fn(f64) -> MeasuredValue,
}

impl Scenario {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: MockMode,
        mode: &'static str,
        mode_raw: u16,
        range_raw: u8,
        unit: &'static str,
        range_label: &'static str,
        range_max: f64,
        duration_secs: f64,
        value_fn: fn(f64) -> MeasuredValue,
    ) -> Self {
        Self {
            id,
            mode,
            mode_raw,
            range_raw,
            unit,
            range_label,
            range_max,
            duration_secs,
            value_fn,
        }
    }
}

fn dcv_value(t: f64) -> MeasuredValue {
    // Period ≈ 9.42 s (matches the original 15-read-per-radian waveform at
    // 100 ms/read: step/15 → t*10/15 → t*2/3).
    MeasuredValue::Normal(5.0 + 3.0 * (t * 2.0 / 3.0).sin())
}

fn acv_value(t: f64) -> MeasuredValue {
    // Period ≈ 6.28 s (step/10 at 100 ms → t).
    MeasuredValue::Normal(120.0 + 2.0 * t.sin())
}

fn ohm_value(t: f64) -> MeasuredValue {
    // Triangle 1..10 with a 10-second period.
    let phase = (t / 10.0).rem_euclid(1.0);
    let v = if phase < 0.5 {
        1.0 + phase * 2.0 * 9.0
    } else {
        10.0 - (phase - 0.5) * 2.0 * 9.0
    };
    MeasuredValue::Normal(v)
}

fn cap_value(t: f64) -> MeasuredValue {
    // Sawtooth ramp 1..20 with a 10-second period.
    MeasuredValue::Normal(1.0 + (t / 10.0).rem_euclid(1.0) * 19.0)
}

fn hz_value(t: f64) -> MeasuredValue {
    // Period ≈ 12.57 s (step/20 at 100 ms → t/2).
    MeasuredValue::Normal(60.0 + 0.5 * (t / 2.0).sin())
}

fn temp_value(t: f64) -> MeasuredValue {
    // Sawtooth ramp 20..30 with an 8-second period.
    MeasuredValue::Normal(20.0 + (t / 8.0).rem_euclid(1.0) * 10.0)
}

fn dcma_value(t: f64) -> MeasuredValue {
    // Period ≈ 5.03 s (step/8 at 100 ms → t*1.25).
    MeasuredValue::Normal(50.0 + 5.0 * (t * 1.25).sin())
}

fn ohm_ol_value(_t: f64) -> MeasuredValue {
    MeasuredValue::Overload
}

fn ncv_value(t: f64) -> MeasuredValue {
    const LEVELS: [u8; 8] = [0, 1, 2, 3, 4, 3, 2, 1];
    let idx = ((t / 0.5).floor().rem_euclid(LEVELS.len() as f64)) as usize;
    MeasuredValue::NcvLevel(LEVELS[idx])
}

fn scenarios() -> Vec<Scenario> {
    vec![
        // 22V range
        Scenario::new(
            MockMode::DcV,
            "DC V",
            0x02,
            1,
            "V",
            "22V",
            22.0,
            10.0,
            dcv_value,
        ),
        // 220V range
        Scenario::new(
            MockMode::AcV,
            "AC V",
            0x00,
            2,
            "V",
            "220V",
            220.0,
            10.0,
            acv_value,
        ),
        // 22kΩ range
        Scenario::new(
            MockMode::Ohm,
            "\u{03A9}",
            0x06,
            2,
            "k\u{03A9}",
            "22k\u{03A9}",
            22.0,
            10.0,
            ohm_value,
        ),
        // 22µF range
        Scenario::new(
            MockMode::Capacitance,
            "Capacitance",
            0x09,
            3,
            "\u{00B5}F",
            "22\u{00B5}F",
            22.0,
            10.0,
            cap_value,
        ),
        // 220Hz range
        Scenario::new(
            MockMode::Hz,
            "Hz",
            0x04,
            1,
            "Hz",
            "220Hz",
            220.0,
            8.0,
            hz_value,
        ),
        Scenario::new(
            MockMode::Temp,
            "Temp \u{00B0}C",
            0x0A,
            0,
            "\u{00B0}C",
            "",
            400.0,
            8.0,
            temp_value,
        ),
        // 220mA range
        Scenario::new(
            MockMode::DcMa,
            "DC mA",
            0x0E,
            1,
            "mA",
            "220mA",
            220.0,
            8.0,
            dcma_value,
        ),
        // 22MΩ range
        Scenario::new(
            MockMode::OhmOl,
            "\u{03A9}",
            0x06,
            5,
            "M\u{03A9}",
            "22M\u{03A9}",
            22.0,
            2.0,
            ohm_ol_value,
        ),
        Scenario::new(MockMode::Ncv, "NCV", 0x14, 0, "", "", 4.0, 4.0, ncv_value),
    ]
}

/// MIN/MAX display cycling state, matching real device behavior.
/// The meter cycles MAX → MIN → MAX as a 2-state toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MinMaxState {
    Off,
    Max,
    Min,
}

/// Peak display cycling state, matching real device behavior.
/// The meter cycles P-MAX → P-MIN → P-MAX as a 2-state toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeakState {
    Off,
    Max,
    Min,
}

/// Mock protocol that generates synthetic measurements without hardware.
pub struct MockProtocol {
    scenarios: Vec<Scenario>,
    current_scenario: usize,
    /// Wall-clock instant the current scenario started. Values are evaluated at
    /// `now - scenario_started`, so the waveform is a smooth function of time
    /// regardless of read cadence. On scenario advance, this is reset to `now`.
    pub(crate) scenario_started: Instant,
    /// When false, stays on the current scenario indefinitely.
    auto_cycle: bool,
    hold: bool,
    held_value: Option<MeasuredValue>,
    rel: bool,
    rel_base: Option<f64>,
    auto_range: bool,
    /// Saved auto_range state before MIN/MAX activation (restored on exit).
    auto_range_before_minmax: bool,
    minmax_state: MinMaxState,
    stored_min: Option<f64>,
    stored_max: Option<f64>,
    peak_state: PeakState,
    stored_peak_min: Option<f64>,
    stored_peak_max: Option<f64>,
    profile: DeviceProfile,
}

impl MockProtocol {
    /// Create a mock protocol that auto-cycles through all scenarios.
    pub fn new() -> Self {
        Self {
            scenarios: scenarios(),
            current_scenario: 0,
            scenario_started: Instant::now(),
            auto_cycle: true,
            hold: false,
            held_value: None,
            rel: false,
            rel_base: None,
            auto_range: true,
            auto_range_before_minmax: true,
            minmax_state: MinMaxState::Off,
            stored_min: None,
            stored_max: None,
            peak_state: PeakState::Off,
            stored_peak_min: None,
            stored_peak_max: None,
            profile: DeviceProfile {
                family_name: "mock",
                model_name: "Mock UT61E+",
                // Verified so the GUI doesn't show the EXPERIMENTAL badge —
                // mock behavior is deterministic and needs no hardware validation.
                stability: Stability::Verified,
                supported_commands: MOCK_COMMANDS,
                verification_issue: None,
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
        self.scenario_started = Instant::now();
    }

    /// Elapsed seconds since the current scenario started. Uses
    /// `checked_duration_since` so a backward clock jump returns 0 instead of
    /// panicking.
    fn elapsed_secs(&self) -> f64 {
        Instant::now()
            .checked_duration_since(self.scenario_started)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
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
        // Extract scenario data up front to avoid borrow conflict with &mut self.
        let elapsed = self.elapsed_secs();
        let scenario = &self.scenarios[self.current_scenario];
        let raw_value = (scenario.value_fn)(elapsed);
        let mode: Cow<'static, str> = Cow::Borrowed(scenario.mode);
        let mode_raw = scenario.mode_raw;
        let range_raw = scenario.range_raw;
        let unit: Cow<'static, str> = Cow::Borrowed(scenario.unit);
        let range_label: Cow<'static, str> = Cow::Borrowed(scenario.range_label);
        let range_max = scenario.range_max;
        let duration_secs = scenario.duration_secs;

        // Apply hold: freeze the value
        let live_value = if self.hold {
            self.held_value.clone().unwrap_or(raw_value.clone())
        } else {
            raw_value.clone()
        };

        // Apply rel: subtract baseline
        let live_value = if self.rel {
            if let (MeasuredValue::Normal(v), Some(base)) = (&live_value, self.rel_base) {
                MeasuredValue::Normal(v - base)
            } else {
                live_value
            }
        } else {
            live_value
        };

        // Update stored MIN/MAX values from live reading
        if self.minmax_state != MinMaxState::Off
            && let MeasuredValue::Normal(v) = &live_value
        {
            self.stored_min = Some(match self.stored_min {
                Some(prev) => prev.min(*v),
                None => *v,
            });
            self.stored_max = Some(match self.stored_max {
                Some(prev) => prev.max(*v),
                None => *v,
            });
        }

        // Update stored Peak values from live reading
        if self.peak_state != PeakState::Off
            && let MeasuredValue::Normal(v) = &live_value
        {
            self.stored_peak_min = Some(match self.stored_peak_min {
                Some(prev) => prev.min(*v),
                None => *v,
            });
            self.stored_peak_max = Some(match self.stored_peak_max {
                Some(prev) => prev.max(*v),
                None => *v,
            });
        }

        // Select display value: stored min/max/peak when active, live otherwise.
        // Real device sends the stored value, not the live reading.
        let value = match self.minmax_state {
            MinMaxState::Max => self
                .stored_max
                .map(MeasuredValue::Normal)
                .unwrap_or(live_value.clone()),
            MinMaxState::Min => self
                .stored_min
                .map(MeasuredValue::Normal)
                .unwrap_or(live_value.clone()),
            MinMaxState::Off => match self.peak_state {
                PeakState::Max => self
                    .stored_peak_max
                    .map(MeasuredValue::Normal)
                    .unwrap_or(live_value.clone()),
                PeakState::Min => self
                    .stored_peak_min
                    .map(MeasuredValue::Normal)
                    .unwrap_or(live_value.clone()),
                PeakState::Off => live_value,
            },
        };

        let display_raw = Self::format_display(&value);
        let progress = Self::compute_progress(&value, range_max);

        let flags = StatusFlags {
            hold: self.hold,
            rel: self.rel,
            auto_range: self.auto_range,
            min: self.minmax_state == MinMaxState::Min,
            max: self.minmax_state == MinMaxState::Max,
            peak_min: self.peak_state == PeakState::Min,
            peak_max: self.peak_state == PeakState::Max,
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
            aux_values: vec![],
            raw_payload: vec![],
            spec: None,
            mode_spec: None,
        };

        if elapsed >= duration_secs {
            if self.auto_cycle {
                self.advance_scenario();
            } else {
                // Loop the pattern without changing mode.
                self.scenario_started = Instant::now();
            }
        }

        Ok(measurement)
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        match command {
            "hold" => {
                self.hold = !self.hold;
                if self.hold {
                    let elapsed = self.elapsed_secs();
                    let scenario = self.current_scenario();
                    self.held_value = Some((scenario.value_fn)(elapsed));
                } else {
                    self.held_value = None;
                }
            }
            "rel" => {
                self.rel = !self.rel;
                if self.rel {
                    let elapsed = self.elapsed_secs();
                    let scenario = self.current_scenario();
                    if let MeasuredValue::Normal(v) = (scenario.value_fn)(elapsed) {
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
                // Real device cycles: Off → MAX → MIN → MAX → MIN ...
                self.minmax_state = match self.minmax_state {
                    MinMaxState::Off => {
                        self.auto_range_before_minmax = self.auto_range;
                        self.auto_range = false;
                        self.stored_min = None;
                        self.stored_max = None;
                        MinMaxState::Max
                    }
                    MinMaxState::Max => MinMaxState::Min,
                    MinMaxState::Min => MinMaxState::Max,
                };
            }
            "exit_minmax" => {
                self.minmax_state = MinMaxState::Off;
                self.stored_min = None;
                self.stored_max = None;
                self.auto_range = self.auto_range_before_minmax;
            }
            "peak" => {
                // Real device cycles: Off → P-MAX → P-MIN → P-MAX → P-MIN ...
                self.peak_state = match self.peak_state {
                    PeakState::Off => {
                        self.stored_peak_min = None;
                        self.stored_peak_max = None;
                        PeakState::Max
                    }
                    PeakState::Max => PeakState::Min,
                    PeakState::Min => PeakState::Max,
                };
            }
            "exit_peak" => {
                self.peak_state = PeakState::Off;
                self.stored_peak_min = None;
                self.stored_peak_max = None;
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
        // Values are a function of elapsed time, so triggering auto-advance
        // requires rewinding the scenario origin past its duration rather than
        // counting reads. Drive the protocol directly to access private state.
        let mut proto = MockProtocol::new();
        let transport = NullTransport;
        let first_mode = proto
            .request_measurement(&transport)
            .unwrap()
            .mode
            .into_owned();
        // Rewind past the current scenario's duration and take a reading,
        // which triggers auto-advance.
        proto.scenario_started -= Duration::from_secs(60);
        let _ = proto.request_measurement(&transport).unwrap();
        let new_mode = proto.request_measurement(&transport).unwrap().mode;
        assert_ne!(first_mode, new_mode.as_ref());
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
    fn test_minmax_flags_cycle() {
        let mut dmm = open_mock().unwrap();

        // First press → MAX only (matching real device)
        dmm.send_command("minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.max, "first press should show MAX");
        assert!(!m.flags.min, "first press should NOT show MIN");
        assert!(
            !m.flags.auto_range,
            "auto_range should be off during MIN/MAX"
        );

        // Second press → MIN only
        dmm.send_command("minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.max, "second press should NOT show MAX");
        assert!(m.flags.min, "second press should show MIN");

        // Third press → back to MAX
        dmm.send_command("minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.max, "third press should show MAX again");
        assert!(!m.flags.min, "third press should NOT show MIN");

        // Exit → both off, auto_range restored
        dmm.send_command("exit_minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.min);
        assert!(!m.flags.max);
        assert!(
            m.flags.auto_range,
            "auto_range should be restored after exit"
        );
    }

    #[test]
    fn test_minmax_reports_stored_values() {
        // Drive the protocol directly so we can rewind the scenario origin
        // between reads, giving the MIN/MAX tracker actual variation to follow.
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        let transport = NullTransport;

        // Advance the waveform a few times before enabling MIN/MAX so the
        // initial stored values are non-zero.
        for _ in 0..5 {
            proto.scenario_started -= Duration::from_millis(100);
            let _ = proto.request_measurement(&transport).unwrap();
        }

        proto.send_command(&transport, "minmax").unwrap();
        let mut max_values = Vec::new();
        for _ in 0..10 {
            proto.scenario_started -= Duration::from_millis(100);
            let m = proto.request_measurement(&transport).unwrap();
            if let MeasuredValue::Normal(v) = &m.value {
                max_values.push(*v);
            }
        }

        // In MAX state, the reported value is the running maximum —
        // non-decreasing regardless of the underlying waveform.
        for window in max_values.windows(2) {
            assert!(
                window[1] >= window[0] - 1e-10,
                "MAX value should be non-decreasing: {} then {}",
                window[0],
                window[1]
            );
        }

        proto.send_command(&transport, "minmax").unwrap();
        let mut min_values = Vec::new();
        for _ in 0..10 {
            proto.scenario_started -= Duration::from_millis(100);
            let m = proto.request_measurement(&transport).unwrap();
            if let MeasuredValue::Normal(v) = &m.value {
                min_values.push(*v);
            }
        }

        // In MIN state, the reported value is the running minimum —
        // non-increasing regardless of the underlying waveform.
        for window in min_values.windows(2) {
            assert!(
                window[1] <= window[0] + 1e-10,
                "MIN value should be non-increasing: {} then {}",
                window[0],
                window[1]
            );
        }

        proto.send_command(&transport, "exit_minmax").unwrap();
    }

    #[test]
    fn test_with_mode_pins_scenario() {
        let mut proto = MockProtocol::with_mode(MockMode::Hz);
        let transport = NullTransport;
        let m1 = proto.request_measurement(&transport).unwrap();
        assert_eq!(m1.mode, "Hz");
        // Rewind several times past the scenario duration — auto_cycle is off
        // so we should stay in Hz no matter how much time passes.
        for _ in 0..5 {
            proto.scenario_started -= Duration::from_secs(30);
            let _ = proto.request_measurement(&transport).unwrap();
        }
        let m2 = proto.request_measurement(&transport).unwrap();
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
    fn test_peak_flags_cycle() {
        let mut dmm = open_mock().unwrap();

        // First press → P-MAX only (matching real device)
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.peak_max, "first press should show P-MAX");
        assert!(!m.flags.peak_min, "first press should NOT show P-MIN");

        // Second press → P-MIN only
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.peak_max, "second press should NOT show P-MAX");
        assert!(m.flags.peak_min, "second press should show P-MIN");

        // Third press → back to P-MAX
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.peak_max, "third press should show P-MAX again");
        assert!(!m.flags.peak_min, "third press should NOT show P-MIN");

        // Exit → both off
        dmm.send_command("exit_peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.peak_min);
        assert!(!m.flags.peak_max);
    }

    #[test]
    fn test_dcv_is_smooth_function_of_time() {
        // dcv_value is `5 + 3 * sin(2t/3)`, angular period 3π seconds, so
        // samples one period apart must match exactly and samples a quarter
        // period apart must be symmetric about the centre value. This is what
        // makes the displayed waveform jitter-free — the value depends only on
        // the sample time, not on the read cadence.
        let period = 3.0 * std::f64::consts::PI;
        let a = match dcv_value(0.0) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let b = match dcv_value(period) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let c = match dcv_value(2.0 * period) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        assert!((a - b).abs() < 1e-9, "period mismatch: {a} vs {b}");
        assert!((b - c).abs() < 1e-9, "period mismatch: {b} vs {c}");

        // Half-period (sin is odd about zero): values symmetric about 5.0.
        let left = match dcv_value(period / 4.0) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        let right = match dcv_value(3.0 * period / 4.0) {
            MeasuredValue::Normal(v) => v,
            _ => panic!("expected Normal"),
        };
        assert!(
            ((left - 5.0) + (right - 5.0)).abs() < 1e-9,
            "half-period samples should be symmetric about 5.0: {left}, {right}"
        );
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
