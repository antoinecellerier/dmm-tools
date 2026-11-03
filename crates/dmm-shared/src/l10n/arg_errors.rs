//! clap's command-line errors in the user's language: an unknown option, a
//! missing command, a value clap refuses. The same in dmm-cli and dmm-gui,
//! so the messages are in `shared.ftl`.
//!
//! clap words them in English inside its `RichFormatter`, whose layout this
//! follows line for line, with each sentence taken from the messages. In
//! English the result is byte for byte clap's own, which a test holds it
//! to, but for the hidden subcommands clap would list among the expected
//! ones. What clap builds as text of its own stays English: the usage
//! line's syntax after its worded heading, and the rare `tip:` hints that
//! are whole sentences (`to pass '--x' as a value, use '-- --x'`). Kinds
//! that are not errors (help, version) and the ones this doesn't word go to
//! clap as they are.

use std::fmt::Write as _;
use std::sync::OnceLock;

use super::{Arg, Localizer};
use crate::tr;
use clap::builder::StyledStr;
use clap::builder::styling::Styles;
use clap::error::{ContextKind, ContextValue, ErrorFormatter, ErrorKind};

/// Two spaces, as clap indents a list under its error line.
const TAB: &str = "  ";

/// Print `e` in `l`'s language and exit, as [`clap::Error::exit`] does.
/// `hidden` names the subcommands kept out of `--help`, which clap still
/// lists among the ones it expected.
pub fn exit(e: clap::Error, l: &Localizer, hidden: &[String]) -> ! {
    if worded(e.kind()) {
        // clap's formatter hook takes no state, so the text is worded here
        // and handed to it: the process ends on this error.
        let _ = WORDED.set(render(&e, l, hidden));
        e.apply::<Worded>().exit()
    } else {
        e.exit()
    }
}

/// What [`exit`] worded, for [`Worded`] to print.
static WORDED: OnceLock<StyledStr> = OnceLock::new();

/// The kinds [`render`] words; the others are help, version or I/O.
fn worded(kind: ErrorKind) -> bool {
    !matches!(
        kind,
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            | ErrorKind::DisplayVersion
            | ErrorKind::Io
            | ErrorKind::Format
            | ErrorKind::InvalidUtf8
    )
}

/// clap's formatter hook: the text [`exit`] worded, else English.
struct Worded;

impl ErrorFormatter for Worded {
    fn format_error(error: &clap::error::Error<Self>) -> StyledStr {
        WORDED
            .get()
            .cloned()
            .unwrap_or_else(|| render(error, &Localizer::en_us(crate::help::RESOURCES), &[]))
    }
}

/// `error` laid out as clap's `RichFormatter` lays it out, in `l`'s
/// language, leaving the `hidden` subcommands out of a list. Both binaries
/// keep clap's default styles and its `--help` flag.
pub fn render<F: ErrorFormatter>(
    error: &clap::error::Error<F>,
    l: &Localizer,
    hidden: &[String],
) -> StyledStr {
    let styles = Styles::styled();
    let paint = Paint(&styles);
    let mut out = StyledStr::new();
    let e = styles.get_error();
    let _ = write!(out, "{e}{}{e:#} ", tr!(l, "args-error"));
    let _ = write!(out, "{}", message(error, l, &paint, hidden));

    let mut suggested = false;
    for (kind, what) in [
        (ContextKind::SuggestedSubcommand, "subcommand"),
        (ContextKind::SuggestedArg, "argument"),
        (ContextKind::SuggestedValue, "value"),
    ] {
        if let Some(names) = error.get(kind) {
            let _ = out.write_str("\n");
            if !suggested {
                let _ = out.write_str("\n");
                suggested = true;
            }
            let _ = write!(out, "{}", similar(l, &paint, what, names));
        }
    }
    if let Some(ContextValue::StyledStrs(tips)) = error.get(ContextKind::Suggested) {
        if !suggested {
            let _ = out.write_str("\n");
        }
        for tip in tips {
            let _ = write!(
                out,
                "\n{TAB}{} {}",
                paint.valid(&tr!(l, "args-tip")),
                tip.ansi()
            );
        }
    }
    if let Some(ContextValue::StyledStr(usage)) = error.get(ContextKind::Usage) {
        // clap's heading, worded as --help words it (`help_layout`).
        let usage = usage
            .ansi()
            .to_string()
            .replacen("Usage:", &tr!(l, "help-usage"), 1);
        let _ = write!(out, "\n\n{usage}");
    }
    let help = paint.literal("--help");
    let _ = write!(out, "\n\n{}\n", tr!(l, "args-more-info", help = help));
    out
}

/// The styles clap paints an error's parts in.
struct Paint<'s>(&'s Styles);

impl Paint<'_> {
    fn valid(&self, s: &str) -> String {
        let v = self.0.get_valid();
        format!("{v}{s}{v:#}")
    }
    fn invalid(&self, s: &str) -> String {
        let v = self.0.get_invalid();
        format!("{v}{s}{v:#}")
    }
    fn literal(&self, s: &str) -> String {
        let v = self.0.get_literal();
        format!("{v}{s}{v:#}")
    }
}

fn string<F: ErrorFormatter>(error: &clap::error::Error<F>, kind: ContextKind) -> Option<&str> {
    match error.get(kind) {
        Some(ContextValue::String(s)) => Some(s),
        _ => None,
    }
}

fn number<F: ErrorFormatter>(error: &clap::error::Error<F>, kind: ContextKind) -> Option<isize> {
    match error.get(kind) {
        Some(ContextValue::Number(n)) => Some(*n),
        _ => None,
    }
}

/// A value clap lists as it writes it: quoted when empty or spaced.
fn escaped(value: &str) -> String {
    if value.is_empty() || value.contains(char::is_whitespace) {
        format!("{value:?}")
    } else {
        value.to_string()
    }
}

/// `[possible values: …]` and its like on a line of its own, empty when
/// clap gave no values. A subcommand list leaves out the `hidden` ones.
fn values_list(
    l: &Localizer,
    paint: &Paint,
    id: &str,
    values: Option<&ContextValue>,
    hidden: &[String],
) -> String {
    let Some(ContextValue::Strings(values)) = values else {
        return String::new();
    };
    let shown: Vec<String> = values
        .iter()
        .filter(|v| id != "args-subcommands" || !hidden.contains(v))
        .map(|v| paint.valid(&escaped(v)))
        .collect();
    if shown.is_empty() {
        return String::new();
    }
    let list = shown.join(", ");
    let line = match id {
        "args-subcommands" => tr!(l, "args-subcommands", values = list),
        _ => tr!(l, "args-possible-values", values = list),
    };
    format!("\n{TAB}{line}")
}

/// The error's sentence, and any list under it.
fn message<F: ErrorFormatter>(
    error: &clap::error::Error<F>,
    l: &Localizer,
    paint: &Paint,
    hidden: &[String],
) -> String {
    let arg = || string(error, ContextKind::InvalidArg).map(|a| paint.invalid(a));
    let literal_arg = || string(error, ContextKind::InvalidArg).map(|a| paint.literal(a));
    let value = || string(error, ContextKind::InvalidValue);
    let fallback = || error.kind().as_str().unwrap_or("unknown cause").to_string();
    match error.kind() {
        ErrorKind::ArgumentConflict => conflict(error, l, paint).unwrap_or_else(fallback),
        ErrorKind::NoEquals => arg().map_or_else(fallback, |arg| {
            tr!(l, "args-equals-needed", arg = arg).into_owned()
        }),
        ErrorKind::InvalidValue => match (literal_arg(), value()) {
            (Some(arg), Some(value)) => {
                let line = if value.is_empty() {
                    let arg = paint.invalid(string(error, ContextKind::InvalidArg).unwrap_or(""));
                    tr!(l, "args-value-required", arg = arg)
                } else {
                    tr!(
                        l,
                        "args-invalid-value",
                        value = paint.invalid(value),
                        arg = arg
                    )
                };
                let list = values_list(
                    l,
                    paint,
                    "args-possible-values",
                    error.get(ContextKind::ValidValue),
                    hidden,
                );
                format!("{line}{list}")
            }
            _ => fallback(),
        },
        ErrorKind::InvalidSubcommand => {
            string(error, ContextKind::InvalidSubcommand).map_or_else(fallback, |command| {
                tr!(
                    l,
                    "args-unrecognized-subcommand",
                    command = paint.invalid(command)
                )
                .into_owned()
            })
        }
        ErrorKind::MissingRequiredArgument => match error.get(ContextKind::InvalidArg) {
            Some(ContextValue::Strings(args)) => {
                let mut line = tr!(l, "args-missing-required").into_owned();
                for a in args {
                    let _ = write!(line, "\n{TAB}{}", paint.valid(a));
                }
                line
            }
            _ => fallback(),
        },
        ErrorKind::MissingSubcommand => {
            string(error, ContextKind::InvalidSubcommand).map_or_else(fallback, |command| {
                let line = tr!(
                    l,
                    "args-missing-subcommand",
                    command = paint.invalid(command)
                );
                let list = values_list(
                    l,
                    paint,
                    "args-subcommands",
                    error.get(ContextKind::ValidSubcommand),
                    hidden,
                );
                format!("{line}{list}")
            })
        }
        ErrorKind::TooManyValues => match (literal_arg(), value()) {
            (Some(arg), Some(value)) => tr!(
                l,
                "args-unexpected-value",
                value = paint.invalid(value),
                arg = arg
            )
            .into_owned(),
            _ => fallback(),
        },
        ErrorKind::TooFewValues => match (
            literal_arg(),
            number(error, ContextKind::ActualNumValues),
            number(error, ContextKind::MinValues),
        ) {
            (Some(arg), Some(actual), Some(min)) => tr!(
                l,
                "args-too-few-values",
                needed = paint.valid(&min.to_string()),
                arg = arg,
                given = paint.invalid(&actual.to_string()),
                count = Arg::Int(actual as i64)
            )
            .into_owned(),
            _ => fallback(),
        },
        ErrorKind::WrongNumberOfValues => match (
            literal_arg(),
            number(error, ContextKind::ActualNumValues),
            number(error, ContextKind::ExpectedNumValues),
        ) {
            (Some(arg), Some(actual), Some(expected)) => tr!(
                l,
                "args-wrong-number-of-values",
                needed = paint.valid(&expected.to_string()),
                arg = arg,
                given = paint.invalid(&actual.to_string()),
                count = Arg::Int(actual as i64)
            )
            .into_owned(),
            _ => fallback(),
        },
        ErrorKind::ValueValidation => match (literal_arg(), value()) {
            (Some(arg), Some(value)) => {
                let value = paint.invalid(value);
                match std::error::Error::source(error) {
                    Some(reason) => tr!(
                        l,
                        "args-invalid-value-because",
                        value = value,
                        arg = arg,
                        reason = reason.to_string()
                    ),
                    None => tr!(l, "args-invalid-value", value = value, arg = arg),
                }
                .into_owned()
            }
            _ => fallback(),
        },
        ErrorKind::UnknownArgument => arg().map_or_else(fallback, |arg| {
            tr!(l, "args-unexpected-argument", arg = arg).into_owned()
        }),
        _ => fallback(),
    }
}

/// An argument or subcommand that can't go with another, or be repeated.
fn conflict<F: ErrorFormatter>(
    error: &clap::error::Error<F>,
    l: &Localizer,
    paint: &Paint,
) -> Option<String> {
    let (what, name) = match (
        string(error, ContextKind::InvalidArg),
        string(error, ContextKind::InvalidSubcommand),
    ) {
        (Some(arg), _) => ("argument", arg),
        (None, Some(command)) => ("subcommand", command),
        (None, None) => return None,
    };
    let prior = error.get(ContextKind::PriorArg);
    let name_painted = paint.invalid(name);
    if what == "argument" && prior == Some(&ContextValue::String(name.to_string())) {
        return Some(tr!(l, "args-repeated", arg = name_painted).into_owned());
    }
    Some(match prior {
        Some(ContextValue::Strings(others)) => {
            let mut line = tr!(
                l,
                "args-conflict-with-list",
                what = what,
                name = name_painted
            )
            .into_owned();
            for o in others {
                let _ = write!(line, "\n{TAB}{}", paint.invalid(o));
            }
            line
        }
        Some(ContextValue::String(other)) => tr!(
            l,
            "args-conflict-with",
            what = what,
            name = name_painted,
            other = paint.invalid(other)
        )
        .into_owned(),
        _ => tr!(
            l,
            "args-conflict-with-others",
            what = what,
            name = name_painted
        )
        .into_owned(),
    })
}

/// `tip: a similar argument exists: '--lang'` and its plural.
fn similar(l: &Localizer, paint: &Paint, what: &str, names: &ContextValue) -> String {
    let names: Vec<&str> = match names {
        ContextValue::String(n) => vec![n.as_str()],
        ContextValue::Strings(ns) => ns.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    };
    let listed = names
        .iter()
        .map(|n| format!("'{}'", paint.valid(n)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{TAB}{} {}",
        paint.valid(&tr!(l, "args-tip")),
        tr!(
            l,
            "args-similar",
            what = what,
            count = Arg::Int(names.len() as i64),
            names = listed
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english() -> Localizer {
        Localizer::en_us(crate::help::RESOURCES)
    }

    /// What clap prints for `args` on `cmd`, and what [`render`] makes of
    /// the same error in English, without colour.
    fn both(cmd: clap::Command, args: &[&str]) -> (String, String) {
        let e = cmd.try_get_matches_from(args).expect_err("an error");
        let ours = render(&e, &english(), &[]).to_string();
        (e.render().to_string(), ours)
    }

    /// Every kind this words, on a made-up command; each binary's tests
    /// hold its own command to clap's words too.
    #[test]
    fn english_matches_clap_on_every_kind() {
        use clap::{Arg, ArgAction, Command};
        let cmd = || {
            Command::new("t")
                .arg(Arg::new("a").long("a").conflicts_with("b"))
                .arg(Arg::new("b").long("b"))
                .arg(Arg::new("r").long("r").action(ArgAction::Set))
                .arg(Arg::new("eq").long("eq").require_equals(true))
                .arg(Arg::new("two").long("two").num_args(2))
                .arg(Arg::new("few").long("few").num_args(3..))
                .arg(Arg::new("need").long("need").required(true))
        };
        for args in [
            &["t", "--need", "x", "--a", "1", "--b", "2"][..],
            &["t", "--need", "x", "--r", "1", "--r", "2"],
            &["t", "--need", "x", "--eq", "1"],
            &["t", "--need", "x", "--two", "1"],
            &["t", "--need", "x", "--few", "1", "2"],
            &["t"],
            &["t", "--need", "x", "--nede", "y"],
        ] {
            let (clap, ours) = both(cmd(), args);
            assert_eq!(ours, clap, "{args:?}");
        }
    }

    /// In another language the sentences change and the names typed stay.
    #[test]
    fn errors_are_worded_in_the_language() {
        static MADE_UP: crate::l10n::Language = crate::l10n::Language {
            id: "fr-CA",
            native_name: "Français (Canada)",
            reviewed: true,
        };
        let resources = [
            crate::help::RESOURCES,
            &[crate::l10n::Resource {
                locale: "fr-CA",
                path: "shared.ftl",
                source: "args-error = erreur :\n\
                         args-invalid-value = valeur « { $value } » refusée pour « { $arg } »\n\
                         args-possible-values = [valeurs possibles : { $values }]\n",
            }],
        ]
        .concat();
        let l = Localizer::new(&MADE_UP, &resources, &Default::default());
        let e = clap::Command::new("t")
            .arg(
                clap::Arg::new("format")
                    .long("format")
                    .value_parser(["text", "csv"]),
            )
            .try_get_matches_from(["t", "--format", "xml"])
            .expect_err("an error");
        assert_eq!(
            render(&e, &l, &[]).to_string(),
            "erreur : valeur « xml » refusée pour « --format <format> »\n  \
             [valeurs possibles : text, csv]\n\n\
             For more information, try '--help'.\n"
        );
    }
}
