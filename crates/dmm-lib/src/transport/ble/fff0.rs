//! The FFF0/FFF4 profile of the generic BLE modules in ZOTEK's meters
//! (`docs/research/zotek/reverse-engineered-protocol.md` §2): one
//! characteristic carries both directions.

use super::profile::{BringUp, GattProfile};
use btleplug::api::CharPropFlags;

/// The service the data characteristic sits in (§2).
const SERVICE: &str = "0000fff0-0000-1000-8000-00805f9b34fb";
/// Both directions: notifications carry the meter's packets, and commands
/// are written to the same characteristic (§2).
const DATA_CHARACTERISTIC: &str = "0000fff4-0000-1000-8000-00805f9b34fb";

/// The profile: no adapter sits in front of these meters, so nothing is
/// stripped from the stream.
pub(super) const FFF0: GattProfile = GattProfile {
    name: "FFF0",
    service: SERVICE,
    notify: DATA_CHARACTERISTIC,
    write: DATA_CHARACTERISTIC,
    // FFF0 is a common service on generic modules, so it is held to what
    // the stream needs: notifications or indications, which btleplug's
    // subscribe takes either of, as ZOTEK's current app does, and a write
    // of either kind (§2).
    notify_needs: CharPropFlags::NOTIFY.union(CharPropFlags::INDICATE),
    write_needs: CharPropFlags::WRITE.union(CharPropFlags::WRITE_WITHOUT_RESPONSE),
    always_unacknowledged: false,
    bring_up: BringUp::Subscribe,
    min_mtu: None,
    strips_adapter_heartbeat: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The UUIDs are the ones ZOTEK's apps use; a typo here would make
    /// every open of such a meter fail to find its data characteristic.
    #[test]
    fn fff0_uuids_match_the_research_doc() {
        assert_eq!(SERVICE, "0000fff0-0000-1000-8000-00805f9b34fb");
        assert_eq!(DATA_CHARACTERISTIC, "0000fff4-0000-1000-8000-00805f9b34fb");
        assert_eq!(FFF0.notify, FFF0.write);
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [SERVICE, DATA_CHARACTERISTIC] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }
}
