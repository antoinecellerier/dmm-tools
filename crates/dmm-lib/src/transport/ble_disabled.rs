//! Stand-in for [`super::ble`] when the `bluetooth` feature is off.
//!
//! Same signatures, so the open and list paths in `lib.rs` carry no `cfg` of
//! their own: a build without the feature simply never finds an adapter.

use crate::DeviceInfo;
use crate::error::{Error, Result};
use crate::transport::{BluetoothPeers, Transport};

/// No selector is a Bluetooth one here, so `--adapter` stays on the HID path
/// and an address nothing answers to fails as a missing adapter.
pub(crate) fn is_bluetooth_selector(_selector: &str) -> bool {
    false
}

pub(crate) fn open_first(_peers: &BluetoothPeers) -> Result<Box<dyn Transport>> {
    Err(not_found())
}

pub(crate) fn open_selected(_selector: &str) -> Result<Box<dyn Transport>> {
    Err(not_found())
}

/// `bluetooth_searched` is false: this build looked at no radio at all, so
/// the help must not offer Bluetooth steps.
fn not_found() -> Error {
    Error::NoTransportFound {
        bluetooth_searched: false,
    }
}

pub(crate) fn list(_peers: &BluetoothPeers) -> Result<Vec<DeviceInfo>> {
    Ok(Vec::new())
}
