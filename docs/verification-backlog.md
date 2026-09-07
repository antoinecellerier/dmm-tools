# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Pending Verification

### UT61+ remote mode selection (cycle-to-target)

Shipped 2026-09-07: `dmm-cli get mode`/`set mode` and the GUI's mode
dropdown work on the UT61+/UT161 family. The meter takes no set-mode command, so the driver
(`crates/dmm-lib/src/protocol/cycle.rs`) presses SELECT (0x4C) or Hz/%
(0x49) and re-reads the mode byte until the target shows, planning from a
per-model dial table recorded in
`docs/research/ut61-family/reverse-engineered-protocol.md` §3.1. The UT61E+
table, its ring orders and the settle timing were verified on the in-house
meter the same day, every position and every entry (Completed table below).

Observed 2026-09-07 on the in-house E+, and deliberate: once the meter is in
Hz (0x04), `get mode` lists only `Hz, Duty %` and `set mode "AC V"` is
refused as unknown there. The mode byte carries no dial information, and the
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

**Verified the same day on the in-house UT61E+**, leads open, V⎓ and V~ dial
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
evidence about the meter. Left to check on the in-house meter: AC µA and
AC mA, and AC+DC V and LPF V, which the code refuses with `peak cannot be
set in <mode> on this meter`. Every mode where the meter reacts but the
list is empty (or the reverse) is a table fix.

The UT61B+ is offered no Peak at all, from the family spec's flag matrix
(§4, blank Peak cells) and command matrix (§6, "No effect"); the UT61D+ is
offered the same AC modes as the E+. Both unverified — issue #7.

Where the buttons do nothing — observed 2026-09-07 on the in-house UT61E+,
each sweep ending in `<button> did nothing`:

- **REL:** AC+DC V (open leads), Hz, Duty % and NCV.
- **MIN/MAX:** continuity, diode, capacitance, Hz, Duty % and NCV.
- **HOLD:** NCV.
- **RANGE:** capacitance and Hz, single-range on this model.
- **AUTO:** LPF V — the meter came up in 1000V manual and stayed there,
  although the manual's AC V table lists LPF on every range.

The manual (§VII) gives each button one line and no per-function list, so
this is the only record. `choices()` still offers these settings there;
narrowing it to what the meter accepts is a separate pass.

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

**Voltcraft VC-890**:
- Polled communication model (0x5E request → live data response)
- Frame extraction (66-byte, AB CD header, BE16 checksum)
- Function code mapping (19 codes, 0x00-0x12, remapped from VC-880!)
- 60,000 count range values (6/60/600 vs 4/40/400)
- 7 display value fields (main + 6 sub-displays) — format and content
- Status flag bytes (8 bytes at msg[56..63]) — all bit positions correct?
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
  the `cycle:` lines
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
  `RUST_LOG=dmm_lib=debug` and paste the `cycle:` lines
- `dmm-cli --device vc880 capture` exercises all of the above on its own
  since 2026-09-07: once the gate steps pass, every mode step is followed by
  `set range`/`hold`/`rel`/`minmax` through each value, filed as
  `<mode>/<setting>:<label>` sub-steps carrying what the meter read back.
  Report any sub-step with `status: error` and the
  `remote control unreliable on this meter` line if it appears — those are
  the commands this meter refused

**UT803 / UT804 (CH9325 HID, proprietary FS9721 framing)** — IMPLEMENTED, NEEDS HARDWARE VERIFICATION:
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
  - UT803 HOLD = nibble 9 bit 3. UT804 HOLD wire encoding is unknown
    (in neither vendor parser).
- Transport: CH9325 HID at 2400 baud — implemented.
- **Needs hardware verification** (all of the above is decompile-derived):
  - One frame per dial position on each meter (settles mode codes and
    decimal tables in one pass)
  - A negative reading (sign bits) and an overload (OL patterns)
  - MIN/MAX/REL/low-battery toggles — candidates: nibbles 12-14,
    UT803 nibble 9 bits 2-1, UT804 nibble 9 bits 3/1
  - UT804 modes 0xE (unknown glyph; hFE?) and 0xF ("mA%") dial
    positions; which of modes 1/2 each V dial sends
  - UT803 frequency range 0 decimal position; tachometer (RPM) frames
  - Whether 0x5A trigger byte helps/hurts; streaming rate
  - Three parser behaviours surfaced by the 2026-09 snapshot tests, to
    settle against real frames rather than change blind: `range_label`
    is never set for either model although the per-mode tables know the
    range; a UT804 "L0" frame (digit 1 = 0xA, digit 2 != 0xC) is reported
    as `Normal(0.0)` with the display text "L0", which CSV/JSON export as
    the string `L0`; and UT804 `acdc == 3` (AC+DC) sets the DC flag.
- See `docs/research/ut803/reverse-engineered-protocol.md` for full spec.
- UT805A uses USB-to-serial (virtual COM port, NOT HID) with a fully
  documented ASCII text protocol (9600/8N1, bidirectional). Needs serial
  transport — separate scope from HID-based meters.

**UT8802 / UT8802N**:
- Frame extraction (8-byte, 0xAC header, no checksum)
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
  (auto-range, HV warning) alongside it. Regression frames in
  `crates/dmm-lib/src/protocol/ut181a/mod.rs`.
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

Resolved 2026-09-07 on the in-house UT61E+ (CP2110 cable,
`RUST_LOG=dmm_lib=debug`) with `dmm-cli get range` / `set range`, which press
0x46 and re-read the range byte until the target rung shows.

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
- **AC V top range: 750V vs 1000V conflict (2026-06 review).** The
  UT61+ Series manual's AC tables end at 1000V (E+ column: 1000.0V),
  but the code uses 750V for E+/B+/D+ (from the vendor decompile).
  Testable on the in-house UT61E+: dial AC V, manual-range up to the
  top range, and read the range byte + display.
- **UT61D+ amps: manual lists 6.000A and 20.00A; code has only 20A**
  (`ut61d_plus.rs` dc_a/ac_a copied from E+). Needs the 6A range row;
  blocked on D+ hardware for index ordering (issue #7).
- **UT61B+/D+ frequency ranges in code are invented structure** — the
  manual gives only a 10.00 Hz–10.00 MHz span, no discrete ranges, and
  the code's five ranges top out at 600 kHz. Issue #7.
- **UT61B+/D+ "[DEDUCED] ascending" range-index ordering is
  unverifiable from the manual** and is in tension with the only
  verified family data point (E+ puts 220mV at index 4, after the
  V ranges). Issue #7.
- **Golden YAML fidelity (2026-06 review):** the three UT61E+ golden
  captures look synthetic — the DC V case lacks the DC-indicator bit
  (verified set on real DC V) and bar-graph bytes are 00 00 despite
  non-zero readings. Re-capture from the real meter
  (`dmm-cli capture`) so the goldens match verified device behavior.
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
- **Hover tooltips are invisible to assistive tech.** egui 0.34 never
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
| Ω | 0x06 | Verified (OL on open leads) |
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
| Remote mode switching (`dmm-cli set mode`, GUI dropdown) | 0x4C / 0x49 | Verified 2026-09-07: every listed entry on every UT61E+ dial position, one press per leg, junction crossings, under HOLD and MIN/MAX; settle 150 ms / 3 reads |
| Remote LIGHT | 0x4B | Verified |
| Remote SELECT2 | 0x49 | Verified (AC V/mV/µA/mA/A → Hz → Duty Cycle → back; Hz ↔ Duty on the Hz/% dial; inert on V⎓, DC mV, hFE, NCV) |
| Remote Peak MIN/MAX | 0x4D | Verified (activates on AC mV and, 2026-09-07, on AC V; context-dependent, no effect on DC V) |
| Remote Exit Peak | 0x4E | Verified (clears peak flags, returns to live readings; used by `set peak off`, 2026-09-07) |
| Remote range/flag setting (`dmm-cli set range`/`hold`/`rel`/`minmax`/`peak`) | 0x46-0x4E | Verified 2026-09-07 on UT61E+: one press per step, each confirmed by read-back; a stale frame is re-read, not re-pressed; MIN/MAX and Peak leave by 0x42 / 0x4E |
| Modes with no range choice (UT61E+) | — | Verified 2026-09-07: `get` prints no range row in DC mV, AC mV, DC A or AC A |
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
