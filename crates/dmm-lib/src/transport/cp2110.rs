use crate::error::{Error, Result};
use crate::transport::{Link, Transport, open};
use hidapi::HidDevice;
use log::{debug, trace};

/// Silicon Labs CP2110 VID.
pub(crate) const VID: u16 = 0x10C4;
/// The CP2110's stock PID, which the cables of the UT61E+ and several other
/// meters keep.
pub(crate) const PID: u16 = 0xEA80;
/// The bridge's name in a registry entry's links, in `dmm-cli list` and in
/// detection.
pub(crate) const NAME: &str = "CP2110";

/// Maximum payload size for a single CP2110 HID interrupt report (AN434 §6.1).
const MAX_REPORT_PAYLOAD: usize = 63;

/// CP2110 UART status from report 0x42 (AN434 §5.3).
#[derive(Debug, Clone)]
pub(crate) struct UartStatus {
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
pub(crate) struct VersionInfo {
    /// Part number (0x0A for CP2110).
    pub part_number: u8,
    /// Device firmware version.
    pub device_version: u8,
}

/// The interrupt OUT report for `data`: a length byte, then the payload
/// (AN434 §6.1). More than one report holds is refused unsent.
fn output_report(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() > MAX_REPORT_PAYLOAD {
        return Err(Error::invalid_response_msg(format!(
            "data too large for single HID report: {} bytes (max {MAX_REPORT_PAYLOAD})",
            data.len()
        )));
    }
    let mut report = Vec::with_capacity(data.len() + 1);
    report.push(data.len() as u8);
    report.extend_from_slice(data);
    Ok(report)
}

/// The UART payload of one interrupt IN report, `n` bytes of `raw`: a length
/// byte, then the payload (AN434 §6.1). A length byte claiming more than
/// arrived yields only the bytes that are there.
fn rx_payload(raw: &[u8], n: usize) -> &[u8] {
    match &raw[..n.min(raw.len())] {
        [len, payload @ ..] => &payload[..(*len as usize).min(payload.len())],
        [] => &[],
    }
}

/// The fields of a UART status report 0x42 (AN434 §5.3).
fn parse_uart_status(buf: &[u8; 7]) -> UartStatus {
    // TX/RX FIFO counts read as 16-bit little-endian (AN434 §3.2 default byte
    // order). UNI-T's software reads them big-endian (UT171 spec §8);
    // idle reads give 0, which fits either, so which is right is an open check
    // (docs/verification-backlog.md, "USB bridges").
    let tx_fifo = u16::from_le_bytes([buf[1], buf[2]]);
    let rx_fifo = u16::from_le_bytes([buf[3], buf[4]]);
    // Error Status: bit 0 = parity error, bit 1 = overrun error
    let error_status = buf[5];
    UartStatus {
        tx_fifo,
        rx_fifo,
        parity_error: error_status & 0x01 != 0,
        overrun_error: error_status & 0x02 != 0,
        line_break: buf[6] != 0,
    }
}

/// CP2110 HID transport wrapping a `HidDevice`.
pub(crate) struct Cp2110 {
    device: HidDevice,
    /// What [`Transport::cable_serial`] reports, read while opening.
    serial: Option<String>,
}

impl Cp2110 {
    /// Take an already-opened HID device and set up its UART with three
    /// feature reports:
    /// 1. Enable UART (report 0x41)
    /// 2. Configure 9600 baud, 8N1 (report 0x50)
    /// 3. Purge receive FIFO (report 0x43)
    pub(crate) fn open(device: HidDevice) -> Result<Self> {
        let cp = Self {
            serial: open::cable_serial(&device),
            device,
        };
        debug!("{NAME}: enabling UART");
        cp.send_feature_report(&[0x41, 0x01])?;

        // Report 0x50 (AN434 §6.3): baud rate (4 bytes BE) + parity + flow ctl + data bits + stop bits
        debug!("{NAME}: configuring 9600/8N1");
        cp.send_feature_report(&[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00])?;

        // 0x02 = purge receive FIFO only (TX is empty at init time)
        debug!("{NAME}: purging RX FIFO");
        cp.send_feature_report(&[0x43, 0x02])?;

        Ok(cp)
    }

    /// Send a HID feature report to the bridge.
    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        trace!("{NAME} feature report: {:02X?}", data);
        self.device.send_feature_report(data).map_err(Error::Hid)?;
        Ok(())
    }

    /// Query CP2110 version information (report 0x46, AN434 §5.7).
    ///
    /// Returns the part number (0x0A for CP2110) and device firmware version.
    pub(crate) fn version_info(&self) -> Result<VersionInfo> {
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
            "{NAME}: part={:#04x} version={}",
            info.part_number, info.device_version
        );
        Ok(info)
    }

    /// Query CP2110 UART status (report 0x42, AN434 §5.3).
    ///
    /// Returns FIFO levels, error flags, and line break status.
    /// Note: reading this report clears the error flags.
    pub(crate) fn uart_status(&self) -> Result<UartStatus> {
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
        Ok(parse_uart_status(&buf))
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
    ///
    /// This function is not called during normal operation; kept as a
    /// diagnostic for a bridge left in a bad state by an interrupted session.
    #[expect(
        dead_code,
        reason = "a diagnostic for a bridge an interrupted session left stuck"
    )]
    pub(crate) fn reset(&self) -> Result<()> {
        debug!("{NAME}: resetting device");
        self.device
            .send_feature_report(&[0x40, 0x00])
            .map_err(Error::Hid)?;
        Ok(())
    }
}

impl Transport for Cp2110 {
    fn write(&self, data: &[u8]) -> Result<()> {
        let report = output_report(data)?;
        trace!("{NAME} TX: {:02X?}", report);
        self.device.write(&report)?;
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        // Stack, not heap: this is the measurement hot path.
        // `framing::read_uart_bytes` can call it up to MAX_EMPTY_READS (256)
        // times per frame while polling an idle bridge, and `read_frame`
        // reads again until a frame completes, so a single measurement was
        // allocating and freeing thousands of short-lived buffers.
        //
        // One interrupt IN report is a length byte plus at most
        // MAX_REPORT_PAYLOAD bytes, so this is the largest read that can
        // land — the previous `buf.len() + 1` heap buffer was sized off the
        // caller's slice, which is always the fixed 64-byte one in
        // `read_uart_bytes` and never needed the extra byte.
        let mut raw = [0u8; MAX_REPORT_PAYLOAD + 1];
        let n = self.device.read_timeout(&mut raw, timeout_ms)?;
        if n == 0 {
            return Ok(0);
        }
        let payload = rx_payload(&raw, n);
        let actual = payload.len().min(buf.len());
        buf[..actual].copy_from_slice(&payload[..actual]);
        trace!("{NAME} RX ({actual} bytes): {:02X?}", &buf[..actual]);
        Ok(actual)
    }

    fn transport_info(&self) -> Result<Option<String>> {
        let ver = self.version_info()?;
        Ok(Some(format!(
            "{NAME} part={:#04x} firmware={}",
            ver.part_number, ver.device_version
        )))
    }

    fn transport_status(&self) -> Result<Option<String>> {
        let st = self.uart_status()?;
        Ok(Some(format!(
            "TX FIFO: {} bytes, RX FIFO: {} bytes, parity_err={}, overrun_err={}, line_break={}",
            st.tx_fifo, st.rx_fifo, st.parity_error, st.overrun_error, st.line_break
        )))
    }

    fn transport_name(&self) -> &'static str {
        NAME
    }

    fn link(&self) -> Option<Link> {
        Some(Link::UsbCable)
    }

    fn cable_serial(&self) -> Option<&str> {
        self.serial.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vid_pid_constants() {
        assert_eq!(VID, 0x10C4, "Silicon Labs VID");
        assert_eq!(PID, 0xEA80, "stock CP2110 PID");
    }

    #[test]
    fn max_report_payload_matches_an434() {
        // AN434 §6.1: max interrupt report payload is 63 bytes
        assert_eq!(MAX_REPORT_PAYLOAD, 63);
    }

    #[test]
    fn a_write_goes_out_behind_its_length() {
        assert_eq!(output_report(&[0xAB, 0xCD]).unwrap(), [2, 0xAB, 0xCD]);
        let full = [0x5A; MAX_REPORT_PAYLOAD];
        assert_eq!(output_report(&full).unwrap()[..2], [63, 0x5A]);
    }

    /// More than one report holds is refused unsent.
    #[test]
    fn an_oversize_write_is_refused() {
        assert!(output_report(&[0x5A; MAX_REPORT_PAYLOAD + 1]).is_err());
    }

    #[test]
    fn a_read_is_the_bytes_its_length_names() {
        let raw = [3, 0xAB, 0xCD, 0x03, 0x99, 0x99];
        assert_eq!(rx_payload(&raw, 6), [0xAB, 0xCD, 0x03]);
    }

    /// A length byte claiming more than arrived yields the bytes that are
    /// there, and a lone length byte none.
    #[test]
    fn a_read_is_clamped_to_what_arrived() {
        assert_eq!(rx_payload(&[63, 0xAB], 2), [0xAB]);
        assert!(rx_payload(&[63], 1).is_empty());
        assert!(rx_payload(&[63, 0xAB], 0).is_empty());
    }

    /// The little-endian FIFO counts pin the current reading, not a verified
    /// one: the byte order is still an open check
    /// (docs/verification-backlog.md, "USB bridges").
    #[test]
    fn uart_status_fields() {
        let st = parse_uart_status(&[0x42, 0x34, 0x12, 0x01, 0x00, 0x03, 0x01]);
        assert_eq!((st.tx_fifo, st.rx_fifo), (0x1234, 1));
        assert!(st.parity_error && st.overrun_error && st.line_break);
        let overrun = parse_uart_status(&[0x42, 0, 0, 0, 0, 0x02, 0]);
        assert!(!overrun.parity_error && overrun.overrun_error && !overrun.line_break);
    }
}
