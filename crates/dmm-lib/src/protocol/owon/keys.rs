//! Remote key presses: the keys each model offers, and the two bytes each
//! sends (`docs/research/owon/reverse-engineered-protocol.md` §7.1, §7.3,
//! §10.8).
//!
//! A press is `[key code, 01]` short or `[key code, 00]` long, written to
//! FFF3 with no reply (spec §7.1, §10.8). Each model offers the keys OWON's
//! app lists for it, plus the long presses its manual gives those keys. A
//! long press of the key that also carries ᛒ switches Bluetooth on the meter
//! (spec §9.1), so it is never offered: code 4 on the B series and the
//! CM2100B, code 5 on the OW16/OW18. The 15-byte meters' BLE key has no
//! known code (spec §10.8), so they take taps, and the long presses their
//! own manuals give; one of those, REL held, is code 4, which goes out only
//! once FFF2 has named the model ([`Key::needs_confirmed`]). The app's
//! Hold/Light long press, `09 01`, is never sent: what code 9 does is
//! inferred from a label (spec §10.8).

use super::decode::function;
use super::decode15::code;
use crate::protocol::{ButtonLabel, MeterKey, MeterKeys, command};

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
    /// Sent only once FFF2 has named the entry's model: a long press of
    /// code 4 switches Bluetooth on the B series and the CM2100B (spec
    /// §7.1, §9.1), which an entry opened without a model code may be.
    pub(super) needs_confirmed: bool,
}

const fn key(command: &'static str, code: u8, press: u8) -> Key {
    Key {
        command,
        code,
        press,
        needs_confirmed: false,
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
const SELECT: Key = key(command::SELECT, 0x01, SHORT);
const RANGE: Key = key(command::RANGE, 0x02, SHORT);
const AUTO: Key = key(command::AUTO, 0x02, LONG);
/// HOLD, and held over 2 s the backlight (and the OW18's flashlight,
/// OW18-UM p.11/6) (B35-UM p.12/7, p.14/9; CM2100-UM p.9/6).
const HOLD: Key = key(command::HOLD, 0x03, SHORT);
const LIGHT: Key = key(command::LIGHT, 0x03, LONG);

/// The OW16 and OW18: Select, Range, Hold/Light and one △/Hz/Duty key,
/// code 5 (spec §7.3). That key is REL in the DC functions and Hz/Duty in
/// AC V, AC A and Hz% (OW18-UM p.22/17), so it is offered under both
/// names. Both of OWON's programs send 5 for Hz/Duty on an OW18B (spec
/// §14.3 D6), so code 4 is not sent.
pub(super) static OW: [Key; 7] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD,
    LIGHT,
    key(command::REL, 0x05, SHORT),
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
    key(command::REL, 0x04, SHORT),
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
    key(command::REL, 0x04, SHORT),
    key("hz_duty", 0x05, SHORT),
    key(command::MINMAX, 0x06, SHORT),
    key(command::EXIT_MINMAX, 0x06, LONG),
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

/// The 15-byte meters' key codes beyond §7.1's (spec §10.8).
const REL: Key = key(command::REL, 0x04, SHORT);
const HZ_DUTY: Key = key("hz_duty", 0x05, SHORT);
const MINMAX: Key = key(command::MINMAX, 0x06, SHORT);
const LPF: Key = key("lpf", 0x07, SHORT);
const PEAK: Key = key(command::PEAK, 0x08, SHORT);
const CURRENT_LOOP: Key = key("current_loop", 0x0A, SHORT);
const DISPLAY: Key = key("display", 0x0C, SHORT);
const INRUSH: Key = key("inrush", 0x0D, SHORT);
const COMPARE: Key = key("compare", 0x0F, SHORT);
const AC_DC: Key = key("ac_dc", 0x10, SHORT);
const MOTOR: Key = key("motor", 0x11, SHORT);
/// The 15-byte meters' Hold/Light and Hold, as a tap (spec §10.8).
const HOLD_TAP: Key = key(command::HOLD, 0x03, SHORT);

/// REL held (about 1 s) leaves REL on the Voltcraft meters (VC871-UM p.89;
/// VC891-UM p.79; VC915-UM p.86; VC925-UM p.87).
const EXIT_REL: Key = Key {
    command: command::EXIT_REL,
    code: 0x04,
    press: LONG,
    needs_confirmed: true,
};

/// The CMS101 and CMS061: the app's `_cms101Keys` (spec §10.8). HOLD held
/// is "DCA to zero" (CMS101-UM p.13/8), not the light the app's label
/// names, so it goes as a tap only.
pub(super) static CMS: [Key; 7] = [SELECT, RANGE, HOLD_TAP, REL, HZ_DUTY, MINMAX, INRUSH];
pub(super) static CMS_COMMANDS: [&str; 7] = commands(&CMS);

/// The OW65B: the app's `_Ow65Keys` (spec §10.8); its manual gives no long
/// press of these keys (OW65-UM p.13/8-14/9).
pub(super) static OW65: [Key; 5] = [SELECT, RANGE, HOLD_TAP, REL, MINMAX];
pub(super) static OW65_COMMANDS: [&str; 5] = commands(&OW65);

/// The OW67B: the app's key list for it (spec §10.8), taps only.
pub(super) static OW67: [Key; 8] = [
    SELECT,
    RANGE,
    HOLD_TAP,
    REL,
    MINMAX,
    PEAK,
    CURRENT_LOOP,
    DISPLAY,
];
pub(super) static OW67_COMMANDS: [&str; 8] = commands(&OW67);

/// The VC871: the OW67B's keys, with RANGE held about 1 s back to auto
/// range (VC871-UM p.88) and REL held leaving REL.
pub(super) static VC871: [Key; 10] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD_TAP,
    REL,
    EXIT_REL,
    MINMAX,
    PEAK,
    CURRENT_LOOP,
    DISPLAY,
];
pub(super) static VC871_COMMANDS: [&str; 10] = commands(&VC871);

/// The OW69B: the app's key list for it (spec §10.8), taps only.
pub(super) static OW69: [Key; 9] = [
    SELECT,
    RANGE,
    HOLD_TAP,
    REL,
    HZ_DUTY,
    MINMAX,
    LPF,
    PEAK,
    CURRENT_LOOP,
];
pub(super) static OW69_COMMANDS: [&str; 9] = commands(&OW69);

/// The VC891: the OW69B's keys, with RANGE held about 1 s back to auto
/// range (VC891-UM p.79) and REL held leaving REL.
pub(super) static VC891: [Key; 11] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD_TAP,
    REL,
    EXIT_REL,
    HZ_DUTY,
    MINMAX,
    LPF,
    PEAK,
    CURRENT_LOOP,
];
pub(super) static VC891_COMMANDS: [&str; 11] = commands(&VC891);

/// The VC915: the app's `_c91Keys` (spec §10.8), with RANGE held about 1 s
/// back to auto range (VC915-UM p.86) and REL held leaving REL.
pub(super) static VC915: [Key; 13] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD_TAP,
    REL,
    EXIT_REL,
    MINMAX,
    LPF,
    COMPARE,
    AC_DC,
    MOTOR,
    CURRENT_LOOP,
    DISPLAY,
];
pub(super) static VC915_COMMANDS: [&str; 13] = commands(&VC915);

/// The VC925 PV: the app's `_c92Keys` (spec §10.8), with RANGE held back to
/// auto range, MAX/MIN held leaving MAX/MIN and REL held leaving REL
/// (VC925-UM p.86-87).
pub(super) static VC925: [Key; 11] = [
    SELECT,
    RANGE,
    AUTO,
    HOLD_TAP,
    REL,
    EXIT_REL,
    MINMAX,
    key(command::EXIT_MINMAX, 0x06, LONG),
    COMPARE,
    CURRENT_LOOP,
    DISPLAY,
];
pub(super) static VC925_COMMANDS: [&str; 11] = commands(&VC925);

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
const B_HZ_DUTY: MeterKey = MeterKey {
    command: "hz_duty",
    label: "Hz/Duty",
    in_words: false,
    hover: None,
    applies: hz_duty_applies,
};

/// The OW's Hz/Duty is its △/ᛒ key: briefly, as held it switches
/// Bluetooth.
static OW_HZ_DUTY: MeterKey = MeterKey {
    command: "hz_duty",
    label: "Hz/Duty",
    in_words: false,
    hover: Some("Press the meter's Hz/Duty△/ᛒ key briefly"),
    applies: hz_duty_applies,
};

/// ZERO zeroes DC A, and is relative for capacitance and voltage
/// (CM2100-UM p.9/6); briefly, as held it switches Bluetooth.
static CM2100_ZERO: MeterKey = MeterKey {
    command: "zero",
    label: "ZERO",
    in_words: false,
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

/// REL is one key with Hz/Duty, code 5: a step of Hz/Duty in AC V, AC A and Hz%,
/// REL where it has it elsewhere (OW18-UM p.22/17). Held, ☀/H is the backlight,
/// and on an OW18 the flashlight too (OW18-UM p.11/6; OW16-UM p.11/6 has no
/// flashlight, and one entry may be either).
const OW_BUTTON_LABELS: &[ButtonLabel] = &[
    ButtonLabel {
        command: command::REL,
        label: None,
        hover: Some("REL; in AC and Hz, steps Hz/Duty"),
    },
    ButtonLabel {
        command: command::LIGHT,
        label: None,
        hover: Some("Toggle the backlight and an OW18's flashlight"),
    },
];

pub(super) const OW_METER_KEYS: MeterKeys = MeterKeys {
    context: std::slice::from_ref(&OW_HZ_DUTY),
    button_labels: OW_BUTTON_LABELS,
    ..MeterKeys::NONE
};

pub(super) const B_METER_KEYS: MeterKeys = MeterKeys {
    context: &[B_HZ_DUTY],
    ..MeterKeys::NONE
};

pub(super) const CM2100_METER_KEYS: MeterKeys = MeterKeys {
    context: std::slice::from_ref(&CM2100_ZERO),
    ..MeterKeys::NONE
};

/// 4-20mA is "only in the DC mA range" (VC871-UM p.71; VC891-UM p.65; VC915-UM
/// p.68, p.79; VC925-UM p.71, p.79), and offered in the function it lands on
/// too, so a second click can try to leave. Its unit is % (spec §10.5).
const CURRENT_LOOP_KEY: MeterKey = MeterKey {
    command: "current_loop",
    label: "4-20mA",
    in_words: false,
    hover: Some("Read a 4-20 mA loop as a percentage"),
    applies: |m| {
        (function_in(m, &[function::DC_A]) && m.unit == "mA") || function_in(m, &[code::LOOP])
    },
};

/// The OW67B's and OW69B's manuals say "When measuring mA current", with no DC
/// in it (OW67-UM p.26/21; OW69-UM p.25/20).
const OW_CURRENT_LOOP: MeterKey = MeterKey {
    applies: |m| {
        (function_in(m, &[function::DC_A, function::AC_A]) && m.unit == "mA")
            || function_in(m, &[code::LOOP])
    },
    ..CURRENT_LOOP_KEY
};

/// Display, F1, changes the readings in AC, DC and USB power (VC871-UM p.85-88;
/// OW67-UM p.27/22 on), whose main reading moves among these functions (spec
/// §14 D11).
const POWER_DISPLAY: MeterKey = MeterKey {
    command: "display",
    label: "Display",
    in_words: false,
    hover: Some("Change what the displays show"),
    applies: |m| {
        function_in(
            m,
            &[
                code::POWER_W,
                code::POWER_VA,
                code::POWER_FACTOR,
                code::POWER_AH,
                code::TIME,
                code::POWER_WH,
                code::POWER_V,
                code::POWER_A,
            ],
        )
    },
};

/// The VC915's DISPLAY swaps Hz/% and voltage in AC V (VC915-UM p.72, p.74),
/// and is offered in the Hz and duty it lands on.
const VC915_DISPLAY: MeterKey = MeterKey {
    hover: Some("Swap Hz/% and voltage"),
    applies: |m| function_in(m, &[function::AC_V, function::HZ, function::DUTY]),
    ..POWER_DISPLAY
};

/// The VC925 PV's symbol table gives Display no limit (VC925-UM p.71), and
/// AC V and PV mode both have it (VC925-UM p.69, p.75).
const VC925_DISPLAY: MeterKey = MeterKey {
    hover: Some("Swap the main and second displays"),
    applies: |_| true,
    ..POWER_DISPLAY
};

/// The OW69B's Lo, F1, is for AC and AC+DC, on the V and mV positions, and F1
/// again leaves it; its manual gives no cutoff (OW69-UM p.28/23-29/24). A status
/// bit carries it (spec §6.6).
const OW69_LO: MeterKey = MeterKey {
    command: "lpf",
    label: "Lo",
    in_words: false,
    hover: Some("Switch the low-pass filter on or off"),
    applies: |m| function_in(m, &[function::AC_V, code::AC_DC_V]),
};

/// The VC891's Lo is "only in ACV mode", a 1 kHz filter from the 6 V range up
/// (VC891-UM p.65, p.93).
const VC891_LO: MeterKey = MeterKey {
    hover: Some("1 kHz low-pass filter"),
    applies: |m| function_in(m, &[function::AC_V]) && m.unit == "V",
    ..OW69_LO
};

/// The VC915 names it LPF, from AC V's menu, from the 2 V range up (VC915-UM
/// p.74, p.105).
const VC915_LPF: MeterKey = MeterKey {
    label: "LPF",
    ..VC891_LO
};

/// COMP: the VC925 PV's manual doesn't limit it, though some softkeys grey out
/// in some functions without saying which (VC925-UM p.86, p.88). First in its
/// list, as it stays.
const COMP: MeterKey = MeterKey {
    command: "compare",
    label: "COMP",
    in_words: false,
    hover: Some("Compare the reading with the set limits"),
    applies: |_| true,
};

/// The VC915's is the same, but for Motor, whose softkeys show only HOLD
/// (VC915-UM p.75-76, p.86, p.89).
const VC915_COMP: MeterKey = MeterKey {
    applies: |m| !function_in(m, &[code::MOTOR]),
    ..COMP
};

/// AC/DC, F1 in AC+DC V, picks what the sub-display shows (VC915-UM p.73).
const VC915_AC_DC: MeterKey = MeterKey {
    command: "ac_dc",
    label: "AC/DC",
    in_words: false,
    hover: Some("Show VAC or VDC on the second display"),
    applies: |m| function_in(m, &[code::AC_DC_V]),
};

/// MOTOR is in the V~ position's AC menu, and offered in the Motor it lands on,
/// though that screen's softkeys show only HOLD (VC915-UM p.75-76).
const VC915_MOTOR: MeterKey = MeterKey {
    command: "motor",
    label: "MOTOR",
    in_words: false,
    hover: Some("Show 3-phase rotation direction"),
    applies: |m| {
        (function_in(m, &[function::AC_V]) && m.unit == "V") || function_in(m, &[code::MOTOR])
    },
};

pub(super) const OW67_METER_KEYS: MeterKeys = MeterKeys {
    context: &[POWER_DISPLAY, OW_CURRENT_LOOP],
    ..MeterKeys::NONE
};

/// The OW69B's keys: the B series' Hz/Duty, in the same functions, Lo and
/// 4-20mA.
pub(super) const OW69_METER_KEYS: MeterKeys = MeterKeys {
    context: &[B_HZ_DUTY, OW69_LO, OW_CURRENT_LOOP],
    ..MeterKeys::NONE
};

/// On the Voltcraft meters REL sets the reading to zero; held, or by changing
/// function, it is left (VC871-UM p.89; VC891-UM p.79; VC915-UM p.86;
/// VC925-UM p.87).
const VC_BUTTON_LABELS: &[ButtonLabel] = &[ButtonLabel {
    command: command::REL,
    label: None,
    hover: Some("Zero the reading — × to exit"),
}];

pub(super) const VC871_METER_KEYS: MeterKeys = MeterKeys {
    context: &[POWER_DISPLAY, CURRENT_LOOP_KEY],
    button_labels: VC_BUTTON_LABELS,
    ..MeterKeys::NONE
};

/// The VC891's keys: the B series' Hz/Duty, Lo, 4-20mA, and REL as on the other
/// Voltcraft meters.
pub(super) const VC891_METER_KEYS: MeterKeys = MeterKeys {
    context: &[B_HZ_DUTY, VC891_LO, CURRENT_LOOP_KEY],
    button_labels: VC_BUTTON_LABELS,
    ..MeterKeys::NONE
};

pub(super) const VC915_METER_KEYS: MeterKeys = MeterKeys {
    context: &[
        VC915_COMP,
        VC915_DISPLAY,
        VC915_LPF,
        VC915_MOTOR,
        VC915_AC_DC,
        CURRENT_LOOP_KEY,
    ],
    button_labels: VC_BUTTON_LABELS,
    ..MeterKeys::NONE
};

pub(super) const VC925_METER_KEYS: MeterKeys = MeterKeys {
    context: &[COMP, VC925_DISPLAY, CURRENT_LOOP_KEY],
    button_labels: VC_BUTTON_LABELS,
    ..MeterKeys::NONE
};

/// On the CMS, MaxMin is entered with one press and left with another
/// (CMS101-UM p.27/22). OWON's app sends Select, which no key on the meter
/// carries; on a UT217A it took V,DC to V,AC and did nothing in scope mode,
/// its effect elsewhere untried (spec §10.8, #45).
const CMS_BUTTON_LABELS: &[ButtonLabel] = &[
    ButtonLabel {
        command: command::MINMAX,
        label: None,
        hover: Some("Enter MAX/MIN; click again to leave it"),
    },
    ButtonLabel {
        command: command::SELECT,
        label: None,
        hover: Some("Send Select; switches V,DC to V,AC"),
    },
];

/// Inrush is AC A's, F3 on the menu's second page, and F3 again leaves it; the
/// function stays AC A in it (CMS101-UM p.15/10, p.18/13; spec §10.6).
const CMS_INRUSH: MeterKey = MeterKey {
    command: "inrush",
    label: "Inrush",
    in_words: false,
    hover: Some("Measure inrush current; click again to leave it"),
    applies: |m| function_in(m, &[function::AC_A]),
};

/// The CMS101 and CMS061: the B series' Hz/Duty, Inrush, and what their shared
/// buttons do.
pub(super) const CMS_METER_KEYS: MeterKeys = MeterKeys {
    context: &[B_HZ_DUTY, CMS_INRUSH],
    button_labels: CMS_BUTTON_LABELS,
    ..MeterKeys::NONE
};

#[cfg(test)]
mod tests {
    use super::super::decode15::decode;
    use super::super::decode15::tests::{frame as reading_frame, g24};
    use super::super::model::{self as entry, MODELS, Model};
    use super::*;

    /// A long press of the key that carries ᛒ switches Bluetooth (spec
    /// §9.1): no model sends one of any 6-byte model's Bluetooth key
    /// unless FFF2 has named the model, as an entry may be opened on
    /// another model's meter.
    #[test]
    fn no_long_press_of_the_bluetooth_key_is_ever_sent() {
        let bluetooth_keys: Vec<u8> = MODELS.iter().filter_map(|m| m.bluetooth_key).collect();
        for model in MODELS {
            for key in model.keys {
                assert!(
                    !(bluetooth_keys.contains(&key.code) && key.press == LONG)
                        || key.needs_confirmed,
                    "{} sends {:02X?}",
                    model.entry.id,
                    frame(key)
                );
            }
        }
        let confirmed: Vec<&str> = MODELS
            .iter()
            .flat_map(|m| m.keys)
            .filter(|k| k.needs_confirmed)
            .map(|k| k.command)
            .collect();
        assert!(confirmed.iter().all(|&c| c == "exit_rel"), "{confirmed:?}");
        for key in MODELS.iter().flat_map(|m| m.keys) {
            assert_ne!(frame(key), [0x09, 0x01], "Hold/Light long (spec §10.8)");
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
        let cms = [
            "select", "range", "hold", "rel", "hz_duty", "minmax", "inrush",
        ];
        let ow65 = ["select", "range", "hold", "rel", "minmax"];
        let ow67 = [
            "select",
            "range",
            "hold",
            "rel",
            "minmax",
            "peak",
            "current_loop",
            "display",
        ];
        let vc871 = [
            "select",
            "range",
            "auto",
            "hold",
            "rel",
            "exit_rel",
            "minmax",
            "peak",
            "current_loop",
            "display",
        ];
        let ow69 = [
            "select",
            "range",
            "hold",
            "rel",
            "hz_duty",
            "minmax",
            "lpf",
            "peak",
            "current_loop",
        ];
        let vc891 = [
            "select",
            "range",
            "auto",
            "hold",
            "rel",
            "exit_rel",
            "hz_duty",
            "minmax",
            "lpf",
            "peak",
            "current_loop",
        ];
        let vc915 = [
            "select",
            "range",
            "auto",
            "hold",
            "rel",
            "exit_rel",
            "minmax",
            "lpf",
            "compare",
            "ac_dc",
            "motor",
            "current_loop",
            "display",
        ];
        let vc925 = [
            "select",
            "range",
            "auto",
            "hold",
            "rel",
            "exit_rel",
            "minmax",
            "exit_minmax",
            "compare",
            "current_loop",
            "display",
        ];
        let expected: [(&str, &[&str]); 15] = [
            ("ow18b", &ow),
            ("ow18e", &ow),
            ("b33", &b33),
            ("b35t+", &b35),
            ("b41t+", &b35),
            ("cm2100b", &cm2100),
            ("cms101", &cms),
            ("cms061", &cms),
            ("ow65b", &ow65),
            ("ow67b", &ow67),
            ("ow69b", &ow69),
            ("vc871", &vc871),
            ("vc891", &vc891),
            ("vc915", &vc915),
            ("vc925pv", &vc925),
        ];
        for (model, (id, commands)) in MODELS.iter().zip(expected) {
            assert_eq!(model.entry.id, id);
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

    /// The 15-byte meters' codes (spec §10.8) and their manuals' long
    /// presses.
    #[test]
    fn the_15_byte_keys_send_the_apps_codes() {
        let code = |keys: &[Key], command| {
            keys.iter()
                .find(|k| k.command == command)
                .map(frame)
                .unwrap()
        };
        assert_eq!(code(&CMS, "hold"), [0x03, 0x01]);
        assert_eq!(code(&CMS, "inrush"), [0x0D, 0x01]);
        assert_eq!(code(&VC871, "peak"), [0x08, 0x01]);
        assert_eq!(code(&VC871, "current_loop"), [0x0A, 0x01]);
        assert_eq!(code(&VC871, "display"), [0x0C, 0x01]);
        assert_eq!(code(&VC891, "lpf"), [0x07, 0x01]);
        assert_eq!(code(&VC915, "compare"), [0x0F, 0x01]);
        assert_eq!(code(&VC915, "ac_dc"), [0x10, 0x01]);
        assert_eq!(code(&VC915, "motor"), [0x11, 0x01]);
        for keys in [&VC871[..], &VC891, &VC915, &VC925] {
            assert_eq!(code(keys, "auto"), [0x02, 0x00]);
            assert_eq!(code(keys, "exit_rel"), [0x04, 0x00]);
        }
        assert_eq!(code(&VC925, "exit_minmax"), [0x06, 0x00]);
    }

    /// The labels `model` offers for a frame of `function` at `prefix`.
    fn offered(model: &Model, function: u8, prefix: u8) -> Vec<&'static str> {
        offered_for(
            model,
            &reading_frame(g24(function, prefix, 1, false), 125, 0, 0, 0),
        )
    }

    fn offered_for(model: &Model, frame: &[u8]) -> Vec<&'static str> {
        let reading = decode(frame, model).unwrap();
        model
            .meter_keys
            .context
            .iter()
            .filter(|key| (key.applies)(&reading))
            .map(|key| key.label)
            .collect()
    }

    /// Inrush in AC A, and still in the frame a UT217A sent from Inrush (#45;
    /// the golden `cms061/inrush.yaml`).
    #[test]
    fn inrush_is_offered_in_ac_a() {
        for model in [&entry::CMS101, &entry::CMS061] {
            assert_eq!(offered(model, 3, 4), ["Hz/Duty", "Inrush"]);
            assert_eq!(offered(model, 2, 4), [""; 0]);
            assert_eq!(offered(model, 6, 4), ["Hz/Duty"]);
        }
        let inrush = [
            0xE2, 0x00, 0xF0, 0x00, 0x00, 0x00, 0xA2, 0x01, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02,
        ];
        assert_eq!(offered_for(&entry::CMS061, &inrush), ["Hz/Duty", "Inrush"]);
    }

    /// 4-20mA in DC mA, by the unit a frame's prefix gives, and in the loop
    /// function; not in DC µA or A. In AC mA only on the OW67B and OW69B, whose
    /// manuals don't say DC.
    #[test]
    fn four_to_twenty_ma_is_offered_in_dc_ma() {
        let has = |model, function, prefix| offered(model, function, prefix).contains(&"4-20mA");
        for model in [
            &entry::OW67B,
            &entry::OW69B,
            &entry::VC871,
            &entry::VC891,
            &entry::VC915,
            &entry::VC925PV,
        ] {
            assert!(has(model, 2, 3), "{} DC mA", model.entry.id);
            assert!(has(model, 17, 4), "{} loop", model.entry.id);
            assert!(!has(model, 2, 2), "{} DC µA", model.entry.id);
            assert!(!has(model, 2, 4), "{} DC A", model.entry.id);
            assert_eq!(
                has(model, 3, 3),
                matches!(model.entry.id, "ow67b" | "ow69b"),
                "{} AC mA",
                model.entry.id
            );
        }
        for model in [&entry::OW65B, &entry::CMS061, &entry::CMS101] {
            assert!(!has(model, 2, 3), "{}", model.entry.id);
        }
    }

    /// Display on each model's own functions.
    #[test]
    fn display_is_offered_where_each_manual_uses_it() {
        let has = |model, function| offered(model, function, 4).contains(&"Display");
        for model in [&entry::OW67B, &entry::VC871] {
            for function in [14, 15, 16, 18, 19, 20, 21, 22] {
                assert!(has(model, function), "{} {function}", model.entry.id);
            }
            for function in [0, 1, 2, 3, 4, 6, 17] {
                assert!(!has(model, function), "{} {function}", model.entry.id);
            }
        }
        for function in [1, 6, 7] {
            assert!(has(&entry::VC915, function), "{function}");
        }
        for function in [0, 2, 3, 4, 17, 23, 24] {
            assert!(!has(&entry::VC915, function), "{function}");
        }
        for function in [0, 1, 2, 6, 17, 25, 26, 28, 29] {
            assert!(has(&entry::VC925PV, function), "{function}");
        }
        for model in [&entry::OW65B, &entry::OW69B, &entry::VC891, &entry::CMS061] {
            for function in [1, 14] {
                assert!(!has(model, function), "{} {function}", model.entry.id);
            }
        }
    }

    /// Lo and LPF in AC V, under each model's own name. The OW69B's in AC+DC
    /// and AC mV too, which its manual has and the VC891's and VC915's don't.
    #[test]
    fn the_low_pass_key_is_offered_in_ac_v() {
        let has = |model, label, function| offered(model, function, 4).contains(&label);
        assert!(has(&entry::OW69B, "Lo", 1));
        assert!(has(&entry::OW69B, "Lo", 23));
        assert!(has(&entry::VC891, "Lo", 1));
        assert!(!has(&entry::VC891, "Lo", 23));
        assert!(has(&entry::VC915, "LPF", 1));
        assert!(!has(&entry::VC915, "LPF", 23));
        for (model, label) in [
            (&entry::OW69B, "Lo"),
            (&entry::VC891, "Lo"),
            (&entry::VC915, "LPF"),
        ] {
            for function in [0, 2, 3, 6] {
                assert!(
                    !has(model, label, function),
                    "{} {function}",
                    model.entry.id
                );
            }
        }
        assert!(offered(&entry::OW69B, 1, 3).contains(&"Lo"));
        assert!(!offered(&entry::VC891, 1, 3).contains(&"Lo"));
        assert!(!offered(&entry::VC915, 1, 3).contains(&"LPF"));
        // A frame with the status bit set offers it still.
        let lpf_on = reading_frame(g24(1, 4, 1, false), 125, 0, 0, 1 << 9);
        assert!(offered_for(&entry::OW69B, &lpf_on).contains(&"Lo"));
    }

    /// What the VC915 and VC925 PV offer, in the order drawn: COMP first, as
    /// it stays, except in the VC915's Motor.
    #[test]
    fn the_vc915_and_vc925_offer_their_own_keys() {
        for (function, prefix, labels) in [
            (0, 4, ["COMP"].as_slice()),
            (1, 4, &["COMP", "Display", "LPF", "MOTOR"]),
            (2, 3, &["COMP", "4-20mA"]),
            (17, 4, &["COMP", "4-20mA"]),
            (23, 4, &["COMP", "AC/DC"]),
            (1, 3, &["COMP", "Display"]),
            (24, 4, &["MOTOR"]),
            (6, 4, &["COMP", "Display"]),
        ] {
            assert_eq!(
                offered(&entry::VC915, function, prefix),
                labels,
                "{function}"
            );
        }
        assert_eq!(offered(&entry::VC925PV, 0, 4), ["COMP", "Display"]);
        assert_eq!(
            offered(&entry::VC925PV, 2, 3),
            ["COMP", "Display", "4-20mA"]
        );
    }

    #[test]
    fn context_keys_follow_the_function() {
        use crate::measurement::{MeasuredValue, Measurement};
        let at = |function: u8| Measurement {
            mode_raw: u16::from(function),
            ..Measurement::from_payload(&[], MeasuredValue::Overload)
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
