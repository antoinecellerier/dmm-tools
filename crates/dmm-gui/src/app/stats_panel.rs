//! The statistics panel: min/max/avg/count and the running integral, for
//! both the session and the visible graph window, pre-formatted once and
//! rendered in either the wide stacked rows or the compact single-line form.

use dmm_lib::stats::{self, RunningStats, ShownDigits};
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, Color32, RichText, Ui};

use super::App;

/// Pre-formatted min/max/avg/count strings for a single stats group.
struct FormattedStatsGroup {
    min: String,
    max: String,
    avg: String,
    /// Sample standard deviation, written like the average.
    sd: String,
    count: u64,
    /// The integral's line, caption and all (e.g. "∫:   0.29250 mAh (12s)"),
    /// or None if not integrable.
    integral: Option<String>,
}

/// Pre-formatted statistics for the stats section, shared by both layout modes.
struct FormattedStats {
    /// Since-reset stats over the meter's main reading.
    session: FormattedStatsGroup,
    /// Visible-window stats, if available.
    visible: Option<FormattedStatsGroup>,
}

impl FormattedStats {
    /// `unit` captions the session block, `visible_unit` the visible-window
    /// block. They differ when the graph plots a sub-value: session stats
    /// follow the meter's main reading, the visible window follows what is
    /// drawn, and captioning Hz values with "V" would be simply wrong.
    ///
    /// The integrals arrive already scaled to their display unit (see
    /// [`stats::integral_display`]) as `(value, display_unit, elapsed_secs)`.
    ///
    /// Every figure and both integrals are written by
    /// [`dmm_shared::stat_figure`], as `dmm-cli read`'s summary writes them:
    /// the figures in a range and to the resolution the meter showed the
    /// session in (`shown`), the integrals to five significant digits. The
    /// visible block takes the session's ranges too, so a window lying
    /// wholly before a range the meter has since left may be written in a
    /// range it did not show itself; accepted, as the two blocks then read
    /// alike. A level's figures are not: `visible_levels` is what the graph
    /// writes a plotted level with ([`Graph::decimals`] of a point, of a
    /// mean), as an NCV level's Min and Max are whole numbers, and `None`
    /// for a measured series.
    ///
    /// [`Graph::decimals`]: crate::graph::Graph::decimals
    #[expect(
        clippy::too_many_arguments,
        reason = "the whole and visible statistics side by side"
    )]
    fn new(
        l10n: &Localizer,
        stats: &RunningStats,
        visible: Option<&RunningStats>,
        shown: &ShownDigits,
        unit: &str,
        visible_unit: &str,
        visible_main: bool,
        visible_levels: bool,
        integral: Option<(f64, &str, Option<f64>)>,
        visible_integral: Option<(f64, &str, Option<f64>)>,
    ) -> Self {
        // Right-aligned to one width, so the figures line up and hold still
        // as they update, whatever unit each one ends on.
        let fmt_in = |unit: &str,
                      v: Option<f64>,
                      decimals: Option<usize>,
                      shown: Option<&ShownDigits>|
         -> String {
            match (v, decimals) {
                (Some(val), Some(decimals)) => format!("{val:>10.decimals$} {unit}"),
                (Some(val), None) => {
                    let (number, unit) = dmm_shared::stat_figure(val, unit, shown);
                    format!("{number:>10} {unit}")
                }
                (None, _) => format!("{:>10} {unit}", crate::NO_DATA),
            }
        };
        let fmt_integral = |info: Option<(f64, &str, Option<f64>)>| -> Option<String> {
            info.map(|(val, disp_unit, dt)| {
                let (number, disp_unit) = dmm_shared::stat_figure(val, disp_unit, None);
                let value = format!("{number:>10}");
                match dt {
                    Some(secs) => tr!(
                        l10n,
                        "stats-integral",
                        value = value,
                        unit = disp_unit.as_ref(),
                        seconds = format!("{secs:.0}")
                    ),
                    None => tr!(
                        l10n,
                        "stats-integral-untimed",
                        value = value,
                        unit = disp_unit.as_ref()
                    ),
                }
                .into_owned()
            })
        };
        // One accumulator type on both sides, so the two groups cannot drift
        // apart in how an absent figure is rendered.
        // A level's points are whole numbers; its mean and σ get one decimal.
        let group = |unit: &str,
                     s: &RunningStats,
                     levels: bool,
                     shown: Option<&ShownDigits>,
                     integral: Option<(f64, &str, Option<f64>)>| {
            let (point, mean) = if levels {
                (Some(0), Some(1))
            } else {
                (None, None)
            };
            FormattedStatsGroup {
                min: fmt_in(unit, s.min, point, shown),
                max: fmt_in(unit, s.max, point, shown),
                avg: fmt_in(unit, s.avg(), mean, shown),
                sd: fmt_in(unit, s.std_dev(), mean, shown),
                count: s.count,
                integral: fmt_integral(integral),
            }
        };
        // What the meter showed is the main reading's: a plotted sub-value
        // (a Raw value, a MIN/MAX in another prefix) may share its base unit
        // and not its ranges.
        let visible_shown = visible_main.then_some(shown);
        Self {
            session: group(unit, stats, false, Some(shown), integral),
            visible: visible.map(|v| {
                group(
                    visible_unit,
                    v,
                    visible_levels,
                    visible_shown,
                    visible_integral,
                )
            }),
        }
    }
}

/// The Min, Max, Mean and σ rows of the wide layout: each caption, the
/// space to put before it so that it ends where the widest one does, and
/// its figure. `width` measures a caption as it is drawn: by its laid-out
/// width, not its characters, which in another language need not be one
/// monospace cell each.
fn captioned_rows<'a>(
    l10n: &'a Localizer,
    group: &'a FormattedStatsGroup,
    width: impl Fn(&str) -> f32,
) -> [(std::borrow::Cow<'a, str>, f32, &'a str); 4] {
    let captions = [
        tr!(l10n, "stats-min"),
        tr!(l10n, "stats-max"),
        tr!(l10n, "stats-mean"),
        tr!(l10n, "stats-sd"),
    ];
    let widths = captions.each_ref().map(|c| width(c));
    let widest = widths.iter().copied().fold(0.0, f32::max);
    let values = [&group.min, &group.max, &group.avg, &group.sd];
    let mut captions = captions.into_iter();
    std::array::from_fn(|i| {
        let caption = captions.next().unwrap_or_default();
        (caption, widest - widths[i], values[i].as_str())
    })
}

/// Draw one stats group as the wide layout's stacked rows.
///
/// `color` tints every row — the visible block is drawn in the weak text
/// colour, the session block in the default one. The integral's gap warning
/// is not drawn here: it belongs to the session block only, and the caller
/// emits it right after this returns.
fn show_stat_rows(
    ui: &mut Ui,
    l10n: &Localizer,
    group: &FormattedStatsGroup,
    font_size: f32,
    color: Option<Color32>,
) {
    let styled = |text: String, font: egui::FontId| {
        let rich = RichText::new(text).font(font);
        match color {
            Some(c) => rich.color(c),
            None => rich,
        }
    };
    let mono = egui::FontId::monospace(font_size);
    // Unrounded: a width rounded up to the pixel would shift a short
    // caption's figures right of the others'.
    let width = |text: &str| {
        let mut job = egui::text::LayoutJob::simple_singleline(
            text.to_string(),
            mono.clone(),
            Color32::WHITE,
        );
        job.round_output_to_gui = false;
        ui.fonts_mut(|f| f.layout_job(job).size().x)
    };
    let style = ui.style().clone();
    let valign = ui.text_valign();
    for (caption, pad, value) in captioned_rows(l10n, group, width) {
        // One galley per row, as a single label was: the caption moved
        // right by `pad`, the figure after it.
        let mut row = egui::text::LayoutJob::default();
        for (text, lead) in [(caption.into_owned(), pad), (value.to_string(), 0.0)] {
            let font = egui::FontSelection::Default;
            styled(text, mono.clone()).append_to(&mut row, &style, font, valign);
            if let Some(last) = row.sections.last_mut()
                && lead > 0.0
            {
                last.leading_space = lead;
            }
        }
        ui.label(row);
    }
    ui.label(styled(
        tr!(l10n, "stats-count", count = group.count).into_owned(),
        egui::FontId::proportional(font_size),
    ));
    if let Some(int) = &group.integral {
        ui.label(styled(int.clone(), mono));
    }
}

/// The compact layout's figures, one unwrapping label each, so a narrow
/// column wraps between two figures rather than between a caption and its
/// value.
fn compact_figures(
    ui: &mut Ui,
    figures: impl IntoIterator<Item = String>,
    font_size: f32,
    color: Option<Color32>,
) {
    for text in figures {
        let rich = RichText::new(text).font(egui::FontId::monospace(font_size));
        let rich = match color {
            Some(c) => rich.color(c),
            None => rich,
        };
        ui.add(egui::Label::new(rich).extend());
    }
}

impl App {
    pub(super) fn show_stats_section(&mut self, ui: &mut Ui, compact: bool, scale: f32) {
        // The unit the figures are held in, not the latest reading's: a word
        // the meter shows instead of a reading, or an overload in another
        // decade ("OL MΩ" between kΩ readings), leaves the series alone, so
        // the figures still belong to the readings before it.
        // Blank without a reading: `clear_session` drops `last_measurement`
        // but `SeriesStats::reset` keeps its series, and the integral's row
        // should vanish with the reading it describes.
        let unit = match self.last_measurement {
            Some(_) => self.capture.session.unit().unwrap_or(""),
            None => "",
        };
        let integral_info = stats::integral_display(self.capture.session.integrator.value(), unit)
            .map(|(value, unit)| (value, unit, self.capture.session.integrator.elapsed_secs()));
        // The visible block reports what the graph draws, which is not the
        // main reading's unit when a sub-value is plotted.
        // Before the unit is borrowed: the graph keeps them between frames.
        let (visible_stats, visible_raw) = self.graph.visible_figures();
        let visible_stats = (visible_stats.count > 0).then_some(visible_stats);
        let visible_unit = self.graph.plotted_unit();
        let visible_integral = visible_raw
            .and_then(|raw| stats::integral_display(raw, visible_unit))
            .map(|(value, unit)| (value, unit, self.graph.visible_data_span_secs()));
        // A handle, as the Reset button below borrows the app.
        let l10n = self.l10n.clone();
        let formatted = FormattedStats::new(
            &l10n,
            &self.capture.session.stats,
            visible_stats.as_ref(),
            self.capture.session.shown(),
            unit,
            visible_unit,
            self.graph.plots_main(),
            self.graph.plots_levels(),
            integral_info,
            visible_integral,
        );
        let main_font = 12.0 * scale;
        let sub_font = 11.0 * scale;

        if compact {
            let (min, max, mean, sd) = (
                tr!(l10n, "stats-min"),
                tr!(l10n, "stats-max"),
                tr!(l10n, "stats-mean"),
                tr!(l10n, "stats-sd"),
            );
            ui.horizontal_wrapped(|ui| {
                let session = &formatted.session;
                compact_figures(
                    ui,
                    [
                        format!("{min}{}", session.min),
                        format!("{max}{}", session.max),
                        format!("{mean}{}", session.avg),
                        format!("{sd}{}", session.sd),
                        format!("({})", session.count),
                    ],
                    main_font,
                    None,
                );
                self.reset_button(ui, sub_font);
            });

            if let Some(vis) = &formatted.visible {
                let weak = ui.visuals().weak_text_color();
                ui.horizontal_wrapped(|ui| {
                    compact_figures(
                        ui,
                        [
                            format!("{} {min}{}", tr!(l10n, "stats-visible-caption"), vis.min),
                            format!("{max}{}", vis.max),
                            format!("{mean}{}", vis.avg),
                            format!("{sd}{}", vis.sd),
                            format!("({})", vis.count),
                        ]
                        .into_iter()
                        .chain(vis.integral.clone()),
                        sub_font,
                        Some(weak),
                    );
                });
            }
            if let Some(int) = &formatted.session.integral {
                ui.label(RichText::new(int).font(egui::FontId::monospace(main_font)));
                self.show_integral_gap_warning(ui, sub_font);
            }
            self.show_alarm_counts(ui, main_font);
        } else {
            let heading = self.settings.theme_colors(ui.visuals().dark_mode).heading();
            ui.label(
                RichText::new(tr!(l10n, "stats-heading"))
                    .font(egui::FontId::proportional(sub_font))
                    .color(heading),
            );
            show_stat_rows(ui, &l10n, &formatted.session, main_font, None);
            if formatted.session.integral.is_some() {
                self.show_integral_gap_warning(ui, sub_font);
            }
            self.show_alarm_counts(ui, main_font);
            self.reset_button(ui, sub_font);

            // Windowed stats for visible graph interval
            if let Some(vis) = &formatted.visible {
                ui.add_space(4.0);
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    RichText::new(tr!(l10n, "stats-visible"))
                        .strong()
                        .font(egui::FontId::proportional(sub_font))
                        .color(heading),
                );
                show_stat_rows(ui, &l10n, vis, sub_font, Some(weak));
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
        ui.label(RichText::new(text).font(egui::FontId::proportional(font_size)))
            .on_hover_text(tr!(self.l10n, "stats-alarm-counts.hover").as_ref());
    }

    /// The session-stats reset control, identical in both layout modes.
    fn reset_button(&mut self, ui: &mut Ui, font_size: f32) {
        if ui
            .add(egui::Button::new(
                RichText::new(tr!(self.l10n, "stats-reset"))
                    .font(egui::FontId::proportional(font_size)),
            ))
            .on_hover_text(tr!(self.l10n, "stats-reset.hover").as_ref())
            .clicked()
        {
            self.capture.session.reset();
            if let Some(alarm) = &mut self.alarm {
                alarm.clear_counts();
            }
        }
    }

    /// Warn that intervals too long to integrate were skipped.
    ///
    /// Shared by the compact and wide stats layouts: the label and its hover
    /// text were duplicated character-for-character between them, which
    /// CLAUDE.md warns about precisely because the two copies drift.
    fn show_integral_gap_warning(&self, ui: &mut Ui, font_size: f32) {
        if self.capture.session.integrator.skipped_intervals == 0 {
            return;
        }
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let limit = self.capture.session.integrator.max_dt_secs().to_string();
        let count = self.capture.session.integrator.skipped_intervals;
        ui.label(
            RichText::new(tr!(self.l10n, "stats-gaps", count = count, limit = &limit))
                .font(egui::FontId::proportional(font_size))
                .color(tc.status_warning()),
        )
        .on_hover_text(tr!(self.l10n, "stats-gaps.hover", limit = &limit).as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::headless_frame;

    /// In English the rows draw every figure where the space-padded text
    /// they were drew it, in the session's font and the visible block's.
    #[test]
    fn the_wide_rows_draw_where_padded_text_did() {
        fn glyphs(shape: &egui::Shape, out: &mut Vec<(char, egui::Pos2)>) {
            match shape {
                egui::Shape::Text(t) => {
                    for row in &t.galley.rows {
                        for g in &row.glyphs {
                            if g.chr != ' ' {
                                out.push((g.chr, t.pos + g.pos.to_vec2() + row.pos.to_vec2()));
                            }
                        }
                    }
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| glyphs(s, out)),
                _ => {}
            }
        }
        let drawn = |draw: &dyn Fn(&mut Ui)| {
            let ctx = crate::app::styled_context();
            let out = headless_frame(&ctx, None, None, vec![], |ui| draw(ui));
            let mut found = Vec::new();
            for clipped in &out.shapes {
                glyphs(&clipped.shape, &mut found);
            }
            found
        };
        let l10n = crate::l10n::english();
        let group = FormattedStatsGroup {
            min: "    1.0000 V".into(),
            max: "    2.0000 V".into(),
            avg: "    1.5000 V".into(),
            sd: "    0.7071 V".into(),
            count: 2,
            integral: None,
        };
        let padded = [
            " Min:    1.0000 V",
            " Max:    2.0000 V",
            "Mean:    1.5000 V",
            "   \u{3c3}:    0.7071 V",
        ];
        for (size, color) in [(14.0, None), (11.0, Some(Color32::GRAY))] {
            let rows = drawn(&|ui| show_stat_rows(ui, &l10n, &group, size, color));
            let before = drawn(&|ui| {
                for text in padded {
                    let rich = RichText::new(text).font(egui::FontId::monospace(size));
                    ui.label(match color {
                        Some(c) => rich.color(c),
                        None => rich,
                    });
                }
                ui.label(RichText::new("Readings: 2").font(egui::FontId::proportional(size)));
            });
            assert!(!before.is_empty());
            assert_eq!(rows, before, "{size} pt");
        }
    }

    /// NCV levels in view: Min and Max as the whole numbers they are, the
    /// average to one decimal, and no integral for a level.
    #[test]
    fn visible_levels_read_as_whole_numbers() {
        let mut levels = RunningStats::new();
        for l in [0.0, 2.0, 3.0] {
            levels.push(l);
        }
        let formatted = FormattedStats::new(
            &crate::l10n::english(),
            &RunningStats::new(),
            Some(&levels),
            &ShownDigits::default(),
            "",
            "",
            true,
            true,
            None,
            None,
        );
        let vis = formatted.visible.expect("levels in view");
        let figures = [&vis.min, &vis.max, &vis.avg, &vis.sd].map(|s| s.trim().to_string());
        assert_eq!(figures, ["0", "3", "1.7", "1.5"]);
        assert_eq!(
            vis.min.len(),
            formatted.session.min.len(),
            "columns line up"
        );
        // A level has no unit, so the panel finds no integral to show.
        assert_eq!(stats::integral_display(6.0, ""), None);
    }

    /// A session carried from MΩ down to Ω writes each figure in a range
    /// the meter showed, to that range's decimals, the visible block too,
    /// and the integral to five significant digits, as the CLI summary
    /// writes them; the figures still end in one column.
    #[test]
    fn session_figures_keep_the_meters_ranges() {
        use dmm_lib::flags::StatusFlags;
        use dmm_lib::measurement::{MeasuredValue, Measurement};
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
        let formatted = FormattedStats::new(
            &crate::l10n::english(),
            &session.stats,
            Some(&session.stats),
            session.shown(),
            "\u{3a9}",
            "\u{3a9}",
            true,
            false,
            Some((0.000_012_34, "mAh", None)),
            None,
        );
        for s in [
            &formatted.session,
            formatted.visible.as_ref().expect("visible"),
        ] {
            assert_eq!(s.min, "    219.82 \u{3a9}");
            assert_eq!(s.max, "    1.2341 M\u{3a9}");
            assert_eq!(s.avg, "  411.5133 k\u{3a9}");
            assert_eq!(s.sd, "  712.3810 k\u{3a9}");
        }
        let integral = formatted.session.integral.as_deref().expect("integral");
        assert!(integral.contains("0.000012340 mAh"), "{integral}");

        // A plotted sub-value in the same base unit isn't what the meter
        // showed in those ranges: five significant digits in its own prefix.
        let sub_value = FormattedStats::new(
            &crate::l10n::english(),
            &session.stats,
            Some(&session.stats),
            session.shown(),
            "\u{3a9}",
            "\u{3a9}",
            false,
            false,
            None,
            None,
        );
        let visible = sub_value.visible.expect("visible");
        assert_eq!(visible.avg, "    411.51 k\u{3a9}");
        assert_eq!(sub_value.session.avg, formatted.session.avg);
    }

    /// The captions are right-aligned to the widest, so the figures start
    /// in one column, and the integral's line carries its own caption.
    #[test]
    fn the_wide_rows_line_up() {
        let l10n = crate::l10n::english();
        let mut readings = RunningStats::new();
        for v in [1.0, 2.0] {
            readings.push(v);
        }
        let formatted = FormattedStats::new(
            &l10n,
            &readings,
            None,
            &ShownDigits::default(),
            "V",
            "V",
            true,
            false,
            Some((3.0, "V\u{b7}s", Some(2.0))),
            None,
        );
        // One cell per character, as the monospace column has them in
        // English.
        let rows = captioned_rows(&l10n, &formatted.session, |c| c.chars().count() as f32);
        let rows = rows.map(|(caption, pad, value)| (caption.into_owned(), pad, value));
        assert_eq!(
            rows,
            [
                ("Min:".to_string(), 1.0, "    1.0000 V"),
                ("Max:".to_string(), 1.0, "    2.0000 V"),
                ("Mean:".to_string(), 0.0, "    1.5000 V"),
                ("\u{3c3}:".to_string(), 3.0, "    707.11 mV"),
            ]
        );
        assert_eq!(
            formatted.session.integral.as_deref(),
            Some("\u{222b}:    3.0000 V\u{b7}s (2s)")
        );
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
