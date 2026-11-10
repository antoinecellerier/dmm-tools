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

/// A mock-only flag given for a session whose device is not simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockOnly {
    /// `--mock-mode`, which pins a scenario of the mock alone.
    MockMode,
    /// `--mock-clock-scale` or `--mock-clock-preseed`, which bend session
    /// time: a meter on a cable is paced by it, and its readings would be
    /// stamped with instants it never produced.
    Clock,
}

/// Whether the mock-only flags fit the session's device, `selection` as
/// typed or saved: `--mock-mode` takes the mock, the clock flags any
/// simulated meter. Nothing implies the mock, so Auto-detect refuses both.
/// A replay is the session's device instead, and is not checked here.
pub fn mock_flags_fit(selection: Selection, mock_mode: bool, clock: bool) -> Result<(), MockOnly> {
    let device = match selection {
        Selection::Device(device) => Some(device),
        Selection::Auto => None,
    };
    if mock_mode && device.is_none_or(|d| d.id != dmm_lib::mock::MOCK.id) {
        return Err(MockOnly::MockMode);
    }
    if clock && device.is_none_or(|d| d.requires_hardware) {
        return Err(MockOnly::Clock);
    }
    Ok(())
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

    /// `--mock-mode` takes the mock, typed or saved; the clock flags any
    /// simulated meter; neither a meter on a cable, nor Auto-detect.
    #[test]
    fn the_mock_only_flags_need_a_simulated_device() {
        let pick = |name: &str| resolve_device(Some(name), None).unwrap().selection();
        let saved = |name: &str| {
            let file = saved(name);
            resolve_device(None, Some(&file)).unwrap().selection()
        };
        for mock in [pick("mock"), saved("mock")] {
            assert_eq!(mock_flags_fit(mock, true, true), Ok(()));
        }
        for zt5b in [pick("mock-zt5b"), saved("mock-zt5b")] {
            assert_eq!(mock_flags_fit(zt5b, false, true), Ok(()));
            assert_eq!(mock_flags_fit(zt5b, true, false), Err(MockOnly::MockMode));
        }
        for real in [
            pick("ut61eplus"),
            saved("ut61eplus"),
            pick("auto"),
            saved(""),
            saved("nope"),
        ] {
            assert_eq!(mock_flags_fit(real, false, false), Ok(()));
            assert_eq!(mock_flags_fit(real, true, false), Err(MockOnly::MockMode));
            assert_eq!(mock_flags_fit(real, false, true), Err(MockOnly::Clock));
            // The first flag the rule names is the one refused.
            assert_eq!(mock_flags_fit(real, true, true), Err(MockOnly::MockMode));
        }
    }
}
