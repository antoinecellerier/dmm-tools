use crate::error::{Error, Result};
use crate::transport::Transport;
use hidapi::HidDevice;
use log::{debug, trace};

/// WCH CH9329 VID.
pub const VID: u16 = 0x1A86;
/// CH9329 PID (HID-to-UART bridge, used in UT-D09 cable).
pub const PID: u16 = 0xE429;
/// The bridge's name in a registry entry's links, in `dmm-cli list` and in
/// detection.
pub(crate) const NAME: &str = "CH9329";

/// HID report size: 1 byte report ID + 64 bytes data.
const HID_REPORT_SIZE: usize = 65;
/// Maximum UART payload bytes per HID report (bytes 2..64).
const MAX_UART_PAYLOAD: usize = 63;

/// CH9329 HID transport wrapping a `HidDevice`.
///
/// The CH9329 uses a simple HID report framing for UART data:
/// - Byte 0: Report ID (always 0x00)
/// - Byte 1: UART data length
/// - Bytes 2..2+len: UART payload
pub struct Ch9329 {
    device: HidDevice,
}

impl Ch9329 {
    /// Wrap an already-opened HID device.
    pub fn new(device: HidDevice) -> Self {
        Self { device }
    }

    /// Read the CH9329 configuration (128 bytes in 4 chunks).
    ///
    /// The vendor DLL uses HidD_SetOutputReport/HidD_GetInputReport (output reports
    /// via control pipe), but hidapi only exposes feature reports via
    /// send_feature_report/get_feature_report. This may not work — the CH9329 might
    /// only accept config commands on the output report endpoint. If this fails,
    /// we may need platform-specific code or a different HID library.
    ///
    /// This function is not called during normal operation (config init is
    /// skipped), and the UT181A cables reported so far stream without it
    /// (issue #5). Kept as a diagnostic for a cable that does not.
    #[allow(dead_code)]
    pub(crate) fn read_config(&self) -> Result<[u8; 128]> {
        let mut config = [0u8; 128];
        let offsets: [u8; 4] = [0x00, 0x20, 0x40, 0x60];

        for (i, &offset) in offsets.iter().enumerate() {
            let mut cmd = [0u8; HID_REPORT_SIZE];
            cmd[0] = 0x00; // report ID
            cmd[1] = 0xA0; // config read command
            cmd[2] = offset;
            cmd[3] = 0x20; // chunk size

            trace!("CH9329 config read cmd[{i}]: {:02X?}", &cmd[..4]);
            self.device.send_feature_report(&cmd).map_err(Error::Hid)?;

            std::thread::sleep(std::time::Duration::from_millis(100));

            let mut resp = [0u8; HID_REPORT_SIZE];
            let n = self
                .device
                .get_feature_report(&mut resp)
                .map_err(Error::Hid)?;
            trace!(
                "CH9329 config read resp[{i}] ({n} bytes): {:02X?}",
                &resp[..n.min(HID_REPORT_SIZE)]
            );

            let chunk_start = i * 32;
            let src_start = 1; // skip report ID byte
            let copy_len = 32.min(n.saturating_sub(src_start));
            config[chunk_start..chunk_start + copy_len]
                .copy_from_slice(&resp[src_start..src_start + copy_len]);
        }

        debug!("CH9329 config (128 bytes): {:02X?}", config);
        Ok(config)
    }

    /// Attempt to initialize the CH9329 transport.
    ///
    /// Currently a no-op — the CH9329 may come pre-configured at 9600 baud
    /// by UNI-T. If data doesn't flow, the `read_config` / config write
    /// sequence may be needed.
    pub fn init(&self) -> Result<()> {
        debug!("CH9329: opening device (VID={VID:#06x} PID={PID:#06x})");
        debug!("CH9329: skipping config init (assumed pre-configured at 9600 baud)");
        debug!("CH9329: if data doesn't flow, try RUST_LOG=dmm_lib=trace to see raw HID reports");
        Ok(())
    }
}

impl Transport for Ch9329 {
    fn write(&self, data: &[u8]) -> Result<()> {
        if data.len() > MAX_UART_PAYLOAD {
            return Err(Error::invalid_response_msg(format!(
                "data too large for single CH9329 HID report: {} bytes (max {MAX_UART_PAYLOAD})",
                data.len()
            )));
        }
        let mut report = [0u8; HID_REPORT_SIZE];
        report[0] = 0x00; // report ID
        report[1] = data.len() as u8; // UART data length
        report[2..2 + data.len()].copy_from_slice(data);
        trace!("CH9329 TX: {:02X?}", &report[..2 + data.len()]);
        self.device.write(&report)?;
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        let mut raw = [0u8; HID_REPORT_SIZE];
        let n = self.device.read_timeout(&mut raw, timeout_ms)?;
        if n == 0 {
            return Ok(0);
        }

        let payload = locate_rx_payload(&raw[..n]);
        let actual = payload.len().min(buf.len());
        buf[..actual].copy_from_slice(&payload[..actual]);
        trace!(
            "CH9329 RX ({actual} bytes, raw[0]={:#04x}): {:02X?}",
            raw[0],
            &buf[..actual]
        );
        Ok(actual)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        trace!("CH9329 feature report: {:02X?}", data);
        self.device.send_feature_report(data).map_err(Error::Hid)?;
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        Ok("CH9329 HID-to-UART bridge (WCH)".to_string())
    }

    fn transport_name(&self) -> &'static str {
        NAME
    }
}

/// The UART payload carried by one raw CH9329 HID report.
///
/// The report layout is:
///   byte 0: report ID (0x00) — may be stripped by hidapi on some platforms
///   byte 1: UART data length
///   bytes 2+: UART payload
///
/// On Linux/hidraw, hidapi includes the report ID in the buffer. On
/// Windows/macOS, hidapi may strip it when the report ID is 0x00. The two
/// layouts are told apart by whether byte 0 reads as a report ID (0x00) or
/// as a length.
///
/// The result is clamped to the bytes that actually arrived, so a length
/// byte claiming more than the report holds yields only what is there.
fn locate_rx_payload(report: &[u8]) -> &[u8] {
    let (payload_len, payload_start) = if report.len() >= 2 && report[0] == 0x00 {
        // Report ID present: byte 0 = 0x00 (report ID), byte 1 = length
        (report[1] as usize, 2)
    } else if !report.is_empty() {
        // Report ID stripped: byte 0 = length. This is the normal layout
        // on Windows/macOS, not an anomaly, so it is not worth a log line
        // of its own — it would fire on every report, and
        // `framing::read_uart_bytes` calls this up to MAX_EMPTY_READS
        // (256) times per frame. The `trace!` in `read_timeout` records
        // raw[0], which is what tells the two layouts apart if the data
        // ever looks wrong and the offset needs platform-specific tuning.
        (report[0] as usize, 1)
    } else {
        return &[];
    };

    let available = report.len().saturating_sub(payload_start);
    &report[payload_start..payload_start + payload_len.min(available)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vid_pid_constants() {
        assert_eq!(VID, 0x1A86, "WCH VID");
        assert_eq!(PID, 0xE429, "CH9329 PID");
    }

    #[test]
    fn max_payload_within_report() {
        // 65-byte report: 1 report ID + 1 length + 63 payload
        assert_eq!(MAX_UART_PAYLOAD, 63);
        assert_eq!(HID_REPORT_SIZE, 65);
    }

    #[test]
    fn write_report_framing() {
        // Verify the expected report layout for a write
        let data = [0xAB, 0xCD, 0x03, 0x5E, 0x01, 0xD9];
        let mut report = [0u8; HID_REPORT_SIZE];
        report[0] = 0x00;
        report[1] = data.len() as u8;
        report[2..2 + data.len()].copy_from_slice(&data);

        assert_eq!(report[0], 0x00); // report ID
        assert_eq!(report[1], 6); // length
        assert_eq!(&report[2..8], &data); // payload
        assert_eq!(report[8], 0x00); // padding
    }

    /// Linux/hidraw: hidapi keeps the 0x00 report ID, so byte 1 is the length.
    #[test]
    fn rx_payload_with_report_id() {
        let expected: &[u8] = &[0xAB, 0xCD, 0x03];
        assert_eq!(locate_rx_payload(&[0x00, 3, 0xAB, 0xCD, 0x03]), expected);
    }

    /// Windows/macOS: hidapi strips the 0x00 report ID, so byte 0 is the
    /// length and the same payload sits one byte earlier.
    #[test]
    fn rx_payload_without_report_id() {
        let expected: &[u8] = &[0xAB, 0xCD, 0x03];
        assert_eq!(locate_rx_payload(&[3, 0xAB, 0xCD, 0x03]), expected);
    }

    #[test]
    fn rx_payload_of_empty_report() {
        assert!(locate_rx_payload(&[]).is_empty());
        // Header present, but the report declares no UART data.
        assert!(locate_rx_payload(&[0x00, 0]).is_empty());
    }

    /// A length byte claiming more than arrived must yield the bytes that are
    /// there rather than index past the end of the report.
    #[test]
    fn rx_payload_length_beyond_report() {
        let expected: &[u8] = &[0xAB, 0xCD];
        assert_eq!(locate_rx_payload(&[0x00, 63, 0xAB, 0xCD]), expected);
        // A lone report-ID byte is read as a zero length in the stripped
        // layout, which leaves no payload rather than a one-byte overrun.
        assert!(locate_rx_payload(&[0x00]).is_empty());
    }
}
