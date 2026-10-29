//! The device registry: every meter a user can select, and the one place the
//! binaries learn about devices.
//!
//! Each family keeps its [`SelectableDevice`] entries in its own `devices`
//! module, and [`DEVICES`] orders them. The CLI and GUI resolve what the user
//! typed with [`resolve_selection`] and open it through the `open_*`
//! functions in `lib.rs`; they never match on a [`DeviceFamily`]
//! or build a protocol type. So a new meter is an entry, a [`Protocol`] and,
//! to be found by `auto`, a `Fingerprint` the entry points at: nothing in
//! `detect.rs` and no app code.

use super::{DeviceFamily, Fingerprint, Protocol};
use super::{
    bm78xbt, bm86x, eevblog121gw, owon, ut61eplus, ut80x, ut171, ut181a, ut8802, ut8803, vc8x0,
    zotek,
};
use crate::mock;

/// A selectable device in the GUI device picker and CLI --device flag.
pub struct SelectableDevice {
    /// Settings/CLI identifier (e.g., "ut61eplus", "ut61b+", "mock").
    pub id: &'static str,
    /// The model, as the device lists show it under its brand (e.g.,
    /// "UT61E+", "Mock UT61E+"); [`Self::display_name`] adds the brand.
    pub model: &'static str,
    /// The maker's name on the meter's case, which the device lists group by.
    pub brand: Brand,
    /// Additional strings that resolve to this entry (case-insensitive).
    pub aliases: &'static [&'static str],
    /// The same meter under other brands' names, each a row of its own in
    /// the device lists. Nothing on the wire tells them from this entry, so
    /// they open it.
    pub sold_as: &'static [SoldAs],
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
    /// Two things read it. Opening mostly orders the candidates —
    /// `open_first_match` still falls back to the remaining cables that relay
    /// UART bytes, so an unusual cable keeps working. It never falls back to
    /// a cable that speaks a meter's protocol itself, and a meter that lists
    /// only such non-relaying cables gets exactly those. Without the order,
    /// selecting a UT803 on a bench that also has a UT61E+ attached opens the
    /// UT61E+'s CP2110 and every read times out. Detection takes it
    /// literally: a bridge is probed only with the fingerprints of the meters
    /// listed on it, and the "no meter answered" help lists those same
    /// meters — so a cable a meter is seen on belongs here, whether or not it
    /// is the likely one.
    ///
    /// A link left off still opens a named meter: any relaying USB bridge
    /// when the meter lists one, its listed cables tried first; the USB adapter
    /// `--adapter` names by serial or path; and the radio at a Bluetooth
    /// address given as `--adapter`. Only three
    /// things follow the list strictly: the automatic Bluetooth fallback when
    /// no cable answers, `auto` detection's fingerprints on each link, and a
    /// meter with the radio built in, listed on Bluetooth alone, never
    /// opening USB.
    pub(crate) links: &'static [&'static str],
    /// The links among `links` a hardware report has seen auto-detection
    /// find this meter on after a restart; the evidence is in
    /// docs/verification-backlog.md, "Device auto-detection". `dmm-cli
    /// capture --unverified` leaves its detection check out on these, as it
    /// does a step marked `.verified()`.
    pub detection_verified: &'static [&'static str],
    /// The name prefixes a meter with the radio built in advertises, which
    /// is how an open for it tells it from an adapter or another meter in
    /// range; empty for every other entry.
    pub(crate) bluetooth_names: &'static [&'static str],
}

impl SelectableDevice {
    /// Brand and model, as the app names the meter: "UNI-T UT61E+".
    pub fn display_name(&self) -> &'static str {
        full_names().get(self.id).map_or(self.model, String::as_str)
    }

    /// The entry under its maker's name, as detection names it.
    pub fn listing(&'static self) -> Listing {
        Listing {
            device: self,
            sold_as: None,
        }
    }

    /// The entry's row under `brand`: the maker's, or the name `brand` sells
    /// it as; `None` when that brand doesn't sell it.
    pub fn listing_under(&'static self, brand: Brand) -> Option<Listing> {
        if brand == self.brand {
            return Some(self.listing());
        }
        let sold_as = self.sold_as.iter().find(|s| s.brand == brand)?;
        Some(Listing {
            device: self,
            sold_as: Some(sold_as),
        })
    }

    /// Whether `reported`, the name a meter gave for itself, is this entry's
    /// model. A recording made before the names dropped their brand reports
    /// "OWON CMS101", which still counts.
    pub fn answers_to(&self, reported: &str) -> bool {
        without_brand(reported.trim()).eq_ignore_ascii_case(self.model)
    }

    /// A meter with the radio built in, looked for by its own names over
    /// Bluetooth alone, whatever the bus holds.
    pub fn bluetooth_only(&self) -> bool {
        !self.bluetooth_names.is_empty()
    }
}

/// A name another brand sells a [`SelectableDevice`]'s meter under
/// ([`SelectableDevice::sold_as`]).
pub struct SoldAs {
    /// The brand on this meter's case, whose heading lists it.
    pub brand: Brand,
    /// Settings/CLI identifier, unique across the registry.
    pub id: &'static str,
    /// The model, as the device lists show it under its brand ("UT217A").
    pub model: &'static str,
    /// Additional strings that resolve to this name (case-insensitive).
    pub aliases: &'static [&'static str],
}

/// A row of the device lists: an entry under its maker's name, or under a
/// name another brand sells it as. Opening it opens [`Self::device`].
#[derive(Clone, Copy)]
pub struct Listing {
    pub device: &'static SelectableDevice,
    /// The other brand's name, `None` for the maker's.
    pub sold_as: Option<&'static SoldAs>,
}

impl Listing {
    /// The id settings and `--device` name this row by.
    pub fn id(&self) -> &'static str {
        self.sold_as.map_or(self.device.id, |s| s.id)
    }

    /// The brand whose heading lists this row.
    pub fn brand(&self) -> Brand {
        self.sold_as.map_or(self.device.brand, |s| s.brand)
    }

    /// The model, as the row reads under its brand's heading.
    pub fn model_name(&self) -> &'static str {
        self.sold_as.map_or(self.device.model, |s| s.model)
    }

    /// Brand and model, as the app names the meter: "UNI-T UT217A".
    pub fn display_name(&self) -> &'static str {
        full_names()
            .get(self.id())
            .map_or(self.model_name(), String::as_str)
    }

    /// Additional strings that resolve to this row.
    pub fn aliases(&self) -> &'static [&'static str] {
        self.sold_as.map_or(self.device.aliases, |s| s.aliases)
    }

    /// The ids and aliases that resolve to this row.
    fn names(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.id()).chain(self.aliases().iter().copied())
    }
}

impl std::fmt::Debug for Listing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Listing({:?})", self.id())
    }
}

/// Every row's brand and model by its id, built once so a display name is a
/// `&'static str` like the model it is made from. A simulated device, which
/// names no brand, has none and goes by its model.
fn full_names() -> &'static std::collections::HashMap<&'static str, String> {
    static NAMES: std::sync::OnceLock<std::collections::HashMap<&str, String>> =
        std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        listings()
            .filter_map(|l| {
                let prefix = l.brand().prefix()?;
                Some((l.id(), format!("{prefix} {}", l.model_name())))
            })
            .collect()
    })
}

/// Every row of the device lists: each entry, then the names it is sold
/// under, in registry order.
pub fn listings() -> impl Iterator<Item = Listing> {
    DEVICES.iter().copied().flat_map(|device| {
        std::iter::once(device.listing()).chain(device.sold_as.iter().map(move |s| Listing {
            device,
            sold_as: Some(s),
        }))
    })
}

/// Whose name a meter carries, as the device lists group them.
///
/// Declared in display order: alphabetical, so a reader finds a brand without
/// knowing ours, with the simulated devices last. It is the brand on the case,
/// not the protocol family: the Voltcraft VC871 runs OWON's frame and lists
/// under Voltcraft, and the UNI-T UT217A, OWON's CMS061, under UNI-T.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Brand {
    Aneng,
    Brymen,
    Bside,
    Eevblog,
    Minipa,
    Owon,
    UniT,
    Voltcraft,
    Zotek,
    Simulated,
}

impl Brand {
    /// Every brand, in display order.
    pub const ALL: [Brand; 10] = [
        Brand::Aneng,
        Brand::Brymen,
        Brand::Bside,
        Brand::Eevblog,
        Brand::Minipa,
        Brand::Owon,
        Brand::UniT,
        Brand::Voltcraft,
        Brand::Zotek,
        Brand::Simulated,
    ];

    /// The brand as a list heading shows it.
    pub fn name(self) -> &'static str {
        match self {
            Brand::Aneng => "ANENG",
            Brand::Brymen => "Brymen",
            Brand::Bside => "BSIDE",
            Brand::Eevblog => "EEVblog",
            Brand::Minipa => "Minipa",
            Brand::Owon => "OWON",
            Brand::UniT => "UNI-T",
            Brand::Voltcraft => "Voltcraft",
            // ZOTEK makes them and sells them under both names.
            Brand::Zotek => "ZOYI / ZOTEK",
            Brand::Simulated => "Simulated",
        }
    }

    /// The name a display name starts with, the maker's where the heading
    /// carries two; `None` for the simulated devices, which name no brand.
    fn prefix(self) -> Option<&'static str> {
        match self {
            Brand::Zotek => Some("ZOTEK"),
            Brand::Simulated => None,
            brand => Some(brand.name()),
        }
    }
}

/// `name` without the brand a display name starts with: "UNI-T UT61E+" is
/// "UT61E+". Any other name comes back whole.
pub fn without_brand(name: &str) -> &str {
    Brand::ALL
        .into_iter()
        .filter_map(|b| b.prefix())
        .find_map(|prefix| name.strip_prefix(prefix)?.strip_prefix(' '))
        .unwrap_or(name)
}

/// The device lists' rows grouped by brand: brands in [`Brand`]'s order, each
/// with its rows in registry order, which keeps related models together and
/// a brand's own meters ahead of the ones it sells under another maker's
/// entry. Brands with no row are left out.
pub fn brand_groups() -> Vec<(Brand, Vec<Listing>)> {
    Brand::ALL
        .into_iter()
        .map(|brand| {
            let rows = listings().filter(|l| l.brand() == brand);
            let (own, sold): (Vec<_>, Vec<_>) = rows.partition(|l| l.sold_as.is_none());
            (brand, own.into_iter().chain(sold).collect::<Vec<_>>())
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect()
}

/// Generic factory for protocols that implement `Default`.
pub(crate) fn factory<P: Protocol + Default + 'static>() -> Box<dyn Protocol> {
    Box::new(P::default())
}

/// All selectable devices, in registry order.
///
/// Each entry lives in its family's `devices` module; this list is the one
/// place that orders them. Auto-detection probes in this order, and the
/// device lists show each brand's entries in it ([`brand_groups`]).
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
    // EEVblog
    &eevblog121gw::devices::EEVBLOG_121GW,
    // Brymen
    &bm78xbt::devices::BM78XBT,
    &bm86x::devices::BM86X,
    &bm86x::devices::BM82X,
    &bm86x::devices::BM52X,
    // OWON, one entry per model code, the Voltcraft meters on its frame
    // included
    &owon::devices::OW18B,
    &owon::devices::OW18E,
    &owon::devices::B33,
    &owon::devices::B35T_PLUS,
    &owon::devices::B41T_PLUS,
    &owon::devices::CM2100B,
    &owon::devices::CMS101,
    &owon::devices::CMS061,
    &owon::devices::OW65B,
    &owon::devices::OW67B,
    &owon::devices::OW69B,
    &owon::devices::VC871,
    &owon::devices::VC891,
    &owon::devices::VC915,
    &owon::devices::VC925PV,
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
/// A name another brand sells the meter under resolves to the entry it opens.
///
/// Matching stays exact: the verify-gui skill opens any `mock-*` name without
/// asking, safe only while no looser match can reach a hardware entry.
pub fn resolve_device(s: &str) -> Option<&'static SelectableDevice> {
    resolve_listing(s).map(|l| l.device)
}

/// The row a connected meter goes by: the one whose tables decode it.
///
/// `opened` is the entry decoding the meter. The user's pick (`picked`,
/// `None` under Auto-detect) keeps its name while it opens `opened`; a pick
/// of another entry, as a saved one under a replay, gives way to `opened`'s
/// maker's name. A meter that names another model keeps this name too: its
/// readings are still `opened`'s, and [`named_under_pick`] is what the
/// warning offers instead.
pub fn connected_listing(picked: Option<Listing>, opened: &'static SelectableDevice) -> Listing {
    picked
        .filter(|p| p.device.id == opened.id)
        .unwrap_or(opened.listing())
}

/// The row to offer for `named`, the model a meter names when it is not the
/// one picked: under the picked brand when that brand sells it — a UNI-T
/// UT217A pick on a meter reporting the CMS101's code is offered the UNI-T
/// UT217B — else under its maker's name.
pub fn named_under_pick(picked: Option<Listing>, named: &'static SelectableDevice) -> Listing {
    picked
        .and_then(|p| named.listing_under(p.brand()))
        .unwrap_or(named.listing())
}

/// The row whose id is exactly `id`, as a file written by this project names
/// it: no aliases.
pub fn find_listing(id: &str) -> Option<Listing> {
    listings().find(|l| l.id() == id)
}

/// Resolve a device string the way [`resolve_device`] does, to the row it
/// names: "ut217a" is the UNI-T UT217A, opening the OWON CMS061's entry.
pub fn resolve_listing(s: &str) -> Option<Listing> {
    // Try exact ID match first
    if let Some(l) = listings().find(|l| l.id().eq_ignore_ascii_case(s)) {
        return Some(l);
    }
    // Try aliases (case-insensitive)
    listings().find(|l| l.names().any(|a| a.eq_ignore_ascii_case(s)))
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
pub(crate) fn default_device() -> &'static SelectableDevice {
    find_device("ut61eplus").expect("ut61eplus must be in DEVICES")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Stability;

    #[test]
    fn find_device_by_id() {
        let d = find_device("ut61eplus").unwrap();
        assert_eq!(d.display_name(), "UNI-T UT61E+");
        assert!(d.requires_hardware);
    }

    #[test]
    fn find_device_mock() {
        let d = find_device("mock").unwrap();
        assert_eq!(d.display_name(), "Mock UT61E+");
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
        for listing in listings() {
            for name in listing.names() {
                assert!(
                    !name.eq_ignore_ascii_case(AUTO_DEVICE_ID),
                    "{name:?} of {} collides with {AUTO_DEVICE_ID:?}",
                    listing.id()
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

    #[test]
    fn default_device_is_ut61eplus() {
        let d = default_device();
        assert_eq!(d.id, "ut61eplus");
    }

    /// The lists read `ALL` for their order, the derived `Ord` is what a
    /// sort would use; both must be the order the doc comment promises.
    #[test]
    fn brands_are_alphabetical_with_simulated_last() {
        let (simulated, real) = Brand::ALL.split_last().unwrap();
        assert_eq!(*simulated, Brand::Simulated);
        assert!(
            real.windows(2)
                .all(|w| w[0].name().to_lowercase() < w[1].name().to_lowercase()),
            "brands out of alphabetical order: {real:?}"
        );
        assert!(Brand::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn brand_groups_cover_every_row_once() {
        let grouped: Vec<&str> = brand_groups()
            .into_iter()
            .flat_map(|(_, rows)| rows.into_iter().map(|l| l.id()))
            .collect();
        let all: Vec<&str> = listings().map(|l| l.id()).collect();
        assert_eq!(grouped.len(), all.len());
        for id in all {
            assert!(grouped.contains(&id), "{id} not grouped");
        }
    }

    /// Under its heading a row is known by its model name alone, so two of
    /// one brand must not share it.
    #[test]
    fn model_names_are_unique_within_a_brand() {
        for (brand, rows) in brand_groups() {
            let mut names: Vec<&str> = rows.iter().map(|l| l.model_name()).collect();
            assert!(names.iter().all(|n| !n.is_empty()), "{brand:?}: {names:?}");
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), rows.len(), "{brand:?}: duplicate model name");
        }
    }

    /// The model alone is what a heading's row shows, so it never repeats
    /// the brand the display name adds.
    #[test]
    fn models_leave_the_brand_out() {
        for listing in listings() {
            let model = listing.model_name();
            for brand in Brand::ALL {
                let prefix = brand.prefix().unwrap_or("Simulated");
                assert!(
                    !model.to_lowercase().starts_with(&prefix.to_lowercase()),
                    "{}: {model:?}",
                    listing.id()
                );
            }
        }
    }

    #[test]
    fn display_name_is_brand_and_model() {
        let name = |id| resolve_listing(id).unwrap().display_name();
        assert_eq!(name("ut61eplus"), "UNI-T UT61E+");
        assert_eq!(name("ut217a"), "UNI-T UT217A");
        assert_eq!(name("zt5b"), "ZOTEK ZT-5B");
        assert_eq!(name("bside-zt5b"), "BSIDE ZT-5B");
        assert_eq!(name("vc871"), "Voltcraft VC871");
        assert_eq!(name("mock"), "Mock UT61E+");
        assert_eq!(find_device("cms061").unwrap().display_name(), "OWON CMS061");
    }

    #[test]
    fn model_name_drops_the_brand() {
        let model = |id| resolve_listing(id).unwrap().model_name();
        assert_eq!(model("vc880"), "VC-880");
        assert_eq!(model("vc871"), "VC871");
        assert_eq!(model("121gw"), "121GW");
        assert_eq!(model("ut61eplus"), "UT61E+");
        assert_eq!(model("zt5b"), "ZT-5B");
        assert_eq!(model("v05b"), "V05B");
        assert_eq!(model("bm52x"), "BM525s/BM521s");
        assert_eq!(model("et2615a"), "ET-2615A");
        assert_eq!(model("mock"), "Mock UT61E+");
    }

    /// A meter answers to its bare model, and a recording made before the
    /// names dropped their brand still answers to the branded one.
    #[test]
    fn a_reported_name_with_or_without_its_brand_is_the_model() {
        let cms101 = find_device("cms101").unwrap();
        assert!(cms101.answers_to("CMS101"));
        assert!(cms101.answers_to("OWON CMS101"));
        assert!(!cms101.answers_to("CMS061"));
        assert!(find_device("ut61eplus").unwrap().answers_to("ut61e+"));
    }

    #[test]
    fn a_connected_meter_keeps_the_picked_name_while_the_pick_decodes_it() {
        let row = |id| resolve_listing(id).unwrap();
        let entry = |id| find_device(id).unwrap();
        // Auto-detect: the entry that answered, by its maker's name.
        assert_eq!(connected_listing(None, entry("cms061")).id(), "cms061");
        // A pick keeps its own name.
        assert_eq!(
            connected_listing(Some(row("ut217a")), entry("cms061")).id(),
            "ut217a"
        );
        // A pick that opened something else gives way to what opened.
        assert_eq!(
            connected_listing(Some(row("ut217a")), entry("ut61eplus")).id(),
            "ut61eplus"
        );
        // The model a meter names instead is offered under the picked brand.
        assert_eq!(
            named_under_pick(Some(row("ut217a")), entry("cms101")).id(),
            "ut217b"
        );
        assert_eq!(
            named_under_pick(Some(row("an9002")), entry("cms101")).id(),
            "cms101"
        );
        assert_eq!(named_under_pick(None, entry("cms101")).id(), "cms101");
    }

    #[test]
    fn without_brand_leaves_the_model() {
        assert_eq!(without_brand("UNI-T UT61E+"), "UT61E+");
        assert_eq!(without_brand("ZOTEK ZT-5B"), "ZT-5B");
        assert_eq!(without_brand("Mock UT61E+"), "Mock UT61E+");
        assert_eq!(without_brand("Mock (simulated)"), "Mock (simulated)");
    }

    /// A sold-as name lists under its own brand and opens the maker's entry.
    #[test]
    fn a_sold_as_name_opens_the_makers_entry() {
        let ut217a = resolve_listing("ut217a").unwrap();
        assert_eq!(ut217a.display_name(), "UNI-T UT217A");
        assert_eq!(ut217a.brand(), Brand::UniT);
        assert_eq!(ut217a.device.id, "cms061");
        assert_eq!(resolve_device("UT217A").unwrap().id, "cms061");
        assert_eq!(resolve_listing("an-9002").unwrap().id(), "an9002");
        assert_eq!(resolve_device("an-9002").unwrap().id, "zt300ab");
        assert_eq!(
            resolve_listing("cms061").unwrap().display_name(),
            "OWON CMS061"
        );
    }

    #[test]
    fn all_ids_unique() {
        let mut ids: Vec<&str> = listings().map(|l| l.id()).collect();
        let len_before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len_before, "device IDs must be unique");
    }

    #[test]
    fn no_alias_conflicts_with_ids() {
        // Every id and alias answers for its own row and no other, so no
        // name shadows another row's.
        for listing in listings() {
            for name in listing.names() {
                let found = resolve_listing(name).map(|l| l.id());
                assert_eq!(found, Some(listing.id()), "{name:?}");
            }
        }
    }

    #[test]
    fn factory_functions_produce_valid_protocols() {
        for device in DEVICES {
            let protocol = (device.new_protocol)();
            let profile = protocol.profile();
            assert!(!profile.family_name.is_empty(), "device {}", device.id);
        }
    }

    /// Several models share a device table. Each registry entry's protocol
    /// must still carry its own id, or a UT161E introduces itself as a UT61E+
    /// and the user never learns which meter the readings were decoded as.
    #[test]
    fn each_factory_builds_its_own_entrys_protocol() {
        for device in DEVICES {
            assert_eq!((device.new_protocol)().profile().id, device.id);
        }
    }

    /// A confirmed detection names only links the entry lists, and only a
    /// meter that needs hardware has one.
    #[test]
    fn detection_is_verified_on_listed_links_only() {
        for device in DEVICES {
            for link in device.detection_verified {
                assert!(device.links.contains(link), "{}: {link}", device.id);
            }
            if !device.requires_hardware {
                assert!(device.detection_verified.is_empty(), "{}", device.id);
            }
        }
    }

    /// The models real hardware has answered for: our UT61E+, the UT61B+
    /// from two captures reported in issue #19 (2026-09-09 and 2026-09-10),
    /// the UT804 from one reporter's meter (issue #16), which walked every
    /// dial position on 2026-09-18, and the ZT-5B from one reporter's two
    /// captures (issue #31, 2026-10-03), and the UT161E from one reporter's two
    /// captures (issue #7, both 2026-10-21). The UT181A, the Brymen BM82x
    /// from one reporter's BM827s (issue #35, 2026-10-27 and 2026-10-28) and
    /// the Brymen BM52x from the same reporter's BM525s (issue #36,
    /// 2026-10-28) have run for their main modes only, so they are
    /// PartlyVerified (see their profiles). Everything else
    /// must stay flagged so the GUI shows the EXPERIMENTAL badge and links to the
    /// verification issue.
    #[test]
    fn only_hardware_backed_models_are_verified() {
        const VERIFIED: &[&str] = &["ut61eplus", "ut61b+", "ut161e", "ut804", "zt5b"];
        const PARTLY_VERIFIED: &[&str] = &["ut181a", "bm82x", "bm52x"];
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
                !device.bluetooth_only() && device.bluetooth_names.is_empty(),
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
        for listing in listings() {
            for name in listing.names() {
                if name.to_lowercase().starts_with("mock") {
                    assert!(
                        !listing.device.requires_hardware,
                        "{} answers {name}",
                        listing.id()
                    );
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
            ("121GW", "121gw"),
            (" 121gw ", "121gw"),
            ("BM78xBT", "bm78xbt"),
            ("bm78xbt", "bm78xbt"),
        ] {
            assert_eq!(ids(name), [id], "{name:?}");
        }
        for name in [
            "UT-D07B", "UT-D07A", "UT61E+", "UT60", "121", "BM78", "BD", "",
        ] {
            assert!(ids(name).is_empty(), "{name:?}");
        }
    }

    /// OWON's meters all advertise "BDM", padded on air with non-printing
    /// bytes, and the 6-byte ones give "LILLIPUT" as a device name; the
    /// Voltcraft ones may advertise "VC871", "VC891", "VCxxx" or "VC8xx_1"
    /// instead. Each finds its OWON entries in table order, and nothing
    /// else; no other meter's name finds them.
    #[test]
    fn bdm_finds_every_owon_entry() {
        const SIX: [&str; 6] = ["ow18b", "ow18e", "b33", "b35t+", "b41t+", "cm2100b"];
        const OWON_15: [&str; 5] = ["cms101", "cms061", "ow65b", "ow67b", "ow69b"];
        const VOLTCRAFT: [&str; 4] = ["vc871", "vc891", "vc915", "vc925pv"];
        let owon: Vec<&str> = [&SIX[..], &OWON_15, &VOLTCRAFT].concat();
        let ids = |name: &str| advertising(name).iter().map(|d| d.id).collect::<Vec<_>>();
        let nuls = format!("BDM{}", "\0".repeat(12));
        let ones = format!("BDM{}", "\u{1}".repeat(12));
        for name in ["BDM", " bdm ", &nuls, &ones] {
            assert_eq!(ids(name), owon, "{name:?}");
        }
        for name in ["LILLIPUT", "Lilliput"] {
            assert_eq!(ids(name), SIX, "{name:?}");
        }
        for name in ["VC871", "VC891", "VC8xx_1", "vc915", "VC925"] {
            assert_eq!(ids(name), VOLTCRAFT, "{name:?}");
        }
        // No other entry advertises a VC name, nor takes these.
        for name in ["VC", "V", "VC7", "VC650BT", "VC-880", "VC-890"] {
            assert!(ids(name).is_empty(), "{name:?}: {:?}", ids(name));
        }
        for name in [
            "Bluetooth DMM",
            "BD",
            "BM78xBT",
            "121GW",
            "UT60BT",
            "UT202BT",
            "UT-D07A",
            "UT-D07B",
        ] {
            assert!(
                !ids(name).iter().any(|id| owon.contains(id)),
                "{name:?}: {:?}",
                ids(name)
            );
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

    /// A meter that advertises names of its own is on Bluetooth alone, and a
    /// meter on Bluetooth alone has the radio built in: the two read as one
    /// thing today. A meter with both a radio and a cable fails this, and
    /// has to decide what `bluetooth_only()` means for it: the open path,
    /// which never tries USB for such a meter, and the "built-in radio"
    /// wording in both binaries.
    #[test]
    fn a_built_in_radio_means_bluetooth_alone() {
        for device in DEVICES {
            assert_eq!(
                !device.bluetooth_names.is_empty(),
                device.links == [crate::BLUETOOTH],
                "{}",
                device.id
            );
        }
    }
}
