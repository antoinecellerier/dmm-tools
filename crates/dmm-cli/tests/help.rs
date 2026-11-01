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
    // clap's own words go through the messages too: none is left English.
    assert!(help.contains("dmm-cli read [OPTIONS]"), "{help}");
    assert!(
        !help.contains("Usage:") && !help.contains("Options:"),
        "{help}"
    );
    assert!(!help.contains("Continuously read measurements"), "{help}");
    assert!(help.starts_with('['), "{help}");
}

/// A folder for `DMM_TOOLS_L10N_DIR` with two languages the build lacks,
/// each naming itself in the top-level help and in `info`'s: what
/// `--lang`, the settings file and the environment can then pick from.
fn languages_dir() -> &'static std::path::Path {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("dmm-cli-languages");
        for (id, name) in [("de", "Deutsch"), ("it", "Italiano")] {
            std::fs::create_dir_all(dir.join(id)).unwrap();
            let ftl = format!("dmm-cli =\n    .about = {name}\ninfo =\n    .about = {name} info\n");
            std::fs::write(dir.join(id).join("cli-help.ftl"), ftl).unwrap();
        }
        dir
    })
}

/// `dmm-cli` able to speak the languages of [`languages_dir`].
fn multilingual() -> std::process::Command {
    let mut cmd = dmm_cli();
    cmd.env("DMM_TOOLS_L10N_DIR", languages_dir());
    cmd
}

/// The first line of the help `cmd` prints for `args`: the about.
fn about(mut cmd: std::process::Command, args: &[&str]) -> String {
    let out = cmd.args(args).output().expect("run dmm-cli");
    assert!(out.status.success(), "dmm-cli {args:?} failed");
    let text = String::from_utf8(out.stdout).expect("utf-8 stdout");
    text.lines().next().unwrap_or_default().to_string()
}

#[test]
fn lang_picks_the_help_language_wherever_it_stands() {
    assert_eq!(
        about(multilingual(), &["--lang", "de", "--help"]),
        "Deutsch"
    );
    assert_eq!(about(multilingual(), &["--lang=it", "-h"]), "Italiano");
    assert_eq!(
        about(multilingual(), &["info", "--lang", "DE", "--help"]),
        "Deutsch info"
    );
    assert_eq!(
        about(multilingual(), &["--lang", "en", "--help"]),
        "CLI tool for USB and Bluetooth digital multimeters"
    );
}

#[test]
fn an_unknown_lang_is_refused_with_the_languages_on_offer() {
    let out = multilingual()
        .args(["--lang", "xx", "list"])
        .output()
        .expect("run dmm-cli");
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8(out.stderr).expect("utf-8 stderr");
    assert!(
        err.contains("invalid value 'xx' for '--lang <LANG>'"),
        "{err}"
    );
    assert!(
        err.contains("[possible values: en-US, fr, pt-BR, zh-CN, es, de, it, system]"),
        "{err}"
    );
}

/// `--lang` beats the settings file's `language`, which beats the
/// environment. Linux only: elsewhere the settings file isn't under
/// `XDG_CONFIG_HOME`, and the system's language isn't the environment's.
#[cfg(target_os = "linux")]
#[test]
fn lang_beats_the_setting_which_beats_the_environment() {
    let run = |setting: Option<&str>, args: &[&str]| {
        let name = format!("dmm-cli-lang-{}", setting.unwrap_or("none"));
        let config = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        std::fs::create_dir_all(config.join("dmm-tools")).unwrap();
        if let Some(language) = setting {
            let json = format!(r#"{{"language":"{language}"}}"#);
            std::fs::write(config.join("dmm-tools/settings.json"), json).unwrap();
        }
        let mut cmd = multilingual();
        cmd.env("XDG_CONFIG_HOME", &config)
            .env("HOME", &config)
            .env_remove("LC_ALL")
            .env("LANG", "de_DE.UTF-8");
        about(cmd, args)
    };
    assert_eq!(run(None, &["-h"]), "Deutsch");
    assert_eq!(run(Some("it"), &["-h"]), "Italiano");
    assert_eq!(run(Some("it"), &["--lang", "de", "-h"]), "Deutsch");
    assert_eq!(run(Some("de"), &["--lang", "it", "-h"]), "Italiano");
    // `system` passes over the settings file.
    assert_eq!(run(Some("it"), &["--lang", "system", "-h"]), "Deutsch");
    // A setting the build lacks falls through to the environment.
    assert_eq!(run(Some("fi"), &["-h"]), "Deutsch");
}
