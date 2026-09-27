//! A reply's segments → [`Measurement`]
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §5-§8).
//!
//! The reply is the LCD: digits as seven-segment patterns, and function,
//! unit, prefix, sign and decimal point as annunciators, with no function
//! or range code (spec §5.1, §6.1). The reading is rebuilt from what is
//! lit, as Brymen's programs do (spec §8): the value in the unit the
//! display shows, the function from the unit and coupling annunciators, and
//! the secondary display as one sub-value.
//!
//! Every bit lands in one of three places: the reading; silence, for what
//! the sheets document and no reading shows (the bar graph, the dashes
//! between symbols, Hi, Lo, LPF, @, the "don't care" bytes); or a report,
//! for what the spec does not cover.

use super::glyph::{self, Cell, Readout, UNKNOWN};
use super::map::{Ann, Lit, MODEL, Map, Row};
use super::reply::{self, MODEL_RUN, REPLY_LEN};
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
    if reply::find_reply(data, series).is_none() {
        return Err(Error::invalid_response(
            format!(
                "{id} reply's model bytes are {:02X?}, expected {:02X}",
                &data[MODEL_RUN..=MODEL],
                series.code()
            ),
            data,
        ));
    }
    let map = series.map();
    let lit = Lit::read(map, data);
    let report = |what| report(series, data, what);

    let mut main_cells = cells(map, &map.main, data);
    let probe = lit.has(Ann::T1) || lit.has(Ann::T2);
    let letter = temperature_letter(&mut main_cells, probe);
    let mode_raw = function_word(lit, letter);

    // Recall on a logging model lights R and C together (BM820s manual
    // p.16, spec §6.5): what shows is logged data or its page and item
    // numbers, not a reading.
    if series.logs() && lit.has(Ann::Record) && lit.has(Ann::Crest) {
        return Ok(Measurement {
            mode: Cow::Borrowed(MODE_RECALL),
            mode_raw,
            value: MeasuredValue::NoReading(RECALL),
            unit: Cow::Borrowed(""),
            display_raw: None,
            ..Measurement::from_payload(data)
        });
    }

    let main = glyph::read(&main_cells, map.main.minus.lit(data));
    let shown = main_shown(&main, letter.is_some(), lit, series, &report);

    let function = function(lit, letter);
    let (mode, unit) = match (shown.mode(), function) {
        (Some(mode), _) => (Cow::Borrowed(mode), ""),
        (None, Some(function)) => {
            let prefix = main_prefix(lit, &report);
            let unit = unit(function.base, prefix).unwrap_or_else(|| {
                report("prefix annunciators");
                function.base.bare()
            });
            let mode = match function.mode {
                Mode::Named(mode) => mode,
                Mode::Coupled(coupling) => coupled(coupling, unit),
            };
            if !lit.has(Ann::LoZ) {
                (Cow::Borrowed(mode), unit)
            } else if let Some(loz) = low_impedance(mode) {
                (Cow::Borrowed(loz), unit)
            } else {
                report("function annunciators");
                (unknown_mode16(mode_raw), "")
            }
        }
        (None, None) => {
            // A word the manuals show explains itself, whatever is lit.
            if !matches!(shown, Shown::Word(_)) {
                report("function annunciators");
            }
            (unknown_mode16(mode_raw), "")
        }
    };

    let mut sub_cells = cells(map, &map.secondary, data);
    // The BM820s puts a temperature's own letter in the last secondary
    // digit; the BM860s none, and its T2 takes the main one's (spec §6.2).
    let sub_probe = probe || lit.has(Ann::T1Sub) || lit.has(Ann::T2Sub);
    let sub_letter = temperature_letter(&mut sub_cells, sub_probe);
    let sub = glyph::read(&sub_cells, map.secondary.minus.lit(data));
    let aux = secondary(lit, &sub, sub_letter.or(letter), &report);
    let main_label = match (&aux, function.map(|f| f.mode)) {
        (Some(a), Some(Mode::Coupled(Coupling::Dc))) if a.label == LABEL_AC => Some(MainLabel::Dc),
        (Some(a), Some(Mode::Named(MODE_T1))) if a.label == LABEL_T2 => Some(MainLabel::T1),
        (Some(a), Some(Mode::Named(MODE_T2))) if a.label == LABEL_T1 => Some(MainLabel::T2),
        _ => None,
    };

    let lead_error = matches!(shown, Shown::Word(INPUT_ERROR));
    let (value, display_raw) = match shown {
        Shown::Number(v, text) => (MeasuredValue::Normal(v), Some(text)),
        Shown::Overload => (MeasuredValue::Overload, None),
        Shown::Field(level) => (MeasuredValue::NcvLevel(level), None),
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

/// A `C` or `F` in a row's last cell, after something else lit, is the
/// temperature unit (spec §5.2, §6.2): taken off the row, it is returned.
/// Only with a `probe`, T1 or T2, lit: EF's word may end in its F (spec
/// §7.3).
fn temperature_letter(cells: &mut Vec<Cell>, probe: bool) -> Option<char> {
    if !probe {
        return None;
    }
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

/// AutoCheck waiting for an input (spec §7.3, §11.3).
const AUTO: &str = "Auto";

/// A logging model in Recall, or showing a session page (spec §7.3).
const RECALL: &str = "Recall";
const MODE_RECALL: &str = "Recall";

/// What the main display shows.
enum Shown {
    Number(f64, String),
    Overload,
    /// EF detection: 0 for "E.F.", ready; else the dashes lit.
    Field(u8),
    /// A word the manuals show (spec §7.3).
    Word(&'static str),
    /// Something no source shows, already reported.
    Unread(&'static str),
}

impl Shown {
    /// The mode a word names on its own, whatever is lit.
    fn mode(&self) -> Option<&'static str> {
        match self {
            Shown::Word(AUTO) => Some("Auto V"),
            Shown::Field(_) => Some("EF"),
            Shown::Word(RECALL) => Some(MODE_RECALL),
            _ => None,
        }
    }
}

/// Read the main display: a number, or a word in the manuals' spelling
/// (spec §7.3). `letter` says a temperature unit followed the row.
fn main_shown(
    main: &Readout,
    letter: bool,
    lit: Lit,
    series: Series,
    report: &impl Fn(&'static str),
) -> Shown {
    let word = main.word.as_str();
    // "E.F.": which digits and points the meter lights is open (spec
    // §7.3), so the points are not read.
    if series.detects_fields() && word == "EF" {
        return Shown::Field(0);
    }
    if let Some(odd) = main.odd {
        report(odd);
    }
    if let Some(v) = main.number {
        return Shown::Number(v, main.text.clone());
    }
    if main.odd == Some("blank digit") {
        return Shown::Unread("?");
    }
    let dashes = !word.is_empty() && word.chars().all(|c| c == '-');
    match word {
        // OL, or .OL (BM860s manual p.10): the letters themselves (spec
        // §8.4).
        "0L" => Shown::Overload,
        // How the meter draws the I is open: either reading of it (spec
        // §7.3).
        "?nEr" | "1nEr" => Shown::Word(INPUT_ERROR),
        // The power-on self-diagnosis (BM860s manual p.15, BM820s manual
        // p.17).
        "rE-0" => Shown::Word("rE-O"),
        "C_Er" => Shown::Word("C_Er"),
        // AutoCheck with no input, LoZ lit (BM820s manual p.6).
        "Auto" if lit.has(Ann::LoZ) => Shown::Word(AUTO),
        // Dashes where a temperature would be.
        _ if dashes && letter => Shown::Word("---"),
        // The field strength, the minus counted with the dashes after it
        // (BM820s manual p.13); how many light at each strength is open
        // (spec §7.3).
        _ if dashes && series.detects_fields() => {
            let marks = main.text.chars().filter(|c| *c == '-').count();
            Shown::Field(u8::try_from(marks).unwrap_or(u8::MAX))
        }
        _ => match logging_word(word).filter(|_| series.logs()) {
            Some(logged) => Shown::Word(logged),
            None => {
                report(if word.contains(UNKNOWN) {
                    "digit glyph"
                } else {
                    "display text"
                });
                Shown::Unread("?")
            }
        },
    }
}

/// A logging model's word, in the manual's spelling (BM820s manual
/// p.14-16, spec §7.3): the logging states, the interval ("t0.05") and a
/// Recall session page ("P.001").
fn logging_word(word: &str) -> Option<&'static str> {
    let number_after = |lead: char| {
        word.strip_prefix(lead)
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    Some(match word {
        "LEFt" => "LEFt",
        "5trt" => "Strt",
        "PAU5" => "PAUS",
        "Cont" => "Cont",
        "5toP" => "StoP",
        _ if number_after('t') => "interval",
        _ if number_after('P') => RECALL,
        _ => return None,
    })
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

/// The unit a reading shows with `prefix`, for the ranges the manuals list
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

/// The main display's coupling, ⎓ and ∿ (spec §5.1, §6.1).
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
const MODE_T2: &str = "T2";
const LABEL_T1: &str = "T1";
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

/// AutoCheck's readings, LoZ lit: DC V, AC V or Ω, the continuity beep
/// with it (BM820s manual p.6, spec §6.5). `None` for any other function.
fn low_impedance(mode: &str) -> Option<&'static str> {
    Some(match mode {
        "DC V" => "LoZ DC V",
        "AC V" => "LoZ AC V",
        "Ω" | "Continuity" => "LoZ Ω",
        _ => return None,
    })
}

/// The function what is lit shows, by the first rule that matches (spec
/// §8.1, §8.2); `None` for a combination no source gives. `letter` is the
/// temperature unit in the digits.
fn function(lit: Lit, letter: Option<char>) -> Option<Function> {
    use Ann::*;
    const UNITS: [Ann; 8] = [V1, A1, Ohm1, Farad1, Hz1, Siemens1, Duty1, Db];
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
        if lit.among(UNITS).next().is_some() {
            return None;
        }
        return match (lit.has(Ann::T1), lit.has(Ann::T2)) {
            (true, true) => Some(named("T1-T2", base)),
            (true, false) => Some(named(MODE_T1, base)),
            (false, true) => Some(named(MODE_T2, base)),
            (false, false) => None,
        };
    }
    // The reference impedance, shown for a second as dBm starts ("600" with
    // Ω and dBm lit; BM860s manual p.7, BM820s manual p.8, spec §8.2
    // `0x2080`).
    if lit.among(UNITS).eq([Ohm1, Db]) {
        return Some(named("dBm reference", Base::Ohm));
    }
    Some(match lit.one_of(UNITS)? {
        Db => named("dBm", Base::Dbm),
        Hz1 if lit.has(Vfd) => named("VFD Hz", Base::Hertz),
        V1 if lit.has(Vfd) && coupling == Coupling::Ac => named("VFD AC V", Base::Volt),
        Ohm1 if lit.has(Continuity) => named("Continuity", Base::Ohm),
        Ohm1 => named("Ω", Base::Ohm),
        Siemens1 => named("nS", Base::Siemens),
        Farad1 => named("Capacitance", Base::Farad),
        // Line and logic frequency look the same on the LCD.
        Hz1 => named("Hz", Base::Hertz),
        Duty1 => named("Duty %", Base::Percent),
        // V with neither coupling: the diode test (spec §8.2).
        V1 if coupling == Coupling::None => named("Diode", Base::Volt),
        V1 => Function {
            mode: Mode::Coupled(coupling),
            base: Base::Volt,
        },
        A1 if coupling != Coupling::None => Function {
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
/// §5.1, §6.1): the frequency, the AC or DC component, T1 or T2, or the
/// loop current's percentage. Blank, or a label word — "diod" (spec §7.3),
/// "Auto" in AutoCheck (BM820s manual p.6) — is none. `letter` is the
/// temperature unit, the secondary display's own or the main one's.
fn secondary(
    lit: Lit,
    sub: &Readout,
    letter: Option<char>,
    report: &impl Fn(&'static str),
) -> Option<AuxValue> {
    use Ann::*;
    const UNITS: [Ann; 9] = [Hz2, V2, A2, T2Sub, T1Sub, Loop, Ohm2, Farad2, Siemens2];
    let prefix = one_prefix(
        lit,
        &[
            (Nano2, Prefix::Nano),
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
    let temperature = |label| match letter {
        Some('C') => Some((label, "°C")),
        Some(_) => Some((label, "°F")),
        None => None,
    };
    let label_word =
        sub.word.is_empty() || sub.word == "diod" || (sub.word == AUTO && lit.has(LoZ));
    if lit.among(UNITS).next().is_none() {
        if !label_word {
            report("secondary display");
        }
        return None;
    }
    let found = match lit.one_of(UNITS) {
        Some(Hz2) => Some(("Frequency", prefixed(Base::Hertz))),
        Some(unit @ (V2 | A2)) => {
            let base = if unit == V2 { Base::Volt } else { Base::Amp };
            match (lit.has(Ac2), lit.has(Dc2)) {
                (true, false) => Some((LABEL_AC, prefixed(base))),
                (false, true) => Some(("DC", prefixed(base))),
                _ => None,
            }
        }
        Some(T2Sub) => temperature(LABEL_T2),
        Some(T1Sub) => temperature(LABEL_T1),
        Some(Loop) => Some(("4-20mA", "%")),
        _ => None,
    };
    let Some((label, unit)) = found else {
        report("secondary annunciators");
        return None;
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

/// The annunciators that are flags (spec §5.1, §5.5, §6.1, §6.5). CREST
/// shows its MAX or MIN beside [C] (BM860s manual p.12, BM820s manual
/// p.13); [R] and [C] together are Recall on a logging model, which
/// `decode` reads first, and no state any other series' manual gives.
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
        loz: lit.has(Ann::LoZ),
        ..StatusFlags::default()
    }
}

/// Every mode the decoder names, for the capture-step test.
#[cfg(test)]
pub(super) const MODES: [&str; 35] = [
    "Recall",
    "Auto V",
    "EF",
    "LoZ DC V",
    "LoZ AC V",
    "LoZ Ω",
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
    use super::super::map::{at, data_index};
    use super::super::reply::tests::bm820_example;
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
    fn put_row(data: &mut [u8], map: &Map, row: &Row, text: &str, negative: bool) {
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
            for (s, mask) in map.segments.iter().enumerate() {
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
        lcd_on(Series::Bm86x, main, main_neg, sub, sub_neg, anns)
    }

    /// A reply of `series` showing `main` and `sub` with `anns` lit, the
    /// model bytes 20-23 the series code.
    fn lcd_on(
        series: Series,
        main: &str,
        main_neg: bool,
        sub: &str,
        sub_neg: bool,
        anns: &[Ann],
    ) -> Vec<u8> {
        let map = series.map();
        let mut data = vec![0u8; REPLY_LEN];
        data[MODEL_RUN..=MODEL].fill(series.code());
        put_row(&mut data, map, &map.main, main, main_neg);
        put_row(&mut data, map, &map.secondary, sub, sub_neg);
        for ann in anns {
            let (_, bit) = map.annunciators.iter().find(|(a, _)| a == ann).unwrap();
            data[bit.index] |= bit.mask;
        }
        data
    }

    const BLANK: &str = "    ";

    fn decoded_as(data: &[u8], series: Series) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(data, series));
        (m.unwrap(), reports)
    }

    fn quiet(data: &[u8]) -> Measurement {
        quiet_as(data, Series::Bm86x)
    }

    fn quiet_as(data: &[u8], series: Series) -> Measurement {
        let (m, reports) = decoded_as(data, series);
        assert!(reports.is_empty(), "{reports:?}");
        m
    }

    fn reported(data: &[u8], what: &str) -> Measurement {
        reported_as(data, Series::Bm86x, what)
    }

    fn reported_as(data: &[u8], series: Series, what: &str) -> Measurement {
        let (m, reports) = decoded_as(data, series);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].starts_with(&format!("{}: unrecognised {what}: reply [", series.id())),
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
        // A letter with neither probe lit is no unit: the row is text.
        let (m, reports) = decoded_as(&lcd("0250.8C", BLANK, &[]), Series::Bm86x);
        assert!(matches!(m.value, MeasuredValue::NoReading("?")));
        assert!(
            reports.iter().any(|r| r.contains("display text")),
            "{reports:?}"
        );
        // A unit lit beside it.
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
        for series in Series::ALL {
            for _ in 0..20_000 {
                let mut data: Vec<u8> = (0..REPLY_LEN)
                    .map(|_| pseudo_random(&mut seed) as u8)
                    .collect();
                data[MODEL_RUN..=MODEL].fill(series.code());
                let _ = capture_reports(|| decode(&data, series));
            }
        }
    }

    // The BM820 map: the BM82x and the BM52x.

    const BM820_SERIES: [Series; 2] = [Series::Bm82x, Series::Bm52x];

    fn lcd8(series: Series, main: &str, sub: &str, anns: &[Ann]) -> Vec<u8> {
        lcd_on(series, main, false, sub, false, anns)
    }

    /// Set Brymen's bit `byte.bit` in `data`, as the sheet numbers it.
    fn set(data: &mut [u8], byte: usize, bit: u32) {
        let bit = at(byte, bit);
        data[bit.index] |= bit.mask;
    }

    /// The BM820 sheet's example (spec §9.2) with digit 5 in byte 11, as
    /// Table 1 has it (spec §4.3): AC 380.1 V and 50.12 Hz, AUTO lit.
    #[test]
    fn bm820_example_with_report_ii_at_byte_10() {
        for series in BM820_SERIES {
            let m = quiet_as(&bm820_example(series.code()), series);
            assert_eq!(
                snapshot(&m),
                "mode=AC V\n\
                 mode_raw=0x05\n\
                 range_raw=0x00\n\
                 value=Normal(380.1)\n\
                 unit=V\n\
                 range_label=\n\
                 display_raw=Some(\"380.1\")\n\
                 flags=auto_range\n\
                 aux=1\n\
                 aux1=Frequency value=Normal(50.12) unit=Hz display_raw=Some(\"50.12\") elapsed_secs=None\n\
                 raw_payload=24",
                "{series:?}"
            );
        }
    }

    /// A BM820 reply read as another series' is refused.
    #[test]
    fn a_bm820_reply_needs_its_own_model_bytes() {
        let bm82x = bm820_example(0x82);
        assert!(decode(&bm82x, Series::Bm52x).is_err());
        assert!(decode(&bm82x, Series::Bm86x).is_err());
        let mut three = bm82x.clone();
        three[MODEL_RUN] = 0x00;
        assert!(decode(&three, Series::Bm82x).is_err());
    }

    /// Brymen's function word from the BM820 bits at their sheet
    /// positions (spec §8.1, the Bs8252x column), and the mode each gives.
    #[test]
    fn bm820_function_bits_are_table_1s() {
        for (bits, word, mode, unit) in [
            (&[(3, 5), (17, 5)][..], 0x0006, "DC V", "V"),
            (&[(3, 4), (17, 5)], 0x0005, "AC V", "V"),
            (&[(3, 4), (3, 5), (17, 5)], 0x0007, "AC+DC V", "V"),
            (&[(3, 4), (3, 5), (17, 4)], 0x0203, "AC+DC A", "A"),
            (&[(17, 0), (17, 3)], 0x0008, "Capacitance", "nF"),
            (&[(16, 5)], 0x0080, "Ω", "Ω"),
            (&[(16, 5), (9, 0)], 0x0180, "Continuity", "Ω"),
            (&[(16, 4)], 0x0400, "Hz", "Hz"),
            (&[(17, 2)], 0x0800, "Duty %", "%"),
            (&[(17, 1), (17, 3)], 0x1000, "nS", "nS"),
            (&[(8, 4), (17, 6)], 0x2000, "dBm", "dBm"),
            (&[(17, 5)], 0x0004, "Diode", "V"),
        ] {
            for series in BM820_SERIES {
                let mut data = lcd8(series, "50.10", BLANK, &[]);
                for (byte, bit) in bits {
                    set(&mut data, *byte, *bit);
                }
                let m = quiet_as(&data, series);
                assert_eq!(
                    (m.mode.as_ref(), m.unit.as_ref(), m.mode_raw),
                    (mode, unit, word),
                    "{bits:?}"
                );
                assert_eq!(normal(&m), Some(50.1));
            }
        }
    }

    /// The prefix bits at their sheet positions (spec §8.3, the Bs8252x
    /// rows), and the minus signs (spec §6.3).
    #[test]
    fn bm820_prefix_and_sign_bits_are_table_1s() {
        let s = Series::Bm82x;
        for (bits, unit) in [
            (&[(16, 5), (16, 7)][..], "kΩ"),
            (&[(16, 5), (16, 6)], "MΩ"),
            (&[(3, 5), (17, 5), (17, 6)], "mV"),
            (&[(3, 5), (17, 4), (17, 7)], "µA"),
            (&[(17, 0), (17, 3)], "nF"),
        ] {
            let mut data = lcd8(s, "50.10", BLANK, &[]);
            for (byte, bit) in bits {
                set(&mut data, *byte, *bit);
            }
            assert_eq!(quiet_as(&data, s).unit, unit, "{bits:?}");
        }
        for (bits, unit) in [
            (&[(16, 0), (16, 2)][..], "kHz"),
            (&[(16, 0), (16, 3)], "MHz"),
            (&[(9, 6), (15, 4), (15, 7)], "mV"),
            (&[(9, 6), (15, 5), (15, 6)], "µA"),
        ] {
            let mut data = lcd8(s, "50.10", "1.234", &[Dc1, V1]);
            for (byte, bit) in bits {
                set(&mut data, *byte, *bit);
            }
            assert_eq!(quiet_as(&data, s).aux_values[0].unit, unit, "{bits:?}");
        }
        // n on the small display is a prefix no frequency takes.
        let mut nano = lcd8(s, "50.10", "1.234", &[Dc1, V1]);
        set(&mut nano, 16, 0);
        set(&mut nano, 15, 2);
        reported_as(&nano, s, "secondary annunciators");
        let mut negative = lcd8(s, "50.10", "1.234", &[Dc1, V1, Hz2]);
        set(&mut negative, 4, 7);
        set(&mut negative, 9, 5);
        let m = quiet_as(&negative, s);
        assert_eq!(normal(&m), Some(-50.1));
        assert_eq!(m.aux_values[0].display_raw.as_deref(), Some("-1.234"));
    }

    /// The BM820 flags at their sheet positions (spec §6.1, §6.5).
    #[test]
    fn bm820_flag_bits_are_table_1s() {
        let f = |bits: &[(usize, u32)]| {
            let mut data = lcd8(Series::Bm82x, "12.34", BLANK, &[Dc1, V1]);
            for (byte, bit) in bits {
                set(&mut data, *byte, *bit);
            }
            quiet_as(&data, Series::Bm82x).flags
        };
        assert!(f(&[(24, 4)]).auto_range && !f(&[]).auto_range);
        assert!(f(&[(24, 7)]).hold);
        assert!(f(&[(4, 6)]).rel);
        assert!(f(&[(9, 3)]).low_battery);
        assert!(f(&[(4, 4)]).loz && !f(&[]).loz);
        let rec = f(&[(24, 5), (3, 0), (3, 3), (3, 1)]);
        assert!(rec.record && rec.max && rec.min && rec.avg);
        // MAX-MIN: both, and the dash between them silent.
        let max_min = f(&[(24, 5), (3, 0), (3, 3), (3, 2)]);
        assert!(max_min.record && max_min.max && max_min.min && !max_min.avg);
        let cmax = f(&[(24, 6), (3, 0)]);
        assert!(cmax.peak_max && !cmax.max && !cmax.record);
        let cmin = f(&[(24, 6), (3, 3)]);
        assert!(cmin.peak_min && !cmin.min);
        assert!(f(&[]).dc);
    }

    /// Documented bits no reading shows stay silent: Hi, Lo, the MAX-MIN
    /// and T1-T2 dashes, %, LPF, @, the small D%, the programs' "mV" bit
    /// 18.3 and every "don't care" bit (spec §6.1, §6.5, §8.1).
    #[test]
    fn bm820_documented_bits_stay_silent() {
        for series in BM820_SERIES {
            let plain_data = lcd8(series, "12.34", BLANK, &[Dc1, V1]);
            let plain = quiet_as(&plain_data, series);
            for (byte, bit) in [
                (3, 7),
                (3, 6),
                (3, 2),
                (4, 5),
                (4, 2),
                (4, 1),
                (9, 4),
                (15, 3),
                (18, 3),
                (24, 0),
                (24, 3),
            ] {
                let mut data = plain_data.clone();
                set(&mut data, byte, bit);
                assert_eq!(
                    snapshot(&quiet_as(&data, series)),
                    snapshot(&plain),
                    "{byte}.{bit}"
                );
            }
            for byte in [2, 18, 25, 26, 27] {
                let mut data = plain_data.clone();
                data[data_index(byte).unwrap()] = 0xFF;
                let m = quiet_as(&data, series);
                assert_eq!(m.mode, "DC V", "{byte}");
                assert_eq!(m.display_raw.as_deref(), Some("12.34"), "{byte}");
            }
        }
    }

    /// The BM820s manual's temperature figures (p.11): the unit letter in
    /// the last digit of each display.
    #[test]
    fn bm820_temperatures() {
        let s = Series::Bm52x;
        let t1 = quiet_as(&lcd8(s, "205C", BLANK, &[T1]), s);
        assert_eq!((t1.mode.as_ref(), t1.unit.as_ref()), ("T1", "°C"));
        assert_eq!(
            (t1.display_raw.as_deref(), normal(&t1)),
            (Some("205"), Some(205.0))
        );
        let t2 = quiet_as(&lcd8(s, "325F", BLANK, &[T2]), s);
        assert_eq!((t2.mode.as_ref(), t2.unit.as_ref()), ("T2", "°F"));
        // T1 +T2: the small display's own letter.
        let both = quiet_as(&lcd8(s, "401F", "325F", &[T1, T2Sub]), s);
        assert_eq!(both.mode, "T1");
        assert_eq!(both.main_label, Some(MainLabel::T1));
        let aux = &both.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("T2", "°F"));
        assert_eq!(aux.display_raw.as_deref(), Some("325"));
        // (T1-T2) +T2, dash lit or not (spec §8.2).
        for dash in [&[T1, T2, T2Sub][..], &[T1, T1T2Dash, T2, T2Sub]] {
            let m = quiet_as(&lcd8(s, "076F", "325F", dash), s);
            assert_eq!(m.mode, "T1-T2");
            assert_eq!(m.main_label, None);
        }
        // T1 on the small display beside T2 on the main one.
        let swapped = quiet_as(&lcd8(s, "325F", "401F", &[T2, T1Sub]), s);
        assert_eq!(swapped.main_label, Some(MainLabel::T2));
        assert_eq!(swapped.aux_values[0].label, "T1");
        // A small T1 with no letter to take a unit from.
        reported_as(
            &lcd8(s, "12.34", "401 ", &[Dc1, V1, T1Sub]),
            s,
            "secondary annunciators",
        );
    }

    /// The BM829s's dBm figures (BM820s manual p.7-9).
    #[test]
    fn bm820_dbm_and_its_reference_impedance() {
        let s = Series::Bm82x;
        let dbm = quiet_as(&lcd8(s, "64.62", "60.08", &[Db, Milli1, Hz2]), s);
        assert_eq!((dbm.mode.as_ref(), dbm.unit.as_ref()), ("dBm", "dBm"));
        assert_eq!(normal(&dbm), Some(64.62));
        assert_eq!(dbm.aux_values[0].label, "Frequency");
        let reference = quiet_as(&lcd8(s, "  50", BLANK, &[Db, Milli1, Ohm1]), s);
        assert_eq!(
            (reference.mode.as_ref(), reference.unit.as_ref()),
            ("dBm reference", "Ω")
        );
    }

    /// The small display's AC and DC components (spec §6.1): DCV +ACV
    /// (BM820s manual p.8), and DC on the small display, which the map has.
    #[test]
    fn bm820_secondary_components() {
        let s = Series::Bm82x;
        let m = quiet_as(
            &lcd_on(s, "003.5", true, "109.8", false, &[Dc1, V1, Ac2, V2]),
            s,
        );
        assert_eq!(
            (m.mode.as_ref(), m.main_label),
            ("DC V", Some(MainLabel::Dc))
        );
        assert_eq!(m.display_raw.as_deref(), Some("-003.5"));
        let aux = &m.aux_values[0];
        assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("AC", "V"));
        let dc = quiet_as(&lcd8(s, "109.8", "003.5", &[Ac1, V1, Dc2, V2]), s);
        assert_eq!(dc.aux_values[0].label, "DC");
        assert_eq!(dc.main_label, None);
        let ma = quiet_as(&lcd8(s, "20.60", "50.18", &[Ac1, A1, Milli1, Hz2]), s);
        assert_eq!(
            (ma.mode.as_ref(), ma.aux_values[0].unit.as_ref()),
            ("AC mA", "Hz")
        );
        reported_as(
            &lcd8(s, "109.8", "003.5", &[Ac1, V1, Ac2, Dc2, V2]),
            s,
            "secondary annunciators",
        );
        reported_as(
            &lcd8(s, "109.8", "003.5", &[Ac1, V1, Ohm2]),
            s,
            "secondary annunciators",
        );
    }

    /// AutoCheck (BM820s manual p.6): "Auto" and LoZ while it waits, then
    /// the function it picks, LoZ lit and "Auto" on the small display.
    #[test]
    fn autocheck() {
        for series in BM820_SERIES {
            let idle = quiet_as(&lcd8(series, "Auto", BLANK, &[LoZ]), series);
            assert_eq!(idle.mode, "Auto V");
            assert!(matches!(idle.value, MeasuredValue::NoReading("Auto")));
            assert!(idle.flags.loz);
            for (anns, mode, unit) in [
                (&[LoZ, Ac1, V1][..], "LoZ AC V", "V"),
                (&[LoZ, Dc1, V1], "LoZ DC V", "V"),
                (&[LoZ, Ohm1, Kilo1], "LoZ Ω", "kΩ"),
                (&[LoZ, Ohm1, Continuity], "LoZ Ω", "Ω"),
            ] {
                let m = quiet_as(&lcd8(series, "220.8", "Auto", anns), series);
                assert_eq!((m.mode.as_ref(), m.unit.as_ref()), (mode, unit), "{anns:?}");
                assert_eq!(normal(&m), Some(220.8));
                assert!(m.flags.loz && m.aux_values.is_empty());
            }
            // A function AutoCheck does not pick.
            reported_as(
                &lcd8(series, "220.8", "Auto", &[LoZ, Dc1, V1, Milli1]),
                series,
                "function annunciators",
            );
            // "Auto" without LoZ is no word the manual shows.
            reported_as(
                &lcd8(series, "Auto", BLANK, &[Dc1, V1]),
                series,
                "display text",
            );
            reported_as(
                &lcd8(series, "220.8", "Auto", &[Dc1, V1]),
                series,
                "secondary display",
            );
        }
    }

    /// EF detection (BM820s manual p.12-13): "E.F." when ready, then the
    /// minus and dashes for the field.
    #[test]
    fn ef_detection() {
        let s = Series::Bm82x;
        // Where the word sits is open (spec §7.3): an F in the last digit
        // is no °F while neither T1 nor T2 is lit.
        for text in ["E.F.  ", "  EF", "  E.F"] {
            let ready = quiet_as(&lcd8(s, text, BLANK, &[]), s);
            assert_eq!(
                (ready.mode.as_ref(), ready.unit.as_ref()),
                ("EF", ""),
                "{text}"
            );
            assert!(matches!(ready.value, MeasuredValue::NcvLevel(0)), "{text}");
        }
        for (text, minus, level) in [
            (" ---", true, 4),
            ("----", true, 5),
            ("--- ", false, 3),
            ("-   ", false, 1),
        ] {
            let m = quiet_as(&lcd_on(s, text, minus, BLANK, false, &[]), s);
            assert_eq!(m.mode, "EF", "{text}");
            assert!(
                matches!(m.value, MeasuredValue::NcvLevel(l) if l == level),
                "{text}: {:?}",
                m.value
            );
        }
        // The logging models have no EF (spec §11.1).
        let (m, reports) = decoded_as(&lcd8(Series::Bm52x, "E.F.  ", BLANK, &[]), Series::Bm52x);
        assert!(matches!(m.value, MeasuredValue::NoReading("?")));
        assert!(
            reports.iter().any(|r| r.contains("display text")),
            "{reports:?}"
        );
        reported_as(
            &lcd_on(Series::Bm52x, " ---", true, BLANK, false, &[Dc1, V1]),
            Series::Bm52x,
            "display text",
        );
    }

    /// The logging models' words (BM820s manual p.14-16), in the manual's
    /// spelling, and a session page read as Recall.
    #[test]
    fn logging_words() {
        let s = Series::Bm52x;
        for (text, word) in [
            ("LEFt", "LEFt"),
            ("5trt", "Strt"),
            ("PAU5", "PAUS"),
            ("Cont", "Cont"),
            ("5toP", "StoP"),
            ("t0.05", "interval"),
            ("t0.1 ", "interval"),
        ] {
            let m = quiet_as(&lcd8(s, text, BLANK, &[]), s);
            assert!(
                matches!(m.value, MeasuredValue::NoReading(w) if w == word),
                "{text}: {:?}",
                m.value
            );
            assert_eq!(m.mode, "Unknown(0x0000)");
        }
        let page = quiet_as(&lcd8(s, "P.028", BLANK, &[]), s);
        assert_eq!(page.mode, "Recall");
        assert!(matches!(page.value, MeasuredValue::NoReading("Recall")));
        // Only the logging models log.
        for text in ["LEFt", "P.028", "t0.05"] {
            reported_as(
                &lcd8(Series::Bm82x, text, BLANK, &[Dc1, V1]),
                Series::Bm82x,
                "display text",
            );
        }
    }

    /// R and C lit together are Recall on a logging model (BM820s manual
    /// p.16): logged data, whatever shows. On the other series no
    /// manual gives the state.
    #[test]
    fn recall_per_series() {
        for (main, sub, anns) in [
            ("223.7", "59.98", &[Record, Crest, Ac1, V1, Hz2][..]),
            ("0206", "   1", &[Record, Crest]),
            ("12?4", "HELP", &[Record, Crest, Max, Hold]),
        ] {
            let m = quiet_as(&lcd8(Series::Bm52x, main, sub, anns), Series::Bm52x);
            assert_eq!(m.mode, "Recall", "{main}");
            assert!(matches!(m.value, MeasuredValue::NoReading("Recall")));
            assert_eq!(m.flags, StatusFlags::default(), "{main}");
            assert!(m.aux_values.is_empty() && m.display_raw.is_none());
        }
        let live = reported_as(
            &lcd8(
                Series::Bm82x,
                "223.7",
                "59.98",
                &[Record, Crest, Ac1, V1, Hz2],
            ),
            Series::Bm82x,
            "REC and CREST annunciators",
        );
        assert_eq!(live.mode, "AC V");
        assert!(live.flags.record);
    }
}
