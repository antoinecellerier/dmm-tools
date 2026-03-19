# UT61D+ Protocol: Reverse-Engineered Specification

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

**The UT61D+ uses the identical wire protocol as the UT61E+.** All
transport, framing, command, and response formats are the same. The
vendor software contains no model-specific protocol code (see
`reverse-engineering-approach.md` for evidence).

This document focuses on documenting the **application-layer
differences**: which mode bytes the UT61D+ sends, what range values
it uses, and what features differ compared to the UT61E+.

The UT61D+ sits between the UT61B+ and UT61E+ in capability: it has
temperature and LoZ modes (absent from UT61B+) but lacks hFE, LPF,
and AC+DC modes (present only on UT61E+).

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
| UT61D+ | 6,000 | 3¾ | 0.01mV (mV range) |
| UT61E+ | 22,000 | 4¼ | 0.01mV (mV range) |

The 7-byte ASCII display field in the protocol is the same size for
both models. The UT61D+ uses fewer significant digits due to the lower
count. Display parsing is identical.

### 3.2 Bar Graph — [MANUAL]

| Model | Segments | Update Rate |
|-------|----------|------------|
| UT61D+ | 31 | 30 times/sec |
| UT61E+ | 46 | 30 times/sec |

The bar graph bytes (offsets 12-13) use the same encoding. The UT61D+
firmware generates values in the range 0-31 instead of 0-46.
**[DEDUCED]** — encoding not verified on real UT61D+ hardware.

### 3.3 Frequency Bandwidth — [MANUAL]

| Model | AC Voltage Bandwidth | AC Current Bandwidth |
|-------|---------------------|---------------------|
| UT61D+ | 40Hz-1kHz | 40Hz-1kHz |
| UT61E+ | 40Hz-10kHz | 40Hz-10kHz |

The UT61D+ has wider bandwidth than the UT61B+ (40Hz-500Hz) but
narrower than the UT61E+ (40Hz-10kHz).

---

## 4. Available Modes — [MANUAL + VENDOR]

The UT61D+ has more modes than the UT61B+ but fewer than the UT61E+.
The mode bytes are the same as the UT61E+.

### 4.1 Modes Present on UT61D+

| Mode Byte | Mode | UT61D+ Dial Position | Also on UT61B+? |
|-----------|------|---------------------|----------------|
| 0x00 | AC V | V~ (with Hz/% via SELECT2) | Yes |
| 0x01 | AC mV | mV (via SELECT) | Yes |
| 0x02 | DC V | V~ + SELECT | Yes |
| 0x03 | DC mV | mV position | Yes |
| 0x04 | Hz | Hz% position, or via SELECT2 | Yes |
| 0x05 | Duty % | Hz% position (via SELECT) | Yes |
| 0x06 | Resistance | Ω position | Yes |
| 0x07 | Continuity | Ω position (via SELECT) | Yes |
| 0x08 | Diode | Diode position | Yes |
| 0x09 | Capacitance | Diode position (via SELECT) | Yes |
| **0x0A** | **Temperature °C** | **°C°F position** | **No** |
| **0x0B** | **Temperature °F** | **°C°F position + SELECT** | **No** |
| 0x0C | DC µA | µA position | Yes |
| 0x0D | AC µA | µA position (via SELECT) | Yes |
| 0x0E | DC mA | mA position | Yes |
| 0x0F | AC mA | mA position (via SELECT) | Yes |
| 0x10 | DC A | A position | Yes |
| 0x11 | AC A | A position (via SELECT) | Yes |
| 0x14 | NCV | NCV position | Yes |
| **0x15** | **LoZ V** | **LoZ V~ position** | **No** |

**UT61D+-exclusive modes** (compared to UT61B+):
- **Temperature** (0x0A, 0x0B): K-type thermocouple input via the mA/µA
  terminal. The meter includes a K-type thermocouple in the box.
  Temperature range: -40°C to 1000°C / -40°F to 1832°F.
- **LoZ ACV** (0x15): Low-impedance AC voltage measurement to
  eliminate ghost voltages. Uses a low-impedance input (~3kΩ) instead
  of the standard ~10MΩ.
- **Peak measurement** (P-MAX/P-MIN): Long press of MAX/MIN button
  enables peak capture. Uses flag byte 16 bits 1 and 2.

### 4.2 Modes NOT Available on UT61D+

| Mode Byte | Mode | Why absent |
|-----------|------|------------|
| 0x12 | hFE | No adapter socket (UT61E+ only) |
| 0x13 | Live | [UNVERIFIED] |
| 0x16 | LoZ V (2) | [VENDOR] — in vendor table, may be alternate LoZ |
| 0x17 | LPF | No LPF on dial (UT61E+ only) |
| 0x18 | LPF V | No LPF on dial (UT61E+ only) |
| 0x19 | AC+DC V | No AC+DC on dial (UT61E+ only) |

**[VENDOR]** The vendor software mode table includes entries for modes
0x15 (LoZ V) and 0x16 (also labeled "LozV" in the string table). Both
share the same display name string, but **the code treats them
differently**: mode 0x16 has SI prefix multiplication applied to its
display value (like Ohm/Cap/Hz modes), while mode 0x15 does not (like
DCV/ACV modes). This suggests 0x15 and 0x16 represent two distinct LoZ
sub-modes with different display value scaling. Which byte(s) the
UT61D+ actually sends for its single LoZ dial position is [UNVERIFIED].

### 4.3 Feature Differences in Flag Bytes

| Feature | Flag Location | UT61D+ | UT61B+ | UT61E+ |
|---------|--------------|--------|--------|--------|
| HOLD | byte 14 bit 1 | Yes | Yes | Yes |
| REL | byte 14 bit 0 | Yes | Yes | Yes |
| MAX | byte 14 bit 3 | Yes | Yes | Yes |
| MIN | byte 14 bit 2 | Yes | Yes | Yes |
| AUTO | byte 15 bit 2 (inv) | Yes | Yes | Yes |
| HV warning | byte 15 bit 0 | Yes | Yes | Yes |
| Low Battery | byte 15 bit 1 | Yes | Yes | Yes |
| **Peak MAX** | byte 16 bit 2 | **Yes** | No | Yes |
| **Peak MIN** | byte 16 bit 1 | **Yes** | No | Yes |
| DC indicator | byte 16 bit 3 | Yes | Yes | Yes |
| Bar polarity | byte 16 bit 0 | Yes | Yes | Yes |

---

## 5. Range Tables — [MANUAL]

The UT61D+ shares the same range tables as the UT61B+ for most modes
(both are 6,000-count meters), with the addition of temperature and
LoZ ranges.

For the full voltage, resistance, capacitance, current, frequency,
continuity, and diode range tables, see
`docs/research/ut61b-plus/reverse-engineered-protocol.md` section 5.
The UT61D+ tables are identical to the UT61B+ tables for those modes,
with one exception: the UT61D+ max current is 20A (vs 10A for UT61B+).

### 5.1 Temperature — [MANUAL] (UT61D+ only)

| Range | Resolution | Accuracy |
|-------|-----------|----------|
| -40 to 0°C | 0.1°C-1°C | ±(1.0%+3°C) |
| 0 to 300°C | 0.1°C-1°C | ±(1.0%+2°C) |
| 300 to 1000°C | 0.1°C-1°C | ±(1.0%+3°C) |
| -40 to 32°F | 0.2°F-2°F | ±(1.0%+6°F) |
| 32 to 572°F | 0.2°F-2°F | ±(1.0%+4°F) |
| 572 to 1832°F | 0.2°F-2°F | ±(1.0%+6°F) |

Notes from manual:
- Only K-type thermocouple is supported
- LCD displays "OL" when meter is powered on without thermocouple
- Measured temperature should be less than 230°C/446°F
  (°F = °C x 1.8 + 32)
- Temperature uses mode bytes 0x0A (°C) and 0x0B (°F)
- SELECT button switches between °C and °F

### 5.2 LoZ ACV — [MANUAL] (UT61D+ only)

| Range | Resolution | Accuracy |
|-------|-----------|----------|
| 600.0V | 0.1V | ±(2.0%+5) |
| 1000V | 1V | ±(2.0%+5) |

Notes from manual:
- Low impedance (~3kΩ) eliminates ghost voltages
- After using LoZ function, wait 3 minutes before next operation
- Uses mode byte 0x15 (LoZ V)
- LoZ ACV measurement is for AC only (not DC)

### 5.3 Current — UT61D+ vs UT61B+

The only difference in shared range tables:

| Range | UT61D+ | UT61B+ |
|-------|--------|--------|
| A (highest range) | 20.00A (10mA resolution) | 10.00A (10mA resolution) |

All other current ranges (µA, mA, lower A) are identical.

---

## 6. Commands — [VENDOR]

The UT61D+ accepts the same commands as the UT61E+. Unlike the
UT61B+, the UT61D+ supports Peak measurement, so the PeakMinMax
command should work:

| Command | Byte | UT61D+ Support |
|---------|------|---------------|
| GetMeasurement | 0x5E | Yes |
| Hold | 0x4A | Yes |
| Range | 0x46 | Yes |
| Auto | 0x47 | Yes [DEDUCED] |
| Rel | 0x48 | Yes [DEDUCED] |
| MinMax | 0x41 | Yes [DEDUCED] |
| ExitMinMax | 0x42 | Yes [DEDUCED] |
| Select | 0x4C | Yes [DEDUCED] |
| Select2 | 0x49 | Yes [DEDUCED] |
| Light | 0x4B | Yes [DEDUCED] |
| PeakMinMax | 0x4D | Yes — long press MAX/MIN [DEDUCED] |
| ExitPeak | 0x4E | Yes [DEDUCED] |
| GetName | 0x5F | [UNVERIFIED] |

---

## 7. UT61D+ vs UT61B+ vs UT61E+ Summary

| Feature | UT61B+ | UT61D+ | UT61E+ |
|---------|--------|--------|--------|
| Display count | 6,000 | 6,000 | 22,000 |
| Bar graph segments | 31 | 31 | 46 |
| AC bandwidth | 40-500Hz | 40Hz-1kHz | 40Hz-10kHz |
| Max current | 10A | 20A | 20A |
| Max resistance | 60MΩ | 60MΩ | 220MΩ |
| Max capacitance | 60mF | 60mF | 220mF |
| Frequency range | 10MHz | 10MHz | 220MHz |
| Temperature | No | **Yes** | No |
| LoZ ACV | No | **Yes** | No |
| hFE | No | No | **Yes** |
| AC+DC V | No | No | **Yes** |
| LPF V | No | No | **Yes** |
| Peak (P-MAX/P-MIN) | No | **Yes** | **Yes** |
| NCV | Yes | Yes | Yes |
| USB data transmission | Yes | Yes | Yes |
| K-type thermocouple | Not included | **Included** | Not included |
| Adapter socket (hFE) | Not included | Not included | **Included** |

---

## 8. What Requires Real Device Verification

1. **Temperature mode bytes** — 0x0A (°C) and 0x0B (°F) are in the
   vendor software mode table. Whether the UT61D+ actually sends
   these exact bytes is [DEDUCED] but not verified on hardware.

2. **LoZ mode byte** — 0x15 is in the vendor table as "LozV". The
   vendor table also has 0x16 as a second "LozV" entry. Which one(s)
   the UT61D+ actually sends is [UNVERIFIED].

3. **Temperature display format** — The manual says the LCD shows
   temperature with °C or °F symbol. How this is encoded in the 7-byte
   ASCII display field is [UNVERIFIED]. The UT61E+ research shows
   temperature modes use special handling in the parser.

4. **Range byte mapping** — Like the UT61B+, the exact mapping of
   range indices to full-scale values is [DEDUCED] from the manual's
   range ordering.

5. **Peak flag behavior** — The UT61D+ supports Peak via long press
   of MAX/MIN. Whether it uses the same flag bits (byte 16 bits 1/2)
   as the UT61E+ is [DEDUCED] from the shared vendor software.

6. **LoZ ACV accuracy at low voltages** — The manual only lists 600V
   and 1000V LoZ ranges. Whether lower voltage LoZ readings are
   possible is [UNVERIFIED].

7. **Temperature range byte** — Temperature modes likely have a single
   range (index 0), as on the UT61E+. This is [DEDUCED].

---

## 9. Summary of Confidence Levels

| Aspect | Status | Source |
|--------|--------|--------|
| Same protocol as UT61E+ | **VENDOR** | No model conditionals in code |
| Same VID/PID/baud/framing | **VENDOR** | Shared CP2110.dll and transport |
| Same command set | **VENDOR** | Shared CustomDmm.dll |
| 6,000 count display | **MANUAL** | UT61+ Series User Manual p.4, p.25 |
| 31-segment bar graph | **MANUAL** | UT61+ Series User Manual p.25 |
| Available modes (19+ of 26) | **MANUAL** | Function dial table p.9 |
| Temperature modes (0x0A, 0x0B) | **MANUAL + VENDOR** | Manual p.21 + vendor mode table |
| LoZ mode (0x15) | **MANUAL + VENDOR** | Manual p.14 + vendor mode table |
| Peak measurement supported | **MANUAL** | Button description p.10 |
| No hFE mode | **MANUAL** | "UT61E+ only" in manual |
| No LPF/AC+DC modes | **MANUAL** | "UT61E+ only" in manual |
| Max 20A current | **MANUAL** | Specifications p.32 |
| Temperature range -40 to 1000°C | **MANUAL** | Specifications p.31 |
| LoZ ACV: 600V, 1000V | **MANUAL** | Specifications p.27 |
| Range tables (full-scale values) | **MANUAL** | Specifications pp.26-34 |
| Mode byte values same as UT61E+ | **DEDUCED** | Shared mode table in vendor software |
| Range byte encoding same | **DEDUCED** | Shared table builder, no model branching |
| Frequency bandwidth 40Hz-1kHz | **MANUAL** | Specifications p.27 |
| Temperature mode byte encoding | **UNVERIFIED** | Not tested on UT61D+ hardware |
| LoZ mode byte (0x15 vs 0x16) | **UNVERIFIED** | Vendor table has both |
| Bar graph byte encoding | **UNVERIFIED** | Not parsed by vendor software |
