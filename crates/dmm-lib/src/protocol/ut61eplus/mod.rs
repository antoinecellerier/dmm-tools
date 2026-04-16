pub mod command;
pub mod mode;
pub mod tables;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery, UT61EPLUS_MEASUREMENT_PAYLOAD_LEN};
use crate::protocol::{DeviceProfile, Protocol, Stability};
use crate::transport::Transport;
use command::Command;
use log::debug;
use mode::Mode;
use std::borrow::Cow;
use std::time::Instant;
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
    /// - "ut61e+", "ut161e" -> UT61E+ table (Verified)
    /// - "ut61b+", "ut161b" -> UT61B+ table (Experimental)
    /// - "ut61d+", "ut161d" -> UT61D+ table (Experimental)
    ///
    /// Returns `None` if the model string is not recognized.
    pub fn for_model(model: &str) -> Option<Self> {
        let table: Box<dyn DeviceTable> = match model.to_lowercase().as_str() {
            "ut61e+" | "ut161e" => Box::new(tables::ut61e_plus::Ut61ePlusTable::new()),
            "ut61b+" | "ut161b" => Box::new(tables::ut61b_plus::Ut61bPlusTable::new()),
            "ut61d+" | "ut161d" => Box::new(tables::ut61d_plus::Ut61dPlusTable::new()),
            _ => return None,
        };
        Some(Self::with_table(table))
    }

    pub fn with_table(table: Box<dyn DeviceTable>) -> Self {
        let model_name = table.model_name();
        // UT61E+ is the only model verified against real hardware.
        // B+ and D+ tables are based on RE of vendor software + manual specs.
        let (stability, verification_issue) = if model_name == "UNI-T UT61E+" {
            (Stability::Verified, None)
        } else {
            (Stability::Experimental, Some(7))
        };
        Self {
            table,
            rx_buf: Vec::with_capacity(64),
            profile: DeviceProfile {
                family_name: "UT61+/UT161",
                model_name,
                stability,
                supported_commands: UT61EPLUS_COMMANDS,
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
        self.read_measurement(transport)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let cmd = Self::command_from_name(command)?;
        let encoded = cmd.encode();
        debug!("sending command: {command}");
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
            CaptureStep {
                id: "hold",
                instruction: "DC V mode: press HOLD on the meter, or we will send the command.",
                command: Some("hold"),
                samples: 3,
            },
            CaptureStep {
                id: "hold_off",
                instruction: "Press HOLD again to turn it off.",
                command: Some("hold"),
                samples: 3,
            },
            CaptureStep {
                id: "rel",
                instruction: "DC V mode: we will send REL.",
                command: Some("rel"),
                samples: 3,
            },
            CaptureStep {
                id: "rel_off",
                instruction: "We will send REL again to turn it off.",
                command: Some("rel"),
                samples: 3,
            },
            CaptureStep {
                id: "minmax",
                instruction: "We will send MIN/MAX.",
                command: Some("minmax"),
                samples: 3,
            },
            CaptureStep {
                id: "minmax_off",
                instruction: "We will exit MIN/MAX.",
                command: Some("exit_minmax"),
                samples: 3,
            },
            CaptureStep {
                id: "range",
                instruction: "We will send RANGE to switch to manual.",
                command: Some("range"),
                samples: 3,
            },
            CaptureStep {
                id: "auto",
                instruction: "We will send AUTO to return to auto-range.",
                command: Some("auto"),
                samples: 3,
            },
        ]
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
    if payload.len() < UT61EPLUS_MEASUREMENT_PAYLOAD_LEN {
        return Err(Error::invalid_response(
            format!(
                "payload too short: {} bytes, expected {}",
                payload.len(),
                UT61EPLUS_MEASUREMENT_PAYLOAD_LEN
            ),
            payload,
        ));
    }

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
        timestamp: Instant::now(),
        mode: Cow::Borrowed(mode.as_static_str()),
        mode_raw: mode_byte as u16,
        range_raw: range_byte,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        progress: Some(progress),
        display_raw: Some(display_raw),
        flags,
        aux_values: vec![],
        raw_payload: payload[..UT61EPLUS_MEASUREMENT_PAYLOAD_LEN].to_vec(),
        spec: None,
        mode_spec: None,
    })
}

/// Build a 14-byte payload and parse it into a `Measurement` using the UT61E+ table.
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
/// Build a 14-byte UT61E+ protocol payload from parts (for tests).
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
