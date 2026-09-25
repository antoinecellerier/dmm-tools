//! Remote key presses: which keys each layout offers, the code each sends,
//! and the frame that carries it
//! (`docs/research/zotek/reverse-engineered-protocol.md` §8).
//!
//! Nothing here has run on a meter: the codes and the rules for picking
//! them are the vendor app's, and which keys a meter acts on is open.

use super::frame::xor_key;
use super::layout::{Coupling, Function, Layout, Showing, Unit};
use crate::error::{Error, Result};

/// Every key, in the order `dmm-cli command` lists them (spec §8.2). `minmax`
/// and `hold` keep the names the other families give those keys; AUTO picks
/// the function, not the range, so it is `auto_function`, not `auto`.
const EVERY_KEY: &[&str] = &[
    "hold",
    "minmax",
    "auto_function",
    "volts",
    "millivolts",
    "ohms",
    "capacitance",
    "hz",
    "diode_continuity",
    "ncv",
    "current",
    "temp_unit",
    "zero",
];

/// Types 1 and 2 have no MAX or MIN bit (spec §7.2, §7.3), so MAX/MIN would
/// change nothing a reading shows; a V05B ignores it (§11).
const TYPE1_KEYS: &[&str] = &[
    "hold",
    "auto_function",
    "volts",
    "millivolts",
    "ohms",
    "capacitance",
    "hz",
    "diode_continuity",
    "ncv",
    "current",
    "temp_unit",
    "zero",
];

/// Type 2's: no MAX/MIN, as type 1, and no Ω or mV: the ZT-5B matches Ω
/// from the input jack and has no mV function, so it has no key they could
/// stand for (spec §8.2).
const TYPE2_KEYS: &[&str] = &[
    "hold",
    "auto_function",
    "volts",
    "capacitance",
    "hz",
    "diode_continuity",
    "ncv",
    "current",
    "temp_unit",
    "zero",
];

/// Type 3's: the app greys out capacitance, NCV, Hz and HOLD and locks AUTO
/// on (spec §8.2), so those five are not offered.
const TYPE3_KEYS: &[&str] = &[
    "minmax",
    "volts",
    "millivolts",
    "ohms",
    "diode_continuity",
    "current",
    "temp_unit",
    "zero",
];

/// The keys `layout`'s registry entry offers.
pub(super) fn commands(layout: &Layout) -> &'static [&'static str] {
    match layout.type_byte {
        1 => TYPE1_KEYS,
        2 => TYPE2_KEYS,
        3 => TYPE3_KEYS,
        _ => EVERY_KEY,
    }
}

/// Whether `command`'s code depends on what the meter shows.
pub(super) fn follows_display(command: &str) -> bool {
    matches!(command, "current" | "temp_unit" | "zero")
}

/// What the driver says when ZERO is asked for outside capacitance.
const ZERO_NEEDS_CAPACITANCE: &str = "ZERO works in capacitance only";

/// The code `command` sends from a meter of `type_byte` while it shows
/// `showing` (spec §8.2). The caller has checked `command` is one of the
/// layout's [`commands`].
pub(super) fn code(type_byte: u8, command: &str, showing: Showing) -> Result<u8> {
    Ok(match command {
        "hold" => 0xB4,
        "minmax" => 0xD1,
        "auto_function" => 0xB8,
        "volts" => 0xC4,
        "millivolts" => 0xC6,
        "ohms" => 0xBE,
        "capacitance" => 0xB0,
        "hz" => 0xB3,
        "diode_continuity" => 0xB1,
        "ncv" => 0xB2,
        "current" => current(type_byte, showing),
        "temp_unit" => temp_unit(type_byte, showing),
        "zero" if showing.unit == Some(Unit::Farad) => 0xB5,
        // The app sends nothing outside capacitance, so neither does the
        // driver: it refuses, and the user switches to capacitance.
        "zero" => return Err(Error::CommandRejected(ZERO_NEEDS_CAPACITANCE.into())),
        _ => return Err(Error::UnsupportedCommand(command.to_string())),
    })
}

/// A type-4 meter's current key names the current shown; the others always
/// send `C9` (spec §8.2, V1's rule).
fn current(type_byte: u8, showing: Showing) -> u8 {
    match (type_byte, showing.function, showing.coupling) {
        (4, Function::Amps, Coupling::Ac) => 0xCB,
        (4, Function::Amps, Coupling::Dc) => 0xC8,
        (4, Function::Milliamps, Coupling::Dc) => 0xCA,
        _ => 0xC9,
    }
}

/// Types 1-2 send `B7` while °C shows, else `B6`; types 3-4 always `B6`
/// (spec §8.2, V1's rule; V2 also sends `B7` from a type 3 showing °C).
fn temp_unit(type_byte: u8, showing: Showing) -> u8 {
    match type_byte {
        1 | 2 if showing.unit == Some(Unit::Celsius) => 0xB7,
        _ => 0xB6,
    }
}

/// The key-press command byte (spec §8.2).
const KEY_PRESS: u8 = 0x03;

/// The frame that presses `key`, as it goes on air: `AB CD 03 <key> 00 00
/// 00 00`, the big-endian 16-bit sum of those eight bytes, all scrambled
/// from key byte 0 (spec §8.1).
pub(super) fn frame(key: u8) -> [u8; 10] {
    let mut frame = [0xAB, 0xCD, KEY_PRESS, key, 0, 0, 0, 0, 0, 0];
    let sum: u16 = frame[..8].iter().map(|&b| u16::from(b)).sum();
    frame[8..].copy_from_slice(&sum.to_be_bytes());
    xor_key(&mut frame);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::zotek::layout::{LAYOUTS, ZT300AB};

    fn showing(unit: Option<Unit>, function: Function, coupling: Coupling) -> Showing {
        Showing {
            unit,
            function,
            coupling,
        }
    }

    fn farad() -> Showing {
        showing(Some(Unit::Farad), Function::Capacitance, Coupling::None)
    }

    /// Spec §9's AUTO frame, plain and on air.
    #[test]
    fn the_auto_frame_is_spec_9s() {
        let raw = frame(0xB8);
        assert_eq!(
            raw,
            [0xEA, 0xEC, 0x70, 0xED, 0xA2, 0xC1, 0x32, 0x71, 0x64, 0x99]
        );
        let mut plain = raw;
        xor_key(&mut plain);
        assert_eq!(
            plain,
            [0xAB, 0xCD, 0x03, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x02, 0x33]
        );
    }

    /// Spec §8.2's on-air form holds for every code: the sum is
    /// `0x017B + key`.
    #[test]
    fn every_key_frame_has_spec_8_2s_on_air_form() {
        for key in 0..=u8::MAX {
            let [hi, lo] = (0x017B + u16::from(key)).to_be_bytes();
            assert_eq!(
                frame(key),
                [
                    0xEA,
                    0xEC,
                    0x70,
                    key ^ 0x55,
                    0xA2,
                    0xC1,
                    0x32,
                    0x71,
                    hi ^ 0x66,
                    lo ^ 0xAA
                ],
                "{key:02X}"
            );
        }
    }

    /// The codes that follow nothing on the display (spec §8.2).
    #[test]
    fn fixed_codes() {
        for (command, key) in [
            ("hold", 0xB4),
            ("minmax", 0xD1),
            ("auto_function", 0xB8),
            ("volts", 0xC4),
            ("millivolts", 0xC6),
            ("ohms", 0xBE),
            ("capacitance", 0xB0),
            ("hz", 0xB3),
            ("diode_continuity", 0xB1),
            ("ncv", 0xB2),
        ] {
            assert!(!follows_display(command), "{command}");
            for type_byte in 1..=4 {
                assert_eq!(code(type_byte, command, Showing::default()).unwrap(), key);
            }
        }
    }

    /// Type 3 goes without five keys, types 1-2 without MAX/MIN, type 2
    /// also without Ω and mV, and type 4 offers every key.
    #[test]
    fn each_layout_offers_its_keys() {
        for layout in LAYOUTS {
            let keys = commands(layout);
            for key in keys {
                assert!(EVERY_KEY.contains(key), "{key}");
                assert!(code(layout.type_byte, key, farad()).is_ok(), "{key}");
            }
            let missing: Vec<_> = EVERY_KEY.iter().filter(|k| !keys.contains(k)).collect();
            let expected: &[&&str] = match layout.type_byte {
                t if t == ZT300AB.type_byte => {
                    &[&"hold", &"auto_function", &"capacitance", &"hz", &"ncv"]
                }
                1 => &[&"minmax"],
                2 => &[&"minmax", &"millivolts", &"ohms"],
                _ => &[],
            };
            assert_eq!(missing, expected, "{}", layout.id);
        }
    }

    /// Type 4 names the current shown; types 1-3 send `C9` whatever shows.
    #[test]
    fn current_follows_the_shown_current_on_type_4_only() {
        let amps = |function, coupling| showing(Some(Unit::Amp), function, coupling);
        let cases = [
            (amps(Function::Amps, Coupling::Ac), 0xCB),
            (amps(Function::Amps, Coupling::Dc), 0xC8),
            (amps(Function::Milliamps, Coupling::Ac), 0xC9),
            (amps(Function::Milliamps, Coupling::Dc), 0xCA),
            (amps(Function::Microamps, Coupling::Dc), 0xC9),
            (amps(Function::Amps, Coupling::None), 0xC9),
            (
                showing(Some(Unit::Volt), Function::Volts, Coupling::Dc),
                0xC9,
            ),
            (Showing::default(), 0xC9),
        ];
        assert!(follows_display("current"));
        for (shown, key) in cases {
            assert_eq!(code(4, "current", shown).unwrap(), key, "{shown:?}");
            for type_byte in 1..=3 {
                assert_eq!(code(type_byte, "current", shown).unwrap(), 0xC9);
            }
        }
    }

    #[test]
    fn temp_unit_follows_celsius_on_types_1_and_2_only() {
        let celsius = showing(Some(Unit::Celsius), Function::Celsius, Coupling::None);
        let fahrenheit = showing(Some(Unit::Fahrenheit), Function::Fahrenheit, Coupling::None);
        assert!(follows_display("temp_unit"));
        for type_byte in [1, 2] {
            assert_eq!(code(type_byte, "temp_unit", celsius).unwrap(), 0xB7);
            assert_eq!(code(type_byte, "temp_unit", fahrenheit).unwrap(), 0xB6);
            assert_eq!(
                code(type_byte, "temp_unit", Showing::default()).unwrap(),
                0xB6
            );
        }
        for type_byte in [3, 4] {
            assert_eq!(code(type_byte, "temp_unit", celsius).unwrap(), 0xB6);
            assert_eq!(code(type_byte, "temp_unit", fahrenheit).unwrap(), 0xB6);
        }
    }

    /// ZERO goes out only while F shows; otherwise it is refused, not
    /// unknown, so the key stays advertised.
    #[test]
    fn zero_only_in_capacitance() {
        assert!(follows_display("zero"));
        for type_byte in 1..=4 {
            assert_eq!(code(type_byte, "zero", farad()).unwrap(), 0xB5);
            for shown in [
                Showing::default(),
                showing(Some(Unit::Ohm), Function::Ohms, Coupling::None),
                showing(Some(Unit::Fahrenheit), Function::Fahrenheit, Coupling::None),
            ] {
                let err = code(type_byte, "zero", shown).unwrap_err();
                assert!(
                    matches!(&err, Error::CommandRejected(m) if m == ZERO_NEEDS_CAPACITANCE),
                    "{err:?}"
                );
            }
        }
    }

    #[test]
    fn an_unknown_name_is_unsupported() {
        for command in ["auto", "range", "rel", ""] {
            assert!(matches!(
                code(1, command, farad()),
                Err(Error::UnsupportedCommand(_))
            ));
        }
    }
}
