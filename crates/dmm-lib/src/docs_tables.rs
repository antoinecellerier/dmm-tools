//! The `--device` table `docs/cli-reference.md` publishes, rendered from the
//! registry. A `dmm-cli` test compares the file's `<!-- devices:start -->` /
//! `<!-- devices:end -->` block against [`cli_reference_table`] and rewrites
//! it under `UPDATE_DOCS=1`, so the ids, aliases and names cannot drift from
//! the registry. The README's table stays hand-written (it is editorial) and
//! is only checked for omissions. Not in [`crate::binary_help`], which is
//! terminal help text, not repository markdown.

use crate::protocol::{Stability, registry};

/// The CLI reference's `--device` table: one row per selectable device.
///
/// The Description column is the device name plus the same tags `--help`
/// appends, so the two cannot disagree about which devices are experimental.
/// Counts, form factor and which models share a protocol table stay in
/// `docs/supported-devices.md`, which the reference links to.
pub fn cli_reference_table() -> String {
    let default_id = registry::default_device().id;
    let mut table = String::from("| Value | Aliases | Description |\n|---|---|---|");
    for device in registry::DEVICES {
        let aliases: Vec<String> = device.aliases.iter().map(|a| format!("`{a}`")).collect();
        let mut tags: Vec<&str> = Vec::new();
        if device.id == default_id {
            tags.push("default");
        }
        if !device.requires_hardware {
            tags.push("no hardware required");
        } else if (device.new_protocol)().profile().stability == Stability::Verified {
            tags.push("verified");
        } else {
            tags.push("experimental");
        }
        let tags = tags.join(", ");
        // A display name that already ends in a parenthetical — the mock's
        // "Mock (simulated)" — takes the tags inside it instead of growing a
        // second bracketed group.
        let description = match device.display_name.strip_suffix(')') {
            Some(head) => format!("{head}, {tags})"),
            None => format!("{} ({tags})", device.display_name),
        };
        table.push_str(&format!(
            "\n| `{}` | {} | {description} |",
            device.id,
            aliases.join(", ")
        ));
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_reference_table_lists_every_id_and_alias() {
        let table = cli_reference_table();
        // Pasted into markdown verbatim, so a stray leading or trailing
        // newline would move the `<!-- devices:end -->` marker.
        assert!(
            !table.starts_with('\n') && !table.ends_with('\n'),
            "{table}"
        );
        for line in table.lines() {
            assert!(line.starts_with("| ") && line.ends_with(" |") || line == "|---|---|---|");
            assert_eq!(line.matches('|').count(), 4, "{line}");
        }
        for device in registry::DEVICES {
            assert!(table.contains(&format!("`{}`", device.id)), "{}", device.id);
            for alias in device.aliases {
                assert!(table.contains(&format!("`{alias}`")), "{alias}");
            }
        }
    }

    /// `--help` and the reference table tag devices the same way; a mismatch
    /// would have a user reading "verified" in the docs and a yellow warning
    /// on the terminal. The mock also checks the tags land inside a display
    /// name that already carries a parenthetical.
    #[test]
    fn cli_reference_tags_default_experimental_and_mock() {
        let table = cli_reference_table();
        assert!(table.contains("| UT61E+ (default, verified) |"), "{table}");
        assert!(table.contains("| UT171A/B/C (experimental) |"), "{table}");
        assert!(
            table.contains("| Mock (simulated, no hardware required) |"),
            "{table}"
        );
    }
}
