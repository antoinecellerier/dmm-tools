// Adapted from egui_system_font_provider 0.37, used here under its MIT
// licence:
//
// Copyright (c) 2018-2021 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the
// following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
// NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
// USE OR OTHER DEALINGS IN THE SOFTWARE.

//! System fonts for the characters the bundled fonts lack, the language's
//! own font first.
//!
//! Follows `egui_system_font_provider` 0.37, with one
//! change: a character of a script is looked up in the script's fallback
//! for the language alone, before the generic sans-serif list. The upstream
//! provider asks both in one query, and fontique tries the generic list
//! first, which fontconfig sorts with no language. On a Debian desktop with
//! fonts-nanum, that puts NanumGothic, a Korean font, ahead of Noto Sans CJK
//! JP: Japanese was drawn from it, its kana off-centre with gaps after them,
//! and Chinese got the Korean font's Han characters. Characters shared by
//! scripts, like «、» and «ー», take the fallback of the scripts they are
//! used with: epaint keeps the first font it discovers ahead of any later
//! one, so a «、» drawn first decides the font of the kana after it.

use eframe::egui;
use egui::epaint::text::{
    FallbackRequest, FontData, FontFamily, FontInsert, FontPriority, FontProvider, InsertFontFamily,
};
use fontique::{
    Collection, CollectionOptions, FallbackKey, GenericFamily, Language, QueryFamily, QueryFont,
    QueryStatus, Script, SourceCache, SourceCacheOptions,
};
use std::sync::Mutex;
use std::thread::JoinHandle;
use unicode_script::UnicodeScript as _;

/// Finds an installed font for a character, by the operating system's own
/// font fallback (fontconfig, DirectWrite, CoreText) through fontique.
pub(super) struct SystemFonts {
    fonts: Mutex<Loading>,
    locale: Option<Language>,
}

/// The system's fonts: enumerated on a thread of their own, as that takes
/// a moment, and waited for at the first lookup.
enum Loading {
    Pending(JoinHandle<Fonts>),
    Ready(Box<Fonts>),
    Failed,
}

struct Fonts {
    collection: Collection,
    source_cache: SourceCache,
}

impl SystemFonts {
    /// Start enumerating the system's fonts. `locale` picks between fonts of
    /// one script, such as Japanese and Chinese ones for Han; `None` takes
    /// the system's locale.
    pub(super) fn new(locale: Option<&str>) -> Self {
        let locale = match locale {
            Some(locale) => Some(locale.to_owned()),
            None => sys_locale::get_locale(),
        };
        let loading = std::thread::Builder::new()
            .name("system-fonts".into())
            .spawn(|| Fonts {
                collection: Collection::new(CollectionOptions {
                    shared: false,
                    system_fonts: true,
                }),
                source_cache: SourceCache::new(SourceCacheOptions::default()),
            });
        let loading = match loading {
            Ok(handle) => Loading::Pending(handle),
            Err(e) => {
                log::warn!("UI: can't start enumerating the system fonts: {e}");
                Loading::Failed
            }
        };
        Self {
            fonts: Mutex::new(loading),
            locale: locale.and_then(|l| Language::parse(&l).ok()),
        }
    }

    /// The first system font that draws `c`, and the name epaint keeps it by.
    fn find(&self, c: char, family: &FontFamily) -> Option<(String, QueryFont)> {
        let mut loading = self.fonts.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(*loading, Loading::Pending(_))
            && let Loading::Pending(handle) = std::mem::replace(&mut *loading, Loading::Failed)
        {
            match handle.join() {
                Ok(fonts) => *loading = Loading::Ready(Box::new(fonts)),
                Err(_) => log::warn!("UI: enumerating the system fonts failed"),
            }
        }
        let Loading::Ready(fonts) = &mut *loading else {
            return None;
        };
        let Fonts {
            collection,
            source_cache,
        } = &mut **fonts;
        let generic = match family {
            FontFamily::Monospace => GenericFamily::Monospace,
            FontFamily::Proportional | FontFamily::Name(_) => GenericFamily::SansSerif,
        };
        let by_script = scripts_of(c).into_iter().find_map(|script| {
            let mut locales = vec![self.locale.as_ref()];
            locales.extend(other_locales(script).iter().map(Some));
            locales.into_iter().find_map(|locale| {
                let mut query = collection.query(source_cache);
                query.set_fallbacks(fallback_key(script, locale));
                first_drawing(&mut query, c)
            })
        });
        let font = by_script.or_else(|| {
            // Punctuation, symbols and emoji have no script to fall back on.
            let mut query = collection.query(source_cache);
            query.set_families(
                [
                    generic,
                    GenericFamily::SystemUi,
                    GenericFamily::Emoji,
                    GenericFamily::Math,
                    GenericFamily::Serif,
                    GenericFamily::Monospace,
                ]
                .map(QueryFamily::Generic),
            );
            first_drawing(&mut query, c)
        })?;
        let family_name = collection.family_name(font.family.0).unwrap_or("unknown");
        let name = format!("system:{family_name}:{}:{}", font.family.1, font.index);
        Some((name, font))
    }
}

impl FontProvider for SystemFonts {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let c = request.cluster.chars().next()?;
        let (name, font) = self.find(c, request.family)?;
        log::debug!("UI: font {name:?} for {c:?}");
        let (blob, _) = font.blob.into_raw_parts();
        Some(FontInsert {
            name,
            data: FontData::from_blob(blob, font.index),
            families: vec![InsertFontFamily {
                family: request.family.clone(),
                priority: FontPriority::Lowest,
            }],
        })
    }
}

/// The first font of the query that draws `c`.
fn first_drawing(query: &mut fontique::Query<'_>, c: char) -> Option<QueryFont> {
    let mut found = None;
    query.matches_with(|font| {
        if draws(font, c) {
            found = Some(font.clone());
            QueryStatus::Stop
        } else {
            QueryStatus::Continue
        }
    });
    found
}

const HAN: Script = Script::from_bytes(*b"Hani");

/// The Han locales, Simplified Chinese first, as `zh-SG`: fontique takes
/// `zh-CN` for its default Han locale, whose font is whatever its first
/// locale-less Han lookup returned, often the Japanese one.
fn han_locales() -> &'static [Language] {
    static LOCALES: std::sync::OnceLock<Vec<Language>> = std::sync::OnceLock::new();
    LOCALES.get_or_init(|| {
        ["zh-SG", "zh-TW", "ja", "ko"]
            .iter()
            .filter_map(|l| Language::parse(l).ok())
            .collect()
    })
}

/// The fallback key for a script in a locale, Simplified Chinese as
/// [`han_locales`] spells it.
fn fallback_key(script: Script, locale: Option<&Language>) -> FallbackKey {
    let simplified_chinese = locale.is_some_and(|l| {
        l.language() == "zh"
            && l.script() != Some("Hant")
            && !matches!(l.region(), Some("HK" | "TW" | "MO"))
    });
    let locale = if script == HAN && simplified_chinese {
        han_locales().first()
    } else {
        locale
    };
    FallbackKey::new(script, locale)
}

/// The locales to try when the language's own font lacks a character: Han
/// fonts each cover a different set.
fn other_locales(script: Script) -> &'static [Language] {
    if script == HAN { han_locales() } else { &[] }
}

/// The script of `c`, or for a character several scripts share (`、`, `ー`)
/// the scripts it is used with; none for punctuation and symbols of every
/// script.
fn scripts_of(c: char) -> Vec<Script> {
    let named = |s: unicode_script::Script| match s {
        unicode_script::Script::Common
        | unicode_script::Script::Inherited
        | unicode_script::Script::Unknown => None,
        s => Script::parse(s.short_name()).ok(),
    };
    match named(c.script()) {
        Some(script) => vec![script],
        None => c.script_extension().iter().filter_map(named).collect(),
    }
}

/// Does the font have a glyph for `c` that epaint can draw: an outline, a
/// bitmap or a colour glyph?
fn draws(font: &QueryFont, c: char) -> bool {
    use skrifa::MetadataProvider as _;
    let Some(glyph) = font.charmap().and_then(|charmap| charmap.map(c)) else {
        return false;
    };
    let glyph = skrifa::GlyphId::new(glyph);
    if glyph == skrifa::GlyphId::NOTDEF {
        return false;
    }
    let Ok(font) = skrifa::FontRef::from_index(font.blob.as_ref(), font.index) else {
        return false;
    };
    font.outline_glyphs().get(glyph).is_some()
        || font.color_glyphs().get(glyph).is_some()
        || font
            .bitmap_strikes()
            .glyph_for_size(skrifa::instance::Size::unscaled(), glyph)
            .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shared character takes the scripts it is used with, so «、» and
    /// «ー» find the language's font as the kana around them do.
    #[test]
    fn shared_characters_take_their_scripts() {
        let kana = Script::from_bytes(*b"Kana");
        assert_eq!(scripts_of('ト'), vec![kana]);
        assert!(scripts_of('ー').contains(&kana));
        assert!(scripts_of('、').contains(&HAN));
        assert!(scripts_of('1').is_empty());
        assert!(scripts_of('⌥').is_empty());
    }

    /// The font each test character comes from on this machine, for a look
    /// at which fonts a language gets: run with `--ignored --nocapture`.
    #[test]
    #[ignore = "depends on the fonts installed on this machine"]
    fn which_fonts() {
        for locale in ["ja", "zh-CN", "zh-Hans", "ko", "en-US"] {
            let fonts = SystemFonts::new(Some(locale));
            let names: Vec<String> = ['、', 'ー', 'ト', 'ド', 'の', '取', '説', '帮', '助', '这']
                .iter()
                .map(|&c| {
                    let name = fonts.find(c, &FontFamily::Proportional).map(|(n, _)| n);
                    format!("{c} {}", name.unwrap_or_default())
                })
                .collect();
            println!("{locale}: {}", names.join(" | "));
        }
    }
}
