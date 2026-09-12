use super::fs9721::Fs9721Protocol;
use super::ut61eplus::Ut61PlusProtocol;
use super::ut171::Ut171Protocol;
use super::ut181a::Ut181aProtocol;
use super::ut8802::Ut8802Protocol;
use super::ut8803::Ut8803Protocol;
use super::vc8x0::vc880::Vc880Protocol;
use super::vc8x0::vc890::Vc890Protocol;
use super::{
    DeviceFamily, Fingerprint, Protocol, fs9721, ut61eplus, ut171, ut181a, ut8802, ut8803, vc8x0,
};
use crate::mock::MockProtocol;

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
}

/// Generic factory for protocols that implement `Default`.
fn factory<P: Protocol + Default + 'static>() -> Box<dyn Protocol> {
    Box::new(P::default())
}

/// Build a UT61+/UT161 protocol for one specific model.
///
/// Every entry in the family goes through here rather than sharing a factory,
/// so a meter reports the model the user actually selected. The UT161x models
/// reuse their UT61x+ counterpart's table, and deriving the profile from the
/// table would make a UT161E introduce itself as a verified UT61E+.
macro_rules! ut61_family_factory {
    ($name:ident, $model:literal) => {
        fn $name() -> Box<dyn Protocol> {
            Box::new(
                Ut61PlusProtocol::for_model($model)
                    .expect(concat!($model, " must be a known UT61+/UT161 model")),
            )
        }
    };
}

ut61_family_factory!(new_ut61eplus, "ut61e+");
ut61_family_factory!(new_ut61bplus, "ut61b+");
ut61_family_factory!(new_ut61dplus, "ut61d+");
ut61_family_factory!(new_ut161e, "ut161e");
ut61_family_factory!(new_ut161b, "ut161b");
ut61_family_factory!(new_ut161d, "ut161d");

const ACTIVATION_UT61EPLUS: &str = "\
1. Insert the USB module into the meter
2. Turn the meter on
3. Long press the USB/Hz button
4. The S icon appears on the LCD";

const ACTIVATION_UT8803: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on";

const ACTIVATION_UT171: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON";

const ACTIVATION_UT181A: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Go to SETUP -> Communication -> ON
Note: this setting resets on power cycle.";

const ACTIVATION_UT803: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on";

const ACTIVATION_VC880: &str = "\
1. Connect the USB cable to the meter
2. Turn the meter on
3. Press the PC button on the meter";

const ACTIVATION_MOCK: &str = "No setup required \u{2014} this is a simulated device.";

/// All selectable devices, in GUI display order.
pub static DEVICES: &[SelectableDevice] = &[
    // UT61E+ family — each model has its own DeviceTable
    SelectableDevice {
        id: "ut61eplus",
        display_name: "UT61E+",
        aliases: &["ut61e+", "ut61e"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut61eplus,
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    },
    SelectableDevice {
        id: "ut61b+",
        display_name: "UT61B+",
        aliases: &["ut61bplus", "ut61b"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut61bplus,
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    },
    SelectableDevice {
        id: "ut61d+",
        display_name: "UT61D+",
        aliases: &["ut61dplus", "ut61d"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut61dplus,
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    },
    SelectableDevice {
        id: "ut161b",
        display_name: "UT161B",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut161b, // same table as UT61B+
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    },
    SelectableDevice {
        id: "ut161d",
        display_name: "UT161D",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut161d, // same table as UT61D+
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    },
    SelectableDevice {
        id: "ut161e",
        display_name: "UT161E",
        aliases: &["ut161"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut161e, // same table as UT61E+
        fingerprint: Some(&ut61eplus::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    },
    // Other families
    SelectableDevice {
        id: "ut8802",
        display_name: "UT8802",
        aliases: &["ut8802n"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT8803, // same setup as UT8803
        family: DeviceFamily::Ut8802,
        new_protocol: factory::<Ut8802Protocol>,
        fingerprint: Some(&ut8802::FINGERPRINT),
        manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT8802"),
    },
    SelectableDevice {
        id: "ut8803",
        display_name: "UT8803",
        aliases: &["ut8803e"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT8803,
        family: DeviceFamily::Ut8803,
        new_protocol: factory::<Ut8803Protocol>,
        fingerprint: Some(&ut8803::FINGERPRINT),
        manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT8803E"),
    },
    SelectableDevice {
        id: "ut803",
        display_name: "UT803",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT803,
        family: DeviceFamily::Fs9721,
        new_protocol: || Box::new(Fs9721Protocol::new_ut803()),
        fingerprint: Some(&fs9721::FINGERPRINT),
        manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT803"),
    },
    SelectableDevice {
        id: "ut804",
        display_name: "UT804",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT803,
        family: DeviceFamily::Fs9721,
        new_protocol: || Box::new(Fs9721Protocol::new_ut804()),
        fingerprint: Some(&fs9721::FINGERPRINT),
        manual_url: Some("https://instruments.uni-trend.com/products/digital-multimeters/UT804"),
    },
    SelectableDevice {
        id: "ut171",
        display_name: "UT171A/B/C",
        aliases: &["ut171a", "ut171b", "ut171c"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT171,
        family: DeviceFamily::Ut171,
        new_protocol: factory::<Ut171Protocol>,
        fingerprint: Some(&ut171::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut171-series/"),
    },
    SelectableDevice {
        id: "ut181a",
        display_name: "UT181A",
        aliases: &["ut181"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT181A,
        family: DeviceFamily::Ut181a,
        new_protocol: factory::<Ut181aProtocol>,
        fingerprint: Some(&ut181a::FINGERPRINT),
        manual_url: Some("https://meters.uni-trend.com/product/ut181a/"),
    },
    // Voltcraft
    SelectableDevice {
        id: "vc880",
        display_name: "Voltcraft VC-880",
        aliases: &["vc-880"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_VC880,
        family: DeviceFamily::Vc880,
        new_protocol: factory::<Vc880Protocol>,
        fingerprint: Some(&vc8x0::VC880_FINGERPRINT),
        manual_url: Some(
            "https://www.conrad.com/p/voltcraft-vc880-handheld-multimeter-digital-calibrated-to-manufacturers-standards-no-certificate-data-logger-cat-iii-124609",
        ),
    },
    SelectableDevice {
        id: "vc650bt",
        display_name: "Voltcraft VC650BT",
        aliases: &["vc-650bt"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_VC880, // same protocol as VC-880
        family: DeviceFamily::Vc880,
        new_protocol: || Box::new(Vc880Protocol::for_model("Voltcraft VC650BT")),
        fingerprint: Some(&vc8x0::VC880_FINGERPRINT),
        manual_url: Some(
            "https://www.conrad.com/p/voltcraft-vc650bt-bench-multimeter-digital-cat-ii-600-v-display-counts-40000-124411",
        ),
    },
    SelectableDevice {
        id: "vc890",
        display_name: "Voltcraft VC-890",
        aliases: &["vc-890"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_VC880, // same activation as VC-880
        family: DeviceFamily::Vc890,
        new_protocol: factory::<Vc890Protocol>,
        fingerprint: Some(&vc8x0::VC890_FINGERPRINT),
        manual_url: Some(
            "https://www.conrad.com/p/voltcraft-vc890-oled-hand-multimeter-digital-oled-display-data-logger-cat-iii-1000-v-cat-iv-600-v-display-counts-60000-124600",
        ),
    },
    // Mock
    SelectableDevice {
        id: "mock",
        display_name: "Mock (simulated)",
        aliases: &[],
        requires_hardware: false,
        activation_instructions: ACTIVATION_MOCK,
        family: DeviceFamily::Mock,
        new_protocol: factory::<MockProtocol>,
        fingerprint: None,
        manual_url: Some(
            "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/cli-reference.md#mock-modes",
        ),
    },
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
    DEVICES.iter().find(|d| d.id == id)
}

/// Resolve a device string: tries exact ID match, then case-insensitive alias match.
pub fn resolve_device(s: &str) -> Option<&'static SelectableDevice> {
    let lower = s.to_lowercase();
    // Try exact ID match first
    if let Some(d) = DEVICES.iter().find(|d| d.id == lower) {
        return Some(d);
    }
    // Try aliases (case-insensitive)
    DEVICES
        .iter()
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
            .filter(|d| d.family == DeviceFamily::Ut61EPlus)
    };
    family()
        .find(|d| d.display_name.to_lowercase() == wanted)
        .or_else(|| family().find(|d| d.aliases.iter().any(|a| a.to_lowercase() == wanted)))
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

    /// The models real hardware has answered for: our UT61E+, and the UT61B+
    /// from two captures reported in issue #19 (2026-09-09 and 2026-09-10).
    /// The UT181A has run for its main modes only, so it is PartlyVerified
    /// (see its profile); everything else must stay flagged so the GUI shows
    /// the EXPERIMENTAL badge and links to the verification issue.
    #[test]
    fn only_hardware_backed_models_are_verified() {
        const VERIFIED: &[&str] = &["ut61eplus", "ut61b+"];
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

    /// The mock is on no cable, so nothing can identify it from the wire.
    #[test]
    fn the_mock_carries_no_fingerprint() {
        assert!(find_device("mock").unwrap().fingerprint.is_none());
    }
}
