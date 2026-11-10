//! The **Alarm** chip and row beside **Scale**, and the alarm behind them:
//! started from the limits the row applies, fed every reading as it arrives,
//! and each breach marked on its reading.
//!
//! With the reading rather than on the graph's toolbar: the alarm judges the
//! main reading whatever the graph plots, its verdict shows on the reading
//! and in Statistics, and an unattended run in the big meter has no graph.
//! The graph draws the limits it is handed (`watched_limits`), and its menu
//! sets or removes one at a height (`set_limit_from_graph`).
//!
//! The rules — base units, the quantity bound, what counts as a breach — are
//! `dmm_lib::alarm`'s, shared with `dmm-cli read --alarm-*`. Session-only,
//! like the transform: limits left in a settings file would raise alarms on
//! the next session's readings unasked.

use super::App;
use super::appearance::SMALL_TEXT_SIZE;
use super::controls::remote::{SplitChip, row_field};
use super::marker_list::log_line;
use super::toast::Toast;
use crate::a11y::AnnouncePolicy;
use crate::display::ReadingState;
use crate::graph::{LimitSide, WatchedLimits};
use dmm_lib::alarm::{Alarm, Breach, Hysteresis, LimitError, LimitUnit, Limits};
use dmm_lib::measurement::Measurement;
use dmm_shared::l10n::Localizer;
use dmm_shared::{input, tr};
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

/// Why a pair of drafts was not applied.
fn limit_message(l10n: &Localizer, e: LimitError) -> String {
    let reason = input::limit_error(l10n, e);
    match e {
        LimitError::NotFinite => tr!(l10n, "alarm-invalid-limit", reason = reason),
        LimitError::LowNotBelowHigh => tr!(l10n, "alarm-invalid-limits", reason = reason),
    }
    .into_owned()
}

/// The draft for the `side` limit as a limit: blank is no limit, anything
/// else must be a number.
fn parse_draft(l10n: &Localizer, text: &str, side: LimitSide) -> Result<Option<f64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse().map(Some).map_err(|_| {
        let reason = input::not_a_number(l10n, text);
        match side {
            LimitSide::Low => tr!(l10n, "alarm-invalid-low", reason = reason),
            LimitSide::High => tr!(l10n, "alarm-invalid-high", reason = reason),
        }
        .into_owned()
    })
}

/// The drafts as a pair of limits and their band, or the toast saying which
/// field to fix.
fn parse_limits(
    l10n: &Localizer,
    low: &str,
    high: &str,
    band: &str,
) -> Result<(Limits, Hysteresis), String> {
    let low = parse_draft(l10n, low, LimitSide::Low)?;
    let high = parse_draft(l10n, high, LimitSide::High)?;
    let limits = Limits::check(low, high).map_err(|e| limit_message(l10n, e))?;
    let band = Hysteresis::parse(band).map_err(|e| {
        let reason = input::hysteresis_error(l10n, e, band.trim());
        tr!(l10n, "alarm-invalid-band", reason = reason).into_owned()
    })?;
    let band = band.check(&limits).map_err(|_| percent_of_zero(l10n))?;
    Ok((limits, band))
}

/// Why a percentage band can't go with the limits.
fn percent_of_zero(l10n: &Localizer) -> String {
    let reason = input::percent_of_zero(l10n);
    tr!(l10n, "alarm-invalid-band", reason = reason).into_owned()
}

/// A reading that breached a limit, waiting for its marker at the end of the
/// frame's drain.
pub(super) struct PendingBreach {
    at: std::time::Instant,
    wall_time: chrono::DateTime<chrono::Local>,
    reading: String,
    /// In the app's language, as a note typed into the marker would be:
    /// the user reads it and can edit it, and recordings and exports keep
    /// it as it is.
    note: String,
}

impl PendingBreach {
    fn new(l10n: &Localizer, m: &Measurement, breach: &Breach) -> Self {
        Self {
            at: m.timestamp,
            wall_time: m.wall_time.into(),
            reading: log_line(l10n, m),
            note: l10n.breach_note(breach),
        }
    }
}

impl App {
    /// The unit the alarm's limits are in for the readings on screen: shown
    /// when they went through a scale, live or when a file was written (see
    /// [`LimitUnit::of`]). Not the alarm's own binding while a reading is
    /// on screen: an idle alarm holds it for readings no longer there.
    /// Without one (a Clear), the binding's, else the scale's: a file's
    /// readings say for themselves.
    pub(super) fn alarm_limit_unit(&self) -> LimitUnit {
        let bound = self.alarm.as_ref().and_then(|a| a.binding());
        match (&self.last_measurement, bound) {
            (Some(m), _) => LimitUnit::of(m),
            (None, Some(b)) => b.limit_unit(),
            (None, None) if !self.transform.is_identity() && !self.source.is_import() => {
                LimitUnit::Shown
            }
            (None, None) => LimitUnit::Base,
        }
    }

    /// What the reading display adds to the meter's reading.
    pub(super) fn reading_state(&self) -> ReadingState {
        ReadingState {
            scaled: !self.transform.is_identity(),
            alarm_set: self.alarm.is_some(),
            alarm: self.alarm.as_ref().and_then(|a| a.reading_zone()),
            announce: AnnouncePolicy::new(
                self.settings.announce_interval_secs,
                self.settings.announce_only_focused,
            ),
        }
    }

    /// The **Alarm** chip: filled while an alarm is set.
    pub(super) fn alarm_chip(&self, ctx: &egui::Context) -> SplitChip<'_> {
        let active = self.alarm.is_some();
        SplitChip {
            name: tr!(self.l10n, "alarm-chip"),
            active,
            ready: active || !self.alarm_editor.is_blank(),
            open: (!self.meter_only()).then_some(self.alarm_editor.open),
            hover: tr!(self.l10n, "alarm-chip.hover"),
            fields_name: tr!(self.l10n, "alarm-settings.a11y"),
            fields_hover: tr!(self.l10n, "alarm-settings.hover"),
            unready_hover: self.chip_setup_hint(ctx),
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
        match parse_limits(&self.l10n, &editor.low, &editor.high, &editor.band) {
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
        // What the limits are in, or why they are idle.
        let alarm = self.alarm.as_ref();
        let relabel = self
            .transform
            .unit
            .as_deref()
            // A file's readings are not scaled again.
            .filter(|_| !self.source.is_import());
        let idle = alarm.and_then(|a| a.idle());
        let unit = alarm.and_then(|a| a.limits_unit(relabel));
        let l10n = &self.l10n;
        let status = match (idle, unit) {
            (Some(idle), _) => Some((
                tr!(l10n, "alarm-idle", unit = idle).into_owned(),
                tc.status_warning(),
                tr!(
                    l10n,
                    "alarm-idle.hover",
                    unit = idle,
                    limits = alarm.and_then(|a| a.bound_unit()).unwrap_or_default(),
                ),
            )),
            (None, Some(unit)) => Some((
                unit.to_string(),
                ui.visuals().weak_text_color(),
                tr!(l10n, "alarm-unit.hover"),
            )),
            // No alarm bound yet: the unit a limit typed now would be in,
            // from the reading on screen, so "121" is known to mean volts
            // before Apply rather than after.
            (None, None) => self
                .last_measurement
                .as_ref()
                .and_then(|m| dmm_lib::alarm::limit_unit(m))
                .map(|unit| {
                    (
                        unit.to_string(),
                        ui.visuals().weak_text_color(),
                        tr!(l10n, "alarm-unit-from-reading.hover"),
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
            let mut field = |ui: &mut Ui, caption: &str, text, hint: &str, focus| {
                let (resp, committed) = row_field(ui, &font, scale, caption, text, hint, focus);
                apply |= committed;
                resp
            };
            let focus = std::mem::take(&mut editor.focus);
            let none = tr!(l10n, "alarm-no-limit");
            field(ui, &tr!(l10n, "alarm-low"), &mut editor.low, &none, focus);
            field(ui, &tr!(l10n, "alarm-high"), &mut editor.high, &none, false);
            // Right after the limits it qualifies, so a wrapped row keeps the
            // unit on their line.
            if let Some((text, color, hover)) = status {
                ui.label(RichText::new(text).font(font.clone()).color(color))
                    .on_hover_text(hover);
            }
            field(
                ui,
                "\u{b1}",
                &mut editor.band,
                &tr!(l10n, "alarm-band"),
                false,
            )
            .on_hover_text(tr!(l10n, "alarm-band.hover"));
            apply |= ui
                .add(egui::Button::new(
                    RichText::new(tr!(l10n, "alarm-apply")).font(font.clone()),
                ))
                .on_hover_text(tr!(l10n, "alarm-apply.hover"))
                .clicked();
            off |= ui
                .add(egui::Button::new(
                    RichText::new(tr!(l10n, "alarm-off")).font(font.clone()),
                ))
                .on_hover_text(tr!(l10n, "alarm-off.hover"))
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
                self.toast = Some(Toast::info(tr!(self.l10n, "alarm-off-toast")));
            }
        } else {
            self.alarm = Some(Alarm::new(limits, band));
            let limits = self.l10n.alarm_limits(&limits, band);
            self.toast = Some(Toast::info(tr!(
                self.l10n,
                "alarm-on-toast",
                limits = limits
            )));
        }
    }

    /// Set one limit from the graph's menu at `value`, in the plotted unit,
    /// or remove it on `None`. The other limit and the band stay as the
    /// alarm in force has them, unless that alarm is idle on another
    /// quantity, whose limits mean nothing here: a fresh alarm starts. While
    /// an alarm is in force, text typed in the row and not applied stays out
    /// of it; with the alarm off, the row's limits and band are the ones Off
    /// kept for the next Apply, and stay if they make a pair. The row is then
    /// written with the limits and band in force.
    pub(super) fn set_limit_from_graph(&mut self, side: LimitSide, value: Option<f64>) {
        let fresh = (Limits::default(), Hysteresis::Auto);
        let (current, band) = match &self.alarm {
            Some(a) if a.idle().is_none() => (a.limits(), a.hysteresis()),
            Some(_) => fresh,
            None => {
                let editor = &self.alarm_editor;
                parse_limits(&self.l10n, &editor.low, &editor.high, &editor.band).unwrap_or(fresh)
            }
        };
        let shown = Limits {
            low: value,
            high: None,
        };
        let value = shown
            .from_unit(self.graph.plotted_unit(), self.alarm_limit_unit())
            .low;
        let (low, high) = match side {
            LimitSide::Low => (value, current.high),
            LimitSide::High => (current.low, value),
        };
        let limits = match Limits::check(low, high) {
            Ok(limits) => limits,
            Err(e) => {
                self.toast = Some(Toast::error(limit_message(&self.l10n, e)));
                return;
            }
        };
        let Ok(band) = band.check(&limits) else {
            self.toast = Some(Toast::error(percent_of_zero(&self.l10n)));
            return;
        };
        self.set_limits(limits, band);
        let draft = |v: Option<f64>| v.map_or_else(String::new, |v| v.to_string());
        self.alarm_editor.low = draft(limits.low);
        self.alarm_editor.high = draft(limits.high);
        self.alarm_editor.band = band.describe().unwrap_or_default();
    }

    /// Judge one reading, as shown, against the limits.
    pub(super) fn check_alarm(&mut self, m: &Measurement) -> Option<PendingBreach> {
        let breach = self.alarm.as_mut()?.check(m)?;
        if !breach.crossing {
            // Out before the alarm saw it cross: said, not marked. In the
            // error style all the same, as the badge and the reading are:
            // the reading is past a limit, which is no good news.
            self.toast = Some(Toast::error(self.l10n.breach_note(&breach)));
            return None;
        }
        Some(PendingBreach::new(&self.l10n, m, &breach))
    }

    /// Judge one imported reading, as the file holds it: as
    /// [`check_alarm`](Self::check_alarm), but a first reading found out
    /// goes unsaid — the import's progress and result toasts hold the slot.
    pub(super) fn check_imported(&mut self, m: &Measurement) -> Option<PendingBreach> {
        let breach = self.alarm.as_mut()?.check(m)?;
        breach
            .crossing
            .then(|| PendingBreach::new(&self.l10n, m, &breach))
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

    /// The limits the graph draws, on the main reading's trace only: not
    /// while the alarm is idle on another quantity. Before a reading binds
    /// it, the next one will, and the trace on screen stands for it: limits
    /// set after a replay ends, or while paused, draw as the other lines do.
    pub(super) fn watched_limits(&self) -> Option<WatchedLimits> {
        let alarm = self.alarm.as_ref()?;
        alarm.idle().is_none().then(|| WatchedLimits {
            limits: alarm.limits(),
            unit: self.alarm_limit_unit(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::graph::LimitSide;
    use crate::l10n::english;
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
        assert!(app.check_alarm(&inside).is_none());

        app.toast = None;
        let m = reading(&app, 5100.0, "mV");
        app.capture
            .ingest(m.clone(), &app.transform, &mut app.graph, &app.connection);
        let breach = app.check_alarm(&m);
        assert!(breach.is_some());
        app.mark_breaches(breach.into_iter().collect());
        let marker = app.markers.iter().next().expect("a marker");
        assert_eq!(marker.note, "Above high limit 5 V");
        assert!(app.toast.is_none(), "the badge says it, not a toast");
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 1);
        assert_eq!(app.reading_state().alarm, Some(dmm_lib::alarm::Zone::Above));

        assert!(app.watched_limits().is_some());
        let ohms = reading(&app, 12.0, "kΩ");
        assert!(app.check_alarm(&ohms).is_none());
        assert!(app.watched_limits().is_none(), "idle on ohms: no lines");

        app.clear_session();
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 0);
    }

    fn parse_limits(
        low: &str,
        high: &str,
        band: &str,
    ) -> Result<(dmm_lib::alarm::Limits, Hysteresis), String> {
        super::parse_limits(&english(), low, high, band)
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
        assert!(app.check_alarm(&m).is_none());
        let toast = app.toast.as_ref().expect("a toast");
        assert!(toast.is_error(), "past a limit, like the badge");
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
        assert!(app.check_alarm(&reading(&app, 4.0, "V")).is_none());
        assert!(app.check_alarm(&reading(&app, 6.0, "V")).is_some());
        app.clear_session();
        app.toast = None;
        assert!(app.check_alarm(&reading(&app, 6.0, "V")).is_none());
        assert!(app.toast.is_none());
        assert_eq!(app.alarm.as_ref().unwrap().high_count, 0);
    }

    /// An import judges the file afresh: the live session's quantity and
    /// zone don't carry over, so a file starting out is found out, not seen
    /// crossing — and unsaid, the import's own toasts hold the slot.
    #[test]
    fn an_import_judges_the_file_afresh() {
        let mut app = app_with_high_limit(5.0);
        assert!(app.check_alarm(&reading(&app, 4.0, "V")).is_none());
        app.reset_session_for_import();
        let alarm = app.alarm.as_ref().expect("still set");
        assert_eq!(alarm.bound_unit(), None);
        assert_eq!(alarm.limits().high, Some(5.0));
        app.toast = None;
        assert!(app.check_imported(&reading(&app, 6.0, "V")).is_none());
        assert!(app.toast.is_none());
        assert_eq!(app.reading_state().alarm, Some(dmm_lib::alarm::Zone::Above));
        assert!(app.check_imported(&reading(&app, 4.0, "V")).is_none());
        assert!(app.check_imported(&reading(&app, 6.0, "V")).is_some());
    }

    /// The graph's menu sets a limit at a height in the plotted unit, keeps
    /// the other limit and the band, and writes the row to match.
    #[test]
    fn the_graph_sets_a_limit_in_the_plotted_unit() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::manual());
        app.set_limits(
            Limits::check(Some(1.0), None).unwrap(),
            Hysteresis::Percent(1.0),
        );
        let m = reading(&app, 4900.0, "mV");
        app.capture
            .ingest(m, &app.transform, &mut app.graph, &app.connection);
        app.set_limit_from_graph(LimitSide::High, Some(3290.0));
        let alarm = app.alarm.as_ref().expect("an alarm");
        assert_eq!(
            alarm.limits(),
            Limits::check(Some(1.0), Some(3.29)).unwrap()
        );
        assert_eq!(alarm.hysteresis(), Hysteresis::Percent(1.0));
        assert_eq!(
            (
                app.alarm_editor.low.as_str(),
                app.alarm_editor.high.as_str()
            ),
            ("1", "3.29")
        );
        assert_eq!(app.alarm_editor.band, "1%");
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Alarm on: above 3.29 or below 1, hysteresis 1%")
        );

        app.set_limit_from_graph(LimitSide::Low, None);
        let alarm = app.alarm.as_ref().expect("still on");
        assert_eq!(alarm.limits(), Limits::check(None, Some(3.29)).unwrap());
        assert_eq!(app.alarm_editor.low, "");
        app.set_limit_from_graph(LimitSide::High, None);
        assert!(app.alarm.is_none(), "no limit left");
    }

    /// With the alarm off, the graph's menu keeps the row's other limit and
    /// band, as Off kept them for the next Apply.
    #[test]
    fn the_graph_keeps_what_off_kept() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::manual());
        app.alarm_editor.set_drafts("1", "5");
        app.alarm_editor.band = "1%".to_string();
        app.apply_alarm_fields();
        app.set_limits(Limits::default(), Hysteresis::Auto);
        app.set_limit_from_graph(LimitSide::High, Some(4.0));
        let alarm = app.alarm.as_ref().expect("an alarm");
        assert_eq!(alarm.limits(), Limits::check(Some(1.0), Some(4.0)).unwrap());
        assert_eq!(alarm.hysteresis(), Hysteresis::Percent(1.0));
        assert_eq!(app.alarm_editor.low, "1");
        assert_eq!(app.alarm_editor.band, "1%");
    }

    /// A low at or above the high in force is refused with the row's words.
    #[test]
    fn the_graph_cannot_cross_the_limits() {
        let mut app = app_with_high_limit(5.0);
        app.set_limit_from_graph(LimitSide::Low, Some(6.0));
        assert_eq!(app.alarm.as_ref().unwrap().limits().low, None);
        let toast = app.toast.as_ref().expect("a toast");
        assert!(toast.is_error());
        assert!(
            toast.message.contains("low limit must be below"),
            "{}",
            toast.message
        );
    }

    /// An alarm idle on another quantity has limits in another unit: a limit
    /// set from the graph starts afresh, on the automatic band, and the row
    /// says so.
    #[test]
    fn the_graph_starts_afresh_over_an_idle_alarm() {
        let mut app = app_with_high_limit(5.0);
        assert!(app.check_alarm(&reading(&app, 4.0, "V")).is_none());
        assert!(app.check_alarm(&reading(&app, 12.0, "kΩ")).is_none());
        app.alarm_editor.band = "5%".to_string();
        app.set_limit_from_graph(LimitSide::Low, Some(2.0));
        let alarm = app.alarm.as_ref().expect("an alarm");
        assert_eq!(alarm.limits(), Limits::check(Some(2.0), None).unwrap());
        assert_eq!(alarm.hysteresis(), Hysteresis::Auto);
        assert_eq!(app.alarm_editor.band, "");
    }

    /// Off stops the alarm and the graph's lines with it.
    #[test]
    fn no_limits_turn_the_alarm_off() {
        let mut app = app_with_high_limit(5.0);
        assert_eq!(
            app.watched_limits().map(|w| w.limits.high),
            Some(Some(5.0)),
            "drawn before a reading binds it"
        );
        assert!(app.check_alarm(&reading(&app, 4.0, "V")).is_none());
        assert_eq!(app.watched_limits().map(|w| w.limits.high), Some(Some(5.0)));
        app.set_limits(Limits::default(), Hysteresis::Auto);
        assert!(app.alarm.is_none());
        assert!(app.watched_limits().is_none());
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Alarm off")
        );
    }
}
