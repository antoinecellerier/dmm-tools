//! Brymen's BU-86X kit (BC-86X cable), the optical USB cable of the BM860s,
//! BM820s and BM520s meters (`docs/research/bm86x/reverse-engineered-protocol.md`).
//!
//! Not a UART bridge: the cable's own microcontroller speaks Brymen's
//! request/reply protocol (spec §1.2). The host writes one 3-byte output
//! report, `00 cc 66`, and reads the reply as three 8-byte input reports;
//! neither carries a report ID on the wire (spec §2). So `write` takes
//! exactly one request and `read_timeout` hands back one report's data
//! bytes; which series a request names is the protocol's business
//! (`protocol::bm86x`).

use crate::error::{Error, Result};
use crate::transport::{Link, Transport};
use hidapi::HidDevice;
use log::{debug, trace};

/// Brymen's cable VID, as Brymen's programs match it (spec §2; the sheets
/// print 0x82).
pub const VID: u16 = 0x0820;
/// The cable's PID, as Brymen's programs match it (spec §2).
pub const PID: u16 = 0x0001;
/// The cable's name in a registry entry's links, in `dmm-cli list` and in
/// detection: the kit's, as the manuals call it (spec §1.2).
pub(crate) const NAME: &str = "BU-86X";

/// The data bytes of one output report: a request is `00 cc 66` (spec §3.1).
const REQUEST_LEN: usize = 3;
/// The data bytes of one input report (spec §2).
const REPORT_LEN: usize = 8;

/// The HID write for a request: report ID 0, then its three bytes, four bytes
/// in all as Brymen's programs write them (spec §2). Any other length is a
/// meter of another family sending its own commands to this cable, which
/// could not carry them.
fn output_report(data: &[u8]) -> Result<[u8; REQUEST_LEN + 1]> {
    let request: [u8; REQUEST_LEN] = data.try_into().map_err(|_| {
        Error::UnsupportedCommand(format!(
            "the {NAME} cable takes {REQUEST_LEN}-byte requests, not {} bytes",
            data.len()
        ))
    })?;
    let mut report = [0u8; REQUEST_LEN + 1];
    report[1..].copy_from_slice(&request);
    Ok(report)
}

/// The data bytes of one input report as hidapi returned `n` of them in
/// `raw`. The reports are unnumbered, so none should carry a report-ID byte;
/// a platform that prepends one returns nine bytes with a leading `00`, and
/// only then is it dropped: the report descriptor is not known (spec §12.2).
fn input_payload(raw: &[u8], n: usize) -> &[u8] {
    let raw = &raw[..n.min(raw.len())];
    match raw {
        [0x00, data @ ..] if n == REPORT_LEN + 1 => data,
        _ => raw,
    }
}

/// The cable's firmware version as Brymen's program shows it: the USB
/// release number in hex, a dot between its bytes (spec §2).
fn firmware(release: u16) -> String {
    let [major, minor] = release.to_be_bytes();
    format!("{major:02X}.{minor:02X}")
}

/// The BU-86X cable, opened.
pub struct Bu86x {
    device: HidDevice,
    /// The USB release number, the cable's firmware version (spec §2).
    release: Option<u16>,
}

impl Bu86x {
    /// Take an already-opened HID device and read what the cable says about
    /// itself. Nothing is written: the cable needs no setup (spec §3.3), and
    /// the first request is the protocol's.
    pub(crate) fn open(device: HidDevice) -> Result<Self> {
        let mut cable = Self {
            device,
            release: None,
        };
        match cable.device.get_device_info() {
            Ok(info) => cable.release = Some(info.release_number()),
            Err(e) => debug!("{NAME}: no device info ({e})"),
        }
        // The report descriptor is in no source (spec §12.2): trace it, so a
        // first report carries it.
        let mut descriptor = [0u8; hidapi::MAX_REPORT_DESCRIPTOR_SIZE];
        match cable.device.get_report_descriptor(&mut descriptor) {
            Ok(n) => trace!("{NAME} report descriptor: {:02X?}", &descriptor[..n]),
            Err(e) => debug!("{NAME}: no report descriptor ({e})"),
        }
        Ok(cable)
    }
}

impl Transport for Bu86x {
    fn write(&self, data: &[u8]) -> Result<()> {
        let report = output_report(data)?;
        trace!("{NAME} TX: {report:02X?}");
        self.device.write(&report)?;
        Ok(())
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        let mut raw = [0u8; 64];
        let n = self.device.read_timeout(&mut raw, timeout_ms)?;
        if n == 0 {
            return Ok(0);
        }
        let payload = input_payload(&raw, n);
        let len = payload.len().min(buf.len());
        buf[..len].copy_from_slice(&payload[..len]);
        trace!("{NAME} RX ({n} bytes): {:02X?}", &raw[..n]);
        Ok(len)
    }

    /// No serial number: this text goes into capture reports.
    fn transport_info(&self) -> Result<String> {
        Ok(match self.release {
            Some(release) => format!("{NAME} cable, firmware {}", firmware(release)),
            None => format!("{NAME} cable"),
        })
    }

    fn transport_name(&self) -> &'static str {
        NAME
    }

    fn link(&self) -> Option<Link> {
        Some(Link::UsbCable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_goes_out_as_four_bytes_behind_report_id_0() {
        assert_eq!(
            output_report(&[0x00, 0x86, 0x66]).unwrap(),
            [0x00, 0x00, 0x86, 0x66]
        );
    }

    /// Another family's frame on this cable is refused unsent.
    #[test]
    fn any_other_length_is_refused() {
        for len in [0, 1, 2, 4, 6, 64] {
            assert!(
                matches!(
                    output_report(&vec![0xAB; len]),
                    Err(Error::UnsupportedCommand(_))
                ),
                "{len}"
            );
        }
    }

    #[test]
    fn an_eight_byte_report_is_the_data() {
        let raw = [0x00, 0x01, 0x11, 0xF8, 0xA0, 0xDA, 0xA9, 0xA0];
        assert_eq!(input_payload(&raw, 8), raw);
    }

    /// Nine bytes led by `00` carry a report ID in front of the eight data
    /// bytes; nine led by anything else are passed on whole.
    #[test]
    fn a_nine_byte_report_loses_its_leading_zero() {
        let raw = [0x00, 0x00, 0x01, 0x11, 0xF8, 0xA0, 0xDA, 0xA9, 0xA0];
        assert_eq!(input_payload(&raw, 9), &raw[1..]);
        let odd = [0x42; 9];
        assert_eq!(input_payload(&odd, 9), odd);
    }

    /// A longer report is passed on as it came, so whatever it is shows in
    /// the protocol's log, and so is a short one.
    #[test]
    fn other_lengths_are_passed_on_whole() {
        let raw: Vec<u8> = (0..16).collect();
        assert_eq!(input_payload(&raw, 16), raw.as_slice());
        assert_eq!(input_payload(&raw, 3), &raw[..3]);
        assert_eq!(input_payload(&raw, 99), raw.as_slice());
    }

    #[test]
    fn the_firmware_reads_as_brymen_shows_it() {
        assert_eq!(firmware(0x0100), "01.00");
        assert_eq!(firmware(0x1A2B), "1A.2B");
        assert_eq!(firmware(0x0000), "00.00");
    }

    #[test]
    fn vid_pid_are_the_programs() {
        assert_eq!((VID, PID), (0x0820, 0x0001));
    }
}
