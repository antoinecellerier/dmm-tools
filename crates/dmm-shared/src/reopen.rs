//! What a lost link may come back on, decided once for both binaries. Each
//! keeps its own wait around it: their cancel sources and control handling
//! differ, and a shared loop's hooks would be as long as either loop.
//!
//! A reopen goes back to the link the session first opened, so its readings
//! stay this meter's and a replay header's `link` stays true:
//!
//! - A Bluetooth session, by address.
//! - A cable, by its serial number when the listing shows that serial on one
//!   path. Its path is no use, being renumbered on replug. Swapping to another
//!   cable is then the user's Disconnect and Connect, as for an adapter.
//! - A cable session never falls through to the radio.
//! - A cable with no serial, or one it shares, has nothing to pin it by, so
//!   the open could take another meter's. With several cables the open may
//!   take, there is no reconnect.
//! - `--adapter` is taken at its word when none of that applies, a path
//!   included, though a replug can renumber one.
//!
//! That leaves one gap: a session with no serial to pin to takes the one cable
//! listed, so another meter's plugged in while ours is out looks like ours
//! coming back. So does one with our serial, for a session pinned to it.

use dmm_lib::error::Error;
use dmm_lib::protocol::registry::SelectableDevice;
use dmm_lib::transport::Link;
use dmm_lib::{DeviceInfo, OpenOptions};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

/// `count` USB cables a session could open. From [`reopen_pin`] or
/// [`ReopenPin::check`], why a lost link can't be reopened, as nothing says
/// which is this meter's. Held as data, and worded where it is shown
/// ([`Help::several_cables`](crate::help::Help::several_cables)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeveralCables {
    pub count: usize,
}

/// What one attempt to reopen a link should do; see [`ReopenPin::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    Open,
    /// Skip this attempt's open: the listing is ambiguous, and the open could
    /// take the other meter's cable. One odd listing doesn't end a session.
    Hold,
    /// Skip this attempt's open: this cable's serial is gone but another the
    /// open could take is listed. Say so, once, as the wait looks stuck.
    WaitForOwnCable,
    /// Give up: an ambiguous listing twice in a row.
    Refuse(SeveralCables),
}

/// How a session's reopen found its link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pinned {
    /// A Bluetooth address or `--adapter`.
    ByName,
    /// The opened cable's own serial.
    BySerial,
    /// Nothing: whichever cable the open takes, while there is only one.
    Unpinned,
}

/// What a session reopens its link with. Chosen once, at the first open: with
/// this cable out, a serial it shares looks unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReopenPin {
    adapter: Option<String>,
    bluetooth: bool,
    pinned: Pinned,
    /// The kinds of cable the open may take.
    kinds: Vec<&'static str>,
    /// The last check found the listing ambiguous.
    held: bool,
    /// See [`beside_others`](Self::beside_others).
    beside: Option<SeveralCables>,
}

/// Choose how a session opened on `first_link` reopens it. `device` is the
/// entry the reopen opens, `None` when it detects the meter again and so may
/// take any cable. `listed` gives what [`dmm_lib::list_devices`] shows now,
/// called only if it matters; one that fails counts as empty, and the session
/// reconnects as asked.
pub fn reopen_pin(
    device: Option<&SelectableDevice>,
    adapter_flag: Option<&str>,
    bluetooth_setting: bool,
    first_link: Option<Link>,
    bluetooth_selector: Option<&str>,
    cable_serial: Option<&str>,
    listed: impl FnOnce() -> Vec<DeviceInfo>,
) -> Result<ReopenPin, SeveralCables> {
    let kinds = dmm_lib::cable_kinds(device);
    let bluetooth = bluetooth_setting && first_link != Some(Link::UsbCable);
    let pin = |adapter: Option<&str>, pinned| ReopenPin {
        adapter: adapter.map(str::to_owned),
        bluetooth,
        pinned,
        kinds: kinds.clone(),
        held: false,
        beside: None,
    };
    if let Some(selector) = bluetooth_selector {
        return Ok(pin(Some(selector), Pinned::ByName));
    }
    let listed = listed();
    // Over a flag: `--adapter` may be a path, and this is the cable it opened.
    if let Some(serial) = cable_serial.filter(|serial| paths_of_serial(&listed, serial) == 1) {
        let count = cables_of(&listed, &kinds);
        return Ok(ReopenPin {
            beside: (adapter_flag.is_none() && count >= 2).then_some(SeveralCables { count }),
            ..pin(Some(serial), Pinned::BySerial)
        });
    }
    // A flag naming a serial two cables carry opens whichever comes first.
    if let Some(adapter) = adapter_flag.filter(|flag| paths_of_serial(&listed, flag) < 2) {
        return Ok(pin(Some(adapter), Pinned::ByName));
    }
    match cables_of(&listed, &kinds) {
        count @ 2.. => Err(SeveralCables { count }),
        _ => Ok(pin(None, Pinned::Unpinned)),
    }
}

impl ReopenPin {
    /// The other cables the first open could have taken, with nothing naming
    /// the one it did: it reconnects to that one, which may not be the meter
    /// the user meant.
    pub fn beside_others(&self) -> Option<SeveralCables> {
        self.beside
    }

    /// The options for this reopen's open.
    pub fn options<'a>(&'a self, stop: Option<&'a AtomicBool>) -> OpenOptions<'a> {
        OpenOptions {
            adapter: self.adapter.as_deref(),
            bluetooth: self.bluetooth,
            stop,
        }
    }

    /// What this attempt should do, given what `listed` gives now, called
    /// only if it matters.
    pub fn check(&mut self, listed: impl FnOnce() -> Vec<DeviceInfo>) -> Attempt {
        if self.pinned == Pinned::ByName {
            return Attempt::Open;
        }
        let listed = listed();
        let cables = match self.adapter.as_deref() {
            Some(serial) => paths_of_serial(&listed, serial),
            None => cables_of(&listed, &self.kinds),
        };
        if cables >= 2 {
            let again = std::mem::replace(&mut self.held, true);
            return if again {
                Attempt::Refuse(SeveralCables { count: cables })
            } else {
                Attempt::Hold
            };
        }
        self.held = false;
        if self.pinned == Pinned::BySerial && cables == 0 && cables_of(&listed, &self.kinds) > 0 {
            return Attempt::WaitForOwnCable;
        }
        Attempt::Open
    }

    /// A failed open as the user should read it. Under a serial pin an adapter
    /// not found is this cable being out: the user may have named no adapter,
    /// and the message would show them a serial for it.
    pub fn open_error(&self, error: Error) -> Error {
        match error {
            Error::AdapterNotFound(_) if self.pinned == Pinned::BySerial => {
                Error::NoTransportFound {
                    cables: self.kinds.clone(),
                    bluetooth_searched: false,
                }
            }
            other => other,
        }
    }
}

/// The USB cables plugged in now, for [`reopen_pin`] and [`ReopenPin::check`].
/// A listing that fails counts as none: the session reconnects as asked.
pub fn listed_cables() -> Vec<DeviceInfo> {
    dmm_lib::list_devices().unwrap_or_default()
}

/// The USB cables `listed` holds of `kinds`. By path, as hidapi lists a record
/// per usage pair, so one cable can list several times. Interfaces of a
/// composite device have their own paths and do count apart.
fn cables_of(listed: &[DeviceInfo], kinds: &[&str]) -> usize {
    paths_where(listed, |cable| kinds.contains(&cable.transport))
}

/// How many paths in `listed` carry `serial`.
fn paths_of_serial(listed: &[DeviceInfo], serial: &str) -> usize {
    paths_where(listed, |cable| cable.serial.as_deref() == Some(serial))
}

fn paths_where(listed: &[DeviceInfo], keep: impl Fn(&DeviceInfo) -> bool) -> usize {
    listed
        .iter()
        .filter(|cable| keep(cable))
        .map(|cable| cable.path.as_str())
        .collect::<HashSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dmm_lib::protocol::registry;

    const BLUETOOTH_ADDRESS: &str = "12:34:56:78:9A:BC";

    fn ut61eplus() -> Option<&'static SelectableDevice> {
        registry::find_device("ut61eplus")
    }

    fn cable(path: &str, transport: &'static str, serial: Option<&str>) -> DeviceInfo {
        DeviceInfo {
            path: path.to_owned(),
            product: None,
            serial: serial.map(str::to_owned),
            transport,
            not_heard: false,
        }
    }

    fn cp2110(path: &str, serial: Option<&str>) -> DeviceInfo {
        cable(path, "CP2110", serial)
    }

    /// A cable session with no flag on `listed`, opened on a cable with
    /// `serial`.
    fn on_cable(serial: Option<&str>, listed: &[DeviceInfo]) -> Result<ReopenPin, SeveralCables> {
        reopen_pin(
            ut61eplus(),
            None,
            true,
            Some(Link::UsbCable),
            None,
            serial,
            || listed.to_vec(),
        )
    }

    fn adapter(pin: &ReopenPin) -> Option<String> {
        pin.options(None).adapter.map(str::to_owned)
    }

    /// [`ReopenPin::check`] on what `listed` holds.
    fn check(pin: &mut ReopenPin, listed: &[DeviceInfo]) -> Attempt {
        pin.check(|| listed.to_vec())
    }

    /// The cables counted are every kind the open falls back to, not only
    /// the meter's own: a UT61E+ opens on another meter's CH9329 too, but
    /// not on a Brymen BU-86X.
    #[test]
    fn the_cables_counted_are_the_ones_the_open_may_take() {
        let kinds = dmm_lib::cable_kinds(ut61eplus());
        let cp = cp2110("a", None);
        let (bu, ch) = (cable("b", "BU-86X", None), cable("b", "CH9329", None));
        assert_eq!(cables_of(std::slice::from_ref(&cp), &kinds), 1);
        assert_eq!(cables_of(&[cp.clone(), bu.clone()], &kinds), 1);
        assert_eq!(cables_of(&[cp, ch], &kinds), 2);
    }

    /// An open with no meter named takes the first cable of any kind, so
    /// every one of them counts.
    #[test]
    fn a_session_that_detects_again_counts_every_cable() {
        let listed = [cp2110("a", None), cable("b", "BU-86X", None)];
        let pin = reopen_pin(None, None, true, Some(Link::UsbCable), None, None, || {
            listed.to_vec()
        });
        assert_eq!(pin, Err(SeveralCables { count: 2 }));
    }

    /// hidapi lists a record per usage pair: one cable, not two.
    #[test]
    fn one_cable_listed_twice_is_one_cable() {
        let listed = [cp2110("a", Some("S1")), cp2110("a", Some("S1"))];
        assert_eq!(paths_of_serial(&listed, "S1"), 1);
        assert_eq!(
            adapter(&on_cable(Some("S1"), &listed).unwrap()),
            Some("S1".into())
        );
        let listed = [cp2110("a", None), cp2110("a", None)];
        assert_eq!(adapter(&on_cable(None, &listed).unwrap()), None);
    }

    /// With a serial its own, a cable reopens by it however many others are
    /// plugged in. The refusal is left to the cables that can't be told apart.
    #[test]
    fn a_cable_with_a_serial_of_its_own_reopens_by_it() {
        let listed = [cp2110("a", Some("S1")), cp2110("b", Some("S2"))];
        let pin = on_cable(Some("S1"), &listed).unwrap();
        assert_eq!(adapter(&pin), Some("S1".into()));
        assert!(!pin.options(None).bluetooth);
    }

    /// It says when others were there to take instead, unless the user named
    /// this one, and not for a cable of another kind or one on its own.
    #[test]
    fn a_pinned_cable_says_when_others_were_there_to_take() {
        let two = [cp2110("a", Some("S1")), cp2110("b", Some("S2"))];
        let others = on_cable(Some("S1"), &two).unwrap().beside_others();
        assert_eq!(others, Some(SeveralCables { count: 2 }));

        let named = reopen_pin(
            ut61eplus(),
            Some("S1"),
            true,
            Some(Link::UsbCable),
            None,
            Some("S1"),
            || two.to_vec(),
        );
        assert_eq!(named.unwrap().beside_others(), None);

        let other_kind = [cp2110("a", Some("S1")), cable("b", "BU-86X", Some("S2"))];
        let pin = on_cable(Some("S1"), &other_kind).unwrap();
        assert_eq!(pin.beside_others(), None);
        let alone = [cp2110("a", Some("S1"))];
        assert_eq!(on_cable(Some("S1"), &alone).unwrap().beside_others(), None);
    }

    /// No serial, a shared serial, and a serial the listing spells otherwise
    /// all fall back to the count.
    #[test]
    fn a_cable_that_cant_be_told_apart_is_refused_beside_another() {
        let refused = Err(SeveralCables { count: 2 });
        let nameless = [cp2110("a", None), cp2110("b", None)];
        assert_eq!(on_cable(None, &nameless), refused);
        let shared = [cp2110("a", Some("S1")), cp2110("b", Some("S1"))];
        assert_eq!(on_cable(Some("S1"), &shared), refused);
        let other = [cp2110("a", Some("S1")), cp2110("b", Some("S2"))];
        assert_eq!(on_cable(Some("S3"), &other), refused);
        // Alone, each has nothing to mistake it for and opens unpinned.
        let alone = [cp2110("a", None)];
        assert_eq!(adapter(&on_cable(None, &alone).unwrap()), None);
        assert_eq!(adapter(&on_cable(Some("S3"), &alone).unwrap()), None);
    }

    /// A listing that fails is an empty one: the session reconnects as asked.
    #[test]
    fn an_empty_listing_refuses_nothing() {
        let mut pin = on_cable(Some("S1"), &[]).unwrap();
        assert_eq!(adapter(&pin), None);
        assert_eq!(check(&mut pin, &[]), Attempt::Open);
    }

    /// `--adapter` names the cable when nothing better does, and a path stands
    /// aside for the serial of the cable it opened.
    #[test]
    fn a_flag_pins_unless_the_cable_has_a_serial_of_its_own() {
        let flag = |flag, serial, listed: &[DeviceInfo]| {
            reopen_pin(
                ut61eplus(),
                Some(flag),
                true,
                Some(Link::UsbCable),
                None,
                serial,
                || listed.to_vec(),
            )
        };
        let path = "/dev/hidraw3";
        let two = [cp2110("a", Some("S1")), cp2110("b", Some("S2"))];
        assert_eq!(
            adapter(&flag(path, Some("S1"), &two).unwrap()),
            Some("S1".into())
        );
        let shared = [cp2110("a", Some("S1")), cp2110("b", Some("S1"))];
        let mut pin = flag(path, Some("S1"), &shared).unwrap();
        assert_eq!(adapter(&pin), Some(path.into()));
        assert_eq!(check(&mut pin, &shared), Attempt::Open, "the user named it");
    }

    /// A flag naming a serial two cables carry names neither: the open would
    /// take whichever comes first.
    #[test]
    fn a_flag_naming_a_shared_serial_is_refused_like_no_flag() {
        let shared = [cp2110("a", Some("S1")), cp2110("b", Some("S1"))];
        let pin = reopen_pin(
            ut61eplus(),
            Some("S1"),
            true,
            Some(Link::UsbCable),
            None,
            Some("S1"),
            || shared.to_vec(),
        );
        assert_eq!(pin, Err(SeveralCables { count: 2 }));
    }

    /// A listing that failed says nothing against the flag: the cable it
    /// named stays named, even when that is this cable's own serial.
    #[test]
    fn a_flag_survives_a_listing_that_failed() {
        let pin = reopen_pin(
            ut61eplus(),
            Some("S1"),
            true,
            Some(Link::UsbCable),
            None,
            Some("S1"),
            Vec::new,
        )
        .unwrap();
        assert_eq!(adapter(&pin), Some("S1".into()));
    }

    /// A session with a name to go back to never looks at the bus, which
    /// enumerates it: a Bluetooth link is retried every two seconds for as
    /// long as an adapter stays off.
    #[test]
    fn a_session_pinned_by_name_lists_nothing() {
        let listing = || -> Vec<DeviceInfo> { panic!("nothing to count") };
        let mut pin = reopen_pin(
            ut61eplus(),
            None,
            true,
            Some(Link::Bluetooth),
            Some(BLUETOOTH_ADDRESS),
            None,
            listing,
        )
        .unwrap();
        assert_eq!(pin.check(listing), Attempt::Open);
    }

    /// A Bluetooth session goes back by address and takes no count, and only
    /// a cable session has the radio taken away from it.
    #[test]
    fn a_session_keeps_the_link_it_opened_on() {
        let two = [cp2110("a", None), cp2110("b", None)];
        let mut pin = reopen_pin(
            ut61eplus(),
            None,
            true,
            Some(Link::Bluetooth),
            Some(BLUETOOTH_ADDRESS),
            None,
            || two.to_vec(),
        )
        .unwrap();
        assert_eq!(adapter(&pin), Some(BLUETOOTH_ADDRESS.into()));
        assert_eq!(check(&mut pin, &two), Attempt::Open);

        let radio = |first_link, setting| {
            reopen_pin(ut61eplus(), None, setting, first_link, None, None, Vec::new)
                .unwrap()
                .options(None)
                .bluetooth
        };
        assert!(!radio(Some(Link::UsbCable), true), "a cable stays a cable");
        assert!(radio(None, true), "nothing known keeps the setting");
        assert!(!radio(None, false));
    }

    /// One odd listing skips its attempt rather than risk the open, and only
    /// a second in a row ends the session.
    #[test]
    fn a_second_cable_holds_then_refuses() {
        let one = [cp2110("a", None)];
        let two = [cp2110("a", None), cp2110("b", None)];
        let mut pin = on_cable(None, &one).unwrap();
        assert_eq!(check(&mut pin, &one), Attempt::Open);
        assert_eq!(check(&mut pin, &two), Attempt::Hold);
        assert_eq!(
            check(&mut pin, &one),
            Attempt::Open,
            "a good listing resets it"
        );
        assert_eq!(check(&mut pin, &two), Attempt::Hold);
        assert_eq!(
            check(&mut pin, &two),
            Attempt::Refuse(SeveralCables { count: 2 })
        );
    }

    /// A serial that turns up twice is no pin any more: the open would take
    /// whichever comes first.
    #[test]
    fn a_pinned_serial_listed_twice_holds_then_refuses() {
        let mut pin = on_cable(Some("S1"), &[cp2110("a", Some("S1"))]).unwrap();
        let doubled = [cp2110("b", Some("S1")), cp2110("c", Some("S1"))];
        assert_eq!(check(&mut pin, &doubled), Attempt::Hold);
        assert_eq!(
            check(&mut pin, &doubled),
            Attempt::Refuse(SeveralCables { count: 2 })
        );
    }

    /// With this cable out and nothing else plugged in, the open runs and
    /// fails as it always did. With another plugged in, it waits for its own.
    #[test]
    fn a_pinned_session_waits_for_its_own_cable() {
        let mut pin = on_cable(Some("S1"), &[cp2110("a", Some("S1"))]).unwrap();
        assert_eq!(check(&mut pin, &[]), Attempt::Open);
        let other = [cp2110("b", Some("S2"))];
        assert_eq!(check(&mut pin, &other), Attempt::WaitForOwnCable);
        // A cable this meter can't use is no reason to say so.
        assert_eq!(
            check(&mut pin, &[cable("c", "BU-86X", None)]),
            Attempt::Open
        );
        // Back, under its new path.
        let back = [cp2110("b", Some("S2")), cp2110("c", Some("S1"))];
        assert_eq!(check(&mut pin, &back), Attempt::Open);
        assert_eq!(adapter(&pin), Some("S1".into()));
    }

    /// The pin holds across a stretch where a shared serial would look unique:
    /// with ours out, the other cable alone carries it.
    #[test]
    fn a_session_with_nothing_to_pin_stays_unpinned() {
        let mut pin = on_cable(Some("S1"), &[cp2110("a", None)]).unwrap();
        check(&mut pin, &[cp2110("b", Some("S1"))]);
        assert_eq!(adapter(&pin), None);
    }

    /// The user never named an adapter, so the serial stays out of what they
    /// read. An adapter they did name is theirs to see.
    #[test]
    fn a_pinned_cable_out_reads_as_the_cable_being_out() {
        let not_found = || Error::AdapterNotFound("S1".into());
        let by_serial = on_cable(Some("S1"), &[cp2110("a", Some("S1"))]).unwrap();
        let shown = by_serial.open_error(not_found()).to_string();
        assert!(!shown.contains("S1"), "{shown}");
        assert!(matches!(
            by_serial.open_error(not_found()),
            Error::NoTransportFound {
                bluetooth_searched: false,
                ..
            }
        ));
        assert!(matches!(
            by_serial.open_error(Error::Timeout),
            Error::Timeout
        ));

        let by_flag = reopen_pin(
            ut61eplus(),
            Some("S1"),
            true,
            Some(Link::UsbCable),
            None,
            None,
            Vec::new,
        )
        .unwrap();
        assert!(matches!(
            by_flag.open_error(not_found()),
            Error::AdapterNotFound(_)
        ));
    }
}
