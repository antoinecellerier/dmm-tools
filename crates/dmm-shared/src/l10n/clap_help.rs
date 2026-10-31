//! A clap command's help in the user's language. clap has no translation
//! hook, so [`translate`] walks the command tree before parsing and swaps
//! each text the tree carries for its message: a command's `about` and
//! `long_about`, an argument's `help` and `long_help`, and the help of an
//! argument's possible values.
//!
//! The message IDs are the texts' places in the tree, never a hash of the
//! English, so a reworded doc comment keeps its translations: the command
//! `read` is the message `read` with attributes `.about` and `.long-about`,
//! its `--interval-ms` is `read--interval-ms` with `.help` and `.long-help`,
//! a value of its `--format` is `read--format.value-replay`, the root is
//! named after itself (`dmm-cli`) and a nested command joins the names
//! below the root with `-`.
//!
//! What the walk leaves alone stays English: clap's scaffolding ("Usage:",
//! "Options:", `[default: …]`, `[possible values: …]`) and errors, the
//! built-in `help` and `version`, which clap adds after the walk, and hidden
//! commands and arguments. Help a binary builds at run time is translated
//! where it is built. The en-US file is generated from the tree by
//! [`render`].

use super::Localizer;
use clap::builder::{PossibleValue, ValueParser};
use clap::{Arg, Command};

/// What turns the translated possible values of the argument with message
/// `id` back into a parser of the argument's own type, matching case as
/// the argument does (`ignore_case`). clap keeps the type behind the
/// parser it holds, so only the binary, which knows it, can rebuild one
/// (`PossibleValuesParser::new(values).try_map(…)`); `None` leaves the
/// values' help English, which [`unmapped_values`] lets a test catch.
pub type ValuesParser<'a> = &'a dyn Fn(&str, Vec<PossibleValue>, bool) -> Option<ValueParser>;

/// `cmd` with the help it carries in `l`'s language.
pub fn translate(cmd: Command, l: &Localizer, values: ValuesParser<'_>) -> Command {
    Walk::new(&cmd, l, values).command(cmd, &mut Vec::new())
}

/// The arguments of `cmd` with described possible values that `values`
/// has no parser for, by message ID: their values' help would stay
/// English in every language.
#[cfg(any(test, feature = "test-support"))]
pub fn unmapped_values(cmd: Command, values: ValuesParser<'_>) -> Vec<String> {
    let l = Localizer::en_us(&[]);
    let mut walk = Walk::new(&cmd, &l, values);
    walk.command(cmd, &mut Vec::new());
    walk.unmapped
}

struct Walk<'a> {
    root: String,
    l: &'a Localizer,
    values: ValuesParser<'a>,
    /// What [`unmapped_values`] reports.
    unmapped: Vec<String>,
}

impl<'a> Walk<'a> {
    fn new(cmd: &Command, l: &'a Localizer, values: ValuesParser<'a>) -> Self {
        Self {
            root: cmd.get_name().to_string(),
            l,
            values,
            unmapped: Vec::new(),
        }
    }

    /// `cmd`, at `path` below the root, translated.
    fn command(&mut self, mut cmd: Command, path: &mut Vec<String>) -> Command {
        let id = command_id(&self.root, path);
        let l = self.l;
        let text = |attribute: &str| text(l, &id, attribute);
        if has_text(cmd.get_about())
            && let Some(about) = text("about")
        {
            cmd = cmd.about(about);
        }
        if has_text(cmd.get_long_about())
            && let Some(long_about) = text("long-about")
        {
            cmd = cmd.long_about(long_about);
        }
        // The plural forms keep the tree's order, which `mut_arg` and
        // `mut_subcommand` would change by putting the one changed last.
        cmd = cmd.mut_args(|a| if a.is_hide_set() { a } else { self.arg(a, &id) });
        cmd.mut_subcommands(|c| {
            if c.is_hide_set() {
                return c;
            }
            path.push(c.get_name().to_string());
            let c = self.command(c, path);
            path.pop();
            c
        })
    }

    /// `arg` of the command whose message is `command`, translated.
    fn arg(&mut self, mut arg: Arg, command: &str) -> Arg {
        let id = arg_id(command, &arg);
        let l = self.l;
        let text = |attribute: &str| text(l, &id, attribute);
        if has_text(arg.get_help())
            && let Some(help) = text("help")
        {
            arg = arg.help(help);
        }
        if has_text(arg.get_long_help())
            && let Some(long_help) = text("long-help")
        {
            arg = arg.long_help(long_help);
        }
        let possible = arg.get_possible_values();
        if possible.iter().any(described) {
            let translated = possible
                .into_iter()
                .map(
                    |v| match described(&v).then(|| text(&value_attribute(&v))) {
                        Some(Some(help)) => v.help(help),
                        _ => v,
                    },
                )
                .collect();
            match (self.values)(&id, translated, arg.is_ignore_case_set()) {
                Some(parser) => arg = arg.value_parser(parser),
                None => self.unmapped.push(id),
            }
        }
        arg
    }
}

/// Attribute `attribute` of message `id` in `l`'s language; `None` where
/// en-US lacks it too, which keeps the tree's own English. The walk runs
/// before every command, so a stale `cli-help.ftl`, which its regeneration
/// test catches, or a partial one under `DMM_TOOLS_L10N_DIR` must not stop
/// the binary the way [`Localizer::message`]'s assertion would.
fn text(l: &Localizer, id: &str, attribute: &str) -> Option<String> {
    l.lookup(&format!("{id}.{attribute}"), &[])
        .map(std::borrow::Cow::into_owned)
}

/// The message of the command at `path`, the subcommand names below the
/// root.
fn command_id(root: &str, path: &[String]) -> String {
    if path.is_empty() {
        root.to_string()
    } else {
        path.join("-")
    }
}

/// The message of `arg` of the command whose message is `command`.
fn arg_id(command: &str, arg: &Arg) -> String {
    format!("{command}--{}", arg.get_id().as_str().replace('_', "-"))
}

/// The attribute of a possible value's help.
fn value_attribute(value: &PossibleValue) -> String {
    format!("value-{}", value.get_name())
}

fn has_text(text: Option<&clap::builder::StyledStr>) -> bool {
    text.is_some_and(|t| !t.to_string().is_empty())
}

fn described(value: &PossibleValue) -> bool {
    !value.is_hide_set() && has_text(value.get_help())
}

#[cfg(any(test, feature = "test-support"))]
pub use generate::{Changes, changes, render};

/// The en-US file from the tree, and what changed since the last one.
#[cfg(any(test, feature = "test-support"))]
mod generate {
    use super::*;
    use crate::l10n::ftl_text::escape;
    use fluent_syntax::ast::{Attribute, Comment, Entry, Identifier, Message, Pattern, Resource};
    use std::collections::BTreeMap;

    fn visible_args(cmd: &Command) -> impl Iterator<Item = &Arg> {
        cmd.get_arguments().filter(|a| !a.is_hide_set())
    }

    fn visible_subcommands(cmd: &Command) -> impl Iterator<Item = &Command> {
        cmd.get_subcommands().filter(|c| !c.is_hide_set())
    }

    /// One message: a command's or an argument's texts.
    struct Node {
        id: String,
        /// The command line it belongs to, for the translator.
        line: String,
        notes: Vec<&'static str>,
        attributes: Vec<(String, String)>,
    }

    /// `cmd`'s texts as an en-US `.ftl` file opening with `header` (the
    /// `###` comment, without its marks), one message per command and per
    /// argument that has any, in the tree's order.
    pub fn render(cmd: &Command, header: &str) -> String {
        let mut nodes = Vec::new();
        let root = cmd.get_name().to_string();
        visit(cmd, &root, &mut Vec::new(), root.clone(), &mut nodes);
        let mut body = vec![Entry::ResourceComment(Comment {
            content: header.lines().map(str::to_string).collect(),
        })];
        for node in nodes {
            let mut content = vec![node.line];
            content.extend(node.notes.iter().map(|n| n.to_string()));
            body.push(Entry::Message(Message {
                id: Identifier { name: node.id },
                value: None,
                attributes: node
                    .attributes
                    .into_iter()
                    .map(|(name, text)| Attribute {
                        id: Identifier { name },
                        value: Pattern {
                            elements: escape(&text),
                        },
                    })
                    .collect(),
                comment: Some(Comment { content }),
            }));
        }
        fluent_syntax::serializer::serialize(&Resource { body })
    }

    fn visit(cmd: &Command, root: &str, path: &mut Vec<String>, line: String, out: &mut Vec<Node>) {
        let id = command_id(root, path);
        let mut attributes = Vec::new();
        let mut notes = Vec::new();
        push(&mut attributes, "about", cmd.get_about());
        if push(&mut attributes, "long-about", cmd.get_long_about()) {
            notes.push(".about is shown by -h and in the list of commands, .long-about by --help.");
        }
        if !attributes.is_empty() {
            out.push(Node {
                id: id.clone(),
                line: line.clone(),
                notes,
                attributes,
            });
        }
        for arg in visible_args(cmd) {
            let mut attributes = Vec::new();
            let mut notes = Vec::new();
            push(&mut attributes, "help", arg.get_help());
            if push(&mut attributes, "long-help", arg.get_long_help()) {
                notes.push(".help is shown by -h, .long-help by --help.");
            }
            let values: Vec<PossibleValue> = arg
                .get_possible_values()
                .into_iter()
                .filter(described)
                .collect();
            for value in &values {
                let name = value_attribute(value);
                assert!(
                    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                    "{name} is no Fluent attribute"
                );
                push(&mut attributes, &name, value.get_help());
            }
            if !values.is_empty() {
                notes.push(".value-… says what that value does, in --help's list of values.");
            }
            if !attributes.is_empty() {
                out.push(Node {
                    id: arg_id(&id, arg),
                    line: format!("{line} {}", flag(arg)),
                    notes,
                    attributes,
                });
            }
        }
        for sub in visible_subcommands(cmd) {
            path.push(sub.get_name().to_string());
            visit(sub, root, path, format!("{line} {}", sub.get_name()), out);
            path.pop();
        }
    }

    /// `attribute` = `text`, when there is text; whether there was.
    fn push(
        attributes: &mut Vec<(String, String)>,
        attribute: &str,
        text: Option<&clap::builder::StyledStr>,
    ) -> bool {
        let Some(text) = text.map(|t| t.to_string()).filter(|t| !t.is_empty()) else {
            return false;
        };
        attributes.push((attribute.to_string(), text));
        true
    }

    /// How the command line spells `arg`: `--long`, `-s`, or `<NAME>`.
    fn flag(arg: &Arg) -> String {
        if let Some(long) = arg.get_long() {
            format!("--{long}")
        } else if let Some(short) = arg.get_short() {
            format!("-{short}")
        } else {
            let name = arg
                .get_value_names()
                .and_then(|n| n.first())
                .map_or_else(|| arg.get_id().as_str().to_uppercase(), |n| n.to_string());
            format!("<{name}>")
        }
    }

    /// The `id.attribute`s whose English differs between two generated
    /// files.
    #[derive(Debug, Default, PartialEq, Eq)]
    pub struct Changes {
        /// In both, worded differently.
        pub changed: Vec<String>,
        /// Only in the new one.
        pub added: Vec<String>,
        /// Only in the old one.
        pub removed: Vec<String>,
    }

    impl Changes {
        pub fn is_empty(&self) -> bool {
            self == &Self::default()
        }
    }

    /// What changed from `old` to `new`, both generated by [`render`].
    pub fn changes(old: &str, new: &str) -> Changes {
        let (old, new) = (texts(old), texts(new));
        let mut out = Changes::default();
        for (key, text) in &new {
            match old.get(key) {
                Some(was) if was != text => out.changed.push(key.clone()),
                Some(_) => {}
                None => out.added.push(key.clone()),
            }
        }
        out.removed = old
            .keys()
            .filter(|k| !new.contains_key(*k))
            .cloned()
            .collect();
        out
    }

    /// `id.attribute` → the attribute's source, for comparison.
    fn texts(source: &str) -> BTreeMap<String, String> {
        let resource = fluent_syntax::parser::parse(source).unwrap_or_else(|(r, _)| r);
        let mut out = BTreeMap::new();
        for entry in &resource.body {
            let Entry::Message(m) = entry else { continue };
            for a in &m.attributes {
                out.insert(
                    format!("{}.{}", m.id.name, a.id.name),
                    format!("{:?}", a.value),
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::{EN_US, Options, Resource};
    use clap::builder::PossibleValuesParser;

    /// A tree with text Fluent would read as syntax, several paragraphs, a
    /// nested command and a described value.
    fn tree() -> Command {
        Command::new("tool")
            .about("A {braced} tool")
            .arg(
                Arg::new("level")
                    .long("level")
                    .help("[x] starts with a bracket")
                    .long_help("* starts with a star\n\n.starts with a dot\n  indented { brace }")
                    .value_parser(PossibleValuesParser::new([
                        PossibleValue::new("low").help("Quiet \"low\""),
                        PossibleValue::new("high"),
                    ])),
            )
            .arg(Arg::new("secret").long("secret").help("Hidden").hide(true))
            .subcommand(
                Command::new("outer")
                    .about("Outer")
                    .long_about("Outer, at length.\n\nSecond paragraph   ")
                    .subcommand(
                        Command::new("inner")
                            .about("Inner")
                            .arg(Arg::new("file_name").help("The file")),
                    ),
            )
    }

    fn localizer(source: &'static str, pseudo: bool) -> Localizer {
        let resources = [Resource {
            locale: "en-US",
            path: "help.ftl",
            source,
        }];
        let options = Options {
            pseudo,
            ..Options::default()
        };
        Localizer::new(&EN_US, &resources, &options)
    }

    fn rebuild(_: &str, values: Vec<PossibleValue>, _: bool) -> Option<ValueParser> {
        Some(PossibleValuesParser::new(values).into())
    }

    /// An argument whose values' help no parser carries is reported, so the
    /// binary's test fails rather than its help staying English.
    #[test]
    fn values_without_a_parser_are_reported() {
        let none = |_: &str, _: Vec<PossibleValue>, _: bool| None;
        assert_eq!(unmapped_values(tree(), &none), ["tool--level"]);
        assert!(unmapped_values(tree(), &rebuild).is_empty());
    }

    fn rendered(mut cmd: Command) -> Vec<String> {
        cmd.build();
        let mut out = vec![
            cmd.render_help().to_string(),
            cmd.render_long_help().to_string(),
        ];
        let mut outer = cmd.find_subcommand("outer").unwrap().clone();
        out.push(outer.render_long_help().to_string());
        let mut inner = outer.find_subcommand("inner").unwrap().clone();
        out.push(inner.render_long_help().to_string());
        out
    }

    /// What `render` writes reads back, through Fluent and the walk, as the
    /// tree's own text.
    #[test]
    fn the_generated_file_reads_back_as_the_tree() {
        let source: &'static str = render(&tree(), "Header").leak();
        let walked = translate(tree(), &localizer(source, false), &rebuild);
        assert_eq!(rendered(walked), rendered(tree()), "{source}");
        for id in [
            "tool.about",
            "tool--level.help",
            "tool--level.long-help",
            "tool--level.value-low",
            "outer.long-about",
            "outer-inner--file-name.help",
        ] {
            let (message, attribute) = id.split_once('.').unwrap();
            assert!(
                source.contains(&format!("\n{message} =")) && source.contains(attribute),
                "no {id} in:\n{source}"
            );
        }
        assert!(!source.contains("Hidden"), "{source}");
        assert!(
            source.contains("# tool outer inner <FILE_NAME>"),
            "{source}"
        );
    }

    /// A text the file lacks keeps the tree's English, and the walk goes on.
    #[test]
    fn a_missing_message_keeps_the_trees_text() {
        let partial = "tool =\n    .about = Un outil\n";
        let walked = translate(tree(), &localizer(partial, false), &rebuild);
        assert_eq!(walked.get_about().unwrap().to_string(), "Un outil");
        let expected = tree().about("Un outil");
        assert_eq!(rendered(walked), rendered(expected));
    }

    #[test]
    fn the_pseudo_locale_brackets_every_walked_text() {
        let source: &'static str = render(&tree(), "Header").leak();
        let walked = translate(tree(), &localizer(source, true), &rebuild);
        let bracketed = |t: Option<&clap::builder::StyledStr>| {
            let t = t.unwrap().to_string();
            assert!(t.starts_with('[') && t.ends_with(']'), "{t}");
        };
        bracketed(walked.get_about());
        let level = walked
            .get_arguments()
            .find(|a| a.get_id() == "level")
            .unwrap();
        bracketed(level.get_help());
        bracketed(level.get_long_help());
        bracketed(level.get_possible_values()[0].get_help());
        let outer = walked.find_subcommand("outer").unwrap();
        bracketed(outer.get_long_about());
        let secret = walked
            .get_arguments()
            .find(|a| a.get_id() == "secret")
            .unwrap();
        assert_eq!(secret.get_help().unwrap().to_string(), "Hidden");
    }

    #[test]
    fn changes_name_the_attributes_whose_english_moved() {
        let old = render(&tree(), "H");
        let new = render(
            &tree()
                .about("A tool, reworded")
                .mut_arg("secret", |a| a.hide(false)),
            "H",
        );
        assert_eq!(
            changes(&old, &new),
            Changes {
                changed: vec!["tool.about".to_string()],
                added: vec!["tool--secret.help".to_string()],
                removed: vec![],
            }
        );
        assert!(changes(&new, &new).is_empty());
    }
}
