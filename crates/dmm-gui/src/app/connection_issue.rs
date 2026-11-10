//! Why the GUI has no readings to show: the failure the acquisition thread
//! reported, classified once as it arrives ([`Diagnosis`]) and kept whole,
//! and the notice worded from it for the reading column on every frame, so
//! a language switch reaches it — text only; `show_connection_help` and the
//! big meter draw it.

use dmm_lib::protocol::registry;
use dmm_lib::text::TextKind;
use dmm_lib::transport::Link;
use dmm_shared::export::escape_controls;
use dmm_shared::help::{
    ConnectedAdapters, Diagnosis, LinksSearched, SetupSection, connected_adapters,
};
use dmm_shared::l10n::{Localizer, link_key};
use dmm_shared::reopen::SeveralCables;
use dmm_shared::tr;

use super::{App, ConnectionState};

/// Why the GUI currently has no readings to show.
///
/// Held as data and worded where it is drawn, in the language of that frame.
#[derive(Debug)]
pub(crate) enum ConnectionIssue {
    /// The library failed: which help that gets, and the error, whose own
    /// words (the stack's, the selector `--adapter` named) the notice quotes.
    /// Opening failures, a meter gone quiet, data that could not be read.
    Lib {
        diagnosis: Diagnosis,
        error: dmm_lib::error::Error,
        /// What the bus held, for an `--adapter` that matched nothing:
        /// listing it walks every HID device, far too heavy for the paint
        /// path, so it is listed once, when the error arrives.
        adapters: Option<ConnectedAdapters>,
    },
    /// A lost link with nothing to say which cable is this meter's. Held as
    /// the reason, and worded where it is drawn.
    SeveralCables(SeveralCables),
    /// The acquisition thread ended without saying why.
    Stopped,
    /// The acquisition thread panicked, with the panic's own text, which
    /// stays English; the lead-in is worded where it is drawn.
    Internal(String),
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
    /// The meter answered with data that could not be read.
    BadData,
    /// A lost link that nothing says which cable to reopen on.
    SeveralCables,
    /// Any other failure: a fixed title, the error itself in the body.
    Failed,
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
    /// The support-level feedback link, as `(text, url)`.
    pub(super) feedback_link: Option<(String, String)>,
}

impl ConnectionNotice {
    /// A notice with no sections and no feedback link: a title and its prose.
    fn plain(kind: NoticeKind, title: String, body: String) -> Self {
        Self {
            kind,
            title,
            sections: Vec::new(),
            body,
            feedback_link: None,
        }
    }

    /// Everything under the title as one string, for the big-meter modes:
    /// they have room for the title alone and hand this to a tooltip.
    pub(super) fn help_text(&self, l10n: &Localizer) -> String {
        let mut out = String::new();
        for section in &self.sections {
            out.push_str(&l10n.help().section_text(section));
            out.push_str("\n\n");
        }
        out.push_str(&self.body);
        out
    }
}

impl ConnectionIssue {
    /// Classify an error the acquisition thread hands over whole, once, as it
    /// arrives. `adapter` is the `--adapter` value the open was given, and
    /// `selected` the meter picked in Settings ([`Diagnosis::of`]).
    pub(super) fn from_error(
        error: dmm_lib::error::Error,
        adapter: Option<&str>,
        selected: Option<registry::Listing>,
    ) -> Self {
        let diagnosis = Diagnosis::of(&error, adapter, selected);
        let adapters = (diagnosis == Diagnosis::AdapterNotFound).then(connected_adapters);
        Self::Lib {
            diagnosis,
            error,
            adapters,
        }
    }

    /// `error` as it arrives from a session with nothing picked and no
    /// `--adapter`.
    #[cfg(test)]
    pub(super) fn of(error: dmm_lib::error::Error) -> Self {
        Self::from_error(error, None, None)
    }

    /// Which help a library failure gets; `None` for the GUI's own issues.
    #[cfg(test)]
    pub(super) fn diagnosis(&self) -> Option<&Diagnosis> {
        match self {
            Self::Lib { diagnosis, .. } => Some(diagnosis),
            _ => None,
        }
    }
}

/// Build the "adapter not found" help: what `--adapter` named, and what the
/// bus held when the error arrived. The list is a snapshot either way — the
/// user has to restart with a different `--adapter` to act on it.
fn adapter_not_found_help(
    l10n: &Localizer,
    selector: &str,
    adapters: &ConnectedAdapters,
) -> String {
    let mut msg = tr!(l10n, "connection-adapter-not-matched", selector = selector).into_owned();
    msg.push_str("\n\n");
    msg.push_str(&l10n.help().adapter_lines(adapters).join("\n"));
    if adapters.offers_choice() {
        msg.push_str("\n\n");
        msg.push_str(&tr!(l10n, "connection-adapter-restart"));
    }
    if let Some(hint) = l10n.help().colonless_address_hint(selector) {
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
fn not_identified_help(l10n: &Localizer, bridge: &str, built_in_radio: bool) -> String {
    let link = link_key(Some(Link::of_bridge(bridge)), built_in_radio);
    let mut msg = tr!(l10n, "connection-not-identified", link = link).into_owned();
    msg.push('\n');
    for (steps, names) in dmm_shared::help::activation_groups(&dmm_lib::devices_on_bridge(bridge)) {
        let names: Vec<std::borrow::Cow<'_, str>> = names
            .into_iter()
            .map(|name| l10n.label(TextKind::Model, name))
            .collect();
        msg.push('\n');
        msg.push_str(&names.join(", "));
        msg.push('\n');
        msg.push_str(&l10n.label(TextKind::Activation, steps));
        msg.push('\n');
    }
    msg.push('\n');
    msg.push_str(&tr!(l10n, "connection-not-identified-report"));
    msg.push('\n');
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
        let l10n = self.l10n.as_ref();
        let notice = ConnectionNotice::plain;

        // The probe is still running: the thread was told no meter, and
        // neither a connection nor a failure has come back yet. It spends up to a
        // couple of seconds giving each family its turn to answer, and an
        // empty column for that long reads as a hang.
        if self.connection.detecting
            && self.connection.state == ConnectionState::Connecting
            && self.connection.last_error.is_none()
        {
            return Some(notice(
                NoticeKind::Detecting,
                tr!(l10n, "connection-detecting").into_owned(),
                String::new(),
            ));
        }

        // The reading on screen is the recording's last, and stays: nothing
        // more is coming, which a trace that just stops would not say. Worded
        // for a Connect after the end too, which ends again at once.
        if self.connection.ended {
            return Some(notice(
                NoticeKind::Ended,
                tr!(l10n, "connection-ended").into_owned(),
                String::new(),
            ));
        }

        // Show waiting indicator before error threshold
        if self.connection.waiting_timeouts > 0 && self.connection.last_error.is_none() {
            // A stretch of the recording with nothing in it, which no meter
            // and no cable can be asked about. Said plainly and left there:
            // the file plays on by itself once the gap is over.
            if self.source.replay.is_some() {
                return Some(notice(
                    NoticeKind::Waiting,
                    tr!(l10n, "connection-no-frames").into_owned(),
                    String::new(),
                ));
            }
            let dots = ".".repeat((self.connection.waiting_timeouts as usize % 4) + 1);
            // Padded to a fixed field: in the big meter this title is measured
            // to fit the window, and a line that grows and shrinks four times
            // a second would either re-fit on every dot or overflow by three
            // characters. The padding is invisible either way.
            let title = tr!(l10n, "connection-waiting", dots = format!("{dots:<4}")).into_owned();
            // Under Auto-detect there is no selection to check, so the hint is
            // the other two things that make a quiet meter talk. With a family
            // selected the way out is named too: the value may have been saved
            // by a detected connect rather than picked, and the user swapping
            // meters has no reason to connect the silence to it.
            let body = if self.selected_device().is_some() {
                tr!(l10n, "connection-waiting-check-selection")
            } else {
                tr!(l10n, "connection-waiting-switch-on")
            };
            return Some(notice(NoticeKind::Waiting, title, body.into_owned()));
        }

        let issue = self.connection.last_error.as_ref()?;
        Some(self.issue_notice(issue))
    }

    /// The notice for a connection that failed, one arm per issue: a new
    /// variant has to say what it shows before this compiles.
    fn issue_notice(&self, issue: &ConnectionIssue) -> ConnectionNotice {
        let l10n = self.l10n.as_ref();
        let notice = ConnectionNotice::plain;
        match issue {
            ConnectionIssue::Lib {
                diagnosis,
                error,
                adapters,
            } => self.lib_notice(diagnosis, error, adapters.as_ref()),
            ConnectionIssue::SeveralCables(several) => notice(
                NoticeKind::SeveralCables,
                tr!(l10n, "connection-several-cables").into_owned(),
                format!(
                    "{} {}",
                    l10n.help().several_cables(*several),
                    tr!(l10n, "connection-several-cables-action")
                ),
            ),
            ConnectionIssue::Stopped => notice(
                NoticeKind::Failed,
                tr!(l10n, "connection-failed").into_owned(),
                format!(
                    "{}\n\n{}",
                    tr!(l10n, "connection-acquisition-stopped"),
                    tr!(l10n, "connection-click-connect")
                ),
            ),
            ConnectionIssue::Internal(detail) => notice(
                NoticeKind::Failed,
                tr!(l10n, "connection-failed").into_owned(),
                format!(
                    "{}\n\n{}",
                    tr!(
                        l10n,
                        "connection-internal-error",
                        detail = escape_controls(detail).into_owned()
                    ),
                    tr!(l10n, "connection-click-connect")
                ),
            ),
        }
    }

    /// The notice for a library failure, one arm per [`Diagnosis`], worded
    /// from `error` where the error has the words.
    fn lib_notice(
        &self,
        diagnosis: &Diagnosis,
        error: &dmm_lib::error::Error,
        adapters: Option<&ConnectedAdapters>,
    ) -> ConnectionNotice {
        let l10n = self.l10n.as_ref();
        let notice = ConnectionNotice::plain;
        match diagnosis {
            Diagnosis::NotFound { bluetooth_searched } => {
                // Nothing answered on the links the open path searched. The
                // sections come from the library so the CLI's help and this panel
                // stay the same advice; only the closing line is the GUI's, since
                // the CLI has no Connect button.
                let links = LinksSearched::from_usb_failure(*bluetooth_searched);
                ConnectionNotice {
                    kind: if *bluetooth_searched {
                        NoticeKind::NoMeterFound
                    } else {
                        NoticeKind::UsbNotFound
                    },
                    title: l10n.help().not_found_title(&links),
                    sections: l10n.help().sections(&links),
                    body: tr!(l10n, "connection-click-connect").into_owned(),
                    feedback_link: self.feedback_link(),
                }
            }
            Diagnosis::BluetoothOnlyNotFound { model, activation } => {
                // The same notice, on the radio alone and with the meter's own
                // steps: it has no cable, and no adapter to wake.
                let links = LinksSearched::BluetoothOnly { model, activation };
                ConnectionNotice {
                    kind: NoticeKind::BluetoothOnlyNotFound,
                    title: l10n.help().not_found_title(&links),
                    sections: l10n.help().sections(&links),
                    body: tr!(l10n, "connection-click-connect").into_owned(),
                    feedback_link: self.feedback_link(),
                }
            }
            Diagnosis::BluetoothOnlyUnreachable { model, activation } => ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: l10n.error(error),
                sections: l10n
                    .help()
                    .sections(&LinksSearched::BluetoothOnly { model, activation }),
                body: tr!(l10n, "connection-click-connect").into_owned(),
                feedback_link: None,
            },
            Diagnosis::BluetoothNotSearched(miss) => {
                let message = match miss {
                    // The remedy is the GUI's own switch; the library's
                    // sentence names none.
                    dmm_lib::error::BluetoothOnlyMiss::SwitchedOff => tr!(
                        l10n,
                        "connection-bluetooth-not-searched-off",
                        error = l10n.error(error),
                        hint = l10n.help().gui_bluetooth_off_hint()
                    ),
                    _ => tr!(
                        l10n,
                        "connection-bluetooth-not-searched-why",
                        error = l10n.error(error)
                    ),
                };
                notice(
                    NoticeKind::BluetoothNotSearched,
                    tr!(l10n, "connection-bluetooth-not-searched").into_owned(),
                    message.into_owned(),
                )
            }
            Diagnosis::NothingAtAddress { address } => {
                let links = LinksSearched::BluetoothAt(address);
                ConnectionNotice {
                    kind: NoticeKind::BluetoothNotFound,
                    title: l10n.help().not_found_title(&links),
                    sections: l10n.help().sections(&links),
                    // The GUI cannot scan, so the way back in is a restart with
                    // the address the scan printed.
                    body: tr!(l10n, "connection-restart-with-address").into_owned(),
                    feedback_link: None,
                }
            }
            // Found, or remembered, but the link would not come up: the
            // stack's own words first, then the steps that wake an adapter.
            Diagnosis::AdapterUnreachable { address } => ConnectionNotice {
                kind: NoticeKind::BluetoothUnreachable,
                title: l10n.error(error),
                sections: l10n.help().sections(&LinksSearched::BluetoothAt(address)),
                body: tr!(l10n, "connection-click-connect").into_owned(),
                feedback_link: None,
            },
            Diagnosis::AdapterNotFound => {
                let selector = l10n.adapter_named(error).unwrap_or_default();
                // `from_error` lists the bus for this diagnosis; only an
                // issue built some other way could have no list.
                let unlisted = ConnectedAdapters::Unavailable;
                notice(
                    NoticeKind::AdapterNotFound,
                    tr!(l10n, "connection-adapter-not-found").into_owned(),
                    adapter_not_found_help(l10n, &selector, adapters.unwrap_or(&unlisted)),
                )
            }
            // The link is fine and the probe ran; nothing on the far end
            // spoke a protocol we know. Which link it was is in the body —
            // the title stays one string per notice, which is what the big
            // meter's fit cache keys on.
            Diagnosis::NotIdentified {
                bridge,
                built_in_radio,
            } => notice(
                NoticeKind::NotIdentified,
                l10n.help().no_meter_answered().into_owned(),
                not_identified_help(l10n, bridge, *built_in_radio),
            ),
            Diagnosis::NoResponse => {
                // The link is up but the meter is not responding.
                // The meter this session is talking about: the one picked, or the
                // one detection found. Under Auto-detect, before anything answered,
                // there is neither a model to name nor steps to give.
                // Name the link the session is on: "adapter" now reads as the
                // Bluetooth one, and over a cable it never was one.
                let built_in_radio = self.active_device().is_some_and(|d| d.bluetooth_only());
                let link = link_key(self.connection.link(), built_in_radio);
                let instructions = match self.active_listing() {
                    Some(meter) => tr!(
                        l10n,
                        "connection-no-response",
                        link = link,
                        model = l10n.label(TextKind::Model, meter.display_name()),
                        steps =
                            l10n.label(TextKind::Activation, meter.device.activation_instructions)
                    ),
                    None => tr!(l10n, "connection-no-response-auto"),
                }
                .into_owned();
                notice(
                    NoticeKind::NoResponse,
                    l10n.help().no_response().into_owned(),
                    instructions,
                )
            }
            // The text says what was wrong; the title stays one string per
            // kind, which is what the big meter's fit cache keys on. Text from
            // a meter or a file is escaped so a control byte cannot act. Mid-read
            // the worker keeps reading and the next good frame clears it; at the
            // open the session has ended, so the way back in is said too.
            Diagnosis::BadData => {
                let text = l10n.error(error);
                let body = if self.connection.state.is_linked() {
                    escape_controls(&text).into_owned()
                } else {
                    format!(
                        "{}\n\n{}",
                        escape_controls(&text),
                        tr!(l10n, "connection-click-connect")
                    )
                };
                notice(
                    NoticeKind::BadData,
                    tr!(l10n, "connection-bad-data").into_owned(),
                    body,
                )
            }
            // Anything else — a cable that cannot carry the meter, a
            // permission error — with its text, which is all the user has to
            // go on.
            Diagnosis::Other => notice(
                NoticeKind::Failed,
                tr!(l10n, "connection-failed").into_owned(),
                format!(
                    "{}\n\n{}",
                    escape_controls(&l10n.error(error)),
                    tr!(l10n, "connection-click-connect")
                ),
            ),
        }
    }

    /// The support-level line a "nothing found" notice ends with, for
    /// a picked meter short of verified. Auto-detect names no meter, so there
    /// is no protocol to warn about until one answers.
    fn feedback_link(&self) -> Option<(String, String)> {
        self.selected_profile()
            .filter(|p| !p.stability.is_verified())
            .map(|profile| {
                let model = self
                    .picked_listing()
                    .map_or(profile.display_name(), |l| l.display_name());
                let warning = self
                    .l10n
                    .help()
                    .stability_warning(&self.l10n.label(TextKind::Model, model), profile.stability);
                (
                    tr!(self.l10n, "connection-feedback", warning = warning).into_owned(),
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
    use dmm_lib::error::Error;

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

    /// Nothing answered, on the USB bus alone or on both links.
    fn not_found(bluetooth_searched: bool) -> ConnectionIssue {
        ConnectionIssue::of(Error::NoTransportFound {
            cables: Vec::new(),
            bluetooth_searched,
        })
    }

    /// A CP2110 cable that nothing on answered the probe.
    fn cp2110_silent() -> ConnectionIssue {
        ConnectionIssue::of(Error::DeviceNotIdentified {
            bridge: "CP2110",
            built_in_radio: false,
        })
    }

    /// A frame whose checksum did not match.
    fn checksum_mismatch() -> ConnectionIssue {
        ConnectionIssue::of(Error::ChecksumMismatch {
            expected: 0x01a3,
            actual: 0x0003,
        })
    }

    /// Under Auto-detect the probe says so while it runs; a picked meter
    /// opening has the top bar's Connecting… alone.
    #[test]
    fn auto_detect_shows_detecting_while_connecting() {
        let mut auto = app(registry::AUTO_DEVICE_ID);
        auto.connection.state = ConnectionState::Connecting;
        auto.connection.detecting = true;
        assert_eq!(
            auto.connection_notice().map(|n| n.kind),
            Some(NoticeKind::Detecting)
        );
        let mut picked = app("ut61eplus");
        picked.connection.state = ConnectionState::Connecting;
        assert!(picked.connection_notice().is_none());

        // A replay names its meter, whatever Settings says: nothing is
        // detected, so it is only connecting.
        let mut replay = app(registry::AUTO_DEVICE_ID);
        replay.source.replay = Some(crate::ReplaySource::fixture());
        replay.connect(&crate::app::styled_context());
        assert_eq!(replay.connection.state, ConnectionState::Connecting);
        assert!(replay.connection_notice().is_none());
        replay.disconnect_and_wait();
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

        let n = notice_for(&mut app, not_found(false));
        assert_eq!(n.kind, NoticeKind::UsbNotFound);
        assert_eq!(n.title, "No USB cable found");
        assert!(n.body.contains("Connect"), "got {:?}", n.body);

        let n = notice_for(
            &mut app,
            ConnectionIssue::of(Error::AdapterNotFound("ABC123".to_string())),
        );
        assert_eq!(n.kind, NoticeKind::AdapterNotFound);
        assert_eq!(n.title, "Adapter not found");
        assert!(n.body.contains("ABC123"), "{}", n.body);

        let n = notice_for(&mut app, cp2110_silent());
        assert_eq!(n.kind, NoticeKind::NotIdentified);
        assert_eq!(n.title, "No meter answered");
        assert!(
            n.body.contains("Long press the USB/Hz button"),
            "{}",
            n.body
        );

        let n = notice_for(&mut app, ConnectionIssue::of(Error::Timeout));
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

        let n = notice_for(&mut app, not_found(true));
        assert_eq!(n.kind, NoticeKind::NoMeterFound);
        assert_eq!(n.title, "No meter found over USB or Bluetooth");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link.as_str()).collect();
        assert_eq!(links, ["USB cable", "Bluetooth"]);
        // The big meter shows the title alone and hands the rest to a
        // tooltip, so every step has to reach that one string.
        let help = n.help_text(&crate::l10n::english());
        assert!(help.contains("USB cable: check it is plugged in"), "{help}");
        assert!(help.contains("Bluetooth: turn on Bluetooth"), "{help}");
        assert!(help.ends_with("Click \"Connect\" after resolving the issue."));
    }

    /// An address nothing answered is about that link alone: no cable steps,
    /// and no USB bus listing — nothing on the bus could have answered it.
    #[test]
    fn a_bluetooth_address_is_answered_on_its_own_link() {
        let mut app = app("ut61eplus");
        let issue = ConnectionIssue::of(Error::AdapterNotFound("12:34:56:78:9A:BC".to_string()));
        if !dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            // A build without the radio has no link that value could name.
            return;
        }
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothNotFound);
        assert_eq!(n.title, "No Bluetooth device found at 12:34:56:78:9A:BC");
        let links: Vec<&str> = n.sections.iter().map(|s| s.link.as_str()).collect();
        assert_eq!(links, ["Bluetooth"]);
        assert!(
            n.help_text(&crate::l10n::english())
                .contains("dmm-cli list"),
            "{}",
            n.help_text(&crate::l10n::english())
        );
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
        let stack = || {
            Error::Bluetooth(dmm_lib::error::BluetoothIssue::Stack(
                "Timed out after 10s".to_string(),
            ))
        };
        let issue = ConnectionIssue::from_error(stack(), Some("12:34:56:78:9A:BC"), None);
        if !dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            // A build without the radio has no address to explain it by.
            assert_eq!(issue.diagnosis(), Some(&Diagnosis::Other));
            return;
        }
        let mut app = app("ut61eplus");
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: Timed out after 10s");
        assert_eq!(
            n.sections,
            crate::l10n::english()
                .help()
                .sections(&LinksSearched::BluetoothAt("12:34:56:78:9A:BC"))
        );
        assert_eq!(n.body, "Click \"Connect\" after resolving the issue.");

        // Without an address, or with a USB one, it stays the bare error.
        for adapter in [None, Some("00C5B27A")] {
            let issue = ConnectionIssue::from_error(stack(), adapter, None);
            assert_eq!(issue.diagnosis(), Some(&Diagnosis::Other), "{adapter:?}");
        }
    }

    /// The search names the meter asked for, not the maker's entry it opens as.
    #[test]
    fn a_bluetooth_only_notice_names_the_picked_meter() {
        use dmm_lib::error::BluetoothOnlyMiss;
        let cms061 = registry::find_device("cms061").expect("registry entry");
        let pick = registry::resolve_listing("ut217a").expect("a sold-as row");
        let issue = ConnectionIssue::from_error(
            Error::BluetoothOnly {
                model: cms061.display_name(),
                activation: cms061.activation_instructions,
                miss: BluetoothOnlyMiss::NotInRange,
            },
            None,
            Some(pick),
        );
        let n = notice_for(&mut app("ut217a"), issue);
        assert_eq!(n.title, "No UNI-T UT217A found over Bluetooth");
    }

    /// A meter with the radio built in is answered on the radio alone, with
    /// its own steps, whichever way the open failed — never with the cable
    /// help. A meter behind an adapter keeps the bare stack error.
    #[test]
    fn a_bluetooth_only_meter_gets_its_own_notice() {
        use dmm_lib::error::BluetoothOnlyMiss;
        let ut60bt = registry::find_device("ut60bt").expect("registry entry");
        let own_steps = LinksSearched::BluetoothOnly {
            model: ut60bt.display_name(),
            activation: ut60bt.activation_instructions,
        };
        let missed = |miss| Error::BluetoothOnly {
            model: ut60bt.display_name(),
            activation: ut60bt.activation_instructions,
            miss,
        };
        let mut app = app("ut60bt");

        let issue = ConnectionIssue::of(missed(BluetoothOnlyMiss::NotInRange));
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothOnlyNotFound);
        assert_eq!(n.title, "No UNI-T UT60BT found over Bluetooth");
        assert_eq!(
            n.sections,
            crate::l10n::english().help().sections(&own_steps)
        );
        assert!(
            n.help_text(&crate::l10n::english())
                .contains("Long press SEL"),
            "{}",
            n.help_text(&crate::l10n::english())
        );
        assert!(n.feedback_link.is_some());

        let stack = || Error::Bluetooth(dmm_lib::error::BluetoothIssue::PoweredOff);
        let n = notice_for(
            &mut app,
            ConnectionIssue::from_error(stack(), None, Some(ut60bt.listing())),
        );
        assert_eq!(n.kind, NoticeKind::BluetoothUnreachable);
        assert_eq!(n.title, "Bluetooth: turned off on this computer");
        assert_eq!(
            n.sections,
            crate::l10n::english().help().sections(&own_steps)
        );

        let issue = ConnectionIssue::of(missed(BluetoothOnlyMiss::SwitchedOff));
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BluetoothNotSearched);
        assert_eq!(n.title, "Bluetooth not searched");
        assert_eq!(
            n.body,
            "UNI-T UT60BT connects over Bluetooth only, and Bluetooth is switched off. \
             Tick \"Look for Bluetooth devices\" in Settings (\u{2699})."
        );
        assert!(n.sections.is_empty());

        let ut61eplus = registry::find_device("ut61eplus").expect("registry entry");
        let issue = ConnectionIssue::from_error(stack(), None, Some(ut61eplus.listing()));
        assert_eq!(issue.diagnosis(), Some(&Diagnosis::Other));
    }

    /// A failure on record is worded on every frame, so a language switch
    /// reaches the notice's body and the reconnect hover, not only the parts
    /// around them.
    #[test]
    fn a_notice_follows_a_language_switch() {
        let options = dmm_shared::l10n::Options {
            pseudo: true,
            ..Default::default()
        };
        let pseudo = std::sync::Arc::new(Localizer::new(
            &dmm_shared::l10n::EN_US,
            &crate::l10n::resources(),
            &options,
        ));
        let mut app = app("ut61eplus");
        for issue in [cp2110_silent, || {
            ConnectionIssue::of(Error::AdapterNotFound("ABC123".to_string()))
        }] {
            app.l10n = crate::l10n::english();
            let english = notice_for(&mut app, issue()).body;
            assert!(!english.starts_with('['), "{english}");
            app.l10n = pseudo.clone();
            let body = app.connection_notice().expect("still on record").body;
            assert!(body.starts_with('['), "{body}");
        }

        app.l10n = crate::l10n::english();
        app.connection.reconnect_last_error = Some(Error::LinkLost);
        let english = app.reconnecting_hover();
        app.l10n = pseudo;
        let hover = app.reconnecting_hover();
        assert_ne!(hover, english);
        // The error itself is worded again too, not only the sentence around it.
        let quoted = crate::l10n::english().error(&Error::LinkLost);
        assert!(!hover.contains(&quoted), "{hover}");
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
        let mut app = app("ut61eplus");
        let n = notice_for(
            &mut app,
            ConnectionIssue::of(Error::AdapterNotFound("ABC123".to_string())),
        );
        assert_eq!(n.kind, NoticeKind::AdapterNotFound);
        assert!(n.body.contains("ABC123"), "got {}", n.body);
        // A USB device that is no cable gets the same help, quoting why.
        let n = notice_for(
            &mut app,
            ConnectionIssue::of(Error::NotAnAdapter("/dev/hidraw3".to_string())),
        );
        assert_eq!(n.kind, NoticeKind::AdapterNotFound);
        assert!(
            n.body
                .contains("'/dev/hidraw3 (device exists but is not a supported USB adapter)'"),
            "got {}",
            n.body
        );
    }

    /// Only a real list of what the bus held is worth the hint to restart
    /// with another selector; "nothing connected" and "couldn't look" are
    /// complete on their own.
    #[test]
    fn only_a_listed_bus_gets_the_restart_hint() {
        let l10n = crate::l10n::english();
        let restart = tr!(l10n, "connection-adapter-restart").into_owned();
        let listed = ConnectedAdapters::Listed(vec!["  [0] a cable".to_string()]);
        let help = adapter_not_found_help(&l10n, "ABC123", &listed);
        assert!(help.contains("[0] a cable"), "{help}");
        assert!(help.contains(&restart), "{help}");
        for adapters in [ConnectedAdapters::None, ConnectedAdapters::Unavailable] {
            let help = adapter_not_found_help(&l10n, "ABC123", &adapters);
            assert!(!help.contains(&restart), "{adapters:?}: {help}");
        }
    }

    /// `DeviceNotIdentified` is an `ErrorKind::Timeout`, which would land it
    /// in `Other` and print the "no response from meter" block naming a
    /// device nobody selected. It is matched by variant for that reason.
    #[test]
    fn nothing_answering_the_probe_is_its_own_case() {
        let issue = ConnectionIssue::of(Error::DeviceNotIdentified {
            bridge: "CP2110",
            built_in_radio: false,
        });
        let n = notice_for(&mut app("ut61eplus"), issue);
        assert_eq!(n.kind, NoticeKind::NotIdentified);
        let help = n.body;
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
        let issue = ConnectionIssue::of(Error::DeviceNotIdentified {
            bridge: "BU-86X",
            built_in_radio: false,
        });
        let n = notice_for(&mut app("ut61eplus"), issue);
        assert_eq!(n.kind, NoticeKind::NotIdentified);
        let help = n.body;
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
        let help = not_identified_help(&crate::l10n::english(), dmm_lib::BLUETOOTH, false);
        assert!(
            help.starts_with("The Bluetooth adapter is connected"),
            "got {help}"
        );
        // The meters UNI-T lists on the adapter, with their steps. The steps
        // themselves are the meter's own and name its USB socket, which is
        // where the adapter plugs in.
        assert!(help.contains("UT61E+"), "got {help}");
        // A meter with the radio built in answered the open: no adapter.
        let help = not_identified_help(&crate::l10n::english(), dmm_lib::BLUETOOTH, true);
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
            let n = notice_for(&mut app, ConnectionIssue::of(Error::Timeout));
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
        let help = not_identified_help(&crate::l10n::english(), "CP2110", false);
        assert!(
            help.contains("UNI-T UT61E+, UT61B+, UT61D+, UT161B, UT161D, UT161E"),
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
                not_found(bluetooth_searched).diagnosis(),
                Some(&Diagnosis::NotFound { bluetooth_searched })
            );
        }
    }

    /// A Bluetooth address takes the link's own case, not the bus listing:
    /// `connected_adapters()` would walk every HID device for an answer that
    /// could not contain the address anyway.
    #[test]
    fn an_unanswered_address_is_classified_as_a_bluetooth_failure() {
        let issue = ConnectionIssue::of(Error::AdapterNotFound("12:34:56:78:9A:BC".to_string()));
        let ConnectionIssue::Lib {
            diagnosis,
            adapters,
            ..
        } = issue
        else {
            panic!("a library failure");
        };
        if dmm_lib::is_bluetooth_selector("12:34:56:78:9A:BC") {
            assert_eq!(
                diagnosis,
                Diagnosis::NothingAtAddress {
                    address: "12:34:56:78:9A:BC".to_string()
                }
            );
            assert!(adapters.is_none(), "the bus was walked for an address");
        } else {
            // A build without the radio has no link that value could name.
            assert_eq!(diagnosis, Diagnosis::AdapterNotFound);
        }
    }

    /// A timeout is the quiet meter, whatever reported it.
    #[test]
    fn a_timeout_is_no_response() {
        let issue = ConnectionIssue::of(Error::Timeout);
        assert_eq!(issue.diagnosis(), Some(&Diagnosis::NoResponse));
    }

    /// Any other failure shows its own words under a fixed title, never the
    /// quiet meter's steps: a cable that cannot carry the meter, or a radio
    /// switched off, has nothing to do with data transmission.
    #[test]
    fn other_errors_show_their_message() {
        let mut app = app("ut61eplus");
        let ut61eplus = registry::find_device("ut61eplus").expect("registry entry");
        for err in [
            Error::WrongCable {
                cable: "BU-86X",
                model: "UNI-T UT61E+",
            },
            Error::Bluetooth(dmm_lib::error::BluetoothIssue::PoweredOff),
        ] {
            let text = crate::l10n::english().error(&err);
            let issue = ConnectionIssue::from_error(err, None, Some(ut61eplus.listing()));
            assert_eq!(issue.diagnosis(), Some(&Diagnosis::Other));
            let n = notice_for(&mut app, issue);
            assert_eq!(n.kind, NoticeKind::Failed);
            assert_eq!(n.title, "Connection failed");
            assert_eq!(
                n.body,
                format!("{text}\n\nClick \"Connect\" after resolving the issue.")
            );
            assert!(!n.body.contains("enable data transmission"), "{}", n.body);
        }
    }

    /// Data the meter sent that could not be read is said as such: the meter
    /// is answering, so there is nothing to switch on, and under a live
    /// reading nothing to reconnect.
    #[test]
    fn unreadable_data_is_not_a_quiet_meter() {
        let mut app = app("ut61eplus");
        let issue = checksum_mismatch();
        assert_eq!(issue.diagnosis(), Some(&Diagnosis::BadData));
        app.connection.state = ConnectionState::Connected;
        let n = notice_for(&mut app, issue);
        assert_eq!(n.kind, NoticeKind::BadData);
        assert_eq!(n.title, "Meter sent data that could not be read");
        assert_eq!(n.body, "checksum mismatch: expected 0x01a3, got 0x0003");
        assert!(n.sections.is_empty());
    }

    /// Unreadable data that ended the open leaves no session running, so
    /// the notice names the way back in, as every other failure does.
    #[test]
    fn unreadable_data_at_the_open_says_to_click_connect() {
        let mut app = app("ut61eplus");
        app.connection.state = ConnectionState::Disconnected;
        let n = notice_for(&mut app, checksum_mismatch());
        assert_eq!(n.kind, NoticeKind::BadData);
        assert_eq!(
            n.body,
            "checksum mismatch: expected 0x01a3, got 0x0003\n\nClick \"Connect\" after resolving the issue."
        );
    }

    /// The text can carry bytes from the meter or a file: a control
    /// character is shown as its escape, not acted on.
    #[test]
    fn a_control_character_in_the_text_is_escaped() {
        let mut app = app("ut61eplus");
        app.connection.state = ConnectionState::Connected;
        let n = notice_for(
            &mut app,
            ConnectionIssue::of(Error::InvalidResponse {
                message: "\u{1b}[2J".to_string(),
                raw: Vec::new(),
            }),
        );
        assert_eq!(n.body, "invalid response: \\u{1b}[2J");
    }

    /// A message that merely mentions the phrase must not be mistaken for
    /// the adapter case — the old `contains` probe would have matched it,
    /// and the prefix probe that replaced it would have matched the same
    /// text arriving with an "adapter not found: " prefix from elsewhere.
    #[test]
    fn a_mention_of_the_phrase_is_not_the_adapter_case() {
        let issue = ConnectionIssue::of(Error::UnknownDevice(
            "meter reports adapter not found somewhere".to_string(),
        ));
        assert_eq!(issue.diagnosis(), Some(&Diagnosis::Other));
    }
}
