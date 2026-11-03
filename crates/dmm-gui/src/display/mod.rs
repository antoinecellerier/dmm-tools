mod aux_grid;
mod fit;
mod readout;
mod text;

use dmm_lib::alarm::Zone;
use dmm_lib::flags::Flag;
use dmm_lib::measurement::{MeasuredValue, Measurement};
use dmm_lib::text::TextKind;
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    Align, Color32, FontId, Layout, RichText, TextFormat, TextStyle, TextWrapMode, Ui, Vec2,
    WidgetText, vec2,
};

use crate::a11y::{AnnouncePolicy, UiA11yExt};
use crate::theme::ThemeColors;
use aux_grid::{AuxGridMetrics, aux_grid_id, aux_summary_job, show_aux_rows};
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use fit::{GridFit, LastFit, ReadingLayout, pick_layout};
pub use fit::{ReadingFit, ReadingRatios};
pub(crate) use readout::mode_switch_offered;
use readout::{RangeAtRest, show_mode_readout, show_range_readout};
pub use readout::{ReadoutChoices, ReadoutPick};
use std::borrow::Cow;
use text::{
    badge_order, format_value_display, live_region_context, live_region_fingerprint,
    live_region_label,
};
pub(crate) use text::{spoken_amount, spoken_unit};

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

/// What the app adds to the meter's reading: whether a software transform
/// re-expressed it, and where it lies against the alarm's limits. Both get a
/// badge after the meter's own and a phrase in the spoken label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct ReadingState {
    pub(crate) scaled: bool,
    /// An alarm is set, so the flags row keeps room for its badge.
    pub(crate) alarm_set: bool,
    /// Where the reading lies against the limits; `None` while no alarm is
    /// judging it.
    pub(crate) alarm: Option<Zone>,
    /// How often a screen reader hears the reading.
    pub(crate) announce: AnnouncePolicy,
}

impl ReadingState {
    #[cfg(test)]
    pub(crate) const PLAIN: Self = Self {
        scaled: false,
        alarm_set: false,
        alarm: None,
        announce: AnnouncePolicy::DEFAULT,
    };
    #[cfg(test)]
    pub(crate) const SCALED: Self = Self {
        scaled: true,
        ..Self::PLAIN
    };

    /// Whether the reading is past one of the alarm's limits.
    fn past_limit(self) -> bool {
        matches!(self.alarm, Some(Zone::Above | Zone::Below))
    }

    /// The badge of a reading past a limit. Not `HIGH` or `LOW`, which read
    /// as the meter's own flags (`LOW BAT`).
    fn limit_badge(self, l10n: &Localizer) -> Option<Cow<'_, str>> {
        match self.alarm? {
            Zone::Above => Some(tr!(l10n, "reading-badge-above")),
            Zone::Below => Some(tr!(l10n, "reading-badge-below")),
            Zone::Inside => None,
        }
    }

    /// The reading's colour: the error colour while past a limit, beside
    /// the badge that says so.
    fn reading_color(self, tc: &ThemeColors) -> Color32 {
        if self.past_limit() {
            tc.status_error()
        } else {
            tc.reading()
        }
    }
}

/// What every reading layout draws with, whatever the reading: the language,
/// the theme, what the app adds to the reading and what the meter can be
/// switched to.
#[derive(Clone, Copy)]
pub(crate) struct ReadingCtx<'a> {
    pub(crate) l10n: &'a Localizer,
    pub(crate) tc: &'a ThemeColors,
    pub(crate) state: ReadingState,
    pub(crate) choices: ReadoutChoices<'a>,
}

/// Size of the big meter's hint line relative to the notice title above it.
const NOTICE_HINT_SIZE_RATIO: f32 = 0.6;

/// Size of the mode line relative to the reading value above or beside it.
const MODE_SIZE_RATIO: f32 = 0.4;

/// Largest font a readout selector's popup entries use. The readout itself
/// follows the reading — a 200 px big-meter reading puts it at 80 px — but
/// the popup is a list to pick from, and stays list-sized.
const MAX_CHOICE_POPUP_FONT_SIZE: f32 = 18.0;

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
    l10n: &Localizer,
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
            &l10n.main_label(label),
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
fn value_display(m: &Measurement, tc: &ThemeColors, state: ReadingState) -> (String, Color32) {
    match &m.value {
        MeasuredValue::Normal(_) => (format_value_display(m), state.reading_color(tc)),
        MeasuredValue::Overload => (format_value_display(m), tc.status_error()),
        MeasuredValue::NcvLevel(_) | MeasuredValue::NoReading(_) | MeasuredValue::Absent => {
            (format_value_display(m), tc.reading())
        }
    }
}

/// Single-line reading with the mode and range selectors beside it.
///
/// The value and unit form the spoken row; the selectors, controls, sit
/// after it in the same row rather than inside it, so the spoken reading's
/// bounds stop at the value and a screen reader exploring by pointer finds
/// the dropdowns as themselves. `draw_value` paints the value and unit labels.
fn show_reading_line_with_selector(
    ui: &mut Ui,
    cx: &ReadingCtx<'_>,
    m: &Measurement,
    draw_value: impl FnOnce(&mut Ui),
    mode_size: f32,
) -> Option<ReadoutPick> {
    let ReadingCtx {
        l10n,
        state,
        choices,
        ..
    } = *cx;
    ui.horizontal(|ui| {
        ui.spoken_reading(
            state.announce,
            live_region_fingerprint(l10n, Some(m), state, ""),
            live_region_context(Some(m)),
            || live_region_label(l10n, Some(m), state, ""),
            |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                draw_value(ui);
            },
        );
        ui.separator();
        let mode = show_mode_readout(ui, l10n, m, mode_size, choices);
        // This line has never carried a range label, so a meter with no rung
        // to pick keeps the line it has: mode, then the badges.
        let range = show_range_readout(ui, l10n, m, mode_size, choices.range, RangeAtRest::Nothing);
        show_flags(ui, cx, m, mode_size);
        mode.or(range)
    })
    .inner
}

/// Single-line reading with no selector on it: value, unit, mode and flags,
/// all inside the spoken row.
///
/// `mode_size` of 0 is the small text style, the convention `show_flags` uses
/// for the same choice.
fn show_reading_line_plain(
    ui: &mut Ui,
    cx: &ReadingCtx<'_>,
    m: &Measurement,
    draw_value: impl FnOnce(&mut Ui),
    mode_size: f32,
) {
    let ReadingCtx { l10n, state, .. } = *cx;
    ui.spoken_reading(
        state.announce,
        live_region_fingerprint(l10n, Some(m), state, ""),
        live_region_context(Some(m)),
        || live_region_label(l10n, Some(m), state, ""),
        |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            draw_value(ui);
            ui.separator();
            let mut mode = RichText::new(l10n.label(TextKind::Mode, &m.mode))
                .color(ui.visuals().weak_text_color());
            if mode_size > 0.0 {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.3).max(2.0);
                mode = mode.font(FontId::proportional(mode_size));
            } else {
                mode = mode.small();
            }
            ui.label(mode);
            show_flags(ui, cx, m, mode_size);
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
    /// The title a screen reader hears in place of a reading.
    fn title<'b>(&self, l10n: &'b Localizer) -> Cow<'b, str>
    where
        'a: 'b,
    {
        match self {
            Self::Plain => tr!(l10n, "reading-none"),
            Self::Notice { title, .. } => Cow::Borrowed(title),
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
    fn show_text(&self, ui: &mut Ui, l10n: &Localizer, title_size: f32) {
        let Some(block) = self.notice_block(ui, title_size) else {
            ui.label(
                RichText::new(tr!(l10n, "reading-none")).color(ui.visuals().weak_text_color()),
            );
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
/// The placeholder and its caption sit in a spoken row like a reading's, so
/// a screen reader hears the whole notice — the connection issue, not just
/// the placeholder's dashes — from the same node it hears readings from.
/// The row itself is hidden from screen readers, so a notice's hint is said
/// after its title, and its help steps, a tooltip on screen, become the
/// node's description, which a screen reader reads on demand.
fn no_reading_placeholder(
    ui: &mut Ui,
    l10n: &Localizer,
    state: ReadingState,
    text: &NoReadingText<'_>,
    add_contents: impl FnOnce(&mut Ui),
) {
    // Trailing dots are animation, not speech: the waiting notice cycles
    // through one to four of them, and a fingerprint that hashed them would
    // have a screen reader repeat the same sentence every timeout.
    let title = text.title(l10n);
    let title = title.trim_end_matches(['.', ' ']);
    let (hint, help) = match text {
        NoReadingText::Notice { hint, tooltip, .. } => (*hint, *tooltip),
        NoReadingText::Plain => ("", ""),
    };
    let spoken: Cow<'_, str> = if hint.is_empty() {
        Cow::Borrowed(title)
    } else {
        let separator = tr!(l10n, "reading-spoken-separator");
        Cow::Owned(format!("{title}{separator}{hint}"))
    };
    ui.spoken_reading(
        state.announce,
        live_region_fingerprint(l10n, None, state, &spoken),
        live_region_context(None),
        || live_region_label(l10n, None, state, &spoken),
        add_contents,
    );
    if !help.is_empty() {
        ui.ctx()
            .accesskit_node_builder(crate::a11y::reading_region_id(), |node| {
                node.set_description(help);
            });
    }
}

/// Render the primary reading display at the given font size (two-line layout).
///
/// Returns the setting and value the user picked from a selector, if any.
fn show_reading_sized(
    ui: &mut Ui,
    cx: &ReadingCtx<'_>,
    measurement: Option<&Measurement>,
    value_size: f32,
    no_reading: NoReadingText<'_>,
) -> Option<ReadoutPick> {
    let ReadingCtx {
        l10n,
        tc,
        state,
        choices,
    } = *cx;
    let unit_size = value_size;
    let mode_size = value_size * MODE_SIZE_RATIO;

    match measurement {
        Some(m) => {
            let (value_text, value_color) = value_display(m, tc, state);

            ui.spoken_reading(
                state.announce,
                live_region_fingerprint(l10n, Some(m), state, ""),
                live_region_context(Some(m)),
                || live_region_label(l10n, Some(m), state, ""),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(value_label(
                        ui,
                        l10n,
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

            let _ = show_aux_rows(ui, l10n, aux_grid_id(ui), m, mode_size, tc);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = (mode_size * 0.5).max(2.0);
                let mode = show_mode_readout(ui, l10n, m, mode_size, choices);
                // The range has always been a label on this line, so it stays
                // one on a meter that lists no rung to pick.
                let range =
                    show_range_readout(ui, l10n, m, mode_size, choices.range, RangeAtRest::Label);
                show_flags(ui, cx, m, mode_size);
                mode.or(range)
            })
            .inner
        }
        None => {
            no_reading_placeholder(ui, l10n, state, &no_reading, |ui| {
                if no_reading.is_notice() {
                    // A notice stacks under the bars, the shape this layout
                    // already gives a reading and its mode line: an issue
                    // title set beside a scaled-up row of dashes would push
                    // the fitted font down to nothing.
                    ui.vertical(|ui| {
                        no_reading_bars(ui, value_size);
                        no_reading.show_text(ui, l10n, mode_size);
                    });
                } else {
                    no_reading_bars(ui, value_size);
                    no_reading.show_text(ui, l10n, mode_size);
                }
            });
            None
        }
    }
}

/// The value and its unit, as the one-row layouts draw them.
fn show_value_and_unit(ui: &mut Ui, cx: &ReadingCtx<'_>, m: &Measurement, value_size: f32) {
    let ReadingCtx {
        l10n, tc, state, ..
    } = *cx;
    let (value_text, value_color) = value_display(m, tc, state);
    ui.label(value_label(
        ui,
        l10n,
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
    cx: &ReadingCtx<'_>,
    measurement: Option<&Measurement>,
    value_size: f32,
    no_reading: NoReadingText<'_>,
) -> (Option<ReadoutPick>, Vec2) {
    let ReadingCtx {
        l10n, tc, state, ..
    } = *cx;
    let mode_size = value_size * MODE_SIZE_RATIO;
    let grid_id = aux_grid_id(ui);
    let top = ui.cursor().top();
    let line = |ui: &Ui| vec2(ui.min_rect().width(), ui.cursor().top() - top);

    match measurement {
        Some(m) => {
            let draw_value = |ui: &mut Ui| show_value_and_unit(ui, cx, m, value_size);
            let picked = if !cx.choices.any_offered() {
                show_reading_line_plain(ui, cx, m, draw_value, mode_size);
                None
            } else {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = one_row_spacing(mode_size);
                    show_reading_line_with_selector(ui, cx, m, draw_value, mode_size)
                })
                .inner
            };
            let line = line(ui);
            let _ = show_aux_rows(ui, l10n, grid_id, m, mode_size, tc);
            (picked, line)
        }
        None => {
            no_reading_placeholder(ui, l10n, state, &no_reading, |ui| {
                if no_reading.is_notice() {
                    no_reading_bars(ui, value_size);
                    no_reading.show_text(ui, l10n, mode_size);
                } else {
                    ui.label(
                        RichText::new(format!("{} {}", crate::NO_DATA, tr!(l10n, "reading-none")))
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
    cx: &ReadingCtx<'_>,
    m: &Measurement,
    value_size: f32,
) -> (Option<ReadoutPick>, Vec2) {
    let ReadingCtx {
        l10n,
        tc,
        state,
        choices,
    } = *cx;
    let mode_size = value_size * MODE_SIZE_RATIO;
    let grid_id = aux_grid_id(ui);
    let grid = AuxGridMetrics::measure(ui, l10n, m, mode_size, tc).map_or(Vec2::ZERO, |g| {
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
                    // Top, not centre: the spoken row is a nested horizontal,
                    // which starts `interact_size.y` tall where the layout puts
                    // it and grows downwards — centred, it would hang half the
                    // value's height low. From the top it fills the slot exactly.
                    Layout::left_to_right(Align::Min),
                    |ui| {
                        ui.spoken_reading(
                            state.announce,
                            live_region_fingerprint(l10n, Some(m), state, ""),
                            live_region_context(Some(m)),
                            || live_region_label(l10n, Some(m), state, ""),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                show_value_and_unit(ui, cx, m, value_size);
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
                |ui| show_aux_rows(ui, l10n, grid_id, m, mode_size, tc),
            );
            ui.separator();
            // Past the dividers, the readouts space as the line under the
            // value spaces them.
            ui.spacing_mut().item_spacing.x = one_row_spacing(mode_size);
            let mode = show_mode_readout(ui, l10n, m, mode_size, choices);
            let range =
                show_range_readout(ui, l10n, m, mode_size, choices.range, RangeAtRest::Nothing);
            show_flags(ui, cx, m, mode_size);
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
/// `cx.state` is what the app adds to the reading — a software transform,
/// the alarm's verdict — which its badges and the spoken label say.
/// `cx.choices` are the modes and ranges the meter can be switched to; where
/// there is more than one, that readout becomes a selector, and a pick comes
/// back as its setting and id.
pub fn show_reading(
    ui: &mut Ui,
    cx: &ReadingCtx<'_>,
    measurement: Option<&Measurement>,
) -> Option<ReadoutPick> {
    show_reading_sized(
        ui,
        cx,
        measurement,
        BASE_READING_FONT_SIZE,
        NoReadingText::Plain,
    )
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
    cx: &ReadingCtx<'_>,
    measurement: Option<&Measurement>,
    fit: ReadingFit<'_>,
    no_reading: NoReadingText<'_>,
) -> (f32, ReadingRatios, Option<ReadoutPick>) {
    let ReadingCtx {
        l10n, tc, choices, ..
    } = *cx;
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
    let last_id = ui.scope_id().with("big_meter_last_fit");
    let last: Option<LastFit> = ui.data(|d| d.get_temp(last_id));
    // Laid out at the size drawn last, scaled from there: text does not
    // scale exactly, but at the size it was laid out at the estimate is the
    // grid as drawn, so a still window fits it to the pixel. The side panel's
    // size, whose glyphs are cached too, stands in before the first frame.
    let reference = last.map_or(BASE_READING_FONT_SIZE, |l| l.size);
    let grid = measurement
        .and_then(|m| AuxGridMetrics::measure(ui, l10n, m, reference * MODE_SIZE_RATIO, tc))
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
            let (picked, line) = show_reading_beside(ui, cx, m, size);
            measured.row_w = line.x / size;
            measured.row_h = line.y / size;
            picked
        }
        (ReadingLayout::TwoLine, ..) => {
            let picked = show_reading_sized(ui, cx, measurement, size, no_reading);
            measured.w = ui.min_rect().width() / size;
            measured.h = (ui.cursor().top() - before) / size;
            picked
        }
        _ => {
            let (picked, line) = show_reading_inline(ui, cx, measurement, size, no_reading);
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
    cx: &ReadingCtx<'_>,
    measurement: Option<&Measurement>,
) -> Option<ReadoutPick> {
    let ReadingCtx {
        l10n, tc, state, ..
    } = *cx;
    match measurement {
        Some(m) => {
            let value_text = format_value_display(m);
            let draw_value = |ui: &mut Ui| {
                let color = state.reading_color(tc);
                ui.label(value_label(
                    ui,
                    l10n,
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

            let picked = if !cx.choices.any_offered() {
                show_reading_line_plain(ui, cx, m, draw_value, 0.0);
                None
            } else {
                // The selectors and badges at the small text size, which is
                // what `.small()` resolves to for the plain label.
                let small = TextStyle::Small.resolve(ui.style()).size;
                show_reading_line_with_selector(ui, cx, m, draw_value, small)
            };

            // One summary line rather than the grid: the compact layout is
            // the narrow-window one, where a label/value/unit grid would
            // squeeze the reading itself. Hidden from screen readers like the
            // grid: the spoken reading says the sub-values.
            let summary = aux_summary_job(ui, l10n, m, tc);
            if !summary.is_empty() {
                let label = ui.label(summary);
                crate::a11y::hide_from_screen_readers(ui, label.id);
            }
            picked
        }
        None => {
            let none = tr!(l10n, "reading-none");
            no_reading_placeholder(ui, l10n, state, &NoReadingText::Plain, |ui| {
                ui.label(
                    RichText::new(format!("{} {none}", crate::NO_DATA))
                        .font(FontId::monospace(COMPACT_READING_FONT_SIZE))
                        .color(ui.visuals().weak_text_color()),
                );
            });
            None
        }
    }
}

fn show_flags(ui: &mut Ui, cx: &ReadingCtx<'_>, m: &Measurement, font_size: f32) {
    let ReadingCtx {
        l10n, tc, state, ..
    } = *cx;
    let text = |label: &str, color: Color32| {
        let text = RichText::new(label).strong().color(color);
        if font_size > 0.0 {
            text.font(FontId::proportional(font_size))
        } else {
            text.small()
        }
    };
    let badge = |ui: &mut Ui, label: &str, color: Color32| {
        ui.label(text(label, color));
    };

    // `badge_order()` puts the hazard first, and `badge_tone` paints it in the
    // error color rather than the generic warning one — this is the meter
    // telling the user the probes are on a dangerous potential. Labels are
    // `Flag::label()`'s in the user's language, so in English the badge, the
    // recording panel and the CSV flags column all say "HV!". `Flag::Dc` has
    // no label and so paints no badge.
    for (flag, label) in badge_order()
        .filter(|f| m.flags.get(*f))
        .filter_map(|f| l10n.flag(f).map(|l| (f, l)))
    {
        badge(ui, &label, badge_tone(flag, tc));
    }
    // After the meter's own badges, in the same accent as AUTO/HOLD: this is
    // the app's state, not the meter's, and it belongs at the end of the row
    // rather than mixed in among the flags the meter reported.
    if state.scaled {
        badge(ui, &tr!(l10n, "reading-badge-scaled"), tc.accent());
    }
    // While an alarm is set its badge has a slot of its own, as wide as the
    // wider of the two, left blank while the reading is inside: the row
    // stays put as readings cross the limits, rather than jumping, and in a
    // wrapping row a badge can't split across lines.
    if state.alarm_set {
        let color = tc.status_error();
        let galley = |label: &str| {
            WidgetText::from(text(label, color)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextStyle::Body,
            )
        };
        let size = [
            tr!(l10n, "reading-badge-above"),
            tr!(l10n, "reading-badge-below"),
        ]
        .map(|label| galley(&label).size())
        .into_iter()
        .fold(Vec2::ZERO, |a, b| a.max(b));
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_size(size);
            if let Some(label) = state.limit_badge(l10n) {
                ui.add(
                    eframe::egui::Label::new(text(&label, color)).wrap_mode(TextWrapMode::Extend),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::aux_grid::AuxRowRects;
    use super::readout::{MODE_READOUT, RANGE_READOUT, show_choice_readout};
    use super::*;
    use crate::listbox::LIVE_CHOICE_MARK;
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::AuxValue;
    use dmm_lib::protocol::{Choice, MeterKey, Setting};
    use std::borrow::Cow;

    /// Build a sub-value the way the protocols do: digits in `display_raw`,
    /// unit empty when it matches the main reading's.
    /// The English the tests read the display in.
    fn en() -> &'static Localizer {
        static EN: std::sync::LazyLock<std::sync::Arc<Localizer>> =
            std::sync::LazyLock::new(crate::l10n::english);
        &EN
    }

    /// A reading drawn in English, with no transform or alarm on it.
    fn plain<'a>(tc: &'a ThemeColors, choices: ReadoutChoices<'a>) -> ReadingCtx<'a> {
        ReadingCtx {
            l10n: en(),
            tc,
            state: ReadingState::PLAIN,
            choices,
        }
    }

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
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut rects = Vec::new();
        for _ in 0..3 {
            rects.clear();
            let mut out = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
                rects = show_aux_rows(ui, en(), aux_grid_id(ui), m, font_size, &tc);
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

    /// The compact layout's summary line reads as `aux_summary` does and is
    /// coloured as the grid is: values and units in the reading colour (an
    /// overload in the error colour), everything else weak.
    #[test]
    fn the_compact_summary_colours_values_as_the_grid_does() {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(5.0), "V", StatusFlags::default());
        let mut over = aux("Max", "0", "");
        over.value = MeasuredValue::Overload;
        over.elapsed_secs = Some(12);
        m.aux_values = vec![aux("Frequency", "50.01", "Hz"), over];
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut job = LayoutJob::default();
        let mut weak = Color32::TRANSPARENT;
        let mut out = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            job = aux_summary_job(ui, en(), &m, &tc);
            weak = ui.visuals().weak_text_color();
        });
        out.textures_delta.clear();

        assert_eq!(job.text, m.aux_summary());
        // `append` merges neighbours of one format into a section, and the
        // single spaces between value and unit are dropped here.
        let colours: Vec<(&str, Color32)> = job
            .sections
            .iter()
            .map(|s| {
                (
                    &job.text[s.byte_range.start.0..s.byte_range.end.0],
                    s.format.color,
                )
            })
            .filter(|(text, _)| !text.trim().is_empty())
            .collect();
        assert_eq!(
            colours,
            [
                ("Frequency ", weak),
                ("50.01", tc.reading()),
                ("Hz", tc.reading()),
                (", Max ", weak),
                ("OL", tc.status_error()),
                ("V", tc.reading()),
                (" @12s", weak),
            ]
        );
    }
    /// While an alarm is set, the flags row is as wide with the reading
    /// inside the limits as past either one, so nothing after it moves.
    #[test]
    fn the_limit_badge_keeps_its_room() {
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m = Measurement::test_fixture(MeasuredValue::Normal(5.0), "V", StatusFlags::default());
        let width = |alarm: Option<Zone>, alarm_set: bool| {
            let state = ReadingState {
                scaled: false,
                alarm_set,
                alarm,
                ..ReadingState::PLAIN
            };
            let mut width = 0.0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                width = ui
                    .horizontal(|ui| {
                        show_flags(
                            ui,
                            &ReadingCtx {
                                state,
                                ..plain(&tc, Default::default())
                            },
                            &m,
                            14.0,
                        )
                    })
                    .response
                    .rect
                    .width();
            });
            out.textures_delta.clear();
            width
        };
        let inside = width(Some(Zone::Inside), true);
        assert!(inside > width(None, false), "room kept while inside");
        assert_eq!(width(Some(Zone::Above), true), inside);
        assert_eq!(width(Some(Zone::Below), true), inside);
        assert_eq!(width(None, true), inside, "idle or no number");
    }

    /// Rects of the readout row's three kinds of widget — mode label, range
    /// dropdown, AUTO badge — laid out the way the big-meter rows lay them
    /// out, in a headless egui context. `wrapped` picks the two-line row
    /// (`horizontal_wrapped` under the reading) over the inline one (a
    /// `horizontal` after the value and a separator).
    fn layout_readout_row(value_size: f32, wrapped: bool) -> [egui::Rect; 3] {
        let ctx = crate::app::styled_context();
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
                        .scope(|ui| {
                            show_choice_readout(ui, en(), &MODE_READOUT, "DC mA", mode_size, &[])
                        })
                        .response
                        .rect;
                    let range = ui
                        .scope(|ui| {
                            show_choice_readout(
                                ui,
                                en(),
                                &RANGE_READOUT,
                                "220mA",
                                mode_size,
                                &ranges,
                            )
                        })
                        .response
                        .rect;
                    let badge = ui
                        .scope(|ui| show_flags(ui, &plain(&tc, Default::default()), &m, mode_size))
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
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut rect = egui::Rect::NOTHING;
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let id = aux_grid_id(ui);
                rect = ui
                    .scope(|ui| show_aux_rows(ui, en(), id, m, font_size, &tc))
                    .response
                    .rect;
            });
            out.textures_delta.clear();
        }
        rect
    }

    fn grid_metrics(m: &Measurement, font_size: f32) -> (AuxGridMetrics, Vec2) {
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let mut metrics = None;
        let mut min_cell = Vec2::ZERO;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            metrics = AuxGridMetrics::measure(ui, en(), m, font_size, &tc);
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
    /// their text as the node's value, and so does the spoken reading, whose
    /// whole sentence would match: it is the one live node, skipped here.
    fn label_bounds(nodes: &[(NodeId, Node)], needle: &str) -> egui::Rect {
        let (_, node) = nodes
            .iter()
            .find(|(_, n)| n.live().is_none() && n.value().is_some_and(|t| t.contains(needle)))
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
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        for rows in [2, 4] {
            let m = reading_with_rows(rows, 0);
            for size in [36.0_f32, 130.0] {
                let mut nodes = Vec::new();
                for _ in 0..3 {
                    nodes = run_frame(&ctx, Vec::new(), |ui| {
                        show_reading_beside(ui, &plain(&tc, ReadoutChoices::default()), &m, size).0
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

    /// Both one-row layouts measure the line the fit caches for both, so
    /// they must agree on it, with selectors on the line or without: a
    /// disagreement makes each pass undo the one before.
    #[test]
    fn both_one_row_layouts_measure_the_same_line() {
        let ctx = crate::app::styled_context();
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
                                show_reading_beside(ui, &plain(&tc, choices), &m, size).1
                            } else {
                                show_reading_inline(
                                    ui,
                                    &plain(&tc, choices),
                                    Some(&m),
                                    size,
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
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m = reading_with_rows(2, 0);
        // A discarded frame is run again inside the same `run_ui`, so it
        // shows as the closure running twice.
        let passes = |beside: bool| {
            let mut passes = 0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                passes += 1;
                if beside {
                    let _ = show_reading_beside(ui, &plain(&tc, Default::default()), &m, 130.0);
                } else {
                    let _ = show_reading_inline(
                        ui,
                        &plain(&tc, Default::default()),
                        Some(&m),
                        130.0,
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
                &plain(&tc, choices),
                Some(m),
                BASE_READING_FONT_SIZE,
                NoReadingText::Plain,
            ),
            "inline" => {
                show_reading_inline(
                    ui,
                    &plain(&tc, choices),
                    Some(m),
                    BASE_READING_FONT_SIZE,
                    NoReadingText::Plain,
                )
                .0
            }
            "beside" => show_reading_beside(ui, &plain(&tc, choices), m, BASE_READING_FONT_SIZE).0,
            _ => show_reading_compact(ui, &plain(&tc, choices), Some(m)),
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
        draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
    ) -> Frame {
        run_frame_at(ctx, None, events, draw)
    }

    /// [`run_frame`] at input time `time`, for what is paced in real time.
    fn run_frame_at(
        ctx: &egui::Context,
        time: Option<f64>,
        events: Vec<egui::Event>,
        mut draw: impl FnMut(&mut Ui) -> Option<ReadoutPick>,
    ) -> Frame {
        ctx.enable_accesskit();
        let mut picked = None;
        let mut out = ctx.run_ui(
            egui::RawInput {
                events,
                time,
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
            let ctx = crate::app::styled_context();
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
            let ctx = crate::app::styled_context();
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
    /// whose value is the live mode, outside the spoken reading.
    #[test]
    fn the_mode_readout_becomes_a_named_combo_with_choices() {
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        for layout in LAYOUTS {
            let ctx = crate::app::styled_context();
            let f = run_frame(&ctx, vec![], |ui| {
                draw_reading(ui, layout, &m, modes(&choices))
            });
            assert_eq!(f.picked, None, "{layout}: drawing is not picking");
            let nodes = &f.nodes;
            let (combo_id, combo) = node_with_role(nodes, Role::ComboBox)
                .unwrap_or_else(|| panic!("{layout}: no combo box"));
            assert_eq!(combo.label(), Some("Mode"), "{layout}");
            assert_eq!(combo.value(), Some("DC V"), "{layout}");
            let (reading, _) = spoken_reading(nodes, layout);
            assert!(
                !descendants(nodes, reading).contains(combo_id),
                "{layout}: the selector is inside the spoken reading"
            );
        }
    }

    /// The one live node in the frame, the spoken reading, checked for what
    /// lets a screen reader hear it at all: a leaf (children would inherit
    /// its live setting and each announce on its own), a role AccessKit does
    /// not filter out (it drops `GenericContainer`), bounds, and the whole
    /// sentence as its value.
    fn spoken_reading<'a>(nodes: &'a [(NodeId, Node)], what: &str) -> (NodeId, &'a Node) {
        let live: Vec<&(NodeId, Node)> = nodes
            .iter()
            .filter(|(_, n)| n.live().is_some_and(|l| l != egui::accesskit::Live::Off))
            .collect();
        let [(id, node)] = live[..] else {
            panic!("{what}: want exactly one live node, got {}", live.len());
        };
        assert_eq!(
            *id,
            crate::a11y::reading_region_id().accesskit_id(),
            "{what}: the live node is not the spoken reading"
        );
        assert_eq!(node.role(), Role::Label, "{what}");
        assert!(
            node.children().is_empty(),
            "{what}: the spoken reading has children"
        );
        assert!(
            node.bounds().is_some(),
            "{what}: the spoken reading has no bounds"
        );
        (*id, node)
    }

    /// A reading that changes within the interval reaches the screen reader
    /// when the interval runs out, not before.
    #[test]
    fn a_new_value_waits_for_the_interval() {
        let ctx = crate::app::styled_context();
        let reading = |v: f64| {
            let mut m =
                Measurement::test_fixture(MeasuredValue::Normal(v), "V", StatusFlags::default());
            m.display_raw = Some(v.to_string());
            m
        };
        let said = |time: f64, v: f64| {
            let m = reading(v);
            let f = run_frame_at(&ctx, Some(time), vec![], |ui| {
                draw_reading(ui, "two-line", &m, modes(&[]))
            });
            let (_, node) = spoken_reading(&f.nodes, &format!("t={time}"));
            node.value().unwrap_or_default().to_owned()
        };
        let first = said(0.0, 1.234);
        assert!(first.contains("1.234"), "{first:?}");
        assert!(
            said(1.0, 5.678).contains("1.234"),
            "held inside the interval"
        );
        assert!(said(5.1, 5.678).contains("5.678"), "shown once it ran out");
    }

    /// Whether `id` or a node above it is hidden from screen readers.
    fn hidden(nodes: &[(NodeId, Node)], id: NodeId) -> bool {
        let mut at = Some(id);
        while let Some(id) = at {
            let Some((_, node)) = nodes.iter().find(|(nid, _)| *nid == id) else {
                return false;
            };
            if node.is_hidden() {
                return true;
            }
            at = nodes
                .iter()
                .find(|(_, n)| n.children().contains(&id))
                .map(|(pid, _)| *pid);
        }
        false
    }

    /// Every layout, with a selector or without, and the placeholder speaks
    /// from the one leaf: nothing inside the row it lies over is live, and
    /// the labels that draw the reading and its sub-values are hidden from
    /// screen readers, the selector not.
    #[test]
    fn every_layout_speaks_the_reading_from_one_leaf() {
        let mut m = ranged_reading();
        m.aux_values.push(aux("Frequency", "60.000", "Hz"));
        let choices = two_choices();
        for layout in LAYOUTS {
            for (offer, picks) in [("with a selector", modes(&choices)), ("plain", modes(&[]))] {
                let ctx = crate::app::styled_context();
                let f = run_frame(&ctx, vec![], |ui| draw_reading(ui, layout, &m, picks));
                let what = format!("{layout}, {offer}");
                let (leaf, node) = spoken_reading(&f.nodes, &what);
                let said = node.value().unwrap_or_default();
                assert!(
                    said.contains("5.678") && said.contains("DC volts") && said.contains("60.000"),
                    "{what}: {said:?}"
                );
                assert!(
                    !hidden(&f.nodes, leaf),
                    "{what}: the spoken reading is hidden"
                );
                for copy in ["5.678", "60.000"] {
                    let labels: Vec<NodeId> = f
                        .nodes
                        .iter()
                        .filter(|(id, n)| {
                            *id != leaf && n.value().is_some_and(|v| v.contains(copy))
                        })
                        .map(|(id, _)| *id)
                        .collect();
                    assert!(!labels.is_empty(), "{what}: no label draws {copy}");
                    for id in labels {
                        assert!(
                            hidden(&f.nodes, id),
                            "{what}: the {copy} label is not hidden"
                        );
                    }
                }
                if let Some((combo, _)) = node_with_role(&f.nodes, Role::ComboBox) {
                    assert!(!hidden(&f.nodes, *combo), "{what}: the selector is hidden");
                }
            }
        }
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let f = run_frame(&ctx, vec![], |ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&[])),
                None,
                BASE_READING_FONT_SIZE,
                NoReadingText::Plain,
            )
        });
        let (_, node) = spoken_reading(&f.nodes, "placeholder");
        assert_eq!(node.value(), Some("No reading"), "placeholder");
    }

    /// The big meter's connection notice is hidden from screen readers with
    /// the rest of the row, so the spoken reading says its hint after its
    /// title, dots dropped, and carries its help steps as the description.
    #[test]
    fn the_notice_speaks_its_hint_and_describes_its_help() {
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let notice = NoReadingText::Notice {
            title: "Waiting for meter...",
            hint: "Press F11 for details",
            tooltip: "Check the meter is on.",
            color: Color32::YELLOW,
        };
        let f = run_frame(&ctx, vec![], |ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&[])),
                None,
                BASE_READING_FONT_SIZE,
                notice,
            )
        });
        let (_, node) = spoken_reading(&f.nodes, "notice");
        assert_eq!(
            node.value(),
            Some("Waiting for meter, Press F11 for details")
        );
        assert_eq!(node.description(), Some("Check the meter is on."));
    }

    /// Open the popup, see both entries with the live one marked, pick the
    /// other: its id comes back. Drawn at a big-meter size so the popup's
    /// font cap is exercised — an 80 px readout must not open an 80 px list.
    #[test]
    fn picking_another_mode_reports_its_id() {
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&choices)),
                Some(&m),
                200.0,
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
        let ctx = crate::app::styled_context();
        let tc = crate::settings::Settings::default().theme_colors(true);
        let m =
            Measurement::test_fixture(MeasuredValue::Normal(5.678), "V", StatusFlags::default());
        let choices = two_choices();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&choices)),
                Some(&m),
                BASE_READING_FONT_SIZE,
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
            let ctx = crate::app::styled_context();
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
            let ctx = crate::app::styled_context();
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
            let ctx = crate::app::styled_context();
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
            let ctx = crate::app::styled_context();
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
        let ctx = crate::app::styled_context();
        let mut draw = |ui: &mut Ui| draw_reading(ui, "two-line", &m, both);

        run_frame(&ctx, vec![], &mut draw);
        let f = run_frame(&ctx, key(egui::Key::Tab, egui::Modifiers::NONE), &mut draw);
        assert_eq!(f.focus, node_id_labelled(&f.nodes, "Mode"), "first Tab");
        let f = run_frame(&ctx, key(egui::Key::Tab, egui::Modifiers::NONE), &mut draw);
        assert_eq!(f.focus, node_id_labelled(&f.nodes, "Range"), "second Tab");
    }

    /// With **Only while the reading has focus**, Tab reaches the reading
    /// before the selectors, keeps it through a new value and through the
    /// placeholder; the focused reading is never live, the placeholder neither.
    #[test]
    fn focus_only_makes_the_reading_a_tab_stop() {
        let ctx = crate::app::styled_context();
        let modes_offered = two_choices();
        let frame = |m: Option<&Measurement>, events: Vec<egui::Event>| {
            let tc = crate::settings::Settings::default().theme_colors(true);
            let state = ReadingState {
                announce: AnnouncePolicy::new(Some(5), true),
                ..ReadingState::PLAIN
            };
            run_frame(&ctx, events, |ui| {
                show_reading_sized(
                    ui,
                    &ReadingCtx {
                        state,
                        ..plain(&tc, modes(&modes_offered))
                    },
                    m,
                    BASE_READING_FONT_SIZE,
                    NoReadingText::Plain,
                )
            })
        };
        let reading = |v: &str| {
            let mut m = ranged_reading();
            m.display_raw = Some(v.to_string());
            m
        };
        let (a, b) = (reading("1.234"), reading("5.678"));
        let tab = || key(egui::Key::Tab, egui::Modifiers::NONE);
        let reading_id = Some(crate::a11y::reading_region_id().accesskit_id());

        frame(Some(&a), vec![]);
        let f = frame(Some(&a), tab());
        assert_eq!(f.focus, reading_id, "first Tab");
        assert!(
            f.nodes
                .iter()
                .all(|(_, n)| n.live().is_none_or(|l| l == egui::accesskit::Live::Off)),
            "a live node with focus only"
        );
        let f = frame(Some(&b), vec![]);
        assert_eq!(f.focus, reading_id, "a new value keeps the focus");
        let f = frame(None, vec![]);
        assert_eq!(f.focus, reading_id, "the placeholder keeps the focus");
        assert!(
            f.nodes
                .iter()
                .all(|(_, n)| n.live().is_none_or(|l| l == egui::accesskit::Live::Off)),
            "the focused placeholder is live, so read twice"
        );
        frame(Some(&a), vec![]);
        let f = frame(Some(&a), tab());
        assert_eq!(f.focus, node_id_labelled(&f.nodes, "Mode"), "second Tab");
    }

    /// Opening the range list and picking a rung reports it against
    /// `Setting::Range`, so the caller knows which setting to switch.
    #[test]
    fn picking_a_range_reports_its_id() {
        let ctx = crate::app::styled_context();
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
        let ctx = crate::app::styled_context();
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
                &plain(&tc, modes(&choices)),
                Some(&m),
                BASE_READING_FONT_SIZE,
                NoReadingText::Plain,
            )
        };

        let ctx = crate::app::styled_context();
        let f = open_with_keyboard(&ctx, &mut draw);
        let live = node_id_labelled(&f.nodes, LIVE).expect("live entry");
        assert_eq!(f.focus, Some(live), "keyboard open");

        let ctx = crate::app::styled_context();
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
        let ctx = crate::app::styled_context();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&choices)),
                Some(&m),
                BASE_READING_FONT_SIZE,
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
        let ctx = crate::app::styled_context();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&choices)),
                Some(&m),
                BASE_READING_FONT_SIZE,
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
        let ctx = crate::app::styled_context();
        let mut draw = |ui: &mut Ui| {
            show_reading_sized(
                ui,
                &plain(&tc, modes(&choices)),
                Some(&m),
                BASE_READING_FONT_SIZE,
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
            let ctx = crate::app::styled_context();
            let mut draw = |ui: &mut Ui| {
                show_reading_sized(
                    ui,
                    &plain(&tc, modes(&choices)),
                    Some(&m),
                    BASE_READING_FONT_SIZE,
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
                &plain(&tc, keys(KEYS)),
                Some(&m),
                BASE_READING_FONT_SIZE,
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
            let ctx = crate::app::styled_context();
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
            let ctx = crate::app::styled_context();
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
        let ctx = crate::app::styled_context();
        let f = open_with_keyboard(&ctx, &mut draw);
        let first = node_id_labelled(&f.nodes, "   V").expect("first entry, unmarked");
        assert_eq!(f.focus, Some(first));
    }
}
