# VC-890: Reverse-Engineered Protocol Specification

Based on ILSpy decompilation of Voltsoft `DMSShare.dll` — same source
as the VC-880 RE. The VC-890 uses classes `VC890Obj` and `VC890Reading`
in the same DLL.

See `docs/research/vc880/reverse-engineering-approach.md` for methodology.

Confidence levels: **[VENDOR]** = from Voltsoft decompilation,
**[MANUAL]** = from the VC-890 user manual, **[VENDOR-DOC]** = from
Conrad's VC890 Protocol Rev 1.3 (2013-1-4, `references/vc890/protocol/`).

---

## Key Differences from VC-880

| Aspect | VC-880 | VC-890 |
|--------|--------|--------|
| Counts | 40,000 | 60,000 |
| Display | LCD | OLED |
| Chipset | ES51966A + MSP430 | ES51997P + EFM32 |
| Communication | Streaming (continuous) | Polled (request/response) |
| Measurement command | None (auto-streams) | 0x5E (same as UT61E+) |
| Live data frame size | 39 bytes | 66 bytes |
| Display fields | 4 (7+7+7+3 bytes) | 7 (7+8+10+8+8+3+4 bytes) |
| Status bytes | 7 bytes at msg[30..36] | 8 bytes at msg[56..63] |
| Range values | 4/40/400 | 6/60/600 |
| Function code 0x00 | DCV | ACV (remapped!) |
| Ack protocol | None | 0xFF+\[0x00\] after responses |
| Battery indicator | Flag bit (LowBatt) | Gear 0-3 (`0x30`-`0x33`) |

---

## Frame Format -- [VENDOR]

Host frames use the same AB CD + BE16 framing as VC-880 and UT61E+.
Rev 1.3 p.2 gives the inbound sum [VENDOR-DOC]: 16 bits, after the payload,
"the sum of the byte values from the first byte of Header to the last byte
of Payload", with no byte order. Voltsoft checks no checksum on a frame the
meter sends: big-endian is inferred from its host-side builder,
[UNVERIFIED]. The length byte counts the bytes from the type byte (inbound,
p.2) or the command byte (outbound, p.9) through the checksum.

**Frame types** [VENDOR-DOC] (Rev 1.3 p.2 says "there are 9 types of
messages" and tabulates these 7; the other two are named nowhere):

| Type | Name | Layout |
|------|------|--------|
| 0x00 | Device ID | p.2 |
| 0x01 | Live Data | pp.3-7 |
| 0x02 | Comp Data | p.7 |
| 0x03 | NOCOMP Data Transfer | p.7 |
| 0x04 | COMP Data Transfer | pp.7-8 |
| 0x05 | SETUP Data Transfer | pp.8-9 |
| 0xFF | Result | p.9 |

Voltsoft asks for 0x05 with command 0x03 and waits for a frame of that
type [VENDOR].

## Communication Model -- [VENDOR]

**Polled**: Host sends measurement request (0x5E), meter responds with
one live data frame. From `VC890Obj.GetReading()` (line 4027):
```csharp
WriteCommand(94);  // 0x5E
byte[] array = ReceiveOneMessage(1);  // type 0x01
```
Rev 1.3 p.1 [VENDOR-DOC]: "VC890 will transfer one set of data only if PC
sends out 0x5e to command the data from it, if not, no data from VC890 will
be transmitted."

**Command confirmation**: `SendCommand(cmd, receive)` (line 4039) writes
the command and waits for a frame of type `receive`, retrying up to 5
times. Every call of it, in `DMSShare.dll` (lines 4089, 4111, 4168, 4247)
and in Voltsoft's `DeviceServer.exe` command handlers, passes `0xFF`: a
command waits for a Result frame. The overload `SendCommand(cmd)` (line
4056), which waits for a frame of the command's own type, has no caller in
either, and no source documents such a frame (second read, 2026-11-07,
with a check of every call in the `DeviceServer.exe` decompile; the first
read left echoes open). `ReceiveOneMessage` skips
frames of any other type, which leaves open whether anything comes ahead of
the 0x5E poll's live frame.

**Ack protocol**: The Voltsoft vendor software wraps every VC-890
request/response pair in an ack sequence. `AckMessage(clear: true)` at
`DMSShare_decompiled.cs:3861` sends `command = 0xFF` with `data = [0x00]`
three times with a 100ms `Thread.Sleep` between writes and a
`FlushBuffer()` afterwards. Using `WriteCommand(byte, byte[], bool)` at
line 3805 to build the frame yields the exact wire bytes

```
AB CD 04 FF 00 02 7B
```

(`0x04` = command(1) + data(1) + checksum(2), and checksum = AB + CD + 04
+ FF + 00 = 0x027B, BE).

The sequence is invoked in two places per measurement cycle:

1. **Pre-clear** — `WriteCommand(byte command, bool ack = true)`
   (line 3773) calls `AckMessage(clear: true)` before writing the
   command frame. `GetReading()` / `SendCommand()` take this default
   path, so every outgoing command is preceded by the 3× ack burst.
   `GetDeviceID()` (line 3895) takes it too: it calls the one-argument
   `WriteCommand(0)` (line 3800), which forwards with `ack: true`.
2. **Post-confirm** — `ReceiveOneMessage(byte messageType, bool ack =
   true)` (line 3977) calls `AckMessage(ack)` right after a valid
   response frame is reassembled (line 4009). Default `ack = true` →
   3× ack burst after every received frame.

A similar single-shot `AckMessage(clear: false)` (one write, no sleeps)
is used on an HID read error path at line 3941. Whether the meter
requires the ack or merely receives it is [UNVERIFIED]
([verification](verification.md#exchanges)).

**[VENDOR-DOC]** The protocol document defines `0xFF` Result both as a
DMM→PC message (§3.7) and as a PC→DMM command (§4.6), laid out
`AB CD <len> FF <result> <checksum>`, carrying "the data transmission
result of the message that required handshaking": `0x00` Successful,
`0x01` Error-Resend-Previous-Message, `0x02` Error-Do-Nothing. Voltsoft's
ack frame above is that command with `0x00` Successful. The document does
not say which messages require handshaking; the fourth column of its
command table is whether a command carries data.

**[VENDOR-DOC]** The document gives no timing: no gap between frames, no
deadline for a reply or a Result, nothing matching Voltsoft's 100 ms
between acks.

## Live Data Frame (66 bytes) -- [VENDOR]

```
Offset  Size  Field
0-1     2     Header: 0xAB 0xCD
2       1     Length
3       1     Type: 0x01 (LiveData)
4       1     Function code (0x00-0x12)
5       1     Range byte (0x30-based)
6-12    7     Display 1: main value (ASCII)
13-20   8     Display 2: time (ASCII)
21-30   10    Display 3: date (ASCII)
31-38   8     Display 4 (ASCII)
39-46   8     Display 5 (ASCII)
47-49   3     Second frequency unit (ASCII)
50-53   4     Display 6 (ASCII)
54-55   2     Bar: tens digit, units digit
56      1     Status 0: COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
              (bits 0-1 are sign bits in Rev 1.3 — see below)
57      1     Status 1: Rel(0), Avg(1), Min(2), Max(3)
58      1     Status 2: Hold(0), Manual(1), OL1(2), OL2(3)
59      1     Status 3: AutoPower(0), Warning(1), Loz(2), Void(3)
              (Loz and Void both exposed as dedicated bools in vendor:
              DMSShare_decompiled.cs:23638-23639; Void: "OLED show
              'VOID' symbol" [VENDOR-DOC] p.6, "Data memory contains
              no values" [MANUAL] §8, p.55)
60      1     Status 4: OuterSel(0), Pass(1), Comp(2), Log_h(3)
61      1     Status 5: Mem(0), BarPol(1), Clr(2), Shift(3)
62      1     Battery level (low nibble, raw 0-15 — see note below;
              0x30-0x33 = gear 0-3 [VENDOR-DOC] p.6)
63      1     Misplug warning (bits 1-0: 0=none, 1=mA err, 2=A err,
              3=V err — DMSShare_decompiled.cs:23649-23665); bit 2
              Memory_Overwrite [VENDOR-DOC] pp.6-7
64-65   2     Checksum (BE16)
```

**[VENDOR-DOC] pp.1, 3-4** gives the same offsets. Display 2 holds "the
time" and Display 3 "the date"; Displays 1, 4, 5 and 6 hold "the value
shown on" that display, all decoded as ASCII; the bar's two bytes are
"Msg[54]: tens digit, Msg[55]: a digit". The p.1 figure shows Display 2
`12:30` top left, Display 3 `2012-02-20` top right, Displays 4 and 5
`<514.32` and `>967.80` below them with COMP lit, and Display 6 `001`
beside LOG. On a NOCOMP log record, whose displays sit at the same offsets
(p.7), Voltsoft joins bytes 21-30 and 13-20 into the record's date and time
(DMSShare_decompiled.cs:23748-23772) [VENDOR]. What Displays 4-6 and the
second frequency unit hold outside COMP and logging: none found in Rev
1.3, the manual or Voltsoft, 2026-11-06 [UNVERIFIED].

**[VENDOR-DOC] p.5**: with MAX, MIN or AVG on, "the current transmitted
data at DISP1 is the maximum value at this range" (minimum, average), and
with REL "the relative value" — DISP1 where the VC-880's Rev 2.4 says DISP2
(VC-880 spec §4.3). The manual agrees: the held value "will be continuously
retained in the main display", the current measurement "in the upper area
of the display (below the date)" (§15, printed page 67) [MANUAL].

**[VENDOR-DOC] pp.5-7** gives bytes 56-63. Bytes 56-62 carry `0 0 1 1` in
bits 7-4, a `0x30` prefix, and byte 63 carries `0 0 1 1 0` in bits 7-3.
Byte 56's bits 0 and 1 are `comp_max_pol_flag` and `comp_min_pol_flag`,
"the sign bit of the comp max value" and "comp min value", "1 means
negative value"; Voltsoft names them `COMP_Max_flag` and `COMP_Min_flag`
and on either copies Display 2 into its comp maximum or minimum
(DMSShare_decompiled.cs:23668-23678) [VENDOR]. Bits 2 and 3 are "the sign
bit of the DISP1" and "DISP2". Rev 1.3's names for bytes 57-61 match the rows
above but for byte 60 bit 0, `Inner_flag` ("1: Inner 0: Outer"), and byte 61
bit 0, `log_A_flag` ("the continuous memory function is on or not"). Byte
62's bits are `bat_grade3_flag` to `bat_grade0_flag`, and "Msg[62].bytes =
0x30, the battery quantum show 0 gear" through 0x33, 3 gear. Byte 63's
bit 2 is `Memory_Overwrite_flag`, bits 1 and 0 `misplug_warning_flag2` and
`flag1`, read as "Msg[63].bytes&0x34 = 0x30; misplug NO EEROR" through
"= 0x33; misplug EEROR ON V INPUT" as printed, the last two at the top of
p.7, where `Memory_Overwrite_flag` is "Memory Type: Overwrite". The
printed mask `&0x34` cannot give 0x31-0x33; `&0x33` fits the four values.
Voltsoft reads `msg[63] & 0xF` and has no arm for 4-15, so with bit 2 set
it keeps its last misplug text [VENDOR].

**Battery level (byte 62, low nibble)**: The DLL stores the raw 0–15
value (`DMSShare_decompiled.cs:23648`, `battery_flag = msg[62] & 0xF`)
and reads it nowhere else. Voltsoft's `DeviceClient.exe` and its web data
logger (`DataLogger/VC890/vc890_data.aspx.cs:1142-1149`) pick one of four
icons for gears 0-3, `battery0` to `battery3`, running from an empty
outline to a full one, and neither has a low-battery threshold or text
[VENDOR]. Rev 1.3 names the gears and says nothing about low [VENDOR-DOC].
Contrast VC-880 (`msg[33]` bit 3 is a single `Low_batt_flag` bool at
`DMSShare_decompiled.cs:16799`, which Voltsoft shows as "Low Battery").
Which gear the meter's low-battery symbol goes with is [UNVERIFIED]
([verification](verification.md#status-bytes)).

## Function Codes -- [VENDOR]

**Remapped from VC-880** — same set of 19 functions but different codes!
Rev 1.3 p.3 [VENDOR-DOC] lists the same codes, continuity as
"Short-Circuit Test".

| Code | Function | VC-880 equivalent |
|------|----------|-------------------|
| 0x00 | AC V | was 0x05 |
| 0x01 | ACV Low-Pass | was 0x12 |
| 0x02 | DC V | was 0x00 |
| 0x03 | AC+DC V | was 0x01 |
| 0x04 | DC mV | was 0x02 |
| 0x05 | Frequency | was 0x03 |
| 0x06 | Duty % | was 0x04 |
| 0x07 | Resistance | was 0x06 |
| 0x08 | Continuity | same |
| 0x09 | Diode | was 0x07 |
| 0x0A | Capacitance | was 0x09 |
| 0x0B | Temperature °C | was 0x0A |
| 0x0C | Temperature °F | was 0x0B |
| 0x0D | DC µA | was 0x0C |
| 0x0E | AC µA | was 0x0D |
| 0x0F | DC mA | was 0x0E |
| 0x10 | AC mA | was 0x0F |
| 0x11 | DC A | was 0x10 |
| 0x12 | AC A | was 0x11 |

## Rotary positions and SHIFT/SETUP sub-functions -- [MANUAL]

The dial picks a *position*; within a position the SHIFT/SETUP button (3)
steps through the functions that position offers. The host cannot turn the
dial, so this table is the boundary of what remote mode selection can reach.

Sources: Fig. 1 on printed page 54 (PDF page 11) of the English VC-890
operating instructions, whose layout gives each position its symbols — the
red ones are the SHIFT/SETUP sub-functions — and the §11 measurement
procedures, which say the same in words. In the quotes below, `[…]` marks a
range symbol that the PDF renders as a glyph the text layer drops; it is read
off the figure, not off the sentence.

| Position | Functions (primary first) | Codes | Manual |
|----------|---------------------------|-------|--------|
| V~ (red Lo) | AC V, ACV low-pass | 0x00, 0x01 | §11j: "select the measurement range “V\[~\]”. Press the SHIFT/SETUP button (3) to switch to the measurement range “\[Lo\]”" |
| V⎓ (red AC+DC) | DC V, AC+DC V | 0x02, 0x03 | §11b: "If required you can select the “AC+DC” measuring function … select the measuring range “V\[⎓\]”. Press the SHIFT/SETUP button (3) to switch to the “AC+DC” measuring function" |
| mV⎓ Hz % | DC mV, Frequency, Duty % | 0x04, 0x05, 0x06 | §11d: "select the measuring range “mV Hz %”. Press the SHIFT/SETUP button (3) until “Hz” appears … press the SHIFT/SETUP button again until “%” appears" |
| Ω (red diode, continuity) | Ω, Diode, Continuity | 0x07, 0x09, 0x08 | §11f/§11g: "select the measuring range “Ω”. Press the SHIFT/SETUP button (3) until the diode test symbol appears" / "until the continuity test symbol appears" |
| ⊣⊢ | Capacitance | 0x0A | §11h |
| °C°F | °C, °F | 0x0B, 0x0C | §11i: "Press the SHIFT/SETUP button (3) to switch to a display in °F" |
| µA≂ | DC µA, AC µA | 0x0D, 0x0E | §11c |
| mA≂ | DC mA, AC mA | 0x0F, 0x10 | §11c |
| A≂ | DC A, AC A | 0x11, 0x12 | §11c: "Press the SHIFT/SETUP button (3) to switch to the AC measuring range … Pressing the button again will switch back" |

**Order is unverified.** Every source says *which* symbol a position offers
("until … appears"), never in which order the presses walk them. The order
above is the manual's order of description, not a claim about the meter.

**No function code is on two positions**, unlike the VC-880's 0x02, so a
reading names its dial position unambiguously.

**LoZ is not a function.** "Low Imp. 400 kΩ" is its own front-panel button
(§16) and shows up as the `Loz` status bit, not as a function code, so it is
not something SHIFT/SETUP can reach and not in this table.

**Note the differences from the VC-880's dial** (§4.4 of that spec): the
low-pass filter is a SHIFT/SETUP sub-function of V~ here rather than a dial
position of its own, and this dial has two OFF positions, one at each end of
the sweep.

**How the implementation uses this.** `crates/dmm-lib/src/protocol/vc8x0/vc890.rs`
holds the table as `DIAL`, one entry per position, each a single SHIFT/SETUP
ring (the VC-890 has no Hz/% button). It is membership only: the shared driver
in `protocol/cycle.rs` presses `Select` (0x4C) and reads the function code back
until the target appears, which works whatever the real press order is. Nothing
in the table is hardware-confirmed.

## Range Tables -- [VENDOR]

60,000 counts: range values are 6/60/600 (vs 4/40/400 for VC-880).

Rev 1.3's range table (p.4) [VENDOR-DOC] has the same rows for every
function below except AC+DC V and low-pass, which it has no column for, and
the two notes below: it spells capacitance index 2 "6000 nF" and leaves
row 0x37 blank.

| Function | Ranges |
|----------|--------|
| Voltage (0x00, 0x02, 0x03) | 6V, 60V, 600V, 1000V |
| ACV LPF (0x01) | fixed 1000V — vendor never reads the range byte (`case 1` at DMSShare_decompiled.cs:23466-23469); see below |
| DC mV (0x04) | 600mV |
| Frequency (0x05) | 60Hz, 600Hz, 6kHz, 60kHz, 600kHz, 6MHz, 60MHz, 600MHz |
| Resistance (0x07) | 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ |
| Capacitance (0x0A) | 60nF, 600nF, 6µF, 60µF, 600µF, 6000µF, 60mF |
| DC/AC µA (0x0D/0x0E) | 600µA, 6000µA |
| DC/AC mA (0x0F/0x10) | 60mA, 600mA |
| DC/AC A (0x11/0x12) | 10A |
| Duty (0x06), Diode (0x09), Cont (0x08), Temp | single range |

**ACV LPF.** The manual lists four low-pass ranges, 6 V, 60 V, 600 V and
1000 V (printed page 85) [MANUAL]; Voltsoft fixes 1000 V and Rev 1.3 has
no column. What the range byte carries in LPF, and whether the meter
auto-ranges there, is [UNVERIFIED].

**Frequency 0x37 (600MHz)** is the vendor software's alone
(DMSShare_decompiled.cs:23460-23464). Rev 1.3's 0x37 row is blank in every
column, its FREQ column ending at 60MHz on 0x36, and the manual's table
stops at 60 MHz (printed page 84). Whether the meter ever sends 0x37 is
[UNVERIFIED].

**Capacitance index 2** is "6 µF" in the manual (printed page 84) and the
vendor software (DMSShare_decompiled.cs:23263-23274), "6000 nF" in Rev 1.3
(p.4): the same full scale. Which digits Display 1 carries on this range,
`6.0000` or `6000.0`, no source shows [UNVERIFIED]; a decoder that takes
µF from the range byte would read `6000.0`-style digits 1000× high.

## Commands -- [VENDOR]

Rev 1.3 pp.10-11 [VENDOR-DOC] lists the VC-880's command bytes (VC-880
spec §5), 0x50 named "SET_COMP_ENTER (SET MENU 2)", and these besides; the
Data column is the document's, whether the command carries data:

| Byte | Rev 1.3 name | Data |
|------|--------------|------|
| 0x03 | GET SETUP Data | No |
| 0x5D | Set date/Time ENTER (SET MENU 1) | No |
| 0x5E | PC→VC890 Send Current Value | No |
| 0x5F | Set Time VALUE | Yes |
| 0x60 | Set date VALUE | Yes |
| 0x61 | Set date/Time ESC | No |
| 0x62 | Enter Data Log Set (SET MENU 3) | No |
| 0x63 | OLED Display Off After 5mins' Data Log | No |
| 0x64 | OLED Display On After 5 mins' Data Log | No |
| 0x65 | Memory Type: FIX | No |
| 0x66 | Memory Type: OVERWRITE | No |
| 0x67 | Continuous Sample Time | Yes |
| 0x68 | Exit Data Log Set (SET MENU 3) | No |
| 0x69 | Enter Other Set (SET MENU 4) | No |
| 0x6A | OLED Brightness Level 1 Enabled After 15 Seconds' Inactivity | No |
| 0x6B | OLED Brightness Level 1 Disabled After 15 Seconds' Inactivity | No |
| 0x6C | Battery Type: Alkaline | No |
| 0x6D | Battery Type: LI-AKKU | No |
| 0x6E | Exit Other Set (SET MENU 4) | No |
| 0x6F | Set APO Time: 5 min | No |
| 0x70 | Set APO Time: 15 min | No |
| 0x71 | Set APO Time: 30 min | No |
| 0x72 | Set APO Time: off min | No |

Its §4.4 and §4.5 (p.12) lay out SET_System_Time and SET_System_Date with
6 data bytes each, without naming their command bytes. Voltsoft sets the
clock with 0x5D, then 0x60 with the date, 0x5F with the time and 0x61,
waiting for a Result after 0x61 only (`SendTime`, line 4078) [VENDOR].
What the meter does with a 0x5F frame without data, as UT61E+ Get Name is
sent: none found in Rev 1.3, the manual or Voltsoft, 2026-11-06
[UNVERIFIED].
