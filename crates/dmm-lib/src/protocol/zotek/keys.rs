//! Remote key presses: which keys each layout offers, the code each sends,
//! and the frame that carries it
//! (`docs/research/zotek/reverse-engineered-protocol.md` §8).
//!
//! Nothing here has run on a meter: the codes and the rules for picking
//! them are the vendor app's, and which keys a meter acts on is open.

use super::frame::xor_key;
use super::layout::{Coupling, Function, Layout, Showing, Unit};
use crate::error::{Error, Result};
use crate::measurement::MeasuredValue;
use crate::protocol::{MeterKey, MeterKeys};

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

/// The function keys and ZERO, for the GUI's mode menu and ZERO chip: each
/// function key applies while a function it picks shows, by the
/// `Function` the decoder put in `mode_raw`.
const VOLTS: MeterKey = MeterKey {
    command: "volts",
    label: "V",
    hover: None,
    applies: |m| Function::Volts.shown_in(m),
};
/// Marked on types 3-4 only: on the auto-ranging type 1 a millivolt reading
/// is a range step of V, and the decoder names it V.
const MILLIVOLTS: MeterKey = MeterKey {
    command: "millivolts",
    label: "mV",
    hover: None,
    applies: |m| Function::Millivolts.shown_in(m),
};
const OHMS: MeterKey = MeterKey {
    command: "ohms",
    label: "Ω",
    hover: None,
    applies: |m| Function::Ohms.shown_in(m),
};
const CAPACITANCE: MeterKey = MeterKey {
    command: "capacitance",
    label: "Capacitance",
    hover: None,
    applies: |m| Function::Capacitance.shown_in(m),
};
const HZ: MeterKey = MeterKey {
    command: "hz",
    label: "Hz",
    hover: None,
    applies: |m| Function::Frequency.shown_in(m),
};
const DIODE_CONTINUITY: MeterKey = MeterKey {
    command: "diode_continuity",
    label: "Diode / continuity",
    hover: None,
    applies: |m| Function::Diode.shown_in(m) || Function::Continuity.shown_in(m),
};
const NCV: MeterKey = MeterKey {
    command: "ncv",
    label: "NCV",
    hover: None,
    applies: |m| Function::Ncv.shown_in(m),
};
const CURRENT: MeterKey = MeterKey {
    command: "current",
    label: "Current",
    hover: None,
    applies: |m| {
        [Function::Amps, Function::Milliamps, Function::Microamps]
            .iter()
            .any(|f| f.shown_in(m))
    },
};
const TEMP_UNIT: MeterKey = MeterKey {
    command: "temp_unit",
    label: "°C / °F",
    hover: None,
    applies: |m| Function::Celsius.shown_in(m) || Function::Fahrenheit.shown_in(m),
};
/// AUTO's own display is the `Auto` word with no function lit (spec §6.4);
/// once it finds a signal the reading names that function instead.
const AUTO_FUNCTION: MeterKey = MeterKey {
    command: "auto_function",
    label: "Auto function",
    hover: None,
    applies: |m| Function::None.shown_in(m) && matches!(m.value, MeasuredValue::NoReading(_)),
};
/// ZERO, offered where `code` sends it: in capacitance.
const ZERO: MeterKey = MeterKey {
    command: "zero",
    label: "ZERO",
    hover: None,
    applies: |m| Function::Capacitance.shown_in(m),
};

/// Every function key, in the order the menu lists them.
const EVERY_FUNCTION_KEY: &[MeterKey] = &[
    VOLTS,
    MILLIVOLTS,
    OHMS,
    CAPACITANCE,
    HZ,
    DIODE_CONTINUITY,
    NCV,
    CURRENT,
    TEMP_UNIT,
    AUTO_FUNCTION,
];

/// Type 2's, without the Ω and mV [`TYPE2_KEYS`] leaves out.
const TYPE2_FUNCTION_KEYS: &[MeterKey] = &[
    VOLTS,
    CAPACITANCE,
    HZ,
    DIODE_CONTINUITY,
    NCV,
    CURRENT,
    TEMP_UNIT,
    AUTO_FUNCTION,
];

/// Type 3's, without the ones [`TYPE3_KEYS`] leaves out.
const TYPE3_FUNCTION_KEYS: &[MeterKey] = &[
    VOLTS,
    MILLIVOLTS,
    OHMS,
    DIODE_CONTINUITY,
    CURRENT,
    TEMP_UNIT,
];

/// The function and context keys `layout`'s registry entry lists, all of
/// them among its [`commands`].
pub(super) fn meter_keys(layout: &Layout) -> MeterKeys {
    MeterKeys {
        functions: match layout.type_byte {
            2 => TYPE2_FUNCTION_KEYS,
            3 => TYPE3_FUNCTION_KEYS,
            _ => EVERY_FUNCTION_KEY,
        },
        context: &[ZERO],
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
    use crate::clock::Clock;
    use crate::measurement::Measurement;
    use crate::protocol::Protocol;
    use crate::protocol::zotek::layout::{LAYOUTS, ZT5B, ZT300AB};
    use crate::protocol::zotek::sim::MockZt5b;
    use crate::transport::NullTransport;
    use std::time::Duration;

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

    /// Every key a layout offers is a function key, the ZERO context key,
    /// or one of the buttons the GUI already draws (HOLD, MIN/MAX).
    #[test]
    fn meter_keys_are_the_layouts_commands() {
        for layout in LAYOUTS {
            let keys = meter_keys(layout);
            let mut listed: Vec<&str> = keys
                .functions
                .iter()
                .chain(keys.context)
                .map(|k| k.command)
                .collect();
            let mut offered: Vec<&str> = commands(layout)
                .iter()
                .copied()
                .filter(|c| !matches!(*c, "hold" | "minmax"))
                .collect();
            listed.sort_unstable();
            offered.sort_unstable();
            assert_eq!(listed, offered, "{}", layout.id);
            assert_eq!(keys.context, [ZERO], "{}", layout.id);
        }
        let labels = |layout| -> Vec<&str> {
            meter_keys(layout)
                .functions
                .iter()
                .map(|k| k.label)
                .collect()
        };
        assert_eq!(
            labels(&ZT300AB),
            ["V", "mV", "Ω", "Diode / continuity", "Current", "°C / °F"]
        );
        assert_eq!(
            labels(&ZT5B),
            [
                "V",
                "Capacitance",
                "Hz",
                "Diode / continuity",
                "NCV",
                "Current",
                "°C / °F",
                "Auto function"
            ]
        );
    }

    /// The commands of the function keys `m` shows the function of.
    fn applying(m: &Measurement) -> Vec<&'static str> {
        EVERY_FUNCTION_KEY
            .iter()
            .filter(|k| (k.applies)(m))
            .map(|k| k.command)
            .collect()
    }

    /// On the simulated ZT-5B, each function key's press leads to readings
    /// that mark that key and no other; ZERO is offered in capacitance only.
    #[test]
    fn a_pressed_function_key_is_the_one_that_applies() {
        let cases: &[(&[&str], &str, &str)] = &[
            (&["volts"], "volts", "V"),
            (&["capacitance"], "capacitance", "Capacitance"),
            (&["hz"], "hz", "Hz"),
            (&["diode_continuity"], "diode_continuity", "Diode"),
            (
                &["diode_continuity", "diode_continuity"],
                "diode_continuity",
                "Continuity",
            ),
            (&["ncv"], "ncv", "NCV"),
            (&["current"], "current", "A"),
            (&["temp_unit"], "temp_unit", "°C"),
            (&["temp_unit", "temp_unit"], "temp_unit", "°F"),
            (&["auto_function"], "auto_function", "Auto"),
        ];
        for &(presses, marked, mode) in cases {
            let clock = Clock::manual();
            let mut mock = MockZt5b::new(clock.clone());
            for key in presses {
                // A read first, as the stream does, for the keys whose code
                // follows the display.
                mock.request_measurement(&NullTransport).unwrap();
                mock.send_command(&NullTransport, key).unwrap();
            }
            clock.advance(Duration::from_millis(500));
            let m = mock.request_measurement(&NullTransport).unwrap();
            assert!(m.mode.contains(mode), "{presses:?}: {}", m.mode);
            assert_eq!(applying(&m), [marked], "{presses:?}: {}", m.mode);
            assert_eq!(
                (ZERO.applies)(&m),
                marked == "capacitance",
                "{presses:?}: {}",
                m.mode
            );
        }
    }

    /// AUTO applies to its own word only: once it finds a voltage the
    /// reading names V, and V is the key marked.
    #[test]
    fn auto_function_applies_to_the_auto_word_only() {
        let clock = Clock::manual();
        let mut mock = MockZt5b::new(clock.clone());
        let m = mock.request_measurement(&NullTransport).unwrap();
        assert_eq!(m.mode, "Auto");
        assert_eq!(applying(&m), ["auto_function"]);
        clock.advance(Duration::from_secs(8));
        let m = mock.request_measurement(&NullTransport).unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(applying(&m), ["volts"]);
    }

    /// Coupling and PEAK ride in `mode_raw` beside the function; they do
    /// not change which key applies. An unknown function marks none.
    #[test]
    fn coupling_and_peak_bits_do_not_hide_the_function() {
        let with = |mode_raw| Measurement {
            mode_raw,
            ..Measurement::from_payload(&[])
        };
        for (mode_raw, marked) in [
            (0x11, &["volts"][..]),
            (0x21, &["volts"]),
            (0x51, &["volts"]),
            (0x62, &["millivolts"]),
            (0x13, &["current"]),
            (0x24, &["current"]),
            (0x15, &["current"]),
            (0x0B, &[]),
            (0x0F, &[]),
            (0x00, &[]),
        ] {
            assert_eq!(applying(&with(mode_raw)), marked, "{mode_raw:#04x}");
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
