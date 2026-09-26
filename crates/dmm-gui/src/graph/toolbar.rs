//! The graph's toolbar: the **Plot:** and **Show:** chip groups, the analysis
//! toggles, and the text fields feeding them.

use eframe::egui::{self, Ui};

use super::{Graph, SeriesOption, TIME_WINDOWS};
use crate::a11y::ResponseA11yExt;
use crate::theme::ThemeColors;

/// Faint container for one labelled toolbar group (**Plot:**, **Show:**).
///
/// The two groups do different things — one picks the series the graph plots,
/// the other picks which sub-value traces are drawn over it — and inline chips
/// gave a first-time user nothing to tell them apart, or apart from the
/// analysis toggles beside them. Boxing each group makes that grouping
/// visible.
///
/// Both colours come from `Visuals`, so the box follows the theme. The fill is
/// `faint_bg_color`, well clear of `selection.bg_fill`: the frame is only a
/// container, and a selected chip inside it must still read as selected.
///
/// The border is `widgets.noninteractive.bg_stroke` — the same stroke egui
/// draws separators with, and the palette's Border colour. It is decorative
/// under the presets that leave that colour at egui's own grey, landing around
/// 1.6:1 against the panel in dark mode and 1.8:1 in light, under WCAG's 3:1
/// threshold for graphical elements; the High Contrast preset pins a border
/// that clears 3:1. Nothing depends on seeing it either way: the grouping is
/// carried by the **Plot:**/**Show:** caption text inside each box, and every
/// chip states its own group in its accessible name.
fn group_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::new()
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
}

/// Caption naming one toolbar group, sitting on the same line as its chips.
///
/// Body size, not `.small()`: these name a group of controls rather than
/// annotating one, so they sit at the same size as the labels inside it and
/// are set apart by weight and colour instead.
///
/// `add_sized` at `interact_size.y` is what keeps the caption's text level
/// with the chip text, and it is load-bearing rather than cosmetic. A plain
/// `ui.label` here rendered ~2 logical px high. The group frame's content ui
/// inherits the toolbar's `horizontal_wrapped` layout, and when the caption is
/// added the row is only as tall as the space the frame started with.
/// `Layout::next_frame_ignore_wrap` sizes each widget to
/// `max(child_size.y, cursor.height())` and then force-translates it down to
/// the row top ("we always want to expand down, or we will overlap the row
/// above") — so the caption and the chips are both top-aligned, and egui never
/// re-centres the caption once the taller chips grow the row. A
/// `selectable_label` is a `Button`: it takes `interact_size.y` and centres
/// its text in that box, while a bare label's text fills its own shorter box
/// from the top. Giving the caption the chips' height and letting `add_sized`
/// centre it inside closes the gap.
///
/// A plain `Label`, deliberately: the caption must stay `Role::Label` for
/// AccessKit. Sizing it as a button would line it up too, but would announce a
/// control that isn't one.
fn group_caption(ui: &mut Ui, text: &str) {
    let text = egui::RichText::new(text)
        .strong()
        .color(ui.visuals().weak_text_color());
    // Zero width: only the height is being imposed, the label keeps its own.
    let height = ui.spacing().interact_size.y;
    ui.add_sized(egui::vec2(0.0, height), egui::Label::new(text));
}

/// Accessible name for one **Plot:** chip. `None` is the main reading.
///
/// The chips are `selectable_label`s whose visible text is just the
/// sub-value's name, so a screen reader would otherwise announce a **Plot:**
/// chip and a **Show:** chip identically ("T2, button"). egui 0.36 never
/// calls AccessKit's `set_description`, so the hover text cannot carry the
/// distinction — it has to be in the name.
pub(super) fn series_chip_label(series: Option<&str>) -> String {
    match series {
        Some(label) => format!("Plot {label}"),
        None => "Plot main reading".to_string(),
    }
}

/// Accessible name for one **Show:** chip. See [`series_chip_label`].
pub(super) fn overlay_chip_label(label: &str) -> String {
    format!("Show {label} trace")
}

/// One toolbar chip: a `selectable_label` showing `on`'s state, with hover
/// text, reporting whether it was clicked and changing nothing itself.
///
/// What the click does is the caller's, so that a chip whose flip carries
/// side effects can run the same method its keyboard shortcut does.
fn chip(ui: &mut Ui, on: bool, label: &str, hover: &str) -> bool {
    ui.selectable_label(on, label)
        .on_hover_text(hover)
        .clicked()
}

/// A [`chip`] over a plain flag, flipped in place when clicked.
///
/// Returns whether it was clicked, so a caller with extra work to do on the
/// transition can see it — the flag is already flipped by then, so such a
/// caller tests the *new* state.
fn toggle_chip(ui: &mut Ui, on: &mut bool, label: &str, hover: &str) -> bool {
    let clicked = chip(ui, *on, label, hover);
    if clicked {
        *on = !*on;
    }
    clicked
}

/// One toolbar text entry: a fixed-width single-line edit with hint and hover
/// text. Parsing stays at the call site — each field accepts a different
/// shape of value — and so does the response, which the reference field also
/// takes the focus from.
fn text_field(
    ui: &mut Ui,
    text: &mut String,
    width: f32,
    hint: &str,
    hover: &str,
) -> egui::Response {
    let resp = ui.add(
        egui::TextEdit::singleline(text)
            .desired_width(width)
            .hint_text(hint),
    );
    // egui's own focused frame is invisible under a pinned Accent; the ring
    // is the field's only keyboard cue then (`a11y::paint_focus_ring`).
    crate::a11y::paint_focus_ring(ui, &resp);
    resp.on_hover_text(hover)
}

impl Graph {
    /// Whether the toolbar's series row has anything to show: a sub-value to
    /// pick from (**Plot:**) or a trace to draw beside the plotted series
    /// (**Show:**). False for single-display meters, which keep the two-row
    /// toolbar they had before sub-values existed.
    pub(super) fn has_series_row(&self) -> bool {
        !self.series_options.is_empty() || !self.overlays.is_empty()
    }

    /// Render the toolbar. Row 1: time presets, LIVE, Y-axis, Reset Zoom.
    /// Row 2, present only for meters that send sub-values: the boxed
    /// **Plot:** series selector and **Show:** trace toggles. Row 3: the
    /// analysis overlays (Mean, Min/Max, Ref, Cursors).
    ///
    /// The series controls get a row of their own rather than sharing one
    /// with the analysis toggles: inline, nothing distinguished the two kinds
    /// of control for a user who didn't already know them. The rows now read
    /// view → what is plotted → what is drawn over it. Each uses
    /// `horizontal_wrapped` so items wrap instead of clipping.
    pub fn show_toolbar(&mut self, ui: &mut Ui, tc: &ThemeColors) {
        // Row 1: time windows + LIVE + Y-axis
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for &(secs, label) in TIME_WINDOWS {
                let tooltip = format!("Show the last {label} of samples ([ / ] to cycle)");
                if ui
                    .selectable_label((self.time_window_secs - secs).abs() < 0.1, label)
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    self.time_window_secs = secs;
                }
            }

            ui.add_space(6.0);

            // On: the selected fill and text colour come from `selected`,
            // like the presets beside it. Off: dimmed text in a plain frame,
            // so it still reads as a button.
            let live_text = egui::RichText::new("LIVE").small();
            let live_text = if self.live {
                live_text
            } else {
                live_text.color(ui.visuals().weak_text_color())
            };
            let live_resp = ui
                .add(egui::Button::new(live_text).selected(self.live))
                .on_hover_text(
                    "Auto-follow the newest samples — off while panning (End to jump back)",
                );
            if live_resp.clicked() {
                self.live = !self.live;
            }

            ui.add_space(6.0);

            let (y_label, y_tooltip) = if self.y_axis_fixed {
                (
                    "Y:Fixed",
                    "Using fixed Y-axis bounds — click to auto-scale to visible data",
                )
            } else {
                (
                    "Y:Auto",
                    "Auto-scaling Y to visible data — click to enter fixed bounds",
                )
            };
            // The chip has already flipped the flag, so `self.y_axis_fixed`
            // below reads "fixed mode was just switched on".
            if toggle_chip(ui, &mut self.y_axis_fixed, y_label, y_tooltip)
                && self.y_axis_fixed
                && !self.y_user_set
            {
                let (view_min, view_max) = self.view_bounds();
                // Snapshot with the overlays included, so pinning the axis
                // doesn't jump the view the moment Y:Fixed is pressed.
                if let Some((y_lo, y_hi)) = self.y_range_for_view_auto(view_min, view_max, true) {
                    self.y_min.set(y_lo);
                    self.y_max.set(y_hi);
                }
            }
            if self.y_axis_fixed {
                let field_width = 50.0;
                let changed_min = text_field(
                    ui,
                    self.y_min.text_mut(),
                    field_width,
                    "Y axis minimum",
                    "Lower bound of the fixed Y axis",
                )
                .changed();
                ui.label(
                    egui::RichText::new("..")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                let changed_max = text_field(
                    ui,
                    self.y_max.text_mut(),
                    field_width,
                    "Y axis maximum",
                    "Upper bound of the fixed Y axis",
                )
                .changed();
                if changed_min && self.y_min.parse() {
                    self.y_user_set = true;
                }
                if changed_max && self.y_max.parse() {
                    self.y_user_set = true;
                }
            }

            ui.add_space(6.0);

            let zoomed = self.is_view_zoomed();
            if ui
                .add_enabled(zoomed, egui::Button::new("Reset Zoom"))
                .on_hover_text("Return to live follow and auto Y (double-click graph)")
                .on_disabled_hover_text("Already at the default live + auto-Y view")
                .clicked()
            {
                self.reset_view();
            }
        });

        // Row 2: what is plotted, and what is drawn beside it. Skipped
        // entirely for single-display meters, which keep the two-row toolbar.
        if self.has_series_row() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                self.show_series_selector(ui);
                self.show_overlay_toggles(ui);
            });
        }

        // Row 3: analysis overlays
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            toggle_chip(
                ui,
                &mut self.show_mean,
                "Mean",
                "Draw a horizontal line at the mean of visible samples (M)",
            );
            toggle_chip(
                ui,
                &mut self.show_envelope,
                "Min/Max",
                "Draw a shaded band between the rolling min and max (X)",
            );
            if self.show_envelope {
                let changed = text_field(
                    ui,
                    self.envelope_window.text_mut(),
                    30.0,
                    "Min/Max window, seconds",
                    "Window size (seconds) used to compute the Min/Max envelope",
                )
                .changed();
                if changed {
                    // A zero or negative window would bucket every sample
                    // together, so it is left as a draft.
                    self.envelope_window.parse_if(|v| v > 0.0);
                }
                ui.label(
                    egui::RichText::new("s")
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            if chip(
                ui,
                self.show_ref_line,
                "Ref",
                "Draw horizontal reference lines at the values in the next field (R)",
            ) {
                self.toggle_ref_lines();
            }
            if self.show_ref_line {
                let resp = text_field(
                    ui,
                    self.ref_lines.text_mut(),
                    80.0,
                    "Reference values",
                    "Reference values, comma- or semicolon-separated (e.g. 3.3, 5, 12)",
                );
                if std::mem::take(&mut self.focus_ref_field) {
                    resp.request_focus();
                }
                if resp.changed() {
                    self.ref_lines.parse();
                }
                toggle_chip(
                    ui,
                    &mut self.show_crossings,
                    "Triggers",
                    "Mark the points where the signal crosses a reference line (T)",
                );
            }
            if chip(
                ui,
                self.cursors_active,
                "Cursors",
                "Click the graph to place two cursors and read Δt / Δv / integral (C)",
            ) {
                self.toggle_cursors();
            }
            if self.cursors_active {
                if let (Some(ta), Some(tb)) = (self.cursor_a, self.cursor_b) {
                    let dt = (tb - ta).abs();
                    let va = self.nearest_point(ta).map(|(_, v)| v);
                    let vb = self.nearest_point(tb).map(|(_, v)| v);
                    let dv = match (va, vb) {
                        (Some(a), Some(b)) => format!("{:.4}", (b - a).abs()),
                        _ => crate::NO_DATA.to_string(),
                    };
                    let unit = &self.current_unit;
                    let delta_color = tc.graph_cursor_delta();

                    let integral_str = self
                        .cursor_integral(ta, tb)
                        .and_then(|raw| dmm_lib::stats::integral_display(raw, unit))
                        .map(|(value, disp_unit)| format!("  \u{222b}={value:.4} {disp_unit}"))
                        .unwrap_or_default();

                    ui.label(
                        egui::RichText::new(format!(
                            "\u{0394}T={dt:.2} s  \u{0394}={dv} {unit}{integral_str}"
                        ))
                        .color(delta_color)
                        .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("click graph to place cursors")
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            }
        });
    }

    /// Chips choosing which series the graph plots: the meter's main reading
    /// or any sub-value it is currently sending.
    ///
    /// Only rendered when the meter sends sub-values, so single-display
    /// meters see the toolbar exactly as before. `selectable_label` rather
    /// than a combo box because that is the toolbar's idiom throughout — and
    /// with at most five choices the chips stay readable.
    fn show_series_selector(&mut self, ui: &mut Ui) {
        if self.series_options.is_empty() {
            return;
        }

        // `None` = the main reading, `Some(i)` = `series_options[i]`. Applied
        // after the loop so the click handler doesn't need a mutable borrow
        // while the option list is being read.
        let mut choice: Option<Option<usize>> = None;

        let frame = group_frame(ui);
        frame.show(ui, |ui| {
            group_caption(ui, "Plot:");

            // `Role::RadioButton` rather than the default button role: exactly
            // one chip is selected at a time, and egui maps its own radio
            // widgets to that role with a toggled state, so AT handling of the
            // pairing is proven.
            let main_selected = self.selected_series.is_none();
            if ui
                .selectable_label(main_selected, "Main")
                .on_hover_text("Plot the meter's main reading")
                .a11y_label(&series_chip_label(None))
                .a11y_role(egui::accesskit::Role::RadioButton)
                .clicked()
            {
                choice = Some(None);
            }

            for (i, SeriesOption { label, unit, .. }) in self.series_options.iter().enumerate() {
                let selected = self.selected_series.as_deref() == Some(label.as_str());
                if ui
                    .selectable_label(selected, label.as_str())
                    .on_hover_text(format!(
                        "Plot the {label} sub-value ({unit}) \u{2014} a different unit restarts the graph"
                    ))
                    .a11y_label(&series_chip_label(Some(label)))
                    .a11y_role(egui::accesskit::Role::RadioButton)
                    .clicked()
                {
                    choice = Some(Some(i));
                }
            }
        });

        // The switch takes effect at the next sample: `push_sample` sees the
        // new series and clears through the same branch a mode change uses,
        // releasing the pinned Y range, the cursors and any bbox state.
        match choice {
            Some(None) => self.selected_series = None,
            Some(Some(i)) => self.selected_series = Some(self.series_options[i].label.clone()),
            None => {}
        }

        ui.add_space(6.0);
    }

    /// Chips choosing which same-unit sub-value traces are drawn beside the
    /// plotted series.
    ///
    /// The plot key is a key, not a control (`Plot::reset()` wipes egui_plot's
    /// own legend state every frame), so the show/hide affordance lives here
    /// in the toolbar with the rest of them. Hiding a trace stops it being
    /// drawn but not recorded — turning it back on brings its history with it.
    fn show_overlay_toggles(&mut self, ui: &mut Ui) {
        if self.overlays.is_empty() {
            return;
        }

        // Applied after the loop: the click handler would otherwise need a
        // mutable borrow while the overlay list is being read.
        let mut toggled: Option<String> = None;

        let frame = group_frame(ui);
        frame.show(ui, |ui| {
            group_caption(ui, "Show:");

            for o in &self.overlays {
                let label = o.label.as_str();
                let shown = !self.hidden_overlays.contains(&o.label);
                let hover = if shown {
                    format!("Hide the {label} trace")
                } else {
                    format!("Draw the {label} trace beside the plotted series")
                };
                // Toggle buttons, unlike the mutually exclusive **Plot:**
                // chips — but with the same naming problem, so the group goes
                // in the accessible name here too.
                if ui
                    .selectable_label(shown, label)
                    .on_hover_text(hover)
                    .a11y_label(&overlay_chip_label(label))
                    .clicked()
                {
                    toggled = Some(o.label.clone());
                }
            }
        });

        if let Some(label) = toggled {
            self.toggle_overlay_hidden(label);
        }

        ui.add_space(6.0);
    }

    /// Flip the reference lines, taking the caret into the values field when
    /// they come on: the field is where the lines are actually chosen, and
    /// on an empty one the chip alone draws nothing.
    ///
    /// Shared by the **Ref** chip and its key, as [`Graph::toggle_cursors`]
    /// is — the two paths would otherwise be free to drift apart.
    pub(super) fn toggle_ref_lines(&mut self) {
        self.show_ref_line = !self.show_ref_line;
        self.focus_ref_field = self.show_ref_line;
    }

    /// Flip the measurement cursors, dropping any placed cursor when they go
    /// off so that switching them back on starts a fresh measurement.
    pub(super) fn toggle_cursors(&mut self) {
        self.cursors_active = !self.cursors_active;
        if !self.cursors_active {
            self.cursor_a = None;
            self.cursor_b = None;
            self.cursor_next_is_b = false;
        }
    }

    /// Flip one sub-value trace between drawn and hidden.
    pub(super) fn toggle_overlay_hidden(&mut self, label: String) {
        if !self.hidden_overlays.remove(&label) {
            self.hidden_overlays.insert(label);
        }
    }

    /// Switch one sub-value trace off, as if its **Show:** chip had been
    /// clicked off. The chip still lists it, unlit, so the user can turn it
    /// back on; unlike [`Graph::toggle_overlay_hidden`] this is idempotent,
    /// which is what a caller reacting to a state change (rather than to a
    /// click) needs.
    pub(crate) fn hide_overlay(&mut self, label: &str) {
        self.hidden_overlays.insert(label.to_string());
    }
}
