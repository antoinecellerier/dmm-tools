//! The top bar: the device label and connection controls on the left, the
//! status landmark in the middle, and the version / Help / shortcuts /
//! settings group on the right, wrapped to a second row when it doesn't fit.

use eframe::egui::{self, RichText, Ui};

use super::{App, ConnectionState};
use crate::a11y::{ResponseA11yExt, UiA11yExt};

impl App {
    /// Render the top bar: controls on the left, info/links on the right.
    ///
    /// Adaptive layout: when the window is wide enough, everything fits on
    /// a single row. When it isn't (narrow window or high zoom), the right
    /// group (version, Help, ?, settings) wraps to a second row to avoid
    /// clipping. The decision uses cached widget widths from the previous
    /// frame (egui Discussion #3468 pattern) — converges in one frame,
    /// imperceptible to the user.
    ///
    /// The right group is rendered via `show_top_bar_right` in left-to-right
    /// order so that Tab key navigation follows visual reading order
    /// (Help → ? → ⚙) rather than the reverse.
    pub(super) fn show_top_bar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Explicit id_salt so the Toolbar landmark stays stable across
        // frames. egui's default scope id is derived from a running
        // `next_auto_id_salt` counter; if any sibling above ever changes
        // shape, the salt shifts and AT loses track of the landmark.
        ui.landmark("top_bar_landmark", egui::accesskit::Role::Toolbar, |ui| {
            self.show_top_bar_inner(ui, ctx);
        });
    }

    fn show_top_bar_inner(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let green = tc.status_ok();
        let orange = tc.status_warning();
        let gray = tc.status_inactive();

        // Cache left/right group widths from the previous frame to decide
        // whether both fit on one row.
        let left_id = egui::Id::new("top_bar_left_w");
        let right_id = egui::Id::new("top_bar_right_w");
        let cached_left: f32 = ui.data(|d| d.get_temp(left_id)).unwrap_or(300.0);
        let cached_right: f32 = ui.data(|d| d.get_temp(right_id)).unwrap_or(200.0);
        let spacing = ui.spacing().item_spacing.x;
        let one_row = cached_left + cached_right + spacing < ui.available_width();

        // Row 1: device label, action buttons, status indicator
        ui.horizontal(|ui| {
            let left_start = ui.cursor().left();

            // `selected_device()`, not `find_device`: it accepts the aliases
            // the CLI and settings file accept, and falls back to the same
            // default the connection will actually open — so the label always
            // names the meter this session will talk to.
            let device_label = self.selected_device().display_name;
            ui.label(RichText::new(device_label).strong());
            ui.separator();

            match &self.connection_state {
                ConnectionState::Disconnected => {
                    if ui
                        .button("Connect")
                        .on_hover_text("Open USB connection to the selected meter (Ctrl+O)")
                        .clicked()
                    {
                        self.connect(ctx);
                    }
                }
                ConnectionState::Connected => {
                    if ui
                        .button("Disconnect")
                        .on_hover_text("Close the active meter connection (Ctrl+O)")
                        .clicked()
                    {
                        self.disconnect();
                    }
                    let (pause_label, pause_tooltip) = if self.paused {
                        ("\u{25B6} Resume", "Resume acquisition (Space)")
                    } else {
                        (
                            "\u{23F8} Pause",
                            "Halt acquisition — stops recording and graph updates (Space)",
                        )
                    };
                    if ui
                        .button(pause_label)
                        .on_hover_text(pause_tooltip)
                        .clicked()
                    {
                        self.set_paused(!self.paused);
                    }
                    if ui
                        .button("Clear")
                        .on_hover_text("Clear graph history and statistics (Ctrl+L)")
                        .clicked()
                    {
                        self.clear_session();
                    }
                }
                ConnectionState::Reconnecting => {
                    let label = self.reconnecting_label();
                    let hover = if let Some(err) = &self.reconnect_last_error {
                        format!(
                            "Retrying the connection automatically — click Disconnect to stop.\nLast error: {err}",
                        )
                    } else {
                        "Retrying the connection automatically — click Disconnect to stop"
                            .to_string()
                    };
                    ui.add_enabled(false, egui::Button::new(label))
                        .on_disabled_hover_text(hover);
                    // The reconnect loop retries every 2 s indefinitely, so
                    // this is the user's only way out short of killing the
                    // app — the status tooltip above tells them to click it.
                    if ui
                        .button("Disconnect")
                        .on_hover_text("Stop retrying and close the connection (Ctrl+O)")
                        .clicked()
                    {
                        self.disconnect();
                    }
                }
            }

            let (dot_color, status_text) = match &self.connection_state {
                ConnectionState::Connected => {
                    let name = self.device_name.as_deref().unwrap_or("Connected");
                    if self.paused {
                        (orange, format!("{name} (paused)"))
                    } else {
                        (green, name.to_string())
                    }
                }
                ConnectionState::Disconnected => (gray, "Disconnected".to_string()),
                ConnectionState::Reconnecting => {
                    let label = self.reconnecting_label();
                    (orange, label)
                }
            };

            // Group status indicators (dot, label, experimental badge, toast)
            // so the whole region exposes a Role::Status landmark to AT.
            //
            // Explicit id_salt: this scope sits inside a horizontal whose
            // sibling layout changes whenever the connection state flips
            // (Connect button vs. Disconnect+Pause+Clear). Without an
            // explicit salt, the auto-derived scope id flips on every
            // state transition and AT loses the Status landmark.
            ui.landmark("status_landmark", egui::accesskit::Role::Status, |ui| {
                // Decorative status dot — not interactive or focusable.
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, dot_color);
                ui.label(RichText::new(&status_text).small());

                // Show EXPERIMENTAL badge based on connected state or selected device.
                let profile = &self.selected_profile;
                let is_experimental = if self.connection_state == ConnectionState::Connected {
                    self.experimental
                } else {
                    profile.stability == dmm_lib::protocol::Stability::Experimental
                };
                if is_experimental {
                    let url = if self.connection_state == ConnectionState::Connected
                        && !self.feedback_url.is_empty()
                    {
                        self.feedback_url.clone()
                    } else {
                        profile.feedback_url()
                    };
                    ui.hyperlink_to(
                        RichText::new("EXPERIMENTAL").small().strong().color(orange),
                        url,
                    )
                    .on_hover_text(format!(
                        "{} support is experimental \u{2014} click to report feedback",
                        profile.model_name
                    ));
                }

                // Toast inline on this row
                if let Some((msg, is_error, _)) = &self.toast {
                    let color = if *is_error { tc.status_error() } else { green };
                    ui.label(RichText::new(msg).small().color(color));
                }
            });

            let left_width = ui.min_rect().right() - left_start;
            ui.data_mut(|d| d.insert_temp(left_id, left_width));

            // If wide enough, render right-side items on the same row
            if one_row {
                self.show_top_bar_right(ui, right_id);
            }
        });

        // If not wide enough, render right-side items on a second row
        if !one_row {
            ui.horizontal(|ui| {
                self.show_top_bar_right(ui, right_id);
            });
        }
    }

    /// Right side of the top bar: version label, Help/GitHub link, keyboard
    /// shortcut help button, and settings button.
    ///
    /// Items are added left-to-right so that egui's Tab order matches the
    /// visual reading direction. A cached-width spacer right-aligns the
    /// group without needing a right-to-left layout (which would reverse
    /// tab order). The cached width comes from the previous frame and
    /// self-corrects in one frame.
    fn show_top_bar_right(&mut self, ui: &mut Ui, cache_id: egui::Id) {
        let cached_width: f32 = ui.data(|d| d.get_temp(cache_id)).unwrap_or(200.0);
        let spacer = (ui.available_width() - cached_width).max(0.0);
        ui.add_space(spacer);
        let before = ui.cursor().left();

        // A Button (not Label) so the AccessKit role is Button, not Label.
        // For Role::Label, egui maps the text to AccessKit's `value` field —
        // `set_label` overrides via `accesskit_node_builder` are ignored, and
        // screen readers read out the literal version string instead of
        // "Show release notes". `frame_when_inactive(false)` keeps the
        // resting visual identical to a label while still painting hover and
        // focus backgrounds when the user mouses over or Tab-focuses it.
        let version_resp = ui
            .add(
                egui::Button::new(
                    RichText::new(crate::version_label())
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .frame_when_inactive(false),
            )
            .a11y_label("Show release notes");
        if version_resp.clicked() {
            if self.whats_new_open {
                self.whats_new_open = false;
            } else {
                self.whats_new_opener = Some(version_resp.id);
                self.open_whats_new();
            }
        }
        version_resp
            .on_hover_text("Show What's New — release notes for this version")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.hyperlink_to(
            "Help / GitHub",
            "https://github.com/antoinecellerier/dmm-tools",
        )
        .on_hover_text("Open the dmm-tools project page on GitHub");
        let shortcuts_btn = ui
            .button("?")
            .on_hover_text("Show keyboard shortcuts and mouse gestures (?)")
            .a11y_label("Keyboard shortcuts and mouse gestures");
        if shortcuts_btn.clicked() {
            let will_open = !self.shortcut_help_open;
            self.shortcut_help_open = will_open;
            if will_open {
                self.shortcut_help_opener = Some(shortcuts_btn.id);
                self.shortcut_help_focus_pending = true;
            }
        }

        let settings_btn = ui
            .button("\u{2699}")
            .on_hover_text("Show or hide the settings panel")
            .a11y_label("Settings");
        if settings_btn.clicked() {
            self.settings_open = !self.settings_open;
        }

        let actual_width = ui.min_rect().right() - before;
        ui.data_mut(|d| d.insert_temp(cache_id, actual_width));
    }
}
