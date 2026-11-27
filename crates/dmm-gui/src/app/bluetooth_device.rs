//! The **Bluetooth device** setting: which device the radio's turn opens.
//!
//! Auto finds it by the name it advertises, which is right for a meter or
//! adapter under the name its entry lists. A meter that goes by another
//! name, or one adapter among several, is chosen here from what a scan
//! hears, and opened by address from then on
//! ([`dmm_lib::OpenOptions::bluetooth_peer`]).

use dmm_lib::BluetoothListing;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};

use super::connection_issue::ConnectionIssue;
use super::task::Task;
use super::{App, ConnectionState};
use crate::a11y::ResponseA11yExt;
use crate::settings::BluetoothDevice;

/// A named Bluetooth device a scan heard: what opens it, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Heard {
    pub(super) selector: String,
    pub(super) name: String,
}

impl Heard {
    /// The devices of a listing that were heard, the ones an entry takes by
    /// name first.
    fn all(listing: BluetoothListing) -> Vec<Self> {
        listing
            .devices
            .into_iter()
            .filter(|d| !d.not_heard)
            .chain(listing.unrecognised)
            .map(|d| Self {
                name: d.product.unwrap_or_else(|| d.path.clone()),
                selector: d.path,
            })
            .collect()
    }
}

/// Why the chosen device is not what the next Connect opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChoiceIgnored {
    /// `--adapter` names what this session opens.
    Adapter,
    /// The meter picked on the Device row is on no Bluetooth link.
    NoBluetoothLink,
    /// **Look for Bluetooth devices** is off.
    BluetoothOff,
}

/// The scan behind the open **Bluetooth device** list, and what it heard.
#[derive(Default)]
pub(super) struct BluetoothPicker {
    /// `--bluetooth-scenario`: the number of made-up devices a scan hears
    /// in place of the radio's, and every open then finds no meter.
    pub(super) scenario: Option<usize>,
    scan: Option<Task<Result<BluetoothListing, dmm_lib::error::Error>>>,
    /// What the last scan heard, kept for the session.
    heard: Vec<Heard>,
    /// Why the last scan heard nothing, in the window's language.
    problem: Option<String>,
    /// Whether a scan has finished, so an empty list can say so.
    scanned: bool,
    /// The last frame the open list was drawn on: the list closes for a
    /// frame to take a new height when the scan answers, and opening again
    /// then is no reason to scan again.
    list_drawn: Option<u64>,
}

/// The devices `--bluetooth-scenario` hears: placeholder addresses, the
/// first a meter under its model name.
pub(super) fn scenario_devices(count: usize) -> Vec<dmm_lib::DeviceInfo> {
    const NAMES: [&str; 5] = [
        "CM2100B",
        "Living Room Speaker",
        "DESKTOP-4F2K",
        "Mi Band 7",
        "TV [Bedroom]",
    ];
    (0..count)
        .map(|i| dmm_lib::DeviceInfo {
            path: format!("12:34:56:78:9A:{i:02X}"),
            product: Some(match NAMES.get(i) {
                Some(name) => (*name).to_string(),
                None => format!("Beacon {i}"),
            }),
            serial: None,
            transport: dmm_lib::BLUETOOTH,
            not_heard: false,
        })
        .collect()
}

impl App {
    /// Why the next Connect leaves the chosen device aside, if it does.
    pub(super) fn bluetooth_choice_ignored(&self) -> Option<ChoiceIgnored> {
        let on_bluetooth = |d: &dmm_lib::protocol::registry::SelectableDevice| {
            d.bluetooth_only() || dmm_lib::searches_bluetooth(Some(d))
        };
        if self.settings.overrides.adapter.is_some() {
            Some(ChoiceIgnored::Adapter)
        } else if !self.selected_device().is_none_or(on_bluetooth) {
            Some(ChoiceIgnored::NoBluetoothLink)
        } else if !self.settings.shared.bluetooth {
            Some(ChoiceIgnored::BluetoothOff)
        } else {
            None
        }
    }

    /// The Bluetooth device the next Connect opens on the radio's turn:
    /// the one chosen, unless something sets it aside. `None` finds one by
    /// name.
    pub(super) fn bluetooth_peer(&self) -> Option<&BluetoothDevice> {
        self.settings
            .bluetooth_device
            .as_ref()
            .filter(|_| self.bluetooth_choice_ignored().is_none())
    }

    /// The chosen device, when `address` is its own and not one `--adapter`
    /// named: what a notice about a failed open calls it. Asked of the
    /// choice itself, not of what the next Connect opens, so a notice keeps
    /// its wording when a setting changes under it.
    pub(super) fn chosen_at(&self, address: &str) -> Option<&BluetoothDevice> {
        self.settings
            .bluetooth_device
            .as_ref()
            .filter(|d| d.selector == address && self.settings.overrides.adapter.is_none())
    }

    /// Choose `device`, `None` for Auto. A session on another Bluetooth
    /// device than the one chosen reconnects, and so does one that is coming
    /// up. One that is up on a cable, on the device chosen or, for Auto, on
    /// any device is where the next Connect would land: it is left alone,
    /// and reopening a Bluetooth device the moment it was let go fails.
    pub(super) fn choose_bluetooth_device(&mut self, device: Option<Heard>) {
        let chosen = device.map(|d| BluetoothDevice {
            selector: d.selector,
            name: d.name,
            report_asked: false,
        });
        let same = |a: &BluetoothDevice, b: &BluetoothDevice| a.selector == b.selector;
        let unchanged = match (&self.settings.bluetooth_device, &chosen) {
            (None, None) => true,
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        };
        if unchanged {
            return;
        }
        let on_bluetooth = self
            .connection
            .meter
            .as_ref()
            .and_then(|m| m.bluetooth_selector.as_deref());
        let stays = match self.connection.state {
            ConnectionState::Disconnected => true,
            ConnectionState::Connected => match (&chosen, on_bluetooth) {
                (Some(chosen), Some(session)) => chosen.selector == session,
                _ => true,
            },
            _ => false,
        };
        self.settings.bluetooth_device = chosen;
        if !stays {
            self.connection.needs_reconnect = true;
        }
    }

    /// Scan for the open list, unless a scan is already running.
    fn start_bluetooth_scan(&mut self, ctx: &egui::Context) {
        if self.bluetooth.scan.is_some() {
            return;
        }
        let scenario = self.bluetooth.scenario;
        self.bluetooth.scan = Some(Task::spawn("bluetooth-scan", ctx, move || match scenario {
            Some(count) => Ok(BluetoothListing {
                devices: Vec::new(),
                unrecognised: scenario_devices(count),
            }),
            None => dmm_lib::list_bluetooth_devices(),
        }));
    }

    /// Take a finished scan's answer.
    fn poll_bluetooth_scan(&mut self) {
        let Some(answer) = self.bluetooth.scan.as_ref().and_then(Task::poll) else {
            return;
        };
        self.bluetooth.scan = None;
        self.bluetooth.scanned = true;
        match answer {
            Ok(Ok(listing)) => {
                self.bluetooth.heard = Heard::all(listing);
                self.bluetooth.problem = None;
            }
            Ok(Err(e)) => {
                self.bluetooth.heard.clear();
                self.bluetooth.problem = Some(self.l10n.error(&e));
            }
            Err(panic) => {
                self.bluetooth.heard.clear();
                self.bluetooth.problem = Some(panic);
            }
        }
    }

    /// The **Bluetooth device** row: Auto, the device chosen, and what a
    /// scan hears, which opening the list starts. Greyed, with the reason
    /// beside it, while the choice is set aside; greyed while a meter opens,
    /// as the Device row is.
    pub(super) fn show_bluetooth_device_row(&mut self, ui: &mut Ui, connecting: bool) {
        use crate::listbox::{choice_entry_text, listbox_dropdown, navigate_choice_entries};
        use egui::{ComboBox, TextWrapMode};

        self.poll_bluetooth_scan();
        let l10n = std::sync::Arc::clone(&self.l10n);
        let ignored = self.bluetooth_choice_ignored();
        let chosen = self.settings.bluetooth_device.as_ref().map(|d| Heard {
            selector: d.selector.clone(),
            name: d.name.clone(),
        });
        let auto = tr!(l10n, "settings-bluetooth-device-auto");
        let shown = chosen.as_ref().map_or(auto.as_ref(), |d| d.name.as_str());
        // Auto, the choice, then the rest of what the scan heard.
        let entries: Vec<Option<&Heard>> = std::iter::once(None)
            .chain(chosen.as_ref().map(Some))
            .chain(
                self.bluetooth
                    .heard
                    .iter()
                    .filter(|h| chosen.as_ref().is_none_or(|c| c.selector != h.selector))
                    .map(Some),
            )
            .collect();
        let status = if self.bluetooth.scan.is_some() {
            Some(tr!(l10n, "settings-bluetooth-device-scanning").into_owned())
        } else if let Some(problem) = &self.bluetooth.problem {
            Some(problem.clone())
        } else if self.bluetooth.scanned && self.bluetooth.heard.is_empty() {
            Some(tr!(l10n, "settings-bluetooth-device-none").into_owned())
        } else {
            None
        };

        // What the open list's height follows: its entries, and the status
        // line under them, which is shorter than an entry, so one replacing
        // the other changes the height at the same count.
        let rows = entries.len() * 2 + usize::from(status.is_some());
        let frame = ui.ctx().cumulative_frame_nr();
        let was_drawn = self.bluetooth.list_drawn;
        let mut opened = false;
        let mut drawn = false;
        let picked = ui
            .horizontal_wrapped(|ui| {
                let picked = ui
                    .scope(|ui| {
                        if ignored.is_some() || connecting {
                            ui.disable();
                        }
                        ui.label(tr!(l10n, "settings-bluetooth-device"));
                        let combo = |combo| {
                            ComboBox::selected_text(combo, shown).wrap_mode(TextWrapMode::Truncate)
                        };
                        let (response, picked) = listbox_dropdown(
                            ui,
                            "bluetooth-device",
                            || rows,
                            combo,
                            |ui, was_open| {
                                opened = !was_open;
                                drawn = true;
                                let mut picked = None;
                                let mut responses = Vec::new();
                                for entry in &entries {
                                    let live = entry.map(|h| &h.selector)
                                        == chosen.as_ref().map(|c| &c.selector);
                                    let name = entry.map_or(auto.as_ref(), |h| h.name.as_str());
                                    let response = ui
                                        .selectable_label(live, choice_entry_text(name, live))
                                        .a11y_label(name);
                                    // Focus lands on the live entry as the
                                    // list opens.
                                    if !was_open && live {
                                        response.request_focus();
                                    }
                                    if response.clicked() {
                                        picked = Some(entry.cloned());
                                    }
                                    responses.push(response);
                                }
                                if let Some(status) = &status {
                                    ui.label(
                                        RichText::new(status)
                                            .small()
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                }
                                navigate_choice_entries(ui.ctx(), &responses);
                                picked
                            },
                        );
                        response
                            .on_hover_text(tr!(l10n, "settings-bluetooth-device.hover"))
                            .a11y_label(&tr!(l10n, "settings-bluetooth-device.a11y"));
                        picked
                    })
                    .inner;
                let note = ignored.map(|why| match why {
                    ChoiceIgnored::Adapter => tr!(l10n, "settings-bluetooth-device-adapter"),
                    ChoiceIgnored::NoBluetoothLink => {
                        tr!(l10n, "settings-bluetooth-device-no-link")
                    }
                    ChoiceIgnored::BluetoothOff => tr!(
                        l10n,
                        "settings-bluetooth-device-off",
                        setting = l10n.help().bluetooth_setting()
                    ),
                });
                if let Some(note) = note {
                    ui.label(
                        RichText::new(note)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                picked
            })
            .inner;
        if drawn {
            self.bluetooth.list_drawn = Some(frame);
        }
        // Opened by the user, not by the list taking its new height.
        let reopened = was_drawn.is_some_and(|last| frame.saturating_sub(last) <= 2);
        if opened && !reopened {
            self.start_bluetooth_scan(ui.ctx());
        }
        if let Some(entry) = picked {
            self.choose_bluetooth_device(entry);
        }
    }

    /// Whether the notice on screen is about the chosen device not opening,
    /// so going back to Auto is offered under it
    /// ([`Self::show_bluetooth_auto_offer`]).
    pub(super) fn offers_bluetooth_auto(&self) -> bool {
        use dmm_shared::help::Diagnosis;
        let about_the_device = matches!(
            &self.connection.last_error,
            Some(ConnectionIssue::Lib {
                diagnosis: Diagnosis::NothingAtAddress { .. }
                    | Diagnosis::AdapterUnreachable { .. }
                    | Diagnosis::NotIdentified { .. },
                ..
            })
        );
        about_the_device
            && self.settings.bluetooth_device.is_some()
            && self.settings.overrides.adapter.is_none()
            && self.connection.state == ConnectionState::Disconnected
    }

    /// Under a notice about the chosen device: a button that sets the
    /// **Bluetooth device** row back to Auto and connects, which looks for
    /// the meter by name again.
    ///
    /// Not drawn in the big-meter modes, as the connection help is not.
    pub(super) fn show_bluetooth_auto_offer(&mut self, ui: &mut Ui) {
        if !self.offers_bluetooth_auto() {
            return;
        }
        ui.add_space(4.0);
        let clicked = ui
            .button(tr!(self.l10n, "connection-chosen-auto"))
            .on_hover_text(tr!(self.l10n, "connection-chosen-auto.hover"))
            .clicked();
        if clicked {
            self.choose_bluetooth_device(None);
            self.connection.needs_reconnect = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::connection::DmmMessage;
    use crate::app::test_support::test_app;
    use dmm_lib::error::Error;

    const ADDRESS: &str = "12:34:56:78:9A:BC";

    fn heard(name: &str) -> Heard {
        Heard {
            selector: ADDRESS.to_string(),
            name: name.to_string(),
        }
    }

    /// An app on `family` with a Bluetooth device chosen.
    fn chosen(family: &str) -> App {
        let mut app = test_app();
        app.settings.shared.device_family = family.to_string();
        app.choose_bluetooth_device(Some(heard("Bench clamp")));
        app
    }

    /// The chosen device is what the radio's turn opens, under Auto-detect
    /// and for a meter on Bluetooth, and is set aside, not forgotten, by
    /// `--adapter`, a cable-only meter and the Bluetooth switch.
    #[test]
    fn the_choice_is_opened_unless_something_sets_it_aside() {
        assert!(test_app().bluetooth_peer().is_none(), "Auto");
        for family in ["auto", "ut61eplus", "cm2100b"] {
            let app = chosen(family);
            assert_eq!(
                app.bluetooth_peer().map(|d| d.selector.as_str()),
                Some(ADDRESS),
                "{family}"
            );
        }
        let mut app = chosen("auto");
        app.settings.overrides.adapter = Some("00C5B27A".to_string());
        assert_eq!(app.bluetooth_choice_ignored(), Some(ChoiceIgnored::Adapter));
        let mut app = chosen("auto");
        app.settings.shared.bluetooth = false;
        assert_eq!(
            app.bluetooth_choice_ignored(),
            Some(ChoiceIgnored::BluetoothOff)
        );
        let app = chosen("ut804");
        assert_eq!(
            app.bluetooth_choice_ignored(),
            Some(ChoiceIgnored::NoBluetoothLink)
        );
        assert!(app.bluetooth_peer().is_none());
        assert!(app.settings.bluetooth_device.is_some(), "kept in the file");
    }

    /// Only a session on another Bluetooth device than the one chosen
    /// reconnects: one on a cable, on the device chosen, or any when Auto
    /// is chosen, is where the next Connect would land.
    #[test]
    fn a_choice_reconnects_only_a_session_on_another_device() {
        use crate::app::connection::ConnectedMeter;
        let session_on = |selector: Option<&str>| {
            let mut app = test_app();
            app.connection.state = ConnectionState::Connected;
            app.connection.meter = Some(ConnectedMeter {
                bluetooth_selector: selector.map(str::to_string),
                ..ConnectedMeter::test_fixture(None)
            });
            app
        };
        let mut app = test_app();
        app.choose_bluetooth_device(Some(heard("Bench clamp")));
        assert!(!app.connection.needs_reconnect, "disconnected");

        let mut app = session_on(Some(ADDRESS));
        app.choose_bluetooth_device(Some(heard("Bench clamp")));
        assert!(!app.connection.needs_reconnect, "already on it");
        assert!(app.settings.bluetooth_device.is_some());
        app.choose_bluetooth_device(None);
        assert!(!app.connection.needs_reconnect, "Auto");

        let mut app = session_on(None);
        app.choose_bluetooth_device(Some(heard("Bench clamp")));
        assert!(!app.connection.needs_reconnect, "on a cable");

        let mut app = session_on(Some("AA:BB:CC:DD:EE:FF"));
        app.choose_bluetooth_device(Some(heard("Bench clamp")));
        assert!(app.connection.needs_reconnect, "on another device");
    }

    /// A chosen device that did not answer is named in the notice, which
    /// offers Auto under it; an address from `--adapter` gets neither.
    #[test]
    fn a_chosen_device_not_heard_is_named_and_auto_offered() {
        let mut app = chosen("auto");
        app.deliver(DmmMessage::Failed(Error::AdapterNotFound(
            ADDRESS.to_string(),
        )));
        let notice = app.connection_notice().expect("a notice");
        assert_eq!(notice.title, "Bench clamp not heard over Bluetooth");
        assert!(!notice.body.contains("--adapter"), "{}", notice.body);
        assert!(app.offers_bluetooth_auto());

        let mut app = test_app();
        app.settings.overrides.adapter = Some(ADDRESS.to_string());
        app.deliver(DmmMessage::Failed(Error::AdapterNotFound(
            ADDRESS.to_string(),
        )));
        let notice = app.connection_notice().expect("a notice");
        assert!(notice.body.contains("--adapter"), "{}", notice.body);
        assert!(!app.offers_bluetooth_auto());
    }

    /// A scan's answer fills the list, the devices an entry takes by name
    /// first and one that was not heard left out; a scan that failed says
    /// why instead.
    #[test]
    fn a_scan_fills_the_list_or_says_why_not() {
        let device = |name: &str, not_heard| dmm_lib::DeviceInfo {
            path: format!("{name}-address"),
            product: Some(name.to_string()),
            serial: None,
            transport: dmm_lib::BLUETOOTH,
            not_heard,
        };
        let mut app = test_app();
        let (tx, task) = Task::by_hand();
        app.bluetooth.scan = Some(task);
        app.poll_bluetooth_scan();
        assert!(app.bluetooth.scan.is_some(), "still scanning");
        tx.send(Ok(Ok(BluetoothListing {
            devices: vec![device("UT-D07B", false), device("UT-D07A", true)],
            unrecognised: vec![device("CM2100B", false)],
        })))
        .unwrap();
        app.poll_bluetooth_scan();
        let names: Vec<&str> = app
            .bluetooth
            .heard
            .iter()
            .map(|h| h.name.as_str())
            .collect();
        assert_eq!(names, ["UT-D07B", "CM2100B"]);
        assert!(app.bluetooth.scan.is_none());

        let (tx, task) = Task::by_hand();
        app.bluetooth.scan = Some(task);
        tx.send(Ok(Err(Error::Timeout))).unwrap();
        app.poll_bluetooth_scan();
        assert!(app.bluetooth.heard.is_empty());
        assert!(app.bluetooth.problem.is_some());
    }

    /// A meter that reads under a name its entry does not list is asked to
    /// be reported once for a chosen device, not at every start.
    #[test]
    fn a_chosen_devices_name_is_asked_for_once() {
        let mut app = chosen("cm2100b");
        app.connection.state = ConnectionState::Connected;
        let opened = dmm_lib::protocol::registry::find_device("cm2100b").expect("an entry");
        let unlisted = || DmmMessage::UnlistedName {
            opened,
            name: "Bench clamp".to_string(),
        };
        app.deliver(unlisted());
        assert!(app.toasts.take_latest().is_some(), "asked");
        app.deliver(unlisted());
        assert!(app.toasts.take_latest().is_none(), "asked again");
    }
}
