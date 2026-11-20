//! Spec text translated in parts: the hand-written `lib-spec-templates.ftl`.
//!
//! A template is a message of a spec kind (its ID starts `band-`, `input-`
//! or `note-`) with slots, standing for every text of that kind that reads
//! as its English: `{ $text } (4000 counts)` for "45Hz~1kHz (4000 counts)"
//! and "+40 to +400 °C (4000 counts)". The texts it reads are not listed in
//! `lib-specs.ftl`, so each is translated once however many values it comes
//! with.
//!
//! - `$text` holds another spec text of the same kind, translated on its
//!   own: a message of `lib-specs.ftl`, a text another template reads, or
//!   numbers and units only, which stay as written.
//! - Any other slot holds one value, passed through as written: one number
//!   with its sign, comparator and unit ([`is_value`]), so a slot never
//!   swallows a clause.
//!
//! The generator (`lib_ftl`) asserts each library text reads one way at
//! most; [`Localizer::spec_text`](super::Localizer::spec_text) composes.

use super::lib_index::{is_value, kind_of_id};
use dmm_lib::text::TextKind;
use fluent_syntax::ast::{Entry, Expression, InlineExpression, PatternElement};

/// The hand-written file of spec templates.
pub(crate) const LIB_SPEC_TEMPLATES: &str = "lib-spec-templates.ftl";

/// The slot that holds another spec text.
pub(crate) const TEXT_SLOT: &str = "text";

#[derive(Debug, PartialEq, Eq)]
enum Part {
    Literal(String),
    Slot(String),
}

/// One message of the en-US `lib-spec-templates.ftl`.
#[derive(Debug)]
pub(crate) struct Template {
    pub id: String,
    pub kind: TextKind,
    parts: Vec<Part>,
}

/// What each slot of a template holds in one text, in pattern order.
pub(crate) type Binding<'t> = Vec<(&'t str, &'t str)>;

/// The templates of an en-US `lib-spec-templates.ftl` `source`, or what is
/// wrong with it: an ID that names no spec kind, a pattern of anything but
/// text and variables, two slots with no text between them.
pub(crate) fn templates(source: &str) -> Result<Vec<Template>, Vec<String>> {
    let resource = fluent_syntax::parser::parse(source)
        .map_err(|(_, errors)| vec![format!("syntax errors: {errors:?}")])?;
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for entry in &resource.body {
        let Entry::Message(message) = entry else {
            continue;
        };
        let id = message.id.name;
        let kind = kind_of_id(id).filter(|k| super::lib_index::GUARDED.contains(k));
        let parts: Option<Vec<Part>> = message.value.as_ref().and_then(|pattern| {
            pattern
                .elements
                .iter()
                .map(|element| match element {
                    PatternElement::TextElement { value } => Some(Part::Literal(value.to_string())),
                    PatternElement::Placeable {
                        expression: Expression::Inline(InlineExpression::VariableReference { id }),
                    } => Some(Part::Slot(id.name.to_string())),
                    PatternElement::Placeable { .. } => None,
                })
                .collect()
        });
        match (kind, parts) {
            (None, _) => problems.push(format!("`{id}` names no spec kind")),
            (_, None) => problems.push(format!("`{id}` is not text and variables only")),
            (Some(_), Some(parts))
                if parts
                    .windows(2)
                    .any(|w| matches!(w, [Part::Slot(_), Part::Slot(_)])) =>
            {
                problems.push(format!("`{id}` has two slots with no text between"));
            }
            (Some(kind), Some(parts)) => out.push(Template {
                id: id.to_string(),
                kind,
                parts,
            }),
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

impl Template {
    /// Every way `text` reads as this template. A value slot takes one
    /// value ([`is_value`]); the text slot takes anything, which the caller
    /// judges.
    pub(crate) fn parses<'t>(&'t self, text: &'t str) -> Vec<Binding<'t>> {
        let mut found = Vec::new();
        self.read(0, text, &mut Vec::new(), &mut found);
        found
    }

    fn read<'t>(
        &'t self,
        at: usize,
        rest: &'t str,
        bound: &mut Binding<'t>,
        found: &mut Vec<Binding<'t>>,
    ) {
        match self.parts.get(at) {
            None => {
                if rest.is_empty() {
                    found.push(bound.clone());
                }
            }
            Some(Part::Literal(literal)) => {
                if let Some(rest) = rest.strip_prefix(literal.as_str()) {
                    self.read(at + 1, rest, bound, found);
                }
            }
            Some(Part::Slot(name)) => {
                // The slot ends where the literal after it starts, or at
                // the end of the text.
                let ends: Vec<usize> = match self.parts.get(at + 1) {
                    Some(Part::Literal(next)) => {
                        rest.match_indices(next.as_str()).map(|(i, _)| i).collect()
                    }
                    _ => vec![rest.len()],
                };
                for end in ends {
                    let held = &rest[..end];
                    let fits = if name == TEXT_SLOT {
                        !held.is_empty()
                    } else {
                        is_value(held)
                    };
                    if fits {
                        bound.push((name.as_str(), held));
                        self.read(at + 1, &rest[end..], bound, found);
                        bound.pop();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(pattern: &str) -> Template {
        templates(&format!("band-x = {pattern}\n"))
            .unwrap()
            .remove(0)
    }

    #[test]
    fn a_wrapper_reads_the_text_it_goes_around() {
        let counts = one("{ $text } (4000 counts)");
        assert_eq!(
            counts.parses("+40 to +400 °C (4000 counts)"),
            [vec![("text", "+40 to +400 °C")]]
        );
        // Nothing to go around, or something after it.
        assert!(counts.parses("(4000 counts)").is_empty());
        assert!(counts.parses(" (4000 counts)").is_empty());
        assert!(counts.parses("1kHz (4000 counts) typ.").is_empty());
    }

    #[test]
    fn a_value_slot_holds_one_value() {
        let range = one("{ $range }: { $text }");
        assert_eq!(
            range.parses("4.000: 250kHz~400kHz, 10 - 30% of MR"),
            [vec![
                ("range", "4.000"),
                ("text", "250kHz~400kHz, 10 - 30% of MR")
            ]]
        );
        assert_eq!(
            range.parses(">5A: max. 10s"),
            [vec![("range", ">5A"), ("text", "max. 10s")]]
        );
        // Words, a list and a span are not one value.
        assert!(range.parses("Typical: Vrms=1 V").is_empty());
        assert!(range.parses("6nF, 60nF: subtract").is_empty());
        assert!(range.parses("10Hz–200kHz: pulse width >2µs").is_empty());
    }

    #[test]
    fn a_template_file_is_checked() {
        assert!(templates("mode-x = { $text } y\n").is_err());
        assert!(templates("band-x = { $a }{ $b }\n").is_err());
        assert!(templates("band-x = { NUMBER($a) } y\n").is_err());
        assert_eq!(
            templates("band-x = { $a } y\n").unwrap()[0].kind,
            TextKind::Band
        );
    }
}
