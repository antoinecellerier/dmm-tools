//! Bluetooth LE transport for UNI-T's UT-D07 adapters and the meters with
//! the radio built in (the UT60BT, UT202BT, ZOTEK's meters, the EEVblog
//! 121GW, the Brymen BM78xBT and OWON's meters), called peers here.
//!
//! The UT-D07B is a transparent BLE-to-UART bridge: the bytes it carries are
//! the ones the USB cable carries. The UT60BT and UT202BT send the same UT61+
//! frames over the same ISSC service
//! (`docs/research/ut61-family/reverse-engineered-protocol.md` §1). So no
//! parser or framing code knows about Bluetooth.
//! Everything Bluetooth-specific is here
//! (`docs/research/ut-d07b/reverse-engineered-protocol.md`).
//!
//! A peer carries its byte stream over one of five GATT profiles, picked
//! from the services it offers once connected (`profile.rs`): ISSC's
//! transparent UART (`issc.rs`), the EEVblog 121GW's own
//! (`eevblog121gw.rs`), Brymen's own (`brymen.rs`), the FFF0/FFF4 one
//! (`fff0.rs`), or OWON's, in the same service (`owon.rs`). Between
//! discovery and the subscribe, Brymen's logs in and OWON's reads one
//! characteristic, whose value the transport hands on without knowing what
//! it says ([`Transport::info_characteristic`]).
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
mod owon;
mod profile;
mod search;
#[cfg(target_os = "windows")]
mod winrt;

use crate::error::{BluetoothIssue, CharacteristicRole, Error, Result};
use crate::transport::{BluetoothPeers, LateReadings, Link, Transport};
use crate::{BluetoothListing, DeviceInfo};
use btleplug::api::{
    Central, CentralState, Characteristic, DEFAULT_MTU_SIZE, Manager as _, Peripheral as _,
    ValueNotification,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::{Stream, StreamExt};
use issc::strip_heartbeats;
use log::{debug, info, trace, warn};
use profile::{BringUp, GattProfile, PROFILES, choose_profile, write_chunk_size, write_type};
use search::{
    Found, Match, Standing, Target, Unrecognised, by_address, is_bd_addr, is_uuid, printable,
    search,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
#[cfg(target_os = "windows")]
use winrt::ShortInterval;

/// What a short-interval request leaves to keep alive: nothing where the
/// platform takes no request ([`request_short_interval`]), and nothing on
/// Android, which keeps one for the rest of the link
/// ([`Ble::request_short_interval_once`]).
#[cfg(not(target_os = "windows"))]
type ShortInterval = std::convert::Infallible;

/// How long to scan before giving up on finding a peer.
///
/// A peer this host is already connected to is found without a scan, so
/// this bounds every other open that is not called off sooner
/// ([`crate::OpenOptions::stop`]).
const SCAN_WINDOW: Duration = Duration::from_secs(3);
/// How often the scan results are re-read while scanning.
const SCAN_POLL: Duration = Duration::from_millis(250);
/// How often an open looks at its stop flag ([`crate::OpenOptions::stop`]):
/// with [`DISCONNECT_TIMEOUT`], how long an open called off takes to return.
const STOP_POLL: Duration = Duration::from_millis(50);
/// How long a connection attempt may take before it is abandoned.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long GATT service discovery and the subscribe may take, the one retry
/// included.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a profile's login may take: at least this long from its start,
/// and never later than this long after the discovery bound, so both tries
/// of the setup share one login bound.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(8);
/// How long a profile's information read may take, bounded as the login is.
const INFO_READ_TIMEOUT: Duration = Duration::from_secs(3);
/// How long one request of the setup may go unanswered: a service
/// discovery, bounded as the login is, and the subscribe.
const SETUP_STEP_TIMEOUT: Duration = Duration::from_secs(3);
/// The pause before retrying a link setup that failed.
const SETUP_RETRY_PAUSE: Duration = Duration::from_millis(500);
/// How often to look again while the service tree is still filling in.
const DISCOVERY_POLL: Duration = Duration::from_millis(500);
/// How long asking for a short connection interval may take
/// ([`request_short_interval`]); past it the link keeps what the peer set.
#[cfg(target_os = "windows")]
const INTERVAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// An interval longer than any a host asks for: past it, the link is on
/// the one the adapter asked for, 280–318.75 ms (research doc §5).
#[cfg(target_os = "android")]
const SLOW_INTERVAL_US: u32 = 100_000;
/// How long after the open the Android request waits to see the peer's own
/// interval reported ([`IntervalRequest`]). Past it the request goes all
/// the same where the link reports none: a system that reports no updates.
#[cfg(any(target_os = "android", test))]
const ADAPTER_INTERVAL_WAIT: Duration = Duration::from_secs(5);
/// How long after the open a long interval reported late still gets the
/// Android request ([`IntervalRequest`]); past it the link is no longer
/// looked at.
#[cfg(any(target_os = "android", test))]
const ADAPTER_INTERVAL_WATCH: Duration = Duration::from_secs(30);
/// How long `hcitool con` may take to list the links ([`connection_handle`]).
#[cfg(target_os = "linux")]
const HANDLE_LOOKUP_TIMEOUT: Duration = Duration::from_secs(1);
/// How long taking a link down is waited for, after a failed open or on
/// drop: long enough for the request to reach the stack, which carries it
/// out without us. BlueZ takes longer than this to confirm one on the
/// UT-D07B, and finishes it after the program has exited (research doc §3).
const DISCONNECT_TIMEOUT: Duration = Duration::from_millis(500);

/// The environment variable that asks for a short connection interval on
/// every peer, not only the adapters, when set to `1`: for trying the fix
/// on another meter before reporting it (`docs/setup.md`, "Some Bluetooth
/// readings arrive late").
const EVERY_PEER_VAR: &str = "DMM_BLE_SHORT_INTERVAL";

/// Set by [`set_short_interval_for_every_peer`].
#[cfg(target_os = "android")]
static EVERY_PEER: AtomicBool = AtomicBool::new(false);

/// Whether to ask for a short connection interval on every Bluetooth peer,
/// as [`EVERY_PEER_VAR`] does where the application has an environment to
/// read it from: Android's has none, so its application calls this at each
/// start instead, with what its property says.
#[cfg(target_os = "android")]
pub fn set_short_interval_for_every_peer(on: bool) {
    EVERY_PEER.store(on, Ordering::Relaxed);
}

/// Whether to shorten this peer's interval: an adapter's, which sets a long
/// one ([`issc::is_adapter`]), or any peer's when [`EVERY_PEER_VAR`] asks.
fn wants_short_interval(name: Option<&str>) -> bool {
    if issc::is_adapter(name) {
        return true;
    }
    #[cfg(target_os = "android")]
    if EVERY_PEER.load(Ordering::Relaxed) {
        return true;
    }
    std::env::var_os(EVERY_PEER_VAR).is_some_and(|v| v == "1")
}

/// Where the short-interval request stands on Android
/// ([`Ble::request_short_interval_once`]).
///
/// A request made before the peer's own update is overridden by it, and
/// that update can land after the open (research doc §5). So the request
/// waits for the peer's long interval to be reported, up to
/// [`ADAPTER_INTERVAL_WAIT`]; one made without it seen is made again if it
/// shows later, until [`ADAPTER_INTERVAL_WATCH`]; one that fails is made
/// once more.
#[cfg(any(target_os = "android", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntervalRequest {
    /// Not made yet: due when the peer's long interval is reported, or at
    /// `due` if the link reports none.
    Due {
        due: std::time::Instant,
        until: std::time::Instant,
    },
    /// Made, or left out with a short interval reported, before the peer's
    /// long interval was seen: made if that shows before `until`.
    Watching { until: std::time::Instant },
    /// Failed: made once more at the next notification.
    Retry { until: std::time::Instant },
    /// Made with the long interval in force, given up on, or not this
    /// peer's to make.
    Done,
}

/// What the link reported of its interval, for [`IntervalRequest`].
#[cfg(any(target_os = "android", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reported {
    /// The peer's own, over `SLOW_INTERVAL_US`.
    Long,
    Short,
    /// Nothing yet: no update has come, or the system reports none.
    Nothing,
    /// Reading it failed.
    Unreadable,
}

#[cfg(any(target_os = "android", test))]
impl IntervalRequest {
    /// A request for a link opened at `now`.
    fn new(now: std::time::Instant) -> Self {
        Self::Due {
            due: now + ADAPTER_INTERVAL_WAIT,
            until: now + ADAPTER_INTERVAL_WATCH,
        }
    }

    /// `None` when the request is to be made now; otherwise the state to
    /// keep.
    fn before(self, now: std::time::Instant, reported: Reported) -> Option<Self> {
        match self {
            Self::Done => Some(Self::Done),
            Self::Retry { .. } => None,
            Self::Due { .. } | Self::Watching { .. } if reported == Reported::Long => None,
            Self::Due { due, .. } if now < due => Some(self),
            Self::Due { until, .. } if reported == Reported::Short => {
                Some(Self::Watching { until })
            }
            Self::Due { .. } => None,
            Self::Watching { until } if now >= until => Some(Self::Done),
            Self::Watching { .. } => Some(self),
        }
    }

    /// The state after a request made from this one: `made` whether it
    /// went through, `long` whether the peer's long interval was in force.
    fn after(self, made: bool, long: bool) -> Self {
        let until = match self {
            Self::Due { until, .. } | Self::Watching { until } | Self::Retry { until } => until,
            Self::Done => return Self::Done,
        };
        match (made, long, self) {
            (true, true, _) | (false, _, Self::Retry { .. }) => Self::Done,
            (true, false, _) => Self::Watching { until },
            (false, _, _) => Self::Retry { until },
        }
    }
}

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
    /// What a [`BringUp::ReadInfo`] profile's read found; `None` for every
    /// other profile, and for a read that failed.
    info_characteristic: Option<Vec<u8>>,
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
    /// Where the short-interval request stands on Android
    /// ([`Ble::request_short_interval_once`]); `Done` for a peer whose
    /// interval is left alone ([`wants_short_interval`]).
    #[cfg(target_os = "android")]
    short_interval: Cell<IntervalRequest>,
    /// For an adapter on Linux, the command that shortens this link's
    /// interval by hand ([`interval_command`]), for [`Transport::late_readings`].
    interval_command: Option<String>,
    /// How many peers matched as well as this one, for
    /// [`Transport::tied_peers`].
    tied: usize,
    /// Drives every btleplug call. Last, so it outlives every field whose
    /// destructor reaches into the stack.
    rt: tokio::runtime::Runtime,
}

impl Ble {
    /// Ask Android for its Balanced interval on an adapter's link, or any
    /// peer's under [`EVERY_PEER_VAR`], when [`IntervalRequest`] says to.
    ///
    /// From the reading path, not the open: the peer's own update can land
    /// after the open. Android keeps a request for the rest of the link,
    /// with nothing to hold. The readings of the 2–3 s it takes to come into
    /// force still bunch, once: the stream does not wait for it, so the
    /// connect is no slower.
    ///
    /// The interval read is the one btleplug caches from Android's updates,
    /// and keeps across a disconnect. A new link never reads an old one's
    /// only because every Android open forgets the earlier peers
    /// (`clear_peripherals` in `connect`).
    #[cfg(target_os = "android")]
    fn request_short_interval_once(&self) {
        use btleplug::api::ConnectionParameterPreset;

        let state = self.short_interval.get();
        if state == IntervalRequest::Done {
            return;
        }
        let reported = match self.rt.block_on(self.peripheral.connection_parameters()) {
            Ok(Some(p)) if p.interval_us > SLOW_INTERVAL_US => Reported::Long,
            Ok(Some(_)) => Reported::Short,
            Ok(None) => Reported::Nothing,
            Err(e) => {
                if matches!(state, IntervalRequest::Due { .. }) {
                    debug!("Bluetooth: the link's interval could not be read: {e}");
                }
                Reported::Unreadable
            }
        };
        if let Some(next) = state.before(std::time::Instant::now(), reported) {
            if next != state && matches!(next, IntervalRequest::Watching { .. }) {
                debug!("Bluetooth: the link's interval is short already; not asking yet");
            }
            self.short_interval.set(next);
            return;
        }
        if matches!(state, IntervalRequest::Due { .. }) && reported != Reported::Long {
            debug!("Bluetooth: the peer's interval was not reported; asking all the same");
        }
        let request = self
            .peripheral
            .request_connection_parameters(ConnectionParameterPreset::Balanced);
        let made = match self.rt.block_on(request) {
            Ok(()) => {
                debug!("Bluetooth: asked for a shorter connection interval");
                true
            }
            Err(e) => {
                debug!("Bluetooth: asking for a shorter connection interval failed: {e}");
                false
            }
        };
        self.short_interval
            .set(state.after(made, reported == Reported::Long));
    }

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

/// Open the first peer in range that `peers` takes, unless `stop` calls it
/// off.
///
/// `heard` is handed the other named devices the search's scan heard
/// ([`crate::OpenOptions::heard`]), whether or not a peer opened.
pub(crate) fn open_first(
    peers: &BluetoothPeers,
    stop: Option<&AtomicBool>,
    heard: Option<&Mutex<Vec<DeviceInfo>>>,
) -> Result<Box<dyn Transport>> {
    open(Target::Named(peers), stop, heard)
}

/// Open the peer `selector` names — an address, or the platform id
/// [`list`] printed — whatever its name, unless `stop` calls it off.
pub(crate) fn open_selected(
    selector: &str,
    stop: Option<&AtomicBool>,
) -> Result<Box<dyn Transport>> {
    open(Target::At(selector), stop, None)
}

/// Whether `selector` names a Bluetooth peer rather than a HID device.
///
/// A Bluetooth address (`12:34:56:78:9A:BC`) or a CoreBluetooth peripheral
/// UUID; no HID serial number or device path on any platform takes either
/// shape, so the open path can tell them apart without being told.
pub(crate) fn is_bluetooth_selector(selector: &str) -> bool {
    is_bd_addr(selector) || is_uuid(selector)
}

/// The peers in range that `peers` takes, and the named devices the scan
/// heard beside them, for `dmm-cli list`.
///
/// Scans, so this takes seconds — never call it from a render path.
pub(crate) fn list(peers: &BluetoothPeers) -> Result<BluetoothListing> {
    let rt = runtime()?;
    rt.block_on(async {
        let (_manager, adapter) = central().await?;
        let found = search(&adapter, Target::Named(peers), Match::All, true).await?;
        Ok(BluetoothListing {
            devices: found
                .candidates
                .into_iter()
                .map(|c| listed(c.selector(), c.name, c.standing == Standing::Known))
                .collect(),
            unrecognised: found.unrecognised.into_iter().map(other_device).collect(),
        })
    })
}

/// A Bluetooth device as a listing shows it.
fn listed(path: String, product: Option<String>, not_heard: bool) -> DeviceInfo {
    DeviceInfo {
        path,
        product,
        not_heard,
        serial: None,
        transport: crate::BLUETOOTH,
    }
}

/// A device the scan heard under a name no entry lists, as a listing shows
/// it: what opens it, and its name.
fn other_device(other: Unrecognised) -> DeviceInfo {
    listed(other.selector, Some(other.name), false)
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
    info_characteristic: Option<Vec<u8>>,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    selector: String,
    advertised_name: Option<String>,
    short_interval: Option<ShortInterval>,
    /// How many peers matched as well as this one ([`Candidate::tied`]).
    tied: usize,
}

/// What an open has started that a call-off must undo: the platform session
/// its search scans on, and the peer it is bringing a link up to — only one
/// it connects itself, never a link the host already had.
#[derive(Default)]
struct Begun {
    session: Option<(Manager, Adapter)>,
    connecting: Option<Peripheral>,
}

/// Open a peer and subscribe to its notifications.
///
/// Once `stop` is set the open is called off within [`STOP_POLL`]: the
/// search or connect in flight is dropped, then a link it was bringing up is
/// disconnected, as a connect that timed out is — a quit in the middle would
/// otherwise leave it linked with nobody reading it — or else its scan is
/// stopped, the host's to leave off. Nothing is looked for when the flag is
/// already set.
fn open(
    target: Target<'_>,
    stop: Option<&AtomicBool>,
    heard: Option<&Mutex<Vec<DeviceInfo>>>,
) -> Result<Box<dyn Transport>> {
    let rt = runtime()?;
    let begun = RefCell::new(Begun::default());
    let opened = rt.block_on(async {
        let Some(stop) = stop else {
            return connect(target, &begun, heard).await;
        };
        let called_off = || stop.load(Ordering::Relaxed);
        if called_off() {
            return Err(Error::Cancelled);
        }
        let connecting = Box::pin(connect(target, &begun, heard));
        let watching = Box::pin(async {
            while !called_off() {
                tokio::time::sleep(STOP_POLL).await;
            }
        });
        match futures::future::select(connecting, watching).await {
            futures::future::Either::Left((opened, _)) => opened,
            futures::future::Either::Right(((), connecting)) => {
                // Dropped before the clean-up, which a call it left in
                // flight would otherwise race.
                drop(connecting);
                let begun = begun.take();
                debug!("Bluetooth: open called off");
                if let Some(peripheral) = &begun.connecting {
                    disconnect(peripheral).await;
                } else if let Some((_manager, adapter)) = &begun.session
                    && let Ok(Err(e)) =
                        tokio::time::timeout(DISCONNECT_TIMEOUT, adapter.stop_scan()).await
                {
                    debug!("Bluetooth: stopping the scan failed: {e}");
                }
                Err(Error::Cancelled)
            }
        }
    })?;
    // Looked up now, while the link is new, rather than from the reading
    // loop when the notice is due.
    #[cfg(target_os = "linux")]
    let interval_command = if wants_short_interval(opened.advertised_name.as_deref()) {
        connection_handle(&opened.selector).map(interval_command)
    } else {
        None
    };
    #[cfg(not(target_os = "linux"))]
    let interval_command = None;
    #[cfg(target_os = "android")]
    let short_interval = if wants_short_interval(opened.advertised_name.as_deref()) {
        IntervalRequest::new(std::time::Instant::now())
    } else {
        IntervalRequest::Done
    };
    Ok(Box::new(Ble {
        rt,
        _manager: opened.manager,
        _adapter: opened.adapter,
        peripheral: opened.peripheral,
        profile: opened.profile,
        write_char: opened.write_char,
        mtu: opened.mtu,
        info_characteristic: opened.info_characteristic,
        notifications: RefCell::new(opened.notifications),
        pending: RefCell::new(VecDeque::new()),
        heartbeats: Cell::new(0),
        selector: opened.selector,
        advertised_name: opened.advertised_name,
        tied: opened.tied,
        _short_interval: opened.short_interval,
        #[cfg(target_os = "android")]
        short_interval: Cell::new(short_interval),
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
        .map_err(|e| {
            Error::from(BluetoothIssue::Runtime {
                detail: e.to_string(),
            })
        })
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
        .ok_or(Error::Bluetooth(BluetoothIssue::NoAdapter))?;
    match adapter.adapter_state().await {
        Ok(CentralState::PoweredOn) => {}
        Ok(CentralState::PoweredOff) => {
            return Err(BluetoothIssue::PoweredOff.into());
        }
        // Not every platform reports a state; an unusable adapter fails at
        // the scan instead, with the platform's own message.
        Ok(CentralState::Unknown) => debug!("Bluetooth: adapter state unknown"),
        Err(e) => debug!("Bluetooth: adapter state unavailable: {e}"),
    }
    Ok((manager, adapter))
}

/// Find a peer, connect to it and subscribe to its profile's notify
/// characteristic. What it starts goes into `begun` (see [`Begun`]) before
/// it is started.
async fn connect(
    target: Target<'_>,
    begun: &RefCell<Begun>,
    heard: Option<&Mutex<Vec<DeviceInfo>>>,
) -> Result<Opened> {
    let selector = target.selector();
    let (manager, adapter) = central().await?;
    begun.borrow_mut().session = Some((manager.clone(), adapter.clone()));
    // Android's backend keeps the peers of this process's earlier sessions,
    // and still counts one as connected after the radio was switched off
    // under it; a setup on that link is never answered. Every open there
    // starts from what a scan hears, which also fails at once, by name, when
    // the radio is off.
    if cfg!(target_os = "android")
        && let Err(e) = adapter.clear_peripherals().await
    {
        debug!("Bluetooth: could not forget earlier peers: {e}");
    }
    let Found {
        candidates: mut found,
        unrecognised,
    } = search(&adapter, target, Match::First, heard.is_some()).await?;
    // Handed over before anything is tried: a known peer that was not heard
    // is still a candidate, and the open that fails on it found no meter.
    if let Some(heard) = heard
        && let Ok(mut heard) = heard.lock()
    {
        *heard = unrecognised.into_iter().map(other_device).collect();
    }
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
    // On our UT-D07B `info` connected three times out of three, two of them
    // through this fallback (#25).
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
    let linked = peripheral.is_connected().await.unwrap_or(false);
    if !linked {
        begun.borrow_mut().connecting = Some(peripheral.clone());
    }
    if !linked && let Err(e) = peripheral.connect_with_timeout(CONNECT_TIMEOUT).await {
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
    let Subscribed {
        profile,
        write_char,
        notifications,
        info_characteristic,
    } = match subscribed {
        Ok(subscribed) => subscribed,
        Err(SetupFailure::Refused(e) | SetupFailure::Other(e)) => {
            disconnect(&peripheral).await;
            return Err(e);
        }
    };

    let mtu = read_mtu(&peripheral, deadline).await;
    match profile.min_mtu {
        Some(min_mtu) if mtu < min_mtu => warn!(
            "Bluetooth: the link's MTU is {mtu} bytes, under the {min_mtu} the meter's \
             readings need; they may arrive cut short; if no readings arrive, report it"
        ),
        _ => debug!("Bluetooth: MTU {mtu}"),
    }
    let short_interval = if wants_short_interval(candidate.taken_by.as_deref()) {
        if !issc::is_adapter(candidate.taken_by.as_deref()) {
            debug!("Bluetooth: {EVERY_PEER_VAR}: asking for a short interval on this peer too");
        }
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
        info_characteristic,
        notifications,
        selector,
        advertised_name: candidate.taken_by,
        short_interval,
        tied: candidate.tied,
    })
}

/// The link's ATT MTU, read once while the link is being set up.
///
/// On BlueZ, btleplug copies the MTU from the daemon's GATT objects at each
/// `discover_services`, and reports its [`DEFAULT_MTU_SIZE`] when none carries
/// one: a look that caught BlueZ re-creating a reconnected peer's objects
/// (seen on a UT-D07B switched off and on, 2026-09-26). So the default earns
/// one more look, within the discovery `deadline`, before it is taken;
/// otherwise a profile's `min_mtu` warning and `info` would report it as the
/// link's. Writes use the value read here and never ask again; at the default
/// they only go in smaller chunks. On our UT-D07B the second look still saw
/// the default, and the link's first write failed, so the reconnect that
/// follows a lost link took over (#25).
async fn read_mtu(peripheral: &Peripheral, deadline: tokio::time::Instant) -> u16 {
    let mtu = peripheral.mtu();
    if mtu != DEFAULT_MTU_SIZE || tokio::time::Instant::now() >= deadline {
        return mtu;
    }
    debug!("Bluetooth: the platform's default MTU; looking again");
    match tokio::time::timeout_at(deadline, peripheral.discover_services()).await {
        Ok(Ok(())) => peripheral.mtu(),
        Ok(Err(e)) => {
            debug!("Bluetooth: looking again for the MTU failed: {e}");
            mtu
        }
        Err(_) => mtu,
    }
}

/// Ask the host stack for a short connection interval, for a peer that sets
/// a long one ([`issc::is_adapter`]), handing back what has to stay alive
/// for the request to hold.
///
/// WinRT takes such a request from an app here (`winrt.rs`), and Android
/// later, from the reading path ([`Ble::request_short_interval_once`]);
/// BlueZ and CoreBluetooth have no call for it, and the link keeps the
/// interval the peer asked for. Best-effort either way: the readings still
/// come, only bunched.
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

/// See the Windows version: Android's request waits for the link's first
/// readings.
#[cfg(target_os = "android")]
async fn request_short_interval(_peripheral: &Peripheral) -> Option<ShortInterval> {
    None
}

/// See the Windows version: this platform takes no request.
#[cfg(not(any(target_os = "windows", target_os = "android")))]
async fn request_short_interval(_peripheral: &Peripheral) -> Option<ShortInterval> {
    debug!("Bluetooth: this platform cannot ask for a shorter connection interval");
    None
}

/// The command that moves the link with this `handle` to a 30–50 ms
/// interval until it disconnects: BlueZ's `hcitool`, run as root. Only the
/// numeric handle goes into the line, so nothing from the peer reaches the
/// root shell.
#[cfg(any(target_os = "linux", test))]
fn interval_command(handle: u16) -> String {
    format!("sudo hcitool lecup --handle {handle} --min 24 --max 40 --latency 0 --timeout 500")
}

/// The handle BlueZ gave the connection to `address`, from `hcitool con`,
/// which needs no root. `None` without `hcitool`, with no such link, or when
/// it has not answered within [`HANDLE_LOOKUP_TIMEOUT`]. It found the link to
/// our UT-D07B under BlueZ (#25).
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

/// Find a profile on a connected peer, run its bring-up, and subscribe to
/// its notifications.
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
/// The login and the information read have bounds of their own
/// ([`LOGIN_TIMEOUT`], [`INFO_READ_TIMEOUT`]), so one that starts near
/// `deadline` still gets its time ([`bring_up_deadline`]).
async fn subscribe_profile(
    peripheral: &Peripheral,
    deadline: tokio::time::Instant,
) -> std::result::Result<Subscribed, SetupFailure> {
    let mut last_look = None;
    let chosen = loop {
        let now = tokio::time::Instant::now();
        let until = bring_up_deadline(deadline, now, SETUP_STEP_TIMEOUT);
        answered(peripheral.discover_services(), until).await?;
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

    let mut info_characteristic = None;
    match chosen.profile.bring_up {
        // Nothing is written to any characteristic first (research doc §3;
        // `docs/research/zotek/reverse-engineered-protocol.md` §3;
        // `docs/research/121gw/reverse-engineered-protocol.md` §3).
        BringUp::Subscribe => {}
        // Read before the subscribe, as OWON's programs read it
        // (`docs/research/owon/reverse-engineered-protocol.md` §3.2, §3.3).
        // `choose_profile` took the profile only with the characteristic.
        BringUp::ReadInfo(_) => {
            if let Some(info) = &chosen.info {
                info_characteristic = read_info(peripheral, info, deadline).await;
            }
        }
        // A BM78xBT streams only once logged in, which comes first in r4's
        // order (`docs/research/bm78xbt/reverse-engineered-protocol.md` §3.1).
        BringUp::BrymenLogin => {
            let now = tokio::time::Instant::now();
            let login_deadline = bring_up_deadline(deadline, now, LOGIN_TIMEOUT);
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
    answered(peripheral.subscribe(&chosen.notify), subscribe_until()).await?;
    let notifications = peripheral.notifications().await.map_err(link_error)?;
    Ok(Subscribed {
        profile: chosen.profile,
        write_char: chosen.write,
        notifications,
        info_characteristic,
    })
}

/// What [`subscribe_profile`] set up.
struct Subscribed {
    profile: &'static GattProfile,
    write_char: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    /// What a [`BringUp::ReadInfo`] read found.
    info_characteristic: Option<Vec<u8>>,
}

/// When a bring-up step starting at `now` has to end: at least `bound`
/// after its start, and never later than `bound` past the discovery
/// `deadline`, so both tries of the setup share one bound.
fn bring_up_deadline(
    deadline: tokio::time::Instant,
    now: tokio::time::Instant,
    bound: Duration,
) -> tokio::time::Instant {
    deadline.max(now + bound).min(deadline + bound)
}

/// Read `info` once, within [`INFO_READ_TIMEOUT`].
///
/// Never fails the open: the stream does not need the value, so one not
/// read is `None`, and what that means is the protocol's.
async fn read_info(
    peripheral: &Peripheral,
    info: &Characteristic,
    deadline: tokio::time::Instant,
) -> Option<Vec<u8>> {
    let now = tokio::time::Instant::now();
    let read_deadline = bring_up_deadline(deadline, now, INFO_READ_TIMEOUT);
    if read_deadline <= now {
        // The first try's read spent the bound.
        debug!("Bluetooth: no time left to read {}", info.uuid);
        return None;
    }
    match tokio::time::timeout_at(read_deadline, peripheral.read(info)).await {
        Ok(Ok(value)) => {
            trace!("Bluetooth RX ({} bytes, read): {value:02X?}", value.len());
            debug!("Bluetooth: read {} ({} bytes)", info.uuid, value.len());
            Some(value)
        }
        Ok(Err(e)) => {
            debug!("Bluetooth: {} not read: {e}", info.uuid);
            None
        }
        Err(_) => {
            debug!("Bluetooth: {} not read in time", info.uuid);
            None
        }
    }
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

/// A failure of the Bluetooth stack itself, which carries the reason rather
/// than looking like a silent meter.
fn stack_error(e: btleplug::Error) -> Error {
    BluetoothIssue::Stack(e.to_string()).into()
}

/// When a subscribe starting now is given up. On Android alone, where one
/// was seen never to be answered: anywhere else the stack fails it on a
/// dead link, and a meter that wants pairing first holds it for as long as
/// the user takes over the system's prompt.
fn subscribe_until() -> tokio::time::Instant {
    let bound = if cfg!(target_os = "android") {
        SETUP_STEP_TIMEOUT
    } else {
        // Tokio's own "never": about thirty years.
        Duration::from_secs(86400 * 365 * 30)
    };
    tokio::time::Instant::now() + bound
}

/// One request of the link's setup, given until `until` to answer. A
/// stack normally fails a step on a link that went down, but Android's
/// leaves it waiting for good when the radio was switched off under a link
/// it still counts as up; no answer in that time is a lost link.
async fn answered(
    step: impl Future<Output = btleplug::Result<()>>,
    until: tokio::time::Instant,
) -> Result<()> {
    match tokio::time::timeout_at(until, step).await {
        Ok(done) => done.map_err(link_error),
        Err(_) => Err(Error::LinkLost),
    }
}

/// A failure of this link. `NotConnected` and a vanished device are the
/// out-of-range case: the link is lost.
fn link_error(e: btleplug::Error) -> Error {
    match e {
        btleplug::Error::NotConnected | btleplug::Error::DeviceNotFound => Error::LinkLost,
        other => BluetoothIssue::Stack(other.to_string()).into(),
    }
}

/// A write that failed on an open link, `connected` whether the link is
/// still up after it. Gone, it is a lost link whatever the stack said, as a
/// read that finds the link down is; a pulled cable reads the same. Up, the
/// stack refused this write (a permission, a GATT error), and its reason is
/// what to report: the link is fine.
///
/// A stack that marks the link down a moment after failing the write (BlueZ
/// may) gives the stack's reason for that one write; the next write or read
/// finds the link down.
fn write_error(e: btleplug::Error, connected: bool) -> Error {
    match e {
        btleplug::Error::NotConnected | btleplug::Error::DeviceNotFound => Error::LinkLost,
        other if !connected => {
            // At debug: every consumer reports the lost link itself, and
            // `debug` goes on polling one.
            debug!("Bluetooth: a write failed as the link went down: {other}");
            Error::LinkLost
        }
        other => stack_error(other),
    }
}

/// The peer answered but carries no profile we know.
fn missing_characteristic(role: CharacteristicRole) -> Error {
    BluetoothIssue::NoCharacteristic { role }.into()
}

/// What `transport_info` adds for a profile that reads a characteristic at
/// bring-up: its value in hex, or that it was not read, so a report shows
/// the bytes the protocol went by. Nothing for any other profile.
fn info_note(bring_up: &BringUp, value: Option<&[u8]>) -> String {
    match (bring_up, value) {
        (BringUp::ReadInfo(_), Some(value)) => {
            let hex: Vec<String> = value.iter().map(|b| format!("{b:02X}")).collect();
            format!(", device information: {}", hex.join(" "))
        }
        (BringUp::ReadInfo(_), None) => ", device information: not read".to_string(),
        (BringUp::Subscribe | BringUp::BrymenLogin, _) => String::new(),
    }
}

impl Transport for Ble {
    fn write(&self, data: &[u8]) -> Result<()> {
        let chunk = write_chunk_size(self.mtu);
        trace!("Bluetooth TX ({} bytes): {data:02X?}", data.len());
        let write_type = write_type(self.profile, &self.write_char);
        self.rt.block_on(async {
            for part in data.chunks(chunk) {
                if let Err(e) = self
                    .peripheral
                    .write(&self.write_char, part, write_type)
                    .await
                {
                    let connected = self.peripheral.is_connected().await.unwrap_or(false);
                    return Err(write_error(e, connected));
                }
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
                #[cfg(target_os = "android")]
                self.request_short_interval_once();
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

    fn transport_info(&self) -> Result<Option<String>> {
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
        let info = info_note(&self.profile.bring_up, self.info_characteristic.as_deref());
        Ok(Some(format!("{name} ({}){info}", self.selector)))
    }

    fn transport_status(&self) -> Result<Option<String>> {
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
        Ok(Some(status))
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

    fn tied_peers(&self) -> Option<usize> {
        (self.tied > 1).then_some(self.tied)
    }

    fn advertised_name(&self) -> Option<&str> {
        self.advertised_name.as_deref()
    }

    fn info_characteristic(&self) -> Option<&[u8]> {
        self.info_characteristic.as_deref()
    }

    /// Any peer's: an adapter sets a long interval, and where the transport
    /// could not get a short one its readings arrive in pairs; another meter
    /// in pairs is one the transport does not know to shorten. Linux gets
    /// the command that shortens it by hand ([`late_readings_notice`]).
    fn late_readings(&self) -> Option<LateReadings> {
        Some(late_readings_notice(
            issc::is_adapter(self.advertised_name.as_deref()),
            self.interval_command.clone(),
        ))
    }
}

/// The notice for a link whose readings came in pairs, asking for a report
/// everywhere but an adapter on Linux: there the cause is known, and the
/// command is the fix. Elsewhere an adapter in pairs is one whose request
/// failed or did not hold, and any other meter one the transport does not
/// know to shorten.
fn late_readings_notice(adapter: bool, command: Option<String>) -> LateReadings {
    LateReadings {
        command,
        adapter,
        report: !(adapter && cfg!(target_os = "linux")),
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

    /// A write that fails with the link down is the link lost, as capture,
    /// the GUI and `read` treat a pulled cable; with the link up, it is the
    /// stack refusing that write.
    #[test]
    fn a_failed_write_is_a_lost_link_when_the_link_is_down_or_says_so() {
        let stack = || btleplug::Error::RuntimeError("operation failed".to_string());
        assert!(matches!(write_error(stack(), false), Error::LinkLost));
        assert!(matches!(
            write_error(stack(), true),
            Error::Bluetooth(BluetoothIssue::Stack(_))
        ));
        assert!(matches!(
            write_error(btleplug::Error::NotConnected, true),
            Error::LinkLost
        ));
        assert!(matches!(
            write_error(btleplug::Error::PermissionDenied, true),
            Error::Bluetooth(_)
        ));
    }

    #[test]
    fn the_interval_request_waits_for_the_peers_long_interval() {
        use std::time::{Duration, Instant};
        let open = Instant::now();
        let soon = open + Duration::from_secs(1);
        let start = IntervalRequest::new(open);
        // Nothing reported yet: wait; the peer's long interval: ask, done.
        assert_eq!(start.before(soon, Reported::Nothing), Some(start));
        assert_eq!(start.before(soon, Reported::Short), Some(start));
        assert_eq!(start.before(soon, Reported::Long), None);
        assert_eq!(start.after(true, true), IntervalRequest::Done);
    }

    /// At the deadline with nothing reported the request goes, and is made
    /// again if the long interval shows later, until the watch ends.
    #[test]
    fn a_request_made_blind_is_made_again_when_the_long_interval_shows() {
        use std::time::{Duration, Instant};
        let open = Instant::now();
        let start = IntervalRequest::new(open);
        let due = open + ADAPTER_INTERVAL_WAIT;
        assert_eq!(start.before(due, Reported::Nothing), None);
        assert_eq!(start.before(due, Reported::Unreadable), None);
        let watching = start.after(true, false);
        let later = due + Duration::from_secs(1);
        assert_eq!(watching.before(later, Reported::Nothing), Some(watching));
        assert_eq!(watching.before(later, Reported::Long), None);
        assert_eq!(watching.after(true, true), IntervalRequest::Done);
        let past = open + ADAPTER_INTERVAL_WATCH;
        assert_eq!(
            watching.before(past, Reported::Nothing),
            Some(IntervalRequest::Done)
        );
    }

    /// A short interval reported at the deadline is left alone, and only
    /// watched.
    #[test]
    fn a_short_interval_at_the_deadline_is_not_asked_for() {
        use std::time::Instant;
        let open = Instant::now();
        let start = IntervalRequest::new(open);
        let next = start.before(open + ADAPTER_INTERVAL_WAIT, Reported::Short);
        assert!(
            matches!(next, Some(IntervalRequest::Watching { .. })),
            "{next:?}"
        );
    }

    /// A failed request is made once more, whatever the link reports, and
    /// not a third time.
    #[test]
    fn a_failed_request_is_made_once_more() {
        use std::time::Instant;
        let open = Instant::now();
        let retry = IntervalRequest::new(open).after(false, true);
        assert!(matches!(retry, IntervalRequest::Retry { .. }), "{retry:?}");
        assert_eq!(retry.before(open, Reported::Nothing), None);
        assert_eq!(retry.after(false, false), IntervalRequest::Done);
    }

    #[test]
    fn a_meter_in_pairs_asks_for_a_report_an_adapter_on_linux_does_not() {
        let meter = late_readings_notice(false, None);
        assert!(meter.report && !meter.adapter);
        let adapter = late_readings_notice(true, Some("sudo fix".to_string()));
        assert!(adapter.adapter);
        assert_eq!(adapter.report, !cfg!(target_os = "linux"));
    }

    #[test]
    fn the_interval_command_sets_the_handle_and_nothing_else() {
        assert_eq!(
            interval_command(2048),
            "sudo hcitool lecup --handle 2048 --min 24 --max 40 --latency 0 --timeout 500"
        );
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

    /// An open whose stop flag is already up looks for nothing: it returns
    /// before the stack is reached, so this needs no adapter.
    #[test]
    fn an_open_called_off_before_it_starts_looks_for_nothing() {
        let stop = AtomicBool::new(true);
        assert!(matches!(
            open_selected("12:34:56:78:9A:BC", Some(&stop)),
            Err(Error::Cancelled)
        ));
        let peers = BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        };
        assert!(matches!(
            open_first(&peers, Some(&stop), None),
            Err(Error::Cancelled)
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

    /// A bring-up step gets its bound even when it starts at the discovery
    /// deadline, and the second try gets only what the first left of it.
    #[test]
    fn both_tries_share_one_bring_up_bound() {
        let bound = Duration::from_secs(3);
        let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
        let early = deadline - Duration::from_secs(10);
        assert_eq!(bring_up_deadline(deadline, early, bound), deadline);
        assert_eq!(
            bring_up_deadline(deadline, deadline, bound),
            deadline + bound
        );
        let late = deadline + Duration::from_secs(1);
        assert_eq!(bring_up_deadline(deadline, late, bound), deadline + bound);
        let spent = deadline + bound;
        assert!(bring_up_deadline(deadline, spent, bound) <= spent);
    }

    /// The value read at bring-up reaches `info` and capture reports in
    /// hex, a failed read says so, and a profile with no read adds nothing.
    #[test]
    fn transport_info_notes_the_information_read() {
        let reads = BringUp::ReadInfo("0000fff2-0000-1000-8000-00805f9b34fb");
        assert_eq!(
            info_note(&reads, Some(&[0x12, 0x63, 0x04, 0x00, 0x09, 0x00])),
            ", device information: 12 63 04 00 09 00"
        );
        assert_eq!(info_note(&reads, None), ", device information: not read");
        assert_eq!(info_note(&BringUp::Subscribe, None), "");
        assert_eq!(info_note(&BringUp::BrymenLogin, None), "");
    }

    /// The opener's bounds have to keep the GUI's synchronous reconnect loop
    /// responsive: worst case is one scan, one connect, one discovery, one
    /// login, a subscribe that is not answered and the disconnect after it.
    /// The setup retry sits inside the discovery bound, a discovery left
    /// unanswered past it inside the login's, and both tries' logins inside
    /// one login bound.
    /// A profile has one bring-up, a login or a read, so the read fits in
    /// the login's place.
    #[test]
    fn open_is_bounded() {
        assert!(SCAN_POLL < SCAN_WINDOW);
        assert!(SETUP_RETRY_PAUSE < DISCOVERY_TIMEOUT);
        assert!(INFO_READ_TIMEOUT <= LOGIN_TIMEOUT);
        assert!(SETUP_STEP_TIMEOUT <= LOGIN_TIMEOUT);
        let worst_case = SCAN_WINDOW
            + CONNECT_TIMEOUT
            + DISCOVERY_TIMEOUT
            + LOGIN_TIMEOUT
            + SETUP_STEP_TIMEOUT
            + DISCONNECT_TIMEOUT;
        assert!(worst_case <= Duration::from_secs(40), "{worst_case:?}");
    }
}
