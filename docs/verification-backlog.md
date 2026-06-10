# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Pending Verification

### Modes not yet tested with real signals

Tracked in [issue #6](https://github.com/antoinecellerier/dmm-tools/issues/6).

- **DC mV (0x03):** Needs small DC voltage source. Currently only tested as auto-range from DC V.
- **AC µA (0x0D):** Needs AC current source.
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
- Battery level nibble (msg[62]) — what do the values mean?
- Misplug warning nibble (msg[63]) — 0=none, 1=mA err, 2=A err, 3=V err
- ACV LPF (0x01) range byte — vendor ignores it and fixes 1000V (2026-06
  review, DMSShare_decompiled.cs:23466); what does the meter send there?
- Inbound checksum — the vendor never validates meter→host checksums; our
  BE16 check is inferred from the host-side builder. If real frames are
  all rejected with ChecksumMismatch, suspect a different inbound scheme.
- Command confirmation frames — vendor `SendCommand` waits for a frame
  whose type byte equals the command byte (5 retries); we fire-and-forget.
- Ack protocol (0xFF+\[0x00\] after responses) — is it required or optional?
- Commands: same as VC-880 plus 0x5D (Set Time) and 0x5E (Get Measurement)
- PC button activation requirement

**Voltcraft VC-880 / VC650BT**:
- Frame extraction (39-byte, AB CD header, BE16 checksum — same as UT61E+)
- Streaming model (no trigger, auto-starts after PC button press)
- Function code mapping (19 codes, 0x00-0x12) — do mode labels match LCD?
- Range byte (0x30-based ASCII) — correct range values per function?
- Main display (7 ASCII bytes) — values match LCD?
- Sub-displays (sub1, sub2, bar) — format and content
- Status flag bytes (7 bytes, 28 named flags) — all bit positions correct?
- Overload detection (OL1 flag + "OL" in display string)
- Commands: hold (0x4A), rel (0x48), range_auto (0x47), range_manual (0x46),
  max_min_avg (0x49), light (0x4B), select (0x4C)
- Streaming rate (manual says 2-3 Hz)
- PC button activation requirement
- VC650BT compatibility (same protocol confirmed by installer comparison)

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

**UT181A** ([issue #5](https://github.com/antoinecellerier/dmm-tools/issues/5)):
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
- Mode word decoding (79 nibble-encoded uint16 modes) — only 0x3111
  (V DC) verified so far. **Corrections from the 2026-06 review** (per
  sigrok + antage, hardware pending): DC-current n1=2 codes
  (0x8121/0x9121/0xA121) are AC+DC, not Hz; 0x4121 = mV DC Peak (sigrok
  notes 0x4131 as a possible alternative — check on hardware); 0x5212 =
  Continuity open-beeper and 0x6112 = Diode Alarm (not REL variants);
  temperature n1 selects the display arrangement (T1(T2)/T2(T1)/
  T1-T2/T2-T1). HOLD command now sends `[0x12, 0x5A]` (antage's
  button-code form) — confirm it toggles HOLD. COMP digits read from
  the low nibble unshifted. Need at least one mode per family to confirm
  the nibble decoder works broadly.
- Device-sent unit string parsing — only "VDC" verified so far
- Relative format (0x10) parsing — implemented, needs hardware verification
  (delta/reference/absolute values parsed into main + aux_values)
- Min/Max format (0x20) parsing — implemented, needs hardware verification
  (current/max/avg/min with timestamps parsed into main + aux_values)
- Peak format (0x40) parsing — implemented, needs hardware verification
  (peak max/min parsed into main + aux_values)
- COMP mode extension parsing — implemented, needs hardware verification
  (comp mode/result/limits parsed into aux_values)
- Range label lookup table — implemented, needs verification that range
  byte values match expected labels in manual range mode
- Misc2 flags: lead_error (bit 3), comp (bit 4), record (bit 5) — now
  parsed but not yet verified on real hardware
- **Not implemented**: recording protocol (0x0A-0x0F), saved measurement
  retrieval (0x07-0x09), SET_MODE/SET_REFERENCE commands, timestamp
  decoding, response types 0x03/0x04/0x05/0x72

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
  the current reading. The code has a 5th entry (range 4=220mV) from vendor
  RE — this may be used by other models (UT61B+/D+) but was never observed
  on the UT61E+. The 220mV capability on the UT61E+ is via DC mV mode (0x03),
  a separate dial position.
- **DC mV mode (0x03) is a separate mode, not DC V range 4.** Auto-range
  stays in DC V mode (0x02) even at 100mV. DC mV (0x03) is only reached
  via the mV dial position. On UT61E+, DC mV has only 1 range (range 0 =
  220mV); the RANGE button has no effect. The code's dc_mv range 1 (2.2V)
  may be used by other models.

### Mode byte collisions — RESOLVED
Previously documented collisions (0x00=ACV/DCA, 0x02=DCV/hFE, 0x04=Hz/NCV)
were incorrect. Each mode has a unique byte: DCA=0x10, hFE=0x12, NCV=0x14.
Confirmed by real device captures and independently by vendor software
decompilation (see `docs/research/ut61eplus/protocol-comparison.md`).

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
- **NVDA or JAWS on Windows** (UI Automation): same checks, since
  AccessKit's Windows backend is separate from AT-SPI.
- **VoiceOver on macOS** (NSAccessibility): same checks on the third
  backend.

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
| NCV | 0x14 | Verified (EF display) |
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
| Remote RANGE | 0x46 | Verified |
| Remote AUTO | 0x47 | Verified |
| Remote SELECT | 0x4C | Verified (cycles DC V → AC+DC) |
| Remote LIGHT | 0x4B | Verified |
| Remote SELECT2 | 0x49 | Verified (AC mV: cycles AC mV → Hz → Duty Cycle → AC mV) |
| Remote Peak MIN/MAX | 0x4D | Verified (activates on AC mV; context-dependent, no effect on DC V) |
| Remote Exit Peak | 0x4E | Verified (clears peak flags, returns to live readings) |
| Get Name | 0x5F | Verified (two-frame response: ack FF 00 + ASCII name) |
| MIN/MAX flag cycling | byte11 bits 2-3 | Verified: MAX only (bit 3) → MIN only (bit 2), 2-state cycle, never both set |
| MIN/MAX value reporting | — | Verified: meter sends stored min/max value, not live reading |
| Peak flag cycling | byte13 bits 1-2 | Verified: P-MAX only (bit 2) → P-MIN only (bit 1), 2-state cycle |
| Peak value reporting | — | Verified: meter sends stored instantaneous peak, not live/RMS |
| Bar graph encoding | bytes 9-10 | Verified: decimal (b9*10+b10), ~46 segments. Negative: bar_pol flag. OL: 44. |
| Bar polarity | bit0 of byte13 | Verified (set on negative readings) |
| DC indicator | bit3 of byte13 | Verified (set on DC V, clear on AC mV) |
| DC V range table | ranges 0-3 | Verified: 0=2.2V, 1=22V, 2=220V, 3=1000V (4 ranges, not 5) |
| DC mV mode | 0x03 | Verified: separate mode via dial, range 0=220mV only on UT61E+ |
| Command ack frames | — | Verified (2-byte payload after commands, skipped in measurement path) |
| Frame format | len includes checksum | Verified (19 bytes total) |
| Checksum | 16-bit BE sum | Verified |
| CP2110 Get Version Info | report 0x46 | Verified (part=0x0A, firmware=1) |
| CP2110 Get UART Status | report 0x42 | Verified (TX/RX FIFO=0, no errors at idle) |
| CP2110 UART Config 9 bytes | report 0x50 | Verified (removed trailing 0x00, meter responds normally) |
| CP2110 Set Reset Device | report 0x40 | Rejected — HID protocol error, likely locked out by UNI-T |
