use super::fs9721::Fs9721Protocol;
use super::ut61eplus::Ut61PlusProtocol;
use super::ut171::Ut171Protocol;
use super::ut181a::Ut181aProtocol;
use super::ut8802::Ut8802Protocol;
use super::ut8803::Ut8803Protocol;
use super::vc880::Vc880Protocol;
use super::vc890::Vc890Protocol;
use super::{DeviceFamily, Protocol};
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
    /// URL to manufacturer's product page (for "Manual" hyperlink in GUI).
    pub manual_url: Option<&'static str>,
}

/// Generic factory for protocols that implement `Default`.
fn factory<P: Protocol + Default + 'static>() -> Box<dyn Protocol> {
    Box::new(P::default())
}

fn new_ut61bplus() -> Box<dyn Protocol> {
    Box::new(Ut61PlusProtocol::for_model("ut61b+").unwrap())
}

fn new_ut61dplus() -> Box<dyn Protocol> {
    Box::new(Ut61PlusProtocol::for_model("ut61d+").unwrap())
}

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
        new_protocol: factory::<Ut61PlusProtocol>,
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
        manual_url: Some("https://meters.uni-trend.com/product/ut61plus-series/"),
    },
    SelectableDevice {
        id: "ut161b",
        display_name: "UT161B",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut61bplus, // same table as UT61B+
        manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    },
    SelectableDevice {
        id: "ut161d",
        display_name: "UT161D",
        aliases: &[],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: new_ut61dplus,
        manual_url: Some("https://meters.uni-trend.com/product/ut161-series/"),
    },
    SelectableDevice {
        id: "ut161e",
        display_name: "UT161E",
        aliases: &["ut161"],
        requires_hardware: true,
        activation_instructions: ACTIVATION_UT61EPLUS,
        family: DeviceFamily::Ut61EPlus,
        new_protocol: factory::<Ut61PlusProtocol>,
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
        new_protocol: factory::<Vc880Protocol>,
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
        manual_url: Some(
            "https://github.com/antoinecellerier/dmm-tools/blob/main/docs/cli-reference.md#mock-modes",
        ),
    },
];

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

/// Returns the default device entry ("ut61eplus").
pub fn default_device() -> &'static SelectableDevice {
    find_device("ut61eplus").expect("ut61eplus must be in DEVICES")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
