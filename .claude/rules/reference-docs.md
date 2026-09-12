---
paths:
  - "docs/cli-reference.md"
  - "docs/gui-reference.md"
---

# Reference doc rules (cli-reference.md, gui-reference.md)

The references answer two questions: what does this feature do, and which
option or control drives it. The general rules are in `docs-user-facing.md`;
this file is the shape of a reference section.

## Shape

- An option, flag, control or shortcut is a table row: what it does, the
  default, when it is absent. One clause each.
- Behaviour the user cannot see on screen is one short paragraph: what
  happens, when, and how to undo it. Semantics, side effect and persistence
  get one sentence each.
- One example block per command or section. One JSON example, not one per
  shape the output can take.
- Every sentence passes two tests: a user needs it to use the feature, and
  they cannot see it when it happens. If the screen, a hover, a message or the
  terminal already says it, cut it.
- Detail only a few readers need (a plan file's keys, a JSON field list) goes
  in an appendix at the end of the file, linked from the command, so the
  commands stay readable in sequence.
