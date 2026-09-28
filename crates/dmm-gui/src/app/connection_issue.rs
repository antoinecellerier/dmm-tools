//! Why the GUI has no readings to show: the failure the acquisition thread
//! reported, classified once as it arrives, and the notice built from it for
//! the reading column — text only; `show_connection_help` and the big meter
//! draw it.

use dmm_lib::protocol::registry;
use dmm_shared::help::{ConnectedAdapters, LinksSearched, SetupSection, connected_adapters};

use super::{App, ConnectionState};

/// Why the GUI currently has no readings to show.
///
/// Replaces a `String` that carried the sentinel `"__device_not_found__"`
/// and was probed with `contains("adapter not found")` at the render site —
/// CLAUDE.md: "Prefer enums over string-typed status/state values." The
/// acquisition thread already distinguishes these cases; this stops the
/// distinction being flattened into text and parsed back out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectionIssue {
    /// Nothing answered on the links that were searched. `bluetooth_searched`
    /// is the open path's own answer to whether the radio got a turn, which
    /// decides the title and the sections the help shows.
    DeviceNotFound { bluetooth_searched: bool },
    /// `--adapter` named a USB device and nothing matched it. Carries the
    /// finished help text, including the connected-device list, because
    /// building it enumerates the USB bus — far too heavy for the paint path.
    AdapterNotFound { help: String },
    /// `--adapter` named a Bluetooth adapter and nothing answered at that
    /// address. The scan the open just ran is the only thing that would say
    /// which addresses are live, so nothing is enumerated here.
    BluetoothNotFound { address: String },
    /// `--adapter` named a Bluetooth adapter the stack could not connect to —
    /// most often one that is paired but asleep. `error` is what the stack
    /// said, shown above the same steps an address nothing answered gets.
    BluetoothUnreachable { address: String, error: String },
    /// A meter with the radio built in was searched for and nothing in range
    /// carried its name. Its display name and the registry's steps that
    /// switch its radio on, from the error.
    BluetoothOnlyNotFound {
        model: &'static str,
        activation: &'static str,
    },
    /// The stack could not search for a meter with the radio built in —
    /// radio off, permission denied. `error` is what the stack said, shown
    /// above the meter's own steps.
    BluetoothOnlyUnreachable {
        model: &'static str,
        activation: &'static str,
        error: String,
    },
    /// A meter with the radio built in, and the radio was not searched: the
    /// setting is off, the build has no Bluetooth, or `--adapter` names a USB
    /// device. `message` is the library's, which says which, finished with
    /// the GUI's own switch when the setting is off.
    BluetoothNotSearched { message: String },
    /// The cable is there but nothing on it answered the detection probe.
    /// Carries the finished help — what to switch on, per meter that could
    /// have been on that bridge — because it is built from the bridge the
    /// error names and the render path no longer has it.
    NotIdentified { help: String },
    /// Anything else, as reported by the acquisition thread.
    Other(String),
}

/// Which connection notice is on screen.
///
/// The big meter keys its fit cache on this: the titles differ in length, so
/// the fitted font has to be re-measured when one notice replaces another.
/// Not derived from the text, which the waiting notice changes every timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NoticeKind {
    Detecting,
    Waiting,
    /// A replay played to its last frame.
    Ended,
    /// Nothing on the USB bus, which is the only link that was searched.
    UsbNotFound,
    /// Nothing on either link.
    NoMeterFound,
    /// Nothing at the Bluetooth address `--adapter` named.
    BluetoothNotFound,
    /// The adapter at that address would not connect, or the stack could
    /// not search for a meter with the radio built in.
    BluetoothUnreachable,
    /// Nothing in range carried the name of a meter with the radio built in.
    BluetoothOnlyNotFound,
    /// A meter with the radio built in, and the radio was not searched.
    BluetoothNotSearched,
    AdapterNotFound,
    NotIdentified,
    NoResponse,
}

/// What the reading column has to say about why there is nothing to show,
/// in pieces, so a layout can render as much of it as it has room for.
pub(super) struct ConnectionNotice {
    pub(super) kind: NoticeKind,
    /// One line naming the problem.
    pub(super) title: String,
    /// The steps to try, one section per link, each drawn under its own bold
    /// label. Empty for a notice that is about no link in particular.
    pub(super) sections: Vec<SetupSection>,
    /// The prose under them, or the whole of a sectionless notice's steps.
    /// Several hard-wrapped lines; empty while detection is still running,
    /// where there is nothing to do but wait.
    pub(super) body: String,
    /// The experimental-support feedback link, as `(text, url)`.
    pub(super) experimental_link: Option<(String, String)>,
}

impl ConnectionNotice {
    /// Everything under the title as one string, for the big-meter modes:
    /// they have room for the title alone and hand this to a tooltip.
    pub(super) fn help_text(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            out.push_str(&section.text());
            out.push_str("\n\n");
        }
        out.push_str(&self.body);
        out
    }
}

impl ConnectionIssue {
    /// Classify an error the acquisition thread hands over whole.
    ///
    /// Matched by variant rather than by [`dmm_lib::error::ErrorKind`]: the
    /// help needs what each one carries — the selector the user typed, the
    /// links that were searched, the bridge nothing answered on — and the
    /// coarse kind carries none of it. Classified once when the error
    /// arrives, not at the render site on every repaint. `adapter` is the
    /// `--adapter` value the open was given, which a stack failure is
    /// explained by when it names a Bluetooth adapter; `selected` the meter
    /// picked in Settings, which explains one when it has the radio built in.
    pub(super) fn from_error(
        err: &dmm_lib::error::Error,
        adapter: Option<&str>,
        selected: Option<&'static registry::SelectableDevice>,
    ) -> Self {
        if let dmm_lib::error::Error::AdapterNotFound(selector) = err {
            // An address is explained by the link it named, not by the USB
            // bus — nothing on the bus could have answered it.
            if dmm_lib::is_bluetooth_selector(selector) {
                return Self::BluetoothNotFound {
                    address: selector.clone(),
                };
            }
            return Self::AdapterNotFound {
                help: adapter_not_found_help(selector),
            };
        }
        // The open path already worked out which links it searched; carrying
        // the answer keeps the help from deriving it again from the build,
        // the settings and the selected meter.
        if let dmm_lib::error::Error::NoTransportFound {
            bluetooth_searched, ..
        } = err
        {
            return Self::DeviceNotFound {
                bluetooth_searched: *bluetooth_searched,
            };
        }
        // Also a `Timeout` kind — retrying does help once transmission is on —
        // so it is matched by variant, ahead of the kind test below.
        if let dmm_lib::error::Error::DeviceNotIdentified {
            bridge,
            built_in_radio,
        } = err
        {
            return Self::NotIdentified {
                help: not_identified_help(bridge, *built_in_radio),
            };
        }
        if let dmm_lib::error::Error::Bluetooth(_) = err
            && let Some(address) = adapter.filter(|a| dmm_lib::is_bluetooth_selector(a))
        {
            return Self::BluetoothUnreachable {
                address: address.to_string(),
                error: err.to_string(),
            };
        }
        if let dmm_lib::error::Error::Bluetooth(_) = err
            && let Some(device) = selected.filter(|d| d.bluetooth_only())
        {
            return Self::BluetoothOnlyUnreachable {
                model: device.display_name,
                activation: device.activation_instructions,
                error: err.to_string(),
            };
        }
        if let dmm_lib::error::Error::BluetoothOnly {
            model,
            activation,
            miss,
        } = err
        {
            return match miss {
                dmm_lib::error::BluetoothOnlyMiss::NotInRange => {
                    Self::BluetoothOnlyNotFound { model, activation }
                }
                // The remedy is the GUI's own switch; the library's sentence
                // names none.
                dmm_lib::error::BluetoothOnlyMiss::SwitchedOff => Self::BluetoothNotSearched {
                    message: format!("{err}. {}", dmm_shared::help::gui_bluetooth_off_hint()),
                },
                _ => Self::BluetoothNotSearched {
                    message: format!("{err}."),
                },
            };
        }
        Self::Other(err.to_string())
    }
}

/// Build the "adapter not found" help, listing what is actually on the bus.
///
/// Called once when the error arrives, not from the render path:
/// `connected_adapters()` constructs a fresh `HidApi` and walks every HID device
/// on the system, and this help stays on screen across every repaint until
/// the user reconnects. The list is a snapshot either way — the user has to
/// restart with a different `--adapter` to act on it.
fn adapter_not_found_help(selector: &str) -> String {
    let adapters = connected_adapters();
    let mut msg = format!("No device matched --adapter '{selector}'.");
    msg.push_str("\n\n");
    msg.push_str(&adapters.lines().join("\n"));
    // An empty bus is a complete answer on its own; the other two leave the
    // user with a choice to make.
    if !matches!(adapters, ConnectedAdapters::None) {
        msg.push_str("\n\nRestart with the correct --adapter value.");
    }
    if let Some(hint) = dmm_shared::help::colonless_address_hint(selector) {
        msg.push_str("\n\n");
        msg.push_str(&hint);
    }
    msg
}

/// Build the "nothing answered" help: what the user can switch on, for every
/// meter that could have been behind that bridge.
///
/// The meters come grouped by their steps, as the CLI lists them
/// ([`dmm_shared::help::activation_groups`]).
fn not_identified_help(bridge: &str, built_in_radio: bool) -> String {
    let mut msg = format!(
        "The {} is connected but no meter identified itself.\n\n\
         Switch on the meter's data transmission, or pick the model in \
         Settings (\u{2699}):\n",
        dmm_lib::transport::bridge_link_name(bridge, built_in_radio)
    );
    for (steps, names) in dmm_shared::help::activation_groups(&dmm_lib::devices_on_bridge(bridge)) {
        msg.push('\n');
        msg.push_str(&names.join(", "));
        msg.push('\n');
        msg.push_str(steps);
        msg.push('\n');
    }
    msg.push_str(
        "\nAlready transmitting and still not recognised? Pick its model in Settings \
         (\u{2699}) and please report it.\n",
    );
    msg
}

impl App {
    /// What the session currently has to say about why there are no readings,
    /// or `None` when there is nothing to report.
    ///
    /// Split from the rendering because the big-meter modes cannot draw all of
    /// it: with the reading scaled to fill the window there is no room below
    /// it, so they put `title` where the reading's placeholder goes and hand
    /// `body` to a tooltip. The normal layouts still draw every piece.
    pub(super) fn connection_notice(&self) -> Option<ConnectionNotice> {
        let notice = |kind, title: String, body: String| ConnectionNotice {
            kind,
            title,
            sections: Vec::new(),
            body,
            experimental_link: None,
        };

        // The probe is still running: nothing names the meter, the channel is
        // up, and neither a connection nor a failure has come back yet. It
        // spends up to a couple of seconds giving each family its turn to
        // answer, and an empty column for that long reads as a hang.
        if self.selected_device().is_none()
            && self.connection.state == ConnectionState::Disconnected
            && self.connection.rx.is_some()
            && self.connection.last_error.is_none()
        {
            return Some(notice(
                NoticeKind::Detecting,
                "Detecting the meter\u{2026}".to_string(),
                String::new(),
            ));
        }

        // The reading on screen is the recording's last, and stays: nothing
        // more is coming, which a trace that just stops would not say. Worded
        // for a Connect after the end too, which ends again at once.
        if self.connection.ended {
            return Some(notice(
                NoticeKind::Ended,
                "The recording has ended".to_string(),
                String::new(),
            ));
        }

        // Show waiting indicator before error threshold
        if self.connection.waiting_timeouts > 0 && self.connection.last_error.is_none() {
            // A stretch of the recording with nothing in it, which no meter
            // and no cable can be asked about. Said plainly and left there:
            // the file plays on by itself once the gap is over.
            if self.replay.is_some() {
                return Some(notice(
                    NoticeKind::Waiting,
                    "No frames in the recording here".to_string(),
                    String::new(),
                ));
            }
            let dots = ".".repeat((self.connection.waiting_timeouts as usize % 4) + 1);
            // Padded to a fixed field: in the big meter this title is measured
            // to fit the window, and a line that grows and shrinks four times
            // a second would either re-fit on every dot or overflow by three
            // characters. The padding is invisible either way.
            let title = format!("Waiting for meter{dots:<4}");
            // Under Auto-detect there is no selection to check, so the hint is
            // the other two things that make a quiet meter talk. With a family
            // selected the way out is named too: the value may have been saved
            // by a detected connect rather than picked, and the user swapping
            // meters has no reason to connect the silence to it.
            let body = if self.selected_device().is_some() {
                "Check that the correct device is selected in Settings (\u{2699}), \
                 or pick Auto-detect there"
            } else {
                "Switch on the meter's data transmission, or pick the model in Settings (\u{2699})"
            };
            return Some(notice(NoticeKind::Waiting, title, body.to_string()));
        }

        let issue = self.connection.last_error.as_ref()?;

        if let ConnectionIssue::DeviceNotFound { bluetooth_searched } = issue {
            // Nothing answered on the links the open path searched. The
            // sections come from the library so the CLI's help and this panel
            // stay the same advice; only the closing line is the GUI's, since
            // the CLI has no Connect button.
            let links = LinksSearched::from_usb_failure(*bluetooth_searched);
            Some(ConnectionNotice {
                kind: if *bluetooth_searched {
                    NoticeKind::NoMeterFound
                } else {
                    NoticeKind::UsbNotFound
                },
                title: links.not_found_title(),
                sections: links.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: self.experimental_link(),
            })
        } else if let ConnectionIssue::BluetoothOnlyNotFound { model, activation } = issue {
            // The same notice, on the radio alone and with the meter's own
            // steps: it has no cable, and no adapter to wake.
            let links = LinksSearched::BluetoothOnly { model, activation };
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothOnlyNotFound,
                title: links.not_found_title(),
                sections: links.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: self.experimental_link(),
            })
        } else if let ConnectionIssue::BluetoothOnlyUnreachable {
            model,
            activation,
            error,
        } = issue
        {
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: error.clone(),
                sections: LinksSearched::BluetoothOnly { model, activation }.sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::BluetoothNotSearched { message } = issue {
            Some(notice(
                NoticeKind::BluetoothNotSearched,
                "Bluetooth not searched".to_string(),
                message.clone(),
            ))
        } else if let ConnectionIssue::BluetoothNotFound { address } = issue {
            let links = LinksSearched::BluetoothAt(address);
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothNotFound,
                title: links.not_found_title(),
                sections: links.sections(),
                // The GUI cannot scan, so the way back in is a restart with
                // the address the scan printed.
                body: "Restart with --adapter set to the address it prints.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::BluetoothUnreachable { address, error } = issue {
            // Found, or remembered, but the link would not come up: the
            // stack's own words first, then the steps that wake an adapter.
            Some(ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: error.clone(),
                sections: LinksSearched::BluetoothAt(address).sections(),
                body: "Click \"Connect\" after resolving the issue.".to_string(),
                experimental_link: None,
            })
        } else if let ConnectionIssue::AdapterNotFound { help } = issue {
            Some(notice(
                NoticeKind::AdapterNotFound,
                "Adapter not found".to_string(),
                help.clone(),
            ))
        } else if let ConnectionIssue::NotIdentified { help } = issue {
            // The link is fine and the probe ran; nothing on the far end
            // spoke a protocol we know. Which link it was is in the body —
            // the title stays one string per notice, which is what the big
            // meter's fit cache keys on.
            Some(notice(
                NoticeKind::NotIdentified,
                "No meter answered".to_string(),
                help.clone(),
            ))
        } else {
            // Dongle found but meter not responding.
            // The meter this session is talking about: the one picked, or the
            // one detection found. Under Auto-detect, before anything answered,
            // there is neither a model to name nor steps to give.
            // Name the link the session is on: "adapter" now reads as the
            // Bluetooth one, and over a cable it never was one.
            let built_in_radio = self.active_device().is_some_and(|d| d.bluetooth_only());
            let link = self
                .connection
                .link()
                .map_or("link", |link| link.full_name(built_in_radio));
            let instructions = match self.active_device() {
                Some(entry) => format!(
                    "The {link} is connected but the meter \n\
                     isn't responding ({} selected).\n\
                     \n\
                     If this is the wrong device, change it in Settings (\u{2699}), or pick Auto-detect there.\n\
                     Otherwise, enable data transmission:\n\
                     {}",
                    entry.display_name, entry.activation_instructions
                ),
                None => "No meter answered \u{2014} switch on the meter's data \n\
                         transmission, or pick the model in Settings (\u{2699})."
                    .to_string(),
            };
            Some(notice(
                NoticeKind::NoResponse,
                "No response from meter".to_string(),
                instructions,
            ))
        }
    }

    /// The experimental-support line a "nothing found" notice ends with, for
    /// a picked meter short of verified. Auto-detect names no meter, so there
    /// is no protocol to warn about until one answers.
    fn experimental_link(&self) -> Option<(String, String)> {
        self.selected_profile
            .as_ref()
            .filter(|p| !p.stability.is_verified())
            .map(|profile| {
                (
                    format!(
                        "{} Report feedback.",
                        dmm_shared::help::experimental_warning(
                            profile.model_name,
                            profile.stability
                        )
                    ),
                    profile.feedback_url(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::connection::ConnectedMeter;
    use crate::settings::Settings;

    /// An app whose settings name `family`.
    fn app(family: &str) -> App {
        let mut settings = Settings::default();
        settings.shared.device_family = family.to_string();
        App::from_settings(settings, dmm_lib::Clock::real())
    }

    /// The notice the app would draw, with `issue` as the failure on record.
    fn notice_for(app: &mut App, issue: ConnectionIssue) -> ConnectionNotice {
        app.connection.last_error = Some(issue);
        app.connection_notice()
            .expect("a failure is always a notice")
    }

    /// A session with nothing wrong has nothing to say: the reading column
    /// draws the reading and stops there.
    #[test]
    fn a_quiet_session_has_no_notice() {
        let app = app("ut61eplus");
        assert!(app.connection_notice().is_none());
    }

    /// Every failure splits into a title that names the problem and a body
    /// that says what to do about it — the split the big meter needs, which
    /// has room for the title only and hands the body to a tooltip.
    #[test]
    fn each_connection_issue_gets_its_own_notice() {
        let mut app = app("ut61eplus");

        let n = notice_for(
            &mut app,
            ConnectionIssue::DeviceNotFound {
                bluetooth_searched: false,
            },
        );
        assert_eq!(n.kind, NoticeKind::UsbNotFound);
        assert_eq!(n.title, "No USB cable found");
        assert!(n.body.contains("Connect"), "got {:?}", n.body);

        let n = notice_for(
            &mut app,
            ConnectionIssue::AdapterNotFound {
                help: "no adapter matched".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::AdapterNotFound);
        assert_eq!(n.title, "Adapter not found");
        assert_eq!(n.body, "no adapter matched");

        let n = notice_for(
            &mut app,
            ConnectionIssue::NotIdentified {
                help: "switch USB mode on".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::NotIdentified);
        assert_eq!(n.title, "No meter answered");
        assert_eq!(n.body, "switch USB mode on");

        let n = notice_for(&mut app, ConnectionIssue::Other("timed out".to_string()));
        assert_eq!(n.kind, NoticeKind::NoResponse);
        assert_eq!(n.title, "No response from meter");
        assert!(
            n.body.contains("enable data transmission"),
            "the selected meter's steps belong in the body, got {:?}",
            n.body
        );
    }

    /// The title says which links were looked at, and the sections say what
    /// to do on each — the cable steps and the radio steps never run into one
    /// another.
    #[test]
    fn the_not_found_notice_is_grouped_by_link() {
        let mut app = app("ut61eplus");

        let n = notice_for(
            &mut app,
            ConnectionIssue::DeviceNotFound {
                bluetooth_searched: true,
            },
        );
        assert_eq!(n.kind, NoticeKind::NoMeterFound);
        assert_eq!(n.title, "No meter found over USB or Bluetooth");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link).collect();
        assert_eq!(links, ["USB cable", "Bluetooth"]);
        // The big meter shows the title alone and hands the rest to a
        // tooltip, so every step has to reach that one string.
        let help = n.help_text();
        assert!(help.contains("USB cable: check it is plugged in"), "{help}");
        assert!(help.contains("Bluetooth: turn on Bluetooth"), "{help}");
        assert!(help.ends_with("Click \"Connect\" after resolving the issue."));
    }

    /// An address nothing answered is about that link alone: no cable steps,
    /// and no USB bus listing — nothing on the bus could have answered it.
    #[test]
    fn a_bluetooth_address_is_answered_on_its_own_link() {
        let mut app = app("ut61eplus");
        let n = notice_for(
            &mut app,
            ConnectionIssue::BluetoothNotFound {
                address: "12:34:56:78:9A:BC".to_string(),
            },
        );
        assert_eq!(n.kind, NoticeKind::BluetoothNotFound);
        assert_eq!(n.title, "No Bluetooth device found at 12:34:56:78:9A:BC");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link).collect();
        assert_eq!(links, ["Bluetooth"]);
        assert!(n.help_text().contains("dmm-cli list"), "{}", n.help_text());
        assert_eq!(
            n.body,
            "Restart with --adapter set to the address it prints."
        );
    }

    /// A named adapter the stack could not reach — paired but asleep — gets
    /// the stack's own words as the title and the same Bluetooth steps as an
    /// address nothing answered, not a silent-meter notice.
    #[test]
    fn an_unreachable_named_adapter_gets_the_bluetooth_steps() {
        let err = dmm_lib::error::Error::Bluetooth("Timed out after 10s".to_string());
        let issue = ConnectionIssue::from_error(&err, Some("12:34:56:78:9A:BC"), None);
        if !dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            // A build without the radio has no address to explain it by.
            assert!(matches!(issue, ConnectionIssue::Other(_)));
            return;
        }
        let mut app = app("ut61eplus");
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: Timed out after 10s");
        assert_eq!(
            n.sections,
            LinksSearched::BluetoothAt("12:34:56:78:9A:BC").sections()
        );
        assert_eq!(n.body, "Click \"Connect\" after resolving the issue.");

        // Without an address, or with a USB one, it stays the bare error.
        assert!(matches!(
            ConnectionIssue::from_error(&err, None, None),
            ConnectionIssue::Other(_)
        ));
        assert!(matches!(
            ConnectionIssue::from_error(&err, Some("00C5B27A"), None),
            ConnectionIssue::Other(_)
        ));
    }

    /// A meter with the radio built in is answered on the radio alone, with
    /// its own steps, whichever way the open failed — never with the cable
    /// help. A meter behind an adapter keeps the bare stack error.
    #[test]
    fn a_bluetooth_only_meter_gets_its_own_notice() {
        use dmm_lib::error::{BluetoothOnlyMiss, Error};
        let ut60bt = registry::find_device("ut60bt").expect("registry entry");
        let own_steps = LinksSearched::BluetoothOnly {
            model: ut60bt.display_name,
            activation: ut60bt.activation_instructions,
        };
        let missed = |miss| Error::BluetoothOnly {
            model: ut60bt.display_name,
            activation: ut60bt.activation_instructions,
            miss,
        };
        let mut app = app("ut60bt");

        let issue = ConnectionIssue::from_error(&missed(BluetoothOnlyMiss::NotInRange), None, None);
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothOnlyNotFound);
        assert_eq!(n.title, "No UT60BT found over Bluetooth");
        assert_eq!(n.sections, own_steps.sections());
        assert!(
            n.help_text().contains("Long press SEL"),
            "{}",
            n.help_text()
        );
        assert!(n.experimental_link.is_some());

        let stack = Error::Bluetooth("turned off on this computer".to_string());
        let n = notice_for(
            &mut app,
            ConnectionIssue::from_error(&stack, None, Some(ut60bt)),
        );
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: turned off on this computer");
        assert_eq!(n.sections, own_steps.sections());

        let issue =
            ConnectionIssue::from_error(&missed(BluetoothOnlyMiss::SwitchedOff), None, None);
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothNotSearched);
        assert_eq!(n.title, "Bluetooth not searched");
        assert_eq!(
            n.body,
            "UT60BT connects over Bluetooth only, and Bluetooth is switched off. \
             Tick \"Look for Bluetooth devices\" in Settings (\u{2699})."
        );
        assert!(n.sections.is_empty());

        let ut61eplus = registry::find_device("ut61eplus").expect("registry entry");
        assert!(matches!(
            ConnectionIssue::from_error(&stack, None, Some(ut61eplus)),
            ConnectionIssue::Other(_)
        ));
    }

    /// The waiting notice is measured to fit the big meter's window, so its
    /// animated dots must not change its width: the fit would otherwise be
    /// redone, or the line overflow, at every timeout.
    #[test]
    fn the_waiting_title_keeps_one_width() {
        let mut app = app("ut61eplus");
        let titles: Vec<String> = (1..=9)
            .map(|timeouts| {
                app.connection.waiting_timeouts = timeouts;
                let n = app.connection_notice().expect("waiting is a notice");
                assert_eq!(n.kind, NoticeKind::Waiting);
                n.title
            })
            .collect();
        let widths: Vec<usize> = titles.iter().map(|t| t.chars().count()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "the title changes width as the dots cycle: {titles:?}"
        );
        // The dots are still animating — a fixed width must not have been
        // bought by dropping them.
        assert!(
            titles
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= 4,
            "the dots stopped cycling: {titles:?}"
        );
    }

    /// The adapter case used to be recovered at the render site with
    /// `error.contains("adapter not found")`, and "no adapter on the bus"
    /// travelled as the sentinel string "__device_not_found__".
    #[test]
    fn adapter_error_is_classified_with_its_selector() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::AdapterNotFound("ABC123".to_string()),
            None,
            None,
        );
        let ConnectionIssue::AdapterNotFound { help } = issue else {
            panic!("expected AdapterNotFound, got {issue:?}");
        };
        assert!(help.contains("ABC123"), "got {help}");
    }

    /// `DeviceNotIdentified` is an `ErrorKind::Timeout`, which would land it
    /// in `Other` and print the "no response from meter" block naming a
    /// device nobody selected. It is matched by variant for that reason.
    #[test]
    fn nothing_answering_the_probe_is_its_own_case() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::DeviceNotIdentified {
                bridge: "CP2110",
                built_in_radio: false,
            },
            None,
            None,
        );
        let ConnectionIssue::NotIdentified { help } = issue else {
            panic!("expected NotIdentified, got {issue:?}");
        };
        assert!(help.contains("UT61E+"), "got {help}");
        assert!(help.contains("Long press the USB/Hz button"), "got {help}");
        // gui.md: user-facing text names the cable, never the bridge chip.
        assert!(help.contains("USB cable is connected"), "got {help}");
        for chip in ["CP2110", "CH9329", "CH9325"] {
            assert!(!help.contains(chip), "{chip} leaked into: {help}");
        }
    }

    /// Nothing answering on Brymen's cable: its help names the Brymen
    /// meters and their steps, and the cable as a USB cable. "BU-86X" is
    /// the cable's own name, which its steps use.
    #[test]
    fn nothing_answering_on_the_bu86x_offers_the_brymen_meters() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::DeviceNotIdentified {
                bridge: "BU-86X",
                built_in_radio: false,
            },
            None,
            None,
        );
        let ConnectionIssue::NotIdentified { help } = issue else {
            panic!("expected NotIdentified, got {issue:?}");
        };
        for model in ["BM869s", "BM829s", "BM525s"] {
            assert!(help.contains(model), "{model}: got {help}");
        }
        assert!(help.contains("USB cable"), "got {help}");
        // The three series share their steps, printed once.
        assert_eq!(
            help.matches("optical PC-Comm port").count(),
            1,
            "got {help}"
        );
        assert!(!help.contains("UT61E+"), "got {help}");
    }

    /// The same probe over the radio: the user switched an adapter on, so the
    /// help must not send them looking at a cable.
    #[test]
    fn the_silent_link_is_named_as_the_user_sees_it() {
        let help = not_identified_help(dmm_lib::BLUETOOTH, false);
        assert!(
            help.starts_with("The Bluetooth adapter is connected"),
            "got {help}"
        );
        // The meters UNI-T lists on the adapter, with their steps. The steps
        // themselves are the meter's own and name its USB socket, which is
        // where the adapter plugs in.
        assert!(help.contains("UT61E+"), "got {help}");
        // A meter with the radio built in answered the open: no adapter.
        let help = not_identified_help(dmm_lib::BLUETOOTH, true);
        assert!(
            help.starts_with("The Bluetooth link is connected but no meter identified itself."),
            "got {help}"
        );
    }

    /// A quiet meter over the radio: an adapter for a meter behind one, the
    /// link for a meter with the radio built in.
    #[test]
    fn a_quiet_meter_names_its_bluetooth_link() {
        for (id, link) in [("ut61eplus", "adapter"), ("ut60bt", "link")] {
            let mut app = app(id);
            app.connection.meter = Some(ConnectedMeter {
                link: Some(dmm_lib::transport::Link::Bluetooth),
                ..ConnectedMeter::test_fixture(None)
            });
            let n = notice_for(&mut app, ConnectionIssue::Other("timed out".to_string()));
            assert_eq!(n.kind, NoticeKind::NoResponse);
            assert!(
                n.body
                    .starts_with(&format!("The Bluetooth {link} is connected but the meter")),
                "{id}: {:?}",
                n.body
            );
        }
    }

    /// Six UT61+ models share one four-step instruction. Listing each meter
    /// separately would repeat those steps six times in a panel the reading
    /// column has to fit.
    #[test]
    fn meters_sharing_activation_steps_are_listed_together() {
        let help = not_identified_help("CP2110", false);
        assert!(
            help.contains("UT61E+, UT61B+, UT61D+, UT161B, UT161D, UT161E"),
            "got {help}"
        );
        assert_eq!(
            help.matches("Long press the USB/Hz button").count(),
            1,
            "the shared steps appear once: {help}"
        );
        // The mock is not on any bridge, so it is never offered as a cure for
        // a silent cable.
        assert!(!help.contains("Mock"), "got {help}");
        assert!(
            help.trim_end()
                .ends_with("Pick its model in Settings (\u{2699}) and please report it."),
            "the help must end with the way out: {help}"
        );
    }

    /// The help used to be selected on the thread side, before the error
    /// crossed the channel; it now falls out of the error, which carries the
    /// links the open path searched.
    #[test]
    fn a_missing_adapter_is_the_device_not_found_case() {
        for bluetooth_searched in [false, true] {
            assert_eq!(
                ConnectionIssue::from_error(
                    &dmm_lib::error::Error::NoTransportFound {
                        cables: Vec::new(),
                        bluetooth_searched
                    },
                    None,
                    None,
                ),
                ConnectionIssue::DeviceNotFound { bluetooth_searched }
            );
        }
    }

    /// A Bluetooth address takes the link's own case, not the bus listing:
    /// `connected_adapters()` would walk every HID device for an answer that
    /// could not contain the address anyway.
    #[test]
    fn an_unanswered_address_is_classified_as_a_bluetooth_failure() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::AdapterNotFound("12:34:56:78:9A:BC".to_string()),
            None,
            None,
        );
        let expected = if dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            ConnectionIssue::BluetoothNotFound {
                address: "12:34:56:78:9A:BC".to_string(),
            }
        } else {
            // A build without the radio has no link that value could name.
            ConnectionIssue::AdapterNotFound {
                help: adapter_not_found_help("12:34:56:78:9A:BC"),
            }
        };
        assert_eq!(issue, expected);
    }

    #[test]
    fn other_errors_keep_their_message() {
        let issue = ConnectionIssue::from_error(&dmm_lib::error::Error::Timeout, None, None);
        assert_eq!(
            issue,
            ConnectionIssue::Other("timeout waiting for response".to_string())
        );
    }

    /// A message that merely mentions the phrase must not be mistaken for
    /// the adapter case — the old `contains` probe would have matched it,
    /// and the prefix probe that replaced it would have matched the same
    /// text arriving with an "adapter not found: " prefix from elsewhere.
    #[test]
    fn a_mention_of_the_phrase_is_not_the_adapter_case() {
        let issue = ConnectionIssue::from_error(
            &dmm_lib::error::Error::UnknownDevice(
                "meter reports adapter not found somewhere".to_string(),
            ),
            None,
            None,
        );
        assert!(matches!(issue, ConnectionIssue::Other(_)));
    }
}
