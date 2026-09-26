//! BM78xBT packets → [`Measurement`]
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §6).
//!
//! Every field is read and lands in one of three places: the reading,
//! silence for what the spec documents and no reading shows (r4's "x" bits,
//! the RTC, the reserved bytes, the Power Source Flag), or a report for what
//! the spec does not cover, fields r4 prints as fixed values holding another
//! value among them.

use super::packet::{self, INFO_LEN, READING_LEN};
use super::tables::{self, Shown, Unknown};
use super::{ID, report};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{MeasuredValue, Measurement};
use crate::protocol::unknown_mode16;
use std::borrow::Cow;

/// Decode a payload as the stream delivers it and a replay file stores it:
/// the information packet and the reading, 56 bytes, or a reading alone, 32
/// bytes (see `packet::extract`).
pub(super) fn decode(payload: &[u8]) -> Result<Measurement> {
    let (info, reading) = match payload.len() {
        READING_LEN => (None, payload),
        n if n == INFO_LEN + READING_LEN => {
            let (info, reading) = payload.split_at(INFO_LEN);
            (Some(info), reading)
        }
        n => {
            return Err(Error::invalid_response(
                format!(
                    "{ID} payload is {n} bytes, expected {READING_LEN} or {}",
                    INFO_LEN + READING_LEN
                ),
                payload,
            ));
        }
    };
    check(reading, "reading", packet::is_reading)?;
    if let Some(info) = info {
        check(info, "information", packet::is_info)?;
        check_info(info);
    }
    let mut m = decode_reading(reading)?;
    // The information packet's [12] bit 1 (spec §6.9); a lone reading
    // carries no battery state, so it reads as not low.
    m.flags.low_battery = info.is_some_and(|info| info[12] & 0x02 != 0);
    m.raw_payload = payload.to_vec();
    Ok(m)
}

/// Report the information packet's fields that hold other than what r4
/// prints for them (spec §6.1, §6.9). The Power Source Flag and the reserve
/// bytes stay silent.
fn check_info(info: &[u8]) {
    // Protocol Version (spec §6.1).
    if info[4] != 0x01 {
        report(info, "protocol version");
    }
    // Device Category ID: multimeter or clamp-on meter (spec §6.1).
    if !matches!(info[5], 0x02 | 0x03) {
        report(info, "device category ID");
    }
    // Device Battery Status: `00`, or `02` at low battery (spec §6.9).
    if !matches!(info[12], 0x00 | 0x02) {
        report(info, "battery status");
    }
    // Reading Packet No.1-3: the four 32-byte blocks after it (spec §6.1).
    if info[16..19] != [0x04, 0x00, 0x00] {
        report(info, "reading packet number");
    }
    // Device Reading PK No.: one display (spec §6.1).
    if info[19] != 0x01 {
        report(info, "device reading PK number");
    }
}

/// Fail on a packet that is not `is_packet`: a CRC mismatch as such, any
/// other flaw (head, length, end) as an invalid response.
fn check(p: &[u8], what: &str, is_packet: fn(&[u8]) -> bool) -> Result<()> {
    if is_packet(p) {
        return Ok(());
    }
    let (stored, computed) = (packet::stored_crc(p), packet::computed_crc(p));
    if stored != computed {
        return Err(Error::ChecksumMismatch {
            expected: stored,
            actual: computed,
        });
    }
    Err(Error::invalid_response(
        format!("{ID} {what} packet is not framed as one"),
        p,
    ))
}

/// Flag0 [14] (spec §6.6).
const CREST: u8 = 0x80;
const REL: u8 = 0x40;
const HOLD: u8 = 0x20;
const AUTO_RANGE: u8 = 0x10;
const AUTO_HOLD: u8 = 0x08;
const ASCII: u8 = 0x04;
/// Flag1 [15] (spec §6.6).
const SIGN: u8 = 0x40;
const OL: u8 = 0x20;
const RECORD: u8 = 0x10;
const MAX: u8 = 0x08;
const MIN: u8 = 0x04;
const AVG: u8 = 0x02;

/// Decode one CRC-checked 32-byte reading packet (spec §6.2).
fn decode_reading(r: &[u8]) -> Result<Measurement> {
    // Logging Data set ID and Device Reading PK ID, both 1 on a BM78xBT
    // (spec §6.2).
    if r[4..7] != [0x01, 0x00, 0x00] {
        report(r, "logging data set ID");
    }
    if r[7] != 0x01 {
        report(r, "device reading PK ID");
    }
    // Device Type: 1 meter, 0 sensor (spec §6.2).
    if r[17] != 0x01 {
        report(r, "device type");
    }
    let (flag0, flag1) = (r[14], r[15]);
    let (main, sub) = (r[18], r[20]);
    let mode_raw = u16::from_be_bytes([main, sub]);

    let (mode, dc) = match tables::function(main, sub) {
        Ok(function) => (Cow::Borrowed(function.name), function.dc),
        Err(unknown) => {
            report(
                r,
                match unknown {
                    Unknown::Main => "main function ID",
                    Unknown::Sub => "sub-function ID",
                },
            );
            (unknown_mode16(mode_raw), false)
        }
    };

    let ascii = flag0 & ASCII != 0;
    let overload = flag1 & OL != 0;
    // The unit and the scaling fields describe a number; while the display
    // shows a word, r4 gives them no meaning, so they are not checked.
    let numeric = !ascii || overload;
    let (unit, decimals) = scaling(r, numeric);
    let (value, display_raw, lead_error) = if overload {
        // "Device Reading [2] ~ [0] can be ignored" (spec §6.3).
        (MeasuredValue::Overload, None, false)
    } else if ascii {
        let (value, lead_error) = ascii_reading(r, main == tables::EF);
        (value, None, lead_error)
    } else {
        let (value, digits) = number(r, flag1, decimals);
        (MeasuredValue::Normal(value), Some(digits), false)
    };

    Ok(Measurement {
        mode,
        mode_raw,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags: StatusFlags {
            lead_error,
            dc,
            loz: main == tables::AUTO_CHECK,
            ..flags(r, flag0, flag1)
        },
        ..Measurement::from_payload(r)
    })
}

/// The most digits after the point a reading is given: 10^9 fits a `u32`.
const MAX_DECIMALS: u32 = 9;

/// The reading's unit, with its prefix, and the digits after its decimal
/// point (spec §6.3, §6.4). `check` reports the fields r4 does not cover.
fn scaling(r: &[u8], check: bool) -> (&'static str, u32) {
    let (point, prefix_byte, unit_code, digits) = (r[24], r[25], r[26], r[27]);
    let prefix = tables::prefix_index(prefix_byte);
    let unit = tables::unit(unit_code, prefix);
    if check {
        if prefix.is_none() {
            report(r, "metric prefix");
        }
        if unit.is_none() {
            report(r, "function unit");
        }
        // r4 lists 3 to 6 digits (spec §6.2).
        if !(3..=6).contains(&digits) {
            report(r, "display digit number");
        }
        // r4's table puts the point before the last digit at most (spec
        // §6.3); 0 is no point.
        if point != 0 && point >= digits {
            report(r, "decimal point");
        }
    }
    // [24] counts the digits before the point, 0 meaning no point, so the
    // digits after it are [27] − [24] (spec §6.3). Capped, so a [27] far
    // outside r4's 3 to 6 cannot overflow the divisor.
    let decimals = if point == 0 || point > digits {
        0
    } else {
        u32::from(digits - point).min(MAX_DECIMALS)
    };
    (unit.unwrap_or(""), decimals)
}

/// The count [21..23], little-endian 24-bit two's complement (spec §6.3).
fn count(r: &[u8]) -> i32 {
    i32::from_le_bytes([r[21], r[22], r[23], 0]) << 8 >> 8
}

/// A numeric reading: the value in the prefixed unit, and the digits with
/// the meter's decimal point.
///
/// Negative when Flag1's sign bit is set or the count is below zero: r4
/// defines both, and which the meter uses is open (spec §6.3, §11.9). A
/// negative count with the sign bit clear is the one case r4's two
/// definitions contradict, so it is reported.
fn number(r: &[u8], flag1: u8, decimals: u32) -> (f64, String) {
    let count = count(r);
    let flagged = flag1 & SIGN != 0;
    if count < 0 && !flagged {
        report(r, "negative count without the sign flag");
    }
    let negative = flagged || count < 0;
    let magnitude = count.unsigned_abs();
    let sign = if negative { "-" } else { "" };
    let divisor = 10u32.pow(decimals);
    let digits = if decimals == 0 {
        format!("{sign}{magnitude}")
    } else {
        let width = decimals as usize;
        format!(
            "{sign}{}.{:0width$}",
            magnitude / divisor,
            magnitude % divisor
        )
    };
    let value = f64::from(magnitude) / f64::from(divisor);
    (if negative { -value } else { value }, digits)
}

/// What an ASCII reading shows (spec §6.7), and whether it is the input
/// warning.
fn ascii_reading(r: &[u8], in_ef: bool) -> (MeasuredValue, bool) {
    // A code is never negative; one that is reads as unknown.
    let code = u32::try_from(count(r)).unwrap_or(u32::MAX);
    let value = match tables::shown(code, in_ef) {
        Shown::Overload => {
            report(r, "display word");
            MeasuredValue::Overload
        }
        Shown::Word(word) => MeasuredValue::NoReading(word),
        Shown::FieldStrength(level) => MeasuredValue::NcvLevel(level),
        Shown::FieldReady => MeasuredValue::NcvLevel(0),
        Shown::AppWord(word) | Shown::OutsideEf(word) => {
            report(r, "display word");
            MeasuredValue::NoReading(word)
        }
        Shown::Unknown => {
            report(r, "display word");
            MeasuredValue::NoReading("?")
        }
    };
    (value, code == tables::INPUT_ERROR)
}

/// The annunciators (spec §6.6). r4's "x" bits and Flag2 are silent.
fn flags(r: &[u8], flag0: u8, flag1: u8) -> StatusFlags {
    let crest = flag0 & CREST != 0;
    let record = flag1 & RECORD != 0;
    let (max, min, avg) = (flag1 & MAX != 0, flag1 & MIN != 0, flag1 & AVG != 0);
    let mut flags = StatusFlags {
        hold: flag0 & (HOLD | AUTO_HOLD) != 0,
        rel: flag0 & REL != 0,
        auto_range: flag0 & AUTO_RANGE != 0,
        record,
        ..StatusFlags::default()
    };
    if crest {
        // CREST lights MAX or MIN, CMAX or CMIN (spec §6.6; BM788BT manual
        // p.18);
        // AVG accompanies RECORD alone (r4's footnote 3).
        if record || avg || max == min {
            report(r, "CREST annunciators");
        }
        flags.peak_max = max;
        flags.peak_min = min;
    } else {
        // MAX, MIN and AVG accompany RECORD or CREST (r4's footnotes 2, 3).
        if !record && (max || min || avg) {
            report(r, "MAX/MIN/AVG annunciators");
        }
        flags.max = max;
        flags.min = min;
        flags.avg = avg;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::super::packet::tests::{example_info, example_reading, hex, sealed};
    use super::*;
    use crate::protocol::capture_reports;
    use crate::protocol::test_support::snapshot;

    /// A numeric reading's value, `None` for any other kind.
    fn normal(v: &MeasuredValue) -> Option<f64> {
        match v {
            MeasuredValue::Normal(v) => Some(*v),
            _ => None,
        }
    }

    /// Decode `p`, and what it reported.
    fn decoded(p: &[u8]) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(p));
        (m.unwrap(), reports)
    }

    /// Decode `p` expecting no report.
    fn quiet(p: &[u8]) -> Measurement {
        let (m, reports) = decoded(p);
        assert!(reports.is_empty(), "{reports:?}");
        m
    }

    /// Decode `p` expecting exactly one report, naming `what`.
    fn reported(p: &[u8], what: &str) -> Measurement {
        let (m, reports) = decoded(p);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].starts_with(&format!("bm78xbt: unrecognised {what}: packet [")),
            "{reports:?}"
        );
        m
    }

    /// Example 1 with `set` applied to it, resealed.
    fn reading(set: impl FnOnce(&mut [u8])) -> Vec<u8> {
        let mut r = example_reading();
        set(&mut r);
        sealed(r)
    }

    /// A reading of `count` in `main`/`sub`, flags clear but AUTO, point 1
    /// of 5 digits, volts.
    fn with(main: u8, sub: u8, count: i32, set: impl FnOnce(&mut [u8])) -> Vec<u8> {
        reading(|r| {
            r[14] = AUTO_RANGE;
            r[15] = 0;
            r[18] = main;
            r[20] = sub;
            r[21..24].copy_from_slice(&count.to_le_bytes()[..3]);
            set(r);
        })
    }

    /// An ASCII reading of `code` in `main`/`sub`.
    fn ascii(main: u8, sub: u8, code: i32) -> Vec<u8> {
        with(main, sub, code, |r| r[14] |= ASCII)
    }

    /// Spec §10 example 2 with its battery byte set to `battery`.
    fn info(battery: u8) -> Vec<u8> {
        let mut info = example_info();
        info[12] = battery;
        sealed(info)
    }

    /// Spec §10 example 1 and example 2, byte for byte.
    #[test]
    fn example_1_negative_dc_volts() {
        let payload = [example_info(), example_reading()].concat();
        assert_eq!(
            snapshot(&quiet(&payload)),
            "mode=DC V\n\
             mode_raw=0x301\n\
             range_raw=0x00\n\
             value=Normal(-1.2345)\n\
             unit=V\n\
             range_label=\n\
             display_raw=Some(\"-1.2345\")\n\
             flags=auto_range,dc\n\
             aux=0\n\
             raw_payload=56"
        );
    }

    /// The information packet reads for its battery bit alone.
    #[test]
    fn example_2_is_not_low_battery() {
        let payload = [example_info(), example_reading()].concat();
        assert!(!quiet(&payload).flags.low_battery);
    }

    #[test]
    fn low_battery_is_bit_1_of_the_information_packet() {
        for (battery, low) in [(0x00, false), (0x02, true)] {
            let payload = [info(battery), example_reading()].concat();
            assert_eq!(quiet(&payload).flags.low_battery, low, "{battery:#04x}");
        }
        // Values r4 does not give are reported, and still read by bit 1.
        for (battery, low) in [(0x03, true), (0x01, false), (0x40, false)] {
            let payload = [info(battery), example_reading()].concat();
            let m = reported(&payload, "battery status");
            assert_eq!(m.flags.low_battery, low, "{battery:#04x}");
        }
    }

    /// A lone reading is what the first read after joining mid-notification
    /// gets: it reads as not low, silently.
    #[test]
    fn a_lone_reading_decodes_quietly() {
        let m = quiet(&example_reading());
        assert_eq!(m.display_raw.as_deref(), Some("-1.2345"));
        assert!(!m.flags.low_battery);
        assert_eq!(m.raw_payload, example_reading());
    }

    #[test]
    fn the_rtc_reserved_and_x_bits_stay_silent() {
        for set in [
            (|r: &mut [u8]| r[8..14].fill(0xA5)) as fn(&mut [u8]),
            |r| r[19] = 0x77,
            |r| r[14] |= 0x03,
            |r| r[15] |= 0x81,
            |r| r[16] = 0xFF,
        ] {
            let m = quiet(&reading(set));
            assert_eq!(m.flags, quiet(&example_reading()).flags);
            assert_eq!(m.display_raw.as_deref(), Some("-1.2345"));
        }
        // A clamp-on meter's category, the Power Source Flag and the
        // reserve bytes.
        let mut odd_info = example_info();
        odd_info[5] = 0x03;
        odd_info[13..16].fill(0x55);
        let payload = [sealed(odd_info), example_reading()].concat();
        quiet(&payload);
    }

    /// The fields r4 prints as fixed values are reported when they hold
    /// another, once per kind, and the reading still decodes.
    #[test]
    fn fixed_fields_holding_another_value_are_reported() {
        for (set, what) in [
            (
                (|r: &mut [u8]| r[4..7].fill(0x42)) as fn(&mut [u8]),
                "logging data set ID",
            ),
            (|r| r[6] = 0x01, "logging data set ID"),
            (|r| r[7] = 0x02, "device reading PK ID"),
        ] {
            let m = reported(&reading(set), what);
            assert_eq!(m.display_raw.as_deref(), Some("-1.2345"));
        }
        for (at, value, what) in [
            (4, 0x02, "protocol version"),
            (5, 0x01, "device category ID"),
            (5, 0x04, "device category ID"),
            (16, 0x03, "reading packet number"),
            (18, 0x01, "reading packet number"),
            (19, 0x02, "device reading PK number"),
        ] {
            let mut info = example_info();
            info[at] = value;
            let payload = [sealed(info), example_reading()].concat();
            let m = reported(&payload, what);
            assert_eq!(m.display_raw.as_deref(), Some("-1.2345"), "{what}");
        }
        let mut filled = example_info();
        filled[13..20].fill(0x55);
        let (_, reports) = decoded(&[sealed(filled), example_reading()].concat());
        assert_eq!(reports.len(), 2, "{reports:?}");
        assert!(reports[0].starts_with("bm78xbt: unrecognised reading packet number: packet ["));
        assert!(reports[1].starts_with("bm78xbt: unrecognised device reading PK number: packet ["));
    }

    #[test]
    fn the_decimal_point_counts_digits_before_it() {
        // [24], [27], count → digits (spec §6.3's table).
        for (point, digits, count, shown) in [
            (0, 5, 12345, "12345"),
            (1, 5, 12345, "1.2345"),
            (2, 5, 12345, "12.345"),
            (3, 5, 12345, "123.45"),
            (4, 5, 12345, "1234.5"),
            (0, 4, 1234, "1234"),
            (1, 4, 1234, "1.234"),
            (3, 4, 1234, "123.4"),
            (2, 6, 123456, "12.3456"),
            (1, 3, 123, "1.23"),
            (2, 5, 5, "0.005"),
        ] {
            let p = with(0x03, 0x01, count, |r| {
                r[24] = point;
                r[27] = digits;
            });
            let m = quiet(&p);
            assert_eq!(m.display_raw.as_deref(), Some(shown), "{point}/{digits}");
            assert_eq!(
                normal(&m.value),
                Some(shown.parse().unwrap()),
                "{point}/{digits}"
            );
        }
    }

    /// The value is in the prefixed unit, as the meter shows it.
    #[test]
    fn the_prefix_goes_on_the_unit() {
        for (prefix, unit_code, unit) in [
            (0xFD, 0x02, "mV"),
            (0xFA, 0x03, "µA"),
            (0x03, 0x04, "kΩ"),
            (0x06, 0x04, "MΩ"),
            (0xF7, 0x05, "nS"),
            (0xF7, 0x06, "nF"),
            (0x03, 0x08, "kHz"),
            (0x00, 0x0A, "%"),
            (0x00, 0x14, "°C"),
            (0x00, 0x15, "°F"),
            (0x00, 0x4F, "%"),
            (0x09, 0x04, "GΩ"),
        ] {
            let m = quiet(&with(0x0D, 0x00, 5000, |r| {
                r[25] = prefix;
                r[26] = unit_code;
            }));
            assert_eq!(m.unit, unit);
            assert_eq!(normal(&m.value), Some(0.5));
        }
    }

    #[test]
    fn overload_ignores_the_count() {
        let m = quiet(&with(0x0D, 0x00, 0x7F_FFFF, |r| r[15] = OL));
        assert!(matches!(m.value, MeasuredValue::Overload));
        assert_eq!(m.display_raw, None);
        assert_eq!(m.mode, "Ω");
        let negative = quiet(&with(0x03, 0x01, 0, |r| r[15] = OL | SIGN));
        assert!(matches!(negative.value, MeasuredValue::Overload));
    }

    /// Either sign convention reads negative; together as example 1 too.
    #[test]
    fn both_sign_cases_read_negative() {
        let flagged = quiet(&with(0x03, 0x01, 12345, |r| r[15] = SIGN));
        assert_eq!(flagged.display_raw.as_deref(), Some("-1.2345"));
        let both = quiet(&with(0x03, 0x01, -12345, |r| r[15] = SIGN));
        assert_eq!(normal(&both.value), Some(-1.2345));
        let count_only = reported(
            &with(0x03, 0x01, -12345, |_| {}),
            "negative count without the sign flag",
        );
        assert_eq!(count_only.display_raw.as_deref(), Some("-1.2345"));
        assert_eq!(normal(&count_only.value), Some(-1.2345));
        let positive = quiet(&with(0x03, 0x01, 12345, |_| {}));
        assert_eq!(normal(&positive.value), Some(1.2345));
    }

    /// r4's two count examples (spec §6.2): 32768 and −32768.
    #[test]
    fn the_count_is_signed_24_bit() {
        assert_eq!(count(&with(0x03, 0x01, 0x8000, |_| {})), 32768);
        let mut r = example_reading();
        r[21..24].copy_from_slice(&[0x00, 0x80, 0xFF]);
        assert_eq!(count(&r), -32768);
    }

    #[test]
    fn every_ascii_code() {
        let value = |p: &[u8]| quiet(p).value;
        // 0 is the app's OL alone (spec §11.15).
        assert!(matches!(
            reported(&ascii(0x0D, 0x00, 0), "display word").value,
            MeasuredValue::Overload
        ));
        assert!(matches!(
            value(&ascii(0x02, 0x03, 1)),
            MeasuredValue::NoReading("Auto")
        ));
        let iner = quiet(&ascii(0x03, 0x01, 2));
        assert!(matches!(iner.value, MeasuredValue::NoReading("InEr")));
        assert!(iner.flags.lead_error);
        assert!(!quiet(&ascii(0x03, 0x01, 1)).flags.lead_error);
        for (code, dashes) in (3..=7).zip(["-", "- -", "- - -", "- - - -", "- - - - -"]) {
            assert!(
                matches!(value(&ascii(0x03, 0x01, code)), MeasuredValue::NoReading(w) if w == dashes),
                "{code}"
            );
            for sub in [0, 1] {
                assert!(
                    matches!(value(&ascii(0x22, sub, code)), MeasuredValue::NcvLevel(l) if i32::from(l) == code - 2),
                    "{code}"
                );
            }
        }
        for (code, word) in [(0x0A, "EF-H"), (0x0B, "EF-L")] {
            for sub in [0x01, 0x00] {
                assert!(matches!(
                    value(&ascii(0x22, sub, code)),
                    MeasuredValue::NcvLevel(0)
                ));
            }
            // Outside EF detection, where no source puts them.
            let m = reported(&ascii(0x03, 0x01, code), "display word");
            assert!(
                matches!(m.value, MeasuredValue::NoReading(w) if w == word),
                "{code}"
            );
        }
        for (code, word) in [
            (0x08, "diSC"),
            (0x09, "CALi"),
            (0x0C, "rS-3"),
            (0x0D, "SoC"),
            (0x0E, "SoH"),
            (0x0F, "bAd"),
        ] {
            let m = reported(&ascii(0x03, 0x01, code), "display word");
            assert!(
                matches!(m.value, MeasuredValue::NoReading(w) if w == word),
                "{code}"
            );
        }
        for code in [0x10, 0x7F_FFFF, -1] {
            let m = reported(&ascii(0x03, 0x01, code), "display word");
            assert!(matches!(m.value, MeasuredValue::NoReading("?")), "{code}");
        }
    }

    /// A word on the display gives the scaling and unit fields no meaning,
    /// so none is checked.
    #[test]
    fn an_ascii_reading_checks_no_scaling_field() {
        let m = quiet(&with(0x22, 0x01, 0x0A, |r| {
            r[14] |= ASCII;
            r[24] = 9;
            r[25] = 0x01;
            r[26] = 0x00;
            r[27] = 0;
        }));
        assert_eq!(m.mode, "EF-H");
        assert!(matches!(m.value, MeasuredValue::NcvLevel(0)));
        assert_eq!(m.unit, "");
    }

    #[test]
    fn flags_follow_the_annunciators() {
        let f = |f0: u8, f1: u8| {
            quiet(&with(0x03, 0x01, 1, |r| {
                r[14] = f0;
                r[15] = f1;
            }))
            .flags
        };
        assert!(f(HOLD, 0).hold);
        assert!(f(AUTO_HOLD, 0).hold);
        assert!(f(REL, 0).rel);
        assert!(f(AUTO_RANGE, 0).auto_range);
        assert!(!f(0, 0).auto_range);
        let all = f(0, RECORD | MAX | MIN | AVG);
        assert!(all.record && all.max && all.min && all.avg);
        for (bit, pick) in [
            (MAX, (|f: StatusFlags| f.max) as fn(StatusFlags) -> bool),
            (MIN, |f| f.min),
            (AVG, |f| f.avg),
        ] {
            let one = f(0, RECORD | bit);
            assert!(one.record && pick(one));
            assert!(!one.peak_max && !one.peak_min);
        }
        let cmax = f(CREST, MAX);
        assert!(cmax.peak_max && !cmax.peak_min && !cmax.max && !cmax.record);
        let cmin = f(CREST, MIN);
        assert!(cmin.peak_min && !cmin.peak_max && !cmin.min);
    }

    #[test]
    fn dc_subs_set_dc_and_autocheck_sets_loz() {
        let m = quiet(&with(0x04, 0x01, 1, |_| {}));
        assert_eq!(m.mode, "DC mV");
        assert!(m.flags.dc && !m.flags.loz);
        for (sub, name, is_dc) in [
            (0, "LoZ AC V", false),
            (1, "LoZ DC V", true),
            (2, "LoZ Ω", false),
            (3, "Auto V", false),
        ] {
            let m = quiet(&with(0x02, sub, 1, |_| {}));
            assert_eq!((m.mode.as_ref(), m.flags.dc), (name, is_dc));
            assert!(m.flags.loz, "{name}");
        }
        assert!(!quiet(&with(0x03, 0x02, 1, |_| {})).flags.dc, "AC+DC V");
    }

    #[test]
    fn mode_raw_is_main_then_sub() {
        let m = quiet(&with(0x06, 0x08, 7510, |r| {
            r[24] = 2;
            r[27] = 4;
            r[26] = 0x4F;
        }));
        assert_eq!((m.mode.as_ref(), m.mode_raw), ("% 4-20mA", 0x0608));
        assert_eq!(
            (m.unit.as_ref(), m.display_raw.as_deref()),
            ("%", Some("75.10"))
        );
    }

    #[test]
    fn every_report_path() {
        reported(&reading(|r| r[17] = 0x00), "device type");
        let m = reported(&with(0x01, 0x00, 1, |_| {}), "main function ID");
        assert_eq!((m.mode.as_ref(), m.mode_raw), ("Unknown(0x0100)", 0x0100));
        let m = reported(&with(0x03, 0x04, 1, |_| {}), "sub-function ID");
        assert_eq!(m.mode, "Unknown(0x0304)");
        assert!(!m.flags.dc);
        reported(&with(0x03, 0x01, 1, |r| r[27] = 7), "display digit number");
        reported(&with(0x03, 0x01, 1, |r| r[27] = 2), "display digit number");
        let m = reported(&with(0x03, 0x01, 12345, |r| r[24] = 6), "decimal point");
        assert_eq!(m.display_raw.as_deref(), Some("12345"));
        // A point at the digit count, which r4's table leaves empty.
        for digits in [4, 5] {
            let m = reported(
                &with(0x03, 0x01, 1234, |r| {
                    r[24] = digits;
                    r[27] = digits;
                }),
                "decimal point",
            );
            assert_eq!(m.display_raw.as_deref(), Some("1234"), "{digits}");
        }
        let m = reported(&with(0x03, 0x01, 1, |r| r[25] = 0x0C), "metric prefix");
        assert_eq!(m.unit, "V");
        let m = reported(&with(0x03, 0x01, 1, |r| r[26] = 0x07), "function unit");
        assert_eq!(m.unit, "");
        reported(
            &with(0x03, 0x01, -1, |_| {}),
            "negative count without the sign flag",
        );
        reported(&ascii(0x03, 0x01, 0x08), "display word");
        for f1 in [0, MAX | MIN, RECORD | MAX, AVG | MAX] {
            reported(
                &with(0x03, 0x00, 1, |r| {
                    r[14] = CREST;
                    r[15] = f1;
                }),
                "CREST annunciators",
            );
        }
        for f1 in [MAX, MIN, AVG] {
            reported(
                &with(0x03, 0x01, 1, |r| r[15] = f1),
                "MAX/MIN/AVG annunciators",
            );
        }
    }

    /// Documented values stay silent: the struck line-frequency subs and
    /// the app's AutoCheck sub.
    #[test]
    fn documented_codes_decode_silently() {
        for main in [0x03, 0x05, 0x06, 0x07] {
            assert_eq!(quiet(&with(main, 0x03, 5000, |_| {})).mode, "Line Hz");
        }
        assert_eq!(quiet(&with(0x23, 0x00, 5000, |_| {})).mode, "Line Hz");
        assert_eq!(quiet(&with(0x02, 0x02, 5000, |_| {})).mode, "LoZ Ω");
    }

    /// A small xorshift generator, so arbitrary input needs no new
    /// dependency.
    fn pseudo_random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    /// Any field values in a CRC-valid packet decode without a panic.
    #[test]
    fn arbitrary_fields_never_panic() {
        let mut seed = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            let r = reading(|r| {
                for byte in &mut r[4..28] {
                    *byte = pseudo_random(&mut seed) as u8;
                }
            });
            let _ = capture_reports(|| decode(&r));
        }
        let wide = with(0x03, 0x01, 12345, |r| r[27] = 200);
        let m = reported(&wide, "display digit number");
        assert_eq!(m.display_raw.as_deref(), Some("0.000012345"));
    }

    #[test]
    fn the_payload_is_checked_before_it_is_read() {
        let payload = [example_info(), example_reading()].concat();
        for len in [0, 31, 33, 55, 57, 152] {
            let mut odd = payload.clone();
            odd.resize(len, 0);
            assert!(
                matches!(decode(&odd), Err(Error::InvalidResponse { .. })),
                "{len}"
            );
        }
        let mut bad = example_reading();
        bad[21] ^= 0x01;
        assert!(matches!(
            decode(&bad),
            Err(Error::ChecksumMismatch {
                expected: 0xC376,
                ..
            })
        ));
        let mut bad_info = payload.clone();
        bad_info[12] = 0x02;
        assert!(matches!(
            decode(&bad_info),
            Err(Error::ChecksumMismatch { .. })
        ));
        let mut no_end = example_reading();
        no_end[31] = 0x04;
        assert!(matches!(
            decode(&no_end),
            Err(Error::InvalidResponse { .. })
        ));
        let swapped = [example_reading(), example_info()].concat();
        assert!(decode(&swapped).is_err());
        assert!(decode(&hex("FF 02 20 05")).is_err());
    }
}
