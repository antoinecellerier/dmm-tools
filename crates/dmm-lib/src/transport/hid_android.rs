//! What stands in for hidapi on Android, where it does not build and an
//! application cannot open a HID device by itself: the cables Android's
//! `UsbManager` lists, opened by it and driven here over usbfs with `nusb`.
//!
//! Listing a cable and opening one are the application's to do, in Java,
//! and reach this module through [`set_usb_host`]. Without a host the bus is
//! empty. An open device behaves as Linux's hidraw does, bytes as they are
//! on the wire (`hid_wire.rs`), so the bridges run unchanged.
//!
//! Only what `open.rs`, `error.rs` and the four bridges call is here, with
//! hidapi's signatures.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::fd::OwnedFd;
use std::sync::OnceLock;
use std::time::Duration;

use log::debug;
use nusb::MaybeFuture;
use nusb::descriptors::TransferType;
use nusb::transfer::{
    Buffer, ControlIn, ControlOut, ControlType, Direction, In, Interrupt, Out, Recipient,
};

use super::hid_wire::{
    FEATURE_REPORT, OUTPUT_REPORT, feature_request_len, place_feature, report_value, wire_bytes,
};

pub(crate) const MAX_REPORT_DESCRIPTOR_SIZE: usize = 4096;

/// The interface class of a HID device.
const HID_CLASS: u8 = 0x03;
/// `bRequest` of the HID class requests used here.
const GET_REPORT: u8 = 0x01;
const SET_REPORT: u8 = 0x09;
/// The standard `GET_DESCRIPTOR` request, and the HID report descriptor's
/// type in its `wValue`.
const GET_DESCRIPTOR: u8 = 0x06;
const REPORT_DESCRIPTOR: u16 = 0x2200;

/// How long a control transfer or a write may take: the kernel's HID
/// driver's own limit.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(5);

/// How many reports wait for a reader, as many as hidraw keeps: a read with
/// no timeout returns one that already arrived, which
/// `framing::discard_queued` counts on. Unlike hidraw, which drops the
/// oldest report when its queue is full, a full queue here holds the device
/// back until one is read.
const QUEUED_REPORTS: usize = 64;

/// A cable Android lists.
#[derive(Debug, Clone)]
pub struct UsbCable {
    /// Android's name for the device, which [`UsbHost::open`] takes. It
    /// changes each time the cable is plugged in.
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub product: Option<String>,
}

/// The application's side of USB on Android: what only its Java code can
/// ask of `UsbManager`.
pub trait UsbHost: Send + Sync {
    /// The devices attached now.
    fn list(&self) -> Vec<UsbCable>;
    /// Open the device of this name, asking the user's permission where it
    /// is needed, and hand over a descriptor this library owns. The error
    /// says why not, for the log.
    fn open(&self, name: &str) -> Result<OwnedFd, String>;
}

static USB_HOST: OnceLock<Box<dyn UsbHost>> = OnceLock::new();

/// Give the library its way to the USB cables. Call it once, before the
/// first open; a second call changes nothing.
pub fn set_usb_host(host: Box<dyn UsbHost>) {
    let _ = USB_HOST.set(host);
}

/// hidapi's error, down to the two variants the crate tells apart.
#[derive(Debug)]
pub enum HidError {
    HidApiError {
        message: String,
    },
    // Never built from a USB error: nusb turns a cancelled transfer into
    // `io::ErrorKind::Interrupted`, which `Error::is_interrupted` reads as
    // the user's Ctrl-C.
    #[allow(dead_code, reason = "matched in error.rs; nothing here raises it")]
    IoError {
        error: std::io::Error,
    },
}

impl std::fmt::Display for HidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HidApiError { message } => write!(f, "USB error: {message}"),
            Self::IoError { error } => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HidError {}

type HidResult<T> = Result<T, HidError>;

fn failed(what: &str, why: impl std::fmt::Display) -> HidError {
    HidError::HidApiError {
        message: format!("{what}: {why}"),
    }
}

/// The bus as it was when this was made.
pub(crate) struct HidApi {
    devices: Vec<DeviceInfo>,
}

impl HidApi {
    pub(crate) fn new() -> HidResult<Self> {
        let cables = USB_HOST.get().map(|host| host.list()).unwrap_or_default();
        let devices = cables
            .into_iter()
            .filter_map(|cable| {
                // A name with a NUL in it names no device.
                let path = CString::new(cable.name.as_str()).ok()?;
                Some(DeviceInfo {
                    path,
                    cable,
                    release: 0,
                })
            })
            .collect();
        Ok(Self { devices })
    }

    pub(crate) fn device_list(&self) -> impl Iterator<Item = &DeviceInfo> {
        self.devices.iter()
    }

    /// The first device with these ids that opens.
    pub(crate) fn open(&self, vid: u16, pid: u16) -> HidResult<HidDevice> {
        let mut last = failed("USB cable", "not attached");
        for info in self
            .devices
            .iter()
            .filter(|info| info.cable.vid == vid && info.cable.pid == pid)
        {
            match HidDevice::open(info) {
                Ok(device) => return Ok(device),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// No cable is known by its serial number here: Android keeps it from
    /// an application until the user has allowed the device, and asks again
    /// each time the cable is plugged in.
    pub(crate) fn open_serial(&self, _vid: u16, _pid: u16, _serial: &str) -> HidResult<HidDevice> {
        Err(failed("USB cable", "none is known by a serial number"))
    }

    pub(crate) fn open_path(&self, path: &CStr) -> HidResult<HidDevice> {
        let info = self
            .devices
            .iter()
            .find(|info| info.path.as_c_str() == path)
            .ok_or_else(|| failed("USB cable", "not attached"))?;
        HidDevice::open(info)
    }
}

/// A listed device.
#[derive(Debug, Clone)]
pub(crate) struct DeviceInfo {
    path: CString,
    cable: UsbCable,
    /// The device's release number, known once it is open.
    release: u16,
}

impl DeviceInfo {
    pub(crate) fn path(&self) -> &CStr {
        &self.path
    }

    pub(crate) fn vendor_id(&self) -> u16 {
        self.cable.vid
    }

    pub(crate) fn product_id(&self) -> u16 {
        self.cable.pid
    }

    pub(crate) fn product_string(&self) -> Option<&str> {
        self.cable.product.as_deref()
    }

    /// Always none: see [`HidApi::open_serial`].
    pub(crate) fn serial_number(&self) -> Option<&str> {
        None
    }

    pub(crate) fn release_number(&self) -> u16 {
        self.release
    }
}

/// An open device: its HID interface, taken from the kernel's driver.
pub(crate) struct HidDevice {
    info: DeviceInfo,
    interface: nusb::Interface,
    /// The interrupt IN endpoint, with [`QUEUED_REPORTS`] transfers waiting
    /// on it. `RefCell`: a transport is used from one thread at a time.
    input: RefCell<nusb::Endpoint<Interrupt, In>>,
    /// The interrupt OUT endpoint. A device without one takes its output
    /// reports on the control endpoint.
    output: Option<RefCell<nusb::Endpoint<Interrupt, Out>>>,
}

impl HidDevice {
    fn open(info: &DeviceInfo) -> HidResult<Self> {
        Self::open_logged(info).inspect_err(|e| {
            // The callers go on to the next cable or the radio and report
            // only that nothing was found.
            debug!("USB cable {} not opened: {e}", info.cable.name);
        })
    }

    fn open_logged(info: &DeviceInfo) -> HidResult<Self> {
        let host = USB_HOST
            .get()
            .ok_or_else(|| failed("USB", "not available"))?;
        let fd = host
            .open(&info.cable.name)
            .map_err(|why| failed("open", why))?;
        let device = nusb::Device::from_fd(fd)
            .wait()
            .map_err(|e| failed("open", e))?;

        let (number, in_address, out_address) = {
            let configuration = device
                .active_configuration()
                .map_err(|e| failed("configuration", e))?;
            configuration
                .interface_alt_settings()
                .filter(|interface| interface.class() == HID_CLASS)
                .find_map(|interface| {
                    let interrupt = |direction| {
                        interface
                            .endpoints()
                            .find(|endpoint| {
                                endpoint.transfer_type() == TransferType::Interrupt
                                    && endpoint.direction() == direction
                            })
                            .map(|endpoint| endpoint.address())
                    };
                    Some((
                        interface.interface_number(),
                        interrupt(Direction::In)?,
                        interrupt(Direction::Out),
                    ))
                })
                .ok_or_else(|| failed("open", "no HID interface"))?
        };

        // The kernel's HID driver holds the interface until it is taken;
        // nusb gives it back when the interface is dropped.
        let interface = device
            .detach_and_claim_interface(number)
            .wait()
            .map_err(|e| failed("claim interface", e))?;
        let mut input = interface
            .endpoint::<Interrupt, In>(in_address)
            .map_err(|e| failed("input endpoint", e))?;
        let output = out_address
            .map(|address| interface.endpoint::<Interrupt, Out>(address))
            .transpose()
            .map_err(|e| failed("output endpoint", e))?
            .map(RefCell::new);

        // One report per transfer: a transfer ends with the packet that
        // carries a report, short or full.
        let report_len = input.max_packet_size();
        while input.pending() < QUEUED_REPORTS {
            input.submit(Buffer::new(report_len));
        }
        debug!(
            "USB cable {}: HID interface {number}, in {in_address:#04x}, out {out_address:02x?}",
            info.cable.name
        );

        Ok(Self {
            info: DeviceInfo {
                release: device.device_descriptor().device_version(),
                ..info.clone()
            },
            interface,
            input: RefCell::new(input),
            output,
        })
    }

    fn set_report(&self, kind: u16, report: &[u8]) -> HidResult<()> {
        self.interface
            .control_out(
                ControlOut {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: SET_REPORT,
                    value: report_value(kind, report),
                    index: u16::from(self.interface.interface_number()),
                    data: wire_bytes(report),
                },
                TRANSFER_TIMEOUT,
            )
            .wait()
            .map_err(|e| failed("set report", e))
    }

    /// Send an output report, its ID first; zero for a device that numbers
    /// none.
    pub(crate) fn write(&self, data: &[u8]) -> HidResult<usize> {
        let payload = wire_bytes(data);
        if payload.is_empty() {
            return Ok(data.len());
        }
        match &self.output {
            Some(output) => output
                .borrow_mut()
                .transfer_blocking(Buffer::from(payload), TRANSFER_TIMEOUT)
                .status
                .map_err(|e| failed("write", e))?,
            None => self.set_report(OUTPUT_REPORT, data)?,
        }
        Ok(data.len())
    }

    /// The next input report as the device sent it, or 0 when none came in
    /// time. A report longer than `buf` is cut. A negative time, hidapi's
    /// wait without end, is none at all here: no bridge asks for it.
    pub(crate) fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> HidResult<usize> {
        let mut input = self.input.borrow_mut();
        // A failed transfer is not queued again, so a device that is gone
        // runs the queue dry.
        if input.pending() == 0 {
            return Err(failed("read", "device disconnected"));
        }
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        let Some(done) = input.wait_next_complete(timeout) else {
            return Ok(0);
        };
        done.status.map_err(|e| failed("read", e))?;
        let n = done.actual_len.min(done.buffer.len()).min(buf.len());
        buf[..n].copy_from_slice(&done.buffer[..n]);
        let mut buffer = done.buffer;
        buffer.clear();
        input.submit(buffer);
        Ok(n)
    }

    pub(crate) fn send_feature_report(&self, data: &[u8]) -> HidResult<()> {
        self.set_report(FEATURE_REPORT, data)
    }

    /// Read the feature report whose ID is in `buf[0]`.
    pub(crate) fn get_feature_report(&self, buf: &mut [u8]) -> HidResult<usize> {
        let sent = self
            .interface
            .control_in(
                ControlIn {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: GET_REPORT,
                    value: report_value(FEATURE_REPORT, buf),
                    index: u16::from(self.interface.interface_number()),
                    length: feature_request_len(buf),
                },
                TRANSFER_TIMEOUT,
            )
            .wait()
            .map_err(|e| failed("get report", e))?;
        Ok(place_feature(buf, &sent))
    }

    pub(crate) fn get_report_descriptor(&self, buf: &mut [u8]) -> HidResult<usize> {
        let sent = self
            .interface
            .control_in(
                ControlIn {
                    control_type: ControlType::Standard,
                    recipient: Recipient::Interface,
                    request: GET_DESCRIPTOR,
                    value: REPORT_DESCRIPTOR,
                    index: u16::from(self.interface.interface_number()),
                    length: u16::try_from(buf.len().min(MAX_REPORT_DESCRIPTOR_SIZE))
                        .unwrap_or(u16::MAX),
                },
                TRANSFER_TIMEOUT,
            )
            .wait()
            .map_err(|e| failed("report descriptor", e))?;
        let n = sent.len().min(buf.len());
        buf[..n].copy_from_slice(&sent[..n]);
        Ok(n)
    }

    pub(crate) fn get_device_info(&self) -> HidResult<DeviceInfo> {
        Ok(self.info.clone())
    }

    /// Always none, so that a session is not tied to a serial number its
    /// cable cannot be found by again: see [`HidApi::open_serial`].
    pub(crate) fn get_serial_number_string(&self) -> HidResult<Option<String>> {
        Ok(None)
    }
}
