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
    /// imperceptible to the user. Narrower still, the status drops the link
    /// it names, which is the one thing on the row the hover can carry
    /// instead — and which the window's minimum width is computed without.
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
        let link_id = egui::Id::new("top_bar_link_w");
        let cached_left: f32 = ui.data(|d| d.get_temp(left_id)).unwrap_or(300.0);
        let cached_right: f32 = ui.data(|d| d.get_temp(right_id)).unwrap_or(200.0);
        // What the link suffix would add. Cached apart from the row's own
        // width so the row can be measured as if the link were not there —
        // that width is what the window's minimum comes from, and naming the
        // link must not stop the window being made as narrow as before.
        let cached_link: f32 = ui.data(|d| d.get_temp(link_id)).unwrap_or(0.0);
        let spacing = ui.spacing().item_spacing.x;
        let available = ui.available_width();
        // The link is the first thing the window edge takes: it names
        // nothing the user has to act on, and the hover spells it out
        // whether or not the bar has room for it.
        let show_link = fits_with_link(cached_left, cached_link, available);
        let link_shown_w = if show_link { cached_link } else { 0.0 };
        let one_row = cached_left + link_shown_w + cached_right + spacing < available;

        // Row 1: device label, action buttons, status indicator
        ui.horizontal(|ui| {
            let left_start = ui.cursor().left();

            // The meter this session is talking to: the one picked, else the
            // one detection found. Under Auto-detect with nothing connected
            // there is no meter to name yet — the label says what will happen
            // instead, and is replaced by the model as soon as one answers.
            let device_label = self
                .active_device()
                .map_or("Auto-detect", |d| d.display_name);
            ui.label(RichText::new(device_label).strong());
            ui.separator();

            match &self.connection.state {
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
                    let (pause_label, pause_tooltip) = if self.connection.paused {
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
                        self.set_paused(!self.connection.paused);
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
                    let hover = if let Some(err) = &self.connection.reconnect_last_error {
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

            // Whether the suffix ends up on the bar this frame, which is what
            // the row's width has to be corrected by below.
            let mut link_drawn = false;
            let (dot_color, status_text) = match &self.connection.state {
                ConnectionState::Connected => {
                    let name = self.connection.device_name.as_deref().unwrap_or("Connected");
                    let link = self.connection.link.filter(|_| show_link);
                    link_drawn = link.is_some();
                    let text = connected_status(name, link, self.connection.paused);
                    if self.connection.paused {
                        (orange, text)
                    } else {
                        (green, text)
                    }
                }
                ConnectionState::Disconnected => (gray, "Disconnected".to_string()),
                ConnectionState::Reconnecting => {
                    let label = self.reconnecting_label();
                    (orange, label)
                }
            };

            // Group status indicators (dot, label, experimental badge) so the
            // whole region exposes a Role::Status landmark to AT.
            //
            // Explicit id_salt: this scope sits inside a horizontal whose
            // sibling layout changes whenever the connection state flips
            // (Connect button vs. Disconnect+Pause+Clear). Without an
            // explicit salt, the auto-derived scope id flips on every
            // state transition and AT loses the Status landmark.
            ui.landmark("status_landmark", egui::accesskit::Role::Status, |ui| {
                // What the bar had no room to say, plus — a replayed session
                // being a meter session in every visible way, which is the
                // point — the file it is coming from, said where a human can
                // find it and a screenshot cannot.
                let mut hover: Vec<String> = Vec::new();
                if self.connection.state == ConnectionState::Connected {
                    hover.push(link_tooltip(self.connection.link, self.replay.is_some()));
                }
                if let Some(source) = &self.replay {
                    hover.push(format!("Replaying {}", source.path.display()));
                }
                let hover = hover.join("\n");

                // Status dot — decorative, so not focusable, but it hovers
                // with the text beside it rather than being a dead spot in
                // the middle of the status.
                let (rect, dot) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, dot_color);
                let status = ui.label(RichText::new(&status_text).small());
                if !hover.is_empty() {
                    dot.on_hover_text(&hover);
                    status.on_hover_text(&hover);
                }

                // The EXPERIMENTAL badge names a protocol, so it comes from
                // the connected one where there is one — under Auto-detect
                // that is the only thing that names a meter at all — and from
                // the selected entry's profile otherwise.
                let badge = if self.connection.state == ConnectionState::Connected {
                    (!self.connection.stability.is_verified()).then(|| {
                        (
                            self.connection.model_name.clone(),
                            self.connection.stability,
                            self.connection.feedback_url.clone(),
                        )
                    })
                } else {
                    self.selected_profile
                        .as_ref()
                        .filter(|p| !p.stability.is_verified())
                        .map(|p| (p.model_name.to_string(), p.stability, p.feedback_url()))
                };
                if let Some((model_name, stability, url)) = badge {
                    ui.hyperlink_to(
                        RichText::new("EXPERIMENTAL").small().strong().color(orange),
                        url,
                    )
                    .on_hover_text(format!(
                        "{} Click to report feedback.",
                        dmm_lib::binary_help::experimental_warning(&model_name, stability)
                    ));
                }
            });

            // The link is measured rather than read off the drawn row, so
            // both numbers mean the same thing whether or not it is on the
            // bar: what the row needs without the link, and what the link
            // would add. Only the first reaches the window's minimum size.
            let link_w = if self.connection.state == ConnectionState::Connected {
                link_suffix_width(ui, self.connection.link)
            } else {
                0.0
            };
            let drawn = if link_drawn { link_w } else { 0.0 };
            let left_width = ui.min_rect().right() - left_start - drawn;
            ui.data_mut(|d| {
                d.insert_temp(left_id, left_width);
                d.insert_temp(link_id, link_w);
            });

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
            if self.whats_new.open {
                self.whats_new.open = false;
            } else {
                self.whats_new.opener = Some(version_resp.id);
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
            .on_hover_text("Show keyboard shortcuts and mouse gestures (? or F1)")
            .a11y_label("Keyboard shortcuts and mouse gestures");
        if shortcuts_btn.clicked() {
            let will_open = !self.shortcut_help.open;
            self.shortcut_help.open = will_open;
            if will_open {
                self.shortcut_help.opener = Some(shortcuts_btn.id);
                self.shortcut_help.focus_pending = true;
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

/// The text beside the status dot while a meter is connected: the meter, the
/// link it answers over, and whether acquisition is halted.
///
/// The link answers the question a second meter or a second cable raises —
/// which of them this window is watching. A replay names the link its
/// recording was made over; the mock is on none, and reads as the meter alone.
/// `link` is `None` for a row too narrow to hold it as well — the hover says
/// it either way.
fn connected_status(name: &str, link: Option<&str>, paused: bool) -> String {
    let mut text = name.to_string();
    if let Some(link) = link {
        text.push_str(&link_suffix(link));
    }
    if paused {
        text.push_str(" (paused)");
    }
    text
}

/// What the link adds to the status text.
fn link_suffix(link: &str) -> String {
    format!(" \u{b7} {link}")
}

/// What that suffix would add to the row's width, in points.
///
/// Laid out rather than read off the drawn row, so the answer is the same
/// whether or not the link is on the bar this frame — the row's own width
/// then stays what it was before the link existed, and with it the narrowest
/// the window may be made.
fn link_suffix_width(ui: &Ui, link: Option<&str>) -> f32 {
    let Some(link) = link else {
        return 0.0;
    };
    egui::WidgetText::from(RichText::new(link_suffix(link)).small())
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        )
        .size()
        .x
}

/// Whether the status row has room for the link as well.
///
/// `row` is what the row needs without it, `link` what it would add, both as
/// the previous frame measured them.
fn fits_with_link(row: f32, link: f32, available: f32) -> bool {
    row + link <= available
}

/// What the status hover says about the link, spelled out in full.
///
/// The bar has the short name, and a narrow window has none at all, so this
/// is the one place the link is always named. `replayed` distinguishes a live
/// link from the one a recording was made over — the rest of the window is
/// deliberately identical for the two.
fn link_tooltip(link: Option<&str>, replayed: bool) -> String {
    match (link, replayed) {
        (Some(link), false) => {
            format!(
                "Connected over the {}",
                dmm_lib::binary_help::full_link_name(link)
            )
        }
        (Some(link), true) => {
            format!(
                "Recorded over the {}",
                dmm_lib::binary_help::full_link_name(link)
            )
        }
        // Nothing is on the far end of a mock session, and a recording whose
        // file names a link this build does not know says only that much.
        (None, false) => "Mock meter, no link".to_string(),
        (None, true) => "Recorded over an unnamed link".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{connected_status, fits_with_link, link_tooltip};

    #[test]
    fn the_status_names_the_link_where_there_is_one() {
        assert_eq!(
            connected_status("UT61E+", Some("Bluetooth"), false),
            "UT61E+ \u{b7} Bluetooth"
        );
        assert_eq!(connected_status("UT61E+", None, false), "UT61E+");
        assert_eq!(
            connected_status("UT61E+", Some("USB cable"), true),
            "UT61E+ \u{b7} USB cable (paused)"
        );
    }

    /// A row with no room for the link drops it and keeps everything else —
    /// including the paused marker, which says whether readings are arriving.
    #[test]
    fn a_narrow_row_drops_the_link_and_keeps_the_rest() {
        let (row, link) = (200.0, 60.0);
        assert!(fits_with_link(row, link, 260.0), "an exact fit still shows");
        assert!(!fits_with_link(row, link, 259.0));
        // What the render then composes at that width.
        assert_eq!(
            connected_status("UT61E+", None, true),
            "UT61E+ (paused)",
            "the link goes, the state stays"
        );
    }

    /// The hover names the link whatever the bar had room for, and says
    /// whether it is this session's link or the recording's.
    #[test]
    fn the_hover_spells_the_link_out() {
        assert_eq!(
            link_tooltip(Some("USB cable"), false),
            "Connected over the USB cable"
        );
        assert_eq!(
            link_tooltip(Some("Bluetooth"), false),
            "Connected over the Bluetooth adapter"
        );
        assert_eq!(
            link_tooltip(Some("USB cable"), true),
            "Recorded over the USB cable"
        );
        assert_eq!(link_tooltip(None, false), "Mock meter, no link");
        assert_eq!(link_tooltip(None, true), "Recorded over an unnamed link");
    }
}
