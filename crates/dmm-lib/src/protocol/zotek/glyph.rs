//! The LCD's seven-segment glyphs and what a row of them spells
//! (`docs/research/zotek/reverse-engineered-protocol.md` §6).
//!
//! The packets carry segments, not numbers: a digit is a glyph byte, and the
//! words the meter shows instead of a reading (Auto, EF, OL, dashes) are
//! glyph patterns. Which layout bytes form each row is the layout's business
//! ([`super::layout`]); this reads the row.

use super::Unrecognised;

/// The bit a glyph byte carries a decimal point (or a sign) in, which a
/// lookup masks out (spec §6.1).
pub(super) const DP: u8 = 0x10;

/// One seven-segment glyph (spec §6.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Glyph {
    Digit(u8),
    Blank,
    A,
    E,
    F,
    L,
    /// `o`, the last letter of `Auto`.
    O,
    /// `u`
    U,
    /// `t`
    T,
    Dash,
    /// `b`, `C` and `d`: in one app's table, used by neither (spec §6.1).
    B,
    C,
    D,
    /// A segment pattern §6.1 does not list.
    Unknown(u8),
}

impl Glyph {
    /// The glyph a segment byte draws, its DP bit ignored (spec §6.1).
    pub(super) fn from_segments(code: u8) -> Self {
        match code & !DP {
            0xEB => Glyph::Digit(0),
            0x0A => Glyph::Digit(1),
            0xAD => Glyph::Digit(2),
            0x8F => Glyph::Digit(3),
            0x4E => Glyph::Digit(4),
            0xC7 => Glyph::Digit(5),
            0xE7 => Glyph::Digit(6),
            0x8A => Glyph::Digit(7),
            0xEF => Glyph::Digit(8),
            0xCF => Glyph::Digit(9),
            0xEE => Glyph::A,
            0xE5 => Glyph::E,
            0xE4 => Glyph::F,
            0x61 => Glyph::L,
            0x27 => Glyph::O,
            0x23 => Glyph::U,
            0x65 => Glyph::T,
            0x04 => Glyph::Dash,
            0x67 => Glyph::B,
            0xE1 => Glyph::C,
            0x2F => Glyph::D,
            0x00 => Glyph::Blank,
            other => Glyph::Unknown(other),
        }
    }

    /// How the glyph reads in `display_raw`: a blank is a space, as the
    /// character-display families pad theirs.
    fn char(self) -> char {
        match self {
            Glyph::Digit(d) => char::from(b'0' + d),
            Glyph::Blank => ' ',
            Glyph::A => 'A',
            Glyph::E => 'E',
            Glyph::F => 'F',
            Glyph::L => 'L',
            Glyph::O => 'o',
            Glyph::U => 'u',
            Glyph::T => 't',
            Glyph::Dash => '-',
            Glyph::B => 'b',
            Glyph::C => 'C',
            Glyph::D => 'd',
            Glyph::Unknown(_) => '?',
        }
    }
}

/// One digit position, most significant first: its glyph, and whether a
/// decimal point sits before it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Cell {
    pub(super) glyph: Glyph,
    pub(super) dp: bool,
}

/// What a row of glyphs shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Shown {
    Number(f64),
    /// Any `L` in the row: `0L` with the point where the range puts it
    /// (spec §6.4, §11.4).
    Overload,
    /// `E` `F`, the NCV idle word (spec §6.4).
    Ef,
    /// One to four dashes and nothing else (spec §6.4, §11.4).
    Dashes(u8),
    /// `A` `u` `t` `o` (spec §6.4).
    Auto,
    /// A glyph or a pattern the spec does not give, already reported.
    Unrecognised,
}

/// A row as read: what it shows, and its text in the meter's precision.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Readout {
    pub(super) shown: Shown,
    pub(super) text: String,
}

/// Read a row of cells, `negative` being its minus sign.
///
/// A blank counts as 0 in the number (spec §6.1). Only one decimal point is
/// expected; with two the meaning is open (spec §6.2, §10), so it is
/// reported and the leftmost kept.
pub(super) fn read(unrecognised: Unrecognised, cells: &[Cell], negative: bool) -> Readout {
    let mut points = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| c.dp)
        .map(|(i, _)| i);
    let dp_at = points.next();
    if points.next().is_some() {
        unrecognised.report("decimal points");
    }

    let mut text = String::with_capacity(cells.len() + 2);
    if negative {
        text.push('-');
    }
    for (i, cell) in cells.iter().enumerate() {
        if dp_at == Some(i) {
            text.push('.');
        }
        text.push(cell.glyph.char());
    }

    let lit: Vec<Glyph> = cells
        .iter()
        .map(|c| c.glyph)
        .filter(|g| *g != Glyph::Blank)
        .collect();
    let shown = if lit.iter().any(|g| matches!(g, Glyph::Unknown(_))) {
        unrecognised.report("digit glyph");
        Shown::Unrecognised
    } else if lit.contains(&Glyph::L) {
        Shown::Overload
    } else if lit == [Glyph::A, Glyph::U, Glyph::T, Glyph::O] {
        Shown::Auto
    } else if lit == [Glyph::E, Glyph::F] {
        Shown::Ef
    } else if !lit.is_empty() && lit.iter().all(|g| *g == Glyph::Dash) {
        Shown::Dashes(lit.len() as u8)
    } else if lit.iter().all(|g| matches!(g, Glyph::Digit(_))) {
        number(cells, dp_at, negative)
    } else {
        unrecognised.report("display text");
        Shown::Unrecognised
    };
    Readout { shown, text }
}

/// The number a row of digits and blanks shows.
fn number(cells: &[Cell], dp_at: Option<usize>, negative: bool) -> Shown {
    // A leading 0 so a point before the first digit still parses.
    let mut s = String::with_capacity(cells.len() + 3);
    if negative {
        s.push('-');
    }
    s.push('0');
    for (i, cell) in cells.iter().enumerate() {
        if dp_at == Some(i) {
            s.push('.');
        }
        s.push(match cell.glyph {
            Glyph::Digit(d) => char::from(b'0' + d),
            _ => '0',
        });
    }
    // Digits, a point and a sign always parse.
    s.parse().map_or(Shown::Unrecognised, Shown::Number)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The segment byte of each glyph, as the §6.1 table gives it.
    pub(crate) const DIGITS: [u8; 10] =
        [0xEB, 0x0A, 0xAD, 0x8F, 0x4E, 0xC7, 0xE7, 0x8A, 0xEF, 0xCF];
    pub(crate) const A: u8 = 0xEE;
    pub(crate) const E: u8 = 0xE5;
    pub(crate) const F: u8 = 0xE4;
    pub(crate) const L: u8 = 0x61;
    pub(crate) const O: u8 = 0x27;
    pub(crate) const U: u8 = 0x23;
    pub(crate) const T: u8 = 0x65;
    pub(crate) const DASH: u8 = 0x04;
    pub(crate) const BLANK: u8 = 0x00;

    /// The segment byte for a character of a display: `0`-`9`, a space for
    /// a blank, or one of the §6.1 letters.
    pub(crate) fn segments(c: char) -> u8 {
        match c {
            '0'..='9' => DIGITS[c as usize - '0' as usize],
            ' ' => BLANK,
            'A' => A,
            'E' => E,
            'F' => F,
            'L' => L,
            'o' => O,
            'u' => U,
            't' => T,
            '-' => DASH,
            'b' => 0x67,
            'C' => 0xE1,
            'd' => 0x2F,
            _ => panic!("no glyph for {c:?}"),
        }
    }

    fn cells(glyphs: &str, dp_at: Option<usize>) -> Vec<Cell> {
        glyphs
            .chars()
            .enumerate()
            .map(|(i, c)| Cell {
                glyph: Glyph::from_segments(segments(c)),
                dp: dp_at == Some(i),
            })
            .collect()
    }

    fn quiet() -> Unrecognised<'static> {
        Unrecognised {
            id: "test",
            packet: &[],
        }
    }

    fn read_quietly(glyphs: &str, dp_at: Option<usize>, negative: bool) -> Readout {
        let (readout, reports) =
            crate::protocol::capture_reports(|| read(quiet(), &cells(glyphs, dp_at), negative));
        assert!(reports.is_empty(), "{glyphs:?}: {reports:?}");
        readout
    }

    #[test]
    fn every_listed_code_reads_as_its_glyph() {
        for (d, code) in DIGITS.iter().enumerate() {
            assert_eq!(Glyph::from_segments(*code), Glyph::Digit(d as u8));
            assert_eq!(Glyph::from_segments(code | DP), Glyph::Digit(d as u8));
        }
        for c in "AEFLout-bCd ".chars() {
            assert!(!matches!(
                Glyph::from_segments(segments(c)),
                Glyph::Unknown(_)
            ));
        }
        assert_eq!(Glyph::from_segments(0x10), Glyph::Blank);
        assert_eq!(Glyph::from_segments(0x01), Glyph::Unknown(0x01));
        assert_eq!(Glyph::from_segments(0x11), Glyph::Unknown(0x01));
    }

    #[test]
    fn a_number_keeps_the_meters_precision() {
        let r = read_quietly("1234", Some(2), true);
        assert_eq!(r.text, "-12.34");
        assert_eq!(r.shown, Shown::Number(-12.34));
        let r = read_quietly("0000", Some(1), false);
        assert_eq!(r.text, "0.000");
        assert_eq!(r.shown, Shown::Number(0.0));
    }

    /// A blank counts as 0 and reads as a space (spec §6.1).
    #[test]
    fn a_blank_is_a_padding_zero() {
        let r = read_quietly(" 123", Some(3), false);
        assert_eq!(r.text, " 12.3");
        assert_eq!(r.shown, Shown::Number(12.3));
    }

    /// `10`: a blank carrying the point (spec Implementation Notes).
    #[test]
    fn a_point_on_a_blank_still_places_the_point() {
        let r = read_quietly(" 123", Some(0), false);
        assert_eq!(r.text, ". 123");
        assert_eq!(r.shown, Shown::Number(0.0123));
    }

    /// Two points: reported, and the leftmost kept.
    #[test]
    fn two_points_report_and_keep_the_leftmost() {
        let mut row = cells("1234", Some(1));
        row[3].dp = true;
        let (r, reports) = crate::protocol::capture_reports(|| read(quiet(), &row, false));
        assert_eq!(r.shown, Shown::Number(1.234));
        assert_eq!(r.text, "1.234");
        assert_eq!(reports.len(), 1, "{reports:?}");
        assert!(reports[0].contains("decimal points"), "{reports:?}");
    }

    /// The three OL forms community captures show on types 1-3, with the
    /// point moving by range (spec §11.4).
    #[test]
    fn every_ol_form_is_an_overload() {
        for (dp_at, text) in [(Some(2), " 0.L "), (Some(1), " .0L "), (Some(3), " 0L. ")] {
            let r = read_quietly(" 0L ", dp_at, false);
            assert_eq!(r.shown, Shown::Overload);
            assert_eq!(r.text, text);
        }
    }

    #[test]
    fn the_words() {
        assert_eq!(read_quietly("Auto", None, false).shown, Shown::Auto);
        assert_eq!(read_quietly(" EF ", None, false).shown, Shown::Ef);
        for (row, n) in [("-   ", 1), ("--  ", 2), ("--- ", 3), ("----", 4)] {
            let r = read_quietly(row, None, false);
            assert_eq!(r.shown, Shown::Dashes(n));
            assert_eq!(r.text, row);
        }
    }

    #[test]
    fn an_unlisted_glyph_is_reported() {
        let mut row = cells("12 4", None);
        row[2].glyph = Glyph::from_segments(0x01);
        let (r, reports) = crate::protocol::capture_reports(|| read(quiet(), &row, false));
        assert_eq!(r.shown, Shown::Unrecognised);
        assert_eq!(r.text, "12?4");
        assert_eq!(reports.len(), 1, "{reports:?}");
    }

    /// Letters that spell no word the spec gives.
    #[test]
    fn a_pattern_that_is_no_word_is_reported() {
        for row in ["bCd ", " E  ", "12Ad", "FE  ", "Aut "] {
            let (r, reports) =
                crate::protocol::capture_reports(|| read(quiet(), &cells(row, None), false));
            assert_eq!(r.shown, Shown::Unrecognised, "{row:?}");
            assert_eq!(reports.len(), 1, "{row:?}: {reports:?}");
        }
    }
}
