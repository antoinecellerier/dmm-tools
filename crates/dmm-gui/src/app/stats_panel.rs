//! The statistics panel: min/max/mean/σ and the running integral of the
//! session since its last reset and of the readings the graph shows, as one
//! table that takes the shape its width allows.
//!
//! In the reading column (and the big meter) the table is transposed when it
//! fits, one row per group; else one column per group; else stacked, one
//! group under the other. In the strip over the graph in a narrow window,
//! where height is what counts, it is transposed when it fits and otherwise a
//! wrapped line per group, `Min 0.00 mA · Max 108.51 mA · …`.
//!
//! The columns are as wide as the widest figure the panel has drawn since its
//! figures started over ([`StatsWidths`]), never as wide as the current one,
//! so a figure ticking from 9.99 to 10.00 never moves a column or changes the
//! shape.

use std::borrow::Cow;

use dmm_lib::stats::{self, RunningStats, ShownDigits};
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Color32, FontId, Ui, text::LayoutJob, text::TextFormat};

use super::App;
use crate::a11y::ResponseA11yExt;

/// Where the panel is drawn, which decides the shapes it may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatsPlace {
    /// The reading column beside the graph, or under the big meter's
    /// reading: transposed, else one column per group, else stacked.
    Column,
    /// The strip over the graph in a narrow window: transposed, else a
    /// wrapped line per group.
    Strip,
}

/// The shape the table is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    /// A row per group, a column per statistic.
    Transposed,
    /// A column per group, a row per statistic.
    Columns,
    /// The groups one under another, a row per statistic each.
    Stacked,
    /// A wrapped line of figures per group.
    Lines,
}

/// One figure as it is drawn: the number apart from its unit, so numbers can
/// end on one edge and units start on another.
#[derive(Debug, Clone, PartialEq)]
struct Figure {
    number: String,
    unit: String,
}

impl Figure {
    /// A figure there is nothing to write for yet.
    fn absent() -> Self {
        Self {
            number: crate::NO_DATA.to_string(),
            unit: String::new(),
        }
    }

    /// The figure as one string, `0.00 mA`.
    fn text(&self) -> String {
        if self.unit.is_empty() {
            self.number.clone()
        } else {
            format!("{} {}", self.number, self.unit)
        }
    }
}

/// The statistics a group writes, in the order they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stat {
    Min,
    Max,
    Mean,
    Sd,
    Integral,
}

impl Stat {
    const ALL: [Stat; 5] = [Stat::Min, Stat::Max, Stat::Mean, Stat::Sd, Stat::Integral];

    fn caption(self, l10n: &Localizer) -> Cow<'_, str> {
        match self {
            Stat::Min => tr!(l10n, "stats-min"),
            Stat::Max => tr!(l10n, "stats-max"),
            Stat::Mean => tr!(l10n, "stats-mean"),
            Stat::Sd => tr!(l10n, "stats-sd"),
            Stat::Integral => tr!(l10n, "stats-integral"),
        }
    }
}

/// Which readings a group's figures cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Group {
    /// Since the last reset, over the meter's main reading.
    Session,
    /// The readings the graph shows, in the unit it plots.
    Visible,
}

impl Group {
    fn name(self, l10n: &Localizer) -> Cow<'_, str> {
        match self {
            Group::Session => tr!(l10n, "stats-session"),
            Group::Visible => tr!(l10n, "stats-visible"),
        }
    }
}

/// How a group writes its figures.
#[derive(Clone, Copy)]
enum FigureStyle<'a> {
    /// In a range and to the resolution the meter showed the session in, by
    /// [`dmm_shared::stat_figure`], as `dmm-cli read`'s summary writes them.
    AsShown(&'a ShownDigits),
    /// To five significant digits in their own prefix: what the meter
    /// showed is the main reading's, and a plotted sub-value (a Raw value, a
    /// MIN/MAX in another prefix) may share its base unit and not its ranges.
    Significant,
    /// As the graph writes a plotted level ([`Graph::decimals`] of a point,
    /// of a mean): an NCV level's Min and Max are whole numbers, its mean
    /// and σ get one decimal.
    ///
    /// [`Graph::decimals`]: crate::graph::Graph::decimals
    Level,
}

/// One group's figures, written once a frame for every shape to draw.
#[derive(Debug, Clone)]
struct FormattedStatsGroup {
    group: Group,
    min: Figure,
    max: Figure,
    avg: Figure,
    /// Sample standard deviation, written like the average.
    sd: Figure,
    /// `None` when the unit has no integral.
    integral: Option<Figure>,
    count: u64,
    /// Seconds from the first to the last reading the figures cover.
    span: Option<f64>,
}

impl FormattedStatsGroup {
    /// The figures of `stats`, captioned with `unit` and written in `style`.
    ///
    /// The integral arrives already scaled to its display unit (see
    /// [`stats::integral_display`]) as `(value, display_unit)`, and is
    /// written by [`dmm_shared::stat_figure`] to five significant digits.
    fn new(
        group: Group,
        stats: &RunningStats,
        unit: &str,
        style: FigureStyle<'_>,
        integral: Option<(f64, &str)>,
        span: Option<f64>,
    ) -> Self {
        let (point, mean, shown) = match style {
            FigureStyle::AsShown(shown) => (None, None, Some(shown)),
            FigureStyle::Significant => (None, None, None),
            FigureStyle::Level => (Some(0), Some(1), None),
        };
        let figure = |v: Option<f64>, decimals: Option<usize>| -> Figure {
            match (v, decimals) {
                (Some(val), Some(decimals)) => Figure {
                    number: format!("{val:.decimals$}"),
                    unit: unit.to_string(),
                },
                (Some(val), None) => {
                    let (number, unit) = dmm_shared::stat_figure(val, unit, shown);
                    Figure {
                        number,
                        unit: unit.into_owned(),
                    }
                }
                (None, _) => Figure::absent(),
            }
        };
        let integral = integral.map(|(val, disp_unit)| {
            let (number, disp_unit) = dmm_shared::stat_figure(val, disp_unit, None);
            Figure {
                number,
                unit: disp_unit.into_owned(),
            }
        });
        Self {
            group,
            min: figure(stats.min, point),
            max: figure(stats.max, point),
            avg: figure(stats.avg(), mean),
            sd: figure(stats.std_dev(), mean),
            integral,
            count: stats.count,
            span,
        }
    }

    /// The figure of `stat`, `None` for an integral the unit lacks.
    fn figure(&self, stat: Stat) -> Option<&Figure> {
        match stat {
            Stat::Min => Some(&self.min),
            Stat::Max => Some(&self.max),
            Stat::Mean => Some(&self.avg),
            Stat::Sd => Some(&self.sd),
            Stat::Integral => self.integral.as_ref(),
        }
    }

    /// Whether `other` writes exactly the same figures over as many
    /// readings: the visible window covering the whole session.
    fn repeats(&self, other: &Self) -> bool {
        self.count == other.count
            && self.min == other.min
            && self.max == other.max
            && self.avg == other.avg
            && self.sd == other.sd
            && self.integral == other.integral
    }
}

/// A span as the graph's time axis writes one: `59 s`, `1m 49s`, `2h 05m`.
fn span_text(secs: Option<f64>) -> String {
    let Some(secs) = secs.filter(|s| s.is_finite() && *s >= 0.0) else {
        return crate::NO_DATA.to_string();
    };
    let whole = secs.floor() as u64;
    if whole < 60 {
        format!("{whole} s")
    } else if whole < 3600 {
        format!("{}m {:02}s", whole / 60, whole % 60)
    } else {
        format!("{}h {:02}m", whole / 3600, whole % 3600 / 60)
    }
}

/// The least gap between two columns, which the shape is picked at, and the
/// most the room left over spreads them to: tight in a narrow panel, airy
/// in a wide one.
const MIN_GAP: f32 = 8.0;
const MAX_GAP: f32 = 24.0;

/// Between two figures on a wrapped line.
const SEPARATOR: &str = " \u{b7} ";

/// Reset's icon: an arrow turning back, as an undo is drawn.
const RESET_ICON: &str = "\u{21BA}";
/// Its size at scale 1: the captions', so it sits in the line as a glyph.
const RESET_ICON_SIZE: f32 = 12.0;

/// The span template a slot is sized for, so the table holds still as the
/// span grows through the first hour.
const SPAN_TEMPLATE: &str = "59m 59s";

/// Characters the number column is always wide enough for: a sign, five
/// digits and the point.
const MIN_NUMBER_CHARS: usize = 7;
/// A base unit and a prefix.
const MIN_UNIT_CHARS: usize = 2;
/// An integral's unit and a prefix, `µAh`.
const MIN_INTEGRAL_UNIT_CHARS: usize = 3;
/// Digits of a count, `#1103`, until a session counts more.
const MIN_COUNT_DIGITS: usize = 4;

/// The widest figure parts the panel has drawn since its figures started
/// over, in characters. The columns are sized from these, not from the
/// figures of the frame: a figure that grows widens its column once, and one
/// that shrinks back moves nothing, so the table never flips between shapes
/// as the readings change.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub(super) struct StatsWidths {
    /// When the session's figures started, as last seen: they start over
    /// on Reset, Clear and a new series, drawn or not.
    since: Option<std::time::Instant>,
    /// The groups drawn, as last seen: hiding the graph or the cursors
    /// takes the widths their figures brought away with them.
    groups: Vec<Group>,
    number: usize,
    unit: usize,
    integral_unit: usize,
    count: usize,
}

impl StatsWidths {
    /// Widen for `groups`, after starting over if the session did, which
    /// began `since`, or the groups drawn changed. Returns whether a width
    /// changed.
    fn update(
        &mut self,
        since: Option<std::time::Instant>,
        groups: &[FormattedStatsGroup],
    ) -> bool {
        let before = self.fit_key();
        if since != self.since
            || !groups
                .iter()
                .map(|g| g.group)
                .eq(self.groups.iter().copied())
        {
            *self = Self {
                since,
                groups: groups.iter().map(|g| g.group).collect(),
                ..Self::default()
            };
        }
        let chars = |s: &str| s.chars().count();
        for g in groups {
            for f in [&g.min, &g.max, &g.avg, &g.sd] {
                self.number = self.number.max(chars(&f.number));
                self.unit = self.unit.max(chars(&f.unit));
            }
            if let Some(f) = &g.integral {
                self.number = self.number.max(chars(&f.number));
                self.integral_unit = self.integral_unit.max(chars(&f.unit));
            }
            self.count = self.count.max(g.count.to_string().len());
        }
        self.fit_key() != before
    }

    fn number(&self) -> usize {
        self.number.max(MIN_NUMBER_CHARS)
    }

    fn unit(&self) -> usize {
        self.unit.max(MIN_UNIT_CHARS)
    }

    fn integral_unit(&self) -> usize {
        self.integral_unit.max(MIN_INTEGRAL_UNIT_CHARS)
    }

    fn count(&self) -> usize {
        self.count.max(MIN_COUNT_DIGITS)
    }

    /// The count as wide as its column is kept for, `#0000`.
    fn count_template<'a>(&self, l10n: &'a Localizer) -> Cow<'a, str> {
        tr!(l10n, "stats-count", count = "0".repeat(self.count()))
    }

    /// What the big meter re-fits on: a column that widened can change the
    /// table's shape, and with it the height left for the reading.
    pub(super) fn fit_key(&self) -> [usize; 4] {
        [
            self.number(),
            self.unit(),
            self.integral_unit(),
            self.count(),
        ]
    }
}

/// Width of `text` in `font` as it is drawn, unrounded: a width rounded up
/// to the pixel would move a right-aligned figure off its edge.
fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    let mut job = LayoutJob::simple_singleline(text.to_string(), font.clone(), Color32::WHITE);
    job.round_output_to_gui = false;
    ui.fonts_mut(|f| f.layout_job(job).size().x)
}

/// The fonts, colours and widths every shape is laid out from.
#[derive(Clone)]
struct Metrics {
    /// Captions and group names.
    main: FontId,
    /// The heading, the statistic captions over a transposed table, and the
    /// span and count.
    sub: FontId,
    /// Numbers and units, and the wrapped lines.
    mono: FontId,
    /// Reset's icon.
    icon: FontId,
    /// One monospace cell.
    digit: f32,
    /// Between two columns: [`MIN_GAP`] when the shape is picked, up to
    /// [`MAX_GAP`] once the room left over is spread between them.
    gap: f32,
    /// The text's scale, the big meter's.
    scale: f32,
    /// Between a group's name, span and count.
    small: f32,
    /// A wrapped line's continuation.
    indent: f32,
    number_w: f32,
    unit_w: f32,
    integral_unit_w: f32,
    /// The widest caption of the rows drawn, in `main`.
    caption_w: f32,
    /// The widest group name, in `main`.
    name_w: f32,
    /// Each group's name in `sub`, in the groups' order: over its column.
    group_names: Vec<f32>,
    /// Each statistic's caption in `sub`, in the statistics' order: over its
    /// column in a transposed table.
    stat_captions: Vec<f32>,
    /// Session's name in `main`, which Reset follows.
    session_w: f32,
    /// The slot a group's name takes at the start of its row or line: the
    /// widest name, or Session's with Reset after it.
    name_slot: f32,
    span_w: f32,
    count_w: f32,
    heading_w: f32,
    reset_w: f32,
    /// A space in `main`, `sub` and `mono`.
    space: [f32; 3],
    text: Color32,
    weak: Color32,
    heading: Color32,
}

impl Metrics {
    /// The unit slot of `stat`.
    fn unit_slot(&self, stat: Stat) -> f32 {
        match stat {
            Stat::Integral => self.integral_unit_w,
            _ => self.unit_w,
        }
    }

    /// The unit column shared by every row of a group.
    fn unit_column(&self, stats: &[Stat]) -> f32 {
        stats.iter().map(|s| self.unit_slot(*s)).fold(0.0, f32::max)
    }

    fn format(&self, font: &FontId, color: Color32) -> TextFormat {
        TextFormat {
            font_id: font.clone(),
            color,
            valign: egui::Align::BOTTOM,
            ..Default::default()
        }
    }

    fn space_in(&self, font: &FontId) -> f32 {
        if *font == self.mono {
            self.space[2]
        } else if *font == self.sub {
            self.space[1]
        } else {
            self.space[0]
        }
    }
}

/// One drawn line of the table as a single galley: each cell a section
/// placed at its x by its leading space, after a real space, so a selection
/// copies the line as `Min 0.00 mA 0.00 mA` and a screen reader reads it as
/// one line. Session's line leaves a slot for Reset, which splits it into
/// the galleys either side.
struct RowJob {
    job: LayoutJob,
    /// Where the line's next text may start, from the line's left edge.
    x: f32,
    /// The text before Reset, and where Reset starts.
    before_reset: Option<(LayoutJob, f32)>,
}

fn empty_job() -> LayoutJob {
    LayoutJob {
        round_output_to_gui: false,
        ..Default::default()
    }
}

impl RowJob {
    fn new() -> Self {
        Self {
            job: empty_job(),
            x: 0.0,
            before_reset: None,
        }
    }

    /// Leave Reset's slot at `x`, after a small gap, `m.reset_w` wide: the
    /// text after it goes in a galley of its own.
    fn reset_slot(&mut self, m: &Metrics, x: f32) {
        let before = std::mem::replace(&mut self.job, empty_job());
        self.before_reset = Some((before, x));
        self.x = x + m.reset_w;
    }

    /// `text`, `width` wide, starting at `x` or right after what is there.
    fn put(&mut self, m: &Metrics, x: f32, text: &str, width: f32, font: &FontId, color: Color32) {
        let format = m.format(font, color);
        let (text, start) = if self.job.text.is_empty() {
            (Cow::Borrowed(text), self.x)
        } else {
            (Cow::Owned(format!(" {text}")), self.x + m.space_in(font))
        };
        let lead = (x - start).max(0.0);
        self.job.append(&text, lead, format);
        self.x = start + lead + width;
    }

    /// `text` ending at `right`.
    fn put_right(
        &mut self,
        m: &Metrics,
        ui: &Ui,
        right: f32,
        text: &str,
        font: &FontId,
        color: Color32,
    ) {
        let w = text_width(ui, text, font);
        self.put(m, right - w, text, w, font, color);
    }

    /// `text` starting at `left`.
    fn put_left(
        &mut self,
        m: &Metrics,
        ui: &Ui,
        left: f32,
        text: &str,
        font: &FontId,
        color: Color32,
    ) {
        let w = text_width(ui, text, font);
        self.put(m, left, text, w, font, color);
    }
}

/// The widths of each shape for these groups, and the one that fits
/// `available` in `place`.
fn pick_shape(
    m: &Metrics,
    groups: &[FormattedStatsGroup],
    stats: &[Stat],
    place: StatsPlace,
    available: f32,
) -> Shape {
    let transposed = transposed_width(m, stats);
    if transposed <= available {
        return Shape::Transposed;
    }
    match place {
        StatsPlace::Strip => Shape::Lines,
        StatsPlace::Column if columns_width(m, groups, stats) <= available => Shape::Columns,
        StatsPlace::Column => Shape::Stacked,
    }
}

/// `m` with the room `shape` leaves in `available` spread between its
/// columns: the widest gap up to [`MAX_GAP`] that still fits. The shape was
/// picked at the least gap, so this never changes it.
///
/// Searched rather than worked out: the gap after Session's column grows
/// nothing while Reset already holds the next name off, so the width is
/// not one slope in the gap. It never shrinks as the gap grows, so the gap
/// found only grows with the room.
fn spread(
    m: &Metrics,
    groups: &[FormattedStatsGroup],
    stats: &[Stat],
    shape: Shape,
    available: f32,
) -> Metrics {
    if !matches!(shape, Shape::Columns | Shape::Transposed) {
        return m.clone();
    }
    // One copy, its gap tried in turn.
    let mut trial = m.clone();
    let mut fits = |gap: f32| {
        trial.gap = gap;
        let width = match shape {
            Shape::Columns => columns_width(&trial, groups, stats),
            _ => transposed_width(&trial, stats),
        };
        width <= available
    };
    let (mut lo, mut hi) = (m.gap, MAX_GAP * m.scale);
    if fits(hi) {
        lo = hi;
    } else {
        // A tenth of a point is finer than anyone sees.
        while hi - lo > 0.1 {
            let mid = (lo + hi) / 2.0;
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    Metrics {
        gap: lo,
        ..m.clone()
    }
}

/// The first column of a transposed table: a group's name slot, its span
/// and its count, or the heading over them.
fn transposed_first(m: &Metrics) -> f32 {
    m.heading_w
        .max(m.name_slot + m.small + m.span_w + m.small + m.count_w)
}

/// The number slot of `stat`'s column in a transposed table: its caption is
/// centred over the numbers and units, and a caption wider than them
/// widens the slot.
fn transposed_number(m: &Metrics, stats: &[Stat], i: usize) -> f32 {
    m.number_w
        .max(m.stat_captions[i] - m.digit - m.unit_slot(stats[i]))
}

fn transposed_width(m: &Metrics, stats: &[Stat]) -> f32 {
    let columns: f32 = (0..stats.len())
        .map(|i| m.gap + transposed_number(m, stats, i) + m.digit + m.unit_slot(stats[i]))
        .sum();
    transposed_first(m) + columns
}

/// One group's column in a column table.
struct Column {
    /// Where its numbers end and, a cell on, its units start.
    number_edge: f32,
    /// The middle of its cell, which its name is centred on.
    centre: f32,
}

/// Where a column table puts things: its caption column's width, each
/// group's column, and how wide it all is.
///
/// Each group's name is centred over its numbers and units, so a name
/// longer than them (a translation) has room either side. The heading sits
/// in the corner over the captions, on the line of the names, reaching right
/// into the space the first name leaves before it: the caption column widens
/// only by what the heading still lacks there. Reset follows Session's name,
/// centred with it, and a column moves right by what the name before would
/// otherwise touch.
fn columns_layout(
    m: &Metrics,
    groups: &[FormattedStatsGroup],
    stats: &[Stat],
) -> (f32, Vec<Column>, f32) {
    let unit_col = m.unit_column(stats);
    // Session's name is centred with Reset after it, as one.
    let names: Vec<f32> = groups
        .iter()
        .zip(&m.group_names)
        .map(|(g, name)| match g.group {
            Group::Session => name + m.small + m.reset_w,
            _ => *name,
        })
        .collect();
    // A cell is the number slot (wide enough for the span ending on its
    // edge), a cell's gap and the unit column; a name wider than that
    // widens the number slot.
    let cells: Vec<(f32, f32)> = names
        .iter()
        .map(|name| {
            let number = m.number_w.max(m.span_w);
            let cell = number + m.digit + unit_col;
            let number = number + (name - cell).max(0.0);
            (number, number + m.digit + unit_col)
        })
        .collect();
    let first_name_at = cells
        .first()
        .zip(names.first())
        .map_or(0.0, |((_, cell), name)| m.gap + (cell - name) / 2.0);
    let caption = m.caption_w.max(m.heading_w + m.gap - first_name_at);
    let mut columns = Vec::with_capacity(cells.len());
    let mut x = caption;
    // Where the line of names ends (Reset included), and where the line of
    // counts does: a count may run on past its unit slot.
    let mut names_end = 0.0_f32;
    let mut counts_end = 0.0_f32;
    for (i, ((number, cell), name)) in cells.iter().zip(&names).enumerate() {
        let mut start = x + m.gap;
        if i > 0 {
            // Clear of the name before, and of the count before by the span.
            let name_at = start + (cell - name) / 2.0;
            let span_at = start + number - m.span_w;
            let clash = (names_end + m.small - name_at).max(counts_end + m.small - span_at);
            start += clash.max(0.0);
        }
        let centre = start + cell / 2.0;
        names_end = centre + name / 2.0;
        counts_end = start + number + m.digit + m.count_w;
        columns.push(Column {
            number_edge: start + number,
            centre,
        });
        x = start + cell;
    }
    // The last count may run on past its unit slot, Reset past the cell.
    let width = columns.last().map_or(caption, |c| {
        (c.number_edge + m.digit + unit_col.max(m.count_w)).max(names_end)
    });
    (caption, columns, width)
}

fn columns_width(m: &Metrics, groups: &[FormattedStatsGroup], stats: &[Stat]) -> f32 {
    columns_layout(m, groups, stats).2
}

impl App {
    /// The Statistics section, in `place`, its text `scale` times its usual
    /// size (the big meter's); returns the shape it took.
    pub(super) fn show_stats_section(
        &mut self,
        ui: &mut Ui,
        place: StatsPlace,
        scale: f32,
    ) -> Shape {
        // The unit the figures are read in, not the latest reading's: a word
        // the meter shows instead of a reading, or an overload in another
        // decade ("OL MΩ" between kΩ readings), leaves the series alone, so
        // the figures still belong to the readings before it.
        // Blank without a reading: `clear_session` drops `last_measurement`
        // but `SeriesStats::reset` keeps its series, and the integral's row
        // should vanish with the reading it describes.
        let session = &self.capture.session;
        let unit = match self.last_measurement {
            Some(_) => session.unit().unwrap_or(""),
            None => "",
        };
        let integral = self
            .last_measurement
            .as_ref()
            .and_then(|_| session.integral_display());
        let session_stats = session.stats();
        let shown = session.shown();
        let mut groups = vec![FormattedStatsGroup::new(
            Group::Session,
            &session_stats,
            unit,
            FigureStyle::AsShown(shown),
            integral,
            session.span_secs(),
        )];
        // The visible group reports what the graph draws, which is not the
        // main reading's unit when a sub-value is plotted; there whenever the
        // graph is, so panning over an empty stretch leaves the table as it is.
        let graph_shown =
            self.settings.show_graph && self.big_meter_mode == super::BigMeterMode::Off;
        if graph_shown {
            let (visible_stats, visible_raw) = self.graph.visible_figures();
            let visible_unit = self.graph.plotted_unit();
            let visible_integral =
                visible_raw.and_then(|raw| stats::integral_display(raw, visible_unit));
            // The visible group takes the session's ranges too, so a window
            // lying wholly before a range the meter has since left may be
            // written in a range it did not show itself; accepted, as the two
            // groups then read alike.
            let style = if self.graph.plots_levels() {
                FigureStyle::Level
            } else if self.graph.plots_main() {
                FigureStyle::AsShown(shown)
            } else {
                FigureStyle::Significant
            };
            groups.push(FormattedStatsGroup::new(
                Group::Visible,
                &visible_stats,
                visible_unit,
                style,
                visible_integral,
                self.graph.visible_data_span_secs(),
            ));
        }
        // A column that widened may change the shape, and with it the height
        // the big meter fitted its reading to with the old widths: another
        // frame fits again.
        if self
            .stats_widths
            .update(self.capture.session.started_at(), &groups)
        {
            ui.ctx().request_repaint();
        }

        // A handle, as the Reset button below borrows the app.
        let l10n = self.l10n.clone();
        let stats: Vec<Stat> = Stat::ALL
            .into_iter()
            .filter(|s| groups.iter().any(|g| g.figure(*s).is_some()))
            .collect();
        let m = self.stats_metrics(ui, &l10n, &stats, &groups, scale);
        let available = ui.available_width();
        let shape = pick_shape(&m, &groups, &stats, place, available);
        let m = spread(&m, &groups, &stats, shape, available);
        match shape {
            Shape::Transposed => self.stats_transposed(ui, &l10n, &m, &groups, &stats),
            Shape::Columns => self.stats_columns(ui, &l10n, &m, &groups, &stats),
            Shape::Stacked => self.stats_stacked(ui, &l10n, &m, &groups, &stats),
            Shape::Lines => self.stats_lines(ui, &l10n, &m, &groups, &stats),
        }
        if groups[0].integral.is_some() {
            self.show_integral_gap_warning(ui, m.sub.size);
        }
        self.show_alarm_counts(ui, m.main.size);
        shape
    }

    fn stats_metrics(
        &self,
        ui: &Ui,
        l10n: &Localizer,
        stats: &[Stat],
        groups: &[FormattedStatsGroup],
        scale: f32,
    ) -> Metrics {
        let main = FontId::proportional(12.0 * scale);
        let sub = FontId::proportional(11.0 * scale);
        let mono = FontId::monospace(12.0 * scale);
        let digit = text_width(ui, "0", &mono);
        let widths = &self.stats_widths;
        let count_template = widths.count_template(l10n);
        let span_w = groups
            .iter()
            .map(|g| text_width(ui, &span_text(g.span), &sub))
            .fold(text_width(ui, SPAN_TEMPLATE, &sub), f32::max);
        let icon = FontId::proportional(RESET_ICON_SIZE * scale);
        let reset_w = text_width(ui, RESET_ICON, &icon) + 2.0 * ui.spacing().button_padding.x;
        let name_w = groups
            .iter()
            .map(|g| text_width(ui, &g.group.name(l10n), &main))
            .fold(0.0, f32::max);
        let session_w = text_width(ui, &Group::Session.name(l10n), &main);
        let small = 6.0 * scale;
        let visuals = ui.visuals();
        Metrics {
            caption_w: stats
                .iter()
                .map(|s| text_width(ui, &s.caption(l10n), &main))
                .fold(0.0, f32::max),
            name_w,
            group_names: groups
                .iter()
                .map(|g| text_width(ui, &g.group.name(l10n), &sub))
                .collect(),
            stat_captions: stats
                .iter()
                .map(|s| text_width(ui, &s.caption(l10n), &sub))
                .collect(),
            session_w,
            name_slot: name_w.max(session_w + small + reset_w),
            span_w,
            count_w: text_width(ui, &count_template, &sub),
            heading_w: text_width(ui, &tr!(l10n, "stats-heading"), &sub),
            reset_w,
            number_w: digit * widths.number() as f32,
            unit_w: digit * widths.unit() as f32,
            integral_unit_w: digit * widths.integral_unit() as f32,
            digit,
            gap: MIN_GAP * scale,
            scale,
            small,
            indent: 12.0 * scale,
            space: [
                text_width(ui, " ", &main),
                text_width(ui, " ", &sub),
                text_width(ui, " ", &mono),
            ],
            text: visuals.text_color(),
            weak: visuals.weak_text_color(),
            heading: self.settings.theme_colors(visuals.dark_mode).heading(),
            main,
            sub,
            mono,
            icon,
        }
    }

    /// The colour of `group`'s figures: dimmed while the visible window
    /// repeats the session's.
    fn figure_color(
        m: &Metrics,
        groups: &[FormattedStatsGroup],
        g: &FormattedStatsGroup,
    ) -> Color32 {
        match g.group {
            Group::Visible if g.repeats(&groups[0]) => m.weak,
            _ => m.text,
        }
    }

    /// `1103 readings over 1m 49s`, what a screen reader says of a group's
    /// span and count.
    fn group_summary(l10n: &Localizer, g: &FormattedStatsGroup) -> String {
        tr!(
            l10n,
            "stats-count.a11y",
            count = g.count,
            span = span_text(g.span)
        )
        .into_owned()
    }

    /// What a screen reader says of a figure: its number, then its unit's
    /// name.
    fn spoken_figure(l10n: &Localizer, f: &Figure) -> String {
        if f.unit.is_empty() {
            f.number.clone()
        } else {
            let unit = crate::display::spoken_unit(l10n, &f.unit, &f.number);
            format!("{} {unit}", f.number)
        }
    }

    /// A line of the table, Reset in the slot its row left for it: right
    /// after Session's name, the figures it starts over. The text either side
    /// is a galley of its own, so the button takes its own hover and click,
    /// and a selection still copies the line as it reads.
    fn stats_line(&mut self, ui: &mut Ui, m: &Metrics, row: RowJob, a11y: &str) {
        let label = |ui: &mut Ui, job: LayoutJob, said: &str| {
            ui.add(egui::Label::new(job).extend()).a11y_label(said)
        };
        let Some((before, reset_x)) = row.before_reset else {
            label(ui, row.job, a11y);
            return;
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let first = label(ui, before, a11y);
            ui.add_space((reset_x - first.rect.width()).max(0.0));
            self.reset_button(ui, m, first.rect.height());
            if !row.job.text.is_empty() {
                // What a screen reader needs of the line is in the first
                // galley's name.
                let rest = label(ui, row.job, "");
                crate::a11y::hide_from_screen_readers(ui, rest.id);
            }
        });
    }

    /// The heading's line.
    fn stats_heading(&mut self, ui: &mut Ui, l10n: &Localizer, m: &Metrics) {
        let mut row = RowJob::new();
        let heading = tr!(l10n, "stats-heading");
        row.put_left(m, ui, 0.0, &heading, &m.sub, m.heading);
        self.stats_line(ui, m, row, &heading);
    }

    /// A row per group, a column per statistic, the heading and the
    /// statistics' captions over them.
    fn stats_transposed(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        m: &Metrics,
        groups: &[FormattedStatsGroup],
        stats: &[Stat],
    ) {
        let first = transposed_first(m);
        // Each statistic's number edge, and the middle of its cell, which
        // its caption is centred on.
        let mut edges = Vec::with_capacity(stats.len());
        let mut centres = Vec::with_capacity(stats.len());
        let mut x = first;
        for (i, s) in stats.iter().enumerate() {
            let start = x + m.gap;
            x = start + transposed_number(m, stats, i);
            edges.push(x);
            x += m.digit + m.unit_slot(*s);
            centres.push((start + x) / 2.0);
        }
        let heading = tr!(l10n, "stats-heading");
        let mut row = RowJob::new();
        row.put_left(m, ui, 0.0, &heading, &m.sub, m.heading);
        let mut said = vec![heading.to_string()];
        for ((s, centre), w) in stats.iter().zip(&centres).zip(&m.stat_captions) {
            let caption = s.caption(l10n);
            row.put(m, centre - w / 2.0, &caption, *w, &m.sub, m.heading);
            said.push(caption.into_owned());
        }
        self.stats_line(ui, m, row, &said.join(", "));

        let span_edge = m.name_slot + m.small + m.span_w;
        let count_edge = span_edge + m.small + m.count_w;
        for g in groups {
            let color = Self::figure_color(m, groups, g);
            let name = g.group.name(l10n);
            let summary = Self::group_summary(l10n, g);
            let mut row = RowJob::new();
            row.put_left(m, ui, 0.0, &name, &m.main, m.heading);
            if g.group == Group::Session {
                row.reset_slot(m, m.session_w + m.small);
            }
            row.put_right(m, ui, span_edge, &span_text(g.span), &m.sub, m.weak);
            let count = tr!(l10n, "stats-count", count = g.count);
            row.put_right(m, ui, count_edge, &count, &m.sub, m.weak);
            let mut said = vec![format!("{name}, {summary}")];
            for (s, edge) in stats.iter().zip(&edges) {
                let Some(f) = g.figure(*s) else { continue };
                row.put_right(m, ui, *edge, &f.number, &m.mono, color);
                if !f.unit.is_empty() {
                    row.put_left(m, ui, edge + m.digit, &f.unit, &m.mono, color);
                }
                said.push(format!(
                    "{} {}",
                    s.caption(l10n),
                    Self::spoken_figure(l10n, f)
                ));
            }
            self.stats_line(ui, m, row, &said.join(", "));
        }
    }

    /// A column per group under its name, span and count, a row per
    /// statistic.
    fn stats_columns(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        m: &Metrics,
        groups: &[FormattedStatsGroup],
        stats: &[Stat],
    ) {
        let (_, columns, _) = columns_layout(m, groups, stats);
        let edges: Vec<f32> = columns.iter().map(|c| c.number_edge).collect();
        // The heading in the corner, the names on the numbers' edges, Reset
        // after Session's; then the spans and counts: the span ends where
        // the numbers do, the count starts where the units do.
        let heading = tr!(l10n, "stats-heading");
        let mut names = RowJob::new();
        names.put_left(m, ui, 0.0, &heading, &m.sub, m.heading);
        let mut meta = RowJob::new();
        let mut said = Vec::new();
        for (g, column) in groups.iter().zip(&columns) {
            let edge = &column.number_edge;
            let name = g.group.name(l10n);
            let name_w = text_width(ui, &name, &m.sub);
            let whole = match g.group {
                Group::Session => name_w + m.small + m.reset_w,
                _ => name_w,
            };
            let name_at = column.centre - whole / 2.0;
            names.put(m, name_at, &name, name_w, &m.sub, m.heading);
            if g.group == Group::Session {
                names.reset_slot(m, name_at + name_w + m.small);
            }
            meta.put_right(m, ui, *edge, &span_text(g.span), &m.sub, m.weak);
            let count = tr!(l10n, "stats-count", count = g.count);
            meta.put_left(m, ui, edge + m.digit, &count, &m.sub, m.weak);
            let summary = Self::group_summary(l10n, g);
            said.push(format!("{name}, {summary}"));
        }
        let named = format!("{heading}: {}", said.join("; "));
        self.stats_line(ui, m, names, &named);
        self.stats_line(ui, m, meta, &said.join("; "));
        for s in stats {
            let caption = s.caption(l10n);
            let mut row = RowJob::new();
            row.put_left(m, ui, 0.0, &caption, &m.main, m.text);
            let mut said = Vec::new();
            for (g, edge) in groups.iter().zip(&edges) {
                let Some(f) = g.figure(*s) else { continue };
                let color = Self::figure_color(m, groups, g);
                row.put_right(m, ui, *edge, &f.number, &m.mono, color);
                if !f.unit.is_empty() {
                    row.put_left(m, ui, edge + m.digit, &f.unit, &m.mono, color);
                }
                said.push(format!(
                    "{} {}",
                    g.group.name(l10n),
                    Self::spoken_figure(l10n, f)
                ));
            }
            let said = format!("{caption}: {}", said.join(", "));
            self.stats_line(ui, m, row, &said);
        }
    }

    /// The groups one under another, each under its name, span and count.
    fn stats_stacked(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        m: &Metrics,
        groups: &[FormattedStatsGroup],
        stats: &[Stat],
    ) {
        self.stats_heading(ui, l10n, m);
        // Session's line holds its name, Reset, then its span ending on the
        // numbers' edge: the caption column widens if they would meet.
        let number = m.number_w.max(m.span_w);
        let reset_end = m.session_w + 2.0 * m.small + m.reset_w;
        let caption_col = m
            .caption_w
            .max(m.name_w)
            .max(reset_end + m.span_w - m.gap - number);
        let edge = caption_col + m.gap + number;
        for (i, g) in groups.iter().enumerate() {
            if i > 0 {
                ui.add_space(m.small * 0.5);
            }
            let color = Self::figure_color(m, groups, g);
            let name = g.group.name(l10n);
            let summary = Self::group_summary(l10n, g);
            let mut row = RowJob::new();
            row.put_left(m, ui, 0.0, &name, &m.main, m.heading);
            if g.group == Group::Session {
                row.reset_slot(m, m.session_w + m.small);
            }
            row.put_right(m, ui, edge, &span_text(g.span), &m.sub, m.weak);
            let count = tr!(l10n, "stats-count", count = g.count);
            row.put_left(m, ui, edge + m.digit, &count, &m.sub, m.weak);
            let said = format!("{name}, {summary}");
            self.stats_line(ui, m, row, &said);
            for s in stats {
                let Some(f) = g.figure(*s) else { continue };
                let caption = s.caption(l10n);
                let mut row = RowJob::new();
                row.put_left(m, ui, 0.0, &caption, &m.main, m.text);
                row.put_right(m, ui, edge, &f.number, &m.mono, color);
                if !f.unit.is_empty() {
                    row.put_left(m, ui, edge + m.digit, &f.unit, &m.mono, color);
                }
                let said = format!("{name} {caption}: {}", Self::spoken_figure(l10n, f));
                self.stats_line(ui, m, row, &said);
            }
        }
    }

    /// A wrapped line per group, each figure its caption and value:
    /// the name, then the span and count, dimmed, then the figures as one
    /// run. The lines break where the columns' widths say, not where the
    /// figures of the frame end, so a figure that grows never re-wraps one.
    fn stats_lines(
        &mut self,
        ui: &mut Ui,
        l10n: &Localizer,
        m: &Metrics,
        groups: &[FormattedStatsGroup],
        stats: &[Stat],
    ) {
        let available = ui.available_width();
        let sep = SEPARATOR;
        let sep_w = text_width(ui, sep, &m.mono);
        let count_template = self.stats_widths.count_template(l10n);
        let meta_w = text_width(
            ui,
            &format!("{SPAN_TEMPLATE}{sep}{count_template}"),
            &m.mono,
        );
        let figure_ws: Vec<f32> = stats
            .iter()
            .map(|s| {
                text_width(ui, &format!("{} ", s.caption(l10n)), &m.mono)
                    + m.number_w
                    + m.digit
                    + m.unit_slot(*s)
            })
            .collect();
        let lead = m.name_slot + m.small;
        for g in groups {
            let color = Self::figure_color(m, groups, g);
            let name = g.group.name(l10n);
            let summary = Self::group_summary(l10n, g);
            let count = tr!(l10n, "stats-count", count = g.count);
            let meta = format!("{}{sep}{count}", span_text(g.span));
            // Line breaks from template widths: the span and count first,
            // then each figure.
            let first_room = available - lead;
            let items: Vec<(Option<Stat>, f32)> = std::iter::once((None, meta_w))
                .chain(
                    stats
                        .iter()
                        .zip(&figure_ws)
                        .filter(|(s, _)| g.figure(**s).is_some())
                        .map(|(s, w)| (Some(*s), *w)),
                )
                .collect();
            let lines = break_lines(&items, first_room, available - m.indent, sep_w);
            for (i, line) in lines.iter().enumerate() {
                let mut row = RowJob::new();
                let mut said = Vec::new();
                let mut x = m.indent;
                if i == 0 {
                    row.put_left(m, ui, 0.0, &name, &m.main, m.heading);
                    if g.group == Group::Session {
                        row.reset_slot(m, m.session_w + m.small);
                    }
                    said.push(format!("{name}, {summary}"));
                    x = lead;
                }
                let figures: Vec<String> = line
                    .iter()
                    .filter_map(|s| {
                        let s = (*s)?;
                        let f = g.figure(s)?;
                        let caption = s.caption(l10n);
                        said.push(format!("{caption} {}", Self::spoken_figure(l10n, f)));
                        Some(format!("{caption} {}", f.text()))
                    })
                    .collect();
                // The span and count, dimmed, then the figures as one run:
                // the colour changes once, where the run starts.
                if line.first() == Some(&None) {
                    let text = if figures.is_empty() {
                        meta.clone()
                    } else {
                        format!("{meta}{}", sep.trim_end())
                    };
                    row.put_left(m, ui, x, &text, &m.mono, m.weak);
                    x = row.x + m.space_in(&m.mono);
                }
                if !figures.is_empty() {
                    row.put_left(m, ui, x, &figures.join(sep), &m.mono, color);
                }
                self.stats_line(ui, m, row, &said.join(", "));
            }
        }
    }

    /// `Alarms: 2 high, 0 low` while the **Alarm** row has limits set, a
    /// count for each limit there is. Reset and Clear zero it.
    fn show_alarm_counts(&self, ui: &mut Ui, font_size: f32) {
        let Some(alarm) = &self.alarm else {
            return;
        };
        let limits = alarm.limits();
        let which = match (limits.high, limits.low) {
            (Some(_), Some(_)) => "both",
            (Some(_), None) => "high",
            (None, Some(_)) => "low",
            // An alarm always has a limit.
            (None, None) => return,
        };
        let text = tr!(
            self.l10n,
            "stats-alarm-counts",
            limits = which,
            high = alarm.high_count,
            low = alarm.low_count
        );
        ui.label(egui::RichText::new(text).font(FontId::proportional(font_size)))
            .on_hover_text(tr!(self.l10n, "stats-alarm-counts.hover").as_ref());
    }

    /// The session-stats reset control, drawn like the big-meter toggle: a
    /// small icon button in the weak text colour, so it reads as a control
    /// beside the figures rather than one of them, and fits the line of the
    /// groups' names at any width the column layout takes. Named for a
    /// screen reader.
    fn reset_button(&mut self, ui: &mut Ui, m: &Metrics, height: f32) {
        let weak = ui.visuals().weak_text_color();
        let name = tr!(self.l10n, "stats-reset").into_owned();
        // The line's height: a button any taller would push the line apart.
        let size = egui::vec2(m.reset_w, height);
        if ui
            .add_sized(
                size,
                egui::Button::new(
                    egui::RichText::new(RESET_ICON)
                        .font(m.icon.clone())
                        .color(weak),
                )
                .small()
                .frame_when_inactive(false),
            )
            .on_hover_text(tr!(self.l10n, "stats-reset.hover").as_ref())
            .a11y_label(&name)
            .clicked()
        {
            self.capture.session.reset();
            if let Some(alarm) = &mut self.alarm {
                alarm.clear_counts();
            }
        }
    }

    /// Warn that intervals too long to integrate were skipped.
    fn show_integral_gap_warning(&self, ui: &mut Ui, font_size: f32) {
        if self.capture.session.integrator.skipped_intervals == 0 {
            return;
        }
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let limit = self.capture.session.integrator.max_dt_secs().to_string();
        let count = self.capture.session.integrator.skipped_intervals;
        ui.label(
            egui::RichText::new(tr!(self.l10n, "stats-gaps", count = count, limit = &limit))
                .font(FontId::proportional(font_size))
                .color(tc.status_warning()),
        )
        .on_hover_text(tr!(self.l10n, "stats-gaps.hover", limit = &limit).as_ref());
    }
}

/// Split `items` (each `None` for the span and count, or a statistic, with
/// its template width) into lines: the first `first_room` wide, the others
/// `room`, `sep` between two items on a line. An item wider than its line
/// takes one of its own.
fn break_lines(
    items: &[(Option<Stat>, f32)],
    first_room: f32,
    room: f32,
    sep: f32,
) -> Vec<Vec<Option<Stat>>> {
    let mut lines: Vec<Vec<Option<Stat>>> = Vec::new();
    let mut line = Vec::new();
    let mut used = 0.0;
    for (item, w) in items {
        let limit = if lines.is_empty() { first_room } else { room };
        let add = if line.is_empty() { *w } else { sep + w };
        if !line.is_empty() && used + add > limit {
            lines.push(std::mem::take(&mut line));
            used = *w;
        } else {
            used += add;
        }
        line.push(*item);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::{headless_frame, test_app};
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::{MeasuredValue, Measurement};
    use std::time::{Duration, Instant};

    /// The app after `values` volts, 100 ms apart, as the meter showed them
    /// to four decimals: in the session and on the graph alike.
    fn app_with(values: &[f64]) -> App {
        let mut app = test_app();
        push(&mut app, values);
        app
    }

    fn push(app: &mut App, values: &[f64]) {
        push_in(app, values, "DC V", "V");
    }

    /// As [`push`], in `mode` and `unit`.
    fn push_in(app: &mut App, values: &[f64], mode: &'static str, unit: &'static str) {
        let t0 = app
            .last_measurement
            .as_ref()
            .map_or_else(Instant::now, |m| m.timestamp + Duration::from_millis(100));
        for (i, v) in values.iter().enumerate() {
            let mut m =
                Measurement::test_fixture(MeasuredValue::Normal(*v), unit, StatusFlags::default());
            m.mode = mode.into();
            let shown = format!("{v:.4}");
            m.display_raw = Some(shown.clone());
            m.timestamp = t0 + Duration::from_millis(100 * i as u64);
            app.capture.session.push(&m);
            app.graph.push(*v, m.timestamp, mode, unit, Some(&shown));
            app.last_measurement = Some(m);
        }
    }

    /// A line drawn: its text, where it sits, and each glyph's x.
    #[derive(Debug)]
    struct Drawn {
        text: String,
        rect: egui::Rect,
        /// Each glyph's left x and its advance.
        glyphs: Vec<(f32, f32)>,
    }

    impl Drawn {
        /// The x where the first occurrence of `needle` starts and ends.
        fn span_of(&self, needle: &str) -> (f32, f32) {
            let start = self.text.find(needle).expect(needle);
            let first = self.text[..start].chars().count();
            let n = needle.chars().count();
            let left = self.glyphs[first].0;
            let (x, advance) = self.glyphs[first + n - 1];
            (left, x + advance)
        }
    }

    /// Draw the section in a `width` wide column, `frames` times; the shape
    /// and the lines of the last frame.
    fn draw(app: &mut App, place: StatsPlace, width: f32, frames: usize) -> (Shape, Vec<Drawn>) {
        let ctx = crate::app::styled_context();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1200.0));
        let mut shape = None;
        let mut out = None;
        for _ in 0..frames {
            out = Some(headless_frame(&ctx, Some(screen), None, vec![], |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width, 1200.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    shape = Some(app.show_stats_section(ui, place, 1.0));
                });
            }));
        }
        let mut drawn = Vec::new();
        fn collect(s: &egui::Shape, out: &mut Vec<Drawn>) {
            match s {
                egui::Shape::Text(t) => {
                    let mut glyphs = Vec::new();
                    for row in &t.galley.rows {
                        for g in &row.glyphs {
                            glyphs.push((t.pos.x + row.pos.x + g.pos.x, g.advance_width));
                        }
                    }
                    out.push(Drawn {
                        text: t.galley.text().to_string(),
                        rect: t.visual_bounding_rect(),
                        glyphs,
                    });
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
                _ => {}
            }
        }
        for clipped in &out.expect("a frame").shapes {
            collect(&clipped.shape, &mut drawn);
        }
        // Galleys on one line are one line, as a selection copies them:
        // Session's is split around Reset.
        // Reset's icon is a button, not the line's text.
        drawn.retain(|d| d.text != RESET_ICON);
        drawn.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
        let mut clusters: Vec<Vec<Drawn>> = Vec::new();
        for d in drawn {
            match clusters.last_mut() {
                Some(c) if (c[0].rect.top() - d.rect.top()).abs() < 3.0 => c.push(d),
                _ => clusters.push(vec![d]),
            }
        }
        let lines = clusters
            .into_iter()
            .map(|mut c| {
                c.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
                let mut parts = c.into_iter();
                let mut line = parts.next().expect("a galley");
                for d in parts {
                    line.text.push(' ');
                    line.glyphs.push((line.rect.right(), 0.0));
                    line.text.push_str(&d.text);
                    line.glyphs.extend(d.glyphs);
                    line.rect = line.rect.union(d.rect);
                }
                line
            })
            .collect();
        (shape.expect("drawn"), lines)
    }

    fn line<'a>(drawn: &'a [Drawn], starts: &str) -> &'a Drawn {
        drawn
            .iter()
            .find(|d| d.text.starts_with(starts))
            .unwrap_or_else(|| panic!("no line starts {starts:?} in {drawn:#?}"))
    }

    /// A sine between 2 and 8 V, as the mock's DC V draws.
    fn sine(n: usize) -> Vec<f64> {
        (0..n).map(|i| 5.0 + 3.0 * (i as f64 / 9.0).sin()).collect()
    }

    /// Each place takes the shape its width allows, and nothing is drawn
    /// past the column's edge in any of them, in English or in French.
    #[test]
    fn the_table_takes_the_shape_its_width_allows() {
        for (place, width, want) in [
            (StatsPlace::Strip, 766.0, Shape::Transposed),
            (StatsPlace::Strip, 520.0, Shape::Lines),
            (StatsPlace::Strip, 300.0, Shape::Lines),
            (StatsPlace::Column, 1000.0, Shape::Transposed),
            (StatsPlace::Column, 224.0, Shape::Columns),
            (StatsPlace::Column, 164.0, Shape::Stacked),
        ] {
            for language in ["en-US", "fr"] {
                let mut app = app_with(&sine(60));
                app.l10n = std::sync::Arc::new(crate::l10n::startup(language));
                let (shape, drawn) = draw(&mut app, place, width, 2);
                if language == "en-US" {
                    assert_eq!(shape, want, "{place:?} at {width}");
                }
                for d in &drawn {
                    assert!(
                        d.rect.right() <= width + 0.5,
                        "{language} {place:?} at {width}: {:?} ends at {}",
                        d.text,
                        d.rect.right()
                    );
                }
            }
        }
    }

    /// Each integral is in its own group's row: the screenshot that started
    /// this had the session's after the visible window's line.
    #[test]
    fn each_integral_is_drawn_with_its_group() {
        let mut app = app_with(&sine(60));
        let (_, drawn) = draw(&mut app, StatsPlace::Strip, 766.0, 2);
        let session = line(&drawn, "Session");
        let visible = line(&drawn, "Visible");
        assert!(session.text.contains("V\u{b7}s"), "{}", session.text);
        assert!(visible.text.contains("V\u{b7}s"), "{}", visible.text);
        assert!(session.rect.bottom() <= visible.rect.top() + 1.0);
    }

    /// In columns, a row copies as its caption and its figures, a space
    /// apart; each group's numbers end on one edge and its units start on
    /// another, the span ending on the numbers' and the count starting on
    /// the units'.
    #[test]
    fn the_columns_line_up() {
        let mut app = app_with(&[1.0, 12.5, 3.25]);
        let (shape, drawn) = draw(&mut app, StatsPlace::Column, 240.0, 2);
        assert_eq!(shape, Shape::Columns);
        let min = line(&drawn, "Min");
        assert_eq!(min.text, "Min 1.0000 V 1.0000 V");
        let edge = |d: &Drawn, needle: &str| d.span_of(needle);
        let max = line(&drawn, "Max");
        let (_, min_end) = edge(min, "1.0000");
        let (_, max_end) = edge(max, "12.5000");
        assert!((min_end - max_end).abs() < 0.5, "{min_end} vs {max_end}");
        let (v_min, _) = edge(min, "V");
        let (v_max, _) = edge(max, "V");
        assert!((v_min - v_max).abs() < 0.5, "{v_min} vs {v_max}");
        let meta = drawn
            .iter()
            .find(|d| d.text.contains('#'))
            .expect("the spans and counts");
        let (_, span_end) = edge(meta, " s");
        // Within a pixel: the span and count are in the proportional font,
        // whose glyphs' advances need not add up to the width it measures.
        assert!((span_end - min_end).abs() < 1.0, "{span_end} vs {min_end}");
        let (count_start, _) = edge(meta, "#3");
        assert!(
            (count_start - v_min).abs() < 1.0,
            "{count_start} vs {v_min}"
        );
    }

    /// Figures that tick wider and back never change the shape, move a
    /// column or re-wrap a line, at a width either side of where the shape
    /// changes.
    #[test]
    fn ticking_figures_move_nothing() {
        for (place, width) in [
            (StatsPlace::Column, 224.0),
            (StatsPlace::Strip, 520.0),
            (StatsPlace::Strip, 300.0),
        ] {
            let mut app = app_with(&sine(40));
            let (first, before) = draw(&mut app, place, width, 2);
            let layout = |d: &[Drawn]| -> Vec<(f32, f32)> {
                d.iter().map(|d| (d.rect.top(), d.rect.left())).collect()
            };
            for v in [9.99, 10.0, -9.99, 0.001] {
                push(&mut app, &[v]);
                let (shape, after) = draw(&mut app, place, width, 1);
                assert_eq!(shape, first, "{place:?} at {width} after {v}");
                assert_eq!(layout(&after).len(), layout(&before).len(), "{v}");
                for (a, b) in layout(&after).iter().zip(layout(&before)) {
                    assert!((a.0 - b.0).abs() < 0.5, "{place:?} {v}: a line moved");
                }
            }
        }
    }

    /// The room a wide panel leaves is spread between the columns, up to a
    /// limit; a narrow one keeps them tight.
    #[test]
    fn a_wide_panel_spreads_the_columns() {
        let pitch = |width| {
            let mut app = app_with(&[1.0, 12.5, 3.25]);
            let (shape, drawn) = draw(&mut app, StatsPlace::Column, width, 2);
            assert_eq!(shape, Shape::Columns, "at {width}");
            let min = line(&drawn, "Min");
            let numbers: Vec<f32> = min
                .text
                .match_indices("1.0000")
                .map(|(i, _)| {
                    let first = min.text[..i].chars().count();
                    min.glyphs[first].0
                })
                .collect();
            numbers[1] - numbers[0]
        };
        let (tight, wide) = (pitch(224.0), pitch(384.0));
        // Room to spare at 384: the gap goes to its most, never past it.
        let grown = wide - tight;
        assert!(grown > 10.0, "{tight} then {wide}");
        assert!(grown <= MAX_GAP - MIN_GAP + 0.5, "{tight} then {wide}");
    }

    /// The gap only grows as the panel widens: the room between the
    /// columns once spread past where Reset holds Visible's name off, then
    /// fell back to the least gap.
    #[test]
    fn the_spread_only_grows_with_the_room() {
        let app = test_app();
        let ctx = crate::app::styled_context();
        headless_frame(&ctx, None, None, vec![], |ui| {
            let l10n = app.l10n.clone();
            let f = |n: &str, u: &str| Figure {
                number: n.into(),
                unit: u.into(),
            };
            let group = |group| FormattedStatsGroup {
                group,
                min: f("0.00", "mA"),
                max: f("108.51", "mA"),
                avg: f("20.95", "mA"),
                sd: f("35.53", "mA"),
                integral: Some(f("187.24", "\u{b5}Ah")),
                count: 325,
                span: Some(32.0),
            };
            let groups = [group(Group::Session), group(Group::Visible)];
            let m = app.stats_metrics(ui, &l10n, &Stat::ALL, &groups, 1.0);
            let mut last = 0.0;
            for width in 200..=400 {
                let gap = spread(&m, &groups, &Stat::ALL, Shape::Columns, width as f32).gap;
                assert!(gap >= last, "{last} then {gap} at {width}");
                last = gap;
            }
            assert_eq!(last, MAX_GAP);
        });
    }

    /// A transposed table's captions are centred over their numbers and
    /// units, so a caption wider than them (a translation) has room either
    /// side.
    #[test]
    fn captions_are_centred_over_their_figures() {
        let mut app = app_with(&sine(40));
        let (_, drawn) = draw(&mut app, StatsPlace::Strip, 766.0, 2);
        let captions = line(&drawn, "Statistics");
        let session = line(&drawn, "Session");
        let (min_l, min_r) = captions.span_of("Min");
        let figure = app.capture.session.stats().min.expect("a minimum");
        let figure = format!("{figure:.4} V");
        let (fig_l, fig_r) = session.span_of(&figure);
        let (a, b) = ((min_l + min_r) / 2.0, (fig_l + fig_r) / 2.0);
        assert!((a - b).abs() < 1.5, "Min at {a}, its figure at {b}");
    }

    /// The wrapped lines open with the group's name, span and count, then
    /// the figures, each its caption and value.
    #[test]
    fn a_narrow_strip_writes_each_group_as_a_line() {
        let mut app = app_with(&sine(40));
        let (shape, drawn) = draw(&mut app, StatsPlace::Strip, 520.0, 2);
        assert_eq!(shape, Shape::Lines);
        let session = line(&drawn, "Session");
        assert!(
            session.text.contains(" s \u{b7} #40 \u{b7} Min "),
            "{}",
            session.text
        );
    }

    /// The session's span and count lead its row in a transposed table.
    #[test]
    fn a_transposed_row_opens_with_its_span_and_count() {
        let mut app = app_with(&sine(40));
        let (_, drawn) = draw(&mut app, StatsPlace::Strip, 766.0, 2);
        let session = line(&drawn, "Session");
        assert!(
            session.text.starts_with("Session 3 s #40 "),
            "{}",
            session.text
        );
    }

    /// The columns' widths grow with the figures and start over with them.
    #[test]
    fn the_widths_start_over_with_the_figures() {
        let mut widths = StatsWidths::default();
        let group = |count, number: &str| FormattedStatsGroup {
            group: Group::Session,
            min: Figure {
                number: number.into(),
                unit: "V".into(),
            },
            max: Figure::absent(),
            avg: Figure::absent(),
            sd: Figure::absent(),
            integral: None,
            count,
            span: None,
        };
        let (first, second) = (Instant::now(), Instant::now() + Duration::from_secs(5));
        assert!(widths.update(Some(first), &[group(5, "1234.56789")]));
        assert_eq!(widths.number(), 10);
        assert!(!widths.update(Some(first), &[group(6, "1.0")]));
        assert_eq!(widths.number(), 10, "a narrower figure moves nothing");
        // Started again, whether the panel saw the count drop or not.
        widths.update(Some(second), &[group(900, "1.0")]);
        assert_eq!(widths.number(), MIN_NUMBER_CHARS, "a reset starts over");
        // Figures of a group no longer drawn leave with it.
        let visible = FormattedStatsGroup {
            group: Group::Visible,
            ..group(900, "1234.56789")
        };
        widths.update(Some(second), &[group(900, "1.0"), visible]);
        assert_eq!(widths.number(), 10);
        widths.update(Some(second), &[group(900, "1.0")]);
        assert_eq!(widths.number(), MIN_NUMBER_CHARS, "the graph hidden");
    }

    /// Nothing on the line of spans and counts overlaps at the narrowest
    /// the columns take, with a five-digit count and long spans: ohms, with
    /// no integral, keep the unit column narrow, and a window over the whole
    /// session gives the visible group a long span too.
    #[test]
    fn a_long_count_keeps_clear_of_the_next_column() {
        let mut app = test_app();
        app.graph.time_window_secs = 3600.0;
        push_in(&mut app, &sine(20_000), "\u{3a9}", "\u{3a9}");
        // The narrowest the columns take, where nothing is spread.
        let width = (150..400)
            .map(|w| w as f32)
            .find(|w| draw(&mut app, StatsPlace::Column, *w, 2).0 == Shape::Columns)
            .expect("columns at some width");
        let (shape, drawn) = draw(&mut app, StatsPlace::Column, width, 2);
        assert_eq!(shape, Shape::Columns);
        let meta = drawn
            .iter()
            .find(|d| d.text.contains("#20000"))
            .expect("the spans and counts");
        // Glyphs sit on whole pixels, so a neighbour may start a fraction
        // of one inside the advance before it.
        for pair in meta.glyphs.windows(2) {
            let ((x0, w0), (x1, _)) = (pair[0], pair[1]);
            assert!(
                x1 >= x0 + w0 - 1.0,
                "{:?} overlaps: {:?}",
                meta.text,
                meta.glyphs
            );
        }
    }

    /// Spans as the graph's time axis writes them, whole seconds.
    #[test]
    fn spans_read_like_the_time_axis() {
        assert_eq!(span_text(None), crate::NO_DATA);
        assert_eq!(span_text(Some(2.37)), "2 s");
        assert_eq!(span_text(Some(59.9)), "59 s");
        assert_eq!(span_text(Some(109.0)), "1m 49s");
        assert_eq!(span_text(Some(7500.0)), "2h 05m");
    }

    /// NCV levels in view: Min and Max as the whole numbers they are, the
    /// average to one decimal, and no integral for a level.
    #[test]
    fn visible_levels_read_as_whole_numbers() {
        let mut levels = RunningStats::new();
        for l in [0.0, 2.0, 3.0] {
            levels.push(l);
        }
        let vis =
            FormattedStatsGroup::new(Group::Visible, &levels, "", FigureStyle::Level, None, None);
        let figures = [&vis.min, &vis.max, &vis.avg, &vis.sd].map(|f| f.number.clone());
        assert_eq!(figures, ["0", "3", "1.7", "1.5"]);
        // A level has no unit, so the panel finds no integral to show.
        assert_eq!(stats::integral_display(6.0, ""), None);
    }

    /// A session carried from MΩ down to Ω writes each figure in a range
    /// the meter showed, to that range's decimals, and the integral to five
    /// significant digits, as the CLI summary writes them.
    #[test]
    fn session_figures_keep_the_meters_ranges() {
        let mut session = dmm_lib::stats::SeriesStats::new(false);
        for (display, value, unit) in [
            (" 1.2341", 1.2341, "M\u{3a9}"),
            (" 0.2201", 0.2201, "k\u{3a9}"),
            ("219.82", 219.82, "\u{3a9}"),
        ] {
            let mut m = Measurement::test_fixture(
                MeasuredValue::Normal(value),
                unit,
                StatusFlags::default(),
            );
            m.display_raw = Some(display.to_string());
            session.push(&m);
        }
        let as_shown = FigureStyle::AsShown(session.shown());
        let whole = FormattedStatsGroup::new(
            Group::Session,
            &session.stats(),
            "\u{3a9}",
            as_shown,
            Some((0.000_012_34, "mAh")),
            None,
        );
        let text = |f: &Figure| f.text();
        assert_eq!(text(&whole.min), "219.82 \u{3a9}");
        assert_eq!(text(&whole.max), "1.2341 M\u{3a9}");
        assert_eq!(text(&whole.avg), "411.5133 k\u{3a9}");
        assert_eq!(text(&whole.sd), "712.3810 k\u{3a9}");
        assert_eq!(
            whole.integral.as_ref().map(text).as_deref(),
            Some("12.340 nAh")
        );

        // A plotted sub-value in the same base unit isn't what the meter
        // showed in those ranges: five significant digits in its own prefix.
        let sub_value = FormattedStatsGroup::new(
            Group::Visible,
            &session.stats(),
            "\u{3a9}",
            FigureStyle::Significant,
            None,
            None,
        );
        assert_eq!(text(&sub_value.avg), "411.51 k\u{3a9}");
    }

    /// Lines break where the template widths say: an item too wide for any
    /// line takes one of its own, the first line is shorter by Reset.
    #[test]
    fn lines_break_on_template_widths() {
        let items = [
            (None, 50.0),
            (Some(Stat::Min), 40.0),
            (Some(Stat::Max), 40.0),
            (Some(Stat::Mean), 200.0),
        ];
        let lines = break_lines(&items, 100.0, 120.0, 5.0);
        assert_eq!(
            lines,
            vec![
                vec![None, Some(Stat::Min)],
                vec![Some(Stat::Max)],
                vec![Some(Stat::Mean)],
            ]
        );
    }

    /// Reset, beside Session's name in every shape, says what it does on
    /// hover, and a click starts the session's figures over.
    #[test]
    fn reset_explains_itself_and_resets() {
        for (place, width) in [
            (StatsPlace::Column, 224.0),
            (StatsPlace::Column, 164.0),
            (StatsPlace::Strip, 766.0),
            (StatsPlace::Strip, 520.0),
        ] {
            let mut app = app_with(&sine(40));
            let ctx = crate::app::styled_context();
            ctx.enable_accesskit();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1200.0));
            let frame = |app: &mut App, time: f64, events: Vec<egui::Event>| {
                headless_frame(&ctx, Some(screen), Some(time), events, |ui| {
                    let rect =
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 1200.0));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        app.show_stats_section(ui, place, 1.0);
                    });
                })
            };
            let out = frame(&mut app, 0.0, vec![]);
            let (nodes, _) = crate::app::test_support::a11y(&out);
            let reset = crate::app::test_support::node_rect(&nodes, |n| n.label() == Some("Reset"))
                .unwrap_or_else(|| panic!("{place:?} at {width}: no Reset"));
            assert!(
                reset.right() <= width,
                "{place:?} at {width}: Reset at {reset:?}"
            );
            let mut tip = false;
            for t in 1..6 {
                let events = if t == 1 {
                    vec![egui::Event::PointerMoved(reset.center())]
                } else {
                    vec![]
                };
                let out = frame(&mut app, f64::from(t), events);
                let mut texts = Vec::new();
                fn walk(s: &egui::Shape, out: &mut Vec<String>) {
                    match s {
                        egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                        egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                        _ => {}
                    }
                }
                out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
                tip |= texts.iter().any(|t| t.starts_with("Reset Min / Max"));
            }
            assert!(tip, "{place:?} at {width}: no hover text on Reset");
            assert!(app.capture.session.stats().count > 0);
            for (t, events) in
                crate::app::test_support::click_frames(reset.center(), egui::PointerButton::Primary)
                    .into_iter()
                    .enumerate()
            {
                frame(&mut app, 10.0 + 0.05 * t as f64, events);
            }
            assert_eq!(
                app.capture.session.stats().count,
                0,
                "{place:?} at {width}: not reset"
            );
        }
    }

    /// One skipped gap is one gap.
    #[test]
    fn the_gap_warning_counts_in_the_singular() {
        let l10n = crate::l10n::english();
        assert_eq!(
            tr!(l10n, "stats-gaps", count = 1u64, limit = "2"),
            "\u{26a0} 1 gap >2s skipped"
        );
        assert_eq!(
            tr!(l10n, "stats-gaps", count = 3u64, limit = "2"),
            "\u{26a0} 3 gaps >2s skipped"
        );
    }
}
