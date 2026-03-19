# UT61+/UT161 Protocol Family: Reverse-Engineered Specification

Covers: **UT61B+**, **UT61D+**, **UT61E+**, **UT161B**, **UT161D**, **UT161E**

Based on:
- UT61+ Series User Manual (UNI-T, covers UT61B+/UT61D+/UT61E+)
- UT161 Series User Manual (UNI-T, covers UT161B/UT161D/UT161E)
- CP2110 Datasheet and AN434 (Silicon Labs)
- UNI-T Software V2.02 (decompiled with Ghidra)

Confidence levels:
- **[KNOWN]** — from official Silicon Labs documentation
- **[VENDOR]** — confirmed by decompiling UNI-T's official software
- **[MANUAL]** — stated in UNI-T's official user manual
- **[DEDUCED]** — logical inferences not yet verified against hardware
- **[UNVERIFIED]** — requires real device testing

For full protocol details (transport, framing, commands, response
parsing, flag byte layout), see:
`docs/research/ut61eplus/reverse-engineered-protocol.md`

This document focuses on the **per-model differences** and the evidence
that all six models share a single protocol.

---

## 1. Protocol is Identical Across All Models — [VENDOR]

The vendor software contains **zero model-specific protocol logic**.
All transport, framing, command, and response formats are shared. See
`reverse-engineering-approach.md` for evidence.

| Aspect | Value | All 6 models |
|--------|-------|:------------:|
| USB VID/PID | 0x10C4 / 0xEA80 | Same |
| Baud rate | 9600 bps, 8N1 | Same |
| Frame header | 0xAB 0xCD | Same |
| Length byte | payload + 2 | Same |
| Checksum | 16-bit BE sum | Same |
| Command format | AB CD 03 cmd chk_hi chk_lo | Same |
| GetMeasurement | 0x5E | Same |
| Response | 19 bytes total | Same |
| Mode byte | offset 3, raw | Same |
| Range byte | offset 4, 0x30 prefix | Same |
| Display | offsets 5-11, 7 ASCII | Same |
| Bar graph | offsets 12-13, raw | Same |
| Flag bytes | offsets 14-16, 0x30 prefix | Same |
| Communication model | Polled (request/response) | Same |

---

## 2. Model Comparison

### 2.1 Hardware Differences — [MANUAL]

| Feature | UT61B+ / UT161B | UT61D+ / UT161D | UT61E+ / UT161E |
|---------|:-:|:-:|:-:|
| Display count | 6,000 (3¾ digits) | 6,000 (3¾ digits) | 22,000 (4¼ digits) |
| Bar graph | 31 segments | 31 segments | 46 segments |
| Bar graph rate | 30 Hz | 30 Hz | 30 Hz |
| Numeric refresh | 2-3 Hz | 2-3 Hz | 2-3 Hz |
| AC bandwidth (V) | 40-500 Hz | 40 Hz-1 kHz | 40 Hz-10 kHz |
| Max current | 10A | 20A | 20A |
| Max resistance | 60 MΩ | 60 MΩ | 220 MΩ |
| Max capacitance | 60 mF | 60 mF | 220 mF |
| Frequency range | 10 MHz | 10 MHz | 220 MHz |
| Temperature | No | Yes (K-type) | No |
| LoZ ACV | No | Yes | No |
| hFE | No | No | Yes |
| AC+DC V | No | No | Yes |
| LPF V | No | No | Yes |
| Peak (P-MAX/P-MIN) | No | Yes | Yes |

The UT161 series mirrors the UT61+ series exactly in capability:
UT161B = UT61B+, UT161D = UT61D+, UT161E = UT61E+ (confirmed by
identical binaries in vendor software).

### 2.2 UT61+ vs UT161 — [VENDOR]

Binary comparison of the UT161E installer vs UT61E+ Software V2.02:

- 67/69 files byte-identical (including all protocol binaries)
- DMM.exe: 8 bytes differ (model name string only)
- options.xml: `<Model>` tag differs
- No functional difference whatsoever

---

## 3. Available Modes Per Model — [MANUAL + VENDOR]

The mode/range table in the vendor software contains entries for ALL
modes (0x00-0x19). The meter firmware determines which modes are
accessible via the physical dial. The PC software accepts any mode byte.

| Byte | Mode | B+/161B | D+/161D | E+/161E |
|------|------|:-------:|:-------:|:-------:|
| 0x00 | ACV | Yes | Yes | Yes |
| 0x01 | AC mV | Yes | Yes | Yes |
| 0x02 | DCV | Yes | Yes | Yes |
| 0x03 | DC mV | Yes | Yes | Yes |
| 0x04 | Hz | Yes | Yes | Yes |
| 0x05 | Duty % | Yes | Yes | Yes |
| 0x06 | Resistance | Yes | Yes | Yes |
| 0x07 | Continuity | Yes | Yes | Yes |
| 0x08 | Diode | Yes | Yes | Yes |
| 0x09 | Capacitance | Yes | Yes | Yes |
| 0x0A | Temp °C | — | **Yes** | — |
| 0x0B | Temp °F | — | **Yes** | — |
| 0x0C | DC µA | Yes | Yes | Yes |
| 0x0D | AC µA | Yes | Yes | Yes |
| 0x0E | DC mA | Yes | Yes | Yes |
| 0x0F | AC mA | Yes | Yes | Yes |
| 0x10 | DC A | Yes | Yes | Yes |
| 0x11 | AC A | Yes | Yes | Yes |
| 0x12 | hFE | — | — | **Yes** |
| 0x13 | Live | [UNVERIFIED] | [UNVERIFIED] | [UNVERIFIED] |
| 0x14 | NCV | Yes | Yes | Yes |
| 0x15 | LoZ V | — | **Yes** | — |
| 0x16 | LoZ V (2) | — | [UNVERIFIED] | — |
| 0x17 | LPF | — | — | **Yes** |
| 0x18 | (unknown) | — | — | [UNVERIFIED] |
| 0x19 | AC+DC V | — | — | **Yes** |

**LoZ modes 0x15 vs 0x16** — [VENDOR]: Both labeled "LozV" in the
vendor software, but mode 0x16 has SI prefix multiplication applied
to its display value while 0x15 does not. Which byte the UT61D+ sends
requires device testing.

---

## 4. Flag Byte Differences — [MANUAL + VENDOR]

The flag byte layout is identical across all models. The meter firmware
simply never sets certain flag bits on models that lack the feature.

| Feature | Flag | B+/161B | D+/161D | E+/161E |
|---------|------|:-------:|:-------:|:-------:|
| HOLD | byte 14 bit 1 | Yes | Yes | Yes |
| REL | byte 14 bit 0 | Yes | Yes | Yes |
| MAX | byte 14 bit 3 | Yes | Yes | Yes |
| MIN | byte 14 bit 2 | Yes | Yes | Yes |
| AUTO (inverted) | byte 15 bit 2 | Yes | Yes | Yes |
| HV warning | byte 15 bit 0 | Yes | Yes | Yes |
| Low Battery | byte 15 bit 1 | Yes | Yes | Yes |
| Peak MAX | byte 16 bit 2 | — | **Yes** | **Yes** |
| Peak MIN | byte 16 bit 1 | — | **Yes** | **Yes** |
| DC indicator | byte 16 bit 3 | Yes | Yes | Yes |
| Bar polarity | byte 16 bit 0 | Yes | Yes | Yes |

---

## 5. Range Tables — [MANUAL]

6,000-count models (UT61B+/D+, UT161B/D) and 22,000-count models
(UT61E+, UT161E) have different range tables. The range byte encoding
is the same: 0x30 prefix, mask with `& 0x0F` to get index.

### 5.1 DC Voltage

| Range | 6,000-count (B+/D+) | 22,000-count (E+) |
|-------|-------|-------|
| 0 | 60.00 mV (0.01 mV) | 220.00 mV (0.01 mV) |
| 1 | 600.0 mV (0.1 mV) | 2.2000 V (0.1 mV) |
| 2 | 6.000 V (1 mV) | 22.000 V (1 mV) |
| 3 | 60.00 V (10 mV) | 220.00 V (10 mV) |
| 4 | 600.0 V (0.1 V) | 1000.0 V (0.1 V) |
| 5 | 1000 V (1 V) | — |

### 5.2 AC Voltage

Same structure as DC voltage. AC bandwidth: 40-500 Hz (B+),
40 Hz-1 kHz (D+), 40 Hz-10 kHz (E+).

LoZ ACV ranges (UT61D+ only): 600.0 V and 1000 V.

### 5.3 Resistance

| Range | 6,000-count | 22,000-count |
|-------|-------------|--------------|
| 0 | 600.0 Ω (0.1 Ω) | 220.00 Ω (0.01 Ω) |
| 1 | 6.000 kΩ (1 Ω) | 2.2000 kΩ (0.1 Ω) |
| 2 | 60.00 kΩ (10 Ω) | 22.000 kΩ (1 Ω) |
| 3 | 600.0 kΩ (100 Ω) | 220.00 kΩ (10 Ω) |
| 4 | 6.000 MΩ (1 kΩ) | 2.2000 MΩ (100 Ω) |
| 5 | 60.00 MΩ (10 kΩ) | 22.000 MΩ (1 kΩ) |
| 6 | — | 220.00 MΩ (10 kΩ) |

### 5.4 Capacitance

| Range | 6,000-count | 22,000-count |
|-------|-------------|--------------|
| 0 | 60.00 nF (10 pF) | 22.000 nF (1 pF) |
| 1 | 600.0 nF (100 pF) | 220.00 nF (10 pF) |
| 2 | 6.000 µF (1 nF) | 2.2000 µF (100 pF) |
| 3 | 60.00 µF (10 nF) | 22.000 µF (1 nF) |
| 4 | 600.0 µF (100 nF) | 220.00 µF (10 nF) |
| 5 | 6.000 mF (1 µF) | 2.2000 mF (100 nF) |
| 6 | 60.00 mF (10 µF) | 22.000 mF (1 µF) |
| 7 | — | 220.00 mF (10 µF) |

### 5.5 Current

**µA ranges:**

| Range | 6,000-count | 22,000-count |
|-------|-------------|--------------|
| 0 | 600.0 µA (0.1 µA) | 220.00 µA (0.01 µA) |
| 1 | 6000 µA (1 µA) | 2200.0 µA (0.1 µA) |

**mA ranges:**

| Range | 6,000-count | 22,000-count |
|-------|-------------|--------------|
| 0 | 60.00 mA (10 µA) | 22.000 mA (1 µA) |
| 1 | 600.0 mA (0.1 mA) | 220.00 mA (10 µA) |

**A ranges:**

| Range | UT61B+ | UT61D+/E+ |
|-------|--------|-----------|
| 0 | 6.000 A (1 mA) | UT61E+: 20.000 A (1 mA) |
| 1 | 10.00 A (10 mA) | UT61D+: 20.00 A (10 mA) |

### 5.6 Temperature (UT61D+ / UT161D only) — [MANUAL]

| Range | Resolution | Accuracy |
|-------|-----------|----------|
| -40 to 0 °C | 0.1 °C | ±(1.0%+3 °C) |
| 0 to 300 °C | 0.1 °C | ±(1.0%+2 °C) |
| 300 to 1000 °C | 1 °C | ±(1.0%+3 °C) |
| -40 to 32 °F | 0.2 °F | ±(1.0%+6 °F) |
| 32 to 572 °F | 0.2 °F | ±(1.0%+4 °F) |
| 572 to 1832 °F | 2 °F | ±(1.0%+6 °F) |

K-type thermocouple only. Uses mode bytes 0x0A (°C) and 0x0B (°F).

### 5.7 Other Modes

Same across all models:
- **Continuity**: 600 Ω range, 0.1 Ω resolution, beep < 50 Ω
- **Diode**: ~3 V open circuit, 0.001 V resolution
- **Duty cycle**: 0.1%-99.9%, 0.1% resolution

---

## 6. Commands — [VENDOR]

All models accept the same command set (same `CustomDmm.dll`). Some
commands have no effect on models lacking the corresponding feature:

| Command | Byte | B+/161B | D+/161D | E+/161E |
|---------|------|:-------:|:-------:|:-------:|
| GetMeasurement | 0x5E | Yes | Yes | Yes |
| Hold | 0x4A | Yes | Yes | Yes |
| Range | 0x46 | Yes | Yes | Yes |
| Auto | 0x47 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Rel | 0x48 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| MinMax | 0x41 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| ExitMinMax | 0x42 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Select | 0x4C | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Select2 | 0x49 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Light | 0x4B | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| PeakMinMax | 0x4D | No effect | [DEDUCED] | [DEDUCED] |
| ExitPeak | 0x4E | No effect | [DEDUCED] | [DEDUCED] |
| GetName | 0x5F | [UNVERIFIED] | [UNVERIFIED] | [UNVERIFIED] |

---

## 7. What Requires Real Device Verification

All remaining unknowns require hardware access — no further RE is
possible from the vendor software.

1. **Range index → full-scale mapping** — manual gives full-scale
   values but not which range index maps to which. Ascending order
   is [DEDUCED].

2. **6,000-count bar graph encoding** — 31 segments (from manual),
   but wire encoding unknown for offsets 12-13. [UNVERIFIED]

3. **LoZ mode byte** — whether UT61D+ sends 0x15, 0x16, or both
   for its single LoZ dial position. [UNVERIFIED]

4. **Temperature display format** — how °C/°F readings are encoded
   in the 7-byte ASCII display field. [UNVERIFIED]

5. **Mode 0x13 (Live)** — availability on each model. [UNVERIFIED]

6. **Commands beyond confirmed 3** — 0x5E, 0x4A, 0x46 are [VENDOR]
   confirmed in the software. All others are [DEDUCED] from UT61E+
   device testing.

7. **UT61B+ Peak command rejection** — whether PeakMinMax (0x4D) is
   silently ignored or returns an error. [UNVERIFIED]

---

## 8. Summary of Confidence Levels

| Aspect | Status | Source |
|--------|--------|--------|
| Identical protocol across all 6 models | **VENDOR** | Zero model conditionals in code |
| UT161 = UT61+ (binary-identical software) | **VENDOR** | 67/69 files match |
| Shared VID/PID/baud/framing | **VENDOR** | Shared CP2110.dll |
| Shared command set | **VENDOR** | Shared CustomDmm.dll |
| 6,000 vs 22,000 count displays | **MANUAL** | UT61+ Series manual |
| 31 vs 46 bar graph segments | **MANUAL** | UT61+ Series manual |
| Mode availability per model | **MANUAL** | Function dial tables |
| Range tables per count type | **MANUAL** | Specifications pages |
| LoZ modes 0x15 vs 0x16 behavior | **VENDOR** | SI multiplier code paths differ |
| LoZ mode byte sent by UT61D+ | **UNVERIFIED** | Requires device |
| Temperature mode bytes (0x0A, 0x0B) | **DEDUCED** | Vendor mode table |
| Range index → full-scale mapping | **DEDUCED** | Ascending order assumed |
| Commands beyond 0x5E/0x4A/0x46 | **DEDUCED** | UT61E+ device testing |
