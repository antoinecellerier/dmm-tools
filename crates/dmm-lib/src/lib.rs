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
pub mod specs;
pub mod stats;
pub mod stream;
pub mod transform;
pub mod transport;
pub mod wall_clock;

pub use clock::Clock;
pub use wall_clock::WallClock;

use error::{Error, Result};
use log::{info, warn};
use protocol::Protocol;
use protocol::registry::{self, SelectableDevice, Selection};
use std::ffi::CString;
use transport::{Transport, ch9325, ch9329, cp2110};

/// Top-level handle for communicating with the multimeter.
pub struct Dmm<T: Transport> {
    transport: T,
    protocol: Box<dyn Protocol>,
    /// The session's time base. Real unless a mock session was opened with a
    /// clock of its own; see [`Dmm::clock`].
    clock: Clock,
}

impl<T: Transport> Dmm<T> {
    /// Create a new Dmm with the given transport and protocol.
    pub fn new(transport: T, mut protocol: Box<dyn Protocol>) -> Result<Self> {
        let profile = protocol.profile();
        info!(
            "connected to {} ({})",
            profile.model_name, profile.family_name
        );
        protocol.init(&transport)?;
        Ok(Self {
            transport,
            protocol,
            clock: Clock::real(),
        })
    }

    /// Stamp this session's readings with `clock` instead of wall time.
    ///
    /// `pub(crate)` because the only sessions that run on a non-real clock are
    /// mock ones, opened through [`mock::open_mock_clocked`].
    pub(crate) fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
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
        m.spec = self.protocol.spec_info(m.mode_raw, m.range_raw);
        m.mode_spec = self.protocol.mode_spec_info(m.mode_raw);
        // Session time is stamped here and nowhere else: the parsers set
        // `Instant::now()` when they build the measurement, which is the same
        // thing on a real clock and wrong on any other.
        m.timestamp = self.clock.now();
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

    /// Request the device name from the meter.
    pub fn get_name(&mut self) -> Result<Option<String>> {
        self.protocol.get_name(&self.transport)
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

/// The USB cable(s) a device family is known to ship with, most likely first.
///
/// Sourced from the cable table in `docs/supported-devices.md`: the CP2110
/// UT-D09 covers UT61x+/UT161x/UT171x/UT880x and the Voltcraft meters, the
/// CH9329 UT-D09 variant is sold for the UT181A and UT171 series, and the
/// CH9325 UT-D04 is what the UT803/UT804 use.
///
/// This only orders the candidates — [`open_first_match`] still falls back to
/// the remaining transports, so an unusual cable keeps working. Without it,
/// selecting a UT803 on a bench that also has a UT61E+ attached opens the
/// UT61E+'s CP2110 and every read times out.
fn preferred_transports(family: protocol::DeviceFamily) -> &'static [&'static str] {
    use protocol::DeviceFamily as F;
    match family {
        F::Ut61EPlus | F::Ut8802 | F::Ut8803 | F::Vc880 | F::Vc890 => &["CP2110"],
        F::Fs9721 => &["CH9325"],
        F::Ut181a => &["CH9329"],
        F::Ut171 => &["CP2110", "CH9329"],
        F::Mock => &[],
    }
}

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
            let ch = ch9325::Ch9325::new(dev);
            ch.init()?;
            Ok(Box::new(ch))
        },
    },
];

/// Open a device by registry ID, automatically selecting the transport.
///
/// Tries transports in order (CP2110, CH9329, CH9325).
/// Returns a type-erased `Dmm<Box<dyn Transport>>` suitable for both CLI and GUI.
///
/// `id` may be [`registry::AUTO_DEVICE_ID`], in which case the meter is
/// identified from the bytes it sends; a caller that wants to know which one
/// it was calls [`open_auto`] instead.
///
/// When `adapter` is `Some`, selects a specific USB adapter by serial number
/// or HID device path (as shown by [`list_devices`]). When `None`, picks the
/// first matching adapter (and logs a warning if multiple are found).
pub fn open_device_by_id_auto(id: &str, adapter: Option<&str>) -> Result<Dmm<Box<dyn Transport>>> {
    let (transport, protocol) = open_transport_by_id_auto(id, adapter)?;
    Dmm::new(transport, protocol)
}

/// Open the transport and build the protocol without running `Protocol::init`.
///
/// Lets a caller wrap the transport — recording wire bytes, say — before the
/// init handshake runs, so those bytes are observable too. Pass the pair to
/// [`Dmm::new`] to finish opening the device.
///
/// With [`registry::AUTO_DEVICE_ID`] the detection probe runs here, before the
/// transport is handed back, so its bytes go out unwrapped. A caller that must
/// see them wraps the transport itself: [`open_transport`] then
/// [`detect::detect_device`].
pub fn open_transport_by_id_auto(
    id: &str,
    adapter: Option<&str>,
) -> Result<(Box<dyn Transport>, Box<dyn Protocol>)> {
    match selection_by_id(id)? {
        Selection::Auto => {
            let (transport, detected) = open_detected(adapter)?;
            Ok((transport, (detected.device.new_protocol)()))
        }
        Selection::Device(entry) => {
            let (transport, _bridge) = open_transport(preferred_transports(entry.family), adapter)?;
            Ok((transport, (entry.new_protocol)()))
        }
    }
}

/// Open the meter on the cable without being told which one it is.
///
/// Opens the first bridge found, identifies the meter on it
/// ([`detect::detect_device`]) and opens it with the entry that came back.
/// The [`detect::Detected`] is returned as well, so a caller can tell the user
/// which meter was picked and what name it reported.
pub fn open_auto(adapter: Option<&str>) -> Result<(Dmm<Box<dyn Transport>>, detect::Detected)> {
    let (transport, detected) = open_detected(adapter)?;
    let protocol = (detected.device.new_protocol)();
    Ok((Dmm::new(transport, protocol)?, detected))
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
fn open_detected(adapter: Option<&str>) -> Result<(Box<dyn Transport>, detect::Detected)> {
    // No family, so no cable to prefer: whichever bridge answers first is the
    // one the meter is probed through.
    let (transport, bridge) = open_transport(&[], adapter)?;
    let detected = detect::detect_device(&*transport, bridge)?;
    Ok((transport, detected))
}

/// Open a USB bridge and hand back the transport alone, with no protocol.
///
/// `preferred` orders the cables to try — [`preferred_transports`] for a known
/// family, empty when the meter has not been identified yet. The returned name
/// is the bridge's (`"CP2110"`, `"CH9329"`, `"CH9325"`), which
/// [`detect::detect_device`] needs to know which probes are worth sending.
///
/// This is the split half of [`open_transport_by_id_auto`]: a caller that
/// wants to wrap the transport before *any* byte flows — recording the
/// detection probe itself, say — opens it here and detects separately.
pub fn open_transport(
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
/// [`preferred_transports`].
///
/// What the "no meter answered" help lists: with nothing identified on a
/// bridge, these are the meters that could have been on it, and their
/// activation instructions are what the user has to act on.
pub fn devices_on_bridge(bridge: &str) -> Vec<&'static SelectableDevice> {
    registry::DEVICES
        .iter()
        .filter(|d| d.requires_hardware)
        .filter(|d| preferred_transports(d.family).contains(&bridge))
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
            "Multiple USB adapters found ({match_count} devices).{preference} \
             Specify an adapter to select a specific device."
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

    Err(Error::NoTransportFound)
}

/// List all connected USB adapters (CP2110, CH9329, CH9325).
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
        });
    }

    Ok(devices)
}

/// Information about a connected USB adapter.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub product: Option<String>,
    pub serial: Option<String>,
    /// Transport type: "CP2110", "CH9329", or "CH9325".
    pub transport: &'static str,
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

    #[test]
    fn device_info_display() {
        let info = DeviceInfo {
            path: "/dev/hidraw0".to_string(),
            product: Some("UT61E+".to_string()),
            serial: Some("12345".to_string()),
            transport: "CP2110",
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
            for name in preferred_transports(device.family) {
                assert!(
                    KNOWN_TRANSPORTS.iter().any(|kt| kt.name == *name),
                    "device {} prefers unknown transport {name:?}",
                    device.id,
                );
            }
        }
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
                !preferred_transports(device.family).is_empty(),
                "device {} has no preferred transport",
                device.id,
            );
        }
    }

    /// The preferred cable must come first, but the others stay reachable so an
    /// unusual pairing still connects.
    #[test]
    fn preference_orders_without_excluding() {
        let preferred = preferred_transports(protocol::DeviceFamily::Fs9721);
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

        assert_eq!(ordered[0], "CH9325", "UT803/UT804 use the CH9325 UT-D04");
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
        assert!(devices_on_bridge("no such bridge").is_empty());
    }

    /// The CH9325 UT-D04 is the FS9721 meters' cable and nothing else's; the
    /// help it prints must not offer a UT61+ setup to a UT803 owner.
    #[test]
    fn ch9325_carries_exactly_the_fs9721_meters() {
        let on_bridge: Vec<&str> = devices_on_bridge("CH9325").iter().map(|d| d.id).collect();
        let fs9721: Vec<&str> = registry::DEVICES
            .iter()
            .filter(|d| d.family == protocol::DeviceFamily::Fs9721)
            .map(|d| d.id)
            .collect();
        assert_eq!(on_bridge, fs9721);
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
