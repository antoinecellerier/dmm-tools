//! Stand-in for [`super::ble`] when the `bluetooth` feature is off.
//!
//! Same signatures, so the open and list paths in `lib.rs` carry no `cfg` of
//! their own: a build without the feature simply never finds an adapter.

use crate::DeviceInfo;
use crate::error::{Error, Result};
use crate::transport::Transport;

/// No selector is a Bluetooth one here, so `--adapter` stays on the HID path
/// and an address nothing answers to fails as a missing adapter.
pub(crate) fn is_bluetooth_selector(_selector: &str) -> bool {
    false
}

pub(crate) fn open_first() -> Result<Box<dyn Transport>> {
    Err(Error::NoTransportFound)
}

pub(crate) fn open_selected(_selector: &str) -> Result<Box<dyn Transport>> {
    Err(Error::NoTransportFound)
}

pub(crate) fn list() -> Result<Vec<DeviceInfo>> {
    Ok(Vec::new())
}
