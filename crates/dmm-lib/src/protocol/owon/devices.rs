//! The OWON registry entries; [`DEVICES`] orders them.
//!
//! One entry per model code a 6-byte meter reads out of FFF2 (spec §1),
//! named for the models OWON's manuals sell under it. There are no `b35` or
//! `b35t` aliases: a B35 or B35T without the "+" may be an older meter that
//! sends the 14-byte ASCII frame (spec §11, §14.4), which this family does
//! not read.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::model;
use super::{FINGERPRINT, OwonProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;

/// OWON's meters have the radio built in and no cable.
const LINKS: &[&str] = &[crate::BLUETOOTH];

/// "BDM" is the default name every manual's app pairs with (spec §2); a
/// B41T+ also gives "LILLIPUT" as its GAP device name, and a B35T+ showed
/// as "Lilliput" on Windows (spec §14.4). The owner can rename a meter
/// (spec §7.2); a renamed one opens with `--device <id> --adapter
/// <address>`.
const NAMES: &[&str] = &["BDM", "LILLIPUT"];

/// The OW18B's and OW18E's, from OW18-UM p.23/18 (Bluetooth on), p.22/17
/// (off after 10 minutes idle), and the OW16B's, from OW16-UM p.24/19,
/// p.22/17, the same. Any phone app goes first: a B35T+ takes one central at
/// a time (spec §14.4).
const ACTIVATION_OW: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the Hz/Duty△/ᛒ key until the ᛒ icon shows
Note: Bluetooth switches off after 10 minutes idle, after two beeps; hold Hz/Duty△/ᛒ again to switch it back on.";

/// B33-UM p.23/18 (Bluetooth on), p.21/16-22/17 (off after 10 minutes
/// idle), p.28/23 (off when an offline recording ends).
const ACTIVATION_B33: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the △/ᛒ key until the ᛒ icon shows
Note: Bluetooth switches off after 10 minutes idle, after two beeps, and when an offline recording ends; hold △/ᛒ again to switch it back on.";

/// B35-UM p.25/20 (Bluetooth on), p.24/19-25/20 (off after 10 minutes
/// idle); the manual covers the B35 series and the B41T.
const ACTIVATION_B35_B41: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the △/ᛒ key until the ᛒ icon shows
Note: Bluetooth switches off after 10 minutes idle, after two beeps; hold △/ᛒ again to switch it back on.";

/// CM2100-UM p.16/13 (Bluetooth on, off after 5 minutes idle), p.9/6
/// (held about 2 s, the key switches Bluetooth on or off), p.14/11-15/12
/// (auto power-off, and cancelling it until the next power cycle).
const ACTIVATION_CM2100B: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the ZERO/ᛒ key for about 2 seconds, until the ᛒ icon shows at the top left of the display
Note: Bluetooth switches off after 5 minutes idle, or when ZERO/ᛒ is held again. The meter switches off after about 15 minutes idle; hold SELECT while turning it on to disable that until it is next switched off.";

/// OWON's product pages, one per series (owontech.com).
const OW18_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/ow-series-bluetooth-multimeter.html";
const OW18DE_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/4-1-2-digits-handheld-digital-multimeter.html";
const B33_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-3-3-4-digit-bluetooth-multimeter.html";
const B35_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-3-5-6-digit-bluetooth-multimeter.html";
const B41T_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-6000-count-multimeter.html";
const CM2100_PAGE: &str = "https://www.owontech.com/digital-multimeters/clamp-meters/cm2100-series-smart-ac-dc-clamp-meter.html";

pub(crate) static OW18B: SelectableDevice = SelectableDevice {
    id: model::OW18B.id,
    display_name: model::OW18B.name,
    aliases: &["ow16b", "owon-ow18b", "owon-ow16b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW18B, 39)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW18_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static OW18E: SelectableDevice = SelectableDevice {
    id: model::OW18E.id,
    display_name: model::OW18E.name,
    aliases: &["owon-ow18e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW18E, 40)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW18DE_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B33: SelectableDevice = SelectableDevice {
    id: model::B33.id,
    display_name: model::B33.name,
    // Which code a T or "+" variant sends is open (spec §1).
    aliases: &["b33t", "b33+", "b33t+", "owon-b33"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B33,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B33, 41)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B33_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B35T_PLUS: SelectableDevice = SelectableDevice {
    id: model::B35.id,
    display_name: model::B35.name,
    aliases: &["b35+", "owon-b35t+"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B35_B41,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B35, 42)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B35_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B41T_PLUS: SelectableDevice = SelectableDevice {
    id: model::B41.id,
    display_name: model::B41.name,
    aliases: &["b41t", "owon-b41t+"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B35_B41,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B41, 43)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B41T_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static CM2100B: SelectableDevice = SelectableDevice {
    id: model::CM2100B.id,
    display_name: model::CM2100B.name,
    aliases: &["owon-cm2100b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_CM2100B,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::CM2100B, 44)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(CM2100_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};
