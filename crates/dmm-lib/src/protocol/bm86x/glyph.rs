//! The seven-segment characters and what a row of them shows
//! (`docs/research/bm86x/reverse-engineered-protocol.md` §7).
//!
//! Segments come in a-g order, a in bit 0 to g in bit 6, whatever byte and
//! bit a series puts them in: [`super::map::Map::segments`] does that
//! translation, so one table serves every map.

/// Each character §7.2 lists, by its segments, and `_` (d alone, the
/// meter's C_Er, spec §7.3), which neither of Brymen's tables has. "O" draws
/// as 0 and "S" as 5 (spec §7.2).
const TABLE: [(u8, char); 31] = [
    (0x00, ' '),
    (0x3F, '0'),
    (0x06, '1'),
    (0x5B, '2'),
    (0x4F, '3'),
    (0x66, '4'),
    (0x6D, '5'),
    (0x7D, '6'),
    (0x07, '7'),
    (0x7F, '8'),
    (0x6F, '9'),
    (0x40, '-'),
    (0x38, 'L'),
    (0x39, 'C'),
    (0x71, 'F'),
    (0x79, 'E'),
    (0x50, 'r'),
    (0x54, 'n'),
    (0x5C, 'o'),
    (0x04, 'i'),
    (0x78, 't'),
    (0x7C, 'b'),
    (0x3E, 'U'),
    (0x73, 'P'),
    (0x76, 'H'),
    (0x67, 'g'),
    (0x77, 'A'),
    (0x5E, 'd'),
    (0x6E, 'y'),
    (0x1C, 'u'),
    (0x08, '_'),
];

/// What a pattern the table does not list reads as, as Brymen's programs
/// show one (spec §7.2).
pub(super) const UNKNOWN: char = '?';

/// The character segments `segments` draw, [`UNKNOWN`] for a pattern §7.2
/// does not list.
pub(super) fn char_of(segments: u8) -> char {
    TABLE
        .iter()
        .find(|(s, _)| *s == segments)
        .map_or(UNKNOWN, |(_, c)| *c)
}

/// One digit position: its segments, a-g in bits 0-6, and whether a decimal
/// point is drawn after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) segments: u8,
    pub(super) point: bool,
}

/// What a row of cells shows.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Readout {
    /// The lit characters, blanks and points left out: "31271", "0L",
    /// "rE-0". Empty for a blank row.
    pub(super) word: String,
    /// The meter's own text: the sign, the lit characters and the point,
    /// the blanks around them dropped.
    pub(super) text: String,
    /// The number `text` is, when every lit cell is a digit and they stand
    /// together.
    pub(super) number: Option<f64>,
    /// A shape no source shows, to report: a blank between digits, or two
    /// points (the leftmost is kept).
    pub(super) odd: Option<&'static str>,
}

/// Read a row, `negative` being its minus sign.
pub(super) fn read(cells: &[Cell], negative: bool) -> Readout {
    let chars: Vec<char> = cells.iter().map(|c| char_of(c.segments)).collect();
    let mut points = cells.iter().enumerate().filter(|(_, c)| c.point);
    let point = points.next().map(|(i, _)| i);
    let mut odd = points.next().map(|_| "decimal points");
    let word: String = chars.iter().filter(|c| **c != ' ').collect();

    let lit = chars
        .iter()
        .position(|c| *c != ' ')
        .zip(chars.iter().rposition(|c| *c != ' '));
    let Some((first, last)) = lit else {
        return Readout {
            word,
            text: String::new(),
            number: None,
            odd,
        };
    };
    let mut text = String::with_capacity(cells.len() + 2);
    if negative {
        text.push('-');
    }
    // A point after a blank before the first character: ".0L" (BM860s
    // manual p.10).
    if point.is_some_and(|p| p < first) {
        text.push('.');
    }
    for (i, c) in chars.iter().enumerate().take(last + 1).skip(first) {
        text.push(*c);
        if point == Some(i) {
            text.push('.');
        }
    }

    let shown = &chars[first..=last];
    let digits_only = shown.iter().all(|c| *c == ' ' || c.is_ascii_digit());
    let number = if !digits_only {
        None
    } else if shown.contains(&' ') {
        odd = Some("blank digit");
        None
    } else {
        text.parse().ok()
    };
    Readout {
        word,
        text,
        number,
        odd,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The segments of a character, from the letters §7.2 gives it:
    /// "abcdef" is 0.
    pub(crate) fn segments_of(letters: &str) -> u8 {
        letters
            .bytes()
            .map(|l| 1u8 << (l - b'a'))
            .fold(0, |segments, bit| segments | bit)
    }

    /// Cells for `text`: each character its segments, "." the point after
    /// the character before it.
    pub(crate) fn cells(text: &str) -> Vec<Cell> {
        let mut cells: Vec<Cell> = Vec::new();
        for c in text.chars() {
            if c == '.' {
                cells.last_mut().expect("a point after a cell").point = true;
                continue;
            }
            let segments = TABLE
                .iter()
                .find(|(_, t)| *t == c)
                .map(|(s, _)| *s)
                .expect("a listed character");
            cells.push(Cell {
                segments,
                point: false,
            });
        }
        cells
    }

    /// §7.2's Segments column, transcribed apart from the table.
    #[test]
    fn every_character_is_its_segments() {
        for (c, letters) in [
            (' ', ""),
            ('0', "abcdef"),
            ('1', "bc"),
            ('2', "abdeg"),
            ('3', "abcdg"),
            ('4', "bcfg"),
            ('5', "acdfg"),
            ('6', "acdefg"),
            ('7', "abc"),
            ('8', "abcdefg"),
            ('9', "abcdfg"),
            ('-', "g"),
            ('L', "def"),
            ('C', "adef"),
            ('F', "aefg"),
            ('E', "adefg"),
            ('r', "eg"),
            ('n', "ceg"),
            ('o', "cdeg"),
            ('i', "c"),
            ('t', "defg"),
            ('b', "cdefg"),
            ('U', "bcdef"),
            ('P', "abefg"),
            ('H', "bcefg"),
            ('g', "abcfg"),
            ('A', "abcefg"),
            ('d', "bcdeg"),
            ('y', "bcdfg"),
            ('u', "cde"),
            ('_', "d"),
        ] {
            assert_eq!(char_of(segments_of(letters)), c, "{letters:?}");
        }
        assert_eq!(TABLE.len(), 31);
    }

    /// No two characters share segments: a lookup is never ambiguous.
    #[test]
    fn no_two_characters_share_segments() {
        for (i, (s, c)) in TABLE.iter().enumerate() {
            assert!(
                !TABLE[i + 1..].iter().any(|(t, _)| t == s),
                "{c:?} {s:#04x}"
            );
        }
    }

    /// §7.2's Bs86x column, BM860 digit bytes with bit 0 clear, read through
    /// the BM860 map; bit 0, a point or an annunciator, changes nothing.
    #[test]
    fn bm860_digit_bytes_read_as_brymen_reads_them() {
        use super::super::map::BM860;
        for (byte, c) in [
            (0x00, ' '),
            (0xBE, '0'),
            (0xA0, '1'),
            (0xDA, '2'),
            (0xF8, '3'),
            (0xE4, '4'),
            (0x7C, '5'),
            (0x7E, '6'),
            (0xA8, '7'),
            (0xFE, '8'),
            (0xFC, '9'),
            (0x40, '-'),
            (0x16, 'L'),
            (0x1E, 'C'),
            (0x4E, 'F'),
            (0x5E, 'E'),
            (0x42, 'r'),
            (0x62, 'n'),
            (0x72, 'o'),
            (0x20, 'i'),
            (0x56, 't'),
            (0x76, 'b'),
            (0xB6, 'U'),
            (0xCE, 'P'),
            (0xE6, 'H'),
            (0xEC, 'g'),
            (0xEE, 'A'),
            (0xF2, 'd'),
            (0xF4, 'y'),
            // In no Bs86x table: c d e, and d alone (spec §7.2).
            (0x32, 'u'),
            (0x10, '_'),
        ] {
            assert_eq!(char_of(BM860.segments(byte)), c, "{byte:#04x}");
            assert_eq!(char_of(BM860.segments(byte | 0x01)), c, "{byte:#04x}");
        }
    }

    /// §7.2's Bs8252x column, BM820 digit bytes with bit 4 clear, read
    /// through the BM820 map; bit 4, a point or an annunciator, changes
    /// nothing.
    #[test]
    fn bm820_digit_bytes_read_as_brymen_reads_them() {
        use super::super::map::BM820;
        for (byte, c) in [
            (0x00, ' '),
            (0xAF, '0'),
            (0xA0, '1'),
            (0xCB, '2'),
            (0xE9, '3'),
            (0xE4, '4'),
            (0x6D, '5'),
            (0x6F, '6'),
            (0xA8, '7'),
            (0xEF, '8'),
            (0xED, '9'),
            (0x40, '-'),
            (0x07, 'L'),
            (0x0F, 'C'),
            (0x4E, 'F'),
            (0x4F, 'E'),
            (0x42, 'r'),
            (0x62, 'n'),
            (0x63, 'o'),
            (0x20, 'i'),
            (0x47, 't'),
            (0x67, 'b'),
            (0xA7, 'U'),
            (0xCE, 'P'),
            (0xE6, 'H'),
            (0xEC, 'g'),
            (0xEE, 'A'),
            (0xE3, 'd'),
            (0xE5, 'y'),
            (0x23, 'u'),
            // In no Bs8252x table: d alone (spec §7.2).
            (0x01, '_'),
        ] {
            assert_eq!(char_of(BM820.segments(byte)), c, "{byte:#04x}");
            assert_eq!(char_of(BM820.segments(byte | 0x10)), c, "{byte:#04x}");
        }
    }

    #[test]
    fn an_unlisted_pattern_is_unknown() {
        // I as e f, one guess at how the meter draws InEr's I (spec §7.3).
        assert_eq!(char_of(segments_of("ef")), UNKNOWN);
        assert_eq!(char_of(0x7F | 0x80), UNKNOWN);
    }

    fn readout(text: &str, negative: bool) -> Readout {
        read(&cells(text), negative)
    }

    #[test]
    fn a_number_keeps_the_meters_digits() {
        let r = readout("31271 ", false);
        let r2 = readout("312.71 ", false);
        assert_eq!((r.text.as_str(), r.number), ("31271", Some(31271.0)));
        assert_eq!((r2.text.as_str(), r2.number), ("312.71", Some(312.71)));
        assert_eq!(r2.word, "31271");
        assert_eq!(r2.odd, None);
        let negative = readout(" 0.0120", true);
        assert_eq!(negative.text, "-0.0120");
        assert_eq!(negative.number, Some(-0.012));
    }

    #[test]
    fn a_word_is_its_lit_characters() {
        let ol = readout(" .0L  ", false);
        assert_eq!((ol.word.as_str(), ol.text.as_str()), ("0L", ".0L"));
        assert_eq!(ol.number, None);
        assert_eq!(readout("rE-0", false).word, "rE-0");
        assert_eq!(readout("  C_Er", false).word, "C_Er");
        let blank = readout("      ", false);
        assert_eq!((blank.word.as_str(), blank.text.as_str()), ("", ""));
        assert_eq!((blank.number, blank.odd), (None, None));
    }

    #[test]
    fn a_blank_between_digits_or_a_second_point_is_odd() {
        let gap = readout("12 45", false);
        assert_eq!((gap.number, gap.odd), (None, Some("blank digit")));
        let two = readout("1.2.34", false);
        assert_eq!(two.odd, Some("decimal points"));
        assert_eq!((two.text.as_str(), two.number), ("1.234", Some(1.234)));
    }
}
