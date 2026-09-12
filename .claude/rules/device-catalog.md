---
paths:
  - "docs/supported-devices.md"
---

# Device catalog rules (supported-devices.md)

The catalog answers one question: does my meter work, and what do I need.
Cables first, then one section per protocol family, then what is not
supported yet. The general rules are in `docs-user-facing.md`.

- A family section is: one line naming the form factor, one line for the
  cable, one line for what to switch on, one table, and at most one
  paragraph on what hardware runs have confirmed and what is pending, linking
  the backlog.
- Table columns are Model, Counts, Status, Notes. A cell is one clause; a
  value the source does not give is `—`, never a guess. Notes carry what
  distinguishes the model from its siblings, nothing about the protocol.
- Status is the registry's vocabulary and nothing else: `✅ Verified` or
  `🧪 Experimental` with the verification issue linked. The README's device
  table uses the same words.
- Research stays out: decompilation sources, line counts, hashes,
  cross-correlation with community code, protocol summaries, candidate
  market analysis. It lives in `docs/research/<family>/` and
  `docs/research/new-device-candidates.md`; the catalog links.
