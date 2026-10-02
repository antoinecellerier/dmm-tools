//! The OWON registry entries; [`DEVICES`] orders them.
//!
//! One entry per model code a meter reads out of FFF2 (spec §1, §10.1),
//! named for the models OWON's and Voltcraft's manuals sell under it. There
//! are no `b35` or `b35t` aliases: a B35 or B35T without the "+" may be an
//! older meter that sends the 14-byte ASCII frame (spec §11, §14.4), which
//! this family does not read.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::model;
use super::{FINGERPRINT, OwonProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{Brand, SelectableDevice};

/// OWON's meters have the radio built in and no cable.
const LINKS: &[&str] = &[crate::BLUETOOTH];

/// "BDM" is the default name every manual's app pairs with (spec §2); a
/// B41T+ also gives "LILLIPUT" as its GAP device name, and a B35T+ showed
/// as "Lilliput" on Windows (spec §14.4). The owner can rename a meter
/// (spec §7.2); a renamed one opens with `--device <id> --adapter
/// <address>`.
const NAMES: &[&str] = &["BDM", "LILLIPUT"];

/// The 15-byte OWON meters' manuals show "BDM" in the app's list (spec §2).
const NAMES_15: &[&str] = &["BDM"];

/// Voltcraft's manuals say to pick "VC871", "VC891" or "VCxxx", its app
/// manual shows "VC8xx_1", and its later app manual "BDM" (spec §2): which
/// one a meter advertises is open.
const NAMES_VOLTCRAFT: &[&str] = &["BDM", "VC8", "VC9"];

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

/// CMS101-UM and CMS061-UM p.33/28-34/29, the same text in both: Tab⇌,
/// which carries the BLE label (p.12/7), held until the Bluetooth icon
/// shows; auto power-off is off while Bluetooth is on.
const ACTIVATION_CMS: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the Tab⇌ key (BLE printed above it) until the Bluetooth icon shows
Note: the meter does not switch itself off while Bluetooth is on.";

/// OW65-UM p.13/8 (SETUP, BLE above it, held turns Bluetooth on or off),
/// p.28/23.
const ACTIVATION_OW65B: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the SETUP key (BLE printed above it) until the Bluetooth icon shows
Note: holding SETUP again switches Bluetooth off.";

/// OW67-UM p.13/8 (<, BLE above it), p.33/28 (held until the icon shows;
/// off after 5 minutes idle).
const ACTIVATION_OW67B: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the < key (BLE printed above it) until the Bluetooth icon shows
Note: Bluetooth switches off after 5 minutes idle; hold < again to switch it back on.";

/// OW69-UM p.13/8, p.30/25.
const ACTIVATION_OW69B: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the < key (BLE printed above it) until the Bluetooth icon shows";

/// VC871-UM p.92 and VC891-UM p.82, the same text; VC-APP p.10, the app
/// manual for both: Bluetooth is off at every power-on.
const ACTIVATION_VC871_VC891: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows at the top left
Note: Bluetooth is off each time the meter is switched on; hold BLE again then.";

/// VC915-UM p.88 and VC925-UM p.94, the same steps.
const ACTIVATION_VC915_VC925: &str = "\
1. Disconnect the meter from any phone app
2. Turn the meter on
3. Hold the BLE button for about 2 seconds, until the meter beeps and the Bluetooth symbol shows";

/// OWON's product pages, one per series (owontech.com).
const OW18_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/ow-series-bluetooth-multimeter.html";
const OW18DE_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/4-1-2-digits-handheld-digital-multimeter.html";
const B33_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-3-3-4-digit-bluetooth-multimeter.html";
const B35_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-3-5-6-digit-bluetooth-multimeter.html";
const B41T_PAGE: &str = "https://www.owontech.com/digital-multimeters/handheld-multimeters/bd-series-6000-count-multimeter.html";
const CM2100_PAGE: &str = "https://www.owontech.com/digital-multimeters/clamp-meters/cm2100-series-smart-ac-dc-clamp-meter.html";
/// The CMS page covers the 1000 A CMS101 and the 600 A CMS061.
const CMS_PAGE: &str = "https://www.owontech.com/digital-multimeters/clamp-meters/cms101-1000a-smart-ac-dc-clamp-meter-and.html";
/// owontech.com and owon.com.hk list no OW65, OW67 or OW69 page; OWON
/// Japan's does, over http only (https does not answer, 2026-10-02).
const OW65_PAGE: &str = "http://owon.co.jp/products_info.asp?ProductID=95";
const OW67_PAGE: &str = "http://owon.co.jp/products_info.asp?ProductID=96";
const OW69_PAGE: &str = "http://owon.co.jp/products_info.asp?ProductID=97";

/// Conrad's manuals, by item number: its shop pages refuse scripted
/// fetches, so no product page could be checked (2026-10-02).
const VC871_MANUAL: &str = "https://asset.conrad.com/media10/add/160267/c1/-/gl/002576867ML00";
const VC891_MANUAL: &str = "https://asset.conrad.com/media10/add/160267/c1/-/gl/002576866ML00";
const VC915_MANUAL: &str = "https://asset.conrad.com/media10/add/160267/c1/-/gl/003072347ML00";
const VC925_MANUAL: &str = "https://asset.conrad.com/media10/add/160267/c1/-/gl/003072348ML00";

pub(crate) static OW18B: SelectableDevice = SelectableDevice {
    id: model::OW18B.id,
    display_name: model::OW18B.name,
    brand: Brand::Owon,
    aliases: &["ow16b", "owon-ow18b", "owon-ow16b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW18B, Some(39))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW18_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static OW18E: SelectableDevice = SelectableDevice {
    id: model::OW18E.id,
    display_name: model::OW18E.name,
    brand: Brand::Owon,
    aliases: &["owon-ow18e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW18E, Some(40))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW18DE_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B33: SelectableDevice = SelectableDevice {
    id: model::B33.id,
    display_name: model::B33.name,
    brand: Brand::Owon,
    // Which code a T or "+" variant sends is open (spec §1).
    aliases: &["b33t", "b33+", "b33t+", "owon-b33"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B33,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B33, Some(41))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B33_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B35T_PLUS: SelectableDevice = SelectableDevice {
    id: model::B35.id,
    display_name: model::B35.name,
    brand: Brand::Owon,
    aliases: &["b35+", "owon-b35t+"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B35_B41,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B35, Some(42))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B35_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static B41T_PLUS: SelectableDevice = SelectableDevice {
    id: model::B41.id,
    display_name: model::B41.name,
    brand: Brand::Owon,
    aliases: &["b41t", "owon-b41t+"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_B35_B41,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::B41, Some(43))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(B41T_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static CM2100B: SelectableDevice = SelectableDevice {
    id: model::CM2100B.id,
    display_name: model::CM2100B.name,
    brand: Brand::Owon,
    aliases: &["owon-cm2100b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_CM2100B,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::CM2100B, Some(44))),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(CM2100_PAGE),
    links: LINKS,
    bluetooth_names: NAMES,
};

pub(crate) static CMS101: SelectableDevice = SelectableDevice {
    id: model::CMS101.id,
    display_name: model::CMS101.name,
    brand: Brand::Owon,
    aliases: &["owon-cms101"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_CMS,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::CMS101, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(CMS_PAGE),
    links: LINKS,
    bluetooth_names: NAMES_15,
};

pub(crate) static CMS061: SelectableDevice = SelectableDevice {
    id: model::CMS061.id,
    display_name: model::CMS061.name,
    brand: Brand::Owon,
    aliases: &["owon-cms061"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_CMS,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::CMS061, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(CMS_PAGE),
    links: LINKS,
    bluetooth_names: NAMES_15,
};

pub(crate) static OW65B: SelectableDevice = SelectableDevice {
    id: model::OW65B.id,
    display_name: model::OW65B.name,
    brand: Brand::Owon,
    aliases: &["owon-ow65b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW65B,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW65B, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW65_PAGE),
    links: LINKS,
    bluetooth_names: NAMES_15,
};

pub(crate) static OW67B: SelectableDevice = SelectableDevice {
    id: model::OW67B.id,
    display_name: model::OW67B.name,
    brand: Brand::Owon,
    aliases: &["owon-ow67b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW67B,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW67B, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW67_PAGE),
    links: LINKS,
    bluetooth_names: NAMES_15,
};

pub(crate) static OW69B: SelectableDevice = SelectableDevice {
    id: model::OW69B.id,
    display_name: model::OW69B.name,
    brand: Brand::Owon,
    aliases: &["owon-ow69b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_OW69B,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::OW69B, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(OW69_PAGE),
    links: LINKS,
    bluetooth_names: NAMES_15,
};

pub(crate) static VC871: SelectableDevice = SelectableDevice {
    id: model::VC871.id,
    display_name: model::VC871.name,
    brand: Brand::Voltcraft,
    aliases: &["vc-871"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_VC871_VC891,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::VC871, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(VC871_MANUAL),
    links: LINKS,
    bluetooth_names: NAMES_VOLTCRAFT,
};

pub(crate) static VC891: SelectableDevice = SelectableDevice {
    id: model::VC891.id,
    display_name: model::VC891.name,
    brand: Brand::Voltcraft,
    aliases: &["vc-891"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_VC871_VC891,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::VC891, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(VC891_MANUAL),
    links: LINKS,
    bluetooth_names: NAMES_VOLTCRAFT,
};

pub(crate) static VC915: SelectableDevice = SelectableDevice {
    id: model::VC915.id,
    display_name: model::VC915.name,
    brand: Brand::Voltcraft,
    aliases: &["vc-915"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_VC915_VC925,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::VC915, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(VC915_MANUAL),
    links: LINKS,
    bluetooth_names: NAMES_VOLTCRAFT,
};

pub(crate) static VC925PV: SelectableDevice = SelectableDevice {
    id: model::VC925PV.id,
    display_name: model::VC925PV.name,
    brand: Brand::Voltcraft,
    aliases: &["vc-925pv"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_VC915_VC925,
    family: DeviceFamily::Owon,
    new_protocol: || Box::new(OwonProtocol::new(&model::VC925PV, None)),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(VC925_MANUAL),
    links: LINKS,
    bluetooth_names: NAMES_VOLTCRAFT,
};
