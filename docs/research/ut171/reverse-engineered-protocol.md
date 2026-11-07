# UT171 Family Protocol: Reverse-Engineered Specification

Based on:
- UT171A/B/C User Manual (UNI-T), read from page renders
- UT171C PC Software (UNI-T) -- Ghidra decompilation of UT171C.exe (881K
  lines), and `objdump` disassembly by virtual address (VA) of the main
  form unit the decompile leaves out (0xBB6D08–0xBC3400)
- SLABHIDtoUART.dll (Silicon Labs) -- Ghidra decompilation (13K lines)
- UT171C Software Installation Instruction (UNI-T)

Cross-referenced against:
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) (Python
  parser for the UT171A), in §10 only

No UT171 has been read yet (issue #4). Every `[VENDOR]` fact below is what
UNI-T's software reads or sends; a meter could differ, and the
[verification list](verification.md) holds the checks.

UT171C.exe citations are VAs in the binary with SHA-256 `ff3368a7…f238`;
`.text` file offset = VA − 0x401000 + 0x400. The approach doc has the method.

Confidence levels:
- **[KNOWN]** -- from official UNI-T manual
- **[VENDOR]** -- from analysis of UNI-T's official vendor software
- **[DEDUCED]** -- logical inference from available evidence
- **[UNVERIFIED]** -- requires real device testing to confirm

---

## 1. Device Overview -- [KNOWN]

### 1.1 Model Comparison

| Feature | UT171A | UT171B | UT171C |
|---------|--------|--------|--------|
| Display count | 40,000 (39,999) | 60,000 (59,999) | 60,000 (59,999) |
| Display type | LCD | VT-WLCD | OLED |
| Battery | AAA x6 | Li-Ion 7.4V/1800mAh | Li-Ion 7.4V/1800mAh |
| Operating temp | 0-40C | 0-40C | -30C to 40C |
| Safety | CAT III 1000V / CAT IV 600V | Same | Same |
| IP rating | IP67 | IP67 | IP67 |
| Data logging | Yes | Yes | Yes |
| USB | CP2110 | CP2110 | CP2110 |
| Bluetooth | No | Yes (UT-D07A/B) | Yes (UT-D07A/B) |
| Conductance (nS) | No | Yes | Yes |
| Temperature | No | Yes (K-type) | Yes (K-type) |
| Square wave out | No | No | Yes |
| 600A clamp | No | No | Yes |
| Bar graph | 21 segments | 31 segments | 31 segments |

The record capacity: none found in the manual's storage section or its
page-4 specification list (page renders, 2026-11-07); the 9,999 records an
earlier reading gave are [UNVERIFIED].

---

## 2. Transport Layer -- [VENDOR]

### 2.1 USB Configuration

From SLABHIDtoUART.dll decompilation and UT171C.exe:

| Parameter | Value | Source |
|-----------|-------|--------|
| USB VID | 0x10C4 | Ghidra: string `L"10C4"` in UT171C.exe |
| USB PID | 0xEA80 | Ghidra: string `L"EA80"` in UT171C.exe |
| Bridge chip | CP2110 | SLABHIDtoUART.dll checks part number = 0x0A |
| Baud rate | 9600 | Ghidra: baud index 6 → 0x00002580 (BE in report 0x50) |
| Data bits | 8 | Ghidra: data bits index 3 → 8-bit |
| Parity | None | Ghidra: parity index 0 |
| Stop bits | 1 (short) | Ghidra: stop bits index 0 |
| Flow control | None | Ghidra: flow control = 0 |

### 2.2 CP2110 Feature Report Map -- [VENDOR]

From SLABHIDtoUART.dll Ghidra decompilation (complete):

| Report ID | Direction | Function | Purpose |
|-----------|-----------|----------|---------|
| 0x40 | Set | HidUart_Reset | Device reset |
| 0x41 | Set/Get | HidUart_SetUartEnable | Enable/disable UART (`[0x41, 0x01]` = enable) |
| 0x42 | Get | HidUart_GetUartStatus | UART status (TX/RX FIFO counts BE, error flags) |
| 0x43 | Set | HidUart_FlushBuffers | Purge FIFOs (0x01=TX, 0x02=RX, 0x03=both) |
| 0x44 | Get | HidUart_ReadLatch | Read GPIO latch |
| 0x45 | Set | HidUart_WriteLatch | Write GPIO latch (16-bit BE) |
| 0x46 | Get | HidUart_GetPartNumber | Part number (must be 0x0A for CP2110) + version |
| 0x47 | Set/Get | HidUart_SetLock/GetLock | Lock OTP configuration |
| 0x50 | Set/Get | HidUart_SetUartConfig | Configure baud/parity/data/stop/flow |
| 0x51 | Set | HidUart_StartBreak | Start line break |
| 0x52 | Set | HidUart_StopBreak | Stop line break |
| 0x60 | Set/Get | HidUart_SetUsbConfig | Set/get USB VID/PID/power/mode |
| 0x65 | Set/Get | (internal) | String descriptor (OTP) |
| 0x66 | Set/Get | HidUart_SetPinConfig | Configure GPIO pins |

All feature reports use 64-byte buffers. Report ID is byte[0].

### 2.3 UART Config Report 0x50 Layout -- [VENDOR]

```
byte[0] = 0x50   (report ID)
byte[1] = (baud >> 24) & 0xFF   (MSB)
byte[2] = (baud >> 16) & 0xFF
byte[3] = (baud >>  8) & 0xFF
byte[4] = baud & 0xFF           (LSB)
byte[5] = parity       (0=none, 1=odd, 2=even, 3=mark, 4=space)
byte[6] = flowControl  (0=none, 1=RTS/CTS)
byte[7] = dataBits     (0=5, 1=6, 2=7, 3=8)
byte[8] = stopBits     (0=short/1, 1=long/1.5-2)
```

For 9600/8N1: `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00]`
-- matches our existing UT61E+ configuration exactly.

### 2.4 Initialization Sequence -- [VENDOR]

From SLABHIDtoUART.dll `HidUart_Open`:
1. Open HID device matching VID/PID
2. Get part number (report 0x46) — verify = 0x0A (CP2110)
3. Enable UART (report 0x41: `[0x41, 0x01]`)

Then the application calls:
4. Set UART config (report 0x50: 9600/8N1)
5. Set timeouts (read: 500ms, write: 1000ms)

**Note**: `HidUart_Open` does NOT purge FIFOs or set baud rate. The
application must do these explicitly after open.

### 2.5 Data Transfer -- [VENDOR]

**Write** (host → meter): Report ID = payload byte count (1-63).
Data follows in the same 64-byte HID interrupt report. Max 4096
bytes per call, chunked into 63-byte reports automatically.

**Read** (meter → host): Same format. Library maintains internal
ring buffer, polls at 1ms HID timeout. Application-level read
timeout is 500ms.

### 2.6 Communication Architecture -- [VENDOR]

From UT171C.exe reader thread (`FUN_00755228`):
- Reader thread polls two Win32 events: shutdown and data-ready
- Data received via `HidUart_Read` into 2048-byte buffer
- Dispatched to main thread via `SendMessageW(hwnd, 0x466, buf, len)`
- Timer interval: 70ms (0x46) for polling (~14 Hz max)

---

## 3. Frame Format -- [VENDOR]

### 3.1 General Structure

```
+------+------+--------+--------+-----------+--------+--------+
| 0xAB | 0xCD | len_lo | len_hi | payload   | chk_lo | chk_hi |
+------+------+--------+--------+-----------+--------+--------+
 byte 0 byte 1 byte 2   byte 3   bytes 4..   len+2    len+3
```

| Field | Size | Encoding | Description |
|-------|------|----------|-------------|
| Header | 2 | Fixed | `0xAB 0xCD` |
| Length | 2 | uint16 LE | Payload byte count + 2 (the checksum) |
| Payload | len − 2 | Variable | Type or command byte, then its data |
| Checksum | 2 | uint16 LE | Sum of bytes[2..len+2): the length bytes and the payload |

Both directions use this shape: the app's receive check takes nothing else
(§3.3), and all 54 of its command send sites write it (§4.1). Byte offsets
in this spec count from the `0xAB`, as the app does.

### 3.2 Checksum Algorithm -- [VENDOR]

```
checksum = sum(frame[2], frame[3], ..., frame[len+1])
frame[len+2] = checksum & 0xFF       // low byte
frame[len+3] = (checksum >> 8) & 0xFF // high byte
```

The app's receive check sums bytes 2 to Count−3 and compares the result,
as a full integer with no masking, with the last two bytes read
little-endian (§3.3). Its dynamic command builders compute the same sum
(VA 0xBBC2F0, 0xBC2C38), and every constant command template in §4.2
carries it.

### 3.3 Frame Validation -- [VENDOR]

The app turns each received byte into a 2-character hex string
(`CP21101ReceiveData`, VA 0xBB92BC) and a timer (`tmr1Timer`, VA 0xBB978C)
cuts frames out of that list:

| Step | VA | What it does |
|---|---|---|
| Resync | 0xBB9860–0xBB98B4 | Finds "CD"; if the item before it is "AB", drops everything before the AB |
| Length | 0xBB98DB–0xBB9927 | Reads bytes 2–3 as a little-endian 16-bit length and waits for length + 4 items |
| Header check | 0xBB9A26, 0xBB9A9F | Byte 0 must be AB and byte 1 CD, or the data is dropped |
| Length check | 0xBB9B45–0xBB9B66 | Item count − 4 must equal the length field |
| Checksum | 0xBB9C1A–0xBB9CB6 | The §3.2 sum against the last two bytes; a mismatch is dropped |
| Dispatch | 0xBB9D44–0xBB9DD9 | On byte 4: `01` reply (§5.11), `72` query reply (§5.11), `02` measurement (§5.1), `03` stored record (§5.10) |

The dispatch compares the type byte with those four values only.

The functions an earlier reading cited for this check (`FUN_0065478e`,
`FUN_0065492d`, `FUN_00654bf5`) parse the app's Delphi Midas/ClientDataSet
packets, not wire frames
([approach](reverse-engineering-approach.md#what-misled-the-first-reading)).

### 3.4 Valid Frame Sizes -- [VENDOR]

Total = length + 4; payload = length − 2, bytes 4 to len+1.

| Frame | Direction | Length field | Total | Section |
|-------|-----------|--------------|-------|---------|
| Command with one parameter byte | host → meter | 0x04 | 8 | §4.2 |
| Command with a u16 parameter (0x0E, 0x0F, 0x10) | host → meter | 0x05 | 9 | §4.5, §4.6 |
| Start auto-save (0x0C) | host → meter | 0x06 | 10 | §4.4 |
| Clock sync (0x15) | host → meter | 0x07 | 11 | §4.8 |
| Set REL value (0x04 0x01) | host → meter | 0x08 | 12 | §4.7 |
| Square-wave settings (0x14) | host → meter | 0x0B | 15 | §4.8 |
| Reply OK/ER/NO (type 0x01) | meter → host | 0x05 | 9 | §5.11 |
| Measurement (type 0x02) | meter → host | 0x0D–0x19 | 17–29 | §5.1, §5.2 |
| Square wave (type 0x02, mode 0x1D) | meter → host | ≥ 0x14 | ≥ 24 | §5.9 |
| NCV (type 0x02, mode 0x1A) | meter → host | unknown | unknown | §5.8 |
| Stored record (type 0x03) | meter → host | ≥ 0x11, ≥ 0x17 with aux | ≥ 21, ≥ 27 with aux | §5.10 |
| Query reply (type 0x72) | meter → host | sub-code dependent | — | §5.11 |

A measurement payload is 11 bytes (type to unit code), plus 6 when byte 5
bit 0 is set (aux block), 4 when bit 3 is set (bar float) and 2 when bit 1
is set (STO minutes): 11 to 23 bytes (§5.2). The app reads the layout from
those flags, not from the length or from byte 6. For the stored record and
the query replies the sizes are the last byte the app reads; it checks no
length of its own on them.

The 1-byte-length layout and the lengths 0x03, 0x0A and 0x12 that this
section once listed came from a builder the app never calls (§4.1).

---

## 4. Commands (Host → Meter) -- [VENDOR]

None of these has been sent to a UT171 yet; what follows is what the app
sends.

### 4.1 Command Frame -- [VENDOR]

Every button writes a constant template from `.data` 0xBE1D9C–0xBE1F46
one byte at a time through `TCP2110`'s write method (VA 0x7551BC, 54 call
sites), or patches one field of it first. All of them use the §3.1 framing,
`AB CD len_lo len_hi cmd … chk_lo chk_hi`, with the command in byte 4.

`TCP2110.SendCommand` (`FUN_00755400`, VA 0x755400) writes a different
layout: a 1-byte length in byte 2 and the command in byte 3. An earlier
reading took it for the app's command builder. It is referenced only by its
own RTTI method-table entry; a direct-call search over `.text` found no
caller (2026-11-07). A call by name at run time was not ruled out. The
commands 0x01 (with a 7+7+1-byte payload), 0x51, 0x52 and 0xFF exist only
in it, and are not in the app's command path.

### 4.2 Command Table -- [VENDOR]

The button captions are the app's; the meanings follow them. Handler VAs
are UT171C.exe.

| Command | Bytes sent | Button | Handler VA | Section |
|---------|------------|--------|------------|---------|
| 0x01 function | `AB CD 04 00 01 MM (MM+5) 00` | Function buttons | 0xBB7570–0xBB7B63, 0xBB873C, 0xBB8778 | §4.7, §6 |
| 0x02 range | `AB CD 04 00 02 00 06 00` | Auto Model | 0xBB7B88 | §4.7 |
| 0x02 range | `AB CD 04 00 02 NN (NN+6) 00`, NN = row 1..8 | Manual Mode | 0xBB7BC4 | §4.7 |
| 0x03 Hz% | `AB CD 04 00 03 5A 61 00` | Hz% | 0xBB70C8 | §4.7 |
| 0x04 REL | `AB CD 08 00 04 01 <float32 LE> chk chk` | Set Relative Value | 0xBB806C | §4.7 |
| 0x04 REL | `AB CD 04 00 04 00 08 00` | Quit Relative Model | 0xBB82C0 | §4.7 |
| 0x05 MAX MIN | `AB CD 04 00 05 01 0A 00` / `… 05 00 09 00` | Max Min | 0xBB71BC | §4.7 |
| 0x06 Peak | `AB CD 04 00 06 01 0B 00` / `… 06 00 0A 00` | Peak | 0xBB70F8 | §4.7 |
| 0x07 HOLD | `AB CD 04 00 07 5A 65 00` | Hold | 0xBB7098 | §4.7 |
| 0x0A stream | `AB CD 04 00 0A 01 0F 00` | Start Receive Data | 0xBB7030 | §4.3 |
| 0x0A stream | `AB CD 04 00 0A 00 0E 00` | Pause Receive Data | 0xBB7064 | §4.3 |
| 0x0B save | `AB CD 04 00 0B 5A 69 00` | Save Current Data | 0xBB82F0 | §7 |
| 0x0C auto-save | `AB CD 06 00 0C <u8> <u16 LE> chk chk` | Start Auto Save | 0xBB8320 | §4.4 |
| 0x0D auto-save | `AB CD 04 00 0D 5A 6B 00` | Close Auto Save | 0xBB84EC | §4.4 |
| 0x0E read record | `AB CD 05 00 0E <u16 LE index> chk chk` | Read Data in Index, Read ALL | 0xBC2C00 | §4.5 |
| 0x0F delete record | `AB CD 05 00 0F <u16 LE index> chk chk` | Delete Data Index | 0xBB8E80 | §4.6 |
| 0x10 delete all | `AB CD 05 00 10 34 12 5B 00` | Delete All Data | 0xBB91D8 | §4.6 |
| 0x11 count | `AB CD 04 00 11 5A 6F 00` | Query Data Count | 0xBB803C | §4.5 |
| 0x12 memory | `AB CD 04 00 12 5A 70 00` | Memory State | 0xBB8540 | §4.5 |
| 0x14 square wave | `AB CD 0B 00 14 <4 bytes> <4 bytes> chk chk` | FREQ/DUTY Send | 0xBB87E4 | §4.8 |
| 0x15 clock | `AB CD 07 00 15 <u32 LE> chk chk` | (time sync) | 0xBBC194 | §4.8 |
| 0x16 device info | `AB CD 04 00 16 5A 74 00` | Device Info | 0xBB87B4 | §4.5 |

The meter's answer to a command is a type 0x01 reply, OK, ER or NO
(§5.11), or for the queries a type 0x72 reply. Which commands draw a 0x01
reply is unknown.

### 4.3 Streaming Commands -- [VENDOR]

**Connect (start streaming):** `AB CD 04 00 0A 01 0F 00`

**Pause (stop streaming):** `AB CD 04 00 0A 00 0E 00`

The manual turns communication on in the setup menu ("Usb", long-press
Hz% → Setup) and names no command. Whether a meter streams before the
connect frame, and whether it answers the connect with a 0x01 reply, is
unknown.

### 4.4 Start and Stop Auto-Save (0x0C, 0x0D) -- [VENDOR]

```
AB CD 06 00 0C <u8> <u16 LE> chk chk     start
AB CD 04 00 0D 5A 6B 00                  stop
```

The two start fields come from the app's parameter settings (config
offsets +0x1C and +0x20) and were not traced further. They are probably the
interval in seconds and the duration in minutes [UNVERIFIED]: the manual's
auto-storage menu sets an interval of 1–240 s and a duration in minutes,
the app's parameter dialog offers "1 to 40000 Minute", and `sys.ini` holds
`InterVal_Second` and `Duration_Minute`. While auto-save runs the meter's
frames carry byte 5 bit 1 and the STO minutes (§5.2), and the app refuses
Read ALL ("First Exit Auto Store Mode,Please!").

### 4.5 Read Commands (0x0E, 0x11, 0x12, 0x16) -- [VENDOR]

| Command | Parameter | Reply |
|---------|-----------|-------|
| 0x0E read record | u16 LE index, never 0 | Stored record, type 0x03 (§5.10) |
| 0x11 query count | `5A` | Type 0x72, sub-code 0x11: the count (§5.11) |
| 0x12 memory state | `5A` | Type 0x72, sub-code 0x12 (§5.11) |
| 0x16 device info | `5A` | Type 0x72, sub-code 0x16: model text and an ID (§5.11) |

The app asks for the count before reading ("First Query Data Count,
Please!"). Read ALL also sends 0x0E (from VA 0xBB8CCA); how it steps the
index was not traced.

### 4.6 Delete Commands (0x0F, 0x10) -- [VENDOR]

- 0x0F deletes one record: `AB CD 05 00 0F <u16 LE index> chk chk`.
- 0x10 deletes every record, with the constant 0x1234 as its parameter:
  `AB CD 05 00 10 34 12 5B 00`.

### 4.7 Function, Range and Key Commands -- [VENDOR]

| Command | Parameter | What it selects |
|---------|-----------|-----------------|
| 0x01 | Mode byte MM, one of 0x01–0x0F, 0x11–0x19, 0x1B–0x1E | The function (§6); no button sends 0x00, 0x10, 0x1A or 0x1F |
| 0x02 | 0x00 | Auto range |
| 0x02 | 1..8 | Manual range, the row of §5.4; the dialog preselects row range − 1 (VA 0xBB7C5E) |
| 0x03 | 0x5A | The Hz% key; what the meter then changes is unknown |
| 0x04 | 0x01 + float32 LE | REL against a value typed into the app |
| 0x04 | 0x00 | Leave REL |
| 0x05 | 0x01 / 0x00 | MAX MIN: enter or step / leave |
| 0x06 | 0x01 / 0x00 | Peak: enter / leave |
| 0x07 | 0x5A | HOLD |

The app gates its own buttons on the status bits (§5.3): while REL (byte 5
bit 4) is set, Max Min sends nothing; while Peak (bit 6) is set, Max Min is
blocked and Peak sends `06 00`; while MAX/MIN (bit 5) is set, Peak is
blocked and Max Min sends `05 00` once byte 6 shows MIN, else `05 01`.

The manual's REL key takes the reading on the display as its reference;
the app's command carries a typed value. A command for the SELECT key:
none found in the app's command table (2026-11-07); its function buttons
send the mode byte directly.

An earlier reading listed a table of mode-transition codes (0xD9–0xDC,
0x109–0x111) and a square-wave pseudo-mode 0x1007 with commands 0xE0/0xE1,
from `FUN_00630e0b`. That function converts field types in the app's
Midas dataset engine; none of its values is a meter command or mode
([approach](reverse-engineering-approach.md#what-misled-the-first-reading)).

### 4.8 Square-Wave and Clock Commands -- [VENDOR]

- 0x14, square-wave settings (UT171C): `AB CD 0B 00 14 <4 bytes> <4 bytes>
  chk chk`, the two floats from the app's FREQ and DUTY fields. Their order
  in the frame was not traced.
- 0x15, clock sync: `AB CD 07 00 15 <u32 LE> chk chk`. The u32 packs the
  year − 2000 in bits 0–5, month 6–9, day 10–14, hour 15–19, minute 20–25
  and second 26–31; the stored record's time uses the same packing
  (VA 0xBC166E–0xBC16B5). The manual sets the date and time on the
  UT171C only.

---

## 5. Measurement Frame and Replies -- [VENDOR]

### 5.1 Measurement Frame (Type 0x02)

`HandleMeasurement` (VA 0xBBFBF0) is the only reader of type 0x02 frames.
It reads byte n only when the frame has more than n bytes; a short frame
leaves the previous mode and range in place and the value at 0.0.

| Offset | Size | Field | What the app does | VA |
|--------|------|-------|-------------------|----|
| 0-1 | 2 | Header | `0xAB 0xCD` | §3.3 |
| 2-3 | 2 | Length | uint16 LE, payload + 2 | §3.3 |
| 4 | 1 | Type | `0x02` selects this handler | 0xBB9D9C |
| 5 | 1 | Status A | Bits (§5.3); bits 0, 1, 3 also set the layout (§5.2) | 0xBBFC2B |
| 6 | 1 | Status B | Bits (§5.3) | 0xBBFC5E |
| 7 | 1 | Mode | Function (§6); a change clears the trend chart. Modes 0x1A and 0x1D stop here and use their own layouts (§5.8, §5.9) | 0xBBFDD7–0xBBFE48 |
| 8 | 1 | Range | Row of the mode's range table, 1-based (§5.4) | 0xBC04E5 |
| 9-12 | 4 | Main value | IEEE 754 float32 LE, shown unscaled (§5.6) | 0xBBF4D8 |
| 13 | 1 | Decimals and state | High nibble decimals, low nibble display state (§5.5) | 0xBC0548 |
| 14 | 1 | Main unit | Unit code (§5.6) | 0xBC0589–0xBC05CF |
| 15.. | 0-12 | Optional blocks | Aux, bar float, STO minutes, by the bits of byte 5 (§5.2) | — |
| last 2 | 2 | Checksum | uint16 LE (§3.2) | §3.3 |

The app logs each reading (number, time, the row's bounds, the formatted
value, the unit string, and the aux value and unit when present) and skips
it when byte 6 bit 2 is set, when the display state is not 0, or when its
own skip-repeat setting is on and the text repeats (VA 0xBC0D5D–0xBC111D).

This replaces the reading of byte 6 as a frame type (0x01 standard, 0x03
extended), of byte 13 as 0x40 DC / 0x20 AC, and of byte 14 as an unknown
0x00/0x01: byte 6 is status bits, byte 13 the decimal count (0x40 is four
decimals, 0x20 two, both with state 0), and byte 14 the unit code
(0x00 VDC, 0x01 VAC).

### 5.2 Optional Blocks: Aux, Bar and STO Minutes

After byte 14 the blocks follow in a fixed order, each present only when
its bit of byte 5 is set:

| Block | Present when | Offset | Content | VA |
|-------|--------------|--------|---------|----|
| Aux | Byte 5 bit 0 | 15–20 | Float32 LE at 15–18; byte 19 decimals and state, as byte 13 (§5.5); byte 20 the aux unit code (§5.6) | 0xBC05D9–0xBC06A7 |
| Bar | Byte 5 bit 3 | 15–18, or 21–24 after the aux | Float32 LE, drawn on the linear gauge after rounding to 4 decimals; no unit, never logged | 0xBC0980, 0xBC09AB–0xBC09C7 |
| STO minutes | Byte 5 bit 1 | k, k+1 with k = 15 + 6 (aux) + 4 (bar) | Big-endian u16, minutes, shown as "H 00:mm", "H 01:00" or "H h: m" | 0xBC0B7B–0xBC0C64 |

So the payload is 11, 13, 15, 17, 19, 21 or 23 bytes, by which blocks are
present (§3.4). The app reads the aux only when the frame has more than 20
bytes, and the bar and the minutes with no length check; it never compares
the length with the flags.

The manual's auxiliary display carries what these blocks would: the Hz,
duty or period that the Hz% key cycles on V~, I~ and Hz/% (with AC and DC
components on AC+DC), REL's reference, the MAX/AVG/MIN word, the recall
number, and "H XX:XX timing value" (LCD item 31). Its bar graph is the
"simulated pointer" of 21 (UT171A) or 31 (UT171B/C) segments. Which modes
set the aux and bar bits, and which aux unit each sends, is unknown.

The earlier readings of this layout are replaced: the 27-byte frame's
bytes 19–20 are the aux state and unit, not extra flags, and its third
float is the bar value, not an AC+DC reading.

### 5.3 Status Bits (Bytes 5 and 6)

The app turns byte 5 into an array S[0..7] with S[i] = bit i (VA 0xBA9584),
and byte 6 into S[8..15] with S[8+i] = bit i (VA 0xBA9614). Every reader
below comes from a scan of all 35 loads of that array's pointer in `.text`
(2026-11-07).

| Byte | Bit | Mask | App reading | Readers (VA) |
|------|-----|------|-------------|--------------|
| 5 | 0 | 0x01 | Aux block present (§5.2) | 0xBC05D9, 0xBC09B5, 0xBC0B90, 0xBC0FEB; record 0xBC1A78, 0xBC1EB9, 0xBC228C |
| 5 | 1 | 0x02 | Auto-save running: lights `STO`; STO minutes present (§5.2); Read ALL refused | 0xBBFC7E, 0xBC0B7B, 0xBC174E, 0xBB8BBD |
| 5 | 2 | 0x04 | No reader; meaning [UNVERIFIED] (§10 Cross-Reference) | none |
| 5 | 3 | 0x08 | Bar float present: shows the gauge (§5.2) | 0xBC0980, 0xBC0B9E, 0xBC1E75 |
| 5 | 4 | 0x10 | REL: lights `△` | 0xBBFC96, 0xBC1769; Max Min 0xBB71C7 |
| 5 | 5 | 0x20 | MAX/MIN mode running; drives no label | Peak 0xBB7103; Max Min 0xBB71F4 |
| 5 | 6 | 0x40 | Peak: lights `Peak` | 0xBBFCAE, 0xBC1784; Peak 0xBB7120; Max Min 0xBB71D7 |
| 5 | 7 | 0x80 | HOLD: lights ` H ` | 0xBBFCC6, 0xBC179F |
| 6 | 0 | 0x01 | AUTO range: lights `Auto` when set | 0xBBFCDE, 0xBC17BA |
| 6 | 1 | 0x02 | Shows a lightning-bolt image, the manual's "Alarm for high voltage" (LCD item 7) | 0xBBFCF6, 0xBC17D5 |
| 6 | 2 | 0x04 | The reading is not logged; meaning unknown | 0xBC0D62 |
| 6 | 3, 4, 7 | 0x18, 0x80 | No reader | none |
| 6 | 5, 6 | 0x60 | Aggregate: bit 6 alone MAX, bit 5 alone AVG, both MIN, neither hidden | 0xBBFD12, 0xBBFD28, 0xBC17F4, 0xBC180A |

The manual's MAX MIN key steps MAX → AVG → MIN, and its Peak (long press
of the same key, AC V and AC I only) shows "Peak" with P-Max and P-Min.
What bit 5 set with an aggregate of neither means is unknown. The manual's
high-voltage alarm states no threshold.

Low battery: no bit drives the app's battery label (`lblPower` is never
referenced by code). The manual shows a battery symbol below 7.3 V
(LCD item 26).

The earlier table (bit 6 an inverted AUTO, bit 1 an extended-frame marker,
bit 3 a MIN/MAX flag gated by bit 0) came from the Midas dataset code §3.3
names and is replaced by the table above.

### 5.4 Range Byte (Offset 8)

The range byte is raw (no 0x0F masking, unlike the UT61E+) and 1-based: row
n of the mode's table is range byte n, and the app's manual-range dialog
preselects row byte − 1 (VA 0xBB7C5E). AUTO is byte 6 bit 0; range byte 0
matches no row. The range picks only the chart's axis and the picker's row;
the unit comes from byte 14 (§5.6).

The range builder (VA 0xBBD330, jump table 0xBBD37A over modes 0–0x1F)
gives each mode these rows, as axis min..max / step, with the unit its row
label names:

| Mode | Label | Row 1 | Row 2 | Row 3 | Row 4 | Row 5 | Row 6 | Row 7 | Row 8 |
|------|-------|-------|-------|-------|-------|-------|-------|-------|-------|
| 0x01 | LoZV | 0..6 /1 | 0..60 /10 | 0..600 /100 | 0..1000 /200 | | | | |
| 0x02 | VDC | −6..6 /3 | −60..60 /30 | −600..600 /300 | −1000..1000 /250 | | | | |
| 0x03, 0x04 | VAC, VADC | as 0x01 | | | | | | | |
| 0x05 | mVDC | −600..600 /300 | | | | | | | |
| 0x06, 0x07 | mVAC, mVADC | 0..600 /100 | | | | | | | |
| 0x08 | Temp-C | −40..1000 /40 | | | | | | | |
| 0x09 | Temp-F | −40..1832 /40 | | | | | | | |
| 0x0A | Ω, kΩ, MΩ | Ω 0..600 /100 | kΩ 0..6 /1 | kΩ 0..60 /10 | kΩ 0..600 /100 | MΩ 0..6 /1 | MΩ 0..60 /10 | | |
| 0x0B | nf, uf, mf | nF 0..6 /1 | nF 0..60 /10 | nF 0..600 /100 | µF 0..6 /1 | µF 0..60 /10 | µF 0..600 /100 | mF 0..6 /1 | mF 0..60 /10 |
| 0x0C | BEEP | 0..600 /100 | | | | | | | |
| 0x0E | nS | 0..60 /10 | | | | | | | |
| 0x0F | Hz, kHz, MHz | Hz 0..60 | Hz 0..600 | kHz 0..6 | kHz 0..60 | kHz 0..600 | MHz 0..6 | MHz 0..60 | |
| 0x11 | uADC | −600..600 /300 | −6000..6000 /3000 | | | | | | |
| 0x12, 0x13 | uAAC, uAADC | 0..600 /100 | 0..6000 /1000 | | | | | | |
| 0x14 | mADC | −60..60 /30 | −600..600 /300 | | | | | | |
| 0x15, 0x16 | mAAC, mAADC | 0..60 /10 | 0..600 /100 | | | | | | |
| 0x17 | ADC | −6..6 /3 | −20..20 /5 | | | | | | |
| 0x18, 0x19 | AAC, (empty) | 0..6 /1 | 0..20 /5 | | | | | | |
| 0x1B | ADC600A | −60..60 /15 | −600..600 /150 | | | | | | |
| 0x1C | AAC600A | 0..60 /10 | 0..600 /100 | | | | | | |
| 0x1E | VFC | 0..600 /100 | 0..1000 /200 | | | | | | |
| 0x1F | % | 0..100 /4 for any range byte | | | | | | | |

- The labels read `<unit> Range n: <min> to <max>`, for example
  `ADC Range 1: -6 to 6` and `kΩ Range 2: 0 to 6`. Modes 0x01–0x04 count
  in V, 0x05–0x07 in mV, 0x11–0x13 in µA, 0x14–0x16 in mA, 0x17–0x19 in A,
  0x1B–0x1C in A, 0x1E in V.
- 0x08 and 0x09 label their single row "Range 1: 0 to 600", which
  disagrees with their axes. 0x0C's label names no unit. 0x1F's label is
  "Range 1: 4 to 20".
- 0x0B maps the range byte through an index table (VA 0xBBD97C) to the
  0..6, 0..60 and 0..600 steps; 0x0F repeats the same pattern (VA 0xBBDB10).
- 0x19's name is empty because its branch tests `cmp eax,0x18` a second
  time (VA 0xBBDF70) where 0x19 was meant.
- 0x0E has no range check. 0x00, 0x0D, 0x10, 0x1A and 0x1D have no rows.

Against the manual (spec tables, page renders): the ADC and AAC row 2 top
of 20 A conflicts with the manual's 10.000 A on every model, and the
frequency row 7 top of 60 MHz with its 10.000 MHz. Every row is the
60,000-count UT171B/C's; the app has no UT171A branch (no 40,000-count
logic, no "UT171A" string), while the manual's UT171A column reads
4/40/400 where the B/C read 6/60/600. Which range byte a UT171A sends is
unknown.

### 5.5 Byte 13: Decimals and Display State

VA 0xBBD1D8 splits byte 13 into two nibbles; byte 19 is split the same way
for the aux value (VA 0xBBD284).

High nibble, decimal places: 1 → `0.0`, 2 → `0.00`, 3 → `0.000`,
4 → `0.0000` (VA 0xBBC3C8); any other value leaves the format empty.

Low nibble, display state (jump table VA 0xBC0853):

| State | Main display | Aux display |
|-------|--------------|-------------|
| 0 | The value, formatted with the decimals | The aux value, formatted |
| 1 | `OL` | `OL` |
| 2 | `-OL` | `-OL` |
| 3 | Dashes by the decimal count: `----.-`, `---.--`, `--.---`, `-.----` | Dashes |
| 4 | `LEAD` | Dashes |
| 5 | `DISC` | Dashes |
| 6 | `Lo` | Dashes |
| 7 | `Hi` | Dashes |
| 8–15 | The formatted value is kept | The label is left as it was |

For any state other than 0 the app sets the value to 0.0 (VA 0xBC0942, aux
0xBC07A9) and does not log it. The manual documents `OL` (overrange, open
resistance and diode, no temperature probe); `-OL`, `LEAD`, `DISC`, `Lo`
and `Hi` are not in its LCD symbol table or its text, whose nearest words
are "DISCHARGE" on capacitance and "diod" on the auxiliary display.

### 5.6 Unit Codes and Scaling

Byte 14 (main) and byte 20 (aux) index one 32-entry table (VA 0xBBEF6C,
jump table 0xBBEF7F); the mode byte only refines 0x0C and 0x0D:

| Code | Unit | Code | Unit | Code | Unit | Code | Unit |
|------|------|------|------|------|------|------|------|
| 0x00 | VDC | 0x08 | uAADC | 0x10 | kΩ | 0x18 | mF |
| 0x01 | VAC | 0x09 | mADC | 0x11 | MΩ | 0x19 | °C |
| 0x02 | VADC | 0x0A | mAAC | 0x12 | Hz | 0x1A | °F |
| 0x03 | mVDC | 0x0B | mAADC | 0x13 | kHz | 0x1B | DIOD |
| 0x04 | mVAC | 0x0C | ADC (ADC600A in mode 0x1B) | 0x14 | MHz | 0x1C | BEEP |
| 0x05 | mVADC | 0x0D | AAC (AAC600A in mode 0x1C) | 0x15 | % | 0x1D | nS |
| 0x06 | uADC | 0x0E | AADC | 0x16 | nF | 0x1E | NCV |
| 0x07 | uAAC | 0x0F | Ω | 0x17 | uF | 0x1F | ms |

This string is the logged unit. The screen labels come from a second
switch on the same code (VA 0xBBC8CD, codes 0–0x1D): unit and coupling
pairs such as "V"/"DC" or "mA"/"AC+DC", "KΩ" for 0x10, the buzzer glyph
for 0x1C, "V"/"DIODE" for 0x1B. Codes 0x1E and 0x1F leave the labels as
they were; a code above 0x1F gives an empty string.

No scaling: the handler (VA 0xBBFBF0–0xBC118F) has no multiply or divide,
and the float is stored as read (VA 0xBC051B), formatted with byte 13's
decimals and plotted against the row's axis. The only arithmetic on any
value is the bar float's rounding. So the app expects each float already
in the unit its code names: kΩ, µF, kHz and so on. The app has no table
linking a range to a unit code; which code a meter sends on each range is
unknown. The Ω rows (kΩ on 2–4, MΩ on 5–6) are what such codes would
match.

### 5.7 Out-of-Table Bytes

What the app does with bytes its tables lack; it shows no message and
reports nothing on any of these paths:

- Range byte 0, or past a mode's rows: no row is chosen. The axis keeps
  the builder's defaults, 0..6 (VA 0xBBD350–0xBBD358), the step keeps its
  previous value, and the picker selects nothing (VA 0xBB7C4B–0xBB7C5C).
  The value and unit are unaffected, and the float widens the axis as it
  plots (VA 0xBC10A2–0xBC10F9).
- Mode above 0x1F: the chart switch is skipped (VA 0xBBD36A).
- Unit code above 0x1D: the screen label is left as it was (VA 0xBBC8D3);
  above 0x1F the unit string is empty (VA 0xBBEF6F).
- Display state 8–15: §5.5.

### 5.8 NCV (Mode 0x1A)

- Live frame: on mode 0x1A the app hides every label of its display
  panel, disables the ribbon's function items, hides the bar gauge and
  returns (VA 0xBBFE4E). It reads nothing after byte 7. What a meter sends after the
  mode, and how long the frame is, is unknown.
- Stored record (§5.10): mode 0x1A shows `EF` with the unit `NCV`
  (VA 0xBC18F6–0xBC1940).
- Manual [KNOWN]: at or below a critical voltage V1 the main display shows
  "EF"; above it, one to five dash segments by steps of a section voltage
  Vd, with a buzzer tempo and the red light. It shows no number. V1 and Vd
  are not stated.

No function button sends 0x1A: NCV is a dial position.

### 5.9 Square-Wave Output (Mode 0x1D)

Mode 0x1D has its own layout (VA 0xBBFF27–0xBC0470), read with no length
check:

| Offset | Field | Shown as |
|--------|-------|----------|
| 8 | Not read | — |
| 9-12 | Float32 LE, frequency | `Freq:` … `Hz`, format `0.0` |
| 13-16 | Float32 LE, duty | `Duty:` … `%`, format `0.0` |
| 17-20 | Float32 LE, pulse width | `Width:` … `mS` |
| 21 | Decimal count of the width | `%.<n>f` |

So the payload is at least 18 bytes. The app logs the three values and
draws a pulse waveform (VA 0xBBC4B0). It calls the function `PLUSE_O`, and
its button sends `01 1D`.

The manual [KNOWN] has square-wave output on the UT171C only: 0.5–4800 Hz
in 0.1 Hz steps, duty 0–100 % in 0.1 % steps (its operation text says
1 %–100 %), about 0.8 Vp, RANGE and MAX MIN choose the frequency and REL
and Hz% the duty. It describes no pulse width and does not say which value
each display shows. Its key text enters the mode by a long SELECT at the µA
position, while its Fig. 13 shows a dedicated output position.

### 5.10 Stored Record (Type 0x03)

The record handler (VA 0xBC159C) reads the live layout 4 bytes later, after
a timestamp:

| Offset | Field |
|--------|-------|
| 4 | Type `0x03` |
| 5-8 | u32 LE time, packed as the clock command's (§4.8) |
| 9, 10 | Status bytes, the bits of §5.3 |
| 11 | Mode |
| 12 | Range |
| 13-16 | Float32 LE value |
| 17 | Decimals and state (§5.5) |
| 18 | Unit code (§5.6) |
| 19-22 | Aux float32 LE |
| 23 | Aux decimals and state |
| 24 | Aux unit code |

Mode 0x1A shows `EF` / `NCV` (§5.8); mode 0x1F shows `Error` (VA 0xBC1948).
The record handler also reads the aux and bar bits (§5.3); where it takes
the bar float and whether a record carries STO minutes was not traced.

### 5.11 Replies (Types 0x01 and 0x72)

Type 0x01 (VA 0xBBF9E0): the app compares the whole frame with three
constants.

| Frame | Payload | App message |
|-------|---------|-------------|
| `AB CD 05 00 01 4F 4B A0 00` | `01 'O' 'K'` | "Instruct Send Ok" |
| `AB CD 05 00 01 45 52 9D 00` | `01 'E' 'R'` | "Instruct Send Fail" |
| `AB CD 05 00 01 4E 4F A3 00` | `01 'N' 'O'` | "Instruct Send Error" |

Type 0x72 (VA 0xBC2800), with a sub-code in byte 5:

| Sub-code | Content |
|----------|---------|
| 0x11 | Record count, big-endian u16 in bytes 6–7 |
| 0x12 | Memory state in byte 6: 0 "Free", 1 "Auto Save", 2 "Local Read", 3 "Format", 4 "Memory Failure" |
| 0x16 | ASCII model text in bytes 6–16, ending at a 00, and an ID as u32 LE in bytes 17–20; shown, never compared |

The count is big-endian, unlike the frame's other multi-byte fields.

---

## 6. Mode Byte Table -- [VENDOR]

The app's own evidence agrees on one map: the function command each button
sends (`01 MM`, §4.7), its caption or handler name, the ribbon group, the
range rows (§5.4) and the record handler (§5.10).

| Byte | Mode | Evidence in the app |
|------|------|---------------------|
| 0x01 | LoZ V~ | Button "LoZV" → `01 01`; LoZ label shown (VA 0xBBCFEF); rows "LoZV" |
| 0x02 | V DC | Button "VDC" → `01 02`; rows "VDC" |
| 0x03 | V AC | Button "VAC" → `01 03`; rows "VAC" |
| 0x04 | V AC+DC | Button "VAC+DC" → `01 04`; rows "VADC" |
| 0x05 | mV DC | Button "mVDC" → `01 05`; rows "mVDC" |
| 0x06 | mV AC | Button "mVAC" → `01 06`; rows "mVAC" |
| 0x07 | mV AC+DC | Button "mVAC+DC" → `01 07`; rows "mVADC" |
| 0x08 | Temperature °C | Handler `DP4_btnTempC` → `01 08`; rows "Temp-C"; mV ribbon group |
| 0x09 | Temperature °F | Handler `DP4_btnTempF` → `01 09`; rows "Temp-F"; mV ribbon group |
| 0x0A | Resistance | Button "OHM" → `01 0A`; Ω/kΩ/MΩ rows |
| 0x0B | Capacitance | Button `DP5_BtnDisc`, caption "CAP" → `01 0B`; nF/µF/mF rows |
| 0x0C | Continuity | Button "BEEP" → `01 0C`; rows "BEEP" |
| 0x0D | Diode | Button "DIOD" → `01 0D`; no rows |
| 0x0E | Conductance (nS) | Button "nS" → `01 0E`; rows "nS" |
| 0x0F | Frequency | Button "Hz%" → `01 0F`; Hz/kHz/MHz rows |
| 0x11–0x13 | µA DC, AC, AC+DC | Buttons "uADC", "uAAC", "uAAC+DC" → `01 11`–`01 13` |
| 0x14–0x16 | mA DC, AC, AC+DC | Buttons "mADC", "mAAC", "mAAC+DC" → `01 14`–`01 16` |
| 0x17–0x19 | A DC, AC, AC+DC | Buttons "ADC", "AAC", "AAC+DC" → `01 17`–`01 19` |
| 0x1A | NCV | No button; live frame blanks the display, record shows `EF` / `NCV` (§5.8) |
| 0x1B | 600 A DC clamp | 600A bar "ADC" → `01 1B`; rows and unit "ADC600A"; clamp image shown |
| 0x1C | 600 A AC clamp | 600A bar "AAC" → `01 1C`; rows and unit "AAC600A" |
| 0x1D | Square-wave output | Button "PLUSE_O" → `01 1D`; its own layout (§5.9) |
| 0x1E | VFC | Button "VFC" → `01 1E`; VFC label shown (VA 0xBBD00B); rows "VFC" |
| 0x1F | % (4-20 mA) | No button; rows "%" "4 to 20"; record shows `Error` |

Duty cycle has no mode byte: the app sends the Hz% key (`03 5A`) and has a
unit code for % (0x15). The bytes 0x00 (no button, no rows), 0x10 and 0x24
appear nowhere in the app's measurement, range, unit, label or command
code, and its range and unit tables stop at 0x1F.

The ribbon groups (VA 0xBBC768) match the manual's dial, where the SELECT
key or a long press reaches the grouped functions:

| Group | Modes | Ribbon caption | Manual |
|-------|-------|----------------|--------|
| 1 | 0x01 | LoZ V~ | LoZ position |
| 2 | 0x03, 0x1E | V~ | V~; VFC by long SELECT |
| 3 | 0x02, 0x04 | V | V⎓; AC+DC by SELECT |
| 4 | 0x05–0x09 | mV | mV; temperature by long SELECT (UT171B/C) |
| 5 | 0x0A–0x0E | Ω | Ω position with nS, diode, continuity, capacitance |
| 6 | 0x0F | Hz% | Hz % position |
| 7 | 0x17–0x19 | A | A position |
| 8 | 0x14–0x16 | mA | mA; % (4-20 mA) by long SELECT (UT171B/C) |
| 9 | 0x11–0x13 | uA | µA; square wave by long SELECT (UT171C) |
| 10 | 0x1B–0x1D | 600A | Clamp (UT171C) |

The square-wave button sits on the µA bar but is grouped with the clamp.
The manual gives the clamp, the square wave and the 600 A rows to the
UT171C only, and temperature, nS, % (4-20 mA), AC+DC and LoZ to the
UT171B/C.

The table this section held before mapped 26 bytes from `FUN_0064081c`,
`FUN_006405b1`, `FUN_00630c1e`, `FUN_00630d5c` and `FUN_00630dd4`. All five
are in the app's statically linked Midas/DBClient dataset engine
(0x630000–0x641000), called only from inside it, and read field-type codes,
not modes
([approach](reverse-engineering-approach.md#what-misled-the-first-reading)).
It disagreed with the map above at 0x08, 0x09, 0x0B–0x0D, 0x10, 0x1A–0x1F
and 0x24.

---

## 7. Data Logging Protocol -- [VENDOR]

The meter keeps records it saves on a STORE press or on a timer
(the manual's auto storage), and shows them in its recall mode. The app
drives the same memory with these commands; none has been sent to a meter.

### 7.1 Command Structure

| Command | Parameter | Reply | Purpose |
|---------|-----------|-------|---------|
| 0x0B | `5A` | 0x01 reply | Save the current reading |
| 0x0C | u8, u16 LE | 0x01 reply | Start auto-save (§4.4) |
| 0x0D | `5A` | 0x01 reply | Stop auto-save |
| 0x0E | u16 LE index, from 1 | Type 0x03 record (§5.10) | Read one record |
| 0x0F | u16 LE index | 0x01 reply | Delete one record |
| 0x10 | `34 12` | 0x01 reply | Delete every record |
| 0x11 | `5A` | Type 0x72 / 0x11 | Record count |
| 0x12 | `5A` | Type 0x72 / 0x12 | Memory state |

Which commands answer with a 0x01 reply is unknown; the column names what
the app's 0x01 handler would show.

### 7.2 Operational Sequence

1. Connect (§4.3).
2. Query the record count (0x11); the app asks for it before any read
   ("First Query Data Count,Please!").
3. Read records by index (0x0E). The app refuses Read ALL while byte 5
   bit 1 shows auto-save running ("First Exit Auto Store Mode,Please!").
4. Or start auto-save (0x0C) and stop it (0x0D); while it runs, the frames
   carry the STO minutes (§5.2).
5. Delete one record (0x0F) or all (0x10).

### 7.3 Configuration Defaults

From `sys.ini`: `InterVal_Second=3`, `Duration_Minute=60`, `ISskipRepeat=0`.
Skip-repeat governs the app's own log (§5.1); it is not sent.

---

## 8. SLABHIDtoUART.dll's CP2110 Handling -- [VENDOR]

What UNI-T's copy of Silicon Labs' HID-to-UART library does with the bridge:

- **Open:** opens the device, checks that the part number is 0x0A, then
  enables the UART. It sets no line settings and purges nothing;
  the caller sends SetUartConfig (§2.3) and the purge itself.
- **Writes:** splits a write of up to 4096 bytes into 63-byte interrupt
  reports.
- **Reads:** fills an internal ring buffer from the interrupt reports and
  hands the caller bytes from it.
- **UART status (report 0x42):** reads the TX and RX FIFO counts big-endian
  (`CONCAT11(byte[1], byte[2])`). Whether the chip sends them that way is an
  open check in the [verification backlog](../../verification-backlog.md#usb-bridges),
  since the CP2110 serves several families.

---

## 9. Key Differences from Other UNI-T Protocols

| Aspect | UT61E+ | UT171 | UT181A | UT8803 |
|--------|--------|-------|--------|--------|
| Header | AB CD | AB CD | AB CD | AB CD |
| Length | 1 byte (payload+2) | 2 bytes LE (payload+2) | 2 bytes LE (payload+2) | 1 byte |
| Comm model | Polled | Streaming | Streaming | Streaming |
| Values | 7x ASCII | LE float32 + unit code byte | LE float32 | 5 raw bytes |
| Mode encoding | 1 byte (0x00-0x19) | 1 byte (0x01-0x1F) | 2 bytes LE (0x1111-0xA231) | UCI functional code |
| Checksum | 16-bit BE | 16-bit LE | 16-bit LE | Alternating-byte |
| Range byte | 0x30 prefix, mask & 0x0F | Raw (1-based row) | Raw (0x00-0x08) | 0x30 prefix |
| AUTO flag | Inverted (clear=ON) | Byte 6 bit 0 (set=ON) | misc2 bit 0 | Inverted |
| Data logging | None | On-meter records (§7) | 20,000 records | None |
| USB library | Raw HID | SLABHIDtoUART.dll | cp211x_uart crate | uci.dll |

---

## 10. Confidence Summary

### Confirmed ([KNOWN] or [VENDOR])

No row is confirmed on hardware: no UT171 has been read (issue #4).

| Finding | Level |
|---------|-------|
| Frame header 0xABCD, 2-byte LE length (payload+2), 16-bit LE checksum, checked by the app (§3.3) | [VENDOR] |
| Every command on the §3.1 framing; the 1-byte builder uncalled (§4.1) | [VENDOR] |
| Command table, 0x01–0x16 (§4.2) | [VENDOR] |
| Measurement layout: mode 7, range 8, float 9–12, byte 13, unit 14 (§5.1) | [VENDOR] |
| Aux, bar and STO blocks set by byte 5 bits 0, 3, 1 (§5.2) | [VENDOR] |
| Byte 13: decimals and display state (§5.5) | [VENDOR] |
| Unit codes 0x00–0x1F; no scaling (§5.6) | [VENDOR] |
| Status bits of bytes 5 and 6, except byte 5 bit 2 (§5.3) | [VENDOR] |
| Range rows per mode, 1-based (§5.4) | [VENDOR] |
| Mode map 0x01–0x0F, 0x11–0x1F (§6) | [VENDOR] |
| Stored record and replies 0x01, 0x72 (§5.10, §5.11) | [VENDOR] |
| Complete CP2110 feature report map | [VENDOR] |
| UART config report 0x50 byte layout | [VENDOR] |
| 9600/8N1, VID 0x10C4, PID 0xEA80 | [VENDOR] |
| All measurement ranges and accuracy | [KNOWN] |
| NCV display: EF, then 1–5 dash segments (§5.8) | [KNOWN] |

### Remaining Gaps ([UNVERIFIED])

The open checks are in [verification.md](verification.md).

### Cross-Reference with Community Sources

[gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) (GPL-3.0), a
Python parser whose README names the UT171A, read 2026-11-06 after the
vendor reading. Its tests hold two frames as hex strings; where they came
from is not stated, and none of its code says its tables came from a
meter. Compared 2026-11-07:

| Finding | This spec | gulux | Agreement |
|---------|-----------|-------|:---------:|
| Frame header AB CD, LE16 length, LE16 checksum over bytes 2 onward | §3.3 | Same | ✓ |
| Main value: LE float32 at 9–12 | §5.1 | Same | ✓ |
| Frame layout: byte 13 decimals and state, byte 14 unit code, blocks set by byte 5 | §5.1, §5.2 | Reads neither byte 13 nor 14; its two test frames decode cleanly under this layout, checksums included: a V DC reading with a bar value, and a V AC reading with a frequency aux in kHz and a bar value | ✓ |
| Mode bytes 0x02, 0x03, 0x05, 0x06, 0x0A, 0x0F, 0x11, 0x12, 0x14, 0x15, 0x17, 0x18 | §6 | Same functions | ✓ |
| Mode byte 0x0B | Capacitance | Diode | ✗ |
| HOLD: byte 5 bit 7 | §5.3 | Same | ✓ |
| Byte 5 bit 2 | No reader in the app | Low battery | gulux only |
| Aux float at 15–18 | Present when byte 5 bit 0 is set; unit from byte 20 | Read whenever the frame is 21 bytes or more; shown only on V AC and mV AC, labelled kHz | partial |
| Resistance | Float in the unit byte 14 names; rows kΩ on 2–4, MΩ on 5–6 | Scales by 1000 from range 2 and by 10⁶ from range 5 | consistent if the meter sends kΩ and MΩ codes on those rows |
| Float at 21–24 of a 27-byte frame | Bar value (§5.2) | Stored, never shown | — |
| Type 0x02 with length bytes 0x04–0x06 and 0x08 | No such frame in the app; its replies are types 0x01 and 0x72 (§5.11) | Its frame filter accepts them | gulux only |

The earlier versions of this spec called gulux's parser capture-driven and
counted its 13 mode bytes as "USB confirmed"; the 2026-11-06 read found no
capture in it.
