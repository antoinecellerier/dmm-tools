//! The Voltcraft VC-880 and VC-890 families' registry entries; [`DEVICES`]
//! orders them.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::vc880::Vc880Protocol;
use super::vc890::Vc890Protocol;
use super::{VC880_FINGERPRINT, VC890_FINGERPRINT};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{SelectableDevice, factory};

const ACTIVATION: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Press the PC button on the meter";

pub(crate) static VC880: SelectableDevice = SelectableDevice {
    id: "vc880",
    display_name: "Voltcraft VC-880",
    aliases: &["vc-880"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Vc880,
    new_protocol: factory::<Vc880Protocol>,
    fingerprint: Some(&VC880_FINGERPRINT),
    manual_url: Some(
        "https://www.conrad.com/p/voltcraft-vc880-handheld-multimeter-digital-calibrated-to-manufacturers-standards-no-certificate-data-logger-cat-iii-124609",
    ),
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static VC650BT: SelectableDevice = SelectableDevice {
    id: "vc650bt",
    display_name: "Voltcraft VC650BT",
    aliases: &["vc-650bt"],
    requires_hardware: true,
    activation_instructions: ACTIVATION, // same protocol as VC-880
    family: DeviceFamily::Vc880,
    new_protocol: || Box::new(Vc880Protocol::for_model("Voltcraft VC650BT")),
    fingerprint: Some(&VC880_FINGERPRINT),
    manual_url: Some(
        "https://www.conrad.com/p/voltcraft-vc650bt-bench-multimeter-digital-cat-ii-600-v-display-counts-40000-124411",
    ),
    bluetooth_only: false,
    bluetooth_names: &[],
};

pub(crate) static VC890: SelectableDevice = SelectableDevice {
    id: "vc890",
    display_name: "Voltcraft VC-890",
    aliases: &["vc-890"],
    requires_hardware: true,
    activation_instructions: ACTIVATION, // same activation as VC-880
    family: DeviceFamily::Vc890,
    new_protocol: factory::<Vc890Protocol>,
    fingerprint: Some(&VC890_FINGERPRINT),
    manual_url: Some(
        "https://www.conrad.com/p/voltcraft-vc890-oled-hand-multimeter-digital-oled-display-data-logger-cat-iii-1000-v-cat-iv-600-v-display-counts-60000-124600",
    ),
    bluetooth_only: false,
    bluetooth_names: &[],
};
