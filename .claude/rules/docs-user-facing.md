---
paths:
  - "docs/cli-reference.md"
  - "docs/gui-reference.md"
  - "docs/supported-devices.md"
  - "docs/setup.md"
  - "CONTRIBUTING.md"
  - "README.md"
---

# User-facing doc rules

A user reads these docs once, top to bottom, to learn what the tool does and
how to drive it. They are not a design note, a changelog or a verification
report. Write the fewest sentences that answer the reader's question, then
stop — a short section is never the defect.

These files ship in the release archives, as Markdown and HTML, through `DOCS`
in `scripts/package-docs.py`. A new user doc goes both in `paths:` above and
in that list.

## Not in a reference

- Rationale: "because", "so that", "since", "would otherwise", rejected
  alternatives. It goes to `docs/ux-design.md`, `docs/*-design.md` or a code
  comment.
- Report or file schema keys, internal state names, tiers, timers. They go to
  the design doc the section links to.
- Verification history: reporter counts, cables tried, which modes are
  confirmed, what remains. It goes to `docs/supported-devices.md` and
  `docs/verification-backlog.md`; the reference says "experimental" and links
  to supported devices.
- Changelog voice: "now", "still", "no longer", "unlike before". The reference
  describes the current release only.
- Numbers the reader cannot act on: millisecond timings, pixel thresholds,
  memory estimates the UI shows on hover.
- Every branch of a state machine. Name the rule and the keys; the edge cases
  a user meets with a message on screen stay out.

## One owner per fact

- A fact lives in the section that owns the surface: CSV columns in CLI
  `read`, Scale semantics in GUI `Scale`, device status in
  `docs/supported-devices.md`. Every other place links to it, as a markdown
  link so it can be followed on GitHub, in one sentence.
- A cross-reference to another command appears once per section.
- A list (mock modes, devices, shortcuts) appears once per file.

## Order

- Sections follow the reader's path: confirm the meter is recognised, read,
  control, analyse, configure, then what only a few readers need. The first
  sections are what every user meets on the first run; the last serve bug
  reporters, scripters and assistive-technology users.
- Rank by the changelog ladder: works at all, then core capability, then more
  of a capability, then lookup tables and polish. A feature for one sensor or
  one meter family ranks below one every user meets.
- Within a section, the screen's own order: top to bottom, left to right.
- Detail a few readers need goes to the Appendix, linked from the section
  that uses it.
- A new section is inserted by rank, never appended.

## Step lists (setup.md, CONTRIBUTING.md)

- Numbered steps and the commands to run, a one-line reason only where the
  reader must choose between steps. The same exclusions and ownership apply;
  a troubleshooting heading is the message the tool prints, verbatim.

## Before commit

- Compare the new text with the sections around it. A section longer than its
  neighbours, for a feature no more important than theirs, needs cutting;
  the neighbours do not grow to match.
- Read the section as the user would and stop at the first sentence that
  explains instead of describes. That sentence goes, or moves to a design doc.
- After a subagent edits a reference, apply both checks to the diff before
  integrating it.
