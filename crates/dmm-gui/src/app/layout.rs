//! The reading column shared by the wide and narrow layouts — reading,
//! remote controls, specs and stats — plus the specs section variants, the
//! big meter toggle that cycles the reading to full screen and back, and the
//! window and panel sizes kept for the next launch.

use dmm_lib::specs::{ModeSpecInfo, SpecInfo};
use eframe::egui::{self, RichText, Ui};

use super::recording_panel::MIN_SPLIT_HEIGHT;
use super::{App, BigMeterMode};
use crate::a11y::ResponseA11yExt;
use crate::display;
use crate::settings::DEFAULT_WINDOW_SIZE;
use crate::specs;
use std::time::Instant;

/// Which of the two multi-panel layouts the reading column is rendered in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ContentLayout {
    /// Reading column is a left side panel; the graph lives beside it.
    Wide,
    /// Reading column is the whole window and stacks the graph below it.
    Narrow,
}

impl App {
    /// The reading column inside the page scroller both multi-panel layouts
    /// wrap it in: in a window too short for the column, the stats — and in
    /// the narrow layout the graph and recording below them — used to be cut
    /// off at the bottom with no way to reach them.
    ///
    /// Returns the scroller's output so tests can see whether it had to scroll.
    pub(super) fn show_reading_column_scrolled(
        &mut self,
        ui: &mut Ui,
        layout: ContentLayout,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        egui::ScrollArea::vertical()
            .id_salt("reading_column")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                self.show_reading_column(ui, layout);
                crate::a11y::scroll_to_focus(ui);
            })
    }

    /// Reading, controls, specs and stats — the column shared by the wide and
    /// narrow multi-panel layouts. The two differ only in the reading widget,
    /// the specs section, the stats section's compact flag, and whether the
    /// graph/recording split is stacked below (narrow) or lives in its own
    /// centre panel (wide).
    pub(super) fn show_reading_column(&mut self, ui: &mut Ui, layout: ContentLayout) {
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let picked = match layout {
            ContentLayout::Wide => display::show_reading(
                ui,
                self.last_measurement.as_ref(),
                &tc,
                self.reading_state(),
                self.connection.readouts(),
            ),
            ContentLayout::Narrow => display::show_reading_compact(
                ui,
                self.last_measurement.as_ref(),
                &tc,
                self.reading_state(),
                self.connection.readouts(),
            ),
        };
        if let Some(pick) = picked {
            self.apply_pick(pick);
        }
        let controls_top = ui.cursor().top();
        self.show_remote_controls(ui, 1.0, Self::big_meter_toggle_width(ui));
        let controls_bottom = ui.cursor().top();
        // Overlay toggle on the last controls row, right-aligned.
        let toggle_rect = egui::Rect::from_min_max(
            egui::pos2(ui.max_rect().left(), controls_top),
            egui::pos2(ui.max_rect().right(), controls_bottom),
        );
        self.show_big_meter_toggle_at(ui, toggle_rect);
        self.show_transform_editor(ui, 1.0);
        self.show_alarm_editor(ui, 1.0);
        self.show_connection_help(ui);
        self.show_late_readings(ui);

        match layout {
            ContentLayout::Wide => {
                ui.add_space(8.0);

                let has_spec = self
                    .last_measurement
                    .as_ref()
                    .is_some_and(|m| m.spec.is_some() || m.mode_spec.is_some());
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
            // The split reads `ui.available_height()`, which inside the
            // column's scroller is the viewport rather than what is left of
            // the window, so give it an explicit height: the rest of the
            // viewport while the column fits — content then equals viewport
            // and no scrollbar appears — and `MIN_SPLIT_HEIGHT` once it
            // doesn't, so the graph stays usable and the column scrolls.
            let height = ui.available_height().max(MIN_SPLIT_HEIGHT).floor();
            ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                self.show_graph_recording_split(ui, true);
            });
        }
    }

    /// Paint the big meter toggle button at a given rect (overlay, no layout impact).
    /// Width the toggle drawn by [`show_big_meter_toggle_at`]
    /// (Self::show_big_meter_toggle_at) covers at the right edge of the
    /// controls row, so that row can keep its last chip clear of it.
    pub(crate) fn big_meter_toggle_width(ui: &Ui) -> f32 {
        // Both icons are the same glyph width; measure the one shown when
        // the toggle sits on the controls row.
        let galley = egui::WidgetText::from(Self::big_meter_toggle_icon(BigMeterMode::Off).0)
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
        galley.size().x + 2.0 * ui.spacing().button_padding.x
    }

    /// Icon and tooltip of the big-meter toggle in `mode`.
    fn big_meter_toggle_icon(mode: BigMeterMode) -> (RichText, &'static str) {
        let (icon, tooltip) = match mode {
            BigMeterMode::Off => (
                "\u{229E}",
                "Hide side panels and show the meter reading full-screen (Ctrl+B)",
            ),
            BigMeterMode::Full | BigMeterMode::Minimal => (
                "\u{229F}",
                "Return to the normal multi-panel layout (Ctrl+B)",
            ),
        };
        (RichText::new(icon).size(14.0), tooltip)
    }

    pub(super) fn show_big_meter_toggle_at(&mut self, ui: &mut Ui, rect: egui::Rect) {
        let (icon, tooltip) = Self::big_meter_toggle_icon(self.big_meter_mode);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
            let color = ui.visuals().weak_text_color();
            let btn = egui::Button::new(icon.color(color));
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

    /// Keep the window's size and maximized state and the reading panel's
    /// width in the settings, for the next launch to open with. The
    /// recording panel's height is kept where it is dragged.
    ///
    /// The size is taken from `content_rect`, not the viewport's
    /// `inner_rect`: that is always `None` on Wayland, and computed with the
    /// old zoom on the frame a zoom is applied. Times `zoom_factor`, it is in
    /// the OS logical points the window is built with, so a UI zoom doesn't
    /// resize the next launch's window.
    ///
    /// Not kept:
    /// - the size while maximized or fullscreen, so un-maximizing goes back to
    ///   the size the user picked;
    /// - anything while minimized, where some platforms report a maximized
    ///   window as not maximized;
    /// - the size in big meter mode: the mode isn't kept, and a small readout
    ///   window would cramp the next launch's full layout;
    /// - the maximized state on macOS after launch, which winit only reports
    ///   when the window is created.
    pub(super) fn track_layout(&mut self, ctx: &egui::Context) {
        let (maximized, fullscreen, minimized) = ctx.input(|i| {
            let v = i.viewport();
            (v.maximized, v.fullscreen, v.minimized)
        });
        let mut changed = false;
        if fullscreen != Some(true) && minimized != Some(true) {
            if let Some(maximized) = maximized
                && maximized != self.settings.window_maximized
            {
                self.settings.window_maximized = maximized;
                changed = true;
            }
            let size = (ctx.content_rect().size() * ctx.zoom_factor()).round();
            let kept = self.settings.window_size.unwrap_or(DEFAULT_WINDOW_SIZE);
            if maximized != Some(true)
                && self.big_meter_mode == BigMeterMode::Off
                && size.is_finite()
                && size != egui::Vec2::from(kept)
            {
                self.settings.window_size = Some(size.into());
                changed = true;
            }
        }
        // egui stores the panel's state when a drag ends, not during it.
        if let Some(panel) = egui::PanelState::load(ctx, egui::Id::new("reading_panel")) {
            let width = panel.outer_rect.width().round();
            if width.is_finite() && width != self.settings.reading_panel_width {
                self.settings.reading_panel_width = width;
                changed = true;
            }
        }
        if changed {
            self.settings_save.schedule(Instant::now());
        }
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

    /// Render a specs section, calling `render_fn` when spec data is available
    /// — a range row, or the mode's data alone — or showing a manual-only link
    /// as fallback.
    fn show_specs_with(
        &self,
        ui: &mut Ui,
        scale: f32,
        render_fn: impl FnOnce(
            &mut Ui,
            Option<&'static SpecInfo>,
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
        if spec.is_some() || mode_spec.is_some() {
            render_fn(ui, spec, mode_spec, manual_url, scale);
        } else if let Some(url) = manual_url {
            specs::show_manual_only(ui, url, scale);
        }
    }

    /// Render specs for the wide (side panel) layout, folded or not as the
    /// user last left its heading, with the fields picked in Settings.
    fn show_specs_section(&mut self, ui: &mut Ui, scale: f32) {
        let expanded = self.settings.specs_expanded;
        let fields = self.settings.spec_fields;
        let heading = self.settings.theme_colors(ui.visuals().dark_mode).heading();
        let mut toggled = false;
        self.show_specs_with(ui, scale, |ui, spec, mode_spec, manual_url, scale| {
            toggled = specs::show_specs(
                ui, spec, mode_spec, manual_url, scale, expanded, fields, heading,
            );
        });
        if toggled {
            self.settings.specs_expanded = !expanded;
            self.settings.save();
        }
    }

    /// Render specs for big meter mode (pipe-separated inline).
    pub(super) fn show_specs_section_inline(&self, ui: &mut Ui, scale: f32) {
        let fields = self.settings.spec_fields;
        self.show_specs_with(ui, scale, |ui, spec, _, manual_url, scale| {
            specs::show_specs_inline(ui, spec, manual_url, scale, fields);
        });
    }

    /// Render specs for the narrow (compact single-line) layout.
    fn show_specs_section_compact(&self, ui: &mut Ui) {
        let fields = self.settings.spec_fields;
        self.show_specs_with(ui, 1.0, |ui, spec, _, manual_url, _| {
            specs::show_specs_compact(ui, spec, manual_url, fields);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{
        SIDE_PANEL_DEFAULT_WIDTH, SIDE_PANEL_MAX_WIDTH, SIDE_PANEL_MIN_WIDTH, Settings,
    };
    use eframe::egui::ViewportInfo;
    use eframe::egui::scroll_area::ScrollAreaOutput;

    /// The page scrollers one multi-panel layout ends up with: the wide
    /// layout has one per column, the narrow layout only the reading column
    /// (the graph is stacked inside it).
    struct Columns {
        reading: ScrollAreaOutput<()>,
        graph: Option<ScrollAreaOutput<()>>,
    }

    /// Lay the panels out at `width` x `height` the way `App::ui` does — top
    /// bar pinned outside the scrollers, then the wide or narrow branch — and
    /// return the last frame's scrollers. Three frames because a scroll area
    /// sizes itself from the state the previous one left.
    fn run_layout(width: f32, height: f32) -> Columns {
        run_layout_with(width, height, Settings::default())
    }

    /// As [`run_layout`], with the panels the given settings show.
    fn run_layout_with(width: f32, height: f32, settings: Settings) -> Columns {
        run_layout_as(width, height, settings, true)
    }

    /// As [`run_layout_with`]; `record` says whether the session's samples
    /// are a recording or, with nothing recorded, the graph's history.
    fn run_layout_as(width: f32, height: f32, settings: Settings, record: bool) -> Columns {
        run_layout_marked(width, height, settings, record, 0)
    }

    /// As [`run_layout_as`], with `markers` markers in the Markers list.
    fn run_layout_marked(
        width: f32,
        height: f32,
        settings: Settings,
        record: bool,
        markers: u64,
    ) -> Columns {
        let settings = Settings {
            // No acquisition thread: this is about layout only.
            auto_connect: false,
            ..settings
        };
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        // A session's worth of data: only then does the graph draw its traces
        // and the recording its sample log — or the line saying Export… saves
        // the graph's samples — and only then does the split fill its
        // allocation to the last pixel, the case where a stray fraction of
        // content would raise a scrollbar on a window that fits.
        let m = dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        );
        if record {
            app.capture.recording.toggle(app.clock.now());
        }
        for _ in 0..50 {
            app.graph.push(
                1.234,
                std::time::Instant::now(),
                "DC V",
                "V",
                Some("  1.234"),
            );
            app.capture.recording.push(&m, 0);
        }
        for i in 0..markers {
            let at = m.timestamp + std::time::Duration::from_millis(i);
            let _ = app
                .markers
                .add(at, chrono::Local::now(), "1.234 V".to_string());
        }
        app.last_measurement = Some(m);

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
        let wide = super::super::meter_fit::is_wide(width);
        let mut columns = None;

        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx().clone();
                    egui::Panel::top("top_bar").show(ui, |ui| {
                        app.show_top_bar(ui, &ctx);
                        app.show_settings_panel(ui);
                    });
                    columns = Some(if wide {
                        let reading = egui::Panel::left("reading_panel")
                            .default_size(SIDE_PANEL_DEFAULT_WIDTH)
                            .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH)
                            .resizable(true)
                            .show(ui, |ui| {
                                app.show_reading_column_scrolled(ui, ContentLayout::Wide)
                            })
                            .inner;
                        let graph = egui::CentralPanel::default()
                            .show(ui, |ui| app.show_graph_column(ui))
                            .inner;
                        Columns {
                            reading,
                            graph: Some(graph),
                        }
                    } else {
                        let reading = egui::CentralPanel::default()
                            .show(ui, |ui| {
                                app.show_reading_column_scrolled(ui, ContentLayout::Narrow)
                            })
                            .inner;
                        Columns {
                            reading,
                            graph: None,
                        }
                    });
                },
            );
            // epaint 0.36 debug-asserts on dropping unapplied texture deltas;
            // this harness renders without a painter.
            out.textures_delta.clear();
        }

        columns.expect("the layout closure runs every frame")
    }

    /// A column whose content is no taller than its viewport: egui raises a
    /// scrollbar — and starts taking the wheel — the moment it is taller.
    fn assert_no_scrollbar(name: &str, column: &ScrollAreaOutput<()>) {
        assert!(
            column.content_size.y <= column.inner_rect.height() + 0.01,
            "{name}: content {} taller than the {} viewport",
            column.content_size.y,
            column.inner_rect.height(),
        );
        assert_eq!(column.state.offset, egui::Vec2::ZERO, "{name} is scrolled");
    }

    /// The Markers list takes only the room the recording panel has, with a
    /// recording's sample log after it or as the panel's last line.
    #[test]
    fn markers_fit_the_recording_panel_without_a_scrollbar() {
        for record in [false, true] {
            for (width, name) in [(1000.0, "wide"), (700.0, "narrow")] {
                let columns = run_layout_marked(width, 640.0, Settings::default(), record, 5);
                let column = columns.graph.as_ref().unwrap_or(&columns.reading);
                assert_no_scrollbar(&format!("{name}, record = {record}"), column);
            }
        }
    }

    /// A window with room for everything looks exactly as it did before the
    /// columns became scrollable: no scrollbar over the reading or the graph,
    /// and nothing scrolled out of sight at the top.
    #[test]
    fn a_wide_window_that_fits_shows_no_scrollbar_on_either_column() {
        let columns = run_layout(1000.0, 640.0);
        assert_no_scrollbar("the reading column", &columns.reading);
        assert_no_scrollbar(
            "the graph column",
            columns.graph.as_ref().expect("wide has a graph column"),
        );
    }

    /// With the recording panel hidden the graph is alone in the column and
    /// sizes itself, so nothing downstream can absorb a pixel it overshoots by.
    #[test]
    fn a_wide_window_without_the_recording_panel_shows_no_scrollbar() {
        let columns = run_layout_with(
            1000.0,
            640.0,
            Settings {
                show_recording: false,
                ..Settings::default()
            },
        );
        assert_no_scrollbar(
            "the graph column",
            columns.graph.as_ref().expect("wide has a graph column"),
        );
    }

    /// The narrow column stacks the graph and recording below the reading, so
    /// it is the one that has to fill its viewport exactly — the split is
    /// handed the height that is left rather than reading the viewport itself.
    #[test]
    fn a_narrow_window_that_fits_shows_no_scrollbar() {
        let columns = run_layout(700.0, 640.0);
        assert_no_scrollbar("the narrow column", &columns.reading);
        assert!(columns.graph.is_none(), "narrow has one column");
    }

    /// With nothing recorded the panel shows one wrapped line instead of the
    /// sample log, and the columns still fit without a scrollbar.
    #[test]
    fn a_session_with_nothing_recorded_shows_no_scrollbar() {
        let wide = run_layout_as(1000.0, 640.0, Settings::default(), false);
        assert_no_scrollbar("the reading column", &wide.reading);
        assert_no_scrollbar(
            "the graph column",
            wide.graph.as_ref().expect("wide has a graph column"),
        );
        let narrow = run_layout_as(700.0, 640.0, Settings::default(), false);
        assert_no_scrollbar("the narrow column", &narrow.reading);
    }

    /// Too short for the stack: the graph keeps its floor and the column grows
    /// past the viewport, which is what hands the user a scrollbar and the
    /// wheel instead of cropping the graph.
    #[test]
    fn a_window_too_short_for_the_stack_scrolls_the_column() {
        let columns = run_layout(700.0, 300.0);
        assert!(
            columns.reading.content_size.y > columns.reading.inner_rect.height() + 1.0,
            "content {} fits the {} viewport, so nothing would scroll",
            columns.reading.content_size.y,
            columns.reading.inner_rect.height(),
        );
    }

    /// One frame of [`App::track_layout`] in a `size` window (OS logical
    /// points) at `zoom`, with the viewport state the backend reports. The
    /// screen is in egui points, as egui-winit passes it: `size / zoom`.
    fn track(app: &mut App, ctx: &egui::Context, size: [f32; 2], zoom: f32, info: ViewportInfo) {
        ctx.set_zoom_factor(zoom);
        // Twice: a zoom set between passes applies from the next one.
        for _ in 0..2 {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::from(size) / zoom,
                )),
                viewports: std::iter::once((egui::ViewportId::ROOT, info.clone())).collect(),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| app.track_layout(ui.ctx()));
            out.textures_delta.clear();
        }
    }

    fn normal() -> ViewportInfo {
        ViewportInfo {
            maximized: Some(false),
            fullscreen: Some(false),
            minimized: Some(false),
            ..Default::default()
        }
    }

    /// A resized window is kept in OS logical points whatever the UI zoom;
    /// the default size, the same size again, and a maximized, fullscreen,
    /// minimized or big-meter window change nothing but the maximized flag.
    #[test]
    fn the_window_size_is_kept_only_when_the_user_picked_it() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let ctx = egui::Context::default();

        track(
            &mut app,
            &ctx,
            crate::settings::DEFAULT_WINDOW_SIZE,
            1.0,
            normal(),
        );
        assert_eq!(app.settings.window_size, None);
        assert!(!app.settings_save.take_pending(), "the default size");

        track(&mut app, &ctx, [1300.0, 900.0], 1.5, normal());
        assert_eq!(
            app.settings.window_size,
            Some([1300.0, 900.0]),
            "zoom undone"
        );
        assert!(app.settings_save.take_pending(), "resized");
        track(&mut app, &ctx, [1300.0, 900.0], 1.0, normal());
        assert!(!app.settings_save.take_pending(), "a zoom change alone");

        let maximized = ViewportInfo {
            maximized: Some(true),
            ..normal()
        };
        track(&mut app, &ctx, [1920.0, 1080.0], 1.0, maximized);
        assert_eq!(app.settings.window_size, Some([1300.0, 900.0]));
        assert!(app.settings.window_maximized);
        assert!(app.settings_save.take_pending(), "maximized");

        for info in [
            ViewportInfo {
                fullscreen: Some(true),
                ..normal()
            },
            // Some platforms report a minimized maximized window as not
            // maximized.
            ViewportInfo {
                minimized: Some(true),
                ..normal()
            },
        ] {
            track(&mut app, &ctx, [640.0, 480.0], 1.0, info);
            assert_eq!(app.settings.window_size, Some([1300.0, 900.0]));
            assert!(app.settings.window_maximized);
            assert!(!app.settings_save.take_pending());
        }

        track(&mut app, &ctx, [1300.0, 900.0], 1.0, normal());
        assert!(!app.settings.window_maximized, "un-maximized");
        app.settings_save.take_pending();
        app.big_meter_mode = BigMeterMode::Minimal;
        track(&mut app, &ctx, [300.0, 200.0], 1.0, normal());
        assert_eq!(app.settings.window_size, Some([1300.0, 900.0]));
        assert!(!app.settings_save.take_pending(), "a big meter readout");
    }
}
