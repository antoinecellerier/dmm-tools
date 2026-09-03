use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, Grid, Rect, RichText, TextFormat, Ui};
use std::borrow::Cow;

use crate::a11y::UiA11yExt;
use crate::theme::ThemeColors;

/// Base font size for the primary reading in the wide (side panel) layout.
pub(crate) const BASE_READING_FONT_SIZE: f32 = 36.0;

/// Minimum font size for the big meter scaled reading. Smaller than
/// `BASE_READING_FONT_SIZE` so the window can shrink to a tiny widget.
pub(crate) const MIN_BIG_METER_FONT_SIZE: f32 = 12.0;

/// Font size for the primary reading in the compact (narrow) layout.
const COMPACT_READING_FONT_SIZE: f32 = 28.0;

/// Floor for the sub-value rows. They derive their size from the caller's
/// reading size, which shrinks to `MIN_BIG_METER_FONT_SIZE` in a tiny big-meter
/// window; without this floor the derived size would fall under the 11 pt
/// minimum `.claude/rules/gui.md` sets.
const MIN_AUX_FONT_SIZE: f32 = 11.0;

/// Format the meter's raw 7-char display string for stable rendering.
///
/// Right-aligns to the meter's own 7-character display width, so the reading
/// keeps a constant width as digits and the minus sign come and go — the
/// jitter `.claude/rules/gui.md` is guarding against with "display value
/// strings use `display_raw` for stable width".
///
/// Ordinary spaces suffice: every caller draws this with
/// `FontId::monospace`, where a space is already digit-width. (This comment
/// previously claimed a figure-space (U+2007) substitution, which the body
/// has never done and which would only matter in a proportional font.)
fn format_display_raw(raw: &str) -> String {
    let trimmed = raw.trim_end();
    format!("{trimmed:>7}")
}

/// Format the measurement value as a display string.
/// Uses the meter's raw 7-char display when available (UT61E+ protocol),
/// otherwise formats the numeric value for float-based protocols.
///
/// The parsed `MeasuredValue` decides first: several protocols flag overload
/// through a status bit while still sending ordinary digits in the display
/// field (UT8802 `sign_byte & 0x40`, UT8803 `payload[12] & 0x04`). Preferring
/// `display_raw` there would render an out-of-range reading as a plausible
/// number. `display_raw` still wins for normal readings, which is what keeps
/// the on-screen width steady.
fn format_value_display(m: &Measurement) -> String {
    match &m.value {
        MeasuredValue::Normal(v) => match m.display_raw.as_deref() {
            Some(raw) => format_display_raw(raw),
            None => format!("{v:>7}"),
        },
        MeasuredValue::Overload => format!("{:>7}", "OL"),
        MeasuredValue::NcvLevel(l) => format!("NCV {l}"),
    }
}

/// Format a measurement as a spoken-friendly one-line description for screen
/// readers. Used as the live-region label on the primary reading. Uses the
/// same value formatting as the visible display so AT users hear exactly
/// what sighted users see.
fn live_region_label(measurement: Option<&Measurement>, scaled: bool) -> String {
    match measurement {
        Some(m) => {
            let value = match &m.value {
                MeasuredValue::Overload => "overload".to_string(),
                MeasuredValue::NcvLevel(l) => format!("NCV level {l}"),
                MeasuredValue::Normal(_) => format_value_display(m).trim().to_string(),
            };
            let mut parts = String::with_capacity(96);
            parts.push_str(&value);
            if !m.unit.is_empty() {
                parts.push(' ');
                parts.push_str(&spoken_unit(&m.unit));
            }
            if !m.mode.is_empty() {
                parts.push_str(", ");
                parts.push_str(&m.mode);
            }
            // Sub-values sit between the mode and the flags, matching the
            // visible order: the rows are drawn under the reading and above
            // the mode/flags line. Without them a UT181A user in MIN/MAX
            // hears only the live value and never the extremes the meter is
            // actually displaying.
            for aux in &m.aux_values {
                parts.push_str(", ");
                parts.push_str(&aux.label);
                parts.push(' ');
                parts.push_str(&spoken_aux_value(aux));
                let unit = spoken_unit(aux.unit_or(&m.unit));
                if !unit.is_empty() {
                    parts.push(' ');
                    parts.push_str(&unit);
                }
                // The visible row ends in "@12s"; spelling it out is the only
                // way an AT user learns *when* a MIN/MAX extreme was caught,
                // which is half of what those readings mean.
                if let Some(secs) = aux.elapsed_secs {
                    parts.push_str(" at ");
                    parts.push_str(&secs.to_string());
                    parts.push_str(if secs == 1 { " second" } else { " seconds" });
                }
            }
            // Speak the same status flags that the visible badge row shows.
            // Without this, a screen reader user toggling HOLD/REL/MIN/MAX/
            // AUTO via the on-device buttons hears the value change but no
            // confirmation that the mode actually flipped.
            append_flags_phrase(&mut parts, &m.flags);
            // Last, after the flags, so it reads as one more badge — which is
            // exactly where the SCALE badge sits on screen. Without it a
            // screen-reader user has no way to tell a software-scaled reading
            // from one the meter produced.
            if scaled {
                parts.push_str(", software scaled");
            }
            parts
        }
        None => "No reading".to_string(),
    }
}

/// Spoken form of a sub-value: the parsed value decides, so an overloaded
/// sub-value is announced as "overload" rather than the letters "O L".
fn spoken_aux_value(aux: &AuxValue) -> Cow<'_, str> {
    match &aux.value {
        MeasuredValue::Overload => Cow::Borrowed("overload"),
        _ => aux.value_str(),
    }
}

/// Spoken form of a unit string.
///
/// Only the degree symbol is rewritten: screen readers differ on whether they
/// read "°" at all, so a temperature sub-value could otherwise be announced as
/// a bare "24.1 C". The substitution is confined to the spoken label — the
/// visible rows keep the symbol.
fn spoken_unit(unit: &str) -> Cow<'_, str> {
    match unit {
        "\u{00B0}C" => Cow::Borrowed("degrees C"),
        "\u{00B0}F" => Cow::Borrowed("degrees F"),
        other => Cow::Borrowed(other),
    }
}

/// The single order the flags are presented in: `show_flags` paints the
/// badges in it and `append_flags_phrase` speaks them in it, so the two cannot
/// drift apart the way three hand-written lists did.
///
/// `Flag::HvWarning` leads — see the hazard-first comment in `show_flags`.
/// Everything else keeps [`Flag::ALL`] order, which is what the recording
/// panel and the CSV flags column already use.
fn badge_order() -> impl Iterator<Item = Flag> {
    std::iter::once(Flag::HvWarning).chain(Flag::ALL.into_iter().filter(|f| *f != Flag::HvWarning))
}

/// Spoken form of a flag for the screen-reader label.
///
/// `None` where [`Flag::label`] is `None`: the DC/AC distinction rides on the
/// mode field, so announcing it again would be noise.
fn spoken(flag: Flag) -> Option<&'static str> {
    Some(match flag {
        Flag::HvWarning => "high voltage warning",
        Flag::AutoRange => "auto range",
        Flag::Hold => "hold",
        Flag::Rel => "relative",
        Flag::Min => "minimum",
        Flag::Max => "maximum",
        Flag::PeakMin => "peak minimum",
        Flag::PeakMax => "peak maximum",
        Flag::LowBattery => "low battery",
        Flag::LeadError => "lead error",
        Flag::Comp => "compare",
        Flag::Record => "recording",
        Flag::LoZ => "low impedance",
        Flag::Void => "void",
        Flag::Dc => return None,
    })
}

/// Badge color for a flag: hazard in the error color, the conditions that
/// cast doubt on the reading in the warning color, the rest in accent.
fn badge_tone(flag: Flag, tc: &ThemeColors) -> Color32 {
    match flag {
        Flag::HvWarning => tc.status_error(),
        Flag::LowBattery | Flag::LeadError | Flag::Void => tc.recording_full_warning(),
        Flag::Hold
        | Flag::Rel
        | Flag::AutoRange
        | Flag::Min
        | Flag::Max
        | Flag::PeakMax
        | Flag::PeakMin
        | Flag::Comp
        | Flag::Record
        | Flag::LoZ
        | Flag::Dc => tc.accent(),
    }
}

/// Append a phrase listing the active status flags, in `badge_order()` — the
/// same iterator `show_flags` paints from, so what is heard and what is seen
/// are the same flags in the same order. Each flag is prefixed with ", " so it
/// reads naturally after the mode field. No-op if all flags are inactive.
fn append_flags_phrase(out: &mut String, flags: &StatusFlags) {
    for phrase in badge_order().filter(|f| flags.get(*f)).filter_map(spoken) {
        out.push_str(", ");
        out.push_str(phrase);
    }
}

/// Pack a `StatusFlags` into a u16 bitfield for fingerprint hashing. Bit `i`
/// is `Flag::ALL[i]`, so the packing stays stable and stays complete as flags
/// are added rather than relying on struct field order.
fn flags_bits(flags: &StatusFlags) -> u16 {
    Flag::ALL.iter().enumerate().fold(0u16, |bits, (i, &flag)| {
        bits | ((flags.get(flag) as u16) << i)
    })
}

/// Build a u64 fingerprint that changes whenever `live_region_label` would
/// produce different output. Lets `set_live_region_cached` skip per-frame
/// `format!`/`String` allocation when the measurement is unchanged.
fn live_region_fingerprint(measurement: Option<&Measurement>, scaled: bool) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // Toggling the transform changes the spoken label without necessarily
    // changing the reading, so the bit has to be in the fingerprint or the
    // cached announcement would never be rebuilt.
    scaled.hash(&mut h);
    match measurement {
        None => 0u8.hash(&mut h),
        Some(m) => {
            1u8.hash(&mut h);
            match &m.value {
                MeasuredValue::Normal(v) => {
                    0u8.hash(&mut h);
                    v.to_bits().hash(&mut h);
                    // display_raw is what we actually format for Normal values,
                    // so include it so the fingerprint catches stable-string
                    // changes that don't show up in the f64 bits.
                    m.display_raw.as_deref().unwrap_or("").hash(&mut h);
                }
                MeasuredValue::Overload => 1u8.hash(&mut h),
                MeasuredValue::NcvLevel(l) => {
                    2u8.hash(&mut h);
                    l.hash(&mut h);
                }
            }
            m.unit.hash(&mut h);
            m.mode.hash(&mut h);
            // Sub-values are part of both the spoken label and the visible
            // rows, so a MIN/MAX extreme moving (or its timestamp advancing)
            // has to invalidate the cached announcement even though the live
            // value may be unchanged.
            m.aux_values.len().hash(&mut h);
            for aux in &m.aux_values {
                aux.label.hash(&mut h);
                match &aux.value {
                    MeasuredValue::Normal(v) => {
                        0u8.hash(&mut h);
                        v.to_bits().hash(&mut h);
                        aux.display_raw.as_deref().unwrap_or("").hash(&mut h);
                    }
                    MeasuredValue::Overload => 1u8.hash(&mut h),
                    MeasuredValue::NcvLevel(l) => {
                        2u8.hash(&mut h);
                        l.hash(&mut h);
                    }
                }
                aux.unit.hash(&mut h);
                aux.elapsed_secs.hash(&mut h);
            }
            flags_bits(&m.flags).hash(&mut h);
        }
    }
    h.finish()
}

/// Right-align a sub-value to the primary reading's 7-character width.
///
/// Same reasoning as [`format_display_raw`]: every caller draws this with
/// `FontId::monospace`, so a fixed width keeps the digits from shifting
/// sideways between frames and lines the sub-value rows up with each other.
fn format_aux_value(aux: &AuxValue) -> String {
    format!("{:>7}", aux.value_str())
}

/// Screen rects of one sub-value row: (label, value+unit).
///
/// Returned by [`show_aux_rows`] so a test can assert the two are on the same
/// baseline. Production callers drop it — the rows are laid out by the grid,
/// not by their caller.
type AuxRowRects = (Rect, Rect);

/// Render one row per sub-value beneath the primary reading.
///
/// Draws nothing at all when the measurement has none, so single-display
/// meters keep the layout they had before sub-values existed.
///
/// `font_size` is the caller's mode-line size, floored at
/// [`MIN_AUX_FONT_SIZE`]: the rows are secondary information and should not
/// compete with the main value, but they still have to stay readable.
///
/// Returns one [`AuxRowRects`] per row, which production callers drop — it
/// exists so a test can assert the label and its value stay on one line.
fn show_aux_rows(
    ui: &mut Ui,
    m: &Measurement,
    font_size: f32,
    tc: &ThemeColors,
) -> Vec<AuxRowRects> {
    if m.aux_values.is_empty() {
        return Vec::new();
    }
    let size = font_size.max(MIN_AUX_FONT_SIZE);
    let mut rects: Vec<AuxRowRects> = Vec::with_capacity(m.aux_values.len());
    // A grid rather than a stack of horizontal rows so labels, digits and
    // timestamps line up in columns however long the individual strings are.
    Grid::new(ui.id().with("aux_rows"))
        .num_columns(3)
        .spacing([(size * 0.5).max(4.0), 2.0])
        .show(ui, |ui| {
            for aux in &m.aux_values {
                let label = ui.label(
                    RichText::new(&*aux.label)
                        .font(FontId::proportional(size))
                        .color(ui.visuals().weak_text_color()),
                );
                // Overload in the error color, as the main value is — and
                // the text still reads "OL", so the state is never signalled
                // by color alone.
                let value_color = match aux.value {
                    MeasuredValue::Overload => tc.status_error(),
                    _ => ui.visuals().text_color(),
                };
                // Value and unit are one label, not a nested `ui.horizontal`.
                // A horizontal scope allocates its child `Ui` at
                // `interact_size.y` (~18 px) and then expands downwards, so
                // taller content lands half a line below the grid row it
                // belongs to: 12 px out at the side panel's 36 px, 66 px out
                // at the big meter's 130 px. With every cell a plain label,
                // the grid's own `LEFT_CENTER` alignment does the work.
                let mut job = LayoutJob::default();
                job.append(
                    &format_aux_value(aux),
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(size),
                        color: value_color,
                        ..Default::default()
                    },
                );
                let unit = aux.unit_or(&m.unit);
                if !unit.is_empty() {
                    job.append(
                        unit,
                        2.0,
                        TextFormat {
                            font_id: FontId::monospace(size),
                            color: ui.visuals().text_color(),
                            ..Default::default()
                        },
                    );
                }
                let value = ui.label(job);
                rects.push((label.rect, value.rect));
                if let Some(secs) = aux.elapsed_secs {
                    ui.label(
                        RichText::new(format!("@{secs}s"))
                            .font(FontId::proportional(size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                ui.end_row();
            }
        });
    rects
}

/// Prepare the value text and color from a measurement.
fn value_display(ui: &Ui, m: &Measurement, tc: &ThemeColors) -> (String, Color32) {
    match &m.value {
        MeasuredValue::Normal(_) => (format_value_display(m), ui.visuals().text_color()),
        MeasuredValue::Overload => (format_value_display(m), tc.status_error()),
        MeasuredValue::NcvLevel(_) => (format_value_display(m), ui.visuals().text_color()),
    }
}

/// Render the primary reading display at the given font size (two-line layout).
fn show_reading_sized(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
) {
    let unit_size = value_size;
    let mode_size = value_size * 0.4;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(ui, m, tc);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m), scaled),
                || live_region_label(Some(m), scaled),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(value_size))
                            .color(value_color),
                    );
                    ui.label(
                        RichText::new(&*m.unit)
                            .font(FontId::monospace(unit_size))
                            .color(ui.visuals().text_color()),
                    );
                },
            );

            let _ = show_aux_rows(ui, m, mode_size, tc);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.5).max(2.0);
                ui.label(
                    RichText::new(&*m.mode)
                        .font(FontId::proportional(mode_size))
                        .color(ui.visuals().weak_text_color()),
                );
                if !m.range_label.is_empty() {
                    ui.label(
                        RichText::new(&*m.range_label)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                show_flags(ui, m, mode_size, tc, scaled);
            });
        }
        None => {
            // Wrap the placeholder + caption in a horizontal scope so the
            // live-region label is attached to the scope id rather than to
            // the inner ui.label() Response. egui maps Role::Label
            // overrides to set_value, not set_label, so attaching directly
            // to the label would silently drop the live-region label.
            ui.live_region_horizontal(
                live_region_fingerprint(None, scaled),
                || live_region_label(None, scaled),
                |ui| {
                    ui.label(
                        RichText::new(crate::NO_DATA)
                            .font(FontId::monospace(value_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.label(RichText::new("No reading").color(ui.visuals().weak_text_color()));
                },
            );
        }
    }
}

/// Render the reading with value and mode on a single line (inline layout).
fn show_reading_inline(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
) {
    let unit_size = value_size;
    let mode_size = value_size * 0.4;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(ui, m, tc);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m), scaled),
                || live_region_label(Some(m), scaled),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(value_size))
                            .color(value_color),
                    );
                    ui.label(
                        RichText::new(&*m.unit)
                            .font(FontId::monospace(unit_size))
                            .color(ui.visuals().text_color()),
                    );
                    ui.separator();
                    ui.spacing_mut().item_spacing.x = (mode_size * 0.3).max(2.0);
                    ui.label(
                        RichText::new(&*m.mode)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                    show_flags(ui, m, mode_size, tc, scaled);
                },
            );

            // Sub-values still get their own rows in the inline layout: the
            // single line is already the widest thing on screen, and folding
            // four UT181A sub-values into it would force the value font down.
            let _ = show_aux_rows(ui, m, mode_size, tc);
        }
        None => {
            // See `show_reading_sized` for why the placeholder is wrapped
            // in a horizontal scope: egui Role::Label silently swallows
            // accesskit set_label overrides.
            ui.live_region_horizontal(
                live_region_fingerprint(None, scaled),
                || live_region_label(None, scaled),
                |ui| {
                    ui.label(
                        RichText::new(format!("{} No reading", crate::NO_DATA))
                            .font(FontId::monospace(value_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                },
            );
        }
    }
}

/// Render the large primary reading display.
///
/// `scaled` marks the reading as passed through a software transform, so the
/// SCALE badge and the spoken label say so.
pub fn show_reading(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
) {
    show_reading_sized(ui, measurement, BASE_READING_FONT_SIZE, tc, scaled);
}

/// Cached ratios of rendered reading dimensions to font size.
/// Used by `show_reading_large` to compute the optimal font size and
/// updated by the caller only on window resize (to avoid oscillation).
#[derive(Clone)]
pub struct ReadingRatios {
    /// Two-line layout: reading width / font_size.
    pub w: f32,
    /// Two-line layout: reading height / font_size.
    pub h: f32,
    /// Inline layout: reading width / font_size.
    pub inline_w: f32,
    /// Inline layout: reading height / font_size.
    pub inline_h: f32,
}

impl Default for ReadingRatios {
    fn default() -> Self {
        Self {
            w: 6.5,
            h: 1.8,
            inline_w: 10.0,
            inline_h: 1.0,
        }
    }
}

/// Render an extra-large reading that scales to fill available space.
/// Used when graph and recording panels are hidden ("big meter" mode).
/// Returns `(scale_factor, measured_ratios)`. The caller should only
/// persist `measured_ratios` into the cached state when recalculating
/// (e.g. on window resize) to avoid frame-to-frame oscillation.
///
/// `base_content_height`: total height of all content below the reading
/// (buttons, stats, etc.) rendered at scale=1. The caller measures this
/// once and passes it in so we can compute the optimal scale.
pub fn show_reading_large(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    base_content_height: f32,
    ratios: &ReadingRatios,
    tc: &ThemeColors,
    scaled: bool,
) -> (f32, ReadingRatios) {
    let available_w = ui.available_width();
    let available_h = ui.available_height();

    let content_coeff = base_content_height / BASE_READING_FONT_SIZE;

    // Two-line layout: value+unit on top, mode below.
    let two_line_w = available_w / ratios.w;
    let two_line_h = available_h / (ratios.h + content_coeff);
    let two_line_size = two_line_w.min(two_line_h);

    // Inline layout: value+unit+mode all on one row.
    let inline_w = available_w / ratios.inline_w;
    let inline_h = available_h / (ratios.inline_h + content_coeff);
    let inline_size = inline_w.min(inline_h);

    // Use inline layout when it produces an equal or larger font size,
    // meaning the window is wide enough to fit everything on one line
    // without shrinking the value.
    let use_inline = inline_size >= two_line_size;
    let size = if use_inline {
        inline_size
    } else {
        two_line_size
    }
    .max(MIN_BIG_METER_FONT_SIZE);

    // Render and measure actual dimensions.
    let before = ui.cursor().top();
    if use_inline {
        show_reading_inline(ui, measurement, size, tc, scaled);
    } else {
        show_reading_sized(ui, measurement, size, tc, scaled);
    }
    let reading_w = ui.min_rect().width();
    let reading_h = ui.cursor().top() - before;

    let mut measured = ratios.clone();
    if size > 0.0 {
        if use_inline {
            measured.inline_w = reading_w / size;
            measured.inline_h = reading_h / size;
        } else {
            measured.w = reading_w / size;
            measured.h = reading_h / size;
        }
    }

    (size / BASE_READING_FONT_SIZE, measured)
}

/// Render the reading as a compact single line (for narrow layout).
pub fn show_reading_compact(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
) {
    match measurement {
        Some(m) => {
            let value_text = format_value_display(m);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m), scaled),
                || live_region_label(Some(m), scaled),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(
                        RichText::new(&value_text)
                            .font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                    );
                    ui.label(
                        RichText::new(&*m.unit).font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(&*m.mode)
                            .color(ui.visuals().weak_text_color())
                            .small(),
                    );
                    show_flags(ui, m, 0.0, tc, scaled);
                },
            );

            // One summary line rather than the grid: the compact layout is
            // the narrow-window one, where a label/value/unit grid would
            // squeeze the reading itself.
            let summary = m.aux_summary();
            if !summary.is_empty() {
                ui.label(RichText::new(summary).font(FontId::monospace(MIN_AUX_FONT_SIZE)));
            }
        }
        None => {
            // See `show_reading_sized` for why the placeholder is wrapped
            // in a horizontal scope: egui Role::Label silently swallows
            // accesskit set_label overrides.
            ui.live_region_horizontal(
                live_region_fingerprint(None, scaled),
                || live_region_label(None, scaled),
                |ui| {
                    ui.label(
                        RichText::new(format!("{} No reading", crate::NO_DATA))
                            .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                            .color(ui.visuals().weak_text_color()),
                    );
                },
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn format_display_raw_normal() {
        assert_eq!(format_display_raw("  5.678"), "  5.678");
    }

    #[test]
    fn format_display_raw_negative_with_space() {
        // "- 55.79" should be right-aligned to 7 chars
        assert_eq!(format_display_raw("- 55.79"), "- 55.79");
    }

    #[test]
    fn format_display_raw_short_value() {
        // Short values get padded to 7 chars
        assert_eq!(format_display_raw("OL"), "     OL");
    }

    #[test]
    fn format_display_raw_trailing_spaces_trimmed() {
        // Trailing spaces trimmed before alignment
        // "1.23  " → trim_end → "1.23" (4 chars) → right-align to 7
        assert_eq!(format_display_raw("1.23  "), "   1.23");
    }

    #[test]
    fn format_display_raw_full_width() {
        assert_eq!(format_display_raw("-12.345"), "-12.345");
    }

    #[test]
    fn format_display_raw_empty() {
        assert_eq!(format_display_raw(""), "       ");
    }

    /// UT8802/UT8803 flag overload through a status bit while still sending
    /// ordinary digits in the display field. Rendering those digits shows an
    /// out-of-range input as a plausible reading (a bare `0` in Ω mode is
    /// indistinguishable from a real short).
    #[test]
    fn overload_beats_display_raw_digits() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        m.display_raw = Some("    0".to_string());
        assert_eq!(format_value_display(&m).trim(), "OL");
    }

    #[test]
    fn ncv_level_beats_display_raw_digits() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::NcvLevel(3), "", StatusFlags::default());
        m.display_raw = Some("  1.234".to_string());
        assert_eq!(format_value_display(&m).trim(), "NCV 3");
    }

    /// Normal readings must still take the meter's own digits — that is what
    /// holds the on-screen width steady between frames.
    #[test]
    fn normal_still_prefers_display_raw() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        assert_eq!(format_value_display(&m), "  5.678");
    }

    /// The visible reading and the spoken label must agree — this pair was
    /// what made the bug user-visible in the first place.
    #[test]
    fn overload_reads_the_same_visibly_and_aloud() {
        let mut m = Measurement::test_fixture(MeasuredValue::Overload, "Ω", StatusFlags::default());
        m.display_raw = Some("    0".to_string());
        assert_eq!(format_value_display(&m).trim(), "OL");
        assert!(live_region_label(Some(&m), false).starts_with("overload"));
    }

    #[test]
    fn live_region_label_includes_active_flags() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                auto_range: true,
                ..Default::default()
            },
        );
        let label = live_region_label(Some(&m), false);
        assert!(label.contains("V"), "got {label:?}");
        assert!(label.contains("DC V"), "got {label:?}");
        assert!(label.contains("auto range"), "got {label:?}");
        assert!(label.contains("hold"), "got {label:?}");
    }

    /// The meter's high-voltage indicator is a safety signal — a screen
    /// reader user must hear it, and hear it before the routine flags.
    #[test]
    fn live_region_label_announces_high_voltage_first() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(400.0),
            "V",
            StatusFlags {
                hv_warning: true,
                auto_range: true,
                ..Default::default()
            },
        );
        let label = live_region_label(Some(&m), false);
        let hv = label.find("high voltage").expect("HV must be announced");
        let auto = label
            .find("auto range")
            .expect("auto range still announced");
        assert!(hv < auto, "HV must come first, got {label:?}");
    }

    #[test]
    fn live_region_label_no_flags_when_inactive() {
        let m = Measurement::test_fixture(MeasuredValue::Normal(0.0), "V", StatusFlags::default());
        let label = live_region_label(Some(&m), false);
        // StatusFlags::default() is all-false, so no flag phrases should
        // appear in the spoken label.
        assert!(!label.contains("hold"), "got {label:?}");
        assert!(!label.contains("relative"), "got {label:?}");
        assert!(!label.contains("auto range"), "got {label:?}");
    }

    #[test]
    fn live_region_fingerprint_changes_on_flag_toggle() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let fp1 = live_region_fingerprint(Some(&m), false);
        m.flags.hold = true;
        let fp2 = live_region_fingerprint(Some(&m), false);
        assert_ne!(fp1, fp2, "toggling HOLD must change the fingerprint");
        m.flags.hold = false;
        m.flags.rel = true;
        let fp3 = live_region_fingerprint(Some(&m), false);
        assert_ne!(fp1, fp3, "toggling REL must change the fingerprint");
        assert_ne!(fp2, fp3, "REL and HOLD must produce distinct fingerprints");
    }

    #[test]
    fn flags_bits_distinct_per_flag() {
        // Each flag must occupy a distinct bit so toggling any one of them
        // changes the packed u16. Catches accidental bit collisions.
        let names = [
            (
                "hold",
                StatusFlags {
                    hold: true,
                    ..Default::default()
                },
            ),
            (
                "rel",
                StatusFlags {
                    rel: true,
                    ..Default::default()
                },
            ),
            (
                "min",
                StatusFlags {
                    min: true,
                    ..Default::default()
                },
            ),
            (
                "max",
                StatusFlags {
                    max: true,
                    ..Default::default()
                },
            ),
            (
                "auto_range",
                StatusFlags {
                    auto_range: true,
                    ..Default::default()
                },
            ),
            (
                "low_battery",
                StatusFlags {
                    low_battery: true,
                    ..Default::default()
                },
            ),
            (
                "hv_warning",
                StatusFlags {
                    hv_warning: true,
                    ..Default::default()
                },
            ),
            (
                "dc",
                StatusFlags {
                    dc: true,
                    ..Default::default()
                },
            ),
            (
                "peak_max",
                StatusFlags {
                    peak_max: true,
                    ..Default::default()
                },
            ),
            (
                "peak_min",
                StatusFlags {
                    peak_min: true,
                    ..Default::default()
                },
            ),
            (
                "lead_error",
                StatusFlags {
                    lead_error: true,
                    ..Default::default()
                },
            ),
            (
                "comp",
                StatusFlags {
                    comp: true,
                    ..Default::default()
                },
            ),
            (
                "record",
                StatusFlags {
                    record: true,
                    ..Default::default()
                },
            ),
            (
                "loz",
                StatusFlags {
                    loz: true,
                    ..Default::default()
                },
            ),
            (
                "void",
                StatusFlags {
                    void: true,
                    ..Default::default()
                },
            ),
        ];
        let mut seen = std::collections::HashSet::new();
        for (name, flags) in &names {
            let bits = flags_bits(flags);
            assert!(
                bits.count_ones() == 1,
                "{name} should set exactly one bit, got {bits:#b}"
            );
            assert!(seen.insert(bits), "{name} collides with another flag bit");
        }
    }

    #[test]
    fn badge_order_starts_with_hv_and_covers_every_flag_once() {
        let order: Vec<Flag> = badge_order().collect();
        assert_eq!(
            order.first(),
            Some(&Flag::HvWarning),
            "the hazard badge must lead the row"
        );
        assert_eq!(order.len(), StatusFlags::COUNT);

        let mut seen = std::collections::HashSet::new();
        for flag in &order {
            assert!(seen.insert(*flag), "{flag:?} appears twice in badge_order");
        }
    }

    /// The badge row and the spoken label are built from the same iterator, so
    /// anything with a badge must have a phrase and vice versa — the drift
    /// that had screen readers announcing peak flags the row never painted.
    #[test]
    fn every_labelled_flag_is_spoken() {
        for flag in Flag::ALL {
            assert_eq!(
                spoken(flag).is_some(),
                flag.label().is_some(),
                "{flag:?} must be either both labelled and spoken, or neither"
            );
        }
    }

    #[test]
    fn spoken_phrase_lists_peak_flags_the_badges_show() {
        let flags = StatusFlags {
            peak_max: true,
            peak_min: true,
            ..Default::default()
        };
        let mut phrase = String::new();
        append_flags_phrase(&mut phrase, &flags);
        assert!(phrase.contains("peak maximum"), "got {phrase:?}");
        assert!(phrase.contains("peak minimum"), "got {phrase:?}");

        let badges: Vec<&str> = badge_order()
            .filter(|f| flags.get(*f))
            .filter_map(Flag::label)
            .collect();
        assert_eq!(badges, ["P-MAX", "P-MIN"]);
    }

    /// Build a sub-value the way the protocols do: digits in `display_raw`,
    /// unit empty when it matches the main reading's.
    fn aux(label: &'static str, display: &str, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(display.trim().parse().unwrap_or(0.0)),
            unit: unit.into(),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
    }

    /// A UT181A in V AC + Hz shows the frequency and period next to the
    /// voltage; a screen reader user has to hear them too, and hear them
    /// where they are drawn — after the mode, before the flags.
    #[test]
    fn live_region_label_lists_sub_values() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(239.22),
            "VAC",
            StatusFlags {
                auto_range: true,
                ..Default::default()
            },
        );
        m.display_raw = Some(" 239.22".to_string());
        m.aux_values = vec![
            aux("Frequency", "50.01", "Hz"),
            aux("Period", "20.00", "ms"),
        ];

        let label = live_region_label(Some(&m), false);
        assert!(label.contains("Frequency 50.01 Hz"), "got {label:?}");
        assert!(label.contains("Period 20.00 ms"), "got {label:?}");
        let mode = label.find("DC V").expect("mode still announced");
        let freq = label.find("Frequency").expect("sub-value announced");
        let auto = label.find("auto range").expect("flags still announced");
        assert!(mode < freq && freq < auto, "got {label:?}");
    }

    /// A MIN/MAX extreme is only half a reading without the moment it was
    /// caught — the visible row says "@12s", so the spoken one has to say it
    /// too. Singular for one second, since "at 1 seconds" is jarring read
    /// aloud.
    #[test]
    fn live_region_label_speaks_extreme_capture_time() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(4.9871), "V", StatusFlags::default());
        let mut max = aux("Max", "5.9010", "");
        max.elapsed_secs = Some(12);
        let mut min = aux("Min", "4.1200", "");
        min.elapsed_secs = Some(1);
        let plain = aux("Avg", "4.5000", "");
        m.aux_values = vec![max, min, plain];

        let label = live_region_label(Some(&m), false);
        assert!(
            label.contains("Max 5.9010 V at 12 seconds"),
            "got {label:?}"
        );
        assert!(label.contains("Min 4.1200 V at 1 second,"), "got {label:?}");
        // A sub-value without a timestamp must not grow a phantom one.
        assert!(label.ends_with("Avg 4.5000 V"), "got {label:?}");
    }

    /// The unit falls back to the main reading's when the sub-value doesn't
    /// carry its own (MIN/MAX), and an overloaded sub-value is spoken as a
    /// word rather than as the letters "O L".
    #[test]
    fn live_region_label_speaks_aux_fallback_unit_and_overload() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(4.9871), "V", StatusFlags::default());
        let mut max = aux("Max", "5.0123", "");
        max.elapsed_secs = Some(12);
        let mut min = aux("Min", "0", "");
        min.value = MeasuredValue::Overload;
        m.aux_values = vec![max, min];

        let label = live_region_label(Some(&m), false);
        assert!(label.contains("Max 5.0123 V"), "got {label:?}");
        assert!(label.contains("Min overload V"), "got {label:?}");
    }

    /// Degree symbols are spelled out only in the spoken string — screen
    /// readers differ on whether they voice "°" at all, and "24.1 C" is not
    /// a temperature. The main reading and its sub-values get the same
    /// treatment, so a dual-thermocouple reading is voiced consistently.
    #[test]
    fn live_region_label_spells_out_degrees() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(23.5),
            "\u{00B0}C",
            StatusFlags::default(),
        );
        m.display_raw = Some("   23.5".to_string());
        m.aux_values = vec![aux("T2", "24.10", "\u{00B0}C")];
        let label = live_region_label(Some(&m), false);
        assert!(label.starts_with("23.5 degrees C"), "got {label:?}");
        assert!(label.contains("T2 24.10 degrees C"), "got {label:?}");
        assert!(
            !label.contains('\u{00B0}'),
            "the symbol must not survive into the spoken label, got {label:?}"
        );
    }

    /// Single-display meters must be announced exactly as before sub-values
    /// existed.
    #[test]
    fn live_region_label_unchanged_without_sub_values() {
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(1.234),
            "V",
            StatusFlags {
                hold: true,
                ..Default::default()
            },
        );
        assert!(m.aux_values.is_empty());
        assert_eq!(live_region_label(Some(&m), false), "5.678 V, DC V, hold");
    }

    /// A MIN/MAX extreme can move while the live reading is unchanged, so
    /// the cached announcement has to be invalidated by the sub-values too.
    #[test]
    fn live_region_fingerprint_changes_on_sub_value_change() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(1.0), "V", StatusFlags::default());
        let bare = live_region_fingerprint(Some(&m), false);

        m.aux_values = vec![aux("Max", "5.0123", "")];
        let with_aux = live_region_fingerprint(Some(&m), false);
        assert_ne!(bare, with_aux, "a sub-value appearing must be noticed");

        m.aux_values[0] = aux("Max", "5.0456", "");
        let moved = live_region_fingerprint(Some(&m), false);
        assert_ne!(with_aux, moved, "a sub-value changing must be noticed");

        m.aux_values[0].elapsed_secs = Some(12);
        let stamped = live_region_fingerprint(Some(&m), false);
        assert_ne!(moved, stamped, "the @Ns column changing must be noticed");

        m.aux_values[0].label = "Min".into();
        assert_ne!(
            stamped,
            live_region_fingerprint(Some(&m), false),
            "a relabelled sub-value must be noticed"
        );
    }

    /// Lay out the sub-value rows in a headless egui context and return the
    /// (label rect, value rect) pairs of the last frame.
    ///
    /// Several frames are run because `egui::Grid` sizes a row from the
    /// heights it recorded on the *previous* frame — on the very first pass
    /// every cell is still its own natural height, so vertical alignment
    /// only becomes meaningful once the grid has settled.
    fn layout_aux_rows(m: &Measurement, font_size: f32) -> Vec<AuxRowRects> {
        let ctx = eframe::egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut rects = Vec::new();
        for _ in 0..3 {
            rects.clear();
            let _ = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
                rects = show_aux_rows(ui, m, font_size, &tc);
            });
        }
        rects
    }

    /// The label and its value must sit on the same line.
    ///
    /// Regression test for the big-meter offset: wrapping the value in a
    /// nested `ui.horizontal` made egui allocate the cell at
    /// `interact_size.y` (~18 px) and then push the taller content down, so
    /// at a 130 px reading font the value hung roughly half a line below its
    /// label. Every cell is a plain label now, and the grid's own
    /// `LEFT_CENTER` alignment lines them up at any size.
    #[test]
    fn aux_label_and_value_share_a_baseline() {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(23.5),
            "\u{00B0}C",
            StatusFlags::default(),
        );
        m.display_raw = Some("   23.5".to_string());
        m.aux_values = vec![aux("T2", "24.10", "\u{00B0}C"), aux("T1", "23.50", "")];

        // 36.0 is the side-panel reading size, 130.0 a big-meter one.
        for size in [36.0_f32, 130.0] {
            let rows = layout_aux_rows(&m, size);
            assert_eq!(rows.len(), 2, "one row per sub-value at {size} px");
            for (i, (label, value)) in rows.iter().enumerate() {
                let delta = (label.center().y - value.center().y).abs();
                assert!(
                    delta <= 1.0,
                    "row {i} at {size} px: label centre {} vs value centre {} (delta {delta} px)",
                    label.center().y,
                    value.center().y
                );
            }
        }
    }

    /// Single-display meters must draw nothing at all — no grid, no row.
    #[test]
    fn aux_rows_draw_nothing_without_sub_values() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(1.234), "V", StatusFlags::default());
        assert!(m.aux_values.is_empty());
        assert!(layout_aux_rows(&m, 130.0).is_empty());
    }

    /// The sub-value rows are monospace, so a fixed width keeps the digits
    /// from shifting sideways as the reading changes.
    #[test]
    fn format_aux_value_pads_to_the_reading_width() {
        assert_eq!(
            format_aux_value(&aux("Frequency", "50.01", "Hz")),
            "  50.01"
        );
        let mut over = aux("Max", "0", "");
        over.value = MeasuredValue::Overload;
        assert_eq!(format_aux_value(&over), "     OL");
    }

    /// The SCALE badge is visual; the spoken label is how an AT user learns
    /// the reading has been through a software transform.
    #[test]
    fn the_live_region_label_says_when_the_reading_is_software_scaled() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(12.34), "A", StatusFlags::default());
        let plain = live_region_label(Some(&m), false);
        assert!(!plain.contains("software scaled"), "{plain:?}");
        let scaled = live_region_label(Some(&m), true);
        assert!(scaled.ends_with(", software scaled"), "{scaled:?}");
        assert!(scaled.starts_with(&plain), "{scaled:?} vs {plain:?}");
    }

    /// The cached announcement is only rebuilt when the fingerprint moves,
    /// so toggling the transform has to move it.
    #[test]
    fn the_live_region_fingerprint_tracks_the_scaled_bit() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(12.34), "A", StatusFlags::default());
        assert_ne!(
            live_region_fingerprint(Some(&m), false),
            live_region_fingerprint(Some(&m), true)
        );
    }

    #[test]
    fn format_display_raw_consistent_width() {
        // All outputs should be at least 7 chars wide
        let inputs = [" 0.0000", "  5.678", "-12.345", "    OL ", "- 55.79"];
        for input in &inputs {
            let output = format_display_raw(input);
            assert!(
                output.len() >= 7,
                "format_display_raw({input:?}) = {output:?} should be >= 7 chars"
            );
        }
    }
}

fn show_flags(ui: &mut Ui, m: &Measurement, font_size: f32, tc: &ThemeColors, scaled: bool) {
    let badge = |ui: &mut Ui, label: &str, color: Color32| {
        let mut text = RichText::new(label).strong().color(color);
        if font_size > 0.0 {
            text = text.font(FontId::proportional(font_size));
        } else {
            text = text.small();
        }
        ui.label(text);
    };

    // `badge_order()` puts the hazard first, and `badge_tone` paints it in the
    // error color rather than the generic warning one — this is the meter
    // telling the user the probes are on a dangerous potential. Labels come
    // from `Flag::label()`, the same source as `StatusFlags::Display`, so the
    // badge, the recording panel and the CSV flags column all say "HV!".
    // `Flag::Dc` has no label and so paints no badge.
    for (flag, label) in badge_order()
        .filter(|f| m.flags.get(*f))
        .filter_map(|f| f.label().map(|l| (f, l)))
    {
        badge(ui, label, badge_tone(flag, tc));
    }
    // After the meter's own badges, in the same accent as AUTO/HOLD: this is
    // the app's state, not the meter's, and it belongs at the end of the row
    // rather than mixed in among the flags the meter reported.
    if scaled {
        badge(ui, "SCALE", tc.accent());
    }
}
