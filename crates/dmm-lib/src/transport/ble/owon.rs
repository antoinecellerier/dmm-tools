//! OWON's GATT profile for its Bluetooth meters
//! (`docs/research/owon/reverse-engineered-protocol.md` §2): readings are
//! notified on one characteristic and key presses written to another, in
//! the FFF0 service, and a third holds the meter's device information (§4),
//! read once before the subscribe. What its bytes say is the protocol's.
//!
//! No challenge is written (§3.4): OWON's PC software never sends one and
//! parses readings (§3.3), and every community client streams without it,
//! up to a B41T+ on firmware 0.1.2 (§14.4).

use super::profile::{BringUp, GattProfile};
use btleplug::api::CharPropFlags;

/// The service the four characteristics sit in (§2).
const SERVICE: &str = "0000fff0-0000-1000-8000-00805f9b34fb";
/// Device information, read once at bring-up (§2, §4).
const INFO: &str = "0000fff2-0000-1000-8000-00805f9b34fb";
/// Host → meter: key presses are written here (§2, §7.1).
const KEYS: &str = "0000fff3-0000-1000-8000-00805f9b34fb";
/// Meter → host: notifications carry every byte the meter sends (§2).
const STREAM: &str = "0000fff4-0000-1000-8000-00805f9b34fb";

/// The profile: the radio is the meter's own, so nothing is stripped from
/// the stream.
pub(super) const OWON: GattProfile = GattProfile {
    name: "OWON",
    service: SERVICE,
    notify: STREAM,
    write: KEYS,
    // FFF0 is a common service on generic modules, so it is held to what
    // the stream needs: notifications, as a B35T+ lists (§14.4), or
    // indications, which btleplug's subscribe takes either of, and a write
    // of either kind: the apps acknowledge theirs, a B41T+ acts on an
    // unacknowledged one (§2, §14.4).
    notify_needs: CharPropFlags::NOTIFY.union(CharPropFlags::INDICATE),
    write_needs: CharPropFlags::WRITE.union(CharPropFlags::WRITE_WITHOUT_RESPONSE),
    always_unacknowledged: false,
    // Both OWON programs read it before anything else (§3.2, §3.3).
    bring_up: BringUp::ReadInfo(INFO),
    min_mtu: None,
    strips_adapter_heartbeat: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The UUIDs are the 16-bit ones both OWON programs use, on the
    /// Bluetooth base UUID (§2); a typo here would make every open of such a
    /// meter fail to find its characteristics.
    #[test]
    fn uuids_match_the_research_doc() {
        assert_eq!(SERVICE, "0000fff0-0000-1000-8000-00805f9b34fb");
        assert_eq!(INFO, "0000fff2-0000-1000-8000-00805f9b34fb");
        assert_eq!(KEYS, "0000fff3-0000-1000-8000-00805f9b34fb");
        assert_eq!(STREAM, "0000fff4-0000-1000-8000-00805f9b34fb");
        assert_eq!(OWON.bring_up, BringUp::ReadInfo(INFO));
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [SERVICE, INFO, KEYS, STREAM] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }
}
