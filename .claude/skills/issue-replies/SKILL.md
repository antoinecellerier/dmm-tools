---
name: issue-replies
description: >-
  Checklist for triaging GitHub issue and PR comments and drafting replies in
  this repo: reply structure and tone, what may be asserted vs framed as a
  hypothesis, citation rules, runnable asks, the standard device-report asks,
  and writing verified results back to the backlog. Use when starting to look
  at an issue ("check issue #NN", a newly opened issue, a reporter's comment,
  a bug or device report) — including in plan mode — and again before
  drafting any reply or running `gh issue comment` / `gh pr comment`.
---

# Triaging issues & drafting GitHub replies

The tracker holds pre-seeded `Help wanted:` threads (per-family protocol verification, platform testing, mode verification) plus whatever contributors open themselves — bug reports, device requests, questions. Triage turns a new issue or a comment into recorded evidence or into one precise, runnable ask. Creating verification issues belongs to `/add-device`.

## Triage a newly opened issue

- Classify first: bug, new-device request, platform report, question, or verification evidence for an existing family.
- Check `gh issue list --state all` for the pre-seeded thread that already covers the family or platform. Link it in the reply and continue where the reporter filed — don't make them re-file. Conversely, a bug or new-device report posted inside a `Help wanted` thread gets a friendly ask to open its own issue.
- Bug report: reproduce or read the code path before replying; cite `file:line`. If a fix lands, commit → push → cite the full SHA.
- New-device request: ask for the meter model, the cable's bridge chip and VID:PID, the user manual (PDF or link), and a `dmm-cli capture` — the tool flags unsupported models and records raw frames anyway. Implementation follows `/add-device`.
- Question: answer it directly, then correct any misconception behind it (a "missing driver" that is really missing software).

## Process

- Draft in the reply or the scratchpad, never as a file in the repo — the posted comment is the record.
- Post via `gh` only when told to ("post it"); the user often posts themselves. A reply citing a SHA or a `main` URL needs those commits on the remote first, so ask for the push before posting — "post it" authorizes the comment, not the push (`CLAUDE.md`, Commit discipline).
- End every reply with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

## Establish what is known before drafting

- Read the whole thread and any linked PR. Supersede stale commands quoted earlier in the thread (e.g. the `ut61eplus` → `dmm-cli` rename) in a footnote instead of repeating them.
- Treat the reporter's observed evidence (their `lsusb`/`system_profiler` paste, the meter's beep, the LCD) as authoritative over any decompile- or manual-derived inference (PR #8: our "Communication ON is sufficient" inference was wrong; the reporter's trace proved SET_MONITOR is required).
- Check `docs/verification-backlog.md` and the cable/bridge table in `docs/supported-devices.md` before asking for anything — never ask for what is already verified. Check what a doc's "confirmed" rests on before treating a report as re-confirmation (setup.md's macOS line initially rested on a single CH9329 cable, so a CP2110-on-macOS report was a new data point).
- Read the meter's manual in `references/<device>/` before theorising about its cable, modes or ranges — a lookup you perform, not an ask you send. Never cite `references/` paths.
- Cite community implementations (sigrok, antage, pylablib…) only for families whose spec already cross-references them (clean-room rule).
- Verify issue and PR numbers with `gh issue list` / `gh pr list` before citing them.

## Structure and tone

- Keep it light: body = answer, what we want confirmed, the ask(s). Every explanatory "why" goes to a footnote. Given a ladder of possible steps, ask for the single highest-value one.
- First person, warm, credit-forward: thank and @-mention the reporter, attribute findings and fixes to their report, credit tooling in prose where it matters ("I looked into this with Claude Code — …").
- Put heavy optional instructions (a full `capture`, multi-step experiments) in a collapsed `<details>` block behind the primary ask, with a quick option alongside (`--list-steps` / `--steps`).
- Make counts match structure: "three things" → exactly three numbered sections.
- Checkboxes track hardware confirmation, not code state: an item under "Fixed" stays `- [ ]` until a reporter confirms it on a meter. Append the observation that closes it in italics (*Any reading with distinct digits confirms this in one shot.*).
- Bold honesty caveats (**never tested against real hardware**, **no code bugs found**); state provenance ("verified from the rendered PDF, not text extraction").
- No emoji beyond the footer.

## Assert only what you validated

- State as fact only what was checked this session; phrase everything else as a hypothesis paired with the experiment that settles it. Checking is cheaper than hedging: `grep` for platform-specific code before saying "should behave the same on macOS"; `gh release view` before saying a prebuilt binary exists.
- Own corrections explicitly — a `### Retracted` section, "we had assumed X; your testing proved Y", a wrong number fixed in a comment ("79, not 97 — docs corrected"). Report a no-change outcome as loudly as a fix.
- For speculative asks, say the odds are low, make the risk concrete and cited, and lead with the most promising path.

## Formatting for GitHub

- Write each paragraph as one long line; GitHub reflows prose, and hard breaks make later edits diff the whole block. Fenced code blocks render verbatim — break them exactly as they should be run. (Repo docs stay hand-wrapped; this rule is for comments only.)
- Escape angle brackets: `--device <family>` inside backticks, `\<foo\>` elsewhere — bare `<tags>` render as invisible HTML.

## Citations must be clickable

- This repo's commits: full unquoted SHA (backticks suppress the auto-link). Cite the fix commit when telling a reporter a fix landed, and confirm it's pushed first.
- This repo's files: full `https://github.com/antoinecellerier/dmm-tools/blob/main/<path>` URL plus `#heading-anchor` where one exists — relative paths don't link from a comment. Issues and PRs: `#N`.
- External sources: explicit markdown URL that opens without auth or anti-bot walls.
- Never `references/`.

## Make asks runnable — and validating

- Copy-pasteable `sh` blocks; meter-side step as an inline comment (`# on the meter: SETUP → Communication → ON`); a revert step for anything changed (udev rule, `device_family` in settings).
- Design the ask so the reporter exercises the code path that needs validating; mention easier routes only as a failover (the packaged release archive when the question is whether packaged bits work; the bridge they actually own).
- Stage the ladder — detection → trace → capture — and make only the first rung the primary ask:

  ```sh
  dmm-cli list
  RUST_LOG=dmm_lib=trace dmm-cli --device <family> debug --count 5
  dmm-cli --device <family> capture   # in <details>; --list-steps / --steps for a subset
  ```

  State what success looks like at each rung (meter beeps on the streaming command; readings appear). Give the OS-native fallback for an empty `list`: `lsusb | grep -E '10C4:EA80|1A86:E429|1A86:E008'`, `ioreg -p IOUSB -l | grep -i CP2110`, Device Manager.

## Standard device- and platform-report asks

Link `CONTRIBUTING.md` for generic instructions; ask only for what the thread lacks:

- Meter model, and firmware version if shown at power-on.
- OS, version and architecture; prebuilt archive (which one) or source build.
- Bridge chip and VID:PID — CP2110 `10C4:EA80`, CH9329 `1A86:E429`, CH9325 `1A86:E008` (RX-only, no `command` support).
- Cable bundled or bought separately, and where/when — this is how production changes (CP2110 → CH9329 on the UT181A) get tracked.
- Per-step pass/fail with error output pasted; `capture-<device>.yaml` attached (auto-saves per step, resumable); an LCD photo beside the tool's output for any display-vs-parsed question.
- Name the highest-value captures when they matter: negative reading, overload, one frame per dial position, MIN/MAX/REL toggled in turn.
- Close platform threads with "even 'it works, no issues' is valuable"; ask whether any prerequisite was missing from the docs.
- Disambiguate a vague symptom before acting ("window doesn't appear, or opens with no data?"); when the feature exists, ask whether they tried it and it failed or didn't spot it.

## Write results back (same commit as the change)

- Reporter-verified item → strike and credit in `docs/verification-backlog.md`: `~~item~~ — **VERIFIED** YYYY-MM-DD by @user on real <meter> (<cable>). <evidence>. See PR #N.` Community-sourced but unrun → `per <source>`, no VERIFIED. Say in the reply that the backlog was updated.
- Family fully verified → follow the sign-off in `docs/adding-devices.md` (Stability flip, golden tests, `docs/supported-devices.md`).
- New unknown from the thread → backlog. Doc gap the reporter hit → fix it in the same commit and link it from the reply.
- Two to three weeks of silence on an ask → one polite nudge.

## Reply shapes

- **Answer + ask** (default): thanks → direct answer → validated vs expected → one primary ask as a `sh` block → `<details>` capture → footnotes → footer.
- **Closure comment** (platform/verification issue done): mirror the issue's checklist back with `[x]` and measured detail (test counts, device path seen), then "Fixes applied" with full SHAs.
- **Review-update broadcast** (after a spec audit): `## <topic> review update (YYYY-MM)` — what was re-audited against which source; `### Fixed (hardware confirmation pending)` with unchecked boxes and a closing observation per item; `### Still open` / `### Resolved` / `### Retracted` / `### Note` as needed; end with the capture command.
