//! The EEVblog 121GW registry entry; [`DEVICES`] orders it.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{Eevblog121gwProtocol, FINGERPRINT, ID};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;

/// Manual p.55 and p.33 (Bluetooth), p.61 (auto power-off).
const ACTIVATION: &str = "\
1. Turn the dial to any function
2. Hold 1ms PEAK until BT shows on the display
Note: the meter switches off after 30 minutes; set APO.oF in SETUP to disable that.";

pub(crate) static EEVBLOG_121GW: SelectableDevice = SelectableDevice {
    id: ID,
    display_name: "EEVblog 121GW",
    aliases: &["eevblog121gw", "eevblog-121gw"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Eevblog121gw,
    new_protocol: || Box::new(Eevblog121gwProtocol::new()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("https://www.eevblog.com/files/EEVblog-121GW-Manual.pdf"),
    // Bluetooth built in, no cable (manual p.15). The name is the one a
    // meter was seen advertising (spec §15.4); EEVblog's app also accepts
    // "Bluegiga", which no meter was seen to send.
    links: &[crate::BLUETOOTH],
    bluetooth_names: &["121GW"],
};
