//! The UT181A registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{FINGERPRINT, Ut181aProtocol};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};

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
    bluetooth_only: false,
    bluetooth_names: &[],
};
