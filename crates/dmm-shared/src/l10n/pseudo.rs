//! The padding pseudo-locale: every message accented, padded by about 40%
//! and bracketed, so a layout that can't take longer text shows it, and a
//! string that never went through the localizer stands out unbracketed.
//!
//! Capitals are accented too. They fill the reading area (HOLD, REL, AUTO),
//! and a scheme that only touched lowercase vowels would leave them at
//! their English width.

use std::borrow::Cow;

/// Fluent's per-text-element transform: accents on every ASCII letter, then
/// `~` padding of 40% of the letter count, rounded up.
///
/// Fluent applies it to each run of text in a pattern, never to a
/// placeable's value, so a model name passed in stays readable. Trailing
/// whitespace stays after the padding: "Connect " becomes "Çóññéçţ~~~ ".
pub(super) fn transform(s: &str) -> Cow<'_, str> {
    let letters = s.chars().filter(char::is_ascii_alphabetic).count();
    if letters == 0 {
        return Cow::Borrowed(s);
    }
    let mut out: String = s.chars().map(accent).collect();
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
    fn text_without_letters_is_left_alone() {
        assert!(matches!(transform(" — "), Cow::Borrowed(" — ")));
    }
}
