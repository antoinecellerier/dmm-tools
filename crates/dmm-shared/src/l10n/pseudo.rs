//! The padding pseudo-locale: every message accented, padded by about 40%
//! and bracketed, so a layout that can't take longer text shows it, and a
//! string that never went through the localizer stands out unbracketed.
//!
//! Capitals are accented too. They fill the reading area (HOLD, REL, AUTO),
//! and a scheme that only touched lowercase vowels would leave them at
//! their English width.

use std::borrow::Cow;

/// Fluent's per-text-element transform: accents on every ASCII letter, then
/// `~` padding of 40% of the letter count, rounded up. A unit symbol right
/// after a number ("10 MΩ", "40Hz") stays as written, as it does in every
/// language, so pseudo spec text keeps its numbers and shows
/// ([`Localizer::spec_text`](super::Localizer::spec_text)).
///
/// Fluent applies it to each run of text in a pattern, never to a
/// placeable's value, so a model name passed in stays readable. Trailing
/// whitespace stays after the padding: "Connect " becomes "Çóññéçţ~~~ ".
pub(super) fn transform(s: &str) -> Cow<'_, str> {
    let letters = s.chars().filter(char::is_ascii_alphabetic).count();
    if letters == 0 {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() * 2);
    // Whether the last character that wasn't a space was a digit.
    let mut after_digit = false;
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        let word_len = rest
            .find(|c: char| !(c.is_alphabetic() || c == '°'))
            .unwrap_or(rest.len());
        if word_len > 0 {
            let word = &rest[..word_len];
            if after_digit && super::lib_index::is_unit(word) {
                out.push_str(word);
            } else {
                out.extend(word.chars().map(accent));
            }
            after_digit = false;
            rest = &rest[word_len..];
            continue;
        }
        out.push(c);
        if !c.is_whitespace() {
            after_digit = c.is_ascii_digit();
        }
        rest = &rest[c.len_utf8()..];
    }
    let tail = out.split_off(out.trim_end().len());
    out.extend(std::iter::repeat_n('~', (letters * 2).div_ceil(5)));
    out.push_str(&tail);
    Cow::Owned(out)
}

/// Brackets around a whole formatted message. Added after formatting, not
/// in [`transform`], which sees one text run at a time.
pub(super) fn bracket(s: &str) -> String {
    format!("[{s}]")
}

fn accent(c: char) -> char {
    const LOWER: [char; 26] = [
        'á', 'ƀ', 'ç', 'ð', 'é', 'ƒ', 'ĝ', 'ĥ', 'í', 'ĵ', 'ķ', 'ĺ', 'ɱ', 'ñ', 'ó', 'þ', 'ǫ', 'ŕ',
        'š', 'ţ', 'ú', 'ṽ', 'ŵ', 'ẋ', 'ý', 'ž',
    ];
    const UPPER: [char; 26] = [
        'Á', 'Ɓ', 'Ç', 'Ð', 'É', 'Ƒ', 'Ĝ', 'Ĥ', 'Í', 'Ĵ', 'Ķ', 'Ĺ', 'Ṁ', 'Ñ', 'Ó', 'Þ', 'Ǫ', 'Ŕ',
        'Š', 'Ţ', 'Û', 'Ṽ', 'Ŵ', 'Ẋ', 'Ý', 'Ž',
    ];
    match c {
        'a'..='z' => LOWER[(c as u8 - b'a') as usize],
        'A'..='Z' => UPPER[(c as u8 - b'A') as usize],
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitals_are_accented_and_padded() {
        assert_eq!(transform("HOLD"), "ĤÓĹÐ~~");
        assert_eq!(transform("Connect "), "Çóññéçţ~~~ ");
    }

    #[test]
    fn a_unit_after_a_number_stays() {
        assert_eq!(transform("About 10 MΩ"), "Áƀóúţ 10 MΩ~~~");
        assert_eq!(transform("40Hz~1kHz"), "40Hz~1kHz~~");
        // Not after a number, the letters are a word like any other.
        assert_eq!(transform("V"), "Ṽ~");
    }

    #[test]
    fn text_without_letters_is_left_alone() {
        assert!(matches!(transform(" — "), Cow::Borrowed(" — ")));
    }
}
