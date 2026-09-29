//! The one WinRT call btleplug cannot make for us: a connection-parameter
//! request that lasts the connection.
//!
//! btleplug 0.13's `request_connection_parameters` drops the request object
//! WinRT hands back as soon as it returns, and releasing it withdraws the
//! preference, so the link goes back to what the peer asked for. Keeping the
//! object, and the device it was made on, for as long as the link is open
//! keeps the preference in force.

use btleplug::api::BDAddr;
use windows::Devices::Bluetooth::{
    BluetoothLEDevice, BluetoothLEPreferredConnectionParameters,
    BluetoothLEPreferredConnectionParametersRequest,
    BluetoothLEPreferredConnectionParametersRequestStatus,
};

/// A short-interval request in force: dropping it withdraws it.
pub(super) struct ShortInterval {
    _request: BluetoothLEPreferredConnectionParametersRequest,
    /// Disposing of the device also restores the default parameters.
    _device: BluetoothLEDevice,
}

/// Ask WinRT for its Balanced preset (30–60 ms) on the peer at `address`.
///
/// Windows 11 (build 22000) and later; an older Windows fails the call, and
/// the link keeps the interval the peer asked for.
pub(super) async fn request_short_interval(address: BDAddr) -> Result<ShortInterval, String> {
    let device = BluetoothLEDevice::FromBluetoothAddressAsync(address.into())
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| e.to_string())?;
    let params = BluetoothLEPreferredConnectionParameters::Balanced().map_err(|e| e.to_string())?;
    let request = device
        .RequestPreferredConnectionParameters(&params)
        .map_err(|e| e.to_string())?;
    let status = request.Status().map_err(|e| e.to_string())?;
    if status != BluetoothLEPreferredConnectionParametersRequestStatus::Success {
        return Err(format!("status {}", status.0));
    }
    Ok(ShortInterval {
        _request: request,
        _device: device,
    })
}
