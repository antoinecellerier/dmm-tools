//! Brymen's own GATT profile for the BM78xBT meters, and the password login
//! they need before they stream
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §2, §3).
//!
//! Readings are notified on one characteristic; commands are written to
//! another as 32-byte packets, and their replies read back from it (§3.2,
//! §5).

use super::{GattProfile, SetupFailure, link_error};
use crate::error::{Error, Result};
use crate::protocol::bm78xbt::packet::{framed, seal};
use crate::protocol::bm78xbt::{ID, RESET_GESTURE};
use crate::protocol::unrecognised::report_unknown;
use btleplug::api::{BDAddr, Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use log::{debug, trace, warn};
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
    strips_adapter_heartbeat: false,
};

/// A command or response packet (§5).
const PACKET_LEN: usize = 32;
/// Arg0-13, bytes 14-27 (§5).
const ARGS: std::ops::Range<usize> = 14..28;
/// A command's and a response's first four bytes: head, `01`, length 32,
/// packet type [3] (§4, §5).
const COMMAND_HEAD: [u8; 4] = [0xFF, 0x01, 0x20, 0x01];
const RESPONSE_HEAD: [u8; 4] = [0xFF, 0x01, 0x20, 0x02];

/// GetBLEAddress, which Brymen's app sends before the login (§3.2, §7.3).
const GET_BLE_ADDRESS: u16 = 0x0101;
/// Verify Connection Password, the login (§7.1).
const VERIFY_PASSWORD: u16 = 0x0151;
/// Every refusal's code (§8).
const FAILURE: u16 = 0x8001;
/// The default password 0000, as binary digits (§7.2). Its digit order is
/// open for any other password (§11.5), so no other is offered.
const PASSWORD: [u8; 4] = [0; 4];

/// The meter's address in the order the wire carries it, least significant
/// octet first (§5).
type Mac = [u8; 6];
/// What goes in a command when the address is not known, as the app sends
/// GetBLEAddress (§3.2).
const UNKNOWN_MAC: Mac = [0; 6];

/// How long the app waits between GetBLEAddress and reading its reply
/// (§3.2).
const ADDRESS_WAIT: Duration = Duration::from_millis(300);
/// How long GetBLEAddress may take before the login goes on without an
/// address: the app's response timeout (§3.2).
const ADDRESS_TIMEOUT: Duration = Duration::from_secs(3);
/// The pause between the login and the subscribe.
pub(super) const SETTLE: Duration = Duration::from_millis(500);

/// The smallest ATT MTU whose notifications carry the meter's 152-byte
/// output whole: 152 bytes and the 3-byte notification header (§4).
const WHOLE_OUTPUT_MTU: u16 = 155;

/// What a reply read from the command characteristic says about `code`.
#[derive(Debug, PartialEq, Eq)]
enum Reply {
    /// The command's code came back: it was taken (§7.1).
    Accepted,
    /// 0x8001 naming the command, with the error code from Arg2-Arg3 (§8).
    Refused(u16),
    /// Not a well-formed response to `code`: another command's, the command
    /// itself still in the characteristic, or no packet at all.
    Other,
}

/// A command packet (§5): protocol version `01`, `code` low byte first in
/// [11..13], [13] = `01`, `args` from Arg0 (the rest `00`), then sealed.
fn command(mac: Mac, code: u16, args: &[u8]) -> [u8; PACKET_LEN] {
    let mut packet = [0u8; PACKET_LEN];
    packet[..4].copy_from_slice(&COMMAND_HEAD);
    packet[4] = 0x01;
    packet[5..11].copy_from_slice(&mac);
    packet[11..13].copy_from_slice(&code.to_le_bytes());
    // Password Identification: `01` in r4, and in both commands as the app
    // sends them (§5).
    packet[13] = 0x01;
    for (slot, arg) in packet[ARGS].iter_mut().zip(args) {
        *slot = *arg;
    }
    seal(&mut packet);
    packet
}

/// `reply` as a response packet: framed, typed `02` and CRC-valid (§4, §5).
fn response(reply: &[u8]) -> Option<&[u8]> {
    let packet = reply.get(..PACKET_LEN)?;
    framed(packet, RESPONSE_HEAD).then_some(packet)
}

/// What `reply` says about the command `code` (§7.1, §8).
fn classify(reply: &[u8], code: u16) -> Reply {
    let Some(packet) = response(reply) else {
        return Reply::Other;
    };
    let word = |at: usize| u16::from_le_bytes([packet[at], packet[at + 1]]);
    match word(11) {
        c if c == code => Reply::Accepted,
        // Arg0-1 name the refused command, Arg2-3 the error (§8). One that
        // names another command is an older reply still in place.
        FAILURE if word(ARGS.start) == code => Reply::Refused(word(ARGS.start + 2)),
        _ => Reply::Other,
    }
}

/// The meter's address a response carries in [5..10] (§5).
fn mac_from_reply(reply: &[u8]) -> Option<Mac> {
    response(reply)?[5..11].try_into().ok()
}

/// `address` in the wire's order: btleplug's is most significant octet
/// first, the meter's least (§5).
fn wire_mac(address: BDAddr) -> Mac {
    let mut mac = address.into_inner();
    mac.reverse();
    mac
}

/// The error for a login the meter refused with `code` (§8).
fn refused(code: u16) -> Error {
    let reason = match code {
        3 | 4 => {
            return Error::Bluetooth(format!(
                "the meter refused the connection password 0000 — to reset it, {RESET_GESTURE}"
            ));
        }
        0 => "checksum error",
        1 => "invalid channel ID",
        2 => "out of setting range",
        5 => "invalid arguments",
        6 => "insufficient permissions",
        _ => "unknown error",
    };
    Error::Bluetooth(format!(
        "the meter refused the login: {reason} (code {code})"
    ))
}

/// Log the MTU, warning when a notification cannot carry a whole output.
pub(super) fn check_mtu(mtu: u16) {
    if mtu < WHOLE_OUTPUT_MTU {
        warn!(
            "Bluetooth: the link's MTU is {mtu} bytes, under the {WHOLE_OUTPUT_MTU} the meter's \
             readings need; they may arrive cut short; if no readings arrive, report it"
        );
    } else {
        debug!("Bluetooth: MTU {mtu}");
    }
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
        mac = tokio::time::timeout(ADDRESS_TIMEOUT, ask_mac(peripheral, commands))
            .await
            .unwrap_or_else(|_| {
                debug!("Bluetooth: no reply to GetBLEAddress in time");
                None
            })
            .unwrap_or(UNKNOWN_MAC);
    }

    write(
        peripheral,
        commands,
        &command(mac, VERIFY_PASSWORD, &PASSWORD),
    )
    .await?;
    judge(read(peripheral, commands).await)
}

/// What the login's reply, or a failure to read it, means for the open.
///
/// A refusal ends it and is not sent again: the app shows an error and
/// does not retry (§3.2 step 4). Anything else lets the stream show whether
/// the meter took the login; a reply that is neither an acceptance nor a
/// refusal is reported, as data the spec does not cover.
fn judge(reply: Result<Vec<u8>>) -> std::result::Result<(), SetupFailure> {
    match reply {
        Ok(reply) => match classify(&reply, VERIFY_PASSWORD) {
            Reply::Accepted => debug!("Bluetooth: login accepted"),
            Reply::Refused(code) => return Err(SetupFailure::Refused(refused(code))),
            Reply::Other => report_unknown(ID, "login reply", format_args!("{reply:02X?}")),
        },
        Err(e) => debug!("Bluetooth: the login's reply was not read ({e}); waiting for readings"),
    }
    Ok(())
}

/// The meter's address from GetBLEAddress's reply, or `None` if any step
/// fails (§3.2).
async fn ask_mac(peripheral: &Peripheral, commands: &Characteristic) -> Option<Mac> {
    let ask = command(UNKNOWN_MAC, GET_BLE_ADDRESS, &[]);
    if let Err(e) = write(peripheral, commands, &ask).await {
        debug!("Bluetooth: GetBLEAddress not sent: {e}");
        return None;
    }
    tokio::time::sleep(ADDRESS_WAIT).await;
    match read(peripheral, commands).await {
        Ok(reply) => {
            let mac = mac_from_reply(&reply);
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
    use crate::protocol::bm78xbt::packet::crc16;
    use crate::protocol::capture_reports;

    /// Parse the spec's space-separated hex.
    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// The MAC of the spec's example 2, 11:22:33:44:55:66 (invented), in
    /// the wire's order (§10).
    const EXAMPLE_MAC: Mac = [0x66, 0x55, 0x44, 0x33, 0x22, 0x11];

    /// A response to `code` from the example's meter, with `args`.
    fn reply(code: u16, args: &[u8]) -> Vec<u8> {
        let mut packet = command(EXAMPLE_MAC, code, args);
        packet[..4].copy_from_slice(&RESPONSE_HEAD);
        seal(&mut packet);
        packet.to_vec()
    }

    /// A refusal of `code` with error `error` (§8).
    fn refusal(code: u16, error: u16) -> Vec<u8> {
        let [c0, c1] = code.to_le_bytes();
        let [e0, e1] = error.to_le_bytes();
        reply(FAILURE, &[c0, c1, e0, e1])
    }

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

    /// Both commands come out byte for byte as the spec's example 3 (§10).
    #[test]
    fn commands_match_the_worked_examples() {
        assert_eq!(
            command(UNKNOWN_MAC, GET_BLE_ADDRESS, &[]).to_vec(),
            hex(
                "FF 01 20 01 01 00 00 00 00 00 00 01 01 01 00 00 00 00 00 00 00 00 00 00 \
                 00 00 00 00 11 35 FF 03"
            )
        );
        assert_eq!(
            command(EXAMPLE_MAC, VERIFY_PASSWORD, &PASSWORD).to_vec(),
            hex(
                "FF 01 20 01 01 66 55 44 33 22 11 51 01 01 00 00 00 00 00 00 00 00 00 00 \
                 00 00 00 00 45 43 FF 03"
            )
        );
    }

    /// Arguments go from Arg0, and the CRC covers them.
    #[test]
    fn command_arguments_start_at_arg0() {
        let packet = command(EXAMPLE_MAC, VERIFY_PASSWORD, &[1, 2, 3, 4]);
        assert_eq!(packet[14..18], [1, 2, 3, 4]);
        assert!(packet[18..28].iter().all(|&b| b == 0));
        assert_eq!(crc16(&packet[2..30]), 0);
    }

    /// The code echoed back is an accepted login.
    #[test]
    fn an_echo_is_accepted() {
        assert_eq!(
            classify(&reply(VERIFY_PASSWORD, &PASSWORD), VERIFY_PASSWORD),
            Reply::Accepted
        );
    }

    /// 0x8001 naming the login carries the error code from Arg2-Arg3 (§8).
    #[test]
    fn a_refusal_carries_its_error_code() {
        for error in [0, 3, 4, 6, 0x0105] {
            assert_eq!(
                classify(&refusal(VERIFY_PASSWORD, error), VERIFY_PASSWORD),
                Reply::Refused(error)
            );
        }
    }

    /// What is not a response to the login decides nothing: the command we
    /// wrote still in the characteristic, a reply or refusal of another
    /// command, a broken CRC, a short read.
    #[test]
    fn other_replies_are_neither() {
        let login = command(EXAMPLE_MAC, VERIFY_PASSWORD, &PASSWORD);
        assert_eq!(classify(&login, VERIFY_PASSWORD), Reply::Other);
        assert_eq!(
            classify(&reply(GET_BLE_ADDRESS, &[]), VERIFY_PASSWORD),
            Reply::Other
        );
        assert_eq!(
            classify(&refusal(GET_BLE_ADDRESS, 5), VERIFY_PASSWORD),
            Reply::Other
        );
        let mut corrupt = reply(VERIFY_PASSWORD, &PASSWORD);
        corrupt[20] ^= 1;
        assert_eq!(classify(&corrupt, VERIFY_PASSWORD), Reply::Other);
        let accepted = reply(VERIFY_PASSWORD, &PASSWORD);
        assert_eq!(classify(&accepted[..31], VERIFY_PASSWORD), Reply::Other);
        assert_eq!(classify(&[], VERIFY_PASSWORD), Reply::Other);
    }

    /// A refusal ends the open as one, so the setup is not tried again; any
    /// other reply, or none, lets it go on, and only an unknown reply is
    /// reported.
    #[test]
    fn only_a_refusal_ends_the_login() {
        let (judged, reports) = capture_reports(|| judge(Ok(refusal(VERIFY_PASSWORD, 3))));
        assert!(
            matches!(judged, Err(SetupFailure::Refused(_))),
            "{judged:?}"
        );
        assert!(reports.is_empty(), "{reports:?}");

        let (judged, reports) = capture_reports(|| judge(Ok(reply(VERIFY_PASSWORD, &PASSWORD))));
        assert!(judged.is_ok());
        assert!(reports.is_empty(), "{reports:?}");

        let (judged, reports) = capture_reports(|| judge(Err(Error::LinkLost)));
        assert!(judged.is_ok());
        assert!(reports.is_empty(), "{reports:?}");

        let login = command(EXAMPLE_MAC, VERIFY_PASSWORD, &PASSWORD).to_vec();
        let (judged, reports) = capture_reports(|| judge(Ok(login)));
        assert!(judged.is_ok());
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(
            reports[0].starts_with("bm78xbt: unrecognised login reply: [FF, 01, 20, 01"),
            "{reports:?}"
        );
    }

    /// A password refusal names the reset gesture; another names its code.
    #[test]
    fn refusals_say_what_went_wrong() {
        for code in [3, 4] {
            let message = refused(code).to_string();
            assert!(message.contains(RESET_GESTURE), "{message}");
        }
        let message = refused(0).to_string();
        assert!(message.contains("checksum error (code 0)"), "{message}");
        let message = refused(0x0105).to_string();
        assert!(message.contains("code 261"), "{message}");
    }

    /// The address comes from any response, in the wire's order, and a
    /// packet that is not one gives none.
    #[test]
    fn the_mac_comes_from_a_response() {
        assert_eq!(
            mac_from_reply(&reply(GET_BLE_ADDRESS, &[])),
            Some(EXAMPLE_MAC)
        );
        assert_eq!(
            mac_from_reply(&refusal(GET_BLE_ADDRESS, 5)),
            Some(EXAMPLE_MAC)
        );
        let ask = command(UNKNOWN_MAC, GET_BLE_ADDRESS, &[]);
        assert_eq!(mac_from_reply(&ask), None);
        assert_eq!(mac_from_reply(&[0xFF, 0x01]), None);
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
