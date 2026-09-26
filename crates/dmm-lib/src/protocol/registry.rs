use super::{DeviceFamily, Fingerprint, Protocol};
use super::{ut61eplus, ut80x, ut171, ut181a, ut8802, ut8803, vc8x0, zotek};
use crate::mock;

/// A selectable device in the GUI device picker and CLI --device flag.
pub struct SelectableDevice {
    /// Settings/CLI identifier (e.g., "ut61eplus", "ut61b+", "mock").
    pub id: &'static str,
    /// Human-readable display name (e.g., "UT61E+", "Mock (simulated)").
    pub display_name: &'static str,
    /// Additional strings that resolve to this entry (case-insensitive).
    pub aliases: &'static [&'static str],
    /// Whether this device requires USB hardware.
    pub requires_hardware: bool,
    /// User-facing instructions for enabling data transmission.
    pub activation_instructions: &'static str,
    /// Internal device family for protocol dispatch.
    pub family: DeviceFamily,
    /// Factory: create a Protocol instance configured for this device.
    pub new_protocol: fn() -> Box<dyn Protocol>,
    /// The family's auto-detection rule (`crate::detect`), `None` for a
    /// device that is never on a cable. Every entry of a family points at the
    /// same static: it is the family that is recognised, not the model — the
    /// rule picks the entry itself, from the name the meter reports.
    ///
    /// Having it here is what lets detection derive both its membership and
    /// its order from this table, the way `new_protocol` already gives the
    /// opener its parser.
    pub(crate) fingerprint: Option<&'static Fingerprint>,
    /// URL to manufacturer's product page (for "Manual" hyperlink in GUI).
    pub manual_url: Option<&'static str>,
    /// The links the meter is found on, most likely first: its USB cables by
    /// their transport's `NAME`, and [`crate::BLUETOOTH`]. Each family's
    /// `devices` module says where its list comes from.
    ///
    /// Two things read it. Opening only orders the candidates —
    /// `open_first_match` still falls back to the remaining transports, so an
    /// unusual cable keeps working; without the order, selecting a UT803 on a
    /// bench that also has a UT61E+ attached opens the UT61E+'s CP2110 and
    /// every read times out. Detection takes it literally: a bridge is probed
    /// only with the fingerprints of the meters listed on it, and the "no
    /// meter answered" help lists those same meters — so a cable a meter is
    /// seen on belongs here, whether or not it is the likely one.
    ///
    /// A link left off still opens a named meter: any USB bridge, the listed
    /// ones tried first; the USB adapter `--adapter` names by serial or path;
    /// and the radio at a Bluetooth address given as `--adapter`. Only three
    /// things follow the list strictly: the automatic Bluetooth fallback when
    /// no cable answers, `auto` detection's fingerprints on each link, and a
    /// meter with the radio built in, listed on Bluetooth alone, never
    /// opening USB.
    pub(crate) links: &'static [&'static str],
    /// The meter has Bluetooth built in and no cable, so it is looked for
    /// over Bluetooth alone, whatever the bus holds.
    pub bluetooth_only: bool,
    /// The name prefixes a `bluetooth_only` meter advertises, which is how
    /// an open for it tells it from an adapter or another meter in range;
    /// empty for every other entry.
    pub(crate) bluetooth_names: &'static [&'static str],
}

/// Generic factory for protocols that implement `Default`.
pub(crate) fn factory<P: Protocol + Default + 'static>() -> Box<dyn Protocol> {
    Box::new(P::default())
}

/// All selectable devices, in GUI display order.
///
/// Each entry lives in its family's `devices` module; this list is the one
/// place that orders them.
pub static DEVICES: &[&SelectableDevice] = &[
    // UT61E+ family — each model has its own DeviceTable
    &ut61eplus::devices::UT61EPLUS,
    &ut61eplus::devices::UT61BPLUS,
    &ut61eplus::devices::UT61DPLUS,
    &ut61eplus::devices::UT161B,
    &ut61eplus::devices::UT161D,
    &ut61eplus::devices::UT161E,
    // Bluetooth built in, no cable
    &ut61eplus::devices::UT60BT,
    &ut61eplus::devices::UT202BT,
    // Other families
    &ut8802::devices::UT8802,
    &ut8803::devices::UT8803,
    &ut80x::devices::UT803,
    &ut80x::devices::UT804,
    &ut80x::devices::UT71AB,
    &ut80x::devices::UT71CDE,
    &ut171::devices::UT171,
    &ut181a::devices::UT181A,
    // Voltcraft
    &vc8x0::devices::VC880,
    &vc8x0::devices::VC650BT,
    &vc8x0::devices::VC890,
    &ut80x::devices::VC920,
    // ZOTEK, one entry per packet layout
    &zotek::devices::ZT300AB,
    &zotek::devices::ZT5566SE,
    &zotek::devices::ZT5BQ,
    &zotek::devices::ZT5B,
    // Mock
    &mock::devices::MOCK,
    &zotek::sim::MOCK_ZT5B,
];

/// The `--device` / `device_family` value that names no meter and asks for
/// the one on the cable to be identified instead (`crate::detect`).
///
/// Deliberately not a [`DEVICES`] entry: it selects no protocol, and every
/// place that needs one has to go through detection first. The tests below
/// keep it from ever colliding with a real id or alias.
pub const AUTO_DEVICE_ID: &str = "auto";

/// What a user-supplied device string resolved to.
#[derive(Clone, Copy)]
pub enum Selection {
    /// Identify the meter from the bytes it sends.
    Auto,
    /// A meter the user named.
    Device(&'static SelectableDevice),
}

impl std::fmt::Debug for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SelectableDevice is a table of function pointers with no Debug of
        // its own; the id is what identifies it in a failed assertion.
        match self {
            Self::Auto => f.write_str("Auto"),
            Self::Device(d) => write!(f, "Device({:?})", d.id),
        }
    }
}

/// Find a device by exact ID match.
pub fn find_device(id: &str) -> Option<&'static SelectableDevice> {
    DEVICES.iter().copied().find(|d| d.id == id)
}

/// Resolve a device string: tries exact ID match, then case-insensitive alias match.
///
/// Matching stays exact: the verify-gui skill opens any `mock-*` name without
/// asking, safe only while no looser match can reach a hardware entry.
pub fn resolve_device(s: &str) -> Option<&'static SelectableDevice> {
    let lower = s.to_lowercase();
    // Try exact ID match first
    if let Some(d) = DEVICES.iter().copied().find(|d| d.id == lower) {
        return Some(d);
    }
    // Try aliases (case-insensitive)
    DEVICES
        .iter()
        .copied()
        .find(|d| d.aliases.iter().any(|a| a.to_lowercase() == lower))
}

/// Resolve a device string the same way [`resolve_device`] does, but with
/// [`AUTO_DEVICE_ID`] answering [`Selection::Auto`] instead of nothing.
///
/// Both binaries resolve through this, so `auto` reaches them as a choice
/// rather than as an unknown device.
pub fn resolve_selection(s: &str) -> Option<Selection> {
    if s.trim().eq_ignore_ascii_case(AUTO_DEVICE_ID) {
        return Some(Selection::Auto);
    }
    resolve_device(s).map(Selection::Device)
}

/// Resolve the ASCII model name a UT61+/UT161 meter answers Get Name
/// (`0x5F`) with — "UT61E+", "UT61B+" — to its registry entry.
///
/// Only [`DeviceFamily::Ut61EPlus`] entries are considered: the name frame is
/// that family's alone, so matching the whole registry would let a meter that
/// happens to report "mock" or "UT171" pick an entry whose framing it does not
/// speak. `display_name` first, then aliases, both case-insensitive — the
/// display names are exactly what the meters send.
pub fn device_for_reported_name(name: &str) -> Option<&'static SelectableDevice> {
    let wanted = name.trim().to_lowercase();
    let family = || {
        DEVICES
            .iter()
            .copied()
            .filter(|d| d.family == DeviceFamily::Ut61EPlus)
    };
    family()
        .find(|d| d.display_name.to_lowercase() == wanted)
        .or_else(|| family().find(|d| d.aliases.iter().any(|a| a.to_lowercase() == wanted)))
}

/// The entries of the meters with the radio built in that advertise `name`,
/// in registry order: those whose `bluetooth_names` it carries, by the rule
/// the Bluetooth search takes a peer with ([`crate::transport::name_matches`]).
///
/// Empty for an adapter's name or one no entry lists; several when meters of
/// different packet layouts share one name, and only their frames can tell
/// them apart.
pub(crate) fn advertising(name: &str) -> Vec<&'static SelectableDevice> {
    advertising_in(DEVICES, name)
}

/// [`advertising`] over any table, for the shapes today's registry lacks.
fn advertising_in<'a>(devices: &[&'a SelectableDevice], name: &str) -> Vec<&'a SelectableDevice> {
    devices
        .iter()
        .copied()
        .filter(|d| {
            d.bluetooth_names
                .iter()
                .any(|prefix| crate::transport::name_matches(prefix, name))
        })
        .collect()
}

/// Returns the default device entry ("ut61eplus").
pub fn default_device() -> &'static SelectableDevice {
    find_device("ut61eplus").expect("ut61eplus must be in DEVICES")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Stability;

    #[test]
    fn find_device_by_id() {
        let d = find_device("ut61eplus").unwrap();
        assert_eq!(d.display_name, "UT61E+");
        assert!(d.requires_hardware);
    }

    #[test]
    fn find_device_mock() {
        let d = find_device("mock").unwrap();
        assert_eq!(d.display_name, "Mock (simulated)");
        assert!(!d.requires_hardware);
    }

    #[test]
    fn find_device_unknown() {
        assert!(find_device("nonexistent").is_none());
    }

    #[test]
    fn resolve_by_id() {
        let d = resolve_device("ut8803").unwrap();
        assert_eq!(d.id, "ut8803");
    }

    #[test]
    fn resolve_by_alias() {
        let d = resolve_device("ut61e+").unwrap();
        assert_eq!(d.id, "ut61eplus");
    }

    #[test]
    fn resolve_alias_case_insensitive() {
        let d = resolve_device("UT61E+").unwrap();
        assert_eq!(d.id, "ut61eplus");
    }

    #[test]
    fn resolve_ut171_alias() {
        let d = resolve_device("ut171a").unwrap();
        assert_eq!(d.id, "ut171");
    }

    #[test]
    fn resolve_ut161_alias() {
        let d = resolve_device("ut161").unwrap();
        assert_eq!(d.id, "ut161e");
    }

    #[test]
    fn resolve_unknown() {
        assert!(resolve_device("nonexistent").is_none());
    }

    /// `auto` selects no meter, so no meter may answer to it: an entry that
    /// did would shadow detection and be unreachable by its own name.
    #[test]
    fn no_device_answers_to_auto() {
        for device in DEVICES {
            assert!(
                !device.id.eq_ignore_ascii_case(AUTO_DEVICE_ID),
                "device id {:?} collides with {AUTO_DEVICE_ID:?}",
                device.id
            );
            for alias in device.aliases {
                assert!(
                    !alias.eq_ignore_ascii_case(AUTO_DEVICE_ID),
                    "alias {alias:?} of {} collides with {AUTO_DEVICE_ID:?}",
                    device.id
                );
            }
        }
    }

    /// The CLI flag and the settings file are both user-typed.
    #[test]
    fn resolve_selection_auto_is_case_insensitive() {
        assert!(matches!(resolve_selection("AUTO"), Some(Selection::Auto)));
        assert!(matches!(resolve_selection("auto"), Some(Selection::Auto)));
    }

    #[test]
    fn resolve_selection_names_a_device() {
        let Some(Selection::Device(d)) = resolve_selection("ut61e+") else {
            panic!("ut61e+ must resolve to a device");
        };
        assert_eq!(d.id, "ut61eplus");
    }

    #[test]
    fn resolve_selection_unknown() {
        assert!(resolve_selection("nonexistent").is_none());
    }

    /// Auto-detection picks the entry from the name the meter reports, so
    /// every model in the family has to be reachable by its own display name
    /// — a new entry whose name does not round-trip would be detected as a
    /// plain UT61E+ and decoded with the wrong table.
    #[test]
    fn reported_names_round_trip_for_the_whole_family() {
        for device in DEVICES
            .iter()
            .filter(|d| d.family == DeviceFamily::Ut61EPlus)
        {
            let found = device_for_reported_name(device.display_name)
                .unwrap_or_else(|| panic!("{} does not round-trip", device.display_name));
            assert_eq!(found.id, device.id);
        }
    }

    /// The meter's ASCII is uppercase, but the lookup must not depend on it.
    #[test]
    fn reported_name_is_case_insensitive() {
        assert_eq!(device_for_reported_name("ut61b+").unwrap().id, "ut61b+");
    }

    /// A UT181A never sends a name frame; if something else ever put that
    /// string in one, falling back to the UT61E+ tables beats decoding
    /// LE16 frames as BE16 ones.
    #[test]
    fn reported_name_ignores_other_families() {
        assert!(device_for_reported_name("UT181A").is_none());
        assert!(device_for_reported_name("mock").is_none());
    }

    #[test]
    fn default_device_is_ut61eplus() {
        let d = default_device();
        assert_eq!(d.id, "ut61eplus");
    }

    #[test]
    fn all_ids_unique() {
        let mut ids: Vec<&str> = DEVICES.iter().map(|d| d.id).collect();
        let len_before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len_before, "device IDs must be unique");
    }

    #[test]
    fn no_alias_conflicts_with_ids() {
        let ids: Vec<&str> = DEVICES.iter().map(|d| d.id).collect();
        for device in DEVICES {
            for alias in device.aliases {
                // An alias should not be another device's primary ID
                // (it's fine if it's its own ID, but aliases shouldn't
                // create ambiguity with other entries' primary IDs)
                let alias_lower = alias.to_lowercase();
                for &id in &ids {
                    if id == device.id {
                        continue;
                    }
                    assert_ne!(
                        alias_lower, id,
                        "alias '{}' for device '{}' conflicts with device ID '{}'",
                        alias, device.id, id
                    );
                }
            }
        }
    }

    #[test]
    fn factory_functions_produce_valid_protocols() {
        for device in DEVICES {
            let protocol = (device.new_protocol)();
            let profile = protocol.profile();
            assert!(!profile.family_name.is_empty(), "device {}", device.id);
            assert!(!profile.model_name.is_empty(), "device {}", device.id);
        }
    }

    /// Several models share a device table. Each registry entry must still
    /// report its own model, or a UT161E introduces itself as a UT61E+ and the
    /// user never learns which meter the readings were decoded as.
    #[test]
    fn distinct_devices_report_distinct_model_names() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for device in DEVICES {
            let model = (device.new_protocol)().profile().model_name;
            if let Some((other, _)) = seen.iter().find(|(_, m)| *m == model) {
                panic!("devices {other} and {} both report {model:?}", device.id);
            }
            seen.push((device.id, model));
        }
    }

    /// The models real hardware has answered for: our UT61E+, the UT61B+
    /// from two captures reported in issue #19 (2026-09-09 and 2026-09-10),
    /// and the UT804 from one reporter's meter (issue #16), which walked every
    /// dial position on 2026-09-18. The UT181A has run for its main modes
    /// only, so it is PartlyVerified (see its profile). Everything else must
    /// stay flagged so the GUI shows the EXPERIMENTAL badge and links to the
    /// verification issue.
    #[test]
    fn only_hardware_backed_models_are_verified() {
        const VERIFIED: &[&str] = &["ut61eplus", "ut61b+", "ut804"];
        const PARTLY_VERIFIED: &[&str] = &["ut181a"];
        for device in DEVICES {
            if !device.requires_hardware {
                continue;
            }
            let protocol = (device.new_protocol)();
            let profile = protocol.profile();
            let expected = if VERIFIED.contains(&device.id) {
                Stability::Verified
            } else if PARTLY_VERIFIED.contains(&device.id) {
                Stability::PartlyVerified
            } else {
                Stability::Experimental
            };
            assert_eq!(
                profile.stability, expected,
                "device {} has unexpected stability",
                device.id
            );
            if !expected.is_verified() {
                assert!(
                    profile.verification_issue.is_some(),
                    "{} device {} must link to a verification issue",
                    expected.label(),
                    device.id
                );
            }
        }
    }

    #[test]
    fn all_devices_have_activation_instructions() {
        for device in DEVICES {
            assert!(
                !device.activation_instructions.is_empty(),
                "device {} missing activation_instructions",
                device.id
            );
        }
    }

    /// A meter this table can open but that points at no fingerprint is one
    /// `--device auto` silently never finds.
    #[test]
    fn every_hardware_device_carries_a_fingerprint() {
        for device in DEVICES.iter().filter(|d| d.requires_hardware) {
            assert!(
                device.fingerprint.is_some(),
                "{} joins no detection cascade",
                device.id
            );
        }
    }

    /// Detection recognises a family, not a model — so the siblings of one
    /// family share a single rule, and a second rule for it would be dead
    /// code whichever of the two ran first.
    #[test]
    fn one_family_has_one_fingerprint() {
        for device in DEVICES {
            let Some(fingerprint) = device.fingerprint else {
                continue;
            };
            assert_eq!(fingerprint.family, device.family, "{}", device.id);
            for other in DEVICES.iter().filter(|d| d.family == device.family) {
                let same = other
                    .fingerprint
                    .is_some_and(|f| std::ptr::eq(f, fingerprint));
                assert!(same, "{} and {} disagree", device.id, other.id);
            }
        }
    }

    /// The mocks are on no cable or radio, so nothing can identify them from
    /// the wire, and no Bluetooth search looks for them.
    #[test]
    fn the_mocks_carry_no_fingerprint_and_no_radio() {
        let mocks: Vec<&str> = DEVICES
            .iter()
            .filter(|d| !d.requires_hardware)
            .map(|d| d.id)
            .collect();
        assert_eq!(mocks, ["mock", "mock-zt5b"]);
        for id in mocks {
            let device = find_device(id).unwrap();
            assert!(device.fingerprint.is_none(), "{id}");
            assert!(
                !device.bluetooth_only && device.bluetooth_names.is_empty(),
                "{id}"
            );
            assert_eq!(device.family, DeviceFamily::Mock, "{id}");
        }
    }

    /// The verify-gui skill opens any `mock` or `mock-*` device without
    /// asking, so no name with that prefix may reach a meter that needs
    /// hardware.
    #[test]
    fn a_mock_name_never_names_hardware() {
        for device in DEVICES {
            for name in std::iter::once(&device.id).chain(device.aliases) {
                if name.to_lowercase().starts_with("mock") {
                    assert!(!device.requires_hardware, "{} answers {name}", device.id);
                }
            }
        }
    }

    /// A peer's name finds the meters that advertise it, by prefix and in
    /// any case; an adapter's name, or one no entry lists, finds none.
    #[test]
    fn an_advertised_name_finds_the_meters_that_carry_it() {
        let ids = |name| advertising(name).iter().map(|d| d.id).collect::<Vec<_>>();
        for (name, id) in [
            ("UT60BT", "ut60bt"),
            ("UT60BTk", "ut60bt"),
            (" ut202bt ", "ut202bt"),
        ] {
            assert_eq!(ids(name), [id], "{name:?}");
        }
        for name in ["UT-D07B", "UT-D07A", "UT61E+", "UT60", ""] {
            assert!(ids(name).is_empty(), "{name:?}");
        }
    }

    /// A name several entries advertise finds each of them, in table order,
    /// and only them.
    #[test]
    fn a_shared_name_finds_every_meter_that_advertises_it() {
        let ut60bt = find_device("ut60bt").unwrap();
        let table = [
            &SelectableDevice {
                id: "first",
                bluetooth_names: &["Shared DMM"],
                ..*ut60bt
            },
            &SelectableDevice {
                id: "own",
                bluetooth_names: &["Own DMM"],
                ..*ut60bt
            },
            &SelectableDevice {
                id: "second",
                bluetooth_names: &["Own DMM", "Shared DMM"],
                ..*ut60bt
            },
        ];
        let ids = |name| {
            advertising_in(&table, name)
                .iter()
                .map(|d| d.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids("shared dmm"), ["first", "second"]);
        assert_eq!(ids("Own DMM"), ["own", "second"]);
        assert!(ids("Other DMM").is_empty());
    }

    /// A meter with the radio built in is found by its names alone, so it
    /// needs some; any other entry reaches the radio through an adapter and
    /// has none of its own.
    #[test]
    fn only_bluetooth_only_meters_carry_bluetooth_names() {
        for device in DEVICES {
            assert_eq!(
                device.bluetooth_only,
                !device.bluetooth_names.is_empty(),
                "{}",
                device.id
            );
        }
    }
}
