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

### 3.1 Function dial positions and cycle rings — [MANUAL]

Source: UT61+ Series User Manual §VII "Function Dial" (printed page 9), read
from the PDF rendering; §11 for the UT61D+ temperature pair. The tables below
say **which modes a dial position reaches with which button** — membership
only. The order a button walks its ring in is *not* claimed: it differs
between models and was never captured for most positions, so the driver in
`crates/dmm-lib/src/protocol/cycle.rs` presses and reads the mode back until
the target shows, which works whatever the real order is.

The two buttons are the orange **SELECT** (command 0x4C) and **Hz/%**
(0x49, `Select2`). A position with both rings joins them at exactly one mode:
crossing rings means walking to that mode with the first button and away with
the second.

**Hz (0x04) and Duty % (0x05) carry no dial information** — [VERIFIED]: the
meter sends the same two bytes from every position that offers them, so a
reading in Hz cannot say which position produced it. History is what
disambiguates: a meter last seen in AC mV is on the mV position.

#### UT61E+ / UT161E

| Dial position | SELECT ring | Hz/% ring |
|---|---|---|
| Hz/% | — | Hz, Duty % |
| V~ | AC V, LPF V | AC V, Hz, Duty % |
| V⎓ | DC V, AC+DC V | — |
| mV | DC mV, AC mV | AC mV, Hz, Duty % |
| Ω | Ω, Continuity, Diode, Capacitance | — |
| hFE | hFE | — |
| µA | DC µA, AC µA | AC µA, Hz, Duty % |
| mA | DC mA, AC mA | AC mA, Hz, Duty % |
| A | DC A, AC A | AC A, Hz, Duty % |
| NCV | NCV | — |

Every row of this table, ring orders included, was walked on a real UT61E+ on
2026-09-07 with `dmm-cli set mode` (leads open, `RUST_LOG=dmm_lib=debug`) —
**[VERIFIED]**:

- SELECT (0x4C): DC V ↔ AC+DC V on V⎓; AC V ↔ LPF V on V~; DC ↔ AC on mV, µA,
  mA and A; Ω → Continuity → Diode → Capacitance → Ω (that order).
- Hz/% (0x49): AC V/mV/µA/mA/A → Hz → Duty % → back, and Hz ↔ Duty % on the
  Hz/% position. It does nothing on V⎓ or in DC mV, and neither button does
  anything on hFE or NCV.
- Every leg needed one press. The meter reported the new mode on the first
  read ~300 ms after the press in all but one case (one extra ~100 ms read on
  the Ω ring), which is what the driver's 150 ms delay / 3 reads settle covers.
- Pressing SELECT while the meter is in Hz or Duty % leaves the Hz/% ring for
  the *other* member of the position's SELECT ring — LPF V on V~, DC mV, DC µA,
  DC mA, DC A — not the junction mode. On the Hz/% position SELECT toggles
  Hz ↔ Duty % like Hz/% does. The driver does not rely on either: it only
  ever presses SELECT from a SELECT-ring mode.
- A switch goes through under HOLD (the SELECT press also clears HOLD) and
  under MIN/MAX. AUTO is back once the target mode shows. LPF V always
  reports manual range (flag byte 15 bit 2 set) and the AUTO button does not
  change that — a property of the meter, not of the switch.
- Modes 0x15 (LoZ V), 0x16 (LoZ V 2) and 0x17 (LPF) are reachable from no
  position of this model, which is why the table above lists none of them.

#### UT61D+ / UT161D

| Dial position | SELECT ring | Hz/% ring |
|---|---|---|
| Hz/% | — | Hz, Duty % |
| V≂ | AC V, DC V | AC V, Hz, Duty % |
| mV | DC mV, AC mV | AC mV, Hz, Duty % |
| Ω | Ω, Continuity, Diode, Capacitance | — |
| °C/°F | °C, °F | — |
| LoZ | LoZ V (0x15) | — |
| LoZ | LoZ V (0x16) | — |
| µA | DC µA, AC µA | AC µA, Hz, Duty % |
| mA | DC mA, AC mA | AC mA, Hz, Duty % |
| A | DC A, AC A | AC A, Hz, Duty % |
| NCV | NCV | — |

This model combines AC and DC volts on one position (§2.1) and has no hFE,
no LPF and no AC+DC. Both ring contents and ring order are **[UNVERIFIED]** —
no UT61D+ has been connected. The two LoZ positions are listed separately
because which byte the meter sends is unresolved (§3) and nothing in the
manual says SELECT cycles between them.

#### UT61B+ / UT161B

| Dial position | SELECT ring | Hz/% ring |
|---|---|---|
| Hz/% | — | Hz, Duty % |
| V~ | — | AC V, Hz, Duty % |
| V⎓ | DC V | — |
| mV | DC mV, AC mV | AC mV, Hz, Duty % |
| Ω/Continuity | Ω, Continuity | — |
| Diode/Capacitance | Diode, Capacitance | — |
| µA | DC µA, AC µA | AC µA, Hz, Duty % |
| mA | DC mA, AC mA | AC mA, Hz, Duty % |
| A | DC A, AC A | AC A, Hz, Duty % |
| NCV | NCV | — |

This model has no AC+DC and no LPF, so its V~ position has no SELECT ring at
all, and it splits the E+'s single Ω position in two. No temperature, no hFE,
no LoZ. Both ring contents and ring order are **[UNVERIFIED]** — no UT61B+
has been connected.

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

A UT61B+ capture reported 2026-09-09 (issue #19) confirmed that the
6,000-count ladders ascend and that index 0 is the bottom rung in Ω,
capacitance, µA, mA and A — auto-ranging sat there with the leads open or
shorted, and the display's decimal count matches each bottom rung's full
scale. Voltage is the exception, below.

### 5.1 DC Voltage

| Range | 6,000-count (B+/D+) | 22,000-count (E+) |
|-------|-------|-------|
| 0 | 60.00 mV (0.01 mV) | 220.00 mV (0.01 mV) |
| 1 | 600.0 mV (0.1 mV) | 2.2000 V (0.1 mV) |
| 2 | 6.000 V (1 mV) | 22.000 V (1 mV) |
| 3 | 60.00 V (10 mV) | 220.00 V (10 mV) |
| 4 | 600.0 V (0.1 V) | 1000.0 V (0.1 V) |
| 5 | 1000 V (1 V) | — |

The *index* column is the manual's own row order, not the wire encoding. On
the UT61E+ the wire indices for DC V are 0=2.2V, 1=22V, 2=220V, 3=1000V —
[VERIFIED] 2026-03-21 and re-walked 2026-09-07 (section 6.1). Its 220 mV
full scale belongs to the separate DC mV mode, not to a DC V range index.

The UT61B+ works the same way. Its wire indices are 0=6V, 1=60V, 2=600V,
3=1000V, with index 0 [VERIFIED] by the 2026-09-09 capture — DC V and AC V
both sent range byte 0 while displaying three decimals (`  0.000`, `  0.037`)
and the meter's screen read volts. The rungs above 0 stay [DEDUCED]; 60 mV
and 600 mV are the mV modes on their own dial position. The UT61D+ shares
the table shape and is assumed to match, unverified — issue #7.

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

### 5.8 Overload display — [VERIFIED] (UT61E+ and UT61B+)

The 7-character display field is a segment dump, not a formatted number: the
meter lights the decimal point belonging to the rung it is on and writes `O`
and `L` into the digit slots either side of it. Overload therefore reaches the
wire in three shapes, and which one depends on the range rather than the
model:

| Display | Seen on |
|---------|---------|
| ` .OL   ` | E+ Ω 2.2MΩ, E+ diode, B+ diode |
| `  O.L  ` | E+ Ω 22kΩ, B+ Ω 60MΩ |
| `  OL.  ` | E+ Ω 220kΩ and 220MΩ, E+ DC mV 220mV, E+ continuity |

Both UT61B+ reports agree with each other and all five of our UT61E+
captures show the same pattern. A parser must ignore the point rather than
match a fixed string.

The point's position is also a second reading of the range byte: the B+'s
`  O.L ` in Ω sits where a 60.00 MΩ rung puts its decimal, which is range
byte 5, exactly as section 9 has it.

## 6. Commands — [VENDOR]

All models accept the same command set (same `CustomDmm.dll`). Some
commands have no effect on models lacking the corresponding feature:

| Command | Byte | B+/161B | D+/161D | E+/161E |
|---------|------|:-------:|:-------:|:-------:|
| GetMeasurement | 0x5E | Yes | Yes | Yes |
| Hold | 0x4A | Yes | Yes | Yes |
| Range | 0x46 | Yes | Yes | Yes |
| Auto | 0x47 | [VERIFIED]* | [DEDUCED] | Yes (§6.1) |
| Rel | 0x48 | [VERIFIED]* | [DEDUCED] | [DEDUCED] |
| MinMax | 0x41 | [VERIFIED]* | [DEDUCED] | [DEDUCED] |
| ExitMinMax | 0x42 | [VERIFIED]* | [DEDUCED] | [DEDUCED] |
| Select | 0x4C | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Select2 | 0x49 | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| Light | 0x4B | [DEDUCED] | [DEDUCED] | [DEDUCED] |
| PeakMinMax | 0x4D | No effect | [DEDUCED] | [DEDUCED] |
| ExitPeak | 0x4E | No effect | [DEDUCED] | [DEDUCED] |
| GetName | 0x5F | [VERIFIED]* | [UNVERIFIED] | [UNVERIFIED] |

\* UT61B+ capture, 2026-09-09 (issue #19): each command moved the flag the
tool expected on the next frame, and GetName answered `UT61B+`. Hold and
Range were already [VENDOR]-confirmed and behaved the same way there.

"Yes" in this table means the model has the command at all. Which *modes*
accept it is section 6.2 — several accept it nowhere useful.

### 6.1 Range and Auto semantics — [VERIFIED] (UT61E+, 2026-09-07)

Walked on our UT61E+ with `dmm-cli set range`, which presses the command and
re-reads the range byte (leads open and with 1.5 V DC applied, V⎓ and V~ dial
positions):

- **Range (0x46)** steps the ladder. From auto-range the first press engages
  *manual* ranging on the rung the meter is already showing — it does not
  step — and every press after that moves exactly one rung up. The top rung
  wraps to the bottom (DC V: 2.2V → 22V → 220V → 1000V → 2.2V). The mode
  byte never changed under a press, so 0x46 does not touch the function.
- **Auto (0x47)** puts the meter back in auto-range from any manual rung,
  in one command.
- The meter answers each command with a 2-byte `FF 00` ack frame, and its
  first measurement frame after a press may still carry the pre-press state:
  a reader that presses again on seeing the old value will overshoot.
- Modes whose range is fixed ignore 0x46 entirely — on the E+ that is DC mV
  and AC mV (section 9), where neither the range byte nor the AUTO
  annunciator moves.

**MIN/MAX and Peak rings — [VERIFIED]** on the same meter: MinMax (0x41)
walks off → MAX → MIN, one press per step; the ring never comes back to off
(2-state cycle, section 4 notes and 2026-03-21 device testing), so
ExitMinMax (0x42) is what leaves. PeakMinMax (0x4D) walks P-MAX → P-MIN the
same way in AC V, and ExitPeak (0x4E) leaves. Hold (0x4A) and Rel (0x48)
each toggle on one press.

### 6.2 Which modes take which command — [VERIFIED] (UT61E+ and UT61B+)

The command table above is per model. It is also per *mode*: a press in the
wrong mode is accepted on the wire and changes nothing on screen. Section 6
used to say the vendor command matrix listed no mode restriction on 0x4A,
0x48 or 0x41; two meters have since contradicted that, refusing the same
commands in the same modes — a UT61E+ over CP2110 (2026-09-07,
`ut61eplus-verify4.yaml`) and a UT61B+ over CH9329 (2026-09-10, issue #19).
Each row below is a press whose flag did not move on the next frame.

| Mode | Hold 0x4A | Rel 0x48 | MinMax 0x41 | Range 0x46 |
|------|:---------:|:--------:|:-----------:|:----------:|
| Continuity (0x07) | Yes | **No effect** | **No effect** | fixed range |
| Diode (0x08) | Yes | untested | untested | fixed range |
| Capacitance (0x09) | Yes | Yes | **No effect** | **No effect** |
| Hz (0x04) | Yes | **No effect** | **No effect** | **No effect** |
| Duty % (0x05) | Yes | **No effect** | **No effect** | fixed range |
| NCV (0x14) | **No effect** | **No effect** | **No effect** | no table |
| AC+DC V (0x19) | Yes | one meter only | Yes | Yes |
| Every other mode | Yes | Yes | Yes | Yes (section 6.1) |

"Fixed range" means the mode has one rung, so there is nothing for 0x46 to
step; "No effect" in that column means the mode has several rungs and the
button still does not move between them — auto-ranging is the only way there.

**A refusal only counts when a real reading was on screen.** The meter also
refuses Rel whenever the display shows OL, whatever the mode: `dcmv/rel:on`
was refused over OL in the 2026-03 run and taken in all three later runs
where DC mV had a value. Hold is not affected — diode's Hold frames carry the
flag over OL.

That is why **diode reads "untested" in both middle columns**: every refusal
recorded there, five across the two meters, was over OL, because open leads
in diode read OL. Nothing is known about diode with a diode connected.

**AC+DC V** refused Rel twice on one UT61E+ with 0.07 V and 0.08 V showing.
That mode does not exist on the UT61B+, so no second meter can corroborate
it, and one meter is not enough to call a button dead. MinMax there works and
says so: `acdcv/minmax:min` read 0.0652 V back with the MIN flag set.

The rows in bold are the ones the code acts on (`HOLD_DEAD`, `REL_DEAD`,
`MINMAX_DEAD` in `ut61eplus/mod.rs`, `FAMILY_FIXED_RANGE_MODES` in
`tables/mod.rs`); the rest stay offered. The D+ and the UT161 models are
[DEDUCED] to match: section 1 establishes that all six run the same command
code.

---

## 7. What Requires Real Device Verification

All remaining unknowns require hardware access — no further RE is
possible from the vendor software.

1. **Range index → full-scale mapping** — manual gives full-scale
   values but not which range index maps to which. Ascending order
   is [DEDUCED] except on the UT61E+ DC V ladder, where it is
   [VERIFIED] (section 6.1: one rung up per RANGE press, 1000V wraps
   to 2.2V), and at the bottom rungs a UT61B+ auto-ranged into
   (section 5).

2. **6,000-count bar graph encoding** — 31 segments (from manual).
   The 2026-09-09 UT61B+ capture carries the bar in the same bytes as
   the E+ (payload 9-10, decimal `b9*10 + b10`), reading 30 at O.L and
   1 at 4% of range — consistent with 31 segments counted 0-30. Never
   walked against a moving input. [UNVERIFIED]

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

8. **Frequency ladder on 6,000-count models** — the 2026-09-09 UT61B+
   capture reported range index 0 in Hz from both the V~ position and
   the Hz dial position, displaying `0.0` in one and `0.00` in the
   other. One index cannot carry both full scales, so the code's five
   invented Hz ranges do not describe the meter. [UNVERIFIED]

9. **NCV display on the UT61B+** — the same capture sent `   ----` in
   NCV (mode 0x14). Four dashes read as detection level 4 under the
   counting rule the UT61E+ taught (§13 of the manual), but the B+ may
   simply idle at four dashes where the E+ idles at `EF`. Needs one
   frame taken with no field nearby. [UNVERIFIED]

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
| Range index → full-scale mapping | **DEDUCED** (E+ DC V, B+ bottom rungs **VERIFIED**) | Ascending order assumed; E+ DC V walked on the device 2026-09-07, B+ bottom rungs from the 2026-09-09 capture |
| Commands beyond 0x5E/0x4A/0x46 | **DEDUCED** | UT61E+ device testing |

---

## 9. Per-Range Full-Scale Limits

**MANUAL** — transcribed from the UT61+ Series specification tables (see the
`specs_*.rs` provenance headers: `references/ut61eplus/ut61e_manual.pdf`
section IX.2 and the UT61B+/UT61D+ equivalents). These are the values the
`RangeInfo.overload_pos`/`overload_neg` fields carried in
`crates/dmm-lib/src/protocol/ut61eplus/tables/` from the first commit until
they were removed from the code; no production code ever read them, only the
table files' own tests. They are kept
here so the numbers stay findable if a software overload check or bar-graph
scaling is built later. Which range *index* maps to which row is [DEDUCED]
(section 7, item 1).

Columns: `Table` is the range table name in the source file; `Modes` lists
the `Mode` variants that share it (derived modes reuse their base mode's
table); `Idx` is the range byte (`payload[1] & 0x0F`); `Full scale (−)` of
`—` means the quantity has no negative range, `0` means the lower limit is
zero (duty cycle, diode, hFE). Generated from the source tables, not retyped.

### UT61E+ (22,000 counts)

Source: `ut61e_plus.rs` (50 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV, AcDcV, LpfV, LozV | 0 | 2.2V | V | 2.2 | -2.2 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 1 | 22V | V | 22 | -22 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 2 | 220V | V | 220 | -220 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 3 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 2.2V | V | 2.2 | -2.2 |
| `ac_v` | AcV | 1 | 22V | V | 22 | -22 |
| `ac_v` | AcV | 2 | 220V | V | 220 | -220 |
| `ac_v` | AcV | 3 | 750V | V | 750 | -750 |
| `dc_mv` | DcMv, AcDcMv, LpfMv | 0 | 220mV | mV | 220 | -220 |
| `dc_mv` | DcMv, AcDcMv, LpfMv | 1 | 2.2V | mV | 2200 | -2200 |
| `ac_mv` | AcMv | 0 | 220mV | mV | 220 | -220 |
| `ac_mv` | AcMv | 1 | 2.2V | mV | 2200 | -2200 |
| `ohm` | Ohm | 0 | 220Ω | Ω | 220 | — |
| `ohm` | Ohm | 1 | 2.2kΩ | kΩ | 2.2 | — |
| `ohm` | Ohm | 2 | 22kΩ | kΩ | 22 | — |
| `ohm` | Ohm | 3 | 220kΩ | kΩ | 220 | — |
| `ohm` | Ohm | 4 | 2.2MΩ | MΩ | 2.2 | — |
| `ohm` | Ohm | 5 | 22MΩ | MΩ | 22 | — |
| `ohm` | Ohm | 6 | 220MΩ | MΩ | 220 | — |
| `capacitance` | Capacitance | 0 | 22nF | nF | 22 | — |
| `capacitance` | Capacitance | 1 | 220nF | nF | 220 | — |
| `capacitance` | Capacitance | 2 | 2.2µF | µF | 2.2 | — |
| `capacitance` | Capacitance | 3 | 22µF | µF | 22 | — |
| `capacitance` | Capacitance | 4 | 220µF | µF | 220 | — |
| `capacitance` | Capacitance | 5 | 2.2mF | mF | 2.2 | — |
| `capacitance` | Capacitance | 6 | 22mF | mF | 22 | — |
| `capacitance` | Capacitance | 7 | 220mF | mF | 220 | — |
| `hz` | Hz | 0 | 22Hz | Hz | 22 | — |
| `hz` | Hz | 1 | 220Hz | Hz | 220 | — |
| `hz` | Hz | 2 | 2.2kHz | kHz | 2.2 | — |
| `hz` | Hz | 3 | 22kHz | kHz | 22 | — |
| `hz` | Hz | 4 | 220kHz | kHz | 220 | — |
| `duty_cycle` | DutyCycle | 0 | Duty | % | 100 | 0 |
| `temp_c` | TempC | 0 | Temp | °C | 1200 | -40 |
| `temp_f` | TempF | 0 | Temp | °F | 2192 | -40 |
| `diode` | Diode | 0 | Diode | V | 2.2 | 0 |
| `continuity` | Continuity | 0 | Cont | Ω | 220 | — |
| `dc_ua` | DcUa | 0 | 220µA | µA | 220 | -220 |
| `dc_ua` | DcUa | 1 | 2200µA | µA | 2200 | -2200 |
| `ac_ua` | AcUa | 0 | 220µA | µA | 220 | -220 |
| `ac_ua` | AcUa | 1 | 2200µA | µA | 2200 | -2200 |
| `dc_ma` | DcMa | 0 | 22mA | mA | 22 | -22 |
| `dc_ma` | DcMa | 1 | 220mA | mA | 220 | -220 |
| `ac_ma` | AcMa | 0 | 22mA | mA | 22 | -22 |
| `ac_ma` | AcMa | 1 | 220mA | mA | 220 | -220 |
| `dc_a` | DcA, LozV2, Lpf, AcDcA2, LpfA | 0 | 20A | A | 20 | -20 |
| `dc_a` | DcA, LozV2, Lpf, AcDcA2, LpfA | 1 | 20A | A | 20 | -20 |
| `ac_a` | AcA | 0 | 20A | A | 20 | -20 |
| `ac_a` | AcA | 1 | 20A | A | 20 | -20 |
| `hfe` | Hfe | 0 | 1000β | β | 1000 | 0 |

On the UT61E+ the mV dial is fixed-range in **both** of its modes — DC mV
[VERIFIED] 2026-03-21, AC mV [VERIFIED] 2026-09-07 (three RANGE presses moved
neither the range byte nor the AUTO annunciator). Only index 0 (220mV) occurs
on this model; the 2.2V rows of `dc_mv`/`ac_mv` belong to other models.

### UT61B+ (6,000 counts)

Source: `ut61b_plus.rs` (49 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 60mV | mV | 60 | -60 |
| `dc_v` | DcV | 1 | 600mV | mV | 600 | -600 |
| `dc_v` | DcV | 2 | 6V | V | 6 | -6 |
| `dc_v` | DcV | 3 | 60V | V | 60 | -60 |
| `dc_v` | DcV | 4 | 600V | V | 600 | -600 |
| `dc_v` | DcV | 5 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 60mV | mV | 60 | -60 |
| `ac_v` | AcV | 1 | 600mV | mV | 600 | -600 |
| `ac_v` | AcV | 2 | 6V | V | 6 | -6 |
| `ac_v` | AcV | 3 | 60V | V | 60 | -60 |
| `ac_v` | AcV | 4 | 600V | V | 600 | -600 |
| `ac_v` | AcV | 5 | 750V | V | 750 | -750 |
| `dc_mv` | DcMv | 0 | 60mV | mV | 60 | -60 |
| `dc_mv` | DcMv | 1 | 600mV | mV | 600 | -600 |
| `ac_mv` | AcMv | 0 | 60mV | mV | 60 | -60 |
| `ac_mv` | AcMv | 1 | 600mV | mV | 600 | -600 |
| `ohm` | Ohm | 0 | 600Ω | Ω | 600 | — |
| `ohm` | Ohm | 1 | 6kΩ | kΩ | 6 | — |
| `ohm` | Ohm | 2 | 60kΩ | kΩ | 60 | — |
| `ohm` | Ohm | 3 | 600kΩ | kΩ | 600 | — |
| `ohm` | Ohm | 4 | 6MΩ | MΩ | 6 | — |
| `ohm` | Ohm | 5 | 60MΩ | MΩ | 60 | — |
| `capacitance` | Capacitance | 0 | 60nF | nF | 60 | — |
| `capacitance` | Capacitance | 1 | 600nF | nF | 600 | — |
| `capacitance` | Capacitance | 2 | 6µF | µF | 6 | — |
| `capacitance` | Capacitance | 3 | 60µF | µF | 60 | — |
| `capacitance` | Capacitance | 4 | 600µF | µF | 600 | — |
| `capacitance` | Capacitance | 5 | 6mF | mF | 6 | — |
| `capacitance` | Capacitance | 6 | 60mF | mF | 60 | — |
| `hz` | Hz | 0 | 60Hz | Hz | 60 | — |
| `hz` | Hz | 1 | 600Hz | Hz | 600 | — |
| `hz` | Hz | 2 | 6kHz | kHz | 6 | — |
| `hz` | Hz | 3 | 60kHz | kHz | 60 | — |
| `hz` | Hz | 4 | 600kHz | kHz | 600 | — |
| `duty_cycle` | DutyCycle | 0 | Duty | % | 100 | 0 |
| `diode` | Diode | 0 | Diode | V | 3 | 0 |
| `continuity` | Continuity | 0 | Cont | Ω | 600 | — |
| `dc_ua` | DcUa | 0 | 600µA | µA | 600 | -600 |
| `dc_ua` | DcUa | 1 | 6000µA | µA | 6000 | -6000 |
| `ac_ua` | AcUa | 0 | 600µA | µA | 600 | -600 |
| `ac_ua` | AcUa | 1 | 6000µA | µA | 6000 | -6000 |
| `dc_ma` | DcMa | 0 | 60mA | mA | 60 | -60 |
| `dc_ma` | DcMa | 1 | 600mA | mA | 600 | -600 |
| `ac_ma` | AcMa | 0 | 60mA | mA | 60 | -60 |
| `ac_ma` | AcMa | 1 | 600mA | mA | 600 | -600 |
| `dc_a` | DcA | 0 | 6A | A | 6 | -6 |
| `dc_a` | DcA | 1 | 10A | A | 10 | -10 |
| `ac_a` | AcA | 0 | 6A | A | 6 | -6 |
| `ac_a` | AcA | 1 | 10A | A | 10 | -10 |

### UT61D+ (6,000 counts)

Source: `ut61d_plus.rs` (53 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 60mV | mV | 60 | -60 |
| `dc_v` | DcV | 1 | 600mV | mV | 600 | -600 |
| `dc_v` | DcV | 2 | 6V | V | 6 | -6 |
| `dc_v` | DcV | 3 | 60V | V | 60 | -60 |
| `dc_v` | DcV | 4 | 600V | V | 600 | -600 |
| `dc_v` | DcV | 5 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 60mV | mV | 60 | -60 |
| `ac_v` | AcV | 1 | 600mV | mV | 600 | -600 |
| `ac_v` | AcV | 2 | 6V | V | 6 | -6 |
| `ac_v` | AcV | 3 | 60V | V | 60 | -60 |
| `ac_v` | AcV | 4 | 600V | V | 600 | -600 |
| `ac_v` | AcV | 5 | 750V | V | 750 | -750 |
| `dc_mv` | DcMv | 0 | 60mV | mV | 60 | -60 |
| `dc_mv` | DcMv | 1 | 600mV | mV | 600 | -600 |
| `ac_mv` | AcMv | 0 | 60mV | mV | 60 | -60 |
| `ac_mv` | AcMv | 1 | 600mV | mV | 600 | -600 |
| `ohm` | Ohm | 0 | 600Ω | Ω | 600 | — |
| `ohm` | Ohm | 1 | 6kΩ | kΩ | 6 | — |
| `ohm` | Ohm | 2 | 60kΩ | kΩ | 60 | — |
| `ohm` | Ohm | 3 | 600kΩ | kΩ | 600 | — |
| `ohm` | Ohm | 4 | 6MΩ | MΩ | 6 | — |
| `ohm` | Ohm | 5 | 60MΩ | MΩ | 60 | — |
| `capacitance` | Capacitance | 0 | 60nF | nF | 60 | — |
| `capacitance` | Capacitance | 1 | 600nF | nF | 600 | — |
| `capacitance` | Capacitance | 2 | 6µF | µF | 6 | — |
| `capacitance` | Capacitance | 3 | 60µF | µF | 60 | — |
| `capacitance` | Capacitance | 4 | 600µF | µF | 600 | — |
| `capacitance` | Capacitance | 5 | 6mF | mF | 6 | — |
| `capacitance` | Capacitance | 6 | 60mF | mF | 60 | — |
| `hz` | Hz | 0 | 60Hz | Hz | 60 | — |
| `hz` | Hz | 1 | 600Hz | Hz | 600 | — |
| `hz` | Hz | 2 | 6kHz | kHz | 6 | — |
| `hz` | Hz | 3 | 60kHz | kHz | 60 | — |
| `hz` | Hz | 4 | 600kHz | kHz | 600 | — |
| `duty_cycle` | DutyCycle | 0 | Duty | % | 100 | 0 |
| `temp_c` | TempC | 0 | Temp | °C | 1000 | -40 |
| `temp_f` | TempF | 0 | Temp | °F | 1832 | -40 |
| `diode` | Diode | 0 | Diode | V | 3 | 0 |
| `continuity` | Continuity | 0 | Cont | Ω | 600 | — |
| `dc_ua` | DcUa | 0 | 600µA | µA | 600 | -600 |
| `dc_ua` | DcUa | 1 | 6000µA | µA | 6000 | -6000 |
| `ac_ua` | AcUa | 0 | 600µA | µA | 600 | -600 |
| `ac_ua` | AcUa | 1 | 6000µA | µA | 6000 | -6000 |
| `dc_ma` | DcMa | 0 | 60mA | mA | 60 | -60 |
| `dc_ma` | DcMa | 1 | 600mA | mA | 600 | -600 |
| `ac_ma` | AcMa | 0 | 60mA | mA | 60 | -60 |
| `ac_ma` | AcMa | 1 | 600mA | mA | 600 | -600 |
| `dc_a` | DcA | 0 | 20A | A | 20 | -20 |
| `dc_a` | DcA | 1 | 20A | A | 20 | -20 |
| `ac_a` | AcA | 0 | 20A | A | 20 | -20 |
| `ac_a` | AcA | 1 | 20A | A | 20 | -20 |
| `loz_v` | LozV, LozV2 | 0 | 600V | V | 600 | -600 |
| `loz_v` | LozV, LozV2 | 1 | 1000V | V | 1000 | -1000 |
