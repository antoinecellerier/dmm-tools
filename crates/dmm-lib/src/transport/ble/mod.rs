//! Bluetooth LE transport for UNI-T's UT-D07 adapters and the meters with
//! the radio built in (the UT60BT and UT202BT), called peers here.
//!
//! The UT-D07B is a transparent BLE-to-UART bridge: the bytes it carries are
//! the ones the USB cable carries. The built-in meters send the same UT61+
//! frames over the same ISSC service (`docs/research/new-device-candidates.md`,
//! Bluetooth section). So no parser or framing code knows about Bluetooth.
//! Everything Bluetooth-specific is here
//! (`docs/research/ut-d07b/reverse-engineered-protocol.md`).
//!
//! A peer carries its byte stream over one of two GATT profiles, picked from
//! the services it offers once connected: ISSC's transparent UART
//! (`issc.rs`), or the FFF0/FFF4 one (`fff0.rs`).
//!
//! There is no background thread and no channel: the struct owns a
//! current-thread tokio runtime and every btleplug call runs inside
//! `block_on` on the caller's thread, so nothing runs between two transport
//! calls. That is the same deal the HID transports have: a meter that streams
//! after its connect frame (the UT171 and UT181A do) keeps sending, and its
//! notifications queue in the platform's socket the way HID reports queue in
//! hidraw, to be taken off at the next read. The framing layer resyncs on the
//! next header, so no pump task is needed.

mod fff0;
mod issc;
mod search;

use crate::DeviceInfo;
use crate::error::{Error, Result};
use crate::transport::{BluetoothPeers, Transport};
use btleplug::api::{
    Central, CentralState, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use fff0::FFF0;
use futures::stream::{Stream, StreamExt};
use issc::{ISSC_UART, strip_heartbeats};
use log::{debug, info, trace};
use search::{Match, Standing, Target, by_address, is_bd_addr, is_uuid, printable, search};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::pin::Pin;
use std::time::Duration;

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
/// The pause before retrying a link setup that failed.
const SETUP_RETRY_PAUSE: Duration = Duration::from_millis(500);
/// How often to look again while the service tree is still filling in.
const DISCOVERY_POLL: Duration = Duration::from_millis(500);
/// How long taking a link down may take, after a failed open or on drop.
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Write size when the platform has not negotiated an MTU yet: the BLE
/// default ATT MTU of 23 bytes minus the three-byte write header.
const DEFAULT_WRITE_CHUNK: usize = 20;

/// A GATT layout a peer carries its byte stream on. UUIDs are lowercase:
/// btleplug renders them that way, and the comparison is textual.
#[derive(Debug, PartialEq, Eq)]
struct GattProfile {
    /// What the log calls it.
    name: &'static str,
    service: &'static str,
    /// Meter → host: its notifications carry the stream.
    notify: &'static str,
    /// Host → meter: commands are written here.
    write: &'static str,
    /// Whether UNI-T's adapter heartbeat is taken off the stream.
    strips_adapter_heartbeat: bool,
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
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    selector: String,
    advertised_name: Option<String>,
}

/// Open a peer and subscribe to its notifications.
fn open(target: Target<'_>) -> Result<Box<dyn Transport>> {
    let rt = runtime()?;
    let opened = rt.block_on(connect(target))?;
    Ok(Box::new(Ble {
        rt,
        _manager: opened.manager,
        _adapter: opened.adapter,
        peripheral: opened.peripheral,
        profile: opened.profile,
        write_char: opened.write_char,
        notifications: RefCell::new(opened.notifications),
        pending: RefCell::new(VecDeque::new()),
        heartbeats: Cell::new(0),
        selector: opened.selector,
        advertised_name: opened.advertised_name,
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
    let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
    let subscribed = match subscribe_profile(&peripheral, deadline).await {
        Err(e) => {
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
        profile,
        write_char,
        notifications,
        selector,
        advertised_name: candidate.taken_by,
    })
}

/// Find a profile on a connected peer and subscribe to its notifications,
/// handing back the profile, the write characteristic and the stream.
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
/// So one look can miss a service the peer has. ISSC is taken the moment it
/// is complete, as before there was a second profile: it comes first, so
/// nothing that arrives later can change the choice. FFF0 is the fallback,
/// taken once two looks in a row saw the same tree — one that stopped
/// changing, so an ISSC service still resolving had its chance — or at the
/// deadline.
async fn subscribe_profile(
    peripheral: &Peripheral,
    deadline: tokio::time::Instant,
) -> Result<(
    &'static GattProfile,
    Characteristic,
    Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
)> {
    let mut last_look = None;
    let chosen = loop {
        peripheral.discover_services().await.map_err(link_error)?;
        let characteristics = peripheral.characteristics();
        let past_deadline = tokio::time::Instant::now() >= deadline;
        match choose_profile(&characteristics) {
            Ok(chosen) if chosen.profile == &ISSC_UART => break chosen,
            Ok(chosen) if past_deadline || last_look.as_ref() == Some(&characteristics) => {
                break chosen;
            }
            Err(role) if past_deadline => return Err(missing_characteristic(role)),
            _ => {}
        }
        last_look = Some(characteristics);
        tokio::time::sleep(DISCOVERY_POLL).await;
    };
    debug!("Bluetooth: {} profile", chosen.profile.name);

    // Bring-up is subscribe and go on either profile: nothing is written to
    // any other characteristic first (research doc §3;
    // `docs/research/zotek/reverse-engineered-protocol.md` §3).
    peripheral
        .subscribe(&chosen.notify)
        .await
        .map_err(link_error)?;
    let notifications = peripheral.notifications().await.map_err(link_error)?;
    Ok((chosen.profile, chosen.write, notifications))
}

/// A profile's characteristics, as one look at a peer's services found them.
struct Chosen {
    profile: &'static GattProfile,
    notify: Characteristic,
    write: Characteristic,
}

/// The profile a peer's discovered characteristics carry, or the role
/// (`"write"` or `"notify"`) that is missing.
///
/// ISSC first, by its two characteristics alone, as before there was a
/// second profile. Then FFF0, whose one characteristic has to both notify
/// and take a write: FFF0 is a common service on generic modules, so it is
/// held to what the stream needs. No known ISSC peer has an FFF0 service
/// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2); one that
/// carried both would be read over ISSC.
fn choose_profile(
    characteristics: &BTreeSet<Characteristic>,
) -> std::result::Result<Chosen, &'static str> {
    let find = |service: &str, uuid: &str| {
        characteristics
            .iter()
            .find(|c| c.service_uuid.to_string() == service && c.uuid.to_string() == uuid)
    };
    let chosen = |profile, notify: &Characteristic, write: &Characteristic| Chosen {
        profile,
        notify: notify.clone(),
        write: write.clone(),
    };

    let issc_notify = find(ISSC_UART.service, ISSC_UART.notify);
    let issc_write = find(ISSC_UART.service, ISSC_UART.write);
    if let (Some(notify), Some(write)) = (issc_notify, issc_write) {
        return Ok(chosen(&ISSC_UART, notify, write));
    }

    let fff0_notify = find(FFF0.service, FFF0.notify).filter(|c| {
        // Indications carry the stream as well: btleplug's subscribe takes
        // whichever the characteristic offers, as ZOTEK's current app does
        // (spec §2).
        c.properties
            .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
    });
    let fff0_write = find(FFF0.service, FFF0.write).filter(|c| {
        c.properties
            .intersects(CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE)
    });
    if let (Some(notify), Some(write)) = (fff0_notify, fff0_write) {
        return Ok(chosen(&FFF0, notify, write));
    }

    // Name what the closer profile lacks; ISSC when neither has anything,
    // which is what a peer with no known profile always heard.
    let fff0_closer = issc_notify.is_none()
        && issc_write.is_none()
        && (fff0_notify.is_some() || fff0_write.is_some());
    let write = if fff0_closer { fff0_write } else { issc_write };
    Err(if write.is_none() { "write" } else { "notify" })
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

/// How many bytes fit in one write at `mtu`: the ATT MTU less the three-byte
/// write header, and the BLE default when the platform has not negotiated one.
fn write_chunk_size(mtu: u16) -> usize {
    match (mtu as usize).checked_sub(3) {
        Some(0) | None => DEFAULT_WRITE_CHUNK,
        Some(n) => n,
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

/// How to write to `write_char`: unacknowledged wherever the peer takes it.
///
/// An acknowledged write costs a round trip per poll (0.8 s against 0.63 s
/// per reading on our adapter), and a dead link is caught by `read_timeout`
/// instead. ISSC peers are always written that way. An FFF4 that lists only
/// acknowledged writes gets those, as ZOTEK's current app writes with the
/// characteristic's own type (`docs/research/zotek/reverse-engineered-protocol.md` §2).
fn write_type(profile: &GattProfile, write_char: &Characteristic) -> WriteType {
    let props = write_char.properties;
    if *profile == ISSC_UART
        || props.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
        || !props.contains(CharPropFlags::WRITE)
    {
        WriteType::WithoutResponse
    } else {
        WriteType::WithResponse
    }
}

/// The peer answered but carries neither profile.
fn missing_characteristic(role: &str) -> Error {
    Error::Bluetooth(format!(
        "the Bluetooth device has no data {role} characteristic we recognise — \
         it is not a supported adapter or meter, or its services never resolved"
    ))
}

impl Transport for Ble {
    fn write(&self, data: &[u8]) -> Result<()> {
        let chunk = write_chunk_size(self.peripheral.mtu());
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

    fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
        // HID-only bring-up. The families on this bridge send none, and a
        // transparent UART has nowhere to put one.
        debug!("Bluetooth: ignoring a HID feature report");
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
        let mut status = format!("MTU: {} bytes", self.peripheral.mtu());
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

    fn bluetooth_selector(&self) -> Option<&str> {
        Some(&self.selector)
    }

    fn advertised_name(&self) -> Option<&str> {
        self.advertised_name.as_deref()
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
                bluetooth_searched: true
            }
        ));
    }

    /// One characteristic as discovery reports it.
    fn characteristic(service: &str, uuid: &str, properties: CharPropFlags) -> Characteristic {
        Characteristic {
            uuid: uuid.parse().unwrap(),
            service_uuid: service.parse().unwrap(),
            properties,
            descriptors: BTreeSet::new(),
        }
    }

    /// The UART characteristics with the flags the UT-D07B lists
    /// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2).
    fn issc() -> [Characteristic; 2] {
        [
            characteristic(ISSC_UART.service, ISSC_UART.notify, CharPropFlags::NOTIFY),
            characteristic(
                ISSC_UART.service,
                ISSC_UART.write,
                CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE,
            ),
        ]
    }

    /// FFF4 with `properties`.
    fn fff4(properties: CharPropFlags) -> Characteristic {
        characteristic(FFF0.service, FFF0.notify, properties)
    }

    /// FFF4 with the flags a ZT-5B lists
    /// (`docs/research/zotek/reverse-engineered-protocol.md` §11.4).
    fn fff4_as_listed() -> Characteristic {
        fff4(CharPropFlags::READ | CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::NOTIFY)
    }

    /// The name of the profile `characteristics` pick, or the missing role.
    fn choose(
        characteristics: impl IntoIterator<Item = Characteristic>,
    ) -> std::result::Result<&'static str, &'static str> {
        choose_profile(&characteristics.into_iter().collect()).map(|c| c.profile.name)
    }

    /// A peer with ISSC is read over it, even beside an FFF0 service, with
    /// the TX characteristic subscribed and the RX one written.
    #[test]
    fn issc_is_chosen_first() {
        assert_eq!(choose(issc()), Ok(ISSC_UART.name));
        assert_eq!(
            choose(issc().into_iter().chain([fff4_as_listed()])),
            Ok(ISSC_UART.name)
        );

        let chosen = choose_profile(&issc().into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), ISSC_UART.notify);
        assert_eq!(chosen.write.uuid.to_string(), ISSC_UART.write);
    }

    /// A peer with FFF0 alone is read over FFF4 both ways, whichever write
    /// it takes.
    #[test]
    fn fff0_is_chosen_without_issc() {
        assert_eq!(choose([fff4_as_listed()]), Ok(FFF0.name));
        assert_eq!(
            choose([fff4(CharPropFlags::WRITE | CharPropFlags::NOTIFY)]),
            Ok(FFF0.name)
        );

        let chosen = choose_profile(&[fff4_as_listed()].into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), FFF0.notify);
        assert_eq!(chosen.write, chosen.notify);
    }

    /// An FFF4 that only indicates still carries the stream: btleplug
    /// subscribes to whichever the characteristic offers.
    #[test]
    fn fff4_may_indicate_instead_of_notify() {
        assert_eq!(
            choose([fff4(
                CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::INDICATE
            )]),
            Ok(FFF0.name)
        );
    }

    /// Writes go unacknowledged wherever the peer takes that: always on
    /// ISSC, and on an FFF4 unless it lists acknowledged writes alone.
    #[test]
    fn write_type_follows_what_fff4_takes() {
        let [_, issc_write] = issc();
        assert_eq!(
            write_type(&ISSC_UART, &issc_write),
            WriteType::WithoutResponse
        );
        assert_eq!(
            write_type(&FFF0, &fff4_as_listed()),
            WriteType::WithoutResponse
        );
        assert_eq!(
            write_type(&FFF0, &fff4(CharPropFlags::WRITE | CharPropFlags::NOTIFY)),
            WriteType::WithResponse
        );
    }

    /// An FFF4 that cannot carry the stream both ways is no profile, and the
    /// error names what it lacks.
    #[test]
    fn fff4_needs_notify_and_a_write() {
        assert_eq!(
            choose([fff4(CharPropFlags::READ | CharPropFlags::NOTIFY)]),
            Err("write")
        );
        assert_eq!(
            choose([fff4(
                CharPropFlags::READ | CharPropFlags::WRITE_WITHOUT_RESPONSE
            )]),
            Err("notify")
        );
        assert_eq!(choose([fff4(CharPropFlags::READ)]), Err("write"));
    }

    /// A peer with neither profile fails as it did with ISSC alone; a known
    /// characteristic under another service does not count.
    #[test]
    fn a_peer_with_neither_profile_is_refused() {
        assert_eq!(choose([]), Err("write"));
        assert_eq!(
            choose([characteristic(
                "0000180a-0000-1000-8000-00805f9b34fb",
                FFF0.notify,
                CharPropFlags::WRITE | CharPropFlags::NOTIFY,
            )]),
            Err("write")
        );
        let [notify, write] = issc();
        assert_eq!(choose([notify]), Err("write"));
        assert_eq!(choose([write]), Err("notify"));
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
