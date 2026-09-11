//! Voltcraft VC-880 / VC650BT multimeter protocol.
//!
//! Streaming protocol: meter streams live data continuously after the user
//! presses the PC button — no trigger command needed from the host.
//!
//! Frame format: identical to UT61E+ — AB CD header, BE16 checksum.
//! Reuses `extract_frame_abcd_be16()`.
//!
//! Live data frame (39 bytes): header(2) + length(1) + type(1) +
//!   function(1) + range(1) + main_value(7) + sub1(7) + sub2(7) +
//!   bar(3) + status(7) + checksum(2).
//!
//! The driver itself is [`super::Vc8x0Protocol`], shared with the
//! VC-890; this module is the [`Vc8x0Model`] it runs on.
//!
//! Based on ILSpy decompilation of Voltsoft DMSShare.dll.
//! See docs/research/vc880/reverse-engineered-protocol.md

use super::{COMMANDS, RangeEntry, Vc8x0Model, Vc8x0Protocol, build_command, re, read_live};
use crate::error::Result;
use crate::flags::StatusFlags;
use crate::protocol::cycle::{CycleButton, DialPosition, Ring, Settle};
use crate::protocol::{DeviceProfile, Stability};
use crate::transport::Transport;
use log::debug;
use std::time::Duration;

/// Minimum payload length for a live data frame.
/// Payload from `extract_frame_abcd_be16` = everything between length byte
/// and checksum. For a 39-byte frame: type(1) + function(1) + range(1) +
/// main(7) + sub1(7) + sub2(7) + bar(3) + status(7) = 34 bytes.
const LIVE_DATA_PAYLOAD_LEN: usize = 34;

/// Function code table: (code, mode_name, unit).
///
/// From `SetDeviceMode_And_Unit_And_Range()` switch statement in
/// DMSShare.dll (line 16335). The unit here is the base unit; the
/// actual unit depends on the range byte (e.g., function 0x06 can be
/// Ω, kΩ, or MΩ depending on range).
const FUNCTION_TABLE: &[(u8, &str, &str)] = &[
    (0x00, "DC V", "V"),
    (0x01, "AC+DC V", "V"),
    (0x02, "DC mV", "mV"),
    (0x03, "Frequency", "Hz"),
    (0x04, "Duty %", "%"),
    (0x05, "AC V", "V"),
    (0x06, "Ω", "Ω"),
    (0x07, "Diode", "V"),
    (0x08, "Continuity", "Ω"),
    (0x09, "Capacitance", "F"),
    (0x0A, "°C", "°C"),
    (0x0B, "°F", "°F"),
    (0x0C, "DC µA", "µA"),
    (0x0D, "AC µA", "µA"),
    (0x0E, "DC mA", "mA"),
    (0x0F, "AC mA", "mA"),
    (0x10, "DC A", "A"),
    (0x11, "AC A", "A"),
    (0x12, "ACV LPF", "V"),
];

/// The range table of a function code, empty for a single-range or unknown
/// function.
///
/// Range tables from DMSShare.dll `SetDeviceMode_And_Unit_And_Range()`
/// and cross-referenced against VC880 user manual pages 62-65. The meter
/// reports the index into this table as `range_raw - 0x30`, so it is also
/// the manual range ladder [`crate::protocol::Setting::Range`] offers.
fn range_table(function: u8) -> &'static [RangeEntry] {
    // `const` items rather than inline literals: a `&[...]` expression would
    // be a temporary this function cannot return.
    const VOLTAGE: &[RangeEntry] = &[re("", "4V"), re("", "40V"), re("", "400V"), re("", "1000V")];
    const FREQUENCY: &[RangeEntry] = &[
        re("Hz", "40Hz"),
        re("Hz", "400Hz"),
        re("kHz", "4kHz"),
        re("kHz", "40kHz"),
        re("kHz", "400kHz"),
        re("MHz", "4MHz"),
        re("MHz", "40MHz"),
        re("MHz", "400MHz"),
    ];
    const RESISTANCE: &[RangeEntry] = &[
        re("\u{03A9}", "400\u{03A9}"),
        re("k\u{03A9}", "4k\u{03A9}"),
        re("k\u{03A9}", "40k\u{03A9}"),
        re("k\u{03A9}", "400k\u{03A9}"),
        re("M\u{03A9}", "4M\u{03A9}"),
        re("M\u{03A9}", "40M\u{03A9}"),
    ];
    const CAPACITANCE: &[RangeEntry] = &[
        re("nF", "40nF"),
        re("nF", "400nF"),
        re("\u{00B5}F", "4\u{00B5}F"),
        re("\u{00B5}F", "40\u{00B5}F"),
        re("\u{00B5}F", "400\u{00B5}F"),
        re("\u{00B5}F", "4000\u{00B5}F"),
        re("mF", "40mF"),
    ];
    const MICROAMPS: &[RangeEntry] = &[re("", "400\u{00B5}A"), re("", "4000\u{00B5}A")];
    const MILLIAMPS: &[RangeEntry] = &[re("", "40mA"), re("", "400mA")];
    const AMPS: &[RangeEntry] = &[re("", "10A")];
    const MILLIVOLTS: &[RangeEntry] = &[re("", "400mV")];

    match function {
        // DCV, ACV, AC+DC V, ACV LPF — all share voltage ranges
        0x00 | 0x01 | 0x05 | 0x12 => VOLTAGE,
        // DC mV
        0x02 => MILLIVOLTS,
        // Frequency
        0x03 => FREQUENCY,
        // Impedance (Resistance)
        0x06 => RESISTANCE,
        // Capacitance
        0x09 => CAPACITANCE,
        // DC/AC µA
        0x0C | 0x0D => MICROAMPS,
        // DC/AC mA
        0x0E | 0x0F => MILLIAMPS,
        // DC/AC A
        0x10 | 0x11 => AMPS,
        // Single-range functions (duty, diode, continuity, temp, LPF)
        _ => &[],
    }
}

/// The dial, position by position, and the functions SHIFT/SETUP reaches on
/// each -- [MANUAL], no hardware has confirmed it.
///
/// From the "Drehschalter (4)" figure on printed page 10 of the VC880 manual
/// (the red symbols beside a position are its SHIFT/SETUP sub-functions, §3)
/// and the §8 measurement procedures. Membership only: the manual says which
/// symbol each position offers, never the press order, so the driver presses
/// and reads the mode back. See `docs/research/vc880/reverse-engineered-protocol.md`
/// §4.4, which also records where the manual's own text disagrees with its
/// figure.
///
/// Function 0x02 is the only code on two positions — the V position reports
/// it for its 400 mV auto range (§4.2). The two share nothing else, so a
/// bare 0x02 puts the dial nowhere (`cycle::DialState::observe`) and offers
/// no switch until a reading has named one of the positions.
const DIAL: &[DialPosition] = &[
    // mV⎓ / Hz % : DC mV, Frequency, Duty %
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x02, 0x03, 0x04],
        }],
    },
    // V⎓ (red AC+DC): DC V, AC+DC V, and DC mV on the 400 mV auto range
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x00, 0x01, 0x02],
        }],
    },
    // V~ : AC V alone — the figure gives this position no red symbol
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x05],
        }],
    },
    // Lo : ACV low-pass, its own dial position (§8j)
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x12],
        }],
    },
    // Ω (red diode, continuity)
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x06, 0x07, 0x08],
        }],
    },
    // ⊣⊢ : Capacitance
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x09],
        }],
    },
    // °C°F
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0A, 0x0B],
        }],
    },
    // µA≂ : DC µA, AC µA
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0C, 0x0D],
        }],
    },
    // mA≂ : DC mA, AC mA
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x0E, 0x0F],
        }],
    },
    // A≂ : DC A, AC A
    DialPosition {
        rings: &[Ring {
            button: CycleButton::Select,
            modes: &[0x10, 0x11],
        }],
    },
];

/// How long a press takes to show up in the stream. Not hardware-tuned:
/// nobody has timed a real VC-880.
///
/// No delay, because the meter streams and every read already waits for the
/// next frame; the budget is in reads instead. `press` drops what was queued
/// when the press landed, so four reads is for the frame mid-flight and the
/// meter's own reaction time.
const SETTLE: Settle = Settle {
    delay: Duration::ZERO,
    reads: 4,
};

/// The drain after a press: reads of up to this timeout, until one comes
/// back empty or the count runs out. A gap of 50 ms never occurs inside a
/// frame at 9600 baud, so an empty read means the queue is clear; the count
/// bounds it should the stream never pause.
const PRESS_DRAIN_TIMEOUT_MS: i32 = 50;
const PRESS_DRAIN_READS: usize = 8;

/// What the shared Voltcraft driver needs to speak VC-880.
///
/// Live data payload — everything between the length byte and the checksum:
///   payload[0]  = type byte (0x01, already filtered by the accept fn)
///   payload[1]  = function code (0x00-0x12)
///   payload[2]  = range byte (0x30-based ASCII)
///   payload[3..10]  = main display value (7 ASCII bytes)
///   payload[10..17] = sub display 1 (7 ASCII bytes)
///   payload[17..24] = sub display 2 (7 ASCII bytes)
///   payload[24..27] = bar graph / sub display 3 (3 bytes)
///   payload[27..34] = status flag bytes (7 bytes)
pub(crate) struct Vc880Model;

impl Vc8x0Model for Vc880Model {
    const LOG: &'static str = "vc880";
    const NAME: &'static str = "VC-880";
    // The VC650BT speaks the same protocol and shares this id; see
    // `Vc8x0Model::DETECTED_ID`.
    const DETECTED_ID: &'static str = "vc880";
    const PAYLOAD_LEN: usize = LIVE_DATA_PAYLOAD_LEN;
    const STATUS_AT: usize = 27;
    const DIAL: &'static [DialPosition] = DIAL;
    const SETTLE: Settle = SETTLE;
    const FUNCTION_TABLE: &'static [(u8, &'static str, &'static str)] = FUNCTION_TABLE;

    fn profile() -> DeviceProfile {
        DeviceProfile {
            family_name: "VC880",
            model_name: "Voltcraft VC-880",
            stability: Stability::Experimental,
            supported_commands: COMMANDS,
            max_aux_values: 0,
            verification_issue: Some(13),
        }
    }

    fn range_table(function: u8) -> &'static [RangeEntry] {
        range_table(function)
    }

    fn extra_flags(flags: &mut StatusFlags, status: &[u8]) {
        // Status byte 3 (payload[30]): bit3=LowBatt
        flags.low_battery = status[3] & 0x08 != 0;
    }

    /// The meter streams: a frame is simply there to be read.
    fn read_live_frame(rx_buf: &mut Vec<u8>, transport: &dyn Transport) -> Result<Vec<u8>> {
        read_live(rx_buf, transport, Self::LOG)
    }

    /// Write the frame and drop what the stream had already queued.
    ///
    /// The meter streams, so whatever was buffered when the command landed
    /// still describes the old state — and `read_frame` hands frames out
    /// oldest first. Drop it here rather than spend the settle budget on it.
    /// The 0xFF Result frame the meter answers with goes the same way; the
    /// live-frame accept filter (type byte 0x01) skips it anyway.
    fn write_button(rx_buf: &mut Vec<u8>, transport: &dyn Transport, cmd: u8) -> Result<()> {
        transport.write(&build_command(cmd))?;
        rx_buf.clear();
        let mut tmp = [0u8; 64];
        for _ in 0..PRESS_DRAIN_READS {
            let n = transport.read_timeout(&mut tmp, PRESS_DRAIN_TIMEOUT_MS)?;
            if n == 0 {
                break;
            }
            debug!("vc880: drained {n} bytes after the command");
        }
        Ok(())
    }
}

/// Protocol implementation for the Voltcraft VC-880 and VC650BT.
pub(crate) type Vc880Protocol = Vc8x0Protocol<Vc880Model>;

impl Vc880Protocol {
    /// Build the protocol for one specific model of the family.
    ///
    /// The VC650BT speaks the same protocol, but it has to report its own name
    /// — otherwise a user who selected the VC650BT sees "Voltcraft VC-880" in
    /// the header and can't tell whether the right device was picked.
    pub(crate) fn for_model(model_name: &'static str) -> Self {
        Self::with_profile(DeviceProfile {
            model_name,
            ..Vc880Model::profile()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_measurement as parse;
    use super::super::test_support::{self as shared, current_ids, ids};
    use super::*;
    use crate::error::Error;
    use crate::measurement::{MeasuredValue, Measurement};
    use crate::protocol::cycle::CycleMeter;
    use crate::protocol::test_support::snapshot;
    use crate::protocol::{Protocol, Setting, framing};
    use crate::transport::mock::MockTransport;

    /// Build a minimal 34-byte VC880 live data payload for testing.
    fn make_payload(function: u8, range: u8, main_display: &[u8; 7], status: [u8; 7]) -> Vec<u8> {
        shared::make_payload::<Vc880Model>(function, range, main_display, &status)
    }

    fn zero_status() -> [u8; 7] {
        [0u8; 7]
    }

    /// Parse a VC880 live data payload; the layout is on [`Vc880Model`].
    fn parse_measurement(payload: &[u8]) -> crate::error::Result<Measurement> {
        parse::<Vc880Model>(payload)
    }

    /// One payload parsed and rendered as a snapshot string.
    fn snap(function: u8, range: u8, display: &[u8; 7], status: [u8; 7]) -> String {
        let m = parse_measurement(&make_payload(function, range, display, status))
            .expect("the frame parses");
        snapshot(&m)
    }

    /// The single command frame a press or exit wrote.
    fn command_frame(transport: &MockTransport) -> Vec<u8> {
        let writes = transport.written.borrow();
        assert_eq!(writes.len(), 1, "the VC-880 sends the frame bare");
        writes[0].clone()
    }

    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x00, 0x31, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "40V");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-6));
    }

    #[test]
    fn parse_acv() {
        let payload = make_payload(0x05, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "4V");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x06, 0x32, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "40kΩ");
    }

    #[test]
    fn parse_capacitance_nf() {
        let payload = make_payload(0x09, 0x30, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Capacitance");
        assert_eq!(m.unit, "nF");
        assert_eq!(m.range_label, "40nF");
    }

    #[test]
    fn parse_frequency_khz() {
        let payload = make_payload(0x03, 0x32, b" 1.2345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Frequency");
        assert_eq!(m.unit, "kHz");
        assert_eq!(m.range_label, "4kHz");
    }

    #[test]
    fn parse_temperature() {
        let payload = make_payload(0x0A, 0x30, b"  23.45", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "°C");
        assert_eq!(m.unit, "°C");
    }

    #[test]
    fn parse_overload_flag() {
        // OL1 = status byte 2, bit 2
        shared::assert_overload_flag::<Vc880Model>(0x06);
    }

    #[test]
    fn parse_overload_display_string() {
        let payload = make_payload(0x06, 0x30, b"     OL", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn parse_negative_value() {
        let payload = make_payload(0x00, 0x31, b" -12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-6));
    }

    #[test]
    fn parse_hold_flag() {
        shared::assert_hold_flag::<Vc880Model>(0x00);
    }

    #[test]
    fn parse_rel_flag() {
        shared::assert_rel_flag::<Vc880Model>(0x00);
    }

    #[test]
    fn parse_max_min_flags() {
        shared::assert_max_min_flags::<Vc880Model>(0x00);
    }

    #[test]
    fn parse_avg_flag() {
        // Status byte 1 bit 1 = Avg (spec byte 31).
        shared::assert_avg_flag::<Vc880Model>(0x00);
    }

    #[test]
    fn parse_auto_range() {
        shared::assert_auto_range::<Vc880Model>(0x00);
    }

    #[test]
    fn parse_low_battery() {
        let mut status = zero_status();
        status[3] = 0x08; // LowBatt bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.low_battery);
    }

    #[test]
    fn parse_hv_warning() {
        let mut status = zero_status();
        status[3] = 0x02; // Warning bit
        let payload = make_payload(0x00, 0x30, b"  1.234", status);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hv_warning);
    }

    #[test]
    fn parse_unknown_function() {
        let payload = make_payload(0x20, 0x30, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert!(m.mode.starts_with("Unknown"));
    }

    #[test]
    fn parse_all_valid_functions() {
        shared::assert_all_functions_named::<Vc880Model>();
    }

    #[test]
    fn parse_payload_too_short() {
        shared::assert_short_payload_rejected::<Vc880Model>();
    }

    #[test]
    fn display_raw_preserved() {
        let payload = make_payload(0x00, 0x30, b" 12.345", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some(" 12.345"));
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x03, 0x32, b"  1.234", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x03);
    }

    #[test]
    fn range_unit_override_works() {
        // Capacitance with nF range
        let payload = make_payload(0x09, 0x30, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "nF"); // overridden from base "F"

        // Capacitance with µF range
        let payload = make_payload(0x09, 0x32, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "µF");

        // Capacitance with mF range
        let payload = make_payload(0x09, 0x36, b"  12.34", zero_status());
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.unit, "mF");
    }

    #[test]
    fn all_voltage_functions_share_ranges() {
        for func in [0x00, 0x01, 0x05, 0x12] {
            let payload = make_payload(func, 0x32, b"  123.4", zero_status());
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.range_label, "400V", "function {func:#04x}");
        }
    }

    #[test]
    fn send_command_builds_correct_frame() {
        // Checksum: 0xAB + 0xCD + 0x03 + 0x47 = 0x01C2
        shared::assert_command_frame(0x47); // autorange
    }

    /// Every status byte 0xFF: the OL1 bit forces Overload, the manual-range
    /// bit clears AUTO, and hold/rel/min/max/avg/HV/low-battery all light.
    #[test]
    fn snapshot_every_status_bit_set() {
        assert_eq!(
            snap(0x00, b'1', b"-1.2345", [0xFF; 7]),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("-1.2345")
flags=hold,rel,min,max,avg,low_battery,hv_warning
aux=0
raw_payload=34"#
        );
    }

    /// The same frame with every status byte clear: AUTO on (the manual bit
    /// is inverted), nothing else set, and the digits parsed as a value.
    #[test]
    fn snapshot_zero_status() {
        assert_eq!(
            snap(0x00, b'1', b"-1.2345", zero_status()),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Normal(-1.2345)
unit=V
range_label=40V
display_raw=Some("-1.2345")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Overload spelled out in the digits rather than flagged by OL1.
    #[test]
    fn snapshot_overload_display_string() {
        assert_eq!(
            snap(0x06, b'0', b"     OL", zero_status()),
            r#"mode=Ω
mode_raw=0x06
range_raw=0x30
value=Overload
unit=Ω
range_label=400Ω
display_raw=Some("     OL")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// The meter's "---" blank-reading form also reads as overload.
    #[test]
    fn snapshot_dashes_display() {
        assert_eq!(
            snap(0x06, b'0', b"    ---", zero_status()),
            r#"mode=Ω
mode_raw=0x06
range_raw=0x30
value=Overload
unit=Ω
range_label=400Ω
display_raw=Some("    ---")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// An all-spaces display: nothing to parse, so the reading falls back to
    /// Overload.
    #[test]
    fn snapshot_blank_display() {
        assert_eq!(
            snap(0x00, b'1', b"       ", zero_status()),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("       ")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Digits that are not a number: same Overload fallback as a blank one.
    #[test]
    fn snapshot_unparsable_display() {
        assert_eq!(
            snap(0x00, b'1', b"1.2.3.4", zero_status()),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x31
value=Overload
unit=V
range_label=40V
display_raw=Some("1.2.3.4")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// A function code outside FUNCTION_TABLE: mode falls back to
    /// `Unknown(..)` and the unit is empty.
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
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// Range byte b'9' indexes past the 4-entry voltage table: the range
    /// label goes empty and the unit falls back to the function's base unit.
    #[test]
    fn snapshot_range_index_past_the_table() {
        assert_eq!(
            snap(0x00, b'9', b"  1.234", zero_status()),
            r#"mode=DC V
mode_raw=0x00
range_raw=0x39
value=Normal(1.234)
unit=V
range_label=
display_raw=Some("  1.234")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// DC mV has a one-entry range table.
    #[test]
    fn snapshot_single_range_function() {
        assert_eq!(
            snap(0x02, b'0', b" 123.45", zero_status()),
            r#"mode=DC mV
mode_raw=0x02
range_raw=0x30
value=Normal(123.45)
unit=mV
range_label=400mV
display_raw=Some(" 123.45")
flags=auto_range
aux=0
raw_payload=34"#
        );
    }

    /// The whole range table as one string: every function code crossed with
    /// every range index, rendered `unit|range_label` (`-` where the lookup
    /// returns None). Pins both the labels and the unit-override fallback.
    #[test]
    fn range_table_snapshot() {
        assert_eq!(
            shared::render_range_table::<Vc880Model>(),
            r#"0x00: V|4V V|40V V|400V V|1000V - - - - -
0x01: V|4V V|40V V|400V V|1000V - - - - -
0x02: mV|400mV - - - - - - - -
0x03: Hz|40Hz Hz|400Hz kHz|4kHz kHz|40kHz kHz|400kHz MHz|4MHz MHz|40MHz MHz|400MHz -
0x04: - - - - - - - - -
0x05: V|4V V|40V V|400V V|1000V - - - - -
0x06: Ω|400Ω kΩ|4kΩ kΩ|40kΩ kΩ|400kΩ MΩ|4MΩ MΩ|40MΩ - - -
0x07: - - - - - - - - -
0x08: - - - - - - - - -
0x09: nF|40nF nF|400nF µF|4µF µF|40µF µF|400µF µF|4000µF mF|40mF - -
0x0a: - - - - - - - - -
0x0b: - - - - - - - - -
0x0c: µA|400µA µA|4000µA - - - - - - -
0x0d: µA|400µA µA|4000µA - - - - - - -
0x0e: mA|40mA mA|400mA - - - - - - -
0x0f: mA|40mA mA|400mA - - - - - - -
0x10: A|10A - - - - - - - -
0x11: A|10A - - - - - - - -
0x12: V|4V V|40V V|400V V|1000V - - - - -"#
        );
    }

    // ---- dial table and mode switching ----

    /// One live-data frame through `request_measurement`, the way the mode
    /// driver gets its picture of the dial.
    fn read_one(function: u8) -> (Vc880Protocol, Measurement) {
        shared::read_one::<Vc880Model>(function)
    }

    #[test]
    fn the_dial_table_is_well_formed() {
        // The V position reports DC mV for its 400 mV auto range, so 0x02 is
        // deliberately on two positions.
        shared::assert_dial_table_is_well_formed::<Vc880Model>(&[0x02]);
    }

    #[test]
    fn a_reading_records_the_dial_position() {
        // Position 4 is the Ω position.
        shared::assert_reading_records_dial_position::<Vc880Model>(0x06, 4);
    }

    /// 0x02 is on both the mV and the V position, which share nothing else,
    /// so a fresh process has no safe guess: nothing to list, and a switch
    /// is refused before any frame is written.
    #[test]
    fn a_dc_mv_reading_without_history_offers_nothing() {
        let (mut proto, m) = read_one(0x02);
        assert!(proto.choices(Setting::Mode, &m).is_empty());

        let transport = MockTransport::new(vec![]);
        let err = proto.select(&transport, Setting::Mode, 0x03).unwrap_err();
        assert!(
            matches!(&err, Error::UnsupportedCommand(m) if m.contains("more than one dial position")),
            "got {err:?}"
        );
        assert!(transport.written.borrow().is_empty());
    }

    /// Once a reading has named a position, 0x02 keeps it.
    #[test]
    fn a_dc_mv_reading_keeps_the_position_the_stream_established() {
        for (first, expected) in [
            (0x03, vec![0x02, 0x03, 0x04]),
            (0x00, vec![0x00, 0x01, 0x02]),
        ] {
            let (mut proto, _) = read_one(first);
            let transport = MockTransport::new(vec![framing::test_frame_be16(&make_payload(
                0x02,
                0x30,
                b"  1.234",
                zero_status(),
            ))]);
            let m = proto.request_measurement(&transport).expect("parses");
            assert_eq!(
                ids(&proto.choices(Setting::Mode, &m)),
                expected,
                "after {first:#04x}"
            );
        }
    }

    /// From the V position the same code is reachable, so a meter that
    /// auto-ranged down to 400 mV can still be switched back.
    #[test]
    fn the_v_position_lists_ac_dc_and_the_400_mv_code() {
        let (proto, m) = read_one(0x00);
        let choices = proto.choices(Setting::Mode, &m);
        assert_eq!(ids(&choices), vec![0x00, 0x01, 0x02]);
        assert_eq!(current_ids(&choices), vec![0x00]);
    }

    #[test]
    fn the_ohm_position_lists_diode_and_continuity() {
        shared::assert_ohm_position_lists_diode_and_continuity::<Vc880Model>(
            0x06,
            &[0x06, 0x07, 0x08],
        );
    }

    // --- Range selection --------------------------------------------------

    #[test]
    fn the_resistance_ladder_is_listed_with_auto_first() {
        shared::assert_resistance_ladder::<Vc880Model>(
            0x06,
            &["400Ω", "4kΩ", "40kΩ", "400kΩ", "4MΩ", "40MΩ"],
        );
    }

    #[test]
    fn an_auto_ranging_reading_marks_auto_current() {
        let (proto, m) = read_one(0x06);
        let choices = proto.choices(Setting::Range, &m);
        assert_eq!(current_ids(&choices), vec![0]);
    }

    /// DC mV has one range and diode none at all: neither is a choice.
    #[test]
    fn single_range_functions_offer_no_ranges() {
        shared::assert_no_range_choices::<Vc880Model>(&[0x02, 0x07, 0x0A]);
    }

    #[test]
    fn pressing_range_writes_one_range_frame() {
        shared::assert_range_button_writes_the_range_frame::<Vc880Model>(&command_frame);
    }

    #[test]
    fn auto_range_writes_the_auto_frame() {
        shared::assert_auto_range_writes_the_auto_frame::<Vc880Model>(&command_frame);
    }

    #[test]
    fn pressing_select_writes_one_shift_setup_frame() {
        shared::assert_select_writes_the_shift_setup_frame::<Vc880Model>(&command_frame);
    }

    /// A frame queued before the press names the old function; the press
    /// discards it so the settle reads start with what comes after.
    #[test]
    fn pressing_select_drops_the_frames_already_queued() {
        let stale = framing::test_frame_be16(&make_payload(0x06, 0x30, b"  1.234", zero_status()));
        let transport = MockTransport::new(vec![stale.clone(), stale]);
        let mut proto = Vc880Protocol::new();
        proto.rx_buf().extend_from_slice(&[0xAB, 0xCD, 0x01]);
        proto
            .press(&transport, CycleButton::Select)
            .expect("the frame is written");
        assert!(proto.rx_buf().is_empty());
        // The next frame after the press is the first one read back.
        transport.push_response(framing::test_frame_be16(&make_payload(
            0x07,
            0x30,
            b"  1.234",
            zero_status(),
        )));
        let m = proto.request_measurement(&transport).expect("parses");
        assert_eq!(m.mode_raw, 0x07, "both queued 0x06 frames were dropped");
    }

    #[test]
    fn pressing_hz_is_refused_without_writing() {
        shared::assert_button_refused::<Vc880Model>(CycleButton::Hz, "Hz/%");
    }

    // --- Flag-backed settings ---------------------------------------------

    #[test]
    fn minmax_lists_the_three_step_ring_and_off() {
        shared::assert_minmax_ring::<Vc880Model>(0x00);
    }

    #[test]
    fn the_flag_buttons_write_their_own_command_bytes() {
        shared::assert_flag_buttons_write_their_bytes::<Vc880Model>(&command_frame);
    }

    #[test]
    fn leaving_minmax_writes_the_exit_frame() {
        shared::assert_leaving_minmax_writes_the_exit_frame::<Vc880Model>(&command_frame);
    }

    #[test]
    fn pressing_peak_is_refused_without_writing() {
        shared::assert_button_refused::<Vc880Model>(CycleButton::Peak, "PEAK");
    }
}
