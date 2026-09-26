//! The EEVblog 121GW's own GATT profile
//! (`docs/research/121gw/reverse-engineered-protocol.md` §2): one
//! characteristic, in a service of the meter's own, carries both directions.

use super::GattProfile;

/// The service the data characteristic sits in (§2).
const SERVICE: &str = "0bd51666-e7cb-469b-8e4d-2742f1ba77cc";
/// Both directions: indications carry the meter's packets, and key presses
/// are written to the same characteristic (§2, §3).
const DATA_CHARACTERISTIC: &str = "e7add780-b042-4876-aae1-112855353cc1";

/// The profile: the radio is the meter's own, so nothing is stripped from
/// the stream.
pub(super) const EEVBLOG_121GW: GattProfile = GattProfile {
    name: "EEVblog 121GW",
    service: SERVICE,
    notify: DATA_CHARACTERISTIC,
    write: DATA_CHARACTERISTIC,
    strips_adapter_heartbeat: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The UUIDs are the ones UEi's app uses; a typo here would make
    /// every open of the meter fail to find its data characteristic.
    #[test]
    fn uuids_match_the_research_doc() {
        assert_eq!(SERVICE, "0bd51666-e7cb-469b-8e4d-2742f1ba77cc");
        assert_eq!(DATA_CHARACTERISTIC, "e7add780-b042-4876-aae1-112855353cc1");
        assert_eq!(EEVBLOG_121GW.notify, EEVBLOG_121GW.write);
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [SERVICE, DATA_CHARACTERISTIC] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }
}
