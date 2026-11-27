//! The **Bluetooth device** setting: which device the radio's turn opens.
//!
//! Auto finds it by the name it advertises, which is right for a meter or
//! adapter under the name its entry lists. A meter that goes by another
//! name, or one adapter among several, is chosen here from what a scan
//! hears, and opened by address from then on
//! ([`dmm_lib::OpenOptions::bluetooth_peer`]).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dmm_lib::BluetoothListing;
use dmm_shared::tr;
use eframe::egui::{self, RichText, Ui};

use super::connection_issue::ConnectionIssue;
use super::task::Task;
use super::{App, ConnectionState};
use crate::a11y::ResponseA11yExt;
use dmm_shared::{BluetoothDevice, ChoiceSetAside as ChoiceIgnored};

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
            .map(Self::of)
            .collect()
    }

    fn of(device: dmm_lib::DeviceInfo) -> Self {
        Self {
            name: device.product.unwrap_or_else(|| device.path.clone()),
            selector: device.path,
        }
    }
}

/// The scan behind the open **Bluetooth device** list, and what it heard.
#[derive(Default)]
pub(super) struct BluetoothPicker {
    /// `--bluetooth-scenario`: the number of made-up devices a scan hears
    /// in place of the radio's, and every open then finds no meter.
    pub(super) scenario: Option<usize>,
    scan: Option<Task<Result<BluetoothListing, dmm_lib::error::Error>>>,
    /// Up while that scan runs on its thread, for a Connect to wait out:
    /// two searches on one radio stop each other's scan.
    scanning: Arc<AtomicBool>,
    /// What the last scan heard, kept for the session.
    heard: Vec<Heard>,
    /// Why the last scan heard nothing, in the window's language.
    problem: Option<String>,
    /// Whether a scan has finished, so an empty list can say so.
    scanned: bool,
    /// Where the Connect under way leaves the other named devices its own
    /// scan heard ([`dmm_lib::OpenOptions::heard`]).
    pub(super) connect_heard: Option<Arc<Mutex<Vec<dmm_lib::DeviceInfo>>>>,
    /// Those devices, once that Connect found no meter: offered under its
    /// notice until the next Connect.
    offered: Vec<Heard>,
    /// Whether the offer is unfolded into its rows.
    unfolded: bool,
    /// The entry the keyboard was on when the open list was last drawn, by
    /// what opens it (`None` for Auto): where focus returns when the list
    /// opens again at a new height.
    focused: Option<Option<String>>,
    /// The last frame the open list was drawn on: the list closes for a
    /// frame to take a new height when the scan answers, and opening again
    /// then is no reason to scan again.
    list_drawn: Option<u64>,
}

impl BluetoothPicker {
    /// The flag that is up while the list's scan runs.
    pub(super) fn scanning(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.scanning)
    }
}

/// Lowers the scanning flag when the scan ends, however it ends.
struct ScanRunning(Arc<AtomicBool>);

impl Drop for ScanRunning {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// How long an open waits for the list's scan, which runs a few seconds.
const SCAN_WAIT: Duration = Duration::from_secs(6);

/// Hold an open back while the list's scan runs, unless `stop` calls the
/// open off: its own search would stop that scan, and be stopped by it.
pub(super) fn wait_for_scan(scanning: &AtomicBool, stop: &AtomicBool) {
    let begun = Instant::now();
    while scanning.load(Ordering::Relaxed)
        && !stop.load(Ordering::Relaxed)
        && begun.elapsed() < SCAN_WAIT
    {
        std::thread::sleep(Duration::from_millis(50));
    }
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
        ChoiceIgnored::of(
            self.settings.overrides.adapter.as_deref(),
            self.selected_device(),
            self.settings.shared.bluetooth,
        )
    }

    /// The Bluetooth device the next Connect opens on the radio's turn:
    /// the one chosen, unless something sets it aside. `None` finds one by
    /// name.
    pub(super) fn bluetooth_peer(&self) -> Option<&BluetoothDevice> {
        self.settings.shared.bluetooth_peer(
            self.settings.overrides.adapter.as_deref(),
            self.selected_device(),
            self.settings.shared.bluetooth,
        )
    }

    /// The chosen device, when `address` is its own and not one `--adapter`
    /// named: what a notice about a failed open calls it. Asked of the
    /// choice itself, not of what the next Connect opens, so a notice keeps
    /// its wording when a setting changes under it.
    pub(super) fn chosen_at(&self, address: &str) -> Option<&BluetoothDevice> {
        self.settings
            .shared
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
        let unchanged = match (&mut self.settings.shared.bluetooth_device, &chosen) {
            (None, None) => true,
            (Some(a), Some(b)) if same(a, b) => {
                // The same device, perhaps under a new name.
                a.name.clone_from(&b.name);
                true
            }
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
        self.settings.shared.bluetooth_device = chosen;
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
        self.bluetooth.scanning.store(true, Ordering::Relaxed);
        let running = ScanRunning(self.bluetooth.scanning());
        self.bluetooth.scan = Some(Task::spawn("bluetooth-scan", ctx, move || {
            let _running = running;
            match scenario {
                Some(count) => Ok(BluetoothListing {
                    devices: Vec::new(),
                    unrecognised: scenario_devices(count),
                }),
                None => dmm_lib::list_bluetooth_devices(),
            }
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
                // The devices listed before keep their rows, and the new
                // ones go under them: the list may be open under a pointer.
                let mut now = Heard::all(listing);
                let mut heard = Vec::with_capacity(now.len());
                for before in &self.bluetooth.heard {
                    if let Some(i) = now.iter().position(|h| h.selector == before.selector) {
                        heard.push(now.remove(i));
                    }
                }
                heard.append(&mut now);
                // The chosen device, heard under a new name, takes it.
                if let Some(chosen) = self.settings.shared.bluetooth_device.as_mut()
                    && let Some(now) = heard.iter().find(|h| h.selector == chosen.selector)
                {
                    chosen.name.clone_from(&now.name);
                }
                self.bluetooth.heard = heard;
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

        // A build without Bluetooth has no device to choose.
        if !dmm_lib::BLUETOOTH_SUPPORTED {
            return;
        }
        self.poll_bluetooth_scan();
        let l10n = std::sync::Arc::clone(&self.l10n);
        let ignored = self.bluetooth_choice_ignored();
        let chosen = self
            .settings
            .shared
            .bluetooth_device
            .as_ref()
            .map(|d| Heard {
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
        // Opened by the list taking its new height, not by the user: focus
        // stays where they put it, and nothing scans again.
        let reopened = was_drawn.is_some_and(|last| frame.saturating_sub(last) <= 2);
        let focused_before = self.bluetooth.focused.clone();
        let mut focused_now = None;
        let mut opened = false;
        let mut drawn = false;
        let picked = ui
            .horizontal_wrapped(|ui| {
                let picked = ui
                    .scope(|ui| {
                        // A reconnect searches the radio as a Connect does,
                        // and the list's scan would cut that search short.
                        let reopening = self.connection.state == ConnectionState::Reconnecting;
                        if ignored.is_some() || connecting || reopening {
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
                                    // user opens the list, and back where
                                    // it was as the list takes a new height.
                                    let key = entry.map(|h| h.selector.clone());
                                    let lands = match (&focused_before, reopened) {
                                        (Some(before), true) => *before == key,
                                        _ => live,
                                    };
                                    if !was_open && lands {
                                        response.request_focus();
                                    }
                                    if response.has_focus() {
                                        focused_now = Some(key);
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
            self.bluetooth.focused = focused_now;
        }
        if opened && !reopened {
            self.start_bluetooth_scan(ui.ctx());
        }
        if let Some(entry) = picked {
            self.choose_bluetooth_device(entry);
        }
    }

    /// Whether the notice on screen is about the chosen device not opening,
    /// so going back to Auto is offered under it
    /// ([`Self::show_bluetooth_offers`]).
    pub(super) fn offers_bluetooth_auto(&self) -> bool {
        use dmm_shared::help::Diagnosis;
        let Some(ConnectionIssue::Lib { diagnosis, .. }) = &self.connection.last_error else {
            return false;
        };
        let about_the_device = match diagnosis {
            Diagnosis::NothingAtAddress { address } | Diagnosis::AdapterUnreachable { address } => {
                self.chosen_at(address).is_some()
            }
            // Nothing recognisable answered over the radio, where the
            // chosen device is the one that was opened.
            Diagnosis::NotIdentified { bridge, .. } => {
                *bridge == dmm_lib::BLUETOOTH && self.bluetooth_peer().is_some()
            }
            _ => false,
        };
        about_the_device && self.connection.state == ConnectionState::Disconnected
    }

    /// A Connect is starting: what the last one offered goes, and this one
    /// gets somewhere to leave what its scan hears.
    pub(super) fn begin_bluetooth_offer(&mut self) -> Arc<Mutex<Vec<dmm_lib::DeviceInfo>>> {
        self.bluetooth.offered.clear();
        self.bluetooth.unfolded = false;
        let heard = Arc::default();
        self.bluetooth.connect_heard = Some(Arc::clone(&heard));
        heard
    }

    /// A Connect failed: keep the other devices its scan heard, when what
    /// it reports is that no meter was found by name.
    pub(super) fn keep_bluetooth_offer(&mut self) {
        use dmm_shared::help::Diagnosis;
        let heard = self.bluetooth.connect_heard.take();
        let no_meter_by_name = matches!(
            &self.connection.last_error,
            Some(ConnectionIssue::Lib {
                diagnosis: Diagnosis::NotFound {
                    bluetooth_searched: true
                } | Diagnosis::BluetoothOnlyNotFound { .. },
                ..
            })
        );
        if !no_meter_by_name {
            return;
        }
        let heard = heard
            .and_then(|heard| {
                heard
                    .lock()
                    .ok()
                    .map(|mut heard| std::mem::take(&mut *heard))
            })
            .unwrap_or_default();
        self.bluetooth.offered = heard.into_iter().map(Heard::of).collect();
    }

    /// Whether the other devices the failed Connect heard are offered under
    /// its notice.
    fn offers_other_devices(&self) -> bool {
        !self.bluetooth.offered.is_empty()
            && self.connection.last_error.is_some()
            && self.connection.state == ConnectionState::Disconnected
    }

    /// What [`Self::show_bluetooth_offers`] draws, for the meter-only view
    /// to key its fit on: nothing, the Auto button, the folded line, or the
    /// line with its rows.
    pub(super) fn bluetooth_offer_shape(&self) -> usize {
        if self.offers_bluetooth_auto() {
            1
        } else if !self.offers_other_devices() {
            0
        } else if self.bluetooth.unfolded {
            3 + self.bluetooth.offered.len()
        } else {
            2
        }
    }

    /// Open `device` and keep it as the **Bluetooth device**.
    fn pick_offered(&mut self, device: Heard) {
        self.choose_bluetooth_device(Some(device));
        self.connection.needs_reconnect = true;
    }

    /// Under a connection notice, what the **Bluetooth device** setting
    /// can do about it. Under one about the chosen device, a button that
    /// sets the row back to Auto and connects, which looks for the meter by
    /// name again. Under one that found no meter by name, the other named
    /// devices that Connect's scan heard, folded into one line: a meter
    /// going by a name no entry lists is among them, and picking it chooses
    /// it and connects.
    ///
    /// Not drawn in the big-meter modes, as the connection help is not.
    pub(super) fn show_bluetooth_offers(&mut self, ui: &mut Ui) {
        if self.offers_bluetooth_auto() {
            ui.add_space(4.0);
            let clicked = ui
                .button(tr!(self.l10n, "connection-chosen-auto"))
                .on_hover_text(tr!(self.l10n, "connection-chosen-auto.hover"))
                .clicked();
            if clicked {
                self.choose_bluetooth_device(None);
                self.connection.needs_reconnect = true;
            }
            return;
        }
        if !self.offers_other_devices() {
            return;
        }
        ui.add_space(4.0);
        let l10n = Arc::clone(&self.l10n);
        let count = self.bluetooth.offered.len();
        let heading = tr!(l10n, "connection-others-heard", count = count);
        let header = egui::CollapsingHeader::new(RichText::new(heading).small())
            .id_salt("bluetooth-others")
            .open(Some(self.bluetooth.unfolded));
        let mut picked = None;
        let shown = header.show(ui, |ui| {
            let hint = match self.picked_listing() {
                Some(listing) => tr!(
                    l10n,
                    "connection-others-pick",
                    meter = l10n.label(dmm_lib::text::TextKind::Model, listing.display_name())
                ),
                None => tr!(l10n, "connection-others-pick-auto"),
            };
            ui.add(
                egui::Label::new(
                    RichText::new(hint)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .wrap(),
            );
            for device in &self.bluetooth.offered {
                let row = ui
                    .button(&device.name)
                    .on_hover_text(tr!(l10n, "connection-others-pick.hover"));
                if row.clicked() {
                    picked = Some(device.clone());
                }
            }
        });
        if shown.header_response.clicked() {
            self.bluetooth.unfolded = !self.bluetooth.unfolded;
        }
        if let Some(device) = picked {
            self.pick_offered(device);
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
        assert!(
            app.settings.shared.bluetooth_device.is_some(),
            "kept in the file"
        );
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
        assert!(app.settings.shared.bluetooth_device.is_some());
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

    /// Auto is offered only under a failure of the chosen device: not under
    /// a cable's, nor a radio that is off, and a device that would not link
    /// gets no command-line step.
    #[test]
    fn auto_is_offered_only_for_the_chosen_devices_own_failures() {
        use dmm_lib::error::BluetoothIssue;
        let failed = |error: Error| {
            let mut app = chosen("auto");
            app.deliver(DmmMessage::Failed(error));
            app
        };
        let app = failed(Error::DeviceNotIdentified {
            bridge: "CP2110",
            built_in_radio: false,
        });
        assert!(!app.offers_bluetooth_auto(), "a cable's failure");
        let app = failed(BluetoothIssue::PoweredOff.into());
        assert!(!app.offers_bluetooth_auto(), "the radio is off");

        let app = failed(BluetoothIssue::Stack("Not Connected".to_string()).into());
        assert!(app.offers_bluetooth_auto(), "the device would not link");
        let notice = app.connection_notice().expect("a notice");
        let help = notice.help_text(&app.l10n);
        assert!(!help.contains("dmm-cli"), "{help}");
    }

    /// Picking the chosen device again under a new name keeps the choice
    /// and takes the name.
    #[test]
    fn the_chosen_device_takes_a_new_name() {
        let mut app = chosen("auto");
        if let Some(device) = app.settings.shared.bluetooth_device.as_mut() {
            device.report_asked = true;
        }
        app.choose_bluetooth_device(Some(heard("Renamed clamp")));
        let device = app
            .settings
            .shared
            .bluetooth_device
            .as_ref()
            .expect("a choice");
        assert_eq!(device.name, "Renamed clamp");
        assert!(device.report_asked, "asked for it already");
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

    fn no_meter() -> DmmMessage {
        DmmMessage::Failed(Error::NoTransportFound {
            cables: Vec::new(),
            bluetooth_searched: true,
        })
    }

    /// An app whose Connect found no meter while its scan heard `count`
    /// other devices.
    fn found_no_meter(count: usize) -> App {
        let mut app = test_app();
        let heard = app.begin_bluetooth_offer();
        *heard.lock().unwrap() = scenario_devices(count);
        app.deliver(no_meter());
        app
    }

    /// A Connect that found no meter by name offers the devices its scan
    /// heard, folded; any other failure offers none, and the next Connect
    /// starts with none.
    #[test]
    fn a_connect_that_found_no_meter_offers_what_it_heard() {
        let mut app = found_no_meter(2);
        assert_eq!(app.bluetooth_offer_shape(), 2, "folded");
        app.bluetooth.unfolded = true;
        assert_eq!(
            app.bluetooth_offer_shape(),
            5,
            "the line, the hint, two rows"
        );
        app.begin_bluetooth_offer();
        assert_eq!(app.bluetooth_offer_shape(), 0, "a new Connect");

        assert_eq!(found_no_meter(0).bluetooth_offer_shape(), 0, "none heard");
        let mut app = test_app();
        let heard = app.begin_bluetooth_offer();
        *heard.lock().unwrap() = scenario_devices(2);
        app.deliver(DmmMessage::Failed(Error::Timeout));
        assert_eq!(app.bluetooth_offer_shape(), 0, "another failure");
    }

    /// Clicking the line unfolds it, and clicking a device chooses it and
    /// asks for a connect.
    #[test]
    fn a_device_picked_under_the_notice_is_chosen_and_opened() {
        use crate::app::test_support::{a11y, click_frames, headless_frame, node_rect};
        let mut app = found_no_meter(2);
        let ctx = crate::app::styled_context();
        ctx.enable_accesskit();
        let frame = |app: &mut App, events| {
            let out = headless_frame(&ctx, None, None, events, |ui| {
                app.show_bluetooth_offers(ui);
            });
            a11y(&out).0
        };
        let click = |app: &mut App, label: &str| {
            frame(app, vec![]);
            let nodes = frame(app, vec![]);
            let at = node_rect(&nodes, |n| n.label() == Some(label))
                .unwrap_or_else(|| panic!("no {label:?} on screen"))
                .center();
            for events in click_frames(at, egui::PointerButton::Primary) {
                frame(app, events);
            }
        };
        click(&mut app, "2 other Bluetooth devices heard");
        assert!(app.bluetooth.unfolded);
        click(&mut app, "CM2100B");
        let chosen = app
            .settings
            .shared
            .bluetooth_device
            .as_ref()
            .expect("a choice");
        assert_eq!(chosen.selector, "12:34:56:78:9A:00");
        assert!(app.connection.needs_reconnect);
    }
}
