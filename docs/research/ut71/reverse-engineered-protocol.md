# UT71A–E / Voltcraft VC920/VC940/VC960: Reverse-Engineered Protocol Specification

Protocol specification for the UNI-T UT71A/B (20000-count) and UT71C/D/E
(40000-count) handheld multimeters and their Voltcraft rebrands VC920,
VC940 and VC960.

The UT71 puts on the wire what the UT804 does: the same 11-byte packet,
the same field layout, the same function and range codes, read by the same
parser (§2). The UT804 spec, `../ut803/reverse-engineered-protocol.md`,
carries the shared detail; this document links to it and records what the
UT71 sources add or contradict.

Based on:
- UNI-T's UT71 interface protocol sheet "UT71通信协议.xls", and its twin
  "VC920_940_960 Protocol.xls" with identical cells, from the "UT71系列接口协议"
  download (`references/ut71/SOURCE.txt`)
- Conrad's VC920/VC960 protocol sheets: two of 2005-2006
  (123296DS01, 123297DS01) and one of 2010 for the VC960 (123298DS02),
  `references/vc920/other/SOURCE.txt`
- UNI-T's UT804 sheet "UT804接口协议", from the same template (author
  UNIT-hwj, created 2005-07-04), through the UT804 spec
- The UT71A/B/C/D/E operating manual (EN REV.3 2022; CN 2009), the UT71
  datasheet, the CN product sheets and the two interface-software manuals
  (`references/ut71/manual-notes.md`)
- The Voltcraft VC920/VC940/VC960 operating instructions (2009 "11/10"
  DE/EN/FR/NL, 2005 DE, 2006 EN "Model VC940"), the USB adapter 120317
  manual and the VC9x0 interface-software manual (same notes file)
- Ghidra decompilation of UNI-T's UT71A_B.exe and UT71C_D_E.exe V3.00,
  compared instruction for instruction with UT804.exe V2.00
  (`references/ut71/software-notes.md`)

Confidence levels:
- **[VENDOR-DOC]** — stated in UNI-T's UT71 sheet; Conrad's sheets are
  named where they add to it
- **[KNOWN]** — stated in the UT71 or Voltcraft manuals
- **[VENDOR]** — confirmed by analyzing UNI-T's UT71 apps
- **[DEDUCED]** — logical inferences from available evidence
- **[UNVERIFIED]** — requires real device testing to confirm
- **[HARDWARE]** — seen on a real meter: none yet for this family

Citations: "EN p.N" and "CN p.N" are the UT71 English and Chinese manuals'
printed pages, "VC09 p.N" the 2009 Voltcraft manual's English printed
page, "VC06 p.N" the 2006 VC940 manual (keys as in `manual-notes.md`).

---

## 1. Transport and Line

### 1.1 Cable and Bridge

**UT71.** The manual names a "USB interface cable" (EN p.6; CN p.4) whose
infrared receiver plugs into the window slot on the meter's upper back
(CN p.26) [KNOWN]. No UT71 document names UT-D04 or a bridge chip.

The apps filter on no VID or PID: they list every HID device by product
string and preselect the last one named `USB to Serial` without a serial
string; they configure it with the CH9325 feature-report layout of §1.2
[VENDOR]. Which bridge the cable carries — CH9325 (1A86:E008) or HE2325U
(04FA:2490) — is [UNVERIFIED].

**Voltcraft.** The supplied cable is an "RS232 optical interface cable"
to a free COM port; an optical USB adapter is optional (VC09 p.35, p.53;
VC06 p.4 "RS232C interface cable") [KNOWN]. The adapter 120317's manual
says its driver is "already installed on Windows 98 and later" and that
ComSetup is "not required for USB" (adapter manual p.3); no chip, VID or
PID is named. The VC9x0 interface-software manual has the same
`COMSetup` / `USB Connect` steps as the UT71 apps (123298IN01 p.3).

### 1.2 Line

| Source | Line |
|---|---|
| UT71 sheet [VENDOR-DOC] | "8bits,2400,none parity,1 stop bit" |
| Conrad 2005-2006 sheets [VENDOR-DOC] | "8-bit character enable odd verify", 2400 |
| Conrad 2010 VC960 sheet, UT804 sheet [VENDOR-DOC] | "7-bit character enable odd verify", 2400 |
| UT71 apps [VENDOR] | COM port opened at 2400 8N1; HID feature report `xx 60 09 00 00 03 00 00 00 00` = 2400 baud, as UT804.exe sends it (UT804 spec §1.2) |
| A UT804 (UT804 spec §1.2) [HARDWARE, UT804 only] | 7O1: bit 7 of every byte is odd parity |

The apps read only low nibbles, except that the RS232 handler matches CR
as the whole byte 0x0D (§2); at 8N1 that rules out even and mark parity
and admits none or odd (0x0D has three set bits, so its odd-parity bit is
0) [DEDUCED]. Whether a UT71 sends parity in bit 7, as the UT804 does, is
[UNVERIFIED]; the sheets disagree among themselves.

---

## 2. Packet

11 bytes: 9 data bytes, then `0x0D`, `0x0A`. No checksum. Data is in the
low nibbles [VENDOR-DOC] [VENDOR]. Layout, bytes numbered from 0 as on the
sheet:

| Byte | Sheet name | Content | Section |
|---|---|---|---|
| 0-4 | LCD(1)-LCD(5) | Digits, most significant first | §3.1 |
| 5 | Range | Range code | §3.5 |
| 6 | Function | Function code | §3.2 |
| 7 | STATE_ACDC | Coupling | §3.3 |
| 8 | +/-, STATE_ATUO | Sign, AUTO/Manual | §3.4 |
| 9 | — | `0x0D` | |
| 10 | — | `0x0A` | |

The UT804 spec numbers the same fields as nibbles 1-11 (its §3.1): its
nibble n is byte n−1 here.

**High nibble.** The UT71 sheet leaves it `xxxx` on every data byte; the
UT804 sheet and all three Conrad sheets fix it at `0011` [VENDOR-DOC].
The apps never test it (below), so for a UT71 it is [UNVERIFIED]; with
bit-7 parity (§1.2) it would read `0011` or `1011`.

**What the apps check** — the same handlers as UT804.exe, instruction for
instruction (UT804 spec §2.2, §2.3) [VENDOR]:
- RS232: a byte equal to 0x0D ends a packet; the packet parses if the low
  nibble of the byte before its 9 data bytes is `A` (the previous LF). CR
  must be 0x0D as a whole byte; LF is checked by its low nibble only. The
  first packet after opening the port is dropped.
- USB: a HID report without payload ends a packet; the first 11 buffered
  bytes parse if the low nibbles of bytes 9-10 are `D`, `A`. Bit 7 of CR
  and LF is ignored here.
- The parser (`LcdDisplay71A`, UT71A_B.exe 0x559280, UT71C_D_E.exe
  0x559284) re-checks `D`, `A` at positions 10-11; both callers guarantee
  it.

The unused UT60A/B/C path with its 14-byte FS9721 check is present in
both UT71 apps as in UT804.exe (UT804 spec §2.4) and reachable from
neither.

---

## 3. Data Format

### 3.1 Digits, Blank, Overload and LO — [VENDOR]

Bytes 0-4 carry digit values `0`-`9` and the flags of the UT804 spec §3.2
and §3.3, decoded by the same code:

| Byte | Value | Meaning |
|---|---|---|
| 0-4 | `0`-`9` | The digit |
| 4 | `A` | Fifth digit blank: 4000-count display (below) |
| 0 | `A` | Overload or LO packet: byte 1 = `C` is LO (app text `L0.`), any other byte 1 an overload (`0L` with the range's point, negative with the sign bit); bytes 2-4 are ignored |
| 3 | `B` | The app shows five zeros; meaning [UNVERIFIED] |

**[VENDOR-DOC]** The Conrad sheets add what the UT71 sheet lacks: LCD(1)
is `0-4`, LCD(5) is `A` at 4000 counts, and the 2010 sheet notes
`12:L`, the `C` = `L` that the UT804 sent (UT804 spec §3.2). The UT71A/B's
20000-count display keeps LCD(1) at most 2 [DEDUCED].

4000-count displays, and so byte 4 = `A` [DEDUCED from the sheets and the
app], come from:
- the blue key held at power-on, on every function (EN p.20; CN p.26;
  VC09 p.39, p.46) [KNOWN];
- RANGE held at power-on, resistance only (EN p.16, p.35) [KNOWN];
- the Voltcraft VC920/940/960, whose resistance is fixed at 4000 counts
  (VC09 p.46, p.59; VC06 p.18) [KNOWN].

The decimal point is not sent: the function and range codes place it
(§3.5). The UT71E's power position also shows VA and cos φ on the
secondary displays (EN p.22, p.45); the apps read no second value, and what,
if anything, carries it is [UNVERIFIED].

### 3.2 Function Codes (Byte 6)

Codes 0-15 are sent as hex digits `0`-`F` [VENDOR]. Sheet names are the
UT71 sheet's [VENDOR-DOC]; the unit and point are what the apps draw, the
point given as "after digit n" counted from the left [VENDOR]; dial
positions are the manuals' (EN p.13-14 Table 2-1, p.41; CN p.3; VC09
p.42) [KNOWN].

| Code | Sheet | App unit; point | Dial position | Models |
|---|---|---|---|---|
| 0 | `AC_mV` | none: the app sets neither unit nor point | none in any UT71 or Voltcraft manual | none; whether ever sent [UNVERIFIED] |
| 1 | `DCV` | V; by range | V DC (A-D); V≂ (E; blue key toggles AC) | all |
| 2 | `ACV` | V; by range. Coupling 3 = AC+DC (§3.3) | V AC (A-D); V≂ + blue (E) | all |
| 3 | `DC_mV` | mV; after digit 3 | mV DC: own position (A, E); shared with Hz/% (B-D) | all |
| 4 | `Ohm` | Ω, kΩ, MΩ; by range | Ω | all |
| 5 | `C` | nF, µF, mF; by range | capacitance | all |
| 6 | `℃` | °C; after digit 4 | °C/°F | B-E; VC9x0 |
| 7 | `uA` | µA; by range | µA≂ | all |
| 8 | `mA` | mA; by range | mA≂ | all |
| 9 | `10A` | A; after digit 2 | A≂ | all |
| A | `Fm` | continuity: beeper + Ω; after digit 3 | Ω, blue key | all |
| B | `Diode` | V; after digit 1 | Ω, blue key | all |
| C | `Hz` | Hz, kHz, MHz; by range. Duty % with the sign bit (§3.4) | %/Hz (A); mV/Hz/% (B-D, VC920/960); °C/°F/Hz/% (E, VC940) | all |
| D | `℉` | °F; after digit 4 | °C/°F, blue key | B-E; VC9x0 |
| E | blank | W; after digit 4 | W | UT71E, VC940 |
| F | `%(4-20mA)` | `mA%`; after digit 3 | mA, blue key | B-E; VC9x0 |

- Code 0 (AC mV) is on the sheet only: no UT71 or Voltcraft document has
  an AC mV function (EN p.59-69; VC09 p.58-60), and the app's case for it
  is empty. Coupling 0 on codes 0, 1, 3, 7, 8, 9 gets the app's "DC" record
  label (§3.3).
- Code A `Fm` (likely 蜂鸣, buzzer) is continuity: the app lights the
  beeper and Ω annunciators [VENDOR].
- Code E is **power** [DEDUCED]: the sheet leaves its name blank (Conrad's
  2005-2006 sheets say "Unspecified" / "leer"), the app draws unit W with the
  point after digit 4, and the UT71E and VC940 alone have a W position
  reading 0-2500 W (EN p.69; VC09 p.34, p.52). Not sent by any other
  model, which has no such position.
- Conrad's 2006 sheet (123297DS01) labels code 13 "Duty Cycle"
  beside the °F full scale 1832; the UT71 sheet and the app make 13 °F,
  and the other Conrad sheets leave its name blank.

### 3.3 Coupling (Byte 7)

| Value | Sheet [VENDOR-DOC] | App [VENDOR] |
|---|---|---|
| 0 | OFF | No annunciator; the record label is the function's default (DC for V, mV, µA, mA, A; OHM, CAP, Temperature, Diode, Frequency for the others; none for AC V, W, %) |
| 1 | AC | AC + T-RMS |
| 2 | DC | DC |
| 3 | AC+DC | AC + T-RMS + DC |

Values 4-9 set nothing; `A`-`F` make the app's integer parse fail. Value 3
comes from the yellow AC+DC key in AC V and AC A (EN p.20, p.31, p.33;
VC09 p.44) [KNOWN]. Whether a UT71 sends 0 or 2 on DC readings is
[UNVERIFIED]; the UT804 sends 0 (UT804 spec §3.5).

### 3.4 Status (Byte 8)

| Bit | UT71 sheet [VENDOR-DOC] | Conrad and UT804 sheets [VENDOR-DOC] | App [VENDOR] |
|---|---|---|---|
| 3 | `x` (not given) | Sign: `0xxx` +, `1xxx` − | Stripped, never used |
| 2 | Sign: `x0xx` +, `x1xx` − | Part of the mode field | Sign: `-` prefix; in Hz mode, duty cycle in % instead |
| 1:0 | `xx01` AUTO, `xx10` Manual | `x000` OFF, `x001` AUTO, `x010` Manual | `01` shows AUTO; `10`, `00`, `11` show nothing |

The UT71 sheet and the apps agree on the sign in bit 2; the UT804, whose
sheet puts it in bit 3, also sends it in bit 2 (UT804 spec §3.6). Bit 3
on a UT71 is [UNVERIFIED]. The app parses this byte as a decimal digit, so
a value `A`-`F` (bit 3 with bit 1 or 2) would fail; if the app works, the
meter keeps byte 8 within `0`-`9` [DEDUCED].

Duty cycle: code C with the sign bit set is duty in %, point after
digit 3 [VENDOR]. HOLD, REL, MAX MIN, PEAK and low battery have no bit
the app reads; the manuals do not say whether they change what is sent
[UNVERIFIED]. On the UT804, HOLD stops the output and REL sends the
relative reading with Manual set (UT804 spec §3.6, §4.2).

### 3.5 Range Codes and Full Scales (Byte 5)

The range code places the decimal point: the app's tables put it where
the full scale less one count reads (UT804 spec §3.7 has the UT804's
identical table) [VENDOR]. The code-to-point mapping is the same for every
model; what differs is the full scale a code stands for.

| Function | UT71C/D/E — sheet [VENDOR-DOC], manual EN p.59-69 [KNOWN] | UT71A/B — manual EN p.60, p.61 Table B, p.63-68; A/B app chart scales [KNOWN] [VENDOR] | VC920/940/960 — VC09 p.58-60 [KNOWN] |
|---|---|---|---|
| DC V (1) | 1: 4 V, 2: 40 V, 3: 400 V, 4: 1000 V | 2, 20, 200, 1000 V | as C/D/E |
| AC V (2) | 1: 4 V, 2: 40 V, 3: 400 V, 4: **1000 V** (sheet: 750 V) | 2, 20, 200, 1000 V | 4, 40, 400, **750 V** |
| DC mV (3) | 0: 400 mV | 200 mV | 400 mV |
| Ω (4) | 1: 400 Ω, 2: 4 kΩ, 3: 40 kΩ, 4: 400 kΩ, 5: 4 MΩ, 6: 40 MΩ | 200 Ω, 2 kΩ, 20 kΩ, 200 kΩ, 2 MΩ, 20 MΩ | as C/D/E, at 4000 counts |
| C (5) | 1: 40 nF, 2: 400 nF, 3: 4 µF, 4: 40 µF, 5: 400 µF, 6: 4 mF, 7: 40 mF | 20 nF … 20 mF | as C/D/E |
| °C (6) | 0: 1000 °C | B: 1000 °C | 1000 °C |
| µA (7) | 0: 400 µA, 1: 4000 µA | 200, 2000 µA | as C/D/E |
| mA (8) | 0: 40 mA, 1: 400 mA | 20, 200 mA | as C/D/E |
| A (9) | 0: 10 A | 10 A | 10 A |
| Continuity (A) | 0, no full scale on the sheet; point after digit 3 | — | — |
| Diode (B) | 0, no full scale; point after digit 1 | — | — |
| Hz (C) | 0: 40 Hz, 1: 400 Hz, 2: 4 kHz, 3: 40 kHz, 4: 400 kHz, 5: 4 MHz, 6: 40 MHz, 7: 400 MHz | 20 Hz … 200 MHz | manual lists 4 kHz-400 MHz only |
| °F (D) | 0: 1832 °F | B: 1832 °F | °F exists (blue key), no spec row |
| W (E) | not on the sheet; 2500 W (E); point after digit 4 | — | 2500 W (VC940) |
| % (F) | 0, no full scale; 0.01 % resolution; point after digit 3 | B: same | same |

- **AC V range 4.** The sheet says 750 V; every UT71 document says 1000 V
  (EN p.59, p.61 Table B; CN p.31; datasheet; both product sheets), and
  the Voltcraft manuals 750 V (VC09 p.34, p.47, p.58; VC06 p.18). Either
  way the point sits after digit 4 (`1000.0`, `750.0`); which meter clips
  where is [UNVERIFIED].
- **UT71A/B.** The manual gives half the full scale on every range but
  1000 V, 10 A and temperature (EN p.5, p.60-68), and the A/B app's chart scale per range code matches
  (2, 20, 200, 1000 V; 200 mV; 200 Ω … 20 MΩ; 20 nF … 20 mF; 200 and
  2000 µA; 20 and 200 mA; 20 Hz … 200 MHz) while it keeps the C/D/E
  point positions. So range code 1 on DC V reads `1.9999` on a UT71A/B
  and `3.9999` on a UT71C/D/E, with the same bytes. The range codes
  themselves have not been seen from a UT71A/B [UNVERIFIED].
- **10 A** is range 0 on the sheet; the UT804 sends 1 (UT804 spec §3.7).
  The point is after digit 2 for any code.
- The codes a UT71 sends in its 4000-count modes are [UNVERIFIED].

---

## 4. Models and Rebrands

### 4.1 UT71A-E — [KNOWN]

| | UT71A | UT71B | UT71C | UT71D | UT71E |
|---|---|---|---|---|---|
| Counts (EN p.5; CN p.3) | 20000 | 20000 | 40000 | 40000 | 40000 |
| °C/°F, 4-20 mA % | — | ✓ | ✓ | ✓ | ✓ |
| Memory (readings, CN p.3) | none | 100 | 100 | 9999 | 100 |
| Power W/VA/cos φ | — | — | — | — | ✓ |
| V positions | DC and AC | DC and AC | DC and AC | DC and AC | one, V≂ |
| Hz/% position | own | on mV | on mV | on mV | on °C/°F |
| SEND | SEND key | hold MAXMIN | hold MAXMIN | hold MAXMIN | hold MAXMIN |

Data output:
- Holding MAXMIN/SEND for more than 1 s starts it and shows "SEND"; EXIT
  stops it (EN p.18; VC09 p.43). The UT71A's SEND key needs one press (EN
  p.19).
- SEND turns auto power-off off (CN p.26; EN p.18 "AUTO mode switch off";
  VC09 p.52). Store mode does the same (EN p.47).
- RECALL + ▶ (HOLD) sends every stored reading and ends by itself; the
  software shows each reading's store time and value (EN p.48; CN p.24;
  VC09 p.54). The app parses only the 11-byte packet of §2, so what
  carries the time is [UNVERIFIED].
- The primary display updates 2-3 times/s (EN p.56); the Voltcraft manual
  gives 3 measurements/s (VC09 p.57).
- Not stated in any manual: whether HOLD, REL or MAX MIN change what is
  sent.

UNI-T ships two apps, UT71A/B and UT71C/D/E [VENDOR]. They differ only in
the count mode (A/B: 20000, limits ±20001; C/D/E: 40000), the A/B chart
scales of §3.5 and two display details; the receive paths and the parser
are the same code. Nothing on the wire tells the two families apart
[DEDUCED]: the user picks the app.

### 4.2 Voltcraft VC920, VC940, VC960

UNI-T's "UT71系列接口协议" download holds the UT71 sheet twice, the second
copy named "VC920_940_960 Protocol.xls" with identical cells
(`references/ut71/SOURCE.txt`); Conrad files sheets from the UT804
template under its VC920/VC940/VC960 items (§1.2, §2, §3.1). The
Voltcraft manuals name no UNI-T model. The 2006 VC940 manual copies the
UT71 manual's chapter and table structure and much of its wording
(`manual-notes.md`).

Correspondence by dial layout, count and power function [DEDUCED from
features]: VC920 ≈ UT71C, VC960 ≈ UT71D, VC940 ≈ UT71E.

| | UT71C/D/E | VC920 / VC960 / VC940 |
|---|---|---|
| AC V top range | 1000 V | 750 V (VC09 p.34, p.47, p.58) |
| Resistance | 40000 counts; 4000 with RANGE at power-on | 4000 counts, fixed (VC09 p.46, p.59) |
| Memory | 100 / 9999 / 100 | 10 / 10000 / 10 (VC09 p.34, p.39, p.54) |
| Cable | USB (IR) | RS232 optical; USB adapter optional (§1.1) |
| SEND, auto power-off | as §4.1 | same (VC09 p.43, p.52, p.54) |

---

## 5. Implementation Notes

Wire facts a decoder meets:

- Packets are 11 bytes ending `0D 0A`, data in the low nibbles, no
  checksum (§2). The UT71 sheet fixes no high nibble; a UT71 may add odd
  parity in bit 7, as the UT804 does, in which case LF arrives as `8A`
  (§1.2).
- The meter sends nothing until SEND is on and needs nothing sent: the
  apps' only output is the CH9325 rate report (§1.2, UT804 spec §4.2).
- The decimal point is not sent; function and range codes place it, and
  the same code stands for half the full scale on a UT71A/B (§3.5). The
  packet does not identify the model, nor whether AC V range 4 is 750 V
  or 1000 V (§3.5, §4).
- Bytes 5, 7 and 8 stay within `0`-`9` [DEDUCED]: the apps' integer
  parses would fail on `A`-`F` (§3.3, §3.4).
- A fifth digit of `A` is a 4000-count display, and byte 0 = `A` an
  overload or LO packet (§3.1).
- The sign is bit 2 of byte 8; in Hz mode it marks duty cycle (§3.4).
- Code E carries power in W from a UT71E or VC940 [DEDUCED] (§3.2).
- The display updates 2-3 times a second (§4.1); whether packets follow
  it is [UNVERIFIED].

---

## 6. What Needs Hardware Verification

Everything here is from documents and the apps; no UT71 or VC9x0 packet
has been seen [UNVERIFIED]:

- The high nibble of the data bytes (`xxxx` on the UT71 sheet, `0011` on
  the others) and bit-7 parity, hence whether LF arrives as `0A` or `8A`
  (§1.2, §2)
- Which USB bridge each cable carries: CH9325 (1A86:E008) or HE2325U
  (04FA:2490), for the UT71 cable and for Conrad's adapter 120317 (§1.1)
- Code E power packets from a UT71E or VC940, and their point (§3.2);
  where VA and cos φ go, if anywhere (§3.1); what the UT71E sends when the
  blue key on its W position selects the circuit's V, A or Hz (CN p.21;
  EN Table 2-1 gives that key no function there)
- That code 0 (AC mV) is never sent (§3.2)
- AC V range 4: 1000 V on a UT71, 750 V on a VC9x0 (§3.5)
- Coupling on DC readings, 0 or 2 (§3.3)
- Status bit 3, and the AUTO/Manual field's values (§3.4)
- Digit values `B`, `D`-`F`; overload and LO packets as the LCD shows
  them (§3.1)
- The duty-cycle sign bit (§3.4)
- HOLD, REL, MAX MIN and PEAK on the wire (§3.4)
- What RECALL + ▶ sends, and how the store time gets to the software
  (§4.1)
- The UT71A/B's range codes and 4000-count codes (§3.5)
- The 10 A range code, 0 (sheet) or 1 (UT804) (§3.5)

---

## 7. Cross-Reference with Community Sources

Added after the independent analysis above; see the approach doc for the
boundary. The sigrok column is libsigrok's `ut71x` parser and its device
entries (read 2026-09-21, commit below). Nothing in §1-6 was changed
because of it: every ✗, "new" and open question is for hardware to
settle.

| Aspect | Our spec | sigrok | Agree? |
|---|---|---|---|
| Line | 2400; UT71 sheet 8N1, the other sheets odd parity with 7 or 8 bits; the apps open 8N1; 7O1 seen on a UT804 only (§1.2) | 2400 7O1 (`ut71x.c` header; serial entries `2400/7o1/rts=0/dtr=1`) | ✓ rate; ✗ parity against the UT71 sheet |
| Framing | 11 bytes, 9 data then CR LF, no checksum; the apps read low nibbles, the RS232 path matches CR as a whole byte (§2) | 11 bytes, valid only if bytes 9-10 are exactly `\r` `\n`; no checksum | ✓ |
| High nibble, bit 7 | UT71 sheet `xxxx`, UT804 and Conrad sheets `0011`; the apps never test it (§2) | Bytes 0-6 read as ASCII (`0`-`9`, `:`, `<`, `'0'` + code), so `0011`; bit 7 dropped by the 7-bit UART, or masked off from UT-D04 reports (`protocol.c`) | ✓ UT804/Conrad sheets; UT71 open |
| Digits, 4000 counts | `0`-`9`; byte 4 = `A` is the blank fifth digit of a 4000-count display (§3.1) | Bytes 0-4 must be digits; byte 4 = `:` is "4000 count mode", value × 10 | ✓ |
| Overload, LO | Byte 0 = `A`: byte 1 = `C` is LO, anything else an overload; the apps ignore bytes 2-4 (§3.1) | Only `::0<:` (A A 0 C A, over limit) and `:<0::` (A C 0 A A, under limit); other non-digits rejected | ✓ on those two; other byte-0 `A` packets unknown |
| Functions 1-9, B-D, F | §3.2 | Same meanings; F is loop current in % | ✓ |
| Function 0 | `AC_mV`, sheet only: no dial position, the apps set no unit or point (§3.2) | AC mV, point after digit 3 | ✓ meaning; ever sent? |
| Function A | Continuity, point after digit 3 (§3.2, §3.5) | Continuity, exponent −1: point after digit 4 | ✗ point |
| Function E | Power in W, point after digit 4, UT71E and VC940 [DEDUCED] (§3.2) | Power in W, "Only available on UT71E (range 0-2500W)", exponent 0: no point | ✓ meaning; ✗ point |
| Coupling | 0 OFF, 1 AC, 2 DC, 3 AC+DC (§3.3) | Bit 0 AC, bit 1 DC | ✓ |
| Status | Sign in bit 2 (UT71 sheet, apps; the UT804 sends it there), in bit 3 on the Conrad and UT804 sheets; bits 1:0 `01` AUTO, `10` Manual (§3.4) | Bit 0 auto, bit 1 manual, bit 2 sign; calls the Conrad sheets' bit-3 sign a typo; single-range modes set neither; auto with manual rejected | ✓ |
| Duty cycle | Hz with the sign bit; point after digit 3 (§3.4) | Hz with the sign bit; point from the Hz range code | ✓ flag; ✗ point except on range code 1 |
| Range codes, points | One code-to-point table for every model (§3.5) | One exponent table for every model, matching ours for V, mV, Ω, C, °C, °F, µA, mA, diode, Hz, loop % | ✓ |
| AC V range 4 | Point after digit 4; 1000 V in the UT71 documents, 750 V on the sheet and in the Voltcraft manuals (§3.5) | Exponent −1: point after digit 4; no full scale | ✓ point |
| 10 A range | Sheet: 0; the UT804 sends 1; the apps put the point after digit 2 for any code (§3.5) | Range 1 only (point after digit 2); calls the Conrad sheets' 0 a typo; range 0 gets no point | ✗ code 0 |
| UT71A/B | 20000 counts; the C/D/E codes and points at half the full scale (§3.5) | UT71A-E share one parser and table | ✓ |
| Models | UT71A-E; VC920, VC960, VC940 ≈ UT71C, D, E [DEDUCED] (§4) | UT71A-E, UT804, Voltcraft VC-920/940/960, Tenma 72-7730, 72-7732, 72-9380A | ✓; Tenma new |
| 4000-count Ω | Fixed on the VC9x0 (§3.1) | VC920 and VC940 4000 counts for resistance, the three Tenma 40000 (comment) | ✓; Tenma new |
| Cables | UT71: a "USB interface cable" with an IR head, no cable name; bridge CH9325 or HE2325U [UNVERIFIED]. Voltcraft: RS232 optical cable, USB adapter 120317 (§1.1) | Serial entries: "UT-D02 cable" (RS232) for every model but the UT804; `uni-t-dmm` entries: the same models on the UT-D04 HID cable (HE2325U or CH9325, per `protocol.c`), VID:PID given by the user, none listed | ✗ cable names in no UT71 document |
| CH9325 report | The apps send `xx 60 09 00 00 03 00 00 00 00` (§1.2) | `[lo, hi, 00, 00, 03]` | ✓ |
| Direction | Nothing sent but the rate report (§5) | "Unidirectional" | ✓ |

Reference implementation:

- [sigrok libsigrok](https://github.com/sigrokproject/libsigrok) — C;
  `src/dmm/ut71x.c`, its declarations in `src/libsigrok-internal.h`, the
  UT71x entries in `src/hardware/serial-dmm/api.c` and
  `src/hardware/uni-t-dmm/api.c`, and the parity masking in
  `src/hardware/uni-t-dmm/protocol.c`; master 0bc2487778 (2025-11-20),
  `ut71x.c` last changed in 94b1d50642 (2018-02-18).

---

## 8. Sources

- UNI-T "UT71系列接口协议" (ut71-series-interface-protocol.rar, md5
  210950ae…, server date 2021-10-30), listed on
  https://meters.uni-trend.com.cn/menu/68.html; holds "UT71通信协议.xls"
  (md5 c01e3922…) and "VC920_940_960 Protocol.xls" (md5 0173aa09…), both
  author UNIT-hwj, created 2005-07-04, last saved 2014-03-06 — line
  settings, packet layout, function and range tables
  (`references/ut71/SOURCE.txt`)
- Conrad protocol sheets, https://asset.conrad.com/media10/add/160267/c1/-/gl/000\<item\>\<slot\>:
  123296DS01 "VC920-VC960 protocol.xls" (2005-08-22, 16 functions),
  123297DS01 (2006-03-28, 16 functions on two pages, "Duty Cycle" at 13),
  123298DS02 "123298_VC960 Protocol.xls" (2010-10-08, "7-bit character",
  `12:L`) (`references/vc920/other/SOURCE.txt`)
- UNI-T "UT804接口协议" (md5 246ca062…) — via `../ut803/reverse-engineered-protocol.md`
  (`references/ut800/ut804/UT804接口协议-SOURCE.txt`)
- UT71A/B/C/D/E operating manual EN, P/N 110401111273X, 2022.8.18 REV.3
  (md5 694721bc…), https://meters.uni-trend.com/download/ut71a-b-c-d-e-user-manual/
  — dial tables (p.13-14, p.41), buttons (p.15-20), SEND (p.18-19),
  RECALL (p.48), counts (p.5, p.56), ranges (p.59-69), Table B (p.61)
- UT71A/B/C/D/E 使用说明书 CN, P/N 110401103946 (md5 3d6427ea…),
  https://meters.uni-trend.com.cn/static/upload/file/20211030/UT71ABCDE-cn-sms.pdf
  — function table (p.3), SEND and auto power-off (p.26), RECALL (p.24)
- UT71 datasheet (md5 b09d067b…), https://meters.uni-trend.com/download/ut71a-b-c-d-e-flyer/;
  CN product sheets UT71A/B (md5 373a61cd…) and UT71C/D/E (md5 a7859740…)
  — counts and AC V 1000 V
- UT71A/B and UT71C/D/E Interface Software Manuals (2011; md5 66d5d95e…,
  59164508…), https://meters.uni-trend.com/download/software-instruction-ut71a-b/
  and .../ut71c-d-e-software-instruction/ — USB port, "USB connect"
- UT71 interface software CD (ut71-interface-software-cd.rar, md5
  fa420851…, server date 2026-05-15), listed on
  https://meters.uni-trend.com.cn/menu/68.html; installs UT71A_B.exe
  (md5 1cc346ad…) and UT71C_D_E.exe (md5 85b8888f…) — Ghidra 12.1.3
  decompilation, RTTI-seeded handler decompile, annotated parser
  disassembly beside UT804.exe's (`references/ut71/software/SOURCE.txt`)
- Voltcraft VC920/VC940/VC960 operating instructions, Version 11/10 with
  note 01/13, 123296ML04 (md5 38861768…), English printed p.32-60 — cable
  (p.35, p.53), dial (p.42), keys and SEND (p.43-45, p.54), auto power-off
  (p.52), counts (p.39, p.46), ranges (p.58-60), rate (p.57)
- Voltcraft Bedienungsanleitung 07/05 (121700ML02) and 12/05 (121701ML02);
  "Model VC940" operating manual 2006 (121700ML03) — cable (p.4), SEND
  (p.7), RECALL (p.15), 750 V and 4000-count Ω (p.18-19)
  (`references/vc920/manuals/SOURCE.txt`)
- USB interface adapter 120317 operating instructions, Version 03/09
  (120317ML03) — driver and ComSetup (p.3); its software zip 120317DL00
  lists "VC920_960 Setup.exe" (not opened)
- "VC920/VC940/VC960 Computer Interface Software" (123298IN01, 2005) —
  RS232C cable, COMSetup and USB Connect (p.3)
