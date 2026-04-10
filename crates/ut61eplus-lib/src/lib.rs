pub mod ch9325;
pub mod ch9329;
pub mod cp2110;
pub mod error;
pub mod flags;
pub mod measurement;
pub mod mock;
pub mod protocol;
pub mod stats;
pub mod transport;

use error::{Error, Result};
use log::{info, warn};
use protocol::Protocol;
use std::ffi::CString;
use transport::Transport;

/// Top-level handle for communicating with the multimeter.
pub struct Dmm<T: Transport> {
    transport: T,
    protocol: Box<dyn Protocol>,
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
        })
    }

    /// Access the underlying transport (e.g. for CP2110-specific queries).
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Request a single measurement from the meter.
    pub fn request_measurement(&mut self) -> Result<measurement::Measurement> {
        self.protocol.request_measurement(&self.transport)
    }

    /// Send a named command to the meter (e.g. "hold", "range", "auto").
    pub fn send_command(&mut self, command: &str) -> Result<()> {
        self.protocol.send_command(&self.transport, command)
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
/// When `adapter` is `Some`, selects a specific USB adapter by serial number
/// or HID device path (as shown by [`list_devices`]). When `None`, picks the
/// first matching adapter (and logs a warning if multiple are found).
pub fn open_device_by_id_auto(id: &str, adapter: Option<&str>) -> Result<Dmm<Box<dyn Transport>>> {
    let entry =
        protocol::registry::find_device(id).ok_or_else(|| Error::UnknownDevice(id.to_string()))?;

    let api = hidapi::HidApi::new().map_err(Error::Hid)?;

    match adapter {
        Some(adapter) => open_with_adapter(&api, adapter),
        None => open_first_match(&api),
    }
    .map(|(device, kt)| {
        info!(
            "found {} adapter (VID={:#06x} PID={:#06x})",
            kt.name, kt.vid, kt.pid
        );
        ((kt.init)(device), (entry.new_protocol)())
    })
    .and_then(|(transport, protocol)| Dmm::new(transport?, protocol))
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

/// Open the first matching adapter across all known transports.
/// Warns if multiple adapters are found.
fn open_first_match(api: &hidapi::HidApi) -> Result<(hidapi::HidDevice, &'static KnownTransport)> {
    let match_count: usize = api
        .device_list()
        .filter(|dev| {
            KNOWN_TRANSPORTS
                .iter()
                .any(|kt| dev.vendor_id() == kt.vid && dev.product_id() == kt.pid)
        })
        .count();

    if match_count > 1 {
        warn!(
            "Multiple USB adapters found ({match_count} devices). \
             Using first match. Specify an adapter to select a specific device."
        );
    }

    for kt in KNOWN_TRANSPORTS {
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
        // Length byte = payload + 2 checksum bytes (matches real wire format)
        let len_byte = (payload.len() + 2) as u8;
        let mut frame = vec![0xAB, 0xCD, len_byte];
        frame.extend_from_slice(&payload);
        let sum: u16 = frame.iter().map(|&b| b as u16).sum();
        frame.push((sum >> 8) as u8);
        frame.push((sum & 0xFF) as u8);
        frame
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
}
