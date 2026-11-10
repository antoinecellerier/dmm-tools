//! Which meter a session opens, from `--device` and the settings file: one
//! rule for both apps, which read the same flag and the same file.

use crate::SharedSettings;
use dmm_lib::protocol::registry::{self, Listing, Selection};

/// The meter a session opens: the row `--device` or the settings file
/// names, or Auto-detect.
#[derive(Clone)]
pub struct DevicePick {
    /// The row picked, `None` for Auto-detect. A sold-as row keeps its own
    /// id (`ut217a`), as the Settings row lists it.
    pub listing: Option<Listing>,
    /// The settings file's `device_family` when no row answers to it: a
    /// hand-edited file, or one a newer build wrote. The session then
    /// detects the meter, and `listing` is `None`.
    pub unknown_saved: Option<String>,
}

impl DevicePick {
    /// The pick as the library opens it.
    pub fn selection(&self) -> Selection {
        self.listing
            .map_or(Selection::Auto, |l| Selection::Device(l.device))
    }

    /// The id the pick is stored under: the row's, or `auto`.
    pub fn id(&self) -> &'static str {
        self.listing.map_or(registry::AUTO_DEVICE_ID, |l| l.id())
    }
}

/// A `--device` value no row answers to, as typed.
#[derive(Debug, PartialEq)]
pub struct UnknownDevice(pub String);

/// The meter `typed` (`--device`) names, else the settings file's, else
/// Auto-detect.
///
/// Only a typed name can be refused: an id in the settings file that no row
/// answers to is reported in [`DevicePick::unknown_saved`] and detected
/// instead, so a file a newer build wrote doesn't stop an older one.
pub fn resolve_device(
    typed: Option<&str>,
    saved: Option<&SharedSettings>,
) -> Result<DevicePick, UnknownDevice> {
    if let Some(raw) = typed {
        return row(raw)
            .map(|listing| DevicePick {
                listing,
                unknown_saved: None,
            })
            .ok_or_else(|| UnknownDevice(raw.to_string()));
    }
    let Some(id) = saved.and_then(SharedSettings::saved_device) else {
        return Ok(DevicePick {
            listing: None,
            unknown_saved: None,
        });
    };
    Ok(match row(id) {
        Some(listing) => DevicePick {
            listing,
            unknown_saved: None,
        },
        None => DevicePick {
            listing: None,
            unknown_saved: Some(id.to_string()),
        },
    })
}

/// The row `name` resolves to, `Some(None)` for `auto`, `None` when nothing
/// answers to it. `auto` is [`registry::resolve_selection`]'s to decide, so
/// both apps spell it as the library does.
fn row(name: &str) -> Option<Option<Listing>> {
    match registry::resolve_selection(name)? {
        Selection::Auto => Some(None),
        Selection::Device(_) => registry::resolve_listing(name).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(family: &str) -> SharedSettings {
        SharedSettings {
            device_family: family.to_string(),
            ..Default::default()
        }
    }

    /// The id each case resolves to, and the unknown saved id it reports.
    fn resolved(
        typed: Option<&str>,
        file: Option<&SharedSettings>,
    ) -> Result<(&'static str, Option<String>), UnknownDevice> {
        resolve_device(typed, file).map(|p| (p.id(), p.unknown_saved))
    }

    #[test]
    fn the_flag_wins_then_the_file_then_auto() {
        let file = saved("ut8803");
        assert_eq!(resolved(Some("ut171"), Some(&file)), Ok(("ut171", None)));
        assert_eq!(resolved(Some("ut171"), None), Ok(("ut171", None)));
        assert_eq!(resolved(None, Some(&file)), Ok(("ut8803", None)));
        assert_eq!(resolved(None, None), Ok(("auto", None)));
    }

    /// A file written before `device_family` existed, or by a GUI where
    /// nothing was picked, holds the empty string: not set, not a device.
    #[test]
    fn an_empty_saved_device_is_auto() {
        assert_eq!(resolved(None, Some(&saved(""))), Ok(("auto", None)));
    }

    /// Aliases and any case resolve to the row; a sold-as name keeps its
    /// own row; `auto` is a choice, however it is spelt.
    #[test]
    fn names_resolve_to_their_listing_row() {
        assert_eq!(resolved(Some("UT61E+"), None), Ok(("ut61eplus", None)));
        assert_eq!(resolved(Some("ut217a"), None), Ok(("ut217a", None)));
        assert_eq!(resolved(Some("cms061"), None), Ok(("cms061", None)));
        assert_eq!(resolved(Some(" AUTO "), None), Ok(("auto", None)));
        assert_eq!(resolved(None, Some(&saved("ut217a"))), Ok(("ut217a", None)));
        assert!(matches!(
            resolve_device(Some("auto"), None).map(|p| p.selection()),
            Ok(Selection::Auto)
        ));
    }

    /// A typed name nothing answers to is refused; a saved one is reported
    /// and detected instead, even under a flag-less run.
    #[test]
    fn only_a_typed_unknown_name_is_refused() {
        let file = saved("nope");
        assert_eq!(
            resolved(Some("nope"), None),
            Err(UnknownDevice("nope".to_string()))
        );
        assert_eq!(
            resolved(Some("nope"), Some(&saved("ut8803"))),
            Err(UnknownDevice("nope".to_string()))
        );
        assert_eq!(
            resolved(None, Some(&file)),
            Ok(("auto", Some("nope".to_string())))
        );
        // A typed name stands in for the file, whose id is then not news.
        assert_eq!(resolved(Some("mock"), Some(&file)), Ok(("mock", None)));
    }
}
