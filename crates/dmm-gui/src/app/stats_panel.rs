//! The statistics panel: min/max/avg/count and the running integral, for
//! both the session and the visible graph window, pre-formatted once and
//! rendered in either the wide stacked rows or the compact single-line form.

use dmm_lib::measurement::MeasuredValue;
use dmm_lib::stats::{self, RunningStats};
use eframe::egui::{self, Color32, RichText, Ui};

use super::App;

/// Pre-formatted min/max/avg/count strings for a single stats group.
struct FormattedStatsGroup {
    min: String,
    max: String,
    avg: String,
    count: u64,
    /// Formatted integral string (e.g. "   0.2925 mAh"), or None if not integrable.
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
    /// `visible_decimals` is what the graph writes the plotted series with,
    /// as ([`Graph::decimals`] of a point, of a mean): an NCV level's Min and
    /// Max are whole numbers.
    ///
    /// [`Graph::decimals`]: crate::graph::Graph::decimals
    fn new(
        stats: &RunningStats,
        visible: Option<&RunningStats>,
        unit: &str,
        visible_unit: &str,
        visible_decimals: (usize, usize),
        integral: Option<(f64, &str, Option<f64>)>,
        visible_integral: Option<(f64, &str, Option<f64>)>,
    ) -> Self {
        let fmt_in = |unit: &str, v: Option<f64>, decimals: usize| -> String {
            match v {
                Some(val) => format!("{val:>10.decimals$} {unit}"),
                None => format!("{:>10} {unit}", crate::NO_DATA),
            }
        };
        let fmt_integral = |info: Option<(f64, &str, Option<f64>)>| -> Option<String> {
            info.map(|(val, disp_unit, dt)| match dt {
                Some(secs) => format!("{val:>10.4} {disp_unit} ({secs:.0}s)"),
                None => format!("{val:>10.4} {disp_unit}"),
            })
        };
        // One accumulator type on both sides, so the two groups cannot drift
        // apart in how an absent figure is rendered.
        let group = |unit: &str,
                     s: &RunningStats,
                     (point, mean): (usize, usize),
                     integral: Option<(f64, &str, Option<f64>)>| {
            FormattedStatsGroup {
                min: fmt_in(unit, s.min, point),
                max: fmt_in(unit, s.max, point),
                avg: fmt_in(unit, s.avg(), mean),
                count: s.count,
                integral: fmt_integral(integral),
            }
        };
        Self {
            session: group(unit, stats, (4, 4), integral),
            visible: visible.map(|v| group(visible_unit, v, visible_decimals, visible_integral)),
        }
    }
}

/// Draw one stats group as the wide layout's stacked rows.
///
/// `color` tints every row — the visible block is drawn in the weak text
/// colour, the session block in the default one. The integral's gap warning
/// is not drawn here: it belongs to the session block only, and the caller
/// emits it right after this returns.
fn show_stat_rows(
    ui: &mut Ui,
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
    ui.label(styled(format!("Min:{}", group.min), mono.clone()));
    ui.label(styled(format!("Max:{}", group.max), mono.clone()));
    ui.label(styled(format!("Avg:{}", group.avg), mono.clone()));
    ui.label(styled(
        format!("Count: {}", group.count),
        egui::FontId::proportional(font_size),
    ));
    if let Some(int) = &group.integral {
        ui.label(styled(format!("\u{222b}:{int}"), mono));
    }
}

impl App {
    pub(super) fn show_stats_section(&mut self, ui: &mut Ui, compact: bool, scale: f32) {
        let unit = match self.last_measurement.as_ref() {
            // A word the meter shows instead of a reading has no unit of its
            // own and leaves the series alone, so the figures still belong
            // to the reading before it.
            Some(m) if matches!(m.value, MeasuredValue::NoReading(_)) => {
                self.capture.session.unit().unwrap_or("")
            }
            Some(m) => &*m.unit,
            None => "",
        };
        // Keyed on the caption unit rather than the session's: `clear_session`
        // drops `last_measurement` but `SeriesStats::reset` keeps its series,
        // and the row should vanish with the reading it describes.
        let integral_info = stats::integral_display(self.capture.session.integrator.value(), unit)
            .map(|(value, unit)| (value, unit, self.capture.session.integrator.elapsed_secs()));
        // The visible block reports what the graph draws, which is not the
        // main reading's unit when a sub-value is plotted.
        let visible_unit = self.graph.plotted_unit();
        let visible_integral = self
            .graph
            .visible_integral()
            .and_then(|raw| stats::integral_display(raw, visible_unit))
            .map(|(value, unit)| (value, unit, self.graph.visible_data_span_secs()));
        let visible_stats = self.graph.visible_stats();
        let formatted = FormattedStats::new(
            &self.capture.session.stats,
            visible_stats.as_ref(),
            unit,
            visible_unit,
            (self.graph.decimals(true), self.graph.decimals(false)),
            integral_info,
            visible_integral,
        );
        let main_font = 12.0 * scale;
        let sub_font = 11.0 * scale;

        if compact {
            ui.horizontal_wrapped(|ui| {
                let session = &formatted.session;
                ui.label(
                    RichText::new(format!(
                        "Min:{}  Max:{}  Avg:{}  ({})",
                        session.min, session.max, session.avg, session.count,
                    ))
                    .font(egui::FontId::monospace(main_font)),
                );
                self.reset_button(ui, sub_font);
            });

            if let Some(vis) = &formatted.visible {
                let vis_line = if let Some(vint) = &vis.integral {
                    format!(
                        "Visible: Min:{} Max:{} Avg:{} ({})  \u{222b}:{vint}",
                        vis.min, vis.max, vis.avg, vis.count,
                    )
                } else {
                    format!(
                        "Visible: Min:{} Max:{} Avg:{} ({})",
                        vis.min, vis.max, vis.avg, vis.count,
                    )
                };
                ui.label(
                    RichText::new(vis_line)
                        .font(egui::FontId::monospace(sub_font))
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if let Some(int) = &formatted.session.integral {
                ui.label(
                    RichText::new(format!("\u{222b}:{int}"))
                        .font(egui::FontId::monospace(main_font)),
                );
                self.show_integral_gap_warning(ui, sub_font);
            }
        } else {
            let heading = self.settings.theme_colors(ui.visuals().dark_mode).heading();
            ui.label(
                RichText::new("Statistics")
                    .font(egui::FontId::proportional(sub_font))
                    .color(heading),
            );
            show_stat_rows(ui, &formatted.session, main_font, None);
            if formatted.session.integral.is_some() {
                self.show_integral_gap_warning(ui, sub_font);
            }
            self.reset_button(ui, sub_font);

            // Windowed stats for visible graph interval
            if let Some(vis) = &formatted.visible {
                ui.add_space(4.0);
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    RichText::new("Visible")
                        .strong()
                        .font(egui::FontId::proportional(sub_font))
                        .color(heading),
                );
                show_stat_rows(ui, vis, sub_font, Some(weak));
            }
        }
    }

    /// The session-stats reset control, identical in both layout modes.
    fn reset_button(&mut self, ui: &mut Ui, font_size: f32) {
        if ui
            .add(egui::Button::new(
                RichText::new("Reset").font(egui::FontId::proportional(font_size)),
            ))
            .on_hover_text("Reset Min / Max / Avg / integral counters")
            .clicked()
        {
            self.capture.session.reset();
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
        let limit = self.capture.session.integrator.max_dt_secs();
        ui.label(
            RichText::new(format!(
                "\u{26A0} {} gaps >{limit}s skipped",
                self.capture.session.integrator.skipped_intervals
            ))
            .font(egui::FontId::proportional(font_size))
            .color(tc.status_warning()),
        )
        .on_hover_text(format!(
            "Stretches of more than {limit} s without a reading are not integrated: \
             5 sample intervals, and at least 2 s. The integral is partial."
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NCV levels in view: Min and Max as the whole numbers they are, the
    /// average to one decimal, and no integral for a level.
    #[test]
    fn visible_levels_read_as_whole_numbers() {
        let mut levels = RunningStats::new();
        for l in [0.0, 2.0, 3.0] {
            levels.push(l);
        }
        let formatted = FormattedStats::new(
            &RunningStats::new(),
            Some(&levels),
            "",
            "",
            (0, 1),
            None,
            None,
        );
        let vis = formatted.visible.expect("levels in view");
        let figures = [&vis.min, &vis.max, &vis.avg].map(|s| s.trim().to_string());
        assert_eq!(figures, ["0", "3", "1.7"]);
        assert_eq!(
            vis.min.len(),
            formatted.session.min.len(),
            "columns line up"
        );
        // A level has no unit, so the panel finds no integral to show.
        assert_eq!(stats::integral_display(6.0, ""), None);
    }
}
