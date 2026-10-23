//! The plot's right-click menu: what can be done at the point clicked —
//! mark its reading, draw a reference line at its height, put a cursor on
//! it — and the removal of what lies under the pointer.

use eframe::egui;
use egui_plot::{PlotPoint, PlotTransform};
use std::time::Instant;

use super::Graph;

/// How near the pointer, in screen pixels, a line must pass for the menu to
/// offer its removal.
const NEAR_PX: f32 = 6.0;

/// The plot's right-click menu. One plot, so one id, known before the plot
/// is drawn.
pub(super) fn plot_menu_id() -> egui::Id {
    egui::Id::unique("main_plot_menu")
}

/// What the plot's right-click menu was opened on, fixed as it opens: by
/// the time an entry is picked the pointer is on the menu, and a live view
/// has moved.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MenuPoint {
    /// The reading nearest the right-click among those in view, with its
    /// value: what "Add marker here" marks and a cursor goes to.
    reading: Option<(Instant, f64)>,
    /// The height right-clicked, in the plotted unit, rounded to what a
    /// pixel tells apart: where "Add reference line" draws.
    y: f64,
    /// The reference line under the pointer.
    ref_line: Option<f64>,
    /// The history the menu was opened over. Once restarted — a clear, a
    /// unit's decade step, another series plotted — the times and heights
    /// above are another trace's, and the menu closes unpicked.
    epoch: u64,
}

/// What an entry of the plot's menu asks of the app, which holds what it
/// acts on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MenuAction {
    /// Mark the reading taken then, of that value on the trace.
    Mark(Instant, f64),
}

/// An entry picked, applied once the menu is drawn.
#[derive(Debug, Clone, Copy)]
enum Pick {
    Mark,
    AddRefLine,
    RemoveRefLine(f64),
    CursorA,
    CursorB,
    RemoveCursors,
}

/// `y` rounded to the coarsest decimal step finer than `units_per_px`, the
/// height a pixel spans, so a line lands where it was asked for and its value
/// reads as typed: 3.29 on a 10 V span 300 px high. A level's are whole.
///
/// Through text at the end: the reference field writes its values with
/// `to_string`, which would show the rounding's last-bit noise
/// (`3.2900000000000005`).
pub(super) fn round_to_pixel(y: f64, units_per_px: f64, levels: bool) -> f64 {
    if levels {
        return y.round() + 0.0;
    }
    if !(units_per_px.is_finite() && units_per_px > 0.0) {
        return y;
    }
    let exponent = units_per_px.log10().floor() as i32;
    let decimals = usize::try_from(-exponent).unwrap_or(0);
    let step = 10f64.powi(exponent);
    let rounded = (y / step).round() * step;
    // `+ 0.0` turns a -0 into 0, which would otherwise read "-0".
    format!("{rounded:.decimals$}")
        .parse::<f64>()
        .map_or(rounded, |v| v + 0.0)
}

/// `v` with `unit` after it, as the menu's entries name a height.
fn with_unit(v: f64, unit: &str) -> String {
    if unit.is_empty() {
        v.to_string()
    } else {
        format!("{v} {unit}")
    }
}

impl Graph {
    /// What the menu opened at `pos` offers.
    fn menu_point(&self, pos: egui::Pos2, transform: &PlotTransform) -> MenuPoint {
        let at = transform.value_from_position(pos);
        let units_per_px =
            (at.y - transform.value_from_position(pos + egui::vec2(0.0, 1.0)).y).abs();
        let screen_y = |v: f64| transform.position_from_point(&PlotPoint::new(at.x, v)).y;
        let ref_line = self
            .show_ref_line
            .then(|| self.ref_lines.values())
            .into_iter()
            .flatten()
            .map(|&v| (v, (screen_y(v) - pos.y).abs()))
            .filter(|&(_, distance)| distance <= NEAR_PX)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(v, _)| v);
        MenuPoint {
            reading: self.nearest_reading_in_view(at.x),
            y: round_to_pixel(at.y, units_per_px, self.levels),
            ref_line,
            epoch: self.history_epoch,
        }
    }

    /// The plot's right-click menu, in groups: the reading, the reference
    /// lines, the cursors. What each entry acts on is fixed as the menu
    /// opens ([`MenuPoint`]).
    ///
    /// Keyboard handling follows the Export… menu: the first entry that can
    /// be picked takes the focus as the menu opens, the arrows move it, and
    /// Tab or Esc close the menu. The focus then goes nowhere rather than to
    /// the plot, where it would turn the graph's keys off.
    pub(super) fn show_context_menu(&mut self, plot: &egui::Response, transform: &PlotTransform) {
        let ctx = plot.ctx.clone();
        if plot.secondary_clicked()
            && let Some(pos) = plot.interact_pointer_pos()
        {
            self.menu = Some(self.menu_point(pos, transform));
        }
        let popup_id = plot_menu_id();
        let was_open = egui::Popup::is_id_open(&ctx, popup_id);
        if was_open {
            let stale = self
                .menu
                .as_ref()
                .is_none_or(|m| m.epoch != self.history_epoch);
            let leave = ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                    | i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)
                    | i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            });
            if leave || stale {
                egui::Popup::close_id(&ctx, popup_id);
                ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }
        let Some(menu) = self.menu.clone() else {
            return;
        };
        let unit = self.plotted_unit().to_string();
        let cursors_on = self.cursors.active;
        // Asked again the frame after it opens: the right-click that opened
        // it is a click outside the entry, which surrenders its focus.
        let focus_again = std::mem::take(&mut self.menu_focus_pending);
        let mut entry_ids = Vec::new();
        let mut picked = None;
        egui::Popup::context_menu(plot).id(popup_id).show(|ui| {
            let has_reading = menu.reading.is_some();
            let mut entries: Vec<(egui::Response, Pick)> = Vec::new();
            entries.push((
                ui.add_enabled(has_reading, egui::Button::new("Add marker here"))
                    .on_disabled_hover_text("No reading in view to mark"),
                Pick::Mark,
            ));
            ui.separator();
            entries.push((
                ui.button(format!(
                    "Add reference line at {}",
                    with_unit(menu.y, &unit)
                )),
                Pick::AddRefLine,
            ));
            if let Some(v) = menu.ref_line {
                entries.push((
                    ui.button(format!("Remove reference line at {}", with_unit(v, &unit))),
                    Pick::RemoveRefLine(v),
                ));
            }
            ui.separator();
            for (label, pick) in [
                ("Place cursor A here", Pick::CursorA),
                ("Place cursor B here", Pick::CursorB),
            ] {
                entries.push((
                    ui.add_enabled(has_reading, egui::Button::new(label))
                        .on_disabled_hover_text("No reading in view to place it on"),
                    pick,
                ));
            }
            if cursors_on {
                entries.push((ui.button("Remove cursors"), Pick::RemoveCursors));
            }

            let enabled: Vec<egui::Response> = entries
                .iter()
                .filter(|(entry, _)| entry.enabled())
                .map(|(entry, _)| entry.clone())
                .collect();
            if (!was_open || focus_again)
                && let Some(first) = enabled.first()
            {
                first.request_focus();
            }
            for (entry, pick) in &entries {
                // Menu entries have no frame, so egui's focus styling is a
                // fill too faint to find; ring the focused one.
                crate::a11y::paint_focus_ring(ui, entry);
                if was_open && entry.clicked() {
                    picked = Some(*pick);
                }
                entry_ids.push(entry.id);
            }
            // A disabled entry cannot take the focus, so it is not a stop
            // for the arrows either: Down would strand the focus on it.
            crate::display::navigate_choice_entries(ui.ctx(), &enabled);
        });
        if let Some(pick) = picked {
            // Enter and Space click without a pointer click, which is the
            // only thing a menu closes on by itself.
            egui::Popup::close_id(&ctx, popup_id);
            self.apply_pick(pick, &menu);
        }
        let is_open = egui::Popup::is_id_open(&ctx, popup_id);
        if !was_open && is_open {
            self.menu_focus_pending = true;
        }
        if was_open && !is_open {
            ctx.memory_mut(|m| {
                for id in entry_ids {
                    m.surrender_focus(id);
                }
            });
        }
        if !is_open {
            self.menu = None;
        }
    }

    /// Do what the entry picked says, at the point the menu was opened on.
    fn apply_pick(&mut self, pick: Pick, menu: &MenuPoint) {
        let reading_secs = menu.reading.map(|(at, _)| self.elapsed_secs(at));
        match pick {
            Pick::Mark => {
                self.menu_action = menu.reading.map(|(at, v)| MenuAction::Mark(at, v));
            }
            Pick::AddRefLine => self.add_ref_line(menu.y),
            Pick::RemoveRefLine(v) => self.remove_ref_line(v),
            Pick::CursorA => {
                if let Some(t) = reading_secs {
                    self.cursors.place_a(t);
                }
            }
            Pick::CursorB => {
                if let Some(t) = reading_secs {
                    self.cursors.place_b(t);
                }
            }
            // Off, as the chip leaves them: kept on, the next click on the
            // plot would place A again.
            Pick::RemoveCursors => self.cursors.toggle(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_round_to_what_a_pixel_tells_apart() {
        // 10 V over 300 px: a pixel is 0.033 V, so hundredths.
        assert_eq!(round_to_pixel(3.287_163, 10.0 / 300.0, false), 3.29);
        // 10 kΩ over 300 px: tens of ohms.
        assert_eq!(round_to_pixel(4_321.7, 10_000.0 / 300.0, false), 4_320.0);
        assert_eq!(round_to_pixel(-0.001, 0.05, false).to_string(), "0");
        assert_eq!(round_to_pixel(2.6, 0.01, true), 3.0);
        assert_eq!(round_to_pixel(1.5, 0.0, false), 1.5, "a flat axis");
        assert_eq!(round_to_pixel(1.5, f64::NAN, false), 1.5);
    }

    /// The field writes what the rounding gives with `to_string`: no
    /// last-bit noise may reach it.
    #[test]
    fn rounded_heights_write_as_typed() {
        for i in 0..1000 {
            let y = f64::from(i) * 0.0137 - 5.0;
            let text = round_to_pixel(y, 0.004, false).to_string();
            assert!(
                text.split('.').nth(1).is_none_or(|d| d.len() <= 3),
                "{y} rounded to {text}"
            );
        }
    }
}
