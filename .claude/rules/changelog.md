---
paths:
  - "CHANGELOG.md"
---

# Changelog rules (CHANGELOG.md)

## Entry shape

`- **<what the user gets>** — <what they saw before, or the one detail that places it>`

- IMPORTANT: 25 words after the dash is a ceiling, not a target — fewer is better. Past 25, mechanism has crept in; cut it. The one exemption is a migration step, below.
- Bold part: present tense, ≤ 12 words, what now works or exists. Not "Fixed X".
- After the dash: no "because", no code mechanism, no how-it-was-fixed, no verification. Those go in the commit body.
- Name a meter, output or shortcut only when the reader needs it to place the change.
- Backtick any literal containing `@`. The section is lifted verbatim into the GitHub release body, where a bare `@name` renders as a mention of a real, uninvolved account — `"@12s"` (a MIN/MAX timestamp) linked a stranger on a published release. Deliberate credit is the exception: link it, as `[@user](https://github.com/user)`.

Before:
> **"Waiting for meter…" no longer lingers after disconnecting** — the timeout counter behind that message was only cleared by an incoming reading, so if the meter went quiet before you clicked Disconnect the banner stayed up for the rest of the disconnected session.

After:
> **"Waiting for meter…" no longer lingers after disconnecting** — it stayed up if the meter went quiet before you clicked Disconnect.

## Migration steps

A change the user must act on by hand — edit or delete a system file, run a command — may append one migration instruction. Test: *if they upgrade and do nothing, are they worse off?* If no, use the standard shape.

- Imperative, ≤ 2 sentences, ≤ 35 words, exempt from the 25-word ceiling. May replace the before-state clause when the bold already carries it.
- Say only what to do. This permits an instruction; it does not relax the ban on "because", mechanism or verification.
- More than two sentences, or per-platform branches: put it in the doc that covers it and point there.

> **The udev rule works on Fedora and other distributions without `plugdev`** — install `70-dmm-tools.rules` and replug the cable; delete `/etc/udev/rules.d/99-dmm-tools.rules` if you installed a previous release. On a headless machine, keep a group on the rule — see `docs/setup.md`.

## Sections

- Order within a version, only when non-empty: `### GUI`, `### CLI`, `### Library`, `### Bug fixes`, `### Documentation`. `### Internal` only for a user-visible symptom of an internal change, led by the symptom.
- Bug fixes are defects: documented or obviously intended behaviour that didn't work. New rendering, prompts, options or output fields go under their component, even when a bug report prompted them.

## One entry per released change

- One entry per user-visible change, not per commit.
- If a later commit in the same Unreleased cycle changes the behaviour again, rewrite the existing entry to the net change since the last release.
- Reread the whole Unreleased section against these rules as part of every release.

## Release heading, tagline and summary

- Release: rename `## Unreleased` to `## v<version>` or `## v<version> — <tagline>`; `release.yml` matches that line exactly, lifts the tagline into the release title, and fails if the section is missing. End the section with `**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v<prev>...v<version>`.
- Tagline (≤ 8 words): what the release changes in scope or intent, stated plainly — `Multi-Device Protocol Support` — not a feature list and not a slogan.
- Summary: one or two short sentences (2–3 lines in GitHub's release view) directly under the heading, naming the intent and the main areas touched; the sections below carry the detail. Exempt from the 25-word rule.
- Omit both when a release has no theme.
- Dev bump: re-insert an empty `## Unreleased` above it.
