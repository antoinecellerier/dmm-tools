//! UT171 measurement frames, read the way UNI-T's UT171C software reads them.
//!
//! Sections are `docs/research/ut171/reverse-engineered-protocol.md`. Offsets
//! here are payload offsets: the wire byte number minus the four header and
//! length bytes, so wire byte 7 (the mode) is `payload[3]`.

use super::FAMILY;
use crate::error::Result;
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{check_len, unknown_mode, unknown_scale};
use std::borrow::Cow;

/// Response type of a measurement frame (§5.1).
pub(super) const RESPONSE_MEASUREMENT: u8 = 0x02;

/// The fixed part of a measurement payload: type, the two status bytes,
/// mode, range, the float, the display byte and the unit code (§5.1).
pub(super) const BASE_PAYLOAD: usize = 11;

/// The longest measurement payload: the fixed part, the aux block, the bar
/// float and the auto-save minutes (§3.4). Anything longer is some other
/// family's frame — the UT181A's, on this framing.
pub(super) const MAX_MEASUREMENT_PAYLOAD: usize = BASE_PAYLOAD + 6 + 4 + 2;

/// NCV, whose frame the software reads nothing past the mode of (§5.8).
pub(super) const MODE_NCV: u8 = 0x1A;

/// The UT171C's square-wave output, with a layout of its own (§5.9).
const MODE_SQUARE_WAVE: u8 = 0x1D;

/// Conductance, whose single range row the software never checks (§5.4).
const MODE_NS: u8 = 0x0E;

/// Type, two status bytes and the mode: all an NCV frame is known to carry.
const NCV_PAYLOAD_MIN: usize = 4;

/// The square-wave layout's last byte, the width's decimals, is `payload[17]`.
const SQUARE_WAVE_PAYLOAD: usize = 18;

// Status byte A, wire byte 5 (§5.3).
const AUX_PRESENT: u8 = 0x01;
const AUTO_SAVE: u8 = 0x02;
const LOW_BATTERY: u8 = 0x04;
const BAR_PRESENT: u8 = 0x08;
const REL: u8 = 0x10;
const PEAK: u8 = 0x40;
const HOLD: u8 = 0x80;

// Status byte B, wire byte 6 (§5.3).
const AUTO_RANGE: u8 = 0x01;
const HIGH_VOLTAGE: u8 = 0x02;
/// The software leaves a reading with this bit out of its log; what the
/// meter means by it is unknown. Documented, so silent, and read by nobody.
const NOT_LOGGED: u8 = 0x04;
const AGGREGATE_BIT5: u8 = 0x20;
const AGGREGATE_BIT6: u8 = 0x40;
const STATUS_B_DOCUMENTED: u8 =
    AUTO_RANGE | HIGH_VOLTAGE | NOT_LOGGED | AGGREGATE_BIT5 | AGGREGATE_BIT6;

/// Whether Peak is on: byte 5 bit 6 (§5.3). It says neither P-MAX nor P-MIN,
/// so it sets no flag; the Peak key's command follows it.
pub(super) fn peak_on(payload: &[u8]) -> bool {
    payload.get(1).is_some_and(|status| status & PEAK != 0)
}

/// The payload length status byte A announces (§3.4): the fixed part, plus
/// the aux block, the bar float and the auto-save minutes it flags.
fn announced_len(status_a: u8) -> usize {
    BASE_PAYLOAD
        + if status_a & AUX_PRESENT != 0 { 6 } else { 0 }
        + if status_a & BAR_PRESENT != 0 { 4 } else { 0 }
        + if status_a & AUTO_SAVE != 0 { 2 } else { 0 }
}

/// Whether `payload` has a measurement frame's shape: the measurement type
/// and the fixed part, or the few bytes an NCV frame is known to carry.
pub(super) fn is_measurement(payload: &[u8]) -> bool {
    payload.first() == Some(&RESPONSE_MEASUREMENT)
        && (payload.len() >= BASE_PAYLOAD
            || (payload.len() >= NCV_PAYLOAD_MIN && payload[3] == MODE_NCV))
}

/// Mode name of a mode byte §6 lists, `None` for any other.
pub(super) fn known_mode(byte: u8) -> Option<&'static str> {
    Some(match byte {
        0x01 => "LoZ V~",
        0x02 => "V DC",
        0x03 => "V AC",
        0x04 => "V AC+DC",
        0x05 => "mV DC",
        0x06 => "mV AC",
        0x07 => "mV AC+DC",
        0x08 => "°C",
        0x09 => "°F",
        0x0A => "Ω",
        0x0B => "Capacitance",
        0x0C => "Continuity",
        0x0D => "Diode",
        0x0E => "nS",
        0x0F => "Hz",
        0x11 => "µA DC",
        0x12 => "µA AC",
        0x13 => "µA AC+DC",
        0x14 => "mA DC",
        0x15 => "mA AC",
        0x16 => "mA AC+DC",
        0x17 => "A DC",
        0x18 => "A AC",
        0x19 => "A AC+DC",
        MODE_NCV => "NCV",
        0x1B => "600A DC",
        0x1C => "600A AC",
        MODE_SQUARE_WAVE => "Square wave",
        0x1E => "VFC",
        0x1F => "% 4-20mA",
        _ => return None,
    })
}

/// The mode's name, or `Unknown(0xNN)` reported.
fn mode_name(byte: u8) -> Cow<'static, str> {
    match known_mode(byte) {
        Some(name) => Cow::Borrowed(name),
        None => {
            report_unknown(FAMILY, "mode byte", format_args!("{byte:#04x}"));
            unknown_mode(byte)
        }
    }
}

/// The unit code table (§5.6): each code's unit, and the label a sub-value
/// in it takes. The software names the three couplings of V, mV, µA, mA and
/// A apart; the coupling here goes into the label, as the manual's Hz/% key
/// steps an aux display through AC, DC and frequency.
const UNITS: [(&str, &str); 32] = [
    ("V", "DC"),
    ("V", "AC"),
    ("V", "Aux"),
    ("mV", "DC"),
    ("mV", "AC"),
    ("mV", "Aux"),
    ("µA", "DC"),
    ("µA", "AC"),
    ("µA", "Aux"),
    ("mA", "DC"),
    ("mA", "AC"),
    ("mA", "Aux"),
    ("A", "DC"),
    ("A", "AC"),
    ("A", "Aux"),
    ("Ω", "Aux"),
    ("kΩ", "Aux"),
    ("MΩ", "Aux"),
    ("Hz", "Frequency"),
    ("kHz", "Frequency"),
    ("MHz", "Frequency"),
    ("%", "Duty"),
    ("nF", "Aux"),
    ("µF", "Aux"),
    ("mF", "Aux"),
    ("°C", "Aux"),
    ("°F", "Aux"),
    // DIOD: the screen shows V beside DIODE.
    ("V", "Aux"),
    // BEEP: the screen shows Ω beside the buzzer.
    ("Ω", "Aux"),
    ("nS", "Aux"),
    // NCV carries no unit.
    ("", "Aux"),
    ("ms", "Period"),
];

/// Unit and sub-value label of unit code `code`. A code past the table gives
/// no unit, reported: the unit is the reading's whole scale.
fn unit_code(code: u8, what: &'static str) -> (&'static str, &'static str) {
    match UNITS.get(usize::from(code)) {
        Some(&entry) => entry,
        None => (
            unknown_scale(FAMILY, what, format_args!("{code:#04x}")),
            "Aux",
        ),
    }
}

/// The range rows the software builds for a mode (§5.4), as its range picker
/// labels them. An empty label is a row whose text gives no range (°C and °F,
/// whose "0 to 600" contradicts their own axis, and continuity, whose row
/// names no unit). A mode with no rows ignores the range byte.
pub(super) fn range_rows(mode: u8) -> &'static [&'static str] {
    match mode {
        0x01..=0x04 => &["6V", "60V", "600V", "1000V"],
        0x05..=0x07 => &["600mV"],
        0x08 | 0x09 | 0x0C => &[""],
        0x0A => &["600Ω", "6kΩ", "60kΩ", "600kΩ", "6MΩ", "60MΩ"],
        0x0B => &[
            "6nF", "60nF", "600nF", "6µF", "60µF", "600µF", "6mF", "60mF",
        ],
        MODE_NS => &["60nS"],
        0x0F => &["60Hz", "600Hz", "6kHz", "60kHz", "600kHz", "6MHz", "60MHz"],
        0x11..=0x13 => &["600µA", "6000µA"],
        0x14..=0x16 => &["60mA", "600mA"],
        0x17..=0x19 => &["6A", "20A"],
        0x1B | 0x1C => &["60A", "600A"],
        0x1E => &["600V", "1000V"],
        _ => &[],
    }
}

/// The label of range byte `range` in `mode`; rows count from 1. A byte off
/// a mode's rows is reported: it no longer decides the scale, so the reading
/// keeps its unit.
fn range_label(mode: u8, range: u8) -> &'static str {
    // nS has one row and the software doesn't check the byte against it
    // (§5.4), as it ignores the byte in % 4-20mA.
    if mode == MODE_NS {
        return "60nS";
    }
    let rows = range_rows(mode);
    if rows.is_empty() {
        return "";
    }
    match usize::from(range).checked_sub(1).and_then(|i| rows.get(i)) {
        Some(label) => label,
        None => {
            report_unknown(
                FAMILY,
                "range byte",
                format_args!("mode {mode:#04x} range {range}"),
            );
            ""
        }
    }
}

/// The software's dashes for a display state of 3, by decimal count (§5.5).
fn dashes(decimals: u8) -> Option<&'static str> {
    Some(match decimals {
        1 => "----.-",
        2 => "---.--",
        3 => "--.---",
        4 => "-.----",
        _ => return None,
    })
}

/// A float's value and digits under the display byte that goes with it
/// (§5.5): the high nibble is the decimal count, the low nibble the state.
/// `main` picks the main display's words for states 4–7; a sub-value shows
/// dashes there, as the software does.
fn shown(float: f32, display: u8, main: bool, mode: u8) -> (MeasuredValue, Option<String>) {
    let what = if main {
        "display byte"
    } else {
        "aux display byte"
    };
    let decimals = display >> 4;
    let word = |word: &'static str| (MeasuredValue::NoReading(word), None);
    let dash_word = || match dashes(decimals) {
        Some(d) => word(d),
        None => {
            report_unknown(
                FAMILY,
                what,
                format_args!("{display:#04x} in mode {mode:#04x}"),
            );
            (MeasuredValue::UNRECOGNISED, None)
        }
    };
    match display & 0x0F {
        // OL and -OL: the meter's own overload.
        1 | 2 => (MeasuredValue::Overload, None),
        3 => dash_word(),
        4 if main => word("LEAD"),
        5 if main => word("DISC"),
        6 if main => word("Lo"),
        7 if main => word("Hi"),
        4..=7 => dash_word(),
        state => {
            let places = (1..=4).contains(&decimals).then_some(decimals);
            // One report per byte: a state past 7, or a decimal count the
            // software has no format for.
            if state != 0 || places.is_none() {
                report_unknown(
                    FAMILY,
                    what,
                    format_args!("{display:#04x} in mode {mode:#04x}"),
                );
            }
            number(float, places, mode)
        }
    }
}

/// A number shown with `places` decimals, or as the shortest decimal that
/// round-trips to the f32 — the wire value, with no precision it didn't
/// carry — when no count is known. A float that is no number is reported and
/// shows `?`: only the display byte's state says OL (D-5 in
/// `MeasuredValue::Overload`'s sense).
fn number(float: f32, places: Option<u8>, mode: u8) -> (MeasuredValue, Option<String>) {
    if !float.is_finite() {
        report_float(float, mode);
        return (MeasuredValue::UNRECOGNISED, None);
    }
    let digits = match places {
        Some(places) => format!("{float:.*}", usize::from(places)),
        None => format!("{float}"),
    };
    (MeasuredValue::Normal(f64::from(float)), Some(digits))
}

/// Report a float that is no number.
fn report_float(float: f32, mode: u8) {
    report_unknown(
        FAMILY,
        "float",
        format_args!("{:02X?} = {float} in mode {mode:#04x}", float.to_le_bytes()),
    );
}

/// The f32 at `payload[at..at + 4]`, little-endian.
fn f32_at(payload: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([
        payload[at],
        payload[at + 1],
        payload[at + 2],
        payload[at + 3],
    ])
}

/// The flags both status bytes carry (§5.3).
fn flags(status_a: u8, status_b: u8) -> StatusFlags {
    let (bit5, bit6) = (
        status_b & AGGREGATE_BIT5 != 0,
        status_b & AGGREGATE_BIT6 != 0,
    );
    if status_b & !STATUS_B_DOCUMENTED != 0 {
        report_unknown(
            FAMILY,
            "status bits",
            format_args!("byte 6 = {status_b:#04x}"),
        );
    }
    StatusFlags {
        hold: status_a & HOLD != 0,
        rel: status_a & REL != 0,
        record: status_a & AUTO_SAVE != 0,
        // No reader in UNI-T's software: `[UNVERIFIED]`, spec §5.3 and §10.
        low_battery: status_a & LOW_BATTERY != 0,
        auto_range: status_b & AUTO_RANGE != 0,
        hv_warning: status_b & HIGH_VOLTAGE != 0,
        max: !bit5 && bit6,
        avg: bit5 && !bit6,
        min: bit5 && bit6,
        ..Default::default()
    }
}

/// A sub-value from a float, its display state and its unit.
fn aux(
    label: &'static str,
    unit: &'static str,
    shown: (MeasuredValue, Option<String>),
) -> AuxValue {
    let (value, display_raw) = shown;
    AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        elapsed_secs: None,
    }
}

/// Parse a UT171 measurement payload (pure function).
pub(crate) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    check_len(FAMILY, payload, NCV_PAYLOAD_MIN)?;
    let (status_a, status_b, mode_byte) = (payload[1], payload[2], payload[3]);
    let mode = mode_name(mode_byte);
    let flags = flags(status_a, status_b);
    let base = Measurement {
        mode,
        mode_raw: u16::from(mode_byte),
        range_raw: payload.get(4).copied().unwrap_or(0),
        flags,
        ..Measurement::from_payload(payload)
    };
    match mode_byte {
        MODE_NCV => Ok(ncv(payload, base)),
        MODE_SQUARE_WAVE => Ok(square_wave(payload, base)),
        _ => general(payload, base),
    }
}

/// NCV (§5.8): the software reads nothing past the mode, and the meter
/// shows EF or dash segments, never a number. What the rest of the frame
/// holds is undocumented, so it is reported and the reading is `?`.
fn ncv(payload: &[u8], base: Measurement) -> Measurement {
    report_unknown(FAMILY, "NCV frame", format_args!("{:02X?}", &payload[4..]));
    Measurement {
        value: MeasuredValue::UNRECOGNISED,
        ..base
    }
}

/// The UT171C's square-wave output (§5.9): frequency, duty and width as
/// floats, the width's decimal count after them. The range byte is unread.
fn square_wave(payload: &[u8], base: Measurement) -> Measurement {
    if payload.len() < SQUARE_WAVE_PAYLOAD {
        report_unknown(
            FAMILY,
            "frame",
            format_args!("square wave, {} bytes", payload.len()),
        );
        return Measurement {
            value: MeasuredValue::UNRECOGNISED,
            ..base
        };
    }
    let one_place = |at| number(f32_at(payload, at), Some(1), MODE_SQUARE_WAVE);
    // The software formats the width with as many places as byte 21 says,
    // none included.
    let mut width_places = payload[17];
    if width_places > 4 {
        report_unknown(
            FAMILY,
            "square wave",
            format_args!("width decimals {width_places}"),
        );
        width_places = 4;
    }
    let width = number(f32_at(payload, 13), Some(width_places), MODE_SQUARE_WAVE);
    let (value, display_raw) = one_place(5);
    Measurement {
        value,
        display_raw,
        unit: Cow::Borrowed("Hz"),
        aux_values: vec![
            aux("Duty", "%", one_place(9)),
            aux("Pulse Width", "ms", width),
        ],
        ..base
    }
}

/// Every other mode (§5.1): the float, its display byte and its unit code,
/// then the blocks status byte A announces.
fn general(payload: &[u8], base: Measurement) -> Result<Measurement> {
    check_len(FAMILY, payload, BASE_PAYLOAD)?;
    let (status_a, mode_byte, range_byte) = (payload[1], payload[3], payload[4]);
    let len = payload.len();
    let announced = announced_len(status_a);
    if len != announced {
        report_unknown(
            FAMILY,
            "frame",
            format_args!("status {status_a:#04x}, {len} bytes"),
        );
    }
    let (value, display_raw) = shown(f32_at(payload, 5), payload[9], true, mode_byte);
    let (unit, _) = unit_code(payload[10], "unit code");
    let mut aux_values = Vec::new();
    // The aux block follows the fixed part when the flag says so and it fits.
    if status_a & AUX_PRESENT != 0 && len >= BASE_PAYLOAD + 6 {
        let (aux_unit, label) = unit_code(payload[16], "aux unit code");
        // REL puts its reference on the aux display (manual; spec §5.2).
        let label = if status_a & REL != 0 {
            "Reference"
        } else {
            label
        };
        let shown = shown(f32_at(payload, 11), payload[15], false, mode_byte);
        aux_values.push(aux(label, aux_unit, shown));
    }
    Ok(Measurement {
        value,
        display_raw,
        unit: Cow::Borrowed(unit),
        range_label: Cow::Borrowed(range_label(mode_byte, range_byte)),
        aux_values,
        ..base
    })
}

/// Every label the parser produces: each mode §6 lists, its range labels
/// and every sub-value label.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn all_labels(out: &mut crate::text::Labels) {
    use crate::text::TextKind;
    let by = super::devices::UT171.id;
    for mode in 0..=u8::MAX {
        let Some(name) = known_mode(mode) else {
            continue;
        };
        out.add(TextKind::Mode, name, by);
        out.add(TextKind::Range, "", by);
        // The range select's first choice.
        out.add(TextKind::Range, crate::protocol::AUTO_RANGE_LABEL, by);
        for &label in range_rows(mode) {
            out.add(TextKind::Range, label, by);
        }
    }
    for (_, label) in UNITS {
        out.add(TextKind::Aux, label, by);
    }
    for label in ["Aux", "Duty", "Pulse Width", "Reference"] {
        out.add(TextKind::Aux, label, by);
    }
}

/// Every unit a reading of this family can carry, for
/// [`crate::protocol::all_units`].
#[cfg(test)]
pub(crate) fn all_units(out: &mut crate::protocol::Units) {
    for (unit, _) in UNITS {
        out.insert(unit);
    }
    for unit in ["", "Hz", "%", "ms"] {
        out.insert(unit);
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::test_support::snapshot;

    /// A payload with no aux, bar or auto-save block: the fixed 11 bytes.
    pub(in crate::protocol::ut171) fn payload(
        mode: u8,
        range: u8,
        value: f32,
        display: u8,
        unit: u8,
    ) -> Vec<u8> {
        let mut p = vec![RESPONSE_MEASUREMENT, 0x00, AUTO_RANGE, mode, range];
        p.extend_from_slice(&value.to_le_bytes());
        p.extend_from_slice(&[display, unit]);
        p
    }

    /// `payload` with an aux block: status A bit 0 and six more bytes.
    pub(in crate::protocol::ut171) fn with_aux(
        mut p: Vec<u8>,
        value: f32,
        display: u8,
        unit: u8,
    ) -> Vec<u8> {
        p[1] |= AUX_PRESENT;
        p.extend_from_slice(&value.to_le_bytes());
        p.extend_from_slice(&[display, unit]);
        p
    }

    /// `p` with the bar float: status A bit 3 and four more bytes.
    fn with_bar(mut p: Vec<u8>, value: f32) -> Vec<u8> {
        p[1] |= BAR_PRESENT;
        p.extend_from_slice(&value.to_le_bytes());
        p
    }

    /// `p` with the auto-save minutes: status A bit 1 and two more bytes.
    fn with_auto_save(mut p: Vec<u8>, minutes: u16) -> Vec<u8> {
        p[1] |= AUTO_SAVE;
        p.extend_from_slice(&minutes.to_be_bytes());
        p
    }

    /// A square-wave payload: frequency, duty, width and the width's places.
    fn square(freq: f32, duty: f32, width: f32, places: u8) -> Vec<u8> {
        let mut p = vec![RESPONSE_MEASUREMENT, 0x00, 0x00, MODE_SQUARE_WAVE, 0x00];
        for f in [freq, duty, width] {
            p.extend_from_slice(&f.to_le_bytes());
        }
        p.push(places);
        p
    }

    fn parse_reporting(payload: &[u8]) -> (Result<Measurement>, Vec<String>) {
        capture_reports(|| parse_measurement(payload))
    }

    fn quiet(payload: &[u8]) -> Measurement {
        let (m, reports) = parse_reporting(payload);
        assert!(reports.is_empty(), "{payload:02X?}: {reports:?}");
        m.unwrap()
    }

    /// One payload whose every field is pinned: auto-ranging V DC with four
    /// decimals.
    #[test]
    fn parse_vdc() {
        let m = quiet(&payload(0x02, 1, 1.2345, 0x40, 0x00));
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC
mode_raw=0x02
range_raw=0x01
value=Normal(1.2345000505447388)
unit=V
range_label=6V
display_raw=Some("1.2345")
flags=auto_range
aux=0
raw_payload=11"#
        );
    }

    /// The unit is byte 14's, whatever the range: no scaling.
    #[test]
    fn the_unit_comes_from_the_unit_code() {
        for (mode, range, code, unit, label) in [
            (0x0A, 1, 0x0F, "Ω", "600Ω"),
            (0x0A, 3, 0x10, "kΩ", "60kΩ"),
            (0x0A, 6, 0x11, "MΩ", "60MΩ"),
            (0x0B, 2, 0x16, "nF", "60nF"),
            (0x0B, 5, 0x17, "µF", "60µF"),
            (0x0B, 8, 0x18, "mF", "60mF"),
            (0x0F, 4, 0x13, "kHz", "60kHz"),
            (0x0F, 6, 0x14, "MHz", "6MHz"),
            (0x0D, 0, 0x1B, "V", ""),
            (0x0C, 1, 0x1C, "Ω", ""),
            (0x0E, 1, 0x1D, "nS", "60nS"),
            (0x08, 1, 0x19, "°C", ""),
            (0x09, 1, 0x1A, "°F", ""),
            (0x1F, 7, 0x15, "%", ""),
        ] {
            let m = quiet(&payload(mode, range, 12.34, 0x20, code));
            assert_eq!(m.unit, unit, "mode {mode:#04x}");
            assert_eq!(m.range_label, label, "mode {mode:#04x}");
            assert_eq!(m.display_raw.as_deref(), Some("12.34"));
        }
    }

    /// The unit table, written out from spec §5.6, not from [`UNITS`].
    #[test]
    fn every_unit_code() {
        let table = [
            "V DC",
            "V AC",
            "V Aux",
            "mV DC",
            "mV AC",
            "mV Aux",
            "µA DC",
            "µA AC",
            "µA Aux",
            "mA DC",
            "mA AC",
            "mA Aux",
            "A DC",
            "A AC",
            "A Aux",
            "Ω Aux",
            "kΩ Aux",
            "MΩ Aux",
            "Hz Frequency",
            "kHz Frequency",
            "MHz Frequency",
            "% Duty",
            "nF Aux",
            "µF Aux",
            "mF Aux",
            "°C Aux",
            "°F Aux",
            "V Aux",
            "Ω Aux",
            "nS Aux",
            " Aux",
            "ms Period",
        ];
        for (code, expected) in (0u8..).zip(table) {
            let m = quiet(&with_aux(
                payload(0x02, 1, 1.0, 0x10, code),
                2.0,
                0x10,
                code,
            ));
            let aux = &m.aux_values[0];
            assert_eq!(format!("{} {}", m.unit, aux.label), expected, "{code:#04x}");
            assert_eq!(aux.unit, m.unit, "{code:#04x}");
        }
    }

    /// The mode map, written out from spec §6, not from [`known_mode`].
    #[test]
    fn every_mode_byte() {
        let table = [
            (0x01, "LoZ V~"),
            (0x02, "V DC"),
            (0x03, "V AC"),
            (0x04, "V AC+DC"),
            (0x05, "mV DC"),
            (0x06, "mV AC"),
            (0x07, "mV AC+DC"),
            (0x08, "°C"),
            (0x09, "°F"),
            (0x0A, "Ω"),
            (0x0B, "Capacitance"),
            (0x0C, "Continuity"),
            (0x0D, "Diode"),
            (0x0E, "nS"),
            (0x0F, "Hz"),
            (0x11, "µA DC"),
            (0x12, "µA AC"),
            (0x13, "µA AC+DC"),
            (0x14, "mA DC"),
            (0x15, "mA AC"),
            (0x16, "mA AC+DC"),
            (0x17, "A DC"),
            (0x18, "A AC"),
            (0x19, "A AC+DC"),
            (0x1A, "NCV"),
            (0x1B, "600A DC"),
            (0x1C, "600A AC"),
            (0x1D, "Square wave"),
            (0x1E, "VFC"),
            (0x1F, "% 4-20mA"),
        ];
        for (byte, name) in table {
            assert_eq!(known_mode(byte), Some(name), "{byte:#04x}");
        }
        let listed = (0..=u8::MAX).filter(|&b| known_mode(b).is_some()).count();
        assert_eq!(listed, table.len());
    }

    #[test]
    fn a_unit_code_past_the_table_gives_no_unit_and_is_reported() {
        let (m, reports) = parse_reporting(&payload(0x02, 1, 1.0, 0x10, 0x20));
        assert_eq!(m.unwrap().unit, "");
        assert_eq!(reports, ["ut171: unrecognised unit code: 0x20"]);
    }

    #[test]
    fn the_display_byte_gives_the_decimals() {
        for (display, digits) in [
            (0x10, "1.2"),
            (0x20, "1.23"),
            (0x30, "1.235"),
            (0x40, "1.2346"),
        ] {
            let m = quiet(&payload(0x02, 1, 1.23456, display, 0x00));
            assert_eq!(m.display_raw.as_deref(), Some(digits));
            crate::protocol::assert_digits_spell_value(&m);
        }
    }

    #[test]
    fn decimals_outside_one_to_four_are_reported_and_keep_the_wire_float() {
        for display in [0x00, 0x50, 0xF0] {
            let (m, reports) = parse_reporting(&payload(0x02, 1, 1.5, display, 0x00));
            assert_eq!(m.unwrap().display_raw.as_deref(), Some("1.5"));
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised display byte: {display:#04x} in mode 0x02"
                )]
            );
        }
    }

    /// The display states (§5.5). The float is ignored outside state 0.
    #[test]
    fn the_display_states() {
        for (state, value) in [
            (1, MeasuredValue::Overload),
            (2, MeasuredValue::Overload),
            (3, MeasuredValue::NoReading("--.---")),
            (4, MeasuredValue::NoReading("LEAD")),
            (5, MeasuredValue::NoReading("DISC")),
            (6, MeasuredValue::NoReading("Lo")),
            (7, MeasuredValue::NoReading("Hi")),
        ] {
            let m = quiet(&payload(0x0A, 1, 0.0, 0x30 | state, 0x0F));
            assert_eq!(
                format!("{:?}", m.value),
                format!("{value:?}"),
                "state {state}"
            );
            assert!(m.display_raw.is_none());
            assert_eq!(m.unit, "Ω");
        }
        for (places, d) in [(1, "----.-"), (2, "---.--"), (4, "-.----")] {
            let m = quiet(&payload(0x02, 1, 0.0, (places << 4) | 3, 0x00));
            assert_eq!(
                format!("{:?}", m.value),
                format!("{:?}", MeasuredValue::NoReading(d))
            );
        }
    }

    #[test]
    fn dashes_with_no_decimal_count_are_reported_unrecognised() {
        let (m, reports) = parse_reporting(&payload(0x02, 1, 0.0, 0x03, 0x00));
        assert_eq!(
            format!("{:?}", m.unwrap().value),
            format!("{:?}", MeasuredValue::UNRECOGNISED)
        );
        assert_eq!(
            reports,
            ["ut171: unrecognised display byte: 0x03 in mode 0x02"]
        );
    }

    #[test]
    fn a_state_past_seven_is_reported_and_the_number_read() {
        let (m, reports) = parse_reporting(&payload(0x02, 1, 2.5, 0x18, 0x00));
        assert_eq!(m.unwrap().display_raw.as_deref(), Some("2.5"));
        assert_eq!(
            reports,
            ["ut171: unrecognised display byte: 0x18 in mode 0x02"]
        );
    }

    /// D-5: a float that is no number is no overload unless the display byte
    /// says OL.
    #[test]
    fn a_non_finite_float_is_reported_and_unrecognised() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let (m, reports) = parse_reporting(&payload(0x0A, 1, value, 0x20, 0x0F));
            assert_eq!(
                format!("{:?}", m.unwrap().value),
                format!("{:?}", MeasuredValue::UNRECOGNISED)
            );
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised float: {:02X?} = {value} in mode 0x0a",
                    value.to_le_bytes()
                )]
            );
        }
        let m = quiet(&payload(0x0A, 1, f32::NAN, 0x21, 0x0F));
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::Overload)
        );
    }

    #[test]
    fn the_status_flags() {
        let flagged = |a: u8, b: u8| {
            let mut p = payload(0x02, 1, 1.0, 0x10, 0x00);
            p[1] = a;
            p[2] = b;
            if a & AUTO_SAVE != 0 {
                p.extend_from_slice(&[0, 5]);
            }
            quiet(&p).flags
        };
        assert!(flagged(HOLD, 0).hold);
        assert!(flagged(REL, 0).rel);
        assert!(flagged(AUTO_SAVE, 0).record);
        assert!(flagged(LOW_BATTERY, 0).low_battery);
        assert!(flagged(0, AUTO_RANGE).auto_range);
        assert!(!flagged(0, 0).auto_range);
        assert!(flagged(0, HIGH_VOLTAGE).hv_warning);
        let f = flagged(0, AGGREGATE_BIT6);
        assert!(f.max && !f.avg && !f.min);
        let f = flagged(0, AGGREGATE_BIT5);
        assert!(!f.max && f.avg && !f.min);
        let f = flagged(0, AGGREGATE_BIT5 | AGGREGATE_BIT6);
        assert!(!f.max && !f.avg && f.min);
        // Documented bits that set no flag: MAX/MIN active, Peak (which says
        // neither P-MAX nor P-MIN), byte 6 bit 2.
        assert_eq!(flagged(0x20 | 0x40, 0x04), StatusFlags::default());
    }

    #[test]
    fn undocumented_status_b_bits_are_reported() {
        for bit in [0x08u8, 0x10, 0x80] {
            let mut p = payload(0x02, 1, 1.0, 0x10, 0x00);
            p[2] |= bit;
            let (m, reports) = parse_reporting(&p);
            assert!(m.unwrap().flags.auto_range);
            assert_eq!(
                reports,
                [format!(
                    "ut171: unrecognised status bits: byte 6 = {:#04x}",
                    AUTO_RANGE | bit
                )]
            );
        }
    }

    #[test]
    fn every_announced_layout_parses_quietly() {
        let base = || payload(0x03, 2, 230.08, 0x20, 0x01);
        let freq = |p| with_aux(p, 0.04998, 0x30, 0x13);
        for (p, aux) in [
            (base(), 0),
            (freq(base()), 1),
            (with_bar(base(), 230.1), 0),
            (with_bar(freq(base()), 230.1), 1),
            (with_auto_save(with_bar(freq(base()), 230.1), 61), 1),
            (with_auto_save(base(), 1), 0),
        ] {
            let m = quiet(&p);
            assert_eq!(m.display_raw.as_deref(), Some("230.08"));
            assert_eq!(m.aux_values.len(), aux, "{p:02X?}");
        }
        assert_eq!(
            MAX_MEASUREMENT_PAYLOAD,
            with_auto_save(with_bar(freq(base()), 0.0), 0).len()
        );
    }

    #[test]
    fn the_aux_block() {
        let m = quiet(&with_aux(
            payload(0x03, 2, 230.08, 0x20, 0x01),
            0.04998,
            0x30,
            0x13,
        ));
        assert_eq!(
            snapshot(&m),
            r#"mode=V AC
mode_raw=0x03
range_raw=0x02
value=Normal(230.0800018310547)
unit=V
range_label=60V
display_raw=Some("230.08")
flags=auto_range
aux=1
aux1=Frequency value=Normal(0.04997999966144562) unit=kHz display_raw=Some("0.050") elapsed_secs=None
raw_payload=17"#
        );
        for (code, label, unit) in [
            (0x15, "Duty", "%"),
            (0x1F, "Period", "ms"),
            (0x00, "DC", "V"),
            (0x01, "AC", "V"),
            (0x02, "Aux", "V"),
        ] {
            let m = quiet(&with_aux(
                payload(0x04, 2, 1.0, 0x10, 0x02),
                2.5,
                0x10,
                code,
            ));
            assert_eq!(m.aux_values[0].label, label);
            assert_eq!(m.aux_values[0].unit, unit);
        }
    }

    /// The software shows dashes for an aux state of 3 to 7.
    #[test]
    fn aux_states() {
        let aux_value = |display| {
            quiet(&with_aux(
                payload(0x03, 1, 1.0, 0x10, 0x01),
                9.9,
                display,
                0x12,
            ))
            .aux_values[0]
                .value
                .clone()
        };
        assert_eq!(
            format!("{:?}", aux_value(0x11)),
            format!("{:?}", MeasuredValue::Overload)
        );
        assert_eq!(
            format!("{:?}", aux_value(0x22)),
            format!("{:?}", MeasuredValue::Overload)
        );
        for state in 3..=7 {
            assert_eq!(
                format!("{:?}", aux_value(0x20 | state)),
                format!("{:?}", MeasuredValue::NoReading("---.--"))
            );
        }
    }

    #[test]
    fn a_length_that_disagrees_with_status_a_is_reported() {
        // The aux flag on a frame too short for the block: the aux is not read.
        let mut short = payload(0x03, 1, 230.0, 0x10, 0x01);
        short[1] |= AUX_PRESENT;
        let (m, reports) = parse_reporting(&short);
        assert!(m.unwrap().aux_values.is_empty());
        assert_eq!(
            reports,
            ["ut171: unrecognised frame: status 0x01, 11 bytes"]
        );

        // A block nobody announced.
        let mut long = payload(0x03, 1, 230.0, 0x10, 0x01);
        long.extend_from_slice(&[0; 4]);
        let (m, reports) = parse_reporting(&long);
        assert!(m.unwrap().aux_values.is_empty());
        assert_eq!(
            reports,
            ["ut171: unrecognised frame: status 0x00, 15 bytes"]
        );
    }

    #[test]
    fn a_payload_shorter_than_the_fixed_part_is_an_error() {
        assert!(parse_measurement(&payload(0x02, 1, 1.0, 0x10, 0x00)[..10]).is_err());
        assert!(parse_measurement(&[0x02, 0, 0]).is_err());
    }

    /// 0x10 and 0x24 came from code that turned out not to be the meter's.
    #[test]
    fn an_unknown_mode_byte_is_reported() {
        for mode in [0x00u8, 0x10, 0x20, 0x24, 0x30] {
            let (m, reports) = parse_reporting(&payload(mode, 1, 1.0, 0x10, 0x00));
            assert_eq!(m.unwrap().mode, format!("Unknown({mode:#04x})"));
            assert_eq!(
                reports,
                [format!("ut171: unrecognised mode byte: {mode:#04x}")]
            );
        }
    }

    #[test]
    fn a_range_off_the_mode_s_rows_is_reported() {
        for (mode, rows) in [
            (0x01u8, 4u8),
            (0x05, 1),
            (0x08, 1),
            (0x0A, 6),
            (0x0B, 8),
            (0x0F, 7),
            (0x17, 2),
            (0x1E, 2),
        ] {
            for range in [0, rows + 1] {
                let (m, reports) = parse_reporting(&payload(mode, range, 1.0, 0x10, 0x00));
                let m = m.unwrap();
                assert_eq!(m.range_label, "");
                assert_eq!(m.unit, "V", "the unit stays");
                assert_eq!(
                    reports,
                    [format!(
                        "ut171: unrecognised range byte: mode {mode:#04x} range {range}"
                    )]
                );
            }
        }
        // Modes with no rows ignore the byte.
        for mode in [0x0D, 0x1F] {
            quiet(&payload(mode, 9, 1.0, 0x10, 0x00));
        }
    }

    #[test]
    fn ncv_reads_unrecognised_and_reports_its_bytes() {
        let mut short = vec![RESPONSE_MEASUREMENT, 0x00, 0x00, MODE_NCV];
        let (m, reports) = parse_reporting(&short);
        let m = m.unwrap();
        assert_eq!(m.mode, "NCV");
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::UNRECOGNISED)
        );
        assert_eq!(reports, ["ut171: unrecognised NCV frame: []"]);
        short.extend_from_slice(&[0x01, 0x02]);
        let (_, reports) = parse_reporting(&short);
        assert_eq!(reports, ["ut171: unrecognised NCV frame: [01, 02]"]);
    }

    #[test]
    fn the_square_wave() {
        let m = quiet(&square(1000.0, 50.0, 0.5, 2));
        assert_eq!(
            snapshot(&m),
            r#"mode=Square wave
mode_raw=0x1d
range_raw=0x00
value=Normal(1000.0)
unit=Hz
range_label=
display_raw=Some("1000.0")
flags=
aux=2
aux1=Duty value=Normal(50.0) unit=% display_raw=Some("50.0") elapsed_secs=None
aux2=Pulse Width value=Normal(0.5) unit=ms display_raw=Some("0.50") elapsed_secs=None
raw_payload=18"#
        );
    }

    #[test]
    fn a_short_square_wave_frame_is_reported() {
        let (m, reports) = parse_reporting(&square(1000.0, 50.0, 0.5, 2)[..17]);
        let m = m.unwrap();
        assert_eq!(
            format!("{:?}", m.value),
            format!("{:?}", MeasuredValue::UNRECOGNISED)
        );
        assert!(m.aux_values.is_empty());
        assert_eq!(
            reports,
            ["ut171: unrecognised frame: square wave, 17 bytes"]
        );
    }

    #[test]
    fn square_wave_width_decimals_past_four_are_reported_and_capped() {
        let (m, reports) = parse_reporting(&square(1000.0, 50.0, 0.123456, 9));
        assert_eq!(
            m.unwrap().aux_values[1].display_raw.as_deref(),
            Some("0.1235")
        );
        assert_eq!(
            reports,
            ["ut171: unrecognised square wave: width decimals 9"]
        );
    }

    #[test]
    fn the_shape_test() {
        assert!(is_measurement(&payload(0x02, 1, 1.0, 0x10, 0x00)));
        assert!(is_measurement(&[RESPONSE_MEASUREMENT, 0, 0, MODE_NCV]));
        assert!(!is_measurement(&[RESPONSE_MEASUREMENT, 0, 0, 0x02]));
        assert!(!is_measurement(&payload(0x02, 1, 1.0, 0x10, 0x00)[..10]));
        assert!(!is_measurement(&[0x01, b'O', b'K']));
    }

    #[test]
    fn a_square_wave_width_with_no_decimals() {
        let m = quiet(&square(1000.0, 50.0, 12.5, 0));
        assert_eq!(m.aux_values[1].display_raw.as_deref(), Some("12"));
    }

    /// A state past 7 with no decimal count is still one report.
    #[test]
    fn one_display_byte_is_one_report() {
        let (m, reports) = parse_reporting(&payload(0x02, 1, 2.5, 0x08, 0x00));
        assert_eq!(m.unwrap().display_raw.as_deref(), Some("2.5"));
        assert_eq!(
            reports,
            ["ut171: unrecognised display byte: 0x08 in mode 0x02"]
        );
    }

    /// REL's reference is on the aux display, whatever its unit code.
    #[test]
    fn rel_labels_the_aux_its_reference() {
        let mut p = with_aux(payload(0x02, 2, 0.5, 0x40, 0x00), 1.2345, 0x40, 0x00);
        p[1] |= REL;
        let m = quiet(&p);
        assert!(m.flags.rel);
        assert_eq!(m.aux_values[0].label, "Reference");
        assert_eq!(m.aux_values[0].unit, "V");
    }

    /// The software doesn't check nS's range byte against its single row.
    #[test]
    fn ns_ignores_its_range_byte() {
        for range in [0, 1, 2, 9] {
            assert_eq!(
                quiet(&payload(MODE_NS, range, 1.0, 0x20, 0x1D)).range_label,
                "60nS"
            );
        }
    }

    /// A negative reading keeps its sign in the digits.
    #[test]
    fn a_negative_reading() {
        let m = quiet(&payload(0x02, 1, -1.6093, 0x40, 0x00));
        assert_eq!(m.display_raw.as_deref(), Some("-1.6093"));
    }
}
