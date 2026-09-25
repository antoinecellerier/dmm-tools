//! The packet layouts, and how a packet becomes a reading
//! (`docs/research/zotek/reverse-engineered-protocol.md` §6, §7).
//!
//! A packet is an image of the LCD: digit glyphs, and one bit per
//! annunciator. Each layout is a table of which bit of which byte lights
//! what; the reading's mode, unit and flags are worked out from the lit
//! annunciators the way the LCD shows them.

use super::Unrecognised;
use super::frame::{self, HEADER, TYPE_AT};
use super::glyph::{self, Cell, Glyph, Shown};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::unknown_mode;
use std::borrow::Cow;

/// A unit annunciator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Unit {
    Volt,
    Amp,
    Ohm,
    Farad,
    Hertz,
    Percent,
    Celsius,
    Fahrenheit,
}

impl Unit {
    /// The unit a reading is in when several are lit, most specific first:
    /// frequency and duty over the V they are measured on, and so on.
    const PRIORITY: [Unit; 8] = [
        Unit::Celsius,
        Unit::Fahrenheit,
        Unit::Percent,
        Unit::Hertz,
        Unit::Farad,
        Unit::Ohm,
        Unit::Amp,
        Unit::Volt,
    ];

    fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A prefix annunciator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Prefix {
    Nano,
    Micro,
    Milli,
    Kilo,
    Mega,
}

/// The unit string for `unit` with `prefix`. Only the pairs a layout's
/// prefix groups allow are reached (spec §7.5).
fn unit_str(prefix: Option<Prefix>, unit: Unit) -> &'static str {
    use Prefix::{Kilo, Mega, Micro, Milli, Nano};
    match (prefix, unit) {
        (Some(Nano), Unit::Volt) => "nV",
        (Some(Micro), Unit::Volt) => "µV",
        (Some(Milli), Unit::Volt) => "mV",
        (Some(Nano), Unit::Amp) => "nA",
        (Some(Micro), Unit::Amp) => "µA",
        (Some(Milli), Unit::Amp) => "mA",
        (Some(Nano), Unit::Farad) => "nF",
        (Some(Micro), Unit::Farad) => "µF",
        (Some(Milli), Unit::Farad) => "mF",
        (Some(Kilo), Unit::Ohm) => "kΩ",
        (Some(Mega), Unit::Ohm) => "MΩ",
        (Some(Kilo), Unit::Hertz) => "kHz",
        (Some(Mega), Unit::Hertz) => "MHz",
        (_, Unit::Volt) => "V",
        (_, Unit::Amp) => "A",
        (_, Unit::Farad) => "F",
        (_, Unit::Ohm) => "Ω",
        (_, Unit::Hertz) => "Hz",
        (_, Unit::Percent) => "%",
        (_, Unit::Celsius) => "°C",
        (_, Unit::Fahrenheit) => "°F",
    }
}

/// What a lit bit means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Meaning {
    Unit(Unit),
    /// A prefix, and the units the layout attaches it to (spec §7.5).
    Prefix(Prefix, &'static [Unit]),
    Ac,
    Dc,
    Diode,
    Continuity,
    Hold,
    Rel,
    AutoRange,
    Min,
    Max,
    LowBattery,
    /// PEAK: the meter does not say whether it holds the maximum or the
    /// minimum.
    Peak,
    /// INRUSH, on the clamp (spec §7.2).
    Inrush,
    /// Over-voltage, type 2's byte 3 bit 2 (spec §7.3).
    OverVoltage,
    /// Type 4's secondary display units (spec §7.4).
    SecondaryPercent,
    SecondaryHertz,
    SecondaryKilo,
    /// Documented, and deliberately not decoded: the Bluetooth icon, the
    /// unnamed bits, and bits community captures show to be something the
    /// reading does not carry (each named where the layout lists it).
    Silent,
}

/// One annunciator bit, or a run of silent ones.
pub(super) struct Bit {
    byte: usize,
    mask: u8,
    meaning: Meaning,
}

/// Bit `n` of `byte`.
const fn bit(byte: usize, n: u8, meaning: Meaning) -> Bit {
    Bit {
        byte,
        mask: 1 << n,
        meaning,
    }
}

/// The bits of `mask` in `byte`, none of which the reading carries.
const fn silent(byte: usize, mask: u8) -> Bit {
    Bit {
        byte,
        mask,
        meaning: Meaning::Silent,
    }
}

const FARAD: &[Unit] = &[Unit::Farad];
const VOLT: &[Unit] = &[Unit::Volt];
const AMP: &[Unit] = &[Unit::Amp];
const OHM_HZ: &[Unit] = &[Unit::Ohm, Unit::Hertz];
const VOLT_AMP_FARAD: &[Unit] = &[Unit::Volt, Unit::Amp, Unit::Farad];

/// Where the digits sit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Digits {
    /// Types 1-3: four digits split across bytes 3-7, each taking its high
    /// nibble from one byte and its low nibble from the next; byte 3 bit 4
    /// is the minus and bytes 4-6 bit 4 the decimal points (spec §6.2).
    Split,
    /// Type 4: one glyph byte per digit, least significant first. Main in
    /// bytes 9-12 with a leading "1" from byte 13 bits 3 and 2 and the minus
    /// in bit 7; secondary in bytes 5-8, its minus in byte 8 bit 4
    /// (spec §6.3).
    Wide,
}

/// A row of up to five cells, most significant first, and its sign.
pub(super) struct Row {
    cells: [Cell; 5],
    len: usize,
    negative: bool,
}

impl Row {
    fn cells(&self) -> &[Cell] {
        &self.cells[..self.len]
    }

    /// Whether no digit is lit, whatever sign or point the blanks carry.
    fn blank(&self) -> bool {
        self.cells().iter().all(|c| c.glyph == Glyph::Blank)
    }
}

const BLANK: Cell = Cell {
    glyph: Glyph::Blank,
    dp: false,
};

impl Digits {
    /// The bits of `byte` the digits take.
    fn mask(self, byte: usize) -> u8 {
        match (self, byte) {
            (Digits::Split, 3) => 0xF0,
            (Digits::Split, 4..=6) => 0xFF,
            (Digits::Split, 7) => 0x0F,
            (Digits::Wide, 5..=12) => 0xFF,
            (Digits::Wide, 13) => MINUS_BIG | LEADING_ONE,
            _ => 0,
        }
    }

    /// The main reading's row.
    fn main(self, packet: &[u8]) -> Row {
        let mut cells = [BLANK; 5];
        match self {
            Digits::Split => {
                for (i, cell) in cells.iter_mut().take(4).enumerate() {
                    let high = packet[3 + i];
                    let low = packet[4 + i];
                    *cell = Cell {
                        glyph: Glyph::from_segments((high & 0xF0) | (low & 0x0F)),
                        // Byte 3 bit 4 is the sign, not a point.
                        dp: i > 0 && high & glyph::DP != 0,
                    };
                }
                Row {
                    cells,
                    len: 4,
                    negative: packet[3] & glyph::DP != 0,
                }
            }
            Digits::Wide => {
                // A half digit, drawn with segments b and g (spec §6.3).
                if packet[13] & LEADING_ONE == LEADING_ONE {
                    cells[0].glyph = Glyph::Digit(1);
                }
                for (cell, &byte) in cells[1..].iter_mut().zip(packet[9..=12].iter().rev()) {
                    *cell = Cell {
                        glyph: Glyph::from_segments(byte),
                        dp: byte & glyph::DP != 0,
                    };
                }
                Row {
                    cells,
                    len: 5,
                    negative: packet[13] & MINUS_BIG != 0,
                }
            }
        }
    }

    /// The secondary display's row, on a layout that has one.
    fn secondary(self, packet: &[u8]) -> Option<Row> {
        match self {
            Digits::Split => None,
            Digits::Wide => {
                let mut cells = [BLANK; 5];
                let bytes = packet[5..=8].iter().rev();
                for (i, (cell, &byte)) in cells.iter_mut().zip(bytes).enumerate() {
                    *cell = Cell {
                        glyph: Glyph::from_segments(byte),
                        // Byte 8 bit 4 is the sign, not a point.
                        dp: i > 0 && byte & glyph::DP != 0,
                    };
                }
                Some(Row {
                    cells,
                    len: 4,
                    negative: packet[8] & glyph::DP != 0,
                })
            }
        }
    }

    /// Every row the layout carries.
    fn rows(self, packet: &[u8]) -> impl Iterator<Item = Row> {
        std::iter::once(self.main(packet)).chain(self.secondary(packet))
    }

    /// Report a leading "1" drawn with one of its two segments: the apps
    /// take it only with both (spec §6.3).
    fn check(self, unrecognised: Unrecognised, packet: &[u8]) {
        if self == Digits::Wide && !matches!(packet[13] & LEADING_ONE, 0 | LEADING_ONE) {
            unrecognised.report("leading digit");
        }
    }
}

/// Type 4 byte 13: the main minus, and the leading "1" (spec §6.3).
const MINUS_BIG: u8 = 0x80;
const LEADING_ONE: u8 = 0x0C;

/// One packet layout: a type byte, and what its bytes carry.
pub(crate) struct Layout {
    pub(super) type_byte: u8,
    /// The registry entry for this layout, which reports name.
    pub(crate) id: &'static str,
    /// The entry's display name, and the profile's model name.
    pub(crate) name: &'static str,
    digits: Digits,
    bits: &'static [Bit],
    /// Whether a prefix picks the reading's mode, as on a meter whose mV,
    /// mA and µA are positions of their own: the ZT-300AB's dial has them
    /// (spec §7.5, its "mV" and "µ/m A" groups), and the ZT-5566SE's mV and
    /// mA buttons select ranges of their own (ZT-5566SE manual p.11-12).
    /// Where the prefix is only an
    /// auto-ranging step, the mode stays the base unit's, so an auto-ranging
    /// current does not start a new series at each step.
    prefix_names_position: bool,
    /// Most sub-values one packet carries.
    pub(crate) max_aux_values: usize,
}

/// Type 3, named for the ZT-300AB (spec §7.1).
pub(crate) static ZT300AB: Layout = Layout {
    type_byte: 3,
    id: "zt300ab",
    name: "ZT-300AB / AN9002",
    digits: Digits::Split,
    bits: &[
        bit(3, 3, Meaning::Continuity),
        // The Bluetooth icon, lit in every community capture (§11.4).
        bit(3, 2, Meaning::Silent),
        bit(3, 1, Meaning::Rel),
        bit(3, 0, Meaning::LowBattery),
        bit(7, 7, Meaning::Diode),
        bit(7, 6, Meaning::Unit(Unit::Celsius)),
        bit(7, 5, Meaning::Unit(Unit::Fahrenheit)),
        bit(7, 4, Meaning::Hold),
        bit(8, 7, Meaning::Prefix(Prefix::Nano, FARAD)),
        bit(8, 6, Meaning::Prefix(Prefix::Milli, FARAD)),
        bit(8, 5, Meaning::Prefix(Prefix::Micro, FARAD)),
        bit(8, 4, Meaning::Unit(Unit::Farad)),
        bit(8, 3, Meaning::Ac),
        bit(8, 2, Meaning::Unit(Unit::Percent)),
        bit(8, 1, Meaning::Min),
        bit(8, 0, Meaning::Max),
        bit(9, 7, Meaning::Unit(Unit::Amp)),
        bit(9, 6, Meaning::Dc),
        bit(9, 5, Meaning::Prefix(Prefix::Milli, VOLT)),
        bit(9, 4, Meaning::Unit(Unit::Volt)),
        bit(9, 3, Meaning::Prefix(Prefix::Mega, OHM_HZ)),
        bit(9, 2, Meaning::Prefix(Prefix::Kilo, OHM_HZ)),
        bit(9, 1, Meaning::Unit(Unit::Ohm)),
        bit(9, 0, Meaning::Unit(Unit::Hertz)),
        // Never set in community captures, a TRUE RMS frame included
        // (§7.1, §11.4).
        silent(10, 0xF0),
        bit(10, 3, Meaning::Prefix(Prefix::Milli, AMP)),
        bit(10, 2, Meaning::Prefix(Prefix::Micro, AMP)),
        // MANUAL, never set even on a manual range (§11.4); AUTO clearing is
        // what says the range is manual.
        bit(10, 1, Meaning::Silent),
        bit(10, 0, Meaning::AutoRange),
    ],
    prefix_names_position: true,
    max_aux_values: 0,
};

/// Type 4, named for the ZT-5566 family (spec §7.4).
pub(crate) static ZT5566SE: Layout = Layout {
    type_byte: 4,
    id: "zt5566se",
    name: "ZT-5566SE / AN999S",
    digits: Digits::Wide,
    bits: &[
        // `vfc`, never set in the community log (§11.4).
        bit(3, 7, Meaning::Silent),
        bit(3, 6, Meaning::Diode),
        bit(3, 5, Meaning::Continuity),
        bit(3, 4, Meaning::Rel),
        // `l1_power`, toggling in long V DC runs (§11.4).
        bit(3, 3, Meaning::Silent),
        bit(3, 2, Meaning::AutoRange),
        // MANU: AUTO clearing is what says the range is manual.
        bit(3, 1, Meaning::Silent),
        bit(4, 7, Meaning::SecondaryPercent),
        bit(4, 6, Meaning::SecondaryHertz),
        bit(4, 5, Meaning::SecondaryKilo),
        bit(4, 4, Meaning::Unit(Unit::Volt)),
        bit(4, 3, Meaning::Hold),
        bit(4, 2, Meaning::Peak),
        bit(4, 1, Meaning::Max),
        bit(4, 0, Meaning::Min),
        bit(13, 6, Meaning::Ac),
        // Bit 5 is set but at exactly zero (§11.4) and bit 4 (the colon to
        // the apps) is the bar graph's first segment (§11.3 D1). Bit 0 is
        // used by neither app and open (§7.4, §10.10): reported.
        silent(13, 0x30),
        bit(13, 1, Meaning::Dc),
        // The analog bar graph, and byte 18 bit 7 always set (§11.3 D1,
        // §11.4).
        silent(14, 0xFF),
        silent(15, 0xFF),
        bit(16, 7, Meaning::Unit(Unit::Hertz)),
        bit(16, 6, Meaning::Unit(Unit::Ohm)),
        bit(16, 5, Meaning::Prefix(Prefix::Kilo, OHM_HZ)),
        bit(16, 4, Meaning::Prefix(Prefix::Mega, OHM_HZ)),
        silent(16, 0x0F),
        silent(17, 0xF0),
        bit(17, 3, Meaning::Prefix(Prefix::Nano, VOLT_AMP_FARAD)),
        bit(17, 2, Meaning::Prefix(Prefix::Milli, VOLT_AMP_FARAD)),
        bit(17, 1, Meaning::Prefix(Prefix::Micro, VOLT_AMP_FARAD)),
        bit(17, 0, Meaning::Unit(Unit::Farad)),
        silent(18, 0xEF),
        bit(18, 4, Meaning::Unit(Unit::Amp)),
    ],
    prefix_names_position: true,
    max_aux_values: 1,
};

/// Type 1, named for the ZT-5BQ clamp (spec §7.2).
pub(crate) static ZT5BQ: Layout = Layout {
    type_byte: 1,
    id: "zt5bq",
    name: "ZT-5BQ / ST207",
    digits: Digits::Split,
    bits: &[
        bit(3, 3, Meaning::Continuity),
        // The Bluetooth icon: set in every community notification, 0 V
        // included, so not a high-voltage mark (§11.3 D6).
        bit(3, 2, Meaning::Silent),
        bit(3, 1, Meaning::Hold),
        bit(3, 0, Meaning::LowBattery),
        bit(7, 7, Meaning::Ac),
        bit(7, 6, Meaning::Dc),
        bit(7, 5, Meaning::Unit(Unit::Volt)),
        bit(7, 4, Meaning::Prefix(Prefix::Nano, VOLT_AMP_FARAD)),
        bit(8, 7, Meaning::Prefix(Prefix::Mega, OHM_HZ)),
        bit(8, 6, Meaning::Prefix(Prefix::Milli, VOLT_AMP_FARAD)),
        bit(8, 5, Meaning::Prefix(Prefix::Kilo, OHM_HZ)),
        bit(8, 4, Meaning::Unit(Unit::Ohm)),
        bit(8, 3, Meaning::Prefix(Prefix::Micro, VOLT_AMP_FARAD)),
        bit(8, 2, Meaning::Unit(Unit::Amp)),
        bit(8, 1, Meaning::Diode),
        bit(8, 0, Meaning::Unit(Unit::Farad)),
        // `power`, set in every community notification (§11.4).
        bit(9, 7, Meaning::Silent),
        bit(9, 6, Meaning::Peak),
        bit(9, 5, Meaning::Unit(Unit::Percent)),
        bit(9, 4, Meaning::Inrush),
        bit(9, 3, Meaning::Unit(Unit::Celsius)),
        bit(9, 2, Meaning::Unit(Unit::Fahrenheit)),
        bit(9, 1, Meaning::Unit(Unit::Hertz)),
        bit(9, 0, Meaning::Rel),
    ],
    // An auto-ranging clamp: a prefix is a range step (spec §1).
    prefix_names_position: false,
    max_aux_values: 0,
};

/// Type 2, named for the ZT-5B (spec §7.3).
pub(crate) static ZT5B: Layout = Layout {
    type_byte: 2,
    id: "zt5b",
    name: "ZT-5B / V05B",
    digits: Digits::Split,
    bits: &[
        bit(3, 3, Meaning::Continuity),
        // Named `over_vol` by the app; community captures have it set at
        // 180 and 233 V AC and clear at low voltage (§11.2).
        bit(3, 2, Meaning::OverVoltage),
        bit(3, 1, Meaning::Hold),
        bit(3, 0, Meaning::LowBattery),
        // The Bluetooth icon, set in every community notification, and
        // `power`, never set (§11.4). Bits 5-4 are used by neither app and
        // never set: reported.
        bit(7, 7, Meaning::Silent),
        bit(7, 6, Meaning::Silent),
        bit(8, 7, Meaning::Prefix(Prefix::Micro, VOLT_AMP_FARAD)),
        bit(8, 6, Meaning::Unit(Unit::Amp)),
        bit(8, 5, Meaning::Diode),
        bit(8, 4, Meaning::Unit(Unit::Farad)),
        bit(8, 3, Meaning::Ac),
        bit(8, 2, Meaning::Dc),
        bit(8, 1, Meaning::Unit(Unit::Volt)),
        bit(8, 0, Meaning::Prefix(Prefix::Nano, VOLT_AMP_FARAD)),
        bit(9, 7, Meaning::Unit(Unit::Celsius)),
        bit(9, 6, Meaning::Unit(Unit::Fahrenheit)),
        bit(9, 5, Meaning::Unit(Unit::Hertz)),
        bit(9, 4, Meaning::Unit(Unit::Percent)),
        bit(9, 3, Meaning::Prefix(Prefix::Mega, OHM_HZ)),
        bit(9, 2, Meaning::Prefix(Prefix::Milli, VOLT_AMP_FARAD)),
        bit(9, 1, Meaning::Prefix(Prefix::Kilo, OHM_HZ)),
        bit(9, 0, Meaning::Unit(Unit::Ohm)),
    ],
    // An auto-only pocket meter: a prefix is a range step (spec §1).
    prefix_names_position: false,
    max_aux_values: 0,
};

/// Every layout, by type byte: one for each type the apps define (spec §1).
pub(super) static LAYOUTS: &[&Layout] = &[&ZT300AB, &ZT5566SE, &ZT5BQ, &ZT5B];

/// The layout a type byte selects; `None` for a type no app defines.
pub(super) fn for_type(type_byte: u8) -> Option<&'static Layout> {
    LAYOUTS
        .iter()
        .copied()
        .find(|layout| layout.type_byte == type_byte)
}

/// The layout of a whole descrambled packet: header, a type byte with a
/// layout, and that type's length (spec §5).
pub(super) fn layout_of(packet: &[u8]) -> Result<&'static Layout> {
    if packet.len() <= TYPE_AT || !packet.starts_with(&HEADER) {
        return Err(Error::invalid_response("zotek: no 5A A5 header", packet));
    }
    let type_byte = packet[TYPE_AT];
    let (Some(layout), Some(len)) = (for_type(type_byte), frame::packet_len(type_byte)) else {
        return Err(Error::invalid_response(
            format!("zotek: no layout has type byte {type_byte:#04x}"),
            packet,
        ));
    };
    if packet.len() != len {
        return Err(Error::invalid_response(
            format!(
                "zotek: type-{type_byte} packet of {} bytes, expected {len}",
                packet.len()
            ),
            packet,
        ));
    }
    Ok(layout)
}

/// The layout of `packet` if it looks like one a meter sends: whole, of a
/// known type, and every digit a glyph the spec lists (spec §6.1),
/// which the words are built from too.
pub(super) fn plausible(packet: &[u8]) -> Option<&'static Layout> {
    let layout = layout_of(packet).ok()?;
    layout
        .digits
        .rows(packet)
        .all(|row| {
            row.cells()
                .iter()
                .all(|c| !matches!(c.glyph, Glyph::Unknown(_)))
        })
        .then_some(layout)
}

/// AC/DC, as the annunciators show it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Coupling {
    #[default]
    None,
    Dc,
    Ac,
    /// Both lit: no source documents it.
    Both,
}

/// The measuring function the annunciators show, with the code
/// [`Measurement::mode_raw`] carries for it.
///
/// `mode_raw` is this code, plus `0x10` for DC and `0x20` for AC on the
/// volt and amp functions and `0x40` for PEAK — an internal number, stable
/// for bug reports, not a byte the meter sends.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Function {
    /// No function annunciator lit.
    #[default]
    None = 0x00,
    Volts = 0x01,
    Millivolts = 0x02,
    Amps = 0x03,
    Milliamps = 0x04,
    Microamps = 0x05,
    Ohms = 0x06,
    Continuity = 0x07,
    Diode = 0x08,
    Capacitance = 0x09,
    Frequency = 0x0A,
    Duty = 0x0B,
    Celsius = 0x0C,
    Fahrenheit = 0x0D,
    Ncv = 0x0E,
    Inrush = 0x0F,
}

impl Function {
    /// The mode name, in the vocabulary the other families use.
    fn mode(self, coupling: Coupling) -> &'static str {
        let coupled = |names: [&'static str; 4]| match coupling {
            Coupling::None => names[0],
            Coupling::Dc => names[1],
            Coupling::Ac => names[2],
            Coupling::Both => names[3],
        };
        match self {
            Function::None => "Auto",
            Function::Volts => coupled(["V", "DC V", "AC V", "AC+DC V"]),
            Function::Millivolts => coupled(["mV", "DC mV", "AC mV", "AC+DC mV"]),
            Function::Amps => coupled(["A", "DC A", "AC A", "AC+DC A"]),
            Function::Milliamps => coupled(["mA", "DC mA", "AC mA", "AC+DC mA"]),
            Function::Microamps => coupled(["µA", "DC µA", "AC µA", "AC+DC µA"]),
            Function::Ohms => "Ω",
            Function::Continuity => "Continuity",
            Function::Diode => "Diode",
            Function::Capacitance => "Capacitance",
            Function::Frequency => "Hz",
            Function::Duty => "Duty %",
            Function::Celsius => "°C",
            Function::Fahrenheit => "°F",
            Function::Ncv => "NCV",
            Function::Inrush => "Inrush",
        }
    }

    /// Whether AC/DC is part of the function's mode.
    fn coupled(self) -> bool {
        matches!(
            self,
            Function::Volts
                | Function::Millivolts
                | Function::Amps
                | Function::Milliamps
                | Function::Microamps
        )
    }

    fn mode_raw(self, coupling: Coupling) -> u16 {
        let coupling_bits = match coupling {
            _ if !self.coupled() => 0,
            Coupling::None => 0,
            Coupling::Dc => 0x10,
            Coupling::Ac => 0x20,
            Coupling::Both => 0x30,
        };
        self as u16 | coupling_bits
    }
}

/// The annunciators a packet lights.
#[derive(Default)]
struct Lit {
    units: u8,
    prefixes: [Option<(Prefix, &'static [Unit])>; 8],
    ac: bool,
    dc: bool,
    diode: bool,
    continuity: bool,
    hold: bool,
    rel: bool,
    auto_range: bool,
    min: bool,
    max: bool,
    low_battery: bool,
    peak: bool,
    inrush: bool,
    over_voltage: bool,
    secondary_percent: bool,
    secondary_hertz: bool,
    secondary_kilo: bool,
}

impl Lit {
    fn read(layout: &Layout, packet: &[u8]) -> Self {
        let mut lit = Lit::default();
        let mut prefixes = 0;
        for b in layout.bits {
            if packet[b.byte] & b.mask == 0 {
                continue;
            }
            match b.meaning {
                Meaning::Unit(unit) => lit.units |= unit.bit(),
                Meaning::Prefix(prefix, units) => {
                    // More prefix bits than any layout has cannot be lit.
                    if let Some(slot) = lit.prefixes.get_mut(prefixes) {
                        *slot = Some((prefix, units));
                        prefixes += 1;
                    }
                }
                Meaning::Ac => lit.ac = true,
                Meaning::Dc => lit.dc = true,
                Meaning::Diode => lit.diode = true,
                Meaning::Continuity => lit.continuity = true,
                Meaning::Hold => lit.hold = true,
                Meaning::Rel => lit.rel = true,
                Meaning::AutoRange => lit.auto_range = true,
                Meaning::Min => lit.min = true,
                Meaning::Max => lit.max = true,
                Meaning::LowBattery => lit.low_battery = true,
                Meaning::Peak => lit.peak = true,
                Meaning::Inrush => lit.inrush = true,
                Meaning::OverVoltage => lit.over_voltage = true,
                Meaning::SecondaryPercent => lit.secondary_percent = true,
                Meaning::SecondaryHertz => lit.secondary_hertz = true,
                Meaning::SecondaryKilo => lit.secondary_kilo = true,
                Meaning::Silent => {}
            }
        }
        lit
    }

    fn has(&self, unit: Unit) -> bool {
        self.units & unit.bit() != 0
    }

    /// The unit the reading is in.
    fn unit(&self) -> Option<Unit> {
        Unit::PRIORITY.into_iter().find(|u| self.has(*u))
    }

    /// The prefix on `unit`. A lit prefix that attaches to no lit unit, or
    /// two on one unit, is reported (spec §7.5).
    fn prefix(&self, unrecognised: Unrecognised, unit: Option<Unit>) -> Option<Prefix> {
        let mut found = None;
        for (prefix, units) in self.prefixes.iter().flatten() {
            if !units.iter().any(|u| self.has(*u)) {
                unrecognised.report("prefix annunciator");
            } else if unit.is_some_and(|u| units.contains(&u)) {
                if found.is_some() {
                    unrecognised.report("prefix annunciators");
                } else {
                    found = Some(*prefix);
                }
            }
        }
        found
    }

    fn coupling(&self) -> Coupling {
        match (self.dc, self.ac) {
            (false, false) => Coupling::None,
            (true, false) => Coupling::Dc,
            (false, true) => Coupling::Ac,
            (true, true) => Coupling::Both,
        }
    }

    /// The function the annunciators show: temperature, diode and
    /// continuity by their own symbols, the rest by the unit.
    fn function(&self, layout: &Layout, unit: Option<Unit>, prefix: Option<Prefix>) -> Function {
        let by_prefix = if layout.prefix_names_position {
            prefix
        } else {
            None
        };
        match unit {
            // The clamp's INRUSH, whatever else is lit (spec §7.2).
            _ if self.inrush => Function::Inrush,
            Some(Unit::Celsius) => Function::Celsius,
            Some(Unit::Fahrenheit) => Function::Fahrenheit,
            _ if self.diode => Function::Diode,
            _ if self.continuity => Function::Continuity,
            Some(Unit::Percent) => Function::Duty,
            Some(Unit::Hertz) => Function::Frequency,
            Some(Unit::Farad) => Function::Capacitance,
            Some(Unit::Ohm) => Function::Ohms,
            Some(Unit::Amp) => match by_prefix {
                Some(Prefix::Milli) => Function::Milliamps,
                Some(Prefix::Micro) => Function::Microamps,
                _ => Function::Amps,
            },
            Some(Unit::Volt) => match by_prefix {
                Some(Prefix::Milli) => Function::Millivolts,
                _ => Function::Volts,
            },
            None => Function::None,
        }
    }
}

impl Layout {
    /// Report any set bit the layout does not list (spec §7: "every bit not
    /// listed as a digit or a flag is read by neither app").
    fn report_stray_bits(&self, unrecognised: Unrecognised, packet: &[u8]) {
        let stray = (TYPE_AT + 1..packet.len()).any(|byte| {
            let known = self
                .bits
                .iter()
                .filter(|b| b.byte == byte)
                .fold(self.digits.mask(byte), |mask, b| mask | b.mask);
            packet[byte] & !known != 0
        });
        if stray {
            unrecognised.report("annunciator bits");
        }
    }

    /// A descrambled packet of a types 1-3 layout showing `cells`, most
    /// significant first, with the minus if `negative`, and each of `lit`
    /// lit: the inverse of [`decode`], for the simulated meter. A prefix is
    /// found by the prefix alone, whatever units it attaches to. `None` for
    /// type 4's digits, or a meaning the layout has no bit for; a silent bit
    /// is never looked up, as several share the meaning.
    pub(super) fn draw(
        &self,
        cells: &[Cell; 4],
        negative: bool,
        lit: &[Meaning],
    ) -> Option<Vec<u8>> {
        if self.digits != Digits::Split {
            return None;
        }
        let mut packet = vec![0u8; frame::packet_len(self.type_byte)?];
        packet[..HEADER.len()].copy_from_slice(&HEADER);
        packet[TYPE_AT] = self.type_byte;
        // Digit i's high nibble in byte 3+i and its low nibble in byte 4+i;
        // byte 3 bit 4 is the minus and bytes 4-6 bit 4 the points
        // (spec §6.2).
        for (i, cell) in cells.iter().enumerate() {
            let segments = cell.glyph.segments();
            packet[3 + i] |= segments & 0xF0;
            packet[4 + i] |= segments & 0x0F;
            if cell.dp && i > 0 {
                packet[3 + i] |= glyph::DP;
            }
        }
        if negative {
            packet[3] |= glyph::DP;
        }
        for &meaning in lit {
            let bit = self.bits.iter().find(|b| match (b.meaning, meaning) {
                (_, Meaning::Silent) => false,
                (Meaning::Prefix(have, _), Meaning::Prefix(want, _)) => have == want,
                (have, want) => have == want,
            })?;
            packet[bit.byte] |= bit.mask;
        }
        Some(packet)
    }
}

/// The secondary display as a sub-value, where its unit is lit (spec
/// §6.3, §7.4): the frequency in V, the duty cycle in frequency.
fn secondary(
    layout: &Layout,
    lit: &Lit,
    unrecognised: Unrecognised,
    packet: &[u8],
) -> Option<AuxValue> {
    let row = layout.digits.secondary(packet)?;
    let (label, unit) = match (lit.secondary_hertz, lit.secondary_percent) {
        (false, false) => {
            // Nothing to label: blank, as community captures show it.
            let drawn = row.negative || row.cells().iter().any(|c| c.glyph != Glyph::Blank);
            if drawn || lit.secondary_kilo {
                unrecognised.report("secondary display");
            }
            return None;
        }
        (true, percent) => {
            if percent {
                unrecognised.report("secondary annunciators");
            }
            let unit = if lit.secondary_kilo { "kHz" } else { "Hz" };
            ("Frequency", unit)
        }
        (false, true) => {
            if lit.secondary_kilo {
                unrecognised.report("secondary annunciators");
            }
            ("Duty", "%")
        }
    };
    // Nothing drawn: no sub-value, whatever unit is lit.
    if row.blank() {
        return None;
    }
    let readout = glyph::read(unrecognised, row.cells(), row.negative);
    let value = match readout.shown {
        Shown::Number(v) => MeasuredValue::Normal(v),
        Shown::Overload | Shown::Unrecognised => MeasuredValue::Overload,
        Shown::Auto | Shown::Ef | Shown::Dashes(_) => {
            unrecognised.report("secondary display text");
            MeasuredValue::Overload
        }
    };
    Some(AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Borrowed(unit),
        display_raw: Some(readout.text),
        elapsed_secs: None,
    })
}

/// The word `n` dashes spell.
fn dashes(n: u8) -> &'static str {
    match n {
        1 => "-",
        2 => "--",
        3 => "---",
        _ => "----",
    }
}

/// What a packet shows that some key codes follow (spec §8.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) struct Showing {
    /// The unit annunciator the reading is in.
    pub(super) unit: Option<Unit>,
    /// The function the reading is in, as its mode names it.
    pub(super) function: Function,
    pub(super) coupling: Coupling,
}

/// Decode one whole descrambled packet, by its own type byte.
pub(super) fn decode(packet: &[u8]) -> Result<Measurement> {
    decode_showing(packet).map(|(reading, _)| reading)
}

/// Whether the main display of a whole descrambled packet has a digit lit:
/// one with none has no reading, and [`decode`] refuses it. No section of
/// the spec shows a blank main display, so one is reported.
pub(super) fn shows_digits(packet: &[u8]) -> bool {
    let Ok(layout) = layout_of(packet) else {
        return false;
    };
    let blank = layout.digits.main(packet).blank();
    if blank {
        Unrecognised {
            id: layout.id,
            packet,
        }
        .report(BLANK_MAIN);
    }
    !blank
}

/// What a blank main display is reported as.
const BLANK_MAIN: &str = "blank main display";

/// [`decode`], and what the packet shows for the keys.
pub(super) fn decode_showing(packet: &[u8]) -> Result<(Measurement, Showing)> {
    let layout = layout_of(packet)?;
    let unrecognised = Unrecognised {
        id: layout.id,
        packet,
    };
    // A main display with no digit lit, every glyph a blank (`00`, spec
    // §6.1), has no reading to give: the packet is reported and refused,
    // before its other bits are.
    let row = layout.digits.main(packet);
    if row.blank() {
        unrecognised.report(BLANK_MAIN);
        return Err(Error::invalid_response(
            "zotek: no digit lit on the main display",
            packet,
        ));
    }
    layout.report_stray_bits(unrecognised, packet);
    layout.digits.check(unrecognised, packet);

    let lit = Lit::read(layout, packet);
    let readout = glyph::read(unrecognised, row.cells(), row.negative);
    let unit = lit.unit();
    let prefix = lit.prefix(unrecognised, unit);
    let shown_function = lit.function(layout, unit, prefix);

    let (value, function) = match readout.shown {
        Shown::Number(v) => (MeasuredValue::Normal(v), shown_function),
        Shown::Overload => (MeasuredValue::Overload, shown_function),
        Shown::Auto => (MeasuredValue::NoReading("Auto"), shown_function),
        // NCV, as both apps take it while no function is lit (spec §6.4).
        Shown::Ef if shown_function == Function::None => {
            (MeasuredValue::NcvLevel(0), Function::Ncv)
        }
        Shown::Ef => {
            unrecognised.report("display text");
            (MeasuredValue::Overload, shown_function)
        }
        // With INRUSH lit, the dashes are the wait for a current to catch
        // (spec §11.3 D3).
        Shown::Dashes(n) if shown_function == Function::Inrush => {
            (MeasuredValue::NoReading(dashes(n)), shown_function)
        }
        // NCV fills one to four dashes from the left (spec §11.4), taken
        // while no function is lit, as EF is (spec §6.4).
        Shown::Dashes(n) if shown_function == Function::None => {
            (MeasuredValue::NcvLevel(n), Function::Ncv)
        }
        Shown::Dashes(_) => {
            unrecognised.report("display text");
            (MeasuredValue::Overload, shown_function)
        }
        // Reported by the reader; shown as OL, as the other families do.
        Shown::Unrecognised => (MeasuredValue::Overload, shown_function),
    };

    let coupling = lit.coupling();
    if function.coupled() && coupling == Coupling::Both {
        unrecognised.report("ac/dc annunciators");
    }
    let mode: Cow<'static, str> = match (function, &value) {
        (Function::None, MeasuredValue::NoReading(_)) => Cow::Borrowed(function.mode(coupling)),
        (Function::None, _) => {
            unrecognised.report("function annunciators");
            unknown_mode(Function::None as u8)
        }
        // PEAK says the reading is a held peak, not whether a maximum or a
        // minimum (spec §7.2, §11.2).
        _ if lit.peak && function != Function::Ncv => {
            Cow::Owned(format!("{} peak", function.mode(coupling)))
        }
        _ => Cow::Borrowed(function.mode(coupling)),
    };
    let peak_bit = if lit.peak { 0x40 } else { 0 };
    let unit_text = match (function, unit) {
        (Function::Ncv, _) | (_, None) => "",
        (_, Some(unit)) => unit_str(prefix, unit),
    };

    let flags = StatusFlags {
        hold: lit.hold,
        rel: lit.rel,
        auto_range: lit.auto_range,
        min: lit.min,
        max: lit.max,
        low_battery: lit.low_battery,
        hv_warning: lit.over_voltage,
        dc: function.coupled() && lit.dc,
        ..StatusFlags::default()
    };

    let showing = Showing {
        unit,
        function,
        coupling,
    };
    let reading = Measurement {
        mode,
        mode_raw: function.mode_raw(coupling) | peak_bit,
        value,
        unit: Cow::Borrowed(unit_text),
        display_raw: Some(readout.text),
        flags,
        aux_values: secondary(layout, &lit, unrecognised, packet)
            .into_iter()
            .collect(),
        ..Measurement::from_payload(packet)
    };
    Ok((reading, showing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::zotek::frame::tests::{EXAMPLES, pseudo_random};
    use crate::protocol::zotek::glyph::tests::segments;

    /// Bytes 3-7 of a types 1-3 packet showing `glyphs` (four characters,
    /// as `segments` spells them), the point before cell `dp_at`, and the
    /// minus.
    fn split_digits(glyphs: &str, dp_at: Option<usize>, negative: bool) -> [u8; 5] {
        let g: Vec<u8> = glyphs.chars().map(segments).collect();
        assert_eq!(g.len(), 4, "{glyphs:?}");
        let dp = |i: usize| if dp_at == Some(i) { glyph::DP } else { 0 };
        [
            (g[0] & 0xF0) | if negative { glyph::DP } else { 0 },
            (g[0] & 0x0F) | (g[1] & 0xF0) | dp(1),
            (g[1] & 0x0F) | (g[2] & 0xF0) | dp(2),
            (g[2] & 0x0F) | (g[3] & 0xF0) | dp(3),
            g[3] & 0x0F,
        ]
    }

    /// A descrambled types 1-3 packet: header, `type_byte`, the digits, and
    /// each `(byte, bits)` of `flags` ORed in.
    fn split_packet(
        type_byte: u8,
        glyphs: &str,
        dp_at: Option<usize>,
        negative: bool,
        flags: &[(usize, u8)],
    ) -> Vec<u8> {
        let len = frame::packet_len(type_byte).expect("a layout's type");
        let mut packet = vec![0u8; len];
        packet[..3].copy_from_slice(&[0x5A, 0xA5, type_byte]);
        packet[3..8].copy_from_slice(&split_digits(glyphs, dp_at, negative));
        for &(byte, bits) in flags {
            packet[byte] |= bits;
        }
        packet
    }

    /// Decode, asserting nothing was reported.
    fn quiet(packet: &[u8]) -> Measurement {
        let (m, reports) = capture_reports(|| decode(packet));
        assert!(reports.is_empty(), "{packet:02X?}: {reports:?}");
        m.unwrap()
    }

    /// Decode, returning what was reported.
    fn reported(packet: &[u8]) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(packet));
        (m.unwrap(), reports)
    }

    fn flags_set(m: &Measurement) -> Vec<&'static str> {
        m.flags
            .as_pairs()
            .iter()
            .filter(|(_, set)| *set)
            .map(|(name, _)| *name)
            .collect()
    }

    fn value(m: &Measurement) -> f64 {
        match m.value {
            MeasuredValue::Normal(v) => v,
            ref other => panic!("not a number: {other:?}"),
        }
    }

    // --- Type 3 (ZT-300AB) ------------------------------------------------

    /// DC and V lit: the base the flag tests switch one more bit on over.
    const T3_DCV: &[(usize, u8)] = &[(9, 0x50)];

    fn t3(glyphs: &str, dp_at: Option<usize>, flags: &[(usize, u8)]) -> Vec<u8> {
        split_packet(3, glyphs, dp_at, false, flags)
    }

    /// Spec §9: −12.34 V DC, AUTO, the Bluetooth icon lit.
    #[test]
    fn type3_worked_example() {
        let m = quiet(EXAMPLES[0].1);
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.mode_raw, 0x11);
        assert_eq!(m.unit, "V");
        assert_eq!(m.display_raw.as_deref(), Some("-12.34"));
        assert!((value(&m) + 12.34).abs() < 1e-9);
        assert_eq!(flags_set(&m), ["auto_range", "dc"]);
        assert_eq!(m.raw_payload, EXAMPLES[0].1);
    }

    /// Every §7.1 bit, one at a time over a function where it needs one:
    /// (bits, mode, unit, flags set).
    #[test]
    fn type3_every_annunciator() {
        type Case = (
            &'static [(usize, u8)],
            &'static str,
            &'static str,
            &'static [&'static str],
        );
        let cases: &[Case] = &[
            (&[(3, 0x08), (9, 0x02)], "Continuity", "Ω", &[]),
            (&[(3, 0x04), (9, 0x50)], "DC V", "V", &["dc"]),
            (&[(3, 0x02), (9, 0x50)], "DC V", "V", &["rel", "dc"]),
            (&[(3, 0x01), (9, 0x50)], "DC V", "V", &["low_battery", "dc"]),
            (&[(7, 0x80), (9, 0x10)], "Diode", "V", &[]),
            (&[(7, 0x40)], "°C", "°C", &[]),
            (&[(7, 0x20)], "°F", "°F", &[]),
            (&[(7, 0x10), (9, 0x50)], "DC V", "V", &["hold", "dc"]),
            (&[(8, 0x90)], "Capacitance", "nF", &[]),
            (&[(8, 0x50)], "Capacitance", "mF", &[]),
            (&[(8, 0x30)], "Capacitance", "µF", &[]),
            (&[(8, 0x10)], "Capacitance", "F", &[]),
            (&[(8, 0x08), (9, 0x10)], "AC V", "V", &[]),
            (&[(8, 0x04)], "Duty %", "%", &[]),
            (&[(8, 0x02), (9, 0x50)], "DC V", "V", &["min", "dc"]),
            (&[(8, 0x01), (9, 0x50)], "DC V", "V", &["max", "dc"]),
            (&[(9, 0xC0)], "DC A", "A", &["dc"]),
            (&[(9, 0xC0), (10, 0x08)], "DC mA", "mA", &["dc"]),
            (&[(9, 0x80), (8, 0x08), (10, 0x04)], "AC µA", "µA", &[]),
            (&[(9, 0x70)], "DC mV", "mV", &["dc"]),
            (&[(9, 0x0A)], "Ω", "MΩ", &[]),
            (&[(9, 0x06)], "Ω", "kΩ", &[]),
            (&[(9, 0x02)], "Ω", "Ω", &[]),
            (&[(9, 0x01)], "Hz", "Hz", &[]),
            (&[(9, 0x05)], "Hz", "kHz", &[]),
            (&[(9, 0x09)], "Hz", "MHz", &[]),
            (&[(9, 0x50), (10, 0xF0)], "DC V", "V", &["dc"]),
            (&[(9, 0x50), (10, 0x02)], "DC V", "V", &["dc"]),
            (&[(9, 0x50), (10, 0x01)], "DC V", "V", &["auto_range", "dc"]),
        ];
        for &(flags, mode, unit, set) in cases {
            let m = quiet(&t3("1234", Some(2), flags));
            assert_eq!(m.mode, mode, "{flags:02X?}");
            assert_eq!(m.unit, unit, "{flags:02X?}");
            assert_eq!(flags_set(&m), set, "{flags:02X?}");
            assert_eq!(m.display_raw.as_deref(), Some("12.34"));
            assert_eq!(value(&m), 12.34);
        }
    }

    /// Spec §9's special displays, bytes 3-7 as given there.
    #[test]
    fn type3_worked_words() {
        let with = |digits: [u8; 5], flags: &[(usize, u8)]| {
            let mut packet = t3("    ", None, flags);
            packet[3..8].copy_from_slice(&digits);
            quiet(&packet)
        };
        let auto = with([0xE0, 0x2E, 0x63, 0x25, 0x07], &[]);
        assert!(matches!(auto.value, MeasuredValue::NoReading("Auto")));
        assert_eq!(auto.mode, "Auto");
        assert_eq!(auto.mode_raw, 0x00);
        assert_eq!(auto.display_raw.as_deref(), Some("Auto"));

        let ef = with([0x04, 0xE0, 0xE5, 0x04, 0x00], &[]);
        assert!(matches!(ef.value, MeasuredValue::NcvLevel(0)));
        assert_eq!((ef.mode.as_ref(), ef.unit.as_ref()), ("NCV", ""));
        assert_eq!(ef.mode_raw, 0x0E);

        let ol = with([0x00, 0xE0, 0x6B, 0x01, 0x00], &[(9, 0x02)]);
        assert!(matches!(ol.value, MeasuredValue::Overload));
        assert_eq!(ol.mode, "Ω");
        assert_eq!(ol.display_raw.as_deref(), Some(" 0L "));

        let dashes = with([0x00, 0x04, 0x04, 0x04, 0x04], &[]);
        assert!(matches!(dashes.value, MeasuredValue::NcvLevel(4)));
        assert_eq!(dashes.mode, "NCV");
    }

    /// Auto while a function is lit takes that function's mode.
    #[test]
    fn auto_takes_the_lit_functions_mode() {
        let m = quiet(&t3("Auto", None, &[(9, 0x10), (10, 0x01)]));
        assert!(matches!(m.value, MeasuredValue::NoReading("Auto")));
        assert_eq!(m.mode, "V");
        assert_eq!(m.unit, "V");
    }

    /// NCV fills one to four dashes from the left (spec §11.4).
    #[test]
    fn dashes_are_the_ncv_level() {
        for (row, level) in [("-   ", 1), ("--  ", 2), ("--- ", 3), ("----", 4)] {
            let m = quiet(&t3(row, None, &[]));
            assert!(
                matches!(m.value, MeasuredValue::NcvLevel(l) if l == level),
                "{row:?}"
            );
            assert_eq!(m.mode, "NCV");
        }
    }

    /// The three OL forms of spec §11.4, whatever the unit.
    #[test]
    fn every_ol_form_is_an_overload() {
        for (dp_at, unit_bits, unit, text) in [
            (Some(2), 0x0A, "MΩ", " 0.L "),
            (Some(1), 0x06, "kΩ", " .0L "),
            (Some(3), 0x02, "Ω", " 0L. "),
        ] {
            let m = quiet(&t3(" 0L ", dp_at, &[(9, unit_bits)]));
            assert!(matches!(m.value, MeasuredValue::Overload), "{text:?}");
            assert_eq!(m.unit, unit);
            assert_eq!(m.display_raw.as_deref(), Some(text));
        }
        let diode = quiet(&t3(" 0L ", Some(1), &[(7, 0x80), (9, 0x10)]));
        assert!(matches!(diode.value, MeasuredValue::Overload));
        assert_eq!(diode.mode, "Diode");
    }

    /// `10`: a blank digit carrying the sign or a point (spec
    /// Implementation Notes) reads as a blank with its sign or point.
    #[test]
    fn a_blank_carrying_a_sign_or_point() {
        let m = quiet(&split_packet(3, " 123", None, true, T3_DCV));
        assert_eq!(m.display_raw.as_deref(), Some("- 123"));
        assert_eq!(value(&m), -123.0);
        assert_eq!(m.raw_payload[3], 0x10);

        let m = quiet(&t3("1 23", Some(1), T3_DCV));
        assert_eq!(m.display_raw.as_deref(), Some("1. 23"));
        assert!((value(&m) - 1.023).abs() < 1e-9);
    }

    /// Two points: reported, and the leftmost kept (spec §6.2).
    #[test]
    fn two_points_report_and_keep_the_leftmost() {
        let mut packet = t3("1234", Some(1), T3_DCV);
        packet[6] |= glyph::DP;
        let (m, reports) = reported(&packet);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].starts_with("zt300ab: unrecognised decimal points"),
            "{reports:?}"
        );
        assert_eq!(m.display_raw.as_deref(), Some("1.234"));
    }

    #[test]
    fn an_unlisted_glyph_is_reported_and_shown_as_ol() {
        let mut packet = t3("1234", None, T3_DCV);
        packet[5] = (packet[5] & 0xF0) | 0x01;
        packet[4] &= 0x0F;
        let (m, reports) = reported(&packet);
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// EF is NCV only while no function is lit (spec §6.4).
    #[test]
    fn ef_with_a_function_lit_is_reported() {
        let (m, reports) = reported(&t3(" EF ", None, T3_DCV));
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// Dashes are NCV only while no function is lit, as EF is (spec §6.4).
    #[test]
    fn dashes_with_a_function_lit_are_reported() {
        let (m, reports) = reported(&t3("--  ", None, T3_DCV));
        assert!(matches!(m.value, MeasuredValue::Overload), "{:?}", m.value);
        assert_eq!(m.mode, "DC V");
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// A reading with no function lit has no mode to show.
    #[test]
    fn a_reading_with_no_function_is_reported() {
        let (m, reports) = reported(&t3("1234", None, &[]));
        assert_eq!(m.mode, "Unknown(0x00)");
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// A prefix lit on no unit of its group (spec §7.5).
    #[test]
    fn a_prefix_on_a_unit_outside_its_group_is_reported() {
        // Capacitance's n with V lit.
        let (m, reports) = reported(&t3("1234", None, &[(8, 0x80), (9, 0x50)]));
        assert_eq!(m.unit, "V");
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// AC and DC together: no source documents it.
    #[test]
    fn ac_and_dc_together_are_reported() {
        let (m, reports) = reported(&t3("1234", None, &[(8, 0x08), (9, 0x50)]));
        assert_eq!(m.mode, "AC+DC V");
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    // --- Type 4 (ZT-5566SE) ----------------------------------------------

    /// A descrambled type-4 packet: the main row's four glyphs (bytes 12 to
    /// 9) with the point before cell `dp_at`, a leading "1" and the minus;
    /// the secondary row's (bytes 8 to 5) likewise; `flags` ORed in.
    fn t4(
        main: &str,
        dp_at: Option<usize>,
        lead: bool,
        negative: bool,
        secondary: Option<(&str, Option<usize>, bool)>,
        flags: &[(usize, u8)],
    ) -> Vec<u8> {
        let mut packet = vec![0u8; 19];
        packet[..3].copy_from_slice(&[0x5A, 0xA5, 4]);
        for (i, c) in main.chars().enumerate() {
            let dp = if dp_at == Some(i) { glyph::DP } else { 0 };
            packet[12 - i] = segments(c) | dp;
        }
        if lead {
            packet[13] |= LEADING_ONE;
        }
        if negative {
            packet[13] |= MINUS_BIG;
        }
        if let Some((glyphs, dp_at, negative)) = secondary {
            for (i, c) in glyphs.chars().enumerate() {
                let dp = if dp_at == Some(i) { glyph::DP } else { 0 };
                packet[8 - i] = segments(c) | dp;
            }
            if negative {
                packet[8] |= glyph::DP;
            }
        }
        for &(byte, bits) in flags {
            packet[byte] |= bits;
        }
        packet
    }

    /// DC and V lit.
    const T4_DCV: &[(usize, u8)] = &[(4, 0x10), (13, 0x02)];

    /// Spec §9: main 1.2345 V DC, AUTO; secondary 50.00 Hz.
    #[test]
    fn type4_worked_example() {
        let m = quiet(EXAMPLES[3].1);
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.mode_raw, 0x11);
        assert_eq!(m.unit, "V");
        assert_eq!(m.display_raw.as_deref(), Some("1.2345"));
        assert!((value(&m) - 1.2345).abs() < 1e-9);
        assert_eq!(flags_set(&m), ["auto_range", "dc"]);
        assert_eq!(m.aux_values.len(), 1);
        let aux = &m.aux_values[0];
        assert_eq!(aux.label, "Frequency");
        assert_eq!(aux.unit, "Hz");
        assert_eq!(aux.display_raw.as_deref(), Some("50.00"));
        assert!(matches!(aux.value, MeasuredValue::Normal(v) if v == 50.0));
    }

    /// Every §7.4 bit, one at a time over a function where it needs one:
    /// (bits, mode, mode_raw, unit, flags set).
    #[test]
    fn type4_every_annunciator() {
        type Case = (
            &'static [(usize, u8)],
            &'static str,
            u16,
            &'static str,
            &'static [&'static str],
        );
        let cases: &[Case] = &[
            (
                &[(3, 0x80), (4, 0x10), (13, 0x02)],
                "DC V",
                0x11,
                "V",
                &["dc"],
            ),
            (&[(3, 0x40), (4, 0x10)], "Diode", 0x08, "V", &[]),
            (&[(3, 0x20), (16, 0x40)], "Continuity", 0x07, "Ω", &[]),
            (
                &[(3, 0x10), (4, 0x10), (13, 0x02)],
                "DC V",
                0x11,
                "V",
                &["rel", "dc"],
            ),
            (
                &[(3, 0x08), (4, 0x10), (13, 0x02)],
                "DC V",
                0x11,
                "V",
                &["dc"],
            ),
            (
                &[(3, 0x04), (4, 0x10), (13, 0x02)],
                "DC V",
                0x11,
                "V",
                &["auto_range", "dc"],
            ),
            (
                &[(3, 0x02), (4, 0x10), (13, 0x02)],
                "DC V",
                0x11,
                "V",
                &["dc"],
            ),
            (&[(4, 0x18), (13, 0x02)], "DC V", 0x11, "V", &["hold", "dc"]),
            (&[(4, 0x14), (13, 0x40)], "AC V peak", 0x61, "V", &[]),
            (&[(4, 0x12), (13, 0x02)], "DC V", 0x11, "V", &["max", "dc"]),
            (&[(4, 0x11), (13, 0x02)], "DC V", 0x11, "V", &["min", "dc"]),
            (&[(4, 0x10), (13, 0x40)], "AC V", 0x21, "V", &[]),
            (&[(4, 0x10), (13, 0x32)], "DC V", 0x11, "V", &["dc"]),
            (
                &[(4, 0x10), (13, 0x02), (14, 0xFF), (15, 0xFF)],
                "DC V",
                0x11,
                "V",
                &["dc"],
            ),
            (
                &[(4, 0x10), (13, 0x02), (16, 0x0F), (17, 0xF0), (18, 0xEF)],
                "DC V",
                0x11,
                "V",
                &["dc"],
            ),
            (&[(16, 0x80)], "Hz", 0x0A, "Hz", &[]),
            (&[(16, 0xA0)], "Hz", 0x0A, "kHz", &[]),
            (&[(16, 0x40)], "Ω", 0x06, "Ω", &[]),
            (&[(16, 0x60)], "Ω", 0x06, "kΩ", &[]),
            (&[(16, 0x50)], "Ω", 0x06, "MΩ", &[]),
            (&[(17, 0x09)], "Capacitance", 0x09, "nF", &[]),
            (&[(17, 0x05)], "Capacitance", 0x09, "mF", &[]),
            (&[(17, 0x03)], "Capacitance", 0x09, "µF", &[]),
            (&[(17, 0x01)], "Capacitance", 0x09, "F", &[]),
            (
                &[(4, 0x10), (13, 0x02), (17, 0x04)],
                "DC mV",
                0x12,
                "mV",
                &["dc"],
            ),
            (&[(18, 0x10), (13, 0x02)], "DC A", 0x13, "A", &["dc"]),
            (
                &[(18, 0x10), (13, 0x40), (17, 0x04)],
                "AC mA",
                0x24,
                "mA",
                &[],
            ),
        ];
        for &(flags, mode, mode_raw, unit, set) in cases {
            let m = quiet(&t4("2345", Some(1), false, false, None, flags));
            assert_eq!(m.mode, mode, "{flags:02X?}");
            assert_eq!(m.mode_raw, mode_raw, "{flags:02X?}");
            assert_eq!(m.unit, unit, "{flags:02X?}");
            assert_eq!(flags_set(&m), set, "{flags:02X?}");
            assert_eq!(m.display_raw.as_deref(), Some(" 2.345"));
            assert_eq!(value(&m), 2.345);
            assert!(m.aux_values.is_empty(), "{flags:02X?}");
        }
    }

    /// The leading "1" adds 10000 counts: 19999 on the ZT-5566 (spec §6.3).
    #[test]
    fn type4_leading_one_and_sign() {
        let m = quiet(&t4("9999", Some(2), true, true, None, T4_DCV));
        assert_eq!(m.display_raw.as_deref(), Some("-199.99"));
        assert!((value(&m) + 199.99).abs() < 1e-9);
        let m = quiet(&t4("0000", None, false, false, None, T4_DCV));
        assert_eq!(m.display_raw.as_deref(), Some(" 0000"));
    }

    /// The secondary display's frequency and duty, and its own minus.
    #[test]
    fn type4_secondary_display() {
        let cases = [
            (0x40, "Frequency", "Hz"),
            (0x60, "Frequency", "kHz"),
            (0x80, "Duty", "%"),
        ];
        for (bits, label, unit) in cases {
            let m = quiet(&t4(
                "2345",
                Some(1),
                false,
                false,
                Some(("1234", Some(3), false)),
                &[(4, 0x10 | bits), (13, 0x40)],
            ));
            assert_eq!(m.aux_values.len(), 1);
            let aux = &m.aux_values[0];
            assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), (label, unit));
            assert_eq!(aux.display_raw.as_deref(), Some("123.4"));
        }
        let m = quiet(&t4(
            "2345",
            None,
            false,
            false,
            Some(("0012", Some(2), true)),
            &[(4, 0x50), (13, 0x40)],
        ));
        assert_eq!(m.aux_values[0].display_raw.as_deref(), Some("-00.12"));
        assert!(matches!(m.aux_values[0].value, MeasuredValue::Normal(v) if v == -0.12));
    }

    #[test]
    fn type4_ol_and_auto() {
        let m = quiet(&t4(" 0L ", Some(2), false, false, None, &[(16, 0x40)]));
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw.as_deref(), Some("  0.L "));
        let m = quiet(&t4("Auto", None, false, false, None, &[]));
        assert!(matches!(m.value, MeasuredValue::NoReading("Auto")));
        assert_eq!(m.mode, "Auto");
    }

    /// Byte 3 bit 0 and byte 13 bit 0, used by neither app (spec §7.4,
    /// §10.10); a leading "1" with one segment; secondary digits with no
    /// unit; `k` on the duty cycle.
    #[test]
    fn type4_undocumented_patterns_are_reported() {
        let packets = [
            t4("2345", None, false, false, None, &[(3, 0x01), (4, 0x10)]),
            t4("2345", None, false, false, None, &[(13, 0x03), (4, 0x10)]),
            t4("2345", None, false, false, None, &[(13, 0x08), (4, 0x10)]),
            t4(
                "2345",
                None,
                false,
                false,
                Some(("1234", None, false)),
                T4_DCV,
            ),
            t4(
                "2345",
                None,
                false,
                false,
                Some(("1234", None, false)),
                &[(4, 0xB0)],
            ),
        ];
        for packet in packets {
            let (_, reports) = reported(&packet);
            assert_eq!(reports.len(), 1, "{packet:02X?}: {reports:?}");
        }
    }

    // --- Type 1 (ZT-5BQ) --------------------------------------------------

    fn t1(glyphs: &str, dp_at: Option<usize>, flags: &[(usize, u8)]) -> Vec<u8> {
        split_packet(1, glyphs, dp_at, false, flags)
    }

    /// Spec §9: 230.5 V AC, HOLD, the Bluetooth icon lit.
    #[test]
    fn type1_worked_example() {
        let m = quiet(EXAMPLES[1].1);
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.mode_raw, 0x21);
        assert_eq!(m.unit, "V");
        assert_eq!(m.display_raw.as_deref(), Some("230.5"));
        assert_eq!(value(&m), 230.5);
        assert_eq!(flags_set(&m), ["hold"]);
    }

    /// Every §7.2 bit, one at a time over a function where it needs one:
    /// (bits, mode, mode_raw, unit, flags set).
    #[test]
    fn type1_every_annunciator() {
        type Case = (
            &'static [(usize, u8)],
            &'static str,
            u16,
            &'static str,
            &'static [&'static str],
        );
        let cases: &[Case] = &[
            (&[(3, 0x08), (8, 0x10)], "Continuity", 0x07, "Ω", &[]),
            (&[(3, 0x04), (7, 0x60)], "DC V", 0x11, "V", &["dc"]),
            (&[(3, 0x02), (7, 0x60)], "DC V", 0x11, "V", &["hold", "dc"]),
            (
                &[(3, 0x01), (7, 0x60)],
                "DC V",
                0x11,
                "V",
                &["low_battery", "dc"],
            ),
            (&[(7, 0xA0)], "AC V", 0x21, "V", &[]),
            (&[(7, 0x70)], "DC V", 0x11, "nV", &["dc"]),
            (&[(8, 0x90)], "Ω", 0x06, "MΩ", &[]),
            (&[(7, 0x60), (8, 0x40)], "DC V", 0x11, "mV", &["dc"]),
            (&[(8, 0x30)], "Ω", 0x06, "kΩ", &[]),
            (&[(8, 0x0C), (7, 0x80)], "AC A", 0x23, "µA", &[]),
            (&[(8, 0x44), (7, 0x80)], "AC A", 0x23, "mA", &[]),
            (&[(8, 0x04), (7, 0x80)], "AC A", 0x23, "A", &[]),
            (&[(8, 0x02), (7, 0x20)], "Diode", 0x08, "V", &[]),
            (&[(8, 0x01)], "Capacitance", 0x09, "F", &[]),
            (&[(8, 0x09)], "Capacitance", 0x09, "µF", &[]),
            (&[(9, 0x80), (7, 0x60)], "DC V", 0x11, "V", &["dc"]),
            (&[(9, 0x40), (7, 0x60)], "DC V peak", 0x51, "V", &["dc"]),
            (&[(9, 0x20)], "Duty %", 0x0B, "%", &[]),
            (&[(9, 0x10), (8, 0x04), (7, 0x80)], "Inrush", 0x0F, "A", &[]),
            (&[(9, 0x08)], "°C", 0x0C, "°C", &[]),
            (&[(9, 0x04)], "°F", 0x0D, "°F", &[]),
            (&[(9, 0x02)], "Hz", 0x0A, "Hz", &[]),
            (&[(9, 0x02), (8, 0x20)], "Hz", 0x0A, "kHz", &[]),
            (&[(9, 0x01), (7, 0x60)], "DC V", 0x11, "V", &["rel", "dc"]),
        ];
        for &(flags, mode, mode_raw, unit, set) in cases {
            let m = quiet(&t1("1234", Some(2), flags));
            assert_eq!(m.mode, mode, "{flags:02X?}");
            assert_eq!(m.mode_raw, mode_raw, "{flags:02X?}");
            assert_eq!(m.unit, unit, "{flags:02X?}");
            assert_eq!(flags_set(&m), set, "{flags:02X?}");
            assert_eq!(value(&m), 12.34);
        }
    }

    /// The community ST207 log's inrush wait: `----` with AC, A and INRUSH
    /// (spec §11.3 D3) is no reading, not an NCV level; the same dashes
    /// without INRUSH stay NCV.
    #[test]
    fn dashes_with_inrush_are_the_inrush_wait() {
        for (row, word) in [("-   ", "-"), ("--  ", "--"), ("----", "----")] {
            let m = quiet(&t1(row, None, &[(7, 0x80), (8, 0x04), (9, 0x90)]));
            assert!(
                matches!(m.value, MeasuredValue::NoReading(w) if w == word),
                "{row:?}: {:?}",
                m.value
            );
            assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("Inrush", "A"));
        }
        let m = quiet(&t1("----", None, &[]));
        assert!(matches!(m.value, MeasuredValue::NcvLevel(4)));

        let m = quiet(&t1("1234", Some(1), &[(7, 0x80), (8, 0x04), (9, 0x90)]));
        assert_eq!(m.mode, "Inrush");
        assert_eq!(value(&m), 1.234);
    }

    /// An auto-ranging clamp's prefix is a range step, so the mode stays
    /// the base unit's.
    #[test]
    fn type1_prefixes_do_not_name_the_mode() {
        let m = quiet(&t1("1234", Some(1), &[(7, 0x60), (8, 0x40)]));
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("DC V", "mV"));
    }

    #[test]
    fn type1_words() {
        let m = quiet(&t1("Auto", None, &[(3, 0x04), (9, 0x80)]));
        assert!(matches!(m.value, MeasuredValue::NoReading("Auto")));
        assert_eq!(m.mode, "Auto");
        let m = quiet(&t1(" EF ", None, &[(3, 0x04), (9, 0x80)]));
        assert!(matches!(m.value, MeasuredValue::NcvLevel(0)));
        let m = quiet(&t1(" 0L ", Some(2), &[(8, 0x90)]));
        assert!(matches!(m.value, MeasuredValue::Overload));
    }

    // --- Type 2 (ZT-5B) ---------------------------------------------------

    fn t2(glyphs: &str, dp_at: Option<usize>, flags: &[(usize, u8)]) -> Vec<u8> {
        split_packet(2, glyphs, dp_at, false, flags)
    }

    /// Spec §9: 4.700 kΩ, the Bluetooth icon lit.
    #[test]
    fn type2_worked_example() {
        let m = quiet(EXAMPLES[2].1);
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.mode_raw, 0x06);
        assert_eq!(m.unit, "kΩ");
        assert_eq!(m.display_raw.as_deref(), Some("4.700"));
        assert_eq!(value(&m), 4.7);
        assert!(flags_set(&m).is_empty());
    }

    /// Every §7.3 bit, one at a time over a function where it needs one:
    /// (bits, mode, mode_raw, unit, flags set).
    #[test]
    fn type2_every_annunciator() {
        type Case = (
            &'static [(usize, u8)],
            &'static str,
            u16,
            &'static str,
            &'static [&'static str],
        );
        let cases: &[Case] = &[
            (&[(3, 0x08), (9, 0x01)], "Continuity", 0x07, "Ω", &[]),
            (&[(3, 0x04), (8, 0x0A)], "AC V", 0x21, "V", &["hv_warning"]),
            (&[(3, 0x02), (8, 0x06)], "DC V", 0x11, "V", &["hold", "dc"]),
            (
                &[(3, 0x01), (8, 0x06)],
                "DC V",
                0x11,
                "V",
                &["low_battery", "dc"],
            ),
            (&[(7, 0x80), (8, 0x06)], "DC V", 0x11, "V", &["dc"]),
            (&[(7, 0x40), (8, 0x06)], "DC V", 0x11, "V", &["dc"]),
            (&[(8, 0xC4)], "DC A", 0x13, "µA", &["dc"]),
            (&[(8, 0x44)], "DC A", 0x13, "A", &["dc"]),
            (&[(8, 0x48), (9, 0x04)], "AC A", 0x23, "mA", &[]),
            (&[(8, 0x22)], "Diode", 0x08, "V", &[]),
            (&[(8, 0x10)], "Capacitance", 0x09, "F", &[]),
            (&[(8, 0x90)], "Capacitance", 0x09, "µF", &[]),
            (&[(8, 0x11)], "Capacitance", 0x09, "nF", &[]),
            (&[(8, 0x10), (9, 0x04)], "Capacitance", 0x09, "mF", &[]),
            (&[(8, 0x0A), (9, 0x04)], "AC V", 0x21, "mV", &[]),
            (&[(9, 0x80)], "°C", 0x0C, "°C", &[]),
            (&[(9, 0x40)], "°F", 0x0D, "°F", &[]),
            (&[(9, 0x20)], "Hz", 0x0A, "Hz", &[]),
            (&[(9, 0x22)], "Hz", 0x0A, "kHz", &[]),
            (&[(9, 0x28)], "Hz", 0x0A, "MHz", &[]),
            (&[(9, 0x10)], "Duty %", 0x0B, "%", &[]),
            (&[(9, 0x09)], "Ω", 0x06, "MΩ", &[]),
            (&[(9, 0x01)], "Ω", 0x06, "Ω", &[]),
        ];
        for &(flags, mode, mode_raw, unit, set) in cases {
            let m = quiet(&t2("1234", Some(2), flags));
            assert_eq!(m.mode, mode, "{flags:02X?}");
            assert_eq!(m.mode_raw, mode_raw, "{flags:02X?}");
            assert_eq!(m.unit, unit, "{flags:02X?}");
            assert_eq!(flags_set(&m), set, "{flags:02X?}");
            assert_eq!(value(&m), 12.34);
        }
    }

    /// Byte 7 bits 5-4: used by neither app, never set (spec §7.3, §11.4).
    #[test]
    fn type2_unread_bits_are_reported() {
        for bits in [0x10, 0x20] {
            let (_, reports) = reported(&t2("1234", None, &[(7, bits), (8, 0x06)]));
            assert_eq!(reports.len(), 1, "{bits:#04x}: {reports:?}");
            assert!(reports[0].starts_with("zt5b: "), "{reports:?}");
        }
    }

    #[test]
    fn type2_words() {
        let m = quiet(&t2("Auto", None, &[(7, 0x80)]));
        assert!(matches!(m.value, MeasuredValue::NoReading("Auto")));
        let m = quiet(&t2(" EF ", None, &[(7, 0x80)]));
        assert!(matches!(m.value, MeasuredValue::NcvLevel(0)));
        let m = quiet(&t2("--  ", None, &[(7, 0x80)]));
        assert!(matches!(m.value, MeasuredValue::NcvLevel(2)));
        let m = quiet(&t2(" 0L ", Some(1), &[(8, 0x22)]));
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.mode, "Diode");
    }

    /// `draw` is `t2`'s packet, and what `decode` reads back.
    #[test]
    fn type2_draw_is_the_inverse_of_decode() {
        let cells = |glyphs: &str, dp_at: Option<usize>| -> [Cell; 4] {
            let mut out = [BLANK; 4];
            for (i, c) in glyphs.chars().enumerate() {
                out[i] = Cell {
                    glyph: Glyph::from_segments(segments(c)),
                    dp: dp_at == Some(i),
                };
            }
            out
        };
        let lit = [
            Meaning::Unit(Unit::Ohm),
            Meaning::Prefix(Prefix::Kilo, &[]),
            Meaning::Hold,
        ];
        let packet = ZT5B.draw(&cells("4700", Some(1)), false, &lit).unwrap();
        assert_eq!(packet, t2("4700", Some(1), &[(9, 0x03), (3, 0x02)]));
        let m = quiet(&packet);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("Ω", "kΩ"));
        assert_eq!(flags_set(&m), ["hold"]);

        let negative = ZT5B.draw(&cells("1234", Some(2)), true, &[Meaning::Dc]);
        assert_eq!(
            negative.unwrap(),
            split_packet(2, "1234", Some(2), true, &[(8, 0x04)])
        );
        // No REL bit on type 2, no split digits on type 4, no silent lookups.
        assert!(
            ZT5B.draw(&cells("1234", None), false, &[Meaning::Rel])
                .is_none()
        );
        assert!(ZT5566SE.draw(&cells("1234", None), false, &[]).is_none());
        assert!(
            ZT5B.draw(&cells("1234", None), false, &[Meaning::Silent])
                .is_none()
        );
    }

    // --- Every layout -----------------------------------------------------

    /// Every type the apps define has a layout, the length the extractor
    /// cuts it at, and a registry entry.
    #[test]
    fn every_defined_type_has_a_layout() {
        for type_byte in 1..=4 {
            let layout = for_type(type_byte).expect("a layout");
            assert!(frame::packet_len(type_byte).is_some());
            assert!(crate::protocol::registry::find_device(layout.id).is_some());
        }
        assert!(for_type(0).is_none() && for_type(5).is_none());
    }

    /// No digit lit on the main display: no reading to give, so the packet
    /// is reported and refused, a sign or a point on the blanks included.
    #[test]
    fn a_blank_main_display_is_refused() {
        for packet in [
            t3("    ", None, T3_DCV),
            split_packet(3, "    ", Some(2), true, T3_DCV),
            t4("    ", None, false, false, None, T4_DCV),
        ] {
            let (m, reports) = capture_reports(|| decode(&packet));
            assert!(m.is_err(), "{packet:02X?}: {m:?}");
            assert_eq!(reports.len(), 1, "{packet:02X?}: {reports:?}");
        }
    }

    /// A blank secondary display carries no sub-value, its unit lit or not.
    #[test]
    fn a_blank_secondary_display_carries_no_sub_value() {
        let m = quiet(&t4(
            "2345",
            Some(1),
            false,
            false,
            Some(("    ", None, false)),
            &[(4, 0x50), (13, 0x40)],
        ));
        assert!(m.aux_values.is_empty(), "{:?}", m.aux_values);
    }

    #[test]
    fn a_packet_that_is_not_whole_is_refused() {
        let (_, plain) = EXAMPLES[0];
        assert!(decode(&plain[..10]).is_err());
        let mut long = plain.to_vec();
        long.push(0);
        assert!(decode(&long).is_err());
        let mut no_header = plain.to_vec();
        no_header[0] = 0;
        assert!(decode(&no_header).is_err());
        let mut unknown_type = plain.to_vec();
        unknown_type[2] = 9;
        assert!(decode(&unknown_type).is_err());
        assert!(decode(&[]).is_err());
    }

    /// What the keys follow: the unit lit, and the function and coupling the
    /// mode names.
    #[test]
    fn showing_is_the_unit_function_and_coupling() {
        let showing = |packet: &[u8]| capture_reports(|| decode_showing(packet)).0.unwrap().1;
        assert_eq!(
            showing(&t1("1234", Some(2), &[(9, 0x08)])),
            Showing {
                unit: Some(Unit::Celsius),
                function: Function::Celsius,
                coupling: Coupling::None
            }
        );
        let ac_ma = t4(
            "2345",
            Some(1),
            false,
            false,
            None,
            &[(18, 0x10), (13, 0x40), (17, 0x04)],
        );
        assert_eq!(
            showing(&ac_ma),
            Showing {
                unit: Some(Unit::Amp),
                function: Function::Milliamps,
                coupling: Coupling::Ac
            }
        );
    }

    #[test]
    fn plausible_needs_listed_glyphs() {
        assert!(plausible(EXAMPLES[0].1).is_some());
        let mut packet = EXAMPLES[0].1.to_vec();
        packet[5] = (packet[5] & 0xF0) | 0x01;
        packet[4] &= 0x0F;
        assert!(plausible(&packet).is_none());
    }

    /// Arbitrary packets of every length never panic the decoder.
    #[test]
    fn arbitrary_packets_never_panic() {
        let mut seed = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            let type_byte = (pseudo_random(&mut seed) % 6) as u8;
            let len = frame::packet_len(type_byte).unwrap_or(12);
            let mut packet: Vec<u8> = (0..len).map(|_| pseudo_random(&mut seed) as u8).collect();
            packet[..3].copy_from_slice(&[0x5A, 0xA5, type_byte]);
            let _ = capture_reports(|| decode(&packet));
        }
    }
}
