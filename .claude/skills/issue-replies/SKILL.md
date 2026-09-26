---
name: issue-replies
description: >-
  Triages GitHub issues and PR comments in this repo and drafts the replies:
  structure and tone, self-checkable asks, what may be asserted vs framed as a
  hypothesis, citation rules, the standard device-report asks, dev builds for
  reporter testing, and writing verified results back to the backlog and the
  issue. Use when starting to look at an issue ("check issue #NN", a newly
  opened issue, a reporter's comment, a bug or device report) — including in
  plan mode — and again before drafting any reply or running
  `gh issue comment` / `gh pr comment`.
---

# Triaging issues & drafting GitHub replies

The tracker holds pre-seeded `Help wanted:` threads (per-family protocol verification, platform testing, mode verification) plus whatever contributors open themselves — bug reports, device requests, questions. Triage turns a new issue or a comment into recorded evidence or into precise asks the reporter can run and judge on their own. Creating verification issues belongs to `/add-device`.

## Triage a newly opened issue

- Classify first: bug, new-device request, platform report, question, or verification evidence for an existing family.
- Check `gh issue list --state all` for the pre-seeded thread that already covers the family or platform. Link it in the reply and continue where the reporter filed — don't make them re-file. Conversely, a bug or new-device report posted inside a `Help wanted` thread gets a friendly ask to open its own issue.
- Bug report: reproduce or read the code path before replying; cite `file:line`. If a fix lands, commit → push → cite the full SHA.
- New-device request: ask for the meter model, the cable's bridge chip and VID:PID, the user manual (PDF or link), and a `dmm-cli capture` — the tool flags unsupported models and records raw frames anyway. Implementation follows `/add-device`.
- Question: answer it directly, then correct any misconception behind it (a "missing driver" that is really missing software).

## Process

- Draft in the scratchpad, never as a file in the repo — the posted comment is the record.
- Re-read the thread (`gh issue view N --comments`) before drafting and again before posting. The user often posts their own reply, and reporters' messages cross ours: build on what is already there, never repeat it.
- A fix for a reporter to test ships on its own: when `git log origin/main..main` holds unrelated work, build it on a branch from `origin/main` in a scratchpad worktree (`CARGO_TARGET_DIR` under `~/.cache`), then push `<branch>:main` on the user's OK.
- A reply that sends the reporter to a fix needs the build to exist first: push → `gh workflow run dev-build.yml` → `gh release view dev-<short-sha>` lists the archives → post. Ask for the push and the build run before posting — "post it" authorizes the comment, not the push (`CLAUDE.md`, Commit discipline).
- Post via `gh` only when told to; the user often posts themselves. Re-read the draft file first — the user may have edited it.
- End every reply with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

## Establish what is known before drafting

- Read the whole thread and any linked PR. Supersede stale commands quoted earlier in the thread (e.g. the `ut61eplus` → `dmm-cli` rename) in a footnote instead of repeating them.
- Treat the reporter's observed evidence (their `lsusb`/`system_profiler` paste, the meter's beep, the LCD) as authoritative over any decompile- or manual-derived inference (PR #8: our "Communication ON is sufficient" inference was wrong; the reporter's trace proved SET_MONITOR is required).
- Note which build the reporter ran (`--version`, the archive's folder name) and check that every flag, step id and quoted string exists at the commit of the build you point them at (`git grep '<string>' <sha>`).
- Check `docs/verification-backlog.md` and the cable/bridge table in `docs/supported-devices.md` before asking for anything — never ask for what is already verified. Check what a doc's "confirmed" rests on before treating a report as re-confirmation (setup.md's macOS line once rested on a single CH9329 cable, so a CP2110-on-macOS report was a new data point).
- Read the meter's manual in `references/<device>/` before theorising about its cable, modes or ranges — a lookup you perform, not an ask you send.
- Cite community implementations (sigrok, antage, pylablib…) only for families whose spec already cross-references them (clean-room rule).
- Verify issue and PR numbers with `gh issue list` / `gh pr list` before citing them.

## Structure and tone

- Short, and led by what is new. Every sentence must help the reporter act or understand their result; cut the rest — an answer to our own side question they didn't take up, an ask that doesn't serve the verification goal, internals (nibbles, feature reports) not tied to their own evidence.
- Explain in the reporter's terms: what the meter or the tool does, quoting their own bytes or output when technical detail is needed.
- Open with thanks and the @-mention on its own line ("Thanks for all the data @user!"), then short declarative sentences. First person, warm, credit-forward: attribute findings and fixes to their report, credit tooling in prose where it matters ("I looked into this with Claude Code — …").
- List what their report changed as one-line bullets under a link to the dev build ("Your notes changed a few things in [dev-\<sha\>](…):").
- Own the tool's bugs that hit their run, a sentence each ("Two bugs of mine turned up in your capture run", "…was the tool's fault").
- When asking for their time, close with thanks for it ("Thank you in advance for your time completing these captures!").
- Name evidence sources concretely: "our UT61E+", "@user's UT181A" — never "in-house" or "third-party".
- Write optional steps inline, as a sentence or a numbered step. `<details>` is only for a pasted plan YAML.
- Make counts match structure: "three things" → exactly three numbered sections.
- In a checklist, boxes track hardware confirmation, not code state: an item under "Fixed" stays `- [ ]` until a reporter confirms it on a meter. Append the observation that closes it in italics (*Any reading with distinct digits confirms this in one shot.*).
- Bold honesty caveats (**never tested against real hardware**, **no code bugs found**); state provenance ("verified from the rendered PDF, not text extraction").
- No emoji beyond the footer.

## Assert only what you validated

- State as fact only what was checked this session; phrase everything else as a hypothesis paired with the experiment that settles it. Checking is cheaper than hedging: `grep` for platform-specific code before saying "should behave the same on macOS"; `gh release view` before saying a prebuilt binary exists.
- Own corrections explicitly — a `### Retracted` section, "we had assumed X; your testing proved Y", a wrong number fixed in a comment ("79, not 97 — docs corrected"). Report a no-change outcome as loudly as a fix.
- For speculative asks, say the odds are low, make the risk concrete and cited, and lead with the most promising path.

## Formatting for GitHub

- Write each paragraph as one long line; GitHub reflows prose, and hard breaks make later edits diff the whole block. Fenced code blocks render verbatim — break them exactly as they should be run. (Repo docs stay hand-wrapped; this rule is for comments only.)
- Escape angle brackets: `--device <id>` inside backticks, `\<foo\>` elsewhere — bare `<tags>` render as invisible HTML.

## Citations must be clickable

- This repo's commits: full unquoted SHA (backticks suppress the auto-link). Cite the fix commit when telling a reporter a single fix landed, and confirm it's pushed first.
- Dev builds: `[dev-<sha>](https://github.com/antoinecellerier/dmm-tools/releases/tag/dev-<sha>)`, naming the archive for the reporter's platform when known ("linux-x86_64 archive"). For a list of several changes, the dev-build link stands in for the commits. A verification issue's body links the dev build listing instead (`/add-device`, `verification-issue.md`).
- This repo's files: full `https://github.com/antoinecellerier/dmm-tools/blob/main/<path>` URL plus `#heading-anchor` where one exists — relative paths don't link from a comment. Issues and PRs: `#N`.
- External sources: explicit markdown URL that opens without auth or anti-bot walls.
- Never `references/` paths or decompile line numbers.

## Make asks runnable and self-checkable

- While nothing works yet, ask for the one trace that explains why. Once a fix is in a build, tested or not, send every remaining ask in one reply — each round trip costs the reporter a session at the bench: bold numbered steps in order, later ones gated on earlier outcomes (**4. If step 1 worked: a full capture**), optional ones marked. Add questions answered by looking at the meter, no command needed ("in diode mode with the leads open, is there a dot before the OL?").
- Each step is a copy-pasteable `sh` block run from the build's folder as `./dmm-cli` (an extracted archive is not on `PATH`); meter-side step as an inline comment (`# on the meter: SETUP → Communication → ON`); a revert step for anything changed (udev rule, `device_family` in settings). For a Windows reporter, give the PowerShell form: `$env:RUST_LOG="dmm_lib=trace"` on its own line, then `.\dmm-cli.exe …`.
- Pair every command with what success and failure look like on the reporter's screen, quoting the exact strings that build prints ("It works if it prints `Detected: UT804`; "No meter answered over the USB cable." means it doesn't."). An either/or experiment names both outcomes and asks which one happened; a result that looks like failure but is valid says so ("No response from meter." is a valid result on the HOLD step). If anything the reporter will see would read ambiguously — a step's wording, the end-of-run summary — fix it in code before asking.
- Keep results on screen and logs in files: `2> <file>.txt` sends the trace to the file while readings still print — no `grep` for the reporter. Name files after the model, build and purpose (`ut804-dev-666fbf5-readings.txt`), never a bare counter.
- For a full capture, list the steps it walks inline, a word or two each ("DC V, DC V shorted, AC V, …"), so the reporter can check their coverage; say that `s` skips a position their meter lacks and that re-running offers to resume.
- Design the ask so the reporter exercises the code path that needs validating; mention easier routes only as a failover (the packaged release archive when the question is whether packaged bits work; the bridge they actually own).
- Detection problems: run the ladder detection → trace → capture, with success stated at each rung (meter beeps on the streaming command; readings appear):

  ```sh
  ./dmm-cli list
  RUST_LOG=dmm_lib=trace ./dmm-cli --device <id> debug --count 5 2> <id>-<build>-readings.txt
  ./dmm-cli --device <id> capture   # --unverified or --steps a,b for a subset
  ```

  Give the OS-native fallback for an empty `list`: `lsusb | grep -E '10C4:EA80|1A86:E429|1A86:E008'`, `ioreg -p IOUSB -l | grep -i CP2110`, Device Manager.

## Standard device- and platform-report asks

Link `CONTRIBUTING.md` for generic instructions; ask only for what the thread lacks:

- Meter model, and firmware version if shown at power-on.
- OS, version and architecture; prebuilt archive (which one) or source build.
- Bridge chip and VID:PID — CP2110 `10C4:EA80`, CH9329 `1A86:E429`, CH9325 `1A86:E008` (RX-only, no `command` support).
- Cable bundled or bought separately, and where/when — this is how production changes (CP2110 → CH9329 on the UT181A) get tracked.
- Per-step pass/fail with error output pasted; `capture-<device>.yaml` attached (auto-saves per step, resumable); an LCD photo beside the tool's output for any display-vs-parsed question. GitHub rejects `.yaml` uploads, so ask for files renamed to `.txt`.
- Match the artefact to the symptom: a `capture` report for readings, parsing or a mode; a `.replay` file — GUI Export… → Replay…, or `dmm-cli read --format replay -o bench.replay` — for behaviour over time (graph, triggers, gaps, a button sequence), which replays their session on our bench.
- Name the highest-value captures when they matter: negative reading, overload, one frame per dial position, MIN/MAX/REL toggled in turn.
- An edge case the shipped steps don't cover: paste a plan YAML in a `<details>` block and ask for `./dmm-cli --device <id> capture --plan edge.yaml` — steps are `id` + `instruction` plus optional `command`, `samples`, `needs` and `expect` (see `docs/cli-reference.md`), and the report attaches like any other.
- Close platform threads with "even 'it works, no issues' is valuable"; ask whether any prerequisite was missing from the docs.
- Disambiguate a vague symptom before acting ("window doesn't appear, or opens with no data?"); when the feature exists, ask whether they tried it and it failed or didn't spot it.

## Write results back (same commit as the change)

- Reporter-verified item → strike and credit in `docs/verification-backlog.md`: `~~item~~ — **VERIFIED** YYYY-MM-DD by @user on real <meter> (<cable>). <evidence>. See PR #N.` Community-sourced but unrun → `per <source>`, no VERIFIED.
- Verification issue body → updated in the same round as the reply: regenerate the checklist (`dmm-cli --device <id> capture --list-steps --format md`, never hand-edited — a verified item flips the step's `.verified()` in the code), and update the summary, and the dev-build line where it names a `dev-<sha>`. Show it, then `gh issue edit` on a go-ahead.
- A `CHANGELOG.md` entry saying a model works or is verified, with credit, lands in the commit that records the reporter's confirmation, not before; a targeted fix with credible evidence gets its entry with the fix (`.claude/rules/changelog.md`).
- Family fully verified → follow the sign-off in `docs/adding-devices.md` (Stability flip, golden tests, `docs/supported-devices.md`).
- New unknown from the thread → backlog. Doc gap the reporter hit → fix it in the same commit and link it from the reply.
- Two to three weeks of silence on an ask → one polite nudge.

## Reply shapes

- **Answer + ask** (first contact, or nothing works yet): thanks → direct answer → validated vs expected → one ask as a `sh` block with its success and failure → footer.
- **Fix to test**: thanks → what their evidence showed, in their own bytes or output → the fix commit and the dev build → numbered, gated steps, each with success and failure → thanks for their time → footer.
- **Follow-up** (after the user's own reply, or messages that crossed): what's new in a few sentences, optional steps inline; no repeated thanks, version notes or asks.
- **Results round** (the reporter confirmed things): thanks on its own line → what is now confirmed (stability, test suite, firsts) → what their notes changed, as bullets under the dev-build link → remaining asks, marked optional → thanks again → footer.
- **Closure comment** (platform/verification issue done): mirror the issue's checklist back with `[x]` and measured detail (test counts, device path seen), then "Fixes applied" with full SHAs.
- **Review-update broadcast** (after a spec audit): `## <topic> review update (YYYY-MM)` — what was re-audited against which source; `### Fixed (hardware confirmation pending)` with unchecked boxes and a closing observation per item; `### Still open` / `### Resolved` / `### Retracted` / `### Note` as needed; end with the capture command.

## Review the draft before showing it

Check each line; revise and re-check until all hold:

- [ ] The thread is re-read; nothing repeats what the user or the reporter already said.
- [ ] Every command runs from the build it names, and every flag exists in that build.
- [ ] Every ask states success and failure as the reporter will see them, with strings checked in the source.
- [ ] No sentence could go without losing an action or a result.
- [ ] Cited SHAs are pushed, the dev build is published, every link is clickable.
- [ ] Footer present.
