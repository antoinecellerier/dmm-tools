//! The CRC every BM78xBT packet ends in, and finding the reading packets in
//! the notification stream
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §4, §6.1, §6.2).
//!
//! A notification is 152 bytes: the 24-byte information packet, the 32-byte
//! reading packet, then three 32-byte blocks of zeros with no framing (spec
//! §4). A reading is found by its head, `FF 02 20 05`, and taken when its
//! CRC and its `FF 03` end hold; the information packet right before it
//! goes with it when it is whole, for its battery bit (spec §6.9). The zero
//! blocks go with the next packet found.

use crate::error::Result;
use log::debug;

/// A reading packet's first four bytes: head, `02`, length 32, type 5
/// (spec §4, §6.2).
pub(super) const READING_HEAD: [u8; 4] = [0xFF, 0x02, 0x20, 0x05];
/// An information packet's first four bytes: head, `01`, length 24, type 4
/// (spec §4, §6.1).
const INFO_HEAD: [u8; 4] = [0xFF, 0x01, 0x18, 0x04];
/// Every framed packet's last two bytes (spec §4).
const END: [u8; 2] = [0xFF, 0x03];

/// The reading packet (spec §6.2).
pub(super) const READING_LEN: usize = 32;
/// The information packet (spec §6.1).
pub(super) const INFO_LEN: usize = 24;
/// A whole notification: the information packet, the reading packet and
/// three 32-byte blocks of zeros, 152 bytes (spec §4).
// The Bluetooth transport's MTU check is its user.
#[cfg_attr(not(feature = "bluetooth"), allow(dead_code))]
pub(crate) const NOTIFICATION_LEN: usize = INFO_LEN + 4 * READING_LEN;

/// CRC-16/MODBUS, as r4 prints it in C (spec §4): start at 0xFFFF; XOR in
/// each byte, then shift right 8 times, XOR-ing 0xA001 whenever bit 0 was
/// set; no final XOR. A packet carries it over [2] up to the byte before
/// it, low byte first (spec §4, §5).
pub(crate) fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFF_u16;
    for &byte in bytes {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// The CRC a packet stores in the two bytes before its end, low byte first.
pub(crate) fn stored_crc(p: &[u8]) -> u16 {
    let at = p.len() - 4;
    u16::from_le_bytes([p[at], p[at + 1]])
}

/// The CRC of a packet's [2] up to its stored CRC (spec §4).
pub(crate) fn computed_crc(p: &[u8]) -> u16 {
    crc16(&p[2..p.len() - 4])
}

/// Whether `p` opens with `head`, ends `FF 03` and carries its CRC.
pub(crate) fn framed(p: &[u8], head: [u8; 4]) -> bool {
    p.len() >= 8 && p[..4] == head && p[p.len() - 2..] == END && computed_crc(p) == stored_crc(p)
}

/// Write `p`'s CRC, low byte first, and its `FF 03` end (spec §4, §5).
// The login's commands and the tests are its users.
#[cfg_attr(not(feature = "bluetooth"), allow(dead_code))]
pub(crate) fn seal(p: &mut [u8]) {
    let crc = computed_crc(p);
    let at = p.len() - 4;
    p[at..at + 2].copy_from_slice(&crc.to_le_bytes());
    p[at + 2..].copy_from_slice(&END);
}

/// Whether `p` is a whole reading packet whose CRC holds.
pub(super) fn is_reading(p: &[u8]) -> bool {
    p.len() == READING_LEN && framed(p, READING_HEAD)
}

/// Whether `p` is a whole information packet whose CRC holds.
pub(super) fn is_info(p: &[u8]) -> bool {
    p.len() == INFO_LEN && framed(p, INFO_HEAD)
}

/// The first reading packet in `buf` whose CRC holds: its offset.
///
/// A head and an end in place around a CRC that fails is logged at DEBUG:
/// a stream of those is the sign of a wrong CRC rule (spec §11.1). A head
/// without its end is a chance match in other bytes, and passes silently.
fn first_reading(buf: &[u8]) -> Option<usize> {
    buf.windows(READING_HEAD.len())
        .enumerate()
        .filter(|(_, w)| *w == READING_HEAD)
        .map(|(at, _)| at)
        .find(|&at| {
            let Some(p) = buf.get(at..at + READING_LEN) else {
                return false;
            };
            if is_reading(p) {
                return true;
            }
            if p[READING_LEN - 2..] == END {
                debug!(
                    "bm78xbt: reading CRC mismatch: stored {:#06x}, computed {:#06x}",
                    stored_crc(p),
                    computed_crc(p)
                );
            }
            false
        })
}

/// Find the first reading in `buf`, for
/// [`crate::protocol::framing::read_frame`].
///
/// Returns the information packet and the reading, 56 bytes, when a whole
/// information packet ends where the reading starts, else the reading
/// alone, 32 bytes; with the offset just past the reading, so the zero
/// blocks and anything else before it go with it. `Ok(None)` until a whole
/// reading has arrived. Never fails: a window whose CRC fails is not a
/// reading, and the scan goes on past it.
pub(super) fn extract(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>> {
    let Some(at) = first_reading(buf) else {
        return Ok(None);
    };
    let end = at + READING_LEN;
    let start = at
        .checked_sub(INFO_LEN)
        .filter(|&start| is_info(&buf[start..at]))
        .unwrap_or(at);
    Ok(Some((buf[start..end].to_vec(), end)))
}

/// The first CRC-valid reading packet anywhere in `buf` that says it is a
/// meter's ([17] = `01`, spec §6.2), for detection.
pub(super) fn first_meter_reading(buf: &[u8]) -> Option<&[u8]> {
    buf.windows(READING_LEN)
        .find(|w| is_reading(w) && w[17] == 0x01)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Parse the spec's space-separated hex.
    pub(crate) fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// Spec §10 example 1: −1.2345 V DC, autorange.
    pub(crate) fn example_reading() -> Vec<u8> {
        hex(
            "FF 02 20 05 01 00 00 01 FA 78 85 03 3A 35 10 40 00 01 03 00 01 C7 CF FF \
             01 00 02 05 76 C3 FF 03",
        )
    }

    /// Spec §10 example 2: the information packet, MAC 11:22:33:44:55:66
    /// (invented).
    pub(crate) fn example_info() -> Vec<u8> {
        hex("FF 01 18 04 01 02 66 55 44 33 22 11 00 00 00 00 04 00 00 01 FC 94 FF 03")
    }

    /// `p` with its CRC recomputed.
    pub(crate) fn sealed(mut p: Vec<u8>) -> Vec<u8> {
        seal(&mut p);
        p
    }

    /// A whole 152-byte notification: `info`, `reading`, 96 zeros (spec §4).
    pub(crate) fn notification(info: &[u8], reading: &[u8]) -> Vec<u8> {
        let mut n = info.to_vec();
        n.extend_from_slice(reading);
        n.extend_from_slice(&[0; 96]);
        n
    }

    /// The spec's example notification (§10, example 2's last sentence).
    pub(crate) fn example_notification() -> Vec<u8> {
        notification(&example_info(), &example_reading())
    }

    /// Sealing a spec example's body gives the example back, and the
    /// result is framed.
    #[test]
    fn seal_writes_the_crc_and_end() {
        let info = hex("FF 01 18 04 01 02 66 55 44 33 22 11 00 00 00 00 04 00 00 01 FC 94 FF 03");
        let mut body = info.clone();
        body[20..].fill(0);
        seal(&mut body);
        assert_eq!(body, info);
        assert!(framed(&info, [0xFF, 0x01, 0x18, 0x04]));
        assert!(!framed(&info, [0xFF, 0x02, 0x18, 0x04]));
        let mut corrupt = info.clone();
        corrupt[12] ^= 0x02;
        assert!(!framed(&corrupt, [0xFF, 0x01, 0x18, 0x04]));
        assert!(!framed(&info[..7], [0xFF, 0x01, 0x18, 0x04]));
    }

    /// The catalogued check value of CRC-16/MODBUS (spec §4).
    #[test]
    fn crc_check_value() {
        assert_eq!(crc16(b"123456789"), 0x4B37);
    }

    /// Every packet of the spec's worked examples (§10) carries the CRC of
    /// [2] up to the byte before it.
    #[test]
    fn worked_examples_carry_their_crc() {
        let reading = example_reading();
        assert_eq!(crc16(&reading[2..28]), 0xC376);
        assert_eq!(stored_crc(&reading), 0xC376);
        assert!(is_reading(&reading));

        let info = example_info();
        assert_eq!(crc16(&info[2..20]), 0x94FC);
        assert_eq!(stored_crc(&info), 0x94FC);
        assert!(is_info(&info));

        for command in [
            "FF 01 20 01 01 00 00 00 00 00 00 01 01 01 00 00 00 00 00 00 00 00 00 00 00 00 \
             00 00 11 35 FF 03",
            "FF 01 20 01 01 66 55 44 33 22 11 51 01 01 00 00 00 00 00 00 00 00 00 00 00 00 \
             00 00 45 43 FF 03",
        ] {
            let command = hex(command);
            assert_eq!(
                computed_crc(&command),
                stored_crc(&command),
                "{command:02X?}"
            );
        }
    }

    /// Run over the CRC bytes too, a packet's CRC comes out 0 (spec §10).
    #[test]
    fn crc_over_the_crc_is_zero() {
        let info = example_info();
        assert_eq!(crc16(&info[2..22]), 0);
    }

    #[test]
    fn a_notification_gives_the_information_packet_and_the_reading() {
        let n = example_notification();
        let (payload, consumed) = extract(&n).unwrap().unwrap();
        assert_eq!(payload, [example_info(), example_reading()].concat());
        assert_eq!(consumed, INFO_LEN + READING_LEN);
        // The zero blocks left behind hold no reading.
        assert_eq!(extract(&n[consumed..]).unwrap(), None);
    }

    /// Back to back, each notification's zeros go with the next reading.
    #[test]
    fn back_to_back_notifications_come_out_in_order() {
        let first = example_notification();
        let mut second_reading = example_reading();
        second_reading[21] = 0x39;
        let second_reading = sealed(second_reading);
        let second = notification(&example_info(), &second_reading);
        let buf = [first, second].concat();
        let (payload, consumed) = extract(&buf).unwrap().unwrap();
        assert_eq!(&payload[INFO_LEN..], example_reading());
        let (payload, next) = extract(&buf[consumed..]).unwrap().unwrap();
        assert_eq!(&payload[..INFO_LEN], example_info());
        assert_eq!(&payload[INFO_LEN..], second_reading);
        assert_eq!(next, 96 + INFO_LEN + READING_LEN);
    }

    #[test]
    fn a_reading_with_no_information_packet_before_it_comes_alone() {
        let n = example_notification();
        let (payload, consumed) = extract(&n[5..]).unwrap().unwrap();
        assert_eq!(
            payload,
            example_reading(),
            "the information packet cut short"
        );
        assert_eq!(consumed, INFO_LEN - 5 + READING_LEN);
        let (payload, _) = extract(&example_reading()).unwrap().unwrap();
        assert_eq!(payload, example_reading());
    }

    /// An information packet whose CRC fails is left behind.
    #[test]
    fn a_broken_information_packet_is_not_attached() {
        let mut n = example_notification();
        n[12] ^= 0x02;
        let (payload, _) = extract(&n).unwrap().unwrap();
        assert_eq!(payload, example_reading());
    }

    #[test]
    fn waits_on_every_cut_of_a_split_reading() {
        let n = example_notification();
        for cut in 0..INFO_LEN + READING_LEN {
            assert_eq!(extract(&n[..cut]).unwrap(), None, "cut at {cut}");
        }
    }

    #[test]
    fn a_bad_crc_is_skipped_for_the_next_reading() {
        let mut bad = example_notification();
        bad[INFO_LEN + 22] ^= 0x01;
        let buf = [bad, example_notification()].concat();
        let (payload, consumed) = extract(&buf).unwrap().unwrap();
        assert_eq!(payload, [example_info(), example_reading()].concat());
        assert_eq!(consumed, buf.len() - 96);
    }

    /// A reading head inside another packet's bytes that fails the CRC is
    /// passed over, and the real one after it read.
    #[test]
    fn a_false_head_is_skipped() {
        let mut buf = READING_HEAD.to_vec();
        buf.extend_from_slice(&[0x11; 40]);
        buf.extend(example_notification());
        let (payload, consumed) = extract(&buf).unwrap().unwrap();
        assert_eq!(&payload[INFO_LEN..], example_reading());
        assert_eq!(consumed, 44 + INFO_LEN + READING_LEN);
    }

    #[test]
    fn detection_wants_a_meters_reading() {
        let n = example_notification();
        assert_eq!(first_meter_reading(&n), Some(&example_reading()[..]));
        let mut sensor = example_reading();
        sensor[17] = 0x00;
        assert_eq!(first_meter_reading(&sealed(sensor)), None, "a sensor's");
        assert_eq!(first_meter_reading(&n[..INFO_LEN + READING_LEN - 1]), None);
        assert_eq!(first_meter_reading(&example_info()), None);
    }

    /// A small xorshift generator, so arbitrary input needs no new
    /// dependency.
    fn pseudo_random(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    /// Arbitrary bytes, with heads and packets sprinkled in to reach every
    /// branch, never panic the extractor or the detection scan.
    #[test]
    fn arbitrary_bytes_never_panic() {
        let mut seed = 0x2545_F491_4F6C_DD1D;
        for _ in 0..20_000 {
            let len = (pseudo_random(&mut seed) % 200) as usize;
            let mut buf: Vec<u8> = (0..len).map(|_| pseudo_random(&mut seed) as u8).collect();
            if len > 4 && pseudo_random(&mut seed).is_multiple_of(2) {
                let at = (pseudo_random(&mut seed) as usize) % (len - 4);
                buf[at..at + 4].copy_from_slice(&READING_HEAD);
            }
            if pseudo_random(&mut seed).is_multiple_of(4) {
                let at = (pseudo_random(&mut seed) as usize) % (len + 1);
                buf.splice(at..at, example_notification());
            }
            let mut rest = buf.as_slice();
            while let Some((payload, consumed)) = extract(rest).unwrap() {
                assert!(matches!(payload.len(), READING_LEN | 56));
                assert!(is_reading(&payload[payload.len() - READING_LEN..]));
                assert!(consumed <= rest.len());
                rest = &rest[consumed..];
            }
            let _ = first_meter_reading(&buf);
        }
    }
}
