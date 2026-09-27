//! The password login a BM78xBT needs before it streams: the command
//! packets, and what the meter's reply to them says
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §3.2, §5, §7, §8).
//!
//! Bytes only: the Bluetooth transport writes the packets and reads the
//! replies back (`transport/ble/brymen.rs`).

use super::ID;
use super::packet::{framed, seal};
use crate::error::Error;
use crate::protocol::unrecognised::report_unknown;
use std::time::Duration;

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
pub(crate) type Mac = [u8; 6];
/// What goes in a command when the address is not known, as the app sends
/// GetBLEAddress (§3.2).
pub(crate) const UNKNOWN_MAC: Mac = [0; 6];

/// How long the app waits between GetBLEAddress and reading its reply
/// (§3.2).
pub(crate) const ADDRESS_WAIT: Duration = Duration::from_millis(300);
/// How long GetBLEAddress may take before the login goes on without an
/// address: the app's response timeout (§3.2).
pub(crate) const ADDRESS_TIMEOUT: Duration = Duration::from_secs(3);

/// What the login's reply means for the open.
#[derive(Debug)]
pub(crate) enum LoginReply {
    /// The login's code came back: it was taken (§7.1).
    Accepted,
    /// The meter refused it, with the error to fail the open with (§8).
    Refused(Error),
    /// Neither, already reported: the stream shows whether the meter took
    /// the login.
    Unknown,
}

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

/// GetBLEAddress, sent where the platform gives no address (§3.2).
pub(crate) fn ask_address() -> [u8; PACKET_LEN] {
    command(UNKNOWN_MAC, GET_BLE_ADDRESS, &[])
}

/// The login with the default password, for the meter at `mac` (§7.1,
/// §7.2).
pub(crate) fn log_in(mac: Mac) -> [u8; PACKET_LEN] {
    command(mac, VERIFY_PASSWORD, &PASSWORD)
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
pub(crate) fn mac_from_reply(reply: &[u8]) -> Option<Mac> {
    response(reply)?[5..11].try_into().ok()
}

/// What the login's `reply` says (§7.1, §8).
///
/// A reply that is neither an acceptance nor a refusal is reported, as data
/// the spec does not cover.
pub(crate) fn judge(reply: &[u8]) -> LoginReply {
    match classify(reply, VERIFY_PASSWORD) {
        Reply::Accepted => LoginReply::Accepted,
        Reply::Refused(code) => LoginReply::Refused(refused(code)),
        Reply::Other => {
            report_unknown(ID, "login reply", format_args!("{reply:02X?}"));
            LoginReply::Unknown
        }
    }
}

/// The error for a login the meter refused with `code` (§8).
fn refused(code: u16) -> Error {
    let reason = match code {
        3 | 4 => {
            return Error::Bluetooth(
                concat!(
                    "the meter refused the connection password 0000 — to reset it, ",
                    reset_gesture!()
                )
                .to_string(),
            );
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::protocol::bm78xbt::packet::crc16;
    use crate::protocol::bm78xbt::packet::tests::hex;
    use crate::protocol::capture_reports;

    /// The MAC of the spec's example 2, 11:22:33:44:55:66 (invented), in
    /// the wire's order (§10).
    pub(crate) const EXAMPLE_MAC: Mac = [0x66, 0x55, 0x44, 0x33, 0x22, 0x11];

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

    /// The meter's acceptance of the login.
    pub(crate) fn login_accepted() -> Vec<u8> {
        reply(VERIFY_PASSWORD, &PASSWORD)
    }

    /// The meter's refusal of the login with error `error`.
    pub(crate) fn login_refused(error: u16) -> Vec<u8> {
        refusal(VERIFY_PASSWORD, error)
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

    /// An acceptance and a refusal are judged as such, silently; any other
    /// reply is reported, once.
    #[test]
    fn only_an_unknown_reply_is_reported() {
        let (judged, reports) = capture_reports(|| judge(&login_refused(3)));
        assert!(matches!(judged, LoginReply::Refused(_)), "{judged:?}");
        assert!(reports.is_empty(), "{reports:?}");

        let (judged, reports) = capture_reports(|| judge(&login_accepted()));
        assert!(matches!(judged, LoginReply::Accepted), "{judged:?}");
        assert!(reports.is_empty(), "{reports:?}");

        let login = log_in(EXAMPLE_MAC);
        let (judged, reports) = capture_reports(|| judge(&login));
        assert!(matches!(judged, LoginReply::Unknown), "{judged:?}");
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
            assert!(message.contains(reset_gesture!()), "{message}");
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
}
