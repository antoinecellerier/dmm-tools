//! Finding 121GW packets in the Bluetooth byte stream
//! (`docs/research/121gw/reverse-engineered-protocol.md` §4, §15.4).
//!
//! A packet is 19 bytes: `F2`, 17 bytes of fields, and the XOR of bytes
//! 0-17 (spec §4). `F2` is not escaped, so a field byte can equal it, and
//! the two clients run on a meter take 18-byte values with no `F2` at all
//! (spec §15.4, "Chunking"). So a packet is found by its body, bytes 1-18,
//! whose XOR is `F2` whether or not the `F2` came with it; a `F2` right
//! before the body is taken with it. Every packet is handed on as 19 bytes,
//! `F2` first, so a reading's raw payload has one shape either way.

use super::tables;
use crate::error::Result;

/// Byte 0, the start byte (spec §4).
pub(super) const START: u8 = 0xF2;

/// A whole packet, `F2` to checksum (spec §4).
pub(super) const LEN: usize = 19;

/// Bytes 1-18: the packet without its start byte.
const BODY_LEN: usize = LEN - 1;

/// Bits the V2 document fixes at `0` (spec §5, §8, §9): byte 5 bit 5,
/// byte 10 bit 3, byte 13 bits 7-5, byte 14 bits 7-6 and byte 17 bit 7.
/// Byte 14 bit 5 is left out: real meters set it (spec §15.3 D4).
const RESERVED: [(usize, u8); 5] = [(5, 0x20), (10, 0x08), (13, 0xE0), (14, 0xC0), (17, 0x80)];

/// XOR of all of `bytes`.
fn xor(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |acc, b| acc ^ b)
}

/// The XOR of bytes 0-17 of a 19-byte packet (spec §4).
pub(super) fn checksum(p: &[u8]) -> u8 {
    xor(&p[..BODY_LEN])
}

/// Whether byte 18 is the XOR of bytes 0-17 (spec §4).
pub(super) fn checksum_holds(p: &[u8]) -> bool {
    checksum(p) == p[BODY_LEN]
}

/// Whether any bit the document fixes at `0` is set.
pub(super) fn has_reserved_bits(p: &[u8]) -> bool {
    RESERVED.iter().any(|&(at, mask)| p[at] & mask != 0)
}

/// Byte 13 bits 7-5 clear, as V2 fixes them (spec §8). Every ASCII letter
/// and digit sets bit 5 or 6, and byte 13 falls among the hex pairs of
/// every ASCII format older firmware sends (spec §12, §15.4), so this is
/// what keeps those from reading as a binary packet.
fn binary(p: &[u8]) -> bool {
    p[13] & 0xE0 == 0
}

/// What starts at an offset of the buffer.
enum Candidate {
    /// A packet may start here, but not all of it has arrived.
    Wait,
    /// No packet starts here.
    No,
    /// A packet, `F2` first, and how many buffer bytes it took.
    Packet([u8; LEN], usize),
}

/// The 19-byte packet whose bytes 1-18 are `body`, if it is one.
///
/// Its XOR must hold and byte 13 be binary. `strict` also requires a mode
/// and range in the tables and no reserved bit set: past where the previous
/// packet ended, an 18-byte window of a partial packet passes the XOR one
/// time in 256, and these are what else tells a packet from one.
fn packet(body: &[u8], strict: bool) -> Option<[u8; LEN]> {
    let mut p = [0u8; LEN];
    p[0] = START;
    p[1..].copy_from_slice(body);
    if !checksum_holds(&p) || !binary(&p) {
        return None;
    }
    if strict && (!tables::decodable(&p) || has_reserved_bits(&p)) {
        return None;
    }
    Some(p)
}

/// The packet at `buf[at]`: `F2` and a body, or a body alone.
fn candidate(buf: &[u8], at: usize, strict: bool) -> Candidate {
    let rest = &buf[at..];
    if rest.first() == Some(&START) {
        if rest.len() < LEN {
            return Candidate::Wait;
        }
        if let Some(p) = packet(&rest[1..LEN], strict) {
            return Candidate::Packet(p, LEN);
        }
    }
    if rest.len() < BODY_LEN {
        return Candidate::Wait;
    }
    match packet(&rest[..BODY_LEN], strict) {
        Some(p) => Candidate::Packet(p, BODY_LEN),
        None => Candidate::No,
    }
}

/// Find the first packet in `buf`, for [`crate::protocol::framing::read_frame`].
///
/// Returns it as 19 bytes, `F2` first, with the offset just past it, so
/// whatever preceded it — a partial packet from joining mid-stream — goes
/// with it. `Ok(None)` until a whole packet has arrived. Never fails: a
/// window that fails the XOR is not a packet.
///
/// Where the previous packet ended (offset 0) the XOR and byte 13 are
/// enough, so a mode or range code the tables lack reaches the decoder,
/// which reports it; past it the window must also decode (see [`packet`]),
/// or be followed right away by another window whose XOR and byte 13 hold:
/// two packets back to back. That brings a meter whose packets the tables
/// do not cover back in step after joining mid-packet, and its next packet
/// is at offset 0 again. Waiting for that second window lasts at most one
/// packet's worth of bytes.
pub(super) fn extract_packet(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    for at in 0..buf.len() {
        match candidate(buf, at, at > 0) {
            Candidate::Wait => return Ok(None),
            Candidate::Packet(p, len) => return Ok(Some((p.to_vec(), at + len))),
            Candidate::No if at == 0 => {}
            Candidate::No => {
                if let Candidate::Packet(p, len) = candidate(buf, at, false) {
                    match candidate(buf, at + len, false) {
                        Candidate::Wait => return Ok(None),
                        Candidate::Packet(..) => return Ok(Some((p.to_vec(), at + len))),
                        Candidate::No => {}
                    }
                }
            }
        }
    }
    Ok(None)
}

/// The first packet anywhere in `buf` that decodes with no reserved bit
/// set, for detection, which takes nothing on the XOR alone.
pub(super) fn first_decodable(buf: &[u8]) -> Option<[u8; LEN]> {
    (0..buf.len()).find_map(|at| match candidate(buf, at, true) {
        Candidate::Packet(p, _) => Some(p),
        Candidate::Wait | Candidate::No => None,
    })
}

/// Whether `buf` holds the start of an ASCII packet, as firmware older
/// than the binary format sends (spec §12): `F2` followed by at least 20
/// ASCII letters and digits.
pub(super) fn holds_ascii_packet(buf: &[u8]) -> bool {
    const ASCII_RUN: usize = 20;
    buf.windows(1 + ASCII_RUN)
        .any(|w| w[0] == START && w[1..].iter().all(u8::is_ascii_alphanumeric))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Spec §13's worked examples 1-4; bytes 1-4 are the spec's invented
    /// identity.
    pub(crate) const EXAMPLES: [[u8; LEN]; 4] = [
        [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x01, 0x00, 0x30, 0x39, 0x64, 0x01, 0x00, 0xF3, 0x00,
            0x00, 0x0C, 0x40, 0x00, 0x35,
        ],
        [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x46, 0x00, 0x56, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x04, 0x40, 0x00, 0xD5,
        ],
        [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x09, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x04, 0x40, 0x00, 0x2C,
        ],
        [
            0xF2, 0x12, 0x61, 0x23, 0x45, 0x03, 0x40, 0x30, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0C, 0x40, 0x00, 0xE1,
        ],
    ];

    /// Community-captured, spec §15.5, not ours: the packet quoted in
    /// sigrok (Duty 0.0 %, internal 27.9 °C) and 121gwcli's (Ω OFL at
    /// 50 MΩ, internal 25.8 °C), `F2` prepended to the latter. Both set
    /// byte 14 bit 5 (spec §15.3 D4).
    pub(crate) const COMMUNITY: [[u8; LEN]; 2] = [
        [
            0xF2, 0x17, 0x84, 0x21, 0x21, 0x08, 0x00, 0x00, 0x00, 0x64, 0x01, 0x01, 0x17, 0x12,
            0x37, 0x02, 0x40, 0x00, 0x7D,
        ],
        [
            0xF2, 0x17, 0x85, 0x43, 0x21, 0x09, 0x86, 0x00, 0x00, 0x64, 0x01, 0x01, 0x02, 0x01,
            0x21, 0x06, 0x40, 0x00, 0x8D,
        ],
    ];

    /// Every example and community packet.
    pub(crate) fn all_packets() -> Vec<[u8; LEN]> {
        EXAMPLES.iter().chain(&COMMUNITY).copied().collect()
    }

    /// `p` with its checksum recomputed.
    pub(crate) fn sealed(mut p: [u8; LEN]) -> [u8; LEN] {
        p[BODY_LEN] = checksum(&p);
        p
    }

    /// An ASCII packet shaped as firmware 1.02's (spec §15.4): `F2`, nine
    /// identity digits, then hex pairs.
    pub(crate) fn ascii_stream() -> Vec<u8> {
        let mut s = vec![START];
        s.extend_from_slice(b"178542121");
        s.extend_from_slice(b"010030390064010117123702400000");
        s.extend_from_slice(b"00000000000000\r\n");
        s
    }

    #[test]
    fn every_packet_passes_its_checksum() {
        for p in all_packets() {
            assert!(checksum_holds(&p), "{p:02X?}");
            assert_eq!(xor(&p), 0);
            assert_eq!(xor(&p[1..]), START, "bytes 1-18 XOR to F2");
        }
    }

    #[test]
    fn each_packet_extracts_with_or_without_its_start_byte() {
        for p in all_packets() {
            assert_eq!(extract_packet(&p).unwrap(), Some((p.to_vec(), LEN)));
            assert_eq!(
                extract_packet(&p[1..]).unwrap(),
                Some((p.to_vec(), BODY_LEN)),
                "18-byte value"
            );
        }
    }

    #[test]
    fn junk_before_a_packet_goes_with_it() {
        for p in all_packets() {
            for form in [&p[..], &p[1..]] {
                let mut buf = vec![0x00, 0x13, 0xFF, 0x42, 0x07];
                buf.extend_from_slice(form);
                let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
                assert_eq!(packet, p);
                assert_eq!(consumed, buf.len());
            }
        }
    }

    #[test]
    fn waits_on_every_cut_of_a_split_packet() {
        for p in all_packets() {
            for cut in 0..LEN {
                assert_eq!(extract_packet(&p[..cut]).unwrap(), None, "cut at {cut}");
            }
            for cut in 0..BODY_LEN {
                assert_eq!(
                    extract_packet(&p[1..1 + cut]).unwrap(),
                    None,
                    "cut at {cut}"
                );
            }
        }
    }

    #[test]
    fn back_to_back_packets_come_out_in_order() {
        let packets = all_packets();
        for with_start in [true, false] {
            let mut buf = Vec::new();
            for p in &packets {
                buf.extend_from_slice(if with_start { &p[..] } else { &p[1..] });
            }
            let mut at = 0;
            for p in &packets {
                let (packet, consumed) = extract_packet(&buf[at..]).unwrap().unwrap();
                assert_eq!(&packet, p);
                at += consumed;
            }
            assert_eq!(at, buf.len());
        }
    }

    /// A false `F2` in front of a packet: its window fails the XOR, and the
    /// packet after it is read.
    #[test]
    fn a_false_start_byte_is_skipped() {
        for p in all_packets() {
            let mut buf = vec![START, 0x01, 0x02];
            buf.extend_from_slice(&p);
            let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
            assert_eq!(packet, p);
            assert_eq!(consumed, buf.len());
        }
    }

    /// A packet whose mode is not in the tables and that sets a reserved
    /// bit, as a firmware the spec does not cover might send.
    pub(crate) fn off_table() -> [u8; LEN] {
        let mut odd = EXAMPLES[0];
        odd[5] = 25;
        odd[17] = 0x80;
        sealed(odd)
    }

    /// Past offset 0, a window whose XOR holds but whose mode is not in the
    /// tables, with junk after it, is taken for part of a partial packet
    /// and skipped.
    #[test]
    fn an_undecodable_window_past_the_start_is_skipped() {
        let mut buf = vec![0xAA];
        buf.extend_from_slice(&off_table());
        buf.extend_from_slice(&[0x55; 7]);
        buf.extend_from_slice(&EXAMPLES[1]);
        let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, EXAMPLES[1]);
        assert_eq!(consumed, buf.len());
    }

    /// Past offset 0, such a window with another packet right after it is
    /// the meter's: two back to back, in either shape.
    #[test]
    fn an_undecodable_packet_followed_by_another_is_the_meters() {
        let odd = off_table();
        for (first, next) in [(&odd[..], &odd[..]), (&odd[1..], &odd[1..])] {
            let mut buf = vec![0xAA, 0x13];
            buf.extend_from_slice(first);
            buf.extend_from_slice(next);
            let (packet, consumed) = extract_packet(&buf).unwrap().unwrap();
            assert_eq!(packet, odd);
            assert_eq!(consumed, 2 + first.len());
            let (packet, consumed) = extract_packet(&buf[2 + first.len()..]).unwrap().unwrap();
            assert_eq!(packet, odd);
            assert_eq!(consumed, next.len());
        }
        // A decodable packet confirms it too.
        let mut buf = vec![0xAA];
        buf.extend_from_slice(&odd);
        buf.extend_from_slice(&EXAMPLES[1]);
        assert_eq!(extract_packet(&buf).unwrap(), Some((odd.to_vec(), 1 + LEN)));
    }

    /// Until the window after it has arrived, such a window is waited on,
    /// not skipped.
    #[test]
    fn an_undecodable_packet_waits_for_the_next() {
        let odd = off_table();
        let mut buf = vec![0xAA];
        buf.extend_from_slice(&odd);
        for cut in 0..LEN {
            let mut partial = buf.clone();
            partial.extend_from_slice(&EXAMPLES[1][..cut]);
            assert_eq!(extract_packet(&partial).unwrap(), None, "cut at {cut}");
        }
    }

    /// Where the previous packet ended, a packet the tables do not cover is
    /// the meter's, for the decoder to report.
    #[test]
    fn an_undecodable_packet_at_the_start_is_the_meters() {
        let odd = off_table();
        assert_eq!(extract_packet(&odd).unwrap(), Some((odd.to_vec(), LEN)));
    }

    #[test]
    fn a_bad_checksum_is_skipped_for_the_next_packet() {
        let mut bad = EXAMPLES[0];
        bad[18] ^= 0x01;
        let mut buf = bad.to_vec();
        buf.extend_from_slice(&EXAMPLES[3]);
        let (packet, _) = extract_packet(&buf).unwrap().unwrap();
        assert_eq!(packet, EXAMPLES[3]);
    }

    /// Firmware 1.02's ASCII frame (spec §15.4), its XOR forced to hold:
    /// its digits give a mode and range in the tables, and byte 13, an
    /// ASCII digit, is what refuses it.
    #[test]
    fn an_ascii_frame_never_reads_as_a_packet() {
        let stream = ascii_stream();
        let mut forced = [0u8; LEN];
        forced.copy_from_slice(&stream[..LEN]);
        let forced = sealed(forced);
        assert!(tables::decodable(&forced), "the digits decode");
        assert_eq!(extract_packet(&forced).unwrap(), None);
        assert_eq!(first_decodable(&forced), None);
        assert_eq!(extract_packet(&stream).unwrap(), None);
    }

    #[test]
    fn an_ascii_packet_is_recognised_as_one() {
        assert!(holds_ascii_packet(&ascii_stream()));
        let mut junk_first = vec![0x00, 0x13];
        junk_first.extend_from_slice(&ascii_stream());
        assert!(holds_ascii_packet(&junk_first));
        for p in all_packets() {
            assert!(!holds_ascii_packet(&p));
        }
        assert!(!holds_ascii_packet(&ascii_stream()[..20]), "too short");
        assert!(!holds_ascii_packet(&ascii_stream()[1..]), "no F2");
    }

    /// Detection takes nothing on the XOR alone: a packet with a reserved
    /// bit set, or a mode the tables lack, is not evidence.
    #[test]
    fn detection_wants_a_decodable_packet() {
        for p in all_packets() {
            assert_eq!(first_decodable(&p), Some(p));
            assert_eq!(first_decodable(&p[1..]), Some(p));
            assert_eq!(first_decodable(&p[..LEN - 1]), None);
        }
        for (at, mask) in RESERVED {
            let mut odd = EXAMPLES[0];
            odd[at] |= mask;
            assert_eq!(first_decodable(&sealed(odd)), None, "byte {at} {mask:#04x}");
        }
    }

    /// A small xorshift generator, so arbitrary input needs no new
    /// dependency.
    fn pseudo_random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    /// Arbitrary bytes, with `F2` and packets sprinkled in to reach every
    /// branch, never panic the extractor or the detection scan.
    #[test]
    fn arbitrary_bytes_never_panic() {
        let mut seed = 0x2545_F491_4F6C_DD1D;
        for _ in 0..20_000 {
            let len = (pseudo_random(&mut seed) % 64) as usize;
            let mut buf: Vec<u8> = (0..len).map(|_| pseudo_random(&mut seed) as u8).collect();
            if len > 0 && pseudo_random(&mut seed).is_multiple_of(2) {
                let at = (pseudo_random(&mut seed) as usize) % len;
                buf[at] = START;
            }
            if pseudo_random(&mut seed).is_multiple_of(4) {
                let at = (pseudo_random(&mut seed) as usize) % (len + 1);
                buf.splice(at..at, EXAMPLES[0]);
            }
            let mut rest = buf.as_slice();
            while let Some((packet, consumed)) = extract_packet(rest).unwrap() {
                assert_eq!(packet.len(), LEN);
                assert!(checksum_holds(&packet));
                assert!(consumed <= rest.len());
                rest = &rest[consumed..];
            }
            let _ = first_decodable(&buf);
            let _ = holds_ascii_packet(&buf);
        }
    }
}
