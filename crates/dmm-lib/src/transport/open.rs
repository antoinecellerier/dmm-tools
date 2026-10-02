//! Picking the link an open goes through: the USB-HID bridges in
//! [`KNOWN_TRANSPORTS`], tried in the order a meter's links ask for, and the
//! Bluetooth fallback when none answers. The public entry points in `lib.rs`
//! (`open_transport`, `open_device_transport`, `list_devices`, ...) call in
//! here.

use crate::error::{BluetoothOnlyMiss, Error, Result};
use crate::protocol::registry::{self, SelectableDevice};
use crate::transport::{BluetoothPeers, Transport, ble, bu86x, ch9325, ch9329, cp2110};
use crate::{BLUETOOTH, BLUETOOTH_SUPPORTED, DeviceInfo, OpenOptions};
use log::{info, warn};
use std::ffi::CString;

/// Descriptor for a known USB-HID transport bridge.
pub(crate) struct KnownTransport {
    pub(crate) vid: u16,
    pub(crate) pid: u16,
    pub(crate) name: &'static str,
    /// The cable passes the meter's UART bytes through, so any UART meter
    /// can be tried on it. A cable that speaks a meter's protocol itself does
    /// not, and is opened only for a meter that lists it or for `auto`.
    pub(crate) relays_uart: bool,
    /// Open the HID device, initialise the bridge, return a boxed Transport.
    init: fn(hidapi::HidDevice) -> Result<Box<dyn Transport>>,
}

impl KnownTransport {
    /// Whether the HID device `dev` is this bridge, by VID and PID.
    fn matches(&self, dev: &hidapi::DeviceInfo) -> bool {
        dev.vendor_id() == self.vid && dev.product_id() == self.pid
    }
}

/// The cables of `table` an open tries, in order, for a meter whose links
/// are `preferred` (empty when the meter is not named yet).
///
/// The meter's own cables come first, in its link order. A meter on a
/// relaying cable falls back to the other relaying ones, so an unusual
/// pairing still connects; it never falls back to a cable that does not
/// relay, which could not carry it and would stand in the way of its
/// Bluetooth fallback. A meter listing only non-relaying cables gets exactly
/// those. With nothing named, every cable is tried, the relaying ones first.
fn usb_candidates<'a>(table: &'a [KnownTransport], preferred: &[&str]) -> Vec<&'a KnownTransport> {
    let mut ordered: Vec<&KnownTransport> = preferred
        .iter()
        .filter_map(|name| table.iter().find(|kt| kt.name == *name))
        .collect();
    let others = |relays: bool| {
        table
            .iter()
            .filter(move |kt| kt.relays_uart == relays && !preferred.contains(&kt.name))
    };
    if preferred.is_empty() || ordered.iter().any(|kt| kt.relays_uart) {
        ordered.extend(others(true));
    }
    if preferred.is_empty() {
        ordered.extend(others(false));
    }
    ordered
}

/// The Bluetooth peers an open for `device` takes, by name: UNI-T's adapters
/// for a meter behind one, a meter with the radio built in by its own names,
/// and every one of both for a meter not named yet (`auto`, `dmm-cli list`).
///
/// A meter is never taken for another: a UT60BT answering an open for a
/// UT61E+ would be decoded with the UT61E+'s range tables. An address on
/// `--adapter` bypasses this: it opens whatever answers there.
pub(crate) fn bluetooth_peers(device: Option<&SelectableDevice>) -> BluetoothPeers {
    match device {
        Some(device) if device.bluetooth_only() => BluetoothPeers {
            adapters: false,
            meters: device.bluetooth_names.to_vec(),
        },
        Some(_) => BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        },
        None => {
            // Several meters may advertise one name; the search needs it once.
            let mut meters: Vec<&'static str> = Vec::new();
            for name in registry::DEVICES.iter().flat_map(|d| d.bluetooth_names) {
                if !meters.contains(name) {
                    meters.push(name);
                }
            }
            BluetoothPeers {
                adapters: true,
                meters,
            }
        }
    }
}

/// Transports are tried in order — most common first.
pub(crate) const KNOWN_TRANSPORTS: &[KnownTransport] = &[
    KnownTransport {
        vid: cp2110::VID,
        pid: cp2110::PID,
        name: cp2110::NAME,
        relays_uart: true,
        init: |dev| Ok(Box::new(cp2110::Cp2110::open(dev)?)),
    },
    KnownTransport {
        vid: ch9329::VID,
        pid: ch9329::PID,
        name: ch9329::NAME,
        relays_uart: true,
        init: |dev| Ok(Box::new(ch9329::Ch9329::open(dev)?)),
    },
    KnownTransport {
        vid: ch9325::VID,
        pid: ch9325::PID,
        name: ch9325::NAME,
        relays_uart: true,
        init: |dev| Ok(Box::new(ch9325::Ch9325::open(dev)?)),
    },
    // Speaks Brymen's protocol itself, so only the meters that list it, or
    // `auto`, open it.
    KnownTransport {
        vid: bu86x::VID,
        pid: bu86x::PID,
        name: bu86x::NAME,
        relays_uart: false,
        init: |dev| Ok(Box::new(bu86x::Bu86x::open(dev)?)),
    },
];

/// Refuse a link `device` cannot be reached through, before anything is
/// sent on it: what `--adapter` opens is not checked against the meter's
/// links, and a cable that speaks its own meters' protocol carries no other
/// meter, nor such a meter another cable. The same rule as
/// [`usb_candidates`]. The radio carries a meter that lists it, or one on a
/// relaying cable through a UART relay such as the UT-D07B.
fn check_cable(device: &SelectableDevice, bridge: &'static str) -> Result<()> {
    let candidates = usb_candidates(KNOWN_TRANSPORTS, device.links);
    let usable = if bridge == BLUETOOTH {
        device.links.contains(&BLUETOOTH) || candidates.iter().any(|kt| kt.relays_uart)
    } else {
        candidates.iter().any(|kt| kt.name == bridge)
    };
    if usable {
        return Ok(());
    }
    Err(Error::WrongCable {
        cable: bridge,
        model: device.display_name,
    })
}

/// Open a meter with the radio built in, over Bluetooth alone.
///
/// It is on no cable, so whatever the bus holds is another meter and the bus
/// is not opened. Every way this fails is the meter's own: a stack fault
/// ([`Error::Bluetooth`]) is passed on, and the rest name what kept the radio
/// from being searched, or that nothing in range carried the meter's name.
pub(crate) fn open_bluetooth_only(
    entry: &SelectableDevice,
    opts: OpenOptions<'_>,
) -> Result<Box<dyn Transport>> {
    let miss = if !BLUETOOTH_SUPPORTED {
        BluetoothOnlyMiss::NotBuilt
    } else if let Some(selector) = opts.adapter {
        if !ble::is_bluetooth_selector(selector) {
            BluetoothOnlyMiss::UsbAdapter
        } else {
            return ble::open_selected(selector);
        }
    } else if !opts.bluetooth {
        BluetoothOnlyMiss::SwitchedOff
    } else {
        match ble::open_first(&bluetooth_peers(Some(entry))) {
            Err(Error::NoTransportFound { .. }) => BluetoothOnlyMiss::NotInRange,
            opened => return opened,
        }
    };
    Err(Error::BluetoothOnly {
        model: entry.display_name,
        activation: entry.activation_instructions,
        miss,
    })
}

/// [`crate::open_transport`], taking only `peers` on the radio, and refusing
/// a link `meter` cannot be reached through ([`check_cable`]).
pub(crate) fn open_links(
    preferred: &[&'static str],
    peers: &BluetoothPeers,
    meter: Option<&SelectableDevice>,
    opts: OpenOptions<'_>,
) -> Result<(Box<dyn Transport>, &'static str)> {
    // An address or a peripheral UUID can only be a Bluetooth adapter, so the
    // selector alone says which opener the user meant — and asking for one by
    // name is asking for the radio, whatever the probing setting says.
    if let Some(selector) = opts.adapter.filter(|s| ble::is_bluetooth_selector(s)) {
        if let Some(meter) = meter {
            check_cable(meter, BLUETOOTH)?;
        }
        return Ok((ble::open_selected(selector)?, BLUETOOTH));
    }

    let radio_gets_a_turn = bluetooth_is_next(preferred, opts);
    let hid = open_hid_transport(preferred, meter, opts.adapter);
    match hid {
        Ok(opened) => Ok(opened),
        Err(err) if radio_gets_a_turn && usb_failure_falls_through(&err) => {
            match ble::open_first(peers) {
                Ok(transport) => Ok((transport, BLUETOOTH)),
                Err(bluetooth_err) => {
                    // The USB error is what the user acts on: it lists the
                    // cables that were looked for. Why Bluetooth found
                    // nothing — stack off, nothing in range — stays in the
                    // log. The error says the radio was searched, so the help
                    // titles itself on both links rather than on the cable.
                    info!("no meter over Bluetooth either: {bluetooth_err}");
                    Err(searched_bluetooth(err))
                }
            }
        }
        Err(err) => Err(err),
    }
}

/// Whether the radio gets a turn when no cable answers.
///
/// Only on the auto path: a user who named a USB adapter asked for that one,
/// and a meter answering on a radio instead is not what they meant. A build
/// without the feature has no radio to search, and neither has a caller that
/// switched probing off.
fn bluetooth_is_next(preferred: &[&'static str], opts: OpenOptions<'_>) -> bool {
    cfg!(feature = "bluetooth")
        && opts.bluetooth
        && opts.adapter.is_none()
        && (preferred.is_empty() || preferred.contains(&BLUETOOTH))
}

/// Whether this USB failure is one the radio may answer instead. `Hid` is in
/// the list because it is what a missing or broken HID API returns, which on
/// a machine with no USB support at all must not stop Bluetooth working.
fn usb_failure_falls_through(err: &Error) -> bool {
    matches!(err, Error::NoTransportFound { .. } | Error::Hid(_))
}

/// Mark a "nothing found" error as having looked at the radio as well.
fn searched_bluetooth(err: Error) -> Error {
    match err {
        Error::NoTransportFound { cables, .. } => Error::NoTransportFound {
            cables,
            bluetooth_searched: true,
        },
        other => other,
    }
}

/// Open one of the USB-HID bridges, the path every meter but a Bluetooth one
/// takes. A cable `meter` cannot be reached through is refused before its
/// bridge is initialised.
fn open_hid_transport(
    preferred: &[&'static str],
    meter: Option<&SelectableDevice>,
    adapter: Option<&str>,
) -> Result<(Box<dyn Transport>, &'static str)> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;

    let (device, kt) = match adapter {
        Some(adapter) => open_with_adapter(&api, adapter),
        None => open_first_match(&api, preferred),
    }?;
    if let Some(meter) = meter {
        check_cable(meter, kt.name)?;
    }
    info!(
        "found {} adapter (VID={:#06x} PID={:#06x})",
        kt.name, kt.vid, kt.pid
    );
    Ok(((kt.init)(device)?, kt.name))
}

/// Open a specific adapter identified by serial number or HID path.
///
/// Tries serial number matching first (most common), then falls back to
/// HID path matching. This avoids needing to guess the format — path
/// formats vary across platforms (Linux `/dev/hidrawN`, Windows `\\?\HID#...`,
/// macOS `IOService:...`).
fn open_with_adapter(
    api: &hidapi::HidApi,
    adapter: &str,
) -> Result<(hidapi::HidDevice, &'static KnownTransport)> {
    // Try serial number first — fast, no enumeration needed.
    for kt in KNOWN_TRANSPORTS {
        if let Ok(device) = api.open_serial(kt.vid, kt.pid, adapter) {
            return Ok((device, kt));
        }
    }

    // Fall back to HID path — enumerate to determine which transport.
    let dev_info = api
        .device_list()
        .find(|dev| dev.path().to_string_lossy() == adapter);

    if let Some(dev_info) = dev_info {
        let kt = KNOWN_TRANSPORTS
            .iter()
            .find(|kt| kt.matches(dev_info))
            .ok_or_else(|| {
                Error::AdapterNotFound(format!(
                    "{adapter} (device exists but is not a supported USB adapter)"
                ))
            })?;

        let path =
            CString::new(adapter).map_err(|_| Error::AdapterNotFound(adapter.to_string()))?;
        let device = api.open_path(&path).map_err(Error::Hid)?;
        Ok((device, kt))
    } else {
        Err(Error::AdapterNotFound(adapter.to_string()))
    }
}

/// Open the first matching adapter, `preferred` naming the cables to try
/// first — the ones the selected family ships with, or nothing at all when no
/// family has been selected. Warns if multiple adapters are found.
fn open_first_match(
    api: &hidapi::HidApi,
    preferred: &[&'static str],
) -> Result<(hidapi::HidDevice, &'static KnownTransport)> {
    let candidates = usb_candidates(KNOWN_TRANSPORTS, preferred);
    let match_count: usize = api
        .device_list()
        .filter(|dev| candidates.iter().any(|kt| kt.matches(dev)))
        .count();

    if match_count > 1 {
        // Nothing to prefer when the meter is still unknown — the probe runs
        // on whichever bridge opens first.
        let preference = if preferred.is_empty() {
            ""
        } else {
            " Preferring the cable this meter uses."
        };
        warn!(
            "Multiple USB cables found ({match_count} devices).{preference} Pass --adapter to pick one."
        );
    }

    for &kt in &candidates {
        if let Ok(device) = api.open(kt.vid, kt.pid) {
            return Ok((device, kt));
        }
    }

    // The bus alone was looked at here; [`open_links`] marks the error
    // if it goes on to search the radio.
    Err(Error::NoTransportFound {
        cables: candidates.iter().map(|kt| kt.name).collect(),
        bluetooth_searched: false,
    })
}

/// The connected USB adapters, for [`crate::list_devices`].
pub(crate) fn list_usb() -> Result<Vec<DeviceInfo>> {
    let api = hidapi::HidApi::new().map_err(Error::Hid)?;
    let mut devices = Vec::new();

    for dev in api.device_list() {
        let transport = KNOWN_TRANSPORTS.iter().find(|kt| kt.matches(dev));
        let Some(kt) = transport else { continue };

        devices.push(DeviceInfo {
            path: dev.path().to_string_lossy().into_owned(),
            product: dev.product_string().map(|s| s.to_string()),
            serial: dev
                .serial_number()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            transport: kt.name,
            not_heard: false,
        });
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typo in an entry's `links` would silently degrade to the old fixed
    /// order rather than failing to build.
    #[test]
    fn preferred_transport_names_exist() {
        for device in registry::DEVICES {
            for name in device.links {
                assert!(
                    *name == BLUETOOTH || KNOWN_TRANSPORTS.iter().any(|kt| kt.name == *name),
                    "device {} prefers unknown transport {name:?}",
                    device.id,
                );
            }
        }
    }

    /// Which links a failed open looked at, which is what the binaries title
    /// their help on. A build without the feature searches no radio, and
    /// neither does a meter that only ships with a cable or a user who named
    /// a USB adapter.
    #[test]
    fn the_radio_gets_a_turn_only_where_it_could_answer() {
        let bluetooth = cfg!(feature = "bluetooth");
        let opts = OpenOptions::new();
        assert_eq!(bluetooth_is_next(&[], opts), bluetooth, "the auto path");
        assert_eq!(
            bluetooth_is_next(registry::find_device("ut61eplus").unwrap().links, opts),
            bluetooth,
            "a family the adapter carries"
        );
        assert!(
            !bluetooth_is_next(registry::find_device("ut804").unwrap().links, opts),
            "a cable-only family"
        );
        assert!(
            !bluetooth_is_next(
                &[],
                OpenOptions {
                    adapter: Some("00C5B27A"),
                    ..OpenOptions::new()
                }
            ),
            "a named USB adapter"
        );
        assert!(
            !bluetooth_is_next(
                &[],
                OpenOptions {
                    bluetooth: false,
                    ..OpenOptions::new()
                }
            ),
            "probing switched off"
        );
    }

    /// The cables an open tries for `id`'s meter, `auto` for none.
    fn cables_for(id: &str) -> Vec<&'static str> {
        let links = match id {
            "auto" => &[][..],
            id => registry::find_device(id).unwrap().links,
        };
        usb_candidates(KNOWN_TRANSPORTS, links)
            .iter()
            .map(|kt| kt.name)
            .collect()
    }

    /// The preferred cable comes first and the other relaying ones stay
    /// reachable, so an unusual pairing still connects; the BU-86X, which
    /// speaks Brymen's protocol itself, is tried only for `auto` and its
    /// own meters.
    #[test]
    fn preference_orders_without_excluding() {
        assert_eq!(
            cables_for("auto"),
            ["CP2110", "CH9329", "CH9325", "BU-86X"],
            "every cable, the one that relays nothing last"
        );
        assert_eq!(
            cables_for("ut804"),
            ["CH9325", "CP2110", "CH9329"],
            "the UT80x family uses the CH9325 and keeps its fallbacks"
        );
        assert_eq!(cables_for("ut61eplus"), ["CP2110", "CH9329", "CH9325"]);
        assert_eq!(cables_for("bm86x"), ["BU-86X"]);
    }

    /// `--adapter` can open any cable: one the named meter cannot be
    /// reached through is refused, naming it.
    #[test]
    fn a_cable_the_meter_cannot_use_is_refused() {
        let device = |id| registry::find_device(id).unwrap();
        for (id, cable) in [
            ("ut61eplus", "BU-86X"),
            ("ut804", "BU-86X"),
            ("bm86x", "CP2110"),
            // No UART relay can carry a meter on a cable of its own.
            ("bm86x", BLUETOOTH),
        ] {
            match check_cable(device(id), cable) {
                Err(Error::WrongCable {
                    cable: named,
                    model,
                }) => {
                    assert_eq!((named, model), (cable, device(id).display_name));
                }
                other => panic!("{id} on {cable}: {other:?}"),
            }
        }
        for (id, cable) in [
            ("ut61eplus", "CP2110"),
            ("ut61eplus", "CH9325"),
            ("ut804", "CP2110"),
            ("bm86x", "BU-86X"),
            ("ut61eplus", BLUETOOTH),
            // A meter on relaying cables alone, through a UART relay.
            ("ut804", BLUETOOTH),
            ("ut8802", BLUETOOTH),
        ] {
            assert!(check_cable(device(id), cable).is_ok(), "{id} on {cable}");
        }
        let refused = |cable| check_cable(device("bm86x"), cable).unwrap_err().to_string();
        assert!(
            refused("CP2110").starts_with("--adapter names a CP2110 cable, which cannot carry"),
            "{}",
            refused("CP2110")
        );
        assert!(
            refused(BLUETOOTH).starts_with("--adapter names a Bluetooth adapter, which cannot"),
            "{}",
            refused(BLUETOOTH)
        );
    }

    /// A table with a cable that does not relay UART bytes between two that
    /// do, for the candidate-order tests.
    fn table_with_non_relaying_cable() -> [KnownTransport; 3] {
        fn cable(name: &'static str, relays_uart: bool) -> KnownTransport {
            KnownTransport {
                vid: 0,
                pid: 0,
                name,
                relays_uart,
                init: |_| Err(Error::Timeout),
            }
        }
        [cable("A", true), cable("OWN", false), cable("B", true)]
    }

    fn candidate_names(table: &[KnownTransport], preferred: &[&str]) -> Vec<&'static str> {
        usb_candidates(table, preferred)
            .iter()
            .map(|kt| kt.name)
            .collect()
    }

    /// With no meter named, every cable is tried, the non-relaying ones last.
    #[test]
    fn candidates_for_auto_try_every_cable() {
        let table = table_with_non_relaying_cable();
        assert_eq!(candidate_names(&table, &[]), ["A", "B", "OWN"]);
    }

    /// A UART meter falls back to the relaying cables only, never to one that
    /// could not carry it; a Bluetooth link in its list changes nothing.
    #[test]
    fn candidates_for_a_uart_meter_skip_non_relaying_cables() {
        let table = table_with_non_relaying_cable();
        assert_eq!(candidate_names(&table, &["A", BLUETOOTH]), ["A", "B"]);
    }

    /// A meter listing only a non-relaying cable gets exactly that one.
    #[test]
    fn candidates_for_a_non_relaying_meter_are_its_cables() {
        let table = table_with_non_relaying_cable();
        assert_eq!(candidate_names(&table, &["OWN"]), ["OWN"]);
    }

    /// The meter's link order wins over the table order.
    #[test]
    fn candidates_keep_the_link_order_first() {
        let table = table_with_non_relaying_cable();
        assert_eq!(candidate_names(&table, &["B", "A"]), ["B", "A"]);
        assert_eq!(candidate_names(&table, &["B"]), ["B", "A"]);
    }

    /// Each entry takes only its own peers on the radio: a meter behind an
    /// adapter the adapters, a meter with the radio built in its own names,
    /// and a meter not named yet every one of them.
    #[test]
    fn each_entry_takes_only_its_own_bluetooth_peers() {
        let peers = |id| bluetooth_peers(Some(registry::find_device(id).expect("registry entry")));
        let adapters = BluetoothPeers {
            adapters: true,
            meters: Vec::new(),
        };
        for id in ["ut61eplus", "ut61b+", "ut161e", "ut171", "ut181a"] {
            assert_eq!(peers(id), adapters, "{id}");
        }
        for (id, name) in [
            ("ut60bt", "UT60BT"),
            ("ut202bt", "UT202BT"),
            ("zt300ab", "Bluetooth DMM"),
            ("zt5566se", "Bluetooth DMM"),
            ("zt5bq", "Bluetooth DMM"),
            ("zt5b", "Bluetooth DMM"),
            ("121gw", "121GW"),
            ("bm78xbt", "BM78xBT"),
        ] {
            assert_eq!(
                peers(id),
                BluetoothPeers {
                    adapters: false,
                    meters: vec![name],
                }
            );
        }
        for id in ["ow18b", "ow18e", "b33", "b35t+", "b41t+", "cm2100b"] {
            assert_eq!(
                peers(id),
                BluetoothPeers {
                    adapters: false,
                    meters: vec!["BDM", "LILLIPUT"],
                },
                "{id}"
            );
        }
        for id in ["cms101", "cms061", "ow65b", "ow67b", "ow69b"] {
            assert_eq!(
                peers(id),
                BluetoothPeers {
                    adapters: false,
                    meters: vec!["BDM"],
                },
                "{id}"
            );
        }
        for id in ["vc871", "vc891", "vc915", "vc925pv"] {
            assert_eq!(
                peers(id),
                BluetoothPeers {
                    adapters: false,
                    meters: vec!["BDM", "VC8", "VC9"],
                },
                "{id}"
            );
        }
        assert_eq!(
            bluetooth_peers(None),
            BluetoothPeers {
                adapters: true,
                meters: vec![
                    "UT60BT",
                    "UT202BT",
                    "Bluetooth DMM",
                    "121GW",
                    "BM78xBT",
                    "BDM",
                    "LILLIPUT",
                    "VC8",
                    "VC9"
                ],
            }
        );
    }

    /// A meter with the radio built in never falls back to the bus, and says
    /// what kept it from the radio rather than that no cable was found.
    #[test]
    fn a_bluetooth_only_meter_names_what_kept_it_off_the_radio() {
        let ut60bt = registry::find_device("ut60bt").expect("registry entry");
        let miss = |opts| match open_bluetooth_only(ut60bt, opts) {
            Err(Error::BluetoothOnly {
                model,
                activation,
                miss,
                ..
            }) => {
                assert_eq!(model, "UT60BT");
                assert_eq!(activation, ut60bt.activation_instructions);
                miss
            }
            Err(e) => panic!("unexpected error: {e}"),
            Ok(_) => panic!("opened a meter with no radio searched"),
        };
        let switched_off = OpenOptions {
            adapter: None,
            bluetooth: false,
        };
        let usb_adapter = OpenOptions {
            adapter: Some("00C5B27A"),
            bluetooth: true,
        };
        let (off, usb) = if BLUETOOTH_SUPPORTED {
            (
                BluetoothOnlyMiss::SwitchedOff,
                BluetoothOnlyMiss::UsbAdapter,
            )
        } else {
            (BluetoothOnlyMiss::NotBuilt, BluetoothOnlyMiss::NotBuilt)
        };
        assert_eq!(miss(switched_off), off);
        assert_eq!(miss(usb_adapter), usb);
    }

    /// Pull the hex value out of `ATTRS{idVendor}=="1a86"` in a udev rule.
    fn rule_attr(line: &str, name: &str) -> Option<u16> {
        let value = line.split_once(&format!("ATTRS{{{name}}}==\""))?.1;
        u16::from_str_radix(value.split_once('"')?.0, 16).ok()
    }

    /// The shipped udev rules and the transports must name the same devices.
    /// A missing rule leaves a plugged-in meter root-only on Linux, and a stale
    /// one hands out access to hardware nothing here opens any more.
    #[test]
    fn udev_rules_cover_exactly_the_known_transports() {
        // The crate is only ever tested from the repository, where the rules
        // file sits two levels up from crates/dmm-lib.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../udev/70-dmm-tools.rules");
        let rules = std::fs::read_to_string(path).expect("read udev/70-dmm-tools.rules");

        let mut in_rules: Vec<String> = rules
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let vid = rule_attr(line, "idVendor")?;
                let pid = rule_attr(line, "idProduct")?;
                Some(format!("{vid:04x}:{pid:04x}"))
            })
            .collect();
        let mut in_code: Vec<String> = KNOWN_TRANSPORTS
            .iter()
            .map(|kt| format!("{:04x}:{:04x}", kt.vid, kt.pid))
            .collect();

        in_rules.sort_unstable();
        in_code.sort_unstable();
        assert_eq!(
            in_rules, in_code,
            "udev/70-dmm-tools.rules and KNOWN_TRANSPORTS disagree on vid:pid"
        );
    }
}
