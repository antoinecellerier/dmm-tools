//! The capture step instructions a user translates for themselves, as a
//! Fluent file: `capture --export-instructions` writes a meter's English,
//! the user replaces the text after each step id, and `capture
//! --instructions` shows their text in its place. The file is the user's:
//! no language ships one, and the report keeps the English.
//!
//! The file holds a `-device` term naming the registry entry it was
//! exported for, then one message per step, its id the step's, each under a
//! `# en:<fingerprint>` comment of the English it was exported from, so a
//! run can name the steps whose English has changed since, and the English
//! itself on the comment's next lines, for the user to compare.

use dmm_shared::l10n::escape;
use fluent_syntax::ast::{
    Comment, Entry, Expression, Identifier, InlineExpression, Message, Pattern, PatternElement,
    Resource, Term,
};

/// The term naming the device a file was exported for.
const DEVICE_TERM: &str = "device";
/// What starts the comment above each step that fingerprints its English.
const ENGLISH_MARK: &str = "en:";

/// A step instruction to export: the step's id and its English.
pub(crate) struct Exported<'a> {
    pub(crate) id: &'a str,
    pub(crate) english: &'a str,
}

/// The file for `device`'s `steps`, under `header`'s lines as the file's
/// comment.
pub(crate) fn export(header: &[String], device: &str, steps: &[Exported<'_>]) -> String {
    let mut body = vec![
        Entry::ResourceComment(Comment {
            content: header.to_vec(),
        }),
        Entry::Term(Term {
            id: Identifier {
                name: DEVICE_TERM.to_string(),
            },
            value: Pattern {
                elements: escape(device),
            },
            attributes: vec![],
            comment: None,
        }),
    ];
    for step in steps {
        body.push(Entry::Message(Message {
            id: Identifier {
                name: step.id.to_string(),
            },
            value: Some(Pattern {
                elements: escape(step.english),
            }),
            attributes: vec![],
            // The English again under its fingerprint, for the user to
            // check a translation against at a glance; nothing reads it back.
            comment: Some(Comment {
                content: std::iter::once(format!("{ENGLISH_MARK}{}", fingerprint(step.english)))
                    .chain(step.english.lines().map(str::to_string))
                    .collect(),
            }),
        }));
    }
    fluent_syntax::serializer::serialize(&Resource { body })
}

/// A short fingerprint of `english`: FNV-1a, 32 bits, in hex. It only has to
/// tell one wording of a step from the next.
pub(crate) fn fingerprint(english: &str) -> String {
    let hash = english.bytes().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    format!("{hash:08x}")
}

/// A file read back: the device it names and each step's text.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Instructions {
    /// The `-device` term; `None` when the user deleted it.
    pub(crate) device: Option<String>,
    pub(crate) steps: Vec<Instruction>,
}

/// One step's text in the user's file.
#[derive(Debug, PartialEq)]
pub(crate) struct Instruction {
    pub(crate) id: String,
    pub(crate) text: String,
    /// The fingerprint of the English it was translated from, when its
    /// comment survived.
    pub(crate) english: Option<String>,
}

/// What keeps a file from being read, by line where there is one.
#[derive(Debug, PartialEq)]
pub(crate) enum Problem {
    /// A line that isn't `id = text`, a comment or a term.
    Syntax { line: usize },
    /// A step whose text holds a `{ … }` other than a quoted literal.
    NotText { id: String },
    /// A step given twice.
    Twice { id: String },
    /// The parser gave up on the file as a whole.
    Unparsable,
}

/// The steps in `source`, a file [`export`] wrote and the user edited. A
/// UTF-8 byte-order mark and Windows line endings are fine.
pub(crate) fn read(source: &str) -> Result<Instructions, Vec<Problem>> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    // fluent-syntax 0.12 panics on a `\u` escape short of its hex digits
    // when a non-ASCII character follows (`{ "\u√" }`), printing a Rust
    // panic first: such a line is named before the parser sees it, and a
    // panic from anything else is caught.
    let escapes = bad_escapes(source);
    if !escapes.is_empty() {
        return Err(escapes);
    }
    let parsed = std::panic::catch_unwind(|| fluent_syntax::parser::parse(source))
        .map_err(|_| vec![Problem::Unparsable])?;
    let (resource, errors) = match parsed {
        Ok(r) => (r, vec![]),
        Err((r, errors)) => (r, errors),
    };
    let mut problems: Vec<Problem> = errors
        .iter()
        .map(|e| Problem::Syntax {
            line: source[..e.pos.start.min(source.len())]
                .matches('\n')
                .count()
                + 1,
        })
        .collect();
    problems.dedup();
    let mut out = Instructions::default();
    for entry in &resource.body {
        match entry {
            Entry::Term(t) if t.id.name == DEVICE_TERM => match text(&t.value) {
                Some(device) => out.device = Some(device.trim().to_string()),
                // Read as no device, it would skip the check that the file
                // is this meter's.
                None => problems.push(Problem::NotText {
                    id: format!("-{DEVICE_TERM}"),
                }),
            },
            Entry::Message(m) => {
                let id = m.id.name.to_string();
                if out.steps.iter().any(|s| s.id == id) {
                    problems.push(Problem::Twice { id });
                    continue;
                }
                let Some(text) = m.value.as_ref().and_then(text) else {
                    problems.push(Problem::NotText { id });
                    continue;
                };
                let english = m.comment.as_ref().and_then(|c| {
                    c.content
                        .iter()
                        .find_map(|l| l.trim().strip_prefix(ENGLISH_MARK))
                        .map(|f| f.trim().to_string())
                });
                out.steps.push(Instruction { id, text, english });
            }
            _ => {}
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

/// The lines outside comments holding a `\u` or `\U` escape without its 4
/// or 6 hex digits.
fn bad_escapes(source: &str) -> Vec<Problem> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with('#'))
        .filter(|(_, line)| {
            line.match_indices('\\').any(|(at, _)| {
                let mut rest = line[at + 1..].chars();
                let digits = match rest.next() {
                    Some('u') => 4,
                    Some('U') => 6,
                    _ => return false,
                };
                !(0..digits).all(|_| rest.next().is_some_and(|c| c.is_ascii_hexdigit()))
            })
        })
        .map(|(i, _)| Problem::Syntax { line: i + 1 })
        .collect()
}

/// `pattern` as plain text: its text and quoted literals, `None` if it holds
/// any other placeable.
fn text(pattern: &Pattern<&str>) -> Option<String> {
    let mut out = String::new();
    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { value } => out.push_str(value),
            PatternElement::Placeable {
                expression: Expression::Inline(InlineExpression::StringLiteral { value }),
            } => out.push_str(&fluent_syntax::unicode::unescape_unicode_to_string(value)),
            PatternElement::Placeable { .. } => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exported() -> String {
        export(
            &["Translate me.".to_string()],
            "bm82x",
            &[
                Exported {
                    id: "dcv",
                    english: "Set the dial to V⎓: leads open.",
                },
                Exported {
                    id: "braces",
                    english: "  [Press] {HOLD} . ",
                },
                Exported {
                    id: "rec_max",
                    english: "Two\nlines",
                },
            ],
        )
    }

    #[test]
    fn an_exported_file_reads_back_as_written() {
        let file = exported();
        assert!(file.starts_with("### Translate me.\n"), "{file}");
        assert!(file.contains("\n-device = bm82x\n"), "{file}");
        // Each step's English stays readable above it, a line per line.
        let dcv = format!(
            "# en:{}\n# Set the dial to V⎓: leads open.\ndcv = ",
            fingerprint("Set the dial to V⎓: leads open.")
        );
        assert!(file.contains(&dcv), "{file}");
        assert!(file.contains("\n# Two\n# lines\nrec_max ="), "{file}");
        let read = read(&file).unwrap();
        assert_eq!(read.device.as_deref(), Some("bm82x"));
        let texts: Vec<(&str, &str)> = read
            .steps
            .iter()
            .map(|s| (s.id.as_str(), s.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            [
                ("dcv", "Set the dial to V⎓: leads open."),
                ("braces", "  [Press] {HOLD} . "),
                ("rec_max", "Two\nlines"),
            ]
        );
        assert_eq!(
            read.steps[0].english.as_deref(),
            Some(fingerprint("Set the dial to V⎓: leads open.").as_str())
        );
    }

    #[test]
    fn a_translated_file_with_a_bom_and_crlf_reads() {
        let file = format!(
            "\u{feff}{}",
            exported()
                .replace("Set the dial to V⎓: leads open.", "Gire a chave para V⎓.")
                .replace('\n', "\r\n")
        );
        let read = read(&file).unwrap();
        assert_eq!(read.steps[0].text, "Gire a chave para V⎓.");
        assert_eq!(read.steps.len(), 3);
    }

    #[test]
    fn what_keeps_a_file_from_reading_is_named() {
        let file = "-device = mock\n\
                    # en:00000000\n\
                    dcv = { $x } here\n\
                    acv = Fine\n\
                    acv = Again\n\
                    this line has no equals sign\n";
        assert_eq!(
            read(file),
            Err(vec![
                Problem::Syntax { line: 6 },
                Problem::NotText { id: "dcv".into() },
                Problem::Twice { id: "acv".into() },
            ])
        );
    }

    #[test]
    fn a_broken_line_at_its_start_is_named_by_its_own_number() {
        for file in [
            "-device = mock\ndcv = A\n= sem id\nacv = B\n",
            "-device = mock\r\ndcv = A\r\n= sem id\r\nacv = B\r\n",
            "-device = mock\ndcv = A\n[x] = y\n",
        ] {
            assert_eq!(
                read(file),
                Err(vec![Problem::Syntax { line: 3 }]),
                "{file:?}"
            );
        }
    }

    #[test]
    fn an_escape_the_parser_would_panic_on_is_named_by_its_line() {
        let file = "-device = mock\n# en:00000000\ndcv = { \"\\u√\" }\nacv = { \"\\u00e9\" }\n";
        assert_eq!(read(file), Err(vec![Problem::Syntax { line: 3 }]));
        // A whole escape reads as its character.
        let read = read("acv = { \"\\u00e9\" }\n").unwrap();
        assert_eq!(read.steps[0].text, "é");
    }

    #[test]
    fn a_device_term_that_isnt_text_is_named() {
        assert_eq!(
            read("-device = { x }\ndcv = A\n"),
            Err(vec![Problem::NotText {
                id: "-device".into()
            }])
        );
    }

    #[test]
    fn a_missing_device_term_or_comment_is_no_problem() {
        let read = read("dcv = Texto\n").unwrap();
        assert_eq!(read.device, None);
        assert_eq!(read.steps[0].english, None);
    }

    #[test]
    fn the_fingerprint_tells_wordings_apart() {
        assert_eq!(fingerprint("a"), fingerprint("a"));
        assert_ne!(fingerprint("Set the dial"), fingerprint("Set the dial."));
        assert_eq!(fingerprint("").len(), 8);
    }
}
