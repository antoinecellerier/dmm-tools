//! Text as Fluent source: the escaping the generated files need, `lib.ftl`
//! (the `lib_ftl` test), `cli-help.ftl` (`clap_help::render`) and dmm-cli's
//! capture instruction files.

use fluent_syntax::ast::{Expression, InlineExpression, PatternElement};

/// A string literal placeable holding `s`. Literals hold their source form,
/// so `\` and `"` are escaped.
fn literal(s: &str) -> PatternElement<String> {
    PatternElement::Placeable {
        expression: Expression::Inline(InlineExpression::StringLiteral {
            value: s.replace('\\', "\\\\").replace('"', "\\\""),
        }),
    }
}

/// `text` as a pattern Fluent reads back unchanged. fluent-syntax's
/// serializer escapes nothing, so what Fluent would read as syntax goes in
/// string literals: braces anywhere, and at a line's start or end the
/// spaces Fluent trims and the `[`, `*` or `.` it would read as a variant
/// or an attribute. Each line is a text element of its own, ending at its
/// `\n`, which is where the serializer indents a multi-line pattern.
pub fn escape(text: &str) -> Vec<PatternElement<String>> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    let flush = |buffer: &mut String, out: &mut Vec<PatternElement<String>>| {
        if !buffer.is_empty() {
            out.push(PatternElement::TextElement {
                value: std::mem::take(buffer),
            });
        }
    };
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            buffer.push('\n');
            flush(&mut buffer, &mut out);
        }
        let body = line.trim_start_matches(' ');
        let mut rest = body.trim_end_matches(' ');
        let trailing = &body[rest.len()..];
        if body.len() < line.len() {
            out.push(literal(&line[..line.len() - body.len()]));
        } else if rest.starts_with(['[', '*', '.']) {
            out.push(literal(&rest[..1]));
            rest = &rest[1..];
        }
        for c in rest.chars() {
            if c == '{' || c == '}' {
                flush(&mut buffer, &mut out);
                out.push(literal(&c.to_string()));
            } else {
                buffer.push(c);
            }
        }
        if !trailing.is_empty() {
            flush(&mut buffer, &mut out);
            out.push(literal(trailing));
        }
    }
    flush(&mut buffer, &mut out);
    out
}
