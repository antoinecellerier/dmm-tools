---
paths:
  - "CHANGELOG.md"
---

# Changelog rules (CHANGELOG.md)

## Entry shape

`- **<what the user gets>**` — optionally followed by ` — <what they saw before, or the one detail that places it>`

- IMPORTANT: 25 words after the dash is a ceiling, not a target — fewer is better. Past 25, mechanism has crept in; cut it. The one exemption is a migration step, below.
- Bold part: present tense, ≤ 12 words, what now works or exists. Not "Fixed X".
- After the dash, only when the bold doesn't already imply it: one before-state, or the one detail that places the change. "**X works** — X didn't work" is the bold again in past tense; end at the bold.
- Not after the dash: "because", code mechanism, how it was fixed, verification, a tour of the new look, when something appears or where on screen it sits, or numbers the reader can't act on (contrast ratios, point sizes, code points). Mechanism goes in the commit body.
- Placing the change is naming the meter, output or shortcut it concerns, and only when the reader needs it.
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

- Order within a version, only when non-empty: `### Devices`, `### GUI`, `### CLI`, `### Bug fixes`, `### Documentation`. `### Internal` only for a user-visible symptom of an internal change, led by the symptom.
- `### Devices`: new models, verification status changes, cables. It comes first because whether the reader's meter works is the first question.
- A feature in both GUI and CLI gets an entry in each — a CLI reader doesn't read the GUI section.
- Bug fixes are defects: documented or obviously intended behaviour that didn't work. New rendering, prompts, options or output fields go under their component, even when a bug report prompted them.
- `### Documentation` is rare: only a change to the docs as a whole that a reader would remark on — they ship as web pages, the screenshots are always current. A new section, an added detail or a rewrite gets no entry.

## Order within a section

Recency is not an order. A new entry goes below the last entry of its tier, never at the top.

- GUI and CLI, by tier, then breadth (every meter before one family), then how often the reader meets it:
  1. Works at all: connecting, detection, a meter or cable now usable.
  2. New capability: something the reader could not do before.
  3. More of an existing capability that changes the experience: a buffer going from minutes to hours, sub-values on screen, new columns, modes, shortcuts.
  4. Polish: appearance, contrast, wording, help text, focus.
- Bug fixes, by what the defect cost: nothing worked, then wrong readings or exported data, then wrong labels, then controls that did nothing or the wrong thing, then cosmetic. A migration entry takes its place by the same ladder.
- At release, the summary under the heading names the first entry or two of each section; if it names something further down, reorder.

## One entry per released change

- One entry per user-visible change, not per commit. Sibling commits to one surface in one cycle are one entry: several panels gaining scrolling, several new shortcuts.
- If a later commit in the same Unreleased cycle changes the behaviour again, rewrite the existing entry to the net change since the last release.
- Before writing an entry for a fix, check the thing fixed shipped in the last release (`git grep` at the tag). If it arrived this cycle, fold it into that feature's entry or add nothing — no release showed the defect.
- An entry says a model works or is verified only once someone has confirmed it on the meter. A targeted fix gets its entry when the evidence that it works is credible (a vendor document, a reporter's trace), confirmed or not.
- Reread the whole Unreleased section against these rules as part of every release.

## Release heading, tagline and summary

- Release: rename `## Unreleased` to `## v<version>` or `## v<version> — <tagline>`; `release.yml` matches that line exactly, lifts the tagline into the release title, and fails if the section is missing. End the section with `**Full Changelog**: https://github.com/antoinecellerier/dmm-tools/compare/v<prev>...v<version>`.
- Tagline (≤ 8 words): what the release changes in scope or intent, stated plainly — `Multi-Device Protocol Support` — not a feature list and not a slogan.
- Summary: one or two short sentences (2–3 lines in GitHub's release view) directly under the heading, naming the intent and the main areas touched; the sections below carry the detail. Exempt from the 25-word rule.
- Omit both when a release has no theme.
- Dev bump: re-insert an empty `## Unreleased` above it.
