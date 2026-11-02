//! The sub-value rows under or beside the reading: their grid, cell layout
//! and the measurements the big meter's fit weighs them by.

use dmm_lib::measurement::{AuxValue, MeasuredValue, Measurement};
use dmm_lib::text::TextKind;
use dmm_shared::l10n::Localizer;
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    FontId, Grid, Id, Rect, RichText, TextFormat, TextStyle, TextWrapMode, Ui, UiBuilder, Vec2,
    WidgetText, vec2,
};

use super::MIN_AUX_FONT_SIZE;
use super::text::format_aux_value;
use crate::theme::ThemeColors;

/// Screen rects of one sub-value row: (label, value+unit).
///
/// Returned by [`show_aux_rows`] so a test can assert the two are on the same
/// baseline. Production callers drop it — the rows are laid out by the grid,
/// not by their caller.
pub(super) type AuxRowRects = (Rect, Rect);

/// One sub-value row's cells: label, value with its unit, and the MIN/MAX
/// timestamp. Built in one place for drawing and for measuring, so the fit's
/// idea of the grid's size can't drift from the grid drawn.
struct AuxCells {
    label: RichText,
    value: LayoutJob,
    secs: Option<RichText>,
}

fn aux_cells(
    ui: &Ui,
    l10n: &Localizer,
    m: &Measurement,
    aux: &AuxValue,
    size: f32,
    tc: &ThemeColors,
) -> AuxCells {
    let label = RichText::new(l10n.label(TextKind::Aux, &aux.label))
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

/// The compact layout's one-line summary, `Measurement::aux_summary`'s text
/// coloured as the grid's cells are: weak labels and timestamps, values and
/// units in the reading colour, an overload in the error colour.
pub(super) fn aux_summary_job(
    ui: &Ui,
    l10n: &Localizer,
    m: &Measurement,
    tc: &ThemeColors,
) -> LayoutJob {
    let format = |color| TextFormat {
        font_id: FontId::monospace(MIN_AUX_FONT_SIZE),
        color,
        ..Default::default()
    };
    let weak = ui.visuals().weak_text_color();
    let mut job = LayoutJob::default();
    for aux in m.present_aux() {
        if !job.is_empty() {
            job.append(", ", 0.0, format(weak));
        }
        job.append(&l10n.label(TextKind::Aux, &aux.label), 0.0, format(weak));
        let value_color = match aux.value {
            MeasuredValue::Overload => tc.status_error(),
            _ => tc.reading(),
        };
        job.append(" ", 0.0, format(weak));
        job.append(&aux.value_str(), 0.0, format(value_color));
        let unit = aux.unit_or(&m.unit);
        if !unit.is_empty() {
            job.append(" ", 0.0, format(weak));
            job.append(unit, 0.0, format(tc.reading()));
        }
        if let Some(secs) = aux.elapsed_secs {
            job.append(&format!(" @{secs}s"), 0.0, format(weak));
        }
    }
    job
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
    pub(super) font_size: f32,
    pub(super) label_w: f32,
    pub(super) value_w: f32,
    /// Zero when no row carries a MIN/MAX timestamp: the grid then has two
    /// columns, not three.
    pub(super) secs_w: f32,
    pub(super) text_h: f32,
    pub(super) rows: usize,
}

impl AuxGridMetrics {
    /// Lay the cells out with a mode line at `font_size`; `None` when the
    /// measurement has no sub-values and the grid draws nothing.
    pub(super) fn measure(
        ui: &Ui,
        l10n: &Localizer,
        m: &Measurement,
        font_size: f32,
        tc: &ThemeColors,
    ) -> Option<Self> {
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
            let cells = aux_cells(ui, l10n, m, aux, size, tc);
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
    pub(super) fn size(&self, font_size: f32, min_cell: Vec2) -> Vec2 {
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
pub(super) fn show_aux_rows(
    ui: &mut Ui,
    l10n: &Localizer,
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
    // Hidden from screen readers, as the reading's own labels are: the spoken
    // reading says the sub-values.
    let scope = ui.scope_builder(UiBuilder::new().scope_id(id), |ui| {
        Grid::new("aux_rows")
            .num_columns(3)
            .spacing([aux_column_gap(size), AUX_ROW_GAP])
            .show(ui, |ui| {
                for aux in &m.aux_values {
                    let cells = aux_cells(ui, l10n, m, aux, size, tc);
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
    crate::a11y::hide_from_screen_readers(ui, scope.response.id);
    rects
}

/// The id every layout gives the sub-value grid drawn into `ui`.
pub(super) fn aux_grid_id(ui: &Ui) -> Id {
    ui.scope_id().with("aux_rows")
}
