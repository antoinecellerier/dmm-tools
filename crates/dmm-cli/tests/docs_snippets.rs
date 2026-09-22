//! Keeps the program-output blocks in `README.md` and `docs/cli-reference.md`
//! as the binary actually prints them.
//!
//! Those blocks used to be typed by hand from a bench session, so they drifted
//! as the output changed and one of them was elided with `…` rather than
//! rerun. Each block now sits between `<!-- snippet via=… -->` and
//! `<!-- /snippet -->` (HTML comments, invisible on GitHub) with the commands
//! in the opening marker: the doc names the command, the test runs it and
//! writes the answer back.
//!
//! `via=mock[:<mock-mode>]` runs the command against the mock device,
//! `via=<name>.replay` against `assets/replays/<name>.replay`, so a block is
//! the same on every machine and needs no meter. The doc shows the command as
//! a user with a meter on the cable would type it — neither flag appears in
//! the block.
//!
//! `UPDATE_DOCS=1 cargo test -p dmm-cli` rewrites a stale block; a plain run
//! fails with a diff. The run pins `TZ=UTC`, which chrono's `Local` ignores on
//! Windows, so the test is Linux and macOS only.
#![cfg(not(windows))]

mod common;

use common::{repo_root, unified_diff};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The docs whose snippets are generated. Every marker pair in each is checked.
const DOCS: [&str; 2] = ["README.md", "docs/cli-reference.md"];

const START: &str = "<!-- snippet ";
/// Ends the opening marker, which sits on its own line after the commands.
const HEADER_END: &str = "\n-->";
const END: &str = "<!-- /snippet -->";

/// Where a snippet's command gets its readings from.
enum Via {
    /// The mock device, optionally pinned to one of its modes.
    Mock(Option<String>),
    /// A recording under `assets/replays/`.
    Replay(PathBuf),
}

/// The 1-based line a byte offset falls on, for an error naming the marker.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

/// Split a command line into arguments, honouring double quotes so a choice
/// with a space in it (`set mode "AC V Hz"`) stays one argument. Nothing else
/// of a shell is supported — the docs quote, and never expand.
fn split_command(line: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

/// `--mock-mode` is an option of `read`, `get` and `set`; `command` has none,
/// a button press being the same whatever the dial reads.
fn takes_mock_mode(args: &[String]) -> bool {
    args.iter()
        .find(|a| !a.starts_with('-'))
        .is_some_and(|sub| matches!(sub.as_str(), "read" | "get" | "set"))
}

/// Read `via=…` from the first line of an opening marker.
fn parse_via(spec: &str, where_: &str) -> Via {
    let spec = spec
        .strip_prefix("via=")
        .unwrap_or_else(|| panic!("{where_}: a snippet marker needs `via=mock[:<mode>]` or `via=<name>.replay`, got `{spec}`"));
    if spec == "mock" {
        return Via::Mock(None);
    }
    if let Some(mode) = spec.strip_prefix("mock:") {
        return Via::Mock(Some(mode.to_string()));
    }
    if let Some(name) = spec.strip_suffix(".replay") {
        let path = repo_root()
            .join("assets/replays")
            .join(format!("{name}.replay"));
        assert!(
            path.exists(),
            "{where_}: no recording at {}",
            path.display()
        );
        return Via::Replay(path);
    }
    panic!("{where_}: `via={spec}` is neither `mock[:<mode>]` nor `<name>.replay`");
}

/// Run one documented command and return what a user would see: stdout, then
/// the stderr the same terminal would interleave (the `--- N samples` summary
/// lives there), with trailing whitespace and blank lines trimmed.
fn output_of(command: &str, via: &Via, where_: &str) -> String {
    let mut args = split_command(command);
    assert!(
        args.first().is_some_and(|a| a == "dmm-cli"),
        "{where_}: a snippet command starts with `dmm-cli`, got `{command}`"
    );
    args.remove(0);
    match via {
        Via::Mock(mode) => {
            let mode = mode.clone().filter(|_| takes_mock_mode(&args));
            // `--device` is global, so it goes before the subcommand.
            args.splice(0..0, ["--device".to_string(), "mock".to_string()]);
            if let Some(mode) = mode {
                args.extend(["--mock-mode".to_string(), mode]);
            }
        }
        Via::Replay(path) => args.extend([
            "--replay".to_string(),
            path.to_str().expect("utf-8 path").to_string(),
        ]),
    }

    let out = Command::new(env!("CARGO_BIN_EXE_dmm-cli"))
        .args(&args)
        .current_dir(repo_root())
        .env("NO_COLOR", "1")
        .env("TZ", "UTC")
        .env_remove("RUST_LOG")
        .output()
        .expect("run dmm-cli");
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8(out.stderr).expect("utf-8 stderr");
    assert!(
        out.status.success(),
        "{where_}: `{command}` exited with {}:\n{stderr}",
        out.status
    );

    let mut lines: Vec<&str> = stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim_end)
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// The fenced block a marker's commands render to: `$ <command>` then its
/// output, one blank line between commands.
fn render(commands: &[String], via: &Via, where_: &str) -> String {
    let mut block = String::from("```\n");
    for (i, command) in commands.iter().enumerate() {
        if i > 0 {
            block.push('\n');
        }
        block.push_str(&format!("$ {command}\n"));
        block.push_str(&output_of(command, via, where_));
    }
    block.push_str("```");
    block
}

/// Check every snippet in one doc, rewriting the stale ones under
/// `UPDATE_DOCS=1` and collecting a diff per block otherwise.
fn check_doc(relative: &str) -> Vec<String> {
    let path = repo_root().join(relative);
    let text = std::fs::read_to_string(&path).expect("read doc");
    let mut updated = String::new();
    let mut copied = 0;
    let mut diffs = Vec::new();

    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(START) {
        let marker = cursor + offset;
        let where_ = format!("{relative}:{}", line_of(&text, marker));
        let header_start = marker + START.len();
        let header_len = text[header_start..]
            .find(HEADER_END)
            .unwrap_or_else(|| panic!("{where_}: `<!-- snippet` marker is never closed by `-->`"));
        let mut header = text[header_start..header_start + header_len].lines();
        let via = parse_via(header.next().unwrap_or_default().trim(), &where_);
        let commands: Vec<String> = header
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        assert!(
            !commands.is_empty(),
            "{where_}: the marker names no command"
        );

        // The markers sit on their own lines, so the block owns the newline
        // after the opening one and the one before the closing one.
        let block_start = header_start + header_len + HEADER_END.len();
        let block_len = text[block_start..].find(END).unwrap_or_else(|| {
            panic!("{where_}: `<!-- snippet` marker has no `<!-- /snippet -->`")
        });
        let block = block_start..block_start + block_len;
        cursor = block.end + END.len();

        let wanted = format!("\n{}\n", render(&commands, &via, &where_));
        if text[block.clone()] == wanted {
            continue;
        }
        diffs.push(unified_diff(
            &where_,
            &text[block.clone()],
            &wanted,
            "from the binary",
        ));
        updated.push_str(&text[copied..block.start]);
        updated.push_str(&wanted);
        copied = block.end;
    }

    if !diffs.is_empty() && std::env::var_os("UPDATE_DOCS").is_some() {
        updated.push_str(&text[copied..]);
        dmm_shared::write_atomic(Path::new(&path), updated.as_bytes()).expect("rewrite doc");
        return Vec::new();
    }
    diffs
}

#[test]
fn doc_snippets_match_what_the_binary_prints() {
    let diffs: Vec<String> = DOCS.iter().flat_map(|doc| check_doc(doc)).collect();
    assert!(
        diffs.is_empty(),
        "documented output is out of date:\n\n{}\nrun `UPDATE_DOCS=1 cargo test -p dmm-cli` to regenerate",
        diffs.join("\n")
    );
}
