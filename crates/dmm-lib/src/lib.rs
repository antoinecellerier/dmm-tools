pub mod binary_help;
pub mod clock;
pub mod detect;
pub mod docs_tables;
pub mod error;
pub mod export;
pub mod flags;
pub mod measurement;
pub mod mock;
pub mod protocol;
pub mod replay;
pub mod specs;
pub mod stats;
pub mod stream;
pub mod transform;
pub mod transport;
pub mod wall_clock;

pub use clock::Clock;
pub use wall_clock::WallClock;

use error::{BluetoothOnlyMiss, Error, Result};
use log::{debug, info, warn};
use protocol::Protocol;
use protocol::registry::{self, SelectableDevice, Selection};
use std::ffi::CString;
use transport::{BluetoothPeers, Transport, ble, ch9325, ch9329, cp2110};

/// Top-level handle for communicating with the multimeter.
pub struct Dmm<T: Transport> {
    transport: T,
    protocol: Box<dyn Protocol>,
    /// The session's time base. Real unless a mock or replay session was
    /// opened with a clock of its own; see [`Dmm::clock`].
    clock: Clock,
    /// Whether the protocol stamps its own readings; see
    /// [`Dmm::with_protocol_timestamps`].
    protocol_stamps: bool,
    /// The name the meter gave on this link, once it has: to detection
    /// ([`Dmm::from_detected`]), to the ask ahead of `init`
    /// ([`Protocol::name_before_init`]) or to [`Dmm::get_name`]. The one copy
    /// of it, for every family: asking again costs a round trip, and a UT61+
    /// beeps at every ask.
    name: Option<String>,
}

impl<T: Transport> Dmm<T> {
    /// Create a new Dmm with the given transport and protocol.
    pub fn new(transport: T, protocol: Box<dyn Protocol>) -> Result<Self> {
        Self::open(transport, protocol, None)
    }

    /// Open the meter [`detect::detect_device`] identified on `transport`,
    /// with the registry entry it picked and the name it reported: the meter
    /// has answered on this link already, so it is not asked again.
    pub fn from_detected(transport: T, detected: &detect::Detected) -> Result<Self> {
        let protocol = (detected.device.new_protocol)();
        Self::open(transport, protocol, detected.reported_name.clone())
    }

    fn open(transport: T, protocol: Box<dyn Protocol>, name: Option<String>) -> Result<Self> {
        let profile = protocol.profile();
        info!(
            "connected to {} ({})",
            profile.model_name, profile.family_name
        );
        let mut dmm = Self {
            transport,
            protocol,
            clock: Clock::real(),
            protocol_stamps: false,
            name,
        };
        if dmm.protocol.name_before_init(&dmm.transport) {
            dmm.ask_name_before_init();
        }
        dmm.protocol.init(&dmm.transport)?;
        Ok(dmm)
    }

    /// The name, for a meter that wants it asked before `init` starts it. A
    /// name already in hand will do: the meter gave it on this link. No name
    /// is no reason to give up: `init` runs either way.
    fn ask_name_before_init(&mut self) {
        match self.get_name() {
            Ok(Some(_)) => {}
            Ok(None) => debug!("no name before init; starting the meter anyway"),
            Err(e) => debug!("no name before init ({e}); starting the meter anyway"),
        }
    }

    /// Stamp this session's readings with `clock` instead of wall time.
    ///
    /// `pub(crate)` because the only sessions that run on a non-real clock are
    /// mock and replay ones, opened through [`mock::open_mock_clocked`] and
    /// [`replay::Replay::open`].
    pub(crate) fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Keep the timestamp the protocol put on each reading.
    ///
    /// `pub(crate)` for [`replay`], the one protocol that knows when its
    /// readings happened: a recording's samples carry the session time they
    /// were taken at, and re-stamping them would move each one to whenever
    /// its playback sleep happened to end.
    pub(crate) fn with_protocol_timestamps(mut self) -> Self {
        self.protocol_stamps = true;
        self
    }

    /// The clock this session's readings are stamped with.
    ///
    /// Callers that need session time — the pacing loop, a [`WallClock`] for
    /// export — take it from here rather than reading `Instant::now()`, so a
    /// scaled or preseeded session stays consistent with its own timestamps.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Access the underlying transport (e.g. for CP2110-specific queries).
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Request a single measurement from the meter.
    ///
    /// Enriches the parsed measurement with the protocol's per-range and
    /// per-mode spec metadata when available, so consumers can display
    /// resolution/accuracy/impedance without knowing which protocol family
    /// they're talking to.
    pub fn request_measurement(&mut self) -> Result<measurement::Measurement> {
        let mut m = self.protocol.request_measurement(&self.transport)?;
        m.spec = self.protocol.spec_info(&m);
        m.mode_spec = self.protocol.mode_spec_info(&m);
        // Session time is stamped here and nowhere else: the parsers set
        // `Instant::now()` when they build the measurement, which is the same
        // thing on a real clock and wrong on any other. The exception is a
        // replay, whose samples already carry the session time they were
        // recorded at — so its exports are the recording's own timestamps.
        if !self.protocol_stamps {
            m.timestamp = self.clock.now();
        }
        Ok(m)
    }

    /// Send a named command to the meter (e.g. "hold", "range", "auto").
    pub fn send_command(&mut self, command: &str) -> Result<()> {
        self.protocol.send_command(&self.transport, command)
    }

    /// Values `setting` can be switched to from where the meter sits now.
    ///
    /// Empty for families that cannot drive the setting remotely. A one-entry
    /// list is the live value alone, which is the same offer, so a caller
    /// draws the control only once the list holds more than one entry.
    pub fn choices(
        &self,
        setting: protocol::Setting,
        current: &measurement::Measurement,
    ) -> Vec<protocol::Choice> {
        self.protocol.choices(setting, current)
    }

    /// Switch `setting` to one of the values [`Dmm::choices`] listed.
    pub fn select(&mut self, setting: protocol::Setting, id: u16) -> Result<()> {
        self.protocol.select(&self.transport, setting, id)
    }

    /// The meter's name for itself: the one it already gave on this link,
    /// else asked of it, and kept once it answers. `None` for a family with no
    /// name query.
    pub fn get_name(&mut self) -> Result<Option<String>> {
        if let Some(name) = &self.name {
            return Ok(Some(name.clone()));
        }
        let name = self.protocol.get_name(&self.transport)?;
        if name.is_some() {
            self.name.clone_from(&name);
        }
        Ok(name)
    }

    /// The name the meter already gave on this link, without asking it: for
    /// a caller that shows the name only when it costs nothing.
    pub fn known_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the device profile.
    pub fn profile(&self) -> &protocol::DeviceProfile {
        self.protocol.profile()
    }

    /// Get capture steps defined by the protocol.
    pub fn capture_steps(&self) -> Vec<protocol::CaptureStep> {
        self.protocol.capture_steps()
    }
}

/// Descriptor for a known USB-HID transport bridge.
struct KnownTransport {
    vid: u16,
    pid: u16,
    name: &'static str,
    /// Open the HID device, initialise the bridge, return a boxed Transport.
    init: fn(hidapi::HidDevice) -> Result<Box<dyn Transport>>,
}

/// The links a device family is found on, most likely first: the USB cables,
/// and UNI-T's Bluetooth adapters for the families seen on them. A meter with
/// the radio built in is on Bluetooth alone ([`device_links`]).
///
/// Sourced from the cable table in `docs/supported-devices.md`: the CP2110
/// UT-D09 covers UT61x+/UT161x/UT171x/UT880x, the Voltcraft meters and older
/// UT181A units; the CH9329 UT-D09 variant is sold for the UT181A and UT171
/// series and is confirmed on a UT61B+ (issue #19); the CH9325 UT-D04 is
/// what the UT803/UT804 use, and the UT71 and Voltcraft VC9x0, which send
/// the UT804's packets, are taken to use it too
/// (docs/research/ut71/reverse-engineered-protocol.md §1). The UT-D07B
/// Bluetooth adapter names the UT61+, UT161, UT171 and UT181 series on
/// UNI-T's accessory page (https://meters.uni-trend.com/product/ut-d-series/,
/// read 2026-09-22); the UT71 is listed for the UT-D07A only, whose GATT
/// layout we have not seen.
///
/// Two things read it. Opening only orders the candidates —
/// [`open_first_match`] still falls back to the remaining transports, so an
/// unusual cable keeps working; without the order, selecting a UT803 on a
/// bench that also has a UT61E+ attached opens the UT61E+'s CP2110 and every
/// read times out. Detection takes it literally: a bridge is probed only
/// with the fingerprints of the families listed on it, and the "no meter
/// answered" help lists those same families — so a cable a family is seen
/// on belongs here, whether or not it is the likely one.
fn preferred_transports(family: protocol::DeviceFamily) -> &'static [&'static str] {
    use protocol::DeviceFamily as F;
    match family {
        F::Ut8802 | F::Ut8803 | F::Vc880 | F::Vc890 => &["CP2110"],
        // The UT-D07B is last in each list: it is a transparent bridge, so
        // any meter with the matching socket can sit behind it, but the cable
        // is what is usually plugged in.
        F::Ut61EPlus | F::Ut171 => &["CP2110", "CH9329", BLUETOOTH],
        F::Ut181a => &["CH9329", "CP2110", BLUETOOTH],
        F::Ut80x => &["CH9325"],
        F::Mock => &[],
    }
}

/// The links one registry entry is found on: its family's, or Bluetooth
/// alone for a meter with the radio built in.
fn device_links(device: &SelectableDevice) -> &'static [&'static str] {
    if device.bluetooth_only {
        &[BLUETOOTH]
    } else {
        preferred_transports(device.family)
    }
}

/// The Bluetooth peers an open for `device` takes, by name: UNI-T's adapters
/// for a meter behind one, a meter with the radio built in by its own names,
/// and every one of both for a meter not named yet (`auto`, `dmm-cli list`).
///
/// A meter is never taken for another: a UT60BT answering an open for a
/// UT61E+ would be decoded with the UT61E+'s range tables. An address on
/// `--adapter` bypasses this: it opens whatever answers there.
fn bluetooth_peers(device: Option<&SelectableDevice>) -> BluetoothPeers {
    match device {
        Some(device) if device.bluetooth_only => BluetoothPeers {
            adapters: false,
            meters: device.bluetooth_names.to_vec(),
        },
        Some(_) => BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        },
        None => BluetoothPeers {
            adapters: true,
            meters: registry::DEVICES
                .iter()
                .flat_map(|d| d.bluetooth_names)
                .copied()
                .collect(),
        },
    }
}

/// The Bluetooth link, as `--adapter`, `dmm-cli list` and the detection
/// engine name it.
///
/// Not a [`KnownTransport`]: that table is the USB one, and the udev and
/// VID:PID invariants that guard it have nothing to say about a radio. It
/// still appears in [`preferred_transports`], which is what puts the UT61+'s
/// fingerprint on this link for detection.
pub const BLUETOOTH: &str = "Bluetooth";

/// Whether this build can reach the Bluetooth link at all.
///
/// The open path answers this for itself; a binary needs it where nothing was
/// opened — `dmm-cli list` deciding whether its "nothing found" help may
/// offer Bluetooth steps.
pub const BLUETOOTH_SUPPORTED: bool = cfg!(feature = "bluetooth");

/// Transports are tried in order — most common first.
const KNOWN_TRANSPORTS: &[KnownTransport] = &[
    KnownTransport {
        vid: cp2110::VID,
        pid: cp2110::PID,
        name: "CP2110",
        init: |dev| {
            let cp = cp2110::Cp2110::new(dev);
            cp.init_uart()?;
            Ok(Box::new(cp))
        },
    },
    KnownTransport {
        vid: ch9329::VID,
        pid: ch9329::PID,
        name: "CH9329",
        init: |dev| {
            let ch = ch9329::Ch9329::new(dev);
            ch.init()?;
            Ok(Box::new(ch))
        },
    },
    KnownTransport {
        vid: ch9325::VID,
        pid: ch9325::PID,
        name: "CH9325",
        init: |dev| {
            let mut ch = ch9325::Ch9325::new(dev);
            ch.init()?;
            Ok(Box::new(ch))
        },
    },
];

/// What a caller asks of the open path besides which meter to open.
///
/// The two travel together everywhere an open happens, so they are one value
/// rather than a pair of arguments each caller has to keep in the right order.
#[derive(Debug, Clone, Copy)]
pub struct OpenOptions<'a> {
    /// A specific adapter to open — a USB one by serial number or HID device
    /// path (as shown by [`list_devices`]), a Bluetooth one by address or
    /// peripheral identifier (as shown by [`list_bluetooth_devices`]).
    /// `None` picks the first matching adapter (and logs a warning if several
    /// are found).
    pub adapter: Option<&'a str>,
    /// Whether an open with nothing on the bus may go on to look for a
    /// Bluetooth adapter or meter in range. Off, nothing scans the radio — but an
    /// address in `adapter` is still opened, having been asked for by name.
    pub bluetooth: bool,
}

impl<'a> OpenOptions<'a> {
    /// Open whatever answers first, radio included: what a caller with no
    /// settings of its own wants.
    pub fn new() -> Self {
        Self {
            adapter: None,
            bluetooth: true,
        }
    }

    /// The same, pinned to one adapter.
    pub fn with_adapter(adapter: Option<&'a str>) -> Self {
        Self {
            adapter,
            ..Self::new()
        }
    }
}

impl Default for OpenOptions<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Open a device by registry ID, automatically selecting the transport.
///
/// Tries the USB bridges in order (CP2110, CH9329, CH9325), then an adapter in
/// Bluetooth range.
/// Returns a type-erased `Dmm<Box<dyn Transport>>` suitable for both CLI and GUI.
///
/// `id` may be [`registry::AUTO_DEVICE_ID`], in which case the meter is
/// identified from the bytes it sends; a caller that wants to know which one
/// it was calls [`open_auto`] instead.
pub fn open_device_by_id_auto(id: &str, opts: OpenOptions<'_>) -> Result<Dmm<Box<dyn Transport>>> {
    match selection_by_id(id)? {
        Selection::Auto => open_auto(opts).map(|(dmm, _)| dmm),
        Selection::Device(entry) => {
            Dmm::new(open_device_transport(entry, opts)?, (entry.new_protocol)())
        }
    }
}

/// Open the link a named meter is on, without running `Protocol::init`.
///
/// Lets a caller wrap the transport — recording wire bytes, say — before the
/// init handshake runs, so those bytes are observable too. Pass it with the
/// entry's protocol to [`Dmm::new`] to finish opening the device.
///
/// The same for a meter not named yet is [`open_transport`], then
/// [`detect::detect_device`] and [`Dmm::from_detected`], which keeps the name
/// detection got.
pub fn open_device_transport(
    device: &SelectableDevice,
    opts: OpenOptions<'_>,
) -> Result<Box<dyn Transport>> {
    if device.bluetooth_only {
        return open_bluetooth_only(device, opts);
    }
    let preferred = preferred_transports(device.family);
    let (transport, _bridge) = open_links(preferred, &bluetooth_peers(Some(device)), opts)?;
    Ok(transport)
}

/// Open the meter on the cable without being told which one it is.
///
/// Opens the first bridge found, identifies the meter on it
/// ([`detect::detect_device`]) and opens it with the entry that came back.
/// The [`detect::Detected`] is returned as well, so a caller can tell the user
/// which meter was picked and what name it reported.
pub fn open_auto(opts: OpenOptions<'_>) -> Result<(Dmm<Box<dyn Transport>>, detect::Detected)> {
    let (transport, detected) = open_detected(opts)?;
    Ok((Dmm::from_detected(transport, &detected)?, detected))
}

/// Resolve a device id for the open path: [`registry::AUTO_DEVICE_ID`], or an
/// exact registry id.
///
/// Exact rather than [`registry::resolve_selection`]'s alias matching, because
/// this is the internal id a binary already resolved once — the aliases are
/// for what the user typed.
fn selection_by_id(id: &str) -> Result<Selection> {
    if id.eq_ignore_ascii_case(registry::AUTO_DEVICE_ID) {
        return Ok(Selection::Auto);
    }
    registry::find_device(id)
        .map(Selection::Device)
        .ok_or_else(|| Error::UnknownDevice(id.to_string()))
}

/// Open a bridge with no family in mind and identify the meter behind it.
fn open_detected(opts: OpenOptions<'_>) -> Result<(Box<dyn Transport>, detect::Detected)> {
    // No family, so no link to prefer: whichever bridge answers first is the
    // one the meter is probed through.
    let (transport, bridge) = open_transport(&[], opts)?;
    let detected = detect::detect_device(&*transport, bridge)?;
    Ok((transport, detected))
}

/// Open a meter with the radio built in, over Bluetooth alone.
///
/// It is on no cable, so whatever the bus holds is another meter and the bus
/// is not opened. Every way this fails is the meter's own: a stack fault
/// ([`Error::Bluetooth`]) is passed on, and the rest name what kept the radio
/// from being searched, or that nothing in range carried the meter's name.
fn open_bluetooth_only(
    entry: &SelectableDevice,
    opts: OpenOptions<'_>,
) -> Result<Box<dyn Transport>> {
    let miss = if !BLUETOOTH_SUPPORTED {
        BluetoothOnlyMiss::NotBuilt
    } else if let Some(selector) = opts.adapter {
        if !ble::is_bluetooth_selector(selector) {
            BluetoothOnlyMiss::UsbAdapter
        } else {
            return ble::open_selected(selector);
        }
    } else if !opts.bluetooth {
        BluetoothOnlyMiss::SwitchedOff
    } else {
        match ble::open_first(&bluetooth_peers(Some(entry))) {
            Err(Error::NoTransportFound { .. }) => BluetoothOnlyMiss::NotInRange,
            opened => return opened,
        }
    };
    Err(Error::BluetoothOnly {
        model: entry.display_name,
        activation: entry.activation_instructions,
        miss,
    })
}

/// Open a bridge and hand back the transport alone, with no protocol, for a
/// meter not identified yet.
///
/// `preferred` orders the links to try, empty when nothing is known about
/// the meter; on the radio, any UNI-T adapter or meter is taken, since
/// detection picks the entry afterwards. The returned name
/// is the bridge's (`"CP2110"`, `"CH9329"`, `"CH9325"`, [`BLUETOOTH`]), which
/// [`detect::detect_device`] needs to know which probes are worth sending.
///
/// The USB bus comes first and Bluetooth is the fallback, which has one known
/// consequence: a cable with a silent meter on it wins over a live meter on a
/// UT-D07B, and detection ends in [`Error::DeviceNotIdentified`] on the cable.
/// Unplug it, or name the adapter.
///
/// This is the split half of [`open_auto`]: a caller that wants to wrap the
/// transport before *any* byte flows — recording the detection probe itself,
/// say — opens it here, detects separately and finishes with
/// [`Dmm::from_detected`].
pub fn open_transport(
    preferred: &[&'static str],
    opts: OpenOptions<'_>,
) -> Result<(Box<dyn Transport>, &'static str)> {
    open_links(preferred, &bluetooth_peers(None), opts)
}

/// [`open_transport`], taking only `peers` on the radio.
fn open_links(
    preferred: &[&'static str],
    peers: &BluetoothPeers,
    opts: OpenOptions<'_>,
) -> Result<(Box<dyn Transport>, &'static str)> {
    // An address or a peripheral UUID can only be a Bluetooth adapter, so the
    // selector alone says which opener the user meant — and asking for one by
    // name is asking for the radio, whatever the probing setting says.
    if let Some(selector) = opts.adapter.filter(|s| ble::is_bluetooth_selector(s)) {
        return Ok((ble::open_selected(selector)?, BLUETOOTH));
    }

    let radio_gets_a_turn = bluetooth_is_next(preferred, opts);
    let hid = open_hid_transport(preferred, opts.adapter);
    match hid {
        Ok(opened) => Ok(opened),
        Err(err) if radio_gets_a_turn && usb_failure_falls_through(&err) => {
            match ble::open_first(peers) {
                Ok(transport) => Ok((transport, BLUETOOTH)),
                Err(bluetooth_err) => {
                    // The USB error is what the user acts on: it lists the
                    // cables that were looked for. Why Bluetooth found
                    // nothing — stack off, nothing in range — stays in the
                    // log. The error says the radio was searched, so the help
                    // titles itself on both links rather than on the cable.
                    info!("no meter over Bluetooth either: {bluetooth_err}");
                    Err(searched_bluetooth(err))
                }
            }
        }
        Err(err) => Err(err),
    }
}

/// Whether the radio gets a turn when no cable answers.
///
/// Only on the auto path: a user who named a USB adapter asked for that one,
/// and a meter answering on a radio instead is not what they meant. A build
/// without the feature has no radio to search, and neither has a caller that
/// switched probing off.
fn bluetooth_is_next(preferred: &[&'static str], opts: OpenOptions<'_>) -> bool {
    cfg!(feature = "bluetooth")
        && opts.bluetooth
        && opts.adapter.is_none()
        && (preferred.is_empty() || preferred.contains(&BLUETOOTH))
}

/// Whether this USB failure is one the radio may answer instead. `Hid` is in
/// the list because it is what a missing or broken HID API returns, which on
/// a machine with no USB support at all must not stop Bluetooth working.
fn usb_failure_falls_through(err: &Error) -> bool {
    matches!(err, Error::NoTransportFound { .. } | Error::Hid(_))
}

/// Mark a "nothing found" error as having looked at the radio as well.
fn searched_bluetooth(err: Error) -> Error {
    match err {
        Error::NoTransportFound { .. } => Error::NoTransportFound {
            bluetooth_searched: true,
        },
        other => other,
    }
}

/// Open one of the USB-HID bridges, the path every meter but a Bluetooth one
/// takes.
fn open_hid_transport(
    preferred: &[&'static str],
    adapter: Option<&str>,
) -> Result<(Box<dyn Transport>, &'static str)> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;

    let (device, kt) = match adapter {
        Some(adapter) => open_with_adapter(&api, adapter),
        None => open_first_match(&api, preferred),
    }?;
    info!(
        "found {} adapter (VID={:#06x} PID={:#06x})",
        kt.name, kt.vid, kt.pid
    );
    Ok(((kt.init)(device)?, kt.name))
}

/// The hardware meters reachable over `bridge`, the inverse of
/// [`device_links`].
///
/// What the "no meter answered" help lists: with nothing identified on a
/// bridge, these are the meters that could have been on it, and their
/// activation instructions are what the user has to act on.
pub fn devices_on_bridge(bridge: &str) -> Vec<&'static SelectableDevice> {
    registry::DEVICES
        .iter()
        .filter(|d| d.requires_hardware)
        .filter(|d| device_links(d).contains(&bridge))
        .collect()
}

/// Open a specific adapter identified by serial number or HID path.
///
/// Tries serial number matching first (most common), then falls back to
/// HID path matching. This avoids needing to guess the format — path
/// formats vary across platforms (Linux `/dev/hidrawN`, Windows `\\?\HID#...`,
/// macOS `IOService:...`).
fn open_with_adapter(
    api: &hidapi::HidApi,
    adapter: &str,
) -> Result<(hidapi::HidDevice, &'static KnownTransport)> {
    // Try serial number first — fast, no enumeration needed.
    for kt in KNOWN_TRANSPORTS {
        if let Ok(device) = api.open_serial(kt.vid, kt.pid, adapter) {
            return Ok((device, kt));
        }
    }

    // Fall back to HID path — enumerate to determine which transport.
    let dev_info = api
        .device_list()
        .find(|dev| dev.path().to_string_lossy() == adapter);

    if let Some(dev_info) = dev_info {
        let kt = KNOWN_TRANSPORTS
            .iter()
            .find(|kt| dev_info.vendor_id() == kt.vid && dev_info.product_id() == kt.pid)
            .ok_or_else(|| {
                Error::AdapterNotFound(format!(
                    "{adapter} (device exists but is not a supported USB adapter)"
                ))
            })?;

        let path =
            CString::new(adapter).map_err(|_| Error::AdapterNotFound(adapter.to_string()))?;
        let device = api.open_path(&path).map_err(Error::Hid)?;
        Ok((device, kt))
    } else {
        Err(Error::AdapterNotFound(adapter.to_string()))
    }
}

/// Open the first matching adapter, `preferred` naming the cables to try
/// first — the ones the selected family ships with, or nothing at all when no
/// family has been selected. Warns if multiple adapters are found.
fn open_first_match(
    api: &hidapi::HidApi,
    preferred: &[&'static str],
) -> Result<(hidapi::HidDevice, &'static KnownTransport)> {
    let match_count: usize = api
        .device_list()
        .filter(|dev| {
            KNOWN_TRANSPORTS
                .iter()
                .any(|kt| dev.vendor_id() == kt.vid && dev.product_id() == kt.pid)
        })
        .count();

    if match_count > 1 {
        // Nothing to prefer when the meter is still unknown — the probe runs
        // on whichever bridge opens first.
        let preference = if preferred.is_empty() {
            ""
        } else {
            " Preferring the cable this meter uses."
        };
        warn!(
            "Multiple USB cables found ({match_count} devices).{preference} Pass --adapter to pick one."
        );
    }

    // Preferred cables first, then everything else as a fallback so an
    // unusual pairing still connects.
    let ordered = preferred
        .iter()
        .filter_map(|name| KNOWN_TRANSPORTS.iter().find(|kt| kt.name == *name))
        .chain(
            KNOWN_TRANSPORTS
                .iter()
                .filter(|kt| !preferred.contains(&kt.name)),
        );

    for kt in ordered {
        if let Ok(device) = api.open(kt.vid, kt.pid) {
            return Ok((device, kt));
        }
    }

    // The bus alone was looked at here; [`open_transport`] marks the error
    // if it goes on to search the radio.
    Err(Error::NoTransportFound {
        bluetooth_searched: false,
    })
}

/// List all connected USB adapters (CP2110, CH9329, CH9325).
///
/// USB only, and instant: the Bluetooth adapters and meters in range come from
/// [`list_bluetooth_devices`], which has to scan for them.
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;
    let mut devices = Vec::new();

    for dev in api.device_list() {
        let transport = KNOWN_TRANSPORTS
            .iter()
            .find(|kt| dev.vendor_id() == kt.vid && dev.product_id() == kt.pid);
        let Some(kt) = transport else { continue };

        devices.push(DeviceInfo {
            path: dev.path().to_string_lossy().into_owned(),
            product: dev.product_string().map(|s| s.to_string()),
            serial: dev
                .serial_number()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            transport: kt.name,
            not_heard: false,
        });
    }

    Ok(devices)
}

/// The UNI-T Bluetooth adapters and meters with the radio built in that are
/// in range, then the known ones the scan did not hear
/// ([`DeviceInfo::not_heard`]).
///
/// Separate from [`list_devices`] because it scans, which takes seconds: the
/// USB listing backs a GUI control and has to stay instant. `path` is what
/// `--adapter` takes to pin one — its address, or the peripheral identifier
/// on a platform that exposes no address.
pub fn list_bluetooth_devices() -> Result<Vec<DeviceInfo>> {
    ble::list(&bluetooth_peers(None))
}

/// Whether an `--adapter` value names a Bluetooth adapter rather than a USB
/// one — an address or a peripheral identifier, neither of which a HID serial
/// number or device path can look like.
///
/// The open path decides this for itself; both binaries ask so that an
/// adapter nothing answered is explained in terms of the link it was on. A
/// build without the `bluetooth` feature answers `false`: there is no radio
/// for the value to have named.
pub fn is_bluetooth_selector(selector: &str) -> bool {
    ble::is_bluetooth_selector(selector)
}

/// Information about a connected adapter.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub product: Option<String>,
    pub serial: Option<String>,
    /// Transport type: "CP2110", "CH9329", "CH9325", or [`BLUETOOTH`].
    pub transport: &'static str,
    /// A Bluetooth adapter the platform knows (paired) that the scan did not
    /// hear: asleep, or missed by the scan. `--adapter` still tries it.
    /// Always false for a cable.
    pub not_heard: bool,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.path, self.transport)?;
        if let Some(ref product) = self.product {
            write!(f, " — {product}")?;
        }
        if let Some(ref serial) = self.serial {
            write!(f, " (S/N: {serial})")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ut61eplus::Ut61PlusProtocol;
    use crate::transport::mock::MockTransport;

    /// Build a complete response frame (header + length + payload + checksum)
    /// for a measurement with the given parameters.
    fn make_measurement_response(
        mode: u8,
        range: u8,
        display: &[u8; 7],
        progress: (u8, u8),
        flags: (u8, u8, u8),
    ) -> Vec<u8> {
        let payload: Vec<u8> = vec![
            mode,         // raw, no 0x30 prefix
            range | 0x30, // has 0x30 prefix
            display[0],
            display[1],
            display[2],
            display[3],
            display[4],
            display[5],
            display[6],
            progress.0,     // raw, no 0x30 prefix
            progress.1,     // raw, no 0x30 prefix
            flags.0 | 0x30, // has 0x30 prefix
            flags.1 | 0x30, // has 0x30 prefix
            flags.2 | 0x30, // has 0x30 prefix
        ];
        crate::protocol::framing::test_frame_be16(&payload)
    }

    #[test]
    fn dmm_request_measurement() {
        let response =
            make_measurement_response(0x02, 0x01, b"  5.678", (0x05, 0x0A), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "DC V");
        assert_eq!(m.unit, "V");
        assert!(m.flags.auto_range);
    }

    #[test]
    fn dmm_split_response() {
        let full =
            make_measurement_response(0x06, 0x02, b" 12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let (part1, part2) = full.split_at(10);
        let mock = MockTransport::new(vec![part1.to_vec(), part2.to_vec()]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Ω");
        assert_eq!(m.range_label, "22kΩ");
    }

    #[test]
    fn dmm_timeout() {
        let mock = MockTransport::new(vec![]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let result = dmm.request_measurement();
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[test]
    fn dmm_sends_correct_request_bytes() {
        let response =
            make_measurement_response(0x02, 0x00, b" 0.0000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let _ = dmm.request_measurement().unwrap();

        let written = dmm.transport.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9]);
    }

    #[test]
    fn dmm_send_command() {
        let mock = MockTransport::new(vec![]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        dmm.send_command("hold").unwrap();

        let written = dmm.transport.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0],
            crate::protocol::ut61eplus::command::Command::Hold.encode()
        );
    }

    #[test]
    fn dmm_unsupported_command() {
        let mock = MockTransport::new(vec![]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let result = dmm.send_command("nonexistent");
        assert!(matches!(result, Err(Error::UnsupportedCommand(_))));
    }

    #[test]
    fn dmm_response_with_leading_garbage() {
        let mut data = vec![0xFF, 0xFE, 0x00];
        data.extend_from_slice(&make_measurement_response(
            0x00,
            0x00,
            b"  1.234",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ));
        let mock = MockTransport::new(vec![data]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "AC V");
    }

    #[test]
    fn dmm_multiple_measurements() {
        let r1 =
            make_measurement_response(0x02, 0x00, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let r2 =
            make_measurement_response(0x02, 0x00, b"  2.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![r1, r2]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m1 = dmm.request_measurement().unwrap();
        let m2 = dmm.request_measurement().unwrap();
        assert!(
            matches!(m1.value, measurement::MeasuredValue::Normal(v) if (v - 1.0).abs() < 1e-6)
        );
        assert!(
            matches!(m2.value, measurement::MeasuredValue::Normal(v) if (v - 2.0).abs() < 1e-6)
        );
    }

    /// Readings carry session time, not the instant the parser happened to
    /// build them: a scaled or preseeded session must be able to hand out
    /// timestamps its own clock agrees with.
    #[test]
    fn dmm_stamps_readings_with_its_clock() {
        let r1 =
            make_measurement_response(0x02, 0x00, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let r2 =
            make_measurement_response(0x02, 0x00, b"  2.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let clock = Clock::manual();
        let mock = MockTransport::new(vec![r1, r2]);
        let mut dmm = Dmm::new(mock, Box::new(Ut61PlusProtocol::new()))
            .unwrap()
            .with_clock(clock.clone());

        let m1 = dmm.request_measurement().unwrap();
        assert_eq!(m1.timestamp, clock.now());

        clock.advance(std::time::Duration::from_secs(5));
        let m2 = dmm.request_measurement().unwrap();
        assert_eq!(
            m2.timestamp.saturating_duration_since(m1.timestamp),
            std::time::Duration::from_secs(5)
        );
    }

    /// A mock on the named link, as `open_transport` hands one back.
    struct Link {
        mock: MockTransport,
        name: &'static str,
    }

    impl Transport for Link {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.mock.write(data)
        }
        fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
            self.mock.read_timeout(buf, timeout_ms)
        }
        fn send_feature_report(&self, data: &[u8]) -> Result<()> {
            self.mock.send_feature_report(data)
        }
        fn transport_name(&self) -> &'static str {
            self.name
        }
    }

    /// What a caller does with the name once the session is open.
    #[derive(Clone, Copy, Debug)]
    enum NameUse {
        /// `read` in text, CSV or JSON, and the GUI with its name option off:
        /// only a name already in hand.
        Known,
        /// `info`, `capture`, `read --format replay`, the GUI with its name
        /// option on — asked twice, to show the second costs nothing.
        Asked,
    }

    /// Open `model` on `link` the way the binaries do, `auto` through
    /// detection, and use the name as `then` says. Returns the Get Name
    /// writes, every write, and the name the caller ends up with.
    fn name_asks(
        model: &str,
        name: &str,
        link: &'static str,
        auto: bool,
        then: NameUse,
    ) -> (usize, Vec<Vec<u8>>, Option<String>) {
        const ACK: [u8; 7] = [0xAB, 0xCD, 0x04, 0xFF, 0x00, 0x02, 0x7B];
        let transport = Link {
            mock: MockTransport::new(vec![
                ACK.to_vec(),
                crate::protocol::framing::test_frame_be16(name.as_bytes()),
            ]),
            name: link,
        };
        let mut dmm = if auto {
            let detected = detect::detect_device(&transport, link).unwrap();
            assert_eq!(detected.device.id, model);
            Dmm::from_detected(transport, &detected).unwrap()
        } else {
            let entry = registry::find_device(model).expect("registry entry");
            Dmm::new(transport, (entry.new_protocol)()).unwrap()
        };
        let shown = match then {
            NameUse::Known => dmm.known_name().map(str::to_owned),
            NameUse::Asked => {
                let first = dmm.get_name().unwrap();
                assert_eq!(dmm.get_name().unwrap(), first);
                first
            }
        };
        let written = dmm.transport().mock.written.borrow().clone();
        let get_name = crate::protocol::ut61eplus::command::Command::GetName.encode();
        let asks = written.iter().filter(|w| **w == get_name).count();
        (asks, written, shown)
    }

    /// The meter is asked its name at most once a session, whoever wants it:
    /// detection's answer, or the ask ahead of `init`, is the one every later
    /// caller gets. A meter nothing asked stays unasked.
    #[test]
    fn a_session_asks_the_meter_its_name_at_most_once() {
        use NameUse::{Asked, Known};
        let ut61e = |link, auto, then| name_asks("ut61eplus", "UT61E+", link, auto, then);
        let ut60bt = |auto, then| name_asks("ut60bt", "UT60BT", BLUETOOTH, auto, then);
        let named = |name: &str| Some(name.to_string());
        for link in ["CP2110", BLUETOOTH] {
            // Detection asks; nothing after it does.
            assert_eq!(ut61e(link, true, Known).0, 1, "{link}");
            assert_eq!(ut61e(link, true, Known).2, named("UT61E+"), "{link}");
            assert_eq!(ut61e(link, true, Asked).0, 1, "{link}");
            assert_eq!(ut61e(link, true, Asked).2, named("UT61E+"), "{link}");
            // Named: only a caller that wants the name asks for it.
            assert_eq!(ut61e(link, false, Known), (0, ut61e_init(link), None));
            assert_eq!(ut61e(link, false, Asked).0, 1, "{link}");
            assert_eq!(ut61e(link, false, Asked).2, named("UT61E+"), "{link}");
        }
        // Asked before the stream, by detection or by the session, and never
        // again.
        let name_then_start = vec![
            crate::protocol::ut61eplus::command::Command::GetName
                .encode()
                .to_vec(),
            crate::protocol::ut61eplus::command::Command::StartStream
                .encode()
                .to_vec(),
        ];
        for auto in [true, false] {
            for then in [Known, Asked] {
                assert_eq!(
                    ut60bt(auto, then),
                    (1, name_then_start.clone(), named("UT60BT")),
                    "auto {auto}, {then:?}"
                );
            }
        }
    }

    /// What a UT61E+ session's `init` writes on `link`: nothing on a cable,
    /// the stream start over Bluetooth.
    fn ut61e_init(link: &str) -> Vec<Vec<u8>> {
        if link == BLUETOOTH {
            vec![
                crate::protocol::ut61eplus::command::Command::StartStream
                    .encode()
                    .to_vec(),
            ]
        } else {
            Vec::new()
        }
    }

    #[test]
    fn device_info_display() {
        let info = DeviceInfo {
            path: "/dev/hidraw0".to_string(),
            product: Some("UT61E+".to_string()),
            serial: Some("12345".to_string()),
            transport: "CP2110",
            not_heard: false,
        };
        let s = info.to_string();
        assert!(s.contains("/dev/hidraw0"));
        assert!(s.contains("CP2110"));
        assert!(s.contains("UT61E+"));
        assert!(s.contains("12345"));
    }

    #[test]
    fn device_info_display_no_optional_fields() {
        let info = DeviceInfo {
            path: "/dev/hidraw0".to_string(),
            product: None,
            serial: None,
            transport: "CH9329",
            not_heard: false,
        };
        assert_eq!(info.to_string(), "/dev/hidraw0 [CH9329]");
    }

    #[test]
    fn dmm_capacitance_mode() {
        let response =
            make_measurement_response(0x09, 0x03, b"  4.567", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Capacitance");
        assert_eq!(m.unit, "µF");
        assert_eq!(m.range_label, "22µF");
    }

    #[test]
    fn dmm_hz_mode() {
        let response =
            make_measurement_response(0x04, 0x02, b" 1.2345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, "Hz");
        assert_eq!(m.unit, "kHz");
        assert_eq!(m.range_label, "2.2kHz");
    }

    #[test]
    fn dmm_negative_value() {
        let response =
            make_measurement_response(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let protocol = Box::new(Ut61PlusProtocol::new());
        let mut dmm = Dmm::new(mock, protocol).unwrap();

        let m = dmm.request_measurement().unwrap();
        assert!(
            matches!(m.value, measurement::MeasuredValue::Normal(v) if (v - (-12.345)).abs() < 1e-6)
        );
    }

    /// A typo in `preferred_transports` would silently degrade to the old
    /// fixed order rather than failing to build.
    #[test]
    fn preferred_transport_names_exist() {
        for device in protocol::registry::DEVICES {
            for name in device_links(device) {
                assert!(
                    *name == BLUETOOTH || KNOWN_TRANSPORTS.iter().any(|kt| kt.name == *name),
                    "device {} prefers unknown transport {name:?}",
                    device.id,
                );
            }
        }
    }

    /// Which links a failed open looked at, which is what the binaries title
    /// their help on. A build without the feature searches no radio, and
    /// neither does a meter that only ships with a cable or a user who named
    /// a USB adapter.
    #[test]
    fn the_radio_gets_a_turn_only_where_it_could_answer() {
        let bluetooth = cfg!(feature = "bluetooth");
        let opts = OpenOptions::new();
        assert_eq!(bluetooth_is_next(&[], opts), bluetooth, "the auto path");
        assert_eq!(
            bluetooth_is_next(
                preferred_transports(protocol::DeviceFamily::Ut61EPlus),
                opts
            ),
            bluetooth,
            "a family the adapter carries"
        );
        assert!(
            !bluetooth_is_next(preferred_transports(protocol::DeviceFamily::Ut80x), opts),
            "a cable-only family"
        );
        assert!(
            !bluetooth_is_next(&[], OpenOptions::with_adapter(Some("00C5B27A"))),
            "a named USB adapter"
        );
        assert!(
            !bluetooth_is_next(
                &[],
                OpenOptions {
                    bluetooth: false,
                    ..OpenOptions::new()
                }
            ),
            "probing switched off"
        );
    }

    /// Every hardware device must name the cable it ships with, otherwise it
    /// falls back to opening whichever adapter happens to be first on the bus.
    #[test]
    fn every_hardware_family_names_its_cable() {
        for device in protocol::registry::DEVICES {
            if !device.requires_hardware {
                continue;
            }
            assert!(
                !device_links(device).is_empty(),
                "device {} has no preferred transport",
                device.id,
            );
        }
    }

    /// The preferred cable must come first, but the others stay reachable so an
    /// unusual pairing still connects.
    #[test]
    fn preference_orders_without_excluding() {
        let preferred = preferred_transports(protocol::DeviceFamily::Ut80x);
        let ordered: Vec<&str> = preferred
            .iter()
            .filter_map(|name| KNOWN_TRANSPORTS.iter().find(|kt| kt.name == *name))
            .chain(
                KNOWN_TRANSPORTS
                    .iter()
                    .filter(|kt| !preferred.contains(&kt.name)),
            )
            .map(|kt| kt.name)
            .collect();

        assert_eq!(ordered[0], "CH9325", "the UT80x family uses the CH9325");
        assert_eq!(
            ordered.len(),
            KNOWN_TRANSPORTS.len(),
            "every transport must stay reachable as a fallback"
        );
    }

    /// Every bridge carries meters, so the "no meter answered" help always has
    /// something to list — an empty list would print a bare failure.
    #[test]
    fn every_bridge_carries_meters() {
        for kt in KNOWN_TRANSPORTS {
            assert!(
                !devices_on_bridge(kt.name).is_empty(),
                "no device lists {} as its cable",
                kt.name
            );
        }
        // The families UNI-T's accessory page names for the UT-D07B: that is
        // what detection probes for over it and what the help it prints
        // offers. The UT80x is not among them — the UT71 is listed for the
        // UT-D07A, a different adapter.
        use protocol::DeviceFamily as F;
        let on_bluetooth: Vec<&str> = devices_on_bridge(BLUETOOTH).iter().map(|d| d.id).collect();
        let listed: Vec<&str> = registry::DEVICES
            .iter()
            .filter(|d| d.requires_hardware)
            .filter(|d| matches!(d.family, F::Ut61EPlus | F::Ut171 | F::Ut181a))
            .map(|d| d.id)
            .collect();
        assert_eq!(on_bluetooth, listed);
        assert!(!on_bluetooth.is_empty());
        assert!(
            !devices_on_bridge(BLUETOOTH)
                .iter()
                .any(|d| d.family == F::Ut80x)
        );
        assert!(devices_on_bridge("no such bridge").is_empty());
    }

    /// Each entry takes only its own peers on the radio: a meter behind an
    /// adapter the adapters, a meter with the radio built in its own names,
    /// and a meter not named yet every one of them.
    #[test]
    fn each_entry_takes_only_its_own_bluetooth_peers() {
        let peers = |id| bluetooth_peers(Some(registry::find_device(id).expect("registry entry")));
        let adapters = BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        };
        for id in ["ut61eplus", "ut61b+", "ut161e", "ut171", "ut181a"] {
            assert_eq!(peers(id), adapters, "{id}");
        }
        for (id, name) in [("ut60bt", "UT60BT"), ("ut202bt", "UT202BT")] {
            assert_eq!(
                peers(id),
                BluetoothPeers {
                    adapters: false,
                    meters: vec![name],
                }
            );
        }
        assert_eq!(
            bluetooth_peers(None),
            BluetoothPeers {
                adapters: true,
                meters: vec!["UT60BT", "UT202BT"],
            }
        );
    }

    /// A meter with the radio built in never falls back to the bus, and says
    /// what kept it from the radio rather than that no cable was found.
    #[test]
    fn a_bluetooth_only_meter_names_what_kept_it_off_the_radio() {
        let ut60bt = registry::find_device("ut60bt").expect("registry entry");
        let miss = |opts| match open_bluetooth_only(ut60bt, opts) {
            Err(Error::BluetoothOnly {
                model,
                activation,
                miss,
                ..
            }) => {
                assert_eq!(model, "UT60BT");
                assert_eq!(activation, ut60bt.activation_instructions);
                miss
            }
            Err(e) => panic!("unexpected error: {e}"),
            Ok(_) => panic!("opened a meter with no radio searched"),
        };
        let switched_off = OpenOptions {
            adapter: None,
            bluetooth: false,
        };
        let usb_adapter = OpenOptions {
            adapter: Some("00C5B27A"),
            bluetooth: true,
        };
        let (off, usb) = if BLUETOOTH_SUPPORTED {
            (
                BluetoothOnlyMiss::SwitchedOff,
                BluetoothOnlyMiss::UsbAdapter,
            )
        } else {
            (BluetoothOnlyMiss::NotBuilt, BluetoothOnlyMiss::NotBuilt)
        };
        assert_eq!(miss(switched_off), off);
        assert_eq!(miss(usb_adapter), usb);
    }

    /// A meter with Bluetooth built in has no cable, so no cable's "no meter
    /// answered" help offers its Bluetooth steps.
    #[test]
    fn bluetooth_only_meters_are_on_no_cable() {
        let bluetooth_only: Vec<&str> = registry::DEVICES
            .iter()
            .filter(|d| d.bluetooth_only)
            .map(|d| d.id)
            .collect();
        assert_eq!(bluetooth_only, ["ut60bt", "ut202bt"]);
        for kt in KNOWN_TRANSPORTS {
            assert!(
                !devices_on_bridge(kt.name).iter().any(|d| d.bluetooth_only),
                "{}",
                kt.name
            );
        }
        let on_bluetooth: Vec<&str> = devices_on_bridge(BLUETOOTH).iter().map(|d| d.id).collect();
        for id in bluetooth_only {
            assert!(on_bluetooth.contains(&id), "{id}");
        }
    }

    /// The CH9325 carries the UT80x family (UT803/UT804, UT71, VC9x0) and
    /// nothing else; the help it prints must not offer a UT61+ setup to a
    /// UT803 owner.
    #[test]
    fn ch9325_carries_exactly_the_ut80x_family() {
        let on_bridge: Vec<&str> = devices_on_bridge("CH9325").iter().map(|d| d.id).collect();
        let ut80x_family: Vec<&str> = registry::DEVICES
            .iter()
            .filter(|d| d.family == protocol::DeviceFamily::Ut80x)
            .map(|d| d.id)
            .collect();
        assert_eq!(on_bridge, ut80x_family);
    }

    /// A UT61B+ is verified over the CH9329 (issue #19), so the help for a
    /// silent CH9329 must offer the UT61+ setup — and detection must send
    /// Get Name there.
    #[test]
    fn ch9329_carries_the_ut61plus_family() {
        assert!(
            devices_on_bridge("CH9329")
                .iter()
                .any(|d| d.family == protocol::DeviceFamily::Ut61EPlus)
        );
    }

    /// The mock needs no cable, and offering it as a candidate on a bridge
    /// that answered nothing would send the user looking for a meter that
    /// does not exist.
    #[test]
    fn no_bridge_lists_the_mock() {
        for kt in KNOWN_TRANSPORTS {
            assert!(devices_on_bridge(kt.name).iter().all(|d| d.id != "mock"));
        }
    }

    /// The open path takes ids a binary already resolved, plus `auto`.
    #[test]
    fn selection_by_id_accepts_auto_and_exact_ids() {
        assert!(matches!(selection_by_id("auto"), Ok(Selection::Auto)));
        assert!(matches!(selection_by_id("AUTO"), Ok(Selection::Auto)));
        let Ok(Selection::Device(d)) = selection_by_id("ut8803") else {
            panic!("ut8803 must resolve");
        };
        assert_eq!(d.id, "ut8803");
        assert!(matches!(
            selection_by_id("nonexistent"),
            Err(Error::UnknownDevice(_))
        ));
    }

    /// Pull the hex value out of `ATTRS{idVendor}=="1a86"` in a udev rule.
    fn rule_attr(line: &str, name: &str) -> Option<u16> {
        let value = line.split_once(&format!("ATTRS{{{name}}}==\""))?.1;
        u16::from_str_radix(value.split_once('"')?.0, 16).ok()
    }

    /// The shipped udev rules and the transports must name the same devices.
    /// A missing rule leaves a plugged-in meter root-only on Linux, and a stale
    /// one hands out access to hardware nothing here opens any more.
    #[test]
    fn udev_rules_cover_exactly_the_known_transports() {
        // The crate is only ever tested from the repository, where the rules
        // file sits two levels up from crates/dmm-lib.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../udev/70-dmm-tools.rules");
        let rules = std::fs::read_to_string(path).expect("read udev/70-dmm-tools.rules");

        let mut in_rules: Vec<String> = rules
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let vid = rule_attr(line, "idVendor")?;
                let pid = rule_attr(line, "idProduct")?;
                Some(format!("{vid:04x}:{pid:04x}"))
            })
            .collect();
        let mut in_code: Vec<String> = KNOWN_TRANSPORTS
            .iter()
            .map(|kt| format!("{:04x}:{:04x}", kt.vid, kt.pid))
            .collect();

        in_rules.sort_unstable();
        in_code.sort_unstable();
        assert_eq!(
            in_rules, in_code,
            "udev/70-dmm-tools.rules and KNOWN_TRANSPORTS disagree on vid:pid"
        );
    }
}
