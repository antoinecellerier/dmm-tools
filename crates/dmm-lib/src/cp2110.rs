use crate::error::{Error, Result};
use crate::transport::Transport;
use hidapi::HidDevice;
use log::{debug, trace};

/// Silicon Labs CP2110 VID.
pub const VID: u16 = 0x10C4;
/// UT61E+ PID (CP2110 HID-to-UART bridge).
pub const PID: u16 = 0xEA80;

/// Maximum payload size for a single CP2110 HID interrupt report (AN434 §6.1).
const MAX_REPORT_PAYLOAD: usize = 63;

/// CP2110 UART status from report 0x42 (AN434 §5.3).
#[derive(Debug, Clone)]
pub struct UartStatus {
    /// Bytes waiting in the transmit FIFO (max 480).
    pub tx_fifo: u16,
    /// Bytes waiting in the receive FIFO (max 480).
    pub rx_fifo: u16,
    /// Parity error detected since last status read.
    pub parity_error: bool,
    /// Overrun error detected since last status read.
    pub overrun_error: bool,
    /// Line break is currently active.
    pub line_break: bool,
}

/// CP2110 version information from report 0x46 (AN434 §5.7).
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// Part number (0x0A for CP2110).
    pub part_number: u8,
    /// Device firmware version.
    pub device_version: u8,
}

/// CP2110 HID transport wrapping a `HidDevice`.
pub struct Cp2110 {
    device: HidDevice,
}

impl Cp2110 {
    /// Wrap an already-opened HID device.
    pub fn new(device: HidDevice) -> Self {
        Self { device }
    }

    /// Send the three feature reports to initialize the UART bridge:
    /// 1. Enable UART (report 0x41)
    /// 2. Configure 9600 baud, 8N1 (report 0x50)
    /// 3. Purge receive FIFO (report 0x43)
    pub fn init_uart(&self) -> Result<()> {
        debug!("CP2110: enabling UART");
        self.send_feature_report(&[0x41, 0x01])?;

        // Report 0x50 (AN434 §6.3): baud rate (4 bytes BE) + parity + flow ctl + data bits + stop bits
        debug!("CP2110: configuring 9600/8N1");
        self.send_feature_report(&[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00])?;

        // 0x02 = purge receive FIFO only (TX is empty at init time)
        debug!("CP2110: purging RX FIFO");
        self.send_feature_report(&[0x43, 0x02])?;

        Ok(())
    }

    /// Get the product string from the HID device.
    pub fn product_string(&self) -> Result<Option<String>> {
        Ok(self.device.get_product_string()?)
    }

    /// Get the HID device path.
    pub fn path(&self) -> String {
        // HidDevice doesn't expose path after open, return placeholder
        String::from("<connected>")
    }

    /// Query CP2110 version information (report 0x46, AN434 §5.7).
    ///
    /// Returns the part number (0x0A for CP2110) and device firmware version.
    pub fn version_info(&self) -> Result<VersionInfo> {
        let mut buf = [0u8; 3];
        buf[0] = 0x46; // Get Version Information report ID
        let n = self
            .device
            .get_feature_report(&mut buf)
            .map_err(Error::Hid)?;
        if n < 3 {
            return Err(Error::invalid_response_msg(format!(
                "version info report too short: {n} bytes"
            )));
        }
        let info = VersionInfo {
            part_number: buf[1],
            device_version: buf[2],
        };
        debug!(
            "CP2110: part={:#04x} version={}",
            info.part_number, info.device_version
        );
        Ok(info)
    }

    /// Query CP2110 UART status (report 0x42, AN434 §5.3).
    ///
    /// Returns FIFO levels, error flags, and line break status.
    /// Note: reading this report clears the error flags.
    pub fn uart_status(&self) -> Result<UartStatus> {
        let mut buf = [0u8; 7];
        buf[0] = 0x42; // Get UART Status report ID
        let n = self
            .device
            .get_feature_report(&mut buf)
            .map_err(Error::Hid)?;
        if n < 7 {
            return Err(Error::invalid_response_msg(format!(
                "UART status report too short: {n} bytes"
            )));
        }
        // TX/RX FIFO counts are 16-bit little-endian (AN434 §3.2 default byte order)
        // Note: SLABHIDtoUART.dll decompilation suggests big-endian (CONCAT11 pattern),
        // but we cannot verify — the CP2110 drains the FIFO to HID in real time, so
        // counts are always 0 in normal operation.
        let tx_fifo = u16::from_le_bytes([buf[1], buf[2]]);
        let rx_fifo = u16::from_le_bytes([buf[3], buf[4]]);
        // Error Status: bit 0 = parity error, bit 1 = overrun error
        let error_status = buf[5];
        Ok(UartStatus {
            tx_fifo,
            rx_fifo,
            parity_error: error_status & 0x01 != 0,
            overrun_error: error_status & 0x02 != 0,
            line_break: buf[6] != 0,
        })
    }

    /// Reset the CP2110 device (report 0x40, AN434 §5.1).
    ///
    /// The device will re-enumerate on the USB bus and all UART configuration
    /// is reset to defaults. The HID handle becomes invalid after this call —
    /// the caller must re-open and re-initialize.
    ///
    /// **Note:** The UT61E+'s CP2110 rejects this report (HID protocol error),
    /// likely because UNI-T locked it out in the device's HID descriptor.
    /// This method is provided for completeness but will return `Err` on
    /// UT61E+ hardware.
    pub fn reset(&self) -> Result<()> {
        debug!("CP2110: resetting device");
        self.device
            .send_feature_report(&[0x40, 0x00])
            .map_err(Error::Hid)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vid_pid_constants() {
        assert_eq!(VID, 0x10C4, "Silicon Labs VID");
        assert_eq!(PID, 0xEA80, "CP2110 PID for UT61E+");
    }

    #[test]
    fn max_report_payload_matches_an434() {
        // AN434 §6.1: max interrupt report payload is 63 bytes
        assert_eq!(MAX_REPORT_PAYLOAD, 63);
    }

    #[test]
    fn uart_status_struct() {
        let status = UartStatus {
            tx_fifo: 100,
            rx_fifo: 200,
            parity_error: true,
            overrun_error: false,
            line_break: true,
        };
        assert_eq!(status.tx_fifo, 100);
        assert_eq!(status.rx_fifo, 200);
        assert!(status.parity_error);
        assert!(!status.overrun_error);
        assert!(status.line_break);
    }

    #[test]
    fn version_info_struct() {
        let info = VersionInfo {
            part_number: 0x0A,
            device_version: 5,
        };
        assert_eq!(info.part_number, 0x0A);
        assert_eq!(info.device_version, 5);
    }

    #[test]
    fn uart_status_debug_format() {
        // Ensure Debug derive works for diagnostic output
        let status = UartStatus {
            tx_fifo: 0,
            rx_fifo: 0,
            parity_error: false,
            overrun_error: false,
            line_break: false,
        };
        let debug = format!("{status:?}");
        assert!(debug.contains("UartStatus"));
        assert!(debug.contains("tx_fifo"));
    }

    #[test]
    fn version_info_clone() {
        let info = VersionInfo {
            part_number: 0x0A,
            device_version: 3,
        };
        let cloned = info.clone();
        assert_eq!(cloned.part_number, info.part_number);
        assert_eq!(cloned.device_version, info.device_version);
    }
}

impl Transport for Cp2110 {
    fn write(&self, data: &[u8]) -> Result<()> {
        // CP2110 interrupt OUT: first byte is length, then payload (AN434 §6.1).
        if data.len() > MAX_REPORT_PAYLOAD {
            return Err(Error::invalid_response_msg(format!(
                "data too large for single HID report: {} bytes (max {MAX_REPORT_PAYLOAD})",
                data.len()
            )));
        }
        let mut report = Vec::with_capacity(data.len() + 1);
        report.push(data.len() as u8);
        report.extend_from_slice(data);
        trace!("CP2110 TX: {:02X?}", report);
        self.device.write(&report)?;
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        let mut raw = vec![0u8; buf.len() + 1];
        let n = self.device.read_timeout(&mut raw, timeout_ms)?;
        if n == 0 {
            return Ok(0);
        }
        // CP2110 interrupt IN: first byte is length, rest is payload
        let payload_len = raw[0] as usize;
        let actual = payload_len.min(n - 1).min(buf.len());
        buf[..actual].copy_from_slice(&raw[1..1 + actual]);
        trace!("CP2110 RX ({actual} bytes): {:02X?}", &buf[..actual]);
        Ok(actual)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        trace!("CP2110 feature report: {:02X?}", data);
        self.device.send_feature_report(data).map_err(Error::Hid)?;
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        let ver = self.version_info()?;
        Ok(format!(
            "CP2110 part={:#04x} firmware={}",
            ver.part_number, ver.device_version
        ))
    }

    fn transport_status(&self) -> Result<String> {
        let st = self.uart_status()?;
        Ok(format!(
            "TX FIFO: {} bytes, RX FIFO: {} bytes, parity_err={}, overrun_err={}, line_break={}",
            st.tx_fifo, st.rx_fifo, st.parity_error, st.overrun_error, st.line_break
        ))
    }

    fn transport_name(&self) -> &'static str {
        "CP2110"
    }
}
