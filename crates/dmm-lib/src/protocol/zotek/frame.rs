//! Finding ZOTEK packets in the notification stream, and descrambling them
//! (`docs/research/zotek/reverse-engineered-protocol.md` §4, §5).
//!
//! Each notification carries one packet, scrambled from key byte 0, but the
//! Bluetooth transport hands the protocol a plain byte stream: notification
//! boundaries are gone, and one read may hold several packets or part of
//! one. So packets are found by their on-air header, `1B 84` followed by the
//! scrambled type byte, and cut at the length their type gives.

use super::layout;
use crate::error::Result;
use log::debug;

/// The fixed key every notification is XORed with, from key byte 0
/// (spec §4).
pub(super) const KEY: [u8; 20] = [
    0x41, 0x21, 0x73, 0x55, 0xA2, 0xC1, 0x32, 0x71, 0x66, 0xAA, 0x3B, 0xD0, 0xE2, 0xA8, 0x33, 0x14,
    0x20, 0x1A, 0xAA, 0xBB,
];

/// Bytes 0-1 of every packet once descrambled (spec §5).
pub(super) const HEADER: [u8; 2] = [0x5A, 0xA5];

/// [`HEADER`] as it arrives on air, XORed with key bytes 0-1 (spec §4).
pub(super) const RAW_HEADER: [u8; 2] = [HEADER[0] ^ KEY[0], HEADER[1] ^ KEY[1]];

/// Byte 2, the type byte that picks the layout (spec §1, §5).
pub(super) const TYPE_AT: usize = 2;

/// A packet's length by its type byte: the apps read bytes 3-9, 3-9, 3-10
/// and 3-18 (spec §5), and community captures show exactly those lengths
/// with nothing after (spec §11.4). `None` for a type no app defines.
pub(super) fn packet_len(type_byte: u8) -> Option<usize> {
    match type_byte {
        1 | 2 => Some(10),
        3 => Some(11),
        4 => Some(19),
        _ => None,
    }
}

/// XOR `bytes` with the key from key byte 0. It is its own inverse, so it
/// descrambles a packet and scrambles one (spec §4).
pub(super) fn xor_key(bytes: &mut [u8]) {
    for (byte, key) in bytes.iter_mut().zip(KEY.iter().cycle()) {
        *byte ^= key;
    }
}

/// What starts at `buf[start]`: a whole packet's length, `Some(None)` for a
/// packet header whose bytes have not all arrived, `None` for no header.
fn candidate(buf: &[u8], start: usize) -> Option<Option<usize>> {
    let rest = &buf[start..];
    if rest.len() < TYPE_AT + 1 || !rest.starts_with(&RAW_HEADER) {
        return None;
    }
    let len = packet_len(rest[TYPE_AT] ^ KEY[TYPE_AT])?;
    Some((rest.len() >= len).then_some(len))
}

/// The whole packet starting at `buf[start]`, descrambled; `None` where no
/// packet header is, or its packet has not fully arrived.
pub(super) fn packet_at(buf: &[u8], start: usize) -> Option<Vec<u8>> {
    let len = candidate(buf, start)??;
    let mut packet = buf[start..start + len].to_vec();
    xor_key(&mut packet);
    Some(packet)
}

/// Find the first packet in `buf`, for [`crate::protocol::framing::read_frame`].
///
/// Returns it descrambled (starting `5A A5 <type>`) with the offset just past
/// it, so whatever preceded it — a partial packet from joining mid-stream —
/// goes with it. `Ok(None)` until the first packet header's whole packet has
/// arrived; a header whose type byte no app defines is not one. Never fails:
/// there is no checksum to fail (spec §5).
///
/// A partial packet can hold bytes that read as a header, and the packet cut
/// there runs into the real one. So past the start of `buf`, a packet whose
/// glyphs are not all listed ([`layout::plausible`], as detection checks) is
/// taken for one of those and the scan goes on from the next byte. At the
/// start, where the previous packet ended, it is the meter's, and decoding
/// reports what it shows.
pub(super) fn extract_packet(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    for start in 0..buf.len() {
        if candidate(buf, start) == Some(None) {
            return Ok(None);
        }
        let Some(packet) = packet_at(buf, start) else {
            continue;
        };
        if start > 0 && layout::plausible(&packet).is_none() {
            debug!("zotek: no packet at offset {start}, its glyphs are unlisted; resyncing");
            continue;
        }
        let consumed = start + packet.len();
        return Ok(Some((packet, consumed)));
    }
    Ok(None)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Spec §9's four packets, on air and descrambled.
    pub(crate) const EXAMPLES: [(&[u8], &[u8]); 4] = [
        (
            &[
                0x1B, 0x84, 0x70, 0x41, 0x08, 0x5C, 0x7D, 0x7F, 0x66, 0xFA, 0x3A,
            ],
            &[
                0x5A, 0xA5, 0x03, 0x14, 0xAA, 0x9D, 0x4F, 0x0E, 0x00, 0x50, 0x01,
            ],
        ),
        (
            &[0x1B, 0x84, 0x72, 0xF3, 0x2F, 0x2E, 0xE9, 0xD6, 0x66, 0xAA],
            &[0x5A, 0xA5, 0x01, 0xA6, 0x8D, 0xEF, 0xDB, 0xA7, 0x00, 0x00],
        ),
        (
            &[0x1B, 0x84, 0x71, 0x15, 0x3C, 0x2B, 0xD9, 0xFA, 0x66, 0xA9],
            &[0x5A, 0xA5, 0x02, 0x40, 0x9E, 0xEA, 0xEB, 0x8B, 0x00, 0x03],
        ),
        (
            &[
                0x1B, 0x84, 0x77, 0x51, 0xF2, 0x2A, 0xC9, 0x9A, 0xA1, 0x6D, 0x75, 0x5F, 0x5F, 0xA6,
                0x33, 0x14, 0x20, 0x1A, 0xAA,
            ],
            &[
                0x5A, 0xA5, 0x04, 0x04, 0x50, 0xEB, 0xFB, 0xEB, 0xC7, 0xC7, 0x4E, 0x8F, 0xBD, 0x0E,
                0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        ),
    ];

    #[test]
    fn the_raw_header_is_5a_a5_scrambled() {
        assert_eq!(RAW_HEADER, [0x1B, 0x84]);
    }

    /// The on-air type bytes spec §4 lists: 72, 71, 70, 77 for types 1-4.
    #[test]
    fn type_bytes_arrive_scrambled() {
        for (t, on_air) in [(1u8, 0x72u8), (2, 0x71), (3, 0x70), (4, 0x77)] {
            assert_eq!(t ^ KEY[TYPE_AT], on_air);
        }
    }

    #[test]
    fn each_worked_example_descrambles() {
        for (raw, plain) in EXAMPLES {
            let (packet, consumed) = extract_packet(raw).unwrap().unwrap();
            assert_eq!(packet, plain);
            assert_eq!(consumed, raw.len());
            let mut scrambled = plain.to_vec();
            xor_key(&mut scrambled);
            assert_eq!(scrambled, raw, "xor_key is its own inverse");
        }
    }

    #[test]
    fn garbage_before_a_packet_goes_with_it() {
        let (raw, plain) = EXAMPLES[0];
        let mut buf = vec![0x00, 0x1B, 0xFF, 0x1B, 0x84];
        buf.extend_from_slice(raw);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, plain);
        assert_eq!(consumed, buf.len());
    }

    /// Joining mid-packet: the tail of one packet is skipped, the next one
    /// read whole.
    #[test]
    fn resyncs_after_a_partial_packet() {
        let (raw, plain) = EXAMPLES[3];
        let mut buf = raw[5..].to_vec();
        buf.extend_from_slice(raw);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, plain);
        assert_eq!(consumed, buf.len());
    }

    /// A packet split across two reads is only returned once all of it is
    /// there.
    #[test]
    fn waits_for_the_rest_of_a_split_packet() {
        let (raw, plain) = EXAMPLES[3];
        for cut in 0..raw.len() {
            assert_eq!(extract_packet(&raw[..cut]).unwrap(), None, "cut at {cut}");
        }
        assert_eq!(extract_packet(raw).unwrap().unwrap().0, plain);
    }

    #[test]
    fn back_to_back_packets_come_out_in_order() {
        let mut buf = Vec::new();
        for (raw, _) in EXAMPLES {
            buf.extend_from_slice(raw);
        }
        let mut at = 0;
        for (_, plain) in EXAMPLES {
            let (packet, consumed) = extract_packet(&buf[at..]).unwrap().unwrap();
            assert_eq!(packet, plain);
            at += consumed;
        }
        assert_eq!(at, buf.len());
        assert_eq!(extract_packet(&buf[at..]).unwrap(), None);
    }

    /// `1B 84` followed by a type no app defines is not a header: the scan
    /// moves past it to the real one.
    #[test]
    fn an_unknown_type_after_the_header_is_skipped() {
        let (raw, plain) = EXAMPLES[0];
        for unknown in [0u8, 5, 0x7F, 0xFF] {
            let mut buf = vec![0x1B, 0x84, unknown ^ KEY[TYPE_AT], 0x00, 0x00];
            buf.extend_from_slice(raw);
            let (packet, _) = extract_packet(&buf).unwrap().unwrap();
            assert_eq!(packet, plain, "type {unknown}");
        }
    }

    /// Where the previous packet ended, a packet with an unlisted glyph is
    /// the meter's, for the decoder to report; past a partial one it is a
    /// false header, skipped.
    #[test]
    fn unlisted_glyphs_are_the_meters_only_at_the_start() {
        let (raw, plain) = EXAMPLES[0];
        let mut odd = raw.to_vec();
        odd[4] ^= 0x01;
        let (packet, _) = extract_packet(&odd).unwrap().unwrap();
        assert_eq!(packet[..4], plain[..4]);

        let mut buf = vec![0xAA];
        buf.extend_from_slice(&odd);
        buf.extend_from_slice(raw);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, plain);
        assert_eq!(consumed, buf.len());
    }

    #[test]
    fn a_short_tail_is_not_a_packet() {
        for tail in [&[][..], &[0x1B], &[0x1B, 0x84], &[0x1B, 0x84, 0x70, 0x41]] {
            assert_eq!(extract_packet(tail).unwrap(), None, "{tail:02X?}");
        }
    }

    #[test]
    fn packet_at_reads_only_where_a_packet_starts() {
        let (raw, plain) = EXAMPLES[1];
        let mut buf = vec![0xAA];
        buf.extend_from_slice(raw);
        assert_eq!(packet_at(&buf, 0), None);
        assert_eq!(packet_at(&buf, 1).as_deref(), Some(plain));
        assert_eq!(packet_at(&buf[..buf.len() - 1], 1), None);
    }

    /// A small xorshift generator, so arbitrary input needs no new
    /// dependency.
    pub(crate) fn pseudo_random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    /// Arbitrary bytes, headers sprinkled in to reach the length checks,
    /// never panic the extractor.
    #[test]
    fn arbitrary_bytes_never_panic() {
        let mut seed = 0x2545_F491_4F6C_DD1D;
        for _ in 0..20_000 {
            let len = (pseudo_random(&mut seed) % 48) as usize;
            let mut buf: Vec<u8> = (0..len).map(|_| pseudo_random(&mut seed) as u8).collect();
            if len > 3 && pseudo_random(&mut seed).is_multiple_of(2) {
                let at = (pseudo_random(&mut seed) as usize) % (len - 2);
                buf[at..at + 2].copy_from_slice(&RAW_HEADER);
            }
            let mut rest = buf.as_slice();
            while let Some((packet, consumed)) = extract_packet(rest).unwrap() {
                assert!(packet.starts_with(&HEADER));
                assert!(consumed <= rest.len());
                rest = &rest[consumed..];
            }
            for start in 0..buf.len() {
                let _ = packet_at(&buf, start);
            }
        }
    }
}
