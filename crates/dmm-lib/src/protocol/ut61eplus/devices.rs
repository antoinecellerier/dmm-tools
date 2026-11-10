//! The UT61+/UT161 family's registry entries; [`DEVICES`] orders them.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::specs::SpecModel;
use super::{FINGERPRINT, Ut61PlusProtocol, tables};
use crate::protocol::registry::{Brand, SelectableDevice};
use crate::protocol::{DeviceFamily, Stability};
use crate::transport::{ch9329, cp2110};

/// The links the UT61+/UT161 meters are found on, most likely first. The
/// CP2110 UT-D09 covers the UT61x+ and UT161x (the cable table in
/// `docs/supported-devices.md`), and the CH9329 variant is confirmed on a
/// UT61B+ (issue #19). The UT-D07B Bluetooth adapter names the UT61+ and
/// UT161 series on UNI-T's accessory page
/// (<https://meters.uni-trend.com/product/ut-d-series/>, read 2026-09-22).
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
    model: "UT61E+",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &["ut61e+", "ut61e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut61e_plus()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    // Our UT61E+, on its CP2110 cable and through a UT-D07B.
    detection_verified: &[cp2110::NAME, crate::BLUETOOTH],
    bluetooth_names: &[],
};

pub(crate) static UT61BPLUS: SelectableDevice = SelectableDevice {
    id: "ut61b+",
    model: "UT61B+",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &["ut61bplus", "ut61b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut61b_plus()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    // Issue #19.
    detection_verified: &[ch9329::NAME],
    bluetooth_names: &[],
};

pub(crate) static UT61DPLUS: SelectableDevice = SelectableDevice {
    id: "ut61d+",
    model: "UT61D+",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &["ut61dplus", "ut61d"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut61d_plus()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    links: LINKS,
    detection_verified: &[],
    bluetooth_names: &[],
};

pub(crate) static UT161B: SelectableDevice = SelectableDevice {
    id: "ut161b",
    model: "UT161B",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut161b()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    detection_verified: &[],
    bluetooth_names: &[],
};

pub(crate) static UT161D: SelectableDevice = SelectableDevice {
    id: "ut161d",
    model: "UT161D",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut161d()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    detection_verified: &[],
    bluetooth_names: &[],
};

pub(crate) static UT161E: SelectableDevice = SelectableDevice {
    id: "ut161e",
    model: "UT161E",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &["ut161"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT61EPLUS,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut161e()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    links: LINKS,
    // Issue #7, on both cables.
    detection_verified: &[cp2110::NAME, ch9329::NAME],
    bluetooth_names: &[],
};

pub(crate) static UT60BT: SelectableDevice = SelectableDevice {
    id: "ut60bt",
    model: "UT60BT",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT60BT,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut60bt()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com.cn/content/1298.html"),
    // Bluetooth built in, no cable.
    links: &[crate::BLUETOOTH],
    detection_verified: &[],
    // One UT60BT advertises `UT60BTk` (docs/research/ut61-family/
    // reverse-engineering-approach.md, cross-reference).
    bluetooth_names: &["UT60BT"],
};

pub(crate) static UT202BT: SelectableDevice = SelectableDevice {
    id: "ut202bt",
    model: "UT202BT",
    brand: Brand::UniT,
    sold_as: &[],
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT202BT,
    family: DeviceFamily::Ut61EPlus,
    new_protocol: || Box::new(ut202bt()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com.cn/content/1341.html"),
    // Bluetooth built in, no cable.
    links: &[crate::BLUETOOTH],
    detection_verified: &[],
    bluetooth_names: &["UT202BT"],
};

// Each model's protocol: its table, stability, spec tables and verification
// issue. The family issue (#7) stays on every model but the UT61E+, verified
// or not: the UT61B+ is decoded
// correctly everywhere it was looked at, and its range rungs above the ones
// auto-ranging reached are still open there. The UT60BT and UT202BT have
// issues of their own.

pub(super) fn ut61e_plus() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT61EPLUS,
        &tables::ut61e_plus::UT61E_PLUS,
        Stability::Verified,
        SpecModel::Ut61ePlus,
        None,
    )
}

/// Verified by three captures reported in issue #19, 2026-09-09 to
/// 2026-09-11: every mode its dial reaches decoded correctly, every command
/// moved the flag it should, the second run passed the gate outright and the
/// third walked the Ω and DC V ladders. What is left open on this model is in
/// `docs/research/ut61-family/verification.md`.
pub(super) fn ut61b_plus() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT61BPLUS,
        &tables::ut61b_plus::UT61B_PLUS,
        Stability::Verified,
        SpecModel::Ut61bPlus,
        Some(7),
    )
}

pub(super) fn ut61d_plus() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT61DPLUS,
        &tables::ut61d_plus::UT61D_PLUS,
        Stability::Experimental,
        SpecModel::Ut61dPlus,
        Some(7),
    )
}

/// Same table as the UT61B+.
pub(super) fn ut161b() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT161B,
        &tables::ut61b_plus::UT61B_PLUS,
        Stability::Experimental,
        SpecModel::Ut161b,
        Some(7),
    )
}

/// Same table as the UT61D+.
pub(super) fn ut161d() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT161D,
        &tables::ut61d_plus::UT61D_PLUS,
        Stability::Experimental,
        SpecModel::Ut161d,
        Some(7),
    )
}

/// Same table as the UT61E+. Verified by two captures reported in issue #7
/// (@frixit, CP2110), both 2026-10-21: its frames carried the UT61E+'s mode,
/// range and flag bytes except after one AC+DC V range press (family spec
/// §2.2), and the second run passed the gate and drove every command. What
/// is left open on this model is in
/// `docs/research/ut61-family/verification.md`.
pub(super) fn ut161e() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT161E,
        &tables::ut61e_plus::UT61E_PLUS,
        Stability::Verified,
        SpecModel::Ut161e,
        Some(7),
    )
}

pub(super) fn ut60bt() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT60BT,
        &tables::ut60bt::UT60BT,
        Stability::Experimental,
        SpecModel::Untranscribed,
        Some(26),
    )
}

pub(super) fn ut202bt() -> Ut61PlusProtocol {
    Ut61PlusProtocol::model(
        &UT202BT,
        &tables::ut202bt::UT202BT,
        Stability::Experimental,
        SpecModel::Untranscribed,
        Some(27),
    )
}

/// The function that builds a model's protocol.
#[cfg(any(test, feature = "test-support"))]
pub(super) type Build = fn() -> Ut61PlusProtocol;

/// Every entry with the function that builds its protocol, in registry order,
/// for what needs the protocol's own type rather than a `dyn Protocol`.
#[cfg(any(test, feature = "test-support"))]
pub(super) static MODELS: &[(&SelectableDevice, Build)] = &[
    (&UT61EPLUS, ut61e_plus),
    (&UT61BPLUS, ut61b_plus),
    (&UT61DPLUS, ut61d_plus),
    (&UT161B, ut161b),
    (&UT161D, ut161d),
    (&UT161E, ut161e),
    (&UT60BT, ut60bt),
    (&UT202BT, ut202bt),
];
