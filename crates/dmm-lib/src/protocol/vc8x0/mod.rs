//! The driver the Voltcraft VC-880 and VC-890 protocol families share.
//!
//! Both use AB CD framing with BE16 checksums, the same command byte
//! assignments and the same DeviceID retrieval, and both drive their mode,
//! range and flag settings through [`crate::protocol::cycle`]. So
//! [`Vc8x0Protocol`] implements [`Protocol`] and [`CycleMeter`] once, and
//! each family supplies a [`Vc8x0Model`] with what actually differs: the
//! labels, the dial, function and range tables, the live-frame layout, and
//! the choreography each meter wraps its reads and writes in (the VC-880
//! streams and drains, the VC-890 polls behind an ack burst).
//!
//! The pieces the model implementations build on live here too:
//! [`RangeEntry`]/[`re`] and [`resolve_range`] for the range tables, and
//! [`resolve_function`], [`main_display`], [`common_flags`] and
//! [`parse_value`] for the steps of [`parse_measurement`].

pub(crate) mod vc880;
pub(crate) mod vc890;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::cycle::{
    self, CycleButton, CycleMeter, DialPosition, DialState, RANGE_BUTTON_NAME, Settle,
};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, Choice, DeviceFamily, DeviceProfile, Evidence, Fingerprint, Probing, Protocol,
    Setting, check_len, unknown_mode, unsupported_setting,
};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Build a data-less command frame: `[0xAB, 0xCD, 0x03, cmd, chk_hi, chk_lo]`.
pub(crate) const fn build_command(cmd: u8) -> [u8; 6] {
    framing::build_abcd_be16(cmd, &[])
}

/// The button the vendor DLL calls `Select` (spec §5, "SELECT button (cycle
/// sub-function)"). Both meters' front panel labels it SHIFT/SETUP, and it is
/// what steps a dial position through its sub-functions.
pub(crate) const CMD_SELECT: u8 = 0x4C;

/// What the front panel calls [`CMD_SELECT`], for messages the user reads.
pub(crate) const SELECT_BUTTON_NAME: &str = "SHIFT/SETUP";

/// The RANGE button: one press steps the manual range ladder (spec §5,
/// "RANGE button"). Whether repeated presses really step it one rung at a
/// time is unverified on both meters — see the VC-880 and VC-890 sections of
/// docs/verification-backlog.md — which is why the range driver reads the
/// range back after every press rather than counting them.
pub(crate) const CMD_RANGE_MANUAL: u8 = 0x46;

/// Return to auto-ranging: its own command, as it is its own button.
pub(crate) const CMD_RANGE_AUTO: u8 = 0x47;

/// HOLD, REL and the MAX/MIN/AVG pair (spec §5). The MAX/MIN/AVG button
/// steps the states and only 0x43 leaves them, exactly as the UT61+ family's
/// 0x41/0x42 pair does.
pub(crate) const CMD_HOLD: u8 = 0x4A;
pub(crate) const CMD_REL: u8 = 0x48;
pub(crate) const CMD_MAX_MIN_AVG: u8 = 0x49;
pub(crate) const CMD_EXIT_MAX_MIN_AVG: u8 = 0x43;

/// What the front panels call the buttons the flag-backed settings press.
pub(crate) const HOLD_BUTTON_NAME: &str = "HOLD";
pub(crate) const REL_BUTTON_NAME: &str = "REL";
pub(crate) const MAX_MIN_AVG_BUTTON_NAME: &str = "MAX/MIN/AVG";

/// The command byte one press of `button` sends, or `None` for a button
/// neither Voltcraft meter has.
pub(crate) fn press_command(button: cycle::CycleButton) -> Option<u8> {
    Some(match button {
        cycle::CycleButton::Select => CMD_SELECT,
        cycle::CycleButton::Range => CMD_RANGE_MANUAL,
        cycle::CycleButton::Hold => CMD_HOLD,
        cycle::CycleButton::Rel => CMD_REL,
        cycle::CycleButton::MinMax => CMD_MAX_MIN_AVG,
        // No Hz/% button (the SHIFT/SETUP ring reaches Hz), and no Peak
        // function at all: the vendor command table lists neither.
        cycle::CycleButton::Hz | cycle::CycleButton::Peak => return None,
    })
}

/// What the front panel calls `button`, for the messages the user reads.
pub(crate) fn button_name(button: cycle::CycleButton) -> &'static str {
    match button {
        cycle::CycleButton::Select => SELECT_BUTTON_NAME,
        cycle::CycleButton::Range => RANGE_BUTTON_NAME,
        cycle::CycleButton::Hold => HOLD_BUTTON_NAME,
        cycle::CycleButton::Rel => REL_BUTTON_NAME,
        cycle::CycleButton::MinMax => MAX_MIN_AVG_BUTTON_NAME,
        cycle::CycleButton::Hz => "Hz/%",
        cycle::CycleButton::Peak => "PEAK",
    }
}

/// The states each flag-backed setting offers; identical on both meters.
///
/// HOLD and REL toggle. MAX/MIN/AVG is a ring of three plus off, its press
/// order unverified — which is why the driver reads the flags back after
/// every press instead of counting them. Neither meter has Peak.
pub(crate) fn flag_states(setting: cycle::FlagSetting) -> &'static [u16] {
    match setting {
        cycle::FlagSetting::Hold | cycle::FlagSetting::Rel => &[0, 1],
        cycle::FlagSetting::MinMax => &[0, 1, 2, 3],
        cycle::FlagSetting::Peak => &[],
    }
}

/// Map a command name to its byte value.
///
/// Command bytes are identical for VC-880 and VC-890.
pub(crate) fn command_byte(command: &str) -> Result<u8> {
    match command {
        "hold" => Ok(CMD_HOLD),
        "rel" => Ok(CMD_REL),
        "max_min_avg" => Ok(CMD_MAX_MIN_AVG),
        "exit_max_min_avg" => Ok(CMD_EXIT_MAX_MIN_AVG),
        "range_auto" => Ok(CMD_RANGE_AUTO),
        "range_manual" => Ok(CMD_RANGE_MANUAL),
        "light" => Ok(0x4B),
        "select" => Ok(CMD_SELECT),
        _ => Err(Error::UnsupportedCommand(command.to_string())),
    }
}

/// Send the GetDeviceID command (0x00) and read the 20-byte ASCII name.
pub(crate) fn read_device_name(
    rx_buf: &mut Vec<u8>,
    transport: &dyn Transport,
    label: &str,
) -> Result<Option<String>> {
    let frame = build_command(0x00);
    debug!("{label}: sending GetDeviceID command");
    transport.write(&frame)?;

    match framing::read_frame(
        rx_buf,
        transport,
        framing::extract_frame_abcd_be16,
        |p| !p.is_empty() && p[0] == 0x00, // DeviceID type
        FrameErrorRecovery::SkipAndRetry,
        &format!("{label}-id"),
        &framing::HEADER,
    ) {
        Ok(payload) if payload.len() >= 21 => {
            let name = String::from_utf8_lossy(&payload[1..21]).trim().to_string();
            debug!("{label}: device name: {name}");
            if name.is_empty() {
                Ok(None)
            } else {
                Ok(Some(name))
            }
        }
        Ok(_) => {
            debug!("{label}: DeviceID response too short");
            Ok(None)
        }
        Err(e) => {
            debug!("{label}: failed to read DeviceID: {e}");
            Ok(None)
        }
    }
}

/// Button commands, identical on both meters.
pub(crate) const COMMANDS: &[&str] = &[
    "hold",
    "rel",
    "max_min_avg",
    "exit_max_min_avg",
    "range_auto",
    "range_manual",
    "light",
    "select",
];

/// Capture steps shared by the VC-880 and VC-890.
///
/// Both meters expose the same dial positions, so the list lived as ~120
/// duplicated lines in each protocol. Device-specific steps are appended by
/// the caller.
pub(crate) fn capture_steps() -> Vec<CaptureStep> {
    use crate::protocol::steps::{self, Ohms, Volts};
    use crate::protocol::{Expect, Need};

    let [dcv, dcv_short, dcv_negative, ohm, ohm_body, ohm_short] = steps::gate_steps(
        Volts::DcV,
        CaptureStep::basic("dcv", "Set meter to DC V"),
        Ohms::Word,
        CaptureStep::basic(
            "ohm",
            "Set meter to Resistance (Ω). Leave leads open (should show OL).",
        ),
    );

    vec![
        dcv,
        dcv_short,
        dcv_negative,
        CaptureStep::basic("acv", "Set meter to AC V").expect(Expect::mode("AC V")),
        CaptureStep::basic("acdcv", "Set meter to AC+DC V").expect(Expect::mode("AC+DC V")),
        CaptureStep::basic("dcmv", "Set meter to DC mV").expect(Expect::mode("DC mV")),
        CaptureStep::basic("dcua", "Set meter to DC µA").expect(Expect::mode("DC µA")),
        CaptureStep::basic("acua", "Set meter to AC µA").expect(Expect::mode("AC µA")),
        CaptureStep::basic("dcma", "Set meter to DC mA").expect(Expect::mode("DC mA")),
        CaptureStep::basic("acma", "Set meter to AC mA").expect(Expect::mode("AC mA")),
        CaptureStep::basic("dca", "Set meter to DC A").expect(Expect::mode("DC A")),
        CaptureStep::basic("aca", "Set meter to AC A").expect(Expect::mode("AC A")),
        ohm,
        ohm_body,
        ohm_short,
        CaptureStep::basic("cont", "Set meter to Continuity").expect(Expect::mode("Continuity")),
        CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
        CaptureStep::basic("cap", "Set meter to Capacitance").expect(Expect::mode("Capacitance")),
        CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Frequency")),
        CaptureStep::basic("duty", "Set meter to Duty Cycle (%)").expect(Expect::mode("Duty %")),
        CaptureStep::basic("tempc", "Set meter to Temperature °C")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°C")),
        CaptureStep::basic("tempf", "Set meter to Temperature °F")
            .needs(&[Need::Thermocouple])
            .expect(Expect::mode("°F")),
        CaptureStep::basic("lpf", "Set meter to ACV Low-Pass Filter")
            .expect(Expect::mode("ACV LPF")),
    ]
}

/// Range table entry: `unit_override` replaces the function's base unit
/// when non-empty. `range_label` is a human-readable string like "40kΩ".
pub(crate) struct RangeEntry {
    unit_override: &'static str,
    range_label: &'static str,
}

/// Shorthand for a [`RangeEntry`] literal, so a range table reads as one
/// line per range.
pub(crate) const fn re(unit_override: &'static str, range_label: &'static str) -> RangeEntry {
    RangeEntry {
        unit_override,
        range_label,
    }
}

/// Index a range table and resolve the entry's unit override against the
/// function's base unit from `function_table`.
///
/// Returns (unit, range_label), or None if the range index is out of bounds.
pub(crate) fn resolve_range(
    table: &[RangeEntry],
    range_idx: u8,
    function_table: &[(u8, &'static str, &'static str)],
    function: u8,
) -> Option<(&'static str, &'static str)> {
    table.get(range_idx as usize).map(|e| {
        let unit = if e.unit_override.is_empty() {
            // Use the function's base unit
            function_table
                .iter()
                .find(|(c, _, _)| *c == function)
                .map(|(_, _, u)| *u)
                .unwrap_or("")
        } else {
            e.unit_override
        };
        (unit, e.range_label)
    })
}

/// The manual range ladder of a function's range table, for
/// `Setting::Range`. Rung `n` of the choice list is entry `n - 1`, which is
/// what the meter reports as `range_raw - 0x30`.
pub(crate) fn range_ladder(table: &'static [RangeEntry]) -> Vec<Cow<'static, str>> {
    cycle::usable_ladder(table.iter().map(|e| Cow::Borrowed(e.range_label)).collect())
}

/// Look a function code up in a family's function table.
///
/// Returns (mode name, base unit); an unrecognised code becomes a generic
/// `unknown_mode` label with an empty base unit.
pub(crate) fn resolve_function(
    table: &[(u8, &'static str, &'static str)],
    code: u8,
    family: &str,
) -> (Cow<'static, str>, &'static str) {
    if let Some((_, name, unit)) = table.iter().find(|(c, _, _)| *c == code) {
        (Cow::Borrowed(*name), *unit)
    } else {
        debug!("{family}: unknown function code {code:#04x}");
        (unknown_mode(code), "")
    }
}

/// Decode the ASCII main display field.
///
/// Returns (raw string, same string with all whitespace removed).
pub(crate) fn main_display(bytes: &[u8]) -> (String, String) {
    let raw = String::from_utf8_lossy(bytes).to_string();
    let trimmed: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    (raw, trimmed)
}

/// Extract the status flags both families encode in the same bits of
/// status bytes 1-3, plus the OL1 (primary overload) bit.
///
/// Callers add their family-specific flags to the returned `StatusFlags`.
pub(crate) fn common_flags(status: &[u8]) -> (StatusFlags, bool) {
    // Status byte 2: bit2 = OL1 (primary overload)
    let ol1 = status[2] & 0x04 != 0;

    let flags = StatusFlags {
        // Status byte 2: bit0=Hold, bit1=Manual
        hold: status[2] & 0x01 != 0,
        // Status byte 1: bit0=Rel, bit1=Avg, bit2=Min, bit3=Max
        // (vc880 spec byte 31, tables at lines 117 and 272; vc890 spec byte 57,
        // line 95 — both families share this byte layout.)
        rel: status[1] & 0x01 != 0,
        avg: status[1] & 0x02 != 0,
        min: status[1] & 0x04 != 0,
        max: status[1] & 0x08 != 0,
        auto_range: status[2] & 0x02 == 0, // Manual bit: 0=auto, 1=manual
        // Status byte 3: bit1=Warning
        hv_warning: status[3] & 0x02 != 0,
        dc: false,
        peak_max: false,
        peak_min: false,
        ..Default::default()
    };

    (flags, ol1)
}

/// Turn the decoded main display into a `MeasuredValue`.
///
/// `trimmed` is the whitespace-stripped display. `setup` says the meter is on
/// its setup screen, whose words are not a reading and are not reported.
pub(crate) fn parse_value(
    family: &'static str,
    ol1: bool,
    trimmed: &str,
    setup: bool,
) -> MeasuredValue {
    if ol1 || trimmed.contains("OL") || trimmed.contains("---") {
        return MeasuredValue::Overload;
    }
    match trimmed.parse::<f64>() {
        // Sign is in the ASCII string itself (leading '-')
        Ok(v) => MeasuredValue::Normal(v),
        Err(_) => {
            if !setup {
                report_unknown(
                    family,
                    "display text",
                    format_args!("{trimmed:?}, shown as OL"),
                );
            }
            MeasuredValue::Overload
        }
    }
}

/// Live-data message type byte: what both meters mark a reading frame with.
pub(crate) const MSG_TYPE_LIVE_DATA: u8 = 0x01;

/// Read one live-data frame off the wire.
///
/// The accept filter is what makes the meters' other frames — the 0xFF
/// command result, the DeviceID answer — cost nothing: they are skipped
/// here rather than drained by the caller.
pub(crate) fn read_live(
    rx_buf: &mut Vec<u8>,
    transport: &dyn Transport,
    label: &str,
) -> Result<Vec<u8>> {
    framing::read_frame(
        rx_buf,
        transport,
        framing::extract_frame_abcd_be16,
        |p| !p.is_empty() && p[0] == MSG_TYPE_LIVE_DATA,
        FrameErrorRecovery::SkipAndRetry,
        label,
        &framing::HEADER,
    )
}

/// What one Voltcraft meter's driver has to say for itself.
///
/// Everything a [`Vc8x0Protocol`] does that both meters do the same way is
/// written once against this trait; a model is only the differences.
///
/// `Send + 'static` because the driver carrying it is a `Box<dyn Protocol>`,
/// which the CLI and GUI hand to their reader threads.
pub(crate) trait Vc8x0Model: Send + 'static {
    /// Log target, and the family label warnings and parse errors quote.
    const LOG: &'static str;

    /// What the front panel calls this meter, for the errors a user reads.
    const NAME: &'static str;

    /// The [`crate::protocol::registry`] id detection pins for this meter.
    ///
    /// The VC650BT shares the VC-880's protocol byte for byte, so a frame
    /// cannot tell the two apart and detection always reports `"vc880"`;
    /// `--device vc650bt` carries the other name, with the same tables.
    const DETECTED_ID: &'static str;

    /// Length of a live-data payload — everything the frame extractor hands
    /// back, between the length byte and the checksum.
    const PAYLOAD_LEN: usize;

    /// Where the status bytes start in that payload; they run to its end.
    const STATUS_AT: usize;

    /// The dial table the mode driver plans its presses over.
    const DIAL: &'static [DialPosition];

    /// How long a press takes to show up in a reading.
    const SETTLE: Settle;

    /// Function code table: (code, mode name, base unit).
    const FUNCTION_TABLE: &'static [(u8, &'static str, &'static str)];

    /// The profile a driver for this meter reports.
    fn profile() -> DeviceProfile;

    /// The range table of a function code, empty for a single-range or
    /// unknown function.
    fn range_table(function: u8) -> &'static [RangeEntry];

    /// Unit and range label for a function's range index, `None` when the
    /// index is past that function's table.
    fn lookup_range(function: u8, range_idx: u8) -> Option<(&'static str, &'static str)> {
        resolve_range(
            Self::range_table(function),
            range_idx,
            Self::FUNCTION_TABLE,
            function,
        )
    }

    /// Set the flags [`common_flags`] does not: each meter puts a few of its
    /// own beyond the bits the two share.
    fn extra_flags(flags: &mut StatusFlags, status: &[u8]);

    /// Whether the status bytes say the meter shows its setup screen.
    fn setup_screen(_status: &[u8]) -> bool {
        false
    }

    /// Capture steps this meter needs on top of [`capture_steps`].
    fn extra_capture_steps() -> Vec<CaptureStep> {
        Vec::new()
    }

    /// The vendor ack burst the VC-890 brackets every exchange with. The
    /// VC-880 needs none, which is what the default does.
    fn ack(_transport: &dyn Transport) -> Result<()> {
        Ok(())
    }

    /// Take one live-data frame, with whatever the meter needs around the
    /// read: the VC-880 streams, the VC-890 has to be asked.
    fn read_live_frame(rx_buf: &mut Vec<u8>, transport: &dyn Transport) -> Result<Vec<u8>>;

    /// Write one button press, with whatever the meter needs around the
    /// frame so that the reading which follows describes the new state.
    fn write_button(rx_buf: &mut Vec<u8>, transport: &dyn Transport, cmd: u8) -> Result<()>;
}

/// Protocol implementation for both Voltcraft families, `M` supplying what
/// differs. Each family exports it under its own name.
pub(crate) struct Vc8x0Protocol<M: Vc8x0Model> {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// Which dial position the readings say the meter is on, for the mode
    /// driver in [`crate::protocol::cycle`].
    dial: DialState,
    model: PhantomData<M>,
}

impl<M: Vc8x0Model> Default for Vc8x0Protocol<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Vc8x0Model> Vc8x0Protocol<M> {
    pub(crate) fn new() -> Self {
        Self::with_profile(M::profile())
    }

    /// Build the driver with a profile the family adjusted — the VC-880 one
    /// serves two model names.
    pub(crate) fn with_profile(profile: DeviceProfile) -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile,
            dial: DialState::default(),
            model: PhantomData,
        }
    }

    /// The frame-reassembly buffer, for the test that checks a press drops
    /// what the stream had already queued.
    #[cfg(test)]
    pub(crate) fn rx_buf(&mut self) -> &mut Vec<u8> {
        &mut self.rx_buf
    }
}

impl<M: Vc8x0Model> Protocol for Vc8x0Protocol<M> {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // Neither meter needs a trigger: the VC-880 streams once the user
        // presses the PC button, and the VC-890 answers one request at a
        // time.
        debug!("{}: init (no trigger needed)", M::LOG);
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let payload = M::read_live_frame(&mut self.rx_buf, transport)?;
        let measurement = parse_measurement::<M>(&payload)?;
        // Readings are the only place the meter states its function, so
        // every one of them is what keeps the dial position current.
        self.dial.observe(M::DIAL, measurement.mode_raw);
        Ok(measurement)
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement::<M>(payload)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let cmd_byte = command_byte(command)?;
        debug!("{}: sending command {command} ({cmd_byte:#04x})", M::LOG);
        M::ack(transport)?;
        transport.write(&build_command(cmd_byte))
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        M::ack(transport)?;
        read_device_name(&mut self.rx_buf, transport, M::LOG)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        let mut steps = capture_steps();
        steps.extend(M::extra_capture_steps());
        steps
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        cycle::choices(self, setting, current)
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        cycle::select(self, transport, setting, id)
    }
}

impl<M: Vc8x0Model> CycleMeter for Vc8x0Protocol<M> {
    fn dial_positions(&self) -> &'static [DialPosition] {
        M::DIAL
    }

    fn dial_state(&self) -> &DialState {
        &self.dial
    }

    fn dial_state_mut(&mut self) -> &mut DialState {
        &mut self.dial
    }

    fn press(&mut self, transport: &dyn Transport, button: CycleButton) -> Result<()> {
        let name = self.button_name(button);
        let Some(cmd) = press_command(button) else {
            return Err(Error::UnsupportedCommand(format!(
                "the {} has no {name} button",
                M::NAME
            )));
        };
        debug!("{}: pressing {name} ({cmd:#04x})", M::LOG);
        M::write_button(&mut self.rx_buf, transport, cmd)
    }

    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        Protocol::request_measurement(self, transport)
    }

    fn mode_label(&self, mode: u16) -> Cow<'static, str> {
        resolve_function(M::FUNCTION_TABLE, mode as u8, M::LOG).0
    }

    fn settle(&self) -> Settle {
        M::SETTLE
    }

    fn button_name(&self, button: CycleButton) -> &'static str {
        button_name(button)
    }

    fn range_ladder(&self, mode: u16) -> Vec<Cow<'static, str>> {
        match u8::try_from(mode) {
            Ok(function) => range_ladder(M::range_table(function)),
            Err(_) => Vec::new(),
        }
    }

    /// The range byte is 0x30-based ASCII, so rung 1 arrives as 0x30.
    fn range_rung(&self, range_raw: u8) -> u16 {
        u16::from(range_raw.wrapping_sub(0x30)) + 1
    }

    fn set_auto_range(&mut self, transport: &dyn Transport) -> Result<()> {
        debug!("{}: sending auto-range ({CMD_RANGE_AUTO:#04x})", M::LOG);
        M::write_button(&mut self.rx_buf, transport, CMD_RANGE_AUTO)
    }

    fn flag_states(&self, setting: cycle::FlagSetting, _mode: u16) -> &'static [u16] {
        flag_states(setting)
    }

    fn exit_flag(&mut self, transport: &dyn Transport, setting: cycle::FlagSetting) -> Result<()> {
        match setting {
            cycle::FlagSetting::MinMax => {
                debug!(
                    "{}: exiting MAX/MIN/AVG ({CMD_EXIT_MAX_MIN_AVG:#04x})",
                    M::LOG
                );
                M::write_button(&mut self.rx_buf, transport, CMD_EXIT_MAX_MIN_AVG)
            }
            // HOLD and REL press their own button back off, and neither
            // meter has Peak, so the driver never asks this of them.
            other => Err(unsupported_setting(other.setting())),
        }
    }
}

/// Parse one live-data payload into a [`Measurement`].
///
/// The steps are the same on both meters — length guard, function code,
/// range byte, main display, status flags, value — and the header they open
/// with is at the same offsets: type byte, function, range, then the 7 ASCII
/// bytes of the main display. What differs is where the status bytes start
/// and which extra flags they carry, both of which the model states; each
/// family module maps its own payload byte for byte.
pub(crate) fn parse_measurement<M: Vc8x0Model>(payload: &[u8]) -> Result<Measurement> {
    check_len(M::LOG, payload, M::PAYLOAD_LEN)?;

    let function_code = payload[1];
    let range_raw = payload[2];
    let main_bytes = &payload[3..10];
    let status_bytes = &payload[M::STATUS_AT..M::PAYLOAD_LEN];

    let (mode, base_unit) = resolve_function(M::FUNCTION_TABLE, function_code, M::LOG);

    // Decode the range byte (0x30-based ASCII).
    let range_idx = range_raw.wrapping_sub(0x30);
    let (unit, range_label) = match M::lookup_range(function_code, range_idx) {
        Some((u, r)) => (u, r),
        // Single-range function or unknown range — use the base unit.
        None => (base_unit, ""),
    };

    let (display_str, display_trimmed) = main_display(main_bytes);

    let (mut flags, ol1) = common_flags(status_bytes);
    M::extra_flags(&mut flags, status_bytes);

    let value = parse_value(M::LOG, ol1, &display_trimmed, M::setup_screen(status_bytes));

    Ok(Measurement {
        mode,
        mode_raw: function_code as u16,
        range_raw,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        display_raw: Some(display_str),
        flags,
        ..Measurement::from_payload(payload)
    })
}

/// Detection for the VC-880: the meter streams live frames unprompted once
/// its PC button is pressed, so there is nothing to send.
pub(crate) static VC880_FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Vc880,
    label: "vc-880 stream",
    trigger: None,
    send_after: &[],
    checksummed: true,
    recognise: recognise_vc880,
};

/// Detection for the VC-890: the meter answers nothing but the `0x5E` poll
/// behind the vendor's ack burst, which is what the trigger sends.
pub(crate) static VC890_FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Vc890,
    label: "vc-890 poll",
    trigger: Some(vc890::request_live),
    send_after: &[],
    checksummed: true,
    recognise: recognise_vc890,
};

fn recognise_vc880(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    recognise_live::<vc880::Vc880Model>(buf)
}

fn recognise_vc890(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    recognise_live::<vc890::Vc890Model>(buf)
}

/// A live-data frame of `M`'s length: the type byte is the same on both
/// meters, so only the payload length tells them apart.
fn recognise_live<M: Vc8x0Model>(buf: &[u8]) -> Option<Evidence> {
    for start in framing::abcd_header_offsets(buf) {
        let Ok(Some((payload, _))) = framing::extract_frame_abcd_be16(&buf[start..]) else {
            continue;
        };
        if payload.first() == Some(&MSG_TYPE_LIVE_DATA) && payload.len() == M::PAYLOAD_LEN {
            return Some(Evidence::Model {
                id: M::DETECTED_ID,
                reported_name: None,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A live frame payload: the type byte, then display fields the
    /// recogniser never looks at.
    fn live_payload(len: usize) -> Vec<u8> {
        let mut payload = vec![b'0'; len];
        payload[0] = MSG_TYPE_LIVE_DATA;
        payload
    }

    fn recognise(buf: &[u8]) -> Option<Evidence> {
        super::recognise_vc880(buf, &Probing::default())
            .or_else(|| super::recognise_vc890(buf, &Probing::default()))
    }

    /// Only the payload length separates the two meters' live frames.
    #[test]
    fn the_payload_length_picks_the_meter() {
        for (len, id) in [
            (vc880::Vc880Model::PAYLOAD_LEN, "vc880"),
            (vc890::Vc890Model::PAYLOAD_LEN, "vc890"),
        ] {
            assert_eq!(
                recognise(&framing::test_frame_be16(&live_payload(len))),
                Some(Evidence::Model {
                    id,
                    reported_name: None,
                }),
                "a {len}-byte live payload is a {id}"
            );
        }
    }

    /// Any other length is some other family's frame, or junk.
    #[test]
    fn other_lengths_are_not_voltcraft_frames() {
        for len in [1, 14, 33, 35, 60, 62] {
            assert_eq!(
                recognise(&framing::test_frame_be16(&live_payload(len))),
                None,
                "a {len}-byte payload is not a live frame"
            );
        }
    }

    #[test]
    fn build_command_checksum() {
        let frame = build_command(0x4A);
        assert_eq!(frame.len(), 6);
        assert_eq!(&frame[..4], &[0xAB, 0xCD, 0x03, 0x4A]);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    #[test]
    fn command_byte_known() {
        // The range driver presses the same byte "range_manual" sends.
        assert_eq!(command_byte("range_manual").unwrap(), CMD_RANGE_MANUAL);
        assert_eq!(command_byte("range_auto").unwrap(), CMD_RANGE_AUTO);
        assert_eq!(command_byte("hold").unwrap(), 0x4A);
        assert_eq!(command_byte("rel").unwrap(), 0x48);
        // The flag-setting driver presses the same bytes.
        assert_eq!(press_command(cycle::CycleButton::Hold), Some(0x4A));
        assert_eq!(press_command(cycle::CycleButton::Rel), Some(0x48));
        assert_eq!(press_command(cycle::CycleButton::MinMax), Some(0x49));
        assert_eq!(command_byte("max_min_avg").unwrap(), CMD_MAX_MIN_AVG);
        assert_eq!(
            command_byte("exit_max_min_avg").unwrap(),
            CMD_EXIT_MAX_MIN_AVG
        );
        // Neither meter has these two.
        assert_eq!(press_command(cycle::CycleButton::Hz), None);
        assert_eq!(press_command(cycle::CycleButton::Peak), None);
        assert_eq!(command_byte("light").unwrap(), 0x4B);
        // The mode driver presses the same byte the "select" command sends.
        assert_eq!(command_byte("select").unwrap(), CMD_SELECT);
    }

    #[test]
    fn command_byte_unknown() {
        assert!(command_byte("nonexistent").is_err());
    }
}

/// The test bodies both families share, in the shape
/// [`cycle::assert_table_invariants`] set: an assertion helper the family's
/// own `#[test]` calls, so a failure still names the meter it broke on.
///
/// What differs between the two — the function codes, the range labels —
/// is an argument. A test whose *content* is those numbers (the parse and
/// snapshot cases, the range table, each meter's write choreography) stays
/// in its family's module.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::transport::mock::MockTransport;

    /// Build a live-data payload for `M`: the header, the 7-byte main
    /// display, blanks through the display fields the parser ignores, then
    /// `status`.
    pub(crate) fn make_payload<M: Vc8x0Model>(
        function: u8,
        range: u8,
        main_display: &[u8; 7],
        status: &[u8],
    ) -> Vec<u8> {
        let mut p = vec![MSG_TYPE_LIVE_DATA, function, range];
        p.extend_from_slice(main_display);
        p.resize(M::STATUS_AT, b' ');
        p.extend_from_slice(status);
        assert_eq!(p.len(), M::PAYLOAD_LEN, "status block is the wrong length");
        p
    }

    /// A status block with every bit clear.
    pub(crate) fn zero_status<M: Vc8x0Model>() -> Vec<u8> {
        vec![0u8; M::PAYLOAD_LEN - M::STATUS_AT]
    }

    /// A status block with the MANUAL range bit set (status byte 2, bit 1).
    pub(crate) fn manual_status<M: Vc8x0Model>() -> Vec<u8> {
        let mut s = zero_status::<M>();
        s[2] |= 0x02;
        s
    }

    /// One reading through `request_measurement`, the way the mode driver
    /// gets its picture of the dial.
    ///
    /// The frame is fed in two chunks: `read_frame` reads at most 64 bytes
    /// at a time and a VC-890 live-data frame is 66.
    pub(crate) fn read_one<M: Vc8x0Model>(function: u8) -> (Vc8x0Protocol<M>, Measurement) {
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &zero_status::<M>());
        let frame = framing::test_frame_be16(&payload);
        let (head, tail) = frame.split_at(32);
        let transport = MockTransport::new(vec![head.to_vec(), tail.to_vec()]);
        let mut proto = Vc8x0Protocol::<M>::new();
        let m = proto
            .request_measurement(&transport)
            .expect("the frame parses");
        (proto, m)
    }

    pub(crate) fn ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().map(|c| c.id).collect()
    }

    pub(crate) fn labels(choices: &[Choice]) -> Vec<String> {
        choices.iter().map(|c| c.label.to_string()).collect()
    }

    pub(crate) fn current_ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().filter(|c| c.current).map(|c| c.id).collect()
    }

    /// A payload far shorter than the frame is rejected, not indexed.
    pub(crate) fn assert_short_payload_rejected<M: Vc8x0Model>() {
        let payload = vec![0x01, 0x00, 0x30]; // way too short
        assert!(parse_measurement::<M>(&payload).is_err());
    }

    /// Every code in the model's function table decodes to a named mode.
    pub(crate) fn assert_all_functions_named<M: Vc8x0Model>() {
        for &(code, _, _) in M::FUNCTION_TABLE {
            let payload = make_payload::<M>(code, 0x30, b"  1.234", &zero_status::<M>());
            let m = parse_measurement::<M>(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "function {code:#04x} should be known"
            );
        }
    }

    /// OL1 — status byte 2, bit 2 — reads as overload.
    pub(crate) fn assert_overload_flag<M: Vc8x0Model>(function: u8) {
        let mut status = zero_status::<M>();
        status[2] = 0x04; // OL1
        let payload = make_payload::<M>(function, 0x30, b"     OL", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    /// Status byte 2, bit 0 = Hold.
    pub(crate) fn assert_hold_flag<M: Vc8x0Model>(function: u8) {
        let mut status = zero_status::<M>();
        status[2] = 0x01; // Hold bit
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(m.flags.hold);
    }

    /// Status byte 1, bit 0 = Rel.
    pub(crate) fn assert_rel_flag<M: Vc8x0Model>(function: u8) {
        let mut status = zero_status::<M>();
        status[1] = 0x01; // Rel bit
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(m.flags.rel);
    }

    /// Status byte 1: bit 3 = Max, bit 2 = Min, one at a time.
    pub(crate) fn assert_max_min_flags<M: Vc8x0Model>(function: u8) {
        let mut status = zero_status::<M>();
        status[1] = 0x08; // Max bit
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);

        let mut status = zero_status::<M>();
        status[1] = 0x04; // Min bit
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(!m.flags.max);
        assert!(m.flags.min);
    }

    /// Status byte 1, bit 1 = Avg, the third step of the meter's
    /// MAX/MIN/AVG cycle. Rel/Min/Max share the byte and stay off.
    pub(crate) fn assert_avg_flag<M: Vc8x0Model>(function: u8) {
        let mut status = zero_status::<M>();
        status[1] = 0x02;
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &status);
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(m.flags.avg);
        assert!(!m.flags.rel);
        assert!(!m.flags.min);
        assert!(!m.flags.max);
    }

    /// The Manual bit (status byte 2, bit 1) is inverted into `auto_range`.
    pub(crate) fn assert_auto_range<M: Vc8x0Model>(function: u8) {
        // Manual bit clear = auto range
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &zero_status::<M>());
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(m.flags.auto_range);

        // Manual bit set = manual range
        let payload = make_payload::<M>(function, 0x30, b"  1.234", &manual_status::<M>());
        let m = parse_measurement::<M>(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    /// The whole range table as one string: every function code crossed
    /// with every range index, rendered `unit|range_label` (`-` where the
    /// lookup returns None).
    pub(crate) fn render_range_table<M: Vc8x0Model>() -> String {
        (0x00u8..=0x12)
            .map(|function| {
                let cells: Vec<String> = (0u8..=8)
                    .map(|range_idx| match M::lookup_range(function, range_idx) {
                        Some((unit, label)) => format!("{unit}|{label}"),
                        None => "-".to_string(),
                    })
                    .collect();
                format!("{function:#04x}: {}", cells.join(" "))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The frame a named command sends: header, command byte, BE16 checksum.
    pub(crate) fn assert_command_frame(cmd: u8) {
        let frame = build_command(cmd);
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xCD);
        assert_eq!(frame[2], 0x03);
        assert_eq!(frame[3], cmd);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    /// The dial table holds together: `shared_modes` are the codes this
    /// meter deliberately reports from more than one position.
    pub(crate) fn assert_dial_table_is_well_formed<M: Vc8x0Model>(shared_modes: &[u16]) {
        cycle::assert_table_invariants(
            M::DIAL,
            shared_modes,
            // Neither meter has an Hz/% button; SHIFT/SETUP is the only one.
            &[CycleButton::Select],
            &|mode| resolve_function(M::FUNCTION_TABLE, mode as u8, M::LOG).0,
        );
    }

    /// A reading records the mode and the dial position it implies.
    pub(crate) fn assert_reading_records_dial_position<M: Vc8x0Model>(
        function: u8,
        position: usize,
    ) {
        let (proto, _) = read_one::<M>(function);
        assert_eq!(proto.dial_state().last_mode(), Some(u16::from(function)));
        assert_eq!(
            proto.dial_state().position(),
            Some(position),
            "the position {function:#04x} sits on"
        );
    }

    /// The Ω position offers diode and continuity too, in `expected` order.
    pub(crate) fn assert_ohm_position_lists_diode_and_continuity<M: Vc8x0Model>(
        ohm: u8,
        expected: &[u16],
    ) {
        let (proto, m) = read_one::<M>(ohm);
        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(ids(&choices), expected);
        assert_eq!(labels(&choices), ["Ω", "Diode", "Continuity"]);
        assert_eq!(current_ids(&choices), vec![u16::from(ohm)]);
    }

    /// The resistance ladder is offered with Auto first, and a reading on
    /// the third rung marks that rung current.
    pub(crate) fn assert_resistance_ladder<M: Vc8x0Model>(ohm: u8, rungs: &[&str]) {
        // Range byte 0x32 is the third rung.
        let m = parse_measurement::<M>(&make_payload::<M>(
            ohm,
            0x32,
            b" 12.345",
            &manual_status::<M>(),
        ))
        .expect("the frame parses");
        let proto = Vc8x0Protocol::<M>::new();
        let choices = proto.choices(Setting::Range, &m);
        assert_eq!(
            ids(&choices),
            (0..=rungs.len() as u16).collect::<Vec<_>>(),
            "Auto plus one id per rung"
        );
        let expected: Vec<&str> = std::iter::once("Auto")
            .chain(rungs.iter().copied())
            .collect();
        assert_eq!(labels(&choices), expected);
        assert_eq!(current_ids(&choices), vec![3]);
    }

    /// A function with one range or none offers no range choice at all.
    pub(crate) fn assert_no_range_choices<M: Vc8x0Model>(functions: &[u8]) {
        let proto = Vc8x0Protocol::<M>::new();
        for &function in functions {
            let m = parse_measurement::<M>(&make_payload::<M>(
                function,
                0x30,
                b"  1.234",
                &manual_status::<M>(),
            ))
            .expect("the frame parses");
            assert!(
                proto.choices(Setting::Range, &m).is_empty(),
                "function {function:#04x} should offer no range"
            );
        }
    }

    /// MAX/MIN/AVG is a three-step ring plus off, with AVG lit; HOLD and
    /// REL are plain toggles and Peak is not a function of these meters.
    pub(crate) fn assert_minmax_ring<M: Vc8x0Model>(volts: u8) {
        /// Status byte 1: bit0=Rel, bit1=Avg, bit2=Min, bit3=Max.
        const S_AVG: u8 = 0x02;

        let mut status = zero_status::<M>();
        status[1] = S_AVG;
        let m = parse_measurement::<M>(&make_payload::<M>(volts, 0x31, b" 12.345", &status))
            .expect("the frame parses");
        assert!(m.flags.avg);

        let proto = Vc8x0Protocol::<M>::new();
        let choices = proto.choices(Setting::MinMax, &m);
        assert_eq!(
            choices.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            choices.iter().map(|c| c.label.as_ref()).collect::<Vec<_>>(),
            vec!["off", "MAX", "MIN", "AVG"]
        );
        assert!(choices[3].current, "AVG is lit");

        assert_eq!(proto.choices(Setting::Hold, &m).len(), 2);
        assert_eq!(proto.choices(Setting::Rel, &m).len(), 2);
        assert!(proto.choices(Setting::Peak, &m).is_empty());
    }

    /// A button neither meter has: refused, and nothing written.
    pub(crate) fn assert_button_refused<M: Vc8x0Model>(button: CycleButton, name: &str) {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc8x0Protocol::<M>::new();
        let err = proto.press(&transport, button).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains(name)),
            "got {err:?}"
        );
        assert!(transport.written.borrow().is_empty());
    }

    /// One RANGE press writes the manual-range command frame.
    ///
    /// `command_frame` is the family's own check of the write shape — the
    /// VC-880 sends the frame bare, the VC-890 behind its ack burst — and
    /// returns the command frame out of it.
    pub(crate) fn assert_range_button_writes_the_range_frame<M: Vc8x0Model>(
        command_frame: &dyn Fn(&MockTransport) -> Vec<u8>,
    ) {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc8x0Protocol::<M>::new();
        proto
            .press(&transport, CycleButton::Range)
            .expect("the frame is written");
        assert_eq!(
            command_frame(&transport),
            build_command(CMD_RANGE_MANUAL),
            "RANGE is command 0x46"
        );
    }

    /// Returning to auto-ranging writes its own command, not a RANGE press.
    ///
    /// `command_frame` as in [`assert_range_button_writes_the_range_frame`].
    pub(crate) fn assert_auto_range_writes_the_auto_frame<M: Vc8x0Model>(
        command_frame: &dyn Fn(&MockTransport) -> Vec<u8>,
    ) {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc8x0Protocol::<M>::new();
        proto.set_auto_range(&transport).expect("written");
        assert_eq!(
            command_frame(&transport),
            build_command(CMD_RANGE_AUTO),
            "AUTO is command 0x47"
        );
    }

    /// One SHIFT/SETUP press writes the six-byte SELECT command frame.
    ///
    /// `command_frame` as in [`assert_range_button_writes_the_range_frame`].
    pub(crate) fn assert_select_writes_the_shift_setup_frame<M: Vc8x0Model>(
        command_frame: &dyn Fn(&MockTransport) -> Vec<u8>,
    ) {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc8x0Protocol::<M>::new();
        proto
            .press(&transport, CycleButton::Select)
            .expect("the frame is written");
        let frame = command_frame(&transport);
        assert_eq!(
            frame,
            build_command(CMD_SELECT),
            "SHIFT/SETUP is command 0x4C"
        );
        assert_eq!(frame.len(), 6);
    }

    /// Each flag button writes its own command byte.
    ///
    /// `command_frame` as in [`assert_range_button_writes_the_range_frame`].
    pub(crate) fn assert_flag_buttons_write_their_bytes<M: Vc8x0Model>(
        command_frame: &dyn Fn(&MockTransport) -> Vec<u8>,
    ) {
        for (button, byte) in [
            (CycleButton::Hold, 0x4Au8),
            (CycleButton::Rel, 0x48),
            (CycleButton::MinMax, 0x49),
        ] {
            let transport = MockTransport::new(vec![]);
            let mut proto = Vc8x0Protocol::<M>::new();
            proto.press(&transport, button).expect("written");
            assert_eq!(command_frame(&transport), build_command(byte), "{button:?}");
        }
    }

    /// Leaving MAX/MIN/AVG writes the exit command, not another press.
    pub(crate) fn assert_leaving_minmax_writes_the_exit_frame<M: Vc8x0Model>(
        command_frame: &dyn Fn(&MockTransport) -> Vec<u8>,
    ) {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc8x0Protocol::<M>::new();
        proto
            .exit_flag(&transport, cycle::FlagSetting::MinMax)
            .expect("written");
        assert_eq!(
            command_frame(&transport),
            build_command(CMD_EXIT_MAX_MIN_AVG),
            "ExitMaxMinAvg is command 0x43"
        );
    }
}
