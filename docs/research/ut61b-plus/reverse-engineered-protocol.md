# UT61B+ Protocol: Reverse-Engineered Specification

Based on:
- UT61+ Series User Manual (UNI-T, covers UT61B+/UT61D+/UT61E+)
- CP2110 Datasheet (Silicon Labs)
- AN434: CP2110/4 Interface Specification (Silicon Labs)
- UNI-T UT61E+ Software V2.02 (decompiled with Ghidra — same software
  handles all three models)

Confidence levels:
- **[KNOWN]** — established facts from official Silicon Labs documentation
- **[VENDOR]** — confirmed by decompiling UNI-T's official Windows software
- **[MANUAL]** — stated in UNI-T's official UT61+ Series User Manual
- **[DEDUCED]** — logical inferences not yet verified against real hardware
- **[UNVERIFIED]** — requires real device testing to confirm

---

## 1. Protocol Summary

**The UT61B+ uses the identical wire protocol as the UT61E+.** All
transport, framing, command, and response formats are the same. The
vendor software contains no model-specific protocol code (see
`reverse-engineering-approach.md` for evidence).

This document focuses on documenting the **application-layer
differences**: which mode bytes the UT61B+ sends, what range values
it uses, and what features are absent compared to the UT61E+.

For the complete protocol specification (transport, framing, commands,
response parsing), see `docs/research/ut61eplus/reverse-engineered-protocol.md`.

---

## 2. Shared Protocol (Same as UT61E+)

The following are identical across all UT61+ models — [VENDOR]:

| Aspect | Value |
|--------|-------|
| USB VID/PID | 0x10C4 / 0xEA80 |
| Baud rate | 9600 bps, 8N1, no flow control |
| Frame header | 0xAB 0xCD |
| Length byte | payload + 2 (includes checksum) |
| Checksum | 16-bit big-endian sum of all preceding bytes |
| Command format | AB CD 03 cmd chk_hi chk_lo |
| GetMeasurement | 0x5E |
| Response size | 19 bytes total (3 header + 14 payload + 2 checksum) |
| Mode byte | offset 3, raw (no masking) |
| Range byte | offset 4, 0x30 prefix (mask with & 0x0F) |
| Display bytes | offsets 5-11, 7 ASCII characters |
| Bar graph bytes | offsets 12-13, raw |
| Flag bytes | offsets 14-16, 0x30 prefix (mask with & 0x0F) |
| Polling model | Request/response (LoopCommandPool) |

---

## 3. Model-Specific Differences

### 3.1 Display Resolution — [MANUAL]

| Model | Max Count | Digits | Resolution |
|-------|-----------|--------|------------|
| UT61B+ | 6,000 | 3¾ | 0.01mV (mV range) |
| UT61E+ | 22,000 | 4¼ | 0.01mV (mV range) |

The 7-byte ASCII display field in the protocol is the same size for
both models. The UT61B+ simply uses fewer significant digits. For
example, a DC voltage reading:
- UT61E+ might display: `" 2.1987"` (22,000 counts, 220mV range)
- UT61B+ might display: `" 5.997"` (6,000 counts, 6V range)

The display field format and parsing are identical — the difference is
purely in the values the meter firmware generates.

### 3.2 Bar Graph — [MANUAL]

| Model | Segments | Update Rate |
|-------|----------|------------|
| UT61B+ | 31 | 30 times/sec |
| UT61E+ | 46 | 30 times/sec |

The bar graph bytes (offsets 12-13) use the same encoding. The UT61B+
firmware generates values in the range 0-31 instead of 0-46.
**[DEDUCED]** — encoding not verified on real UT61B+ hardware.

### 3.3 Current Measurement Range — [MANUAL]

| Model | Max Current (A range) |
|-------|-----------------------|
| UT61B+ | 10A |
| UT61E+ | 20A |

The UT61B+ has a 10A fuse and reports a maximum of 10.00A. The UT61E+
has a 20A fuse. This affects the range table entries for DC A and AC A
modes.

### 3.4 Frequency Bandwidth — [MANUAL]

| Model | AC Voltage Bandwidth | AC Current Bandwidth |
|-------|---------------------|---------------------|
| UT61B+ | 40Hz-500Hz | 40Hz-500Hz |
| UT61E+ | 40Hz-10kHz | 40Hz-10kHz |

This is a firmware/hardware difference. The protocol carries the same
frequency measurement data regardless.

---

## 4. Available Modes — [MANUAL + VENDOR]

The vendor software's mode table contains entries for ALL modes across
all three models. The UT61B+ meter firmware only uses a subset. The
mode bytes are the same as the UT61E+.

### 4.1 Modes Present on UT61B+

| Mode Byte | Mode | UT61B+ Dial Position |
|-----------|------|---------------------|
| 0x00 | AC V | V~ (with SELECT for DC) |
| 0x01 | AC mV | mV (via SELECT) |
| 0x02 | DC V | V~ + SELECT |
| 0x03 | DC mV | mV position |
| 0x04 | Hz | Hz% position, or via SELECT2 on V~/mV/A |
| 0x05 | Duty % | Hz% position (via SELECT) |
| 0x06 | Resistance | Ω position |
| 0x07 | Continuity | Ω position (via SELECT on UT61B+) |
| 0x08 | Diode | Diode position |
| 0x09 | Capacitance | Diode position (via SELECT on UT61B+) |
| 0x0C | DC µA | µA position |
| 0x0D | AC µA | µA position (via SELECT) |
| 0x0E | DC mA | mA position |
| 0x0F | AC mA | mA position (via SELECT) |
| 0x10 | DC A | A position |
| 0x11 | AC A | A position (via SELECT) |
| 0x14 | NCV | NCV position |

### 4.2 Modes NOT Available on UT61B+

| Mode Byte | Mode | Why absent |
|-----------|------|------------|
| 0x0A | Temperature °C | No thermocouple input (UT61D+ only) |
| 0x0B | Temperature °F | No thermocouple input (UT61D+ only) |
| 0x12 | hFE | No adapter socket (UT61E+ only) |
| 0x13 | Live | [UNVERIFIED] — may not be on UT61B+ |
| 0x15 | LoZ V | No LoZ mode on dial (UT61D+ only) |
| 0x16 | LoZ V (2) | No LoZ mode on dial (UT61D+ only) |
| 0x17 | LPF | No LPF mode on dial (UT61E+ only) |
| 0x18 | LPF V | No LPF mode on dial (UT61E+ only) |
| 0x19 | AC+DC V | No AC+DC mode on dial (UT61E+ only) |

**[VENDOR]** The vendor software will accept and decode ANY mode byte
from any model. If a UT61B+ were to send mode 0x0A (temperature), the
software would decode it correctly. The filtering happens in the
meter's firmware, not the PC software.

### 4.3 Feature Differences in Flag Bytes

| Feature | Flag Location | UT61B+ | UT61E+ |
|---------|--------------|--------|--------|
| HOLD | byte 14 bit 1 | Yes | Yes |
| REL | byte 14 bit 0 | Yes | Yes |
| MAX | byte 14 bit 3 | Yes (MAX/MIN button) | Yes |
| MIN | byte 14 bit 2 | Yes (MAX/MIN button) | Yes |
| AUTO | byte 15 bit 2 (inverted) | Yes | Yes |
| HV warning | byte 15 bit 0 | Yes | Yes |
| Low Battery | byte 15 bit 1 | Yes | Yes |
| Peak MAX | byte 16 bit 2 | **No** — [MANUAL] | Yes |
| Peak MIN | byte 16 bit 1 | **No** — [MANUAL] | Yes |
| DC indicator | byte 16 bit 3 | Yes | Yes |
| Bar polarity | byte 16 bit 0 | Yes | Yes |

**[MANUAL]** The UT61B+ manual (p.10) states the MAX/MIN button
"Short press to cycle through the measured maximum and minimum"
but does NOT mention Peak (P-MAX/P-MIN). Peak measurement is listed
as "(UT61D+/UT61E+)" only. The UT61B+ button is labeled just
"MAX/MIN" with no long-press Peak function.

---

## 5. Range Tables — [MANUAL]

The UT61B+ has different range values than the UT61E+ due to its
6,000-count display. The range byte encoding is the same (0x30
prefix, mask with & 0x0F to get index).

### 5.1 DC Voltage

| Range Index | UT61B+/UT61D+ | UT61E+ |
|-------------|---------------|--------|
| 0 | 60.00mV (0.01mV) | 220.00mV (0.01mV) |
| 1 | 600.0mV (0.1mV) | 2.2000V (0.1mV) |
| 2 | 6.000V (0.001V) | 22.000V (1mV) |
| 3 | 60.00V (0.01V) | 220.00V (10mV) |
| 4 | 600.0V (0.1V) | 1000.0V (0.1V) |
| 5 | 1000V (1V) | — |

Note: The UT61B+/UT61D+ has 6 DC voltage ranges; the UT61E+ has 5.
The UT61E+ does not have separate mV ranges on the DC V mode — the
mV ranges are accessed via the dedicated mV dial position (modes
0x02/0x03). The UT61B+ also folds its mV ranges into the V
measurement modes.

### 5.2 AC Voltage

| Range Index | UT61B+/UT61D+ | UT61E+ |
|-------------|---------------|--------|
| 0 | 60.00mV (0.01mV) | 220.00mV (0.01mV) |
| 1 | 600.0mV (0.1mV) | 2.2000V (0.1mV) |
| 2 | 6.000V (0.001V) | 22.000V (1mV) |
| 3 | 60.00V (0.01V) | 220.00V (10mV) |
| 4 | 600.0V (0.1V) | 1000.0V (0.1V) |
| 5 | 1000V (1V) | — |

AC voltage bandwidth: 40Hz-500Hz (UT61B+) vs 40Hz-10kHz (UT61E+).
LoZ ACV ranges (UT61D+ only): 600.0V and 1000V.

### 5.3 Resistance

| Range Index | UT61B+/UT61D+ | UT61E+ |
|-------------|---------------|--------|
| 0 | 600.0Ω (0.1Ω) | 220.00Ω (0.01Ω) |
| 1 | 6.000kΩ (1Ω) | 2.2000kΩ (0.1Ω) |
| 2 | 60.00kΩ (10Ω) | 22.000kΩ (1Ω) |
| 3 | 600.0kΩ (100Ω) | 220.00kΩ (10Ω) |
| 4 | 6.000MΩ (1kΩ) | 2.2000MΩ (100Ω) |
| 5 | 60.00MΩ (10kΩ) | 22.000MΩ (1kΩ) |
| 6 | — | 220.00MΩ (10kΩ) |

The UT61B+/UT61D+ has 6 resistance ranges (max 60MΩ); the UT61E+ has
7 (max 220MΩ).

### 5.4 Capacitance

| Range Index | UT61B+/UT61D+ | UT61E+ |
|-------------|---------------|--------|
| 0 | 60.00nF (10pF) | 22.000nF (1pF) |
| 1 | 600.0nF (100pF) | 220.00nF (10pF) |
| 2 | 6.000µF (1nF) | 2.2000µF (100pF) |
| 3 | 60.00µF (10nF) | 22.000µF (1nF) |
| 4 | 600.0µF (100nF) | 220.00µF (10nF) |
| 5 | 6.000mF (1µF) | 2.2000mF (100nF) |
| 6 | 60.00mF (10µF) | 22.000mF (1µF) |
| 7 | — | 220.00mF (10µF) |

The UT61B+/UT61D+ has 7 capacitance ranges (max 60mF); the UT61E+ has
8 (max 220mF).

### 5.5 DC Current

| Range Index | UT61B+/UT61D+ | UT61E+ |
|-------------|---------------|--------|
| 0 | 600.0µA (0.1µA) | 220.00µA (0.01µA) |
| 1 | 6000µA (1µA) | 2200.0µA (0.1µA) |
| 2 | 60.00mA (10µA) | 22.000mA (1µA) |
| 3 | 600.0mA (0.1mA) | 220.00mA (10µA) |
| 4 | 6.000A (1mA) | 20.000A (1mA) |
| 5 | 10.00A (10mA) — UT61B+ | — |
| 5 | 20.00A (10mA) — UT61D+ | — |

**[MANUAL]** The UT61B+ max current is 10A; the UT61D+ max current is
20A. The UT61E+ uses separate mode bytes for µA (0x0C/0x0D), mA
(0x0E/0x0F), and A (0x10/0x11), each with 2 ranges. The UT61B+/UT61D+
appears to use the same mode bytes but the firmware may map ranges
differently given the 6,000-count display.

### 5.6 AC Current

Same range structure as DC current, with the same model differences
(UT61B+: max 10A, UT61D+: max 20A).

### 5.7 Frequency

| Range | UT61B+/UT61D+ | UT61E+ |
|-------|---------------|--------|
| Full range | 10.00Hz - 10.00MHz | 10Hz - 220MHz |
| Resolution | 0.01Hz - 0.01MHz | 0.01Hz (varies) |

### 5.8 Continuity and Diode

Same across all models:
- Continuity: 600Ω range, resolution 0.1Ω, beep threshold <50Ω
- Diode: ~3V open circuit, resolution 0.001V

### 5.9 Duty Cycle

Same across all models: 0.1% - 99.9%, resolution 0.1%.

---

## 6. Commands — [VENDOR]

The UT61B+ accepts the same commands as the UT61E+. However, some
commands will have no effect because the corresponding feature does
not exist on the UT61B+:

| Command | Byte | UT61B+ Support |
|---------|------|---------------|
| GetMeasurement | 0x5E | Yes |
| Hold | 0x4A | Yes |
| Range | 0x46 | Yes |
| Auto | 0x47 | Yes [DEDUCED] |
| Rel | 0x48 | Yes [DEDUCED] |
| MinMax | 0x41 | Yes (MAX/MIN only, no Peak) [DEDUCED] |
| ExitMinMax | 0x42 | Yes [DEDUCED] |
| Select | 0x4C | Yes [DEDUCED] |
| Select2 | 0x49 | Yes [DEDUCED] |
| Light | 0x4B | Yes [DEDUCED] |
| PeakMinMax | 0x4D | **No effect** — feature not available [DEDUCED] |
| ExitPeak | 0x4E | **No effect** — feature not available [DEDUCED] |
| GetName | 0x5F | [UNVERIFIED] |

---

## 7. What Requires Real Device Verification

1. **Range byte mapping** — The manual gives range full-scale values
   (60mV, 600mV, 6V, etc.) but does not specify which range index
   (0x30, 0x31, ...) maps to which full-scale value. The ordering is
   [DEDUCED] to follow the same ascending pattern as the UT61E+.

2. **Mode bytes for UT61B+** — The mode byte values (0x00-0x14) are
   assumed identical to the UT61E+ based on the shared vendor software
   mode table. This is [DEDUCED] from the vendor software analysis but
   not verified against a real UT61B+ device.

3. **Bar graph encoding** — 31 segments confirmed by manual, but the
   actual byte encoding (raw value 0-31? combined nibbles?) is
   [UNVERIFIED] for the UT61B+.

4. **Peak command rejection** — Whether the UT61B+ silently ignores
   PeakMinMax (0x4D) or responds with an error is [UNVERIFIED].

5. **Current mode byte mapping** — Whether the UT61B+ uses the same
   separate mode bytes (0x0C-0x11) for µA/mA/A or consolidates them
   is [UNVERIFIED]. The vendor software table has entries for all
   current modes, suggesting the same byte assignments.

6. **NCV display format** — The UT61B+ NCV mode displays "EF" when no
   voltage is detected (per manual). The display format in the protocol
   response is [UNVERIFIED].

7. **MAX/MIN without Peak** — The UT61B+ MAX/MIN button behavior is
   [DEDUCED] from the manual — it likely sets the same flag bits
   (byte 14 bits 2/3) but never sets Peak bits (byte 16 bits 1/2).

---

## 8. Summary of Confidence Levels

| Aspect | Status | Source |
|--------|--------|--------|
| Same protocol as UT61E+ | **VENDOR** | No model conditionals in code |
| Same VID/PID/baud/framing | **VENDOR** | Shared CP2110.dll and transport |
| Same command set | **VENDOR** | Shared CustomDmm.dll |
| 6,000 count display | **MANUAL** | UT61+ Series User Manual p.4, p.25 |
| 31-segment bar graph | **MANUAL** | UT61+ Series User Manual p.25 |
| Available modes (17 of 26) | **MANUAL** | Function dial table p.9 |
| No temperature mode | **MANUAL** | "UT61D+ only" in manual |
| No hFE mode | **MANUAL** | "UT61E+ only" in manual |
| No LoZ mode | **MANUAL** | "UT61D+ only" in manual |
| No Peak measurement | **MANUAL** | "UT61D+/UT61E+ only" in manual |
| No LPF/AC+DC modes | **MANUAL** | "UT61E+ only" in manual |
| Max 10A current | **MANUAL** | Specifications p.32 |
| Range tables (full-scale values) | **MANUAL** | Specifications pp.26-34 |
| Mode byte values same as UT61E+ | **DEDUCED** | Shared mode table in vendor software |
| Range byte encoding same | **DEDUCED** | Shared table builder, no model branching |
| Frequency bandwidth 40-500Hz | **MANUAL** | Specifications p.27 |
| Commands beyond vendor 3 | **UNVERIFIED** | Not tested on UT61B+ hardware |
| Bar graph byte encoding | **UNVERIFIED** | Not parsed by vendor software |
