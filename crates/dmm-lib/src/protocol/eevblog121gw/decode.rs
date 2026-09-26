//! 121GW packet → [`Measurement`]
//! (`docs/research/121gw/reverse-engineered-protocol.md` §5-§9).
//!
//! Every field is read and each one lands in one of three places: the
//! reading, silence for what the spec documents and no reading shows, or a
//! report for what the spec does not cover.

use super::packet::{self, LEN, START};
use super::tables::{self, Range};
use super::{ID, report};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::{check_len, unknown_mode};
use std::borrow::Cow;

/// Decode one 19-byte packet, `F2` first.
pub(super) fn decode(p: &[u8]) -> Result<Measurement> {
    check_len(ID, p, LEN)?;
    if p.len() != LEN {
        return Err(Error::invalid_response(
            format!("{ID} packet is {} bytes, expected {LEN}", p.len()),
            p,
        ));
    }
    if p[0] != START {
        return Err(Error::invalid_response(
            format!("{ID} packet starts {:#04x}, expected {START:#04x}", p[0]),
            p,
        ));
    }
    if !packet::checksum_holds(p) {
        return Err(Error::ChecksumMismatch {
            expected: u16::from(p[LEN - 1]),
            actual: u16::from(packet::checksum(p)),
        });
    }
    if packet::has_reserved_bits(p) {
        report(p, "reserved bits");
    }

    let mode_raw = tables::mode_code(p);
    let range_raw = tables::range_code(p);
    // Byte 15 bits 4-3: 0 none, 1 DC, 2 AC, 3 DC+AC (spec §9).
    let coupling = (p[15] >> 3) & 0x03;
    let negative = p[6] & 0x40 != 0;
    // 18 bits: byte 5 bits 7-6 above bytes 7-8 (spec §6.3).
    let count = (u32::from(p[5] >> 6) << 16) | (u32::from(p[7]) << 8) | u32::from(p[8]);

    let (mode, unit, range_label, value, display_raw) = match tables::lookup(p) {
        Some((mode, range)) => {
            let (value, display_raw) = if p[6] & 0x80 != 0 {
                // OFL (spec §6.3).
                (MeasuredValue::Overload, None)
            } else {
                let (value, digits) = scaled(count, range.decimals, negative);
                (MeasuredValue::Normal(value), Some(digits))
            };
            let name = mode_name(p, mode_raw, mode.name, coupling);
            let unit = unit(p, mode_raw, range);
            // Temperature is named for its unit, °C or °F as the c/F
            // setting picks it (spec §6.4).
            let name = if mode_raw == 5 { unit } else { name };
            (
                Cow::Borrowed(name),
                Cow::Borrowed(unit),
                Cow::Borrowed(range.label),
                value,
                display_raw,
            )
        }
        None => {
            // A code the tables lack: the count, unscaled and unlabelled.
            let (mode, what) = match tables::MODES.get(usize::from(mode_raw)) {
                Some(mode) => (Cow::Borrowed(mode.name), "range code"),
                None => (unknown_mode(mode_raw), "mode code"),
            };
            report(p, what);
            let (value, digits) = scaled(count, 0, negative);
            (
                mode,
                Cow::Borrowed(""),
                Cow::Borrowed(""),
                MeasuredValue::Normal(value),
                Some(digits),
            )
        }
    };

    Ok(Measurement {
        mode,
        mode_raw: u16::from(mode_raw),
        range_raw,
        value,
        unit,
        range_label,
        display_raw,
        flags: flags(p, coupling),
        aux_values: secondary(p, mode_raw),
        ..Measurement::from_payload(p)
    })
}

/// `count` with `decimals` digits after the point: the value, and the
/// digits as the meter shows them.
fn scaled(count: u32, decimals: u8, negative: bool) -> (f64, String) {
    let sign = if negative { "-" } else { "" };
    let divisor = 10u32.pow(u32::from(decimals));
    let digits = if decimals == 0 {
        format!("{sign}{count}")
    } else {
        let width = usize::from(decimals);
        format!(
            "{sign}{}.{:0width$}",
            count / divisor,
            count % divisor,
            width = width
        )
    };
    let magnitude = f64::from(count) / f64::from(divisor);
    (if negative { -magnitude } else { magnitude }, digits)
}

/// The reading's name: the table's, or the one the annunciators make of it.
fn mode_name(p: &[u8], mode_raw: u8, base: &'static str, coupling: u8) -> &'static str {
    // The V position's third MODE step keeps the mode code and lights
    // DC+AC (spec §6.1, §15.4).
    let base = if matches!(mode_raw, 1 | 2) && coupling == 3 {
        "AC+DC V"
    } else {
        base
    };
    // 1 kHz low-pass filter, byte 15 bit 6 (spec §9): REL held in an AC
    // mode (manual p.33).
    if p[15] & 0x40 == 0 {
        return base;
    }
    match tables::lpf_name(mode_raw) {
        Some(name) if coupling != 3 => name,
        _ => {
            report(p, "1 kHz filter annunciator");
            base
        }
    }
}

/// The reading's unit: the range's, or for temperature the one byte 6 names.
fn unit(p: &[u8], mode_raw: u8, range: &Range) -> &'static str {
    if mode_raw != 5 {
        // Byte 6 bits 5-4 outside temperature are left alone: they may
        // mirror the c/F setting (spec §6.4).
        return range.unit;
    }
    // Byte 6 bit 5 °C, bit 4 °F (spec §6.4). Firmware before 1.21 sets
    // neither (spec §1); °C is then assumed, as UEi's app reads that
    // packet (spec §6.2).
    match (p[6] & 0x20 != 0, p[6] & 0x10 != 0) {
        (_, false) => "°C",
        (false, true) => "°F",
        (true, true) => {
            report(p, "temperature unit bits");
            "°C"
        }
    }
}

/// The annunciators a reading carries (spec §9). APO, BT, dBm, the
/// V2 °C/℉ bits 15.7 and 16.7, TEST, byte 17 bits 1-0 and ↙ are documented
/// and shown by no flag, so they stay silent.
fn flags(p: &[u8], coupling: u8) -> StatusFlags {
    let mut flags = StatusFlags {
        auto_range: p[15] & 0x04 != 0,
        low_battery: p[15] & 0x01 != 0,
        rel: p[16] & 0x10 != 0,
        dc: coupling == 1,
        // MEM: non-zero lights MEM (spec §9); logging or playback.
        record: p[17] & 0x30 != 0,
        ..StatusFlags::default()
    };
    // A-HOLD 1, HOLD 2 (spec §9).
    match (p[17] >> 2) & 0x03 {
        0 => {}
        1 | 2 => flags.hold = true,
        _ => report(p, "hold annunciator"),
    }
    let min_max = p[16] & 0x07;
    if p[15] & 0x20 != 0 {
        // "1ms" lit: 1 ms PEAK shows the max peak (manual p.33, p.36);
        // MIN/MAX 2 beside it is the min peak, as firmware 1.02 sends it
        // (spec §15.4).
        match min_max {
            0 | 1 => flags.peak_max = true,
            2 => flags.peak_min = true,
            _ => {
                flags.peak_max = true;
                report(p, "peak MIN/MAX annunciator");
            }
        }
    } else {
        // 1 MAX, 2 MIN, 3 AVG, 4 all three (spec §9).
        match min_max {
            0 => {}
            1 => flags.max = true,
            2 => flags.min = true,
            3 => flags.avg = true,
            4 => {
                flags.min = true;
                flags.max = true;
                flags.avg = true;
            }
            _ => report(p, "MIN/MAX annunciator"),
        }
    }
    flags
}

/// What a secondary-display code stands for (spec §7.1).
enum Secondary {
    /// A sub-value with this label and unit.
    Value(&'static str, &'static str),
    /// A setting or a figure the main reading already carries.
    Silent,
    /// Not in the spec.
    Unknown,
}

const VOLTAGE: &str = "Voltage";
const CURRENT: &str = "Current";

/// The unit a current code of the secondary display is in: its mode's
/// range 0 unit, as UEi's app reads it (spec §7.1), in every VA range.
fn current_unit(code: u8) -> &'static str {
    match code {
        16 | 17 => "µA",
        _ => "mA",
    }
}

fn classify(code: u8, k: bool) -> Secondary {
    match code {
        // The VA modes' voltage operand, in V as UEi's app reads it
        // (spec §7.1); EEVblog's app would add an "m" when the main range
        // is m/µ, so the unit waits on a capture.
        1 | 2 => Secondary::Value(VOLTAGE, "V"),
        6 => Secondary::Value("Frequency", if k { "kHz" } else { "Hz" }),
        16..=21 => Secondary::Value(CURRENT, current_unit(code)),
        100 | 101 => Secondary::Value("Internal temperature", "°C"),
        105 | 106 => Secondary::Value("Internal temperature", "°F"),
        110 => Secondary::Value("Battery", "V"),
        // UEi's unit; EEVblog's app lights V (spec §7.1).
        150 => Secondary::Value("Burden voltage", "mV"),
        180 => Secondary::Value("dBm", "dBm"),
        // The diode test voltage the range label carries, APO, date and
        // time, burden setup, LCD, continuity thresholds, logging interval.
        11
        | 120
        | 121
        | 125
        | 126
        | 130
        | 131
        | 135..=137
        | 140..=142
        | 155
        | 156
        | 160
        | 170..=173
        | 190 => Secondary::Silent,
        _ => Secondary::Unknown,
    }
}

/// The secondary display, bytes 9-12 (spec §7).
///
/// One sub-value, or none while the display is blank (all four bytes zero,
/// spec §15.4) or shows a setting. In the VA modes the display alternates
/// the voltage and the current operand (spec §15.4), so there every frame
/// carries both first, voltage then current, the one it does not show
/// [`MeasuredValue::Absent`] (both while it shows neither): each keeps its
/// own column and trace. Anything else it shows follows them.
fn secondary(p: &[u8], mode_raw: u8) -> Vec<AuxValue> {
    let shown = shown(p);
    if !tables::is_va(mode_raw) {
        return shown.into_iter().collect();
    }
    let absent = |label: &'static str, unit: &'static str| AuxValue {
        label: Cow::Borrowed(label),
        value: MeasuredValue::Absent,
        unit: Cow::Borrowed(unit),
        display_raw: None,
        elapsed_secs: None,
    };
    // The current code µVA's jack sends (16-17), or mVA's and VA's (18-21)
    // (spec §6.2 VA note).
    let current_code = if matches!(mode_raw, 13 | 22) { 16 } else { 18 };
    let mut values = vec![
        absent(VOLTAGE, "V"),
        absent(CURRENT, current_unit(current_code)),
    ];
    match shown {
        Some(v) if v.label == VOLTAGE => values[0] = v,
        Some(v) if v.label == CURRENT => values[1] = v,
        Some(v) => values.push(v),
        None => {}
    }
    values
}

/// What the secondary display shows, if it is a sub-value.
fn shown(p: &[u8]) -> Option<AuxValue> {
    let code = p[9];
    let flags = p[10];
    if p[9..13] == [0; 4] {
        return None;
    }
    let k = flags & 0x20 != 0;
    let hz = flags & 0x10 != 0;
    let point = flags & 0x07;
    if point > 4 {
        report(p, "secondary point");
    }
    if (k || hz) && code != 6 {
        report(p, "secondary annunciators");
    }
    let (label, unit) = match classify(code, k) {
        Secondary::Value(label, unit) => (label, unit),
        Secondary::Silent => return None,
        Secondary::Unknown => {
            report(p, "secondary display mode");
            return None;
        }
    };
    let (value, display_raw) = if flags & 0x80 != 0 {
        // OFL (spec §7.2).
        (MeasuredValue::Overload, None)
    } else {
        let count = u32::from(u16::from_be_bytes([p[11], p[12]]));
        let (value, digits) = scaled(count, point, flags & 0x40 != 0);
        (MeasuredValue::Normal(value), Some(digits))
    };
    Some(AuxValue {
        label: Cow::Borrowed(label),
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        elapsed_secs: None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::packet::tests::{COMMUNITY, EXAMPLES, sealed};
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
            reports[0].starts_with(&format!("121gw: unrecognised {what}: packet [")),
            "{reports:?}"
        );
        m
    }

    /// Example 1 with `set` applied to it, resealed. Its identity bytes are
    /// the spec's invented ones (spec §13).
    fn example1(set: impl FnOnce(&mut [u8; LEN])) -> [u8; LEN] {
        let mut p = EXAMPLES[0];
        set(&mut p);
        sealed(p)
    }

    #[test]
    fn example_1_dc_volts_with_the_internal_temperature() {
        assert_eq!(
            snapshot(&quiet(&EXAMPLES[0])),
            "mode=DC V\n\
             mode_raw=0x01\n\
             range_raw=0x00\n\
             value=Normal(1.2345)\n\
             unit=V\n\
             range_label=5V\n\
             display_raw=Some(\"1.2345\")\n\
             flags=auto_range,dc\n\
             aux=1\n\
             aux1=Internal temperature value=Normal(24.3) unit=°C display_raw=Some(\"24.3\") elapsed_secs=None\n\
             raw_payload=19"
        );
    }

    #[test]
    fn example_2_an_eighteen_bit_frequency() {
        assert_eq!(
            snapshot(&quiet(&EXAMPLES[1])),
            "mode=Hz\n\
             mode_raw=0x06\n\
             range_raw=0x00\n\
             value=Normal(87.654)\n\
             unit=Hz\n\
             range_label=99.999Hz\n\
             display_raw=Some(\"87.654\")\n\
             flags=auto_range\n\
             aux=0\n\
             raw_payload=19"
        );
    }

    #[test]
    fn example_3_resistance_overload() {
        assert_eq!(
            snapshot(&quiet(&EXAMPLES[2])),
            "mode=Ω\n\
             mode_raw=0x09\n\
             range_raw=0x06\n\
             value=Overload\n\
             unit=MΩ\n\
             range_label=50MΩ\n\
             display_raw=None\n\
             flags=auto_range\n\
             aux=0\n\
             raw_payload=19"
        );
    }

    #[test]
    fn example_4_negative_millivolts() {
        assert_eq!(
            snapshot(&quiet(&EXAMPLES[3])),
            "mode=DC mV\n\
             mode_raw=0x03\n\
             range_raw=0x00\n\
             value=Normal(-12.345)\n\
             unit=mV\n\
             range_label=50mV\n\
             display_raw=Some(\"-12.345\")\n\
             flags=auto_range,dc\n\
             aux=0\n\
             raw_payload=19"
        );
    }

    /// Community-captured, spec §15.5: both decode with nothing reported,
    /// byte 14 bit 5 set in both.
    #[test]
    fn the_community_packets_decode_without_a_report() {
        let duty = quiet(&COMMUNITY[0]);
        assert_eq!(duty.mode, "Duty %");
        assert_eq!(normal(&duty.value), Some(0.0));
        assert_eq!(duty.display_raw.as_deref(), Some("0.0"));
        assert_eq!(duty.aux_values.len(), 1);
        assert_eq!(duty.aux_values[0].label, "Internal temperature");
        assert_eq!(duty.aux_values[0].display_raw.as_deref(), Some("27.9"));
        assert_eq!(duty.aux_values[0].unit, "°C");
        assert!(!duty.flags.auto_range);

        let ohms = quiet(&COMMUNITY[1]);
        assert_eq!(ohms.mode, "Ω");
        assert!(matches!(ohms.value, MeasuredValue::Overload));
        assert_eq!(ohms.range_label, "50MΩ");
        assert_eq!(ohms.aux_values[0].display_raw.as_deref(), Some("25.8"));
        assert!(ohms.flags.auto_range);
    }

    /// Constructed from 121gwcli's decoded fields (spec §15.5): 1.638 mV AC
    /// beside 62.97 Hz.
    #[test]
    fn ac_millivolts_with_the_frequency() {
        let p = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x04, 0x00, 0x06, 0x66, 0x06, 0x12, 0x18, 0x99, 0x01,
            0x00, 0x14, 0x40, 0x00, 0x43,
        ];
        let m = quiet(&p);
        assert_eq!(m.mode, "AC mV");
        assert_eq!(m.display_raw.as_deref(), Some("1.638"));
        assert!(!m.flags.dc);
        let hz = &m.aux_values[0];
        assert_eq!((hz.label.as_ref(), hz.unit.as_ref()), ("Frequency", "Hz"));
        assert_eq!(normal(&hz.value), Some(62.97));
        assert_eq!(hz.display_raw.as_deref(), Some("62.97"));
    }

    #[test]
    fn the_k_bit_makes_the_frequency_kilohertz() {
        let p = example1(|p| {
            p[9] = 6;
            p[10] = 0x30 | 3;
            p[11..13].copy_from_slice(&1234u16.to_be_bytes());
        });
        let hz = &quiet(&p).aux_values[0];
        assert_eq!(hz.unit, "kHz");
        assert_eq!(hz.display_raw.as_deref(), Some("1.234"));
    }

    /// DC+AC lit on the AC V code: the V position's third MODE step.
    #[test]
    fn ac_dc_volts() {
        let p = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x02, 0x01, 0x30, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x1C, 0x40, 0x00, 0xB1,
        ];
        let m = quiet(&p);
        assert_eq!(m.mode, "AC+DC V");
        assert_eq!(m.display_raw.as_deref(), Some("12.345"));
        assert_eq!(m.range_label, "50V");
        assert!(!m.flags.dc);
    }

    #[test]
    fn celsius_from_the_unit_bit() {
        let p = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x05, 0x20, 0x00, 0xEB, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x04, 0x40, 0x00, 0x6D,
        ];
        let m = quiet(&p);
        assert_eq!((m.mode.as_ref(), m.unit.as_ref()), ("°C", "°C"));
        assert_eq!(m.display_raw.as_deref(), Some("23.5"));
    }

    #[test]
    fn temperature_units() {
        let temp = |bits: u8| {
            example1(|p| {
                p[5] = 5;
                p[6] = bits;
            })
        };
        let name_and_unit = |m: Measurement| (m.mode.into_owned(), m.unit.into_owned());
        let named = |mode: &str, unit: &str| (mode.to_string(), unit.to_string());
        assert_eq!(name_and_unit(quiet(&temp(0x10))), named("°F", "°F"));
        assert_eq!(name_and_unit(quiet(&temp(0x20))), named("°C", "°C"));
        // Firmware before 1.21 sets neither bit.
        assert_eq!(name_and_unit(quiet(&temp(0x00))), named("°C", "°C"));
        assert_eq!(
            name_and_unit(reported(&temp(0x30), "temperature unit bits")),
            named("°C", "°C")
        );
        // Outside temperature the bits mean nothing to the reading.
        let volts = example1(|p| p[6] = 0x30);
        assert_eq!(quiet(&volts).unit, "V");
    }

    /// The manual's p.37 figure: 0.7308 V AC beside −0.505 dBm.
    #[test]
    fn dbm_beside_ac_volts() {
        let p = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x02, 0x00, 0x1C, 0x8C, 0xB4, 0x43, 0x01, 0xF9, 0x00,
            0x00, 0x14, 0x48, 0x00, 0x26,
        ];
        let m = quiet(&p);
        assert_eq!(m.mode, "AC V");
        assert_eq!(m.display_raw.as_deref(), Some("0.7308"));
        let dbm = &m.aux_values[0];
        assert_eq!((dbm.label.as_ref(), dbm.unit.as_ref()), ("dBm", "dBm"));
        assert_eq!(normal(&dbm.value), Some(-0.505));
        assert_eq!(dbm.display_raw.as_deref(), Some("-0.505"));
    }

    #[test]
    fn dc_volt_amperes() {
        let p = [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x18, 0x03, 0x30, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0C, 0x40, 0x00, 0xB9,
        ];
        let m = quiet(&p);
        assert_eq!(m.mode, "DC VA");
        assert_eq!((m.unit.as_ref(), m.range_label.as_ref()), ("VA", "500VA"));
        assert_eq!(m.display_raw.as_deref(), Some("123.45"));
        assert!(m.flags.dc);
        // A blank secondary display: both operands absent.
        assert_eq!(m.aux_values.len(), 2);
        assert!(
            m.aux_values
                .iter()
                .all(|a| matches!(a.value, MeasuredValue::Absent))
        );
    }

    /// Every (mode, range) at count 12345, against spec §6.2 transcribed
    /// separately from the tables: unit, label, digits.
    #[test]
    fn every_range_scales_as_the_spec_table() {
        /// Unit, label, digits.
        type Row = (&'static str, &'static str, &'static str);
        const EXPECTED: &[(u8, &[Row])] = &[
            (0, &[("V", "600V", "1234.5")]),
            (
                1,
                &[
                    ("V", "5V", "1.2345"),
                    ("V", "50V", "12.345"),
                    ("V", "500V", "123.45"),
                    ("V", "600V", "1234.5"),
                ],
            ),
            (
                2,
                &[
                    ("V", "5V", "1.2345"),
                    ("V", "50V", "12.345"),
                    ("V", "500V", "123.45"),
                    ("V", "600V", "1234.5"),
                ],
            ),
            (3, &[("mV", "50mV", "12.345"), ("mV", "500mV", "123.45")]),
            (4, &[("mV", "50mV", "12.345"), ("mV", "500mV", "123.45")]),
            (5, &[("°C", "", "1234.5")]),
            (
                6,
                &[
                    ("Hz", "99.999Hz", "12.345"),
                    ("Hz", "999.99Hz", "123.45"),
                    ("kHz", "9.9999kHz", "1.2345"),
                    ("kHz", "99.999kHz", "12.345"),
                    ("kHz", "999.99kHz", "123.45"),
                ],
            ),
            (
                7,
                &[
                    ("ms", "", "1.2345"),
                    ("ms", "", "12.345"),
                    ("ms", "", "123.45"),
                ],
            ),
            (8, &[("%", "", "1234.5")]),
            (
                9,
                &[
                    ("Ω", "50Ω", "12.345"),
                    ("Ω", "500Ω", "123.45"),
                    ("kΩ", "5kΩ", "1.2345"),
                    ("kΩ", "50kΩ", "12.345"),
                    ("kΩ", "500kΩ", "123.45"),
                    ("MΩ", "5MΩ", "1.2345"),
                    ("MΩ", "50MΩ", "12.345"),
                ],
            ),
            (10, &[("Ω", "500Ω", "123.45")]),
            (11, &[("V", "3V", "1.2345"), ("V", "15V", "12.345")]),
            (
                12,
                &[
                    ("nF", "10nF", "123.45"),
                    ("nF", "100nF", "1234.5"),
                    ("µF", "1µF", "12.345"),
                    ("µF", "10µF", "123.45"),
                    ("µF", "100µF", "1234.5"),
                    ("µF", "9999µF", "12345"),
                ],
            ),
            (
                13,
                &[
                    ("µVA", "250µVA", "123.45"),
                    ("µVA", "2500µVA", "1234.5"),
                    ("µVA", "2500µVA", "1234.5"),
                    ("µVA", "25000µVA", "12345"),
                ],
            ),
            (
                14,
                &[
                    ("mVA", "25mVA", "12.345"),
                    ("mVA", "250mVA", "123.45"),
                    ("mVA", "250mVA", "123.45"),
                    ("mVA", "2500mVA", "1234.5"),
                ],
            ),
            (
                15,
                &[
                    ("mVA", "2500mVA", "1234.5"),
                    ("mVA", "25000mVA", "12345"),
                    ("VA", "50VA", "12.345"),
                    ("VA", "500VA", "123.45"),
                ],
            ),
            (16, &[("µA", "50µA", "12.345"), ("µA", "500µA", "123.45")]),
            (17, &[("µA", "50µA", "12.345"), ("µA", "500µA", "123.45")]),
            (18, &[("mA", "5mA", "1.2345"), ("mA", "50mA", "12.345")]),
            (19, &[("mA", "5mA", "1.2345"), ("mA", "50mA", "12.345")]),
            (
                20,
                &[
                    ("mA", "500mA", "123.45"),
                    ("A", "5A", "1.2345"),
                    ("A", "10A", "12.345"),
                ],
            ),
            (
                21,
                &[
                    ("mA", "500mA", "123.45"),
                    ("A", "5A", "1.2345"),
                    ("A", "10A", "12.345"),
                ],
            ),
            (
                22,
                &[
                    ("µVA", "250µVA", "123.45"),
                    ("µVA", "2500µVA", "1234.5"),
                    ("µVA", "2500µVA", "1234.5"),
                    ("µVA", "25000µVA", "12345"),
                ],
            ),
            (
                23,
                &[
                    ("mVA", "25mVA", "12.345"),
                    ("mVA", "250mVA", "123.45"),
                    ("mVA", "250mVA", "123.45"),
                    ("mVA", "2500mVA", "1234.5"),
                ],
            ),
            (
                24,
                &[
                    ("mVA", "2500mVA", "1234.5"),
                    ("mVA", "25000mVA", "12345"),
                    ("VA", "50VA", "12.345"),
                    ("VA", "500VA", "123.45"),
                ],
            ),
        ];
        assert_eq!(EXPECTED.len(), 25);
        for &(mode, ranges) in EXPECTED {
            for (range, &(unit, label, digits)) in ranges.iter().enumerate() {
                let p = example1(|p| {
                    p[5] = mode;
                    p[6] = range as u8;
                    p[7..9].copy_from_slice(&12345u16.to_be_bytes());
                    p[9..13].fill(0);
                });
                let m = quiet(&p);
                assert_eq!(
                    (m.unit.as_ref(), m.range_label.as_ref()),
                    (unit, label),
                    "mode {mode} range {range}"
                );
                assert_eq!(
                    m.display_raw.as_deref(),
                    Some(digits),
                    "mode {mode} range {range}"
                );
                let expected: f64 = digits.parse().unwrap();
                assert_eq!(
                    normal(&m.value),
                    Some(expected),
                    "mode {mode} range {range}"
                );
            }
        }
    }

    #[test]
    fn small_counts_keep_their_leading_zeros() {
        let p = example1(|p| p[7..9].copy_from_slice(&5u16.to_be_bytes()));
        let m = quiet(&p);
        assert_eq!(m.display_raw.as_deref(), Some("0.0005"));
        assert_eq!(normal(&m.value), Some(0.0005));
    }

    #[test]
    fn a_mode_past_the_table_reads_as_its_count() {
        let p = example1(|p| p[5] = 25);
        let m = reported(&p, "mode code");
        assert_eq!(m.mode, "Unknown(0x19)");
        assert_eq!((m.unit.as_ref(), m.range_label.as_ref()), ("", ""));
        assert_eq!(normal(&m.value), Some(12345.0));
        assert_eq!(m.display_raw.as_deref(), Some("12345"));
    }

    #[test]
    fn a_range_past_the_table_reads_as_its_count() {
        let p = example1(|p| p[6] = 0x40 | 4);
        let m = reported(&p, "range code");
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.range_raw, 4);
        assert_eq!(normal(&m.value), Some(-12345.0));
        assert_eq!(m.display_raw.as_deref(), Some("-12345"));
    }

    #[test]
    fn each_reserved_bit_is_reported() {
        for (at, mask) in [(5, 0x20), (10, 0x08), (13, 0x20), (14, 0x40), (17, 0x80)] {
            let p = example1(|p| p[at] |= mask);
            reported(&p, "reserved bits");
        }
        // Byte 14 bit 5, which real meters set (spec §15.3 D4).
        quiet(&example1(|p| p[14] |= 0x20));
    }

    #[test]
    fn the_filter_names_the_ac_modes() {
        for (mode, name) in [
            (2, "LPF V"),
            (4, "LPF mV"),
            (16, "LPF µA"),
            (18, "LPF mA"),
            (20, "LPF A"),
        ] {
            let p = example1(|p| {
                p[5] = mode;
                p[15] = 0x40 | 0x10;
            });
            assert_eq!(quiet(&p).mode, name);
        }
        let dc = example1(|p| p[15] |= 0x40);
        assert_eq!(reported(&dc, "1 kHz filter annunciator").mode, "DC V");
        let ac_dc = example1(|p| {
            p[5] = 2;
            p[15] = 0x40 | 0x18;
        });
        assert_eq!(reported(&ac_dc, "1 kHz filter annunciator").mode, "AC+DC V");
    }

    #[test]
    fn flags_follow_the_annunciators() {
        let f = |set: fn(&mut [u8; LEN])| quiet(&example1(set)).flags;
        assert!(!f(|p| p[15] = 0).auto_range);
        assert!(f(|p| p[15] |= 0x01).low_battery);
        assert!(f(|p| p[16] |= 0x10).rel);
        assert!(f(|p| p[15] = 0x08).dc);
        assert!(!f(|p| p[15] = 0x10).dc, "AC");
        assert!(f(|p| p[17] = 0x04).hold, "A-HOLD");
        assert!(f(|p| p[17] = 0x08).hold, "HOLD");
        for mem in [0x10, 0x20, 0x30] {
            let p = example1(|p| p[17] = mem);
            assert!(quiet(&p).flags.record, "MEM {mem:#04x}");
        }
        assert!(f(|p| p[16] |= 0x01).max);
        assert!(f(|p| p[16] |= 0x02).min);
        assert!(f(|p| p[16] |= 0x03).avg);
        let all = f(|p| p[16] |= 0x04);
        assert!(all.min && all.max && all.avg);
        let peak = f(|p| p[15] |= 0x20);
        assert!(peak.peak_max && !peak.max);
        assert!(
            f(|p| {
                p[15] |= 0x20;
                p[16] |= 0x01;
            })
            .peak_max
        );
        let peak_min = f(|p| {
            p[15] |= 0x20;
            p[16] |= 0x02;
        });
        assert!(peak_min.peak_min && !peak_min.peak_max);
    }

    /// Documented annunciators no flag shows: APO, BT, dBm, ↙, TEST, the
    /// V2 °C/℉ bits, byte 17 bits 1-0, and the bar bytes.
    #[test]
    fn documented_annunciators_stay_silent() {
        for set in [
            (|p: &mut [u8; LEN]| p[15] |= 0x02) as fn(&mut [u8; LEN]),
            |p| p[16] |= 0x40,
            |p| p[16] |= 0x08,
            |p| p[16] |= 0x20,
            |p| p[17] |= 0x40,
            |p| p[15] |= 0x80,
            |p| p[16] |= 0x80,
            |p| p[17] |= 0x03,
            |p| {
                p[13] = 0x1F;
                p[14] = 0x3F;
            },
            |p| p[1..5].fill(0x99),
        ] {
            let m = quiet(&example1(set));
            assert_eq!(m.flags, quiet(&EXAMPLES[0]).flags);
        }
    }

    #[test]
    fn undocumented_annunciator_values_are_reported() {
        reported(&example1(|p| p[17] = 0x0C), "hold annunciator");
        for min_max in 5..=7 {
            reported(&example1(|p| p[16] |= min_max), "MIN/MAX annunciator");
        }
        for min_max in 3..=7 {
            let p = example1(|p| {
                p[15] |= 0x20;
                p[16] |= min_max;
            });
            assert!(reported(&p, "peak MIN/MAX annunciator").flags.peak_max);
        }
    }

    /// Example 1's secondary display replaced by `code`, `range` and
    /// `value`.
    fn with_secondary(mode: u8, main_range: u8, code: u8, range: u8, value: u16) -> [u8; LEN] {
        example1(|p| {
            p[5] = mode;
            p[6] = main_range;
            p[9] = code;
            p[10] = range;
            p[11..13].copy_from_slice(&value.to_be_bytes());
        })
    }

    #[test]
    fn every_secondary_value_code_names_its_quantity() {
        for (code, range, label, unit, digits) in [
            (1, 3, "Voltage", "V", "1.234"),
            (2, 3, "Voltage", "V", "1.234"),
            (16, 3, "Current", "µA", "1.234"),
            (17, 3, "Current", "µA", "1.234"),
            (18, 3, "Current", "mA", "1.234"),
            (19, 3, "Current", "mA", "1.234"),
            (20, 3, "Current", "mA", "1.234"),
            (21, 3, "Current", "mA", "1.234"),
            (100, 1, "Internal temperature", "°C", "123.4"),
            (101, 1, "Internal temperature", "°C", "123.4"),
            (105, 1, "Internal temperature", "°F", "123.4"),
            (106, 1, "Internal temperature", "°F", "123.4"),
            (110, 1, "Battery", "V", "123.4"),
            (150, 1, "Burden voltage", "mV", "123.4"),
            (180, 0x43, "dBm", "dBm", "-1.234"),
        ] {
            let m = quiet(&with_secondary(1, 0, code, range, 1234));
            assert_eq!(m.aux_values.len(), 1, "code {code}");
            let aux = &m.aux_values[0];
            assert_eq!(
                (
                    aux.label.as_ref(),
                    aux.unit.as_ref(),
                    aux.display_raw.as_deref()
                ),
                (label, unit, Some(digits)),
                "code {code}"
            );
        }
    }

    #[test]
    fn settings_on_the_secondary_display_are_silent() {
        for code in [
            11, 120, 121, 125, 126, 130, 131, 135, 136, 137, 140, 141, 142, 155, 156, 160, 170,
            171, 172, 173, 190,
        ] {
            let m = quiet(&with_secondary(1, 0, code, 0, 3));
            assert!(m.aux_values.is_empty(), "code {code}");
        }
    }

    #[test]
    fn unknown_secondary_codes_are_reported() {
        for code in [
            0, 3, 4, 5, 7, 8, 9, 10, 12, 13, 14, 15, 22, 23, 24, 25, 99, 107, 200, 255,
        ] {
            let m = reported(&with_secondary(1, 0, code, 0, 1), "secondary display mode");
            assert!(m.aux_values.is_empty(), "code {code}");
        }
    }

    #[test]
    fn a_blank_secondary_display_carries_nothing() {
        let m = quiet(&with_secondary(1, 0, 0, 0, 0));
        assert!(m.aux_values.is_empty());
    }

    #[test]
    fn secondary_overload_sign_and_point() {
        let ofl = quiet(&with_secondary(1, 0, 110, 0x81, 0));
        assert!(matches!(ofl.aux_values[0].value, MeasuredValue::Overload));
        assert_eq!(ofl.aux_values[0].display_raw, None);
        let negative = quiet(&with_secondary(1, 0, 110, 0x42, 5));
        assert_eq!(negative.aux_values[0].display_raw.as_deref(), Some("-0.05"));
        assert_eq!(normal(&negative.aux_values[0].value), Some(-0.05));
        let odd = reported(&with_secondary(1, 0, 110, 5, 5), "secondary point");
        assert_eq!(odd.aux_values[0].display_raw.as_deref(), Some("0.00005"));
        reported(
            &with_secondary(1, 0, 110, 0x11, 5),
            "secondary annunciators",
        );
        reported(
            &with_secondary(1, 0, 110, 0x21, 5),
            "secondary annunciators",
        );
    }

    /// In the VA modes a frame carries both operands, voltage first, the
    /// one the display does not show absent.
    #[test]
    fn va_frames_carry_both_operands() {
        let volts = quiet(&with_secondary(24, 3, 1, 3, 12345));
        let labels: Vec<&str> = volts.aux_values.iter().map(|a| a.label.as_ref()).collect();
        assert_eq!(labels, ["Voltage", "Current"]);
        assert_eq!(volts.aux_values[0].display_raw.as_deref(), Some("12.345"));
        assert!(matches!(volts.aux_values[1].value, MeasuredValue::Absent));
        assert_eq!(
            volts.aux_values[1].unit, "mA",
            "UEi's unit on the 10 A range of DC VA too"
        );

        let amps = quiet(&with_secondary(24, 3, 21, 3, 5000));
        assert!(matches!(amps.aux_values[0].value, MeasuredValue::Absent));
        assert_eq!(amps.aux_values[0].unit, "V");
        assert_eq!(amps.aux_values[1].unit, "mA");
        assert_eq!(amps.aux_values[1].display_raw.as_deref(), Some("5.000"));

        // Every range of VA and mVA is in mA, µVA's in µA.
        for range in 0..4 {
            let m = quiet(&with_secondary(15, range, 20, 2, 5000));
            assert_eq!(m.aux_values[1].unit, "mA", "AC VA range {range}");
        }
        assert_eq!(
            quiet(&with_secondary(15, 1, 1, 2, 5)).aux_values[1].unit,
            "mA"
        );
        assert_eq!(
            quiet(&with_secondary(22, 0, 1, 2, 5)).aux_values[1].unit,
            "µA"
        );
        assert_eq!(
            quiet(&with_secondary(14, 0, 2, 2, 5)).aux_values[1].unit,
            "mA"
        );

        // Anything else on the secondary display follows the two, both
        // absent, so neither column ever moves.
        let temp = quiet(&with_secondary(24, 3, 100, 1, 243));
        let labels: Vec<&str> = temp.aux_values.iter().map(|a| a.label.as_ref()).collect();
        assert_eq!(labels, ["Voltage", "Current", "Internal temperature"]);
        assert!(matches!(temp.aux_values[0].value, MeasuredValue::Absent));
        assert!(matches!(temp.aux_values[1].value, MeasuredValue::Absent));
        assert_eq!(temp.aux_values[2].display_raw.as_deref(), Some("24.3"));

        // A blank display or a setting leaves the two absent.
        for (code, range, value) in [(0, 0, 0), (160, 0, 3)] {
            let m = quiet(&with_secondary(23, 0, code, range, value));
            let labels: Vec<&str> = m.aux_values.iter().map(|a| a.label.as_ref()).collect();
            assert_eq!(labels, ["Voltage", "Current"], "code {code}");
            assert!(
                m.aux_values
                    .iter()
                    .all(|a| matches!(a.value, MeasuredValue::Absent))
            );
        }
    }

    /// No frame carries more sub-values than the profile says.
    #[test]
    fn no_frame_carries_more_sub_values_than_the_profile() {
        let most = super::super::Eevblog121gwProtocol::new()
            .profile
            .max_aux_values;
        for mode in 0..25 {
            for code in 0..=255 {
                let (m, _) = decoded(&with_secondary(mode, 0, code, 1, 5));
                assert!(m.aux_values.len() <= most, "mode {mode} code {code}");
            }
        }
    }

    #[test]
    fn the_packet_is_checked_before_it_is_read() {
        assert!(matches!(
            decode(&EXAMPLES[0][..18]),
            Err(Error::InvalidResponse { .. })
        ));
        let mut long = EXAMPLES[0].to_vec();
        long.push(0);
        assert!(matches!(decode(&long), Err(Error::InvalidResponse { .. })));
        let mut no_start = EXAMPLES[0];
        no_start[0] = 0xF3;
        no_start[18] ^= 0x01;
        assert!(matches!(
            decode(&no_start),
            Err(Error::InvalidResponse { .. })
        ));
        let mut bad = EXAMPLES[0];
        bad[18] ^= 0x01;
        assert!(matches!(
            decode(&bad),
            Err(Error::ChecksumMismatch {
                expected: 0x34,
                actual: 0x35
            })
        ));
    }
}
