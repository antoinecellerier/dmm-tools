//! Finding a Bluetooth peer: the devices the host knows, a scan, and which
//! of them the caller takes.

use super::issc::ADAPTER_NAME_PREFIX;
use super::{SCAN_POLL, SCAN_WINDOW, stack_error};
use crate::error::Result;
use crate::transport::{BluetoothPeers, name_matches};
use btleplug::api::{Central, Peripheral as _, RetrievePeripheralsOptions, ScanFilter};
use btleplug::platform::{Adapter, Peripheral};
#[cfg(windows)]
use log::info;
use log::{debug, warn};

/// A peripheral the adapter knows about, with what the platform says about it.
pub(super) struct Candidate {
    pub(super) peripheral: Peripheral,
    /// Platform identifier: a D-Bus path on Linux, a UUID on macOS.
    id: String,
    /// Bluetooth address, empty where the platform exposes none (macOS).
    address: String,
    /// The name to show: the host's alias for the device where it has one.
    pub(super) name: Option<String>,
    /// The name the search took the peer by, for the registry to look up
    /// (`name_matches`); for a peer picked by address, the name it
    /// advertises, else [`Candidate::name`].
    pub(super) taken_by: Option<String>,
    /// How sure we are that it can answer, which orders the candidates.
    pub(super) standing: Standing,
}

impl Candidate {
    /// What `list` prints and `--adapter` takes: the address where there is
    /// one, otherwise the platform's own identifier.
    pub(super) fn selector(&self) -> String {
        if self.address.is_empty() {
            self.id.clone()
        } else {
            self.address.clone()
        }
    }

    /// The name to show, falling back to the selector for a peer that
    /// advertised none.
    pub(super) fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.selector())
    }
}

/// Why a candidate is worth trying, best first: the derived order is the
/// order candidates are tried and listed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Standing {
    /// Connected to this host: usable at once, no scan.
    Connected,
    /// Heard advertising during our scan, so awake.
    Heard,
    /// Only in the platform's list of known devices — paired, or on BlueZ
    /// any device it still caches. It may be asleep (research doc §4), or
    /// awake and missed by the scan (§4 again): only a connect tells.
    Known,
}

/// Which peers a search is after.
#[derive(Clone, Copy)]
pub(super) enum Target<'a> {
    /// The one at this address or platform id, whatever its name.
    At(&'a str),
    /// Any whose name the caller takes.
    Named(&'a BluetoothPeers),
}

impl<'a> Target<'a> {
    /// The address or platform id named, if one was.
    pub(super) fn selector(self) -> Option<&'a str> {
        match self {
            Target::At(selector) => Some(selector),
            Target::Named(_) => None,
        }
    }
}

/// How many matches a search wants back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Match {
    /// The best match, for opening (see [`search`]). Only that one is tried.
    First,
    /// Everything that matched, for listing.
    All,
}

/// Look for peers: the ones this host already knows, then a scan.
///
/// With no address named, a peer the caller takes is picked in [`Standing`]
/// order: one connected to this host, found without a scan; else the first
/// heard advertising during the scan; else a known one that was not heard,
/// tried by address. That last step is there because the scan misses awake adapters:
/// btleplug's BlueZ backend scans with BR/EDR and LE interleaved and offers
/// no LE-only mode, and in that mode BlueZ rarely hears our adapter, while a
/// connect by address listens on LE alone and reaches it (research doc §4).
/// An adapter that really is asleep costs one bounded connect.
///
/// An address the user named is taken from the known devices or the scan,
/// whatever its standing, since they asked for that one.
pub(super) async fn search(
    adapter: &Adapter,
    target: Target<'_>,
    want: Match,
) -> Result<Vec<Candidate>> {
    // A signal strength in this list is another program's discovery, not
    // ours, so only a connection lifts a known device above `Known`.
    let known = usable(known_peripherals(adapter).await, target, false).await;
    // Opening stops here when the known list already settles it: a connected
    // peer, or the named one. Listing scans regardless, so a peer
    // advertising in range shows up beside the known ones.
    let settled = |found: &[Candidate]| match target {
        Target::At(_) => !found.is_empty(),
        Target::Named(_) => found.iter().any(|c| c.standing < Standing::Known),
    };
    if want == Match::First && settled(&known) {
        return Ok(keep_matches(known, want));
    }

    debug!("Bluetooth: scanning for {SCAN_WINDOW:?}");
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(stack_error)?;
    let deadline = tokio::time::Instant::now() + SCAN_WINDOW;
    let scanned = loop {
        tokio::time::sleep(SCAN_POLL).await;
        // Read while the scan runs: the signal strength that says a device
        // was heard is gone once it stops.
        let seen = adapter.peripherals().await.unwrap_or_default();
        let scanned = usable(seen, target, true).await;
        // `All` keeps scanning to the end of the window: a second peer
        // that answers late still belongs in the list.
        if (want == Match::First && settled(&scanned)) || tokio::time::Instant::now() >= deadline {
            break scanned;
        }
    };
    // The scan is the host's, not ours: leave it off however the search went.
    if let Err(e) = adapter.stop_scan().await {
        debug!("Bluetooth: could not stop the scan: {e}");
    }
    // A backend whose scan results do not include its known devices would
    // otherwise drop the known ones; one that lists a device in both keeps
    // its better standing.
    let mut all = known;
    for c in scanned {
        match all.iter_mut().find(|k| k.id == c.id) {
            Some(k) if c.standing < k.standing => *k = c,
            Some(_) => {}
            None => all.push(c),
        }
    }
    Ok(keep_matches(all, want))
}

/// The peripherals the platform already knows about, with no scan.
///
/// On BlueZ that is every device the daemon caches: paired ones, and for
/// about half a minute any other it heard. btleplug does not pass BlueZ's
/// `Paired` property on, so a cached peer counts as known by name alone;
/// an unpaired one is only cached shortly after it was heard, when it is
/// likely awake anyway. CoreBluetooth has no list without identifiers to
/// look up, and WinRT's is the connected devices, so on those the backend's
/// own cache — what this process's scan found — is what there is.
async fn known_peripherals(adapter: &Adapter) -> Vec<Peripheral> {
    let mut peripherals = match adapter
        .retrieve_peripherals(RetrievePeripheralsOptions::default())
        .await
    {
        Ok(found) => found,
        Err(e) => {
            debug!("Bluetooth: no known-peripheral list: {e}");
            Vec::new()
        }
    };
    if let Ok(cached) = adapter.peripherals().await {
        for p in cached {
            if !peripherals.iter().any(|seen| seen.id() == p.id()) {
                peripherals.push(p);
            }
        }
    }
    peripherals
}

/// A peripheral for the address `selector` names, made without having heard
/// it, for when neither the known devices nor the scan had it.
///
/// Windows only. There the known list is just the connected devices, so an
/// adapter the scan missed would be out of reach; WinRT builds a device from
/// a bare address, and a connect makes Windows look for it itself. BlueZ
/// names a peripheral by a D-Bus path it only makes for a device it has
/// heard, and CoreBluetooth has no addresses.
#[cfg(windows)]
pub(super) async fn by_address(adapter: &Adapter, selector: Option<&str>) -> Option<Candidate> {
    let address = named_address(selector)?;
    let peripheral = match adapter
        .add_peripheral(&btleplug::platform::PeripheralId::from(address))
        .await
    {
        Ok(peripheral) => peripheral,
        Err(e) => {
            debug!("Bluetooth: cannot reach {address} by address: {e}");
            return None;
        }
    };
    info!("Bluetooth: {address} not heard, trying it by address");
    Some(Candidate {
        id: peripheral.id().to_string(),
        address: address.to_string(),
        name: None,
        taken_by: None,
        standing: Standing::Known,
        peripheral,
    })
}

#[cfg(not(windows))]
pub(super) async fn by_address(_adapter: &Adapter, _selector: Option<&str>) -> Option<Candidate> {
    None
}

/// The Bluetooth address `selector` is, if it is one.
#[cfg(any(windows, test))]
fn named_address(selector: Option<&str>) -> Option<btleplug::api::BDAddr> {
    selector.filter(|s| is_bd_addr(s))?.parse().ok()
}

/// A candidate's standing from what the platform says about it. `heard` is
/// only believed from our own scan (`scanning`): BlueZ drops the RSSI of
/// every device when a discovery ends, so one read while scanning is ours.
fn standing(connected: bool, heard: bool, scanning: bool) -> Standing {
    if connected {
        Standing::Connected
    } else if heard && scanning {
        Standing::Heard
    } else {
        Standing::Known
    }
}

/// The peripherals the caller may use: the one an address names, or every
/// peer the caller takes by name, each with its standing.
async fn usable(
    peripherals: Vec<Peripheral>,
    target: Target<'_>,
    scanning: bool,
) -> Vec<Candidate> {
    let mut usable = Vec::new();
    for peripheral in peripherals {
        let properties = peripheral.properties().await.ok().flatten();
        let address = peripheral.address();
        // CoreBluetooth has no addresses and reports an all-zero one.
        let address = if address == btleplug::api::BDAddr::default() {
            String::new()
        } else {
            address.to_string()
        };
        let id = peripheral.id().to_string();
        let name = properties.as_ref().and_then(|p| {
            p.local_name
                .clone()
                .or_else(|| p.advertisement_name.clone())
        });
        // The name the device advertises, which an alias does not change.
        let advertised_name = properties
            .as_ref()
            .and_then(|p| p.advertisement_name.clone());
        let (wanted, taken_by) = match target {
            Target::At(selector) => (
                matches_selector(selector, &id, &address),
                advertised_name.as_deref().or(name.as_deref()),
            ),
            Target::Named(peers) => {
                let taken_by = takes(peers, name.as_deref(), advertised_name.as_deref());
                (taken_by.is_some(), taken_by)
            }
        };
        let taken_by = taken_by.map(str::to_string);
        // Every device the stack reports, kept or not: when a peer in range
        // is not found, this line says what the platform made of it.
        debug!(
            "Bluetooth: saw {address:?} id {id:?} local name {:?} advertised {:?} \
             RSSI {:?}: {}",
            properties.as_ref().and_then(|p| p.local_name.as_deref()),
            advertised_name,
            properties.as_ref().and_then(|p| p.rssi),
            if wanted { "match" } else { "no match" }
        );
        if !wanted {
            continue;
        }
        // Asked only of a device already picked: it costs the stack a round
        // trip per device.
        let connected = peripheral.is_connected().await.unwrap_or(false);
        let heard = properties.as_ref().is_some_and(|p| p.rssi.is_some());
        usable.push(Candidate {
            standing: standing(connected, heard, scanning),
            peripheral,
            id,
            address,
            // Printed by `list` and `info`: a radio neighbour chooses it.
            name: name.as_deref().map(printable),
            taken_by,
        });
    }
    usable
}

/// Put the candidates in [`Standing`] order and keep as many as the caller
/// asked for, warning when several share the best standing.
fn keep_matches(mut matched: Vec<Candidate>, want: Match) -> Vec<Candidate> {
    // Stable: equals stay in the order the platform listed them.
    matched.sort_by_key(|c| c.standing);
    if want == Match::First {
        let tied = tied_for_best(&matched.iter().map(|c| c.standing).collect::<Vec<_>>());
        if tied > 1 {
            warn!(
                "Multiple Bluetooth adapters or meters found ({tied}). Pass --adapter to pick one."
            );
        }
        matched.truncate(1);
    }
    matched
}

/// How many of the sorted `standings` share the first one's.
fn tied_for_best(standings: &[Standing]) -> usize {
    standings
        .first()
        .map_or(0, |best| standings.iter().filter(|s| *s == best).count())
}

/// Whether `selector` names this peripheral.
fn matches_selector(selector: &str, id: &str, address: &str) -> bool {
    selector.eq_ignore_ascii_case(id)
        || (!address.is_empty() && selector.eq_ignore_ascii_case(address))
}

/// The name `peers` takes a peripheral by, of the host's alias for it and
/// the name it advertises: one carrying [`ADAPTER_NAME_PREFIX`] when adapters
/// are taken, or one of the meters' prefixes ([`name_matches`]). `None` when
/// neither is taken.
///
/// The name alone. UNI-T's iDMM2.0 app picks the meters with the radio built
/// in by name alone (`docs/research/new-device-candidates.md`, Bluetooth
/// section). The
/// `0000ff12` UUID the adapter also advertises is no evidence — it is a
/// vendor-range UUID any device may carry, and not a service on the adapter
/// (§2).
fn takes<'n>(
    peers: &BluetoothPeers,
    name: Option<&'n str>,
    advertised_name: Option<&'n str>,
) -> Option<&'n str> {
    let adapter = peers.adapters.then_some(ADAPTER_NAME_PREFIX);
    [name, advertised_name].into_iter().flatten().find(|n| {
        adapter
            .iter()
            .chain(&peers.meters)
            .any(|prefix| name_matches(prefix, n))
    })
}

/// A peripheral name made safe for a terminal.
///
/// Any device in radio range picks its own name, and the CLI prints it: a
/// control character in it (an escape sequence) would reach the user's
/// terminal. Each one becomes U+FFFD.
pub(super) fn printable(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// Whether `s` is a `XX:XX:XX:XX:XX:XX` Bluetooth address.
pub(super) fn is_bd_addr(s: &str) -> bool {
    let mut parts = 0;
    for part in s.split(':') {
        if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_hexdigit()) {
            return false;
        }
        parts += 1;
    }
    parts == 6
}

/// Whether `s` is a hyphenated UUID, the identity CoreBluetooth hands out.
pub(super) fn is_uuid(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(len, group)| group.len() == *len)
        && groups
            .iter()
            .all(|g| g.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name a neighbour advertises cannot carry an escape sequence to the
    /// terminal `list` prints it on.
    #[test]
    fn peripheral_names_lose_their_control_characters() {
        assert_eq!(printable("UT-D07B"), "UT-D07B");
        assert_eq!(
            printable("UT-D07\u{1b}]0;x\u{7}"),
            "UT-D07\u{fffd}]0;x\u{fffd}"
        );
    }

    /// A selector matches either identity the platform exposes, and nothing
    /// else — an address the host does not have must not match everything.
    #[test]
    fn a_selector_matches_the_id_or_the_address() {
        let id = "hci0/dev_12_34_56_78_9A_BC";
        let address = "12:34:56:78:9A:BC";
        assert!(matches_selector(address, id, address));
        assert!(matches_selector("12:34:56:78:9a:bc", id, address));
        assert!(matches_selector(id, id, address));
        assert!(!matches_selector("12:34:56:78:9A:BD", id, address));
        // A macOS peripheral has no address; an empty one matches nothing.
        assert!(!matches_selector("", id, ""));
    }

    /// Only an address is tried by address: a UUID or a platform id names a
    /// device the stack already has, and btleplug's looser parse (single
    /// digits) must not take what the open path routes to HID.
    #[test]
    fn only_a_named_address_is_tried_by_address() {
        assert_eq!(
            named_address(Some("12:34:56:78:9a:bc")).map(|a| a.into_inner()),
            Some([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC])
        );
        assert!(named_address(None).is_none());
        assert!(named_address(Some("1ca4b9a3-9e6f-4f1e-8b0c-2d1f3a4b5c6d")).is_none());
        assert!(named_address(Some("hci0/dev_12_34_56_78_9A_BC")).is_none());
        assert!(named_address(Some("1:2:3:4:5:6")).is_none());
        assert!(named_address(Some("123456789ABC")).is_none());
    }

    /// The peers an open for a meter behind an adapter takes, one for a
    /// meter with the radio built in, and what `auto` takes.
    fn adapters() -> BluetoothPeers {
        BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        }
    }
    fn meters(meters: &[&'static str]) -> BluetoothPeers {
        BluetoothPeers {
            adapters: false,
            meters: meters.to_vec(),
        }
    }

    /// An adapter is picked by the name it advertises, which a host alias
    /// does not hide.
    #[test]
    fn an_adapter_is_recognised_by_name() {
        let peers = adapters();
        assert!(takes(&peers, Some("UT-D07B"), Some("UT-D07B")).is_some());
        assert!(takes(&peers, Some("UT-D07A"), None).is_some());
        assert!(takes(&peers, Some("UT-D07A-1234"), None).is_some());
        assert!(takes(&peers, Some(" ut-d07b "), None).is_some());
        assert_eq!(
            takes(&peers, Some("Bench meter"), Some("UT-D07B")),
            Some("UT-D07B"),
            "taken by the advertised name"
        );

        assert!(takes(&peers, Some("UT61E+"), None).is_none());
        assert!(takes(&peers, Some(""), Some("")).is_none());
        assert!(takes(&peers, Some("   "), None).is_none());
        assert!(takes(&peers, None, None).is_none());
        assert!(takes(&peers, Some("Headphones"), Some("Headphones")).is_none());
    }

    /// A meter with the radio built in is taken by the prefix the caller
    /// names, in any case and with its suffix.
    #[test]
    fn a_built_in_meter_is_recognised_by_its_own_prefix() {
        let peers = meters(&["UT60BT"]);
        assert!(takes(&peers, Some("UT60BT"), None).is_some());
        // Advertised by one UT60BT that answers Get Name with `UT60BT`.
        assert!(takes(&peers, Some("UT60BTk"), Some("UT60BTk")).is_some());
        assert!(takes(&peers, None, Some(" ut60bt ")).is_some());
        assert!(takes(&peers, Some("UT60"), None).is_none());
    }

    /// A meter renamed on the host is taken by the name it advertises, and
    /// that is the name the registry is asked about, so it finds the meter.
    #[test]
    fn a_renamed_meter_is_taken_by_its_advertised_name() {
        let taken = takes(&meters(&["UT60BT"]), Some("Bench meter"), Some("UT60BTk"));
        assert_eq!(taken, Some("UT60BTk"));
        assert!(!crate::protocol::registry::advertising("UT60BTk").is_empty());
    }

    /// Only the peers the caller names are taken: a meter selected behind an
    /// adapter never lands on a meter with the radio built in, nor one of
    /// those on an adapter or on the other one.
    #[test]
    fn only_the_named_peers_are_taken() {
        assert!(takes(&adapters(), Some("UT60BT"), None).is_none());
        assert!(takes(&adapters(), Some("UT202BT"), None).is_none());
        let ut60bt = meters(&["UT60BT"]);
        assert!(takes(&ut60bt, Some("UT-D07B"), None).is_none());
        assert!(takes(&ut60bt, Some("UT202BT"), None).is_none());

        let any = BluetoothPeers {
            adapters: true,
            meters: vec!["UT60BT", "UT202BT"],
        };
        for name in ["UT-D07B", "UT60BTk", "\tUT202BT \n"] {
            assert!(takes(&any, Some(name), None).is_some(), "{name:?}");
        }
        assert!(takes(&any, Some("UT61E+"), None).is_none());
    }

    /// A connection counts wherever it is seen; a signal strength only when
    /// our own scan is running, since one read before it is another
    /// program's discovery. A paired adapter with neither is still known.
    #[test]
    fn standing_comes_from_connection_then_our_scan() {
        assert_eq!(standing(true, false, false), Standing::Connected);
        assert_eq!(standing(true, true, true), Standing::Connected);
        assert_eq!(standing(false, true, true), Standing::Heard);

        assert_eq!(standing(false, false, true), Standing::Known, "not heard");
        assert_eq!(standing(false, false, false), Standing::Known, "asleep");
        assert_eq!(
            standing(false, true, false),
            Standing::Known,
            "a signal strength from before our scan"
        );
    }

    /// Opening takes a connected adapter over a heard one over a known one
    /// that was not heard, and only warns when the best is shared.
    #[test]
    fn candidates_are_tried_connected_then_heard_then_known() {
        use Standing::*;
        let mut standings = vec![Known, Heard, Connected, Heard];
        standings.sort();
        assert_eq!(standings, [Connected, Heard, Heard, Known]);
        assert_eq!(tied_for_best(&standings), 1);

        // Nothing connected: the heard ones tie, the known one waits.
        assert_eq!(tied_for_best(&[Heard, Heard, Known]), 2);
        // Nothing heard either: the fallback is a known adapter.
        assert_eq!(tied_for_best(&[Known]), 1);
        assert_eq!(tied_for_best(&[]), 0);
    }
}
