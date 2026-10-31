//! The top bar: the device label and connection controls on the left, the
//! status landmark in the middle, and the version / Help / shortcuts /
//! settings group on the right, wrapped to a second row when it doesn't fit.

use dmm_lib::protocol::Stability;
use dmm_lib::transport::Link;
use dmm_shared::l10n::Localizer;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};
use std::borrow::Cow;

use super::shortcuts::Shortcut;
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
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        let tc = self.settings.theme_colors(ui.visuals().dark_mode);
        let green = tc.status_ok();
        let orange = tc.status_warning();
        let gray = tc.status_inactive();

        // Cache left/right group widths from the previous frame to decide
        // whether both fit on one row.
        let left_id = egui::Id::unique("top_bar_left_w");
        let right_id = egui::Id::unique("top_bar_right_w");
        let link_id = egui::Id::unique("top_bar_link_w");
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
        // The update notice in full: the right group moves to a row of its
        // own rather than drop it, and shortens it only where even that row
        // is too narrow. Left out of the group's cached width, so it never
        // raises the window's minimum.
        let notice_w = self
            .update_check
            .notice(self.settings.check_for_updates)
            .map_or(0.0, |tag| text_width(ui, &tag.label()) + spacing);
        let one_row = cached_left + link_shown_w + cached_right + notice_w + spacing < available;

        // Row 1: device label, action buttons, status indicator
        ui.horizontal(|ui| {
            let left_start = ui.cursor().left();

            // The meter this session is talking to: the one picked, else the
            // one detection found. Under Auto-detect with nothing connected
            // there is no meter to name yet — the label says what will happen
            // instead, and is replaced by the model as soon as one answers.
            //
            // An imported session is the file's meter's, whatever Settings
            // names.
            let imported_meter: Option<String> = self.imported.as_ref().map(|_| {
                self.capture
                    .recording_layout
                    .device
                    .as_deref()
                    .map_or_else(|| tr!(l10n, "top-bar-imported-file"), Cow::Borrowed)
                    .into_owned()
            });
            let device_label = imported_meter.clone().unwrap_or_else(|| {
                self.active_listing()
                    .map_or_else(
                        || tr!(l10n, "top-bar-auto-detect"),
                        |l| Cow::Borrowed(l.display_name()),
                    )
                    .into_owned()
            });
            ui.label(RichText::new(device_label).strong());
            ui.separator();

            match &self.connection.state {
                ConnectionState::Disconnected => {
                    let keys = Shortcut::ConnectToggle.keys(ctx);
                    if ui
                        .button(tr!(l10n, "top-bar-connect"))
                        .on_hover_text(tr!(l10n, "top-bar-connect.hover", keys = &keys))
                        .clicked()
                    {
                        self.request_connect(ctx);
                    }
                }
                ConnectionState::Connected => {
                    let keys = Shortcut::ConnectToggle.keys(ctx);
                    if ui
                        .button(tr!(l10n, "top-bar-disconnect"))
                        .on_hover_text(tr!(l10n, "top-bar-disconnect.hover", keys = &keys))
                        .clicked()
                    {
                        self.disconnect();
                    }
                    let keys = Shortcut::TogglePause.keys(ctx);
                    let (pause_label, pause_tooltip) = if self.connection.paused {
                        (
                            tr!(l10n, "top-bar-resume"),
                            tr!(l10n, "top-bar-resume.hover", keys = &keys),
                        )
                    } else {
                        (
                            tr!(l10n, "top-bar-pause"),
                            tr!(l10n, "top-bar-pause.hover", keys = &keys),
                        )
                    };
                    if ui
                        .button(pause_label)
                        .on_hover_text(pause_tooltip)
                        .clicked()
                    {
                        self.set_paused(!self.connection.paused);
                    }
                    let keys = Shortcut::ClearSession.keys(ctx);
                    if ui
                        .button(tr!(l10n, "top-bar-clear"))
                        .on_hover_text(tr!(l10n, "top-bar-clear.hover", keys = &keys))
                        .clicked()
                    {
                        self.clear_session();
                    }
                }
                ConnectionState::Reconnecting => {
                    let label = self.reconnecting_label();
                    let hover = match &self.connection.reconnect_last_error {
                        Some(err) => tr!(l10n, "top-bar-reconnecting-hover-error", error = err),
                        None => tr!(l10n, "top-bar-reconnecting-hover"),
                    };
                    ui.add_enabled(false, egui::Button::new(label))
                        .on_disabled_hover_text(hover);
                    // The reconnect loop retries every 2 s indefinitely, so
                    // this is the user's only way out short of killing the
                    // app — the status tooltip above tells them to click it.
                    let keys = Shortcut::ConnectToggle.keys(ctx);
                    if ui
                        .button(tr!(l10n, "top-bar-stop-retrying"))
                        .on_hover_text(tr!(l10n, "top-bar-stop-retrying.hover", keys = &keys))
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
                    let name = self
                        .connection
                        .meter
                        .as_ref()
                        .and_then(|m| m.name.clone())
                        .unwrap_or_else(|| tr!(l10n, "top-bar-connected").into_owned());
                    let link = self.connection.link().filter(|_| show_link);
                    link_drawn = link.is_some();
                    let text = connected_status(l10n, &name, link, self.connection.paused);
                    if self.connection.paused {
                        (orange, text)
                    } else {
                        (green, text)
                    }
                }
                ConnectionState::Disconnected => match (&self.imported, imported_meter) {
                    // The file stands where a link would: what the session
                    // is on, dropped first when the bar runs out of room.
                    (Some(from), Some(meter)) => {
                        link_drawn = show_link;
                        let text = if show_link {
                            file_status(l10n, &meter, &from.path)
                        } else {
                            meter
                        };
                        (gray, text)
                    }
                    _ => (gray, tr!(l10n, "top-bar-disconnected").into_owned()),
                },
                ConnectionState::Reconnecting => {
                    let label = self.reconnecting_label();
                    (orange, label)
                }
            };

            // Group status indicators (dot, label, stability badge) so the
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
                    // The connected entry, live or recorded: a meter with the
                    // radio built in is on no adapter.
                    let built_in_radio = self
                        .connection
                        .detected()
                        .is_some_and(|d| d.bluetooth_only());
                    hover.push(link_tooltip(
                        l10n,
                        self.connection.link(),
                        built_in_radio,
                        self.replay.is_some(),
                    ));
                }
                if let Some(source) = &self.replay {
                    let path = source.path.display().to_string();
                    hover.push(tr!(l10n, "top-bar-replaying", path = path).into_owned());
                }
                if let Some(from) = &self.imported {
                    let path = from.path.display().to_string();
                    hover.push(
                        tr!(
                            l10n,
                            "top-bar-imported-from",
                            readings = from.readings,
                            path = path
                        )
                        .into_owned(),
                    );
                }
                let hover = hover.join("\n");

                // Status dot — decorative, so not focusable, but it hovers
                // with the text beside it rather than being a dead spot in
                // the middle of the status.
                let (rect, dot) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                if self.imported.is_some() {
                    // A ring, not a dot: no meter is on the other end, and a
                    // shape says so where a colour alone would not.
                    ui.painter().circle_stroke(
                        rect.center(),
                        4.5,
                        egui::Stroke::new(1.5, ui.visuals().text_color()),
                    );
                } else {
                    ui.painter().circle_filled(rect.center(), 5.0, dot_color);
                }
                let status = ui.label(RichText::new(&status_text).small());
                if !hover.is_empty() {
                    dot.on_hover_text(&hover);
                    status.on_hover_text(&hover);
                }

                // The stability badge names a protocol, so it comes from
                // the connected one where there is one — under Auto-detect
                // that is the only thing that names a meter at all — and from
                // the selected entry's profile otherwise.
                let badge = if self.imported.is_some() {
                    // No protocol is decoding anything.
                    None
                } else if self.connection.state == ConnectionState::Connected {
                    self.connection
                        .meter
                        .as_ref()
                        .filter(|m| !m.stability.is_verified())
                        .and_then(|m| {
                            let name = m.listing?.display_name().to_string();
                            Some((name, m.stability, m.feedback_url.clone()))
                        })
                } else {
                    self.selected_profile()
                        .filter(|p| !p.stability.is_verified())
                        .map(|p| {
                            let name = self
                                .picked_listing()
                                .map_or(p.display_name(), |l| l.display_name());
                            (name.to_string(), p.stability, p.feedback_url())
                        })
                };
                if let Some((model_name, stability, url)) = badge {
                    ui.hyperlink_to(
                        RichText::new(badge_text(l10n, stability))
                            .small()
                            .strong()
                            .color(orange),
                        url,
                    )
                    .on_hover_text(tr!(
                        l10n,
                        "top-bar-badge-hover",
                        warning = l10n.help().stability_warning(&model_name, stability)
                    ));
                }
            });

            // The link is measured rather than read off the drawn row, so
            // both numbers mean the same thing whether or not it is on the
            // bar: what the row needs without the link, and what the link
            // would add. Only the first reaches the window's minimum size.
            let link_w = if self.connection.state == ConnectionState::Connected {
                link_suffix_width(ui, l10n, self.connection.link())
            } else if let Some(from) = &self.imported {
                // The meter's name stands in for itself: only the difference
                // the file makes is wanted.
                let meter = "M";
                text_width(ui, &file_status(l10n, meter, &from.path)) - text_width(ui, meter)
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

    /// Right side of the top bar: version label, the link to a newer release
    /// when one is out, Help & Feedback link, keyboard shortcut help button, and
    /// settings button.
    ///
    /// Items are added left-to-right so that egui's Tab order matches the
    /// visual reading direction. A cached-width spacer right-aligns the
    /// group without needing a right-to-left layout (which would reverse
    /// tab order). The cached width comes from the previous frame and
    /// self-corrects in one frame.
    fn show_top_bar_right(&mut self, ui: &mut Ui, cache_id: egui::Id) {
        let l10n = self.l10n.clone();
        let l10n = l10n.as_ref();
        // The group's width without the update notice, which is what the
        // window's minimum comes from: the notice takes whatever room is
        // left, in full, shortened, or not at all, and never asks for more.
        let cached_width: f32 = ui.data(|d| d.get_temp(cache_id)).unwrap_or(200.0);
        let notice = self
            .update_check
            .notice(self.settings.check_for_updates)
            .cloned();
        let spacing = ui.spacing().item_spacing.x;
        let room = ui.available_width() - cached_width;
        let notice_text = notice.as_ref().and_then(|tag| {
            [tag.label(), tag.short_label()]
                .into_iter()
                .map(|text| {
                    let width = text_width(ui, &text) + spacing;
                    (text, width)
                })
                .find(|(_, width)| *width <= room)
        });
        let notice_w = notice_text.as_ref().map_or(0.0, |(_, width)| *width);
        // A notice the bar has no room for: the version button carries it.
        let hidden_notice = notice.as_ref().filter(|_| notice_text.is_none());
        let spacer = (room - notice_w).max(0.0);
        ui.add_space(spacer);
        let before = ui.cursor().left();

        // A Button (not Label) so the AccessKit role is Button, not Label.
        // For Role::Label, egui maps the text to AccessKit's `value` field —
        // `set_label` overrides via `accesskit_node_builder` are ignored, and
        // screen readers read out the literal version string instead of
        // "Show release notes". `frame_when_inactive(false)` keeps the
        // resting visual identical to a label while still painting hover and
        // focus backgrounds when the user mouses over or Tab-focuses it.
        let version_resp = ui.add(
            egui::Button::new(
                RichText::new(crate::version_label())
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .frame_when_inactive(false),
        );
        let version_resp = match hidden_notice {
            None => version_resp.a11y_label(&tr!(l10n, "top-bar-version.a11y")),
            Some(tag) => version_resp.a11y_label(&tr!(
                l10n,
                "top-bar-version-update.a11y",
                version = tag.name()
            )),
        };
        if version_resp.clicked() {
            if self.whats_new.open {
                self.whats_new.open = false;
            } else {
                self.whats_new.opener = Some(version_resp.id);
                self.open_whats_new();
            }
        }
        let version_resp = match hidden_notice {
            None => version_resp.on_hover_text(tr!(l10n, "top-bar-version.hover")),
            Some(tag) => version_resp.on_hover_text(tr!(
                l10n,
                "top-bar-version-update.hover",
                version = tag.name()
            )),
        };
        version_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if let (Some(tag), Some((text, _))) = (&notice, &notice_text) {
            ui.hyperlink_to(RichText::new(text).small(), tag.url())
                .on_hover_text(tr!(l10n, "top-bar-update.hover", version = tag.name()))
                .a11y_label(&tr!(l10n, "top-bar-update.a11y", version = tag.name()));
        }
        ui.hyperlink_to(l10n.help().project_link(), dmm_shared::help::PROJECT_URL)
            .on_hover_text(tr!(l10n, "top-bar-project-link.hover"));
        let (keys, other) = Shortcut::help_keys(ui.ctx());
        let shortcuts_btn = ui
            .button("?")
            .on_hover_text(tr!(
                l10n,
                "top-bar-shortcuts.hover",
                keys = keys,
                other = other
            ))
            .a11y_label(&tr!(l10n, "top-bar-shortcuts.a11y"));
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
            .on_hover_text(tr!(l10n, "top-bar-settings.hover"))
            .a11y_label(&tr!(l10n, "top-bar-settings.a11y"));
        if settings_btn.clicked() {
            self.settings_open = !self.settings_open;
            if self.settings_open {
                // A theme file dropped or edited since shows on the Theme row.
                self.start_theme_scan(ui.ctx());
            }
        }

        let actual_width = ui.min_rect().right() - before - notice_w;
        ui.data_mut(|d| d.insert_temp(cache_id, actual_width));
    }
}

/// The width `text` takes at the small size the bar's secondary text uses,
/// laid out rather than read off the bar so it is known before the widget is
/// placed.
fn text_width(ui: &Ui, text: &str) -> f32 {
    egui::WidgetText::from(RichText::new(text).small())
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        )
        .size()
        .x
}

/// The text beside the status dot while a meter is connected: the meter, the
/// link it answers over, and whether acquisition is halted.
///
/// The link answers the question a second meter or a second cable raises —
/// which of them this window is watching. A replay names the link its
/// recording was made over; the mock is on none, and reads as the meter alone.
/// `link` is `None` for a row too narrow to hold it as well — the hover says
/// it either way.
fn connected_status(l10n: &Localizer, name: &str, link: Option<Link>, paused: bool) -> String {
    let paused = if paused { "yes" } else { "no" };
    match link {
        Some(link) => {
            let link = l10n.help().short_name(link);
            tr!(
                l10n,
                "top-bar-status-link",
                name = name,
                link = link,
                paused = paused
            )
        }
        None => tr!(l10n, "top-bar-status", name = name, paused = paused),
    }
    .into_owned()
}

/// The status of an imported session: its meter, then its file's name where
/// a live session names its link.
fn file_status(l10n: &Localizer, meter: &str, path: &std::path::Path) -> String {
    let file = super::import::file_name(path);
    tr!(l10n, "top-bar-status-file", name = meter, file = file).into_owned()
}

/// What that suffix would add to the row's width, in points.
///
/// Laid out rather than read off the drawn row, so the answer is the same
/// whether or not the link is on the bar this frame — the row's own width
/// then stays what it was before the link existed, and with it the narrowest
/// the window may be made.
fn link_suffix_width(ui: &Ui, l10n: &Localizer, link: Option<Link>) -> f32 {
    // A stand-in name: only the difference the link makes is wanted.
    let name = "M";
    link.map_or(0.0, |link| {
        text_width(ui, &connected_status(l10n, name, Some(link), false))
            - text_width(ui, &connected_status(l10n, name, None, false))
    })
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
/// deliberately identical for the two. `built_in_radio` as for
/// [`Link::full_name`].
fn link_tooltip(
    l10n: &Localizer,
    link: Option<Link>,
    built_in_radio: bool,
    replayed: bool,
) -> String {
    // Nothing is on the far end of a mock session, and a recording whose
    // file names a link this build does not know says only that much.
    let link = super::connection_issue::link_key(link, built_in_radio);
    if replayed {
        tr!(l10n, "top-bar-link-recorded", link = link)
    } else {
        tr!(l10n, "top-bar-link-live", link = link)
    }
    .into_owned()
}

/// The badge's word for a level: the level in capitals, as the meter's own
/// annunciators are.
fn badge_text(l10n: &Localizer, stability: Stability) -> Cow<'_, str> {
    match stability {
        Stability::Verified => tr!(l10n, "top-bar-badge-verified"),
        Stability::PartlyVerified => tr!(l10n, "top-bar-badge-partly-verified"),
        Stability::Experimental => tr!(l10n, "top-bar-badge-experimental"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Link, Stability, badge_text, connected_status, fits_with_link, link_tooltip};

    #[test]
    fn the_badge_says_the_level_in_capitals() {
        let l10n = crate::l10n::english();
        for stability in [
            Stability::Verified,
            Stability::PartlyVerified,
            Stability::Experimental,
        ] {
            assert_eq!(
                badge_text(&l10n, stability),
                l10n.help().stability(stability).to_uppercase()
            );
        }
    }

    #[test]
    fn the_status_names_the_link_where_there_is_one() {
        assert_eq!(
            connected_status(
                &crate::l10n::english(),
                "UT61E+",
                Some(Link::Bluetooth),
                false
            ),
            "UT61E+ \u{b7} Bluetooth"
        );
        assert_eq!(
            connected_status(&crate::l10n::english(), "UT61E+", None, false),
            "UT61E+"
        );
        assert_eq!(
            connected_status(
                &crate::l10n::english(),
                "UT61E+",
                Some(Link::UsbCable),
                true
            ),
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
            connected_status(&crate::l10n::english(), "UT61E+", None, true),
            "UT61E+ (paused)",
            "the link goes, the state stays"
        );
    }

    /// The hover names the link whatever the bar had room for, and says
    /// whether it is this session's link or the recording's.
    #[test]
    fn the_hover_spells_the_link_out() {
        assert_eq!(
            link_tooltip(&crate::l10n::english(), Some(Link::UsbCable), false, false),
            "Connected over the USB cable"
        );
        assert_eq!(
            link_tooltip(&crate::l10n::english(), Some(Link::Bluetooth), false, false),
            "Connected over the Bluetooth adapter"
        );
        assert_eq!(
            link_tooltip(&crate::l10n::english(), Some(Link::UsbCable), false, true),
            "Recorded over the USB cable"
        );
        assert_eq!(
            link_tooltip(&crate::l10n::english(), None, false, false),
            "Mock meter, no link"
        );
        assert_eq!(
            link_tooltip(&crate::l10n::english(), None, false, true),
            "Recorded over an unnamed link"
        );
        // A meter with the radio built in, live or recorded, has no adapter.
        assert_eq!(
            link_tooltip(&crate::l10n::english(), Some(Link::Bluetooth), true, false),
            "Connected over the Bluetooth link"
        );
        assert_eq!(
            link_tooltip(&crate::l10n::english(), Some(Link::Bluetooth), true, true),
            "Recorded over the Bluetooth link"
        );
    }
}
