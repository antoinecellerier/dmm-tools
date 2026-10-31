//! `--help` follows the run's language and the completion scripts don't.
//!
//! A script is installed once and serves every later run, whatever it
//! speaks, so it is written from the English tree. Only a run of the binary
//! can show that: the language is chosen in `main`, which no unit test
//! reaches.

mod common;

use common::dmm_cli;

const PSEUDO: &str = "DMM_TOOLS_PSEUDO_LOCALE";

fn stdout(args: &[&str], pseudo: bool) -> String {
    let mut cmd = dmm_cli();
    if pseudo {
        cmd.env(PSEUDO, "1");
    }
    let out = cmd.args(args).output().expect("run dmm-cli");
    assert!(out.status.success(), "dmm-cli {args:?} failed");
    String::from_utf8(out.stdout).expect("utf-8 stdout")
}

#[test]
fn completions_stay_english_under_the_pseudo_locale() {
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let args = ["completions", shell];
        assert_eq!(stdout(&args, false), stdout(&args, true), "{shell}");
    }
    // zsh's script carries the help, so its equality above means something.
    let zsh = stdout(&["completions", "zsh"], true);
    assert!(zsh.contains("Continuously read measurements"), "{zsh}");
}

#[test]
fn help_speaks_the_pseudo_locale() {
    let help = stdout(&["read", "--help"], true);
    assert!(help.contains("Usage: dmm-cli read"), "{help}");
    assert!(!help.contains("Continuously read measurements"), "{help}");
    assert!(help.starts_with('['), "{help}");
}
