//! The ZOTEK registry entries; [`DEVICES`] orders them.
//!
//! ZOTEK meters are sold as ZOYI, ZOTEK, BSIDE and ANENG. A packet names its
//! layout only, never the model or brand, so there is one entry per layout,
//! named for the models the layout is known from
//! (docs/research/zotek/reverse-engineered-protocol.md §1, §11.4).
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, ZotekProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;

/// ZOTEK's own download page, where each model's manual is listed; the
/// files themselves are on Google Drive, which ZOTEK may re-upload.
const ZOTEK_SUPPORT_URL: &str = "https://zotektools.com/?support/";

/// ZT-300AB manual p.10 and p.28 (Bluetooth), p.11 (auto power-off).
const ACTIVATION_ZT300AB: &str = "\
1. Turn the meter on
2. Hold the Hz% button for 2 seconds; the Bluetooth symbol shows
Note: the meter switches off after 15 minutes idle; hold SEL while turning it on to disable that.";

/// ZT-5566SE manual p.22: POWER switches Bluetooth on for the speaker; the
/// manual gives no steps for an app link.
const ACTIVATION_ZT5566SE: &str = "\
1. Turn the meter on
2. Press POWER to turn Bluetooth on; the Bluetooth symbol flashes
Note: the manual documents Bluetooth for its speaker only; readings over it are unverified.";

/// ZT-5BQ manual p.2 panel 7: Power and Hz together switch Bluetooth on;
/// the same panel gives the auto power-off.
const ACTIVATION_ZT5BQ: &str = "\
1. Turn the meter on
2. Press Power and Hz together; the Bluetooth symbol shows
Note: the meter switches off after 15 minutes idle; hold Hz/NCV while turning it on to disable that.";

/// ZT-5B manual p.1 panel 3: the red button turns the meter on and, with a
/// short press, Bluetooth; the same panel gives the auto power-off.
const ACTIVATION_ZT5B: &str = "\
1. Hold the red button for 2 seconds to turn the meter on
2. Short press the red button; the Bluetooth symbol shows
Note: the meter switches off after 15 minutes idle; press NCV before turning it on to disable that.";

pub(crate) static ZT300AB: SelectableDevice = SelectableDevice {
    id: "zt300ab",
    display_name: "ZT-300AB / AN9002",
    aliases: &["zt-300ab", "an9002", "an-9002"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_ZT300AB,
    family: DeviceFamily::Zotek,
    new_protocol: || Box::new(ZotekProtocol::new_zt300ab()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(ZOTEK_SUPPORT_URL),
    bluetooth_only: true,
    bluetooth_names: &["Bluetooth DMM"],
};

pub(crate) static ZT5566SE: SelectableDevice = SelectableDevice {
    id: "zt5566se",
    display_name: "ZT-5566SE / AN999S",
    // Not the plain ZT-5566: its manual documents Bluetooth only as a
    // speaker.
    aliases: &["zt-5566se", "zt5566s", "zt-5566s", "an999s", "an-999s"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_ZT5566SE,
    family: DeviceFamily::Zotek,
    new_protocol: || Box::new(ZotekProtocol::new_zt5566se()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(ZOTEK_SUPPORT_URL),
    bluetooth_only: true,
    bluetooth_names: &["Bluetooth DMM"],
};

pub(crate) static ZT5BQ: SelectableDevice = SelectableDevice {
    id: "zt5bq",
    display_name: "ZT-5BQ / ST207",
    aliases: &["zt-5bq", "st207"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_ZT5BQ,
    family: DeviceFamily::Zotek,
    new_protocol: || Box::new(ZotekProtocol::new_zt5bq()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(ZOTEK_SUPPORT_URL),
    bluetooth_only: true,
    bluetooth_names: &["Bluetooth DMM"],
};

pub(crate) static ZT5B: SelectableDevice = SelectableDevice {
    id: "zt5b",
    display_name: "ZT-5B / V05B",
    aliases: &["zt-5b", "v05b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_ZT5B,
    family: DeviceFamily::Zotek,
    new_protocol: || Box::new(ZotekProtocol::new_zt5b()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some(ZOTEK_SUPPORT_URL),
    bluetooth_only: true,
    bluetooth_names: &["Bluetooth DMM"],
};
