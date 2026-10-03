use dmm_lib::mock::MockMode;
use dmm_lib::protocol::registry;
use dmm_shared::help::BLUETOOTH_SETTING;
use eframe::egui::{self, RichText, Ui};

use crate::a11y::ResponseA11yExt;
use crate::settings::{
    ColorPreset, GraphLines, HexColor, PaletteOverrides, SpecFields, ThemeChoice, ThemeMode,
    buffer_memory_estimate, format_sample_count,
};
use crate::theme::links;
use crate::theme::named;
use crate::theme::{PaletteField, ThemeColors};

use super::appearance::ALWAYS_ON_TOP_WAYLAND_HINT;
use super::{App, BigMeterMode};

/// Show a settings checkbox with a hover tooltip; returns `true` if the value changed.
fn setting_checkbox(ui: &mut Ui, value: &mut bool, label: &str, tooltip: &str) -> bool {
    ui.checkbox(value, label).on_hover_text(tooltip).changed()
}

/// The **Specifications** checkbox and, while it is on, the fields the
/// panel shows after it. The group is one unit, sized before it is
/// placed so the wrapped row moves it to the next line whole, and it wraps
/// within itself only when a whole line is too narrow for it (`color_edit`
/// says why a `ui.horizontal` can't do this). Returns whether the panel
/// checkbox changed, and whether a field did.
fn specs_checkboxes(ui: &mut Ui, show: &mut bool, fields: &mut SpecFields) -> (bool, bool) {
    // Read before a click can flip it, so the frame draws what it measured.
    let on = *show;
    let panel_label = if on {
        "Specifications:"
    } else {
        "Specifications"
    };
    let SpecFields {
        resolution,
        accuracy,
        input_impedance,
        notes,
    } = fields;
    let mut field_boxes = [
        (
            resolution,
            "Resolution",
            "Show the resolution in the specifications",
        ),
        (
            accuracy,
            "Accuracy",
            "Show the accuracy in the specifications",
        ),
        (
            input_impedance,
            "Input Z",
            "Show the input impedance in the full Specifications panel",
        ),
        (
            notes,
            "Notes",
            "Show the manual's notes in the full Specifications panel",
        ),
    ];
    let shown = if on { field_boxes.len() } else { 0 };

    // A checkbox as egui lays it out: the box, the icon gap, the label.
    let checkbox_width = |label: &str| {
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        );
        ui.spacing().icon_width + ui.spacing().icon_spacing + galley.size().x
    };
    let width = std::iter::once(panel_label)
        .chain(field_boxes[..shown].iter().map(|(_, label, _)| *label))
        .map(checkbox_width)
        .sum::<f32>()
        + ui.spacing().item_spacing.x * shown as f32;
    let size = egui::vec2(
        width.min(ui.max_rect().width()),
        ui.spacing().interact_size.y,
    );
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
        |ui| {
            let panel_changed = setting_checkbox(
                ui,
                show,
                panel_label,
                "Show accuracy and resolution for the current mode",
            );
            let mut fields_changed = false;
            for (value, label, tooltip) in &mut field_boxes[..shown] {
                fields_changed |= setting_checkbox(ui, value, label, tooltip);
            }
            (panel_changed, fields_changed)
        },
    )
    .inner
}

/// Show a selectable_label with a hover tooltip; returns `true` if clicked.
fn setting_selectable(
    ui: &mut Ui,
    selected: bool,
    label: impl Into<egui::WidgetText>,
    tooltip: &str,
) -> bool {
    ui.selectable_label(selected, label)
        .on_hover_text(tooltip)
        .clicked()
}

/// One chip in a settings row: the setting it stands for, what it says, what
/// it says on hover, and whether it is the current selection.
struct Chip<T> {
    value: T,
    selected: bool,
    label: String,
    tooltip: String,
}

/// A caption followed by a run of selectable chips, as every settings row in
/// the panel draws them. Returns the value of the chip clicked this frame.
///
/// The caller keeps its own layout container and its own trailing hint: the
/// rows differ in whether they wrap and in what they add after the chips.
fn chip_row<T>(ui: &mut Ui, caption: &str, chips: impl IntoIterator<Item = Chip<T>>) -> Option<T> {
    ui.label(caption);
    let mut picked = None;
    for chip in chips {
        if setting_selectable(ui, chip.selected, chip.label, &chip.tooltip) {
            picked = Some(chip.value);
        }
    }
    picked
}

/// The most columns the **Device** list spreads over: at four the tallest
/// is UNI-T's, and more would only leave short columns side by side.
const MAX_DEVICE_COLUMNS: usize = 4;

/// The fewest rows the **Device** list keeps below its button before it
/// lets egui slide it up over the row instead.
const MIN_DEVICE_LIST_ROWS: f32 = 8.0;

/// Splits groups `heights` tall, kept whole and in order, into at most `k`
/// columns, the tallest as short as it can be. Returns each column's range
/// of groups; a tie goes to the split whose earlier columns are taller, so
/// the eye reads down the long ones first.
fn pack_columns(heights: &[usize], k: usize) -> Vec<std::ops::Range<usize>> {
    /// Every way to cut `n` groups into `k` non-empty runs, as the index
    /// each run after the first starts at.
    fn cuts(from: usize, n: usize, k: usize, acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if k == 1 {
            out.push(acc.clone());
            return;
        }
        for cut in from + 1..=n - (k - 1) {
            acc.push(cut);
            cuts(cut, n, k - 1, acc, out);
            acc.pop();
        }
    }
    let n = heights.len();
    let k = k.clamp(1, n.max(1));
    let mut splits = Vec::new();
    cuts(0, n, k, &mut Vec::new(), &mut splits);
    let ranges = |cuts: &[usize]| -> Vec<std::ops::Range<usize>> {
        let starts = std::iter::once(0).chain(cuts.iter().copied());
        let ends = cuts.iter().copied().chain(std::iter::once(n));
        starts.zip(ends).map(|(a, b)| a..b).collect()
    };
    let tallest = |ranges: &[std::ops::Range<usize>]| {
        ranges
            .iter()
            .map(|r| heights[r.clone()].iter().sum::<usize>())
            .max()
            .unwrap_or(0)
    };
    splits
        .iter()
        .map(|cuts| ranges(cuts))
        // Reversed, the latest cuts come first, and `min_by_key` keeps the
        // first of equals: the taller columns lead.
        .rev()
        .min_by_key(|ranges| tallest(ranges))
        .unwrap_or_default()
}

/// How the open **Device** list lays out: the brand groups and the columns
/// they are spread over, as many as the window has room for.
struct DeviceListLayout {
    groups: Vec<(registry::Brand, Vec<&'static registry::SelectableDevice>)>,
    columns: Vec<std::ops::Range<usize>>,
    col_w: f32,
    gap: f32,
}

impl DeviceListLayout {
    fn new(ctx: &egui::Context, style: &egui::Style) -> Self {
        let groups = registry::brand_groups();
        // Column width: the widest heading or marked entry, plus a
        // selectable's padding. Measured in the font the list draws in.
        let font = egui::TextStyle::Button.resolve(style);
        let measure = |text: &str| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        };
        let padding = 2.0 * style.spacing.button_padding.x;
        let col_w = groups
            .iter()
            .flat_map(|(brand, devices)| {
                let entries = devices.iter().map(|d| {
                    measure(&crate::display::choice_entry_text(d.model_name(), true)) + padding
                });
                std::iter::once(measure(brand.name())).chain(entries)
            })
            .fold(0.0, f32::max)
            .ceil();
        let gap = style.spacing.item_spacing.x * 3.0;
        // The menu's frame and the room egui keeps from the window's edge.
        let margins = 4.0 * style.spacing.menu_margin.sum().x;
        let k = (((ctx.content_rect().width() - margins + gap) / (col_w + gap)).floor() as usize)
            .clamp(1, MAX_DEVICE_COLUMNS);
        let heights: Vec<usize> = groups.iter().map(|(_, d)| d.len() + 1).collect();
        let columns = pack_columns(&heights, k);
        Self {
            groups,
            columns,
            col_w,
            gap,
        }
    }
}

/// The **Device** dropdown: the selected meter on the box, and in the open
/// list Auto-detect over every meter under its brand, the brands spread over
/// as many columns as the window has room for. Returns the id picked this
/// frame, [`registry::AUTO_DEVICE_ID`] for Auto-detect.
///
/// Keyboard as in every choice list ([`listbox_dropdown`]), plus Left/Right
/// to the nearest entry of the next column over. The list's layout is worked
/// out only while it is open: the closed box is drawn every frame the
/// Settings panel is.
fn device_dropdown(ui: &mut Ui, selected_id: &str, label: &str) -> Option<&'static str> {
    use crate::display::{choice_entry_text, listbox_dropdown, navigate_choice_entries};
    use egui::{Align, ComboBox, Layout, TextWrapMode, vec2};

    let ctx = ui.ctx().clone();
    let style = ui.style().clone();
    let list = std::cell::OnceCell::new();
    let list = || list.get_or_init(|| DeviceListLayout::new(&ctx, &style));

    // Below the button and no further: a list taller than that scrolls,
    // where egui would slide it up over the button. egui also caps it at
    // the `default_area_size` its popups are first measured in (400 pt),
    // which four columns fit; a narrower window scrolls within that.
    let row = ui.spacing().interact_size.y;
    let below = ctx.content_rect().bottom()
        - ui.cursor().top()
        - row
        - 2.0 * ui.spacing().menu_margin.sum().y;
    let height = below.max(MIN_DEVICE_LIST_ROWS * row);

    let combo = |combo: ComboBox| {
        combo
            .selected_text(label)
            // On its own row, the box would otherwise widen past a narrow
            // panel for a long name; the hover text gives it whole.
            .wrap_mode(TextWrapMode::Truncate)
            .height(height)
    };
    let shape = || list().columns.len();
    let (response, picked) = listbox_dropdown(ui, "device", shape, combo, |ui, was_open| {
        let DeviceListLayout {
            groups,
            columns,
            col_w,
            gap,
        } = list();
        let mut picked = None;
        // Every entry in reading order, and the column each sits in (none
        // for Auto-detect, which spans them).
        let mut entries = Vec::with_capacity(registry::DEVICES.len() + 1);
        let mut add = |ui: &mut Ui, id: &'static str, text: &str, full: &str, hover: &str| {
            let live = id == selected_id;
            let entry = ui
                .selectable_label(live, choice_entry_text(text, live))
                .on_hover_text(hover)
                .a11y_label(full);
            if !was_open && live {
                entry.request_focus();
            }
            if entry.gained_focus() {
                entry.scroll_to_me(None);
            }
            if entry.clicked() {
                picked = Some(id);
            }
            entry
        };
        let auto = add(
            ui,
            registry::AUTO_DEVICE_ID,
            "Auto-detect",
            "Auto-detect",
            "Work out which meter is connected",
        );
        entries.push((auto, None));
        ui.add_space(ui.spacing().item_spacing.y);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = *gap;
            for (c, range) in columns.iter().enumerate() {
                let layout = Layout::top_down_justified(Align::LEFT);
                ui.allocate_ui_with_layout(vec2(*col_w, 0.0), layout, |ui| {
                    for (i, (brand, devices)) in groups[range.clone()].iter().enumerate() {
                        if i > 0 {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);
                        }
                        ui.label(RichText::new(brand.name()).color(ui.visuals().weak_text_color()));
                        for d in devices {
                            let hover = format!("Talk to a {}", d.display_name);
                            let entry = add(ui, d.id, d.model_name(), d.display_name, &hover);
                            entries.push((entry, Some(c)));
                        }
                    }
                });
            }
        });
        navigate_columns(ui.ctx(), &entries);
        let responses: Vec<_> = entries.into_iter().map(|(r, _)| r).collect();
        navigate_choice_entries(ui.ctx(), &responses);
        picked
    });
    response.on_hover_text(label).a11y_label("Device");
    picked
}

/// Left/Right in the open **Device** list: focus moves to the entry of the
/// next column over nearest the focused one's height. Up/Down and the rest
/// are [`navigate_choice_entries`]'s, run after this.
fn navigate_columns(ctx: &egui::Context, entries: &[(egui::Response, Option<usize>)]) {
    let Some((focused, Some(column))) = entries
        .iter()
        .find(|(r, _)| r.has_focus())
        .map(|(r, c)| (r, *c))
    else {
        return;
    };
    let pressed = |key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
    let target = if pressed(egui::Key::ArrowRight) {
        column + 1
    } else if pressed(egui::Key::ArrowLeft) && column > 0 {
        column - 1
    } else {
        return;
    };
    let y = focused.rect.center().y;
    let nearest = entries
        .iter()
        .filter(|(_, c)| *c == Some(target))
        .min_by(|(a, _), (b, _)| {
            let da = (a.rect.center().y - y).abs();
            let db = (b.rect.center().y - y).abs();
            da.total_cmp(&db)
        });
    if let Some((entry, _)) = nearest {
        entry.request_focus();
    }
}

/// A **Sample interval** chip's label: what a 0 ms interval means, and whole
/// seconds as seconds, as the **Buffer size** row beside it keeps its
/// labels short.
fn interval_label(ms: u32) -> String {
    match ms {
        0 => "Every reading".to_string(),
        ms if ms >= 1000 && ms % 1000 == 0 => format!("{}s", ms / 1000),
        ms => format!("{ms}ms"),
    }
}

/// What a **Sample interval** chip keeps, for its tooltip.
fn interval_tooltip(ms: u32) -> String {
    let every = match ms {
        0 => return "Keep every reading the meter produces, at its own pace".to_string(),
        1000 => "a second".to_string(),
        ms if ms % 1000 == 0 => format!("every {} s", ms / 1000),
        ms => format!("every {ms} ms"),
    };
    format!("At most one reading {every}: the one nearest each tick")
}

/// What a bound of `n` samples costs, as the **Buffer size** row states it:
/// the memory the graph and the sample buffer take, and how long the bound
/// lasts at the current sample interval.
fn buffer_cost(n: usize, overlays: usize, aux: usize, interval_ms: u32) -> (String, String) {
    // The same wire-time floor `Graph::set_sample_interval_ms` assumes for a
    // 0 ms interval, so the row and the gap detector agree on the rate.
    let interval_secs = (interval_ms as f64 / 1000.0).max(0.1);
    let hours = n as f64 * interval_secs / 3600.0;
    let span = if hours < 10.0 {
        format!("{hours:.1} h")
    } else {
        format!("{hours:.0} h")
    };
    (buffer_memory_estimate(n, overlays, aux), span)
}

/// What the settings panel leaves below itself for the rest of the window.
/// Raising it keeps more of the reading and the graph visible on a short
/// window, and starts scrolling the settings sooner.
const SETTINGS_RESERVE: f32 = 160.0;

/// The shortest the settings rows are ever squashed to, even when that eats
/// into [`SETTINGS_RESERVE`]: below a few lines the panel is a scrollbar
/// beside a sliver of content, and nothing can be found in it.
const SETTINGS_MIN_HEIGHT: f32 = 96.0;

/// How tall the scrolling settings rows may be, given the window height and
/// where the rows start. Floored to whole points so that a fractional
/// overflow can't raise a scrollbar beside rows that fit.
fn settings_scroll_cap(window_h: f32, top: f32) -> f32 {
    (window_h - top - SETTINGS_RESERVE)
        .max(SETTINGS_MIN_HEIGHT)
        .floor()
}

/// A context key's tooltip: like every remote button's, it promises a press
/// and nothing more — the key's own words when the meter has no key of that
/// name to press.
fn context_key_hover(key: &dmm_lib::protocol::MeterKey) -> String {
    key.hover.map_or_else(
        || format!("Press the meter's {} key", key.label),
        str::to_string,
    )
}

/// Width of the rule between the meter's buttons and the Scale chip, in
/// points before zoom: egui's own separator spacing.
pub(super) const SCALE_RULE_WIDTH: f32 = 6.0;

impl App {
    /// The meter's buttons, with the **Scale** chip on the end of their last
    /// line when it fits and on a line of its own otherwise. `right_reserve`
    /// is width the caller paints over at the row's right edge (the big-meter
    /// toggle), which the chip must not run under.
    pub(super) fn show_remote_controls(&mut self, ui: &mut Ui, scale: f32, right_reserve: f32) {
        use super::ConnectionState;

        let font_size = 12.0 * scale;
        let spacing = 3.0 * scale;

        // Only show controls when connected with measurement data and supported commands
        if self.connection.state != ConnectionState::Connected
            || self.last_measurement.is_none()
            || self.connection.supported_commands().is_empty()
        {
            // No button row to join: Scale keeps its own, so an active
            // scale can still be turned off while disconnected.
            let clicked = ui
                .horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    self.show_scale_button(ui, font_size)
                })
                .inner;
            if clicked {
                self.toggle_transform_editor();
            }
            return;
        }
        let flags = self.last_measurement.as_ref().map(|m| m.flags);
        let has_cmd = |cmd: &str| {
            self.connection
                .supported_commands()
                .iter()
                .any(|c| c == cmd)
        };
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let active_color = tc.accent();

        // Collected rather than acted on inside the closure, which holds
        // `has_cmd`'s borrow of `self`.
        let mut scale_clicked = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            // Whether any meter button landed on the row: only then is there
            // something for the Scale chip to be set apart from.
            let mut placed = false;

            let hold = flags.is_some_and(|f| f.hold);
            let rel = flags.is_some_and(|f| f.rel);
            let manual_range = flags.is_some_and(|f| !f.auto_range);
            let auto = flags.is_some_and(|f| f.auto_range);
            let min_max = flags.is_some_and(|f| f.min || f.max);
            let peak = flags.is_some_and(|f| f.peak_min || f.peak_max);

            // Simple toggle commands: label, active flag, command, tooltip.
            // Tooltips are phrased to be device-agnostic — they describe
            // the generic DMM behavior, not model-specific details.
            for &(label, active, cmd, tooltip) in &[
                ("HOLD", hold, "hold", "Freeze the current reading"),
                (
                    "REL",
                    rel,
                    "rel",
                    "Show readings relative to the current value",
                ),
                (
                    "RANGE",
                    manual_range,
                    "range",
                    "Press RANGE (manual range, one step)",
                ),
                ("AUTO", auto, "auto", "Return the meter to auto-range"),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                // `selected` announces the state and paints the selected fill
                // when on; the button keeps its frame when off so it still
                // reads as actionable, unlike `Button::selectable`.
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                let resp = ui
                    .add(egui::Button::new(text).selected(active))
                    .on_hover_text(tooltip);
                placed = true;
                if resp.clicked() {
                    self.send_command(cmd);
                }
            }

            // MIN/MAX and Peak: clicking always cycles (never exits), matching
            // the real device's short-press behavior. A separate "x" button
            // exits the mode (like the real device's long-press).
            for &(label, active, cycle_cmd, exit_cmd, tooltip) in &[
                (
                    "MIN/MAX",
                    min_max,
                    "minmax",
                    "exit_minmax",
                    "Record minimum, maximum, and average readings — click to cycle, × to exit",
                ),
                (
                    "PEAK",
                    peak,
                    "peak",
                    "exit_peak",
                    "Capture peak minimum and maximum — click to cycle, × to exit",
                ),
            ] {
                if !has_cmd(cycle_cmd) {
                    continue;
                }
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                let resp = ui
                    .add(egui::Button::new(text).selected(active))
                    .on_hover_text(tooltip);
                placed = true;
                if resp.clicked() {
                    self.send_command(cycle_cmd);
                }
                if active && has_cmd(exit_cmd) {
                    let x_text = RichText::new("x")
                        .font(egui::FontId::proportional(font_size * 0.8))
                        .color(active_color);
                    let x_btn = egui::Button::new(x_text).min_size(egui::Vec2::ZERO);
                    let exit_label = format!("Exit {label} mode");
                    let x_resp = ui
                        .add(x_btn)
                        .on_hover_text(exit_label.clone())
                        .a11y_label(&exit_label);
                    if x_resp.clicked() {
                        self.send_command(exit_cmd);
                    }
                }
            }

            // The meter's context keys (ZOTEK's ZERO in capacitance, the
            // 121GW's 1kHz in AC, OWON's Hz/Duty, and the CM2100B's ZERO),
            // each only while the reading is one it applies to.
            let reading = self.last_measurement.as_ref();
            for key in self.connection.meter_keys().context {
                if !reading.is_some_and(|m| (key.applies)(m)) {
                    continue;
                }
                let text = RichText::new(key.label).font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(context_key_hover(key))
                    .clicked()
                {
                    self.send_command(key.command);
                }
                placed = true;
            }

            // Non-toggle commands
            for &(label, cmd, tooltip) in &[
                (
                    "SELECT",
                    "select",
                    "Cycle through the secondary functions of the current dial position",
                ),
                ("LIGHT", "light", "Toggle the meter's backlight"),
            ] {
                if !has_cmd(cmd) {
                    continue;
                }
                let text = RichText::new(label).font(egui::FontId::proportional(font_size));
                if ui
                    .add(egui::Button::new(text))
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.send_command(cmd);
                }
                placed = true;
            }

            // Scale, set apart by a rule: it changes nothing on the meter,
            // and sitting it among the buttons with no boundary would
            // suggest the meter knows about the factor. Measured as one
            // unit before placing, so the rule can never be left dangling
            // at the end of a line the chip wrapped off. On a line of its
            // own the line break is the boundary and the rule is dropped.
            let rule_width = SCALE_RULE_WIDTH * scale;
            let chip_width = Self::scale_button_width(ui, font_size);
            let needed = rule_width + spacing + chip_width + spacing + right_reserve;
            if placed && needed <= ui.available_size_before_wrap().x {
                let height = ui.cursor().height();
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(rule_width, height), egui::Sense::hover());
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
            } else if placed {
                ui.end_row();
            }
            scale_clicked = self.show_scale_button(ui, font_size);
        });
        if scale_clicked {
            self.toggle_transform_editor();
        }
    }

    /// The rows below the bar row while the settings are open. Returns the
    /// scroll area's output so a test can check that nothing is scrolled when
    /// the rows fit, and `None` when the settings are closed.
    pub(super) fn show_settings_panel(
        &mut self,
        ui: &mut Ui,
    ) -> Option<egui::scroll_area::ScrollAreaOutput<()>> {
        if !self.settings_open {
            return None;
        }

        ui.separator();
        // A panel clips its content and never scrolls, so on a short window
        // the rows below would simply be cut off — Zoom, the way back from a
        // scale that made the window unusable, among them. Cap the rows and
        // scroll them instead. The cap is measured from the window rather
        // than from `available_height()`: inside a panel the content ui's
        // `max_rect` is last frame's panel rect, so the space a `ScrollArea`
        // would size itself from is stale, and often nothing at all. The cap
        // is handed over as a child ui rather than via `set_max_height`,
        // which unions the new bound with everything placed so far and then
        // moves the cursor back to the top of the panel — the rows would be
        // painted over the bar row.
        let cap = settings_scroll_cap(ui.ctx().content_rect().height(), ui.cursor().top());
        let scrolled = ui
            .allocate_ui(egui::vec2(ui.available_width(), cap), |ui| {
                // Full width, so the bar sits at the panel's edge rather than
                // at the widest row's.
                egui::ScrollArea::vertical()
                    .id_salt("settings_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.show_settings_rows(ui);
                        crate::a11y::scroll_to_focus(ui);
                    })
            })
            .inner;

        // Outside the scroller: the rule closing the panel belongs to the
        // panel, not to the last row that happens to be scrolled into view.
        ui.separator();
        Some(scrolled)
    }

    /// Why the session's meter is not the Device row's to change, when it
    /// isn't: the note the pinned row carries, saying how to get it back.
    ///
    /// A replay's frames come from the file whatever the row names — Connect
    /// re-opens the recording — and the clock flags bend session time, which
    /// only the mock can be asked to run on (`main.rs` refuses them beside a
    /// hardware `--device`), so picking a meter would leave the session timing
    /// its recording and dating its exports by a clock no meter ever ran on.
    fn device_row_pin(&self) -> Option<&'static str> {
        if self.replay.is_some() {
            Some("(restart without --replay to pick a meter)")
        } else if !self.clock.is_real() {
            Some("(restart without the clock flags to pick a meter)")
        } else {
            None
        }
    }

    /// The settings rows, top to bottom. Drawn inside the scroll area that
    /// [`Self::show_settings_panel`] caps, and kept in their own method so
    /// that the rows stay at one indentation level.
    fn show_settings_rows(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let active = self.settings.active_theme().map(|t| t.name.as_str());
            let from_flag = self.settings.overrides.has_theme();
            let overrides = &self.settings.color_overrides;
            let customized = |mode: ThemeMode| match mode {
                ThemeMode::Dark => overrides.dark != PaletteOverrides::default(),
                ThemeMode::Light => overrides.light != PaletteOverrides::default(),
                ThemeMode::System => false,
            };
            let modes = [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System].map(|mode| {
                let selected = active.is_none() && self.settings.theme == mode;
                let base = match mode {
                    ThemeMode::Dark => "Dark",
                    ThemeMode::Light => "Light",
                    ThemeMode::System => "System",
                };
                Chip {
                    value: ThemeChoice::Mode(mode),
                    selected,
                    // Dark and Light keep their own customizations while a
                    // named theme is on; say so on the chip, as a theme's
                    // chip does, picked or not, so a customized Light isn't
                    // taken for plain Light.
                    label: if selected && from_flag {
                        format!("{base} (--theme)")
                    } else if customized(mode) {
                        format!("{base} (customized)")
                    } else {
                        base.to_string()
                    },
                    tooltip: match mode {
                        ThemeMode::System => {
                            "Follow the desktop's light/dark setting (Dark if it reports none)"
                                .to_string()
                        }
                        _ => format!("Use {base} mode for the whole GUI"),
                    },
                }
            });
            let user = self.settings.user_themes.clone();
            let themes_dir = named::user_dir();
            let named = named::listed(&user.themes).into_iter().map(|theme| {
                let selected = active == Some(theme.name.as_str());
                let customized = self
                    .settings
                    .color_overrides
                    .for_theme(&theme.name)
                    .is_some_and(|t| *t != PaletteOverrides::default());
                // Customized whether picked or not, as Dark and Light are.
                let suffix = match (selected && from_flag, customized) {
                    (true, _) => " (--theme)",
                    (false, true) => " (customized)",
                    (false, false) => "",
                };
                Chip {
                    value: ThemeChoice::Named(theme.name.clone()),
                    selected,
                    label: format!("{}{suffix}", theme.name),
                    tooltip: {
                        let mode = if theme.dark { "Dark" } else { "Light" };
                        // A user's theme names its file in full, so it can be
                        // found to edit, share or delete.
                        match (&theme.file, &themes_dir) {
                            (Some(file), Some(dir)) => format!(
                                "{mode} theme, from {}. Delete the file to remove it.",
                                dir.join(file).display()
                            ),
                            (Some(file), None) => format!("{mode} theme, from {file}"),
                            (None, _) => format!("Built-in {} theme", mode.to_lowercase()),
                        }
                    },
                }
            });
            if let Some(choice) = chip_row(ui, "Theme:", modes.into_iter().chain(named)) {
                match choice {
                    ThemeChoice::Mode(mode) => {
                        self.settings.theme = mode;
                        self.settings.named_theme = None;
                    }
                    ThemeChoice::Named(name) => {
                        // Its own mode is the fallback should its file go.
                        if let Some(theme) = named::find(&name, &self.settings.user_themes.themes) {
                            self.settings.theme = if theme.dark {
                                ThemeMode::Dark
                            } else {
                                ThemeMode::Light
                            };
                        }
                        self.settings.named_theme = Some(name);
                    }
                }
                // Clear the override — user explicitly chose a theme
                self.settings.overrides.theme = None;
                self.settings.save();
            }
            let skipped = &self.settings.user_themes.skipped;
            if !skipped.is_empty() {
                let mut list: Vec<String> = themes_dir
                    .iter()
                    .map(|dir| format!("In {}:", dir.display()))
                    .collect();
                list.extend(
                    skipped
                        .iter()
                        .map(|(file, reason)| format!("{file}: {reason}")),
                );
                let caption = match skipped.len() {
                    1 => "1 theme file skipped".to_string(),
                    n => format!("{n} theme files skipped"),
                };
                ui.label(
                    RichText::new(caption)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .on_hover_text(list.join("\n"));
            }
        });

        // -- Color preset selector --
        // Presets are palettes for Dark, Light and System; a named theme
        // carries its own, so the row only shows where it applies.
        if self.settings.active_theme().is_none() {
            self.show_color_preset_row(ui);
        }

        // -- Collapsible color customization --
        self.show_color_customization(ui);

        // -- Graph line style --
        ui.horizontal_wrapped(|ui| {
            let chips = [
                (
                    GraphLines::Patterned,
                    "Patterned",
                    "Dashed and dotted sub-value lines, told apart without colour",
                ),
                (
                    GraphLines::Solid,
                    "Solid",
                    "Continuous lines, told apart by colour and the key",
                ),
            ]
            .map(|(value, label, tooltip)| Chip {
                value,
                selected: self.settings.graph_lines == value,
                label: label.to_string(),
                tooltip: tooltip.to_string(),
            });
            if let Some(lines) = chip_row(ui, "Graph lines:", chips) {
                self.settings.graph_lines = lines;
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let changed = setting_checkbox(
                ui,
                &mut self.settings.show_graph,
                "Graph",
                "Show the rolling time-series plot",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_stats,
                "Statistics",
                "Show Min / Max / Avg / integral for the live session",
            ) | setting_checkbox(
                ui,
                &mut self.settings.show_recording,
                "Recording",
                "Show the recording controls and sample log",
            );
            let (specs_changed, fields_changed) = specs_checkboxes(
                ui,
                &mut self.settings.show_specs,
                &mut self.settings.spec_fields,
            );
            if changed || specs_changed {
                // Manual settings change exits big meter toggle.
                self.big_meter_mode = BigMeterMode::Off;
            }
            if changed || specs_changed || fields_changed {
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let changed = setting_checkbox(
                ui,
                &mut self.settings.auto_connect,
                "Auto-connect on start",
                "Open the USB connection automatically when the app launches",
            ) | setting_checkbox(
                ui,
                &mut self.settings.query_device_name,
                "Show device name on connect (beeps)",
                "Query the meter's name after connecting — the meter will beep once",
            );
            if changed {
                self.settings.save();
            }
        });

        ui.horizontal_wrapped(|ui| {
            let chips = [0u32, 100, 200, 300, 500, 1000, 2000]
                .into_iter()
                .map(|ms| Chip {
                    value: ms,
                    selected: self.settings.sample_interval_ms == ms,
                    label: interval_label(ms),
                    tooltip: interval_tooltip(ms),
                });
            if let Some(ms) = chip_row(ui, "Sample interval:", chips) {
                self.settings.sample_interval_ms = ms;
                self.settings.save();
                self.apply_sample_interval();
            }
        });

        ui.horizontal_wrapped(|ui| {
            // The cost of a bound depends on what this meter is sending right
            // now: a UT181A's four sub-values roughly triple a buffered
            // sample's size and add a trace to the graph's.
            let overlays = self.graph.overlays_len();
            let aux = self
                .last_measurement
                .as_ref()
                .map_or(0, |m| m.aux_values.len());
            let interval_ms = self.settings.sample_interval_ms;
            let chips = [100_000usize, 500_000, 1_000_000, 2_000_000, 5_000_000]
                .into_iter()
                .map(|n| {
                    let (memory, span) = buffer_cost(n, overlays, aux, interval_ms);
                    // Every reading comes at the meter's own pace, which the
                    // estimate cannot know: it says what it assumed.
                    let pace = if interval_ms == 0 {
                        "at 10 readings a second"
                    } else {
                        "at the current sample interval"
                    };
                    Chip {
                        value: n,
                        selected: self.settings.max_samples == n,
                        label: format_sample_count(n),
                        tooltip: format!(
                            "Keep up to {} samples in the graph and for export \u{2014} {memory}, \
                             about {span} {pace}. A stopped recording kept beside them can \
                             take as much again",
                            format_sample_count(n)
                        ),
                    }
                });
            if let Some(n) = chip_row(ui, "Buffer size:", chips) {
                self.settings.max_samples = n;
                self.settings.save();
                // Live: the graph and the history behind Export… evict down
                // to the new bound on the spot, and a recording already past
                // it stops rather than losing the samples it has.
                self.graph.set_max_points(n);
                if self.capture.recording.set_max_samples(n) {
                    self.buffer_shrunk_toast();
                }
            }
            let (memory, span) = buffer_cost(self.settings.max_samples, overlays, aux, interval_ms);
            ui.label(
                RichText::new(format!("({memory}, about {span})"))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.horizontal_wrapped(|ui| {
            let pinned = self.device_row_pin();
            let has_override = self.settings.overrides.has_device();
            // What the selection resolves to, not what the file spells: an
            // alias, or an id no entry answers to, would otherwise leave the
            // row with nothing marked while the session opened something.
            let selected_id = self
                .selected_device()
                .map_or(registry::AUTO_DEVICE_ID, |d| d.id);
            let name = registry::find_device(selected_id).map_or("Auto-detect", |d| d.display_name);
            let label = if has_override {
                format!("{name} (--device)")
            } else {
                name.to_string()
            };
            let picked = ui
                .scope(|ui| {
                    if pinned.is_some() {
                        ui.disable();
                    }
                    ui.label("Device:");
                    device_dropdown(ui, selected_id, &label)
                })
                .inner;
            if let Some(note) = pinned {
                ui.label(
                    RichText::new(note)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if let Some(id) = picked {
                self.settings.shared.device_family = id.to_string();
                // Clear the override — user explicitly chose a device
                self.settings.overrides.device_family = None;
                self.settings.save();
                // Auto-reconnect if currently connected
                if self.connection.state != super::ConnectionState::Disconnected {
                    self.connection.needs_reconnect = true;
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            let label = if self.settings.overrides.has_bluetooth() {
                format!("{BLUETOOTH_SETTING} (--no-bluetooth)")
            } else {
                BLUETOOTH_SETTING.to_string()
            };
            // No reconnect: a session already running over an adapter would
            // be dropped by one, and the setting only decides what the next
            // connect looks at.
            if setting_checkbox(
                ui,
                &mut self.settings.shared.bluetooth,
                &label,
                "Look for a Bluetooth adapter or meter in range when no USB cable answers. \
                 Takes effect on the next connect.",
            ) {
                // Cleared like every other row the user sets by hand: the
                // value they picked is theirs to keep.
                self.settings.overrides.bluetooth = None;
                self.settings.save();
            }
        });

        // Mock mode selector (only shown when mock device is selected)
        if self
            .selected_device()
            .is_some_and(|d| d.id == dmm_lib::mock::MOCK.id)
        {
            ui.horizontal_wrapped(|ui| {
                let has_override = self.settings.overrides.has_mock_mode();
                // "Auto" = cycle through all modes, and leads the row.
                let auto_selected = self.settings.mock_mode.is_empty();
                let auto = std::iter::once(Chip {
                    value: String::new(),
                    selected: auto_selected,
                    label: if auto_selected && has_override {
                        "Auto (cycle) (--mock-mode)"
                    } else {
                        "Auto (cycle)"
                    }
                    .to_string(),
                    tooltip: "Cycle through all synthetic modes to exercise the GUI".to_string(),
                });
                let modes = MockMode::ALL.iter().map(|mode| {
                    let mode_label = mode.label();
                    let selected = self.settings.mock_mode == mode_label;
                    Chip {
                        value: mode_label.to_string(),
                        selected,
                        label: if selected && has_override {
                            format!("{mode_label} (--mock-mode)")
                        } else {
                            mode_label.to_string()
                        },
                        tooltip: mode.description().to_string(),
                    }
                });
                if let Some(mock_mode) = chip_row(ui, "Mock mode:", auto.chain(modes)) {
                    self.settings.mock_mode = mock_mode;
                    // Clear the override — user explicitly chose a mock mode
                    self.settings.overrides.mock_mode = None;
                    self.settings.save();
                    if self.connection.state != super::ConnectionState::Disconnected {
                        self.connection.needs_reconnect = true;
                    }
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            let chips = Self::ZOOM_LEVELS.iter().map(|&level| Chip {
                value: level,
                selected: self.settings.zoom_pct == level,
                label: format!("{level}%"),
                tooltip: if level == 100 {
                    "Scale the GUI to 100% (Ctrl+0, or Ctrl+/- to step)".to_string()
                } else {
                    format!("Scale the GUI to {level}% (Ctrl+/- to step)")
                },
            });
            if let Some(level) = chip_row(ui, "Zoom:", chips) {
                self.settings.zoom_pct = level;
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+/- to adjust, Ctrl+0 = 100%)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        // The Wayland caption is a sentence, so it wraps rather than
        // running off the edge of a narrow window.
        ui.horizontal_wrapped(|ui| {
            // Greyed rather than hidden on Wayland, and the saved value is
            // left alone: a `true` written on an X11 session still applies
            // there.
            let response = ui
                .add_enabled(
                    !self.on_wayland,
                    egui::Checkbox::new(&mut self.settings.always_on_top, "Always on top"),
                )
                .on_hover_text("Keep the window above other desktop windows (Ctrl+T)")
                .on_disabled_hover_text(ALWAYS_ON_TOP_WAYLAND_HINT);
            if response.changed() {
                self.apply_always_on_top(ui.ctx());
                self.settings.save();
            }
            if self.on_wayland {
                ui.label(
                    RichText::new(format!("({ALWAYS_ON_TOP_WAYLAND_HINT})"))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        ui.horizontal_wrapped(|ui| {
            if setting_checkbox(
                ui,
                &mut self.settings.hide_decorations,
                "Hide window decorations",
                "Borderless window — use Ctrl+D to toggle back",
            ) {
                self.apply_decorations(ui.ctx());
                self.settings.save();
            }
            ui.label(
                RichText::new("(Ctrl+D to toggle)")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

        // Only downloaded builds ask, so only they have the row. Last, so the
        // rows above keep the places the doc screenshots click.
        if self.update_check.applies() {
            ui.horizontal_wrapped(|ui| {
                if setting_checkbox(
                    ui,
                    &mut self.settings.check_for_updates,
                    "Check for new versions",
                    "Once a day, ask GitHub whether a newer release is out. \
                     GitHub sees your IP address; nothing about your meter is sent.",
                ) {
                    self.settings.save();
                }
            });
        }
    }

    /// The **Colors** row: the palette presets for Dark, Light and System.
    fn show_color_preset_row(&mut self, ui: &mut Ui) {
        // Only the presets' own customizations: a named theme's are kept
        // when the preset changes, so they don't count here.
        let has_overrides = self.settings.color_overrides.dark != PaletteOverrides::default()
            || self.settings.color_overrides.light != PaletteOverrides::default();
        ui.horizontal_wrapped(|ui| {
            let chips = [
                ColorPreset::Default,
                ColorPreset::HighContrast,
                ColorPreset::ColorblindSafe,
            ]
            .into_iter()
            .map(|preset| {
                let selected = self.settings.color_preset == preset;
                let base = match preset {
                    ColorPreset::Default => "Default",
                    ColorPreset::HighContrast => "High Contrast",
                    ColorPreset::ColorblindSafe => "Colorblind",
                };
                Chip {
                    value: preset,
                    selected,
                    label: if selected && has_overrides {
                        format!("{base} (customized)")
                    } else {
                        base.to_string()
                    },
                    tooltip: match preset {
                        ColorPreset::Default => "Balanced palette tuned for everyday use",
                        ColorPreset::HighContrast => {
                            "Maximum-contrast palette for bright lighting or projectors"
                        }
                        ColorPreset::ColorblindSafe => {
                            "Palette that stays distinguishable for protan/deutan vision"
                        }
                    }
                    .to_string(),
                }
            });
            if let Some(preset) = chip_row(ui, "Colors:", chips) {
                self.settings.color_preset = preset;
                // Clear the presets' overrides when switching presets.
                self.settings.color_overrides.dark = PaletteOverrides::default();
                self.settings.color_overrides.light = PaletteOverrides::default();
                self.applied.ui_colors = None; // force reapply
                self.settings.save();
            }
            if has_overrides {
                ui.label(
                    RichText::new("Selecting a preset will clear customizations")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });
    }

    /// Show the collapsible color customization section.
    fn show_color_customization(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;

        let collapsing = egui::CollapsingHeader::new("Customize colors")
            .default_open(false)
            .show(ui, |ui| {
                // A named theme's own palette only in its own mode, as
                // `Settings::color_tweaks` has it: on the frame a chip is
                // picked, the UI may still be in the other.
                let named = self
                    .settings
                    .active_theme()
                    .filter(|t| t.dark == dark)
                    .map(|t| t.name.clone());
                let editing = match &named {
                    Some(name) => format!("(editing {name} colors)"),
                    None if dark => "(editing dark theme colors)".to_string(),
                    None => "(editing light theme colors)".to_string(),
                };
                ui.label(
                    RichText::new(editing)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );

                let mut changed = false;
                // What every swatch shows: followers already moved with
                // their anchors.
                let current = self.settings.theme_colors(dark);
                // And what it is before the user's changes: a warning is for
                // what they changed, not what the palette chose.
                let base = self.settings.uncustomized_colors(dark);
                // A named theme's tweaks are edited on a copy and put back
                // only when they change, so that just showing the swatches
                // doesn't leave an empty entry behind for every theme.
                let mut tweaks = named
                    .as_ref()
                    .and_then(|name| self.settings.color_overrides.for_theme(name))
                    .cloned()
                    .unwrap_or_default();
                let overrides = match named {
                    Some(_) => &mut tweaks,
                    None => self.settings.color_overrides.for_mode_mut(dark),
                };

                // In families: an anchor on its row, and the colours that
                // follow it on an indented row beneath, so what moves with
                // what shows without hovering. Each swatch's label, tooltip
                // and override slot come from the enum, so a colour can't be
                // listed with another's tooltip or wired to its override.
                for &(caption, row) in PANEL_ROWS {
                    match row {
                        PanelRow::Family(anchor) => {
                            // One row per family, the settings panel being
                            // short on height: the anchor, an arrow, the
                            // colours that follow it — dashed while they do.
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, caption);
                                changed |= color_edit(ui, anchor, overrides, &current, &base);
                                ui.label(
                                    RichText::new("\u{2192}").color(ui.visuals().weak_text_color()),
                                )
                                .on_hover_text(format!(
                                    "These follow {}, keeping their offset from it, until \
                                     you pick one on its own",
                                    anchor.label()
                                ));
                                for field in links::followers(anchor) {
                                    changed |= color_edit(ui, field, overrides, &current, &base);
                                }
                            });
                        }
                        PanelRow::Fields(fields) => {
                            ui.horizontal_wrapped(|ui| {
                                caption_cell(ui, caption);
                                for &field in fields {
                                    changed |= color_edit(ui, field, overrides, &current, &base);
                                }
                            });
                        }
                    }
                }
                if changed && let Some(name) = &named {
                    self.settings.color_overrides.set_for_theme(name, tweaks);
                }

                // Reset button
                ui.horizontal(|ui| {
                    if ui
                        .button("Reset colors")
                        .on_hover_text(match &named {
                            Some(name) => format!("Discard your changes to {name}'s colors"),
                            None => "Discard your changes to the colors of this mode".to_string(),
                        })
                        .clicked()
                    {
                        match &named {
                            Some(name) => self
                                .settings
                                .color_overrides
                                .set_for_theme(name, PaletteOverrides::default()),
                            None => {
                                *self.settings.color_overrides.for_mode_mut(dark) =
                                    PaletteOverrides::default()
                            }
                        }
                        changed = true;
                    }
                    // On the same row: the section is tall enough already.
                    self.show_theme_save_row(ui, dark);
                });

                if changed {
                    self.applied.ui_colors = None; // force reapply
                    // Deferred: a drag or a held arrow key changes the
                    // colour every frame, and each save is an fsync.
                    self.settings_save.schedule(std::time::Instant::now());
                }
            });
        // Paint an explicit focus ring on the header when Tab-focused —
        // egui's CollapsingHeader shows only a subtle highlight otherwise,
        // which is easy to miss.
        crate::a11y::paint_focus_ring(ui, &collapsing.header_response);
        collapsing
            .header_response
            .on_hover_text("Per-color overrides on top of the selected preset or theme");
    }
}

/// One row of the Customize colors swatches.
#[derive(Clone, Copy)]
enum PanelRow {
    /// An anchor, then the colours that follow it, on one row.
    Family(PaletteField),
    /// Colours that follow nothing.
    Fields(&'static [PaletteField]),
}

/// The swatch rows, top to bottom, under their captions. Every palette
/// field is on exactly one: as an anchor, as one of its followers, or in a
/// row of its own (`panel_rows_list_every_field_once`).
const PANEL_ROWS: &[(&str, PanelRow)] = &[
    ("UI:", PanelRow::Family(PaletteField::Background)),
    ("", PanelRow::Family(PaletteField::Text)),
    ("", PanelRow::Fields(&[PaletteField::Accent])),
    ("Graph:", PanelRow::Family(PaletteField::GraphLine)),
    (
        "",
        PanelRow::Fields(&[
            PaletteField::GraphGap,
            PaletteField::GraphMean,
            PaletteField::GraphRef,
            PaletteField::GraphCrossing,
            PaletteField::GraphCursor,
            PaletteField::GraphMarker,
        ]),
    ),
    (
        "Status:",
        PanelRow::Fields(&[
            PaletteField::StatusOk,
            PaletteField::StatusWarning,
            PaletteField::StatusError,
            PaletteField::StatusInactive,
        ]),
    ),
];

/// A row's caption, at one width so the swatches line up beneath it.
fn caption_cell(ui: &mut Ui, caption: &str) {
    let width = 56.0;
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.label(caption);
        },
    );
}

/// A colour as the picker's hex field shows it.
fn hex_text(c: egui::Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// A colour typed or pasted as `#RRGGBB`, the `#` optional.
fn parse_hex(text: &str) -> Option<egui::Color32> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(egui::Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// Where a swatch whose relink button was just pressed waits to take focus
/// back on the next frame.
fn relink_refocus_id() -> egui::Id {
    egui::Id::new("color_relink_refocus")
}

/// Render a color edit button with label. Returns true if the color was
/// changed, or a follower set on its own was linked again.
///
/// A follower its anchor still moves has a dashed outline; one set on its
/// own a solid one and a relink button. A colour under its contrast floor
/// that the user is answerable for — set it, or set its ground — carries a
/// warning with the ratio; a linked follower never does, it keeps its floor.
fn color_edit(
    ui: &mut Ui,
    field: PaletteField,
    overrides: &mut PaletteOverrides,
    current: &ThemeColors,
    base: &ThemeColors,
) -> bool {
    let label = field.label();
    let anchor = links::anchor(field);
    let set = field.override_slot(overrides).is_some();
    let linked = anchor.is_some() && !set;
    // The colour on screen: this field's override, or where the preset, the
    // theme or its anchor put it.
    let mut color = current.effective_color(field);

    // Render the swatch as a plain Button with an explicit fill, so we control
    // the open lifecycle. egui's `color_edit_button_srgba` also uses
    // `Popup::menu`, but doesn't move focus into the popup when it opens —
    // keyboard users end up stranded on the settings panel.
    //
    // The swatch and its label are one unit, sized before it is placed so the
    // wrapped group row can move it to the next line. A `ui.horizontal` here
    // claimed the rest of the row and never wrapped, and its overflow widened
    // the whole settings panel — every other row then kept folding at that
    // width instead of the window's.
    let gap = 2.0;
    let btn_size = egui::Vec2::splat(ui.spacing().interact_size.y);
    let text = egui::WidgetText::from(RichText::new(label).small());
    let galley = text.clone().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    let unit = egui::vec2(
        btn_size.x + gap + galley.size().x,
        btn_size.y.max(galley.size().y),
    );
    let response = ui.allocate_ui_with_layout(
        unit,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let btn = ui.add(egui::Button::new("").fill(color).min_size(btn_size));
            ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
            btn
        },
    );

    // The swatch's visible content is just a color, which screen readers
    // can't describe — give it the label text as its accessible name.
    let btn_response = response
        .inner
        .on_hover_text(field.tooltip())
        .a11y_label(&match anchor {
            Some(anchor) if linked => format!("{label}, follows {}", anchor.label()),
            _ => label.to_string(),
        });
    if ui
        .ctx()
        .data(|d| d.get_temp::<egui::Id>(relink_refocus_id()))
        == Some(btn_response.id)
    {
        ui.ctx()
            .data_mut(|d| d.remove::<egui::Id>(relink_refocus_id()));
        btn_response.request_focus();
    }
    // An outline on every swatch, so one the colour of the panel it sits
    // on still shows; dashed while it follows its anchor.
    let rect = btn_response.rect;
    let stroke = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    if linked {
        let r = rect.expand(1.0);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ];
        ui.painter()
            .extend(egui::Shape::dashed_line(&corners, stroke, 3.0, 2.0));
    } else {
        ui.painter()
            .rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
    }
    // The fill covers the usual button border, so paint an explicit focus
    // ring when the swatch is keyboard-focused.
    crate::a11y::paint_focus_ring(ui, &btn_response);

    let mut relinked = false;
    if let Some(anchor) = anchor
        && set
        && ui
            .small_button("\u{21BA} relink")
            .on_hover_text(format!("Follow {} again", anchor.label()))
            .a11y_label(&format!("Relink {label} to {}", anchor.label()))
            .clicked()
    {
        *field.override_slot(overrides) = None;
        relinked = true;
        // The button goes with the override; keep keyboard focus on the
        // colour it belonged to rather than dropping it to the window's
        // first widget. Next frame: egui hands focus to the button it was
        // clicked on after this runs, and the button is gone by then.
        ui.ctx()
            .data_mut(|d| d.insert_temp(relink_refocus_id(), btn_response.id));
        ui.ctx().request_repaint();
    }
    // Only where the user's changes brought it under: the presets' own
    // decorative borders sit under the graphical floor on purpose. A linked
    // colour warns too, when no lightness could keep it readable.
    if let Some((ratio, ground, floor)) = links::floor_failure(current, field)
        && links::floor_failure(base, field).is_none()
    {
        ui.label(
            RichText::new(format!(
                "{} {ratio:.1}:1 on {}",
                super::toast::ERROR_GLYPH,
                ground.label()
            ))
            .small()
            .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text(format!(
            "Under the {floor}:1 contrast this color needs on {}",
            ground.label()
        ));
    }

    // Graph colours drawn together have to read apart; a turn of the line
    // can bring an overlay onto a colour that stayed put. Said only where
    // the user's changes did it, as the contrast warning is.
    if let Some((other, d, floor)) = links::too_close(current, field)
        && links::too_close(base, field).is_none()
    {
        ui.label(
            RichText::new(format!(
                "{} close to {}",
                super::toast::ERROR_GLYPH,
                other.label()
            ))
            .small()
            .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text(format!(
            "Only {d:.0} apart (\u{0394}E) where graph colors need {floor:.0} to read apart"
        ));
    }

    let popup_id = btn_response.id.with("color_popup");
    // "This click is the one that opens the popup" — true only on the click
    // frame *and* only when the popup was closed at the start of the frame.
    // `Popup::menu` flips the memory state inside its own `show` call, so at
    // this point `is_id_open` still returns the pre-toggle value.
    let newly_opened = btn_response.clicked() && !egui::Popup::is_id_open(ui.ctx(), popup_id);

    // Popup has no built-in Esc handling (unlike egui::Modal), so consume
    // Esc manually while the popup is open.
    if egui::Popup::is_id_open(ui.ctx(), popup_id)
        && ui
            .ctx()
            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        egui::Popup::close_id(ui.ctx(), popup_id);
    }

    // Track open state across frames to detect close transitions (click
    // outside, Esc, or swatch re-click) so focus can be restored to the
    // swatch regardless of how the popup closed.
    let was_open_key = btn_response.id.with("color_popup_was_open");
    let was_open: bool = ui.ctx().data(|d| d.get_temp(was_open_key)).unwrap_or(false);

    let mut color_changed = false;
    let hsva_cache_key = btn_response.id.with("hsva_cache");
    egui::Popup::menu(&btn_response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            // Trap Tab focus to the popup's layer. Without this, Tab cycles
            // through main-settings widgets (which are registered earlier in
            // the frame) rather than through the picker's drag values and
            // sliders.
            ui.ctx().memory_mut(|m| m.set_modal_layer(ui.layer_id()));

            // Invisible focusable anchor. When the popup first opens, focus
            // is still on the swatch — in a layer *below* the modal layer and
            // therefore no longer focusable — so Tab wouldn't advance to
            // anything. Requesting focus on this anchor puts the user inside
            // the popup's focus cycle immediately.
            // Zero-sized and placed with `interact` rather than added, so it
            // takes no row of its own: an empty label here left a blank line
            // at the top of the picker.
            let focus_anchor = ui.interact(
                egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::ZERO),
                btn_response.id.with("popup_focus_anchor"),
                egui::Sense::focusable_noninteractive(),
            );
            if newly_opened {
                focus_anchor.request_focus();
            }

            // HSVA is the source of truth while the popup is open.
            // Converting srgba → Hsva each frame is slightly lossy (sRGB
            // gamma), so we cache the Hsva in ctx temp data and only seed
            // from the current color on first open.
            let mut hsva: egui::ecolor::Hsva = if newly_opened {
                egui::ecolor::Hsva::from(color)
            } else {
                ui.ctx()
                    .data(|d| d.get_temp::<egui::ecolor::Hsva>(hsva_cache_key))
                    .unwrap_or_else(|| egui::ecolor::Hsva::from(color))
            };

            // Arrow-key HSV adjustment. egui's `color_slider_1d` and
            // `color_slider_2d` only handle `interact_pointer_pos` (mouse
            // drag), so Tab-focused sliders do nothing on arrow press.
            // Detect which slider currently has focus via the rect shape
            // of the focused widget (2D slider is square, hue slider is
            // wide-and-short) and apply the arrow deltas directly to
            // `hsva` before the picker renders. The size thresholds are
            // chosen to include 100-wide sliders (observed in this app's
            // theme) while excluding small toggle/drag widgets (~20 px).
            let hex_id = btn_response.id.with("hex_field");
            if let Some(fid) = ui.ctx().memory(|m| m.focused())
                && fid != btn_response.id
                // The hex field is as wide and short as the hue slider,
                // and its arrows move the caret.
                && fid != hex_id
                && let Some(focused_resp) = ui.ctx().read_response(fid)
            {
                let rect = focused_resp.rect;
                let is_2d_slider =
                    rect.width() >= 50.0 && (rect.width() - rect.height()).abs() < 2.0;
                let is_hue_slider = rect.width() >= 50.0 && rect.width() > rect.height() * 3.0;

                let left = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
                let right = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
                let up = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                let down = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));

                let step = 0.02;
                let dx = if right { step } else { 0.0 } - if left { step } else { 0.0 };
                let dy = if up { step } else { 0.0 } - if down { step } else { 0.0 };

                if is_2d_slider && (dx != 0.0 || dy != 0.0) {
                    hsva.s = (hsva.s + dx).clamp(0.0, 1.0);
                    hsva.v = (hsva.v + dy).clamp(0.0, 1.0);
                    color_changed = true;
                } else if is_hue_slider && dx != 0.0 {
                    hsva.h = (hsva.h + dx).rem_euclid(1.0);
                    color_changed = true;
                }
            }

            color_changed |= egui::color_picker::color_picker_hsva_2d(
                ui,
                &mut hsva,
                egui::color_picker::Alpha::Opaque,
            );

            // The colour as text, to copy from one swatch and paste into
            // another rather than retyping three channels. It shows the
            // colour while not being edited; while it is, a valid entry
            // applies at once.
            let text_key = hex_id.with("text");
            let editing = ui.ctx().memory(|m| m.has_focus(hex_id));
            let mut text = if editing {
                ui.ctx()
                    .data(|d| d.get_temp::<String>(text_key))
                    .unwrap_or_else(|| hex_text(egui::Color32::from(hsva)))
            } else {
                hex_text(egui::Color32::from(hsva))
            };
            ui.horizontal(|ui| {
                let caption = ui.label("Hex");
                let field = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .id(hex_id)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(80.0),
                    )
                    .labelled_by(caption.id);
                let field = field.on_hover_text("Copy this color, or paste one as #RRGGBB");
                if field.changed()
                    && let Some(pasted) = parse_hex(&text)
                {
                    hsva = egui::ecolor::Hsva::from(pasted);
                    color_changed = true;
                }
            });
            ui.ctx().data_mut(|d| d.insert_temp(text_key, text));

            // Write back to Color32 and persist Hsva for next frame.
            color = egui::Color32::from(hsva);
            ui.ctx().data_mut(|d| d.insert_temp(hsva_cache_key, hsva));
        });

    let is_open_now = egui::Popup::is_id_open(ui.ctx(), popup_id);
    if is_open_now {
        // Trap arrow keys on whichever widget inside the popup currently
        // has focus. egui's color_slider_1d/2d (hue + saturation-value) are
        // focusable but don't respond to arrow keys; without trapping, the
        // first arrow press Tab-jumps focus off the slider spatially.
        // Trapping keeps focus inside the popup so the user can still Tab
        // between sliders and the RGBA drag values (which ARE keyboard-
        // adjustable via Enter-to-edit + Up/Down). See the "Known
        // limitations" note in docs/gui-reference.md on the color picker.
        if let Some(focused_id) = ui.ctx().memory(|m| m.focused())
            && focused_id != btn_response.id
        {
            ui.ctx().memory_mut(|m| {
                m.set_focus_lock_filter(
                    focused_id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
            // `set_focus_lock_filter` only takes effect on the *next*
            // frame because of its `had_focus_last_frame` gate. Cover the
            // first-frame case by also resetting `focus_direction` if any
            // arrow is held this frame.
            let any_arrow_down = ui.ctx().input(|i| {
                i.key_down(egui::Key::ArrowLeft)
                    || i.key_down(egui::Key::ArrowRight)
                    || i.key_down(egui::Key::ArrowUp)
                    || i.key_down(egui::Key::ArrowDown)
            });
            if any_arrow_down {
                ui.ctx()
                    .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
        }
    }
    if was_open && !is_open_now {
        // Popup just closed — put focus back on the swatch so keyboard users
        // don't get teleported to the top of the Tab order.
        ui.ctx().memory_mut(|m| m.request_focus(btn_response.id));
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(was_open_key, is_open_now));

    if color_changed {
        *field.override_slot(overrides) = Some(HexColor(color));
        return true;
    }

    relinked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hex_field_reads_what_it_writes_and_what_is_pasted() {
        let c = egui::Color32::from_rgb(0xB8, 0x00, 0x96);
        assert_eq!(hex_text(c), "#B80096");
        assert_eq!(parse_hex("#B80096"), Some(c));
        assert_eq!(parse_hex("  b80096 "), Some(c));
        for bad in ["#B8009", "#B800966", "#G80096", "", "#"] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
    }

    /// Every palette field has one swatch: as an anchor, as one of its
    /// followers, or in a row of its own — and a follower sits under its own
    /// anchor, not in a flat row.
    #[test]
    fn panel_rows_list_every_field_once() {
        let mut listed = Vec::new();
        for &(_, row) in PANEL_ROWS {
            match row {
                PanelRow::Family(anchor) => {
                    assert_eq!(links::anchor(anchor), None, "{anchor:?}");
                    listed.push(anchor);
                    listed.extend(links::followers(anchor));
                }
                PanelRow::Fields(fields) => {
                    for &f in fields {
                        assert_eq!(links::anchor(f), None, "{f:?} belongs under its anchor");
                        listed.push(f);
                    }
                }
            }
        }
        for &f in PaletteField::ALL {
            assert_eq!(listed.iter().filter(|&&g| g == f).count(), 1, "{f:?}");
        }
        assert_eq!(listed.len(), PaletteField::ALL.len());
    }
    use crate::app::update_check::{Tag, UpdateCheck};
    use crate::settings::Settings;
    use eframe::egui::scroll_area::ScrollAreaOutput;
    use eframe::egui::{Id, Pos2, Rect, vec2};

    /// The settings rows laid out under the real bar row.
    struct SettingsFrame {
        ctx: egui::Context,
        /// Where the bar row ends: the rows must start below it.
        bar_bottom: f32,
        scrolled: ScrollAreaOutput<()>,
        /// The AccessKit tree, empty unless the run enabled it.
        nodes: Vec<(egui::accesskit::NodeId, egui::accesskit::Node)>,
        /// The AccessKit node with keyboard focus, when the run enabled it.
        focus: Option<egui::accesskit::NodeId>,
    }

    /// An open settings panel in a `w` x `h` window, driven a frame at a
    /// time. The bar row is the app's own, since one bug this guards against
    /// is the rows being laid over it.
    struct SettingsRun {
        app: App,
        ctx: egui::Context,
        screen: Rect,
        /// Jumps a second per frame, so egui's scroll animation — a few
        /// hundred milliseconds — has always finished by the next one.
        seconds: f64,
    }

    impl SettingsRun {
        fn new(w: f32, h: f32) -> Self {
            let mut app = App::from_settings(Settings::default(), dmm_lib::Clock::real());
            app.settings_open = true;
            // As a downloaded build, which has one row more, and with its
            // notice on the bar — forced, so nothing is fetched or saved.
            app.update_check = UpdateCheck::new(Tag::parse("v9.9.9"));
            Self {
                app,
                ctx: egui::Context::default(),
                screen: Rect::from_min_size(Pos2::ZERO, vec2(w, h)),
                seconds: 0.0,
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) -> SettingsFrame {
            self.frame_after(1.0, events)
        }

        /// A frame `secs` after the previous one.
        fn frame_after(&mut self, secs: f64, events: Vec<egui::Event>) -> SettingsFrame {
            let mut bar_bottom = 0.0;
            let mut scrolled = None;
            let app = &mut self.app;
            self.seconds += secs;
            let mut out = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(self.screen),
                    events,
                    time: Some(self.seconds),
                    ..Default::default()
                },
                |ui| {
                    egui::Panel::top("top_bar").show(ui, |ui| {
                        let ctx = ui.ctx().clone();
                        app.show_top_bar(ui, &ctx);
                        bar_bottom = ui.min_rect().bottom();
                        scrolled = app.show_settings_panel(ui);
                    });
                },
            );
            out.textures_delta.clear();
            let (nodes, focus) = out
                .platform_output
                .accesskit_update
                .map(|update| (update.nodes, Some(update.focus)))
                .unwrap_or_default();
            SettingsFrame {
                ctx: self.ctx.clone(),
                bar_bottom,
                scrolled: scrolled.expect("the settings are open"),
                nodes,
                focus,
            }
        }

        /// Click `pos` the way a mouse does — move, press, release on
        /// successive frames — and return the release frame. The release
        /// follows the press within egui's click window, not a second later.
        fn click(&mut self, pos: Pos2) -> SettingsFrame {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            self.frame(vec![egui::Event::PointerMoved(pos)]);
            self.frame(vec![egui::Event::PointerMoved(pos), button(true)]);
            self.frame_after(0.1, vec![egui::Event::PointerMoved(pos), button(false)])
        }
    }

    /// Where a widget sits, as AccessKit reports it: a button carries its
    /// text as the node's label, a label as its value.
    fn node_bounds(frame: &SettingsFrame, text: &str) -> egui::accesskit::Rect {
        frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(text) || n.value() == Some(text))
            .and_then(|(_, n)| n.bounds())
            .unwrap_or_else(|| panic!("no {text:?} widget in the settings"))
    }

    /// After three headless frames — a panel sizes itself from the previous
    /// frame, so the first one alone proves nothing.
    fn settings_panel(w: f32, h: f32) -> SettingsFrame {
        let mut run = SettingsRun::new(w, h);
        run.frame(vec![]);
        run.frame(vec![]);
        run.frame(vec![])
    }

    /// Tab pressed and released within one frame.
    fn tab() -> Vec<egui::Event> {
        press(egui::Key::Tab)
    }

    /// `key` pressed and released within one frame.
    fn press(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }

    /// Whether the focused node is one named `label`: the Device box's
    /// value and its live entry's label are the same name.
    fn focused_on(frame: &SettingsFrame, label: &str) -> bool {
        frame.nodes.iter().any(|(id, n)| {
            (n.label() == Some(label) || n.value() == Some(label)) && frame.focus == Some(*id)
        })
    }

    /// Whether the open Device list is drawn: its Auto-detect entry is.
    fn device_list_open(frame: &SettingsFrame) -> bool {
        frame.nodes.iter().any(|(_, n)| {
            n.label() == Some("Auto-detect") && n.role() != egui::accesskit::Role::ComboBox
        })
    }

    /// A settings run with the **Device** list opened from the keyboard:
    /// Tab to the box, then Space, as a keyboard user opens it.
    fn open_device_list(run: &mut SettingsRun) -> SettingsFrame {
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let mut reached = false;
        for _ in 0..60 {
            let frame = run.frame(tab());
            if focused_on(&frame, "Device") {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab never reached the Device box");
        run.frame(press(egui::Key::Space));
        run.frame(vec![])
    }

    #[test]
    fn device_columns_keep_groups_whole_and_balance_the_tallest() {
        // Brymen, EEVblog, OWON, UNI-T, Voltcraft, ZOTEK, Simulated, each
        // with its heading.
        let heights = [5, 2, 12, 17, 9, 5, 3];
        assert_eq!(pack_columns(&heights, 1), vec![0..7]);
        assert_eq!(pack_columns(&heights, 4), vec![0..2, 2..3, 3..4, 4..7]);
        // Three: UNI-T alone cannot be beaten, so the split puts the rest
        // either side of it.
        assert_eq!(pack_columns(&heights, 3), vec![0..3, 3..4, 4..7]);
        // More columns than groups: one group each.
        assert_eq!(pack_columns(&[1, 1], 4), vec![0..1, 1..2]);
        // A tie goes to the taller columns first.
        assert_eq!(pack_columns(&[1, 1, 1], 2), vec![0..2, 2..3]);
    }

    /// Opened from the keyboard, the list puts focus on the selected meter,
    /// so Enter picks it and a screen reader names it, with no Tab first.
    #[test]
    fn the_device_list_opens_on_the_selected_meter() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        let frame = open_device_list(&mut run);
        assert!(device_list_open(&frame), "Space did not open the list");
        assert!(
            focused_on(&frame, "UT61E+"),
            "focus is not on the selected meter"
        );
    }

    /// Down follows the brand's column, Right jumps to the next column over
    /// — UNI-T to Voltcraft at this width — and Left comes back.
    #[test]
    fn arrows_walk_the_device_list_down_and_across() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        let frame = run.frame(press(egui::Key::ArrowDown));
        assert!(focused_on(&frame, "UT61B+"));
        let frame = run.frame(press(egui::Key::ArrowRight));
        assert!(
            focused_on(&frame, "Voltcraft VC650BT"),
            "Right left the row"
        );
        let frame = run.frame(press(egui::Key::ArrowLeft));
        assert!(focused_on(&frame, "UT61B+"));
    }

    /// Enter picks the focused meter: saved as the device, the list closed
    /// and the keyboard back on the box.
    #[test]
    fn enter_picks_the_focused_meter() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        run.frame(press(egui::Key::ArrowDown));
        run.frame(press(egui::Key::Enter));
        let frame = run.frame(vec![]);
        assert_eq!(run.app.settings.shared.device_family, "ut61b+");
        assert!(!device_list_open(&frame), "the list stayed open");
        let (id, node) = frame
            .nodes
            .iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::ComboBox && n.label() == Some("Device")
            })
            .expect("the Device box, named for a screen reader");
        assert_eq!(node.value(), Some("UT61B+"));
        assert_eq!(frame.focus, Some(*id), "focus did not return to the box");
    }

    /// Esc leaves the list with nothing picked.
    #[test]
    fn escape_closes_the_device_list_without_a_pick() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        run.app.settings.shared.device_family = "ut61eplus".to_string();
        open_device_list(&mut run);
        run.frame(press(egui::Key::ArrowDown));
        run.frame(press(egui::Key::Escape));
        let frame = run.frame(vec![]);
        assert_eq!(run.app.settings.shared.device_family, "ut61eplus");
        assert!(!device_list_open(&frame), "the list stayed open");
        assert!(
            focused_on(&frame, "UT61E+"),
            "focus did not return to the box"
        );
    }

    /// egui measures a popup once, as it opens, and only widens it after:
    /// narrowed under an open list, the list kept its four columns and ran
    /// off the window. It closes and re-opens at its new size instead.
    #[test]
    fn an_open_device_list_follows_the_window() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        open_device_list(&mut run);
        for width in [700.0, 1600.0, 700.0] {
            run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 1000.0));
            // A frame closed, one measuring, and the list is back.
            run.frame(vec![]);
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            assert!(
                device_list_open(&frame),
                "the list stayed closed at {width}"
            );
            for (_, n) in &frame.nodes {
                if let Some(b) = n.bounds() {
                    assert!(
                        b.x1 <= f64::from(width),
                        "{:?} runs off a {width} pt window: {b:?}",
                        n.label().or(n.value())
                    );
                }
            }
        }
    }

    /// Esc on the frame a resize re-shapes the list closes it for good: the
    /// re-measuring close used to queue a re-open behind the user's.
    #[test]
    fn escape_during_a_resize_keeps_the_device_list_closed() {
        let mut run = SettingsRun::new(1600.0, 1000.0);
        open_device_list(&mut run);
        run.screen = Rect::from_min_size(Pos2::ZERO, vec2(700.0, 1000.0));
        run.frame(press(egui::Key::Escape));
        for _ in 0..3 {
            let frame = run.frame(vec![]);
            assert!(!device_list_open(&frame), "the list opened again");
        }
    }

    /// The longest name, overridden, on the narrowest window: the box cuts
    /// its text short rather than running off the panel.
    #[test]
    fn a_long_device_name_stays_within_a_narrow_panel() {
        let width = 400.0;
        let mut run = SettingsRun::new(width, 900.0);
        run.app.settings.shared.device_family = "bm82x".to_string();
        run.app.settings.overrides.device_family = Some("auto".to_string());
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        let (_, node) = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Device"))
            .expect("the Device box");
        let bounds = node.bounds().expect("the box is laid out");
        let edge = frame.scrolled.inner_rect.right();
        assert!(
            bounds.x1 <= f64::from(edge),
            "the Device box runs off the {edge} pt panel: {bounds:?}"
        );
    }

    #[test]
    fn interval_chips_name_what_they_keep() {
        assert_eq!(interval_label(0), "Every reading");
        assert_eq!(interval_label(300), "300ms");
        assert_eq!(interval_label(1000), "1s");
        assert_eq!(interval_label(2000), "2s");
        assert_eq!(
            interval_tooltip(0),
            "Keep every reading the meter produces, at its own pace"
        );
        assert_eq!(
            interval_tooltip(1000),
            "At most one reading a second: the one nearest each tick"
        );
        assert_eq!(
            interval_tooltip(2000),
            "At most one reading every 2 s: the one nearest each tick"
        );
        assert_eq!(
            interval_tooltip(100),
            "At most one reading every 100 ms: the one nearest each tick"
        );
    }

    #[test]
    fn the_scroll_cap_follows_the_window_down_to_a_floor() {
        assert_eq!(
            settings_scroll_cap(900.0, 40.0),
            900.0 - 40.0 - SETTINGS_RESERVE
        );
        // Too short for the reserve: the rows keep their few lines instead.
        assert_eq!(settings_scroll_cap(220.0, 40.0), SETTINGS_MIN_HEIGHT);
    }

    /// A panel clips its content and never scrolls, so without the cap the
    /// top bar grew over the whole window and the central panel — and the
    /// reading with it — was squeezed out from below.
    #[test]
    fn a_short_window_keeps_the_settings_panel_off_the_rest_of_it() {
        let frame = settings_panel(400.0, 220.0);
        let panel = egui::PanelState::load(&frame.ctx, Id::new("top_bar")).expect("the panel ran");
        // The floor wins at this height, so the panel is the bar row, the
        // rows' cap, the two separators and the frame's margin — and the
        // rest of the window is left to the central panel.
        let bottom = panel.outer_rect.bottom();
        assert!(
            bottom <= frame.bar_bottom + SETTINGS_MIN_HEIGHT + 32.0,
            "settings panel took {bottom} pt of a 220 pt window (bar ends at {})",
            frame.bar_bottom
        );
        // And the rows do scroll: there is more than the cap can show.
        let inner = frame.scrolled.inner_rect;
        assert!(
            frame.scrolled.content_size.y > inner.height(),
            "rows {} pt tall fit a {} pt viewport, nothing to scroll",
            frame.scrolled.content_size.y,
            inner.height()
        );
    }

    /// `set_max_height` would have put the rows here: it unions the new
    /// bound with what was already placed and moves the cursor back to the
    /// top of the panel, so the rows were painted over the bar row.
    #[test]
    fn the_rows_start_below_the_bar_row() {
        for h in [220.0, 900.0] {
            let frame = settings_panel(400.0, h);
            let top = frame.scrolled.inner_rect.top();
            assert!(
                top >= frame.bar_bottom,
                "rows start at {top} pt, over a bar row ending at {} pt, in a {h} pt window",
                frame.bar_bottom
            );
        }
    }

    /// And when the window is tall enough, every row is on screen and the
    /// scroll area is invisible: it shrinks to the rows, so no bar appears
    /// and there is nothing to scroll.
    #[test]
    fn a_tall_window_shows_every_row_with_nothing_scrolled() {
        // Tall enough for every row, wrapped at this width.
        let frame = settings_panel(400.0, 1200.0);
        let panel = egui::PanelState::load(&frame.ctx, Id::new("top_bar")).expect("the panel ran");
        let height = panel.outer_rect.height();
        assert!(
            height > SETTINGS_MIN_HEIGHT + 40.0,
            "settings panel is only {height} pt tall in a 1200 pt window"
        );
        // Short of the cap, so the rows fit inside it.
        assert!(
            height < settings_scroll_cap(1200.0, 0.0),
            "settings panel is {height} pt tall, at the cap"
        );
        let inner = frame.scrolled.inner_rect;
        assert!(
            frame.scrolled.content_size.y <= inner.height() + 0.01,
            "rows {} pt tall overflow a {} pt viewport",
            frame.scrolled.content_size.y,
            inner.height()
        );
        assert_eq!(frame.scrolled.state.offset, egui::Vec2::ZERO);
    }

    /// egui scrolls to a focused widget only when assistive tech asks, so
    /// Tab walked below the fold with nothing on screen to show for it.
    #[test]
    fn tab_brings_the_focused_row_into_view() {
        let mut run = SettingsRun::new(400.0, 220.0);
        for _ in 0..3 {
            run.frame(vec![]);
        }
        let mut rows_focused = 0;
        let mut scrolled_down = false;
        for _ in 0..60 {
            run.frame(tab());
            // Focus moves within the Tab frame and the scroller is asked at
            // once, but egui animates the move and places the content a frame
            // behind the offset — a person sees it settle within two frames.
            run.frame(vec![]);
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            let Some(id) = run.ctx.memory(|m| m.focused()) else {
                continue;
            };
            let Some(response) = run.ctx.read_response(id) else {
                continue;
            };
            let inner = frame.scrolled.inner_rect;
            let content = Rect::from_min_size(
                inner.min - frame.scrolled.state.offset,
                frame.scrolled.content_size,
            );
            // Only the rows are the scroller's business; the bar row's
            // buttons are outside it, though the scrolled content's rect
            // reaches up over them once the rows are scrolled far enough.
            if !content.contains_rect(response.rect) || response.rect.bottom() <= frame.bar_bottom {
                continue;
            }
            rows_focused += 1;
            scrolled_down |= frame.scrolled.state.offset.y > 0.0;
            assert!(
                response.rect.top() >= inner.top() - 1.0
                    && response.rect.bottom() <= inner.bottom() + 1.0,
                "focused control at {:?} is outside the {:?} viewport",
                response.rect,
                inner
            );
        }
        assert!(
            rows_focused > 5,
            "Tab reached only {rows_focused} settings controls"
        );
        assert!(scrolled_down, "Tab never had to scroll the rows");
    }

    /// The scroller takes the panel's width, so its bar sits at the panel's
    /// edge — not at the widest row's, part-way across the window.
    /// Expanding **Customize colors** used to run the Graph swatches off the
    /// right edge of a narrow panel, and that overflow held every wrapped row
    /// at the overflowed width: the chip rows stopped reflowing with the
    /// window. Reported with the section opened in a wide window that was
    /// then narrowed, so the run does the same.
    #[test]
    fn the_expanded_colours_wrap_and_the_other_rows_keep_reflowing() {
        // Narrower than the Graph swatch row and than the Zoom row.
        let width = 700.0;
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        let header = node_bounds(&frame, "Customize colors");
        let centre = Pos2::new(
            ((header.x0 + header.x1) / 2.0) as f32,
            ((header.y0 + header.y1) / 2.0) as f32,
        );
        run.click(centre);
        // The body animates open; a second per frame has it fully open.
        run.frame(vec![]);
        run.frame(vec![]);
        run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 900.0));
        run.frame(vec![]);
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        // The section is open: its first swatch fits whatever the width.
        node_bounds(&frame, "Background");

        // A widget cut off at the edge leaves the AccessKit tree.
        let crosshair = frame
            .nodes
            .iter()
            .find_map(|(_, n)| {
                n.label()
                    .is_some_and(|l| l.starts_with("Crosshair"))
                    .then(|| n.bounds())
            })
            .flatten()
            .expect("the last Graph swatch is cut off by the panel edge");
        assert!(
            crosshair.x1 <= f64::from(width),
            "the last Graph swatch runs off the panel: {crosshair:?}"
        );
        let scrolled = &frame.scrolled;
        assert!(
            scrolled.content_size.x <= scrolled.inner_rect.width(),
            "the rows overflow the panel: content {} wide in {}",
            scrolled.content_size.x,
            scrolled.inner_rect.width()
        );
        for (_, node) in &frame.nodes {
            if let Some(b) = node.bounds() {
                assert!(
                    b.x1 <= f64::from(width),
                    "{:?} runs off the panel: {b:?}",
                    node.label().or(node.value())
                );
            }
        }
        // The Zoom row folded at the window, not at the swatch row.
        let first = node_bounds(&frame, "30%");
        let last = node_bounds(&frame, "(Ctrl+/- to adjust, Ctrl+0 = 100%)");
        assert!(
            last.y0 >= first.y1,
            "the Zoom row did not reflow: 30% at {first:?}, its hint at {last:?}"
        );
    }

    /// The Specifications fields follow the panel checkbox on its row, the
    /// word said once; and the group moves to the next line whole, wrapping
    /// within itself only when a line is too narrow for it.
    #[test]
    fn the_specifications_fields_wrap_as_one_group() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let wide = run.frame(vec![]);
        let graph = node_bounds(&wide, "Graph");
        let specs = node_bounds(&wide, "Specifications:");
        let notes = node_bounds(&wide, "Notes");
        assert_eq!(notes.y0, graph.y0, "the fields left the panel row");
        assert!(
            !wide
                .nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Specifications")),
            "\"Specifications\" is said twice"
        );
        // A window that just fits the row keeps it whole, so the group's
        // measure isn't over; one a little narrower moves the group down
        // whole; one narrower than the group splits it.
        let right_gap = 1200.0 - wide.scrolled.inner_rect.right();
        let group = (notes.x1 - specs.x0) as f32;
        for (width, same_row, split) in [
            ((notes.x1 as f32 + right_gap + 1.0).ceil(), true, false),
            (notes.x1 as f32 + right_gap - 20.0, false, false),
            (graph.x0 as f32 + group * 0.6 + right_gap, false, true),
        ] {
            run.screen = Rect::from_min_size(Pos2::ZERO, vec2(width, 900.0));
            run.frame(vec![]);
            let frame = run.frame(vec![]);
            let graph = node_bounds(&frame, "Graph");
            let specs = node_bounds(&frame, "Specifications:");
            let notes = node_bounds(&frame, "Notes");
            assert_eq!(specs.y0 == graph.y0, same_row, "at {width}: {specs:?}");
            assert_eq!(notes.y0 > specs.y0, split, "at {width}: {notes:?}");
            assert!(notes.x1 <= f64::from(width), "at {width}: {notes:?}");
        }
    }

    /// With the panel off, its fields are hidden and the checkbox loses the
    /// colon that led into them.
    #[test]
    fn the_specifications_fields_hide_with_the_panel() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.app.settings.show_specs = false;
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);
        node_bounds(&frame, "Specifications");
        assert!(
            !frame.nodes.iter().any(|(_, n)| n.label() == Some("Notes")),
            "the fields show with the panel off"
        );
    }

    /// A replay's frames come from the file whatever this row names — Connect
    /// re-opens the recording — so the row is pinned the way the clock flags
    /// pin it. Picking a meter used to save the choice and reconnect, and the
    /// session went on playing the file under the name of another meter.
    #[test]
    fn a_replay_pins_the_device_row() {
        let mut run = SettingsRun::new(1200.0, 900.0);
        run.app.replay = Some(crate::ReplaySource::fixture());
        run.ctx.enable_accesskit();
        run.frame(vec![]);
        let frame = run.frame(vec![]);

        let disabled = frame
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Device"))
            .map(|(_, n)| n.is_disabled())
            .expect("the Device row is drawn");
        assert!(disabled, "the list is still pickable during a replay");
        // And the row says how to get the choice back.
        node_bounds(&frame, "(restart without --replay to pick a meter)");
    }

    #[test]
    fn the_scroller_spans_the_panel() {
        let frame = settings_panel(400.0, 220.0);
        let right = frame.scrolled.inner_rect.right();
        assert!(right >= 400.0 - 24.0, "scroller ends at {right} pt of 400");
    }

    /// A context key offered in farads only.
    const ZERO_IN_FARADS: &[dmm_lib::protocol::MeterKey] = &[dmm_lib::protocol::MeterKey {
        command: "zero",
        label: "ZERO",
        hover: None,
        applies: |m| m.unit.ends_with('F'),
    }];

    /// The labels of the buttons the connected reading column draws for a
    /// reading in `unit`, on a meter with HOLD and [`ZERO_IN_FARADS`].
    fn chips_for(unit: &'static str) -> Vec<String> {
        let settings = Settings {
            // No acquisition thread: the connected state is set by hand.
            auto_connect: false,
            ..Settings::default()
        };
        let mut app = App::from_settings(settings, dmm_lib::Clock::real());
        app.connection.state = super::super::ConnectionState::Connected;
        app.connection.meter = Some(crate::app::ConnectedMeter {
            supported_commands: vec!["hold".to_string(), "zero".to_string()],
            meter_keys: dmm_lib::protocol::MeterKeys {
                functions: &[],
                context: ZERO_IN_FARADS,
            },
            ..crate::app::ConnectedMeter::test_fixture(None)
        });
        app.last_measurement = Some(dmm_lib::measurement::Measurement::test_fixture(
            dmm_lib::measurement::MeasuredValue::Normal(1.234),
            unit,
            dmm_lib::flags::StatusFlags::default(),
        ));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                app.show_reading_column(ui, super::super::layout::ContentLayout::Wide);
            });
        });
        // This harness renders without a painter.
        out.textures_delta.clear();
        out.platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, n)| n.role() == egui::accesskit::Role::Button)
            .filter_map(|(_, n)| n.label().map(str::to_string))
            .collect()
    }

    /// A context key joins the meter's buttons only while it applies to the
    /// reading: ZERO in capacitance, and nowhere else.
    #[test]
    fn a_context_key_shows_only_while_it_applies() {
        let farads = chips_for("nF");
        assert!(farads.iter().any(|l| l == "HOLD"), "{farads:?}");
        assert!(farads.iter().any(|l| l == "ZERO"), "{farads:?}");
        let volts = chips_for("V");
        assert!(volts.iter().any(|l| l == "HOLD"), "{volts:?}");
        assert!(!volts.iter().any(|l| l == "ZERO"), "{volts:?}");
    }

    /// A context key's tooltip names the key to press, or says how the
    /// meter does it when no key carries its label.
    #[test]
    fn a_context_key_tooltip_says_how_the_meter_does_it() {
        assert_eq!(
            super::context_key_hover(&ZERO_IN_FARADS[0]),
            "Press the meter's ZERO key"
        );
        let long_press = dmm_lib::protocol::MeterKey {
            hover: Some("Hold the meter's REL key"),
            ..ZERO_IN_FARADS[0]
        };
        assert_eq!(
            super::context_key_hover(&long_press),
            "Hold the meter's REL key"
        );
    }
}
