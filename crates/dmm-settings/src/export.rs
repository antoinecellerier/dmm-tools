//! What an export is called, wherever it is written from.
//!
//! `dmm-gui`'s Export… and `dmm-cli read -o` (given without a file name) both
//! open on the same name, so a folder holding exports from either sorts as one
//! set rather than two.

use chrono::{DateTime, Local};

/// The name an export opens with: the meter, the mode it stayed in and the
/// moment the recording started, as
/// `measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv`, so a folder of exports
/// sorts by meter and by run. A recording that crossed a function switch has
/// no one mode and leaves that segment out.
pub fn default_name(
    model: &str,
    mode: Option<&str>,
    start: DateTime<Local>,
    extension: &str,
) -> String {
    let mode = mode
        .map(|m| format!("{}-", file_safe(m)))
        .unwrap_or_default();
    format!(
        "measurements-{}-{mode}{}.{extension}",
        file_safe(model),
        start.format("%Y-%m-%d_%H-%M-%S"),
    )
}

/// A meter or mode name as one file-name word: runs of whitespace become a
/// single `-` and the separators a path could read drop out, so "Mock
/// UT61E+" exports as `Mock-UT61E+`.
pub fn file_safe(name: &str) -> String {
    name.replace(['/', '\\', ':'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn start() -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, 15, 14, 30, 5)
            .single()
            .expect("a fixed local timestamp")
    }

    #[test]
    fn a_name_carries_the_meter_the_mode_and_the_start() {
        assert_eq!(
            default_name("UT61E+", Some("DC V"), start(), "csv"),
            "measurements-UT61E+-DC-V-2026-09-15_14-30-05.csv"
        );
        // A recording with no one mode keeps meter and time, nothing between.
        assert_eq!(
            default_name("UT61E+", None, start(), "replay"),
            "measurements-UT61E+-2026-09-15_14-30-05.replay"
        );
    }

    /// A model or mode name goes into the file name as one word: the dialog
    /// opens on a name the user can save as typed, not one carrying a path
    /// separator.
    #[test]
    fn a_name_is_folded_into_one_file_name_word() {
        assert_eq!(file_safe("Mock UT61E+"), "Mock-UT61E+");
        assert_eq!(file_safe("UT61E+ / UT61B+"), "UT61E+-UT61B+");
        assert_eq!(file_safe("DC V"), "DC-V");
        assert_eq!(file_safe("\u{3a9}"), "\u{3a9}");
    }
}
