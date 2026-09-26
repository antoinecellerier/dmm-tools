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
//! The driver itself is [`super::Vc8x0Protocol`], shared with the
//! VC-880; this module is the [`Vc8x0Model`] it runs on.
//!
//! Based on ILSpy decompilation of Voltsoft DMSShare.dll (VC890Obj,
//! VC890Reading classes).
//! See docs/research/vc890/reverse-engineered-protocol.md

use super::{
    CMD_EXIT_MAX_MIN_AVG, CMD_HOLD, CMD_LIGHT, CMD_MAX_MIN_AVG, CMD_RANGE_AUTO, CMD_RANGE_MANUAL,
    CMD_REL, CMD_SELECT, COMMANDS, RangeEntry, Vc8x0Model, Vc8x0Protocol, build_command, re,
    read_live, resolve_range,
};
use crate::error::Result;
use crate::flags::StatusFlags;
use crate::protocol::cycle::{CycleButton, DialPosition, Ring, Settle};
use crate::protocol::framing;
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{CaptureStep, DeviceProfile, MeterKeys, Stability};
use crate::transport::Transport;
use std::thread;
use std::time::Duration;

/// Ack frame sent as both a pre-clear (before each command) and a
/// post-confirm (after each received frame). Decoded from
/// `DMSShare_decompiled.cs:3861` (`AckMessage`) plus the `WriteCommand`
/// builder at line 3805: command = `0xFF`, data = `[0x00]`, header
/// `0xAB 0xCD`, length byte `0x04` (= 2 + cmd + 1 data), checksum
/// `0xAB + 0xCD + 0x04 + 0xFF + 0x00 = 0x027B` (BE).
///
/// `pub(crate)` because detection sends the same burst before its poll.
pub(crate) const ACK_FRAME: [u8; 7] = framing::build_abcd_be16(0xFF, &[0x00]);

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

/// Measurement request command (polled model).
const CMD_GET_MEASUREMENT: u8 = 0x5E;

/// The poll frame [`request_live`] asks for a reading with. `pub(crate)`
/// alongside [`ACK_FRAME`]: detection sends the pair as its VC-890 probe.
pub(crate) const POLL_FRAME: [u8; 6] = build_command(CMD_GET_MEASUREMENT);

/// Ask the meter for a reading: the vendor's pre-clear ack burst, then the
/// poll. Shared with detection, whose probe is exactly this and nothing else
/// — a meter that answers it has already said what it is.
pub(crate) fn request_live(transport: &dyn Transport) -> Result<()> {
    send_ack_sequence(transport)?;
    transport.write(&POLL_FRAME)
}

/// Minimum payload length for a VC890 live data frame.
/// Payload = type(1) + function(1) + range(1) + value1(7) + value2(8) +
///   value3(10) + value4(8) + value5(8) + freq_unit(3) + value6(4) +
///   bar(2) + status(8) = 61 bytes.
const LIVE_DATA_PAYLOAD_LEN: usize = 61;

/// The last misplug warning value the spec gives (byte 63: 3 = V input).
const MISPLUG_V_ERROR: u8 = 3;

/// Every command byte this driver sends besides the ack and GetDeviceID,
/// whose types the spec's message list already has: the bytes of
/// [`COMMANDS`], and the poll.
const ECHOED_COMMANDS: [u8; 9] = [
    CMD_EXIT_MAX_MIN_AVG,
    CMD_RANGE_MANUAL,
    CMD_RANGE_AUTO,
    CMD_REL,
    CMD_MAX_MIN_AVG,
    CMD_HOLD,
    CMD_LIGHT,
    CMD_SELECT,
    CMD_GET_MEASUREMENT,
];

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
/// also the manual range ladder [`crate::protocol::Setting::Range`] offers.
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

/// What the shared Voltcraft driver needs to speak VC-890.
///
/// Live data payload (from VC890Reading.SetReadingValue + SetStatus):
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
pub(crate) struct Vc890Model;

impl Vc8x0Model for Vc890Model {
    const LOG: &'static str = "vc890";
    const NAME: &'static str = "VC-890";
    const DETECTED_ID: &'static str = super::devices::VC890.id;
    const PAYLOAD_LEN: usize = LIVE_DATA_PAYLOAD_LEN;
    const STATUS_AT: usize = 53;
    const DIAL: &'static [DialPosition] = DIAL;
    const SETTLE: Settle = SETTLE;
    const FUNCTION_TABLE: &'static [(u8, &'static str, &'static str)] = FUNCTION_TABLE;
    // Spec "Range Tables": the vendor fixes ACV LPF at 1000 V without
    // reading the range byte.
    const LOW_PASS: u8 = 0x01;
    // Spec "Live Data Frame": bits 0-3 of frame bytes 56-61. Bytes 62 and 63
    // hold the battery and misplug nibbles.
    const NAMED_STATUS_BITS: &'static [u8] = &[0x0F; 6];
    // Spec "Communication Model": the vendor waits for a frame of the
    // command's own type after a button command. The poll is sent the same
    // way, and its answer is read as a live frame, which leaves open whether
    // an echo comes first.
    const ECHOED_COMMANDS: &'static [u8] = &ECHOED_COMMANDS;

    fn profile() -> DeviceProfile {
        DeviceProfile {
            family_name: "VC890",
            model_name: "Voltcraft VC-890",
            stability: Stability::Experimental,
            supported_commands: COMMANDS,
            max_aux_values: 0,
            verification_issue: Some(14),
            meter_keys: MeterKeys::NONE,
        }
    }

    fn range_table(function: u8) -> &'static [RangeEntry] {
        range_table(function)
    }

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

    fn extra_flags(flags: &mut StatusFlags, status: &[u8]) {
        // Spec byte 59: Loz(2), Void(3). Vendor (DMSShare_decompiled.cs:23638-23639)
        // reads both into dedicated `Loz_flag` / `Void_flag` bools.
        flags.loz = status[3] & 0x04 != 0;
        flags.void = status[3] & 0x08 != 0;
        // Battery level is the low nibble of status[6]. The vendor DLL
        // (`DMSShare_decompiled.cs:23648`, `battery_flag = msg[62] & 0xF`)
        // stores the raw 0-15 value and does not threshold it here — the
        // VoltSoft GUI (a separate binary not yet reversed) decides what
        // counts as "low". Without that threshold the previous guess of
        // `>= 3 == low` was as likely to cry wolf as to help, so we report
        // low_battery only for a fully-empty nibble. The raw level stays
        // available in raw_payload for consumers that want the gauge.
        let battery_level = status[6] & 0x0F;
        flags.low_battery = battery_level == 0;
    }

    fn report_unrecognised_status(status: &[u8]) {
        // Spec byte 63: the misplug warning is the low nibble, 0 (none) to
        // 3 (V input). The high nibble is not described.
        if status[7] & 0x0F > MISPLUG_V_ERROR {
            report_unknown(
                Self::LOG,
                "misplug nibble",
                format_args!("frame byte 63 = {:#04x}", status[7]),
            );
        }
    }

    fn extra_capture_steps() -> Vec<CaptureStep> {
        // The battery nibble (payload byte 59) is in raw_hex on every
        // sample already, but nothing records what the meter itself was
        // showing, so the values can't be interpreted. Ask for that here:
        // the step's screen-confirmation prompt is where the answer lands.
        // We currently treat 0 as "empty", which is a guess — see the
        // VC-890 entry in docs/verification-backlog.md.
        vec![
            CaptureStep::basic(
                "battery",
                "Any mode. When prompted, type the battery indicator \
                 shown on the meter (e.g. \"full\", \"2 of 3 bars\", \
                 \"low-battery symbol lit\").",
            )
            .samples(3),
        ]
    }

    /// The vendor wraps every exchange in an ack burst — see the spec's
    /// "Ack protocol" section and `send_ack_sequence` above. `get_name` needs
    /// it too: the vendor's `GetDeviceID()` calls `WriteCommand(0)`
    /// (DMSShare_decompiled.cs:3895), and that overload is
    /// `WriteCommand(command, ack: true)` (:3800), whose body opens with
    /// `AckMessage(clear: true)` (:3775).
    ///
    /// The vendor also retries the whole GetDeviceID exchange up to 10 times
    /// with a FlushBuffer between attempts; we make one. See the VC-890 entry
    /// in docs/verification-backlog.md.
    fn ack(transport: &dyn Transport) -> Result<()> {
        send_ack_sequence(transport)
    }

    /// The meter is polled: ask for a reading, then confirm the frame.
    fn read_live_frame(rx_buf: &mut Vec<u8>, transport: &dyn Transport) -> Result<Vec<u8>> {
        // The ack burst and the 0x5E request — the same command as UT61E+.
        request_live(transport)?;

        let payload = read_live::<Self>(rx_buf, transport)?;

        // Post-confirm ack after a valid frame is reassembled.
        Self::ack(transport)?;
        Ok(payload)
    }

    /// Write the frame the way every VC-890 exchange starts: the vendor's
    /// pre-clear ack burst (`WriteCommand(cmd, ack: true)` →
    /// `AckMessage(clear: true)`), then the frame itself.
    ///
    /// Nothing to read back here: the meter echoes the command in a frame of
    /// its own, and the live-frame accept filter (type byte 0x01) already
    /// skips it.
    fn write_button(_rx_buf: &mut Vec<u8>, transport: &dyn Transport, cmd: u8) -> Result<()> {
        Self::ack(transport)?;
        transport.write(&build_command(cmd))
    }
}

/// Protocol implementation for the Voltcraft VC-890.
pub(crate) type Vc890Protocol = Vc8x0Protocol<Vc890Model>;

#[cfg(test)]
mod tests {
    use super::super::parse_measurement as parse;
    use super::super::test_support::{self as shared};
    use super::*;
    use crate::measurement::{MeasuredValue, Measurement};
    use crate::protocol::Protocol;
    use crate::protocol::test_support::snapshot;
    use crate::transport::mock::MockTransport;

    /// Build a minimal VC890 live data payload for testing.
    fn make_payload(function: u8, range: u8, main_display: &[u8; 7], status: [u8; 8]) -> Vec<u8> {
        shared::make_payload::<Vc890Model>(function, range, main_display, &status)
    }

    fn zero_status() -> [u8; 8] {
        [0u8; 8]
    }

    /// Parse a VC890 live data payload; the layout is on [`Vc890Model`].
    fn parse_measurement(payload: &[u8]) -> crate::error::Result<Measurement> {
        parse::<Vc890Model>(payload)
    }

    /// One payload parsed and rendered as a snapshot string.
    fn snap(function: u8, range: u8, display: &[u8; 7], status: [u8; 8]) -> String {
        let m = parse_measurement(&make_payload(function, range, display, status))
            .expect("the frame parses");
        snapshot(&m)
    }

    /// The command frame a press or exit wrote, after the vendor ack burst
    /// every VC-890 write is wrapped in.
    fn command_frame(transport: &MockTransport) -> Vec<u8> {
        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 4, "three acks then the command");
        for (i, w) in writes[..3].iter().enumerate() {
            assert_eq!(w.as_slice(), ACK_FRAME, "write {i} should be an ack frame");
        }
        writes[3].clone()
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
        shared::assert_overload_flag::<Vc890Model>(0x07);
    }

    #[test]
    fn parse_hold_flag() {
        shared::assert_hold_flag::<Vc890Model>(0x02);
    }

    #[test]
    fn parse_rel_flag() {
        shared::assert_rel_flag::<Vc890Model>(0x02);
    }

    #[test]
    fn parse_max_min_flags() {
        shared::assert_max_min_flags::<Vc890Model>(0x02);
    }

    #[test]
    fn parse_avg_flag() {
        // Status byte 1 bit 1 = Avg (spec byte 57); Rel/Min/Max share the byte.
        shared::assert_avg_flag::<Vc890Model>(0x02);
    }

    #[test]
    fn parse_auto_range() {
        shared::assert_auto_range::<Vc890Model>(0x02);
    }

    #[test]
    fn parse_all_valid_functions() {
        shared::assert_all_functions_named::<Vc890Model>();
    }

    #[test]
    fn parse_payload_too_short() {
        shared::assert_short_payload_rejected::<Vc890Model>();
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
        shared::assert_command_frame(CMD_GET_MEASUREMENT);
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
    fn an_unknown_function_code_is_reported() {
        shared::assert_unknown_function_reported::<Vc890Model>();
    }

    #[test]
    fn a_range_byte_past_the_table_is_reported() {
        shared::assert_range_past_the_table_reported::<Vc890Model>(0x02);
    }

    /// Duty cycle, continuity and diode have no range table, and ACV LPF's
    /// range byte is never read.
    #[test]
    fn unread_range_bytes_are_not_reported() {
        shared::assert_unread_range_bytes_are_silent::<Vc890Model>(&[0x01, 0x06, 0x08, 0x09]);
    }

    #[test]
    fn a_sign_bit_without_a_minus_is_reported() {
        shared::assert_sign_bit_without_minus_reported::<Vc890Model>(56);
    }

    /// Bits 6-7 of frame bytes 56-61; bits 4-5 are left out as a possible
    /// ASCII prefix.
    #[test]
    fn undefined_status_bits_are_reported() {
        shared::assert_undefined_status_bits_reported::<Vc890Model>(56, &[0xC0; 6]);
    }

    #[test]
    fn a_live_frame_of_another_length_is_reported() {
        shared::assert_frame_length_reported::<Vc890Model>();
    }

    /// A misplug nibble past 3 is reported whatever the high nibble holds,
    /// and changes nothing in the reading.
    #[test]
    fn a_misplug_value_past_3_is_reported() {
        for value in [0x04, 0x0F, 0x34] {
            let mut status = zero_status();
            status[7] = value;
            let (m, reports) = shared::parse_capturing::<Vc890Model>(&make_payload(
                0x02, 0x30, b"  1.234", status,
            ));
            assert_eq!(
                snapshot(&m.unwrap()),
                snap(0x02, 0x30, b"  1.234", zero_status())
            );
            assert_eq!(
                reports,
                [format!(
                    "vc890: unrecognised misplug nibble: frame byte 63 = {value:#04x}"
                )]
            );
        }
    }

    #[test]
    fn an_unlisted_frame_type_is_reported() {
        assert_eq!(
            shared::frame_type_reports::<Vc890Model>(&[0x05]),
            ["vc890: unrecognised frame type: 0x05, 2 payload bytes"]
        );
    }

    /// The echo set is every command byte the driver sends that the spec's
    /// message list does not already have.
    #[test]
    fn the_echoed_commands_are_the_ones_the_driver_sends() {
        let mut sent: Vec<u8> = COMMANDS
            .iter()
            .map(|c| super::super::command_byte(c).unwrap())
            .chain([CMD_GET_MEASUREMENT])
            .collect();
        sent.sort_unstable();
        let mut echoed = ECHOED_COMMANDS.to_vec();
        echoed.sort_unstable();
        assert_eq!(echoed, sent);
    }

    /// Every function code with each of its ranges, the display forms, the
    /// named status bits, the battery and misplug nibbles, the listed frame
    /// types and the command echoes.
    #[test]
    fn documented_frames_report_nothing() {
        let reports = shared::documented_frame_reports::<Vc890Model>();
        assert!(reports.is_empty(), "{reports:?}");

        let ((), reports) = crate::protocol::capture_reports(|| {
            for nibble in 0x00..=0x0F {
                for high in [0x00, 0x30, 0xF0] {
                    let mut status = zero_status();
                    status[6] = nibble | high;
                    status[7] = (nibble & 0x03) | high;
                    parse_measurement(&make_payload(0x02, 0x30, b"  1.234", status)).unwrap();
                }
            }
        });
        assert!(reports.is_empty(), "{reports:?}");

        let mut types = vec![0x00, 0x02, 0x03, 0x04, 0xFF];
        types.extend(ECHOED_COMMANDS);
        let reports = shared::frame_type_reports::<Vc890Model>(&types);
        assert!(reports.is_empty(), "{reports:?}");
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
    /// The shared DeviceID helper writes bare, so the model's `ack` is what
    /// puts the burst in front of it.
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
    /// AUTO, hold/rel/min/max/avg/HV/LoZ/VOID light — and low_battery stays off,
    /// because the battery nibble reads 0xF (full), not 0 (empty).
    #[test]
    fn snapshot_every_status_bit_set() {
        assert_eq!(
            snap(0x00, b'1', b"-1.2345", [0xFF; 8]),
            r#"mode=AC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=60V
display_raw=Some("-1.2345")
flags=hold,rel,min,max,avg,hv_warning,loz,void
aux=0
raw_payload=61"#
        );
    }

    /// Every status byte clear: AUTO on, and low_battery ON because the
    /// battery nibble is 0.
    #[test]
    fn snapshot_zero_status() {
        assert_eq!(
            snap(0x00, b'1', b"-1.2345", zero_status()),
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
        assert_eq!(
            snap(0x07, b'0', b"     OL", zero_status()),
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
        assert_eq!(
            snap(0x07, b'0', b"    ---", zero_status()),
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
        assert_eq!(
            snap(0x00, b'1', b"       ", zero_status()),
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
        assert_eq!(
            snap(0x00, b'1', b"1.2.3.4", zero_status()),
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
        assert_eq!(
            snap(0x7F, b'0', b"  1.234", zero_status()),
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
        assert_eq!(
            snap(0x00, b'9', b"  1.234", zero_status()),
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
        assert_eq!(
            snap(0x04, b'0', b" 123.45", zero_status()),
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
        assert_eq!(
            snap(0x02, b'0', b"  1.234", status),
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
        assert_eq!(
            snap(0x02, b'0', b"  1.234", status),
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
        assert_eq!(
            snap(0x02, b'0', b"  1.234", status),
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
        assert_eq!(
            snap(0x02, b'0', b"  1.234", status),
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
        assert_eq!(
            snap(0x02, b'0', b"  1.234", status),
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
        assert_eq!(
            snap(0x01, b'0', b" 230.45", zero_status()),
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
        assert_eq!(
            snap(0x01, b'7', b" 230.45", zero_status()),
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
        assert_eq!(
            shared::render_range_table::<Vc890Model>(),
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

    #[test]
    fn the_dial_table_is_well_formed() {
        // Unlike the VC-880, no function code is reported from two positions.
        shared::assert_dial_table_is_well_formed::<Vc890Model>(&[]);
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
        // Position 3 is the Ω position.
        shared::assert_reading_records_dial_position::<Vc890Model>(0x07, 3);
    }

    #[test]
    fn the_ohm_position_lists_diode_and_continuity() {
        shared::assert_ohm_position_lists_diode_and_continuity::<Vc890Model>(
            0x07,
            &[0x07, 0x09, 0x08],
        );
    }

    // --- Range selection --------------------------------------------------

    #[test]
    fn the_resistance_ladder_is_listed_with_auto_first() {
        shared::assert_resistance_ladder::<Vc890Model>(
            0x07,
            &["600Ω", "6kΩ", "60kΩ", "600kΩ", "6MΩ", "60MΩ"],
        );
    }

    /// DC mV has one range, and the vendor fixes ACV LPF at 1000V without
    /// ever reading the range byte: neither offers a choice.
    #[test]
    fn single_range_functions_offer_no_ranges() {
        shared::assert_no_range_choices::<Vc890Model>(&[0x01, 0x04, 0x08]);
    }

    #[test]
    fn pressing_range_sends_the_ack_burst_then_the_range_frame() {
        shared::assert_range_button_writes_the_range_frame::<Vc890Model>(&command_frame);
    }

    #[test]
    fn auto_range_sends_the_ack_burst_then_the_auto_frame() {
        shared::assert_auto_range_writes_the_auto_frame::<Vc890Model>(&command_frame);
    }

    /// The ack burst goes first here too — a bare frame is what the VC-880
    /// sends, and this meter ignores it.
    #[test]
    fn pressing_select_sends_the_ack_burst_then_the_frame() {
        shared::assert_select_writes_the_shift_setup_frame::<Vc890Model>(&command_frame);
    }

    #[test]
    fn pressing_hz_is_refused_without_writing() {
        shared::assert_button_refused::<Vc890Model>(CycleButton::Hz, "Hz/%");
    }

    // --- Flag-backed settings ---------------------------------------------

    #[test]
    fn minmax_lists_the_three_step_ring_and_off() {
        shared::assert_minmax_ring::<Vc890Model>(0x02);
    }

    #[test]
    fn the_flag_buttons_write_their_own_command_bytes() {
        shared::assert_flag_buttons_write_their_bytes::<Vc890Model>(&command_frame);
    }

    #[test]
    fn leaving_minmax_writes_the_exit_frame() {
        shared::assert_leaving_minmax_writes_the_exit_frame::<Vc890Model>(&command_frame);
    }

    #[test]
    fn pressing_peak_is_refused_without_writing() {
        shared::assert_button_refused::<Vc890Model>(CycleButton::Peak, "PEAK");
    }
}
