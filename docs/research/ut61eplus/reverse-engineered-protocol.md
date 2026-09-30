# UT61E+ Protocol: Reverse-Engineered Specification

Based on:
- UT61E+ User Manual (UNI-T)
- CP2110 Datasheet (Silicon Labs)
- AN434: CP2110/4 Interface Specification (Silicon Labs)
- UNI-T UT61E+ Software V2.02 (decompiled with Ghidra)
- UNI-T protocol deck "UT61+系列通讯协议" (UT161/UT61+/UT202S), published on
  the UT61E+ product page of meters.uni-trend.com.cn — see
  `docs/research/ut61-family/reverse-engineering-approach.md`
- UNI-T's iDMM2.0 Android app, decompiled with jadx, for command 0x5D
  (same approach doc)

Confidence levels:
- **[KNOWN]** — established facts from official Silicon Labs documentation
- **[VENDOR]** — confirmed by decompiling UNI-T's official Windows software
- **[VENDOR-DOC]** — stated in UNI-T's published protocol deck
- **[MANUAL]** — stated in UNI-T's official user manual
- **[DEDUCED]** — logical inferences not yet verified against real hardware
- **[UNVERIFIED]** — requires real device testing to confirm
- **[VERIFIED]** — confirmed against a real UT61E+ device
- **[COMMUNITY]** — from a community source, §7 only; never in §1-6

---

## 1. Transport Layer: CP2110 HID Bridge

### 1.1 Device Identification — [VENDOR]

From DMM.exe initialization code (at offset 0x4A90):

| Parameter | Value | Source |
|-----------|-------|--------|
| USB VID | **0x10C4** | DMM.exe `setCp2110VID(0x10C4)` |
| USB PID | **0xEA80** | DMM.exe `setCp2110PID(0xEA80)` |
| USB Class | HID | CP2110 datasheet |
| TX/RX FIFO | 480 bytes each | CP2110 datasheet |

UNI-T kept the Silicon Labs default VID/PID.

### 1.2 HID Report Structure — [KNOWN]

From AN434:

**UART Data Transfer (Interrupt Transfers):**
- Report IDs 0x01 through 0x3F carry UART data
- The report ID encodes the byte count (1-63 data bytes)
- Byte index 0 = Report ID, bytes 1-63 = UART data

**Device Configuration (Control Transfers / Feature Reports):**
- Report ID 0x41: Get/Set UART Enable
- Report ID 0x42: Get UART Status
- Report ID 0x43: Set Purge FIFOs
- Report ID 0x46: Get Version Information
- Report ID 0x50: Get/Set UART Config

### 1.3 UART Configuration — [VENDOR]

From CP2110.dll constructor (at 0x10001100):

| Parameter | Value | Evidence |
|-----------|-------|----------|
| Baud rate | **9600 bps** | `this+0x44 = 0x2580` (CP2110.dll); `setBaudrate(0x2580)` (DMM.exe) |
| Data bits | **8** | `this+0x48 = 3` (0x03 = 8 bits per AN434) |
| Parity | None (0x00) | Default (no override found in code) |
| Stop bits | 1 / Short (0x00) | Default (no override found in code) |
| Flow control | None (0x00) | Default (no override found in code) |
| Read timeout | 100 ms | DMM.exe `setReadTimeout(0x64)` |
| Write timeout | 100 ms | DMM.exe `setWriteTimeout(0x64)` |

**[VENDOR-DOC]** The protocol deck states the same line settings: 9600 baud,
no parity, 1 start bit, 8 data bits, 1 stop bit.

### 1.4 Initialization Sequence — [KNOWN + VENDOR]

The CP2110.dll dynamically loads `SLABHIDtoUART.dll` and resolves these
function pointers:

1. `HidUart_Open` — open device by VID/PID
2. `HidUart_SetUartEnable(1)` — enable UART
3. `HidUart_SetUartConfig(9600, 0, 0, 3, 0)` — 9600/8N1/no flow control
4. `HidUart_FlushBuffers` — clear FIFOs
5. `HidUart_SetTimeouts(100, 100)` — 100ms read/write timeout

The same operations as raw HID feature reports, without the
SLABHIDtoUART wrapper:

1. **Enable UART:** `[0x41, 0x01]`
2. **Configure 9600/8N1:** `[0x50, 0x00, 0x00, 0x25, 0x80, 0x00, 0x00, 0x03, 0x00]`
   - Bytes 1-4: baud rate = `0x00002580` = 9600 (big-endian)
   - Byte 5: `0x00` = no parity
   - Byte 6: `0x00` = no flow control
   - Byte 7: `0x03` = 8 data bits
   - Byte 8: `0x00` = short stop bit (1 stop bit)
3. **Purge RX FIFO:** `[0x43, 0x02]` (0x01=TX, 0x02=RX, 0x03=both — RX only
   since TX is empty at init)

**[VERIFIED]** Our UT61E+'s CP2110 takes the 0x50 report at exactly these
nine bytes; the meter answers normally after it.

### 1.5 CP2110 Diagnostic Reports — [KNOWN]

These are CP2110 HID feature reports (not meter protocol), documented in
AN434. They're useful for troubleshooting the UART bridge itself.

**Get Version Information (report 0x46)** — Get (device → host), 2 data bytes:

| Offset | Size | Description |
|--------|------|-------------|
| 1 | 1 | Part number (0x0A for CP2110) |
| 2 | 1 | Device firmware version |

**Get UART Status (report 0x42)** — Get (device → host), 6 data bytes:

| Offset | Size | Description |
|--------|------|-------------|
| 1-2 | 2 | TX FIFO byte count (LE, max 480) |
| 3-4 | 2 | RX FIFO byte count (LE, max 480) |
| 5 | 1 | Error status (bit 0 = parity, bit 1 = overrun) |
| 6 | 1 | Break status (0x00 = inactive, 0x01 = active) |

Reading this report clears the error flags. Useful for detecting overrun
errors that would otherwise only manifest as checksum failures in the meter
protocol.

**[VERIFIED]** on our UT61E+'s CP2110: report 0x46 answers part number 0x0A,
firmware version 1, and report 0x42 reads both FIFOs empty and no errors
while the link is idle.

**Set Reset Device (report 0x40)** — Set (host → device), payload `[0x40, 0x00]`.
Resets the CP2110 and re-enumerates on USB. All UART config is lost — must
re-initialize after re-opening.

**[VERIFIED] UT61E+ quirk:** Report 0x40 is rejected with a HID protocol
error on the UT61E+'s CP2110. UNI-T likely locked this report out in the
device's HID descriptor.

### 1.6 CH9329 Alternate Transport — [VENDOR + DEDUCED; VERIFIED on a UT181A and a UT61B+]

Some UT-D09 cables (sold by UNI-T for UT181A, UT171 series, UT243) use a
WCH CH9329 instead of a CP2110. Vendor software includes `CH9329DLL.dll`
for this bridge. The meter-facing UART protocol bytes are identical — only
the HID report framing differs.

| Parameter | Value |
|-----------|-------|
| USB VID | **0x1A86** |
| USB PID | **0xE429** |
| Baud rate | 9600 (configured at the chip level) |
| Host-side UART setup | None required |
| Driver | None — driverless HID on all platforms |
| HID report size | 65 bytes |
| Byte 0 | Report ID (`0x00`) |
| Byte 1 | UART data length |
| Bytes 2-64 | UART payload |

Initialization requires no feature reports — the chip is ready for data
transfer as soon as the HID device is opened.

**[VERIFIED on a UT181A and a UT61B+]** The HID framing above was deduced
from the CH9329 datasheet and the vendor `CH9329DLL.dll` filename, then
confirmed on a real UT181A by two reporters (issue #5, 2026-04-07 and
2026-09-02): the meter streams over it and honours the host's start
command, so the UART bytes match the CP2110 path. Three UT61B+ captures by
@ChrisTheExpie (issue #19, 2026-09-09, 2026-09-10 and 2026-09-11) confirm
the same for this family — the second recorded the Get Name handshake itself
over the CH9329: `AB CD 03 5F 01 DA` out, `AB CD 04 FF 00 02 7B` and the name
frame back.

---

## 2. Application Protocol — [VENDOR]

Confirmed by decompiling `CustomDmm.dll` (the protocol plugin DLL).

### 2.1 Message Framing

From `FUN_10002460` (frame builder) and `FUN_10002540` (frame parser):

```
+------+------+--------+------------------+----------+----------+
| 0xAB | 0xCD | length | payload          | chk_high | chk_low  |
+------+------+--------+------------------+----------+----------+
  1 byte 1 byte 1 byte   variable           1 byte     1 byte
```

- **Header**: Fixed 2-byte sequence `0xAB 0xCD`
- **Length**: Number of bytes following (payload + 2 checksum bytes)
- **Payload**: Command or response data
- **Checksum**: 16-bit big-endian sum of all preceding bytes (header +
  length + payload)

**Frame builder pseudocode** (from decompilation):
```
frame = [0xAB, 0xCD]
frame.append(len(payload) + 2)   // length = payload + 2 checksum bytes
frame.extend(payload)
checksum = sum(frame) & 0xFFFF   // 16-bit sum of all bytes so far
frame.append(checksum >> 8)      // high byte
frame.append(checksum & 0xFF)    // low byte
```

**Frame parser pseudocode** (from decompilation):
```
if buf[0] != 0xAB or buf[1] != 0xCD: discard and clear buffer
length = buf[2]
total_frame_size = length + 3        // header(2) + length_byte(1) + length
checksum_offset = total_frame_size - 2
computed = sum(buf[0:checksum_offset]) & 0xFFFF
received = (buf[checksum_offset] << 8) | buf[checksum_offset + 1]
if computed != received: reject frame
```

**[VENDOR-DOC]** The deck describes the same frame in both directions:
header `AB CD`, a length byte counting from the command (or first data
byte) through the checksum, and a two-byte checksum summing everything
from the header up. It does not give the checksum's byte order; the
big-endian order above is the one every real frame checks out with.

### 2.2 Request/Response Model — [VENDOR]

The software uses a **polled** model. From `FUN_100016d0` (MyDmm
constructor):

- A `LoopCommandPool` continuously sends **GetMeasurement** commands
  (command byte 0x5E) on a timer
- An `OnceCommandPool` sends one-shot commands (Hold, Range, etc.)
- Default polling interval: 1000 ms (can be configured in `options.xml`
  via `SampleRate`)

The deck words 0x5E and 0x5F as commands that "enable" the meter to send
its display value and its model name. "Enable" means "send one": over the
CP2110 each 0x5E brings back exactly one measurement frame (§2.9), and the
one command that does start readings coming unasked — 0x5D, which the deck
omits — is acked and changes nothing on the cable (§2.3).

### 2.3 Command Format (Host → Meter) — [VENDOR]

Commands use the standard framing with a single-byte payload:

```
AB CD 03 <cmd> <chk_hi> <chk_lo>
```

The length byte is always 0x03 (1 byte command + 2 bytes checksum).

**Confirmed commands** (from CustomDmm.dll and DMM.exe UI):

| Command Byte | ASCII | Name | Evidence |
|-------------|-------|------|----------|
| 0x5E | `^` | GetMeasurement | `QByteArray::append('^')` in constructor |
| 0x4A | `J` | Hold | `QByteArray::append('J')` in FUN_10002170 |
| 0x46 | `F` | Range | `QByteArray::append('F')` in FUN_100021f0 — see note below |

**Note on 0x46 (Range)** — what a press does on the meter, manual ranging on
the rung showing and then one rung up per press with the mode byte never
moving, is **[VERIFIED]** in the family spec §6.1.

**Further commands** — first inferred from DMM.exe UI action names (not
seen in decompiled code; the DMM.exe decompilation was incomplete), now
**[VENDOR-DOC]**: every byte below, and the three above, is in the
protocol deck's command table (命令表一) with the same meaning.

| Byte | Name | UI Action | Hardware Status |
|--------------|------|-----------|-----------------|
| 0x41 | MinMax toggle | `actionMaxMin` | **[VERIFIED]** (remote) |
| 0x42 | ExitMinMax | `actionExitMaxMin` | **[VERIFIED]** (remote) |
| 0x47 | Auto | `actionRangeAuto` | **[VERIFIED]** (restores auto-range) |
| 0x48 | Rel | `actionRel` | **[VERIFIED]** (remote) |
| 0x49 | Select2 (Hz/USB) | `actionHz` | **[VERIFIED]** (AC mV: cycles mV → Hz → Duty% → mV; no effect on DC V) — rings per dial position: `docs/research/ut61-family/reverse-engineered-protocol.md` §3.1 |
| 0x4B | Light | `actionLight` | **[VERIFIED]** (backlight toggle) |
| 0x4C | Select (orange) | `actionSelect` | **[VERIFIED]** (cycles sub-modes, e.g. DC V → AC+DC V) — rings per dial position: `docs/research/ut61-family/reverse-engineered-protocol.md` §3.1 |
| 0x4D | PeakMinMax | `actionPeak` | **[VERIFIED]** (AC modes only; beeps but no visible effect on DC V) |
| 0x4E | ExitPeak | `actionExitPeak` | **[VERIFIED]** (clears peak flags, returns to live) |
| 0x5F | GetName | (device discovery) | **[VERIFIED]** — two-frame response (FF 00 ack, then ASCII name) |

Hardware verification: commands issued against a real UT61E+ via `dmm-cli`
command tools; effects observed on the meter LCD and subsequent response
frames.

**Command 0x5D** — `AB CD 03 5D 01 D8`, in neither the deck nor V2.02.
UNI-T's own Android client writes it once to start reading and then only
listens (the decompiled app, `../ut61-family/reverse-engineering-approach.md`;
§7 has the community's names for it). What it does depends on the link, both
tested on our UT61E+ on 2026-09-22:

| Link | What follows the command | Status |
|------|--------------------------|--------|
| CP2110 cable | the `FF 00` ack and nothing else; readings still take one 0x5E each | **[VERIFIED]** |
| UT-D07B adapter | no ack ever seen, then readings unasked from about 1-2 s on, about 3.2 a second | **[VERIFIED]** |

The pair puts the command at the adapter rather than at the meter: the
adapter polls the meter itself once told to, and what the meter does with
0x5D is what the cable shows
(`../ut-d07b/reverse-engineered-protocol.md` §3, §5).

**Clamp-meter commands — [VENDOR-DOC]**, in the same table and for
features no UT61+ model has:

| Byte | Deck name |
|------|-----------|
| 0x43 | INRUSH (enter inrush current test) |
| 0x44 | Exit INRUSH |
| 0x45 | ZERO (DC current zeroing) |
| 0x4F | Flight (flashlight) |

The deck says nothing of the `FF 00` ack the meter sends after each command
(family spec §6.1); that is from our captures only.

**Checksum formula for commands**: Since length is always 0x03 and
payload is one byte, the checksum for command `cmd` is:
```
checksum = 0xAB + 0xCD + 0x03 + cmd = cmd + 0x17B
```

Example: GetMeasurement (0x5E): checksum = 0x5E + 0x17B = 0x1D9 →
frame = `AB CD 03 5E 01 D9`

### 2.4 Measurement Response Format — [VENDOR]

From `FUN_10007d50` (response parser). The measurement response has
length byte 0x10 (16), making the total frame 19 bytes:

```
AB CD 10 <mode> <range> <display×7> <bar×2> <flags×3> <chk_hi> <chk_lo>
```

**[VERIFIED]** Every measurement frame from our UT61E+ is 19 bytes, its
length byte counting the checksum.

**Byte layout** (offsets from start of frame):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | Header1 | 0xAB |
| 1 | 1 | Header2 | 0xCD |
| 2 | 1 | Length | 0x10 (16) |
| 3 | 1 | Mode | Measurement mode (raw, no masking) |
| 4 | 1 | Range | Range index (see mode/range table) |
| 5-11 | 7 | Display | ASCII display value |
| 12-13 | 2 | Bar Graph | Bar graph position (raw bytes) |
| 14 | 1 | Flags1 | REL, HOLD, MIN, MAX |
| 15 | 1 | Flags2 | HV, LowBat, AUTO |
| 16 | 1 | Flags3 | bar_pol, P-MIN, P-MAX, AC/DC |
| 17-18 | 2 | Checksum | 16-bit BE sum of bytes 0-16 |

**[VENDOR-DOC]** The deck gives this layout byte for byte (its
`Msg[0]`–`Msg[18]`): length 0x10, mode ("function position") and range at
3-4, the display as ASCII at 5-11, the bar graph at 12-13, status at
14-16.

**Display value parsing** (from decompilation):
1. Extract bytes 5-11 as Latin-1 string
2. Check for "OL" → overload condition
3. Strip all spaces: `replace(" ", "")`
4. Parse as `double`
5. For modes with SI prefix (Hz, Ohm, Continuity, Cap, hFE, Live, NCV,
   LozV): multiply by the range's SI multiplier

**[VERIFIED]** The 7-char field is right-aligned with leading space
padding on the real device. Examples observed: `" 12.345"` (normal
reading), `"-12.345"` (negative value), `"    OL "` (overload).

**NCV display format** — the NCV mode (0x14) draws a level, not a number:
`"   EF  "` while no field is detected, and one `-` segment per level as
the detected field grows, so the level is the dash count.
**[VERIFIED]** `"   EF  "` (no field) and `"     - "` (level 1, meter
beeping at a mains cable) on 2026-09-07; **[MANUAL]** §13 for the further
segments, not yet observed on the E+; a UT61B+ sent four (#19, ut61-family
spec). The decompilation's `-` check for
`cVar1 == '\x14'` (§2.5) is the same display.

**Overload detection** (from `FUN_100026a0`):
- If display contains "O" AND "L" → OL (overload)
- If display also contains "-" → negative OL
- Returns: 0 = normal, 1 = negative OL, 2 = positive OL

### 2.5 Mode Byte Values — [VENDOR]

From the mode string lookup table at 0xD324 in CustomDmm.dll, and
confirmed by the mode-specific code paths in FUN_10007d50:

| Byte | Mode | Display Name | Confirmed By | UT61E+ Hardware |
|------|------|-------------|--------------|-----------------|
| 0x00 | AC Voltage | ACV | String table position | **[VERIFIED]** |
| 0x01 | AC Millivolt | ACmV | String table position | **[VERIFIED]** |
| 0x02 | DC Voltage | DCV | String table position | **[VERIFIED]** |
| 0x03 | DC Millivolt | DCmV | String table position | **[VERIFIED]** (mV dial, mode byte capture) |
| 0x04 | Frequency | FREQ | Multiplier check `cVar1 == '\x04'` | **[VERIFIED]** (V~ and mA via SELECT2) |
| 0x05 | Duty Cycle | Duty Cycle | Bar graph "-" check `cVar1 == '\x05'` | **[VERIFIED]** (mA via SELECT2) |
| 0x06 | Resistance | RES | Multiplier check `cVar1 == '\x06'` | **[VERIFIED]** |
| 0x07 | Continuity | Short-Circuit | Multiplier check `cVar1 == '\a'` (0x07) | **[VERIFIED]** |
| 0x08 | Diode | Diode | String table position | **[VERIFIED]** |
| 0x09 | Capacitance | CAP | Multiplier check `cVar1 == '\t'` (0x09) | **[VERIFIED]** |
| 0x0A | Temperature °C | Celsius | Special handling `cVar1 == '\n'` (0x0A) | — (not on UT61E+) |
| 0x0B | Temperature °F | Fahrenheit | Special handling `cVar1 == '\v'` (0x0B) | — (not on UT61E+) |
| 0x0C | DC µA | DCuA | String table position | **[VERIFIED]** |
| 0x0D | AC µA | ACuA | String table position | **[VERIFIED]** (µA + SELECT, mode byte capture) |
| 0x0E | DC mA | DCmA | String table position | **[VERIFIED]** |
| 0x0F | AC mA | ACmA | String table position | **[VERIFIED]** (mA + SELECT, mode byte capture) |
| 0x10 | DC A | DCA | String table position | **[VERIFIED]** (A⎓ dial) |
| 0x11 | AC A | ACA | String table position | **[VERIFIED]** (A⎓ + SELECT) |
| 0x12 | hFE | hFE | Multiplier check `cVar1 == '\x12'` | **[VERIFIED]** |
| 0x13 | Live (contact live/neutral wire check, [VENDOR-DOC]) | Live | Bar graph "-" check `cVar1 == '\x13'` | — (not on UT61E+) |
| 0x14 | NCV | NCV | Multiplier/"-" checks `cVar1 == '\x14'` | **[VERIFIED]** |
| 0x15 | LoZ Voltage (low-impedance AC V, [VENDOR-DOC]) | LozV | String table position | — (not on UT61E+) |
| 0x16 | Clamp AC A ([VENDOR-DOC]) | LozV | Multiplier check `cVar1 == '\x16'` | — (not on UT61E+) |
| 0x17 | Clamp DC A ([VENDOR-DOC]) | LPF | Bar graph "-" check `cVar1 == '\x17'` | — (not on UT61E+) |
| 0x18 | LPF V | | Gap in string table | **[VERIFIED]** (V~ + SELECT; no signal needed) |
| 0x19 | AC+DC V | AC+DC | AC/DC flag check `cVar1 == '\x19'` | **[VERIFIED]** (V⎓ + SELECT; no signal needed) |

Hardware-verified entries come from driving the real UT61E+ through each
physical dial position and observing the mode byte in the response frame.
The mode byte reflects the *active* measurement unit, not the dial
position — e.g. on DC V dial with auto-range, the meter reports 0x02 (DCV)
even when showing mV-scale values. The range byte determines the actual
scale.

**DC A, hFE and NCV have bytes of their own** — **[VERIFIED]** and
**[VENDOR]**: DC A (0x10), hFE (0x12) and NCV (0x14) share nothing with AC V
(0x00), DC V (0x02) or Hz (0x04).

**0x16 and 0x17 — the software and the deck disagree.** V2.02's display
names make 0x16 a second "LozV" and 0x17 "LPF". The protocol deck, whose
mode table spans the family's meters and clamps, makes them a clamp
meter's AC A and DC A, and puts LPF at 0x18, where the E+ sends LPF V
**[VERIFIED]**. No UT61+ model reaches either byte (family spec §3.1), so
no meter has confirmed either reading of 0x16/0x17.

**Mode bytes 0x1A-0x1E — [VENDOR-DOC]:** not in the vendor software's
mode string table (which ends at 0x19) and never observed from a UT61+
meter. The protocol deck names them:

| Byte | Mode (deck) | Hardware Status |
|------|-------------|-----------------|
| 0x1A | LPF, meter AC current | — (on no UT61+ dial) |
| 0x1B | AC+DC, meter current | — (on no UT61+ dial) |
| 0x1C | LPF, clamp AC current | — (clamp) |
| 0x1D | AC+DC, clamp current | — (clamp) |
| 0x1E | Inrush, clamp current | — (clamp) |

The deck gives no ranges for the current variants 0x1A/0x1B.

**Bit 7** of the mode byte marks a clamp meter's secondary-display frame,
the function in the low seven bits (family spec §2.3). No UT61+ capture
holds one.

### 2.6 Unit Prefix Table — [VENDOR]

From `FUN_10001000` (static initializer), the range byte maps to a unit
prefix through a lookup table:

| Index | Prefix | Multiplier | Example |
|-------|--------|-----------|---------|
| 0 | T (Tera) | 10^12 | |
| 1 | G (Giga) | 10^9 | |
| 2 | M (Mega) | 10^6 | 22 MΩ |
| 3 | k (kilo) | 10^3 | 2.2 kΩ |
| 4 | K (Kilo) | 10^3 | (alternate) |
| 5 | (space) | 1 | 220 V |
| 6 | (empty) | 1 | (unitless) |
| 7 | m (milli) | 10^-3 | 220 mV |
| 8 | µ (micro) | 10^-6 | 220 µA |
| 9 | n (nano) | 10^-9 | 22 nF |
| 10 | p (pico) | 10^-12 | |

**[VENDOR]** The vendor software applies **no masking** to the range byte
before looking it up in the mode/range table (`FUN_100023f0`). The table
stores entries with the packed value `(mode << 8) | range_byte`.

**The range byte has a 0x30 prefix.** From the disassembly of the
mode/range table builder (`FUN_00413f30` in DMM.exe), every range byte in
the table starts at 0x30:
- Range index 0 → byte value 0x30
- Range index 1 → byte value 0x31
- Range index 2 → byte value 0x32
- etc.

To extract the range index: `range_index = range_byte & 0x0F` (or
equivalently `range_byte - 0x30`). **[VERIFIED]** Our UT61E+ sends the range
byte with this prefix.

**Mode bytes are raw** (no prefix). Mode values 0x00-0x0A are stored
directly in the table with no transformation.

**[VENDOR]** The bar graph bytes at offsets 12-13 are NOT parsed by the
vendor software's main display function. The bar graph full-scale range
is stored in the mode/range table (e.g., ACV 220mV → 6, ACV 2.2V → 60).
The actual bar graph position data from the response bytes is unused
by the PC software.

**[VERIFIED]** Bar graph encoding (from real UT61E+ testing): bytes 12
and 13 arrive raw (no `0x30` prefix) and combine as

```
segments = byte12 * 10 + byte13
```

where `byte12` is the tens digit and `byte13` is the ones digit
(**[VENDOR-DOC]** the deck says the same).
Represents the number of lit segments on the 46-segment LCD bar graph.
The LCD has fixed markings at 0, 5, 10, 15, 20; their meaning in real
units scales with the range (e.g., on 22V range: 0=0V, 5≈5V, 20≈20V;
on 2.2V range: 0=0V, 5≈0.5V, 20≈2V).

Measured on DC V:

| Input | Range | byte12 | byte13 | Segments |
|-------|-------|--------|--------|----------|
| 0 V | 22V | 0 | 0 | 0 |
| 5 V | 22V | 0 | 9 | 9 |
| 10 V | 22V | 2 | 0 | 20 |
| 20 V | 22V | 3 | 9 | 39 |
| 1 V | 2.2V | 2 | 0 | 20 |

Consistent with `segments ≈ |value| / range_max * 46`.

For negative values the bar graph holds the *magnitude* and flag byte 16
bit 0 (`bar_pol`) is set instead. On overload (OL), the bar graph reads
44 (near full scale).

### 2.7 Flag Bytes — [VENDOR]

From FUN_10007d50 (response parser), the three flag bytes at offsets
14-16 have these bit assignments:

All three flag bytes arrive with a `0x30` high nibble and must be masked
with `& 0x0F` before bit-extraction (verified on real device).

**[VENDOR-DOC]** The deck draws each status byte as `0 0 1 1` over four
flags, and bits 0-2 of all three bytes carry the names below. It names
Flags2 bit 2 `Manu_flag` ("AUTO" shown when clear), as the table has it.

**Byte 14 (offset 0x0E) — Flags1:**

| Bit | Mask | Flag | Status | Evidence |
|-----|------|------|--------|----------|
| 0 | 0x01 | REL | **[VERIFIED]** | `bVar3 & 1` → `FUN_10008830(this, ...)` |
| 1 | 0x02 | HOLD | **[VERIFIED]** | `bVar3 >> 1 & 1` → `FUN_10008ca0(this, ...)` |
| 2 | 0x04 | MIN | **[VERIFIED]** | `(bVar3 & 4) → "MIN"` |
| 3 | 0x08 | MAX | **[VERIFIED]** | `(bVar3 & 8) → "MAX"` |

**[VERIFIED] MIN/MAX cycle:** MAX only → MIN only → MAX (2-state, bits
never both set); no AVG state is sent. When MIN or MAX is set, the
`display` field contains the *stored* extremum, not the live reading. The
AUTO flag is cleared (range locked) for the duration of MIN/MAX mode.

**Byte 15 (offset 0x0F) — Flags2:**

| Bit | Mask | Flag | Status | Evidence |
|-----|------|------|--------|----------|
| 0 | 0x01 | **HV warning** | **[VERIFIED]** | Set at 31V on DC V (manual: >30V). DmmData offset 0x3d — stored but not displayed by PC UI. |
| 1 | 0x02 | **Low battery** | **[VERIFIED]** | Intermittent on real device. DmmData offset 0x3c — passed to a UI indicator widget. |
| 2 | 0x04 | **!AUTO** (inverted) | **[VERIFIED]** | DmmData offset 0x3b. Bit CLEAR = auto-range ON. DMM.exe hides "AUTO" label when set. |
| 3 | 0x08 | **APO** (auto power-off) | **[VENDOR-DOC]** | The deck names it `APO_flag`. Stored at DmmData offset 0x3a; never read by PC UI. Never seen set (below). |

**APO while polled:** no capture has set bit 3, although the manual has APO
on by default. **[VERIFIED]** On 2026-09-19 our UT61E+, polled without a
break over the CP2110 in AC+DC V (`dmm-cli debug`), stayed on for more than
30 minutes, past the 15 the manual gives, with the bit clear and no APO
symbol on the LCD. Whether talking over USB turns APO off, each request
restarts its timer, or APO was off before polling began is [UNVERIFIED].

**AUTO flag confirmed inverted** (from DMM.exe UI code at line 2128-2131):
```c
cVar3 = getDmmDataField_0x3b(param_1);  // byte15 bit2
if (cVar3 != '\0' || mode == Continuity || mode == Diode || mode == NCV) {
    label = "";      // hide AUTO
} else {
    label = "AUTO";  // show AUTO
}
```
So bit2 SET = manual range (no AUTO label), bit2 CLEAR = auto range (show AUTO).

**Byte 16 (offset 0x10) — Flags3:**

| Bit | Mask | Flag | Status | Evidence |
|-----|------|------|--------|----------|
| 0 | 0x01 | bar_pol | **[VERIFIED]** | Set when the reading is negative; bar graph then holds the magnitude. Not in AC+DC V: there it was set on both components' frames across a 1.6 V cell either way round, the display text carrying the sign (2026-09-19), and on some open-lead frames. |
| 1 | 0x02 | P-MIN | **[VERIFIED]** | `(bVar3 & 2) → "P-MIN"` |
| 2 | 0x04 | P-MAX | **[VERIFIED]** | `(bVar3 & 4) → "P-MAX"` |
| 3 | 0x08 | AC/DC (set = AC) | **[VERIFIED]** + **[VENDOR-DOC]** | `(bVar3 & 8)`; the deck's `AC_DC flag`, "AC" when set and "DC" when clear. In AC+DC V (0x19) the meter alternates frames between the AC and DC components, as the LCD blinks AC/DC, and this bit is set on the AC one: across a 1.6 V cell (2026-09-19) the ±1.61 V frames had it clear and the 0.0000 V frames set. Clear in every DC V and AC-mode capture, so it only tells the components apart. The meter changes component on its own clock, not per request: polled as fast as it answers (about every 0.67 s in AC+DC V) the frames alternate strictly, a 1 s poll reads DC, AC, AC, DC, DC, AC, AC, DC and a 1.5 s poll runs of three or four (2026-09-26). HOLD freezes whichever component was on screen, and only that component's frames follow; under MIN/MAX both keep alternating, each with its own extreme (2026-09-26). |

**[VERIFIED] Peak cycle:** P-MAX only → P-MIN only → P-MAX (2-state,
bits never both set). When set, the `display` field contains the stored
instantaneous peak (not RMS). Peak mode is context-dependent: activates
on AC modes (e.g. AC mV) and is silently ignored on DC V — the meter
beeps to acknowledge the command but no flag bit or display change
occurs.

### 2.8 Sampling Rate — [VERIFIED]

Maximum effective sampling rate is **~10 Hz** (~100 ms per request-response
cycle). This is a hard limit of the 9600 baud firmware — tested and
confirmed that the meter does not respond at 19200 or 115200 baud.

Measured throughput (2026-03-18, CLI `--interval-ms` over 10 s):

| Configured delay | Samples/10s | Effective Hz |
|------------------|-------------|--------------|
| 0 ms (fastest) | 101 | ~10.1 |
| 100 ms | 56 | ~5.6 |
| 200 ms | ~40 | ~4.0 |
| 300 ms | 25 | ~2.5 |
| 500 ms | 18 | ~1.8 |
| 1000 ms | 9 | ~0.9 |
| 2000 ms | 5 | ~0.5 |

The configured delay adds on top of the ~100 ms wire round-trip time.

### 2.9 Implementation Quirks — [VERIFIED]

- **Byte-at-a-time delivery:** CP2110 at 9600 baud delivers response
  bytes one at a time via HID interrupt reports, so an `AB CD` frame
  spans many reports: a full measurement response takes ~19 of them.
- **Request-response only:** the meter never streams data; each reading
  requires sending the `0x5E` request command. 0x5D does not change that
  (§2.3).
- **Mode byte reflects active unit, not dial position:** on DC V dial
  with auto-range, the meter reports mode 0x02 (DCV) even when showing
  mV-scale values. The range byte determines the actual scale.
- **SELECT2 and Peak commands are context-dependent:** they beep
  (acknowledged) but only produce visible effects in specific modes
  (e.g. SELECT2 on AC V for frequency display, Peak on AC modes).
- **MIN/MAX and Peak report stored values, not live:** when MIN, MAX,
  P-MIN, or P-MAX flags are set, the display-value field contains the
  stored statistic, not the current live reading. The bar graph may
  still reflect the live signal.

---

## 3. Measurement Modes and Ranges (from Manual)

*(Section unchanged — see UT61E+ manual for complete range tables.)*

The UT61E+ has 22,000 counts maximum, 46-segment bar graph (30 Hz),
and 2-3 Hz numeric refresh rate.

---

## 4. Software Architecture — [VENDOR]

The UNI-T software (V2.02) is a Qt 5 application with plugin DLLs:

| Component | Role |
|-----------|------|
| `DMM.exe` | Qt GUI application (chart, LCD display, recording) |
| `Lib/CustomDmm.dll` | Protocol plugin: framing, parsing, commands |
| `Lib/CP2110.dll` | Transport plugin: CP2110 HID bridge via SLABHIDtoUART |
| `DeviceSelector.dll` | USB device discovery and selection |
| `SLABHIDtoUART.dll` | Silicon Labs HID UART library (runtime) |
| `SLABHIDDevice.dll` | Silicon Labs HID device library (runtime) |
| `CH9329DLL.dll` | CH9329 chip support (alternate USB bridge) |

The software supports two USB bridge chips: **CP2110** (Silicon Labs) and
**CH9329** (WCH). Both use the same application-layer protocol; only the
transport differs.

Configuration is stored in `options.xml`:
- `Model`: Device model (e.g., "UT61D+")
- `SampleRate`: Polling interval in ms (default: 1000)
- `SamplePoints`: Chart data points (default: 1000)

---

## 5. Verification Status

The open checks are in [verification.md](verification.md); what our UT61E+
has confirmed is tagged **[VERIFIED]** where §1-2 state it.

---

## 6. Summary of Confidence Levels

What is still open is in [verification.md](verification.md), not here.

| Aspect | Status | Source |
|--------|--------|--------|
| VID 0x10C4, PID 0xEA80 | **VENDOR** | DMM.exe binary |
| Baud rate 9600 8N1 | **VENDOR** | CP2110.dll + DMM.exe binary |
| Read/Write timeout 100ms | **VENDOR** | DMM.exe binary |
| Frame header AB CD | **VENDOR** | CustomDmm.dll `FUN_10002460` |
| Length byte = payload + 2 | **VENDOR** | CustomDmm.dll `FUN_10002460` |
| Checksum: 16-bit BE sum | **VENDOR** | CustomDmm.dll `FUN_10002460`/`FUN_10002540` |
| Polled request/response | **VENDOR** | CustomDmm.dll LoopCommandPool |
| GetMeasurement = 0x5E | **VENDOR** | CustomDmm.dll constructor |
| Hold = 0x4A, Range = 0x46 | **VENDOR** | CustomDmm.dll `FUN_10002170`/`FUN_100021f0` |
| Response: 19 bytes total | **VENDOR** | CustomDmm.dll `FUN_10007d50` |
| Mode at byte[3], raw | **VENDOR** | `QByteArray::at(param_1, 3)` |
| Range at byte[4] | **VENDOR** | `QByteArray::at(param_1, 4)` |
| Display at bytes[5-11], ASCII | **VENDOR** | `fromLatin1(data+5)`, `toDouble` |
| Display: strip spaces, parse float | **VENDOR** | `replace(" ","")` then `toDouble` |
| OL detection: "O"+"L" in display | **VENDOR** | `FUN_100026a0` |
| Flags1 at byte[14]: REL/HOLD/MIN/MAX | **VENDOR** | `FUN_10007d50` bit operations |
| Flags3 at byte[16]: P-MIN, P-MAX, AC/DC | **VENDOR** | `FUN_10007d50` bit operations |
| Byte16 bit3 set = AC component in AC+DC V | **VERIFIED** | 1.6 V cell, 2026-09-19; the deck's AC_DC flag |
| Mode values 0x00-0x19 | **VENDOR** | String table + code path checks |
| SI prefix table (T/G/M/k/m/µ/n/p) | **VENDOR** | `FUN_10001000` initializer |
| CP2110 HID report format | **KNOWN** | AN434 |
| UART config report format | **KNOWN** | AN434 |
| Meter modes and ranges | **KNOWN** | UT61E+ manual |
| Display: 22,000 counts | **KNOWN** | UT61E+ manual |
| Mode byte: raw, no masking | **VENDOR** | Table builder + parser code |
| Range byte: 0x30 prefix confirmed | **VENDOR** | Table builder stores 0x30+index |
| Full mode/range table with bar graph ranges | **VENDOR** | FUN_00413f30 disassembly |
| Only 3 commands in vendor software | **VENDOR** | Searched all 4 decompiled binaries |
| Command table 0x41-0x4F, 0x5E, 0x5F | **VENDOR-DOC** | Protocol deck, 命令表一 |
| Frame layout, bar graph digits, flag bits 0-2 | **VENDOR-DOC** | Protocol deck |
| Byte15 bit2 = !AUTO (inverted) | **VENDOR** | DMM.exe UI hides "AUTO" when set |
| Byte15 bit1 = UI indicator widget | **VENDOR** | DMM.exe passes to widget method |
| Byte15 bits 0,3 = stored, not displayed | **VENDOR** | DMM.exe never reads back |
| Bar graph bytes (12-13) not used by vendor | **VENDOR** | No reads in display function |
| Byte15 bit0 = HV warning | **VERIFIED** | Set at 31V on DC V (real device) |
| Byte15 bit1 = Low Battery | **VERIFIED** | Observed intermittently on real device |
| Byte15 bit3 = APO | **VENDOR-DOC** | Protocol deck; never seen set |
| Bar graph position encoding (bytes 12-13) | **VERIFIED** | `byte12*10 + byte13` decimal, real device |
| Commands 0x41/0x42/0x47/0x48/0x49/0x4B/0x4C/0x4D/0x4E | **VERIFIED** | Exercised against real UT61E+ via CLI |
| Command 0x5F (GetName) | **VERIFIED** | Not in vendor software V2.02; confirmed on real UT61E+ — two-frame response (FF 00 ack, then ASCII name) |
| Command 0x5D (start reading) | **VERIFIED** | Acked and inert over the CP2110; behind a UT-D07B the adapter, not the meter, streams (§2.3) |
| Sampling rate ~10 Hz at 9600 baud | **VERIFIED** | Measured throughput, 19200/115200 unresponsive |
| MIN/MAX and Peak 2-state cycles | **VERIFIED** | MAX → MIN → MAX, P-MAX → P-MIN → P-MAX |
| Range byte 0x30 prefix sent by meter | **VERIFIED** | Real device observation |
| CH9329 alternate transport | **VERIFIED** | A UT181A (issue #5) and a UT61B+ (issue #19) over it (§1.6) |

---

## 7. Cross-reference with Community Sources [COMMUNITY]

Read 2026-09-22, when the clean-room boundary was opened for the question
of how fast the family can be read over Bluetooth. Nothing here was merged
into §1-6. Sources, and what was deliberately left unread, are listed in
`docs/research/ut61-family/reverse-engineering-approach.md`.

**Command 0x5D — `AB CD 03 5D 01 D8`.** Absent from the protocol deck and
from Software V2.02 (§2.3), but it is what UNI-T's own Android client
sends to start reading, and two community BLE clients send it too:

- [webspiderteam/Bluetooth-DMM-For-Windows](https://github.com/webspiderteam/Bluetooth-DMM-For-Windows)
  (C#, commit `2b83d9e`) defines it as `GetData` beside `GetDeviceTypeName`
  (0x5F) and `GetBackLight` (0x4B), writes Get Name, waits 200 ms, writes
  0x5D **once** per connection and then never writes on a timer
  (`GattMonitor.cs:22-30, 48-56, 213-231`).
- [libreble/multimeter](https://github.com/libreble/multimeter)
  (TypeScript, commit `e887b0f`) names it "start streaming measurements"
  (`packages/protocol/src/framing.ts:13-24`) and records, live-confirmed on
  a **UT60BT** on 2026-06-06, that after the handshake "the meter streams
  19-byte `AB CD 10 …` measurement frames a few times a second"
  ([`docs/protocols/uni-t.md`](https://github.com/libreble/multimeter/blob/main/docs/protocols/uni-t.md)).
  The UT60BT is this family with the radio built in, so this is evidence
  about the meter's firmware, not about an adapter.

So 0x5D starting a continuous send is confirmed on a meter of the family
that carries its own radio. On our UT61E+ over the CP2110 cable it is acked
and starts nothing, and behind a UT-D07B the readings come from the adapter
(§2.3).

**Handshake order.** The same document states, live-confirmed, that the
meter "ignores GET-DATA until it has answered GET-NAME", that a blind
"wait ~200 ms then GET-DATA" loses the race and leaves the meter silent,
and that a single 0x5D can be dropped — so the client waits for the name
frame, then re-sends 0x5D up to five times with a 700 ms wait each.

**0x31-0x37.** The six further button bytes the vendor app sends (our APK
reading) appeared in **no** community source read that day. Every community
command table stopped at 0x41-0x4C: `framing.ts:16-23` and ljakob's
`_COMMANDS` (65-78, i.e. 0x41-0x4E). The 2026-09-25 read found four of them
in libreble's UT202BT client (family spec §10).

**Polled 0x5E is what every USB implementation uses**, and none of them
mentions 0x5D: [ljakob](https://github.com/ljakob/unit_ut61eplus)
(`_SEQUENCE_SEND_DATA`), [mwuertinger](https://github.com/mwuertinger/ut61ep)
(`requestData`), [mbraune/ut161b](https://github.com/mbraune/ut161b)
(`ut161b_protocol.md`) and [olegv142/ut61xpy](https://github.com/olegv142/ut61xpy)
(`TRIGGER_CMD`, over Bluetooth as well). ljakob's checksum is
`cmd = cmd + 379 # don't ask it's from the java source` — the same
`cmd + 0x17B` as §2.3, from the same vendor Java.

**Rate.** ut61xpy's README: "The minimum achievable data readout interval
is around **180 msec for USB** adapter and around **800 msec for
Bluetooth** adapter", polling 0x5E in both cases. The Bluetooth figure
matches our 0.63-0.8 s per poll over a UT-D07B
(`../ut-d07b/reverse-engineered-protocol.md` §5); the USB figure is well
above the ~100 ms / ~10 Hz measured here (§2.8) and looks tool-bound, but
it is the only other published number. No community source reports a
streamed rate for this family over an adapter.

**Frame layout.** ljakob's byte map (mode, range as ASCII digit, 7-byte
display, two bargraph bytes, three flag bytes, 16-bit big-endian sum) and
libreble's table agree with §2.4 and §2.7 field for field, including
`auto` being the *inverse* of flags-B bit 2 and the AC/DC bit in flags C.
