//! The library's words, by kind, for apps that translate them.
//!
//! The library speaks English and keeps doing so: its labels are also match
//! keys, `set` arguments, export columns and saved settings, so they never
//! change language here. An app that translates looks a label up by its
//! English text and its [`TextKind`] at the moment it draws it. The kind
//! matters because one English word can be a mode on one meter and a
//! sub-value on another, and a translator may want different words for each.
//!
//! A family that builds a mode label by putting a fixed word around another
//! mode ("LPF AC V") takes the word from a [`TextKind::Qualifier`] pattern
//! and lists the pattern, so an app can translate the word once rather than
//! every combination.
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
    /// A word a family puts around a mode's label, as a pattern with
    /// [`MODE_SLOT`] for the mode: "LPF {mode}" makes "LPF AC V" of "AC V".
    Qualifier,
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
    pub const ALL: [TextKind; 12] = [
        TextKind::Mode,
        TextKind::Qualifier,
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
            TextKind::Qualifier => "qualifier",
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

/// Where a [`TextKind::Qualifier`] pattern puts the mode.
pub const MODE_SLOT: &str = "{mode}";

/// The mode label qualifier `pattern` makes of `mode`.
pub(crate) fn qualify(pattern: &str, mode: &str) -> String {
    pattern.replacen(MODE_SLOT, mode, 1)
}

#[cfg(test)]
mod tests {
    use super::TextKind;
    use std::collections::BTreeSet;

    /// The kinds named once, as an exhaustive `match` and as a list: a kind
    /// added to [`TextKind`] fails to compile here until it is named.
    macro_rules! every_kind {
        ($($kind:ident),* $(,)?) => {{
            let _exhaustive = |k: TextKind| match k {
                $(TextKind::$kind)|* => {}
            };
            [$(TextKind::$kind),*]
        }};
    }

    /// [`TextKind::ALL`] is kept by hand; a kind it leaves out would get no
    /// IDs in the generated `lib.ftl`.
    #[test]
    fn all_lists_every_kind() {
        let every = every_kind!(
            Mode, Qualifier, Range, Aux, Toggle, Key, KeyHover, Model, Activation, Table, Input,
            Note,
        );
        let all: BTreeSet<TextKind> = TextKind::ALL.into_iter().collect();
        assert_eq!(all.len(), TextKind::ALL.len(), "ALL repeats a kind");
        assert_eq!(all, every.into_iter().collect());
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use list::{Labels, all_texts};
#[cfg(test)]
pub(crate) use list::{assert_golden_listed, assert_listed, listed};

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

        /// The strings of `kinds` listed as used by device `by`.
        #[cfg(test)]
        pub(crate) fn of_device(
            &self,
            by: &str,
            kinds: &[TextKind],
        ) -> BTreeSet<(TextKind, String)> {
            self.entries()
                .filter(|(kind, _, users)| kinds.contains(kind) && users.contains(by))
                .map(|(kind, text, _)| (kind, text.to_string()))
                .collect()
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
        crate::protocol::all_labels(&mut out);
        out
    }

    /// Whether a model name says something in words ("Mock UT61E+") rather
    /// than being a part number alone ("UT61E+", "BM869s").
    fn has_words(model: &str) -> bool {
        model
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|w| w.len() >= 3 && w.chars().any(|c| c.is_ascii_lowercase()))
    }

    /// [`all_texts`], built once for the family tests that check against it.
    #[cfg(test)]
    pub(crate) fn listed() -> &'static Labels {
        static LISTED: std::sync::OnceLock<Labels> = std::sync::OnceLock::new();
        LISTED.get_or_init(all_texts)
    }

    /// Panic unless every label of each golden fixture under
    /// `tests/golden/<device>/`, decoded with that device's parser, is
    /// listed for it ([`assert_listed`]).
    #[cfg(test)]
    pub(crate) fn assert_golden_listed(all: &Labels, device: &'static str) {
        let entry = registry::find_device(device).expect("a registry id");
        let protocol = (entry.new_protocol)();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(device);
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "yaml"))
            .collect();
        paths.sort();
        assert!(!paths.is_empty(), "no fixtures in {}", dir.display());
        for path in paths {
            let yaml = std::fs::read_to_string(&path).unwrap();
            let hex = yaml
                .lines()
                .find_map(|l| l.strip_prefix("raw_hex:"))
                .unwrap_or_else(|| panic!("{}: no raw_hex", path.display()));
            let hex: String = hex.chars().filter(char::is_ascii_hexdigit).collect();
            let payload: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let m = protocol
                .parse_payload(&payload)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_listed(all, protocol.as_ref(), &m, device);
        }
    }

    /// Panic unless every label of reading `m`, and of the choices
    /// `protocol` offers from it, is listed under its kind as used by device
    /// `by`. Choices named by a [`Flag`] label are the apps' to translate by
    /// variant and are skipped.
    ///
    /// [`Flag`]: crate::flags::Flag
    #[cfg(test)]
    pub(crate) fn assert_listed(
        all: &Labels,
        protocol: &dyn crate::protocol::Protocol,
        m: &crate::measurement::Measurement,
        by: &str,
    ) {
        use crate::flags::Flag;
        use crate::protocol::Setting;
        let check = |kind: TextKind, text: &str| {
            if text.trim().is_empty() {
                return;
            }
            let users = all.0.get(&(kind, Cow::Owned(text.to_string())));
            assert!(
                users.is_some_and(|u| u.contains(by)),
                "{by}: {kind:?} {text:?} is not listed for it (listed for {users:?})"
            );
        };
        check(TextKind::Mode, &m.mode);
        check(TextKind::Range, &m.range_label);
        for aux in &m.aux_values {
            check(TextKind::Aux, &aux.label);
        }
        for setting in Setting::ALL {
            let kind = match setting {
                Setting::Mode => TextKind::Mode,
                Setting::Range => TextKind::Range,
                _ => TextKind::Toggle,
            };
            for choice in protocol.choices(setting, m) {
                if !Flag::ALL
                    .iter()
                    .any(|f| f.label() == Some(choice.label.as_ref()))
                {
                    check(kind, &choice.label);
                }
            }
        }
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

        /// Every meter shows at least one mode, so a device no family's
        /// `all_labels` lists under a mode is a device it forgot.
        #[test]
        fn every_device_has_its_modes_listed() {
            let all = all_texts();
            let with_modes: BTreeSet<&str> = all
                .entries()
                .filter(|(kind, _, _)| *kind == TextKind::Mode)
                .flat_map(|(_, _, by)| by.iter().copied())
                .collect();
            for device in registry::DEVICES {
                assert!(with_modes.contains(device.id), "{}", device.id);
            }
        }

        /// Each qualifier is a word around exactly one mode slot.
        #[test]
        fn qualifiers_hold_one_mode_slot_and_a_word() {
            use crate::text::{MODE_SLOT, qualify};
            let all = all_texts();
            let qualifiers: Vec<&str> = all
                .entries()
                .filter(|(kind, _, _)| *kind == TextKind::Qualifier)
                .map(|(_, text, _)| text)
                .collect();
            assert!(!qualifiers.is_empty());
            for q in qualifiers {
                assert_eq!(q.matches(MODE_SLOT).count(), 1, "{q:?}");
                assert!(q.len() > MODE_SLOT.len(), "{q:?}");
                assert_eq!(qualify(q, "DC V"), q.replace(MODE_SLOT, "DC V"));
            }
        }

        /// The library's diagnostic fallbacks stay English and unlisted.
        #[test]
        fn fallbacks_are_not_listed() {
            for (kind, text, _) in all_texts().entries() {
                assert!(
                    !crate::protocol::is_unknown_mode(text) && !text.starts_with("range "),
                    "{kind:?} {text:?}"
                );
            }
        }
    }
}
