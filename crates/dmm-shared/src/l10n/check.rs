//! The test that stands in for compile-time message checks: every
//! [`tr!`](crate::tr) in a crate's source names an en-US message, passes
//! exactly the variables that message uses, and every message and attribute
//! a crate owns is used and has its variables explained to translators.
//!
//! Each crate runs [`check`] over its own `src/` from a test. The scan reads
//! source text, so code behind `cfg` for another platform counts too. It
//! skips comments, string literals and inline `#[cfg(test)]` modules, whose
//! lookups are checked when the tests run.

use super::Resource;
use fluent_syntax::ast;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One `tr!` call in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Use {
    pub file: PathBuf,
    pub line: usize,
    /// `message` or `message.attribute`.
    pub id: String,
    /// The variable names passed, in call order.
    pub args: Vec<String>,
}

/// Every `tr!` call in the `.rs` files under `dir`.
pub fn scan(dir: &Path) -> Vec<Use> {
    let mut files = Vec::new();
    collect_rs(dir, &mut files);
    files.sort();
    files
        .iter()
        .flat_map(|f| {
            let text = std::fs::read_to_string(f).unwrap_or_default();
            scan_text(&text)
                .into_iter()
                .map(|(line, id, args)| Use {
                    file: f.clone(),
                    line,
                    id,
                    args,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `(line, id, variable names)` of each `tr!` call in `text`, outside
/// comments, string literals and inline `#[cfg(test)]` modules.
fn scan_text(text: &str) -> Vec<(usize, String, Vec<String>)> {
    let (text, bare) = blank(text);
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = bare[from..].find("tr!(") {
        let start = from + at;
        from = start + 4;
        // A word ending in `tr` (`attr!(`) is another macro.
        let before = bare[..start].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let Some(args) = split_args(&text[from..]) else {
            continue;
        };
        let line = text[..start].matches('\n').count() + 1;
        let Some(id) = args.get(1).and_then(|a| string_literal(a)) else {
            continue;
        };
        let names = args[2..]
            .iter()
            .filter_map(|a| a.split_once('=').map(|(n, _)| n.trim().to_string()))
            .collect();
        out.push((line, id, names));
    }
    out
}

/// `text` with its comments and inline `#[cfg(test)]` modules blanked, and
/// that again with the insides of string and character literals blanked
/// too. Each blanked byte becomes a space and newlines stay, so offsets and
/// line numbers match the source in both. A test module declared out of
/// line (`mod tests;`) blanks nothing: the scan reads its file as any other.
fn blank(text: &str) -> (String, String) {
    fn spaces(v: &mut [u8]) {
        for b in v.iter_mut().filter(|b| **b != b'\n') {
            *b = b' ';
        }
    }
    let b = text.as_bytes();
    let mut code = b.to_vec();
    let mut bare = b.to_vec();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut i = 0;
    while i < b.len() {
        let ident_before = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if b[i..].starts_with(b"//") {
            let end = b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(b.len(), |n| i + n);
            spaces(&mut code[i..end]);
            spaces(&mut bare[i..end]);
            i = end;
        } else if b[i..].starts_with(b"/*") {
            // Block comments nest.
            let (mut j, mut depth) = (i + 2, 1);
            while j < b.len() && depth > 0 {
                if b[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if b[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            spaces(&mut code[i..j]);
            spaces(&mut bare[i..j]);
            i = j;
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let j = j.min(b.len());
            spaces(&mut bare[i + 1..j]);
            i = j + 1;
        } else if !ident_before && (b[i] == b'r' || (b[i] == b'b' && at(i + 1) == b'r')) {
            // `r"…"`, `r#"…"#`, `br"…"`: no escapes, closed by the quote
            // and as many `#`.
            let r = if b[i] == b'r' { i } else { i + 1 };
            let hashes = b[r + 1..].iter().take_while(|&&c| c == b'#').count();
            let open = r + 1 + hashes;
            if at(open) != b'"' {
                i += 1;
                continue;
            }
            let mut close = vec![b'"'];
            close.extend(std::iter::repeat_n(b'#', hashes));
            let end = b[open + 1..]
                .windows(close.len())
                .position(|w| w == close.as_slice())
                .map_or(b.len(), |n| open + 1 + n);
            spaces(&mut bare[open + 1..end]);
            i = end + close.len();
        } else if b[i] == b'\'' {
            // A character literal, or else a lifetime or label.
            let end = if at(i + 1) == b'\\' {
                let s = (i + 3).min(b.len());
                b[s..].iter().position(|&c| c == b'\'').map(|n| s + n)
            } else {
                text[i + 1..]
                    .chars()
                    .next()
                    .map(|c| i + 1 + c.len_utf8())
                    .filter(|&e| at(e) == b'\'')
            };
            match end {
                Some(end) => {
                    spaces(&mut bare[i + 1..end]);
                    i = end + 1;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    // Inline test modules, found where comments and literals can't fake one.
    let mut from = 0;
    while let Some(n) = find(&bare[from..], b"#[cfg(test)]") {
        let start = from + n;
        from = start + 1;
        let mut j = start + b"#[cfg(test)]".len();
        let skip_space = |j: &mut usize| {
            while bare.get(*j).is_some_and(u8::is_ascii_whitespace) {
                *j += 1;
            }
        };
        skip_space(&mut j);
        if !bare[j..].starts_with(b"mod") || !bare.get(j + 3).is_some_and(u8::is_ascii_whitespace) {
            continue;
        }
        j += 3;
        skip_space(&mut j);
        while bare
            .get(j)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            j += 1;
        }
        skip_space(&mut j);
        if bare.get(j) != Some(&b'{') {
            continue;
        }
        let mut depth = 0usize;
        let mut end = bare.len();
        for (k, &c) in bare.iter().enumerate().skip(j) {
            match c {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = k + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        spaces(&mut code[start..end]);
        spaces(&mut bare[start..end]);
        from = end;
    }
    (
        String::from_utf8_lossy(&code).into_owned(),
        String::from_utf8_lossy(&bare).into_owned(),
    )
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The top-level comma-separated arguments of a call whose `(` was just
/// passed, up to its `)`.
fn split_args(text: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                current.push(c);
                while let Some(c) = chars.next() {
                    current.push(c);
                    match c {
                        '\\' => current.extend(chars.next()),
                        '"' => break,
                        _ => {}
                    }
                }
            }
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' if depth == 0 => {
                args.push(current.trim().to_string());
                args.retain(|a| !a.is_empty());
                return Some(args);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => args.push(std::mem::take(&mut current).trim().to_string()),
            _ => current.push(c),
        }
    }
    None
}

fn string_literal(arg: &str) -> Option<String> {
    arg.strip_prefix('"')?.strip_suffix('"').map(str::to_string)
}

/// What a message (or attribute) needs: the variables its pattern and the
/// messages it references use.
#[derive(Debug, Default)]
struct Messages {
    /// `id` or `id.attribute` → variables used directly.
    variables: BTreeMap<String, BTreeSet<String>>,
    /// `id` or `id.attribute` → message ids it references.
    references: BTreeMap<String, BTreeSet<String>>,
    /// Which resource path defines each message id.
    defined_in: BTreeMap<String, &'static str>,
    /// The translator comments each message id sits under: its own, its
    /// group's and its file's.
    comments: BTreeMap<String, String>,
}

impl Messages {
    fn needed(&self, id: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut out = BTreeSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(id) = stack.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            out.extend(self.variables.get(&id).into_iter().flatten().cloned());
            stack.extend(self.references.get(&id).into_iter().flatten().cloned());
        }
        out
    }
}

/// Problems with the en-US `resources` against the uses under `src`: an id
/// en-US lacks, variables that differ from what the message uses, a syntax
/// error or a duplicate id; and in the `owned` resource paths, a message or
/// attribute that nothing uses or references, or a variable (selectors
/// included) that its message's comment, group comment or file comment
/// doesn't name as `$variable`. Empty when all is well.
pub fn check(src: &Path, resources: &[Resource], owned: &[&str]) -> Vec<String> {
    check_uses(&scan(src), resources, owned)
}

fn check_uses(uses: &[Use], resources: &[Resource], owned: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    let messages = parse(resources, &mut problems);
    let mut used = BTreeSet::new();
    for u in uses {
        let at = format!("{}:{}", u.file.display(), u.line);
        if !messages.variables.contains_key(&u.id) {
            problems.push(format!("{at}: no en-US message `{}`", u.id));
            continue;
        }
        used.insert(u.id.clone());
        let given: BTreeSet<String> = u.args.iter().cloned().collect();
        let needed = messages.needed(&u.id);
        if given != needed {
            problems.push(format!(
                "{at}: `{}` takes {needed:?}, the call passes {given:?}",
                u.id
            ));
        }
    }
    let referenced: BTreeSet<&String> = messages.references.values().flatten().collect();
    for (key, variables) in &messages.variables {
        let id = key.split_once('.').map_or(key.as_str(), |(m, _)| m);
        let path = messages.defined_in[id];
        if !owned.contains(&path) {
            continue;
        }
        if !used.contains(key) && !referenced.contains(key) {
            problems.push(format!("{path}: `{key}` is never used"));
        }
        let comment = &messages.comments[id];
        let unexplained: BTreeSet<&String> = variables
            .iter()
            .filter(|v| !names_variable(comment, v))
            .collect();
        if !unexplained.is_empty() {
            problems.push(format!(
                "{path}: `{key}` uses {unexplained:?}, which no comment above it explains"
            ));
        }
    }
    problems
}

/// Whether `comment` names `$variable` as a whole word.
fn names_variable(comment: &str, variable: &str) -> bool {
    let needle = format!("${variable}");
    comment.match_indices(&needle).any(|(at, _)| {
        !comment[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
    })
}

fn parse(resources: &[Resource], problems: &mut Vec<String>) -> Messages {
    let mut messages = Messages::default();
    for r in resources.iter().filter(|r| r.locale == super::EN_US.id) {
        let resource = match fluent_syntax::parser::parse(r.source) {
            Ok(res) => res,
            Err((res, errors)) => {
                problems.push(format!("en-US/{}: {errors:?}", r.path));
                res
            }
        };
        // A `###` comment speaks for its file, a `##` one for the messages
        // up to the next.
        let (mut file, mut group) = (String::new(), String::new());
        for entry in resource.body {
            let m = match entry {
                ast::Entry::Message(m) => m,
                ast::Entry::ResourceComment(c) => {
                    file = file + "\n" + &c.content.join("\n");
                    continue;
                }
                ast::Entry::GroupComment(c) => {
                    group = c.content.join("\n");
                    continue;
                }
                _ => continue,
            };
            let id = m.id.name.to_string();
            if messages.defined_in.insert(id.clone(), r.path).is_some() {
                problems.push(format!("en-US/{}: `{id}` is defined twice", r.path));
            }
            let own = m.comment.as_ref().map(|c| c.content.join("\n"));
            let comment = [own.unwrap_or_default().as_str(), &group, &file].join("\n");
            messages.comments.insert(id.clone(), comment);
            if let Some(value) = &m.value {
                record(&mut messages, &id, value);
            }
            for a in &m.attributes {
                record(&mut messages, &format!("{id}.{}", a.id.name), &a.value);
            }
        }
    }
    messages
}

fn record(messages: &mut Messages, key: &str, pattern: &ast::Pattern<&str>) {
    let mut variables = BTreeSet::new();
    let mut references = BTreeSet::new();
    walk_pattern(pattern, &mut variables, &mut references);
    messages.variables.insert(key.to_string(), variables);
    messages.references.insert(key.to_string(), references);
}

fn walk_pattern(p: &ast::Pattern<&str>, vars: &mut BTreeSet<String>, refs: &mut BTreeSet<String>) {
    for element in &p.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            walk_expression(expression, vars, refs);
        }
    }
}

fn walk_expression(
    e: &ast::Expression<&str>,
    vars: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    match e {
        ast::Expression::Select { selector, variants } => {
            walk_inline(selector, vars, refs);
            for v in variants {
                walk_pattern(&v.value, vars, refs);
            }
        }
        ast::Expression::Inline(i) => walk_inline(i, vars, refs),
    }
}

fn walk_inline(
    i: &ast::InlineExpression<&str>,
    vars: &mut BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    match i {
        ast::InlineExpression::VariableReference { id } => {
            vars.insert(id.name.to_string());
        }
        ast::InlineExpression::MessageReference { id, attribute } => {
            refs.insert(match attribute {
                Some(a) => format!("{}.{}", id.name, a.name),
                None => id.name.to_string(),
            });
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for p in &arguments.positional {
                walk_inline(p, vars, refs);
            }
            for n in &arguments.named {
                walk_inline(&n.value, vars, refs);
            }
        }
        ast::InlineExpression::Placeable { expression } => walk_expression(expression, vars, refs),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_are_found_across_lines_and_past_nested_commas() {
        let src = r#"
fn f() {
    let a = tr!(l, "plain");
    let b = tr!(
        self.l10n(),
        "with-args",
        model = format!("{}, {}", x, y),
        n = v.len(),
    );
    // tr!(l, "in-a-comment")
    let c = x; // tr!(l, "after-code")
    /* tr!(l, "in-a-block") /* nested */ tr!(l, "still-in-it") */
    let s = "tr!(l, \"in-a-string\")";
    attr!(l, "another-macro");
}
#[cfg(test)]
mod tests {
    fn t() { tr!(l, "in-a-test"); }
}
"#;
        let found: Vec<(String, Vec<String>)> = scan_text(src)
            .into_iter()
            .map(|(_, id, args)| (id, args))
            .collect();
        assert_eq!(
            found,
            [
                ("plain".to_string(), vec![]),
                (
                    "with-args".to_string(),
                    vec!["model".to_string(), "n".to_string()]
                ),
            ]
        );
    }

    /// An inline test module ends at its closing brace, braces in its
    /// literals and comments aside, and one declared out of line hides
    /// nothing.
    #[test]
    fn only_inline_test_modules_are_skipped() {
        let src = r##"
#[cfg(test)]
mod tests;
fn f() { tr!(l, "after-out-of-line"); }
#[cfg(test)]
mod tests {
    fn t() {
        let _ = ('{', "}}", r#"{"#, b'{');
        // }
        tr!(l, "in-a-test");
    }
}
fn g<'a>(s: &'a str) { tr!(l, "after-inline"); }
"##;
        let found: Vec<String> = scan_text(src).into_iter().map(|(_, id, _)| id).collect();
        assert_eq!(found, ["after-out-of-line", "after-inline"]);
    }

    /// Each attribute counts as used on its own, and each variable must be
    /// named in a comment that covers its message.
    #[test]
    fn attributes_are_used_and_variables_explained_one_by_one() {
        let source = "### $file is explained for the whole file.\n\
                      # $own is explained here.\n\
                      a = { $own } { $file }\n    .hover = { $other }\n    .unused = Unused\n\
                      ## $grouped is explained for this group.\n\n\
                      b = { $grouped ->\n   *[x] { $owner }\n}\n\
                      ##\n\n\
                      c = { $grouped }\n";
        let resources = [Resource {
            locale: "en-US",
            path: "t.ftl",
            source,
        }];
        let used = |id: &str, args: &[&str]| Use {
            file: PathBuf::from("f.rs"),
            line: 1,
            id: id.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
        };
        let uses = [
            used("a", &["own", "file"]),
            used("a.hover", &["other"]),
            used("b", &["grouped", "owner"]),
            used("c", &["grouped"]),
        ];
        let problems = check_uses(&uses, &resources, &["t.ftl"]);
        assert_eq!(
            problems,
            [
                "t.ftl: `a.hover` uses {\"other\"}, which no comment above it explains",
                "t.ftl: `a.unused` is never used",
                "t.ftl: `b` uses {\"owner\"}, which no comment above it explains",
                "t.ftl: `c` uses {\"grouped\"}, which no comment above it explains",
            ]
        );
    }

    #[test]
    fn variables_come_from_selectors_and_referenced_messages() {
        let mut problems = Vec::new();
        let resources = [Resource {
            locale: "en-US",
            path: "t.ftl",
            source: "a = { $n ->\n    [one] One { $what }\n   *[other] Many\n}\nb = { a } and { $more }\n    .hover = { $tip }\n",
        }];
        let m = parse(&resources, &mut problems);
        assert!(problems.is_empty(), "{problems:?}");
        let names = |id: &str| m.needed(id).into_iter().collect::<Vec<_>>();
        assert_eq!(names("a"), ["n", "what"]);
        assert_eq!(names("b"), ["more", "n", "what"]);
        assert_eq!(names("b.hover"), ["tip"]);
    }
}
