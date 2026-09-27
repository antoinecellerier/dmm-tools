//! The registry entry of Brymen's meters on the BU-86X cable; [`DEVICES`]
//! orders it.
//!
//! One entry per series: each has its own request and model bytes (spec
//! §1.1, §4.2), and nothing tells the models of a series apart (spec §4.2).
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{Bm86xProtocol, FINGERPRINT_86, Series};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;
use crate::transport::bu86x;

/// The BM860s manual p.13 (the cable at the optical port, "BU-86X"), p.14
/// and p.17 (auto power-off after about 17 minutes, SELECT at power-on to
/// disable it). No key or menu step starts the output (spec §1.2).
/// Detection sends the reading request, which a meter in capacitance can
/// take longer to answer than it listens (spec §3.3).
const ACTIVATION: &str = "\
1. Attach the BU-86X cable to the optical PC-Comm port at the back of the meter and plug it in
2. Turn the meter on; there is nothing to press on the meter
Note: the meter switches off after about 17 minutes idle; hold SELECT while turning it on to disable that.
For auto-detection, set a voltage function.";

pub(crate) static BM86X: SelectableDevice = SelectableDevice {
    id: Series::Bm86x.id(),
    display_name: "Brymen BM869s/BM867s",
    aliases: &["bm869s", "bm867s", "brymen-bm869s", "brymen-bm867s"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Bm86x,
    new_protocol: || Box::new(Bm86xProtocol::new(Series::Bm86x, BM86X.display_name)),
    fingerprint: Some(&FINGERPRINT_86),
    manual_url: Some(
        "http://www.brymen.com/images/ProductsList/BM860s_List/BM860s-manual-2-033Ed2-ES636-Print1.pdf",
    ),
    // The manual names this cable (spec §1.2); it relays no UART, so the
    // meter is on no other.
    links: &[bu86x::NAME],
    bluetooth_names: &[],
};
