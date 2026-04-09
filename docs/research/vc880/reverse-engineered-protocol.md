# VC-880 / VC650BT: Reverse-Engineered Protocol Specification

Based on ILSpy decompilation of Voltsoft `DMSShare.dll` and the VC880
user manual. See `reverse-engineering-approach.md` for methodology.

Confidence levels:
- **[KNOWN]** -- from the user manual
- **[VENDOR]** -- from Voltsoft decompilation
- **[INFERRED]** -- logically deduced
- **[UNVERIFIED]** -- requires real device testing

---

## 1. Transport Layer -- [VENDOR]

Silicon Labs CP2110 HID-to-UART bridge (VID `0x10C4`, PID `0xEA80`).

**UART configuration** (from `VC880Obj.OpenDevice()`, line 21228):
```
HidUart_SetUartConfig(handle, 9600, 3, 2, 0, 0)
  baudRate = 9600
  dataBits = 3  (SiLabs HID_UART_EIGHT_DATA_BITS)
  parity   = 2  (SiLabs HID_UART_NO_PARITY — note: enum order differs from Win32)
  stopBits = 0  (SiLabs HID_UART_SHORT_STOP_BIT)
  flowCtrl = 0  (SiLabs HID_UART_NO_FLOW_CONTROL)
```

**Note**: The SiLabs HID UART API uses different enum values than the
Win32 serial API. `dataBits=3` maps to 8 data bits in the SiLabs
`HID_UART_DATA_BITS` enum. [VENDOR, cross-check with SLABHIDtoUART.h]

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

From `VC880Obj` fields (lines 21049-21059):

| Type | Value | Frame Size | Description |
|------|-------|------------|-------------|
| DeviceID | 0x00 | Variable | Device identification string |
| LiveData | 0x01 | 39 bytes | Real-time measurement (continuous stream) |
| CompData | 0x02 | 23 bytes | Comparator mode data |
| NOCOMPDataTransfer | 0x03 | 15 bytes | Non-COMP log data transfer |
| COMPDataTransfer | 0x04 | 30 bytes | COMP log data transfer |
| Result | 0xFF | Variable | Command result/acknowledgement |

Frame sizes include the 2-byte header + length byte + type byte + payload + 2-byte checksum.

**Checksum validation per message type** (from `CheckSum()`, line 21756):
- LiveData (0x01): sum bytes 0..36, check against bytes 37-38. Total = 39 bytes.
- CompData (0x02): sum bytes 0..20, check against bytes 21-22. Total = 23 bytes.
- NOCOMPDataTransfer (0x03): sum bytes 0..12, check against bytes 13-14. Total = 15 bytes.
- COMPDataTransfer (0x04): sum bytes 0..27, check against bytes 28-29. Total = 30 bytes.

---

## 4. Live Data Payload (type 0x01, 39 bytes) -- [VENDOR]

This is the primary measurement frame, streamed continuously at ~2-3 Hz.

```
Offset  Size  Field           Description
0       2     Header          0xAB 0xCD
2       1     Length          0x24 (36 = 33 data + 3)
3       1     Type            0x01 (LiveData)
4       1     Function        Function code (0x00-0x12), see §4.1
5       1     Range           Range index (0x30-based ASCII), see §4.2
6-12    7     Main value      Primary display (7 ASCII bytes)
13-19   7     Sub value 1     Secondary display 1 (7 ASCII bytes)
20-26   7     Sub value 2     Secondary display 2 (7 ASCII bytes)
27-29   3     Bar/Sub value 3 Bar graph or tertiary value (3 ASCII bytes)
30      1     Status 0        COMP_Max(0), COMP_Min(1), Sign1(2), Sign2(3)
31      1     Status 1        Rel(0), Avg(1), Min(2), Max(3)
32      1     Status 2        Hold(0), Manual(1), OL1(2), OL2(3)
33      1     Status 3        AutoPower(0), Warning(1), Light(2), LowBatt(3)
34      1     Status 4        OuterSel(0), Pass(1), Comp(2), MisplugWarn(3)
35      1     Status 5        Mem(0), BarPol(1), Clr(2), Shift(3)
36      1     Status 6        DoubleDisp(0), Setup(1), BarOL(2), BarDispEn(3), PassBeep(4), NgBeep(5)
37-38   2     Checksum        BE16 sum of bytes 0-36
```

**Display values**: 7 bytes each, parsed as ASCII strings via
`Encoding.ASCII.GetString()`. Right-justified with leading spaces.
"OL" or "---" for overload (observed in pylablib, [UNVERIFIED] from
vendor code). [VENDOR: `SetReadingValue()` line 16726]

### 4.1 Function Codes (byte 4) -- [VENDOR]

From `SetDeviceMode_And_Unit_And_Range()` switch statement (line 16335):

| Code | Function | Unit(s) | Manual Section |
|------|----------|---------|----------------|
| 0x00 | DC Voltage | V, mV | §8b, page 62 |
| 0x01 | AC+DC Voltage | V | §8b, page 63 |
| 0x02 | DC Millivolt | mV | §8b, page 62 |
| 0x03 | Frequency | Hz, kHz, MHz | §8d, page 64 |
| 0x04 | Duty Cycle | % | §8d, page 65 |
| 0x05 | AC Voltage | V | §8b, page 62-63 |
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
| 0x12 | ACV Low-Pass | V | §8j, page 63 |

### 4.2 Range Tables (byte 5) -- [VENDOR] + [KNOWN]

Range byte is ASCII-encoded: `0x30` = range index 0, `0x31` = index 1, etc.

Tables derived from the vendor switch statement, cross-referenced against
manual spec tables.

**Voltage (functions 0x00 DCV, 0x01 AC+DC V, 0x05 ACV, 0x12 LPF ACV)**:

| Index | Range | Resolution | Manual |
|-------|-------|------------|--------|
| 0 (0x30) | 4V | 0.0001V | ✓ page 62 |
| 1 (0x31) | 40V | 0.001V | ✓ |
| 2 (0x32) | 400V | 0.01V | ✓ |
| 3 (0x33) | 1000V | 0.1V | ✓ |

Note: DCV function 0x00 also has a 400mV range. When function=0x00 and
range=0, the vendor maps to measurement 5 (DCV, V, 4V range). The 400mV
range appears as function=0x02 (DCV mV) with range=0 (400mV). [VENDOR]

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

**Frequency (function 0x03)**:

| Index | Range | Unit | Manual |
|-------|-------|------|--------|
| 0 (0x30) | 40Hz | Hz | ✓ page 64 |
| 1 (0x31) | 400Hz | Hz | ✓ |
| 2 (0x32) | 4kHz | kHz | ✓ |
| 3 (0x33) | 40kHz | kHz | ✓ |
| 4 (0x34) | 400kHz | kHz | ✓ |
| 5 (0x35) | 4MHz | MHz | ✓ |
| 6 (0x36) | 40MHz | MHz | ✓ |
| 7 (0x37) | 400MHz | MHz | ✓ (unspecified accuracy) |

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
- ACV Low-Pass (0x12): range = 1000 [VENDOR]

### 4.3 Status Flag Bytes -- [VENDOR]

From `SetStatus()` (line 16782). Each flag is extracted as
`(byte & (1 << position)) != 0`.

**Byte 30 (Status 0)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | COMP_Max | Comparator max flag |
| 1 | COMP_Min | Comparator min flag |
| 2 | Sign1 | Primary display sign (1=negative) |
| 3 | Sign2 | Secondary display sign (1=negative) |

**Byte 31 (Status 1)**:

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
| 0 | OuterSel | Comp mode: outer selection |
| 1 | Pass | Comp mode: pass result |
| 2 | Comp | Comparator mode active |
| 3 | MisplugWarn | Misplug warning (leads in wrong jacks) |

**Byte 35 (Status 5)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | Mem | Memory/logging active |
| 1 | BarPol | Bar graph polarity (1=negative) |
| 2 | Clr | Clear flag |
| 3 | Shift | Shift mode active |

**Byte 36 (Status 6)**:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | DoubleDisp | Dual display enabled |
| 1 | Setup | Setup mode active |
| 2 | BarOL | Bar graph overload |
| 3 | BarDispEn | Bar graph display enabled |
| 4 | PassBeep | Pass beep enabled (comp mode) |
| 5 | NgBeep | NG (fail) beep enabled (comp mode) |

---

## 5. Commands (host → meter) -- [VENDOR]

From `VC880Obj` static fields (lines 21063-21125):

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

**No trigger needed**: Unlike the UT8803 (which requires a `0x5A` trigger
byte), the VC-880 streams immediately after USB connection + PC button
press. The `init()` method should be a no-op. [VENDOR + INFERRED]

**Read loop**: `ContinuouslyReadDataLoop()` calls `ReadData()` in a
loop. `ReadData()` reads up to 50 bytes, appends to a buffer, then
calls `ProcessMessages()` which extracts and dispatches complete frames.

**Command flow**: Simple commands are sent as 6-byte frames. The meter
replies with a Result frame (type 0xFF) containing a 1-byte status:
0=success, 1=error-resend, 2=error-do-nothing.

---

## 7. Implementation Notes

### Framing reuse

The VC-880 uses **identical framing** to the UT61E+:
- Same header (`0xAB 0xCD`)
- Same length encoding (payload + 3)
- Same checksum (BE16 sum of all preceding bytes)

Reuse `extract_frame_abcd_be16()` with an accept filter for type 0x01.

### Key differences from UT61E+

| Aspect | UT61E+ | VC-880 |
|--------|--------|--------|
| Communication model | Polled (request → response) | Streaming (continuous) |
| Init | Send measurement request | No-op (streams automatically) |
| Display encoding | 7 raw bytes (sometimes non-ASCII) | 7 ASCII bytes |
| Range encoding | Raw byte with mode-specific meaning | ASCII '0'-'7' (0x30-based) |
| Status flags | 2 flag bytes (14 bits) | 7 flag bytes (28 named bits) |
| Commands | 5 (hold, rel, select, etc.) | 28 (including comp mode, logging) |

### Key differences from UT8803

| Aspect | UT8803 | VC-880 |
|--------|--------|--------|
| Frame format | Fixed 21 bytes, custom checksum | Variable length, AB CD + BE16 checksum |
| Init | Send 0x5A trigger | No trigger needed |
| Display encoding | 5 raw bytes | 7 ASCII bytes |
| Function codes | 23 modes (0x00-0x16) | 19 modes (0x00-0x12) |
| Overload | Flag bit in status word | OL1/OL2 flags + "OL" in display string |
