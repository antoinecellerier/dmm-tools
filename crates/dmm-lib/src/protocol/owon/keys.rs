//! Remote key presses: the keys each model offers, and the two bytes each
//! sends (`docs/research/owon/reverse-engineered-protocol.md` §7.1, §7.3).
//!
//! A press is `[key code, 01]` short or `[key code, 00]` long, written to
//! FFF3 with no reply (spec §7.1). Each model offers the keys OWON's app
//! lists for it, plus the long presses its manual gives those keys. A long
//! press of the key that also carries ᛒ switches Bluetooth on the meter
//! (spec §9.1), so it is never offered: code 4 on the B series and the
//! CM2100B, code 5 on the OW16/OW18.

use super::decode::function;
use crate::protocol::{MeterKey, MeterKeys};

/// A short press.
const SHORT: u8 = 0x01;
/// A long press: the PC software's button held over 2 s (spec §7.1).
const LONG: u8 = 0x00;

/// One remote key: the command that sends it, its code and the press.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) command: &'static str,
    pub(super) code: u8,
    pub(super) press: u8,
}

const fn key(command: &'static str, code: u8, press: u8) -> Key {
    Key {
        command,
        code,
        press,
    }
}

/// The two bytes a press sends (spec §7.1).
pub(super) fn frame(key: &Key) -> [u8; 2] {
    [key.code, key.press]
}

/// `keys`' commands, in order, for a profile.
const fn commands<const N: usize>(keys: &[Key; N]) -> [&'static str; N] {
    let mut commands = [""; N];
    let mut i = 0;
    while i < N {
        commands[i] = keys[i].command;
        i += 1;
    }
    commands
}

/// SELECT steps through a position's functions; RANGE held over 2 s goes
/// back to autorange (B35-UM p.14/9; OW18-UM p.14/9; B33-UM p.13/8).
const SELECT: Key = key("select", 0x01, SHORT);
const RANGE: Key = key("range", 0x02, SHORT);
const AUTO: Key = key("auto", 0x02, LONG);
/// HOLD, and held over 2 s the backlight (and the OW18's flashlight,
/// OW18-UM p.11/6) (B35-UM p.12/7, p.14/9; CM2100-UM p.9/6).
const HOLD: Key = key("hold", 0x03, SHORT);
const LIGHT: Key = key("light", 0x03, LONG);

/// The OW16 and OW18: Select, Range, Hold/Light and one △/Hz/Duty key,
/// code 5 (spec §7.3). That key is REL in the DC functions and Hz/Duty in
/// AC V, AC A and Hz% (OW18-UM p.22/17), so it is offered under both
/// names. One community client sends code 4 for Hz/Duty on an OW18B; both
/// of OWON's programs send 5 (spec §14.3 D6), and 4 is not sent.
pub(super) static OW: [Key; 7] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD,
    LIGHT,
    key("rel", 0x05, SHORT),
    key("hz_duty", 0x05, SHORT),
];
pub(super) static OW_COMMANDS: [&str; 7] = commands(&OW);
/// Hz/Duty△/ᛒ (spec §9.1).
pub(super) const OW_BLUETOOTH_KEY: u8 = 0x05;

/// The B33: Select, Range, Hold, Rel and Hz/Duty (spec §7.3). Its
/// backlight is a key of its own, ☀, which no program sends (spec §7.3),
/// so there is no `light`; nor MIN/MAX, which the meter lacks.
pub(super) static B33: [Key; 6] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD,
    key("rel", 0x04, SHORT),
    key("hz_duty", 0x05, SHORT),
];
pub(super) static B33_COMMANDS: [&str; 6] = commands(&B33);

/// The B35T+ and B41T+: the B33's keys, Hold/Light and Max/Min, which
/// held over 2 s leaves MIN/MAX (B35-UM p.14/9, p.23/18).
pub(super) static B35_B41: [Key; 9] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD,
    LIGHT,
    key("rel", 0x04, SHORT),
    key("hz_duty", 0x05, SHORT),
    key("minmax", 0x06, SHORT),
    key("exit_minmax", 0x06, LONG),
];
pub(super) static B35_B41_COMMANDS: [&str; 9] = commands(&B35_B41);
/// △/ᛒ on the B series (spec §9.1).
pub(super) const B_BLUETOOTH_KEY: u8 = 0x04;

/// The CM2100B: Select, Hold/Light and Zero, no Range (spec §7.3). Its
/// SELECT held 2 s toggles VFC (CM2100-UM p.9/6), which OWON's app does
/// not send, so it is not offered.
pub(super) static CM2100: [Key; 4] = [SELECT, HOLD, LIGHT, key("zero", 0x04, SHORT)];
pub(super) static CM2100_COMMANDS: [&str; 4] = commands(&CM2100);
/// ZERO/ᛒ (spec §9.1).
pub(super) const CM2100_BLUETOOTH_KEY: u8 = 0x04;

/// Whether the reading's function is one of `codes`, by the function code
/// the decoder put in `mode_raw`.
fn function_in(m: &crate::measurement::Measurement, codes: &[u8]) -> bool {
    u8::try_from(m.mode_raw).is_ok_and(|code| codes.contains(&code))
}

/// Hz/Duty applies in AC V, AC A and Hz% (B35-UM p.21/16; OW18-UM
/// p.22/17; B33-UM p.13/8).
fn hz_duty_applies(m: &crate::measurement::Measurement) -> bool {
    function_in(
        m,
        &[function::AC_V, function::AC_A, function::HZ, function::DUTY],
    )
}

/// The B series' Hz/Duty key.
static B_HZ_DUTY: MeterKey = MeterKey {
    command: "hz_duty",
    label: "Hz/Duty",
    hover: None,
    applies: hz_duty_applies,
};

/// The OW's Hz/Duty is its △/ᛒ key: briefly, as held it switches
/// Bluetooth.
static OW_HZ_DUTY: MeterKey = MeterKey {
    command: "hz_duty",
    label: "Hz/Duty",
    hover: Some("Press the meter's Hz/Duty△/ᛒ key briefly"),
    applies: hz_duty_applies,
};

/// ZERO zeroes DC A, and is relative for capacitance and voltage
/// (CM2100-UM p.9/6); briefly, as held it switches Bluetooth.
static CM2100_ZERO: MeterKey = MeterKey {
    command: "zero",
    label: "ZERO",
    hover: Some("Press the meter's ZERO/ᛒ key briefly"),
    applies: |m| {
        function_in(
            m,
            &[
                function::DC_A,
                function::CAPACITANCE,
                function::DC_V,
                function::AC_V,
            ],
        )
    },
};

pub(super) const OW_METER_KEYS: MeterKeys = MeterKeys {
    functions: &[],
    context: std::slice::from_ref(&OW_HZ_DUTY),
};

pub(super) const B_METER_KEYS: MeterKeys = MeterKeys {
    functions: &[],
    context: std::slice::from_ref(&B_HZ_DUTY),
};

pub(super) const CM2100_METER_KEYS: MeterKeys = MeterKeys {
    functions: &[],
    context: std::slice::from_ref(&CM2100_ZERO),
};

#[cfg(test)]
mod tests {
    use super::super::model::MODELS;
    use super::*;

    /// A long press of the key that carries ᛒ switches Bluetooth (spec
    /// §9.1); no model may send one.
    #[test]
    fn no_long_press_of_the_bluetooth_key_is_ever_sent() {
        for model in MODELS {
            for key in model.keys {
                assert!(
                    !(key.code == model.bluetooth_key && key.press == LONG),
                    "{} sends {:02X?}",
                    model.id,
                    frame(key)
                );
            }
        }
        assert_eq!(OW_BLUETOOTH_KEY, 0x05);
        assert_eq!(B_BLUETOOTH_KEY, 0x04);
        assert_eq!(CM2100_BLUETOOTH_KEY, 0x04);
    }

    /// Each model's commands, as spec §7.3 and the manuals give them.
    #[test]
    fn each_model_offers_its_own_keys() {
        let ow = ["select", "range", "auto", "hold", "light", "rel", "hz_duty"];
        let b33 = ["select", "range", "auto", "hold", "rel", "hz_duty"];
        let b35 = [
            "select",
            "range",
            "auto",
            "hold",
            "light",
            "rel",
            "hz_duty",
            "minmax",
            "exit_minmax",
        ];
        let cm2100 = ["select", "hold", "light", "zero"];
        let expected: [(&str, &[&str]); 6] = [
            ("ow18b", &ow),
            ("ow18e", &ow),
            ("b33", &b33),
            ("b35t+", &b35),
            ("b41t+", &b35),
            ("cm2100b", &cm2100),
        ];
        for (model, (id, commands)) in MODELS.iter().zip(expected) {
            assert_eq!(model.id, id);
            assert_eq!(model.commands, commands, "{id}");
            let names: Vec<&str> = model.keys.iter().map(|k| k.command).collect();
            assert_eq!(names, commands, "{id}");
        }
    }

    /// The codes: REL is 5 on the OW, 4 on the B series (spec §7.3).
    #[test]
    fn rel_and_hz_duty_share_the_ows_key() {
        let code = |keys: &[Key], command| {
            keys.iter()
                .find(|k| k.command == command)
                .map(frame)
                .unwrap()
        };
        assert_eq!(code(&OW, "rel"), [0x05, 0x01]);
        assert_eq!(code(&OW, "hz_duty"), [0x05, 0x01]);
        assert_eq!(code(&B33, "rel"), [0x04, 0x01]);
        assert_eq!(code(&B35_B41, "hz_duty"), [0x05, 0x01]);
        assert_eq!(code(&B35_B41, "exit_minmax"), [0x06, 0x00]);
        assert_eq!(code(&CM2100, "zero"), [0x04, 0x01]);
        assert_eq!(code(&CM2100, "light"), [0x03, 0x00]);
    }

    #[test]
    fn context_keys_follow_the_function() {
        use crate::measurement::Measurement;
        let at = |function: u8| Measurement {
            mode_raw: u16::from(function),
            ..Measurement::from_payload(&[])
        };
        let hz_duty = OW_METER_KEYS.context[0];
        for f in [1, 3, 6, 7] {
            assert!((hz_duty.applies)(&at(f)), "{f}");
            assert!((B_METER_KEYS.context[0].applies)(&at(f)), "{f}");
        }
        for f in [0, 2, 4, 5, 8, 13] {
            assert!(!(hz_duty.applies)(&at(f)), "{f}");
        }
        let zero = CM2100_METER_KEYS.context[0];
        for f in [0, 1, 2, 5] {
            assert!((zero.applies)(&at(f)), "{f}");
        }
        for f in [3, 4, 6, 7, 10, 11, 13] {
            assert!(!(zero.applies)(&at(f)), "{f}");
        }
    }
}
