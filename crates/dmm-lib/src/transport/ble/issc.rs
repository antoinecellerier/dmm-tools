//! The ISSC transparent-UART profile, and what UNI-T's UT-D07 adapters add
//! to it.

use std::collections::VecDeque;

/// ISSC transparent-UART service the UT-D07B carries
/// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2), and the
/// meters with Bluetooth built in too.
pub(super) const UART_SERVICE: &str = "49535343-fe7d-4ae5-8fa9-9fafd205e455";
/// UART TX, meter → host: notifications carry the meter's frames (§2).
pub(super) const UART_TX_CHARACTERISTIC: &str = "49535343-1e4d-4bd9-ba61-23c647249616";
/// UART RX, host → meter: commands are written here (§2).
pub(super) const UART_RX_CHARACTERISTIC: &str = "49535343-8841-43f4-a8d4-ecbe34729bb3";
/// Local name prefix UNI-T's adapters advertise, uppercase: the UT-D07B
/// advertises `UT-D07B` (research doc §2) and the UT-D07A a name starting
/// `UT-D07A` (§7). A meter with the radio built in is named by the caller
/// ([`crate::transport::BluetoothPeers::meters`]).
pub(super) const ADAPTER_NAME_PREFIX: &str = "UT-D07";

/// The frame the adapter itself puts on the stream: once when a link comes
/// up and about once a second while the meter is silent (research doc §3).
/// It is the adapter's, not the meter's, so it never reaches a parser. A
/// meter with the radio built in never sends it.
const ADAPTER_HEARTBEAT: [u8; 9] = [0xAB, 0xCD, 0x06, 0xAA, 0xAA, 0x6E, 0x67, 0x03, 0xA7];

/// Remove every complete [`ADAPTER_HEARTBEAT`] from `pending`, returning how
/// many there were.
///
/// Only whole frames: one split across two notifications is left for the
/// parser to report, which the adapter's notifications — ATT-sized, on an
/// MTU well past nine bytes — make rare enough not to buffer for.
pub(super) fn strip_heartbeats(pending: &mut VecDeque<u8>) -> u32 {
    let mut stripped = 0;
    let mut at = 0;
    while at + ADAPTER_HEARTBEAT.len() <= pending.len() {
        if pending
            .range(at..at + ADAPTER_HEARTBEAT.len())
            .eq(ADAPTER_HEARTBEAT.iter())
        {
            pending.drain(at..at + ADAPTER_HEARTBEAT.len());
            stripped += 1;
        } else {
            at += 1;
        }
    }
    stripped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The UUIDs are the ones read off our own adapter; a typo here would
    /// make every open fail to find the UART.
    #[test]
    fn uart_uuids_match_the_research_doc() {
        assert_eq!(UART_SERVICE, "49535343-fe7d-4ae5-8fa9-9fafd205e455");
        assert_eq!(
            UART_TX_CHARACTERISTIC,
            "49535343-1e4d-4bd9-ba61-23c647249616"
        );
        assert_eq!(
            UART_RX_CHARACTERISTIC,
            "49535343-8841-43f4-a8d4-ecbe34729bb3"
        );
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [UART_SERVICE, UART_TX_CHARACTERISTIC, UART_RX_CHARACTERISTIC] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }

    /// The adapter's own frame is dropped wherever it lands in a
    /// notification, and a real frame around it is left whole.
    #[test]
    fn adapter_heartbeats_are_stripped_from_the_stream() {
        let reading = [0xAB, 0xCD, 0x03, 0xFF, 0x00, 0x02, 0x7B];
        let mut pending: VecDeque<u8> = VecDeque::new();
        pending.extend(ADAPTER_HEARTBEAT);
        pending.extend(reading);
        pending.extend(ADAPTER_HEARTBEAT);
        pending.extend(ADAPTER_HEARTBEAT);
        assert_eq!(strip_heartbeats(&mut pending), 3);
        assert_eq!(pending.iter().copied().collect::<Vec<u8>>(), reading);

        // A split heartbeat is not a heartbeat yet.
        let mut partial: VecDeque<u8> = ADAPTER_HEARTBEAT[..5].iter().copied().collect();
        assert_eq!(strip_heartbeats(&mut partial), 0);
        assert_eq!(partial.len(), 5);

        // The checksum is part of the match: a frame that only starts like
        // one is the meter's.
        let mut lookalike: VecDeque<u8> = ADAPTER_HEARTBEAT.iter().copied().collect();
        *lookalike.back_mut().unwrap() = 0xA8;
        assert_eq!(strip_heartbeats(&mut lookalike), 0);
    }
}
