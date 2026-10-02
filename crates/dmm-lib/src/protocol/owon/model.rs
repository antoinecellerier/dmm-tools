//! The models a registry entry stands for, by the model code they read out
//! of FFF2, and that read itself
//! (`docs/research/owon/reverse-engineered-protocol.md` §1, §4, §10.1).

use super::keys::{self, Key};
use crate::protocol::MeterKeys;
use std::fmt;

/// Which frame a model sends (spec §10.1): OWON's app picks its parser by
/// the model code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FrameKind {
    /// Three 16-bit words (spec §5, §6).
    Six,
    /// Five 24-bit words with a sub-display (spec §10.2).
    Fifteen,
}

impl FrameKind {
    /// The most sub-values one frame carries: the 15-byte frame's
    /// sub-display (spec §10.2).
    pub(super) fn max_aux_values(self) -> usize {
        match self {
            FrameKind::Six => 0,
            FrameKind::Fifteen => 1,
        }
    }

    /// How messages name it.
    pub(super) fn name(self) -> &'static str {
        match self {
            FrameKind::Six => "6-byte",
            FrameKind::Fifteen => "15-byte",
        }
    }
}

/// What function 13 is on a model (spec §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Function13 {
    /// NCV, levels 0-4 (spec §6.7): the OW16, OW18, CM2100 and CMS, which
    /// have an NCV position or function (spec §9.2, §9.4).
    Ncv,
    /// The app's NCV against the PC's "ADP" on the B series, whose dials
    /// have neither (spec §6.2), and on the other 15-byte meters, which
    /// have no NCV (spec §9.4): what such a meter sends is open.
    Open,
}

/// One registry entry's model: the FFF2 code that names it and what it
/// does differently.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Model {
    /// FFF2 byte 0 (spec §1).
    pub(super) code: u8,
    pub(super) frame: FrameKind,
    /// The registry id.
    pub(super) id: &'static str,
    /// The registry display name, and the profile's model name.
    pub(super) name: &'static str,
    pub(super) function13: Function13,
    /// Whether status bit 7, RMR, is the model's own: "only B41 model" in
    /// the manual's app table (spec §6.6), so silent there.
    pub(super) rmr: bool,
    /// The remote keys, by command (spec §7.3).
    pub(super) keys: &'static [Key],
    /// The keys' commands, in the same order, for the profile.
    pub(super) commands: &'static [&'static str],
    /// The key code a long press of which switches Bluetooth (spec §7.1,
    /// §9.1); never sent long. `None` on the 15-byte meters, whose BLE
    /// key's code is unknown (spec §10.8).
    pub(super) bluetooth_key: Option<u8>,
    pub(super) meter_keys: MeterKeys,
    /// Whether the sub-display takes the main display's function under
    /// REL, MAX or MIN: the VC871 alone (spec §10.7).
    pub(super) sub_follows_main: bool,
}

/// Code 18, the OW18B: OWON's PC software names it `OW18_16`, the one
/// link to the OW16B, which neither program lists (spec §1).
pub(super) static OW18B: Model = Model {
    code: 18,
    id: "ow18b",
    name: "OWON OW18B/OW16B",
    frame: FrameKind::Six,
    function13: Function13::Ncv,
    rmr: false,
    keys: &keys::OW,
    commands: &keys::OW_COMMANDS,
    bluetooth_key: Some(keys::OW_BLUETOOTH_KEY),
    meter_keys: keys::OW_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static OW18E: Model = Model {
    code: 20,
    id: "ow18e",
    name: "OWON OW18E",
    frame: FrameKind::Six,
    function13: Function13::Ncv,
    rmr: false,
    keys: &keys::OW,
    commands: &keys::OW_COMMANDS,
    bluetooth_key: Some(keys::OW_BLUETOOTH_KEY),
    meter_keys: keys::OW_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static B33: Model = Model {
    code: 33,
    id: "b33",
    name: "OWON B33",
    frame: FrameKind::Six,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::B33,
    commands: &keys::B33_COMMANDS,
    bluetooth_key: Some(keys::B_BLUETOOTH_KEY),
    meter_keys: keys::B_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static B35: Model = Model {
    code: 35,
    id: "b35t+",
    name: "OWON B35T+",
    frame: FrameKind::Six,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::B35_B41,
    commands: &keys::B35_B41_COMMANDS,
    bluetooth_key: Some(keys::B_BLUETOOTH_KEY),
    meter_keys: keys::B_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static B41: Model = Model {
    code: 41,
    id: "b41t+",
    name: "OWON B41T+",
    frame: FrameKind::Six,
    function13: Function13::Open,
    rmr: true,
    keys: &keys::B35_B41,
    commands: &keys::B35_B41_COMMANDS,
    bluetooth_key: Some(keys::B_BLUETOOTH_KEY),
    meter_keys: keys::B_METER_KEYS,
    sub_follows_main: false,
};

/// Code 21 is in OWON's app only (spec §1).
pub(super) static CM2100B: Model = Model {
    code: 21,
    id: "cm2100b",
    name: "OWON CM2100B",
    frame: FrameKind::Six,
    function13: Function13::Ncv,
    rmr: false,
    keys: &keys::CM2100,
    commands: &keys::CM2100_COMMANDS,
    bluetooth_key: Some(keys::CM2100_BLUETOOTH_KEY),
    meter_keys: keys::CM2100_METER_KEYS,
    sub_follows_main: false,
};

/// The CMS101 and CMS061, clamp meters with an oscilloscope mode and NCV
/// (spec §9.4), share the app's key list (spec §10.8).
pub(super) static CMS101: Model = Model {
    code: 101,
    id: "cms101",
    name: "OWON CMS101",
    frame: FrameKind::Fifteen,
    function13: Function13::Ncv,
    rmr: false,
    keys: &keys::CMS,
    commands: &keys::CMS_COMMANDS,
    bluetooth_key: None,
    meter_keys: keys::HZ_DUTY_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static CMS061: Model = Model {
    code: 61,
    id: "cms061",
    name: "OWON CMS061",
    frame: FrameKind::Fifteen,
    function13: Function13::Ncv,
    rmr: false,
    keys: &keys::CMS,
    commands: &keys::CMS_COMMANDS,
    bluetooth_key: None,
    meter_keys: keys::HZ_DUTY_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static OW65B: Model = Model {
    code: 65,
    id: "ow65b",
    name: "OWON OW65B",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::OW65,
    commands: &keys::OW65_COMMANDS,
    bluetooth_key: None,
    meter_keys: MeterKeys::NONE,
    sub_follows_main: false,
};

/// The OW67B is the VC871's twin by its manual (spec §9.4), but the app
/// reads it with the ordinary parser (spec §10.7).
pub(super) static OW67B: Model = Model {
    code: 67,
    id: "ow67b",
    name: "OWON OW67B",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::OW67,
    commands: &keys::OW67_COMMANDS,
    bluetooth_key: None,
    meter_keys: MeterKeys::NONE,
    sub_follows_main: false,
};

pub(super) static OW69B: Model = Model {
    code: 69,
    id: "ow69b",
    name: "OWON OW69B",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::OW69,
    commands: &keys::OW69_COMMANDS,
    bluetooth_key: None,
    meter_keys: keys::HZ_DUTY_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static VC871: Model = Model {
    code: 87,
    id: "vc871",
    name: "Voltcraft VC871",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::VC871,
    commands: &keys::VC871_COMMANDS,
    bluetooth_key: None,
    meter_keys: MeterKeys::NONE,
    sub_follows_main: true,
};

pub(super) static VC891: Model = Model {
    code: 89,
    id: "vc891",
    name: "Voltcraft VC891",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::VC891,
    commands: &keys::VC891_COMMANDS,
    bluetooth_key: None,
    meter_keys: keys::HZ_DUTY_METER_KEYS,
    sub_follows_main: false,
};

pub(super) static VC915: Model = Model {
    code: 91,
    id: "vc915",
    name: "Voltcraft VC915",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::VC915,
    commands: &keys::VC915_COMMANDS,
    bluetooth_key: None,
    meter_keys: MeterKeys::NONE,
    sub_follows_main: false,
};

pub(super) static VC925PV: Model = Model {
    code: 92,
    id: "vc925pv",
    name: "Voltcraft VC925 PV",
    frame: FrameKind::Fifteen,
    function13: Function13::Open,
    rmr: false,
    keys: &keys::VC925,
    commands: &keys::VC925_COMMANDS,
    bluetooth_key: None,
    meter_keys: MeterKeys::NONE,
    sub_follows_main: false,
};

/// Every model, in registry order.
pub(super) static MODELS: [&Model; 15] = [
    &OW18B, &OW18E, &B33, &B35, &B41, &CM2100B, &CMS101, &CMS061, &OW65B, &OW67B, &OW69B, &VC871,
    &VC891, &VC915, &VC925PV,
];

/// What detection falls back to with no model code, or one no entry has:
/// OWON's PC software decodes an unknown code as a B-series meter (spec
/// §1), which sends the 6-byte frame.
pub(super) static FALLBACK: &Model = &B35;

impl Model {
    /// The model whose code this is; `None` for a code no entry carries.
    pub(super) fn for_code(code: u8) -> Option<&'static Model> {
        MODELS.iter().copied().find(|m| m.code == code)
    }
}

/// Why a model code's meter cannot be read here, for the codes OWON's
/// programs read but no entry carries, to follow "the meter reports OWON
/// model code N"; `None` for every other code.
pub(super) fn unsupported_format(code: u8) -> Option<&'static str> {
    match code {
        // The app's VC831 and VC851, which have no Bluetooth (spec §1,
        // §10.1).
        83 | 85 => {
            Some(", which this tool does not read yet; please open an issue with this message")
        }
        // Series 55's 6-byte frame takes its sign from the function word
        // (spec §6.5).
        55 => Some(
            ": it sends OWON's 6-byte frame with the sign in the function word, which this tool \
             does not read yet",
        ),
        _ => None,
    }
}

/// FFF2's value (spec §4). Every field but the code is optional: OWON's PC
/// software reads each one only if the value holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fff2 {
    /// Byte 0, the model code.
    pub(super) code: u8,
    /// Byte 1, battery percent (PC); anything above 100 is "not supported".
    pub(super) battery: Option<u8>,
    /// Bytes 2-4, shown "b2.b3.b4".
    pub(super) firmware: Option<[u8; 3]>,
    /// Byte 5, the offline record state.
    pub(super) record: Option<Record>,
}

/// FFF2 byte 5 (spec §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Record {
    /// `FF`: no offline record.
    None,
    /// `00`: offline record, not recording.
    Idle,
    /// `01`: recording to the meter's memory.
    Recording,
    /// Any other value.
    Other(u8),
}

impl Fff2 {
    /// Parse a value; `None` only for an empty one.
    pub(crate) fn parse(value: &[u8]) -> Option<Fff2> {
        let (&code, rest) = value.split_first()?;
        Some(Fff2 {
            code,
            battery: rest.first().copied(),
            firmware: rest.get(1..4).map(|f| [f[0], f[1], f[2]]),
            record: rest.get(4).map(|&b| match b {
                0xFF => Record::None,
                0x00 => Record::Idle,
                0x01 => Record::Recording,
                other => Record::Other(other),
            }),
        })
    }
}

/// For the DEBUG line `init` writes.
impl fmt::Display for Fff2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "model code {}", self.code)?;
        if let Some([a, b, c]) = self.firmware {
            write!(f, ", firmware {a}.{b}.{c}")?;
        }
        match self.battery {
            Some(percent @ 0..=100) => write!(f, ", battery {percent}%"),
            Some(_) => write!(f, ", battery not reported"),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §12.2's value: code 18, battery 99 %, firmware 4.0.9, not
    /// recording.
    #[test]
    fn the_worked_example_is_an_ow18b() {
        let value = [
            0x12, 0x63, 0x04, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let info = Fff2::parse(&value).unwrap();
        assert_eq!(
            info,
            Fff2 {
                code: 18,
                battery: Some(99),
                firmware: Some([4, 0, 9]),
                record: Some(Record::Idle),
            }
        );
        assert_eq!(Model::for_code(info.code).map(|m| m.id), Some("ow18b"));
        assert_eq!(
            info.to_string(),
            "model code 18, firmware 4.0.9, battery 99%"
        );
    }

    /// Spec §14.5's B41T+ value: byte 1 `FF`, firmware 0.1.2.
    #[test]
    fn a_b41t_value_reports_no_battery() {
        let info = Fff2::parse(&[0x29, 0xFF, 0x00, 0x01, 0x02, 0x00]).unwrap();
        assert_eq!(info.code, 41);
        assert_eq!(info.firmware, Some([0, 1, 2]));
        assert_eq!(Model::for_code(41).map(|m| m.id), Some("b41t+"));
        assert_eq!(
            info.to_string(),
            "model code 41, firmware 0.1.2, battery not reported"
        );
    }

    /// A shorter value gives what it holds; only an empty one fails.
    #[test]
    fn a_short_value_gives_its_fields() {
        assert_eq!(Fff2::parse(&[]), None);
        assert_eq!(
            Fff2::parse(&[0x21]),
            Some(Fff2 {
                code: 33,
                battery: None,
                firmware: None,
                record: None,
            })
        );
        let info = Fff2::parse(&[0x23, 0x50, 0x01, 0x02]).unwrap();
        assert_eq!(info.battery, Some(80));
        assert_eq!(info.firmware, None);
        let recording = Fff2::parse(&[0x23, 0x50, 0x00, 0x01, 0x02, 0x01]).unwrap();
        assert_eq!(recording.record, Some(Record::Recording));
        let none = Fff2::parse(&[0x23, 0x50, 0x00, 0x01, 0x02, 0xFF]).unwrap();
        assert_eq!(none.record, Some(Record::None));
    }

    /// Codes 55, 83, 85 and 223 have no entry (spec §1, §10.1).
    #[test]
    fn codes_without_an_entry_name_no_model() {
        for code in [55, 83, 85, 223, 0] {
            assert!(Model::for_code(code).is_none(), "{code}");
        }
        for model in MODELS {
            assert!(std::ptr::eq(Model::for_code(model.code).unwrap(), model));
            assert!(unsupported_format(model.code).is_none(), "{}", model.id);
        }
    }

    /// The VC831/VC851 codes and series 55 are refused, nothing else.
    #[test]
    fn the_other_formats_are_named() {
        for code in [83, 85] {
            assert!(
                unsupported_format(code).is_some_and(|why| why.contains("open an issue")),
                "{code}"
            );
        }
        assert!(unsupported_format(55).is_some_and(|why| why.contains("sign")));
        assert!(unsupported_format(223).is_none());
    }

    /// The app's 15-byte codes are the nine 15-byte entries, in registry
    /// order (spec §10.1), each with one sub-value and no Bluetooth key
    /// code; only the VC871's sub-display follows the main one (spec
    /// §10.7).
    #[test]
    fn the_15_byte_codes_are_their_entries() {
        let fifteen: Vec<(u8, &str)> = MODELS
            .iter()
            .filter(|m| m.frame == FrameKind::Fifteen)
            .map(|m| (m.code, m.id))
            .collect();
        assert_eq!(
            fifteen,
            [
                (101, "cms101"),
                (61, "cms061"),
                (65, "ow65b"),
                (67, "ow67b"),
                (69, "ow69b"),
                (87, "vc871"),
                (89, "vc891"),
                (91, "vc915"),
                (92, "vc925pv"),
            ]
        );
        for model in MODELS {
            let fifteen = model.frame == FrameKind::Fifteen;
            assert_eq!(model.bluetooth_key.is_none(), fifteen, "{}", model.id);
            assert_eq!(model.frame.max_aux_values(), usize::from(fifteen));
            assert_eq!(model.sub_follows_main, model.code == 87, "{}", model.id);
        }
        let ncv: Vec<&str> = MODELS
            .iter()
            .filter(|m| m.function13 == Function13::Ncv)
            .map(|m| m.id)
            .collect();
        assert_eq!(ncv, ["ow18b", "ow18e", "cm2100b", "cms101", "cms061"]);
    }
}
