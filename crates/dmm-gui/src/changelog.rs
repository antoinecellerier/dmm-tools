//! Embedded changelog display for the "What's New" popup.
//!
//! The full `CHANGELOG.md` is embedded at compile time via `include_str!`.
//! Rendering uses `egui_commonmark` for proper GitHub-flavored markdown
//! (tables, bold, code, headers, links).

use eframe::egui::Ui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// Returns `true` if the changelog contains a `## v{version}` section.
pub(crate) fn has_version_section(version: &str) -> bool {
    let header = format!("## v{version}");
    CHANGELOG.lines().any(|line| line == header)
}

/// Render the full embedded changelog into the given UI.
pub(crate) fn show_changelog(ui: &mut Ui, cache: &mut CommonMarkCache) {
    CommonMarkViewer::new().show(ui, cache, CHANGELOG);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_section_for_known_version() {
        // v0.1.0 is always in the changelog.
        assert!(has_version_section("0.1.0"));
    }

    #[test]
    fn no_section_for_unknown_version() {
        assert!(!has_version_section("99.99.99"));
    }

    #[test]
    fn no_section_for_dev_version() {
        // Dev versions use "## Unreleased", not "## v0.4.0-dev".
        assert!(!has_version_section("0.4.0-dev"));
    }

    #[test]
    fn changelog_is_embedded() {
        assert!(CHANGELOG.starts_with("# Changelog"));
        assert!(CHANGELOG.contains("## Unreleased"));
    }
}
