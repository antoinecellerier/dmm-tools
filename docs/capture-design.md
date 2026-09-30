# Capture Design

`dmm-cli capture` is the verification path for every model short of `Stability::Verified`: a
guided run that walks the meter through its family's steps and writes one YAML report with the
raw bytes. It rests on three properties. A step advances on what the meter shows rather than on
a keypress. Every byte on the wire reaches the report, even when the parser rejects it. And the
tool's autonomy scales with how much of the family's protocol is already trusted.

The code is in `crates/dmm-cli/src/capture/`, and each section names the file to open. The step
definitions, `CaptureStep` and `Expect`, are in `dmm-lib`'s `protocol` module, because each
family declares its own.

## A. Auto-advance on observed state

Capture watches the meter instead of asking. After printing an instruction it polls
continuously and captures on its own once the meter reaches a new, stable state. Keys stay
as overrides: Enter captures now, `s` skips, `q` finishes the run. A dial-only step needs no
keypress at all: read the instruction, turn the dial, and the samples appear.

The watcher is `StateWatcher` in `watch.rs`, pure decision logic over readings. It picks one of
two detectors per step, by whether the step carries an `expect`:

- **Semantic** (step has `expect`): advance when `STABLE_FRAMES` consecutive frames satisfy
  the predicate — mode, flags, range auto/manual, value class. A stable state that does not
  match is reported once, and the tool keeps waiting rather than filing the wrong mode.
- **Raw-diff** (no `expect`): advance when the payload bytes the previous step's samples held
  constant change, and then hold. This works even when the parser is wrong about everything.
  The first step of a run has no baseline, so only Enter ends its wait.

Stability is the state holding still, not the value: the signature leaves the digits out, so a
reading still on its way satisfies it. After `STEP_TIMEOUT` with nothing new, the step offers
Enter and keeps watching, so an operator fetching a thermocouple is never stranded.

**Enter-only and gated steps.** Some changes cannot be seen arriving. At the dial position the
previous reading was already in, only the leads move, and open probes wander enough to satisfy
an expectation on their own, so the step waits for Enter. A step with `needs` is usually
reached dial first, and open leads pass "DC V, finite" before the battery is on, so it captures
on its own only after a reading in its mode has failed first. Both still report a wrong mode.
The exceptions, and a meter that alternates two states by design, are in `watch.rs`.

Command steps (`hold`, `minmax`, …) run the same watcher after `send_command`, against the
frames read just before it, and expect the flag to flip within `COMMAND_TIMEOUT`. If it does
not, the step files the error and no samples: pre-command frames filed as the result are what
made a dead command look like a captured state. A command whose flag flipped proves that
setting for the sweeps (D).

`--settle MS` waits before every sample, for readings that take seconds to come down after a
change, such as a UT61E+'s top two Ω rungs after a RANGE press. It is off by default. Waiting
for the digits themselves to hold still would never finish on leads with nothing stable across
them, and `r` at the confirmation prompt is the operator's guard when nobody asked for a delay.

A display with no reading is not a state worth sampling either. After a step's wait, and after
every switch the tool makes (D), `read_past_blank` in `step.rs` reads past a no-reading frame
for up to `BLANK_READS` readings. A UT181A sends no value for a while after any change, and
every range sub-step of one run held only that frame (issue #5).

A step whose samples start in its expected mode and then leave it is retaken, with a line
saying so: the operator moved on before sampling was done, and a UT804 run filed Diode and Ω
samples under `cont` that way (issue #16). Only the mode is compared, so a wandering value never
retakes (`left_mode` in `step.rs`).

## B. Full wire trace and parse diagnostics

`RecordingTransport` in `recording.rs` wraps the transport and logs every read and write with a
timestamp and the active step id. `open_recording_with_help` in `crates/dmm-cli/src/open.rs`
wraps it before detection and before the `Dmm` is built, so the detection probe and the init
handshake are in the trace too. The wrapper lives in the CLI: `Transport` is a public trait, so
it needs no library support.

Per step the report carries a bounded `frames` list beside `samples`. Received bytes that arrive
close together are one event (`RX_COALESCE_GAP_MS`), because the CP2110 hands up one byte per
HID report. The per-step cap, `MAX_FRAMES_PER_STEP`, keeps the newest events and counts the rest
in `frames_dropped`. That count does not flag the step: a step that waits on the operator passes
the cap as a matter of course.

Sampling records parse rejections into the step's `diagnostics` and keeps going instead of
breaking out: on an unproven protocol those are the most interesting frames. A step with an
unknown mode, a rejection or too few samples is flagged `needs_attention`, so a maintainer
scanning the YAML finds it without grepping.

A meter that never answers the check before the first step still leaves a report, the case
bring-up hits most: `no_response: true`, no steps, and the bytes received in `init_frames`. It
goes to a file of its own beside the report, so it never replaces a report in progress or an
earlier failure.

One file to attach stays the rule: no sidecar trace.

## C. Trust tiers

| Tier | When | Detector | Confirmation | Remote driving |
|---|---|---|---|---|
| 0 Sniff | `--sniff` | raw-diff | every step, inline | none |
| 1 Gate | short of `Stability::Verified` | semantic where the step declares an expectation | every step, inline | after the gate passes |
| 2 Trusted | `Stability::Verified`, or gate passed | semantic where the step declares an expectation | gate steps inline; deferred batch review for the rest | yes |

The **gate** is a small block of steps marked `gate` in the family's step list: DC V open, DC V
shorted, Ω open (OL), Ω across the operator's body, Ω shorted, and a negative reading. Those six
establish mode byte, digits, decimal point, OL and sign. The body reading is the only one that
puts non-zero digits on screen, open leads showing OL and shorted ones zero. Gate steps are
tagged `(gate)` as they run and confirm inline. The question is yes-or-no everywhere, and the
meter's text is asked for only after a no, so nothing compares typed digits and a word typed at
the prompt is never filed as the screen. A mismatch on a gate step is a failure whether or not
the operator said what the screen held.

`r` at the inline prompt retakes the step: its samples are dropped and the same wait runs
again, so a step captured before the leads were in place is redone rather than corrected. The
deferred review (F) has no retake, because by then the dial has moved on.

Once every gate step has a result, `Trust::update` in `report.rs` rules on it once and says so.
All confirmed: the report records `core_semantics: confirmed` and the run moves to tier 2.
Anything else: `core_semantics: failed`, the step ids under `gate_failures`, and the run stays
at tier 1.

Remote driving (D) runs only at tier 2, which makes where a family puts its gate steps
load-bearing. The run reaches tier 2 only when the *last* of them reports, and a gate step is
never swept itself, so a step scheduled among them is one whose ranges and flags nobody walks.
Declare the whole block first. A split gate cost the UT61+ list its AC V, DC mV and AC mV
ladders (issue #19); a test in `step.rs` holds the order, with the families still to fix named
in its allow-list.

A ladder the gate steps themselves sit on needs a plain step of its own after the block —
`ohm_ranges`, `dcv_ranges` on the UT61+ — since the gate step that established the mode cannot
be swept.

`--sniff` is the tier for a parser nobody trusts yet: `expect` is ignored, so every step
advances on the payload bytes changing, and a passed gate does not promote the run.

## D. Autonomous sub-steps through `select`

For families implementing `choices`/`select` (UT61+/UT161, UT181A, VC-880, VC-890, mock), each
mode step is followed, without a prompt, by a walk of Hold, Rel, every MinMax and Peak choice
and the Range ladder (`sweep_step` in `drive.rs`). Each sub-step captures the step's own sample
count and checks that the meter reports the target. Sub-steps are filed as `<mode>/hold:on`,
`<mode>/range:60V` and so on, which turns "one range per mode" into "every range per mode" at no
coordination cost. Mode is never swept: the dial is the operator's. Sub-steps are protocol
evidence, not screen checks, so they are neither confirmed inline nor listed in F's review, and
they do not count toward coverage.

Driving relies on reading mode, range and flags back correctly, so:

- It runs only at tier 2, and only after a non-gate step the operator set by hand.
- It is fail-soft. A refusal is filed as the sub-step's error and never retried. A setting that
  already worked this run is refused because the mode lacks it (MIN/MAX in continuity), so that
  refusal is neither counted nor flagged. `DRIVE_FAILURE_BUDGET` counted failures disable the
  sweeps for the rest of the run, and `MAX_DRIVE_SUBSTEPS_PER_STEP` caps one mode step.
- The baseline (auto range, flags off) is restored with a read-back before the next mode step,
  even once the budget is spent: a meter left latched in HOLD is worse than one more command. A
  range with no auto — a UT181A's in Peak — goes back to the rung the sweep started on.
- Range sweeps go through `choices(Range)` only, which cycles to target with read-back and stops
  when the read-back stops moving. No blind repeated presses.

A step whose `expect.mode` a button reaches from the current dial position — duty from Hz,
continuity from Ω — is switched to by the tool before the step is watched
(`switch_mode_from`), so the operator turns the dial and nothing else. The switch is filed as
its own sub-step, `<step>/mode:<label>`, so its frames survive the step's wait for a hand switch,
which would trim them off the step's cap (issue #20, a UT61B+ timing out partway through a
two-press walk). A plan naming several modes on one dial position reproduces such a bug without
the GUI: each switch starts from the mode the previous step left, which a one-off `dmm-cli set`
does not.

`--no-drive` opts out, for a cable that carries no commands or a cautious reporter. The report's
`drive` says whether the sweeps ran, so a report with no sub-steps says why.

## E. Per-step verification status

Each `CaptureStep` says whether hardware has confirmed it (`verified`), beside `gate`, `expect`,
`needs` and `wait_for_enter`. Around that flag:

- `dmm-cli capture --unverified` runs only unverified steps plus the freeform pass. This is the
  one-line ask in every device verification issue. With `--steps` the two intersect.
- `--list-steps` marks each verified step, so reporter and maintainer read the same list, and
  `--list-steps --format md` emits the checklist the issues use, so the issue and the code
  cannot drift.
- The epilogue prints how many unverified steps the report covers, and the issue to attach it
  to, from `DeviceProfile::feedback_url()`.
- Tests keep the flags honest: a family short of Verified declares at least one unverified step,
  and every gate step has an `expect`.

The family's `docs/research/<family>/verification.md` lists what a step would settle; once a
report settles it, the item goes, its result becomes a spec fact crediting the reporter (or a
code comment, for what the driver does), and the code flag flips in the same commit. Unknowns that are not modes
(VC-890 battery nibble, UT8802 byte 6) follow the "one step whose typed answer resolves it"
pattern, so the step stays the right unit of verification.

## F. Lower-friction confirmation

After the protocol steps and before the freeform pass, every reading captured without a
confirmation is printed as a numbered table (index, step id, the sample's summary line) and
the user gives the indices that were wrong; LCD text is typed only for those. The inline
prompt (C) and the freeform pass ask the same way, a closed question first and the text
after it, so no answer to one is a valid answer to the other. A run with nobody to ask —
stderr is not a terminal — skips the review and leaves those steps unconfirmed rather than
recording agreement nobody gave.

The answers are structured fields on `StepResult` in `report.rs`, which replaced a free-text
`screen` string. Reports carrying that string still load, so a resume across versions works.

## G. Preparation up front

Before the first step, capture lists what the run needs — shorted leads, a DC source, a
thermocouple, a live wire, a power adapter with a load, a transistor, an SCR — from the `needs`
tag on steps, each line naming the steps waiting on it. The user gives the numbers of anything
they haven't got, and those steps are marked `skipped` before the run starts, so the bench is
set up once instead of a thermocouple turning up mid-run. A later run asks again. The checklist
covers only the steps the run selected, and a run with nobody to ask prints the list and
attempts everything.

The step-order rules are in
[adding-devices.md](adding-devices.md#phase-6-real-device-verification).

## H. Maintainer-authored plans

`--plan file.yaml` runs a list of steps a maintainer pastes into an issue, so a nitpicky
sequence that does not belong in the family's shipped list is captured without waiting for a
release. A plan step sets the `CaptureStep` fields a plan may set. The keys and the errors are
in [the CLI reference](cli-reference.md#capture-plan-files), and the parser is `plan.rs`.

The plan replaces the device's list for that run and everything else stays: watcher, tiers,
needs checklist, sweeps and the freeform pass. Plan steps are never `gate`, so a Verified
family starts Trusted and reviews them in one pass while an Experimental one stays at Gate
and confirms each inline. The report records `plan: <file>`, and the epilogue counts the plan's
steps, since the device's unverified coverage says nothing about steps that are not in its
list. The default output file is named after the plan, so a plan run never resumes into the
full report.

## Report format

The schema is `CaptureReport` and `StepResult` in `report.rs`, with each field documented there.
The report level holds the run's context: the device, `init_frames`, `tier`, `core_semantics`,
`drive` and `plan`. Each step holds its `samples`, `frames`, `diagnostics`, the operator's
confirmation and `needs_attention`; sub-steps, mode switches included, are steps of their own. A
rejected transfer is visible as a frame with no matching sample and a line under
`diagnostics`. Every field added since the first report format is optional on read, so older
reports still resume.

```yaml
steps:
  - id: dcv
    instruction: "Set meter to DC V (V⎓). Leave leads open."
    status: captured
    samples:
      - raw_hex: "02 30 20 30 2E 30 30 30 30 00 00 30 30 30"
        mode_byte: "0x02"
        mode: "DC V"
        display_raw: "0.0000"
        value: "0.0"
        unit: "V"
        range_label: "2.2V"
    confirmed: true
    confirmed_by: inline
    frames:
      - at_ms: 0
        dir: tx
        hex: "AB CD 03 5E 01 D9"
      - at_ms: 41
        dir: rx
        hex: "AB CD 10 02 30 20 30 2E 30 30 30 30 00 00 30 30 30 03 88"
      - at_ms: 128
        dir: rx
        hex: "AB CD 10 02 30 20 30 2E 30 30 30 30 00 00 30 30 30 03 89"
    diagnostics:
      - "checksum mismatch: expected 0x0389, got 0x0388"
    needs_attention: true
```
