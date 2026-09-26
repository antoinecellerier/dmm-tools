//! The UT171 registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut171Protocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};

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
    bluetooth_only: false,
    bluetooth_names: &[],
};
