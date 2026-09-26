//! The UT181A registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut181aProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};
use crate::transport::{ch9329, cp2110};

/// The links the UT181A is found on, most likely first. The CH9329 UT-D09
/// variant is sold for the UT181A series and the CP2110 UT-D09 comes with
/// older units (the cable table in `docs/supported-devices.md`); the UT-D07B
/// Bluetooth adapter names the UT181 series on UNI-T's accessory page
/// (https://meters.uni-trend.com/product/ut-d-series/, read 2026-09-22).
/// The adapter is last: it is a transparent bridge, so any meter with the
/// matching socket can sit behind it, but the cable is what is usually plugged
/// in.
const LINKS: &[&str] = &[ch9329::NAME, cp2110::NAME, crate::BLUETOOTH];

const ACTIVATION: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON
Note: this setting resets on power cycle.";

pub(crate) static UT181A: SelectableDevice = SelectableDevice {
    id: "ut181a",
    display_name: "UT181A",
    aliases: &["ut181"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Ut181a,
    new_protocol: factory::<Ut181aProtocol>,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut181a/"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};
