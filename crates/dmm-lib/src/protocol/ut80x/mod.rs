//! UT803/UT804 bench multimeter protocol, which the UT71 and Voltcraft
//! VC920/VC940/VC960 handhelds speak too.
//!
//! These meters send 11-byte packets: 9 data bytes (`0x30`-`0x3F`, with odd
//! parity in bit 7 on the UT804), then CR LF. No checksum. Only the low
//! nibbles carry data, and they hold structured measurement data (mode
//! codes, range codes, digit values, status flags), NOT raw LCD segment
//! data.
//!
//! The UT71A/B, UT71C/D/E and VC920/VC940/VC960 send the UT804's layout:
//! their vendor apps decode it with the UT804 app's parser, at the same 2400
//! baud. Only the range labels differ, as the full scales do, and function
//! code E is their power position
//! (docs/research/ut71/reverse-engineered-protocol.md §1-§4).
//!
//! **UT803 and UT804 use different payload layouts** (2026-06 review,
//! re-derived from UT803.exe V1.01 / UT804.exe V2.00 with string constants
//! resolved from the recovered binaries and the vendor LCD fonts rendered).
//! Positions are the vendor parsers' 1-based indices into the low nibbles.
//!
//! UT804 (position k = byte k):
//! - positions 1-5: digits 1-5 (MSD first; 0xA = blank)
//! - position 6: range (decimal point position via per-mode table)
//! - position 7: mode code (1-15)
//! - position 8: AC/DC (0=default, 1=AC, 2=DC, 3=AC+DC)
//! - position 9: status — bit 3 unknown, bit 2 = **negative sign** (duty-%
//!   selector in frequency mode), bits 1-0: AUTO when == 1
//! - positions 10-11: 0xD 0xA (low nibbles of CR/LF)
//!
//! UT803 (position k = byte k-1: the vendor parser reads 0xA, bytes 1-9,
//! then 0xD):
//! - position 2: range
//! - positions 3-6: digits 1-4 (MSD first)
//! - position 7: mode code (different meanings from UT804!)
//! - position 8: bit 3 = alt-mode (RPM / °C-vs-°F), bit 2 = **negative
//!   sign**, bit 1 = unknown, bit 0 = overload
//! - position 9: bit 3 = HOLD, bits 2-1 = unknown indicators
//! - position 10: bit 3 = DC, bit 2 = AC, bit 1 = AUTO (else MANU)
//! - positions 1, 11: the fixed 0xA and 0xD, never read by the parser
//!
//! The decimal point value in the per-mode range tables is the position
//! FROM THE LEFT (point placed after digit position+1), matching the
//! vendor's display assembly.
//!
//! See docs/research/ut803/reverse-engineered-protocol.md

use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::framing::{self, FrameErrorRecovery};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{
    CaptureStep, DeviceFamily, DeviceProfile, Evidence, Fingerprint, MeterKeys, Probing, Protocol,
    Stability, unknown_mode,
};
use crate::specs::{ModeSpecInfo, ModeSpecs, RangeSpec, SpecInfo, SpecSheetTable};
use crate::transport::{Transport, ch9325};
use log::debug;
use std::borrow::Cow;
use std::fmt;

pub(crate) mod devices;
mod specs_ut803;
mod specs_ut804;

/// Which meter model the packets come from. All share the framing; the
/// UT803 has a payload layout of its own, and the rest send the UT804's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Model {
    Ut803,
    Ut804,
    /// UT71A/B: the UT804's packets, on a 20000-count display
    /// (docs/research/ut71/reverse-engineered-protocol.md §3.5, §4).
    Ut71Ab,
    /// UT71C/D/E: the UT804's packets and range labels (ut71 spec §3.5, §4).
    Ut71Cde,
    /// Voltcraft VC920/VC940/VC960: the UT71C/D/E's packets, with a 750V
    /// top AC V range (ut71 spec §3.5, §4).
    Vc920,
}

impl Model {
    /// Whether the model sends the UT804's payload layout.
    fn ut804_layout(self) -> bool {
        match self {
            Model::Ut803 => false,
            Model::Ut804 | Model::Ut71Ab | Model::Ut71Cde | Model::Vc920 => true,
        }
    }

    /// The registry id reports name the model by, so the hint they print
    /// selects the same parser.
    fn report_id(self) -> &'static str {
        match self {
            Model::Ut803 => "ut803",
            Model::Ut804 => "ut804",
            Model::Ut71Ab => "ut71ab",
            Model::Ut71Cde => "ut71cde",
            Model::Vc920 => "vc920",
        }
    }

    /// The range label this model shows for a (mode, range) pair
    /// [`ut804_mode_info`] labels `ut804`: the same codes and decimal points
    /// on every UT804-layout model, with full scales of their own (ut71 spec
    /// §3.5). The UT803 parser gives no labels and never asks.
    fn range_label(self, mode: u8, range: u8, ut804: &'static str) -> &'static str {
        match self {
            Model::Ut803 | Model::Ut804 | Model::Ut71Cde => ut804,
            Model::Ut71Ab => ut71ab_range_label(mode, range),
            // The Voltcraft manuals stop AC V at 750V; DC V keeps 1000V.
            Model::Vc920 if (mode, range) == (0x2, 4) => "750V",
            Model::Vc920 => ut804,
        }
    }
}

/// UT804 per-(mode, range) display info: mode name, unit, decimal point
/// position from the left (point after digit `pos+1`; a position past the
/// last digit means an integer display), and range label.
///
/// From the UT804.exe parse function `FUN_00558a7c` unit-string appends
/// and range switches (spec §7.4 item 7), with unit glyphs resolved from
/// the vendor LCD fonts (`#`=°C, `?`=°F, `)`=diode, `&`=beeper, `*`=Ω).
/// The labels are the full ranges of the UT804 manual's Table 2-3 (spec
/// §3.7), which match the decimal points, with two corrections. That table
/// gives AC mA the µA ranges, a typo, as the meter's `00.000 mA` on AC mA
/// range 0 shows (#16). It gives AC V's top range as 750V, where the
/// manual's own spec tables and the datasheet give 1000V, as for DC.
/// Temperature has no label, as on the other families, nor do duty and the
/// 4-20 mA %, which the table gives no range.
fn ut804_mode_info(mode: u8, range: u8) -> Option<(&'static str, &'static str, u8, &'static str)> {
    Some(match mode {
        // Modes 1 and 2 have byte-identical handlers; the AC/DC label
        // comes solely from position 8. DC V sends 1, AC V 2 (#16).
        0x1 | 0x2 => match range {
            1 => ("V", "V", 0, "4V"),    // 3.9999
            2 => ("V", "V", 1, "40V"),   // 39.999
            3 => ("V", "V", 2, "400V"),  // 399.99
            4 => ("V", "V", 3, "1000V"), // 1000.0
            _ => return None,
        },
        0x3 => ("mV", "mV", 2, "400mV"), // 399.99 fixed
        0x4 => match range {
            1 => ("Ω", "Ω", 2, "400Ω"), // 399.99 Ω
            2 => ("Ω", "kΩ", 0, "4kΩ"),
            3 => ("Ω", "kΩ", 1, "40kΩ"),
            4 => ("Ω", "kΩ", 2, "400kΩ"),
            5 => ("Ω", "MΩ", 0, "4MΩ"),
            6 => ("Ω", "MΩ", 1, "40MΩ"),
            _ => return None,
        },
        0x5 => match range {
            1 => ("Capacitance", "nF", 1, "40nF"),
            2 => ("Capacitance", "nF", 2, "400nF"),
            3 => ("Capacitance", "µF", 0, "4µF"),
            4 => ("Capacitance", "µF", 1, "40µF"),
            5 => ("Capacitance", "µF", 2, "400µF"),
            6 => ("Capacitance", "mF", 0, "4mF"),
            7 => ("Capacitance", "mF", 1, "40mF"),
            _ => return None,
        },
        0x6 => ("Temperature", "°C", 3, ""),
        0x7 => match range {
            0 => ("µA", "µA", 2, "400µA"),  // 399.99
            1 => ("µA", "µA", 3, "4000µA"), // 3999.9
            _ => return None,
        },
        0x8 => match range {
            0 => ("mA", "mA", 1, "40mA"),  // 39.999
            1 => ("mA", "mA", 2, "400mA"), // 399.99
            _ => return None,
        },
        0x9 => ("A", "A", 1, "10A"), // 10.000
        0xA => ("Continuity", "Ω", 2, "400Ω"),
        0xB => ("Diode", "V", 0, "4V"),
        0xC => match range {
            0 => ("Frequency", "Hz", 1, "40Hz"),
            1 => ("Frequency", "Hz", 2, "400Hz"),
            2 => ("Frequency", "kHz", 0, "4kHz"),
            3 => ("Frequency", "kHz", 1, "40kHz"),
            4 => ("Frequency", "kHz", 2, "400kHz"),
            5 => ("Frequency", "MHz", 0, "4MHz"),
            6 => ("Frequency", "MHz", 1, "40MHz"),
            7 => ("Frequency", "MHz", 2, "400MHz"),
            _ => return None,
        },
        0xD => ("Temperature", "°F", 3, ""),
        // Power in watts, on the UT71E's and VC940's W position; the UT804
        // has none and never sends it
        // (docs/research/ut71/reverse-engineered-protocol.md §3.2).
        0xE => ("Power", "W", 3, ""),
        // The 4-20 mA loop current as a % reading, on the mA position (UT804
        // manual Table 2-1). The mode keeps the vendor's unit string "mA%"
        // as its name; the LCD shows the unit as "%" (#16).
        0xF => ("mA%", "%", 2, ""),
        _ => return None,
    })
}

/// UT804 modes whose AC/DC nibble value 0 defaults to a DC label
/// (vendor applies "DC" to V/mV/µA/mA/A, spec §3.5).
fn ut804_default_dc(mode: u8) -> bool {
    matches!(mode, 0x1 | 0x2 | 0x3 | 0x7 | 0x8 | 0x9)
}

/// UT71A/B range labels for the pairs [`ut804_mode_info`] knows: the UT804's
/// codes and decimal points on a 20000-count display, so half its full
/// scales, with 1000V still on top. Each is the full scale both the A/B
/// vendor app and the UT71 manual's A/B ranges give
/// (docs/research/ut71/reverse-engineered-protocol.md §3.5). Neither gives
/// continuity or diode a range, so they have no label, nor, as on the
/// UT804, do temperature, power and the 4-20 mA %.
fn ut71ab_range_label(mode: u8, range: u8) -> &'static str {
    match (mode, range) {
        (0x1 | 0x2, 1) => "2V",
        (0x1 | 0x2, 2) => "20V",
        (0x1 | 0x2, 3) => "200V",
        (0x1 | 0x2, 4) => "1000V",
        (0x3, _) => "200mV",
        (0x4, 1) => "200Ω",
        (0x4, 2) => "2kΩ",
        (0x4, 3) => "20kΩ",
        (0x4, 4) => "200kΩ",
        (0x4, 5) => "2MΩ",
        (0x4, 6) => "20MΩ",
        (0x5, 1) => "20nF",
        (0x5, 2) => "200nF",
        (0x5, 3) => "2µF",
        (0x5, 4) => "20µF",
        (0x5, 5) => "200µF",
        (0x5, 6) => "2mF",
        (0x5, 7) => "20mF",
        (0x7, 0) => "200µA",
        (0x7, 1) => "2000µA",
        (0x8, 0) => "20mA",
        (0x8, 1) => "200mA",
        (0x9, _) => "10A",
        (0xC, 0) => "20Hz",
        (0xC, 1) => "200Hz",
        (0xC, 2) => "2kHz",
        (0xC, 3) => "20kHz",
        (0xC, 4) => "200kHz",
        (0xC, 5) => "2MHz",
        (0xC, 6) => "20MHz",
        (0xC, 7) => "200MHz",
        _ => "",
    }
}

/// UT803 per-(mode, range) display info, same convention as
/// [`ut804_mode_info`] but with 4 digits and the UT803's own mode codes
/// (spec §7.4 item 4). `alt` is position 8 bit 3 (selects RPM for
/// frequency, °C vs °F for temperature).
fn ut803_mode_info(mode: u8, range: u8, alt: bool) -> Option<(&'static str, &'static str, u8)> {
    Some(match mode {
        0xB => match range {
            0 => ("V", "V", 0),   // 5.999
            1 => ("V", "V", 1),   // 59.99
            2 => ("V", "V", 2),   // 599.9
            3 => ("V", "V", 3),   // 1000
            4 => ("mV", "mV", 2), // 599.9 mV
            _ => return None,
        },
        0xD => match range {
            0 => ("µA", "µA", 2), // 599.9
            1 => ("µA", "µA", 3), // 5999
            _ => return None,
        },
        0xF => match range {
            0 => ("mA", "mA", 1), // 59.99
            1 => ("mA", "mA", 2), // 599.9
            _ => return None,
        },
        0x9 => ("A", "A", 1), // 10.00
        0x3 => match range {
            0 => ("Ω", "Ω", 2), // 599.9
            1 => ("Ω", "kΩ", 0),
            2 => ("Ω", "kΩ", 1),
            3 => ("Ω", "kΩ", 2),
            4 => ("Ω", "MΩ", 0),
            5 => ("Ω", "MΩ", 1),
            _ => return None,
        },
        0x2 => {
            // Frequency, or tachometer RPM when the alt bit is set.
            // Range 0's decimal position is [UNVERIFIED] (the vendor
            // handler for it is ambiguous); treat as integer Hz.
            let (name, unit) = if alt {
                ("Tachometer", "RPM")
            } else {
                ("Frequency", "Hz")
            };
            match (range, alt) {
                (0, _) => (name, unit, 3),
                (1, false) => (name, "kHz", 1),
                (2, false) => (name, "kHz", 2),
                (3, false) => (name, "MHz", 0),
                (4, false) => (name, "MHz", 1),
                (5, false) => (name, "MHz", 2),
                (1, true) => (name, "kRPM", 1),
                (2, true) => (name, "kRPM", 2),
                (3, true) => (name, "MRPM", 0),
                (4, true) => (name, "MRPM", 1),
                (5, true) => (name, "MRPM", 2),
                _ => return None,
            }
        }
        0x4 => {
            if alt {
                ("Temperature", "°C", 3)
            } else {
                ("Temperature", "°F", 3)
            }
        }
        0x5 => ("Continuity", "Ω", 3),
        0x1 => ("Diode", "V", 0), // 5.999
        0x6 => match range {
            0 => ("Capacitance", "nF", 0),
            1 => ("Capacitance", "nF", 1),
            2 => ("Capacitance", "nF", 2),
            3 => ("Capacitance", "µF", 0),
            4 => ("Capacitance", "µF", 1),
            5 => ("Capacitance", "µF", 2),
            6 => ("Capacitance", "mF", 0),
            7 => ("Capacitance", "mF", 1),
            _ => return None,
        },
        // Two indicators, no unit, possibly hFE.
        0xE => ("ADP", "", 3),
        _ => return None,
    })
}

/// The packet being parsed, for reporting what in it no spec section
/// covers. Displays as the low nibbles of data bytes 1-9 in hex, the same
/// wire order for both models.
#[derive(Clone, Copy)]
struct Unrecognised<'a> {
    model: &'static str,
    nibbles: &'a [u8],
}

impl Unrecognised<'_> {
    fn report(self, what: &'static str) {
        report_unknown(self.model, what, format_args!("nibbles {self}"));
    }
}

impl fmt::Display for Unrecognised<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.nibbles.iter().try_for_each(|n| write!(f, "{n:X}"))
    }
}

/// Build the display string and numeric value from MSD-first digit
/// nibbles and a decimal position from the left (point after digit
/// `dp_pos+1`). Digit nibble 0xA renders as a blank (trailing blank =
/// 4-digit reading on the UT804's 5-digit field, its 4000-count setting).
/// `blank` is the digit the spec documents as blank, if any; a blank
/// anywhere else still renders as one, and is reported.
fn assemble_value(
    unrecognised: Unrecognised,
    digits: &[u8],
    blank: Option<usize>,
    dp_pos: u8,
    negative: bool,
) -> Result<(String, f64)> {
    // Digits are 0-9 (spec §3.2), blank only where `blank` says (§3.1).
    if digits
        .iter()
        .enumerate()
        .any(|(i, &d)| d > 0xA || (d == 0xA && blank != Some(i)))
    {
        unrecognised.report("digit nibble");
    }
    let mut s = String::with_capacity(digits.len() + 2);
    if negative {
        s.push('-');
    }
    let mut digit_count = 0usize;
    for &d in digits {
        match d {
            0x0..=0x9 => {
                s.push((b'0' + d) as char);
                digit_count += 1;
            }
            0xA => {} // blank digit
            _ => {
                return Err(Error::invalid_response(
                    format!("ut80x invalid digit nibble {d:#04x}"),
                    digits,
                ));
            }
        }
        // Insert the decimal point after digit position dp_pos+1
        // (skipped when dp_pos points past the last digit = integer).
        if digit_count == dp_pos as usize + 1 && digit_count < digits.len() {
            s.push('.');
        }
    }
    let trimmed = s.trim_end_matches('.').to_string();
    let value: f64 = trimmed.parse().map_err(|_| {
        unrecognised.report("value");
        Error::invalid_response(format!("ut80x unparseable value {trimmed:?}"), digits)
    })?;
    Ok((trimmed, value))
}

/// Packet length: 9 data bytes, CR, LF (spec §2.1).
const PACKET_LEN: usize = 11;

/// The packet terminator, compared with bit 7 masked: issue #16's UT804
/// sends CR as `0D` and LF as `8A`.
const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

/// Whether `bytes` is one whole packet: nine `0x30`-`0x3F` data bytes, CR,
/// LF.
///
/// Bit 7 is masked, not checked. The UT804 puts odd parity there (7O1 read
/// as 8N1, issue #16), but which data-bit setting the CH9325 takes from our
/// feature report is unverified (spec §1.2), so a stream with the parity
/// bit stripped is accepted too.
fn is_packet(bytes: &[u8]) -> bool {
    bytes.len() == PACKET_LEN
        && bytes[..PACKET_LEN - 2].iter().all(|b| b & 0x70 == 0x30)
        && bytes[PACKET_LEN - 2] & 0x7F == CR
        && bytes[PACKET_LEN - 1] & 0x7F == LF
}

/// Find the first whole packet in `buf`.
///
/// Returns the packet's bytes as received, bit 7 included, and the offset
/// just past it. Never fails: the stream has no header to resync on, so a
/// partial first packet, or one that lost a byte, is only a window that is
/// not a packet — the previous packet's CR or LF sits where a data byte
/// should be. `Ok(None)` until a whole packet has arrived.
fn extract_packet(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    Ok(buf
        .windows(PACKET_LEN)
        .position(is_packet)
        .map(|start| (buf[start..start + PACKET_LEN].to_vec(), start + PACKET_LEN)))
}

/// The low nibbles of a whole packet: the UT804 parser's positions 1-11.
fn low_nibbles(packet: &[u8]) -> Result<[u8; PACKET_LEN]> {
    if !is_packet(packet) {
        return Err(Error::invalid_response(
            "ut80x: not a packet of 9 data bytes and CR LF",
            packet,
        ));
    }
    let mut nibbles = [0u8; PACKET_LEN];
    for (nibble, byte) in nibbles.iter_mut().zip(packet) {
        *nibble = byte & 0x0F;
    }
    Ok(nibbles)
}

/// The AC/DC coupling a packet names for its reading.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Coupling {
    Dc,
    Ac,
    AcDc,
}

/// A UT804 packet's positions (spec §2.1), read here once for everything
/// that decodes a UT804 payload.
struct Ut804Fields {
    /// Positions 1-11; `nibbles[k-1]` is position k.
    nibbles: [u8; PACKET_LEN],
    /// Position 6 (spec §3.7).
    range: u8,
    /// Position 7 (spec §3.4).
    mode: u8,
    /// Position 8 (spec §3.5).
    acdc: u8,
    /// Position 9 (spec §3.6).
    status: u8,
}

impl Ut804Fields {
    /// Fails on anything that is not one whole packet.
    fn decode(packet: &[u8]) -> Result<Self> {
        let nibbles = low_nibbles(packet)?;
        Ok(Self {
            nibbles,
            range: nibbles[5],
            mode: nibbles[6],
            acdc: nibbles[7],
            status: nibbles[8],
        })
    }

    /// Status bit 2: the minus sign (spec §3.6).
    fn sign_bit(&self) -> bool {
        self.status & 0x4 != 0
    }

    /// In frequency mode the sign bit selects the duty-cycle display
    /// (spec §7.4 item 7) — a negative frequency is impossible, so the
    /// bit is reused.
    fn duty(&self) -> bool {
        self.mode == 0xC && self.sign_bit()
    }

    /// The coupling position 8 names (spec §3.5): 1 AC, 2 DC, 3 AC+DC, and
    /// 0 DC on the modes [`ut804_default_dc`] lists. A 1-3 on a mode without
    /// coupling is still returned (the parser reports it); `None` is a 0
    /// there, or a value past 3.
    fn coupling(&self) -> Option<Coupling> {
        match self.acdc {
            1 => Some(Coupling::Ac),
            2 => Some(Coupling::Dc),
            3 => Some(Coupling::AcDc),
            0 if ut804_default_dc(self.mode) => Some(Coupling::Dc),
            _ => None,
        }
    }
}

/// A UT803 packet's positions (spec §2.1), read here once for everything
/// that decodes a UT803 payload.
struct Ut803Fields {
    /// The vendor parser reads 0xA, the low nibbles of bytes 1-9, then
    /// 0xD; this rebuilds that string, so `nibbles[k-1]` is position k, as
    /// for the UT804.
    nibbles: [u8; PACKET_LEN],
    /// Position 2.
    range: u8,
    /// Position 7 (spec §7.4 item 4).
    mode: u8,
    /// Position 8: alt, sign, overload (module doc).
    nib8: u8,
    /// Position 9: HOLD and indicators (module doc).
    nib9: u8,
    /// Position 10: DC, AC, AUTO (module doc).
    nib10: u8,
}

impl Ut803Fields {
    /// Fails on anything that is not one whole packet.
    fn decode(packet: &[u8]) -> Result<Self> {
        let wire = low_nibbles(packet)?;
        let mut nibbles = [0u8; PACKET_LEN];
        nibbles[0] = 0xA;
        nibbles[1..PACKET_LEN - 1].copy_from_slice(&wire[..PACKET_LEN - 2]);
        nibbles[PACKET_LEN - 1] = 0xD;
        Ok(Self {
            nibbles,
            range: nibbles[1],
            mode: nibbles[6],
            nib8: nibbles[7],
            nib9: nibbles[8],
            nib10: nibbles[9],
        })
    }

    /// Position 8 bit 3: RPM for frequency, °C for temperature (spec §7.4
    /// item 4).
    fn alt(&self) -> bool {
        self.nib8 & 0x8 != 0
    }

    /// Position 10 bit 3 is DC and bit 2 AC; both is AC+DC, open on the
    /// UT803 (spec §5), and neither is `None`.
    fn coupling(&self) -> Option<Coupling> {
        match (self.nib10 & 0x8 != 0, self.nib10 & 0x4 != 0) {
            (true, false) => Some(Coupling::Dc),
            (false, true) => Some(Coupling::Ac),
            (true, true) => Some(Coupling::AcDc),
            (false, false) => None,
        }
    }
}

/// Mode name, unit and decimal point position, falling back to an unnamed
/// mode when the (mode, range) pair is absent from the per-model table.
fn mode_info_or_unknown(
    unrecognised: Unrecognised,
    info: Option<(&'static str, &'static str, u8)>,
) -> (&'static str, &'static str, u8) {
    info.unwrap_or_else(|| {
        // The tables hold every pair the spec gives (§7.4 items 4 and 7).
        unrecognised.report("mode/range pair");
        ("?", "", 3)
    })
}

/// The vendor app's overload text, signed when the sign bit is set.
fn overload_display(negative: bool) -> Option<String> {
    Some(if negative {
        "-0L".to_string()
    } else {
        "0L".to_string()
    })
}

/// What a UT804's LCD shows for a packet whose digit 1 is blank — an
/// overload, or LO on the 4-20 mA % reading: the digit nibbles as drawn, A
/// a blank and C an `L`, with the range's decimal point after digit
/// `dp_pos + 1` and the sign in front (spec §3.3). `None` where a nibble is
/// one the LCD has not been seen to draw.
///
/// Issue #16's LCD confirmed four frames: `A A 0 C A` on Ω range 6 shows
/// `.OL`, on diode `. OL`, on continuity `0.L`, and `A C 0 A A` on mode F
/// with the sign bit `- LO.`. The 7-segment O is the digit 0, so the text
/// keeps `0`, and a blank draws nothing, so the diode's reads `.0L`.
fn ut804_lcd_text(digits: &[u8], dp_pos: u8, negative: bool) -> Option<String> {
    let mut s = String::with_capacity(digits.len() + 2);
    if negative {
        s.push('-');
    }
    for (i, &d) in digits.iter().enumerate() {
        match d {
            0x0..=0x9 => s.push((b'0' + d) as char),
            0xA => {}
            0xC => s.push('L'),
            _ => return None,
        }
        // The point follows a digit position, drawn or blank: Ω range 6
        // has two blanks before it. None past the last digit.
        if i == dp_pos as usize && i + 1 < digits.len() {
            s.push('.');
        }
    }
    Some(s)
}

/// Parse a UT804 packet. Position k is byte k (spec §2.1), so
/// `nibbles[k-1]` is position k.
#[cfg(test)]
fn parse_measurement_ut804(packet: &[u8]) -> Result<Measurement> {
    parse_ut804_layout(Model::Ut804, packet)
}

/// Parse a packet in the UT804's layout for `model`. The UT71 and VC9x0 send
/// the UT804's packets, which their vendor apps decode with the UT804 app's
/// parser (docs/research/ut71/reverse-engineered-protocol.md §2, §3); only
/// the range labels, and the id in reports, differ.
fn parse_ut804_layout(model: Model, packet: &[u8]) -> Result<Measurement> {
    let f = Ut804Fields::decode(packet)?;
    let nibbles = &f.nibbles;
    let unrecognised = Unrecognised {
        model: model.report_id(),
        nibbles: &nibbles[..9],
    };

    // Status nibble (vendor char 9, spec §3.6):
    // bit 3 stripped (unknown), bit 2 = sign, remaining value == 1 → AUTO.
    let sign_bit = f.sign_bit();
    let auto_range = f.status & 0x3 == 0x1;
    // Bit 3 has no known meaning (§3.6); bits 1 and 0 are MAN and AUTO,
    // never both (§8).
    if f.status & 0x8 != 0 || f.status & 0x3 == 0x3 {
        unrecognised.report("status bits");
    }
    // Coupling takes values 0-3, and only V, mV, µA, mA and A have one
    // (§3.5).
    if f.acdc > 3 || (f.acdc != 0 && !ut804_default_dc(f.mode)) {
        unrecognised.report("ac/dc nibble");
    }

    let coupling = f.coupling();
    let info = ut804_mode_info(f.mode, f.range);
    let (mode_name, unit, dp_pos) = mode_info_or_unknown(
        unrecognised,
        info.map(|(name, unit, dp_pos, _)| (name, unit, dp_pos)),
    );

    let (mode, negative, dp_pos, unit): (Cow<'static, str>, bool, u8, &'static str) = if f.duty() {
        (Cow::Borrowed("Duty %"), false, 2, "%")
    } else if mode_name == "?" {
        (unknown_mode(f.mode), sign_bit, dp_pos, unit)
    } else {
        // AC/DC labeling comes from position 8 for the V/mV/current
        // modes (0 = default DC); other modes keep their plain name.
        let label = match coupling {
            Some(Coupling::Ac) => match mode_name {
                "V" => Some("AC V"),
                "mV" => Some("AC mV"),
                "µA" => Some("AC µA"),
                "mA" => Some("AC mA"),
                "A" => Some("AC A"),
                _ => None,
            },
            Some(Coupling::Dc) => match mode_name {
                "V" => Some("DC V"),
                "mV" => Some("DC mV"),
                "µA" => Some("DC µA"),
                "mA" => Some("DC mA"),
                "A" => Some("DC A"),
                _ => None,
            },
            Some(Coupling::AcDc) => match mode_name {
                "V" => Some("AC+DC V"),
                "mV" => Some("AC+DC mV"),
                "µA" => Some("AC+DC µA"),
                "mA" => Some("AC+DC mA"),
                "A" => Some("AC+DC A"),
                _ => None,
            },
            None => None,
        };
        (
            Cow::Borrowed(label.unwrap_or(mode_name)),
            sign_bit,
            dp_pos,
            unit,
        )
    };
    // Duty has no range of its own (spec §3.7).
    let range_label = match info {
        Some((.., label)) if !f.duty() => model.range_label(f.mode, f.range, label),
        _ => "",
    };

    let dc = matches!(coupling, Some(Coupling::Dc | Coupling::AcDc));

    // Overload frames: digit 1 = 0xA. The vendor reads 0.0 ("L0") when
    // digit 2 == 0xC, and an overload (possibly negative) otherwise;
    // digits 3-5 are ignored (spec §7.4 item 6). An idle frame (digit 4 ==
    // 0xB) shows all zeros.
    let (value, display_raw) = if nibbles[0] == 0xA {
        // Digit 2 is A (overload), C ("L0") or F ("HI") in the known
        // overload patterns (§8).
        if !matches!(nibbles[1], 0xA | 0xC | 0xF) {
            unrecognised.report("overload pattern");
        }
        // The LCD's own text for the patterns it has been seen to draw,
        // the vendor's fixed "L0" / "0L" for the rest.
        let lcd = matches!(nibbles[1], 0xA | 0xC)
            .then(|| ut804_lcd_text(&nibbles[0..5], dp_pos, negative))
            .flatten();
        if nibbles[1] == 0xC {
            (
                MeasuredValue::Normal(0.0),
                lcd.or_else(|| Some("L0".to_string())),
            )
        } else {
            (
                MeasuredValue::Overload,
                lcd.or_else(|| overload_display(negative)),
            )
        }
    } else if nibbles[3] == 0xB {
        (MeasuredValue::Normal(0.0), Some("0".to_string()))
    } else {
        // Digit 5 is blank on the 4000-count display (§3.1, §5).
        let (display, v) = assemble_value(unrecognised, &nibbles[0..5], Some(4), dp_pos, negative)?;
        (MeasuredValue::Normal(v), Some(display))
    };

    let flags = StatusFlags {
        auto_range,
        dc,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: f.mode as u16,
        range_raw: f.range,
        value,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label),
        display_raw,
        flags,
        ..Measurement::from_payload(packet)
    })
}

/// Parse a UT803 packet, by the vendor parser's positions
/// ([`Ut803Fields`]).
pub(crate) fn parse_measurement_ut803(packet: &[u8]) -> Result<Measurement> {
    let f = Ut803Fields::decode(packet)?;
    let nibbles = &f.nibbles;
    // Positions 2-10 are data bytes 1-9.
    let unrecognised = Unrecognised {
        model: Model::Ut803.report_id(),
        nibbles: &nibbles[1..10],
    };

    let alt = f.alt();
    let negative = f.nib8 & 0x4 != 0;
    let overload = f.nib8 & 0x1 != 0;
    // HOLD lights the LCDHold widget from char 9 bit 3
    // (spec §7.4 item 2); bits 2-1 drive unlabeled indicators.
    let hold = f.nib9 & 0x8 != 0;
    let dc = f.nib10 & 0x8 != 0;
    let auto_range = f.nib10 & 0x2 != 0;
    // Nibble 8 bit 1 and nibble 9 bit 0 have no known meaning, and the alt
    // bit only picks RPM or °C (§5, §7.4 items 2 and 4; bit map in the
    // module doc). Nibble 9 bits 2-1 are left out: the vendor lights
    // indicators from them, so the meter sets them in ordinary use.
    if f.nib8 & 0x2 != 0 || f.nib9 & 0x1 != 0 || (alt && !matches!(f.mode, 0x2 | 0x4)) {
        unrecognised.report("status bits");
    }

    let (mode_name, unit, dp_pos) =
        mode_info_or_unknown(unrecognised, ut803_mode_info(f.mode, f.range, alt));

    let mode: Cow<'static, str> = if mode_name == "?" {
        unknown_mode(f.mode)
    } else if mode_name == "V" || mode_name == "mV" {
        // Volts are DC or AC: both bits is AC+DC, open on the UT803 (§5),
        // and neither is undocumented.
        if !matches!(f.coupling(), Some(Coupling::Dc | Coupling::Ac)) {
            unrecognised.report("ac/dc bits");
        }
        Cow::Borrowed(match (mode_name, dc) {
            ("V", true) => "DC V",
            ("V", false) => "AC V",
            ("mV", true) => "DC mV",
            _ => "AC mV",
        })
    } else {
        Cow::Borrowed(mode_name)
    };

    let (value, display_raw) = if overload {
        (MeasuredValue::Overload, overload_display(negative))
    } else {
        // The spec documents no UT803 blank digit (§7.4 item 4).
        let (display, v) = assemble_value(unrecognised, &nibbles[2..6], None, dp_pos, negative)?;
        (MeasuredValue::Normal(v), Some(display))
    };

    let flags = StatusFlags {
        hold,
        auto_range,
        dc,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: f.mode as u16,
        range_raw: f.range,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags,
        ..Measurement::from_payload(packet)
    })
}

// --- Protocol trait implementation ---

const COMMANDS: &[&str] = &[];

/// How long the UT803's init leaves the CH9325 after changing its rate,
/// as the transport's own start-up does after its first report.
const UT803_RATE_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// Protocol implementation for the meters on the CH9325 11-byte packet
/// stream: the UT803/UT804 bench meters, and the UT71 and Voltcraft VC9x0
/// handhelds that send the UT804's packets.
pub(crate) struct Ut80xProtocol {
    rx_buf: Vec<u8>,
    model: Model,
    profile: DeviceProfile,
}

impl Ut80xProtocol {
    fn new(
        model: Model,
        model_name: &'static str,
        stability: Stability,
        verification_issue: u16,
    ) -> Self {
        Self {
            rx_buf: Vec::with_capacity(128),
            model,
            profile: DeviceProfile {
                family_name: "UT803/UT804",
                model_name,
                stability,
                supported_commands: COMMANDS,
                max_aux_values: 0,
                verification_issue: Some(verification_issue),
                meter_keys: MeterKeys::NONE,
            },
        }
    }

    pub(crate) fn new_ut803() -> Self {
        Self::new(Model::Ut803, "UNI-T UT803", Stability::Experimental, 15)
    }

    pub(crate) fn new_ut804() -> Self {
        Self::new(Model::Ut804, "UNI-T UT804", Stability::Verified, 16)
    }

    pub(crate) fn new_ut71ab() -> Self {
        Self::new(Model::Ut71Ab, "UNI-T UT71A/B", Stability::Experimental, 22)
    }

    pub(crate) fn new_ut71cde() -> Self {
        Self::new(
            Model::Ut71Cde,
            "UNI-T UT71C/D/E",
            Stability::Experimental,
            22,
        )
    }

    pub(crate) fn new_vc920() -> Self {
        Self::new(
            Model::Vc920,
            "Voltcraft VC920/VC940/VC960",
            Stability::Experimental,
            23,
        )
    }

    /// The manual table for reading `m`, decoded again from the payload:
    /// the coupling, alt and sign bits that pick the table are not in the
    /// reading's fields. With it, the range byte whose row the reading
    /// takes, or `None` where it takes the table's mode data only. `None`
    /// for a payload that is not a packet; a pair the parser does not know
    /// has no spec either.
    fn spec_table(&self, m: &Measurement) -> Option<(&'static ModeSpecs, Option<u8>)> {
        match self.model {
            Model::Ut803 => {
                let f = Ut803Fields::decode(&m.raw_payload).ok()?;
                ut803_mode_info(f.mode, f.range, f.alt())?;
                let table = specs_ut803::table(f.mode, f.range, f.alt(), f.coupling())?;
                Some((table, Some(f.range)))
            }
            Model::Ut804 => {
                let f = Ut804Fields::decode(&m.raw_payload).ok()?;
                ut804_mode_info(f.mode, f.range)?;
                let table = specs_ut804::table(f.mode, f.coupling(), f.duty())?;
                let row = specs_ut804::has_rows(f.coupling()).then_some(f.range);
                Some((table, row))
            }
            // No spec tables transcribed from their manuals yet.
            Model::Ut71Ab | Model::Ut71Cde | Model::Vc920 => None,
        }
    }

    /// The manual row reading `m` takes, if any.
    fn spec_row(&self, m: &Measurement) -> Option<&'static RangeSpec> {
        let (table, range) = self.spec_table(m)?;
        table.row(range?)
    }
}

impl Protocol for Ut80xProtocol {
    fn init(&mut self, transport: &dyn Transport) -> Result<()> {
        // The meter streams once its SEND or RS232 button is on (spec
        // §4.2); nothing is sent to it. [UNVERIFIED] whether the CH9325
        // transport's 0x5A trigger helps.
        match self.model {
            // The CH9325 transport starts at 2400 baud, the UT804's rate,
            // and a UT804 streams there (#16). The UT71 and VC9x0 vendor apps
            // set the same 2400 baud (docs/research/ut71/
            // reverse-engineered-protocol.md §1).
            Model::Ut804 | Model::Ut71Ab | Model::Ut71Cde | Model::Vc920 => {
                debug!(
                    "ut80x: init ({}, the transport's 2400 baud)",
                    self.profile.model_name
                );
            }
            // The UT803 talks at 19200 (spec §1.2), which the transport's
            // start-up only reaches when nothing answers at 2400 — and the
            // bridge reports even while the meter is silent (spec §2.2).
            Model::Ut803 => {
                debug!("ut80x: init (UT803, setting the CH9325 to 19200 baud)");
                transport.send_feature_report(&ch9325::FALLBACK_FEATURE_REPORT)?;
                std::thread::sleep(UT803_RATE_SETTLE);
            }
        }
        Ok(())
    }

    fn request_measurement(&mut self, transport: &dyn Transport) -> Result<Measurement> {
        // `extract_packet` never fails, so the skip pattern is never used;
        // `read_frame` only needs it non-empty, and CR is always 0x0D.
        let packet = framing::read_frame(
            &mut self.rx_buf,
            transport,
            extract_packet,
            |_| true,
            FrameErrorRecovery::Propagate,
            "ut80x",
            &[CR],
        )?;
        self.parse_payload(&packet)
    }

    /// The parsers refuse anything that is not a whole packet, so a golden
    /// fixture can only pin what the stream would have delivered.
    fn parse_payload(&self, payload: &[u8]) -> Result<Measurement> {
        match self.model {
            Model::Ut803 => parse_measurement_ut803(payload),
            Model::Ut804 | Model::Ut71Ab | Model::Ut71Cde | Model::Vc920 => {
                parse_ut804_layout(self.model, payload)
            }
        }
    }

    fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    fn spec_info(&self, m: &Measurement) -> Option<&'static SpecInfo> {
        self.spec_row(m).map(|row| &row.spec)
    }

    fn mode_spec_info(&self, m: &Measurement) -> Option<&'static ModeSpecInfo> {
        self.spec_table(m).map(|(table, _)| &table.mode)
    }

    fn spec_sheet(&self) -> Vec<SpecSheetTable> {
        match self.model {
            Model::Ut803 => specs_ut803::ALL.iter().map(|t| t.sheet_table()).collect(),
            Model::Ut804 => specs_ut804::ALL.iter().map(|t| t.sheet_table()).collect(),
            Model::Ut71Ab | Model::Ut71Cde | Model::Vc920 => Vec::new(),
        }
    }

    fn capture_steps(&self) -> Vec<CaptureStep> {
        use crate::protocol::steps::{self, Ohms, Volts};
        use crate::protocol::{Expect, Need, RangeExpect};
        use Model::{Ut71Ab, Ut71Cde, Ut803, Ut804, Vc920};

        // The two parsers name the same dial position differently, and a
        // label this model cannot report leaves the step waiting for a state
        // that never arrives. Only the UT803's V and mV carry an AC/DC
        // prefix — its current modes are named by unit alone — and it has
        // neither the duty-cycle display nor the "mA%" mode, while AC+DC is
        // the UT804 layout's.
        let ut804_layout = self.model.ut804_layout();
        let for_model = |ut804: Option<&'static str>, ut803_label: Option<&'static str>| {
            if ut804_layout { ut804 } else { ut803_label }
        };
        // Assert the mode where this model has a label for it, and nothing
        // where it has not.
        let named = |step: CaptureStep, label: Option<&'static str>| match label {
            Some(label) => step.expect(Expect::mode(label)),
            None => step,
        };
        // The UT71 and VC9x0 handhelds reach AC and each position's other
        // functions with the blue button and AC+DC with the yellow one, and
        // turn SEND on by holding MAX MIN, or with the UT71A's SEND key (UT71
        // manual Tables 2-1 and 2-2; VC920/940/960 manual §6, §7). `say`
        // picks their wording of a step the bench meters word otherwise.
        let handheld = match self.model {
            Ut803 | Ut804 => false,
            Ut71Ab | Ut71Cde | Vc920 => true,
        };
        let say = |bench: &'static str, handheld_text: &'static str| {
            if handheld { handheld_text } else { bench }
        };

        // These models reach OL in a step of their own, after a plain "turn
        // the dial" one, so the resistance gate is the second of the pair
        // rather than the step called "ohm".
        let [dcv, dcv_short, dcv_negative, ohm_ol, ohm_body, ohm_short] = steps::gate_steps(
            Volts::DcV,
            CaptureStep::basic("dcv", "Set meter to DC V"),
            Ohms::Word,
            CaptureStep::basic(
                "ohm_ol",
                "Set meter to Resistance (Ω) with open leads (overload)",
            ),
        );

        // Which models' dial or buttons reach a step. The UT804 has no AC mV,
        // tachometer or ADP position (its manual's Table 2-1, #16); RANGE,
        // MAX MIN and REL are asked of the UT804 and the handhelds. On the
        // handhelds the blue button steps resistance to continuity and
        // diode, and Hz follows mV, whose position the UT71B/C/D and
        // VC920/VC960 share it with; a UT71E or VC940 turns ahead to its
        // °C/°F position for Hz and back. W sits between Ω and capacitance,
        // as on those two dials. Issue #16's reporter walked every UT804 step
        // by 2026-09-19; no other model has answered.
        const ALL: &[Model] = &[Ut803, Ut804, Ut71Ab, Ut71Cde, Vc920];
        const BENCH: &[Model] = &[Ut803, Ut804];
        const HANDHELD: &[Model] = &[Ut71Ab, Ut71Cde, Vc920];
        const BUTTONS: &[Model] = &[Ut804, Ut71Ab, Ut71Cde, Vc920];
        let model = self.model;
        let on = |models: &[Model], step: CaptureStep| {
            models
                .contains(&model)
                .then(|| step.verified_if(model == Ut804))
        };

        [
            on(ALL, dcv),
            on(ALL, dcv_short),
            on(ALL, dcv_negative),
            on(
                ALL,
                CaptureStep::basic(
                    "acv",
                    say(
                        "Set meter to AC V",
                        "Set meter to AC V (on a shared V position, press the blue button for AC)",
                    ),
                )
                .expect(Expect::mode("AC V")),
            ),
            // AC+DC is the AC/DC nibble's fourth value; the spec (§5) has it
            // on the UT804, behind a button of its own on the AC V position,
            // and leaves the UT803 open. The handhelds' yellow button gives
            // it on AC V (UT71 manual Table 2-2).
            on(
                ALL,
                named(
                    CaptureStep::basic(
                        "acdcv",
                        match model {
                            Ut803 => "Set meter to AC+DC V (if the meter has it)",
                            Ut804 => "AC V: press the AC/AC+DC button so the display shows AC+DC",
                            Ut71Ab | Ut71Cde | Vc920 => {
                                "AC V: press the yellow AC+DC button so the display shows AC+DC"
                            }
                        },
                    ),
                    for_model(Some("AC+DC V"), None),
                ),
            ),
            on(
                ALL,
                CaptureStep::basic("dcmv", "Set meter to DC mV").expect(Expect::mode("DC mV")),
            ),
            on(
                &[Ut803],
                CaptureStep::basic("acmv", "Set meter to AC mV (if the meter has it)")
                    .expect(Expect::mode("AC mV")),
            ),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "hz",
                    "Set meter to Frequency (Hz): the dial position marked Hz, \
                     then the blue button until Hz shows",
                )
                .expect(Expect::mode("Frequency")),
            ),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "duty",
                    "Frequency mode: press the blue button until % shows (duty cycle)",
                )
                .expect(Expect::mode("Duty %")),
            ),
            on(
                ALL,
                CaptureStep::basic("ohm", "Set meter to Resistance (Ω)").expect(Expect::mode("Ω")),
            ),
            on(ALL, ohm_ol),
            on(ALL, ohm_body),
            on(ALL, ohm_short),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "cont",
                    "Resistance position: press the blue button for Continuity",
                )
                .expect(Expect::mode("Continuity")),
            ),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "diode",
                    "Resistance position: press the blue button again for Diode",
                )
                .expect(Expect::mode("Diode")),
            ),
            // Function code E (docs/research/ut71/reverse-engineered-protocol.md
            // §3.2). The UT71E's and VC940's power adapter plugs into the
            // input jacks and into a mains outlet, and the load into it (UT71
            // manual, "Power Measurement"; VC920/940/960 manual §9 l).
            on(
                HANDHELD,
                CaptureStep::basic(
                    "power",
                    "Set meter to W (power, if the meter has it): its adapter into a live \
                     mains outlet, and a load into the adapter",
                )
                .needs(&[Need::PowerAdapter])
                .expect(Expect::mode("Power")),
            ),
            on(
                ALL,
                CaptureStep::basic("cap", "Set meter to Capacitance")
                    .expect(Expect::mode("Capacitance")),
            ),
            // Both models name the frequency mode "Frequency", not "Hz".
            on(
                BENCH,
                CaptureStep::basic("hz", "Set meter to Frequency (Hz)")
                    .expect(Expect::mode("Frequency")),
            ),
            on(
                BENCH,
                named(
                    CaptureStep::basic(
                        "duty",
                        "Frequency mode: switch the display to Duty Cycle (%)",
                    ),
                    for_model(Some("Duty %"), None),
                ),
            ),
            // Frequency with the alt bit set (spec §5).
            on(
                &[Ut803],
                CaptureStep::basic("rpm", "Set meter to Tachometer / RPM")
                    .expect(Expect::mode("Tachometer")),
            ),
            on(
                BENCH,
                CaptureStep::basic("diode", "Set meter to Diode").expect(Expect::mode("Diode")),
            ),
            // #16's second run verifies this one on the UT804: in the first,
            // the reporter pressed SELECT on to Diode and Ω before the
            // samples were in.
            on(
                BENCH,
                CaptureStep::basic("cont", "Set meter to Continuity")
                    .expect(Expect::mode("Continuity")),
            ),
            on(
                ALL,
                CaptureStep::basic(
                    "temp",
                    say(
                        "Set meter to temperature (K-type thermocouple, if available)",
                        "Set meter to temperature °C (if the meter has it; K-type \
                         thermocouple, if available)",
                    ),
                )
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("Temperature")),
            ),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "temp_f",
                    "Temperature: press the blue button for °F (if the meter has it; K-type \
                     thermocouple, if available)",
                )
                .needs(&[Need::Thermocouple])
                .expect(Expect::mode("Temperature")),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic("dcua", "Set meter to DC µA"),
                    for_model(Some("DC µA"), Some("µA")),
                ),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic(
                        "acua",
                        say(
                            "Set meter to AC µA",
                            "µA position: press the blue button for AC µA",
                        ),
                    ),
                    for_model(Some("AC µA"), Some("µA")),
                ),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic("dcma", "Set meter to DC mA"),
                    for_model(Some("DC mA"), Some("mA")),
                ),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic(
                        "acma",
                        say(
                            "Set meter to AC mA",
                            "mA position: press the blue button for AC mA",
                        ),
                    ),
                    for_model(Some("AC mA"), Some("mA")),
                ),
            ),
            // The 4-20 mA loop reading in %, one more blue press on the mA
            // position. With nothing in the loop it reads LO, as the UT804's
            // did in #16, so no loop source is asked for.
            on(
                HANDHELD,
                CaptureStep::basic(
                    "ma_percent",
                    "mA position: press the blue button until % shows (4-20 mA loop, \
                     if the meter has it)",
                )
                .expect(Expect::mode("mA%")),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic("dca", "Set meter to DC A"),
                    for_model(Some("DC A"), Some("A")),
                ),
            ),
            on(
                ALL,
                named(
                    CaptureStep::basic(
                        "aca",
                        say(
                            "Set meter to AC A",
                            "A position: press the blue button for AC A",
                        ),
                    ),
                    for_model(Some("AC A"), Some("A")),
                ),
            ),
            // Mode 14 "ADP / Logic" is named by the vendor binaries alone
            // (spec §3.4). Mode 15 is the mA position's 4-20 mA loop reading
            // in %, whose mode keeps the vendor's name "mA%".
            on(
                &[Ut803],
                CaptureStep::basic("adp", "Set meter to ADP / logic").expect(Expect::mode("ADP")),
            ),
            on(
                BENCH,
                named(
                    CaptureStep::basic("ma_percent", "Set meter to % (4-20 mA loop)"),
                    for_model(Some("mA%"), None),
                ),
            ),
            // RANGE sets status bit 1 (Manual), MAX MIN puts nothing on the
            // wire, and REL sends the relative reading with bit 1 set (spec
            // §3.6, #16), so the steps assert nothing. MAX MIN works on a
            // manual range only (UT804 manual and UT71 manual, "Using MAX MIN"),
            // so it follows RANGE. EXIT leaves all three, and turns the
            // meter's data output off (#16), so every step that presses it
            // asks for SEND after it. Each step waits for Enter: every press,
            // and the previous step's EXIT and SEND, leaves a state the meter
            // reports, and the watcher would capture the first of them. The
            // handhelds get a step of their own to return to AUTO.
            on(
                BUTTONS,
                CaptureStep::basic("manual_range", "DC V: press RANGE, then Enter.")
                    .wait_for_enter(),
            ),
            on(
                BUTTONS,
                CaptureStep::basic(
                    "max_min",
                    say(
                        "DC V with AUTO off: press MAX MIN, then Enter. \
                         Press EXIT, then SEND, afterwards.",
                        "DC V, still on the manual range: press MAX MIN, then Enter.",
                    ),
                )
                .wait_for_enter(),
            ),
            on(
                HANDHELD,
                CaptureStep::basic(
                    "auto_range",
                    "DC V: press EXIT until AUTO shows and turn SEND on again, then Enter.",
                )
                .wait_for_enter()
                .expect(Expect::new().range(RangeExpect::Auto)),
            ),
            on(
                BUTTONS,
                CaptureStep::basic(
                    "rel",
                    say(
                        "DC V: press REL, then Enter. Press EXIT, then SEND, afterwards.",
                        "DC V: press REL, then Enter. Press EXIT, then turn SEND on again, \
                         afterwards.",
                    ),
                )
                .wait_for_enter(),
            ),
            // The HOLD wire encoding is what this step is for, so it asserts
            // nothing about the flag. The UT804 sends nothing while HOLD is
            // on (#16), which the step reports as "No response from meter."
            // only after its sampling reads time out, so its instruction says
            // that is a valid result, that it can take up to a minute, and
            // how to leave HOLD for the rest of the run: EXIT turns the data
            // output off as well, and SEND turns it back on. That line is the
            // result the reporter confirmed on the UT804; the handhelds, whose
            // HOLD nobody has run, get the same. There the step waits for
            // Enter, as REL's clean-up before it leaves a state of its own.
            on(
                ALL,
                match model {
                    Ut803 => CaptureStep::basic(
                        "hold",
                        "Press HOLD (wire encoding unknown — capture needed)",
                    ),
                    Ut804 => CaptureStep::basic(
                        "hold",
                        "Press HOLD on the meter, then Enter. If the meter stops sending, \
                         \"No response from meter.\" shows within a minute: that is a valid \
                         result. Press EXIT, then SEND, on the meter afterwards.",
                    )
                    .wait_for_enter(),
                    Ut71Ab | Ut71Cde | Vc920 => CaptureStep::basic(
                        "hold",
                        "Press HOLD on the meter, then Enter. If the meter stops sending, \
                         \"No response from meter.\" shows within a minute: that is a valid \
                         result. Press EXIT, then turn SEND on again, afterwards.",
                    )
                    .wait_for_enter(),
                },
            ),
            // Holding the blue button at power-on drops every function to
            // 4000 counts until the next power cycle (UT71 manual Table 2-2;
            // VC920/940/960 manual §9), which blanks digit 5 (ut71 spec §3.1).
            // Powering on leaves SEND off.
            on(
                HANDHELD,
                CaptureStep::basic(
                    "fast_mode",
                    "Turn the meter off, then back on at DC V while holding the blue button \
                     (4000 counts); turn SEND on again, then Enter.",
                )
                .wait_for_enter()
                .expect(Expect::mode("DC V")),
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// Detection for the meters on this packet stream.
///
/// The meters stream 11-byte CR LF packets and take no commands past the
/// CH9325 transport's own init, so there is nothing to send
/// (`docs/research/ut803/reverse-engineered-protocol.md`). A UT71 or VC9x0
/// packet is the UT804's, so those meters are claimed as a UT804 and have to
/// be named for their own range labels
/// (`docs/research/ut71/reverse-engineered-protocol.md` §4).
pub(crate) static FINGERPRINT: Fingerprint = Fingerprint {
    family: DeviceFamily::Ut80x,
    label: "ut80x stream",
    trigger: None,
    send_after: &[],
    checksummed: false,
    recognise,
};

fn recognise(buf: &[u8], _probing: &Probing) -> Option<Evidence> {
    // A packet does not say which model sent it. The CH9325 starts at 2400
    // baud, where the UT804 is heard: the UT803 talks at 19200 (spec §1.2,
    // §5), so it is not detected and has to be named. The UT71 and VC9x0
    // send the UT804's packets at 2400 (ut71 spec §1, §2), so they are
    // claimed as a UT804, and have to be named too.
    extract_packet(buf).ok().flatten().map(|_| Evidence::Model {
        id: "ut804",
        reported_name: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::test_support::snapshot;
    use crate::transport::mock::MockTransport;

    // Real packets from a UT804 on its bundled CH9325 cable at 2400 baud
    // (issue #16), all on DC V range 1 with AUTO on.

    /// Open leads: digits 00000, range 1, mode 1, coupling 0, status 1.
    const ISSUE16_OPEN_LEADS: [u8; 11] = [
        0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0x31, 0x31, 0xB0, 0x31, 0x0D, 0x8A,
    ];
    /// Digits 00013 with status 5 (sign and AUTO).
    const ISSUE16_MINUS_0_0013: [u8; 11] = [
        0xB0, 0xB0, 0xB0, 0x31, 0xB3, 0x31, 0x31, 0xB0, 0xB5, 0x0D, 0x8A,
    ];
    /// Digits 00000 with the sign bit set.
    const ISSUE16_SIGNED_ZERO: [u8; 11] = [
        0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0x31, 0x31, 0xB0, 0xB5, 0x0D, 0x8A,
    ];
    /// Digits 00008 with status 5.
    const ISSUE16_MINUS_0_0008: [u8; 11] = [
        0xB0, 0xB0, 0xB0, 0xB0, 0x38, 0x31, 0x31, 0xB0, 0xB5, 0x0D, 0x8A,
    ];

    /// A packet as the UT804 sends it: each data nibble as `0x30 | n`, bit 7
    /// set for odd parity, then CR LF as `0D 8A`.
    fn wire(data: [u8; 9]) -> Vec<u8> {
        let mut packet: Vec<u8> = data
            .iter()
            .map(|&n| {
                let byte = 0x30 | n;
                if byte.count_ones() % 2 == 0 {
                    byte | 0x80
                } else {
                    byte
                }
            })
            .collect();
        packet.extend_from_slice(&[0x0D, 0x8A]);
        packet
    }

    /// Reports as the CH9325 delivers them: one byte each.
    fn one_byte_per_report(bytes: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
        bytes.iter().map(|&b| vec![b])
    }

    #[test]
    fn wire_builds_the_bytes_the_meter_sends() {
        assert_eq!(wire([0, 0, 0, 0, 0, 1, 1, 0, 1]), ISSUE16_OPEN_LEADS);
        assert_eq!(wire([0, 0, 0, 1, 3, 1, 1, 0, 5]), ISSUE16_MINUS_0_0013);
        assert_eq!(wire([0, 0, 0, 0, 8, 1, 1, 0, 5]), ISSUE16_MINUS_0_0008);
    }

    // --- Packet extraction ---

    #[test]
    fn a_whole_packet_comes_back_as_received() {
        let (packet, consumed) = extract_packet(&ISSUE16_OPEN_LEADS).unwrap().unwrap();
        assert_eq!(packet, ISSUE16_OPEN_LEADS);
        assert_eq!(consumed, PACKET_LEN);
    }

    /// Joining the stream mid-packet: the tail is not a packet, because the
    /// CR and LF sit where data bytes should be.
    #[test]
    fn a_partial_first_packet_is_skipped() {
        let mut buf = ISSUE16_OPEN_LEADS[4..].to_vec();
        buf.extend_from_slice(&ISSUE16_MINUS_0_0013);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, ISSUE16_MINUS_0_0013);
        assert_eq!(consumed, buf.len());
    }

    /// A packet that lost a data byte: the 11 bytes ending at its LF start
    /// at the previous packet's LF, which is not a data byte.
    #[test]
    fn a_packet_that_lost_a_byte_is_skipped() {
        let mut short = ISSUE16_MINUS_0_0013.to_vec();
        short.remove(4);
        let mut buf = vec![0x0D, 0x8A];
        buf.extend_from_slice(&short);
        assert_eq!(extract_packet(&buf).unwrap(), None);

        buf.extend_from_slice(&ISSUE16_MINUS_0_0008);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, ISSUE16_MINUS_0_0008);
        assert_eq!(consumed, buf.len());
    }

    #[test]
    fn a_packet_is_not_whole_until_its_lf_arrives() {
        assert_eq!(extract_packet(&ISSUE16_OPEN_LEADS[..10]).unwrap(), None);
        assert_eq!(extract_packet(&[]).unwrap(), None);
    }

    /// The parity bit is masked, not checked: a parity-stripped stream and a
    /// CR with bit 7 set are packets too, and read the same.
    #[test]
    fn bit_7_is_ignored() {
        let stripped: Vec<u8> = ISSUE16_MINUS_0_0013.iter().map(|b| b & 0x7F).collect();
        let mut cr_high = ISSUE16_MINUS_0_0013;
        cr_high[9] = 0x8D;
        for packet in [stripped.as_slice(), &cr_high] {
            let (found, _) = extract_packet(packet).unwrap().unwrap();
            assert_eq!(found, packet);
            let m = parse_measurement_ut804(packet).unwrap();
            assert_eq!(m.display_raw.as_deref(), Some("-0.0013"));
        }
    }

    #[test]
    fn a_data_byte_outside_0x30_to_0x3f_breaks_the_packet() {
        for bad in [0x00, 0x2F, 0x40, 0xC0] {
            let mut packet = ISSUE16_OPEN_LEADS;
            packet[3] = bad;
            assert_eq!(extract_packet(&packet).unwrap(), None, "{bad:#04x}");
        }
    }

    /// `parse_payload` feeds golden fixtures straight to the parsers, which
    /// take nothing but a whole packet.
    #[test]
    fn parse_payload_refuses_what_is_not_a_packet() {
        let fs9721_frame: Vec<u8> = (1..=14u8).map(|i| i << 4).collect();
        let mut no_lf = ISSUE16_OPEN_LEADS;
        no_lf[10] = 0xB0;
        let not_packets: [&[u8]; 4] =
            [&[0u8; 12], &fs9721_frame, &no_lf, &ISSUE16_OPEN_LEADS[..10]];
        for proto in [
            Box::new(Ut80xProtocol::new_ut803()) as Box<dyn Protocol>,
            Box::new(Ut80xProtocol::new_ut804()),
            Box::new(Ut80xProtocol::new_ut71ab()),
            Box::new(Ut80xProtocol::new_ut71cde()),
            Box::new(Ut80xProtocol::new_vc920()),
        ] {
            let name = proto.profile().model_name;
            for bytes in not_packets {
                assert!(proto.parse_payload(bytes).is_err(), "{name}: {bytes:02X?}");
            }
            assert!(proto.parse_payload(&ISSUE16_OPEN_LEADS).is_ok(), "{name}");
        }
    }

    /// Issue #16's cable delivers one byte per report and empty reports
    /// between packets; a read that joins mid-packet waits for the next
    /// whole one and keeps its wire bytes as the payload.
    #[test]
    fn request_measurement_reads_a_packet_a_byte_at_a_time() {
        let reports: Vec<Vec<u8>> = one_byte_per_report(&ISSUE16_OPEN_LEADS[6..])
            .chain(std::iter::repeat_n(Vec::new(), 50))
            .chain(one_byte_per_report(&ISSUE16_MINUS_0_0013))
            .chain(std::iter::repeat_n(Vec::new(), 50))
            .chain(one_byte_per_report(&ISSUE16_MINUS_0_0008))
            .collect();
        let mock = MockTransport::new(reports);
        let mut proto = Ut80xProtocol::new_ut804();

        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("-0.0013"));
        assert_eq!(m.raw_payload, ISSUE16_MINUS_0_0013);
        let m = proto.request_measurement(&mock).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("-0.0008"));
        assert!(matches!(
            proto.request_measurement(&mock),
            Err(Error::Timeout)
        ));
    }

    /// The UT803 talks at 19200, which the CH9325's start-up at 2400 never
    /// leaves while the bridge keeps reporting; the UT804, the UT71 and the
    /// VC9x0 stay at 2400, their start-up untouched.
    #[test]
    fn only_the_ut803_init_changes_the_bridge_rate() {
        let mock = MockTransport::new(Vec::new());
        for mut proto in [
            Ut80xProtocol::new_ut804(),
            Ut80xProtocol::new_ut71ab(),
            Ut80xProtocol::new_ut71cde(),
            Ut80xProtocol::new_vc920(),
        ] {
            proto.init(&mock).unwrap();
            let name = proto.profile().model_name;
            assert!(mock.feature_reports.borrow().is_empty(), "{name}");
            assert!(mock.written.borrow().is_empty(), "{name}");
        }

        Ut80xProtocol::new_ut803().init(&mock).unwrap();
        assert_eq!(
            *mock.feature_reports.borrow(),
            [ch9325::FALLBACK_FEATURE_REPORT.to_vec()]
        );
        assert!(mock.written.borrow().is_empty());
    }

    /// A step may only ask for a label its own model's parser can report:
    /// the UT803 names its current modes by unit alone, so asking it for
    /// "DC µA" leaves the step waiting for a state that never arrives.
    #[test]
    fn each_model_is_asked_for_the_labels_its_parser_reports() {
        let expected = |proto: &dyn Protocol, id: &str| -> Option<&'static str> {
            proto
                .capture_steps()
                .into_iter()
                .find(|s| s.id == id)
                .expect("the step list has the step")
                .expect
                .and_then(|e| e.mode)
        };

        let ut803 = Ut80xProtocol::new_ut803();
        assert_eq!(expected(&ut803, "dcua"), Some("µA"));
        assert_eq!(expected(&ut803, "acma"), Some("mA"));
        assert_eq!(expected(&ut803, "dca"), Some("A"));
        assert_eq!(expected(&ut803, "rpm"), Some("Tachometer"));
        // Modes the UT803 parser has no label for at all.
        for id in ["duty", "ma_percent", "acdcv"] {
            assert_eq!(expected(&ut803, id), None, "{id} asserted on the UT803");
        }

        let ut804 = Ut80xProtocol::new_ut804();
        assert_eq!(expected(&ut804, "dcua"), Some("DC µA"));
        assert_eq!(expected(&ut804, "duty"), Some("Duty %"));
        assert_eq!(expected(&ut804, "acdcv"), Some("AC+DC V"));
    }

    /// The UT804 has no tachometer, AC mV or ADP position (its manual's
    /// Table 2-1), and the reporter walking it in #16 found none: its list
    /// asks for none of them, and adds RANGE, MAX MIN and REL before HOLD.
    #[test]
    fn each_model_is_asked_only_for_what_it_has() {
        let ids = |proto: Ut80xProtocol| -> Vec<&'static str> {
            proto.capture_steps().iter().map(|s| s.id).collect()
        };

        let ut804 = ids(Ut80xProtocol::new_ut804());
        for id in ["rpm", "acmv", "adp"] {
            assert!(!ut804.contains(&id), "the UT804 is asked for {id}");
        }
        let hold = ut804.iter().position(|&id| id == "hold");
        for id in ["manual_range", "max_min", "rel"] {
            let at = ut804.iter().position(|&s| s == id);
            assert!(at.is_some() && at < hold, "{id} before hold: {ut804:?}");
        }

        // Button sequences on the UT804 capture on Enter alone (#16: REL
        // captured the state MAX MIN's clean-up left). Everything else still
        // captures on its own.
        let waits = |proto: Ut80xProtocol| -> Vec<&'static str> {
            let steps = proto.capture_steps();
            steps
                .iter()
                .filter(|s| s.wait_for_enter)
                .map(|s| s.id)
                .collect()
        };
        assert_eq!(
            waits(Ut80xProtocol::new_ut804()),
            ["manual_range", "max_min", "rel", "hold"]
        );
        assert!(waits(Ut80xProtocol::new_ut803()).is_empty());

        // #16's reporter walked every UT804 step; nobody has run a UT803.
        let unverified = |proto: Ut80xProtocol| -> Vec<&'static str> {
            let steps = proto.capture_steps();
            steps.iter().filter(|s| !s.verified).map(|s| s.id).collect()
        };
        assert!(unverified(Ut80xProtocol::new_ut804()).is_empty());
        assert_eq!(
            unverified(Ut80xProtocol::new_ut803()),
            ids(Ut80xProtocol::new_ut803())
        );

        assert_eq!(
            ids(Ut80xProtocol::new_ut803()),
            [
                "dcv",
                "dcv_short",
                "dcv_negative",
                "acv",
                "acdcv",
                "dcmv",
                "acmv",
                "ohm",
                "ohm_ol",
                "ohm_body",
                "ohm_short",
                "cap",
                "hz",
                "duty",
                "rpm",
                "diode",
                "cont",
                "temp",
                "dcua",
                "acua",
                "dcma",
                "acma",
                "dca",
                "aca",
                "adp",
                "ma_percent",
                "hold",
            ]
        );
    }

    // --- UT71 and VC9x0 --------------------------------------------------

    /// The UT804-layout models that are not the UT804.
    const HANDHELDS: [Model; 3] = [Model::Ut71Ab, Model::Ut71Cde, Model::Vc920];

    /// The handhelds' protocols, in [`HANDHELDS`] order.
    fn handheld_protocols() -> [Ut80xProtocol; 3] {
        [
            Ut80xProtocol::new_ut71ab(),
            Ut80xProtocol::new_ut71cde(),
            Ut80xProtocol::new_vc920(),
        ]
    }

    /// The UT71 and VC9x0 walk their own dials: the gate, the UT804's list
    /// reworded for their blue and yellow buttons, W and °F, and a step back
    /// to AUTO after MAX MIN, which works on a manual range only. Nobody has
    /// run one, so nothing is verified.
    #[test]
    fn the_handhelds_walk_their_own_list() {
        let expected = [
            "dcv",
            "dcv_short",
            "dcv_negative",
            "acv",
            "acdcv",
            "dcmv",
            "hz",
            "duty",
            "ohm",
            "ohm_ol",
            "ohm_body",
            "ohm_short",
            "cont",
            "diode",
            "power",
            "cap",
            "temp",
            "temp_f",
            "dcua",
            "acua",
            "dcma",
            "acma",
            "ma_percent",
            "dca",
            "aca",
            "manual_range",
            "max_min",
            "auto_range",
            "rel",
            "hold",
            "fast_mode",
        ];
        for proto in handheld_protocols() {
            let name = proto.profile().model_name;
            let steps = proto.capture_steps();
            let ids: Vec<&str> = steps.iter().map(|s| s.id).collect();
            assert_eq!(ids, expected, "{name}");
            assert!(steps.iter().all(|s| !s.verified), "{name}");
            let waits: Vec<&str> = steps
                .iter()
                .filter(|s| s.wait_for_enter)
                .map(|s| s.id)
                .collect();
            assert_eq!(
                waits,
                [
                    "manual_range",
                    "max_min",
                    "auto_range",
                    "rel",
                    "hold",
                    "fast_mode"
                ],
                "{name}"
            );
            // EXIT turns SEND off (UT71 manual Table 2-2).
            for step in steps.iter().filter(|s| s.instruction.contains("EXIT")) {
                assert!(
                    step.instruction.contains("turn SEND on again"),
                    "{name} {}: {:?}",
                    step.id,
                    step.instruction
                );
            }
        }
    }

    /// Every mode a handheld step asserts is one its parser reports, or the
    /// step waits for a state that never arrives.
    #[test]
    fn the_handhelds_are_asked_for_labels_their_parser_reports() {
        for (model, proto) in HANDHELDS.into_iter().zip(handheld_protocols()) {
            let modes: std::collections::HashSet<String> = ut804_layout_accepted_readings(model)
                .into_iter()
                .map(|(_, m)| m.mode.into_owned())
                .collect();
            for step in proto.capture_steps() {
                if let Some(mode) = step.expect.and_then(|e| e.mode) {
                    assert!(modes.contains(mode), "{model:?} {}: {mode}", step.id);
                }
            }
        }
    }

    /// The handhelds send the UT804's packets, which their vendor apps read
    /// with the UT804 app's parser (ut71 spec §2, §3): every reading the
    /// UT804 accepts reads the same on them, range label aside. The UT71C/D/E
    /// keeps the UT804's labels and the VC9x0 all but AC V's top one; the
    /// UT71A/B's have a test of their own.
    #[test]
    fn the_handhelds_read_every_ut804_reading_the_same() {
        for (p, ut804) in ut804_accepted_readings() {
            let f = Ut804Fields::decode(&p).unwrap();
            for model in HANDHELDS {
                let (m, reports) =
                    crate::protocol::capture_reports(|| parse_ut804_layout(model, &p));
                let m = m.unwrap();
                assert!(reports.is_empty(), "{model:?} {p:02X?}: {reports:?}");
                assert_eq!(m.raw_payload, ut804.raw_payload);
                let label = match model {
                    Model::Vc920 if (f.mode, f.range) == (0x2, 4) => Some("750V"),
                    Model::Ut71Cde | Model::Vc920 => Some(ut804.range_label.as_ref()),
                    Model::Ut71Ab | Model::Ut803 | Model::Ut804 => None,
                };
                if let Some(label) = label {
                    assert_eq!(m.range_label, label, "{model:?} {p:02X?}");
                }
                let relabelled = Measurement {
                    range_label: ut804.range_label.clone(),
                    ..m
                };
                assert_eq!(
                    snapshot(&relabelled),
                    snapshot(&ut804),
                    "{model:?} {p:02X?}"
                );
            }
        }
    }

    /// The UT71A/B counts to 20000 on the UT804's codes and decimal points
    /// (ut71 spec §3.5): each label is half the UT804's full scale, apart
    /// from the 1000V and 10A tops, and continuity and diode, which neither
    /// the A/B vendor app nor the UT71 manual gives a range.
    #[test]
    fn ut71ab_range_labels_are_half_the_ut804s() {
        for (p, ut804) in ut804_accepted_readings() {
            let f = Ut804Fields::decode(&p).unwrap();
            let ab = parse_ut804_layout(Model::Ut71Ab, &p).unwrap();
            let label = ab.range_label.as_ref();
            let context = format!(
                "{} {p:02X?}: {label:?} for {:?}",
                ut804.mode, ut804.range_label
            );
            match (f.mode, f.range) {
                _ if ut804.range_label.is_empty() => assert_eq!(label, "", "{context}"),
                (0xA | 0xB, _) => assert_eq!(label, "", "{context}"),
                (0x1 | 0x2, 4) | (0x9, _) => assert_eq!(label, ut804.range_label, "{context}"),
                _ => {
                    let (full, unit) = si_quantity(&ut804.range_label).unwrap();
                    let (half, ab_unit) = si_quantity(label).expect(&context);
                    assert_eq!(ab_unit, unit, "{context}");
                    assert!((half - full / 2.0).abs() <= 1e-9 * full, "{context}");
                }
            }
        }
        // As the A/B app's chart and the manual's A/B tables give them.
        for (mode, range, acdc, label) in [
            (0x1, 1, 2, "2V"),
            (0x2, 4, 1, "1000V"),
            (0x3, 0, 0, "200mV"),
            (0x4, 1, 0, "200Ω"),
            (0x4, 6, 0, "20MΩ"),
            (0x5, 1, 0, "20nF"),
            (0x5, 7, 0, "20mF"),
            (0x7, 1, 0, "2000µA"),
            (0x8, 0, 1, "20mA"),
            (0x9, 0, 0, "10A"),
            (0xC, 0, 0, "20Hz"),
            (0xC, 7, 0, "200MHz"),
            (0xA, 0, 0, ""),
            (0xB, 0, 0, ""),
            (0x6, 0, 0, ""),
            (0xE, 0, 0, ""),
            (0xF, 0, 0, ""),
        ] {
            let p = ut804_payload(&[0, 1, 2, 3, 0xA], range, mode, acdc, 0x0);
            let m = parse_ut804_layout(Model::Ut71Ab, &p).unwrap();
            assert_eq!(m.range_label, label, "mode {mode:#x} range {range}");
        }
    }

    /// The Voltcraft manuals stop AC V at 750V; DC V, and every UT71's AC V,
    /// stop at 1000V (ut71 spec §3.5).
    #[test]
    fn only_the_vc920s_ac_v_tops_out_at_750v() {
        let label = |model, range, mode, acdc| {
            let p = ut804_payload(&[0, 2, 3, 0, 0], range, mode, acdc, 0x0);
            parse_ut804_layout(model, &p).unwrap().range_label
        };
        assert_eq!(label(Model::Vc920, 4, 0x2, 1), "750V");
        assert_eq!(label(Model::Vc920, 4, 0x2, 3), "750V");
        assert_eq!(label(Model::Vc920, 3, 0x2, 1), "400V");
        assert_eq!(label(Model::Vc920, 4, 0x1, 2), "1000V");
        for model in [Model::Ut71Ab, Model::Ut71Cde] {
            assert_eq!(label(model, 4, 0x2, 1), "1000V", "{model:?}");
        }
    }

    /// Function code E is the UT71E's and VC940's power position, in watts
    /// with the point after digit 4 (ut71 spec §3.2), and reads without a
    /// report on every UT804-layout model. Function 0 has no position in any
    /// of their manuals (§3.2, §6), so it is still reported, under each
    /// model's own id.
    #[test]
    fn code_e_is_power_and_code_0_is_still_reported() {
        let power = ut804_payload(&[0, 1, 2, 3, 4], 0, 0xE, 0, 0x0);
        let code_0 = ut804_payload(&[1, 2, 3, 4, 0xA], 0, 0x0, 0, 0x0);
        for proto in [
            Ut80xProtocol::new_ut804(),
            Ut80xProtocol::new_ut71ab(),
            Ut80xProtocol::new_ut71cde(),
            Ut80xProtocol::new_vc920(),
        ] {
            let id = proto.model.report_id();
            let (m, reports) = crate::protocol::capture_reports(|| proto.parse_payload(&power));
            let m = m.unwrap();
            assert_eq!(
                (m.mode.as_ref(), m.unit.as_ref(), m.display_raw.as_deref()),
                ("Power", "W", Some("0123.4")),
                "{id}"
            );
            assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 123.4).abs() < 1e-9));
            assert!(reports.is_empty(), "{id}: {reports:?}");

            let (m, reports) = crate::protocol::capture_reports(|| proto.parse_payload(&code_0));
            assert_eq!(m.unwrap().mode, "Unknown(0x00)", "{id}");
            assert_eq!(
                reports,
                [format!(
                    "{id}: unrecognised mode/range pair: nibbles 1234A0000"
                )]
            );
        }
    }

    /// A report's hint reruns `--device <id>`, so each model reports under
    /// its own registry id: a VC920 owner is not sent to the UT804's parser.
    #[test]
    fn each_model_reports_under_its_registry_id() {
        use crate::protocol::registry;
        // A digit nibble past 0xA among either layout's digits, on a UT804
        // mode that is not a UT803 overload.
        let p = wire([1, 0xF, 0xF, 0, 0xA, 1, 8, 2, 0]);
        let devices: Vec<_> = registry::DEVICES
            .iter()
            .filter(|d| d.family == DeviceFamily::Ut80x)
            .collect();
        assert_eq!(devices.len(), 5);
        for device in devices {
            let proto = (device.new_protocol)();
            let (_, reports) = crate::protocol::capture_reports(|| proto.parse_payload(&p));
            let prefix = format!("{}: ", device.id);
            assert!(
                !reports.is_empty() && reports.iter().all(|r| r.starts_with(&prefix)),
                "{}: {reports:?}",
                device.id
            );
        }
        let (_, reports) =
            crate::protocol::capture_reports(|| Ut80xProtocol::new_vc920().parse_payload(&p));
        assert_eq!(
            reports,
            ["vc920: unrecognised digit nibble: nibbles 1FF0A1820"]
        );
    }

    /// Build a UT804 packet.
    /// digits = MSD-first positions 1-5; then range, mode, acdc, status at
    /// positions 6-9.
    fn ut804_payload(digits: &[u8; 5], range: u8, mode: u8, acdc: u8, status: u8) -> Vec<u8> {
        wire([
            digits[0], digits[1], digits[2], digits[3], digits[4], range, mode, acdc, status,
        ])
    }

    /// Build a UT803 packet.
    /// Position k is byte k-1: range at position 2 (byte 1), digits =
    /// MSD-first positions 3-6 (bytes 2-5), mode at 7 (byte 6), nib8/nib9/
    /// nib10 at positions 8/9/10 (bytes 7-9).
    fn ut803_payload(
        digits: &[u8; 4],
        range: u8,
        mode: u8,
        nib8: u8,
        nib9: u8,
        nib10: u8,
    ) -> Vec<u8> {
        wire([
            range, digits[0], digits[1], digits[2], digits[3], mode, nib8, nib9, nib10,
        ])
    }

    // --- UT804 ---

    #[test]
    fn ut804_dcv_range1() {
        // DC V range 1: 3.999 full scale → decimal after digit 1.
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 1, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 3.999).abs() < 1e-9));
        assert_eq!(m.display_raw.as_deref(), Some("3.999"));
    }

    #[test]
    fn ut804_negative_sign_nibble9_bit2() {
        // Status nibble bit 2 = negative sign (NOT hold).
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(!m.flags.hold);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-9));
        assert_eq!(m.display_raw.as_deref(), Some("-12.34"));
    }

    #[test]
    fn ut804_auto_flag() {
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x1);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(m.flags.auto_range);
        // Sign bit set alongside: AUTO still derived after stripping.
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x5);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(m.flags.auto_range);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v < 0.0));
    }

    #[test]
    fn ut804_overload_nibble1() {
        // Digit1 = 0xA + any digit2 but 0xC → overload.
        let p = ut804_payload(&[0xA, 0x1, 0, 0, 0], 1, 0x4, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));

        // Negative overload via status bit 2.
        let p = ut804_payload(&[0xA, 0x1, 0, 0, 0], 1, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw.as_deref(), Some("-0L"));
    }

    #[test]
    fn ut804_resistance_kilo_range() {
        // Ω range 2 = 39.99 kΩ? No: range 2 → kΩ with point after digit 1
        // (3.999 kΩ style).
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 2, 0x4, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "kΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 3.999).abs() < 1e-9));
    }

    #[test]
    fn ut804_frequency_and_duty() {
        // Mode 0xC range 2 = kHz, point after digit 1.
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0xC, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Frequency");
        assert_eq!(m.unit, "kHz");
        // Sign bit in frequency mode = duty-cycle display, not negative.
        let p = ut804_payload(&[5, 0, 0, 0, 0xA], 2, 0xC, 0, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Duty %");
        assert_eq!(m.unit, "%");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v > 0.0));
    }

    #[test]
    fn ut804_temperature_modes() {
        let p = ut804_payload(&[0, 0, 2, 5, 0xA], 0, 0x6, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "Temperature");
        assert_eq!(m.unit, "°C");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 25.0).abs() < 1e-9));
        let p = ut804_payload(&[0, 0, 7, 7, 0xA], 0, 0xD, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.unit, "°F");
    }

    #[test]
    fn ut804_acv_label_from_acdc_nibble() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 1, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "AC V");
        assert!(!m.flags.dc);
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 3, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "AC+DC V");
    }

    #[test]
    fn ut804_current_modes() {
        // Mode 7 = µA (not Hz as the old table claimed).
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 0, 0x7, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC µA");
        assert_eq!(m.unit, "µA");
        // Mode 9 = A.
        let p = ut804_payload(&[1, 0, 0, 0, 0xA], 0, 0x9, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(m.mode, "DC A");
        assert_eq!(m.unit, "A");
    }

    /// The labels are the manual's full ranges: the top volts range is 1000V
    /// on AC and AC+DC too, AC mA has DC mA's ranges, and temperature, duty
    /// and the 4-20 mA % have none.
    #[test]
    fn ut804_range_labels_follow_the_manual() {
        for (range, mode, acdc, status, label) in [
            (4, 0x1, 0, 0x0, "1000V"),
            (4, 0x1, 2, 0x0, "1000V"),
            (4, 0x2, 1, 0x0, "1000V"),
            (4, 0x2, 3, 0x0, "1000V"),
            (0, 0x8, 1, 0x1, "40mA"),
            (1, 0x8, 1, 0x1, "400mA"),
            (0, 0xA, 0, 0x0, "400Ω"),
            (0, 0xB, 0, 0x0, "4V"),
            (7, 0xC, 0, 0x1, "400MHz"),
            (0, 0x6, 0, 0x0, ""),
            (0, 0xC, 0, 0x5, ""),
            (0, 0xF, 0, 0x0, ""),
        ] {
            let p = ut804_payload(&[0, 1, 2, 3, 0xA], range, mode, acdc, status);
            let m = parse_measurement_ut804(&p).unwrap();
            assert_eq!(m.range_label, label, "mode {mode:#x} range {range}");
        }
    }

    #[test]
    fn ut804_idle_frame() {
        // Digit 4 == 0xB → idle, all displays zero.
        let p = ut804_payload(&[0, 0, 0, 0xB, 0], 1, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if v == 0.0));
    }

    #[test]
    fn ut804_packet_without_cr_rejected() {
        let mut p = ut804_payload(&[1, 2, 3, 4, 0xA], 1, 0x1, 2, 0x0);
        p[9] = 0x0;
        assert!(parse_measurement_ut804(&p).is_err());
    }

    #[test]
    fn ut804_five_digit_reading() {
        // All five digits present (no blank): the 40000-count display.
        let p = ut804_payload(&[1, 2, 3, 4, 5], 2, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 12.345).abs() < 1e-9));
    }

    /// Issue #16, open leads: an AUTO DC V reading of zero.
    #[test]
    fn ut804_snapshot_issue16_open_leads() {
        let m = parse_measurement_ut804(&ISSUE16_OPEN_LEADS).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Normal(0.0)
unit=V
range_label=4V
display_raw=Some("0.0000")
flags=auto_range,dc
aux=0
raw_payload=11"#
        );
    }

    /// Issue #16: the status nibble's sign bit alongside AUTO.
    #[test]
    fn ut804_snapshot_issue16_negative() {
        let m = parse_measurement_ut804(&ISSUE16_MINUS_0_0013).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Normal(-0.0013)
unit=V
range_label=4V
display_raw=Some("-0.0013")
flags=auto_range,dc
aux=0
raw_payload=11"#
        );
    }

    /// Issue #16: zero digits with the sign bit. Pinned as the parser reads
    /// it today; what the LCD shows is open (backlog).
    #[test]
    fn ut804_snapshot_issue16_signed_zero() {
        let m = parse_measurement_ut804(&ISSUE16_SIGNED_ZERO).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Normal(-0.0)
unit=V
range_label=4V
display_raw=Some("-0.0000")
flags=auto_range,dc
aux=0
raw_payload=11"#
        );
    }

    #[test]
    fn ut804_issue16_last_digit() {
        let m = parse_measurement_ut804(&ISSUE16_MINUS_0_0008).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("-0.0008"));
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-0.0008)).abs() < 1e-12));
        assert!(m.flags.auto_range);
    }

    // --- UT803 ---

    /// The UT803 parser reads positions 2-10, which are bytes 1-9.
    #[test]
    fn ut803_positions_2_to_10_are_bytes_1_to_9() {
        // Byte 1 range, 2-5 digits, 6 mode, 7 sign, 8 HOLD, 9 DC + AUTO.
        let packet = wire([0x1, 5, 6, 7, 8, 0xB, 0x4, 0x8, 0xA]);
        let m = parse_measurement_ut803(&packet).unwrap();
        assert_eq!(m.range_raw, 1);
        assert_eq!(m.mode_raw, 0xB);
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.display_raw.as_deref(), Some("-56.78"));
        assert!(m.flags.hold && m.flags.dc && m.flags.auto_range);
        assert_eq!(m.raw_payload, packet);
    }

    #[test]
    fn ut803_dcv() {
        let p = ut803_payload(&[5, 9, 9, 9], 0, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.dc);
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 5.999).abs() < 1e-9));
    }

    #[test]
    fn ut803_negative_sign_nib8_bit2() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x4, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - (-12.34)).abs() < 1e-9));
    }

    #[test]
    fn ut803_overload_nib8_bit0() {
        let p = ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x1, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    #[test]
    fn ut803_hold_nib9_bit3() {
        let p = ut803_payload(&[1, 0, 0, 0], 0, 0xB, 0x0, 0x8, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn ut803_auto_and_ac() {
        // nib10: bit 2 = AC, bit 1 = AUTO.
        let p = ut803_payload(&[2, 3, 0, 0], 1, 0xB, 0x0, 0x0, 0x6);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "AC V");
        assert!(m.flags.auto_range);
        assert!(!m.flags.dc);
    }

    #[test]
    fn ut803_mv_range4() {
        let p = ut803_payload(&[5, 9, 9, 9], 4, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "DC mV");
        assert_eq!(m.unit, "mV");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 599.9).abs() < 1e-9));
    }

    #[test]
    fn ut803_resistance_mega() {
        let p = ut803_payload(&[5, 9, 9, 9], 4, 0x3, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "MΩ");
        assert!(matches!(m.value, MeasuredValue::Normal(v) if (v - 5.999).abs() < 1e-9));
    }

    #[test]
    fn ut803_temperature_alt_bit() {
        // nib8 bit 3 set → °C; clear → °F.
        let p = ut803_payload(&[0, 0, 2, 5], 0, 0x4, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.unit, "°C");
        let p = ut803_payload(&[0, 0, 7, 7], 0, 0x4, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.unit, "°F");
    }

    #[test]
    fn ut803_tachometer_alt_bit() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(m.mode, "Tachometer");
        assert_eq!(m.unit, "kRPM");
    }

    #[test]
    fn ut803_unknown_mode_permissive() {
        let p = ut803_payload(&[1, 0, 0, 0], 0, 0x7, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert!(m.mode.starts_with("Unknown"));
    }

    #[test]
    fn payload_too_short() {
        assert!(parse_measurement_ut804(&[0x1, 0x2]).is_err());
        assert!(parse_measurement_ut803(&[0x1, 0x2]).is_err());
    }

    #[test]
    fn invalid_digit_nibble_errors() {
        let p = ut804_payload(&[1, 0xF, 0, 0, 0xA], 1, 0x1, 2, 0x0);
        assert!(parse_measurement_ut804(&p).is_err());
    }

    /// DC V range 2 (39.99 full scale): a plain positive reading.
    #[test]
    fn ut804_snapshot_normal_positive() {
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x02
value=Normal(12.34)
unit=V
range_label=40V
display_raw=Some("12.34")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// The same frame with the status nibble's sign bit set.
    #[test]
    fn ut804_snapshot_negative() {
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x02
value=Normal(-12.34)
unit=V
range_label=40V
display_raw=Some("-12.34")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Overload: digit 1 = 0xA and digit 2 anything but 0xC.
    #[test]
    fn ut804_snapshot_overload_positive() {
        let p = ut804_payload(&[0xA, 0x1, 0, 0, 0], 1, 0x4, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x04
range_raw=0x01
value=Overload
unit=Ω
range_label=400Ω
display_raw=Some("0L")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// The same overload with the sign bit: the digits read "-0L".
    #[test]
    fn ut804_snapshot_overload_negative() {
        let p = ut804_payload(&[0xA, 0x1, 0, 0, 0], 1, 0x1, 2, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Overload
unit=V
range_label=4V
display_raw=Some("-0L")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Digit 1 = 0xA with digit 2 = 0xC: LO, which the vendor reads as zero
    /// rather than an overload. Issue #16's 4-20 mA % reading below its
    /// range, as run 1 sent it.
    #[test]
    fn ut804_snapshot_l0() {
        let p = [
            0xBA, 0xBC, 0xB0, 0xBA, 0xBA, 0xB0, 0xBF, 0xB0, 0xB0, 0x0D, 0x8A,
        ];
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=mA%
mode_raw=0x0f
range_raw=0x00
value=Normal(0.0)
unit=%
range_label=
display_raw=Some("L0.")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// Four frames whose LCD issue #16's reporter read back: the text is
    /// the digit nibbles as drawn, with the range's point and the sign.
    #[test]
    fn ut804_overload_and_lo_text_is_the_lcds() {
        for (packet, mode, unit, lcd) in [
            // Ω range 6, open leads: ".OL MΩ".
            (
                [
                    0xBA, 0xBA, 0xB0, 0xBC, 0xBA, 0xB6, 0x34, 0xB0, 0x31, 0x0D, 0x8A,
                ],
                "Ω",
                "MΩ",
                ".0L",
            ),
            // Diode, open leads: ". OL", the point where a reading's is.
            (
                [
                    0xBA, 0xBA, 0xB0, 0xBC, 0xBA, 0xB0, 0x3B, 0xB0, 0xB0, 0x0D, 0x8A,
                ],
                "Diode",
                "V",
                ".0L",
            ),
            // Continuity, open leads: "0.L Ω".
            (
                [
                    0xBA, 0xBA, 0xB0, 0xBC, 0xBA, 0xB0, 0xBA, 0xB0, 0xB0, 0x0D, 0x8A,
                ],
                "Continuity",
                "Ω",
                "0.L",
            ),
            // The 4-20 mA % reading with the sign bit: "- LO. %".
            (
                [
                    0xBA, 0xBC, 0xB0, 0xBA, 0xBA, 0xB0, 0xBF, 0xB0, 0x34, 0x0D, 0x8A,
                ],
                "mA%",
                "%",
                "-L0.",
            ),
        ] {
            let m = parse_measurement_ut804(&packet).unwrap();
            assert_eq!(
                (m.mode.as_ref(), m.unit.as_ref(), m.display_raw.as_deref()),
                (mode, unit, Some(lcd)),
                "{packet:02X?}"
            );
        }
    }

    /// Digit 4 = 0xB is the idle frame: all displays zero.
    #[test]
    fn ut804_snapshot_idle() {
        let p = ut804_payload(&[0, 0, 0, 0xB, 0], 1, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x01
value=Normal(0.0)
unit=V
range_label=4V
display_raw=Some("0")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Mode code 0 is not in the table: `Unknown(..)`, no unit, and the
    /// fallback decimal position.
    #[test]
    fn ut804_snapshot_unknown_mode() {
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 0, 0x0, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x00)
mode_raw=0x00
range_raw=0x00
value=Normal(1234.0)
unit=
range_label=
display_raw=Some("1234")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// A known mode (V) with a range its table doesn't list: `ut804_mode_info`
    /// returns None, so the reading takes the same fallback as an unknown mode.
    #[test]
    fn ut804_snapshot_range_outside_the_mode_table() {
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 9, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x01)
mode_raw=0x01
range_raw=0x09
value=Normal(1234.0)
unit=
range_label=
display_raw=Some("1234")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// In frequency mode the sign bit selects the duty-cycle display instead
    /// of a negative value.
    #[test]
    fn ut804_snapshot_duty() {
        let p = ut804_payload(&[5, 0, 0, 0, 0xA], 2, 0xC, 0, 0x4);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Duty %
mode_raw=0x0c
range_raw=0x02
value=Normal(500.0)
unit=%
range_label=
display_raw=Some("500.0")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// AC/DC nibble 0 on V: the vendor's default DC label.
    #[test]
    fn ut804_snapshot_acdc_default() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, 0, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x03
value=Normal(230.0)
unit=V
range_label=400V
display_raw=Some("230.0")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// AC/DC nibble 1 on V.
    #[test]
    fn ut804_snapshot_acdc_ac() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, 1, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC V
mode_raw=0x01
range_raw=0x03
value=Normal(230.0)
unit=V
range_label=400V
display_raw=Some("230.0")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// AC/DC nibble 2 on V.
    #[test]
    fn ut804_snapshot_acdc_dc() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, 2, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x01
range_raw=0x03
value=Normal(230.0)
unit=V
range_label=400V
display_raw=Some("230.0")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// AC/DC nibble 3 on V.
    #[test]
    fn ut804_snapshot_acdc_ac_plus_dc() {
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, 3, 0x0);
        let m = parse_measurement_ut804(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=AC+DC V
mode_raw=0x01
range_raw=0x03
value=Normal(230.0)
unit=V
range_label=400V
display_raw=Some("230.0")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// DC V range 0 (5.999 full scale), DC bit set in nibble 10.
    #[test]
    fn ut803_snapshot_normal_positive() {
        let p = ut803_payload(&[5, 9, 9, 9], 0, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x0b
range_raw=0x00
value=Normal(5.999)
unit=V
range_label=
display_raw=Some("5.999")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Nibble 8 bit 2 is the sign.
    #[test]
    fn ut803_snapshot_negative() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x4, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x0b
range_raw=0x01
value=Normal(-12.34)
unit=V
range_label=
display_raw=Some("-12.34")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Nibble 8 bit 0 is overload; the digits are ignored.
    #[test]
    fn ut803_snapshot_overload_positive() {
        let p = ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x1, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x03
range_raw=0x00
value=Overload
unit=Ω
range_label=
display_raw=Some("0L")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// Overload with the sign bit: the digits read "-0L".
    #[test]
    fn ut803_snapshot_overload_negative() {
        let p = ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x5, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x03
range_raw=0x00
value=Overload
unit=Ω
range_label=
display_raw=Some("-0L")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// Mode code 7 is not in the UT803 table.
    #[test]
    fn ut803_snapshot_unknown_mode() {
        let p = ut803_payload(&[1, 0, 0, 0], 0, 0x7, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x07)
mode_raw=0x07
range_raw=0x00
value=Normal(1000.0)
unit=
range_label=
display_raw=Some("1000")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// A known mode (V) with a range its table doesn't list: `ut803_mode_info`
    /// returns None, so the reading takes the unknown-mode fallback.
    #[test]
    fn ut803_snapshot_range_outside_the_mode_table() {
        let p = ut803_payload(&[1, 2, 3, 4], 9, 0xB, 0x0, 0x0, 0x8);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Unknown(0x0b)
mode_raw=0x0b
range_raw=0x09
value=Normal(1234.0)
unit=
range_label=
display_raw=Some("1234")
flags=dc
aux=0
raw_payload=11"#
        );
    }

    /// Temperature with the alt bit clear reads °F.
    #[test]
    fn ut803_snapshot_temperature_alt_clear() {
        let p = ut803_payload(&[0, 0, 7, 7], 0, 0x4, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Temperature
mode_raw=0x04
range_raw=0x00
value=Normal(77.0)
unit=°F
range_label=
display_raw=Some("0077")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// The same frame with nibble 8 bit 3 set reads °C.
    #[test]
    fn ut803_snapshot_temperature_alt_set() {
        let p = ut803_payload(&[0, 0, 2, 5], 0, 0x4, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Temperature
mode_raw=0x04
range_raw=0x00
value=Normal(25.0)
unit=°C
range_label=
display_raw=Some("0025")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// Mode 2 with the alt bit clear is a frequency.
    #[test]
    fn ut803_snapshot_frequency_alt_clear() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x0, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Frequency
mode_raw=0x02
range_raw=0x01
value=Normal(12.34)
unit=kHz
range_label=
display_raw=Some("12.34")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// The same frame with the alt bit set is a tachometer reading in RPM.
    #[test]
    fn ut803_snapshot_tachometer_alt_set() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x8, 0x0, 0x0);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Tachometer
mode_raw=0x02
range_raw=0x01
value=Normal(12.34)
unit=kRPM
range_label=
display_raw=Some("12.34")
flags=
aux=0
raw_payload=11"#
        );
    }

    /// Nibble 9 bit 3 = HOLD, nibble 10 bit 3 = DC, nibble 10 bit 1 = AUTO.
    #[test]
    fn ut803_snapshot_hold_dc_and_auto() {
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x0, 0x8, 0xA);
        let m = parse_measurement_ut803(&p).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=DC V
mode_raw=0x0b
range_raw=0x01
value=Normal(12.34)
unit=V
range_label=
display_raw=Some("12.34")
flags=hold,auto_range,dc
aux=0
raw_payload=11"#
        );
    }

    /// The message a non-packet produces, pinned so the wording survives
    /// the shared packet check.
    #[test]
    fn ut804_error_not_a_packet() {
        let err = parse_measurement_ut804(&[0x1, 0x2]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid response: ut80x: not a packet of 9 data bytes and CR LF"
        );
    }

    /// As `ut804_error_not_a_packet`, for the UT803 layout.
    #[test]
    fn ut803_error_not_a_packet() {
        let err = parse_measurement_ut803(&[0x1, 0x2]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid response: ut80x: not a packet of 9 data bytes and CR LF"
        );
    }

    /// A digit nibble outside 0x0-0xA is rejected by `assemble_value`.
    #[test]
    fn ut804_error_invalid_digit_nibble() {
        let p = ut804_payload(&[1, 0xF, 0, 0, 0xA], 1, 0x1, 2, 0x0);
        let err = parse_measurement_ut804(&p).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid response: ut80x invalid digit nibble 0x0f"
        );
    }

    /// As `ut804_error_invalid_digit_nibble`, for the UT803 layout.
    #[test]
    fn ut803_error_invalid_digit_nibble() {
        let p = ut803_payload(&[1, 0xF, 0, 0], 0, 0xB, 0x0, 0x0, 0x8);
        let err = parse_measurement_ut803(&p).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid response: ut80x invalid digit nibble 0x0f"
        );
    }

    // --- Unrecognised data -----------------------------------------------

    /// Parse `packet` with `parse` and return what was reported with it.
    fn parse_reported(
        parse: fn(&[u8]) -> Result<Measurement>,
        packet: &[u8],
    ) -> (Result<Measurement>, Vec<String>) {
        crate::protocol::capture_reports(|| parse(packet))
    }

    /// The packets the tests above take from issue #16 or build from the
    /// spec, and the overload patterns of spec §8, report nothing: a report
    /// there would warn every user of the meter.
    #[test]
    fn documented_packets_report_nothing() {
        let mut ut804 = vec![
            ISSUE16_OPEN_LEADS.to_vec(),
            ISSUE16_MINUS_0_0013.to_vec(),
            ISSUE16_SIGNED_ZERO.to_vec(),
            ISSUE16_MINUS_0_0008.to_vec(),
            ut804_payload(&[3, 9, 9, 9, 0xA], 1, 0x1, 2, 0x0),
            ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, 0x4),
            ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x1),
            ut804_payload(&[1, 0, 0, 0, 0xA], 1, 0x1, 2, 0x5),
            ut804_payload(&[3, 9, 9, 9, 0xA], 2, 0x4, 0, 0x0),
            ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0xC, 0, 0x0),
            ut804_payload(&[5, 0, 0, 0, 0xA], 2, 0xC, 0, 0x4),
            ut804_payload(&[0, 0, 2, 5, 0xA], 0, 0x6, 0, 0x0),
            ut804_payload(&[0, 0, 7, 7, 0xA], 0, 0xD, 0, 0x0),
            ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 1, 0x0),
            ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x2, 3, 0x0),
            ut804_payload(&[3, 9, 9, 9, 0xA], 0, 0x7, 0, 0x0),
            ut804_payload(&[1, 0, 0, 0, 0xA], 0, 0x9, 0, 0x0),
            ut804_payload(&[0, 0, 0, 0xB, 0], 1, 0x1, 2, 0x0),
            ut804_payload(&[1, 2, 3, 4, 5], 2, 0x1, 2, 0x0),
            ut804_payload(&[0xA, 0xC, 2, 3, 4], 1, 0x1, 2, 0x0),
            // Overload, "L0" and "HI" (spec §8).
            ut804_payload(&[0xA, 0xA, 0, 0xC, 0xA], 3, 0x4, 0, 0x1),
            ut804_payload(&[0xA, 0xC, 0, 0xA, 0xA], 0, 0xF, 0, 0x0),
            ut804_payload(&[0xA, 0xF, 1, 0xA, 0xA], 0, 0xF, 0, 0x0),
        ];
        for acdc in 0..=3 {
            ut804.push(ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, acdc, 0x0));
        }
        let ut803 = [
            wire([0x1, 5, 6, 7, 8, 0xB, 0x4, 0x8, 0xA]),
            ut803_payload(&[5, 9, 9, 9], 0, 0xB, 0x0, 0x0, 0x8),
            ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x4, 0x0, 0x8),
            ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x1, 0x0, 0x0),
            ut803_payload(&[0, 0, 0, 0], 0, 0x3, 0x5, 0x0, 0x0),
            ut803_payload(&[1, 0, 0, 0], 0, 0xB, 0x0, 0x8, 0x8),
            ut803_payload(&[2, 3, 0, 0], 1, 0xB, 0x0, 0x0, 0x6),
            ut803_payload(&[5, 9, 9, 9], 4, 0xB, 0x0, 0x0, 0x8),
            ut803_payload(&[5, 9, 9, 9], 4, 0x3, 0x0, 0x0, 0x0),
            ut803_payload(&[0, 0, 2, 5], 0, 0x4, 0x8, 0x0, 0x0),
            ut803_payload(&[0, 0, 7, 7], 0, 0x4, 0x0, 0x0, 0x0),
            ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x8, 0x0, 0x0),
            ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x0, 0x0, 0x0),
            ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x0, 0x8, 0xA),
        ];
        let cases = ut804
            .iter()
            .map(|p| (parse_measurement_ut804 as fn(&[u8]) -> _, p))
            .chain(
                ut803
                    .iter()
                    .map(|p| (parse_measurement_ut803 as fn(&[u8]) -> _, p)),
            );
        for (parse, packet) in cases {
            let (m, reports) = parse_reported(parse, packet);
            assert!(m.is_ok(), "{packet:02X?}: {m:?}");
            assert!(reports.is_empty(), "{packet:02X?}: {reports:?}");
        }
        // The handhelds send the UT804's packets (ut71 spec §2, §3).
        for model in HANDHELDS {
            for packet in &ut804 {
                let (m, reports) =
                    crate::protocol::capture_reports(|| parse_ut804_layout(model, packet));
                assert!(m.is_ok(), "{model:?} {packet:02X?}: {m:?}");
                assert!(reports.is_empty(), "{model:?} {packet:02X?}: {reports:?}");
            }
        }
    }

    /// A pair outside the model's table still reads as an unknown mode.
    #[test]
    fn a_mode_range_pair_outside_the_table_is_reported() {
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 0, 0x0, 0, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut804, &p);
        assert_eq!(m.unwrap().mode, "Unknown(0x00)");
        assert_eq!(
            reports,
            ["ut804: unrecognised mode/range pair: nibbles 1234A0000"]
        );

        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 9, 0x1, 2, 0x0);
        let (_, reports) = parse_reported(parse_measurement_ut804, &p);
        assert_eq!(
            reports,
            ["ut804: unrecognised mode/range pair: nibbles 1234A9120"]
        );

        let p = ut803_payload(&[1, 0, 0, 0], 0, 0x7, 0x0, 0x0, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut803, &p);
        assert_eq!(m.unwrap().mode, "Unknown(0x07)");
        assert_eq!(
            reports,
            ["ut803: unrecognised mode/range pair: nibbles 010007000"]
        );
    }

    /// A digit nibble past 0xA, or digits that make no number, still fail
    /// the parse, and are reported first.
    #[test]
    fn a_bad_digit_or_value_is_reported_with_its_error() {
        let p = ut804_payload(&[1, 0xF, 0, 0, 0xA], 1, 0x1, 2, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut804, &p);
        assert!(m.is_err());
        assert_eq!(
            reports,
            ["ut804: unrecognised digit nibble: nibbles 1F00A1120"]
        );

        let p = ut803_payload(&[1, 0xF, 0, 0], 0, 0xB, 0x0, 0x0, 0x8);
        let (m, reports) = parse_reported(parse_measurement_ut803, &p);
        assert!(m.is_err());
        assert_eq!(
            reports,
            ["ut803: unrecognised digit nibble: nibbles 01F00B008"]
        );

        // Four blanks leave no number: only the UT803 can get there, as a
        // UT804 packet with digit 1 blank is an overload.
        let p = ut803_payload(&[0xA; 4], 0, 0xB, 0x0, 0x0, 0x8);
        let (m, reports) = parse_reported(parse_measurement_ut803, &p);
        assert_eq!(
            m.unwrap_err().to_string(),
            "invalid response: ut80x unparseable value \"\""
        );
        assert_eq!(
            reports,
            [
                "ut803: unrecognised digit nibble: nibbles 0AAAAB008",
                "ut803: unrecognised value: nibbles 0AAAAB008",
            ]
        );
    }

    #[test]
    fn ut804_coupling_outside_its_modes_or_values_is_reported() {
        // Value 4 on V: no label, as before.
        let p = ut804_payload(&[2, 3, 0, 0, 0xA], 3, 0x1, 4, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut804, &p);
        let m = m.unwrap();
        assert_eq!(m.mode, "V");
        assert!(!m.flags.dc);
        assert_eq!(
            reports,
            ["ut804: unrecognised ac/dc nibble: nibbles 2300A3140"]
        );

        // AC on resistance: the label stays plain.
        let p = ut804_payload(&[3, 9, 9, 9, 0xA], 2, 0x4, 1, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut804, &p);
        assert_eq!(m.unwrap().mode, "Ω");
        assert_eq!(
            reports,
            ["ut804: unrecognised ac/dc nibble: nibbles 3999A2410"]
        );

        // AC+DC on the current modes is known.
        for mode in [0x7, 0x8, 0x9] {
            let p = ut804_payload(&[1, 0, 0, 0, 0xA], 1, mode, 3, 0x0);
            let (_, reports) = parse_reported(parse_measurement_ut804, &p);
            assert!(reports.is_empty(), "mode {mode:#x}: {reports:?}");
        }
    }

    #[test]
    fn ut804_undefined_status_bits_are_reported() {
        for status in [0x8, 0x3, 0x7, 0x9] {
            let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, status);
            let (m, reports) = parse_reported(parse_measurement_ut804, &p);
            assert!(m.is_ok());
            assert_eq!(
                reports,
                [format!(
                    "ut804: unrecognised status bits: nibbles 1234A212{status:X}"
                )]
            );
        }
        // Bit 1 alone is MAN, with or without the sign (spec §8).
        for status in [0x2, 0x6] {
            let p = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0x1, 2, status);
            let (_, reports) = parse_reported(parse_measurement_ut804, &p);
            assert!(reports.is_empty(), "status {status:#x}: {reports:?}");
        }
    }

    /// Digit 1 blank with digit 2 other than A, C or F still reads as an
    /// overload.
    #[test]
    fn ut804_undocumented_overload_pattern_is_reported() {
        let p = ut804_payload(&[0xA, 0x1, 0, 0, 0], 1, 0x4, 0, 0x0);
        let (m, reports) = parse_reported(parse_measurement_ut804, &p);
        let m = m.unwrap();
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw.as_deref(), Some("0L"));
        assert_eq!(
            reports,
            ["ut804: unrecognised overload pattern: nibbles A10001400"]
        );
    }

    /// A blank inside a UT804 reading still renders as a gap; digit 5 blank
    /// is the 4000-count display and the idle frame is known.
    ///
    /// Range 4 puts the decimal point after digit 4, past the blanks: a
    /// blank just after the point's digit repeats the point ("12..45"), and
    /// the value fails to parse.
    #[test]
    fn ut804_blank_inside_a_reading_is_reported() {
        for (digits, nibbles, display) in [
            ([1, 0xA, 3, 4, 5], "1A3454120", "1345"),
            ([1, 2, 0xA, 4, 5], "12A454120", "1245"),
            ([1, 2, 3, 0xA, 5], "123A54120", "1235"),
        ] {
            let p = ut804_payload(&digits, 4, 0x1, 2, 0x0);
            let (m, reports) = parse_reported(parse_measurement_ut804, &p);
            assert_eq!(m.unwrap().display_raw.as_deref(), Some(display));
            assert_eq!(
                reports,
                [format!(
                    "ut804: unrecognised digit nibble: nibbles {nibbles}"
                )]
            );
        }

        for digits in [[1, 2, 3, 4, 0xA], [0, 0, 0, 0xB, 0xA]] {
            let p = ut804_payload(&digits, 2, 0x1, 2, 0x0);
            let (_, reports) = parse_reported(parse_measurement_ut804, &p);
            assert!(reports.is_empty(), "{digits:X?}: {reports:?}");
        }
    }

    #[test]
    fn ut803_blank_digit_is_reported() {
        for (digits, nibbles) in [([0xA, 2, 3, 4], "1A234B008"), ([1, 2, 3, 0xA], "1123AB008")] {
            let p = ut803_payload(&digits, 1, 0xB, 0x0, 0x0, 0x8);
            let (m, reports) = parse_reported(parse_measurement_ut803, &p);
            assert!(m.is_ok());
            assert_eq!(
                reports,
                [format!(
                    "ut803: unrecognised digit nibble: nibbles {nibbles}"
                )]
            );
        }
        // An overload's digits are not read.
        let p = ut803_payload(&[0xA; 4], 0, 0x3, 0x1, 0x0, 0x0);
        let (_, reports) = parse_reported(parse_measurement_ut803, &p);
        assert!(reports.is_empty(), "{reports:?}");
    }

    #[test]
    fn ut803_undefined_status_bits_are_reported() {
        // Nibble 8 bit 1, nibble 9 bit 0, and the alt bit on volts.
        for (nib8, nib9, nibbles) in [
            (0x2, 0x0, "11234B208"),
            (0x0, 0x1, "11234B018"),
            (0x8, 0x0, "11234B808"),
        ] {
            let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, nib8, nib9, 0x8);
            let (m, reports) = parse_reported(parse_measurement_ut803, &p);
            assert_eq!(m.unwrap().mode, "DC V");
            assert_eq!(
                reports,
                [format!(
                    "ut803: unrecognised status bits: nibbles {nibbles}"
                )]
            );
        }
    }

    /// Nibble 9 bits 2-1 light indicators of their own (§7.4 item 2), so the
    /// meter sets them in ordinary use and they are no cause to ask for a
    /// report.
    #[test]
    fn ut803_indicator_bits_stay_silent() {
        for nib9 in [0x2, 0x4, 0x6] {
            let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x0, nib9, 0x8);
            let (m, reports) = parse_reported(parse_measurement_ut803, &p);
            assert_eq!(m.unwrap().mode, "DC V");
            assert!(reports.is_empty(), "nibble 9 = {nib9:#x}: {reports:?}");
        }
    }

    #[test]
    fn ut803_volts_with_both_or_neither_coupling_bit_are_reported() {
        for (nib10, mode, nibbles) in [
            (0xC, "DC V", "11234B00C"),
            (0x0, "AC V", "11234B000"),
            (0x2, "AC V", "11234B002"),
        ] {
            let p = ut803_payload(&[1, 2, 3, 4], 1, 0xB, 0x0, 0x0, nib10);
            let (m, reports) = parse_reported(parse_measurement_ut803, &p);
            assert_eq!(m.unwrap().mode, mode);
            assert_eq!(
                reports,
                [format!("ut803: unrecognised ac/dc bits: nibbles {nibbles}")]
            );
        }
        // Neither bit on resistance is not about coupling.
        let p = ut803_payload(&[1, 2, 3, 4], 1, 0x3, 0x0, 0x0, 0x0);
        let (_, reports) = parse_reported(parse_measurement_ut803, &p);
        assert!(reports.is_empty(), "{reports:?}");
    }

    // --- UT803 specs ------------------------------------------------------

    /// Why a reading has no spec, and which readings that is.
    type NoSpec = (&'static str, fn(&Ut803Fields) -> bool);

    /// UT803 readings the parser accepts that have no spec in the manual.
    const UT803_NO_SPEC: &[NoSpec] = &[
        ("the tachometer: the manual gives RPM no spec", |f| {
            f.mode == 0x2 && f.alt()
        }),
        (
            "frequency range 5 (600MHz): the manual stops at 60MHz",
            |f| f.mode == 0x2 && !f.alt() && f.range == 5,
        ),
        ("capacitance range 7 (60mF): the manual stops at 6mF", |f| {
            f.mode == 0x6 && f.range == 7
        }),
        ("ADP: that it is hFE is only a guess", |f| f.mode == 0xE),
        ("current with both or neither coupling bit", |f| {
            matches!(f.mode, 0x9 | 0xD | 0xF)
                && !matches!(f.coupling(), Some(Coupling::Dc | Coupling::Ac))
        }),
    ];

    /// Every mode, range, coupling and alt bit the parser accepts without a
    /// report, as a packet and its reading.
    fn ut803_accepted_readings() -> Vec<(Vec<u8>, Measurement)> {
        let mut readings = Vec::new();
        for mode in 0..=0xF {
            for range in 0..=0xF {
                for nib10 in [0x0, 0x4, 0x8, 0xC] {
                    for nib8 in [0x0, 0x8] {
                        let p = ut803_payload(&[1, 2, 3, 4], range, mode, nib8, 0x0, nib10);
                        if let (Ok(m), reports) = parse_reported(parse_measurement_ut803, &p)
                            && reports.is_empty()
                        {
                            readings.push((p, m));
                        }
                    }
                }
            }
        }
        readings
    }

    /// Each reading resolves a spec or is listed as having none, and every
    /// row of every table is some reading's, apart from the transistor
    /// table, which no reading reaches (its comment says why).
    #[test]
    fn ut803_every_reading_has_a_spec_or_is_listed() {
        let proto = Ut80xProtocol::new_ut803();
        let mut rows_reached = std::collections::HashSet::new();
        let mut listed_reached = [false; UT803_NO_SPEC.len()];
        for (p, m) in ut803_accepted_readings() {
            let f = Ut803Fields::decode(&p).unwrap();
            let listed: Vec<usize> = (0..UT803_NO_SPEC.len())
                .filter(|&i| (UT803_NO_SPEC[i].1)(&f))
                .collect();
            let row = proto.spec_row(&m);
            match (row, listed.as_slice()) {
                (Some(row), []) => {
                    assert!(std::ptr::eq(proto.spec_info(&m).unwrap(), &row.spec));
                    assert!(proto.mode_spec_info(&m).is_some());
                    rows_reached.insert(std::ptr::from_ref(row));
                }
                (None, [i]) => {
                    assert!(proto.spec_info(&m).is_none());
                    listed_reached[*i] = true;
                }
                (Some(_), _) => panic!("{} {p:02X?}: has a spec, yet is listed", m.mode),
                (None, _) => panic!("{} {p:02X?}: no spec, and not listed once", m.mode),
            }
        }
        for (i, (why, _)) in UT803_NO_SPEC.iter().enumerate() {
            assert!(listed_reached[i], "no reading is {why}");
        }
        for table in specs_ut803::ALL {
            for row in table.ranges {
                assert_eq!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    table.name != "K. Transistor",
                    "{} / {}",
                    table.name,
                    row.label
                );
            }
        }
    }

    /// `label` as a value in its base unit, and that unit: "6kHz" is
    /// (6000.0, "Hz"). `None` for a label that is not a number, an SI
    /// prefix and a unit.
    fn si_quantity(label: &str) -> Option<(f64, &str)> {
        let digits = label
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(label.len());
        let value: f64 = label[..digits].parse().ok()?;
        let unit = &label[digits..];
        let base = ["Hz", "Ω", "V", "A", "F"]
            .into_iter()
            .find(|base| unit.ends_with(base))?;
        let factor = match &unit[..unit.len() - base.len()] {
            "" => 1.0,
            "n" => 1e-9,
            "µ" => 1e-6,
            "m" => 1e-3,
            "k" => 1e3,
            "M" => 1e6,
            _ => return None,
        };
        Some((value * factor, base))
    }

    /// UT803 ranges whose full scale is not the 6000 counts the decimal
    /// point gives, as (mode, range byte, full scale in the reading's unit).
    const UT803_FULL_SCALE_EXCEPTIONS: &[(u8, u8, f64)] = &[
        // The 1000V range is an integer display that stops at 1000, not 5999.
        (0xB, 3, 1000.0),
    ];

    /// A row answers the reading's range: its label's full scale is the one
    /// the reading's decimal point gives, 6000 counts. A row mapped to the
    /// wrong range byte names another decade, or another quantity. The rows
    /// with no range byte (10A, continuity, diode, temperature) are one
    /// fixed range each, and are left out.
    #[test]
    fn ut803_rows_match_the_readings_full_scale() {
        let proto = Ut80xProtocol::new_ut803();
        for (p, m) in ut803_accepted_readings() {
            let Some(row) = proto.spec_row(&m) else {
                continue;
            };
            if row.range.is_none() {
                continue;
            }
            let f = Ut803Fields::decode(&p).unwrap();
            let (_, unit, dp) = ut803_mode_info(f.mode, f.range, f.alt()).unwrap();
            let scale = UT803_FULL_SCALE_EXCEPTIONS
                .iter()
                .find(|&&(mode, range, _)| (mode, range) == (f.mode, f.range))
                .map_or(6000.0 / 10f64.powi(3 - i32::from(dp)), |&(.., scale)| scale);
            let full_scale = format!("{scale}{unit}");
            let reading = si_quantity(&full_scale).unwrap();
            let label =
                si_quantity(row.label).unwrap_or_else(|| panic!("{} is not a quantity", row.label));
            assert!(
                label.1 == reading.1 && (label.0 - reading.0).abs() <= 1e-9 * reading.0,
                "{} {p:02X?}: row {} for a {full_scale} range",
                m.mode,
                row.label
            );
        }
    }

    /// A DC table answers only DC readings and an AC one only AC readings,
    /// by the coupling bits; the other tables have no coupling.
    #[test]
    fn ut803_tables_match_the_readings_coupling() {
        let proto = Ut80xProtocol::new_ut803();
        for (p, m) in ut803_accepted_readings() {
            let Some((table, _)) = proto.spec_table(&m) else {
                continue;
            };
            let expected = match table.name {
                "A. DC Voltage" | "C. DC Current" => Coupling::Dc,
                "B. AC Voltage" | "D. AC Current" => Coupling::Ac,
                _ => continue,
            };
            let coupling = Ut803Fields::decode(&p).unwrap().coupling();
            assert_eq!(
                coupling,
                Some(expected),
                "{} {p:02X?}: {}",
                m.mode,
                table.name
            );
        }
    }

    /// A row's resolution is in the reading's unit, prefix aside: a row of
    /// another quantity (continuity for diode, °F for °C) resolves in
    /// another unit.
    #[test]
    fn ut803_rows_resolve_in_the_readings_unit() {
        use crate::protocol::test_support::unit_family;
        let proto = Ut80xProtocol::new_ut803();
        for (p, m) in ut803_accepted_readings() {
            let Some(row) = proto.spec_row(&m) else {
                continue;
            };
            let unit = row
                .spec
                .resolution
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            assert_eq!(
                unit_family(unit),
                unit_family(&m.unit),
                "{} {p:02X?}: row {} resolves in {}",
                m.mode,
                row.label,
                row.spec.resolution
            );
        }
    }

    fn ut803_table_name(p: &[u8]) -> Option<&'static str> {
        let m = parse_measurement_ut803(p).unwrap();
        let (table, _) = Ut80xProtocol::new_ut803().spec_table(&m)?;
        Some(table.name)
    }

    fn ut803_spec(p: &[u8]) -> Option<&'static SpecInfo> {
        let m = parse_measurement_ut803(p).unwrap();
        Ut80xProtocol::new_ut803().spec_info(&m)
    }

    /// DC and AC current share their mode codes; the coupling bits pick the
    /// table.
    #[test]
    fn ut803_ac_and_dc_ma_pick_their_own_tables() {
        let dc = ut803_payload(&[1, 2, 3, 4], 0, 0xF, 0x0, 0x0, 0x8);
        let ac = ut803_payload(&[1, 2, 3, 4], 0, 0xF, 0x0, 0x0, 0x4);
        assert_eq!(ut803_table_name(&dc), Some("C. DC Current"));
        assert_eq!(ut803_table_name(&ac), Some("D. AC Current"));
        assert_eq!(ut803_spec(&dc).unwrap().accuracy[0].accuracy, "0.5%+3");
        assert_eq!(ut803_spec(&ac).unwrap().accuracy[0].accuracy, "1.0%+5");
    }

    #[test]
    fn ut803_rpm_has_no_spec_and_hz_has() {
        let hz = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x0, 0x0, 0x0);
        let rpm = ut803_payload(&[1, 2, 3, 4], 1, 0x2, 0x8, 0x0, 0x0);
        assert_eq!(ut803_spec(&hz).unwrap().resolution, "0.01kHz");
        assert_eq!(ut803_table_name(&rpm), None);
    }

    #[test]
    fn ut803_alt_bit_picks_the_temperature_row() {
        let c = ut803_payload(&[0, 0, 2, 5], 0, 0x4, 0x8, 0x0, 0x0);
        let f = ut803_payload(&[0, 0, 7, 7], 0, 0x4, 0x0, 0x0, 0x0);
        assert_eq!(ut803_spec(&c).unwrap().resolution, "1°C");
        assert_eq!(ut803_spec(&f).unwrap().resolution, "1°F");
    }

    /// The 600mV range is range 4 of the volts mode, with an input impedance
    /// of its own.
    #[test]
    fn ut803_600mv_has_its_own_input_impedance() {
        let proto = Ut80xProtocol::new_ut803();
        let impedance = |range| {
            let p = ut803_payload(&[1, 2, 3, 4], range, 0xB, 0x0, 0x0, 0x8);
            let m = parse_measurement_ut803(&p).unwrap();
            proto.mode_spec_info(&m).unwrap().input_impedance
        };
        assert_eq!(impedance(4), Some("Around > 3000MΩ"));
        assert_eq!(impedance(0), Some("Around 10MΩ"));
    }

    #[test]
    fn ut803_malformed_payload_has_no_spec() {
        let proto = Ut80xProtocol::new_ut803();
        let p = ut803_payload(&[1, 2, 3, 4], 0, 0xB, 0x0, 0x0, 0x8);
        let mut m = parse_measurement_ut803(&p).unwrap();
        assert!(proto.spec_info(&m).is_some());
        for payload in [vec![], p[..10].to_vec(), vec![0xFF; PACKET_LEN]] {
            m.raw_payload = payload;
            assert!(proto.spec_info(&m).is_none());
            assert!(proto.mode_spec_info(&m).is_none());
        }
    }

    // --- UT804 specs ------------------------------------------------------

    /// Why a UT804 reading has no row or no spec, and which readings that is.
    type Ut804NoSpec = (&'static str, fn(&Ut804Fields) -> bool);

    /// UT804 readings that take their table's mode data but no row.
    const UT804_MODE_SPEC_ONLY: &[Ut804NoSpec] = &[(
        "AC+DC V and current: the remarks add (1%+35 digits) to the AC table, a sum the manual does not print",
        |f| matches!(f.mode, 0x1 | 0x2 | 0x7 | 0x8 | 0x9) && f.coupling() == Some(Coupling::AcDc),
    )];

    /// UT804 readings the parser accepts that have no spec in the manual.
    const UT804_NO_SPEC: &[Ut804NoSpec] = &[
        ("power: no dial position, and no table", |f| f.mode == 0xE),
        (
            "AC or AC+DC mV: the manual gives the UT804 no AC mV function, and table B no 400mV row",
            |f| f.mode == 0x3 && matches!(f.coupling(), Some(Coupling::Ac | Coupling::AcDc)),
        ),
    ];

    /// Every mode, range, coupling, sign and AUTO bit the parser accepts
    /// without a report, as a packet and its reading.
    fn ut804_accepted_readings() -> Vec<(Vec<u8>, Measurement)> {
        ut804_layout_accepted_readings(Model::Ut804)
    }

    /// As [`ut804_accepted_readings`], read as `model`.
    fn ut804_layout_accepted_readings(model: Model) -> Vec<(Vec<u8>, Measurement)> {
        let mut readings = Vec::new();
        for mode in 0..=0xF {
            for range in 0..=0xF {
                for acdc in 0..=0x4 {
                    for status in [0x0, 0x1, 0x4, 0x5] {
                        let p = ut804_payload(&[1, 2, 3, 4, 0xA], range, mode, acdc, status);
                        if let (Ok(m), reports) =
                            crate::protocol::capture_reports(|| parse_ut804_layout(model, &p))
                            && reports.is_empty()
                        {
                            readings.push((p, m));
                        }
                    }
                }
            }
        }
        readings
    }

    /// Each reading resolves a spec, is listed as taking its table's mode
    /// data only, or is listed as having none; every row of every table is
    /// some reading's.
    #[test]
    fn ut804_every_reading_has_a_spec_or_is_listed() {
        let proto = Ut80xProtocol::new_ut804();
        let lists = [UT804_MODE_SPEC_ONLY, UT804_NO_SPEC];
        let mut rows_reached = std::collections::HashSet::new();
        let mut listed_reached = std::collections::HashSet::new();
        for (p, m) in ut804_accepted_readings() {
            let f = Ut804Fields::decode(&p).unwrap();
            let listed: Vec<(usize, &str)> = lists
                .iter()
                .enumerate()
                .flat_map(|(l, list)| list.iter().map(move |entry| (l, entry)))
                .filter(|(_, (_, hit))| hit(&f))
                .map(|(l, (why, _))| (l, *why))
                .collect();
            let has_mode_spec = proto.mode_spec_info(&m).is_some();
            match (proto.spec_row(&m), listed.as_slice()) {
                (Some(row), []) => {
                    assert!(std::ptr::eq(proto.spec_info(&m).unwrap(), &row.spec));
                    assert!(has_mode_spec);
                    rows_reached.insert(std::ptr::from_ref(row));
                }
                (None, [(l, why)]) => {
                    assert!(proto.spec_info(&m).is_none());
                    // The first list keeps the mode data, the second has none.
                    assert_eq!(has_mode_spec, *l == 0, "{} {p:02X?}: {why}", m.mode);
                    listed_reached.insert(*why);
                }
                (Some(_), _) => panic!("{} {p:02X?}: has a spec, yet is listed", m.mode),
                (None, _) => panic!("{} {p:02X?}: no spec, and not listed once", m.mode),
            }
        }
        for (why, _) in lists.iter().flat_map(|list| list.iter()) {
            assert!(listed_reached.contains(why), "no reading is {why}");
        }
        for table in specs_ut804::ALL {
            for row in table.ranges {
                assert!(
                    rows_reached.contains(&std::ptr::from_ref(row)),
                    "{} / {} is no reading's",
                    table.name,
                    row.label
                );
            }
        }
    }

    /// A row carries the range label the parser gives the reading. The
    /// continuity and diode rows are labelled by a symbol.
    #[test]
    fn ut804_rows_carry_the_readings_range_label() {
        let proto = Ut80xProtocol::new_ut804();
        for (p, m) in ut804_accepted_readings() {
            let Some(row) = proto.spec_row(&m) else {
                continue;
            };
            if m.range_label.is_empty() || row.label.ends_with("symbol)") {
                continue;
            }
            assert_eq!(row.label, m.range_label, "{} {p:02X?}", m.mode);
        }
    }

    /// A DC table answers only DC readings, and an AC table AC and AC+DC
    /// ones, by the coupling nibble; the other tables answer readings with
    /// no coupling.
    #[test]
    fn ut804_tables_match_the_readings_coupling() {
        let proto = Ut80xProtocol::new_ut804();
        for (p, m) in ut804_accepted_readings() {
            let Some((table, _)) = proto.spec_table(&m) else {
                continue;
            };
            let coupling = Ut804Fields::decode(&p).unwrap().coupling();
            let fits = match table.name {
                "A. DC Voltage" | "C. DC Current" => coupling == Some(Coupling::Dc),
                "B. AC Voltage (AC+DC measurement is available)"
                | "D. AC Current (AC+DC measurement is available)" => {
                    matches!(coupling, Some(Coupling::Ac | Coupling::AcDc))
                }
                _ => coupling.is_none(),
            };
            assert!(fits, "{} {p:02X?}: {} for {coupling:?}", m.mode, table.name);
        }
    }

    /// A row's resolution is in the reading's unit, prefix aside: a row of
    /// another quantity (continuity for diode, °F for °C) resolves in
    /// another unit.
    #[test]
    fn ut804_rows_resolve_in_the_readings_unit() {
        use crate::protocol::test_support::unit_family;
        let proto = Ut80xProtocol::new_ut804();
        for (p, m) in ut804_accepted_readings() {
            let Some(row) = proto.spec_row(&m) else {
                continue;
            };
            let unit = row
                .spec
                .resolution
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            assert_eq!(
                unit_family(unit),
                unit_family(&m.unit),
                "{} {p:02X?}: row {} resolves in {}",
                m.mode,
                row.label,
                row.spec.resolution
            );
        }
    }

    fn ut804_table_name(p: &[u8]) -> Option<&'static str> {
        let m = parse_measurement_ut804(p).unwrap();
        let (table, _) = Ut80xProtocol::new_ut804().spec_table(&m)?;
        Some(table.name)
    }

    fn ut804_spec(p: &[u8]) -> Option<&'static SpecInfo> {
        let m = parse_measurement_ut804(p).unwrap();
        Ut80xProtocol::new_ut804().spec_info(&m)
    }

    /// DC V and AC V share range 4, 1000V, in their own tables. AC+DC takes
    /// the AC table's mode data, whose remarks give its adder, but no row.
    #[test]
    fn ut804_coupling_picks_the_volts_table() {
        let volts = |acdc| ut804_payload(&[0, 2, 3, 0, 0], 4, 0x2, acdc, 0x0);
        assert_eq!(ut804_table_name(&volts(2)), Some("A. DC Voltage"));
        assert_eq!(
            ut804_spec(&volts(2)).unwrap().accuracy[0].accuracy,
            "0.1%+8"
        );
        let ac = "B. AC Voltage (AC+DC measurement is available)";
        assert_eq!(ut804_table_name(&volts(1)), Some(ac));
        assert_eq!(ut804_spec(&volts(1)).unwrap().accuracy[0].accuracy, "1%+30");

        let acdc = parse_measurement_ut804(&volts(3)).unwrap();
        let proto = Ut80xProtocol::new_ut804();
        assert_eq!(ut804_table_name(&volts(3)), Some(ac));
        assert!(proto.spec_info(&acdc).is_none());
        let notes = proto.mode_spec_info(&acdc).unwrap().notes;
        assert!(notes.iter().any(|n| n.contains("AC+DC")), "{notes:?}");
    }

    #[test]
    fn ut804_sign_bit_picks_duty_over_frequency() {
        let hz = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0xC, 0, 0x0);
        let duty = ut804_payload(&[1, 2, 3, 4, 0xA], 2, 0xC, 0, 0x4);
        assert_eq!(ut804_spec(&hz).unwrap().resolution, "0.0001kHz");
        assert_eq!(ut804_table_name(&duty), Some("J. Duty Cycle"));
    }

    /// DC mV is A's 400mV row, with its own input impedance; AC mV has no
    /// row to take.
    #[test]
    fn ut804_millivolts_take_the_400mv_row_on_dc_only() {
        let proto = Ut80xProtocol::new_ut804();
        let dc = parse_measurement_ut804(&ut804_payload(&[1, 1, 1, 3, 7], 0, 0x3, 0, 0x0));
        let dc = dc.unwrap();
        assert_eq!(proto.spec_info(&dc).unwrap().resolution, "0.01mV");
        assert_eq!(
            proto.mode_spec_info(&dc).unwrap().input_impedance,
            Some("Around 2.5GΩ")
        );
        let ac = ut804_payload(&[1, 1, 1, 3, 7], 0, 0x3, 1, 0x0);
        assert_eq!(ut804_table_name(&ac), None);
    }

    #[test]
    fn ut804_malformed_payload_has_no_spec() {
        let proto = Ut80xProtocol::new_ut804();
        let p = ut804_payload(&[1, 2, 3, 4, 0xA], 1, 0x1, 2, 0x0);
        let mut m = parse_measurement_ut804(&p).unwrap();
        assert!(proto.spec_info(&m).is_some());
        for payload in [vec![], p[..10].to_vec(), vec![0xFF; PACKET_LEN]] {
            m.raw_payload = payload;
            assert!(proto.spec_info(&m).is_none());
            assert!(proto.mode_spec_info(&m).is_none());
        }
    }

    // --- Detection (crate::detect) ---------------------------------------

    /// A packet does not name its model, and the UT804 is heard at the
    /// CH9325's 2400 baud start-up rate, so any whole packet is a UT804.
    #[test]
    fn a_whole_packet_is_recognised_as_a_ut804() {
        let recognise = FINGERPRINT.recognise;
        let mut buf = ISSUE16_OPEN_LEADS[3..].to_vec();
        assert_eq!(recognise(&buf, &Probing::default()), None);
        buf.extend_from_slice(&ISSUE16_MINUS_0_0008);
        assert_eq!(
            recognise(&buf, &Probing::default()),
            Some(Evidence::Model {
                id: "ut804",
                reported_name: None,
            })
        );
    }

    /// Whether a UT71 or VC9x0 sets the UT804's parity bit is open (ut71
    /// spec §6): a packet with bit 7 clear throughout, CR LF as `0D 0A`, is
    /// still claimed as a UT804, which reads it as one.
    #[test]
    fn a_packet_without_parity_is_recognised_as_a_ut804() {
        // DC V 12.345 on range 2, AUTO: every nibble as 0x30 | n.
        let packet = [
            0x31, 0x32, 0x33, 0x34, 0x35, 0x32, 0x31, 0x30, 0x31, 0x0D, 0x0A,
        ];
        assert!(packet.iter().all(|b| b & 0x80 == 0));
        assert_eq!(
            (FINGERPRINT.recognise)(&packet, &Probing::default()),
            Some(Evidence::Model {
                id: "ut804",
                reported_name: None,
            })
        );
        let m = parse_ut804_layout(Model::Ut71Cde, &packet).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.345"));
        assert_eq!(m.mode, "DC V");
    }
}
