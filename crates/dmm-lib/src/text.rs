//! The library's words, by kind, for apps that translate them.
//!
//! The library speaks English and keeps doing so: its labels are also match
//! keys, `set` arguments, export columns and saved settings, so they never
//! change language here. An app that translates looks a label up by its
//! English text and its [`TextKind`] at the moment it draws it. The kind
//! matters because one English word can be a mode on one meter and a
//! sub-value on another, and a translator may want different words for each.
//!
//! [`all_texts`] lists every such string, for the app side to keep its
//! translation catalogue complete. Fallbacks the library builds from bytes it
//! did not recognise (`Unknown(0x..)`, `range 3`) are not listed: they stay
//! English, as a diagnostic.

/// What kind of text a library string is, as an app looks it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextKind {
    /// A measurement mode, `Measurement::mode`, and the mode choices a meter
    /// offers ("DC V", "Capacitance").
    Mode,
    /// A range, `Measurement::range_label`, and the range choices ("Auto").
    Range,
    /// A sub-value's label, `AuxValue::label` ("Frequency").
    Aux,
    /// An on/off state of a meter setting offered as a choice ("on").
    Toggle,
    /// A meter key's name on a remote button ("SEL").
    Key,
    /// How the meter's keypad does what a remote button does, as its hover.
    KeyHover,
    /// A model name with words in it, not just a part number ("Mock UT61E+").
    Model,
    /// The steps that switch a meter's data transmission on.
    Activation,
    /// A specification table's name ("(1) AC Voltage").
    Table,
    /// A specification table's input impedance or overload protection.
    Input,
    /// A note under a specification table.
    Note,
}

impl TextKind {
    pub const ALL: [TextKind; 11] = [
        TextKind::Mode,
        TextKind::Range,
        TextKind::Aux,
        TextKind::Toggle,
        TextKind::Key,
        TextKind::KeyHover,
        TextKind::Model,
        TextKind::Activation,
        TextKind::Table,
        TextKind::Input,
        TextKind::Note,
    ];

    /// One word for the kind, for message ids and catalogue headings.
    pub fn name(self) -> &'static str {
        match self {
            TextKind::Mode => "mode",
            TextKind::Range => "range",
            TextKind::Aux => "aux",
            TextKind::Toggle => "toggle",
            TextKind::Key => "key",
            TextKind::KeyHover => "key-hover",
            TextKind::Model => "model",
            TextKind::Activation => "activation",
            TextKind::Table => "table",
            TextKind::Input => "input",
            TextKind::Note => "note",
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use list::{Labels, all_texts};

#[cfg(any(test, feature = "test-support"))]
mod list {
    use super::TextKind;
    use crate::protocol::registry;
    use std::borrow::Cow;
    use std::collections::{BTreeMap, BTreeSet};

    /// Strings collected by kind, each with the devices that use it.
    #[derive(Debug, Default)]
    pub struct Labels(BTreeMap<(TextKind, Cow<'static, str>), BTreeSet<&'static str>>);

    impl Labels {
        /// Record `text` as `kind`, used by the device with registry id `by`.
        /// Empty text is nothing to translate and is dropped.
        pub(crate) fn add(
            &mut self,
            kind: TextKind,
            text: impl Into<Cow<'static, str>>,
            by: &'static str,
        ) {
            let text = text.into();
            if !text.trim().is_empty() {
                self.0.entry((kind, text)).or_default().insert(by);
            }
        }

        /// Every string, sorted by kind then text, with the registry ids of
        /// the devices that use it.
        pub fn entries(&self) -> impl Iterator<Item = (TextKind, &str, &BTreeSet<&'static str>)> {
            self.0.iter().map(|((k, t), by)| (*k, t.as_ref(), by))
        }

        pub fn contains(&self, kind: TextKind, text: &str) -> bool {
            self.0.contains_key(&(kind, Cow::Owned(text.to_string())))
        }
    }

    /// Every string the library can show a user, by kind: what the registry
    /// and the specification tables hold, and every label each protocol
    /// family can produce.
    pub fn all_texts() -> Labels {
        let mut out = Labels::default();
        for device in registry::DEVICES {
            let id = device.id;
            if has_words(device.model) {
                out.add(TextKind::Model, device.model, id);
            }
            out.add(TextKind::Activation, device.activation_instructions, id);
            let protocol = (device.new_protocol)();
            let keys = protocol.profile().meter_keys;
            for key in keys.functions.iter().chain(keys.context) {
                out.add(TextKind::Key, key.label, id);
                if let Some(hover) = key.hover {
                    out.add(TextKind::KeyHover, hover, id);
                }
            }
            for table in protocol.spec_sheet() {
                out.add(TextKind::Table, table.name, id);
                let mode = table.mode;
                for input in [mode.input_impedance, mode.overload_protection] {
                    out.add(TextKind::Input, input.unwrap_or_default(), id);
                }
                for note in mode.notes {
                    out.add(TextKind::Note, *note, id);
                }
            }
        }
        out
    }

    /// Whether a model name says something in words ("Mock UT61E+") rather
    /// than being a part number alone ("UT61E+", "BM869s").
    fn has_words(model: &str) -> bool {
        model
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|w| w.len() >= 3 && w.chars().any(|c| c.is_ascii_lowercase()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn only_worded_models_are_listed() {
            assert!(has_words("Mock UT61E+"));
            for part in ["UT61E+", "BM869s", "VC650BT", "ET-2517A", "121GW"] {
                assert!(!has_words(part), "{part}");
            }
        }

        #[test]
        fn the_registry_and_the_spec_tables_are_listed() {
            let all = all_texts();
            for device in registry::DEVICES {
                assert!(
                    all.contains(TextKind::Activation, device.activation_instructions),
                    "{}",
                    device.id
                );
            }
            assert!(all.contains(TextKind::Model, "Mock UT61E+"));
            let kinds: BTreeSet<TextKind> = all.entries().map(|(k, _, _)| k).collect();
            for kind in [
                TextKind::Activation,
                TextKind::Model,
                TextKind::Key,
                TextKind::Table,
                TextKind::Input,
                TextKind::Note,
            ] {
                assert!(kinds.contains(&kind), "nothing of kind {kind:?}");
            }
        }
    }
}
