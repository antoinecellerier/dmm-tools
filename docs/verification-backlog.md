# Protocol Verification Backlog

Items that need real components or specific setups to verify.

## Pending Verification

### Modes not yet tested with real signals
- **DC mV (0x03):** Needs small DC voltage source. Currently only tested as auto-range from DC V.
- **AC µA (0x0D):** Needs AC current source.
- **AC mA (0x0F):** Needs AC current source.
- **DC A (0x10):** Needs high-current circuit. Note: A range is 20A per manual (our table says 10A — needs fix).
- **AC A (0x11):** Needs high-current circuit.
- **Temperature °C (0x0A):** Needs K-type thermocouple.
- **Temperature °F (0x0B):** Needs K-type thermocouple.
- **Duty Cycle % (0x05):** Needs PWM signal source.
- **AC+DC V (0x19):** Needs AC+DC signal. SELECT cycles to it on V⎓ dial.
- **LPF V (0x18):** Low pass filter mode. Needs AC signal.
- **LPF mV (0x1A), LPF A (0x1C):** Need appropriate signals.
- **AC+DC mV (0x1B), AC+DC A (0x16, 0x17, 0x1D):** Need appropriate signals.
- **Live (0x13):** Unknown purpose.
- **LoZ V (0x15):** Low impedance ACV (UT61D+ feature, may not apply to E+).
- **Inrush (0x1E):** Inrush current mode.

### Commands not fully verified
- **SELECT2 (0x49):** Received by meter (beeps) but no visible effect on DC V mode. Likely needs AC V mode for Hz/duty cycle display.
- **Peak MIN/MAX (0x4D):** Received by meter (beeps) but no visible effect on DC V mode. May need active signal or specific mode.
- **Exit Peak (0x4E):** Sent but not confirmed — need to first activate peak mode.
- **Get Name (0x5F):** Not tested — need to parse response format.

### Range tables
- Range byte values for each mode need verification against real device at each range.
- A range is 20A per manual, our table says 10A — **confirmed bug, needs fix**.
- DC mV mode (0x03) ranges not verified — does it share tables with DC V range 0?

### Mode byte collisions
Three mode byte values are shared between different functions:
- **0x00:** AC V and DC A — need range byte or context to distinguish.
- **0x02:** DC V and hFE — no protocol-level distinction known.
- **0x04:** Hz and NCV — display content ("EF") distinguishes NCV.

These collisions need further investigation. The reference implementations don't address them.

## Completed Verification

| Mode/Feature | Mode byte | Status |
|---|---|---|
| AC V | 0x00 | Verified (open leads + body voltage) |
| AC mV | 0x01 | Verified (mode byte capture) |
| DC V | 0x02 | Verified (open, shorted, body voltage) |
| Hz | 0x04 | Verified (mode byte capture) |
| Ω | 0x06 | Verified (OL on open leads) |
| Continuity | 0x07 | Verified (OL on open leads) |
| Diode | 0x08 | Verified (OL on open leads) |
| Capacitance | 0x09 | Verified (stray cap reading) |
| DC µA | 0x0C | Verified (mode byte capture) |
| DC mA | 0x0E | Verified (mode byte capture) |
| hFE | 0x12 | Verified (mode byte capture) |
| NCV | 0x14 | Verified (EF display) |
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
| Frame format | len includes checksum | Verified (19 bytes total) |
| Checksum | 16-bit BE sum | Verified |
