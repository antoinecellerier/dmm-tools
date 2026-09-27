//! Brymen's own GATT profile for the BM78xBT meters, and the GATT steps of
//! the password login they need before they stream
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §2, §3).
//!
//! Readings are notified on one characteristic; commands are written to
//! another as 32-byte packets, and their replies read back from it (§3.2,
//! §5). The packets, and what a reply says, are the protocol's
//! (`protocol/bm78xbt/login.rs`).

use super::{BringUp, GattProfile, SetupFailure, link_error};
use crate::error::Result;
use crate::protocol::bm78xbt::login::{self, LoginReply, Mac, UNKNOWN_MAC};
use crate::protocol::bm78xbt::packet::NOTIFICATION_LEN;
use btleplug::api::{BDAddr, CharPropFlags, Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use log::{debug, trace};
use std::time::Duration;

/// The meter's service (§2).
const SERVICE: &str = "0003cdd0-0000-1000-8000-00805f9b0131";
/// Meter → host: notifications carry the readings (§2).
const READING_CHARACTERISTIC: &str = "0003cdd5-0000-1000-8000-00805f9b0131";
/// Host → meter: 32-byte commands are written here, and their replies read
/// back (§2, §3.2).
const COMMAND_CHARACTERISTIC: &str = "0003cdd4-0000-1000-8000-00805f9b0131";

/// The profile: the radio is the meter's own, so nothing is stripped from
/// the stream.
pub(super) const BRYMEN: GattProfile = GattProfile {
    name: "Brymen BM78xBT",
    service: SERVICE,
    notify: READING_CHARACTERISTIC,
    write: COMMAND_CHARACTERISTIC,
    // The readings come by notification, and the command characteristic
    // has to take an acknowledged write, the only kind the login sends;
    // whether it reads, for the login's reply, is left to the login (§2,
    // §3.2).
    notify_needs: CharPropFlags::NOTIFY,
    write_needs: CharPropFlags::WRITE,
    always_unacknowledged: false,
    bring_up: BringUp::BrymenLogin,
    min_mtu: Some(WHOLE_OUTPUT_MTU),
    strips_adapter_heartbeat: false,
};

/// The pause between the login and the subscribe.
pub(super) const SETTLE: Duration = Duration::from_millis(500);

/// The smallest ATT MTU whose notifications carry the meter's 152-byte
/// output whole: 152 bytes and the 3-byte notification header (§4).
const WHOLE_OUTPUT_MTU: u16 = NOTIFICATION_LEN as u16 + 3;

/// `address` in the wire's order: btleplug's is most significant octet
/// first, the meter's least (§5).
fn wire_mac(address: BDAddr) -> Mac {
    let mut mac = address.into_inner();
    mac.reverse();
    mac
}

/// Log in, so the meter streams once its readings are subscribed to
/// (§3.1, r4's order).
///
/// The address in the command comes from the platform, or from
/// GetBLEAddress where the platform has none (always on macOS); zeros when
/// neither gives it (§3.2). The login's reply is read at once, as the app
/// reads it (§3.2), and [`judge`]d.
///
/// Every write is acknowledged and whole, as the app sends them (§2): the
/// platform splits a packet over a small MTU, never this code.
pub(super) async fn log_in(
    peripheral: &Peripheral,
    commands: &Characteristic,
) -> std::result::Result<(), SetupFailure> {
    let mut mac = wire_mac(peripheral.address());
    if mac == UNKNOWN_MAC {
        mac = tokio::time::timeout(login::ADDRESS_TIMEOUT, ask_mac(peripheral, commands))
            .await
            .unwrap_or_else(|_| {
                debug!("Bluetooth: no reply to GetBLEAddress in time");
                None
            })
            .unwrap_or(UNKNOWN_MAC);
    }

    write(peripheral, commands, &login::log_in(mac)).await?;
    judge(read(peripheral, commands).await)
}

/// What the login's reply, or a failure to read it, means for the open.
///
/// A refusal ends it and is not sent again: the app shows an error and
/// does not retry (§3.2 step 4). Anything else lets the stream show whether
/// the meter took the login; [`login::judge`] reports a reply that is
/// neither an acceptance nor a refusal.
fn judge(reply: Result<Vec<u8>>) -> std::result::Result<(), SetupFailure> {
    match reply {
        Ok(reply) => match login::judge(&reply) {
            LoginReply::Accepted => debug!("Bluetooth: login accepted"),
            LoginReply::Refused(e) => return Err(SetupFailure::Refused(e)),
            LoginReply::Unknown => {}
        },
        Err(e) => debug!("Bluetooth: the login's reply was not read ({e}); waiting for readings"),
    }
    Ok(())
}

/// The meter's address from GetBLEAddress's reply, or `None` if any step
/// fails (§3.2).
async fn ask_mac(peripheral: &Peripheral, commands: &Characteristic) -> Option<Mac> {
    if let Err(e) = write(peripheral, commands, &login::ask_address()).await {
        debug!("Bluetooth: GetBLEAddress not sent: {e}");
        return None;
    }
    tokio::time::sleep(login::ADDRESS_WAIT).await;
    match read(peripheral, commands).await {
        Ok(reply) => {
            let mac = login::mac_from_reply(&reply);
            if mac.is_none() {
                debug!("Bluetooth: GetBLEAddress's reply carries no address");
            }
            mac
        }
        Err(e) => {
            debug!("Bluetooth: GetBLEAddress's reply not read: {e}");
            None
        }
    }
}

/// Write `packet` to the command characteristic, acknowledged and whole.
async fn write(peripheral: &Peripheral, commands: &Characteristic, packet: &[u8]) -> Result<()> {
    trace!("Bluetooth TX ({} bytes): {packet:02X?}", packet.len());
    peripheral
        .write(commands, packet, WriteType::WithResponse)
        .await
        .map_err(link_error)
}

/// Read the command characteristic, where a command's reply is left (§3.2).
async fn read(peripheral: &Peripheral, commands: &Characteristic) -> Result<Vec<u8>> {
    let reply = peripheral.read(commands).await.map_err(link_error)?;
    trace!("Bluetooth RX ({} bytes, read): {reply:02X?}", reply.len());
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::protocol::bm78xbt::login::tests::{EXAMPLE_MAC, login_accepted, login_refused};
    use crate::protocol::capture_reports;

    /// The UUIDs are the ones r4 prints; a typo here would make every open
    /// of the meter fail to find its characteristics.
    #[test]
    fn uuids_match_the_research_doc() {
        assert_eq!(SERVICE, "0003cdd0-0000-1000-8000-00805f9b0131");
        assert_eq!(
            READING_CHARACTERISTIC,
            "0003cdd5-0000-1000-8000-00805f9b0131"
        );
        assert_eq!(
            COMMAND_CHARACTERISTIC,
            "0003cdd4-0000-1000-8000-00805f9b0131"
        );
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [SERVICE, READING_CHARACTERISTIC, COMMAND_CHARACTERISTIC] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }

    /// A refusal ends the open as one, so the setup is not tried again; any
    /// other reply, or none, lets it go on.
    #[test]
    fn only_a_refusal_ends_the_login() {
        let (judged, _) = capture_reports(|| judge(Ok(login_refused(3))));
        assert!(
            matches!(judged, Err(SetupFailure::Refused(_))),
            "{judged:?}"
        );

        let (judged, _) = capture_reports(|| judge(Ok(login_accepted())));
        assert!(judged.is_ok());

        let (judged, reports) = capture_reports(|| judge(Err(Error::LinkLost)));
        assert!(judged.is_ok());
        assert!(reports.is_empty(), "{reports:?}");

        let login = login::log_in(EXAMPLE_MAC).to_vec();
        let (judged, _) = capture_reports(|| judge(Ok(login)));
        assert!(judged.is_ok());
    }

    /// btleplug's address is most significant octet first; the wire wants
    /// the least first (§5).
    #[test]
    fn the_platform_address_is_reversed() {
        let address = BDAddr::from([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        assert_eq!(address.to_string(), "11:22:33:44:55:66");
        assert_eq!(wire_mac(address), EXAMPLE_MAC);
        assert_eq!(wire_mac(BDAddr::default()), UNKNOWN_MAC);
    }
}
