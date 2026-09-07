//! Voltcraft VC-890 multimeter protocol.
//!
//! Polled protocol: host sends 0x5E measurement request, meter responds
//! with a 66-byte live data frame. Same AB CD + BE16 framing as UT61E+.
//!
//! Key differences from VC-880:
//! - Polled (request/response) instead of streaming
//! - 60,000 counts (vs 40,000) — range values 6/60/600 instead of 4/40/400
//! - 66-byte frames (vs 39) — more display fields
//! - Different function code assignments (remapped)
//! - OLED display, ES51997P + EFM32 MCU chipset
//!
//! Based on ILSpy decompilation of Voltsoft DMSShare.dll (VC890Obj,
//! VC890Reading classes).
//! See docs/research/vc890/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::measurement::Measurement;
use crate::protocol::cycle::RANGE_BUTTON_NAME;
use crate::protocol::cycle::{
    self, CycleButton, CycleMeter, DialPosition, DialState, Ring, Settle,
};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::vc8x0_common::{
    CMD_RANGE_AUTO, CMD_RANGE_MANUAL, CMD_SELECT, RangeEntry, SELECT_BUTTON_NAME, common_flags,
    main_display, parse_value, re, resolve_function, resolve_range,
};
use crate::protocol::{
    Choice, DeviceProfile, Protocol, Setting, Stability, check_len, unsupported_setting,
};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;
use std::thread;
use std::time::Duration;

/// Ack frame sent as both a pre-clear (before each command) and a
/// post-confirm (after each received frame). Decoded from
/// `DMSShare_decompiled.cs:3861` (`AckMessage`) plus the `WriteCommand`
/// builder at line 3805: command = `0xFF`, data = `[0x00]`, header
/// `0xAB 0xCD`, length byte `0x04` (= 2 + cmd + 1 data), checksum
/// `0xAB + 0xCD + 0x04 + 0xFF + 0x00 = 0x027B` (BE).
const ACK_FRAME: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];

/// Gap between the three ack writes, matching `Thread.Sleep(100)` in
/// the vendor code.
const ACK_GAP: Duration = Duration::from_millis(100);

/// Send the vendor's 3× ack sequence (`AckMessage(clear: true)`):
/// three ack frames separated by 100ms sleeps. The vendor also calls
/// `FlushBuffer()` at the end, but the `Transport` trait has no flush
/// and we have not observed a functional need for one.
fn send_ack_sequence(transport: &dyn Transport) -> Result<()> {
    for i in 0..3 {
        transport.write(&ACK_FRAME)?;
        if i < 2 {
            thread::sleep(ACK_GAP);
        }
    }
    Ok(())
}

/// Live data message type byte.
const MSG_TYPE_LIVE_DATA: u8 = 0x01;

/// Measurement request command (polled model).
const CMD_GET_MEASUREMENT: u8 = 0x5E;

/// Minimum payload length for a VC890 live data frame.
/// Payload = type(1) + function(1) + range(1) + value1(7) + value2(8) +
///   value3(10) + value4(8) + value5(8) + freq_unit(3) + value6(4) +
///   bar(2) + status(8) = 61 bytes.
const LIVE_DATA_PAYLOAD_LEN: usize = 61;

/// VC-890 function code table: (code, mode_name, base_unit).
///
/// Note: function codes are DIFFERENT from VC-880 — remapped!
/// From VC890Reading.SetDeviceMode_And_Unit_And_Range() in DMSShare.dll.
const FUNCTION_TABLE: &[(u8, &str, &str)] = &[
    (0x00, "AC V", "V"),
    (0x01, "ACV LPF", "V"),
    (0x02, "DC V", "V"),
    (0x03, "AC+DC V", "V"),
    (0x04, "DC mV", "mV"),
    (0x05, "Frequency", "Hz"),
    (0x06, "Duty %", "%"),
    (0x07, "Ω", "Ω"),
    (0x08, "Continuity", "Ω"),
    (0x09, "Diode", "V"),
    (0x0A, "Capacitance", "F"),
    (0x0B, "°C", "°C"),
    (0x0C, "°F", "°F"),
    (0x0D, "DC µA", "µA"),
    (0x0E, "AC µA", "µA"),
    (0x0F, "DC mA", "mA"),
    (0x10, "AC mA", "mA"),
    (0x11, "DC A", "A"),
    (0x12, "AC A", "A"),
];

/// The range table of a function code, empty for a single-range or unknown
/// function.
///
/// VC-890 has 60,000 counts — range values are 6/60/600 (not 4/40/400). The
/// meter reports the index into this table as `range_raw - 0x30`, so it is
/// also the manual range ladder [`Setting::Range`] offers.
fn range_table(function: u8) -> &'static [RangeEntry] {
    // `const` items rather than inline literals: a `&[...]` expression would
    // be a temporary this function cannot return.
    const VOLTAGE: &[RangeEntry] = &[re("", "6V"), re("", "60V"), re("", "600V"), re("", "1000V")];
    const MILLIVOLTS: &[RangeEntry] = &[re("", "600mV")];
    const FREQUENCY: &[RangeEntry] = &[
        re("Hz", "60Hz"),
        re("Hz", "600Hz"),
        re("kHz", "6kHz"),
        re("kHz", "60kHz"),
        re("kHz", "600kHz"),
        re("MHz", "6MHz"),
        re("MHz", "60MHz"),
        re("MHz", "600MHz"),
    ];
    const RESISTANCE: &[RangeEntry] = &[
        re("\u{03A9}", "600\u{03A9}"),
        re("k\u{03A9}", "6k\u{03A9}"),
        re("k\u{03A9}", "60k\u{03A9}"),
        re("k\u{03A9}", "600k\u{03A9}"),
        re("M\u{03A9}", "6M\u{03A9}"),
        re("M\u{03A9}", "60M\u{03A9}"),
    ];
    const CAPACITANCE: &[RangeEntry] = &[
        re("nF", "60nF"),
        re("nF", "600nF"),
        re("\u{00B5}F", "6\u{00B5}F"),
        re("\u{00B5}F", "60\u{00B5}F"),
        re("\u{00B5}F", "600\u{00B5}F"),
        re("\u{00B5}F", "6000\u{00B5}F"),
        re("mF", "60mF"),
    ];
    const MICROAMPS: &[RangeEntry] = &[re("", "600\u{00B5}A"), re("", "6000\u{00B5}A")];
    const MILLIAMPS: &[RangeEntry] = &[re("", "60mA"), re("", "600mA")];
    const AMPS: &[RangeEntry] = &[re("", "10A")];

    match function {
        // ACV, DCV, AC+DC V — voltage ranges
        0x00 | 0x02 | 0x03 => VOLTAGE,
        // DC mV
        0x04 => MILLIVOLTS,
        // Frequency
        0x05 => FREQUENCY,
        // Impedance (Resistance)
        0x07 => RESISTANCE,
        // Capacitance
        0x0A => CAPACITANCE,
        // DC/AC µA
        0x0D | 0x0E => MICROAMPS,
        // DC/AC mA
        0x0F | 0x10 => MILLIAMPS,
        // DC/AC A
        0x11 | 0x12 => AMPS,
        // Single-range functions, ACV LPF among them (see `lookup_range`)
        _ => &[],
    }
}

/// Look up range info for a function code and range index.
fn lookup_range(function: u8, range_idx: u8) -> Option<(&'static str, &'static str)> {
    // ACV LPF: vendor never reads the range byte and fixes the range at
    // 1000V (DMSShare_decompiled.cs:23466-23469, `case 1`), unlike the other
    // voltage functions. What the meter sends in the range byte for LPF is
    // unknown — accept any index. `range_table` reports no ladder for it, so
    // no range choice is offered there either.
    if function == 0x01 {
        return Some(("V", "1000V"));
    }
    resolve_range(range_table(function), range_idx, FUNCTION_TABLE, function)
}

use super::vc8x0_common::COMMANDS as VC890_COMMANDS;

/// The dial, position by position, and the functions SHIFT/SETUP reaches on
/// each -- [MANUAL], no hardware has confirmed it.
///
/// From Fig. 1 on printed page 54 of the English VC-890 operating
/// instructions (the red symbols beside a position are its SHIFT/SETUP
/// sub-functions) and the §11 measurement procedures. Membership only: the
/// manual says which symbol each position offers ("press … until the symbol
/// appears"), never the press order, so the driver presses and reads the mode
/// back. See `docs/research/vc890/reverse-engineered-protocol.md`.
///
/// LoZ is not here: it is a separate "Low Imp. 400 kΩ" button and a status
/// flag, not a function code.
const DIAL: &[DialPosition] = &[
    // V~ (red Lo): AC V, ACV low-pass
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x00, 0x01],
        }],
    },
    // V⎓ (red AC+DC): DC V, AC+DC V
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x02, 0x03],
        }],
    },
    // mV⎓ / Hz % : DC mV, Frequency, Duty %
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x04, 0x05, 0x06],
        }],
    },
    // Ω (red diode, continuity) — listed in the order §11f/§11g describe them
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x07, 0x09, 0x08],
        }],
    },
    // ⊣⊢ : Capacitance
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0A],
        }],
    },
    // °C°F
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0B, 0x0C],
        }],
    },
    // µA≂ : DC µA, AC µA
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0D, 0x0E],
        }],
    },
    // mA≂ : DC mA, AC mA
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0F, 0x10],
        }],
    },
    // A≂ : DC A, AC A
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x11, 0x12],
        }],
    },
];

/// How long a press takes to show up in a reading. Not hardware-tuned:
/// nobody has timed a real VC-890.
///
/// No delay, because this meter is polled and a read is a full request; two
/// of them is the budget, since each already costs two ack bursts (400 ms of
/// vendor-prescribed sleeps) before the meter answers.
const SETTLE: Settle = Settle {
    delay: Duration::ZERO,
    reads: 2,
};

/// Protocol implementation for the Voltcraft VC-890.
pub struct Vc890Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// Which dial position the readings say the meter is on, for the mode
    /// driver in [`crate::protocol::cycle`].
    dial: DialState,
}

impl Default for Vc890Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Vc890Protocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            profile: DeviceProfile {
                family_name: "VC890",
                model_name: "Voltcraft VC-890",
                stability: Stability::Experimental,
                supported_commands: VC890_COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(14),
            },
            dial: DialState::default(),
        }
    }

    /// Write one command frame the way every VC-890 exchange starts: the
    /// vendor's pre-clear ack burst (`WriteCommand(cmd, ack: true)` →
    /// `AckMessage(clear: true)`), then the frame itself.
    ///
    /// `send_command` and the mode driver's `press` both go through here so
    /// the two can't drift apart.
    fn write_command(&self, transport: &dyn Transport, cmd: u8) -> Result<()> {
        send_ack_sequence(transport)?;
        transport.write(&super::vc8x0_common::build_command(cmd))
    }
}

impl Protocol for Vc890Protocol {
    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // VC-890 is polled — no init needed. The meter responds to
        // individual measurement requests (0x5E).
        debug!("vc890: init (polled model, no trigger needed)");
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // Vendor wraps every request/response in an ack burst — see the
        // spec's "Ack protocol" section and `send_ack_sequence` above.
        send_ack_sequence(transport)?;

        // Send measurement request (0x5E) — same command as UT61E+.
        let request = super::vc8x0_common::build_command(CMD_GET_MEASUREMENT);
        transport.write(&request)?;

        // Read response.
        let payload = framing::read_frame(
            &mut self.rx_buf,
            transport,
            framing::extract_frame_abcd_be16,
            |p| !p.is_empty() && p[0] == MSG_TYPE_LIVE_DATA,
            FrameErrorRecovery::SkipAndRetry,
            "vc890",
            &framing::HEADER,
        )?;

        // Post-confirm ack after a valid frame is reassembled.
        send_ack_sequence(transport)?;

        let measurement = parse_measurement(&payload)?;
        // Readings are the only place the meter states its function, so
        // every one of them is what keeps the dial position current.
        self.dial.observe(DIAL, measurement.mode_raw);
        Ok(measurement)
    }

    fn send_command(&mut self, transport: &dyn Transport, command: &str) -> Result<()> {
        let cmd_byte = super::vc8x0_common::command_byte(command)?;
        debug!("vc890: sending command {command} ({cmd_byte:#04x})");
        self.write_command(transport, cmd_byte)
    }

    fn get_name(&mut self, transport: &dyn Transport) -> Result<Option<String>> {
        // The shared helper is VC-880-shaped and writes the GetDeviceID frame
        // bare. The VC-890 needs the ack burst first, like its every other
        // exchange: the vendor's `GetDeviceID()` calls `WriteCommand(0)`
        // (DMSShare_decompiled.cs:3895), and that overload is
        // `WriteCommand(command, ack: true)` (:3800), whose body opens with
        // `AckMessage(clear: true)` (:3775).
        //
        // The vendor also retries the whole exchange up to 10 times with a
        // FlushBuffer between attempts; we make one. See the VC-890 entry in
        // docs/verification-backlog.md.
        send_ack_sequence(transport)?;
        super::vc8x0_common::read_device_name(&mut self.rx_buf, transport, "vc890")
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        let mut steps = super::vc8x0_common::capture_steps();
        // The battery nibble (payload byte 59) is in raw_hex on every
        // sample already, but nothing records what the meter itself was
        // showing, so the values can't be interpreted. Ask for that here:
        // the step's screen-confirmation prompt is where the answer lands.
        // We currently treat 0 as "empty", which is a guess — see the
        // VC-890 entry in docs/verification-backlog.md.
        steps.push(crate::protocol::CaptureStep {
            id: "battery",
            instruction: "Any mode. When prompted, type the battery indicator \
                          shown on the meter (e.g. \"full\", \"2 of 3 bars\", \
                          \"low-battery symbol lit\").",
            command: None,
            samples: 3,
        });
        steps
    }

    fn choices(&self, setting: Setting, current: &Measurement) -> Vec<Choice> {
        match setting {
            Setting::Mode => cycle::mode_choices(self, current),
            Setting::Range => cycle::range_choices(self, current),
            _ => Vec::new(),
        }
    }

    fn select(&mut self, transport: &dyn Transport, setting: Setting, id: u16) -> Result<()> {
        match setting {
            Setting::Mode => cycle::select_mode(self, transport, id),
            Setting::Range => cycle::select_range(self, transport, id),
            _ => Err(unsupported_setting(setting)),
        }
    }
}

impl CycleMeter for Vc890Protocol {
    fn dial_positions(&self) -> &'static [DialPosition] {
        DIAL
    }

    fn dial_state(&self) -> &DialState {
        &self.dial
    }

    fn dial_state_mut(&mut self) -> &mut DialState {
        &mut self.dial
    }

    fn press(&mut self, transport: &dyn Transport, button: CycleButton) -> Result<()> {
        let (name, cmd) = match button {
            CycleButton::Select => (SELECT_BUTTON_NAME, CMD_SELECT),
            CycleButton::Range => (RANGE_BUTTON_NAME, CMD_RANGE_MANUAL),
            CycleButton::Hz => {
                return Err(Error::UnsupportedCommand(format!(
                    "the VC-890 has no {} button",
                    self.button_name(button)
                )));
            }
        };
        debug!("vc890: pressing {name} ({cmd:#04x})");
        // Nothing to read back here: the meter echoes the command in a frame
        // of its own, and `request_measurement`'s accept filter (type byte
        // 0x01) already skips it.
        self.write_command(transport, cmd)
    }

    fn read(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        Protocol::request_measurement(self, transport)
    }

    fn mode_label(&self, mode: u16) -> Cow<'static, str> {
        resolve_function(FUNCTION_TABLE, mode as u8, "vc890").0
    }

    fn settle(&self) -> Settle {
        SETTLE
    }

    fn button_name(&self, button: CycleButton) -> &'static str {
        match button {
            CycleButton::Select => SELECT_BUTTON_NAME,
            CycleButton::Range => RANGE_BUTTON_NAME,
            CycleButton::Hz => "Hz/%",
        }
    }

    fn range_ladder(&self, mode: u16) -> Vec<Cow<'static, str>> {
        match u8::try_from(mode) {
            Ok(function) => super::vc8x0_common::range_ladder(range_table(function)),
            Err(_) => Vec::new(),
        }
    }

    /// The range byte is 0x30-based ASCII, so rung 1 arrives as 0x30.
    fn range_rung(&self, range_raw: u8) -> u16 {
        u16::from(range_raw.wrapping_sub(0x30)) + 1
    }

    fn set_auto_range(&mut self, transport: &dyn Transport) -> Result<()> {
        debug!("vc890: sending auto-range ({CMD_RANGE_AUTO:#04x})");
        self.write_command(transport, CMD_RANGE_AUTO)
    }
}

/// Parse a VC890 live data payload (61+ bytes).
///
/// Frame layout (from VC890Reading.SetReadingValue + SetStatus):
///   payload[0]     = type byte (0x01)
///   payload[1]     = function code (0x00-0x12)
///   payload[2]     = range byte (0x30-based)
///   payload[3..10] = value 1: main display (7 ASCII bytes)
///   payload[10..18]= value 2: sub display (8 ASCII bytes)
///   payload[18..28]= value 3: (10 bytes)
///   payload[28..36]= value 4: (8 bytes)
///   payload[36..44]= value 5: (8 bytes)
///   payload[44..47]= second freq unit (3 bytes)
///   payload[47..51]= value 6: (4 bytes)
///   payload[51..53]= bar graph (2 bytes)
///   payload[53]    = status 0: COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
///   payload[54]    = status 1: Rel(0), Avg(1), Min(2), Max(3)
///   payload[55]    = status 2: Hold(0), Manual(1), OL1(2), OL2(3)
///   payload[56]    = status 3: AutoPower(0), Warning(1), Loz(2), Void(3)
///   payload[57]    = status 4: OuterSel(0), Pass(1), Comp(2), Log_h(3)
///   payload[58]    = status 5: Mem(0), BarPol(1), Clr(2), Shift(3)
///   payload[59]    = battery level (low nibble)
///   payload[60]    = misplug warning (low nibble: 0=none, 1=mA err, 2=A err,
///     3=V err — DMSShare_decompiled.cs:23649-23665)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len("vc890", payload, LIVE_DATA_PAYLOAD_LEN)?;

    let function_code = payload[1];
    let range_raw = payload[2];
    let main_bytes = &payload[3..10];

    // Status bytes start at payload[53] (msg[56] in the raw frame)
    let status_bytes = &payload[53..61];

    let (mode, base_unit) = resolve_function(FUNCTION_TABLE, function_code, "vc890");

    // Decode range
    let range_idx = range_raw.wrapping_sub(0x30);
    let (unit, range_label) = if let Some((u, r)) = lookup_range(function_code, range_idx) {
        (u, r)
    } else {
        (base_unit, "")
    };

    // Parse main display
    let (display_str, display_trimmed) = main_display(main_bytes);

    // Extract status flags. Status bytes 1-3 here are payload[54..57]; the
    // hv_warning bit is byte 59 bit 1 of the raw frame.
    let (mut flags, ol1) = common_flags(status_bytes);
    // Spec byte 59: Loz(2), Void(3). Vendor (DMSShare_decompiled.cs:23638-23639)
    // reads both into dedicated `Loz_flag` / `Void_flag` bools.
    flags.loz = status_bytes[3] & 0x04 != 0;
    flags.void = status_bytes[3] & 0x08 != 0;
    // Battery level is the low nibble of status_bytes[6]. The vendor DLL
    // (`DMSShare_decompiled.cs:23648`, `battery_flag = msg[62] & 0xF`)
    // stores the raw 0-15 value and does not threshold it here — the
    // VoltSoft GUI (a separate binary not yet reversed) decides what
    // counts as "low". Without that threshold the previous guess of
    // `>= 3 == low` was as likely to cry wolf as to help, so we report
    // low_battery only for a fully-empty nibble. The raw level stays
    // available in raw_payload for consumers that want the gauge.
    let battery_level = status_bytes[6] & 0x0F;
    flags.low_battery = battery_level == 0;

    let value = parse_value("vc890", ol1, &display_trimmed, &display_str);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::MeasuredValue;
    use crate::protocol::test_support::snapshot;
    use crate::transport::mock::MockTransport;

    /// Build a minimal VC890 live data payload for testing.
    fn make_payload(function: u8, range: u8, main_display: &[u8; 7], status: [u8; 8]) -> Vec<u8> {
        let mut p = vec![MSG_TYPE_LIVE_DATA, function, range];
        p.extend_from_slice(main_display); // value1 (7)
        p.extend_from_slice(b"        "); // value2 (8)
        p.extend_from_slice(b"          "); // value3 (10)
        p.extend_from_slice(b"        "); // value4 (8)
        p.extend_from_slice(b"        "); // value5 (8)
        p.extend_from_slice(b"   "); // freq_unit (3)
        p.extend_from_slice(b"    "); // value6 (4)
        p.extend_from_slice(b"  "); // bar (2)
        p.extend_from_slice(&status); // status (8 bytes: 6 flag bytes + battery + misplug)
        assert_eq!(p.len(), LIVE_DATA_PAYLOAD_LEN);
        p
    }

    fn zero_status() -> [u8; 8] {
        [0u8; 8]
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x02, 0x31, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "60V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        // Note: VC-890 function 0x00 = ACV (different from VC-880!)
        let payload = make_payload(0x00, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "6V");
    }

    #[test]
    fn parse_acv_lpf_fixed_range() {
        // Vendor fixes LPF at 1000V and never reads the range byte
        // (DMSShare_decompiled.cs:23466-23469) — any index must work.
        for range_byte in [0x30, 0x33, 0x00, 0xFF] {
            let payload = make_payload(0x01, range_byte, b" 230.45", zero_status());
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.mode, "ACV LPF");
            assert_eq!(m.unit, "V");
            assert_eq!(m.range_label, "1000V");
        }
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x07, 0x32, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "60kΩ");
    }

    #[test]
    fn parse_overload_flag() {
        let mut status = zero_status();
        status[2] = 0x04; // OL1
        let payload = make_payload(0x07, 0x30, b"     OL", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_hold_flag() {
        let mut status = zero_status();
        status[2] = 0x01;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        let mut status = zero_status();
        status[1] = 0x01;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        let mut status = zero_status();
        status[1] = 0x08; // Max
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);

        let mut status = zero_status();
        status[1] = 0x04; // Min
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.min);
    }

    #[test]
    fn parse_auto_range() {
        let payload = make_payload(0x02, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        let mut status = zero_status();
        status[2] = 0x02; // Manual bit
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_all_valid_functions() {
        for &(code, _, _) in FUNCTION_TABLE {
            let payload = make_payload(code, 0x30, b"  1.234", zero_status());
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !m.mode.starts_with("Unknown"),
                "function {code:#04x} should be known"
            );
        }
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x01, 0x00, 0x30];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn range_60k_counts() {
        // Verify 60K count range values (6/60/600 instead of 4/40/400)
        let payload = make_payload(0x02, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_label, "6V"); // not 4V like VC-880

        let payload = make_payload(0x07, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_label, "600Ω"); // not 400Ω like VC-880
    }

    #[test]
    fn function_codes_differ_from_vc880() {
        // VC-890: 0x00 = ACV, 0x02 = DCV
        // VC-880: 0x00 = DCV, 0x05 = ACV
        let payload = make_payload(0x00, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V"); // NOT "DC V"
    }

    #[test]
    fn send_command_builds_correct_frame() {
        let frame = super::super::vc8x0_common::build_command(CMD_GET_MEASUREMENT);
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xCD);
        assert_eq!(frame[2], 0x03);
        assert_eq!(frame[3], CMD_GET_MEASUREMENT);
        let sum: u16 = frame[..4].iter().map(|&b| b as u16).sum();
        assert_eq!(frame[4], (sum >> 8) as u8);
        assert_eq!(frame[5], (sum & 0xFF) as u8);
    }

    #[test]
    fn parse_loz_flag() {
        // Byte 59 bit 2 = LoZ. Maps to status_bytes[3] & 0x04.
        let mut status = zero_status();
        status[3] = 0x04;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.loz);
        assert!(!m.flags.void);
    }

    #[test]
    fn parse_void_flag() {
        // Byte 59 bit 3 = Void. Maps to status_bytes[3] & 0x08.
        let mut status = zero_status();
        status[3] = 0x08;
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.void);
        assert!(!m.flags.loz);
    }

    #[test]
    fn parse_loz_and_void_together() {
        let mut status = zero_status();
        status[3] = 0x0C; // bits 2 and 3
        let payload = make_payload(0x02, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.loz);
        assert!(m.flags.void);
    }

    #[test]
    fn ack_frame_matches_vendor_bytes() {
        // AB + CD + 04 + FF + 00 = 0x027B (BE checksum)
        assert_eq!(ACK_FRAME, [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B]);
        let sum: u16 = ACK_FRAME[..5].iter().map(|&b| b as u16).sum();
        assert_eq!(ACK_FRAME[5], (sum >> 8) as u8);
        assert_eq!(ACK_FRAME[6], (sum & 0xFF) as u8);
    }

    /// The vendor sends the ack burst before GetDeviceID like every other
    /// exchange (`WriteCommand(0)` → `WriteCommand(cmd, ack: true)` →
    /// `AckMessage(clear: true)`, DMSShare_decompiled.cs:3895/3800/3775).
    /// Ours went through the VC-880-shaped helper, which writes bare.
    #[test]
    fn get_name_sends_the_ack_burst_first() {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc890Protocol::new();
        // No response queued, so this fails after the writes — the writes are
        // what we're checking.
        let _ = proto.get_name(&transport);

        let writes = transport.written.borrow();
        assert!(
            writes.len() >= 4,
            "expected 3 ack frames then the GetDeviceID frame, got {}",
            writes.len()
        );
        for (i, w) in writes.iter().take(3).enumerate() {
            assert_eq!(w.as_slice(), ACK_FRAME, "write {i} should be an ack frame");
        }
        // GetDeviceID is command byte 0x00.
        assert_eq!(writes[3][3], 0x00, "fourth write should be GetDeviceID");
    }

    #[test]
    fn send_ack_sequence_writes_three_copies() {
        let transport = MockTransport::new(vec![]);
        send_ack_sequence(&transport).unwrap();
        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 3, "ack should be sent three times");
        for w in writes.iter() {
            assert_eq!(w.as_slice(), &ACK_FRAME);
        }
    }

    /// Every status byte 0xFF: OL1 forces Overload, the manual bit clears
    /// AUTO, hold/rel/min/max/HV/LoZ/VOID light — and low_battery stays off,
    /// because the battery nibble reads 0xF (full), not 0 (empty).
    #[test]
    fn snapshot_every_status_bit_set() {
        let payload = make_payload(0x00, b'1', b"-1.2345", [0xFF; 8]);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=60V
display_raw=Some("-1.2345")
flags=hold,rel,min,max,hv_warning,loz,void
aux=0
raw_payload=61"#
        );
    }

    /// Every status byte clear: AUTO on, and low_battery ON because the
    /// battery nibble is 0.
    #[test]
    fn snapshot_zero_status() {
        let payload = make_payload(0x00, b'1', b"-1.2345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x31
value=Normal(-1.2345)
unit=V
range_label=60V
display_raw=Some("-1.2345")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// Overload spelled out in the digits rather than flagged by OL1.
    #[test]
    fn snapshot_overload_display_string() {
        let payload = make_payload(0x07, b'0', b"     OL", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x07
range_raw=0x30
value=Overload
unit=Ω
range_label=600Ω
display_raw=Some("     OL")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// The meter's "---" blank-reading form also reads as overload.
    #[test]
    fn snapshot_dashes_display() {
        let payload = make_payload(0x07, b'0', b"    ---", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x07
range_raw=0x30
value=Overload
unit=Ω
range_label=600Ω
display_raw=Some("    ---")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// An all-spaces display falls back to Overload.
    #[test]
    fn snapshot_blank_display() {
        let payload = make_payload(0x00, b'1', b"       ", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=60V
display_raw=Some("       ")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// Digits that are not a number: same Overload fallback as a blank one.
    #[test]
    fn snapshot_unparsable_display() {
        let payload = make_payload(0x00, b'1', b"1.2.3.4", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=60V
display_raw=Some("1.2.3.4")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// A function code outside FUNCTION_TABLE.
    #[test]
    fn snapshot_unknown_function_code() {
        let payload = make_payload(0x7F, b'0', b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x7f)
mode_raw=0x7f
range_raw=0x30
value=Normal(1.234)
unit=
range_label=
display_raw=Some("  1.234")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// Range byte b'9' indexes past the 4-entry voltage table.
    #[test]
    fn snapshot_range_index_past_the_table() {
        let payload = make_payload(0x00, b'9', b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x39
value=Normal(1.234)
unit=V
range_label=
display_raw=Some("  1.234")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// DC mV has a one-entry range table.
    #[test]
    fn snapshot_single_range_function() {
        let payload = make_payload(0x04, b'0', b" 123.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC mV
mode_raw=0x04
range_raw=0x30
value=Normal(123.45)
unit=mV
range_label=600mV
display_raw=Some(" 123.45")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// Status byte 3 bit 2 alone: LoZ set, VOID clear.
    #[test]
    fn snapshot_loz_only() {
        let mut status = zero_status();
        status[3] = 0x04;
        let payload = make_payload(0x02, b'0', b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x30
value=Normal(1.234)
unit=V
range_label=6V
display_raw=Some("  1.234")
flags=auto_range,low_battery,loz
aux=0
raw_payload=61"#
        );
    }

    /// Status byte 3 bit 3 alone: VOID set, LoZ clear.
    #[test]
    fn snapshot_void_only() {
        let mut status = zero_status();
        status[3] = 0x08;
        let payload = make_payload(0x02, b'0', b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x30
value=Normal(1.234)
unit=V
range_label=6V
display_raw=Some("  1.234")
flags=auto_range,low_battery,void
aux=0
raw_payload=61"#
        );
    }

    /// Battery nibble 0x0 is the only value we report as low battery.
    #[test]
    fn snapshot_battery_nibble_empty() {
        let mut status = zero_status();
        status[6] = 0x00;
        let payload = make_payload(0x02, b'0', b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x30
value=Normal(1.234)
unit=V
range_label=6V
display_raw=Some("  1.234")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// Battery nibble 0x1 — the lowest non-empty level — is not reported as
    /// low battery: the vendor's threshold is unknown (see the VC-890 entry
    /// in docs/verification-backlog.md).
    #[test]
    fn snapshot_battery_nibble_one() {
        let mut status = zero_status();
        status[6] = 0x01;
        let payload = make_payload(0x02, b'0', b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x30
value=Normal(1.234)
unit=V
range_label=6V
display_raw=Some("  1.234")
flags=auto_range
aux=0
raw_payload=61"#
        );
    }

    /// Battery nibble 0xF: not low.
    #[test]
    fn snapshot_battery_nibble_full() {
        let mut status = zero_status();
        status[6] = 0x0F;
        let payload = make_payload(0x02, b'0', b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x02
range_raw=0x30
value=Normal(1.234)
unit=V
range_label=6V
display_raw=Some("  1.234")
flags=auto_range
aux=0
raw_payload=61"#
        );
    }

    /// ACV LPF ignores the range byte and is fixed at 1000V.
    #[test]
    fn snapshot_acv_lpf_low_range_byte() {
        let payload = make_payload(0x01, b'0', b" 230.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=ACV LPF
mode_raw=0x01
range_raw=0x30
value=Normal(230.45)
unit=V
range_label=1000V
display_raw=Some(" 230.45")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// The same, with a range byte no other function's table would accept.
    #[test]
    fn snapshot_acv_lpf_high_range_byte() {
        let payload = make_payload(0x01, b'7', b" 230.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=ACV LPF
mode_raw=0x01
range_raw=0x37
value=Normal(230.45)
unit=V
range_label=1000V
display_raw=Some(" 230.45")
flags=auto_range,low_battery
aux=0
raw_payload=61"#
        );
    }

    /// The whole range table as one string: every function code crossed with
    /// every range index, rendered `unit|range_label` (`-` where the lookup
    /// returns None). Pins both the labels and the unit-override fallback.
    #[test]
    fn range_table_snapshot() {
        let table: Vec<String> = (0x00u8..=0x12)
            .map(|function| {
                let cells: Vec<String> = (0u8..=8)
                    .map(|range_idx| match lookup_range(function, range_idx) {
                        Some((unit, label)) => format!("{unit}|{label}"),
                        None => "-".to_string(),
                    })
                    .collect();
                format!("{function:#04x}: {}", cells.join(" "))
            })
            .collect();
        assert_eq!(
            table.join("\n"),
            r#"0x00: V|6V V|60V V|600V V|1000V - - - - -
0x01: V|1000V V|1000V V|1000V V|1000V V|1000V V|1000V V|1000V V|1000V V|1000V
0x02: V|6V V|60V V|600V V|1000V - - - - -
0x03: V|6V V|60V V|600V V|1000V - - - - -
0x04: mV|600mV - - - - - - - -
0x05: Hz|60Hz Hz|600Hz kHz|6kHz kHz|60kHz kHz|600kHz MHz|6MHz MHz|60MHz MHz|600MHz -
0x06: - - - - - - - - -
0x07: Ω|600Ω kΩ|6kΩ kΩ|60kΩ kΩ|600kΩ MΩ|6MΩ MΩ|60MΩ - - -
0x08: - - - - - - - - -
0x09: - - - - - - - - -
0x0a: nF|60nF nF|600nF µF|6µF µF|60µF µF|600µF µF|6000µF mF|60mF - -
0x0b: - - - - - - - - -
0x0c: - - - - - - - - -
0x0d: µA|600µA µA|6000µA - - - - - - -
0x0e: µA|600µA µA|6000µA - - - - - - -
0x0f: mA|60mA mA|600mA - - - - - - -
0x10: mA|60mA mA|600mA - - - - - - -
0x11: A|10A - - - - - - - -
0x12: A|10A - - - - - - - -"#
        );
    }

    // ---- dial table and mode switching ----

    /// One reading through `request_measurement`, the way the mode driver
    /// gets its picture of the dial.
    ///
    /// The frame is fed in two chunks: `read_frame` reads at most 64 bytes at
    /// a time and a VC-890 live-data frame is 66.
    fn read_one(function: u8) -> (Vc890Protocol, Measurement) {
        let payload = make_payload(function, 0x30, b"  1.234", zero_status());
        let frame = framing::test_frame_be16(&payload);
        let (head, tail) = frame.split_at(32);
        let transport = MockTransport::new(vec![head.to_vec(), tail.to_vec()]);
        let mut proto = Vc890Protocol::new();
        let m = proto
            .request_measurement(&transport)
            .expect("the frame parses");
        (proto, m)
    }

    fn ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().map(|c| c.id).collect()
    }

    fn labels(choices: &[Choice]) -> Vec<String> {
        choices.iter().map(|c| c.label.to_string()).collect()
    }

    fn current_ids(choices: &[Choice]) -> Vec<u16> {
        choices.iter().filter(|c| c.current).map(|c| c.id).collect()
    }

    #[test]
    fn the_dial_table_is_well_formed() {
        cycle::assert_table_invariants(
            DIAL,
            // Unlike the VC-880, no function code is reported from two
            // positions.
            &[],
            // The VC-890 has no Hz/% button; SHIFT/SETUP is the only one.
            &[CycleButton::Select],
            &|mode| resolve_function(FUNCTION_TABLE, mode as u8, "vc890").0,
        );
    }

    /// V~ carries the low-pass filter as its SHIFT/SETUP sub-function — on
    /// the VC-880 that is a dial position of its own.
    #[test]
    fn the_ac_volts_position_lists_the_low_pass_filter() {
        assert_eq!(DIAL[0].modes(), vec![0x00, 0x01]);
    }

    /// AC+DC sits on V⎓, as it does on the VC-880 — but under different
    /// function codes.
    #[test]
    fn the_dc_volts_position_lists_ac_dc() {
        assert_eq!(DIAL[1].modes(), vec![0x02, 0x03]);
    }

    #[test]
    fn a_reading_records_the_dial_position() {
        let (proto, _) = read_one(0x07);
        assert_eq!(proto.dial.last_mode(), Some(0x07));
        assert_eq!(proto.dial.position(), Some(3), "the Ω position");
    }

    #[test]
    fn the_ohm_position_lists_diode_and_continuity() {
        let (proto, m) = read_one(0x07);
        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(ids(&choices), vec![0x07, 0x09, 0x08]);
        assert_eq!(labels(&choices), ["Ω", "Diode", "Continuity"]);
        assert_eq!(current_ids(&choices), vec![0x07]);
    }

    // --- Range selection --------------------------------------------------

    /// A status block with the MANUAL range bit set (status byte 2, bit 1).
    fn manual_status() -> [u8; 8] {
        let mut s = zero_status();
        s[2] |= 0x02;
        s
    }

    #[test]
    fn the_resistance_ladder_is_listed_with_auto_first() {
        // Range byte 0x32 is the third rung, 60kΩ.
        let m = parse_measurement(&make_payload(0x07, 0x32, b" 12.345", manual_status()))
            .expect("the frame parses");
        let proto = Vc890Protocol::new();
        let choices = proto.choices(Setting::Range, &m);
        assert_eq!(ids(&choices), vec![0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(
            labels(&choices),
            ["Auto", "600Ω", "6kΩ", "60kΩ", "600kΩ", "6MΩ", "60MΩ"]
        );
        assert_eq!(current_ids(&choices), vec![3]);
    }

    /// DC mV has one range, and the vendor fixes ACV LPF at 1000V without
    /// ever reading the range byte: neither offers a choice.
    #[test]
    fn single_range_functions_offer_no_ranges() {
        let proto = Vc890Protocol::new();
        for function in [0x01, 0x04, 0x08] {
            let m = parse_measurement(&make_payload(function, 0x30, b"  1.234", manual_status()))
                .expect("the frame parses");
            assert!(
                proto.choices(Setting::Range, &m).is_empty(),
                "function {function:#04x} should offer no range"
            );
        }
    }

    #[test]
    fn pressing_range_sends_the_ack_burst_then_the_range_frame() {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc890Protocol::new();
        proto
            .press(&transport, CycleButton::Range)
            .expect("the frame is written");
        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 4, "3 ack frames then RANGE: {writes:02X?}");
        assert_eq!(
            writes[3],
            super::super::vc8x0_common::build_command(0x46),
            "RANGE is command 0x46"
        );
    }

    #[test]
    fn auto_range_sends_the_ack_burst_then_the_auto_frame() {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc890Protocol::new();
        proto.set_auto_range(&transport).expect("written");
        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 4);
        assert_eq!(
            writes[3],
            super::super::vc8x0_common::build_command(0x47),
            "AUTO is command 0x47"
        );
    }

    /// The ack burst goes first here too — a bare frame is what the VC-880
    /// sends, and this meter ignores it.
    #[test]
    fn pressing_select_sends_the_ack_burst_then_the_frame() {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc890Protocol::new();
        proto
            .press(&transport, CycleButton::Select)
            .expect("the frames are written");

        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 4, "three ack frames then SHIFT/SETUP");
        for (i, w) in writes.iter().take(3).enumerate() {
            assert_eq!(w.as_slice(), ACK_FRAME, "write {i} should be an ack frame");
        }
        assert_eq!(
            writes[3],
            super::super::vc8x0_common::build_command(0x4C),
            "SHIFT/SETUP is command 0x4C"
        );
    }

    #[test]
    fn pressing_hz_is_refused_without_writing() {
        let transport = MockTransport::new(vec![]);
        let mut proto = Vc890Protocol::new();
        let err = proto.press(&transport, CycleButton::Hz).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("Hz/%")),
            "got {err:?}"
        );
        assert!(transport.written.borrow().is_empty());
    }
}
