# Contributing

## Feedback

Bugs, ideas, questions, or something in the app or the docs that confused you: [open an issue](https://github.com/antoinecellerier/dmm-tools/issues/new). Results for a meter that has a verification issue (linked from [supported devices](docs/supported-devices.md)) go there.

## Bug reports

When filing a bug report, please include:

1. What you expected vs. what happened
2. Your meter model (printed on the meter, e.g. "UT61E+") and cable
3. Your OS and the output of `dmm-cli --version`
4. For a wrong or missing reading, a protocol capture (see below) — this is the single most useful thing you can attach
5. For a graph or timing problem, a replay file: the arrow beside **Export…**, then **Replay…**, in the [GUI](docs/gui-reference.md#recording), or [`dmm-cli read --format replay`](docs/cli-reference.md#dmm-cli-read)

## Protocol captures

The protocols were reverse-engineered, not documented by the vendors. Captures from real meters are how bugs get found and new models confirmed.

### Running a capture

The built-in capture wizard walks you through each measurement mode and button, records the raw bytes, and asks you to confirm what the meter's screen shows:

```sh
dmm-cli capture

# Name the meter if detection fails or you want to force a model
dmm-cli --device ut8803 capture
```

It opens with what the run needs on the bench — shorted leads, a DC source, a thermocouple — and skips the steps for anything you don't have. Each step captures on its own once the meter is in the mode asked for; Enter captures now, `s` skips, `q` finishes and saves. Last, it asks you to restart the meter and checks that auto-detection finds it. The report is a YAML file such as `capture-ut61eplus.yaml`. The [CLI reference](docs/cli-reference.md#dmm-cli-capture) describes the options.

If you're short on time, run only the steps no one has confirmed on hardware yet:

```sh
# List available steps, marked ✓ (confirmed) or ·
dmm-cli capture --list-steps

# Run only the unconfirmed ones, plus the freeform pass
dmm-cli capture --unverified

# Or pick steps yourself
dmm-cli capture --steps dcmv,temp,duty
```

A full `dmm-cli capture` stays the thorough option: it re-checks the confirmed steps too.

If a maintainer posts a short plan file in your issue, run it with `dmm-cli capture --plan edge.yaml` and attach the report it writes.

The steps' instructions are in English. To read them in your language, run `dmm-cli capture --export-instructions steps.ftl`, translate the text after each `=` (by hand or with a translation tool), then run `dmm-cli capture --instructions steps.ftl`.

Captures save after each step, so you can interrupt and resume later.

### What to do with the capture

Attach the YAML file to your GitHub issue. For a meter that is not yet verified, attach it to the meter's verification issue, linked from [supported devices](docs/supported-devices.md) — those captures are especially valuable.

## Unconfirmed platform testing

macOS ARM (Apple Silicon), Linux ARM (Raspberry Pi 3B+), and Windows ARM have been confirmed working. The following platforms have builds but haven't been verified yet:

- **macOS Intel** — [issue #2](https://github.com/antoinecellerier/dmm-tools/issues/2)

If you have one of these platforms and a supported meter:

1. Download the appropriate build from the [latest release](https://github.com/antoinecellerier/dmm-tools/releases/latest) — or, to test a fix that has not been released yet, a [dev build](docs/setup.md#dev-builds) — or build from source: `cargo build --workspace`
2. Plug in the USB cable and run `dmm-cli list`
3. If the cable is found, try `dmm-cli read` and `dmm-gui`
4. Comment on the relevant issue with your results — include your OS version, cable, meter model, and whether readings were correct

Even "it works" is valuable. If something doesn't work, the output of `RUST_LOG=dmm_lib=trace dmm-cli read` will help us debug.

## Translations

The apps' words are in [Fluent](https://projectfluent.org/fluent/guide/) files under `i18n/<locale>/`, `en-US` being the reference: one file per screen area, each message under an ID, with comments saying where it shows and what each `{ $variable }` holds.

Most languages start machine-translated, and stay marked so in the **Language** list until a native speaker has read them: a review is the most useful help.

### Reviewing a translation

1. From a clone of this repository, write the language beside its English as one page, then open it in a browser. Each message sits under its English and its notes, the glossary comes last, and buttons show only what is missing or identical to the English.

   ```sh
   python3 scripts/l10n-side-by-side.py pt-BR pt-BR.html
   ```

   On Windows: `python scripts\l10n-side-by-side.py pt-BR pt-BR.html`.
2. Read it in the apps too, on a simulated meter, where length and context show. dmm-gui follows the system's language; otherwise pick yours in Settings, **Language**.

   ```sh
   dmm-gui --device mock
   dmm-cli --lang pt-BR --help
   dmm-cli --lang pt-BR --device mock read --count 5
   ```
3. Look for:
   - a text that says something other than its English, above all about a measurement or what to do with the meter;
   - meter vocabulary that strays from the glossary or from what meter manuals in your language say;
   - what must stay as written that didn't: the meter's own key and display labels (HOLD, REL, AC/DC as the meter prints them), unit symbols, numbers with their decimal point (1.5, never 1,5), `--options`, command lines, `{ $variables }`;
   - "same as English" on a sentence rather than a label;
   - text cut off or wrapping badly in a small window;
   - a tone or form of address foreign to software in your language. `i18n/<locale>/STYLE.md` records the choices made, and `SUMMARY.md`, where there is one, those most worth a second opinion.
4. Fix what reads wrong, [by hand](#manually-translating), and open a pull request, or list it in an issue. Once the whole language has been read, set `reviewed: true` on its entry in `crates/dmm-shared/src/l10n/lang.rs` in the same pull request.

### Translating

A language ships once all its files are translated, but for the specification text in `lib.ftl` (the `table-`, `note-`, `input-` and `band-` messages) and `capture`'s text in `cli/capture.ftl`: these show in English until someone translates them. Text a code change adds or rewords needs only its English: it shows in English until the translations are filled in before the next release. A language should also have a translation of the README, `README.<lang>.md`, which the language row at the top of the README links to.

#### Using a coding agent

To start a whole new language, open [Claude Code](https://claude.com/claude-code) in a clone of this repository and ask `/translate` for it. The skill (`.claude/skills/translate/`) runs the process the existing languages went through: a glossary and style sheet from approved sources, drafting, review on screenshots and CLI output, and the checks.

#### Manually translating

1. Copy a file from `i18n/en-US/` to the same path under your locale, such as `i18n/fr/gui/settings.ftl`.
2. Translate the text after each `=`. Keep the IDs, the `{ $variables }` and the `[selector]` keys as they are; numbers, unit symbols and command lines stay as written. A message you leave out shows in English.

   In VS Code, `code --diff i18n/en-US/gui/settings.ftl i18n/fr/gui/settings.ftl` shows the English beside your file, message for message, with your side editable; the [vscode-fluent](https://marketplace.visualstudio.com/items?itemName=macabeus.vscode-fluent) extension (`macabeus.vscode-fluent`) highlights the syntax and flags errors.
3. Try it in a release build, without compiling: point `DMM_TOOLS_L10N_DIR` at the folder holding your locale folder, then pick your language in Settings, **Language**, or with dmm-cli's `--lang`.

   ```sh
   DMM_TOOLS_L10N_DIR=~/dmm-tools/i18n dmm-gui
   DMM_TOOLS_L10N_DIR=~/dmm-tools/i18n dmm-cli --lang fr --help
   ```

   A folder for a language the apps don't ship yet makes it selectable.
4. Open a pull request with the files.

`DMM_TOOLS_PSEUDO_LOCALE=1` shows every message accented and padded, which finds the places where a longer language won't fit; the [development guide](docs/development.md#translations) has the details.

## Code changes

1. Fork and create a feature branch
2. Make sure `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass (the pre-commit hook runs all three — see [development guide](docs/development.md))
3. Include tests for new functionality
4. For protocol changes: verify against a real device (`RUST_LOG=dmm_lib=trace dmm-cli debug`)
5. Open a pull request with a description of what and why
