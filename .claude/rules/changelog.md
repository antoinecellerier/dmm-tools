---
paths:
  - "CHANGELOG.md"
---

# Changelog rules (CHANGELOG.md)

## Entry shape

`- **<what the user gets>** — <what they saw before, or the one detail that places it>`

- IMPORTANT: 25 words after the dash is a ceiling, not a target — fewer is better. Past 25, mechanism has crept in; cut it.
- Bold part: present tense, ≤ 12 words, what now works or exists. Not "Fixed X".
- After the dash: no "because", no code mechanism, no how-it-was-fixed, no verification. Those go in the commit body.
- Name a meter, output or shortcut only when the reader needs it to place the change.

Before:
> **"Waiting for meter…" no longer lingers after disconnecting** — the timeout counter behind that message was only cleared by an incoming reading, so if the meter went quiet before you clicked Disconnect the banner stayed up for the rest of the disconnected session.

After:
> **"Waiting for meter…" no longer lingers after disconnecting** — it stayed up if the meter went quiet before you clicked Disconnect.

## Sections

- Order within a version, only when non-empty: `### GUI`, `### CLI`, `### Library`, `### Bug fixes`, `### Documentation`. `### Internal` only for a user-visible symptom of an internal change, led by the symptom.
- Bug fixes are defects: documented or obviously intended behaviour that didn't work. New rendering, prompts, options or output fields go under their component, even when a bug report prompted them.

## One entry per released change

- One entry per user-visible change, not per commit.
- If a later commit in the same Unreleased cycle changes the behaviour again, rewrite the existing entry to the net change since the last release.
- Reread the whole Unreleased section against these rules as part of every release.

## Release headings

- Release: rename `## Unreleased` to `## v<version>`, or `## v<version> — <tagline>` to title the release; `release.yml` matches that line exactly, lifts the tagline into the release title, and fails if the section is missing. A themed release opens with a one-paragraph summary before the first `###`. End the section with `**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v<prev>...v<version>`.
- Dev bump: re-insert an empty `## Unreleased` above it.
