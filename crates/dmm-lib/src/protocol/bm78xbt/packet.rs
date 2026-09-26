//! The CRC and the end every framed BM78xBT packet carries
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md` §4).

// The Bluetooth transport's login is the only user so far.
#![cfg_attr(not(feature = "bluetooth"), allow(dead_code))]

/// Every framed packet's last two bytes (spec §4).
const END: [u8; 2] = [0xFF, 0x03];

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
pub(crate) fn seal(p: &mut [u8]) {
    let crc = computed_crc(p);
    let at = p.len() - 4;
    p[at..at + 2].copy_from_slice(&crc.to_le_bytes());
    p[at + 2..].copy_from_slice(&END);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse the spec's space-separated hex.
    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// The CRC stored at `packet[at..at + 2]`, low byte first.
    fn stored(packet: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([packet[at], packet[at + 1]])
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
        let reading = hex(
            "FF 02 20 05 01 00 00 01 FA 78 85 03 3A 35 10 40 00 01 03 00 01 C7 CF FF \
             01 00 02 05 76 C3 FF 03",
        );
        assert_eq!(crc16(&reading[2..28]), 0xC376);
        assert_eq!(stored(&reading, 28), 0xC376);

        let info = hex("FF 01 18 04 01 02 66 55 44 33 22 11 00 00 00 00 04 00 00 01 FC 94 FF 03");
        assert_eq!(crc16(&info[2..20]), 0x94FC);
        assert_eq!(stored(&info, 20), 0x94FC);

        for command in [
            "FF 01 20 01 01 00 00 00 00 00 00 01 01 01 00 00 00 00 00 00 00 00 00 00 00 00 \
             00 00 11 35 FF 03",
            "FF 01 20 01 01 66 55 44 33 22 11 51 01 01 00 00 00 00 00 00 00 00 00 00 00 00 \
             00 00 45 43 FF 03",
        ] {
            let command = hex(command);
            assert_eq!(
                crc16(&command[2..28]),
                stored(&command, 28),
                "{command:02X?}"
            );
        }
    }

    /// Run over the CRC bytes too, a packet's CRC comes out 0 (spec §10).
    #[test]
    fn crc_over_the_crc_is_zero() {
        let info = hex("FF 01 18 04 01 02 66 55 44 33 22 11 00 00 00 00 04 00 00 01 FC 94 FF 03");
        assert_eq!(crc16(&info[2..22]), 0);
    }
}
