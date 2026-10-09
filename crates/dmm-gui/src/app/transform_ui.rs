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
use eframe::egui::{self, RichText, Ui};

use super::App;
use super::appearance::SMALL_TEXT_SIZE;
use super::controls::remote::{SplitChip, row_field};
use super::toast::Toast;

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
    /// when the chip's body opens the row to have a scale typed.
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

/// Tooltip on the chip's caret.
const SCALE_HOVER: &str = "Set the scale";

/// The chip's name.
const SCALE_NAME: &str = "Scale";

/// Turn the three draft strings into a [`Transform`].
///
/// Empty means "leave this alone": no scale is ×1, no offset is +0, no unit
/// is no relabel. What counts as a usable factor is [`Transform`]'s rule, not
/// this row's — a zero scale is rejected rather than accepted as a flat line,
/// because it destroys the reading and is far more likely a half-typed "0.01"
/// than a deliberate choice.
fn parse_transform_fields(scale: &str, offset: &str, unit: &str) -> Result<Transform, String> {
    let scale = parse_field(scale, "scale", 1.0, Transform::check_scale)?;
    let offset = parse_field(offset, "offset", 0.0, Transform::check_offset)?;
    Ok(Transform::linear(scale, offset, Some(unit.to_string())))
}

/// Parse one numeric field and put it through `check`, naming the field in
/// any error so the toast says which box to go back to. `empty` is what a
/// blank field means, and is taken as-is: it is this row's own default, not
/// something the user typed.
fn parse_field(
    text: &str,
    name: &str,
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
        .map_err(|_| format!("Invalid {name}: \u{201c}{text}\u{201d} is not a number"))?;
    check(value).map_err(|e| match e {
        FactorError::NotFinite => format!("Invalid {name}: must be a finite number"),
        FactorError::ZeroScale => {
            format!("Invalid {name}: a scale of zero would erase the reading")
        }
    })
}

impl App {
    /// The **Scale** chip: filled while a scale is active. Styled like the
    /// remote buttons, which is why a caller sitting it next to them draws a
    /// rule in between: those mirror and drive the meter's own state, this
    /// one changes nothing on the meter at all.
    pub(super) fn scale_chip(&self) -> SplitChip<'static> {
        let active = !self.transform.is_identity();
        SplitChip {
            name: SCALE_NAME,
            active,
            ready: active || !self.transform_editor.is_blank(),
            open: (!self.meter_only()).then_some(self.transform_editor.open),
            hover: "Turn scaling on or off",
            fields_hover: SCALE_HOVER,
            unready_hover: self.chip_setup_hint(),
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

    /// Open the editor row if closed, closing the Alarm row so one row sits
    /// under the chips; close it if open.
    pub(super) fn toggle_transform_editor(&mut self) {
        self.transform_editor.open = !self.transform_editor.open;
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
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;
            let editor = &mut self.transform_editor;
            let mut field = |ui: &mut Ui, sign, text, hint, focus| {
                apply |= row_field(ui, &font, scale, sign, text, hint, focus).1;
            };
            field(ui, "\u{D7}", &mut editor.scale, "Scale factor", focus);
            field(ui, "+", &mut editor.offset, "Offset", false);
            field(ui, "\u{2192}", &mut editor.unit, "Unit label", false);
            apply |= ui
                .add(egui::Button::new(RichText::new("Apply").font(font.clone())))
                .on_hover_text("Apply these values to every reading from now on")
                .clicked();
            off |= ui
                .add(egui::Button::new(RichText::new("Off").font(font.clone())))
                .on_hover_text("Stop scaling and show the meter's own reading again")
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
        let mut message = if new.is_identity() {
            "Scaling off".to_string()
        } else {
            format!("Scaling readings: {}", new.describe())
        };
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
        if self.capture.recording.active {
            message.push_str(" \u{2014} recording continues with scaled values");
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
        if self.alarm.take().is_some() {
            message.push_str(" \u{2014} alarm off; set its limits again in the new unit");
        }
        self.toast = Some(Toast::info(message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controls::remote::SCALE_RULE_WIDTH;
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
        let settings = Settings {
            // No acquisition thread: the connected state is set by hand.
            auto_connect: false,
            ..Settings::default()
        };
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        app.connection.state = super::super::ConnectionState::Connected;
        app.connection.meter = Some(super::super::ConnectedMeter {
            supported_commands: [
                "hold", "rel", "range", "auto", "minmax", "peak", "select", "light",
            ]
            .map(String::from)
            .to_vec(),
            ..super::super::ConnectedMeter::test_fixture(None)
        });
        app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            "V",
            dmm_lib::flags::StatusFlags::default(),
        ));

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 600.0));
        let mut nodes: Vec<(egui::accesskit::NodeId, Node)> = Vec::new();
        for _ in 0..2 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        app.show_reading_column(ui, super::super::layout::ContentLayout::Narrow);
                    });
                },
            );
            // This harness renders without a painter.
            out.textures_delta.clear();
            nodes = out
                .platform_output
                .accesskit_update
                .map(|update| update.nodes)
                .unwrap_or_default();
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
        let t = parse_transform_fields("100", "", "A").expect("valid");
        assert_eq!(t, Transform::linear(100.0, 0.0, Some("A".to_string())));
        assert_eq!(t.describe(), "\u{D7}100 \u{2192} A");
    }

    #[test]
    fn an_empty_unit_field_means_no_relabel() {
        let t = parse_transform_fields("2", "0.5", "  ").expect("valid");
        assert_eq!(t, Transform::linear(2.0, 0.5, None));
        assert_eq!(t.unit, None);
    }

    /// Opening the row and pressing Apply with nothing typed must not turn
    /// scaling on.
    #[test]
    fn all_empty_fields_are_the_identity() {
        let t = parse_transform_fields("", "", "").expect("valid");
        assert!(t.is_identity());
    }

    #[test]
    fn a_zero_or_unparsable_scale_is_rejected_by_name() {
        for bad in ["0", "abc", "inf", "1,5"] {
            let err = parse_transform_fields(bad, "", "").expect_err("rejected");
            assert!(err.contains("scale"), "{bad:?} gave {err:?}");
        }
    }

    #[test]
    fn an_unparsable_offset_is_rejected_by_name() {
        let err = parse_transform_fields("", "abc", "").expect_err("rejected");
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

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(DmmMessage::Measurement(reading()))
            .expect("the channel is open");
        app.connection.rx = Some(rx);
        app.drain_messages();
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
        let t = parse_transform_fields("-1", "", "").expect("valid");
        assert_eq!(t, Transform::linear(-1.0, 0.0, None));
    }

    /// Limits typed against volts must not start judging a clamp's amps:
    /// a new scale turns the alarm off, and says so.
    #[test]
    fn a_new_scale_turns_the_alarm_off() {
        let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
        let limits = dmm_lib::alarm::Limits::check(None, Some(5.0)).unwrap();
        app.set_limits(limits, dmm_lib::alarm::Hysteresis::Auto);
        app.set_transform(Transform::linear(100.0, 0.0, Some("A".to_string())));
        assert!(app.alarm.is_none());
        assert!(app.watched_limits().is_none());
        let toast = app.toast.as_ref().expect("a toast");
        assert!(toast.message.contains("alarm off"), "{}", toast.message);
    }
}
