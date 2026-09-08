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
//! whose `--help` and reference the tables describe.

use dmm_lib::docs_tables;
use dmm_lib::protocol::{DeviceFamily, registry};
use std::path::{Path, PathBuf};

const START: &str = "<!-- devices:start -->";
const END: &str = "<!-- devices:end -->";

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
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

/// A unified diff of the two blocks, trimmed to the lines that differ.
///
/// Adding one device changes one row; printing the whole table would bury it.
fn unified_diff(name: &str, found: &str, wanted: &str) -> String {
    let found: Vec<&str> = found.lines().collect();
    let wanted: Vec<&str> = wanted.lines().collect();
    let head = found
        .iter()
        .zip(&wanted)
        .take_while(|(a, b)| a == b)
        .count();
    let tail = found[head..]
        .iter()
        .rev()
        .zip(wanted[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let mut diff = format!(
        "--- {name} (in the file)\n+++ {name} (from the registry)\n@@ -{},{} +{},{} @@\n",
        head + 1,
        found.len() - head - tail,
        head + 1,
        wanted.len() - head - tail
    );
    for line in &found[head..found.len() - tail] {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &wanted[head..wanted.len() - tail] {
        diff.push_str(&format!("+{line}\n"));
    }
    diff
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
    let rendered = docs_tables::cli_reference_table();
    let (path, text, block) = marked_block(relative);
    let wanted = format!("\n{rendered}\n");
    if text[block.clone()] == wanted {
        return;
    }
    if std::env::var_os("UPDATE_DOCS").is_some() {
        let updated = format!("{}{wanted}{}", &text[..block.start], &text[block.end..]);
        dmm_settings::write_atomic(&path, updated.as_bytes()).expect("rewrite doc");
        return;
    }
    panic!(
        "{relative} is out of date with the device registry:\n\n{}\nrun `UPDATE_DOCS=1 cargo test -p dmm-cli` to regenerate",
        unified_diff(relative, &text[block], &wanted)
    );
}

/// The README's table is hand-written on purpose — it abbreviates runs of
/// models ("UT161B/D/E"), spells families the way a reader would ("UT803/UT804"
/// rather than the chip name), and carries status wording no enum holds — so
/// this checks only that nothing is *missing*.
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
}
