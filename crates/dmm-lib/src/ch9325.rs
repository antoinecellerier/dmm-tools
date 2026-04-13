//! WCH CH9325 HID-to-UART transport.
//!
//! The CH9325 (or its predecessor HE2325U) is found in bench meters like the
//! UT632, UT803, and UT804. It uses 8-byte HID reports with a different framing
//! from both CP2110 (64-byte, length-prefixed) and CH9329 (65-byte, report-ID +
//! length).
//!
//! Key differences:
//! - RX: first byte = `0xF0 + payload_length`, then up to 7 UART bytes
//! - TX: first byte = `payload_length`, then up to 7 UART bytes
//! - Max 7 UART bytes per HID report (vs 63 for CP2110/CH9329)
//! - Baud rates: 2400 (primary) or 19200 (fallback), not 9600
//!
//! Reference: docs/research/uci-bench-family/reverse-engineered-protocol.md §4

use crate::error::{Error, Result};
use crate::transport::Transport;
use hidapi::HidDevice;
use log::{debug, trace, warn};

/// WCH VID (shared with CH9329).
pub const VID: u16 = 0x1A86;
/// CH9325 PID (HID-to-UART bridge, used in UT-D04 cable and bench meters).
pub const PID: u16 = 0xE008;

/// CH9325 HID reports are 8 data bytes.
const HID_REPORT_DATA_SIZE: usize = 8;
/// Maximum UART payload bytes per HID report (8 bytes minus 1-byte header).
const MAX_UART_PAYLOAD: usize = 7;

/// Primary init feature report: 2400 baud (0x0960 LE), 8N1.
///
/// Byte layout: `[report_id=0x00, baud_lo=0x60, baud_hi=0x09, config=0x03,
///               0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
///
/// Reference: §4.3 (FUN_1001d360)
const PRIMARY_FEATURE_REPORT: [u8; 10] =
    [0x00, 0x60, 0x09, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// Fallback init feature report: 19200 baud (0x4B00 LE), 8N1.
///
/// Reference: §4.4 (FUN_1001d270)
const FALLBACK_FEATURE_REPORT: [u8; 10] =
    [0x00, 0x00, 0x4B, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// CH9325 HID transport wrapping a `HidDevice`.
pub struct Ch9325 {
    device: HidDevice,
}

impl Ch9325 {
    /// Wrap an already-opened HID device.
    pub fn new(device: HidDevice) -> Self {
        Self { device }
    }

    /// Initialize the CH9325 transport by probing baud rates.
    ///
    /// Tries primary init (2400 baud + 0x5A trigger) first, then falls back
    /// to 19200 baud if no data is received. Matches the vendor DLL probing
    /// sequence from FUN_1001ef50.
    ///
    /// Reference: §4.3–4.4
    pub fn init(&self) -> Result<()> {
        debug!("CH9325: opening device (VID={VID:#06x} PID={PID:#06x})");

        // Primary init: 2400 baud + 0x5A trigger (§4.3)
        debug!("CH9325: trying primary init (2400 baud + trigger)");
        self.device
            .send_feature_report(&PRIMARY_FEATURE_REPORT)
            .map_err(Error::Hid)?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Send 0x5A trigger byte (§4.3 step 2)
        let mut tx_buf = [0u8; HID_REPORT_DATA_SIZE + 1];
        tx_buf[0] = 0x00; // report ID for hidapi
        tx_buf[1] = 0x01; // 1 byte of UART data
        tx_buf[2] = 0x5A; // trigger byte
        self.device.write(&tx_buf).map_err(Error::Hid)?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        // Probe: try to read data within 300ms (§2.2 step 3d)
        let mut probe_buf = [0u8; HID_REPORT_DATA_SIZE + 1];
        let n = self.device.read_timeout(&mut probe_buf, 300)?;
        if n > 0 {
            debug!("CH9325: primary init succeeded ({n} bytes received)");
            return Ok(());
        }

        // Fallback init: 19200 baud, no trigger (§4.4)
        debug!("CH9325: primary init failed, trying fallback (19200 baud)");
        self.device
            .send_feature_report(&FALLBACK_FEATURE_REPORT)
            .map_err(Error::Hid)?;
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Probe again
        let n = self.device.read_timeout(&mut probe_buf, 300)?;
        if n > 0 {
            debug!("CH9325: fallback init succeeded ({n} bytes received)");
        } else {
            warn!("CH9325: no data received after init — device may need manual activation");
        }

        Ok(())
    }
}

impl Transport for Ch9325 {
    fn write(&self, data: &[u8]) -> Result<()> {
        // Split UART data across multiple 8-byte HID reports if needed.
        // Each report: [report_id=0x00, length, data..., zero-padded to 8 data bytes]
        for chunk in data.chunks(MAX_UART_PAYLOAD) {
            let mut report = [0u8; HID_REPORT_DATA_SIZE + 1];
            report[0] = 0x00; // report ID for hidapi
            report[1] = chunk.len() as u8; // UART payload length
            report[2..2 + chunk.len()].copy_from_slice(chunk);
            trace!("CH9325 TX: {:02X?}", &report[..2 + chunk.len()]);
            self.device.write(&report).map_err(Error::Hid)?;
        }
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        // Read one 8-byte HID report. Use a 9-byte buffer to handle platforms
        // that might include a report ID byte (the CH9325 descriptor has no
        // report ID, so most platforms return 8 bytes directly).
        let mut raw = [0u8; HID_REPORT_DATA_SIZE + 1];
        let n = self.device.read_timeout(&mut raw, timeout_ms)?;
        if n == 0 {
            return Ok(0);
        }

        // Parse the RX framing (§4.2):
        //   First data byte = 0xF0 + payload_length (range 0xF0–0xF7)
        //   Following bytes = UART payload, zero-padded
        //
        // Detect whether a report ID byte (0x00) was prepended by hidapi:
        let (header_byte, payload_start) = if raw[0] == 0x00 && n >= 2 && raw[1] >= 0xF0 {
            // Report ID present: byte 0 = 0x00, byte 1 = 0xF0+len
            (raw[1], 2)
        } else if raw[0] >= 0xF0 {
            // No report ID: byte 0 = 0xF0+len directly
            (raw[0], 1)
        } else {
            // Unexpected framing — log and return empty
            trace!(
                "Ch9325 RX: unexpected framing, raw[0]={:#04x}, n={n}, skipping",
                raw[0]
            );
            return Ok(0);
        };

        let payload_len = (header_byte & 0x0F) as usize;
        if payload_len == 0 {
            return Ok(0);
        }

        let available = n.saturating_sub(payload_start);
        let actual = payload_len.min(available).min(buf.len());
        buf[..actual].copy_from_slice(&raw[payload_start..payload_start + actual]);
        trace!("CH9325 RX ({actual} bytes): {:02X?}", &buf[..actual]);
        Ok(actual)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        trace!("CH9325 feature report: {:02X?}", data);
        self.device.send_feature_report(data).map_err(Error::Hid)?;
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        Ok("CH9325 HID-to-UART bridge (WCH)".to_string())
    }

    fn transport_name(&self) -> &'static str {
        "CH9325"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vid_pid_constants() {
        assert_eq!(VID, 0x1A86, "WCH VID");
        assert_eq!(PID, 0xE008, "CH9325 PID");
    }

    #[test]
    fn vid_shared_with_ch9329_pid_differs() {
        assert_eq!(VID, crate::ch9329::VID);
        assert_ne!(PID, crate::ch9329::PID);
    }

    #[test]
    fn report_size_constants() {
        assert_eq!(HID_REPORT_DATA_SIZE, 8);
        assert_eq!(MAX_UART_PAYLOAD, 7);
    }

    #[test]
    fn primary_feature_report_encoding() {
        // Baud rate bytes 1-2 encode 2400 (0x0960 LE)
        let baud = u16::from_le_bytes([PRIMARY_FEATURE_REPORT[1], PRIMARY_FEATURE_REPORT[2]]);
        assert_eq!(baud, 2400);
        assert_eq!(PRIMARY_FEATURE_REPORT[0], 0x00); // report ID
        assert_eq!(PRIMARY_FEATURE_REPORT[3], 0x03); // config (8N1)
    }

    #[test]
    fn fallback_feature_report_encoding() {
        // Baud rate bytes 1-2 encode 19200 (0x4B00 LE)
        let baud = u16::from_le_bytes([FALLBACK_FEATURE_REPORT[1], FALLBACK_FEATURE_REPORT[2]]);
        assert_eq!(baud, 19200);
        assert_eq!(FALLBACK_FEATURE_REPORT[0], 0x00); // report ID
        assert_eq!(FALLBACK_FEATURE_REPORT[3], 0x03); // config (8N1)
    }

    #[test]
    fn tx_report_framing_single_byte() {
        // Trigger byte (0x5A) fits in a single report
        let data = [0x5A];
        let mut report = [0u8; HID_REPORT_DATA_SIZE + 1];
        report[0] = 0x00; // report ID
        report[1] = data.len() as u8;
        report[2] = data[0];

        assert_eq!(report[0], 0x00);
        assert_eq!(report[1], 1);
        assert_eq!(report[2], 0x5A);
        // Remaining bytes zero-padded
        assert!(report[3..].iter().all(|&b| b == 0));
    }

    #[test]
    fn tx_report_framing_max_payload() {
        // 7 UART bytes fills exactly one report
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(data.len(), MAX_UART_PAYLOAD);
        let chunks: Vec<&[u8]> = data.chunks(MAX_UART_PAYLOAD).collect();
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn tx_report_framing_splits_across_reports() {
        // 8 UART bytes requires two reports (7 + 1)
        let data = [0xAC, 0x05, 0x12, 0x34, 0x50, 0x01, 0x00, 0x00];
        let chunks: Vec<&[u8]> = data.chunks(MAX_UART_PAYLOAD).collect();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 7);
        assert_eq!(chunks[1].len(), 1);
    }

    #[test]
    fn rx_parsing_no_report_id() {
        // Normal case: 8-byte read, no report ID prefix
        // 0xF2 = 2 payload bytes
        let raw: [u8; 8] = [0xF2, 0x35, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00];
        let n = 8usize;

        let (header_byte, payload_start) = if raw[0] == 0x00 && n >= 2 && raw[1] >= 0xF0 {
            (raw[1], 2usize)
        } else if raw[0] >= 0xF0 {
            (raw[0], 1usize)
        } else {
            panic!("unexpected framing");
        };

        assert_eq!(header_byte, 0xF2);
        assert_eq!(payload_start, 1);

        let payload_len = (header_byte & 0x0F) as usize;
        assert_eq!(payload_len, 2);
        assert_eq!(
            &raw[payload_start..payload_start + payload_len],
            &[0x35, 0x41]
        );
    }

    #[test]
    fn rx_parsing_with_report_id() {
        // Platform where hidapi prepends report ID 0x00
        let raw: [u8; 9] = [0x00, 0xF3, 0xAC, 0x05, 0x12, 0x00, 0x00, 0x00, 0x00];
        let n = 9usize;

        let (header_byte, payload_start) = if raw[0] == 0x00 && n >= 2 && raw[1] >= 0xF0 {
            (raw[1], 2usize)
        } else if raw[0] >= 0xF0 {
            (raw[0], 1usize)
        } else {
            panic!("unexpected framing");
        };

        assert_eq!(header_byte, 0xF3);
        assert_eq!(payload_start, 2);

        let payload_len = (header_byte & 0x0F) as usize;
        assert_eq!(payload_len, 3);
        assert_eq!(
            &raw[payload_start..payload_start + payload_len],
            &[0xAC, 0x05, 0x12]
        );
    }

    #[test]
    fn rx_max_payload() {
        // 0xF7 = 7 bytes (maximum)
        let payload_len = (0xF7u8 & 0x0F) as usize;
        assert_eq!(payload_len, MAX_UART_PAYLOAD);
    }

    #[test]
    fn rx_zero_payload() {
        // 0xF0 = 0 bytes (empty report)
        let payload_len = (0xF0u8 & 0x0F) as usize;
        assert_eq!(payload_len, 0);
    }

    #[test]
    fn rx_header_range() {
        // Valid RX headers are 0xF0 through 0xF7
        for len in 0..=7u8 {
            let header = 0xF0 | len;
            assert!(header >= 0xF0);
            assert_eq!((header & 0x0F) as usize, len as usize);
        }
    }
}
