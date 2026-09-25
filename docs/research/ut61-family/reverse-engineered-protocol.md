# UT61+/UT161 Protocol Family: Reverse-Engineered Specification

Covers: **UT61B+**, **UT61D+**, **UT61E+**, **UT161B**, **UT161D**, **UT161E**,
and the **UT60BT** and **UT202BT**, which have Bluetooth built in

UNI-T's protocol deck (below) also covers the **UT202S** clamp meter, which
dmm-tools does not support.

Based on:
- UT61+ Series User Manual (UNI-T, covers UT61B+/UT61D+/UT61E+)
- UT161 Series User Manual (UNI-T, covers UT161B/UT161D/UT161E)
- CP2110 Datasheet and AN434 (Silicon Labs)
- UNI-T Software V2.02 (decompiled with Ghidra)
- UNI-T protocol deck "UT61+系列通讯协议" (UT161/UT61+/UT202S), published on
  the UT61E+ product page of meters.uni-trend.com.cn — see
  `reverse-engineering-approach.md`
- UNI-T's iDMM2.0 Android app, its UT60BT and UT202BT range assets, and the
  UT60BT and UT202T/UT202BT manuals (§9)

Confidence levels:
- **[KNOWN]** — from official Silicon Labs documentation
- **[VENDOR]** — confirmed by decompiling UNI-T's official software
- **[VENDOR-DOC]** — stated in UNI-T's published protocol deck
- **[MANUAL]** — stated in UNI-T's official user manual
- **[COMMUNITY]** — reported by a community source; the sources are in
  §10 and UT61E+ spec §7
- **[DEDUCED]** — logical inferences not yet verified against hardware
- **[UNVERIFIED]** — requires real device testing

For full protocol details (transport, framing, commands, response
parsing, flag byte layout), see:
`docs/research/ut61eplus/reverse-engineered-protocol.md`

This document focuses on the **per-model differences** and the evidence
that all six models share a single protocol.

---

## 1. Protocol is Identical Across All Models — [VENDOR + VENDOR-DOC]

The vendor software contains **zero model-specific protocol logic**.
All transport, framing, command, and response formats are shared. See
`reverse-engineering-approach.md` for evidence.

The protocol deck says the same outright: it is one "Bluetooth
communication protocol" for the UT161 series, the UT61+ series and the
UT202S, with one mode table spanning meters and clamps (§3) and a range
table per model (§5). The same bytes travel over the USB cable.

The UT60BT and UT202BT send the same frames with the radio built in, over
the ISSC service the UT-D07B carries (`docs/research/ut-d07b/`). UNI-T's
iDMM2.0 app decodes them with the parser it uses for the UT61+ behind a
UT-D07B, after the same 0x5F then 0x5D handshake (§6.4), and a range table
each (§9) [VENDOR]. Over a UT60BT's own radio each frame arrives whole in one
notification, and no adapter heartbeat appears [COMMUNITY] (§10).

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
identical binaries in vendor software). Their manuals' specifications
differ only in the input fuses: 600mA 1000V Φ6x32mm and 11A 1000V
Φ10x38mm on the UT161, 1A 240V and 10A 240V, both Φ6x25mm, on the UT61+.

### 2.2 UT61+ vs UT161 — [VENDOR]

Binary comparison of the UT161E installer vs UT61E+ Software V2.02:

- 67/69 files byte-identical (including all protocol binaries)
- DMM.exe: 8 bytes differ (model name string only)
- options.xml: `<Model>` tag differs
- No functional difference whatsoever

### 2.3 Secondary-display frame — [VENDOR + VENDOR-DOC + MANUAL]

A clamp meter with a second display sends it in a frame of its own: the
19-byte measurement frame (UT61E+ spec §2.4) with bit 7 of the mode byte
set. [VENDOR] from iDMM2.0's parser, which reads it from any model:

| Offset | Field | In a secondary frame |
|--------|-------|----------------------|
| 3 | Mode | bit 7 set; bits 0-6 are the function, numbered as a main frame's mode byte (UT61E+ spec §2.5) |
| 4 | Range | as a main frame, an index into that function's range table |
| 5-11 | Display | 7 ASCII characters, as a main frame |
| 12-16 | Bar graph, flags | not read by the app; content unknown |

The app shows the value beside the main reading and blanks it 500 ms after
a main frame when no new secondary has come.

Which meters send it:

- **UT202S** — [VENDOR-DOC] the protocol deck's UT202S slide: in ACV, ACA,
  LPF ACV, LPF ACA and °C/°F the meter sends a main and a secondary display,
  and the phone shows both. The deck does not give the encoding.
- **UT202BT** — [MANUAL] the auxiliary display shows frequency in AC V
  (P8/14) and AC A (P11/20), °F beside °C (P11/19), and "CUT" in AC A when
  the clamp overheats (P12/21). The manual says nothing of it in DC V, LPF,
  peak, inrush, Ω or capacitance.
- **UT60BT** — no second display in its manual.

[UNVERIFIED]: no capture from any meter holds a secondary frame. How often
one comes and where it falls against the main frames, what the bar graph and
flag bytes hold, and how "CUT" is sent are unknown.

---

## 3. Available Modes Per Model — [MANUAL + VENDOR + VENDOR-DOC]

The mode/range table in the vendor software contains entries for ALL
modes (0x00-0x19). The meter firmware determines which modes are
accessible via the physical dial. The PC software accepts any mode byte.
The protocol deck's mode table runs to 0x1E and names 0x16, 0x17 and
0x1A-0x1E as clamp functions and current variants, none of them on a
UT61+ dial (UT61E+ spec §2.5).

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
| 0x13 | Live | — | — | — |
| 0x14 | NCV | Yes | Yes | Yes |
| 0x15 | LoZ V | — | **Yes** | — |
| 0x16 | Clamp AC A | — | — | — |
| 0x17 | Clamp DC A | — | — | — |
| 0x18 | LPF V | — | — | **Yes** |
| 0x19 | AC+DC V | — | — | **Yes** |

"—" for Live (0x13) is the manual's dial, which lists no such function on
any of the three models; no capture has shown the byte.

**LoZ is 0x15** — [VENDOR-DOC]: V2.02 labels both 0x15 and 0x16 "LozV"
(and applies an SI prefix to 0x16 only), which left open which byte the
UT61D+ sends. The protocol deck settles it: 0x15 is low-impedance AC
voltage, 0x16 a clamp meter's AC A. No UT61D+ has confirmed it.

---

### 3.1 Function dial positions and cycle rings — [MANUAL]

Source: UT61+ Series User Manual §VII "Function Dial" (printed page 9), read
from the PDF rendering; §11 for the UT61D+ temperature pair. The tables below
say **which modes a dial position reaches with which button** — membership
only. The order a button walks its ring in is *not* claimed: it differs
between models and was never captured for most positions.

The two buttons are the orange **SELECT** (command 0x4C) and **Hz/%**
(0x49, `Select2`). A position with both rings joins them at exactly one mode:
crossing rings means walking to that mode with the first button and away with
the second.

**Hz (0x04) and Duty % (0x05) carry no dial information** — [VERIFIED]: the
meter sends the same two bytes from every position that offers them, so a
reading in Hz cannot say which position produced it. Nor can the rest of the
frame, at least with the leads open: a UT61E+ Hz frame from V~ (2026-09-14)
matches the one from the Hz/% position (2026-09-07) byte for byte,
`04 30 20 20 20 30 2E 30 30 00 00 30 30 30`. History is what disambiguates: a
meter last seen in AC mV is on the mV position.

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
  the Ω ring).
- One walk takes two presses in a row: Hz → Duty % → AC V. It went through on
  2026-09-13 **[VERIFIED]**, the new mode showing ~0.3 s after the press into
  Duty % and ~0.6 s after the press into AC V. A UT61B+ fails this walk
  (issue #20).
- Pressing SELECT while the meter is in Hz or Duty % leaves the Hz/% ring for
  the *other* member of the position's SELECT ring — LPF V on V~, DC mV, DC µA,
  DC mA, DC A — not the junction mode. On the Hz/% position SELECT toggles
  Hz ↔ Duty % like Hz/% does.
- A switch goes through under MIN/MAX. Under HOLD a SELECT switch goes
  through and the press clears HOLD, but a Hz/% press does nothing
  (section 6.3). AUTO is back once the target mode shows. LPF V always
  reports manual range (flag byte 15 bit 2 set) and the AUTO button does not
  change that — a property of the meter, not of the switch. Entering LPF V
  on open leads reads high at first: on 2026-09-14 three entries opened at
  361.1 V, 164.8 V and 35.5 V with the HV warning lit, the last decaying
  through 5.1 V to 0.0 V over three reads 300 ms apart, with or without HOLD.
- Modes 0x15 (LoZ V), 0x16 and 0x17 (clamp AC A and DC A) are reachable
  from no position of this model, which is why the table above lists none
  of them.

#### UT61D+ / UT161D

| Dial position | SELECT ring | Hz/% ring |
|---|---|---|
| Hz/% | — | Hz, Duty % |
| V≂ | AC V, DC V | AC V, Hz, Duty % |
| mV | DC mV, AC mV | AC mV, Hz, Duty % |
| Ω | Ω, Continuity, Diode, Capacitance | — |
| °C/°F | °C, °F | — |
| LoZ | LoZ V | — |
| µA | DC µA, AC µA | AC µA, Hz, Duty % |
| mA | DC mA, AC mA | AC mA, Hz, Duty % |
| A | DC A, AC A | AC A, Hz, Duty % |
| NCV | NCV | — |

This model combines AC and DC volts on one position (§2.1) and has no hFE,
no LPF and no AC+DC. Both ring contents and ring order are **[UNVERIFIED]** —
no UT61D+ has been connected. The LoZ position sends 0x15 per the protocol
deck (§3).

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
no LoZ. Ring contents are **[VERIFIED]** for six legs, each reached in one press by
the driver in the 2026-09-10 capture (issue #19): SELECT took Ω → Continuity,
Diode → Capacitance, and DC → AC on µA, mA and A, and Hz/% took Hz → Duty %
on the Hz/% position. The V~ Hz/% ring is **[VERIFIED]** in the table's order,
AC V → Hz → Duty % → AC V, one press per step (2026-09-14 capture, issue #20).
**[UNVERIFIED]**: the mV position's SELECT leg (DC mV ↔ AC mV), which the
operator pressed rather than the driver, and the other three-member Hz/% rings
(AC mV/AC µA/AC mA/AC A → Hz → Duty %) — the only rings on this model where
order could differ from the table. Tracked in issue
#7; see `docs/verification-backlog.md`.

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
| AC/DC (set on the AC component of AC+DC) | byte 16 bit 3 | — | — | **Yes** |
| Bar polarity | byte 16 bit 0 | Yes | Yes | Yes |

The protocol deck also names byte 15 bit 3 APO (auto power-off); no
capture from either meter has set it.

---

## 5. Range Tables — [MANUAL]

6,000-count models (UT61B+/D+, UT161B/D) and 22,000-count models
(UT61E+, UT161E) have different range tables. The range byte encoding
is the same: 0x30 prefix, mask with `& 0x0F` to get index.

A UT61B+ capture reported 2026-09-09 (issue #19) confirmed that the
6,000-count ladders ascend and that index 0 is the bottom rung in Ω,
capacitance, µA, mA and A — auto-ranging sat there with the leads open or
shorted, and the display's decimal count matches each bottom rung's full
scale. Voltage is the exception, below. A third capture on 2026-09-11 walked
the whole Ω and DC V ladders with RANGE, one rung per press.

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
3=1000V, all four [VERIFIED] for DC V by the 2026-09-11 RANGE walk, which
loses one decimal per rung (`  0.001`, `   0.00`, `    0.0`, `     0 `). In
AC V, index 0 is [VERIFIED] by the 2026-09-09 capture (`  0.037` with the
screen reading volts) and index 2 by 236.6 V of mains on 2026-09-11; indices
1 and 3 stay [DEDUCED]. 60 mV and 600 mV are the mV modes on their own dial
position. The UT61D+ shares the table shape and is assumed to match,
unverified — issue #7.

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

All six 6,000-count rungs are [VERIFIED] on a UT61B+ by the 2026-09-11 RANGE
walk, each identified by the decimal its overload dump lights (section 5.8).

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

Rung 5 is [VERIFIED] on a UT61B+ at 4.514 mF on 2026-09-11; with rung 0 above,
that leaves 1-4 and 6 [DEDUCED].

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

| Range | UT61B+ | UT61D+ | UT61E+ |
|-------|--------|--------|--------|
| 0 | 6.000 A (1 mA) | 6.000 A (1 mA) [VENDOR-DOC] | [UNVERIFIED] |
| 1 | 10.00 A (10 mA) | 20.00 A (10 mA) [UNVERIFIED] | 20.000 A (1 mA) |

The UT61E+ manual prints one A range, and the captures show range 1 — the
only byte the protocol deck's UT61E+ table gives (as "10A"); byte 0 has not
been seen. The UT61D+ order follows the UT61B+'s, which issue #19 verified,
and the deck's joint UT61B+/UT61D+ table puts 6A at byte 0 [VENDOR-DOC]. The
two sources conflict on byte 1: the deck gives 10A for both models, the
manual gives the UT61D+ 20.00 A (the table's value). No UT61D+ has confirmed
either — issue #7.

### 5.6 Temperature (UT61D+ / UT161D only) — [MANUAL]

| Range | Resolution | Accuracy |
|-------|-----------|----------|
| -40 to 0 °C | 0.1 °C | ±(1.0%+3 °C) |
| 0 to 300 °C | 0.1 °C | ±(1.0%+2 °C) |
| 300 to 1000 °C | 1 °C | ±(1.0%+3 °C) |
| -40 to 32 °F | 0.2 °F | ±(1.0%+6 °F) |
| 32 to 572 °F | 0.2 °F | ±(1.0%+4 °F) |
| 572 to 1832 °F | 2 °F | ±(1.0%+6 °F) |

K-type thermocouple only. Uses mode bytes 0x0A (°C) and 0x0B (°F), each
with two range bytes in the protocol deck [VENDOR-DOC]: 0 for -40~300 °C
(-40~572 °F) and 1 for 300~1000 °C (572~1832 °F), the split where the
resolution changes.

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
| ` .OL   ` | E+ Ω 2.2MΩ, E+ diode, B+ diode, B+ Ω 6kΩ and 6MΩ |
| `  O.L  ` | E+ Ω 22kΩ, B+ Ω 60kΩ and 60MΩ |
| `  OL.  ` | E+ Ω 220kΩ and 220MΩ, E+ DC mV 220mV, E+ continuity, B+ Ω 600Ω and 600kΩ |

All three UT61B+ reports agree with each other and all five of our UT61E+
captures show the same pattern; the 2026-09-11 Ω walk showed all three shapes
on one meter in one run, as the E+'s 82 kΩ run did. The point's place in the
OL string is not fixed.

The point's position is also a second reading of the range byte: the B+'s
`  O.L ` in Ω sits where a 60.00 MΩ rung puts its decimal, which is range
byte 5, exactly as section 9 has it.

### 5.9 Frequency — [VENDOR-DOC]

The manual gives only a span (10.00 Hz–10.00 MHz on the 6,000-count
models, 10 Hz–220 MHz on the UT61E+); the protocol deck gives the rungs:

| Range | UT61B+/UT61D+ | UT61E+ |
|-------|---------------|--------|
| 0 | 99.99 Hz | 22 Hz |
| 1 | 999.9 Hz | 220 Hz |
| 2 | 9.999 kHz | 2.2 kHz |
| 3 | 99.99 kHz | 22 kHz |
| 4 | 999.9 kHz | 220 kHz |
| 5 | 9.999 MHz | 2.2 MHz |
| 6 | — | 22 MHz |
| 7 | — | 220 MHz |

The 6,000-count ladder counts to 9,999, and its top rung (9.999 MHz) stops
short of the manual's 10.00 MHz. Rung 0 fits both UT61B+ Hz/%-position
frames (`0.00`, `49.98`) and the UT61E+'s (`0.00`); no other rung has been
seen on a meter (section 7, item 8).

## 6. Commands — [VENDOR]

All models accept the same command set (same `CustomDmm.dll`), and every
byte below is in the protocol deck's command table [VENDOR-DOC]; the deck's
clamp-only commands are listed in the UT61E+ spec §2.3. Some commands have
no effect on models lacking the corresponding feature:

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
| Diode (0x08) | Yes | **No effect** | **No effect** | fixed range |
| Capacitance (0x09) | Yes | Yes | **No effect** | **No effect** |
| Hz (0x04) | Yes | **No effect** | **No effect** | **No effect** |
| Duty % (0x05) | Yes | **No effect** | **No effect** | fixed range |
| NCV (0x14) | **No effect** | **No effect** | **No effect** | no table |
| AC+DC V (0x19) | Yes | **No effect** | Yes | Yes |
| Every other mode | Yes | Yes | Yes | Yes (section 6.1) |

"Fixed range" means the mode has one rung, so there is nothing for 0x46 to
step; "No effect" in that column means the mode has several rungs and the
button still does not move between them — auto-ranging is the only way there.

**A refusal of Rel only counts when a real reading was on screen.** The meter
refuses Rel whenever the display shows OL, whatever the mode: `dcmv/rel:on`
was refused over OL in the 2026-03 run and taken in all three later runs where
DC mV had a value. Hold and MinMax are not affected — diode's Hold frames
carry the flag over OL, and `dcmv/minmax` was taken twice over OL.

**Diode** refused both Rel and MinMax on both meters, each asked with a diode
fitted — the only evidence there not confounded by OL, since open leads in
that mode read OL. Our UT61E+ on 2026-09-10 with a Schottky forward-biased at
0.1968 V, and a UT61B+ on 2026-09-11 at 0.515 V (issue #19), where Hold was
taken in the same step. MinMax was settled first: the UT61B+ had already
refused it five times and OL is no confound for that button.

**AC+DC V** refused Rel on all three runs that reached it — 2026-09-07 twice
at 0.07 V and 0.08 V, and 2026-09-10 again — which is every meter that has
the mode, the UT61B+ having no such dial position. Hold and MinMax both work
there, so it is Rel specifically: `acdcv/minmax:min` read 0.0652 V back with
the MIN flag set.

The rows in bold are the ones the code acts on (`HOLD_DEAD`, `REL_DEAD`,
`MINMAX_DEAD` in `ut61eplus/mod.rs`, `FAMILY_FIXED_RANGE_MODES` in
`tables/mod.rs`); the rest stay offered. The D+ and the UT161 models are
[DEDUCED] to match: section 1 establishes that all six run the same command
code.

### 6.3 Commands under HOLD — [VERIFIED] (UT61E+, 2026-09-14)

Our UT61E+ over CP2110, leads open: `dmm-cli set hold on`, one raw
`dmm-cli command` press, then `debug` frames read back and the LCD watched.

| Command | Pressed from | Under HOLD |
|---------|--------------|------------|
| Select 0x4C | AC V, Hz | Switches (to LPF V) and clears HOLD |
| Select2 0x49 | AC V, Hz | **No effect** — the meter beeps, HOLD stays lit |
| Range 0x46 | DC V and AC V on auto | Goes manual on the rung showing and clears HOLD |
| Auto 0x47 | DC V and AC V on a manual rung | Back to auto and clears HOLD |
| Rel 0x48 | AC V | **No effect** — HOLD stays lit |

- **An ignored press is dropped, not deferred**: after HOLD was released the
  meter stayed in Hz for the next five frames over 2.5 s, and REL was still
  off.
- The front-panel SELECT and Hz/% buttons behave the same way by hand.
- A UT61B+ drops Hz/% under HOLD too. In Hz on the V~ position (issue #20
  capture, 2026-09-14) a 0x49 was acked and the following frames stayed in Hz
  with HOLD lit, and the owner found the buttons ignored by hand. Section 3.1
  gives that position a Hz/% ring and no SELECT ring. Range and Auto under
  HOLD are unasked on the B+.

### 6.4 Start order over Bluetooth — [VENDOR]

UNI-T's iDMM2.0 app opens every Bluetooth link to the family the same way:
Get Name (0x5F), then, once the name frame is back, 0x5D once. It never
polls with 0x5E. That holds for the UT60BT and UT202BT and for the UT61+ and
UT161 behind a UT-D07B.

- **A UT60BT ignores 0x5D until it has answered 0x5F** — [COMMUNITY], a live
  capture (UT61E+ spec §7). [UNVERIFIED] here: no UT60BT or UT202BT has been
  connected.
- **Behind a UT-D07B the order does not matter** — [VERIFIED] on our UT61E+:
  0x5D alone starts the readings, the adapter acting on it itself
  (`../ut-d07b/reverse-engineered-protocol.md` §3).
- **A UT60BT answers the 0x5E poll all the same**, one frame per request —
  [COMMUNITY] (§10).

### 6.5 UT60BT and UT202BT buttons — [VENDOR]

The bytes iDMM2.0's button pages send to the two meters, and no others; the
button names are the manuals'. What each does on the meter is [UNVERIFIED].

| Byte | UT60BT | UT202BT |
|------|--------|---------|
| 0x31 | — | A~ (yellow), short press |
| 0x32 | — | A~, long press (inrush) |
| 0x33 | — | V~ (red) |
| 0x35 | — | Ω (blue) |
| 0x36 | — | NCV/PEAK, short press |
| 0x37 | — | NCV/PEAK, long press (peak capture) |
| 0x46 | RANGE | RANGE |
| 0x47 | RANGE, long press (auto) | — |
| 0x48 | REL | — |
| 0x4A | HOLD | HOLD |
| 0x4C | SELECT | — |

0x31-0x37 are in neither the protocol deck nor Software V2.02. The app
disables MAX/MIN on the UT60BT, which has no such button. Community clients
send further bytes to a UT60BT (§10).

---

## 7. What Requires Real Device Verification

All remaining unknowns require hardware access — no further RE is
possible from the vendor software.

1. **Range index → full-scale mapping** — manual gives full-scale
   values but not which range index maps to which. Ascending order
   is [DEDUCED] except on the UT61E+ DC V ladder, where it is
   [VERIFIED] (section 6.1: one rung up per RANGE press, 1000V wraps
   to 2.2V), on the UT61B+ Ω and DC V ladders, walked the same way on
   2026-09-11, and at the bottom rungs a UT61B+ auto-ranged into
   (section 5).

2. **6,000-count bar graph encoding** — 31 segments (from manual).
   The 2026-09-09 UT61B+ capture carries the bar in the same bytes as
   the E+ (payload 9-10, decimal `b9*10 + b10`), reading 30 at O.L and
   1 at 4% of range — consistent with 31 segments counted 0-30. Never
   walked against a moving input. [UNVERIFIED]

3. **LoZ mode byte** — 0x15 per the protocol deck (§3) [VENDOR-DOC];
   no UT61D+ has sent it yet. [UNVERIFIED]

4. **Temperature display format** — how °C/°F readings are encoded
   in the 7-byte ASCII display field. [UNVERIFIED]

5. **Mode 0x13 (Live)** — a contact live/neutral wire check
   [VENDOR-DOC]. No UT61+ dial position lists it (§3.1) and no capture
   has shown it. [UNVERIFIED]

6. ~~**Commands beyond confirmed 3**~~ — RESOLVED: every command byte is
   in the protocol deck [VENDOR-DOC]; what each does per model is §6.

7. **UT61B+ Peak command rejection** — whether PeakMinMax (0x4D) is
   silently ignored or returns an error. [UNVERIFIED]

8. **Frequency ladder on 6,000-count models** — the rungs are now the
   protocol deck's (§5.9), which fit the Hz/% position's `0.00` and
   `49.98` at index 0. Still open: the 2026-09-09 UT61B+ capture's `0.0`
   at index 0 from the V~ position, one decimal where rung 0 has two. The
   manual's AC remarks give the UT61B+/UT61D+ frequency 0.1 Hz resolution
   on the AC positions, which would explain it; whether the range byte
   then leaves 0 above 99.99 Hz is unknown. [UNVERIFIED]

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
| LoZ mode byte sent by UT61D+ | **VENDOR-DOC** (0x15) | Protocol deck; no UT61D+ capture |
| Temperature mode bytes (0x0A, 0x0B) | **DEDUCED** | Vendor mode table |
| Range index → full-scale mapping | **DEDUCED** (E+ DC V, B+ bottom rungs **VERIFIED**) | Ascending order assumed; E+ DC V walked on the device 2026-09-07, B+ bottom rungs from the 2026-09-09 capture |
| Commands beyond 0x5E/0x4A/0x46 | **VENDOR-DOC** | Protocol deck; per-model effects in §6 |

---

## 9. Per-Range Full-Scale Limits

**MANUAL** — transcribed from the UT61+ Series specification tables
(`references/ut61eplus/ut61e_manual.pdf`, "IX. Specifications", 2. Electrical
Specifications). The Hz rungs and the UT61D+ temperature split are the
protocol deck's (§5.9, §5.6) [VENDOR-DOC]. Which range *index* maps to which
row is [DEDUCED] except where section 5 says otherwise (section 7, item 1).

Columns: `Table` is the range table name in the source file; `Modes` lists
the `Mode` variants that share it (derived modes reuse their base mode's
table); `Idx` is the range byte (`payload[1] & 0x0F`); `Full scale (−)` of
`—` means the quantity has no negative range, `0` means the lower limit is
zero (duty cycle, diode, hFE).

### UT61E+ (22,000 counts)

Source: `ut61e_plus.rs` (51 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV, AcDcV, LpfV, LozV | 0 | 2.2V | V | 2.2 | -2.2 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 1 | 22V | V | 22 | -22 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 2 | 220V | V | 220 | -220 |
| `dc_v` | DcV, AcDcV, LpfV, LozV | 3 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 2.2V | V | 2.2 | -2.2 |
| `ac_v` | AcV | 1 | 22V | V | 22 | -22 |
| `ac_v` | AcV | 2 | 220V | V | 220 | -220 |
| `ac_v` | AcV | 3 | 1000V | V | 1000 | -1000 |
| `dc_mv` | DcMv | 0 | 220mV | mV | 220 | -220 |
| `ac_mv` | AcMv | 0 | 220mV | mV | 220 | -220 |
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
| `hz` | Hz | 5 | 2.2MHz | MHz | 2.2 | — |
| `hz` | Hz | 6 | 22MHz | MHz | 22 | — |
| `hz` | Hz | 7 | 220MHz | MHz | 220 | — |
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
| `dc_a` | DcA | 0 | 20A | A | [UNVERIFIED] | [UNVERIFIED] |
| `dc_a` | DcA | 1 | 20A | A | 20 | -20 |
| `ac_a` | AcA | 0 | 20A | A | [UNVERIFIED] | [UNVERIFIED] |
| `ac_a` | AcA | 1 | 20A | A | 20 | -20 |
| `hfe` | Hfe | 0 | 1000β | β | 1000 | 0 |

On the UT61E+ the mV dial is fixed-range in **both** of its modes — DC mV
[VERIFIED] 2026-03-21, AC mV [VERIFIED] 2026-09-07 (three RANGE presses moved
neither the range byte nor the AUTO annunciator). Only index 0 (220mV) has
ever been seen there, and the protocol deck's UT61E+ table has that rung
alone.

Index 0 of `dc_a`/`ac_a` has not been seen on the UT61E+ (§5.5).

### UT61B+ (6,000 counts)

Source: `ut61b_plus.rs` (46 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 6V | V | 6 | -6 |
| `dc_v` | DcV | 1 | 60V | V | 60 | -60 |
| `dc_v` | DcV | 2 | 600V | V | 600 | -600 |
| `dc_v` | DcV | 3 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 6V | V | 6 | -6 |
| `ac_v` | AcV | 1 | 60V | V | 60 | -60 |
| `ac_v` | AcV | 2 | 600V | V | 600 | -600 |
| `ac_v` | AcV | 3 | 1000V | V | 1000 | -1000 |
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
| `hz` | Hz | 0 | 99.99Hz | Hz | 99.99 | — |
| `hz` | Hz | 1 | 999.9Hz | Hz | 999.9 | — |
| `hz` | Hz | 2 | 9.999kHz | kHz | 9.999 | — |
| `hz` | Hz | 3 | 99.99kHz | kHz | 99.99 | — |
| `hz` | Hz | 4 | 999.9kHz | kHz | 999.9 | — |
| `hz` | Hz | 5 | 9.999MHz | MHz | 9.999 | — |
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

Source: `ut61d_plus.rs` (52 ranges).

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 6V | V | 6 | -6 |
| `dc_v` | DcV | 1 | 60V | V | 60 | -60 |
| `dc_v` | DcV | 2 | 600V | V | 600 | -600 |
| `dc_v` | DcV | 3 | 1000V | V | 1000 | -1000 |
| `ac_v` | AcV | 0 | 6V | V | 6 | -6 |
| `ac_v` | AcV | 1 | 60V | V | 60 | -60 |
| `ac_v` | AcV | 2 | 600V | V | 600 | -600 |
| `ac_v` | AcV | 3 | 1000V | V | 1000 | -1000 |
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
| `hz` | Hz | 0 | 99.99Hz | Hz | 99.99 | — |
| `hz` | Hz | 1 | 999.9Hz | Hz | 999.9 | — |
| `hz` | Hz | 2 | 9.999kHz | kHz | 9.999 | — |
| `hz` | Hz | 3 | 99.99kHz | kHz | 99.99 | — |
| `hz` | Hz | 4 | 999.9kHz | kHz | 999.9 | — |
| `hz` | Hz | 5 | 9.999MHz | MHz | 9.999 | — |
| `duty_cycle` | DutyCycle | 0 | Duty | % | 100 | 0 |
| `temp_c` | TempC | 0 | -40~300°C | °C | 300 | -40 |
| `temp_c` | TempC | 1 | 300~1000°C | °C | 1000 | 300 |
| `temp_f` | TempF | 0 | -40~572°F | °F | 572 | -40 |
| `temp_f` | TempF | 1 | 572~1832°F | °F | 1832 | 572 |
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
| `dc_a` | DcA | 0 | 6A | A | 6 [UNVERIFIED] | -6 [UNVERIFIED] |
| `dc_a` | DcA | 1 | 20A | A | 20 [UNVERIFIED] | -20 [UNVERIFIED] |
| `ac_a` | AcA | 0 | 6A | A | 6 [UNVERIFIED] | -6 [UNVERIFIED] |
| `ac_a` | AcA | 1 | 20A | A | 20 [UNVERIFIED] | -20 [UNVERIFIED] |
| `loz_v` | LozV | 0 | 600V | V | 600 | -600 |
| `loz_v` | LozV | 1 | 1000V | V | 1000 | -1000 |

The `dc_a`/`ac_a` rows are the manual's values in the UT61B+'s order; the
deck gives 10A at index 1, and no UT61D+ has confirmed either (§5.5).

### UT60BT (9,999 counts)

Source: `ut60bt.rs` (45 ranges).

**VENDOR + MANUAL**, unconfirmed on hardware. Which modes and rungs exist,
their order and units are the iDMM2.0 asset `funOl1_UT60BT.json`; the labels
are the UT60BT manual's printed ranges (§X, P2) where it prints the rung, the
asset's otherwise. The full-scale columns are the asset's bounds as it gives
them.

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 999.9mV | mV | 999.9 | -999.9 |
| `dc_v` | DcV | 1 | 9.999V | V | 9.999 | -9.999 |
| `dc_v` | DcV | 2 | 99.99V | V | 99.99 | -99.99 |
| `dc_v` | DcV | 3 | 999.9V | V | 999.9 | -999.9 |
| `ac_v` | AcV | 0 | 999.9mV | mV | 999.9 | — |
| `ac_v` | AcV | 1 | 9.999V | V | 9.999 | — |
| `ac_v` | AcV | 2 | 99.99V | V | 99.99 | — |
| `ac_v` | AcV | 3 | 999.9V | V | 999.9 | — |
| `dc_mv` | DcMv | 0 | 9.999mV | mV | 9.999 | -9.999 |
| `dc_mv` | DcMv | 1 | 99.99mV | mV | 99.99 | -99.99 |
| `ac_mv` | AcMv | 0 | 9.999mV | mV | 9.999 | — |
| `ac_mv` | AcMv | 1 | 99.99mV | mV | 99.99 | — |
| `ohm` | Ohm | 0 | 999.9Ω | Ω | 999.9 | — |
| `ohm` | Ohm | 1 | 9.999kΩ | kΩ | 9.999 | — |
| `ohm` | Ohm | 2 | 99.99kΩ | kΩ | 99.99 | — |
| `ohm` | Ohm | 3 | 999.9kΩ | kΩ | 999.9 | — |
| `ohm` | Ohm | 4 | 9.999MΩ | MΩ | 9.999 | — |
| `ohm` | Ohm | 5 | 99.99MΩ | MΩ | 99.99 | — |
| `capacitance` | Capacitance | 0 | 9.999nF | nF | 9.999 | — |
| `capacitance` | Capacitance | 1 | 99.99nF | nF | 99.99 | — |
| `capacitance` | Capacitance | 2 | 999.9nF | nF | 999.9 | — |
| `capacitance` | Capacitance | 3 | 9.999µF | µF | 9.999 | — |
| `capacitance` | Capacitance | 4 | 99.99µF | µF | 99.99 | — |
| `capacitance` | Capacitance | 5 | 999.9µF | µF | 999.9 | — |
| `capacitance` | Capacitance | 6 | 9.999mF | mF | 9.999 | — |
| `capacitance` | Capacitance | 7 | 99.99mF | mF | 99.99 | — |
| `hz` | Hz | 0 | 9.999Hz | Hz | 9.999 | — |
| `hz` | Hz | 1 | 99.99Hz | Hz | 99.99 | — |
| `hz` | Hz | 2 | 999.9Hz | Hz | 999.9 | — |
| `hz` | Hz | 3 | 9.999kHz | kHz | 9.999 | — |
| `hz` | Hz | 4 | 99.99kHz | kHz | 99.99 | — |
| `hz` | Hz | 5 | 999.9kHz | kHz | 999.9 | — |
| `hz` | Hz | 6 | 9.999MHz | MHz | 9.999 | — |
| `hz` | Hz | 7 | 99.99MHz | MHz | 99.99 | — |
| `duty_cycle` | DutyCycle | 0 | Duty | % | 99.9 | 0 |
| `temp_c` | TempC | 0 | -40~1000°C | °C | 1000 | -40 |
| `temp_f` | TempF | 0 | -40~1832°F | °F | 1832 | -40 |
| `diode` | Diode | 0 | Diode | V | 9.999 | 0 |
| `continuity` | Continuity | 0 | Cont | Ω | 999.9 | — |
| `dc_ua` | DcUa | 0 | 999.9µA | µA | 999.9 | -999.9 |
| `ac_ua` | AcUa | 0 | 999.9µA | µA | 999.9 | — |
| `dc_ma` | DcMa | 0 | 999.9mA | mA | 999.9 | -999.9 |
| `dc_ma` | DcMa | 1 | 9.999A | A | 9.999 | -9.999 |
| `ac_ma` | AcMa | 0 | 999.9mA | mA | 999.9 | — |
| `ac_ma` | AcMa | 1 | 9.999A | A | 9.999 | — |

Capacitance rung 7 and Hz rungs 0 and 7 are the asset's alone: the manual
stops at 9.999mF and prints frequency as one "99.99Hz~9.999MHz" row. The
asset labels its Hz rungs with a fifth digit ("9.9990Hz") and MHz as "mHz";
the labels above follow its bounds. Ω rung 4 is "9.99MΩ" in the asset, over
a 9.999 bound. The A rung is range byte 1 of the mA modes: the manual's A
mA position reads past 999.9mA on the same terminal. The asset lists DC A and
AC A with no rungs.

### UT202BT (9,999 counts)

Source: `ut202bt.rs` (38 ranges).

**VENDOR + MANUAL**, unconfirmed on hardware, as for the UT60BT: the asset
is `funOl1_UT202BT.json`, the labels are the UT202T/UT202BT manual's (spec
tables, P14/26 to P17/31) where it prints the rung.

| Table | Modes | Idx | Label | Unit | Full scale (+) | Full scale (−) |
|---|---|---|---|---|---|---|
| `dc_v` | DcV | 0 | 9.999V | V | 9.999 | -9.999 |
| `dc_v` | DcV | 1 | 99.99V | V | 99.99 | -99.99 |
| `dc_v` | DcV | 2 | 600.0V | V | 610 | -610 |
| `ac_v` | AcV | 0 | 9.999V | V | 9.999 | -9.999 |
| `ac_v` | AcV | 1 | 99.99V | V | 99.99 | -99.99 |
| `ac_v` | AcV | 2 | 600.0V | V | 610 | -610 |
| `lpf_v` | LpfV | 2 | 600.0V | V | 610 | -610 |
| `ac_a` | AcA, ClampAcA | 0 | 9.999A | A | 9.999 | -9.999 |
| `ac_a` | AcA, ClampAcA | 1 | 99.99A | A | 99.99 | -99.99 |
| `ac_a` | AcA, ClampAcA | 2 | 600.0A | A | 610 | -610 |
| `lpf_a` | ClampLpfA | 2 | 600.0A | A | 610 | -610 |
| `inrush` | Inrush | 0 | 9.999A | A | 9.999 | -9.999 |
| `inrush` | Inrush | 1 | 99.99A | A | 99.99 | -99.99 |
| `inrush` | Inrush | 2 | 600.0A | A | 610 | -610 |
| `ohm` | Ohm | 0 | 99.99Ω | Ω | 99.99 | — |
| `ohm` | Ohm | 1 | 999.9Ω | Ω | 999.9 | — |
| `ohm` | Ohm | 2 | 9.999kΩ | kΩ | 9.999 | — |
| `ohm` | Ohm | 3 | 99.99kΩ | kΩ | 99.99 | — |
| `ohm` | Ohm | 4 | 999.9kΩ | kΩ | 999.9 | — |
| `ohm` | Ohm | 5 | 9.999MΩ | MΩ | 9.999 | — |
| `ohm` | Ohm | 6 | 99.99MΩ | MΩ | 99.99 | — |
| `capacitance` | Capacitance | 0 | 99.99nF | nF | 99.99 | — |
| `capacitance` | Capacitance | 1 | 999.9nF | nF | 999.9 | — |
| `capacitance` | Capacitance | 2 | 9.999µF | µF | 9.999 | — |
| `capacitance` | Capacitance | 3 | 99.99µF | µF | 99.99 | — |
| `capacitance` | Capacitance | 4 | 999.9µF | µF | 999.9 | — |
| `capacitance` | Capacitance | 5 | 9.999mF | mF | 9.999 | — |
| `capacitance` | Capacitance | 6 | 99.9mF | mF | 105 | — |
| `hz` | Hz | 0 | 99.99Hz | Hz | 100 | -100 |
| `hz` | Hz | 1 | 999.9Hz | Hz | 1000 | -1000 |
| `hz` | Hz | 2 | 9.999kHz | kHz | 10 | -10 |
| `hz` | Hz | 3 | 99.99kHz | kHz | 100 | -100 |
| `hz` | Hz | 4 | 999.9kHz | kHz | 1000 | -1000 |
| `hz` | Hz | 5 | 9.999MHz | MHz | 10 | -10 |
| `continuity` | Continuity | 0 | 999.9Ω | Ω | 999.9 | — |
| `continuity` | Continuity | 1 | 999.9Ω | Ω | 999.9 | — |
| `temp_c` | TempC | 1 | -40~1000°C | °C | 1010 | -50 |
| `temp_f` | TempF | 0 | -40~1832°F | °F | 590 | -58 |

LPF V, LPF A and °C are listed at the range bytes shown and no others. AC A
is under both 0x11 and 0x16, the two codes the app names "ACA"; which one the
meter sends is unknown. Inrush rung 0 and continuity rung 1 are the asset's
alone. The capacitance top rung is printed "99.9mF" in the manual and "105mF"
in the asset. The manual has no frequency table: frequency shows on the
auxiliary display in AC V and AC A. The asset's °F bounds (590, −58) do not
match its own label.

---

## 10. Cross-reference with Community Sources [COMMUNITY]

Read 2026-09-25 for the UT60BT and UT202BT, after the iDMM2.0 read. The
sources, their commits and the boundary are in `reverse-engineering-approach.md`
("2026-09-25: UT60BT and UT202BT"); the 2026-09-22 read on reading the family
over Bluetooth is UT61E+ spec §7. Only the UT60BT has been run by a community
source; no UT202BT capture exists anywhere.

- **Framing over the meter's own radio.** On a live UT60BT each 19-byte frame
  arrives whole in one notification, never split, its checksum valid, and
  neither the UT-D07B's `AB CD 06 AA AA …` heartbeat nor any other adapter
  frame appears ([libreble/multimeter](https://github.com/libreble/multimeter),
  `docs/protocols/uni-t.md`).
- **The 0x5E poll is answered.** iDMM2.0 never sends it (§6.4), but
  [QtDMM](https://github.com/qtdmm/QtDMM) and
  [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy) poll a UT60BT with
  `AB CD 03 5E 01 D9` and get one 19-byte frame per request; QtDMM's dial walk
  of 2026-09-24 was read that way.
- **Buttons.** libreble's UT60BT client sends 0x41 MAX/MIN, 0x46, 0x47,
  0x48, 0x49 Hz/duty, 0x4A, 0x4B backlight and 0x4C, and says they all take
  effect. 0x49 and 0x4B are beyond the app's set (§6.5); 0x41 is one the app
  never sends. Its UT202BT client, unrun on a meter, sends 0x31, 0x33, 0x35,
  0x36, 0x46 and 0x4A, all in the app's set.
- **Secondary display.** Every community decoder masks bit 7 of the mode
  byte (§2.3); libreble finds it always clear on a UT60BT.
