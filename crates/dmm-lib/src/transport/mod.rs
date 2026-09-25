#[cfg(feature = "bluetooth")]
pub(crate) mod ble;
/// Same module path either way, so nothing that opens a device needs a `cfg`.
#[cfg(not(feature = "bluetooth"))]
#[path = "ble_disabled.rs"]
pub(crate) mod ble;
pub(crate) mod ch9325;
pub(crate) mod ch9329;
pub(crate) mod cp2110;

use crate::error::Result;

/// Abstraction over HID transport for testability.
///
/// The `Send` bound enables `Box<dyn Transport>` to be moved across threads
/// (required by the GUI's background device thread).
pub trait Transport: Send {
    /// Write data to the device (interrupt OUT report).
    fn write(&self, data: &[u8]) -> Result<()>;

    /// Read data from the device (interrupt IN report).
    /// Returns the number of bytes read, or 0 on timeout.
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize>;

    /// Send a HID feature report.
    fn send_feature_report(&self, data: &[u8]) -> Result<()>;

    /// Query transport-specific version/identification info.
    /// Returns a human-readable string. Default: not supported.
    fn transport_info(&self) -> Result<String> {
        Err(crate::error::Error::invalid_response_msg(
            "not supported by this transport",
        ))
    }

    /// Query transport-specific diagnostic status.
    /// Returns a human-readable string. Default: not supported.
    fn transport_status(&self) -> Result<String> {
        Err(crate::error::Error::invalid_response_msg(
            "not supported by this transport",
        ))
    }

    /// Human-readable transport name (e.g. "CP2110", "CH9329").
    fn transport_name(&self) -> &'static str {
        NO_LINK
    }

    /// For a Bluetooth link, the [`crate::OpenOptions::adapter`] value that
    /// opens this same adapter again by address, with no scan. `None` for
    /// every other link.
    fn bluetooth_selector(&self) -> Option<&str> {
        None
    }

    /// For a Bluetooth link, the name the peer goes by, as the open found
    /// it: what it advertises, or the host's alias for it. `None` for every
    /// other link, and for a peer opened by address that no name was heard
    /// from.
    ///
    /// A fact about the link, not a model: which meters advertise it is the
    /// registry's to say (`crate::built_in_meters`), and when any does,
    /// detection runs only their fingerprints (`docs/detection-design.md`,
    /// Names and the registry).
    fn advertised_name(&self) -> Option<&str> {
        None
    }
}

/// The Bluetooth peers an open or a listing takes, by the name they
/// advertise. The caller picks them from the registry (`bluetooth_peers()` in
/// `lib.rs`), so an open for one meter never lands on another — unless
/// `--adapter` names an address, which opens whatever answers there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BluetoothPeers {
    /// UNI-T's Bluetooth adapters, which carry a family's meter; the
    /// transport knows their names itself.
    pub(crate) adapters: bool,
    /// Name prefixes of meters with the radio built in, as the registry
    /// lists them.
    pub(crate) meters: Vec<&'static str>,
}

/// Whether a peer that goes by `name` carries `prefix`: the name starts with
/// it once trimmed, in any case.
///
/// A prefix because one UT60BT advertises `UT60BTk`
/// (`docs/research/new-device-candidates.md`, Bluetooth section). The one rule
/// both the search and the registry lookup of a peer's name apply
/// (`registry::advertising`), so a peer the search took is found there too.
pub(crate) fn name_matches(prefix: &str, name: &str) -> bool {
    name.trim()
        .to_ascii_uppercase()
        .starts_with(&prefix.to_ascii_uppercase())
}

/// What a transport with no link behind it calls itself.
///
/// The mock and a replay produce their readings in the process, so there is
/// no cable or radio to name — [`crate::binary_help::Link::from_bridge`] reads
/// this back as no link rather than guessing a cable. What a replay says it
/// is on comes from the file instead, not from here.
pub(crate) const NO_LINK: &str = "unknown";

/// Delegate trait through `Box<dyn Transport>` so `Dmm<Box<dyn Transport>>`
/// works for runtime transport selection (CP2110 vs CH9329).
impl Transport for Box<dyn Transport> {
    fn write(&self, data: &[u8]) -> Result<()> {
        (**self).write(data)
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        (**self).read_timeout(buf, timeout_ms)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        (**self).send_feature_report(data)
    }

    fn transport_info(&self) -> Result<String> {
        (**self).transport_info()
    }

    fn transport_status(&self) -> Result<String> {
        (**self).transport_status()
    }

    fn transport_name(&self) -> &'static str {
        (**self).transport_name()
    }

    fn bluetooth_selector(&self) -> Option<&str> {
        (**self).bluetooth_selector()
    }

    fn advertised_name(&self) -> Option<&str> {
        (**self).advertised_name()
    }
}

/// A no-op transport for the mock/simulated device.
pub struct NullTransport;

impl Transport for NullTransport {
    fn write(&self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
        Ok(0)
    }

    fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_transport_all_methods_ok() {
        let t = NullTransport;
        assert!(t.write(&[1, 2, 3]).is_ok());
        let mut buf = [0u8; 64];
        assert_eq!(t.read_timeout(&mut buf, 1000).unwrap(), 0);
        assert!(t.send_feature_report(&[0x41, 0x01]).is_ok());
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod mock {
    use super::*;
    use std::cell::RefCell;

    /// A mock transport that replays pre-recorded responses.
    pub struct MockTransport {
        responses: RefCell<Vec<Vec<u8>>>,
        pub written: RefCell<Vec<Vec<u8>>>,
        pub feature_reports: RefCell<Vec<Vec<u8>>>,
    }

    impl MockTransport {
        /// Build a mock that replays `responses` in order.
        ///
        /// An empty entry models an HID report that carried no UART payload
        /// (an idle poll); once the queue is exhausted the mock stays silent,
        /// which is what a real timeout looks like.
        pub fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses: RefCell::new(responses),
                written: RefCell::new(Vec::new()),
                feature_reports: RefCell::new(Vec::new()),
            }
        }

        /// Queue another response after the mock has gone silent.
        pub fn push_response(&self, response: Vec<u8>) {
            self.responses.borrow_mut().push(response);
        }
    }

    impl Transport for MockTransport {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.written.borrow_mut().push(data.to_vec());
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            let mut responses = self.responses.borrow_mut();
            if responses.is_empty() {
                return Ok(0);
            }
            let response = responses.remove(0);
            let len = response.len().min(buf.len());
            buf[..len].copy_from_slice(&response[..len]);
            Ok(len)
        }

        fn send_feature_report(&self, data: &[u8]) -> Result<()> {
            self.feature_reports.borrow_mut().push(data.to_vec());
            Ok(())
        }
    }
}
