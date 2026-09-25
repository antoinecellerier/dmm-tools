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
use crate::measurement::{MeasuredValue, Measurement};
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

/// Where the digits sit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Digits {
    /// Types 1-3: four digits split across bytes 3-7, each taking its high
    /// nibble from one byte and its low nibble from the next; byte 3 bit 4
    /// is the minus and bytes 4-6 bit 4 the decimal points (spec §6.2).
    Split,
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
        }
    }

    /// Every row the layout carries.
    fn rows(self, packet: &[u8]) -> impl Iterator<Item = Row> {
        std::iter::once(self.main(packet))
    }
}

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
    /// (spec §7.5, its "mV" and "µ/m A" groups). Where the prefix is only an
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

/// Every layout implemented, by type byte.
pub(super) static LAYOUTS: &[&Layout] = &[&ZT300AB];

/// The layout a type byte selects, where it is implemented.
pub(super) fn for_type(type_byte: u8) -> Option<&'static Layout> {
    LAYOUTS
        .iter()
        .copied()
        .find(|layout| layout.type_byte == type_byte)
}

/// The layout of a whole descrambled packet: header, a type byte whose
/// layout is implemented, and that type's length (spec §5).
pub(super) fn layout_of(packet: &[u8]) -> Result<&'static Layout> {
    if packet.len() <= TYPE_AT || !packet.starts_with(&HEADER) {
        return Err(Error::invalid_response("zotek: no 5A A5 header", packet));
    }
    let type_byte = packet[TYPE_AT];
    let Some(len) = frame::packet_len(type_byte) else {
        return Err(Error::invalid_response(
            format!("zotek: no layout has type byte {type_byte:#04x}"),
            packet,
        ));
    };
    let Some(layout) = for_type(type_byte) else {
        return Err(Error::invalid_response(
            format!("zotek: the type-{type_byte} layout is not supported"),
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

/// The layout of `packet` if it looks like one a meter sends: whole, of an
/// implemented type, and every digit a glyph the spec lists (spec §6.1),
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Coupling {
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
/// volt and amp functions — an internal number, stable for bug reports, not
/// a byte the meter sends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Function {
    /// No function annunciator lit.
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

/// Decode one whole descrambled packet, by its own type byte.
pub(super) fn decode(packet: &[u8]) -> Result<Measurement> {
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
        _ => Cow::Borrowed(function.mode(coupling)),
    };
    let unit = match (function, unit) {
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
        dc: function.coupled() && lit.dc,
        ..StatusFlags::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: function.mode_raw(coupling),
        value,
        unit: Cow::Borrowed(unit),
        display_raw: Some(readout.text),
        flags,
        ..Measurement::from_payload(packet)
    })
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

    /// No digit lit on the main display: no reading to give, so the packet
    /// is reported and refused, a sign or a point on the blanks included.
    #[test]
    fn a_blank_main_display_is_refused() {
        for packet in [
            t3("    ", None, T3_DCV),
            split_packet(3, "    ", Some(2), true, T3_DCV),
        ] {
            let (m, reports) = capture_reports(|| decode(&packet));
            assert!(m.is_err(), "{packet:02X?}: {m:?}");
            assert_eq!(reports.len(), 1, "{packet:02X?}: {reports:?}");
        }
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
