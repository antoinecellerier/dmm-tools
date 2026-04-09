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
- Misplug warning nibble (msg[63]) — 0=none, 1=mA err, 2=A err
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

**UT8802 / UT8802N**:
- Frame extraction (8-byte, 0xAC header, no checksum)
- 0x5A streaming trigger byte
- Position code mapping (35 codes, 0x01-0x2D with gaps)
- BCD display encoding (5 nibbles from bytes 2-4)
- Decimal point position (byte 5 low nibble, 0-4)
- AC/DC coupling flags (byte 5 bits 4-5)
- Sign/polarity (byte 7 bit 7)
- AUTO flag inverted logic (byte 7 bit 2 clear = auto ON)
- Byte 7 flag bits: exact HOLD/REL/MAX/MIN positions [UNVERIFIED] —
  current assignments (bits 6/5/4/3) are best-guess from Ghidra
- Byte 6 purpose: bargraph or secondary status? [UNVERIFIED]
- Overload detection (BCD nibble 0x0C)
- Streaming rate

**UT8803 / UT8803E** ([issue #3](https://github.com/antoinecellerier/dmm-tools/issues/3)):
- Frame extraction (21-byte, AB CD header, BE checksum)
- 0x5A streaming trigger byte
- Mode byte mapping (23 position codes, 0x00-0x16)
- Range byte (0x30 prefix, like UT61E+)
- Display bytes (5 raw bytes — ASCII or binary encoding?)
- Flag byte → semantic flag mapping (HOLD, REL, MIN, MAX, AUTO, OL).
  The RE spec documents the constructed 32-bit status word, but the
  raw-byte-to-status-word construction is complex. Current bit assignments
  are plausible guesses — need real device verification.
- Display value parsing (5 bytes → float)
- Streaming rate (~2-3 Hz per manual)

**UT171A / UT171B / UT171C** ([issue #4](https://github.com/antoinecellerier/dmm-tools/issues/4)):
- Frame extraction (1-byte length, LE checksum)
- Connect command (`AB CD 04 00 0A 01 0F 00`) — may be needed before streaming
- Mode byte mapping (26 modes, 0x01-0x24)
- Float32 LE value parsing
- Flags byte (HOLD bit 7, AUTO bit 6 inverted, Low Battery bit 2)
- Range byte (raw, 1-based)
- Extended frame (28 bytes, frame type 0x03) — not yet parsed
- Status2 byte (offset 13) meaning
- Aux value interpretation

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
- Mode word decoding (97 nibble-encoded uint16 modes) — only 0x3111
  (V DC) verified so far
- Device-sent unit string parsing — only "VDC" verified so far
- Misc byte format variants (normal, relative, min/max, peak)
- Misc2 flags (auto-range, HV warning, lead error, COMP, record)
- Measurement packet variants beyond normal format

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
decompilation (see `references/protocol-comparison.md`).

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
