//! The **Scale** button and editor: a session-only software transform over
//! the reading.
//!
//! A current clamp reads 10 mV/A, a pressure transducer reads V/PSI — the
//! meter knows nothing about either. This row lets the user say what the
//! reading really means, and [`dmm_lib::transform`] does the arithmetic in
//! base units so the factor survives an auto-range switch.
//!
//! Deliberately *not* persisted, for the same reason `Graph::selected_series`
//! isn't: a factor left in a settings file would silently corrupt the next
//! session's readings, which is far worse than retyping it. It does survive
//! disconnect/reconnect, a change of device and `Ctrl+L`, and the button is
//! always on screen so an active transform can always be turned back off.

use dmm_lib::transform::{FactorError, RAW_LABEL, Transform};
use dmm_shared::l10n::Localizer;
use dmm_shared::{input, tr};
use eframe::egui::{self, RichText, Ui};

use super::App;
use super::appearance::SMALL_TEXT_SIZE;
use super::controls::remote::{RowField, SplitChip, row_field};
use super::toast::Toast;
use crate::text_marks::MarkStyle;

/// Draft text for the three fields, kept apart from the applied
/// [`Transform`]: the row commits on Apply or Enter only, so a half-typed
/// "1e" or a lone "-" never reaches the reading — and never clears the
/// session the way an applied change does.
#[derive(Default)]
pub(super) struct TransformEditor {
    pub(super) open: bool,
    pub(super) scale: String,
    pub(super) offset: String,
    pub(super) unit: String,
    /// Put the caret in the scale field the next time the row is drawn: set
    /// when the chip opens it.
    focus: bool,
}

impl TransformEditor {
    /// Nothing typed in any field: no scale for the chip to switch on.
    fn is_blank(&self) -> bool {
        [&self.scale, &self.offset, &self.unit]
            .iter()
            .all(|f| f.trim().is_empty())
    }
}

/// Turn the three draft strings into a [`Transform`].
///
/// Empty means "leave this alone": no scale is ×1, no offset is +0, no unit
/// is no relabel. What counts as a usable factor is [`Transform`]'s rule, not
/// this row's — a zero scale is rejected rather than accepted as a flat line,
/// because it destroys the reading and is far more likely a half-typed "0.01"
/// than a deliberate choice.
fn parse_transform_fields(
    l10n: &Localizer,
    scale: &str,
    offset: &str,
    unit: &str,
) -> Result<Transform, String> {
    let scale = parse_field(l10n, scale, Field::Scale, 1.0, Transform::check_scale)?;
    let offset = parse_field(l10n, offset, Field::Offset, 0.0, Transform::check_offset)?;
    Ok(Transform::linear(scale, offset, Some(unit.to_string())))
}

/// The row's two numeric fields, for the toast that names one.
#[derive(Clone, Copy)]
enum Field {
    Scale,
    Offset,
}

impl Field {
    /// Why the field's text was refused, framed with the field's name.
    fn invalid(self, l10n: &Localizer, reason: std::borrow::Cow<'_, str>) -> String {
        match self {
            Field::Scale => tr!(l10n, "scale-invalid-factor", reason = reason),
            Field::Offset => tr!(l10n, "scale-invalid-offset", reason = reason),
        }
        .into_owned()
    }
}

/// Parse one numeric field and put it through `check`, naming the field in
/// any error so the toast says which box to go back to. `empty` is what a
/// blank field means, and is taken as-is: it is this row's own default, not
/// something the user typed.
fn parse_field(
    l10n: &Localizer,
    text: &str,
    field: Field,
    empty: f64,
    check: fn(f64) -> Result<f64, FactorError>,
) -> Result<f64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(empty);
    }
    // "inf" and "nan" parse, as do overflowing literals; `check` is what
    // turns them away — either one makes every reading a non-number with no
    // way to tell why.
    let value = text
        .parse::<f64>()
        .map_err(|_| field.invalid(l10n, input::not_a_number(l10n, text)))?;
    check(value).map_err(|e| field.invalid(l10n, input::factor_error(l10n, e)))
}

/// How the row marks a factor or offset draft `text` that doesn't read as
/// a number `check` takes ([`crate::text_marks`]).
fn number_mark(
    l10n: &Localizer,
    text: &str,
    check: fn(f64) -> Result<f64, FactorError>,
) -> Option<(MarkStyle, String)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let why = match text.parse::<f64>() {
        Err(_) => input::not_a_number(l10n, text),
        Ok(v) => input::factor_error(l10n, check(v).err()?),
    };
    Some((MarkStyle::Unread, why.into_owned()))
}

impl App {
    /// The **Scale** chip: filled while a scale is active. Styled like the
    /// remote buttons, which is why a caller sitting it next to them draws a
    /// rule in between: those mirror and drive the meter's own state, this
    /// one changes nothing on the meter at all.
    pub(super) fn scale_chip(&self, ctx: &egui::Context) -> SplitChip<'_> {
        let active = !self.transform.is_identity();
        SplitChip {
            name: tr!(self.l10n, "scale-chip"),
            active,
            ready: active || !self.transform_editor.is_blank(),
            open: (!self.meter_only()).then_some(self.transform_editor.open),
            hover: tr!(self.l10n, "scale-chip.hover"),
            fields_name: tr!(self.l10n, "scale-settings.a11y"),
            fields_hover: tr!(self.l10n, "scale-settings.hover"),
            unready_hover: self.chip_setup_hint(ctx),
        }
    }

    /// The chip's body: scaling off if on; else on with the fields' values,
    /// or with none typed the row opened to type them.
    pub(super) fn toggle_scale(&mut self) {
        if !self.transform.is_identity() {
            self.set_transform(Transform::default());
        } else if self.transform_editor.is_blank() {
            if !self.transform_editor.open {
                self.toggle_transform_editor();
            }
            self.transform_editor.focus = true;
        } else {
            self.apply_transform_fields();
        }
    }

    /// Apply what the fields hold, or say which to fix: the row's Apply, and
    /// the chip's body.
    fn apply_transform_fields(&mut self) {
        match parse_transform_fields(
            &self.l10n,
            &self.transform_editor.scale,
            &self.transform_editor.offset,
            &self.transform_editor.unit,
        ) {
            Ok(new) => self.set_transform(new),
            // Nothing is applied — the previous transform (identity or
            // not) keeps running, and the toast names the field to fix.
            Err(message) => self.toast = Some(Toast::error(message)),
        }
    }

    /// Open the editor row if closed, with the caret in the factor field and
    /// the Alarm row closed, so one row sits under the chips; close it if
    /// open: the chip's caret.
    pub(super) fn toggle_transform_editor(&mut self) {
        self.transform_editor.open = !self.transform_editor.open;
        self.transform_editor.focus = self.transform_editor.open;
        if self.transform_editor.open {
            self.alarm_editor.open = false;
        }
    }

    /// The `× [scale] + [offset] → [unit] [Apply] [Off]` row under the
    /// buttons, shown while the editor is open.
    pub(super) fn show_transform_editor(&mut self, ui: &mut Ui, scale: f32) {
        if !self.transform_editor.open {
            return;
        }
        // Floored at the 11 pt minimum: unlike the remote buttons this row
        // holds text the user has to type into, and the big meter's scale
        // factor goes below 0.4 in a small window.
        let font_size = (12.0 * scale).max(SMALL_TEXT_SIZE);
        let font = egui::FontId::proportional(font_size);

        // Collected rather than acted on inside the closure: the field
        // borrows of `transform_editor` are live in there, and applying
        // needs `&mut self`.
        let mut apply = false;
        let mut off = false;
        let focus = std::mem::take(&mut self.transform_editor.focus);
        let l10n = &self.l10n;
        let error = self
            .settings
            .theme_colors(ui.visuals().dark_mode)
            .status_error();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;
            let editor = &mut self.transform_editor;
            apply |= row_field(
                ui,
                &font,
                scale,
                &mut editor.scale,
                RowField {
                    caption: "\u{D7}",
                    hint: &tr!(l10n, "scale-factor"),
                    focus,
                    hover: None,
                    judge: &|t| number_mark(l10n, t, Transform::check_scale),
                    error,
                },
            )
            .1;
            apply |= row_field(
                ui,
                &font,
                scale,
                &mut editor.offset,
                RowField {
                    caption: "+",
                    hint: &tr!(l10n, "scale-offset"),
                    focus: false,
                    hover: None,
                    judge: &|t| number_mark(l10n, t, Transform::check_offset),
                    error,
                },
            )
            .1;
            apply |= row_field(
                ui,
                &font,
                scale,
                &mut editor.unit,
                RowField {
                    caption: "\u{2192}",
                    hint: &tr!(l10n, "scale-unit"),
                    focus: false,
                    hover: None,
                    judge: &|_| None,
                    error,
                },
            )
            .1;
            apply |= ui
                .add(egui::Button::new(
                    RichText::new(tr!(l10n, "scale-apply")).font(font.clone()),
                ))
                .on_hover_text(tr!(l10n, "scale-apply.hover"))
                .clicked();
            off |= ui
                .add(egui::Button::new(
                    RichText::new(tr!(l10n, "scale-off")).font(font.clone()),
                ))
                .on_hover_text(tr!(l10n, "scale-off.hover"))
                .clicked();
        });

        // Off keeps the fields, so Apply turns the same scale back on.
        if off {
            self.set_transform(Transform::default());
        } else if apply {
            self.apply_transform_fields();
        }
    }

    /// Install a transform, resetting everything derived from the old scale.
    ///
    /// A relabel moves `m.unit`, which the mode/unit check in
    /// `Capture::ingest` and `Graph::push_sample` both already answer by
    /// clearing. A pure scale or offset change does not, so the reset has to
    /// be explicit here — otherwise volt-scale statistics would carry into
    /// amp-scale readings. A recording is left alone, exactly as the Clear
    /// button leaves it; the history goes with the graph.
    fn set_transform(&mut self, new: Transform) {
        if new == self.transform {
            return;
        }
        let identity = new.is_identity();
        // Without a relabel the `Raw` sub-value shares the plotted unit, so
        // it is overlaid by default — and a ×100 companion squeezes the
        // scaled trace into a flat line against the auto-fit Y range. Start
        // it off; the **Show:** chip still lists it for anyone who wants the
        // comparison.
        if !new.is_identity() {
            self.graph.hide_overlay(RAW_LABEL);
        }
        self.transform = new;
        self.clear_session();
        let recording = self.capture.recording.is_active();
        if recording {
            // A scale switched on mid-capture still needs its trailing CSV
            // column. Only grows: switching back off leaves the group empty
            // rather than shifting every column the file already promised.
            self.capture.recording_layout.extra_slots = self
                .capture
                .recording_layout
                .extra_slots
                .max(self.transform.extra_aux_count());
        }
        // Limits typed against the old unit would judge the new one: a high
        // limit of 5 meant for volts becomes 5 A under a clamp scale. The
        // alarm stops instead, its fields kept for Apply.
        let alarm = self.alarm.take().is_some();
        self.alarm_bound_before = None;
        let also = match (recording, alarm) {
            (true, true) => "both",
            (true, false) => "recording",
            (false, true) => "alarm",
            (false, false) => "none",
        };
        let message = if identity {
            tr!(self.l10n, "scale-cleared", also = also)
        } else {
            let transform = self.transform.describe();
            tr!(self.l10n, "scale-on", transform = transform, also = also)
        };
        self.toast = Some(Toast::info(message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row marks a factor or offset that isn't a number, or one the
    /// scale refuses; blank is the default.
    #[test]
    fn the_row_marks_a_number_it_wont_use() {
        let l = crate::l10n::english();
        let mark = |text, check| super::number_mark(&l, text, check).map(|(_, why)| why);
        assert_eq!(mark(" ", Transform::check_scale), None);
        assert_eq!(mark("100", Transform::check_scale), None);
        assert_eq!(
            mark("abc", Transform::check_scale).as_deref(),
            Some("\u{201c}abc\u{201d} is not a number")
        );
        assert!(mark("0", Transform::check_scale).is_some());
        assert_eq!(mark("0", Transform::check_offset), None);
    }
    use crate::app::controls::remote::SCALE_RULE_WIDTH;
    use crate::app::test_support::{a11y, connected_app, headless_frame, test_app};
    use crate::settings::Settings;
    use eframe::egui::accesskit::Node;
    use eframe::egui::{Pos2, Rect, vec2};

    /// Bounds of the meter's `LIGHT` chip and the `Scale` chip after laying
    /// the connected reading column out in a window `width` points wide. Two
    /// frames: a wrapped row is cut at the width the previous frame settled.
    fn chip_bounds(width: f32) -> (egui::accesskit::Rect, egui::accesskit::Rect) {
        let bounds = column_bounds(width);
        (bounds("LIGHT"), bounds("Scale"))
    }

    /// The bounds of each labelled widget in the connected reading column,
    /// laid out as [`chip_bounds`] says.
    fn column_bounds(width: f32) -> impl Fn(&str) -> egui::accesskit::Rect {
        let mut app = connected_app(
            &[
                "hold", "rel", "range", "auto", "minmax", "peak", "select", "light",
            ],
            "V",
        );
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 600.0));
        let mut nodes: Vec<(egui::accesskit::NodeId, Node)> = Vec::new();
        for _ in 0..2 {
            let out = headless_frame(&ctx, Some(screen), None, vec![], |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    app.show_reading_column(ui, super::super::layout::ContentLayout::Narrow);
                });
            });
            (nodes, _) = a11y(&out);
        }
        move |label: &str| {
            nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .and_then(|(_, n)| n.bounds())
                .unwrap_or_else(|| panic!("{label} chip is drawn"))
        }
    }

    /// A chip's halves sit a hairline apart, the chips themselves a button
    /// gap apart, as the meter's buttons are.
    #[test]
    fn a_split_chip_is_joined_and_spaced_from_the_next() {
        let bounds = column_bounds(900.0);
        let (scale, caret, alarm) = (bounds("Scale"), bounds("Scale settings"), bounds("Alarm"));
        let joint = caret.x0 - scale.x1;
        let between = alarm.x0 - caret.x1;
        assert!(joint < 2.0, "the halves join: {joint}");
        assert!(between > joint + 1.0, "the chips stand apart: {between}");
    }

    /// With room to spare the Scale chip ends the meter's button line, set
    /// off by a rule's width rather than a button gap.
    #[test]
    fn scale_chip_joins_the_button_line_when_it_fits() {
        let (light, scale) = chip_bounds(900.0);
        assert!(
            (light.y0 - scale.y0).abs() < 0.5,
            "Scale sits on the LIGHT line: LIGHT {light:?}, Scale {scale:?}"
        );
        let gap = scale.x0 - light.x1;
        assert!(
            gap > SCALE_RULE_WIDTH as f64,
            "a rule separates Scale from LIGHT (gap {gap})"
        );
    }

    /// With no room left on the button line the chip drops to a line of its
    /// own, as it always did, instead of running under the big-meter toggle.
    #[test]
    fn scale_chip_takes_its_own_line_when_the_button_line_is_full() {
        let (light, scale) = chip_bounds(420.0);
        assert!(
            scale.y0 >= light.y1,
            "Scale sits below LIGHT: LIGHT {light:?}, Scale {scale:?}"
        );
        // A fresh line starts at the margin, with no rule in front of it.
        assert!(scale.x0 < light.x0, "Scale starts its line at the margin");
    }

    /// The 10 mV/A clamp: ×100, no offset, relabelled to amps.
    #[test]
    fn parse_transform_fields_reads_a_clamp_factor() {
        let t = parse_transform_fields(&crate::l10n::english(), "100", "", "A").expect("valid");
        assert_eq!(t, Transform::linear(100.0, 0.0, Some("A".to_string())));
        assert_eq!(t.describe(), "\u{D7}100 \u{2192} A");
    }

    #[test]
    fn an_empty_unit_field_means_no_relabel() {
        let t = parse_transform_fields(&crate::l10n::english(), "2", "0.5", "  ").expect("valid");
        assert_eq!(t, Transform::linear(2.0, 0.5, None));
        assert_eq!(t.unit, None);
    }

    /// Opening the row and pressing Apply with nothing typed must not turn
    /// scaling on.
    #[test]
    fn all_empty_fields_are_the_identity() {
        let t = parse_transform_fields(&crate::l10n::english(), "", "", "").expect("valid");
        assert!(t.is_identity());
    }

    #[test]
    fn a_zero_or_unparsable_scale_is_rejected_by_name() {
        for bad in ["0", "abc", "inf", "1,5"] {
            let err =
                parse_transform_fields(&crate::l10n::english(), bad, "", "").expect_err("rejected");
            assert!(err.contains("scale"), "{bad:?} gave {err:?}");
        }
    }

    #[test]
    fn an_unparsable_offset_is_rejected_by_name() {
        let err =
            parse_transform_fields(&crate::l10n::english(), "", "abc", "").expect_err("rejected");
        assert!(err.contains("offset"), "{err:?}");
    }

    /// A scale change drops the history with the graph, so the history that
    /// follows is all scaled readings, each with the `Raw` column the file
    /// reserves for it.
    #[test]
    fn a_scale_change_restarts_the_history_with_its_raw_column() {
        use super::super::connection::DmmMessage;
        let mut app = App::from_settings(
            Settings {
                auto_connect: false,
                ..Settings::default()
            },
            dmm_lib::Clock::real(),
        );
        let reading = || {
            dmm_lib::measurement::Measurement::test_fixture(
                dmm_lib::measurement::MeasuredValue::Normal(1.234),
                "V",
                dmm_lib::flags::StatusFlags::default(),
            )
        };
        app.capture.recording.push(&reading(), 0);

        app.set_transform(Transform::linear(2.0, 0.0, None));
        assert_eq!(app.capture.recording.export_samples().len(), 0);

        app.deliver(DmmMessage::Measurement(reading()));
        assert_eq!(app.capture.recording.export_samples().len(), 1);
        assert_eq!(
            app.capture
                .recording
                .export_samples()
                .next()
                .map(|s| s.extra_aux),
            Some(1)
        );
        assert_eq!(app.capture.history_layout.extra_slots, 1);
    }

    /// A negative scale is a legitimate probe polarity flip, not an error.
    #[test]
    fn a_negative_scale_is_accepted() {
        let t = parse_transform_fields(&crate::l10n::english(), "-1", "", "").expect("valid");
        assert_eq!(t, Transform::linear(-1.0, 0.0, None));
    }

    /// Limits typed against volts must not start judging a clamp's amps:
    /// a new scale turns the alarm off, and says so.
    #[test]
    fn a_new_scale_turns_the_alarm_off() {
        let mut app = test_app();
        let limits = dmm_lib::alarm::Limits::check(None, Some(5.0)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        app.set_transform(Transform::linear(100.0, 0.0, Some("A".to_string())));
        assert!(app.alarm.is_none());
        assert!(app.watched_limits().is_none());
        let toast = app.toast.as_ref().expect("a toast");
        assert_eq!(
            toast.message,
            "Scaling readings: \u{D7}100 \u{2192} A \u{2014} alarm off; set its limits again in the new unit"
        );
    }

    /// The toast says each thing the change brings with it, in one
    /// sentence: a recording going on, and the alarm turned off.
    #[test]
    fn the_scale_toast_says_what_came_with_it() {
        let mut app = test_app();
        app.capture.recording.toggle(app.clock.now());
        app.set_transform(Transform::linear(2.0, 0.5, None));
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Scaling readings: \u{D7}2 + 0.5 \u{2014} recording continues with scaled values")
        );
        let limits = dmm_lib::alarm::Limits::check(None, Some(5.0)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        app.set_transform(Transform::default());
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some(
                "Scaling off \u{2014} recording continues with unscaled values \u{2014} alarm off; \
                 set its limits again in the new unit"
            )
        );
        app.capture.recording.toggle(app.clock.now());
        app.set_transform(Transform::linear(3.0, 0.0, None));
        app.set_transform(Transform::default());
        assert_eq!(
            app.toast.as_ref().map(|t| t.message.as_str()),
            Some("Scaling off")
        );
    }
}
