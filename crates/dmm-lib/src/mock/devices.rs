//! The mock's registry entry; [`DEVICES`] orders it among the others.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::MockProtocol;
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};

pub(crate) const ACTIVATION: &str = "No setup required \u{2014} this is a simulated device.";

pub(crate) static MOCK: SelectableDevice = SelectableDevice {
    id: "mock",
    display_name: "Mock (simulated)",
    aliases: &[],
    requires_hardware: false,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Mock,
    new_protocol: factory::<MockProtocol>,
    fingerprint: None,
    manual_url: Some(
        "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/cli-reference.md#mock-modes",
    ),
    bluetooth_only: false,
    bluetooth_names: &[],
};
