# VC-890: Reverse-Engineered Protocol Specification

Based on ILSpy decompilation of Voltsoft `DMSShare.dll` — same source
as the VC-880 RE. The VC-890 uses classes `VC890Obj` and `VC890Reading`
in the same DLL.

See `docs/research/vc880/reverse-engineering-approach.md` for methodology.

Confidence levels: **[VENDOR]** = from Voltsoft decompilation,
**[MANUAL]** = from the VC-890 user manual.

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
| Battery indicator | Flag bit (LowBatt) | Nibble value (level 0-?) |

---

## Frame Format -- [VENDOR]

Same AB CD + BE16 framing as VC-880 and UT61E+.

## Communication Model -- [VENDOR]

**Polled**: Host sends measurement request (0x5E), meter responds with
one live data frame. From `VC890Obj.GetReading()` (line 4027):
```csharp
WriteCommand(94);  // 0x5E
byte[] array = ReceiveOneMessage(1);  // type 0x01
```

**Command confirmation**: `SendCommand(cmd)` (line 4056) writes the
command and waits for a frame whose type byte is the command byte itself,
retrying up to 5 times, so the meter answers a button command with a frame
of that type. `ReceiveOneMessage` skips frames of any other type, which
leaves open whether the 0x5E poll is echoed ahead of its live frame.

**Ack protocol**: The Voltsoft vendor software wraps every VC-890
request/response pair in an ack sequence. `AckMessage(clear: true)` at
`DMSShare_decompiled.cs:3861` sends `command = 0xFF` with `data = [0x00]`
three times with a 100ms `Thread.Sleep` between writes and a
`FlushBuffer()` afterwards. Using `WriteCommand(byte, byte[], bool)` at
line 3805 to build the frame yields the exact wire bytes

```
AB CD 04 FF 00 02 7B
```

(`0x04` = header(2) + command(1) + data(1), and checksum = AB + CD + 04
+ FF + 00 = 0x027B, BE).

The sequence is invoked in two places per measurement cycle:

1. **Pre-clear** — `WriteCommand(byte command, bool ack = true)`
   (line 3773) calls `AckMessage(clear: true)` before writing the
   command frame. `GetReading()` / `SendCommand()` take this default
   path, so every outgoing command is preceded by the 3× ack burst.
2. **Post-confirm** — `ReceiveOneMessage(byte messageType, bool ack =
   true)` (line 3977) calls `AckMessage(ack)` right after a valid
   response frame is reassembled (line 4009). Default `ack = true` →
   3× ack burst after every received frame.

A similar single-shot `AckMessage(clear: false)` (one write, no sleeps)
is used on an HID read error path at line 3941. The meter still
receiving / initiating vs. requiring the ack is [UNVERIFIED], but the
vendor's double bracketing is strong enough evidence to ship the
sequence on both sides of a measurement.

## Live Data Frame (66 bytes) -- [VENDOR]

```
Offset  Size  Field
0-1     2     Header: 0xAB 0xCD
2       1     Length
3       1     Type: 0x01 (LiveData)
4       1     Function code (0x00-0x12)
5       1     Range byte (0x30-based)
6-12    7     Value 1: main display (ASCII)
13-20   8     Value 2: sub display (ASCII)
21-30   10    Value 3: (ASCII)
31-38   8     Value 4: (ASCII)
39-46   8     Value 5: (ASCII)
47-49   3     Second frequency unit (ASCII)
50-53   4     Value 6: (ASCII)
54-55   2     Bar graph
56      1     Status 0: COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
57      1     Status 1: Rel(0), Avg(1), Min(2), Max(3)
58      1     Status 2: Hold(0), Manual(1), OL1(2), OL2(3)
59      1     Status 3: AutoPower(0), Warning(1), Loz(2), Void(3)
              (Loz and Void both exposed as dedicated bools in vendor:
              DMSShare_decompiled.cs:23638-23639)
60      1     Status 4: OuterSel(0), Pass(1), Comp(2), Log_h(3)
61      1     Status 5: Mem(0), BarPol(1), Clr(2), Shift(3)
62      1     Battery level (low nibble, raw 0-15 — see note below)
63      1     Misplug warning (low nibble: 0=none, 1=mA err, 2=A err,
              3=V err — DMSShare_decompiled.cs:23649-23665)
64-65   2     Checksum (BE16)
```

**Battery level (byte 62, low nibble)**: The DLL stores the raw 0–15
value (`DMSShare_decompiled.cs:23648`, `battery_flag = msg[62] & 0xF`)
and does nothing else with it — `battery_flag` is declared `public int`
but never read elsewhere in DMSShare.dll, and no low-battery threshold
is computed at this layer. The thresholding therefore lives in the
VoltSoft GUI (`VoltSoft System.exe` / `DeviceClient`, not yet
decompiled). Contrast VC-880 (`msg[33]` bit 3 is a single `Low_batt_flag`
bool at `DMSShare_decompiled.cs:16799`). Until the GUI is reversed or
a real device is tested, consumers should surface the raw level and
treat "low battery" conservatively.

## Function Codes -- [VENDOR]

**Remapped from VC-880** — same set of 19 functions but different codes!

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

| Function | Ranges |
|----------|--------|
| Voltage (0x00, 0x02, 0x03) | 6V, 60V, 600V, 1000V |
| ACV LPF (0x01) | fixed 1000V — vendor never reads the range byte (`case 1` at DMSShare_decompiled.cs:23466-23469); wire contents of the range byte in LPF mode unknown |
| DC mV (0x04) | 600mV |
| Frequency (0x05) | 60Hz, 600Hz, 6kHz, 60kHz, 600kHz, 6MHz, 60MHz, 600MHz |
| Resistance (0x07) | 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ |
| Capacitance (0x0A) | 60nF, 600nF, 6µF, 60µF, 600µF, 6000µF, 60mF |
| DC/AC µA (0x0D/0x0E) | 600µA, 6000µA |
| DC/AC mA (0x0F/0x10) | 60mA, 600mA |
| DC/AC A (0x11/0x12) | 10A |
| Duty (0x06), Diode (0x09), Cont (0x08), Temp | single range |

## Commands -- [VENDOR]

Same command bytes as VC-880, plus:
- `0x5D` = Set Time (VC-890 only)
- `0x5E` = Get Measurement (polled, VC-890 only)
