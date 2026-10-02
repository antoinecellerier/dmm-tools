//! 15-byte frames → [`Measurement`]
//! (`docs/research/owon/reverse-engineered-protocol.md` §10).
//!
//! Five little-endian 24-bit words: main function/range and reading, the
//! sub-display's, and status (spec §10.2). Functions 0-12 and NCV are the
//! 6-byte frame's (spec §6.2); 14-29 are this frame's own (spec §10.5). The
//! sub-display is one sub-value, read only when the main word says it is
//! there (spec §10.3). What no spec section covers is decoded where it can
//! be and reported: main function-word bits 11 and 13-23 other than the
//! marker, reading bit 19, status bits 11 and 19-23, functions 30-31 and 13
//! on a model without NCV, decimal-point code 5, an NCV level or Motor value
//! off its table, and a sub-display word without its marker. Values the
//! spec documents stay silent.

use super::decode::{
    self, AMP_UNITS, AUTO, DP_5, DP_OL, DP_UL, HOLD, LOW_BATTERY, MAX, MIN, REL, Unit, VOLT_UNITS,
    function, number,
};
use super::frame15::{FRAME_LEN, MARKER};
use super::model::{Function13, Model};
use crate::error::{Error, Result};
use crate::flags::StatusFlags;
use crate::measurement::{AuxValue, MeasuredValue, Measurement};
use crate::protocol::unknown_mode;
use crate::protocol::unrecognised::report_unknown;
use log::debug;
use std::borrow::Cow;

/// The function codes above the 6-byte frame's (spec §10.5, §6.2).
mod code {
    pub(super) const POWER_W: u8 = 14;
    pub(super) const POWER_VA: u8 = 15;
    pub(super) const POWER_FACTOR: u8 = 16;
    pub(super) const LOOP: u8 = 17;
    pub(super) const POWER_AH: u8 = 18;
    pub(super) const TIME: u8 = 19;
    pub(super) const POWER_WH: u8 = 20;
    pub(super) const POWER_V: u8 = 21;
    pub(super) const POWER_A: u8 = 22;
    pub(super) const AC_DC_V: u8 = 23;
    pub(super) const MOTOR: u8 = 24;
    pub(super) const SOLAR: u8 = 25;
    pub(super) const ANGLE: u8 = 26;
    pub(super) const COMPASS: u8 = 27;
    pub(super) const HV_DC: u8 = 28;
    pub(super) const HV_AC: u8 = 29;
}

/// Each prefixed unit of this frame's functions, by prefix code (spec
/// §6.3).
const WATT_UNITS: [&str; 8] = ["pW", "nW", "µW", "mW", "W", "kW", "MW", "GW"];
const VA_UNITS: [&str; 8] = ["pVA", "nVA", "µVA", "mVA", "VA", "kVA", "MVA", "GVA"];
const AH_UNITS: [&str; 8] = ["pAh", "nAh", "µAh", "mAh", "Ah", "kAh", "MAh", "GAh"];
const WH_UNITS: [&str; 8] = ["pWh", "nWh", "µWh", "mWh", "Wh", "kWh", "MWh", "GWh"];
const IRRADIANCE_UNITS: [&str; 8] = [
    "pW/m²", "nW/m²", "µW/m²", "mW/m²", "W/m²", "kW/m²", "MW/m²", "GW/m²",
];
const DEGREE_UNITS: [&str; 8] = ["p°", "n°", "µ°", "m°", "°", "k°", "M°", "G°"];

/// G24 bit 12: the sub-display words count (spec §10.3).
const SUB_PRESENT: u32 = 1 << 12;

/// The main G24's bits 11 and 13-23, and what every VC871 frame holds
/// there: bit 11 and 13-15 clear, 16-23 the marker (spec §10.3, §14.4).
/// The sub-display word's bit 11 is set in every capture: not checked.
const GEAR_HIGH: u32 = 0xFF_E800;
const GEAR_HIGH_SEEN: u32 = 0xF0_0000;

/// Status bits beyond 0-5 (spec §6.6, §10.6). RMR (7) and CosPhi (12) are
/// documented and carry nothing a reading lacks: silent.
const AVG: u32 = 1 << 6;
const LOZ: u32 = 1 << 8;
const LPF: u32 = 1 << 9;
const PEAK: u32 = 1 << 10;
const AC: u32 = 1 << 13;
const DC: u32 = 1 << 14;
const USB: u32 = 1 << 15;
/// "Err_port", sent after a VC871's dial moved (spec §10.6, §14.4).
const ERR_PORT: u32 = 1 << 16;
const INRUSH: u32 = 1 << 17;
const OSC: u32 = 1 << 18;
/// Bit 11, which the app names nothing, and 19-23 (spec §6.6, §10.6).
const UNDOCUMENTED: u32 = 1 << 11 | 0xF8_0000;

/// V24's fields (spec §10.4). Bit 19 is not read by the app and no
/// capture sets it.
const MAGNITUDE: u32 = 0x7_FFFF;
const BIT_19: u32 = 1 << 19;
const SIGN: u32 = 1 << 23;

/// Motor's texts by value (spec §10.4).
const MOTOR_TEXTS: [&str; 4] = ["- - -", "- - -", "1-2-3", "3-2-1"];

/// A function: its mode name, what a sub-display of it is called, and its
/// unit.
struct Function {
    mode: &'static str,
    label: &'static str,
    unit: Unit,
}

/// Function `f` on `model` (spec §6.2, §10.5), or `None` for one no spec
/// section covers on it.
fn function_of(f: u8, model: &Model) -> Option<Function> {
    use Unit::{Bare, Prefixed};
    let named = |mode, label, unit| Some(Function { mode, label, unit });
    match f {
        // The duty sub-display beside Hz.
        function::DUTY => named("Duty %", "Duty", Bare("%")),
        0..=function::NCV => decode::function_of(f, model).map(|(mode, unit)| Function {
            mode,
            label: mode,
            unit,
        }),
        code::POWER_W => named("Power W", "W", Prefixed(&WATT_UNITS)),
        code::POWER_VA => named("Power VA", "VA", Prefixed(&VA_UNITS)),
        code::POWER_FACTOR => named("Power factor", "PF", Bare("")),
        code::LOOP => named("4-20mA", "4-20mA", Bare("%")),
        code::POWER_AH => named("Power Ah", "Ah", Prefixed(&AH_UNITS)),
        // Seconds: the app shows "H:MM:SS" (spec §10.4).
        code::TIME => named("Time", "Time", Bare("s")),
        code::POWER_WH => named("Power Wh", "Wh", Prefixed(&WH_UNITS)),
        code::POWER_V => named("Power V", "V", Prefixed(&VOLT_UNITS)),
        code::POWER_A => named("Power A", "A", Prefixed(&AMP_UNITS)),
        code::AC_DC_V => named("AC+DC V", "AC+DC V", Prefixed(&VOLT_UNITS)),
        code::MOTOR => named("Motor", "Motor", Bare("")),
        code::SOLAR => named("Solar", "Solar", Prefixed(&IRRADIANCE_UNITS)),
        code::ANGLE => named("Angle", "Angle", Prefixed(&DEGREE_UNITS)),
        code::COMPASS => named("Compass", "Compass", Prefixed(&DEGREE_UNITS)),
        code::HV_DC => named("HV DC V", "HV DC V", Prefixed(&VOLT_UNITS)),
        code::HV_AC => named("HV AC V", "HV AC V", Prefixed(&VOLT_UNITS)),
        // 13 on a model without NCV; 30-31 (spec §10.5).
        _ => None,
    }
}

/// The power functions' modes by status bit 13, 14 or 15: AC, DC or USB
/// power; the VC871 sends V and A under each (spec §14.3 D11).
fn power_mode(f: u8, status: u32) -> Option<&'static str> {
    let context = if status & USB != 0 {
        3
    } else if status & DC != 0 {
        2
    } else if status & AC != 0 {
        1
    } else {
        0
    };
    let names: [&str; 4] = match f {
        code::POWER_W => ["Power W", "AC power W", "DC power W", "USB power W"],
        code::POWER_VA => ["Power VA", "AC power VA", "DC power VA", "USB power VA"],
        code::POWER_AH => ["Power Ah", "AC power Ah", "DC power Ah", "USB power Ah"],
        code::POWER_WH => ["Power Wh", "AC power Wh", "DC power Wh", "USB power Wh"],
        code::POWER_V => ["Power V", "AC power V", "DC power V", "USB power V"],
        code::POWER_A => ["Power A", "AC power A", "DC power A", "USB power A"],
        _ => return None,
    };
    Some(names[context])
}

/// One function/range word's fields (spec §10.3).
struct Gear {
    dp: u8,
    prefix: u8,
    function: u8,
}

impl Gear {
    fn new(word: u32) -> Self {
        Gear {
            dp: (word & 0x07) as u8,
            prefix: ((word >> 3) & 0x07) as u8,
            function: ((word >> 6) & 0x1F) as u8,
        }
    }
}

/// A little-endian 24-bit word.
fn u24(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

/// What a reading word decodes to: the value and the digits.
type Reading = (MeasuredValue, Option<String>);

/// The reading `v` of `function` with decimal-point code `dp` (spec §10.4).
fn reading(v: u32, function: u8, dp: u8, model: &Model, report: &dyn Fn(&'static str)) -> Reading {
    let negative = v & SIGN != 0;
    let magnitude = v & MAGNITUDE;
    if v & BIT_19 != 0 {
        report("reading bit 19");
    }
    match (v >> 20) & 0x07 {
        // The app's full scale, not the magnitude, is the value under OL;
        // the VC871 sends neither (spec §14.4).
        1 => return (MeasuredValue::Overload, None),
        2 => return (MeasuredValue::NoReading("UL"), None),
        3 => return (MeasuredValue::NoReading("HI"), None),
        4 => return (MeasuredValue::NoReading("LO"), None),
        // A number the app's log can filter out as large error data.
        5 => debug!("owon: V24 status 5 (large error data) in {v:06X}"),
        6 | 7 => return (MeasuredValue::NoReading("----"), None),
        _ => {}
    }
    if function == function::NCV && model.function13 == Function13::Ncv {
        if dp == 0 && !negative && magnitude <= 4 {
            // Spec §6.7's EF and dashes.
            return (MeasuredValue::NcvLevel(magnitude as u8), None);
        }
        report("NCV level");
    }
    if function == code::MOTOR {
        match MOTOR_TEXTS.get(magnitude as usize) {
            Some(&text) if dp == 0 && !negative => return (MeasuredValue::NoReading(text), None),
            _ => report("Motor value"),
        }
    }
    match dp {
        DP_OL => (MeasuredValue::Overload, None),
        DP_UL => (MeasuredValue::NoReading("UL"), None),
        _ => {
            // As OWON's app (spec §6.4).
            if dp == DP_5 {
                report("decimal-point code 5");
            }
            let (value, digits) = number(negative, magnitude, dp);
            (MeasuredValue::Normal(value), Some(digits))
        }
    }
}

/// The unit of `function` with `prefix`: the no-prefix functions leave it
/// out (spec §6.3, §10.5).
fn unit_of(function: Option<&Function>, prefix: u8) -> &'static str {
    match function.map(|f| &f.unit) {
        Some(Unit::Prefixed(units)) => units[usize::from(prefix)],
        Some(Unit::Bare(unit)) => unit,
        Some(Unit::Unknown) | None => "",
    }
}

/// Decode one frame as `model` reads it.
pub(super) fn decode(p: &[u8], model: &Model) -> Result<Measurement> {
    if p.len() != FRAME_LEN {
        return Err(Error::invalid_response(
            format!(
                "{} payload is {} bytes, expected {FRAME_LEN}",
                model.id,
                p.len()
            ),
            p,
        ));
    }
    let report =
        |what: &'static str| report_unknown(model.id, what, format_args!("frame {p:02X?}"));

    // Main function/range word (spec §10.3).
    let word = u24(&p[0..3]);
    let gear = Gear::new(word);
    let main = function_of(gear.function, model);
    if main.is_none() {
        report("function code");
    }
    if word & GEAR_HIGH != GEAR_HIGH_SEEN {
        report("function-word bits");
    }

    // Status word (spec §10.2, §10.6).
    let status = u24(&p[12..15]);
    if status & UNDOCUMENTED != 0 {
        report("status bits");
    }

    let (value, display_raw) = if main.is_none() && gear.function > code::HV_AC {
        // The app's lookup throws: no reading (spec §10.5).
        (MeasuredValue::NoReading("----"), None)
    } else {
        reading(u24(&p[3..6]), gear.function, gear.dp, model, &report)
    };
    let mode = mode(main.as_ref(), gear.function, &value, status);
    let unit = unit_of(main.as_ref(), gear.prefix);

    // The sub-display, only when the main word says so; otherwise its
    // bytes hold stale words (spec §10.3, §14.4).
    let aux_values = if word & SUB_PRESENT != 0 {
        if p[8] != MARKER {
            report("sub-display word");
        }
        vec![sub_value(p, gear.function, status, model, &report)]
    } else {
        Vec::new()
    };

    let flag = |bit: u16| status & u32::from(bit) != 0;
    Ok(Measurement {
        mode,
        mode_raw: u16::from(gear.function),
        range_raw: (word & 0x3F) as u8,
        value,
        unit: Cow::Borrowed(unit),
        display_raw,
        flags: StatusFlags {
            hold: flag(HOLD),
            rel: flag(REL),
            auto_range: flag(AUTO),
            low_battery: flag(LOW_BATTERY),
            min: flag(MIN),
            max: flag(MAX),
            avg: status & AVG != 0,
            loz: status & LOZ != 0,
            lead_error: status & ERR_PORT != 0,
            dc: matches!(gear.function, function::DC_V | function::DC_A | code::HV_DC),
            ..StatusFlags::default()
        },
        aux_values,
        ..Measurement::from_payload(p)
    })
}

/// The main reading's mode: the function's, with what the status word adds
/// (spec §10.5, §10.6): the scope and inrush modes, AC, DC or USB power,
/// Motor's rotation, and low-pass and peak.
fn mode(
    main: Option<&Function>,
    function: u8,
    value: &MeasuredValue,
    status: u32,
) -> Cow<'static, str> {
    let Some(main) = main else {
        return unknown_mode(function);
    };
    let base = if status & OSC != 0 {
        "Scope"
    } else if status & INRUSH != 0 {
        "Inrush"
    } else if let Some(power) = power_mode(function, status) {
        power
    } else if function == code::MOTOR {
        match value {
            MeasuredValue::NoReading("1-2-3") => "Motor 1-2-3",
            MeasuredValue::NoReading("3-2-1") => "Motor 3-2-1",
            _ => main.mode,
        }
    } else {
        main.mode
    };
    match (status & LPF != 0, status & PEAK != 0) {
        (false, false) => Cow::Borrowed(base),
        (true, false) => Cow::Owned(format!("LPF {base}")),
        (false, true) => Cow::Owned(format!("Peak {base}")),
        (true, true) => Cow::Owned(format!("LPF Peak {base}")),
    }
}

/// The sub-display (spec §10.2-10.5, §10.7). On the VC871 under REL, MAX or
/// MIN it shows the main display's function, with its own prefix and
/// point: the reference under REL and the live reading under MAX/MIN (spec
/// §9.4).
fn sub_value(
    p: &[u8],
    main_function: u8,
    status: u32,
    model: &Model,
    report: &dyn Fn(&'static str),
) -> AuxValue {
    let gear = Gear::new(u24(&p[6..9]));
    let rel = status & u32::from(REL) != 0;
    let follows = model.sub_follows_main && (rel || status & u32::from(MIN | MAX) != 0);
    let function = if follows {
        main_function
    } else {
        gear.function
    };
    let sub = function_of(function, model);
    let label: Cow<'static, str> = match &sub {
        _ if follows && rel => Cow::Borrowed("Reference"),
        _ if follows => Cow::Borrowed("Live"),
        // AC+DC V with its DC or AC part (spec §10.5).
        Some(_) if main_function == code::AC_DC_V && function == function::DC_V => {
            Cow::Borrowed("DC")
        }
        Some(_) if main_function == code::AC_DC_V && function == function::AC_V => {
            Cow::Borrowed("AC")
        }
        Some(f) => Cow::Borrowed(f.label),
        None => {
            report("sub-display function code");
            unknown_mode(function)
        }
    };
    let (value, display_raw) = if sub.is_none() && function > code::HV_AC {
        (MeasuredValue::NoReading("----"), None)
    } else {
        reading(u24(&p[9..12]), function, gear.dp, model, report)
    };
    AuxValue {
        label,
        value,
        unit: Cow::Borrowed(unit_of(sub.as_ref(), gear.prefix)),
        display_raw,
        elapsed_secs: None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::frame15::tests::VECTORS;
    use super::super::model::{CMS101, OW67B, OW69B, VC871, VC915, VC925PV};
    use super::*;
    use crate::protocol::capture_reports;

    /// Decode `p` as `model`, with no report allowed.
    fn quiet(p: &[u8], model: &Model) -> Measurement {
        let (m, reports) = capture_reports(|| decode(p, model));
        assert!(reports.is_empty(), "{p:02X?}: {reports:?}");
        m.unwrap()
    }

    /// Decode `p` as `model`, and what it reported.
    fn reported(p: &[u8], model: &Model) -> (Measurement, Vec<String>) {
        let (m, reports) = capture_reports(|| decode(p, model));
        (m.unwrap(), reports)
    }

    fn value(v: &MeasuredValue) -> String {
        format!("{v:?}")
    }

    /// A frame built per spec §10.2: main `gear` and `reading`, sub `sub`
    /// and `sub_reading`, and `status`.
    pub(crate) fn frame(
        gear: u32,
        reading: u32,
        sub: u32,
        sub_reading: u32,
        status: u32,
    ) -> [u8; 15] {
        let mut p = [0u8; 15];
        for (at, word) in [
            (0, gear),
            (3, reading),
            (6, sub),
            (9, sub_reading),
            (12, status),
        ] {
            p[at..at + 3].copy_from_slice(&word.to_le_bytes()[..3]);
        }
        p
    }

    /// A G24 word with the marker: `function`, `prefix`, `dp`, and bit 12
    /// if `sub`.
    pub(crate) fn g24(function: u8, prefix: u8, dp: u8, sub: bool) -> u32 {
        0xF0_0000
            | u32::from(sub) << 12
            | u32::from(function) << 6
            | u32::from(prefix) << 3
            | u32::from(dp)
    }

    /// Spec §14.5's VC871 frames: each main reading, its sub-value where
    /// bit 12 is set, and nothing reported.
    #[test]
    fn the_vc871_vectors_decode_to_their_readings() {
        type Sub = Option<(&'static str, &'static str, &'static str)>;
        let cases: [(&str, &str, &str, Sub); 28] = [
            ("Capacitance", "MF", "NoReading(\"UL\")", None),
            ("DC V", "V", "-1.0054", None),
            ("DC V", "mV", "OL", None),
            ("°C", "°C", "24.9", Some(("°F", "°F", "76.8"))),
            ("AC power W", "W", "0.0", Some(("VA", "VA", "0.0"))),
            ("USB power Wh", "Wh", "0", Some(("Time", "s", "0"))),
            ("DC V", "V", "0.5409", None),
            ("DC V", "mV", "540.74", None),
            ("AC V", "mV", "57.1", Some(("Hz", "Hz", "0.00"))),
            ("Ω", "kΩ", "369.7", None),
            ("Diode", "V", "0.5507", None),
            ("Capacitance", "nF", "0.0694", None),
            ("Hz", "Hz", "50.003", Some(("Duty", "%", "44.71"))),
            ("Duty %", "%", "52.42", Some(("Hz", "Hz", "50.838"))),
            ("°C", "°C", "22.6", Some(("°F", "°F", "72.8"))),
            ("°F", "°F", "78.3", Some(("°C", "°C", "25.7"))),
            ("DC A", "µA", "-1592.6", None),
            ("AC A", "µA", "167.2", Some(("Hz", "Hz", "0.00"))),
            ("DC A", "mA", "-1.680", None),
            ("AC A", "mA", "0.98", Some(("Hz", "Hz", "0.00"))),
            ("DC A", "A", "0.2893", None),
            ("AC A", "A", "0.078", Some(("Hz", "Hz", "0.00"))),
            ("AC power W", "W", "473.8", Some(("VA", "VA", "0.0"))),
            ("Power factor", "", "-0.001", Some(("Hz", "Hz", "0.0"))),
            ("USB power V", "V", "0.00", Some(("A", "A", "0.29"))),
            ("USB power Ah", "mAh", "0", Some(("Time", "s", "0"))),
            ("AC power V", "V", "0.4", Some(("AC A", "A", "0.0"))),
            ("DC A", "A", "0.000", None),
        ];
        let shown = |m: &MeasuredValue, digits: Option<&str>| match m {
            MeasuredValue::Normal(_) => digits.unwrap().to_string(),
            MeasuredValue::Overload => "OL".to_string(),
            other => value(other),
        };
        for (p, (mode, unit, reading, sub)) in VECTORS.iter().zip(cases) {
            let m = quiet(p, &VC871);
            assert_eq!((m.mode.as_ref(), m.unit.as_ref()), (mode, unit), "{p:02X?}");
            assert_eq!(
                shown(&m.value, m.display_raw.as_deref()),
                reading,
                "{p:02X?}"
            );
            let subs: Vec<(String, String, String)> = m
                .aux_values
                .iter()
                .map(|a| {
                    (
                        a.label.to_string(),
                        a.unit.to_string(),
                        shown(&a.value, a.display_raw.as_deref()),
                    )
                })
                .collect();
            let want: Vec<(String, String, String)> = sub
                .iter()
                .map(|(l, u, v)| (l.to_string(), u.to_string(), v.to_string()))
                .collect();
            assert_eq!(subs, want, "{p:02X?}");
            assert_eq!(m.main_label, None);
            assert_eq!(m.raw_payload, p);
        }
    }

    /// The flags of the vectors: HOLD with AUTO; none in the power and
    /// temperature modes; AUTO and the lead error after a dial move (spec
    /// §14.4).
    #[test]
    fn the_status_word_sets_the_flags() {
        let held = quiet(&VECTORS[1], &VC871);
        assert!(held.flags.hold && held.flags.auto_range && held.flags.dc);
        let power = quiet(&VECTORS[4], &VC871);
        assert_eq!(power.flags, StatusFlags::default());
        let moved = quiet(&VECTORS[27], &VC871);
        assert!(moved.flags.lead_error && moved.flags.auto_range && moved.flags.dc);
        // Bits 1, 3-6 and 8: REL, low battery, MIN, MAX, AVG, LoZ.
        let p = frame(g24(1, 4, 2, false), 100, 0, 0, 0x017A);
        let f = quiet(&p, &OW67B).flags;
        assert!(f.rel && f.low_battery && f.min && f.max && f.avg && f.loz);
        assert!(!f.hold && !f.auto_range && !f.dc && !f.lead_error);
    }

    /// The main words of spec §14.5's two OL rows, with the rest of a frame
    /// built around them: V24 status 1 is OL whatever the magnitude.
    #[test]
    fn v24_status_1_is_overload() {
        for (main, unit) in [
            ([0x2F, 0x01, 0xF0, 0xDB, 0x2A, 0x10], "kΩ"),
            ([0x4F, 0x01, 0xF0, 0xFF, 0xFF, 0x17], "nF"),
        ] {
            let mut p = frame(0, 0, 0, 0, 0x04);
            p[..6].copy_from_slice(&main);
            let m = quiet(&p, &VC871);
            assert_eq!(value(&m.value), "Overload");
            assert_eq!(m.unit, unit);
            assert_eq!(m.display_raw, None);
        }
    }

    /// V24 statuses 2-7 (spec §10.4): words, a number, or no reading.
    #[test]
    fn every_v24_status_decodes_silently() {
        let cases = [
            (2, "NoReading(\"UL\")"),
            (3, "NoReading(\"HI\")"),
            (4, "NoReading(\"LO\")"),
            (5, "Normal(1.234)"),
            (6, "NoReading(\"----\")"),
            (7, "NoReading(\"----\")"),
        ];
        for (code, want) in cases {
            let p = frame(g24(0, 4, 3, false), code << 20 | 1234, 0, 0, 0);
            let m = quiet(&p, &VC915);
            assert_eq!(value(&m.value), want, "{code}");
        }
        // Decimal codes 6 and 7: UL and OL.
        let ul = quiet(&frame(g24(0, 4, 6, false), 5, 0, 0, 0), &VC915);
        assert_eq!(value(&ul.value), "NoReading(\"UL\")");
        let ol = quiet(&frame(g24(0, 4, 7, false), 5, 0, 0, 0), &VC915);
        assert_eq!(value(&ol.value), "Overload");
    }

    /// Functions 17 and 23-29, built per spec §10.5, with their units and
    /// prefixes.
    #[test]
    fn the_15_byte_functions_decode() {
        let cases: [(u8, u8, &str, &str, &str); 8] = [
            (17, 4, "4-20mA", "%", "12.5"),
            (23, 4, "AC+DC V", "V", "12.5"),
            (25, 4, "Solar", "W/m²", "12.5"),
            (25, 5, "Solar", "kW/m²", "12.5"),
            (26, 4, "Angle", "°", "12.5"),
            (27, 4, "Compass", "°", "12.5"),
            (28, 5, "HV DC V", "kV", "12.5"),
            (29, 4, "HV AC V", "V", "12.5"),
        ];
        for (function, prefix, mode, unit, digits) in cases {
            let p = frame(g24(function, prefix, 1, false), 125, 0, 0, 0);
            let m = quiet(&p, &VC925PV);
            assert_eq!(
                (m.mode.as_ref(), m.unit.as_ref()),
                (mode, unit),
                "{function}"
            );
            assert_eq!(m.display_raw.as_deref(), Some(digits), "{function}");
            assert_eq!(m.mode_raw, u16::from(function));
            assert_eq!(m.flags.dc, function == 28, "{function}");
        }
        // The no-prefix functions leave it out, whatever it is (spec §10.5).
        for function in [7, 8, 9, 12, 16, 17, 19, 24] {
            let p = frame(g24(function, 3, 0, false), 5, 0, 0, 0);
            let (m, _) = reported(&p, &VC915);
            assert!(!m.unit.starts_with('m'), "{function}: {}", m.unit);
        }
    }

    /// AC+DC V with a DC or AC V sub-display: "DC" and "AC" (spec §10.5).
    #[test]
    fn ac_dc_names_its_parts() {
        for (sub, label) in [(0, "DC"), (1, "AC"), (6, "Hz")] {
            let p = frame(g24(23, 4, 2, true), 1234, g24(sub, 4, 2, false), 99, 0x04);
            let m = quiet(&p, &OW69B);
            let aux = &m.aux_values[0];
            assert_eq!(aux.label, label);
            assert_eq!(aux.display_raw.as_deref(), Some("0.99"));
        }
    }

    /// Motor's texts and rotation (spec §10.4); another value is reported
    /// and read as a number.
    #[test]
    fn motor_shows_its_texts() {
        let cases = [
            (0, "Motor", "- - -"),
            (1, "Motor", "- - -"),
            (2, "Motor 1-2-3", "1-2-3"),
            (3, "Motor 3-2-1", "3-2-1"),
        ];
        for (v, mode, text) in cases {
            let p = frame(g24(24, 4, 0, true), v, g24(1, 4, 1, false), 2301, 0);
            let m = quiet(&p, &VC915);
            assert_eq!(m.mode, mode);
            assert_eq!(value(&m.value), format!("NoReading({text:?})"));
            assert_eq!(m.aux_values[0].label, "AC V");
            assert_eq!(m.aux_values[0].display_raw.as_deref(), Some("230.1"));
        }
        let (m, reports) = reported(&frame(g24(24, 4, 0, false), 4, 0, 0, 0), &VC915);
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(reports[0].contains("Motor value"), "{reports:?}");
        assert_eq!(value(&m.value), "Normal(4.0)");
    }

    /// NCV on a CMS: levels 0-4 (spec §6.7); function 13 elsewhere is
    /// reported.
    #[test]
    fn ncv_is_read_on_the_cms_only() {
        for level in 0..=4 {
            let p = frame(g24(13, 4, 0, false), level, 0, 0, 0);
            let m = quiet(&p, &CMS101);
            assert_eq!(m.mode, "NCV");
            assert_eq!(value(&m.value), format!("NcvLevel({level})"));
            let (m, reports) = reported(&p, &VC871);
            assert_eq!(reports.len(), 1, "{reports:?}");
            assert!(reports[0].contains("function code"), "{reports:?}");
            assert_eq!(m.mode, "Unknown(0x0d)");
        }
    }

    /// The status word's mode context (spec §10.6): the scope, inrush,
    /// low-pass and peak; RMR and CosPhi are silent.
    #[test]
    fn status_bits_add_to_the_mode() {
        let at = |status: u32, model: &Model| {
            quiet(&frame(g24(1, 4, 1, false), 1, 0, 0, status), model)
                .mode
                .into_owned()
        };
        assert_eq!(at(OSC, &CMS101), "Scope");
        assert_eq!(at(INRUSH, &CMS101), "Inrush");
        assert_eq!(at(LPF, &OW69B), "LPF AC V");
        assert_eq!(at(PEAK, &OW69B), "Peak AC V");
        assert_eq!(at(1 << 7 | 1 << 12, &OW69B), "AC V");
        // AC, DC and USB name nothing outside the power functions.
        assert_eq!(at(AC | DC | USB, &OW69B), "AC V");
        let w = |status| quiet(&frame(g24(14, 4, 1, false), 1, 0, 0, status), &VC871).mode;
        assert_eq!(w(0), "Power W");
        assert_eq!(w(DC), "DC power W");
    }

    /// What no spec section covers is reported, once per frame.
    #[test]
    fn undocumented_values_are_reported() {
        let cases: [(&str, [u8; 15]); 10] = [
            ("status bits", frame(g24(0, 4, 1, false), 1, 0, 0, 1 << 11)),
            ("status bits", frame(g24(0, 4, 1, false), 1, 0, 0, 1 << 19)),
            (
                "function-word bits",
                frame(g24(0, 4, 1, false) | 1 << 11, 1, 0, 0, 0),
            ),
            (
                "function-word bits",
                frame(g24(0, 4, 1, false) | 1 << 13, 1, 0, 0, 0),
            ),
            (
                "function-word bits",
                frame(g24(0, 4, 1, false) ^ 1 << 20, 1, 0, 0, 0),
            ),
            (
                "reading bit 19",
                frame(g24(0, 4, 1, false), 1 << 19, 0, 0, 0),
            ),
            (
                "reading bit 19",
                frame(g24(0, 4, 1, true), 1, g24(6, 4, 1, false), 1 << 19, 0),
            ),
            ("function code", frame(g24(30, 4, 1, false), 1, 0, 0, 0)),
            (
                "decimal-point code 5",
                frame(g24(0, 4, 5, false), 1, 0, 0, 0),
            ),
            ("sub-display word", {
                let mut p = frame(g24(0, 4, 1, true), 1, g24(6, 4, 1, false), 1, 0);
                p[8] = 0x00;
                p
            }),
        ];
        for (what, p) in cases {
            let (_, reports) = reported(&p, &VC915);
            assert_eq!(reports.len(), 1, "{what}: {reports:?}");
            assert!(reports[0].contains(what), "{what}: {reports:?}");
            assert!(reports[0].starts_with("vc915:"), "{reports:?}");
        }
        // Functions 30 and 31 give no reading.
        for function in [30, 31] {
            let (m, _) = reported(
                &frame(
                    g24(function, 4, 1, true),
                    1,
                    g24(function, 4, 1, false),
                    1,
                    0,
                ),
                &VC915,
            );
            assert_eq!(value(&m.value), "NoReading(\"----\")");
            assert_eq!(value(&m.aux_values[0].value), "NoReading(\"----\")");
        }
    }

    /// Under REL, MAX or MIN the VC871's sub-display takes the main
    /// function, with its own prefix and point (spec §10.7); the OW67B's
    /// keeps its own.
    #[test]
    fn the_vc871s_sub_display_follows_the_main_under_rel_and_minmax() {
        // DC V, main 1.234 V; sub word Hz, prefix m, dp 1, 5000.
        for (status, label) in [(0x02 | 0x04, "Reference"), (0x20, "Live"), (0x10, "Live")] {
            let p = frame(g24(0, 4, 3, true), 1234, g24(6, 3, 1, false), 5000, status);
            let vc871 = quiet(&p, &VC871);
            let aux = &vc871.aux_values[0];
            assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), (label, "mV"));
            assert_eq!(aux.display_raw.as_deref(), Some("500.0"));
            let ow67 = quiet(&p, &OW67B);
            let aux = &ow67.aux_values[0];
            assert_eq!((aux.label.as_ref(), aux.unit.as_ref()), ("Hz", "mHz"));
        }
        // Without them, the VC871's sub-display is its own.
        let p = frame(g24(0, 4, 3, true), 1234, g24(6, 3, 1, false), 5000, 0x04);
        assert_eq!(quiet(&p, &VC871).aux_values[0].label, "Hz");
    }

    /// Bit 12 clear: the sub bytes are ignored, whatever they hold (spec
    /// §14.4).
    #[test]
    fn a_clear_bit_12_drops_the_sub_display() {
        let mut p = frame(g24(0, 4, 3, false), 1234, 0, 0, 0);
        p[6..12].copy_from_slice(&[0xFF; 6]);
        assert!(quiet(&p, &VC871).aux_values.is_empty());
    }

    /// All 19 magnitude bits are read (spec §10.4). OWON's app shows no
    /// reading for a count longer than the model's digits (spec §10.4);
    /// this tool shows the number, an open item in the family's
    /// verification.md.
    #[test]
    fn a_count_above_65535_keeps_bits_16_to_18() {
        let p = frame(g24(0, 4, 4, false), 0x7_FFFF, 0, 0, 0);
        let m = quiet(&p, &VC871);
        assert_eq!(m.display_raw.as_deref(), Some("52.4287"));
    }

    #[test]
    fn a_frame_of_another_length_is_an_error() {
        for len in [0, 6, 14, 16] {
            let p = vec![0xF0; len];
            assert!(
                matches!(decode(&p, &VC871), Err(Error::InvalidResponse { .. })),
                "{len}"
            );
        }
    }
}
