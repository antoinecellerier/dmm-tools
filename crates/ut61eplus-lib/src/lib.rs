pub mod cp2110;
pub mod error;
pub mod flags;
pub mod measurement;
pub mod mock;
pub mod protocol;
pub mod transport;

use error::{Error, Result};
use log::info;
use protocol::{DeviceFamily, Protocol};
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

/// Open the first UT61E+ device found via hidapi and return an initialized Dmm.
pub fn open() -> Result<Dmm<cp2110::Cp2110>> {
    open_device(DeviceFamily::Ut61EPlus)
}

/// Open a device with the specified protocol family.
///
/// The `Mock` family is not supported here — use [`mock::open_mock()`] instead.
/// Passing `DeviceFamily::Mock` will panic; callers must route mock before calling this.
pub fn open_device(family: DeviceFamily) -> Result<Dmm<cp2110::Cp2110>> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;
    let device = api
        .open(cp2110::VID, cp2110::PID)
        .map_err(|_| Error::DeviceNotFound {
            vid: cp2110::VID,
            pid: cp2110::PID,
        })?;

    let cp = cp2110::Cp2110::new(device);
    cp.init_uart()?;

    let protocol: Box<dyn Protocol> = match family {
        DeviceFamily::Ut61EPlus => Box::new(protocol::ut61eplus::Ut61PlusProtocol::new()),
        DeviceFamily::Ut8803 => Box::new(protocol::ut8803::Ut8803Protocol::new()),
        DeviceFamily::Ut171 => Box::new(protocol::ut171::Ut171Protocol::new()),
        DeviceFamily::Ut181a => Box::new(protocol::ut181a::Ut181aProtocol::new()),
        DeviceFamily::Mock => unreachable!("handled above"),
    };

    Dmm::new(cp, protocol)
}

/// List all connected CP2110 devices.
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;
    let mut devices = Vec::new();

    for dev in api.device_list() {
        if dev.vendor_id() == cp2110::VID && dev.product_id() == cp2110::PID {
            devices.push(DeviceInfo {
                path: dev.path().to_string_lossy().into_owned(),
                product: dev.product_string().map(|s| s.to_string()),
                serial: dev.serial_number().map(|s| s.to_string()),
            });
        }
    }

    Ok(devices)
}

/// Information about a connected device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub product: Option<String>,
    pub serial: Option<String>,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)?;
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
        };
        let s = info.to_string();
        assert!(s.contains("/dev/hidraw0"));
        assert!(s.contains("UT61E+"));
        assert!(s.contains("12345"));
    }

    #[test]
    fn device_info_display_no_optional_fields() {
        let info = DeviceInfo {
            path: "/dev/hidraw0".to_string(),
            product: None,
            serial: None,
        };
        assert_eq!(info.to_string(), "/dev/hidraw0");
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
