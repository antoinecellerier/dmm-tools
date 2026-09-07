# Capture Design

`dmm-cli capture` is the verification path for every protocol family except the UT61E+, the
only `Stability::Verified` one. The design below reshapes it around three properties: a
step advances on what the meter shows rather than on a keypress, every byte on the wire
reaches the report even when the parser rejects it, and the tool's autonomy scales with how
much of the family's protocol is already trusted.

## A. Auto-advance on observed state

Capture watches the meter instead of asking. After printing an instruction it polls
continuously and captures on its own once the meter reaches a new, stable state. Keys stay
as overrides: Enter captures now, `s` skips, `q` finishes the run.

Two detectors, picked per step by whether the step carries an `expect`:

- **Semantic** (step has `expect`): advance when `STABLE_FRAMES` consecutive frames satisfy
  the predicate — mode label, flag values, range auto/manual, value class (Overload,
  Negative). A stable state that does not match is reported once
  (`meter shows: mode is "AC V", want "DC V"`) and the tool keeps waiting rather than filing
  the wrong mode.
- **Raw-diff** (no `expect`): advance when the payload bytes the previous step's samples held
  constant differ from that baseline and then hold for `STABLE_FRAMES` frames. Works even
  when the parser is wrong about everything. The first step of a run has no baseline, so only
  Enter ends its wait; a resumed run takes the baseline from the report's stored samples.

Stability means `STABLE_FRAMES` = 3 identical signatures — (mode, range, flags) plus, in
raw-diff, the baseline's constant payload bytes; digits may vary. `STEP_TIMEOUT` = 45 s with
nothing new prints "press Enter when the meter is ready" and keeps watching, so a user
hunting for a thermocouple is never stranded. A step whose `expect` the previous step's last
reading already satisfies is Enter-only and prints that line immediately: the mode is
unchanged and nothing observable would announce the action, as when the probes are shorted
on DC V. Ω across the body is not such a step — the same dial position, but OL to a finite
reading is a change open leads cannot fake. An Enter-only step still reports a mismatch, so
the wrong dial position is caught.

Command steps (`hold`, `minmax`, …) run the same watcher after `send_command`, against the
frames read just before it, and expect the flag to flip within `COMMAND_TIMEOUT` = 3 s. If it
does not, the step records `<command> did nothing; the meter still shows …` in its `error`
rather than filing pre-command frames — the fix for the stale-frame class of bug.

A dial-only step captures without a keypress: read the instruction, turn the dial, and the
samples appear. The per-step confirmation prompt after them is what F defers to the end of the run.

## B. Full wire trace and parse diagnostics

A recording layer wraps the transport and logs every read and write with a timestamp and the
active step id. `open_device_by_id_auto` splits into "open transport + protocol" and
`Dmm::new` so the CLI can insert the wrapper before construction, which puts the init
handshake in the trace too. The wrapper lives in the CLI — `Transport` is a public trait with
`&self` methods, so delegation is trivial.

Per step the report carries a bounded `frames` list (ms offset, direction, raw hex)
alongside `samples`; rejections are listed under `diagnostics`. Consecutive reads on one
step within 50 ms are recorded as a single event, because the CP2110 delivers one UART byte
per HID report and a 19-byte frame would otherwise be 19 events. The per-step cap and the
recorder's bound are explicit constants; the oldest events over the cap are trimmed and
counted in `frames_dropped`, which does not flag the step — a step that waits on the
operator passes the cap as a matter of course.

Sampling records `InvalidResponse`, `ChecksumMismatch` and `UnknownMode` into the step's
`diagnostics` and keeps going instead of breaking out — on an unproven protocol those are the
most interesting frames. A step whose samples contain `Unknown(0x..)` in `mode`, or an
unparsed value, is flagged `needs_attention: true` so a maintainer scanning the YAML finds it
without grepping.

One file to attach stays the rule: no sidecar trace.

## C. Trust tiers

| Tier | When | Detector | Confirmation | Remote driving |
|---|---|---|---|---|
| 0 Sniff | `--sniff` | raw-diff | every step, inline | none |
| 1 Gate | `Stability::Experimental` | semantic where the step declares an expectation | every step, inline | after the gate passes |
| 2 Trusted | `Stability::Verified`, or gate passed | semantic where the step declares an expectation | gate steps inline; deferred batch review for the rest | yes |

The **gate** is a small block of steps marked `gate: true` in the family's step list: DC V
open, DC V shorted, Ω open (OL), Ω shorted, and a negative reading. Those five establish mode
byte, digits, decimal point, OL and sign. They are tagged `(gate)` in the step header as they
run, and confirm inline with the existing prompt — Enter means the LCD shows exactly this
line, otherwise type what it shows. The decision is Enter-or-type everywhere, so nothing
compares typed digits; a typed correction on a gate step is a mismatch.

Once every gate step has a result the run rules on it, once, and says so. All captured and
confirmed: the report records `core_semantics: confirmed`, the run switches to tier 2, and
the steps after it are reviewed at the end. Any of them corrected, skipped, timed out or
errored: `core_semantics: failed` with the step ids under `gate_failures`, and the run stays
at tier 1 — inline confirmation, no remote driving. A resumed run rules on what the loaded
report already holds. The tier reached is recorded as `tier`.

Remote driving (D) runs only at tier 2, which also buys the deferred review.

`--sniff` is the tier for a parser nobody trusts yet: `expect` is ignored, so every step
advances on the payload bytes changing, and a passed gate does not promote the run.

## D. Autonomous sub-steps through `select`

For families implementing `choices`/`select` (UT61+/UT161, UT181A, VC-880,
VC-890, mock), each mode step is followed, without a prompt, by a walk of Hold
on/off, Rel on/off, every MinMax choice, every Peak choice and the Range
ladder, capturing the step's own sample count at each and verifying the meter
reports the target before moving on. Sub-steps are filed as `<mode>/hold:on`,
`<mode>/range:60V` and so on, which turns "one range per mode" into "every
range per mode" at no coordination cost. Mode is never swept: the dial is the
operator's. Sub-steps are protocol evidence, not screen checks, so they are
neither confirmed inline nor listed in F's review, and they do not enter the
coverage arithmetic — they are not steps the device declares.

These paths are hardware-unverified on three of the four families, so:

- Only at tier 2 and only after a non-gate step, because driving relies on
  reading mode, range and flags back correctly.
- Fail-soft: any refusal — `CommandRejected`, `UnsupportedCommand`, a timeout —
  records the sub-step with `status: error` and its `error`, and is never
  retried. A setting the meter has already taken a value on earlier in the run
  is refused because the current mode has no such function — MIN/MAX in
  continuity — so that refusal is filed but does not count: only an unproven
  setting's does. One refusal also answers for the setting's remaining choices
  in that step, which are not asked for. After `DRIVE_FAILURE_BUDGET` = 3
  counted failures the sweep says `remote control unreliable on this meter`
  once and disables itself for the rest of the run. At most
  `MAX_DRIVE_SUBSTEPS_PER_STEP` = 24 sub-steps are filed per mode step.
- The baseline (auto range, flags off) is restored with a read-back before the
  next mode step, even once the budget is spent — a meter left latched in HOLD
  is worse than one more command. A failed restore prints
  `<setting> could not be reset — press the meter's button` and counts a
  failure.
- Range sweeps go through `choices(Range)` only, which cycles to target with
  read-back and stops when the read-back stops moving. No blind repeated
  presses.
- REL is skipped while the reading is OL: the meter is entitled to refuse it
  there, and the refusal would spend the failure budget.

`--no-drive` opts out, for receive-only cables (CH9325) or a cautious reporter.
The report records `drive: on | off | disabled` — `off` for `--no-drive` and
for a family that offered no choice at all, `disabled` when the budget ran out.

## E. Per-step verification status

`CaptureStep` carries `verified: bool` (false for new steps), `gate: bool` (C) and
`expect: Option<Expect>` (A). Around them:

- `dmm-cli capture --unverified` runs only unverified steps plus the freeform pass. This is
  the one-line ask in every device verification issue. With `--steps` the two intersect.
- `--list-steps` marks each step ✓/· so reporter and maintainer read the same list, and
  `--list-steps --format md` emits the `- [ ]`/`- [x]` checklist the issues already use, so
  the issue and the code cannot drift.
- The epilogue prints coverage: "Covered 9 of 14 unverified steps for VC-890", followed by
  the issue to attach the report to, from `DeviceProfile::feedback_url()`.
- Tests: a Verified family declares no unverified steps, an Experimental family declares at
  least one, every gate step has an `expect`.

`docs/verification-backlog.md` keeps the *why*; verified step ids are struck through and
credited there as today, and the code flag flips in the same commit. Unknowns that are not
modes (VC-890 battery nibble, UT8802 byte 6) already follow the "one step whose typed answer
resolves it" pattern, so the step stays the right unit of verification.

## F. Lower-friction confirmation

After the protocol steps and before the freeform pass, every reading captured without a
confirmation is printed as a numbered table (index, step id, the sample's summary line) and
the user gives the indices that were wrong; LCD text is typed only for those. A run with
nobody to ask — stderr is not a terminal — skips the review and leaves those steps
unconfirmed rather than recording agreement nobody gave.

Structured fields replace the `screen: "confirmed: …"` string: `confirmed: Option<bool>`,
`lcd: Option<String>` for the typed correction, and `confirmed_by: inline | batch`. Reports
carrying the old `screen` string still load, so a resume across versions works.

## G. Preparation up front (planned)

Before the first step, capture lists what the run needs — shorted leads, a small DC source,
thermocouple, capacitor, live wire — derived from a `needs` tag on steps, and lets the user
deselect groups once ("no thermocouple" skips `temp` and `tempf`). Family step lists are
ordered so dial rotation is monotonic and lead changes are grouped; `docs/adding-devices.md`
states that rule for new families.

## H. Maintainer-authored plans (planned)

`--plan file.yaml` runs a list of steps with the same fields as `CaptureStep` (id,
instruction, command, samples, expect) in owned form. A maintainer pastes a short YAML into
an issue and the reporter runs it without waiting for a release — for nitpicky sequences that
do not belong in the family's shipped list.

## Report schema additions

Report level: `device_id`, `init_frames`, `wire_events_dropped` (B); `unverified_only`
(E); `tier`, `core_semantics`, `gate_failures` (C); `drive` (D); `plan` (H). Per step:
`frames`, `frames_dropped`, `diagnostics` and `needs_attention` (B); `confirmed`, `lcd` (typed corrections
only) and `confirmed_by` (F); sub-step ids (D). Frames are raw wire transfers; a rejected
one is visible as a frame with no matching sample and a line under `diagnostics`. Every
addition is optional on read, so older reports still resume.

```yaml
steps:
  - id: dcv/range:22V
    instruction: "Set range to 22V"
    status: captured
    needs_attention: false
    confirmed: true
    confirmed_by: batch
    samples:
      - raw_hex: "02 30 20 30 2E 30 30 30 30 00 00 30 30 30"
        mode: "DC V"
        display_raw: "0.0000"
        value: "0.0"
        unit: "V"
        range_label: "22V"
    frames:
      - at_ms: 0
        dir: tx
        hex: "AB CD 03 46 01 C1"
      - at_ms: 41
        dir: rx
        hex: "AB CD 10 02 30 20 30 2E 30 30 30 30 00 00 30 30 30 03 2B"
      - at_ms: 128
        dir: rx
        hex: "AB CD 10 02 30 20 30 2E 30 30 30 30 00 00 30 30 30 03 2C"
    diagnostics:
      - "checksum mismatch: expected 0x032B, got 0x032C"
```
