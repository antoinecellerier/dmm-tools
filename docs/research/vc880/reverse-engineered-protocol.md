# VC-880 / VC650BT: Reverse-Engineered Protocol Specification

Based on ILSpy decompilation of Voltsoft `DMSShare.dll`, Conrad's "VC880
Protocol Rev 2.4" and the VC880 and VC650BT user manuals. See
`reverse-engineering-approach.md` for methodology.

Confidence levels:
- **[KNOWN]** / **[MANUAL]** -- from the user manual
- **[VENDOR]** -- from Voltsoft decompilation
- **[VENDOR-DOC]** -- from "VC880 Protocol Rev 2.4" (Conrad, filed under
  both the VC-880 and the VC650BT), read from rendered pages; "Rev 2.4 p.N"
  is its PDF page
- **[INFERRED]** -- logically deduced
- **[UNVERIFIED]** -- requires real device testing
- **[COMMUNITY]** -- community code, §8 only

---

## 1. Transport Layer -- [VENDOR]

Silicon Labs CP2110 HID-to-UART bridge (VID `0x10C4`, PID `0xEA80`).

**Line settings** [VENDOR-DOC]: "Baud rate is 9600 bits / No parity /
1 start bit / 8 data bits (LSB first) / 1 stop bit" (Rev 2.4 p.1), i.e. 8N1.

**UART configuration** (from `VC880Obj.OpenDevice()`, line 21228):
```
HidUart_SetUartConfig(handle, 9600, 3, 2, 0, 0)
  baudRate = 9600
  dataBits = 3  (SiLabs HID_UART_EIGHT_DATA_BITS)
  parity   = 2  (even in the config report's numbering, see below)
  stopBits = 0  (SiLabs HID_UART_SHORT_STOP_BIT)
  flowCtrl = 0  (SiLabs HID_UART_NO_FLOW_CONTROL)
```

**Note**: The SiLabs HID UART API uses different enum values than the
Win32 serial API. `dataBits=3` maps to 8 data bits in the SiLabs
`HID_UART_DATA_BITS` enum. [VENDOR, cross-check with SLABHIDtoUART.h]

**Parity**: Voltsoft's `parity = 2` contradicts Rev 2.4's "No parity". The
UART config report reads 2 as even
([ut171 spec §2.3](../ut171/reverse-engineered-protocol.md#23-uart-config-report-0x50-layout----vendor),
0=none, 1=odd, 2=even); the DLL API's own numbering: none found in the
CP2110 DLL's decompile, and we have no copy of its header, 2026-11-07, so
why Voltsoft passes 2 is unexplained. Its VC-890 class passes 0
(line 3755).

**Activation**: User must press the PC button on the meter to enable USB
communication. There is no software command for this — it is meter-side
only. [KNOWN from manual, INFERRED from absence of activation command]

---

## 2. Frame Format -- [VENDOR]

Identical to the UT61E+ AB CD framing — reuses `extract_frame_abcd_be16()`.

### Outbound (host → meter)

```
+------+------+------+------+------+------+------+------+
| 0xAB | 0xCD | len  | cmd  | data | ...  | chk  | chk  |
|      |      |      |      |  [0] |      |  hi  |  lo  |
+------+------+------+------+------+------+------+------+
```

| Byte | Field | Description |
|------|-------|-------------|
| 0 | Header high | `0xAB` (171) |
| 1 | Header low | `0xCD` (205) |
| 2 | Length | `data.length + 3` (includes cmd byte + 2 checksum bytes) |
| 3 | Command | Command byte (see §5) |
| 4..N | Data | Optional payload (most commands have no data) |
| N+1 | Checksum high | Big-endian high byte of sum |
| N+2 | Checksum low | Big-endian low byte of sum |

**Checksum**: 16-bit big-endian sum of all preceding bytes (header +
length + command + data). [VENDOR: `WriteCommand()` line 21243]

**Simple command (no data)**: 6 bytes total — `[0xAB, 0xCD, 0x03, cmd, chk_hi, chk_lo]`.

### Inbound (meter → host)

Same frame format. The message type is at byte 3.

**Frame recovery**: Scan buffer for `0xAB 0xCD` header, skip non-matching
bytes. Length byte at offset 2 determines total frame size.
[VENDOR: `GetOneMessage()` line 21555]

---

## 3. Message Types -- [VENDOR]

From `VC880Obj` fields (lines 21049-21059); Rev 2.4 pp.2-7 lists the same
six types with these layouts [VENDOR-DOC]:

| Type | Value | Frame Size | Description |
|------|-------|------------|-------------|
| DeviceID | 0x00 | 26 bytes | Device identification string (20 ASCII bytes) |
| LiveData | 0x01 | 39 bytes | Real-time measurement (continuous stream) |
| CompData | 0x02 | 23 bytes | Comparator mode data |
| NOCOMPDataTransfer | 0x03 | 15 bytes | Non-COMP log data transfer |
| COMPDataTransfer | 0x04 | 30 bytes | COMP log data transfer |
| Result | 0xFF | 7 bytes | Command result/acknowledgement (1 byte) |

Frame sizes include the 2-byte header + length byte + type byte + payload + 2-byte checksum.
The length byte counts "from Msg Type to the last byte of Check Sum" (Rev 2.4 p.2).

**Checksum validation per message type** (from `CheckSum()`, line 21756):
- LiveData (0x01): sum bytes 0..36, check against bytes 37-38. Total = 39 bytes.
- CompData (0x02): sum bytes 0..20, check against bytes 21-22. Total = 23 bytes.
- NOCOMPDataTransfer (0x03): sum bytes 0..12, check against bytes 13-14. Total = 15 bytes.
- COMPDataTransfer (0x04): sum bytes 0..27, check against bytes 28-29. Total = 30 bytes.

---

## 4. Live Data Payload (type 0x01, 39 bytes) -- [VENDOR]

This is the primary measurement frame, streamed continuously. (The
"~2-3 Hz" rate is a manual claim, [UNVERIFIED] — nothing in the
decompile establishes a rate.)

```
Offset  Size  Field           Description
0       2     Header          0xAB 0xCD
2       1     Length          0x24 (36 = 33 data + 3)
3       1     Type            0x01 (LiveData)
4       1     Function        Function code (0x00-0x12), see §4.1
5       1     Range           Range index (0x30-based ASCII), see §4.2
6-12    7     Display 1       Main display (7 ASCII bytes)
13-19   7     Display 2       Small upper-right display (7 ASCII bytes)
20-26   7     Display 3       Small upper-left display (7 ASCII bytes)
27-29   3     Display 4       "Analog Graphic Bar" (3 ASCII bytes)
30      1     Status 0        COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
31      1     Status 1        Rel(0), Avg(1), Min(2), Max(3)
32      1     Status 2        Hold(0), Manual(1), OL1(2), OL2(3)
33      1     Status 3        AutoPower(0), Warning(1), Light(2), LowBatt(3)
34      1     Status 4        OuterSel(0), Pass(1), Comp(2), MisplugWarn(3)
35      1     Status 5        Mem(0), BarPol(1), Clr(2), Shift(3)
36      1     Status 6        DoubleDisp(0), Setup(1), BarOL(2), BarDispEn(3), PassBeep(4), NgBeep(5)
37-38   2     Checksum        BE16 sum of bytes 0-36
```

**Fields** [VENDOR-DOC]: Rev 2.4 p.3 gives the same offsets and names the
four displays Display 1-4; its LCD figure (p.1) places them as above, and
p.4 calls Display 4 the "Analog Graphic Bar". All four are ASCII ("Please
decode this value at ASIC-II string"). What Displays 2 and 3 show in each
mode, and the bar's text, are [UNVERIFIED].

**Display values**: 7 bytes each, parsed as ASCII strings via
`Encoding.ASCII.GetString()`. Right-justified with leading spaces.
"OL" for overload; the manual also shows `.OL` on a reversed diode (§8f)
and `OL.` on continuity (§8g) and temperature (§8i) [MANUAL], [UNVERIFIED]
whether the point reaches the wire. What an all-dashes field means: none
found in Rev 2.4, the manual or Voltsoft, 2026-11-07 (§8 has a community
reading). The vendor displays the raw
string either way; [UNVERIFIED] when each occurs on the wire.
[VENDOR: `SetReadingValue()` line 16726]

### 4.1 Function Codes (byte 4) -- [VENDOR]

From `SetDeviceMode_And_Unit_And_Range()` switch statement (line 16335).
Rev 2.4 p.3 lists the same 19 codes [VENDOR-DOC], naming 0x08
"Short-Circuit Test" and 0x12 "Low-Pass Filter".

| Code | Function | Unit(s) | Manual Section |
|------|----------|---------|----------------|
| 0x00 | DC Voltage | V | §8b, page 62 |
| 0x01 | AC+DC Voltage | V | §8b, page 64 |
| 0x02 | DC Millivolt | mV | §8b, page 62 |
| 0x03 | Frequency | Hz, kHz, MHz | §8d, page 64 |
| 0x04 | Duty Cycle | % | §8d, page 65 |
| 0x05 | AC Voltage | V | §8b, page 63 |
| 0x06 | Impedance (Resistance) | Ω, kΩ, MΩ | §8e, page 64 |
| 0x07 | Diode | V | §8f, page 65 |
| 0x08 | Continuity | Ω | §8g, page 65 |
| 0x09 | Capacitance | nF, µF, mF | §8h, page 64 |
| 0x0A | Temperature °C | °C | §8i, page 65 |
| 0x0B | Temperature °F | °F | §8i, page 65 |
| 0x0C | DC µA | µA | §8c, page 63 |
| 0x0D | AC µA | µA | §8c, page 63 |
| 0x0E | DC mA | mA | §8c, page 63 |
| 0x0F | AC mA | mA | §8c, page 63 |
| 0x10 | DC A | A | §8c, page 63 |
| 0x11 | AC A | A | §8c, page 63 |
| 0x12 | ACV Low-Pass | V | §8j, page 50; no row in the technical data, pages 62-65 |

### 4.2 Range Tables (byte 5) -- [VENDOR] + [KNOWN]

Range byte is ASCII-encoded: `0x30` = range index 0, `0x31` = index 1, etc.

Tables derived from the vendor switch statement, cross-referenced against
manual spec tables. Rev 2.4's range table (pp.3-4) [VENDOR-DOC] has the
same rows for every function below except AC+DC V and low-pass, which it
has no column for; it spells capacitance index 2 "4000 nF" (see
Capacitance).

**Voltage (functions 0x00 DCV, 0x01 AC+DC V, 0x05 ACV)** — NOT 0x12:
no vendor source gives the range byte in ACV LPF (searched 2026-11-06; §8
has a community reading). Rev 2.4 has no low-pass column,
the manual no low-pass row (pages 62-65), and the vendor never reads the
byte there, fixing the range at 1000 (`case 18`,
DMSShare_decompiled.cs:16671-16674). What the meter sends there is
[UNVERIFIED] ([verification list](verification.md#frames-and-display)):

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 4V | 0.0001V | ✓ pages 62-64 |
| 1 (0x31) | 40V | 0.001V | ✓ |
| 2 (0x32) | 400V | 0.01V | ✓ |
| 3 (0x33) | 1000V | 0.1V | ✓ |

Note: the manual's DC voltage table (page 62) starts at 400mV, but its
procedure puts that range on the mV position: "For lower voltages up to
400 mV, select 'mV'" (page 44) [MANUAL]. When function=0x00 and range=0,
the vendor maps to measurement 5 (DCV, V, 4V range); 400mV is
function=0x02 (DCV mV) with range=0 [VENDOR]. Rev 2.4 likewise has 400mV
only in its DCmV column.

**DC Millivolt (function 0x02)**:

| Index | Range | Resolution |
|-------|-------|------------|
| 0 (0x30) | 400mV | 0.01mV |

**Impedance/Resistance (function 0x06)**:

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 400Ω | 0.01Ω | ✓ page 64 |
| 1 (0x31) | 4kΩ | 0.1Ω | ✓ |
| 2 (0x32) | 40kΩ | 10Ω | ✓ |
| 3 (0x33) | 400kΩ | 100Ω | ✓ |
| 4 (0x34) | 4MΩ | 1kΩ | ✓ |
| 5 (0x35) | 40MΩ | 10kΩ | ✓ |

**Capacitance (function 0x09)**:

| Index | Range | Unit | Resolution | Manual |
|-------|-------|------|------------|--------|
| 0 (0x30) | 40nF | nF | 1pF | ✓ page 64 |
| 1 (0x31) | 400nF | nF | 10pF | ✓ |
| 2 (0x32) | 4µF | µF | 100pF | ✓ |
| 3 (0x33) | 40µF | µF | 1nF | ✓ |
| 4 (0x34) | 400µF | µF | 10nF | ✓ |
| 5 (0x35) | 4000µF | µF | 100nF | ✓ |
| 6 (0x36) | 40mF | mF | 1µF | ✓ |

Index 2 is "4 µF" in the manual (page 64) and the vendor software
(DMSShare_decompiled.cs:16468-16479), "4000 nF" in Rev 2.4 (p.4): the same
full scale. Which digits Display 1 carries on this range, `4.0000` or
`4000.0`, no source shows [UNVERIFIED]; a decoder that takes µF from the
range byte would read `4000.0`-style digits 1000× high.

**Frequency (function 0x03)**:

The per-decade rows are Rev 2.4's (pp.3-4) [VENDOR-DOC] and the vendor
software's. The manual (page 64) has two rows: "10 Hz - 40 MHz" and
"400 MHz", the latter with accuracy not specified.

| Index | Range | Unit |
|-------|-------|------|
| 0 (0x30) | 40Hz | Hz |
| 1 (0x31) | 400Hz | Hz |
| 2 (0x32) | 4kHz | kHz |
| 3 (0x33) | 40kHz | kHz |
| 4 (0x34) | 400kHz | kHz |
| 5 (0x35) | 4MHz | MHz |
| 6 (0x36) | 40MHz | MHz |
| 7 (0x37) | 400MHz | MHz |

**DC/AC µA (functions 0x0C, 0x0D)**:

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 400µA | 0.01µA | ✓ page 63 |
| 1 (0x31) | 4000µA | 0.1µA | ✓ |

**DC/AC mA (functions 0x0E, 0x0F)**:

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 40mA | 0.001mA | ✓ |
| 1 (0x31) | 400mA | 0.01mA | ✓ |

**DC/AC A (functions 0x10, 0x11)**:

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 10A | 0.001A | ✓ |

**Single-range functions (no range byte variation)**:
- Duty Cycle (0x04): range = 100 [VENDOR]
- Diode (0x07): range = 4V [VENDOR]
- Continuity (0x08): range = 400Ω [VENDOR]
- Temperature °C (0x0A): range = 1000 [VENDOR]
- Temperature °F (0x0B): range = 1832 [VENDOR]
- ACV Low-Pass (0x12): range = 1000 [VENDOR], a constant the vendor sets
  without reading the byte; whether the meter auto-ranges here is in no
  source (see Voltage above)

**VC650BT.** Rev 2.4 is filed under its item (124411) too, and Voltsoft
runs it through the VC-880 class with no model test [VENDOR]. Its own
manual and datasheet disagree with the tables above [MANUAL]: DC, AC and
AC+DC voltage top out at 600 V (manual printed pages 68-70, datasheet
pp.2-3), low-pass has one row, 600 V (printed page 73), and capacitance
stops at 4000 µF with no 40 mF row (printed page 72, datasheet p.4),
though its text mentions measuring "at mF range" (printed page 57). What a
VC650BT sends at voltage index 3 (0x33), and whether it ever sends
capacitance 0x36, are [UNVERIFIED].

### 4.3 Status Flag Bytes -- [VENDOR]

From `SetStatus()` (line 16782). Each flag is extracted as
`(byte & (1 << position)) != 0`.

Rev 2.4 pp.4-6 [VENDOR-DOC] gives the same bit positions, and the bits
above them: bits 7-4 of bytes 30-35 are `0 0 1 1`, so each of those bytes
is 0x30 plus its flags, like the range byte; byte 36's bits 7-6 are `0 0`.
Descriptions quoted below are Rev 2.4's.

**Byte 30 (Status 0)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | COMP_Max | Comparator max flag (Rev 2.4: `COMP_Max_flag`, no description) |
| 1 | COMP_Min | Comparator min flag (Rev 2.4: `COMP_Min_flag`, no description) |
| 2 | Sign1 | "the sign bit of the DISP1", 1 = negative |
| 3 | Sign2 | "the sign bit of the DISP2", 1 = negative |

**Byte 31 (Status 1)**:

Rev 2.4 says that with MAX, MIN or AVG on "the current transmitted data at
DISP2" is the maximum, minimum or average, and with REL "the relative
value". The manual puts them the other way round on the LCD: the held
value "is shown continually on the main display", the current value "in
the small upper display" (page 52). Which display carries which on the wire is
[UNVERIFIED].

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | Rel | Relative/delta mode active |
| 1 | Avg | Average mode active |
| 2 | Min | Minimum hold active |
| 3 | Max | Maximum hold active |

**Byte 32 (Status 2)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | Hold | HOLD mode active |
| 1 | Manual | Manual range (1=manual, 0=auto) |
| 2 | OL1 | Primary display overload |
| 3 | OL2 | Secondary display overload |

**Byte 33 (Status 3)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | AutoPower | Auto power off enabled |
| 1 | Warning | High voltage warning (⚠ symbol) |
| 2 | Light | Backlight on |
| 3 | LowBatt | Low battery indicator |

**Byte 34 (Status 4)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | OuterSel | Comp mode: outer selection (Rev 2.4: `Inner_flag`, "0: Inner 1: Outer") |
| 1 | Pass | Comp mode: pass result ("0: No Good 1: Pass") |
| 2 | Comp | Comparator mode active |
| 3 | MisplugWarn | Misplug warning (leads in wrong jacks) |

**Byte 35 (Status 5)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | Mem | "the continuous memory function is on or not" |
| 1 | BarPol | Rev 2.4: `Bar_pol_flag`, "the overload status of the Simulate strip number" (the bar); its name says polarity, and no source says which [UNVERIFIED] |
| 2 | Clr | "the status of the clear memory function" |
| 3 | Shift | Shift mode active |

**Byte 36 (Status 6)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | DoubleDisp | "the DISP2 is being used at the current test mode" |
| 1 | Setup | "the setup button is pressed or not" |
| 2 | BarOL | Rev 2.4: `Bar_ol_flag`, "No use at the current revision." |
| 3 | BarDispEn | "the Simulate strip number is being used at the current test mode" (the bar) |
| 4 | PassBeep | Pass beep enabled (comp mode) (Rev 2.4: `pass_beep_enable`) |
| 5 | NgBeep | NG (fail) beep enabled (comp mode) (Rev 2.4: `ng_beep_enable`) |

### 4.4 Rotary positions and SHIFT/SETUP sub-functions -- [MANUAL]

The dial picks a *position*; within a position the SHIFT/SETUP button (3)
steps through the functions that position offers. The host cannot turn the
dial, so this table is the boundary of what remote mode selection can reach.

Sources: the "Drehschalter (4)" figure on printed page 10 of the VC880 user
manual, whose layout gives each position its symbols; §3 of the same manual
("SHIFT/SETUP-Taste zur Funktionsumschaltung (rote Symbole)"), which says the
red symbols beside a position are what SHIFT/SETUP reaches; and the §8
measurement procedures, which say the same in words. Function codes are from
§4.1.

| Position | Functions (primary first) | Codes | Manual |
|----------|---------------------------|-------|--------|
| mV⎓ Hz % | DC mV, Frequency, Duty % | 0x02, 0x03, 0x04 | §8d: "Drücken Sie die „SHIFT/SETUP“-Taste bis im Display „Hz“ erscheint … erneut … bis „%“ im Display erscheint" |
| V⎓ (red AC+DC) | DC V, AC+DC V, (DC mV) | 0x00, 0x01, (0x02) | figure; §8b |
| V~ | AC V | 0x05 | figure (no red symbol on this position) |
| Lo | ACV low-pass | 0x12 | §8j: its own dial symbol, no SHIFT/SETUP |
| Ω (red diode, continuity) | Ω, Diode, Continuity | 0x06, 0x07, 0x08 | §8g: "Drücken Sie die Taste „SHIFT/SETUP“ um die Messfunktion umzuschalten … Eine erneute Betätigung schaltet in die erste Messfunktion usw." |
| ⊣⊢ | Capacitance | 0x09 | §8h |
| °C°F | °C, °F | 0x0A, 0x0B | §8i: "Drücken Sie die Taste „SHIFT/SETUP“ um die Messfunktion auf eine Anzeige in °F umzuschalten. Eine erneute Betätigung schaltet in die erste Messfunktion usw." |
| µA≂ | DC µA, AC µA | 0x0C, 0x0D | §8c |
| mA≂ | DC mA, AC mA | 0x0E, 0x0F | §8c |
| A≂ | DC A, AC A | 0x10, 0x11 | §8c: "Drücken Sie die Taste „SHIFT/SETUP“ (3) um in den AC-Messbereich umzuschalten … Eine erneute Betätigung schaltet wieder zurück usw." |

**Order is unverified.** Every source says *which* symbol a position offers
("bis … erscheint" — until it appears), never in which order the presses walk
them. The order above is the manual's order of description, not a claim about
the meter.

**0x02 may sit on two positions.** That the V⎓ position auto-ranges down to
400 mV and reports it as function 0x02 rather than 0x00: none found in
Rev 2.4, the manual or Voltsoft (second read, 2026-11-07) [UNVERIFIED]: the
vendor only maps 0x02 to mV, and the manual puts voltages up to 400 mV on
the mV position (§4.2). If it does, a reading of 0x02 alone does not say
which of the two positions the dial is on. The two positions share no other
function, so neither is a safe guess for the other.

**The manual's §8b text contradicts its own figure.** The AC-voltage procedure
reads "wählen den Messbereich „V“. Drücken Sie die Taste „SHIFT/SETUP“ (3) um
in den AC-Messbereich umzuschalten" (the range symbol after „V“ renders blank
in this PDF and is absent from its text layer), which describes a meter where
AC V is a SHIFT/SETUP sub-function of a voltage position. The figure instead
gives V~ a position of its own and puts AC+DC on V⎓. One guess at the mismatch
— not a finding — is that the text is written for the series rather than this
model: §7 introduces its symbol list as „alle möglichen Symbole und Angaben der
Serie VC800“. We follow the figure. A meter would settle it: see the
[verification list](verification.md#dial-and-modes).

**The VC650BT's dial differs** [MANUAL]. Its figure (manual printed page 3)
and position list (printed page 49) give: V~ (low-pass), V⎓ (AC+DC), mV⎓
(°C°F), Ω (diode, continuity), ⊣⊢, Hz %, µA≂, mA≂, A≂. Low-pass is a
SHIFT function of V~ and °C/°F of mV (printed page 58), Hz and % have a
position of their own (printed page 55), and there is no Lo or °C°F
position. The table above is the VC-880's.

**How the implementation uses this.** `crates/dmm-lib/src/protocol/vc8x0/vc880.rs`
holds the table as `DIAL`, one entry per position, each a single SHIFT/SETUP
ring (the VC-880 has no Hz/% button). It is membership only: the shared driver
in `protocol/cycle.rs` presses `Select` (0x4C) and reads the function code back
until the target appears, which works whatever the real press order is. Nothing
in the table is hardware-confirmed.

---

## 5. Commands (host → meter) -- [VENDOR]

From `VC880Obj` static fields (lines 21063-21125). Rev 2.4 p.8 lists the
same codes and no others [VENDOR-DOC]; pp.8-9 give the data layouts of
0x51, 0x52 and 0x01 and the comp-setting sequence (0x50, then the values,
then 0x55).

| Byte | Name | Data | Description |
|------|------|------|-------------|
| 0x00 | GetDeviceID | none | Request device identification string |
| 0x41 | ContinueLog | none | Start continuous logging |
| 0x42 | LoadLogCompData | none | Download comparator log data |
| 0x43 | ExitMaxMinAvg | none | Exit MAX/MIN/AVG mode |
| 0x44 | LoadLogNoCompData | none | Download non-comp log data |
| 0x45 | CLR_NoComp | none | Clear non-comp log data |
| 0x46 | ManualRange | none | Switch to manual range |
| 0x47 | AutoRange | none | Switch to auto range |
| 0x48 | Rel | none | Toggle relative/delta mode |
| 0x49 | MaxMinAvg | none | Toggle MAX/MIN/AVG mode |
| 0x4A | Hold | none | Toggle HOLD mode |
| 0x4B | Light | none | Toggle backlight |
| 0x4C | Select | none | SELECT button (cycle sub-function) |
| 0x4D | COMP | none | Toggle comparator mode |
| 0x4E | SingleLog | none | Log single measurement |
| 0x4F | CLR_Comp | none | Clear comp log data |
| 0x50 | SetCompEnter | none | Enter comp settings |
| 0x51 | SetCompHighValue | 7 bytes | Set comp high limit (ASCII value) |
| 0x52 | SetCompLowValue | 7 bytes | Set comp low limit (ASCII value) |
| 0x53 | SetCompInner | none | Set comp mode: inner |
| 0x54 | SetCompOuter | none | Set comp mode: outer |
| 0x55 | SetCompEsc | none | Exit comp settings |
| 0x56 | LoadNoCompEsc | none | Abort non-comp data download |
| 0x57 | LoadCompEsc | none | Abort comp data download |
| 0x5A | USBOff | none | Disconnect USB |
| 0x5B | PassBeepEnable | none | Toggle pass beep |
| 0x5C | NGBeepEnable | none | Toggle NG beep |
| 0x01 | SetCompModeAll | 15 bytes | Set comp high+low+mode in one command |
| 0x02 | CompData | none | Request comp data |
| 0xFF | Result | 1 byte | Send result/acknowledgement (0=OK, 1=resend, 2=error) |

---

## 6. Communication Model -- [VENDOR]

**Streaming**: After `OpenDevice()` configures the CP2110 UART and starts
the read thread, the meter streams LiveData (type 0x01) frames
continuously without any trigger command. The host simply reads from
the HID endpoint.

**No trigger needed**: the VC-880 streams immediately after USB connection
+ PC button press. [VENDOR + INFERRED] Rev 2.4 says nothing on when the
meter sends Live Data.

**Read loop**: `ContinuouslyReadDataLoop()` calls `ReadData()` in a
loop. `ReadData()` reads up to 50 bytes, appends to a buffer, then
calls `ProcessMessages()` which extracts and dispatches complete frames.

**Command flow**: Simple commands are sent as 6-byte frames. The meter
replies with a Result frame (type 0xFF) containing a 1-byte status:
0=success, 1=error-resend, 2=error-do-nothing (Rev 2.4 p.7: "Successful",
"Error-Resend-Previous-Message", "Error-Do-Nothing" [VENDOR-DOC]).

---

## 7. Implementation Notes

### Framing reuse

The VC-880 uses **identical framing** to the UT61E+:
- Same header (`0xAB 0xCD`)
- Same length encoding (payload + 3)
- Same checksum (BE16 sum of all preceding bytes)

Live data is type 0x01; the other types of §3 use the same framing.

### Key differences from UT61E+

| Aspect | UT61E+ | VC-880 |
|--------|--------|--------|
| Communication model | Polled (request → response) | Streaming (continuous) |
| Trigger | A request per reading | None (streams automatically) |
| Display encoding | 7 raw bytes (sometimes non-ASCII) | 7 ASCII bytes |
| Range encoding | Raw byte with mode-specific meaning | ASCII '0'-'7' (0x30-based) |
| Status flags | 2 flag bytes (14 bits) | 7 flag bytes (28 named bits) |
| Commands | 5 (hold, rel, select, etc.) | 28 (including comp mode, logging) |

### Key differences from UT8803

| Aspect | UT8803 | VC-880 |
|--------|--------|--------|
| Frame format | Fixed 21 bytes, custom checksum | Variable length, AB CD + BE16 checksum |
| Display encoding | 5 raw bytes | 7 ASCII bytes |
| Function codes | 23 modes (0x00-0x16) | 19 modes (0x00-0x12) |
| Overload | Flag bit in status word | OL1/OL2 flags + "OL" in display string |

---

## 8. Cross-reference with community sources [COMMUNITY]

pylablib's VC880 class (`pylablib/devices/Voltcraft/multimeter.py`), read
after §1-7 were written from vendor sources; the wider comparison is the
[approach doc's cross-reference](reverse-engineering-approach.md#cross-reference-against-pylablib-phase-3).
None of it rests on a capture we have seen.

- **All-dashes display (§4):** pylablib reads it as "no reading" and
  returns None, distinct from overload (corrected 2026-06).
- **ACV LPF range byte (§4.2):** pylablib indexes the voltage table for
  function 0x12, where the vendor fixes 1000 without reading the byte
  (corrected 2026-06).
- **Capacitance index 2 (§4.2):** pylablib's capacitance list gives
  `4E-6` (farads) at 0x32, the same full scale as Rev 2.4's "4000 nF" and
  the manual's 4 µF; it does not say which digits the meter sends (read
  2026-11-06, main branch). Its class docstring names the VC650BT too.
- **Displays 2-4 (§4):** pylablib names them the upper-right
  MIN/MAX/AVG/REL value, the upper-left memory value and the bottom bar
  graph.
