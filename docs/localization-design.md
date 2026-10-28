# Localization Design

> **Status: proposal.** Nothing here is implemented. Every user-facing string is still an English literal where it is used.

This proposal would let dmm-gui and dmm-cli speak the user's language using [Fluent](https://projectfluent.org/). The first request came in [issue #35](https://github.com/antoinecellerier/dmm-tools/issues/35). A reporter whose Brymen meters are sold in Brazil under the Minipa brand ran `capture` through their own wrapper, which rewrote its prompts into Brazilian Portuguese. Because the wrapper piped stdio, the review step and `detect` were skipped. The reporter named the capture flow as the place translation helps most, and offered to contribute and review a pt-BR translation. This proposal leaves the capture tooling in English (see [Ruled out](#ruled-out)), so it serves that audience but not the case that prompted the request.

One principle runs through every section. **Canonical English stays the data.** The library's strings stay what they are and keep serving as keys, file contents and saved settings. Translation happens in the apps, at the moment text reaches the screen or the terminal.

## What gets translated

Translated:

- GUI text: labels, tooltips, toasts, dialogs, accessibility names
- `dmm_shared::help`, which both binaries print
- library labels: modes, ranges, sub-values, flags, links
- user-facing library errors, device names, activation steps
- spec-data notes, guarded (see [Library fixed text](#library-fixed-text))
- optionally, the rest of the CLI's stderr prose and `--help`

Stays English:

- every data format: CSV, JSON, replay, capture YAML, export file names
- `read`'s stdout line, `get --format json`, `set` arguments
- logs, including the library warnings shown by default
- the capture tooling: step instructions, `Expect` reasons, `triage`
- What's New (`CHANGELOG.md`), doc screenshots
- unit symbols, decimal points, timestamps

Estimated size, about 1,250 messages in all:

| Source | Messages |
|---|---|
| GUI | ~580 |
| CLI stderr prose | ~95 |
| `dmm-shared` | ~55 |
| Library labels | ~190 |
| Library errors | ~30 |
| Activation steps | 28 |
| Spec notes | ~280 |

On top of that, the clap `--help` text is about 160 lines. Merging strings that say the same thing in different words would save only a few percent, and writing whole sentences (see [Writing messages](#writing-messages)) adds back about as many. A few of those merges are worth doing anyway as consistency fixes: "Avg" and "Mean" name the same statistic, and both binaries word the limit and date errors separately.

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

A new `l10n` module in `dmm-shared` holds the loader, the language choice and the shared messages, because both binaries must agree on them. Resources go in `i18n/<locale>/{shared,gui,cli,lib}.ftl`, with `en-US` as the reference. The plan is plain `fluent-bundle` behind a small loader. The alternatives are `i18n-embed` with `i18n-embed-fl`, whose `fl!` macro checks message IDs at compile time, and `fluent-templates`. A trial build with `cargo tree` should settle the choice.

**A localizer handle, not a global.** The GUI keeps the localizer in its app state, and the CLI creates one at startup. Switching language replaces the GUI's handle. Each test builds its own handle pinned to en-US or the pseudo-locale, so tests running in parallel don't race on shared state.

**Embedded resources.** The `.ftl` files are compiled into the binaries, so a release archive keeps working wherever it is unpacked.

**No bidi isolation marks.** By default Fluent wraps each placeable in U+2068/U+2069. egui hides them when drawing, but they would still end up in stderr, copied text, screen-reader names and test assertions. `set_use_isolating(false)` turns them off. The right-to-left scripts that need them are out of scope anyway (see [Scripts, fonts and layout](#scripts-fonts-and-layout)).

**Pre-formatted numbers.** A raw `f64` passed to Fluent loses the app's precision: `5.0` prints as "5". Readings and statistics go into messages as already-formatted strings.

## Choosing the language

A `language` field in `SharedSettings` (none means follow the system) is shared by both binaries. dmm-cli gets `--lang`, and the GUI gets a dropdown that lists each language by its own name. The GUI switches immediately, because an immediate-mode UI redraws every label on the next frame anyway. The dropdown ships together with the first real language, not before.

Without a setting, `sys-locale` reports the system language: the UI language on Windows, the preferred-languages list on macOS, and the locale variables on Unix. On Unix it returns `LANGUAGE` first and reduces `C.UTF-8` to "C", so it can't apply gettext's C-locale rule itself.

**C locale means English.** On Unix, the app takes the first non-empty value of `LC_ALL`, `LC_MESSAGES` and `LANG`, as gettext does. If that value is `C` or `POSIX` (including `C.UTF-8`), the app uses en-US and ignores `LANGUAGE`. Only otherwise does it ask `sys-locale`. Scripts, cron jobs and the screenshot pipeline set `LC_ALL=C` and need English, even on Debian and Ubuntu desktops, which set `LANGUAGE`.

## Library fixed text

Each kind of library text is translated by the mechanism that fits it:

- **Enum labels.** `Flag`, `Link` and `Stability` are enums, so their labels translate by variant, through an exhaustive `match`.
- **Strings.** Mode, range and aux labels, device names, activation steps and spec notes are strings. The apps look them up the way gettext does, by their English text plus the kind of text the caller is drawing.

When the apps start, `l10n` reads the en-US `lib.ftl` and builds a map from (kind, English text) to message ID. The kind matters: "Hz" is a mode on one meter and a sub-value on another, and translators may want different words for each. IDs are readable (`mode-capacitance`, `aux-period`). A string with no letters, such as "Ω", gets a numbered ID. Lookup is case-sensitive. A label read from a replay or an import is found the same way as a live one.

The en-US `lib.ftl` is generated:

- A `test-support` enumerator in dmm-lib lists every string with its kind. Device names and activation steps come from the registry, notes from the spec tables, and labels from a new per-family `all_labels()`.
- Each family's `all_labels()` runs its own label builder over every possible input. That includes labels built at run time from a finite set of parts, such as a mode with "Peak" or "REL" appended. Fallbacks like `Unknown(0x..)` stay English.
- A test in dmm-shared regenerates the file on request, the way dmm-cli's `docs_snippets.rs` keeps the docs' CLI examples current.
- The generator only adds entries and reports orphaned ones. It never renames an ID, so the maintainer chooses: correct the English under its existing ID, or give a changed meaning a new ID.
- A coverage test replays the golden fixtures and the mock devices, and asserts that every label they produce is in `lib.ftl`.

**English text is the lookup key.** dmm-lib's strings and stored data don't change. The library gains only `all_labels()` and the enumerator. Translators decide label by label, abbreviations and flags included. One lookup serves the whole reading area, so labels don't come out half-translated. A translator may keep "DC V" and "HOLD" as the meter prints them.

**Spec numbers survive translation.** A translated spec note must contain the same numbers and units as its English source, compared with spacing normalised: "About 10MΩ" may become "Environ 10 MΩ". The check runs when translations load, and a note that fails it is shown in English. The check can't catch a flipped comparison or a lost "typical", so the specifications panel also shows the English note on hover.

## Library run-time text

Some library errors build their English message at run time with `format!`, so their text cannot be looked up. They split by who reads them:

| Group | Errors | Treatment |
|---|---|---|
| Developer diagnostics | invalid response, checksum mismatch, unknown mode byte, replay parse, HID | Stay English |
| Already structured | `Timeout`, `LinkLost`, `AdapterNotFound`, `UnknownDevice`, `DeviceNotIdentified`, `WrongCable`, `NoTransportFound`, `BluetoothOnly` | Translated by variant and fields |
| Built from sentences | `UnsupportedCommand`, `CommandRejected`, `Bluetooth`, about 30 messages, many in `protocol/cycle.rs` | Gain a reason enum |

The reason enum's details:

- `Bluetooth` keeps a variant that passes the Bluetooth stack's own error text through unchanged.
- Button names (RANGE, SELECT) inside a reason stay as the meter prints them.
- Mode labels inside a reason go through the lookup in [Library fixed text](#library-fixed-text).

The library's `Display` stays the English rendering, used by logs and by library users without Fluent. `dmm-shared` renders each reason through Fluent. One test renders sample reasons through en-US and checks that the result matches `Display`.

**Structure, not sentences.** The renderer is an exhaustive `match`, so a reason added without a message fails to compile. Matching on error text would fail silently instead.

Alarm toasts render from `Breach`'s fields, which needs no library change. Alarm marker notes stay English: they go into the export's `note` column alongside the user's own notes.

## Writing messages

- **Whole sentences.** A message never joins fragments. Several things that are assembled today become single messages using Fluent selectors and CLDR plural rules: the "{n} unexported {samples} and unsaved changes to {markers}" prompt, its `if n == 1` plural nouns, and the screen reader's live-region summary.
- **Shortcuts as placeables.** Tooltips get their shortcut text from `format_shortcut` instead of a hard-coded "(Ctrl+O)". On macOS this also shows Cmd. The change lands on its own, before any extraction.
- **egui's own words** stay English, including the modifier names `format_shortcut` returns.
- **Easy to miss:** the file-type names in rfd's save and open dialogs, accessibility names set through `WidgetInfo`, and the `Comment` line in `assets/dmm-tools.desktop`.

## Scripts, fonts and layout

| Script | Coverage |
|---|---|
| Latin, Cyrillic | Bundled fonts (Ubuntu-Light, with Hack as fallback) |
| Greek | Partial in Ubuntu-Light; check before offering a Greek translation |
| Chinese, Japanese, Korean | Only through eframe's system-font fallback, so it depends on the user's installed fonts |
| Arabic, Hebrew, Persian | Not possible: egui 0.37 has no bidirectional text or right-to-left layout |

Most of the layout measures its text, but longer strings will still find some trouble spots:

- fixed widths on toasts, the shortcut-help columns and the specs columns;
- truncated labels in the device and marker lists;
- the top bar, whose width sets the window's minimum width;
- the big meter, whose fit shrinks the reading when the notice title above it gets longer.

**A padding pseudo-locale finds layout bugs.** `fluent-pseudo` lengthens text by doubling lowercase vowels, which leaves capitals like HOLD, REL and AUTO, and the reading area full of them, at their original width. Instead, a custom `set_transform` adds accents and pads every string by about 40%. A developer-only environment variable switches it on. verify-gui can set that variable, and the pseudo-locale never appears in the dropdown.

## Tests and pipelines

Many tests check English text or find a widget by its label. Each builds a localizer pinned to en-US, so a developer with a French desktop gets the same results. `scripts/doc-screenshots.sh` and verify-gui set `LC_ALL=C`, which the C-locale rule turns into English. Doc screenshots stay English-only because their click positions assume English widths.

The resources get three checks:

- every `.ftl` file parses;
- no translation defines a message ID that en-US lacks;
- the label coverage test from [Library fixed text](#library-fixed-text).

## Translations

- **Contributions** start as pull requests against the `.ftl` files, which are readable plain text. A `CONTRIBUTING.md` section covers how to add or update a language.
- **Weblate** can come later, once translators who aren't developers turn up. It has supported Fluent since 4.8 and opens pull requests for review. It has no plural editor for Fluent, so translators edit select expressions as text.
- **Machine-drafted translations** are made offline with an LLM or OPUS-MT. They draw on a glossary of meter terms and the wording rules the English follows. A comment at the top of a drafted file marks it unreviewed until a native speaker has read it.
- **A message ID is renamed** when the meaning of the English changes, so stale translations fall back to English rather than saying the wrong thing. A correction that keeps the meaning keeps its ID and its translations.
- **A missing message** falls back to English one message at a time. The dropdown can wait until a language reaches a completeness threshold.

The cost continues after launch:

- Every new or reworded string also touches `en-US/*.ftl`.
- Maintainers can't review languages they don't read.
- Bug reports will come with screenshots in other languages. That is why logs stay English and `--lang en` is there.

## User documentation

`scripts/package-docs.py` renders the user docs to HTML with pandoc for the release archives. That is about 28,000 words in seven files. The order of translation is:

| Doc | When |
|---|---|
| `docs/setup.md` | First: install, drivers and permissions are where a non-English user gets stuck |
| `README.md` | Next, linking the translated pages |
| `docs/gui-reference.md` | Once the GUI is translated, so the UI names it quotes match |
| `docs/supported-devices.md` | Optional: mostly model names and tables |
| `docs/cli-reference.md` | Only if the CLI is translated |
| `CHANGELOG.md`, `CONTRIBUTING.md` | Never: they stay English |

The changelog changes every release and feeds What's New and the release notes. Contributions to the project are made in English.

**Parallel Markdown files.** A translation is a full copy at `docs/i18n/<lang>/<doc>.md`. `package-docs.py` renders it alongside the English. Links go to the translated page where one exists and to the English page otherwise. Tools that split docs into PO entries paragraph by paragraph, such as po4a and mdpo, track changes more finely. They add a toolchain to the release job, so they become worth it only once Weblate is in use.

**Translations say how current they are.** A comment on a translated file's first line records the hash of the English file it was translated from (`git hash-object`). When the English file's hash differs, `package-docs.py` puts a banner on the rendered page linking the current English. This works in a shallow CI clone and survives a rebase. A translation never has to change in the same commit as the English.

What makes doc translation harder than message translation:

- **UI names have to match.** The docs quote labels in bold, such as **Alarm**. A translated page uses the words from that language's `gui.ftl`, and a check can flag a bold name that isn't in that file.
- **Generated blocks stay English.** The copies keep the blocks that `docs_snippets.rs` and `docs_tables.rs` regenerate verbatim. Either the regeneration test covers the copies, or it leaves them out.
- **Screenshots stay English** (see [Tests and pipelines](#tests-and-pipelines)).
- **Anchors.** The apps link to `setup.md`, `supported-devices.md` and anchors in `cli-reference.md` on GitHub. Translated headings produce different anchors, and neither GitHub nor pandoc's GFM reader accepts `{#id}`. So a translated page keeps each English anchor as a raw `<a id="…"></a>` before its heading.

## Ruled out

**On-the-fly machine translation.** Firefox translates pages locally with Bergamot, which runs Marian models, and in Rust `candle` can run Marian models too. Here it would mean a heavy dependency tree and a model download of tens of MB per language. Short UI labels with no context ("Hold", "Range", "REL") are where neural translation does worst. And every string this design translates is fixed or structured, so it can all be drafted offline, with better quality and at no runtime cost.

**Translating the capture tooling.** That means about 800 step instructions, the `Expect` reasons and `triage`. It is the largest translation job of all. The step texts also change whenever a family's verification moves on. Capture serves only people running a guided capture for an issue, and maintainers can't check a translated instruction against the meter. Step IDs aren't the obstacle: they are unique per device, so device-scoped message IDs would work.

#35's need can be met far more cheaply, and without a wrapper. `capture` would export a device's step instructions to a file keyed by step ID. The user translates that file by hand or by machine and passes it back in. The run is unchanged except for the printed text: gate steps, expectations, review and `detect` all behave as before, and the report keeps the English. The file belongs to the user, so maintainers keep no catalogue in sync. It doesn't depend on any part of this proposal.

**Right-to-left scripts**, until egui supports them (see [Scripts, fonts and layout](#scripts-fonts-and-layout)).

**Localized number formats**, for the reasons in [What gets translated](#what-gets-translated).

**Translated `set` arguments.** The English tokens have to keep working in scripts. Translated aliases could be added on top later.

## Rollout

1. **Spike:**
   - trial-build the three crate options;
   - render one translated panel, one CJK string and the pseudo-locale under verify-gui;
   - check what a screen reader does with non-English names, since AccessKit receives no language tag.
2. Shortcut placeables, which also bring the macOS Cmd fix.
3. The `l10n` infrastructure, the localizer handle and the en-US files. Tests are pinned to en-US, and nothing visible changes.
4. GUI extraction, one panel per commit, with pseudo-locale layout passes.
5. [Library fixed text](#library-fixed-text), then [Library run-time text](#library-run-time-text).
6. The language setting and the first reviewed language. This step also translates `dmm_shared::help`, so the CLI's help and error hints change language too.
7. Optional, as demand appears: the rest of the CLI's prose and `--help`, machine-drafted languages, and Weblate.
8. Translated user docs, starting with `docs/setup.md` (see [User documentation](#user-documentation)).

## Open questions

- Is one request, #35, enough to justify the complexity now?
- Should the capture tooling come back in scope, given that #35's reporter offered to translate and review it? Or is a user-supplied instruction file enough?
- Beyond `dmm_shared::help`, should the CLI's own prose wait? clap has no translation hook: translated help means rebuilding it at run time, and clap's own errors stay English.
- Is Brazilian Portuguese the first language, with #35's reporter as its reviewer?
- Should contributions go through pull requests only, or through Weblate too?
- Should machine-drafted languages ship marked unreviewed, or wait for a native speaker?
- Should library text, and the spec notes in particular, arrive with the first language or later?
- Should the user docs be translated at all, and is `setup.md` enough to start with?
