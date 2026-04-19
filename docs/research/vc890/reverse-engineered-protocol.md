# VC-890: Reverse-Engineered Protocol Specification

Based on ILSpy decompilation of Voltsoft `DMSShare.dll` — same source
as the VC-880 RE. The VC-890 uses classes `VC890Obj` and `VC890Reading`
in the same DLL.

See `docs/research/vc880/reverse-engineering-approach.md` for methodology.

Confidence levels: **[VENDOR]** = from Voltsoft decompilation.

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
60      1     Status 4: OuterSel(0), Pass(1), Comp(2), Log_h(3)
61      1     Status 5: Mem(0), BarPol(1), Clr(2), Shift(3)
62      1     Battery level (low nibble)
63      1     Misplug warning (low nibble: 0=none, 1=mA err, 2=A err)
64-65   2     Checksum (BE16)
```

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

## Range Tables -- [VENDOR]

60,000 counts: range values are 6/60/600 (vs 4/40/400 for VC-880).

| Function | Ranges |
|----------|--------|
| Voltage (0x00-0x03) | 6V, 60V, 600V, 1000V |
| DC mV (0x04) | 600mV |
| Frequency (0x05) | 60Hz, 600Hz, 6kHz, 60kHz, 600kHz, 6MHz, 60MHz, 600MHz |
| Resistance (0x07) | 600Ω, 6kΩ, 60kΩ, 600kΩ, 6MΩ, 60MΩ |
| Capacitance (0x0A) | 60nF, 600nF, 6µF, 60µF, 600µF, 6000µF, 60mF |
| DC/AC µA (0x0D/0x0E) | 600µA, 6000µA |
| DC/AC mA (0x0F/0x10) | 60mA, 600mA |
| DC/AC A (0x11/0x12) | 10A |
| Duty (0x06), Diode (0x09), Cont (0x08), Temp, LPF | single range |

## Commands -- [VENDOR]

Same command bytes as VC-880, plus:
- `0x5D` = Set Time (VC-890 only)
- `0x5E` = Get Measurement (polled, VC-890 only)
