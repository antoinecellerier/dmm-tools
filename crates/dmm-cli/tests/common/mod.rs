//! Helpers shared by the integration tests. Each test compiles its own copy
//! and uses only some of them, hence the `dead_code` allowance.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `dmm-cli` binary, run with its settings lookup pointed at an empty
/// directory, so a developer's own `settings.json` (a pinned meter, say)
/// cannot change what the tests see. `XDG_CONFIG_HOME` covers Linux, `HOME`
/// macOS and `APPDATA` Windows.
///
/// In English, whatever the developer's desktop speaks: the C locale means
/// English on every platform, and the other locale variables are cleared so
/// none of them is read first.
pub fn dmm_cli() -> Command {
    let config = Path::new(env!("CARGO_TARGET_TMPDIR")).join("dmm-cli-no-settings");
    std::fs::create_dir_all(&config).expect("create the empty config dir");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dmm-cli"));
    cmd.env("XDG_CONFIG_HOME", &config)
        .env("HOME", &config)
        .env("APPDATA", &config)
        .env("LC_ALL", "C")
        .env_remove("LC_MESSAGES")
        .env_remove("LANG")
        .env_remove("LANGUAGE")
        .env_remove("DMM_TOOLS_PSEUDO_LOCALE")
        .env_remove("DMM_TOOLS_L10N_DIR");
    cmd
}

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
