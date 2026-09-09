//! A meter without hardware: the device the GUI, the CLI demos and the
//! screenshots run against.
//!
//! The mock stands in for a UT61E+, so it emits that family's mode and range
//! bytes, answers spec lookups from its table, and reaches its settings
//! through the same [`cycle`] driver the button-cycling families use — a
//! press being a change to [`state::MeterState`] instead of a wire write,
//! and a fresh frame being the next synthesised reading. Only the mode
//! selector is its own (see [`MockProtocol::mode_choices`]).
//!
//! [`scenarios`] holds what it measures, [`state`] what its buttons do.

mod scenarios;
mod state;

use crate::Dmm;
use crate::clock::Clock;
use crate::error::{Error, Result};
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::cycle::{self, CycleButton, CycleMeter, FlagSetting};
use crate::protocol::ut61eplus::mode::Mode;
use crate::protocol::ut61eplus::tables::ut61e_plus::Ut61ePlusTable;
use crate::protocol::ut61eplus::tables::{self, DeviceTable};
use crate::protocol::{Choice, DeviceProfile, Protocol, Setting, Stability, unsupported_setting};
use crate::transport::{NullTransport, Transport};
use scenarios::{AuxSpec, Scenario, scenarios};
use state::MeterState;
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
    AcVHz,
    TempDual,
    TempDiff,
    TempDiffRev,
}

/// One row of the mode table: everything `MockMode` exposes for a variant.
struct MockModeInfo {
    mode: MockMode,
    /// Short string label for CLI and display.
    label: &'static str,
    description: &'static str,
    /// Extra spellings `FromStr` accepts, beyond `label`.
    aliases: &'static [&'static str],
}

/// Every mode, in auto-cycle order. The order of the first nine entries is
/// load-bearing for the GUI demo and several tests; the multi-display and
/// differential modes were appended so it stayed unchanged.
const MODES: [MockModeInfo; 13] = [
    MockModeInfo {
        mode: MockMode::DcV,
        label: "dcv",
        description: "DC Voltage (sine wave around 5V)",
        aliases: &["dc-v", "dc_v"],
    },
    MockModeInfo {
        mode: MockMode::AcV,
        label: "acv",
        description: "AC Voltage (sine wave around 120V)",
        aliases: &["ac-v", "ac_v"],
    },
    MockModeInfo {
        mode: MockMode::Ohm,
        label: "ohm",
        description: "Resistance (step 1-10 kΩ)",
        aliases: &["ohms", "resistance"],
    },
    MockModeInfo {
        mode: MockMode::Capacitance,
        label: "cap",
        description: "Capacitance (ramp 1-20 µF)",
        aliases: &["capacitance"],
    },
    MockModeInfo {
        mode: MockMode::Hz,
        label: "hz",
        description: "Frequency (sine wave around 60Hz)",
        aliases: &["freq", "frequency"],
    },
    MockModeInfo {
        mode: MockMode::Temp,
        label: "temp",
        description: "Temperature (ramp 20-30°C)",
        aliases: &["temperature"],
    },
    MockModeInfo {
        mode: MockMode::DcMa,
        label: "dcma",
        description: "DC mA (sine wave around 50mA)",
        aliases: &["dc-ma", "dc_ma", "ma"],
    },
    MockModeInfo {
        mode: MockMode::OhmOl,
        label: "ohm-ol",
        description: "Resistance overload (OL)",
        aliases: &["ohm_ol", "ol", "overload"],
    },
    MockModeInfo {
        mode: MockMode::Ncv,
        label: "ncv",
        description: "NCV (cycling levels 0-4)",
        aliases: &[],
    },
    MockModeInfo {
        mode: MockMode::AcVHz,
        label: "acv-hz",
        description: "AC Voltage with frequency and period sub-displays",
        aliases: &["acvhz", "acv_hz"],
    },
    MockModeInfo {
        mode: MockMode::TempDual,
        label: "temp2",
        description: "Temperature with a second thermocouple (T2)",
        aliases: &["temp-dual", "temp_dual"],
    },
    MockModeInfo {
        mode: MockMode::TempDiff,
        label: "temp-diff",
        description: "Temperature difference T1-T2",
        aliases: &["tempdiff", "temp_diff"],
    },
    MockModeInfo {
        mode: MockMode::TempDiffRev,
        label: "temp-diff-rev",
        description: "Temperature difference T2-T1",
        aliases: &["tempdiffrev", "temp_diff_rev"],
    },
];

/// `MODES` projected to bare variants, so `ALL` can't drift from the table.
const fn all_modes() -> [MockMode; MODES.len()] {
    let mut out = [MockMode::DcV; MODES.len()];
    let mut i = 0;
    while i < MODES.len() {
        out[i] = MODES[i].mode;
        i += 1;
    }
    out
}

impl MockMode {
    /// All available modes in scenario order.
    pub const ALL: &[MockMode] = &all_modes();

    /// This mode's table row. `every_mode_has_exactly_one_table_entry`
    /// guarantees the lookup finds one.
    fn info(self) -> &'static MockModeInfo {
        MODES
            .iter()
            .find(|info| info.mode == self)
            .expect("MODES must contain every MockMode variant")
    }

    /// Short string label for CLI and display.
    pub fn label(self) -> &'static str {
        self.info().label
    }

    /// Human-readable description.
    pub fn description(self) -> &'static str {
        self.info().description
    }

    /// This mode's index in [`MockMode::ALL`] — the id the mock hands to
    /// `Protocol::choices` and takes back in `Protocol::select`.
    fn choice_id(self) -> Option<u16> {
        MockMode::ALL
            .iter()
            .position(|m| *m == self)
            .and_then(|i| u16::try_from(i).ok())
    }

    /// Every label, comma-separated: the list `--mock-mode` help and the
    /// parse error both quote, so the two can't name different modes.
    pub(crate) fn label_list() -> String {
        MockMode::ALL
            .iter()
            .map(|m| m.label())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The mode behind a `Protocol::choices` id — the inverse of
    /// [`Self::choice_id`], for a consumer that has to name the scenario it
    /// just picked (the GUI re-pins its Settings row with it).
    pub fn from_choice_id(id: u16) -> Option<MockMode> {
        MockMode::ALL.get(usize::from(id)).copied()
    }
}

/// The mode groups the mock offers as remote mode choices, standing in for a
/// meter's "same dial position, different function" set (the UT181A's V AC
/// with and without its Hz sub-display, say). Everything outside a group
/// reports no choices, so consumers exercise the unsupported path too.
const MOCK_MODE_GROUPS: [&[MockMode]; 2] = [
    &[MockMode::AcV, MockMode::AcVHz],
    // Four entries, like a real UT181A on the temperature dial: one probe,
    // both probes, and the two arithmetic arrangements.
    &[
        MockMode::Temp,
        MockMode::TempDual,
        MockMode::TempDiff,
        MockMode::TempDiffRev,
    ],
];

impl std::str::FromStr for MockMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let needle = s.to_lowercase();
        for info in &MODES {
            if info.label == needle || info.aliases.contains(&needle.as_str()) {
                return Ok(info.mode);
            }
        }
        Err(format!(
            "unknown mock mode: {s}\nValid modes: {}",
            MockMode::label_list()
        ))
    }
}

impl std::fmt::Display for MockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Mock protocol that generates synthetic measurements without hardware.
///
/// Remote mode selection (`Protocol::choices` / `Protocol::select`)
/// is modelled on the two multi-display scenario pairs listed in
/// [`MOCK_MODE_GROUPS`]: from either member the mock offers both, and
/// selecting one jumps the live scenario there and restarts its waveform. An
/// auto-cycling mock keeps auto-cycling — it resumes the cycle from the
/// selected scenario rather than pinning to it, so the demo keeps moving.
pub struct MockProtocol {
    scenarios: Vec<Scenario>,
    current_scenario: usize,
    /// Session instant the current scenario started. Values are evaluated at
    /// `now - scenario_started`, so the waveform is a smooth function of time
    /// regardless of read cadence. On scenario advance, this is reset to `now`.
    scenario_started: Instant,
    /// The session's time base, so a scaled or preseeded run gets the waveform
    /// a real run of that length would have produced. Real by default.
    clock: Clock,
    /// When false, stays on the current scenario indefinitely.
    auto_cycle: bool,
    /// What the buttons have done to the meter, and what that does to a
    /// reading.
    state: MeterState,
    /// The mode last read back, for the cycle driver. There is no dial to
    /// infer (see [`MockProtocol::dial_positions`]).
    dial: cycle::DialState,
    profile: DeviceProfile,
    /// The mock stands in for a UT61E+, so it answers spec lookups from the
    /// UT61E+ table rather than carrying a second copy that can drift.
    table: Ut61ePlusTable,
}

impl MockProtocol {
    /// Create a mock protocol that auto-cycles through all scenarios.
    pub fn new() -> Self {
        let clock = Clock::real();
        Self {
            scenarios: scenarios(),
            current_scenario: 0,
            scenario_started: clock.now(),
            clock,
            auto_cycle: true,
            state: MeterState::default(),
            dial: cycle::DialState::default(),
            profile: DeviceProfile {
                family_name: "mock",
                model_name: "Mock UT61E+",
                // Verified so the GUI doesn't show the EXPERIMENTAL badge —
                // mock behavior is deterministic and needs no hardware validation.
                stability: Stability::Verified,
                supported_commands: MOCK_COMMANDS,
                max_aux_values: 2,
                verification_issue: None,
            },
            table: Ut61ePlusTable::new(),
        }
    }

    /// Create a mock protocol pinned to a specific mode.
    /// The mode repeats indefinitely; use `select`/`select2` commands to switch manually.
    pub fn with_mode(mode: MockMode) -> Self {
        let all = scenarios();
        // test_with_mode_covers_all_variants below enforces this lookup can't fail.
        let idx = all
            .iter()
            .position(|s| s.id == mode)
            .expect("scenarios() must contain every MockMode variant");
        let mut proto = Self::new();
        proto.current_scenario = idx;
        proto.auto_cycle = false;
        proto
    }

    /// Run the waveform on `clock` instead of wall time, restarting the
    /// current scenario at its `now`.
    ///
    /// `pub(crate)` because a mock session is built as a whole by
    /// [`open_mock_clocked`], which hands the same clock to the [`Dmm`].
    pub(crate) fn with_clock(mut self, clock: Clock) -> Self {
        self.scenario_started = clock.now();
        self.clock = clock;
        self
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
        self.scenario_started = self.clock.now();
        self.state.leave_scenario();
    }

    /// A scenario's mode byte as the UT61E+ table knows it.
    fn table_mode(mode_raw: u16) -> Option<Mode> {
        u8::try_from(mode_raw)
            .ok()
            .and_then(|b| Mode::from_byte(b).ok())
    }

    /// The range byte the meter reports: the rung RANGE was stepped to, or
    /// the one auto-ranging picked for the scenario.
    fn reported_range(&self) -> u8 {
        self.state.reported_range(self.current_scenario().range_raw)
    }

    /// What the reading calls its range.
    ///
    /// A manually selected rung renames the range the way the meter's own
    /// reading would. The value and its unit stay the scenario's: the label
    /// table carries no numeric limit to rescale them with, and a unit
    /// swapped on its own would put the reading off by a decade.
    fn range_label(&self, scenario: &Scenario, range_raw: u8) -> Cow<'static, str> {
        if self.state.on_a_manual_rung()
            && let Some(info) = Self::table_mode(scenario.mode_raw)
                .and_then(|mode| self.table.range_info(mode, range_raw))
        {
            return Cow::Borrowed(info.label);
        }
        Cow::Borrowed(scenario.range_label)
    }

    /// The mock's own state as a reading, for the cycle driver's benefit.
    ///
    /// `Protocol::choices` is handed a reading by the caller, but the mock is
    /// its own source of truth for what it is measuring and the caller's may
    /// be stale — so the mode, range and flag fields the driver looks at are
    /// filled from the live scenario instead. It reads no other field.
    fn state_frame(&self) -> Measurement {
        Measurement {
            mode_raw: self.current_scenario().mode_raw,
            range_raw: self.reported_range(),
            flags: self.state.flags(),
            ..Measurement::from_payload(&[])
        }
    }

    /// The scenarios reachable from the live one, for `Protocol::choices`.
    ///
    /// Not `cycle::mode_choices`: the mock has no dial table to resolve a
    /// position from, and its ids are scenario indices rather than mode
    /// bytes — four temperature scenarios all report 0x0A, so a mode byte
    /// cannot name which one is live. [`MOCK_MODE_GROUPS`] stands in for the
    /// dial position.
    fn mode_choices(&self) -> Vec<Choice> {
        let live = self.current_mode();
        let Some(group) = MOCK_MODE_GROUPS.iter().find(|g| g.contains(&live)) else {
            return Vec::new();
        };
        group
            .iter()
            .filter_map(|&mode| {
                let scenario = self.scenarios.iter().find(|s| s.id == mode)?;
                Some(Choice {
                    id: mode.choice_id()?,
                    label: Cow::Borrowed(scenario.mode),
                    current: mode == live,
                })
            })
            .collect()
    }

    /// Jump the live scenario, for `Protocol::select`.
    ///
    /// Not `cycle::select_mode`, for the reason [`Self::mode_choices`] gives:
    /// there is no ring to walk, so the scenario is switched outright and its
    /// waveform restarted.
    fn select_scenario(&mut self, id: u16) -> Result<()> {
        let scenario = MockMode::from_choice_id(id)
            .and_then(|mode| self.scenarios.iter().position(|s| s.id == mode));
        let Some(idx) = scenario else {
            return Err(Error::UnsupportedCommand(format!("mode {id:#06x}")));
        };
        self.current_scenario = idx;
        self.scenario_started = self.clock.now();
        Ok(())
    }

    /// The scenario's sub-values at `elapsed`.
    fn aux_values(specs: &'static [AuxSpec], elapsed: f64, duration: f64) -> Vec<AuxValue> {
        specs
            .iter()
            .map(|spec| {
                let value = (spec.value_fn)(elapsed, duration);
                AuxValue {
                    label: Cow::Borrowed(spec.label),
                    display_raw: Self::format_display(&value),
                    value,
                    unit: Cow::Borrowed(spec.unit),
                    elapsed_secs: None,
                }
            })
            .collect()
    }

    /// Elapsed seconds since the current scenario started. Uses
    /// `checked_duration_since` so a backward clock jump returns 0 instead of
    /// panicking.
    fn elapsed_secs(&self) -> f64 {
        self.clock
            .now()
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

    /// Lit segments on the UT61E+'s bar graph at full scale.
    ///
    /// The real meter sends `byte12 * 10 + byte13` = segments on a
    /// 46-segment LCD bar (hardware-verified, see
    /// docs/research/ut61eplus/reverse-engineered-protocol.md §"Bar graph
    /// encoding"). The mock previously scaled to 0-800, roughly 17x that, so
    /// anything developed or tested against `--device mock` saw a range the
    /// hardware never produces.
    const BAR_GRAPH_SEGMENTS: f64 = 46.0;

    /// Bar-graph position for a reading, on the meter's own segment scale.
    fn compute_progress(value: &MeasuredValue, range_max: f64) -> Option<u16> {
        match value {
            MeasuredValue::Normal(v) => {
                let ratio = v.abs() / range_max;
                Some((ratio * Self::BAR_GRAPH_SEGMENTS).min(Self::BAR_GRAPH_SEGMENTS) as u16)
            }
            // Over-range pins the bar full.
            MeasuredValue::Overload => Some(Self::BAR_GRAPH_SEGMENTS as u16),
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
        let elapsed = self.elapsed_secs();
        // A copy, so the meter state can be borrowed mutably below while the
        // reading is still being built from the scenario.
        let scenario = *self.current_scenario();
        let raw = (scenario.value_fn)(elapsed, scenario.duration_secs);
        let aux_values = Self::aux_values(
            scenario.aux,
            self.state.aux_elapsed(elapsed),
            scenario.duration_secs,
        );
        let value = self.state.apply(raw);
        let range_raw = self.reported_range();

        let measurement = Measurement {
            mode: Cow::Borrowed(scenario.mode),
            mode_raw: scenario.mode_raw,
            range_raw,
            range_label: self.range_label(&scenario, range_raw),
            unit: Cow::Borrowed(scenario.unit),
            display_raw: Self::format_display(&value),
            progress: Self::compute_progress(&value, scenario.range_max),
            value,
            flags: self.state.flags(),
            aux_values,
            // The mock has no wire bytes to report.
            ..Measurement::from_payload(&[])
        };
        // Every reading the cycle driver takes is one of these, so record the
        // mode from here as the hardware families do from their parser.
        let positions = self.dial_positions();
        self.dial.observe(positions, measurement.mode_raw);

        if elapsed >= scenario.duration_secs {
            if self.auto_cycle {
                self.advance_scenario();
            } else {
                // Loop the pattern without changing mode.
                self.scenario_started = self.clock.now();
            }
        }

        Ok(measurement)
    }

    fn parse_payload(&self, _payload: &[u8]) -> Result<Measurement> {
        // The mock synthesises readings; there are no wire bytes to decode.
        Err(Error::UnsupportedCommand(
            "parse_payload: the mock has no wire format".to_string(),
        ))
    }

    fn send_command(&mut self, _transport: &dyn Transport, command: &str) -> Result<()> {
        let elapsed = self.elapsed_secs();
        let scenario = *self.current_scenario();
        match command {
            "hold" => {
                let value = (scenario.value_fn)(elapsed, scenario.duration_secs);
                self.state.press_hold(value, elapsed);
            }
            "rel" => {
                let value = (scenario.value_fn)(elapsed, scenario.duration_secs);
                self.state.press_rel(value);
            }
            "range" => {
                let ladder = self.range_ladder(scenario.mode_raw);
                self.state.press_range(ladder.len(), scenario.range_raw);
            }
            "auto" => self.state.set_auto_range(),
            "minmax" => self.state.press_minmax(),
            "exit_minmax" => self.state.exit_minmax(),
            "peak" => {
                // The meter reacts to Peak only where the mode has one, and
                // silently ignores the press elsewhere.
                if scenario.peak_applies() {
                    self.state.press_peak();
                }
            }
            "exit_peak" => self.state.exit_peak(),
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

    // The mock emits UT61E+ mode and range bytes, so it answers spec lookups
    // from the UT61E+ table (with `mock_specs_match_the_ut61eplus_table`
    // asserting it). Without these overrides the trait defaults returned None,
    // so `Measurement::spec` was never populated and the GUI's Specifications
    // panel stayed empty for the whole mock session — the path used for demos,
    // screenshots and UI development.
    fn spec_info(&self, mode_raw: u16, range_raw: u8) -> Option<&'static crate::specs::SpecInfo> {
        Mode::from_byte(mode_raw as u8)
            .ok()
            .and_then(|mode| self.table.spec_info(mode, range_raw))
    }

    fn mode_spec_info(&self, mode_raw: u16) -> Option<&'static crate::specs::ModeSpecInfo> {
        Mode::from_byte(mode_raw as u8)
            .ok()
            .and_then(|mode| self.table.mode_spec_info(mode))
    }

    /// The `current` argument is ignored: the mock is its own source of
    /// truth for what it is measuring, and a caller could hand back a stale
    /// reading, so the driver is given [`Self::state_frame`] instead.
    fn choices(&self, setting: Setting, _current: &Measurement) -> Vec<Choice> {
        match setting {
            Setting::Mode => self.mode_choices(),
            Setting::Range => cycle::range_choices(self, &self.state_frame()),
            flag => match FlagSetting::of(flag) {
                Some(flag) => cycle::flag_choices(self, flag, &self.state_frame()),
                None => Vec::new(),
            },
        }
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        match setting {
            Setting::Mode => self.select_scenario(id),
            Setting::Range => {
                // The meter locks the range while MIN/MAX records (verified
                // 2026-03-21), and the walk cannot discover that: the lock
                // freezes the reading on the rung it caught, so a select of
                // that very rung would look like it had already arrived.
                // Only the rungs the walk would otherwise reach are refused,
                // leaving an unknown id to the driver's own validation.
                if self.state.minmax_recording()
                    && cycle::range_choices(self, &self.state_frame())
                        .iter()
                        .any(|c| c.id == id)
                {
                    return Err(Error::CommandRejected(
                        "MIN/MAX locks the range; leave MIN/MAX before changing it".to_string(),
                    ));
                }
                cycle::select_range(self, transport, id)
            }
            flag => match FlagSetting::of(flag) {
                Some(flag) => cycle::select_flag(self, transport, flag, id),
                None => Err(unsupported_setting(setting)),
            },
        }
    }
}

/// The mock reaches its settings through the same driver the button-cycling
/// families use, so a choice list or a select that works here works there. A
/// press is a change to [`MeterState`] rather than a wire write, and the
/// fresh frame the driver reads back is the next synthesised reading.
impl CycleMeter for MockProtocol {
    /// No dial to walk: the mock's ids are scenario indices, not mode bytes
    /// (see [`MockProtocol::mode_choices`]), so mode walking never applies
    /// and the inferred position stays unknown.
    fn dial_positions(&self) -> &'static [cycle::DialPosition] {
        &[]
    }

    fn dial_state(&self) -> &cycle::DialState {
        &self.dial
    }

    fn dial_state_mut(&mut self) -> &mut cycle::DialState {
        &mut self.dial
    }

    /// The button's own command — the one `dmm-cli command` sends.
    fn press(&mut self, transport: &dyn Transport, button: CycleButton) -> Result<()> {
        let command = match button {
            CycleButton::Select => "select",
            CycleButton::Hz => "select2",
            CycleButton::Range => "range",
            CycleButton::Hold => "hold",
            CycleButton::Rel => "rel",
            CycleButton::MinMax => "minmax",
            CycleButton::Peak => "peak",
        };
        self.send_command(transport, command)
    }

    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        Protocol::request_measurement(self, transport)
    }

    /// The scenario's own name, so a message names the mode the way the
    /// reading does. The live scenario answers first: several share a mode
    /// byte, and the messages are always about the one the mock is in.
    fn mode_label(&self, mode: u16) -> Cow<'static, str> {
        let live = self.current_scenario();
        if live.mode_raw == mode {
            return Cow::Borrowed(live.mode);
        }
        match self.scenarios.iter().find(|s| s.mode_raw == mode) {
            Some(scenario) => Cow::Borrowed(scenario.mode),
            None => Cow::Owned(format!("mode {mode:#06x}")),
        }
    }

    /// Nothing to wait for: a press lands in the mock's own state and the
    /// next synthesised reading already shows it, so no stale frame can
    /// arrive.
    fn settle(&self) -> cycle::Settle {
        cycle::Settle {
            delay: Duration::ZERO,
            reads: 1,
        }
    }

    /// The mock stands in for a UT61E+, so the ladder is that meter's, from
    /// the same table the range labels come from.
    fn range_ladder(&self, mode: u16) -> Vec<Cow<'static, str>> {
        match Self::table_mode(mode) {
            Some(mode) => tables::range_ladder(&self.table, mode),
            None => Vec::new(),
        }
    }

    fn set_auto_range(&mut self, transport: &dyn Transport) -> Result<()> {
        self.send_command(transport, "auto")
    }

    /// HOLD, REL and MIN/MAX everywhere, MIN/MAX as the MAX/MIN ring with no
    /// AVG, and Peak only where the live scenario reacts to it — offering it
    /// elsewhere would list a state select cannot reach.
    ///
    /// Keyed on the scenario rather than the `mode` argument: the resistance
    /// scenarios share mode byte 0x06 and disagree about Peak, so the byte
    /// alone cannot answer.
    fn flag_states(&self, setting: FlagSetting, _mode: u16) -> &'static [u16] {
        match setting {
            FlagSetting::Hold | FlagSetting::Rel => &[0, 1],
            FlagSetting::MinMax => &[0, 1, 2],
            FlagSetting::Peak if self.current_scenario().peak_applies() => &[0, 1, 2],
            FlagSetting::Peak => &[],
        }
    }

    fn exit_flag(&mut self, transport: &dyn Transport, setting: FlagSetting) -> Result<()> {
        match setting {
            FlagSetting::MinMax => self.send_command(transport, "exit_minmax"),
            FlagSetting::Peak => self.send_command(transport, "exit_peak"),
            // HOLD and REL press their own button back off, so the driver
            // never asks this of them.
            other => Err(unsupported_setting(other.setting())),
        }
    }
}

/// Create a mock Dmm instance that auto-cycles through all scenarios.
pub fn open_mock() -> Result<Dmm<NullTransport>> {
    open_mock_clocked(None, Clock::real())
}

/// Create a mock Dmm instance pinned to a specific mode.
pub fn open_mock_mode(mode: MockMode) -> Result<Dmm<NullTransport>> {
    open_mock_clocked(Some(mode), Clock::real())
}

/// Create a mock Dmm instance running on `clock`, pinned to `mode` if given.
///
/// The one entry point that takes a clock: both halves of a mock session need
/// it — the waveform is a function of session time, and the `Dmm` stamps its
/// readings with the same clock — and the mock is the only device a virtual
/// clock makes sense for, a real meter being paced by USB.
pub fn open_mock_clocked(mode: Option<MockMode>, clock: Clock) -> Result<Dmm<NullTransport>> {
    let protocol = match mode {
        Some(mode) => MockProtocol::with_mode(mode),
        None => MockProtocol::new(),
    };
    let dmm = Dmm::new(NullTransport, Box::new(protocol.with_clock(clock.clone())))?;
    Ok(dmm.with_clock(clock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scenarios::{
        scenario_duration, temp_diff_rev_value, temp_diff_value, temp_value, temp2_value,
    };

    /// The table is the single source of truth for `ALL`, `label()`,
    /// `description()` and `FromStr`; `info()` looks a mode up there instead of
    /// matching, so a missing or duplicated row must fail here.
    #[test]
    fn every_mode_has_exactly_one_table_entry() {
        assert_eq!(MODES.len(), MockMode::ALL.len());
        for mode in MockMode::ALL {
            let matches = MODES.iter().filter(|info| info.mode == *mode).count();
            assert_eq!(matches, 1, "{mode:?} has {matches} table entries");
        }
    }

    #[test]
    fn labels_and_aliases_are_unique_and_round_trip() {
        let mut seen: Vec<&str> = Vec::new();
        for info in &MODES {
            for name in std::iter::once(&info.label).chain(info.aliases) {
                assert!(!seen.contains(name), "'{name}' appears twice in MODES");
                seen.push(name);
                assert_eq!(
                    name.parse::<MockMode>().unwrap(),
                    info.mode,
                    "'{name}' should parse as {:?}",
                    info.mode
                );
            }
            assert_eq!(info.mode.label(), info.label);
            assert_eq!(info.mode.to_string(), info.label);
        }
    }

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

    /// `Dmm::request_measurement` is what attaches specs, so go through it —
    /// the parser alone leaves `spec` as None by design. Without the mock's
    /// spec_info overrides the GUI's Specifications panel is empty for the
    /// whole session.
    #[test]
    fn mock_measurements_carry_specs() {
        let mut dmm = open_mock().unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "DC V");
        let spec = m.spec.expect("DC V should resolve a spec from the table");
        assert!(!spec.resolution.is_empty());
        assert!(
            m.mode_spec.is_some_and(|ms| ms.input_impedance.is_some()),
            "DC V should report an input impedance"
        );
    }

    /// The mock is meant to stand in for a UT61E+, so its specs must be the
    /// UT61E+'s — not a second copy that can drift.
    #[test]
    fn mock_specs_match_the_ut61eplus_table() {
        let mut dmm = open_mock().unwrap();
        let m = dmm.request_measurement().unwrap();
        let mode = Mode::from_byte(m.mode_raw as u8).expect("mock emits valid UT61E+ mode bytes");
        let direct = Ut61ePlusTable::new().spec_info(mode, m.range_raw);
        assert_eq!(
            m.spec.map(|s| s.resolution),
            direct.map(|s| s.resolution),
            "mock spec must come from the UT61E+ table"
        );
    }

    /// .claude/rules/protocol.md: "Mocks must match real-device behavior …
    /// Mocks that diverge create false confidence." The real UT61E+ bar graph
    /// is 46 segments; the mock used to emit up to 800.
    #[test]
    fn progress_stays_on_the_real_bar_graph_scale() {
        let mut dmm = open_mock().unwrap();
        for _ in 0..20 {
            let m = dmm.request_measurement().unwrap();
            if let Some(p) = m.progress {
                assert!(
                    p <= MockProtocol::BAR_GRAPH_SEGMENTS as u16,
                    "progress {p} exceeds the meter's 46-segment bar"
                );
            }
        }
    }

    #[test]
    fn overload_pins_the_bar_full() {
        let p = MockProtocol::compute_progress(&MeasuredValue::Overload, 22.0);
        assert_eq!(p, Some(MockProtocol::BAR_GRAPH_SEGMENTS as u16));
    }

    #[test]
    fn progress_scales_with_the_range() {
        // Half of full scale → half the segments.
        let half = MockProtocol::compute_progress(&MeasuredValue::Normal(11.0), 22.0);
        assert_eq!(half, Some(23));
        let zero = MockProtocol::compute_progress(&MeasuredValue::Normal(0.0), 22.0);
        assert_eq!(zero, Some(0));
    }

    #[test]
    fn test_mode_cycling() {
        // Values are a function of elapsed time, so triggering auto-advance
        // means moving session time past the scenario duration rather than
        // counting reads. Drive the protocol directly, on a clock the test owns.
        let clock = Clock::manual();
        let mut proto = MockProtocol::new().with_clock(clock.clone());
        let transport = NullTransport;
        let first_mode = proto
            .request_measurement(&transport)
            .unwrap()
            .mode
            .into_owned();
        // Move past the current scenario's duration and take a reading, which
        // triggers auto-advance.
        clock.advance(Duration::from_secs(60));
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
    fn profile_max_aux_values_covers_every_scenario() {
        // The CLI and GUI size their fixed sub-value columns from this, so a
        // scenario may never emit more than the profile promises.
        let widest = scenarios().iter().map(|s| s.aux.len()).max().unwrap_or(0);
        assert_eq!(MockProtocol::new().profile().max_aux_values, widest);
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
        // Drive the protocol directly so we can move session time between
        // reads, giving the MIN/MAX tracker actual variation to follow.
        let clock = Clock::manual();
        let mut proto = MockProtocol::with_mode(MockMode::DcV).with_clock(clock.clone());
        let transport = NullTransport;

        // Advance the waveform a few times before enabling MIN/MAX so the
        // initial stored values are non-zero.
        for _ in 0..5 {
            clock.advance(Duration::from_millis(100));
            let _ = proto.request_measurement(&transport).unwrap();
        }

        proto.send_command(&transport, "minmax").unwrap();
        let mut max_values = Vec::new();
        for _ in 0..10 {
            clock.advance(Duration::from_millis(100));
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
            clock.advance(Duration::from_millis(100));
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
    fn test_with_mode_covers_all_variants() {
        // with_mode() panics if scenarios() is missing a MockMode variant.
        // Iterate every known variant to guarantee the lookup never fails —
        // a new variant without a matching scenario would trip this test
        // before it reaches a caller.
        for mode in MockMode::ALL {
            let proto = MockProtocol::with_mode(*mode);
            assert_eq!(proto.current_mode(), *mode);
        }
    }

    #[test]
    fn test_with_mode_pins_scenario() {
        let clock = Clock::manual();
        let mut proto = MockProtocol::with_mode(MockMode::Hz).with_clock(clock.clone());
        let transport = NullTransport;
        let m1 = proto.request_measurement(&transport).unwrap();
        assert_eq!(m1.mode, "Hz");
        // Run several times past the scenario duration — auto_cycle is off so
        // we should stay in Hz no matter how much time passes.
        for _ in 0..5 {
            clock.advance(Duration::from_secs(30));
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
        assert_eq!("acv-hz".parse::<MockMode>().unwrap(), MockMode::AcVHz);
        assert_eq!("acv_hz".parse::<MockMode>().unwrap(), MockMode::AcVHz);
        assert_eq!("acvhz".parse::<MockMode>().unwrap(), MockMode::AcVHz);
        assert_eq!("temp2".parse::<MockMode>().unwrap(), MockMode::TempDual);
        assert_eq!("temp-dual".parse::<MockMode>().unwrap(), MockMode::TempDual);
        assert_eq!("temp_dual".parse::<MockMode>().unwrap(), MockMode::TempDual);
        assert_eq!("temp-diff".parse::<MockMode>().unwrap(), MockMode::TempDiff);
        assert_eq!("temp_diff".parse::<MockMode>().unwrap(), MockMode::TempDiff);
        assert_eq!(
            "temp-diff-rev".parse::<MockMode>().unwrap(),
            MockMode::TempDiffRev
        );
        assert_eq!(
            "temp_diff_rev".parse::<MockMode>().unwrap(),
            MockMode::TempDiffRev
        );
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
    fn test_peak_ignored_on_dc_mode() {
        // Real device silently ignores Peak on DC modes (spec §2.7).
        let mut dmm = open_mock().unwrap(); // starts on DC V
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(!m.flags.peak_max && !m.flags.peak_min);
    }

    #[test]
    fn test_peak_and_minmax_mutually_exclusive() {
        let mut dmm = open_mock().unwrap();
        dmm.send_command("select").unwrap(); // DC V → AC V
        dmm.send_command("minmax").unwrap();
        dmm.send_command("peak").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.peak_max, "peak engaged");
        assert!(
            !m.flags.min && !m.flags.max,
            "minmax must end when peak starts"
        );
        dmm.send_command("minmax").unwrap();
        let m = dmm.request_measurement().unwrap();
        assert!(m.flags.max, "minmax engaged");
        assert!(
            !m.flags.peak_max && !m.flags.peak_min,
            "peak must end when minmax starts"
        );
    }

    #[test]
    fn test_peak_flags_cycle() {
        let mut dmm = open_mock().unwrap();
        dmm.send_command("select").unwrap(); // move off DC V (peak ignored there)

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
    fn acv_hz_emits_frequency_and_period_sub_values() {
        let mut dmm = open_mock_mode(MockMode::AcVHz).unwrap();
        let m = dmm.request_measurement().unwrap();
        // Named for its sub-displays, so it doesn't read as the plain AC V
        // scenario it shares a mode group with.
        assert_eq!(m.mode, "AC V Hz");
        assert_eq!(m.unit, "V");
        assert_eq!(m.aux_values.len(), 2);
        assert_eq!(m.aux_values[0].label, "Frequency");
        assert_eq!(m.aux_values[0].unit, "Hz");
        assert_eq!(m.aux_values[1].label, "Period");
        assert_eq!(m.aux_values[1].unit, "ms");
        for a in &m.aux_values {
            assert!(
                a.display_raw.is_some(),
                "{}: sub-values need display digits",
                a.label
            );
        }
        // Period must be the reciprocal of the frequency it sits next to.
        let (hz, ms) = match (&m.aux_values[0].value, &m.aux_values[1].value) {
            (MeasuredValue::Normal(hz), MeasuredValue::Normal(ms)) => (*hz, *ms),
            other => panic!("expected Normal sub-values, got {other:?}"),
        };
        assert!(
            (ms - 1000.0 / hz).abs() < 1e-9,
            "period {ms} ms should be 1000/{hz}"
        );
    }

    #[test]
    fn temp_dual_emits_a_second_thermocouple() {
        let mut dmm = open_mock_mode(MockMode::TempDual).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Temp \u{00B0}C T1 (T2)");
        assert_eq!(m.unit, "\u{00B0}C");
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "T2");
        assert_eq!(m.aux_values[0].unit, "\u{00B0}C");
        assert!(m.aux_values[0].display_raw.is_some());

        // T2 has its own shape, so it must actually leave T1 somewhere over a
        // period — while staying in the same band, so one Y axis holds both.
        let duration = scenario_duration(MockMode::TempDual);
        const SAMPLES: usize = 16;
        let mut differed = false;
        for i in 0..SAMPLES {
            let t = i as f64 / SAMPLES as f64 * duration;
            let (t1, t2) = match (temp_value(t, duration), temp2_value(t, duration)) {
                (MeasuredValue::Normal(t1), MeasuredValue::Normal(t2)) => (t1, t2),
                other => panic!("expected Normal values, got {other:?}"),
            };
            assert!(
                (20.0..=30.0).contains(&t2),
                "T2 {t2} at t={t} left T1's 20-30 °C band"
            );
            differed |= (t1 - t2).abs() > 0.1;
        }
        assert!(differed, "T2 never departs from T1");
    }

    /// The arithmetic arrangements must show the real difference of the two
    /// probes `temp2` displays — a separate waveform would read as a
    /// contradiction when switching between the modes — and the reversed
    /// arrangement must be its negation. Neither carries sub-values: the
    /// meter's aux layout in these arrangements is unverified, so the mock
    /// claims nothing.
    #[test]
    fn temperature_differentials_are_opposite_and_single_display() {
        let mut dmm = open_mock_mode(MockMode::TempDiff).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Temp \u{00B0}C T1-T2");
        assert_eq!(m.unit, "\u{00B0}C");
        assert!(m.aux_values.is_empty());

        let mut dmm = open_mock_mode(MockMode::TempDiffRev).unwrap();
        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Temp \u{00B0}C T2-T1");
        assert!(m.aux_values.is_empty());

        // Sharing the dual scenario's clock is what lets the four readings
        // agree: the same elapsed time must mean the same T1 and T2 in all of
        // them.
        let duration = scenario_duration(MockMode::TempDiff);
        assert_eq!(duration, scenario_duration(MockMode::TempDual));
        assert_eq!(duration, scenario_duration(MockMode::TempDiffRev));

        const SAMPLES: usize = 16;
        let mut swing: f64 = 0.0;
        for i in 0..SAMPLES {
            let t = i as f64 / SAMPLES as f64 * duration;
            let (t1, t2) = match (temp_value(t, duration), temp2_value(t, duration)) {
                (MeasuredValue::Normal(t1), MeasuredValue::Normal(t2)) => (t1, t2),
                other => panic!("expected Normal probe values, got {other:?}"),
            };
            match (
                temp_diff_value(t, duration),
                temp_diff_rev_value(t, duration),
            ) {
                (MeasuredValue::Normal(fwd), MeasuredValue::Normal(rev)) => {
                    assert!(
                        (fwd - (t1 - t2)).abs() < 1e-9,
                        "T1-T2 read {fwd} but the probes say {t1} - {t2} at t={t}"
                    );
                    assert!(
                        (rev - (t2 - t1)).abs() < 1e-9,
                        "T2-T1 read {rev} but the probes say {t2} - {t1} at t={t}"
                    );
                    swing = swing.max(fwd.abs());
                }
                other => panic!("expected Normal values, got {other:?}"),
            }
        }
        // The probes are shaped to cross, so the difference has to move — a
        // constant zero would satisfy every assertion above.
        assert!(swing > 1.0, "the differential never left {swing} °C");
    }

    #[test]
    fn hold_freezes_sub_values_with_the_main_value() {
        let clock = Clock::manual();
        let mut proto = MockProtocol::with_mode(MockMode::AcVHz).with_clock(clock.clone());
        let transport = NullTransport;
        let _ = proto.request_measurement(&transport).unwrap();
        proto.send_command(&transport, "hold").unwrap();
        let held1 = proto.request_measurement(&transport).unwrap();
        // Advance the waveform on the session clock, as the other time-travel
        // tests do — well short of the 10 s scenario duration.
        clock.advance(Duration::from_secs(2));
        let held2 = proto.request_measurement(&transport).unwrap();
        assert!(held2.flags.hold);
        assert_eq!(held1.aux_values.len(), 2);
        for (a, b) in held1.aux_values.iter().zip(&held2.aux_values) {
            assert_eq!(a.label, b.label);
            assert_eq!(a.value_str(), b.value_str(), "{} moved while held", a.label);
        }

        // Release: the same 2 s shift must move the sub-values again,
        // otherwise the assertions above would pass on a frozen waveform.
        proto.send_command(&transport, "hold").unwrap();
        let live1 = proto.request_measurement(&transport).unwrap();
        clock.advance(Duration::from_secs(2));
        let live2 = proto.request_measurement(&transport).unwrap();
        assert!(!live2.flags.hold);
        assert_ne!(
            live1.aux_values[0].value_str(),
            live2.aux_values[0].value_str()
        );
    }

    #[test]
    fn single_display_scenarios_have_no_sub_values() {
        for mode in MockMode::ALL {
            if matches!(mode, MockMode::AcVHz | MockMode::TempDual) {
                continue;
            }
            let mut dmm = open_mock_mode(*mode).unwrap();
            let m = dmm.request_measurement().unwrap();
            assert!(
                m.aux_values.is_empty(),
                "{mode:?} should not emit sub-values"
            );
        }
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

    /// Every group member must offer the whole group, with exactly one entry
    /// flagged as the live one.
    #[test]
    fn mode_choices_cover_the_live_scenario_group() {
        let transport = NullTransport;
        for group in MOCK_MODE_GROUPS {
            for mode in group {
                let mut proto = MockProtocol::with_mode(*mode);
                let m = proto.request_measurement(&transport).unwrap();
                let choices = proto.choices(Setting::Mode, &m);
                assert_eq!(choices.len(), group.len(), "{mode:?}");

                let current: Vec<&Choice> = choices.iter().filter(|c| c.current).collect();
                assert_eq!(current.len(), 1, "{mode:?} flagged {current:?} as current");
                assert_eq!(current[0].id, mode.choice_id().unwrap());

                let ids: Vec<u16> = choices.iter().map(|c| c.id).collect();
                let expected: Vec<u16> = group.iter().map(|m| m.choice_id().unwrap()).collect();
                assert_eq!(ids, expected, "{mode:?}");

                // The GUI shows these as popup entries, so two choices that
                // read the same are indistinguishable to the user.
                let labels: Vec<&str> = choices.iter().map(|c| c.label.as_ref()).collect();
                let mut unique = labels.clone();
                unique.sort_unstable();
                unique.dedup();
                assert_eq!(
                    unique.len(),
                    labels.len(),
                    "{mode:?} offers duplicate labels: {labels:?}"
                );
            }
        }
    }

    /// Labels use the same vocabulary as `Measurement::mode`, so a consumer
    /// can put a choice and the live reading's mode side by side.
    #[test]
    fn mode_choice_labels_match_the_measurement_mode() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::AcV);
        let m = proto.request_measurement(&transport).unwrap();
        for choice in proto.choices(Setting::Mode, &m) {
            let mut other = MockProtocol::with_mode(MockMode::ALL[choice.id as usize]);
            let reading = other.request_measurement(&transport).unwrap();
            assert_eq!(choice.label, reading.mode);
        }
    }

    /// Scenarios outside a group report nothing, which is how a consumer
    /// learns the control doesn't apply.
    #[test]
    fn ungrouped_scenarios_offer_no_mode_choices() {
        let transport = NullTransport;
        for mode in MockMode::ALL {
            if MOCK_MODE_GROUPS.iter().any(|g| g.contains(mode)) {
                continue;
            }
            let mut proto = MockProtocol::with_mode(*mode);
            let m = proto.request_measurement(&transport).unwrap();
            assert!(proto.choices(Setting::Mode, &m).is_empty(), "{mode:?}");
        }
    }

    #[test]
    fn select_mode_switches_the_live_scenario() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::Temp);
        let id = MockMode::TempDual.choice_id().unwrap();
        proto.select(&transport, Setting::Mode, id).unwrap();
        assert_eq!(proto.current_mode(), MockMode::TempDual);
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.aux_values.len(), 1, "temp2 emits its second probe");
    }

    /// Selecting a mode on an auto-cycling mock moves it there and lets the
    /// cycle continue, rather than pinning the scenario.
    #[test]
    fn select_mode_keeps_an_auto_cycling_mock_cycling() {
        let transport = NullTransport;
        let clock = Clock::manual();
        let mut proto = MockProtocol::new().with_clock(clock.clone());
        proto
            .select(
                &transport,
                Setting::Mode,
                MockMode::AcVHz.choice_id().unwrap(),
            )
            .unwrap();
        assert_eq!(proto.current_mode(), MockMode::AcVHz);
        // Run the scenario past its duration: the cycle must advance.
        clock.advance(Duration::from_secs(60));
        proto.request_measurement(&transport).unwrap();
        assert_ne!(proto.current_mode(), MockMode::AcVHz);
    }

    // --- Flag-backed settings ----------------------------------------------

    #[test]
    fn every_flag_setting_round_trips() {
        let transport = NullTransport;
        // AC V: the one scenario where all four settings do something.
        let mut proto = MockProtocol::with_mode(MockMode::AcV);
        for (setting, states) in [
            (Setting::Hold, &[1u16, 0][..]),
            (Setting::Rel, &[1, 0][..]),
            (Setting::MinMax, &[1, 2, 1, 0][..]),
            (Setting::Peak, &[1, 2, 1, 0][..]),
        ] {
            for &id in states {
                proto.select(&transport, setting, id).expect("switched");
                let m = proto.request_measurement(&transport).unwrap();
                let live: Vec<u16> = proto
                    .choices(setting, &m)
                    .iter()
                    .filter(|c| c.current)
                    .map(|c| c.id)
                    .collect();
                assert_eq!(live, vec![id], "{setting} -> {id}");
            }
        }
    }

    #[test]
    fn flag_choices_are_named_like_the_badges() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::AcV);
        let m = proto.request_measurement(&transport).unwrap();
        let labels = |setting| -> Vec<String> {
            proto
                .choices(setting, &m)
                .iter()
                .map(|c| c.label.to_string())
                .collect()
        };
        assert_eq!(labels(Setting::Hold), vec!["off", "on"]);
        assert_eq!(labels(Setting::MinMax), vec!["off", "MAX", "MIN"]);
        assert_eq!(labels(Setting::Peak), vec!["off", "P-MAX", "P-MIN"]);
    }

    /// The mock stands in for a UT61E+, which ignores Peak on DC.
    #[test]
    fn peak_is_not_offered_where_the_meter_ignores_it() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        let m = proto.request_measurement(&transport).unwrap();
        assert!(proto.choices(Setting::Peak, &m).is_empty());
        assert!(proto.select(&transport, Setting::Peak, 1).is_err());
        // The other three are still offered.
        assert_eq!(proto.choices(Setting::MinMax, &m).len(), 3);
    }

    #[test]
    fn selecting_peak_ends_minmax_and_the_other_way_round() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::AcV);
        proto.select(&transport, Setting::MinMax, 2).unwrap();
        proto.select(&transport, Setting::Peak, 1).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert!(m.flags.peak_max);
        assert!(
            !m.flags.min && !m.flags.max,
            "MIN/MAX ends when peak starts"
        );

        proto.select(&transport, Setting::MinMax, 1).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert!(m.flags.max);
        assert!(
            !m.flags.peak_max && !m.flags.peak_min,
            "peak ends when MIN/MAX starts"
        );
    }

    /// Selecting MIN/MAX locks the range, as the meter does while recording.
    #[test]
    fn minmax_selected_by_name_still_locks_the_range() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        proto.select(&transport, Setting::MinMax, 1).unwrap();
        let err = proto.select(&transport, Setting::Range, 2).unwrap_err();
        assert!(matches!(err, Error::CommandRejected(_)), "{err}");
        proto.select(&transport, Setting::MinMax, 0).unwrap();
        proto.select(&transport, Setting::Range, 2).expect("free");
    }

    // --- Range selection ---------------------------------------------------

    /// A manual rung relabels the range only: the value and its unit stay the
    /// scenario's, since swapping the unit alone would move the reading a
    /// decade.
    #[test]
    fn a_manual_rung_keeps_the_scenarios_value_and_unit() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::Hz);
        let before = proto.request_measurement(&transport).unwrap();
        proto.select(&transport, Setting::Range, 3).unwrap();
        let after = proto.request_measurement(&transport).unwrap();
        assert_eq!(after.unit, before.unit);
        assert_ne!(after.range_label, before.range_label, "the rung shows");
        assert!(!after.flags.auto_range);
    }

    #[test]
    fn range_choices_are_auto_plus_the_modes_ladder() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        let m = proto.request_measurement(&transport).unwrap();
        let choices = proto.choices(Setting::Range, &m);
        let listed: Vec<(u16, &str)> = choices.iter().map(|c| (c.id, c.label.as_ref())).collect();
        assert_eq!(
            listed,
            vec![
                (0, "Auto"),
                (1, "2.2V"),
                (2, "22V"),
                (3, "220V"),
                (4, "1000V")
            ]
        );
        assert_eq!(
            choices
                .iter()
                .filter(|c| c.current)
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![0],
            "the mock starts auto-ranging"
        );
    }

    /// Temperature and NCV have no ladder on the UT61E+ the mock stands in
    /// for, so it offers no range there either.
    #[test]
    fn a_scenario_without_a_ladder_offers_no_ranges() {
        let transport = NullTransport;
        for mode in [MockMode::Temp, MockMode::Ncv] {
            let mut proto = MockProtocol::with_mode(mode);
            let m = proto.request_measurement(&transport).unwrap();
            assert!(proto.choices(Setting::Range, &m).is_empty(), "{mode:?}");
        }
    }

    #[test]
    fn selecting_a_range_reports_it_and_auto_comes_back() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        proto.select(&transport, Setting::Range, 4).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.range_label, "1000V");
        assert_eq!(m.range_raw, 3);
        assert!(!m.flags.auto_range);
        let current: Vec<u16> = proto
            .choices(Setting::Range, &m)
            .into_iter()
            .filter(|c| c.current)
            .map(|c| c.id)
            .collect();
        assert_eq!(current, vec![4]);

        proto.select(&transport, Setting::Range, 0).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert!(m.flags.auto_range);
        assert_eq!(m.range_label, "22V", "back to the scenario's own range");
    }

    /// The RANGE button and `select` drive the same state: a press engages
    /// manual ranging where auto had left the meter, then steps the ladder.
    #[test]
    fn the_range_command_and_select_agree() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        proto.send_command(&transport, "range").unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert!(!m.flags.auto_range);
        assert_eq!(m.range_label, "22V", "manual at the rung auto had picked");

        proto.send_command(&transport, "range").unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.range_label, "220V");

        proto.select(&transport, Setting::Range, 3).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.range_label, "220V", "the same rung, chosen absolutely");

        proto.send_command(&transport, "auto").unwrap();
        assert!(
            proto
                .request_measurement(&transport)
                .unwrap()
                .flags
                .auto_range
        );
    }

    #[test]
    fn select_range_rejects_an_id_the_ladder_lacks() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        let err = proto.select(&transport, Setting::Range, 5).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCommand(_)),
            "got {err:?}, want UnsupportedCommand"
        );
    }

    /// The meter locks the range while MIN/MAX records (verified
    /// 2026-03-21), so the mock refuses a range change there.
    #[test]
    fn minmax_locks_the_range() {
        let transport = NullTransport;
        let mut proto = MockProtocol::with_mode(MockMode::DcV);
        proto.send_command(&transport, "minmax").unwrap();
        let err = proto.select(&transport, Setting::Range, 2).unwrap_err();
        assert!(
            matches!(err, Error::CommandRejected(_)),
            "got {err:?}, want CommandRejected"
        );
        // The RANGE button is equally dead while MIN/MAX holds the range.
        proto.send_command(&transport, "range").unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.range_label, "22V");
        // The lock only answers for rungs that exist; an id off the ladder is
        // still an unknown id, as it is with MIN/MAX off.
        let err = proto.select(&transport, Setting::Range, 5).unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCommand(_)),
            "got {err:?}, want UnsupportedCommand"
        );

        proto.send_command(&transport, "exit_minmax").unwrap();
        proto.select(&transport, Setting::Range, 2).unwrap();
        let m = proto.request_measurement(&transport).unwrap();
        assert_eq!(m.range_label, "22V");
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn select_mode_rejects_an_unknown_id() {
        let transport = NullTransport;
        let mut proto = MockProtocol::new();
        let err = proto
            .select(&transport, Setting::Mode, MockMode::ALL.len() as u16)
            .unwrap_err();
        assert!(
            matches!(err, Error::UnsupportedCommand(_)),
            "got {err:?}, want UnsupportedCommand"
        );
    }

    /// `from_choice_id` inverts `choice_id` for every mode, and gives nothing
    /// for an id past the table.
    #[test]
    fn choice_ids_round_trip() {
        for mode in MockMode::ALL {
            let id = mode.choice_id().expect("every mode has a choice id");
            assert_eq!(MockMode::from_choice_id(id), Some(*mode), "{mode:?}");
        }
        assert_eq!(MockMode::from_choice_id(MockMode::ALL.len() as u16), None);
    }
}
