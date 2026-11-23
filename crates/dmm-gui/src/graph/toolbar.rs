//! The graph's toolbar: the **Plot:** and **Show:** chip groups, the analysis
//! toggles, and the text fields feeding them.

use crate::l10n::key_name;
use std::borrow::Cow;

use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use dmm_shared::{input, tr};
use eframe::egui::{self, Ui};

use super::field::{Mark, Undrawn};
use super::key::{KeyStyle, SAMPLE_STROKE, SAMPLE_WIDTH, line_sample};
use super::{AxisRefusal, Graph, MAX_AXES, SeriesOption, TIME_WINDOWS};
use crate::a11y::ResponseA11yExt;
use crate::text_marks::{self, MarkStyle, TextMark};

/// The cursor readout's text size: the floor `.claude/rules/gui.md` sets.
const READOUT_SIZE: f32 = 11.0;
use crate::app::Sizing;
use crate::theme::ThemeColors;

/// Faint container for one labelled toolbar group (**Plot:**, **Show:**).
///
/// The two groups do different things — one picks the series the graph plots,
/// the other picks which sub-value traces are drawn over it — and inline chips
/// gave a first-time user nothing to tell them apart, or apart from the
/// analysis toggles beside them. Boxing each group makes that grouping
/// visible.
///
/// Both colours come from `Visuals`, so the box follows the theme. The fill is
/// `faint_bg_color`, well clear of `selection.bg_fill`: the frame is only a
/// container, and a selected chip inside it must still read as selected.
///
/// The border is `widgets.noninteractive.bg_stroke` — the same stroke egui
/// draws separators with, and the palette's Border colour. It is decorative
/// under the presets that leave that colour at egui's own grey, landing around
/// 1.6:1 against the panel in dark mode and 1.8:1 in light, under WCAG's 3:1
/// threshold for graphical elements; the High Contrast preset pins a border
/// that clears 3:1. Nothing depends on seeing it either way: the grouping is
/// carried by the **Plot:**/**Show:** caption text inside each box, and every
/// chip states its own group in its accessible name.
fn group_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::new()
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
}

/// Caption naming one toolbar group, sitting on the same line as its chips.
///
/// Body size, not `.small()`: these name a group of controls rather than
/// annotating one, so they sit at the same size as the labels inside it and
/// are set apart by weight and colour instead.
///
/// `add_sized` at `interact_size.y` is what keeps the caption's text level
/// with the chip text, and it is load-bearing rather than cosmetic. A plain
/// `ui.label` here rendered ~2 logical px high. The group frame's content ui
/// inherits the toolbar's `horizontal_wrapped` layout, and when the caption is
/// added the row is only as tall as the space the frame started with.
/// `Layout::next_frame_ignore_wrap` sizes each widget to
/// `max(child_size.y, cursor.height())` and then force-translates it down to
/// the row top ("we always want to expand down, or we will overlap the row
/// above") — so the caption and the chips are both top-aligned, and egui never
/// re-centres the caption once the taller chips grow the row. A
/// `selectable_label` is a `Button`: it takes `interact_size.y` and centres
/// its text in that box, while a bare label's text fills its own shorter box
/// from the top. Giving the caption the chips' height and letting `add_sized`
/// centre it inside closes the gap.
///
/// A plain `Label`, deliberately: the caption must stay `Role::Label` for
/// AccessKit. Sizing it as a button would line it up too, but would announce a
/// control that isn't one.
fn group_caption(ui: &mut Ui, text: &str) {
    let text = egui::RichText::new(text)
        .strong()
        .color(ui.visuals().weak_text_color());
    // Zero width: only the height is being imposed, the label keeps its own.
    let height = ui.spacing().interact_size.y;
    ui.add_sized(egui::vec2(0.0, height), egui::Label::new(text));
}

/// How wide what `add_group` draws is with a row of any length to itself.
///
/// Known before the group is drawn for good, so that one the rest of the
/// row can't hold starts a row of its own. Left where it is, a frame is
/// handed what remains of the row and wraps its chips inside that: a column
/// of them beside the group before it, taller than the plot in a narrow
/// window.
///
/// Measured by laying the group out once more, so its width has the one
/// definition: what egui makes of the widgets. The rehearsal leaves nothing
/// behind. Invisible, it paints nothing, its widgets are disabled and egui
/// keeps them out of the accessibility tree; and it stands far under the
/// window, since a disabled widget still shows its hover text wherever the
/// pointer is over its rect.
fn width_on_one_row(ui: &mut Ui, add_group: impl FnOnce(&mut Ui)) -> f32 {
    const FAR_BELOW: f32 = 1e6;
    let row = egui::Rect::from_min_size(egui::pos2(0.0, FAR_BELOW), egui::vec2(f32::INFINITY, 0.0));
    let mut rehearsal = ui.new_child(egui::UiBuilder::new().max_rect(row).invisible());
    add_group(&mut rehearsal);
    rehearsal.min_rect().width()
}

/// Accessible name for one **Plot:** chip. `None` is the main reading.
///
/// The chips are `selectable_label`s whose visible text is just the
/// sub-value's name, so a screen reader would otherwise announce a **Plot:**
/// chip and a **Show:** chip identically ("T2, button"). egui passes the
/// hover text on only as the description, which a screen reader reads after
/// the name if at all, so the distinction has to be in the name.
/// `series` is the chip's series by name: a sub-value's label, or the name
/// the meter gives its main reading; `None` for an unnamed main reading.
pub(super) fn series_chip_label<'a>(l10n: &'a Localizer, series: Option<&str>) -> Cow<'a, str> {
    match series {
        Some(label) => tr!(l10n, "graph-plot-series.a11y", series = label),
        None => tr!(l10n, "graph-plot-main.a11y"),
    }
}

/// Accessible name for one **Show:** chip. See [`series_chip_label`].
pub(super) fn overlay_chip_label<'a>(l10n: &'a Localizer, label: &str) -> Cow<'a, str> {
    tr!(l10n, "graph-show-chip.a11y", series = label)
}

/// What a **Plot:** or **Show:** chip reads: its series' `label`, with its
/// `unit` where the meter's series are in more than one (`with_units`). The
/// **Plot:** chips then say which one plots hertz, and a **Show:** chip
/// ties its line to an axis, where two traces share one.
pub(super) fn chip_text<'a>(
    l10n: &'a Localizer,
    label: &'a str,
    unit: &str,
    with_units: bool,
) -> Cow<'a, str> {
    if with_units && !unit.is_empty() {
        tr!(l10n, "graph-chip-with-unit", series = label, unit = unit)
    } else {
        Cow::Borrowed(label)
    }
}

/// A graph key as the hovers that name one write it: `M`, `[`, `End`.
///
/// A named key in the window's language ([`crate::l10n::key_name`]), else
/// its symbol where it has one, not `ctx.format_shortcut`: away from macOS
/// that spells a bare key by name, so `[` would read "OpenBracket".
fn graph_key(l10n: &Localizer, key: egui::Key) -> std::borrow::Cow<'_, str> {
    key_name(l10n, key).unwrap_or(std::borrow::Cow::Borrowed(key.symbol_or_name()))
}

/// The two keys that step through the time windows, as their hovers say.
fn cycle_keys() -> String {
    format!(
        "{} / {}",
        egui::Key::OpenBracket.symbol_or_name(),
        egui::Key::CloseBracket.symbol_or_name()
    )
}

/// One toolbar chip: a `selectable_label` showing `on`'s state, with hover
/// text, reporting whether it was clicked and changing nothing itself.
///
/// What the click does is the caller's, so that a chip whose flip carries
/// side effects can run the same method its keyboard shortcut does.
fn chip(ui: &mut Ui, on: bool, label: &str, hover: &str) -> bool {
    ui.selectable_label(on, label)
        .on_hover_text(hover)
        .clicked()
}

/// How wide a chip showing `text` is, known before it is drawn, so a row
/// can tell whether a group would wrap.
fn chip_width(ui: &Ui, text: impl Into<egui::WidgetText>) -> f32 {
    text.into()
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        )
        .size()
        .x
        + 2.0 * ui.spacing().button_padding.x
}

/// What the time-window list shows: the preset's own label, or for a span
/// the minimap or a zoom set, that span as the time axis writes it, to a
/// tenth of a second under ten.
fn window_label(secs: f64, preset: Option<usize>) -> String {
    match preset {
        Some(i) => TIME_WINDOWS[i].1.to_string(),
        None if secs < 10.0 => format!("{secs:.1}s"),
        None => super::time::format_time_label(secs),
    }
}

/// The gap between the first row's groups.
const VIEW_GAP: f32 = 6.0;

/// The width a run of widgets takes on one line, added up as they are drawn,
/// whichever lines a narrow row wraps them onto.
struct RowWidth {
    width: f32,
    spacing: f32,
}

impl RowWidth {
    fn new(ui: &Ui) -> Self {
        Self {
            width: 0.0,
            spacing: ui.spacing().item_spacing.x,
        }
    }

    /// A widget, and the gap after it.
    fn add(&mut self, response: &egui::Response) {
        self.width += response.rect.width() + self.spacing;
    }

    /// Space added between widgets.
    fn space(&mut self, width: f32) {
        self.width += width;
    }

    /// The run's width, without the gap after its last widget.
    fn width(&self) -> f32 {
        (self.width - self.spacing).max(0.0)
    }
}

/// Room a chip's line sample has around it on its ground, each side.
const SAMPLE_PAD: egui::Vec2 = egui::vec2(4.0, 5.0);

/// A chip's line sample with its ground.
fn sample_size() -> egui::Vec2 {
    egui::vec2(SAMPLE_WIDTH, SAMPLE_STROKE) + 2.0 * SAMPLE_PAD
}

/// A **Plot:** or **Show:** chip carrying a sample of its trace's line left
/// of its `label`, so the chips are the graph's key.
///
/// The sample lies on a patch of the plot's background, not on the chip's
/// fill: a line's colour is picked to show on the plot, and against a
/// selected chip's fill most of them fall under the 3:1 a graphic needs —
/// the default dark theme's plotted line among them. On its own ground the
/// sample is the line as the plot draws it, whatever the chip's state and
/// the palette.
///
/// One `Button` with the sample as a painted atom, not a row of two
/// widgets: it wraps in the toolbar as one unit, and its accessible name is
/// its text alone.
fn sampled_chip<'a>(
    selected: bool,
    label: &'a str,
    ground: egui::Color32,
    (color, style): (egui::Color32, egui_plot::LineStyle),
) -> egui::Button<'a> {
    let sample = egui::Atom::paint(sample_size(), move |ui, args| {
        let painter = ui.painter();
        painter.rect_filled(args.rect, 2.0, ground);
        let left = args.rect.left_center() + egui::vec2(SAMPLE_PAD.x, 0.0);
        painter.extend(line_sample(left, color, style));
    });
    egui::Button::selectable(selected, (sample, label))
}

/// A [`chip`] over a plain flag, flipped in place when clicked.
///
/// Returns whether it was clicked, so a caller with extra work to do on the
/// transition can see it — the flag is already flipped by then, so such a
/// caller tests the *new* state.
fn toggle_chip(ui: &mut Ui, on: &mut bool, label: &str, hover: &str) -> bool {
    let clicked = chip(ui, *on, label, hover);
    if clicked {
        *on = !*on;
    }
    clicked
}

/// One toolbar text entry: a fixed-width single-line edit with hint and hover
/// text, under `id`, whose focus a field asks before it is drawn. Parsing
/// stays at the call site — each field accepts a different shape of value —
/// and so does the response, which the reference field also takes the focus
/// from.
fn text_field(
    ui: &mut Ui,
    id: egui::Id,
    text: &mut String,
    width: f32,
    hint: &str,
    hover: &str,
) -> egui::Response {
    let resp = ui.add(text_edit(id, text, width, hint, ui));
    field_cues(ui, resp, hover)
}

/// The edit [`text_field`] draws, for a field that lays its text out itself.
fn text_edit<'t>(
    id: egui::Id,
    text: &'t mut String,
    width: f32,
    hint: &str,
    ui: &Ui,
) -> egui::TextEdit<'t> {
    egui::TextEdit::singleline(text)
        .id(id)
        .desired_width(width)
        // The buttons' padding rather than egui's 2 pt, so a field
        // showing up beside them doesn't make its row taller and shift
        // the graph below.
        .margin(ui.spacing().button_padding)
        .hint_text(hint)
}

/// A [`text_field`] holding one number, its entry marked when it doesn't
/// read as one, which leaves the value in force as it is
/// ([`crate::text_marks`]); the hover says why.
#[allow(
    clippy::too_many_arguments,
    reason = "text_field's six, and what judging a number needs"
)]
fn number_field(
    ui: &mut Ui,
    id: egui::Id,
    text: &mut String,
    width: f32,
    hint: &str,
    hover: &str,
    l10n: &Localizer,
    error: egui::Color32,
) -> egui::Response {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let margin = ui.spacing().button_padding;
    let judge = |t: &str| {
        let t = t.trim();
        let reads = t.is_empty() || t.parse::<f64>().is_ok_and(f64::is_finite);
        (!reads).then(|| (MarkStyle::Unread, input::not_a_number(l10n, t).into_owned()))
    };
    let (output, why) = text_marks::show_judged(
        ui,
        text,
        // As `text_edit` sets one up.
        |edit| {
            edit.id(id)
                .desired_width(width)
                .margin(margin)
                .hint_text(hint)
        },
        &font_id,
        error,
        &judge,
    );
    let hover = match why {
        Some(why) => format!("{hover}\n{why}"),
        None => hover.to_string(),
    };
    field_cues(ui, output.response.response, &hover)
}

/// The focus ring and hover text every toolbar field has.
fn field_cues(ui: &Ui, resp: egui::Response, hover: &str) -> egui::Response {
    // egui's own focused frame is invisible under a pinned Accent; the ring
    // is the field's only keyboard cue then (`a11y::paint_focus_ring`).
    crate::a11y::paint_focus_ring(ui, &resp);
    resp.on_hover_text(hover)
}

/// The hover text's line for an entry that draws no line, saying why.
fn mark_text(l10n: &Localizer, text: &str, m: &Mark) -> String {
    let entry = &text[m.span.clone()];
    match &m.why {
        Undrawn::NotANumber => tr!(l10n, "graph-ref-not-a-number", entry = entry).into_owned(),
        Undrawn::UnknownUnit => tr!(l10n, "graph-ref-unknown-unit", entry = entry).into_owned(),
        Undrawn::OtherQuantity(unit) if unit.is_empty() => {
            tr!(l10n, "graph-ref-other-quantity-unitless", entry = entry).into_owned()
        }
        Undrawn::OtherQuantity(unit) => tr!(
            l10n,
            "graph-ref-other-quantity",
            entry = entry,
            unit = unit.as_str()
        )
        .into_owned(),
    }
}

/// The toolbar fields' ids. Those a decade step re-expresses say by their
/// focus whether their text can be rewritten (see `field.rs`).
pub(super) const Y_MIN_FIELD: &str = "graph-y-min-field";
pub(super) const Y_MAX_FIELD: &str = "graph-y-max-field";
pub(super) const REF_FIELD: &str = "graph-ref-field";
const ENVELOPE_FIELD: &str = "graph-envelope-window-field";

impl Graph {
    /// Whether the toolbar's series row has anything to show: a sub-value to
    /// pick from (**Plot:**) or a trace to draw beside the plotted series
    /// (**Show:**). False for single-display meters, which keep the two-row
    /// toolbar they had before sub-values existed.
    pub(super) fn has_series_row(&self) -> bool {
        !self.series_options.is_empty() || !self.overlays.is_empty()
    }

    /// Render the toolbar. Row 1: time presets, LIVE, Y-axis, Reset Zoom.
    /// Row 2, present only for meters that send sub-values: the boxed
    /// **Plot:** series selector and **Show:** trace toggles. Row 3: the
    /// analysis overlays (Mean, Min/Max, Histogram, Ref, Cursors).
    ///
    /// The series controls get a row of their own rather than sharing one
    /// with the analysis toggles: inline, nothing distinguished the two kinds
    /// of control for a user who didn't already know them. The rows now read
    /// view → what is plotted → what is drawn over it. Each uses
    /// `horizontal_wrapped` so items wrap instead of clipping.
    pub fn show_toolbar(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        // Captions ("Plot:") and labels level with the chips beside them;
        // the plot below keeps the column's own row height.
        let row = ui.spacing().interact_size.y;
        Sizing::rows_fit_controls(ui);
        self.show_toolbar_rows(ui, l10n, tc);
        ui.spacing_mut().interact_size.y = row;
    }

    fn show_toolbar_rows(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        // Row 1
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = Sizing::of(ui.ctx()).gap(2.0);
            self.show_view_row(ui, l10n, tc);
        });

        // Row 2: what is plotted, and what is drawn beside it. Skipped
        // entirely for single-display meters, which keep the two-row toolbar.
        if self.has_series_row() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = Sizing::of(ui.ctx()).gap(2.0);
                self.show_series_selector(ui, l10n, tc);
                self.show_overlay_toggles(ui, l10n, tc);
            });
        }

        // Row 3
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = Sizing::of(ui.ctx()).gap(2.0);
            self.show_analysis_row(ui, l10n, tc);
        });
    }

    /// The toolbar's first row: the time windows, LIVE, the Y axis's Auto or
    /// Fixed with its range, and Reset Zoom.
    ///
    /// While the row would wrap, the windows' chips collapse into one list.
    /// Decided from the chips' text and the rest of the row as last frame
    /// drew it, which the list leaves as it is: the choice never flips back
    /// and forth.
    fn show_view_row(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        let key = |k| graph_key(l10n, k);
        let cycle_keys = cycle_keys();
        let spacing = ui.spacing().item_spacing.x;
        let chips: f32 = TIME_WINDOWS
            .iter()
            .map(|&(_, label)| chip_width(ui, label) + spacing)
            .sum();
        let room = ui.available_size_before_wrap().x;
        // The rest counts the gap it starts with.
        if chips + self.view_row_rest > room {
            self.show_window_list(ui, l10n, &cycle_keys);
        } else {
            for &(secs, label) in TIME_WINDOWS {
                let tooltip = tr!(
                    l10n,
                    "graph-window.hover",
                    window = label,
                    keys = &cycle_keys
                );
                if ui
                    .selectable_label((self.time_window_secs - secs).abs() < 0.1, label)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.set_time_window(secs);
                }
            }
        }
        let mut rest = RowWidth::new(ui);

        ui.add_space(VIEW_GAP);
        rest.space(VIEW_GAP);

        // On: the selected fill and text colour come from `selected`, like the
        // presets beside it. Off: dimmed text in a plain frame, so it still
        // reads as a button.
        let live_text = egui::RichText::new(tr!(l10n, "graph-live")).small();
        let live_text = if self.live {
            live_text
        } else {
            live_text.color(ui.visuals().weak_text_color())
        };
        let live_resp = ui
            .add(egui::Button::new(live_text).selected(self.live))
            .on_hover_text(tr!(l10n, "graph-live.hover", keys = key(egui::Key::End)));
        rest.add(&live_resp);
        if live_resp.clicked() {
            if self.live {
                self.leave_live();
            } else {
                self.set_live(true);
            }
        }

        ui.add_space(VIEW_GAP);
        rest.space(VIEW_GAP);

        let (y_label, y_tooltip) = if self.y_axis.fixed {
            (tr!(l10n, "graph-y-fixed"), tr!(l10n, "graph-y-fixed.hover"))
        } else {
            (tr!(l10n, "graph-y-auto"), tr!(l10n, "graph-y-auto.hover"))
        };
        // The chip has already flipped the flag, so `self.y_axis.fixed` below
        // reads "fixed mode was just switched on".
        let y_chip = ui
            .selectable_label(self.y_axis.fixed, y_label.as_ref())
            .on_hover_text(y_tooltip.as_ref());
        rest.add(&y_chip);
        if y_chip.clicked() {
            self.y_axis.fixed = !self.y_axis.fixed;
            if self.y_axis.fixed {
                self.pin_y_axes_to_view();
            }
        }
        if self.y_axis.fixed {
            let field_width = 50.0;
            // A decade step rewrites a field's text only once it isn't being
            // typed in.
            let ids = (egui::Id::unique(Y_MIN_FIELD), egui::Id::unique(Y_MAX_FIELD));
            if !ui.memory(|m| m.has_focus(ids.0)) {
                self.y_axis.min.flush();
            }
            if !ui.memory(|m| m.has_focus(ids.1)) {
                self.y_axis.max.flush();
            }
            let error = tc.status_error();
            let min = number_field(
                ui,
                ids.0,
                self.y_axis.min.text_mut(),
                field_width,
                &tr!(l10n, "graph-y-min"),
                &tr!(l10n, "graph-y-min.hover"),
                l10n,
                error,
            );
            rest.add(&min);
            rest.add(
                &ui.label(
                    egui::RichText::new("..")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                ),
            );
            let max = number_field(
                ui,
                ids.1,
                self.y_axis.max.text_mut(),
                field_width,
                &tr!(l10n, "graph-y-max"),
                &tr!(l10n, "graph-y-max.hover"),
                l10n,
                error,
            );
            rest.add(&max);
            let (changed_min, changed_max) = (min.changed(), max.changed());
            // The unit the bounds are in, the plotted one: it moves a prefix
            // with an auto-range step, and the numbers with it.
            if let Some(unit) = self.plotted_unit_if_any().filter(|u| !u.is_empty()) {
                rest.add(
                    &ui.label(
                        egui::RichText::new(unit)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    ),
                );
            }
            if changed_min && self.y_axis.min.parse() {
                self.y_axis.user_set = true;
            }
            if changed_max && self.y_axis.max.parse() {
                self.y_axis.user_set = true;
            }
        }

        ui.add_space(VIEW_GAP);
        rest.space(VIEW_GAP);

        let zoomed = self.is_view_zoomed();
        let reset = ui
            .add_enabled(zoomed, egui::Button::new(tr!(l10n, "graph-reset-zoom")))
            .on_hover_text(tr!(l10n, "graph-reset-zoom.hover"))
            .on_disabled_hover_text(tr!(l10n, "graph-reset-zoom.hover-disabled"));
        rest.add(&reset);
        if reset.clicked() {
            self.reset_view();
        }
        // Next frame's choice between the windows' chips and their list;
        // drawn again at once when it moves, rather than at the next input.
        let rest = rest.width();
        if (rest - self.view_row_rest).abs() > 0.5 {
            self.view_row_rest = rest;
            ui.ctx().request_repaint();
        }
    }

    /// The time windows as one list, for a row too narrow for their chips.
    fn show_window_list(&mut self, ui: &mut Ui, l10n: &Localizer, cycle_keys: &str) {
        let current = TIME_WINDOWS
            .iter()
            .position(|&(secs, _)| (self.time_window_secs - secs).abs() < 0.1);
        let shown = window_label(self.time_window_secs, current);
        let (response, picked) = ui
            .scope(|ui| {
                // The arrow at the text's height, not Touch's checkbox size,
                // which would make the box taller than the chips beside it.
                let text = ui.text_style_height(&egui::TextStyle::Button);
                let spacing = ui.spacing_mut();
                spacing.icon_width = spacing.icon_width.min(text);
                crate::listbox::listbox_dropdown(
                    ui,
                    "graph-window-list",
                    || 0,
                    |combo| egui::ComboBox::width(combo, 0.0).selected_text(shown),
                    |ui, was_open| {
                        let mut picked = None;
                        let mut entries = Vec::with_capacity(TIME_WINDOWS.len());
                        for (i, &(_, label)) in TIME_WINDOWS.iter().enumerate() {
                            let on = current == Some(i);
                            // Marked as the readouts' lists mark theirs, not
                            // by the fill alone.
                            let entry = ui
                                .selectable_label(on, crate::listbox::choice_entry_text(label, on));
                            if !was_open && (on || (current.is_none() && i == 0)) {
                                entry.request_focus();
                            }
                            if entry.clicked() {
                                picked = Some(i);
                            }
                            entries.push(entry);
                        }
                        crate::listbox::navigate_choice_entries(ui.ctx(), &entries);
                        picked
                    },
                )
            })
            .inner;
        response
            .on_hover_text(tr!(l10n, "graph-window-list.hover", keys = cycle_keys))
            .a11y_label(&tr!(l10n, "graph-window-list.a11y"));
        if let Some(i) = picked {
            self.set_time_window(TIME_WINDOWS[i].0);
        }
    }

    /// The toolbar's analysis row: Mean, Min/Max with its window,
    /// Histogram, Ref with its values and Triggers, and Cursors with their
    /// readout.
    fn show_analysis_row(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        let key = |k| graph_key(l10n, k);
        toggle_chip(
            ui,
            &mut self.show_mean,
            &tr!(l10n, "graph-mean"),
            &tr!(l10n, "graph-mean.hover", keys = key(egui::Key::M)),
        );
        toggle_chip(
            ui,
            &mut self.show_envelope,
            &tr!(l10n, "graph-envelope"),
            &tr!(l10n, "graph-envelope.hover", keys = key(egui::Key::X)),
        );
        if self.show_envelope {
            let changed = text_field(
                ui,
                egui::Id::unique(ENVELOPE_FIELD),
                self.envelope_window.text_mut(),
                30.0,
                &tr!(l10n, "graph-envelope-window"),
                &tr!(l10n, "graph-envelope-window.hover"),
            )
            .changed();
            if changed {
                // A zero or negative window would bucket every sample together,
                // so it is left as a draft.
                self.envelope_window.parse_if(|v| v > 0.0);
            }
            ui.label(
                egui::RichText::new("s")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
        let hover = self.histogram_hover(l10n, &key(egui::Key::H));
        let label = tr!(l10n, "graph-histogram");
        toggle_chip(ui, &mut self.show_histogram, &label, &hover);
        if chip(
            ui,
            self.show_ref_line,
            &tr!(l10n, "graph-ref"),
            &tr!(l10n, "graph-ref.hover", keys = key(egui::Key::R)),
        ) {
            self.toggle_ref_lines();
        }
        if self.show_ref_line {
            let id = egui::Id::unique(REF_FIELD);
            if !ui.memory(|m| m.has_focus(id)) {
                let plotted = self.plotted_unit_if_any().map(str::to_string);
                self.ref_lines.flush(plotted.as_deref());
            }
            let units = self.shown_units();
            let known = |unit: &str| units.iter().any(|u| u == unit);
            let marker = self.ref_lines.marker();
            let weak = ui.visuals().weak_text_color();
            let font_id = egui::TextStyle::Body.resolve(ui.style());
            let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                let marks: Vec<TextMark> = marker
                    .marks(text.as_str(), known)
                    .iter()
                    .map(Mark::text_mark)
                    .collect();
                let job = text_marks::marked_job(ui, &font_id, text.as_str(), &marks, weak);
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let hint = tr!(l10n, "graph-ref-values");
            let output = text_edit(id, self.ref_lines.text_mut(), 120.0, &hint, ui)
                .layouter(&mut layouter)
                .show(ui);
            if output.response.changed() {
                let plotted = self.plotted_unit_if_any().map(str::to_string);
                self.ref_lines.parse(plotted.as_deref());
            }
            let text = self.ref_lines.text();
            // The entry the caret is in may be half typed: nothing is said
            // of it until it is finished, under the text or in the hover.
            let caret = text_marks::caret(&output, text);
            let mut marks = self.ref_lines.marker().marks(text, known);
            marks.retain(|m| !m.text_mark().held_back(caret));
            let drawn: Vec<TextMark> = marks.iter().map(Mark::text_mark).collect();
            text_marks::underline(ui, &output, text, &drawn, tc.status_error(), weak);
            let mut hover = tr!(l10n, "graph-ref-values.hover").into_owned();
            for m in &marks {
                hover.push('\n');
                hover.push_str(&mark_text(l10n, text, m));
            }
            let resp = field_cues(ui, output.response.response, &hover);
            if std::mem::take(&mut self.focus_ref_field) {
                resp.request_focus();
            }
            toggle_chip(
                ui,
                &mut self.show_crossings,
                &tr!(l10n, "graph-triggers"),
                &tr!(l10n, "graph-triggers.hover", keys = key(egui::Key::T)),
            );
        }
        if chip(
            ui,
            self.cursors.active,
            &tr!(l10n, "graph-cursors"),
            &tr!(l10n, "graph-cursors.hover", keys = key(egui::Key::C)),
        ) {
            self.cursors.toggle();
        }
        if self.cursors.active {
            self.show_cursor_readout(ui, l10n, tc);
        }
    }

    /// The cursors' readout beside their chip, in their colour and written
    /// as Statistics' wrapped lines are, or how to place them until both are.
    fn show_cursor_readout(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        if !self.cursor_readout.is_empty() {
            let color = tc.graph_cursor_delta();
            const GAP: f32 = 6.0;
            // Monospace a point under egui's: Hack's glyphs stand as tall at
            // 11 pt as the chips' Ubuntu Light at 12.5, where at 12 they read
            // a size larger.
            let font = egui::FontId::monospace(READOUT_SIZE);
            let galleys: Vec<_> = self
                .cursor_readout
                .iter()
                .map(|text| {
                    egui::WidgetText::from(
                        egui::RichText::new(text).font(font.clone()).color(color),
                    )
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::TextStyle::Monospace,
                    )
                })
                .collect();
            // The readout stays together: on the chips' row while it fits
            // there, else on a row of its own, and only a window too narrow for
            // that splits it, between two figures.
            let between = GAP + 2.0 * ui.spacing().item_spacing.x;
            let width = galleys.iter().map(|g| g.size().x).sum::<f32>()
                + between * galleys.len().saturating_sub(1) as f32;
            // Set off from the chip by the gap between two figures, while it
            // shares the chip's row.
            let own_row = width + between > ui.available_size_before_wrap().x;
            if own_row {
                ui.end_row();
            }
            for (k, galley) in galleys.into_iter().enumerate() {
                if k > 0 || !own_row {
                    ui.add_space(GAP);
                }
                ui.add(egui::Label::new(galley));
            }
        } else {
            ui.label(
                egui::RichText::new(tr!(l10n, "graph-cursors-hint"))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    }

    /// The main reading's unit, where the graph knows it: it is plotted, or
    /// kept beside the plotted series. Empty otherwise.
    fn main_unit(&self) -> &str {
        if self.current_series.is_none() {
            return &self.current_unit;
        }
        let main = self.main_name();
        self.overlays
            .iter()
            .find(|o| o.label == main)
            .map_or("", |o| o.unit.as_str())
    }

    /// Whether the meter's series are in more than one unit, so that a chip
    /// has to say which is its own ([`chip_text`]).
    pub(super) fn series_in_several_units(&self) -> bool {
        let mut units = std::iter::once(self.current_unit.as_str())
            .chain(self.series_options.iter().map(|s| s.unit.as_str()))
            .chain(self.overlays.iter().map(|o| o.unit.as_str()))
            .filter(|unit| !unit.is_empty());
        units
            .next()
            .is_some_and(|first| units.any(|unit| unit != first))
    }

    /// Chips choosing which series the graph plots: the meter's main reading
    /// or any sub-value it is currently sending.
    ///
    /// Only rendered when the meter sends sub-values, so single-display
    /// meters see the toolbar exactly as before. `selectable_label` rather
    /// than a combo box because that is the toolbar's idiom throughout — and
    /// with at most five choices the chips stay readable.
    ///
    /// The chip of the series drawn as the plotted line carries its sample:
    /// it is the only thing naming that line. The others carry none: picking
    /// one doesn't draw the line its series has now, and a series drawn
    /// beside the plotted one has its sample on its **Show:** chip.
    fn show_series_selector(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        if self.series_options.is_empty() {
            return;
        }

        // `None` = the main reading, `Some(i)` = `series_options[i]`. Applied
        // after the loop so the click handler doesn't need a mutable borrow
        // while the option list is being read.
        let mut choice: Option<Option<usize>> = None;

        let frame = group_frame(ui);
        frame.show(ui, |ui| {
            group_caption(ui, &tr!(l10n, "graph-plot"));
            let plotted = KeyStyle::Plotted.line(tc, self.solid_lines);
            let look = (tc.plot_background(), plotted);
            // The sample goes with the series drawn, the selected look with
            // the one chosen: they part while a choice waits for its first
            // reading, as it does while paused.
            fn chip(
                (selected, drawn): (bool, bool),
                text: &str,
                (ground, plotted): (egui::Color32, (egui::Color32, egui_plot::LineStyle)),
            ) -> egui::Button<'_> {
                match drawn {
                    true => sampled_chip(selected, text, ground, plotted),
                    false => egui::Button::selectable(selected, text),
                }
            }

            // `Role::RadioButton` rather than the default button role: exactly
            // one chip is selected at a time, and egui maps its own radio
            // widgets to that role with a toggled state, so AT handling of the
            // pairing is proven.
            let with_units = self.series_in_several_units();
            let main_selected = self.selected_series.is_none();
            let main_text = self.main_text(l10n);
            let main_chip = chip_text(l10n, &main_text, self.main_unit(), with_units);
            let named = self.main_label.is_some().then_some(main_text.as_ref());
            let main_hover = match named {
                Some(name) => tr!(l10n, "graph-plot-main-named.hover", series = name),
                None => tr!(l10n, "graph-plot-main.hover"),
            };
            if ui
                .add(chip(
                    (main_selected, self.current_series.is_none()),
                    main_chip.as_ref(),
                    look,
                ))
                .on_hover_text(main_hover)
                .a11y_label(&series_chip_label(l10n, named))
                .a11y_role(egui::accesskit::Role::RadioButton)
                .clicked()
            {
                choice = Some(None);
            }

            for (i, SeriesOption { label, unit, .. }) in self.series_options.iter().enumerate() {
                let selected = self.selected_series.as_deref() == Some(label.as_str());
                let text = l10n.label(TextKind::Aux, label);
                if ui
                    .add(chip(
                        (
                            selected,
                            self.current_series.as_deref() == Some(label.as_str()),
                        ),
                        chip_text(l10n, &text, unit, with_units).as_ref(),
                        look,
                    ))
                    .on_hover_text(tr!(
                        l10n,
                        "graph-plot-series.hover",
                        series = text.as_ref(),
                        unit = unit
                    ))
                    .a11y_label(&series_chip_label(l10n, Some(&text)))
                    .a11y_role(egui::accesskit::Role::RadioButton)
                    .clicked()
                {
                    choice = Some(Some(i));
                }
            }
        });

        // A series kept beside the plotted one changes places with it now, so
        // the switch shows with no reading coming (paused, a replay that has
        // ended). One not kept yet takes over at its next sample:
        // `push_sample` restarts the graph on it.
        match choice {
            Some(None) => self.selected_series = None,
            Some(Some(i)) => self.selected_series = Some(self.series_options[i].label.clone()),
            None => {}
        }
        if choice.is_some() {
            self.swap_to_selected();
            ui.ctx().request_repaint();
        }
    }

    /// Chips choosing which sub-value traces are drawn beside the plotted
    /// series, in its unit or on an axis of their own.
    ///
    /// Each chip carries its trace's line sample, shown or not: the chip
    /// says which line a click hides, or which one it will draw. Hiding a
    /// trace stops it being drawn but not recorded — turning it back on
    /// brings its history with it.
    /// A trace whose unit can't have an axis says why: with every axis taken
    /// its chip is unlit and disabled, while one shed for width stays as the
    /// user left it.
    fn show_overlay_toggles(&mut self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) {
        if self.overlays.is_empty() {
            return;
        }

        // Beside the **Plot:** group while it fits there whole, else on a
        // row of its own ([`width_on_one_row`]).
        if !self.series_options.is_empty() {
            // The gap between the two groups, where they share a row: after
            // a **Plot:** group filling its row it would overhang the toolbar.
            const GROUP_GAP: f32 = 6.0;
            let group = width_on_one_row(ui, |ui| {
                self.overlay_group(ui, l10n, tc);
            });
            if GROUP_GAP + group > ui.available_size_before_wrap().x {
                ui.end_row();
            } else {
                ui.add_space(GROUP_GAP);
            }
        }

        // Applied once the group is drawn: the click handler would otherwise
        // need a mutable borrow while the overlay list is being read.
        if let Some(label) = self.overlay_group(ui, l10n, tc) {
            self.click_overlay_chip(label);
        }
    }

    /// The **Show:** group: its frame, its caption and a chip for each
    /// trace. Returns the label of the trace whose chip was clicked.
    fn overlay_group(&self, ui: &mut Ui, l10n: &Localizer, tc: &ThemeColors) -> Option<String> {
        let mut toggled = None;
        let with_units = self.series_in_several_units();
        group_frame(ui).show(ui, |ui| {
            group_caption(ui, &tr!(l10n, "graph-show"));

            let ground = tc.plot_background();
            for (k, o) in self.overlays.iter().enumerate() {
                let label = self.series_text(l10n, &o.label);
                let label = label.as_ref();
                let text = chip_text(l10n, label, &o.unit, with_units);
                let refusal = self.axis_refusal(o);
                let full = refusal == Some(AxisRefusal::Full);
                let shown = !full && !self.is_hidden(o);
                let state = match refusal {
                    Some(AxisRefusal::Full) => "full",
                    Some(AxisRefusal::Narrow) if shown => "narrow",
                    _ if shown => "shown",
                    _ => "hidden",
                };
                let hover = tr!(
                    l10n,
                    "graph-show-chip.hover",
                    state = state,
                    series = label,
                    max = MAX_AXES
                );
                // Toggle buttons, unlike the mutually exclusive **Plot:**
                // chips — but with the same naming problem, so the group goes
                // in the accessible name here too.
                let line = KeyStyle::Overlay(k).line(tc, self.solid_lines);
                if ui
                    .add_enabled(!full, sampled_chip(shown, &text, ground, line))
                    .on_hover_text(hover.clone())
                    .on_disabled_hover_text(hover)
                    .a11y_label(&overlay_chip_label(l10n, label))
                    .clicked()
                {
                    toggled = Some(o.label.clone());
                }
            }
        });
        toggled
    }

    /// Flip the reference lines, taking the caret into the values field when
    /// they come on: the field is where the lines are actually chosen, and
    /// on an empty one the chip alone draws nothing.
    ///
    /// Shared by the **Ref** chip and its key, as [`super::MeasureCursors::toggle`]
    /// is — the two paths would otherwise be free to drift apart.
    pub(super) fn toggle_ref_lines(&mut self) {
        self.show_ref_line = !self.show_ref_line;
        self.focus_ref_field = self.show_ref_line;
    }

    /// Draw a reference line at `v` too, switching the lines on: the plot's
    /// menu. Lines switched off with the chip come back with it, as the chip
    /// would bring them. The caret stays where it was: the value is chosen.
    pub(super) fn add_ref_line(&mut self, v: f64) {
        if !self.ref_lines.values().contains(&v) {
            self.ref_lines.push(v, &self.current_unit);
        }
        self.show_ref_line = true;
    }

    /// Stop drawing the reference line at `v`: the plot's menu.
    pub(super) fn remove_ref_line(&mut self, v: f64) {
        self.ref_lines.remove(v, &self.current_unit);
    }

    /// A **Show:** chip was clicked: flip its trace, and leave the choice for
    /// the settings to remember ([`Graph::take_trace_choice`]).
    ///
    /// The main reading's own chip flips [`Graph::main_hidden`] instead and
    /// leaves nothing to remember: kept, it would hide that reading on the
    /// next meter to plot a sub-value.
    pub(crate) fn click_overlay_chip(&mut self, label: String) {
        if self.is_main_trace(&label) {
            self.main_hidden = !self.main_hidden;
        } else {
            let hidden = self.toggle_overlay_hidden(label.clone());
            self.trace_choice = Some((label, hidden));
        }
    }

    /// Flip one sub-value trace between drawn and hidden, returning whether
    /// it is now hidden.
    pub(super) fn toggle_overlay_hidden(&mut self, label: String) -> bool {
        if self.hidden_overlays.remove(&label) {
            false
        } else {
            self.hidden_overlays.insert(label);
            true
        }
    }

    /// Switch one sub-value trace off, as if its **Show:** chip had been
    /// clicked off. The chip still lists it, unlit, so the user can turn it
    /// back on; unlike [`Graph::toggle_overlay_hidden`] this is idempotent,
    /// which is what a caller reacting to a state change (rather than to a
    /// click) needs.
    pub(crate) fn hide_overlay(&mut self, label: &str) {
        self.hidden_overlays.insert(label.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A span set off the presets reads as one in the list's box.
    #[test]
    fn the_window_list_names_a_span_off_the_presets() {
        assert_eq!(window_label(60.0, Some(3)), TIME_WINDOWS[3].1);
        assert_eq!(window_label(47.3, None), "47s");
        assert_eq!(window_label(72.0, None), "1m12s");
        assert_eq!(window_label(2.45, None), "2.5s");
    }

    /// The keys a hover names keep their symbols: `[`, not "OpenBracket".
    #[test]
    fn hovers_write_keys_as_symbols() {
        let l10n = crate::l10n::english();
        let window = tr!(
            l10n,
            "graph-window.hover",
            window = "10s",
            keys = &cycle_keys()
        );
        assert_eq!(window, "Show the last 10s of samples ([ / ] to cycle)");
        let live = tr!(
            l10n,
            "graph-live.hover",
            keys = graph_key(&l10n, egui::Key::End)
        );
        assert_eq!(
            live,
            "Auto-follow the newest samples — off while panning (End to jump back)"
        );
        let mean = tr!(
            l10n,
            "graph-mean.hover",
            keys = graph_key(&l10n, egui::Key::M)
        );
        assert_eq!(
            mean,
            "Draw a horizontal line at the mean of visible samples (M)"
        );
    }
}
