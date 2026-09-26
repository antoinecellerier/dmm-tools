//! The Brymen BM788BT and BM787BT, Bluetooth LE built in
//! (`docs/research/bm78xbt/reverse-engineered-protocol.md`).
//!
//! The meter streams only once the host has logged in with its connection
//! password, which the Bluetooth transport does at bring-up (spec §3;
//! `transport/ble/brymen.rs`).
//!
//! - `packet.rs`: the CRC and the end every framed packet carries

pub(crate) mod packet;

/// The registry id, which the report hint names as `--device`.
#[cfg_attr(not(feature = "bluetooth"), allow(dead_code))]
pub(crate) const ID: &str = "bm78xbt";

/// The factory reset of the meter's connection password and Bluetooth name,
/// from the BM788BT manual p.20 (printed 19) (spec §9.2). It is the way out
/// of a refused password, which is the only one this driver sends (0000).
#[cfg_attr(not(feature = "bluetooth"), allow(dead_code))]
pub(crate) const RESET_GESTURE: &str = "hold the Hz button while turning the dial from OFF \
     to capacitance within 0.6 s: the meter shows \"Org\" and its connection password and \
     Bluetooth name are back to the factory settings";
