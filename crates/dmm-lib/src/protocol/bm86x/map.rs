//! Where each LCD segment sits in a reply
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §5.1, §6.1, §7.1).
//!
//! A reply is 24 data bytes: Brymen's bytes 2-9, 11-18 and 20-27, bytes 1,
//! 10 and 19 being the report IDs the wire does not carry (spec §4.1). Every
//! cell here is written in Brymen's numbering, byte n and bit b as the
//! sheet's Table 1 prints them, through [`at`], so a line reads against the
//! sheet.

/// One bit of a reply's data bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bit {
    pub(super) index: usize,
    pub(super) mask: u8,
}

impl Bit {
    pub(super) fn lit(self, data: &[u8]) -> bool {
        data[self.index] & self.mask != 0
    }
}

/// The data index of Brymen's byte `byte`, 1-based as Table 1 counts it:
/// `None` for a report-ID byte (1, 10, 19) or one past the reply (spec
/// §4.1).
pub(super) const fn data_index(byte: usize) -> Option<usize> {
    match byte {
        2..=9 => Some(byte - 2),
        11..=18 => Some(byte - 3),
        20..=27 => Some(byte - 4),
        _ => None,
    }
}

/// Bit `bit` of Brymen's byte `byte`. A report-ID byte is refused: in a
/// static that fails the build.
pub(super) const fn at(byte: usize, bit: u32) -> Bit {
    assert!(bit < 8, "a byte has bits 0-7");
    match data_index(byte) {
        Some(index) => Bit {
            index,
            mask: 1 << bit,
        },
        None => panic!("bytes 1, 10 and 19 are report IDs, and a reply ends at byte 27"),
    }
}

/// The data index of Brymen's byte `byte`, for the tables below.
const fn byte(byte: usize) -> usize {
    at(byte, 0).index
}

/// Where the series code sits: byte 23, the one model byte every sheet
/// names (spec §4.2).
pub(super) const MODEL: usize = byte(23);

/// Every annunciator of either Table 1 but the minus signs and the decimal
/// points, which belong to their rows (spec §5.1, §6.1). A `1` suffix is
/// the main display's symbol (circled ① in the sheets), a `2` the
/// secondary display's (②).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ann {
    Auto,
    Record,
    Crest,
    Hold,
    Dc1,
    Max,
    Min,
    Avg,
    Ac1,
    T1,
    /// The dash between T1 and T2 (spec §5.5).
    T1T2Dash,
    T2,
    BarScale,
    /// The bar graph's minus (spec §5.4).
    BarMinus,
    Vfd,
    /// Relative zero, △ (spec §5.5).
    Rel,
    V1,
    Micro2,
    Milli2,
    A2,
    /// %4~20mA, the loop current's percentage (spec §8.1).
    Loop,
    Ac2,
    T2Sub,
    LowBattery,
    Continuity,
    Mega2,
    Kilo2,
    Hz2,
    V2,
    Siemens1,
    Farad1,
    Nano1,
    A1,
    Hz1,
    Db,
    Milli1,
    Micro1,
    Ohm1,
    Mega1,
    Kilo1,
    Duty1,
    // The BM820 map's own (spec §6.1, §6.5).
    Hi,
    Lo,
    /// The dash of "MAX-MIN" (spec §6.5).
    MaxMinDash,
    /// The "%" beside △ (spec §6.5).
    Percent,
    /// Low impedance, lit in AutoCheck (spec §6.5).
    LoZ,
    Lpf,
    /// "@" (spec §6.5).
    At,
    Dc2,
    T1Sub,
    Duty2,
    Nano2,
    Siemens2,
    Farad2,
    Ohm2,
}

impl Ann {
    fn flag(self) -> u64 {
        1 << self as u32
    }
}

/// The annunciators a reply lights.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Lit(u64);

impl Lit {
    pub(super) fn read(map: &Map, data: &[u8]) -> Self {
        Lit(map
            .annunciators
            .iter()
            .filter(|(_, bit)| bit.lit(data))
            .fold(0, |lit, (ann, _)| lit | ann.flag()))
    }

    pub(super) fn has(self, ann: Ann) -> bool {
        self.0 & ann.flag() != 0
    }

    /// Those of `anns` that are lit.
    pub(super) fn among<const N: usize>(self, anns: [Ann; N]) -> impl Iterator<Item = Ann> {
        anns.into_iter().filter(move |a| self.has(*a))
    }

    /// The one of `anns` lit; `None` for none or several.
    pub(super) fn one_of<const N: usize>(self, anns: [Ann; N]) -> Option<Ann> {
        let mut on = self.among(anns);
        let first = on.next();
        on.next().is_none().then_some(first).flatten()
    }
}

/// One display's digits: their bytes, most significant first, the decimal
/// points, and the minus sign.
pub(super) struct Row {
    pub(super) digits: &'static [usize],
    /// Each point as the digit it is drawn after (0-based in `digits`) and
    /// its bit.
    pub(super) points: &'static [(usize, Bit)],
    pub(super) minus: Bit,
}

/// One series' LCD map.
pub(super) struct Map {
    /// The bit of each segment a-g in a digit byte (spec §7.1).
    pub(super) segments: [u8; 7],
    pub(super) main: Row,
    pub(super) secondary: Row,
    pub(super) annunciators: &'static [(Ann, Bit)],
}

impl Map {
    /// The segments digit byte `byte` lights, a in bit 0 to g in bit 6, the
    /// form [`super::glyph`] reads.
    pub(super) fn segments(&self, byte: u8) -> u8 {
        self.segments
            .iter()
            .enumerate()
            .filter(|(_, mask)| byte & **mask != 0)
            .fold(0, |segments, (i, _)| segments | 1 << i)
    }
}

/// The BM860s: BM860 Table 1 (spec §5.1). Segments b g c d a f e in bits
/// 7-1 of each digit byte, bit 0 an annunciator or the point of the digit
/// before (spec §5.2, §7.1).
pub(super) static BM860: Map = Map {
    // a, b, c, d, e, f, g.
    segments: [0x08, 0x80, 0x20, 0x10, 0x02, 0x04, 0x40],
    main: Row {
        digits: &[byte(5), byte(6), byte(7), byte(8), byte(9), byte(11)],
        // 1p-4p, each in bit 0 of the next digit's byte; no 5p (spec §5.2).
        points: &[(0, at(6, 0)), (1, at(7, 0)), (2, at(8, 0)), (3, at(9, 0))],
        // ▭ ① (spec §5.3).
        minus: at(4, 7),
    },
    secondary: Row {
        digits: &[byte(13), byte(14), byte(15), byte(16)],
        // 7p-9p (spec §5.2).
        points: &[(0, at(14, 0)), (1, at(15, 0)), (2, at(16, 0))],
        // ▭ ② (spec §5.3).
        minus: at(12, 4),
    },
    annunciators: &[
        (Ann::Avg, at(3, 7)),
        (Ann::Min, at(3, 6)),
        (Ann::Max, at(3, 5)),
        (Ann::Dc1, at(3, 4)),
        (Ann::Hold, at(3, 3)),
        (Ann::Crest, at(3, 2)),
        (Ann::Record, at(3, 1)),
        (Ann::Auto, at(3, 0)),
        (Ann::Vfd, at(4, 6)),
        (Ann::BarMinus, at(4, 5)),
        (Ann::BarScale, at(4, 4)),
        (Ann::T2, at(4, 3)),
        (Ann::T1T2Dash, at(4, 2)),
        (Ann::T1, at(4, 1)),
        (Ann::Ac1, at(4, 0)),
        (Ann::Rel, at(5, 0)),
        (Ann::V1, at(11, 0)),
        (Ann::LowBattery, at(12, 7)),
        (Ann::T2Sub, at(12, 6)),
        (Ann::Ac2, at(12, 5)),
        (Ann::Loop, at(12, 3)),
        (Ann::A2, at(12, 2)),
        (Ann::Milli2, at(12, 1)),
        (Ann::Micro2, at(12, 0)),
        (Ann::Continuity, at(13, 0)),
        (Ann::A1, at(17, 7)),
        (Ann::Nano1, at(17, 6)),
        (Ann::Farad1, at(17, 5)),
        (Ann::Siemens1, at(17, 4)),
        (Ann::V2, at(17, 3)),
        (Ann::Hz2, at(17, 2)),
        (Ann::Kilo2, at(17, 1)),
        (Ann::Mega2, at(17, 0)),
        (Ann::Duty1, at(18, 7)),
        (Ann::Kilo1, at(18, 6)),
        (Ann::Mega1, at(18, 5)),
        (Ann::Ohm1, at(18, 4)),
        (Ann::Micro1, at(18, 3)),
        (Ann::Milli1, at(18, 2)),
        (Ann::Db, at(18, 1)),
        (Ann::Hz1, at(18, 0)),
    ],
};

/// The BM820s and BM520s: BM820 Table 1, report II's ID in byte 10 (spec
/// §4.3, §6.1). Segments b g c in bits 7-5 and a f e d in bits 3-0 of each
/// digit byte, bit 4 the point after that digit or an annunciator (spec
/// §6.2, §7.1).
pub(super) static BM820: Map = Map {
    // a, b, c, d, e, f, g.
    segments: [0x08, 0x80, 0x20, 0x01, 0x02, 0x04, 0x40],
    main: Row {
        digits: &[byte(5), byte(6), byte(7), byte(8)],
        // 1P-3P, each in its own digit's byte; 8.4 is dB (spec §6.2).
        points: &[(0, at(5, 4)), (1, at(6, 4)), (2, at(7, 4))],
        // ① ▭ (spec §6.3).
        minus: at(4, 7),
    },
    secondary: Row {
        digits: &[byte(11), byte(12), byte(13), byte(14)],
        // 4P-6P; 14.4 is %4~20mA (spec §6.2).
        points: &[(0, at(11, 4)), (1, at(12, 4)), (2, at(13, 4))],
        // ② ▭ (spec §6.3).
        minus: at(9, 5),
    },
    annunciators: &[
        (Ann::Hi, at(3, 7)),
        (Ann::Lo, at(3, 6)),
        (Ann::Dc1, at(3, 5)),
        (Ann::Ac1, at(3, 4)),
        (Ann::Min, at(3, 3)),
        (Ann::MaxMinDash, at(3, 2)),
        (Ann::Avg, at(3, 1)),
        (Ann::Max, at(3, 0)),
        (Ann::Rel, at(4, 6)),
        (Ann::Percent, at(4, 5)),
        (Ann::LoZ, at(4, 4)),
        (Ann::T2, at(4, 3)),
        (Ann::Lpf, at(4, 2)),
        (Ann::T1T2Dash, at(4, 1)),
        (Ann::T1, at(4, 0)),
        (Ann::Db, at(8, 4)),
        (Ann::Dc2, at(9, 7)),
        (Ann::Ac2, at(9, 6)),
        (Ann::At, at(9, 4)),
        (Ann::LowBattery, at(9, 3)),
        (Ann::T1Sub, at(9, 2)),
        (Ann::T2Sub, at(9, 1)),
        (Ann::Continuity, at(9, 0)),
        (Ann::Loop, at(14, 4)),
        (Ann::Milli2, at(15, 7)),
        (Ann::Micro2, at(15, 6)),
        (Ann::A2, at(15, 5)),
        (Ann::V2, at(15, 4)),
        (Ann::Duty2, at(15, 3)),
        (Ann::Nano2, at(15, 2)),
        (Ann::Siemens2, at(15, 1)),
        (Ann::Farad2, at(15, 0)),
        (Ann::Kilo1, at(16, 7)),
        (Ann::Mega1, at(16, 6)),
        (Ann::Ohm1, at(16, 5)),
        (Ann::Hz1, at(16, 4)),
        (Ann::Mega2, at(16, 3)),
        (Ann::Kilo2, at(16, 2)),
        (Ann::Ohm2, at(16, 1)),
        (Ann::Hz2, at(16, 0)),
        (Ann::Micro1, at(17, 7)),
        (Ann::Milli1, at(17, 6)),
        (Ann::V1, at(17, 5)),
        (Ann::A1, at(17, 4)),
        (Ann::Nano1, at(17, 3)),
        (Ann::Duty1, at(17, 2)),
        (Ann::Siemens1, at(17, 1)),
        (Ann::Farad1, at(17, 0)),
        // Byte 18 is "don't care": the programs' "mV" bit 18.3 stays out
        // (spec §8.1).
        (Ann::Hold, at(24, 7)),
        (Ann::Crest, at(24, 6)),
        (Ann::Record, at(24, 5)),
        (Ann::Auto, at(24, 4)),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bit the map names: the digits' segments, the points, the minus
    /// signs and the annunciators.
    fn every_bit(map: &Map) -> Vec<Bit> {
        let mut bits: Vec<Bit> = Vec::new();
        for row in [&map.main, &map.secondary] {
            for &index in row.digits {
                bits.extend(map.segments.iter().map(|&mask| Bit { index, mask }));
            }
            bits.extend(row.points.iter().map(|(_, bit)| *bit));
            bits.push(row.minus);
        }
        bits.extend(map.annunciators.iter().map(|(_, bit)| *bit));
        bits
    }

    #[test]
    fn no_bit_is_named_twice() {
        for map in [&BM860, &BM820] {
            let bits = every_bit(map);
            for (i, bit) in bits.iter().enumerate() {
                assert!(bit.mask.is_power_of_two(), "{bit:?}");
                assert!(!bits[i + 1..].contains(bit), "{bit:?} is named twice");
            }
        }
    }

    /// BM820 Table 1 fills bytes 3-9 and 11-17 to the last bit, and the
    /// top half of byte 24; 2, 18, 20-23 (the model bytes), the rest of 24
    /// and 25-27 are "don't care" (spec §6.1).
    #[test]
    fn the_bm820_map_is_table_1_whole() {
        let bits = every_bit(&BM820);
        let mut named = [0u8; 24];
        for bit in &bits {
            named[bit.index] |= bit.mask;
        }
        for byte in (3..=9).chain(11..=17) {
            assert_eq!(named[data_index(byte).unwrap()], 0xFF, "byte {byte}");
        }
        assert_eq!(named[data_index(24).unwrap()], 0xF0);
        for byte in [2, 18].into_iter().chain(20..=23).chain(25..=27) {
            assert_eq!(named[data_index(byte).unwrap()], 0x00, "byte {byte}");
        }
        assert_eq!(bits.len(), 14 * 8 + 4);
    }

    /// `Lit` keeps one bit per annunciator in a `u64`.
    #[test]
    fn every_annunciator_fits_lit() {
        assert!((Ann::Ohm2 as u32) < u64::BITS);
    }

    /// Table 1 fills bytes 3-9 and 11-18 to the last bit; 2 and 20-27 hold
    /// "don't care" bits and the model bytes (spec §5.1).
    #[test]
    fn the_bm860_map_is_table_1_whole() {
        let bits = every_bit(&BM860);
        let mut named = [0u8; 24];
        for bit in &bits {
            named[bit.index] |= bit.mask;
        }
        for byte in (3..=9).chain(11..=18) {
            assert_eq!(named[data_index(byte).unwrap()], 0xFF, "byte {byte}");
        }
        for byte in [2].into_iter().chain(20..=27) {
            assert_eq!(named[data_index(byte).unwrap()], 0x00, "byte {byte}");
        }
        assert_eq!(bits.len(), 15 * 8);
    }

    /// No two annunciators share a name, so `Lit` cannot confuse them.
    #[test]
    fn each_annunciator_is_named_once() {
        for map in [&BM860, &BM820] {
            let anns: Vec<Ann> = map.annunciators.iter().map(|(a, _)| *a).collect();
            for (i, ann) in anns.iter().enumerate() {
                assert!(!anns[i + 1..].contains(ann), "{ann:?}");
            }
        }
    }

    #[test]
    fn brymen_bytes_skip_the_report_ids() {
        assert_eq!(data_index(2), Some(0));
        assert_eq!(data_index(9), Some(7));
        assert_eq!(data_index(11), Some(8));
        assert_eq!(data_index(18), Some(15));
        assert_eq!(data_index(20), Some(16));
        assert_eq!(data_index(23), Some(19));
        assert_eq!(data_index(27), Some(23));
        for byte in [0, 1, 10, 19, 28] {
            assert_eq!(data_index(byte), None, "{byte}");
        }
        assert_eq!(MODEL, 19);
        assert_eq!(
            at(12, 4),
            Bit {
                index: 9,
                mask: 0x10
            }
        );
    }

    #[test]
    fn at_refuses_a_report_id_byte() {
        for byte in [1, 10, 19] {
            assert!(std::panic::catch_unwind(|| at(byte, 0)).is_err(), "{byte}");
        }
        assert!(std::panic::catch_unwind(|| at(3, 8)).is_err());
    }

    /// The segments come out in a-g order whatever the series puts where
    /// (spec §7.1): `BE` is 0, a to f.
    #[test]
    fn a_digit_byte_reads_as_segments_a_to_g() {
        assert_eq!(BM860.segments(0xBE), 0b011_1111);
        assert_eq!(BM860.segments(0xBF), 0b011_1111, "bit 0 is no segment");
        assert_eq!(BM860.segments(0x40), 0b100_0000);
        // BM820: `AF` is 0, and bit 4 is no segment.
        assert_eq!(BM820.segments(0xAF), 0b011_1111);
        assert_eq!(BM820.segments(0xBF), 0b011_1111);
        assert_eq!(BM820.segments(0x01), 0b000_1000);
    }
}
