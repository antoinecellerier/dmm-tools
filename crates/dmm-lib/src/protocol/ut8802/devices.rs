//! The UT8802 registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut8802Protocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};
use crate::protocol::ut8803;
use crate::transport::cp2110;

/// The link the UT8802 is found on: the CP2110 UT-D09 covers the UT880x
/// (the cable table in `docs/supported-devices.md`).
const LINKS: &[&str] = &[cp2110::NAME];

pub(crate) static UT8802: SelectableDevice = SelectableDevice {
    id: "ut8802",
    display_name: "UT8802",
    aliases: &["ut8802n"],
    requires_hardware: true,
    activation_instructions: ut8803::devices::ACTIVATION, // same setup as UT8803
    family: DeviceFamily::Ut8802,
    new_protocol: factory::<Ut8802Protocol>,
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT8802"),
    links: LINKS,
    bluetooth_only: false,
    bluetooth_names: &[],
};
