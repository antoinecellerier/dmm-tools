//! Bluetooth LE transport for the UT-D07B adapter.
//!
//! The UT-D07B is a transparent BLE-to-UART bridge: the bytes it carries are
//! the ones the USB cable carries, so no parser or framing code knows about
//! it. Everything Bluetooth-specific is here
//! (`docs/research/ut-d07b/reverse-engineered-protocol.md`).
//!
//! There is no background thread and no channel: the struct owns a
//! current-thread tokio runtime and every btleplug call runs inside
//! `block_on` on the caller's thread, so nothing runs between two transport
//! calls. That is the same deal the HID transports have: a meter that streams
//! after its connect frame (the UT171 and UT181A do) keeps sending, and its
//! notifications queue in the platform's socket the way HID reports queue in
//! hidraw, to be taken off at the next read. The framing layer resyncs on the
//! next header, so no pump task is needed.

use crate::DeviceInfo;
use crate::error::{Error, Result};
use crate::transport::Transport;
use btleplug::api::{
    Central, CentralState, Characteristic, Manager as _, Peripheral as _,
    RetrievePeripheralsOptions, ScanFilter, ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::{Stream, StreamExt};
use log::{debug, info, trace, warn};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;

/// ISSC transparent-UART service the UT-D07B carries
/// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2).
const UART_SERVICE: &str = "49535343-fe7d-4ae5-8fa9-9fafd205e455";
/// UART TX, meter → host: notifications carry the meter's frames (§2).
const UART_TX_CHARACTERISTIC: &str = "49535343-1e4d-4bd9-ba61-23c647249616";
/// UART RX, host → meter: commands are written here (§2).
const UART_RX_CHARACTERISTIC: &str = "49535343-8841-43f4-a8d4-ecbe34729bb3";
/// Local name prefix the UNI-T adapters advertise (§2).
const NAME_PREFIX: &str = "UT-D07";

/// How long to scan before giving up on finding an adapter.
///
/// An adapter this host is already connected to is found without a scan, so
/// this bounds every other open — and the GUI's reconnect loop calls the
/// opener synchronously, so a quit during a failing reconnect waits for it.
const SCAN_WINDOW: Duration = Duration::from_secs(3);
/// How often the scan results are re-read while scanning.
const SCAN_POLL: Duration = Duration::from_millis(250);
/// How long a connection attempt may take before it is abandoned.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long GATT service discovery and the subscribe may take, the one retry
/// included.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
/// The pause before retrying a link setup that failed.
const SETUP_RETRY_PAUSE: Duration = Duration::from_millis(500);
/// How often to look again while the service tree is still filling in.
const DISCOVERY_POLL: Duration = Duration::from_millis(500);
/// How long taking a link down may take, after a failed open or on drop.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Write size when the platform has not negotiated an MTU yet: the BLE
/// default ATT MTU of 23 bytes minus the three-byte write header.
const DEFAULT_WRITE_CHUNK: usize = 20;

/// The frame the adapter itself puts on the stream: once when a link comes
/// up and about once a second while the meter is silent (research doc §3).
/// It is the adapter's, not the meter's, so it never reaches a parser.
const ADAPTER_HEARTBEAT: [u8; 9] = [0xAB, 0xCD, 0x06, 0xAA, 0xAA, 0x6E, 0x67, 0x03, 0xA7];

/// A UT-D07B adapter, open and subscribed.
pub(crate) struct Ble {
    /// Kept alive: dropping the manager tears the platform session down under
    /// the peripheral.
    _manager: Manager,
    _adapter: Adapter,
    peripheral: Peripheral,
    /// Host → meter. The notify characteristic is not kept: it is only needed
    /// to subscribe, which the opener has already done.
    rx_char: Characteristic,
    notifications: RefCell<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>>,
    /// Bytes a notification delivered that did not fit the caller's buffer.
    pending: RefCell<VecDeque<u8>>,
    /// Heartbeats stripped so far, for `transport_status`: a rising count
    /// with no readings says the adapter is up and the meter is not.
    heartbeats: Cell<u32>,
    /// What `dmm-cli list` printed for this adapter, and what `--adapter`
    /// takes to pin it.
    selector: String,
    /// Drives every btleplug call. Last, so it outlives every field whose
    /// destructor reaches into the stack.
    rt: tokio::runtime::Runtime,
}

impl Ble {
    /// Whether the link is still up, as cheaply as the platform allows.
    ///
    /// An error here means the platform cannot say, which on every backend
    /// means the peripheral is gone.
    fn is_connected(&self) -> bool {
        self.rt
            .block_on(self.peripheral.is_connected())
            .unwrap_or(false)
    }
}

/// Open the first UT-D07B in range.
pub(crate) fn open_first() -> Result<Box<dyn Transport>> {
    open(None)
}

/// Open the adapter `selector` names — an address, or the platform id
/// [`list`] printed.
pub(crate) fn open_selected(selector: &str) -> Result<Box<dyn Transport>> {
    open(Some(selector))
}

/// Whether `selector` names a Bluetooth adapter rather than a HID device.
///
/// A Bluetooth address (`12:34:56:78:9A:BC`) or a CoreBluetooth peripheral
/// UUID; no HID serial number or device path on any platform takes either
/// shape, so the open path can tell them apart without being told.
pub(crate) fn is_bluetooth_selector(selector: &str) -> bool {
    is_bd_addr(selector) || is_uuid(selector)
}

/// The UT-D07B adapters in range, for `dmm-cli list`.
///
/// Scans, so this takes seconds — never call it from a render path.
pub(crate) fn list() -> Result<Vec<DeviceInfo>> {
    let rt = runtime()?;
    rt.block_on(async {
        let (_manager, adapter) = central().await?;
        let found = search(&adapter, None, Match::All).await?;
        Ok(found
            .into_iter()
            .map(|c| DeviceInfo {
                path: c.selector(),
                not_heard: c.standing == Standing::Known,
                product: c.name,
                serial: None,
                transport: crate::BLUETOOTH,
            })
            .collect())
    })
}

/// A peripheral the adapter knows about, with what the platform says about it.
struct Candidate {
    peripheral: Peripheral,
    /// Platform identifier: a D-Bus path on Linux, a UUID on macOS.
    id: String,
    /// Bluetooth address, empty where the platform exposes none (macOS).
    address: String,
    /// The name to show: the host's alias for the device where it has one.
    name: Option<String>,
    /// How sure we are that it can answer, which orders the candidates.
    standing: Standing,
}

impl Candidate {
    /// What `list` prints and `--adapter` takes: the address where there is
    /// one, otherwise the platform's own identifier.
    fn selector(&self) -> String {
        if self.address.is_empty() {
            self.id.clone()
        } else {
            self.address.clone()
        }
    }

    /// The name to show, falling back to the selector for an adapter that
    /// advertised none.
    fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.selector())
    }
}

/// Why a candidate is worth trying, best first: the derived order is the
/// order candidates are tried and listed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Standing {
    /// Connected to this host: usable at once, no scan.
    Connected,
    /// Heard advertising during our scan, so awake.
    Heard,
    /// Only in the platform's list of known devices — paired, or on BlueZ
    /// any device it still caches. It may be asleep (research doc §4), or
    /// awake and missed by the scan (§4 again): only a connect tells.
    Known,
}

/// How many matches a search wants back.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Match {
    /// The best match, for opening (see [`search`]). Only that one is tried.
    First,
    /// Everything that matched, for listing.
    All,
}

/// Everything one open produced, so the whole sequence fits in one
/// `block_on`.
struct Opened {
    manager: Manager,
    adapter: Adapter,
    peripheral: Peripheral,
    rx_char: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    selector: String,
}

/// Open an adapter and subscribe to its UART notifications.
fn open(selector: Option<&str>) -> Result<Box<dyn Transport>> {
    let rt = runtime()?;
    let opened = rt.block_on(connect(selector))?;
    Ok(Box::new(Ble {
        rt,
        _manager: opened.manager,
        _adapter: opened.adapter,
        peripheral: opened.peripheral,
        rx_char: opened.rx_char,
        notifications: RefCell::new(opened.notifications),
        pending: RefCell::new(VecDeque::new()),
        heartbeats: Cell::new(0),
        selector: opened.selector,
    }))
}

/// Build the runtime every btleplug call runs inside.
///
/// Current-thread: there is no work to do while the caller is not in a
/// transport call. `enable_all` rather than `enable_time` alone because the
/// platform backends bring their own I/O (BlueZ talks D-Bus over a socket).
fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Bluetooth(format!("could not start the Bluetooth runtime: {e}")))
}

/// The first Bluetooth adapter on the host, once it is usable.
async fn central() -> Result<(Manager, Adapter)> {
    let manager = Manager::new().await.map_err(stack_error)?;
    let adapter = manager
        .adapters()
        .await
        .map_err(stack_error)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Bluetooth("not available on this computer".to_string()))?;
    match adapter.adapter_state().await {
        Ok(CentralState::PoweredOn) => {}
        Ok(CentralState::PoweredOff) => {
            return Err(Error::Bluetooth("turned off on this computer".to_string()));
        }
        // Not every platform reports a state; an unusable adapter fails at
        // the scan instead, with the platform's own message.
        Ok(CentralState::Unknown) => debug!("Bluetooth: adapter state unknown"),
        Err(e) => debug!("Bluetooth: adapter state unavailable: {e}"),
    }
    Ok((manager, adapter))
}

/// Find an adapter, connect to it and subscribe to the UART characteristic.
async fn connect(selector: Option<&str>) -> Result<Opened> {
    let (manager, adapter) = central().await?;
    let mut found = search(&adapter, selector, Match::First).await?;
    if found.is_empty() {
        return Err(match selector {
            Some(selector) => Error::AdapterNotFound(selector.to_string()),
            None => Error::NoTransportFound,
        });
    }
    let candidate = found.remove(0);
    let peripheral = candidate.peripheral.clone();
    // Nothing named and nothing heard: the scan may simply have missed an
    // awake adapter (research doc §4), so the known one is tried by address.
    let fallback = selector.is_none() && candidate.standing == Standing::Known;
    if fallback {
        info!(
            "Bluetooth: no adapter heard, trying the known {} ({})",
            candidate.label(),
            candidate.selector()
        );
    }

    // A paired adapter is often already linked, and BlueZ refuses a second
    // connect on one.
    if !peripheral.is_connected().await.unwrap_or(false)
        && let Err(e) = peripheral.connect_with_timeout(CONNECT_TIMEOUT).await
    {
        // The Linux backend waits for BlueZ to resolve the services inside
        // its connect, and gives up before BlueZ does on this adapter
        // (research doc §3). With the link up, discovery below waits it out.
        if peripheral.is_connected().await.unwrap_or(false) {
            debug!("Bluetooth: connected, services still resolving ({e})");
        } else {
            // BlueZ keeps a timed-out connect pending; cancel it, or an
            // adapter that wakes later is linked with nobody reading it.
            disconnect(&peripheral).await;
            if fallback {
                // A known adapter that does not answer is the asleep case,
                // which is "nothing found", not a Bluetooth fault.
                info!("Bluetooth: {} did not answer: {e}", candidate.label());
                return Err(Error::NoTransportFound);
            }
            return Err(link_error(e));
        }
    }

    // An adapter just switched on can refuse the first setup on a link that
    // is up (an ATT error), and answer the next one: try once more on the
    // same link before giving it up. Both tries share one discovery bound.
    let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
    let uart = match subscribe_uart(&peripheral, deadline).await {
        Err(e) => {
            debug!("Bluetooth: link setup failed, trying once more: {e}");
            tokio::time::sleep_until(deadline.min(tokio::time::Instant::now() + SETUP_RETRY_PAUSE))
                .await;
            subscribe_uart(&peripheral, deadline).await
        }
        uart => uart,
    };
    // From here on the link is up, so a failure has to take it down again:
    // left connected, the adapter stays awake with nobody reading it.
    let (rx_char, notifications) = match uart {
        Ok(uart) => uart,
        Err(e) => {
            disconnect(&peripheral).await;
            return Err(e);
        }
    };

    let selector = candidate.selector();
    info!(
        "connected to {} over Bluetooth ({selector})",
        candidate.label()
    );
    Ok(Opened {
        manager,
        adapter,
        peripheral,
        rx_char,
        notifications,
        selector,
    })
}

/// Find the UART characteristics on a connected adapter and subscribe to its
/// notifications, handing back the write characteristic and the stream.
async fn subscribe_uart(
    peripheral: &Peripheral,
    deadline: tokio::time::Instant,
) -> Result<(
    Characteristic,
    Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
)> {
    // The service tree fills in as the platform resolves it, so look again
    // until both UART characteristics are there or the deadline passes.
    let (rx_char, tx_char) = loop {
        peripheral.discover_services().await.map_err(link_error)?;
        let characteristics = peripheral.characteristics();
        let uart = |uuid: &str| {
            characteristics
                .iter()
                .find(|c| c.service_uuid.to_string() == UART_SERVICE && c.uuid.to_string() == uuid)
                .cloned()
        };
        match (uart(UART_RX_CHARACTERISTIC), uart(UART_TX_CHARACTERISTIC)) {
            (Some(rx), Some(tx)) => break (rx, tx),
            (rx, _) if tokio::time::Instant::now() >= deadline => {
                return Err(missing_characteristic(if rx.is_none() {
                    "write"
                } else {
                    "notify"
                }));
            }
            _ => tokio::time::sleep(DISCOVERY_POLL).await,
        }
    };

    // Bring-up is subscribe and go: the meter answers a command written to
    // the RX characteristic with a notification, with nothing written to the
    // control characteristic first (research doc §3).
    peripheral.subscribe(&tx_char).await.map_err(link_error)?;
    let notifications = peripheral.notifications().await.map_err(link_error)?;
    Ok((rx_char, notifications))
}

/// Take the link down, bounded: a stack that does not answer must not hold
/// up the error the caller is waiting for, or a quit.
async fn disconnect(peripheral: &Peripheral) {
    match tokio::time::timeout(DISCONNECT_TIMEOUT, peripheral.disconnect()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => debug!("Bluetooth: disconnect failed: {e}"),
        Err(_) => debug!("Bluetooth: disconnect timed out"),
    }
}

/// Look for adapters: the ones this host already knows, then a scan.
///
/// With no address named, a UNI-T adapter is taken in [`Standing`] order: one
/// connected to this host, found without a scan; else the first heard
/// advertising during the scan; else a known one that was not heard, tried by
/// address. That last step is there because the scan misses awake adapters:
/// btleplug's BlueZ backend scans with BR/EDR and LE interleaved and offers
/// no LE-only mode, and in that mode BlueZ rarely hears our adapter, while a
/// connect by address listens on LE alone and reaches it (research doc §4).
/// An adapter that really is asleep costs one bounded connect.
///
/// An address the user named is taken from the known devices or the scan,
/// whatever its standing, since they asked for that one.
async fn search(adapter: &Adapter, selector: Option<&str>, want: Match) -> Result<Vec<Candidate>> {
    // A signal strength in this list is another program's discovery, not
    // ours, so only a connection lifts a known device above `Known`.
    let known = usable(known_peripherals(adapter).await, selector, false).await;
    // Opening stops here when the known list already settles it: a connected
    // adapter, or the named one. Listing scans regardless, so an adapter
    // advertising in range shows up beside the known ones.
    let settled = |found: &[Candidate]| match selector {
        Some(_) => !found.is_empty(),
        None => found.iter().any(|c| c.standing < Standing::Known),
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
        let scanned = usable(seen, selector, true).await;
        // `All` keeps scanning to the end of the window: a second adapter
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
/// `Paired` property on, so a cached UT-D07 counts as known by name alone;
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

/// The peripherals the caller may use: the one `selector` names, or — with
/// none — every UNI-T adapter, each with its standing.
async fn usable(
    peripherals: Vec<Peripheral>,
    selector: Option<&str>,
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
        let wanted = match selector {
            Some(selector) => matches_selector(selector, &id, &address),
            None => is_ut_d07(name.as_deref(), advertised_name.as_deref()),
        };
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
                "Multiple Bluetooth adapters found ({tied} devices). Pass --adapter to pick one."
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

/// Whether a peripheral is a UNI-T Bluetooth adapter, by the name it
/// advertises or the host's alias for it.
///
/// The name alone: the adapter advertises `UT-D07B` (research doc §2) and
/// the UT-D07A a name starting `UT-D07A` (§7). The `0000ff12` UUID it also
/// advertises is no evidence — it is a vendor-range UUID any device may
/// carry, and not a service on the adapter (§2).
fn is_ut_d07(name: Option<&str>, advertised_name: Option<&str>) -> bool {
    [name, advertised_name]
        .into_iter()
        .flatten()
        .any(|n| n.trim().to_ascii_uppercase().starts_with(NAME_PREFIX))
}

/// A peripheral name made safe for a terminal.
///
/// Any device in radio range picks its own name, and the CLI prints it: a
/// control character in it (an escape sequence) would reach the user's
/// terminal. Each one becomes U+FFFD.
fn printable(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// Whether `s` is a `XX:XX:XX:XX:XX:XX` Bluetooth address.
fn is_bd_addr(s: &str) -> bool {
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
fn is_uuid(s: &str) -> bool {
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

/// How many bytes fit in one write at `mtu`: the ATT MTU less the three-byte
/// write header, and the BLE default when the platform has not negotiated one.
fn write_chunk_size(mtu: u16) -> usize {
    match (mtu as usize).checked_sub(3) {
        Some(0) | None => DEFAULT_WRITE_CHUNK,
        Some(n) => n,
    }
}

/// Remove every complete [`ADAPTER_HEARTBEAT`] from `pending`, returning how
/// many there were.
///
/// Only whole frames: one split across two notifications is left for the
/// parser to report, which the adapter's notifications — ATT-sized, on an
/// MTU well past nine bytes — make rare enough not to buffer for.
fn strip_heartbeats(pending: &mut VecDeque<u8>) -> u32 {
    let mut stripped = 0;
    let mut at = 0;
    while at + ADAPTER_HEARTBEAT.len() <= pending.len() {
        if pending
            .range(at..at + ADAPTER_HEARTBEAT.len())
            .eq(ADAPTER_HEARTBEAT.iter())
        {
            pending.drain(at..at + ADAPTER_HEARTBEAT.len());
            stripped += 1;
        } else {
            at += 1;
        }
    }
    stripped
}

/// Move as much of `pending` into `buf` as fits, leaving the rest for the
/// next read.
fn drain_pending(pending: &mut VecDeque<u8>, buf: &mut [u8]) -> usize {
    let n = pending.len().min(buf.len());
    for (slot, byte) in buf.iter_mut().zip(pending.drain(..n)) {
        *slot = byte;
    }
    n
}

/// A failure of the Bluetooth stack itself: nothing the caller retries will
/// fix it, so it carries the reason rather than looking like a silent meter.
fn stack_error(e: btleplug::Error) -> Error {
    Error::Bluetooth(e.to_string())
}

/// A failure of this link. `NotConnected` and a vanished device are the
/// out-of-range case, which the GUI reconnects from.
fn link_error(e: btleplug::Error) -> Error {
    match e {
        btleplug::Error::NotConnected | btleplug::Error::DeviceNotFound => Error::LinkLost,
        other => Error::Bluetooth(other.to_string()),
    }
}

/// The adapter answered but carries no transparent UART.
fn missing_characteristic(role: &str) -> Error {
    Error::Bluetooth(format!(
        "the Bluetooth device has no UART {role} characteristic — \
         it is not a supported adapter, or its services never resolved"
    ))
}

impl Transport for Ble {
    fn write(&self, data: &[u8]) -> Result<()> {
        let chunk = write_chunk_size(self.peripheral.mtu());
        trace!("Bluetooth TX ({} bytes): {data:02X?}", data.len());
        // Unacknowledged: an acknowledged write costs a round trip per poll
        // (0.8 s against 0.63 s per reading on our adapter). A dead link is
        // caught by `read_timeout` instead.
        self.rt.block_on(async {
            for part in data.chunks(chunk) {
                self.peripheral
                    .write(&self.rx_char, part, WriteType::WithoutResponse)
                    .await
                    .map_err(link_error)?;
            }
            Ok(())
        })
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        {
            let mut pending = self.pending.borrow_mut();
            if !pending.is_empty() {
                let n = drain_pending(&mut pending, buf);
                trace!("Bluetooth RX ({n} bytes, buffered): {:02X?}", &buf[..n]);
                return Ok(n);
            }
        }

        let timeout = Duration::from_millis(timeout_ms.max(0) as u64);
        // The borrow is taken outside the future: nothing else touches the
        // stream, and holding a `RefCell` guard across an await point is a
        // deadlock waiting to happen on any runtime but this one.
        let received = {
            let mut stream = self.notifications.borrow_mut();
            // The timer is created inside the runtime: built outside
            // `block_on` it has no reactor and panics.
            self.rt
                .block_on(async { tokio::time::timeout(timeout, stream.next()).await })
        };

        match received {
            Ok(Some(note)) => {
                // Only the UART characteristic is subscribed, so anything
                // else is the platform replaying a subscription we did not
                // make; its bytes are not UART payload.
                if note.uuid.to_string() != UART_TX_CHARACTERISTIC {
                    debug!("Bluetooth: ignoring a notification from {}", note.uuid);
                    return Ok(0);
                }
                let mut pending = self.pending.borrow_mut();
                pending.extend(note.value.iter().copied());
                let stripped = strip_heartbeats(&mut pending);
                if stripped > 0 {
                    self.heartbeats.set(self.heartbeats.get() + stripped);
                    debug!("Bluetooth: adapter heartbeat ({stripped})");
                }
                let n = drain_pending(&mut pending, buf);
                trace!("Bluetooth RX ({n} bytes): {:02X?}", &buf[..n]);
                Ok(n)
            }
            // The notification stream ends when the platform tears the
            // subscription down, which only happens with the link.
            Ok(None) => Err(Error::LinkLost),
            Err(_elapsed) => {
                // Silence is normal — the meter answers when it answers. A
                // dropped link looks exactly the same from here, so this one
                // check is the whole disconnect detection.
                if self.is_connected() {
                    Ok(0)
                } else {
                    Err(Error::LinkLost)
                }
            }
        }
    }

    fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
        // HID-only bring-up. The families on this bridge send none, and a
        // transparent UART has nowhere to put one.
        debug!("Bluetooth: ignoring a HID feature report");
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        // The adapter's own name and identity. It cannot say which meter is
        // behind it — its Device Information strings are empty (research doc
        // §1) — so this is all there is to report.
        let name = self
            .rt
            .block_on(self.peripheral.properties())
            .ok()
            .flatten()
            .and_then(|p| p.local_name)
            .map(|n| printable(&n))
            .unwrap_or_else(|| "unnamed device".to_string());
        Ok(format!("{name} ({})", self.selector))
    }

    fn transport_status(&self) -> Result<String> {
        let mut status = format!(
            "MTU: {} bytes, adapter heartbeats: {}",
            self.peripheral.mtu(),
            self.heartbeats.get()
        );
        self.rt.block_on(async {
            // Both are best-effort: no backend exposes all of it, and a
            // reporter's `info` output is more useful with what it does.
            match self.peripheral.connection_parameters().await {
                Ok(Some(p)) => status.push_str(&format!(
                    ", interval: {:.1} ms, latency: {}, supervision timeout: {:.1} s",
                    p.interval_us as f64 / 1000.0,
                    p.latency,
                    p.supervision_timeout_us as f64 / 1_000_000.0
                )),
                Ok(None) => status.push_str(", interval: not reported"),
                Err(e) => debug!("Bluetooth: connection parameters unavailable: {e}"),
            }
            match self.peripheral.read_rssi().await {
                Ok(rssi) => status.push_str(&format!(", RSSI: {rssi} dBm")),
                Err(e) => debug!("Bluetooth: RSSI unavailable: {e}"),
            }
        });
        Ok(status)
    }

    fn transport_name(&self) -> &'static str {
        crate::BLUETOOTH
    }

    fn bluetooth_selector(&self) -> Option<&str> {
        Some(&self.selector)
    }
}

impl Drop for Ble {
    /// Leaving the link up would keep the adapter awake and drain its
    /// batteries long after the session ended.
    fn drop(&mut self) {
        debug!("Bluetooth: disconnecting {}", self.selector);
        self.rt.block_on(async {
            disconnect(&self.peripheral).await;
            // The stream's own destructor talks to the stack, so it runs
            // here, inside the runtime, not with the fields afterwards.
            *self.notifications.borrow_mut() = Box::pin(futures::stream::empty());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The UUIDs are the ones read off our own adapter; a typo here would
    /// make every open fail to find the UART.
    #[test]
    fn uart_uuids_match_the_research_doc() {
        assert_eq!(UART_SERVICE, "49535343-fe7d-4ae5-8fa9-9fafd205e455");
        assert_eq!(
            UART_TX_CHARACTERISTIC,
            "49535343-1e4d-4bd9-ba61-23c647249616"
        );
        assert_eq!(
            UART_RX_CHARACTERISTIC,
            "49535343-8841-43f4-a8d4-ecbe34729bb3"
        );
        // btleplug renders UUIDs lowercase, and the comparison is textual.
        for uuid in [UART_SERVICE, UART_TX_CHARACTERISTIC, UART_RX_CHARACTERISTIC] {
            assert_eq!(uuid, uuid.to_ascii_lowercase());
        }
    }

    /// The open path tells a Bluetooth selector from a HID one by shape
    /// alone: a HID serial or path must never be routed to the BLE opener.
    #[test]
    fn bluetooth_selectors_are_addresses_and_uuids() {
        assert!(is_bluetooth_selector("12:34:56:78:9A:BC"));
        assert!(is_bluetooth_selector("12:34:56:78:9a:bc"));
        assert!(is_bluetooth_selector(
            "1ca4b9a3-9e6f-4f1e-8b0c-2d1f3a4b5c6d"
        ));

        assert!(!is_bluetooth_selector("/dev/hidraw0"));
        assert!(!is_bluetooth_selector("0001:0005:00"));
        assert!(!is_bluetooth_selector("\\\\?\\HID#VID_10C4"));
        assert!(!is_bluetooth_selector("IOService:/AppleACPIPlat"));
        assert!(!is_bluetooth_selector("0123456789AB"));
        assert!(!is_bluetooth_selector(""));
        assert!(!is_bluetooth_selector("12:34:56:78:9A"));
        assert!(!is_bluetooth_selector("12:34:56:78:9A:BC:22"));
        assert!(!is_bluetooth_selector("ZZ:34:56:78:9A:BC"));
        assert!(!is_bluetooth_selector("1ca4b9a3-9e6f-4f1e-8b0c"));
    }

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

    /// Auto-detection picks the adapter by the name it advertises, which a
    /// host alias does not hide.
    #[test]
    fn ut_d07_is_recognised_by_name() {
        assert!(is_ut_d07(Some("UT-D07B"), Some("UT-D07B")));
        assert!(is_ut_d07(Some("UT-D07A"), None));
        assert!(is_ut_d07(Some(" ut-d07b "), None));
        assert!(is_ut_d07(Some("Bench meter"), Some("UT-D07B")));

        assert!(!is_ut_d07(Some("UT61E+"), None));
        assert!(!is_ut_d07(None, None));
        assert!(!is_ut_d07(Some("Headphones"), Some("Headphones")));
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

    /// An over-MTU write is rejected by the peer, so the chunk size has to
    /// follow whatever the platform negotiated.
    #[test]
    fn writes_are_chunked_to_the_negotiated_mtu() {
        assert_eq!(write_chunk_size(23), 20);
        assert_eq!(write_chunk_size(247), 244);
        // Not negotiated yet, or a value too small to carry a write header.
        assert_eq!(write_chunk_size(0), DEFAULT_WRITE_CHUNK);
        assert_eq!(write_chunk_size(3), DEFAULT_WRITE_CHUNK);

        let frame = [0xABu8; 45];
        let chunks: Vec<usize> = frame
            .chunks(write_chunk_size(23))
            .map(<[u8]>::len)
            .collect();
        assert_eq!(chunks, vec![20, 20, 5]);
    }

    /// The adapter's own frame is dropped wherever it lands in a
    /// notification, and a real frame around it is left whole.
    #[test]
    fn adapter_heartbeats_are_stripped_from_the_stream() {
        let reading = [0xAB, 0xCD, 0x03, 0xFF, 0x00, 0x02, 0x7B];
        let mut pending: VecDeque<u8> = VecDeque::new();
        pending.extend(ADAPTER_HEARTBEAT);
        pending.extend(reading);
        pending.extend(ADAPTER_HEARTBEAT);
        pending.extend(ADAPTER_HEARTBEAT);
        assert_eq!(strip_heartbeats(&mut pending), 3);
        assert_eq!(pending.iter().copied().collect::<Vec<u8>>(), reading);

        // A split heartbeat is not a heartbeat yet.
        let mut partial: VecDeque<u8> = ADAPTER_HEARTBEAT[..5].iter().copied().collect();
        assert_eq!(strip_heartbeats(&mut partial), 0);
        assert_eq!(partial.len(), 5);

        // The checksum is part of the match: a frame that only starts like
        // one is the meter's.
        let mut lookalike: VecDeque<u8> = ADAPTER_HEARTBEAT.iter().copied().collect();
        *lookalike.back_mut().unwrap() = 0xA8;
        assert_eq!(strip_heartbeats(&mut lookalike), 0);
    }

    /// A notification can carry more than the caller's buffer holds; the rest
    /// has to come back on the next read, in order and once.
    #[test]
    fn pending_bytes_carry_over_between_reads() {
        let mut pending: VecDeque<u8> = (1u8..=10).collect();
        let mut buf = [0u8; 4];

        assert_eq!(drain_pending(&mut pending, &mut buf), 4);
        assert_eq!(buf, [1, 2, 3, 4]);
        assert_eq!(drain_pending(&mut pending, &mut buf), 4);
        assert_eq!(buf, [5, 6, 7, 8]);
        assert_eq!(drain_pending(&mut pending, &mut buf), 2);
        assert_eq!(buf[..2], [9, 10]);
        assert!(pending.is_empty());
        assert_eq!(drain_pending(&mut pending, &mut buf), 0);
    }

    /// The buffer the framing layer passes is bigger than one notification,
    /// so the common case is a single drained read that leaves nothing.
    #[test]
    fn a_short_notification_drains_in_one_read() {
        let mut pending: VecDeque<u8> = [0xAB, 0xCD, 0x03].into_iter().collect();
        let mut buf = [0u8; 64];
        assert_eq!(drain_pending(&mut pending, &mut buf), 3);
        assert_eq!(&buf[..3], &[0xAB, 0xCD, 0x03]);
        assert!(pending.is_empty());
    }

    /// The opener's bounds have to keep the GUI's synchronous reconnect loop
    /// responsive: worst case is one scan, one connect, one discovery and
    /// the disconnect after it fails. The setup retry sits inside the
    /// discovery bound.
    #[test]
    fn open_is_bounded() {
        assert!(SCAN_POLL < SCAN_WINDOW);
        assert!(SETUP_RETRY_PAUSE < DISCOVERY_TIMEOUT);
        let worst_case = SCAN_WINDOW + CONNECT_TIMEOUT + DISCOVERY_TIMEOUT + DISCONNECT_TIMEOUT;
        assert!(worst_case <= Duration::from_secs(30), "{worst_case:?}");
    }
}
