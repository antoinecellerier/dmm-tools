//! WCH CH9325 HID-to-UART transport.
//!
//! The CH9325 (or its predecessor HE2325U) is found in bench meters like the
//! UT632, UT803, and UT804; the UT71 and Voltcraft VC9x0 apps set up their
//! cables as a CH9325 too (docs/research/ut71/reverse-engineered-protocol.md
//! §1). It uses 8-byte HID reports with a different framing from both CP2110
//! (64-byte, length-prefixed) and CH9329 (65-byte, report-ID + length).
//!
//! Key differences:
//! - RX: first byte = `0xF0 + payload_length`, then up to 7 UART bytes
//! - TX: first byte = `payload_length`, then up to 7 UART bytes
//! - Max 7 UART bytes per HID report (vs 63 for CP2110/CH9329)
//! - Baud rates: 2400 (primary) or 19200 (fallback), not 9600
//!
//! Reference: docs/research/uci-bench-family/reverse-engineered-protocol.md §4

use crate::error::{Error, Result};
use crate::transport::{Link, Transport, open};
use hidapi::HidDevice;
use log::{debug, trace, warn};
use std::cell::Cell;

/// WCH VID (shared with CH9329).
pub(crate) const VID: u16 = 0x1A86;
/// CH9325 PID (HID-to-UART bridge, used in UT-D04 cable and bench meters).
pub(crate) const PID: u16 = 0xE008;
/// The bridge's name in a registry entry's links, in `dmm-cli list` and in
/// detection.
pub(crate) const NAME: &str = "CH9325";

/// CH9325 HID reports are 8 data bytes.
const HID_REPORT_DATA_SIZE: usize = 8;
/// Maximum UART payload bytes per HID report (8 bytes minus 1-byte header).
const MAX_UART_PAYLOAD: usize = 7;

/// The feature report that sets the bridge to `baud`, config `0x03` (8
/// data bits).
///
/// Byte layout: `[report_id=0x00, baud (4 bytes LE), config=0x03, 0x00,
/// 0x00, 0x00, 0x00]` — for 2400 and 19200 the rate fits bytes 1-2, as the
/// UT803/UT804 apps send it. The SDK DLL puts `0x03` in byte 3 instead;
/// which layout the bridge reads is unverified.
///
/// Reference: docs/research/ut803/reverse-engineered-protocol.md §1.2
fn baud_report(baud: u32) -> [u8; 10] {
    let mut report = [0u8; 10];
    report[1..5].copy_from_slice(&baud.to_le_bytes());
    report[5] = 0x03;
    report
}

const BRIDGE: &str = "CH9325 HID-to-UART bridge (WCH)";

/// What start-up settled on: the rate it left the bridge at, and what the
/// probe read there. Start-up takes any report as an answer, so the meter
/// bytes it carried are what say whether the meter was heard at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Startup {
    baud: u32,
    /// Meter bytes in the probe's report; `None` when no report came.
    probe_bytes: Option<usize>,
}

impl Startup {
    /// The bridge's rate now, `baud`, and what start-up heard, with the rate
    /// it heard it at where a later report changed it.
    fn describe(self, baud: u32) -> String {
        let heard = match self.probe_bytes {
            Some(n) => format!("start-up report carried {n} meter bytes"),
            None => "no start-up report".to_string(),
        };
        if baud == self.baud {
            format!("{baud} baud, {heard}")
        } else {
            format!("{baud} baud, {heard} at {}", self.baud)
        }
    }
}

/// CH9325 HID transport wrapping a `HidDevice`.
pub(crate) struct Ch9325 {
    device: HidDevice,
    /// What [`Transport::cable_serial`] reports, read while opening.
    serial: Option<String>,
    startup: Startup,
    /// The rate the last feature report set, which the bridge is left at: a
    /// protocol's `init` may change the one start-up chose (the UT803's,
    /// through `set_baud`).
    baud: Cell<u32>,
}

impl Ch9325 {
    /// Take an already-opened HID device and bring the bridge up
    /// ([`start_up`]).
    pub(crate) fn open(device: HidDevice) -> Result<Self> {
        let startup = start_up(&device)?;
        Ok(Self {
            serial: open::cable_serial(&device),
            device,
            startup,
            baud: Cell::new(startup.baud),
        })
    }
}

/// Initialize the CH9325 transport by probing baud rates.
///
/// Tries primary init (2400 baud + 0x5A trigger) first, then falls back
/// to 19200 baud if no data is received. Matches the vendor DLL probing
/// sequence (§2.2 step 3).
///
/// Reference: §4.3–4.4
fn start_up(device: &HidDevice) -> Result<Startup> {
    debug!("{NAME}: opening device (VID={VID:#06x} PID={PID:#06x})");

    // Primary init: 2400 baud + 0x5A trigger (§4.3)
    debug!("{NAME}: trying primary init (2400 baud + trigger)");
    send_baud(device, 2400)?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send 0x5A trigger byte (§4.3 step 2)
    send_uart(device, &[0x5A])?;

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Probe: try to read data within 300ms (§2.2 step 3d). Any report
    // counts, even one carrying no meter bytes.
    if let Some(bytes) = probe(device)? {
        debug!("{NAME}: primary init got a report carrying {bytes} meter bytes");
        return Ok(Startup {
            baud: 2400,
            probe_bytes: Some(bytes),
        });
    }

    // Fallback init: 19200 baud, no trigger (§4.4)
    debug!("{NAME}: primary init failed, trying fallback (19200 baud)");
    send_baud(device, 19200)?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Probe again
    let probe_bytes = probe(device)?;
    match probe_bytes {
        Some(bytes) => {
            debug!("{NAME}: fallback init got a report carrying {bytes} meter bytes")
        }
        None => {
            warn!("{NAME}: no data received after init — device may need manual activation")
        }
    }
    Ok(Startup {
        baud: 19200,
        probe_bytes,
    })
}

/// Wait for one raw report and return how many meter bytes it carried,
/// or `None` when none came.
fn probe(device: &HidDevice) -> Result<Option<usize>> {
    let mut raw = [0u8; HID_REPORT_DATA_SIZE + 1];
    let n = device.read_timeout(&mut raw, 300)?;
    if n == 0 {
        return Ok(None);
    }
    trace!("{NAME} probe report ({n} bytes): {:02X?}", &raw[..n]);
    Ok(Some(rx_payload(&raw, n).map_or(0, |(_, len)| len)))
}

/// Send the feature report that sets the bridge to `baud`.
fn send_baud(device: &HidDevice, baud: u32) -> Result<()> {
    let report = baud_report(baud);
    trace!("{NAME} feature report: {:02X?}", report);
    device.send_feature_report(&report).map_err(Error::Hid)?;
    Ok(())
}

/// Send `data` to the meter, in as many reports as it takes.
fn send_uart(device: &HidDevice, data: &[u8]) -> Result<()> {
    for report in output_reports(data) {
        trace!("{NAME} TX: {:02X?}", &report[..2 + report[1] as usize]);
        device.write(&report).map_err(Error::Hid)?;
    }
    Ok(())
}

/// The HID reports that carry `data`: up to seven UART bytes each, as
/// `[report_id=0x00, length, data..., zero-padded to 8 data bytes]` (§4.2).
fn output_reports(data: &[u8]) -> impl Iterator<Item = [u8; HID_REPORT_DATA_SIZE + 1]> + '_ {
    data.chunks(MAX_UART_PAYLOAD).map(|chunk| {
        let mut report = [0u8; HID_REPORT_DATA_SIZE + 1];
        report[1] = chunk.len() as u8;
        report[2..2 + chunk.len()].copy_from_slice(chunk);
        report
    })
}

/// Locate the UART payload inside one raw RX report (§4.2).
///
/// The first data byte is `0xF0 + payload_length` (0xF0–0xF7). Whether
/// hidapi prepends the 0x00 report ID depends on the platform — the CH9325
/// descriptor declares none, so most return the 8 data bytes directly — so
/// detect which layout arrived rather than assuming.
///
/// Returns `(header_byte, payload_start)`, or `None` for a report matching
/// neither layout.
///
/// A free function so the tests exercise the same code the transport runs:
/// they used to paste a copy of this branch into the test body and assert on
/// the copy, which left the real one free to drift.
fn locate_rx_payload(raw: &[u8], n: usize) -> Option<(u8, usize)> {
    match raw.first() {
        // Report ID present: byte 0 = 0x00, byte 1 = 0xF0+len
        Some(0x00) if n >= 2 && raw[1] >= 0xF0 => Some((raw[1], 2)),
        // No report ID: byte 0 = 0xF0+len directly
        Some(&b) if b >= 0xF0 => Some((b, 1)),
        _ => None,
    }
}

/// Where the UART payload starts in one raw RX report and how many bytes it
/// holds, or `None` for a report matching neither layout.
fn rx_payload(raw: &[u8], n: usize) -> Option<(usize, usize)> {
    let (header_byte, start) = locate_rx_payload(raw, n)?;
    let len = ((header_byte & 0x0F) as usize).min(n.saturating_sub(start));
    Some((start, len))
}

impl Transport for Ch9325 {
    fn write(&self, data: &[u8]) -> Result<()> {
        send_uart(&self.device, data)
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

        let Some((payload_start, payload_len)) = rx_payload(&raw, n) else {
            // Unexpected framing — log and return empty
            trace!(
                "{NAME} RX: unexpected framing, raw[0]={:#04x}, n={n}, skipping",
                raw[0]
            );
            return Ok(0);
        };

        if payload_len == 0 {
            return Ok(0);
        }

        let actual = payload_len.min(buf.len());
        buf[..actual].copy_from_slice(&raw[payload_start..payload_start + actual]);
        trace!("{NAME} RX ({actual} bytes): {:02X?}", &buf[..actual]);
        Ok(actual)
    }

    /// Send the feature report for `baud` and record it as the bridge's
    /// rate.
    fn set_baud(&self, baud: u32) -> Result<()> {
        send_baud(&self.device, baud)?;
        self.baud.set(baud);
        Ok(())
    }

    fn transport_info(&self) -> Result<String> {
        Ok(format!(
            "{BRIDGE}, {}",
            self.startup.describe(self.baud.get())
        ))
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
        assert_eq!(VID, 0x1A86, "WCH VID");
        assert_eq!(PID, 0xE008, "CH9325 PID");
    }

    #[test]
    fn vid_shared_with_ch9329_pid_differs() {
        assert_eq!(VID, crate::transport::ch9329::VID);
        assert_ne!(PID, crate::transport::ch9329::PID);
    }

    #[test]
    fn report_size_constants() {
        assert_eq!(HID_REPORT_DATA_SIZE, 8);
        assert_eq!(MAX_UART_PAYLOAD, 7);
    }

    /// The baud rate reads the same whether the bridge takes bytes 1-2 or
    /// bytes 1-4, and `0x03` sits in byte 5, where the meters' apps put it.
    fn assert_report(report: &[u8; 10], baud: u32) {
        assert_eq!(report[0], 0x00); // report ID
        let rate = u32::from_le_bytes([report[1], report[2], report[3], report[4]]);
        assert_eq!(rate, baud);
        assert_eq!(report[5], 0x03);
        assert!(report[6..].iter().all(|&b| b == 0));
    }

    #[test]
    fn primary_feature_report_encoding() {
        assert_report(&baud_report(2400), 2400);
    }

    #[test]
    fn fallback_feature_report_encoding() {
        assert_report(&baud_report(19200), 19200);
    }

    /// The bytes the start-up and the UT803's init sent when they were
    /// constants, which the UT803/UT804 apps send (spec §1.2).
    #[test]
    fn the_reports_match_the_apps_bytes() {
        assert_eq!(
            baud_report(2400),
            [0x00, 0x60, 0x09, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            baud_report(19200),
            [0x00, 0x00, 0x4B, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00]
        );
    }

    /// The start-up trigger goes out as one report.
    #[test]
    fn a_byte_goes_out_as_one_report() {
        let reports: Vec<_> = output_reports(&[0x5A]).collect();
        assert_eq!(reports, [[0x00, 0x01, 0x5A, 0, 0, 0, 0, 0, 0]]);
    }

    /// Eight bytes take two reports, seven then one.
    #[test]
    fn eight_bytes_go_out_as_two_reports() {
        let data = [0xAC, 0x05, 0x12, 0x34, 0x50, 0x01, 0x00, 0x7E];
        let reports: Vec<_> = output_reports(&data).collect();
        assert_eq!(
            reports,
            [
                [0x00, 0x07, 0xAC, 0x05, 0x12, 0x34, 0x50, 0x01, 0x00],
                [0x00, 0x01, 0x7E, 0, 0, 0, 0, 0, 0],
            ]
        );
    }

    /// Both layouts go through the production `rx_payload`. These tests
    /// used to paste a copy of that branch into the test body, so changing
    /// the real one — e.g. tightening the report-ID heuristic after hardware
    /// validation — left them green.
    #[test]
    fn rx_parsing_no_report_id() {
        // Normal case: 8-byte read, no report ID prefix. 0xF2 = 2 payload bytes
        let raw: [u8; 8] = [0xF2, 0x35, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00];
        let (start, len) = rx_payload(&raw, 8).expect("framing recognised");
        assert_eq!(raw[start..start + len], [0x35, 0x41]);
    }

    #[test]
    fn rx_parsing_with_report_id() {
        // Platform where hidapi prepends report ID 0x00
        let raw: [u8; 9] = [0x00, 0xF3, 0xAC, 0x05, 0x12, 0x00, 0x00, 0x00, 0x00];
        let (start, len) = rx_payload(&raw, 9).expect("framing recognised");
        assert_eq!(raw[start..start + len], [0xAC, 0x05, 0x12]);
    }

    /// A report matching neither layout must be reported as unrecognised, so
    /// read_timeout skips it instead of reading a bogus length.
    #[test]
    fn rx_parsing_rejects_unknown_framing() {
        assert_eq!(rx_payload(&[0x12, 0x34], 2), None);
        assert_eq!(rx_payload(&[], 0), None);
        // Report ID present but the next byte isn't a header.
        assert_eq!(rx_payload(&[0x00, 0x12], 2), None);
        // Report ID present but the read was too short to carry a header.
        assert_eq!(rx_payload(&[0x00, 0xF2], 1), None);
    }

    /// The probe counts meter bytes, not the report: an empty report carries
    /// none, and a short read no more than arrived.
    #[test]
    fn rx_payload_counts_the_meter_bytes() {
        assert_eq!(rx_payload(&[0xF0, 0, 0, 0, 0, 0, 0, 0], 8), Some((1, 0)));
        assert_eq!(
            rx_payload(&[0x00, 0xF7, 1, 2, 3, 4, 5, 6, 7], 9),
            Some((2, 7))
        );
        assert_eq!(rx_payload(&[0xF7, 1, 2], 3), Some((1, 2)));
        assert_eq!(rx_payload(&[0x12, 0x34], 2), None);
    }

    #[test]
    fn startup_says_the_rate_and_what_the_probe_heard() {
        let heard = Startup {
            baud: 2400,
            probe_bytes: Some(7),
        };
        assert_eq!(
            heard.describe(2400),
            "2400 baud, start-up report carried 7 meter bytes"
        );
        let silent = Startup {
            baud: 19200,
            probe_bytes: None,
        };
        assert_eq!(silent.describe(19200), "19200 baud, no start-up report");
    }

    /// A UT803's init moves the bridge to 19200 after start-up heard it at
    /// 2400: the rate named is the one the bridge was left at.
    #[test]
    fn a_rate_set_after_start_up_is_the_one_named() {
        let startup = Startup {
            baud: 2400,
            probe_bytes: Some(1),
        };
        assert_eq!(
            startup.describe(19200),
            "19200 baud, start-up report carried 1 meter bytes at 2400"
        );
    }
}
