//! Embedded changelog display for the "What's New" popup.
//!
//! The full `CHANGELOG.md` is embedded at compile time via `include_str!`.
//! Rendering uses `egui_commonmark` for proper GitHub-flavored markdown
//! (tables, bold, code, headers, links). Each release is its own folding
//! section, and only the open ones are parsed and laid out each frame.

use eframe::egui::{self, RichText, TextStyle, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::sync::LazyLock;

const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// One `## ` section of the changelog: its heading without the `## ` and the
/// markdown up to the next one.
struct Section {
    heading: &'static str,
    body: &'static str,
}

/// The changelog split at its `## ` headings, once. The `# Changelog` title
/// line before the first one is dropped. The file has no code fences, so a
/// line-start split is safe.
fn sections() -> &'static [Section] {
    static SECTIONS: LazyLock<Vec<Section>> = LazyLock::new(|| split_sections(CHANGELOG));
    &SECTIONS
}

fn split_sections(text: &'static str) -> Vec<Section> {
    let mut starts: Vec<usize> = text.match_indices("\n## ").map(|(i, _)| i + 1).collect();
    if text.starts_with("## ") {
        starts.insert(0, 0);
    }
    starts
        .iter()
        .enumerate()
        .map(|(k, &start)| {
            let end = starts.get(k + 1).copied().unwrap_or(text.len());
            let chunk = &text[start + 3..end];
            let (heading, body) = chunk.split_once('\n').unwrap_or((chunk, ""));
            Section {
                heading: heading.trim_end(),
                body,
            }
        })
        .collect()
}

/// Whether `heading` (without `## `) is the section for `version`: `v{version}`,
/// optionally followed by ` — Tagline`.
fn is_version_heading(heading: &str, version: &str) -> bool {
    heading
        .strip_prefix('v')
        .and_then(|h| h.strip_prefix(version))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(" — "))
}

/// Whether the section under `heading` opens expanded for this build: its own
/// release, or "Unreleased" in a `-dev` build.
fn opens_expanded(heading: &str, version: &str) -> bool {
    if version.contains("-dev") {
        heading == "Unreleased"
    } else {
        is_version_heading(heading, version)
    }
}

/// Returns `true` if the changelog contains a `## v{version}` section.
/// The heading may carry a tagline: `## v{version} — Tagline`.
pub(crate) fn has_version_section(version: &str) -> bool {
    sections()
        .iter()
        .any(|s| is_version_heading(s.heading, version))
}

/// Render the embedded changelog into the given UI: one folding section per
/// release, the one for `version` open (the first if none matches).
pub(crate) fn show_changelog(ui: &mut Ui, cache: &mut CommonMarkCache, version: &str) {
    let all = sections();
    let open = all
        .iter()
        .find(|s| opens_expanded(s.heading, version))
        .or(all.first())
        .map(|s| s.heading);
    // The size egui_commonmark gives a `##` heading (its `Style::to_richtext`),
    // so a folded header reads as the heading it replaces.
    let style = ui.style();
    let body = style
        .text_styles
        .get(&TextStyle::Body)
        .map_or(14.0, |f| f.size);
    let big = style
        .text_styles
        .get(&TextStyle::Heading)
        .map_or(32.0, |f| f.size);
    let size = body + (big - body) * 0.835;
    for s in all {
        // Pre-wrapped: a CollapsingHeader lays its text out on one line.
        let wrap = ui.available_width() - ui.spacing().indent - ui.spacing().button_padding.x;
        let galley = egui::WidgetText::from(RichText::new(s.heading).strong().size(size))
            .into_galley(ui, Some(egui::TextWrapMode::Wrap), wrap, TextStyle::Button);
        let section = egui::CollapsingHeader::new(egui::WidgetText::Galley(galley))
            .id_salt(("whats_new_section", s.heading))
            .default_open(Some(s.heading) == open)
            .show(ui, |ui| {
                CommonMarkViewer::new().show(ui, cache, s.body);
            });
        let id = section.header_response.id;
        let is_open = egui::collapsing_header::CollapsingState::load(ui.ctx(), id)
            .is_some_and(|state| state.is_open());
        ui.ctx()
            .accesskit_node_builder(id, |builder| builder.set_expanded(is_open));
        crate::a11y::paint_focus_ring(ui, &section.header_response);
    }
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
    fn has_section_for_heading_with_tagline() {
        // v0.3.0's heading is "## v0.3.0 — Specifications, ...".
        assert!(has_version_section("0.3.0"));
    }

    #[test]
    fn no_section_for_unknown_version() {
        assert!(!has_version_section("99.99.99"));
    }

    #[test]
    fn no_section_for_version_prefix() {
        // "## v0.1" is a prefix of "## v0.1.0" but not a heading for 0.1.
        assert!(!has_version_section("0.1"));
    }

    #[test]
    fn no_section_for_dev_version() {
        // Dev versions use "## Unreleased", not "## v0.7.0-dev".
        assert!(!has_version_section("0.7.0-dev"));
    }

    #[test]
    fn changelog_is_embedded() {
        assert!(CHANGELOG.starts_with("# Changelog"));
        assert!(CHANGELOG.contains("## v0.1.0"));
    }

    #[test]
    fn sections_cover_every_release_heading_in_order() {
        let headings: Vec<_> = sections().iter().map(|s| s.heading).collect();
        let lines: Vec<_> = CHANGELOG
            .lines()
            .filter_map(|l| l.strip_prefix("## "))
            .collect();
        assert_eq!(headings, lines);
        assert!(headings.last().unwrap().starts_with("v0.1.0"));
        let mut unique = headings.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), headings.len());
    }

    #[test]
    fn opens_expanded_matches_only_its_version() {
        let v070 = "v0.7.0 — Auto-Detection, Meter Control and Newly Verified Models";
        assert!(opens_expanded("Unreleased", "0.8.0-dev"));
        assert!(opens_expanded(v070, "0.7.0"));
        assert!(!opens_expanded(v070, "0.7.1"));
        assert!(!opens_expanded("v0.1.0 — First Release", "0.1"));
    }

    /// Two frames of What's New as a `version` build shows it, in the
    /// viewport's default size, and the accessibility nodes of the last one.
    fn changelog_nodes(version: &str) -> Vec<egui::accesskit::Node> {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(520.0, 480.0));
        let mut cache = CommonMarkCache::default();
        let mut nodes = Vec::new();
        for _ in 0..2 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_changelog(ui, &mut cache, version);
                    });
                },
            );
            // epaint 0.36 debug-asserts on dropping unapplied texture deltas;
            // this harness renders without a painter.
            out.textures_delta.clear();
            nodes = out
                .platform_output
                .accesskit_update
                .map(|update| update.nodes.into_iter().map(|(_, n)| n).collect())
                .unwrap_or_default();
        }
        nodes
    }

    #[test]
    fn a_release_opens_on_its_own_section_with_the_rest_folded() {
        // An explicit release: the workspace is a `-dev` build, which would
        // open Unreleased, and Unreleased has no "Full Changelog" line.
        let nodes = changelog_nodes("0.7.0");
        let header = |prefix: &str| {
            nodes
                .iter()
                .find(|n| n.label().is_some_and(|l| l.starts_with(prefix)))
                .unwrap_or_else(|| panic!("no header node for {prefix}"))
        };
        // Folded sections stay in the tree, so a screen reader can reach them.
        for s in sections() {
            header(s.heading);
        }
        // Six released sections carry the line; only the open one is laid out.
        let full_changelog = nodes
            .iter()
            .filter(|n| n.value() == Some("Full Changelog"))
            .count();
        assert_eq!(full_changelog, 1);
        assert_eq!(header("v0.7.0").is_expanded(), Some(true));
        assert_eq!(header("v0.6.0").is_expanded(), Some(false));
    }
}
