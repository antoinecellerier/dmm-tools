# Localization Design

dmm-gui and dmm-cli speak the user's language using [Fluent](https://projectfluent.org/): English, Brazilian Portuguese, French, Japanese, Simplified Chinese and Spanish. The first request came in [issue #35](https://github.com/antoinecellerier/dmm-tools/issues/35). A reporter whose Brymen meters are sold in Brazil under the Minipa brand ran `capture` through their own wrapper, which rewrote its prompts into Brazilian Portuguese. Because the wrapper piped stdio, the review step and `detect` were skipped. The reporter named the capture flow as the place translation helps most, and offered to contribute and review a pt-BR translation. `capture` now speaks the user's language, and its step instructions can be read in the user's own translation (see [Ruled out](#ruled-out)), so a run needs no wrapper.

One principle runs through every section. **Canonical English stays the data.** The library's strings stay what they are and keep serving as keys, file contents and saved settings. Translation happens in the apps, at the moment text reaches the screen or the terminal.

## What gets translated

Translated:

- GUI text: labels, tooltips, toasts, dialogs, accessibility names, the Linux desktop entry's comment
- `dmm_shared::help`, which both binaries print
- library labels: modes, ranges, sub-values, flags, links, meter keys
- library errors, device names, activation steps, alarm limits and breach notes
- spec-data notes, guarded (see [Library fixed text](#library-fixed-text))
- the CLI's stderr prose and `--help`, clap's headings and command-line errors included
- dmm-gui's `--help` and the command-line errors it prints before its window opens
- `capture`'s own prompts, notices and closing lines, which a shipped language may leave English

Stays English:

- every data format: CSV, JSON, replay, capture YAML, export file names
- `read`'s stdout line, `get --format json`, and every token typed on a command line, such as `set` arguments
- logs, including the library warnings shown by default
- in `capture`: the step instructions, unless the user translates them for themselves; the readings it echoes and the report; `--list-steps` and `triage`
- What's New (`CHANGELOG.md`), the shell completion scripts
- the user docs and their screenshots, but for the README and its picture (see [User documentation](#user-documentation))
- unit symbols, decimal points, timestamps — except in what a screen reader says (see [Writing messages](#writing-messages))
- theme names, which are also setting values, `--theme` arguments and file names

About 1,450 messages in all, plus about 240 attributes for tooltips, accessibility names and long help:

| Source | Messages |
|---|---|
| GUI (`gui/`) | ~480 |
| CLI prose (`cli/`) | ~105 |
| `capture`'s own text (`cli/capture.ftl`) | ~110 |
| CLI `--help` (`cli-help.ftl`) | ~50 |
| GUI `--help` (`gui-help.ftl`, `gui/command-line.ftl`) | ~15 |
| `dmm_shared::help`, clap's words and errors (`shared.ftl`) | ~95 |
| Library labels and keys (`lib.ftl`) | ~220 |
| Activation steps (`lib.ftl`) | 28 |
| Spec text (`lib.ftl`) | ~250 |
| Library names, errors and alarms (`lib-*.ftl`) | ~100 |

**Numbers keep their point.** The meter's LCD shows a `.`, input fields parse only `.`, and the reference-lines field accepts `,` (alongside `;` and a space) as a separator, so `1,5` reads as two values. A localized decimal comma would break all three.

**Logs stay English.** An issue report then always comes with diagnostics the maintainers can read. `triage` also parses library log lines as data.

## Library labels are identifiers too

`Measurement.mode`, `unit` and `range_label`, `AuxValue.label` and `Choice.label` are English `Cow<'static, str>` values. For the first three, see "Static strings in readings" in [architecture.md](architecture.md). Most come from static tables, but some families build them at run time. Each one is also:

- a match key for spec tables and capture expectations;
- the text a user types in `dmm-cli set mode`;
- a value in the CSV/JSON `mode` and `auxN_label` columns, and part of the export file name;
- a saved setting key: the GUI remembers hidden traces by aux label.

`Flag` already separates the two roles: `name()` is the key and `label()` is the text. But the CSV `flags` cell holds the labels, and import reads them back, so they are part of the file format.

**Translate at display time.** Renaming a label at its source would change exported files, break saved settings and break the `set` arguments people already use. Looking up a translation just before drawing changes none of these.

**dmm-lib stays Fluent-free.** The library stays self-contained (`.claude/rules/protocol.md`). It provides English text and structure, and the apps provide the translations.

## Fluent in the apps

An `l10n` module in `dmm-shared` holds the loader, the language choice and the shared messages, because both binaries must agree on them. Resources go in `i18n/<locale>/`, one file per screen area, with `en-US` as the reference.

**Plain `fluent-bundle`.** It adds a dozen crates and reads several files per language into one bundle. `i18n-embed` with `i18n-embed-fl` checks message IDs at compile time, but adds over forty crates, fails `cargo deny` on an unmaintained dependency, and takes one file per crate. `fluent-templates` loads into a static and finds a broken file only at run time. In place of `fl!`, `tr!` takes only a literal ID, and a test checks every ID and variable used against the en-US files.

**Translations can be tried without a build.** `DMM_TOOLS_L10N_DIR` points at a folder whose `<locale>/` files load over the built-in ones, and a folder for a new locale makes that language selectable. A translator can then check their work in a release binary.

**A handle in the GUI, one localizer per run in the CLI.** The GUI keeps its localizer in its app state, and switching language replaces it. Tests use an immutable English localizer, or build one pinned to the pseudo-locale, so tests running in parallel don't race on shared state. A dmm-cli run speaks one language from its first line to its last, from every corner of the command tree, so `main` sets one process-wide localizer; unit tests never set it, and read English. dmm-gui's `main` builds the first localizer before it parses the command line, so `--help` and the errors before the window opens speak the window's language, then hands it to the app state.

**Embedded resources.** The `.ftl` files are compiled into the binaries, so a release archive keeps working wherever it is unpacked.

**No bidi isolation marks.** By default Fluent wraps each placeable in U+2068/U+2069. egui hides them when drawing, but they would still end up in stderr, copied text, screen-reader names and test assertions. `set_use_isolating(false)` turns them off. The right-to-left scripts that need them are out of scope anyway (see [Scripts, fonts and layout](#scripts-fonts-and-layout)).

**Pre-formatted numbers.** A raw `f64` passed to Fluent loses the app's precision: `5.0` prints as "5". Readings and statistics go into messages as already-formatted strings. Formatted in Rust, they keep the English decimal point in every language (`1.5 h`, `1.2K`, a reading's decimals): locale number formatting is out of scope.

**No message cache.** Each frame formats its text again, about 0.2 µs per message: 25–70 µs a frame, against frames of 0.6–1.8 ms. A cache, and clearing it on a language switch, would pay off only past about 0.3 ms.

## Choosing the language

A `language` field in `SharedSettings` (empty means follow the system) is shared by both binaries. dmm-cli takes `--lang`, and the GUI's Settings has a Language row listing System, then each language by its own name. dmm-gui takes no `--lang`: its `--help` speaks the language its window would. The GUI switches immediately, because an immediate-mode UI redraws every label on the next frame anyway; a notice or toast already on screen keeps its language.

Without a setting, `sys-locale` reports the system language: the UI language on Windows, the preferred-languages list on macOS, and the locale variables on Unix. On Unix it returns `LANGUAGE` first and reduces `C.UTF-8` to "C", so it can't apply gettext's C-locale rule itself.

**C locale means English.** When neither `--lang` nor the setting names a language, the app takes the first non-empty value of `LC_ALL`, `LC_MESSAGES` and `LANG`, as gettext does, on every platform: the test helpers rely on it on the macOS and Windows runners too. If that value is `C` or `POSIX` (including `C.UTF-8`), the app uses en-US and ignores `LANGUAGE`; on Linux and the BSDs, where these variables are the system's language, so does finding none of the three set, as glibc does. Only otherwise does it ask `sys-locale`, skipping any `C` or `POSIX` in its list. Scripts, cron jobs and the screenshot pipeline set `LC_ALL=C` and need English, even on Debian and Ubuntu desktops, which set `LANGUAGE`.

## Library fixed text

Each kind of library text is translated by the mechanism that fits it:

- **Enum labels.** `Flag`, `Link`, `MainLabel` and `Brand` are enums, so their names translate by variant, through an exhaustive `match` over the hand-written `lib-names.ftl`. Of the brands only the simulated meters' heading translates: real brands are names. `Stability` translates with the help text that tags device lists.
- **Strings.** Mode, range and aux labels, meter keys, device names, activation steps and spec notes are strings. The apps look them up the way gettext does, by their English text plus the kind of text the caller is drawing.

On its first library lookup, `l10n` reads the en-US `lib.ftl` and builds a map from (kind, English text) to message ID. The kind matters: "Capacitance" is a mode on one meter and a sub-value on another, and translators may want different words for each. IDs are readable (`mode-capacitance`, `aux-period`), never hashed. Activation steps all start alike, so theirs name the first meter that shows them (`activation-ut181a`). Lookup is case-sensitive. A label read from a replay or an import is found the same way as a live one.

**Unit-only labels are not listed.** A label whose letters are all unit symbols and SI prefixes, such as "200mV", "-40~1000°C" or "kΩ", has nothing to translate: unit symbols stay as they are. It gets no message, and its lookup returns the English. That leaves out almost every range, and modes such as "mA". Words printed beside a unit, such as "AC", "DC", "Auto" and "REL", are listed, because some languages translate them. The symbol list is in `crates/dmm-shared/src/l10n/lib_index.rs`, which spec notes check their numbers against too.

**Mode qualifiers are translated once.** Some families build a mode by putting a word around another: "LPF" and "Peak" before it on OWON's meters, "peak" after it on ZOTEK's, "REL" after it on the UT181A. Their decoders accept every combination, most of which no real meter shows, and translators shouldn't face each one. The family takes the word from one pattern ("LPF {mode}") and lists it as a qualifier, which `lib.ftl` carries once as a message with a `$mode` variable that a translator may move. The modes it composes are left out. The lookup tries a mode whole, then as a qualifier around a mode it can look up: "LPF Peak AC V" is LPF around Peak around "AC V".

The en-US `lib.ftl` is generated:

- A `test-support` enumerator in dmm-lib lists every string with its kind. Device names and activation steps come from the registry, notes from the spec tables, and labels from each family's `all_labels()`.
- Each family's `all_labels()` runs its own label builder over every possible input. That includes labels built at run time from a finite set of parts, such as a mode with "Peak" or "REL" appended. Fallbacks like `Unknown(0x..)` stay English.
- A test in dmm-shared regenerates the file on request, the way dmm-cli's `docs_snippets.rs` keeps the docs' CLI examples current.
- The generator only adds entries and reports orphaned ones. It never renames an ID, so the maintainer chooses: correct the English under its existing ID, or give a changed meaning a new ID.
- A coverage test replays the golden fixtures and the mock devices, and asserts that every label they produce is in `lib.ftl`.

**English text is the lookup key.** dmm-lib's strings and stored data don't change; for translation the library carries only `all_labels()` and the enumerator. Translators decide label by label, abbreviations and flags included. One lookup serves the whole reading area, so labels don't come out half-translated. A translator may keep "DC V" and "HOLD" as the meter prints them.

**Spec numbers survive translation.** A translated spec note, input (impedance or overload protection) or accuracy band ("In REL state", "Not Specified") must carry the same numbers as its English, in any order, since a translation may reorder clauses. Each number keeps its sign, its ± and the unit symbol after it, compared with spacing dropped: "About 10MΩ" may become "Environ 10 MΩ", but "-40°C" may not become "40°C". A comma between digits reads as a decimal point, so "0,6" matches "0.6"; regrouped thousands ("1 000") don't match, and the English shows. Each message is checked once, the first time a localizer is asked for spec text, and one that fails is shown in English. The check can't catch a flipped comparison or a lost "typical", so the specifications panel shows the English on hover wherever the text differs. `Localizer::spec_text` in `crates/dmm-shared/src/l10n/lib_index.rs` holds the rules.

## Library run-time text

Some library errors build their English message at run time with `format!`, so their text cannot be looked up. They split by who reads them:

| Group | Errors | Treatment |
|---|---|---|
| Developer diagnostics | invalid response, checksum mismatch, unknown mode byte, replay parse, HID | Lead-in translated, detail stays English |
| Already structured | `Timeout`, `LinkLost`, `AdapterNotFound`, `UnknownDevice`, `DeviceNotIdentified`, `WrongCable`, `NoTransportFound`, `BluetoothOnly` | Translated by variant and fields |
| Built from sentences | `UnsupportedCommand`, `CommandRejected`, `Bluetooth`, about 40 messages, many in `protocol/cycle.rs` | A reason enum per error |

The reason enums' details:

- `Bluetooth` keeps a variant that passes the Bluetooth stack's own error text through unchanged.
- Button names (RANGE, SELECT) inside a reason stay as the meter prints them.
- Mode labels inside a reason go through the lookup in [Library fixed text](#library-fixed-text).

The library's `Display` stays the English rendering, used by logs and by library users without Fluent. `dmm-shared` renders each error and reason through the hand-written `lib-errors.ftl`. Tests render samples through en-US and check that the result matches the library's English.

**Structure, not sentences.** The renderer is an exhaustive `match`, so a reason added without a message fails to compile. Matching on error text would fail silently instead.

**Alarm text comes from the breach's fields.** The library words an alarm's limits and breach notes with `format!`; `lib-alarm.ftl` words them again from `Breach`'s fields. A breach's marker note is written in the user's language, as a note they typed would be, since they read it and can edit it; recordings and exports keep it as written.

## Writing messages

- **Whole sentences.** A message never joins fragments. What would otherwise be assembled from pieces is one message using Fluent selectors and CLDR plural rules: the "{n} unexported {samples} and unsaved changes to {markers}" prompt, its plural nouns, and the screen reader's live-region summary.
- **Spoken units by name.** A screen reader spells a unit symbol out letter by letter ("mV" as "m V"), so the spoken reading and the plot's summary say units by name, in the language. One message per base unit takes the SI prefix's word: joining a prefix to a unit is the one fragment join allowed, as it holds in every language the apps speak. Ohm also takes the prefix's key, for the pairs a language contracts (English kilohm). The plural follows the number as shown (`Arg::Decimal`), so English says "1.000 volts" and French "1.5 volt". One message joins a number and its unit's name, told whether the unit is a percentage, as Chinese says percent before the number (百分之五十). The screen keeps the symbols.
- **Shortcuts as placeables.** Tooltips take their shortcut text from the key bindings instead of a hard-coded "(Ctrl+O)", which also shows Cmd on macOS.
- **Key names are messages.** Keyboards print their keys in the local language, so Ctrl, Space and the arrows each have a message (`shortcuts-key-*`); on a Mac the modifiers show as symbols.
- **Easy to miss:** the file-type names in rfd's save and open dialogs, accessibility names set through `WidgetInfo`, and the `Comment` line of the Linux desktop entry: the GUI writes the entry from `assets/dmm-tools.desktop`, which stays English, adding a `Comment[<locale>]` line for each language that translates `desktop-comment`, as the desktop picks the line by its own locale, not the app's.

## Scripts, fonts and layout

| Script | Coverage |
|---|---|
| Latin, Cyrillic | Bundled fonts (Ubuntu-Light, with Hack as fallback) |
| Greek | Partial in Ubuntu-Light; check before offering a Greek translation |
| Chinese, Japanese, Korean | Only through a system font, the language's own before the generic sans-serif list (`app/system_fonts.rs`), so it depends on the user's installed fonts |
| Arabic, Hebrew, Persian | Not possible: egui 0.37 has no bidirectional text or right-to-left layout |

Most of the layout measures its text. The places longer strings strain are:

- fixed widths on toasts, the shortcut-help columns and the specs columns;
- truncated labels in the device and marker lists;
- the top bar, whose width sets the window's minimum width;
- the big meter, whose fit shrinks the reading when the notice title above it gets longer.

**A padding pseudo-locale finds layout bugs.** `fluent-pseudo` lengthens text by doubling lowercase vowels, which leaves capitals like HOLD, REL and AUTO, and the reading area full of them, at their original width. Instead, a custom `set_transform` adds accents and pads every string by about 40%, and each message is bracketed, so text without brackets never went through the translations. `DMM_TOOLS_PSEUDO_LOCALE` switches it on ([development.md](development.md#translations)); verify-gui can set it, and the pseudo-locale never appears in the Language row.

## Tests and pipelines

Many tests check English text or find a widget by its label. Each builds a localizer pinned to en-US, so a developer with a French desktop gets the same results. `scripts/doc-screenshots.sh` and verify-gui set `LC_ALL=C`, which the C-locale rule turns into English. Doc screenshots stay English, as their scenes click at positions that assume English widths, but for the README's hero, taken once per README language: its scene loads its state from the recording and clicks only an empty spot. Its Chinese and Japanese shots run under a fontconfig file that hides every CJK face but the language's own, so the picture shows the font each language is meant to draw in, whatever else is installed. A stock Debian desktop draws Chinese in the Japanese face for now ([Localization follow-ups](future-improvements.md#localization-follow-ups)).

The checks on the resources, listed in [development.md](development.md#translations), fail the build on a broken translation (a wrong variable or plural, a lost `--option`, `code` span or spec number), on a `tr!` call naming no en-US message, and on a library label missing from `lib.ftl`. They never fail on a missing translation.

## Translations

- **Contributions** start as pull requests against the `.ftl` files, which are readable plain text. The `CONTRIBUTING.md` section covers how to add or update a language, and what a language needs to ship.
- **Weblate** can come later, once translators who aren't developers turn up. It has supported Fluent since 4.8 and opens pull requests for review. It has no plural editor for Fluent, so translators edit select expressions as text.
- **Machine-drafted languages ship marked.** A language is drafted offline from its glossary and style sheet (`i18n/<lang>/`), themselves built from meter manuals in that language and Microsoft's terminology and style guides. Until a native speaker has reviewed it, its `SHIPPED` entry says `reviewed: false`: the Language row marks it machine-translated, in its own words, and a line under the row asks its speakers to review it.
- **A message ID is renamed** when the meaning of the English changes, so stale translations fall back to English rather than saying the wrong thing. A correction that keeps the meaning keeps its ID and its translations.
- **A missing message** falls back to English one message at a time. A language ships complete, but for its specification text and `capture`'s text; text added after that shows in English until the translations are filled before the next release, so a change to the English never waits on them.

The cost continues after launch:

- Every new or reworded string also touches `en-US/*.ftl`.
- Maintainers can't review languages they don't read.
- Bug reports will come with screenshots in other languages. That is why logs stay English and `--lang en-US` is there; dmm-gui has no `--lang`, so `LC_ALL=C dmm-gui --help` prints its help in English, unless the Language setting names another language.

## User documentation

The README is translated; the other user docs stay English for now. `scripts/package-docs.py` renders the user docs to HTML with pandoc for the release archives: about 28,000 words in seven files.

**The README comes first.** It is the page a visitor and a search engine meet before anything is installed, about 1,300 words, and it is where a reader learns that the apps speak their language. A copy is `README.<lang>.md` beside the English, `<lang>` being the `i18n/` locale id: the usual convention, found in the root listing, and the English relative links work unchanged. A row under the badges names English and each language that has a copy, by its own name in the order of `SHIPPED`, and every copy carries the same row. A language may ship before its copy exists; the release check asks for one.

**A copy is refreshed at release.** Its first line records the `git hash-object` of the English it was translated from, so `git cat-file blob` gives that English back for a diff against the current one. The English changes on its own; before each release, with the messages, every copy whose hash is stale is brought up to date. A copy as of the last release describes the binary people download. Copies are not in the archives, whose English README links to them on GitHub.

What a copy keeps besides the prose:

- **Its program output.** `docs_snippets.rs` regenerates the `dmm-cli read` sample with the copy's `--lang`, so it shows that language's summary line. The device table is copied with the release refresh, its status words those of the language's `stability-*` messages.
- **Its picture.** The hero screenshot is the one taken in every README language (see [Tests and pipelines](#tests-and-pipelines)). The badges stay as they are: their services don't translate them.
- **Its own anchors.** Nothing outside a copy links into it, so its in-page links follow the anchors GitHub makes from its translated headings.
- **A note that the English is the reference,** a line under the row, since even a reviewed copy lags the English until the release. While the language's `reviewed` is false, it also says the page is machine-translated.

The other docs, when translated, come in this order:

| Doc | When |
|---|---|
| `docs/setup.md` | First: install, drivers and permissions are where a non-English user gets stuck |
| `docs/gui-reference.md` | Then, quoting the UI names from that language's `gui/` files |
| `docs/supported-devices.md` | Optional: mostly model names and tables |
| `docs/cli-reference.md` | With the GUI reference |
| `CHANGELOG.md`, `CONTRIBUTING.md` | Never: they stay English |

The changelog changes every release and feeds What's New and the release notes. Contributions to the project are made in English.

**Parallel Markdown files.** A translated doc is a full copy at `docs/i18n/<lang>/<doc>.md`, with the README's hash line. `package-docs.py` renders it alongside the English, and puts a banner linking the current English on a page whose hash is stale; the hash works in a shallow CI clone and survives a rebase. Links go to the translated page where one exists and to the English page otherwise. Tools that split docs into PO entries paragraph by paragraph, such as po4a and mdpo, track changes more finely. They add a toolchain to the release job, so they become worth it only once Weblate is in use.

What makes these harder than the README:

- **UI names have to match.** The docs quote labels in bold, such as **Alarm**. A translated page uses the words from that language's `gui/` files, and a check can flag a bold name that isn't in them.
- **Generated blocks.** The regeneration tests in `docs_snippets.rs` and `docs_tables.rs` cover the copies, as they do the README's.
- **Screenshots stay English** (see [Tests and pipelines](#tests-and-pipelines)).
- **Anchors.** The apps link to `setup.md`, `supported-devices.md` and anchors in `cli-reference.md` on GitHub. Translated headings produce different anchors, and neither GitHub nor pandoc's GFM reader accepts `{#id}`. So a translated page keeps each English anchor as a raw `<a id="…"></a>` before its heading.

## Ruled out

**On-the-fly machine translation.** Firefox translates pages locally with Bergamot, which runs Marian models, and in Rust `candle` can run Marian models too. Here it would mean a heavy dependency tree and a model download of tens of MB per language. Short UI labels with no context ("Hold", "Range", "REL") are where neural translation does worst. And every string this design translates is fixed or structured, so it can all be drafted offline, with better quality and at no runtime cost.

**Shipping translated capture step instructions.** There are about 800, the largest translation job of all, and they change whenever a family's verification moves on. Only people running a guided capture for an issue read them, and maintainers can't check a translated instruction against the meter. Instead the user exports a meter's instructions, translates them by hand or by machine, and passes the file back in ([capture design](capture-design.md#i-instructions-in-the-users-words)). The file is theirs, so maintainers keep no catalogue in sync. `capture`'s own prompts, which every run shares, are translated like the rest of dmm-cli.

**Right-to-left scripts**, until egui supports them (see [Scripts, fonts and layout](#scripts-fonts-and-layout)).

**Localized number formats**, for the reasons in [What gets translated](#what-gets-translated).

**Translated `set` arguments.** The English tokens have to keep working in scripts. Translated aliases could be added on top later.
