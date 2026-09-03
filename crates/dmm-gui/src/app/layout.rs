//! The reading column shared by the wide and narrow layouts — reading,
//! remote controls, specs and stats — plus the specs section variants and the
//! big meter toggle that cycles the reading to full screen and back.

use dmm_lib::protocol::ut61eplus::tables::{ModeSpecInfo, SpecInfo};
use eframe::egui::{self, RichText, Ui};

use super::{App, BigMeterMode};
use crate::a11y::ResponseA11yExt;
use crate::display;
use crate::specs;

/// Which of the two multi-panel layouts the reading column is rendered in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ContentLayout {
    /// Reading column is a left side panel; the graph lives beside it.
    Wide,
    /// Reading column is the whole window and stacks the graph below it.
    Narrow,
}

impl App {
    /// Reading, controls, specs and stats — the column shared by the wide and
    /// narrow multi-panel layouts. The two differ only in the reading widget,
    /// the specs section, the stats section's compact flag, and whether the
    /// graph/recording split is stacked below (narrow) or lives in its own
    /// centre panel (wide).
    pub(super) fn show_reading_column(&mut self, ui: &mut Ui, layout: ContentLayout) {
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        match layout {
            ContentLayout::Wide => display::show_reading(
                ui,
                self.last_measurement.as_ref(),
                &tc,
                !self.transform.is_identity(),
            ),
            ContentLayout::Narrow => display::show_reading_compact(
                ui,
                self.last_measurement.as_ref(),
                &tc,
                !self.transform.is_identity(),
            ),
        }
        let controls_top = ui.cursor().top();
        self.show_remote_controls(ui, 1.0);
        let controls_bottom = ui.cursor().top();
        // Overlay toggle on the last controls row, right-aligned.
        let toggle_rect = egui::Rect::from_min_max(
            egui::pos2(ui.max_rect().left(), controls_top),
            egui::pos2(ui.max_rect().right(), controls_bottom),
        );
        self.show_big_meter_toggle_at(ui, toggle_rect);
        self.show_transform_row(ui, 1.0);
        self.show_connection_help(ui);

        match layout {
            ContentLayout::Wide => {
                ui.add_space(8.0);

                let has_spec = self
                    .last_measurement
                    .as_ref()
                    .map(|m| m.spec.is_some())
                    .unwrap_or(false);
                if has_spec || self.manual_url().is_some() {
                    ui.separator();
                    self.show_specs_section(ui, 1.0);
                }
            }
            ContentLayout::Narrow => self.show_specs_section_compact(ui),
        }

        if self.settings.show_stats {
            ui.separator();
            let compact = matches!(layout, ContentLayout::Narrow);
            self.show_stats_section(ui, compact, 1.0);
        }

        // Wide keeps the graph in its own centre panel next to this column.
        if let ContentLayout::Narrow = layout
            && (self.settings.show_graph || self.settings.show_recording)
        {
            ui.separator();
            self.show_graph_recording_split(ui, true);
        }
    }

    /// Paint the big meter toggle button at a given rect (overlay, no layout impact).
    pub(super) fn show_big_meter_toggle_at(&mut self, ui: &mut Ui, rect: egui::Rect) {
        let (icon, tooltip) = match self.big_meter_mode {
            BigMeterMode::Off => (
                "\u{229E}",
                "Hide side panels and show the meter reading full-screen (Ctrl+B)",
            ),
            BigMeterMode::Full | BigMeterMode::Minimal => (
                "\u{229F}",
                "Return to the normal multi-panel layout (Ctrl+B)",
            ),
        };
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
            let color = ui.visuals().weak_text_color();
            let btn = egui::Button::new(RichText::new(icon).size(14.0).color(color));
            let response = ui.add(btn).on_hover_text(tooltip).a11y_label(tooltip);
            if response.clicked() {
                if self.big_meter_mode == BigMeterMode::Off {
                    // Enter big meter — use cycle_big_meter() to handle
                    // the "already_big" restore-all-panels case.
                    self.cycle_big_meter();
                } else {
                    self.big_meter_mode = BigMeterMode::Off;
                }
            }
        });
    }

    pub(super) fn cycle_big_meter(&mut self) {
        match self.big_meter_mode {
            BigMeterMode::Off => {
                let already_big = !self.settings.show_graph
                    && !self.settings.show_recording
                    && !self.settings.show_stats
                    && !self.settings.show_specs;
                if already_big {
                    // All panels already hidden via settings — restore them all.
                    self.settings.show_graph = true;
                    self.settings.show_recording = true;
                    self.settings.show_stats = true;
                    self.settings.show_specs = true;
                    self.settings.save();
                } else {
                    self.big_meter_mode = BigMeterMode::Full;
                }
            }
            BigMeterMode::Full => {
                self.big_meter_mode = BigMeterMode::Minimal;
            }
            BigMeterMode::Minimal => {
                self.big_meter_mode = BigMeterMode::Off;
            }
        }
    }

    /// Render a specs section, calling `render_fn` when spec data is available,
    /// or showing a manual-only link as fallback.
    fn show_specs_with(
        &self,
        ui: &mut Ui,
        scale: f32,
        render_fn: fn(
            &mut Ui,
            &'static SpecInfo,
            Option<&'static ModeSpecInfo>,
            Option<&'static str>,
            f32,
        ),
    ) {
        if !self.settings.show_specs {
            return;
        }
        let manual_url = self.manual_url();
        let spec = self.last_measurement.as_ref().and_then(|m| m.spec);
        let mode_spec = self.last_measurement.as_ref().and_then(|m| m.mode_spec);
        if let Some(spec) = spec {
            render_fn(ui, spec, mode_spec, manual_url, scale);
        } else if let Some(url) = manual_url {
            specs::show_manual_only(ui, url, scale);
        }
    }

    /// Render specs for the wide (side panel) layout.
    fn show_specs_section(&self, ui: &mut Ui, scale: f32) {
        self.show_specs_with(ui, scale, specs::show_specs);
    }

    /// Render specs for big meter mode (pipe-separated inline).
    pub(super) fn show_specs_section_inline(&self, ui: &mut Ui, scale: f32) {
        self.show_specs_with(ui, scale, specs::show_specs_inline);
    }

    /// Render specs for the narrow (compact single-line) layout.
    fn show_specs_section_compact(&self, ui: &mut Ui) {
        self.show_specs_with(ui, 1.0, specs::show_specs_compact_scaled);
    }
}
