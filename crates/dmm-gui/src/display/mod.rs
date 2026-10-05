mod text;

use dmm_lib::flags::Flag;
use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use dmm_lib::protocol::{Choice, MeterKey, Setting};
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    Align, Color32, ComboBox, Context, EventFilter, FocusDirection, FontId, Grid, Id, IdSalt, Key,
    Layout, Modifiers, Popup, Rect, Response, RichText, Stroke, TextFormat, TextStyle,
    TextWrapMode, Ui, UiBuilder, Vec2, WidgetText, vec2,
};

use crate::a11y::{ResponseA11yExt, UiA11yExt};
use crate::theme::ThemeColors;
use text::{
    badge_order, format_aux_value, format_value_display, live_region_fingerprint, live_region_label,
};

/// Base font size for the primary reading in the wide (side panel) layout.
pub(crate) const BASE_READING_FONT_SIZE: f32 = 36.0;

/// Minimum font size for the big meter scaled reading. Smaller than
/// `BASE_READING_FONT_SIZE` so the window can shrink to a tiny widget.
pub(crate) const MIN_BIG_METER_FONT_SIZE: f32 = 12.0;

/// Font size for the primary reading in the compact (narrow) layout.
const COMPACT_READING_FONT_SIZE: f32 = 28.0;

/// Floor for the readout's secondary text: the sub-value rows and the big
/// meter's connection notice. Both derive their size from the caller's
/// reading size, which shrinks to `MIN_BIG_METER_FONT_SIZE` in a tiny big-meter
/// window; without this floor the derived size would fall under the 11 pt
/// minimum `.claude/rules/gui.md` sets.
const MIN_AUX_FONT_SIZE: f32 = 11.0;

/// What the readout says when there is nothing to show and nothing to blame.
const NO_READING_TITLE: &str = "No reading";

/// Size of the big meter's hint line relative to the notice title above it.
const NOTICE_HINT_SIZE_RATIO: f32 = 0.6;

/// Size of the mode line relative to the reading value above or beside it.
const MODE_SIZE_RATIO: f32 = 0.4;

/// Largest font a readout selector's popup entries use. The readout itself
/// follows the reading — a 200 px big-meter reading puts it at 80 px — but
/// the popup is a list to pick from, and stays list-sized.
const MAX_CHOICE_POPUP_FONT_SIZE: f32 = 18.0;

/// Marks the live entry in a readout selector's popup, in text as well as in
/// the selection colour.
pub(crate) const LIVE_CHOICE_MARK: &str = "\u{25CF}";

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
        | Flag::Avg
        | Flag::PeakMax
        | Flag::PeakMin
        | Flag::Comp
        | Flag::Record
        | Flag::LoZ
        | Flag::Dc => tc.accent(),
    }
}

/// Screen rects of one sub-value row: (label, value+unit).
///
/// Returned by [`show_aux_rows`] so a test can assert the two are on the same
/// baseline. Production callers drop it — the rows are laid out by the grid,
/// not by their caller.
type AuxRowRects = (Rect, Rect);

/// One sub-value row's cells: label, value with its unit, and the MIN/MAX
/// timestamp. Built in one place for drawing and for measuring, so the fit's
/// idea of the grid's size can't drift from the grid drawn.
struct AuxCells {
    label: RichText,
    value: LayoutJob,
    secs: Option<RichText>,
}

fn aux_cells(ui: &Ui, m: &Measurement, aux: &AuxValue, size: f32, tc: &ThemeColors) -> AuxCells {
    let label = RichText::new(&*aux.label)
        .font(FontId::proportional(size))
        .color(ui.visuals().weak_text_color());
    // Overload in the error color, as the main value is — and the text
    // still reads "OL", so the state is never signalled by color alone.
    let value_color = match aux.value {
        MeasuredValue::Overload => tc.status_error(),
        _ => tc.reading(),
    };
    // Value and unit are one label, not a nested `ui.horizontal`. A
    // horizontal scope allocates its child `Ui` at `interact_size.y` (~18 px)
    // and then expands downwards, so taller content lands half a line below
    // the grid row it belongs to: 12 px out at the side panel's 36 px, 66 px
    // out at the big meter's 130 px. With every cell a plain label, the
    // grid's own `LEFT_CENTER` alignment does the work.
    let mut value = LayoutJob::default();
    value.append(
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
        value.append(
            unit,
            2.0,
            TextFormat {
                font_id: FontId::monospace(size),
                color: tc.reading(),
                ..Default::default()
            },
        );
    }
    let secs = aux.elapsed_secs.map(|secs| {
        RichText::new(format!("@{secs}s"))
            .font(FontId::proportional(size))
            .color(ui.visuals().weak_text_color())
    });
    AuxCells { label, value, secs }
}

/// The sub-value grid's font size for a mode line at `font_size`: the rows
/// are secondary information and should not compete with the main value, but
/// they still have to stay readable.
fn aux_font_size(font_size: f32) -> f32 {
    font_size.max(MIN_AUX_FONT_SIZE)
}

/// Gap between the sub-value grid's columns.
fn aux_column_gap(size: f32) -> f32 {
    (size * 0.5).max(4.0)
}

/// Gap between the sub-value grid's rows.
const AUX_ROW_GAP: f32 = 2.0;

/// The sub-value grid's text, laid out at one font size: the widest cell of
/// each column and the tallest cell. The big meter's fit scales these to
/// whatever size it is weighing rather than laying text out at every
/// candidate size, which would fill the font atlas with sizes never drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AuxGridMetrics {
    /// The size the cells were laid out at, floored as the grid floors it.
    font_size: f32,
    label_w: f32,
    value_w: f32,
    /// Zero when no row carries a MIN/MAX timestamp: the grid then has two
    /// columns, not three.
    secs_w: f32,
    text_h: f32,
    rows: usize,
}

impl AuxGridMetrics {
    /// Lay the cells out with a mode line at `font_size`; `None` when the
    /// measurement has no sub-values and the grid draws nothing.
    fn measure(ui: &Ui, m: &Measurement, font_size: f32, tc: &ThemeColors) -> Option<Self> {
        if m.aux_values.is_empty() {
            return None;
        }
        let size = aux_font_size(font_size);
        let galley = |text: WidgetText| {
            text.into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextStyle::Body,
            )
            .size()
        };
        let mut metrics = Self {
            font_size: size,
            label_w: 0.0,
            value_w: 0.0,
            secs_w: 0.0,
            text_h: 0.0,
            rows: m.aux_values.len(),
        };
        for aux in &m.aux_values {
            let cells = aux_cells(ui, m, aux, size, tc);
            let label = galley(cells.label.into());
            let value = galley(cells.value.into());
            let secs = cells.secs.map_or(Vec2::ZERO, |s| galley(s.into()));
            metrics.label_w = metrics.label_w.max(label.x);
            metrics.value_w = metrics.value_w.max(value.x);
            metrics.secs_w = metrics.secs_w.max(secs.x);
            metrics.text_h = metrics.text_h.max(label.y).max(value.y).max(secs.y);
        }
        Some(metrics)
    }

    /// The grid's size with a mode line at `font_size`, as `show_aux_rows`
    /// lays it out: each cell at least `min_cell` (egui's `interact_size`,
    /// the grid's default floor), columns and rows apart by the grid's gaps.
    fn size(&self, font_size: f32, min_cell: Vec2) -> Vec2 {
        let size = aux_font_size(font_size);
        let k = size / self.font_size;
        let col = |w: f32| (w * k).max(min_cell.x);
        let (mut width, mut columns) = (col(self.label_w) + col(self.value_w), 2.0);
        if self.secs_w > 0.0 {
            width += col(self.secs_w);
            columns += 1.0;
        }
        width += (columns - 1.0) * aux_column_gap(size);
        let row = (self.text_h * k).max(min_cell.y);
        let rows = self.rows as f32;
        vec2(width, rows * row + (rows - 1.0) * AUX_ROW_GAP)
    }
}

/// Render one row per sub-value: beneath the primary reading, or in its own
/// column beside it.
///
/// Draws nothing at all when the measurement has none, so single-display
/// meters keep the layout they had before sub-values existed.
///
/// `font_size` is the caller's mode-line size, floored at
/// [`MIN_AUX_FONT_SIZE`]. `id` is the grid's scope, used as it is rather
/// than mixed with the id of the `ui` it is drawn in: the beside layout nests
/// the grid deeper than the others do, and a grid under a new id spends a
/// frame sizing itself unseen, then keeps column widths from another size.
///
/// Returns one [`AuxRowRects`] per row, which production callers drop — it
/// exists so a test can assert the label and its value stay on one line.
fn show_aux_rows(
    ui: &mut Ui,
    id: Id,
    m: &Measurement,
    font_size: f32,
    tc: &ThemeColors,
) -> Vec<AuxRowRects> {
    if m.aux_values.is_empty() {
        return Vec::new();
    }
    let size = aux_font_size(font_size);
    let mut rects: Vec<AuxRowRects> = Vec::with_capacity(m.aux_values.len());
    // A grid rather than a stack of horizontal rows so labels, digits and
    // timestamps line up in columns however long the individual strings are.
    ui.scope_builder(UiBuilder::new().id(id), |ui| {
        Grid::new("aux_rows")
            .num_columns(3)
            .spacing([aux_column_gap(size), AUX_ROW_GAP])
            .show(ui, |ui| {
                for aux in &m.aux_values {
                    let cells = aux_cells(ui, m, aux, size, tc);
                    let label = ui.label(cells.label);
                    let value = ui.label(cells.value);
                    rects.push((label.rect, value.rect));
                    if let Some(secs) = cells.secs {
                        ui.label(secs);
                    }
                    ui.end_row();
                }
            });
    });
    rects
}

/// The id every layout gives the sub-value grid drawn into `ui`.
fn aux_grid_id(ui: &Ui) -> Id {
    ui.id().with("aux_rows")
}

/// The reading's digits, led by its own name when it has one ("DC" beside an
/// "AC" row): muted and at `caption_size`, the sub-value labels' size, so it
/// reads as the first of those labels rather than as part of the value. Every
/// meter but the UT61E+ in AC+DC V gets the digits alone.
///
/// One galley rather than two labels: a horizontal row centres each widget on
/// the height it has reached so far, so a small label placed before the
/// digits sat at the top of the row. Within one job each section is centred
/// on the line instead.
fn value_label(
    ui: &Ui,
    m: &Measurement,
    value_text: &str,
    font: FontId,
    color: Color32,
    caption_size: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut gap = 0.0;
    // Not over blank digits: a frame shown without its main reading (the
    // first after a connect, or HOLD on the AC component) has no DC to name.
    if let Some(label) = m.main_label.filter(|_| m.has_main_reading()) {
        let size = caption_size.max(MIN_AUX_FONT_SIZE);
        job.append(
            label.as_str(),
            0.0,
            TextFormat {
                font_id: FontId::proportional(size),
                color: ui.visuals().weak_text_color(),
                valign: eframe::egui::Align::Center,
                ..Default::default()
            },
        );
        // The gap the sub-value grid leaves between its label and value
        // columns.
        gap = (size * 0.5).max(4.0);
    }
    job.append(
        value_text,
        gap,
        TextFormat {
            font_id: font,
            color,
            valign: eframe::egui::Align::Center,
            ..Default::default()
        },
    );
    job
}

/// Prepare the value text and color from a measurement.
fn value_display(m: &Measurement, tc: &ThemeColors) -> (String, Color32) {
    match &m.value {
        MeasuredValue::Normal(_) => (format_value_display(m), tc.reading()),
        MeasuredValue::Overload => (format_value_display(m), tc.status_error()),
        MeasuredValue::NcvLevel(_) | MeasuredValue::NoReading(_) | MeasuredValue::Absent => {
            (format_value_display(m), tc.reading())
        }
    }
}

/// The lists the two readout dropdowns offer, as the acquisition thread last
/// reported them. Passed as one value so the reading widgets keep a short
/// signature as further settings join them.
#[derive(Clone, Copy, Default)]
pub struct ReadoutChoices<'a> {
    /// Modes reachable from the current dial position.
    pub mode: &'a [Choice],
    /// `Auto` plus the rungs of the current mode's ladder.
    pub range: &'a [Choice],
    /// The meter's function keys, from its profile: the mode readout lists
    /// them when `mode` has nothing to pick.
    pub keys: &'a [MeterKey],
}

impl ReadoutChoices<'_> {
    /// Whether either readout has something to pick. The single-line layouts
    /// draw plain labels inside the live region when neither does, and the
    /// selector row when one does.
    fn any_offered(&self) -> bool {
        self.mode_offered() || mode_switch_offered(self.range)
    }

    /// Whether the mode readout is a dropdown: of modes, or of keys.
    pub(crate) fn mode_offered(&self) -> bool {
        mode_switch_offered(self.mode) || !self.keys.is_empty()
    }
}

/// What a pick in a readout dropdown asks of the meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadoutPick {
    /// Switch a setting to one of the ids `Dmm::choices` listed.
    Select(Setting, u16),
    /// Press one of the meter's keys, by its command.
    Press(&'static str),
}

/// Whether a readout is a selector rather than a plain label.
///
/// A single choice is the live mode on its own — the single-variant UT181A
/// dials (Ohm, nS, Cap, Hz, Duty, Pulse Width) report exactly that — so it
/// means what an empty list means: nothing to pick, and no control to draw.
pub(crate) fn mode_switch_offered(choices: &[Choice]) -> bool {
    choices.len() > 1
}

/// One readout dropdown's wording: the widget id it keys its popup and focus
/// state under — two dropdowns on the same line must not share one — the
/// hover text, the name a screen reader gives it, and the caption over its
/// list, if any.
struct Dropdown {
    id_salt: &'static str,
    hover: &'static str,
    a11y: &'static str,
    heading: Option<&'static str>,
}

/// A dropdown over one setting's choices, and the setting a pick switches.
struct ChoiceReadout {
    setting: Setting,
    dropdown: Dropdown,
}

/// The mode readout: which mode of the current dial position the meter is in.
const MODE_READOUT: ChoiceReadout = ChoiceReadout {
    setting: Setting::Mode,
    dropdown: Dropdown {
        id_salt: "mode_select",
        hover: "Switch the meter to another mode of the current dial position",
        a11y: "Mode",
        heading: None,
    },
};

/// The range readout beside it: which rung of the current mode's ladder the
/// meter is on, or `Auto` while it picks the rung itself.
const RANGE_READOUT: ChoiceReadout = ChoiceReadout {
    setting: Setting::Range,
    dropdown: Dropdown {
        id_salt: "range_select",
        hover: "Switch the meter to another range of the current mode",
        a11y: "Range",
        heading: None,
    },
};

/// The mode readout on a meter that lists function keys instead of modes.
/// A screen reader still calls it "Mode"; the caption says the entries are
/// key presses, not modes the meter is sure to land on.
const KEYS_READOUT: Dropdown = Dropdown {
    id_salt: "mode_keys",
    hover: "Press one of the meter's function keys",
    a11y: "Mode",
    heading: Some("Meter keys"),
};

/// The mode readout: a dropdown of the modes the meter can be switched to,
/// else of its function keys, else the plain label.
///
/// In the key list the marked entry is the key whose function the reading
/// shows, and every pick is a press — the marked one too, as a key can
/// cycle within its function (diode and continuity, °C and °F).
fn show_mode_readout(
    ui: &mut Ui,
    m: &Measurement,
    size: f32,
    choices: ReadoutChoices<'_>,
) -> Option<ReadoutPick> {
    if mode_switch_offered(choices.mode) || choices.keys.is_empty() {
        return show_choice_readout(ui, &MODE_READOUT, &m.mode, size, choices.mode);
    }
    let entries: Vec<(&str, bool)> = choices
        .keys
        .iter()
        .map(|k| (k.label, (k.applies)(m)))
        .collect();
    show_dropdown(ui, &KEYS_READOUT, &m.mode, size, &entries)
        .map(|i| ReadoutPick::Press(choices.keys[i].command))
}

/// What the range readout leaves behind on a meter with no rung to pick.
///
/// The two-line layout has always drawn the range as a plain label beside the
/// mode, so it keeps it; the single-line layouts never drew one, and a meter
/// that cannot switch ranges must not gain text there.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeAtRest {
    Label,
    Nothing,
}

/// The range readout beside the mode one.
///
/// The closed text is always the live label the meter reports, so it reads
/// the same whether the meter picked the rung or the user did; the marked
/// entry is `Auto` while the meter is auto-ranging and the live rung
/// otherwise. `at_rest` decides what a meter with nothing to pick gets.
fn show_range_readout(
    ui: &mut Ui,
    m: &Measurement,
    size: f32,
    choices: &[Choice],
    at_rest: RangeAtRest,
) -> Option<ReadoutPick> {
    // A meter that reports no range at all has nothing to label, in any
    // layout — the UT61E+ temperature and NCV positions, for two.
    if m.range_label.is_empty() {
        return None;
    }
    if at_rest == RangeAtRest::Nothing && !mode_switch_offered(choices) {
        return None;
    }
    show_choice_readout(ui, &RANGE_READOUT, &m.range_label, size, choices)
}

/// A readout under the reading: a plain label, or — when the meter lists more
/// than one value the setting can take from where it sits — a dropdown
/// listing them, with the live one marked.
///
/// Returns the setting and the id of a value the user picked that differs
/// from the live one.
fn show_choice_readout(
    ui: &mut Ui,
    readout: &ChoiceReadout,
    label: &str,
    size: f32,
    choices: &[Choice],
) -> Option<ReadoutPick> {
    if !mode_switch_offered(choices) {
        ui.label(
            RichText::new(label)
                .font(FontId::proportional(size))
                .color(ui.visuals().weak_text_color()),
        );
        return None;
    }
    let entries: Vec<(&str, bool)> = choices.iter().map(|c| (&*c.label, c.current)).collect();
    let picked = &choices[show_dropdown(ui, &readout.dropdown, label, size, &entries)?];
    (!picked.current).then_some(ReadoutPick::Select(readout.setting, picked.id))
}

/// A readout drawn as a dropdown over `entries`, each a label and whether it
/// is the live one, which the list marks. Returns the index of the entry
/// picked this frame, the live one included.
///
/// The dropdown is drawn at the label's size and with no frame at rest, so
/// it reads as the readout it replaces; hover and the open state keep egui's
/// own highlight so it still answers as a control. Its popup is capped at
/// [`MAX_CHOICE_POPUP_FONT_SIZE`], under the dropdown's caption if it has one.
///
/// The open list behaves as a native listbox through [`listbox_dropdown`]:
/// focus lands on the live entry as it opens (the first when none is live),
/// Up/Down (Home/End) move it, Enter/Space or a click picks, Esc or Tab
/// closes without a pick, and focus returns to the readout on every close.
fn show_dropdown(
    ui: &mut Ui,
    dropdown: &Dropdown,
    label: &str,
    size: f32,
    entries: &[(&str, bool)],
) -> Option<usize> {
    let focus_on_open = entries.iter().position(|&(_, live)| live).unwrap_or(0);
    let popup_size = size.clamp(MIN_AUX_FONT_SIZE, MAX_CHOICE_POPUP_FONT_SIZE);
    let ctx = ui.ctx().clone();
    let row_height = ctx.fonts_mut(|f| f.row_height(&FontId::proportional(size)));
    let (response, picked) = ui
        .scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            widgets.inactive.bg_stroke = Stroke::NONE;
            // egui draws its arrow to `icon_width`, a fixed 14 px that is
            // lost beside a big-meter readout; follow the text instead.
            let spacing = ui.spacing_mut();
            spacing.icon_width = (size * 0.8).max(8.0);
            spacing.icon_spacing = (size * 0.25).max(2.0);
            // `ComboBox::show_ui` lays the box out in a nested `ui.horizontal`,
            // whose row starts `interact_size.y` (~18 px) tall and pushes
            // taller content down rather than over the row above (the quirk
            // noted on the sub-value grid). Start that row at the box's own
            // height — egui's text-or-icon height plus the button padding —
            // so a big-meter readout stays centred beside its label.
            let box_height = row_height.max(spacing.icon_width) + 2.0 * spacing.button_padding.y;
            spacing.interact_size.y = spacing.interact_size.y.max(box_height);
            // The interactive colour, not the label's weak one: this is a
            // control, and it has to clear the text contrast bar as one.
            let text_color = ui.visuals().text_color();
            let combo = |combo: ComboBox| {
                combo
                    .width(0.0)
                    // egui's default cap scrolls a list of ten keys; let it
                    // grow to the window, scrolling only when the window is
                    // shorter.
                    .height(ctx.content_rect().height())
                    .selected_text(
                        RichText::new(label)
                            .font(FontId::proportional(size))
                            .color(text_color),
                    )
            };
            listbox_dropdown(
                ui,
                dropdown.id_salt,
                || 0,
                combo,
                |ui, was_open| {
                    if let Some(heading) = dropdown.heading {
                        ui.label(
                            RichText::new(heading)
                                .font(FontId::proportional(popup_size))
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                    let mut picked = None;
                    let mut responses = Vec::with_capacity(entries.len());
                    for (i, &(text, live)) in entries.iter().enumerate() {
                        let entry = ui.selectable_label(
                            live,
                            RichText::new(choice_entry_text(text, live))
                                .font(FontId::proportional(popup_size)),
                        );
                        // Focus lands on the live entry as the list opens — by
                        // click, or by Enter/Space on the readout — so a screen
                        // reader announces it and Enter picks it.
                        if !was_open && i == focus_on_open {
                            entry.request_focus();
                        }
                        if entry.clicked() {
                            picked = Some(i);
                        }
                        responses.push(entry);
                    }
                    navigate_choice_entries(ui.ctx(), &responses);
                    picked
                },
            )
        })
        .inner;
    response
        .on_hover_text(dropdown.hover)
        .a11y_label(dropdown.a11y);
    picked
}

/// An entry of an open choice list: `text`, marked when it is the live one.
pub(crate) fn choice_entry_text(text: &str, live: bool) -> String {
    // Three spaces sit close enough under the mark to keep the entries
    // aligned without a figure space the bundled fonts may not have.
    if live {
        format!("{LIVE_CHOICE_MARK} {text}")
    } else {
        format!("   {text}")
    }
}

/// A `ComboBox` whose open list behaves as a native listbox, for every
/// dropdown that lists choices: the readouts' and the Settings device list.
///
/// `combo` styles the box (`selected_text`, width, height); `add_entries`
/// draws the list and returns what was picked this frame. It is handed
/// whether the list was already open last frame, false on the frame it
/// opens, which is when it moves focus onto an entry; arrow keys between
/// entries are its own (`navigate_choice_entries`).
///
/// `layout` names the list's shape, such as its column count, and is asked
/// only while the list is open: egui sizes a popup on the frame it opens and
/// only ever widens it after, so when the shape changes under an open list,
/// the list closes for a frame and opens again at its new size.
///
/// This owns the rest. Esc and Tab close the list without a pick, a pick
/// closes it, and focus returns to the box on every close but a click
/// elsewhere. The mechanisms are the ones `color_edit` uses for its picker:
/// a was-open flag to see the open and close transitions, consumed keys plus
/// a cancelled focus move, and a focus lock filter on the entry.
pub(crate) fn listbox_dropdown<R>(
    ui: &mut Ui,
    id_salt: &'static str,
    layout: impl Fn() -> usize,
    combo: impl FnOnce(ComboBox) -> ComboBox,
    add_entries: impl FnOnce(&mut Ui, bool) -> Option<R>,
) -> (Response, Option<R>) {
    let ctx = ui.ctx().clone();
    // The id `ComboBox::from_id_salt` derives below, known up front so the
    // list's state can be read before the box is drawn. The salt is wrapped
    // in `IdSalt::new` the way `from_id_salt` wraps it: hashing the bare
    // string, or an `Id`, gives a different id.
    let button_id = ui.make_persistent_id(IdSalt::new(id_salt));
    let was_open_key = button_id.with("was_open");
    let was_open: bool = ctx.data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    // The popup's id as `ComboBox` derives it from the box's (its private
    // `widget_to_popup_id`, behind `ComboBox::is_open`).
    let popup_id = button_id.with("popup");
    let layout_key = button_id.with("layout");
    let reopen_key = button_id.with("reopen");
    if ctx.data_mut(|d| d.remove_temp::<bool>(reopen_key)) == Some(true) {
        Popup::open_id(&ctx, popup_id);
    }
    // Keys that leave the list, handled before it is drawn so this frame
    // already shows it closed: Tab and Shift+Tab step out of a listbox
    // rather than through it, Esc abandons it. `consume_key` drops the press;
    // `move_focus(None)` cancels the focus jump egui queued from it as the
    // frame began.
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
        // A list left this frame stays closed, whatever its shape did.
        let shape = layout();
        let last: Option<usize> = ctx.data(|d| d.get_temp(layout_key));
        ctx.data_mut(|d| d.insert_temp(layout_key, shape));
        if !leave && last.is_some_and(|last| last != shape) {
            Popup::close_id(&ctx, popup_id);
            ctx.data_mut(|d| d.insert_temp(reopen_key, true));
        }
    }

    let mut picked = None;
    // No `set_modal_layer` here, unlike `color_edit`: Tab and the arrows are
    // handled outright, and the modal layer outlives the list by a frame, in
    // which egui surrenders the focus just handed back to the box
    // (`Context::create_widget` on a layer below the modal).
    let inner =
        combo(ComboBox::from_id_salt(id_salt)).show_ui(ui, |ui| picked = add_entries(ui, was_open));

    // Enter/Space "clicks" the focused entry without a pointer click, which
    // is the only thing a menu popup closes on by itself.
    let activated = picked.is_some();
    if activated {
        Popup::close_all(&ctx);
    }
    let is_open = ComboBox::is_open(&ctx, button_id);
    if is_open && !was_open {
        // The shape it opened at, for the next frame to compare against.
        ctx.data_mut(|d| d.insert_temp(layout_key, layout()));
    }
    // A click outside the list that closed it may have landed on another
    // widget, which took the focus as it was drawn; that click is the user's
    // choice of focus. A click on an entry is `activated` and not
    // "elsewhere" in this sense.
    let clicked_away = inner.response.clicked_elsewhere() && !activated;
    if was_open && !is_open && !clicked_away {
        // Closed by pick, Esc or Tab: focus goes back to the box rather than
        // to the top of the Tab order.
        ctx.memory_mut(|m| m.request_focus(button_id));
    }
    ctx.data_mut(|d| d.insert_temp(was_open_key, is_open));
    (inner.response, picked)
}

/// Keyboard navigation inside an open readout list: ArrowDown/ArrowUp move
/// focus between the entries, clamped at the ends; Home/End jump to the
/// first and last. Nothing reaches the meter until Enter, Space or a click
/// picks the focused entry.
pub(crate) fn navigate_choice_entries(ctx: &Context, entries: &[Response]) {
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

/// Single-line reading with the mode and range selectors beside it.
///
/// The value and unit form the live region; the selectors, controls, sit
/// after it in the same row rather than inside it — a screen reader would
/// otherwise have the dropdowns re-announced with every reading update.
/// `draw_value` paints the value and unit labels.
fn show_reading_line_with_selector(
    ui: &mut Ui,
    m: &Measurement,
    scaled: bool,
    draw_value: impl FnOnce(&mut Ui),
    mode_size: f32,
    choices: ReadoutChoices<'_>,
    tc: &ThemeColors,
) -> Option<ReadoutPick> {
    ui.horizontal(|ui| {
        ui.live_region_horizontal(
            live_region_fingerprint(Some(m), scaled, NO_READING_TITLE),
            || live_region_label(Some(m), scaled, NO_READING_TITLE),
            |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                draw_value(ui);
            },
        );
        ui.separator();
        let mode = show_mode_readout(ui, m, mode_size, choices);
        // This line has never carried a range label, so a meter with no rung
        // to pick keeps the line it has: mode, then the badges.
        let range = show_range_readout(ui, m, mode_size, choices.range, RangeAtRest::Nothing);
        show_flags(ui, m, mode_size, tc, scaled);
        mode.or(range)
    })
    .inner
}

/// Single-line reading with no selector on it: value, unit, mode and flags,
/// all inside the live region.
///
/// `mode_size` of 0 is the small text style, the convention `show_flags` uses
/// for the same choice.
fn show_reading_line_plain(
    ui: &mut Ui,
    m: &Measurement,
    scaled: bool,
    draw_value: impl FnOnce(&mut Ui),
    mode_size: f32,
    tc: &ThemeColors,
) {
    ui.live_region_horizontal(
        live_region_fingerprint(Some(m), scaled, NO_READING_TITLE),
        || live_region_label(Some(m), scaled, NO_READING_TITLE),
        |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            draw_value(ui);
            ui.separator();
            let mut mode = RichText::new(&*m.mode).color(ui.visuals().weak_text_color());
            if mode_size > 0.0 {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.3).max(2.0);
                mode = mode.font(FontId::proportional(mode_size));
            } else {
                mode = mode.small();
            }
            ui.label(mode);
            show_flags(ui, m, mode_size, tc, scaled);
        },
    );
}

/// What stands in for a reading when there is none.
///
/// [`NoReadingText::Plain`] is the weak "No reading" every normal layout has
/// always drawn. [`NoReadingText::Notice`] is the big meter's form: with the
/// reading scaled to fill the window there is no room under it for the
/// connection help, so the issue's title takes the readout's place, a hint
/// line says how to get the steps back, and the steps themselves ride along
/// as hover text.
#[derive(Clone, Copy)]
pub(crate) enum NoReadingText<'a> {
    Plain,
    Notice {
        /// One line naming the problem, in place of "No reading".
        title: &'a str,
        /// Weak line under the title — how to reach the full help.
        hint: &'a str,
        /// The help body, which only fits as a tooltip here.
        tooltip: &'a str,
        /// Title colour; the warning colour, not the weak text colour.
        color: Color32,
    },
}

impl<'a> NoReadingText<'a> {
    /// What a screen reader hears in place of a reading.
    fn title(&self) -> &'a str {
        match self {
            Self::Plain => NO_READING_TITLE,
            Self::Notice { title, .. } => title,
        }
    }

    fn is_notice(&self) -> bool {
        matches!(self, Self::Notice { .. })
    }

    /// The notice's title with the hint under it, as one two-line galley —
    /// `None` for the plain placeholder.
    ///
    /// One galley rather than a nested `ui.vertical`: a child layout claims
    /// the top of the row it is placed in, so beside a reading-sized row of
    /// dashes the two lines would sit level with the top of that row instead
    /// of centred against it. `title_size` is the reading's mode size, so
    /// the notice scales with the meter exactly as the mode line would —
    /// floored, because in a tiny window it is the only thing left to read.
    fn notice_block(&self, ui: &Ui, title_size: f32) -> Option<LayoutJob> {
        let Self::Notice {
            title, hint, color, ..
        } = self
        else {
            return None;
        };
        let title_size = title_size.max(MIN_AUX_FONT_SIZE);
        let hint_format = TextFormat {
            font_id: FontId::proportional(
                (title_size * NOTICE_HINT_SIZE_RATIO).max(MIN_AUX_FONT_SIZE),
            ),
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        };
        let mut block = LayoutJob::default();
        block.append(
            title,
            0.0,
            TextFormat {
                font_id: FontId::proportional(title_size),
                color: *color,
                ..Default::default()
            },
        );
        block.append("\n", 0.0, hint_format.clone());
        block.append(hint, 0.0, hint_format);
        Some(block)
    }

    /// Both big-meter layouts' dimensions per point of reading font, worked
    /// out from the galleys rather than read back from a drawn frame.
    ///
    /// The fit re-measures only the layout it drew, so the other one keeps
    /// whatever ratio the cache holds — for a placeholder that is the default
    /// tuned to a seven-character value, twice the width of three dashes and
    /// a title. Compared against that, the inline row won a 3:2 window it had
    /// no business winning, and once picked it was the only one ever
    /// measured again. Laying both out here costs two galleys and removes
    /// the guess. Measured at the base size: the floors under the title and
    /// hint make the ratio drift slightly at tiny sizes, where the fit is at
    /// its own floor anyway.
    fn layout_ratios(&self, ui: &Ui) -> Option<ReadingRatios> {
        let base = BASE_READING_FONT_SIZE;
        let block = self.notice_block(ui, base * MODE_SIZE_RATIO)?;
        let painter = ui.painter();
        let bars = painter
            .layout_no_wrap(
                crate::NO_DATA.to_string(),
                FontId::monospace(base),
                Color32::PLACEHOLDER,
            )
            .size();
        let block = painter.layout_job(block).size();
        let spacing = ui.spacing().item_spacing;
        Some(ReadingRatios {
            w: bars.x.max(block.x) / base,
            h: (bars.y + spacing.y + block.y) / base,
            row_w: (bars.x + spacing.x + block.x) / base,
            row_h: bars.y.max(block.y) / base,
        })
    }

    /// Draw the title, and for a notice the hint under it, as one block whose
    /// hover text is the help body.
    ///
    /// `title_size` is the reading's mode size, so the notice scales with the
    /// meter exactly as the mode line would — floored, because in a tiny
    /// window it is the only thing left to read. The plain placeholder keeps
    /// the body-sized weak label the normal layouts have always shown.
    fn show_text(&self, ui: &mut Ui, title_size: f32) {
        let Some(block) = self.notice_block(ui, title_size) else {
            ui.label(RichText::new(NO_READING_TITLE).color(ui.visuals().weak_text_color()));
            return;
        };
        let Self::Notice { tooltip, .. } = self else {
            return;
        };
        let block = ui.label(block);
        // Detection, the one notice with no steps to give, has an empty body;
        // an empty tooltip would still pop a box up under the pointer.
        if !tooltip.is_empty() {
            block.on_hover_text(*tooltip);
        }
    }
}

/// The dashes that stand in for a value.
fn no_reading_bars(ui: &mut Ui, value_size: f32) {
    ui.label(
        RichText::new(crate::NO_DATA)
            .font(FontId::monospace(value_size))
            .color(ui.visuals().weak_text_color()),
    );
}

/// What each layout shows in place of a reading.
///
/// Wrap the placeholder + caption in a horizontal scope so the live-region
/// label is attached to the scope id rather than to the inner `ui.label()`
/// Response. egui maps Role::Label overrides to set_value, not set_label, so
/// attaching directly to the label would silently drop the live-region label.
fn no_reading_placeholder(
    ui: &mut Ui,
    scaled: bool,
    title: &str,
    add_contents: impl FnOnce(&mut Ui),
) {
    // Trailing dots are animation, not speech: the waiting notice cycles
    // through one to four of them, and a fingerprint that hashed them would
    // have a screen reader repeat the same sentence every timeout.
    let spoken = title.trim_end_matches(['.', ' ']);
    ui.live_region_horizontal(
        live_region_fingerprint(None, scaled, spoken),
        || live_region_label(None, scaled, spoken),
        add_contents,
    );
}

/// Render the primary reading display at the given font size (two-line layout).
///
/// Returns the setting and value the user picked from a selector, if any.
fn show_reading_sized(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
    no_reading: NoReadingText<'_>,
) -> Option<ReadoutPick> {
    let unit_size = value_size;
    let mode_size = value_size * MODE_SIZE_RATIO;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(m, tc);

            ui.live_region_horizontal(
                live_region_fingerprint(Some(m), scaled, NO_READING_TITLE),
                || live_region_label(Some(m), scaled, NO_READING_TITLE),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(value_label(
                        ui,
                        m,
                        &value_text,
                        FontId::monospace(value_size),
                        value_color,
                        mode_size,
                    ));
                    ui.label(
                        RichText::new(&*m.unit)
                            .font(FontId::monospace(unit_size))
                            .color(tc.reading()),
                    );
                },
            );

            let _ = show_aux_rows(ui, aux_grid_id(ui), m, mode_size, tc);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.5).max(2.0);
                let mode = show_mode_readout(ui, m, mode_size, choices);
                // The range has always been a label on this line, so it stays
                // one on a meter that lists no rung to pick.
                let range = show_range_readout(ui, m, mode_size, choices.range, RangeAtRest::Label);
                show_flags(ui, m, mode_size, tc, scaled);
                mode.or(range)
            })
            .inner
        }
        None => {
            no_reading_placeholder(ui, scaled, no_reading.title(), |ui| {
                if no_reading.is_notice() {
                    // A notice stacks under the bars, the shape this layout
                    // already gives a reading and its mode line: an issue
                    // title set beside a scaled-up row of dashes would push
                    // the fitted font down to nothing.
                    ui.vertical(|ui| {
                        no_reading_bars(ui, value_size);
                        no_reading.show_text(ui, mode_size);
                    });
                } else {
                    no_reading_bars(ui, value_size);
                    no_reading.show_text(ui, mode_size);
                }
            });
            None
        }
    }
}

/// The value and its unit, as the one-row layouts draw them.
fn show_value_and_unit(ui: &mut Ui, m: &Measurement, value_size: f32, tc: &ThemeColors) {
    let (value_text, value_color) = value_display(m, tc);
    ui.label(value_label(
        ui,
        m,
        &value_text,
        FontId::monospace(value_size),
        value_color,
        value_size * MODE_SIZE_RATIO,
    ));
    ui.label(
        RichText::new(&*m.unit)
            .font(FontId::monospace(value_size))
            .color(tc.reading()),
    );
}

/// Space between the blocks of a one-row reading — value, separators,
/// sub-values, mode — for a mode line at `mode_size`.
fn one_row_spacing(mode_size: f32) -> f32 {
    (mode_size * 0.3).max(2.0)
}

/// Space either side of the divider after the value, for a mode line at
/// `mode_size`: the selector line spaces its blocks as every one-row reading
/// does, the plain line keeps the value's own 2 pt. The beside layout's
/// second divider takes the same, so the two one-row layouts differ by the
/// grid and one divider only, and either can measure the line for both.
fn divider_gap(mode_size: f32, selectors: bool) -> f32 {
    if selectors {
        one_row_spacing(mode_size)
    } else {
        2.0
    }
}

/// Width egui gives a separator across its line.
const SEPARATOR_WIDTH: f32 = 6.0;

/// Render the reading with value and mode on a single line and the
/// sub-values in rows under it (below layout).
///
/// Returns the setting and value the user picked from a selector, if any,
/// and the size of the line alone, the space under it included — what the
/// fit measures the one-row layouts by.
fn show_reading_inline(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
    no_reading: NoReadingText<'_>,
) -> (Option<ReadoutPick>, Vec2) {
    let mode_size = value_size * MODE_SIZE_RATIO;
    let grid_id = aux_grid_id(ui);
    let top = ui.cursor().top();
    let line = |ui: &Ui| vec2(ui.min_rect().width(), ui.cursor().top() - top);

    match measurement {
        Some(m) => {
            let draw_value = |ui: &mut Ui| show_value_and_unit(ui, m, value_size, tc);
            let picked = if !choices.any_offered() {
                show_reading_line_plain(ui, m, scaled, draw_value, mode_size, tc);
                None
            } else {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = one_row_spacing(mode_size);
                    show_reading_line_with_selector(
                        ui, m, scaled, draw_value, mode_size, choices, tc,
                    )
                })
                .inner
            };
            let line = line(ui);
            let _ = show_aux_rows(ui, grid_id, m, mode_size, tc);
            (picked, line)
        }
        None => {
            no_reading_placeholder(ui, scaled, no_reading.title(), |ui| {
                if no_reading.is_notice() {
                    no_reading_bars(ui, value_size);
                    no_reading.show_text(ui, mode_size);
                } else {
                    ui.label(
                        RichText::new(format!("{} {NO_READING_TITLE}", crate::NO_DATA))
                            .font(FontId::monospace(value_size))
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            });
            (None, line(ui))
        }
    }
}

/// Render the reading, its sub-values and the mode line on one row, each a
/// column with a divider between (beside layout): the sub-values are
/// readings, so they sit next to the reading, and the mode, a control, comes
/// last.
///
/// The value and the grid are placed at heights known up front, so the row
/// centres them on each other whichever is the taller — four UT181A rows
/// outgrow the value. A nested layout placed at its natural size would start
/// `interact_size.y` tall and grow downwards from the top of the row.
///
/// Returns the pick, as [`show_reading`] does, and the size of the line
/// as the below layout would draw it — the row without the grid, its second
/// separator and their spacing, and with the space under it — so either
/// one-row layout measures the line for both.
fn show_reading_beside(
    ui: &mut Ui,
    m: &Measurement,
    value_size: f32,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
) -> (Option<ReadoutPick>, Vec2) {
    let mode_size = value_size * MODE_SIZE_RATIO;
    let grid_id = aux_grid_id(ui);
    let grid = AuxGridMetrics::measure(ui, m, mode_size, tc).map_or(Vec2::ZERO, |g| {
        g.size(mode_size, ui.spacing().interact_size)
    });
    let value_h = ui.fonts_mut(|f| f.row_height(&FontId::monospace(value_size)));
    let spacing_y = ui.spacing().item_spacing.y;
    let gap = divider_gap(mode_size, choices.any_offered());
    let (picked, line_h) = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.set_min_height(value_h.max(grid.y));
            let value = ui
                .allocate_ui_with_layout(
                    vec2(ui.available_width(), value_h),
                    // Top, not centre: the live region is a nested horizontal,
                    // which starts `interact_size.y` tall where the layout puts
                    // it and grows downwards — centred, it would hang half the
                    // value's height low. From the top it fills the slot exactly.
                    Layout::left_to_right(Align::Min),
                    |ui| {
                        ui.live_region_horizontal(
                            live_region_fingerprint(Some(m), scaled, NO_READING_TITLE),
                            || live_region_label(Some(m), scaled, NO_READING_TITLE),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                show_value_and_unit(ui, m, value_size, tc);
                            },
                        );
                    },
                )
                .response
                .rect;
            ui.separator();
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), grid.y),
                Layout::top_down(Align::Min),
                |ui| show_aux_rows(ui, grid_id, m, mode_size, tc),
            );
            ui.separator();
            // Past the dividers, the readouts space as the line under the
            // value spaces them.
            ui.spacing_mut().item_spacing.x = one_row_spacing(mode_size);
            let mode = show_mode_readout(ui, m, mode_size, choices);
            let range = show_range_readout(ui, m, mode_size, choices.range, RangeAtRest::Nothing);
            show_flags(ui, m, mode_size, tc, scaled);
            // The row is the line's height unless the grid made it taller; then
            // the value is, as the selectors centred beside it are no taller.
            let row = ui.min_rect().height();
            let line_h = if grid.y <= value.height() {
                row
            } else {
                value.height()
            };
            (mode.or(range), line_h)
        })
        .inner;
    let extra = grid.x + SEPARATOR_WIDTH + 2.0 * gap;
    (
        picked,
        vec2(ui.min_rect().width() - extra, line_h + spacing_y),
    )
}

/// Render the large primary reading display.
///
/// `scaled` marks the reading as passed through a software transform, so the
/// SCALE badge and the spoken label say so. `choices` are the modes and
/// ranges the meter can be switched to; where there is more than one, that
/// readout becomes a selector, and a pick comes back as its setting and id.
pub fn show_reading(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
) -> Option<ReadoutPick> {
    show_reading_sized(
        ui,
        measurement,
        BASE_READING_FONT_SIZE,
        tc,
        scaled,
        choices,
        NoReadingText::Plain,
    )
}

/// Cached ratios of rendered reading dimensions to font size.
/// Used by `show_reading_large` to compute the optimal font size and
/// updated by the caller only on window resize (to avoid oscillation).
///
/// The two one-row layouts share the line — value, separator, mode — and
/// differ only in where the sub-value grid goes, under it or beside it. So
/// the cache holds the line alone, and the fit adds the grid, whose size it
/// works out from the text: either layout measures the line for both, and
/// the one not drawn is never judged by a stale guess.
#[derive(Clone)]
pub struct ReadingRatios {
    /// Two-line layout: reading width / font_size.
    pub w: f32,
    /// Two-line layout: reading height / font_size.
    pub h: f32,
    /// One-row layouts: the line's width / font_size, without the grid.
    pub row_w: f32,
    /// One-row layouts: the line's height / font_size, the space under it
    /// included, without the grid.
    pub row_h: f32,
}

impl ReadingRatios {
    /// Whether a re-measure agrees with these within 2%: text snaps to
    /// pixels, so the ratios never repeat exactly.
    pub(crate) fn settled(&self, measured: &Self) -> bool {
        let near = |a: f32, b: f32| (a - b).abs() <= 0.02 * a.abs().max(b.abs());
        near(self.w, measured.w)
            && near(self.h, measured.h)
            && near(self.row_w, measured.row_w)
            && near(self.row_h, measured.row_h)
    }

    /// The larger of each ratio: a reading that size fits both measures.
    pub(crate) fn max(&self, other: &Self) -> Self {
        Self {
            w: self.w.max(other.w),
            h: self.h.max(other.h),
            row_w: self.row_w.max(other.row_w),
            row_h: self.row_h.max(other.row_h),
        }
    }
}

impl Default for ReadingRatios {
    fn default() -> Self {
        Self {
            w: 6.5,
            h: 1.8,
            row_w: 10.0,
            row_h: 1.0,
        }
    }
}

/// The cached measurements the big meter sizes its font from. One parameter
/// because the two are measured, cached and invalidated together, and the
/// solver needs both to arrive at a single font size.
pub struct ReadingFit<'a> {
    /// Total height of all content below the reading (buttons, stats, etc.)
    /// rendered at scale=1. The caller measures it once and passes it back
    /// in so the optimal scale can be computed.
    pub base_content_height: f32,
    pub ratios: &'a ReadingRatios,
}

/// Where the big meter puts the mode line and the sub-values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadingLayout {
    /// Value, then the sub-value rows, then the mode line wrapped under them.
    TwoLine,
    /// Value │ mode on one line, the sub-value rows under it.
    Below,
    /// Value │ sub-values │ mode, all on one row.
    Beside,
}

/// The sub-value grid as the fit weighs it: its text, laid out once, and
/// the pixel spacing egui puts around it.
struct GridFit {
    metrics: AuxGridMetrics,
    /// egui's `interact_size`, the floor under every grid cell.
    min_cell: Vec2,
    /// egui's vertical `item_spacing`, left under the grid as under any
    /// widget.
    spacing_y: f32,
    /// Whether the mode line carries selectors, which space its dividers
    /// wider ([`divider_gap`]).
    selectors: bool,
}

impl GridFit {
    /// The grid's size beside or under a value at `value_size`.
    fn size(&self, value_size: f32) -> Vec2 {
        self.metrics
            .size(value_size * MODE_SIZE_RATIO, self.min_cell)
    }

    /// What the beside layout adds to the line's width at `value_size`: the
    /// grid, a second separator, and the spacing either side of it.
    fn beside_extra_width(&self, value_size: f32) -> f32 {
        self.size(value_size).x
            + SEPARATOR_WIDTH
            + 2.0 * divider_gap(value_size * MODE_SIZE_RATIO, self.selectors)
    }
}

/// The largest font size at which `fits` holds, searched up to `limit`; 0
/// when even the smallest does not fit. `fits` must grow monotonically
/// stricter with the size, as every layout's extent does.
fn largest_fitting_size(limit: f32, fits: impl Fn(f32) -> bool) -> f32 {
    let (mut lo, mut hi) = (0.0_f32, limit.max(1.0));
    if fits(hi) {
        return hi;
    }
    // Forty halvings take any window's range well under a hundredth of a
    // point, far below what a pixel shows.
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Pick the big meter's layout and value font size for a reading in
/// `avail`, with `content_coeff` points of content under it per point of
/// font: whichever layout gives the largest value wins, so there is no
/// width breakpoint. `grid` is `None` when there are no sub-values, and
/// then the beside layout is no different from the below one and is never
/// offered.
///
/// Beside has to beat the others outright: at a tie the below layout, which
/// predates it, keeps the picture it always drew.
///
/// `last` is the layout drawn the frame before. With sub-values the choice
/// is weighed every frame from their live text, so a timestamp growing a
/// digit or a value turning to OL near the boundary would otherwise flip the
/// whole reading back and forth; the last layout stays until another beats
/// it by [`LAYOUT_SWITCH_MARGIN`]. Without sub-values the choice comes from
/// the cached ratios alone, which move only on a re-measure, and `last` is
/// ignored.
fn pick_layout(
    avail: Vec2,
    content_coeff: f32,
    ratios: &ReadingRatios,
    grid: Option<&GridFit>,
    last: Option<ReadingLayout>,
) -> (ReadingLayout, f32) {
    let two_line = (avail.x / ratios.w).min(avail.y / (ratios.h + content_coeff));
    let (below, beside) = match grid {
        // The closed form, as with no grid the line is the whole reading.
        None => (
            (avail.x / ratios.row_w).min(avail.y / (ratios.row_h + content_coeff)),
            0.0,
        ),
        Some(grid) => {
            let limit = avail.x.max(avail.y);
            let below = largest_fitting_size(limit, |s| {
                let g = grid.size(s);
                ratios.row_w * s <= avail.x
                    && g.x <= avail.x
                    && (ratios.row_h + content_coeff) * s + g.y + grid.spacing_y <= avail.y
            });
            let beside = largest_fitting_size(limit, |s| {
                let g = grid.size(s);
                ratios.row_w * s + grid.beside_extra_width(s) <= avail.x
                    && (ratios.row_h * s).max(g.y + grid.spacing_y) + content_coeff * s <= avail.y
            });
            (below, beside)
        }
    };
    let (layout, size) = if below >= two_line {
        (ReadingLayout::Below, below)
    } else {
        (ReadingLayout::TwoLine, two_line)
    };
    let best = if beside > size {
        (ReadingLayout::Beside, beside)
    } else {
        (layout, size)
    };
    let kept = last.filter(|_| grid.is_some()).map(|last| {
        let size = match last {
            ReadingLayout::TwoLine => two_line,
            ReadingLayout::Below => below,
            ReadingLayout::Beside => beside,
        };
        (last, size)
    });
    match kept {
        Some((last, size)) if size > 0.0 && best.1 <= size * (1.0 + LAYOUT_SWITCH_MARGIN) => {
            (last, size)
        }
        _ => best,
    }
}

/// How much larger another layout must make the value before the big meter
/// leaves the one it drew last; see [`pick_layout`].
const LAYOUT_SWITCH_MARGIN: f32 = 0.02;

/// What the big meter drew last frame, kept in egui's memory: the layout,
/// for [`pick_layout`]'s margin, and the value's size, which the fit lays the
/// sub-values out at to weigh them — a size already drawn, so its glyphs are
/// cached, and the very size the next frame picks once the window is still.
#[derive(Clone, Copy)]
struct LastFit {
    layout: ReadingLayout,
    size: f32,
}

/// Render an extra-large reading that scales to fill available space.
/// Used when graph and recording panels are hidden ("big meter" mode).
/// Returns `(scale_factor, measured_ratios, picked)`. The caller should
/// only persist `measured_ratios` into the cached state when recalculating
/// (e.g. on window resize) to avoid frame-to-frame oscillation; `picked` is
/// the setting and value the user chose in a selector, as for
/// [`show_reading`].
///
/// `no_reading` is what takes the reading's place when there is none — the
/// connection issue's title, where the caller has one.
pub fn show_reading_large(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    fit: ReadingFit<'_>,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
    no_reading: NoReadingText<'_>,
) -> (f32, ReadingRatios, Option<ReadoutPick>) {
    let ReadingFit {
        base_content_height,
        ratios,
    } = fit;
    // A notice placeholder sizes both layouts from its own text; see
    // `NoReadingText::layout_ratios` for why the cache cannot be trusted.
    let own_ratios = measurement
        .is_none()
        .then(|| no_reading.layout_ratios(ui))
        .flatten();
    let ratios = own_ratios.as_ref().unwrap_or(ratios);
    let last_id = ui.id().with("big_meter_last_fit");
    let last: Option<LastFit> = ui.data(|d| d.get_temp(last_id));
    // Laid out at the size drawn last, scaled from there: text does not
    // scale exactly, but at the size it was laid out at the estimate is the
    // grid as drawn, so a still window fits it to the pixel. The side panel's
    // size, whose glyphs are cached too, stands in before the first frame.
    let reference = last.map_or(BASE_READING_FONT_SIZE, |l| l.size);
    let grid = measurement
        .and_then(|m| AuxGridMetrics::measure(ui, m, reference * MODE_SIZE_RATIO, tc))
        .map(|metrics| GridFit {
            metrics,
            min_cell: ui.spacing().interact_size,
            spacing_y: ui.spacing().item_spacing.y,
            selectors: choices.any_offered(),
        });
    let avail = ui.available_size();
    let content_coeff = base_content_height / BASE_READING_FONT_SIZE;
    let (layout, size) = pick_layout(
        avail,
        content_coeff,
        ratios,
        grid.as_ref(),
        last.map(|l| l.layout),
    );
    let size = size.max(MIN_BIG_METER_FONT_SIZE);
    ui.data_mut(|d| d.insert_temp(last_id, LastFit { layout, size }));

    // Render and measure actual dimensions.
    let before = ui.cursor().top();
    let mut measured = ratios.clone();
    let picked = match (layout, measurement, grid.as_ref()) {
        (ReadingLayout::Beside, Some(m), Some(_)) => {
            let (picked, line) = show_reading_beside(ui, m, size, tc, scaled, choices);
            measured.row_w = line.x / size;
            measured.row_h = line.y / size;
            picked
        }
        (ReadingLayout::TwoLine, ..) => {
            let picked = show_reading_sized(ui, measurement, size, tc, scaled, choices, no_reading);
            measured.w = ui.min_rect().width() / size;
            measured.h = (ui.cursor().top() - before) / size;
            picked
        }
        _ => {
            let (picked, line) =
                show_reading_inline(ui, measurement, size, tc, scaled, choices, no_reading);
            measured.row_w = line.x / size;
            measured.row_h = line.y / size;
            picked
        }
    };

    (size / BASE_READING_FONT_SIZE, measured, picked)
}

/// Render the reading as a compact single line (for narrow layout).
///
/// Returns the setting and value the user picked from a selector, as for
/// [`show_reading`].
pub fn show_reading_compact(
    ui: &mut Ui,
    measurement: Option<&Measurement>,
    tc: &ThemeColors,
    scaled: bool,
    choices: ReadoutChoices<'_>,
) -> Option<ReadoutPick> {
    match measurement {
        Some(m) => {
            let value_text = format_value_display(m);
            let draw_value = |ui: &mut Ui| {
                let color = tc.reading();
                ui.label(value_label(
                    ui,
                    m,
                    &value_text,
                    FontId::monospace(COMPACT_READING_FONT_SIZE),
                    color,
                    MIN_AUX_FONT_SIZE,
                ));
                ui.label(
                    RichText::new(&*m.unit)
                        .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                        .color(color),
                );
            };

            let picked = if !choices.any_offered() {
                show_reading_line_plain(ui, m, scaled, draw_value, 0.0, tc);
                None
            } else {
                // The selectors and badges at the small text size, which is
                // what `.small()` resolves to for the plain label.
                let small = TextStyle::Small.resolve(ui.style()).size;
                show_reading_line_with_selector(ui, m, scaled, draw_value, small, choices, tc)
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
            no_reading_placeholder(ui, scaled, NO_READING_TITLE, |ui| {
                ui.label(
                    RichText::new(format!("{} {NO_READING_TITLE}", crate::NO_DATA))
                        .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                        .color(ui.visuals().weak_text_color()),
                );
            });
            None
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

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::flags::StatusFlags;
    use std::borrow::Cow;

    /// Build a sub-value the way the protocols do: digits in `display_raw`,
    /// unit empty when it matches the main reading's.
    pub(super) fn aux(label: &'static str, display: &str, unit: &'static str) -> AuxValue {
        AuxValue {
            label: label.into(),
            value: MeasuredValue::Normal(display.trim().parse().unwrap_or(0.0)),
            unit: unit.into(),
            display_raw: Some(display.to_string()),
            elapsed_secs: None,
        }
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
            let mut out = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
                rects = show_aux_rows(ui, aux_grid_id(ui), m, font_size, &tc);
            });
            // See `run_frame`: epaint 0.36 debug-asserts that texture deltas
            // were applied before being dropped.
            out.textures_delta.clear();
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

    /// Rects of the readout row's three kinds of widget — mode label, range
    /// dropdown, AUTO badge — laid out the way the big-meter rows lay them
    /// out, in a headless egui context. `wrapped` picks the two-line row
    /// (`horizontal_wrapped` under the reading) over the inline one (a
    /// `horizontal` after the value and a separator).
    fn layout_readout_row(value_size: f32, wrapped: bool) -> [egui::Rect; 3] {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m = Measurement::test_fixture(
            MeasuredValue::Normal(50.001),
            "mA",
            StatusFlags {
                auto_range: true,
                ..Default::default()
            },
        );
        let mode_size = value_size * MODE_SIZE_RATIO;
        let ranges = range_choices();
        let mut rects = [egui::Rect::NOTHING; 3];
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut row = |ui: &mut Ui| {
                    let mode = ui
                        .scope(|ui| show_choice_readout(ui, &MODE_READOUT, "DC mA", mode_size, &[]))
                        .response
                        .rect;
                    let range = ui
                        .scope(|ui| {
                            show_choice_readout(ui, &RANGE_READOUT, "220mA", mode_size, &ranges)
                        })
                        .response
                        .rect;
                    let badge = ui
                        .scope(|ui| show_flags(ui, &m, mode_size, &tc, false))
                        .response
                        .rect;
                    rects = [mode, range, badge];
                };
                if wrapped {
                    ui.label(RichText::new("50.001mA").font(FontId::monospace(value_size)));
                    ui.horizontal_wrapped(row);
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("50.001mA").font(FontId::monospace(value_size)));
                        ui.separator();
                        row(ui);
                    });
                }
            });
            out.textures_delta.clear();
        }
        rects
    }

    /// The range dropdown and the AUTO badge must sit level with the mode
    /// label at any reading size.
    ///
    /// Regression test for the big-meter offset: `ComboBox::show_ui` lays
    /// the box out in a nested `ui.horizontal`, whose row starts
    /// `interact_size.y` (~18 px) tall and pushes taller content down, so at
    /// a 130 px reading the dropdown hung about 23 px below its label — and,
    /// in the wrapped row, dragged the badge after it half as far.
    #[test]
    fn readout_dropdown_and_badge_share_a_baseline() {
        // 36.0 is the side-panel reading size, 130.0 a big-meter one.
        for wrapped in [false, true] {
            for size in [36.0_f32, 130.0] {
                let [mode, range, badge] = layout_readout_row(size, wrapped);
                for (name, rect) in [("dropdown", range), ("badge", badge)] {
                    let delta = (rect.center().y - mode.center().y).abs();
                    assert!(
                        delta <= 1.0,
                        "{name} at {size} px (wrapped: {wrapped}): centre {} vs label centre {} (delta {delta} px)",
                        rect.center().y,
                        mode.center().y
                    );
                }
            }
        }
    }

    /// A reading with `rows` sub-values, the first `with_secs` of them
    /// stamped as MIN/MAX extremes are.
    fn reading_with_rows(rows: usize, with_secs: usize) -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        m.aux_values = (0..rows)
            .map(|i| {
                let mut a = aux(["Max", "Average", "Min", "Raw"][i], &format!("1.{i}25"), "");
                a.elapsed_secs = (i < with_secs).then_some(12);
                a
            })
            .collect();
        m
    }

    /// The grid as drawn: run a few frames, as the grid sizes its columns
    /// from the frame before, and return the last frame's rect.
    fn drawn_grid(m: &Measurement, font_size: f32) -> egui::Rect {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut rect = egui::Rect::NOTHING;
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let id = aux_grid_id(ui);
                rect = ui
                    .scope(|ui| show_aux_rows(ui, id, m, font_size, &tc))
                    .response
                    .rect;
            });
            out.textures_delta.clear();
        }
        rect
    }

    fn grid_metrics(m: &Measurement, font_size: f32) -> (AuxGridMetrics, Vec2) {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut metrics = None;
        let mut min_cell = Vec2::ZERO;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            metrics = AuxGridMetrics::measure(ui, m, font_size, &tc);
            min_cell = ui.spacing().interact_size;
        });
        out.textures_delta.clear();
        (metrics.expect("the reading has sub-values"), min_cell)
    }

    /// The beside layout centres the grid on a height worked out from the
    /// text, and the fit adds the grid's width to the line's: both are only
    /// right if the worked-out size is the size the grid draws at. Laid out
    /// at the size drawn it matches to the pixel; scaled from the side
    /// panel's size, as the fit scales it, to within a couple of percent.
    #[test]
    fn the_grid_size_worked_out_from_the_text_is_the_size_drawn() {
        let base = BASE_READING_FONT_SIZE * MODE_SIZE_RATIO;
        for (rows, secs) in [(2, 0), (4, 0), (3, 3)] {
            let m = reading_with_rows(rows, secs);
            // 14.4 is the side panel's mode size, 52 a 130 px big meter's.
            for size in [base, 52.0] {
                let drawn = drawn_grid(&m, size).size();
                let (exact, min_cell) = grid_metrics(&m, size);
                let worked_out = exact.size(size, min_cell);
                assert!(
                    (worked_out - drawn).abs().max_elem() <= 1.0,
                    "{rows} rows ({secs} stamped) at {size}: worked out {worked_out:?}, drawn {drawn:?}"
                );
                let (scaled, _) = grid_metrics(&m, base);
                let scaled = scaled.size(size, min_cell);
                let off = ((scaled - drawn) / drawn).abs().max_elem();
                assert!(
                    off <= 0.02,
                    "{rows} rows ({secs} stamped) at {size}: scaled {scaled:?}, drawn {drawn:?}"
                );
            }
        }
    }

    /// Bounds of the plain label whose text contains `needle`. Labels carry
    /// their text as the node's value; the live region's row carries the
    /// whole sentence as its label, so it never matches here.
    fn label_bounds(nodes: &[(NodeId, Node)], needle: &str) -> egui::Rect {
        let (_, node) = nodes
            .iter()
            .find(|(_, n)| n.value().is_some_and(|t| t.contains(needle)))
            .unwrap_or_else(|| panic!("no label reads {needle:?}"));
        let b = node.bounds().expect("drawn widgets have bounds");
        egui::Rect::from_min_max(
            egui::pos2(b.x0 as f32, b.y0 as f32),
            egui::pos2(b.x1 as f32, b.y1 as f32),
        )
    }

    /// Value │ sub-values │ mode: the three blocks run left to right and sit
    /// centred on each other, whether the grid is shorter than the value
    /// (two rows) or outgrows it (four). A nested layout placed at its
    /// natural size starts `interact_size.y` tall and grows downwards, which
    /// left the value hanging under the grid and the mode line.
    #[test]
    fn the_beside_row_centres_value_sub_values_and_mode() {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        for rows in [2, 4] {
            let m = reading_with_rows(rows, 0);
            for size in [36.0_f32, 130.0] {
                let mut nodes = Vec::new();
                for _ in 0..3 {
                    nodes = run_frame(&ctx, Vec::new(), |ui| {
                        show_reading_beside(ui, &m, size, &tc, false, ReadoutChoices::default()).0
                    })
                    .nodes;
                }
                let case = format!("{rows} rows at {size} px");
                let value = label_bounds(&nodes, "5.678");
                let grid = label_bounds(&nodes, "Max")
                    .union(label_bounds(&nodes, "1.025"))
                    .union(label_bounds(&nodes, &format!("1.{}25", rows - 1)));
                let mode = label_bounds(&nodes, "DC V");
                for (name, rect) in [("sub-values", grid), ("mode", mode)] {
                    let delta = (rect.center().y - value.center().y).abs();
                    assert!(
                        delta <= 1.0,
                        "{case}: {name} centre {} vs value centre {} (delta {delta} px)",
                        rect.center().y,
                        value.center().y
                    );
                }
                assert!(
                    value.right() < grid.left() && grid.right() < mode.left(),
                    "{case}: value {value:?}, sub-values {grid:?}, mode {mode:?} out of order"
                );
            }
        }
    }

    /// Two T1/T2-sized sub-values, as `GridFit` sees them at the side
    /// panel's size: a 2-row grid about 120 px wide and 40 tall there.
    fn two_row_grid() -> GridFit {
        GridFit {
            metrics: AuxGridMetrics {
                font_size: BASE_READING_FONT_SIZE * MODE_SIZE_RATIO,
                label_w: 18.0,
                value_w: 80.0,
                secs_w: 0.0,
                text_h: 18.0,
                rows: 2,
            },
            min_cell: vec2(40.0, 18.0),
            spacing_y: 3.0,
            selectors: true,
        }
    }

    /// A value │ mode line about seven value-widths long, as measured.
    fn line_ratios() -> ReadingRatios {
        ReadingRatios {
            w: 6.5,
            h: 1.8,
            row_w: 7.0,
            row_h: 1.2,
        }
    }

    /// A wide, short window is the one the rows under the value starve of
    /// height: the sub-values move beside it, and the value grows.
    #[test]
    fn a_wide_short_window_puts_the_sub_values_beside_the_value() {
        let grid = two_row_grid();
        for avail in [vec2(1998.0, 275.0), vec2(1200.0, 200.0)] {
            let (layout, size) = pick_layout(avail, 0.0, &line_ratios(), Some(&grid), None);
            assert_eq!(layout, ReadingLayout::Beside, "{avail:?}");
            // And what it picked fits.
            let width = line_ratios().row_w * size + grid.beside_extra_width(size);
            assert!(
                width <= avail.x + 0.01,
                "{avail:?}: {width} px wide at {size}"
            );
            let height = (line_ratios().row_h * size).max(grid.size(size).y + grid.spacing_y);
            assert!(
                height <= avail.y + 0.01,
                "{avail:?}: {height} px tall at {size}"
            );
        }
    }

    /// Where width is what runs out first, a third column only costs size:
    /// a narrow or tall window keeps the rows under the value.
    #[test]
    fn a_narrow_or_tall_window_keeps_the_sub_values_under_the_value() {
        let grid = two_row_grid();
        for avail in [vec2(420.0, 170.0), vec2(900.0, 640.0), vec2(1998.0, 1400.0)] {
            let (layout, _) = pick_layout(avail, 0.0, &line_ratios(), Some(&grid), None);
            assert_ne!(layout, ReadingLayout::Beside, "{avail:?}");
        }
    }

    /// With no sub-values the beside layout is the below one, and is never
    /// picked; the choice and size are the closed form the big meter has
    /// always used.
    #[test]
    fn a_reading_without_sub_values_never_goes_beside() {
        let ratios = line_ratios();
        for avail in [vec2(1998.0, 275.0), vec2(420.0, 170.0), vec2(900.0, 640.0)] {
            for content_coeff in [0.0, 2.5] {
                let (layout, size) = pick_layout(avail, content_coeff, &ratios, None, None);
                assert_ne!(layout, ReadingLayout::Beside, "{avail:?}");
                let last = Some(ReadingLayout::Beside);
                assert_eq!(
                    pick_layout(avail, content_coeff, &ratios, None, last),
                    (layout, size),
                    "{avail:?}: a last layout moved a reading with no sub-values"
                );
                let two_line = (avail.x / ratios.w).min(avail.y / (ratios.h + content_coeff));
                let below = (avail.x / ratios.row_w).min(avail.y / (ratios.row_h + content_coeff));
                assert_eq!(size, two_line.max(below), "{avail:?}");
            }
        }
    }

    /// Near the boundary the layout drawn last stays, unless another makes
    /// the value clearly larger: with sub-values the choice is weighed every
    /// frame, and a timestamp growing a digit must not flip the reading.
    #[test]
    fn the_last_layout_stays_until_another_clearly_wins() {
        let grid = two_row_grid();
        let ratios = line_ratios();
        let pick = |avail, last| pick_layout(avail, 0.0, &ratios, Some(&grid), last);
        // Grow the height until beside stops winning: one pixel short of
        // that, it wins by a hair over the layout that takes over.
        let mut avail = vec2(1200.0, 100.0);
        while pick(avail, None).0 == ReadingLayout::Beside {
            avail.y += 1.0;
        }
        let (other, _) = pick(avail, None);
        avail.y -= 1.0;
        assert_eq!(pick(avail, None).0, ReadingLayout::Beside);
        assert_eq!(
            pick(avail, Some(other)).0,
            other,
            "a marginal gain flipped the layout from {other:?}"
        );
        // A window where beside wins outright switches straight away.
        assert_eq!(
            pick(vec2(1998.0, 275.0), Some(ReadingLayout::Below)).0,
            ReadingLayout::Beside
        );
    }

    /// Both one-row layouts measure the line the fit caches for both, so
    /// they must agree on it, with selectors on the line or without: a
    /// disagreement makes each pass undo the one before.
    #[test]
    fn both_one_row_layouts_measure_the_same_line() {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m = reading_with_rows(2, 0);
        let offered = two_choices();
        for choices in [ReadoutChoices::default(), modes(&offered)] {
            for size in [36.0_f32, 130.0] {
                let mut lines = [Vec2::ZERO; 2];
                for (i, beside) in [false, true].into_iter().enumerate() {
                    for _ in 0..3 {
                        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                            lines[i] = if beside {
                                show_reading_beside(ui, &m, size, &tc, false, choices).1
                            } else {
                                show_reading_inline(
                                    ui,
                                    Some(&m),
                                    size,
                                    &tc,
                                    false,
                                    choices,
                                    NoReadingText::Plain,
                                )
                                .1
                            };
                        });
                        out.textures_delta.clear();
                    }
                }
                let [below, beside] = lines;
                let selectors = choices.any_offered();
                assert!(
                    (below.x - beside.x).abs() <= 1.0,
                    "selectors {selectors}, {size} px: below's line is {below:?}, beside's {beside:?}"
                );
            }
        }
    }

    /// A layout switch keeps the sub-value grid's id. A grid under a new id
    /// hides itself for a sizing pass and discards the frame — and that
    /// discarded frame counted against the fit's re-measure passes.
    #[test]
    fn switching_to_beside_keeps_the_grid() {
        let ctx = egui::Context::default();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m = reading_with_rows(2, 0);
        // A discarded frame is run again inside the same `run_ui`, so it
        // shows as the closure running twice.
        let passes = |beside: bool| {
            let mut passes = 0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                passes += 1;
                if beside {
                    let _ = show_reading_beside(ui, &m, 130.0, &tc, false, Default::default());
                } else {
                    let _ = show_reading_inline(
                        ui,
                        Some(&m),
                        130.0,
                        &tc,
                        false,
                        Default::default(),
                        NoReadingText::Plain,
                    );
                }
            });
            out.textures_delta.clear();
            passes
        };
        for _ in 0..3 {
            passes(false);
        }
        assert_eq!(passes(true), 1, "the grid started over under a new id");
    }

    /// Single-display meters must draw nothing at all — no grid, no row.
    #[test]
    fn aux_rows_draw_nothing_without_sub_values() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(1.234), "V", StatusFlags::default());
        assert!(m.aux_values.is_empty());
        assert!(layout_aux_rows(&m, 130.0).is_empty());
    }

    // ---- the mode selector ------------------------------------------------

    use eframe::egui;
    use eframe::egui::accesskit::{Node, NodeId, Role, Toggled};

    fn choice(id: u16, label: &'static str, current: bool) -> Choice {
        Choice {
            id,
            label: Cow::Borrowed(label),
            current,
        }
    }

    /// Two entries for one dial position, the fixture's own mode live so the
    /// readout and the marked entry agree.
    fn two_choices() -> Vec<Choice> {
        vec![
            choice(0x1111, "DC V", true),
            choice(0x1121, "AC V Hz", false),
        ]
    }

    /// Auto plus a two-rung ladder, the meter auto-ranging — what the mock
    /// reports on its DC V position before anything is picked.
    fn range_choices() -> Vec<Choice> {
        vec![
            choice(0, "Auto", true),
            choice(1, "2.2V", false),
            choice(2, "22V", false),
        ]
    }

    /// Only the mode readout has something to offer.
    fn modes(choices: &[Choice]) -> ReadoutChoices<'_> {
        ReadoutChoices {
            mode: choices,
            range: &[],
            keys: &[],
        }
    }

    /// Only the range readout has.
    fn ranges(choices: &[Choice]) -> ReadoutChoices<'_> {
        ReadoutChoices {
            mode: &[],
            range: choices,
            keys: &[],
        }
    }

    /// A reading on the mock's DC V position: 22 V rung, auto-ranging, so the
    /// range readout has a label to show whether or not it can be switched.
    fn ranged_reading() -> Measurement {
        let mut m = Measurement::test_fixture(
            MeasuredValue::Normal(5.678),
            "V",
            StatusFlags {
                auto_range: true,
                ..StatusFlags::default()
            },
        );
        m.range_label = Cow::Borrowed("22V");
        m
    }

    /// The layouts the readout appears in, drawn at the side-panel size.
    const LAYOUTS: [&str; 4] = ["two-line", "inline", "beside", "compact"];

    fn draw_reading(
        ui: &mut Ui,
        layout: &str,
        m: &Measurement,
        choices: ReadoutChoices<'_>,
    ) -> Option<ReadoutPick> {
        let tc = crate::settings::Settings::default().theme_colors(true);
        match layout {
            "two-line" => show_reading_sized(
                ui,
                Some(m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                choices,
                NoReadingText::Plain,
            ),
            "inline" => {
                show_reading_inline(
                    ui,
                    Some(m),
                    BASE_READING_FONT_SIZE,
                    &tc,
                    false,
                    choices,
                    NoReadingText::Plain,
                )
                .0
            }
            "beside" => show_reading_beside(ui, m, BASE_READING_FONT_SIZE, &tc, false, choices).0,
            _ => show_reading_compact(ui, Some(m), &tc, false, choices),
        }
    }

    /// One headless frame with AccessKit on: what `draw` returned, the
    /// frame's accessibility nodes and the focused one — the one place a
    /// test can see which widgets were drawn, what they are called, where
    /// they are, and which has the keyboard.
    struct Frame {
        picked: Option<ReadoutPick>,
        nodes: Vec<(NodeId, Node)>,
        focus: Option<NodeId>,
    }

    fn run_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        mut draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
    ) -> Frame {
        ctx.enable_accesskit();
        let mut picked = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| picked = draw(ui),
        );
        // epaint 0.36 added a `Drop` guard on `TexturesDelta` that
        // debug-asserts the deltas were applied. This harness renders without
        // a painter, so discard them explicitly.
        out.textures_delta.clear();
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
        mut draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
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
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, ReadoutChoices::default())
            });
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
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, modes(&choices))
            });
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
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, modes(&choices))
            });
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
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                Some(&m),
                200.0,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
        };

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(
            &node_with_role(&f.nodes, Role::ComboBox)
                .expect("combo box")
                .1,
        );
        let f = click(&ctx, combo, &mut draw);
        assert_eq!(f.picked, None, "opening the popup is not a pick");

        let f = run_frame(&ctx, vec![], &mut draw);
        let live = node_labelled(&f.nodes, &format!("{LIVE_CHOICE_MARK} DC V"))
            .expect("live entry, marked");
        assert_eq!(live.toggled(), Some(Toggled::True));
        let other = node_labelled(&f.nodes, "   AC V Hz").expect("other entry");
        assert_eq!(other.toggled(), Some(Toggled::False));
        let b = other.bounds().expect("entry bounds");
        let height = (b.y1 - b.y0) as f32;
        assert!(
            height < 2.0 * MAX_CHOICE_POPUP_FONT_SIZE,
            "popup entry {height} px tall beside an 80 px readout"
        );

        let f = click(&ctx, centre(other), &mut draw);
        assert_eq!(f.picked, Some(ReadoutPick::Select(Setting::Mode, 0x1121)));
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
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
        };

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(
            &node_with_role(&f.nodes, Role::ComboBox)
                .expect("combo box")
                .1,
        );
        click(&ctx, combo, &mut draw);
        let f = run_frame(&ctx, vec![], &mut draw);
        let live =
            node_labelled(&f.nodes, &format!("{LIVE_CHOICE_MARK} DC V")).expect("live entry");
        let f = click(&ctx, centre(live), &mut draw);
        assert_eq!(f.picked, None);
    }

    // ---- the range selector -----------------------------------------------

    /// A meter that cannot be ranged remotely — and one whose mode has a
    /// single rung — keeps the plain range label the two-line layout has
    /// always drawn beside the mode.
    #[test]
    fn the_range_readout_stays_a_label_without_choices() {
        let m = ranged_reading();
        for choices in [vec![], vec![choice(2, "22V", true)]] {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, "two-line", &m, ranges(&choices))
            });
            assert_eq!(f.picked, None);
            assert!(
                node_with_role(&f.nodes, Role::ComboBox).is_none(),
                "{} choices drew a combo box",
                choices.len()
            );
            assert!(
                f.nodes
                    .iter()
                    .any(|(_, n)| n.role() == Role::Label && n.value() == Some("22V")),
                "{} choices lost the range label",
                choices.len()
            );
        }
    }

    /// With a ladder to pick from, the range readout is a combo box a screen
    /// reader calls "Range" whose value is the rung the meter reports.
    #[test]
    fn the_range_readout_becomes_a_named_combo_with_choices() {
        let m = ranged_reading();
        let choices = range_choices();
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, ranges(&choices))
            });
            assert_eq!(f.picked, None, "{layout}: drawing is not picking");
            let combo = node_labelled(&f.nodes, "Range")
                .unwrap_or_else(|| panic!("{layout}: no range combo"));
            assert_eq!(combo.role(), Role::ComboBox, "{layout}");
            assert_eq!(combo.value(), Some("22V"), "{layout}");
        }
    }

    /// The single-line layouts never carried a range label, and a meter that
    /// cannot be ranged must not gain one there.
    #[test]
    fn the_single_line_layouts_show_no_range_without_choices() {
        let m = ranged_reading();
        for layout in ["inline", "compact"] {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, modes(&two_choices()))
            });
            assert!(
                node_labelled(&f.nodes, "Range").is_none(),
                "{layout}: a range readout appeared"
            );
            assert!(
                !f.nodes
                    .iter()
                    .any(|(_, n)| n.value() == Some("22V") || n.label() == Some("22V")),
                "{layout}: the range text appeared"
            );
        }
    }

    /// Both readouts on one line are two independent controls: egui keys a
    /// combo's popup and focus state by its id, and a shared id would have
    /// them open and close together.
    #[test]
    fn the_two_readouts_have_distinct_ids() {
        let m = ranged_reading();
        let (modes, ranges) = (two_choices(), range_choices());
        let both = ReadoutChoices {
            mode: &modes,
            range: &ranges,
            keys: &[],
        };
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, both));
            let mode = node_id_labelled(&f.nodes, "Mode")
                .unwrap_or_else(|| panic!("{layout}: no mode combo"));
            let range = node_id_labelled(&f.nodes, "Range")
                .unwrap_or_else(|| panic!("{layout}: no range combo"));
            assert_ne!(mode, range, "{layout}");
        }
    }

    /// Tab walks the reading line left to right: the mode readout, then the
    /// range one — the order they are read in.
    #[test]
    fn tab_reaches_the_mode_readout_then_the_range_one() {
        let m = ranged_reading();
        let (modes, ranges) = (two_choices(), range_choices());
        let both = ReadoutChoices {
            mode: &modes,
            range: &ranges,
            keys: &[],
        };
        let ctx = egui::Context::default();
        let mut draw = |ui: &mut Ui| draw_reading(ui, "two-line", &m, both);

        run_frame(&ctx, vec![], &mut draw);
        let f = run_frame(&ctx, key(egui::Key::Tab, egui::Modifiers::NONE), &mut draw);
        assert_eq!(f.focus, node_id_labelled(&f.nodes, "Mode"), "first Tab");
        let f = run_frame(&ctx, key(egui::Key::Tab, egui::Modifiers::NONE), &mut draw);
        assert_eq!(f.focus, node_id_labelled(&f.nodes, "Range"), "second Tab");
    }

    /// Opening the range list and picking a rung reports it against
    /// `Setting::Range`, so the caller knows which setting to switch.
    #[test]
    fn picking_a_range_reports_its_id() {
        let ctx = egui::Context::default();
        let m = ranged_reading();
        let choices = range_choices();
        let mut draw = |ui: &mut Ui| draw_reading(ui, "two-line", &m, ranges(&choices));

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(node_labelled(&f.nodes, "Range").expect("range combo"));
        click(&ctx, combo, &mut draw);

        let f = run_frame(&ctx, vec![], &mut draw);
        let auto = node_labelled(&f.nodes, &format!("{LIVE_CHOICE_MARK} Auto"))
            .expect("Auto is marked while the meter auto-ranges");
        assert_eq!(auto.toggled(), Some(Toggled::True));
        let rung = node_labelled(&f.nodes, "   22V").expect("rung entry");

        let f = click(&ctx, centre(rung), &mut draw);
        assert_eq!(f.picked, Some(ReadoutPick::Select(Setting::Range, 2)));
    }

    /// Re-picking the marked entry sends nothing to the meter.
    #[test]
    fn picking_the_live_range_is_not_a_pick() {
        let ctx = egui::Context::default();
        let m = ranged_reading();
        let choices = range_choices();
        let mut draw = |ui: &mut Ui| draw_reading(ui, "two-line", &m, ranges(&choices));

        let f = run_frame(&ctx, vec![], &mut draw);
        let combo = centre(node_labelled(&f.nodes, "Range").expect("range combo"));
        click(&ctx, combo, &mut draw);
        let f = run_frame(&ctx, vec![], &mut draw);
        let live =
            node_labelled(&f.nodes, &format!("{LIVE_CHOICE_MARK} Auto")).expect("live entry");
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
        mut draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
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
    fn assert_closed_on_the_readout(
        ctx: &egui::Context,
        draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
    ) {
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

    fn keyboard_fixture() -> (Measurement, Vec<Choice>, ThemeColors) {
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
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
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
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
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
        assert_eq!(f.picked, Some(ReadoutPick::Select(Setting::Mode, 0x1121)));
        assert_closed_on_the_readout(&ctx, &mut draw);
    }

    /// ArrowUp stops at the first entry; Home and End jump to the ends.
    #[test]
    fn home_end_and_arrow_up_stay_within_the_list() {
        let (m, choices, tc) = keyboard_fixture();
        let ctx = egui::Context::default();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
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
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                modes(&choices),
                NoReadingText::Plain,
            )
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
                show_reading_sized(
                    ui,
                    Some(&m),
                    BASE_READING_FONT_SIZE,
                    &tc,
                    false,
                    modes(&choices),
                    NoReadingText::Plain,
                )
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

    // ---- the key list -----------------------------------------------------

    /// Two function keys: V applies to a volt reading, Ω to none here.
    const KEYS: &[MeterKey] = &[
        MeterKey {
            command: "volts",
            label: "V",
            hover: None,
            applies: |m| m.unit == "V",
        },
        MeterKey {
            command: "ohms",
            label: "Ω",
            hover: None,
            applies: |m| m.unit == "Ω",
        },
    ];

    fn keys(keys: &[MeterKey]) -> ReadoutChoices<'_> {
        ReadoutChoices {
            mode: &[],
            range: &[],
            keys,
        }
    }

    /// Draw a reading in `unit` with [`KEYS`] on the mode readout.
    fn draw_with_keys(unit: &'static str) -> impl FnMut(&mut Ui) -> Option<ReadoutPick> {
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), unit, StatusFlags::default());
        move |ui: &mut Ui| {
            show_reading_sized(
                ui,
                Some(&m),
                BASE_READING_FONT_SIZE,
                &tc,
                false,
                keys(KEYS),
                NoReadingText::Plain,
            )
        }
    }

    /// With function keys and no modes the readout is a combo box a screen
    /// reader still calls "Mode", valued the live mode, in every layout.
    #[test]
    fn the_mode_readout_lists_keys_as_a_named_combo() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        for layout in LAYOUTS {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, keys(KEYS)));
            assert_eq!(f.picked, None, "{layout}: drawing is not picking");
            let (_, combo) = node_with_role(&f.nodes, Role::ComboBox)
                .unwrap_or_else(|| panic!("{layout}: no combo box"));
            assert_eq!(combo.label(), Some("Mode"), "{layout}");
            assert_eq!(combo.value(), Some("DC V"), "{layout}");
        }
    }

    /// The open list is captioned, marks the key the reading shows, and a
    /// pick is a press — of the marked key too, which can cycle within its
    /// function.
    #[test]
    fn picking_a_key_presses_it_even_the_marked_one() {
        let mut draw = draw_with_keys("V");
        let marked = format!("{LIVE_CHOICE_MARK} V");
        for (entry, command) in [(marked.as_str(), "volts"), ("   Ω", "ohms")] {
            let ctx = egui::Context::default();
            let f = run_frame(&ctx, vec![], &mut draw);
            let combo = centre(&node_with_role(&f.nodes, Role::ComboBox).expect("combo").1);
            click(&ctx, combo, &mut draw);
            let f = run_frame(&ctx, vec![], &mut draw);
            assert!(
                f.nodes
                    .iter()
                    .any(|(_, n)| n.role() == Role::Label && n.value() == Some("Meter keys")),
                "the list has its caption"
            );
            let node = node_labelled(&f.nodes, entry).unwrap_or_else(|| panic!("{entry:?}"));
            let f = click(&ctx, centre(node), &mut draw);
            assert_eq!(f.picked, Some(ReadoutPick::Press(command)), "{entry:?}");
        }
    }

    /// With no key applying — a function no key picks — focus still lands
    /// in the list as it opens, on the first entry.
    #[test]
    fn opening_the_key_list_with_no_live_key_focuses_the_first() {
        let mut draw = draw_with_keys("A");
        let ctx = egui::Context::default();
        let f = open_with_keyboard(&ctx, &mut draw);
        let first = node_id_labelled(&f.nodes, "   V").expect("first entry, unmarked");
        assert_eq!(f.focus, Some(first));
    }
}
