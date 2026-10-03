//! Linked colours: a palette colour that follows another — its anchor —
//! until the user picks it on its own.
//!
//! The roles a theme adds come in families: the frame, plot, buttons and
//! borders sit on the background, the reading, secondary text and headings
//! are kinds of text. Recolouring the anchor alone left every follower on the
//! old colour, so a theme took a dozen picks to retint. A follower now keeps
//! its designed relation to its anchor — Bubble Gum's frame is a slightly
//! deeper mint than its background, so a lavender background gets a slightly
//! deeper lavender frame — and keeps the contrast it needs, until the user
//! sets it themselves.
//!
//! The relation is an OKLCH offset — a lightness step, a chroma ratio and a
//! hue turn — rather than an RGB or Lab difference, which would carry a
//! tinted frame's hue onto a grey background. Where the moved colour would
//! fall under its contrast floor, only its lightness moves, just far enough.
//! Where a follower is its anchor — the presets' frame and reading — it
//! simply copies it. A colour the palette leaves to egui, a preset's border,
//! stays egui's: its hover outlines go with it.

use std::cell::RefCell;

use eframe::egui::Color32;

use crate::settings::{ColorPreset, HexColor, PaletteOverrides};
use crate::theme::{PaletteField, ThemeColors, contrast, layered};

/// The colour a field follows, if it follows one.
///
/// The sub-value overlays sit at set hue angles around the data line, and a
/// hue turn keeps that spacing, so they follow it. Accent, the status
/// colours and the other graph colours follow nothing: their hues carry
/// meaning — a gap's red is data lost — and turning them with the line could
/// land one on another's. [`too_close`] is the check on what does collide.
pub(crate) fn anchor(field: PaletteField) -> Option<PaletteField> {
    use PaletteField::*;
    match field {
        Frame | PlotBackground | Button | Border => Some(Background),
        Reading | WeakText | Heading | GraphCrosshair => Some(Text),
        GraphEnvelope | GraphOverlay1 | GraphOverlay2 | GraphOverlay3 | MinimapViewport => {
            Some(GraphLine)
        }
        _ => None,
    }
}

/// The fields that follow `anchor`, in panel order.
pub(crate) fn followers(of: PaletteField) -> impl Iterator<Item = PaletteField> {
    PaletteField::ALL
        .iter()
        .copied()
        .filter(move |&f| anchor(f) == Some(of))
}

/// The contrast a field needs, as `(grounds, floor)`: text 4.5:1, borders and
/// graph lines 3:1. A ground (frame, plot, button) is instead held to keep
/// the text readable on it — see [`floor_failure`].
fn floor(field: PaletteField) -> Option<(&'static [PaletteField], f64)> {
    use PaletteField::*;
    Some(match field {
        Text | Reading | WeakText | Heading => (&[Background, Frame], 4.5),
        Accent => (&[Background, Frame], 4.5),
        Border => (&[Background, Frame, PlotBackground], 3.0),
        GraphCrosshair | GraphEnvelope | GraphOverlay1 | GraphOverlay2 | GraphOverlay3
        | MinimapViewport => (&[PlotBackground], 3.0),
        Frame | Button | PlotBackground => (&[Text], 4.5),
        _ => return None,
    })
}

/// Whether `field` is a surface other colours are drawn on.
fn is_surface(field: PaletteField) -> bool {
    matches!(
        field,
        PaletteField::Frame | PaletteField::Button | PaletteField::PlotBackground
    )
}

/// The worst ratio `field` has against the grounds it needs a floor on,
/// with that ground, when it is under the floor.
pub(crate) fn floor_failure(
    tc: &ThemeColors,
    field: PaletteField,
) -> Option<(f64, PaletteField, f64)> {
    let (grounds, min) = floor(field)?;
    let colour = tc.effective_color(field);
    grounds
        .iter()
        .map(|&g| (contrast(colour, tc.effective_color(g)), g))
        .filter(|&(ratio, _)| ratio < min)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(ratio, g)| (ratio, g, min))
}

/// The palette with the user's `tweaks` laid over `base`, every follower of
/// a recoloured anchor moved with it. A follower the user set themselves
/// stays where they put it; one whose anchor they left alone stays on the
/// base. `base` is the preset's or theme's own palette, overrides included.
///
/// Every widget asks for the palette, many times a frame, and a follower
/// held to its floor is searched for; the last answer is kept, keyed by
/// everything it depends on.
pub(crate) fn apply(dark: bool, base: &ThemeColors, tweaks: &PaletteOverrides) -> PaletteOverrides {
    type Key = (bool, ColorPreset, PaletteOverrides, PaletteOverrides);
    thread_local! {
        static LAST: RefCell<Option<(Key, PaletteOverrides)>> = const { RefCell::new(None) };
    }
    let key = (
        dark,
        base.preset(),
        base.overrides().clone(),
        tweaks.clone(),
    );
    if let Some(hit) = LAST.with_borrow(|last| {
        last.as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    }) {
        return hit;
    }
    let out = resolve(dark, base, tweaks);
    LAST.set(Some((key, out.clone())));
    out
}

fn resolve(dark: bool, base: &ThemeColors, tweaks: &PaletteOverrides) -> PaletteOverrides {
    let mut out = layered(base.overrides(), tweaks);
    let mut tweaks = tweaks.clone();
    // Anchors first, then their followers in an order where every ground a
    // follower is checked against has already settled: the grounds, the
    // border on them, then the text-like colours on top.
    let order = [
        PaletteField::Frame,
        PaletteField::PlotBackground,
        PaletteField::Button,
        PaletteField::Border,
        PaletteField::Reading,
        PaletteField::WeakText,
        PaletteField::Heading,
        PaletteField::GraphCrosshair,
        PaletteField::GraphEnvelope,
        PaletteField::GraphOverlay1,
        PaletteField::GraphOverlay2,
        PaletteField::GraphOverlay3,
        PaletteField::MinimapViewport,
    ];
    for field in order {
        let Some(anchor) = anchor(field) else {
            continue;
        };
        if field.override_slot(&mut tweaks).is_some() {
            continue; // set on its own
        }
        // Following would pin it, and with it the hover and press outlines
        // egui draws while the border is its own.
        if field == PaletteField::Border && !base.border_pinned() {
            continue;
        }
        if anchor.override_slot(&mut tweaks).is_some() {
            let new_anchor = ThemeColors::new(dark, base.preset(), &out).effective_color(anchor);
            let moved = follow(
                base.effective_color(field),
                base.effective_color(anchor),
                new_anchor,
            );
            *field.override_slot(&mut out) = Some(HexColor(moved));
        }
        // A colour drawn on a surface is held to its floor when its anchor
        // or its ground moved it under — never where the preset or theme
        // itself sits under on purpose, as the presets' faint decorative
        // borders do. A surface is never moved for what is drawn on it: it
        // follows its anchor and nothing else, as the editor shows, and text
        // that no longer reads on it warns on the text.
        let tc = ThemeColors::new(dark, base.preset(), &out);
        if !is_surface(field)
            && floor_failure(&tc, field).is_some()
            && floor_failure(base, field).is_none()
        {
            let kept = keep_floor(&tc, field, tc.effective_color(field));
            *field.override_slot(&mut out) = Some(HexColor(kept));
        }
    }
    out
}

/// The graph colours drawn together on one plot, which have to read apart.
pub(crate) const GRAPH_SERIES: &[PaletteField] = &[
    PaletteField::GraphLine,
    PaletteField::GraphOverlay1,
    PaletteField::GraphOverlay2,
    PaletteField::GraphOverlay3,
    PaletteField::GraphMean,
    PaletteField::GraphRef,
    PaletteField::GraphCursor,
    PaletteField::GraphCrossing,
    PaletteField::GraphMarker,
    PaletteField::GraphGap,
];

/// How far apart two graph colours have to be, in CIE76 ΔE. Line style tells
/// some apart without colour, but a palette whose colours fall into one
/// family is a hard read — Desert's terracotta overlay, orange cursors and
/// brown reference were ΔE 21–27 apart. 30 between any two; 35 between the
/// line and an overlay, the lines most often crossing.
fn distinct_floor(a: PaletteField, b: PaletteField) -> f64 {
    let overlay = |f: PaletteField| {
        matches!(
            f,
            PaletteField::GraphOverlay1 | PaletteField::GraphOverlay2 | PaletteField::GraphOverlay3
        )
    };
    let line = PaletteField::GraphLine;
    if (a == line && overlay(b)) || (b == line && overlay(a)) {
        35.0
    } else {
        30.0
    }
}

/// The graph colour nearest `field`, with their distance and the floor,
/// when they are under it. `None` for a field that is not a graph series.
pub(crate) fn too_close(tc: &ThemeColors, field: PaletteField) -> Option<(PaletteField, f64, f64)> {
    if !GRAPH_SERIES.contains(&field) {
        return None;
    }
    let colour = tc.effective_color(field);
    GRAPH_SERIES
        .iter()
        .filter(|&&other| other != field)
        .map(|&other| {
            let d = delta_e(colour, tc.effective_color(other));
            (other, d, distinct_floor(field, other))
        })
        .filter(|&(_, d, floor)| d < floor)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// CIE76 colour difference: the distance between two colours in L*a*b*.
fn delta_e(a: Color32, b: Color32) -> f64 {
    let (a, b) = (lab(a), lab(b));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// CIE L*a*b* of an sRGB colour, D65 white.
fn lab(c: Color32) -> [f64; 3] {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let (r, g, b) = (to_linear(r), to_linear(g), to_linear(b));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f64| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// `follower`'s relation to `anchor`, applied to `new_anchor`.
fn follow(follower: Color32, anchor: Color32, new_anchor: Color32) -> Color32 {
    if follower == anchor {
        // Exactly, not through OKLCH and back, which can land a step off.
        return new_anchor;
    }
    let [fr, fg, fb, fa] = follower.to_srgba_unmultiplied();
    let f = oklch(fr, fg, fb);
    let a = oklch_of(anchor);
    let n = oklch_of(new_anchor);
    // Chroma as a ratio, so a pastel follower of a pastel anchor stays
    // pastel; a grey anchor has no hue to turn by and no chroma to scale.
    let (chroma, hue) = if a[1] > 0.02 {
        (n[1] * f[1] / a[1], n[2] + (f[2] - a[2]))
    } else if f[1] <= 0.02 {
        // A grey follower of a grey anchor is a lighter or darker shade of
        // it, and takes the new anchor's tint.
        (n[1], n[2])
    } else {
        (f[1], f[2])
    };
    let [r, g, b] = from_oklch(n[0] + (f[0] - a[0]), chroma, hue);
    Color32::from_rgba_unmultiplied(r, g, b, fa)
}

/// `colour`, its lightness moved the least distance that clears its floor
/// in `tc`, hue and chroma kept. Unchanged if no lightness does.
fn keep_floor(tc: &ThemeColors, field: PaletteField, colour: Color32) -> Color32 {
    let [r, g, b, a] = colour.to_srgba_unmultiplied();
    let [l, c, h] = oklch(r, g, b);
    let mut overrides = tc.overrides().clone();
    for step in 1..=200 {
        for sign in [-1.0, 1.0] {
            let [r, g, b] = from_oklch(l + sign * step as f64 * 0.005, c, h);
            let candidate = Color32::from_rgba_unmultiplied(r, g, b, a);
            *field.override_slot(&mut overrides) = Some(HexColor(candidate));
            let moved = ThemeColors::new(tc.is_dark(), tc.preset(), &overrides);
            if floor_failure(&moved, field).is_none() {
                return candidate;
            }
        }
    }
    colour
}

fn oklch_of(c: Color32) -> [f64; 3] {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    oklch(r, g, b)
}

fn to_linear(v: u8) -> f64 {
    let v = f64::from(v) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(v: f64) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let v = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

/// OKLCH (lightness, chroma, hue in radians) of an sRGB colour.
fn oklch(r: u8, g: u8, b: u8) -> [f64; 3] {
    let (r, g, b) = (to_linear(r), to_linear(g), to_linear(b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    let lightness = 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s;
    let a = 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s;
    let bb = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s;
    [lightness, a.hypot(bb), bb.atan2(a)]
}

/// Linear sRGB of an OKLCH colour, possibly out of gamut.
fn linear_rgb(l: f64, c: f64, h: f64) -> [f64; 3] {
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_612_701 * s_,
    ]
}

/// The sRGB colour nearest an OKLCH one: chroma is reduced until it fits,
/// so an out-of-gamut colour keeps its lightness and hue.
fn from_oklch(l: f64, c: f64, h: f64) -> [u8; 3] {
    let l = l.clamp(0.0, 1.0);
    let fits = |c: f64| {
        linear_rgb(l, c, h)
            .iter()
            .all(|v| (-1e-4..=1.0 + 1e-4).contains(v))
    };
    let mut c = c.max(0.0);
    if !fits(c) {
        let (mut lo, mut hi) = (0.0, c);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        c = lo;
    }
    linear_rgb(l, c, h).map(to_srgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(rgb: u32) -> Option<HexColor> {
        let [_, r, g, b] = rgb.to_be_bytes();
        Some(HexColor(Color32::from_rgb(r, g, b)))
    }

    #[test]
    fn oklch_round_trips_srgb() {
        for (r, g, b) in [
            (0, 0, 0),
            (255, 255, 255),
            (185, 249, 194),
            (184, 0, 150),
            (10, 144, 240),
        ] {
            let [l, c, h] = oklch(r, g, b);
            assert_eq!(from_oklch(l, c, h), [r, g, b]);
        }
    }

    /// In a preset a follower is its anchor, so recolouring the background
    /// recolours the frame to match, and the text the reading.
    #[test]
    fn in_a_preset_a_follower_copies_its_anchor() {
        let base = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
        let tweaks = PaletteOverrides {
            background: hex(0xE6DAFF),
            text: hex(0x2A1A5E),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
        assert_eq!(tc.frame(), tc.background());
        assert_eq!(tc.reading(), tc.text());
    }

    /// A theme's frame a step deeper than its background stays a step
    /// deeper on a new background, in the new hue.
    #[test]
    fn a_follower_keeps_its_designed_offset() {
        let theme = PaletteOverrides {
            background: hex(0xB9F9C2),
            frame: hex(0x96F6A3),
            ..Default::default()
        };
        let base = ThemeColors::new(false, ColorPreset::Default, &theme);
        let tweaks = PaletteOverrides {
            background: hex(0xE6DAFF),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
        let (bg, frame) = (oklch_of(tc.background()), oklch_of(tc.frame()));
        let (bg0, frame0) = (oklch_of(base.background()), oklch_of(base.frame()));
        assert!(
            (frame[0] - bg[0] - (frame0[0] - bg0[0])).abs() < 0.01,
            "lightness step kept"
        );
        assert!((frame[2] - bg[2]).abs() < 0.2, "the frame took the new hue");
    }

    /// A follower the user set stays put, and one whose anchor they left
    /// alone stays on the base.
    #[test]
    fn a_set_follower_and_an_untouched_family_stay_put() {
        let base = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
        let tweaks = PaletteOverrides {
            background: hex(0xE6DAFF),
            frame: hex(0xFFD9EC),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
        assert_eq!(tc.frame(), Color32::from_rgb(0xFF, 0xD9, 0xEC));
        assert_eq!(tc.graph_envelope(), base.graph_envelope());
        // The weak text's anchor stayed, but its ground moved under it: it
        // keeps its hue and only darkens as far as the new frame needs.
        assert_eq!(floor_failure(&tc, PaletteField::WeakText), None);
        let (was, now) = (oklch_of(base.weak_text()), oklch_of(tc.weak_text()));
        assert!(now[0] < was[0] && now[1] < 0.02, "{:?}", tc.weak_text());
    }

    /// Moving an anchor never leaves a follower under its floor when some
    /// lightness clears it: a mid-tone background drags the weak text with
    /// it only as far as it can still be read.
    #[test]
    fn a_moved_follower_keeps_its_floor() {
        let base = ThemeColors::new(false, ColorPreset::Default, &PaletteOverrides::default());
        for bg in [0xA9B8FF, 0x7F7F7F, 0xF0F0F0, 0x202020] {
            let tweaks = PaletteOverrides {
                background: hex(bg),
                ..Default::default()
            };
            let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
            for field in [
                PaletteField::WeakText,
                PaletteField::Heading,
                PaletteField::Reading,
            ] {
                if floor_failure(&tc, PaletteField::Text).is_none() {
                    assert_eq!(floor_failure(&tc, field), None, "{bg:06X} {field:?}");
                }
            }
        }
    }

    /// A translucent follower — Default's dark envelope — keeps its alpha.
    #[test]
    fn a_translucent_follower_keeps_its_alpha() {
        let base = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        let tweaks = PaletteOverrides {
            graph_line: hex(0x40C0FF),
            ..Default::default()
        };
        let tc = ThemeColors::new(true, ColorPreset::Default, &apply(true, &base, &tweaks));
        assert_eq!(tc.graph_envelope().a(), base.graph_envelope().a());
    }

    #[test]
    fn every_follower_has_an_anchor_that_follows_nothing() {
        for &field in PaletteField::ALL {
            if let Some(a) = anchor(field) {
                assert_eq!(
                    anchor(a),
                    None,
                    "{field:?} follows {a:?}, which follows another"
                );
            }
        }
    }

    /// Turning the data line's hue turns the overlays by as much, so they
    /// stay as far from each other, and from the line, as the theme set them.
    #[test]
    fn the_overlays_turn_with_the_data_line() {
        let theme = crate::theme::named::find("Bubble Gum", &[]).unwrap();
        let base = ThemeColors::new(false, ColorPreset::Default, &theme.colors);
        let tweaks = PaletteOverrides {
            graph_line: hex(0x2E8B22),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
        let turn = |c: &ThemeColors, k| {
            let (o, l) = (oklch_of(c.graph_overlay(k)), oklch_of(c.graph_line()));
            (o[2] - l[2]).rem_euclid(std::f64::consts::TAU)
        };
        let overlays = [
            PaletteField::GraphOverlay1,
            PaletteField::GraphOverlay2,
            PaletteField::GraphOverlay3,
        ];
        for (k, field) in overlays.into_iter().enumerate() {
            assert!((turn(&tc, k) - turn(&base, k)).abs() < 0.1, "overlay {k}");
            assert_eq!(floor_failure(&tc, field), None, "overlay {k}");
        }
    }

    /// A surface follows its anchor and nothing else: recolouring the text,
    /// even to one that no longer reads on the frame, leaves the frame where
    /// the background put it, and the text carries the warning.
    #[test]
    fn a_surface_follows_only_its_anchor() {
        let theme = crate::theme::named::find("Bubble Gum", &[]).unwrap();
        let base = ThemeColors::new(false, ColorPreset::Default, &theme.colors);
        let tweaks = PaletteOverrides {
            text: hex(0x7FE08F),
            ..Default::default()
        };
        let tc = ThemeColors::new(false, ColorPreset::Default, &apply(false, &base, &tweaks));
        assert_eq!(tc.frame(), base.frame());
        assert_eq!(tc.button(), base.button());
        assert_eq!(tc.plot_background(), base.plot_background());
        assert!(floor_failure(&tc, PaletteField::Text).is_some());
    }

    /// Recolouring a preset's background leaves its border to egui, as the
    /// preset does, so egui's hover and press outlines stay.
    #[test]
    fn a_presets_unpinned_border_stays_egui_s() {
        let base = ThemeColors::new(true, ColorPreset::Default, &PaletteOverrides::default());
        let tweaks = PaletteOverrides {
            background: hex(0x202838),
            ..Default::default()
        };
        let out = apply(true, &base, &tweaks);
        assert_eq!(out.border, None);
        assert!(!ThemeColors::new(true, ColorPreset::Default, &out).border_pinned());
    }
}
