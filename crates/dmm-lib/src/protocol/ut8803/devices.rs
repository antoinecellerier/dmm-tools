//! The UT8803 registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut8803Protocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};
use crate::transport::cp2110;

/// The link the UT8803 is found on: the CP2110 UT-D09 covers the UT880x
/// (the cable table in `docs/supported-devices.md`).
const LINKS: &[&str] = &[cp2110::NAME];

pub(crate) const ACTIVATION: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on";

pub(crate) static UT8803: SelectableDevice = SelectableDevice {
    id: "ut8803",
    display_name: "UT8803",
    aliases: &["ut8803e"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Ut8803,
    new_protocol: factory::<Ut8803Protocol>,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT8803E"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};
