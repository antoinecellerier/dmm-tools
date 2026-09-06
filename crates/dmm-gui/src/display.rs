use dmm_lib::flags::{Flag, StatusFlags};
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use dmm_lib::protocol::ModeChoice;
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    Color32, ComboBox, Context, EventFilter, FocusDirection, FontId, Grid, Id, Key, Modifiers,
    Popup, Rect, Response, RichText, Stroke, TextFormat, TextStyle, Ui,
};
use std::borrow::Cow;

use crate::a11y::{ResponseA11yExt, UiA11yExt};
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

/// Largest font the mode selector's popup entries use. The readout itself
/// follows the reading — a 200 px big-meter reading puts it at 80 px — but
/// the popup is a list to pick from, and stays list-sized.
const MAX_MODE_POPUP_FONT_SIZE: f32 = 18.0;

/// Marks the live entry in the mode selector's popup, in text as well as in
/// the selection colour.
const LIVE_MODE_MARK: &str = "\u{25CF}";

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

/// Whether the mode readout is a selector rather than a plain label.
///
/// A single choice is the live mode on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to pick, and no control to draw.
pub(crate) fn mode_switch_offered(choices: &[ModeChoice]) -> bool {
    choices.len() > 1
}

/// The mode readout under the reading: a plain label, or — when the meter
/// can be switched to other modes from where its dial sits — a dropdown
/// listing them, with the live one marked.
///
/// Returns the id of a mode the user picked that differs from the live one.
///
/// The dropdown is drawn at the label's size and with no frame at rest, so
/// it reads as the readout it replaces; hover and the open state keep egui's
/// own highlight so it still answers as a control. Its popup is capped at
/// [`MAX_MODE_POPUP_FONT_SIZE`].
///
/// The open list behaves as a native listbox: focus lands on the live entry
/// as it opens, Up/Down (Home/End) move it, Enter/Space or a click picks,
/// Esc or Tab closes without a pick, and focus returns to the readout on
/// every close. The mechanisms are the ones `color_edit` uses for its
/// picker: a was-open flag to see the open and close transitions, consumed
/// keys plus a cancelled focus move, and a focus lock filter on the entry.
fn show_mode_readout(ui: &mut Ui, mode: &str, size: f32, choices: &[ModeChoice]) -> Option<u16> {
    if !mode_switch_offered(choices) {
        ui.label(
            RichText::new(mode)
                .font(FontId::proportional(size))
                .color(ui.visuals().weak_text_color()),
        );
        return None;
    }
    let current = choices.iter().find(|c| c.current).map(|c| c.id);
    let mut picked = current;
    let popup_size = size.clamp(MIN_AUX_FONT_SIZE, MAX_MODE_POPUP_FONT_SIZE);
    let ctx = ui.ctx().clone();
    let response = ui
        .scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            widgets.inactive.bg_stroke = Stroke::NONE;
            // egui draws its arrow to `icon_width`, a fixed 14 px that is
            // lost beside a big-meter readout; follow the text instead.
            let spacing = ui.spacing_mut();
            spacing.icon_width = (size * 0.8).max(8.0);
            spacing.icon_spacing = (size * 0.25).max(2.0);
            // The interactive colour, not the label's weak one: this is a
            // control, and it has to clear the text contrast bar as one.
            let text_color = ui.visuals().text_color();

            // The id `ComboBox::from_id_salt` derives below, known up front
            // so the list's state can be read before the box is drawn. The
            // salt is wrapped in `Id::new` the way `from_id_salt` wraps it:
            // hashing the bare string gives a different id.
            let button_id = ui.make_persistent_id(Id::new("mode_select"));
            let was_open_key = button_id.with("was_open");
            let was_open: bool = ctx.data(|d| d.get_temp(was_open_key)).unwrap_or(false);

            // Keys that leave the list, handled before it is drawn so this
            // frame already shows it closed: Tab and Shift+Tab step out of a
            // listbox rather than through it, Esc abandons it. `consume_key`
            // drops the press; `move_focus(None)` cancels the focus jump egui
            // queued from it as the frame began.
            if was_open {
                let leave = ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Tab)
                        | i.consume_key(Modifiers::SHIFT, Key::Tab)
                        | i.consume_key(Modifiers::NONE, Key::Escape)
                });
                if leave {
                    Popup::close_all(&ctx);
                    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
                }
            }

            let mut activated = false;
            let inner = ComboBox::from_id_salt("mode_select")
                .width(0.0)
                .selected_text(
                    RichText::new(mode)
                        .font(FontId::proportional(size))
                        .color(text_color),
                )
                .show_ui(ui, |ui| {
                    // No `set_modal_layer` here, unlike `color_edit`: Tab
                    // and the arrows are handled outright, and the modal
                    // layer outlives the list by a frame, in which egui
                    // surrenders the focus just handed back to the readout
                    // (`Context::create_widget` on a layer below the modal).
                    let mut entries = Vec::with_capacity(choices.len());
                    for c in choices {
                        // Three spaces sit close enough under the mark to
                        // keep the entries aligned without a figure space
                        // the bundled fonts may not have.
                        let text = if c.current {
                            format!("{LIVE_MODE_MARK} {}", c.label)
                        } else {
                            format!("   {}", c.label)
                        };
                        // egui's selectable label reports itself as a plain
                        // button, so the selected state is set by hand.
                        let entry = ui
                            .selectable_value(
                                &mut picked,
                                Some(c.id),
                                RichText::new(text).font(FontId::proportional(popup_size)),
                            )
                            .a11y_toggled(c.current);
                        // Focus lands on the live entry as the list opens —
                        // by click, or by Enter/Space on the readout — so a
                        // screen reader announces it and Enter picks it.
                        if !was_open && c.current {
                            entry.request_focus();
                        }
                        activated |= entry.clicked();
                        entries.push(entry);
                    }
                    navigate_mode_entries(&ctx, &entries);
                });

            // Enter/Space "clicks" the focused entry without a pointer
            // click, which is the only thing a menu popup closes on by
            // itself.
            if activated {
                Popup::close_all(&ctx);
            }
            let is_open = ComboBox::is_open(&ctx, button_id);
            if was_open && !is_open {
                // Closed by any route — pick, Esc, Tab, click elsewhere:
                // focus goes back to the readout rather than to the top of
                // the Tab order.
                ctx.memory_mut(|m| m.request_focus(button_id));
            }
            ctx.data_mut(|d| d.insert_temp(was_open_key, is_open));
            inner.response
        })
        .inner;
    response
        .on_hover_text("Switch the meter to another mode of the current dial position")
        .a11y_label("Mode");
    if picked == current { None } else { picked }
}

/// Keyboard navigation inside the open mode list: ArrowDown/ArrowUp move
/// focus between the entries, clamped at the ends; Home/End jump to the
/// first and last. Nothing reaches the meter until Enter, Space or a click
/// picks the focused entry.
fn navigate_mode_entries(ctx: &Context, entries: &[Response]) {
    let Some(i) = entries.iter().position(Response::has_focus) else {
        return;
    };
    let last = entries.len() - 1;
    let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    let target = if pressed(Key::ArrowDown) {
        (i + 1).min(last)
    } else if pressed(Key::ArrowUp) {
        i.saturating_sub(1)
    } else if pressed(Key::Home) {
        0
    } else if pressed(Key::End) {
        last
    } else {
        i
    };
    // egui queued its own spatial move from the same press (and would move
    // on Left/Right too); cancel it so the list's is the only one.
    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
    if target != i {
        entries[target].request_focus();
    }
    // From the next frame on, arrows, Tab and Esc are delivered to the
    // focused entry instead of moving focus. The filter only binds once the
    // entry has held focus for a frame, which is what the reset above covers.
    ctx.memory_mut(|m| {
        m.set_focus_lock_filter(
            entries[i].id,
            EventFilter {
                tab: true,
                vertical_arrows: true,
                escape: true,
                ..Default::default()
            },
        );
    });
}

/// Single-line reading with the mode selector beside it.
///
/// The value and unit form the live region; the selector, a control, sits
/// after it in the same row rather than inside it — a screen reader would
/// otherwise have the dropdown re-announced with every reading update.
/// `draw_value` paints the value and unit labels.
fn show_reading_line_with_selector(
    ui: &mut Ui,
    m: &Measurement,
    scaled: bool,
    draw_value: impl FnOnce(&mut Ui),
    mode_size: f32,
    mode_choices: &[ModeChoice],
    tc: &ThemeColors,
) -> Option<u16> {
    ui.horizontal(|ui| {
        ui.live_region_horizontal(
            live_region_fingerprint(Some(m), scaled),
            || live_region_label(Some(m), scaled),
            |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                draw_value(ui);
            },
        );
        ui.separator();
        let picked = show_mode_readout(ui, &m.mode, mode_size, mode_choices);
        show_flags(ui, m, mode_size, tc, scaled);
        picked
    })
    .inner
}

/// Render the primary reading display at the given font size (two-line layout).
///
/// Returns the mode the user picked from the selector, if any.
fn show_reading_sized(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
    mode_choices: &[ModeChoice],
) -> Option<u16> {
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
                let picked = show_mode_readout(ui, &m.mode, mode_size, mode_choices);
                if !m.range_label.is_empty() {
                    ui.label(
                        RichText::new(&*m.range_label)
                            .font(FontId::proportional(mode_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                show_flags(ui, m, mode_size, tc, scaled);
                picked
            })
            .inner
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
            None
        }
    }
}

/// Render the reading with value and mode on a single line (inline layout).
///
/// Returns the mode the user picked from the selector, if any.
fn show_reading_inline(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
    mode_choices: &[ModeChoice],
) -> Option<u16> {
    let unit_size = value_size;
    let mode_size = value_size * 0.4;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(ui, m, tc);
            let draw_value = |ui: &mut Ui| {
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
            };

            let picked = if !mode_switch_offered(mode_choices) {
                ui.live_region_horizontal(
                    live_region_fingerprint(Some(m), scaled),
                    || live_region_label(Some(m), scaled),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        draw_value(ui);
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
                None
            } else {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = (mode_size * 0.3).max(2.0);
                    show_reading_line_with_selector(
                        ui,
                        m,
                        scaled,
                        draw_value,
                        mode_size,
                        mode_choices,
                        tc,
                    )
                })
                .inner
            };

            // Sub-values still get their own rows in the inline layout: the
            // single line is already the widest thing on screen, and folding
            // four UT181A sub-values into it would force the value font down.
            let _ = show_aux_rows(ui, m, mode_size, tc);
            picked
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
            None
        }
    }
}

/// Render the large primary reading display.
///
/// `scaled` marks the reading as passed through a software transform, so the
/// SCALE badge and the spoken label say so. `mode_choices` are the modes the
/// meter can be switched into; when there are any, the mode readout becomes
/// a selector, and the id of a mode the user picks is returned.
pub fn show_reading(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
    mode_choices: &[ModeChoice],
) -> Option<u16> {
    show_reading_sized(
        ui,
        measurement,
        BASE_READING_FONT_SIZE,
        tc,
        scaled,
        mode_choices,
    )
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
/// Returns `(scale_factor, measured_ratios, picked_mode)`. The caller should
/// only persist `measured_ratios` into the cached state when recalculating
/// (e.g. on window resize) to avoid frame-to-frame oscillation;
/// `picked_mode` is the id of a mode the user chose in the selector, as for
/// [`show_reading`].
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
    mode_choices: &[ModeChoice],
) -> (f32, ReadingRatios, Option<u16>) {
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
    let picked = if use_inline {
        show_reading_inline(ui, measurement, size, tc, scaled, mode_choices)
    } else {
        show_reading_sized(ui, measurement, size, tc, scaled, mode_choices)
    };
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

    (size / BASE_READING_FONT_SIZE, measured, picked)
}

/// Render the reading as a compact single line (for narrow layout).
///
/// Returns the mode the user picked from the selector, as for
/// [`show_reading`].
pub fn show_reading_compact(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
    mode_choices: &[ModeChoice],
) -> Option<u16> {
    match measurement {
        Some(m) => {
            let value_text = format_value_display(m);
            let draw_value = |ui: &mut Ui| {
                ui.label(
                    RichText::new(&value_text).font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                );
                ui.label(
                    RichText::new(&*m.unit).font(FontId::monospace(COMPACT_READING_FONT_SIZE)),
                );
            };

            let picked = if !mode_switch_offered(mode_choices) {
                ui.live_region_horizontal(
                    live_region_fingerprint(Some(m), scaled),
                    || live_region_label(Some(m), scaled),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        draw_value(ui);
                        ui.separator();
                        ui.label(
                            RichText::new(&*m.mode)
                                .color(ui.visuals().weak_text_color())
                                .small(),
                        );
                        show_flags(ui, m, 0.0, tc, scaled);
                    },
                );
                None
            } else {
                // The selector and badges at the small text size, which is
                // what `.small()` resolves to for the plain label.
                let small = TextStyle::Small.resolve(ui.style()).size;
                show_reading_line_with_selector(ui, m, scaled, draw_value, small, mode_choices, tc)
            };

            // One summary line rather than the grid: the compact layout is
            // the narrow-window one, where a label/value/unit grid would
            // squeeze the reading itself.
            let summary = m.aux_summary();
            if !summary.is_empty() {
                ui.label(RichText::new(summary).font(FontId::monospace(MIN_AUX_FONT_SIZE)));
            }
            picked
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
            None
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

    // ---- the mode selector ------------------------------------------------

    use eframe::egui;
    use eframe::egui::accesskit::{Node, NodeId, Role, Toggled};

    fn choice(id: u16, label: &'static str, current: bool) -> ModeChoice {
        ModeChoice {
            id,
            label: Cow::Borrowed(label),
            current,
        }
    }

    /// Two entries for one dial position, the fixture's own mode live so the
    /// readout and the marked entry agree.
    fn two_choices() -> Vec<ModeChoice> {
        vec![
            choice(0x1111, "DC V", true),
            choice(0x1121, "AC V Hz", false),
        ]
    }

    /// The layouts the readout appears in, drawn at the side-panel size.
    const LAYOUTS: [&str; 3] = ["two-line", "inline", "compact"];

    fn draw_reading(
        ui: &mut Ui,
        layout: &str,
        m: &Measurement,
        choices: &[ModeChoice],
    ) -> Option<u16> {
        let tc = crate::settings::Settings::default().theme_colors(true);
        match layout {
            "two-line" => {
                show_reading_sized(ui, Some(m), BASE_READING_FONT_SIZE, &tc, false, choices)
            }
            "inline" => {
                show_reading_inline(ui, Some(m), BASE_READING_FONT_SIZE, &tc, false, choices)
            }
            _ => show_reading_compact(ui, Some(m), &tc, false, choices),
        }
    }

    /// One headless frame with AccessKit on: what `draw` returned, the
    /// frame's accessibility nodes and the focused one — the one place a
    /// test can see which widgets were drawn, what they are called, where
    /// they are, and which has the keyboard.
    struct Frame {
        picked: Option<u16>,
        nodes: Vec<(NodeId, Node)>,
        focus: Option<NodeId>,
    }

    fn run_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        mut draw: impl FnMut(&mut Ui) -> Option<u16>,
    ) -> Frame {
        ctx.enable_accesskit();
        let mut picked = None;
        let out = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| picked = draw(ui),
        );
        let (nodes, focus) = out
            .platform_output
            .accesskit_update
            .map(|update| (update.nodes, Some(update.focus)))
            .unwrap_or_default();
        Frame {
            picked,
            nodes,
            focus,
        }
    }

    fn node_with_role(nodes: &[(NodeId, Node)], role: Role) -> Option<&(NodeId, Node)> {
        nodes.iter().find(|(_, n)| n.role() == role)
    }

    fn node_labelled<'a>(nodes: &'a [(NodeId, Node)], label: &str) -> Option<&'a Node> {
        nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.label() == Some(label))
    }

    fn node_id_labelled(nodes: &[(NodeId, Node)], label: &str) -> Option<NodeId> {
        nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .map(|(id, _)| *id)
    }

    /// A key pressed and released within one frame.
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect()
    }

    /// Every node under `root`, however deep.
    fn descendants(nodes: &[(NodeId, Node)], root: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            if let Some((_, n)) = nodes.iter().find(|(nid, _)| *nid == id) {
                for child in n.children() {
                    out.push(*child);
                    todo.push(*child);
                }
            }
        }
        out
    }

    fn centre(node: &Node) -> egui::Pos2 {
        let b = node.bounds().expect("drawn widgets have bounds");
        egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    }

    fn pointer(pos: egui::Pos2, pressed: Option<bool>) -> Vec<egui::Event> {
        let mut events = vec![egui::Event::PointerMoved(pos)];
        if let Some(pressed) = pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        events
    }

    /// Click `pos` the way a mouse does — move, press, release on successive
    /// frames — and return the release frame.
    fn click(
        ctx: &egui::Context,
        pos: egui::Pos2,
        mut draw: impl FnMut(&mut Ui) -> Option<u16>,
    ) -> Frame {
        run_frame(ctx, pointer(pos, None), &mut draw);
        run_frame(ctx, pointer(pos, Some(true)), &mut draw);
        run_frame(ctx, pointer(pos, Some(false)), &mut draw)
    }

    /// Meters without remote mode selection keep the readout they had: a
    /// plain label, no combo box anywhere, nothing to report.
    #[test]
    fn the_mode_readout_stays_a_label_without_choices() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, &[]));
            assert_eq!(f.picked, None, "{layout}");
            assert!(
                node_with_role(&f.nodes, Role::ComboBox).is_none(),
                "{layout}: a combo box was drawn with no choices"
            );
            assert!(
                f.nodes
                    .iter()
                    .any(|(_, n)| n.role() == Role::Label && n.value() == Some("DC V")),
                "{layout}: the mode label is missing"
            );
        }
    }

    /// The single-variant UT181A dials (Ohm, nS, Cap, Hz, Duty, Pulse Width)
    /// report one choice: the mode the meter is already in. A dropdown whose
    /// only entry is selected has nothing to offer, so the readout stays the
    /// plain label — the same as a meter that cannot switch at all.
    #[test]
    fn the_mode_readout_stays_a_label_with_only_the_live_mode() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = [choice(0x5111, "DC V", true)];
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, &choices));
            assert_eq!(f.picked, None, "{layout}");
            assert!(
                node_with_role(&f.nodes, Role::ComboBox).is_none(),
                "{layout}: a combo box was drawn for the live mode alone"
            );
            assert!(
                f.nodes
                    .iter()
                    .any(|(_, n)| n.role() == Role::Label && n.value() == Some("DC V")),
                "{layout}: the mode label is missing"
            );
        }
    }

    /// With choices the readout is a combo box a screen reader calls "Mode"
    /// whose value is the live mode — and, in the single-line layouts, it
    /// sits beside the live region rather than inside it, so it is not
    /// re-announced with every reading.
    #[test]
    fn the_mode_readout_becomes_a_named_combo_with_choices() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, &choices));
            assert_eq!(f.picked, None, "{layout}: drawing is not picking");
            let nodes = &f.nodes;
            let (combo_id, combo) = node_with_role(nodes, Role::ComboBox)
                .unwrap_or_else(|| panic!("{layout}: no combo box"));
            assert_eq!(combo.label(), Some("Mode"), "{layout}");
            assert_eq!(combo.value(), Some("DC V"), "{layout}");

            let live_regions: Vec<NodeId> = nodes
                .iter()
                .filter(|(_, n)| n.live() == Some(egui::accesskit::Live::Polite))
                .map(|(id, _)| *id)
                .collect();
            assert!(
                !live_regions.is_empty(),
                "{layout}: the reading lost its live region"
            );
            for region in live_regions {
                assert!(
                    !descendants(nodes, region).contains(combo_id),
                    "{layout}: the selector is inside the live region"
                );
            }
        }
    }

    /// Open the popup, see both entries with the live one marked, pick the
    /// other: its id comes back. Drawn at a big-meter size so the popup's
    /// font cap is exercised — an 80 px readout must not open an 80 px list.
    #[test]
    fn picking_another_mode_reports_its_id() {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        let mut draw = |ui: &mut Ui| show_reading_sized(ui, Some(&m), 200.0, &tc, false, &choices);

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(
            &node_with_role(&f.nodes, Role::ComboBox)
                .expect("combo box")
                .1,
        );
        let f = click(&ctx, combo, &mut draw);
        assert_eq!(f.picked, None, "opening the popup is not a pick");

        let f = run_frame(&ctx, vec![], &mut draw);
        let live =
            node_labelled(&f.nodes, &format!("{LIVE_MODE_MARK} DC V")).expect("live entry, marked");
        assert_eq!(live.toggled(), Some(Toggled::True));
        let other = node_labelled(&f.nodes, "   AC V Hz").expect("other entry");
        assert_eq!(other.toggled(), Some(Toggled::False));
        let b = other.bounds().expect("entry bounds");
        let height = (b.y1 - b.y0) as f32;
        assert!(
            height < 2.0 * MAX_MODE_POPUP_FONT_SIZE,
            "popup entry {height} px tall beside an 80 px readout"
        );

        let f = click(&ctx, centre(other), &mut draw);
        assert_eq!(f.picked, Some(0x1121));
    }

    /// Re-picking the live mode sends nothing to the meter.
    #[test]
    fn picking_the_live_mode_is_not_a_pick() {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
        };

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(
            &node_with_role(&f.nodes, Role::ComboBox)
                .expect("combo box")
                .1,
        );
        click(&ctx, combo, &mut draw);
        let f = run_frame(&ctx, vec![], &mut draw);
        let live = node_labelled(&f.nodes, &format!("{LIVE_MODE_MARK} DC V")).expect("live entry");
        let f = click(&ctx, centre(live), &mut draw);
        assert_eq!(f.picked, None);
    }

    // ---- keyboard ---------------------------------------------------------

    const LIVE: &str = "\u{25CF} DC V";
    const OTHER: &str = "   AC V Hz";

    /// Tab reaches the readout — the only focusable widget in a bare
    /// reading — and Enter opens the list. Returns the frame after opening.
    fn open_with_keyboard(
        ctx: &egui::Context,
        mut draw: impl FnMut(&mut Ui) -> Option<u16>,
    ) -> Frame {
        run_frame(ctx, vec![], &mut draw);
        let f = run_frame(ctx, key(egui::Key::Tab, egui::Modifiers::NONE), &mut draw);
        let combo = node_with_role(&f.nodes, Role::ComboBox)
            .expect("combo box")
            .0;
        assert_eq!(f.focus, Some(combo), "Tab must reach the readout");
        run_frame(ctx, key(egui::Key::Enter, egui::Modifiers::NONE), &mut draw);
        run_frame(ctx, vec![], &mut draw)
    }

    /// The list is gone and the readout has the keyboard again.
    fn assert_closed_on_the_readout(ctx: &egui::Context, draw: impl FnMut(&mut Ui) -> Option<u16>) {
        let f = run_frame(ctx, vec![], draw);
        assert!(
            node_labelled(&f.nodes, OTHER).is_none(),
            "the list is still open"
        );
        let combo = node_with_role(&f.nodes, Role::ComboBox)
            .expect("combo box")
            .0;
        assert_eq!(f.focus, Some(combo), "focus must return to the readout");
    }

    fn keyboard_fixture() -> (Measurement, Vec<ModeChoice>, ThemeColors) {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        (
            m,
            two_choices(),
            crate::settings::Settings::default().theme_colors(true),
        )
    }

    /// Opening the list — by Enter on the Tab-focused readout, and by a
    /// click — puts focus straight on the live entry: Enter would pick it,
    /// and a screen reader announces it, with no Tab needed first.
    #[test]
    fn opening_the_list_focuses_the_live_entry() {
        let (m, choices, tc) = keyboard_fixture();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
        };

        let ctx = egui::Context::default();
        let f = open_with_keyboard(&ctx, &mut draw);
        let live = node_id_labelled(&f.nodes, LIVE).expect("live entry");
        assert_eq!(f.focus, Some(live), "keyboard open");

        let ctx = egui::Context::default();
        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(
            &node_with_role(&f.nodes, Role::ComboBox)
                .expect("combo box")
                .1,
        );
        click(&ctx, combo, &mut draw);
        let f = run_frame(&ctx, vec![], &mut draw);
        let live = node_id_labelled(&f.nodes, LIVE).expect("live entry");
        assert_eq!(f.focus, Some(live), "mouse open");
    }

    /// ArrowDown moves focus to the next entry and stops at the last without
    /// sending anything; Enter then picks the focused entry, closes the list
    /// and hands focus back to the readout.
    #[test]
    fn arrow_down_then_enter_picks_the_next_entry() {
        let (m, choices, tc) = keyboard_fixture();
        let ctx = egui::Context::default();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
        };
        open_with_keyboard(&ctx, &mut draw);

        run_frame(
            &ctx,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
            &mut draw,
        );
        let f = run_frame(&ctx, vec![], &mut draw);
        let other = node_id_labelled(&f.nodes, OTHER).expect("other entry");
        assert_eq!(f.focus, Some(other));

        let f = run_frame(
            &ctx,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
            &mut draw,
        );
        assert_eq!(f.picked, None, "moving is not picking");
        let f = run_frame(&ctx, vec![], &mut draw);
        assert_eq!(f.focus, Some(other), "clamped at the last entry");

        let f = run_frame(
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
            &mut draw,
        );
        assert_eq!(f.picked, Some(0x1121));
        assert_closed_on_the_readout(&ctx, &mut draw);
    }

    /// ArrowUp stops at the first entry; Home and End jump to the ends.
    #[test]
    fn home_end_and_arrow_up_stay_within_the_list() {
        let (m, choices, tc) = keyboard_fixture();
        let ctx = egui::Context::default();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
        };
        let f = open_with_keyboard(&ctx, &mut draw);
        let live = node_id_labelled(&f.nodes, LIVE).expect("live entry");
        let other = node_id_labelled(&f.nodes, OTHER).expect("other entry");

        for (k, want, what) in [
            (egui::Key::ArrowUp, live, "ArrowUp at the first entry stays"),
            (egui::Key::End, other, "End jumps to the last entry"),
            (egui::Key::Home, live, "Home jumps to the first entry"),
        ] {
            run_frame(&ctx, key(k, egui::Modifiers::NONE), &mut draw);
            let f = run_frame(&ctx, vec![], &mut draw);
            assert_eq!(f.focus, Some(want), "{what}");
            assert!(
                node_labelled(&f.nodes, OTHER).is_some(),
                "{what}: list closed"
            );
        }
    }

    /// Esc closes the list with nothing picked, focus back on the readout.
    #[test]
    fn escape_closes_the_list_without_a_pick() {
        let (m, choices, tc) = keyboard_fixture();
        let ctx = egui::Context::default();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
        };
        open_with_keyboard(&ctx, &mut draw);
        run_frame(
            &ctx,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
            &mut draw,
        );
        let f = run_frame(
            &ctx,
            key(egui::Key::Escape, egui::Modifiers::NONE),
            &mut draw,
        );
        assert_eq!(f.picked, None);
        assert_closed_on_the_readout(&ctx, &mut draw);
    }

    /// Tab and Shift+Tab step out of the list rather than through it: it
    /// closes with nothing picked, and focus is on the readout, not wherever
    /// egui's Tab cycle would have gone.
    #[test]
    fn tab_closes_the_list_without_a_pick() {
        let (m, choices, tc) = keyboard_fixture();
        for modifiers in [egui::Modifiers::NONE, egui::Modifiers::SHIFT] {
            let ctx = egui::Context::default();
            let mut draw = |ui: &mut Ui| {
                show_reading_sized(ui, Some(&m), BASE_READING_FONT_SIZE, &tc, false, &choices)
            };
            open_with_keyboard(&ctx, &mut draw);
            run_frame(
                &ctx,
                key(egui::Key::ArrowDown, egui::Modifiers::NONE),
                &mut draw,
            );
            let f = run_frame(&ctx, key(egui::Key::Tab, modifiers), &mut draw);
            assert_eq!(f.picked, None, "{modifiers:?}");
            assert_closed_on_the_readout(&ctx, &mut draw);
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
