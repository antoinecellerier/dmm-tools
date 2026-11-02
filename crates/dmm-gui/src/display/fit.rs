//! How the big meter sizes its reading: the cached text ratios, the
//! candidate layouts and the pick between them.

use eframe::egui::Vec2;

use super::aux_grid::AuxGridMetrics;
use super::{MODE_SIZE_RATIO, SEPARATOR_WIDTH, divider_gap};

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
pub(super) enum ReadingLayout {
    /// Value, then the sub-value rows, then the mode line wrapped under them.
    TwoLine,
    /// Value │ mode on one line, the sub-value rows under it.
    Below,
    /// Value │ sub-values │ mode, all on one row.
    Beside,
}

/// The sub-value grid as the fit weighs it: its text, laid out once, and
/// the pixel spacing egui puts around it.
pub(super) struct GridFit {
    pub(super) metrics: AuxGridMetrics,
    /// egui's `interact_size`, the floor under every grid cell.
    pub(super) min_cell: Vec2,
    /// egui's vertical `item_spacing`, left under the grid as under any
    /// widget.
    pub(super) spacing_y: f32,
    /// Whether the mode line carries selectors, which space its dividers
    /// wider ([`divider_gap`]).
    pub(super) selectors: bool,
}

impl GridFit {
    /// The grid's size beside or under a value at `value_size`.
    pub(super) fn size(&self, value_size: f32) -> Vec2 {
        self.metrics
            .size(value_size * MODE_SIZE_RATIO, self.min_cell)
    }

    /// What the beside layout adds to the line's width at `value_size`: the
    /// grid, a second separator, and the spacing either side of it.
    pub(super) fn beside_extra_width(&self, value_size: f32) -> f32 {
        self.size(value_size).x
            + SEPARATOR_WIDTH
            + 2.0 * divider_gap(value_size * MODE_SIZE_RATIO, self.selectors)
    }
}

/// The largest font size at which `fits` holds, searched up to `limit`; 0
/// when even the smallest does not fit. `fits` must grow monotonically
/// stricter with the size, as every layout's extent does.
pub(super) fn largest_fitting_size(limit: f32, fits: impl Fn(f32) -> bool) -> f32 {
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
pub(super) fn pick_layout(
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
pub(super) struct LastFit {
    pub(super) layout: ReadingLayout,
    pub(super) size: f32,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::BASE_READING_FONT_SIZE;
    use eframe::egui::vec2;

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
}
