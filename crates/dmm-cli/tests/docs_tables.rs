//! Keeps the device tables in `docs/cli-reference.md` and `README.md` honest
//! about the registry.
//!
//! Adding a meter used to mean hand-editing the same ids, aliases, names and
//! issue links in both files, and they drifted. Each file now delimits its
//! table with `<!-- devices:start -->` / `<!-- devices:end -->` (an HTML
//! comment, invisible on GitHub), but the two are treated differently: the CLI
//! reference's block is generated from the registry, while the README's stays
//! hand-written — it is editorial — and is only checked for omissions.
//!
//! `UPDATE_DOCS=1 cargo test -p dmm-cli` rewrites the generated block; a plain
//! run fails with a diff. This lives in `dmm-cli` because that is the binary
//! whose `--help` and reference the tables describe. The table is repository
//! markdown, not terminal help, so it is rendered here rather than in
//! `dmm_shared::help`; the tags come from there, shared with `--help`.

mod common;

use common::{repo_root, unified_diff};
use dmm_lib::protocol::{DeviceFamily, registry};
use std::path::PathBuf;

const START: &str = "<!-- devices:start -->";
const END: &str = "<!-- devices:end -->";

/// The CLI reference's `--device` table: one row per selectable device.
///
/// The Description column is the device name plus the same tag `--help`
/// appends ([`dmm_shared::help::device_tag`]), so the two cannot disagree
/// about which devices are experimental. Counts, form factor and which models
/// share a protocol table stay in `docs/supported-devices.md`, which the
/// reference links to.
fn cli_reference_table() -> String {
    let mut table = String::from("| Value | Aliases | Description |\n|---|---|---|");
    // Auto leads the table and carries the default tag: it is what `--device`
    // does when nothing names a meter, and the one value that is not a
    // registry entry. Its description links to the design the way the rows
    // below send a reader to `supported-devices.md`.
    table.push_str(&format!(
        "\n| `{}` |  | [Detect the connected meter](detection-design.md) (default) |",
        registry::AUTO_DEVICE_ID
    ));
    for device in registry::DEVICES {
        let aliases: Vec<String> = device.aliases.iter().map(|a| format!("`{a}`")).collect();
        let tag = dmm_shared::help::device_tag(device, true).expect("every row is tagged");
        // A display name that already ends in a parenthetical — the mock's
        // "Mock (simulated)" — takes the tag inside it instead of growing a
        // second bracketed group.
        let description = match device.display_name.strip_suffix(')') {
            Some(head) => format!("{head}, {tag})"),
            None => format!("{} ({tag})", device.display_name),
        };
        table.push_str(&format!(
            "\n| `{}` | {} | {description} |",
            device.id,
            aliases.join(", ")
        ));
    }
    table
}

/// A doc's full text and the byte range its marked block spans.
///
/// The markers sit on their own lines, so the block owns the newline after the
/// opening one and the one before the closing one.
fn marked_block(relative: &str) -> (PathBuf, String, std::ops::Range<usize>) {
    let path = repo_root().join(relative);
    let text = std::fs::read_to_string(&path).expect("read doc");
    let start = text.find(START).expect("start marker") + START.len();
    let end = start + text[start..].find(END).expect("end marker");
    (path, text, start..end)
}

/// Each hardware protocol family as `(family_name, model display names)`, in
/// registry order.
fn hardware_families() -> Vec<(&'static str, Vec<&'static str>)> {
    let mut families: Vec<(DeviceFamily, &'static str, Vec<&'static str>)> = Vec::new();
    for device in registry::DEVICES {
        // The README answers "will my meter work"; the mock is not a meter.
        if !device.requires_hardware {
            continue;
        }
        match families.iter_mut().find(|(f, _, _)| *f == device.family) {
            Some((_, _, models)) => models.push(device.display_name),
            None => {
                let family_name = (device.new_protocol)().profile().family_name;
                families.push((device.family, family_name, vec![device.display_name]));
            }
        }
    }
    families
        .into_iter()
        .map(|(_, name, models)| (name, models))
        .collect()
}

#[test]
fn cli_reference_device_table_matches_the_registry() {
    let relative = "docs/cli-reference.md";
    let rendered = cli_reference_table();
    let (path, text, block) = marked_block(relative);
    let wanted = format!("\n{rendered}\n");
    if text[block.clone()] == wanted {
        return;
    }
    if std::env::var_os("UPDATE_DOCS").is_some() {
        let updated = format!("{}{wanted}{}", &text[..block.start], &text[block.end..]);
        dmm_shared::write_atomic(&path, updated.as_bytes()).expect("rewrite doc");
        return;
    }
    panic!(
        "{relative} is out of date with the device registry:\n\n{}\nrun `UPDATE_DOCS=1 cargo test -p dmm-cli` to regenerate",
        unified_diff(relative, &text[block], &wanted, "from the registry")
    );
}

/// The README's table is hand-written on purpose — it abbreviates runs of
/// models ("UT161B/D/E"), prefixes the brand a reader would search for, and
/// gives each link a status no enum holds — so this checks only that nothing
/// is *missing*, and that each row links one verification issue at most.
///
/// The rule is per protocol family, not per device, because those abbreviations
/// mean a model need not appear under its own display name: a family counts as
/// listed when the block names it (its `family_name`) or any one of its models.
/// Every verification issue must be linked, so a new experimental family cannot
/// ship without a row pointing readers at where to help.
#[test]
fn readme_device_table_names_every_family_and_issue() {
    let (_, text, block) = marked_block("README.md");
    let block = &text[block];
    for (family_name, models) in hardware_families() {
        assert!(
            block.contains(family_name) || models.iter().any(|m| block.contains(m)),
            "README device table names neither the {family_name} family nor any of its models {models:?}"
        );
    }
    for device in registry::DEVICES {
        let Some(issue) = (device.new_protocol)().profile().verification_issue else {
            continue;
        };
        // With the closing bracket, so #1 does not match a link to #16.
        assert!(
            block.contains(&format!("/issues/{issue})")),
            "README device table has no link to verification issue #{issue} ({})",
            device.id
        );
    }
    for row in block.lines() {
        assert!(
            row.matches("/issues/").count() <= 1,
            "README device table row links more than one issue: {row}"
        );
    }
}

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
    // The default is detection, not a model: `auto` carries the tag and
    // leads the table, and no meter claims it.
    let first_row = table.lines().nth(2).expect("a first device row");
    assert_eq!(
        first_row,
        "| `auto` |  | [Detect the connected meter](detection-design.md) (default) |"
    );
    assert!(table.contains("| UT61E+ (verified) |"), "{table}");
    assert_eq!(table.matches("default").count(), 1, "{table}");
    assert!(table.contains("| UT171A/B/C (experimental) |"), "{table}");
    assert!(table.contains("| UT181A (partly verified) |"), "{table}");
    assert!(
        table.contains("| Mock (simulated, no hardware required) |"),
        "{table}"
    );
}

/// The Description paragraph links every command section, in the order the
/// sections come.
#[test]
fn cli_reference_description_links_every_command_section() {
    let text = std::fs::read_to_string(repo_root().join("docs/cli-reference.md")).unwrap();
    let sections: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("### dmm-cli "))
        .collect();
    let description = text
        .split("## Description\n")
        .nth(1)
        .and_then(|rest| rest.trim_start().split("\n\n").next())
        .expect("the Description paragraph");
    let linked: Vec<&str> = description
        .split("(#dmm-cli-")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .collect();
    assert_eq!(linked, sections);
}
