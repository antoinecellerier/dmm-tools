//! The registry entries of Brymen's meters on the BU-86X cable; [`DEVICES`]
//! orders them.
//!
//! One entry per series: each has its own request and model bytes (spec
//! §1.1, §4.2), and nothing tells the models of a series apart (spec §4.2).
//!
//! [`DEVICES`]: crate::protocol::registry::DEVICES

use super::{Bm86xProtocol, FINGERPRINT_52, FINGERPRINT_82, FINGERPRINT_86, Series};
use crate::protocol::DeviceFamily;
use crate::protocol::registry::SelectableDevice;
use crate::transport::bu86x;

/// One text for the three series, so the help for a silent cable prints it
/// once. The BM860s manual p.13 (the cable at the optical port, "BU-86X"),
/// p.14 and p.17 (auto power-off after about 17 minutes, SELECT at power-on
/// to disable it); the BM820s manual p.14 and p.19 (about 30 minutes, the
/// same key). No key or menu step starts the output (spec §1.2, §11.6).
/// Detection sends the reading request, which a meter in capacitance can
/// take longer to answer than it listens (spec §3.3).
const ACTIVATION: &str = "\
1. Attach the BU-86X cable to the optical PC-Comm port at the back of the meter and plug it in
2. Turn the meter on at a voltage function
Note: the meter switches off after about 17 minutes idle on a BM869s or BM867s, 30 minutes on the others; hold SELECT while turning it on to disable that.";

/// The BM820s and BM520s share one manual (spec §1.1).
const BM820S_520S_MANUAL: &str =
    "http://www.brymen.com/images/ProductsList/BM820s_List/BM820s-520s-Print1(033Ed2).pdf";

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

pub(crate) static BM82X: SelectableDevice = SelectableDevice {
    id: Series::Bm82x.id(),
    display_name: "Brymen BM829s/BM827s/BM822s/BM821s",
    aliases: &[
        "bm829s",
        "bm827s",
        "bm822s",
        "bm821s",
        "brymen-bm829s",
        "brymen-bm827s",
        "brymen-bm822s",
        "brymen-bm821s",
    ],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Bm82x,
    new_protocol: || Box::new(Bm86xProtocol::new(Series::Bm82x, BM82X.display_name)),
    fingerprint: Some(&FINGERPRINT_82),
    manual_url: Some(BM820S_520S_MANUAL),
    // The manual names the BU-86X once and the BU-82X once, Brymen's
    // program for the series the BC-86X cable alone (spec §1.2).
    links: &[bu86x::NAME],
    bluetooth_names: &[],
};

pub(crate) static BM52X: SelectableDevice = SelectableDevice {
    id: Series::Bm52x.id(),
    display_name: "Brymen BM525s/BM521s",
    aliases: &["bm525s", "bm521s", "brymen-bm525s", "brymen-bm521s"],
    requires_hardware: true,
    activation_instructions: ACTIVATION,
    family: DeviceFamily::Bm52x,
    new_protocol: || Box::new(Bm86xProtocol::new(Series::Bm52x, BM52X.display_name)),
    fingerprint: Some(&FINGERPRINT_52),
    manual_url: Some(BM820S_520S_MANUAL),
    links: &[bu86x::NAME],
    bluetooth_names: &[],
};

/// The entry that opens `series`, which an error names to the user.
pub(super) fn entry(series: Series) -> &'static SelectableDevice {
    match series {
        Series::Bm86x => &BM86X,
        Series::Bm82x => &BM82X,
        Series::Bm52x => &BM52X,
    }
}
