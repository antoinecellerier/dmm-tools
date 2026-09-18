# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Device auto-detection

Detection identifies the meter from the bytes it sends instead of being
told which family to expect (`crates/dmm-lib/src/detect.rs`; the algorithm
and its failure modes are in `docs/detection-design.md`). What each family
is probed with, and how well that probe is backed:

| Family | Detection sends | Expects back | Hardware status |
|---|---|---|---|
| UT61E+ | `AB CD 03 5F 01 DA` (Get Name) | ack `AB CD 04 FF 00 02 7B`, then an ASCII name frame | **Verified through the detector** 2026-09-11 on our UT61E+ (CP2110): identified in under a second, ack at 85 ms and name at 193 ms in the capture's `init_frames`; with the meter off the cascade ends in 2.7 s with the not-identified help |
| UT61B+ | same | same, the name being `UT61B+` | Verified over CH9329 ([issue #19](https://github.com/antoinecellerier/dmm-tools/issues/19)) |
| UT61D+, UT161B/D/E | same | same, the name being the model | Unverified — no report has named one of these meters |
| UT181A | `AB CD 04 00 05 01 0A 00` (SET_MONITOR) | 2-byte-LE frames, type `0x02`, payload ≥ 31 bytes | The reply is verified on hardware ([PR #8](https://github.com/antoinecellerier/dmm-tools/pull/8), [issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)), never through the detector |
| UT171 | `AB CD 04 00 0A 01 0F 00` (connect) | 2-byte-LE frames, type `0x02`, 16- or 22-byte payload | Deduced from the vendor traces, unverified |
| UT8802 | nothing — the meter streams | two `0xAC` frames exactly 8 bytes apart | Deduced from the vendor traces, unverified |
| UT8803 | nothing — the meter streams | `AB CD` frame, byte 3 `0x02`, 21-byte checksum | Deduced from the vendor traces, unverified |
| UT803, UT804 | nothing beyond the CH9325 init's `0x5A` | any 11-byte CR LF packet, taken as a UT804; a UT803 (19200 baud) is not detected | ~~detection unverified~~ — **VERIFIED** 2026-09-17 by @clazie on a real UT804 (UT-D04 / CH9325): `detect: ut804 identified from 11 received bytes during ut80x stream`, then `connected to UNI-T UT804`. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The UT803 is still undetectable by design |
| VC-880, VC650BT | nothing — the meter streams once PC is pressed; a VC650BT is reported as a VC-880, the protocol being byte-identical | `AB CD` BE16 frame, payload `[0] == 0x01`, 34 bytes | Deduced from the vendor traces, unverified |
| VC-890 | 3× `AB CD 04 FF 00 02 7B`, then `AB CD 03 5E 01 D9` | `AB CD` BE16 frame, payload `[0] == 0x01`, 61 bytes | Deduced from the vendor traces, unverified |

Open questions, each needing a meter:

- **UT181A and UT171 payload lengths overlap** (19 bytes without aux or
  bargraph against 16/22), so a short frame is attributed to whichever
  probe went out last. Splitting them by parse needs UT171 hardware.
- **The UT171 connect frame is UT181A opcode `0x0A`, start recording.** The
  cascade identifies a UT181A with Communication ON in the step before, and
  one with Communication OFF ignores everything — that no UT181A ever starts
  a recording during detection is reasoned, not observed. The ordering only
  covers a meter that answers inside its own ~600 ms window: a reply that
  finishes arriving later is classified in the connect step instead, where
  `0x0A` has already gone out and a short frame reads as a UT171. No UT181A
  reply has been timed through the detector.
- **What `0x5F` (Get Name) does to a VC-880, VC-890, UT171 or UT181A** is
  unknown; step 1 sends it to whatever is on the cable.
- **What SET_MONITOR (`0x05`) does to a UT171** is unknown; it goes out
  before the UT171's own connect frame.
- **The UT61D+ and UT161B/D/E reported names are unverified.** An
  unrecognised name falls back to the UT61E+ tables and is logged, so a
  reporter's `RUST_LOG=dmm_lib=debug` output is what turns one into a
  registry alias.
- ~~**Does a UT61+ beep on `0x5F`?**~~ — **VERIFIED** 2026-09-11 on our
  UT61E+: it does, so every auto connect beeps once, whatever the GUI's
  name-query setting says; a pinned `--device ut61eplus` read stays silent.
- **Does a VC-890 answer `0x5E` on the first attempt?** The vendor software
  retries the name request up to 10 times with a buffer flush between
  attempts, so a single poll may not be enough.
- **Opening after detection runs the family's `init` again**, so a UT181A
  receives SET_MONITOR twice and a UT171 its connect frame twice per auto
  open. Harmless on paper — both are what the meter was already sent — but
  no meter has been watched doing it.

## Pending Verification

### UT61+ remote mode selection (cycle-to-target)

Shipped 2026-09-07: `dmm-cli get mode`/`set mode` and the GUI's mode
dropdown work on the UT61+/UT161 family. The meter takes no set-mode command, so the driver
(`crates/dmm-lib/src/protocol/cycle.rs`) presses SELECT (0x4C) or Hz/%
(0x49) and re-reads the mode byte until the target shows, planning from a
per-model dial table recorded in
`docs/research/ut61-family/reverse-engineered-protocol.md` §3.1. The UT61E+
table, its ring orders and the settle timing were verified on our own
meter the same day, every position and every entry (Completed table below).

Observed 2026-09-07 on our E+, and deliberate: once the meter is in
Hz (0x04), `get mode` lists only `Hz, Duty %` and `set mode "AC V"` is
refused as unknown there. The mode byte carries no dial information, nor does
the rest of an open-lead frame (ut61-family spec §3.1), and the
driver does not guess between dial positions that do not nest, so from Hz it
will not walk back to the position's AC mode. `set mode "%"` still works (one
press), and from Duty % a raw `dmm-cli --device ut61eplus command select2`
press returns the meter to AC V — the next frame reflects it about a second
later.

UT61B+/UT61D+/UT161x owners — [issue #7](https://github.com/antoinecellerier/dmm-tools/issues/7).
Their dial tables come from the manual alone and no press has been observed,
so the listing itself is the thing to check: on each dial position,
`dmm-cli --device ut61b+ get mode` (or `ut61d+`) should name exactly the
functions the meter's own SELECT and Hz/% buttons reach there, and a switch
to each should land. Two specifics: the UT61D+ V≂ position is expected to
carry both AC V and DC V on SELECT, and its temperature position to switch
°C/°F on SELECT; the UT61B+ V~ position is expected to have no SELECT
function at all. A mode listed but unreachable shows up as
`<mode> never appeared; the meter is back in <mode>`.

### UT61+ flag settings (HOLD, REL, MIN/MAX, Peak)

Shipped 2026-09-07: `Setting::Hold`, `Rel`, `MinMax` and `Peak` reach a
named state by pressing the family's own button (0x4A, 0x48, 0x41, 0x4D)
and reading the flag back, leaving MIN/MAX and Peak by 0x42 and 0x4E.

**Verified the same day on our UT61E+**, leads open, V⎓ and V~ dial
positions, `RUST_LOG=dmm_lib=debug`:

- `set hold on` then `set hold off`: one press each, each confirmed on the
  next frame. A repeated `set hold on` presses nothing and answers "Meter is
  already HOLD on".
- `set rel on` then `set rel off`: one press each.
- `set minmax max` (from off) and `set minmax min` (from MAX): one press
  each. `set minmax off` logs `cycle: leaving minmax (in MIN)` and leaves by
  the 0x42 exit command, not by a press.
- On V~: `set peak p-max` and `set peak p-min`, one press each;
  `set peak off` logs `cycle: leaving peak (in P-MIN)` and leaves by 0x4E.
- Every press is answered with a 2-byte `[FF, 00]` ack frame, which the
  parser skips. The first measurement frame after a press can still carry
  the old state; the walk then logs `cycle: meter still reports <state>,
  re-reading` and re-reads rather than pressing again (seen once each on
  `set peak p-min` and `set peak off`). Without that it would overshoot.

Partly settled — **which modes Peak is offered in.** Device evidence now
covers AC V (2026-09-07: the meter entered and left P-MAX/P-MIN) on top of
AC mV, and DC V is still the one mode confirmed not to react (2026-03-21,
"MIN/MAX and Peak measurement reporting" below). A `get` on 2026-09-07 also
showed the Peak row offered in AC V and AC A and absent in DC V, DC A and
DC mV — but that listing is the code's own table (`AC_PEAK_MODES` in
`tables/mod.rs`, the five pure-AC modes), so it is a cross-check, not
evidence about the meter. Left to check on our meter: AC µA and
AC mA, and AC+DC V and LPF V, which the code refuses with `peak cannot be
set in <mode> on this meter`. Every mode where the meter reacts but the
list is empty (or the reverse) is a table fix.

The UT61B+ is offered no Peak at all, from the family spec's flag matrix
(§4, blank Peak cells) and command matrix (§6, "No effect"); the UT61D+ is
offered the same AC modes as the E+. Both unverified — issue #7.

Where the buttons do nothing — **VERIFIED** on our UT61E+ (2026-09-07, four
runs) and again by @ChrisTheExpie on a real UT61B+ (2026-09-10, CH9329,
issue #19), each sweep ending in `<button> did nothing`.
The two meters refused the same commands in the same modes:

- **REL:** continuity, Hz, Duty %, NCV and AC+DC V.
- **MIN/MAX:** continuity, diode, capacitance, Hz, Duty % and NCV.
- **HOLD:** NCV.
- **RANGE:** capacitance and Hz. Both have multi-rung tables and auto-ranging
  reaches those rungs; it is only the button that does nothing.

`choices()` was narrowed to match on 2026-09-10: `HOLD_DEAD`, `REL_DEAD` and
`MINMAX_DEAD` in `ut61eplus/mod.rs`, `FAMILY_FIXED_RANGE_MODES` in
`tables/mod.rs`, and spec §6.2. The manual (§VII) gives each button one line
and no per-function list, so the meters are the only record.

**The bar for that list: a refusal reproduced with a real reading on screen,
on every meter that can be asked.** Both halves earn their place:

- **A refusal of REL over OL says nothing about the mode.** This meter refuses
  REL whenever the display reads OL, whatever the mode — `dcmv/rel:on` was
  refused over OL in the 2026-03 run and taken in all three later runs where
  DC mV had a value. **HOLD and MIN/MAX are not affected**: diode's HOLD
  frames carry the flag over OL, and `dcmv/minmax` was taken twice over OL.
  Tallying every REL and MIN/MAX sub-step by what was on screen, `dcmv` + REL
  is the only case in any capture where the outcome differs by screen state.
- ~~**Diode + REL, asked with a diode fitted on a second meter**~~ —
  **VERIFIED** 2026-09-11 by @ChrisTheExpie on a real UT61B+ (CH9329). Diode
  is settled for both buttons on both meters that have the mode, each asked
  with a diode fitted rather than over OL — open leads there read OL, and the
  meter refuses REL over OL whatever the mode. Our UT61E+ on 2026-09-10 with a
  Schottky at 0.1968 V (`ut61eplus-diode.yaml`): REL and MIN/MAX refused, HOLD
  taken. The B+'s `capture --steps diode` run at 0.515 V: `diode/rel:on`
  refused (`REL did nothing in off`) with the flag nibble unmoved across all
  three settle reads, `diode/hold:on` taken. REL decoding is proven on that
  meter by `dcv_ranges/rel:on` the same day, so the refusal is the meter's and
  not a parse miss. Diode is now in `REL_DEAD` as well as `MINMAX_DEAD`.
- **AC+DC V joins the REL list**, on three refusals with a real reading:
  2026-09-07 at 0.07 V and 0.08 V, and 2026-09-10 at 0.0005-0.0175 V
  (`ut61eplus-acdcv.yaml`). That is every meter that has the mode — the
  UT61B+ has no such dial position, so three runs on the only meter with it is
  the ceiling, not a shortfall. HOLD and MIN/MAX both work there, so it is REL
  specifically: `acdcv/minmax:min` read back 0.0652 V with the MIN flag set.

Narrowing `choices()` reaches `dmm-cli get`/`set` and the capture sweep, and
nothing else. The GUI's HOLD/REL/MIN-MAX/PEAK buttons come from
`supported_commands` and press through `send_command`, as does
`dmm-cli command <name>`, so a wrong entry here cannot stop anyone pressing
the button — but it can stop `set` reaching a state the meter does have.

**AUTO in LPF V** is a separate E+-only observation: the meter came up in
1000V manual and stayed there, although the manual's AC V table lists LPF on
every range. Not encoded.

The mock follows the same matrix from 2026-09-10, since a mock that offers a
control the meter ignores is the false confidence `.claude/rules/protocol.md`
warns about. From 2026-09-14 it also meets HOLD as the E+ does: REL and
Hz/% dropped, SELECT, RANGE and AUTO releasing it (ut61-family spec §6.3). One divergence is left and predates this: the mock offers Peak in
Hz, Ω, capacitance, temperature and NCV, where the E+ offers it only in the
five pure-AC modes (`AC_PEAK_MODES`). `Scenario::peak_applies` is what would
narrow it.

The sweep skips REL while the reading is OL, so some rows read as absent
rather than refused in a given run: `continuity/rel:on` was only attempted in
the runs where the probes were still touching (2026-03 and
`ut61eplus-verify4.yaml`), and both of those refused it.

### UT61E+ — range ladders walked end to end (2026-09-10)

`capture --unverified` on our UT61E+ ran the two steps added that day
(`ohm_ranges`, `dcv_ranges`), and a `--steps acdcv` run walked a third ladder.
Report: `ut61eplus-ladders.yaml`, `ut61eplus-acdcv.yaml`.

- **Every rung of Ω, DC V and AC+DC V is [VERIFIED]**, one rung per RANGE
  press, ascending, with no press skipped or repeated: Ω 0-6 (220Ω, 2.2kΩ,
  22kΩ, 220kΩ, 2.2MΩ, 22MΩ, 220MΩ) and DC V and AC+DC V 0-3 (2.2V, 22V, 220V,
  1000V). Each rung is identified by the decimal count the meter sent there,
  which on a 22,000-count display names the full scale outright. AC+DC V
  shares the DC V table, as `ut61e_plus.rs` has it. The probes were shorted
  throughout, so every rung read zero: the decimal placement is verified, the
  decoding of a non-zero value at each rung is not. A resistor of 1k-100k
  across the probes would settle that in one run — it lands inside five of the
  seven rungs and each must decode to the same resistance.
- **The AC V ladder was walked earlier, on 2026-09-07** (`acv/range:*` in
  `ut61eplus-verify5.yaml`), which this section originally left out: all four
  rungs are golden fixtures, and their decimal counts run 4, 3, 2, 1 across
  indices 0-3 — `  0.0647`, `   0.395`, `    0.35`, `     0.0` — matching the
  manual's 0.1mV/1mV/10mV/0.1V resolutions for 2.2000V, 22.000V, 220.00V and
  1000.0V. AC V does *not* share the DC V table in the code, and does not
  need to.
- That closes the 2026-09-07 worry that RANGE could not be swept: the blind
  six-press sweep that produced indices 0, 2, 0, 0, 0, 0 was the old code
  pressing without reading back. `choices(Range)` walks to target with a
  read-back per press and gets every rung.
- HOLD, REL, MIN and MAX were all taken in Ω, DC V and AC+DC V except REL in
  AC+DC V (above).
- Both steps were marked verified **for the UT61E+ only** until 2026-09-11,
  when the UT61B+ walked them too (issue #19). A verified model that has not
  run a step must not be marked verified for it by the family-wide `hw` flag,
  which is what kept `capture --unverified` asking the B+ for exactly these
  two; with both verified models through every step, it asks neither for
  anything and the per-model flag is gone.
- **A resistor across the probes confirms the decode across rungs, and all
  three overload spellings at once** (`ut61eplus-ohm-82k.yaml`, 82 kΩ marked,
  reading 80.45 kΩ on auto). The three rungs it overflows produced
  `  OL.  ` at 220Ω, ` .OL   ` at 2.2kΩ and `  O.L  ` at 22kΩ — the whole of
  §5.8 in one run on one meter, rather than inferred across seven captures.
  Re-run with `--settle 3000` (`ut61eplus-ohm-82k-settled.yaml`), every rung
  it fits decodes to the same resistance: `80.46` kΩ at 220kΩ, `0.0804` MΩ at
  2.2MΩ, `0.08` MΩ at 220MΩ, against `80.45` kΩ on auto. Different unit,
  different decimal count, one resistor — which is the check shorted probes
  could not make, since every rung then reads zero.
- **The top Ω rungs settle slowly.** With the probes shorted, 22MΩ read
  0.081 MΩ (81 counts) and 220MΩ read 0.18 MΩ (18 counts), and both were seen
  on the meter to drop back over several seconds. The sweep samples about
  200 ms after the press, so **range sub-step values on slow-settling ranges
  are transients, not measurements**. It does not touch the rung-to-label
  mapping, which is the decimal placement, nor the golden fixtures, which
  assert the parse of a frame whatever it held. With the 82 kΩ resistor the
  scale of it showed: 220MΩ filed 4.38 MΩ, fifty times the true value, and
  220kΩ filed two OL frames before the reading came down into range.
  `capture --settle MS` was added for this; waiting for the reading to hold
  still instead would never finish on leads with nothing stable across them.
- **A delay after the press cannot cover HOLD, REL or MIN/MAX**, which act on
  the live reading rather than on the meter's next one. `ohm_ranges/hold:on`
  filed 12.59 kΩ three times on the 82 kΩ resistor
  (`ut61eplus-ohm-82k-settled-2.yaml`, 2026-09-10): the press followed the
  220MΩ rung handing back to auto, so the meter froze a reading still on its
  way down and every later sample read the frozen value. Settling **before**
  the press is what would fix it — the wait is on the wrong side of the button
  for these three.
- **Golden fixtures**: 20 in `crates/dmm-lib/tests/golden/ut61eplus/` from these
  runs (2026-09-11) — the 82 kΩ resistor at the four rungs it fits and the three
  overload shapes at the rungs it does not, the AC+DC V ladder above 2.2V, HOLD, REL
  and MIN/MAX in Ω and AC+DC V, the Schottky diode with and without HOLD, and a
  `- 0.000` frame. The hand-built `ohm_overload` fixture, whose `OL` carried no
  decimal point, is replaced by the real 220Ω frame.

### Capture leaves the meter manually ranged

The sweep restores each setting it drove, and `docs/capture-design.md` says
the baseline is auto range with the flags off. It is not, at the end of a run:
the range walk restores Auto, then the MIN/MAX walk locks the range again (as
the meter does while recording), and leaving MIN/MAX by 0x42 does not put
auto-ranging back. Seen 2026-09-10 — one `ohm_ranges` run ended manual, and
the next run opened on a manually ranged meter, which cost it the `22MΩ` rung
because that rung was then the current choice and the sweep only walks the
others.

Not harmful — the operator's next dial turn clears it — but it makes a
resumed or repeated run cover a different set of rungs than a fresh one.
Re-asserting Auto after the flag sweeps would fix it.

### UT61B+ — hardware reports

Four UT61B+ captures by @ChrisTheExpie in
[issue #19](https://github.com/antoinecellerier/dmm-tools/issues/19), all over
a CH9329 cable, are the family's device evidence beyond our own UT61E+:

- **2026-09-09, v0.6.0** — parsed samples, no wire frames, with the reporter
  confirming the LCD beside them.
- **2026-09-10, v0.7.0-dev (6406037)** — wire frames throughout, driven range
  and flag sweeps, and a gate that passed outright
  (`core_semantics: confirmed`).
- **2026-09-11, v0.7.0-dev (88e80ed)** — a `capture --unverified` run, driven,
  trusted tier, walking the two ladder steps the model was still asked for,
  plus three freeform extras (a diode, a capacitor, live mains AC V).
- **2026-09-11 later, v0.7.0-dev (88e80ed)** — `capture --steps diode` with a
  diode fitted, answering the REL ask, alongside a `debug --count 5` terminal
  capture on the manually set 600Ω rung answering the other.

Together they carried the model to `Stability::Verified`: every mode its dial
reaches was captured and decoded correctly, and every command moved the flag it
should. The family issue stays linked on it for the rungs below.

Settled:

- The meter names itself `UT61B+` (GetName 0x5F) and speaks the UT61E+'s
  protocol unchanged — AB CD framing, BE16 checksum, 14-byte payload, mode
  bytes 0x00/02/03/04/05/06/07/08/09/0C/0E/10/14, and all three flag nibbles.
  The 2026-09-10 run recorded the handshake itself: `AB CD 03 5F 01 DA` out,
  `AB CD 04 FF 00 02 7B` and `AB CD 08 55 54 36 31 42 2B 02 FD` back. First
  UT61+ family run on the CH9329 bridge.
- HOLD (0x4A), REL (0x48), MIN/MAX (0x41), ExitMinMax (0x42), RANGE (0x46)
  and AUTO (0x47) each moved the expected flag on the next frame, and MIN/MAX
  cycles MAX then MIN as on the E+.
- Ascending range-index order, and these rungs are now [VERIFIED] rather than
  deduced — each identified by the decimal the meter lit there: **every Ω rung
  0-5** and **every DC V rung 0-3**, walked with RANGE on 2026-09-11 one rung
  per press and read back from the overload shape (`   OL. ` at 600Ω and
  600kΩ, `  .OL  ` at 6kΩ and 6MΩ, `   O.L ` at 60kΩ and 60MΩ) or, at 0 V,
  from the decimal count (`  0.001`, `   0.00`, `    0.0`, `     0 ` for 6V,
  60V, 600V and 1000V); **AC V 0** (6V) and **AC V 2** (600V, 236.6 V of mains
  with the HV warning lit — the first HV-warning frame from a B+);
  **capacitance 5** (6mF, a real capacitor at 4.514 mF, LCD-confirmed);
  **µA 0 and 1** (600µA, 6000µA), **mA 0 and 1** (60mA, 600mA), **A 0 and 1**
  (6A, 10A — the B+ tops out where the E+ has 20A).
- **Diode reads 0.515-0.516 V with a diode fitted** (2026-09-11,
  LCD-confirmed) — the model's first finite diode frames; the earlier one is
  OL. The later run of that day drove the step: HOLD taken, REL refused.
- HOLD, MIN and MAX were taken in Ω over OL, and HOLD, REL, MIN and MAX in
  DC V at 0 V, with the same nibbles as on the E+ (2026-09-11).
- Bar graph full scale is 30 across modes (`-9.33` on 60V → 4, `11.72` on
  60mV → 5, Ω O.L → 30), matching the manual's 31 segments for 6,000-count
  models.
- **DC V/AC V range 0 is 6V, not 60mV** — fixed the same day in
  `ut61b_plus.rs`, and the same shape applied to `ut61d_plus.rs` as
  [DEDUCED]. Every voltage read a thousandth low because the deduced table
  put the mV ranges at indices 0-1. The E+ needed the same correction in
  March; there the spare entry sat at index 4, where the meter never lands.
  The 2026-09-10 run confirms the fix on the meter.
- ~~**NCV levels.**~~ — **VERIFIED** 2026-09-10 by @ChrisTheExpie on a real
  UT61B+ (CH9329). The dash count is the level: the 2026-09-09 frame
  `14 30 20 20 20 2D 2D 2D 2D 00 00 30 34 30` is four dashes and the reporter
  confirmed all four bars on the LCD, and the 2026-09-10 run sent one dash
  next to a weaker field. That report printed `NCV:0` for the four-dash frame
  only because dash counting (5daf64a) landed after v0.6.0. Both frames are
  golden fixtures. The B+'s no-field display is still unseen — the E+ idles at
  `EF`.
- **Duty % is reached from the Hz/% dial position with the USB short-press**,
  as `ut61b_plus.rs`'s Hz/% ring (0x49) says. The reporter's first run used
  the button from V~ instead, which is why the earlier note read SELECT.
- **Golden fixtures**: 46 in `crates/dmm-lib/tests/golden/ut61b+/`, lifted
  from all four reports.

Left open on this model:

- **AC V rungs 1 (60V) and 3 (1000V)** — **not a hardware ask**. The ladder
  has four entries in ascending order, which this meter has shown across
  every Ω and DC V rung and both current ladders; rung 0 is pinned by
  `  0.589` (three decimals, 6.000V full scale) and rung 2 by 236.6 V of
  mains. Rungs 1 and 3 have nowhere else to sit, and every AC V rung carries
  unit `V`, so a wrong label here cannot change a reading — only the
  `range_label` string. Rung 3's value came from the manual on 2026-09-12
  (below); no safe capture can confirm it. A run would still be welcome for
  its own sake: `acv` is not a gate step, so the sweep walks its ladder as it
  does any other mode step's — it went unswept on 2026-09-10 only because it
  then sat before the gate closed (below), and the two runs since were
  `--unverified` and `--steps`, neither of which reaches a step already
  marked verified. `capture --steps acv` with the leads open covers it.
- **Capacitance rungs 1-4 and 6** — pinned by arithmetic, but worth a real
  measurement if capacitors turn up, because it is one of the two open
  ladders whose units change mid-way (nF/µF/mF; the Hz ladder is the other,
  and Ω is the third such ladder but fully verified), and `unit` comes
  straight from the
  range table (`ut61eplus/mod.rs:866`), so a wrong rung here is a 1000x
  error rather than a wrong label. Rung 0 is pinned by `   0.03` (60.00 nF)
  and rung 5 by `  4.514` (6.000 mF, a capacitor on 2026-09-11); with both
  ends of a seven-entry decade ladder fixed and the order ascending, 1-4 are
  the four manual rungs between them and 6 the one above. Reaching them for
  real needs six capacitors, one per decade — RANGE is dead in capacitance
  (below), so auto-ranging is the only way there.
- **The mV ladder** — rung 0 is *measured*, not deduced: `   1.43` in
  `dcmv.yaml` and `  11.72` in `acmv.yaml`, two decimals each, which is
  60.00 mV full scale. With two ascending entries that leaves rung 1 at
  600mV, and both carry unit `mV`. What is genuinely untested is whether
  RANGE walks them at all (next item).
- ~~**The 600Ω manual rung.**~~ — the rung is stable; what is left is a
  question about the press path, not the range table. The 2026-09-11 walk
  pressed RANGE four times from 600kΩ auto and read back 3 (manual), 4, 5, 0 —
  then, with no further press, the next poll 150 ms later reported rung 1
  (6kΩ), still flagged manual, and all three samples of the step sat there:
  five transitions for four presses, and the tool filed the step
  `needs_attention`. **VERIFIED** 2026-09-11 by @ChrisTheExpie that the meter
  does not leave the rung on its own: with 600Ω set by the meter's own RANGE
  button and the leads open, `debug --count 5` returned five consecutive
  frames of `06 30 20 20 20 4F 4C 2E 20 03 00 30 34 30` — rung 0, manual
  (flag2 bit 2 set), the `   OL. ` shape — byte-identical to the one frame the
  sweep caught there. It had not dropped off at entry time either, or it would
  have been on 6kΩ before `debug` ran. That matches the E+, which holds its
  220Ω rung over OL (the 82 kΩ run, 2026-09-10). The remaining candidate is
  the fourth 0x46 landing twice past the tool: our tx log has exactly four
  presses, so a duplicate would be in the bridge or the meter's own handling.
  One observation, on one cable; the check is whether a RANGE walk on our own
  meter over CH9329 ever gains a rung it did not press for.
- **The three-member Hz/% rings, and the mV SELECT leg.** Six ring legs are
  [VERIFIED] on this model: the 2026-09-10 run had the *driver* press through
  them, one press each, every one reaching its target — SELECT for
  Ω → Continuity (step `continuity`), Diode → Capacitance (`capacitance`) and
  DC → AC on µA, mA and A (`acua`, `acma`, `aca`), plus Hz/% for
  Hz → Duty % (`duty`). All six are two-member rings, where order is trivial.
  The V~ ring is [VERIFIED] too, in the table's order: issue #20's
  `hz-walk.yaml` run (2026-09-14) pressed Hz/% once per step, AC V → Hz →
  Duty % → AC V, each press reaching the next mode.
  What is left is the mV position's SELECT leg (DC mV ↔ AC mV — the operator
  pressed that one, so `acmv` carries no press of ours) and the other four
  three-member Hz/% rings, AC mV/AC µA/AC mA/AC A → Hz → Duty %, which
  are the only rings on the model where the order could differ from the
  table. Nothing rests on the order — `cycle.rs` presses and reads the mode
  back until the target shows, so a wrong order costs at most an extra press
  — but wrong *contents* could leave `set mode` unable to reach a mode. A
  `dmm-cli --device ut61b+ set mode duty` from each of those AC modes answers
  the rest. Issue #7, now that issue #19 is closed.
- **Hz → AC V and Hz → AC mV switches time out (issue #20, 88e80ed, GUI).**
  The walk is two Hz/% presses in a row; per the reporter's recording the
  meter takes the first (Hz → Duty %) and stops there, and the GUI reports a
  timeout. Our UT61E+ (CP2110) does the same walk without error. The
  reporter's `hz-walk.yaml` capture (7b48053, 2026-09-14, leads open) caught
  a timeout of the same shape, on a HOLD press in Hz (`hz_tool/hold:on`): the
  driver polled 202 ms after the press, the press's ack came 14 ms after that
  poll, and the poll was never answered — 2 s, then the error, with HOLD lit
  on the next frame. In open-lead Hz this meter acks 216–217 ms after a
  press, past the driver's 200 ms read-back. Across every capture on record a
  poll went out ahead of its ack 12 times on this model (this one lost, the
  closest; the rest 65–165 ms ahead and answered) and 55 times on our E+
  (26–214 ms ahead, all answered): one loss, not a rule, and the bytes cannot
  say whether the meter or the CH9329 dropped it. The GUI walk's first press
  is Hz/% in Hz with the same timing, which fits the recording — the press
  lands on Duty % and the read after it times out. Nothing supports the
  other reading, a second press ignored: of the run's 22 presses only Hz/%
  under HOLD changed nothing. The capture's own Hz → AC V switch never ran
  (next item). Whether the #20 run had open leads was never said. A press
  now waits for the meter's ack (up to 1 s) before anything is polled, and
  a read-back that times out is read again, never pressed again. Both are
  unverified on this model: a re-run of the same `hz-walk.yaml` answers them.
  Our UT61E+ ran that plan clean with both on 2026-09-14: none of its 27
  presses was followed by a poll before the ack (acks 63–382 ms), and the
  Hz → Duty % → AC V switch took its two presses.
- **Buttons under HOLD (issue #20).** The reporter found the meter's buttons
  ignored while HOLD is lit on the V~ position (2026-09-14, by hand),
  apparently during the `hz-walk.yaml` run above: the timed-out HOLD step
  left HOLD on, because the capture sweep skips its restore after any error.
  The next step's Hz/% press in Hz was acked and the meter stayed in Hz with
  HOLD lit; ~12 s later the frames show HOLD released and Hz/% pressed twice
  by hand. So this model drops Hz/% under HOLD on the wire too. Our
  UT61E+ ignores Hz/% the same way, and takes SELECT, RANGE and AUTO, each of
  which clears HOLD (ut61-family spec §6.3). RANGE and AUTO under HOLD are
  unasked on the B+. A mode or range walk now presses HOLD off and sends a
  press that changed nothing under HOLD again; unverified on the B+, where
  `set hold on` then `set mode Hz` from AC V exercises it.
- **The Hz ladder — PARKED 2026-09-12, needs a signal generator we do not
  have.** Raise it again if one turns up, or if a reporter offers. This is
  the only open item on the model with a real correctness risk: the Hz rungs
  are Hz, Hz, kHz, kHz, kHz, and `unit` comes from the range table, so if the
  range byte really does stay at 0 above 6 kHz then everything up there is
  labelled Hz while the meter shows kHz. Range index 0 carried two different
  full scales across the two runs: `0.0` (one decimal) from the V~ Hz path on
  2026-09-09 and `0.00` / `49.98` (two decimals) from the Hz/% dial position
  on 2026-09-10. One index cannot mean both, so either the index is pinned at
  0 in Hz and the rung shows only in the decimal placement, or the two paths
  range differently. The code's five invented Hz ranges describe neither, and
  the E+'s five are invented too, so no sibling can arbitrate. Settling it
  needs one frame on a signal above 60 Hz — say a 1 kHz square wave — to see
  whether the range byte ever leaves 0.

### Capture: gate steps placed after other steps

The capture run drives the meter's own settings only once every gate step has
reported, and it never sweeps a gate step itself. A family that scatters its
six gate steps through its list therefore has every step *before* the last of
them go unswept — no range ladder, no flag sub-steps. The UT61+ list did, and
lost the AC V, DC mV and AC mV ladders in the 2026-09-10 UT61B+ run (issue
#19) before being reordered; `ohm_ranges` and `dcv_ranges` were added so the
two ladders the gate steps sit on get swept as well.

Still split, each needing that family's own dial order and a hardware run:

- **UT181A** — `vdc_acdc`, `vdc_peak`, `vac`, `vac_hz`, `vac_lpf`, `vac_dbv`,
  `vac_dbm`, `mvdc`, `mvdc_peak`, `mvac`, `mvac_hz`, `mvac_peak`, `mvac_acdc`.
  Issue #5.
- **VC-880 / VC650BT / VC-890** — `acv`, `acdcv`, `dcmv`, `dcua`, `acua`,
  `dcma`, `acma`, `dca`, `aca`.

The UT8802, UT8803, UT803, UT804 and UT171 lists are split too, but those
families declare no `choices`, so nothing would be swept whatever the order.
The allow-list in `every_device_finishes_its_gate_before_any_other_step`
(`crates/dmm-cli/src/capture/step.rs`) names all of them; deleting an entry is
how a fix lands.

### Modes not yet tested with real signals

Tracked in [issue #6](https://github.com/antoinecellerier/dmm-tools/issues/6).

- **DC mV (0x03):** Mode byte verified on the mV dial. Needs small DC voltage source for value verification.
- **AC µA (0x0D):** Mode byte verified via SELECT on µA dial. Needs AC current source for value verification.
- **AC mA (0x0F):** Mode byte verified via SELECT on mA dial. Needs AC current source for value verification.
- **AC A (0x11):** Mode byte verified via SELECT on A⎓ dial. Needs high-current AC for value verification.
- **Temperature °C (0x0A):** Needs K-type thermocouple.
- **Temperature °F (0x0B):** Needs K-type thermocouple.
- **Duty Cycle % (0x05):** Mode byte verified via SELECT2 on AC mA. Needs PWM signal for value verification.
- **LPF mV (0x1A), LPF A (0x1C):** Need appropriate signals and dial positions.
- **AC+DC mV (0x1B), AC+DC A (0x1D):** Need appropriate signals and dial positions.
- **Live (0x13):** Unknown purpose.
- **Inrush (0x1E):** Inrush current mode.

### Modes not reachable on UT61E+

Tracked in [issue #7](https://github.com/antoinecellerier/dmm-tools/issues/7) — needs UT61D+ or UT61B+ hardware.

These modes exist in the vendor software but could not be reached on the
UT61E+ via any dial position + SELECT/SELECT2 combination. They are likely
UT61D+-only or other-model features. Verified 2026-03-19 by exhaustively
cycling SELECT and SELECT2 on V~, V=, mA, and A⎓ dial positions.
- **LoZ V (0x15):** Low impedance ACV (UT61D+ feature).
- **0x16 (LoZ V 2):** Vendor software names it "LozV". Not reachable on UT61E+.
- **0x17 (LPF):** Vendor software names it "LPF". Not reachable on UT61E+.

### Experimental protocol families (no real hardware access)

These protocols are implemented based on reverse engineering (vendor software
decompilation, community implementations) but have **never been tested against
real hardware**. Every aspect needs end-to-end verification.

The ask in every family's issue (#3, #4, #5, #7, #12, #13, #14, #15, #16) is
the same: `dmm-cli --device <id> capture --unverified`, attach the report.
The issue's checklist is `dmm-cli --device <id> capture --list-steps
--format md`, so a step a report confirms flips `.verified()` in code, is
credited here in the same commit, and the checklist is regenerated into the
issue. The items below are the wire-level questions those steps answer,
plus what no step reaches.

**Voltcraft VC-890**:
- Polled communication model (0x5E request → live data response)
- Frame extraction (66-byte, AB CD header, BE16 checksum)
- Function code mapping (19 codes, 0x00-0x12, remapped from VC-880!)
- 60,000 count range values (6/60/600 vs 4/40/400)
- 7 display value fields (main + 6 sub-displays) — format and content
- Status flag bytes (8 bytes at msg[56..63]) — all bit positions correct?
  Do bytes 56-61 carry a 0x30 prefix like the range byte? The parser
  reports their bits 6-7 as unrecognised and leaves bits 4-5 out until a
  capture settles this (noted 2026-09-17)
- Sign1 (msg[56] bit 2) — the value's sign is taken from the display text;
  the parser reports Sign1 set on a display without a `-` (noted 2026-09-17)
- AVG flag (byte 57 bit 1) — parsed since 2026-09-07 and reported wherever
  flags are shown; unverified on hardware. Put the meter in MAX/MIN/AVG and
  confirm AVG lights only on the AVG step of the cycle.
- Battery level nibble (msg[62]) — what do the values mean?
- Misplug warning nibble (msg[63]) — 0=none, 1=mA err, 2=A err, 3=V err
- ACV LPF (0x01) range byte — vendor ignores it and fixes 1000V (2026-06
  review, DMSShare_decompiled.cs:23466). On hardware (2026-09-07) LPF V
  reported range byte 1000V with AUTO off and refused AUTO; whether any
  other range is reachable with a signal applied is open.
- AC+DC V (0x19) alternates frames between the DC and AC components with
  flag byte 3 bit 0x08 toggling (spec §2.7). Which component the set bit
  marks needs a known source: a battery plus a mains-hum pickup would tell.
- Inbound checksum — the vendor never validates meter→host checksums; our
  BE16 check is inferred from the host-side builder. If real frames are
  all rejected with ChecksumMismatch, suspect a different inbound scheme.
- Command confirmation frames — vendor `SendCommand` waits for a frame
  whose type byte equals the command byte (5 retries); we fire-and-forget.
- Ack protocol (0xFF+\[0x00\] after responses) — is it required or optional?
- GetDeviceID retries — the vendor loops up to 10 times with a `FlushBuffer`
  between attempts (`DMSShare_decompiled.cs:3895`); we make a single attempt.
  Does one attempt reliably return the name on real hardware?
- Battery nibble ground truth — every capture report already carries the raw
  byte in `raw_hex`, but nothing records what the meter's own battery
  indicator showed at the time, so the values stay uninterpretable. The
  `battery` capture step now asks for that; a report from a meter with a
  fresh pack *and* one the meter flags as low would settle whether `0` means
  empty or "not populated" (which is what our `low_battery` currently
  assumes).
- Commands: same as VC-880 plus 0x5D (Set Time) and 0x5E (Get Measurement)
- PC button activation requirement
- Dial table and SHIFT/SETUP mode switching — implemented 2026-09-07 as
  `dmm-cli get mode`/`set mode` (and the GUI's mode dropdown) over the
  [MANUAL] dial table in the spec's "Rotary positions" section. Nothing in it is
  hardware-confirmed: the manual says which symbol each position offers,
  never the order the presses walk them. Runnable checks, one dial position
  at a time:
  - `dmm-cli --device vc890 get mode` on every position — the listing should
    name exactly the functions that position offers, `*` on the live one.
    The capacitance position offers one function, so it prints "no
    switchable modes" instead of a list. Report the dial symbol and the
    list whenever they disagree
  - switch to every entry the listing offers, with
    `RUST_LOG=dmm_lib=debug dmm-cli --device vc890 set mode "<label>"`, and
    paste the log: one `cycle: pressing SHIFT/SETUP (in X, want Y)` line per
    press, so it records both the press count and the order the function
    codes actually came round in
  - the raw cycle, independent of our table: `dmm-cli --device vc890 command
    select` followed by `dmm-cli --device vc890 read --count 3`, repeated
    until the display returns to where it started, once per position
  - V~ should carry the low-pass filter (0x01) as its sub-function here,
    where the VC-880 gives Lo a dial position of its own — confirm on the
    meter, since the two families' dials otherwise match
  - the settle constants are untuned guesses (no delay, 2 reads for a press
    to show up). `the meter refused …: SHIFT/SETUP did nothing in <mode>`
    while the display *did* change means they are too tight — report the
    mode and how long the meter takes to answer
- `Setting::Range` — implemented 2026-09-07 the same way, pressing RANGE
  (0x46) and re-reading the range byte, with 0x47 for auto. Unverified:
  nobody has confirmed that repeated 0x46 steps the ladder one rung at a
  time on this meter. Runnable check, on a dial position with a stable
  input applied: `dmm-cli --device vc890 get range` should name the rungs
  that function offers, `*` on the live one — report any rung the meter's
  own RANGE button reaches that the listing leaves out. Then
  `RUST_LOG=dmm_lib=debug dmm-cli --device vc890 set range <label>` for each
  of them, and `set range auto` to finish. Paste the
  `cycle: pressing RANGE (in X, want Y)` lines: the press count per rung is
  what says whether 0x46 steps one at a time.
  `<label> never appeared; the meter is back in <label>` means it does not,
  and `the mode changed to <mode>; stopped pressing RANGE` means 0x46 moves
  the function byte too
- `Setting::Hold`, `Rel` and `MinMax` — implemented 2026-09-07 by pressing
  0x4A, 0x48 and 0x49 and reading the flag back, with 0x43 to leave
  MAX/MIN/AVG. MIN/MAX is offered as off/MAX/MIN/AVG. Unverified: the order
  0x49 walks those three in (the driver re-reads after every press, so any
  order works, but a state the meter never lights shows up as
  `<state> never appeared; the meter is back in <state>`), and whether
  every mode accepts HOLD and REL. Peak is not offered — the vendor command
  table lists no peak command. Runnable check:
  `dmm-cli --device vc890 set minmax max`, then `set minmax min`, then
  `set minmax avg`, then `set minmax off`, each with the badge the LCD
  shows; `set minmax avg` is also the hardware check the AVG flag item
  above wants, since it only succeeds if byte 31 bit 1 is read back. Then
  `set hold on` / `set hold off` and `set rel on` / `set rel off` on two or
  three dial positions. Run them under `RUST_LOG=dmm_lib=debug` and paste
  the `cycle:` lines. Also unknown: which buttons a held meter drops. A
  mode or range walk presses HOLD off and sends a dropped press again;
  `set hold on` then `set mode` or `set range` exercises it
- `dmm-cli --device vc890 capture` exercises all of the above on its own
  since 2026-09-07: once the gate steps pass, every mode step is followed by
  `set range`/`hold`/`rel`/`minmax` through each value, filed as
  `<mode>/<setting>:<label>` sub-steps carrying what the meter read back.
  Report any sub-step with `status: error` and the
  `remote control unreliable on this meter` line if it appears — those are
  the commands this meter refused

**Voltcraft VC-880 / VC650BT**:
- Frame extraction (39-byte, AB CD header, BE16 checksum — same as UT61E+)
- Streaming model (no trigger, auto-starts after PC button press)
- Function code mapping (19 codes, 0x00-0x12) — do mode labels match LCD?
- Range byte (0x30-based ASCII) — correct range values per function?
- Main display (7 ASCII bytes) — values match LCD?
- Sub-displays (sub1, sub2, bar) — format and content
- Status flag bytes (7 bytes, 28 named flags) — all bit positions correct?
  Do bytes 30-35 carry a 0x30 prefix like the range byte? The parser
  reports bits 6-7 of bytes 30-36 as unrecognised and leaves bits 4-5 of
  bytes 30-35 out until a capture settles this (noted 2026-09-17)
- Sign1 (msg[30] bit 2) — the value's sign is taken from the display text;
  the parser reports Sign1 set on a display without a `-` (noted 2026-09-17)
- AVG flag (byte 31 bit 1) — parsed since 2026-09-07 and reported wherever
  flags are shown; unverified on hardware. Press MAX/MIN/AVG (0x49) through
  the cycle and confirm AVG lights only on the AVG step.
- Overload detection (OL1 flag + "OL" in display string)
- Commands: hold (0x4A), rel (0x48), range_auto (0x47), range_manual (0x46),
  max_min_avg (0x49), light (0x4B), select (0x4C)
- Streaming rate (manual says 2-3 Hz)
- PC button activation requirement
- VC650BT compatibility (same protocol confirmed by installer comparison)
- Dial table and SHIFT/SETUP mode switching — implemented 2026-09-07 as
  `dmm-cli get mode`/`set mode` (and the GUI's mode dropdown) over the
  [MANUAL] dial table in spec §4.4. Nothing in it is hardware-confirmed: the manual says which
  symbol each position offers, never the order the presses walk them, and
  its §8b text contradicts its own figure over where AC V lives. Runnable
  checks, one dial position at a time:
  - `dmm-cli --device vc880 get mode` on every position — the listing should
    name exactly the functions that position offers, `*` on the live one.
    V~, Lo and capacitance offer one function each, so they print "no
    switchable modes" instead of a list. Report the dial symbol and the
    list whenever they disagree
  - switch to every entry the listing offers, with
    `RUST_LOG=dmm_lib=debug dmm-cli --device vc880 set mode "<label>"`, and
    paste the log: one `cycle: pressing SHIFT/SETUP (in X, want Y)` line per
    press, so it records both the press count and the order the function
    codes actually came round in
  - the raw cycle, independent of our table: `dmm-cli --device vc880 command
    select` followed by `dmm-cli --device vc880 read --count 3`, repeated
    until the display returns to where it started, once per position
  - the figure/text conflict: V~ and Lo are separate positions in the
    figure, with AC+DC on V⎓. If SHIFT/SETUP on V~ switches anything, the
    figure is wrong and §8b was right — say what the display did
  - the 0x02 overlap: on the V⎓ position, let the meter auto-range below
    400 mV and check with `dmm-cli --device vc880 read --count 1` whether
    the reading turns into `DC mV` (`dmm-cli --device vc880 debug` prints
    the raw function byte). That overlap is why a bare 0x02 with no other
    code seen yet lists nothing to switch to
  - the settle constants are untuned guesses (no delay, 4 reads for a press
    to show up in the stream). `the meter refused …: SHIFT/SETUP did nothing
    in <mode>` while the display *did* change means they are too tight —
    report the mode and the streaming rate
- `Setting::Range` — implemented 2026-09-07 the same way, pressing RANGE
  (0x46) and re-reading the range byte, with 0x47 for auto. Unverified:
  nobody has confirmed that repeated 0x46 steps the ladder one rung at a
  time on this meter. Runnable check, on a dial position with a stable
  input applied: `dmm-cli --device vc880 get range` should name the rungs
  that function offers, `*` on the live one — report any rung the meter's
  own RANGE button reaches that the listing leaves out. Then
  `RUST_LOG=dmm_lib=debug dmm-cli --device vc880 set range <label>` for each
  of them, and `set range auto` to finish. Paste the
  `cycle: pressing RANGE (in X, want Y)` lines: the press count per rung is
  what says whether 0x46 steps one at a time.
  `<label> never appeared; the meter is back in <label>` means it does not,
  and `the mode changed to <mode>; stopped pressing RANGE` means 0x46 moves
  the function byte too
- `Setting::Hold`, `Rel` and `MinMax` — implemented 2026-09-07 by pressing
  0x4A, 0x48 and 0x49 and reading the flag back, with 0x43 to leave
  MAX/MIN/AVG. MIN/MAX is offered as off/MAX/MIN/AVG. Unverified: the order
  0x49 walks those three in, and whether every mode accepts HOLD and REL.
  Peak is not offered — the vendor command table lists no peak command.
  Runnable check: `dmm-cli --device vc880 set minmax max`, then
  `set minmax min`, then `set minmax avg`, then `set minmax off`, each with
  the badge the LCD shows; `set minmax avg` is also the hardware check the
  AVG flag item above wants, since it only succeeds if byte 31 bit 1 is
  read back. Then `set hold on` / `set hold off` and `set rel on` /
  `set rel off` on two or three dial positions. Run them under
  `RUST_LOG=dmm_lib=debug` and paste the `cycle:` lines. Also unknown: which
  buttons a held meter drops. A mode or range walk presses HOLD off and sends
  a dropped press again; `set hold on` then `set mode` or `set range`
  exercises it
- `dmm-cli --device vc880 capture` exercises all of the above on its own
  since 2026-09-07: once the gate steps pass, every mode step is followed by
  `set range`/`hold`/`rel`/`minmax` through each value, filed as
  `<mode>/<setting>:<label>` sub-steps carrying what the meter read back.
  Report any sub-step with `status: error` and the
  `remote control unreliable on this meter` line if it appears — those are
  the commands this meter refused

**UT803 / UT804 (CH9325 HID, proprietary structured packets)** — UT804
VERIFIED 2026-09-18, UT803 IMPLEMENTED AND NEEDS HARDWARE VERIFICATION:
- **UT804 hardware reports** — three runs by @clazie in
  [#16](https://github.com/antoinecellerier/dmm-tools/issues/16), all over
  the UT-D04 (CH9325) cable at 2400 baud:
  - **2026-09-17, v0.7.0-dev (666fbf5), Linux Mint** — `debug --count 5`
    and a `capture` that stopped after `dcv`: the packet framing, DC V and
    auto-detection. Nine dial positions also read right by eye in the
    Windows GUI, with no frames.
  - **2026-09-18, v0.7.0-dev (3806742), Windows** — all 27 steps at the
    trusted tier, the gate passed (`core_semantics: confirmed`): every dial
    position and SELECT alternate, the mode codes and AUTO. Its `acdcv`
    step caught AC V, and its `cont` samples ran on into Diode and Ω.
  - **2026-09-18 later, same build, Windows** — the gate failed on
    `ohm_ol` (the LCD's `.OL MΩ` against our `0L MΩ`): AC+DC V, signed
    zero, the LCD's OL and LO text, HOLD stopping the stream, and SEND.

  Together they carried the model to `Stability::Verified`: every dial
  position and SELECT alternate decoded correctly (°F has no step). The
  issue stays open for MAX MIN and REL.
- ~~**Readings on a real meter**~~ — **VERIFIED** 2026-09-17 by @clazie on a
  real UT804 (UT-D04 / CH9325). `debug --count 5` and a `capture` `dcv` step
  each returned five DC V readings with no errors, and the reporter confirmed
  the last capture sample against the LCD. See
  [#16](https://github.com/antoinecellerier/dmm-tools/issues/16).
- **Golden fixtures**: 3 in `crates/dmm-lib/tests/golden/ut804/` from that
  run — open leads, the confirmed 1.4 mV reading and a negative one.
- ~~**CH9325 on Windows**~~ — **VERIFIED** 2026-09-17 by @clazie: the same
  meter and cable ran under the Windows GUI as well as Linux Mint, with no
  driver installed, which is what `docs/setup.md` tells users to expect of a
  plain HID cable. Windows was verified before over CP2110 only
  ([#1](https://github.com/antoinecellerier/dmm-tools/issues/1), UT61E+), so
  this is the CH9325's first run on anything but Linux. Both 2026-09-18
  captures ran on Windows. macOS is still open for every bridge but CP2110
  ([#2](https://github.com/antoinecellerier/dmm-tools/issues/2)).
- **Handler decompile (2026-09-16, spec §2)** — the form's event
  handlers, missed by the first decompile, change what the driver needs:
  - ~~**Wire format**~~ **Fixed 2026-09-17**: both apps' `USB Connect`
    and RS232 handlers take 11-byte packets (9 data bytes, CR, LF) and
    use only low nibbles; the 14-byte index-nibble check belongs to an
    unused UT60A/B/C path. The driver now splits the stream on CR LF.
    **VERIFIED** 2026-09-17 by @clazie on a real UT804 (UT-D04 / CH9325):
    every packet in the `debug` and `capture` runs was 11 bytes ending
    `0D 8A` and decoded. See
    [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)
  - **UT803 rate**: the UT803 app sets 19200 on the CH9325 and on its
    serial port, and the UT803 manual gives 19200 7O1.
    `transport/ch9325.rs` tries 2400 first and takes any report as an
    answer, so a UT803 stays at 2400
  - ~~**UT803 positions**~~ **Fixed 2026-09-17**: the vendor parser's
    position k is packet byte k-1; `parse_measurement_ut803` is fed `A`,
    bytes 1-9 and `D`, as the vendor's is
  - ~~**UT804 overload**~~ **Fixed 2026-09-17**: the vendor reads
    nibble 1 = A as an overload unless nibble 2 = C, which it shows as
    "L0." with value 0 (spec §7.4 item 6), and so does
    `parse_measurement_ut804` now. **VERIFIED** 2026-09-18 by @clazie on
    a real UT804 (UT-D04 / CH9325): Ω, diode and continuity OL, and LO on
    the 4-20 mA %, decode as the LCD reads them. See
    [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)
  - **Nothing sent**: the apps send only the feature report; no trigger
    byte or command
  - **Idle reports**: the apps end each packet at a report without
    payload and release the connection after 1900 in a row, so the
    vendor expects the bridge to report while the meter is silent
- **Resolved (2026-06 review)** — see spec §7.4 for full evidence:
  - **Sign**: UT804 = nibble 9 bit 2 (previously misread as HOLD);
    UT803 = nibble 8 bit 2. The old "sign global with no writer" was
    Delphi RTL locale state (NegCurrFormat), a red herring.
  - **Two layouts**: UT803 and UT804 use different payloads (UT803:
    range=nib 2, digits=nibs 3-6, own mode codes, no 0xD/0xA markers);
    the parser is now model-split.
  - **Decimal positions count from the left**; all range→dp tables
    re-derived per mode.
  - **UT804 mode table corrected** for 9 of 15 codes (6=°C, 7=µA,
    8=mA, 9=A, A=Cont, B=Diode, C=Freq/Duty, D=°F, F=mA%).
  - **Overload** = nibble 1 == 0xA (UT804) / nibble 8 bit 0 (UT803).
  - **Nibbles 12-14 never read by the vendor** (confirmed via the
    Delphi-string access pattern); UT803 also ignores nibbles 1, 11-14.
  - UT803 HOLD = nibble 9 bit 3. UT804 HOLD is in neither vendor
    parser; the meter sends nothing while it is on (below).
- Transport: CH9325 HID, 2400 baud first, 19200 fallback — implemented.
- **Community cross-check (2026-09-16, spec §8)** — sigrok and
  `UT804.LOG` contradicted the UT804 framing we implemented then:
  - ~~**Wire format**~~ **Fixed 2026-09-17**: both give UT71x — 11
    bytes, 2400 7O1, `0x30`-`0x3F` characters, CR LF — and #16's UT804
    sends exactly that (`3x`/`Bx` bytes, then `0D 8A`). The driver
    splits on CR LF with bit 7 masked. **VERIFIED** 2026-09-17 by @clazie
    on a real UT804 (UT-D04 / CH9325): that framing carried readings end
    to end at 2400 baud. See
    [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The
    UT803 was not cross-checked
  - **Overload**: `UT804.LOG` reads `::0<:` (nibbles A A 0 C A) as
    overload and the 4-20 mA underflow `:<0::` as "L0", as the vendor
    does and, since the fix above, our parser
  - **HOLD and REL**: `UT804.LOG` says nothing is transmitted while HOLD
    is on and REL is never transmitted; a UT71x packet has no nibbles
    12-14. ~~HOLD~~ — **VERIFIED** 2026-09-18 by @clazie on a real UT804
    (UT-D04 / CH9325): with HOLD on the LCD and SEND still lit, no
    packet arrives. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). REL is open
  - `UT804.LOG` holds 36 real packets across 9 dial positions and their
    sub-functions. Run through `parse_measurement_ut804` as low nibbles
    (2026-09-16, throwaway test): mode, unit, decimal point, AUTO/MAN,
    coupling and sign match the log's labels on every non-overload
    packet; all five overload packets came out as "L0" 0.0 before the
    overload fix. Test vectors
    once we decide on attribution (GPL-3.0 repository, author of the log
    unknown)
- **Needs hardware verification** (the payload layouts above are
  decompile-derived):
  - ~~One frame per dial position~~ — **VERIFIED** 2026-09-18 by @clazie
    on a real UT804 (UT-D04 / CH9325): every dial position and SELECT
    alternate, with its mode code, decimal point and unit. See
    [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The UT803's is open
  - ~~A negative reading (sign bits) and an overload (OL patterns)~~ —
    **VERIFIED** 2026-09-18 by @clazie on a real UT804 (UT-D04 /
    CH9325): -12.041 and -24.196 V DC, and OL on Ω, diode and
    continuity. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The
    UT803's are open
  - ~~Does the UT804 need SEND after a power cycle~~ — **VERIFIED**
    2026-09-18 by @clazie on a real UT804 (UT-D04 / CH9325): SEND is off
    at power-on, nothing is sent until it is pressed, and EXIT turns it
    off, as the manual's Table 2-2 says. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)
  - MIN/MAX/REL/low-battery toggles — candidates: UT803 nibble 9
    bits 2-1, UT804 nibble 9 bit 3. The UT803 pair lights indicators of
    its own (§7.4 item 2), so the parser leaves it silent; the UT804 bit
    is in neither vendor parser, and `UT804.LOG` says HOLD stops
    transmission rather than setting a bit, so it is reported as
    unrecognised to draw a trace (noted 2026-09-17)
  - ~~UT804 mode 0xF ("mA%") dial position; which of modes 1/2 each V
    dial sends~~ — **VERIFIED** 2026-09-18 by @clazie on a real UT804
    (UT-D04 / CH9325): 0xF is the mA position's 4-20 mA %, shown with
    unit `%`; DC V sends 1 and AC V 2. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)
  - UT804 mode 0xE (unknown glyph; hFE?): the UT804 has no dial position
    for it (manual Table 2-1), so it may never be sent
  - UT804 °F: no capture step asks for it
  - UT804 diode OL: the rule the other OL frames follow predicts `.0L`,
    which we print, but the reporter twice accepted `0L`; unconfirmed
  - UT803 frequency range 0 decimal position; tachometer (RPM) frames
  - A blank digit (A) inside a reading: `assemble_value` renders a blank
    right after the decimal-point digit with the point twice ("12..45"),
    an unparseable value. No spec'd packet has one; the parser reports
    it as unrecognised (noted 2026-09-17)
  - Whether 0x5A trigger byte helps/hurts; the UT803's streaming rate
  - CH9325 feature-report layout: the UT803/UT804 apps send
    `60 09 00 00 03` (`0x03` in byte 5), the SDK DLL `60 09 03 00 00`
    (spec §1.2). `transport/ch9325.rs` sent the DLL's until 2026-09-16 and
    sends the apps' since, for both rates. sigrok, Lukas Schwarz and
    `he2325u.cpp` all send the apps' layout (spec §8). The sigrok wiki
    reads byte 5 as the data-bit count, so the DLL's layout asks for 5
    data bits. Issue #16's UT804 gave clean bytes at 2400 with both
    layouts (spec §1.2), which settles neither: 2400 may be the bridge's
    default
  - CH9325 start-up takes any report as an answer, even one with no meter
    bytes, and falls back to 19200 baud when none comes within 300 ms —
    a rate the UT804 app never sets (the UT803 app's rate).
    Lukas Schwarz and sigrok both describe `F0` reports while the meter
    is silent, so a report at start-up proves nothing
  - Three parser behaviours surfaced by the 2026-09 snapshot tests, to
    settle against real frames rather than change blind: ~~`range_label`
    is never set for either model although the per-mode tables know the
    range~~ — **VERIFIED** 2026-09-18 by @clazie on a real UT804 (UT-D04 /
    CH9325): the UT804 labels its readings from the manual's Table 2-3,
    whose ranges match the decimal points of every range captured. See
    [#16](https://github.com/antoinecellerier/dmm-tools/issues/16). The UT803's is
    still unset; a UT804 LO frame (digit 1 = 0xA, digit 2 = 0xC) is reported
    as `Normal(0.0)` with the LCD's text "L0." ("-L0." with the sign
    bit), which CSV/JSON export as that string; and UT804 `acdc == 3`
    (AC+DC) sets the DC flag.
  - ~~Signed zero: a UT804 packet with zero digits and the sign bit
    (issue #16) reads `-0.0000`, value `-0.0`. What does the LCD show?~~
    — **VERIFIED** 2026-09-18 by @clazie on a real UT804 (UT-D04 /
    CH9325): the LCD shows the minus, `-000.00 µA`, `-00.000 mA` and
    `-00.000 A`. See [#16](https://github.com/antoinecellerier/dmm-tools/issues/16)
  - After a long pause in reading, the kernel's HID report queue can
    drop reports, and the bytes left could splice two packets into one
    that passes the packet check (not seen yet)
  - A UT803 is not auto-detected: the CH9325 starts at 2400 and the
    UT803 talks at 19200, so it has to be named. Its packets are
    unconfirmed on hardware
- See `docs/research/ut803/reverse-engineered-protocol.md` for full spec.
- UT805A uses USB-to-serial (virtual COM port, NOT HID) with a fully
  documented ASCII text protocol (9600/8N1, bidirectional). Needs serial
  transport — separate scope from HID-based meters.

**UT8802 / UT8802N**:
- Frame extraction (8-byte, 0xAC header, no checksum)
- ~~Negative reading shown and exported unsigned~~ **Fixed 2026-09-08**:
  the parser now puts the sign in `display_raw`; the digit nibbles cannot
  carry one (uci_dll_decompiled.txt:24813). The `dcv_negative` capture
  step still confirms the bit itself
- 0x5A streaming trigger byte — the vendor DLL only sends 0x5A on the
  QinHeng/CH9325 init path, never to CP2110 devices (2026-06 review);
  does the UT8802 stream without it, and is sending it harmful?
- Position code mapping (35 codes, 0x01-0x2D with gaps)
- Unit magnitude per position. **Resolved from vendor [VENDOR]** (2026-07
  review): the display digits are range-relative, and `FUN_1001cd30`
  (uci_dll_decompiled.txt:23603) maps each position code to the SI prefix
  the vendor renders via `FUN_1001cec0`. Previously the table reported
  base units with the decade stranded in `range_label`, so capacitance
  and frequency readings were exported off by up to 10^9 (10 nF logged
  as "10.00 F"). Hardware must confirm: does a 200 mV reading arrive as
  millivolts (e.g. "123.45") as the prefix table implies, and does a
  2 kΩ range report "1.234" for 1.234 kΩ?
- Display digit order. **Corrected from vendor [VENDOR]** (2026-06
  review): MSD = byte 4 low nibble, then byte 3 hi/lo, byte 2 hi/lo
  (uci_dll_decompiled.txt:24714-24719) — previous code had the order
  reversed. Hardware confirmation pending: any reading with distinct
  digits settles it.
- Decimal point position (byte 5 low nibble, 0-4)
- AC/DC determination. **Corrected from vendor [VENDOR]** (2026-06
  review): AC/DC comes from a position-code lookup (FUN_1001ca30);
  byte 5 bits 4-5 are diode/SCR probe direction, not coupling. What
  byte 5 bits 4-5 carry outside diode/SCR modes is unverified.
- Overload. **Corrected from vendor [VENDOR]** (2026-06 review): the
  vendor's only OL mechanism is byte 7 bit 6 (uci_dll:24806-24821);
  digit nibbles are never checked. We keep the 0x0C digit check as a
  defensive secondary. Does a real OL set bit 6, send 0x0C nibbles,
  both, or neither?
- Sign/polarity (byte 7 bit 7)
- AUTO flag inverted logic (byte 7 bit 2 clear = auto ON)
- Byte 7 flag bits (HOLD/REL/MAX/MIN). **Resolved from vendor [VENDOR]**
  (2026-04-19): a second Ghidra pass traced each status-word bit back
  to a specific byte-7 bit via the shift chain in `FUN_1001e0a0`.
  Mapping: MIN=bit 0, MAX=bit 1, AUTO=bit 2 (inverted), REL=bit 3,
  HOLD=bit 4, Over=bit 5, OL=bit 6, Sign=bit 7. All five previous
  guesses (bits 6/5/4/3 for HOLD/REL/MAX/MIN) were wrong — HOLD and
  REL swap with bit-range 4-5 vs 0-3. Real-hardware confirmation is
  still pending. See `docs/research/uci-bench-family/reverse-engineered-protocol.md` §3.5.
- Byte 6 purpose: bargraph or secondary status? [UNVERIFIED]
- Byte 4 high nibble and byte 5 bits 6-7: the vendor never reads them,
  and the parser reports them as unrecognised when set (noted 2026-09-17)
- Overload detection (BCD nibble 0x0C)
- Streaming rate

**UT8803 / UT8803E** ([issue #3](https://github.com/antoinecellerier/dmm-tools/issues/3)):
- Frame extraction (21-byte, AB CD header, BE checksum)
- 0x5A streaming trigger byte. **Corrected (2026-06 review)**: the
  vendor never sends 0x5A on the CP2110 path (FUN_1001d460 performs no
  UART write; the 0x5A lives in the CH9325 init FUN_1001d360). We no
  longer send it. Hardware should confirm the meter streams unprompted.
- Mode byte mapping (23 position codes, 0x00-0x16)
- Range byte (0x30 prefix, like UT61E+)
- `range_raw` is stored unmasked (0x31 for range 1) where every other
  family masks off the 0x30 prefix — inert until a UT8803 spec table
  keys on it (noted 2026-09-08)
- Sign of a negative value: the ASCII display field may carry `-`
  itself, in which case the parse-time sign bit double-negates. Until a
  `dcv_negative` capture settles it, `display_raw` keeps the meter's own
  digits, so a negative reading shows and exports unsigned where the
  UT8802 now shows the minus (noted 2026-09-08)
- Unit magnitude prefixes per (mode, range). **Resolved from vendor
  [VENDOR]** (2026-06 review): FUN_1001cdc0 maps (mode, range) → n/µ/m/
  none/k/M and FUN_1001cff0 gives base units (IndR/CapR are ESR in Ω;
  IndQ/CapD are unitless). The display value is range-relative, so the
  displayed unit now carries the prefix (e.g. kΩ). Hardware must
  confirm range-byte values per mode before this counts as verified.
- Display bytes (5 raw bytes — ASCII or binary encoding?)
- Flag byte → semantic flag mapping (HOLD, REL, MIN, MAX, AUTO, OL, Sign).
  **Resolved from vendor [VENDOR]** (2026-04-19): a second Ghidra pass
  traced each status-word bit back to a specific raw-frame bit by
  following the intermediate locals and shift chain in `FUN_1001e5f0`.
  Mapping: HOLD / OL / Sign from frame byte 14 bits 0/2/3, REL / AUTO
  (inverted) from frame byte 15 bits 0/1, MIN / MAX from frame byte 16
  bits 0/1. Still needs real-hardware confirmation but no longer a
  speculative guess. See `docs/research/ut8803/reverse-engineered-protocol.md`
  §2.3 for the derivation.
- Flag bytes 14-18: do they carry a 0x30 prefix like the range byte (and
  the UT61E+'s flag bytes)? The parser reports any bit the vendor does not
  read (bit map in spec §2.3) except bits 4-5, which it leaves out until a
  capture settles this (noted 2026-09-17)
- Display value parsing (5 bytes → float)
- Streaming rate (~2-3 Hz per manual)

**UT171A / UT171B / UT171C** ([issue #4](https://github.com/antoinecellerier/dmm-tools/issues/4)):
- Frame extraction. **Corrected (2026-06 review)**: framing is
  byte-identical to UT181A — 2-byte LE length = payload + checksum,
  total = length + 4, LE16 checksum over [2..len+2). The previous
  1-byte-length model (total = length + 5) could never have validated a
  real frame; its "reserved" byte was the length high byte and its
  "padding" byte the checksum low byte. Confirmed by connect-command
  arithmetic and gulux/Uni-T-CP2110; needs one real measurement frame
  to close.
- Connect command (`AB CD 04 00 0A 01 0F 00`) — may be needed before streaming
- Mode byte mapping (26 modes, 0x01-0x24)
- Float32 LE value parsing — resistance is range-relative (kΩ at range
  >= 2, MΩ at >= 5 per gulux); scaling for capacitance/conductance
  [UNVERIFIED]
- Flags byte (HOLD bit 7, AUTO bit 6 inverted, Low Battery bit 2) — the
  decompile citations previously backing bits 0/1/3 were Delphi dataset
  code, not wire protocol (2026-06 review); all flag bits need hardware
- Range byte (raw, 1-based)
- Extended frame (27 bytes, frame type 0x03) — not yet parsed; no
  decompile evidence located for its layout
- Status2 byte (offset 13) — capture-deduced 0x40=DC/0x20=AC, no
  decompile evidence
- Aux value interpretation — kHz frequency on V AC / mV AC per gulux;
  other modes unknown
- Remote function selection — the vendor app's function grid proves the
  meter takes some command to change function, and the (from, to) mode
  transition table in the spec (§4.7, `FUN_00630e0b`) is the lead, but the
  codes it lists exceed the one-byte command field the frame builder
  (`FUN_00755400`) writes, and the two were never reconciled. Recovering
  the real encoding — Delphi virtual dispatch, the method-table route that
  found the UT181A's SET_MODE — is what stands between the UT171 and
  `dmm-cli set mode`. The cycle-to-target driver does not apply either: no
  cycle-button command is known for this family.

### UT181A — confirmed on hardware, formats still open

Two reporters have run the UT181A on a real meter, both over the CH9329
(UT-D09) cable ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)).
The start command, framing, the normal-format value layout, V DC,
V AC + Hz and dual-thermocouple temperature are confirmed; the binaries
keep the EXPERIMENTAL label until the items below are closed. A vendor
trace of `UT181A.exe` V1.05 on 2026-09-06 added the mode- and
range-command semantics (spec §6.1, §7.1) — evidence about what UNI-T's
own software sends, not hardware confirmation.

- ~~SET_MONITOR command required during init~~ — **VERIFIED** 2026-04-07
  by @alexander-magon on real UT181A (CH9329 cable). The meter does not
  stream until the host sends CMD_CONT_DATA (`AB CD 04 00 05 01 0A 00`).
  Communication ON alone is not sufficient. See PR #8.
- ~~Frame extraction (2-byte LE length, LE checksum)~~ — **VERIFIED**
  2026-04-07 by @alexander-magon: frames parse correctly on real hardware.
- ~~Float32 LE value parsing with precision byte~~ — **VERIFIED**
  2026-04-07 by @alexander-magon: VDC mode returns valid float32 values.
  Precision byte decimal places (bits 4-7) confirmed to produce sane
  display formatting.
- ~~Normal-format value layout (main + aux1 + aux2 + bargraph)~~ —
  **VERIFIED** 2026-09-02 by @diego351 on real UT181A (CH9329 cable).
  A 57-byte V AC payload (`0x1121`, mains) consumes exactly as a 6-byte
  header + 13 + 13 + 13 + **12**: main, aux1 and aux2 each carry a
  precision byte and the bargraph does not (spec §5.3, previously
  community-sourced only). A 32-byte temperature payload (`0x4211`,
  two thermocouples) consumes exactly as 6 + 13 + 13. Confirms `misc`
  bits 1/2/3 (aux1 / aux2 / bargraph present) and `misc2` bits 0/1
  (auto-range, HV warning) alongside it. A 31-byte V DC payload
  (`0x3111`, PR #8, second meter) consumes as 6 + 13 + **12**, so the
  12-byte bargraph width holds on another meter and mode. Regression
  frames in `crates/dmm-lib/src/protocol/ut181a/parse.rs` and
  `crates/dmm-lib/tests/golden/ut181a/`.
- Mode word decoding (79 nibble-encoded uint16 modes) — `0x3111`
  (V DC), `0x4211` (°C) and `0x1121` (V AC Hz) verified on hardware;
  the rest still need a meter. **Vendor-confirmed 2026-09-06** from
  `UT181A.exe` V1.05 (spec §6.1): the composition rule
  `family | primary << 4 | secondary` is what the vendor app both sends
  and decodes, and the per-family variant table (which primary variants
  exist per dial position, their on-meter captions, and which offer REL)
  is now written down. Still 3 modes hardware-confirmed, not 79 — the
  vendor binary says what UNI-T's software sends, not what the meter
  accepts. The 2026-06 corrections all survive the vendor trace:
  DC-current n1=2 codes (0x8121/0x9121/0xA121) are AC+DC, not Hz;
  0x4121 = mV DC Peak (the vendor's own label decoder has no 0x4131,
  so sigrok's alternative can be dropped once hardware agrees);
  0x5212 = Continuity open-beeper and 0x6112 = Diode Alarm (not REL
  variants); temperature n1 selects the display arrangement
  (T1(T2)/T2(T1)/T1-T2/T2-T1) — the n1=1 arrangement is hardware-
  confirmed to put one probe on the main display and the other in aux1;
  the other three are vendor-confirmed but not hardware-confirmed.
  COMP digits read from the low nibble unshifted. Need at least one mode
  per family to confirm the nibble decoder works broadly.
- HOLD command `[0x12, 0x5A]` — **vendor-confirmed 2026-09-06**: the
  wrapper at `0x8703e8` hard-codes the single payload byte `0x5A` and is
  the Hold action's only call site, independently of antage. Still needs
  a meter: confirm it actually toggles HOLD, and whether bare `[0x12]`
  works too
- mV AC+DC (`0x2141`) — the vendor UI emits it, but its own label
  decoder has no case for family `0x21` with n1=4 (spec §6.1). Needs
  hardware to say whether the meter accepts the word
- Device-sent unit string parsing — "VDC", "VAC", "Hz", "ms" and
  Latin-1 "°C" (`0xB0 0x43`) verified on hardware; the remaining unit
  strings in spec §8 (`~`, `k~`, `M~`, `nS`, `nF`, `uF`, `dBV`, `dBm`,
  `%`, "°F" …) still need a meter
- Relative format (0x10) parsing — implemented, needs hardware verification
  (delta/reference/absolute values parsed into main + aux_values)
- Min/Max format (0x20) parsing — implemented, needs hardware verification
  (current/max/avg/min with timestamps parsed into main + aux_values)
- Peak format (0x40) parsing — implemented, needs hardware verification
  (peak max/min parsed into main + aux_values)
- COMP mode extension parsing — implemented, needs hardware verification
  (comp mode/result/limits parsed into aux_values)
- Sub-value display end to end (2026-09-02) — the GUI reading panel,
  recording log, graph selector/overlay and both CSV exports now consume
  `aux_values`, so a capture in REL, MIN/MAX, Peak or COMP verifies the
  parser and the display in one go. Ask for `read --format csv` runs in
  V AC and dual-thermocouple modes (checks the `auxN_*` columns) and a
  GUI screenshot in MIN/MAX (three same-unit overlays) alongside the
  capture YAML
- Range label lookup table — range byte 0x03 on V AC decodes to "600V",
  confirmed against a 239 V mains reading (2026-09-02), but the meter
  chose that range itself; **manual** range mode is still unverified,
  as are the other families' ladders. **Vendor-confirmed 2026-09-06**
  (spec §7.1): the ladders per family, that the index is 1-based, and
  that A DC, A AC, Celsius, Fahrenheit, Beeper, ns and Diode have no
  manual range. Duty and ms-Pulse ladders are new and have no community
  cross-check at all
- Misc2 flags: lead_error (bit 3), comp (bit 4), record (bit 5) — now
  parsed but not yet verified on real hardware. Bits 0 (auto-range) and
  1 (HV warning) confirmed 2026-09-02
- `lookup_range_label` answers "Auto" for range byte 0 before consulting
  the mode's ladder, so a fixed-range mode (Duty, °C) with range 0 reads
  as auto-ranging. The real temperature frame sends 0x01 and decodes to
  "", so this may only ever hit synthetic frames (noted 2026-09-08)
- Bargraph value meaning — the misc bit 3 field carries a float32 plus
  its own unit, and in the mains capture it read 241.02 VAC against a
  239.22 VAC main reading, so it is *not* the displayed value. Whether
  it is the bargraph pointer or the meter's fast (10 Sa/s) sample is
  unresolved; the value is parsed over but not exposed. Needs a capture
  with a deliberately changing input to tell the two apart
- CP2110 cable on a UT181A — both hardware reports so far used the
  CH9329 (UT-D09). The CP2110 transport itself is well exercised by the
  UT61E+, but nobody has run the two together, and @diego351's older
  CP2110-equipped unit was never detected on macOS at all (with other
  software, before dmm-tools existed). Unverified, not known-broken
- SET_MODE (0x01) — vendor-traced; implemented 2026-09-06 as
  `dmm-cli get mode` (lists the modes the dial reaches) and `set mode`
  (switches by label or by a unique fragment of one), and the GUI's mode
  dropdown under the
  reading. No meter has answered one yet. Runnable checks, each with an
  LCD photo beside the tool's output:
  - `dmm-cli --device ut181a get mode` on the V AC dial — expect six
    choices, `*` on the live one
  - `dmm-cli --device ut181a set mode "V AC Hz"` (0x1121), then
    `set mode "V AC dBm"` (0x1161), then `set mode "V AC"` (0x1111)
  - on the temperature dial, `set mode t1-t2` (0x4231), then
    `set mode "°C"` (0x4211) — a unique fragment of a label is enough
  - a switch prints `Meter now in <label>`; `Meter did not switch` or a
    refusal is the interesting result — report it verbatim
  - the family-local rule: the listing should never offer a word from
    another dial position, and a word from another family should be
    refused or ignored
- REL command — implemented 2026-09-06 (`dmm-cli command rel`, and the
  GUI's REL button now appears for this meter) as SET_MODE with nibble 0
  flipped, gated per variant from the vendor UI. Needs hardware: on
  V AC, `dmm-cli --device ut181a command rel` should light REL on the
  LCD and the meter should report the companion word 0x1112 (shown as
  `V AC REL`); a second `command rel` should leave REL. Continuity and
  diode spend nibble 0 = 2 on the open-beeper and alarm functions, so
  the tool refuses `command rel` there without sending anything —
  confirm the meter's own REL button is equally dead in those modes
- REL companion words on the non-plain variants — 0x1142 (V AC LPF),
  0x1152 (V AC dBV), 0x1162 (V AC dBm), 0x2142 (mV AC+DC), 0x3122
  (V DC AC+DC), 0x4222 (°C T2, and its °F mirror 0x4322), 0x8122 /
  0x9122 / 0xA122 (µA / mA / A DC AC+DC) are vendor-traced only. Needs
  hardware: enter each variant, toggle REL, and confirm the mode word
  the meter reports back is the companion listed here
- SET_RANGE (0x02) — vendor-traced 1-based index semantics. Since
  2026-09-07 `Setting::Range` sends it absolutely: the choice id *is* the
  SET_RANGE byte, 0 being auto, so one named switch tests the encoding
  directly instead of inferring it from a step. Needs hardware: on V DC,
  `dmm-cli --device ut181a set range <second rung's label>` (take the
  label from `get range`), then
  `dmm-cli --device ut181a get range --format json` — `"current"` and the
  `current: true` entry should both name that rung, and it should match the
  LCD's range annunciator. Then `dmm-cli --device ut181a set range auto`
  should hand ranging back, and `get range --format json` show `"Auto"`
  current. `dmm-cli --device ut181a debug` prints the raw payload if the
  range byte itself is wanted. `dmm-cli --device ut181a command range`
  still steps the ladder a rung at a time from the last range the meter
  reported, so it is the fallback if a named switch is refused
- `dmm-cli --device ut181a capture` exercises SET_RANGE and the flags on
  its own since 2026-09-07: once the gate steps pass, every mode step is
  followed by `set range`/`hold`/`rel`/`minmax` through each value, filed
  as `<mode>/<setting>:<label>` sub-steps carrying what the meter read
  back. Report any sub-step with `status: error` and the
  `remote control unreliable on this meter` line if it appears — those are
  the commands this meter refused
- Duty cycle (0x7211) and pulse width (0x7311) range labels — the vendor
  form's range combo holds four items for each (spec §7.1: 60 / 600 / 6000 /
  60000) but no source says what the LCD calls those rungs, so `get range`
  offers nothing there and the GUI shows a plain label. Needs hardware: on
  the Hz dial switched to Duty, `dmm-cli --device ut181a command range`
  four times with `read --count 1` after each, noting the LCD's range
  annunciator and the `"range"` field; then `command auto`. The labels the
  LCD shows are what the table needs
- Range bytes and sub-values the parser reports as unrecognised (noted
  2026-09-17): on a fixed-range mode anything above 1 (the temperature
  frame sends 1; A DC/AC, continuity, nS and diode frames would confirm
  it), on duty cycle and pulse width only bytes past 8 until the item
  above settles their ladder, and a sub-value on any mode but the T1/T2
  temperature arrangements and the Hz variants — the T1-T2 difference
  included
- SET_MIN_MAX (0x04) payload width — the vendor app sends **one** byte,
  not the uint32 antage and sigrok describe (spec §4.2). The code sends
  one byte; a meter needs to confirm MIN/MAX actually engages: on V DC,
  `dmm-cli --device ut181a command minmax` should light MIN/MAX on the
  LCD and put min/max/avg sub-values in the stream, and
  `command exit_minmax` should leave it
- `Setting::Hold`, `Rel` and `MinMax` — implemented 2026-09-07, all three
  absolute: HOLD is the 0x12/0x5A button press, REL is SET_MODE with nibble
  0 flipped (and is offered only where `mode::rel_supported` says the
  vendor app enables it), MIN/MAX is SET_MIN_MAX 0/1, whose payload width
  is the open question above. Each is confirmed by re-reading the meter's
  own flags, so a command the meter ignores now reports
  `<setting> <state> did nothing; the meter is still in <state>` — worth
  quoting in any report. Peak is not offered as a setting: it is a mode
  variant on this meter, reached through `dmm-cli set mode`
- Command replies (type 0x01, "OK" / "ER") — community-sourced only (the
  vendor app's handling of them was not traced), never seen from a meter. Since 2026-09-06 every command above waits
  for one and turns "ER" into an error, while silence still passes, so
  run the checks with `RUST_LOG=dmm_lib=debug` and report which line
  appears: `ut181a: <command> acknowledged` (the meter answered) or
  `ut181a: no reply to <command> … assuming accepted` (it did not),
  alongside the LCD photo showing whether the command took effect
- **Not implemented**: recording protocol (0x0A-0x0F), saved measurement
  retrieval (0x07-0x09), SET_REFERENCE command, timestamp decoding,
  response types 0x03/0x04/0x05/0x72

### CP2110 feature reports (AN434)
- (none pending)

### Commands not fully verified
- **Get Name (0x5F):** Verified — returns two frames: ack (FF 00) then ASCII name (e.g. "UT61E+").

### MIN/MAX and Peak measurement reporting — RESOLVED

Verified 2026-03-21 on real UT61E+ with bench PSU (DC V, 3.1V→5V ramp)
and AC mV (open leads, ~8.7 mV noise).

- **MIN/MAX sends the stored value, not the live reading.** With MIN/MAX
  active during a 3.1V→5V ramp: MAX state reported 5.004V (frozen),
  MIN state reported 3.102V (frozen). The display value field contains
  the stored min or max, not the live measurement.
- **MIN and MAX flag bits cycle independently.** The meter cycles
  MAX (byte 11 bit 3 only) → MIN (byte 11 bit 2 only) → MAX → ...
  as a 2-state cycle. The bits are never both set simultaneously.
  No AVG state is reported over USB (AVG may be LCD-only or absent on UT61E+).
- **AUTO flag is cleared during MIN/MAX** (byte 12 bit 2 set = manual range).
  The meter locks the range when MIN/MAX recording is active.
- **Peak mode works the same way.** Peak command (0x4D) activates on AC mV
  (context-dependent — does not activate on DC V). Reports stored
  instantaneous peak values (not RMS): P-MAX=19.33mV, P-MIN=-290.25mV.
  Cycles P-MAX (byte 13 bit 2 only) → P-MIN (byte 13 bit 1 only).
- **Exit Peak (0x4E) works.** Clears peak flags, returns to live readings.
- **Mock updated** to match: independent flag cycling, stored values,
  AUTO cleared during MIN/MAX.

### UT61E+ RANGE command (0x46) — RESOLVED

Resolved 2026-09-07 on our UT61E+ (CP2110 cable, `RUST_LOG=dmm_lib=debug`)
with `dmm-cli get range` / `set range`, which press 0x46 and re-read the range
byte until the target rung shows.

- **The first press from auto engages manual ranging on the rung the meter
  is already in** — it does not step. Every further press steps exactly one
  rung up.
- **The top rung wraps to the bottom:** 1000V → 2.2V on DC V.
- **`0x47` restores auto-ranging**, from a manual rung, in one command.
- **The mode byte never moved under any press**, on either the V⎓ or the V~
  dial position. So `0x46` is a pure range stepper.
- With 1.5 V DC applied, the same walk read 1.5023 V (2.2V), 1.502 V (22V),
  1.51 V (220V) and 1.5 V (1000V): resolution follows the rung, as on the LCD.

Evidence — leads open, V⎓ dial, auto in 2.2V at the start:

```
set range 22V     cycle: pressing RANGE (in Auto, want 22V)
                  cycle: pressing RANGE (in 2.2V, want 22V)     → 22V
set range 220V    cycle: pressing RANGE (in 22V, want 220V)     → 220V
set range 2.2V    cycle: pressing RANGE (in 220V, want 2.2V)
                  cycle: pressing RANGE (in 1000V, want 2.2V)   → 2.2V
set range 1000V   three presses: 2.2V → 22V → 220V → 1000V
set range auto    cycle: setting auto-range (in 1000V)          → auto (220V, settling to 22V)
```

The 2026-07-29 capture that opened this item — six `range` presses whose
range index went 0, 2, 0, 0, 0, 0 and whose mode byte appeared to flip
DC V ↔ AC+DC V at presses 4 and 6 — is explained by reading the frame before
the meter had applied the press: the indices are stale reads, not a strange
stepping order. Whatever produced the mode flips there, it was not `0x46`.
The library now waits for a fresh frame and re-reads a stale one instead of
pressing again, which is why the walk above lands one rung per press. The
capture wizard still sends `range` once and restores auto rather than
sweeping.

`get` prints **no range row** where there is nothing to choose: verified
2026-09-07 in DC mV (fixed range, and `get range` / `set range` answer
`Note: UNI-T UT61E+ has no switchable ranges in DC mV — use the dial.`),
in DC A and in AC A (both table entries read `20A`). AC mV joined them the
same day — see "Range tables" below.

### Range tables

Tracked in [issue #6](https://github.com/antoinecellerier/dmm-tools/issues/6).

- Range byte values for most modes still need verification against real device.
- ~~**AC V top range: 750V vs 1000V conflict (2026-06 review).**~~ —
  **RESOLVED** 2026-09-12 in favour of **1000V**, from the manual's AC table
  read off the PDF rendering (printed page 27): the UT61E+ column ends at
  `1000.0V / 0.1V`, the shared UT61B+/UT61D+ column at `1000V / 1V`, and
  neither has a 750V row. The same section gives max input voltage and
  overload protection as 1000V, and the LoZ ACV rows the UT61D+ adds are
  600.0V and 1000V, which `ut61d_plus.rs` already had right.
  **The 750V had no source.** It is absent from the archived vendor
  decompile, which carries no range-label strings at all — nor do `DMM.exe`
  and `MyCore.dll` in either ASCII or UTF-16 — so the "from the vendor
  decompile" attribution in this item was wrong; `git log -S` puts it in the
  bootstrap commit 048d44b. Corrected for all three models, with the family
  spec's three `ac_v` rows.
  **Hardware is consistent but cannot arbitrate this one.** The 2026-09-07
  UT61E+ RANGE walk reached the rung and read one decimal there
  (`    0.0`, range byte 3), which is the manual's 0.1V resolution — but a
  750.0V rung would also carry 0.1V on a 22,000-count display, so the
  decimal count cannot separate them. Only applying more than 750V AC could,
  which is not a test worth running; the manual settles it instead.
- **UT61D+ amps: manual lists 6.000A and 20.00A; code has only 20A**
  (`ut61d_plus.rs` dc_a/ac_a copied from E+). Needs the 6A range row;
  blocked on D+ hardware for index ordering (issue #7).
- **Frequency ranges in code are invented structure, on every model in the
  family** — the manual gives only a span (10.00 Hz–10.00 MHz for the
  6,000-count models), no discrete ranges, and the code's five ranges top out
  at 600 kHz on the B+/D+ and 220 kHz on the E+. The E+'s five have no
  provenance comment and no verification entry either, so it cannot arbitrate
  for its siblings. The 2026-09-09 UT61B+ capture contradicts the structure
  outright: two Hz readings at range index 0 with different full scales. The
  2026-09-10 re-run cannot break the tie — it read `0.00` and `49.98` at
  index 0, both the 60Hz rung — and RANGE is dead in Hz on both meters, so
  the button cannot walk the ladder either. Needs one frame on a signal above
  60 Hz. **PARKED 2026-09-12: no signal generator here**; the risk it carries
  is set out under the UT61B+'s open items. Issue #7.
- **UT61B+/D+ range-index ordering: ascending, and the mV ranges are
  not part of the V ladder** — settled for the B+: index 0 is each
  ladder's bottom rung; the 2026-09-10 capture pinned DC V 0–1, AC V 0,
  Ω 0/4/5, µA 0–1, mA 0–1 and A 0–1 by the decimal count the meter sent
  at each, and the 2026-09-11 RANGE walk the rest of Ω and DC V, plus
  AC V 2 and capacitance 5. The mV rung 0s are measured too (`dcmv.yaml`,
  `acmv.yaml`). What is left is not [DEDUCED] in the guessing sense but
  *pinned between measured ends*: with ascending order established and both
  ends of a ladder measured, AC V 1, capacitance 1–4 and 6 and mV 1 have
  nowhere else to sit. Only the Hz ladder is still genuinely unknown, and
  the whole D+ table remains [DEDUCED] for want of D+ hardware. Issue #7.
- **UT61B+/D+ mV ladders are offered to the RANGE driver, untested — but
  needs no ask and no new step.** The `dcmv` and `acmv` steps are not gate
  steps, so the sweep walks their ladders like any other mode step's; they
  came back without range sub-steps on 2026-09-10 only because they then ran
  before the gate closed (above). The next plain `capture` run on a B+ or D+
  files either `dcmv/range:600mV` or a `RANGE did nothing` error, whichever
  is true.
  **Our UT61E+ cannot answer it.** `range_is_fixed` covers DC mV and AC mV
  there on real runs (DC mV 2026-03-21, AC mV 2026-09-07, three presses with
  the range byte and the AUTO annunciator unmoved), so the sweep offers
  nothing in mV on that model and a rerun cannot produce a mV rung. The
  reason it is dead there does not carry over either: the E+'s mV dial has
  one usable rung, its table's second entry being another model's, so
  "RANGE does nothing" needs no explanation beyond having nowhere to step.
  **The B+'s own evidence points the other way**: on 2026-09-10 RANGE drove
  all six of its two-rung ladders to both rungs — `dcua`, `acua`, `dcma`,
  `acma`, `dca`, `aca`, twelve range sub-steps, every one captured — so two
  rungs are not inherently fixed on that model; mV would have to be
  specially dead.
  Low stakes whichever way it falls: both rungs carry unit `mV`, so no
  reading can be misreported, and the only cost of being wrong is offering a
  control the meter refuses. Issue #7.
- **UT61B+ golden set** — 46 fixtures in
  `crates/dmm-lib/tests/golden/ut61b+/`, lifted from the four issue #19
  reports: every mode the dial reaches, the Ω, DC V and both current
  ladders driven rung by rung, HOLD/REL/MIN/MAX in Ω and DC V, HOLD over a
  forward-biased diode, all three overload shapes, both NCV levels seen,
  mains AC V with the HV warning.
- ~~**Golden YAML fidelity (2026-06 review):**~~ — **DONE** 2026-09-12; every
  UT61E+ golden fixture is now a captured frame. The last two hand-built ones
  went without a new hardware run:
  - `dcv_5.678` → `dcv_battery`, a real 1.6109 V frame off a battery on the
    2.2V rung (2026-09-07, `ut61eplus-verify4-plan.yaml`, step `bat_auto`).
    Its bar-graph bytes were the real defect — `00 00` behind a 5.678 V
    reading on the 22V rung, which the meter cannot send. **This item's
    other stated reason was wrong**: it said the fixture lacked "the
    DC-indicator bit (verified set on real DC V)", but flag3 bit 3 is set
    only in AC+DC V frames, and no real DC V frame in any capture sets it.
    Bit 0 there is bar polarity, correctly clear for a positive reading.
  - `ncv_3` deleted, not replaced. It asserted a frame shape no meter sends:
    a literal ASCII `3` where both meters draw the level as "-" segments
    (`ncv` at one dash on the E+, `ncv_4` at four on the B+, both captured).
    It was the only cover for `parse_measurement`'s numeric-NCV fallback,
    which is a guess at other firmware — that now lives in
    `parse_ncv_numeric_fallback`, where a hand-built payload belongs.
  The third, `ohm_overload`, whose `OL` had no decimal point, was replaced on
  2026-09-11 by the real 220Ω frame from the 82 kΩ run.
- **DC V ranges verified (2026-03-21):** 4 ranges (0=2.2V, 1=22V, 2=220V, 3=1000V).
  The RANGE button cycles 0→1→2→3→0, skipping ranges that would overflow
  the current reading; one rung per press and the 1000V→2.2V wrap were
  re-confirmed 2026-09-07 (see the resolved 0x46 item above). The code
  carried a 5th entry (range 4=220mV) from vendor RE, never observed on the
  UT61E+; it was dropped on 2026-09-07 along with its rows in the family
  spec tables.
  The 220mV capability on the UT61E+ is via DC mV mode (0x03), a separate
  dial position. The UT61B+/D+ tables keep their own shapes.
- **DC mV mode (0x03) is a separate mode, not DC V range 4.** Auto-range
  stays in DC V mode (0x02) even at 100mV. DC mV (0x03) is only reached
  via the mV dial position. On UT61E+, DC mV has only 1 range (range 0 =
  220mV); the RANGE button has no effect. The code's dc_mv range 1 (2.2V)
  may be used by other models.
- **AC mV (0x01): RANGE is dead there too — verified 2026-09-07.** On the
  mV dial in AC mV, `set range 2.2V` pressed RANGE once and re-read three
  times; neither the range byte (220mV throughout) nor the AUTO annunciator
  moved, and the walk gave up with "RANGE did nothing in 220mV". Both mV
  modes are therefore fixed-range on the E+ and the choice list offers no
  range in either. The table's 2.2V entry belongs to another model.

### Mode byte collisions — RESOLVED
Previously documented collisions (0x00=ACV/DCA, 0x02=DCV/hFE, 0x04=Hz/NCV)
were incorrect. Each mode has a unique byte: DCA=0x10, hFE=0x12, NCV=0x14.
Confirmed by real device captures and independently by vendor software
decompilation (see `docs/research/ut61eplus/protocol-comparison.md`).

### VC-890 VOID readings are plotted as valid

`flags.void` means the meter marked a reading invalid (misplug /
reference-disconnect detection). It arrives alongside an ordinary `Normal`
value, and the GUI plots that value, records it and exports it like any
other — so an invalid reading is indistinguishable from a good one
everywhere except the flags column.

Needs a VC-890 to settle two things before changing behaviour: whether the
accompanying value is meaningful at all when VOID is set, and whether
`lead_error` behaves the same way. If the value is meaningless, it should not
be plotted — which makes this a correctness fix rather than a rendering one.

### Entering NCV leaves the previous mode's trace on the graph

`Graph::push_sample` is what detects a mode change and clears history, but in
NCV mode every sample is `MeasuredValue::NcvLevel`, which never reaches it —
the App's `resolve_plot_input` returns `None` for a level with no place on a
value axis, so nothing is pushed. So switching to NCV leaves the previous
mode's data on screen indefinitely, labelled with the old unit.

Reproducible without hardware via the mock's `ncv` scenario. Fixing it means
routing non-plottable samples through something that carries mode/unit, and
establishing the time origin without any plottable points. That is also the
prerequisite for banding NCV — see `docs/future-improvements.md`.

### A meter power cycle surfaces a checksum error

Seen on our UT61E+ (2026-09-17, `dmm-gui`, CP2110): powering the meter off
and back on at a different dial position showed `checksum mismatch: expected
0x3534, got 0x055a` as a UI error. The received value is ASCII `"54"` —
display digits where the checksum should be — while the computed one is a
plausible sum for a whole frame, so the frame boundary was lost rather than
the data corrupted.

`read_frame` leaves `rx_buf` untouched when a read times out
(`crates/dmm-lib/src/protocol/framing.rs`, the `n == 0` arm). A meter that
stops mid-frame leaves a partial frame there; when it comes back, the new
stream is appended to it, so `locate` finds the old `AB CD`, the length byte
points into fresh data, and the bytes at the checksum position are payload.

Recovery already works — the UT61+ propagates the error and clears the buffer
on the way out (the vendor parser's discard-and-clear, ut61eplus spec §2.1),
so the next read is clean. What it costs is one user-visible error for an
ordinary action.

Two candidate fixes: clear `rx_buf` when a read times out, which removes the
cause and also covers UT803/UT804 (the other family that propagates), or
retry once after the clear. A timeout means 2 s without a single byte
(`read_uart_bytes` returns 0 only at the deadline), and no frame we handle
takes that long to arrive, so keeping a partial frame across one buys
nothing. Reproducible without hardware: feed `read_frame` a partial frame, a
transport that returns no bytes once, then a fresh stream.

### GUI accessibility — screen reader walk-through

The GUI accessibility pass wired up AccessKit labels, toggle-state
announcements, focus rings on custom widgets, modal focus trapping, a
text summary on the plot, a polite live region on the primary reading,
and landmark roles (Toolbar / Main / Status).

**Keyboard accessibility — verified.** Manual keyboard-only walk-through
confirmed:

- Tab order is sensible across every panel (top bar, graph toolbar,
  plot, stats, recording, remote controls, settings including the
  expanded Customize colors section).
- Visible focus rings appear on every Tab stop, including the color-
  picker swatches, the graph minimap, the recording-panel resize
  divider, and the left-panel resize handle.
- Arrow-key behaviour: Left/Right pans the minimap when focused;
  arrow keys adjust the saturation/value 2D area and the hue 1D
  gradient inside the color-picker popup; Up/Down resizes the
  recording-panel divider; Left/Right resizes the left panel handle.
- Modal focus trapping: opening the `?` shortcut help moves focus
  inside the modal, Tab cycles within it, and closing (via Esc, the
  × button activated with Space, or clicking outside) restores focus
  to the `?` button that opened it. The version-label → What's New
  viewport follows the same pattern.

**Screen reader walk-through — still pending.** What needs manual
verification:

- **Orca on Linux** (AT-SPI): Tab through every interactive widget and
  confirm each announces a sensible name. Toggle HOLD/REL/RANGE/AUTO/
  MIN-MAX/PEAK/LIVE and confirm "pressed"/"not pressed" is spoken.
  Check that the plot's state summary is read when focused and that
  the main reading updates are announced politely (not continuously).
  Confirm Orca's landmark-nav shortcut (Orca+Ctrl+Shift+L) lists
  Toolbar, Main, Status.
  With a sub-value-capable meter (UT181A, or the mock), also confirm:
  the graph toolbar's **Plot:** chips announce as "Plot \<name\>" radio
  buttons and the **Show:** chips as "Show \<name\> trace" toggles, each
  with its selected/pressed state, so the two rows are told apart by
  ear; the reading's live region speaks the sub-values (and a MIN/MAX
  extreme's "at N seconds") after the mode without flooding while the
  meter streams; and the plot summary's "Also showing …" phrase names
  the drawn traces.
  With a software scale applied (the **Scale** row), also confirm: the
  reading's live region ends with ", software scaled" and drops the
  phrase again when scaling is turned off; the **Scale** button
  announces its on/off state; and the three fields announce as "Scale
  factor", "Offset" and "Unit label" from their hint text.
- **NVDA or JAWS on Windows** (UI Automation): same checks, since
  AccessKit's Windows backend is separate from AT-SPI.
- **VoiceOver on macOS** (NSAccessibility): same checks on the third
  backend.
- **Hover tooltips are invisible to assistive tech.** egui 0.36 never
  calls AccessKit's `set_description`, so `on_hover_text` reaches sighted
  users only — any control whose meaning lives solely in its tooltip is
  unexplained to a screen reader. The toolbar chips work around this by
  folding the group into their accessible name. A follow-up could add an
  `a11y_description` helper mirroring `on_hover_text` and apply it where
  the tooltip carries real information.

Report findings by opening a GitHub issue; the docs should be updated
to reflect what is actually confirmed working and what still needs fixes.

## Completed Verification

| Mode/Feature | Mode byte | Status |
|---|---|---|
| AC V | 0x00 | Verified (open leads + body voltage) |
| AC mV | 0x01 | Verified (mode byte capture) |
| DC V | 0x02 | Verified (open, shorted, body voltage, bench PSU: 1V→2.2V, 5V→22V, 25V→220V ranges) |
| Hz | 0x04 | Verified (mode byte capture) |
| Ω | 0x06 | Verified (OL on open leads; 2.3–3.4 MΩ across the body on the 22MΩ rung, 2026-09-07) |
| Continuity | 0x07 | Verified (OL on open leads) |
| Diode | 0x08 | Verified (OL on open leads) |
| Capacitance | 0x09 | Verified (stray cap reading) |
| DC µA | 0x0C | Verified (PPK2 + 56kΩ: 59µA reading, cross-checked with PPK2 ~61µA) |
| DC mA | 0x0E | Verified (bench PSU: 10mA→22mA range, 100mA→220mA range) |
| DC A | 0x10 | Verified (bench PSU: 100mA, range byte=0x01 for 20A) |
| hFE | 0x12 | Verified (mode byte capture) |
| AC mA | 0x0F | Verified (mA + SELECT) |
| DC A | 0x10 | Verified (A⎓ dial, bench PSU ~100mA, range byte=0x01) |
| AC A | 0x11 | Verified (A⎓ + SELECT) |
| NCV | 0x14 | Verified (`"   EF  "` idle, `"     - "` at a mains cable — one `-` per level, manual §13; two or more segments unobserved) |
| LPF V | 0x18 | Verified (V~ + SELECT, mode byte capture) |
| AC+DC V | 0x19 | Verified (V⎓ + SELECT, mode byte capture) |
| Duty Cycle % | 0x05 | Verified (AC mA + SELECT2, mode byte capture) |
| Mode collisions | — | Disproven: NCV=0x14, hFE=0x12, DCA=0x10 are unique (vendor RE + device) |
| HOLD flag | bit1 of byte11 | Verified (physical + remote) |
| REL flag | bit0 of byte11 | Verified (physical + remote) |
| MIN flag | bit2 of byte11 | Verified (physical) |
| MAX flag | bit3 of byte11 | Verified (physical + remote) |
| AUTO flag | !bit2 of byte12 | Verified (inverted logic) |
| HV warning | bit0 of byte12 | Verified (>30V per manual; confirmed set at 31V on DC V) |
| LOW BAT | bit1 of byte12 | Verified (intermittent) |
| Remote HOLD | 0x4A | Verified |
| Remote REL | 0x48 | Verified |
| Remote MIN/MAX | 0x41 | Verified |
| Remote Exit MIN/MAX | 0x42 | Verified |
| Remote RANGE | 0x46 | Verified 2026-09-07 on UT61E+: from auto the first press engages manual on the rung already showing, each further press steps one rung up, 1000V wraps to 2.2V; the mode byte never moves |
| Remote AUTO | 0x47 | Verified 2026-09-07: restores auto-ranging from a manual rung in one command |
| Remote SELECT | 0x4C | Verified on every dial position (§3.1 of the ut61-family spec: V⎓, V~, mV, Ω, µA, mA, A rings; inert on hFE, NCV) |
| Remote mode switching (`dmm-cli set mode`, GUI dropdown) | 0x4C / 0x49 | Verified 2026-09-07: every listed entry on every UT61E+ dial position, one press per leg, junction crossings, under MIN/MAX, and under HOLD for SELECT; 2026-09-14: a Hz/% press under HOLD does nothing (ut61-family spec §6.3); settle 150 ms / 3 reads |
| Remote LIGHT | 0x4B | Verified |
| Remote SELECT2 | 0x49 | Verified (AC V/mV/µA/mA/A → Hz → Duty Cycle → back; Hz ↔ Duty on the Hz/% dial; inert on V⎓, DC mV, hFE, NCV) |
| Remote Peak MIN/MAX | 0x4D | Verified (activates on AC mV and, 2026-09-07, on AC V; context-dependent, no effect on DC V) |
| Remote Exit Peak | 0x4E | Verified (clears peak flags, returns to live readings; used by `set peak off`, 2026-09-07) |
| Remote range/flag setting (`dmm-cli set range`/`hold`/`rel`/`minmax`/`peak`) | 0x46-0x4E | Verified 2026-09-07 on UT61E+: one press per step, each confirmed by read-back; a stale frame is re-read, not re-pressed; MIN/MAX and Peak leave by 0x42 / 0x4E |
| Mode and range switches under HOLD | 0x49 + 0x4A | Verified 2026-09-14 on UT61E+: from held AC V, `set mode Hz` pressed Hz/%, saw nothing change over three reads, pressed HOLD off and Hz/% again, and landed in Hz; `set mode "LPF V"`, `set range 22V` and `set range auto` under HOLD took their own presses with no release; `set rel on` under HOLD was refused and HOLD left lit |
| Modes with no range choice (UT61E+) | — | Verified 2026-09-07: `get` prints no range row in DC mV, AC mV, DC A or AC A |
| Capture steps `dcv_negative`, `ohm_body`, `acdcv`, `lpfv`, `acmv`, `acua`, `acma`, `aca` | — | Verified 2026-09-07 on UT61E+ (captures 4 and 5): sign on a AAA battery, body resistance, and each SELECT sub-mode read back by the tool with open leads |
| Get Name | 0x5F | Verified (two-frame response: ack FF 00 + ASCII name) |
| MIN/MAX flag cycling | byte11 bits 2-3 | Verified: MAX only (bit 3) → MIN only (bit 2), 2-state cycle, never both set |
| MIN/MAX value reporting | — | Verified: meter sends stored min/max value, not live reading |
| Peak flag cycling | byte13 bits 1-2 | Verified: P-MAX only (bit 2) → P-MIN only (bit 1), 2-state cycle |
| Peak value reporting | — | Verified: meter sends stored instantaneous peak, not live/RMS |
| Bar graph encoding | bytes 9-10 | Verified: decimal (b9*10+b10), ~46 segments. Negative: bar_pol flag. OL: 44. |
| Bar polarity | bit0 of byte13 | Verified (set on negative readings) |
| DC indicator | bit3 of byte13 | Verified (set on DC V, clear on AC mV) |
| DC V range table | ranges 0-3 | Verified: 0=2.2V, 1=22V, 2=220V, 3=1000V (4 ranges, not 5) |
| DC mV mode | 0x03 | Verified: separate mode via dial, range 0=220mV only on UT61E+; RANGE has no effect |
| AC mV range | 0x01 | Verified 2026-09-07: fixed at 220mV — 3 RANGE presses moved neither the range byte nor AUTO |
| Command ack frames | — | Verified (2-byte payload after commands, skipped in measurement path) |
| Frame format | len includes checksum | Verified (19 bytes total) |
| Checksum | 16-bit BE sum | Verified |
| CP2110 Get Version Info | report 0x46 | Verified (part=0x0A, firmware=1) |
| CP2110 Get UART Status | report 0x42 | Verified (TX/RX FIFO=0, no errors at idle) |
| CP2110 UART Config 9 bytes | report 0x50 | Verified (removed trailing 0x00, meter responds normally) |
| CP2110 Set Reset Device | report 0x40 | Rejected — HID protocol error, likely locked out by UNI-T |
| CP2110 read path (stack buffer) | — | Verified 2026-07-29: 50 consecutive reads, no skipped frames |
| Paced-read loop (cancellable sleep) | — | Verified 2026-07-29: pacing intact over 50 reads; Ctrl-C responsiveness still untested |
| Idle HID report handling | — | Verified 2026-07-29 on CP2110: no false timeouts over 50 reads |
