//! The UT171 registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut171Protocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};
use crate::transport::{ch9329, cp2110};

/// The links the UT171 is found on, most likely first. The CP2110 UT-D09
/// covers the UT171x and the CH9329 variant is sold for the UT171 series (the
/// cable table in `docs/supported-devices.md`); the UT-D07B Bluetooth adapter
/// names the UT171 series on UNI-T's accessory page
/// (https://meters.uni-trend.com/product/ut-d-series/, read 2026-09-22).
/// The adapter is last: it is a transparent bridge, so any meter with the
/// matching socket can sit behind it, but the cable is what is usually plugged
/// in.
const LINKS: &[&str] = &[cp2110::NAME, ch9329::NAME, crate::BLUETOOTH];

const ACTIVATION: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON";

pub(crate) static UT171: SelectableDevice = SelectableDevice {
    id: "ut171",
    display_name: "UT171A/B/C",
    aliases: &["ut171a", "ut171b", "ut171c"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Ut171,
    new_protocol: factory::<Ut171Protocol>,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://meters.uni-trend.com/product/ut171-series/"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};
