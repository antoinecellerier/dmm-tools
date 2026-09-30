---
paths:
  - "docs/architecture.md"
  - "docs/*-design.md"
  - "docs/development.md"
  - "docs/adding-devices.md"
  - "docs/protocol.md"
  - "docs/future-improvements.md"
---

# Developer doc rules

A contributor reads these to learn how the pieces fit and why, then goes to
the code for the rest. The doc names the type or file to open; the
mechanism, edge cases and constants live in that file's doc comments, where
they change with the code.

- A sentence that must change when a private function is renamed or a
  constant retuned belongs in the code, not here.
- One owner per topic. Other docs link to it instead of restating it.
- Updating a section means rewriting the paragraph so it still reads whole,
  never appending a clause. A section past about a screen pushes detail down
  into the code.
- Table cells are one line: a cell is where clauses pile up unseen. A
  family's internals are its `mod.rs` docs.
- Design docs describe the current design. A reason stays, with its issue
  number; dated investigation logs go to `docs/verification-backlog.md` or
  the issue.
- A design decision is a bold label and its reason in one to three lines,
  cited by label, never by number: numbers shift when a decision is added or
  merged, and every citation breaks silently.
- A doc describes the work, not an assistant's conduct. "Ask first", "wait
  for confirmation" and "with the maintainer's approval" belong in
  `CLAUDE.md`, `.claude/rules/` or a skill; check they are there before
  cutting them from a doc.
- `docs/architecture.md` is device-agnostic: no byte offsets, flag bits,
  per-meter quirks or model examples. They live in the family spec or the
  family's code, and architecture links.
- Before cutting text, find where each fact lives afterwards: a comment
  stating it, a new comment beside the code it explains, or the owner doc.
  Code that merely does something holds a *what*, never a *why*.
