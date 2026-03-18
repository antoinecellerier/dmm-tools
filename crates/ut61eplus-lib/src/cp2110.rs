use crate::error::{Error, Result};
use crate::transport::Transport;
use hidapi::HidDevice;
use log::{debug, trace};

/// Silicon Labs CP2110 VID.
pub const VID: u16 = 0x10C4;
/// UT61E+ PID (CP2110 HID-to-UART bridge).
pub const PID: u16 = 0xEA80;

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
    /// 1. Enable UART
    /// 2. Configure 9600 baud, 8N1
    /// 3. Purge FIFOs
    pub fn init_uart(&self) -> Result<()> {
        debug!("CP2110: enabling UART");
        self.send_feature_report(&[0x41, 0x01])?;

        debug!("CP2110: configuring 9600/8N1");
        self.send_feature_report(&[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00, 0x00])?;

        debug!("CP2110: purging FIFOs");
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
}

impl Transport for Cp2110 {
    fn write(&self, data: &[u8]) -> Result<()> {
        // CP2110 interrupt OUT: first byte is length, then payload.
        // Max payload for a single HID interrupt report is 63 bytes.
        debug_assert!(data.len() <= 63, "data too large for single HID report: {}", data.len());
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
        self.device
            .send_feature_report(data)
            .map_err(Error::Hid)?;
        Ok(())
    }
}
