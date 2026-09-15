//! Helpers shared by the doc-checking tests (`docs_tables`, `docs_snippets`).

use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest directory.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

/// A unified diff of the two blocks, trimmed to the lines that differ, with
/// `source` naming where the wanted text came from.
///
/// Adding one device changes one row and a snippet usually moves one line;
/// printing the whole block would bury it.
pub fn unified_diff(name: &str, found: &str, wanted: &str, source: &str) -> String {
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
        "--- {name} (in the file)\n+++ {name} ({source})\n@@ -{},{} +{},{} @@\n",
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
