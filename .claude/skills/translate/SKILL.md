---
name: translate
description: >-
  Adds a language to dmm-gui and dmm-cli, or fills the translations the
  shipped languages lack: approved sources only, a glossary and style sheet
  per language with the user's rulings, a drafting workflow reviewed on
  screenshots and CLI output, the checks, the commit split and the
  machine-translated mark. Use when adding or reviewing a language, at the
  release step that fills translations, or when
  `shipped_languages_are_complete` lists gaps.
---

# Translations

Run this in the main session; don't hand it to an agent: its workflows and checkpoints need the session the user talks to.

The ground rules are in the docs; read the parts you need rather than restating them:
- `docs/localization-design.md`: what is translated and what stays English.
- `docs/development.md`, "Translations": the checks, `DMM_TOOLS_L10N_DIR` and `DMM_TOOLS_PSEUDO_LOCALE`.
- `CONTRIBUTING.md`, "Translations": what a language needs to ship, and the `reviewed` flag.
- `.claude/rules/readme.md`, "Translated copies": what a `README.<lang>.md` keeps.

Archive everything worth keeping under `references/l10n/<lang>/` (gitignored): sources, glossary drafts, workflow results, review screenshots. The scratchpad does not survive a reboot.

## Sources

Clean-room rule. Use only these:
- multimeter manuals in the language, from meter makers;
- Microsoft's localization style guide and Terminology for the language;
- Apple's and Microsoft's public support pages, for OS setting names;
- GitHub Docs in the language (docs.github.com/<lang>/), for GitHub's own terms such as issue and release.

Never read other apps' translations, vendor app string files, community translations or machine-translation output. Save each download under `references/l10n/<lang>/`, and list its URL and title in `SOURCE.txt` there. For any other source, ask the user first.

## A new language

```
Language: <lang>
- [ ] 1. Glossary, style sheet and SUMMARY.md drafted from approved sources
- [ ] 2. Rulings settled with the user; glossary committed
- [ ] 3. Workflow run with the user's go-ahead; result archived; native review run
- [ ] 4. Three translation commits; the last one ships the language
- [ ] 5. Checks, screenshots and per-commit check pass
- [ ] 6. README translation, its picture and the language row
```

1. **Glossary.** First choose the locale id: the bare language (`fr`, `es`) when one text can serve all its regions, a region (`pt-BR`, `zh-CN`) when the regions write differently. Then spawn one agent with `scripts/glossary-brief.md`, filling in `<LANG>`, `<NAME>`, `<REPO>`, and `<MANUAL_HINTS>` (the meter brands that print manuals in that language). It writes to `references/l10n/<lang>/glossary-draft/`:
   - `glossary.tsv`;
   - `STYLE.md`, with its §0 Rulings first.

   Its report ends with `SUMMARY.md`, the questions to rule on first; save it there yourself, as a subagent can't write that file.
2. **Rulings.** Show the user `SUMMARY.md`.
   - For a language the user reads, the user rules on each question. Put the rulings in STYLE.md §0, and fix the glossary rows they change.
   - For any other language, the drafter's choices stand as provisional, and `SUMMARY.md` stays with them for a future reviewer.

   Then copy the three files into `i18n/<lang>/` and commit them as `docs: add the <Name> glossary and style sheet`.
3. **Workflow.** It runs about nine agents, so get the user's go-ahead first. Ask too whether the drafters and fixer may use a lighter model (`writeModel: "sonnet"`); the reviewers keep the session's model. Then run:

   `Workflow({scriptPath: ".claude/skills/translate/scripts/translate-language.js", args: {repo: "<absolute repo path>", lang: "<lang>", name: "<English name>", writeModel?}})`

   - **What it does:** six drafters, one per file group; a GUI reviewer working on verify-gui screenshots; a text reviewer for CLI output and the library's text; then a fixer.
   - **What it writes:** only `i18n/<lang>/`. It never commits.
   - **While it runs,** make no Rust edits and no commits in the checkout. Its agents build there, and the commit hook would test half-written files.
   - **Real meters:** the GUI reviewer picks only simulated meters, unless `dmm-cli list` shows nothing attached.

   Then run the [native review](#native-review) over the language, and save both results to `references/l10n/<lang>/workflow/`. Read the reviewers' notes:
   - Fold the glossary gaps and style fixes the drafters, reviewers and fixer report into the glossary commit.
   - Anything that needs code rather than translation becomes a fix of its own, or an entry under "Localization follow-ups" in `docs/future-improvements.md`. First check it against "Stays English" in `docs/localization-design.md`: reviewers also flag text that is English by design, such as logs and `set` tokens.
4. **Commits.** Three, in this order. Each adds `ftl!` entries in the same order the other languages use. Each commit's hook run must see only that commit's files under `i18n/<lang>/`: a translation with no `ftl!` line fails `messages_match_their_uses`. So keep the full result under `references/l10n/<lang>/workflow/`, and copy each group back into `i18n/<lang>/` just before its commit.
   1. `shared: translate the help and the library's text into <lang>`: `shared.ftl` and the four `lib*.ftl` files, listed in `help.rs` `RESOURCES` and `lib_index.rs` `LIB_RESOURCES`.
   2. `gui: translate the GUI and its desktop entry into <lang>`: `gui/*.ftl` and `gui-help.ftl`, listed in `crates/dmm-gui/src/l10n.rs` (GUI and GUI_HELP).
   3. `shared, cli, gui: translate dmm-cli into <lang> and offer <lang>`: `cli/*.ftl` and `cli-help.ftl`, listed in `crates/dmm-cli/src/l10n.rs` (CLI and CLI_HELP). This commit also:
      - adds the language to `SHIPPED` in `crates/dmm-shared/src/l10n/lang.rs`, with its native name and `reviewed: false`, in the place its doc comment's order gives it;
      - updates the possible-values tests in `crates/dmm-cli/src/cli.rs` and `crates/dmm-cli/tests/help.rs`;
      - rewrites the CHANGELOG's GUI and CLI translation entries, the README's features line and the first line of `docs/localization-design.md` to name the new language. The GUI entry says which languages are machine-translated.
5. **Checks.**
   - `cargo test -p dmm-shared translations`, and `shipped_languages_are_complete -- --ignored`.
   - `long_labels -- --ignored --nocapture`: look in the window at what it lists, starting with the tight places: the top bar, the alarm and scale fields, the stats row and the shortcut list.
   - `dmm-cli --lang <lang> --help`, and `--device mock read --count 3`.
   - A verify-gui shot of the Language row, showing the native name, and of the top bar at 700 px wide. Compare each shot with en-US at the same width and state: paused, the top bar wraps at 700 px in English too.
   - A history rewrite is followed by clippy and tests on every rewritten commit (`git rebase -x` in a worktree).
6. **README.** Recommended, in a `docs:` commit of its own: draft `README.<lang>.md` from the English README and the glossary, following `.claude/rules/readme.md`; add the language to the row in `README.md` and in every copy; add its marker notes and a `gui-wide-layout.<lang>` row to `scripts/doc-screenshots.sh` and run that scene; fill the copy's `read` sample with `UPDATE_DOCS=1 cargo test -p dmm-cli docs_snippets`. Run the [native review](#native-review) with `target: "readme"`, then check its in-page links with `cargo test -p dmm-cli --test readme_languages`. Show the user the copy if they read the language.

## Native review

A draft passes a single review before it ships; this pass finds what that one misses. It runs after a new language's workflow, on a README copy, and on any shipped language the user asks to polish:

`Workflow({scriptPath: ".claude/skills/translate/scripts/native-review.js", args: {repo: "<absolute repo path>", langs: [{lang: "es", name: "Spanish"}, …], target?: "app" | "readme", accuracy?: false, readLangs?: ["fr"], writeModel?}})`

- **Three readers per file group.** A Sonnet and an Opus reviewer check accuracy independently: meaning, glossary terms, leftover English, structure. They catch different things. An Opus reviewer reads for native feel only: calques, word order, prepositions and particles, noun chains, a translated-sounding register. `accuracy: false` runs only the native reader, for a language whose accuracy was just reviewed.
- **Platform terms come from the platform.** A word GitHub, Windows or macOS shows in its own interface takes that platform's word in the language, from its own docs ([Sources](#sources)).
- **One fixer per language** applies the findings within the house rules, settles disagreements by the style sheet and the English, and reruns the checks.
- **The user rules on what they read.** For a language in `readLangs`, the findings come back unapplied: show them to the user as a table of current text, proposed text and your take, and apply what they approve. A ruling on a term goes into the language's glossary and its STYLE.md §0 in the same change.

Ask the user's go-ahead first: for the apps it runs twelve reviewers per language (four with `accuracy: false`), plus a fixer.

## Reviewed

`scripts/l10n-side-by-side.py <lang> <lang>.html` sets a language beside its English on one page, for a reviewer, or for the user when shown new messages, to read through. Once a native speaker has read the language in the app, set `reviewed: true` on its `SHIPPED` entry, and cut the note under its `README.<lang>.md` row down to the line naming the English README as the reference. For a language the user reads, the user's review counts. In the same change, update the CHANGELOG:
- if the entry that says the language is machine-translated is still under `## Unreleased`, drop the language from its "machine-translated" clause;
- if it has been released, add an entry saying the translation has been reviewed.

## Filling gaps

New or reworded English ships without its translations: it shows in English until they are filled in, at the latest by the release step in `docs/development.md`.

1. `cargo test -p dmm-shared shipped_languages_are_complete -- --ignored` lists each language's missing messages, the optional ones (the specification text, `cli/capture.ftl`) aside; `shipped_languages_have_the_optional_text` lists those too.
2. For a handful, translate them inline, following each language's `i18n/<lang>/STYLE.md` and `glossary.tsv`, and read each back for native feel as the [native review](#native-review) does. A missing file is copied from en-US, translated, and given an `ftl!` line per language next to the en-US one; until it has that line, `cargo test` names it. For more, run:

   `Workflow({scriptPath: ".claude/skills/translate/scripts/fill-gaps.js", args: {repo: "<absolute repo path>", langs: [{lang: "fr", name: "French"}, …], writeModel?, optional?}})`

   Per language, it runs a drafter, a reviewer of the added messages, and a fixer. `optional: true` drafts the optional messages too.
3. Show the user the new messages in any language they read before committing.
4. Commit each language's additions with the component prefixes of the files touched.
5. At the release step, bring the README's translations up to date too: `docs/development.md`, release step 5.

## Rewording English later

- **A message whose meaning changes gets a new ID,** so stale translations fall back to English instead of saying the wrong thing. A rewording that keeps the meaning keeps its ID and its translations.
- **`cli-help.ftl` and `gui-help.ftl` are generated.** Regenerating one lists the IDs whose English changed (`docs/development.md`, "Translations"). Where the meaning changed, delete those messages from the other languages.

## Scripts

Run these through the Workflow tool; don't read them into context:
- `scripts/translate-language.js`: one new language, from draft to fix.
- `scripts/fill-gaps.js`: the missing messages of the shipped languages.
- `scripts/native-review.js`: accuracy and native-feel review of shipped languages or README copies, then a fixer per language.

`scripts/glossary-brief.md` is the glossary agent's brief, with placeholders to fill in.
