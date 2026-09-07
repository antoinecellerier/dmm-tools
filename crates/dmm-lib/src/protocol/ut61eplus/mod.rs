pub mod command;
pub mod mode;
pub mod tables;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN};
use crate::protocol::{
    Choice, DeviceProfile, Protocol, Setting, Stability, check_len, cycle, unknown_mode,
    unknown_mode16, unsupported_setting,
};
use crate::transport::Transport;
use command::Command;
use log::debug;
use mode::Mode;
use std::borrow::Cow;
use std::time::Duration;
use tables::DeviceTable;

const UT61EPLUS_COMMANDS: &[&str] = &[
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

/// Protocol implementation for the UT61E+/UT61B+/UT61D+/UT161 family.
pub struct Ut61PlusProtocol {
    table: Box<dyn DeviceTable>,
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// What the last reading said about where the dial sits, for
    /// [`Protocol::choices`] and [`Protocol::select`] for [`Setting::Mode`].
    dial: cycle::DialState,
}

impl Default for Ut61PlusProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut61PlusProtocol {
    pub fn new() -> Self {
        Self::with_table(Box::new(tables::ut61e_plus::Ut61ePlusTable::new()))
    }

    /// Create a protocol instance for a specific model name.
    ///
    /// Recognized model strings (case-insensitive):
    /// - "ut61e+" (Verified), "ut161e" -> UT61E+ table
    /// - "ut61b+", "ut161b" -> UT61B+ table
    /// - "ut61d+", "ut161d" -> UT61D+ table
    ///
    /// Several models share a table — the UT161x meters are believed to speak
    /// the same protocol as their UT61x+ counterparts — so the reported model
    /// name and stability come from the requested model, not from the table.
    /// Otherwise a UT161E would introduce itself as a verified UT61E+.
    ///
    /// Returns `None` if the model string is not recognized.
    pub fn for_model(model: &str) -> Option<Self> {
        // (table, reported model name, verified against real hardware)
        let (table, model_name, verified): (Box<dyn DeviceTable>, _, _) =
            match model.to_lowercase().as_str() {
                "ut61e+" => (
                    Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
                    "UNI-T UT61E+",
                    true,
                ),
                "ut161e" => (
                    Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
                    "UNI-T UT161E",
                    false,
                ),
                "ut61b+" => (
                    Box::new(tables::ut61b_plus::Ut61bPlusTable::new()),
                    "UNI-T UT61B+",
                    false,
                ),
                "ut161b" => (
                    Box::new(tables::ut61b_plus::Ut61bPlusTable::new()),
                    "UNI-T UT161B",
                    false,
                ),
                "ut61d+" => (
                    Box::new(tables::ut61d_plus::Ut61dPlusTable::new()),
                    "UNI-T UT61D+",
                    false,
                ),
                "ut161d" => (
                    Box::new(tables::ut61d_plus::Ut61dPlusTable::new()),
                    "UNI-T UT161D",
                    false,
                ),
                _ => return None,
            };
        Some(Self::with_profile(table, model_name, verified))
    }

    /// Build a protocol whose profile is derived from the wrapped table.
    ///
    /// Only correct when the table's model is the model actually connected;
    /// prefer [`Ut61PlusProtocol::for_model`], which keeps the two separate.
    pub fn with_table(table: Box<dyn DeviceTable>) -> Self {
        let model_name = table.model_name();
        // UT61E+ is the only model verified against real hardware.
        let verified = model_name == "UNI-T UT61E+";
        Self::with_profile(table, model_name, verified)
    }

    fn with_profile(table: Box<dyn DeviceTable>, model_name: &'static str, verified: bool) -> Self {
        // Everything except the UT61E+ is based on RE of the vendor software
        // plus manual specs, so it reports as experimental and points at the
        // family verification issue.
        let (stability, verification_issue) = if verified {
            (Stability::Verified, None)
        } else {
            (Stability::Experimental, Some(7))
        };
        Self {
            table,
            rx_buf: Vec::with_capacity(64),
            dial: cycle::DialState::default(),
            profile: DeviceProfile {
                family_name: "UT61+/UT161",
                model_name,
                stability,
                supported_commands: UT61EPLUS_COMMANDS,
                max_aux_values: 0,
                verification_issue,
            },
        }
    }

    /// Read a raw payload frame from the transport.
    fn read_raw_payload(&mut self, transport: &dyn Transport) -> Result<Vec<u8>> {
        framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_be16,
            |_| true,
            FrameErrorRecovery::Propagate,
            "ut61eplus",
            &framing::HEADER,
        )
    }

    /// Read and parse a measurement response, skipping non-measurement frames.
    fn read_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        for _ in 0..5 {
            let payload = self.read_raw_payload(transport)?;
            if payload.len() >= UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
                return parse_measurement(&payload, self.table.as_ref());
            }
            debug!(
                "skipping non-measurement frame ({} bytes): {:02X?}",
                payload.len(),
                payload
            );
        }
        Err(Error::Timeout)
    }

    /// Write one command frame and drain whatever the meter answers with.
    ///
    /// The meter acks a button press with a short frame; leaving it in the
    /// buffer would make the next measurement read start mid-stream.
    fn press_command(&mut self, transport: &dyn Transport, cmd: Command) -> Result<()> {
        let encoded = cmd.encode();
        transport.write(&encoded)?;

        // Drain any ack/response the meter sends back.
        self.rx_buf.clear();
        let mut tmp = [0u8; 64];
        for _ in 0..3 {
            let n = transport.read_timeout(&mut tmp, 50)?;
            if n == 0 {
                break;
            }
            debug!("drained {} bytes after command", n);
        }

        Ok(())
    }

    fn command_from_name(name: &str) -> Result<Command> {
        match name {
            "hold" => Ok(Command::Hold),
            "minmax" => Ok(Command::MinMax),
            "exit_minmax" => Ok(Command::ExitMinMax),
            "range" => Ok(Command::Range),
            "auto" => Ok(Command::Auto),
            "rel" => Ok(Command::Rel),
            "select2" => Ok(Command::Select2),
            "select" => Ok(Command::Select),
            "light" => Ok(Command::Light),
            "peak" => Ok(Command::PeakMinMax),
            "exit_peak" => Ok(Command::ExitPeak),
            _ => Err(Error::UnsupportedCommand(name.to_string())),
        }
    }
}

impl Protocol for Ut61PlusProtocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // CP2110 init (UART enable, config, purge) is done by Cp2110::init_uart()
        // before the protocol is created. Nothing else needed here.
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let cmd = Command::GetMeasurement.encode();
        debug!("sending measurement request");
        transport.write(&cmd)?;
        let m = self.read_measurement(transport)?;
        // The stream is the only place the meter states its mode, so every
        // reading is what keeps the dial position current.
        self.dial.observe(self.table.dial_positions(), m.mode_raw);
        Ok(m)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let cmd = Self::command_from_name(command)?;
        debug!("sending command: {command}");
        self.press_command(transport, cmd)
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        let cmd = Command::GetName.encode();
        debug!("sending get_name request");
        transport.write(&cmd)?;

        // Read two frames: ack + name
        for _ in 0..2 {
            let payload = self.read_raw_payload(transport)?;
            if payload.first() != Some(&0xFF) {
                let name = String::from_utf8_lossy(&payload).to_string();
                debug!("device name: {name}");
                return Ok(Some(name));
            }
        }

        Ok(None)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn spec_info(&self, mode_raw: u16, range_raw: u8) -> Option<&'static crate::specs::SpecInfo> {
        let mode = Mode::from_byte(mode_raw as u8).ok()?;
        self.table.spec_info(mode, range_raw)
    }

    fn mode_spec_info(&self, mode_raw: u16) -> Option<&'static crate::specs::ModeSpecInfo> {
        let mode = Mode::from_byte(mode_raw as u8).ok()?;
        self.table.mode_spec_info(mode)
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        match setting {
            Setting::Mode => cycle::mode_choices(self, current),
            Setting::Range => cycle::range_choices(self, current),
            flag => match cycle::FlagSetting::of(flag) {
                Some(flag) => cycle::flag_choices(self, flag, current),
                None => Vec::new(),
            },
        }
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        match setting {
            Setting::Mode => cycle::select_mode(self, transport, id),
            Setting::Range => cycle::select_range(self, transport, id),
            flag => match cycle::FlagSetting::of(flag) {
                Some(flag) => cycle::select_flag(self, transport, flag, id),
                None => Err(unsupported_setting(setting)),
            },
        }
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::CaptureStep;
        vec![
            // Measurement modes
            CaptureStep {
                id: "dcv",
                instruction: "Set meter to DC V (V\u{23CF}). Leave leads open.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "dcv_short",
                instruction: "DC V mode: touch the two probe tips together.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "acv",
                instruction: "Set meter to AC V (V~). Leave leads open.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "dcmv",
                instruction: "Set meter to DC mV. Leave leads open.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "ohm",
                instruction: "Set meter to \u{03A9}. Leave leads open (should show OL).",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "ohm_short",
                instruction: "\u{03A9} mode: touch the two probe tips together.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "continuity",
                instruction: "Set meter to continuity (buzzer). Touch probes together.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "diode",
                instruction: "Set meter to diode. Leave leads open (should show OL).",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "capacitance",
                instruction: "Set meter to capacitance. Leave leads open.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "hz",
                instruction: "Set meter to Hz (press SELECT2 on AC mA or V~ mode).",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "duty",
                instruction: "Hz mode: press SELECT2 again for Duty %.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "ncv",
                instruction: "Set meter to NCV. Hold near a live wire.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "hfe",
                instruction: "Set meter to hFE (transistor test).",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "dcua",
                instruction: "Set meter to DC uA.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "dcma",
                instruction: "Set meter to DC mA.",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "dca",
                instruction: "Set meter to DC A (A\u{23CF}).",
                command: None,
                samples: 3,
            },
            CaptureStep {
                id: "temp",
                instruction: "Set meter to temperature (K-type thermocouple, if available).",
                command: None,
                samples: 3,
            },
            // Flags & commands
            CaptureStep::with_command(
                "hold",
                "DC V mode: press HOLD on the meter, or we will send the command.",
                "hold",
                3,
            ),
            CaptureStep::with_command("hold_off", "Press HOLD again to turn it off.", "hold", 3),
            CaptureStep::with_command("rel", "DC V mode: we will send REL.", "rel", 3),
            CaptureStep::with_command(
                "rel_off",
                "We will send REL again to turn it off.",
                "rel",
                3,
            ),
            CaptureStep::with_command("minmax", "We will send MIN/MAX.", "minmax", 3),
            CaptureStep::with_command("minmax_off", "We will exit MIN/MAX.", "exit_minmax", 3),
            // A single RANGE press, not a sweep. A six-step sweep was tried
            // and removed: on hardware it produced range indices 0, 2, 0, 0,
            // 0, 0 — never visiting 22V or 1000V — and flipped the mode byte
            // between DC V (0x02) and AC+DC V (0x19) partway through, which
            // is the documented effect of SELECT (0x4C), not RANGE (0x46).
            // Until what 0x46 actually does is known, stepping it repeatedly
            // just files misleading data. See the UT61E+ section of
            // docs/verification-backlog.md.
            CaptureStep::with_command(
                "range",
                "We will send RANGE to switch to manual.",
                "range",
                3,
            ),
            CaptureStep::with_command(
                "auto",
                "We will send AUTO to return to auto-range.",
                "auto",
                3,
            ),
        ]
    }
}

/// How long to leave the meter alone after a button press before asking it
/// what mode it is in.
///
/// The meter is polled, so a press can land while a frame is already on its
/// way and the reading after it still shows the old mode. On a UT61E+ the
/// first read after this delay (which follows `press_command`'s own 150 ms
/// drain) showed the new mode in every leg but one, where a second read did.
const SELECT_SETTLE_DELAY: Duration = Duration::from_millis(150);
/// Readings taken after a press before concluding it changed nothing.
///
/// RANGE presses reuse both constants: nobody has timed 0x46 separately, and
/// the meter answers a press the same way whichever button sent it.
const SELECT_SETTLE_READS: usize = 3;

impl cycle::CycleMeter for Ut61PlusProtocol {
    fn dial_positions(&self) -> &'static [cycle::DialPosition] {
        self.table.dial_positions()
    }

    fn dial_state(&self) -> &cycle::DialState {
        &self.dial
    }

    fn dial_state_mut(&mut self) -> &mut cycle::DialState {
        &mut self.dial
    }

    fn press(&mut self, transport: &dyn Transport, button: cycle::CycleButton) -> Result<()> {
        let cmd = match button {
            cycle::CycleButton::Select => Command::Select,
            cycle::CycleButton::Hz => Command::Select2,
            cycle::CycleButton::Range => Command::Range,
            cycle::CycleButton::Hold => Command::Hold,
            cycle::CycleButton::Rel => Command::Rel,
            cycle::CycleButton::MinMax => Command::MinMax,
            cycle::CycleButton::Peak => Command::PeakMinMax,
        };
        self.press_command(transport, cmd)
    }

    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        Protocol::request_measurement(self, transport)
    }

    fn mode_label(&self, mode: u16) -> Cow<'static, str> {
        match u8::try_from(mode) {
            Ok(byte) => match Mode::from_byte(byte) {
                Ok(m) => Cow::Borrowed(m.as_static_str()),
                Err(_) => unknown_mode(byte),
            },
            // This family's mode field is one byte wide; anything wider than
            // that never came from a meter.
            Err(_) => unknown_mode16(mode),
        }
    }

    fn settle(&self) -> cycle::Settle {
        cycle::Settle {
            delay: SELECT_SETTLE_DELAY,
            reads: SELECT_SETTLE_READS,
        }
    }

    /// The model's own range table for the mode, in range-byte order — the
    /// same table `range_label` reads, so a rung is named exactly as the
    /// reading that lands on it will be.
    fn range_ladder(&self, mode: u16) -> Vec<Cow<'static, str>> {
        match u8::try_from(mode).map(Mode::from_byte) {
            Ok(Ok(mode)) => tables::range_ladder(self.table.as_ref(), mode),
            // A mode byte this family's parser cannot name has no table.
            _ => Vec::new(),
        }
    }

    fn set_auto_range(&mut self, transport: &dyn Transport) -> Result<()> {
        self.press_command(transport, Command::Auto)
    }

    /// HOLD, REL and MIN/MAX are taken in every mode — the family's command
    /// matrix (research spec §6) lists no mode restriction on 0x4A, 0x48 or
    /// 0x41, and remote mode switching was verified on a UT61E+ under an
    /// active MIN/MAX. Peak is the one that depends on both model and mode.
    fn flag_states(&self, setting: cycle::FlagSetting, mode: u16) -> &'static [u16] {
        match setting {
            cycle::FlagSetting::Hold | cycle::FlagSetting::Rel => &[0, 1],
            // MAX then MIN, the order the meter's own 2-state ring cycles in.
            // No AVG: the UT61E+ reports none over USB.
            cycle::FlagSetting::MinMax => &[0, 1, 2],
            cycle::FlagSetting::Peak => match u8::try_from(mode).map(Mode::from_byte) {
                Ok(Ok(mode)) if self.table.peak_modes().contains(&mode) => &[0, 1, 2],
                _ => &[],
            },
        }
    }

    fn exit_flag(&mut self, transport: &dyn Transport, setting: cycle::FlagSetting) -> Result<()> {
        match setting {
            cycle::FlagSetting::MinMax => self.press_command(transport, Command::ExitMinMax),
            cycle::FlagSetting::Peak => self.press_command(transport, Command::ExitPeak),
            // HOLD and REL press their own button back off, so the driver
            // never asks this of them.
            other => Err(unsupported_setting(other.setting())),
        }
    }
}

/// Parse a UT61E+/UT61B+/UT61D+/UT161 measurement payload (pure function).
///
/// Layout (verified against real device captures):
/// - byte 0:    mode   (raw, no masking — does not have 0x30 prefix)
/// - byte 1:    range  (& 0x0F — has 0x30 prefix)
/// - bytes 2-8: display value (7 ASCII chars, no masking needed)
/// - byte 9:    bar graph tens digit (raw, no 0x30 prefix; value = b9*10+b10)
/// - byte 10:   bar graph ones digit (raw, no 0x30 prefix)
/// - byte 11:   flag1  (& 0x0F — has 0x30 prefix)
/// - byte 12:   flag2  (& 0x0F — has 0x30 prefix)
/// - byte 13:   flag3  (& 0x0F — has 0x30 prefix)
pub fn parse_measurement(payload: &[u8], table: &dyn DeviceTable) -> Result<Measurement> {
    check_len("ut61eplus", payload, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN)?;

    // Mode byte is raw (no 0x30 prefix), range byte has 0x30 prefix
    let mode_byte = payload[0];
    let range_byte = payload[1] & 0x0F;
    let display_bytes = &payload[2..9];
    // Bar graph bytes are raw (no 0x30 prefix observed on real device).
    // Encoding is decimal (byte9 * 10 + byte10), NOT nibble shift.
    // Verified on real device: 5V→9, 10V→20, 20V→39 on 22V range;
    // 1V→20 on 2.2V range. Maps to ~46 LCD bar segments.
    let bar_hi = payload[9] as u16;
    let bar_lo = payload[10] as u16;
    let flag1 = payload[11] & 0x0F;
    let flag2 = payload[12] & 0x0F;
    let flag3 = payload[13] & 0x0F;

    let mode = Mode::from_byte(mode_byte)?;
    let display_raw = String::from_utf8_lossy(display_bytes).to_string();
    let progress = bar_hi * 10 + bar_lo;
    let flags = StatusFlags::parse(flag1, flag2, flag3);

    // Look up range info from device table
    let range_info = table.range_info(mode, range_byte);
    let unit = range_info.map(|r| r.unit).unwrap_or("");
    let range_label = range_info.map(|r| r.label).unwrap_or("");

    // Parse display value.
    let display_trimmed = display_raw.trim();
    let display_compact: String = display_trimmed.chars().filter(|c| *c != ' ').collect();
    let value = if mode == Mode::Ncv {
        let level = display_compact.parse::<u8>().unwrap_or(0);
        MeasuredValue::NcvLevel(level)
    } else if display_compact == "OL" || display_compact.contains("OL") {
        MeasuredValue::Overload
    } else {
        match display_compact.parse::<f64>() {
            Ok(v) => MeasuredValue::Normal(v),
            Err(_) => {
                debug!(
                    "could not parse display value: {:?} (compact: {:?})",
                    display_trimmed, display_compact
                );
                MeasuredValue::Overload
            }
        }
    };

    Ok(Measurement {
        mode: Cow::Borrowed(mode.as_static_str()),
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: Some(progress),
        display_raw: Some(display_raw),
        flags,
        ..Measurement::from_payload(&payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN])
    })
}

/// Build a 14-byte UT61E+ protocol payload from parts (for tests).
///
/// This is a convenience helper for tests that need a realistic `Measurement`
/// produced by the protocol parser rather than a hand-constructed struct.
///
/// Parameters mirror the raw protocol layout:
/// - `mode`: mode byte (e.g. 0x02 = DC V)
/// - `range`: range nibble (0x30 prefix added automatically)
/// - `display`: 7-byte ASCII display value (e.g. `b"  5.678"`)
/// - `progress`: (tens, ones) bar graph digits — decoded as tens*10+ones
/// - `flags`: (flag1, flag2, flag3) nibbles (0x30 prefix added automatically)
#[cfg(any(test, feature = "test-support"))]
fn make_payload(
    mode: u8,
    range: u8,
    display: &[u8; 7],
    progress: (u8, u8),
    flags: (u8, u8, u8),
) -> Vec<u8> {
    vec![
        mode,
        range | 0x30,
        display[0],
        display[1],
        display[2],
        display[3],
        display[4],
        display[5],
        display[6],
        progress.0,
        progress.1,
        flags.0 | 0x30,
        flags.1 | 0x30,
        flags.2 | 0x30,
    ]
}

#[cfg(any(test, feature = "test-support"))]
pub fn make_test_measurement(
    mode: u8,
    range: u8,
    display: &[u8; 7],
    progress: (u8, u8),
    flags: (u8, u8, u8),
) -> Measurement {
    let table = tables::ut61e_plus::Ut61ePlusTable::new();
    let payload = make_payload(mode, range, display, progress, flags);
    parse_measurement(&payload, &table).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tables::ut61e_plus::Ut61ePlusTable;

    // --- Remote mode selection (protocol::cycle) --------------------------
    //
    // The cycles these exercise are verified on a real UT61E+ (research spec
    // §2.3/§2.5); what is not verified is the settle timing, which the fake
    // transport below sidesteps by answering instantly.

    use crate::protocol::cycle::{CycleButton, CycleMeter};
    use crate::protocol::framing::test_frame_be16;
    use crate::transport::mock::MockTransport;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    /// A frame carrying a DC-V-shaped reading in `mode`.
    fn frame_in(mode: u8) -> Vec<u8> {
        test_frame_be16(&make_payload(
            mode,
            0x01,
            b" 12.345",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ))
    }

    #[test]
    fn press_writes_the_select_and_hz_frames() {
        let mock = MockTransport::new(vec![]);
        let mut proto = Ut61PlusProtocol::new();
        proto.press(&mock, CycleButton::Select).unwrap();
        proto.press(&mock, CycleButton::Hz).unwrap();

        let written = mock.written.borrow();
        // 0x4C + 379 = 0x01C7, 0x49 + 379 = 0x01C4.
        assert_eq!(written.len(), 2, "{written:02X?}");
        assert_eq!(written[0], [0xAB, 0xCD, 0x03, 0x4C, 0x01, 0xC7]);
        assert_eq!(written[1], [0xAB, 0xCD, 0x03, 0x49, 0x01, 0xC4]);
    }

    #[test]
    fn mode_choices_on_the_dc_volts_dial_offer_ac_dc() {
        let mock = MockTransport::new(vec![frame_in(0x02)]);
        let mut proto = Ut61PlusProtocol::new();
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.mode, "DC V");

        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(
            choices.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![0x02, 0x19]
        );
        assert_eq!(choices[0].label, "DC V");
        assert!(choices[0].current, "DC V is the live mode");
        assert_eq!(choices[1].label, "AC+DC V");
        assert!(!choices[1].current);
    }

    /// A UT61E+ on the V⎓ dial: it answers 0x5E with a reading, and SELECT
    /// (0x4C) flips DC V ↔ AC+DC V and acks with `FF 00`.
    ///
    /// `MockTransport` cannot stand in here — it ignores writes, so the drain
    /// after a press would swallow a queued measurement frame.
    struct VoltsDial {
        mode: Cell<u8>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        presses: Cell<usize>,
    }

    impl VoltsDial {
        fn new(mode: u8) -> Self {
            Self {
                mode: Cell::new(mode),
                queued: RefCell::new(VecDeque::new()),
                presses: Cell::new(0),
            }
        }
    }

    impl Transport for VoltsDial {
        fn write(&self, data: &[u8]) -> Result<()> {
            match data.get(3) {
                Some(&0x5E) => self
                    .queued
                    .borrow_mut()
                    .push_back(frame_in(self.mode.get())),
                Some(&0x4C) => {
                    self.presses.set(self.presses.get() + 1);
                    self.mode
                        .set(if self.mode.get() == 0x02 { 0x19 } else { 0x02 });
                    // The meter acks a press before the next reading.
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                _ => {}
            }
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn select_mode_presses_once_and_the_stream_keeps_parsing() {
        let meter = VoltsDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        assert_eq!(proto.request_measurement(&meter).unwrap().mode, "DC V");

        proto.select(&meter, Setting::Mode, 0x19).expect("switched");
        assert_eq!(meter.presses.get(), 1, "one press per ring step");
        assert_eq!(proto.dial.last_mode(), Some(0x19));

        // The ack must not have been left in the buffer for the next read.
        let m = proto.request_measurement(&meter).unwrap();
        assert_eq!(m.mode, "AC+DC V");
        assert_eq!(m.range_label, "22V");
    }

    // --- Range selection (protocol::cycle) --------------------------------

    /// Flag nibble 2 with the MANUAL range bit set (`flags.auto_range` is
    /// its inverse).
    const MANUAL: u8 = 0x04;

    fn range_ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().map(|c| c.id).collect()
    }

    #[test]
    fn dc_volts_offers_auto_and_the_four_manual_ranges() {
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, MANUAL, 0));
        let proto = Ut61PlusProtocol::new();
        let choices = proto.choices(Setting::Range, &m);
        assert_eq!(range_ids(&choices), vec![0, 1, 2, 3, 4]);
        let labels: Vec<_> = choices.iter().map(|c| c.label.as_ref()).collect();
        assert_eq!(labels, vec!["Auto", "2.2V", "22V", "220V", "1000V"]);
        // Range byte 1 is the second rung.
        assert_eq!(
            choices
                .iter()
                .filter(|c| c.current)
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn an_auto_ranging_reading_marks_auto_current() {
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        let proto = Ut61PlusProtocol::new();
        let choices = proto.choices(Setting::Range, &m);
        assert!(choices[0].current, "auto is the live choice");
        assert!(choices[1..].iter().all(|c| !c.current));
    }

    /// The mV dial has one range on the E+ and RANGE does nothing in DC mV
    /// or AC mV; DC A's table is a placeholder plus the one verified 20A
    /// entry; diode has a single entry. None of the four is a ladder.
    #[test]
    fn modes_without_a_ladder_offer_no_ranges() {
        let proto = Ut61PlusProtocol::new();
        for (mode, range) in [(0x03, 0x00), (0x01, 0x00), (0x10, 0x01), (0x08, 0x00)] {
            let m = make_test_measurement(mode, range, b" 12.345", (0, 0), (0, MANUAL, 0));
            assert!(
                proto.choices(Setting::Range, &m).is_empty(),
                "{} should offer no range",
                m.mode
            );
        }
    }

    /// A UT61E+ on the V⎓ dial whose RANGE button (0x46) steps the four DC V
    /// ranges and whose AUTO button (0x47) returns to auto-ranging.
    struct RangeDial {
        range: Cell<u8>,
        manual: Cell<bool>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        presses: Cell<usize>,
        autos: Cell<usize>,
    }

    impl RangeDial {
        fn auto() -> Self {
            Self {
                range: Cell::new(1),
                manual: Cell::new(false),
                queued: RefCell::new(VecDeque::new()),
                presses: Cell::new(0),
                autos: Cell::new(0),
            }
        }

        fn manual_at(range: u8) -> Self {
            let dial = Self::auto();
            dial.range.set(range);
            dial.manual.set(true);
            dial
        }
    }

    impl Transport for RangeDial {
        fn write(&self, data: &[u8]) -> Result<()> {
            match data.get(3) {
                Some(&0x5E) => {
                    let flag2 = if self.manual.get() { MANUAL } else { 0 };
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&make_payload(
                            0x02,
                            self.range.get(),
                            b" 12.345",
                            (0x00, 0x00),
                            (0x00, flag2, 0x00),
                        )));
                }
                Some(&0x46) => {
                    self.presses.set(self.presses.get() + 1);
                    // The first press engages manual ranging where auto had
                    // left the meter; later ones step the ladder.
                    if self.manual.replace(true) {
                        self.range.set((self.range.get() + 1) % 4);
                    }
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                Some(&0x47) => {
                    self.autos.set(self.autos.get() + 1);
                    self.manual.set(false);
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&[0xFF, 0x00]));
                }
                _ => {}
            }
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn selecting_a_range_presses_the_range_button_to_it() {
        let meter = RangeDial::manual_at(1);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Range, 4).expect("switched");
        assert_eq!(meter.presses.get(), 2, "22V -> 220V -> 1000V");
        assert_eq!(meter.range.get(), 3);
        assert_eq!(meter.autos.get(), 0);
    }

    #[test]
    fn selecting_auto_sends_the_auto_command_and_confirms_it() {
        let meter = RangeDial::manual_at(2);
        let mut proto = Ut61PlusProtocol::new();
        proto
            .select(&meter, Setting::Range, 0)
            .expect("back to auto");
        assert_eq!(meter.autos.get(), 1);
        assert_eq!(meter.presses.get(), 0);
        assert!(!meter.manual.get());
    }

    #[test]
    fn a_meter_already_auto_ranging_is_not_told_to_be() {
        let meter = RangeDial::auto();
        let mut proto = Ut61PlusProtocol::new();
        proto
            .select(&meter, Setting::Range, 0)
            .expect("already auto");
        assert_eq!(meter.autos.get(), 0);
        assert_eq!(meter.presses.get(), 0);
    }

    // --- Flag-backed settings (HOLD, REL, MIN/MAX, Peak) ------------------

    /// Flag nibble 1 bits: REL is bit 0, HOLD bit 1, MIN bit 2, MAX bit 3.
    const F_REL: u8 = 0x01;
    const F_HOLD: u8 = 0x02;
    const F_MIN: u8 = 0x04;
    const F_MAX: u8 = 0x08;
    /// Flag nibble 3 bits: P-MIN is bit 1, P-MAX bit 2.
    const F_PEAK_MIN: u8 = 0x02;
    const F_PEAK_MAX: u8 = 0x04;

    fn flag_labels(choices: &[Choice]) -> Vec<String> {
        choices.iter().map(|c| c.label.to_string()).collect()
    }

    #[test]
    fn hold_and_rel_choices_follow_the_flags() {
        let proto = Ut61PlusProtocol::new();
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_HOLD, 0, 0));

        let hold = proto.choices(Setting::Hold, &m);
        assert_eq!(range_ids(&hold), vec![0, 1]);
        assert_eq!(flag_labels(&hold), vec!["off", "on"]);
        assert_eq!(hold.iter().filter(|c| c.current).count(), 1);
        assert!(hold[1].current, "HOLD is lit");

        let rel = proto.choices(Setting::Rel, &m);
        assert_eq!(range_ids(&rel), vec![0, 1]);
        assert!(rel[0].current, "REL is dark");
    }

    #[test]
    fn minmax_choices_are_the_two_state_ring_plus_off() {
        let proto = Ut61PlusProtocol::new();
        let m = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_MIN, MANUAL, 0));
        let choices = proto.choices(Setting::MinMax, &m);
        assert_eq!(range_ids(&choices), vec![0, 1, 2]);
        assert_eq!(flag_labels(&choices), vec!["off", "MAX", "MIN"]);
        assert!(choices[2].current, "MIN is lit");

        let max = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (F_MAX, MANUAL, 0));
        assert!(proto.choices(Setting::MinMax, &max)[1].current);
    }

    /// Peak activates on AC mV and does nothing on DC V, verified 2026-03-21
    /// (docs/verification-backlog.md).
    #[test]
    fn peak_is_offered_in_ac_but_not_in_dc_volts() {
        let proto = Ut61PlusProtocol::new();
        let ac = make_test_measurement(0x01, 0x00, b"  8.700", (0, 0), (0, 0, F_PEAK_MAX));
        let choices = proto.choices(Setting::Peak, &ac);
        assert_eq!(range_ids(&choices), vec![0, 1, 2]);
        assert_eq!(flag_labels(&choices), vec!["off", "P-MAX", "P-MIN"]);
        assert!(choices[1].current, "P-MAX is lit");

        let dc = make_test_measurement(0x02, 0x01, b" 12.345", (0, 0), (0, 0, 0));
        assert!(proto.choices(Setting::Peak, &dc).is_empty());
    }

    /// The B+ has no Peak flags and its command matrix marks 0x4D/0x4E "No
    /// effect" (research spec §4 and §6).
    #[test]
    fn the_b_plus_offers_no_peak_anywhere() {
        let proto = Ut61PlusProtocol::for_model("ut61b+").expect("known model");
        let ac = make_test_measurement(0x01, 0x00, b"  8.700", (0, 0), (0, 0, 0));
        assert!(proto.choices(Setting::Peak, &ac).is_empty());
        // The buttons it does have are still offered.
        assert_eq!(proto.choices(Setting::MinMax, &ac).len(), 3);
    }

    /// A UT61E+ whose HOLD (0x4A), MIN/MAX (0x41) and Peak (0x4D) buttons do
    /// what the 2026-03-21 capture saw, and whose 0x42/0x4E leave those
    /// states.
    struct FlagDial {
        mode: u8,
        flag1: Cell<u8>,
        flag3: Cell<u8>,
        queued: RefCell<VecDeque<Vec<u8>>>,
        writes: RefCell<Vec<u8>>,
    }

    impl FlagDial {
        fn new(mode: u8) -> Self {
            Self {
                mode,
                flag1: Cell::new(0),
                flag3: Cell::new(0),
                queued: RefCell::new(VecDeque::new()),
                writes: RefCell::new(Vec::new()),
            }
        }

        /// Step a two-state ring held in `cell`: off enters the first state,
        /// and the two swap from there. Never returns to off.
        fn ring(cell: &Cell<u8>, first: u8, second: u8) {
            let now = cell.get();
            let next = if now & first != 0 { second } else { first };
            cell.set((now & !(first | second)) | next);
        }

        fn ack(&self) {
            self.queued
                .borrow_mut()
                .push_back(test_frame_be16(&[0xFF, 0x00]));
        }
    }

    impl Transport for FlagDial {
        fn write(&self, data: &[u8]) -> Result<()> {
            let Some(&cmd) = data.get(3) else {
                return Ok(());
            };
            self.writes.borrow_mut().push(cmd);
            match cmd {
                0x5E => {
                    self.queued
                        .borrow_mut()
                        .push_back(test_frame_be16(&make_payload(
                            self.mode,
                            0x00,
                            b" 12.345",
                            (0x00, 0x00),
                            (self.flag1.get(), 0x00, self.flag3.get()),
                        )));
                    return Ok(());
                }
                0x4A => self.flag1.set(self.flag1.get() ^ F_HOLD),
                0x48 => self.flag1.set(self.flag1.get() ^ F_REL),
                0x41 => Self::ring(&self.flag1, F_MAX, F_MIN),
                0x42 => self.flag1.set(self.flag1.get() & !(F_MAX | F_MIN)),
                0x4D => Self::ring(&self.flag3, F_PEAK_MAX, F_PEAK_MIN),
                0x4E => self
                    .flag3
                    .set(self.flag3.get() & !(F_PEAK_MAX | F_PEAK_MIN)),
                _ => {}
            }
            self.ack();
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let Some(frame) = self.queued.borrow_mut().pop_front() else {
                return Ok(0);
            };
            let len = frame.len().min(buf.len());
            buf[..len].copy_from_slice(&frame[..len]);
            Ok(len)
        }

        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            Ok(())
        }
    }

    /// The command bytes the meter was sent, measurement requests aside.
    fn commands(meter: &FlagDial) -> Vec<u8> {
        meter
            .writes
            .borrow()
            .iter()
            .copied()
            .filter(|&c| c != 0x5E)
            .collect()
    }

    #[test]
    fn holding_presses_the_hold_command_once() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Hold, 1).expect("held");
        assert_eq!(commands(&meter), vec![0x4A]);
        assert_eq!(meter.flag1.get(), F_HOLD);
    }

    #[test]
    fn reaching_min_presses_the_minmax_button_twice() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::MinMax, 2).expect("in MIN");
        assert_eq!(commands(&meter), vec![0x41, 0x41], "off -> MAX -> MIN");
        assert_eq!(meter.flag1.get(), F_MIN);
    }

    #[test]
    fn leaving_minmax_sends_the_exit_command() {
        let meter = FlagDial::new(0x02);
        meter.flag1.set(F_MAX);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::MinMax, 0).expect("left");
        assert_eq!(commands(&meter), vec![0x42]);
        assert_eq!(meter.flag1.get(), 0);
    }

    #[test]
    fn reaching_peak_min_presses_the_peak_button_and_leaves_by_its_own() {
        let meter = FlagDial::new(0x01);
        let mut proto = Ut61PlusProtocol::new();
        proto.select(&meter, Setting::Peak, 2).expect("in P-MIN");
        assert_eq!(commands(&meter), vec![0x4D, 0x4D]);
        assert_eq!(meter.flag3.get(), F_PEAK_MIN);

        proto.select(&meter, Setting::Peak, 0).expect("left peak");
        assert_eq!(commands(&meter), vec![0x4D, 0x4D, 0x4E]);
        assert_eq!(meter.flag3.get(), 0);
    }

    #[test]
    fn peak_in_dc_volts_is_refused_without_writing() {
        let meter = FlagDial::new(0x02);
        let mut proto = Ut61PlusProtocol::new();
        proto.request_measurement(&meter).unwrap();
        let err = proto.select(&meter, Setting::Peak, 1).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m)
                if m == "peak cannot be set in DC V on this meter"),
            "{err}"
        );
        assert!(commands(&meter).is_empty());
    }

    #[test]
    fn parse_dc_voltage() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x01, b" 12.345", (0x02, 0x06), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.mode_raw, 0x02);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "22V");
        assert!(m.flags.auto_range);
    }

    #[test]
    fn parse_overload() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x06, 0x00, b"    OL ", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert_eq!(m.mode, "Ω");
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.unit, "Ω");
    }

    #[test]
    fn parse_with_hold_flag() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x00, b"  1.234", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert!(m.flags.hold);
        assert!(m.flags.auto_range);
        assert!(!m.flags.rel);
    }

    #[test]
    fn parse_negative_with_space() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x03, 0x00, b"- 55.79", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-55.79)).abs() < 1e-6));
    }

    #[test]
    fn parse_payload_too_short() {
        let table = Ut61ePlusTable::new();
        let payload = vec![0x30; 10];
        assert!(parse_measurement(&payload, &table).is_err());
    }

    #[test]
    fn display_format() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x02, 0x01, b"  5.678", (0x00, 0x00), (0x02, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        let s = m.to_string();
        assert!(s.contains("5.678"));
        assert!(s.contains("V"));
        assert!(s.contains("HOLD"));
        assert!(s.contains("AUTO"));
    }

    #[test]
    fn parse_ncv() {
        let table = Ut61ePlusTable::new();
        let payload = make_payload(0x14, 0x00, b"      3", (0x00, 0x00), (0x00, 0x00, 0x00));
        let m = parse_measurement(&payload, &table).unwrap();
        assert_eq!(m.mode, "NCV");
        assert!(matches!(m.value, MeasuredValue::NcvLevel(3)));
    }
}
