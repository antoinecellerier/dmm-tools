# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Pending Verification

### Modes not yet tested with real signals
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

**UT8803 / UT8803E:**
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

**UT171A / UT171B / UT171C:**
- Frame extraction (1-byte length, LE checksum)
- Connect command (`AB CD 04 00 0A 01 0F 00`) — may be needed before streaming
- Mode byte mapping (26 modes, 0x01-0x24)
- Float32 LE value parsing
- Flags byte (HOLD bit 7, AUTO bit 6 inverted, Low Battery bit 2)
- Range byte (raw, 1-based)
- Extended frame (28 bytes, frame type 0x03) — not yet parsed
- Status2 byte (offset 13) meaning
- Aux value interpretation

**UT181A:**
- Frame extraction (2-byte LE length, LE checksum)
- Mode word decoding (97 nibble-encoded uint16 modes)
- Float32 LE value parsing with precision byte
- Device-sent unit string parsing
- Misc byte format variants (normal, relative, min/max, peak)
- Misc2 flags (auto-range, HV warning, lead error, COMP, record)
- Measurement packet variants beyond normal format

### CP2110 feature reports (AN434)
- (none pending)

### Commands not fully verified
- **SELECT2 (0x49):** Received by meter (beeps) but no visible effect on DC V mode. Likely needs AC V mode for Hz/duty cycle display.
- **Peak MIN/MAX (0x4D):** Received by meter (beeps) but no visible effect on DC V mode. May need active signal or specific mode.
- **Exit Peak (0x4E):** Sent but not confirmed — need to first activate peak mode.
- **Get Name (0x5F):** Verified — returns two frames: ack (FF 00) then ASCII name (e.g. "UT61E+").

### MIN/MAX and Peak measurement reporting
- **What does the meter send over USB during MIN/MAX mode?** The 14-byte
  measurement payload has no dedicated min/max value fields. The meter LCD
  shows min and max values, but the protocol likely still sends the live
  measurement with the MIN/MAX flags set. Need to verify: capture data with
  MIN/MAX active and a varying signal, confirm the reported value is the
  live reading (not the stored min or max).
- **Do the MIN and MAX flag bits cycle independently?** The meter's LCD
  cycles through showing MIN, MAX, and AVG when pressing the MIN/MAX button
  repeatedly. But the protocol might always set both bits together (both
  are set in existing captures). Verify whether the meter ever reports
  only-MIN or only-MAX over USB, or if the display cycling is LCD-only.
- **Same question for Peak mode:** Does the meter report the live value with
  peak flags, or does it report the captured peak value?
- The mock currently sends the live waveform with flags set, which matches
  the assumed protocol behavior. Update the mock if real device shows
  otherwise.

### Range tables
- Range byte values for each mode need verification against real device at each range.
- DC mV mode (0x03) ranges not verified — does it share tables with DC V range 0?

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
| LOW BAT | bit1 of byte12 | Verified (intermittent) |
| Remote HOLD | 0x4A | Verified |
| Remote REL | 0x48 | Verified |
| Remote MIN/MAX | 0x41 | Verified |
| Remote Exit MIN/MAX | 0x42 | Verified |
| Remote RANGE | 0x46 | Verified |
| Remote AUTO | 0x47 | Verified |
| Remote SELECT | 0x4C | Verified (cycles DC V → AC+DC) |
| Remote LIGHT | 0x4B | Verified |
| Get Name | 0x5F | Verified (two-frame response: ack FF 00 + ASCII name) |
| Command ack frames | — | Verified (2-byte payload after commands, skipped in measurement path) |
| Frame format | len includes checksum | Verified (19 bytes total) |
| Checksum | 16-bit BE sum | Verified |
| CP2110 Get Version Info | report 0x46 | Verified (part=0x0A, firmware=1) |
| CP2110 Get UART Status | report 0x42 | Verified (TX/RX FIFO=0, no errors at idle) |
| CP2110 UART Config 9 bytes | report 0x50 | Verified (removed trailing 0x00, meter responds normally) |
| CP2110 Set Reset Device | report 0x40 | Rejected — HID protocol error, likely locked out by UNI-T |
