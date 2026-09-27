//! A reply's segments → [`Measurement`]
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §5, §7, §8).
//!
//! The reply is the LCD: digits as seven-segment patterns, and function,
//! unit, prefix, sign and decimal point as annunciators, with no function
//! or range code (spec §5.1). The reading is rebuilt from what is lit, as
//! Brymen's programs do (spec §8): the value in the unit the display shows,
//! the function from the unit and coupling annunciators, and the secondary
//! display as one sub-value.
//!
//! Every bit lands in one of three places: the reading; silence, for what
//! the sheet documents and no reading shows (the bar graph, the T1-T2 dash,
//! the "don't care" bytes); or a report, for what the spec does not cover.

use super::glyph::{self, Cell, Readout, UNKNOWN};
use super::map::{Ann, Lit, MODEL, Map, Row};
use super::reply::REPLY_LEN;
use super::{Series, report};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MainLabel, MeasuredValue, Measurement};
use crate::protocol::unknown_mode16;
use std::borrow::Cow;

/// Decode one reply's 24 data bytes.
pub(super) fn decode(data: &[u8], series: Series) -> Result<Measurement> {
    let id = series.id();
    if data.len() != REPLY_LEN {
        return Err(Error::invalid_response(
            format!("{id} reply is {} bytes, expected {REPLY_LEN}", data.len()),
            data,
        ));
    }
    if data[MODEL] != series.code() {
        return Err(Error::invalid_response(
            format!(
                "{id} reply's model byte is {:02X}, expected {:02X}",
                data[MODEL],
                series.code()
            ),
            data,
        ));
    }
    let map = series.map();
    let lit = Lit::read(map, data);
    let report = |what| report(series, data, what);

    let mut main_cells = cells(map, &map.main, data);
    let letter = temperature_letter(&mut main_cells);
    let main = glyph::read(&main_cells, map.main.minus.lit(data));
    if let Some(odd) = main.odd {
        report(odd);
    }
    let shown = main_shown(&main, letter.is_some(), &report);

    let prefix = main_prefix(lit, &report);
    let function = function(lit, letter);
    let mode_raw = function_word(lit, letter);
    let (mode, unit) = match function {
        Some(function) => {
            let unit = unit(function.base, prefix).unwrap_or_else(|| {
                report("prefix annunciators");
                function.base.bare()
            });
            let mode = match function.mode {
                Mode::Named(mode) => mode,
                Mode::Coupled(coupling) => coupled(coupling, unit),
            };
            (Cow::Borrowed(mode), unit)
        }
        None => {
            // A word the manuals show explains itself, whatever is lit.
            if !matches!(shown, Shown::Word(_)) {
                report("function annunciators");
            }
            (unknown_mode16(mode_raw), "")
        }
    };

    let sub = glyph::read(
        &cells(map, &map.secondary, data),
        map.secondary.minus.lit(data),
    );
    let aux = secondary(lit, &sub, letter, &report);
    let main_label = match (&aux, function.map(|f| f.mode)) {
        (Some(a), Some(Mode::Coupled(Coupling::Dc))) if a.label == LABEL_AC => Some(MainLabel::Dc),
        (Some(a), Some(Mode::Named(MODE_T1))) if a.label == LABEL_T2 => Some(MainLabel::T1),
        _ => None,
    };

    let lead_error = matches!(shown, Shown::Word(INPUT_ERROR));
    let (value, display_raw) = match shown {
        Shown::Number(v, text) => (MeasuredValue::Normal(v), Some(text)),
        Shown::Overload => (MeasuredValue::Overload, None),
        Shown::Word(word) | Shown::Unread(word) => (MeasuredValue::NoReading(word), None),
    };
    Ok(Measurement {
        mode,
        mode_raw,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags: StatusFlags {
            lead_error,
            dc: lit.has(Ann::Dc1) && !lit.has(Ann::Ac1),
            ..flags(lit, &report)
        },
        aux_values: aux.into_iter().collect(),
        main_label,
        ..Measurement::from_payload(data)
    })
}

/// A row's cells, most significant first.
fn cells(map: &Map, row: &Row, data: &[u8]) -> Vec<Cell> {
    row.digits
        .iter()
        .enumerate()
        .map(|(i, &index)| Cell {
            segments: map.segments(data[index]),
            point: row
                .points
                .iter()
                .any(|(after, bit)| *after == i && bit.lit(data)),
        })
        .collect()
}

/// A `C` or `F` in the last main cell, after something else lit, is the
/// temperature unit (spec §5.2): taken off the row, it is returned.
fn temperature_letter(cells: &mut Vec<Cell>) -> Option<char> {
    let (last, rest) = cells.split_last()?;
    let letter = glyph::char_of(last.segments);
    if matches!(letter, 'C' | 'F') && rest.iter().any(|c| c.segments != 0) {
        cells.pop();
        Some(letter)
    } else {
        None
    }
}

/// The Beep-Jack warning, as the manual spells it (spec §7.3).
const INPUT_ERROR: &str = "InEr";

/// What the main display shows.
enum Shown {
    Number(f64, String),
    Overload,
    /// A word the manuals show (spec §7.3).
    Word(&'static str),
    /// Something no source shows, already reported.
    Unread(&'static str),
}

/// Read the main display: a number, or a word in the manuals' spelling
/// (spec §7.3). `letter` says a temperature unit followed the row.
fn main_shown(main: &Readout, letter: bool, report: &impl Fn(&'static str)) -> Shown {
    if let Some(v) = main.number {
        return Shown::Number(v, main.text.clone());
    }
    if main.odd == Some("blank digit") {
        return Shown::Unread("?");
    }
    match main.word.as_str() {
        // OL, or .OL (BM860s manual p.10): the letters themselves (spec
        // §8.4).
        "0L" => Shown::Overload,
        // How the meter draws the I is open: either reading of it (spec
        // §7.3).
        "?nEr" | "1nEr" => Shown::Word(INPUT_ERROR),
        // The power-on self-diagnosis (BM860s manual p.15).
        "rE-0" => Shown::Word("rE-O"),
        "C_Er" => Shown::Word("C_Er"),
        // Dashes where a temperature would be.
        w if letter && !w.is_empty() && w.chars().all(|c| c == '-') => Shown::Word("---"),
        w if w.contains(UNKNOWN) => {
            report("digit glyph");
            Shown::Unread("?")
        }
        _ => {
            report("display text");
            Shown::Unread("?")
        }
    }
}

/// A metric prefix annunciator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prefix {
    Nano,
    Micro,
    Milli,
    Kilo,
    Mega,
}

/// The one prefix lit among `prefixes`, `None` for none. Two or more give
/// none, as in Brymen's programs (spec §8.3), and are reported.
fn one_prefix(
    lit: Lit,
    prefixes: &[(Ann, Prefix)],
    report: &impl Fn(&'static str),
) -> Option<Prefix> {
    let mut on = prefixes.iter().filter(|(ann, _)| lit.has(*ann));
    let first = on.next().map(|(_, p)| *p);
    if on.next().is_some() {
        report("prefix annunciators");
        return None;
    }
    first
}

/// The main display's prefix (spec §8.3). With dB lit, m is the m of the
/// "dBm" label, never a prefix, and no other prefix belongs (spec §8.2).
fn main_prefix(lit: Lit, report: &impl Fn(&'static str)) -> Option<Prefix> {
    let others = [
        (Ann::Nano1, Prefix::Nano),
        (Ann::Micro1, Prefix::Micro),
        (Ann::Kilo1, Prefix::Kilo),
        (Ann::Mega1, Prefix::Mega),
    ];
    if lit.has(Ann::Db) {
        if others.iter().any(|(ann, _)| lit.has(*ann)) {
            report("prefix annunciators");
        }
        return None;
    }
    let [nano, micro, kilo, mega] = others;
    one_prefix(
        lit,
        &[nano, micro, (Ann::Milli1, Prefix::Milli), kilo, mega],
        report,
    )
}

/// A reading's unit before its prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Base {
    Volt,
    Amp,
    Ohm,
    Farad,
    Hertz,
    Siemens,
    Percent,
    Dbm,
    Celsius,
    Fahrenheit,
}

impl Base {
    fn bare(self) -> &'static str {
        match self {
            Base::Volt => "V",
            Base::Amp => "A",
            Base::Ohm => "Ω",
            Base::Farad => "F",
            Base::Hertz => "Hz",
            Base::Siemens => "S",
            Base::Percent => "%",
            Base::Dbm => "dBm",
            Base::Celsius => "°C",
            Base::Fahrenheit => "°F",
        }
    }
}

/// The unit a reading shows with `prefix`, for the ranges the manual lists
/// (spec §11.5); `None` for a prefix none of them has. µ is U+00B5 and Ω
/// U+03A9, the characters `transform::si_prefix` reads.
fn unit(base: Base, prefix: Option<Prefix>) -> Option<&'static str> {
    use Base::*;
    use Prefix::*;
    Some(match (base, prefix) {
        (Volt, Some(Milli)) => "mV",
        (Amp, Some(Micro)) => "µA",
        (Amp, Some(Milli)) => "mA",
        (Ohm, Some(Kilo)) => "kΩ",
        (Ohm, Some(Mega)) => "MΩ",
        (Farad, Some(Nano)) => "nF",
        (Farad, Some(Micro)) => "µF",
        (Farad, Some(Milli)) => "mF",
        (Hertz, Some(Kilo)) => "kHz",
        (Hertz, Some(Mega)) => "MHz",
        (Siemens, Some(Nano)) => "nS",
        (Volt | Amp | Ohm | Hertz | Percent | Dbm | Celsius | Fahrenheit, None) => base.bare(),
        _ => return None,
    })
}

/// The main display's coupling, ⎓ and ∿ (spec §5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Coupling {
    None,
    Dc,
    Ac,
    AcDc,
}

/// How a function's mode is named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Named(&'static str),
    /// A voltage or current reading, named by its coupling and its unit
    /// once the prefix is known.
    Coupled(Coupling),
}

/// A function: its mode and its unit before the prefix.
#[derive(Clone, Copy, Debug)]
struct Function {
    mode: Mode,
    base: Base,
}

const fn named(mode: &'static str, base: Base) -> Function {
    Function {
        mode: Mode::Named(mode),
        base,
    }
}

const MODE_T1: &str = "T1";
const LABEL_T2: &str = "T2";
const LABEL_AC: &str = "AC";

/// A voltage or current reading's mode.
fn coupled(coupling: Coupling, unit: &'static str) -> &'static str {
    use Coupling::*;
    match (coupling, unit) {
        (Dc, "mV") => "DC mV",
        (Ac, "mV") => "AC mV",
        (AcDc, "mV") => "AC+DC mV",
        (Dc, "µA") => "DC µA",
        (Ac, "µA") => "AC µA",
        (AcDc, "µA") => "AC+DC µA",
        (Dc, "mA") => "DC mA",
        (Ac, "mA") => "AC mA",
        (AcDc, "mA") => "AC+DC mA",
        (Dc, "A") => "DC A",
        (Ac, "A") => "AC A",
        (AcDc, "A") => "AC+DC A",
        (Dc, _) => "DC V",
        (Ac, _) => "AC V",
        (AcDc | None, _) => "AC+DC V",
    }
}

/// The function what is lit shows, by the first rule that matches (spec
/// §8.1, §8.2); `None` for a combination no source gives. `letter` is the
/// temperature unit in the digits.
fn function(lit: Lit, letter: Option<char>) -> Option<Function> {
    use Ann::*;
    let units: Vec<Ann> = lit
        .among([V1, A1, Ohm1, Farad1, Hz1, Siemens1, Duty1, Db])
        .collect();
    let coupling = match (lit.has(Dc1), lit.has(Ac1)) {
        (false, false) => Coupling::None,
        (true, false) => Coupling::Dc,
        (false, true) => Coupling::Ac,
        (true, true) => Coupling::AcDc,
    };
    // The unit letter in the digits, and T1 and T2 which probe: both lit
    // read as the difference, whatever the dash between them (spec §8.2).
    if let Some(letter) = letter {
        let base = if letter == 'C' {
            Base::Celsius
        } else {
            Base::Fahrenheit
        };
        if !units.is_empty() {
            return None;
        }
        return match (lit.has(Ann::T1), lit.has(Ann::T2)) {
            (true, true) => Some(named("T1-T2", base)),
            (true, false) => Some(named(MODE_T1, base)),
            (false, true) => Some(named("T2", base)),
            (false, false) => None,
        };
    }
    Some(match units.as_slice() {
        // The reference impedance, shown for a second as dBm starts ("600"
        // with Ω and dBm lit; BM860s manual p.7, spec §8.2 `0x2080`).
        [Ohm1, Db] => named("dBm reference", Base::Ohm),
        [Db] => named("dBm", Base::Dbm),
        [Hz1] if lit.has(Vfd) => named("VFD Hz", Base::Hertz),
        [V1] if lit.has(Vfd) && coupling == Coupling::Ac => named("VFD AC V", Base::Volt),
        [Ohm1] if lit.has(Continuity) => named("Continuity", Base::Ohm),
        [Ohm1] => named("Ω", Base::Ohm),
        [Siemens1] => named("nS", Base::Siemens),
        [Farad1] => named("Capacitance", Base::Farad),
        // Line and logic frequency look the same on the LCD.
        [Hz1] => named("Hz", Base::Hertz),
        [Duty1] => named("Duty %", Base::Percent),
        // V with neither coupling: the diode test (spec §8.2).
        [V1] if coupling == Coupling::None => named("Diode", Base::Volt),
        [V1] => Function {
            mode: Mode::Coupled(coupling),
            base: Base::Volt,
        },
        [A1] if coupling != Coupling::None => Function {
            mode: Mode::Coupled(coupling),
            base: Base::Amp,
        },
        _ => return None,
    })
}

/// Brymen's function word F for what is lit, masked to 16 bits: the
/// reading's `mode_raw` (spec §8.1).
fn function_word(lit: Lit, letter: Option<char>) -> u16 {
    let codes = [
        (Ann::Ac1, 0x0001),
        (Ann::Dc1, 0x0002),
        (Ann::V1, 0x0004),
        (Ann::Farad1, 0x0008),
        (Ann::Ohm1, 0x0080),
        (Ann::Continuity, 0x0100),
        (Ann::A1, 0x0200),
        (Ann::Hz1, 0x0400),
        (Ann::Duty1, 0x0800),
        (Ann::Siemens1, 0x1000),
        (Ann::Db, 0x2000),
        (Ann::T2, 0x4000),
        (Ann::T1, 0x8000),
    ];
    let letter = match letter {
        Some('C') => 0x0020,
        Some('F') => 0x0040,
        _ => 0,
    };
    codes
        .iter()
        .filter(|(ann, _)| lit.has(*ann))
        .fold(letter, |word, (_, code)| word | code)
}

/// The secondary display as a sub-value, where a unit is lit on it (spec
/// §5.1): the frequency, the AC component, T2, or the loop current's
/// percentage. Blank, or the "diod" label (spec §7.3), is none.
fn secondary(
    lit: Lit,
    sub: &Readout,
    letter: Option<char>,
    report: &impl Fn(&'static str),
) -> Option<AuxValue> {
    use Ann::*;
    let units: Vec<Ann> = lit.among([Hz2, V2, A2, T2Sub, Loop]).collect();
    let prefix = one_prefix(
        lit,
        &[
            (Micro2, Prefix::Micro),
            (Milli2, Prefix::Milli),
            (Kilo2, Prefix::Kilo),
            (Mega2, Prefix::Mega),
        ],
        report,
    );
    let prefixed = |base: Base| {
        unit(base, prefix).unwrap_or_else(|| {
            report("secondary annunciators");
            base.bare()
        })
    };
    let (label, unit) = match units.as_slice() {
        [] if sub.word.is_empty() || sub.word == "diod" => return None,
        [Hz2] => ("Frequency", prefixed(Base::Hertz)),
        [V2] if lit.has(Ac2) => (LABEL_AC, prefixed(Base::Volt)),
        [A2] if lit.has(Ac2) => (LABEL_AC, prefixed(Base::Amp)),
        // The secondary display has no letter of its own: T2 is in the
        // main one's unit.
        [T2Sub] => match letter {
            Some('C') => (LABEL_T2, "°C"),
            Some(_) => (LABEL_T2, "°F"),
            None => {
                report("secondary annunciators");
                return None;
            }
        },
        [Loop] => ("4-20mA", "%"),
        [] => {
            report("secondary display");
            return None;
        }
        _ => {
            report("secondary annunciators");
            return None;
        }
    };
    if let Some(odd) = sub.odd {
        report(odd);
    }
    let (value, display_raw) = match (sub.number, sub.word.as_str()) {
        (Some(v), _) => (MeasuredValue::Normal(v), Some(sub.text.clone())),
        (None, "0L") => (MeasuredValue::Overload, None),
        _ => {
            report("secondary display");
            return None;
        }
    };
    Some(AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        elapsed_secs: None,
    })
}

/// The annunciators that are flags (spec §5.1, §5.5). CREST shows its MAX
/// or MIN beside [C] (BM860s manual p.12); [R] and [C] together are no
/// state the BM860s manual gives.
fn flags(lit: Lit, report: &impl Fn(&'static str)) -> StatusFlags {
    let crest = lit.has(Ann::Crest);
    let record = lit.has(Ann::Record);
    if crest && record {
        report("REC and CREST annunciators");
    }
    let (max, min, avg) = (lit.has(Ann::Max), lit.has(Ann::Min), lit.has(Ann::Avg));
    StatusFlags {
        auto_range: lit.has(Ann::Auto),
        hold: lit.has(Ann::Hold),
        rel: lit.has(Ann::Rel),
        record,
        max: max && !crest,
        min: min && !crest,
        avg: avg && !crest,
        peak_max: max && crest,
        peak_min: min && crest,
        low_battery: lit.has(Ann::LowBattery),
        ..StatusFlags::default()
    }
}

/// Every mode the decoder names, for the capture-step test.
#[cfg(test)]
pub(super) const MODES: [&str; 29] = [
    "T1",
    "T2",
    "T1-T2",
    "dBm reference",
    "dBm",
    "VFD Hz",
    "VFD AC V",
    "Continuity",
    "Ω",
    "nS",
    "Capacitance",
    "Hz",
    "Duty %",
    "Diode",
    "DC V",
    "AC V",
    "AC+DC V",
    "DC mV",
    "AC mV",
    "AC+DC mV",
    "DC µA",
    "AC µA",
    "AC+DC µA",
    "DC mA",
    "AC mA",
    "AC+DC mA",
    "DC A",
    "AC A",
    "AC+DC A",
];

#[cfg(test)]
pub(super) mod tests {
    use super::super::glyph::tests::cells as glyph_cells;
    use super::super::map::BM860;
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::test_support::snapshot;
    use Ann::*;

    /// The sheet's example (spec §9.1) as its hex column prints it, the
    /// "don't care" bytes `00`: 24 data bytes, bytes 1, 10 and 19 left out.
    pub(crate) fn example() -> Vec<u8> {
        let printed = [
            0x00, 0x00, 0x01, 0x11, 0xF8, 0xA0, 0xDA, 0xA9, 0xA0, 0x00, 0x00, 0x00, 0x7E, 0xBF,
            0xA0, 0xA0, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x86, 0x00, 0x00, 0x00, 0x00,
        ];
        printed
            .iter()
            .enumerate()
            .filter(|(i, _)| ![0, 9, 18].contains(i))
            .map(|(_, b)| *b)
            .collect()
    }

    /// A row's digit bytes for `text` (see `glyph::tests::cells`; `?` is an
    /// unlisted pattern, e and f), `negative` its minus.
    fn put_row(data: &mut [u8], row: &Row, text: &str, negative: bool) {
        let cells = if text.contains('?') {
            text.chars()
                .map(|c| match c {
                    '?' => Cell {
                        segments: 0x30,
                        point: false,
                    },
                    c => glyph_cells(&c.to_string())[0],
                })
                .collect()
        } else {
            glyph_cells(text)
        };
        assert_eq!(cells.len(), row.digits.len(), "{text:?}");
        for (i, cell) in cells.iter().enumerate() {
            let byte = &mut data[row.digits[i]];
            for (s, mask) in BM860.segments.iter().enumerate() {
                if cell.segments & 1 << s != 0 {
                    *byte |= mask;
                }
            }
            if cell.point {
                let (_, bit) = row
                    .points
                    .iter()
                    .find(|(after, _)| *after == i)
                    .unwrap_or_else(|| panic!("no point after digit {i} in {text:?}"));
                data[bit.index] |= bit.mask;
            }
        }
        if negative {
            data[row.minus.index] |= row.minus.mask;
        }
    }

    /// A BM860s reply showing `main` and `sub` with `anns` lit, the model
    /// bytes 20-23 `86`.
    pub(crate) fn lcd(main: &str, sub: &str, anns: &[Ann]) -> Vec<u8> {
        lcd_signed(main, false, sub, false, anns)
    }

    fn lcd_signed(main: &str, main_neg: bool, sub: &str, sub_neg: bool, anns: &[Ann]) -> Vec<u8> {
        let mut data = vec![0u8; REPLY_LEN];
        data[16..20].fill(0x86);
        put_row(&mut data, &BM860.main, main, main_neg);
        put_row(&mut data, &BM860.secondary, sub, sub_neg);
        for ann in anns {
            let (_, bit) = BM860.annunciators.iter().find(|(a, _)| a == ann).unwrap();
            data[bit.index] |= bit.mask;
        }
        data
    }

    const BLANK: &str = "    ";

    fn decoded(data: &[u8]) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(data, Series::Bm86x));
        (m.unwrap(), reports)
    }

    fn quiet(data: &[u8]) -> Measurement {
        let (m, reports) = decoded(data);
        assert!(reports.is_empty(), "{reports:?}");
        m
    }

    fn reported(data: &[u8], what: &str) -> Measurement {
        let (m, reports) = decoded(data);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].starts_with(&format!("bm86x: unrecognised {what}: reply [")),
            "{reports:?}"
        );
        m
    }

    fn normal(m: &Measurement) -> Option<f64> {
        match m.value {
            MeasuredValue::Normal(v) => Some(v),
            _ => None,
        }
    }

    /// Spec §9.1 as printed: the main V is not set, so AC alone names no
    /// function (one report), and the secondary display reads 6.011 Hz
    /// with 7p where the hex puts it.
    #[test]
    fn example_as_printed() {
        let m = reported(&example(), "function annunciators");
        assert_eq!(
            snapshot(&m),
            "mode=Unknown(0x0001)\n\
             mode_raw=0x01\n\
             range_raw=0x00\n\
             value=Normal(312.71)\n\
             unit=\n\
             range_label=\n\
             display_raw=Some(\"312.71\")\n\
             flags=auto_range\n\
             aux=1\n\
             aux1=Frequency value=Normal(6.011) unit=Hz display_raw=Some(\"6.011\") elapsed_secs=None\n\
             raw_payload=24"
        );
    }

    /// The example with the main V the caption and figure show: AC V.
    #[test]
    fn example_with_the_main_v() {
        let mut data = example();
        data[8] |= 0x01;
        assert_eq!(
            snapshot(&quiet(&data)),
            "mode=AC V\n\
             mode_raw=0x05\n\
             range_raw=0x00\n\
             value=Normal(312.71)\n\
             unit=V\n\
             range_label=\n\
             display_raw=Some(\"312.71\")\n\
             flags=auto_range\n\
             aux=1\n\
             aux1=Frequency value=Normal(6.011) unit=Hz display_raw=Some(\"6.011\") elapsed_secs=None\n\
             raw_payload=24"
        );
    }

    /// Hold Δ at DCV: six digits (BM860s manual p.12).
    #[test]
    fn six_digits_at_500000_counts() {
        let m = quiet(&lcd("12.3456", BLANK, &[Dc1, V1]));
        assert_eq!(m.display_raw.as_deref(), Some("12.3456"));
        assert_eq!(normal(&m), Some(12.3456));
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("DC V", "V"));
        assert!(m.flags.dc);
        let low = quiet(&lcd("0.50012", BLANK, &[Dc1, V1]));
        assert_eq!(low.display_raw.as_deref(), Some("0.50012"));
    }

    /// Digit 6 is blank at 50000 counts; blanks around the digits drop.
    #[test]
    fn blanks_around_the_digits_drop() {
        let m = quiet(&lcd_signed("0012.0 ", true, BLANK, false, &[Dc1, V1]));
        assert_eq!(m.display_raw.as_deref(), Some("-0012.0"));
        assert_eq!(normal(&m), Some(-12.0));
        let cap = quiet(&lcd(" 38.28 ", BLANK, &[Farad1, Nano1]));
        assert_eq!(cap.display_raw.as_deref(), Some("38.28"));
        assert_eq!(
            (cap.mode.as_ref(), cap.unit.as_ref()),
            ("Capacitance", "nF")
        );
    }

    /// The manual's temperature figures (BM860s manual p.8).
    #[test]
    fn a_letter_in_the_last_cell_is_the_temperature_unit() {
        let t1 = quiet(&lcd("0250.8C", BLANK, &[T1]));
        assert_eq!((t1.mode.as_ref(), t1.unit.as_ref()), ("T1", "°C"));
        assert_eq!(t1.display_raw.as_deref(), Some("0250.8"));
        assert_eq!(normal(&t1), Some(250.8));
        assert_eq!(t1.mode_raw, 0x8020);
        let f = quiet(&lcd("0483.4F", BLANK, &[T1]));
        assert_eq!((f.mode.as_ref(), f.unit.as_ref()), ("T1", "°F"));
        assert_eq!(f.mode_raw, 0x8040);
        let t2 = quiet(&lcd("0083.2F", BLANK, &[T2]));
        assert_eq!(t2.mode, "T2");
        // T1 +T2: the secondary is T2, in the main display's unit.
        let both = quiet(&lcd("0483.4F", "083.2", &[T1, T2Sub]));
        assert_eq!(both.mode, "T1");
        assert_eq!(both.main_label, Some(MainLabel::T1));
        let aux = &both.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("T2", "°F"));
        assert_eq!(aux.display_raw.as_deref(), Some("083.2"));
        // T1 and T2 lit are the difference, dash or not (spec §8.2).
        for dash in [&[T1, T2, T2Sub][..], &[T1, T1T2Dash, T2, T2Sub]] {
            let m = quiet(&lcd("0400.2F", "083.2", dash));
            assert_eq!(m.mode, "T1-T2");
            assert_eq!(m.main_label, None);
        }
        // A letter with neither probe named, or a unit lit beside it.
        reported(&lcd("0250.8C", BLANK, &[]), "function annunciators");
        reported(&lcd("0250.8C", BLANK, &[T1, V1]), "function annunciators");
        // T2 on the secondary display with no letter to take a unit from.
        reported(
            &lcd("12.345 ", "083.2", &[Dc1, V1, T2Sub]),
            "secondary annunciators",
        );
    }

    #[test]
    fn dashes_before_the_letter_are_no_reading() {
        let m = quiet(&lcd("  ---C", BLANK, &[T1]));
        assert_eq!(m.mode, "T1");
        assert!(matches!(m.value, MeasuredValue::NoReading("---")));
        // Dashes with no letter are no word the BM860s manual shows.
        reported(&lcd("  --- ", BLANK, &[Dc1, V1]), "display text");
    }

    /// The dBm figures (BM860s manual p.7-8): m is part of the label.
    #[test]
    fn dbm_and_its_reference_impedance() {
        let dbm = quiet(&lcd(" 53.83 ", "60.20", &[Db, Milli1, Hz2]));
        assert_eq!((dbm.mode.as_ref(), dbm.unit.as_ref()), ("dBm", "dBm"));
        assert_eq!(normal(&dbm), Some(53.83));
        assert_eq!(dbm.aux_values[0].label, "Frequency");
        let low = quiet(&lcd_signed("  05.33", true, BLANK, false, &[Db, Milli1]));
        assert_eq!(low.display_raw.as_deref(), Some("-05.33"));
        let reference = quiet(&lcd("   600", BLANK, &[Db, Milli1, Ohm1]));
        assert_eq!(
            (reference.mode.as_ref(), reference.unit.as_ref()),
            ("dBm reference", "Ω")
        );
        assert_eq!(normal(&reference), Some(600.0));
        assert_eq!(reference.mode_raw, 0x2080);
        // dB without its m reads the same; another prefix is reported.
        assert_eq!(quiet(&lcd(" 53.83 ", BLANK, &[Db])).mode, "dBm");
        reported(&lcd(" 53.83 ", BLANK, &[Db, Kilo1]), "prefix annunciators");
    }

    /// The VFD figures (BM860s manual p.6).
    #[test]
    fn vfd_functions() {
        let hz = quiet(&lcd("60.102 ", "120.3", &[Vfd, Hz1, Ac2, V2]));
        assert_eq!((hz.mode.as_ref(), hz.unit.as_ref()), ("VFD Hz", "Hz"));
        let aux = &hz.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("AC", "V"));
        assert_eq!(hz.main_label, None);
        let v = quiet(&lcd("120.32 ", "60.10", &[Vfd, Ac1, V1, Hz2]));
        assert_eq!((v.mode.as_ref(), v.unit.as_ref()), ("VFD AC V", "V"));
        assert_eq!(v.aux_values[0].label, "Frequency");
    }

    #[test]
    fn the_other_units() {
        for (anns, mode, unit) in [
            (&[Ohm1, Kilo1][..], "Ω", "kΩ"),
            (&[Ohm1, Mega1], "Ω", "MΩ"),
            (&[Ohm1], "Ω", "Ω"),
            (&[Ohm1, Continuity], "Continuity", "Ω"),
            (&[Siemens1, Nano1], "nS", "nS"),
            (&[Farad1, Micro1], "Capacitance", "µF"),
            (&[Farad1, Milli1], "Capacitance", "mF"),
            (&[Hz1], "Hz", "Hz"),
            (&[Hz1, Kilo1], "Hz", "kHz"),
            (&[Hz1, Mega1], "Hz", "MHz"),
            (&[Duty1], "Duty %", "%"),
            (&[V1], "Diode", "V"),
            (&[Dc1, V1], "DC V", "V"),
            (&[Ac1, V1], "AC V", "V"),
            (&[Dc1, Ac1, V1], "AC+DC V", "V"),
            (&[Dc1, V1, Milli1], "DC mV", "mV"),
            (&[Ac1, V1, Milli1], "AC mV", "mV"),
            (&[Dc1, Ac1, V1, Milli1], "AC+DC mV", "mV"),
            (&[Dc1, A1, Micro1], "DC µA", "µA"),
            (&[Ac1, A1, Micro1], "AC µA", "µA"),
            (&[Dc1, Ac1, A1, Micro1], "AC+DC µA", "µA"),
            (&[Dc1, A1, Milli1], "DC mA", "mA"),
            (&[Ac1, A1, Milli1], "AC mA", "mA"),
            (&[Dc1, Ac1, A1, Milli1], "AC+DC mA", "mA"),
            (&[Dc1, A1], "DC A", "A"),
            (&[Ac1, A1], "AC A", "A"),
            (&[Dc1, Ac1, A1], "AC+DC A", "A"),
        ] {
            let m = quiet(&lcd("50.107 ", BLANK, anns));
            assert_eq!((m.mode.as_ref(), m.unit.as_ref()), (mode, unit), "{anns:?}");
            assert_eq!(normal(&m), Some(50.107), "{anns:?}");
            assert_eq!(m.flags.dc, mode.starts_with("DC "), "{mode}");
            assert!(MODES.contains(&mode), "{mode}");
        }
    }

    /// Brymen's function word, as `mode_raw` (spec §8.1).
    #[test]
    fn mode_raw_is_the_function_word() {
        for (anns, word) in [
            (&[Dc1, V1][..], 0x0006),
            (&[Ac1, Dc1, A1], 0x0203),
            (&[Ohm1, Continuity], 0x0180),
            (&[Farad1, Nano1], 0x0008),
            (&[Hz1], 0x0400),
            (&[Duty1], 0x0800),
            (&[Siemens1, Nano1], 0x1000),
            (&[Db], 0x2000),
        ] {
            assert_eq!(quiet(&lcd("50.107 ", BLANK, anns)).mode_raw, word);
        }
    }

    /// DCV +ACV (BM860s manual p.7): the DC reading beside its AC one.
    #[test]
    fn the_ac_component_beside_a_dc_reading() {
        let m = quiet(&lcd_signed(
            "0012.0 ",
            true,
            "120.3",
            false,
            &[Dc1, V1, Ac2, V2],
        ));
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.main_label, Some(MainLabel::Dc));
        let aux = &m.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("AC", "V"));
        assert!(matches!(aux.value, MeasuredValue::Normal(v) if v == 120.3));
        let ma = quiet(&lcd(
            "12.028 ",
            "38.29",
            &[Dc1, A1, Milli1, Ac2, A2, Milli2],
        ));
        assert_eq!(ma.aux_values[0].unit, "mA");
        assert_eq!(ma.main_label, Some(MainLabel::Dc));
        // DC+AC with its AC: not the DC component.
        let acdc = quiet(&lcd("120.89 ", "120.3", &[Dc1, Ac1, V1, Ac2, V2]));
        assert_eq!(acdc.main_label, None);
        // The mV figure's secondary display reads mV.
        let mv = quiet(&lcd(
            "128.02 ",
            "30.20",
            &[Dc1, V1, Milli1, Ac2, V2, Milli2],
        ));
        assert_eq!(mv.aux_values[0].unit, "mV");
    }

    /// DC mA with the loop current's percentage (BM860s manual p.10-11).
    #[test]
    fn the_loop_percentage() {
        let m = quiet(&lcd_signed(
            "12.028 ",
            true,
            "50.17",
            false,
            &[Dc1, A1, Milli1, Loop],
        ));
        assert_eq!(m.mode, "DC mA");
        let aux = &m.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("4-20mA", "%"));
        assert_eq!(aux.display_raw.as_deref(), Some("50.17"));
        assert_eq!(m.main_label, None);
    }

    #[test]
    fn secondary_frequency_prefixes() {
        let m = quiet(&lcd("380.74 ", "60.20", &[Ac1, V1, Hz2]));
        assert_eq!(m.aux_values[0].unit, "Hz");
        let k = quiet(&lcd("380.74 ", "1.234", &[Ac1, V1, Hz2, Kilo2]));
        assert_eq!(k.aux_values[0].unit, "kHz");
        let ol = quiet(&lcd("380.74 ", " 0L ", &[Ac1, V1, Hz2]));
        assert!(matches!(ol.aux_values[0].value, MeasuredValue::Overload));
    }

    /// The diode figures (BM860s manual p.9): "diod" labels the function.
    #[test]
    fn diode_and_overload() {
        let m = quiet(&lcd("0.6002 ", "diod", &[V1]));
        assert_eq!(m.mode, "Diode");
        assert!(m.aux_values.is_empty());
        let open = quiet(&lcd(" .0L   ", "diod", &[V1]));
        assert!(matches!(open.value, MeasuredValue::Overload));
        assert_eq!(open.display_raw, None);
        let ohm = quiet(&lcd_signed("  0L  ", true, BLANK, false, &[Ohm1, Mega1]));
        assert!(matches!(ohm.value, MeasuredValue::Overload));
    }

    #[test]
    fn the_manuals_words() {
        // I drawn as a pattern §7.2 does not list, or as a 1.
        for text in ["?nEr  ", "1nEr  ", "  1nEr"] {
            let m = quiet(&lcd(text, BLANK, &[Dc1, V1]));
            assert!(
                matches!(m.value, MeasuredValue::NoReading("InEr")),
                "{text}"
            );
            assert!(m.flags.lead_error, "{text}");
            assert_eq!(m.mode, "DC V");
        }
        // Whatever is lit with them, the self-diagnosis words are silent.
        for (text, word) in [("rE-0  ", "rE-O"), ("  C_Er", "C_Er")] {
            let m = quiet(&lcd(text, BLANK, &[]));
            assert!(
                matches!(m.value, MeasuredValue::NoReading(w) if w == word),
                "{text}"
            );
            assert!(!m.flags.lead_error);
            assert_eq!(m.mode, "Unknown(0x0000)");
        }
    }

    #[test]
    fn unknown_words_and_glyphs_are_reported() {
        let m = reported(&lcd("  HELP", BLANK, &[Dc1, V1]), "display text");
        assert!(matches!(m.value, MeasuredValue::NoReading("?")));
        let m = reported(&lcd("12?4  ", BLANK, &[Dc1, V1]), "digit glyph");
        assert!(matches!(m.value, MeasuredValue::NoReading("?")));
        reported(&lcd("      ", BLANK, &[Dc1, V1]), "display text");
    }

    #[test]
    fn odd_digit_rows_are_reported() {
        let gap = reported(&lcd("12 45 ", BLANK, &[Dc1, V1]), "blank digit");
        assert!(matches!(gap.value, MeasuredValue::NoReading("?")));
        let two = reported(&lcd("1.2.345 ", BLANK, &[Dc1, V1]), "decimal points");
        assert_eq!(two.display_raw.as_deref(), Some("1.2345"));
    }

    #[test]
    fn nothing_lit_is_reported() {
        let m = reported(&lcd("12.345 ", BLANK, &[]), "function annunciators");
        assert_eq!(m.mode, "Unknown(0x0000)");
        for anns in [&[A1][..], &[Ohm1, Hz1], &[Ohm1, Db, Hz1]] {
            reported(&lcd("12.345 ", BLANK, anns), "function annunciators");
        }
    }

    #[test]
    fn prefixes_out_of_place_are_reported() {
        let m = reported(
            &lcd("12.345 ", BLANK, &[Dc1, V1, Kilo1, Mega1]),
            "prefix annunciators",
        );
        assert_eq!(m.unit, "V");
        let m = reported(
            &lcd("12.345 ", BLANK, &[Dc1, V1, Kilo1]),
            "prefix annunciators",
        );
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("DC V", "V"));
        reported(&lcd("12.345 ", BLANK, &[Farad1]), "prefix annunciators");
        reported(
            &lcd("12.345 ", "60.20", &[Ac1, V1, Hz2, Milli2]),
            "secondary annunciators",
        );
    }

    #[test]
    fn secondary_digits_with_no_unit_are_reported() {
        let m = reported(&lcd("12.345 ", "60.20", &[Dc1, V1]), "secondary display");
        assert!(m.aux_values.is_empty());
        reported(
            &lcd("12.345 ", "60.20", &[Dc1, V1, V2]),
            "secondary annunciators",
        );
        reported(
            &lcd("12.345 ", "60.20", &[Dc1, V1, Hz2, Loop]),
            "secondary annunciators",
        );
        reported(&lcd("12.345 ", "Auto", &[Dc1, V1]), "secondary display");
        reported(
            &lcd("12.345 ", "HELP", &[Dc1, V1, Hz2]),
            "secondary display",
        );
    }

    #[test]
    fn flags_follow_the_annunciators() {
        let f = |anns: &[Ann]| {
            let mut all = vec![Dc1, V1];
            all.extend_from_slice(anns);
            quiet(&lcd("12.345 ", BLANK, &all)).flags
        };
        assert!(f(&[Auto]).auto_range && !f(&[]).auto_range);
        assert!(f(&[Hold]).hold);
        assert!(f(&[Rel]).rel);
        assert!(f(&[LowBattery]).low_battery);
        let rec = f(&[Record, Max, Min, Avg]);
        assert!(rec.record && rec.max && rec.min && rec.avg);
        assert!(!rec.peak_max && !rec.peak_min);
        let max = f(&[Record, Max]);
        assert!(max.record && max.max && !max.min && !max.avg);
        let cmax = f(&[Crest, Max]);
        assert!(cmax.peak_max && !cmax.peak_min && !cmax.max && !cmax.record);
        let cmin = f(&[Crest, Min]);
        assert!(cmin.peak_min && !cmin.peak_max && !cmin.min);
        assert!(!f(&[Dc1, Ac1]).dc);
    }

    /// [R] and [C] together: the BM860s manual gives no such state.
    #[test]
    fn rec_and_crest_together_are_reported() {
        let m = reported(
            &lcd("12.345 ", BLANK, &[Dc1, V1, Record, Crest]),
            "REC and CREST annunciators",
        );
        assert!(m.flags.record);
        assert_eq!(normal(&m), Some(12.345));
    }

    /// Documented bits no reading shows stay silent: the bar graph's, the
    /// T1-T2 dash, and every "don't care" byte (spec §5.1, §5.4, §5.5).
    #[test]
    fn documented_bits_stay_silent() {
        let plain = quiet(&lcd("12.345 ", BLANK, &[Dc1, V1]));
        for anns in [&[BarScale][..], &[BarMinus], &[T1T2Dash]] {
            let mut all = vec![Dc1, V1];
            all.extend_from_slice(anns);
            let m = quiet(&lcd("12.345 ", BLANK, &all));
            assert_eq!(snapshot(&m), snapshot(&plain), "{anns:?}");
        }
        for index in [0].into_iter().chain(16..19).chain(20..24) {
            let mut data = lcd("12.345 ", BLANK, &[Dc1, V1]);
            data[index] = 0xFF;
            let m = quiet(&data);
            assert_eq!(m.mode, "DC V", "{index}");
            assert_eq!(m.display_raw.as_deref(), Some("12.345"), "{index}");
        }
    }

    #[test]
    fn the_payload_is_checked_before_it_is_read() {
        for len in [0, 23, 25, 27] {
            let mut data = example();
            data.resize(len, 0);
            assert!(
                matches!(
                    decode(&data, Series::Bm86x),
                    Err(Error::InvalidResponse { .. })
                ),
                "{len}"
            );
        }
        let mut other = example();
        other[MODEL] = 0x82;
        assert!(matches!(
            decode(&other, Series::Bm86x),
            Err(Error::InvalidResponse { .. })
        ));
    }

    /// A small xorshift generator, so arbitrary input needs no dependency.
    fn pseudo_random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    #[test]
    fn arbitrary_bytes_never_panic() {
        let mut seed = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            let mut data = example();
            for (i, byte) in data.iter_mut().enumerate() {
                if i != MODEL {
                    *byte = pseudo_random(&mut seed) as u8;
                }
            }
            let _ = capture_reports(|| decode(&data, Series::Bm86x));
        }
    }
}
