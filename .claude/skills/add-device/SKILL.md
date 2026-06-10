---
name: add-device
description: End-to-end checklist for adding support for a new multimeter model — clean-room reverse engineering, implementation, docs, and the verification issue. Use when the user wants to add, evaluate, or start work on a new meter.
---

# Add support for a new multimeter

The authoritative methodology is `docs/adding-devices.md` — **read it in full before starting**. This skill is the checklist of gates and deliverables around it.

## Gates

1. **Clean-room source approval.** Only consume sources the user has explicitly approved (manual, vendor software, datasheets). Do not read community implementations or external code until the independent analysis is complete and the user approves cross-referencing. Document sources used and avoided so the boundary is auditable.
2. **Physical device steps need confirmation.** Describe each setup (dial position, leads, connections) and wait for the user to confirm before proceeding. Never assume the device state.
3. **Real-device verification before "done".** Capture raw bytes with:
   `RUST_LOG=dmm_lib=trace cargo run --bin dmm-cli -- --device <id> debug`
   Protocol code that has only been tested against mocks or captured traces is not done.

## Workflow

1. Discovery and candidate assessment (`docs/adding-devices.md` Phase 1): identify transport (VID:PID), gather vendor software + manual into `references/<device>/` (gitignored, never committed).
2. Clean-room reverse engineering (Phase 2): write up findings in `docs/research/<family>/reverse-engineered-protocol.md` and the RE methodology alongside it.
3. Implementation: transport (if new) + protocol family in `crates/dmm-lib/` — the path-scoped rules in `.claude/rules/protocol.md` apply. Tests use known-good byte sequences from real traces.
4. Track open unknowns in `docs/verification-backlog.md` as you go.

## Documentation deliverables (same commits as the change)

Touch **all** of:

- `README.md`
- `docs/supported-devices.md`
- `docs/verification-backlog.md`
- `docs/architecture.md`
- `docs/cli-reference.md`
- `docs/gui-reference.md`
- `CHANGELOG.md` (`## Unreleased`, user-visible phrasing)
- `docs/protocol.md` (index entry for the new family spec)

## GitHub verification issue

Create a verification issue matching the pattern of issues #3/#4/#5/#12/#13/#14 (use `gh issue view` on one for the template) and link it from `docs/supported-devices.md`. This tracks community validation on real hardware we don't own.
