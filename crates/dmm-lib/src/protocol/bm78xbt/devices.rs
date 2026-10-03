//! The Brymen BM78xBT registry entry; [`DEVICES`] orders it.
//!
//! One entry for the BM788BT and the BM787BT: nothing on the wire tells the
//! two apart (spec §1), and the tables hold what either sends.
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{Bm78xbtProtocol, FINGERPRINT, ID};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::{Brand, SelectableDevice};

/// BM788BT manual p.19 (Bluetooth), p.5 (what works in AutoV), p.18-19
/// and p.23 (auto power-off: 30 minutes in the text, 15 in the
/// specifications), p.20 (the reset). Any phone app goes first: whether the
/// meter takes a second connection is not stated (spec §2).
const ACTIVATION: &str = concat!(
    "\
1. Disconnect the meter from any phone app
2. Turn the dial to any function but Auto V/LoZ
3. Hold Δ for one second or more, until ((D)) shows on the display
Note: the meter switches off after 15 to 30 minutes idle; hold SELECT while turning it on to disable that (dSAPO shows).
If the meter is not found or its connection password was changed, ",
    reset_gesture!(),
    "."
);

pub(crate) static BM78XBT: SelectableDevice = SelectableDevice {
    id: ID,
    display_name: "Brymen BM788BT/BM787BT",
    brand: Brand::Brymen,
    aliases: &["bm788bt", "bm787bt", "brymen-bm788bt", "brymen-bm787bt"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Bm78xbt,
    new_protocol: || Box::new(Bm78xbtProtocol::new()),
    fingerprint: Some(&FINGERPRINT),
    manual_url: Some("http://www.brymen.com/images/ProductsList/BM788BT_List/BM780(BT)-Print2.pdf"),
    // Bluetooth built in, no cable (BM788BT manual p.19). The name is r4's
    // default, which the owner can change (spec §2); a renamed meter opens
    // with `--device bm78xbt --adapter <address>`.
    links: &[crate::BLUETOOTH],
    detection_verified: &[],
    bluetooth_names: &["BM78xBT"],
};
