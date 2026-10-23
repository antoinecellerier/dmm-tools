//! The plot's right-click menu: what can be done at the point clicked —
//! mark or copy its reading, draw a reference line or set an alarm limit at
//! its height, put a cursor on it — and the removal of what lies under the
//! pointer: a marker, a reference line, a limit.

use eframe::egui;
use egui_plot::{PlotPoint, PlotTransform};
use std::time::Instant;

use super::time::format_pointer_time;
use super::{Cursor, Graph, LimitSide};
use crate::a11y::ResponseA11yExt;
use crate::markers::Marker;

/// Why the reading's entries are greyed.
const NO_READING: &str = "No reading in view";

/// How far the focus ring reaches past an entry: see
/// [`crate::a11y::paint_focus_ring`].
const FOCUS_RING_REACH: f32 = 4.0;

/// How near the pointer, in screen pixels, a line must pass for the menu to
/// offer its removal.
const NEAR_PX: f32 = 6.0;

/// The plot's right-click menu. One plot, so one id, known before the plot
/// is drawn.
pub(super) fn plot_menu_id() -> egui::Id {
    egui::Id::unique("main_plot_menu")
}

/// The digits the meter showed for its reading taken at an instant, of the
/// sub-value of that label or of the main reading on `None`; `None` where
/// the app no longer holds the reading. The app's, which keeps the readings
/// in full: the graph keeps only their values.
pub(crate) type ReadingDigits<'a> = &'a dyn Fn(Instant, Option<&str>) -> Option<String>;

/// The reading the menu's entries act on, and how they name it.
#[derive(Debug, Clone, PartialEq)]
struct MenuReading {
    at: Instant,
    /// Its value on the trace.
    value: f64,
    /// Its time on the plot, as the crosshair writes it. A name for the
    /// reading only: a marker or cursor goes on `at`, exactly.
    time: String,
    /// What "Copy" puts on the clipboard: the meter's digits, else the
    /// trace's value as the crosshair writes it.
    digits: String,
}

/// What the plot's right-click menu was opened on, fixed as it opens: by
/// the time an entry is picked the pointer is on the menu, and a live view
/// has moved.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MenuPoint {
    /// The reading nearest the right-click among those in view: what the
    /// menu marks, copies and puts a cursor on.
    reading: Option<MenuReading>,
    /// The height right-clicked, in the plotted unit, rounded to what a
    /// pixel tells apart: where "Add reference line" draws.
    y: f64,
    /// The reference line under the pointer.
    ref_line: Option<f64>,
    /// The number of the marker whose line is under the pointer.
    marker: Option<u32>,
    /// The cursor whose line is under the pointer.
    cursor: Option<Cursor>,
    /// Whether the alarm's limits can be set here: the alarm judges the
    /// main reading, so not while a sub-value is plotted, nor on a level.
    limits_apply: bool,
    /// The alarm limit whose line is under the pointer.
    limit: Option<LimitSide>,
    /// The history the menu was opened over. Once restarted — a clear, a
    /// unit's decade step, another series plotted — the times and heights
    /// above are another trace's, and the menu closes unpicked.
    epoch: u64,
}

/// What an entry of the plot's menu asks of the app, which holds what it
/// acts on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MenuAction {
    /// Mark the reading taken then, of that value on the trace.
    Mark(Instant, f64),
    /// Put that text, a reading's digits, on the clipboard.
    Copy(String),
    /// Delete the marker of that number.
    DeleteMarker(u32),
    /// Set that alarm limit at that height, in the plotted unit.
    SetLimit(LimitSide, f64),
    /// Remove that alarm limit.
    ClearLimit(LimitSide),
}

/// An entry picked, applied once the menu is drawn.
#[derive(Debug, Clone, Copy)]
enum Pick {
    Mark,
    Copy,
    DeleteMarker(u32),
    AddRefLine,
    RemoveRefLine(f64),
    SetCursor(Cursor),
    RemoveCursor(Cursor),
    RemoveCursors,
    SetLimit(LimitSide),
    ClearLimit(LimitSide),
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

/// Of `lines`, each with its distance from the pointer in screen pixels,
/// the nearest, if near enough to be the one under it.
fn under_pointer<T>(lines: impl Iterator<Item = (T, f32)>) -> Option<T> {
    lines
        .filter(|&(_, distance)| distance <= NEAR_PX)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(line, _)| line)
}

/// The entries putting a cursor on the menu's reading — each a label, the
/// word joining it to the reading's name, and the cursor it sets: one for
/// the first cursor, then one placing the missing cursor and one moving
/// each placed one not already on the reading at `reading_secs`. Named by
/// what they do, so a second pick of the same
/// entry can't move the cursor just placed where a pair was meant.
fn cursor_entries(
    cursors: super::MeasureCursors,
    reading_secs: Option<f64>,
) -> Vec<(String, &'static str, Cursor)> {
    let placed = [Cursor::A, Cursor::B].map(|c| (c, cursors.at(c)));
    if placed.iter().all(|(_, t)| t.is_none()) {
        return vec![("Put a cursor".to_string(), "on", Cursor::A)];
    }
    let put = placed
        .iter()
        .filter(|(_, t)| t.is_none())
        .map(|&(c, _)| (format!("Put cursor {}", c.name()), "on", c));
    let moves = placed
        .iter()
        // Not the cursor already on the reading: it would stay put.
        .filter(|(_, t)| t.is_some() && *t != reading_secs)
        .map(|&(c, _)| (format!("Move cursor {}", c.name()), "to", c));
    put.chain(moves).collect()
}

/// A line naming what the entries under it act on: `caption` muted and
/// `value` in the text colour, both small, so it reads apart from an entry,
/// greyed or not. Spaced from the first entry by the focus ring's reach,
/// which would otherwise cover its lower edge.
fn heading(ui: &mut egui::Ui, caption: &str, value: Option<&str>) {
    let style = ui.style().clone();
    let mut job = egui::text::LayoutJob::default();
    let mut part = |text: egui::RichText| {
        text.small().append_to(
            &mut job,
            &style,
            egui::FontSelection::Default,
            egui::Align::Center,
        );
    };
    part(egui::RichText::new(caption).color(ui.visuals().weak_text_color()));
    if let Some(value) = value {
        part(egui::RichText::new(format!(" {value}")).strong());
    }
    ui.label(job);
    ui.add_space(FOCUS_RING_REACH);
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
    /// What the menu opened at `pos` offers, over the markers `in_view` and
    /// the alarm's `limit_lines`, naming its reading with `digits`.
    fn menu_point(
        &self,
        pos: egui::Pos2,
        transform: &PlotTransform,
        in_view: &[(f64, &Marker)],
        limit_lines: &[(f64, LimitSide)],
        digits: ReadingDigits,
    ) -> MenuPoint {
        let at = transform.value_from_position(pos);
        let units_per_px =
            (at.y - transform.value_from_position(pos + egui::vec2(0.0, 1.0)).y).abs();
        let screen_y = |v: f64| transform.position_from_point(&PlotPoint::new(at.x, v)).y;
        let ref_values = self.show_ref_line.then(|| self.ref_lines.values());
        let ref_line = under_pointer(
            ref_values
                .into_iter()
                .flatten()
                .map(|&v| (v, (screen_y(v) - pos.y).abs())),
        );
        let limit = under_pointer(
            limit_lines
                .iter()
                .map(|&(v, side)| (side, (screen_y(v) - pos.y).abs())),
        );
        let screen_x = |t: f64| transform.position_from_point(&PlotPoint::new(t, at.y)).x;
        let marker = under_pointer(
            in_view
                .iter()
                .map(|&(t, m)| (m.number, (screen_x(t) - pos.x).abs())),
        );
        let cursor = under_pointer([Cursor::A, Cursor::B].into_iter().filter_map(|c| {
            let t = self.cursors.at(c)?;
            Some((c, (screen_x(t) - pos.x).abs()))
        }));
        // The series the history holds, which a sub-value just chosen
        // becomes only with its first reading.
        let plotted = self.current_series.as_deref();
        let reading = self
            .nearest_reading_in_view(at.x)
            .map(|(at, value)| MenuReading {
                at,
                value,
                time: format_pointer_time(self.elapsed_secs(at)),
                digits: digits(at, plotted)
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| format!("{value:.prec$}", prec = self.decimals(true))),
            });
        MenuPoint {
            reading,
            y: round_to_pixel(at.y, units_per_px, self.levels),
            ref_line,
            marker,
            cursor,
            limits_apply: !self.levels && self.current_series.is_none(),
            limit,
            epoch: self.history_epoch,
        }
    }

    /// The plot's right-click menu, in groups: the reading, the reference
    /// lines, the cursors, the alarm. What each entry acts on is fixed as
    /// the menu opens ([`MenuPoint`]).
    ///
    /// Keyboard handling follows the Export… menu: the first entry that can
    /// be picked takes the focus as the menu opens, the arrows move it, and
    /// Tab or Esc close the menu. The focus then goes nowhere rather than to
    /// the plot, where it would turn the graph's keys off.
    pub(super) fn show_context_menu(
        &mut self,
        plot: &egui::Response,
        transform: &PlotTransform,
        in_view: &[(f64, &Marker)],
        limit_lines: &[(f64, LimitSide)],
        digits: ReadingDigits,
    ) {
        let ctx = plot.ctx.clone();
        if plot.secondary_clicked()
            && let Some(pos) = plot.interact_pointer_pos()
        {
            self.menu = Some(self.menu_point(pos, transform, in_view, limit_lines, digits));
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
        let cursors = self.cursors;
        let reading_secs = menu.reading.as_ref().map(|r| self.elapsed_secs(r.at));
        // Asked again the frame after it opens: the right-click that opened
        // it is a click outside the entry, which surrenders its focus.
        let focus_again = std::mem::take(&mut self.menu_focus_pending);
        let mut entry_ids = Vec::new();
        let mut picked = None;
        egui::Popup::context_menu(plot).id(popup_id).show(|ui| {
            let reading = menu.reading.as_ref();
            let has_reading = reading.is_some();
            // Two groups, each under a muted heading naming what its entries
            // act on — the reading nearest the click, the height clicked —
            // so the entries are verbs alone. A screen reader walks the
            // entries, not the headings: each is named in full for it.
            let mut entries: Vec<(egui::Response, Pick)> = Vec::new();
            let mut entry = |ui: &mut egui::Ui, enabled, short: String, spoken: String, pick| {
                let response = ui
                    .add_enabled(enabled, egui::Button::new(short))
                    .on_disabled_hover_text(NO_READING)
                    .a11y_label(&spoken);
                entries.push((response, pick));
            };

            let the_reading = match reading {
                Some(r) => format!("the reading at {}", r.time),
                None => "a reading".to_string(),
            };
            match reading {
                Some(r) => heading(ui, "Reading at", Some(&r.time)),
                None => heading(ui, NO_READING, None),
            }
            entry(
                ui,
                has_reading,
                "Mark".to_string(),
                format!("Mark {the_reading}"),
                Pick::Mark,
            );
            let copy = reading.map_or("Copy".to_string(), |r| format!("Copy {}", r.digits));
            entry(ui, has_reading, copy.clone(), copy, Pick::Copy);
            if let Some(number) = menu.marker {
                let label = format!("Delete marker {number}");
                entry(ui, true, label.clone(), label, Pick::DeleteMarker(number));
            }
            for (short, preposition, which) in cursor_entries(cursors, reading_secs) {
                let spoken = format!("{short} {preposition} {the_reading}");
                entry(ui, has_reading, short, spoken, Pick::SetCursor(which));
            }
            if let Some(which) = menu.cursor {
                let label = format!("Remove cursor {}", which.name());
                entry(ui, true, label.clone(), label, Pick::RemoveCursor(which));
            }
            if cursors.placed().next().is_some() {
                let label = "Remove cursors".to_string();
                entry(ui, true, label.clone(), label, Pick::RemoveCursors);
            }

            ui.separator();
            let height = with_unit(menu.y, &unit);
            heading(ui, "At", Some(&height));
            // Not over a line at that very height, where it would add none.
            if menu.ref_line != Some(menu.y) {
                let spoken = format!("Add reference line at {height}");
                entry(
                    ui,
                    true,
                    "Add reference line".to_string(),
                    spoken,
                    Pick::AddRefLine,
                );
            }
            if let Some(v) = menu.ref_line {
                let label = format!("Remove reference line {}", with_unit(v, &unit));
                entry(ui, true, label.clone(), label, Pick::RemoveRefLine(v));
            }
            if menu.limits_apply {
                for side in [LimitSide::High, LimitSide::Low] {
                    let short = format!("Set alarm {}", side.name().to_lowercase());
                    let spoken = format!("{short} at {height}");
                    entry(ui, true, short, spoken, Pick::SetLimit(side));
                }
                if let Some(side) = menu.limit {
                    let label = format!("Remove alarm {}", side.name().to_lowercase());
                    entry(ui, true, label.clone(), label, Pick::ClearLimit(side));
                }
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

    /// Ask what an entry of the menu asks, as if picked.
    #[cfg(test)]
    pub(crate) fn request(&mut self, action: MenuAction) {
        self.menu_action = Some(action);
    }

    /// Do what the entry picked says, at the point the menu was opened on.
    fn apply_pick(&mut self, pick: Pick, menu: &MenuPoint) {
        let reading = menu.reading.as_ref();
        let reading_secs = reading.map(|r| self.elapsed_secs(r.at));
        match pick {
            Pick::Mark => self.menu_action = reading.map(|r| MenuAction::Mark(r.at, r.value)),
            Pick::Copy => self.menu_action = reading.map(|r| MenuAction::Copy(r.digits.clone())),
            Pick::DeleteMarker(number) => self.menu_action = Some(MenuAction::DeleteMarker(number)),
            Pick::AddRefLine => self.add_ref_line(menu.y),
            Pick::RemoveRefLine(v) => self.remove_ref_line(v),
            Pick::SetCursor(which) => {
                if let Some(t) = reading_secs {
                    self.cursors.set(which, t);
                }
            }
            Pick::RemoveCursor(which) => self.cursors.remove(which),
            // Off, as the chip leaves them: kept on, the next click on the
            // plot would place A again.
            Pick::RemoveCursors => self.cursors.toggle(),
            Pick::SetLimit(side) => self.menu_action = Some(MenuAction::SetLimit(side, menu.y)),
            Pick::ClearLimit(side) => self.menu_action = Some(MenuAction::ClearLimit(side)),
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
