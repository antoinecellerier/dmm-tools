//! The UT803/UT804 family's registry entries, the UT71 and Voltcraft
//! VC920/VC940/VC960 among them; [`DEVICES`] orders them.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut80xProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;
use crate::transport::ch9325;

/// The link the UT80x meters are found on. The CH9325 UT-D04 is what the
/// UT803/UT804 use (the cable table in `docs/supported-devices.md`), and the
/// UT71 and Voltcraft VC9x0, which send the UT804's packets, are taken to use
/// it too (docs/research/ut71/reverse-engineered-protocol.md §1). No
/// Bluetooth: UNI-T's accessory page lists the UT71 for the UT-D07A only
/// (https://meters.uni-trend.com/product/ut-d-series/, read 2026-09-22),
/// whose GATT layout we have not seen.
const LINKS: &[&str] = &[ch9325::NAME];

/// The UT803 manual's RS232 button starts and stops the data output.
const ACTIVATION_UT803: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Press RS232; the display shows RS232";

/// The UT804 sends nothing until SEND is pressed, off at power-on, and
/// nothing while HOLD is on; EXIT turns SEND off (#16, UT804 manual
/// Table 2-2).
const ACTIVATION_UT804: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Press SEND; the display shows SEND
Note: HOLD pauses the data, and EXIT turns SEND off.";

/// The UT71 and the Voltcraft VC920/VC940/VC960 send nothing until MAX MIN
/// is held for over a second, or the UT71A's SEND key is pressed, and the
/// display shows SEND; EXIT turns it off (UT71 manual Table 2-2,
/// VC920/940/960 manual §7). One text for all three entries, so the "no
/// meter answered" help lists them once.
const ACTIVATION_UT71: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Hold MAX MIN for 1 s, or press SEND on a UT71A; the display shows SEND
Note: EXIT turns SEND off.";

pub(crate) static UT803: SelectableDevice = SelectableDevice {
    id: "ut803",
    display_name: "UT803",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT803,
    family: DeviceFamily::Ut80x,
    new_protocol: || Box::new(Ut80xProtocol::new_ut803()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT803"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static UT804: SelectableDevice = SelectableDevice {
    id: "ut804",
    display_name: "UT804",
    aliases: &[],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT804,
    family: DeviceFamily::Ut80x,
    new_protocol: || Box::new(Ut80xProtocol::new_ut804()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT804"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static UT71AB: SelectableDevice = SelectableDevice {
    id: "ut71ab",
    display_name: "UT71A/B",
    aliases: &["ut71a", "ut71b"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT71,
    family: DeviceFamily::Ut80x,
    new_protocol: || Box::new(Ut80xProtocol::new_ut71ab()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut71-series/"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static UT71CDE: SelectableDevice = SelectableDevice {
    id: "ut71cde",
    display_name: "UT71C/D/E",
    aliases: &["ut71c", "ut71d", "ut71e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT71,
    family: DeviceFamily::Ut80x,
    new_protocol: || Box::new(Ut80xProtocol::new_ut71cde()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut71-series/"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static VC920: SelectableDevice = SelectableDevice {
    id: "vc920",
    display_name: "Voltcraft VC920/VC940/VC960",
    aliases: &["vc-920", "vc940", "vc-940", "vc960", "vc-960"],
    requires_hardware: true,
    activation_instructions: ACTIVATION_UT71, // same keys as the UT71
    family: DeviceFamily::Ut80x,
    new_protocol: || Box::new(Ut80xProtocol::new_vc920()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://asset.conrad.com/media10/add/160267/c1/-/gl/000123296ML04"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};
