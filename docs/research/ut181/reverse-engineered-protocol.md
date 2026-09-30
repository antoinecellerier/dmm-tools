# UT181A Protocol: Reverse-Engineered Specification

Based on cross-referencing three independent community implementations:
- [antage/ut181a](https://github.com/antage/ut181a) (Rust, with [Protocol.md](https://github.com/antage/ut181a/blob/master/Protocol.md))
- [loblab/ut181a](https://github.com/loblab/ut181a) (C++)
- [sigrok uni-t-ut181a](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) (C)

All three implementations agree on every protocol detail documented
here.

Confidence levels:
- **[KNOWN]** -- confirmed by 3 independent implementations or official manual
- **[VENDOR]** -- read out of the vendor application `UT181A.exe`
  (V1.05); see `reverse-engineering-approach.md`
- **[DEDUCED]** -- logical inference
- **[HARDWARE]** -- seen on a real UT181A, with the issue, the cable and
  the reporter. The "Hardware-confirmed" notes are the same evidence:
  @diego351's meter on the CH9329 (UT-D09) cable, issue #5
- **[UNVERIFIED]** -- needs device testing

[KNOWN] here means the three community implementations agree, which is
not the same as tested on a meter. What a real UT181A has confirmed is
tagged [HARDWARE]; the open checks are in [verification.md](verification.md).

---

## 1. Device Overview -- [KNOWN]

| Parameter | Value |
|-----------|-------|
| Display | 60,000 counts, 4-5/6 digits |
| Screen | 3.5" 64K color TFT LCD (320x240) |
| Safety | CAT IV 600V / CAT III 1000V |
| DCV accuracy | 0.025% + 5 counts |
| Chipset | Cyrustek ES51997 analog frontend |
| MCU | STM32F103 |
| USB bridge | CP2110 (VID 0x10C4, PID 0xEA80) on older units; CH9329 (VID 0x1A86, PID 0xE429) on current production |
| Sample rate | 2 Sa/s (60K counts), 10 Sa/s (600 counts) |
| Data logging | 20,000 saved measurements |
| Recording | Up to 20 named recordings |
| Battery | 7.4V 2200 mAh Li-ion + CR2032 backup |

**Prerequisite**: User must enable communication on the meter before
each session: SETUP -> Communication -> ON. This setting resets on
power cycle.

---

## 2. Transport Layer -- [KNOWN]

### 2.1 USB Configuration

| Parameter | Value |
|-----------|-------|
| USB VID | 0x10C4 (Silicon Labs), or 0x1A86 (WCH) on a CH9329 cable |
| USB PID | 0xEA80 (CP2110 default), or 0xE429 (CH9329) |
| Baud rate | 9600 |
| Data bits | 8 |
| Parity | None |
| Stop bits | 1 |
| Flow control | None |

Older units use the same CP2110 bridge as the UT61E+ and UT8803, with
the standard CP2110 UART enable + configure. Current production ships a
UT-D09 built on a WCH CH9329 instead; the UART framing above is
identical either way, only the USB transport differs.

### 2.2 Header Byte Clarification

**Important**: The UT181A wire bytes are **0xAB, 0xCD** -- identical to
the UT61E+. The "reversed 0xCDAB header" description found in some
sources (including the antage Protocol.md) refers to reading these two
bytes as a little-endian uint16: `byte[0]=0xAB, byte[1]=0xCD` →
LE uint16 = 0xCDAB. The UT61E+ reads the same bytes as big-endian:
0xABCD. The **wire bytes are the same**; only the host-side integer
interpretation differs.

---

## 3. Frame Format -- [KNOWN]

```
+------+------+--------+--------+-----------+--------+--------+
| 0xAB | 0xCD | len_lo | len_hi | payload   | chk_lo | chk_hi |
+------+------+--------+--------+-----------+--------+--------+
 byte 0 byte 1 byte 2   byte 3   bytes 4..   last 2 bytes
```

| Field | Size | Encoding | Description |
|-------|------|----------|-------------|
| Magic | 2 | Fixed | `0xAB 0xCD` |
| Length | 2 | uint16 LE | `payload_size + 2` (includes checksum bytes) |
| Payload | N | Variable | Command or response data |
| Checksum | 2 | uint16 LE | Sum of all bytes from offset 2 through end of payload |

### Checksum Algorithm

```
checksum = sum of bytes[2] through bytes[3 + payload_size - 1]
         = length_lo + length_hi + payload[0] + ... + payload[N-1]
```

The checksum covers the length field and all payload bytes. It does
**not** include the 2-byte magic header.

[HARDWARE] Frames from two meters, both on the CH9329 (UT-D09) cable, fit
this layout and checksum: @alexander-magon's (PR #8, 2026-04-07) and
@diego351's (issue #5), about 8,100 of whose frames checksum in the
2026-09-27 capture.

**[VENDOR] confirmation.** The vendor app builds every outgoing frame in
one generic sender at `0x870408`, taking `(connection, opcode,
payload_ptr, payload_len)`. It allocates `payload_len + 7`, writes
`0xAB 0xCD`, then `payload_len + 3` as a uint16 LE length, then the
opcode byte, then the payload, then a uint16 LE sum of every byte from
offset 2 through the end of the payload. That is byte-for-byte the
`build_command` helper in `crates/dmm-lib/src/protocol/ut181a/command.rs`.
Every command below is a thin wrapper that fills a small stack buffer
and calls it.

### Key Differences from UT61E+

| Aspect | UT61E+ | UT181A |
|--------|--------|--------|
| Header bytes | 0xAB 0xCD | 0xAB 0xCD (same) |
| Length field | 1 byte | 2 bytes (uint16 LE) |
| Length meaning | Bytes after length (payload + checksum) | Payload + 2 (payload + checksum size) |
| Checksum | 16-bit BE sum of all bytes before checksum | 16-bit LE sum of length + payload only |
| Values | 7-byte ASCII display string | float32 LE (IEEE 754) |
| Communication | Polled (request/response) | Monitor mode (streaming) + commands |

---

## 4. Packet Types -- [KNOWN]

### 4.1 Response Types (Device -> Host)

| Code | Type | Description |
|------|------|-------------|
| 0x01 | Reply Code | OK (`0x4F4B` = "OK") or Error (`0x4552` = "ER") |
| 0x02 | Measurement | Real-time measurement data |
| 0x03 | Save | Saved measurement with timestamp |
| 0x04 | Record Info | Recording metadata (name, interval, stats) |
| 0x05 | Record Data | Recording samples (batched) |
| 0x72 | Reply Data | Generic data reply (e.g., saved/recording counts) |

Hardware-confirmed 2026-09-27 (issue #5): the meter answers every command
in §4.2 it was sent with a type 0x01 reply — `AB CD 05 00 01 4F 4B A0 00`
("OK") or the same frame carrying `45 52` ("ER") — within about 70 ms,
interleaved with the measurement stream. A SET_MODE's OK can arrive after
one more frame in the old mode.

### 4.2 Commands (Host -> Device)

| Code | Command | Parameters | Description |
|------|---------|------------|-------------|
| 0x01 | SET_MODE | uint16 LE mode word | Set measurement mode. [VENDOR] — wrapper at `0x8702b8` writes `[lo, hi]` and calls the sender with opcode 0x01; see §6.1 |
| 0x02 | SET_RANGE | uint8 (0x00 = auto, else 1-based) | Set range. [VENDOR] — wrapper at `0x8702d8` sends `is_auto ? 0 : index`; see §7.1 |
| 0x03 | SET_REFERENCE | float32 LE | Set relative reference value. [VENDOR] — wrapper at `0x87059c`, driven by the REL edit box (`tmrRelTimer`, `0x86f2c4`) |
| 0x04 | SET_MIN_MAX | uint8 (0 or 1) | Enable/disable min/max. [VENDOR] — wrapper at `0x870588` sends **one** payload byte, so the frame is 8 bytes total. antage and sigrok describe a uint32; the vendor app disagrees |
| 0x05 | SET_MONITOR | uint8 (0 or 1) | Enable/disable streaming. [VENDOR] — wrapper at `0x870344` |
| 0x06 | SAVE_MEAS | (none) | Save current measurement. [VENDOR] — wrapper at `0x8703fc`, zero-length payload, called from the Max/Min dialog's Save button |
| 0x07 | GET_SAVED_MEAS | uint16 LE index (1-based) | Retrieve saved measurement |
| 0x08 | GET_SAVED_COUNT | (none) | Get count of saved measurements |
| 0x09 | DEL_SAVED_MEAS | uint16 LE index (0xFFFF = all) | Delete saved measurement(s) |
| 0x0A | START_RECORDING | name(11) + interval(2) + duration(4) | Start recording |
| 0x0B | STOP_RECORDING | (none) | Stop recording |
| 0x0C | GET_REC_INFO | uint16 LE index (1-based) | Get recording metadata |
| 0x0D | GET_REC_SAMPLES | uint16 LE index + uint32 LE offset(1-based) | Get recording data. [VENDOR] — wrapper at `0x8703a4`, 6-byte payload |
| 0x0E | GET_REC_COUNT | (none) | Get count of recordings |
| 0x0F | DEL_RECORDING | uint16 LE index | Delete recording [VENDOR] — confirmed from UT181A.exe decompilation: called after "Are you sure that you want to delete this record?" dialog, followed by GET_REC_COUNT refresh. Not in community implementations. |
| 0x12 | HOLD (button press) | `0x5A` = HOLD button code | [VENDOR] — wrapper at `0x8703e8` hard-codes a single payload byte `0x5A` (`mov BYTE PTR [esp],0x5a`) and is the Hold action's only call. This matches antage's `toggle_hold`, the only community implementation that transmits 0x12; sigrok defines the opcode but never sends it. Whether bare `[0x12]` also works is untested. |

What a real meter did with them (2026-09-27, issue #5, CH9329 cable):

- **SET_MODE** moved the meter between variants of the dial's family —
  continuity to its open beeper, diode to its alarm, µA/mA DC to AC+DC and
  Peak, µA AC to Hz and Peak — and the stream showed the new word about
  150 ms later. Flipping nibble 0 to 2 entered REL on nS, Duty, Pulse
  width and the V/µA/mA DC AC+DC variants (§6.1).
- **SET_RANGE** with index *n* made the meter report range byte *n* with
  the auto bit clear, on every rung of the Cap, Hz, µA, mA and V DC AC+DC
  ladders; index 0 set the auto bit again. In a Peak variant it answered
  ER to index 0 (§7).
- **HOLD** `12 5A` toggled misc bit 7 on every dial position tried,
  Peak included.
- **SET_MIN_MAX** with one payload byte entered (1) and left (0) the
  min/max format; the meter answered ER to 1 in a Peak variant.

---

## 5. Measurement Packet (Type 0x02) -- [KNOWN]

### 5.1 Common Header (5 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | misc | Bit flags (see below) |
| 1 | 1 | misc2 | Bit flags (see below) |
| 2 | 2 | mode | uint16 LE mode word |
| 4 | 1 | range | 0x00 = auto, 0x01-0x08 = manual |

**misc byte**:

| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x01 | Set in every min/max frame, meaning unknown (see below) |
| 1 | 0x02 | Has aux1 display value |
| 2 | 0x04 | Has aux2 display value |
| 3 | 0x08 | Has bargraph / fast mode |
| 4-6 | 0x70 | Format: 0x00=normal, 0x10=relative, 0x20=min/max, 0x40=peak |
| 7 | 0x80 | HOLD active |

In the formats other than normal the presence bits do not describe the
layout (2026-09-27 capture): relative frames always set bits 1 and 2,
peak frames bit 1, and min/max frames bits 0, 1 and 2, with bit 3 kept
from the mode although no min/max frame carries a bargraph.

**misc2 byte**:

| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x01 | Auto-range active |
| 1 | 0x02 | High voltage warning |
| 3 | 0x08 | Lead error |
| 4 | 0x10 | COMP (comparator) mode active |
| 5 | 0x20 | Record mode active |

Hardware-confirmed 2026-09-27: bits 0 (cleared by SET_RANGE, and by
entering REL, min/max or a Peak variant), 1 (set on a 316 V AC reading
and on an overloaded mV AC input) and 4 (COMP, below). Bit 3 was set for
a few frames at a time on the V DC, µA DC and A DC dials and while
turning to mV DC. [HARDWARE] On 2026-09-28 it stayed set through every
A DC sample (misc2 `0x09`), and @diego351 accepted the capture's sample
line for them (issue #5, CH9329 / UT-D09; dmm-tools shows the bit as
`LEAD ERR`). Which LCD warning, if any, goes with it is [UNVERIFIED].

The meter sends a measurement frame every 100 ms; the main reading
changes at most every 500 ms (2 Sa/s), the frames in between repeat it.
On the temperature dial the reading changed every 700–900 ms instead,
both thermocouples at once (2026-09-28 capture).

### 5.2 Value Encoding

**Full value** (13 bytes):

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | float32 LE (IEEE 754) |
| 4 | 1 | Precision byte |
| 5 | 8 | Unit string (null-terminated) |

**Short value** (5 bytes, used in min/max sub-values):

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | float32 LE |
| 4 | 1 | Precision byte |

**Precision byte**:

| Bits | Meaning |
|------|---------|
| 0 | Positive overload (OL) |
| 1 | Negative overload (-OL) |
| 4-7 | Decimal places (0-15) |

[HARDWARE] (PR #8, CH9329 / UT-D09, @alexander-magon, 2026-04-07): V DC
readings arrive as float32 LE, and the decimal places in bits 4-7 give
sane display formatting.

Hardware-confirmed 2026-09-27 (issue #5):

- An overload sets one bit and keeps the out-of-range float: open leads
  on Ω read `0x31` with 62.96 MΩ, an overloaded mV AC input `0x21` with
  690.13 mV, a Peak minimum `0x12` with -959.36 mV.
- **Both bits set, with a float of 0.0, is a blank value** — the meter
  has no reading to give. Every value slot goes blank for six or seven
  frames after a range, mode or dial change, and for up to 20 (2 s) on
  Ω, mV AC, the current dials and the AC+DC and Peak variants
  (2026-09-28 capture). A capacitance range with nothing connected
  stays blank, and the min/max and Peak slots are blank until their
  first reading lands. The capture never showed both bits with any
  other float.

### 5.3 Measurement Variants

**Normal (format 0x00)** -- after 5-byte header:
- Main value: 13 bytes (float32 + precision + unit)
- Aux1: 13 bytes (optional, if misc bit 1 set)
- Aux2: 13 bytes (optional, if misc bit 2 set)
- Bargraph: float32 + 8-byte unit (optional, if misc bit 3 set) --
  **12 bytes, no precision byte**

Hardware-confirmed 2026-09-02 (issue #5): a V AC frame with all three
optional fields present has a 57-byte payload, which only accounts as
6 + 13 + 13 + 13 + 12. The bargraph's missing precision byte is what
makes the arithmetic close. [HARDWARE] The same accounting holds for a
32-byte temperature payload (`0x4211`, two thermocouples: 6 + 13 + 13)
from @diego351's meter and a 31-byte V DC payload (`0x3111`: 6 + 13 + 12)
from @alexander-magon's (PR #8), both on the CH9329 (UT-D09) cable.
The bargraph is not the displayed value (241.02 VAC beside a 239.22 VAC
main reading): it is the meter's fast sample, which changes on every
100 ms frame while the main reading holds for 500 ms (2026-09-27
capture, Ω and mV readings with a drifting input).

What the aux slots hold depends on the mode word (2026-09-27 capture,
every variant below seen on a real meter):

| Variant | Aux1 | Aux2 | Main |
|---|---|---|---|
| V AC / mV AC / µA, mA, A AC Hz (n1=2) | frequency, `Hz` | period, `ms` | the AC reading |
| AC+DC (`0x2141`, `0x3121`, `0x8121`, `0x9121`, `0xA121`) | AC part | DC part | √(AC² + DC²), unit `…ac+dc`; no bargraph |
| V AC dBV (`0x1151`) | the voltage, `VAC` | -- | 20·log10(V / 1 V) |
| V AC dBm (`0x1161`) | the voltage, `VAC` | reference impedance, 600 `~` | 10·log10(V² / Z / 1 mW) |
| Temperature T1,T2 / T2,T1 (n1=1/2) | the other probe | -- | T1 / T2 |
| Temperature T1-T2 / T2-T1 (n1=3/4) | T1 | T2 | T1 − T2 / T2 − T1 |

A probe that is not plugged in reads as a positive overload.

**Relative (format 0x10)**:
- Relative value: 13 bytes
- Reference value: 13 bytes
- Absolute value: 13 bytes
- Fast value: conditional on misc bit 3

Hardware-confirmed 2026-09-27: 45-byte frames without and 57-byte frames
with the fast value, relative = absolute − reference to the display's
resolution. Entering REL — the meter's button or SET_MODE with nibble 0
= 2 — changes the mode word and the format together (`0x3111` → `0x3112`).

**Min/Max (format 0x20)**:
- Current: 5 bytes (short value)
- Max: 5 bytes + uint32 LE timestamp (seconds from start)
- Average: 5 bytes + uint32 LE timestamp
- Min: 5 bytes + uint32 LE timestamp
- Unit: 8 bytes (shared)

Hardware-confirmed 2026-09-27: 46-byte frames. Max's and min's timestamps
are the second each was set; the average's counts the seconds elapsed, and
the average is the running mean of the 2 Sa/s readings. Max, average and
min are blank (§5.2) until the first reading lands. On the temperature
dial the first frames carried stale bytes in the average and min slots,
and for about 300 ms after leaving min/max aux1 and aux2 repeated the
main value.

**Peak (format 0x40)**:
- Max: 13 bytes (full value with unit)
- Min: 13 bytes (full value with unit)

Hardware-confirmed 2026-09-27: 32-byte frames. The maximum fills first
and the minimum stays blank for up to about 700 ms after entering Peak.

### 5.4 COMP Mode Extension

When misc2 bit 4 (COMP) is set, after the bargraph unit field:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | Comparison mode: 0=INNER, 1=OUTER, 2=BELOW, 3=ABOVE |
| 1 | 1 | Result: 0=PASS, 1=FAIL |
| 2 | 1 | Precision/digits — **low nibble, unshifted** (sigrok protocol.c:112: "1 byte digits, not shifted as in other precision fields") |
| 3 | 4 | High limit (float32 LE) |
| 7 | 4 | Low limit (float32 LE, only for INNER/OUTER modes) |

Hardware-confirmed 2026-09-27: on V DC, a 42-byte frame (6 + 13 + 12 +
11) carried INNER, FAIL, digits `0x04` — the V DC 6 V range's four
decimals, read from the low nibble — high 3.0 and low 1.0, beside a
0.0033 V reading outside that window.

---

## 6. Mode Word Table -- [KNOWN]

The mode word is uint16 LE with structured nibble encoding:
- Nibble 3 (MSB): measurement function family
- Nibble 2: sub-function
- Nibble 1: variant (1=normal, 2=Hz/peak/ACDC, 3=peak, 4=LPF, etc.)
- Nibble 0 (LSB): 1=standard, 2=REL variant

79 total modes (count corrected 2026-06: both sigrok and antage define 79).
A real meter has sent 58 of them (2026-09-27 capture, issue #5): every
word with nibble 0 = 1 in §6.1's table, including every row below except
V AC REL, plus the continuity open beeper `0x5212`, Diode Alarm `0x6112`
and seven REL words — `0x3112`, `0x3122`, `0x5312`, `0x7212`, `0x7312`,
`0x8122`, `0x9122`. The 21 never seen are all REL words: `0x1112`,
`0x1142`, `0x1152`, `0x1162`, `0x2112`, `0x2142`, `0x4112`, `0x4212`,
`0x4222`, `0x4312`, `0x4322`, `0x5112`, `0x6212`, `0x7112`, `0x8112`,
`0x8212`, `0x9112`, `0x9212`, `0xA112`, `0xA122`, `0xA212`. No word
outside §6.1's table appeared. Selected examples:

| Mode | Code | Description |
|------|------|-------------|
| V AC | 0x1111 | V AC |
| V AC REL | 0x1112 | V AC relative |
| V AC Hz | 0x1121 | V AC frequency (aux1 = Hz, aux2 = period) |
| V AC Peak | 0x1131 | V AC peak |
| V AC LPF | 0x1141 | V AC low-pass filter |
| V AC dBV | 0x1151 | V AC dBV |
| V AC dBm | 0x1161 | V AC dBm |
| mV AC | 0x2111 | mV AC |
| mV AC+DC | 0x2141 | mV AC+DC coupled |
| V DC | 0x3111 | V DC |
| V DC AC+DC | 0x3121 | V DC AC+DC coupled |
| V DC Peak | 0x3131 | V DC peak |
| mV DC | 0x4111 | mV DC |
| Temp C T1(T2) | 0x4211 | Temperature C, T1 main, T2 aux |
| Temp C T2(T1) | 0x4221 | Temperature C, T2 main, T1 aux |
| Temp C T1-T2 | 0x4231 | Temperature C, differential |
| Temp F T1(T2) | 0x4311 | Temperature F, T1 main |
| Resistance | 0x5111 | Resistance |
| Continuity | 0x5211 | Continuity (short) |
| Conductance | 0x5311 | Conductance (nS) |
| Diode | 0x6111 | Diode test |
| Capacitance | 0x6211 | Capacitance |
| Frequency | 0x7111 | Frequency |
| Duty Cycle | 0x7211 | Duty cycle |
| Pulse Width | 0x7311 | Pulse width |
| uA DC | 0x8111 | uA DC |
| uA AC | 0x8211 | uA AC |
| mA DC | 0x9111 | mA DC |
| mA AC | 0x9211 | mA AC |
| A DC | 0xA111 | A DC |
| A AC | 0xA211 | A AC |

Each mode has REL variant (+1 to LSB nibble), and current/voltage
modes have Hz, Peak, and AC+DC variants.

### 6.1 Mode switching (SET_MODE) -- [VENDOR]

Traced out of the Setting dialog (`TfrmSetting`) of the vendor
application, V1.05. See `reverse-engineering-approach.md`, "Phase 3",
for how the handlers were recovered and how to reproduce this.
A real meter has since taken SET_MODE from the host within a family,
and switched REL through nibble 0 (§4.2, 2026-09-27); which variant
words it reports is in §6.

#### Composition rule

The dialog composes the word it sends from three Delphi `Tag`
properties (`0x86d700`):

```
word = ActivePage.Tag                 ; dial family, high 12 bits
     + (primary_radio.Tag   << 4)     ; nibble 1
     +  secondary_radio.Tag           ; nibble 0
```

`ActivePage` is the tab sheet for the dial position; the primary and
secondary radios live in the group boxes captioned "Primary Mode"
(`Tag = 1`) and "Secondary Mode" (`Tag = 2`).

The receive path decomposes the same word (`0x86cfc0`, called with the
mode field of every measurement packet): it makes visible **only** the
tab whose `Tag` equals `word & 0xFF00`, then checks the primary radio
whose `Tag` equals `(word & 0xF0) >> 4` and the secondary radio whose
`Tag` equals `word & 0x0F`. Send side and receive side therefore agree
on the nibble layout already described in §6 — this is a direct vendor
confirmation of it, not an inference.

Two consequences for a host implementation:

- **The PC can only move the meter inside the family the dial has
  selected.** The vendor app hides every other tab, so it never emits a
  word with a different high byte. Turning the dial stays the user's job.
- The app suppresses the send when the composed word equals the last
  word received, and sleeps 100 ms after each SET_MODE
  (`btnUpdate1Click`, `0x86ceac`: `call 0x8702b8` then `push 0x64;
  call Sleep`).

#### Dial families and primary variants

Base words are assigned in `FormCreate` (`0x86d31c`) as
`PageControl1.Pages[i].Tag`, in tab order. Captions below are the
`TRadioButton.Caption` values from the form resource, or the caption the
click handler writes at runtime where it overrides the resource.

| Family (base) | n1=1 | n1=2 | n1=3 | n1=4 | n1=5 | n1=6 | REL for n1 |
|---|---|---|---|---|---|---|---|
| V AC `0x1100` | VAC | VAC,HZ | Peak | LowPass | dBV | dBm | 1, 4, 5, 6 |
| mV AC `0x2100` | mVAC | mVAC,HZ | Peak | AC+DC | -- | -- | 1, 4 |
| V DC `0x3100` | VDC | AC+DC | Peak | -- | -- | -- | 1, 2 |
| mV DC `0x4100` | mVDC | Peak | -- | -- | -- | -- | 1 |
| Celsius `0x4200` | T1,T2 | T2,T1 | T1-T2 | T2-T1 | -- | -- | 1, 2 |
| Fahrenheit `0x4300` | T1,T2 | T2,T1 | T1-T2 | T2-T1 | -- | -- | 1, 2 |
| Ohm `0x5100` | OHM | -- | -- | -- | -- | -- | 1 |
| Beeper `0x5200` | Beeper | -- | -- | -- | -- | -- | none |
| ns `0x5300` | ns | -- | -- | -- | -- | -- | 1 |
| Diode `0x6100` | Diode | -- | -- | -- | -- | -- | none |
| Cap `0x6200` | Cap | -- | -- | -- | -- | -- | 1 |
| Hz `0x7100` | Hz | -- | -- | -- | -- | -- | 1 |
| Duty `0x7200` | % | -- | -- | -- | -- | -- | 1 |
| ms-Pulse `0x7300` | ms-Pulse | -- | -- | -- | -- | -- | 1 |
| uA DC `0x8100` | uADC | AC+DC | Peak | -- | -- | -- | 1, 2 |
| uA AC `0x8200` | uAAC | uAAC,Hz | Peak | -- | -- | -- | 1 |
| mA DC `0x9100` | mADC | AC+DC | Peak | -- | -- | -- | 1, 2 |
| mA AC `0x9200` | mAAC | mAAC,Hz | Peak | -- | -- | -- | 1 |
| A DC `0xA100` | ADC | AC+DC | Peak | -- | -- | -- | 1, 2 |
| A AC `0xA200` | AAC | AAC,Hz | Peak | -- | -- | -- | 1 |

The mode word for a cell is `base + (n1 << 4) + n0`. For example V AC
dBm plain is `0x1100 + (6 << 4) + 1 = 0x1161`, and V DC AC+DC relative
is `0x3100 + (2 << 4) + 2 = 0x3122`.

#### Secondary nibble

| n0 | Meaning |
|----|---------|
| 1 | Plain — the primary variant with no modifier. The radio's caption just repeats the primary's |
| 2 | REL, where the primary variant offers it (right-hand column above). Beeper and Diode instead use it for `Open` and `Alarm` |
| 3 | Unreachable — see below |

A third secondary radio captioned "Peak" (`Tag = 3`) exists on every
tab, but it is `Visible = False` in the form resource on eight of them
and is disabled by the primary click handlers on the rest, so **no
`n0 = 3` word is reachable from the vendor UI**. (`rbtnVAC_M6Click` does
not touch that radio: it stays disabled by whichever handler ran before
it.)

REL gating is done by the primary radio's click handler, which calls
`TControl.SetEnabled` (`0x4aeb9c`) and `SetCaption` (`0x4aecdc`) on the
three secondary radios. The "REL for n1" column above is that gating,
read out of each handler: REL is offered on the plain, AC+DC, LowPass,
dBV, dBm and `T1,T2` / `T2,T1` variants, and withheld on every Hz
variant, every Peak variant and the two differential-temperature
variants (`T1-T2`, `T2-T1`). Families with a single primary radio
(Ohm, Beeper, ns, Diode, Cap, Hz, Duty, ms-Pulse) have no such handler;
their secondary group is what the form resource declares.

The meter's own keys agree, per a reporter's map of which functions
take REL and MIN/MAX (issue #5, firmware 2.06, 2026-09-28): REL on
every variant this column allows and on Ω, nS, Cap, Hz, Duty and
Pulse, none on continuity or diode; MIN/MAX on every variant except
Peak.

#### Caveats and vendor quirks

- **mV AC+DC (`0x2141`).** The AC+DC radio on the mVAC
  tab carries `Tag = 4`, so the vendor app does emit `0x2141`, agreeing
  with the community row in §6. Two things undercut it: the radio is
  literally named `rbtnmVDC_M2` with handler `rbtnmVDC_M2Click`
  (`0x86e584`) — a copy-paste from the mVDC tab — and the receive-side
  label decoder `FUN_0085e69c` has **no `0x40` case for family `0x21`**,
  so a meter reporting `0x2141` would show a blank secondary label in
  the vendor app's own record grid. A real meter does report `0x2141`
  for its mV AC+DC function (2026-09-27 capture, set from the meter's own
  keys, with unit `mVac+dc` and the AC and DC parts in aux1/aux2);
  whether it takes the word from the host is untested.
- **Duty tab.** `rbtnDuty_M1` has no `OnClick` at all in the form
  resource, so clicking it never refreshes the cached primary nibble.
  In practice the nibble is already 1 (set from the last received word
  by `0x86cfc0`), so `0x7211` / `0x7212` still come out right.
- **`btnUpdate1Click` guard.** Before composing, the handler returns
  early if the live family is `0x1100` **and** both `rbtnVAC_M6` (dBm,
  field `+0x430`) and `rbtnVAC_F3` (Peak, field `+0x440`) are checked —
  the app refuses to ask for V AC dBm + Peak. Since `F3` is disabled
  everywhere it can be reached, this path is normally dead code. Field
  offsets read from the `TfrmSetting` published-field RTTI table.
- **Independent corroboration.** The main window enables three
  mode-specific buttons on `(word & 0xFFF0) == 0x1140` (LowPass),
  `== 0x5210` (Beeper) and `== 0x6110` (Diode), which exercises the same
  nibble split from a completely different code path.

#### Agreement with the community table (§6)

`FUN_0085e69c` builds the "Pri" / "Sec" labels of the record grid by
switching on the mode word's high byte and on `low & 0xF0`. Its cases
settle several rows that §6 previously carried on community agreement
alone:

| Word | Vendor label | Note |
|------|--------------|------|
| `0x1121` | `VAC,Hz` | Also hardware-confirmed (§6) |
| `0x1141` / `0x1151` / `0x1161` | `LowPass` / `dBV` / `dBm` | |
| `0x3121` | `AC+DC` | Not Hz |
| `0x4121` | `Peak` | Confirms mV DC Peak is `0x4121`; sigrok's alternative `0x4131` is not what the vendor uses |
| `0x4211` / `0x4221` / `0x4231` / `0x4241` | `T1,T2` / `T2,T1` / `T1-T2` / `T2-T1` | `0x43x1` identical for Fahrenheit |
| `0x8121` / `0x9121` / `0xA121` | `AC+DC` | Confirms the 2026-06 correction: DC-current n1=2 is AC+DC, not Hz |
| `0x8221` / `0x9221` / `0xA221` | `uAAC,Hz` / `mAAC,Hz` / `AAC,Hz` | AC-current n1=2 *is* Hz |
| `0x5212` / `0x6112` | Beeper `Open` / Diode `Alarm` | n0=2 here is not REL |

Families `0x51`, `0x52`, `0x53`, `0x61`, `0x62`, `0x71`, `0x72`, `0x73`
have a single label each with no n1 switch, matching their single
primary radio.

---

## 7. Range Byte -- [KNOWN]

| Value | mV | V | uA | mA | A | Ohm | Hz | Cap |
|-------|-----|------|------|------|-----|---------|---------|------|
| 0x00 | Auto | Auto | Auto | Auto | Auto | Auto | Auto | Auto |
| 0x01 | 60mV | 6V | 600uA | 60mA | -- | 600R | 60Hz | 6nF |
| 0x02 | 600mV | 60V | 6000uA | 600mA | -- | 6kR | 600Hz | 60nF |
| 0x03 | -- | 600V | -- | -- | -- | 60kR | 6kHz | 600nF |
| 0x04 | -- | 1000V | -- | -- | -- | 600kR | 60kHz | 6uF |
| 0x05 | -- | -- | -- | -- | -- | 6MR | 600kHz | 60uF |
| 0x06 | -- | -- | -- | -- | -- | 60MR | 6MHz | 600uF |
| 0x07 | -- | -- | -- | -- | -- | -- | 60MHz | 6mF |
| 0x08 | -- | -- | -- | -- | -- | -- | -- | 60mF |

Temperature: fixed range. Current A: fixed at 10A.

Hardware-confirmed 2026-09-27 (issue #5):

- The positions with no manual range — temperature, A DC and A AC,
  continuity, nS and diode — report range byte 1 with the auto bit set,
  and so did Duty and Pulse width on auto.
- A Peak variant enters with the auto bit set and clears it about 600 ms
  later, keeping the rung it entered on; the meter answers ER to
  SET_RANGE 0 there. [UNVERIFIED] One SET_RANGE in µA DC Peak had the
  manual rung reported back (issue #5, CH9329 / UT-D09, @diego351,
  2026-09-28); the mode it ran in is unconfirmed.
- V AC LPF (`0x1141`) enters on range 4 (1000 V) with the auto bit clear.
- REL and min/max clear the auto bit and keep the rung.

### 7.1 Vendor range ladders -- [VENDOR]

The Setting dialog gives each dial family one "Range" combo box, inside
the group box tagged 3. Item 0 is always `Auto`. `cbBoxRangeChange`
(`0x86cf0c`) passes the combo's `ItemIndex` and `ItemIndex == 0`
straight to the SET_RANGE wrapper (`0x8702d8`), which sends
`is_auto ? 0 : index` as a single byte; the receive path does the exact
inverse, writing the meter's range byte into the combo's `ItemIndex`
(`0x86d1c4`). **A manual range is therefore a 1-based index into the
ladder below**, and it is the same numbering the meter reports back in
the measurement packet's range field.

Combo contents. The resource stores each item as a span string
(`'0 - 600'`, `'-60 - 60'`); the table condenses those to the upper
bound, with `±` where the span is bipolar. The strings carry no unit —
the unit is the dial family's.

| Dial family | Items after `Auto` (index 1, 2, 3, ...) |
|---|---|
| V AC | 6 / 60 / 600 / 1000 |
| V DC | ±6 / ±60 / ±600 / ±1000 |
| mV AC | 60 / 600 |
| mV DC | ±60 / ±600 |
| Ohm | 600 / 6000 / 60000 / 600000 / 6000000 / 60000000 |
| Cap | 6 / 60 / 600 / 6000 / 60000 / 600000 / 6000000 / 60000000 |
| Hz | 60 / 600 / 6000 / 60000 / 600000 / 6000000 / 60000000 |
| Duty | 60 / 600 / 6000 / 60000 |
| ms-Pulse | 60 / 600 / 6000 / 60000 |
| uA DC | ±600 / ±6000 |
| uA AC | 600 / 6000 |
| mA DC | ±60 / ±600 |
| mA AC | 60 / 600 |

Cross-check against §7: the V, mV, Ohm, uA, mA, Hz and Cap ladders agree
with the community table in both length and step, index for index. (The
Cap numbers only line up if read as nF — `6000000` is §7's 6mF entry —
which is consistent with §8 listing `nF` as the wire unit.) **Duty and
ms-Pulse are new** — §7 has no column for them. The `±` spans mark the
DC families' bipolar ranges; they do not imply a separate range code.

Hardware-confirmed 2026-09-27: SET_RANGE with each index of the Cap, Hz,
µA DC, µA AC and mA DC ladders, and of the V DC ladder on its AC+DC
variant, made the meter report that index with the matching unit (`nF`
through `mF`, `Hz` through `MHz`). The Duty and ms-Pulse rungs, and what
the LCD calls them, are still untested.

**Families with no manual range.** The `Range` group box is
`Visible = False` on **A DC, A AC, Celsius, Fahrenheit, Beeper, ns and
Diode**, so the vendor app never sends SET_RANGE for them. This matches
§7's "Temperature: fixed range" and its empty A column, and adds Beeper,
ns and Diode. Those hidden combos still hold placeholder item lists
copied from another family (A DC / A AC hold the mA DC list; Celsius,
Fahrenheit hold the V AC list; Beeper, ns and Diode hold the Ohm list) —
they are dead UI, not range data.

Two combos (`Cap`, `mA AC`) ship without an `ItemIndex` property in the
resource, so they start unselected rather than on `Auto`. Cosmetic.

---

## 8. Unit Strings -- [KNOWN]

The UT181A sends unit strings as part of measurement packets (8 bytes,
null-terminated). The device determines the unit, not the host.

| Wire String | Unit | Notes |
|-------------|------|-------|
| `mVDC` | millivolt DC | |
| `VDC` | volt DC | |
| `mVAC` | millivolt AC | |
| `VAC` | volt AC | Hardware-confirmed 2026-09-02 |
| `mVac+dc` | millivolt AC+DC | |
| `Vac+dc` | volt AC+DC | |
| `uADC` | microampere DC | |
| `mADC` | milliampere DC | |
| `ADC` | ampere DC | |
| `uAAC` | microampere AC | |
| `mAAC` | milliampere AC | |
| `AAC` | ampere AC | |
| `uAac+dc` | microampere AC+DC | |
| `mAac+dc` | milliampere AC+DC | |
| `Aac+dc` | ampere AC+DC | |
| `~` | ohm | Tilde represents omega |
| `k~` | kilohm | |
| `M~` | megohm | |
| `nS` | nanosiemens | Conductance |
| `nF` | nanofarad | |
| `uF` | microfarad | |
| `mF` | millifarad | |
| `Hz` | hertz | Hardware-confirmed 2026-09-02 |
| `kHz` | kilohertz | |
| `MHz` | megahertz | |
| `%` | percent | Duty cycle |
| `ms` | millisecond | Pulse width; also the period on V AC Hz (hardware-confirmed 2026-09-02) |
| `dBV` | decibel-volt | |
| `dBm` | decibel-milliwatt | |
| `\xB0C` | degrees Celsius | 0xB0 = degree symbol (Latin-1); hardware-confirmed 2026-09-02 |
| `\xB0F` | degrees Fahrenheit | 0xB0 = degree symbol (Latin-1) |
| `V` | volt | Diode test (no coupling suffix) |

Hardware-confirmed 2026-09-27 (issue #5): every string in this table was
sent by a real meter. The unit follows the range — `k~` and `M~` on the
upper Ω rungs, `nF`/`uF`/`mF` and `Hz`/`kHz`/`MHz` along their ladders —
and `~` is also the unit of dBm's reference impedance (§5.3).

---

## 9. Timestamp Format -- [KNOWN]

Used in saved measurements (type 0x03), recording info (type 0x04),
and recording data (type 0x05). Packed into 32 bits:

```
Bits [5:0]   -> year - 2000    (range: 2000-2063)
Bits [9:6]   -> month           (1-12)
Bits [14:10] -> day             (1-31)
Bits [19:15] -> hour            (0-23)
Bits [25:20] -> minute          (0-59)
Bits [31:26] -> second          (0-59)
```

---

## 10. Recording Protocol -- [KNOWN]

### 10.1 Start Recording (Command 0x0A)

| Field | Size | Description |
|-------|------|-------------|
| Name | 11 | Null-terminated ASCII (max 10 chars) |
| Interval | 2 | uint16 LE, seconds (1-3600) |
| Duration | 4 | uint32 LE, seconds (up to 143,999 minutes) |

Maximum 20 named recordings on the device.

[VENDOR] The wrapper at `0x8705b0` builds exactly this 17-byte payload
(`push 0x11`, opcode 0x0A): it copies the name, forces a NUL at payload
offset 9, then writes the interval at offset 11 and the duration at
offset 13. The forced NUL means the vendor app caps the name at **9**
characters, not the 10 the field width allows. Whether the meter itself
accepts a 10-character name is untested.

### 10.2 Recording Info (Response Type 0x04)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 11 | Name (null-terminated) |
| 11 | 8 | Unit string |
| 19 | 2 | Interval (uint16 LE, seconds) |
| 21 | 4 | Duration (uint32 LE, seconds) |
| 25 | 4 | Sample count (uint32 LE) |
| 29 | 5 | Max value (float32 LE + precision) |
| 34 | 5 | Average value (float32 LE + precision) |
| 39 | 5 | Min value (float32 LE + precision) |
| 44 | 4 | Start timestamp (packed 32-bit) |

### 10.3 Recording Data (Response Type 0x05)

Downloaded in chunks via command 0x0D. Each response:

| Field | Size | Description |
|-------|------|-------------|
| Count | 1 | Number of samples in this packet (max 250) |
| Samples | 9 * N | Per sample: float32 LE (4) + precision (1) + timestamp (4) |

Samples are requested by offset, starting at 1; each reply carries one
chunk.

---

## 11. Implementation Considerations

### 11.1 Device Discrimination

The UT181A shares VID 0x10C4, PID 0xEA80 with UT61E+, UT8802, and
UT8803. What sets it apart on the wire:

1. **Frame length**: UT181A uses 2-byte LE length vs UT61E+ 1-byte.
   What a UT181A answers to a UT61E+ measurement request is
   [UNVERIFIED].
2. **Monitor mode**: after command 0x05 (SET_MONITOR, enable) a UT181A
   streams type 0x02 packets.

A UT171 stream has the same framing and type byte, and its payload
lengths overlap the UT181A's; detection is written up in
[docs/detection-design.md](../../detection-design.md).

### 11.2 Communication Mode

The meter requires "Communication ON" in settings before USB works.
This is a manual step on the device -- there is no USB command to
enable it. The setting resets on power cycle.

[HARDWARE] (PR #8, CH9329 / UT-D09, @alexander-magon, 2026-04-07):
Communication ON alone does not start the stream. The meter sends nothing
until the host sends SET_MONITOR 1, `AB CD 04 00 05 01 0A 00`.

The device cannot measure while charging.

### 11.3 Value Encoding

Unlike UT61E+ (ASCII display string) or UT8803 (BCD/raw bytes), the
UT181A sends IEEE 754 float32 values. The host receives both the
numeric value and its unit string, making parsing straightforward.
The precision byte indicates decimal places for display formatting.

### 11.4 Existing Rust Implementation

The [antage/cp211x_uart](https://github.com/antage/cp211x_uart) crate
provides CP2110 UART control in Rust. The
[antage/ut181a](https://github.com/antage/ut181a) crate provides a
complete UT181A protocol library.

---

## 12. Confidence Summary

Most protocol detail is **[KNOWN]** -- confirmed by three independent
implementations (antage/ut181a Rust, loblab/ut181a C++, sigrok C
driver). No item rests on inference alone. The command payloads, mode
composition and range ladders in §4.2, §6.1 and §7.1 are additionally
**[VENDOR]**: read out of `UT181A.exe` V1.05, which is where the one
outright correction lives (SET_MIN_MAX takes one byte, not four).

That is agreement between implementations and with the vendor binary,
not hardware coverage. A real meter has since confirmed (2026-09-27,
issue #5) the transport, framing, all four measurement formats and the
COMP extension, 58 of the 79 mode words, every unit string, the OK/ER
reply, and SET_MODE, SET_RANGE, SET_MIN_MAX and HOLD. The recording and
saved-measurement protocols, SET_REFERENCE and 21 REL words have never
run against one. The open checks are in [verification.md](verification.md).

| Aspect | Status | Sources |
|--------|--------|---------|
| Frame format (header, length, checksum) | [KNOWN] | 3 implementations + `UT181A.exe` `0x870408` |
| All 15 commands (0x01-0x12) | [KNOWN] | antage + sigrok + loblab |
| Command payload sizes (0x01-0x0F, 0x12) | [VENDOR] | `UT181A.exe` send wrappers |
| SET_MIN_MAX payload is 1 byte, not 4 | [VENDOR] | `UT181A.exe` `0x870588` |
| Mode word nibble layout (family / primary / secondary) | [VENDOR] | `UT181A.exe` `0x86d700` + `0x86cfc0` |
| Per-family mode variants and captions (§6.1) | [VENDOR] | `TfrmSetting` resource + click handlers |
| All 79 mode words | [KNOWN] | antage + sigrok; 58 from a real meter (2026-09-27) |
| Range bytes 0x00-0x08 | [KNOWN] | 3 implementations |
| Range ladders per family, 1-based index (§7.1) | [VENDOR] | `TfrmSetting` range combos + real meter, six ladders (2026-09-27) |
| Measurement packet (all 4 variants) | [KNOWN] | antage + sigrok + real meter (2026-09-27) |
| COMP mode extension | [KNOWN] | sigrok driver + real meter (2026-09-27) |
| Unit strings | [KNOWN] | antage + sigrok + real meter (2026-09-27) |
| Timestamp format | [KNOWN] | 3 implementations |
| Recording protocol | [KNOWN] | 3 implementations |
| Transport (9600/8N1 CP2110) | [KNOWN] | 3 implementations |
| Header bytes = 0xAB 0xCD (same as UT61E+) | [KNOWN] | 3 codebases verified |
| Device specs (60K counts, modes, ranges) | [KNOWN] | User manual |
