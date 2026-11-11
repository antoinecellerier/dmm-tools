//! What the plot paints over itself in text: the marker flags in the time
//! axis's row, the cursor readouts, and the mean, reference and limit
//! labels, each placed clear of the others.

use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Ui};
use egui_plot::PlotTransform;

use super::{Graph, MeasureCursors};

/// Widest a marker's flag grows, its note included.
const FLAG_MAX_WIDTH: f32 = 170.0;
/// Room left between a flag and its neighbours' lines.
pub(super) const FLAG_AIR: f32 = 4.0;
/// Space between a flag's edge and its text.
const FLAG_PAD: f32 = 5.0;
/// Half the width of a flag's point, and its height.
const FLAG_TIP: egui::Vec2 = egui::vec2(5.0, 6.0);

/// A marker flag's text.
fn flag_font() -> egui::FontId {
    egui::FontId::proportional(12.0)
}

/// A marker flag's height, its point left out: the time labels' row, which
/// it shares, or its own text's if that is taller.
fn flag_height(ui: &Ui) -> f32 {
    let tick_font = egui::TextStyle::Body.resolve(ui.style());
    ui.fonts_mut(|f| f.row_height(&tick_font).max(f.row_height(&flag_font())))
}

/// The flags of the markers at screen `x` (ascending) under `plot`, laid
/// out with [`layout_marker_flags`] in the flags' font.
pub(super) fn marker_flags(ui: &Ui, markers: &[(f32, u32, &str)], plot: egui::Rect) -> Vec<Flag> {
    let width_of = |s: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), flag_font(), egui::Color32::PLACEHOLDER)
                .size()
                .x
        })
    };
    layout_marker_flags(markers, plot, flag_height(ui), width_of)
}

/// One marker's flag on the plot: the line it points at, where its tag is
/// drawn, and what the tag says.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Flag {
    pub(super) number: u32,
    pub(super) x: f32,
    pub(super) rect: egui::Rect,
    pub(super) label: String,
}

/// The flags of the markers at screen `x` (ascending), each labelled with
/// its number and note, hanging under `plot` in the time axis's row, their
/// points up at their lines.
///
/// A flag is centred on its line. Its note gets no more than the gap to the
/// nearer neighbouring line, so notes are cut before they cover a neighbour;
/// the number always stays. A flag that would still touch the one before it
/// slides right, and one that would slide off its own line is left out — its
/// line still shows. Near a plot edge a flag slides inward, its point staying
/// on the line. `width_of` measures a label.
pub(super) fn layout_marker_flags(
    markers: &[(f32, u32, &str)],
    plot: egui::Rect,
    height: f32,
    width_of: impl Fn(&str) -> f32,
) -> Vec<Flag> {
    let mut flags: Vec<Flag> = Vec::new();
    for (i, &(x, number, note)) in markers.iter().enumerate() {
        let before = i.checked_sub(1).map_or(f32::INFINITY, |j| x - markers[j].0);
        let after = markers.get(i + 1).map_or(f32::INFINITY, |next| next.0 - x);
        let room = FLAG_MAX_WIDTH.min(before - FLAG_AIR).min(after - FLAG_AIR);
        let label = fit_flag_label(number, note, room - 2.0 * FLAG_PAD, &width_of);
        let width = width_of(&label) + 2.0 * FLAG_PAD;
        let mut left = (x - width / 2.0).min(plot.right() - width).max(plot.left());
        if let Some(prev) = flags.last() {
            left = left.max(prev.rect.right() + FLAG_AIR);
        }
        // The point has to leave from the tag.
        if x < left || x > left + width || left + width > plot.right() {
            continue;
        }
        let rect =
            egui::Rect::from_min_size(egui::pos2(left, plot.bottom()), egui::vec2(width, height));
        flags.push(Flag {
            number,
            x,
            rect,
            label,
        });
    }
    flags
}

/// The longest of "3 note" and "3 no…" that fits `room`, else "3".
fn fit_flag_label(number: u32, note: &str, room: f32, width_of: &impl Fn(&str) -> f32) -> String {
    let number = number.to_string();
    let note = note.trim();
    if note.is_empty() {
        return number;
    }
    // The top bar's separator, so a numeric note stays apart from the number.
    let full = format!("{number} \u{00B7} {note}");
    if width_of(&full) <= room {
        return full;
    }
    // Widths grow with the characters kept, so search for the most that fit
    // rather than measuring every length.
    let chars: Vec<char> = note.chars().collect();
    let cut = |k: usize| {
        let kept: String = chars[..k].iter().collect();
        format!("{number} \u{00B7} {}\u{2026}", kept.trim_end())
    };
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width_of(&cut(mid)) <= room {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo > 0 { cut(lo) } else { number }
}

/// How far a cursor's readout stands off its point: clear of the vertical
/// line and of the dashed level line.
const CURSOR_LABEL_OFFSET: egui::Vec2 = egui::vec2(4.0, 2.0);

/// Whether `rect` keeps a label's gap from every `taken` one, so two labels
/// never sit flush.
fn is_clear(taken: &[egui::Rect], rect: egui::Rect) -> bool {
    let gap = LEVEL_LABEL_GAP;
    taken.iter().all(|t| {
        rect.left() >= t.right() + gap
            || rect.right() + gap <= t.left()
            || rect.top() >= t.bottom() + gap
            || rect.bottom() + gap <= t.top()
    })
}

/// Rows a cursor's readout may move further from its point when all four
/// corners next to it are taken.
const CURSOR_LABEL_ROWS: usize = 2;

/// Where a cursor's readout goes around its point: the first of right-above,
/// left-above, right-below and left-below that stays inside the plot, clear
/// of the `taken` labels and off the trace, then the first inside and clear
/// of `taken`. When all four are taken — the other readout, a marker's
/// flag — the same corners a row further out, up to [`CURSOR_LABEL_ROWS`].
/// Failing all of those, None: a readout on top of another label is no
/// easier to read than none.
pub(super) fn cursor_label_rect(
    point: egui::Pos2,
    size: egui::Vec2,
    plot: egui::Rect,
    taken: &[egui::Rect],
    hits_trace: impl Fn(egui::Rect) -> bool,
) -> Option<egui::Rect> {
    use egui::{Align2, vec2};
    let (dx, dy) = (CURSOR_LABEL_OFFSET.x, CURSOR_LABEL_OFFSET.y);
    let corners = |away: f32| {
        [
            Align2::LEFT_BOTTOM.anchor_size(point + vec2(dx, -dy - away), size),
            Align2::RIGHT_BOTTOM.anchor_size(point + vec2(-dx, -dy - away), size),
            Align2::LEFT_TOP.anchor_size(point + vec2(dx, dy + away), size),
            Align2::RIGHT_TOP.anchor_size(point + vec2(-dx, dy + away), size),
        ]
    };
    let clear = |r: &egui::Rect| is_clear(taken, *r);
    for row in 0..=CURSOR_LABEL_ROWS {
        let row = corners(row as f32 * (size.y + dy));
        let inside = || row.iter().copied().filter(|r| plot.contains_rect(*r));
        // `clear` first: it is cheap, and the trace walk isn't.
        let found = inside()
            .filter(clear)
            .find(|r| !hits_trace(*r))
            .or_else(|| inside().find(clear));
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Where an overlay label's rim is drawn around its text: a pixel out on
/// every side, so the lines under it stop short of the letters.
const LABEL_HALO: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (0.0, 1.0),
    (1.0, 1.0),
];

/// Space between a mean or reference label and its line, and between two
/// labels stacked in a column.
const LEVEL_LABEL_GAP: f32 = 2.0;
/// How far a mean or reference label ends short of the plot's right edge,
/// or of a cursor or marker line it steps left of.
const LEVEL_LABEL_INSET: f32 = 4.0;

/// Where the labels of the mean and reference lines at screen `ys` go, given
/// the labels already placed (`taken`) and the vertical cursor and marker
/// lines at `line_xs`. `ys` runs top to bottom, and `sizes` matches it.
///
/// The labels form one column at the plot's right edge, in their lines'
/// order. Each sits just above its line — below it when there is no room
/// above — unless the label before it is in the way; then it goes right under
/// that one. A label never has a mean or reference line through its text:
/// it moves past the line instead, so lines closer together than a label is
/// tall get one label above them and the rest stacked below them. A label
/// steps left past the cursor and marker lines that would cross it, giving
/// up at the plot's left edge, and moves past a taken spot. The column is built top down and bottom up, with the
/// labels above their lines and then below them; the first of those clear of
/// the trace wins, else the first that fits. When no column fits, the lines
/// may cross the text; when none fits even so, each label goes where it
/// can and the rest are left out (None). A line above or below the view is
/// labelled at that edge.
///
/// In live view a cursor or marker line scrolling left through a label takes
/// it along until it has passed, then the label springs back to the edge,
/// and crowded labels can hop between columns as the trace moves under
/// them.
pub(super) fn level_label_rects(
    ys: &[f32],
    sizes: &[egui::Vec2],
    plot: egui::Rect,
    taken: &[egui::Rect],
    line_xs: &[f32],
    hits_trace: impl Fn(egui::Rect) -> bool,
) -> Vec<Option<egui::Rect>> {
    let gap = LEVEL_LABEL_GAP;
    // The lines drawn, which no label's text may cross.
    let drawn: Vec<f32> = ys
        .iter()
        .copied()
        .filter(|&y| plot.y_range().contains(y))
        .collect();
    let ys: Vec<f32> = ys
        .iter()
        .map(|y| y.max(plot.top()).min(plot.bottom()))
        .collect();
    // Above the line, or below it when asked or when above has no room.
    let usual_top = |y: f32, h: f32, below: bool| {
        let (above, under) = (y - gap - h, y + gap);
        if (below && under + h <= plot.bottom()) || above < plot.top() {
            under
        } else {
            above
        }
    };

    // Each label at its usual spot, or past the one before it: top down,
    // or bottom up. `level_ys` are the lines kept out of the text.
    let column = |down: bool, below: bool, level_ys: &[f32]| {
        let mut rects: Vec<egui::Rect> = Vec::with_capacity(ys.len());
        let order: Vec<usize> = if down {
            (0..ys.len()).collect()
        } else {
            (0..ys.len()).rev().collect()
        };
        for i in order {
            let (y, size) = (ys[i], sizes[i]);
            let top = match rects.last() {
                None => usual_top(y, size.y, below),
                Some(r) if down => usual_top(y, size.y, below).max(r.bottom() + gap),
                Some(r) => usual_top(y, size.y, below).min(r.top() - gap - size.y),
            };
            let rect = fit_level_label(top, size, down, plot, taken, level_ys, line_xs)?;
            rects.push(rect);
        }
        if !down {
            rects.reverse();
        }
        Some(rects)
    };
    // The first of: above the lines top down, bottom up, then below them
    // top down, bottom up — the first clear of the trace, if one is. Only
    // a clear column counts: a noisy trace crosses every column by a count
    // that changes each frame, and chasing the least would move the labels
    // with it. Lines too close for any column to keep out of the text may
    // cross it.
    for level_ys in [&drawn[..], &[]] {
        let columns: Vec<Vec<egui::Rect>> =
            [(true, false), (false, false), (true, true), (false, true)]
                .into_iter()
                .filter_map(|(down, below)| column(down, below, level_ys))
                .collect();
        if let Some(rects) = columns
            .iter()
            .find(|rects| !rects.iter().any(|r| hits_trace(*r)))
            .or(columns.first())
        {
            return rects.iter().copied().map(Some).collect();
        }
    }

    // No room for them all: each in turn where it fits, and none for the
    // rest rather than a pile no one can read.
    let mut placed: Vec<egui::Rect> = taken.to_vec();
    ys.iter()
        .zip(sizes)
        .map(|(&y, &size)| {
            let top = usual_top(y, size.y, false);
            let rect = fit_level_label(top, size, true, plot, &placed, &[], line_xs)
                .or_else(|| fit_level_label(top, size, false, plot, &placed, &[], line_xs))?;
            placed.push(rect);
            Some(rect)
        })
        .collect()
}

/// The first spot for a mean or reference label at or past `top`, moving
/// down or up: clear of the `level_ys` lines and of `taken`, stepped left
/// past the `line_xs` that would cross it. None once it leaves the plot.
fn fit_level_label(
    mut top: f32,
    size: egui::Vec2,
    down: bool,
    plot: egui::Rect,
    taken: &[egui::Rect],
    level_ys: &[f32],
    line_xs: &[f32],
) -> Option<egui::Rect> {
    let (gap, h) = (LEVEL_LABEL_GAP, size.y);
    let at =
        |top: f32, right: f32| egui::Rect::from_min_size(egui::pos2(right - size.x, top), size);
    let edge = plot.right() - LEVEL_LABEL_INSET;
    // Every pass moves past a line or a taken spot, so it ends.
    for _ in 0..=level_ys.len() + taken.len() {
        if top < plot.top() || top + h > plot.bottom() {
            return None;
        }
        // A mean or reference line through the text: past it.
        let through = level_ys
            .iter()
            .copied()
            .filter(|&y| top - gap < y && y < top + h + gap);
        if let Some(y) = if down {
            through.reduce(f32::max)
        } else {
            through.reduce(f32::min)
        } {
            top = if down { y + gap } else { y - gap - h };
            continue;
        }
        // Along the edge, then left of each line crossing it.
        let mut right = edge;
        let mut first_clear = None;
        // Each step passes at least one line.
        for _ in 0..=line_xs.len() {
            let rect = at(top, right);
            if !plot.contains_rect(rect) {
                break;
            }
            let clear = is_clear(taken, rect);
            let crossing = line_xs
                .iter()
                .copied()
                .filter(|&x| rect.left() < x && x < rect.right())
                .reduce(f32::min);
            match crossing {
                None if clear => return Some(rect),
                None => break,
                Some(x) => {
                    if clear && first_clear.is_none() {
                        first_clear = Some(rect);
                    }
                    right = x - LEVEL_LABEL_INSET;
                }
            }
        }
        if first_clear.is_some() {
            return first_clear;
        }
        // Taken at the edge: past whatever is there.
        let rect = at(top, edge);
        let blockers = taken.iter().filter(|t| !is_clear(&[**t], rect));
        top = if down {
            blockers.map(|t| t.bottom()).reduce(f32::max)? + gap
        } else {
            blockers.map(|t| t.top()).reduce(f32::min)? - gap - h
        };
    }
    None
}

/// Whether the straight segment from `a` to `b` passes through `rect`.
pub(super) fn segment_hits_rect(a: egui::Pos2, b: egui::Pos2, rect: egui::Rect) -> bool {
    clip_segment(a, b, rect).is_some()
}

/// The part of the straight segment from `a` to `b` inside `rect`, as the
/// fractions of the way from `a` it enters and leaves at; `None` when it
/// misses (Liang–Barsky: clip the parameter range against each edge). In
/// f64, so a fraction of a segment running far off the rect still places
/// its ends within a small fraction of a point.
pub(super) fn clip_segment(a: egui::Pos2, b: egui::Pos2, rect: egui::Rect) -> Option<(f64, f64)> {
    let (ax, ay) = (f64::from(a.x), f64::from(a.y));
    let (dx, dy) = (f64::from(b.x) - ax, f64::from(b.y) - ay);
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, ax - f64::from(rect.left())),
        (dx, f64::from(rect.right()) - ax),
        (-dy, ay - f64::from(rect.top())),
        (dy, f64::from(rect.bottom()) - ay),
    ] {
        if p == 0.0 {
            // Parallel to this edge: outside it means outside the rect.
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                enter = enter.max(r);
            } else {
                leave = leave.min(r);
            }
            if enter > leave {
                return None;
            }
        }
    }
    Some((enter, leave))
}

/// The plot's labelled horizontal lines — the mean, the reference lines, the
/// alarm limits and the cursors — worked out before the plot: the plot draws
/// the lines from it (`MainFrame::draw_over_trace`) and `paint_overlay_labels`
/// labels them once it is drawn, so a line and its label share their values.
pub(super) struct OverlayLabelData {
    pub(super) show_mean: bool,
    pub(super) mean_value: Option<f64>,
    pub(super) show_ref: bool,
    pub(super) ref_values: Vec<f64>,
    /// The alarm limits drawn, in the plotted unit, with their names.
    pub(super) limit_lines: Vec<(f64, super::LimitSide)>,
    pub(super) cursors: MeasureCursors,
    pub(super) cursor_va: Option<f64>,
    pub(super) cursor_vb: Option<f64>,
    /// What each cursor's label writes of its reading: the meter's digits
    /// with their unit (see `Graph::point_text`).
    pub(super) cursor_texts: [Option<String>; 2],
    /// The times of the markers in view, whose lines the mean and reference
    /// labels step aside from.
    pub(super) marker_times: Vec<f64>,
    pub(super) overlay_unit: String,
    /// [`Graph::decimals`] for the mean and the reference lines, which
    /// aren't points.
    pub(super) other_decimals: usize,
    /// The plotted series is a level: a whole number with no unit.
    pub(super) levels: bool,
    pub(super) view_max: f64,
    pub(super) mean_color: egui::Color32,
    pub(super) ref_color: egui::Color32,
    pub(super) limit_color: egui::Color32,
    pub(super) cursor_color: egui::Color32,
    /// The plot's background, which rims each label so grid lines, the
    /// trace and other lines stop short of its letters.
    pub(super) halo_color: egui::Color32,
}

impl Graph {
    /// Each marker's flag at the bottom of the plot: a tag in the marker
    /// colour pointing up at its line, with the number and as much of the
    /// note as fits, in the plot background's colour. Returns the number of
    /// the marker whose flag was clicked, with the button — the primary
    /// opens its note, the secondary its menu — and each flag's point, the
    /// part in the plot, which the overlay labels keep off.
    ///
    /// A flag takes clicks but not the keyboard focus: the plot would gain a
    /// Tab stop per marker, and the Recording panel's log already has one.
    pub(super) fn paint_marker_flags(
        ui: &Ui,
        l10n: &Localizer,
        transform: &PlotTransform,
        plot_rect: egui::Rect,
        in_view: &[(f64, &crate::markers::Marker)],
        color: egui::Color32,
        text_color: egui::Color32,
    ) -> (Option<(u32, egui::PointerButton)>, Vec<egui::Rect>) {
        if in_view.is_empty() {
            return (None, Vec::new());
        }
        let mut clicked = None;
        let mut rects = Vec::new();
        let painter = ui.painter();
        let at: Vec<(f32, u32, &str)> = in_view
            .iter()
            .map(|&(x, m)| {
                let px = transform
                    .position_from_point(&egui_plot::PlotPoint::new(x, 0.0))
                    .x;
                (px, m.number, m.note.as_str())
            })
            .collect();
        for flag in marker_flags(ui, &at, plot_rect) {
            let top = flag.rect.top();
            // The point's base stays on the tag, even for one slid to an edge.
            let base = |dx: f32| (flag.x + dx).clamp(flag.rect.left(), flag.rect.right());
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(base(-FLAG_TIP.x), top),
                    egui::pos2(flag.x, top - FLAG_TIP.y),
                    egui::pos2(base(FLAG_TIP.x), top),
                ],
                color,
                egui::Stroke::NONE,
            ));
            painter.rect_filled(flag.rect, 3.0, color);
            painter.text(
                flag.rect.center(),
                egui::Align2::CENTER_CENTER,
                &flag.label,
                flag_font(),
                text_color,
            );
            let edit = tr!(l10n, "graph-marker-flag", number = flag.number);
            let response = ui
                .interact(
                    flag.rect,
                    ui.scope_id().with(("marker_flag", flag.number)),
                    egui::Sense::CLICK,
                )
                .on_hover_text(&*edit)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &edit));
            if response.clicked() {
                clicked = Some((flag.number, egui::PointerButton::Primary));
            } else if response.secondary_clicked() {
                clicked = Some((flag.number, egui::PointerButton::Secondary));
            }
            // Only the point reaches into the plot.
            rects.push(egui::Rect::from_min_max(
                egui::pos2(flag.x - FLAG_TIP.x, top - FLAG_TIP.y),
                egui::pos2(flag.x + FLAG_TIP.x, top),
            ));
        }
        (clicked, rects)
    }

    /// Paint text labels for overlays (mean, reference lines, cursors) using the
    /// UI painter so they render outside the plot's clip rect.
    ///
    /// Each label keeps off `taken` — the key and the marker flags — and off
    /// the labels placed before it. The cursor readouts go first, as they
    /// have only four corners to choose from; the mean and reference labels
    /// then form one column around them (see [`level_label_rects`]).
    pub(super) fn paint_overlay_labels(
        ui: &Ui,
        l10n: &Localizer,
        plot_rect: egui::Rect,
        transform: &PlotTransform,
        data: &OverlayLabelData,
        trace: &[Vec<[f64; 2]>],
        mut taken: Vec<egui::Rect>,
    ) {
        let painter = ui.painter();
        let label_font = egui::FontId::proportional(12.0);
        let screen =
            |t: f64, v: f64| transform.position_from_point(&egui_plot::PlotPoint::new(t, v));
        let layout = |text: String, color: egui::Color32| {
            painter.layout_no_wrap(text, label_font.clone(), color)
        };
        let paint = |rect: egui::Rect, galley: std::sync::Arc<egui::Galley>, color| {
            for (dx, dy) in LABEL_HALO {
                let at = rect.min + egui::vec2(dx, dy);
                painter.galley_with_override_text_color(at, galley.clone(), data.halo_color);
            }
            painter.galley(rect.min, galley, color);
        };

        // The vertical lines a mean or reference label steps aside from.
        let line_xs: Vec<f32> = data
            .marker_times
            .iter()
            .copied()
            .chain(data.cursors.placed())
            .map(|t| screen(t, 0.0).x)
            .filter(|&x| plot_rect.x_range().contains(x))
            .collect();

        // Whether the trace runs through a label. Only the stretch under the
        // label's time span can touch it.
        let hits_trace = |rect: egui::Rect| {
            let t_lo = transform.value_from_position(rect.left_top()).x;
            let t_hi = transform.value_from_position(rect.right_top()).x;
            trace.iter().any(|segment| {
                segment.windows(2).any(|w| {
                    w[1][0] >= t_lo
                        && w[0][0] <= t_hi
                        && segment_hits_rect(
                            screen(w[0][0], w[0][1]),
                            screen(w[1][0], w[1][1]),
                            rect,
                        )
                })
            })
        };

        // In the prefix the left axis writes its ticks in, beside them.
        let bounds = transform.bounds();
        let prefix = super::axes::axis_prefix(&data.overlay_unit, bounds.min()[1], bounds.max()[1]);
        let in_axis = |v: f64| super::axes::in_prefix(prefix.as_ref(), &data.overlay_unit, v);
        // A height the user put there, a reference or a limit: in the
        // axis's prefix, as typed.
        let level_text = |v: f64| {
            let (v, unit) = in_axis(v);
            let v = match prefix {
                Some(_) => super::axes::short_figure(v),
                None => format!("{v:.prec$}", prec = data.other_decimals),
            };
            format!("{v} {unit}").trim_end().to_string()
        };
        // A figure of the readings, the mean: in a prefix that fits it, to
        // five digits, as Statistics writes it, or a level's decimals.
        let figure_text = |v: f64| {
            if data.levels {
                return format!(
                    "{v:.prec$} {}",
                    data.overlay_unit,
                    prec = data.other_decimals
                )
                .trim_end()
                .to_string();
            }
            let (number, unit) = dmm_shared::stat_figure(v, &data.overlay_unit, None);
            format!("{number} {unit}").trim_end().to_string()
        };
        // Cursor labels
        if data.cursors.active {
            for (name, t, value, text) in [
                ("A", data.cursors.a, data.cursor_va, &data.cursor_texts[0]),
                ("B", data.cursors.b, data.cursor_vb, &data.cursor_texts[1]),
            ] {
                let Some(t) = t else { continue };
                let y_val = value.unwrap_or(0.0);
                let pos = screen(t, y_val);
                // Scrolled out of view: no line to label. A reading above or
                // below the Y axis still has its line in view, so its readout
                // goes against that edge.
                if !plot_rect.x_range().contains(pos.x) {
                    continue;
                }
                let pos = egui::pos2(pos.x, pos.y.max(plot_rect.top()).min(plot_rect.bottom()));
                let reading = text.clone().unwrap_or_else(|| crate::NO_DATA.to_string());
                let galley = layout(format!("{name}: {t:.2} s / {reading}"), data.cursor_color);
                if let Some(rect) =
                    cursor_label_rect(pos, galley.size(), plot_rect, &taken, hits_trace)
                {
                    paint(rect, galley, data.cursor_color);
                    taken.push(rect);
                }
            }
        }

        // The mean and reference labels, one column in their lines' order:
        // the mean first among equals, one label for a value entered twice.
        let mut levels: Vec<(f64, String, egui::Color32)> = Vec::new();
        if data.show_mean
            && let Some(avg) = data.mean_value
        {
            let label = tr!(l10n, "graph-mean-line", value = figure_text(avg));
            levels.push((avg, label.into_owned(), data.mean_color));
        }
        if data.show_ref {
            let mut refs = data.ref_values.clone();
            refs.sort_by(|a, b| b.total_cmp(a));
            refs.dedup();
            levels.extend(refs.into_iter().map(|v| (v, level_text(v), data.ref_color)));
        }
        levels.extend(data.limit_lines.iter().map(|&(v, side)| {
            let side = super::menu::side_arg(side);
            let label = tr!(l10n, "alarm-limit-line", side = side, value = level_text(v));
            (v, label.into_owned(), data.limit_color)
        }));
        let mut levels: Vec<_> = levels
            .into_iter()
            .map(|(v, text, color)| (screen(data.view_max, v).y, layout(text, color), color))
            .collect();
        levels.sort_by(|a, b| a.0.total_cmp(&b.0));
        let ys: Vec<f32> = levels.iter().map(|l| l.0).collect();
        let sizes: Vec<egui::Vec2> = levels.iter().map(|l| l.1.size()).collect();
        let rects = level_label_rects(&ys, &sizes, plot_rect, &taken, &line_xs, hits_trace);
        for ((_, galley, color), rect) in levels.into_iter().zip(rects) {
            if let Some(rect) = rect {
                paint(rect, galley, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four corners `cursor_label_rect` picks from, for a 100x20 readout at
    /// (300, 200) in a 500x400 plot: right-above, left-above, right-below,
    /// left-below.
    fn readout_corners() -> [egui::Rect; 4] {
        use egui::{Rect, pos2};
        [
            Rect::from_min_max(pos2(304.0, 178.0), pos2(404.0, 198.0)),
            Rect::from_min_max(pos2(196.0, 178.0), pos2(296.0, 198.0)),
            Rect::from_min_max(pos2(304.0, 202.0), pos2(404.0, 222.0)),
            Rect::from_min_max(pos2(196.0, 202.0), pos2(296.0, 222.0)),
        ]
    }

    fn readout_rect(plot_right: f32, hits: impl Fn(egui::Rect) -> bool) -> egui::Rect {
        readout_rect_around(plot_right, &[], hits).expect("a corner inside the plot")
    }

    fn readout_rect_around(
        plot_right: f32,
        taken: &[egui::Rect],
        hits: impl Fn(egui::Rect) -> bool,
    ) -> Option<egui::Rect> {
        let plot = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(plot_right, 400.0));
        cursor_label_rect(
            egui::pos2(300.0, 200.0),
            egui::vec2(100.0, 20.0),
            plot,
            taken,
            hits,
        )
    }

    #[test]
    fn cursor_readout_takes_the_first_clear_corner_inside_the_plot() {
        let [right_above, left_above, right_below, left_below] = readout_corners();
        // Room on the right and no trace: right-above.
        assert_eq!(readout_rect(500.0, |_| false), right_above);
        // Against the plot's right edge, the right-hand corners are out.
        assert_eq!(readout_rect(400.0, |_| false), left_above);
        // Out on the right and the trace through left-above: left-below.
        assert_eq!(readout_rect(400.0, |r| r == left_above), left_below);
        // Trace through right-above only: left-above, before right-below.
        assert_eq!(readout_rect(500.0, |r| r == right_above), left_above);
        // Trace through both above: right-below.
        assert_eq!(
            readout_rect(500.0, |r| r == right_above || r == left_above),
            right_below
        );
    }

    #[test]
    fn cursor_readout_falls_back_when_no_corner_is_clear() {
        let [_, left_above, ..] = readout_corners();
        // The trace everywhere: the first corner inside the plot.
        assert_eq!(readout_rect(400.0, |_| true), left_above);
        // A plot narrower than the readout: no readout.
        let narrow = egui::Rect::from_min_max(egui::pos2(250.0, 0.0), egui::pos2(350.0, 400.0));
        let rect = cursor_label_rect(
            egui::pos2(300.0, 200.0),
            egui::vec2(100.0, 20.0),
            narrow,
            &[],
            |_| false,
        );
        assert_eq!(rect, None);
    }

    #[test]
    fn cursor_readout_keeps_off_labels_placed_before_it() {
        let [right_above, left_above, right_below, left_below] = readout_corners();
        // The other readout, a mean label or the key over right-above: left-above.
        let other = right_above.translate(egui::vec2(30.0, -5.0));
        assert_eq!(
            readout_rect_around(500.0, &[other], |_| false),
            Some(left_above)
        );
        // A flag under both below corners changes nothing above.
        let flag = right_below.union(left_below);
        assert_eq!(
            readout_rect_around(500.0, &[flag], |_| false),
            Some(right_above)
        );
        // A label counts before the trace: left-above is taken, so the corner on
        // the trace (left-below) wins over it.
        assert_eq!(
            readout_rect_around(400.0, &[left_above], |r| r == left_below),
            Some(left_below)
        );
        // All four taken, as by the other readout and a flag: the same corners a
        // row further out, above first.
        let near = right_above.union(left_below);
        let row = egui::vec2(0.0, 22.0);
        assert_eq!(
            readout_rect_around(400.0, &[near], |_| false),
            Some(left_above.translate(-row))
        );
        // Every row taken: no readout rather than one on top of another label.
        let all = near.expand2(egui::vec2(0.0, 3.0 * 22.0));
        assert_eq!(readout_rect_around(400.0, &[all], |_| false), None);
    }

    /// A 500x400 plot, and a 100x14 label, for the mean and reference labels.
    const LEVEL_PLOT: egui::Rect =
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(500.0, 400.0));
    const LEVEL_SIZE: egui::Vec2 = egui::vec2(100.0, 14.0);

    /// A 100x14 label ending at `right` and `bottom`; the usual right edge is 496.
    fn level_at(right: f32, bottom: f32) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(right - 100.0, bottom - 14.0),
            egui::pos2(right, bottom),
        )
    }

    /// The column for lines at `ys`, top to bottom.
    fn column(
        ys: &[f32],
        taken: &[egui::Rect],
        line_xs: &[f32],
        hits_trace: impl Fn(egui::Rect) -> bool,
    ) -> Vec<egui::Rect> {
        let sizes = vec![LEVEL_SIZE; ys.len()];
        level_label_rects(ys, &sizes, LEVEL_PLOT, taken, line_xs, hits_trace)
            .into_iter()
            .map(|r| r.expect("every label placed"))
            .collect()
    }

    fn plain_column(ys: &[f32]) -> Vec<egui::Rect> {
        column(ys, &[], &[], |_| false)
    }

    /// Whether the labels run top to bottom without touching.
    fn stacked(rects: &[egui::Rect]) -> bool {
        rects.windows(2).all(|w| w[0].bottom() < w[1].top())
    }

    #[test]
    fn level_labels_sit_above_their_lines_at_the_right_edge() {
        assert_eq!(plain_column(&[200.0]), [level_at(496.0, 198.0)]);
        // Lines well apart don't disturb each other.
        assert_eq!(
            plain_column(&[100.0, 200.0, 300.0]),
            [
                level_at(496.0, 98.0),
                level_at(496.0, 198.0),
                level_at(496.0, 298.0)
            ]
        );
    }

    #[test]
    fn close_lines_get_one_label_above_them_and_the_rest_below() {
        // Four lines within 7 px, as a mean among three close references.
        let ys = [337.0, 338.0, 343.0, 344.0];
        assert_eq!(
            plain_column(&ys),
            [
                level_at(496.0, 335.0),
                level_at(496.0, 360.0),
                level_at(496.0, 376.0),
                level_at(496.0, 392.0)
            ]
        );
    }

    #[test]
    fn the_column_keeps_off_the_trace() {
        let ys = [337.0, 338.0, 343.0, 344.0];
        // The trace runs under the lines: the column goes above them, bottom up.
        let rects = column(&ys, &[], &[], |r| r.bottom() > 340.0);
        assert!(stacked(&rects), "{rects:?}");
        assert_eq!(rects[3], level_at(496.0, 335.0));
        // A noisy trace through every column: the first, as with no trace, not
        // whichever it crosses least this frame.
        let crossed_less_above = |r: egui::Rect| r.top() > 300.0;
        assert_eq!(column(&ys, &[], &[], crossed_less_above), plain_column(&ys));
        // A lone label with the trace through its spot goes below its line.
        assert_eq!(
            column(&[200.0], &[], &[], |r| r == level_at(496.0, 198.0)),
            [level_at(496.0, 216.0)]
        );
    }

    #[test]
    fn level_labels_at_the_plots_edges() {
        // No room above: below the line.
        assert_eq!(plain_column(&[10.0]), [level_at(496.0, 26.0)]);
        // Lines above the view are labelled under the top edge, stacked.
        assert_eq!(
            plain_column(&[-50.0, -40.0, -30.0]),
            [
                level_at(496.0, 16.0),
                level_at(496.0, 32.0),
                level_at(496.0, 48.0)
            ]
        );
        // Close lines at the bottom: no room below them, so the column is built
        // bottom up, all above them.
        let rects = plain_column(&[390.0, 392.0, 394.0]);
        assert!(stacked(&rects), "{rects:?}");
        assert_eq!(rects[2], level_at(496.0, 388.0));
    }

    #[test]
    fn lines_too_close_to_keep_out_of_the_text_may_cross_it() {
        // Sixteen lines 4 px apart: a column clear of them all needs more room
        // than the plot has, so the labels stack through them, still in order.
        let ys: Vec<f32> = (0..16).map(|i| 230.0 + 4.0 * i as f32).collect();
        let rects = plain_column(&ys);
        assert!(stacked(&rects), "{rects:?}");
        let crossed = |r: &egui::Rect| ys.iter().any(|&y| r.top() < y && y < r.bottom());
        assert!(rects.iter().any(crossed));
    }

    #[test]
    fn labels_with_no_room_are_left_out_rather_than_piled() {
        // Thirty labels, room for about twenty-four: the rest are dropped, and
        // those drawn don't touch.
        let sizes = vec![LEVEL_SIZE; 30];
        let rects = level_label_rects(&[200.0; 30], &sizes, LEVEL_PLOT, &[], &[], |_| false);
        let drawn: Vec<egui::Rect> = rects.iter().flatten().copied().collect();
        assert!(drawn.len() > 20 && drawn.len() < 30, "{}", drawn.len());
        for (i, a) in drawn.iter().enumerate() {
            for b in &drawn[i + 1..] {
                assert!(!a.intersects(*b), "{a:?} overlaps {b:?}");
            }
        }
        // A plot shorter than a label has room for none.
        let short = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(500.0, 10.0));
        let rects = level_label_rects(&[5.0], &[LEVEL_SIZE], short, &[], &[], |_| false);
        assert_eq!(rects, [None]);
    }

    #[test]
    fn a_level_label_steps_left_of_the_lines_through_it() {
        // One line through the usual spot: just left of it, on its own line.
        assert_eq!(
            column(&[200.0], &[], &[450.0], |_| false),
            [level_at(446.0, 198.0)]
        );
        // Two lines close together: left of both, not between them.
        assert_eq!(
            column(&[200.0], &[], &[450.0, 420.0], |_| false),
            [level_at(416.0, 198.0)]
        );
        // A line clear of the label changes nothing.
        assert_eq!(
            column(&[200.0], &[], &[300.0], |_| false),
            [level_at(496.0, 198.0)]
        );
        // Four lines 90 px apart: each step lands on the next one, until the
        // label is left of them all.
        assert_eq!(
            column(&[200.0], &[], &[470.0, 380.0, 290.0, 200.0], |_| false),
            [level_at(196.0, 198.0)]
        );
        // Lines all the way across: it would leave the plot, so it stays at the
        // edge.
        let across: Vec<f32> = (0..6).map(|i| 470.0 - 90.0 * i as f32).collect();
        assert_eq!(
            column(&[200.0], &[], &across, |_| false),
            [level_at(496.0, 198.0)]
        );
    }

    #[test]
    fn a_level_label_moves_past_a_readout_on_its_spot() {
        // A readout on the spot above the line: below the line instead.
        let readout = level_at(496.0, 198.0);
        assert_eq!(
            column(&[200.0], &[readout], &[], |_| false),
            [level_at(496.0, 216.0)]
        );
        // One ending a pixel short of the spot still counts: labels keep a gap.
        let flush = readout.translate(egui::vec2(-101.0, 0.0));
        assert_eq!(
            column(&[200.0], &[flush], &[], |_| false),
            [level_at(496.0, 216.0)]
        );
    }

    #[test]
    fn segment_hits_rect_follows_the_segment_not_its_ends() {
        use egui::{Rect, pos2};
        let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(20.0, 20.0));
        // One end inside.
        assert!(segment_hits_rect(pos2(15.0, 15.0), pos2(40.0, 40.0), rect));
        // Both ends outside, straight through: a step edge crossing a readout.
        assert!(segment_hits_rect(pos2(15.0, 0.0), pos2(15.0, 30.0), rect));
        assert!(segment_hits_rect(pos2(0.0, 12.0), pos2(30.0, 18.0), rect));
        // Beside it, parallel to an edge.
        assert!(!segment_hits_rect(pos2(25.0, 0.0), pos2(25.0, 30.0), rect));
        // Diagonal that passes the corner.
        assert!(!segment_hits_rect(pos2(0.0, 5.0), pos2(30.0, -5.0), rect));
        // Short of it.
        assert!(!segment_hits_rect(pos2(0.0, 15.0), pos2(8.0, 15.0), rect));
    }

    /// Flags as a 7 px-per-character font would lay them out in a 1000 px plot.
    fn flags(markers: &[(f32, u32, &str)]) -> Vec<(f32, egui::Rect, String)> {
        let plot = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1000.0, 300.0));
        layout_marker_flags(markers, plot, 19.0, |s| s.chars().count() as f32 * 7.0)
            .into_iter()
            .map(|f| (f.x, f.rect, f.label))
            .collect()
    }

    #[test]
    fn a_flag_is_centred_on_its_line_and_cut_to_its_cap() {
        let long = "load on: 2.2 ohm across the output, supply set to 12.07 V";
        let laid = flags(&[(500.0, 3, long)]);
        let (x, rect, label) = &laid[0];
        assert_eq!(*x, 500.0);
        assert!((rect.center().x - 500.0).abs() < 0.01, "centred");
        assert!(rect.width() <= 170.0, "{rect:?}");
        assert!(
            label.starts_with("3 \u{00B7} load on") && label.ends_with('\u{2026}'),
            "{label}"
        );
        assert_eq!(rect.top(), 300.0, "hanging under the plot");
    }

    /// Close neighbours keep their numbers and lose their notes; closer ones
    /// slide apart, and a flag pushed off its own line is left out.
    #[test]
    fn close_flags_keep_their_numbers() {
        let laid = flags(&[(400.0, 4, "fan on"), (430.0, 5, "load off")]);
        let labels: Vec<&str> = laid.iter().map(|(_, _, l)| l.as_str()).collect();
        assert_eq!(labels, ["4", "5"]);
        assert!(laid[0].1.right() < laid[1].1.left(), "no overlap");

        // 18 px apart, as two markers 1.2 s apart on a one-minute window: the
        // second slides right, its point still on its tag.
        let laid = flags(&[(400.0, 3, ""), (418.0, 4, "")]);
        assert_eq!(laid.len(), 2, "{laid:?}");
        assert!(laid[0].1.right() + 4.0 <= laid[1].1.left());
        assert!(laid[1].1.left() + 5.0 <= 418.0);

        let laid = flags(&[(400.0, 4, ""), (402.0, 5, "")]);
        let labels: Vec<&str> = laid.iter().map(|(_, _, l)| l.as_str()).collect();
        assert_eq!(labels, ["4"], "the second would leave its line");
    }

    /// At the plot's edge a flag slides inward, its line still under it, and a
    /// neighbour it reaches moves over.
    #[test]
    fn an_edge_flag_slides_in_and_its_neighbour_moves_over() {
        let laid = flags(&[(3.0, 1, "supply warm-up done and settled"), (150.0, 2, "")]);
        assert_eq!(laid.len(), 2, "{laid:?}");
        let (x, rect, _) = &laid[0];
        assert_eq!(rect.left(), 0.0);
        assert!(rect.left() <= *x && *x <= rect.right());
        assert!(laid[0].1.right() + 4.0 <= laid[1].1.left());
        assert!(laid[1].1.left() <= 150.0 && 150.0 <= laid[1].1.right());
    }
}
