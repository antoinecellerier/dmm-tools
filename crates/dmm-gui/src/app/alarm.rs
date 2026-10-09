//! The **Alarm** chip and row beside **Scale**, and the alarm behind them:
//! started from the limits the row applies, fed every reading as it arrives,
//! and each breach marked on its reading.
//!
//! With the reading rather than on the graph's toolbar: the alarm judges the
//! main reading whatever the graph plots, its verdict shows on the reading
//! and in Statistics, and an unattended run in the big meter has no graph.
//! The graph only draws the limits it is handed (`Graph::alarm_view`).
//!
//! The rules — base units, the quantity bound, what counts as a breach — are
//! `dmm_lib::alarm`'s, shared with `dmm-cli read --alarm-*`. Session-only,
//! like the transform: limits left in a settings file would raise alarms on
//! the next session's readings unasked.

use super::App;
use super::appearance::SMALL_TEXT_SIZE;
use super::controls::remote::SplitChip;
use super::marker_list::log_line;
use super::toast::Toast;
use crate::display::ReadingState;
use dmm_lib::alarm::{Alarm, Breach, Hysteresis, HysteresisError, LimitError, Limits};
use dmm_lib::measurement::Measurement;
use eframe::egui::{self, RichText, Ui};

/// Draft text for the two limits, kept apart from the alarm in force: the
/// row applies on Apply or Enter only, so typing 12 never raises an alarm at
/// 1 on the way.
#[derive(Default)]
pub(super) struct AlarmEditor {
    pub(super) open: bool,
    low: String,
    high: String,
    /// The hysteresis band: a value, `1%`, or blank for the automatic one.
    band: String,
    /// Put the caret in the low field the next time the row is drawn: set
    /// when the chip opens it.
    focus: bool,
}

impl AlarmEditor {
    /// Type `low` and `high` into the fields.
    #[cfg(test)]
    pub(super) fn set_drafts(&mut self, low: &str, high: &str) {
        self.low = low.to_string();
        self.high = high.to_string();
    }

    /// No limit typed: nothing for the chip to switch on. A band alone
    /// watches nothing.
    fn is_blank(&self) -> bool {
        self.low.trim().is_empty() && self.high.trim().is_empty()
    }
}

/// Tooltip on the chip's caret.
const ALARM_HOVER: &str = "Set the limits";

/// Width of each limit field, in points before zoom: the Scale row's.
const FIELD_WIDTH: f32 = 50.0;

/// The chip's name.
const ALARM_NAME: &str = "Alarm";

/// Why a pair of drafts was not applied, in the GUI's words.
fn limit_message(e: LimitError) -> &'static str {
    match e {
        LimitError::NotFinite => "Invalid limit: must be a finite number",
        LimitError::LowNotBelowHigh => "Invalid limits: the low limit must be below the high one",
    }
}

/// A draft as a limit: blank is no limit, anything else must be a number.
fn parse_draft(text: &str, name: &str) -> Result<Option<f64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| format!("Invalid {name} limit: \u{201c}{text}\u{201d} is not a number"))
}

/// The drafts as a pair of limits and their band, or the toast saying which
/// field to fix.
fn parse_limits(low: &str, high: &str, band: &str) -> Result<(Limits, Hysteresis), String> {
    let low = parse_draft(low, "low")?;
    let high = parse_draft(high, "high")?;
    let limits = Limits::check(low, high).map_err(|e| limit_message(e).to_string())?;
    let band = Hysteresis::parse(band).map_err(|e| match e {
        HysteresisError::NotANumber => format!(
            "Invalid hysteresis: \u{201c}{}\u{201d} is not a number or a percentage",
            band.trim()
        ),
        HysteresisError::NotABand | HysteresisError::PercentOfZero => {
            "Invalid hysteresis: must be zero or more".to_string()
        }
    })?;
    let band = band.check(&limits).map_err(|_| {
        "Invalid hysteresis: a percentage of a limit of 0 is 0; type a value instead".to_string()
    })?;
    Ok((limits, band))
}

/// Tooltip on the hysteresis field.
const BAND_HOVER: &str = "How far back the reading must come before a limit alarms again: a value, or a percentage such as 1%. Blank: automatic";

/// A reading that breached a limit, waiting for its marker at the end of the
/// frame's drain.
pub(super) struct PendingBreach {
    at: std::time::Instant,
    wall_time: chrono::DateTime<chrono::Local>,
    reading: String,
    note: String,
}

impl PendingBreach {
    fn new(m: &Measurement, breach: &Breach) -> Self {
        Self {
            at: m.timestamp,
            wall_time: m.wall_time.into(),
            reading: log_line(m),
            note: breach.note(),
        }
    }
}

impl App {
    /// Whether the readings the alarm judges went through the scale: a
    /// file's never do, it holds them as they were shown.
    pub(super) fn alarm_scaled(&self) -> bool {
        !self.transform.is_identity() && self.imported.is_none() && self.import_job.is_none()
    }

    /// What the reading display adds to the meter's reading.
    pub(super) fn reading_state(&self) -> ReadingState {
        ReadingState {
            scaled: !self.transform.is_identity(),
            alarm_set: self.alarm.is_some(),
            alarm: self.alarm.as_ref().and_then(|a| a.reading_zone()),
        }
    }

    /// The **Alarm** chip: filled while an alarm is set.
    pub(super) fn alarm_chip(&self) -> SplitChip<'static> {
        let active = self.alarm.is_some();
        SplitChip {
            name: ALARM_NAME,
            active,
            ready: active || !self.alarm_editor.is_blank(),
            open: (!self.meter_only()).then_some(self.alarm_editor.open),
            hover: "Turn the alarm on or off",
            fields_hover: ALARM_HOVER,
            unready_hover: self.chip_setup_hint(),
        }
    }

    /// The chip's body: the alarm off if on; else on with the fields'
    /// limits, or with none typed the row opened to type them.
    pub(super) fn toggle_alarm(&mut self) {
        if self.alarm.is_some() {
            self.set_limits(Limits::default(), Hysteresis::Auto);
        } else if self.alarm_editor.is_blank() {
            if !self.alarm_editor.open {
                self.toggle_alarm_editor();
            }
            self.alarm_editor.focus = true;
        } else {
            self.apply_alarm_fields();
        }
    }

    /// Apply what the fields hold, or say which to fix: the row's Apply, and
    /// the chip's body.
    fn apply_alarm_fields(&mut self) {
        let editor = &self.alarm_editor;
        match parse_limits(&editor.low, &editor.high, &editor.band) {
            Ok((limits, band)) => self.set_limits(limits, band),
            // The alarm in force, if any, keeps running.
            Err(message) => self.toast = Some(Toast::error(message)),
        }
    }

    /// Open the row if closed, with the caret in the low field and the
    /// Scale row closed, so one row sits under the chips; close it if open:
    /// the chip's caret.
    pub(super) fn toggle_alarm_editor(&mut self) {
        self.alarm_editor.open = !self.alarm_editor.open;
        self.alarm_editor.focus = self.alarm_editor.open;
        if self.alarm_editor.open {
            self.transform_editor.open = false;
        }
    }

    /// The `Low [ ] High [ ] V [Apply] [Off]` row under the buttons, shown
    /// while the editor is open, laid out as the Scale row is.
    pub(super) fn show_alarm_editor(&mut self, ui: &mut Ui, scale: f32) {
        if !self.alarm_editor.open {
            return;
        }
        // Floored at the 11 pt minimum, as the Scale row's: this row holds
        // text the user types, and the big meter's scale goes below 0.4.
        let font_size = (12.0 * scale).max(SMALL_TEXT_SIZE);
        let font = egui::FontId::proportional(font_size);
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        // What the limits are in, or why they are idle: the graph's copy of
        // the alarm's state, which `sync_alarm_view` keeps.
        let view = &self.graph.alarm_view;
        let status = match (&view.idle, &view.unit) {
            (Some(idle), _) => Some((
                format!("idle: {idle}"),
                tc.status_warning(),
                format!(
                    "Waiting: the readings are in {idle}, the limits in {}",
                    view.watched.as_deref().unwrap_or_default()
                ),
            )),
            (None, Some(unit)) => Some((
                unit.clone(),
                ui.visuals().weak_text_color(),
                "The unit the limits are in".to_string(),
            )),
            // No alarm bound yet: the unit a limit typed now would be in,
            // from the reading on screen, so "121" is known to mean volts
            // before Apply rather than after.
            (None, None) => self
                .last_measurement
                .as_ref()
                .and_then(|m| dmm_lib::alarm::limit_unit(m, self.alarm_scaled()))
                .map(|unit| {
                    (
                        unit.to_string(),
                        ui.visuals().weak_text_color(),
                        "The unit the limits are in, from the reading on screen".to_string(),
                    )
                }),
        };

        // Collected rather than acted on inside the closure, which borrows
        // the editor's fields.
        let mut apply = false;
        let mut off = false;
        let editor = &mut self.alarm_editor;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;
            let width = FIELD_WIDTH * scale;
            let mut field = |ui: &mut Ui,
                             caption: &str,
                             text: &mut String,
                             hint: &str,
                             focus: bool|
             -> egui::Response {
                // A caption and its field wrap together: a caption left at
                // the end of a line reads as labelling nothing.
                let caption = RichText::new(caption).font(font.clone());
                let caption_width = egui::WidgetText::from(caption.clone())
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        egui::TextStyle::Body,
                    )
                    .size()
                    .x;
                let needed = caption_width + ui.spacing().item_spacing.x + width;
                if needed > ui.available_size_before_wrap().x {
                    ui.end_row();
                }
                ui.label(caption);
                let resp = ui.add(
                    egui::TextEdit::singleline(text)
                        .desired_width(width)
                        .font(font.clone())
                        .hint_text(RichText::new(hint).font(font.clone())),
                );
                if focus {
                    resp.request_focus();
                }
                crate::a11y::paint_focus_ring(ui, &resp);
                // Enter, never `.changed()`: a limit taken per keystroke
                // would raise an alarm at 1 on the way to typing 12.
                apply |= resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                resp
            };
            let focus = std::mem::take(&mut editor.focus);
            field(ui, "Low", &mut editor.low, "none", focus);
            field(ui, "High", &mut editor.high, "none", false);
            // Right after the limits it qualifies, so a wrapped row keeps the
            // unit on their line.
            if let Some((text, color, hover)) = status {
                ui.label(RichText::new(text).font(font.clone()).color(color))
                    .on_hover_text(hover);
            }
            field(ui, "\u{b1}", &mut editor.band, "auto", false).on_hover_text(BAND_HOVER);
            apply |= ui
                .add(egui::Button::new(RichText::new("Apply").font(font.clone())))
                .on_hover_text("Turn the alarm on with these limits")
                .clicked();
            off |= ui
                .add(egui::Button::new(RichText::new("Off").font(font.clone())))
                .on_hover_text("Turn the alarm off")
                .clicked();
        });

        // Off keeps the fields, so Apply turns the same limits back on.
        if off {
            self.set_limits(Limits::default(), Hysteresis::Auto);
        } else if apply {
            self.apply_alarm_fields();
        }
    }

    /// Start a new alarm on `limits` and its hysteresis `band`, or stop the
    /// alarm on none.
    ///
    /// A new alarm binds afresh to the next reading's quantity: applying is
    /// the user saying what to watch now.
    pub(super) fn set_limits(&mut self, limits: Limits, band: Hysteresis) {
        if limits.is_empty() {
            if self.alarm.take().is_some() {
                self.toast = Some(Toast::info("Alarm off"));
            }
        } else {
            self.alarm = Some(Alarm::new(limits, band));
            self.toast = Some(Toast::info(format!("Alarm on: {}", limits.describe(band))));
        }
        self.sync_alarm_view();
    }

    /// Judge one reading, as shown, against the limits; `scaled` says
    /// whether it went through a software transform — the live session's,
    /// or none for a reading imported as the file holds it.
    pub(super) fn check_alarm(&mut self, m: &Measurement, scaled: bool) -> Option<PendingBreach> {
        let breach = self.alarm.as_mut()?.check(m, scaled)?;
        if !breach.crossing {
            // Out before the alarm saw it cross: said, not marked. In the
            // error style all the same, as the badge and the reading are:
            // the reading is past a limit, which is no good news.
            self.toast = Some(Toast::error(breach.note()));
            return None;
        }
        Some(PendingBreach::new(m, &breach))
    }

    /// Judge one imported reading, as the file holds it: as
    /// [`check_alarm`](Self::check_alarm), but a first reading found out
    /// goes unsaid — the import's progress and result toasts hold the slot.
    pub(super) fn check_imported(&mut self, m: &Measurement) -> Option<PendingBreach> {
        let breach = self.alarm.as_mut()?.check(m, false)?;
        breach.crossing.then(|| PendingBreach::new(m, &breach))
    }

    /// Mark each breach the drain found on its reading. No toast: the badge,
    /// the reading's colour, the marker and the Alarms count already say it,
    /// and a toast over a narrow window or the big meter hid the reading.
    pub(super) fn mark_breaches(&mut self, breaches: Vec<PendingBreach>) {
        for b in breaches {
            self.markers
                .add_noted(b.at, b.wall_time, b.reading, &b.note);
        }
    }

    /// Hand the graph what the alarm watches, for its lines and the unit
    /// after the fields. Compared before it is written, as it runs once per
    /// drained frame.
    pub(super) fn sync_alarm_view(&mut self) {
        let scaled = self.alarm_scaled();
        let relabel = self.transform.unit.as_deref().filter(|_| scaled);
        let alarm = self.alarm.as_ref();
        let watched = alarm.and_then(|a| a.quantity());
        let idle = alarm.and_then(|a| a.idle());
        let unit = alarm.and_then(|a| a.limits_unit(relabel));
        let limits = alarm.map(|a| a.limits()).unwrap_or_default();
        let view = &mut self.graph.alarm_view;
        view.limits = limits;
        if view.watched.as_deref() != watched {
            view.watched = watched.map(str::to_string);
        }
        if view.idle.as_deref() != idle {
            view.idle = idle.map(str::to_string);
        }
        if view.unit.as_deref() != unit {
            view.unit = unit.map(str::to_string);
        }
        view.scaled = scaled;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_limits;
    use crate::app::App;
    use crate::settings::Settings;
    use dmm_lib::alarm::{Hysteresis, Limits};
    use dmm_lib::flags::StatusFlags;
    use dmm_lib::measurement::{MeasuredValue, Measurement};

    fn reading(app: &App, value: f64, unit: &'static str) -> Measurement {
        let mut m =
            Measurement::test_fixture(MeasuredValue::Normal(value), unit, StatusFlags::default());
        m.timestamp = app.clock.now();
        m
    }

    fn app_with_high_limit(high: f64) -> App {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::manual());
        app.set_limits(Limits::check(None, Some(high)).unwrap(), Hysteresis::Auto);
        app
    }

    /// A breach is marked on its reading and counted; the reading display
    /// shows it; another quantity idles the alarm and its lines.
    #[test]
    fn a_breach_marks_its_reading_and_counts() {
        let mut app = app_with_high_limit(5.0);
        assert!(app.alarm.is_some());
        let inside = reading(&app, 4900.0, "mV");
        assert!(app.check_alarm(&inside, false).is_none());

        app.toast = None;
        let m = reading(&app, 5100.0, "mV");
        app.capture
            .ingest(m.clone(), &app.transform, &mut app.graph, &app.connection);
        let breach = app.check_alarm(&m, false);
        assert!(breach.is_some());
        app.mark_breaches(breach.into_iter().collect());
        let marker = app.markers.iter().next().expect("a marker");
        assert_eq!(marker.note, "Above high limit 5 V");
        assert!(app.toast.is_none(), "the badge says it, not a toast");
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 1);
        assert_eq!(app.reading_state().alarm, Some(dmm_lib::alarm::Zone::Above));

        app.sync_alarm_view();
        assert_eq!(app.graph.alarm_view.watched.as_deref(), Some("V"));
        let ohms = reading(&app, 12.0, "kΩ");
        assert!(app.check_alarm(&ohms, false).is_none());
        app.sync_alarm_view();
        assert_eq!(app.graph.alarm_view.idle.as_deref(), Some("Ω"));

        app.clear_session();
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 0);
    }

    #[test]
    fn the_drafts_must_make_a_pair_of_limits() {
        assert_eq!(
            parse_limits(" ", "5.25", ""),
            Ok((Limits::check(None, Some(5.25)).unwrap(), Hysteresis::Auto))
        );
        assert_eq!(
            parse_limits("", "5", "2%").map(|(_, band)| band),
            Ok(Hysteresis::Percent(2.0))
        );
        assert!(parse_limits("5", "1", "").is_err());
        let message = parse_limits("abc", "", "").unwrap_err();
        assert!(message.contains("low limit"), "{message}");
        assert!(parse_limits("", "inf", "").is_err());
        assert!(parse_limits("", "5", "-1").is_err());
    }

    /// A reading already out when the limits are set is said in a toast,
    /// not marked or counted: no crossing was seen.
    #[test]
    fn a_reading_already_out_is_said_not_marked() {
        let mut app = app_with_high_limit(5.0);
        let m = reading(&app, 6.0, "V");
        assert!(app.check_alarm(&m, false).is_none());
        let toast = app.toast.as_ref().expect("a toast");
        assert!(toast.is_error, "past a limit, like the badge");
        assert!(
            toast.message.starts_with("Already above high limit 5 V"),
            "{}",
            toast.message
        );
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 0);
        assert!(app.markers.iter().next().is_none());
        assert_eq!(app.reading_state().alarm, Some(dmm_lib::alarm::Zone::Above));
    }

    /// Clear and Reset zero the count but keep where the reading lies: one
    /// still out is the breach it was, with no "already out" toast.
    #[test]
    fn a_clear_keeps_a_reading_that_is_out_quiet() {
        let mut app = app_with_high_limit(5.0);
        assert!(app.check_alarm(&reading(&app, 4.0, "V"), false).is_none());
        assert!(app.check_alarm(&reading(&app, 6.0, "V"), false).is_some());
        app.clear_session();
        app.toast = None;
        assert!(app.check_alarm(&reading(&app, 6.0, "V"), false).is_none());
        assert!(app.toast.is_none());
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 0);
    }

    /// An import judges the file afresh: the live session's quantity and
    /// zone don't carry over, so a file starting out is found out, not seen
    /// crossing — and unsaid, the import's own toasts hold the slot.
    #[test]
    fn an_import_judges_the_file_afresh() {
        let mut app = app_with_high_limit(5.0);
        assert!(app.check_alarm(&reading(&app, 4.0, "V"), false).is_none());
        app.reset_session_for_import();
        let alarm = app.alarm.as_ref().expect("still set");
        assert_eq!(alarm.quantity(), None);
        assert_eq!(alarm.limits().high, Some(5.0));
        app.toast = None;
        assert!(app.check_imported(&reading(&app, 6.0, "V")).is_none());
        assert!(app.toast.is_none());
        assert_eq!(app.reading_state().alarm, Some(dmm_lib::alarm::Zone::Above));
        assert!(app.check_imported(&reading(&app, 4.0, "V")).is_none());
        assert!(app.check_imported(&reading(&app, 6.0, "V")).is_some());
    }

    /// Off stops the alarm and the graph's lines with it.
    #[test]
    fn no_limits_turn_the_alarm_off() {
        let mut app = app_with_high_limit(5.0);
        assert_eq!(app.graph.alarm_view.limits.high, Some(5.0));
        app.set_limits(Limits::default(), Hysteresis::Auto);
        assert!(app.alarm.is_none());
        assert!(app.graph.alarm_view.limits.is_empty());
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Alarm off")
        );
    }
}
