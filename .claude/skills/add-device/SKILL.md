---
name: add-device
description: Runs the work of adding a new multimeter model, over USB or Bluetooth — candidate scoping, clean-room reverse engineering, the plan, implementation through subagents, standing reviews, docs, and the verification issues. Use when the user wants to add, evaluate, plan or start work on a new meter or meter group, including in plan mode.
---

# Add support for a new multimeter

`docs/adding-devices.md` is the methodology — sources, tools, code steps, capture-step rules and the Phase 7 doc touchpoints. Read it in full before starting. This skill is the run-book around it: the gates, where to stop and ask the user, what each subagent brief carries, and the reviews that run without being asked.

## Gates

1. **Clean room** (CLAUDE.md): community sources stay closed until phase 4 opens them.
2. **Physical steps need confirmation**, one setup at a time. Say early which phases will need the meter so it can stay off meanwhile (the UT-D07B adapter sleeps after 5 minutes idle; a paired adapter that blinks but never connects: `docs/setup.md`, "Bluetooth adapter not found").
3. **Hardware decides "done"** (`.claude/rules/protocol.md`). Without a real-device run the model ships `Experimental`, with backlog lines for what waits.
4. **Side effects stay on the new model.** Every change that runs for an already-supported meter is listed with its before and after, and needs a reason that spans devices.
5. **No invented issue numbers** — the `ISSUE_TO_OPEN` pattern in `docs/adding-devices.md`, Phase 4.

## Phases

### 1. Scope (ask the user)

- Check `docs/research/new-device-candidates.md` and `docs/research/<family>/` first: RE, cross-reference or assets may already exist. Start from what is missing.
- Group the candidates by protocol and take the most popular group first (EEVBlog threads are the popularity signal).
- One registry entry per distinct table set, or per packet layout for a meter that never names its model, so detection never shows a name the packets can't prove. Issues: `docs/adding-devices.md`, Phase 7.
- If the sources can't yield a decoder (no manual, a vendor handler that decodes nothing), stop and bring the options: park it in the candidates doc, or ask a reporter for a capture first.
- **Ask:** the group, the entries, and whether the user has the meter.

### 2. Acquire (main context)

Download and archive per `docs/adding-devices.md` Phases 1–2 into `references/<dev>/` with a `SOURCE.txt`. Prepare readable sources before any agent opens them: jadx for APKs, Ghidra for native code (for Delphi, the RTTI-seeded method in `docs/research/ut803/reverse-engineering-approach.md`), beautified JS with offset prefixes (a working tool: `references/zotek/e-bull-v2/pretty.js`). Agents read decompiler output; raw disassembly only for a branch the decompiler drops.

### 3. Clean-room RE (subagents)

Mechanical reading goes to subagents; the main context briefs, saves and adjudicates. Each brief:
- names the approved source directories by path, and the ones it must not open;
- asks for the findings as its final message under a fixed first heading — subagents can't write report files;
- says `.claude/rules/research-docs.md` applies, and that every fact carries a source cite and a tag.

Save each report to `references/<dev>/analysis/findings/<name>.md` at once; `/tmp` does not survive a reboot. A lost report is the longest string input of the `*Handback*` tool call in `~/.claude/projects/<project>/<session>/subagents/agent-<id>.jsonl`. A side question whose research could bias the drafting agent (may we publish this key, what does a community project say) goes to a separate fresh agent; only the decision comes back.

Then a **grounding check** by a fresh subagent: every cite against its source, every tag against its evidence, and anything that reads as filled in from memory. The drafter applies the fixes; a **narrow re-check** covers only the changed rows, because fixes bring new errors.

### 4. Community cross-reference (ask the user)

Ask to open the boundary once the spec is committed; then do it without further prompting. Re-check each contradiction against the vendor sources before recording who is right. New facts go back into the committed spec: the cross-reference section, plus pointers from the body.

### 5. Plan (ask the user)

Before ExitPlanMode, a Plan subagent critiques the draft; fold in its fixes. Its checklist:
- the table of changes that reach existing meters (gate 4);
- touchpoints: `git grep` the nearest sibling's id, display name and, for a new bridge, VID:PID — every hit is a candidate. A first meter on a new transport also greps wording that assumes a cable;
- keys and commands: the vendor app's or manual's set, with their codes;
- a simulated meter only if the generic mock can't show what the meter adds;
- capture steps for every mode, AUTO included (`docs/adding-devices.md`, Phase 6);
- Bluetooth: names, fields and failure modes (sleep, link loss, pairing);
- the issue count;
- layering: transport vs protocol, module split;
- a commit series in which every commit builds.

### 6. Implement (subagents in worktrees)

Each brief carries the decisions plus:
- behaviour keyed on the model; `report_unknown` for data the spec doesn't cover, never a silent skip;
- instruction text from this model's manual, never a sibling's;
- placeholder Bluetooth addresses and serials in tests and docs, never the user's;
- its own `CARGO_TARGET_DIR=~/.cache/dmm-<task>`, deleted when done;
- review fixes as `--fixup` commits on the commit they fix, then `git rebase -x 'cargo check --workspace --all-targets' <base>` over the series before handing back — replayed picks skip the hook, which still gates the tip.

### 7. Reviews (standing)

Run these without being asked, and don't wait on the issues for them:
1. **Capture steps.** Render `dmm-cli --device <id> capture --list-steps --format md` for each new id and read it against the manual and the Phase 6 capture-step rules. This catches what code review misses.
2. **Code.** A subagent runs the `/code-review` skill.
3. **User-facing text.** A subagent reads every changed string and doc against `.claude/rules/docs-user-facing.md`, `device-catalog.md`, `readme.md` and `changelog.md`: one term per concept, per-meter figures named, each new model and accessory under `### Devices` as experimental.
4. **Pictures.** `scripts/doc-screenshots.sh <asset>` for each documented picture whose UI text changed.

### 8. Docs and verification issues (ask the user)

Work through `docs/adding-devices.md` Phase 7. Draft the issues per `verification-issue.md` (beside this file) and show them. On the user's word, post them, then link them in one commit: the profiles' `verification_issue`, the README row, the catalog Status, the changelog and the backlog, deleting `ISSUE_TO_OPEN`.

### 9. Push and wrap-up (ask the user)

- Leak check before asking to push: `git log -p origin/main..` for `([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}` (Bluetooth addresses) and `/home/`.
- Ask for the push (CLAUDE.md, Commit discipline). The issues point at the newest nightly dev build, which won't carry the device until the next night; offer `gh workflow run dev-build.yml` if the user wants it sooner.
- Backlog lines for what waits on reporters, including the spec tables.
- `du -sh ~/.cache/dmm-* 2>/dev/null`; delete the dirs no agent is still building in.
