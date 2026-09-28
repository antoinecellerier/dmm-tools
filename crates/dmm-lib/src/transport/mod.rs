#[cfg(feature = "bluetooth")]
pub(crate) mod ble;
/// Same module path either way, so nothing that opens a device needs a `cfg`.
#[cfg(not(feature = "bluetooth"))]
#[path = "ble_disabled.rs"]
pub(crate) mod ble;
pub(crate) mod bu86x;
pub(crate) mod ch9325;
pub(crate) mod ch9329;
pub(crate) mod cp2110;
pub(crate) mod open;

use crate::error::Result;

/// Abstraction over a meter link's byte I/O: a USB-HID bridge, Bluetooth LE,
/// or a mock for tests.
///
/// The `Send` bound enables `Box<dyn Transport>` to be moved across threads
/// (required by the GUI's background device thread).
pub trait Transport: Send {
    /// Write data to the device (interrupt OUT report).
    fn write(&self, data: &[u8]) -> Result<()>;

    /// Read data from the device (interrupt IN report).
    /// Returns the number of bytes read, or 0 on timeout.
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize>;

    /// Move the meter's serial line to `baud`, for a meter that talks at
    /// another rate than the one the link set up. Default: unsupported, for a
    /// link that cannot change its rate. The error names the link, not the
    /// chip.
    fn set_baud(&self, baud: u32) -> Result<()> {
        let link = self.link().map_or("link", |link| link.full_name(false));
        Err(crate::error::Error::UnsupportedCommand(format!(
            "the {link} cannot change its rate to {baud} baud"
        )))
    }

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

    /// The bridge's name, for display and logs (e.g. "CP2110", "CH9329").
    /// Default: "unknown", for a transport with no bridge behind it.
    ///
    /// What the meter is on is [`Transport::link`]'s to say; nothing should
    /// branch on this text.
    fn transport_name(&self) -> &'static str {
        "unknown"
    }

    /// The link the meter is on, and `None` for a transport with nothing on
    /// the far end: the mock answers from inside the process. A replay's
    /// reports the link its file was recorded over.
    /// Required, so a wrapper cannot forget to pass it on.
    fn link(&self) -> Option<Link>;

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
/// `transport/open.rs`), so an open for one meter never lands on another —
/// unless `--adapter` names an address, which opens whatever answers there.
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

/// The link a meter is on, as the user knows it.
///
/// Never the bridge chip: someone plugged in a USB cable or switched a
/// Bluetooth adapter on, and has no reason to know which chip is inside it.
/// The error text, the CLI help and the GUI's connection messages all take
/// their wording from [`Link::full_name`] (and `dmm_shared::help` for the
/// shorter status-line form), so the three cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Link {
    UsbCable,
    Bluetooth,
}

impl Link {
    /// The name for text with the room to spell it out — an error, or a
    /// hover where the bar had to shorten or drop it.
    ///
    /// `built_in_radio` is a meter with Bluetooth built in on the far end
    /// (a registry entry that advertises `bluetooth_names`, or a peer
    /// advertising the name of one): there is no adapter to name.
    ///
    /// Here rather than with the apps' other wording because [`Error`]'s
    /// messages use it.
    ///
    /// [`Error`]: crate::error::Error
    pub const fn full_name(self, built_in_radio: bool) -> &'static str {
        match self {
            Self::UsbCable => "USB cable",
            Self::Bluetooth if built_in_radio => "Bluetooth link",
            Self::Bluetooth => "Bluetooth adapter",
        }
    }
}

/// The full name of the link a bridge is on, by the bridge's name in an
/// error: Bluetooth or a USB cable. `built_in_radio` as for
/// [`Link::full_name`].
pub fn bridge_link_name(bridge: &str, built_in_radio: bool) -> &'static str {
    let link = if bridge == crate::BLUETOOTH {
        Link::Bluetooth
    } else {
        Link::UsbCable
    };
    link.full_name(built_in_radio)
}

/// Delegate trait through `Box<dyn Transport>` so `Dmm<Box<dyn Transport>>`
/// works for runtime transport selection (whichever cable or radio the open
/// found).
impl Transport for Box<dyn Transport> {
    fn write(&self, data: &[u8]) -> Result<()> {
        (**self).write(data)
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        (**self).read_timeout(buf, timeout_ms)
    }

    fn set_baud(&self, baud: u32) -> Result<()> {
        (**self).set_baud(baud)
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

    fn link(&self) -> Option<Link> {
        (**self).link()
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

    fn set_baud(&self, _baud: u32) -> Result<()> {
        Ok(())
    }

    fn link(&self) -> Option<Link> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user plugged in a cable or switched an adapter on; either way the
    /// bridge chip must stay out of what they read.
    #[test]
    fn a_link_is_named_cable_or_adapter_never_the_chip() {
        assert_eq!(
            bridge_link_name(crate::BLUETOOTH, false),
            "Bluetooth adapter"
        );
        for bridge in ["CP2110", "CH9329", "CH9325", "BU-86X"] {
            for built_in in [false, true] {
                assert_eq!(bridge_link_name(bridge, built_in), "USB cable");
            }
        }
    }

    /// A meter with the radio built in has no adapter for the text to name.
    #[test]
    fn a_built_in_radio_is_no_adapter() {
        assert_eq!(bridge_link_name(crate::BLUETOOTH, true), "Bluetooth link");
        assert_eq!(Link::Bluetooth.full_name(true), "Bluetooth link");
    }

    #[test]
    fn null_transport_all_methods_ok() {
        let t = NullTransport;
        assert!(t.write(&[1, 2, 3]).is_ok());
        let mut buf = [0u8; 64];
        assert_eq!(t.read_timeout(&mut buf, 1000).unwrap(), 0);
        assert!(t.set_baud(19200).is_ok());
        assert_eq!(t.link(), None);
    }

    /// A wrapper that let the rate fall to the default would fail every
    /// UT803 open, which sets its rate through the boxed transport.
    #[test]
    fn a_boxed_transport_forwards_the_rate() {
        let t: Box<dyn Transport> = Box::new(NullTransport);
        assert!(t.set_baud(19200).is_ok());
    }

    /// A link with no rate to set says so by the link the user plugged in,
    /// never the bridge chip inside it.
    #[test]
    fn the_default_rate_change_names_the_link() {
        struct Cable(Option<Link>);
        impl Transport for Cable {
            fn write(&self, _data: &[u8]) -> Result<()> {
                Ok(())
            }
            fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
                Ok(0)
            }
            fn transport_name(&self) -> &'static str {
                "CP2110"
            }
            fn link(&self) -> Option<Link> {
                self.0
            }
        }
        let reason = |t: Cable| match t.set_baud(19200) {
            Err(crate::error::Error::UnsupportedCommand(reason)) => reason,
            other => panic!("expected UnsupportedCommand, got {other:?}"),
        };
        assert_eq!(
            reason(Cable(Some(Link::UsbCable))),
            "the USB cable cannot change its rate to 19200 baud"
        );
        assert_eq!(
            reason(Cable(None)),
            "the link cannot change its rate to 19200 baud"
        );
    }

    /// The UT61+ protocol streams or polls by the link, so a box must pass
    /// on a transport's link, not stand in for it.
    #[test]
    fn a_boxed_transport_forwards_the_link() {
        struct Radio;
        impl Transport for Radio {
            fn write(&self, _data: &[u8]) -> Result<()> {
                Ok(())
            }
            fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
                Ok(0)
            }
            fn link(&self) -> Option<Link> {
                Some(Link::Bluetooth)
            }
        }
        let t: Box<dyn Transport> = Box::new(Radio);
        assert_eq!(t.link(), Some(Link::Bluetooth));
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod mock {
    use super::*;
    use std::cell::RefCell;

    /// A mock transport that replays pre-recorded responses.
    pub struct MockTransport {
        responses: RefCell<Vec<Vec<u8>>>,
        /// Held back until the next write, as a polled meter's answer is.
        replies: RefCell<Vec<Vec<u8>>>,
        pub written: RefCell<Vec<Vec<u8>>>,
        /// The rates `set_baud` was asked for, in order.
        pub bauds: RefCell<Vec<u32>>,
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
                replies: RefCell::new(Vec::new()),
                written: RefCell::new(Vec::new()),
                bauds: RefCell::new(Vec::new()),
            }
        }

        /// Queue another response after the mock has gone silent.
        pub fn push_response(&self, response: Vec<u8>) {
            self.responses.borrow_mut().push(response);
        }

        /// Queue a response that arrives only once something is next
        /// written: a polled meter's answer to the next request, rather than
        /// a reply already waiting.
        pub fn push_reply(&self, response: Vec<u8>) {
            self.replies.borrow_mut().push(response);
        }
    }

    impl Transport for MockTransport {
        fn write(&self, data: &[u8]) -> Result<()> {
            self.written.borrow_mut().push(data.to_vec());
            let replies = std::mem::take(&mut *self.replies.borrow_mut());
            self.responses.borrow_mut().extend(replies);
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

        fn set_baud(&self, baud: u32) -> Result<()> {
            self.bauds.borrow_mut().push(baud);
            Ok(())
        }

        fn link(&self) -> Option<Link> {
            None
        }
    }
}
