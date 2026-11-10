//! UT8802/UT8802N bench multimeter protocol.
//!
//! Streaming protocol: the host sends nothing after CP2110 init, and the
//! meter streams 8-byte measurement frames continuously.
//!
//! Frame format: `AC [position] [d4d5] [d2d3] [xd1] [dir+dp] [status+sign] [error]`
//!
//! Key difference from UT8803: single-byte 0xAC header, BCD-encoded display,
//! combined position codes (function + range), no checksum.
//!
//! See docs/research/uci-bench-family/reverse-engineered-protocol.md

pub(crate) mod devices;
mod frame;

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Probing, Protocol, Stability,
    check_len, display_number, unknown_mode,
};
use crate::transport::Transport;
use log::debug;
use std::borrow::Cow;
use std::cell::Cell;

/// UT8802 position code table: (code, mode_name, unit, range_label).
///
/// Combined function + range position codes from the programming manual
/// (page 10) and the vendor's position table (spec §3.3).
/// Both sources agree on all 35 entries.
///
/// `unit` carries the SI prefix for the position, because the meter's five
/// display digits are **range-relative**: with a decimal point at 0-4 the
/// smallest magnitude it can express is 0.0001, so 10 nF or 1.5 MHz simply
/// cannot be sent in farads or hertz. The prefix per position comes from
/// the vendor's prefix lookup, which returns the UnitMag index
/// (0=n, 1=µ, 2=m, 3=none, 4=k, 5=M) the vendor renders as a prefix; the
/// base units it combines with match the `unit` column here. The sibling
/// UT8803 parser resolves its units the same way (`display_unit`).
/// See docs/research/uci-bench-family/reverse-engineered-protocol.md §3.3.
///
/// `range_label` is left empty where the manual publishes no numeric span —
/// the capacitance and frequency positions are named only by their decade,
/// which is now in `unit`.
const POSITION_TABLE: &[(u8, &str, &str, &str)] = &[
    (0x01, "DC V", "mV", "200mV"),
    (0x03, "DC V", "V", "2V"),
    (0x04, "DC V", "V", "20V"),
    (0x05, "DC V", "V", "200V"),
    (0x06, "DC V", "V", "1000V"),
    (0x09, "AC V", "V", "2V"),
    (0x0A, "AC V", "V", "20V"),
    (0x0B, "AC V", "V", "200V"),
    (0x0C, "AC V", "V", "750V"),
    (0x0D, "DC µA", "µA", "200µA"),
    (0x0E, "DC mA", "mA", "2mA"),
    (0x10, "AC mA", "mA", "2mA"),
    (0x11, "DC mA", "mA", "20mA"),
    (0x12, "DC mA", "mA", "200mA"),
    (0x13, "AC mA", "mA", "20mA"),
    (0x14, "AC mA", "mA", "200mA"),
    (0x16, "DC A", "A", "2A"),
    (0x18, "AC A", "A", "20A"),
    (0x19, "Ω", "Ω", "200Ω"),
    (0x1A, "Ω", "kΩ", "2kΩ"),
    (0x1B, "Ω", "kΩ", "20kΩ"),
    (0x1C, "Ω", "kΩ", "200kΩ"),
    (0x1D, "Ω", "MΩ", "2MΩ"),
    (0x1F, "Ω", "MΩ", "200MΩ"),
    (0x22, "Duty %", "%", ""),
    (0x23, "Diode", "V", ""),
    (0x24, "Continuity", "Ω", ""),
    (0x25, "hFE", "β", ""),
    (0x27, "Capacitance", "nF", ""),
    (0x28, "Capacitance", "µF", ""),
    (0x29, "Capacitance", "mF", ""),
    (0x2A, "SCR", "V", ""),
    (0x2B, "Hz", "Hz", ""),
    (0x2C, "Hz", "kHz", ""),
    (0x2D, "Hz", "MHz", ""),
];

/// Look up a position code in the table. Returns (mode, unit, range_label).
fn lookup_position(code: u8) -> Option<(&'static str, &'static str, &'static str)> {
    POSITION_TABLE
        .iter()
        .find(|(c, _, _, _)| *c == code)
        .map(|(_, mode, unit, range)| (*mode, *unit, *range))
}

/// Every label the parser produces: the position table's modes and range
/// labels.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn all_labels(out: &mut crate::text::Labels) {
    use crate::text::TextKind;
    for &(code, ..) in POSITION_TABLE {
        if let Some((mode, _, range)) = lookup_position(code) {
            out.add(TextKind::Mode, mode, devices::UT8802.id);
            out.add(TextKind::Range, range, devices::UT8802.id);
        }
    }
}

const UT8802_COMMANDS: &[&str] = &[];

/// Protocol implementation for the UT8802/UT8802N bench multimeter.
pub(crate) struct Ut8802Protocol {
    rx_buf: Vec<u8>,
    profile: DeviceProfile,
    /// The unlisted position of the last frame refused, since a good one.
    last_unlisted: Cell<Option<u8>>,
}

impl Default for Ut8802Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl Ut8802Protocol {
    pub(crate) fn new() -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            last_unlisted: Cell::new(None),
            profile: DeviceProfile {
                id: "ut8802",
                stability: Stability::Experimental,
                supported_commands: UT8802_COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(12),
                meter_keys: MeterKeys::NONE,
            },
        }
    }
}

impl Protocol for Ut8802Protocol {
    fn delivery(&self) -> crate::protocol::Delivery {
        crate::protocol::Delivery::Streamed
    }

    fn discard_input(&mut self, transport: &dyn Transport) -> Result<()> {
        self.rx_buf.clear();
        crate::protocol::framing::discard_queued(transport)
    }

    fn init(&mut self, _transport: &dyn Transport) -> Result<()> {
        // No trigger byte: the vendor's CP2110 init path never writes to
        // the UART; the 0x5A trigger we previously sent belongs to the
        // QinHeng/CH9325 init path for other meters. See the UT8803 init for the
        // same correction.
        debug!("ut8802: init (no trigger byte; meter streams unprompted)");
        Ok(())
    }

    /// A meter sending a position outside the table (spec §3.3) would look
    /// dead: the extractor refuses each frame, and with no checksum it can't
    /// tell a real one from a false `0xAC` while syncing, so it reports nothing.
    /// A read that got no frame can tell, having seen the same position twice
    /// in a row, here or in the read before: no source gives the frame rate.
    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        let last = &self.last_unlisted;
        let repeated = Cell::new(None);
        let read = framing::read_frame(
            &mut self.rx_buf,
            transport,
            |buf: &[u8]| {
                let position = frame::unlisted_position(buf);
                if position.is_some() {
                    if last.get() == position {
                        repeated.set(position);
                    }
                    last.set(position);
                }
                frame::extract_frame_ut8802(buf)
            },
            |_| true,
            FrameErrorRecovery::SkipAndRetry {
                header: &frame::UT8802_HEADER,
            },
            "ut8802",
        );
        match read {
            Ok(payload) => {
                self.last_unlisted.set(None);
                parse_measurement(&payload)
            }
            Err(e) => {
                if let (Error::Timeout, Some(position)) = (&e, repeated.get()) {
                    report_position(position);
                }
                Err(e)
            }
        }
    }

    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        parse_measurement(payload)
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn capture_steps(&self) -> Vec<crate::protocol::CaptureStep> {
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{CaptureStep, Expect, Need};

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
            CaptureStep::basic("dcua", "Set meter to DC µA").expect(Expect::mode("DC µA")),
            CaptureStep::basic("dcma", "Set meter to DC mA").expect(Expect::mode("DC mA")),
            CaptureStep::basic("dca", "Set meter to DC A").expect(Expect::mode("DC A")),
            CaptureStep::basic("acma", "Set meter to AC mA").expect(Expect::mode("AC mA")),
            CaptureStep::basic("aca", "Set meter to AC A").expect(Expect::mode("AC A")),
            ohm,
            ohm_body,
            ohm_short,
            CaptureStep::basic("cont", "Set meter to Continuity")
                .expect(Expect::mode("Continuity")),
            CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
            CaptureStep::basic("cap", "Set meter to Capacitance")
                .expect(Expect::mode("Capacitance")),
            CaptureStep::basic("hz", "Set meter to Frequency (Hz)").expect(Expect::mode("Hz")),
            CaptureStep::basic("duty", "Set meter to Duty Cycle (%)")
                .expect(Expect::mode("Duty %")),
            CaptureStep::basic("hfe", "Set meter to hFE (transistor test)")
                .needs(&[Need::Transistor])
                .expect(Expect::mode("hFE")),
            CaptureStep::basic("scr", "Set meter to SCR (thyristor test)")
                .needs(&[Need::Scr])
                .expect(Expect::mode("SCR")),
            // Which byte carries the key flags rests on the vendor DLL alone
            // (spec §3.5), so these assert nothing: the raw frames are the
            // answer. No UT8802 manual names the keys.
            CaptureStep::basic(
                "hold",
                "DC V: press HOLD, then Enter; skip if it has none. Press HOLD again afterwards.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
            CaptureStep::basic(
                "rel",
                "DC V: press REL, then Enter; skip if it has none. Press REL again afterwards.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
            CaptureStep::basic(
                "max_min",
                "DC V: press MAX/MIN, then Enter; skip if it has none. Exit MAX/MIN afterwards.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
            CaptureStep::basic(
                "manual_range",
                "DC V: press RANGE, then Enter; skip if it has none. Return to auto range afterwards.",
            )
            .wait_for_enter()
            .expect(Expect::mode("DC V")),
        ]
    }
}

/// Whether a position code measures a DC quantity.
///
/// The vendor derives the status-word AC/DC field from the position code
/// alone via a lookup (spec §3.4:
/// returns 2=DC for the codes below, 1=AC for 0x09-0x0C/0x10/0x13/0x14/0x18,
/// 0 otherwise). Frame byte 5 bits 4-5 — previously misread as AC/DC
/// coupling — are diode/SCR probe-direction indicators.
fn position_is_dc(position: u8) -> bool {
    matches!(
        position,
        0x01 | 0x03..=0x06 | 0x0D | 0x0E | 0x11 | 0x12 | 0x16
    )
}

/// Convert a BCD nibble to its display character.
///
/// - 0x0-0x9 → '0'-'9'
/// - 0x0A → '0', kept defensively: the vendor gives it no meaning, adding
///   0x30 to every nibble (spec §3.2)
/// - 0x0C → '?': read as an `L` once, but OL is frame byte 6 bit 6 alone
///   (spec §3.2), so the text is unrecognised and reported
///
/// Other values should be rejected by the frame extractor's validation,
/// but we handle them defensively with '?'.
fn bcd_to_char(nibble: u8) -> char {
    match nibble {
        0x00..=0x09 => (b'0' + nibble) as char,
        0x0A => '0',
        _ => '?',
    }
}

/// Report a position code the table (spec §3.3) doesn't list.
fn report_position(position: u8) {
    report_unknown("ut8802", "position code", format_args!("{position:#04x}"));
}

/// Parse a UT8802 measurement payload (7 bytes = frame bytes 1..8).
///
/// Layout (spec §3.1; payload index =
/// frame byte − 1):
/// - byte 0: position code (0x01-0x2D, combined function + range)
/// - bytes 1-3: 5 display nibbles, most significant digit in byte 3's low
///   nibble: d1=b3 lo, d2=b2 hi, d3=b2 lo, d4=b1 hi, d5(LSD)=b1 lo
///   (spec §3.2; byte 3's
///   high nibble is never read, and is reported if set)
/// - byte 4: decimal point position (low nibble, 0-4) + diode/SCR probe
///   direction (bits 4-5, consumed only for positions 0x23/0x2A,
///   spec §3.4 — NOT AC/DC coupling);
///   bits 6-7 are never read, and are reported if set
/// - byte 5: status flags, sign in bit 7 (spec §3.5)
/// - byte 6: error flag in bit 7, bits 0-6 never read and reported if set
///   (spec §3.6)
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len("ut8802", payload, 7)?;

    let position = payload[0];
    let dp_pos = payload[4] & 0x0F;
    // The framing extractor already rejects dp_pos > 4, but the index math
    // in `display_text` (`chars.len() - dp_pos - 1`) would panic on larger
    // values, so guard here too in case this is ever called on unvalidated
    // payloads.
    if dp_pos > 4 {
        return Err(Error::invalid_response(
            format!("ut8802 invalid decimal point position {dp_pos}"),
            payload,
        ));
    }
    let status = payload[5];

    // Frame byte 4 carries only the MSD in its low nibble, frame byte 5 only
    // the decimal point and bits 4-5, and frame byte 7 only the error flag
    // in bit 7; the vendor never reads the rest
    // (docs/research/uci-bench-family/reverse-engineered-protocol.md §3.1,
    // §3.2, §3.4, §3.6), so a bit set there is undocumented.
    if payload[3] & 0xF0 != 0 {
        report_unknown(
            "ut8802",
            "status bits",
            format_args!("frame byte 4 = {:#04x}", payload[3]),
        );
    }
    if payload[4] & 0xC0 != 0 {
        report_unknown(
            "ut8802",
            "status bits",
            format_args!("frame byte 5 = {:#04x}", payload[4]),
        );
    }
    if payload[6] & 0x7F != 0 {
        report_unknown(
            "ut8802",
            "status bits",
            format_args!("frame byte 7 = {:#04x}", payload[6]),
        );
    }

    // Look up position code
    let (mode, unit, range_label): (Cow<'static, str>, &'static str, &'static str) =
        if let Some((m, u, r)) = lookup_position(position) {
            (Cow::Borrowed(m), u, r)
        } else {
            report_position(position);
            (unknown_mode(position), "", "")
        };

    let (display_str, overload) = display_text(payload, dp_pos, status);

    // Parse numeric value
    let value = if overload {
        MeasuredValue::Overload
    } else {
        let trimmed: String = display_str.chars().filter(|c| !c.is_whitespace()).collect();
        display_number("ut8802", &trimmed)
    };

    // Frame byte 6's bits, as the vendor's status word takes them
    // (docs/research/uci-bench-family/reverse-engineered-protocol.md §3.5,
    // §3.7):
    //
    //   bit 0: MIN  (→ status-word D29)
    //   bit 1: MAX  (→ D28)
    //   bit 2: AUTO **inverted** (→ D6)
    //   bit 3: REL  (→ D30)
    //   bit 4: HOLD (→ D31)
    //   bit 5: Over (→ D18; not surfaced by StatusFlags)
    //   bit 6: OL   (→ D7)
    //   bit 7: sign (→ D19, folded into the display string by
    //         `display_text`)
    //
    // Frame byte 7 bit 7 is the error flag (§3.6), which no StatusFlags
    // field carries.
    let auto_range = status & 0x04 == 0;
    let hold = status & 0x10 != 0;
    let rel = status & 0x08 != 0;
    let max_flag = status & 0x02 != 0;
    let min_flag = status & 0x01 != 0;

    let flags = StatusFlags {
        hold,
        rel,
        min: min_flag,
        max: max_flag,
        auto_range,
        low_battery: false,
        hv_warning: false,
        dc: position_is_dc(position),
        peak_max: false,
        peak_min: false,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: position as u16,
        range_raw: position, // UT8802 combines mode+range in position code
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        // On overload the vendor shows "  0L " whatever the digits say
        // (spec §3.2), so they are not the meter's screen: with none,
        // every renderer falls back to OL.
        display_raw: (!overload).then_some(display_str),
        flags,
        ..Measurement::from_payload(payload, value)
    })
}

/// The display string from the five digit nibbles — leading zeros blanked,
/// then the decimal point and the sign, both left off on overload — and
/// whether the reading is over range (`status` bit 6).
/// `status` is frame byte 6. `dp_pos` must be 0-4.
fn display_text(payload: &[u8], dp_pos: u8, status: u8) -> (String, bool) {
    debug_assert!(dp_pos <= 4, "parse_measurement rejects dp_pos {dp_pos}");
    // Decode the 5 display nibbles, MSD first. The vendor parser fills its
    // display buffer as [b3 lo, b2 hi, b2 lo, b1 hi, b1 lo] (payload
    // indexing; spec §3.2) and runs atof directly
    // on the resulting string, so byte 3's low nibble is the most
    // significant digit.
    let nibbles = [
        payload[3] & 0x0F,
        payload[2] >> 4,
        payload[2] & 0x0F,
        payload[1] >> 4,
        payload[1] & 0x0F,
    ];

    // The only overload mechanism is frame byte 6 bit 6: the vendor skips
    // the atof and substitutes a sentinel plus the literal display "  0L "
    // (spec §3.2). The digits are then not what the meter shows, so
    // `parse_measurement` drops them; the point and sign are left off too.
    let overload = status & 0x40 != 0;
    let mut chars: Vec<char> = nibbles.iter().map(|&n| bcd_to_char(n)).collect();

    // Blank leading zeros up to the units digit (the one left of the
    // decimal point, or the last when dp_pos=0); the vendor blanks only the
    // first character (spec §3.2). Without this, an all-zero reading would become "     "
    // and fail to parse as 0.0.
    let keep_pos = if dp_pos > 0 {
        chars.len() - dp_pos as usize - 1
    } else {
        chars.len() - 1
    };
    for (i, ch) in chars.iter_mut().enumerate() {
        if i >= keep_pos {
            break;
        }
        if *ch == '0' {
            *ch = ' ';
        } else {
            break;
        }
    }

    // Insert decimal point
    if dp_pos > 0 && !overload {
        let insert_pos = chars.len() - dp_pos as usize;
        chars.insert(insert_pos, '.');
    }

    // The digits never carry a sign: the vendor hands its string builder the
    // digit string and frame byte 6 bit 7 separately (spec §3.2).
    // Every display and export path prefers `display_raw` for a normal
    // value, so the sign has to be in the string, not only on the float.
    if status & 0x80 != 0 && !overload {
        chars.insert(0, '-');
    }

    (chars.iter().collect(), overload)
}

/// Detection for the UT8802.
///
/// The meter streams unprompted, so it needs no trigger. Its 8-byte format
/// carries no checksum and its validation accepts roughly 1% of random input,
/// so one frame is not evidence: two consecutive ones are required, and its
/// claim ranks below any checksummed one
/// (`docs/research/uci-bench-family/reverse-engineered-protocol.md` §3).
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Ut8802,
    label: "ut8802 stream",
    trigger: None,
    send_after: &[],
    checksummed: false,
    claims_info_links: false,
    recognise,
};

fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    const FRAME_LEN: usize = frame::UT8802_FRAME_LEN;
    // The extractor searches forward for the next `0xAC`, so a frame it
    // returns is not necessarily the one at the offset asked for: a match
    // consuming exactly one frame is what pins it there, and with it the
    // "consecutive" this rule rests on.
    let frame_at = |at: usize| {
        matches!(
            frame::extract_frame_ut8802(&buf[at..]),
            Ok(Some((_, FRAME_LEN)))
        )
    };
    for (start, _) in buf
        .iter()
        .enumerate()
        .filter(|&(_, &b)| b == frame::UT8802_HEADER[0])
    {
        let next = start + FRAME_LEN;
        if next + FRAME_LEN > buf.len() {
            break;
        }
        if frame_at(start) && frame_at(next) {
            return Some(Evidence::Model {
                device: &devices::UT8802,
                reported_name: None,
            });
        }
    }
    None
}

/// Every unit a reading of this family can carry, for
/// [`crate::protocol::all_units`].
#[cfg(test)]
pub(crate) fn all_units(out: &mut crate::protocol::Units) {
    out.insert("");
    out.extend(POSITION_TABLE.iter().map(|&(_, _, unit, _)| unit));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::test_support::snapshot;
    use crate::transport::mock::MockTransport;

    /// Build a 7-byte UT8802 payload from components.
    fn make_payload(
        position: u8,
        digits: [u8; 5],
        dp_pos: u8,
        direction: u8,
        status: u8,
        error: u8,
    ) -> Vec<u8> {
        frame::test_frame_ut8802(position, digits, dp_pos, direction, status, error)[1..].to_vec()
    }

    /// The one payload whose every parsed field is pinned: an ordinary DC V
    /// reading, 200V range, display "1234.5" (dp_pos=1).
    #[test]
    fn parse_dcv() {
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x05
range_raw=0x05
value=Normal(1234.5)
unit=V
range_label=200V
display_raw=Some("1234.5")
flags=auto_range,dc
aux=0
raw_payload=7"#
        );
    }

    #[test]
    fn parse_acv() {
        // AC V 20V, display "12.34" (dp_pos=2)
        let payload = make_payload(0x0A, [0, 1, 2, 3, 4], 2, 0x01, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.unit, "V");
        assert_eq!(m.range_label, "20V");
    }

    #[test]
    fn parse_resistance() {
        // Resistance 2kΩ, display "1.234" (dp_pos=3). The digits are
        // range-relative, so this is 1.234 kΩ — the unit carries the prefix.
        let payload = make_payload(0x1A, [0, 1, 2, 3, 4], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.range_label, "2kΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.234).abs() < 1e-6));
    }

    /// The five display digits are range-relative: 10 nF is sent as "10.00"
    /// on the nF position, and reporting that in farads would be off by 10^9.
    /// Prefixes come from the vendor's prefix lookup (spec §3.3).
    #[test]
    fn capacitance_units_carry_the_range_decade() {
        for (position, expected) in [(0x27, "nF"), (0x28, "µF"), (0x29, "mF")] {
            let payload = make_payload(position, [0, 1, 0, 0, 0], 2, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.mode, "Capacitance");
            assert_eq!(m.unit, expected, "position {position:#04x}");
            assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 10.0).abs() < 1e-6));
        }
    }

    #[test]
    fn frequency_units_carry_the_range_decade() {
        for (position, expected) in [(0x2B, "Hz"), (0x2C, "kHz"), (0x2D, "MHz")] {
            let payload = make_payload(position, [0, 1, 5, 0, 0], 3, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.mode, "Hz");
            assert_eq!(m.unit, expected, "position {position:#04x}");
            assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1.5).abs() < 1e-6));
        }
    }

    /// The 200 mV position reports millivolts, not volts — the vendor's
    /// prefix lookup returns `m` for position 0x01.
    #[test]
    fn millivolt_range_reports_millivolts() {
        let payload = make_payload(0x01, [0, 1, 2, 3, 4], 2, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "mV");
        assert_eq!(m.range_label, "200mV");
    }

    /// Positions whose base unit needs no prefix must stay unprefixed.
    #[test]
    fn unprefixed_positions_keep_the_base_unit() {
        for (position, expected) in [
            (0x04, "V"),  // DC V 20V
            (0x16, "A"),  // DC A 2A
            (0x19, "Ω"),  // 200Ω
            (0x22, "%"),  // duty
            (0x23, "V"),  // diode
            (0x24, "Ω"),  // continuity
            (0x25, "β"),  // hFE
            (0x2B, "Hz"), // Hz
        ] {
            let payload = make_payload(position, [0, 0, 1, 0, 0], 1, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert_eq!(m.unit, expected, "position {position:#04x}");
        }
    }

    #[test]
    fn parse_overload() {
        // Frame byte 6 bit 6 → overload
        let payload = make_payload(0x01, [0, 0, 0, 0, 0], 0, 0x00, 0x40, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Overload
unit=mV
range_label=200mV
display_raw=None
flags=auto_range,dc
aux=0
raw_payload=7"#
        );
    }

    /// A 0x0C digit nibble is no overload: bit 6 is the only one (spec
    /// §3.2), so the digits are text nothing recognises, reported, `?`.
    #[test]
    fn a_0c_nibble_is_not_an_overload() {
        let payload = make_payload(0x01, [0, 0, 0x0C, 0, 0], 0, 0x00, 0x00, 0x00);
        let (m, reports) =
            crate::protocol::capture_reports(|| parse_measurement(&payload).unwrap());
        assert!(
            matches!(m.value, MeasuredValue::NoReading("?")),
            "{:?}",
            m.value
        );
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    #[test]
    fn out_of_range_dp_pos_errors_instead_of_panicking() {
        // dp_pos 5-15 would underflow the decimal-insertion index math;
        // normally rejected by the framing extractor, but parse_measurement
        // must also refuse it rather than panic.
        for dp_pos in 5..=15u8 {
            let payload = make_payload(0x05, [1, 2, 3, 4, 5], dp_pos, 0x02, 0x00, 0x00);
            assert!(parse_measurement(&payload).is_err(), "dp_pos={dp_pos}");
        }
    }

    /// Frame byte 6 bit 7 negates the value, and the sign reaches the
    /// display string too: display and CSV render `display_raw`, so a
    /// negative reading used to show and export unsigned.
    #[test]
    fn parse_negative() {
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x80, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-1234.5)).abs() < 1e-6));
        assert_eq!(m.display_raw.as_deref(), Some("-1234.5"));
    }

    /// The sign goes in front of the blanked leading zero, the way the
    /// UT61E+ spells its own display, and the number still parses.
    #[test]
    fn parse_negative_with_leading_blank() {
        let payload = make_payload(0x05, [0, 1, 2, 3, 4], 2, 0x02, 0x80, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == -12.34));
        assert_eq!(m.display_raw.as_deref(), Some("- 12.34"));
    }

    /// Overload wins: the sign bit set beside OL leaves no digits behind.
    #[test]
    fn parse_negative_overload_carries_no_digits() {
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0xC0, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw, None);
    }

    #[test]
    fn digit_order_matches_vendor_wire_layout() {
        // Raw bytes, not via make_payload: digits 1,2,3,4,5 (MSD→LSD) are
        // d1=byte3 lo, d2=byte2 hi, d3=byte2 lo, d4=byte1 hi, d5=byte1 lo
        // (spec §3.2). dp_pos=1 → "1234.5".
        let payload = [0x05, 0x45, 0x23, 0x01, 0x01, 0x00, 0x00];
        let m = parse_measurement(&payload).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 1234.5).abs() < 1e-6));
    }

    /// Vendor overload = frame byte 6 bit 6, with ordinary digits on the wire
    /// (spec §3.2).
    ///
    /// The digits are dropped: capture's confirmation line and the JSON
    /// export render `display_raw` as is, so keeping them showed an
    /// over-range input as a plausible reading ("12345 V") while the meter
    /// showed OL.
    #[test]
    fn overload_from_status_bit() {
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x00, 0x40, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x05
range_raw=0x05
value=Overload
unit=V
range_label=200V
display_raw=None
flags=auto_range,dc
aux=0
raw_payload=7"#
        );
        // And the export/display path must call it what it is.
        assert_eq!(m.value_export_str(), "OL");
    }

    #[test]
    fn dc_flag_from_position_code() {
        // AC position (0x09 = AC V, spec §3.4) must not set dc even
        // though byte 4 bits 4-5 (diode/SCR direction) read as "2".
        let payload = make_payload(0x09, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.dc);

        // DC position (0x05 = DC V) sets dc regardless of byte 4 bits.
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x01, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.dc);
    }

    #[test]
    fn parse_auto_range() {
        // Bit 2 clear = auto ON (inverted logic)
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.auto_range);

        // Bit 2 set = auto OFF
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x04, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.auto_range);
    }

    #[test]
    fn parse_hold_flag() {
        // HOLD = frame byte 6 bit 4 (status-word D31)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x10, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_rel_flag() {
        // REL = frame byte 6 bit 3 (D30)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x08, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.rel);
    }

    #[test]
    fn parse_max_min_flags() {
        // MAX = frame byte 6 bit 1 (D28), MIN = bit 0 (D29)
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x02, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.max);
        assert!(!m.flags.min);

        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x01, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(!m.flags.max);
        assert!(m.flags.min);
    }

    /// Byte 4 bits 4-5 do **not** encode AC/DC coupling.
    ///
    /// They were read that way once ("0=OFF, 1=AC, 2=DC, 3=AC+DC"), and this
    /// test was written to that reading — but it only ever passed because
    /// each case's position code happened to agree with the coupling bits it
    /// asserted on. `position_is_dc` documents the corrected reading: the
    /// vendor derives AC/DC from the position code alone (spec §3.4),
    /// and these bits are diode/SCR probe-direction indicators.
    ///
    /// So sweep all four bit values against a fixed position and assert the
    /// flag doesn't move. That fails if anyone reintroduces the old reading.
    #[test]
    fn acdc_bits_do_not_affect_the_dc_flag() {
        for acdc_bits in 0..=3u8 {
            // 0x05 = DC V: dc must stay set whatever the bits say.
            let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, acdc_bits, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(m.flags.dc, "DC position with acdc_bits={acdc_bits}");

            // 0x0A = AC V: dc must stay clear whatever the bits say.
            let payload = make_payload(0x0A, [0, 1, 2, 3, 4], 2, acdc_bits, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(!m.flags.dc, "AC position with acdc_bits={acdc_bits}");
        }
    }

    #[test]
    fn parse_unknown_position() {
        // Position 0x02 is a gap — frame extractor would reject this,
        // but the parser should handle it gracefully
        let payload = make_payload(0x02, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x02)
mode_raw=0x02
range_raw=0x02
value=Normal(12345.0)
unit=
range_label=
display_raw=Some("12345")
flags=auto_range
aux=0
raw_payload=7"#
        );
    }

    #[test]
    fn parse_all_valid_positions() {
        for &(code, _, _, _) in POSITION_TABLE {
            let payload = make_payload(code, [0, 1, 0, 0, 0], 0, 0x00, 0x00, 0x00);
            let m = parse_measurement(&payload).unwrap();
            assert!(
                !crate::protocol::is_unknown_mode(&m.mode),
                "position {code:#04x} should be known"
            );
        }
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x01, 0x12, 0x34];
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn decimal_point_positions() {
        // dp_pos=0 → "12345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12345"));

        // dp_pos=1 → "1234.5"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 1, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("1234.5"));

        // dp_pos=2 → "123.45"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 2, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("123.45"));

        // dp_pos=3 → "12.345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.345"));

        // dp_pos=4 → "1.2345"
        let payload = make_payload(0x01, [1, 2, 3, 4, 5], 4, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("1.2345"));
    }

    #[test]
    fn leading_zeros_replaced_with_spaces() {
        // digits [0, 0, 1, 2, 3] → "  123"
        let payload = make_payload(0x01, [0, 0, 1, 2, 3], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("  123"));
    }

    #[test]
    fn zero_reading_integer() {
        // All-zero digits, dp_pos=0 → "    0" → value 0.0
        let payload = make_payload(0x01, [0, 0, 0, 0, 0], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("    0"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn zero_reading_with_decimal() {
        // All-zero digits, dp_pos=4 (200mV range) → "0.0000" → value 0.0
        let payload = make_payload(0x01, [0, 0, 0, 0, 0], 4, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("0.0000"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn zero_reading_dp3() {
        // All-zero digits, dp_pos=3 → " 0.000" → value 0.0
        let payload = make_payload(0x1A, [0, 0, 0, 0, 0], 3, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some(" 0.000"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn nibble_0a_treated_as_zero() {
        // 0x0A → '0', and it's a leading zero so replaced with space
        let payload = make_payload(0x01, [0x0A, 0, 1, 2, 3], 0, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("  123"));
    }

    /// Parse `payload` and `clean`, check both read the same, and return what
    /// parsing `payload` reported.
    fn reports_leaving_reading_unchanged(payload: &[u8], clean: &[u8]) -> Vec<String> {
        let (m, reports) = crate::protocol::capture_reports(|| parse_measurement(payload).unwrap());
        assert_eq!(snapshot(&m), snapshot(&parse_measurement(clean).unwrap()));
        reports
    }

    /// Read through the driver, one response per read, and what it reported.
    fn read_reporting(responses: Vec<Vec<u8>>) -> (Result<Measurement>, Vec<String>) {
        let mock = MockTransport::new(responses);
        let mut protocol = Ut8802Protocol::new();
        capture_reports(|| protocol.request_measurement(&mock))
    }

    /// Frames at position 0x07, a gap in the table (spec §3.3).
    fn unlisted_stream(frames: usize) -> Vec<u8> {
        frame::test_frame_ut8802(0x07, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00).repeat(frames)
    }

    /// A meter on a position outside the table says so once a read has failed,
    /// however the cable hands the bytes up: a CP2110 gives about one per read.
    #[test]
    fn an_unlisted_position_is_reported_when_the_read_fails() {
        let stream = unlisted_stream(5);
        for responses in [
            vec![stream.clone()],
            stream.chunks(1).map(<[u8]>::to_vec).collect(),
        ] {
            let (m, reports) = read_reporting(responses);
            assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
            assert_eq!(reports, ["ut8802: unrecognised position code: 0x07"]);
        }
    }

    /// A pulled cable is what the read ends on, whatever came before it.
    #[test]
    fn a_lost_link_is_not_an_unlisted_position() {
        let mock = MockTransport::new(vec![unlisted_stream(5)]).fail_after(1, Error::cable_pulled);
        let mut protocol = Ut8802Protocol::new();
        let (m, reports) = capture_reports(|| protocol.request_measurement(&mock));
        assert!(m.is_err_and(|e| !matches!(e, Error::Timeout)));
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A slow meter may send one frame per read.
    #[test]
    fn an_unlisted_position_is_reported_across_reads() {
        let mut protocol = Ut8802Protocol::new();
        let mock = MockTransport::new(vec![unlisted_stream(1)]);
        let (m, reports) = capture_reports(|| protocol.request_measurement(&mock));
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert!(reports.is_empty(), "{reports:?}");
        let mock = MockTransport::new(vec![unlisted_stream(1)]);
        let (m, reports) = capture_reports(|| protocol.request_measurement(&mock));
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert_eq!(reports, ["ut8802: unrecognised position code: 0x07"]);
    }

    /// A position outside the table is where a display that isn't a number
    /// is likeliest: blanks here, in digits the extractor would refuse too.
    #[test]
    fn an_unlisted_position_with_an_unreadable_display_is_reported() {
        let stream = frame::test_frame_ut8802(0x07, [0x0F, 0x0F, 0x0F, 0x0F, 0x0F], 1, 0, 0, 0);
        let (m, reports) = read_reporting(vec![stream.repeat(5)]);
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert_eq!(reports, ["ut8802: unrecognised position code: 0x07"]);
    }

    /// A 0xAC in either of a frame's last two bytes starts a false frame
    /// between its real ones, and that must not break the run.
    #[test]
    fn a_0xac_in_the_last_bytes_does_not_hide_an_unlisted_position() {
        for (status, error) in [(0xAC, 0x00), (0x00, 0xAC)] {
            let frame = frame::test_frame_ut8802(0x07, [1, 2, 3, 4, 5], 1, 0x02, status, error);
            let (_, reports) = read_reporting(vec![frame.repeat(5)]);
            assert_eq!(
                reports,
                ["ut8802: unrecognised position code: 0x07"],
                "{status:#04x}, {error:#04x}"
            );
        }
    }

    /// A run past what one read examines still ends in the report.
    #[test]
    fn a_long_run_of_unlisted_positions_is_reported_once() {
        let stream = unlisted_stream(70);
        let (m, reports) = read_reporting(stream.chunks(64).map(<[u8]>::to_vec).collect());
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert_eq!(reports, ["ut8802: unrecognised position code: 0x07"]);
    }

    /// Only a read that fails says anything.
    #[test]
    fn a_good_frame_after_unlisted_positions_is_not_reported() {
        let mut stream = unlisted_stream(3);
        stream.extend(frame::test_frame_ut8802(
            0x05,
            [1, 2, 3, 4, 5],
            1,
            0x02,
            0x00,
            0x00,
        ));
        let (m, reports) = read_reporting(vec![stream]);
        assert!(m.is_ok(), "{m:?}");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A good frame starts the count again.
    #[test]
    fn a_good_frame_resets_the_repeat() {
        let good = frame::test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let mock = MockTransport::new(vec![
            [unlisted_stream(1), good].concat(),
            unlisted_stream(1),
        ]);
        let mut protocol = Ut8802Protocol::new();
        let (m, _) = capture_reports(|| protocol.request_measurement(&mock));
        assert!(m.is_ok(), "{m:?}");
        let (m, reports) = capture_reports(|| protocol.request_measurement(&mock));
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// Positions that keep changing are as likely noise.
    #[test]
    fn changing_unlisted_positions_are_not_reported() {
        let frame = |position| frame::test_frame_ut8802(position, [1, 2, 3, 4, 5], 1, 0, 0, 0);
        let stream = [frame(0x07), frame(0x08), frame(0x07), frame(0x08)].concat();
        let (m, reports) = read_reporting(vec![stream]);
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// One bad frame is as likely a false `0xAC`, with no checksum to say.
    #[test]
    fn one_unlisted_position_is_not_reported() {
        let (m, reports) = read_reporting(vec![unlisted_stream(1)]);
        assert!(matches!(m, Err(Error::Timeout)), "{m:?}");
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// A status byte of 0xAC (negative, over range, REL, manual range) reads
    /// as a header when a stream is joined just before it. Any frame that
    /// reads ends the question.
    #[test]
    fn a_stream_joined_mid_frame_is_not_reported() {
        let good = frame::test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0xAC, 0x00);
        for joined_at in 1..good.len() {
            let mut stream = good[joined_at..].to_vec();
            stream.extend_from_slice(&good.repeat(3));
            for responses in [
                vec![stream.clone()],
                stream.chunks(1).map(<[u8]>::to_vec).collect(),
            ] {
                let (m, reports) = read_reporting(responses);
                assert!(m.is_ok(), "joined at {joined_at}: {m:?}");
                assert!(reports.is_empty(), "joined at {joined_at}: {reports:?}");
            }
        }
    }

    /// Replay and golden fixtures reach the parser without the extractor.
    #[test]
    fn an_unlisted_position_in_a_payload_is_reported() {
        let payload = make_payload(0x07, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let (m, reports) = crate::protocol::capture_reports(|| parse_measurement(&payload));
        assert_eq!(m.unwrap().mode, "Unknown(0x07)");
        assert_eq!(reports, ["ut8802: unrecognised position code: 0x07"]);
    }

    /// Detection scans every meter's bytes, and another family's are not this
    /// meter's unknown position.
    #[test]
    fn detection_reports_nothing() {
        let stream = unlisted_stream(5);
        let (found, reports) =
            crate::protocol::capture_reports(|| recognise(&stream, &Probing::default()));
        assert_eq!(found, None);
        assert!(reports.is_empty(), "{reports:?}");
    }

    /// Frame byte 4's high nibble is never read (spec §3.2).
    #[test]
    fn frame_byte_4_high_nibble_is_reported() {
        let clean = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let mut payload = clean.clone();
        payload[3] |= 0x80;
        assert_eq!(
            reports_leaving_reading_unchanged(&payload, &clean),
            ["ut8802: unrecognised status bits: frame byte 4 = 0x81"]
        );
    }

    /// Frame byte 5 bits 6-7 are never read (spec §3.1); bits 4-5 are the
    /// probe direction and stay silent (`documented_frames_report_nothing`).
    #[test]
    fn frame_byte_5_high_bits_are_reported() {
        let clean = make_payload(0x23, [0, 0, 5, 1, 2], 3, 0x01, 0x00, 0x00);
        let mut payload = clean.clone();
        payload[4] |= 0x40;
        assert_eq!(
            reports_leaving_reading_unchanged(&payload, &clean),
            ["ut8802: unrecognised status bits: frame byte 5 = 0x53"]
        );
    }

    /// Frame byte 7 carries only the error flag (spec §3.6): flags there set
    /// nothing and are reported, so a meter that sends them there says so.
    #[test]
    fn flags_in_frame_byte_7_set_nothing_and_are_reported() {
        let clean = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        for bit in 0..7 {
            let mut payload = clean.clone();
            payload[6] = 1 << bit;
            assert_eq!(
                reports_leaving_reading_unchanged(&payload, &clean),
                [format!(
                    "ut8802: unrecognised status bits: frame byte 7 = {:#04x}",
                    1 << bit
                )]
            );
        }
    }

    /// The error flag is not the sign: frame byte 7 bit 7 leaves the value
    /// as it is, silently.
    #[test]
    fn the_error_flag_does_not_negate() {
        let clean = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let payload = make_payload(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x80);
        assert!(reports_leaving_reading_unchanged(&payload, &clean).is_empty());
    }

    /// Every documented value — each position, digit nibble, decimal point,
    /// probe direction, frame byte 6 value and frame byte 7's error flag —
    /// parses silently.
    #[test]
    fn documented_frames_report_nothing() {
        let ((), reports) = crate::protocol::capture_reports(|| {
            for &(position, _, _, _) in POSITION_TABLE {
                for direction in 0..=3 {
                    for dp_pos in 0..=4 {
                        let payload =
                            make_payload(position, [1, 2, 3, 4, 5], dp_pos, direction, 0, 0);
                        parse_measurement(&payload).unwrap();
                    }
                }
            }
            parse_measurement(&make_payload(0x05, [0x0A, 1, 2, 3, 4], 0, 0, 0, 0)).unwrap();
            for status in 0..=0xFF {
                for error in [0x00, 0x80] {
                    parse_measurement(&make_payload(0x05, [0; 5], 1, 0, status, error)).unwrap();
                }
            }
        });
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x2B, [0, 5, 0, 0, 0], 1, 0x00, 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x2B);
        assert_eq!(m.mode, "Hz");
    }

    // --- Detection (crate::detect) ---------------------------------------

    /// One frame is not evidence — the format has no checksum, so a single
    /// validation pass accepts roughly 1% of random input. Two consecutive
    /// frames are what detection asks for.
    #[test]
    fn two_consecutive_frames_identify_the_meter_and_one_does_not() {
        let frame = frame::test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let mut pair = frame.clone();
        pair.extend_from_slice(&frame);
        let recognise = FINGERPRINT.recognise;
        assert_eq!(
            recognise(&pair, &Probing::default()),
            Some(Evidence::Model {
                device: &devices::UT8802,
                reported_name: None,
            })
        );
        assert_eq!(recognise(&frame, &Probing::default()), None);
    }

    /// Two valid frames with anything between them are not a stream — a
    /// meter sends them back to back, and random bytes are what puts a
    /// second `0xAC` somewhere further along.
    #[test]
    fn two_frames_that_are_not_adjacent_identify_nothing() {
        let frame = frame::test_frame_ut8802(0x05, [1, 2, 3, 4, 5], 1, 0x02, 0x00, 0x00);
        let mut spaced = frame.clone();
        spaced.extend_from_slice(&[0x00, 0x00, 0x00]);
        spaced.extend_from_slice(&frame);
        let recognise = FINGERPRINT.recognise;
        assert_eq!(recognise(&spaced, &Probing::default()), None);
    }

    #[test]
    fn every_positions_labels_are_listed() {
        let all = crate::text::listed();
        let protocol = Ut8802Protocol::new();
        for &(position, _, _, _) in POSITION_TABLE {
            let payload = make_payload(position, [1, 2, 3, 4, 5], 1, 0, 0, 0);
            let m = parse_measurement(&payload).unwrap();
            crate::text::assert_listed(all, &protocol, &m, devices::UT8802.id);
        }
    }
}
