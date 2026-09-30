---
paths:
  - "docs/research/**"
---

# Research doc rules

A family spec answers one question: what does the meter do on the wire. The
approach doc beside it records how we learned it, and `docs/architecture.md`
describes our code for every meter at once.

- `docs/research/<family>/reverse-engineered-protocol.md` records what the
  meter does. How our driver reacts — retries, timeouts, "the parser keeps…",
  "we follow…" — goes to a code comment beside the code that does it, to
  `docs/architecture.md` only when it shapes every family, or to
  `docs/verification-backlog.md` while it is open.
- An Implementation Notes section states what the wire requires of any
  decoder, as facts about the meter ("packets end in CR LF; bit 7 is
  parity"), not as directives to our code.
- Each `reverse-engineering-approach.md` lists the sources used and the
  sources avoided, with the date a boundary was opened.
- Community sources are cited only in a labelled cross-reference section.
- A value no source confirms is marked `[UNVERIFIED]`.
- Absence is what a search found: "none found in <sources>, <date>", never
  "none exists" or "no one rebrands it".
