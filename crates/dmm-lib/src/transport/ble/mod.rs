//! Bluetooth LE transport for UNI-T's UT-D07 adapters and the meters with
//! the radio built in (the UT60BT, UT202BT, ZOTEK's meters, the EEVblog
//! 121GW and the Brymen BM78xBT), called peers here.
//!
//! The UT-D07B is a transparent BLE-to-UART bridge: the bytes it carries are
//! the ones the USB cable carries. The UT60BT and UT202BT send the same UT61+
//! frames over the same ISSC service (`docs/research/new-device-candidates.md`,
//! Bluetooth section). So no parser or framing code knows about Bluetooth.
//! Everything Bluetooth-specific is here
//! (`docs/research/ut-d07b/reverse-engineered-protocol.md`).
//!
//! A peer carries its byte stream over one of four GATT profiles, picked
//! from the services it offers once connected (`profile.rs`): ISSC's
//! transparent UART (`issc.rs`), the EEVblog 121GW's own
//! (`eevblog121gw.rs`), Brymen's own (`brymen.rs`), or the FFF0/FFF4 one
//! (`fff0.rs`). Brymen's alone has a login, which runs between discovery
//! and the subscribe.
//!
//! There is no background thread and no channel: the struct owns a
//! current-thread tokio runtime and every btleplug call runs inside
//! `block_on` on the caller's thread, so nothing runs between two transport
//! calls. That is the same deal the HID transports have: a meter that streams
//! after its connect frame (the UT171 and UT181A do) keeps sending, and its
//! notifications queue in the platform's socket the way HID reports queue in
//! hidraw, to be taken off at the next read. The framing layer resyncs on the
//! next header, so no pump task is needed. The stream reads a streaming meter
//! continuously, so its notifications don't wait long there, and `Dmm` drops
//! what queued while nobody read (a pause, a reconnect).

mod brymen;
mod eevblog121gw;
mod fff0;
mod issc;
mod profile;
mod search;
#[cfg(target_os = "windows")]
mod winrt;

use crate::DeviceInfo;
use crate::error::{Error, Result};
use crate::transport::{BluetoothPeers, LateReadings, Link, Transport};
use btleplug::api::{
    Central, CentralState, Characteristic, Manager as _, Peripheral as _, ValueNotification,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::{Stream, StreamExt};
use issc::strip_heartbeats;
use log::{debug, info, trace, warn};
use profile::{BringUp, GattProfile, PROFILES, choose_profile, write_chunk_size, write_type};
use search::{Match, Standing, Target, by_address, is_bd_addr, is_uuid, printable, search};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;
#[cfg(target_os = "windows")]
use winrt::ShortInterval;

/// What a short-interval request leaves to keep alive: nothing where the
/// platform takes no request ([`request_short_interval`]).
#[cfg(not(target_os = "windows"))]
type ShortInterval = std::convert::Infallible;

/// How long to scan before giving up on finding a peer.
///
/// A peer this host is already connected to is found without a scan, so
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
/// How long a profile's login may take: at least this long from its start,
/// and never later than this long after the discovery bound, so both tries
/// of the setup share one login bound.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(8);
/// The pause before retrying a link setup that failed.
const SETUP_RETRY_PAUSE: Duration = Duration::from_millis(500);
/// How often to look again while the service tree is still filling in.
const DISCOVERY_POLL: Duration = Duration::from_millis(500);
/// How long asking for a short connection interval may take
/// ([`request_short_interval`]); past it the link keeps what the peer set.
#[cfg(target_os = "windows")]
const INTERVAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// How long `hcitool con` may take to list the links ([`connection_handle`]).
#[cfg(target_os = "linux")]
const HANDLE_LOOKUP_TIMEOUT: Duration = Duration::from_secs(1);
/// How long taking a link down may take, after a failed open or on drop.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// A peer, open and subscribed.
pub(crate) struct Ble {
    /// Kept alive: dropping the manager tears the platform session down under
    /// the peripheral.
    _manager: Manager,
    _adapter: Adapter,
    peripheral: Peripheral,
    /// The profile the peer's services picked.
    profile: &'static GattProfile,
    /// Host → meter. The notify characteristic is not kept: it is only needed
    /// to subscribe, which the opener has already done.
    write_char: Characteristic,
    /// The ATT MTU, as [`read_mtu`] found it when the link was set up.
    mtu: u16,
    notifications: RefCell<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>>,
    /// Bytes a notification delivered that did not fit the caller's buffer.
    pending: RefCell<VecDeque<u8>>,
    /// Heartbeats stripped so far, for `transport_status`: a rising count
    /// with no readings says the adapter is up and the meter is not.
    heartbeats: Cell<u32>,
    /// What `dmm-cli list` printed for this peer, and what `--adapter`
    /// takes to pin it.
    selector: String,
    /// The name the peer goes by, as the search found it; `None` for one
    /// opened by address with no name heard.
    advertised_name: Option<String>,
    /// The short-interval request, kept for as long as the link is open:
    /// dropping it withdraws it ([`request_short_interval`]).
    _short_interval: Option<ShortInterval>,
    /// For an adapter on Linux, the command that shortens this link's
    /// interval by hand ([`interval_command`]), for [`Transport::late_readings`].
    interval_command: Option<String>,
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

/// Open the first peer in range that `peers` takes.
pub(crate) fn open_first(peers: &BluetoothPeers) -> Result<Box<dyn Transport>> {
    open(Target::Named(peers))
}

/// Open the peer `selector` names — an address, or the platform id
/// [`list`] printed — whatever its name.
pub(crate) fn open_selected(selector: &str) -> Result<Box<dyn Transport>> {
    open(Target::At(selector))
}

/// Whether `selector` names a Bluetooth peer rather than a HID device.
///
/// A Bluetooth address (`12:34:56:78:9A:BC`) or a CoreBluetooth peripheral
/// UUID; no HID serial number or device path on any platform takes either
/// shape, so the open path can tell them apart without being told.
pub(crate) fn is_bluetooth_selector(selector: &str) -> bool {
    is_bd_addr(selector) || is_uuid(selector)
}

/// The peers in range that `peers` takes, for `dmm-cli list`.
///
/// Scans, so this takes seconds — never call it from a render path.
pub(crate) fn list(peers: &BluetoothPeers) -> Result<Vec<DeviceInfo>> {
    let rt = runtime()?;
    rt.block_on(async {
        let (_manager, adapter) = central().await?;
        let found = search(&adapter, Target::Named(peers), Match::All).await?;
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

/// Everything one open produced, so the whole sequence fits in one
/// `block_on`.
struct Opened {
    manager: Manager,
    adapter: Adapter,
    peripheral: Peripheral,
    profile: &'static GattProfile,
    write_char: Characteristic,
    mtu: u16,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    selector: String,
    advertised_name: Option<String>,
    short_interval: Option<ShortInterval>,
}

/// Open a peer and subscribe to its notifications.
fn open(target: Target<'_>) -> Result<Box<dyn Transport>> {
    let rt = runtime()?;
    let opened = rt.block_on(connect(target))?;
    // Looked up now, while the link is new, rather than from the reading
    // loop when the notice is due.
    #[cfg(target_os = "linux")]
    let interval_command = if issc::is_adapter(opened.advertised_name.as_deref()) {
        connection_handle(&opened.selector)
            .and_then(|handle| interval_command(&opened.selector, handle))
    } else {
        None
    };
    #[cfg(not(target_os = "linux"))]
    let interval_command = None;
    Ok(Box::new(Ble {
        rt,
        _manager: opened.manager,
        _adapter: opened.adapter,
        peripheral: opened.peripheral,
        profile: opened.profile,
        write_char: opened.write_char,
        mtu: opened.mtu,
        notifications: RefCell::new(opened.notifications),
        pending: RefCell::new(VecDeque::new()),
        heartbeats: Cell::new(0),
        selector: opened.selector,
        advertised_name: opened.advertised_name,
        _short_interval: opened.short_interval,
        interval_command,
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

/// Find a peer, connect to it and subscribe to its profile's notify
/// characteristic.
async fn connect(target: Target<'_>) -> Result<Opened> {
    let selector = target.selector();
    let (manager, adapter) = central().await?;
    let mut found = search(&adapter, target, Match::First).await?;
    // A named address the search did not turn up may still answer a connect.
    let mut from_address = false;
    if found.is_empty()
        && let Some(candidate) = by_address(&adapter, selector).await
    {
        found.push(candidate);
        from_address = true;
    }
    if found.is_empty() {
        return Err(not_found(selector));
    }
    let candidate = found.remove(0);
    let peripheral = candidate.peripheral.clone();
    // Nothing named and nothing heard: the scan may simply have missed an
    // awake adapter (research doc §4), so the known one is tried by address.
    let fallback = selector.is_none() && candidate.standing == Standing::Known;
    // Either way nothing vouched for the peer being awake, so one that
    // does not answer is not found rather than a fault.
    let unheard = fallback || from_address;
    if fallback {
        info!(
            "Bluetooth: no device heard, trying the known {} ({})",
            candidate.label(),
            candidate.selector()
        );
    }

    // A paired peer is often already linked, and BlueZ refuses a second
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
            if unheard {
                // A peer nothing heard that does not answer is the asleep
                // case, which is "nothing found", not a Bluetooth fault.
                info!("Bluetooth: {} did not answer: {e}", candidate.label());
                return Err(not_found(selector));
            }
            return Err(link_error(e));
        }
    }

    // An adapter just switched on can refuse the first setup on a link that
    // is up (an ATT error), and answer the next one: try once more on the
    // same link before giving it up. Both tries share one discovery bound.
    // A refused login is final.
    let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
    let subscribed = match subscribe_profile(&peripheral, deadline).await {
        Err(SetupFailure::Other(e)) => {
            debug!("Bluetooth: link setup failed, trying once more: {e}");
            tokio::time::sleep_until(deadline.min(tokio::time::Instant::now() + SETUP_RETRY_PAUSE))
                .await;
            subscribe_profile(&peripheral, deadline).await
        }
        subscribed => subscribed,
    };
    // From here on the link is up, so a failure has to take it down again:
    // left connected, the peer stays awake with nobody reading it.
    let (profile, write_char, notifications) = match subscribed {
        Ok(subscribed) => subscribed,
        Err(SetupFailure::Refused(e) | SetupFailure::Other(e)) => {
            disconnect(&peripheral).await;
            return Err(e);
        }
    };

    let Some(mtu) = read_mtu(&peripheral) else {
        disconnect(&peripheral).await;
        return Err(Error::LinkLost);
    };
    match profile.min_mtu {
        Some(min_mtu) if mtu < min_mtu => warn!(
            "Bluetooth: the link's MTU is {mtu} bytes, under the {min_mtu} the meter's \
             readings need; they may arrive cut short; if no readings arrive, report it"
        ),
        Some(_) => debug!("Bluetooth: MTU {mtu}"),
        None => {}
    }
    let short_interval = if issc::is_adapter(candidate.taken_by.as_deref()) {
        request_short_interval(&peripheral).await
    } else {
        None
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
        profile,
        write_char,
        mtu,
        notifications,
        selector,
        advertised_name: candidate.taken_by,
        short_interval,
    })
}

/// The link's ATT MTU, read once while the link is being set up.
///
/// btleplug 0.13's BlueZ backend unwraps a characteristic's MTU that BlueZ
/// leaves unset while it re-creates a reconnected peer's GATT objects (seen
/// on a UT-D07B switched off and on, 2026-09-26). The panic fires with the
/// peripheral's service lock held, poisoning it for every later call, so it
/// is caught here, once, and the open fails as a lost link: the caller's
/// next try gets a fresh peripheral. Writes use the value read here and never
/// ask again.
fn read_mtu(peripheral: &Peripheral) -> Option<u16> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| peripheral.mtu())) {
        Ok(mtu) => Some(mtu),
        Err(_) => {
            debug!("Bluetooth: the platform could not report the MTU yet");
            None
        }
    }
}

/// Ask the host stack for a short connection interval, for a peer that sets
/// a long one ([`issc::is_adapter`]), handing back what has to stay alive
/// for the request to hold.
///
/// Only WinRT takes such a request from an app (`winrt.rs`); BlueZ and
/// CoreBluetooth have no call for it, and the link keeps the interval the
/// peer asked for. Best-effort either way: the readings still come, only
/// bunched.
#[cfg(target_os = "windows")]
async fn request_short_interval(peripheral: &Peripheral) -> Option<ShortInterval> {
    let request = winrt::request_short_interval(peripheral.address());
    match tokio::time::timeout(INTERVAL_REQUEST_TIMEOUT, request)
        .await
        .unwrap_or_else(|_| Err("no answer".to_string()))
    {
        Ok(held) => {
            // Not the interval itself: it moves a second or two later, when
            // the peer's own update request is answered (research doc §5).
            debug!("Bluetooth: asked for a shorter connection interval");
            Some(held)
        }
        Err(e) => {
            debug!("Bluetooth: asking for a shorter connection interval failed: {e}");
            None
        }
    }
}

/// See the Windows version: this platform takes no request.
#[cfg(not(target_os = "windows"))]
async fn request_short_interval(_peripheral: &Peripheral) -> Option<ShortInterval> {
    debug!("Bluetooth: this platform cannot ask for a shorter connection interval");
    None
}

/// The command that moves the link with this `handle` to the peer at
/// `address` to a 30–50 ms interval until it disconnects: BlueZ's
/// `hcitool`, run as root.
///
/// `None` for anything but a Bluetooth address, which is what makes it safe
/// to put in a line run as root.
#[cfg(any(target_os = "linux", test))]
fn interval_command(address: &str, handle: u16) -> Option<String> {
    is_bd_addr(address).then(|| {
        format!("sudo hcitool lecup --handle {handle} --min 24 --max 40 --latency 0 --timeout 500")
    })
}

/// The handle BlueZ gave the connection to `address`, from `hcitool con`,
/// which needs no root. `None` without `hcitool`, with no such link, or when
/// it has not answered within [`HANDLE_LOOKUP_TIMEOUT`].
#[cfg(target_os = "linux")]
fn connection_handle(address: &str) -> Option<u16> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    let mut child = Command::new("hcitool")
        .arg("con")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + HANDLE_LOOKUP_TIMEOUT;
    // Its few lines fit the pipe, so it can run to the end before the read.
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                debug!("Bluetooth: hcitool con did not answer");
                return None;
            }
        }
    }
    let mut connections = String::new();
    child.stdout.take()?.read_to_string(&mut connections).ok()?;
    handle_in(&connections, address)
}

/// The handle on the line of `hcitool con`'s output that names `address`:
/// `< LE 12:34:56:78:9A:BC handle 2048 state 1 lm CENTRAL`.
#[cfg(any(target_os = "linux", test))]
fn handle_in(connections: &str, address: &str) -> Option<u16> {
    connections.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        words.find(|word| word.eq_ignore_ascii_case(address))?;
        if words.next()? != "handle" {
            return None;
        }
        words.next()?.parse().ok()
    })
}

/// Find a profile on a connected peer, log in where the profile has a login,
/// and subscribe to its notifications, handing back the profile, the write
/// characteristic and the stream.
///
/// The service tree fills in as the platform resolves it, so it is looked
/// at again until a profile is there or the deadline passes. What one look
/// sees depends on the backend (btleplug 0.13):
/// - CoreBluetooth answers `discover_services` once every service's
///   characteristics are in, so each look is the whole tree.
/// - WinRT lists the services all at once, but leaves out of that look any
///   service whose characteristics failed to load, and tries it again at the
///   next.
/// - BlueZ's is a snapshot of the objects the daemon exports at that moment,
///   with no wait for it to finish resolving. The connect waits for that,
///   but a peer that was already linked skips the connect, and one whose
///   connect timed out with the link up goes on without it.
///
/// So one look can miss a service the peer has. The first of [`PROFILES`],
/// ISSC, is taken the moment it is complete: nothing that arrives later can
/// change the choice. Any other profile is
/// taken once two looks in a row saw the same tree — one that stopped
/// changing, so a service that outranks it still resolving had its chance —
/// or at the deadline.
///
/// The login has a bound of its own ([`LOGIN_TIMEOUT`]), so one that starts
/// near `deadline` still gets its time.
async fn subscribe_profile(
    peripheral: &Peripheral,
    deadline: tokio::time::Instant,
) -> std::result::Result<
    (
        &'static GattProfile,
        Characteristic,
        Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    ),
    SetupFailure,
> {
    let mut last_look = None;
    let chosen = loop {
        peripheral.discover_services().await.map_err(link_error)?;
        let characteristics = peripheral.characteristics();
        let past_deadline = tokio::time::Instant::now() >= deadline;
        match choose_profile(&characteristics) {
            Ok(chosen) if chosen.profile == PROFILES[0] => break chosen,
            Ok(chosen) if past_deadline || last_look.as_ref() == Some(&characteristics) => {
                break chosen;
            }
            Err(role) if past_deadline => return Err(missing_characteristic(role).into()),
            _ => {}
        }
        last_look = Some(characteristics);
        tokio::time::sleep(DISCOVERY_POLL).await;
    };
    debug!("Bluetooth: {} profile", chosen.profile.name);

    match chosen.profile.bring_up {
        // Nothing is written to any characteristic first (research doc §3;
        // `docs/research/zotek/reverse-engineered-protocol.md` §3;
        // `docs/research/121gw/reverse-engineered-protocol.md` §3).
        BringUp::Subscribe => {}
        // A BM78xBT streams only once logged in, which comes first in r4's
        // order (`docs/research/bm78xbt/reverse-engineered-protocol.md` §3.1).
        BringUp::BrymenLogin => {
            let now = tokio::time::Instant::now();
            let login_deadline = deadline
                .max(now + LOGIN_TIMEOUT)
                .min(deadline + LOGIN_TIMEOUT);
            if login_deadline <= now {
                // The first try's login spent the bound; the stream shows
                // whether the meter took it.
                debug!("Bluetooth: no time left for the login; waiting for readings");
            } else {
                match tokio::time::timeout_at(
                    login_deadline,
                    brymen::log_in(peripheral, &chosen.write),
                )
                .await
                {
                    Ok(logged_in) => logged_in?,
                    // The stream shows whether the meter took it.
                    Err(_) => warn!(
                        "Bluetooth: the meter did not answer the login in time; waiting for readings"
                    ),
                }
                tokio::time::sleep(brymen::SETTLE).await;
            }
        }
    }
    peripheral
        .subscribe(&chosen.notify)
        .await
        .map_err(link_error)?;
    let notifications = peripheral.notifications().await.map_err(link_error)?;
    Ok((chosen.profile, chosen.write, notifications))
}

/// Why a link setup failed, for the opener's one retry.
#[derive(Debug)]
enum SetupFailure {
    /// The meter refused the login: final, never sent again.
    Refused(Error),
    /// Anything else, which a second try on the same link may get past.
    Other(Error),
}

impl From<Error> for SetupFailure {
    fn from(e: Error) -> Self {
        SetupFailure::Other(e)
    }
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

/// Move as much of `pending` into `buf` as fits, leaving the rest for the
/// next read.
fn drain_pending(pending: &mut VecDeque<u8>, buf: &mut [u8]) -> usize {
    let n = pending.len().min(buf.len());
    for (slot, byte) in buf.iter_mut().zip(pending.drain(..n)) {
        *slot = byte;
    }
    n
}

/// Nothing answered: the peer `selector` named, or with none, any peer.
fn not_found(selector: Option<&str>) -> Error {
    match selector {
        Some(selector) => Error::AdapterNotFound(selector.to_string()),
        // The scan ran and found nothing, which is what the flag says.
        None => Error::NoTransportFound {
            cables: Vec::new(),
            bluetooth_searched: true,
        },
    }
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

/// The peer answered but carries no profile we know.
fn missing_characteristic(role: &str) -> Error {
    Error::Bluetooth(format!(
        "the Bluetooth device has no data {role} characteristic we recognise — \
         it is not a supported adapter or meter, or its services never resolved"
    ))
}

impl Transport for Ble {
    fn write(&self, data: &[u8]) -> Result<()> {
        let chunk = write_chunk_size(self.mtu);
        trace!("Bluetooth TX ({} bytes): {data:02X?}", data.len());
        let write_type = write_type(self.profile, &self.write_char);
        self.rt.block_on(async {
            for part in data.chunks(chunk) {
                self.peripheral
                    .write(&self.write_char, part, write_type)
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
                // Only the profile's notify characteristic is subscribed, so
                // anything else is the platform replaying a subscription we
                // did not make; its bytes are not the meter's.
                if note.uuid.to_string() != self.profile.notify {
                    debug!("Bluetooth: ignoring a notification from {}", note.uuid);
                    return Ok(0);
                }
                let mut pending = self.pending.borrow_mut();
                pending.extend(note.value.iter().copied());
                if self.profile.strips_adapter_heartbeat {
                    let stripped = strip_heartbeats(&mut pending);
                    if stripped > 0 {
                        self.heartbeats.set(self.heartbeats.get() + stripped);
                        debug!("Bluetooth: adapter heartbeat ({stripped})");
                    }
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

    fn set_baud(&self, baud: u32) -> Result<()> {
        // GATT has no line rate to set: whatever relays a meter's UART over
        // the radio sets its own.
        debug!("Bluetooth: ignoring a request for {baud} baud");
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        // The peer's own name and identity. An adapter cannot say which
        // meter is behind it — its Device Information strings are empty
        // (research doc §1) — so this is all there is to report.
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
        let mut status = format!("MTU: {} bytes", self.mtu);
        // Only a profile an adapter can sit on has heartbeats to count.
        if self.profile.strips_adapter_heartbeat {
            status.push_str(&format!(", adapter heartbeats: {}", self.heartbeats.get()));
        }
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

    fn link(&self) -> Option<Link> {
        Some(Link::Bluetooth)
    }

    fn bluetooth_selector(&self) -> Option<&str> {
        Some(&self.selector)
    }

    fn advertised_name(&self) -> Option<&str> {
        self.advertised_name.as_deref()
    }

    /// The adapters, which set a long interval: where the open could not
    /// ask for a short one, their readings arrive in pairs. Linux gets the
    /// command that shortens it by hand.
    fn late_readings(&self) -> Option<LateReadings> {
        issc::is_adapter(self.advertised_name.as_deref()).then(|| LateReadings {
            command: self.interval_command.clone(),
        })
    }
}

impl Drop for Ble {
    /// Leaving the link up would keep the peer awake and drain its
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

    #[test]
    fn the_interval_command_names_the_peer_and_nothing_else() {
        assert_eq!(
            interval_command("12:34:56:78:9a:bc", 2048).as_deref(),
            Some("sudo hcitool lecup --handle 2048 --min 24 --max 40 --latency 0 --timeout 500")
        );
        // A CoreBluetooth id, or anything else, never reaches a root shell.
        assert_eq!(
            interval_command("1ca4b9a3-9e6f-4f1e-8b0c-2d1f3a4b5c6d", 1),
            None
        );
        assert_eq!(interval_command("12:34:56:78:9A:BC; rm -rf ~", 1), None);
    }

    #[test]
    fn the_handle_is_read_off_the_line_naming_the_peer() {
        // As `hcitool con` printed it with headphones linked too.
        let connections = "Connections:\n\
                           \t> ACL AA:BB:CC:DD:EE:FF handle 3 state 1 lm PERIPHERAL AUTH ENCRYPT\n\
                           \t< LE 12:34:56:78:9A:BC handle 2048 state 1 lm CENTRAL \n";
        assert_eq!(handle_in(connections, "12:34:56:78:9a:bc"), Some(2048));
        assert_eq!(handle_in(connections, "11:22:33:44:55:66"), None);
        assert_eq!(handle_in("", "12:34:56:78:9A:BC"), None);
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

    /// An adapter nothing heard and nothing answered is not found — the one
    /// named, or any — rather than a lost link or a stack fault.
    #[test]
    fn an_unanswered_adapter_is_not_found() {
        assert!(matches!(
            not_found(Some("12:34:56:78:9A:BC")),
            Error::AdapterNotFound(s) if s == "12:34:56:78:9A:BC"
        ));
        assert!(matches!(
            not_found(None),
            Error::NoTransportFound {
                bluetooth_searched: true,
                ..
            }
        ));
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
    /// responsive: worst case is one scan, one connect, one discovery, one
    /// login and the disconnect after it fails. The setup retry sits inside
    /// the discovery bound, and both tries' logins inside one login bound.
    #[test]
    fn open_is_bounded() {
        assert!(SCAN_POLL < SCAN_WINDOW);
        assert!(SETUP_RETRY_PAUSE < DISCOVERY_TIMEOUT);
        let worst_case =
            SCAN_WINDOW + CONNECT_TIMEOUT + DISCOVERY_TIMEOUT + LOGIN_TIMEOUT + DISCONNECT_TIMEOUT;
        assert!(worst_case <= Duration::from_secs(40), "{worst_case:?}");
    }
}
