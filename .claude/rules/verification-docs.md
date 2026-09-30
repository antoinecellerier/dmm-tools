---
paths:
  - "docs/verification-backlog.md"
  - "docs/research/*/verification.md"
---

# Verification doc rules

A verification file is a to-do list for someone holding a meter: what to
check, and what the answer changes. What is already known lives in the spec
or the code, so the list only ever holds open work.

- Which file, first match wins:
  1. Code several families share (detection, the capture tool, framing, the
     app): `docs/verification-backlog.md`.
  2. A transport with its own research folder (`ut-d07b`): its own file.
  3. The UT61+ family: frame, flag and CP2110 items go to `ut61eplus`;
     commands, dials, range tables and the other models go to `ut61-family`,
     even when a UT61E+ runs them.
  4. Anything else: the family whose meter runs it. An item that needs two
     families lives with the one whose hardware runs it; the other file
     carries a one-line pointer.
- When an item is verified, delete it and record the result where it
  belongs. What the meter does goes into the spec, tagged `[HARDWARE]` (the
  UT61 specs use `[VERIFIED]`) with the issue, the cable and the reporter.
  What our driver does goes into the code comment beside it, with the issue;
  a capture step's `.verified()` flips in the same commit. A negative result
  is a fact too. The history stays in git and the issue.
- An item is one bullet of up to three lines: what to check, what it decides
  (the code or spec it would change), what running it takes (meter, cable,
  mode, or a capture step id), and the issue. Report narratives and dated
  logs go to the issue; community evidence stays in the spec's
  cross-reference section, and the item links it.
- Headings group items by topic and carry no dates: other files link to
  them as anchors. A family file's first line links its issues.
- Not verification: an idea goes to `docs/future-improvements.md`, a model
  we don't support to `docs/research/new-device-candidates.md`, a bug to
  "Known defects" in `docs/verification-backlog.md` (symptom, cause and fix,
  where known). A defect that only one family's meter can decide is an item
  in that family's file.
- A spec or approach doc keeps no to-do list of its own. An unconfirmed fact
  stays `[UNVERIFIED]` where it is stated, and the family's
  `verification.md` lists the check. A section that held such a list becomes
  a one-line pointer, so the section numbers other docs cite stay valid.
