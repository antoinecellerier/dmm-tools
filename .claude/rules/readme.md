---
paths:
  - "README.md"
---

# README rules

The README is read two ways: by a visitor deciding in ten seconds what the
project does and where to learn more, and by search engines. It is an entry
point, not a fourth reference. The general rules are in
`docs-user-facing.md`.

- The first paragraph names what the tool does, the meter families and
  models, and the platforms, in prose — a crawler weights the first hundred
  words, and a table cell less than a sentence. Cable names belong in the
  catalog, not the intro.
- The GUI and CLI lists are what a user can do, at most six bullets each,
  the most-used capability first. No mechanism, no option names except the
  one flag that names the feature.
- Quick start is the three steps to a first reading: get the binary, the one
  platform step that blocks a first run, run. Everything else links to
  `docs/setup.md`; never restate it.
- The Documentation index is split by persona: user docs as a list, one line
  for contributors. Supported devices is a user doc.
- Acknowledgements are credit, kept compact: one bullet per project, what it
  contributed, no methodology.
- The device table is hand-written and guarded by a test; keep its status
  words those of `Stability::label()`, capitalised, with the emoji
  `device-catalog.md` gives them.
- One device row per verification issue, brand first in the Meter cell;
  columns Meter, USB, Bluetooth, Issue. A Bluetooth status says "(adapter)"
  or "(built in)"; `—` marks a link the meter is not supported over.
- The screenshot is refreshed at a release whose changelog changes what the
  picture shows.
