//! The UT61+/UT161 family's registry entries; [`DEVICES`] orders them.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut61PlusProtocol};
use crate::protocol::registry::SelectableDevice;
use crate::protocol::{DeviceFamily, Protocol};
use crate::transport::{ch9329, cp2110};

/// Build a UT61+/UT161 protocol for one specific model.
///
/// Every entry in the family goes through here rather than sharing a factory,
/// so a meter reports the model the user actually selected. The UT161x models
/// reuse their UT61x+ counterpart's table, and deriving the profile from the
/// table would make a UT161E introduce itself as a verified UT61E+.
macro_rules! ut61_family_factory {
    ($name:ident, $model:literal) => {
        fn $name() -> Box<dyn Protocol> {
            Box::new(
                Ut61PlusProtocol::for_model($model)
                    .expect(concat!($model, " must be a known UT61+/UT161 model")),
            )
        }
    };
}

ut61_family_factory!(new_ut61eplus, "ut61e+");
ut61_family_factory!(new_ut61bplus, "ut61b+");
ut61_family_factory!(new_ut61dplus, "ut61d+");
ut61_family_factory!(new_ut161e, "ut161e");
ut61_family_factory!(new_ut161b, "ut161b");
ut61_family_factory!(new_ut161d, "ut161d");
ut61_family_factory!(new_ut60bt, "ut60bt");
ut61_family_factory!(new_ut202bt, "ut202bt");

/// The links the UT61+/UT161 meters are found on, most likely first. The
/// CP2110 UT-D09 covers the UT61x+ and UT161x (the cable table in
/// `docs/supported-devices.md`), and the CH9329 variant is confirmed on a
/// UT61B+ (issue #19). The UT-D07B Bluetooth adapter names the UT61+ and
/// UT161 series on UNI-T's accessory page
/// (https://meters.uni-trend.com/product/ut-d-series/, read 2026-09-22).
/// The adapter is last: it is a transparent bridge, so any meter with the
/// matching socket can sit behind it, but the cable is what is usually plugged
/// in.
const LINKS: &[&str] = &[cp2110::NAME, ch9329::NAME, crate::BLUETOOTH];

const ACTIVATION_UT61EPLUS: &str = "\
1. Insert the USB module into the meter
2. Turn the meter on
3. Long press the USB/Hz button
4. The S icon appears on the LCD";

/// UT60BT manual §VIII and §11: long-pressing SEL switches the radio on and
/// shows the Bluetooth symbol, which flashes once an app has connected.
const ACTIVATION_UT60BT: &str = "\
1. Turn the meter on
2. Long press SEL until the Bluetooth symbol shows";

/// UT202T/UT202BT manual P13/24 and P14/25: the symbol flashes until an app
/// connects, then stays on; the radio switches itself off after 5 minutes
/// without a connection.
const ACTIVATION_UT202BT: &str = "\
1. Turn the meter on
2. Short press the Bluetooth button; the Bluetooth symbol flashes
Note: Bluetooth turns itself off after 5 minutes without a connection.";

pub(crate) static UT61EPLUS: SelectableDevice = SelectableDevice {
    id: "ut61eplus",
    display_name: "UT61E+",
    aliases: &["ut61e+", "ut61e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut61eplus,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT61BPLUS: SelectableDevice = SelectableDevice {
    id: "ut61b+",
    display_name: "UT61B+",
    aliases: &["ut61bplus", "ut61b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut61bplus,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT61DPLUS: SelectableDevice = SelectableDevice {
    id: "ut61d+",
    display_name: "UT61D+",
    aliases: &["ut61dplus", "ut61d"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut61dplus,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT161B: SelectableDevice = SelectableDevice {
    id: "ut161b",
    display_name: "UT161B",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut161b, // same table as UT61B+
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT161D: SelectableDevice = SelectableDevice {
    id: "ut161d",
    display_name: "UT161D",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut161d, // same table as UT61D+
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT161E: SelectableDevice = SelectableDevice {
    id: "ut161e",
    display_name: "UT161E",
    aliases: &["ut161"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut161e, // same table as UT61E+
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    bluetooth_names: &[],
};

pub(crate) static UT60BT: SelectableDevice = SelectableDevice {
    id: "ut60bt",
    display_name: "UT60BT",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT60BT,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut60bt,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com.cn/content/1298.html"),
    // Bluetooth built in, no cable.
    links: &[crate::BLUETOOTH],
    // One UT60BT advertises `UT60BTk` (docs/research/new-device-candidates.md).
    bluetooth_names: &["UT60BT"],
};

pub(crate) static UT202BT: SelectableDevice = SelectableDevice {
    id: "ut202bt",
    display_name: "UT202BT",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT202BT,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: new_ut202bt,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com.cn/content/1341.html"),
    // Bluetooth built in, no cable.
    links: &[crate::BLUETOOTH],
    bluetooth_names: &["UT202BT"],
};
