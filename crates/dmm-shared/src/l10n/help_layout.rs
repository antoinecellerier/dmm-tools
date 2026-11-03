//! The words clap puts around `--help` in the user's language: the "Usage:",
//! "Commands:", "Arguments:" and "Options:" headings, `[default: …]` and
//! `[possible values: …]` after an option, the long help's "Possible
//! values:" list, and the help of clap's own `-h`, `-V` and `help`. The same
//! in dmm-cli and dmm-gui, so the messages are in `shared.ftl`.
//!
//! clap writes these in English with no hook to change them, so [`localize`]
//! takes them over once clap has built the tree (which is when it adds its
//! own flags): a help template per command with the headings worded, and
//! each option's defaults and values hidden from clap and written after its
//! help the way clap lays them out. English keeps clap's own layout
//! untouched; each binary's test lays its tree out this way in English too
//! and compares ([`english_layout_differences`]), so the two stay alike.

use std::fmt::Write as _;

use super::Localizer;
use crate::tr;
use clap::builder::styling::Styles;
use clap::{Arg, Command};

/// `cmd`'s help layout in `l`'s language; English is left to clap.
pub fn localize(cmd: Command, l: &Localizer) -> Command {
    if l.plain_english() {
        return cmd;
    }
    lay_out(invoked_as(cmd), l)
}

/// `cmd` named as the binary was run (`dmm-cli.exe` on Windows), as clap
/// names it once it parses the command line: building the tree before that
/// would fix the subcommands' usage lines to the command's own name.
pub fn invoked_as(cmd: Command) -> Command {
    let name = std::env::args_os().next().and_then(|arg| {
        std::path::Path::new(&arg)
            .file_name()
            .and_then(|n| n.to_str().map(str::to_string))
    });
    match name {
        Some(name) if cmd.get_bin_name().is_none() => cmd.bin_name(name),
        _ => cmd,
    }
}

/// [`localize`], whatever the language.
fn lay_out(mut cmd: Command, l: &Localizer) -> Command {
    cmd.build();
    command(cmd, l, &Styles::styled())
}

/// One built command and its subcommands.
fn command(mut cmd: Command, l: &Localizer, styles: &Styles) -> Command {
    cmd = cmd.mut_args(|a| arg(a, l, styles));
    if cmd.get_arguments().any(|a| a.get_id() == "help") {
        cmd = cmd.mut_arg("help", |a| {
            if a.get_long_help().is_some() {
                a.help(tr!(l, "help-flag-short").into_owned())
                    .long_help(tr!(l, "help-flag-long").into_owned())
            } else {
                a.help(tr!(l, "help-flag").into_owned())
            }
        });
    }
    if cmd.get_arguments().any(|a| a.get_id() == "version") {
        cmd = cmd.mut_arg("version", |a| {
            a.help(tr!(l, "help-version-flag").into_owned())
        });
    }
    let template = template(&cmd, l, styles);
    cmd = cmd.help_template(template);
    cmd.mut_subcommands(|c| {
        let c = if c.get_name() == "help" && !c.is_hide_set() && c.get_about().is_some() {
            c.about(tr!(l, "help-command").into_owned())
                .mut_args(|a| a.help(tr!(l, "help-command-subcommand").into_owned()))
        } else {
            c
        };
        command(c, l, styles)
    })
}

/// `{all-args}` spelled out with worded headings, sections only where
/// clap would write them.
fn template(cmd: &Command, l: &Localizer, styles: &Styles) -> String {
    let (u, h) = (styles.get_usage(), styles.get_header());
    let shown = |a: &&Arg| !a.is_hide_set() && a.get_help_heading().is_none();
    let mut sections = Vec::new();
    if cmd.get_subcommands().any(|c| !c.is_hide_set()) {
        sections.push(format!(
            "{h}{}{h:#}\n{{subcommands}}",
            tr!(l, "help-commands")
        ));
    }
    if cmd.get_positionals().any(|a| shown(&a)) {
        sections.push(format!(
            "{h}{}{h:#}\n{{positionals}}",
            tr!(l, "help-arguments")
        ));
    }
    if cmd.get_arguments().any(|a| !a.is_positional() && shown(&a)) {
        sections.push(format!("{h}{}{h:#}\n{{options}}", tr!(l, "help-options")));
    }
    let mut out = format!(
        "{{before-help}}{{about-with-newline}}\n{u}{}{u:#} {{usage}}",
        tr!(l, "help-usage")
    );
    if !sections.is_empty() {
        out.push_str("\n\n");
        out.push_str(&sections.join("\n\n"));
    }
    out.push_str("{after-help}");
    out
}

/// `arg` with its default and possible values written by us, after its
/// help, where clap would write them.
fn arg(mut arg: Arg, l: &Localizer, styles: &Styles) -> Arg {
    if arg.is_hide_set() {
        return arg;
    }
    let (ctx, val, lit) = (
        styles.get_context(),
        styles.get_context_value(),
        styles.get_literal(),
    );
    let comma = format!("{ctx}, {ctx:#}");
    let default = (arg.get_action().takes_values()
        && !arg.is_hide_default_value_set()
        && !arg.get_default_values().is_empty())
    .then(|| {
        let values: Vec<String> = arg
            .get_default_values()
            .iter()
            .map(|v| quoted(&v.to_string_lossy()))
            .collect();
        format!(
            "{ctx}{}{ctx:#}",
            tr!(
                l,
                "help-default",
                value = format!("{ctx:#}{val}{}{val:#}{ctx}", values.join(" "))
            )
        )
    });
    let values: Vec<_> = if arg.is_hide_possible_values_set() {
        Vec::new()
    } else {
        arg.get_possible_values()
            .into_iter()
            .filter(|v| !v.is_hide_set())
            .collect()
    };
    let described = values.iter().any(|v| v.get_help().is_some());
    let listed = (!values.is_empty()).then(|| {
        let names: Vec<String> = values
            .iter()
            .map(|v| format!("{val}{}{val:#}", quoted(v.get_name())))
            .collect();
        format!(
            "{ctx}{}{ctx:#}",
            tr!(
                l,
                "help-possible-values",
                values = format!("{ctx:#}{}{ctx}", names.join(&comma))
            )
        )
    });
    if default.is_none() && listed.is_none() {
        return arg;
    }
    // clap's short help falls back to the long, and the long to the short.
    let short_help = arg
        .get_help()
        .or_else(|| arg.get_long_help())
        .map(|h| h.ansi().to_string());
    let long_help = arg
        .get_long_help()
        .or_else(|| arg.get_help())
        .map(|h| h.ansi().to_string());

    // Short help: the values after the help on its line.
    let short_specs: Vec<&String> = default.iter().chain(listed.iter()).collect();
    let mut short = short_help.clone().unwrap_or_default();
    if !short.is_empty() {
        short.push(' ');
    }
    short.push_str(&join(&short_specs, " "));

    // Long help: described values as a list, then the rest a line each.
    let mut long = long_help.unwrap_or_default();
    if described {
        let longest = values
            .iter()
            .map(|v| v.get_name().chars().count())
            .max()
            .unwrap_or(0);
        if !long.is_empty() {
            long.push_str("\n\n");
        }
        long.push_str(&tr!(l, "help-possible-values-heading"));
        for v in &values {
            let name = v.get_name();
            let _ = write!(long, "\n- {lit}{name}{lit:#}");
            if let Some(help) = v.get_help() {
                let pad = longest - name.chars().count();
                let _ = write!(long, ": {:pad$}{}", "", help.ansi());
            }
        }
    }
    let long_specs: Vec<&String> = if described {
        default.iter().collect()
    } else {
        short_specs.clone()
    };
    if !long_specs.is_empty() {
        if !long.is_empty() {
            long.push_str("\n\n");
        }
        long.push_str(&join(&long_specs, "\n"));
    }
    arg = arg
        .hide_default_value(true)
        .hide_possible_values(true)
        .help(short);
    arg.long_help(long)
}

fn join(parts: &[&String], sep: &str) -> String {
    parts
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(sep)
}

/// A value as clap writes one in help: quoted when empty or spaced.
fn quoted(value: &str) -> String {
    if value.is_empty() || value.contains(char::is_whitespace) {
        format!("{value:?}")
    } else {
        value.to_string()
    }
}

/// Every page of `cmd` whose layout by [`localize`] in English, forced,
/// differs from clap's own, with both texts: none, but for the spaces clap
/// leaves off the empty line before a "Possible values:" list, which nobody
/// sees. `l` is an English localizer.
#[cfg(any(test, feature = "test-support"))]
pub fn english_layout_differences(cmd: Command, l: &Localizer) -> Vec<String> {
    let mut clap = cmd.clone();
    clap.build();
    let mut ours = lay_out(cmd, l);
    let mut out = Vec::new();
    for path in paths(&clap) {
        let (a, b) = (find(&mut clap, &path), find(&mut ours, &path));
        for (flag, theirs, mine) in [
            ("-h", a.render_help(), b.render_help()),
            ("--help", a.render_long_help(), b.render_long_help()),
        ] {
            let (theirs, mine) = (
                trimmed(theirs.ansi().to_string()),
                trimmed(mine.ansi().to_string()),
            );
            if theirs != mine {
                out.push(format!("{path:?} {flag}\nclap:\n{theirs}\nours:\n{mine}"));
            }
        }
    }
    out
}

/// The English words of clap's that a page of `cmd`, laid out by
/// [`localize`] under `pseudo`, the pseudo-locale, still shows: none, as it
/// marks every word it translates.
#[cfg(any(test, feature = "test-support"))]
pub fn english_left(cmd: Command, pseudo: &Localizer) -> Vec<String> {
    let mut cmd = localize(cmd, pseudo);
    let mut out = Vec::new();
    for path in paths(&cmd) {
        let c = find(&mut cmd, &path);
        for text in [
            c.render_help().to_string(),
            c.render_long_help().to_string(),
        ] {
            for english in [
                "Usage:",
                "Commands:",
                "Arguments:",
                "Options:",
                "[default:",
                "[possible values:",
                "Possible values:",
                "Print help",
                "Print version",
                "Print this message",
            ] {
                if text.contains(english) {
                    out.push(format!("{path:?}: {english}\n{text}"));
                }
            }
        }
    }
    out
}

/// Every command's path below the root, the root first.
#[cfg(any(test, feature = "test-support"))]
fn paths(cmd: &Command) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new()];
    for c in cmd.get_subcommands().filter(|c| !c.is_hide_set()) {
        for mut p in paths(c) {
            p.insert(0, c.get_name().to_string());
            out.push(p);
        }
    }
    out
}

#[cfg(any(test, feature = "test-support"))]
fn find<'c>(cmd: &'c mut Command, path: &[String]) -> &'c mut Command {
    match path.split_first() {
        None => cmd,
        Some((first, rest)) => find(cmd.find_subcommand_mut(first).expect("a subcommand"), rest),
    }
}

/// `text` without the spaces that end its lines.
#[cfg(any(test, feature = "test-support"))]
fn trimmed(text: String) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::{EN_US, Options};
    use clap::ArgAction;

    /// A command with every part the layout words: a subcommand, a
    /// positional, a default, plain and described values, a long help.
    fn made_up() -> Command {
        Command::new("t")
            .version("1.0")
            .arg(Arg::new("level").long("level").default_value("3"))
            .arg(
                Arg::new("color")
                    .long("color")
                    .value_parser(["auto", "never"])
                    .help("When to colour"),
            )
            .subcommand(
                Command::new("run")
                    .about("Run it")
                    .arg(Arg::new("file").help("What to run"))
                    .arg(
                        Arg::new("mode")
                            .long("mode")
                            .long_help("How to run it")
                            .value_parser([
                                clap::builder::PossibleValue::new("fast").help("Quickly"),
                                clap::builder::PossibleValue::new("safe").help("Carefully"),
                            ]),
                    )
                    .arg(Arg::new("dry").long("dry").action(ArgAction::SetTrue)),
            )
    }

    #[test]
    fn english_layout_is_claps() {
        let l = Localizer::en_us(crate::help::RESOURCES);
        let differences = english_layout_differences(made_up(), &l);
        assert!(differences.is_empty(), "{}", differences.join("\n\n"));
    }

    #[test]
    fn no_english_layout_is_left() {
        let options = Options {
            pseudo: true,
            ..Default::default()
        };
        let l = Localizer::new(&EN_US, crate::help::RESOURCES, &options);
        let left = english_left(made_up(), &l);
        assert!(left.is_empty(), "{}", left.join("\n\n"));
    }
}
