pub mod command;
pub mod cp2110;
pub mod error;
pub mod flags;
pub mod measurement;
pub mod mode;
pub(crate) mod protocol;
pub mod tables;
pub mod transport;

use command::Command;
use error::{Error, Result};
use log::{debug, info};
use measurement::Measurement;
use tables::DeviceTable;
use transport::Transport;

/// Top-level handle for communicating with the multimeter.
pub struct Dmm<T: Transport> {
    transport: T,
    table: Box<dyn DeviceTable>,
    rx_buf: Vec<u8>,
}

impl<T: Transport> Dmm<T> {
    /// Create a new Dmm with the given transport and device table.
    pub fn new(transport: T, table: Box<dyn DeviceTable>) -> Self {
        info!("connected to {}", table.model_name());
        Self {
            transport,
            table,
            rx_buf: Vec::with_capacity(64),
        }
    }

    /// Request a single measurement from the meter.
    pub fn request_measurement(&mut self) -> Result<Measurement> {
        let cmd = Command::GetMeasurement.encode();
        debug!("sending measurement request");
        self.transport.write(&cmd)?;
        self.read_response()
    }

    /// Send a button-press command to the meter.
    ///
    /// After sending, drains any response/ack from the meter to avoid
    /// stale bytes confusing the next measurement request.
    pub fn send_command(&mut self, command: Command) -> Result<()> {
        let cmd = command.encode();
        debug!("sending command: {command:?}");
        self.transport.write(&cmd)?;

        // Drain any ack/response the meter sends back.
        // Use short timeout and bounded iterations to avoid discarding
        // a measurement response that arrives immediately after the ack.
        self.rx_buf.clear();
        let mut tmp = [0u8; 64];
        for _ in 0..3 {
            let n = self.transport.read_timeout(&mut tmp, 50)?;
            if n == 0 {
                break;
            }
            debug!("drained {} bytes after command", n);
        }

        Ok(())
    }

    /// Request the device name from the meter.
    ///
    /// Returns the name string (e.g., "UT61E+"). The meter responds with
    /// two frames: an acknowledgment (payload `FF 00`) and the name.
    pub fn get_name(&mut self) -> Result<String> {
        let cmd = Command::GetName.encode();
        debug!("sending get_name request");
        self.transport.write(&cmd)?;

        // Read two frames: ack + name
        for _ in 0..2 {
            let payload = self.read_raw_payload()?;
            // The name frame has ASCII payload (not the FF 00 ack)
            if payload.first() != Some(&0xFF) {
                let name = String::from_utf8_lossy(&payload).to_string();
                debug!("device name: {name}");
                return Ok(name);
            }
        }

        Err(Error::InvalidResponse("no name frame received".to_string()))
    }

    /// Read a raw payload frame (used for non-measurement responses).
    fn read_raw_payload(&mut self) -> Result<Vec<u8>> {
        const READ_TIMEOUT_MS: i32 = 2000;
        const MAX_ATTEMPTS: usize = 64;

        for _ in 0..MAX_ATTEMPTS {
            match protocol::extract_frame(&self.rx_buf)? {
                Some((payload, consumed)) => {
                    self.rx_buf.drain(..consumed);
                    return Ok(payload);
                }
                None => {
                    let mut tmp = [0u8; 64];
                    let n = self.transport.read_timeout(&mut tmp, READ_TIMEOUT_MS)?;
                    if n == 0 {
                        return Err(Error::Timeout);
                    }
                    self.rx_buf.extend_from_slice(&tmp[..n]);
                }
            }
        }

        Err(Error::Timeout)
    }

    /// Read and parse a measurement response.
    fn read_response(&mut self) -> Result<Measurement> {
        let payload = self.read_raw_payload()?;
        Measurement::parse(&payload, self.table.as_ref())
    }
}

/// Open the first UT61E+ device found via hidapi and return an initialized Dmm.
pub fn open() -> Result<Dmm<cp2110::Cp2110>> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;
    let device = api
        .open(cp2110::VID, cp2110::PID)
        .map_err(|_| Error::DeviceNotFound {
            vid: cp2110::VID,
            pid: cp2110::PID,
        })?;

    let cp = cp2110::Cp2110::new(device);
    cp.init_uart()?;

    let table = Box::new(tables::ut61e_plus::Ut61ePlusTable::new());
    Ok(Dmm::new(cp, table))
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
    use crate::tables::ut61e_plus::Ut61ePlusTable;
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
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, mode::Mode::DcV);
        assert_eq!(m.unit, "V");
        assert!(m.flags.auto_range);
    }

    #[test]
    fn dmm_split_response() {
        // Response arrives in two chunks
        let full =
            make_measurement_response(0x06, 0x02, b" 12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let (part1, part2) = full.split_at(10);
        let mock = MockTransport::new(vec![part1.to_vec(), part2.to_vec()]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, mode::Mode::Ohm);
        assert_eq!(m.range_label, "22kΩ");
    }

    #[test]
    fn dmm_timeout() {
        // Empty responses → timeout
        let mock = MockTransport::new(vec![]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let result = dmm.request_measurement();
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[test]
    fn dmm_sends_correct_request_bytes() {
        let response =
            make_measurement_response(0x02, 0x00, b" 0.0000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let _ = dmm.request_measurement().unwrap();

        let written = dmm.transport.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9]);
    }

    #[test]
    fn dmm_send_command() {
        let mock = MockTransport::new(vec![]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        dmm.send_command(Command::Hold).unwrap();

        let written = dmm.transport.written.borrow();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], Command::Hold.encode());
    }

    #[test]
    fn dmm_response_with_leading_garbage() {
        let mut data = vec![0xFF, 0xFE, 0x00]; // garbage before frame
        data.extend_from_slice(&make_measurement_response(
            0x00,
            0x00,
            b"  1.234",
            (0x00, 0x00),
            (0x00, 0x00, 0x00),
        ));
        let mock = MockTransport::new(vec![data]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, mode::Mode::AcV);
    }

    #[test]
    fn dmm_multiple_measurements() {
        let r1 =
            make_measurement_response(0x02, 0x00, b"  1.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let r2 =
            make_measurement_response(0x02, 0x00, b"  2.000", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![r1, r2]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

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
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, mode::Mode::Capacitance);
        assert_eq!(m.unit, "µF");
        assert_eq!(m.range_label, "22µF");
    }

    #[test]
    fn dmm_hz_mode() {
        let response =
            make_measurement_response(0x04, 0x02, b" 1.2345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert_eq!(m.mode, mode::Mode::Hz);
        assert_eq!(m.unit, "kHz");
        assert_eq!(m.range_label, "2.2kHz");
    }

    #[test]
    fn dmm_negative_value() {
        let response =
            make_measurement_response(0x02, 0x01, b"-12.345", (0x00, 0x00), (0x00, 0x00, 0x00));
        let mock = MockTransport::new(vec![response]);
        let table = Box::new(Ut61ePlusTable::new());
        let mut dmm = Dmm::new(mock, table);

        let m = dmm.request_measurement().unwrap();
        assert!(
            matches!(m.value, measurement::MeasuredValue::Normal(v) if (v - (-12.345)).abs() < 1e-6)
        );
    }
}
