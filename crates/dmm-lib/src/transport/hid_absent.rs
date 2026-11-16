//! What stands in for hidapi on Android, where it does not build and an
//! application cannot open a HID device by itself anyway: a bus with nothing
//! on it. Listing finds no cable and every open fails, so the code that
//! opens a device runs unchanged and reports that no cable was found.
//!
//! Only what `open.rs`, `error.rs` and the four bridges call is here, with
//! hidapi's signatures.

use std::ffi::CStr;

pub(crate) const MAX_REPORT_DESCRIPTOR_SIZE: usize = 4096;

/// hidapi's error, down to the two variants the crate tells apart.
#[derive(Debug)]
pub enum HidError {
    HidApiError {
        message: String,
    },
    #[allow(dead_code, reason = "matched in error.rs; nothing here raises it")]
    IoError {
        error: std::io::Error,
    },
}

impl std::fmt::Display for HidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HidApiError { message } => write!(f, "hidapi error: {message}"),
            Self::IoError { error } => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HidError {}

type HidResult<T> = Result<T, HidError>;

fn no_usb<T>() -> HidResult<T> {
    Err(HidError::HidApiError {
        message: "USB cables are not supported on Android".to_string(),
    })
}

/// The empty bus.
pub(crate) struct HidApi;

impl HidApi {
    pub(crate) fn new() -> HidResult<Self> {
        Ok(Self)
    }

    pub(crate) fn device_list(&self) -> impl Iterator<Item = &DeviceInfo> {
        std::iter::empty()
    }

    pub(crate) fn open(&self, _vid: u16, _pid: u16) -> HidResult<HidDevice> {
        no_usb()
    }

    pub(crate) fn open_serial(&self, _vid: u16, _pid: u16, _serial: &str) -> HidResult<HidDevice> {
        no_usb()
    }

    pub(crate) fn open_path(&self, _path: &CStr) -> HidResult<HidDevice> {
        no_usb()
    }
}

/// A listed device. There is none: the type has no value.
pub(crate) enum DeviceInfo {}

impl DeviceInfo {
    pub(crate) fn path(&self) -> &CStr {
        match *self {}
    }

    pub(crate) fn vendor_id(&self) -> u16 {
        match *self {}
    }

    pub(crate) fn product_id(&self) -> u16 {
        match *self {}
    }

    pub(crate) fn product_string(&self) -> Option<&str> {
        match *self {}
    }

    pub(crate) fn serial_number(&self) -> Option<&str> {
        match *self {}
    }

    pub(crate) fn release_number(&self) -> u16 {
        match *self {}
    }
}

/// An open device. There is none: the type has no value.
pub(crate) enum HidDevice {}

impl HidDevice {
    pub(crate) fn write(&self, _data: &[u8]) -> HidResult<usize> {
        match *self {}
    }

    pub(crate) fn read_timeout(&self, _buf: &mut [u8], _timeout_ms: i32) -> HidResult<usize> {
        match *self {}
    }

    pub(crate) fn send_feature_report(&self, _data: &[u8]) -> HidResult<()> {
        match *self {}
    }

    pub(crate) fn get_feature_report(&self, _buf: &mut [u8]) -> HidResult<usize> {
        match *self {}
    }

    pub(crate) fn get_report_descriptor(&self, _buf: &mut [u8]) -> HidResult<usize> {
        match *self {}
    }

    pub(crate) fn get_device_info(&self) -> HidResult<DeviceInfo> {
        match *self {}
    }

    pub(crate) fn get_serial_number_string(&self) -> HidResult<Option<String>> {
        match *self {}
    }
}
