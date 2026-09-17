//! UT181A measurement payloads: mode words, range labels and the normal,
//! REL, MIN/MAX, Peak and COMP layouts, decoded into a `Measurement`.

use super::mode;
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::unrecognised::report_unknown;
use crate::protocol::{check_len, unknown_mode16};
use log::debug;
use std::borrow::Cow;

/// Response type of a live measurement frame, the only one that carries a
/// reading: `0x01` is the OK/ER reply to a command
/// (`docs/research/ut181/reverse-engineered-protocol.md` §5).
pub(super) const RESPONSE_MEASUREMENT: u8 = 0x02;

/// A measurement payload this long can only be a UT181A: its normal format
/// passes 31 bytes as soon as it carries an aux value or a bargraph (§5.3),
/// while the UT171's longest measurement response is 21 payload bytes
/// (`docs/research/ut171/reverse-engineered-protocol.md` §3.4). Below it the
/// two families are told apart by which trigger elicited the frame.
pub(super) const EXCLUSIVE_PAYLOAD_MIN: usize = 31;

/// Decode a UT181A mode word (uint16 LE) into a human-readable string.
///
/// Nibble encoding: N3 N2 N1 N0
/// N3 = measurement family, N2 = sub-function, N1 = variant, N0 = 1=std/2=REL
pub(super) fn decode_mode_word(mode: u16) -> Cow<'static, str> {
    let n3 = (mode >> 12) & 0xF;
    let n2 = (mode >> 8) & 0xF;
    let n1 = (mode >> 4) & 0xF;
    let n0 = mode & 0xF;

    // Two mode words break the "N0=2 means REL" rule (sigrok
    // MODE_CONT_OPEN / MODE_DIODE_ALARM; antage Beeper_Open /
    // Diode_Alarm agree):
    match mode {
        0x5212 => return Cow::Borrowed("Continuity (open)"),
        0x6112 => return Cow::Borrowed("Diode Alarm"),
        _ => {}
    }

    // Temperature families use N1 as the display arrangement, not the
    // generic variant nibble (sigrok: T1(T2), T2(T1), T1-T2, T2-T1).
    if n3 == 0x4 && (n2 == 0x2 || n2 == 0x3) {
        let family = if n2 == 0x2 { "°C" } else { "°F" };
        let arrangement = match n1 {
            0x2 => " T2",
            0x3 => " T1-T2",
            0x4 => " T2-T1",
            _ => "",
        };
        let rel = if n0 == 0x2 { " REL" } else { "" };
        return if arrangement.is_empty() && rel.is_empty() {
            Cow::Borrowed(family)
        } else {
            Cow::Owned(format!("{family}{arrangement}{rel}"))
        };
    }

    let family = match n3 {
        0x1 => "V AC",
        0x2 => "mV AC",
        0x3 => "V DC",
        0x4 => match n2 {
            0x1 => "mV DC",
            0x2 => "°C",
            0x3 => "°F",
            _ => return unknown_mode16(mode),
        },
        0x5 => match n2 {
            0x1 => "Ω",
            0x2 => "Continuity",
            0x3 => "nS",
            _ => return unknown_mode16(mode),
        },
        0x6 => match n2 {
            0x1 => "Diode",
            0x2 => "Capacitance",
            _ => return unknown_mode16(mode),
        },
        0x7 => match n2 {
            0x1 => "Hz",
            0x2 => "Duty %",
            0x3 => "Pulse Width",
            _ => return unknown_mode16(mode),
        },
        0x8 => match n2 {
            0x1 => "µA DC",
            0x2 => "µA AC",
            _ => return unknown_mode16(mode),
        },
        0x9 => match n2 {
            0x1 => "mA DC",
            0x2 => "mA AC",
            _ => return unknown_mode16(mode),
        },
        0xA => match n2 {
            0x1 => "A DC",
            0x2 => "A AC",
            _ => return unknown_mode16(mode),
        },
        _ => return unknown_mode16(mode),
    };

    let variant = match n1 {
        0x1 => "",
        0x2 => match (n3, n2) {
            // V AC / mV AC: frequency display
            (0x1 | 0x2, _) => " Hz",
            // V DC: AC+DC
            (0x3, _) => " AC+DC",
            // mV DC: 0x4121 = mV DC Peak per sigrok/antage (sigrok notes
            // the code might be 0x4131 — hardware check pending)
            (0x4, 0x1) => " Peak",
            // Currents: n1=2 on the DC sub-function (n2=1) is AC+DC
            // (sigrok MODE_uA/mA/A_DC_ACDC = 0x8121/0x9121/0xA121);
            // Hz applies only to the AC sub-function (n2=2)
            (0x8..=0xA, 0x1) => " AC+DC",
            (0x8..=0xA, 0x2) => " Hz",
            _ => "",
        },
        0x3 => " Peak",
        0x4 => match n3 {
            0x1 => " LPF",
            0x2 => " AC+DC",
            _ => "",
        },
        0x5 => " dBV",
        0x6 => " dBm",
        _ => "",
    };

    let rel = if n0 == 0x2 { " REL" } else { "" };

    // When no variant or rel suffix, return the static family string directly
    if variant.is_empty() && rel.is_empty() {
        Cow::Borrowed(family)
    } else {
        Cow::Owned(format!("{family}{variant}{rel}"))
    }
}

/// Parse a UT181A unit string from 8 bytes (null-terminated).
///
/// The meter sends Latin-1, not UTF-8 (spec §8: 0xB0 = degree symbol),
/// so decode byte-by-byte — `from_utf8_lossy` would mangle °C/°F into
/// replacement characters.
fn parse_unit_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect()
}

/// Labels for the two aux slots of a normal-format measurement.
///
/// The meter sends each sub-value's own unit but never says what the value
/// *is*, so the label has to come from the mode word. Two arrangements are
/// pinned by a real UT181A capture (@diego351, issue #5, 2026-09-02):
/// `0x4211` puts one thermocouple on the main display and the other in aux1,
/// and `0x1121` puts the frequency in aux1 with its period in aux2
/// (1/50.00875 Hz = 19.9965 ms, exactly the aux2 reading in that frame). The
/// remaining modes follow the same nibble rule with no frame behind them; any
/// slot whose meaning is unknown keeps its positional label.
fn aux_labels(mode: u16) -> (&'static str, &'static str) {
    let n3 = (mode >> 12) & 0xF;
    let n2 = (mode >> 8) & 0xF;
    let n1 = (mode >> 4) & 0xF;

    // Temperature: n1 selects the display arrangement, so the aux slot holds
    // the other probe. The differential arrangements (n1 = 3/4) put a
    // difference on the main display and no source says which probe lands in
    // the aux slot — those stay positional.
    if n3 == 0x4 && (n2 == 0x2 || n2 == 0x3) {
        return match n1 {
            0x1 => ("T2", POSITIONAL_AUX.1),
            0x2 => ("T1", POSITIONAL_AUX.1),
            _ => POSITIONAL_AUX,
        };
    }

    // The same n1 = 2 codes `decode_mode_word` suffixes with " Hz": the
    // frequency display, with the period alongside it.
    if n1 == 0x2 && matches!((n3, n2), (0x1 | 0x2, _) | (0x8..=0xA, 0x2)) {
        return ("Frequency", "Period");
    }

    POSITIONAL_AUX
}

/// The labels [`aux_labels`] gives a slot nothing describes.
const POSITIONAL_AUX: (&str, &str) = ("Aux1", "Aux2");

/// Report a sub-value that went into a slot [`aux_labels`] has no name for.
///
/// Research spec §6 says what the aux slots hold only for the T1/T2
/// temperature arrangements and the Hz variants, and those are what the
/// captures carry; a sub-value anywhere else is undocumented.
fn report_positional_aux(label: &str, mode_word: u16) {
    if label == POSITIONAL_AUX.0 || label == POSITIONAL_AUX.1 {
        report_unknown(
            "ut181a",
            "aux value",
            format_args!("{label} in mode {mode_word:#06x}"),
        );
    }
}

/// Report a float and precision byte outside research spec §5.2. The value
/// is read the same either way.
fn report_unrecognised_value(float: f32, precision: u8) {
    // Bits 0 and 1 are +OL and -OL and bits 4-7 the decimals: bits 2-3 are
    // undefined, and nothing says both overloads can be set at once.
    if precision & 0x0C != 0 || precision & 0x03 == 0x03 {
        report_unknown("ut181a", "precision byte", format_args!("{precision:#04x}"));
    }
    // An overload is signalled by those bits; NaN and infinity are never
    // mentioned.
    if !float.is_finite() && precision & 0x03 == 0 {
        report_unknown(
            "ut181a",
            "float",
            format_args!(
                "{float} ({:#010x}) without an overload bit, shown as OL",
                float.to_bits()
            ),
        );
    }
}

/// Look up range label from mode word and range byte.
///
/// Uses the table from protocol spec Section 7. The family nibble (N3) and
/// sub-function nibble (N2) together determine which range table applies.
/// Temperature and A current have fixed ranges (no label).
pub(super) fn lookup_range_label(mode_word: u16, range: u8) -> &'static str {
    if range == 0 {
        return "Auto";
    }
    let family = (mode_word >> 12) & 0xF;
    let sub = (mode_word >> 8) & 0xF;

    match (family, sub, range) {
        // mV DC (0x4, sub 0x1) and mV AC (0x2, sub 0x1)
        (0x2 | 0x4, 0x1, 1) => "60mV",
        (0x2 | 0x4, 0x1, 2) => "600mV",

        // V AC (0x1) and V DC (0x3)
        (0x1 | 0x3, _, 1) => "6V",
        (0x1 | 0x3, _, 2) => "60V",
        (0x1 | 0x3, _, 3) => "600V",
        (0x1 | 0x3, _, 4) => "1000V",

        // µA DC (0x8, sub 0x1) and µA AC (0x8, sub 0x2)
        (0x8, _, 1) => "600\u{00B5}A",
        (0x8, _, 2) => "6000\u{00B5}A",

        // mA DC (0x9, sub 0x1) and mA AC (0x9, sub 0x2)
        (0x9, _, 1) => "60mA",
        (0x9, _, 2) => "600mA",

        // A DC/AC (0xA): fixed 10A range, no label needed
        (0xA, _, _) => "",

        // Resistance (0x5, sub 0x1)
        (0x5, 0x1, 1) => "600\u{2126}",
        (0x5, 0x1, 2) => "6k\u{2126}",
        (0x5, 0x1, 3) => "60k\u{2126}",
        (0x5, 0x1, 4) => "600k\u{2126}",
        (0x5, 0x1, 5) => "6M\u{2126}",
        (0x5, 0x1, 6) => "60M\u{2126}",

        // Continuity (0x5, sub 0x2), Conductance (0x5, sub 0x3): fixed range
        (0x5, _, _) => "",

        // Diode (0x6, sub 0x1): fixed range
        (0x6, 0x1, _) => "",

        // Capacitance (0x6, sub 0x2)
        (0x6, 0x2, 1) => "6nF",
        (0x6, 0x2, 2) => "60nF",
        (0x6, 0x2, 3) => "600nF",
        (0x6, 0x2, 4) => "6\u{00B5}F",
        (0x6, 0x2, 5) => "60\u{00B5}F",
        (0x6, 0x2, 6) => "600\u{00B5}F",
        (0x6, 0x2, 7) => "6mF",
        (0x6, 0x2, 8) => "60mF",

        // Frequency (0x7, sub 0x1)
        (0x7, 0x1, 1) => "60Hz",
        (0x7, 0x1, 2) => "600Hz",
        (0x7, 0x1, 3) => "6kHz",
        (0x7, 0x1, 4) => "60kHz",
        (0x7, 0x1, 5) => "600kHz",
        (0x7, 0x1, 6) => "6MHz",
        (0x7, 0x1, 7) => "60MHz",

        // Duty cycle (0x7, sub 0x2), Pulse width (0x7, sub 0x3): the vendor
        // range combo holds four items (60/600/6000/60000, see the spec's
        // §7.1) but no source says what the LCD calls them, so the rungs
        // stay unnamed and `range_choices` offers none of them.
        (0x7, _, _) => "",

        // Temperature (0x4, sub 0x2/0x3): fixed range
        (0x4, _, _) => "",

        _ => "",
    }
}

/// Parse a UT181A measurement payload (type 0x02 packet).
///
/// Common header (after type byte):
/// - byte 0:   type (0x02, already verified)
/// - byte 1:   misc (flags: bit7=HOLD, bits4-6=format, bit3=bargraph, etc.)
/// - byte 2:   misc2 (bit0=auto, bit1=HV, bit3=lead_error, bit4=COMP, bit5=record)
/// - bytes 3-4: mode word (uint16 LE)
/// - byte 5:   range (0x00=auto, 0x01-0x08=manual)
///
/// After header, the format-dependent value section starts at byte 6.
///
/// Full value = 13 bytes: float32(4) + precision(1) + unit_string(8)
/// Short value = 5 bytes: float32(4) + precision(1)
/// Parse a 13-byte "full value": float32(4) + precision(1) + unit_string(8).
fn parse_full_value(data: &[u8]) -> Result<(MeasuredValue, Option<String>, String)> {
    if data.len() < 13 {
        return Err(Error::invalid_response_msg(format!(
            "ut181a full value too short: {} bytes, need 13",
            data.len()
        )));
    }
    let float = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let precision = data[4];
    let unit = parse_unit_string(&data[5..13]);
    report_unrecognised_value(float, precision);
    let is_overload = precision & 0x01 != 0 || precision & 0x02 != 0;
    let dp = ((precision >> 4) & 0x0F) as usize;

    if is_overload || float.is_nan() || float.is_infinite() {
        Ok((MeasuredValue::Overload, None, unit))
    } else {
        let v = float as f64;
        Ok((MeasuredValue::Normal(v), Some(format!("{v:.dp$}")), unit))
    }
}

/// Parse a 5-byte "short value": float32(4) + precision(1).
fn parse_short_value(data: &[u8]) -> Result<(MeasuredValue, Option<String>)> {
    if data.len() < 5 {
        return Err(Error::invalid_response_msg(format!(
            "ut181a short value too short: {} bytes, need 5",
            data.len()
        )));
    }
    let float = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let precision = data[4];
    report_unrecognised_value(float, precision);
    let is_overload = precision & 0x01 != 0 || precision & 0x02 != 0;
    let dp = ((precision >> 4) & 0x0F) as usize;

    if is_overload || float.is_nan() || float.is_infinite() {
        Ok((MeasuredValue::Overload, None))
    } else {
        let v = float as f64;
        Ok((MeasuredValue::Normal(v), Some(format!("{v:.dp$}"))))
    }
}

/// Build an `AuxValue` from a full value parse result.
fn make_aux(
    label: &'static str,
    value: MeasuredValue,
    unit: &str,
    display_raw: Option<String>,
    elapsed_secs: Option<u32>,
) -> AuxValue {
    AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Owned(unit.to_string()),
        display_raw,
        elapsed_secs,
    }
}

pub(super) fn parse_measurement(payload: &[u8]) -> Result<Measurement> {
    // Minimum header: type(1) + misc(1) + misc2(1) + mode(2) + range(1) = 6
    check_len("ut181a", payload, 6)?;

    // The meter answers commands on the same stream it measures on, and only
    // type 0x02 carries a reading. `request_measurement` filters the stream
    // to that type, but a payload handed straight to `Protocol::parse_payload`
    // — a golden fixture — has not been through it, and a command reply read
    // as a measurement decodes its mode word out of the wrong bytes and
    // "parses" cleanly.
    if payload[0] != RESPONSE_MEASUREMENT {
        return Err(Error::invalid_response(
            format!(
                "ut181a: not a measurement packet (type {:#04x}, expected 0x02)",
                payload[0]
            ),
            payload,
        ));
    }

    let misc = payload[1];
    let misc2 = payload[2];
    let mode_word = u16::from_le_bytes([payload[3], payload[4]]);
    let range = payload[5];

    let format_type = (misc >> 4) & 0x07;
    let hold = misc & 0x80 != 0;
    let auto_range = misc2 & 0x01 != 0;
    let hv_warning = misc2 & 0x02 != 0;
    let lead_error = misc2 & 0x08 != 0;
    let comp_active = misc2 & 0x10 != 0;
    let record = misc2 & 0x20 != 0;

    let mode = decode_mode_word(mode_word);
    // Research spec §6 lists 79 mode words; any other decodes to a label the
    // nibble rule guesses, or to Unknown.
    if !mode::is_known_word(mode_word) {
        report_unknown(
            "ut181a",
            "mode word",
            format_args!("{mode_word:#06x}, shown as {mode}"),
        );
    }
    // §7 and §7.1: the dial family's range ladder.
    if !mode::is_known_range(mode_word, range) {
        report_unknown(
            "ut181a",
            "range byte",
            format_args!("mode {mode_word:#06x} range {range:#04x}"),
        );
    }
    // §5.1 defines neither misc bit 0 nor misc2 bits 2, 6 and 7.
    if misc & 0x01 != 0 || misc2 & 0xC4 != 0 {
        report_unknown(
            "ut181a",
            "misc bits",
            format_args!("misc {misc:#04x} misc2 {misc2:#04x}"),
        );
    }
    let data = &payload[6..]; // format-dependent value section

    let (value, display_raw, unit, aux_values) = match format_type {
        // Normal format (0x00)
        0x00 => {
            if data.len() < 13 {
                return Err(Error::invalid_response(
                    format!("ut181a normal format too short: {} bytes", payload.len()),
                    payload,
                ));
            }
            let (val, disp, unit) = parse_full_value(data)?;
            let mut aux = Vec::new();
            let mut offset = 13;
            let (aux1_label, aux2_label) = aux_labels(mode_word);

            // Aux1 (optional, misc bit 1)
            if misc & 0x02 != 0 && data.len() >= offset + 13 {
                let (av, ad, au) = parse_full_value(&data[offset..])?;
                report_positional_aux(aux1_label, mode_word);
                aux.push(make_aux(aux1_label, av, &au, ad, None));
                offset += 13;
            }
            // Aux2 (optional, misc bit 2)
            if misc & 0x04 != 0 && data.len() >= offset + 13 {
                let (av, ad, au) = parse_full_value(&data[offset..])?;
                report_positional_aux(aux2_label, mode_word);
                aux.push(make_aux(aux2_label, av, &au, ad, None));
                offset += 13;
            }
            // Bargraph (optional, misc bit 3) — skip for now, just advance offset
            if misc & 0x08 != 0 && data.len() >= offset + 12 {
                offset += 12; // float32(4) + unit(8)
            }

            // COMP extension (when misc2 bit 4 set)
            if comp_active && data.len() >= offset + 7 {
                let comp_mode = data[offset];
                let comp_result = data[offset + 1];
                let comp_prec = data[offset + 2];
                let high_float = f32::from_le_bytes([
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                ]);
                // Low limit present for INNER/OUTER modes
                let low_float = ((comp_mode == 0 || comp_mode == 1) && data.len() >= offset + 11)
                    .then(|| {
                        f32::from_le_bytes([
                            data[offset + 7],
                            data[offset + 8],
                            data[offset + 9],
                            data[offset + 10],
                        ])
                    });
                // §5.4: modes 0-3, results 0-1, digits in the low nibble and
                // two float32 limits.
                if comp_mode > 3
                    || comp_result > 1
                    || comp_prec > 0x0F
                    || !high_float.is_finite()
                    || low_float.is_some_and(|f| !f.is_finite())
                {
                    let end = offset + if low_float.is_some() { 11 } else { 7 };
                    report_unknown(
                        "ut181a",
                        "comp field",
                        format_args!("{:02X?}", &data[offset..end]),
                    );
                }
                // COMP digits live in the LOW nibble, unshifted — unlike
                // the other precision fields (sigrok protocol.c:112
                // "1 byte digits, not shifted as in other precision
                // fields"; decode at protocol.c:2123).
                let dp = (comp_prec & 0x0F) as usize;
                let comp_mode_str = match comp_mode {
                    0 => "INNER",
                    1 => "OUTER",
                    2 => "BELOW",
                    3 => "ABOVE",
                    _ => "?",
                };
                let result_str = if comp_result == 0 { "PASS" } else { "FAIL" };
                let high_v = high_float as f64;
                aux.push(make_aux(
                    "COMP High",
                    MeasuredValue::Normal(high_v),
                    &unit,
                    Some(format!("{high_v:.dp$}")),
                    None,
                ));

                if let Some(low_float) = low_float {
                    let low_v = low_float as f64;
                    aux.push(make_aux(
                        "COMP Low",
                        MeasuredValue::Normal(low_v),
                        &unit,
                        Some(format!("{low_v:.dp$}")),
                        None,
                    ));
                }

                debug!("ut181a: COMP {comp_mode_str} {result_str} high={high_float}");
            }

            (val, disp, unit, aux)
        }

        // Relative format (0x10 >> 4 = 1)
        0x01 => {
            // 3 full values: relative (delta), reference, absolute
            if data.len() < 39 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a relative format too short: {} bytes, need >= 45",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (rel_val, rel_disp, rel_unit) = parse_full_value(data)?;
            let (ref_val, ref_disp, ref_unit) = parse_full_value(&data[13..])?;
            let (abs_val, abs_disp, abs_unit) = parse_full_value(&data[26..])?;

            let aux = vec![
                make_aux("Reference", ref_val, &ref_unit, ref_disp, None),
                make_aux("Absolute", abs_val, &abs_unit, abs_disp, None),
            ];
            // Main value = delta (matches meter display)
            (rel_val, rel_disp, rel_unit, aux)
        }

        // Min/Max format (0x20 >> 4 = 2)
        0x02 => {
            // current(5) + max(5)+ts(4) + avg(5)+ts(4) + min(5)+ts(4) + unit(8) = 40
            if data.len() < 40 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a minmax format too short: {} bytes, need >= 46",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (cur_val, cur_disp) = parse_short_value(data)?;

            let (max_val, max_disp) = parse_short_value(&data[5..])?;
            let max_ts = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);

            let (avg_val, avg_disp) = parse_short_value(&data[14..])?;
            let avg_ts = u32::from_le_bytes([data[19], data[20], data[21], data[22]]);

            let (min_val, min_disp) = parse_short_value(&data[23..])?;
            let min_ts = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

            let unit = parse_unit_string(&data[32..40]);

            let aux = vec![
                make_aux("Max", max_val, &unit, max_disp, Some(max_ts)),
                make_aux("Average", avg_val, &unit, avg_disp, Some(avg_ts)),
                make_aux("Min", min_val, &unit, min_disp, Some(min_ts)),
            ];
            (cur_val, cur_disp, unit, aux)
        }

        // Peak format (0x40 >> 4 = 4)
        0x04 => {
            // 2 full values: peak max, peak min
            if data.len() < 26 {
                return Err(Error::invalid_response(
                    format!(
                        "ut181a peak format too short: {} bytes, need >= 32",
                        payload.len()
                    ),
                    payload,
                ));
            }
            let (pmax_val, pmax_disp, pmax_unit) = parse_full_value(data)?;
            let (pmin_val, pmin_disp, pmin_unit) = parse_full_value(&data[13..])?;

            let aux = vec![make_aux("Peak Min", pmin_val, &pmin_unit, pmin_disp, None)];
            (pmax_val, pmax_disp, pmax_unit, aux)
        }

        // Unknown format — try to parse as normal
        _ => {
            // §5.1 lists formats 0x00, 0x10, 0x20 and 0x40 only.
            report_unknown(
                "ut181a",
                "format type",
                format_args!("{:#04x}, read as normal", misc & 0x70),
            );
            if data.len() < 13 {
                return Err(Error::invalid_response(
                    format!("ut181a unknown format too short: {} bytes", payload.len()),
                    payload,
                ));
            }
            let (val, disp, unit) = parse_full_value(data)?;
            (val, disp, unit, vec![])
        }
    };

    // COMP extension can also apply to relative/peak, but only documented for
    // normal format. Parse it there only; for other formats, just set the flag.
    let flags = StatusFlags {
        hold,
        auto_range,
        hv_warning,
        lead_error,
        comp: comp_active,
        record,
        min: format_type == 0x02,
        max: format_type == 0x02,
        rel: format_type == 0x01,
        peak_max: format_type == 0x04,
        peak_min: format_type == 0x04,
        ..Default::default()
    };

    Ok(Measurement {
        mode,
        mode_raw: mode_word,
        range_raw: range,
        value,
        unit: Cow::Owned(unit),
        range_label: Cow::Borrowed(lookup_range_label(mode_word, range)),
        display_raw,
        flags,
        aux_values,
        ..Measurement::from_payload(payload)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::protocol::test_support::snapshot;

    pub(crate) fn make_payload(
        mode: u16,
        value: f32,
        precision: u8,
        unit: &[u8; 8],
        misc: u8,
        misc2: u8,
    ) -> Vec<u8> {
        let vbytes = value.to_le_bytes();
        let mbytes = mode.to_le_bytes();
        let mut p = vec![
            0x02,  // type
            misc,  // misc
            misc2, // misc2
            mbytes[0], mbytes[1], // mode word LE
            0x00,      // range
            vbytes[0], vbytes[1], vbytes[2], vbytes[3], // value
            precision, // precision
        ];
        p.extend_from_slice(unit); // 8 bytes
        p
    }

    /// A reply packet is not a reading. The stream filter drops those before
    /// they reach the parser, but a golden fixture goes straight in, and a
    /// 0x01 "OK" or a 0x03 saved measurement used to decode as a measurement
    /// with every field read a byte out of place.
    #[test]
    fn only_measurement_packets_parse() {
        let mut payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        for kind in [0x01, 0x03, 0x04, 0x05, 0x72] {
            payload[0] = kind;
            assert!(
                parse_measurement(&payload).is_err(),
                "packet type {kind:#04x} should not parse as a measurement"
            );
        }
    }

    /// The one payload whose every parsed field is pinned: an ordinary
    /// auto-ranging V DC reading, precision 0x40 (bits 4-7 = 4 decimals).
    #[test]
    fn parse_vdc() {
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC
mode_raw=0x3111
range_raw=0x00
value=Normal(12.345000267028809)
unit=VDC
range_label=Auto
display_raw=Some("12.3450")
flags=auto_range
aux=0
raw_payload=19"#
        );
    }

    #[test]
    fn parse_vac() {
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "V AC");
        assert_eq!(m.unit, "VAC");
    }

    #[test]
    fn parse_resistance() {
        let payload = make_payload(0x5111, 470.0, 0x20, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.unit, "~");
    }

    /// Precision bit 0 = +OL, and an overload carries no digits of its own.
    #[test]
    fn parse_overload_precision() {
        let payload = make_payload(0x5111, 0.0, 0x01, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x5111
range_raw=0x00
value=Overload
unit=~
range_label=Auto
display_raw=None
flags=
aux=0
raw_payload=19"#
        );
    }

    #[test]
    fn parse_hold_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x80, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hold);
    }

    #[test]
    fn parse_hv_warning() {
        let payload = make_payload(0x3111, 500.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x02);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.hv_warning);
    }

    #[test]
    fn decode_mode_word_known() {
        assert_eq!(decode_mode_word(0x1111), "V AC");
        assert_eq!(decode_mode_word(0x3111), "V DC");
        assert_eq!(decode_mode_word(0x5111), "Ω");
        assert_eq!(decode_mode_word(0x6211), "Capacitance");
        assert_eq!(decode_mode_word(0x7111), "Hz");
        assert_eq!(decode_mode_word(0x8111), "µA DC");
        assert_eq!(decode_mode_word(0xA111), "A DC");
    }

    #[test]
    fn decode_mode_word_variants() {
        assert_eq!(decode_mode_word(0x1121), "V AC Hz");
        assert_eq!(decode_mode_word(0x1131), "V AC Peak");
        assert_eq!(decode_mode_word(0x1141), "V AC LPF");
        assert_eq!(decode_mode_word(0x3121), "V DC AC+DC");
        assert_eq!(decode_mode_word(0x1112), "V AC REL");
        // DC currents with n1=2 are AC+DC (sigrok MODE_*_DC_ACDC), not Hz;
        // Hz applies only to the AC sub-function.
        assert_eq!(decode_mode_word(0x8121), "µA DC AC+DC");
        assert_eq!(decode_mode_word(0x9121), "mA DC AC+DC");
        assert_eq!(decode_mode_word(0xA121), "A DC AC+DC");
        assert_eq!(decode_mode_word(0x8221), "µA AC Hz");
        // 0x4121 = mV DC Peak (sigrok/antage)
        assert_eq!(decode_mode_word(0x4121), "mV DC Peak");
        // Non-REL exceptions to the n0=2 rule
        assert_eq!(decode_mode_word(0x5212), "Continuity (open)");
        assert_eq!(decode_mode_word(0x6112), "Diode Alarm");
        // Temperature display arrangements
        assert_eq!(decode_mode_word(0x4211), "°C");
        assert_eq!(decode_mode_word(0x4221), "°C T2");
        assert_eq!(decode_mode_word(0x4231), "°C T1-T2");
        assert_eq!(decode_mode_word(0x4241), "°C T2-T1");
        assert_eq!(decode_mode_word(0x4321), "°F T2");
    }

    #[test]
    fn parse_unit_string_latin1_degree() {
        // 0xB0 = '°' in Latin-1; from_utf8_lossy would produce U+FFFD.
        assert_eq!(parse_unit_string(&[0xB0, b'C', 0, 0, 0, 0, 0, 0]), "°C");
        assert_eq!(parse_unit_string(&[0xB0, b'F', 0, 0, 0, 0, 0, 0]), "°F");
    }

    #[test]
    fn aux_labels_by_mode() {
        // One probe on the main display, the other in aux1. The n1 = 1
        // arrangement is hardware-confirmed (issue #5); n1 = 2 is its
        // documented mirror.
        assert_eq!(aux_labels(0x4211).0, "T2");
        assert_eq!(aux_labels(0x4221).0, "T1");
        assert_eq!(aux_labels(0x4311).0, "T2");
        // Differential arrangements: no source says which probe feeds the aux
        // slot, so the label stays positional.
        assert_eq!(aux_labels(0x4231).0, "Aux1");
        assert_eq!(aux_labels(0x4241).0, "Aux1");
        // The modes decode_mode_word suffixes with " Hz" carry the frequency
        // and its period.
        assert_eq!(aux_labels(0x1121), ("Frequency", "Period"));
        assert_eq!(aux_labels(0x2121), ("Frequency", "Period"));
        assert_eq!(aux_labels(0x8221), ("Frequency", "Period"));
        // Everything else keeps the positional labels.
        assert_eq!(aux_labels(0x3111), ("Aux1", "Aux2"));
        assert_eq!(aux_labels(0x8121), ("Aux1", "Aux2"));
    }

    /// Hex as a capture report writes it in `raw_hex` (spaces optional).
    pub(crate) fn hex(s: &str) -> Vec<u8> {
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(clean.len().is_multiple_of(2), "odd-length hex: {clean}");
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).expect("valid hex"))
            .collect()
    }

    /// Real UT181A frame: temperature with two thermocouples connected
    /// (@diego351, issue #5, 2026-09-02 — the first hardware confirmation of
    /// a UT181A mode other than V DC).
    ///
    /// Reaches the normal-format aux walk that the synthetic `make_payload`
    /// frames never do: 26 payload bytes after the 6-byte header = 2 x 13, so
    /// the aux1 slot is what makes the frame add up. The meter sends 0xB0 for
    /// the degree sign (Latin-1), precision 0x10 is the LCD's one decimal,
    /// and temperature is fixed-range, so no label despite range byte 0x01.
    /// The frame's bytes, named so `crate::detect`'s tests can put the same
    /// real capture on the wire instead of a second copy of it.
    pub(crate) fn real_frame_temp_dual_probe() -> Vec<u8> {
        hex(
            "02 02 01 11 42 01 F0 ED CA 41 10 B0 43 00 43 00 00 00 00 26 FC C4 41 \
             10 B0 43 00 00 00 00 00 5A",
        )
    }

    #[test]
    fn parse_real_frame_temp_dual_probe() {
        let payload = real_frame_temp_dual_probe();
        assert_eq!(payload.len(), 32);

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=°C
mode_raw=0x4211
range_raw=0x01
value=Normal(25.366180419921875)
unit=°C
range_label=
display_raw=Some("25.4")
flags=auto_range
aux=1
aux1=T2 value=Normal(24.623119354248047) unit=°C display_raw=Some("24.6") elapsed_secs=None
raw_payload=32"#
        );
    }

    /// Real UT181A frame: V AC with the Hz secondary display, mains on the
    /// 600 V range (@diego351, issue #5, 2026-09-02).
    ///
    /// The 51 payload bytes after the header only add up as 13 + 13 + 13 + 12:
    /// main, aux1 and aux2 each carry a precision byte, the bargraph does not
    /// (spec §5.3). Get the bargraph field's size wrong and this frame
    /// desynchronises. misc2 bit 1 is the meter flagging mains voltage, and
    /// auto-range settled on 600V (range byte 0x03).
    /// As [`real_frame_temp_dual_probe`], for the V AC frame.
    pub(crate) fn real_frame_vac_hz() -> Vec<u8> {
        hex(
            "02 0E 03 21 11 03 52 38 6F 43 20 56 41 43 00 00 00 00 00 F6 08 48 42 \
             20 48 7A 00 00 00 00 00 5A D5 F8 9F 41 20 6D 73 00 43 00 00 00 00 3D \
             06 71 43 56 41 43 00 00 00 00 00",
        )
    }

    #[test]
    fn parse_real_frame_vac_hz_bargraph() {
        let payload = real_frame_vac_hz();
        assert_eq!(payload.len(), 57);

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V AC Hz
mode_raw=0x1121
range_raw=0x03
value=Normal(239.22000122070313)
unit=VAC
range_label=600V
display_raw=Some("239.22")
flags=auto_range,hv_warning
aux=2
aux1=Frequency value=Normal(50.008750915527344) unit=Hz display_raw=Some("50.01") elapsed_secs=None
aux2=Period value=Normal(19.99650001525879) unit=ms display_raw=Some("20.00") elapsed_secs=None
raw_payload=57"#
        );
    }

    #[test]
    fn decode_mode_word_unknown() {
        let s = decode_mode_word(0xFFFF);
        assert!(s.starts_with("Unknown"));
    }

    #[test]
    fn parse_nan_overload() {
        let payload = make_payload(0x5111, f32::NAN, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=Ω
mode_raw=0x5111
range_raw=0x00
value=Overload
unit=~
range_label=Auto
display_raw=None
flags=
aux=0
raw_payload=19"#
        );
    }

    #[test]
    fn parse_payload_too_short() {
        let payload = vec![0x02, 0x00, 0x00, 0x11, 0x31]; // 5 bytes, need >= 19
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn mode_raw_preserved() {
        let payload = make_payload(0x7211, 50.0, 0x00, b"%\0\0\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode_raw, 0x7211);
        assert_eq!(m.mode, "Duty %");
        // Range byte 0 answers "Auto" before the mode's ladder is consulted,
        // so this fixed-range mode reads as auto-ranging
        // (docs/verification-backlog.md).
        assert_eq!(m.range_label, "Auto");
    }

    /// The precision byte's high nibble is the decimal-place count: the three
    /// payloads below ask for 4, 2 and 0 places.
    #[test]
    fn display_raw_uses_precision_decimal_places() {
        // precision 0x40 => bits 4-7 = 4 decimal places
        let payload = make_payload(0x3111, 12.345, 0x40, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("12.3450"));

        // precision 0x20 => bits 4-7 = 2 decimal places
        let payload = make_payload(0x1111, 230.5, 0x20, b"VAC\0\0\0\0\0", 0x00, 0x00);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("230.50"));

        // precision 0x00 => 0 decimal places
        let payload = make_payload(0x5111, 470.0, 0x00, b"~\0\0\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.display_raw.as_deref(), Some("470"));
    }

    #[test]
    fn range_label_auto() {
        assert_eq!(lookup_range_label(0x3111, 0x00), "Auto");
        assert_eq!(lookup_range_label(0x5111, 0x00), "Auto");
    }

    #[test]
    fn range_label_voltage() {
        assert_eq!(lookup_range_label(0x3111, 1), "6V");
        assert_eq!(lookup_range_label(0x3111, 2), "60V");
        assert_eq!(lookup_range_label(0x3111, 3), "600V");
        assert_eq!(lookup_range_label(0x3111, 4), "1000V");
        // V AC uses same ranges
        assert_eq!(lookup_range_label(0x1111, 2), "60V");
    }

    #[test]
    fn range_label_millivolt() {
        assert_eq!(lookup_range_label(0x4111, 1), "60mV");
        assert_eq!(lookup_range_label(0x4111, 2), "600mV");
        assert_eq!(lookup_range_label(0x2111, 1), "60mV");
    }

    #[test]
    fn range_label_resistance() {
        assert_eq!(lookup_range_label(0x5111, 1), "600\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 3), "60k\u{2126}");
        assert_eq!(lookup_range_label(0x5111, 6), "60M\u{2126}");
    }

    #[test]
    fn range_label_capacitance() {
        assert_eq!(lookup_range_label(0x6211, 1), "6nF");
        assert_eq!(lookup_range_label(0x6211, 4), "6\u{00B5}F");
        assert_eq!(lookup_range_label(0x6211, 8), "60mF");
    }

    #[test]
    fn range_label_frequency() {
        assert_eq!(lookup_range_label(0x7111, 1), "60Hz");
        assert_eq!(lookup_range_label(0x7111, 5), "600kHz");
        assert_eq!(lookup_range_label(0x7111, 7), "60MHz");
    }

    #[test]
    fn range_label_current() {
        assert_eq!(lookup_range_label(0x8111, 1), "600\u{00B5}A");
        assert_eq!(lookup_range_label(0x9111, 2), "600mA");
        // A current: fixed range
        assert_eq!(lookup_range_label(0xA111, 1), "");
    }

    #[test]
    fn range_label_fixed_range_modes() {
        // Temperature, continuity, conductance, diode: no range label
        assert_eq!(lookup_range_label(0x4211, 1), ""); // Temp C
        assert_eq!(lookup_range_label(0x5211, 1), ""); // Continuity
        assert_eq!(lookup_range_label(0x5311, 1), ""); // Conductance
        assert_eq!(lookup_range_label(0x6111, 1), ""); // Diode
        assert_eq!(lookup_range_label(0x7211, 1), ""); // Duty cycle
    }

    /// The range byte is at payload[5], which `make_payload` sets to 0x00 —
    /// the meter's autorange.
    #[test]
    fn range_raw_populated() {
        let payload = make_payload(0x3111, 12.0, 0x20, b"VDC\0\0\0\0\0", 0x00, 0x01);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.range_raw, 0x00);
        assert_eq!(m.range_label, "Auto");
    }

    /// Build a full value block (13 bytes): float32 LE + precision + unit(8).
    fn full_value(val: f32, precision: u8, unit: &[u8; 8]) -> Vec<u8> {
        let mut v = val.to_le_bytes().to_vec();
        v.push(precision);
        v.extend_from_slice(unit);
        v
    }

    /// Build a short value block (5 bytes): float32 LE + precision.
    fn short_value(val: f32, precision: u8) -> Vec<u8> {
        let mut v = val.to_le_bytes().to_vec();
        v.push(precision);
        v
    }

    /// Build a relative format payload (format 0x10).
    pub(crate) fn make_relative_payload(
        mode: u16,
        delta: f32,
        reference: f32,
        absolute: f32,
    ) -> Vec<u8> {
        let mbytes = mode.to_le_bytes();
        let mut p = vec![
            0x02, // type
            0x10, // misc: format_type=1 (relative) in bits 4-6
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        p.extend_from_slice(&full_value(delta, 0x30, b"VDC\0\0\0\0\0"));
        p.extend_from_slice(&full_value(reference, 0x30, b"VDC\0\0\0\0\0"));
        p.extend_from_slice(&full_value(absolute, 0x30, b"VDC\0\0\0\0\0"));
        p
    }

    /// The main value is the delta; reference and absolute follow as
    /// sub-values.
    #[test]
    fn parse_relative_format() {
        let payload = make_relative_payload(0x3112, 2.345, 10.0, 12.345);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC REL
mode_raw=0x3112
range_raw=0x00
value=Normal(2.3450000286102295)
unit=VDC
range_label=Auto
display_raw=Some("2.345")
flags=rel,auto_range
aux=2
aux1=Reference value=Normal(10.0) unit=VDC display_raw=Some("10.000") elapsed_secs=None
aux2=Absolute value=Normal(12.345000267028809) unit=VDC display_raw=Some("12.345") elapsed_secs=None
raw_payload=45"#
        );
    }

    #[test]
    fn parse_relative_too_short() {
        // 6 header + only 26 bytes of data (need 39)
        let mut payload = vec![0x02, 0x10, 0x01, 0x11, 0x31, 0x00];
        payload.extend_from_slice(&full_value(1.0, 0x20, b"VDC\0\0\0\0\0"));
        payload.extend_from_slice(&full_value(2.0, 0x20, b"VDC\0\0\0\0\0"));
        // Missing third value
        assert!(parse_measurement(&payload).is_err());
    }

    /// A MIN/MAX-format payload (misc format_type 2) — how the meter reports
    /// while SET_MIN_MAX is on, which is what sets the MIN and MAX flags.
    pub(crate) fn minmax_payload(mode: u16) -> Vec<u8> {
        let mbytes = mode.to_le_bytes();
        let mut payload = vec![
            0x02, // type
            0x20, // misc: format_type=2 (minmax)
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // current: 5.0
        payload.extend_from_slice(&short_value(5.0, 0x30));
        // max: 10.0, timestamp 120s
        payload.extend_from_slice(&short_value(10.0, 0x30));
        payload.extend_from_slice(&120u32.to_le_bytes());
        // avg: 7.5, timestamp 60s
        payload.extend_from_slice(&short_value(7.5, 0x30));
        payload.extend_from_slice(&60u32.to_le_bytes());
        // min: 3.0, timestamp 30s
        payload.extend_from_slice(&short_value(3.0, 0x30));
        payload.extend_from_slice(&30u32.to_le_bytes());
        // shared unit
        payload.extend_from_slice(b"VDC\0\0\0\0\0");
        payload
    }

    /// The main value is the current reading; max, average and min follow as
    /// sub-values, each with the seconds since the mode started.
    #[test]
    fn parse_minmax_format() {
        let payload = minmax_payload(0x3111);
        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC
mode_raw=0x3111
range_raw=0x00
value=Normal(5.0)
unit=VDC
range_label=Auto
display_raw=Some("5.000")
flags=auto_range,min,max
aux=3
aux1=Max value=Normal(10.0) unit=VDC display_raw=Some("10.000") elapsed_secs=Some(120)
aux2=Average value=Normal(7.5) unit=VDC display_raw=Some("7.500") elapsed_secs=Some(60)
aux3=Min value=Normal(3.0) unit=VDC display_raw=Some("3.000") elapsed_secs=Some(30)
raw_payload=46"#
        );
    }

    #[test]
    fn parse_minmax_too_short() {
        let mbytes = 0x3111u16.to_le_bytes();
        let mut payload = vec![0x02, 0x20, 0x01, mbytes[0], mbytes[1], 0x00];
        // Only 10 bytes of data (need 40)
        payload.extend_from_slice(&short_value(5.0, 0x30));
        payload.extend_from_slice(&short_value(10.0, 0x30));
        assert!(parse_measurement(&payload).is_err());
    }

    /// The main value is the peak max; the peak min follows as a sub-value.
    #[test]
    fn parse_peak_format() {
        let mbytes = 0x3131u16.to_le_bytes(); // V DC Peak
        let mut payload = vec![
            0x02, // type
            0x40, // misc: format_type=4 (peak)
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        payload.extend_from_slice(&full_value(15.0, 0x30, b"VDC\0\0\0\0\0"));
        payload.extend_from_slice(&full_value(-3.0, 0x30, b"VDC\0\0\0\0\0"));

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC Peak
mode_raw=0x3131
range_raw=0x00
value=Normal(15.0)
unit=VDC
range_label=Auto
display_raw=Some("15.000")
flags=auto_range,peak_max,peak_min
aux=1
aux1=Peak Min value=Normal(-3.0) unit=VDC display_raw=Some("-3.000") elapsed_secs=None
raw_payload=32"#
        );
    }

    #[test]
    fn parse_peak_too_short() {
        let mbytes = 0x3131u16.to_le_bytes();
        let mut payload = vec![0x02, 0x40, 0x01, mbytes[0], mbytes[1], 0x00];
        // Only one full value (need two)
        payload.extend_from_slice(&full_value(15.0, 0x30, b"VDC\0\0\0\0\0"));
        assert!(parse_measurement(&payload).is_err());
    }

    #[test]
    fn parse_lead_error_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x08);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.lead_error);
    }

    #[test]
    fn parse_comp_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x10);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.comp);
    }

    #[test]
    fn parse_record_flag() {
        let payload = make_payload(0x3111, 1.0, 0x00, b"VDC\0\0\0\0\0", 0x00, 0x20);
        let m = parse_measurement(&payload).unwrap();
        assert!(m.flags.record);
    }

    /// A normal-format frame with COMP active carries the two limits as
    /// sub-values.
    #[test]
    fn parse_comp_extension() {
        let mbytes = 0x3111u16.to_le_bytes();
        let mut payload = vec![
            0x02, // type
            0x00, // misc: normal format
            0x11, // misc2: auto_range + COMP (bit 4)
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // Main value
        payload.extend_from_slice(&full_value(5.0, 0x30, b"VDC\0\0\0\0\0"));
        // COMP extension: INNER mode, PASS, precision 0x30, high=10.0, low=1.0
        payload.push(0x00); // comp_mode = INNER
        payload.push(0x00); // result = PASS
        payload.push(0x30); // precision
        payload.extend_from_slice(&10.0f32.to_le_bytes()); // high limit
        payload.extend_from_slice(&1.0f32.to_le_bytes()); // low limit

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(
            snapshot(&m),
            r#"mode=V DC
mode_raw=0x3111
range_raw=0x00
value=Normal(5.0)
unit=VDC
range_label=Auto
display_raw=Some("5.000")
flags=auto_range,comp
aux=2
aux1=COMP High value=Normal(10.0) unit=VDC display_raw=Some("10") elapsed_secs=None
aux2=COMP Low value=Normal(1.0) unit=VDC display_raw=Some("1") elapsed_secs=None
raw_payload=30"#
        );
    }

    #[test]
    fn parse_normal_with_aux1() {
        let mbytes = 0x4211u16.to_le_bytes(); // Temp C T1(T2)
        let mut payload = vec![
            0x02, // type
            0x02, // misc: bit 1 = has aux1
            0x01, // misc2: auto_range
            mbytes[0], mbytes[1], 0x00, // range
        ];
        // Main value: T1
        payload.extend_from_slice(&full_value(23.5, 0x10, b"\xB0C\0\0\0\0\0\0"));
        // Aux1: T2
        payload.extend_from_slice(&full_value(21.0, 0x10, b"\xB0C\0\0\0\0\0\0"));

        let m = parse_measurement(&payload).unwrap();
        assert_eq!(m.mode, "\u{00B0}C");
        assert_eq!(m.aux_values.len(), 1);
        assert_eq!(m.aux_values[0].label, "T2");
        assert!(
            matches!(m.aux_values[0].value, MeasuredValue::Normal(v) if (v - 21.0).abs() < 0.01)
        );
    }

    // --- Unrecognised data (protocol::unrecognised) -----------------------

    use crate::protocol::capture_reports;

    const VDC: &[u8; 8] = b"VDC\0\0\0\0\0";
    const CELSIUS: &[u8; 8] = b"\xB0C\0\0\0\0\0\0";

    /// Parse `payload`, keeping what it reported.
    fn parse_reporting(payload: &[u8]) -> (Result<Measurement>, Vec<String>) {
        capture_reports(|| parse_measurement(payload))
    }

    /// A measurement payload: the six header bytes, then `body`.
    fn frame(misc: u8, misc2: u8, mode: u16, range: u8, body: &[&[u8]]) -> Vec<u8> {
        let [lo, hi] = mode.to_le_bytes();
        let mut p = vec![0x02, misc, misc2, lo, hi, range];
        for part in body {
            p.extend_from_slice(part);
        }
        p
    }

    /// A normal-format V DC frame with the COMP extension (misc2 bit 4).
    fn comp_payload(
        comp_mode: u8,
        result: u8,
        precision: u8,
        high: f32,
        low: Option<f32>,
    ) -> Vec<u8> {
        let mut comp = vec![comp_mode, result, precision];
        comp.extend_from_slice(&high.to_le_bytes());
        if let Some(low) = low {
            comp.extend_from_slice(&low.to_le_bytes());
        }
        frame(0x00, 0x11, 0x3111, 0, &[&full_value(5.0, 0x30, VDC), &comp])
    }

    /// The highest manual rung research spec §7 and §7.1 give each dial
    /// family. A fixed range reports 1, as the real temperature frame does.
    fn top_rung(word: u16) -> u8 {
        match word >> 8 {
            0x11 | 0x31 | 0x72 | 0x73 => 4,
            0x21 | 0x41 | 0x81 | 0x82 | 0x91 | 0x92 => 2,
            0x51 => 6,
            0x62 => 8,
            0x71 => 7,
            _ => 1,
        }
    }

    /// The captured payloads and frames shaped as the spec documents them
    /// report nothing: a report there would warn every user of the meter.
    #[test]
    fn documented_frames_report_nothing() {
        let mut payloads = vec![
            real_frame_temp_dual_probe(),
            real_frame_vac_hz(),
            // V DC, leads open, bargraph only (PR #8; golden
            // `vdc_open_leads.yaml`).
            hex(
                "02 08 01 11 31 01 AF 73 D1 38 40 56 44 43 00 00 00 00 00 60 A2 23 BC \
                 56 44 43 00 00 00 00 00",
            ),
        ];
        // Every spec §6 word, on every rung of its family's ladder.
        for word in mode::known_words() {
            for range in 0..=top_rung(word) {
                let mut p = make_payload(word, 1.5, 0x10, VDC, 0x00, 0x01);
                p[5] = range;
                payloads.push(p);
            }
        }
        // The sub-values §6 names: the other probe on the T1/T2
        // arrangements, frequency and period on the Hz variants.
        for word in [0x4211, 0x4212, 0x4221, 0x4222, 0x4311, 0x4321] {
            let t = full_value(21.5, 0x10, CELSIUS);
            payloads.push(frame(0x02, 0x01, word, 1, &[&t, &t]));
        }
        let bargraph = [&230.0f32.to_le_bytes()[..], b"VAC\0\0\0\0\0".as_slice()].concat();
        for word in [0x1121, 0x2121, 0x8221, 0x9221, 0xA221] {
            let main = full_value(230.0, 0x20, b"VAC\0\0\0\0\0");
            let hz = full_value(50.0, 0x20, b"Hz\0\0\0\0\0\0");
            let period = full_value(20.0, 0x20, b"ms\0\0\0\0\0\0");
            payloads.push(frame(
                0x0E,
                0x03,
                word,
                1,
                &[&main, &hz, &period, &bargraph],
            ));
        }
        // The other formats, every documented status bit, and overloads.
        payloads.push(make_relative_payload(0x3112, 2.345, 10.0, 12.345));
        payloads.push(minmax_payload(0x3111));
        let peak = [full_value(15.0, 0x30, VDC), full_value(-3.0, 0x30, VDC)];
        payloads.push(frame(0x40, 0x01, 0x3131, 0, &[&peak[0], &peak[1]]));
        payloads.push(make_payload(0x3111, 1.5, 0x10, VDC, 0x80, 0x2B));
        for (value, precision) in [
            (0.0, 0x01),
            (0.0, 0x02),
            (9.9, 0x41),
            (f32::NAN, 0x01),
            (f32::INFINITY, 0x01),
            (f32::NEG_INFINITY, 0x02),
        ] {
            payloads.push(make_payload(
                0x5111,
                value,
                precision,
                b"~\0\0\0\0\0\0\0",
                0,
                1,
            ));
        }
        for (comp_mode, result) in [(0, 0), (1, 1), (2, 0), (3, 1)] {
            let low = (comp_mode < 2).then_some(1.0);
            payloads.push(comp_payload(comp_mode, result, 0x0F, 10.0, low));
        }

        for payload in &payloads {
            let (m, reports) = parse_reporting(payload);
            assert!(m.is_ok(), "{payload:02X?}: {m:?}");
            assert!(reports.is_empty(), "{payload:02X?}: {reports:?}");
        }
    }

    /// A word outside §6 keeps the label it always had.
    #[test]
    fn a_mode_word_off_the_spec_table_is_reported() {
        for (word, shown) in [
            (0x4131, "mV DC Peak"),  // the alternative mV DC Peak code
            (0x1122, "V AC Hz REL"), // no REL on a Hz variant
            (0x1113, "V AC"),        // no n0 = 3
            (0x3141, "V DC"),        // no n1 = 4 on V DC
            (0x5121, "Ω"),           // one variant only
            (0x4251, "°C"),          // no fifth probe arrangement
            (0xB111, "Unknown(0xb111)"),
            (0x6311, "Unknown(0x6311)"),
        ] {
            let (m, reports) = parse_reporting(&make_payload(word, 1.5, 0x10, VDC, 0, 1));
            assert_eq!(m.unwrap().mode, shown);
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised mode word: {word:#06x}, shown as {shown}"
                )]
            );
        }
    }

    /// One rung past each family's ladder is reported, except on duty cycle
    /// and pulse width, whose range bytes nothing names: there only §7's
    /// cap of 8 applies.
    #[test]
    fn a_range_byte_past_the_ladder_is_reported() {
        for word in mode::known_words() {
            let range = top_rung(word) + 1;
            let mut p = make_payload(word, 1.5, 0x10, VDC, 0, 1);
            p[5] = range;
            let (m, reports) = parse_reporting(&p);
            assert_eq!(m.unwrap().range_raw, range);
            if matches!(word >> 8, 0x72 | 0x73) {
                assert!(reports.is_empty(), "{word:#06x}: {reports:?}");
            } else {
                assert_eq!(
                    reports,
                    [format!(
                        "ut181a: unrecognised range byte: mode {word:#06x} range {range:#04x}"
                    )]
                );
            }
        }

        for (word, range) in [(0x7211, 9), (0x7312, 0xFF)] {
            let mut p = make_payload(word, 1.5, 0x10, b"%\0\0\0\0\0\0\0", 0, 0);
            p[5] = range;
            let (_, reports) = parse_reporting(&p);
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised range byte: mode {word:#06x} range {range:#04x}"
                )]
            );
        }

        // A word from no family: the word, and the byte past the cap.
        let mut p = make_payload(0xB111, 1.5, 0x10, VDC, 0, 0);
        p[5] = 9;
        let (_, reports) = parse_reporting(&p);
        assert_eq!(
            reports,
            [
                "ut181a: unrecognised mode word: 0xb111, shown as Unknown(0xb111)",
                "ut181a: unrecognised range byte: mode 0xb111 range 0x09",
            ]
        );
    }

    /// NaN or infinity with neither overload bit still reads as OL.
    #[test]
    fn a_non_finite_value_without_an_overload_bit_is_reported() {
        for bits in [0x7FC0_0000u32, 0x7F80_0000, 0xFF80_0000] {
            let value = f32::from_bits(bits);
            let p = make_payload(0x5111, value, 0x20, b"~\0\0\0\0\0\0\0", 0, 1);
            let (m, reports) = parse_reporting(&p);
            let m = m.unwrap();
            assert!(matches!(m.value, MeasuredValue::Overload), "{m:?}");
            assert_eq!(m.display_raw, None);
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised float: {value} ({bits:#010x}) without an overload bit, \
                     shown as OL"
                )]
            );
        }

        // A MIN/MAX short value too.
        let mut p = minmax_payload(0x3111);
        p[6..10].copy_from_slice(&0x7FC0_0000u32.to_le_bytes());
        let (m, reports) = parse_reporting(&p);
        assert!(matches!(m.unwrap().value, MeasuredValue::Overload));
        assert_eq!(
            reports,
            ["ut181a: unrecognised float: NaN (0x7fc00000) without an overload bit, shown as OL"]
        );
    }

    /// Precision bits 2-3, or both overload bits, are outside §5.2; the
    /// value is read as before.
    #[test]
    fn undefined_precision_bits_are_reported() {
        for (precision, display) in [
            (0x04, Some("1")),
            (0x28, Some("1.25")),
            (0x03, None),
            (0x43, None),
        ] {
            let p = make_payload(0x3111, 1.25, precision, VDC, 0, 1);
            let (m, reports) = parse_reporting(&p);
            assert_eq!(m.unwrap().display_raw.as_deref(), display);
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised precision byte: {precision:#04x}"
                )]
            );
        }

        // A MIN/MAX short value too.
        let mut p = minmax_payload(0x3111);
        p[10] = 0x3C;
        let (m, reports) = parse_reporting(&p);
        assert_eq!(m.unwrap().display_raw.as_deref(), Some("5.000"));
        assert_eq!(reports, ["ut181a: unrecognised precision byte: 0x3c"]);
    }

    /// misc bit 0 and misc2 bits 2, 6 and 7 are undefined (§5.1); the flags
    /// read the same.
    #[test]
    fn undefined_misc_bits_are_reported() {
        for (misc, misc2) in [(0x01, 0x01), (0x00, 0x05), (0x80, 0x41), (0x00, 0x81)] {
            let p = make_payload(0x3111, 1.5, 0x10, VDC, misc, misc2);
            let (m, reports) = parse_reporting(&p);
            let m = m.unwrap();
            assert_eq!(m.flags.hold, misc & 0x80 != 0);
            assert_eq!(m.flags.auto_range, misc2 & 0x01 != 0);
            assert!(m.aux_values.is_empty());
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised misc bits: misc {misc:#04x} misc2 {misc2:#04x}"
                )]
            );
        }
    }

    /// Formats 0x30, 0x50, 0x60 and 0x70 are read as normal, as before, and
    /// one too short for that still fails.
    #[test]
    fn an_undocumented_format_type_is_reported() {
        for misc in [0x30, 0x50, 0x60, 0x70] {
            let p = make_payload(0x3111, 1.5, 0x10, VDC, misc, 0x01);
            let (m, reports) = parse_reporting(&p);
            let m = m.unwrap();
            assert_eq!(m.display_raw.as_deref(), Some("1.5"));
            assert!(!(m.flags.rel || m.flags.min || m.flags.max || m.flags.peak_max));
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised format type: {misc:#04x}, read as normal"
                )]
            );
        }

        let p = make_payload(0x3111, 1.5, 0x10, VDC, 0x30, 0x01);
        let (m, reports) = parse_reporting(&p[..10]);
        assert!(m.is_err());
        assert_eq!(
            reports,
            ["ut181a: unrecognised format type: 0x30, read as normal"]
        );
    }

    /// A COMP mode past ABOVE, a result past FAIL, digits past the low
    /// nibble or a non-finite limit (§5.4); the limits parse as before.
    #[test]
    fn undocumented_comp_fields_are_reported() {
        for (p, limits) in [
            (comp_payload(4, 0, 0x03, 10.0, None), 1),
            (comp_payload(2, 2, 0x03, 10.0, None), 1),
            (comp_payload(3, 0, 0x13, 10.0, None), 1),
            (comp_payload(2, 0, 0x03, f32::INFINITY, None), 1),
            (comp_payload(0, 1, 0x03, 10.0, Some(f32::NAN)), 2),
        ] {
            let (m, reports) = parse_reporting(&p);
            assert_eq!(m.unwrap().aux_values.len(), limits, "{p:02X?}");
            assert_eq!(
                reports,
                [format!(
                    "ut181a: unrecognised comp field: {:02X?}",
                    &p[19..]
                )]
            );
        }
    }

    /// A sub-value where §6 names none keeps its positional label.
    #[test]
    fn a_sub_value_in_an_unnamed_slot_is_reported() {
        let v = full_value(1.5, 0x10, VDC);
        let t = full_value(21.5, 0x10, CELSIUS);
        for (p, labels, expected) in [
            (
                frame(0x02, 0x01, 0x3111, 0, &[&v, &v]),
                &["Aux1"][..],
                &["Aux1 in mode 0x3111"][..],
            ),
            (
                frame(0x06, 0x01, 0x3111, 0, &[&v, &v, &v]),
                &["Aux1", "Aux2"][..],
                &["Aux1 in mode 0x3111", "Aux2 in mode 0x3111"][..],
            ),
            (
                frame(0x02, 0x01, 0x4231, 1, &[&t, &t]),
                &["Aux1"][..],
                &["Aux1 in mode 0x4231"][..],
            ),
            (
                frame(0x06, 0x01, 0x4211, 1, &[&t, &t, &t]),
                &["T2", "Aux2"][..],
                &["Aux2 in mode 0x4211"][..],
            ),
        ] {
            let (m, reports) = parse_reporting(&p);
            let got: Vec<String> = m
                .unwrap()
                .aux_values
                .iter()
                .map(|a| a.label.to_string())
                .collect();
            assert_eq!(got, labels);
            let expected: Vec<String> = expected
                .iter()
                .map(|e| format!("ut181a: unrecognised aux value: {e}"))
                .collect();
            assert_eq!(reports, expected);
        }
    }
}
