# Capture Design

`dmm-cli capture` is the verification path for every protocol family except the UT61E+, the
only `Stability::Verified` one. The design below reshapes it around three properties: a
step advances on what the meter shows rather than on a keypress, every byte on the wire
reaches the report even when the parser rejects it, and the tool's autonomy scales with how
much of the family's protocol is already trusted.

## A. Auto-advance on observed state (planned)

Capture watches the meter instead of asking. After printing an instruction it polls
continuously and captures on its own once the meter reaches a new, stable state. Keys stay
as overrides: Enter captures now, `s` skips, `q` finishes the run.

Two detectors, picked per step by whether the step carries an `expect`:

- **Semantic** (step has `expect`): advance when K consecutive frames satisfy the predicate —
  mode label, flag values, range auto/manual, value class (Overload, Negative). A stable
  state that does not match is reported ("meter shows AC V, step wants DC V") and the tool
  keeps waiting rather than filing the wrong mode.
- **Raw-diff** (no `expect`, or the parser is untrusted): advance when the payload's
  non-digit bytes differ from the previous step's baseline and then hold identical for K
  frames. Works even when the parser is wrong about everything.

Stability means K identical (mode, range, flags) tuples, or K identical non-digit raw bytes;
digits may vary. K = 3, with a per-step inactivity timeout that falls back to "press Enter
when ready" so a user hunting for a thermocouple is never stranded.

Command steps (`hold`, `minmax`, …) run the same watcher after `send_command` and expect the
flag to flip. If it does not flip within the timeout the step records `did nothing` in its
`error` rather than filing pre-command frames — the fix for the stale-frame class of bug.

A dial-only step costs zero interactions: read the instruction, turn the dial, see
`✓ DC V, 5 samples` and the next instruction.

## B. Full wire trace and parse diagnostics (planned)

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

## C. Trust tiers (planned)

| Tier | When | Detector | Confirmation | Remote driving |
|---|---|---|---|---|
| 0 Sniff | family with no working parser, or `--sniff` | raw-diff | per state: user names the dial position (Enter accepts the parser's guess) | none |
| 1 Gate | `Stability::Experimental` | raw-diff until the gate passes, then semantic | inline on gate steps; deferred batch review for the rest | after the gate passes |
| 2 Trusted | `Stability::Verified`, or gate passed | semantic | deferred batch review only | yes |

The **gate** is a small block of steps marked `gate: true` in the family's step list: DC V
open, DC V shorted, Ω open (OL), Ω shorted, and a negative reading. Those five establish mode
byte, digits, decimal point, OL and sign. Gate steps confirm inline with the existing one-key
prompt — Enter means the LCD shows exactly this line, otherwise type what it shows. The
decision is Enter-only everywhere, so nothing compares typed digits; a typed correction on a
gate step is a mismatch.

If every gate step is confirmed the report records `core_semantics: confirmed` and the run
switches to tier 2. If any is corrected or skipped the run stays at tier 1 — inline
confirmation, no remote driving — and the report says why.

## D. Autonomous sub-steps through `select` (planned)

For families implementing `choices`/`select` (UT61+, VC-880, VC-890, UT181A), each mode step
is followed, without a prompt, by a walk of Hold on/off, Rel on/off, every MinMax choice,
every Peak choice and the Range ladder, capturing K frames at each and verifying the meter
reports the target before moving on. Sub-steps are filed as `<mode>/hold`,
`<mode>/range:60V` and so on. This replaces the manual `hold`/`hold_off`/… steps and turns
"one range per mode" into "every range per mode" at no coordination cost.

These paths are hardware-unverified on three of the four families, so:

- Only at tier 2, because driving relies on reading mode, range and flags back correctly.
- Fail-soft: `CommandRejected` or "did nothing" records the sub-step's `error` and moves on;
  after a bounded number of failures the sweep disables itself for the rest of the run and
  says so. A meter is never spammed with a command the protocol may have wrong.
- The baseline (auto range, flags off) is restored with a read-back before the next mode
  step; a failed restore is reported so the user can press the button.
- Range sweeps go through `choices(Range)` only, which cycles to target with read-back and
  stops when the read-back stops moving. No blind repeated presses.

`--no-drive` opts out, for receive-only cables (CH9325) or a cautious reporter.

## E. Per-step verification status (planned)

`CaptureStep` gains `verified: Verified | Unverified` (Unverified is the default for new
steps), `gate: bool` (C) and `expect: Option<Expect>` (A). Around them:

- `dmm-cli capture --unverified` runs only unverified steps plus the freeform pass. This is
  the one-line ask in every device verification issue.
- `--list-steps` marks each step ✓/✗ so reporter and maintainer read the same list, and
  `--list-steps --format md` emits the `- [ ]`/`- [x]` checklist the issues already use, so
  the issue and the code cannot drift.
- The epilogue prints coverage: "covered 9 of 14 unverified steps for VC-890; attach to issue
  #14", using `DeviceProfile::feedback_url()`.
- Tests: a Verified family declares no unverified steps, an Experimental family declares at
  least one, every gate step has an `expect`.

`docs/verification-backlog.md` keeps the *why*; verified step ids are struck through and
credited there as today, and the code flag flips in the same commit. Unknowns that are not
modes (VC-890 battery nibble, UT8802 byte 6) already follow the "one step whose typed answer
resolves it" pattern, so the step stays the right unit of verification.

## F. Lower-friction confirmation (planned)

After the run, a numbered table (step, what we read) is printed and the user gives the
indices that were wrong; LCD text is typed only for those. Structured fields replace the
`screen: "confirmed: …"` string: `confirmed: Option<bool>`, `lcd: Option<String>` for the
typed correction, and `confirmed_by: inline | batch`. Reports carrying the old `screen`
string still load, so a resume across versions works.

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
