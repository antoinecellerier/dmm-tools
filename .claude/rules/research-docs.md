---
paths:
  - "docs/research/**"
  - "docs/architecture.md"
---

# Research and architecture doc rules

A family spec answers one question: what does the meter do on the wire. The
approach doc beside it records how we learned it, and `docs/architecture.md`
describes our code for every meter at once.

- `docs/research/<family>/reverse-engineered-protocol.md` records what the
  meter does. How our driver reacts — retries, timeouts, "the parser keeps…",
  "we follow…" — goes to `docs/architecture.md` when it holds for every
  meter, otherwise to a code comment or `docs/verification-backlog.md`.
- An Implementation Notes section states what the wire requires of any
  decoder, as facts about the meter ("packets end in CR LF; bit 7 is
  parity"), not as directives to our code.
- `docs/architecture.md` is device-agnostic: no byte offsets, flag bits or
  per-meter quirks. They live in the family spec; architecture links.
- Each `reverse-engineering-approach.md` lists the sources used and the
  sources avoided, with the date a boundary was opened.
- Community sources are cited only in a labelled cross-reference section.
- A value no source confirms is marked `[UNVERIFIED]`.
